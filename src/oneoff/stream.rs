//! The runtime-neutral view of a one-off's `stream.jsonl` (quick 261103-gt1).
//!
//! The agent runtime ([`super::runtime`]) classifies each raw line into a
//! [`StreamEvent`]; everything here works on those events and never on the agent's own
//! wire format: the byte-cursor tail, and (for the viewer) the renderer and the spend
//! fold. One classifier drives both the daemon (session lifecycle, soft cap) and the
//! viewer, so they cannot disagree about what a line means.

use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{self, Read, Seek, SeekFrom};
use std::path::{Path, PathBuf};

use super::inbox::{InjectionKind, InjectionOutcome, InjectionRecord};
use super::runtime::AgentRuntime;

/// Token counts of one assistant message, as the stream reports them.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct UsageTokens {
    pub input: u64,
    pub output: u64,
    pub cache_creation: u64,
    /// The 1-hour part of `cache_creation` (0 when the stream does not split it).
    pub cache_creation_1h: u64,
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
        /// Turns the agent has queued behind this one (0 when the line does not say).
        queued_turns: u32,
        /// Background subagents the result's own statistics still count as running
        /// (0 when the line does not say).
        subagents_outstanding: u32,
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
        /// The characters of this line's text blocks plus the compact-JSON length of
        /// its tool inputs: the visible part of the message's output.
        content_chars: u64,
    },
    System {
        subtype: String,
    },
    /// The set of outstanding background tasks changed; `count` is its new size.
    BackgroundTasks {
        count: u32,
    },
    /// A background task started.
    TaskStarted {
        task_id: String,
    },
    /// A background task reached a terminal status.
    TaskEnded {
        task_id: String,
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

/// The most per-message usage entries the estimate keeps (and the most message ids it
/// remembers as already folded); past it they fold into the baseline.
const MAX_TRACKED_MESSAGES: usize = 10_000;

/// One assistant message as the estimate holds it until a result folds it in.
#[derive(Debug, Clone)]
struct PendingMessage {
    model: String,
    tokens: UsageTokens,
    /// Visible characters seen over all the lines of this message.
    chars: u64,
    cost: f64,
}

/// A running estimate of a session's spend, in notional dollars (C-15, I-9, mnr D-06).
///
/// The last `total_cost_usd` the agent reported (cumulative for the whole session and
/// including subagents), plus the notional cost of the assistant `usage` seen since,
/// deduplicated by message id (a message repeats its usage once per content block).
/// Subagent messages are forwarded in the stream and folded like any other.
///
/// Streamed `output_tokens` are a message-START snapshot, and thinking text is not
/// visible, so the live figure undercounts: each message's output is estimated as the
/// larger of the reported count and the visible characters / 4. It is therefore a LOWER
/// BOUND until a `result` snaps it to the reported cumulative total
/// ([`is_exact`](Self::is_exact)). A message id already folded into a result's baseline
/// is ignored if it repeats. One pure fold, shared by the daemon (soft cap) and the
/// viewer (meter). [`usd`](Self::usd) never decreases.
#[derive(Debug, Clone, Default)]
pub struct SpendEstimate {
    baseline: f64,
    per_message: BTreeMap<String, PendingMessage>,
    /// Ids already folded into `baseline`.
    folded: BTreeSet<String>,
    high: f64,
    /// Whether the latest spend-changing event was a result with a reported cost.
    reported: bool,
}

impl SpendEstimate {
    fn pending(&self) -> f64 {
        self.per_message.values().map(|m| m.cost).sum()
    }

    /// Move every pending message into the folded set.
    fn fold_pending_ids(&mut self) {
        let ids: Vec<String> = std::mem::take(&mut self.per_message).into_keys().collect();
        self.folded.extend(ids);
        if self.folded.len() > MAX_TRACKED_MESSAGES {
            self.folded.clear();
        }
    }

    /// Fold one classified event in.
    pub fn apply(&mut self, event: &StreamEvent) {
        match event {
            StreamEvent::Usage {
                message_id,
                model,
                tokens,
                content_chars,
                ..
            } => {
                if self.folded.contains(message_id) {
                    return;
                }
                self.reported = false;
                let entry = self
                    .per_message
                    .entry(message_id.clone())
                    .or_insert_with(|| PendingMessage {
                        model: String::new(),
                        tokens: UsageTokens::default(),
                        chars: 0,
                        cost: 0.0,
                    });
                entry.chars = entry.chars.saturating_add(*content_chars);
                entry.model.clone_from(model);
                entry.tokens = *tokens;
                let mut priced = *tokens;
                priced.output = priced.output.max(entry.chars.div_ceil(4));
                entry.cost = AgentRuntime::default().notional_usd(&entry.model, &priced);
                if self.per_message.len() > MAX_TRACKED_MESSAGES {
                    self.baseline += self.pending();
                    self.fold_pending_ids();
                }
            }
            StreamEvent::TurnResult { cost_usd, .. } => {
                match cost_usd {
                    Some(cost) => self.baseline = *cost,
                    None => self.baseline += self.pending(),
                }
                self.reported = cost_usd.is_some();
                self.fold_pending_ids();
            }
            _ => {}
        }
        self.high = self.high.max(self.baseline + self.pending());
    }

    /// The estimate, in notional dollars.
    pub fn usd(&self) -> f64 {
        self.high
    }

    /// Whether the figure is a result's reported cumulative total, not an estimate: the
    /// latest spend-changing event was a result with a reported cost and the high-water
    /// mark equals it.
    pub fn is_exact(&self) -> bool {
        self.reported && (self.high - self.baseline).abs() < 1e-9
    }
}

/// The latest rate-limit status the stream reported.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct RateLimitSeen {
    pub status: String,
    pub kind: Option<String>,
    pub utilization: Option<f64>,
    pub resets_at: Option<i64>,
}

/// What the viewer's header shows besides the run's state.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct Meter {
    pub estimate: f64,
    /// Whether `estimate` is a result's reported total, not a live lower bound.
    pub exact: bool,
    pub rate: Option<RateLimitSeen>,
}

/// `resets_at` (unix seconds) as local `HH:MM`.
fn local_time(unix: i64) -> Option<String> {
    use chrono::TimeZone;
    chrono::Local
        .timestamp_opt(unix, 0)
        .single()
        .map(|t| t.format("%H:%M").to_string())
}

/// The rate-limit status as one phrase: `allowed_warning five_hour 85% resets 14:05`.
pub fn rate_text(rate: &RateLimitSeen) -> String {
    let mut text = rate.status.clone();
    if let Some(kind) = &rate.kind {
        text.push(' ');
        text.push_str(kind);
    }
    if let Some(u) = rate.utilization {
        let percent = if u <= 1.0 { u * 100.0 } else { u };
        text.push_str(&format!(" {percent:.0}%"));
    }
    if let Some(at) = rate.resets_at.and_then(local_time) {
        text.push_str(&format!(" resets {at}"));
    }
    text
}

/// An RFC 3339 time as local `HH:MM`.
fn local_hhmm(rfc3339: &str) -> Option<String> {
    chrono::DateTime::parse_from_rfc3339(rfc3339)
        .ok()
        .map(|t| t.with_timezone(&chrono::Local).format("%H:%M").to_string())
}

/// A soft cap as text: `$5.00` or `$5.00 (hit 14:05)`.
pub fn soft_cap_text(usd: f64, hit_at: Option<&str>) -> String {
    let mut text = format!("${usd:.2}");
    if let Some(at) = hit_at.and_then(local_hhmm) {
        text.push_str(&format!(" (hit {at})"));
    }
    text
}

/// The meter line: `spend ≥ $1.23 (lower bound) / soft $5.00 (hit 14:05) / hard $10.00
/// (notional) | rate: ...`. `soft` is `off` without a cap. `lower_bound` says the figure
/// is a live estimate that undercounts (`spend ≥ $X (lower bound)`); false renders the
/// exact `spend $X`.
pub fn meter_text(
    meter: &Meter,
    hard_usd: f64,
    soft_usd: Option<f64>,
    soft_hit_at: Option<&str>,
    lower_bound: bool,
) -> String {
    let soft = match soft_usd {
        Some(usd) => format!("soft {}", soft_cap_text(usd, soft_hit_at)),
        None => "soft off".to_owned(),
    };
    let spend = if lower_bound {
        format!("spend ≥ ${:.2} (lower bound)", meter.estimate)
    } else {
        format!("spend ${:.2}", meter.estimate)
    };
    let mut text = format!("{spend} / {soft} / hard ${hard_usd:.2} (notional)");
    if let Some(rate) = &meter.rate {
        text.push_str(" | rate: ");
        text.push_str(&rate_text(rate));
    }
    text
}

/// The most characters of one rendered line (mirrors `DRIVER_OUTPUT_LINE_CELLS`).
pub const RENDER_LINE_CELLS: usize = 512;
/// The most lines one event renders (mirrors `DRIVER_OUTPUT_RECORD_MAX_LINES`).
pub const RENDER_EVENT_MAX_LINES: usize = 64;
/// The most characters of a tool result's first line.
const RESULT_FIRST_LINE_CELLS: usize = 120;

fn cut(text: &str, max: usize) -> String {
    let flat: String = text
        .chars()
        .map(|c| if c == '\t' { ' ' } else { c })
        .collect();
    if flat.chars().count() <= max {
        flat
    } else {
        let mut out: String = flat.chars().take(max - 1).collect();
        out.push('…');
        out
    }
}

/// Turns events into plain display lines, ASCII-marked (C-23). The lines are
/// agent-influenced: the screen draws them through `render_for_terminal`.
#[derive(Debug, Clone, Default)]
pub struct Renderer {
    replays_seen: usize,
}

impl Renderer {
    /// A block of `text` as bounded lines, `first` before its first line and `rest`
    /// before the others.
    fn block(prefix: &str, first: &str, text: &str, suffix_first: &str) -> Vec<String> {
        let all: Vec<&str> = text.lines().collect();
        let mut out = Vec::new();
        for (i, line) in all.iter().take(RENDER_EVENT_MAX_LINES).enumerate() {
            let lead = if i == 0 { first } else { "" };
            let tail = if i == 0 { suffix_first } else { "" };
            out.push(cut(
                &format!("{prefix}{lead}{line}{tail}"),
                RENDER_LINE_CELLS,
            ));
        }
        if all.len() > RENDER_EVENT_MAX_LINES {
            out.push(format!(
                "{prefix}… ({} more lines)",
                all.len() - RENDER_EVENT_MAX_LINES
            ));
        }
        if out.is_empty() {
            out.push(cut(
                &format!("{prefix}{first}{suffix_first}"),
                RENDER_LINE_CELLS,
            ));
        }
        out
    }

    /// The lines for one event (none for usage, system and unknown events).
    pub fn render(&mut self, event: &StreamEvent) -> Vec<String> {
        let sub_prefix = |sub: &bool| if *sub { "[sub] " } else { "" };
        match event {
            StreamEvent::AssistantText { sub, text } => {
                Self::block(sub_prefix(sub), "", text.trim_end(), "")
            }
            StreamEvent::ToolCall { sub, name, summary } => {
                let body = if summary.is_empty() {
                    format!("* {name}")
                } else {
                    format!("* {name}: {summary}")
                };
                vec![cut(
                    &format!("{}{body}", sub_prefix(sub)),
                    RENDER_LINE_CELLS,
                )]
            }
            StreamEvent::ToolResult {
                sub,
                is_error,
                first_line,
                lines,
            } => {
                let marker = if *is_error { "  ! error: " } else { "  = " };
                vec![cut(
                    &format!(
                        "{}{marker}{} ({lines} lines)",
                        sub_prefix(sub),
                        cut(first_line, RESULT_FIRST_LINE_CELLS)
                    ),
                    RENDER_LINE_CELLS,
                )]
            }
            StreamEvent::UserReplay { text } => {
                let launcher = AgentRuntime::default().wrap_up_message();
                if text.trim() == launcher {
                    return Self::block("", "launcher: ", text, "");
                }
                self.replays_seen += 1;
                if self.replays_seen == 1 {
                    let first = text.lines().next().unwrap_or("");
                    vec![cut(
                        &format!("prompt: {first} ({} lines)", text.lines().count().max(1)),
                        RENDER_LINE_CELLS,
                    )]
                } else {
                    Self::block("", "you: ", text.trim_end(), "")
                }
            }
            StreamEvent::TurnResult {
                subtype,
                cost_usd,
                turns,
                ..
            } => {
                let mut text = format!("-- turn done: {subtype}");
                if let Some(cost) = cost_usd {
                    text.push_str(&format!(", ${cost:.2}"));
                }
                if let Some(turns) = turns {
                    text.push_str(&format!(", {turns} turns"));
                }
                text.push_str(" --");
                vec![text]
            }
            StreamEvent::RateLimit { .. }
            | StreamEvent::Usage { .. }
            | StreamEvent::System { .. }
            | StreamEvent::BackgroundTasks { .. }
            | StreamEvent::TaskStarted { .. }
            | StreamEvent::TaskEnded { .. }
            | StreamEvent::Other => Vec::new(),
        }
    }

    /// A refused injection as one line, so the refusal is visible where the operator
    /// typed (I-10). Delivered ones show as their replay.
    pub fn render_injection(&self, record: &InjectionRecord) -> Vec<String> {
        if record.outcome != InjectionOutcome::Refused {
            return Vec::new();
        }
        let kind = match record.kind {
            InjectionKind::Message => "message",
            InjectionKind::SoftCap => "soft_cap",
            InjectionKind::WrapUp => "wrap_up",
            InjectionKind::Pause => "pause",
        };
        let reason = record.reason.as_deref().unwrap_or("no reason given");
        vec![cut(
            &format!("! refused {kind}: {reason}"),
            RENDER_LINE_CELLS,
        )]
    }
}

/// What one [`Follower::poll`] found.
#[derive(Debug, Clone, PartialEq, Default)]
pub struct FollowBatch {
    pub lines: Vec<String>,
    pub meter: Meter,
}

/// Tails a run's `stream.jsonl` and `injections.jsonl` from offset 0, classifies each
/// line through the runtime seam, renders it and folds the spend estimate. The viewer's
/// background loop and `task attach` share it.
#[derive(Debug)]
pub struct Follower {
    stream: StreamTail,
    injections: StreamTail,
    renderer: Renderer,
    estimate: SpendEstimate,
    rate: Option<RateLimitSeen>,
}

impl Follower {
    pub fn new(run_dir: &Path) -> Self {
        Follower {
            stream: StreamTail::new(&run_dir.join(super::STREAM_FILE)),
            injections: StreamTail::new(&run_dir.join(super::INJECTIONS_FILE)),
            renderer: Renderer::default(),
            estimate: SpendEstimate::default(),
            rate: None,
        }
    }

    /// The new rendered lines and the meter as it stands.
    pub fn poll(&mut self) -> io::Result<FollowBatch> {
        let runtime = AgentRuntime::default();
        let mut lines = Vec::new();
        for raw in self.stream.poll()? {
            for event in runtime.classify(&raw) {
                self.estimate.apply(&event);
                if let StreamEvent::RateLimit {
                    status,
                    kind,
                    utilization,
                    resets_at,
                } = &event
                {
                    self.rate = Some(RateLimitSeen {
                        status: status.clone(),
                        kind: kind.clone(),
                        utilization: *utilization,
                        resets_at: *resets_at,
                    });
                }
                lines.extend(self.renderer.render(&event));
            }
        }
        for raw in self.injections.poll().unwrap_or_default() {
            if let Ok(record) = serde_json::from_str::<InjectionRecord>(&raw) {
                lines.extend(self.renderer.render_injection(&record));
            }
        }
        Ok(FollowBatch {
            lines,
            meter: Meter {
                estimate: self.estimate.usd(),
                exact: self.estimate.is_exact(),
                rate: self.rate.clone(),
            },
        })
    }
}

/// The spend estimate of a whole stream file: the same tail, classifier and
/// [`SpendEstimate`] fold the daemon and the viewer use (quick 261103-hx7, I-17). A
/// missing or unreadable file is `0.0`. Used for a run recorded before `spend_usd` was
/// kept, and after a session ends (the capture pump writes the last lines late).
pub fn spend_of_file(path: &Path) -> f64 {
    meter_of_file(path).estimate
}

/// The whole-file fold of [`spend_of_file`], plus whether the figure is exact and the
/// latest rate-limit event (quick 261103-mnr, D-05). A missing or unreadable file is the
/// default meter.
pub fn meter_of_file(path: &Path) -> Meter {
    let runtime = AgentRuntime::default();
    let mut tail = StreamTail::new(path);
    let mut estimate = SpendEstimate::default();
    let mut rate = None;
    // `poll` reads a bounded batch; keep going until the file is drained.
    while let Ok(lines) = tail.poll() {
        if lines.is_empty() {
            break;
        }
        for line in lines {
            for event in runtime.classify(&line) {
                estimate.apply(&event);
                if let StreamEvent::RateLimit {
                    status,
                    kind,
                    utilization,
                    resets_at,
                } = &event
                {
                    rate = Some(RateLimitSeen {
                        status: status.clone(),
                        kind: kind.clone(),
                        utilization: *utilization,
                        resets_at: *resets_at,
                    });
                }
            }
        }
    }
    Meter {
        estimate: estimate.usd(),
        exact: estimate.is_exact(),
        rate,
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

    /// A model id with a price row, spelled so this neutral file never names the agent.
    fn opus() -> String {
        ["cla", "ude-opus-5-5"].concat()
    }

    fn usage(id: &str, model: &str, input: u64, output: u64) -> StreamEvent {
        StreamEvent::Usage {
            sub: false,
            message_id: id.into(),
            model: model.into(),
            tokens: UsageTokens {
                input,
                output,
                ..UsageTokens::default()
            },
            content_chars: 0,
        }
    }

    fn usage_with_chars(id: &str, model: &str, output: u64, chars: u64) -> StreamEvent {
        StreamEvent::Usage {
            sub: false,
            message_id: id.into(),
            model: model.into(),
            tokens: UsageTokens {
                output,
                ..UsageTokens::default()
            },
            content_chars: chars,
        }
    }

    fn result(cost: Option<f64>) -> StreamEvent {
        StreamEvent::TurnResult {
            subtype: "success".into(),
            cost_usd: cost,
            turns: Some(3),
            is_error: false,
            queued_turns: 0,
            subagents_outstanding: 0,
        }
    }

    #[test]
    fn a_message_counts_once_however_often_its_usage_repeats() {
        let mut e = SpendEstimate::default();
        for _ in 0..3 {
            e.apply(&usage("m1", &opus(), 1_000_000, 0));
        }
        assert!((e.usd() - 4.0).abs() < 1e-9, "{}", e.usd());
        e.apply(&usage("m1", &opus(), 1_000_000, 1_000_000));
        assert!(
            (e.usd() - 24.0).abs() < 1e-9,
            "replaced, not added: {}",
            e.usd()
        );
    }

    #[test]
    fn a_reported_cost_is_the_baseline_and_usage_adds_on_top() {
        let mut e = SpendEstimate::default();
        e.apply(&usage("m1", &opus(), 1_000_000, 0));
        e.apply(&result(Some(5.0)));
        assert!((e.usd() - 5.0).abs() < 1e-9);
        e.apply(&usage("m2", &opus(), 1_000_000, 0));
        assert!((e.usd() - 9.0).abs() < 1e-9, "{}", e.usd());
        // A result without a cost folds the pending usage into the baseline.
        e.apply(&result(None));
        assert!((e.usd() - 9.0).abs() < 1e-9);
        // m2 is folded now: a repeat is ignored; m3 is a new message.
        e.apply(&usage("m2", &opus(), 1_000_000, 0));
        assert!((e.usd() - 9.0).abs() < 1e-9, "a folded id is ignored");
        e.apply(&usage("m3", &opus(), 1_000_000, 0));
        assert!(
            (e.usd() - 13.0).abs() < 1e-9,
            "m3 is a new message after a result"
        );
    }

    #[test]
    fn the_estimate_never_decreases() {
        let mut e = SpendEstimate::default();
        let mut last = 0.0;
        for event in [
            usage("m1", &opus(), 2_000_000, 0),
            result(Some(1.0)),
            usage("m2", &opus(), 10, 0),
            result(Some(1.0)),
        ] {
            e.apply(&event);
            assert!(e.usd() >= last, "{} < {last}", e.usd());
            last = e.usd();
        }
    }

    #[test]
    fn the_meter_line_names_spend_and_the_rate_limit() {
        let reset = 1_800_003_600;
        let meter = Meter {
            estimate: 1.234,
            exact: false,
            rate: Some(RateLimitSeen {
                status: "allowed_warning".into(),
                kind: Some("five_hour".into()),
                utilization: Some(0.85),
                resets_at: Some(reset),
            }),
        };
        let text = meter_text(&meter, 10.0, None, None, true);
        assert!(
            text.contains("spend ≥ $1.23 (lower bound) / soft off / hard $10.00 (notional)"),
            "{text}"
        );
        let exact = meter_text(&meter, 10.0, None, None, false);
        assert!(
            exact.contains("spend $1.23 / soft off / hard $10.00 (notional)"),
            "{exact}"
        );
        assert!(!exact.contains("lower bound"), "{exact}");
        for part in ["allowed_warning", "five_hour", "85%"] {
            assert!(text.contains(part), "{part} in {text}");
        }
        assert!(text.contains(&local_time(reset).unwrap()), "{text}");
        assert!(!meter_text(&Meter::default(), 10.0, None, None, true).contains("rate:"));
        let capped = meter_text(&meter, 10.0, Some(5.0), Some("2026-11-03T10:00:00Z"), true);
        assert!(capped.contains("soft $5.00 (hit "), "{capped}");
        assert!(!meter_text(&meter, 10.0, Some(5.0), None, true).contains("(hit"));
    }

    fn rendered(events: &[StreamEvent]) -> Vec<String> {
        let mut r = Renderer::default();
        events.iter().flat_map(|e| r.render(e)).collect()
    }

    #[test]
    fn events_render_as_ascii_marked_lines() {
        let lines = rendered(&[
            StreamEvent::UserReplay {
                text: "first\nsecond".into(),
            },
            StreamEvent::UserReplay {
                text: "please stop".into(),
            },
            StreamEvent::AssistantText {
                sub: true,
                text: "thinking out loud".into(),
            },
            StreamEvent::ToolCall {
                sub: false,
                name: "Bash".into(),
                summary: "cargo test".into(),
            },
            StreamEvent::ToolResult {
                sub: false,
                is_error: false,
                first_line: "ok".into(),
                lines: 7,
            },
            StreamEvent::ToolResult {
                sub: false,
                is_error: true,
                first_line: "boom".into(),
                lines: 1,
            },
            result(Some(0.42)),
            usage("m", "x", 1, 1),
            StreamEvent::Other,
        ]);
        assert_eq!(
            lines,
            [
                "prompt: first (2 lines)",
                "you: please stop",
                "[sub] thinking out loud",
                "* Bash: cargo test",
                "  = ok (7 lines)",
                "  ! error: boom (1 lines)",
                "-- turn done: success, $0.42, 3 turns --",
            ]
        );
    }

    #[test]
    fn a_long_event_is_bounded_in_lines_and_cells() {
        let text = (0..100)
            .map(|i| format!("l{i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let lines = rendered(&[StreamEvent::AssistantText { sub: false, text }]);
        assert_eq!(lines.len(), RENDER_EVENT_MAX_LINES + 1);
        assert_eq!(lines.last().unwrap(), "… (36 more lines)");
        let wide = rendered(&[StreamEvent::AssistantText {
            sub: false,
            text: "x".repeat(2000),
        }]);
        assert_eq!(wide[0].chars().count(), RENDER_LINE_CELLS);
        assert!(wide[0].ends_with('…'));
    }

    #[test]
    fn only_a_refused_injection_renders() {
        let mut record = InjectionRecord {
            at: "t".into(),
            id: "1-1".into(),
            kind: InjectionKind::Message,
            text: Some("x".into()),
            usd: None,
            outcome: InjectionOutcome::Delivered,
            reason: None,
        };
        let r = Renderer::default();
        assert!(r.render_injection(&record).is_empty());
        record.outcome = InjectionOutcome::Refused;
        record.reason = Some("the session is ending".into());
        assert_eq!(
            r.render_injection(&record),
            ["! refused message: the session is ending"]
        );
    }

    #[test]
    fn the_render_bounds_mirror_the_drivers() {
        assert_eq!(
            RENDER_LINE_CELLS,
            crate::ui::screens::DRIVER_OUTPUT_LINE_CELLS
        );
        assert_eq!(
            RENDER_EVENT_MAX_LINES,
            crate::ui::screens::DRIVER_OUTPUT_RECORD_MAX_LINES
        );
    }

    #[test]
    fn a_follower_replays_from_the_start_then_yields_only_the_new_lines() {
        let dir = TempDir::new().unwrap();
        let stream = dir.path().join("stream.jsonl");
        let user = |text: &str| {
            format!(
                r#"{{"type":"user","isReplay":true,"message":{{"role":"user","content":[{{"type":"text","text":"{text}"}}]}},"parent_tool_use_id":null}}"#
            )
        };
        append(&stream, format!("{}\n", user("go")).as_bytes());
        append(
            &stream,
            br#"{"type":"rate_limit_event","rate_limit_info":{"status":"allowed","rateLimitType":"five_hour"}}
"#,
        );
        append(
            &dir.path().join("injections.jsonl"),
            br#"{"at":"t","id":"1-1","kind":"message","outcome":"refused","reason":"nope"}
"#,
        );
        let mut f = Follower::new(dir.path());
        let first = f.poll().unwrap();
        assert_eq!(
            first.lines,
            ["prompt: go (1 lines)", "! refused message: nope"]
        );
        assert_eq!(first.meter.rate.as_ref().unwrap().status, "allowed");
        append(&stream, format!("{}\n", user("more")).as_bytes());
        let second = f.poll().unwrap();
        assert_eq!(second.lines, ["you: more"]);
        assert!(f.poll().unwrap().lines.is_empty());
    }

    #[test]
    fn the_spend_of_a_file_is_the_last_reported_cost() {
        let dir = TempDir::new().unwrap();
        let path = dir.path().join("s.jsonl");
        assert_eq!(spend_of_file(&path), 0.0, "missing file");
        append(
            &path,
            b"{\"type\":\"result\",\"subtype\":\"success\",\"total_cost_usd\":1.5}\n",
        );
        assert!((spend_of_file(&path) - 1.5).abs() < 1e-9);
    }

    #[test]
    fn visible_characters_floor_the_output_estimate_and_accumulate_per_message() {
        let mut e = SpendEstimate::default();
        // 1 reported output token, 4000 visible characters: 1000 output tokens.
        e.apply(&usage_with_chars("m1", &opus(), 1, 4000));
        assert!((e.usd() - 1000.0 * 20.0 / 1e6).abs() < 1e-9, "{}", e.usd());
        // Another line of the same message adds its characters.
        e.apply(&usage_with_chars("m1", &opus(), 1, 4000));
        assert!((e.usd() - 2000.0 * 20.0 / 1e6).abs() < 1e-9, "{}", e.usd());
        // A larger reported count wins over the characters.
        e.apply(&usage_with_chars("m2", &opus(), 5000, 4));
        assert!(
            (e.usd() - (2000.0 + 5000.0) * 20.0 / 1e6).abs() < 1e-9,
            "{}",
            e.usd()
        );
    }

    #[test]
    fn a_message_already_folded_into_a_result_is_ignored() {
        let mut e = SpendEstimate::default();
        e.apply(&usage("m1", &opus(), 1_000_000, 0));
        e.apply(&result(Some(2.0)));
        assert!((e.usd() - 4.0).abs() < 1e-9, "the estimate led the report");
        let mut e = SpendEstimate::default();
        e.apply(&usage("m1", &opus(), 100, 0));
        e.apply(&result(Some(2.0)));
        assert!((e.usd() - 2.0).abs() < 1e-9);
        // The same message again, after the result: not counted a second time.
        e.apply(&usage("m1", &opus(), 1_000_000, 0));
        assert!((e.usd() - 2.0).abs() < 1e-9, "{}", e.usd());
        assert!(e.is_exact());
        // A new message does count.
        e.apply(&usage("m2", &opus(), 1_000_000, 0));
        assert!((e.usd() - 6.0).abs() < 1e-9, "{}", e.usd());
    }

    #[test]
    fn the_estimate_is_exact_only_right_after_a_reported_result() {
        let mut e = SpendEstimate::default();
        assert!(!e.is_exact(), "nothing seen");
        e.apply(&usage("m1", &opus(), 10, 0));
        assert!(!e.is_exact());
        e.apply(&result(Some(1.0)));
        assert!(e.is_exact());
        e.apply(&usage("m2", &opus(), 10, 0));
        assert!(!e.is_exact(), "later usage");
        e.apply(&result(None));
        assert!(!e.is_exact(), "a result without a cost");
        // An estimate that overshot the report is not exact either.
        let mut e = SpendEstimate::default();
        e.apply(&usage("m1", &opus(), 1_000_000, 0));
        e.apply(&result(Some(1.0)));
        assert!(!e.is_exact());
    }

    #[test]
    fn the_recorded_background_run_meters_to_its_reported_total() {
        let path = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/oneoff/real-bg-subagents.stream.jsonl");
        let meter = meter_of_file(&path);
        assert!(
            (meter.estimate - 0.86173693).abs() < 1e-6,
            "{}",
            meter.estimate
        );
        assert!(meter.exact);
        let rate = meter.rate.expect("a rate-limit event");
        assert_eq!(rate.status, "allowed");
        assert_eq!(rate.kind.as_deref(), Some("seven_day"));
        assert_eq!(rate.utilization, Some(0.21));
        assert_eq!(rate.resets_at, Some(1_794_326_400));
        assert!(rate_text(&rate).starts_with("allowed seven_day 21%"));
        assert_eq!(spend_of_file(&path), meter.estimate);
        assert_eq!(
            meter_of_file(&path.with_extension("missing")),
            Meter::default()
        );
    }
}
