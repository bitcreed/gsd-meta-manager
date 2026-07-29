// ============================================================================
// Stopping a run removes the whole tree (CTRL-01, ROADMAP success criterion #1).
//
// *"Pressing stop during an active run leaves no `claude` process, no grandchild
// build or server process, and no zombie behind — verified 15 seconds later."*
//
// These tests need **three real process levels**, which is why none of them is
// an in-source unit test:
//
//   1. the driver — a real detached `gsd-meta-manager drive` in its own process
//      image and its own process group;
//   2. its fake `claude` — `fake-claude-spawner.sh`, spawned by the driver's
//      executor as the leader of a SECOND process group, which is the fact the
//      whole two-layer teardown exists for;
//   3. that agent's grandchild — a real `sleep 600` the stand-in backgrounds.
//
// **The grandchild is the entire point.** Killing the direct child alone looks
// identical to killing the tree unless something outlives the leader and can be
// checked afterwards; without level 3 a stop that orphans every build server and
// dev server the agent started would pass every assertion here.
//
// The zombie half is read from `/proc/<pid>/stat` through
// `driver::liveness::is_zombie` and **never** through the `kill -0` helper at
// `tests/executor_lifecycle.rs:92`: the zero signal succeeds for a zombie, which
// still owns a pid and a process-table entry, so that helper reports a zombie as
// ALIVE and cannot answer this half of the criterion at all. There is no
// `kill -0` in this file.
//
// Unix-only by construction (D-05).
// ============================================================================

#![cfg(unix)]

use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use gsd_meta_manager::config::{save_config, Config, DriverOptIn, RegisteredProject};
use gsd_meta_manager::driver::kill::{self, ReapArm, StopOutcome, DRIVER_TEARDOWN_GRACE};
use gsd_meta_manager::driver::{liveness, lock};
use gsd_meta_manager::journal::reader::{self, JournalRecord, ParsedLine, TailCursor};
use tempfile::TempDir;

/// The grandchild-spawning stand-in. The only fixture that can prove what
/// criterion #1 claims.
const FAKE_SPAWNER: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/fake-claude-spawner.sh"
);

/// The point at which ROADMAP success criterion #1 verifies, taken from the
/// criterion's own words: *"verified 15 seconds later"*.
///
/// It is a real wall-clock wait and not a poll-until-true, because the claim is
/// about the state of the process table at a fixed later instant. Polling would
/// let a tree that dies at T+14.9s and a tree that was already gone at T+0.2s
/// pass identically, and only one of those is what the criterion describes.
const VERIFY_AFTER: Duration = Duration::from_secs(15);

/// How long a driver is given to come up before a test gives up on it.
const STARTUP_LIMIT: Duration = Duration::from_secs(30);

const ALIAS: &str = "stoppable";
const COMMAND: &str = "/gsd-progress";

/// A project root with a `.planning/` directory, plus a config naming it opted
/// in.
struct Fixture {
    root: TempDir,
    _config_dir: TempDir,
    config_path: PathBuf,
}

impl Fixture {
    fn new(note: &str) -> Self {
        let root = TempDir::new().expect("temp dir");
        std::fs::create_dir_all(root.path().join(".planning")).expect("scratch .planning");

        let config_dir = TempDir::new().expect("temp dir for the config");
        let config_path = config_dir.path().join("config.json");

        let mut config = Config::new();
        config.projects.insert(
            ALIAS.to_string(),
            RegisteredProject {
                path: root.path().to_path_buf(),
                added: "2026-07-29T12:00:00Z".to_string(),
                driver_opt_in: Some(DriverOptIn {
                    opted_in_at: "2026-07-29T11:59:00Z".to_string(),
                    claude_md_digest: None,
                }),
                extra: Default::default(),
            },
        );
        save_config(&config, &config_path)
            .unwrap_or_else(|e| panic!("write the driver's config for {note}: {e}"));

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

    fn journal(&self, run_id: &str) -> PathBuf {
        gsd_meta_manager::journal::run_paths(&self.planning(), run_id).journal
    }
}

/// The `drive` argument list, plus the two hidden development flags that point
/// the driver's executor at the checked-in stand-in.
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
        "--claude-program".to_string(),
        FAKE_SPAWNER.to_string(),
    ]
}

/// Spawn a real detached driver and immediately let go of its handle.
///
/// `tokio::process::Command` with `kill_on_drop(false)` and `process_group(0)`,
/// then the handle is dropped — which is what makes [`ReapArm::Adopted`] the
/// honest arm for these tests. Dropping it hands the `wait()` to Tokio's orphan
/// queue, which stands in for init exactly as init stands in for a TUI that has
/// restarted; this test process holds no `Child` and could not `wait()` even if
/// `stop_run` wanted to.
///
/// `kill_on_drop` must stay `false` or the drop below would SIGKILL the driver
/// on the spot and every assertion afterwards would be about a process nothing
/// stopped.
fn spawn_detached_driver(fixture: &Fixture, run_id: &str) -> u32 {
    let mut cmd = tokio::process::Command::new(env!("CARGO_BIN_EXE_gsd-meta-manager"));
    cmd.args(drive_args(&fixture.config_path, run_id));
    cmd.current_dir(fixture.root());
    cmd.stdin(std::process::Stdio::null());
    cmd.stdout(std::process::Stdio::null());
    cmd.stderr(std::process::Stdio::null());
    cmd.kill_on_drop(false);
    cmd.as_std_mut().process_group(0);

    let child = cmd.spawn().expect("the driver binary is built and spawnable");
    let pid = child.id().expect("a freshly spawned child has a pid");
    drop(child);
    pid
}

/// Poll until the run's driver is observably live.
async fn live_within(pid: u32, run_id: &str, limit: Duration) -> bool {
    let deadline = tokio::time::Instant::now() + limit;
    while tokio::time::Instant::now() < deadline {
        if liveness::is_run_alive(pid, run_id) {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    false
}

/// Poll until the run has written its `run.json`.
///
/// **`live_within` is not a sufficient gate for anything that reads disk, and
/// the gap is real rather than theoretical.** `is_run_alive` answers off
/// `/proc/<pid>/cmdline`, which carries `--run-id` from the moment of `exec` —
/// so a driver is "live" while it is still establishing its process group,
/// taking its lock and starting its journal, all of which precede the first
/// `run.json` write. A test that read the record straight after `live_within`
/// was racing the driver's own startup and failed intermittently under load.
async fn recorded_within(run_json: &Path, limit: Duration) -> bool {
    let deadline = tokio::time::Instant::now() + limit;
    loop {
        if run_json.is_file() {
            return true;
        }
        if tokio::time::Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

/// Poll until the run's driver is gone.
async fn gone_within(pid: u32, run_id: &str, limit: Duration) -> bool {
    let deadline = tokio::time::Instant::now() + limit;
    loop {
        if !liveness::is_run_alive(pid, run_id) {
            return true;
        }
        if tokio::time::Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

/// Every parsed record currently in `journal`.
fn journal_records(journal: &Path) -> Vec<JournalRecord> {
    let Ok(read) = reader::tail_lines(journal, TailCursor::default()) else {
        return Vec::new();
    };
    read.lines
        .iter()
        .filter_map(|line| match reader::parse_line(line) {
            ParsedLine::Record(record) => Some(record),
            ParsedLine::Unparseable { .. } => None,
        })
        .collect()
}

/// Poll the journal until `extract` yields a value.
async fn journal_yields<T>(
    journal: &Path,
    limit: Duration,
    extract: impl Fn(&[JournalRecord]) -> Option<T>,
) -> Option<T> {
    let deadline = tokio::time::Instant::now() + limit;
    loop {
        if let Some(value) = extract(&journal_records(journal)) {
            return Some(value);
        }
        if tokio::time::Instant::now() >= deadline {
            return None;
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
}

/// The grandchild pid the stand-in announced on the stream.
///
/// The announcement uses a message `type` no CLI version emits, so the tolerant
/// parser carries it through as a forward-compatible unknown **with its raw text
/// intact**, and the journal records it as an `exec_event` on the `unknown`
/// stream. That is the only route by which a pid chosen at runtime by a shell
/// script can reach a test.
fn announced_grandchild(records: &[JournalRecord]) -> Option<u32> {
    for record in records {
        if record.kind != "exec_event" {
            continue;
        }
        let text = record.rest.get("text")?.as_str().unwrap_or_default();
        if !text.contains("grandchild_announcement") {
            continue;
        }
        let rest = text.split("\"pid\":").nth(1)?;
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        if let Ok(pid) = digits.parse::<u32>() {
            return Some(pid);
        }
    }
    None
}

/// The `claude` process group id, off the `exec_started` record (D-09).
///
/// This value exists on the journal precisely so a stop can be checked — and so
/// a SIGKILLed driver does not orphan an untraceable tree. It is the group this
/// test probes for survivors, and it is **not** the driver's group: that
/// distinction is the whole reason the teardown is two-layer.
fn journaled_claude_pgid(records: &[JournalRecord]) -> Option<u32> {
    records
        .iter()
        .filter(|record| record.kind == "exec_started")
        .find_map(|record| record.rest.get("claude_pgid")?.as_u64())
        .map(|pgid| pgid as u32)
}

/// Every pid currently in the process group `pgid`, with its state character.
///
/// A `/proc` scan rather than a signal probe, for two independent reasons. The
/// zero signal reports a **zombie** as alive, and a surviving zombie is one of
/// the four things criterion #1 forbids — so a probe that cannot see one is
/// useless here. And `kill(-pgid, 0)` answers a yes/no where a failure wants to
/// name which pids survived and in what state.
///
/// Field 5 of `/proc/<pid>/stat` is the process group. The parse skips past the
/// **last** `)` for the same reason `driver::liveness::process_state` does: the
/// `comm` field is unescaped and may itself contain parentheses, so counting
/// tokens from the start of the line is the classic way to read the wrong field.
fn group_members(pgid: u32) -> Vec<(u32, char)> {
    let Ok(entries) = std::fs::read_dir("/proc") else {
        return Vec::new();
    };
    let mut found = Vec::new();
    for entry in entries.flatten() {
        let name = entry.file_name();
        let Some(pid) = name.to_string_lossy().parse::<u32>().ok() else {
            continue;
        };
        let Ok(stat) = std::fs::read_to_string(format!("/proc/{pid}/stat")) else {
            continue;
        };
        let Some(after_comm) = stat.rfind(')').and_then(|at| at.checked_add(2)) else {
            continue;
        };
        let Some(tail) = stat.get(after_comm..) else {
            continue;
        };
        let mut fields = tail.split_whitespace();
        let state = fields.next().and_then(|f| f.chars().next()).unwrap_or('?');
        // state, ppid, pgrp.
        let Some(Ok(group)) = fields.nth(1).map(str::parse::<u32>) else {
            continue;
        };
        if group == pgid {
            found.push((pid, state));
        }
    }
    found
}

/// Whether `pid` has any process-table entry at all — including as a zombie.
fn present(pid: u32) -> bool {
    liveness::process_state(pid).is_some()
}

#[tokio::test]
async fn stopping_a_run_leaves_no_claude_no_grandchild_and_no_zombie() {
    // If the grace ever grows past the point at which the criterion verifies,
    // the criterion becomes unverifiable by construction — the stop would still
    // be in progress when the check runs. Failing here, loudly, is better than
    // a test that quietly starts measuring nothing.
    assert!(
        VERIFY_AFTER > DRIVER_TEARDOWN_GRACE,
        "criterion #1 verifies at {VERIFY_AFTER:?}, which must stay above the \
         teardown grace ({DRIVER_TEARDOWN_GRACE:?})"
    );

    const RUN_ID: &str = "2026-07-29T13-00-00Z-kill";
    let fixture = Fixture::new("criterion #1");

    let driver_pid = spawn_detached_driver(&fixture, RUN_ID);
    assert!(
        live_within(driver_pid, RUN_ID, STARTUP_LIMIT).await,
        "the driver never came up; nothing after this point would mean anything"
    );

    let journal = fixture.journal(RUN_ID);

    // Level 3: the pid of a real process the fake agent backgrounded.
    let grandchild = journal_yields(&journal, STARTUP_LIMIT, announced_grandchild)
        .await
        .expect("the spawner stand-in must announce its grandchild's pid on the stream");

    // Level 2: the SECOND process group. `kill(-driver_pgid, …)` does not reach
    // it, which is why layer 2 of the teardown has to exist at all (D-06, D-09).
    let claude_pgid = journal_yields(&journal, STARTUP_LIMIT, journaled_claude_pgid)
        .await
        .expect("the exec_started record must carry the claude process group id");
    assert_ne!(
        claude_pgid, driver_pid,
        "the agent must lead a process group of its own, distinct from the \
         driver's — if these were equal the whole two-layer design would be \
         unnecessary and this test would prove nothing about it"
    );

    // The preconditions. A test that never sees the tree alive proves nothing
    // about killing it, and both halves are asserted rather than assumed.
    assert!(
        present(grandchild),
        "precondition: the grandchild {grandchild} must be running BEFORE the stop"
    );
    assert!(
        !group_members(claude_pgid).is_empty(),
        "precondition: the agent's process group {claude_pgid} must have members \
         before the stop"
    );

    // `Adopted` because this test process holds no `Child` — it dropped the
    // handle at spawn — which is also the arm a restarted TUI takes, and
    // therefore the one worth exercising here (D-07).
    let outcome = kill::stop_run(driver_pid, driver_pid, RUN_ID, ReapArm::Adopted).await;
    assert!(
        matches!(
            outcome,
            StopOutcome::ExitedOnTerminate | StopOutcome::ExitedAfterKill
        ),
        "the stop must report the driver dead, got {outcome:?}"
    );

    tokio::time::sleep(VERIFY_AFTER).await;

    // ---- The four halves of criterion #1, fifteen seconds after the stop ----

    assert!(
        !present(driver_pid),
        "the driver {driver_pid} is still in the process table {VERIFY_AFTER:?} \
         after the stop, in state {:?}",
        liveness::process_state(driver_pid)
    );

    let survivors = group_members(claude_pgid);
    assert!(
        survivors.is_empty(),
        "the agent's process group {claude_pgid} still has members: {survivors:?}. \
         This is the failure D-06 exists to prevent — the driver was stopped and \
         `claude` was not, because they are two different process groups and a \
         signal to one does not reach the other"
    );

    assert!(
        !present(grandchild),
        "the grandchild {grandchild} outlived the stop, in state {:?}. A stop that \
         reaches only the direct child orphans every build server, dev server and \
         long-lived shell the agent started — invisible to the user and holding \
         files, ports and quota they did not consent to (T-15-30)",
        liveness::process_state(grandchild)
    );

    // The zombie half, through a `/proc` state read. The existing `alive()`
    // helper uses the zero signal, which SUCCEEDS for a zombie and would report
    // every assertion below as satisfied while the process table filled up.
    for (label, pid) in [
        ("the driver", driver_pid),
        ("the agent", claude_pgid),
        ("the grandchild", grandchild),
    ] {
        assert!(
            !liveness::is_zombie(pid),
            "{label} ({pid}) is a ZOMBIE {VERIFY_AFTER:?} after the stop. It was \
             torn down but never reaped, which criterion #1 forbids in as many \
             words and which the zero-signal helper cannot detect"
        );
    }
}

#[tokio::test]
async fn the_adopted_arm_confirms_death_by_probing_proc_rather_than_by_wait() {
    // D-07's second arm. The driver here is not this process's child in any
    // usable sense: the `Child` was dropped at spawn, so `wait()` from here is
    // not available at all — and for a genuinely adopted run, a run rediscovered
    // after a TUI restart, it would return ECHILD. Death can therefore be
    // confirmed **only** by re-probing `/proc/<pid>` until the pid disappears.
    // Without that re-probe this test either hangs until the timeout below fires
    // or reports a signal failure; both are the failure it exists to catch.
    const RUN_ID: &str = "2026-07-29T13-00-01Z-adopt";
    let fixture = Fixture::new("adopted arm");

    let driver_pid = spawn_detached_driver(&fixture, RUN_ID);
    assert!(
        live_within(driver_pid, RUN_ID, STARTUP_LIMIT).await,
        "the driver never came up"
    );
    // Gated on the record and not merely on liveness, so the stop lands on a run
    // that has actually reached its drain loop — see `recorded_within`.
    assert!(
        recorded_within(
            &gsd_meta_manager::journal::run_paths(&fixture.planning(), RUN_ID).run_json,
            STARTUP_LIMIT
        )
        .await,
        "the driver never wrote its run record"
    );

    // A bound slightly above the grace. Plan 17-02 recorded that
    // `tokio::time::timeout` cannot bound a **blocking syscall** on the runtime
    // it shares, because `Timeout::poll` polls the inner future inline — the
    // suite hangs instead of failing. That hazard does not apply here and the
    // absence is a choice, not luck: `stop_run` is genuinely async, yielding at
    // a `tokio::time::sleep` between every `/proc` read, so the timer is polled
    // on schedule. Its only synchronous work is a handful of small file reads.
    let bound = DRIVER_TEARDOWN_GRACE + Duration::from_secs(5);
    let outcome = tokio::time::timeout(
        bound,
        kill::stop_run(driver_pid, driver_pid, RUN_ID, ReapArm::Adopted),
    )
    .await
    .unwrap_or_else(|_| {
        panic!(
            "the adopted arm must confirm death by probing /proc and return within \
             {bound:?}. A timeout here means it is waiting for something that will \
             never arrive — a `wait()` on a process this one cannot reap (D-07)"
        )
    });

    assert_eq!(
        outcome,
        StopOutcome::ExitedOnTerminate,
        "the driver handles the terminate signal, so the adopted arm must observe \
         it gone within the grace rather than escalating or reporting a signal \
         failure"
    );
    assert!(
        !liveness::is_run_alive(driver_pid, RUN_ID),
        "the outcome claims the driver exited, so the probe must agree"
    );
}

#[tokio::test]
async fn a_stopped_run_leaves_a_terminal_record_and_a_released_lock() {
    // Layer 3 of D-06: the driver finishes its journal and releases its lock
    // before it exits, so a stopped run has a terminal record rather than
    // looking like a crash.
    const RUN_ID: &str = "2026-07-29T13-00-02Z-record";
    let fixture = Fixture::new("terminal record");

    let driver_pid = spawn_detached_driver(&fixture, RUN_ID);
    assert!(
        live_within(driver_pid, RUN_ID, STARTUP_LIMIT).await,
        "the driver never came up"
    );

    // The run must be un-ended before the stop, or nothing below is about a stop.
    let run_json = gsd_meta_manager::journal::run_paths(&fixture.planning(), RUN_ID).run_json;
    assert!(
        recorded_within(&run_json, STARTUP_LIMIT).await,
        "the driver never wrote its run record; a stop against a run that has not \
         started yet proves nothing about terminal records"
    );
    let before: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&run_json).expect("run.json is on disk"))
            .expect("run.json parses");
    assert!(
        before.get("ended_at").map(|v| v.is_null()).unwrap_or(true),
        "precondition: the run must still be running when it is stopped"
    );

    let outcome = kill::stop_run(driver_pid, driver_pid, RUN_ID, ReapArm::Adopted).await;
    assert!(
        matches!(
            outcome,
            StopOutcome::ExitedOnTerminate | StopOutcome::ExitedAfterKill
        ),
        "got {outcome:?}"
    );
    assert!(
        gone_within(driver_pid, RUN_ID, Duration::from_secs(10)).await,
        "the driver must be gone before its final writes are inspected"
    );

    let after: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&run_json).expect("run.json is on disk"))
            .expect("run.json parses");

    // **The property that distinguishes a stop from a crash.** Phase 16's
    // contract is that an absent `ended_at` beside a dead pid IS the crash
    // record; a stop that skipped its terminal write would be indistinguishable
    // from one, and `src/driver/reconcile.rs` would report every stopped run as
    // crashed on the next scan.
    let ended_at = after
        .get("ended_at")
        .and_then(|v| v.as_str())
        .expect("a stopped run must carry an ended_at — without it the stop is \
                 indistinguishable on disk from a crash (D-06.3, D-12)");
    assert!(!ended_at.is_empty());

    assert_eq!(
        after.get("outcome").and_then(|v| v.as_str()),
        Some("killed"),
        "a stop is recorded as killed, never as failed: reporting the user's own \
         deliberate action back to them as a failure is the specific way this \
         record goes wrong"
    );

    let records = journal_records(&fixture.journal(RUN_ID));
    assert_eq!(
        records.last().map(|record| record.kind.as_str()),
        Some("run_ended"),
        "the run-ended record is always the last one, got: {:?}",
        records.iter().map(|r| &r.kind).collect::<Vec<_>>()
    );
    assert!(
        records
            .iter()
            .any(|record| record.kind == "diagnostic"
                && record.rest.get("code").and_then(|v| v.as_str())
                    == Some("terminate_signal_shutdown")),
        "the reason must survive alongside the ending, or a later reader can see \
         that a run was killed but not that a terminate signal is what arrived"
    );

    assert_eq!(
        gsd_meta_manager::journal::writer::read_active_run(&fixture.planning()),
        None,
        "the active pointer must be cleared by the terminal write; a stale one is \
         half of the crash signal and would make this run look crashed"
    );

    // The lock is released by the descriptor closing when `DriverRun` drops,
    // which happens **after** the terminal write. A fresh acquire from this
    // process is the only way to prove the release actually happened — the file
    // still exists either way, so its presence proves nothing.
    let reacquired = lock::acquire(&fixture.planning(), "a-later-run", std::process::id())
        .expect(
            "the stopped run must have released its lock, or the project is \
             permanently unstartable until the machine is rebooted (D-20.2)",
        );
    drop(reacquired);
}
