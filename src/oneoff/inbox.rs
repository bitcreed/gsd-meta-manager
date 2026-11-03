//! The host-only injection inbox of a one-off run (quick 261103-gt1, C-09..C-13).
//!
//! The operator (the TUI, `task send`) writes `<run-dir>/inbox/<nanos>-<pid>.json`
//! atomically; the launcher daemon takes the files each tick, in name order, and
//! records every request in `<run-dir>/injections.jsonl` BEFORE it acts on it. The
//! inbox lives under the run-dir root, which is never mounted into the sandbox, and the
//! run dir and the inbox are 0700: only the local operator can inject, never the
//! agent. No socket: the driverctl socket is bound INTO the sandbox, so reusing it
//! would hand the agent the same lever.

use serde::{Deserialize, Serialize};

use super::MAX_MESSAGE_CHARS;

/// One request a client puts in the inbox.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum InboxRequest {
    /// A message for the running agent.
    Message { text: String },
    /// Set (`Some`) or clear (`None`) the soft spend cap.
    SoftCap { usd: Option<f64> },
}

/// What a record in `injections.jsonl` was.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InjectionKind {
    Message,
    SoftCap,
    /// The launcher's own wrap-up message at the soft cap.
    WrapUp,
}

/// Whether the launcher acted on a request.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum InjectionOutcome {
    Delivered,
    Refused,
}

/// One line of `injections.jsonl` (0600, host-written, never mounted).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct InjectionRecord {
    pub at: String,
    /// The inbox file stem the request came in as (`wrap_up` records mint their own).
    pub id: String,
    pub kind: InjectionKind,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub text: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub usd: Option<f64>,
    pub outcome: InjectionOutcome,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
}

/// The largest request file the daemon reads.
pub const MAX_REQUEST_BYTES: u64 = 16 * 1024;

/// Judge a message's text (C-12): 1..=[`MAX_MESSAGE_CHARS`] characters, no control
/// characters other than `\n` and `\t`, no invisible formatting.
pub fn validate_message(text: &str) -> Result<(), String> {
    let count = text.chars().count();
    if count == 0 {
        return Err("the message is empty".to_owned());
    }
    if count > MAX_MESSAGE_CHARS {
        return Err(format!(
            "the message is {count} characters; at most {MAX_MESSAGE_CHARS}"
        ));
    }
    if text
        .chars()
        .any(|c| c.is_control() && c != '\n' && c != '\t')
    {
        return Err("the message carries a control character".to_owned());
    }
    if crate::text::carries_invisible_formatting(text) {
        return Err("the message carries invisible formatting characters".to_owned());
    }
    Ok(())
}

/// Whether `name` is an inbox request name: `<digits>-<digits>.json`.
pub fn valid_inbox_name(name: &str) -> bool {
    let Some(stem) = name.strip_suffix(".json") else {
        return false;
    };
    let Some((nanos, pid)) = stem.split_once('-') else {
        return false;
    };
    let digits = |s: &str, max: usize| {
        !s.is_empty() && s.len() <= max && s.bytes().all(|b| b.is_ascii_digit())
    };
    digits(nanos, 20) && digits(pid, 10)
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub use self::fs_inbox::*;

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod fs_inbox {
    use std::fmt;
    use std::fs;
    use std::io::Write;
    use std::os::unix::fs::{DirBuilderExt, MetadataExt, OpenOptionsExt};
    use std::path::Path;
    use std::sync::atomic::{AtomicU64, Ordering};
    use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

    use super::{
        valid_inbox_name, validate_message, InboxRequest, InjectionRecord, MAX_REQUEST_BYTES,
    };
    use crate::oneoff::store::{lock_path, probe, LockSeen};
    use crate::oneoff::stream::StreamTail;
    use crate::oneoff::{INBOX_DIR, INJECTIONS_FILE};
    use crate::state_reader::driver_frontmatter::read_run_file_capped;

    /// The most request files one [`take`] reads.
    const TAKE_MAX: usize = 64;

    /// Why a client could not put a request in the inbox.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub enum InboxError {
        /// The run is not live: the project lock is not held by this run id.
        NotLive,
        /// The request is not valid.
        Invalid(String),
        Io(String),
    }

    impl fmt::Display for InboxError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            match self {
                InboxError::NotLive => write!(f, "the one-off is not live"),
                InboxError::Invalid(why) | InboxError::Io(why) => write!(f, "{why}"),
            }
        }
    }

    impl std::error::Error for InboxError {}

    /// Create `<run_dir>/inbox`, 0700.
    pub fn ensure_inbox(run_dir: &Path) -> std::io::Result<()> {
        let dir = run_dir.join(INBOX_DIR);
        match fs::DirBuilder::new().mode(0o700).create(&dir) {
            Ok(()) => Ok(()),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => Ok(()),
            Err(e) => Err(e),
        }
    }

    /// Whether the run in `run_dir` holds its project's lock.
    fn is_live(run_dir: &Path) -> bool {
        let (Some(project_dir), Some(run_id)) = (
            run_dir.parent(),
            run_dir.file_name().and_then(|n| n.to_str()),
        ) else {
            return false;
        };
        probe(&lock_path(project_dir)) == LockSeen::HeldBy(run_id.to_owned())
    }

    /// A monotonic nanosecond stamp, so two requests from one process never collide.
    fn stamp() -> u128 {
        static LAST: AtomicU64 = AtomicU64::new(0);
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_or(0, |d| d.as_nanos());
        let now = u64::try_from(now).unwrap_or(u64::MAX - 1);
        let mut prev = LAST.load(Ordering::SeqCst);
        loop {
            let next = now.max(prev + 1);
            match LAST.compare_exchange(prev, next, Ordering::SeqCst, Ordering::SeqCst) {
                Ok(_) => return u128::from(next),
                Err(actual) => prev = actual,
            }
        }
    }

    /// Put `request` in the inbox atomically and return its id (the file stem).
    fn put(run_dir: &Path, request: &InboxRequest) -> Result<String, InboxError> {
        if !is_live(run_dir) {
            return Err(InboxError::NotLive);
        }
        let dir = run_dir.join(INBOX_DIR);
        let meta = fs::symlink_metadata(&dir)
            .map_err(|e| InboxError::Io(format!("the run's inbox is not ready: {e}")))?;
        if !meta.file_type().is_dir() {
            return Err(InboxError::Io("the run's inbox is not a directory".into()));
        }
        let id = format!("{}-{}", stamp(), std::process::id());
        let tmp = dir.join(format!(".tmp-{id}"));
        let target = dir.join(format!("{id}.json"));
        let body =
            serde_json::to_vec(request).map_err(|e| InboxError::Io(format!("encode: {e}")))?;
        let write = || -> std::io::Result<()> {
            let mut file = fs::OpenOptions::new()
                .write(true)
                .create_new(true)
                .mode(0o600)
                .open(&tmp)?;
            file.write_all(&body)?;
            file.sync_all()?;
            fs::rename(&tmp, &target)
        };
        write().map_err(|e| {
            let _ = fs::remove_file(&tmp);
            InboxError::Io(format!("write the request: {e}"))
        })?;
        Ok(id)
    }

    /// Send a message to the running agent of `run_dir`. Returns the request id.
    pub fn send_message(run_dir: &Path, text: &str) -> Result<String, InboxError> {
        validate_message(text).map_err(InboxError::Invalid)?;
        put(
            run_dir,
            &InboxRequest::Message {
                text: text.to_owned(),
            },
        )
    }

    /// Set or clear the soft spend cap of the run in `run_dir`. Returns the request id.
    pub fn set_soft_cap(run_dir: &Path, usd: Option<f64>) -> Result<String, InboxError> {
        put(run_dir, &InboxRequest::SoftCap { usd })
    }

    /// One request the daemon took from the inbox.
    #[derive(Debug, Clone, PartialEq)]
    pub struct Taken {
        pub id: String,
        pub request: Result<InboxRequest, String>,
    }

    /// Take the pending requests, in name order, deleting each file. A symlink, a
    /// non-regular file, a file of another uid, a file over [`MAX_REQUEST_BYTES`] and a
    /// malformed or invalid request come back refused. Names that are not request names
    /// (and `.tmp-` files, which are half-written) are left alone.
    pub fn take(run_dir: &Path) -> Vec<Taken> {
        let dir = run_dir.join(INBOX_DIR);
        let Ok(entries) = fs::read_dir(&dir) else {
            return Vec::new();
        };
        let mut names: Vec<String> = entries
            .filter_map(Result::ok)
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|n| !n.starts_with('.') && valid_inbox_name(n))
            .collect();
        names.sort();
        names.truncate(TAKE_MAX);
        let me = rustix::process::geteuid().as_raw();
        let mut taken = Vec::new();
        for name in names {
            let path = dir.join(&name);
            let id = name.trim_end_matches(".json").to_owned();
            let request = (|| {
                let meta = fs::symlink_metadata(&path).map_err(|e| format!("unreadable: {e}"))?;
                if !meta.file_type().is_file() {
                    return Err("not a regular file".to_owned());
                }
                if meta.uid() != me {
                    return Err("not owned by the launcher's user".to_owned());
                }
                if meta.len() > MAX_REQUEST_BYTES {
                    return Err(format!("larger than {MAX_REQUEST_BYTES} bytes"));
                }
                let bytes = read_run_file_capped(&path).map_err(|e| format!("unreadable: {e}"))?;
                let request: InboxRequest =
                    serde_json::from_slice(&bytes).map_err(|e| format!("malformed: {e}"))?;
                if let InboxRequest::Message { text } = &request {
                    validate_message(text)?;
                }
                Ok(request)
            })();
            if fs::remove_file(&path).is_err() {
                let _ = fs::remove_dir(&path);
            }
            taken.push(Taken { id, request });
        }
        taken
    }

    /// Append one record to `injections.jsonl` (0600, one line, flushed).
    pub fn append_injection(run_dir: &Path, record: &InjectionRecord) -> std::io::Result<()> {
        let mut line = serde_json::to_string(record)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e))?;
        line.push('\n');
        let mut file = fs::OpenOptions::new()
            .create(true)
            .append(true)
            .mode(0o600)
            .open(run_dir.join(INJECTIONS_FILE))?;
        file.write_all(line.as_bytes())?;
        file.flush()
    }

    /// Wait up to `budget` for the daemon's record of request `id`.
    pub fn wait_outcome(run_dir: &Path, id: &str, budget: Duration) -> Option<InjectionRecord> {
        let mut tail = StreamTail::new(&run_dir.join(INJECTIONS_FILE));
        let deadline = Instant::now() + budget;
        loop {
            if let Ok(lines) = tail.poll() {
                for line in lines {
                    if let Ok(record) = serde_json::from_str::<InjectionRecord>(&line) {
                        if record.id == id {
                            return Some(record);
                        }
                    }
                }
            }
            if Instant::now() >= deadline {
                return None;
            }
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use tempfile::TempDir;

        fn run_dir() -> TempDir {
            let dir = TempDir::new().unwrap();
            ensure_inbox(dir.path()).unwrap();
            dir
        }

        fn write(dir: &TempDir, name: &str, body: &str) {
            fs::write(dir.path().join(INBOX_DIR).join(name), body).unwrap();
        }

        #[test]
        fn requests_are_taken_in_name_order_and_deleted() {
            let dir = run_dir();
            write(&dir, "2-1.json", r#"{"kind":"message","text":"b"}"#);
            write(&dir, "1-1.json", r#"{"kind":"message","text":"a"}"#);
            write(&dir, "3-1.json", r#"{"kind":"soft_cap","usd":null}"#);
            write(&dir, ".tmp-4-1", "half");
            write(&dir, "junk.txt", "x");
            let taken = take(dir.path());
            let ids: Vec<&str> = taken.iter().map(|t| t.id.as_str()).collect();
            assert_eq!(ids, ["1-1", "2-1", "3-1"]);
            assert_eq!(
                taken[0].request,
                Ok(InboxRequest::Message { text: "a".into() })
            );
            assert_eq!(taken[2].request, Ok(InboxRequest::SoftCap { usd: None }));
            assert!(!dir.path().join(INBOX_DIR).join("1-1.json").exists());
            assert!(dir.path().join(INBOX_DIR).join(".tmp-4-1").exists());
            assert!(dir.path().join(INBOX_DIR).join("junk.txt").exists());
        }

        #[test]
        fn hostile_requests_are_refused_and_deleted() {
            let dir = run_dir();
            let outside = dir.path().join("outside");
            fs::write(&outside, r#"{"kind":"message","text":"x"}"#).unwrap();
            std::os::unix::fs::symlink(&outside, dir.path().join(INBOX_DIR).join("1-1.json"))
                .unwrap();
            fs::create_dir(dir.path().join(INBOX_DIR).join("2-1.json")).unwrap();
            write(&dir, "3-1.json", &"x".repeat(17 * 1024));
            write(&dir, "4-1.json", "not json");
            write(&dir, "5-1.json", r#"{"kind":"nope"}"#);
            write(
                &dir,
                "6-1.json",
                r#"{"kind":"message","text":"\u001b[31m"}"#,
            );
            let taken = take(dir.path());
            assert_eq!(taken.len(), 6);
            for t in &taken {
                assert!(t.request.is_err(), "{t:?}");
            }
            assert!(outside.exists(), "the symlink target is never touched");
            assert_eq!(fs::read_dir(dir.path().join(INBOX_DIR)).unwrap().count(), 0);
        }

        #[test]
        fn a_run_that_is_not_live_refuses_and_writes_nothing() {
            let dir = run_dir();
            assert_eq!(send_message(dir.path(), "hello"), Err(InboxError::NotLive));
            assert_eq!(fs::read_dir(dir.path().join(INBOX_DIR)).unwrap().count(), 0);
        }

        #[test]
        fn injections_append_and_are_found_by_id() {
            let dir = TempDir::new().unwrap();
            let record = InjectionRecord {
                at: "t".into(),
                id: "9-1".into(),
                kind: super::super::InjectionKind::Message,
                text: Some("hi".into()),
                usd: None,
                outcome: super::super::InjectionOutcome::Delivered,
                reason: None,
            };
            append_injection(dir.path(), &record).unwrap();
            let found = wait_outcome(dir.path(), "9-1", Duration::from_millis(200)).unwrap();
            assert_eq!(found, record);
            assert!(wait_outcome(dir.path(), "other", Duration::from_millis(60)).is_none());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn message_text_is_judged() {
        assert!(validate_message("a\n\tb").is_ok());
        assert!(validate_message("").is_err());
        assert!(validate_message(&"x".repeat(4000)).is_ok());
        assert!(validate_message(&"x".repeat(4001)).is_err());
        assert!(validate_message("a\u{1b}[31m").is_err());
        assert!(validate_message("a\u{202e}b").is_err());
    }

    #[test]
    fn inbox_names_are_judged() {
        assert!(valid_inbox_name("1700000000000000000-123.json"));
        assert!(!valid_inbox_name(".tmp-1-2"));
        assert!(!valid_inbox_name("1-2.txt"));
        assert!(!valid_inbox_name("a-2.json"));
        assert!(!valid_inbox_name("1-.json"));
        assert!(!valid_inbox_name("../1-2.json"));
    }

    #[test]
    fn requests_use_the_documented_wire_shape() {
        assert_eq!(
            serde_json::to_string(&InboxRequest::Message { text: "x".into() }).unwrap(),
            r#"{"kind":"message","text":"x"}"#
        );
        assert_eq!(
            serde_json::from_str::<InboxRequest>(r#"{"kind":"soft_cap","usd":null}"#).unwrap(),
            InboxRequest::SoftCap { usd: None }
        );
    }
}
