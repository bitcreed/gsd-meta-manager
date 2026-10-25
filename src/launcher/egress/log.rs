//! The header-free `egress.log` (28.1-01, D-05).
//!
//! One line per request: when, which kind, method (or `CONNECT host:port`), path
//! without query, status and a verdict phrase. Nothing else is ever written: no
//! header value, no token, no body, no resolved address. A refusal names the
//! address CLASS (`loopback`), never the address.

use std::io;
use std::path::{Path, PathBuf};

/// What a request turned out to be.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// A `/v1/messages`-family request.
    Inference,
    /// A `CONNECT`.
    Connect,
    /// A request refused before it was classified further (unknown target, a
    /// head that never completed).
    Refused,
}

impl Kind {
    pub fn as_str(self) -> &'static str {
        match self {
            Kind::Inference => "inference",
            Kind::Connect => "connect",
            Kind::Refused => "refused",
        }
    }
}

/// The longest any one logged field may be.
const FIELD_MAX_CHARS: usize = 200;

/// A request-derived value made safe to write as one field of one line: control
/// characters, `\n`, `\r` and the field separator `|` become spaces, the result
/// is trimmed and cut at 200 characters (the `store::header_safe` idiom).
pub fn clean(text: &str) -> String {
    let cleaned: String = text
        .chars()
        .map(|c| if c == '|' || c.is_control() { ' ' } else { c })
        .take(FIELD_MAX_CHARS)
        .collect();
    cleaned.trim().to_owned()
}

/// One log line: `timestamp|kind|subject|path|status|verdict\n`, every
/// request-derived field passed through [`clean`].
pub fn line(
    timestamp: &str,
    kind: Kind,
    subject: &str,
    path: &str,
    status: u16,
    verdict: &str,
) -> String {
    format!(
        "{}|{}|{}|{}|{}|{}\n",
        clean(timestamp),
        kind.as_str(),
        clean(subject),
        clean(path),
        status,
        clean(verdict)
    )
}

/// The most one `egress.log` file may grow to (8 MiB `[inferred — audit]`). The driver
/// controls the request rate and the file sits on the host outside the cgroup, so an
/// unbounded log is a disk-fill channel (WR-03). At the cap the file is rotated once to
/// `egress.log.1`, so the log never exceeds twice this and always holds the most recent
/// records (WR-10).
pub const LOG_MAX_BYTES: u64 = 8 * 1024 * 1024;

/// A refused or aborted request may take at most `cap / REFUSAL_SHARE` bytes of one log
/// file; later ones are counted, not written (WR-10). The driver can make a refusal at
/// will with junk requests, so refusals must not be able to fill the file and push out
/// the records that matter (an allowed `CONNECT`, a forwarded inference call).
const REFUSAL_SHARE: u64 = 4;

/// The verdict of the line that opens a rotated file.
const ROTATED_VERDICT: &str = "log rotated: egress.log reached its size cap";

/// True for the verdicts [`EgressServer`](super::EgressServer) gives a request it did
/// not carry out: `denied: ...` and `aborted: ...`. Written by the launcher, never by
/// the driver, so the driver cannot make a forwarded request look like a refusal or the
/// other way round.
fn is_refusal(verdict: &str) -> bool {
    verdict.starts_with("denied") || verdict.starts_with("aborted")
}

/// The name of the rotated file: `<path>.1`.
fn rotated_path(path: &Path) -> PathBuf {
    let mut name = path.as_os_str().to_owned();
    name.push(".1");
    PathBuf::from(name)
}

/// What the log's mutex guards: the file and the cap bookkeeping, so the cap check,
/// the write and the byte count are one critical section.
struct Sink {
    file: tokio::fs::File,
    /// Bytes in the file: its length at open plus everything written since.
    written: u64,
    /// Bytes of refusal records written to this file ([`REFUSAL_SHARE`]).
    refused_written: u64,
    /// Refusals dropped and not yet reported in a line.
    suppressed: u64,
    /// The line saying the refusal budget is used up was written to this file.
    budget_noted: bool,
    /// Rotation failed: nothing more is written (the log never stops traffic).
    stopped: bool,
}

/// The append-only `egress.log`, mode 0600, capped at a byte limit per file.
pub struct EgressLog {
    sink: tokio::sync::Mutex<Sink>,
    path: PathBuf,
    cap: u64,
}

async fn open_append(path: &Path) -> io::Result<tokio::fs::File> {
    tokio::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(path)
        .await
}

impl EgressLog {
    /// Open (creating it 0600 if need be) `path` for appending, capped at
    /// [`LOG_MAX_BYTES`] per file. Synchronous, so it runs before the server's runtime
    /// exists and no `async fn` blocks.
    pub fn open(path: &Path) -> io::Result<EgressLog> {
        EgressLog::open_capped(path, LOG_MAX_BYTES)
    }

    /// [`EgressLog::open`] with an explicit cap (the test seam). A file already at
    /// or past `cap` is rotated by the first record written to it.
    pub fn open_capped(path: &Path, cap: u64) -> io::Result<EgressLog> {
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .mode(0o600)
            .open(path)?;
        file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
        let written = file.metadata()?.len();
        Ok(EgressLog {
            sink: tokio::sync::Mutex::new(Sink {
                file: tokio::fs::File::from_std(file),
                written,
                refused_written: 0,
                suppressed: 0,
                budget_noted: false,
                stopped: false,
            }),
            path: path.to_owned(),
            cap,
        })
    }

    /// Move the full file to `<path>.1` (replacing an older one) and start a new file
    /// whose first line says so. The one place a record can be lost, and only the
    /// oldest ones: the previous file stays on disk as the audit of what came before.
    async fn rotate(&self, sink: &mut Sink, timestamp: &str) -> io::Result<()> {
        use tokio::io::AsyncWriteExt;
        let _ = sink.file.flush().await;
        tokio::fs::rename(&self.path, rotated_path(&self.path)).await?;
        sink.file = open_append(&self.path).await?;
        let verdict = if sink.suppressed > 0 {
            format!(
                "{ROTATED_VERDICT}; {} refused records were not logged",
                sink.suppressed
            )
        } else {
            ROTATED_VERDICT.to_owned()
        };
        let marker = line(timestamp, Kind::Refused, "-", "-", 0, &verdict);
        sink.file.write_all(marker.as_bytes()).await?;
        sink.written = marker.len() as u64;
        sink.refused_written = 0;
        sink.suppressed = 0;
        sink.budget_noted = false;
        Ok(())
    }

    /// Append one line. A write failure is dropped: the log never stops traffic.
    ///
    /// At the cap the file is rotated (see [`EgressLog::rotate`]), so the log keeps
    /// the most recent records instead of going silent. Refusals have a smaller share
    /// ([`REFUSAL_SHARE`]) of each file: past it they are counted, and the count is
    /// written before the next record that was carried out. The cap check, the write
    /// and the byte counts all happen under the one lock, so concurrent records cannot
    /// push a file past the cap plus its marker lines.
    pub async fn record(&self, kind: Kind, subject: &str, path: &str, status: u16, verdict: &str) {
        use tokio::io::AsyncWriteExt;
        let timestamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let text = line(&timestamp, kind, subject, path, status, verdict);
        let refusal = is_refusal(verdict);
        let mut sink = self.sink.lock().await;
        if sink.stopped {
            return;
        }
        let mut pending = String::new();
        if refusal && sink.refused_written + text.len() as u64 > self.cap / REFUSAL_SHARE {
            sink.suppressed += 1;
            if sink.budget_noted {
                return;
            }
            sink.budget_noted = true;
            pending.push_str(&line(
                &timestamp,
                Kind::Refused,
                "-",
                "-",
                0,
                "refusal budget of this log file reached: later refusals are counted, not logged",
            ));
        } else {
            if !refusal && sink.suppressed > 0 {
                pending.push_str(&line(
                    &timestamp,
                    Kind::Refused,
                    "-",
                    "-",
                    0,
                    &format!("{} refused records were not logged", sink.suppressed),
                ));
                sink.suppressed = 0;
            }
            pending.push_str(&text);
        }
        if sink.written + pending.len() as u64 > self.cap
            && self.rotate(&mut sink, &timestamp).await.is_err()
        {
            sink.stopped = true;
            return;
        }
        if sink.file.write_all(pending.as_bytes()).await.is_ok() {
            sink.written += pending.len() as u64;
            if refusal {
                sink.refused_written += pending.len() as u64;
            }
        }
        let _ = sink.file.flush().await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn request_derived_text_cannot_break_a_line_or_a_field() {
        assert_eq!(clean("a\nb\rc|d\te\u{7f}f\u{0}g"), "a b c d e f g");
        assert_eq!(clean("  padded  "), "padded");
        assert_eq!(clean(&"x".repeat(500)).len(), 200);
        assert_eq!(clean("é"), "é");
    }

    #[test]
    fn a_line_has_six_pipe_separated_fields_and_one_newline() {
        let l = line(
            "2026-10-24T20:00:00.000Z",
            Kind::Inference,
            "POST",
            "/v1/messages",
            200,
            "forwarded",
        );
        assert_eq!(
            l,
            "2026-10-24T20:00:00.000Z|inference|POST|/v1/messages|200|forwarded\n"
        );
        let hostile = line(
            "t",
            Kind::Refused,
            "GE\nT",
            "/a|b\r\n/c",
            403,
            "denied: x\ny",
        );
        assert_eq!(hostile.matches('\n').count(), 1, "{hostile:?}");
        assert!(hostile.ends_with('\n'));
        assert_eq!(hostile.trim_end().split('|').count(), 6, "{hostile:?}");
    }

    // -------------------------------------------------------------- the cap

    async fn record_n(log: &EgressLog, n: usize) {
        for _ in 0..n {
            log.record(Kind::Inference, "POST", "/v1/messages", 200, "forwarded")
                .await;
        }
    }

    async fn refuse_n(log: &EgressLog, n: usize) {
        for i in 0..n {
            log.record(
                Kind::Refused,
                "GET",
                &format!("/junk/{i}"),
                403,
                "denied: not an allowed path",
            )
            .await;
        }
    }

    /// A marker or notice line is at most this long: six fields, the longest being
    /// the verdict, plus a count.
    const MARKER_ROOM: u64 = 200;

    fn assert_whole_lines(text: &str) {
        assert!(
            text.is_empty() || text.ends_with('\n'),
            "partial line: {text:?}"
        );
        for l in text.lines() {
            assert_eq!(l.split('|').count(), 6, "not a whole record: {l:?}");
        }
    }

    fn read(path: &Path) -> String {
        std::fs::read_to_string(path).expect("read")
    }

    #[tokio::test]
    async fn a_full_log_is_rotated_once_and_keeps_the_most_recent_records() {
        let dir = tempfile::TempDir::new().expect("tempdir");
        let path = dir.path().join("egress.log");
        let log = EgressLog::open_capped(&path, 300).expect("open");
        record_n(&log, 40).await;
        let (current, old) = (read(&path), read(&rotated_path(&path)));
        // Two files, each within the cap plus its marker; no third file.
        assert!(current.len() as u64 <= 300 + MARKER_ROOM, "{current}");
        assert!(old.len() as u64 <= 300 + MARKER_ROOM, "{old}");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 2);
        assert!(current.lines().next().unwrap().contains(ROTATED_VERDICT));
        // Records keep arriving after the cap: the last one is in the current file.
        assert!(current.lines().last().unwrap().contains("|forwarded"));
        assert_whole_lines(&current);
        assert_whole_lines(&old);
    }

    #[tokio::test]
    async fn a_full_log_reopened_by_a_later_launch_rotates_and_goes_on() {
        let dir = tempfile::TempDir::new().expect("tempdir");
        let path = dir.path().join("egress.log");
        let first = EgressLog::open_capped(&path, 300).expect("open");
        record_n(&first, 4).await;
        drop(first);
        let before = read(&path);
        let second = EgressLog::open_capped(&path, 300).expect("reopen");
        record_n(&second, 3).await;
        let current = read(&path);
        assert!(current.matches("|forwarded").count() >= 1, "{current}");
        // What the first launch wrote is still on disk, whichever file holds it.
        let all = format!("{}{current}", read(&rotated_path(&path)));
        assert!(all.contains(before.lines().next().unwrap().split('|').nth(1).unwrap()));
    }

    #[tokio::test]
    async fn junk_refusals_cannot_push_out_the_records_that_matter() {
        // WR-10: the driver chooses its request rate, so a flood of refusals must
        // neither silence the log nor hide a request that was carried out.
        let dir = tempfile::TempDir::new().expect("tempdir");
        let path = dir.path().join("egress.log");
        let log = EgressLog::open_capped(&path, 2000).expect("open");
        refuse_n(&log, 200).await;
        log.record(Kind::Connect, "CONNECT crates.io:443", "-", 200, "allowed")
            .await;
        let text = read(&path);
        assert!(
            text.lines().last().unwrap().contains("|allowed"),
            "the allowed CONNECT is in the log: {text}"
        );
        // Refusals stopped being written at their share of the file ...
        assert!(text.matches("denied: not an allowed path").count() < 200);
        assert!(text.contains("refusal budget of this log file reached"));
        // ... and the count of the dropped ones was written before the next real record.
        assert!(text.contains("refused records were not logged"), "{text}");
        // The file never went past the cap plus its notice lines, and was not rotated
        // by the flood alone.
        assert!(
            text.len() as u64 <= 2000 + 2 * MARKER_ROOM,
            "{} bytes",
            text.len()
        );
        assert!(!rotated_path(&path).exists());
        assert_whole_lines(&text);
    }

    #[tokio::test]
    async fn concurrent_records_never_pass_the_cap() {
        let dir = tempfile::TempDir::new().expect("tempdir");
        let path = dir.path().join("egress.log");
        let log = std::sync::Arc::new(EgressLog::open_capped(&path, 2000).expect("open"));
        let mut tasks = Vec::new();
        for _ in 0..32 {
            let log = std::sync::Arc::clone(&log);
            tasks.push(tokio::spawn(async move { record_n(&log, 20).await }));
        }
        for task in tasks {
            task.await.expect("task");
        }
        for file in [path.clone(), rotated_path(&path)] {
            let text = read(&file);
            assert!(
                text.len() as u64 <= 2000 + MARKER_ROOM,
                "{} bytes in {file:?}",
                text.len()
            );
            assert_whole_lines(&text);
        }
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 2);
    }
}
