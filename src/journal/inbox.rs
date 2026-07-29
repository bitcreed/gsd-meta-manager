//! `inbox.jsonl`: the TUI→driver message channel (D-03, D-04, D-05, D-06).
//!
//! **The TUI holds no handle on the run, and that single fact shapes this whole
//! module.** `ExecutionHandle::stdin_tx` (`src/executor/mod.rs:410`) exists only
//! inside the detached driver process, so the TUI cannot write to the agent's
//! stdin — the two processes share exactly one primitive, the filesystem. The
//! injection path is therefore, in order: the TUI appends a line here, the
//! driver tails it from a byte cursor, the driver calls `Executor::send`, the
//! driver journals the outcome, the TUI reads the journal. There is no shortcut,
//! and a design in which the TUI "sends" anywhere other than to a file cannot
//! survive a TUI restart and therefore cannot satisfy STEER-03.
//!
//! Three properties are decisions rather than incidental implementation:
//!
//! 1. **The append is `write_all` of one `\n`-terminated buffer followed by
//!    `sync_data()`** — see [`append`], which also records why it diverges from
//!    its sibling [`super::writer`] on the durability syscall.
//! 2. **The tail delegates to [`super::reader::tail_lines`]** (D-04). That
//!    function already handles a missing file, a shrunk file, a file growing
//!    during the read, a non-UTF-8 fragment and an oversize line without a
//!    newline. The inbox is the same problem, and a second tailer would be a
//!    second, subtly different copy of five edge cases that took a measurement
//!    each to get right.
//! 3. **Every line carries a client-generated id** (D-05). Text alone is not a
//!    correlation key: a user may legitimately send the same sentence twice, and
//!    STEER-02's states are per message rather than per string. The id is
//!    generated at queue time, so the `queued` state is addressable before any
//!    other process has seen the line.
//!
//! Nothing here logs message content (D-28, T-18-03): a body could carry text a
//! user typed, and the diagnostics this module surfaces are flags and counts.

use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::Path;

use serde::{Deserialize, Serialize};

use super::reader::{tail_lines, TailCursor};

/// The inbox's file name inside a run directory.
///
/// Referenced by [`super::run_paths`] rather than joined at call sites, so the
/// layout lives in exactly one place (D-01's precedent).
pub const INBOX_FILE: &str = "inbox.jsonl";

/// Upper bound on one injected message, **in `char`s and never in bytes**.
///
/// The unit matters more than the number. A byte-slice truncation splits a
/// multi-byte scalar and panics, and the text arriving here is human-typed and
/// routinely contains non-ASCII; counting `char`s means the cap can never land
/// mid-scalar. See [`cap_chars`].
///
/// 4 000 is a defensible starting value with **no tuning data behind it** — it
/// is a named constant so tuning is a one-line change, following the
/// `main_loop.rs:41-49` idiom. Note it bounds what is *delivered*; the copy
/// recorded in the journal is independently bounded in **bytes** by
/// [`super::MAX_EVENT_PAYLOAD_BYTES`], which is the redactor's remit and not
/// this module's.
pub const MAX_INBOX_TEXT_CHARS: usize = 4_000;

/// One line of `inbox.jsonl` (D-05).
///
/// The minimum shape CONTEXT fixed, and no more: an id, a timestamp, and the
/// text verbatim. **The text is a Rust `String` end to end** — written with
/// `serde_json`, read back with `serde_json`, handed to `UserMessage::text` —
/// so a multi-byte or emoji message round-trips byte-identically from this line
/// to the agent's stdin. Nothing in the path slices bytes.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct InboxMessage {
    /// The client-generated correlation id (D-05).
    pub id: String,
    /// RFC3339 UTC, to second precision, stamped when the message was queued.
    pub ts: String,
    /// The user's text, verbatim and never interpreted (Phase 21 owns any
    /// interpretation; this phase stores what was typed).
    pub text: String,
}

impl InboxMessage {
    /// Build a message with a fresh id, the current timestamp, and `text`
    /// capped by [`cap_chars`].
    ///
    /// The id is minted **here**, at queue time and by the writing side, which
    /// is what makes the `queued` state addressable before any reader exists
    /// (D-05).
    pub fn new(text: &str) -> Self {
        Self {
            id: new_message_id(),
            ts: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
            text: cap_chars(text, MAX_INBOX_TEXT_CHARS),
        }
    }
}

/// A fresh correlation id: a v4 UUID in its simple, undashed hex form (D-05).
///
/// `uuid` is already a dependency (`journal::new_run_id` uses it), so this adds
/// none. The value authenticates nothing and is never a security control — it
/// exists so two messages carrying the same sentence stay distinguishable.
pub fn new_message_id() -> String {
    uuid::Uuid::new_v4().simple().to_string()
}

/// Truncate `text` to at most `max` **`char`s**, never to `max` bytes.
///
/// **The failure this prevents is a panic, not a cosmetic clipping.**
/// `&text[..max]` panics when `max` falls inside a multi-byte scalar, and every
/// emoji and accented character makes that reachable with ordinary human input.
/// A `char`-boundary truncation cannot land mid-scalar by construction.
///
/// Pure, and that is what makes the dangerous branch testable at all: the cap is
/// only reached by a message longer than [`MAX_INBOX_TEXT_CHARS`], which no
/// end-to-end test wants to type.
pub fn cap_chars(text: &str, max: usize) -> String {
    match text.char_indices().nth(max) {
        Some((byte_index, _)) => text[..byte_index].to_string(),
        None => text.to_string(),
    }
}

/// What one inbox tail read yielded.
///
/// Every field is a message, a cursor, a flag or a count — **no field carries a
/// diagnostic string built from message content** (D-28).
#[derive(Debug, Default)]
pub struct InboxRead {
    /// The messages this read consumed, in file order.
    pub messages: Vec<InboxMessage>,
    /// Advance the caller's stored cursor to this.
    pub cursor: TailCursor,
    /// The file shrank under us and the cursor was reset to 0.
    ///
    /// **Surfaced, never swallowed**, for the same reason
    /// [`super::reader::TailRead::restarted`] gives: nothing in this design
    /// truncates an inbox in place, so a `true` here can only mean an invariant
    /// broke, and the consequence is re-delivery of messages already sent.
    pub restarted: bool,
    /// A line exceeded the tail bound and was stepped over. Diagnostic.
    ///
    /// Stepping over loses one region; not stepping over parks the cursor
    /// forever and loses every later message — see
    /// [`super::reader::TailRead::skipped_oversize`].
    pub skipped_oversize: bool,
    /// Complete lines that did not deserialise as an [`InboxMessage`].
    ///
    /// **Counted rather than discarded silently.** A torn line is not possible
    /// here — [`tail_lines`] leaves a partial line unconsumed — so a line that
    /// is complete and still does not parse means something other than this
    /// module wrote to the file, which is worth a human's attention even though
    /// it is not fatal.
    pub unparseable: usize,
}

/// Append one message durably, as a single `\n`-terminated line.
///
/// The framing discipline is [`super::writer`]'s, copied deliberately: **one
/// record is one `write_all` of one buffer that already ends in `\n`**, never a
/// write of the line followed by a write of the newline, because two calls
/// reintroduce exactly the torn-line window that one call closes.
///
/// **The `sync_data()` is the deliberate divergence from that sibling, and it is
/// the whole of STEER-03.** `journal/writer.rs:13-15` records that the journal
/// has *no* per-event durability syscall and why: a `write(2)` that has returned
/// has handed its bytes to the kernel page cache, and process death cannot
/// revoke pages the kernel already owns — so the journal's promise, survival of
/// *process* death, is met without one. This file's promise is different.
/// STEER-03 asks that *"a message queued moments before the TUI restarts is
/// still delivered"*, and the acceptance test's whole shape is that the bytes
/// were on disk **before any reader existed**. A buffered write that dies with
/// the process satisfies the UI and fails the criterion, so the syscall is paid
/// here and nowhere else. The cost is bounded by the cadence: a human types a
/// steering message every few minutes, not every few milliseconds.
///
/// The call is blocking filesystem work and must run behind
/// `tokio::task::spawn_blocking` (D-28, WR-10) — the deadlock that discipline
/// prevents is recorded as *observed* at `tests/driver_lock.rs:201-215`.
pub fn append(inbox_path: &Path, message: &InboxMessage) -> io::Result<()> {
    let line = serde_json::to_string(message).map_err(io::Error::other)?;

    let mut buffer = String::with_capacity(line.len() + 1);
    buffer.push_str(&line);
    buffer.push('\n');

    let mut file = OpenOptions::new()
        .create(true)
        .append(true)
        .open(inbox_path)?;
    file.write_all(buffer.as_bytes())?;
    file.sync_data()
}

/// Read every complete inbox line after `cursor`, leaving a torn tail alone.
///
/// **Delegates to [`tail_lines`] rather than re-implementing a tail** (D-04).
/// That function is a measured solution to five edge cases — a missing file, a
/// file shorter than the cursor, a file growing during the read, a non-UTF-8
/// fragment, and a line too long to complete inside the read bound — and the
/// inbox has every one of them. It is also what bounds this read against an
/// adversarially large file (T-18-04): `MAX_TAIL_BYTES` caps the bytes consumed
/// and the cursor steps over an oversize region rather than parking.
///
/// Both of its diagnostic flags are propagated onto [`InboxRead`] rather than
/// dropped, and an unparseable line is **counted** rather than discarded
/// silently. Losing either would turn a real anomaly into a message that simply
/// never arrives, which is the failure this whole channel exists to make
/// visible.
pub fn tail(inbox_path: &Path, cursor: TailCursor) -> io::Result<InboxRead> {
    let read = tail_lines(inbox_path, cursor)?;

    let mut messages = Vec::with_capacity(read.lines.len());
    let mut unparseable = 0usize;
    for line in &read.lines {
        match serde_json::from_str::<InboxMessage>(line) {
            Ok(message) => messages.push(message),
            Err(_) => unparseable += 1,
        }
    }

    Ok(InboxRead {
        messages,
        cursor: read.cursor,
        restarted: read.restarted,
        skipped_oversize: read.skipped_oversize,
        unparseable,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn message(text: &str) -> InboxMessage {
        InboxMessage {
            id: "3f2a".to_string(),
            ts: "2026-07-29T21:40:02Z".to_string(),
            text: text.to_string(),
        }
    }

    #[test]
    fn an_appended_message_is_one_newline_terminated_line_that_tails_back_whole() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join(INBOX_FILE);

        append(&path, &message("skip the UI review")).expect("append");
        let bytes = std::fs::read_to_string(&path).expect("read the inbox back");
        assert_eq!(
            bytes.matches('\n').count(),
            1,
            "one message is one line, so two writes cannot tear it"
        );
        assert!(bytes.ends_with('\n'));

        let read = tail(&path, TailCursor::default()).expect("tail");
        assert_eq!(read.messages.len(), 1);
        assert_eq!(read.messages[0].text, "skip the UI review");
        assert_eq!(read.messages[0].id, "3f2a");
        assert_eq!(read.unparseable, 0);
        assert!(!read.restarted);
        assert!(!read.skipped_oversize);
        assert_eq!(
            read.cursor.offset,
            path.metadata().expect("stat").len(),
            "a complete file leaves the cursor at its end"
        );

        // A second read from the advanced cursor must not re-deliver it.
        let again = tail(&path, read.cursor).expect("tail");
        assert!(
            again.messages.is_empty(),
            "a message delivered once must not arrive twice"
        );
    }

    #[test]
    fn a_torn_line_is_neither_delivered_nor_discarded_and_arrives_once_completed() {
        // The durability property STEER-03 leans on, at the inbox's own layer.
        // The delegation to `tail_lines` is what supplies it — this test is
        // what fails if someone "simplifies" the tail into a line-iterating
        // reader, which treats end-of-file as a line terminator and would yield
        // the torn fragment as though it were a whole message.
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join(INBOX_FILE);

        append(&path, &message("first")).expect("append");
        let first = tail(&path, TailCursor::default()).expect("tail");
        assert_eq!(first.messages.len(), 1);

        // A torn write lands: no trailing newline yet.
        let whole = serde_json::to_string(&message("second")).expect("serialise");
        let (head, tail_bytes) = whole.split_at(whole.len() / 2);
        std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .expect("open")
            .write_all(head.as_bytes())
            .expect("write the torn half");

        let second = tail(&path, first.cursor).expect("tail");
        assert!(
            second.messages.is_empty(),
            "a torn fragment must not be delivered as a message"
        );
        assert_eq!(
            second.cursor, first.cursor,
            "and it must not be discarded either: the cursor stays put"
        );

        // The writer finishes it; the whole message arrives exactly once.
        let mut rest = tail_bytes.to_string();
        rest.push('\n');
        std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .expect("open")
            .write_all(rest.as_bytes())
            .expect("complete the line");

        let third = tail(&path, second.cursor).expect("tail");
        assert_eq!(third.messages.len(), 1, "the completed line arrives once");
        assert_eq!(third.messages[0].text, "second");
        assert!(
            tail(&path, third.cursor)
                .expect("tail")
                .messages
                .is_empty(),
            "and is not re-delivered on the read after that"
        );
    }

    #[test]
    fn a_multibyte_message_round_trips_byte_identically() {
        // STEER-01's encoding case. The text is a Rust `String` end to end and
        // is serialised by `serde_json`, so this asserts that nothing on the
        // path reaches for a byte slice.
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join(INBOX_FILE);
        let text = "skip the UI review \u{1F680} \u{00E9}\u{4E2D}\u{6587} \u{1F44D}\u{1F3FD}";

        append(&path, &message(text)).expect("append");
        let read = tail(&path, TailCursor::default()).expect("tail");
        assert_eq!(
            read.messages[0].text.as_bytes(),
            text.as_bytes(),
            "a multi-byte message must survive the file byte for byte"
        );
    }

    #[test]
    fn the_length_cap_counts_chars_and_never_splits_a_scalar() {
        // A byte-slice truncation of this input panics; a char-count one cannot.
        let emoji = "\u{1F680}".repeat(10);
        assert_eq!(cap_chars(&emoji, 3).chars().count(), 3);
        assert_eq!(
            // `String::len` is a BYTE count, which is exactly the point here.
            cap_chars(&emoji, 3).len(),
            12,
            "three four-byte scalars, so the cap is not a byte count"
        );
        assert_eq!(cap_chars(&emoji, 0), "");
        assert_eq!(cap_chars(&emoji, 100), emoji, "an unreached cap is a no-op");
        assert_eq!(cap_chars("", 10), "");

        let built = InboxMessage::new(&"\u{00E9}".repeat(MAX_INBOX_TEXT_CHARS + 50));
        assert_eq!(built.text.chars().count(), MAX_INBOX_TEXT_CHARS);
        assert!(!built.id.is_empty());
    }

    #[test]
    fn a_complete_line_that_is_not_a_message_is_counted_rather_than_swallowed() {
        let dir = tempfile::tempdir().expect("temp dir");
        let path = dir.path().join(INBOX_FILE);

        append(&path, &message("before")).expect("append");
        std::fs::OpenOptions::new()
            .append(true)
            .open(&path)
            .expect("open")
            .write_all(b"this is not an inbox message at all\n")
            .expect("write");
        append(&path, &message("after")).expect("append");

        let read = tail(&path, TailCursor::default()).expect("tail");
        assert_eq!(
            read.unparseable, 1,
            "the count is the only signal that a line was skipped"
        );
        assert_eq!(
            read.messages
                .iter()
                .map(|m| m.text.as_str())
                .collect::<Vec<_>>(),
            vec!["before", "after"],
            "the messages on both sides of it must still arrive"
        );
    }

    #[test]
    fn a_missing_inbox_reads_as_empty_rather_than_as_an_error() {
        // The run directory exists before the first append, so this is the
        // steady state for every run nobody has steered.
        let dir = tempfile::tempdir().expect("temp dir");
        let read = tail(&dir.path().join(INBOX_FILE), TailCursor::default())
            .expect("a missing inbox is not an error");
        assert!(read.messages.is_empty());
        assert_eq!(read.cursor.offset, 0);
    }

    #[test]
    fn two_freshly_minted_ids_differ() {
        assert_ne!(new_message_id(), new_message_id());
        assert_eq!(new_message_id().len(), 32, "simple, undashed hex");
    }
}
