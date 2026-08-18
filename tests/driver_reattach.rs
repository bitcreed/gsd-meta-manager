// ============================================================================
// A run outlives the process that started it, and is found again (CTRL-04).
//
// This file proves ROADMAP success criterion #2 — *"Closing the TUI mid-run
// leaves the run going; reopening the TUI shows that run still live with its
// current step."* Every test here needs a real driver in its own process image,
// which is why none of them is an in-source unit test: a detached process that
// is never actually detached from anything proves nothing.
//
// **What this file deliberately does NOT attempt: live output re-streaming after
// a restart.** Once the spawning process exits, the child's stdout pipe is gone.
// There is no re-attaching to a pipe whose other end no longer exists, and
// REQUIREMENTS lists re-streaming as out of scope for exactly that reason
// (D-11). What is proved instead is that the **run** survives and that the
// **journal** carries its current step. There is no `#[ignore]`d aspirational
// test for the impossible half — an ignored test for something physics forbids
// is a promise, not a plan.
//
// The adopted state is therefore "observed": journal tail for history and
// current step, plus a pgid to signal. Not "reattached", not "streaming".
//
// Unix-only by construction: `process_group` is `std::os::unix`, and driving is
// a Unix capability (D-05).
// ============================================================================

#![cfg(unix)]

use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use gsd_meta_manager::config::{save_config, Config, DriverOptIn, RegisteredProject};
use gsd_meta_manager::driver::{liveness, reconcile};
use gsd_meta_manager::journal::reader::{self, ParsedLine, TailCursor};
use tempfile::TempDir;

/// The paced stand-in. Sixty heartbeats at 100ms is about six seconds, which is
/// comfortably longer than any poll loop below needs and short enough that the
/// one test which waits for completion does not dominate the suite.
const FAKE_SLOW: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/fake-claude-slow.sh"
);
const HEARTBEATS: &str = "60";
const INTERVAL: &str = "0.1";

const ALIAS: &str = "detached";
const GOAL: &str = "prove the run outlives its parent";
const COMMAND: &str = "/gsd-progress";

/// A project root with a `.planning/` directory, plus a config file naming it.
struct Fixture {
    root: TempDir,
    _config_dir: TempDir,
    config_path: PathBuf,
}

impl Fixture {
    fn new(run_note: &str) -> Self {
        let root = TempDir::new().expect("temp dir");
        std::fs::create_dir_all(root.path().join(".planning")).expect("scratch .planning");

        let config_dir = TempDir::new().expect("temp dir for the config");
        let config_path = config_dir.path().join("config.json");
        save_config(&registry(root.path()), &config_path).unwrap_or_else(|e| {
            panic!("write the driver's config for {run_note}: {e}");
        });

        Fixture {
            root,
            _config_dir: config_dir,
            config_path,
        }
    }

    fn root(&self) -> &Path {
        self.root.path()
    }

    fn planning(&self) -> PathBuf {
        self.root.path().join(".planning")
    }
}

/// A one-entry registry pointing at `root`, opted in.
fn registry(root: &Path) -> Config {
    let mut config = Config::new();
    config.projects.insert(
        ALIAS.to_string(),
        RegisteredProject {
            path: root.to_path_buf(),
            added: "2026-07-29T12:00:00Z".to_string(),
            driver_opt_in: Some(DriverOptIn {
                opted_in_at: "2026-07-29T11:59:00Z".to_string(),
                claude_md_digest: None,
                branch_namespace: None,
                credential: None,
                pr_cap_per_24h: None,
                pr_cap_per_run: None,
            }),
            extra: Default::default(),
        },
    );
    config
}

/// The `drive` argument list, as `src/driver/spawn.rs::drive_argv` builds it,
/// plus the two hidden development flags a test may use and the TUI may not.
fn drive_args(config_path: &Path, run_id: &str) -> Vec<String> {
    vec![
        "--config".to_string(),
        config_path.display().to_string(),
        "drive".to_string(),
        ALIAS.to_string(),
        "--command".to_string(),
        COMMAND.to_string(),
        "--run-id".to_string(),
        run_id.to_string(),
        "--goal".to_string(),
        GOAL.to_string(),
        "--claude-program".to_string(),
        FAKE_SLOW.to_string(),
        "--claude-args".to_string(),
        HEARTBEATS.to_string(),
        "--claude-args".to_string(),
        INTERVAL.to_string(),
        "--claude-args".to_string(),
        "result".to_string(),
    ]
}

/// `/proc/<pid>/stat` field 5, the process group id.
///
/// Same comm-field-skipping parse as `driver::liveness::process_state`: the
/// executable name is parenthesised and unescaped, so counting tokens from the
/// start of the line is the classic way to get this wrong. After the last `)`
/// the fields are state, ppid, pgrp.
fn process_group_of(pid: u32) -> Option<u32> {
    let stat = std::fs::read_to_string(format!("/proc/{}/stat", pid)).ok()?;
    let after_comm = stat.rfind(')')?.checked_add(2)?;
    stat.get(after_comm..)?
        .split_whitespace()
        .nth(2)?
        .parse()
        .ok()
}

/// SIGKILL a whole process group.
///
/// The shell builtin rather than a signalling crate: `rustix` is a normal
/// dependency and an integration test is a separate crate that cannot see it,
/// and `pgid` is an integer so the interpolation cannot carry anything else.
/// This is `tests/executor_lifecycle.rs`'s `alive()` idiom, one signal along.
fn kill_group(pgid: u32) {
    let _ = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!("kill -KILL -{pgid} 2>/dev/null"))
        .status();
}

/// Poll until the run's driver is observably live, giving up after `limit`.
fn live_within(pid: u32, run_id: &str, limit: Duration) -> bool {
    let deadline = std::time::Instant::now() + limit;
    while std::time::Instant::now() < deadline {
        if liveness::is_run_alive(pid, run_id) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    false
}

/// Poll until the run's driver is gone, giving up after `limit`.
fn gone_within(pid: u32, run_id: &str, limit: Duration) -> bool {
    let deadline = std::time::Instant::now() + limit;
    while std::time::Instant::now() < deadline {
        if !liveness::is_run_alive(pid, run_id) {
            return true;
        }
        std::thread::sleep(Duration::from_millis(25));
    }
    false
}

/// Every file under `dir`, as `(relative path, byte length, content digest)`.
///
/// Content is digested and not merely measured, because the most plausible
/// unwanted write here is a `run.json` rewritten with an `ended_at` of the same
/// width, or an `active` pointer rewritten with the same run id — and a
/// length-only comparison cannot see either.
fn fingerprint_tree(dir: &Path) -> Vec<(String, u64, String)> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<(String, u64, String)>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(kind) = entry.file_type() else { continue };
            if kind.is_dir() {
                walk(root, &path, out);
            } else {
                let bytes = std::fs::read(&path).unwrap_or_default();
                let relative = path
                    .strip_prefix(root)
                    .unwrap_or(&path)
                    .to_string_lossy()
                    .into_owned();
                let digest = gsd_meta_manager::journal::argv_digest(&[String::from_utf8_lossy(
                    &bytes,
                )
                .into_owned()]);
                out.push((relative, bytes.len() as u64, digest));
            }
        }
    }

    let mut out = Vec::new();
    walk(dir, dir, &mut out);
    out.sort();
    out
}

#[tokio::test]
async fn a_run_outlives_the_process_that_spawned_it() {
    // Criterion #2, first half. The spawn is configured **exactly** as
    // `src/driver/spawn.rs::spawn_detached` configures it — new process group,
    // three null stdio handles, `kill_on_drop(false)` — and the fidelity of that
    // claim is pinned by an assertion over that module's own source at the
    // bottom of this test, not left as a comment somebody can falsify.
    //
    // The production function itself cannot be called from here: it spawns
    // `std::env::current_exe()`, which inside an integration test is this test
    // harness and not `gsd-meta-manager`. Reproducing the configuration and
    // pinning it is the honest alternative.
    const RUN_ID: &str = "2026-07-29T12-00-00Z-aaaa";
    let fixture = Fixture::new("outlives");

    let mut cmd = tokio::process::Command::new(env!("CARGO_BIN_EXE_gsd-meta-manager"));
    cmd.args(drive_args(&fixture.config_path, RUN_ID));
    cmd.current_dir(fixture.root());
    cmd.stdin(std::process::Stdio::null());
    cmd.stdout(std::process::Stdio::null());
    cmd.stderr(std::process::Stdio::null());
    cmd.kill_on_drop(false);
    cmd.as_std_mut().process_group(0);

    let child = cmd.spawn().expect("the driver binary is built and spawnable");
    let pid = child.id().expect("a freshly spawned child has a pid");

    assert!(
        live_within(pid, RUN_ID, Duration::from_secs(30)),
        "the driver never came up; nothing after this point would mean anything"
    );

    // Its own process group, which is what makes `RunRecord.pgid == pid` true
    // and what gives the kill switch a group to signal (D-01, D-04). Without it
    // the driver would share the TUI's group and a group-directed signal aimed
    // at one would reach the other.
    assert_eq!(
        process_group_of(pid),
        Some(pid),
        "the driver must lead its own process group"
    );

    // THE load-bearing line of this file, and it looks trivial. Dropping a
    // `tokio::process::Child` configured with `kill_on_drop(true)` sends SIGKILL;
    // configured with `kill_on_drop(false)` it does not. A clean TUI shutdown
    // drops exactly this handle, so an accidental `true` would silently destroy
    // the entire point of the phase — the run would die precisely when the user
    // closed the window, which is the one moment the phase exists to survive
    // (D-02). Everything above this line passes either way.
    drop(child);

    std::thread::sleep(Duration::from_millis(500));
    assert!(
        liveness::is_run_alive(pid, RUN_ID),
        "the run must still be alive AFTER its parent handle was dropped. If this \
         fails, kill_on_drop is true somewhere and closing the TUI kills the run"
    );

    // The configuration this test reproduces, pinned against drift.
    let production = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/src/driver/spawn.rs"
    ))
    .expect("the production spawn module is readable");
    let executable: String = production
        .lines()
        .filter(|line| !line.trim_start().starts_with("//"))
        .collect::<Vec<_>>()
        .join("\n");
    assert!(
        executable.contains("kill_on_drop(false)")
            && !executable.contains("kill_on_drop(true)")
            && executable.matches("Stdio::null()").count() == 3
            && executable.contains("process_group(0)"),
        "the reproduction above claims to match `spawn_detached`; that claim is \
         only worth anything while it is true"
    );

    // Cleanup. The handle is gone by construction, so the group is signalled
    // rather than waited on — which is also the exact shape D-07 requires of the
    // kill switch when the TUI adopted a run it did not spawn. Tokio's orphan
    // queue reaps the dropped child, so no zombie survives this test.
    kill_group(pid);
    assert!(
        gone_within(pid, RUN_ID, Duration::from_secs(10)),
        "the orphaned driver must die when its group is signalled"
    );
}

#[tokio::test]
async fn a_fresh_scan_finds_the_orphaned_run_live_with_its_last_journal_step() {
    // Criterion #2, second half: a TUI that has just started and knows nothing
    // but its registry finds the run again, off disk.
    const RUN_ID: &str = "2026-07-29T12-00-01Z-bbbb";
    let fixture = Fixture::new("fresh scan");

    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_gsd-meta-manager"))
        .args(drive_args(&fixture.config_path, RUN_ID))
        .current_dir(fixture.root())
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .process_group(0)
        .spawn()
        .expect("the driver binary is built and spawnable");
    let pid = child.id();

    assert!(
        live_within(pid, RUN_ID, Duration::from_secs(30)),
        "the driver never came up"
    );

    // A **new** `Config` value, built from nothing but the registry — not the
    // one that spawned the run, and not a handle held over from it. This is the
    // whole of "reopening the TUI": the scan has to rediscover the run from
    // disk (CTRL-04).
    let fresh = registry(fixture.root());
    let observed = reconcile::reconcile_all(&fresh.projects);

    assert_eq!(observed.len(), 1, "exactly one project has a run to observe");
    let run = &observed[0];
    assert_eq!(run.alias, ALIAS);
    assert_eq!(run.run_id, RUN_ID);
    assert_eq!(
        run.verdict(),
        reconcile::RunVerdict::Live,
        "the rediscovered run is live"
    );
    assert_eq!(run.pid, pid, "the pid came off the driver's own run.json");
    assert_eq!(run.pgid, pid, "the driver leads its own group (D-04)");
    // These two came off `run.json` and could not have been guessed.
    assert_eq!(run.goal, GOAL);
    assert_eq!(run.gsd_command, COMMAND);

    // "Current step" means the last journal record and NOT a live stream (D-11).
    // The read goes through the same two functions the TUI uses — a byte-offset
    // tail then a per-line parse — because after a restart the journal is the
    // only source there is.
    let journal = gsd_meta_manager::journal::run_paths(&fixture.planning(), RUN_ID)
        .expect("the fixture run id is a plain path component")
        .journal;
    let tail = reader::tail_lines(&journal, TailCursor::default()).expect("the journal is readable");
    assert!(
        !tail.lines.is_empty(),
        "a live run must have written at least its run_started record"
    );

    let last = tail
        .lines
        .iter()
        .filter_map(|line| match reader::parse_line(line) {
            ParsedLine::Record(record) => Some(record),
            ParsedLine::Unparseable { .. } => None,
        })
        .next_back()
        .expect("at least one journal line parsed");

    // Membership in a small set rather than a single value: how far the paced
    // stand-in has got by now is a scheduling question, and pinning one kind
    // would make this a timing test rather than a reattachment test.
    const REACHABLE: &[&str] = &[
        "run_started",
        "exec_started",
        "exec_event",
        "cost",
        "diagnostic",
    ];
    assert!(
        REACHABLE.contains(&last.kind.as_str()),
        "the current step must be a kind the run has actually reached, got {:?}",
        last.kind
    );
    assert!(last.seq >= 1, "sequence numbers start at 1");

    // Let the run finish on its own and reap it, so the suite leaves nothing
    // behind. A completed run also ends with `ended_at` set, which is what makes
    // the next assertion the opposite of the crash test below.
    let status = child.wait().expect("reap the driver");
    assert!(status.success(), "the run completed cleanly");
    assert_eq!(
        reconcile::reconcile_all(&fresh.projects).len(),
        0,
        "a run that reached its terminal write has nothing left to observe"
    );
}

#[tokio::test]
async fn a_run_killed_without_an_ending_is_reported_crashed_and_nothing_on_disk_is_repaired() {
    // D-12 at integration scale.
    const RUN_ID: &str = "2026-07-29T12-00-02Z-cccc";
    let fixture = Fixture::new("crash");

    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_gsd-meta-manager"))
        .args(drive_args(&fixture.config_path, RUN_ID))
        .current_dir(fixture.root())
        .stdin(std::process::Stdio::null())
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .process_group(0)
        .spawn()
        .expect("the driver binary is built and spawnable");
    let pid = child.id();

    assert!(
        live_within(pid, RUN_ID, Duration::from_secs(30)),
        "the driver never came up"
    );

    let run_json = gsd_meta_manager::journal::run_paths(&fixture.planning(), RUN_ID)
        .expect("the fixture run id is a plain path component")
        .run_json;
    let raw = std::fs::read_to_string(&run_json).expect("the run record is on disk");
    let record: serde_json::Value = serde_json::from_str(&raw).expect("the run record parses");
    assert!(
        record.get("ended_at").map(|v| v.is_null()).unwrap_or(true),
        "the run must still be un-ended when it is killed, or this test proves \
         nothing about a crash"
    );

    // SIGKILL the driver's whole process group: the one signal it cannot handle,
    // so no cooperative path — no journal finish, no lock release, no pointer
    // clear — can possibly have run. Whatever the disk looks like afterwards is
    // what a real crash leaves.
    //
    // The `claude` stand-in is in its OWN group (Phase 15 spawns it with
    // `ProcessGroup::leader()`), so this does not reach it; it exits on its own
    // within a few seconds and writes nothing under `.planning`. The driver's
    // two-layer teardown that would have reached it is plan 17-06's, and SIGKILL
    // skips it by definition.
    kill_group(pid);
    let status = child.wait().expect("reap the killed driver");
    assert!(!status.success(), "a SIGKILLed process does not exit cleanly");
    assert!(gone_within(pid, RUN_ID, Duration::from_secs(10)));

    let before = fingerprint_tree(&fixture.planning());
    // Pinned against vacuity: two empty walks compare equal to each other, so a
    // fingerprint that silently found nothing would "prove" the assertion below.
    assert!(
        before.iter().any(|(path, _, _)| path.ends_with("run.json")),
        "the walk must have seen the run record it is protecting, got: {before:?}"
    );

    let fresh = registry(fixture.root());
    let observed = reconcile::reconcile_all(&fresh.projects);
    assert_eq!(observed.len(), 1, "a crashed run is still surfaced");
    assert_eq!(observed[0].run_id, RUN_ID);
    // The verdict rather than `!is_live()`, and the difference is now real: since
    // plan 17-08 there are two non-live verdicts, and only one of them is a
    // crash. `LivenessUnknown` would satisfy a negated `is_live()` while meaning
    // the opposite of what this assertion is about (CR-05).
    assert_eq!(
        observed[0].verdict(),
        reconcile::RunVerdict::CrashedWithoutEnding,
        "a run whose driver is dead must be reported crashed — not live, and not \
         merely 'not live'"
    );

    let after = fingerprint_tree(&fixture.planning());
    assert_eq!(
        before, after,
        "the scan must leave the .planning tree BYTE-IDENTICAL. Leaving the disk \
         untouched is the decision: the stale pointer plus a missing ended_at \
         plus a dead pid IS the crash record, and a repair would both destroy \
         that evidence and break the exactly-twice run.json write contract (D-12)"
    );

    // The specific "repair" a well-meaning implementation adds, checked on its
    // own so a whole-tree assertion cannot mask it behind a compensating change.
    let pointer = gsd_meta_manager::journal::runs_root(&fixture.planning()).join("active");
    assert_eq!(
        std::fs::read_to_string(&pointer)
            .expect("the stale active pointer still exists")
            .trim(),
        RUN_ID,
        "the active pointer must still name the dead run"
    );
}
