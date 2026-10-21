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
    blank_comments, control_re, item_re, parse_direction, section_heading, write_body, Section,
    SpliceOutcome,
};

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
            SpliceOutcome::SkippedInsaneBody => SteerOutcome::NotWritten(
                "the body lacks \"## Direction\" or \"## Items\"".to_string(),
            ),
            SpliceOutcome::GaveUp => SteerOutcome::NotWritten(
                "another save kept changing the file, so this one gave up".to_string(),
            ),
            SpliceOutcome::NoDirection => {
                SteerOutcome::NotWritten("the run has no DIRECTION.md".to_string())
            }
        },
    }
}

#[cfg(test)]
mod tests {
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
}
