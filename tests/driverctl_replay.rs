#![cfg(target_os = "linux")]
// ============================================================================
// SC1: three recorded spike runs replayed through the REAL Rust client against a
// REAL Rust `driverctl serve` (Phase 27, plan 27-06; D-01, 27-RESEARCH "SC1
// acceptance tests" item 2).
//
// SC1 says the Rust binary replaces `tools/driverctl.py` with the same protocol.
// The proof is a replay: every driver-originated event of a recorded LOG.md
// (start, reconcile, dispatched, report, verified, status, needs-human,
// recovered, end) is sent as `driverctl log <event> ...` argv, with a report's
// body on stdin, and the LOG the Rust server writes must carry the same events
// in the same order with the same item, keys, values and detail lines.
//
// Three things are deliberately NOT replayed, because the tool computes them and
// the driver never supplies them: `base=` (a `git rev-parse` of the project; the
// replay project is not a repository), `dispatch=` (a count the tool keeps; it is
// compared by its counter, see below) and `truncated=` (derived from the body
// length; reproduced by sending one line more than the cap keeps).
//
// Launcher- and hook-written lines (`launch`, `connect`, `gap-observed`, ...) are
// not driver events and are not compared.
//
// The recorded `dispatch=N/B` is compared on N exactly. B is the item's
// `dispatches` budget, which the tool reads from the DIRECTION body; a recorded B
// that differs from the fixture's body budget (the body was edited after the
// line was written) is reported in the test output and the regenerated B is
// asserted against the fixture body instead.
//
// The recorded DIRECTION.md copy is the run's starting body: the server
// regenerates its tool-written frontmatter block on the first event.
// ============================================================================

use std::fs;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::time::{Duration, Instant};

use gsd_meta_manager::driver::direction::parse_direction;
use gsd_meta_manager::driverctl::log::{parse_log, LogEvent};
use gsd_meta_manager::state_reader::driver_frontmatter::parse_driver_frontmatter;
use tempfile::TempDir;

/// The unwritable data dir every spawn runs under (T-27-11).
const UNWRITABLE_DATA_HOME: &str = "/proc/nonexistent-gsdmm";

/// The events a driver sends; every other LOG line is launcher- or hook-written.
const DRIVER_EVENTS: &[&str] = &[
    "start",
    "reconcile",
    "dispatched",
    "report",
    "verified",
    "status",
    "needs-human",
    "recovered",
    "end",
];

/// Keys the tool computes; they are never sent as flags.
const TOOL_KEYS: &[&str] = &["base", "dispatch", "truncated"];

/// A recorded run: its id, LOG.md and starting DIRECTION.md.
struct Recorded {
    run_id: &'static str,
    log: &'static str,
    direction: &'static str,
}

const R261A: Recorded = Recorded {
    run_id: "s003-r261a",
    log: include_str!("fixtures/driverctl/spike003/r261a/LOG.md"),
    direction: include_str!("fixtures/driverctl/direction/s003-r261a.md"),
};

const RUN1: Recorded = Recorded {
    run_id: "s003-run1",
    log: include_str!("fixtures/driverctl/spike003/run1/LOG.md"),
    direction: include_str!("fixtures/driverctl/direction/s003-run1.md"),
};

const R2: Recorded = Recorded {
    run_id: "s002-R2",
    log: include_str!("fixtures/driverctl/replay/s002-R2/LOG.md"),
    direction: include_str!("fixtures/driverctl/direction/s002-R2-hidden-fixture.md"),
};

/// A project holding the run directory, plus a scratch dir for the socket.
struct Replay {
    project: TempDir,
    scratch: TempDir,
    run_dir: PathBuf,
}

/// A running server; killed on drop so a failed test leaves nothing behind.
struct Server {
    child: Child,
    stderr: PathBuf,
}

impl Drop for Server {
    fn drop(&mut self) {
        if matches!(self.child.try_wait(), Ok(None)) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

impl Replay {
    fn new(recorded: &Recorded) -> Self {
        let project = TempDir::new().expect("project dir");
        let scratch = TempDir::new().expect("scratch dir");
        let run_dir = project
            .path()
            .join(".planning/meta-manager/drivers")
            .join(recorded.run_id);
        fs::create_dir_all(&run_dir).expect("run dir");
        fs::write(run_dir.join("DIRECTION.md"), recorded.direction).expect("DIRECTION.md");
        Replay {
            project,
            scratch,
            run_dir,
        }
    }

    fn sock(&self) -> PathBuf {
        self.scratch.path().join("replay.sock")
    }

    fn serve(&self) -> Server {
        let stderr = self.scratch.path().join("serve.err");
        let child = Command::new(env!("CARGO_BIN_EXE_gsd-meta-manager"))
            .args(["driverctl", "serve", "--run-dir"])
            .arg(&self.run_dir)
            .arg("--project")
            .arg(self.project.path())
            .arg("--sock")
            .arg(self.sock())
            .env("XDG_DATA_HOME", UNWRITABLE_DATA_HOME)
            .env_remove("XDG_RUNTIME_DIR")
            .env_remove("DRIVERCTL_SOCK")
            .env_remove("CLAUDE_CODE_SESSION_ID")
            .stdin(Stdio::null())
            .stdout(fs::File::create(self.scratch.path().join("serve.out")).expect("stdout"))
            .stderr(fs::File::create(&stderr).expect("stderr"))
            .spawn()
            .expect("spawn serve");
        let mut server = Server { child, stderr };
        let started = Instant::now();
        loop {
            let text = fs::read_to_string(&server.stderr).unwrap_or_default();
            if text.contains("listening on") {
                return server;
            }
            if let Some(status) = server.child.try_wait().expect("try_wait") {
                panic!("serve exited early ({status}): {text}");
            }
            assert!(
                started.elapsed() < Duration::from_secs(10),
                "serve never reported listening: {text}"
            );
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    /// Run `driverctl <args>` (report bodies on stdin) and return rc plus stderr.
    fn client(&self, args: &[String], stdin: Option<&str>) -> (Option<i32>, String) {
        let mut child = Command::new(env!("CARGO_BIN_EXE_gsd-meta-manager"))
            .arg("driverctl")
            .args(args)
            .env("DRIVERCTL_SOCK", self.sock())
            .env("XDG_DATA_HOME", UNWRITABLE_DATA_HOME)
            .env_remove("CLAUDE_CODE_SESSION_ID")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .expect("spawn the client");
        {
            let mut pipe = child.stdin.take().expect("client stdin");
            if let Some(text) = stdin {
                pipe.write_all(text.as_bytes()).expect("write the body");
            }
        }
        let out = child.wait_with_output().expect("client output");
        (
            out.status.code(),
            String::from_utf8_lossy(&out.stderr).into_owned(),
        )
    }
}

/// The argv (and stdin) that reproduces one recorded driver event.
fn replay_args(event: &LogEvent) -> (Vec<String>, Option<String>) {
    let mut args = vec!["log".to_owned(), event.event.clone()];
    if event.item != "RUN" {
        args.push("--item".to_owned());
        args.push(event.item.clone());
    }
    for (key, value) in &event.kv {
        if TOOL_KEYS.contains(&key.as_str()) {
            continue;
        }
        // The LOG header spells `loop_mode`; the flag is `--loop-mode`.
        args.push(format!("--{}", key.replace('_', "-")));
        args.push(value.clone());
    }
    let stdin = (event.event == "report").then(|| {
        let mut lines = event.detail.clone();
        if event.get("truncated") == Some("yes") {
            lines.push("EXTRA: a line past the cap, so the report truncates again".to_owned());
        }
        format!("{}\n", lines.join("\n"))
    });
    (args, stdin)
}

fn driver_events(log: &str) -> Vec<LogEvent> {
    parse_log(log)
        .into_iter()
        .filter(|e| DRIVER_EVENTS.contains(&e.event.as_str()))
        .collect()
}

/// `(k, budget)` of a `dispatch=k/budget` value.
fn split_dispatch(value: &str) -> (&str, &str) {
    value.split_once('/').unwrap_or((value, ""))
}

/// Replay `recorded` and compare; returns the dispatch lines whose recorded
/// budget differs from the fixture body's (for the SUMMARY).
fn replay_and_compare(recorded: &Recorded) -> Vec<String> {
    let replay = Replay::new(recorded);
    let _server = replay.serve();
    let wanted = driver_events(recorded.log);
    assert!(
        wanted.len() >= 10,
        "{}: the recorded LOG should carry a real run, got {} driver events",
        recorded.run_id,
        wanted.len()
    );

    for (n, event) in wanted.iter().enumerate() {
        let (args, stdin) = replay_args(event);
        let (rc, err) = replay.client(&args, stdin.as_deref());
        assert_eq!(
            rc,
            Some(0),
            "{}: event #{n} `{}` ({} at {}) exited {rc:?}: {err}\nargv: {args:?}",
            recorded.run_id,
            event.event,
            event.item,
            event.at
        );
    }

    let regenerated = driver_events(&fs::read_to_string(replay.run_dir.join("LOG.md")).unwrap());
    assert_eq!(
        regenerated.len(),
        wanted.len(),
        "{}: the regenerated LOG has the same number of driver events",
        recorded.run_id
    );

    let body_budgets = parse_direction(recorded.direction);
    let mut budget_edits = Vec::new();
    for (n, (want, got)) in wanted.iter().zip(&regenerated).enumerate() {
        let where_ = format!(
            "{} event #{n} ({} {})",
            recorded.run_id, want.item, want.event
        );
        assert_eq!(want.item, got.item, "{where_}: item");
        assert_eq!(want.event, got.event, "{where_}: event");
        assert_eq!(want.detail, got.detail, "{where_}: detail lines");

        let comparable = |e: &LogEvent| -> Vec<(String, String)> {
            e.kv.iter()
                .filter(|(k, _)| k != "base" && k != "dispatch")
                .cloned()
                .collect()
        };
        assert_eq!(
            comparable(want),
            comparable(got),
            "{where_}: key/value pairs"
        );

        if let Some(recorded_dispatch) = want.get("dispatch") {
            let got_dispatch = got
                .get("dispatch")
                .unwrap_or_else(|| panic!("{where_}: the regenerated line has no dispatch="));
            let (want_n, want_budget) = split_dispatch(recorded_dispatch);
            let (got_n, got_budget) = split_dispatch(got_dispatch);
            assert_eq!(want_n, got_n, "{where_}: the dispatch counter");
            let body_budget = body_budgets
                .items
                .iter()
                .find(|i| i.id == want.item)
                .and_then(|i| i.budget.get("dispatches"))
                .map(i64::to_string)
                .unwrap_or_else(|| "None".to_owned());
            assert_eq!(
                got_budget, body_budget,
                "{where_}: the budget comes from the DIRECTION body"
            );
            if want_budget != got_budget {
                budget_edits.push(format!(
                    "{where_}: recorded {recorded_dispatch}, regenerated {got_dispatch}"
                ));
            }
        } else {
            assert!(
                got.get("dispatch").is_none(),
                "{where_}: only dispatched lines carry dispatch="
            );
        }
    }

    // The regenerated frontmatter agrees with the recorded one on the audit
    // counter and the item counts.
    let recorded_fm = parse_driver_frontmatter(recorded.direction).expect("recorded frontmatter");
    let regenerated_fm = parse_driver_frontmatter(
        &fs::read_to_string(replay.run_dir.join("DIRECTION.md")).expect("DIRECTION.md"),
    )
    .expect("regenerated frontmatter");
    assert_eq!(
        regenerated_fm.dispatch_audit.logged, recorded_fm.dispatch_audit.logged,
        "{}: dispatch_audit.logged",
        recorded.run_id
    );
    assert_eq!(
        regenerated_fm.items, recorded_fm.items,
        "{}: item counts",
        recorded.run_id
    );

    for line in &budget_edits {
        eprintln!("REPLAY-NOTE budget differs from the recorded line: {line}");
    }
    budget_edits
}

#[test]
fn replay_s003_r261a_reproduces_every_driver_event() {
    replay_and_compare(&R261A);
}

#[test]
fn replay_s003_run1_reproduces_every_driver_event() {
    replay_and_compare(&RUN1);
}

#[test]
fn replay_s002_r2_reproduces_every_driver_event() {
    replay_and_compare(&R2);
}

#[test]
fn the_replay_fixtures_carry_the_events_the_replay_is_meant_to_cover() {
    // Guards the guard: a fixture that lost its reports (or its truncated
    // report) would make the replay pass without replaying them.
    let mut seen = std::collections::BTreeSet::new();
    let mut truncated = 0;
    for recorded in [&R261A, &RUN1, &R2] {
        for event in driver_events(recorded.log) {
            if event.event == "report" && event.get("truncated") == Some("yes") {
                truncated += 1;
            }
            seen.insert(event.event);
        }
    }
    for needed in [
        "start",
        "reconcile",
        "dispatched",
        "report",
        "verified",
        "status",
        "needs-human",
        "end",
    ] {
        assert!(seen.contains(needed), "no recorded `{needed}` event");
    }
    assert!(truncated >= 1, "no recorded truncated report");
}
