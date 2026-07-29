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
use std::path::Path;

use gsd_meta_manager::executor::claude::{interrupt_stopped_a_turn, ClaudeExecutor};
use gsd_meta_manager::executor::stream_json::{parse_line, Envelope, StreamMessage, UserMessage};
use gsd_meta_manager::executor::{
    DrivableProject, ExecutionEvent, ExecutionOptions, Executor, InterruptAck, RunOutcome,
    TurnOutcome,
};
use tempfile::TempDir;

const FAKE_CLAUDE: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/fake-claude.sh");
const FAKE_CLAUDE_ECHO: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/fake-claude-echo.sh"
);
const CLEAN_BASELINE: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/transcripts/01-success-textonly.ndjson"
);
/// The real abort: an interrupt that landed on a streaming turn.
const INTERRUPT_ABORTED: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/transcripts/06-interrupt-aborted-streaming.ndjson"
));
/// The race case: an accepted request that cancelled nothing meaningful.
const INTERRUPT_EARLY: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/transcripts/07-interrupt-early.ndjson"
));

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

/// Build an executor pointed at the **stdin-reactive** stand-in.
///
/// It advertises the three observed 2.1.220 capabilities, answers user
/// messages with a replay echo and control requests with a correlated
/// response, and appends every line the driver writes to `stdin_log`.
fn reacting(stdin_log: &Path) -> ClaudeExecutor {
    ClaudeExecutor::with_program(
        FAKE_CLAUDE_ECHO,
        vec![
            OsString::from("interrupt_receipt_v1,interrupt_cancel_queued_v1,msg_lifecycle_v1"),
            OsString::from("2.1.220"),
            OsString::from("none"),
            stdin_log.to_path_buf().into_os_string(),
        ],
    )
}

/// Every line the driver wrote to the child's stdin, in order.
fn stdin_lines(stdin_log: &Path) -> Vec<String> {
    std::fs::read_to_string(stdin_log)
        .expect("the stand-in records the driver's stdin")
        .lines()
        .map(str::to_string)
        .collect()
}

/// Parse a golden transcript into its messages.
fn messages(transcript: &str) -> Vec<StreamMessage> {
    transcript
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| match parse_line(line) {
            Envelope::Parsed { msg, .. } => Some(msg),
            Envelope::Unparseable { .. } => None,
        })
        .collect()
}

/// Everything a caller could possibly know **at the moment the interrupt was
/// acknowledged**: the ack itself, and the turns that had closed before it.
///
/// Splitting the transcript here is the whole point. A driver that reports on
/// the ack sees exactly this much, and it is never enough.
fn at_acknowledgement_time(transcript: &str) -> (InterruptAck, Vec<TurnOutcome>) {
    let mut closed: Vec<TurnOutcome> = Vec::new();
    for msg in messages(transcript) {
        match msg {
            StreamMessage::Result(result) => closed.push(TurnOutcome::from_result(&result)),
            StreamMessage::ControlResponse(response) => {
                return (InterruptAck::from_response(&response), closed)
            }
            _ => {}
        }
    }
    panic!("an interrupt transcript must carry a control_response");
}

/// The turns a golden transcript closed, over the whole stream.
fn turns(transcript: &str) -> Vec<TurnOutcome> {
    messages(transcript)
        .into_iter()
        .filter_map(|msg| match msg {
            StreamMessage::Result(result) => Some(TurnOutcome::from_result(&result)),
            _ => None,
        })
        .collect()
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

// ============================================================================
// The outbound half of the duplex channel (D-22, D-31)
// ============================================================================

#[tokio::test]
async fn send_writes_the_observed_ndjson_wire_shape_to_the_child_stdin() {
    let scratch = TempDir::new().expect("temp dir");
    let stdin_log = scratch.path().join("stdin.log");
    let project = DrivableProject::for_testing("send", scratch.path());
    let executor = reacting(&stdin_log);

    let mut handle = executor
        .start(
            &project,
            "/gsd-progress".to_string(),
            ExecutionOptions::default(),
        )
        .await
        .expect("the stand-in advertises every required capability");

    executor
        .send(&mut handle, UserMessage::text("second message"))
        .await
        .expect("send");
    handle.close_input().await.expect("close stdin");

    while handle.events.recv().await.is_some() {}
    handle.wait_outcome().await;

    let written = stdin_lines(&stdin_log);
    assert_eq!(
        written.len(),
        2,
        "the released prompt and the sent message, and nothing else: {written:?}"
    );
    assert_eq!(
        written[0],
        r#"{"type":"user","message":{"role":"user","content":[{"type":"text","text":"/gsd-progress"}]}}"#,
        "the first user message on the wire drifted from the observed shape"
    );
    assert_eq!(
        written[1],
        r#"{"type":"user","message":{"role":"user","content":[{"type":"text","text":"second message"}]}}"#,
        "send() must write the same shape as the released prompt"
    );
}

#[tokio::test]
async fn a_message_sent_mid_turn_is_not_buffered_by_the_driver() {
    let scratch = TempDir::new().expect("temp dir");
    let stdin_log = scratch.path().join("stdin.log");
    let project = DrivableProject::for_testing("mid-turn", scratch.path());
    let executor = reacting(&stdin_log);

    let mut handle = executor
        .start(
            &project,
            "/gsd-progress".to_string(),
            ExecutionOptions::default(),
        )
        .await
        .expect("start");

    // The first turn is in flight — the stand-in is holding the released
    // prompt and has not echoed it yet. Write the second message anyway: the
    // CLI queues it and runs it as its own turn, so the driver keeps no
    // turn-boundary flush buffer of its own (D-31).
    executor
        .send(&mut handle, UserMessage::text("mid-turn"))
        .await
        .expect("send");
    handle.close_input().await.expect("close stdin");

    let mut echo_ids: Vec<String> = Vec::new();
    let mut echoes_before_any_terminal_envelope = 0usize;
    let mut saw_terminal_envelope = false;

    while let Some(event) = handle.events.recv().await {
        match event {
            ExecutionEvent::Message(msg) => {
                if let StreamMessage::User(turn) = *msg {
                    if turn.is_replay {
                        echo_ids.push(turn.uuid.clone().unwrap_or_default());
                        if !saw_terminal_envelope {
                            echoes_before_any_terminal_envelope += 1;
                        }
                    }
                }
            }
            ExecutionEvent::TurnCompleted(_) => saw_terminal_envelope = true,
            _ => {}
        }
    }
    handle.wait_outcome().await;

    assert_eq!(
        echo_ids.len(),
        2,
        "each message gets its own replay echo: {echo_ids:?}"
    );
    assert_ne!(
        echo_ids[0], echo_ids[1],
        "the echoes are distinct events, not one coalesced acknowledgement"
    );
    assert_eq!(
        echoes_before_any_terminal_envelope, 2,
        "both messages reached the child before any terminal envelope, so the \
         driver held nothing back to a turn boundary (D-31)"
    );
    assert_eq!(
        stdin_lines(&stdin_log).len(),
        2,
        "both messages were written straight through to stdin"
    );
}

#[tokio::test]
async fn interrupt_correlates_on_request_id_and_reports_what_remains_queued() {
    let scratch = TempDir::new().expect("temp dir");
    let stdin_log = scratch.path().join("stdin.log");
    let project = DrivableProject::for_testing("interrupt", scratch.path());
    let executor = reacting(&stdin_log);

    let mut handle = executor
        .start(
            &project,
            "/gsd-progress".to_string(),
            ExecutionOptions::default(),
        )
        .await
        .expect("start");

    let ack = executor
        .interrupt(&mut handle)
        .await
        .expect("the stand-in answers the control request");

    assert_eq!(
        ack.request_id, "req_1",
        "the ack must correlate to the id we wrote, not to whatever arrived next"
    );
    assert_eq!(
        ack.subtype, "success",
        "success acknowledges the REQUEST; it is not a cancellation claim"
    );
    assert_eq!(
        ack.still_queued.len(),
        1,
        "still_queued is read from the doubly nested response field: {:?}",
        ack.still_queued
    );

    handle.close_input().await.expect("close stdin");
    while handle.events.recv().await.is_some() {}
    handle.wait_outcome().await;

    let written = stdin_lines(&stdin_log);
    assert!(
        written.iter().any(|line| line
            == r#"{"type":"control_request","request_id":"req_1","request":{"subtype":"interrupt"}}"#),
        "the request-and-response form must be written verbatim; the single-field \
         form was empirically refuted on 2.1.220: {written:?}"
    );
}

#[tokio::test]
async fn an_accepted_interrupt_is_never_a_cancellation_at_acknowledgement_time() {
    // Golden transcript 07 is the race case: the acceptance arrives on the wire
    // *before the target turn has even been dequeued* — the replay echo follows
    // it. Golden transcript 06 is the interrupt that landed on a genuinely
    // streaming turn. Both produce the SAME acknowledgement, which is exactly
    // why the acknowledgement cannot be the thing a user is told about.
    for (name, transcript) in [
        ("07-interrupt-early", INTERRUPT_EARLY),
        ("06-interrupt-aborted-streaming", INTERRUPT_ABORTED),
    ] {
        let (ack, closed_so_far) = at_acknowledgement_time(transcript);

        assert_eq!(
            ack.subtype, "success",
            "{name}: an ACCEPTED request is precisely what makes this a trap"
        );
        assert!(
            ack.still_queued.is_empty(),
            "{name}: nothing was queued behind the interrupt, which is not evidence \
             that anything stopped: {:?}",
            ack.still_queued
        );
        assert!(
            !interrupt_stopped_a_turn(&closed_so_far),
            "{name}: at acknowledgement time no turn had ended, so an accepted \
             request must never be presented as a cancellation (D-31, Pitfall D, T-15-18)"
        );
    }

    // The confirmation exists — it simply arrives later, from the stream, as a
    // turn closing with an aborted-streaming terminal reason.
    for (name, transcript) in [
        ("07-interrupt-early", INTERRUPT_EARLY),
        ("06-interrupt-aborted-streaming", INTERRUPT_ABORTED),
    ] {
        assert!(
            interrupt_stopped_a_turn(&turns(transcript)),
            "{name}: the terminal envelope is the actual confirmation the turn stopped"
        );
    }
}
