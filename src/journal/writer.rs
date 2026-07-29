//! The append-only NDJSON journal writer (OBS-01, D-03, D-04, D-29).
//!
//! One [`JournalWriter`] owns one run's `journal.jsonl` for the run's lifetime.
//! Four properties of this file are decisions, not incidental implementation:
//!
//! 1. **One record is one `write_all` of one buffer that already ends in `\n`.**
//!    Never a write of the line followed by a write of the newline: two calls
//!    reintroduce exactly the torn-line window that one call closes.
//! 2. **The handle stays open for the run.** Reopening per append costs an
//!    open/close syscall pair per event and buys nothing under `O_APPEND`.
//! 3. **No userspace buffering wrapper sits in front of the file.** Such a
//!    wrapper's entire effect would be to hold already-serialised events in the
//!    dying process's memory — which is precisely the data D-29 promises is on
//!    disk. There is likewise no per-event durability syscall; see the module
//!    root for the measurement that shows it buys nothing against process death.
//! 4. **`append` takes `&mut self`.** D-29's ordering contract — an event is
//!    written before the driver proceeds past the action it describes — is
//!    therefore satisfied *structurally*: exactly one owner can write at a time
//!    and records land in call order. A fire-and-forget spawn per event is the
//!    shape D-29 forbids, and exclusive `&mut` ownership is a stronger guarantee
//!    than an ordered channel, so no channel is introduced here for a producer
//!    that does not exist until Phase 17.
//!
//! One cosmetic consequence worth writing down so nobody "fixes" it:
//! `serde_json::Map` is a `BTreeMap` in this project — the ordering-preserving
//! feature is off — so a record's keys serialise alphabetically and a line reads
//! `kind`, `seq`, `ts`, then its payload rather than in the sketch order. That
//! is deliberate and entirely irrelevant to framing.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Context;
use serde_json::Value;

use super::redact::RedactedLine;
use super::JournalEvent;

/// An open append-only journal for one run.
pub struct JournalWriter {
    file: File,
    path: PathBuf,
    next_seq: u64,
    bytes_written: u64,
}

impl JournalWriter {
    /// Open (creating if absent) the run's journal and keep the handle.
    ///
    /// The run directory must already exist; creating it is the run-start
    /// sequence's job, which also writes the ignore file that protects this
    /// journal before a single byte of it is produced (D-08).
    pub fn open(journal_path: &Path) -> anyhow::Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(journal_path)
            .with_context(|| {
                format!("Failed to open journal at {}", journal_path.display())
            })?;

        Ok(Self {
            file,
            path: journal_path.to_path_buf(),
            next_seq: 1,
            bytes_written: 0,
        })
    }

    /// Append one event and return the `seq` it was written with.
    ///
    /// The event is rendered to a `Value`, stamped with `ts` and `seq`, handed
    /// to [`RedactedLine`] — the only type this writer can turn into bytes — and
    /// written as a single buffer.
    pub fn append(&mut self, event: &JournalEvent) -> anyhow::Result<u64> {
        let mut value =
            serde_json::to_value(event).context("Failed to render a journal event as JSON")?;

        let seq = self.next_seq;
        {
            let object = value
                .as_object_mut()
                .context("a journal event always serialises to a JSON object")?;
            object.insert(
                "ts".to_string(),
                Value::String(
                    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                ),
            );
            object.insert("seq".to_string(), Value::from(seq));
        }

        let line = RedactedLine::new(value);
        let mut buffer = String::with_capacity(line.as_line().len() + 1);
        buffer.push_str(line.as_line());
        buffer.push('\n');

        self.file
            .write_all(buffer.as_bytes())
            .with_context(|| format!("Failed to append to {}", self.path.display()))?;

        self.next_seq += 1;
        self.bytes_written += buffer.len() as u64;
        Ok(seq)
    }

    /// The `seq` the **next** append will use.
    ///
    /// Starts at 1 and increments monotonically, so a tailing reader can detect
    /// gaps; a gap is a reported diagnostic, never a parse failure (D-03).
    pub fn seq(&self) -> u64 {
        self.next_seq
    }

    /// Bytes this writer has appended. Plan 16-03 reads it for the per-run cap.
    pub fn bytes_written(&self) -> u64 {
        self.bytes_written
    }

    /// The journal path this writer holds open.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::reader::{parse_line, tail_lines, ParsedLine, TailCursor};
    use crate::journal::run_paths;

    /// An Anthropic-shaped key. Planted, never real.
    const PLANTED_KEY: &str = "sk-ant-api03-AbCdEf012345_-XyZ";
    /// Claude Code's dash-encoded session-directory form of a fictional user's
    /// home. This is the WR-15 shape — the one that historically slipped past a
    /// sweep that checked only `/home/<user>`.
    const PLANTED_DASH_HOME: &str = "-home-fakeuser-projects-secretrepo";
    /// What the redactor must have put in their place, in the file's bytes.
    const EXPECTED_KEY_LITERAL: &str = "[REDACTED:anthropic-key]";
    const EXPECTED_HOME_LITERAL: &str = "-home-redacted-project";

    fn open_in(dir: &Path) -> (JournalWriter, PathBuf) {
        let paths = run_paths(&dir.join(".planning"), "2026-07-28T14-03-11Z-a3f9");
        std::fs::create_dir_all(&paths.dir).expect("create the run directory");
        let writer = JournalWriter::open(&paths.journal).expect("open the journal");
        (writer, paths.journal)
    }

    fn planted_event() -> JournalEvent {
        JournalEvent::ExecEvent {
            stream: "assistant".to_string(),
            text: format!("cwd {PLANTED_DASH_HOME} key {PLANTED_KEY} done"),
        }
    }

    #[test]
    fn a_planted_credential_is_already_redacted_in_the_file_bytes() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut writer, journal) = open_in(dir.path());
        writer.append(&planted_event()).expect("append");

        // The assertion that closes the loop reads the BYTES ON DISK (D-27).
        // An in-memory assertion is the same category of vacuous check as
        // grepping filtered `cargo` output for `warning:`.
        let bytes = std::fs::read_to_string(&journal).expect("read the journal back");

        assert!(
            !bytes.contains(PLANTED_KEY),
            "the planted credential survived to disk"
        );
        assert!(
            !bytes.contains(PLANTED_DASH_HOME),
            "the planted dash-encoded home path survived to disk"
        );
        assert!(bytes.contains(EXPECTED_KEY_LITERAL), "got: {bytes}");
        assert!(bytes.contains(EXPECTED_HOME_LITERAL), "got: {bytes}");

        // Then read it back the way a live tail would, from a zero cursor.
        let read = tail_lines(&journal, TailCursor::default()).expect("tail");
        assert_eq!(read.lines.len(), 1, "one appended event is one tailed line");
        assert_eq!(
            read.cursor.offset,
            journal.metadata().expect("stat").len(),
            "the cursor must advance to the end of a complete file"
        );
        assert!(!read.restarted);

        match parse_line(&read.lines[0]) {
            ParsedLine::Record(record) => {
                assert_eq!(record.seq, 1);
                assert_eq!(record.kind, "exec_event");
                assert_eq!(record.rest["stream"], "assistant");
                assert!(!record.ts.is_empty());
            }
            ParsedLine::Unparseable { raw, error } => {
                panic!("the writer produced an unparseable line: {error} in {raw}")
            }
        }
    }

    #[test]
    fn every_appended_record_is_one_complete_line_with_one_newline() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut writer, journal) = open_in(dir.path());

        writer.append(&planted_event()).expect("append");
        let bytes = std::fs::read_to_string(&journal).expect("read");
        assert_eq!(bytes.matches('\n').count(), 1, "exactly one newline");
        assert!(bytes.ends_with('\n'), "the record is newline-terminated");
        assert!(
            !bytes.trim_end_matches('\n').contains('\n'),
            "no embedded newline may break the NDJSON framing"
        );

        // Three more, including one whose payload is full of newlines and
        // carriage returns: compact serialisation escapes them, so framing
        // cannot be forged from a payload.
        writer
            .append(&JournalEvent::ExecEvent {
                stream: "user".to_string(),
                text: "line one\nline two\r\n{\"kind\":\"forged\"}".to_string(),
            })
            .expect("append");
        writer
            .append(&JournalEvent::EventsDropped { count: 40 })
            .expect("append");
        writer
            .append(&JournalEvent::RunEnded {
                outcome: "completed".to_string(),
                ended_at: "2026-07-28T14:10:00Z".to_string(),
            })
            .expect("append");

        let bytes = std::fs::read_to_string(&journal).expect("read");
        assert_eq!(bytes.matches('\n').count(), 4);
        assert!(bytes.ends_with('\n'));
        assert!(!bytes.contains("\n\n"), "no blank lines (D-03)");
        for line in bytes.lines() {
            serde_json::from_str::<Value>(line).expect("every written line reparses as JSON");
        }
        assert!(
            !bytes.contains(r#"{"kind":"forged"}"#),
            "an embedded object literal must stay escaped inside its string"
        );
    }

    #[test]
    fn seq_starts_at_one_and_increments_monotonically() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut writer, journal) = open_in(dir.path());

        assert_eq!(writer.seq(), 1, "the first append uses seq 1");
        for expected in 1..=5u64 {
            assert_eq!(writer.append(&planted_event()).expect("append"), expected);
        }
        assert_eq!(writer.seq(), 6);
        assert!(writer.bytes_written() > 0);

        let bytes = std::fs::read_to_string(&journal).expect("read");
        assert_eq!(
            writer.bytes_written(),
            bytes.len() as u64,
            "the running byte count must match the file"
        );
        let seqs: Vec<u64> = bytes
            .lines()
            .map(|line| match parse_line(line) {
                ParsedLine::Record(record) => record.seq,
                ParsedLine::Unparseable { error, .. } => panic!("unparseable: {error}"),
            })
            .collect();
        assert_eq!(seqs, vec![1, 2, 3, 4, 5]);
    }
}
