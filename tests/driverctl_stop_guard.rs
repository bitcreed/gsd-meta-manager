// ============================================================================
// `driverctl stop-guard` through the REAL binary (Phase 28, plan 28-06; D-17,
// LNCH-03, T-28-16). A port of the spike-003 `stop_guard.py` selftest plus the
// committed spike LOGs as fixtures: the guard must make the decisions the spike
// guard recorded for the same LOG prefixes.
//
// Every call is a fresh process with an empty environment but
// `CLAUDE_CONFIG_DIR`, exactly as Claude Code runs the hook inside the sandbox.
// ============================================================================

use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};

use gsd_meta_manager::driverctl::counters::outstanding_dispatch;
use gsd_meta_manager::driverctl::log::parse_log;
use serde_json::{json, Value};
use tempfile::TempDir;

const FIXTURES: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/driverctl/spike003"
);

const STOP: &str = r#"{"hook_event_name":"Stop","session_id":"s","stop_hook_active":false,"background_tasks":[],"last_assistant_message":"PLANTED-SECRET-c0ffee"}"#;

fn fixture(name: &str, file: &str) -> String {
    fs::read_to_string(Path::new(FIXTURES).join(name).join(file))
        .unwrap_or_else(|e| panic!("fixture {name}/{file}: {e}"))
}

struct Fx {
    _root: TempDir,
    rd: PathBuf,
    cfg: PathBuf,
}

impl Fx {
    fn with_log(log: &str) -> Fx {
        let root = TempDir::new().unwrap();
        let rd = root.path().join("rundir");
        let cfg = root.path().join("cfg");
        fs::create_dir_all(&rd).unwrap();
        fs::create_dir_all(&cfg).unwrap();
        fs::write(rd.join("LOG.md"), log).unwrap();
        Fx {
            _root: root,
            rd,
            cfg,
        }
    }

    fn put(&self, log: &str) {
        fs::write(self.rd.join("LOG.md"), log).unwrap();
    }

    fn command(&self) -> Command {
        let mut cmd = Command::new(env!("CARGO_BIN_EXE_gsd-meta-manager"));
        cmd.args(["driverctl", "stop-guard", "--run-dir"])
            .arg(&self.rd)
            .env_clear()
            .env("CLAUDE_CONFIG_DIR", &self.cfg);
        cmd
    }

    /// One hook call.
    fn call(&self, stdin: &str) -> Output {
        run_with_stdin(self.command(), stdin)
    }

    /// The decision lines in `stop-guard.log`, timestamps stripped.
    fn decisions(&self) -> Vec<String> {
        fs::read_to_string(self.cfg.join("stop-guard.log"))
            .unwrap_or_default()
            .lines()
            .map(|l| {
                let (stamp, rest) = l.split_once(' ').expect("a stamped line");
                assert!(stamp.ends_with('Z') && stamp.len() == 20, "{l}");
                rest.to_owned()
            })
            .collect()
    }

    fn last_decision(&self) -> String {
        self.decisions().pop().expect("a decision was logged")
    }

    fn counter(&self) -> Value {
        serde_json::from_str(&fs::read_to_string(self.cfg.join("stop-guard.count")).unwrap())
            .unwrap()
    }
}

fn run_with_stdin(mut cmd: Command, stdin: &str) -> Output {
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the binary is built and spawnable");
    // The hook may exit before reading (it does not here); ignore a broken pipe.
    let _ = child.stdin.take().unwrap().write_all(stdin.as_bytes());
    child.wait_with_output().unwrap()
}

fn blocked(out: &Output) -> bool {
    assert_eq!(out.status.code(), Some(0), "the hook always exits 0");
    !out.stdout.is_empty()
}

#[test]
fn budget_is_three_blocks_then_the_limit_for_one_dispatch() {
    let fx = Fx::with_log(&fixture("run1b", "LOG.md"));
    let results: Vec<Output> = (0..5).map(|_| fx.call(STOP)).collect();

    let kinds: Vec<bool> = results.iter().map(blocked).collect();
    assert_eq!(kinds, [true, true, true, false, false]);

    let reply: Value = serde_json::from_slice(&results[0].stdout).unwrap();
    assert_eq!(reply["decision"], "block");
    let reason = reply["reason"].as_str().unwrap();
    assert!(reason.contains("I-01") && reason.contains("SendMessage") && reason.contains("SAME"));
    assert!(
        !String::from_utf8_lossy(&results[0].stdout).contains("PLANTED-SECRET"),
        "the payload is not echoed"
    );

    let decisions = fx.decisions();
    let key = fx.counter()["key"].as_str().unwrap().to_owned();
    assert_eq!(
        decisions[..4],
        [
            format!("reset from=- to={key}"),
            "block item=I-01 attempt=1".to_owned(),
            "block item=I-01 attempt=2".to_owned(),
            "block item=I-01 attempt=3".to_owned(),
        ]
    );
    assert_eq!(decisions[4..], ["allow reason=limit", "allow reason=limit"]);
    assert_eq!(fx.counter()["n"], 4);
    // The key names the live dispatch the way the exit classifier would.
    let events = parse_log(&fixture("run1b", "LOG.md"));
    let (item, index) = outstanding_dispatch(&events).expect("run1b has an unreported dispatch");
    assert_eq!(key, format!("{item}@{index}"));
}

#[test]
fn a_new_dispatch_resets_the_budget_and_logs_the_reset() {
    let base = fixture("run1b", "LOG.md");
    let fx = Fx::with_log(&base);
    for _ in 0..4 {
        fx.call(STOP);
    }
    let key0 = fx.counter()["key"].as_str().unwrap().to_owned();

    let report = "- 2026-10-17T03:09:00Z | I-01 | report | status=done\n";
    let redo = "- 2026-10-17T03:10:00Z | I-01 | dispatched | step=/gsd-plan-phase 1 | base=e1003bc | round=0 | dispatch=2/8 | phase=1\n";
    fx.put(&format!("{base}{report}{redo}"));
    assert!(blocked(&fx.call(STOP)));
    let key1 = fx.counter()["key"].as_str().unwrap().to_owned();
    assert_ne!(key0, key1);
    assert_eq!(fx.counter()["n"], 1);
    let decisions = fx.decisions();
    assert_eq!(
        decisions[decisions.len() - 2..],
        [
            format!("reset from={key0} to={key1}"),
            "block item=I-01 attempt=1".to_owned()
        ]
    );
}

#[test]
fn nothing_outstanding_allows_and_clears_a_nonzero_counter() {
    let base = fixture("run1b", "LOG.md");
    let fx = Fx::with_log(&base);
    fx.call(STOP);
    let key = fx.counter()["key"].as_str().unwrap().to_owned();

    fx.put(&format!(
        "{base}- 2026-10-17T03:09:00Z | I-01 | report | status=done\n"
    ));
    let out = fx.call(STOP);
    assert!(!blocked(&out));
    let decisions = fx.decisions();
    assert_eq!(
        decisions[decisions.len() - 2..],
        [
            format!("reset from={key} to=-"),
            "allow reason=no-outstanding".to_owned()
        ]
    );
    assert!(!fx.cfg.join("stop-guard.count").exists());

    let before = fx.decisions().len();
    fx.call(STOP);
    assert_eq!(fx.decisions().len(), before + 1, "no second reset line");
}

#[test]
fn the_run1_log_is_run_end_whole_and_no_outstanding_before_its_end() {
    let run1 = fixture("run1", "LOG.md");
    let fx = Fx::with_log(&run1);
    assert!(!blocked(&fx.call(STOP)));
    assert_eq!(fx.last_decision(), "allow reason=run-end");

    // The same LOG cut before `RUN | end`: every dispatch is reported or parked.
    let cut = run1
        .lines()
        .position(|l| l.contains("| RUN | end |"))
        .expect("run1 ends");
    let prefix: String = run1.lines().take(cut).map(|l| format!("{l}\n")).collect();
    fx.put(&prefix);
    assert!(!blocked(&fx.call(STOP)));
    assert_eq!(fx.last_decision(), "allow reason=no-outstanding");
}

#[test]
fn the_other_allow_paths_never_touch_the_counter() {
    let fx = Fx::with_log(&fixture("run1b", "LOG.md"));
    let with = |key: &str, value: Value| {
        let mut payload: Value = serde_json::from_str(STOP).unwrap();
        payload[key] = value;
        payload.to_string()
    };
    let cases: Vec<(String, &str)> = vec![
        (
            with("background_tasks", json!([{"task_id": "a1"}])),
            "background",
        ),
        (with("agent_id", json!("a1")), "not-top-level"),
        (
            with("hook_event_name", json!("SubagentStop")),
            "not-top-level",
        ),
        ("{not json".to_owned(), "bad-input"),
        (String::new(), "bad-input"),
        ("[1, 2]".to_owned(), "bad-input"),
    ];
    for (payload, reason) in cases {
        assert!(!blocked(&fx.call(&payload)), "{payload}");
        assert_eq!(fx.last_decision(), format!("allow reason={reason}"));
    }
    assert!(!fx.cfg.join("stop-guard.count").exists());

    // No `--run-dir` at all.
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_gsd-meta-manager"));
    cmd.args(["driverctl", "stop-guard"])
        .env_clear()
        .env("CLAUDE_CONFIG_DIR", &fx.cfg);
    assert!(!blocked(&run_with_stdin(cmd, STOP)));
    assert_eq!(fx.last_decision(), "allow reason=bad-input");
}

#[test]
fn run_end_and_a_missing_config_dir_allow() {
    let fx = Fx::with_log(&format!(
        "{}- 2026-10-17T03:08:00Z | RUN | end | reason=stop\n",
        fixture("run1b", "LOG.md")
    ));
    assert!(!blocked(&fx.call(STOP)));
    assert_eq!(fx.last_decision(), "allow reason=run-end");

    // CLAUDE_CONFIG_DIR unset: allowed, and nothing is written anywhere.
    let fx = Fx::with_log(&fixture("run1b", "LOG.md"));
    let mut cmd = fx.command();
    cmd.env_remove("CLAUDE_CONFIG_DIR");
    let out = run_with_stdin(cmd, STOP);
    assert!(!blocked(&out));
    assert_eq!(fs::read_dir(&fx.cfg).unwrap().count(), 0);
}

#[test]
fn a_forged_counter_can_never_extend_the_budget() {
    // IN-01: the counter lives where the model can write. Whatever it holds, at
    // most three blocks per dispatch come out of six calls.
    let forged = [
        "-999\n",
        "garbage\n",
        "3\n",
        r#"{"key": "I-01@0", "n": -999}"#,
        r#"{"key": "I-01@0", "n": "1"}"#,
        r#"{"key": "I-01@0", "n": true}"#,
        r#"{"n": 2}"#,
        "[1, 2]",
        "",
    ];
    for content in forged {
        let fx = Fx::with_log(&fixture("run1b", "LOG.md"));
        fs::write(fx.cfg.join("stop-guard.count"), content).unwrap();
        let results: Vec<Output> = (0..6).map(|_| fx.call(STOP)).collect();
        let blocks = results.iter().filter(|o| blocked(o)).count();
        assert_eq!(blocks, 3, "{content:?}");
        assert!(
            fx.decisions()
                .iter()
                .any(|d| d == "block item=I-01 attempt=1"),
            "{content:?}: {:?}",
            fx.decisions()
        );
    }
    // A forged count that names the live key and is in range is honoured only as
    // far as the clamp allows.
    let fx = Fx::with_log(&fixture("run1b", "LOG.md"));
    let (item, index) = outstanding_dispatch(&parse_log(&fixture("run1b", "LOG.md"))).unwrap();
    fs::write(
        fx.cfg.join("stop-guard.count"),
        json!({"key": format!("{item}@{index}"), "n": 2}).to_string(),
    )
    .unwrap();
    assert!(blocked(&fx.call(STOP)));
    assert_eq!(fx.last_decision(), "block item=I-01 attempt=3");
    assert!(!blocked(&fx.call(STOP)));
}

#[test]
fn the_hook_exits_zero_on_every_path_and_never_writes_under_the_run_dir() {
    let fx = Fx::with_log(&fixture("run1b", "LOG.md"));
    let snapshot = |dir: &Path| -> Vec<(String, u64)> {
        let mut out: Vec<_> = fs::read_dir(dir)
            .unwrap()
            .filter_map(Result::ok)
            .map(|e| {
                (
                    e.file_name().to_string_lossy().into_owned(),
                    e.metadata().unwrap().len(),
                )
            })
            .collect();
        out.sort();
        out
    };
    let before = snapshot(&fx.rd);
    for _ in 0..5 {
        assert_eq!(fx.call(STOP).status.code(), Some(0));
    }
    assert_eq!(
        snapshot(&fx.rd),
        before,
        "the run dir is read-only to the guard"
    );

    // A run dir that does not exist, one that is a file, a config dir that does
    // not exist, a closed stdin and non-UTF-8 input: all exit 0 and print nothing.
    let mut cmd2 = Command::new(env!("CARGO_BIN_EXE_gsd-meta-manager"));
    cmd2.args([
        "driverctl",
        "stop-guard",
        "--run-dir",
        "/nonexistent/run/dir",
    ])
    .env_clear()
    .env("CLAUDE_CONFIG_DIR", &fx.cfg);
    let out = run_with_stdin(cmd2, STOP);
    assert_eq!(out.status.code(), Some(0));
    assert!(out.stdout.is_empty());

    let mut cmd3 = Command::new(env!("CARGO_BIN_EXE_gsd-meta-manager"));
    cmd3.args(["driverctl", "stop-guard", "--run-dir"])
        .arg(fx.rd.join("LOG.md"))
        .env_clear()
        .env("CLAUDE_CONFIG_DIR", fx.cfg.join("does/not/exist"));
    let out = run_with_stdin(cmd3, STOP);
    assert_eq!(out.status.code(), Some(0));
    assert!(out.stdout.is_empty());

    let out = fx
        .command()
        .stdin(Stdio::null())
        .output()
        .expect("spawnable");
    assert_eq!(out.status.code(), Some(0));
    let garbage = run_with_stdin(fx.command(), "\u{0}\u{1}not json\n");
    assert_eq!(garbage.status.code(), Some(0));
    assert!(garbage.stdout.is_empty());
}

/// The LOG lines of `name` up to and including the first line that contains `marker`.
fn log_through(name: &str, marker: &str) -> String {
    let log = fixture(name, "LOG.md");
    let mut out = String::new();
    for line in log.lines() {
        out.push_str(line);
        out.push('\n');
        if line.contains(marker) {
            return out;
        }
    }
    panic!("{marker} not in {name}/LOG.md");
}

#[test]
fn the_r261_l8_run_replays_to_the_decisions_the_spike_guard_recorded() {
    // results/r261-l8/stop-guard.log: background, background, block attempt 1,
    // background, run-end. Replay each call at the LOG prefix it was made at.
    let recorded: Vec<String> = fixture("r261-l8", "stop-guard.log")
        .lines()
        .map(|l| l.split_once(' ').unwrap().1.to_owned())
        .collect();
    assert_eq!(
        recorded,
        [
            "allow reason=background",
            "allow reason=background",
            "block item=I-01 attempt=1",
            "allow reason=background",
            "allow reason=run-end"
        ]
    );

    let fx = Fx::with_log("");
    let background = json!({
        "hook_event_name": "Stop", "session_id": "s", "background_tasks": [{"task_id": "a1"}]
    })
    .to_string();
    let dispatched = log_through("r261-l8", "| I-01 | dispatched |");
    let ended = log_through("r261-l8", "| RUN | end |");

    fx.put(&dispatched);
    fx.call(&background);
    fx.call(&background);
    fx.call(STOP);
    fx.call(&background);
    fx.put(&ended);
    fx.call(STOP);
    assert_eq!(fx.decisions(), {
        // The first block is preceded by its `reset` line.
        let key = format!("reset from=- to=I-01@{}", {
            outstanding_dispatch(&parse_log(&dispatched)).unwrap().1
        });
        let mut want = recorded.clone();
        want.insert(2, key);
        want
    });
}
