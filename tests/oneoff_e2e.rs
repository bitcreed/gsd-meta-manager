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
    prepared(Fixture::new())
}

/// `fx` with its project made a git repository holding one committed pending todo.
fn prepared(fx: Fixture) -> Fixture {
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
    // Fake scenarios that end a turn without writing a report must not wait the
    // production 120 s idle settle (every e2e launch goes through here).
    cmd.env("GSDMM_ONEOFF_IDLE_SETTLE_SECS", "1");
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
    // The default fixture lives under /tmp, so the data and config roots are not masked.
    let data_root = fx.scratch.path().join("data/gsd-meta-manager");
    assert!(
        !shims.contains(&format!("--tmpfs {}", data_root.display())),
        "{shims}"
    );
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
    // klf C-15: the landing records its range and clears the deleted branch.
    let landing = &after["record"]["landed"][0];
    assert_eq!(landing["how"], "ff", "{landing}");
    assert_eq!(landing["merge_sha"], record["head"], "{landing}");
    assert_eq!(
        landing["range"],
        format!(
            "{}..{}",
            record["base"].as_str().unwrap(),
            record["head"].as_str().unwrap()
        ),
        "the checkout was at the run's base: {landing}"
    );
    assert!(after["record"]["tracked_branch"].is_null(), "{after}");
    assert_eq!(after["record"]["tracking_note"], "deleted after landing");
    let text = run(&fx, &["status", ALIAS, "--run-id", &id]);
    let text = String::from_utf8_lossy(&text.stdout);
    assert!(
        text.contains("branch:    none (deleted after landing)"),
        "{text}"
    );
    assert!(text.contains("range "), "{text}");
    let discarded = run(&fx, &["land", "discard", ALIAS, &id]);
    assert_eq!(discarded.status.code(), Some(0), "{discarded:?}");
    assert!(!dir.exists());
}

fn status_of(fx: &Fixture) -> Value {
    status(fx)
}

#[test]
fn a_merge_is_refused_on_a_dirty_checkout() {
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
    // Nothing was merged, so the tracked branch stays.
    assert!(branch_head(fx.project(), &id).is_some());
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

/// Waits for `release`, then commits a file named after its own pid and reports done.
const PARALLEL_SCENARIO: &str = r#"
touch "$FAKE_CLAUDE_DIR/started-$$"
while [ ! -f "$FAKE_CLAUDE_DIR/release" ]; do sleep 0.1; done
G="git -c user.name=t -c user.email=t@example.com"
echo work > "file-$$.txt"
$G add -A
$G commit -qm "work $$"
gsd-meta-manager taskctl report --status done --headline "Did $$" --verified "true: ok"
printf '%s\n' '{"type":"result","subtype":"success"}'
"#;

/// How many scenarios have started (each touches `started-<pid>`).
fn started_count(fx: &Fixture) -> usize {
    fs::read_dir(fx.rec_dir())
        .map(|dir| {
            dir.flatten()
                .filter(|e| e.file_name().to_string_lossy().starts_with("started-"))
                .count()
        })
        .unwrap_or(0)
}

/// The run id `task run` printed.
fn started_id(out: &Output) -> String {
    let stdout = String::from_utf8_lossy(&out.stdout);
    stdout
        .split_whitespace()
        .find(|w| w.starts_with("oneoff-"))
        .unwrap_or_else(|| panic!("no run id in {out:?}"))
        .to_owned()
}

fn status_of_run(fx: &Fixture, id: &str) -> Value {
    let out = run(fx, &["status", ALIAS, "--run-id", id, "--json"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    serde_json::from_slice(&out.stdout).expect("status --json parses")
}

/// Wait until run `id` is finished and its lock is free.
fn wait_run_finished(fx: &Fixture, id: &str) -> Value {
    let mut last = Value::Null;
    let done = wait_until(Duration::from_secs(60), || {
        let out = run(fx, &["status", ALIAS, "--run-id", id, "--json"]);
        if !out.status.success() {
            return false;
        }
        last = serde_json::from_slice(&out.stdout).unwrap_or(Value::Null);
        last["live"] == "finished"
    });
    assert!(done, "run {id} never finished: {last}");
    last
}

fn branch_head(project: &Path, id: &str) -> Option<String> {
    let out = Command::new("git")
        .current_dir(project)
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .args([
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("refs/heads/gsd-oneoff/{id}^{{commit}}"),
        ])
        .output()
        .expect("git runs");
    out.status
        .success()
        .then(|| String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

#[test]
fn two_concurrent_runs_on_one_project_run_and_are_tracked() {
    let fx = fixture();
    fx.write_scenario(PARALLEL_SCENARIO);
    let spawn = |text: &str| {
        task(&fx, &["run", ALIAS, "--text", text])
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .spawn()
            .unwrap()
    };
    // Both are started at the same moment: the run dir is claimed by an atomic mkdir.
    let (first, second) = (spawn("alpha"), spawn("beta"));
    let first = first.wait_with_output().unwrap();
    let second = second.wait_with_output().unwrap();
    assert_eq!(first.status.code(), Some(0), "{first:?}");
    assert_eq!(second.status.code(), Some(0), "{second:?}");
    let (a, b) = (started_id(&first), started_id(&second));
    assert_ne!(a, b, "two simultaneous launches shared an id");
    assert!(
        wait_until(Duration::from_secs(30), || started_count(&fx) == 2),
        "both scenarios must be running at once"
    );
    for id in [&a, &b] {
        assert_eq!(status_of_run(&fx, id)["live"], "running", "{id}");
    }

    // No verb guesses between live runs.
    let status = run(&fx, &["status", ALIAS]);
    assert_eq!(status.status.code(), Some(1), "{status:?}");
    let stop = run(&fx, &["stop", ALIAS]);
    assert_eq!(stop.status.code(), Some(1), "{stop:?}");
    for out in [&status, &stop] {
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(stderr.contains(&a) && stderr.contains(&b), "{out:?}");
        assert!(stderr.contains("--run-id"), "{out:?}");
    }
    let listed = run(&fx, &["list", ALIAS, "--json"]);
    assert_eq!(listed.status.code(), Some(0), "{listed:?}");
    let listed: Value = serde_json::from_slice(&listed.stdout).unwrap();
    let ids: Vec<&str> = listed
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["record"]["run_id"].as_str().unwrap())
        .collect();
    assert_eq!(ids.len(), 2, "{listed}");
    assert!(ids.contains(&a.as_str()) && ids.contains(&b.as_str()));

    fs::write(fx.rec_dir().join("release"), "").unwrap();
    for id in [&a, &b] {
        let done = wait_run_finished(&fx, id);
        let head = done["record"]["head"].as_str().unwrap();
        assert_eq!(done["record"]["commits"], 1, "{done}");
        assert_eq!(
            done["record"]["tracked_branch"],
            format!("refs/heads/gsd-oneoff/{id}"),
            "{done}"
        );
        assert_eq!(branch_head(fx.project(), id).as_deref(), Some(head), "{id}");
    }
    let unmerged = git_text(fx.project(), &["branch", "--no-merged"]);
    assert!(
        unmerged.contains(&format!("gsd-oneoff/{a}"))
            && unmerged.contains(&format!("gsd-oneoff/{b}")),
        "{unmerged}"
    );

    // A fast-forward landing deletes the tracked branch of the run it landed.
    let merged = run(&fx, &["land", "merge", ALIAS, &a]);
    assert_eq!(merged.status.code(), Some(0), "{merged:?}");
    assert_eq!(branch_head(fx.project(), &a), None, "{merged:?}");
    assert!(branch_head(fx.project(), &b).is_some());
    let a_after = status_of_run(&fx, &a);
    assert!(a_after["record"]["tracked_branch"].is_null(), "{a_after}");
    assert_eq!(a_after["record"]["tracking_note"], "deleted after landing");
    let landing = &a_after["record"]["landed"][0];
    assert_eq!(landing["how"], "ff");
    assert_eq!(landing["merge_sha"], a_after["record"]["head"]);
    assert!(landing["range"]
        .as_str()
        .unwrap()
        .ends_with(a_after["record"]["head"].as_str().unwrap()));
    let b_after = status_of_run(&fx, &b);
    assert_eq!(
        b_after["record"]["tracked_branch"],
        format!("refs/heads/gsd-oneoff/{b}"),
        "an unlanded run keeps its branch"
    );
    // The finished runs no longer need an id: the newest is the default.
    assert_eq!(run(&fx, &["status", ALIAS]).status.code(), Some(0));
}

#[test]
fn a_finished_run_is_tracked_as_a_branch_and_discard_deletes_it() {
    let fx = fixture();
    let (id, status) = quick_run(&fx);
    let head = status["record"]["head"].as_str().unwrap().to_owned();
    assert_eq!(
        branch_head(fx.project(), &id).as_deref(),
        Some(head.as_str())
    );
    assert_eq!(
        status["record"]["tracked_branch"],
        format!("refs/heads/gsd-oneoff/{id}")
    );
    let shown = run(&fx, &["status", ALIAS]);
    assert!(
        String::from_utf8_lossy(&shown.stdout)
            .contains(&format!("branch:    gsd-oneoff/{id} @ {}", &head[..12])),
        "{shown:?}"
    );
    let discarded = run(&fx, &["land", "discard", ALIAS, &id]);
    assert_eq!(discarded.status.code(), Some(0), "{discarded:?}");
    assert!(
        String::from_utf8_lossy(&discarded.stdout).contains("deleted branch"),
        "{discarded:?}"
    );
    assert_eq!(branch_head(fx.project(), &id), None);
    assert!(!run_dir(&fx, &id).exists());

    // A branch that no longer points at the recorded head is left alone.
    let (moved, _) = quick_run(&fx);
    let init = git_text(fx.project(), &["rev-parse", "HEAD"]);
    git(
        fx.project(),
        &[
            "update-ref",
            &format!("refs/heads/gsd-oneoff/{moved}"),
            &init,
        ],
    );
    let left = run(&fx, &["land", "discard", ALIAS, &moved]);
    assert_eq!(left.status.code(), Some(0), "{left:?}");
    assert!(
        String::from_utf8_lossy(&left.stdout).contains(&format!("left branch gsd-oneoff/{moved}")),
        "{left:?}"
    );
    assert_eq!(
        branch_head(fx.project(), &moved).as_deref(),
        Some(init.as_str())
    );
    assert!(!run_dir(&fx, &moved).exists());
}

#[test]
fn a_run_without_commits_has_no_branch_and_says_why() {
    let fx = fixture();
    fx.write_scenario("printf '%s\\n' '{\"type\":\"result\",\"subtype\":\"success\"}'\n");
    let out = run(&fx, &["run", ALIAS, "--text", "nothing", "--foreground"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let record = &status(&fx)["record"];
    assert!(record["tracked_branch"].is_null(), "{record}");
    assert!(
        record["tracking_note"]
            .as_str()
            .is_some_and(|n| n.contains("no commits")),
        "{record}"
    );
    let id = record["run_id"].as_str().unwrap();
    assert_eq!(branch_head(fx.project(), id), None);
    assert!(git_text(fx.project(), &["branch", "--list", "gsd-oneoff/*"]).is_empty());
}

#[test]
fn a_second_run_starts_while_one_is_live_and_stop_takes_a_run_id() {
    let fx = fixture();
    fx.write_scenario(PARALLEL_SCENARIO);
    let mut child = task(&fx, &["run", ALIAS, "--text", "first", "--foreground"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    assert!(
        wait_until(Duration::from_secs(30), || started_count(&fx) == 1),
        "the first scenario never started"
    );
    let live = status(&fx);
    let first = live["record"]["run_id"].as_str().unwrap().to_owned();
    assert_eq!(live["live"], "running");

    let second = run(&fx, &["run", ALIAS, "--text", "second"]);
    assert_eq!(second.status.code(), Some(0), "{second:?}");
    let second = started_id(&second);
    assert_ne!(first, second);
    assert!(
        wait_until(Duration::from_secs(30), || started_count(&fx) == 2),
        "the second scenario never started"
    );

    let stopped = run(&fx, &["stop", ALIAS, "--run-id", &first]);
    assert_eq!(stopped.status.code(), Some(0), "{stopped:?}");
    assert!(
        String::from_utf8_lossy(&stopped.stdout).contains(&first),
        "{stopped:?}"
    );
    let exited = wait_until(Duration::from_secs(30), || {
        matches!(child.try_wait(), Ok(Some(_)))
    });
    if !exited {
        let _ = child.kill();
    }
    assert!(exited, "the foreground run did not exit after stop");
    let after = status_of_run(&fx, &first);
    assert_eq!(after["record"]["outcome"]["kind"], "stopped", "{after}");
    let stop = &after["record"]["stop"];
    assert_eq!(stop["kind"], "hard", "{after}");
    assert_eq!(stop["by"], "cli", "{after}");
    assert_eq!(stop["escalated"], false, "{after}");
    assert!(stop["ended_at"].is_string(), "{after}");
    assert_eq!(after["live"], "finished");
    assert_eq!(
        status_of_run(&fx, &second)["live"],
        "running",
        "stopping one run must not touch the other"
    );
    // One live run left: no id is needed any more, and it is the one picked.
    let picked = status(&fx);
    assert_eq!(picked["record"]["run_id"], second.as_str(), "{picked}");

    fs::write(fx.rec_dir().join("release"), "").unwrap();
    let done = wait_run_finished(&fx, &second);
    assert_eq!(done["record"]["outcome"]["kind"], "finished", "{done}");
}

#[test]
fn a_data_root_outside_home_is_masked_in_the_sandbox() {
    let base = PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("hx7-mask");
    let mut fx = Fixture::new_in(&base);
    // The socket path limit (107 bytes) needs a short runtime dir.
    let runtime = tempfile::TempDir::new().unwrap();
    fs::set_permissions(
        runtime.path(),
        std::os::unix::fs::PermissionsExt::from_mode(0o700),
    )
    .unwrap();
    fx.runtime_dir = runtime.path().to_path_buf();
    let fx = prepared(fx);
    fx.write_scenario(QUICK_SCENARIO);
    let out = run(&fx, &["run", ALIAS, "--todo", TODO, "--foreground"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let id = status(&fx)["record"]["run_id"].as_str().unwrap().to_owned();
    let work = run_dir(&fx, &id).join("work").display().to_string();
    let shims = fx.shim_log();
    let line = shims
        .lines()
        .find(|l| l.starts_with("bwrap ") && l.contains("--tmpfs"))
        .unwrap_or_else(|| panic!("no bwrap line in {shims}"));
    let at = |needle: String| {
        line.find(&needle)
            .unwrap_or_else(|| panic!("{needle:?} missing from {line}"))
    };
    let data = at(format!(
        " --tmpfs {}",
        fx.scratch.path().join("data/gsd-meta-manager").display()
    ));
    let config = at(format!(
        " --tmpfs {}",
        fx.scratch.path().join("config/gsd-meta-manager").display()
    ));
    let bind = at(format!(" --bind {work} {work}"));
    assert!(data < bind && config < bind, "{line}");
    assert!(data < config, "{line}");
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

/// One background task outstanding at the first result: the session must stay open
/// for a message, then end only after the final result with a report (quick 261103-mnr).
const BACKGROUND_SCENARIO: &str = r#"
printf '%s\n' '{"type":"system","subtype":"init","model":"fake"}'
printf '%s\n' '{"type":"system","subtype":"background_tasks_changed","tasks":[{"task_id":"bg1"}]}'
printf '%s\n' '{"type":"system","subtype":"task_started","task_id":"bg1","is_backgrounded":true}'
printf '%s\n' '{"type":"result","subtype":"success","total_cost_usd":0.1,"num_turns":1,"queued_turn_count":0,"subagent_stats":{"spawned":1,"completed":0,"failed":0,"killed":{"parent":0,"system":0,"user":0}}}'
touch "$FAKE_CLAUDE_DIR/started"
IFS= read -r line
printf '%s\n' "$line" | sed 's/^{/{"isReplay":true,/'
printf '%s\n' '{"type":"system","subtype":"task_notification","task_id":"bg1","status":"completed"}'
printf '%s\n' '{"type":"system","subtype":"background_tasks_changed","tasks":[]}'
printf '%s\n' '{"type":"system","subtype":"init","model":"fake"}'
gsd-meta-manager taskctl report --status done --headline "Background work done"
printf '%s\n' '{"type":"result","subtype":"success","total_cost_usd":0.2,"num_turns":2,"queued_turn_count":0,"subagent_stats":{"spawned":1,"completed":1,"failed":0,"killed":{"parent":0,"system":0,"user":0}}}'
while IFS= read -r line; do :; done
touch "$FAKE_CLAUDE_DIR/eof"
"#;

#[test]
fn background_work_keeps_the_session_open_for_messages() {
    let fx = fixture();
    fx.write_scenario(BACKGROUND_SCENARIO);
    let out = run(&fx, &["run", ALIAS, "--text", "do the background thing"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(
        wait_until(Duration::from_secs(30), || fx.scenario_started()),
        "the scenario never started"
    );
    let live = status(&fx);
    let id = live["record"]["run_id"].as_str().unwrap().to_owned();
    let dir = run_dir(&fx, &id);

    // Three idle settles (1 s each) with a task outstanding and no report: still open.
    std::thread::sleep(Duration::from_secs(3));
    assert!(
        !fx.rec_dir().join("eof").exists(),
        "the input was closed while a background task was outstanding"
    );

    let sent = run(&fx, &["send", ALIAS, "status check"]);
    assert_eq!(sent.status.code(), Some(0), "{sent:?}");
    assert!(
        String::from_utf8_lossy(&sent.stdout).contains("delivered"),
        "{sent:?}"
    );

    let done = wait_finished(&fx);
    assert_eq!(done["record"]["outcome"]["kind"], "finished", "{done}");
    assert_eq!(done["record"]["report"]["status"], "done", "{done}");
    assert!(fx.rec_dir().join("eof").exists());
    let injections = lines_of(&dir.join("injections.jsonl"));
    let messages: Vec<&Value> = injections
        .iter()
        .filter(|r| r["kind"] == "message")
        .collect();
    assert_eq!(messages.len(), 1, "{injections:?}");
    assert_eq!(messages[0]["outcome"], "delivered");
    assert!(
        !injections.iter().any(|r| r["outcome"] == "refused"),
        "{injections:?}"
    );
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

// ---- quick 261103-gt1 Task 3: the soft cap, task budget and task attach ----

/// One assistant line that prices at $4 (1M opus-5-5 input tokens), then replays every
/// stdin line forever and never ends its turn.
const SOFT_CAP_SCENARIO: &str = r#"
touch "$FAKE_CLAUDE_DIR/started"
printf '%s\n' '{"type":"assistant","message":{"id":"m1","model":"claude-opus-5-5","content":[{"type":"text","text":"working"}],"usage":{"input_tokens":1000000,"output_tokens":0}},"parent_tool_use_id":null}'
while IFS= read -r line; do
  printf '%s\n' "$line" | sed 's/^{/{"isReplay":true,/'
done
"#;

#[test]
fn a_soft_cap_wraps_up_once_then_stops_the_run_as_over_budget() {
    let fx = fixture();
    fx.write_scenario(SOFT_CAP_SCENARIO);
    let out = task(&fx, &["run", ALIAS, "--text", "spend"])
        .env("GSDMM_ONEOFF_SOFT_CAP_GRACE_SECS", "1")
        .output()
        .expect("the binary runs");
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(
        wait_until(Duration::from_secs(30), || fx.scenario_started()),
        "the scenario never started"
    );
    let id = status(&fx)["record"]["run_id"].as_str().unwrap().to_owned();
    let dir = run_dir(&fx, &id);

    let too_high = run(&fx, &["budget", ALIAS, "--soft", "20"]);
    assert_eq!(too_high.status.code(), Some(1), "{too_high:?}");
    assert!(
        String::from_utf8_lossy(&too_high.stderr).contains("hard ceiling"),
        "{too_high:?}"
    );

    let set = run(&fx, &["budget", ALIAS, "--soft", "1"]);
    assert_eq!(set.status.code(), Some(0), "{set:?}");
    assert!(
        String::from_utf8_lossy(&set.stdout).contains("delivered"),
        "{set:?}"
    );

    let done = wait_finished(&fx);
    let record = &done["record"];
    assert_eq!(record["outcome"]["kind"], "over_budget", "{done}");
    assert_eq!(record["soft_cap_usd"], 1.0, "{record}");
    assert!(record["soft_cap_hit_at"].is_string(), "{record}");

    let injections = lines_of(&dir.join("injections.jsonl"));
    let kinds: Vec<&str> = injections
        .iter()
        .map(|i| i["kind"].as_str().unwrap_or(""))
        .collect();
    assert_eq!(
        kinds.iter().filter(|k| **k == "wrap_up").count(),
        1,
        "{injections:?}"
    );
    assert!(kinds.contains(&"soft_cap"), "{injections:?}");
    let wrap = injections.iter().find(|i| i["kind"] == "wrap_up").unwrap();
    assert_eq!(wrap["outcome"], "delivered");
    assert!(wrap["text"].as_str().unwrap().starts_with("Budget reached"));
    let stream = fs::read_to_string(dir.join("stream.jsonl")).unwrap();
    assert!(
        stream
            .lines()
            .any(|l| l.contains("isReplay") && l.contains("Budget reached")),
        "{stream}"
    );
}

/// The real rate-limit shape, one assistant line that prices at $4, then a wait for
/// `release` and a result that reports $4.50.
const LIVE_SPEND_SCENARIO: &str = r#"
printf '%s\n' '{"type":"rate_limit_event","rate_limit_info":{"status":"allowed","rateLimitType":"seven_day","resetsAt":1794326400,"unifiedWindows":{"seven_day":{"status":"allowed","resetsAt":1794326400,"utilization":0.21}}}}'
printf '%s\n' '{"type":"assistant","message":{"id":"m1","model":"claude-opus-5-5","content":[{"type":"text","text":"working"}],"usage":{"input_tokens":1000000,"output_tokens":0}},"parent_tool_use_id":null}'
touch "$FAKE_CLAUDE_DIR/started"
while [ ! -f "$FAKE_CLAUDE_DIR/release" ]; do sleep 0.1; done
printf '%s\n' '{"type":"result","subtype":"success","total_cost_usd":4.5,"num_turns":1}'
"#;

#[test]
fn task_status_shows_live_spend_and_the_rate_limit_of_a_running_run() {
    let fx = fixture();
    fx.write_scenario(LIVE_SPEND_SCENARIO);
    let out = run(&fx, &["run", ALIAS, "--text", "spend"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(
        wait_until(Duration::from_secs(30), || fx.scenario_started()),
        "the scenario never started"
    );
    // The capture pump may lag the scenario: wait until the stream shows the spend.
    let mut json = Value::Null;
    assert!(
        wait_until(Duration::from_secs(30), || {
            json = status(&fx);
            json["live_spend_usd"].as_f64().is_some_and(|v| v >= 4.0)
        }),
        "no live spend in {json}"
    );
    assert_eq!(json["live_spend_is_lower_bound"], true, "{json}");
    assert_eq!(json["rate_limit"]["kind"], "seven_day", "{json}");
    assert_eq!(json["rate_limit"]["status"], "allowed", "{json}");
    assert_eq!(json["rate_limit"]["utilization"], 0.21, "{json}");
    assert_eq!(json["rate_limit"]["resets_at"], 1_794_326_400_i64, "{json}");

    let text = run(&fx, &["status", ALIAS]);
    assert_eq!(text.status.code(), Some(0), "{text:?}");
    let text = String::from_utf8_lossy(&text.stdout).into_owned();
    assert!(text.contains("≥ $4.00 so far"), "{text}");
    assert!(text.contains("lower bound"), "{text}");
    assert!(text.contains("rate:      allowed seven_day 21%"), "{text}");

    fx.release();
    let done = wait_finished(&fx);
    assert_eq!(done["record"]["outcome"]["kind"], "finished", "{done}");
    assert!(done.get("live_spend_usd").is_none(), "{done}");
    assert!(done.get("rate_limit").is_none(), "{done}");
    let text = run(&fx, &["status", ALIAS]);
    let text = String::from_utf8_lossy(&text.stdout).into_owned();
    let spend = text
        .lines()
        .find(|l| l.starts_with("spend:"))
        .unwrap_or_else(|| panic!("no spend line in {text}"));
    assert!(spend.contains("$4.50"), "{spend}");
    assert!(!spend.contains('≥'), "{spend}");
}

#[test]
fn attach_prints_a_finished_run() {
    let fx = fixture();
    fx.write_scenario(STREAM_TURN_SCENARIO);
    let out = run(&fx, &["run", ALIAS, "--text", "quick one", "--foreground"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let finished = run(&fx, &["attach", ALIAS, "--no-input"]);
    assert_eq!(finished.status.code(), Some(0), "{finished:?}");
    let text = String::from_utf8_lossy(&finished.stdout);
    assert!(text.contains("prompt: "), "{text}");
    assert!(text.contains("turn done"), "{text}");
    assert!(
        String::from_utf8_lossy(&run(&fx, &["attach", "nope", "--no-input"]).stderr)
            .contains("task attach"),
    );
}

#[test]
fn attach_turns_stdin_lines_into_injected_messages() {
    use std::io::Write;
    let fx = fixture();
    fx.write_scenario(INJECT_SCENARIO);
    let out = run(&fx, &["run", ALIAS, "--text", "wait for me"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(
        wait_until(Duration::from_secs(30), || fx.scenario_started()),
        "the scenario never started"
    );
    let id = status(&fx)["record"]["run_id"].as_str().unwrap().to_owned();
    let mut child = task(&fx, &["attach", ALIAS])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    {
        let stdin = child.stdin.as_mut().unwrap();
        stdin.write_all(b"hi\n").unwrap();
        stdin.flush().unwrap();
    }
    let exited = wait_until(Duration::from_secs(40), || {
        matches!(child.try_wait(), Ok(Some(_)))
    });
    if !exited {
        let _ = child.kill();
    }
    assert!(exited, "attach never exited after the run finished");
    let output = child.wait_with_output().unwrap();
    assert_eq!(output.status.code(), Some(0), "{output:?}");
    assert!(
        String::from_utf8_lossy(&output.stdout).contains("you: hi"),
        "{output:?}"
    );
    let injections = lines_of(&run_dir(&fx, &id).join("injections.jsonl"));
    assert!(
        injections
            .iter()
            .any(|i| i["text"] == "hi" && i["outcome"] == "delivered"),
        "{injections:?}"
    );
}

// ---------------------------------------------------------------- merge landing
// (quick 261103-hx7, Task 2). The fake agent runs ONE scenario for the run and for a
// conflict resolver alike; the resolver's cwd is mid-merge (MERGE_HEAD exists), the
// run's is not.

/// Give the project a repo-local git identity: the host merges with the project's own.
fn with_identity(fx: Fixture) -> Fixture {
    git(fx.project(), &["config", "user.name", "t"]);
    git(fx.project(), &["config", "user.email", "t@example.com"]);
    fx
}

/// A project whose base commit has `shared.txt`, for a modify/modify conflict.
fn conflict_fixture() -> Fixture {
    let fx = with_identity(fixture());
    fs::write(fx.project().join("shared.txt"), "base\n").unwrap();
    git(fx.project(), &["add", "shared.txt"]);
    git(fx.project(), &["commit", "-qm", "base shared"]);
    fx
}

/// The scenario for a run that edits `shared.txt`, and a resolver that is `resolver`.
fn merge_scenario(resolver: &str) -> String {
    format!(
        r#"
G="git -c user.name=t -c user.email=t@example.com"
if git rev-parse -q --verify MERGE_HEAD >/dev/null 2>&1; then
{resolver}
else
  echo run-side > shared.txt
  $G add -A
  $G commit -qm "run edits shared"
  gsd-meta-manager taskctl report --status done --headline "Edited shared" --verified "true: ok"
  printf '%s\n' '{{"type":"result","subtype":"success","total_cost_usd":1.5}}'
fi
"#
    )
}

/// A resolver that resolves, commits the merge and reports done.
const RESOLVES: &str = r#"
  printf 'merged\n' > shared.txt
  $G add shared.txt
  $G commit -q --no-edit
  gsd-meta-manager taskctl report --status done --headline resolved --verified "true: ok"
  printf '%s\n' '{"type":"result","subtype":"success","total_cost_usd":0.5}'
"#;

/// Start one run with `scenario` and wait for it; the project then edits `shared.txt`
/// differently and commits, so the run's head is no fast-forward and conflicts.
fn conflicting_run(fx: &Fixture, scenario: &str) -> (String, Value, String) {
    fx.write_scenario(scenario);
    let out = run(fx, &["run", ALIAS, "--text", "edit shared"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let id = started_id(&out);
    let status = wait_run_finished(fx, &id);
    assert_eq!(status["record"]["commits"], 1, "{status}");
    fs::write(fx.project().join("shared.txt"), "project-side\n").unwrap();
    git(fx.project(), &["add", "shared.txt"]);
    git(fx.project(), &["commit", "-qm", "project edits shared"]);
    let tip = git_text(fx.project(), &["rev-parse", "HEAD"]);
    (id, status, tip)
}

/// `git rev-list --parents -n 1 HEAD` of the project, as fields.
fn head_parents(project: &Path) -> Vec<String> {
    git_text(project, &["rev-list", "--parents", "-n", "1", "HEAD"])
        .split_whitespace()
        .map(str::to_owned)
        .collect()
}

#[test]
fn a_non_conflicting_merge_lands_as_a_merge_commit() {
    let fx = with_identity(fixture());
    let (id, status) = quick_run(&fx);
    let head = status["record"]["head"].as_str().unwrap().to_owned();
    // The project moves on with an unrelated file: the run's head is no fast-forward.
    fs::write(fx.project().join("unrelated.txt"), "x").unwrap();
    git(fx.project(), &["add", "unrelated.txt"]);
    git(fx.project(), &["commit", "-qm", "moved on"]);
    let tip = git_text(fx.project(), &["rev-parse", "HEAD"]);

    let merged = run(&fx, &["land", "merge", ALIAS, &id]);
    assert_eq!(merged.status.code(), Some(0), "{merged:?}");
    let parents = head_parents(fx.project());
    assert_eq!(parents[1..], [tip, head], "{parents:?}");
    let subject = git_text(fx.project(), &["log", "-1", "--format=%s"]);
    assert!(
        subject.starts_with(&format!("Merge one-off {id} (")),
        "{subject}"
    );
    assert!(fx.project().join("unrelated.txt").exists());
    assert!(fx
        .project()
        .join(".planning/todos/completed")
        .join(TODO)
        .exists());

    let after = status_of_run(&fx, &id);
    let landing = after["record"]["landed"]
        .as_array()
        .unwrap()
        .last()
        .unwrap();
    assert_eq!(landing["how"], "merge", "{after}");
    assert_eq!(landing["merge_sha"], parents[0].as_str());
    assert_eq!(
        landing["range"],
        format!("{}..{}", parents[1], parents[0]),
        "range = target tip..merge commit: {landing}"
    );
    assert!(after["record"]["tracked_branch"].is_null(), "{after}");
    assert_eq!(after["record"]["tracking_note"], "deleted after landing");
    let text_status = run(&fx, &["status", ALIAS, "--run-id", &id]);
    let text_status = String::from_utf8_lossy(&text_status.stdout);
    let sha_full = parents[0].as_str();
    assert!(
        text_status.contains(&format!("diff {sha_full}^1 {sha_full}")),
        "{text_status}"
    );
    assert!(
        !run_dir(&fx, &id).join("land/1/stream.jsonl").exists(),
        "a clean merge has no resolver"
    );
    assert_eq!(
        branch_head(fx.project(), &id),
        None,
        "the branch is deleted"
    );
    let text = String::from_utf8_lossy(&merged.stdout);
    assert!(text.contains("merge commit"), "{text}");

    // The TUI's `d` on a clean merge: no conflicted files, the run's own changes.
    let sha = landing["merge_sha"].as_str().unwrap();
    let diff = gsd_meta_manager::oneoff::land::merge_diff(fx.project(), sha, &[])
        .expect("the merge diff reads");
    assert_eq!(diff[0], "conflicted: none (a clean merge)", "{diff:?}");
    assert!(
        diff.len() > 3,
        "the run's changes follow the header: {diff:?}"
    );
    // klf: header, then the per-file summary, a blank line, then the hunks.
    let blank = diff
        .iter()
        .position(String::is_empty)
        .expect("a blank line");
    assert!(blank >= 2, "a summary follows the header: {diff:?}");
    assert!(diff[1].starts_with("+1 -0  "), "{diff:?}");
    assert!(
        diff[blank + 1..]
            .iter()
            .any(|l| l.starts_with("diff --git ")),
        "{diff:?}"
    );
    assert!(!diff.last().unwrap().starts_with("truncated"), "{diff:?}");
    assert!(
        !diff.join("\n").contains("unrelated.txt"),
        "the target's own commit is not in the diff against the first parent"
    );
}

#[test]
fn a_conflicting_merge_is_resolved_by_the_agent_and_lands() {
    let fx = conflict_fixture();
    let (id, status, tip) = conflicting_run(&fx, &merge_scenario(RESOLVES));
    let head = status["record"]["head"].as_str().unwrap().to_owned();

    let merged = run(&fx, &["land", "merge", ALIAS, &id]);
    assert_eq!(merged.status.code(), Some(0), "{merged:?}");
    let parents = head_parents(fx.project());
    assert_eq!(parents[1..], [tip, head], "{parents:?}");
    assert_eq!(
        fs::read_to_string(fx.project().join("shared.txt")).unwrap(),
        "merged\n"
    );
    assert_eq!(git_text(fx.project(), &["status", "--porcelain"]), "");

    let after = status_of_run(&fx, &id);
    let landing = after["record"]["landed"]
        .as_array()
        .unwrap()
        .last()
        .unwrap();
    assert_eq!(landing["how"], "resolved", "{after}");
    assert_eq!(landing["conflicted"], serde_json::json!(["shared.txt"]));
    assert_eq!(landing["attempts"], serde_json::json!(["land/1"]));
    assert_eq!(landing["merge_sha"], parents[0].as_str());
    assert!(after["record"]["land_failure"].is_null(), "{after}");
    let attempt = run_dir(&fx, &id).join("land/1");
    assert!(attempt.join("stream.jsonl").is_file());
    assert!(attempt.join("report/report.json").is_file());
    assert!(attempt.join("launch.log").is_file());
    // It ran inside the sandbox profile, in a scope of its own.
    let shims = fx.shim_log();
    assert!(shims.contains(&format!("{id}-land1")), "{shims}");
    // The resolver's budget is the run's remaining budget: 10 - 1.5.
    let argv = fx.recorded("argv.txt");
    let lines: Vec<&str> = argv.lines().collect();
    let at = lines.iter().position(|l| *l == "--max-budget-usd").unwrap();
    assert_eq!(lines[at + 1], "8.5", "{argv}");
    assert!(fx.recorded("prompt.txt").contains("shared.txt"));
    // Its spend is added to the run's: 1.5 + 0.5.
    let spent = after["record"]["spend_usd"].as_f64().unwrap();
    assert!((spent - 2.0).abs() < 1e-6, "{spent}");
    assert_eq!(
        branch_head(fx.project(), &id),
        None,
        "the branch is deleted"
    );
    let text = String::from_utf8_lossy(&merged.stdout);
    assert!(text.contains("resolved by agent"), "{text}");
    // `task status` prints how it landed.
    let human = run(&fx, &["status", ALIAS, "--run-id", &id]);
    let human = String::from_utf8_lossy(&human.stdout);
    assert!(
        human.contains("resolved by agent; conflicted: shared.txt"),
        "{human}"
    );
    assert!(human.contains("[resolved]"), "{human}");

    // The TUI's `d`: the merge against its first parent, headed by the conflicted files.
    let sha = landing["merge_sha"].as_str().unwrap();
    let diff =
        gsd_meta_manager::oneoff::land::merge_diff(fx.project(), sha, &["shared.txt".to_owned()])
            .expect("the merge diff reads");
    assert_eq!(diff[0], "conflicted: shared.txt", "{diff:?}");
    let body = diff.join("\n");
    assert!(
        body.contains("diff --git a/shared.txt b/shared.txt"),
        "{body}"
    );
    assert!(
        body.contains("-project-side") && body.contains("+merged"),
        "{body}"
    );
    // A sha that is not one is refused before git runs.
    assert!(gsd_meta_manager::oneoff::land::merge_diff(fx.project(), "HEAD", &[]).is_err());
}

/// Every landing that fails leaves the checkout and the branch as they were and says why.
fn assert_nothing_landed(fx: &Fixture, id: &str, tip: &str, out: &Output) -> Value {
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert_eq!(git_text(fx.project(), &["rev-parse", "HEAD"]), tip);
    assert_eq!(
        fs::read_to_string(fx.project().join("shared.txt")).unwrap(),
        "project-side\n"
    );
    assert_eq!(git_text(fx.project(), &["status", "--porcelain"]), "");
    assert!(branch_head(fx.project(), id).is_some(), "the branch stays");
    let after = status_of_run(fx, id);
    assert!(
        after["record"]["landed"].as_array().unwrap().is_empty(),
        "{after}"
    );
    after
}

#[test]
fn a_failed_resolution_lands_nothing_and_keeps_the_branch() {
    let fx = conflict_fixture();
    let resolver = r#"
  gsd-meta-manager taskctl report --status failed --headline "could not resolve"
  printf '%s\n' '{"type":"result","subtype":"success","total_cost_usd":0.5}'
"#;
    let (id, _, tip) = conflicting_run(&fx, &merge_scenario(resolver));
    let out = run(&fx, &["land", "merge", ALIAS, &id]);
    let after = assert_nothing_landed(&fx, &id, &tip, &out);
    let failure = &after["record"]["land_failure"];
    let reason = failure["reason"].as_str().unwrap();
    assert!(
        reason.contains("report") && reason.contains("failed"),
        "{reason}"
    );
    assert_eq!(failure["conflicted"], serde_json::json!(["shared.txt"]));
    assert_eq!(failure["attempts"], serde_json::json!(["land/1"]));
    assert!(
        run_dir(&fx, &id).join("land/1/stream.jsonl").is_file(),
        "kept"
    );
    // The resolver's spend still counts against the run.
    assert!((after["record"]["spend_usd"].as_f64().unwrap() - 2.0).abs() < 1e-6);
    let human = run(&fx, &["status", ALIAS, "--run-id", &id]);
    assert!(
        String::from_utf8_lossy(&human.stdout).contains("land failed:"),
        "{human:?}"
    );
}

#[test]
fn leftover_conflict_markers_land_nothing() {
    let fx = conflict_fixture();
    // The resolver commits the file with its markers still in it and reports done.
    let resolver = r#"
  $G add shared.txt
  $G commit -q --no-edit
  gsd-meta-manager taskctl report --status done --headline resolved --verified "true: ok"
  printf '%s\n' '{"type":"result","subtype":"success","total_cost_usd":0.5}'
"#;
    let (id, _, tip) = conflicting_run(&fx, &merge_scenario(resolver));
    let out = run(&fx, &["land", "merge", ALIAS, &id]);
    let after = assert_nothing_landed(&fx, &id, &tip, &out);
    let reason = after["record"]["land_failure"]["reason"].as_str().unwrap();
    assert!(reason.contains("conflict markers"), "{reason}");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("conflict markers"),
        "{out:?}"
    );
}

/// A resolver that also moves the project's branch: on its first invocation only when
/// `once`, every time otherwise.
fn moving_resolver(project: &Path, once: bool) -> String {
    let guard = if once {
        r#"if [ ! -f "$FAKE_CLAUDE_DIR/moved-once" ]; then touch "$FAKE_CLAUDE_DIR/moved-once"; MOVE=1; fi"#
    } else {
        "MOVE=1"
    };
    format!(
        r#"
  MOVE=0
  {guard}
  if [ "$MOVE" = 1 ]; then
    P="{project}"
    echo moved > "$P/moved-$$.txt"
    git -C "$P" add "moved-$$.txt"
    git -C "$P" -c user.name=t -c user.email=t@example.com commit -qm "moved $$"
  fi
{RESOLVES}"#,
        project = project.display()
    )
}

#[test]
fn a_moving_target_is_merged_again_on_the_new_tip() {
    let fx = conflict_fixture();
    let scenario = merge_scenario(&moving_resolver(fx.project(), true));
    let (id, status, _) = conflicting_run(&fx, &scenario);
    let head = status["record"]["head"].as_str().unwrap().to_owned();

    let out = run(&fx, &["land", "merge", ALIAS, &id]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let parents = head_parents(fx.project());
    assert_eq!(parents[2], head, "{parents:?}");
    // The first parent is the commit the first resolver moved the project to.
    let moved = git_text(fx.project(), &["rev-parse", "HEAD^1"]);
    assert_eq!(
        git_text(fx.project(), &["log", "-1", "--format=%s", &moved])
            .split(' ')
            .next(),
        Some("moved")
    );
    let after = status_of_run(&fx, &id);
    let landing = after["record"]["landed"]
        .as_array()
        .unwrap()
        .last()
        .unwrap();
    assert_eq!(landing["how"], "resolved", "{after}");
    assert_eq!(landing["attempts"], serde_json::json!(["land/1", "land/2"]));
    assert!(run_dir(&fx, &id).join("land/2/stream.jsonl").is_file());
    // Both resolvers' spend counts: 1.5 + 0.5 + 0.5.
    assert!((after["record"]["spend_usd"].as_f64().unwrap() - 2.5).abs() < 1e-6);
}

#[test]
fn a_target_that_keeps_moving_is_refused_after_three_attempts() {
    let fx = conflict_fixture();
    let scenario = merge_scenario(&moving_resolver(fx.project(), false));
    let (id, _, _) = conflicting_run(&fx, &scenario);
    let before = git_text(fx.project(), &["rev-parse", "HEAD"]);

    let out = run(&fx, &["land", "merge", ALIAS, &id]);
    assert_eq!(out.status.code(), Some(1), "{out:?}");
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("the target kept moving"),
        "{out:?}"
    );
    for n in 1..=3 {
        assert!(
            run_dir(&fx, &id).join(format!("land/{n}")).is_dir(),
            "land/{n}"
        );
    }
    assert!(
        !run_dir(&fx, &id).join("land/4").exists(),
        "at most 3 attempts"
    );
    assert!(branch_head(fx.project(), &id).is_some(), "the branch stays");
    // Three resolvers moved the branch; none of their merges landed.
    assert_ne!(git_text(fx.project(), &["rev-parse", "HEAD"]), before);
    assert_eq!(
        git_text(fx.project(), &["log", "--merges", "--format=%H"]),
        ""
    );
    let after = status_of_run(&fx, &id);
    assert!(after["record"]["landed"].as_array().unwrap().is_empty());
    assert_eq!(
        after["record"]["land_failure"]["attempts"],
        serde_json::json!(["land/1", "land/2", "land/3"])
    );
    assert!(after["record"]["land_failure"]["reason"]
        .as_str()
        .unwrap()
        .contains("the target kept moving"));
}

#[test]
fn a_landing_with_no_budget_left_refuses_before_any_resolver_starts() {
    let fx = conflict_fixture();
    fx.write_scenario(&merge_scenario(RESOLVES));
    // The run spends 1.5 of a 1.6 budget: 0.1 is left, under the 0.25 floor.
    let out = run(
        &fx,
        &[
            "run",
            ALIAS,
            "--text",
            "edit shared",
            "--max-budget-usd",
            "1.6",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let id = started_id(&out);
    wait_run_finished(&fx, &id);
    fs::write(fx.project().join("shared.txt"), "project-side\n").unwrap();
    git(fx.project(), &["add", "shared.txt"]);
    git(fx.project(), &["commit", "-qm", "project edits shared"]);
    let tip = git_text(fx.project(), &["rev-parse", "HEAD"]);

    let out = run(&fx, &["land", "merge", ALIAS, &id]);
    let after = assert_nothing_landed(&fx, &id, &tip, &out);
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("no budget left to resolve conflicts"),
        "{out:?}"
    );
    assert!(after["record"]["land_failure"]["reason"]
        .as_str()
        .unwrap()
        .contains("no budget left"));
    assert!(
        !run_dir(&fx, &id).join("land/1/stream.jsonl").exists(),
        "no resolver ran"
    );
}

#[test]
fn a_second_landing_of_the_same_run_is_refused_while_one_is_in_progress() {
    let fx = conflict_fixture();
    // The resolver waits for `release`, so the land lock stays held meanwhile.
    let resolver = format!(
        r#"
  touch "$FAKE_CLAUDE_DIR/landing-started"
  while [ ! -f "$FAKE_CLAUDE_DIR/release" ]; do sleep 0.1; done
{RESOLVES}"#
    );
    let (id, _, _) = conflicting_run(&fx, &merge_scenario(&resolver));
    let first = task(&fx, &["land", "merge", ALIAS, &id])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    assert!(
        wait_until(Duration::from_secs(30), || fx
            .rec_dir()
            .join("landing-started")
            .exists()),
        "the resolver never started"
    );

    let second = run(&fx, &["land", "merge", ALIAS, &id]);
    assert_eq!(second.status.code(), Some(1), "{second:?}");
    assert!(
        String::from_utf8_lossy(&second.stderr).contains("already in progress"),
        "{second:?}"
    );
    let discard = run(&fx, &["land", "discard", ALIAS, &id]);
    assert_eq!(discard.status.code(), Some(1), "{discard:?}");
    assert!(
        String::from_utf8_lossy(&discard.stderr).contains("in progress"),
        "{discard:?}"
    );
    assert!(run_dir(&fx, &id).exists(), "discard left the run");

    fs::write(fx.rec_dir().join("release"), "").unwrap();
    let first = first.wait_with_output().unwrap();
    assert_eq!(first.status.code(), Some(0), "{first:?}");
    // The lock is free again: the run can now be discarded.
    let discard = run(&fx, &["land", "discard", ALIAS, &id]);
    assert_eq!(discard.status.code(), Some(0), "{discard:?}");
}

// ---------------------------------------------------------------- klf: the bounded diff

/// A run that commits a huge file and a small one, reporting done.
const BIG_SCENARIO: &str = r#"
set -e
G="git -c user.name=t -c user.email=t@example.com"
i=0
: > a-big.txt
while [ $i -lt 20000 ]; do echo "line $i" >> a-big.txt; i=$((i+1)); done
echo small > z-small.txt
$G add -A
$G commit -qm "big and small"
gsd-meta-manager taskctl report --status done --headline "Big" --verified "true: ok"
printf '%s\n' '{"type":"result","subtype":"success"}'
"#;

#[test]
fn a_huge_merge_diff_is_capped_summarized_and_exported_in_full() {
    let fx = with_identity(fixture());
    fx.write_scenario(BIG_SCENARIO);
    let out = run(&fx, &["run", ALIAS, "--text", "big"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let id = started_id(&out);
    wait_run_finished(&fx, &id);
    fs::write(fx.project().join("unrelated.txt"), "x").unwrap();
    git(fx.project(), &["add", "unrelated.txt"]);
    git(fx.project(), &["commit", "-qm", "moved on"]);
    let merged = run(&fx, &["land", "merge", ALIAS, &id]);
    assert_eq!(merged.status.code(), Some(0), "{merged:?}");
    let after = status_of_run(&fx, &id);
    let landing = after["record"]["landed"]
        .as_array()
        .unwrap()
        .last()
        .unwrap();
    let sha = landing["merge_sha"].as_str().unwrap().to_owned();

    let diff = gsd_meta_manager::oneoff::land::merge_diff(fx.project(), &sha, &[])
        .expect("the merge diff reads");
    // header + 2 summary lines + blank + at most 5000 hunk lines + the marker.
    assert!(diff.len() <= 2 + 2 + 5000 + 1, "{}", diff.len());
    let joined = diff.join("\n");
    assert!(
        joined.contains("+20000 -0  a-big.txt"),
        "{}",
        &joined[..400.min(joined.len())]
    );
    assert!(joined.contains("+1 -0  z-small.txt"));
    let marker = diff.last().unwrap();
    assert!(marker.starts_with("truncated — "), "{marker}");
    assert!(marker.contains("1 more file,"), "{marker}");
    let more: u64 = marker
        .split("~")
        .nth(1)
        .and_then(|rest| rest.split_whitespace().next())
        .and_then(|n| n.parse().ok())
        .expect("a line count in the marker");
    assert!(
        (14900..=15100).contains(&more),
        "about 15000 lines were cut: {marker}"
    );

    let dir = run_dir(&fx, &id);
    let path = gsd_meta_manager::oneoff::land::export_merge_diff(fx.project(), &dir, &sha)
        .expect("the export writes");
    assert_eq!(path, dir.join(format!("land/merge-{}.diff", &sha[..12])));
    assert_eq!(fs::metadata(&path).unwrap().mode() & 0o777, 0o600);
    let expected = Command::new("git")
        .current_dir(fx.project())
        .env("GIT_CONFIG_GLOBAL", "/dev/null")
        .env("GIT_CONFIG_NOSYSTEM", "1")
        .args([
            "diff",
            "--no-ext-diff",
            "--no-textconv",
            "--no-color",
            "--no-renames",
            &format!("{sha}^1"),
            &sha,
        ])
        .output()
        .unwrap()
        .stdout;
    assert_eq!(
        fs::read(&path).unwrap(),
        expected,
        "byte-equal to git's own diff"
    );

    // A symlink planted at the export path is not followed.
    let victim = fx.scratch.path().join("victim.txt");
    fs::write(&victim, "keep").unwrap();
    fs::remove_file(&path).unwrap();
    std::os::unix::fs::symlink(&victim, &path).unwrap();
    let refused = gsd_meta_manager::oneoff::land::export_merge_diff(fx.project(), &dir, &sha);
    assert!(refused.is_err(), "{refused:?}");
    assert_eq!(fs::read_to_string(&victim).unwrap(), "keep");
}

#[test]
fn a_failed_discard_leaves_no_stale_branch_ref() {
    let fx = fixture();
    let (id, _) = quick_run(&fx);
    let dir = run_dir(&fx, &id);
    // An unremovable subdirectory with an entry makes `remove_dir_all` fail half-way.
    let stuck = dir.join("report/stuck");
    fs::create_dir_all(&stuck).unwrap();
    fs::write(stuck.join("f"), "x").unwrap();
    fs::set_permissions(&stuck, std::os::unix::fs::PermissionsExt::from_mode(0o500)).unwrap();
    // A superuser would remove it anyway; the test is about the unprivileged case.
    let discarded = run(&fx, &["land", "discard", ALIAS, &id]);
    fs::set_permissions(&stuck, std::os::unix::fs::PermissionsExt::from_mode(0o700)).unwrap();
    if discarded.status.success() {
        return;
    }
    assert_eq!(discarded.status.code(), Some(1), "{discarded:?}");
    assert_eq!(branch_head(fx.project(), &id), None, "the branch is gone");
    // `remove_dir_all` may have unlinked run.json before it met the stuck directory
    // (the order is the filesystem's); when it survives it must not name the branch.
    if let Ok(text) = fs::read_to_string(dir.join("run.json")) {
        let record: Value = serde_json::from_str(&text).unwrap();
        assert!(record["tracked_branch"].is_null(), "{record}");
        assert_eq!(record["tracking_note"], "deleted by discard", "{record}");
    }
}

// ---------------------------------------------------------------- klf: stop, kill, pause

/// Ignores SIGTERM and spins: only a KILL ends it.
const IGNORES_TERM_SCENARIO: &str = r#"
trap '' TERM
touch "$FAKE_CLAUDE_DIR/started"
while :; do sleep 0.1; done
"#;

/// Start a run detached with the graces shortened, and wait until its scenario runs.
fn start_with_graces(fx: &Fixture, scenario: &str, stop_secs: &str, pause_secs: &str) -> String {
    fx.write_scenario(scenario);
    let out = task(fx, &["run", ALIAS, "--text", "spin"])
        .env("GSDMM_ONEOFF_STOP_GRACE_SECS", stop_secs)
        .env("GSDMM_ONEOFF_PAUSE_GRACE_SECS", pause_secs)
        .output()
        .expect("the binary runs");
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(
        wait_until(Duration::from_secs(30), || fx.scenario_started()),
        "the scenario never started"
    );
    started_id(&out)
}

#[test]
fn a_hard_stop_escalates_to_kill_when_the_agent_ignores_term() {
    let fx = fixture();
    let id = start_with_graces(&fx, IGNORES_TERM_SCENARIO, "1", "300");
    let stopped = run(&fx, &["stop", ALIAS, "--run-id", &id]);
    assert_eq!(stopped.status.code(), Some(0), "{stopped:?}");
    let done = wait_run_finished(&fx, &id);
    let record = &done["record"];
    assert_eq!(record["outcome"]["kind"], "killed", "{done}");
    assert_eq!(record["stop"]["kind"], "hard", "{done}");
    assert_eq!(record["stop"]["escalated"], true, "{done}");
    assert_eq!(record["stop"]["by"], "cli", "{done}");
    let log = fx.shim_log();
    let term = log
        .find("systemctl --user kill --signal=TERM gsdmm-oneoff-")
        .unwrap_or_else(|| panic!("no TERM kill in {log}"));
    let kill = log
        .find("systemctl --user kill --signal=KILL gsdmm-oneoff-")
        .unwrap_or_else(|| panic!("no KILL kill in {log}"));
    assert!(term < kill, "TERM comes before KILL: {log}");
}

#[test]
fn kill_is_immediate_and_sends_no_term() {
    let fx = fixture();
    fx.write_scenario(PARALLEL_SCENARIO);
    let out = run(&fx, &["run", ALIAS, "--text", "wait"]);
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let id = started_id(&out);
    assert!(
        wait_until(Duration::from_secs(30), || started_count(&fx) == 1),
        "the scenario never started"
    );
    let killed = run(&fx, &["stop", ALIAS, "--run-id", &id, "--kill"]);
    assert_eq!(killed.status.code(), Some(0), "{killed:?}");
    let done = wait_run_finished(&fx, &id);
    let record = &done["record"];
    assert_eq!(record["outcome"]["kind"], "killed", "{done}");
    assert_eq!(record["stop"]["kind"], "kill", "{done}");
    assert_eq!(record["stop"]["escalated"], false, "{done}");
    let log = fx.shim_log();
    assert!(log.contains("--signal=KILL gsdmm-oneoff-"), "{log}");
    assert!(!log.contains("--signal=TERM"), "no TERM on a kill: {log}");
}

/// Replays every stdin line; on the pause message it commits `wip.txt`, reports
/// `partial` and ends.
const PAUSE_HONOURED_SCENARIO: &str = r#"
touch "$FAKE_CLAUDE_DIR/started"
G="git -c user.name=t -c user.email=t@example.com"
while IFS= read -r line; do
  printf '%s\n' "$line" | sed 's/^{/{"isReplay":true,/'
  case "$line" in
    *"Pause requested"*)
      echo wip > wip.txt
      $G add -A
      $G commit -qm "wip: paused"
      gsd-meta-manager taskctl report --status partial --headline "Paused" --followup "finish it"
      printf '%s\n' '{"type":"result","subtype":"success"}'
      exit 0;;
  esac
done
"#;

/// Replays every stdin line and never ends its turn.
const PAUSE_IGNORED_SCENARIO: &str = r#"
touch "$FAKE_CLAUDE_DIR/started"
while IFS= read -r line; do
  printf '%s\n' "$line" | sed 's/^{/{"isReplay":true,/'
done
"#;

/// A run paused through the honouring scenario; returns its id and the finished status.
fn paused_run(fx: &Fixture) -> (String, Value) {
    let id = start_with_graces(fx, PAUSE_HONOURED_SCENARIO, "5", "60");
    let paused = run(
        fx,
        &[
            "stop",
            ALIAS,
            "--run-id",
            &id,
            "--pause",
            "--wait-secs",
            "30",
        ],
    );
    assert_eq!(paused.status.code(), Some(0), "{paused:?}");
    let done = wait_run_finished(fx, &id);
    (id, done)
}

#[test]
fn a_paused_run_commits_its_wip_reports_partial_and_is_tracked() {
    let fx = fixture();
    let (id, done) = paused_run(&fx);
    let record = &done["record"];
    assert_eq!(record["outcome"]["kind"], "paused", "{done}");
    assert_eq!(record["report"]["status"], "partial", "{done}");
    assert_eq!(record["report_source"]["kind"], "agent", "{done}");
    assert_eq!(record["stop"]["kind"], "pause", "{done}");
    assert_eq!(record["stop"]["escalated"], false, "{done}");
    assert_eq!(
        record["tracked_branch"],
        format!("refs/heads/gsd-oneoff/{id}"),
        "{done}"
    );
    assert_eq!(
        branch_head(fx.project(), &id).as_deref(),
        record["head"].as_str(),
        "{done}"
    );
    let dir = run_dir(&fx, &id);
    let injections = lines_of(&dir.join("injections.jsonl"));
    let pauses: Vec<&Value> = injections.iter().filter(|i| i["kind"] == "pause").collect();
    assert_eq!(pauses.len(), 1, "{injections:?}");
    assert_eq!(pauses[0]["outcome"], "delivered");
    assert!(pauses[0]["text"]
        .as_str()
        .unwrap()
        .contains("taskctl report --status partial"));
    let stream = fs::read_to_string(dir.join("stream.jsonl")).unwrap();
    assert!(stream.contains("Pause requested"), "{stream}");
}

#[test]
fn a_pause_the_agent_ignores_is_cut_after_its_grace() {
    let fx = fixture();
    let id = start_with_graces(&fx, PAUSE_IGNORED_SCENARIO, "1", "1");
    let paused = run(
        &fx,
        &[
            "stop",
            ALIAS,
            "--run-id",
            &id,
            "--pause",
            "--wait-secs",
            "30",
        ],
    );
    assert_eq!(paused.status.code(), Some(0), "{paused:?}");
    let done = wait_run_finished(&fx, &id);
    let record = &done["record"];
    assert_eq!(record["outcome"]["kind"], "killed", "{done}");
    assert_eq!(record["stop"]["kind"], "pause", "{done}");
    assert_eq!(record["stop"]["escalated"], true, "{done}");
}

/// A conflicting run whose resolver announces itself and waits for `release`, then runs
/// `then`.
fn waiting_resolver(then: &str) -> String {
    merge_scenario(&format!(
        r#"
  touch "$FAKE_CLAUDE_DIR/resolver-started"
  while [ ! -f "$FAKE_CLAUDE_DIR/release" ]; do sleep 0.1; done
{then}
"#
    ))
}

/// Spawn `task land merge` as a child and wait until its resolver runs.
fn landing_child(fx: &Fixture, id: &str) -> std::process::Child {
    let child = task(fx, &["land", "merge", ALIAS, id])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    assert!(
        wait_until(Duration::from_secs(60), || fx
            .rec_dir()
            .join("resolver-started")
            .exists()),
        "the resolver never started"
    );
    child
}

#[test]
fn stopping_a_resolver_lands_nothing_and_keeps_the_branch() {
    let fx = conflict_fixture();
    let (id, _, tip) = conflicting_run(&fx, &waiting_resolver(RESOLVES));
    let child = landing_child(&fx, &id);
    let stopped = run(&fx, &["stop", ALIAS, "--run-id", &id]);
    assert_eq!(stopped.status.code(), Some(0), "{stopped:?}");
    assert!(
        String::from_utf8_lossy(&stopped.stdout).contains("landing"),
        "it says it stopped the landing: {stopped:?}"
    );
    let out = child.wait_with_output().unwrap();
    let after = assert_nothing_landed(&fx, &id, &tip, &out);
    let failure = &after["record"]["land_failure"];
    assert_eq!(failure["reason"], "stopped by user", "{after}");
    assert_eq!(failure["stop"], "hard", "{after}");
}

#[test]
fn pausing_a_resolver_lands_nothing_even_when_it_resolves() {
    let fx = conflict_fixture();
    let (id, _, tip) = conflicting_run(&fx, &waiting_resolver(RESOLVES));
    let child = landing_child(&fx, &id);
    let paused = run(
        &fx,
        &[
            "stop",
            ALIAS,
            "--run-id",
            &id,
            "--pause",
            "--wait-secs",
            "0",
        ],
    );
    assert_eq!(paused.status.code(), Some(0), "{paused:?}");
    fs::write(fx.rec_dir().join("release"), "").unwrap();
    let out = child.wait_with_output().unwrap();
    let after = assert_nothing_landed(&fx, &id, &tip, &out);
    let failure = &after["record"]["land_failure"];
    assert_eq!(failure["reason"], "paused", "{after}");
    assert_eq!(failure["stop"], "pause", "{after}");
    let attempts: Vec<_> = fs::read_dir(run_dir(&fx, &id).join("land"))
        .unwrap()
        .flatten()
        .filter(|e| e.path().is_dir())
        .collect();
    assert_eq!(attempts.len(), 1, "no further attempt starts: {attempts:?}");

    // `task resume` on a run whose landing was paused retries the landing and refuses
    // `--from`; a stale request does not stop it: the landing clears it with its lock.
    let refused = run(&fx, &["resume", ALIAS, "--run-id", &id, "--from", "branch"]);
    assert_eq!(refused.status.code(), Some(1), "{refused:?}");
    assert!(
        String::from_utf8_lossy(&refused.stderr).contains("retries the landing"),
        "{refused:?}"
    );
    let again = run(&fx, &["resume", ALIAS, "--run-id", &id]);
    assert_eq!(again.status.code(), Some(0), "{again:?}");
    assert_eq!(
        fs::read_to_string(fx.project().join("shared.txt")).unwrap(),
        "merged\n"
    );
}

// ---------------------------------------------------------------- klf: resume

/// The resumed run: records the log it can see, then commits a file and reports done once
/// released. A conflict resolver (MERGE_HEAD exists) resolves with `resolver`.
fn resumed_scenario(resolver: &str) -> String {
    format!(
        r#"
G="git -c user.name=t -c user.email=t@example.com"
if git rev-parse -q --verify MERGE_HEAD >/dev/null 2>&1; then
{resolver}
else
  touch "$FAKE_CLAUDE_DIR/agent-session"
  git log --format=%s > "$FAKE_CLAUDE_DIR/resumed-log.txt"
  while [ ! -f "$FAKE_CLAUDE_DIR/release" ]; do sleep 0.1; done
  echo more > more.txt
  $G add -A
  $G commit -qm "resumed work"
  gsd-meta-manager taskctl report --status done --headline "Finished it" --verified "true: ok"
  printf '%s\n' '{{"type":"result","subtype":"success"}}'
fi
"#
    )
}

/// The id after `resumed as ` in a `task resume` line.
fn resumed_id(out: &Output) -> String {
    let stdout = String::from_utf8_lossy(&out.stdout);
    let mut words = stdout.split_whitespace();
    while let Some(word) = words.next() {
        if word == "as" {
            if let Some(id) = words.next().filter(|w| w.starts_with("oneoff-")) {
                return id.to_owned();
            }
        }
    }
    panic!("no resumed id in {out:?}");
}

#[test]
fn a_paused_run_resumes_from_its_branch_as_a_new_run() {
    let fx = fixture();
    let (old, paused) = paused_run(&fx);
    let old_head = paused["record"]["head"].as_str().unwrap().to_owned();
    fx.write_scenario(&resumed_scenario("  :"));
    let out = run(
        &fx,
        &[
            "resume", ALIAS, "--run-id", &old, "--from", "branch", "--detach",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let new = resumed_id(&out);
    assert_ne!(new, old);
    assert!(
        wait_until(Duration::from_secs(30), || fx
            .rec_dir()
            .join("agent-session")
            .exists()),
        "the resumed session never started"
    );

    let live = status_of_run(&fx, &new);
    assert_eq!(live["record"]["resumed_from"], old.as_str(), "{live}");
    assert_eq!(live["record"]["base"], old_head.as_str(), "{live}");
    let old_after = status_of_run(&fx, &old);
    assert_eq!(
        old_after["record"]["resumed_by"],
        new.as_str(),
        "{old_after}"
    );

    // The session is a fresh one, primed with the old task, report and log.
    let prompt = fx.recorded("prompt.txt");
    assert!(prompt.contains(&old), "{prompt}");
    assert!(prompt.contains("Paused"), "{prompt}");
    assert!(prompt.contains("finish it"), "{prompt}");
    assert!(prompt.contains("wip: paused"), "{prompt}");
    assert!(
        fx.recorded("resumed-log.txt").contains("wip: paused"),
        "the new clone starts at the old head"
    );

    // A second resume of the old run, and a resume of the live new one, are refused.
    let again = run(
        &fx,
        &[
            "resume", ALIAS, "--run-id", &old, "--from", "branch", "--detach",
        ],
    );
    assert_eq!(again.status.code(), Some(1), "{again:?}");
    assert!(
        String::from_utf8_lossy(&again.stderr).contains("already resumed"),
        "{again:?}"
    );
    let live_resume = run(
        &fx,
        &[
            "resume", ALIAS, "--run-id", &new, "--from", "branch", "--detach",
        ],
    );
    assert_eq!(live_resume.status.code(), Some(1), "{live_resume:?}");
    assert!(
        String::from_utf8_lossy(&live_resume.stderr).contains("still live"),
        "{live_resume:?}"
    );
    let listed = run(&fx, &["list", ALIAS]);
    assert!(
        String::from_utf8_lossy(&listed.stdout).contains(&format!("resumed as {new}")),
        "{listed:?}"
    );

    fs::write(fx.rec_dir().join("release"), "").unwrap();
    let done = wait_run_finished(&fx, &new);
    assert_eq!(done["record"]["outcome"]["kind"], "finished", "{done}");
    assert_eq!(done["record"]["commits"], 1, "{done}");
    // Both runs' commits land together; the old run no longer counts as to land.
    let merged = run(&fx, &["land", "merge", ALIAS, &new]);
    assert_eq!(merged.status.code(), Some(0), "{merged:?}");
    let log = git_text(fx.project(), &["log", "--format=%s"]);
    assert!(
        log.contains("wip: paused") && log.contains("resumed work"),
        "{log}"
    );
}

#[test]
fn resume_from_master_merges_the_checkout_tip_into_the_run_first() {
    let fx = with_identity(fixture());
    let (old, paused) = paused_run(&fx);
    let old_head = paused["record"]["head"].as_str().unwrap().to_owned();
    fs::write(fx.project().join("other.txt"), "x").unwrap();
    git(fx.project(), &["add", "other.txt"]);
    git(fx.project(), &["commit", "-qm", "main moved on"]);
    let tip = git_text(fx.project(), &["rev-parse", "HEAD"]);

    fx.write_scenario(&resumed_scenario("  :"));
    fs::write(fx.rec_dir().join("release"), "").unwrap();
    let out = run(
        &fx,
        &[
            "resume", ALIAS, "--run-id", &old, "--from", "master", "--detach",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let new = resumed_id(&out);
    let done = wait_run_finished(&fx, &new);
    let record = &done["record"];
    assert_eq!(record["outcome"]["kind"], "finished", "{done}");
    assert_eq!(record["resume_merge"]["how"], "merge", "{done}");
    let base = record["base"].as_str().unwrap();
    assert_eq!(record["resume_merge"]["merge_sha"], base, "{done}");
    let parents: Vec<String> = git_text(fx.project(), &["rev-list", "--parents", "-n", "1", base])
        .split_whitespace()
        .map(str::to_owned)
        .collect();
    assert_eq!(parents[1..], [old_head, tip], "{parents:?}");
    let text = run(&fx, &["status", ALIAS, "--run-id", &new]);
    assert!(
        String::from_utf8_lossy(&text.stdout).contains("resume:    merged main as"),
        "{text:?}"
    );
}

/// A pause that commits a conflicting edit of `shared.txt`.
const PAUSE_EDITS_SHARED: &str = r#"
touch "$FAKE_CLAUDE_DIR/started"
G="git -c user.name=t -c user.email=t@example.com"
while IFS= read -r line; do
  printf '%s\n' "$line" | sed 's/^{/{"isReplay":true,/'
  case "$line" in
    *"Pause requested"*)
      echo run-side > shared.txt
      $G add -A
      $G commit -qm "wip: shared"
      gsd-meta-manager taskctl report --status partial --headline "Paused" --followup "finish it"
      printf '%s\n' '{"type":"result","subtype":"success"}'
      exit 0;;
  esac
done
"#;

/// A paused run on a conflict fixture, then `main` edits the same file.
fn paused_conflicting_run(fx: &Fixture) -> String {
    let id = start_with_graces(fx, PAUSE_EDITS_SHARED, "5", "60");
    let paused = run(
        fx,
        &[
            "stop",
            ALIAS,
            "--run-id",
            &id,
            "--pause",
            "--wait-secs",
            "30",
        ],
    );
    assert_eq!(paused.status.code(), Some(0), "{paused:?}");
    wait_run_finished(fx, &id);
    fs::write(fx.project().join("shared.txt"), "project-side\n").unwrap();
    git(fx.project(), &["add", "shared.txt"]);
    git(fx.project(), &["commit", "-qm", "project edits shared"]);
    // `start_with_graces` waits for the scenario's `started` marker; clear it.
    let _ = fs::remove_file(fx.rec_dir().join("started"));
    id
}

#[test]
fn a_conflicting_resume_from_master_is_resolved_by_the_agent() {
    let fx = conflict_fixture();
    let old = paused_conflicting_run(&fx);
    fx.write_scenario(&resumed_scenario(RESOLVES));
    fs::write(fx.rec_dir().join("release"), "").unwrap();
    let out = run(
        &fx,
        &[
            "resume", ALIAS, "--run-id", &old, "--from", "master", "--detach",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let new = resumed_id(&out);
    let done = wait_run_finished(&fx, &new);
    let record = &done["record"];
    assert_eq!(record["outcome"]["kind"], "finished", "{done}");
    assert_eq!(record["resume_merge"]["how"], "resolved", "{done}");
    assert_eq!(
        record["resume_merge"]["conflicted"],
        serde_json::json!(["shared.txt"]),
        "{done}"
    );
    assert_eq!(
        record["resume_merge"]["attempts"],
        serde_json::json!(["land/1"]),
        "{done}"
    );
    assert!(
        fx.rec_dir().join("agent-session").exists(),
        "the agent session ran after the resume merge"
    );
    // The resolver's spend is on the new run.
    assert!(record["spend_usd"].as_f64().unwrap() >= 0.5, "{done}");
}

#[test]
fn a_failing_resume_merge_ends_the_run_failed_and_never_strands_its_predecessor() {
    let fx = conflict_fixture();
    let old = paused_conflicting_run(&fx);
    let failing = r#"
  gsd-meta-manager taskctl report --status failed --headline "could not resolve"
  printf '%s\n' '{"type":"result","subtype":"success","total_cost_usd":0.5}'
"#;
    fx.write_scenario(&resumed_scenario(failing));
    let out = run(
        &fx,
        &[
            "resume", ALIAS, "--run-id", &old, "--from", "master", "--detach",
        ],
    );
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let new = resumed_id(&out);
    let done = wait_run_finished(&fx, &new);
    let record = &done["record"];
    assert_eq!(record["outcome"]["kind"], "failed", "{done}");
    assert!(record["resume_merge"]["failure"].is_string(), "{done}");
    assert!(
        !fx.rec_dir().join("agent-session").exists(),
        "no agent session after a failed resume merge"
    );
    // The old run can be resumed again.
    fs::write(fx.rec_dir().join("release"), "").unwrap();
    let again = run(
        &fx,
        &[
            "resume", ALIAS, "--run-id", &old, "--from", "branch", "--detach",
        ],
    );
    assert_eq!(again.status.code(), Some(0), "{again:?}");
}
