// ============================================================================
// A stop issued while the agent is still STARTING is acted on (CR-01, CTRL-01).
//
// The finding this file closes, and the reproduction:
//
//   A stop issued while the driver was still inside `Executor::start()` was
//   swallowed. `tokio::signal::unix::signal` replaces SIGTERM's default
//   disposition the moment it is constructed — at the very top of `execute_run`
//   — but nothing polled `term.recv()` until the drain loop some two hundred
//   lines later. So the driver SURVIVED the TUI's group SIGTERM, the TUI waited
//   out its twelve-second grace and escalated to SIGKILL, and the `claude` group
//   — a DIFFERENT process group, which had never been signalled and whose driver
//   was now dead — was orphaned along with its grandchildren.
//
// **`tests/driver_kill.rs` cannot see this.** Every test there is built on
// `fake-claude-spawner.sh`, which emits `system/init` immediately, so the
// capability gate opens at once and the driver is already in its drain loop by
// the time any stop arrives. That is the one path the terminate signal was
// always raced on correctly. The window is only reachable with a stand-in that
// never emits `system/init`, which is what `fake-claude-silent.sh` is for — and
// with a real `claude` the window is minutes rather than microseconds whenever
// the documented `SessionStart` hook hang reproduces
// (`src/executor/mod.rs:225-239`).
//
// Three real process levels, for the same reason `tests/driver_kill.rs` needs
// them: the driver, its fake `claude` leading a second process group, and that
// agent's backgrounded grandchild. **The grandchild is the point** — killing the
// direct child alone looks identical to killing the tree unless something
// outlives the leader and can be checked afterwards.
//
// The zombie half is read from `/proc/<pid>/stat` through
// `driver::liveness::is_zombie` and **never** through a `kill -0` probe: the
// zero signal succeeds for a zombie, which still owns a pid and a process-table
// entry, so it reports a zombie as ALIVE and cannot answer this question at all.
// There is no `kill -0` in this file.
//
// Unix-only by construction (D-05).
// ============================================================================

#![cfg(unix)]

use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::time::Duration;

use gsd_meta_manager::config::{save_config, Config, DriverOptIn, RegisteredProject};
use gsd_meta_manager::driver::kill::{self, ReapArm, StopOutcome, DRIVER_TEARDOWN_GRACE};
use gsd_meta_manager::driver::liveness;
use gsd_meta_manager::journal::reader::{self, JournalRecord, ParsedLine, TailCursor};
use tempfile::TempDir;

/// The stand-in that never emits `system/init`, so the capability gate never
/// opens and `Executor::start` never returns. That omission is the fixture.
const FAKE_SILENT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/fake-claude-silent.sh"
);

/// How long a driver is given to come up before a test gives up on it.
const STARTUP_LIMIT: Duration = Duration::from_secs(30);

/// A bound on the stop itself, so a regression fails rather than hangs.
///
/// Comfortably above [`DRIVER_TEARDOWN_GRACE`] plus its escalation window: the
/// point of the elapsed-time assertion below is to measure where inside the grace
/// the stop landed, and a timeout that fired first would replace that measurement
/// with a panic that says nothing about it.
const STOP_BOUND: Duration = Duration::from_secs(20);

const ALIAS: &str = "stoppable";
const COMMAND: &str = "/gsd-progress";
const RUN_ID: &str = "2026-07-29T14-00-00Z-startup";

/// A project root with a `.planning/` directory, plus a config naming it opted
/// in.
///
/// Copied from `tests/driver_kill.rs` rather than shared, following this
/// repository's per-file integration-test convention: each file states the whole
/// of its own fixture, so reading one test does not mean reading two files.
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
        gsd_meta_manager::journal::run_paths(&self.planning(), run_id)
        .expect("the fixture run id is a plain path component")
        .journal
    }
}

/// The `drive` argument list, plus the two hidden development flags that point
/// the driver's executor at the silent stand-in and hand it the pid file.
fn drive_args(config_path: &Path, run_id: &str, pidfile: &Path) -> Vec<String> {
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
        FAKE_SILENT.to_string(),
        "--claude-args".to_string(),
        pidfile.display().to_string(),
    ]
}

/// Spawn a real detached driver and immediately let go of its handle.
///
/// `kill_on_drop` must stay `false`, or the drop below would SIGKILL the driver
/// on the spot and every assertion afterwards would be about a process nothing
/// stopped. Dropping the handle also makes [`ReapArm::Adopted`] the honest arm:
/// this process holds no `Child` and could not `wait()` even if `stop_run`
/// wanted to.
fn spawn_detached_driver(fixture: &Fixture, run_id: &str, pidfile: &Path) -> u32 {
    let mut cmd = tokio::process::Command::new(env!("CARGO_BIN_EXE_gsd-meta-manager"));
    cmd.args(drive_args(&fixture.config_path, run_id, pidfile));
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

/// Poll the pid file until it holds two parseable pids.
///
/// **The pid file is the only channel this fixture has.** The journal cannot
/// carry these two numbers: journal `exec_event` records are projected from the
/// executor's event drain, and the drain does not run until `start` returns —
/// which, by construction here, it never does.
async fn agent_pids_within(pidfile: &Path, limit: Duration) -> Option<(u32, u32)> {
    let deadline = tokio::time::Instant::now() + limit;
    loop {
        if let Ok(contents) = std::fs::read_to_string(pidfile) {
            let pids: Vec<u32> = contents
                .split_whitespace()
                .filter_map(|token| token.parse().ok())
                .collect();
            if let [agent, grandchild] = pids[..] {
                return Some((agent, grandchild));
            }
        }
        if tokio::time::Instant::now() >= deadline {
            return None;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

/// Poll until `run.json` exists.
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

/// Poll until a record of `kind` appears in the journal.
async fn journal_has_within(journal: &Path, kind: &str, limit: Duration) -> bool {
    let deadline = tokio::time::Instant::now() + limit;
    loop {
        if journal_records(journal)
            .iter()
            .any(|record| record.kind == kind)
        {
            return true;
        }
        if tokio::time::Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
}

/// Whether `pid` has any process-table entry at all — including as a zombie.
fn present(pid: u32) -> bool {
    liveness::process_state(pid).is_some()
}

/// Poll until `pid` has no process-table entry at all.
async fn absent_within(pid: u32, limit: Duration) -> bool {
    let deadline = tokio::time::Instant::now() + limit;
    loop {
        if !present(pid) {
            return true;
        }
        if tokio::time::Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

// `assertions_on_constants` is allowed here, and the lint firing is itself the
// evidence the assertion is doing its job. `LIVENESS_SUPPORTED` is a `const`
// precisely so the non-Linux answer is a *value* rather than an attribute
// (D-05); on this platform it is `true`, which is what clippy is observing, and
// on a platform where the /proc technique does not apply it is `false` and this
// test fails loudly instead of passing vacuously. Rewriting the assertion to
// hide the constant would defeat both halves.
#[allow(clippy::assertions_on_constants)]
#[tokio::test]
async fn a_stop_during_agent_startup_tears_down_the_agent_group_and_records_a_killed_run() {
    // The non-vacuity guard, first. Every assertion below is a `/proc` read;
    // where `process_state` always yields `None` they would all pass by
    // measuring nothing at all, and the zombie half would pass loudest of all.
    assert!(
        liveness::LIVENESS_SUPPORTED,
        "this test is a sequence of /proc reads. Where the /proc technique does \
         not apply, `process_state` yields None for every pid and every assertion \
         below passes VACUOUSLY — including the zombie half, which is exactly the \
         half a vacuous pass would hide (CR-05)"
    );

    let fixture = Fixture::new("a stop during startup");
    let pidfile = fixture.root().join("agent-pids");

    let driver_pid = spawn_detached_driver(&fixture, RUN_ID, &pidfile);
    assert!(
        live_within(driver_pid, RUN_ID, STARTUP_LIMIT).await,
        "the driver never came up; nothing after this point would mean anything"
    );

    // Levels 2 and 3: the agent — which leads a process group of its own, so its
    // pid IS its pgid — and the grandchild it backgrounded.
    let (agent, grandchild) = agent_pids_within(&pidfile, STARTUP_LIMIT)
        .await
        .expect("the silent stand-in must report its own pid and its grandchild's");
    assert_ne!(
        agent, driver_pid,
        "the agent must be a different process from the driver, and it leads a \
         different process group — that distinction is the whole reason the \
         teardown is two-layer (D-06, D-09)"
    );

    // **The precondition that makes this a STARTUP test rather than a duplicate
    // of `tests/driver_kill.rs`.**
    let run_json = gsd_meta_manager::journal::run_paths(&fixture.planning(), RUN_ID)
        .expect("the fixture run id is a plain path component")
        .run_json;
    assert!(
        recorded_within(&run_json, STARTUP_LIMIT).await,
        "the driver never wrote its run record, so the journal had not started and \
         a stop here would prove nothing about terminal records"
    );
    assert!(
        journal_has_within(&fixture.journal(RUN_ID), "run_started", STARTUP_LIMIT).await,
        "the journal must have opened before the stop"
    );
    assert!(
        !journal_records(&fixture.journal(RUN_ID))
            .iter()
            .any(|record| record.kind == "exec_started"),
        "there must be NO exec_started record when the stop is issued. That record \
         is projected from the executor's event drain, and the drain does not run \
         until `Executor::start` returns — so its absence is positive evidence \
         that the driver is parked inside startup. A test that stopped a run \
         already in its drain loop would prove nothing here, because that path \
         raced the terminate signal correctly all along"
    );

    // The tree is alive before the stop. A test that never sees it alive proves
    // nothing about killing it.
    assert!(
        present(agent),
        "precondition: the agent {agent} must be running BEFORE the stop"
    );
    assert!(
        present(grandchild),
        "precondition: the grandchild {grandchild} must be running BEFORE the stop"
    );

    let started = std::time::Instant::now();
    let outcome = tokio::time::timeout(
        STOP_BOUND,
        kill::stop_run(driver_pid, driver_pid, RUN_ID, ReapArm::Adopted),
    )
    .await
    .unwrap_or_else(|_| {
        panic!(
            "the stop did not return within {STOP_BOUND:?}. A hang here is a \
             regression that fails as a timeout instead of as an assertion, which \
             is why the bound exists at all"
        )
    });
    let elapsed = started.elapsed();

    // ---- (a) The signal was ACTED ON, not swallowed ----
    //
    // `ExitedOnTerminate` **exactly**, and never a tolerant `matches!` over the
    // clean outcome and the escalated one. The escalated outcome is precisely
    // what the defect produced — the driver ignored SIGTERM, outlasted the whole
    // grace, and was SIGKILLed — so an assertion that accepted both would pass
    // against the very bug it exists to catch. `assert_eq!` prints the variant it
    // actually got, so nothing is lost by not naming it here.
    assert_eq!(
        outcome,
        StopOutcome::ExitedOnTerminate,
        "a stop issued while the agent was still starting must be handled by the \
         driver's own terminate arm and reported as a CLEAN exit. An escalated \
         outcome means the driver survived its terminate signal and had to be \
         killed, which is the swallowed-signal defect (CR-01)"
    );
    assert!(
        elapsed < DRIVER_TEARDOWN_GRACE,
        "the stop took {elapsed:?}, which is at or above the teardown grace \
         ({DRIVER_TEARDOWN_GRACE:?}). An elapsed time that reaches the grace means \
         the driver outlasted its terminate signal and was killed rather than \
         stopping — the driver is then SIGKILLed mid-startup and the agent group \
         it never signalled is orphaned"
    );

    // ---- (b) The tree is gone, and is not a zombie ----
    //
    // Polled rather than read once: the driver is the agent's parent and exits
    // without reaping it, so there is a real window in which the agent is a `Z`
    // waiting for init to adopt it.
    assert!(
        absent_within(agent, Duration::from_secs(10)).await,
        "the agent {agent} outlived the stop, in state {:?}",
        liveness::process_state(agent)
    );
    assert!(
        absent_within(grandchild, Duration::from_secs(10)).await,
        "the grandchild {grandchild} outlived the stop, in state {:?}. A stop that \
         reaches only the direct child orphans every build server, dev server and \
         long-lived shell the agent started — invisible to the user and holding \
         files, ports and quota they did not consent to (T-15-30)",
        liveness::process_state(grandchild)
    );
    for (label, pid) in [("the agent", agent), ("the grandchild", grandchild)] {
        assert!(
            !liveness::is_zombie(pid),
            "{label} ({pid}) is a ZOMBIE after the stop: torn down but never \
             reaped, which criterion #1 forbids in as many words"
        );
        assert!(!present(pid), "{label} ({pid}) is still in the process table");
    }
    assert!(
        absent_within(driver_pid, Duration::from_secs(10)).await,
        "the driver {driver_pid} is still in the process table, in state {:?}",
        liveness::process_state(driver_pid)
    );

    // ---- (c) A startup stop is distinguishable on disk from a crash ----
    let after: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&run_json).expect("run.json is on disk"))
            .expect("run.json parses");
    let ended_at = after.get("ended_at").and_then(|v| v.as_str()).expect(
        "a run stopped during startup must still carry an ended_at — without it \
         the stop is indistinguishable on disk from a crash, and every scan would \
         report the user's own deliberate stop as a death (D-06.3, D-12)",
    );
    assert!(!ended_at.is_empty());
    assert_eq!(
        after.get("outcome").and_then(|v| v.as_str()),
        Some("killed"),
        "a stop is recorded as killed, never as failed or spawn_failed: the run \
         was stopped, not broken"
    );

    let records = journal_records(&fixture.journal(RUN_ID));
    assert_eq!(
        records.last().map(|record| record.kind.as_str()),
        Some("run_ended"),
        "the run-ended record is always the last one, got: {:?}",
        records.iter().map(|r| &r.kind).collect::<Vec<_>>()
    );
    assert!(
        records.iter().any(|record| record.kind == "diagnostic"
            && record.rest.get("code").and_then(|v| v.as_str())
                == Some("terminate_signal_shutdown")),
        "the reason must survive alongside the ending, under the SAME code the \
         drain-loop path writes — a second code for 'stopped, but earlier' would \
         split one question across two searches"
    );

    assert_eq!(
        gsd_meta_manager::journal::writer::read_active_run(&fixture.planning()),
        None,
        "the active pointer must be cleared by the terminal write; a stale one is \
         half of the crash signal and would make this stopped run look crashed on \
         the next reconciliation scan"
    );
}
