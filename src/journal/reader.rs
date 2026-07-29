//! Byte-offset tail and tolerant record parse (OBS-01, OBS-06, D-13, D-30).
//!
//! Parsing here is **tolerant by construction and never fatal**, for the same
//! reason `src/executor/stream_json.rs:1-32` gives for the wire model: every
//! line arriving here comes from a file an untrusted agent's output shaped.
//! Concretely:
//!
//! - Serde's strict unknown-field rejection attribute (`deny_unknown_fields`) is
//!   never opted into anywhere in this file, and its absence is grepped for as a
//!   mechanical guard.
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
    /// truncated in place and a run directory is never pruned while active, so a
    /// `true` here means an invariant broke. Journal it as a diagnostic — the
    /// same treatment D-30 gives a `seq` gap.
    pub restarted: bool,
    /// A line exceeded [`MAX_TAIL_BYTES`] and was stepped over. Diagnostic.
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

    fn append(path: &Path, text: &str) {
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .expect("open");
        f.write_all(text.as_bytes()).expect("write");
    }

    #[test]
    fn a_torn_final_line_is_left_for_the_next_read() {
        let dir = tempfile::tempdir().expect("temp dir");
        let journal = dir.path().join("journal.jsonl");

        append(&journal, "{\"seq\":1}\n{\"seq\":2}\n");
        let first = tail_lines(&journal, TailCursor::default()).expect("tail");
        assert_eq!(first.lines, vec!["{\"seq\":1}", "{\"seq\":2}"]);
        assert_eq!(first.cursor.offset, 20);

        // A torn write lands: no trailing newline yet.
        append(&journal, "{\"seq\":3,\"partial\":tr");
        let second = tail_lines(&journal, first.cursor).expect("tail");
        assert!(
            second.lines.is_empty(),
            "a torn trailing line must not be yielded"
        );
        assert_eq!(
            second.cursor, first.cursor,
            "the cursor must not advance past an incomplete line"
        );

        // The writer finishes it; the whole line arrives exactly once.
        append(&journal, "ue}\n");
        let third = tail_lines(&journal, second.cursor).expect("tail");
        assert_eq!(third.lines, vec!["{\"seq\":3,\"partial\":true}"]);
        assert_eq!(
            third.cursor.offset,
            journal.metadata().expect("stat").len()
        );
    }

    #[test]
    fn a_missing_journal_reads_as_empty_rather_than_erroring() {
        let dir = tempfile::tempdir().expect("temp dir");
        let read = tail_lines(&dir.path().join("nope.jsonl"), TailCursor::default())
            .expect("a missing file is not an error");
        assert!(read.lines.is_empty());
        assert_eq!(read.cursor.offset, 0);
        assert!(!read.restarted);
    }

    #[test]
    fn a_shrinking_journal_restarts_the_cursor_and_says_so() {
        let dir = tempfile::tempdir().expect("temp dir");
        let journal = dir.path().join("journal.jsonl");
        append(&journal, "{\"seq\":1}\n{\"seq\":2}\n");

        let stale = TailCursor { offset: 999 };
        let read = tail_lines(&journal, stale).expect("tail");
        assert!(read.restarted, "a shorter file must report a restart");
        assert_eq!(read.lines.len(), 2);
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
