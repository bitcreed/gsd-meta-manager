//! Byte-offset tail and tolerant record parse (OBS-01, OBS-06, D-13, D-30).
//!
//! Parsing here is **tolerant by construction and never fatal**, for the same
//! reason `src/executor/stream_json.rs:1-32` gives for the wire model: every
//! line arriving here comes from a file an untrusted agent's output shaped.
//! Concretely:
//!
//! - Serde's strict unknown-field rejection attribute is never opted into
//!   anywhere in this module tree, and its absence is grepped for as a
//!   mechanical guard — the same discipline `stream_json.rs:6-8` records for the
//!   wire model. The attribute is deliberately **not named in prose here**: the
//!   guard is a grep, and spelling the name in a comment is how such a guard
//!   comes to match itself and stop meaning anything.
//! - An unknown `kind` is carried **with its payload**, never absorbed into a
//!   unit variant. `kind` is a plain `String` and never an enum: an
//!   internally-tagged enum with a catch-all unit variant *discards the unknown
//!   variant's fields*, which is exactly what D-30 requires them to carry. The
//!   same rule `stream_json.rs` states for `subtype` and `terminal_reason`
//!   applies here with more force, because Phase 20 is a **known** future
//!   emitter of new `kind` values.
//! - A line that does not parse is reported as a diagnostic and skipped; it
//!   never fails a read.
//! - No panicking accessor exists in the non-test region of this file.
//!
//! ## Why the tail splits on bytes
//!
//! The standard line-iterating reader wrapper (`BufReader::lines()`) treats
//! end-of-file as a line terminator, so it yields a **torn** trailing line as if
//! it were complete. That turns a self-healing torn write into a permanently
//! lost event — the exact failure D-13 forbids. Measured against a file whose
//! last line was torn:
//!
//! ```text
//! BufReader::lines  -> ["{\"a\":1}", "{\"b\":2}", "{\"c\":tor"]   <-- yields the TORN line
//! tail_lines        -> ["{\"a\":1}", "{\"b\":2}"]                 <-- leaves it for next read
//! ```
//!
//! So the tail reads bytes, stops at the last `\n`, and leaves any trailing
//! partial line unconsumed to be re-read once the writer finishes it.
//!
//! ## Why rotation is detected by length and never by inode identity
//!
//! `MetadataExt::ino()` would be a more precise rotation signal than "the file
//! is shorter than the cursor", and it is deliberately **not** used. It is
//! `cfg(unix)`-only, so it would put a platform split through the one function
//! every driver read goes through — and it buys nothing here, because the cursor
//! is keyed by run and **each run owns its own immutable directory** (D-01,
//! D-13). No journal is ever rotated, renamed or replaced underneath a live
//! cursor, so `len < offset` is the only reachable signal that anything moved.
//! A more precise detector for a condition that cannot occur is cost without
//! coverage (RESEARCH §2.4).
//!
//! Nothing in this file logs event content (D-28): the diagnostics it surfaces
//! are flags and counts.

use std::fs::File;
use std::io::{self, ErrorKind, Read, Seek, SeekFrom};
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use super::MAX_TAIL_BYTES;

/// How far into a journal a reader has already consumed.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct TailCursor {
    /// Byte offset of the first unconsumed byte.
    pub offset: u64,
}

/// The result of one tail read.
#[derive(Debug, Default)]
pub struct TailRead {
    /// Complete, newline-terminated lines only.
    pub lines: Vec<String>,
    /// Advance the caller's stored cursor to this.
    pub cursor: TailCursor,
    /// The file shrank under us and the cursor was reset to 0.
    ///
    /// **Surface this, do not swallow it.** Under D-31/D-32 a journal is never
    /// truncated in place and a run directory is never pruned while active, so
    /// under this phase's own invariants a `true` here **can only mean an
    /// invariant broke**. That is precisely why it must reach a human: it is not
    /// a routine condition the reader absorbed, it is evidence that something
    /// upstream did a thing the design says is impossible. A call site should
    /// `tracing::warn!` on it and journal it as a `Diagnostic` — never fail the
    /// read, which is the same treatment D-30 gives a `seq` gap.
    pub restarted: bool,
    /// A line exceeded [`MAX_TAIL_BYTES`] and was stepped over. Diagnostic.
    ///
    /// With the per-event payload cap in place (D-31, [`MAX_EVENT_PAYLOAD_BYTES`])
    /// this branch is **pure defence against a corrupted file** — no line this
    /// writer produces can approach the bound, so reaching it means the bytes on
    /// disk are not bytes this writer wrote.
    ///
    /// **The cursor advancing past the offending region is the entire point.**
    /// The alternative — leaving the cursor parked, as an ordinary torn line
    /// does — is not a smaller failure but a permanent one: a line that cannot
    /// complete never completes, so the offset never moves again and *every
    /// later event in the run becomes invisible forever*. Stepping over loses one
    /// unparseable region; not stepping over loses the rest of the run.
    ///
    /// [`MAX_EVENT_PAYLOAD_BYTES`]: super::MAX_EVENT_PAYLOAD_BYTES
    pub skipped_oversize: bool,
}

/// Read every complete line after `cursor`, leaving a torn tail unconsumed.
///
/// Four edge cases are handled deliberately, each marked at its branch:
/// a missing file, a file shorter than the cursor, a file growing during the
/// read, and a non-UTF-8 fragment.
pub fn tail_lines(path: &Path, mut cursor: TailCursor) -> io::Result<TailRead> {
    // (a) The file may not exist yet: the run directory is created before the
    //     first append, so a watcher can fire on the directory first.
    let mut f = match File::open(path) {
        Ok(f) => f,
        Err(e) if e.kind() == ErrorKind::NotFound => {
            return Ok(TailRead {
                cursor,
                ..Default::default()
            })
        }
        Err(e) => return Err(e),
    };

    let len = f.metadata()?.len();

    // (b) Truncation or rotation: the file is shorter than where we left off.
    let mut restarted = false;
    if len < cursor.offset {
        cursor.offset = 0;
        restarted = true;
    }
    if len == cursor.offset {
        return Ok(TailRead {
            cursor,
            restarted,
            ..Default::default()
        });
    }

    f.seek(SeekFrom::Start(cursor.offset))?;

    // (c) Another process may be appending RIGHT NOW. Reading to end rather than
    //     reading exactly `len - offset` bytes is what tolerates the file having
    //     grown since the metadata call, and cannot fail if it shrank. `.take()`
    //     bounds the read.
    let mut buf = Vec::new();
    (&mut f).take(MAX_TAIL_BYTES).read_to_end(&mut buf)?;

    match buf.iter().rposition(|&b| b == b'\n') {
        Some(idx) => {
            let lines = buf[..=idx]
                .split(|&b| b == b'\n')
                .filter(|s| !s.is_empty())
                // (d) A non-UTF-8 fragment is skipped, never lossily mangled: a
                //     lossy conversion would silently inject U+FFFD into a
                //     payload and the corruption would be indistinguishable from
                //     content.
                .filter_map(|s| std::str::from_utf8(s).ok().map(str::to_owned))
                .collect();
            Ok(TailRead {
                lines,
                cursor: TailCursor {
                    offset: cursor.offset + idx as u64 + 1,
                },
                restarted,
                skipped_oversize: false,
            })
        }
        // A full buffer with no newline anywhere: the line cannot complete
        // within our bound. Step over it and report, or the cursor never
        // advances again and every later event is invisible forever.
        None if buf.len() as u64 >= MAX_TAIL_BYTES => Ok(TailRead {
            lines: Vec::new(),
            cursor: TailCursor {
                offset: cursor.offset + buf.len() as u64,
            },
            restarted,
            skipped_oversize: true,
        }),
        // An ordinary trailing partial line: consume nothing, leave the cursor
        // where it was, and re-read it once the writer has finished it.
        None => Ok(TailRead {
            lines: Vec::new(),
            cursor,
            restarted,
            skipped_oversize: false,
        }),
    }
}

/// One parsed journal record, with everything past the framing fields carried.
///
/// `kind` is a plain `String` and the payload is flattened into `rest`, so a
/// record whose `kind` this build has never heard of arrives **complete**.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JournalRecord {
    /// RFC3339 UTC timestamp the writer stamped.
    pub ts: String,
    /// Monotonic sequence number, starting at 1.
    pub seq: u64,
    /// The event kind, carried verbatim.
    pub kind: String,
    /// Every other field of the record.
    #[serde(flatten)]
    pub rest: Map<String, Value>,
}

/// What one observed line yielded.
#[derive(Debug, Clone)]
pub enum ParsedLine {
    /// The line parsed. Its `kind` may be one this build does not model.
    Record(JournalRecord),
    /// The line did not parse — a torn write or invalid JSON. A real
    /// diagnostic, deliberately distinct from a record of an unknown kind,
    /// which is a *known-good* line the reader carries forward.
    Unparseable {
        /// The raw line.
        raw: String,
        /// The parse error, rendered.
        error: String,
    },
}

/// Parse one NDJSON journal line. Never panics, never fails a read.
pub fn parse_line(raw: &str) -> ParsedLine {
    match serde_json::from_str::<JournalRecord>(raw) {
        Ok(record) => ParsedLine::Record(record),
        Err(e) => ParsedLine::Unparseable {
            raw: raw.to_string(),
            error: e.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    // Every tail test below runs against a **real file** in a real temporary
    // directory, never an in-memory buffer. The property under test is about
    // file offsets, file length and end-of-file semantics; a fake would test the
    // fake. The expected values are RESEARCH §2.3's executed table, so these
    // tests transcribe a measurement rather than guess an outcome.

    fn append(path: &Path, text: &str) {
        append_bytes(path, text.as_bytes());
    }

    fn append_bytes(path: &Path, bytes: &[u8]) {
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .expect("open");
        f.write_all(bytes).expect("write");
    }

    #[test]
    fn a_missing_journal_reads_as_empty_without_an_error() {
        let dir = tempfile::tempdir().expect("temp dir");
        let read = tail_lines(&dir.path().join("nope.jsonl"), TailCursor::default())
            .expect("a missing file is not an error");
        assert!(read.lines.is_empty());
        assert_eq!(read.cursor.offset, 0);
        assert!(!read.restarted);
        assert!(!read.skipped_oversize);
    }

    #[test]
    fn two_whole_lines_are_returned_and_the_cursor_lands_past_the_last_newline() {
        let dir = tempfile::tempdir().expect("temp dir");
        let journal = dir.path().join("journal.jsonl");
        append(&journal, "{\"seq\":1}\n{\"seq\":2}\n");

        let read = tail_lines(&journal, TailCursor::default()).expect("tail");
        assert_eq!(read.lines, vec!["{\"seq\":1}", "{\"seq\":2}"]);
        // Two 10-byte lines: the next unconsumed byte is byte 20, one past the
        // final newline — and exactly the file's length, since nothing is torn.
        assert_eq!(read.cursor.offset, 20);
        assert_eq!(read.cursor.offset, journal.metadata().expect("stat").len());
        assert!(!read.restarted);
        assert!(!read.skipped_oversize);
    }

    #[test]
    fn a_partial_trailing_line_is_not_consumed_and_completes_on_the_next_read() {
        // THE most important test in this file, and the reason the tail splits on
        // bytes rather than using the standard line-iterating reader: that reader
        // treats end-of-file as a line terminator and therefore yields a TORN
        // line as if it were complete. A torn write would then be a *lost event*
        // instead of a self-healing condition — the exact failure D-13 forbids
        // and the one RESEARCH §2.1 demonstrated by measurement. The assertions
        // are written so they fail against that implementation: the torn line
        // must be absent from the first read, and must arrive complete and
        // exactly once from the second.
        let dir = tempfile::tempdir().expect("temp dir");
        let journal = dir.path().join("journal.jsonl");

        append(&journal, "{\"seq\":1}\n{\"seq\":2}\n");
        let first = tail_lines(&journal, TailCursor::default()).expect("tail");
        let cursor_before = first.cursor;

        // A torn write lands: no trailing newline yet.
        append(&journal, "{\"seq\":3,\"partial\":tr");
        let second = tail_lines(&journal, cursor_before).expect("tail");
        assert!(
            second.lines.is_empty(),
            "a torn trailing line must not be yielded: {:?}",
            second.lines
        );
        assert_eq!(
            second.cursor, cursor_before,
            "the cursor must not advance one byte past an incomplete line"
        );

        // The writer finishes it; the whole line arrives exactly once.
        append(&journal, "ue}\n");
        let third = tail_lines(&journal, second.cursor).expect("tail");
        assert_eq!(third.lines, vec!["{\"seq\":3,\"partial\":true}"]);
        assert_eq!(third.lines.len(), 1, "the completed line must arrive once");
        assert_eq!(third.cursor.offset, journal.metadata().expect("stat").len());

        // And a fourth read yields nothing: the line is not re-delivered.
        let fourth = tail_lines(&journal, third.cursor).expect("tail");
        assert!(fourth.lines.is_empty());
    }

    #[test]
    fn a_file_that_shrank_resets_the_cursor_and_reports_it() {
        let dir = tempfile::tempdir().expect("temp dir");
        let journal = dir.path().join("journal.jsonl");
        append(&journal, "{\"seq\":1}\n{\"seq\":2}\n");

        let stale = TailCursor { offset: 999 };
        let read = tail_lines(&journal, stale).expect("tail");
        assert!(read.restarted, "a shorter file must report a restart");
        assert_eq!(
            read.lines,
            vec!["{\"seq\":1}", "{\"seq\":2}"],
            "a reset must re-read the whole file's complete lines"
        );
        assert_eq!(read.cursor.offset, 20, "the cursor restarts from zero");
    }

    #[test]
    fn a_line_that_cannot_complete_inside_the_bound_is_stepped_over() {
        let dir = tempfile::tempdir().expect("temp dir");
        let journal = dir.path().join("journal.jsonl");

        // A region that fills the whole tail bound with no newline anywhere. No
        // line this writer produces can reach this size (D-31 caps the payload),
        // so this only happens to a corrupted file — and the cursor MUST still
        // move, or every later event in the run is invisible forever.
        let oversize = vec![b'x'; MAX_TAIL_BYTES as usize];
        append_bytes(&journal, &oversize);

        let start = TailCursor::default();
        let read = tail_lines(&journal, start).expect("tail");
        assert!(read.lines.is_empty());
        assert!(read.skipped_oversize, "stepping over must be reported");
        assert!(
            read.cursor.offset > start.offset,
            "the cursor must strictly advance, or it is parked forever: {} !> {}",
            read.cursor.offset,
            start.offset
        );
        assert_eq!(read.cursor.offset, MAX_TAIL_BYTES);

        // Having stepped over it, the reader makes progress again.
        append_bytes(&journal, b"\n{\"seq\":1}\n");
        let next = tail_lines(&journal, read.cursor).expect("tail");
        assert_eq!(next.lines, vec!["{\"seq\":1}"]);
    }

    #[test]
    fn a_non_utf8_fragment_is_skipped_rather_than_mangled() {
        let dir = tempfile::tempdir().expect("temp dir");
        let journal = dir.path().join("journal.jsonl");

        append(&journal, "{\"seq\":1}\n");
        // A lone continuation byte and a truncated two-byte sequence: not valid
        // UTF-8 under any interpretation.
        append_bytes(&journal, &[0x80, 0xff, 0xc3, b'\n']);
        append(&journal, "{\"seq\":2}\n");

        let read = tail_lines(&journal, TailCursor::default()).expect("tail");
        assert_eq!(
            read.lines,
            vec!["{\"seq\":1}", "{\"seq\":2}"],
            "the lines around a bad fragment must still arrive"
        );
        for line in &read.lines {
            assert!(
                !line.contains('\u{fffd}'),
                "a lossy conversion would inject U+FFFD into a payload, making \
                 corruption indistinguishable from content: {line:?}"
            );
        }
        assert_eq!(read.cursor.offset, journal.metadata().expect("stat").len());
    }

    #[test]
    fn an_unknown_kind_arrives_with_its_payload_intact() {
        // The Phase 20 shape this reader must already tolerate (D-30).
        let raw = r#"{"ts":"2026-07-29T00:00:00Z","seq":8,"kind":"parked","reason":"verification_gaps_found","needs":"human"}"#;
        match parse_line(raw) {
            ParsedLine::Record(record) => {
                assert_eq!(record.kind, "parked");
                assert_eq!(record.seq, 8);
                assert_eq!(record.rest["reason"], "verification_gaps_found");
                assert_eq!(record.rest["needs"], "human");
            }
            ParsedLine::Unparseable { error, .. } => panic!("must parse: {error}"),
        }
    }

    #[test]
    fn a_malformed_line_is_a_diagnostic_and_not_a_failure() {
        match parse_line("{\"seq\":1,\"kind\":") {
            ParsedLine::Unparseable { raw, error } => {
                assert!(raw.starts_with("{\"seq\""));
                assert!(!error.is_empty());
            }
            ParsedLine::Record(_) => panic!("a torn line must not parse as a record"),
        }
    }
}
