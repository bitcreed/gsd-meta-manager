//! The header-free `egress.log` (28.1-01, D-05).
//!
//! One line per request: when, which kind, method (or `CONNECT host:port`), path
//! without query, status and a verdict phrase. Nothing else is ever written: no
//! header value, no token, no body, no resolved address. A refusal names the
//! address CLASS (`loopback`), never the address.

use std::io;
use std::path::Path;

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

/// The append-only `egress.log`, mode 0600.
pub struct EgressLog {
    file: tokio::sync::Mutex<tokio::fs::File>,
}

impl EgressLog {
    /// Open (creating it 0600 if need be) `path` for appending. Synchronous, so it
    /// runs before the server's runtime exists and no `async fn` blocks.
    pub fn open(path: &Path) -> io::Result<EgressLog> {
        use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
        let file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .mode(0o600)
            .open(path)?;
        file.set_permissions(std::fs::Permissions::from_mode(0o600))?;
        Ok(EgressLog {
            file: tokio::sync::Mutex::new(tokio::fs::File::from_std(file)),
        })
    }

    /// RED stub: the cap is not enforced yet.
    pub fn open_capped(path: &Path, _cap: u64) -> io::Result<EgressLog> {
        EgressLog::open(path)
    }

    /// Append one line. A write failure is dropped: the log never stops traffic.
    pub async fn record(&self, kind: Kind, subject: &str, path: &str, status: u16, verdict: &str) {
        use tokio::io::AsyncWriteExt;
        let timestamp = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true);
        let text = line(&timestamp, kind, subject, path, status, verdict);
        let mut file = self.file.lock().await;
        let _ = file.write_all(text.as_bytes()).await;
        let _ = file.flush().await;
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

    const MARKER: &str = "log truncated: egress.log reached its size cap";

    async fn record_n(log: &EgressLog, n: usize) {
        for _ in 0..n {
            log.record(Kind::Inference, "POST", "/v1/messages", 200, "forwarded")
                .await;
        }
    }

    /// A marker line is at most this long: six fields, the longest being the verdict.
    fn marker_line_len() -> u64 {
        line(
            "2026-10-24T20:00:00.000Z",
            Kind::Refused,
            "-",
            "-",
            0,
            MARKER,
        )
        .len() as u64
    }

    fn assert_whole_lines(text: &str) {
        assert!(
            text.is_empty() || text.ends_with('\n'),
            "partial line: {text:?}"
        );
        for l in text.lines() {
            assert_eq!(l.split('|').count(), 6, "not a whole record: {l:?}");
        }
    }

    #[tokio::test]
    async fn the_log_stops_at_its_cap_with_one_marker_line() {
        let dir = tempfile::TempDir::new().expect("tempdir");
        let path = dir.path().join("egress.log");
        let log = EgressLog::open_capped(&path, 300).expect("open");
        record_n(&log, 20).await;
        let text = std::fs::read_to_string(&path).expect("read");
        assert!(
            text.len() as u64 <= 300 + marker_line_len(),
            "{} bytes",
            text.len()
        );
        assert_eq!(text.matches("log truncated").count(), 1, "{text}");
        assert!(text.lines().last().unwrap().ends_with(MARKER), "{text}");
        assert_whole_lines(&text);
    }

    #[tokio::test]
    async fn a_full_log_reopened_by_a_later_launch_stays_full() {
        let dir = tempfile::TempDir::new().expect("tempdir");
        let path = dir.path().join("egress.log");
        let first = EgressLog::open_capped(&path, 300).expect("open");
        record_n(&first, 20).await;
        drop(first);
        let before = std::fs::metadata(&path).expect("stat").len();
        let second = EgressLog::open_capped(&path, 300).expect("reopen");
        record_n(&second, 5).await;
        assert_eq!(std::fs::metadata(&path).expect("stat").len(), before);
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
        let text = std::fs::read_to_string(&path).expect("read");
        assert!(
            text.len() as u64 <= 2000 + marker_line_len(),
            "{} bytes",
            text.len()
        );
        assert_eq!(text.matches("log truncated").count(), 1, "{text}");
        assert_whole_lines(&text);
    }
}
