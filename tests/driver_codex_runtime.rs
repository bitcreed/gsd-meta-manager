// ============================================================================
// The Codex runtime, end to end (260922-hdj): registry key → `drive` → runtime
// resolution → `AgentExecutor::Codex` → `codex exec` (the logging stand-in) →
// the shared coordinator → journal and `run.json`.
//
// The stand-in is `tests/fixtures/fake-codex.sh`, which replays a captured
// `codex exec --json` transcript and logs the argv, what its stdin is, and the
// NAMES of the agent-family variables it inherited. No test here runs the real
// `codex`.
//
// Unix-only by construction, like every driving test (D-05).
// ============================================================================

#![cfg(unix)]

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use gsd_meta_manager::config::{Config, DriverOptIn, RegisteredProject};
use gsd_meta_manager::driver::{drive, DriveArgs};
use serde_json::{Map, Value};
use tempfile::TempDir;

const FAKE_CODEX: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/fake-codex.sh");
const SUCCESS: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/codex/01-exec-success.jsonl"
);

const ALIAS: &str = "codexed";
/// A fixed run id so the test knows where to read, rather than globbing.
const RUN_ID: &str = "2026-09-22T12-00-00Z-c0de";
/// The thread id on the first line of the success transcript.
const SUCCESS_THREAD: &str = "01a0ca2f-8a0c-72c0-87fb-11c28739d140";

fn nonblank(raw: &str) -> gsd_meta_manager::driver::payload::NonBlank {
    gsd_meta_manager::driver::payload::NonBlank::new(raw)
        .expect("a visible test literal is a payload")
}

/// Point the envelope at a temp root for this test binary, exactly once — the
/// same isolation `tests/driver_tracer.rs` documents.
fn isolate_envelope_root() {
    static ROOT: std::sync::OnceLock<TempDir> = std::sync::OnceLock::new();
    ROOT.get_or_init(|| {
        let dir = TempDir::new().expect("an envelope temp root");
        std::env::set_var(gsd_meta_manager::envelope::ENVELOPE_ROOT_ENV, dir.path());
        dir
    });
}

/// A project root with a `.planning/` directory, initialised as a git
/// repository so `<root>/.git` is the directory the Codex sandbox is given.
fn project_root() -> TempDir {
    let root = TempDir::new().expect("temp dir");
    std::fs::create_dir_all(root.path().join(".planning")).expect("scratch .planning");
    let status = std::process::Command::new("git")
        .args(["init", "-q"])
        .current_dir(root.path())
        .status()
        .expect("git runs");
    assert!(status.success(), "git init");
    root
}

/// A one-entry, opted-in registry pointing at `root`, whose entry carries
/// `entry_extra` and whose preferences carry `preference_extra`.
fn config_for(
    root: &Path,
    entry_extra: Map<String, Value>,
    preference_extra: Map<String, Value>,
) -> Config {
    isolate_envelope_root();
    let mut config = Config::new();
    config.preferences.extra = preference_extra;
    config.projects.insert(
        ALIAS.to_string(),
        RegisteredProject {
            path: root.to_path_buf(),
            added: "2026-09-22T12:00:00Z".to_string(),
            driver_opt_in: Some(DriverOptIn {
                opted_in_at: "2026-09-22T11:59:00Z".to_string(),
                claude_md_digest: None,
                prompt_inputs: gsd_meta_manager::registry::current_prompt_inputs(root),
                extra: Default::default(),
                branch_namespace: None,
                credential: None,
                pr_cap_per_24h: None,
                pr_cap_per_run: None,
            }),
            extra: entry_extra,
        },
    );
    config
}

/// `{"<key>": "<value>"}`.
fn one(key: &str, value: &str) -> Map<String, Value> {
    let mut map = Map::new();
    map.insert(key.to_string(), Value::from(value));
    map
}

/// Drive arguments pointed at the logging stand-in.
fn args(transcript: &str, exit_code: &str, log_dir: &Path) -> DriveArgs {
    DriveArgs {
        alias: nonblank(ALIAS),
        command: Some(nonblank("/gsd-progress")),
        target_phase: None,
        max_steps: None,
        wall_clock_cap_secs: None,
        max_escalations: None,
        approved_plan: None,
        run_id: Some(nonblank(RUN_ID)),
        dry_run: false,
        goal: None,
        claude_program: Some(FAKE_CODEX.into()),
        claude_args: vec![
            OsString::from(transcript),
            OsString::from(exit_code),
            log_dir.as_os_str().to_os_string(),
        ],
    }
}

/// The stand-in's log directory, fresh per test.
fn log_dir() -> TempDir {
    TempDir::new().expect("a log dir")
}

/// The lines of one stand-in log file.
fn logged(dir: &Path, name: &str) -> Vec<String> {
    std::fs::read_to_string(dir.join(name))
        .unwrap_or_else(|err| panic!("{name} log is readable: {err}"))
        .lines()
        .map(str::to_string)
        .collect()
}

fn run_paths(root: &Path) -> gsd_meta_manager::journal::RunPaths {
    gsd_meta_manager::journal::run_paths(&root.join(".planning"), RUN_ID)
        .expect("the fixture run id is a plain path component")
}

/// The parsed `run.json` for the fixed run id under `root`.
fn run_record(root: &Path) -> Value {
    let raw = std::fs::read_to_string(run_paths(root).run_json).expect("run.json is readable");
    serde_json::from_str(&raw).expect("run.json parses")
}

/// Every record in the fixed run's journal, in order.
fn journal_records(root: &Path) -> Vec<Value> {
    std::fs::read_to_string(run_paths(root).journal)
        .expect("journal.jsonl is readable")
        .lines()
        .filter(|line| !line.trim().is_empty())
        .map(|line| serde_json::from_str(line).expect("every journal line parses"))
        .collect()
}

fn canonical(path: &Path) -> PathBuf {
    path.canonicalize().expect("canonicalize")
}

#[tokio::test]
async fn a_codex_registered_project_is_driven_through_codex_exec_end_to_end() {
    let root = project_root();
    let logs = log_dir();
    let config = config_for(root.path(), one("runtime", "codex"), Map::new());

    drive(args(SUCCESS, "0", logs.path()), &config)
        .await
        .expect("a codex project replaying the success transcript completes");

    let root_path = root.path().display().to_string();
    assert_eq!(
        logged(logs.path(), "argv"),
        [
            "exec".to_string(),
            "--json".to_string(),
            "-s".to_string(),
            "workspace-write".to_string(),
            "--add-dir".to_string(),
            format!("{root_path}/.git"),
            "-C".to_string(),
            root_path.clone(),
            "--".to_string(),
            "$gsd-progress".to_string(),
        ],
        "the exact documented argv, with the command translated only here"
    );
    assert_eq!(
        logged(logs.path(), "stdin"),
        ["/dev/null"],
        "an inherited stdin hangs codex exec; the child must get /dev/null"
    );
    assert!(canonical(root.path()).is_dir());

    let record = run_record(root.path());
    assert_eq!(
        record["gsd_command"], "/gsd-progress",
        "the record keeps the canonical command; translation is executor-only"
    );
    let outcome = record["outcome"].as_str().expect("outcome is a string");
    assert!(
        outcome == "succeeded_no_changes" || outcome == "succeeded_with_changes",
        "a clean codex turn is a success, got {outcome}"
    );

    let records = journal_records(root.path());
    let started = records
        .iter()
        .find(|r| r["kind"] == "exec_started")
        .expect("the thread start is journaled as exec_started");
    assert_eq!(started["session_id"], SUCCESS_THREAD);
    assert!(
        records.iter().any(|r| r["kind"] == "exec_event"
            && r["stream"] == "codex:completed:agent_message"
            && r["text"] == "DONE"),
        "the agent's final message is journaled under its codex label"
    );
    assert!(
        records
            .iter()
            .any(|r| r["kind"] == "exec_event" && r["stream"] == "turn_completed"),
        "the synthesized turn boundary is journaled like a Claude one"
    );
}
