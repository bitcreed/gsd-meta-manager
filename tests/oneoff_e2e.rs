//! `task run|status|stop|land` end to end on the fake systemd-run / bwrap /
//! systemctl / claude harness (quick 261103-f5y, Task 2). No real agent and no
//! network: the fake agent runs a scenario script in the clone.

#![cfg(unix)]

mod launcher_support;

use std::fs;
use std::os::unix::fs::MetadataExt;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::time::{Duration, Instant};

use gsd_meta_manager::driverctl::serve::hash8;
use launcher_support::{wait_until, Fixture, ALIAS};
use serde_json::Value;

const TODO: &str = "2026-11-03-demo.md";
const SUMMARY: &str = ".planning/quick/261103-abc-demo/261103-abc-SUMMARY.md";

/// `git` in `dir` with a fixed identity and no host configuration.
fn git(dir: &Path, args: &[&str]) -> Output {
    let out = Command::new("git")
        .current_dir(dir)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .args(["-c", "user.name=t", "-c", "user.email=t@example.com"])
        .args(args)
        .output()
        .expect("git runs");
    assert!(out.status.success(), "git {args:?}: {out:?}");
    out
}

fn git_text(dir: &Path, args: &[&str]) -> String {
    String::from_utf8_lossy(&git(dir, args).stdout)
        .trim()
        .to_owned()
}

/// A fixture whose project is a git repository with one committed pending todo.
fn fixture() -> Fixture {
    let fx = Fixture::new();
    let project = fx.project();
    let pending = project.join(".planning/todos/pending");
    fs::create_dir_all(&pending).unwrap();
    fs::write(
        pending.join(TODO),
        "---\ntitle: \"Demo the one-off\"\n---\n\n## Problem\nx\n",
    )
    .unwrap();
    git(project, &["init", "-q", "-b", "main"]);
    git(project, &["add", "-A"]);
    git(project, &["commit", "-qm", "init"]);
    fx
}

/// `gsd-meta-manager task <args...>` under the fixture's hermetic environment.
fn task(fx: &Fixture, args: &[&str]) -> Command {
    let mut cmd = fx.hermetic_command();
    cmd.arg("task").args(args);
    cmd
}

fn run(fx: &Fixture, args: &[&str]) -> Output {
    task(fx, args).output().expect("the binary runs")
}

fn project_dir(fx: &Fixture) -> PathBuf {
    let canonical = fs::canonicalize(fx.project()).unwrap();
    fx.scratch
        .path()
        .join("data/gsd-meta-manager/oneoff")
        .join(hash8(&canonical))
}

fn run_dir(fx: &Fixture, id: &str) -> PathBuf {
    project_dir(fx).join(id)
}

fn status(fx: &Fixture) -> Value {
    let out = run(fx, &["status", ALIAS, "--json"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    serde_json::from_slice(&out.stdout).expect("status --json parses")
}

/// Wait until the latest run is finished and its lock is free.
fn wait_finished(fx: &Fixture) -> Value {
    let mut last = Value::Null;
    let done = wait_until(Duration::from_secs(60), || {
        let out = run(fx, &["status", ALIAS, "--json"]);
        if !out.status.success() {
            return false;
        }
        last = serde_json::from_slice(&out.stdout).unwrap_or(Value::Null);
        last["live"] == "finished"
    });
    assert!(done, "the run never finished: {last}");
    last
}

const QUICK_SCENARIO: &str = r#"
set -e
G="git -c user.name=t -c user.email=t@example.com"
mkdir -p .planning/quick/261103-abc-demo
echo summary > .planning/quick/261103-abc-demo/261103-abc-SUMMARY.md
$G add -A
$G commit -qm "docs: summary"
mkdir -p .planning/todos/completed
$G mv .planning/todos/pending/2026-11-03-demo.md .planning/todos/completed/2026-11-03-demo.md
$G commit -qm "chore: todo done"
if gsd-meta-manager taskctl report --status done --headline "" 2>/dev/null; then echo "bad args accepted" >&2; exit 9; fi
gsd-meta-manager taskctl report --status done --headline "Did the demo" --changed "the summary" --verified "true: ok"
printf '%s\n' '{"type":"result","subtype":"success"}'
"#;

/// Run the quick scenario on the todo, detached, and wait for it to finish.
fn quick_run(fx: &Fixture) -> (String, Value) {
    fx.write_scenario(QUICK_SCENARIO);
    let out = run(fx, &["run", ALIAS, "--todo", TODO]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let stdout = String::from_utf8_lossy(&out.stdout);
    let id = stdout
        .split_whitespace()
        .find(|w| w.starts_with("oneoff-"))
        .unwrap_or_else(|| panic!("no run id in {stdout}"))
        .to_owned();
    let status = wait_finished(fx);
    assert_eq!(status["record"]["run_id"], id.as_str());
    (id, status)
}

fn first_object_file(dir: &Path) -> Option<PathBuf> {
    for entry in fs::read_dir(dir).ok()?.flatten() {
        let path = entry.path();
        let kind = entry.file_type().ok()?;
        if kind.is_file() {
            return Some(path);
        }
        if kind.is_dir() {
            if let Some(found) = first_object_file(&path) {
                return Some(found);
            }
        }
    }
    None
}

#[test]
fn a_detached_quick_run_on_a_todo_reports_records_and_merges() {
    let fx = fixture();
    let (id, status) = quick_run(&fx);
    let record = &status["record"];
    assert_eq!(record["report_source"]["kind"], "agent", "{record}");
    assert_eq!(record["report"]["status"], "done");
    assert_eq!(record["commits"], 2);
    assert_eq!(record["summary_path"], SUMMARY);
    assert_eq!(record["outcome"]["kind"], "finished");

    let dir = run_dir(&fx, &id);
    let stream = fs::read_to_string(dir.join("stream.jsonl")).unwrap();
    assert!(stream.contains(r#""subtype":"success""#), "{stream}");
    let prompt = fx.recorded("prompt.txt");
    assert!(prompt.starts_with("/gsd-quick "), "{prompt}");
    assert!(
        prompt.contains("details: .planning/todos/pending/"),
        "{prompt}"
    );
    let argv = fx.recorded("argv.txt");
    assert!(argv.contains("--max-budget-usd"), "{argv}");
    assert!(argv.contains("--setting-sources"), "{argv}");
    let shims = fx.shim_log();
    assert!(shims.contains("RuntimeMaxSec="), "{shims}");
    let report = dir.join("report").display().to_string();
    assert!(
        shims.contains(&format!("--bind {report} {report}")),
        "{shims}"
    );
    assert!(!shims.contains("launcher.lock"), "{shims}");
    assert!(!shims.contains("gsdmm-driverctl/oneoff"), "{shims}");
    assert!(!dir.join("claude-config/bin/driverctl").exists());
    assert!(dir.join("claude-config/bin/gsd-meta-manager").exists());
    let object = first_object_file(&dir.join("work/.git/objects")).expect("an object file");
    assert_eq!(fs::metadata(&object).unwrap().nlink(), 1, "{object:?}");

    // (b) merge, then discard.
    let merged = run(&fx, &["land", "merge", ALIAS, &id]);
    assert_eq!(merged.status.code(), Some(0), "{merged:?}");
    assert_eq!(
        git_text(fx.project(), &["rev-parse", "HEAD"]),
        record["head"].as_str().unwrap()
    );
    assert!(fx
        .project()
        .join(".planning/todos/completed")
        .join(TODO)
        .exists());
    let after = status_of(&fx);
    assert_eq!(after["record"]["landed"][0]["kind"], "merged");
    let discarded = run(&fx, &["land", "discard", ALIAS, &id]);
    assert_eq!(discarded.status.code(), Some(0), "{discarded:?}");
    assert!(!dir.exists());
}

fn status_of(fx: &Fixture) -> Value {
    status(fx)
}

#[test]
fn a_merge_is_refused_on_a_dirty_checkout_and_when_the_project_moved() {
    let fx = fixture();
    let (id, _) = quick_run(&fx);
    let before = git_text(fx.project(), &["rev-parse", "HEAD"]);

    fs::write(fx.project().join("untracked.txt"), "x").unwrap();
    let dirty = run(&fx, &["land", "merge", ALIAS, &id]);
    assert_ne!(dirty.status.code(), Some(0), "{dirty:?}");
    assert!(
        String::from_utf8_lossy(&dirty.stderr).contains("untracked"),
        "{dirty:?}"
    );
    assert_eq!(git_text(fx.project(), &["rev-parse", "HEAD"]), before);

    git(fx.project(), &["add", "untracked.txt"]);
    git(fx.project(), &["commit", "-qm", "moved on"]);
    let moved_head = git_text(fx.project(), &["rev-parse", "HEAD"]);
    let moved = run(&fx, &["land", "merge", ALIAS, &id]);
    assert_ne!(moved.status.code(), Some(0), "{moved:?}");
    assert!(
        String::from_utf8_lossy(&moved.stderr).contains("not a fast-forward"),
        "{moved:?}"
    );
    assert_eq!(git_text(fx.project(), &["rev-parse", "HEAD"]), moved_head);
}

#[test]
fn a_push_lands_on_the_reserved_branch_once() {
    let fx = fixture();
    let origin = fx.scratch.path().join("origin.git");
    git(fx.scratch.path(), &["init", "-q", "--bare", "origin.git"]);
    git(
        fx.project(),
        &["remote", "add", "origin", &origin.display().to_string()],
    );
    let (id, status) = quick_run(&fx);
    let pushed = run(&fx, &["land", "push", ALIAS, &id]);
    assert_eq!(pushed.status.code(), Some(0), "{pushed:?}");
    let dest = format!("refs/heads/gsd-auto/{ALIAS}/{id}");
    assert_eq!(
        git_text(&origin, &["rev-parse", &dest]),
        status["record"]["head"].as_str().unwrap()
    );
    let again = run(&fx, &["land", "push", ALIAS, &id]);
    assert_eq!(again.status.code(), Some(0), "{again:?}");
    assert!(
        String::from_utf8_lossy(&again.stdout).contains("already pushed"),
        "{again:?}"
    );
}

#[test]
fn no_report_is_synthesized_from_the_outcome() {
    let fx = fixture();
    fx.write_scenario(
        "printf '%s\\n' '{\"type\":\"result\",\"subtype\":\"error_max_budget_usd\"}'\nexit 1\n",
    );
    let out = run(&fx, &["run", ALIAS, "--text", "do x", "--foreground"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let record = &status(&fx)["record"];
    assert_eq!(record["outcome"]["kind"], "over_budget", "{record}");
    assert_eq!(record["report_source"]["kind"], "synthesized");
    assert_eq!(record["report"]["status"], "failed");
}

#[test]
fn a_fast_run_has_the_fast_prompt_and_no_summary() {
    let fx = fixture();
    fx.write_scenario("printf '%s\\n' '{\"type\":\"result\",\"subtype\":\"success\"}'\n");
    let out = run(
        &fx,
        &[
            "run",
            ALIAS,
            "--mode",
            "fast",
            "--text",
            "x",
            "--foreground",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(fx.recorded("prompt.txt").starts_with("/gsd-fast x"));
    let record = &status(&fx)["record"];
    assert!(record["summary_path"].is_null(), "{record}");
    assert_eq!(record["report_source"]["kind"], "synthesized");
    assert_eq!(record["report"]["status"], "partial");
}

#[test]
fn a_second_run_is_refused_while_one_is_live_and_stop_ends_it() {
    let fx = fixture();
    fx.write_scenario(
        "touch \"$FAKE_CLAUDE_DIR/started\"\nwhile [ ! -f \"$FAKE_CLAUDE_DIR/release\" ]; do sleep 0.1; done\n",
    );
    let mut child = task(&fx, &["run", ALIAS, "--text", "wait", "--foreground"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    assert!(
        wait_until(Duration::from_secs(30), || fx.scenario_started()),
        "the scenario never started"
    );
    let live = status(&fx);
    let id = live["record"]["run_id"].as_str().unwrap().to_owned();
    assert_eq!(live["live"], "running");

    let second = run(&fx, &["run", ALIAS, "--text", "another"]);
    assert_eq!(second.status.code(), Some(3), "{second:?}");
    assert!(
        String::from_utf8_lossy(&second.stderr).contains(&id),
        "{second:?}"
    );

    let stopped = run(&fx, &["stop", ALIAS]);
    assert_eq!(stopped.status.code(), Some(0), "{stopped:?}");
    let exited = wait_until(Duration::from_secs(30), || {
        matches!(child.try_wait(), Ok(Some(_)))
    });
    if !exited {
        let _ = child.kill();
    }
    assert!(exited, "the foreground run did not exit after stop");
    let after = status(&fx);
    assert_eq!(after["record"]["outcome"]["kind"], "stopped", "{after}");
    assert_eq!(after["live"], "finished");
}

#[test]
fn the_wall_clock_times_a_run_out() {
    let fx = fixture();
    fx.write_scenario("sleep 30\n");
    let started = Instant::now();
    let out = run(
        &fx,
        &[
            "run",
            ALIAS,
            "--text",
            "slow",
            "--cap-minutes",
            "0.02",
            "--foreground",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(
        started.elapsed() < Duration::from_secs(20),
        "{:?}",
        started.elapsed()
    );
    let record = &status(&fx)["record"];
    assert_eq!(record["outcome"]["kind"], "timed_out", "{record}");
}

#[test]
fn an_uncommitted_todo_fails_before_the_agent_starts() {
    let fx = fixture();
    fs::write(
        fx.project().join(".planning/todos/pending/new.md"),
        "# New\n",
    )
    .unwrap();
    let out = run(&fx, &["run", ALIAS, "--todo", "new.md", "--foreground"]);
    assert_ne!(out.status.code(), Some(0), "{out:?}");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("commit the todo first"),
        "{out:?}"
    );
    assert!(!fx.rec_dir().join("prompt.txt").exists());
}

// ---- quick 261103-gt1: stdin stream-json, the inbox and the session lifecycle ----

/// A turn that ends at once (the shim already replayed the prompt), then reads stdin
/// until EOF and records that the launcher closed it.
const STREAM_TURN_SCENARIO: &str = r#"
printf '%s\n' '{"type":"result","subtype":"success","total_cost_usd":0.01,"num_turns":1}'
while IFS= read -r line; do :; done
touch "$FAKE_CLAUDE_DIR/eof"
"#;

/// Waits for operator messages: replays each, ends its turn with a result, and records
/// the stdin EOF. No result is printed for the prompt, so the session stays open.
const INJECT_SCENARIO: &str = r#"
touch "$FAKE_CLAUDE_DIR/started"
while IFS= read -r line; do
  printf '%s\n' "$line" | sed 's/^{/{"isReplay":true,/'
  printf '%s\n' '{"type":"result","subtype":"success","total_cost_usd":0.5,"num_turns":1}'
done
touch "$FAKE_CLAUDE_DIR/eof"
"#;

fn lines_of(path: &Path) -> Vec<Value> {
    fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(|l| serde_json::from_str(l).unwrap_or(Value::Null))
        .collect()
}

#[test]
fn the_prompt_arrives_on_a_piped_stdin_and_the_turn_ends_the_session() {
    let fx = fixture();
    fx.write_scenario(STREAM_TURN_SCENARIO);
    let out = run(&fx, &["run", ALIAS, "--todo", TODO, "--foreground"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let argv = fx.recorded("argv.txt");
    for flag in [
        "--input-format",
        "--replay-user-messages",
        "--forward-subagent-text",
    ] {
        assert!(argv.lines().any(|l| l == flag), "{flag} in {argv}");
    }
    assert!(!argv.lines().any(|l| l == "--"), "{argv}");
    let prompt = fx.recorded("prompt.txt");
    assert!(prompt.starts_with("/gsd-quick "), "{prompt}");
    assert!(
        prompt.contains("details: .planning/todos/pending/"),
        "{prompt}"
    );
    assert!(
        fx.recorded("stdin.txt").starts_with("pipe:"),
        "{}",
        fx.recorded("stdin.txt")
    );
    assert!(
        fx.rec_dir().join("eof").exists(),
        "the launcher never closed the agent's stdin"
    );
    let record = status(&fx);
    assert_eq!(record["record"]["outcome"]["kind"], "finished", "{record}");
    let id = record["record"]["run_id"].as_str().unwrap();
    let stream = fs::read_to_string(run_dir(&fx, id).join("stream.jsonl")).unwrap();
    assert!(stream.contains("isReplay"), "{stream}");
}

#[test]
fn task_send_is_delivered_recorded_replayed_and_refused_once_the_run_is_over() {
    let fx = fixture();
    fx.write_scenario(INJECT_SCENARIO);
    let out = run(&fx, &["run", ALIAS, "--text", "wait for me"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(
        wait_until(Duration::from_secs(30), || fx.scenario_started()),
        "the scenario never started"
    );
    let live = status(&fx);
    let id = live["record"]["run_id"].as_str().unwrap().to_owned();
    let dir = run_dir(&fx, &id);

    let sent = run(&fx, &["send", ALIAS, "hello there"]);
    assert_eq!(sent.status.code(), Some(0), "{sent:?}");
    assert!(
        String::from_utf8_lossy(&sent.stdout).contains("delivered"),
        "{sent:?}"
    );

    let injections = lines_of(&dir.join("injections.jsonl"));
    assert_eq!(injections.len(), 1, "{injections:?}");
    assert_eq!(injections[0]["kind"], "message");
    assert_eq!(injections[0]["outcome"], "delivered");
    assert_eq!(injections[0]["text"], "hello there");
    assert!(injections[0]["id"].as_str().is_some_and(|i| !i.is_empty()));
    assert_eq!(
        fs::metadata(dir.join("injections.jsonl")).unwrap().mode() & 0o777,
        0o600
    );
    assert_eq!(
        fs::metadata(dir.join("inbox")).unwrap().mode() & 0o777,
        0o700
    );

    let done = wait_finished(&fx);
    assert_eq!(done["record"]["outcome"]["kind"], "finished", "{done}");
    let stream = fs::read_to_string(dir.join("stream.jsonl")).unwrap();
    assert!(
        stream
            .lines()
            .any(|l| l.contains("isReplay") && l.contains("hello there")),
        "{stream}"
    );
    assert!(fx.rec_dir().join("eof").exists());

    let late = run(&fx, &["send", ALIAS, "too late"]);
    assert_ne!(late.status.code(), Some(0), "{late:?}");
    assert!(
        String::from_utf8_lossy(&late.stderr).contains("not live"),
        "{late:?}"
    );
    assert_eq!(lines_of(&dir.join("injections.jsonl")).len(), 1);
}

#[test]
fn no_sandbox_bind_covers_the_inbox() {
    let fx = fixture();
    fx.write_scenario(STREAM_TURN_SCENARIO);
    let out = run(&fx, &["run", ALIAS, "--text", "x", "--foreground"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let id = status(&fx)["record"]["run_id"].as_str().unwrap().to_owned();
    let inbox = run_dir(&fx, &id).join("inbox");
    let shims = fx.shim_log();
    let line = shims
        .lines()
        .filter(|l| l.starts_with("bwrap "))
        .find(|l| l.contains("--tmpfs"))
        .unwrap_or_else(|| panic!("no bwrap line in {shims}"));
    let words: Vec<&str> = line.split(' ').collect();
    let mut binds = 0;
    let mut masked = false;
    for pair in words.windows(2) {
        let (flag, arg) = (pair[0], pair[1]);
        if flag == "--tmpfs" && inbox.starts_with(arg) {
            masked = true;
        }
        if [
            "--bind",
            "--ro-bind",
            "--bind-try",
            "--ro-bind-try",
            "--dev-bind",
        ]
        .contains(&flag)
        {
            binds += 1;
            if flag == "--ro-bind" && arg == "/" {
                // The read-only root: the run dir is hidden by the `--tmpfs` checked
                // below, and a read-only mount cannot take a request anyway.
                continue;
            }
            assert!(
                !inbox.starts_with(arg),
                "{arg} covers the inbox {}",
                inbox.display()
            );
        }
    }
    assert!(binds > 0, "no bind was parsed from {line}");
    assert!(
        masked,
        "the read-only root must be masked by a tmpfs over the run dir: {line}"
    );
}
