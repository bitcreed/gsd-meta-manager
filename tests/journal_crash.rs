// ============================================================================
// Crash survival and on-disk redaction: a real process, a real signal, and the
// bytes that are left afterwards (OBS-01 criteria 1 and 3, SAFE-04, D-27, D-34)
//
// Why this cannot be an in-source test. The claim under test is "a writer that
// is killed leaves every completed step readable". Satisfying it needs three
// things a `#[cfg(test)] mod tests` cannot supply: a second process, a real
// signal delivered to it, and — the part that is easy to get wrong — a child
// running the REAL `JournalWriter` rather than a stand-in that merely writes
// lines that look like it. A shell fixture can produce plausible NDJSON; it
// cannot exercise the writer's framing, its sequence counter, or the redaction
// seam every line travels through, so a test built on one would prove nothing
// about the code that ships.
//
// The harness is RESEARCH §6.2's, which was built and run eight times in the
// research session (§3.2's table is its output): re-exec this very test binary
// with libtest arguments that select exactly one test, select the child role
// with an environment sentinel, and use the standard library's child kill —
// which IS SIGKILL on unix, confirmed by `unix_wait_status(9)` in every run.
// No new dependency, no hidden subcommand in the release binary.
//
// Unix-only by construction: the signal semantics this file asserts are unix's.
// ============================================================================

#![cfg(unix)]

use std::path::{Path, PathBuf};
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::{Duration, Instant};

use gsd_meta_manager::journal::reader::{parse_line, seq_gaps, JournalRecord, ParsedLine};
use gsd_meta_manager::journal::redact::redact;
use gsd_meta_manager::journal::writer::{
    crash_test_writer_loop, CRASH_TEST_PLANTED_HOME, CRASH_TEST_PLANTED_KEY,
};

/// The sentinel that selects the child role, carrying the journal path.
const CHILD_ENV: &str = "GSD_JOURNAL_CHILD";

/// How many bytes the child must have produced before the kill.
///
/// The point is that the assertions are not vacuous: a child that was spawned
/// but never wrote would leave an empty file that trivially has no torn line
/// and no sequence gap. Measured ~520 KB in the research window, so this is a
/// floor reached almost immediately.
const PRODUCING_BYTES: u64 = 50_000;

/// How long to wait for the child to reach [`PRODUCING_BYTES`] before giving
/// up. Generous — a loaded CI machine is slow, but a child that has produced
/// nothing in this long is broken, not slow.
const PRODUCING_DEADLINE: Duration = Duration::from_secs(20);

/// The re-exec entry point: **the child role, not a test of anything.**
///
/// Under an ordinary `cargo test` the sentinel is absent and this returns
/// immediately, passing trivially — that is why it appears in the test list
/// with no assertions in it, and it is not dead weight. When the sentinel is
/// present it holds the journal path and this calls into the library's real
/// writer loop, which never returns; the parent kills it.
#[test]
fn journal_child_writer() {
    let Ok(path) = std::env::var(CHILD_ENV) else {
        return;
    };
    crash_test_writer_loop(Path::new(&path));
}

/// What one kill left behind.
struct Aftermath {
    /// The child's wait status.
    status: ExitStatus,
    /// The journal's raw bytes.
    raw: Vec<u8>,
    /// The journal, decoded. A torn tail can split a multi-byte character, so
    /// the decode is lossy **on purpose**: the replacement character can only
    /// ever appear inside the one incomplete trailing line, which is the line
    /// this file already tolerates.
    text: String,
    /// Kept alive so the temporary directory outlives the assertions.
    _dir: tempfile::TempDir,
}

/// Spawn the child, wait until it is genuinely producing, kill it, reap it, and
/// read back what survived.
///
/// Two things this deliberately does **not** reuse, because a pattern-matching
/// reader would reach for both:
///
/// - **The shell fixtures under `tests/fixtures/`.** They are the right tool
///   for the executor's transport tests and the wrong one here: they cannot run
///   a `JournalWriter`, which is the entire reason this harness re-execs the
///   test binary instead of spawning a script.
/// - **The process-group wrapper the executor's lifecycle tests use.** That
///   would test process-group teardown, which is the executor's concern and not
///   this one. A journal writer is a single process with no children, and the
///   standard library's child kill is already the signal this test needs —
///   confirmed by the child's wait status in every research run. Pulling the
///   wrapper in would add a dependency to this file and test the wrong thing.
fn kill_a_producing_writer() -> Aftermath {
    let dir = tempfile::tempdir().expect("temp dir");
    let journal: PathBuf = dir.path().join("journal.jsonl");

    let mut child: Child = Command::new(std::env::current_exe().expect("this test binary's path"))
        .args(["--exact", "journal_child_writer", "--nocapture"])
        .env(CHILD_ENV, &journal)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("re-exec the test binary as the writer child");

    // Deadline-bounded, so a child that never produces fails with a message
    // rather than hanging the suite forever (T-16-32).
    let deadline = Instant::now() + PRODUCING_DEADLINE;
    let mut produced = 0u64;
    while Instant::now() < deadline {
        produced = journal.metadata().map(|m| m.len()).unwrap_or(0);
        if produced > PRODUCING_BYTES {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    if produced <= PRODUCING_BYTES {
        let _ = child.kill();
        let _ = child.wait();
        panic!(
            "the writer child produced only {produced} bytes in {PRODUCING_DEADLINE:?}; \
             every assertion below would be vacuous against a file this small"
        );
    }

    // `std::process::Child::kill()` IS SIGKILL on unix — no libc, no nix, no
    // new dependency.
    child.kill().expect("kill the writer child");
    let status = child.wait().expect("reap the writer child");

    let raw = std::fs::read(&journal).expect("read the surviving journal");
    let text = String::from_utf8_lossy(&raw).into_owned();
    Aftermath {
        status,
        raw,
        text,
        _dir: dir,
    }
}

impl Aftermath {
    /// Every newline-delimited segment, including a final incomplete one.
    fn segments(&self) -> Vec<&str> {
        let mut segments: Vec<&str> = self.text.split('\n').collect();
        // A file ending in `\n` splits with a trailing empty element that is
        // not a line; a file that does not ends with the torn fragment, which
        // is.
        if self.raw.last() == Some(&b'\n') {
            segments.pop();
        }
        segments
    }
}

#[test]
fn a_sigkilled_writer_leaves_every_flushed_line_readable() {
    let after = kill_a_producing_writer();

    // 1. Died BY SIGNAL, not by exiting. Without this the whole test passes
    //    against a child that simply finished, which would make every other
    //    assertion a statement about an ordinary file.
    assert_eq!(
        after.status.code(),
        None,
        "the child must have died by signal, not exited: {:?}",
        after.status
    );

    let segments = after.segments();
    let mut records: Vec<JournalRecord> = Vec::new();
    let mut torn: Vec<usize> = Vec::new();
    for (index, segment) in segments.iter().enumerate() {
        match parse_line(segment) {
            // 5. Every complete line reparses as JSON — parsing it into a
            //    record is a strictly stronger statement than that.
            ParsedLine::Record(record) => records.push(record),
            ParsedLine::Unparseable { .. } => torn.push(index),
        }
    }

    // 2. The writer was genuinely producing. Measured ~2 150 lines in the
    //    research window; 100 is a deliberately generous floor.
    assert!(
        records.len() > 100,
        "only {} complete lines survived, from {} bytes",
        records.len(),
        after.raw.len()
    );

    // 3. ZERO sequence gaps. This is what proves ordering (D-29) rather than
    //    mere presence: a file missing an interior event would still have many
    //    complete, valid lines.
    assert_eq!(
        seq_gaps(&records),
        Vec::<(u64, u64)>::new(),
        "the surviving records must be an unbroken run of sequence numbers"
    );
    assert_eq!(records[0].seq, 1, "seq starts at 1");
    assert_eq!(
        records[records.len() - 1].seq,
        records.len() as u64,
        "and the last surviving seq equals the count, so nothing before it is missing"
    );

    // 4. At most ONE torn line, and if it exists it is the LAST. This is a
    //    ceiling, not an expectation: RESEARCH §3.2 measured zero torn lines in
    //    eight of eight runs. An interior unparseable line would fail here,
    //    which is what makes the assertion mean something.
    assert!(
        torn.len() <= 1,
        "at most one line may be torn, found {} at {torn:?}",
        torn.len()
    );
    if let Some(index) = torn.first() {
        assert_eq!(
            *index,
            segments.len() - 1,
            "a torn line is only tolerable as the last one"
        );
    }
}

#[test]
fn a_sigkilled_writers_surviving_bytes_are_already_redacted() {
    // D-27: the assertion is on the BYTES ON DISK after the kill, not on an
    // in-memory value. An in-memory assertion is the same category of vacuous
    // check as grepping filtered `cargo` output for a warning marker.
    let after = kill_a_producing_writer();
    assert_eq!(after.status.code(), None, "died by signal");

    // The planted values come from the library constants the child actually
    // wrote, and the expected replacements come from the library redactor
    // itself — so neither side of this comparison can drift out of agreement
    // with the code under test by being retyped here.
    let expected_key = redact(CRASH_TEST_PLANTED_KEY);
    let expected_home = redact(CRASH_TEST_PLANTED_HOME);
    assert_ne!(
        expected_key, CRASH_TEST_PLANTED_KEY,
        "the redactor must actually change the planted credential"
    );
    assert_ne!(
        expected_home, CRASH_TEST_PLANTED_HOME,
        "the redactor must actually change the planted home path"
    );

    assert!(
        !after.text.contains(CRASH_TEST_PLANTED_KEY),
        "the planted credential survived the kill and is on disk"
    );
    assert!(
        after.text.contains(&expected_key),
        "the replacement literal is not in the file, so nothing was written at all"
    );

    // The dash-encoded home path gets its own explicit assertion because it is
    // the WR-15 case: `tests/fixtures/transcripts/README.md` records that the
    // original Phase 15 sweep matched only the slash form and missed this shape
    // in seven of eight fixtures, and concludes that a grep for the slash form
    // alone is not evidence of a clean capture.
    assert!(
        !after.text.contains(CRASH_TEST_PLANTED_HOME),
        "the planted dash-encoded home path survived the kill and is on disk (WR-15)"
    );
    assert!(
        after.text.contains(&expected_home),
        "the dash-encoded replacement literal is not in the file (WR-15)"
    );

    // And the file is not merely redacted, it is still a journal: the redaction
    // pass must not have broken the framing it runs inside.
    let complete = after
        .segments()
        .iter()
        .filter(|segment| matches!(parse_line(segment), ParsedLine::Record(_)))
        .count();
    assert!(complete > 100, "only {complete} complete lines survived");
}
