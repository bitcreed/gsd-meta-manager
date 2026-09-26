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
use std::path::Path;

use gsd_meta_manager::config::{Config, DriverOptIn, RegisteredProject};
use gsd_meta_manager::driver::{drive, DriveArgs};
use serde_json::{Map, Value};
use tempfile::TempDir;

const FAKE_CODEX: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/fake-codex.sh");
const SUCCESS: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/codex/01-exec-success.jsonl"
);
const FAILURE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/codex/02-exec-failure.jsonl"
);
/// The diagnostic code every Codex run journals before its first exec record.
const RUNTIME_ENVELOPE_PARTIAL: &str = "runtime_envelope_partial";

const ALIAS: &str = "codexed";
/// A fixed run id so the test knows where to read, rather than globbing.
const RUN_ID: &str = "2026-09-22T12-00-00Z-c0de";
/// The thread id on the first line of the success transcript.
const SUCCESS_THREAD: &str = "01a0ca2f-8a0c-72c0-87fb-11c28739d140";

fn nonblank(raw: &str) -> gsd_meta_manager::driver::payload::NonBlank {
    gsd_meta_manager::driver::payload::NonBlank::new(raw)
        .expect("a visible test literal is a payload")
}

/// The agent-family variables this binary plants in its own environment, so
/// the env-hardening test has something to see scrubbed. `CODEX_HOME` is the
/// one that must survive. `GSD_RUNTIME` is planted as `claude` — the value a
/// user who exported it globally would leak into a Codex child.
const PLANTED_ENV: [(&str, &str); 5] = [
    ("CODEX_THREAD_ID", "01a0ca2f-0000-7000-8000-00000000beef"),
    ("CODEX_SANDBOX_NETWORK_DISABLED", "1"),
    ("CODEX_HOME", "/nonexistent/codex-home"),
    ("CLAUDECODE", "1"),
    ("GSD_RUNTIME", "claude"),
];

/// Point the envelope at a temp root for this test binary and plant
/// [`PLANTED_ENV`] — both exactly once, inside one `OnceLock`, so every
/// `set_var` in this binary happens before any test drives a run (the
/// isolation `tests/driver_tracer.rs` documents).
fn isolate_envelope_root() {
    static ROOT: std::sync::OnceLock<TempDir> = std::sync::OnceLock::new();
    ROOT.get_or_init(|| {
        let dir = TempDir::new().expect("an envelope temp root");
        std::env::set_var(gsd_meta_manager::envelope::ENVELOPE_ROOT_ENV, dir.path());
        for (key, value) in PLANTED_ENV {
            std::env::set_var(key, value);
        }
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

/// The index of the first journal record matching `pred`.
fn position(records: &[Value], pred: impl Fn(&Value) -> bool) -> Option<usize> {
    records.iter().position(pred)
}

fn is_disclosure(record: &Value) -> bool {
    record["kind"] == "diagnostic" && record["code"] == RUNTIME_ENVELOPE_PARTIAL
}

#[tokio::test]
async fn every_codex_run_discloses_the_partial_envelope_before_its_first_exec_record() {
    let root = project_root();
    let logs = log_dir();
    let config = config_for(root.path(), one("runtime", "codex"), Map::new());
    drive(args(SUCCESS, "0", logs.path()), &config)
        .await
        .expect("the run completes");

    let records = journal_records(root.path());
    let disclosure = position(&records, is_disclosure)
        .expect("a codex run journals the runtime_envelope_partial diagnostic");
    let started = position(&records, |r| r["kind"] == "exec_started").expect("exec_started");
    assert!(
        disclosure < started,
        "the disclosure must precede the first exec record, so nothing reads as a \
         fully-enveloped run first ({disclosure} vs {started})"
    );
    let detail = records[disclosure]["detail"].as_str().expect("detail");
    for fact in ["codex exec", "workspace-write", "PreToolUse", "deny list"] {
        assert!(
            detail.contains(fact),
            "the disclosure names {fact:?}: {detail}"
        );
    }
}

#[tokio::test]
async fn a_project_with_no_runtime_key_spawns_the_unchanged_claude_argv() {
    let root = project_root();
    // GSD's OWN runtime key says codex. The manager must ignore it (ID-2): the
    // assertion below goes red the moment it silently becomes the default.
    std::fs::write(
        root.path().join(".planning").join("config.json"),
        br#"{"runtime": "codex"}"#,
    )
    .expect("write GSD config");
    let logs = log_dir();
    let config = config_for(root.path(), Map::new(), Map::new());

    // Not asserted: the stand-in replays codex JSONL, which carries no Claude
    // `system/init`, so the Claude run ends as a startup failure. Only what
    // was SPAWNED is under test here.
    let _ = drive(args(SUCCESS, "0", logs.path()), &config).await;

    let argv = logged(logs.path(), "argv");
    assert_eq!(
        argv[..8],
        [
            "-p",
            "--input-format",
            "stream-json",
            "--output-format",
            "stream-json",
            "--verbose",
            "--replay-user-messages",
            "--session-id",
        ],
        "{argv:?}"
    );
    let session = uuid::Uuid::parse_str(&argv[8]).expect("position 8 is the session UUID");
    assert_eq!(session.get_version_num(), 4, "a v4 session id");
    assert_eq!(
        argv[9..14],
        [
            "--setting-sources",
            "project",
            "--permission-mode",
            "dontAsk",
            "--strict-mcp-config",
        ],
        "{argv:?}"
    );
    assert_eq!(argv.len(), 18, "nothing beyond the envelope pair: {argv:?}");
    assert_eq!(argv[14], "--disallowedTools");
    assert!(!argv[15].is_empty(), "the deny list is non-empty");
    assert_eq!(argv[16], "--settings");
    assert!(!argv[17].is_empty(), "the settings path is named");

    if let Ok(raw) = std::fs::read_to_string(run_paths(root.path()).journal) {
        assert!(
            !raw.contains(RUNTIME_ENVELOPE_PARTIAL),
            "a Claude run carries no runtime disclosure; its journal is unchanged"
        );
    }
}

#[tokio::test]
async fn the_default_runtime_preference_applies_only_when_the_entry_names_none() {
    // Preference alone: Codex.
    let root = project_root();
    let logs = log_dir();
    let config = config_for(root.path(), Map::new(), one("default_runtime", "codex"));
    drive(args(SUCCESS, "0", logs.path()), &config)
        .await
        .expect("a codex run by preference completes");
    assert_eq!(logged(logs.path(), "argv")[0], "exec");

    // Entry `claude` outranks a `codex` preference.
    let root = project_root();
    let logs = log_dir();
    let config = config_for(
        root.path(),
        one("runtime", "claude"),
        one("default_runtime", "codex"),
    );
    let _ = drive(args(SUCCESS, "0", logs.path()), &config).await;
    assert_eq!(logged(logs.path(), "argv")[0], "-p");
}

#[tokio::test]
async fn an_unrecognized_runtime_is_refused_before_anything_is_created() {
    for dry_run in [false, true] {
        let root = project_root();
        let logs = log_dir();
        let config = config_for(root.path(), one("runtime", "gemini"), Map::new());
        let mut drive_args = args(SUCCESS, "0", logs.path());
        drive_args.dry_run = dry_run;

        let err = drive(drive_args, &config)
            .await
            .expect_err("an unknown runtime is refused");
        assert!(
            matches!(
                err,
                gsd_meta_manager::error::DriveError::RuntimeUnrecognized(_)
            ),
            "dry_run={dry_run}: got {err:?}"
        );
        assert!(
            err.to_string().contains("gemini"),
            "the refusal names the value: {err}"
        );
        assert!(
            !logs.path().join("argv").exists(),
            "dry_run={dry_run}: nothing was spawned"
        );
        assert!(
            !root.path().join(".planning").join("meta-manager").exists(),
            "dry_run={dry_run}: no run directory, no journal"
        );
    }
}

#[tokio::test]
async fn a_failed_codex_turn_fails_the_run_and_journals_the_error() {
    let root = project_root();
    let logs = log_dir();
    let config = config_for(root.path(), one("runtime", "codex"), Map::new());
    let _ = drive(args(FAILURE, "1", logs.path()), &config).await;

    let record = run_record(root.path());
    assert_eq!(record["outcome"], "failed", "{record}");
    let records = journal_records(root.path());
    assert!(
        records
            .iter()
            .any(|r| r["kind"] == "exec_event" && r["stream"] == "codex:error"),
        "the top-level error line is journaled"
    );
}

#[tokio::test]
async fn a_goal_only_drive_of_a_codex_project_is_refused_without_spawning() {
    let root = project_root();
    let logs = log_dir();
    let config = config_for(root.path(), one("runtime", "codex"), Map::new());
    let mut drive_args = args(SUCCESS, "0", logs.path());
    drive_args.command = None;
    drive_args.goal = Some(nonblank("get phase 3 to verified"));

    let err = drive(drive_args, &config)
        .await
        .expect_err("the model seam is not available under codex");
    assert!(
        matches!(
            err,
            gsd_meta_manager::error::DriveError::GoalSeamUnusable { .. }
        ),
        "got {err:?}"
    );
    assert!(
        !logs.path().join("argv").exists(),
        "zero spawns: the refusal happened before any process existed, and \
         nothing silently fell back to Claude"
    );
}

#[tokio::test]
async fn the_codex_child_inherits_codex_home_and_no_other_agent_variable() {
    let root = project_root();
    let logs = log_dir();
    let config = config_for(root.path(), one("runtime", "codex"), Map::new());
    drive(args(SUCCESS, "0", logs.path()), &config)
        .await
        .expect("the run completes");

    let env = logged(logs.path(), "env");
    assert!(env.iter().any(|name| name == "CODEX_HOME"), "{env:?}");
    for scrubbed in [
        "CODEX_THREAD_ID",
        "CODEX_SANDBOX_NETWORK_DISABLED",
        "CLAUDECODE",
    ] {
        assert!(
            !env.iter().any(|name| name == scrubbed),
            "{scrubbed} leaked: {env:?}"
        );
    }
    assert!(
        !env.iter().any(|name| name.starts_with("CLAUDE")),
        "no CLAUDE* variable reaches a codex child: {env:?}"
    );
    // Positive control: the fixture does log GSD_* names, so the absence of
    // GSD_RUNTIME below is a scrub and not a blind spot.
    assert!(
        env.iter()
            .any(|name| name == gsd_meta_manager::envelope::ENVELOPE_ROOT_ENV),
        "GSD_* names are logged and the envelope root is inherited: {env:?}"
    );
    assert!(
        !env.iter().any(|name| name == "GSD_RUNTIME"),
        "an inherited GSD_RUNTIME must not reach a codex child — it would outrank \
         the child GSD's own config.runtime and install marker: {env:?}"
    );
}
