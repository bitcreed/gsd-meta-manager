// ============================================================================
// A human-authored message on disk reaches a live agent's stdin (STEER-01,
// STEER-03).
//
// This file proves the phase's spine end to end: a line appended and **fsynced**
// to `.planning/meta-manager/runs/<run-id>/inbox.jsonl`, with no reader process
// yet in existence, is later read by the running driver, written to the agent's
// stdin, and recorded in `journal.jsonl` as an `interjected` record carrying the
// same client-generated id. Every one of those steps is in a different module,
// and a break anywhere renders a perfect UI over nothing.
//
// It is an integration test rather than an in-source one because it needs a
// **real child process**: the property under test is that a `write(2)` which has
// returned is visible to a process that did not exist when it was made, and a
// fake stdin would be testing the fake. The driver itself runs in-process — the
// run body is an ordinary `async fn` and calling it directly is what lets the
// test assert against the run's own return value rather than an exit status it
// would have to interpret.
//
// **The criterion is durability, so the proof is an ORDERING assertion and never
// a sleep.** The message must be on disk before any reader exists; a test that
// slept and then observed delivery would pass just as well against a design that
// buffered the write in the writer's memory, which is the design STEER-03 exists
// to reject.
//
// **What this file deliberately does NOT attempt.** It does not assert the
// `acted-on` transition: `--replay-user-messages` is emitted at *dequeue*, was
// measured 55 seconds after the write on the real CLI, and correlating it is
// plan 18-02's. It does not drive the TUI, which has no injection surface until
// plan 18-07. There is no `#[ignore]`d aspirational test for either — an ignored
// test for work another plan owns is a promise, not a plan.
//
// Unix-only by construction: driving is a Unix capability (D-05).
// ============================================================================

#![cfg(unix)]

use std::ffi::OsString;
use std::path::{Path, PathBuf};

use gsd_meta_manager::config::{Config, DriverOptIn, RegisteredProject};
use gsd_meta_manager::driver::{drive, DriveArgs};
use gsd_meta_manager::journal::inbox::{self, InboxMessage};
use gsd_meta_manager::journal::reader::{self, JournalRecord};
use gsd_meta_manager::journal::writer;
use tempfile::TempDir;

/// The multi-turn stand-in. `fake-claude-echo.sh` emits its only `result` after
/// stdin EOF, which cannot close a D-11 loop: the driver waits for a turn
/// boundary before closing stdin, and that stand-in waits for the close before
/// producing one. This one emits a `result` per turn, as the real CLI does.
const FAKE_TURNS: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/fake-claude-turns.sh"
);
/// The three capabilities observed on 2.1.220.
const CAPABILITIES: &str = "interrupt_receipt_v1,interrupt_cancel_queued_v1,msg_lifecycle_v1";
const VERSION: &str = "2.1.220";
const API_KEY_SOURCE: &str = "none";

const ALIAS: &str = "steered";
const GOAL: &str = "prove a queued message reaches the agent";
const COMMAND: &str = "/gsd-progress";
const RUN_ID: &str = "2026-07-29T21-40-02Z-3f2a";

/// The injected text, **carrying multi-byte scalars on purpose** (STEER-01's
/// encoding case). The text is a Rust `String` from the inbox line to
/// `UserMessage::text`, so this asserts that nothing on the path reaches for a
/// byte slice — a byte-truncating implementation would either mangle these or
/// panic on a scalar boundary.
const INJECTED: &str = "skip the UI review \u{1F680} \u{4E2D}\u{6587}";

/// A project root with a `.planning/` directory, plus the registry naming it.
struct Fixture {
    root: TempDir,
    config: Config,
}

impl Fixture {
    fn new() -> Self {
        let root = TempDir::new().expect("temp dir");
        std::fs::create_dir_all(root.path().join(".planning")).expect("scratch .planning");

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

        Fixture { root, config }
    }

    fn root(&self) -> &Path {
        self.root.path()
    }

    fn planning(&self) -> PathBuf {
        self.root.path().join(".planning")
    }
}

/// The `drive` arguments, plus the two hidden development flags a test may use
/// and the TUI may not.
fn drive_args(stdin_log: &Path) -> DriveArgs {
    DriveArgs {
        alias: ALIAS.to_string(),
        command: COMMAND.to_string(),
        run_id: Some(RUN_ID.to_string()),
        dry_run: false,
        goal: Some(GOAL.to_string()),
        claude_program: Some(PathBuf::from(FAKE_TURNS)),
        claude_args: vec![
            OsString::from(CAPABILITIES),
            OsString::from(VERSION),
            OsString::from(API_KEY_SOURCE),
            stdin_log.to_path_buf().into_os_string(),
        ],
    }
}

/// Every line the driver wrote to the child's stdin, in order.
fn stdin_lines(stdin_log: &Path) -> Vec<String> {
    std::fs::read_to_string(stdin_log)
        .expect("the stand-in records the driver's stdin")
        .lines()
        .map(str::to_string)
        .collect()
}

/// The exact line `Executor::send` must produce for `text`.
///
/// The envelope is written out literally so a change to its shape fails here;
/// only the *string escaping* is delegated to `serde_json`, because hand-escaping
/// a multi-byte literal in a test is how a test starts asserting its own bug.
fn send_wire_shape(text: &str) -> String {
    format!(
        r#"{{"type":"user","message":{{"role":"user","content":[{{"type":"text","text":{}}}]}}}}"#,
        serde_json::to_string(text).expect("the text encodes as a JSON string")
    )
}

/// The run's journal, read back through the reader's whole-file API.
fn journal_records(journal: &Path) -> Vec<JournalRecord> {
    let (records, diagnostics) = reader::read_all(journal).expect("read the journal back");
    assert_eq!(
        diagnostics.unparseable, 0,
        "every line the driver wrote must parse"
    );
    assert_eq!(
        diagnostics.gaps,
        Vec::<(u64, u64)>::new(),
        "a journal written in one process has no sequence gaps"
    );
    records
}

fn of_kind<'a>(records: &'a [JournalRecord], kind: &str) -> Vec<&'a JournalRecord> {
    records.iter().filter(|record| record.kind == kind).collect()
}

/// The load-bearing test of the whole phase.
///
/// Multi-threaded on purpose: the run body races a terminate signal against
/// blocking work on `spawn_blocking`, and `tests/driver_lock.rs:201-215` records
/// what a current-thread runtime does with that combination — it deadlocks
/// instead of failing, which is strictly worse than a red test.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_message_written_and_fsynced_before_any_reader_exists_is_delivered_and_journaled() {
    let fixture = Fixture::new();
    let paths = writer::create_run_dir(&fixture.planning(), RUN_ID).expect("create the run dir");

    // --- The durability boundary, and the ordering that proves it ---
    let message = InboxMessage::new(INJECTED);
    let id = message.id.clone();
    inbox::append(&paths.inbox, &message).expect("the inbox append must succeed and fsync");

    assert!(
        paths.inbox.metadata().expect("stat the inbox").len() > 0,
        "the message must be bytes on disk before anything is asked to read it"
    );
    assert!(
        !paths.journal.exists(),
        "no reader exists yet: a journal here would mean a driver had already run, \
         and the criterion under test is delivery to a process that did not exist \
         when the write was made"
    );

    let stdin_log = fixture.root().join("stdin.log");
    drive(drive_args(&stdin_log), &fixture.config)
        .await
        .expect("a steered run must complete rather than error");

    // --- It reached the agent's stdin, in the pinned wire shape ---
    let written = stdin_lines(&stdin_log);
    assert!(
        written.contains(&send_wire_shape(INJECTED)),
        "the injected message never reached the agent's stdin in the shape \
         `Executor::send` promises; got {written:?}"
    );
    assert!(
        written.len() >= 2,
        "the run must write the command prompt AND the injected message, so a \
         driver that closed stdin at spawn cannot pass: got {written:?}"
    );

    // --- It is evidence on disk, correlated by the client-generated id ---
    let records = journal_records(&paths.journal);
    let interjected = of_kind(&records, "interjected");
    assert_eq!(
        interjected.len(),
        1,
        "exactly one delivery record, not at least one — a message delivered twice \
         is a turn the user did not ask for"
    );
    assert_eq!(
        interjected[0].rest["id"], id,
        "the record must carry the id the writing side minted, because text alone \
         is not a correlation key (D-05)"
    );
    assert_eq!(
        interjected[0].rest["text"], INJECTED,
        "a multi-byte message must survive the whole path byte for byte"
    );
    assert_eq!(
        interjected[0].rest["delivered"], true,
        "`delivered` is the answer to \"did the stdin write return Ok\" and nothing \
         weaker"
    );

    // --- And the steered run still ends naturally (D-11) ---
    let finished = of_kind(&records, "exec_finished");
    assert_eq!(finished.len(), 1, "one agent process, one ending");
    assert_eq!(
        finished[0].rest["exit"], 0,
        "closing stdin is \"no more input\", not \"stop\": the agent must drain its \
         queued turns and exit cleanly"
    );
    assert!(
        of_kind(&records, "run_ended")
            .last()
            .is_some_and(|record| record.rest["outcome"] != "killed"),
        "a steered run that ended on its own must not be recorded as killed"
    );
    assert!(
        of_kind(&records, "interjection_missed").is_empty(),
        "a message delivered before the close must never also be recorded as missed"
    );
}

/// The other half of D-11: a run nobody steers must still terminate.
///
/// This is the regression that a naive "never close stdin" would pass every
/// injection assertion and still hang forever. It asserts the *absence* of an
/// injection record for the same reason: a driver that journalled a delivery it
/// never made would satisfy the test above while doing nothing.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_run_with_an_empty_inbox_closes_stdin_at_the_first_turn_boundary_and_exits_zero() {
    let fixture = Fixture::new();
    let paths = writer::create_run_dir(&fixture.planning(), RUN_ID).expect("create the run dir");
    assert!(
        !paths.inbox.exists(),
        "the inbox file does not exist until something is queued"
    );

    let stdin_log = fixture.root().join("stdin.log");
    drive(drive_args(&stdin_log), &fixture.config)
        .await
        .expect("an unsteered run must complete rather than error");

    let written = stdin_lines(&stdin_log);
    assert_eq!(
        written.len(),
        1,
        "an unsteered run writes exactly the command prompt: got {written:?}"
    );

    let records = journal_records(&paths.journal);
    assert!(
        of_kind(&records, "interjected").is_empty(),
        "nothing was queued, so nothing may be recorded as delivered"
    );
    assert_eq!(
        of_kind(&records, "exec_finished")[0].rest["exit"],
        0,
        "the agent must exit 0 after end-of-input"
    );
}
