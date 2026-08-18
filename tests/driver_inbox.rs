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
// **What this file deliberately does NOT attempt.** It does not drive the TUI,
// which has no injection surface until plan 18-07. There is no `#[ignore]`d
// aspirational test for it — an ignored test for work another plan owns is a
// promise, not a plan.
//
// It DOES now assert the `acted-on` transition and the `missed` one, which plan
// 18-02 added. Those two turn the four states from labels into observations:
// `acted-on` comes from the agent's own `--replay-user-messages` echo, correlated
// by exact text (D-07, D-08), and `missed` is the honest fourth state for a
// message that arrived after stdin closed and can therefore never be delivered
// (D-10).
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

/// A one-entry registry pointing at `root`, opted in.
///
/// Built fresh per caller rather than shared, following `tests/driver_lock.rs`:
/// `Config` is not `Clone`, and a test that drives the run on its own task needs
/// an owned value inside that task.
fn config_for(root: &Path) -> Config {
    isolate_envelope_root();
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

impl Fixture {
    fn new() -> Self {
        let root = TempDir::new().expect("temp dir");
        std::fs::create_dir_all(root.path().join(".planning")).expect("scratch .planning");
        let config = config_for(root.path());

        Fixture { root, config }
    }

    fn root(&self) -> &Path {
        self.root.path()
    }

    fn planning(&self) -> PathBuf {
        self.root.path().join(".planning")
    }
}

/// The paced stand-in. Same protocol as [`FAKE_TURNS`], with the first turn's
/// `result` held back and a linger after stdin EOF — the two windows a test
/// needs in order to stand on the close boundary rather than race it. See the
/// fixture's own header.
const FAKE_PACED: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/fake-claude-paced.sh"
);

/// The `drive` arguments, plus the two hidden development flags a test may use
/// and the TUI may not.
fn drive_args(stdin_log: &Path) -> DriveArgs {
    drive_args_with(FAKE_TURNS, stdin_log)
}

/// The same, against a named stand-in.
fn drive_args_with(program: &str, stdin_log: &Path) -> DriveArgs {
    DriveArgs {
        alias: ALIAS.to_string(),
        command: COMMAND.to_string(),
        run_id: Some(RUN_ID.to_string()),
        dry_run: false,
        goal: Some(GOAL.to_string()),
        claude_program: Some(PathBuf::from(program)),
        claude_args: vec![
            OsString::from(CAPABILITIES),
            OsString::from(VERSION),
            OsString::from(API_KEY_SOURCE),
            stdin_log.to_path_buf().into_os_string(),
        ],
    }
}

/// Where [`FAKE_PACED`] announces that it has seen stdin EOF.
///
/// Derived from the stdin-log path by the same rule the fixture uses, so the
/// two cannot drift: there is one convention, stated in the fixture's header.
fn eof_marker_of(stdin_log: &Path) -> PathBuf {
    let mut name = stdin_log.as_os_str().to_os_string();
    name.push(".eof");
    PathBuf::from(name)
}

/// Poll until `predicate` holds, or fail with `what` after ten seconds.
///
/// **Polling for an observable fact, never sleeping for a duration.** A fixed
/// sleep proves the same property on an idle machine and flakes on a loaded one,
/// and the difference between the two is invisible in the failure output.
async fn wait_until(what: &str, mut predicate: impl FnMut() -> bool) {
    for _ in 0..400 {
        if predicate() {
            return;
        }
        tokio::time::sleep(std::time::Duration::from_millis(25)).await;
    }
    panic!("timed out after 10s waiting for: {what}");
}

/// Split one message's serialised line into a fragment and its completion.
///
/// The head carries **no trailing newline**, which is the only thing that makes
/// it torn: `tail_lines` frames on `\n` and nothing else, so this is exactly the
/// state a reader observes when it arrives between the writer's `write_all` and
/// the kernel's next scheduling of it.
fn torn_halves(message: &InboxMessage) -> (String, String) {
    let line = serde_json::to_string(message).expect("an inbox message serialises");
    let mut split = line.len() / 2;
    while !line.is_char_boundary(split) {
        split += 1;
    }
    (line[..split].to_string(), format!("{}\n", &line[split..]))
}

/// Append bytes to the inbox verbatim — no framing, no fsync, no message model.
///
/// Deliberately NOT `inbox::append`, which exists to make a torn line
/// impossible. Reaching around it is the only way to produce the state the
/// backstop is about.
fn append_raw(inbox_path: &Path, bytes: &str) {
    use std::io::Write;
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(inbox_path)
        .expect("open the inbox for a raw append");
    file.write_all(bytes.as_bytes())
        .expect("the raw append succeeds");
    file.sync_data().expect("and reaches the disk");
}

/// The ids the driver has journaled as delivered **so far**, read mid-run.
///
/// Tolerant where [`journal_records`] is strict, and the difference is the point
/// of observation: a journal read while its writer is still appending may end in
/// a partial line, which is a fact about the read and not a defect in the file.
/// The strict version is for the post-mortem, where a torn tail would mean
/// something really was wrong.
fn delivered_ids_now(journal: &Path) -> Vec<String> {
    let Ok((records, _)) = reader::read_all(journal) else {
        return Vec::new();
    };
    ids_of_kind(&records, "interjected")
}

/// The ids the agent has **dequeued** so far, read mid-run.
///
/// Tolerant in the same way and for the same reason as [`delivered_ids_now`].
/// This is the transition a test synchronises on when it needs to stand *past* a
/// turn boundary: the echo is emitted at dequeue, so a message's `acted_on`
/// record cannot exist before the boundary that started its turn.
fn acted_on_ids_now(journal: &Path) -> Vec<String> {
    let Ok((records, _)) = reader::read_all(journal) else {
        return Vec::new();
    };
    ids_of_kind(&records, "interjection_acted_on")
}

/// The `interjected` / `interjection_acted_on` / `interjection_missed` ids a
/// journal carries, in the order the driver wrote them.
fn ids_of_kind(records: &[JournalRecord], kind: &str) -> Vec<String> {
    of_kind(records, kind)
        .iter()
        .filter_map(|record| record.rest["id"].as_str().map(str::to_string))
        .collect()
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

// ============================================================================
// The acted-on transition (STEER-02, D-07, D-08)
//
// `isReplay: true` is emitted at DEQUEUE — measured 55 seconds after the write
// on the real CLI, 45ms after the PREVIOUS turn's `result`. That is what makes
// this a third state rather than a second name for `delivered`, and it is why
// the word for it is "acted-on" and never "received".
// ============================================================================

/// The load-bearing test of STEER-02's third state.
///
/// A driver that journalled `interjection_acted_on` at the moment of the stdin
/// write would pass every assertion in the delivery test above and turn three
/// labels into two real states — the named "looks done but isn't" failure. So
/// the assertion is not merely that the record exists but that it arrives
/// **after** the delivery record it acks, which is the only shape the dequeue
/// echo can produce.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_delivered_message_is_journaled_acted_on_when_the_replay_echo_arrives() {
    let fixture = Fixture::new();
    let paths = writer::create_run_dir(&fixture.planning(), RUN_ID).expect("create the run dir");

    let message = InboxMessage::new(INJECTED);
    let id = message.id.clone();
    inbox::append(&paths.inbox, &message).expect("the inbox append must succeed and fsync");

    let stdin_log = fixture.root().join("stdin.log");
    drive(drive_args(&stdin_log), &fixture.config)
        .await
        .expect("a steered run must complete rather than error");

    let records = journal_records(&paths.journal);

    let acted_on = of_kind(&records, "interjection_acted_on");
    assert_eq!(
        acted_on.len(),
        1,
        "one message, one acted-on transition — a second would mean the echo \
         matched twice"
    );
    assert_eq!(
        acted_on[0].rest["id"], id,
        "the transition must name the id the WRITING side minted; the echo \
         carries a uuid of the agent's own, which correlates nothing"
    );

    let interjected = of_kind(&records, "interjected");
    assert_eq!(interjected.len(), 1);
    assert!(
        interjected[0].seq < acted_on[0].seq,
        "acted-on must follow delivery on disk. A driver that recorded the \
         transition at the moment of the stdin write would satisfy every other \
         assertion here while collapsing `delivered` and `acted-on` into one \
         state measured 55 seconds apart (D-07)"
    );
    assert!(
        of_kind(&records, "interjection_missed").is_empty(),
        "a message the agent dequeued was plainly not missed"
    );
}

/// The case the FIFO exists for, end to end.
///
/// Two identical messages are legitimate — a user may say the same thing twice —
/// and the two echoes are byte-identical, so nothing in the echo itself says
/// which message it acks. Delivery order is the only answer, and an
/// implementation that matched the newest pending entry first would ack the
/// second message twice and leave the first showing `delivered` forever.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn two_identical_messages_are_acked_in_delivery_order() {
    const SAME: &str = "continue";

    let fixture = Fixture::new();
    let paths = writer::create_run_dir(&fixture.planning(), RUN_ID).expect("create the run dir");

    let first = InboxMessage::new(SAME);
    let second = InboxMessage::new(SAME);
    assert_ne!(
        first.id, second.id,
        "two messages with the same text still get distinct ids, which is the \
         whole reason the id is the correlation key and the text is not"
    );
    inbox::append(&paths.inbox, &first).expect("append the first");
    inbox::append(&paths.inbox, &second).expect("append the second");

    let stdin_log = fixture.root().join("stdin.log");
    drive(drive_args(&stdin_log), &fixture.config)
        .await
        .expect("a steered run must complete rather than error");

    let records = journal_records(&paths.journal);

    assert_eq!(
        ids_of_kind(&records, "interjected"),
        vec![first.id.clone(), second.id.clone()],
        "both were delivered, in inbox order"
    );
    assert_eq!(
        ids_of_kind(&records, "interjection_acted_on"),
        vec![first.id.clone(), second.id.clone()],
        "and both were acked in DELIVERY order. Matching the newest pending \
         entry first would produce [second, second] here, which reads as \
         'everything is fine' on any per-id display that takes the last record \
         for an id as its state"
    );
}

// ============================================================================
// The honest fourth state (STEER-02, D-10)
//
// Every one of these stands on the close boundary rather than racing it: the
// paced stand-in announces stdin EOF by creating a marker file and then lingers,
// so "after the close" is an observed fact and not an elapsed duration.
// ============================================================================

/// The window the final pre-close drain exists to win, **twice** (D-10, D-11,
/// CR-01).
///
/// A human's last keystroke and the turn boundary are in a genuine race, and the
/// design resolves it in the user's favour: stdin is closed only after a drain
/// that found nothing. This asserts the property from the other side — a message
/// appended while the run is live reaches the agent and is never recorded as
/// missed.
///
/// **It asserts the SECOND message as well, and that half is the regression
/// guard.** Steering is only steering if it can be done more than once. Two arms
/// deliver from the inbox — the 750ms poll and the turn-boundary drain — and
/// because the poll fires every 750ms while a turn lasts minutes, the poll arm
/// wins for essentially every message a real user types. A driver that discards
/// the poll arm's delivery count sees an empty inbox at the next boundary,
/// concludes nothing was delivered, and closes stdin **while the injected turn is
/// queued in the CLI and about to run**. Every message after that is `missed`.
/// This test's earlier form executed precisely that sequence and asserted only
/// the first message, so the defect sat inside its own fixture unseen; the second
/// injection is what makes it visible.
///
/// The second message is appended after `interjection_acted_on` for the first —
/// the agent's own dequeue echo, which cannot arrive before turn one's boundary
/// has been crossed. So "after a boundary" is an observed fact here, never an
/// elapsed duration, and the paced stand-in holds turn two open for the poll to
/// find the message inside.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_message_appended_before_the_final_drain_is_still_delivered() {
    const SECOND: &str = "and now run the tests";

    let fixture = Fixture::new();
    let paths = writer::create_run_dir(&fixture.planning(), RUN_ID).expect("create the run dir");
    let stdin_log = fixture.root().join("stdin.log");
    let eof_marker = eof_marker_of(&stdin_log);

    let root = fixture.root().to_path_buf();
    let log = stdin_log.clone();
    let run = tokio::spawn(async move {
        let config = config_for(&root);
        drive(drive_args_with(FAKE_PACED, &log), &config).await
    });

    // The run is live once the command prompt has reached the agent's stdin,
    // and the paced stand-in holds the first turn open from there.
    wait_until("the command prompt to reach the agent's stdin", || {
        stdin_log.exists() && !stdin_lines(&stdin_log).is_empty()
    })
    .await;
    assert!(
        !eof_marker.exists(),
        "stdin must still be open when the message is appended, or this test is \
         asserting the missed path by accident"
    );

    let first = InboxMessage::new(INJECTED);
    inbox::append(&paths.inbox, &first).expect("append while the run is live");

    // The dequeue echo for the first message. It is emitted as turn two starts,
    // so observing it proves turn ONE's boundary is behind us — which is exactly
    // the point at which a driver that dropped the poll arm's delivery count has
    // already closed stdin.
    wait_until("the agent to dequeue the first injected message", || {
        acted_on_ids_now(&paths.journal).contains(&first.id)
    })
    .await;
    assert!(
        !eof_marker.exists(),
        "a run that can be steered more than once must still have stdin open \
         one turn boundary after its first injection — the agent has just \
         dequeued a human-steered turn, which is the least plausible moment to \
         stop listening"
    );

    let second = InboxMessage::new(SECOND);
    inbox::append(&paths.inbox, &second).expect("append after the first turn boundary");

    run.await
        .expect("the run task did not panic")
        .expect("a steered run must complete rather than error");

    let records = journal_records(&paths.journal);
    assert_eq!(
        ids_of_kind(&records, "interjected"),
        vec![first.id.clone(), second.id.clone()],
        "BOTH messages are delivered, in order. A run that can be steered \
         exactly once produces only the first here and journals the second \
         `missed`"
    );
    assert!(
        of_kind(&records, "interjection_missed").is_empty(),
        "and neither is ALSO reported missed: the two records are mutually \
         exclusive for one id"
    );
    let written = stdin_lines(&stdin_log);
    assert!(
        written.contains(&send_wire_shape(INJECTED)),
        "the first reached the agent's stdin in the pinned wire shape"
    );
    assert!(
        written.contains(&send_wire_shape(SECOND)),
        "and so did the second"
    );
}

/// The state that stops an undeliverable message looking like a slow one (D-10).
///
/// Once stdin is closed it cannot be reopened, so a message arriving afterwards
/// has nowhere to go — **ever**. Leaving it in `queued` is PITFALLS' Pitfall 11
/// dressed up as a spinner: indistinguishable, to the user, from an agent that is
/// merely taking its time. The assertion is therefore two-sided: exactly one
/// `missed` record for the id, and no `interjected` record for it at all.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_message_appended_after_stdin_closed_is_journaled_missed_and_never_retried() {
    let fixture = Fixture::new();
    let paths = writer::create_run_dir(&fixture.planning(), RUN_ID).expect("create the run dir");
    let stdin_log = fixture.root().join("stdin.log");
    let eof_marker = eof_marker_of(&stdin_log);

    let root = fixture.root().to_path_buf();
    let log = stdin_log.clone();
    let run = tokio::spawn(async move {
        let config = config_for(&root);
        drive(drive_args_with(FAKE_PACED, &log), &config).await
    });

    // The marker is the agent observing EOF, which it can only do after the
    // driver's `close_input()`. Nothing appended from here on is deliverable.
    wait_until("the agent to observe stdin EOF", || eof_marker.exists()).await;

    let message = InboxMessage::new("too late to steer this run");
    inbox::append(&paths.inbox, &message).expect("append after the close");

    run.await
        .expect("the run task did not panic")
        .expect("a run whose steering arrived too late still completes normally");

    let records = journal_records(&paths.journal);

    assert_eq!(
        ids_of_kind(&records, "interjection_missed"),
        vec![message.id.clone()],
        "exactly one terminal record, naming WHICH message was lost. A count \
         would satisfy 'nothing is silently abandoned' in prose and leave the \
         display unable to say anything a user can act on"
    );
    assert!(
        !ids_of_kind(&records, "interjected").contains(&message.id),
        "`interjected` and `interjection_missed` are mutually exclusive for one \
         id: the first says the stdin write returned Ok, the second says there \
         was no stdin left to write to"
    );
    assert!(
        !stdin_lines(&stdin_log).contains(&send_wire_shape("too late to steer this run")),
        "and it must NOT have been retried onto a stdin that is closed"
    );
}

/// The held-out durability backstop: a torn final line (D-04, STEER-03).
///
/// The inbox is appended to by one process and tailed by another, so the reader
/// will eventually observe a line mid-write. The only two wrong answers are
/// delivering a fragment — which would hand the agent a truncated instruction —
/// and consuming it, which loses the message with no record anywhere. The right
/// answer is to leave it alone until the newline arrives, and this asserts all
/// three halves: nothing delivered while torn, nothing lost, and delivered
/// exactly once when completed.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_torn_final_line_is_neither_delivered_nor_lost_and_arrives_once_completed() {
    const COMPLETE: &str = "this line was whole from the start";
    const TORN: &str = "this line arrived in two pieces";

    let fixture = Fixture::new();
    let paths = writer::create_run_dir(&fixture.planning(), RUN_ID).expect("create the run dir");
    let stdin_log = fixture.root().join("stdin.log");

    // One whole line, then a fragment with no terminating newline — exactly the
    // state a reader finds when it arrives mid-append.
    let whole = InboxMessage::new(COMPLETE);
    let torn = InboxMessage::new(TORN);
    inbox::append(&paths.inbox, &whole).expect("append the complete message");
    let (head, tail) = torn_halves(&torn);
    append_raw(&paths.inbox, &head);

    let root = fixture.root().to_path_buf();
    let log = stdin_log.clone();
    let run = tokio::spawn(async move {
        let config = config_for(&root);
        drive(drive_args_with(FAKE_PACED, &log), &config).await
    });

    // The driver's own `interjected` record is what says it has read past the
    // whole line and stopped at the fragment. Deliberately NOT the agent's stdin
    // log: the stand-in appends to that when it *consumes* a line, which is one
    // turn later, and waiting on it would put this assertion after the first
    // turn boundary rather than before it.
    wait_until("the driver to deliver the complete message", || {
        delivered_ids_now(&paths.journal).contains(&whole.id)
    })
    .await;
    assert!(
        !delivered_ids_now(&paths.journal).contains(&torn.id),
        "the fragment must NOT have been delivered while torn — a truncated \
         instruction is worse than a late one"
    );

    // Now finish the line the way the writing process would have.
    append_raw(&paths.inbox, &tail);

    run.await
        .expect("the run task did not panic")
        .expect("a steered run must complete rather than error");

    let records = journal_records(&paths.journal);
    assert_eq!(
        ids_of_kind(&records, "interjected"),
        vec![whole.id.clone(), torn.id.clone()],
        "the fragment was neither delivered early nor discarded: once completed \
         it is delivered, exactly once, after the message that preceded it"
    );
    assert!(
        of_kind(&records, "interjection_missed").is_empty(),
        "nothing was lost, so nothing may wear the missed label"
    );
    assert_eq!(
        stdin_lines(&stdin_log)
            .iter()
            .filter(|line| *line == &send_wire_shape(TORN))
            .count(),
        1,
        "exactly once — a re-read that re-delivered it would give the user a \
         turn they did not ask for"
    );
}
