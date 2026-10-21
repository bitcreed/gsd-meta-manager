//! Pure readers (and, in later plans, transforms) over a `DIRECTION.md` **body**.
//!
//! The Driver tab shows each item with the fields only the human's body carries: its
//! id, its title, its `hold` flag and its `answer` (D-06). Everything else about an
//! item (state, dispatches, outcome) is tool-written frontmatter and never read here.
//!
//! This module shares the readers of [`crate::driver::direction`] instead of copying
//! them (WR-01): comments are blanked by [`blank_comments`], sections come from
//! [`section_heading`] and `### I-NN` headings from the one [`item_re`]. A commented-
//! out item, or one outside `## Items`, is therefore invisible here exactly as it is
//! to the store's parser and to the `driver check` lint.
//!
//! The same module also holds the **transforms** the Driver tab's steering keys apply
//! to a body ([`set_control`], and in the next task `answer` and `unhold`) and the one
//! blocking helper that writes them, [`steer_run`]. Every transform is a pure
//! `&str -> Result<String, SteerError>`: no file, no clock, no platform facility, so
//! they compile and are tested everywhere. [`steer_run`] feeds them to
//! [`write_body`], which is Linux-only and reports `UnsupportedPlatform` elsewhere
//! (the App refuses before it ever gets there).
//!
//! **Every write the tab makes to `DIRECTION.md` is a [`write_body`] closure** (D-14):
//! the shared `DIRECTION.lock`, the exchange-verify loop, and the `---` tool block
//! kept byte-for-byte. `write_body` may run its closure more than once on a fresh body
//! when a human save raced the write, so every transform here is idempotent and a
//! closure never carries state from one call to the next.

use std::path::Path;

use crate::driver::direction::{
    blank_comments, body_sane, control_re, item_re, parse_direction, section_heading, write_body,
    Section, SpliceOutcome,
};
use crate::driver_templates::lint::lint_direction;

/// One `### I-NN` item as the human wrote it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BodyItem {
    /// The item id, e.g. `I-01`.
    pub id: String,
    /// The heading text after the id, trimmed, with any trailing HTML comment
    /// removed. Untrusted display text: sanitise before drawing.
    pub title: String,
    /// The value of the item's first uncommented `- hold:` line, trimmed; `None`
    /// when the line is absent or empty.
    pub hold: Option<String>,
    /// The value of the item's first uncommented `- answer:` line, trimmed; `None`
    /// when the line is absent or empty.
    pub answer: Option<String>,
}

impl BodyItem {
    /// Whether the item is on hold: `- hold: true`, in any letter case.
    pub fn is_on_hold(&self) -> bool {
        self.hold
            .as_deref()
            .is_some_and(|word| word.eq_ignore_ascii_case("true"))
    }
}

/// The value of a `- <key>: <value>` line, or `None` when `line` is not one for
/// `key`. The key matches in any letter case and the line may be indented, the same
/// looseness `parse_direction` allows the `control` line.
fn key_value<'a>(line: &'a str, key: &str) -> Option<&'a str> {
    let rest = line.trim_start().strip_prefix('-')?;
    let rest = rest.strip_prefix([' ', '\t'])?.trim_start();
    let (name, value) = rest.split_once(':')?;
    name.trim().eq_ignore_ascii_case(key).then(|| value.trim())
}

/// The items of a DIRECTION body in document order.
///
/// Comments are blanked first, `### I-NN` headings count only under `## Items`, and
/// an item's segment runs to the next `### ` or `## ` heading. `hold` and `answer`
/// are the first uncommented line of each in the segment. The title is read from the
/// original text at the heading's own byte offsets (the blanked copy has the same
/// length and char boundaries), minus a trailing comment.
pub fn read_items(body: &str) -> Vec<BodyItem> {
    let blanked = blank_comments(body);
    let mut items: Vec<BodyItem> = Vec::new();
    let mut section = Section::Preamble;
    // True while the lines being walked belong to the item last pushed.
    let mut in_item = false;
    let mut offset = 0usize;
    for raw in blanked.split_inclusive('\n') {
        let line_start = offset;
        offset += raw.len();
        let line = raw.trim_end_matches(['\n', '\r']);
        if let Some(next) = section_heading(line) {
            section = next;
            in_item = false;
            continue;
        }
        if section != Section::Items {
            continue;
        }
        if line.starts_with("### ") {
            in_item = false;
            if let Some(caps) = item_re().captures(line) {
                let (Some(whole), Some(id)) = (caps.get(0), caps.get(1)) else {
                    continue;
                };
                let visible = line[whole.end()..].trim_end().len();
                let title_start = line_start + whole.end();
                let title = body
                    .get(title_start..title_start + visible)
                    .unwrap_or_default()
                    .trim()
                    .to_string();
                items.push(BodyItem {
                    id: id.as_str().to_string(),
                    title,
                    hold: None,
                    answer: None,
                });
                in_item = true;
            }
            continue;
        }
        if !in_item {
            continue;
        }
        let Some(item) = items.last_mut() else {
            continue;
        };
        if item.hold.is_none() {
            if let Some(value) = key_value(line, "hold") {
                item.hold = Some(value).filter(|v| !v.is_empty()).map(str::to_string);
            }
        }
        if item.answer.is_none() {
            if let Some(value) = key_value(line, "answer") {
                item.answer = Some(value).filter(|v| !v.is_empty()).map(str::to_string);
            }
        }
    }
    items
}

// ── Steering transforms (DTAB-03, D-15) ─────────────────────────────────────

/// The three words `- control:` may hold, in the order the chooser lists them.
pub const CONTROL_WORDS: [&str; 3] = ["run", "pause", "stop"];

/// One edit the Driver tab's steering keys ask for. Plain data, so it travels inside
/// an `Action`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SteerOp {
    /// Set every uncommented `- control:` line to this word (`run`, `pause`, `stop`).
    SetControl(String),
    /// Answer the item: set its `- answer:` to `text` and its `- hold:` to `false`
    /// in the same write (D-15).
    Answer { item: String, text: String },
    /// Take the item off hold: its `- hold:` becomes `false`.
    Unhold { item: String },
}

/// Why a transform refused. A refusal writes nothing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SteerError {
    /// The control word is not one of [`CONTROL_WORDS`].
    BadControlWord(String),
    /// The body has no `## Direction` heading to put a `- control:` line under.
    NoDirectionSection,
    /// The item id is not a `### I-NN` heading under `## Items` (or is commented out).
    NoSuchItem(String),
    /// The answer cannot be written as one line of the body.
    AnswerRejected(AnswerRule),
    /// The transform produced a body that does not say what was asked, or whose byte
    /// offsets could not be trusted; the old body stands.
    PostCondition(String),
}

impl std::fmt::Display for SteerError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SteerError::BadControlWord(word) => {
                write!(f, "\"{word}\" is not a control word (run, pause or stop)")
            }
            SteerError::NoDirectionSection => {
                write!(
                    f,
                    "the body has no \"## Direction\" heading to put it under"
                )
            }
            SteerError::NoSuchItem(id) => {
                write!(f, "{id} is not an item under \"## Items\" in DIRECTION.md")
            }
            SteerError::AnswerRejected(rule) => write!(f, "{rule}"),
            SteerError::PostCondition(what) => {
                write!(f, "the edit would not have said what was asked ({what})")
            }
        }
    }
}

impl std::error::Error for SteerError {}

/// `body` with each `(start, end, replacement)` range replaced, in order. `None` when
/// a range is out of order or off a char boundary: offsets read from the comment-
/// blanked copy are the body's own (the blanking keeps byte offsets), but a wrong
/// one must refuse the edit rather than panic a blocking task.
fn splice_ranges(body: &str, edits: &[(usize, usize, &str)]) -> Option<String> {
    let mut out = String::with_capacity(body.len() + 32);
    let mut last = 0usize;
    for &(start, end, replacement) in edits {
        if start < last || end < start {
            return None;
        }
        out.push_str(body.get(last..start)?);
        out.push_str(replacement);
        last = end;
    }
    out.push_str(body.get(last..)?);
    Some(out)
}

/// The line terminator `raw` (one `split_inclusive('\n')` piece) ends with.
fn eol_of(raw: &str) -> &'static str {
    if raw.ends_with("\r\n") {
        "\r\n"
    } else {
        "\n"
    }
}

/// Set the human's control word to `word` (`run`, `pause` or `stop`).
///
/// Every uncommented `- control:` line is rewritten at the byte offsets of its word
/// (the rest of each line, its indentation and a trailing comment included, is kept).
/// With none, `- control: {word}` is inserted directly after the first `## Direction`
/// heading. The post-condition is the store's own reading of the new body:
/// `parse_direction(&new).control == Some(word)`, which also proves that a second
/// `- control:` line cannot hold the run (`parse_direction` lets `pause`/`stop` win).
/// Idempotent: `set_control(set_control(b, w), w) == set_control(b, w)`.
pub fn set_control(body: &str, word: &str) -> Result<String, SteerError> {
    if !CONTROL_WORDS.contains(&word) {
        return Err(SteerError::BadControlWord(word.to_string()));
    }
    let blanked = blank_comments(body);
    let mut edits: Vec<(usize, usize, &str)> = control_re()
        .captures_iter(&blanked)
        .filter_map(|caps| caps.get(1))
        .map(|m| (m.start(), m.end(), word))
        .collect();
    let insertion;
    if edits.is_empty() {
        let mut offset = 0usize;
        let mut at: Option<(usize, &'static str, bool)> = None;
        for raw in blanked.split_inclusive('\n') {
            offset += raw.len();
            if section_heading(raw.trim_end_matches(['\n', '\r'])) == Some(Section::Direction) {
                at = Some((offset, eol_of(raw), raw.ends_with('\n')));
                break;
            }
        }
        let Some((end, eol, terminated)) = at else {
            return Err(SteerError::NoDirectionSection);
        };
        // A heading on the very last, unterminated line needs a newline of its own.
        insertion = if terminated {
            format!("- control: {word}{eol}")
        } else {
            format!("{eol}- control: {word}{eol}")
        };
        edits.push((end, end, insertion.as_str()));
    }
    let new = splice_ranges(body, &edits)
        .ok_or_else(|| SteerError::PostCondition("offsets".to_string()))?;
    if parse_direction(&new).control.as_deref() != Some(word) {
        return Err(SteerError::PostCondition(format!("control: {word}")));
    }
    Ok(new)
}

// ── Answer and un-hold (D-15, D-16) ─────────────────────────────────────────

/// How many `char`s an answer may hold. Counted in `char`s, not bytes, so a CJK
/// answer gets the same room as an ASCII one (`[inferred — audit]`, UI-SPEC).
pub const DIRECTION_ANSWER_MAX_CHARS: usize = 500;

/// Why an answer was refused. The class decides the copy the screen shows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AnswerRule {
    /// Nothing is left after trimming.
    Empty,
    /// A control character (tab, CR and LF included), U+2028 or U+2029: the answer
    /// would not stay one line of the body.
    ControlChar,
    /// `<!--` or `-->`: the answer would open or close a comment and hide or reveal
    /// the rest of the body.
    CommentMarker,
    /// More than [`DIRECTION_ANSWER_MAX_CHARS`] characters.
    TooLong,
}

impl std::fmt::Display for AnswerRule {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AnswerRule::Empty => write!(f, "an answer cannot be empty"),
            AnswerRule::ControlChar => {
                write!(f, "an answer must be one line with no control characters")
            }
            AnswerRule::CommentMarker => write!(f, "an answer cannot contain <!-- or -->"),
            AnswerRule::TooLong => write!(
                f,
                "an answer is limited to {DIRECTION_ANSWER_MAX_CHARS} characters"
            ),
        }
    }
}

/// Trim `text` and check it can be written as the value of one `- answer:` line.
///
/// Refused: empty, any control character (`char::is_control`: tab, CR, LF, DEL and
/// the C1 range), U+2028, U+2029, `<!--`, `-->`, and more than
/// [`DIRECTION_ANSWER_MAX_CHARS`] characters. `|` is allowed (an answer is not a log
/// field, unlike the `one_line` predicate this mirrors). The check runs before any
/// write, so a refused answer touches nothing (T-30-11).
pub fn validate_answer(text: &str) -> Result<String, AnswerRule> {
    let trimmed = text.trim();
    if trimmed.is_empty() {
        return Err(AnswerRule::Empty);
    }
    if trimmed.chars().count() > DIRECTION_ANSWER_MAX_CHARS {
        return Err(AnswerRule::TooLong);
    }
    if trimmed
        .chars()
        .any(|c| c.is_control() || c == '\u{2028}' || c == '\u{2029}')
    {
        return Err(AnswerRule::ControlChar);
    }
    if trimmed.contains("<!--") || trimmed.contains("-->") {
        return Err(AnswerRule::CommentMarker);
    }
    Ok(trimmed.to_string())
}

/// Where one item's lines sit in a body: the byte offset just after its `### I-NN`
/// heading line, the offset where its segment ends (the next `### ` or `## ` heading,
/// or the end of the body), and the heading's line terminator.
struct Segment {
    start: usize,
    end: usize,
    eol: &'static str,
}

/// The segment of the FIRST `### {id}` heading under `## Items`, read from the
/// comment-blanked body exactly as [`read_items`] walks it (a repeated id keeps its
/// first heading, as the merged list does).
fn item_segment(blanked: &str, id: &str) -> Option<Segment> {
    let mut section = Section::Preamble;
    let mut found: Option<Segment> = None;
    let mut offset = 0usize;
    for raw in blanked.split_inclusive('\n') {
        let line_start = offset;
        offset += raw.len();
        let line = raw.trim_end_matches(['\n', '\r']);
        let heading = section_heading(line);
        if let Some(next) = heading {
            section = next;
        }
        if let Some(segment) = found.as_mut() {
            if heading.is_some() || line.starts_with("### ") {
                segment.end = line_start;
                return found;
            }
            continue;
        }
        if heading.is_some() || section != Section::Items || !line.starts_with("### ") {
            continue;
        }
        let is_it = item_re()
            .captures(line)
            .and_then(|caps| caps.get(1))
            .is_some_and(|m| m.as_str() == id);
        if is_it {
            found = Some(Segment {
                start: offset,
                end: blanked.len(),
                eol: eol_of(raw),
            });
        }
    }
    found
}

/// One `- {key}:` line inside an item segment, located in the blanked body.
struct FieldLine {
    /// Absolute offset of the first byte after the line's `:`.
    after_colon: usize,
    /// Absolute offsets of the trimmed value (equal when the value is empty).
    value: (usize, usize),
}

/// The `- {key}:` line `read_items` would read for `key`: the first one with a value,
/// else the first one at all.
fn field_line(blanked: &str, segment: &Segment, key: &str) -> Option<FieldLine> {
    let mut first_empty: Option<FieldLine> = None;
    let mut offset = segment.start;
    for raw in blanked
        .get(segment.start..segment.end)?
        .split_inclusive('\n')
    {
        let line_start = offset;
        offset += raw.len();
        let line = raw.trim_end_matches(['\n', '\r']);
        if key_value(line, key).is_none() {
            continue;
        }
        let Some(colon) = line.find(':') else {
            continue;
        };
        let rest = &line[colon + 1..];
        let lead = rest.len() - rest.trim_start().len();
        let value = rest.trim();
        let after_colon = line_start + colon + 1;
        let start = after_colon + lead;
        let field = FieldLine {
            after_colon,
            value: (start, start + value.len()),
        };
        if !value.is_empty() {
            return Some(field);
        }
        first_empty.get_or_insert(field);
    }
    first_empty
}

/// Set `- {key}: {value}` in the segment of `id`: the value of the line `read_items`
/// reads is rewritten in place (the rest of the line is kept); with no such line,
/// `- {key}: {value}` is appended after the segment's last non-blank line.
fn set_item_field(body: &str, id: &str, key: &str, value: &str) -> Result<String, SteerError> {
    let blanked = blank_comments(body);
    let segment =
        item_segment(&blanked, id).ok_or_else(|| SteerError::NoSuchItem(id.to_string()))?;
    let offsets = || SteerError::PostCondition("offsets".to_string());
    if let Some(field) = field_line(&blanked, &segment, key) {
        let (start, end) = field.value;
        let edit = if start == end {
            (field.after_colon, field.after_colon, format!(" {value}"))
        } else {
            (start, end, value.to_string())
        };
        return splice_ranges(body, &[(edit.0, edit.1, edit.2.as_str())]).ok_or_else(offsets);
    }
    // Append after the last non-blank line of the segment (or the heading itself).
    let mut at = segment.start;
    let mut offset = segment.start;
    let mut terminated = true;
    let lines = body.get(segment.start..segment.end).ok_or_else(offsets)?;
    for raw in lines.split_inclusive('\n') {
        offset += raw.len();
        if !raw.trim().is_empty() {
            at = offset;
            terminated = raw.ends_with('\n');
        }
    }
    let eol = segment.eol;
    let line = if terminated {
        format!("- {key}: {value}{eol}")
    } else {
        format!("{eol}- {key}: {value}{eol}")
    };
    splice_ranges(body, &[(at, at, line.as_str())]).ok_or_else(offsets)
}

/// The ids of `body`'s items in document order, for the "no item was dropped" check.
fn item_ids(body: &str) -> Vec<String> {
    read_items(body).into_iter().map(|item| item.id).collect()
}

/// Check a transformed body still lists exactly the items the old one did, and that
/// item `id` reads as off hold (and as `answer`, when given).
fn check_item(
    old: &str,
    new: String,
    id: &str,
    answer: Option<&str>,
) -> Result<String, SteerError> {
    if item_ids(&new) != item_ids(old) {
        return Err(SteerError::PostCondition("item ids".to_string()));
    }
    let item = read_items(&new)
        .into_iter()
        .find(|item| item.id == id)
        .ok_or_else(|| SteerError::NoSuchItem(id.to_string()))?;
    if item.is_on_hold() {
        return Err(SteerError::PostCondition("hold: false".to_string()));
    }
    if answer.is_some_and(|text| item.answer.as_deref() != Some(text)) {
        return Err(SteerError::PostCondition("answer".to_string()));
    }
    Ok(new)
}

/// Take item `id` off hold: its first uncommented `- hold:` value becomes `false`, or
/// `- hold: false` is appended to its segment. Idempotent; item order and ids are
/// untouched (checked, not assumed).
pub fn unhold(body: &str, id: &str) -> Result<String, SteerError> {
    let new = set_item_field(body, id, "hold", "false")?;
    check_item(body, new, id, None)
}

/// Answer item `id` with `text` and lift its hold in the same write: the first
/// `- answer:` line takes the validated text (or one is appended), then [`unhold`].
/// A refused answer is an error before any byte moves.
pub fn answer(body: &str, id: &str, text: &str) -> Result<String, SteerError> {
    let text = validate_answer(text).map_err(SteerError::AnswerRejected)?;
    let with_answer = set_item_field(body, id, "answer", &text)?;
    let new = set_item_field(&with_answer, id, "hold", "false")?;
    check_item(body, new, id, Some(&text))
}

/// Apply one [`SteerOp`] to a body.
pub fn apply_op(body: &str, op: &SteerOp) -> Result<String, SteerError> {
    match op {
        SteerOp::SetControl(word) => set_control(body, word),
        SteerOp::Answer { item, text } => answer(body, item, text),
        SteerOp::Unhold { item } => unhold(body, item),
    }
}

/// How a [`steer_run`] ended.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SteerOutcome {
    /// The new body is in place. `lock_timed_out` is true when `DIRECTION.lock` could
    /// not be taken within its budget and the write went ahead without it (27 D-20):
    /// the exchange-verify check still kept every save.
    Written { lock_timed_out: bool },
    /// The body already said it; nothing changed.
    Unchanged,
    /// The transform refused (an unknown item, an answer that cannot be written).
    Refused(SteerError),
    /// `write_body` could not place the write; the file stands. One-line reason.
    NotWritten(String),
    /// A file or platform fault; the file was not changed.
    Failed(String),
}

/// Write one steering edit to `<run_dir>/DIRECTION.md`'s body, under
/// `DIRECTION.lock`, keeping the tool block verbatim (D-14).
///
/// **Blocking: call it only inside `spawn_blocking`.** The closure handed to
/// [`write_body`] may run more than once; each run recomputes from the live body and
/// forgets the previous run's verdict.
pub fn steer_run(run_dir: &Path, op: &SteerOp) -> SteerOutcome {
    let mut refusal: Option<SteerError> = None;
    let mut changed = false;
    let report = write_body(run_dir, |live| match apply_op(live, op) {
        Ok(new) => {
            refusal = None;
            changed = new != live;
            new
        }
        Err(error) => {
            refusal = Some(error);
            changed = false;
            live.to_string()
        }
    });
    match report {
        Err(error) => SteerOutcome::Failed(error.to_string()),
        Ok(report) => match report.outcome {
            SpliceOutcome::Ok => match refusal {
                Some(error) => SteerOutcome::Refused(error),
                None if !changed => SteerOutcome::Unchanged,
                None => SteerOutcome::Written {
                    lock_timed_out: report.lock_timed_out,
                },
            },
            other => SteerOutcome::NotWritten(not_written_reason(other).to_string()),
        },
    }
}

// ── The editor flow: a body-only temp that never clobbers (D-17, D-24) ──────

/// The edit temp's file name, inside the run directory. It does **not** start with the
/// splice prefix `.DIRECTION.md.fm.` (so `sweep_stale_temps` never deletes a user's
/// unsaved text) and the watcher classifies it `Ignored` (D-24). The name is the
/// UI-SPEC's; the `.DIRECTION.edit.md` refinement is not taken (`[inferred — audit]`).
pub const EDIT_TEMP_NAME: &str = ".DIRECTION.md.edit";

/// The largest `DIRECTION.md` or edit temp the flow reads: 1 MiB. A body past it is
/// refused rather than slurped (T-30-12).
pub const EDIT_TEMP_MAX_BYTES: u64 = 1 << 20;

/// What [`prepare_edit`] made: the temp the editor opens, and the body it was filled
/// from (the base the apply is checked against).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreparedEdit {
    pub temp: std::path::PathBuf,
    pub base: String,
}

/// Why [`prepare_edit`] made no temp.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PrepareError {
    /// A temp is already there (an edit a crashed TUI never applied, or something
    /// planted). It is never overwritten: the user's text may be in it.
    StaleTemp(std::path::PathBuf),
    /// `DIRECTION.md` could not be read, or is over [`EDIT_TEMP_MAX_BYTES`].
    Read(String),
    /// The temp could not be created or written.
    Create(String),
}

/// Read `<run_dir>/DIRECTION.md` whole, refusing one past [`EDIT_TEMP_MAX_BYTES`].
///
/// Public so the Driver tab's launch preflight and new-run check read the file under
/// the same cap the edit flow uses (30-05). **Blocking: `spawn_blocking` only.**
pub fn read_direction(run_dir: &Path) -> Result<String, String> {
    let path = run_dir.join(crate::driver::direction::DIRECTION_FILE);
    let file = crate::state_reader::driver_frontmatter::open_run_file(&path)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    read_capped(file, &path)
}

/// Read `file` as UTF-8, at most [`EDIT_TEMP_MAX_BYTES`].
fn read_capped(file: std::fs::File, path: &Path) -> Result<String, String> {
    use std::io::Read;
    let mut bytes = Vec::new();
    file.take(EDIT_TEMP_MAX_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| format!("{}: {e}", path.display()))?;
    if bytes.len() as u64 > EDIT_TEMP_MAX_BYTES {
        return Err(format!("{} is larger than 1 MiB", path.display()));
    }
    String::from_utf8(bytes).map_err(|_| format!("{} is not valid UTF-8", path.display()))
}

/// The live body of `<run_dir>/DIRECTION.md` (the text after the tool block), for the
/// re-open path: the base a kept edit is checked against from now on.
///
/// **Blocking: `spawn_blocking` only.**
pub fn live_body(run_dir: &Path) -> Result<String, String> {
    let text = read_direction(run_dir)?;
    Ok(crate::driver::direction::split_frontmatter(&text)
        .1
        .to_string())
}

/// Write the live body of `<run_dir>/DIRECTION.md` (never the `---` tool block) to
/// `<run_dir>/.DIRECTION.md.edit` for the user's editor.
///
/// The temp is created with `create_new` (`O_EXCL`: it fails on any existing path,
/// a planted symlink included) and mode 0600, so a sandboxed process that can write the
/// run directory cannot pre-place it and cannot read the user's text. An existing
/// temp is [`PrepareError::StaleTemp`] and is left exactly as it was.
///
/// **Blocking: `spawn_blocking` only.** No lock is taken or held: the editor session
/// can last hours and the lock exists for single writes (D-17).
pub fn prepare_edit(run_dir: &Path) -> Result<PreparedEdit, PrepareError> {
    use std::io::Write;
    let text = read_direction(run_dir).map_err(PrepareError::Read)?;
    let base = crate::driver::direction::split_frontmatter(&text)
        .1
        .to_string();
    let temp = run_dir.join(EDIT_TEMP_NAME);
    let mut options = std::fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let mut file = match options.open(&temp) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {
            return Err(PrepareError::StaleTemp(temp));
        }
        Err(e) => return Err(PrepareError::Create(format!("{}: {e}", temp.display()))),
    };
    if let Err(e) = file
        .write_all(base.as_bytes())
        .and_then(|()| file.sync_all())
    {
        // The temp is the file this call just created, so taking it back is ours.
        drop(file);
        let _ = std::fs::remove_file(&temp);
        return Err(PrepareError::Create(format!("{}: {e}", temp.display())));
    }
    Ok(PreparedEdit { temp, base })
}

/// Remove the edit temp of `run_dir` (the user chose to discard it). The path is built
/// here from the run directory, never taken from a caller, so this can only ever
/// remove `<run_dir>/.DIRECTION.md.edit`. An absent temp is fine.
///
/// **Blocking: `spawn_blocking` only.**
pub fn discard_edit(run_dir: &Path) -> Result<(), String> {
    let temp = run_dir.join(EDIT_TEMP_NAME);
    match std::fs::remove_file(&temp) {
        Ok(()) => Ok(()),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(e) => Err(format!("{}: {e}", temp.display())),
    }
}

/// How an [`apply_edit`] ended. Every outcome but `Unchanged` and `Applied` keeps the
/// temp, so the user's text is never lost.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EditOutcome {
    /// The editor saved nothing new; the temp was removed and nothing was written.
    Unchanged,
    /// The live body is no longer the base the user started from: nothing was written.
    Conflict,
    /// The edited body lacks `## Direction` or `## Items`: nothing was written.
    Insane,
    /// `driver check` found problems in the edit; each is one rendered line. Nothing
    /// was written; applying anyway is the user's call (`accept_lint`).
    LintProblems(Vec<String>),
    /// The edit is in place and the temp was removed.
    Applied { lock_timed_out: bool },
    /// The temp or the file could not be read or written; the file stands.
    Failed(String),
}

/// Why `write_body` placed nothing, in one phrase.
fn not_written_reason(outcome: SpliceOutcome) -> &'static str {
    match outcome {
        SpliceOutcome::Ok => "written",
        SpliceOutcome::SkippedInsaneBody => "the body lacks \"## Direction\" or \"## Items\"",
        SpliceOutcome::GaveUp => "another save kept changing the file, so this one gave up",
        SpliceOutcome::NoDirection => "the run has no DIRECTION.md",
    }
}

/// Apply the user's saved edit (`temp`) to `<run_dir>/DIRECTION.md`'s body, **only if
/// the live body still equals `base`**, the text the editor session started from.
///
/// The base is compared before the write and again inside the [`write_body`] closure
/// (which sees the body under `DIRECTION.lock`, and again on a fresh body after a
/// raced human save), so a change made while the editor was open is never overwritten:
/// the outcome is [`EditOutcome::Conflict`], the other change stands and the temp is
/// kept (T-30-10). The `---` tool block is never touched.
///
/// `temp` must be a regular file (not a symlink; refused on the open handle) of at most [`EDIT_TEMP_MAX_BYTES`]
/// holding UTF-8. `accept_lint` is the user's "apply it anyway" after a
/// [`EditOutcome::LintProblems`].
///
/// **Blocking: `spawn_blocking` only.**
pub fn apply_edit(run_dir: &Path, temp: &Path, base: &str, accept_lint: bool) -> EditOutcome {
    // One open for the whole check-and-read (T-30-24): the type is checked on the
    // handle `open_run_file` returns, the size comes from it, and `read_capped` reads
    // it. The path is never opened a second time, so a link swapped in after the
    // check is never followed.
    let file = match crate::state_reader::driver_frontmatter::open_run_file(temp) {
        Ok(file) => file,
        Err(e) => return EditOutcome::Failed(format!("{}: {e}", temp.display())),
    };
    match file.metadata() {
        Ok(meta) if meta.len() > EDIT_TEMP_MAX_BYTES => {
            return EditOutcome::Failed(format!("{} is larger than 1 MiB", temp.display()));
        }
        Ok(_) => {}
        Err(e) => return EditOutcome::Failed(format!("{}: {e}", temp.display())),
    }
    let edited = match read_capped(file, temp) {
        Ok(text) => text,
        Err(error) => return EditOutcome::Failed(error),
    };

    if edited == base {
        let _ = std::fs::remove_file(temp);
        return EditOutcome::Unchanged;
    }
    if !body_sane(&edited) {
        return EditOutcome::Insane;
    }
    if !accept_lint {
        let problems: Vec<String> = lint_direction(&edited)
            .problems
            .iter()
            .map(|finding| finding.to_string())
            .collect();
        if !problems.is_empty() {
            return EditOutcome::LintProblems(problems);
        }
    }
    match live_body(run_dir) {
        Ok(live) if live == base => {}
        Ok(_) => return EditOutcome::Conflict,
        Err(error) => return EditOutcome::Failed(error),
    }

    let mut conflict = false;
    let report = write_body(run_dir, |live| {
        if live == base {
            conflict = false;
            edited.clone()
        } else {
            conflict = true;
            live.to_string()
        }
    });
    match report {
        Err(crate::driver::direction::DirectionError::InsaneNewBody) => EditOutcome::Insane,
        Err(error) => EditOutcome::Failed(error.to_string()),
        Ok(report) => match report.outcome {
            SpliceOutcome::Ok if conflict => EditOutcome::Conflict,
            SpliceOutcome::Ok => {
                let _ = std::fs::remove_file(temp);
                EditOutcome::Applied {
                    lock_timed_out: report.lock_timed_out,
                }
            }
            other => EditOutcome::Failed(not_written_reason(other).to_string()),
        },
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    const BODY: &str = "# Driver direction: r\n\n## Direction\n\n- control: run\n\n## Items\n\n\
### I-01 first thing\n\n- hold: false\n- answer:\n\n### I-02 second thing\n\n- hold: true\n\
- answer: use /run/user\n";

    #[test]
    fn the_template_yields_one_item_with_hold_false_and_no_answer() {
        let items = read_items(crate::driver_templates::DIRECTION_TEMPLATE);
        assert_eq!(items.len(), 1, "{items:?}");
        assert_eq!(items[0].id, "I-01");
        assert_eq!(items[0].title, "{short title}");
        assert_eq!(items[0].hold.as_deref(), Some("false"));
        assert_eq!(items[0].answer, None);
        assert!(!items[0].is_on_hold());
    }

    #[test]
    fn ids_keep_document_order_and_carry_their_own_fields() {
        let items = read_items(BODY);
        let ids: Vec<&str> = items.iter().map(|i| i.id.as_str()).collect();
        assert_eq!(ids, ["I-01", "I-02"]);
        assert_eq!(items[0].title, "first thing");
        assert_eq!(items[0].answer, None);
        assert!(items[1].is_on_hold());
        assert_eq!(items[1].answer.as_deref(), Some("use /run/user"));
    }

    #[test]
    fn a_commented_out_item_is_not_an_item() {
        let body = "## Direction\n\n- control: run\n\n## Items\n\n<!--\n### I-09 hidden\n- hold: true\n-->\n\n### I-01 shown\n- hold: false\n";
        let items = read_items(body);
        assert_eq!(items.len(), 1, "{items:?}");
        assert_eq!(items[0].id, "I-01");
    }

    #[test]
    fn an_item_heading_outside_items_is_not_an_item() {
        let body =
            "## Direction\n\n### I-05 misplaced\n- hold: true\n\n## Items\n\n### I-01 real\n";
        let items = read_items(body);
        assert_eq!(items.len(), 1, "{items:?}");
        assert_eq!(items[0].id, "I-01");
        assert_eq!(items[0].hold, None);
    }

    #[test]
    fn a_hold_inside_a_comment_is_ignored() {
        let body = "## Items\n\n### I-01 t\n<!-- - hold: true -->\n- hold: false\n";
        let items = read_items(body);
        assert_eq!(items[0].hold.as_deref(), Some("false"));
        let body = "## Items\n\n### I-01 t\n<!-- - hold: true -->\n";
        assert_eq!(read_items(body)[0].hold, None);
    }

    #[test]
    fn a_cjk_title_round_trips() {
        let body = "## Items\n\n### I-01 配線を確認する 🚀\n- hold: false\n";
        let items = read_items(body);
        assert_eq!(items[0].title, "配線を確認する 🚀");
    }

    #[test]
    fn a_trailing_comment_is_not_part_of_the_title() {
        let body = "## Items\n\n### I-01 wire it <!-- 配線 note -->\n- hold: true\n";
        let items = read_items(body);
        assert_eq!(items[0].title, "wire it");
        assert!(items[0].is_on_hold());
    }

    #[test]
    fn fields_of_one_item_do_not_leak_into_the_next_and_a_sub_heading_ends_the_segment() {
        let body = "## Items\n\n### I-01 a\n- hold: true\n### Notes\n- answer: stray\n### I-02 b\n";
        let items = read_items(body);
        assert_eq!(items.len(), 2);
        assert_eq!(
            items[0].answer, None,
            "a non-item `###` heading ends the item"
        );
        assert_eq!(items[1].hold, None);
    }

    #[test]
    fn crlf_bodies_read_the_same_values() {
        let body = "## Items\r\n\r\n### I-01 crlf title\r\n- hold: true\r\n- answer: yes\r\n";
        let items = read_items(body);
        assert_eq!(items[0].title, "crlf title");
        assert_eq!(items[0].hold.as_deref(), Some("true"));
        assert_eq!(items[0].answer.as_deref(), Some("yes"));
    }

    #[test]
    fn keys_match_in_any_case_and_an_empty_value_is_none() {
        let body = "## Items\n\n### I-01 t\n- HOLD: TRUE\n- Answer:   \n";
        let items = read_items(body);
        assert!(items[0].is_on_hold());
        assert_eq!(items[0].answer, None);
    }

    #[test]
    fn a_body_with_no_items_section_has_no_items() {
        assert!(read_items("").is_empty());
        assert!(read_items("## Direction\n\n- control: run\n").is_empty());
    }

    // ── set_control (Task 1) ────────────────────────────────────────────────

    use crate::driver_templates::DIRECTION_TEMPLATE;

    #[test]
    fn set_control_rewrites_only_the_word_on_the_template() {
        for word in ["pause", "stop", "run"] {
            let new = set_control(DIRECTION_TEMPLATE, word).expect("the template steers");
            assert_eq!(parse_direction(&new).control.as_deref(), Some(word));
            // Only the word moved: putting `run` back gives the original bytes.
            let restored = set_control(&new, "run").expect("and back");
            assert_eq!(restored, DIRECTION_TEMPLATE, "{word}");
        }
    }

    #[test]
    fn set_control_keeps_the_rest_of_a_control_line() {
        let body = "## Direction\n\n  - Control :  run   <!-- why -->\n\n## Items\n";
        let new = set_control(body, "pause").expect("steers");
        assert_eq!(
            new,
            "## Direction\n\n  - Control :  pause   <!-- why -->\n\n## Items\n"
        );
    }

    #[test]
    fn launcher_steer_two_control_lines_both_follow_the_word() {
        // Edge E7: `parse_direction` lets a `pause` win over a `run`, so setting only
        // one line would leave the run held (or not) by the other.
        let body = "## Direction\n\n- control: run\n- control: pause\n\n## Items\n";
        let stopped = set_control(body, "stop").expect("steers");
        assert_eq!(
            stopped,
            "## Direction\n\n- control: stop\n- control: stop\n\n## Items\n"
        );
        let running = set_control(&stopped, "run").expect("steers");
        assert_eq!(
            running,
            "## Direction\n\n- control: run\n- control: run\n\n## Items\n"
        );
        assert_eq!(parse_direction(&running).control.as_deref(), Some("run"));
    }

    #[test]
    fn a_control_line_inside_a_comment_is_not_a_control_line() {
        let body = "# t\n\n## Direction\n\n<!-- - control: stop -->\n- goal: x\n\n## Items\n";
        let new = set_control(body, "pause").expect("steers");
        // Inserted directly under the heading; the commented line is untouched.
        assert_eq!(
            new,
            "# t\n\n## Direction\n- control: pause\n\n<!-- - control: stop -->\n- goal: x\n\n## Items\n"
        );
        assert_eq!(parse_direction(&new).control.as_deref(), Some("pause"));
    }

    #[test]
    fn a_missing_control_line_is_inserted_with_the_headings_line_ending() {
        let crlf = "## Direction\r\n\r\n- goal: x\r\n\r\n## Items\r\n";
        let new = set_control(crlf, "stop").expect("steers");
        assert!(
            new.starts_with("## Direction\r\n- control: stop\r\n"),
            "{new:?}"
        );
        // A heading on an unterminated last line gets a newline of its own.
        let bare = "## Items\n\n## Direction";
        let new = set_control(bare, "run").expect("steers");
        assert_eq!(new, "## Items\n\n## Direction\n- control: run\n");
    }

    #[test]
    fn set_control_is_idempotent() {
        for body in [
            DIRECTION_TEMPLATE,
            "## Direction\n\n## Items\n",
            "## Direction\n- control: run\n- control: pause\n## Items\n",
        ] {
            for word in CONTROL_WORDS {
                let once = set_control(body, word).expect("steers");
                assert_eq!(set_control(&once, word).as_deref(), Ok(once.as_str()));
            }
        }
    }

    #[test]
    fn set_control_refuses_a_bad_word_and_a_body_without_a_direction_section() {
        assert_eq!(
            set_control(DIRECTION_TEMPLATE, "halt"),
            Err(SteerError::BadControlWord("halt".to_string()))
        );
        assert_eq!(
            set_control(DIRECTION_TEMPLATE, "Pause"),
            Err(SteerError::BadControlWord("Pause".to_string())),
            "the chooser only offers the lower-case words"
        );
        assert_eq!(
            set_control("# no sections\n\n## Items\n", "run"),
            Err(SteerError::NoDirectionSection)
        );
        // The heading is only a heading outside a comment.
        assert_eq!(
            set_control("<!--\n## Direction\n-->\n## Items\n", "run"),
            Err(SteerError::NoDirectionSection)
        );
    }

    #[test]
    fn a_cjk_body_keeps_its_bytes_around_the_edit() {
        let body =
            "# 配線 🚀\n\n## Direction\n\n- control: run\n- goal: 配線を確認する\n\n## Items\n";
        let new = set_control(body, "stop").expect("steers");
        assert_eq!(new, body.replace("control: run", "control: stop"));
    }

    // ── steer_run: the write goes through write_body (Linux) ────────────────

    /// A run dir holding an emitted frontmatter block and `body`; returns the block.
    #[cfg(target_os = "linux")]
    pub(crate) fn write_run(dir: &Path, body: &str) -> String {
        let fm = crate::state_reader::driver_frontmatter::parse_driver_frontmatter(include_str!(
            "../../tests/fixtures/driverctl/direction/s003-run1b.md"
        ))
        .expect("the spike fixture parses");
        let block = fm.emit();
        std::fs::write(dir.join("DIRECTION.md"), format!("{block}{body}")).expect("DIRECTION.md");
        block
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn launcher_steer_a_control_write_changes_only_the_body() {
        let dir = tempfile::tempdir().expect("temp dir");
        let block = write_run(dir.path(), DIRECTION_TEMPLATE);

        let outcome = steer_run(dir.path(), &SteerOp::SetControl("pause".to_string()));
        assert_eq!(
            outcome,
            SteerOutcome::Written {
                lock_timed_out: false
            }
        );
        let text = std::fs::read_to_string(dir.path().join("DIRECTION.md")).unwrap();
        let (fm, body) = crate::driver::direction::split_frontmatter(&text);
        assert_eq!(fm, Some(block.as_str()), "the tool block is byte-identical");
        assert_eq!(parse_direction(body).control.as_deref(), Some("pause"));
        assert_eq!(body, set_control(DIRECTION_TEMPLATE, "pause").unwrap());

        // The same word again: nothing to write.
        let outcome = steer_run(dir.path(), &SteerOp::SetControl("pause".to_string()));
        assert_eq!(outcome, SteerOutcome::Unchanged);
        // A refusal writes nothing and says why.
        let outcome = steer_run(dir.path(), &SteerOp::SetControl("halt".to_string()));
        assert_eq!(
            outcome,
            SteerOutcome::Refused(SteerError::BadControlWord("halt".to_string()))
        );
        let after = std::fs::read_to_string(dir.path().join("DIRECTION.md")).unwrap();
        assert_eq!(
            after, text,
            "neither the no-op nor the refusal changed a byte"
        );
    }

    /// `(inode, mtime)` of the run's DIRECTION.md, and whether any splice temp is left.
    #[cfg(target_os = "linux")]
    fn file_identity(dir: &Path) -> (u64, std::time::SystemTime) {
        use std::os::unix::fs::MetadataExt;
        let meta = std::fs::metadata(dir.join("DIRECTION.md")).unwrap();
        (meta.ino(), meta.modified().unwrap())
    }

    #[cfg(target_os = "linux")]
    fn no_splice_temp(dir: &Path) -> bool {
        std::fs::read_dir(dir).unwrap().all(|entry| {
            !entry
                .unwrap()
                .file_name()
                .to_string_lossy()
                .starts_with(crate::driver::direction::TEMP_PREFIX)
        })
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn steer_run_unchanged_and_refused_leave_the_inode_alone() {
        let dir = tempfile::tempdir().expect("temp dir");
        write_run(dir.path(), &clean_body());
        let before = file_identity(dir.path());
        let text = live_text(dir.path());

        // The body already says `run` (the template's control word).
        let outcome = steer_run(dir.path(), &SteerOp::SetControl("run".to_string()));
        assert_eq!(outcome, SteerOutcome::Unchanged);
        assert_eq!(
            file_identity(dir.path()),
            before,
            "an unchanged body is not rewritten"
        );

        let outcome = steer_run(
            dir.path(),
            &SteerOp::Answer {
                item: "I-99".to_string(),
                text: "nope".to_string(),
            },
        );
        assert!(matches!(outcome, SteerOutcome::Refused(_)), "{outcome:?}");
        assert_eq!(
            file_identity(dir.path()),
            before,
            "a refusal is not rewritten"
        );

        let outcome = steer_run(dir.path(), &SteerOp::SetControl("halt".to_string()));
        assert!(matches!(outcome, SteerOutcome::Refused(_)), "{outcome:?}");
        assert_eq!(file_identity(dir.path()), before);
        assert_eq!(live_text(dir.path()), text);
        assert!(no_splice_temp(dir.path()));

        // A real change still writes: the inode moves.
        let outcome = steer_run(dir.path(), &SteerOp::SetControl("pause".to_string()));
        assert!(
            matches!(outcome, SteerOutcome::Written { .. }),
            "{outcome:?}"
        );
        assert_ne!(file_identity(dir.path()).0, before.0);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn apply_edit_conflict_leaves_the_inode_alone() {
        let dir = tempfile::tempdir().expect("temp dir");
        write_run(dir.path(), &clean_body());
        let prepared = prepare_edit(dir.path()).expect("prepare");
        std::fs::write(&prepared.temp, prepared.base.replace("Prove it", "edited")).unwrap();
        // Someone else changes the body while the editor is open.
        let raced = clean_body().replace("It works.", "It works, raced.");
        write_run(dir.path(), &raced);
        let before = file_identity(dir.path());

        let outcome = apply_edit(dir.path(), &prepared.temp, &prepared.base, false);

        assert_eq!(outcome, EditOutcome::Conflict);
        assert_eq!(
            file_identity(dir.path()),
            before,
            "the raced file is not rewritten"
        );
        assert!(no_splice_temp(dir.path()));
        assert!(prepared.temp.exists(), "the user's text is kept");
    }

    #[test]
    fn answer_on_an_unterminated_last_heading_writes_a_line_of_its_own() {
        let body = "## Direction\n\n- control: run\n\n## Items\n\n### I-05 last title";
        let new = answer(body, "I-05", "foo").expect("an answer on the last heading");
        assert!(
            new.ends_with("### I-05 last title\n- answer: foo\n- hold: false\n"),
            "{new:?}"
        );
        let items = read_items(&new);
        assert_eq!(items.len(), 1, "{items:?}");
        assert_eq!(items[0].title, "last title");
        assert_eq!(items[0].answer.as_deref(), Some("foo"));
        assert_eq!(items[0].hold.as_deref(), Some("false"));

        let freed = unhold(body, "I-05").expect("un-hold on the last heading");
        assert!(
            freed.ends_with("### I-05 last title\n- hold: false\n"),
            "{freed:?}"
        );
        assert_eq!(read_items(&freed)[0].hold.as_deref(), Some("false"));

        // The same shape with CRLF line ends keeps its terminator.
        let crlf = "## Direction\r\n\r\n- control: run\r\n\r\n## Items\r\n\r\n### I-05 last title";
        let new = answer(crlf, "I-05", "foo").expect("answer on a CRLF body");
        assert!(
            new.ends_with("### I-05 last title\r\n- answer: foo\r\n- hold: false\r\n"),
            "{new:?}"
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn launcher_steer_a_missing_file_and_an_insane_body_are_not_written() {
        let dir = tempfile::tempdir().expect("temp dir");
        let op = SteerOp::SetControl("run".to_string());
        assert!(matches!(
            steer_run(dir.path(), &op),
            SteerOutcome::NotWritten(_)
        ));
        // A body without both section headers is never built on.
        let block = write_run(dir.path(), "# half a save\n");
        let outcome = steer_run(dir.path(), &op);
        assert!(
            matches!(outcome, SteerOutcome::Failed(_) | SteerOutcome::Refused(_)),
            "{outcome:?}"
        );
        let text = std::fs::read_to_string(dir.path().join("DIRECTION.md")).unwrap();
        assert_eq!(text, format!("{block}# half a save\n"));
    }

    // ── validate_answer, answer, unhold (Task 2) ────────────────────────────

    #[test]
    fn launcher_steer_validate_answer_table() {
        // (input, verdict)
        let ok = |input: &str, expected: &str| {
            assert_eq!(validate_answer(input).as_deref(), Ok(expected), "{input:?}");
        };
        let refused = |input: &str, rule: AnswerRule| {
            assert_eq!(validate_answer(input), Err(rule), "{input:?}");
        };
        ok("use /run/user", "use /run/user");
        ok("  trimmed  ", "trimmed");
        ok("a | b | c", "a | b | c");
        ok("配線を確認する 🚀", "配線を確認する 🚀");
        // A pasted trailing newline is trimmed, not refused.
        ok("one line\n", "one line");

        refused("", AnswerRule::Empty);
        refused("   \t ", AnswerRule::Empty);
        for hostile in [
            "a\nb",
            "a\rb",
            "a\r\nb",
            "a\tb",
            "a\u{0}b",
            "a\u{1b}[31mb",
            "a\u{7f}b",
            "a\u{85}b",
            "a\u{2028}b",
            "a\u{2029}b",
        ] {
            refused(hostile, AnswerRule::ControlChar);
        }
        refused("closes --> early", AnswerRule::CommentMarker);
        refused("opens <!-- here", AnswerRule::CommentMarker);
        refused("<!-- x -->", AnswerRule::CommentMarker);
    }

    #[test]
    fn launcher_steer_the_answer_cap_is_five_hundred_chars_not_bytes() {
        assert_eq!(DIRECTION_ANSWER_MAX_CHARS, 500);
        let exactly = "x".repeat(500);
        assert_eq!(validate_answer(&exactly).as_deref(), Ok(exactly.as_str()));
        assert_eq!(validate_answer(&"x".repeat(501)), Err(AnswerRule::TooLong));
        // 500 three-byte chars are 1500 bytes and still fit.
        let cjk = "配".repeat(500);
        assert_eq!(validate_answer(&cjk).as_deref(), Ok(cjk.as_str()));
        assert_eq!(validate_answer(&"配".repeat(501)), Err(AnswerRule::TooLong));
    }

    const TWO_ITEMS: &str = "## Direction\n\n- control: run\n\n## Items\n\n\
### I-01 first thing\n\n- hold: true\n- answer:\n\n### I-02 second thing\n\n- hold: true\n\
- answer: old\n";

    #[test]
    fn unhold_rewrites_only_the_hold_value_of_that_item() {
        let new = unhold(TWO_ITEMS, "I-01").expect("steers");
        assert_eq!(
            new,
            TWO_ITEMS.replacen("- hold: true\n- answer:\n", "- hold: false\n- answer:\n", 1)
        );
        let items = read_items(&new);
        assert!(!items[0].is_on_hold() && items[1].is_on_hold());
        assert_eq!(
            unhold(&new, "I-01").as_deref(),
            Ok(new.as_str()),
            "idempotent"
        );
    }

    #[test]
    fn answer_sets_the_answer_and_lifts_the_hold_in_one_edit() {
        let new = answer(TWO_ITEMS, "I-01", "  use /run/user  ").expect("steers");
        let items = read_items(&new);
        assert_eq!(items[0].answer.as_deref(), Some("use /run/user"));
        assert!(!items[0].is_on_hold(), "the answer lifts the hold");
        assert_eq!(
            items[1].answer.as_deref(),
            Some("old"),
            "the neighbour is untouched"
        );
        assert!(items[1].is_on_hold());
        assert_eq!(
            new,
            TWO_ITEMS.replacen(
                "- hold: true\n- answer:\n",
                "- hold: false\n- answer: use /run/user\n",
                1
            )
        );
    }

    #[test]
    fn answer_replaces_an_existing_answer_value_and_keeps_a_trailing_comment() {
        let body = "## Direction\n\n## Items\n\n### I-01 t\n\n- hold: true <!-- why -->\n- answer: old <!-- note -->\n";
        let new = answer(body, "I-01", "new").expect("steers");
        assert_eq!(
            new,
            "## Direction\n\n## Items\n\n### I-01 t\n\n- hold: false <!-- why -->\n- answer: new <!-- note -->\n"
        );
    }

    #[test]
    fn launcher_steer_a_missing_hold_or_answer_line_is_appended_to_the_segment() {
        // Edge E8: the template's item has no `- hold:` change to make, so strip the lines.
        let body = "## Direction\n\n## Items\n\n### I-01 first\n\n- intent: x\n\n### I-02 second\n\n- intent: y\n";
        let new = answer(body, "I-01", "yes").expect("steers");
        assert_eq!(
            new,
            "## Direction\n\n## Items\n\n### I-01 first\n\n- intent: x\n- answer: yes\n- hold: false\n\n### I-02 second\n\n- intent: y\n"
        );
        let items = read_items(&new);
        assert_eq!(items[0].answer.as_deref(), Some("yes"));
        assert_eq!(items[0].hold.as_deref(), Some("false"));
        assert_eq!(items[1].answer, None, "the next item gets nothing");
        // The last item has no trailing newline: the append adds the separator.
        let tail = "## Items\n\n### I-01 t\n- intent: x";
        let new = unhold(tail, "I-01").expect("steers");
        assert_eq!(new, "## Items\n\n### I-01 t\n- intent: x\n- hold: false\n");
        // A heading-only item gets its lines right under the heading.
        let bare = "## Items\n\n### I-01 t\n";
        assert_eq!(
            unhold(bare, "I-01").as_deref(),
            Ok("## Items\n\n### I-01 t\n- hold: false\n")
        );
    }

    #[test]
    fn launcher_steer_the_template_item_takes_an_answer_with_an_empty_answer_line() {
        let new = answer(DIRECTION_TEMPLATE, "I-01", "use the socket").expect("steers");
        let items = read_items(&new);
        assert_eq!(items[0].answer.as_deref(), Some("use the socket"));
        assert_eq!(items[0].hold.as_deref(), Some("false"));
        assert!(
            new.ends_with("- hold: false\n- answer: use the socket\n"),
            "{new:?}"
        );
    }

    #[test]
    fn launcher_steer_lines_inside_a_comment_are_not_the_items_lines() {
        let body =
            "## Items\n\n### I-01 t\n\n<!-- - hold: true\n- answer: hidden -->\n- hold: true\n";
        let new = answer(body, "I-01", "visible").expect("steers");
        assert_eq!(
            new,
            "## Items\n\n### I-01 t\n\n<!-- - hold: true\n- answer: hidden -->\n- hold: false\n- answer: visible\n"
        );
    }

    #[test]
    fn launcher_steer_no_such_item_is_refused() {
        assert_eq!(
            unhold(TWO_ITEMS, "I-09"),
            Err(SteerError::NoSuchItem("I-09".to_string()))
        );
        assert_eq!(
            answer(TWO_ITEMS, "I-09", "x"),
            Err(SteerError::NoSuchItem("I-09".to_string()))
        );
        // A commented-out item and an item outside `## Items` are not items.
        let hidden = "## Items\n\n<!--\n### I-05 hidden\n- hold: true\n-->\n";
        assert_eq!(
            unhold(hidden, "I-05"),
            Err(SteerError::NoSuchItem("I-05".to_string()))
        );
        let misplaced = "## Direction\n\n### I-05 misplaced\n- hold: true\n\n## Items\n";
        assert_eq!(
            unhold(misplaced, "I-05"),
            Err(SteerError::NoSuchItem("I-05".to_string()))
        );
    }

    #[test]
    fn launcher_steer_an_answer_with_a_comment_marker_is_refused_before_any_edit() {
        for text in ["see <!-- this", "ends --> here"] {
            assert_eq!(
                answer(TWO_ITEMS, "I-01", text),
                Err(SteerError::AnswerRejected(AnswerRule::CommentMarker))
            );
        }
        // Even for an item that does not exist, the answer is judged first.
        assert_eq!(
            answer(TWO_ITEMS, "I-09", "a\nb"),
            Err(SteerError::AnswerRejected(AnswerRule::ControlChar))
        );
    }

    #[test]
    fn launcher_steer_transforms_are_idempotent_and_keep_every_item() {
        // Edge E9: write_body may run the closure again on a fresh body.
        for body in [TWO_ITEMS, DIRECTION_TEMPLATE] {
            let ids = item_ids(body);
            let first = read_items(body)[0].id.clone();
            let once = answer(body, &first, "yes | no").expect("steers");
            assert_eq!(
                answer(&once, &first, "yes | no").as_deref(),
                Ok(once.as_str())
            );
            assert_eq!(item_ids(&once), ids, "no item is dropped or reordered");
            let held = unhold(body, &first).expect("steers");
            assert_eq!(unhold(&held, &first).as_deref(), Ok(held.as_str()));
            assert_eq!(item_ids(&held), ids);
        }
    }

    #[test]
    fn launcher_steer_apply_op_covers_all_three_ops() {
        let body = TWO_ITEMS;
        assert_eq!(
            apply_op(body, &SteerOp::SetControl("pause".to_string())),
            set_control(body, "pause")
        );
        assert_eq!(
            apply_op(
                body,
                &SteerOp::Answer {
                    item: "I-02".to_string(),
                    text: "ok".to_string()
                }
            ),
            answer(body, "I-02", "ok")
        );
        assert_eq!(
            apply_op(
                body,
                &SteerOp::Unhold {
                    item: "I-02".to_string()
                }
            ),
            unhold(body, "I-02")
        );
    }

    #[test]
    fn launcher_steer_a_crlf_body_keeps_its_line_endings() {
        let body = "## Items\r\n\r\n### I-01 t\r\n\r\n- intent: x\r\n";
        let new = answer(body, "I-01", "yes").expect("steers");
        assert_eq!(
            new,
            "## Items\r\n\r\n### I-01 t\r\n\r\n- intent: x\r\n- answer: yes\r\n- hold: false\r\n"
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn launcher_steer_answer_lifts_hold_and_keeps_every_item() {
        let dir = tempfile::tempdir().expect("temp dir");
        let block = write_run(dir.path(), TWO_ITEMS);
        let before = std::fs::read_to_string(dir.path().join("DIRECTION.md")).unwrap();
        let ids_before = item_ids(crate::driver::direction::split_frontmatter(&before).1);

        let outcome = steer_run(
            dir.path(),
            &SteerOp::Answer {
                item: "I-01".to_string(),
                text: "the socket is /run/user/1000/x".to_string(),
            },
        );
        assert_eq!(
            outcome,
            SteerOutcome::Written {
                lock_timed_out: false
            }
        );
        let text = std::fs::read_to_string(dir.path().join("DIRECTION.md")).unwrap();
        let (fm, body) = crate::driver::direction::split_frontmatter(&text);
        assert_eq!(fm, Some(block.as_str()), "the tool block is byte-identical");
        let items = read_items(body);
        assert_eq!(
            items.iter().map(|i| i.id.clone()).collect::<Vec<_>>(),
            ids_before
        );
        assert_eq!(
            items[0].answer.as_deref(),
            Some("the socket is /run/user/1000/x")
        );
        assert!(!items[0].is_on_hold());

        // An un-hold of an item that is not on hold is a no-op.
        let outcome = steer_run(
            dir.path(),
            &SteerOp::Unhold {
                item: "I-01".to_string(),
            },
        );
        assert_eq!(outcome, SteerOutcome::Unchanged);
        // An answer with a comment marker is refused and the file is unchanged.
        let after_answer = std::fs::read_to_string(dir.path().join("DIRECTION.md")).unwrap();
        let outcome = steer_run(
            dir.path(),
            &SteerOp::Answer {
                item: "I-02".to_string(),
                text: "evil -->".to_string(),
            },
        );
        assert_eq!(
            outcome,
            SteerOutcome::Refused(SteerError::AnswerRejected(AnswerRule::CommentMarker))
        );
        assert_eq!(
            std::fs::read_to_string(dir.path().join("DIRECTION.md")).unwrap(),
            after_answer
        );
    }

    /// The lock-timeout case (orchestrator decision 3): another writer holds
    /// `DIRECTION.lock` past the 2 s budget, the write goes ahead without it, lands
    /// exactly once, leaves the tool block byte-identical, and is reported.
    #[cfg(target_os = "linux")]
    #[test]
    fn launcher_steer_a_held_lock_past_the_budget_still_writes_once_and_says_so() {
        use crate::driver::direction::direction_lock;
        let dir = tempfile::tempdir().expect("temp dir");
        let block = write_run(dir.path(), TWO_ITEMS);
        // The lock is held by a different thread for longer than the 2 s budget.
        let (taken_tx, taken_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel::<()>();
        let run_dir = dir.path().to_path_buf();
        let holder = std::thread::spawn(move || {
            let guard = direction_lock(&run_dir).expect("the holder takes the lock");
            taken_tx.send(()).expect("signal");
            let _ = release_rx.recv_timeout(std::time::Duration::from_secs(3));
            drop(guard);
        });
        taken_rx.recv().expect("the lock is held");

        let outcome = steer_run(
            dir.path(),
            &SteerOp::Unhold {
                item: "I-02".to_string(),
            },
        );
        release_tx.send(()).ok();
        holder.join().expect("holder thread");

        assert_eq!(
            outcome,
            SteerOutcome::Written {
                lock_timed_out: true
            },
            "the write is reported as written, without the lock"
        );
        let text = std::fs::read_to_string(dir.path().join("DIRECTION.md")).unwrap();
        let (fm, body) = crate::driver::direction::split_frontmatter(&text);
        assert_eq!(fm, Some(block.as_str()), "the tool block is byte-identical");
        assert_eq!(
            body,
            unhold(TWO_ITEMS, "I-02").unwrap(),
            "the body changed exactly once"
        );
        // The App copy names the missing lock.
        let (message, error) = crate::ui::screens::driver_steer::steered_message(
            &SteerOp::Unhold {
                item: "I-02".to_string(),
            },
            &outcome,
            true,
            None,
        );
        assert!(!error);
        assert!(
            message.contains("Written without DIRECTION.lock"),
            "{message}"
        );
    }

    // ── The editor flow (Task 3) ────────────────────────────────────────────
    //
    // The editor is simulated by writing the temp: no process is spawned in `src/`
    // (`tests/spawn_seam_guard.rs`).

    /// A body that lints clean: the template with its placeholders filled in.
    #[cfg(target_os = "linux")]
    pub(crate) fn clean_body() -> String {
        DIRECTION_TEMPLATE
            .replace("{run-id}", "20261020-edit")
            .replace(
                "{one or two sentences: what this run is for}",
                "Prove the edit flow.",
            )
            .replace("{short title}", "Prove it")
            .replace(
                "{what should be true afterwards and why, in plain words}",
                "It works.",
            )
            .replace(
                "{a check the driver can run itself, e.g. `cargo test --no-fail-fast` passes AND `.planning/phases/21-*/21-VERIFICATION.md` has status: passed}",
                "`cargo test --no-fail-fast` passes",
            )
            .replace("- start-with: {optional GSD command hint, e.g. /gsd-plan-phase 26; the driver may choose otherwise}\n", "")
            .replace(
                "- {project-specific rules, e.g. \"run cargo test --no-fail-fast, not cargo test\"}\n",
                "",
            )
    }

    #[cfg(target_os = "linux")]
    fn live_text(dir: &Path) -> String {
        std::fs::read_to_string(dir.join("DIRECTION.md")).unwrap()
    }

    #[test]
    fn launcher_edit_the_temp_name_is_not_a_splice_temp_and_the_watcher_ignores_it() {
        use crate::driver::direction::{sweep_stale_temps, TEMP_PREFIX};
        assert!(
            !EDIT_TEMP_NAME.starts_with(TEMP_PREFIX),
            "sweep_stale_temps must never delete a user's unsaved text"
        );
        let root = Path::new("/home/user/projects/app");
        let path = root
            .join(".planning/meta-manager/drivers/20261020-edit")
            .join(EDIT_TEMP_NAME);
        assert_eq!(
            crate::journal::classify_change(root, &path),
            crate::journal::ChangeKind::Ignored
        );
        // And the sweep really leaves it, however old.
        let dir = tempfile::tempdir().expect("temp dir");
        std::fs::write(dir.path().join(EDIT_TEMP_NAME), "unsaved").unwrap();
        let removed = sweep_stale_temps(dir.path(), std::time::Duration::ZERO).unwrap();
        assert_eq!(removed, 0);
        assert!(dir.path().join(EDIT_TEMP_NAME).exists());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn launcher_edit_prepare_then_rewrite_then_apply_applies_and_keeps_the_tool_block() {
        let dir = tempfile::tempdir().expect("temp dir");
        let block = write_run(dir.path(), &clean_body());
        let prepared = prepare_edit(dir.path()).expect("prepares");
        assert_eq!(prepared.temp, dir.path().join(EDIT_TEMP_NAME));
        assert_eq!(prepared.base, clean_body());
        assert_eq!(
            std::fs::read_to_string(&prepared.temp).unwrap(),
            clean_body(),
            "the temp holds the body and no tool block"
        );

        let edited = clean_body().replace("Prove the edit flow.", "Prove the edit flow twice.");
        std::fs::write(&prepared.temp, &edited).unwrap();
        let outcome = apply_edit(dir.path(), &prepared.temp, &prepared.base, false);
        assert_eq!(
            outcome,
            EditOutcome::Applied {
                lock_timed_out: false
            }
        );
        assert!(!prepared.temp.exists(), "an applied temp is removed");
        let text = live_text(dir.path());
        let (fm, body) = crate::driver::direction::split_frontmatter(&text);
        assert_eq!(fm, Some(block.as_str()), "the tool block is byte-identical");
        assert_eq!(body, edited);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn launcher_edit_an_unchanged_save_writes_nothing_and_removes_the_temp() {
        // Edge E8.
        let dir = tempfile::tempdir().expect("temp dir");
        write_run(dir.path(), &clean_body());
        let before = live_text(dir.path());
        let prepared = prepare_edit(dir.path()).expect("prepares");
        let outcome = apply_edit(dir.path(), &prepared.temp, &prepared.base, false);
        assert_eq!(outcome, EditOutcome::Unchanged);
        assert!(!prepared.temp.exists());
        assert_eq!(live_text(dir.path()), before);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn launcher_edit_a_change_made_while_editing_is_never_overwritten() {
        // Edge E7: a base that differs by a single byte is a conflict.
        let dir = tempfile::tempdir().expect("temp dir");
        write_run(dir.path(), &clean_body());
        let prepared = prepare_edit(dir.path()).expect("prepares");
        std::fs::write(
            &prepared.temp,
            clean_body().replace("Prove it", "My edit of the title"),
        )
        .unwrap();

        // A human (or the driver's splice) saves while the editor is open.
        let racing = clean_body().replace("It works.", "It works!");
        crate::driver::direction::write_body(dir.path(), |_| racing.clone()).unwrap();
        let live_before = live_text(dir.path());

        let outcome = apply_edit(dir.path(), &prepared.temp, &prepared.base, false);
        assert_eq!(outcome, EditOutcome::Conflict);
        assert!(prepared.temp.exists(), "the user's text is kept");
        assert_eq!(
            live_text(dir.path()),
            live_before,
            "the racing change stands"
        );
        assert_eq!(
            std::fs::read_to_string(&prepared.temp).unwrap(),
            clean_body().replace("Prove it", "My edit of the title")
        );
    }

    /// The second comparison, inside the write: `apply_edit`'s pre-check passes, then a
    /// human save lands while `write_body` waits for `DIRECTION.lock`. The closure sees
    /// the new body under the lock, writes nothing, and the racing change stands.
    /// (If the scheduler delays the apply past the save, the pre-check reports the same
    /// conflict, so the assertion cannot flake; it only exercises the inner check in
    /// the common case.)
    #[cfg(target_os = "linux")]
    #[test]
    fn launcher_edit_a_save_landing_while_the_write_waits_for_the_lock_is_kept() {
        use crate::driver::direction::direction_lock;
        let dir = tempfile::tempdir().expect("temp dir");
        let block = write_run(dir.path(), &clean_body());
        let prepared = prepare_edit(dir.path()).expect("prepares");
        std::fs::write(
            &prepared.temp,
            clean_body().replace("Prove it", "My edit of the title"),
        )
        .unwrap();
        let racing = clean_body().replace("It works.", "It works!");

        let (taken_tx, taken_rx) = std::sync::mpsc::channel();
        let run_dir = dir.path().to_path_buf();
        let racing_for_holder = racing.clone();
        let block_for_holder = block.clone();
        let holder = std::thread::spawn(move || {
            let guard = direction_lock(&run_dir).expect("the holder takes the lock");
            taken_tx.send(()).expect("signal");
            std::thread::sleep(std::time::Duration::from_millis(400));
            // The human's save, made while the lock is held by "another writer".
            std::fs::write(
                run_dir.join("DIRECTION.md"),
                format!("{block_for_holder}{racing_for_holder}"),
            )
            .expect("the racing save");
            drop(guard);
        });
        taken_rx.recv().expect("the lock is held");

        let outcome = apply_edit(dir.path(), &prepared.temp, &prepared.base, false);
        holder.join().expect("holder thread");

        assert_eq!(outcome, EditOutcome::Conflict);
        assert!(prepared.temp.exists(), "the user's text is kept");
        let text = live_text(dir.path());
        assert_eq!(text, format!("{block}{racing}"), "the racing save stands");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn launcher_edit_an_insane_edit_and_a_lint_problem_edit_keep_the_temp() {
        let dir = tempfile::tempdir().expect("temp dir");
        write_run(dir.path(), &clean_body());
        let before = live_text(dir.path());
        let prepared = prepare_edit(dir.path()).expect("prepares");

        // No `## Items` any more.
        let insane = clean_body().replace("## Items", "## Things");
        std::fs::write(&prepared.temp, &insane).unwrap();
        assert_eq!(
            apply_edit(dir.path(), &prepared.temp, &prepared.base, false),
            EditOutcome::Insane
        );
        assert!(prepared.temp.exists());
        assert_eq!(live_text(dir.path()), before);

        // The template's placeholders are lint problems.
        std::fs::write(&prepared.temp, DIRECTION_TEMPLATE).unwrap();
        let outcome = apply_edit(dir.path(), &prepared.temp, &prepared.base, false);
        let EditOutcome::LintProblems(problems) = outcome else {
            panic!("the placeholders must be reported, got {outcome:?}");
        };
        assert!(!problems.is_empty());
        assert!(
            problems.iter().all(|p| p.starts_with("DIRECTION.md:")),
            "{problems:?}"
        );
        assert!(prepared.temp.exists());
        assert_eq!(live_text(dir.path()), before, "nothing was written");

        // The user's "apply it anyway".
        let outcome = apply_edit(dir.path(), &prepared.temp, &prepared.base, true);
        assert_eq!(
            outcome,
            EditOutcome::Applied {
                lock_timed_out: false
            }
        );
        assert!(!prepared.temp.exists());
        assert!(live_text(dir.path()).ends_with(DIRECTION_TEMPLATE));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn launcher_edit_a_pre_existing_temp_is_never_overwritten() {
        let dir = tempfile::tempdir().expect("temp dir");
        write_run(dir.path(), &clean_body());
        let temp = dir.path().join(EDIT_TEMP_NAME);
        std::fs::write(&temp, "the user's unsaved text").unwrap();
        assert_eq!(
            prepare_edit(dir.path()),
            Err(PrepareError::StaleTemp(temp.clone()))
        );
        assert_eq!(
            std::fs::read_to_string(&temp).unwrap(),
            "the user's unsaved text"
        );
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn read_direction_refuses_a_fifo_without_blocking() {
        use crate::state_reader::driver_frontmatter::fifo_test_support::{mkfifo, within_5s};
        let dir = tempfile::tempdir().expect("temp dir");
        mkfifo(&dir.path().join("DIRECTION.md"));
        let run = dir.path().to_path_buf();
        let result = within_5s(move || read_direction(&run));
        let why = result.expect_err("a FIFO is not read");
        assert!(why.contains("not a regular file"), "{why}");

        // A symlinked DIRECTION.md is refused too, even onto a valid body.
        let other = tempfile::tempdir().expect("temp dir");
        write_run(other.path(), &clean_body());
        let linked = tempfile::tempdir().expect("temp dir");
        std::os::unix::fs::symlink(
            other.path().join("DIRECTION.md"),
            linked.path().join("DIRECTION.md"),
        )
        .unwrap();
        assert!(read_direction(linked.path()).is_err());
        assert!(live_body(linked.path()).is_err());
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn launcher_edit_a_temp_that_is_a_symlink_is_refused() {
        let dir = tempfile::tempdir().expect("temp dir");
        write_run(dir.path(), &clean_body());
        let base = clean_body();
        // A link onto a perfectly good edited body: still not read.
        let outside = dir.path().join("elsewhere.md");
        std::fs::write(&outside, base.replace("Prove it", "changed")).unwrap();
        let temp = dir.path().join(EDIT_TEMP_NAME);
        std::os::unix::fs::symlink(&outside, &temp).unwrap();
        let before = live_text(dir.path());

        let outcome = apply_edit(dir.path(), &temp, &base, false);
        let EditOutcome::Failed(why) = outcome else {
            panic!("a symlinked temp must fail, got {outcome:?}");
        };
        assert!(why.contains("not a regular file"), "{why}");
        assert_eq!(live_text(dir.path()), before, "nothing was written");
        assert!(temp.symlink_metadata().is_ok(), "the link is kept");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn launcher_edit_a_planted_symlink_is_a_stale_temp_and_never_followed() {
        let dir = tempfile::tempdir().expect("temp dir");
        write_run(dir.path(), &clean_body());
        let outside = dir.path().join("elsewhere.txt");
        std::fs::write(&outside, "do not touch").unwrap();
        let temp = dir.path().join(EDIT_TEMP_NAME);
        std::os::unix::fs::symlink(&outside, &temp).unwrap();

        // prepare: O_EXCL fails on the link itself, and the target is untouched.
        assert_eq!(
            prepare_edit(dir.path()),
            Err(PrepareError::StaleTemp(temp.clone()))
        );
        assert_eq!(std::fs::read_to_string(&outside).unwrap(), "do not touch");

        // apply: a link is not a regular file, so nothing is read through it.
        let before = live_text(dir.path());
        let outcome = apply_edit(dir.path(), &temp, &clean_body(), false);
        let EditOutcome::Failed(why) = outcome else {
            panic!("a symlinked temp must fail, got {outcome:?}");
        };
        assert!(why.contains("not a regular file"), "{why}");
        assert!(temp.symlink_metadata().is_ok(), "the link is kept");
        assert_eq!(live_text(dir.path()), before);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn launcher_edit_the_temp_is_private_and_a_huge_or_binary_temp_is_refused() {
        use std::os::unix::fs::PermissionsExt;
        let dir = tempfile::tempdir().expect("temp dir");
        write_run(dir.path(), &clean_body());
        let prepared = prepare_edit(dir.path()).expect("prepares");
        let mode = std::fs::metadata(&prepared.temp)
            .unwrap()
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode & 0o077, 0, "no group or other access: {mode:o}");
        assert_eq!(mode & 0o600, 0o600);

        std::fs::write(
            &prepared.temp,
            vec![b'x'; (EDIT_TEMP_MAX_BYTES + 1) as usize],
        )
        .unwrap();
        let outcome = apply_edit(dir.path(), &prepared.temp, &prepared.base, true);
        assert!(
            matches!(&outcome, EditOutcome::Failed(why) if why.contains("larger than 1 MiB")),
            "{outcome:?}"
        );
        std::fs::write(&prepared.temp, [0xff, 0xfe, 0xfd]).unwrap();
        let outcome = apply_edit(dir.path(), &prepared.temp, &prepared.base, true);
        assert!(
            matches!(&outcome, EditOutcome::Failed(why) if why.contains("UTF-8")),
            "{outcome:?}"
        );
        assert!(prepared.temp.exists(), "a refused temp is kept");
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn launcher_edit_prepare_reports_an_unreadable_file_and_discard_removes_only_the_temp() {
        let dir = tempfile::tempdir().expect("temp dir");
        assert!(matches!(
            prepare_edit(dir.path()),
            Err(PrepareError::Read(_))
        ));
        write_run(dir.path(), &clean_body());
        let prepared = prepare_edit(dir.path()).expect("prepares");
        assert_eq!(discard_edit(dir.path()), Ok(()));
        assert!(!prepared.temp.exists());
        assert!(dir.path().join("DIRECTION.md").exists());
        assert_eq!(discard_edit(dir.path()), Ok(()), "an absent temp is fine");
        assert_eq!(live_body(dir.path()).as_deref(), Ok(clean_body().as_str()));
    }
}
