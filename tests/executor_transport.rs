// ============================================================================
// End-to-end transport tracer
//
// One GSD command travelling the whole path: argv build, process-group spawn,
// first system/init gate, first user message released to stdin, NDJSON framed
// and parsed off the render thread, per-turn result collected, process exit,
// RunOutcome derived.
//
// Unix-only by construction: process-group spawn and group signalling are the
// entire reason `process-wrap` is a dependency.
// ============================================================================

#![cfg(unix)]

use std::ffi::OsString;

use gsd_meta_manager::executor::claude::ClaudeExecutor;
use gsd_meta_manager::executor::{
    DrivableProject, ExecutionEvent, ExecutionOptions, Executor, RunOutcome,
};
use tempfile::TempDir;

const FAKE_CLAUDE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/fake-claude.sh");
const CLEAN_BASELINE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/transcripts/01-success-textonly.ndjson"
);

/// Build an executor pointed at the stand-in, replaying `transcript` and
/// exiting with `exit_code`.
fn replaying(transcript: &str, exit_code: i32) -> ClaudeExecutor {
    ClaudeExecutor::with_program(
        FAKE_CLAUDE,
        vec![
            OsString::from(transcript),
            OsString::from(exit_code.to_string()),
        ],
    )
}

#[tokio::test]
async fn tracer_runs_one_command_end_to_end() {
    let scratch = TempDir::new().expect("temp dir");
    std::fs::create_dir_all(scratch.path().join(".planning")).expect("scratch .planning");

    let project = DrivableProject::for_testing("tracer", scratch.path());
    let executor = replaying(CLEAN_BASELINE, 0);

    let mut handle = executor
        .start(
            &project,
            "/gsd-progress".to_string(),
            ExecutionOptions::default(),
        )
        .await
        .expect("the clean baseline advertises every required capability, so start must succeed");

    // The gate has passed and the prompt has been released. One command means
    // one message, so signal end-of-input: EOF is "no more input", not "stop".
    handle.close_input().await.expect("close stdin");

    let mut session_started: Option<(String, Vec<String>, String)> = None;
    let mut turns_completed = 0usize;
    let mut exit_code: Option<i32> = None;
    let mut unparseable = 0usize;

    while let Some(event) = handle.events.recv().await {
        match event {
            ExecutionEvent::SessionStarted {
                session_id,
                capabilities,
                claude_code_version,
                api_key_source,
                ..
            } => {
                assert_eq!(
                    api_key_source.as_deref(),
                    Some("none"),
                    "the subscription/OAuth path must be alive, got: {api_key_source:?}"
                );
                session_started = Some((session_id, capabilities, claude_code_version));
            }
            ExecutionEvent::TurnCompleted(result) => {
                assert_eq!(
                    result.subtype, "success",
                    "the clean baseline closes with a success turn, got: {}",
                    result.subtype
                );
                turns_completed += 1;
            }
            ExecutionEvent::Exited(status) => exit_code = status.code(),
            ExecutionEvent::Unparseable { raw, error } => {
                unparseable += 1;
                eprintln!("unparseable line: {error} :: {raw}");
            }
            _ => {}
        }
    }

    let (session_id, capabilities, version) =
        session_started.expect("no SessionStarted event was ever emitted");
    assert_eq!(
        capabilities.len(),
        3,
        "expected the three observed 2.1.220 capabilities, got: {capabilities:?}"
    );
    assert!(
        !session_id.is_empty(),
        "SessionStarted must carry the session id"
    );
    assert_eq!(
        version, "2.1.220",
        "the observed CLI version must be recorded, got: {version}"
    );
    assert_eq!(
        unparseable, 0,
        "every line of the clean baseline must parse, {unparseable} did not"
    );
    assert_eq!(
        turns_completed, 1,
        "the clean baseline is a single turn, got {turns_completed}"
    );
    assert_eq!(
        exit_code,
        Some(0),
        "expected a clean exit, got: {exit_code:?}"
    );

    let outcome = handle.wait_outcome().await;
    assert!(
        matches!(
            outcome,
            RunOutcome::SucceededNoChanges { .. } | RunOutcome::SucceededWithChanges { .. }
        ),
        "expected a succeeded outcome, got: {outcome:?}"
    );
    assert!(
        !handle.is_running(),
        "the run must be marked finished once the process has exited"
    );
}

#[tokio::test]
async fn starting_against_a_missing_project_root_never_spawns() {
    let project = DrivableProject::for_testing("gone", "/nonexistent/gsd-meta-manager/tracer");
    let executor = replaying(CLEAN_BASELINE, 0);

    let result = executor
        .start(&project, "/gsd-progress".to_string(), ExecutionOptions::default())
        .await;

    assert!(
        result.is_err(),
        "a project root that is not a directory must be refused before spawn"
    );
}
