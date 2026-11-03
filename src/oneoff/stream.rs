//! The runtime-neutral view of a one-off's `stream.jsonl` (quick 261103-gt1).
//!
//! The agent runtime ([`super::runtime`]) classifies each raw line into a
//! [`StreamEvent`]; everything here works on those events and never on the agent's own
//! wire format: the byte-cursor tail, and (for the viewer) the renderer and the spend
//! fold. One classifier drives both the daemon (session lifecycle, soft cap) and the
//! viewer, so they cannot disagree about what a line means.

use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

/// Token counts of one assistant message, as the stream reports them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UsageTokens {
    pub input: u64,
    pub output: u64,
    pub cache_creation: u64,
    pub cache_read: u64,
}

/// One thing a stream line says, runtime-neutral. `sub` means the event came from a
/// subagent (the line carried a parent tool-use id).
#[derive(Debug, Clone, PartialEq)]
pub enum StreamEvent {
    AssistantText {
        sub: bool,
        text: String,
    },
    ToolCall {
        sub: bool,
        name: String,
        summary: String,
    },
    ToolResult {
        sub: bool,
        is_error: bool,
        first_line: String,
        lines: usize,
    },
    /// A user message the agent read back: the initial prompt or an injection.
    UserReplay {
        text: String,
    },
    /// The end of a turn.
    TurnResult {
        subtype: String,
        cost_usd: Option<f64>,
        turns: Option<u32>,
        is_error: bool,
    },
    RateLimit {
        status: String,
        kind: Option<String>,
        utilization: Option<f64>,
        resets_at: Option<i64>,
    },
    Usage {
        sub: bool,
        message_id: String,
        model: String,
        tokens: UsageTokens,
    },
    System {
        subtype: String,
    },
    Other,
}

/// The longest line the tail keeps. It equals the capture pump's bound, so the pump
/// never writes a line the tail would have to skip.
pub const MAX_EVENT_LINE: u64 = 4 * 1024 * 1024;

/// What replaces a line longer than the bound: a fixed note, never a fragment.
pub const OVERLONG_EVENT_NOTE: &str = "(a line of the stream was too long to keep)";

/// The most bytes one [`StreamTail::poll`] reads, so a huge backlog arrives in batches.
const POLL_MAX_BYTES: u64 = 8 * 1024 * 1024;

/// A byte-cursor tail of a growing line file: only complete lines are returned.
#[derive(Debug)]
pub struct StreamTail {
    path: PathBuf,
    offset: u64,
    partial: Vec<u8>,
    skipping: bool,
    max_line: u64,
}

impl StreamTail {
    /// A tail that starts at offset 0.
    pub fn new(path: &Path) -> Self {
        Self::with_max_line(path, MAX_EVENT_LINE)
    }

    pub(crate) fn with_max_line(path: &Path, max_line: u64) -> Self {
        StreamTail {
            path: path.to_owned(),
            offset: 0,
            partial: Vec::new(),
            skipping: false,
            max_line,
        }
    }

    /// The complete lines written since the last poll (lossy UTF-8, without the
    /// newline). A missing file is empty, not an error. At most [`POLL_MAX_BYTES`]
    /// are read per call; call again when the batch was not empty.
    pub fn poll(&mut self) -> io::Result<Vec<String>> {
        let mut file = match File::open(&self.path) {
            Ok(file) => file,
            Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
            Err(e) => return Err(e),
        };
        let len = file.metadata()?.len();
        if len < self.offset {
            // The file was replaced by a shorter one: start over.
            self.offset = 0;
            self.partial.clear();
            self.skipping = false;
        }
        file.seek(SeekFrom::Start(self.offset))?;
        let mut budget = POLL_MAX_BYTES.min(len - self.offset);
        let mut lines = Vec::new();
        let mut chunk = vec![0_u8; 64 * 1024];
        while budget > 0 {
            let want = usize::try_from(budget.min(chunk.len() as u64)).unwrap_or(chunk.len());
            let read = file.read(&mut chunk[..want])?;
            if read == 0 {
                break;
            }
            budget -= read as u64;
            self.offset += read as u64;
            self.feed(&chunk[..read], &mut lines);
        }
        Ok(lines)
    }

    fn feed(&mut self, bytes: &[u8], lines: &mut Vec<String>) {
        let mut rest = bytes;
        while !rest.is_empty() {
            match rest.iter().position(|b| *b == b'\n') {
                Some(i) => {
                    let (head, tail) = rest.split_at(i);
                    rest = &tail[1..];
                    if self.skipping {
                        self.skipping = false;
                        continue;
                    }
                    self.partial.extend_from_slice(head);
                    if self.partial.len() as u64 > self.max_line {
                        lines.push(OVERLONG_EVENT_NOTE.to_owned());
                    } else {
                        lines.push(String::from_utf8_lossy(&self.partial).into_owned());
                    }
                    self.partial.clear();
                }
                None => {
                    if !self.skipping {
                        self.partial.extend_from_slice(rest);
                        if self.partial.len() as u64 > self.max_line {
                            lines.push(OVERLONG_EVENT_NOTE.to_owned());
                            self.partial.clear();
                            self.skipping = true;
                        }
                    }
                    rest = &[];
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;
    use tempfile::TempDir;

    fn append(path: &Path, bytes: &[u8]) {
        let mut f = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)
            .unwrap();
        f.write_all(bytes).unwrap();
    }

    #[test]
    fn a_partial_line_waits_for_its_newline() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("s.jsonl");
        let mut tail = StreamTail::new(&path);
        assert_eq!(tail.poll().unwrap(), Vec::<String>::new(), "missing file");
        append(&path, b"one\ntwo\nthr");
        assert_eq!(tail.poll().unwrap(), ["one", "two"]);
        assert!(tail.poll().unwrap().is_empty());
        append(&path, b"ee\n");
        assert_eq!(tail.poll().unwrap(), ["three"]);
    }

    #[test]
    fn an_overlong_line_is_one_note_and_the_cursor_resumes_after_it() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("s.jsonl");
        let mut tail = StreamTail::with_max_line(&path, 8);
        append(&path, b"ok\n0123456789012345678");
        assert_eq!(tail.poll().unwrap(), ["ok", OVERLONG_EVENT_NOTE]);
        append(&path, b"more garbage");
        assert!(tail.poll().unwrap().is_empty());
        append(&path, b"\nnext\n");
        assert_eq!(tail.poll().unwrap(), ["next"]);
        // A whole overlong line arriving at once is also one note.
        append(&path, b"0123456789\nafter\n");
        assert_eq!(tail.poll().unwrap(), [OVERLONG_EVENT_NOTE, "after"]);
    }

    #[cfg(unix)]
    #[test]
    fn the_line_bound_equals_the_capture_pumps() {
        assert_eq!(MAX_EVENT_LINE, crate::launcher::process::MAX_STREAM_LINE);
    }
}
