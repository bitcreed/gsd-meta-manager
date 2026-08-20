// ============================================================================
// The drive tracer: argv → opt-in gate → journal start → executor spawn →
// event drain → journal finish → exit, and the refusal that writes nothing.
//
// This is an integration test rather than an in-source one for two reasons that
// both come down to needing the real world: it spawns a genuine child process
// through the checked-in shell stand-in, and it builds a real `Config` whose
// registry entry points at a real directory on disk. Neither is something a
// unit test in `src/` should be doing.
//
// Unix-only by construction: driving is a Unix capability (D-05), and the run
// body is `#[cfg(unix)]`.
// ============================================================================

#![cfg(unix)]

use std::ffi::OsString;
use std::path::Path;

use gsd_meta_manager::config::{Config, DriverOptIn, RegisteredProject};
use gsd_meta_manager::driver::{drive, DriveArgs};
use serde_json::Value;
use tempfile::TempDir;

const FAKE_CLAUDE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/fake-claude.sh");
/// The clean-success capture named in `tests/fixtures/transcripts/README.md`.
const CLEAN_BASELINE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/transcripts/01-success-textonly.ndjson"
);

const ALIAS: &str = "tracer";
/// A fixed run id so the test knows where to read, rather than globbing.
const RUN_ID: &str = "2026-07-29T12-00-00Z-a3f9";

/// A project root with a `.planning/` directory.
fn project_root() -> TempDir {
    let root = TempDir::new().expect("temp dir");
    std::fs::create_dir_all(root.path().join(".planning")).expect("scratch .planning");
    root
}

/// Point the envelope at a temp root for this test binary.
///
/// **Since plan 19-07 a driven run establishes its envelope before the executor
/// is constructed**, and the envelope lives under the application data
/// directory. Without this redirect these fixtures would write hook stubs, a
/// generated git config and a settings file into the developer's real
/// `~/.local/share` under a fixture's alias — the same "a test may not write
/// into the developer's real data directory" rule `envelope::hooks::guard_in`
/// records for its own explicit-root sibling.
///
/// The `set_var` happens exactly **once** per test binary, inside the
/// `OnceLock` initialiser, and the `TempDir` is held by the `static` for the
/// process lifetime so the root outlives every test that drives a run.
fn isolate_envelope_root() {
    static ROOT: std::sync::OnceLock<TempDir> = std::sync::OnceLock::new();
    ROOT.get_or_init(|| {
        let dir = TempDir::new().expect("an envelope temp root");
        std::env::set_var(gsd_meta_manager::envelope::ENVELOPE_ROOT_ENV, dir.path());
        dir
    });
}

/// A one-entry registry pointing at `root`, opted in or not.
fn config_for(root: &Path, opted_in: bool) -> Config {
    isolate_envelope_root();
    let mut config = Config::new();
    config.projects.insert(
        ALIAS.to_string(),
        RegisteredProject {
            path: root.to_path_buf(),
            added: "2026-07-29T12:00:00Z".to_string(),
            driver_opt_in: opted_in.then(|| DriverOptIn {
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

/// Drive arguments pointed at the transcript-replaying stand-in.
fn args() -> DriveArgs {
    DriveArgs {
        alias: ALIAS.to_string(),
        command: Some("/gsd-progress".to_string()),
        target_phase: None,
        max_steps: None,
        wall_clock_cap_secs: None,
        max_escalations: None,
        run_id: Some(RUN_ID.to_string()),
        dry_run: false,
        goal: Some("prove the spine".to_string()),
        claude_program: Some(FAKE_CLAUDE.into()),
        claude_args: vec![OsString::from(CLEAN_BASELINE), OsString::from("0")],
    }
}

/// Run one command against `root` to completion.
async fn drive_once(root: &Path) {
    let config = config_for(root, true);
    drive(args(), &config)
        .await
        .expect("an opted-in project driving the clean baseline must complete");
}

/// The parsed `run.json` for the fixed run id under `root`.
fn run_record(root: &Path) -> Value {
    let paths = gsd_meta_manager::journal::run_paths(&root.join(".planning"), RUN_ID)
        .expect("the fixture run id is a plain path component");
    let raw = std::fs::read_to_string(&paths.run_json).expect("run.json exists and is readable");
    serde_json::from_str(&raw).expect("run.json parses")
}

/// Every record in the fixed run's journal, in order.
fn journal_records(root: &Path) -> Vec<Value> {
    let paths = gsd_meta_manager::journal::run_paths(&root.join(".planning"), RUN_ID)
        .expect("the fixture run id is a plain path component");
    std::fs::read_to_string(&paths.journal)
        .expect("journal.jsonl exists and is readable")
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("every journal line parses"))
        .collect()
}

#[tokio::test]
async fn a_drive_against_an_opted_in_project_leaves_a_complete_run_on_disk() {
    let root = project_root();
    drive_once(root.path()).await;

    let paths = gsd_meta_manager::journal::run_paths(&root.path().join(".planning"), RUN_ID)
        .expect("the fixture run id is a plain path component");
    assert!(paths.dir.is_dir(), "the run directory must exist on disk");

    let record = run_record(root.path());
    assert!(
        record["ended_at"].is_string(),
        "a finished run stamps ended_at; absent is the crash signal (D-12), got: {}",
        record["ended_at"]
    );
    assert!(
        record["outcome"].is_string(),
        "a finished run records its derived outcome, got: {}",
        record["outcome"]
    );
    assert_eq!(
        record["gsd_command"], "/gsd-progress",
        "the record carries the command the run was started with"
    );
    assert_eq!(
        record["opt_in"], "2026-07-29T11:59:00Z",
        "the opt-in timestamp lands in the field Phase 16 reserved for it"
    );

    let records = journal_records(root.path());
    assert_eq!(
        records.first().expect("the journal is non-empty")["kind"],
        "run_started",
        "the journal opens with run_started"
    );
    assert_eq!(
        records.last().expect("the journal is non-empty")["kind"],
        "run_ended",
        "run_ended is always the last record"
    );

    assert!(
        !paths.active.exists(),
        "finish clears the active pointer; a stale pointer would read as a live run"
    );
}

#[tokio::test]
async fn a_drive_against_a_project_with_no_opt_in_record_is_refused_and_writes_nothing() {
    let root = project_root();
    let config = config_for(root.path(), false);

    let err = drive(args(), &config)
        .await
        .expect_err("a project with no opt-in record must never be driven");
    assert!(
        err.to_string().contains(ALIAS),
        "the refusal must name the alias so it is actionable, got: {err}"
    );

    // The refusal is proved by the ABSENCE OF A PATH, not merely by an `Err`.
    // Nothing at all was written: no runs root, no gitignore, no journal
    // (CTRL-03).
    let meta_manager = root.path().join(".planning").join("meta-manager");
    assert!(
        !meta_manager.exists(),
        "a refused drive writes nothing at all, but {} exists",
        meta_manager.display()
    );
}

#[tokio::test]
async fn the_run_record_carries_the_drivers_own_pid_and_an_equal_pgid() {
    let root = project_root();
    drive_once(root.path()).await;

    let record = run_record(root.path());
    let pid = record["pid"].as_u64().expect("pid is a number");
    let pgid = record["pgid"].as_u64().expect("pgid is a number");

    assert_ne!(pid, 0, "a zero pid names no process");
    assert_eq!(
        pid, pgid,
        "the driver is its own group leader, so pgid == pid — established by \
         setpgid(0, 0) at run entry rather than assumed (D-04)"
    );
}

#[tokio::test]
async fn the_exec_started_event_carries_the_claude_process_group_id() {
    let root = project_root();
    drive_once(root.path()).await;

    let records = journal_records(root.path());
    let exec_started = records
        .iter()
        .find(|record| record["kind"] == "exec_started")
        .expect("a run that spawned an agent journals exec_started");

    let pgid = exec_started["claude_pgid"]
        .as_u64()
        .expect("the claude process group id is present on exec_started (D-09)");
    assert_ne!(
        pgid, 0,
        "a zero pgid is no teardown handle at all; a SIGKILLed driver would \
         orphan an untraceable claude tree"
    );
}
