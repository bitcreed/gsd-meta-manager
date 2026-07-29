//! Byte-offset tail and tolerant record parse (OBS-01, OBS-06, D-13, D-30).
//!
//! Parsing here is **tolerant by construction and never fatal**, for the same
//! reason `src/executor/stream_json.rs:1-32` gives for the wire model: every
//! line arriving here comes from a file an untrusted agent's output shaped.
//! Concretely:
//!
//! - Serde's strict unknown-field rejection attribute is never opted into
//!   anywhere under `src/`, and the guard that enforces that is
//!   `tests/spawn_seam_guard.rs::no_executable_line_in_src_opts_into_strict_unknown_field_rejection`.
//!   It walks every non-comment line in the tree and fails if the attribute
//!   appears. The attribute is still deliberately **not named in prose here**,
//!   but the reason has changed: the guard assembles the searched literal from
//!   two halves at runtime precisely so that naming it somewhere cannot make the
//!   guard match itself, and there is no reason to spend that tolerance here.
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

/// What a whole-journal read observed besides the records themselves.
///
/// Every field is a count, an offset-free pair of sequence numbers, or a
/// sequence number. **None of it carries event content** (D-28), so a caller may
/// log the whole struct without going near the redactor's remit.
#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ReadDiagnostics {
    /// Lines that did not parse. Skipped, counted, never fatal.
    pub unparseable: usize,
    /// Discontinuities in `seq`, each naming the last seen and the next seen.
    pub gaps: Vec<(u64, u64)>,
    /// The `seq` of the last record read, if any records were read.
    pub last_seq: Option<u64>,
}

/// Every discontinuity in a record run's `seq`, as `(last seen, next seen)`.
///
/// A gap is a **reported diagnostic and never an error** (D-03, D-30): `seq`
/// exists so a tailing reader can notice that something is missing, and a reader
/// that refused to continue on noticing would convert a partial record into no
/// record at all — strictly worse than the condition it was reacting to.
///
/// Any step other than exactly `+1` counts, so a repeated or backwards `seq` is
/// reported too. Both are equally impossible under the writer's monotonic
/// counter, and equally worth surfacing if they ever appear.
pub fn seq_gaps(records: &[JournalRecord]) -> Vec<(u64, u64)> {
    records
        .windows(2)
        .filter(|pair| pair[1].seq != pair[0].seq + 1)
        .map(|pair| (pair[0].seq, pair[1].seq))
        .collect()
}

/// Read a whole journal from offset zero, with its diagnostics.
///
/// **Neither a gap nor an unparseable line aborts the read** (D-03, D-30). A
/// line that does not parse is counted and skipped; a discontinuity in `seq` is
/// reported in [`ReadDiagnostics::gaps`]. The only `Err` this returns is a real
/// I/O failure, and a *missing* journal is not one of those — it reads as empty,
/// because the run directory exists before the first append.
///
/// It reuses [`tail_lines`] rather than reading the file whole, so the whole-file
/// path inherits the tail's torn-line and oversize-line handling instead of
/// growing a second, subtly different copy of it. The loop is bounded by the
/// cursor strictly advancing: a read that makes no progress ends it.
pub fn read_all(path: &Path) -> io::Result<(Vec<JournalRecord>, ReadDiagnostics)> {
    let mut records: Vec<JournalRecord> = Vec::new();
    let mut unparseable = 0usize;
    let mut cursor = TailCursor::default();

    loop {
        let read = tail_lines(path, cursor)?;
        for line in &read.lines {
            match parse_line(line) {
                ParsedLine::Record(record) => records.push(record),
                ParsedLine::Unparseable { .. } => unparseable += 1,
            }
        }
        // A tail read that did not advance the cursor has nothing more to give:
        // either end of file, or a torn trailing line the writer has not
        // finished. Both end the read; neither is an error.
        if read.cursor == cursor {
            break;
        }
        cursor = read.cursor;
    }

    let diagnostics = ReadDiagnostics {
        unparseable,
        gaps: seq_gaps(&records),
        last_seq: records.last().map(|record| record.seq),
    };
    Ok((records, diagnostics))
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

    /// One well-formed NDJSON journal line, built by hand.
    ///
    /// Deliberately **not** built through `JournalWriter`: the reader's contract
    /// is with the bytes on disk, and a fixture routed through this build's own
    /// writer could only ever produce shapes this build already understands —
    /// which is the opposite of what the forward-compatibility tests need.
    fn record_line(seq: u64, kind: &str) -> String {
        format!(
            "{{\"ts\":\"2026-07-29T00:00:0{}Z\",\"seq\":{seq},\"kind\":\"{kind}\"}}\n",
            seq % 10
        )
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
    fn an_unknown_kind_keeps_every_one_of_its_fields() {
        // The load-bearing forward-compatibility test. Asserting only that this
        // line "does not error" is exactly the warning sign RESEARCH Pitfall 5
        // names, because the shape that loses the data does not error either:
        // an internally-tagged enum with a catch-all UNIT variant deserialises
        // this very line to a bare marker and `reason` and `needs` simply vanish
        // — the reader believes it tolerated the event while having destroyed
        // it. That is why the reader models `kind` as a plain String with the
        // remainder flattened, and why the executor made the same call for
        // `subtype` at `src/executor/stream_json.rs:21-26`. So this test asserts
        // the VALUES of both carried fields, not merely that a record came back.
        //
        // The line is written by hand in a Phase 20 shape (D-36): a
        // parked-with-reason record carrying two fields this build's writer
        // never emits.
        let raw = r#"{"ts":"2026-07-29T00:00:00Z","seq":8,"kind":"parked","reason":"verification_gaps_found","needs":"human"}"#;
        match parse_line(raw) {
            ParsedLine::Record(record) => {
                assert_eq!(record.kind, "parked");
                assert_eq!(record.seq, 8);
                assert_eq!(record.rest["reason"], "verification_gaps_found");
                assert_eq!(record.rest["needs"], "human");
                assert_eq!(record.rest.len(), 2, "no payload field may be dropped");
            }
            ParsedLine::Unparseable { error, .. } => panic!("must parse: {error}"),
        }
    }

    #[test]
    fn a_record_with_an_unexpected_extra_field_still_parses() {
        // A different serde behaviour than an unknown kind: the kind here IS one
        // this build models, and the surprise is a field alongside it. Rejecting
        // unknown fields is opt-in and this module tree never opts in (D-30), so
        // the extra field is carried rather than refused.
        let raw = r#"{"ts":"2026-07-29T00:00:00Z","seq":3,"kind":"exec_event","stream":"assistant","text":"hi","attention_budget":0.25}"#;
        match parse_line(raw) {
            ParsedLine::Record(record) => {
                assert_eq!(record.kind, "exec_event");
                assert_eq!(record.rest["text"], "hi");
                assert_eq!(record.rest["attention_budget"], 0.25);
            }
            ParsedLine::Unparseable { error, .. } => panic!("must parse: {error}"),
        }
    }

    #[test]
    fn a_sequence_gap_is_reported_and_the_read_still_returns_every_record() {
        let dir = tempfile::tempdir().expect("temp dir");
        let journal = dir.path().join("journal.jsonl");

        // RESEARCH §9.2's verified fixture: a gap (1, 2, 4), a non-JSON line,
        // then a resumption at 5.
        for seq in [1, 2, 4] {
            append(&journal, &record_line(seq, "exec_event"));
        }
        append(&journal, "this is not json at all\n");
        append(&journal, &record_line(5, "run_ended"));

        let (records, diagnostics) = read_all(&journal).expect("read");
        assert_eq!(diagnostics.gaps, vec![(2, 4)], "the gap names 2 then 4");
        assert_eq!(
            records.len(),
            4,
            "a gap must not stop the read: every well-formed line comes back"
        );
        assert_eq!(
            records.iter().map(|r| r.seq).collect::<Vec<_>>(),
            vec![1, 2, 4, 5]
        );
        assert_eq!(diagnostics.last_seq, Some(5));
        assert_eq!(diagnostics.unparseable, 1);
    }

    #[test]
    fn an_unparseable_line_is_counted_and_the_records_around_it_survive() {
        let dir = tempfile::tempdir().expect("temp dir");
        let journal = dir.path().join("journal.jsonl");

        append(&journal, &record_line(1, "run_started"));
        append(&journal, "{\"seq\":2,\"kind\":\n");
        append(&journal, &record_line(3, "run_ended"));

        let (records, diagnostics) = read_all(&journal).expect("read");
        assert_eq!(diagnostics.unparseable, 1);
        assert_eq!(records.len(), 2, "the records on both sides survive");
        assert_eq!(records[0].seq, 1, "the record BEFORE it is present");
        assert_eq!(records[0].kind, "run_started");
        assert_eq!(records[1].seq, 3, "the record AFTER it is present");
        assert_eq!(records[1].kind, "run_ended");
        // The torn line's own seq is absent, so the surviving run is discontinuous
        // and that too is reported rather than swallowed.
        assert_eq!(diagnostics.gaps, vec![(1, 3)]);
    }

    #[test]
    fn a_journal_that_does_not_exist_reads_as_no_records_and_no_diagnostics() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (records, diagnostics) = read_all(&dir.path().join("nope.jsonl")).expect("read");
        assert!(records.is_empty());
        assert_eq!(diagnostics, ReadDiagnostics::default());
    }

    #[test]
    fn an_unbroken_run_of_sequence_numbers_reports_no_gaps() {
        let records: Vec<JournalRecord> = (1..=5)
            .map(|seq| match parse_line(&record_line(seq, "exec_event")) {
                ParsedLine::Record(record) => record,
                ParsedLine::Unparseable { error, .. } => panic!("{error}"),
            })
            .collect();
        assert!(seq_gaps(&records).is_empty());
        assert!(seq_gaps(&records[..1]).is_empty(), "one record has no pair");
        assert!(seq_gaps(&[]).is_empty());
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
