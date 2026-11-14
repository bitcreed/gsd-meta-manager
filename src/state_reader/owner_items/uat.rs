//! The UAT row parser and `categorize_item`, ported from gsd-core 1.21.0's
//! `uat.cjs` (`parseUatItemsWithStats`, `categorizeItem`).
//!
//! Rows are column-0 `### N.` headings found fence-aware (through
//! [`markdown::tokenize_headings`]), with inline and block-scalar `result:`,
//! `reason:`, `blocked_by:` and `expected:` fields, the parse-gap tallies (fence
//! shortfall, unowned indented rows, unterminated fence, rows without a result), the
//! `## Gaps` bullets and tables, and the resolved-issue skip.
//!
//! JavaScript's character classes are reproduced where they decide an outcome:
//! `\w` is ASCII, `\s` is [`js_space`], and `.` excludes four line terminators.

use super::{markdown, AuditItem, Category, Origin};

/// What one parse of a UAT body produced.
#[derive(Debug, Default, PartialEq)]
pub(super) struct UatParse {
    pub items: Vec<AuditItem>,
    /// Row headings that yielded neither an item nor a recognized pass (upstream's
    /// `headingsSeen`).
    pub headings_seen: usize,
}

/// JavaScript's `\s` (and `String.prototype.trim`'s whitespace).
pub(super) fn js_space(c: char) -> bool {
    matches!(
        c,
        '\t' | '\n' | '\u{b}' | '\u{c}' | '\r' | ' ' | '\u{a0}' | '\u{1680}' | '\u{2000}'
            ..='\u{200a}'
                | '\u{2028}'
                | '\u{2029}'
                | '\u{202f}'
                | '\u{205f}'
                | '\u{3000}'
                | '\u{feff}'
    )
}

pub(super) fn js_trim(text: &str) -> &str {
    text.trim_matches(js_space)
}

/// A JavaScript `.` does not match these.
pub(super) fn line_terminator(c: char) -> bool {
    matches!(c, '\n' | '\r' | '\u{2028}' | '\u{2029}')
}

fn word_char(c: char) -> bool {
    c.is_ascii_alphanumeric() || c == '_'
}

/// `\r\n` and a lone `\r` become `\n`, as upstream's `normalizeLineEndings`.
pub(super) fn normalize_line_endings(text: &str) -> String {
    text.replace("\r\n", "\n").replace('\r', "\n")
}

/// Whether `text` contains `needle` ignoring ASCII case (JS `/needle/i` without `u`).
fn contains_ci(text: &str, needle: &str) -> bool {
    text.to_ascii_lowercase().contains(needle)
}

/// `/third.party|twilio|stripe/i`.
fn is_third_party(text: &str) -> bool {
    let lower = text.to_ascii_lowercase();
    if lower.contains("twilio") || lower.contains("stripe") {
        return true;
    }
    lower.match_indices("third").any(|(at, _)| {
        let mut rest = lower[at + "third".len()..].chars();
        rest.next().is_some_and(|c| !line_terminator(c)) && rest.as_str().starts_with("party")
    })
}

/// `/^["']?deferred follow-up\b/i`.
fn is_deferred_follow_up(reason: &str) -> bool {
    let lower = reason.to_ascii_lowercase();
    let rest = lower
        .strip_prefix('"')
        .or_else(|| lower.strip_prefix('\''))
        .unwrap_or(&lower);
    rest.strip_prefix("deferred follow-up")
        .is_some_and(|after| !after.chars().next().is_some_and(word_char))
}

/// Port of upstream's `categorizeItem`.
pub(super) fn categorize_item(
    result: &str,
    reason: Option<&str>,
    blocked_by: Option<&str>,
) -> Category {
    let result = result.to_lowercase();
    let blocked_by = blocked_by.filter(|text| !text.is_empty());
    let reason = reason.filter(|text| !text.is_empty());
    if result == "blocked" || blocked_by.is_some() {
        if let Some(by) = blocked_by {
            if contains_ci(by, "server") {
                return Category::ServerBlocked;
            }
            if contains_ci(by, "device") || contains_ci(by, "physical") {
                return Category::DeviceNeeded;
            }
            if contains_ci(by, "build") || contains_ci(by, "release") || contains_ci(by, "preview")
            {
                return Category::BuildNeeded;
            }
            if is_third_party(by) {
                return Category::ThirdParty;
            }
        }
        return Category::Blocked;
    }
    if result == "skipped" {
        if let Some(reason) = reason {
            if is_deferred_follow_up(reason) {
                return Category::Deferred;
            }
            if contains_ci(reason, "server")
                || contains_ci(reason, "not running")
                || contains_ci(reason, "not available")
            {
                return Category::ServerBlocked;
            }
            if contains_ci(reason, "simulator")
                || contains_ci(reason, "physical")
                || contains_ci(reason, "device")
            {
                return Category::DeviceNeeded;
            }
            if contains_ci(reason, "build")
                || contains_ci(reason, "release")
                || contains_ci(reason, "preview")
            {
                return Category::BuildNeeded;
            }
        }
        return Category::SkippedUnresolved;
    }
    match result.as_str() {
        "pending" => Category::Pending,
        "human_needed" => Category::HumanUat,
        "issue" => Category::Issue,
        _ => Category::Unknown,
    }
}

/// `isTestRowHeadingText` plus `parseTestRowHeadingText`: `(number, name)` of a
/// heading text such as `3. Login` (`^\d+\.(?!\d)`, then `^(\d+)\.\s*(.*)$`). The name
/// falls back to the whole text when empty.
fn parse_row_text(text: &str) -> Option<(String, String)> {
    let digits = text.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return None;
    }
    let after = text[digits..].strip_prefix('.')?;
    if after.bytes().next().is_some_and(|b| b.is_ascii_digit()) {
        return None;
    }
    let name = js_trim(after);
    let name = if name.is_empty() { js_trim(text) } else { name };
    let number = text[..digits].trim_start_matches('0');
    let number = if number.is_empty() { "0" } else { number };
    Some((number.to_owned(), name.to_owned()))
}

/// `INDENTED_FENCE_DELIM_RE` (`^ {1,3}` then a fence run) when `indented`, else
/// `INDENT_TOLERANT_DELIM_RE` (`^ {0,3}`).
fn is_delimiter_shaped(line: &str, indented: bool) -> bool {
    let line = line.strip_suffix('\r').unwrap_or(line);
    let spaces = line.bytes().take_while(|b| *b == b' ').count();
    if spaces > 3 || (indented && spaces == 0) {
        return false;
    }
    let rest = &line[spaces..];
    ["```", "~~~"].iter().any(|run| rest.starts_with(run))
}

/// Port of `blankIndentedFenceDelimiters`: the lines with the two delimiter lines of
/// every wholly indented fenced block (and any delimiter-shaped line between them)
/// overwritten by spaces. Column 0 is structure and indentation is content, so only
/// delimiters are blanked, never a block's body.
fn blank_indented_fence_delimiters(lines: &[&str]) -> Vec<String> {
    let mut blank = std::collections::BTreeSet::new();
    for block in markdown::scan_fenced_blocks(lines) {
        if !is_delimiter_shaped(lines[block.open_line], true) {
            continue;
        }
        if block
            .close_line
            .is_some_and(|close| !is_delimiter_shaped(lines[close], true))
        {
            continue;
        }
        blank.insert(block.open_line);
        let inner = block.close_line.unwrap_or(lines.len());
        if let Some(close) = block.close_line {
            blank.insert(close);
        }
        for (index, line) in lines
            .iter()
            .enumerate()
            .take(inner)
            .skip(block.open_line + 1)
        {
            if is_delimiter_shaped(line, false) {
                blank.insert(index);
            }
        }
    }
    lines
        .iter()
        .enumerate()
        .map(|(index, line)| {
            if blank.contains(&index) {
                " ".repeat(line.chars().count())
            } else {
                (*line).to_owned()
            }
        })
        .collect()
}

/// `RESULT_LINE_RE`: `^result:\s*\[?(\w+)\]?[^]*$` with the `i` flag. The lower-cased
/// token, or `None`.
fn result_token(line: &str) -> Option<String> {
    let head = line.get(..7)?;
    if !head.eq_ignore_ascii_case("result:") {
        return None;
    }
    let rest = line[7..].trim_start_matches(js_space);
    let rest = rest.strip_prefix('[').unwrap_or(rest);
    let token: String = rest.chars().take_while(|c| word_char(*c)).collect();
    (!token.is_empty()).then(|| token.to_ascii_lowercase())
}

/// The block-scalar kind of a header line `key: |`, if it is one.
fn block_scalar_header(line: &str, key: &str) -> Option<char> {
    let rest = line.strip_prefix(key)?.strip_prefix(':')?;
    let rest = rest.trim_start_matches([' ', '\t']);
    let mut chars = rest.chars();
    let style = chars.next().filter(|c| matches!(c, '|' | '>'))?;
    let mut tail = chars.as_str();
    let indicator_len = {
        let bytes = tail.as_bytes();
        match bytes.first() {
            Some(b'1'..=b'9') => 1 + usize::from(matches!(bytes.get(1), Some(b'+' | b'-'))),
            Some(b'+' | b'-') => 1 + usize::from(matches!(bytes.get(1), Some(b'1'..=b'9'))),
            _ => 0,
        }
    };
    tail = &tail[indicator_len..];
    let tail = tail.trim_start_matches([' ', '\t']);
    (tail.is_empty() || tail.starts_with('#')).then_some(style)
}

/// A column-0 `key:` line followed by whitespace (`^\w[\w-]*:\s`).
fn is_key_line(line: &str, is_last: bool) -> bool {
    let mut chars = line.chars();
    if !chars.next().is_some_and(word_char) {
        return false;
    }
    let name_len = line
        .chars()
        .take_while(|c| word_char(*c) || *c == '-')
        .map(char::len_utf8)
        .sum::<usize>();
    let Some(after) = line[name_len..].strip_prefix(':') else {
        return false;
    };
    match after.chars().next() {
        Some(c) => js_space(c),
        // The line ends here: a following newline is the `\s`, unless it is the last.
        None => !is_last,
    }
}

fn fold_scalar_body(body: &str) -> String {
    let mut paragraphs = Vec::new();
    let mut current: Vec<&str> = Vec::new();
    for line in body.split('\n') {
        if line.is_empty() {
            paragraphs.push(current.join(" "));
            current.clear();
        } else {
            current.push(line);
        }
    }
    paragraphs.push(current.join(" "));
    paragraphs.join("\n")
}

/// Port of upstream's `extractScalarField`: a block scalar (`|` or `>`) or a plain
/// inline value of the line-start `key:` in `block`.
fn extract_scalar_field(block: &str, key: &str) -> Option<String> {
    let lines: Vec<&str> = block.split('\n').collect();
    let last = lines.len() - 1;
    let openers: Vec<(usize, char)> = lines
        .iter()
        .enumerate()
        .filter(|(index, _)| *index < last)
        .filter_map(|(index, line)| block_scalar_header(line, key).map(|style| (index, style)))
        .collect();
    let end_of_body =
        |from: usize| (from..lines.len()).find(|index| is_key_line(lines[*index], *index == last));
    let chosen = openers
        .iter()
        .find_map(|(index, style)| {
            end_of_body(index + 1).map(|end| (*style, lines[index + 1..end].to_vec()))
        })
        .or_else(|| {
            openers.first().and_then(|(index, style)| {
                let rest = &lines[index + 1..];
                (!(rest.len() == 1 && rest[0].is_empty())).then(|| (*style, rest.to_vec()))
            })
        });
    if let Some((style, body)) = chosen {
        let dedented = body
            .iter()
            .map(|line| line.strip_prefix("  ").unwrap_or(line))
            .collect::<Vec<_>>()
            .join("\n");
        let dedented = js_trim(&dedented);
        return Some(if style == '>' {
            fold_scalar_body(dedented)
        } else {
            dedented.to_owned()
        });
    }
    inline_field(block, key)
}

/// `^key:\s*(.+)\s*$` (multiline), first match, trimmed.
fn inline_field(block: &str, key: &str) -> Option<String> {
    let prefix = format!("{key}:");
    let mut line_start = 0;
    loop {
        if block[line_start..].starts_with(&prefix) {
            let value_from = line_start + prefix.len();
            let tail = &block[value_from..];
            let skipped = tail.len() - tail.trim_start_matches(js_space).len();
            let after = &tail[skipped..];
            match after.chars().next() {
                Some(c) if !line_terminator(c) => {
                    let end = after.find(line_terminator).unwrap_or(after.len());
                    return Some(js_trim(&after[..end]).to_owned());
                }
                Some(_) => {}
                None => {
                    // Only whitespace follows: `.+` can still take a blank that
                    // is not a line terminator, giving an empty value.
                    if tail[..skipped].chars().any(|c| !line_terminator(c)) {
                        return Some(String::new());
                    }
                }
            }
        }
        line_start += block[line_start..].find('\n')? + 1;
    }
}

fn non_empty(value: Option<String>) -> Option<String> {
    value.filter(|text| !text.is_empty())
}

/// `FENCE_OPENER_RE`: a fence opener at column 0 (three or more backticks or tildes).
fn is_column_0_fence_opener(line: &str) -> bool {
    line.starts_with("```") || line.starts_with("~~~")
}

/// `dropTopLevelFencedRegions`: `raw_lines` with every column-0 fenced region and its
/// delimiter lines removed (raw text kept everywhere else).
fn drop_top_level_fenced_regions(raw_lines: &[&str]) -> String {
    let mut kept: Vec<&str> = Vec::new();
    let mut open: Option<(char, usize)> = None;
    for line in raw_lines {
        let plain = line.strip_suffix('\r').unwrap_or(line);
        let run_char = plain.chars().next().filter(|c| matches!(c, '`' | '~'));
        let delimiter = run_char.and_then(|ch| {
            let len = plain.chars().take_while(|c| *c == ch).count();
            let trailing = &plain[len..];
            (len >= 3 && !trailing.chars().any(line_terminator)).then_some((ch, len, trailing))
        });
        if let Some((ch, len, trailing)) = delimiter {
            match open {
                None => {
                    if ch == '`' && trailing.contains('`') {
                        kept.push(line);
                        continue;
                    }
                    open = Some((ch, len));
                }
                Some((open_ch, open_len)) => {
                    if ch == open_ch && len >= open_len && trailing.chars().all(js_space) {
                        open = None;
                    }
                }
            }
            continue;
        }
        if open.is_none() {
            kept.push(line);
        }
    }
    kept.join("\n")
}

/// `clipBlockAtFirstFence`: the raw block bounded to the part the heading tokenizer
/// also treated as visible, so a row cannot read a later row's `expected:` through a
/// fence.
fn clip_block_at_first_fence(block: &str) -> String {
    let raw_lines: Vec<&str> = block.split('\n').collect();
    let Some(first_fence) = raw_lines.iter().position(|l| is_column_0_fence_opener(l)) else {
        return block.to_owned();
    };
    let before_fence = raw_lines[..first_fence].join("\n");
    if non_empty(extract_scalar_field(&before_fence, "expected")).is_some() {
        return before_fence;
    }
    let visible = drop_top_level_fenced_regions(&raw_lines);
    if non_empty(extract_scalar_field(&visible, "expected")).is_some() {
        return visible;
    }
    before_fence
}

/// `^#{3}(?!#)[ \t]+\d+\.(?!\d)` after the optional indent: whether `line` is shaped
/// like a `### N.` row heading. `indented` selects `INDENTED_TEST_HEADING_LINE_RE`
/// (`^[ \t]+` first) over `TEST_HEADING_LINE_RE` (column 0).
fn is_test_heading_line(line: &str, indented: bool) -> bool {
    let body = line.trim_start_matches([' ', '\t']);
    if indented == (body.len() == line.len()) {
        return false;
    }
    let Some(after) = body.strip_prefix("###") else {
        return false;
    };
    if after.starts_with('#') {
        return false;
    }
    let after_space = after.trim_start_matches([' ', '\t']);
    if after_space.len() == after.len() {
        return false;
    }
    let digits = after_space.bytes().take_while(u8::is_ascii_digit).count();
    let Some(rest) = after_space[digits..].strip_prefix('.') else {
        return false;
    };
    digits > 0 && !rest.bytes().next().is_some_and(|b| b.is_ascii_digit())
}

/// `ANY_KEY_SCALAR_HEADER_LINE_RE`: `^[A-Za-z_][\w-]*:` then a block-scalar opener.
fn is_any_key_scalar_header(line: &str) -> bool {
    let mut chars = line.chars();
    if !chars
        .next()
        .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
    {
        return false;
    }
    let name_len = line
        .chars()
        .take_while(|c| word_char(*c) || *c == '-')
        .map(char::len_utf8)
        .sum::<usize>();
    block_scalar_header(line, &line[..name_len]).is_some()
}

/// `countUnattributedIndentedRows`: indented `### N.` lines that no preceding
/// block-scalar header owns. One forward pass tracks the nearest preceding non-blank
/// column-0 line.
fn count_unattributed_indented_rows(lines: &[&str]) -> usize {
    let mut owner: Option<usize> = None;
    let mut count = 0;
    for (index, line) in lines.iter().enumerate() {
        if is_test_heading_line(line, true) {
            let owned = owner.is_some_and(|at| is_any_key_scalar_header(lines[at]));
            if !owned {
                count += 1;
            }
        }
        if !js_trim(line).is_empty() && !line.starts_with([' ', '\t']) {
            owner = Some(index);
        }
    }
    count
}

// ---------------------------------------------------------------------------
// `## Gaps`
// ---------------------------------------------------------------------------

/// `HYPHEN_BULLET_MARKERS.open` (`^(\s*)(-)\s`): the indent of a hyphen list opener.
fn list_opener_indent(line: &str) -> Option<usize> {
    let body = line.trim_start_matches(js_space);
    let indent = line.chars().count() - body.chars().count();
    let rest = body.strip_prefix('-')?;
    rest.chars().next().is_some_and(js_space).then_some(indent)
}

/// `HYPHEN_BULLET_MARKERS.strip` (`^(\s*)-\s+(.*)$`): the text after the marker.
fn strip_hyphen_marker(line: &str) -> Option<&str> {
    let rest = line.trim_start_matches(js_space).strip_prefix('-')?;
    let body = rest.trim_start_matches(js_space);
    if body.len() == rest.len() || body.chars().any(line_terminator) {
        return None;
    }
    Some(body)
}

/// `splitGapsEntries` for the hyphen grammar: top-level `- ` openers start entries;
/// deeper lines fold into the open one; separator lines (`- - -`) between entries are
/// skipped; lines before the first opener are dropped.
fn split_gaps_entries(body: &str) -> Vec<Vec<&str>> {
    let mut entries: Vec<Vec<&str>> = Vec::new();
    let mut current: Option<Vec<&str>> = None;
    let mut base_indent: Option<usize> = None;
    for line in body.split('\n') {
        let opener = list_opener_indent(line);
        if let Some(indent) = opener {
            let after_marker = line.trim_start_matches(js_space).strip_prefix('-');
            let remainder = after_marker.map_or("", |rest| {
                let mut chars = rest.chars();
                chars.next();
                chars.as_str()
            });
            let separator =
                remainder.chars().all(|c| c == '-' || js_space(c)) && remainder.contains('-');
            if separator && (current.is_none() || indent <= base_indent.unwrap_or(0)) {
                continue;
            }
        }
        if let Some(indent) = opener {
            let base = *base_indent.get_or_insert(indent);
            if indent <= base {
                entries.extend(current.take());
                current = Some(vec![line]);
                continue;
            }
        }
        if let Some(open) = current.as_mut() {
            open.push(line);
        }
    }
    entries.extend(current);
    entries
}

/// The fields of one entry that the parsers read, first occurrence of a key wins.
#[derive(Default)]
struct GapFields {
    status: Option<String>,
    truth: Option<String>,
    reason: Option<String>,
    test: Option<String>,
}

/// `parseGapEntryFieldLine`: a `key: value` line, the marker stripped on the
/// entry's first line only, a `**Key:**` form unwrapped and lower-cased.
fn gap_field_line(raw: &str, strip_marker: bool) -> Option<(String, String)> {
    let bare = if strip_marker {
        strip_hyphen_marker(raw).unwrap_or_else(|| js_trim(raw))
    } else {
        js_trim(raw)
    };
    let content = unwrap_bold_key(bare);
    let name_len = {
        let mut chars = content.chars();
        if !chars
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
        {
            return None;
        }
        1 + chars
            .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
            .count()
    };
    let rest = content[name_len..].strip_prefix(':')?;
    let value = rest.trim_start_matches(js_space);
    if value.chars().any(line_terminator) {
        return None;
    }
    let mut value = js_trim(value);
    if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
        value = &value[1..value.len() - 1];
    }
    Some((content[..name_len].to_owned(), value.to_owned()))
}

/// `^\*+([A-Za-z_][A-Za-z0-9_-]*):\*+` rewritten to `key:` with the key lower-cased.
fn unwrap_bold_key(text: &str) -> std::borrow::Cow<'_, str> {
    let unstarred = text.trim_start_matches('*');
    if unstarred.len() == text.len()
        || !unstarred
            .chars()
            .next()
            .is_some_and(|c| c.is_ascii_alphabetic() || c == '_')
    {
        return text.into();
    }
    let name_len = unstarred
        .chars()
        .take_while(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
        .count();
    let Some(after_colon) = unstarred[name_len..].strip_prefix(':') else {
        return text.into();
    };
    let tail = after_colon.trim_start_matches('*');
    if tail.len() == after_colon.len() {
        return text.into();
    }
    format!("{}:{tail}", unstarred[..name_len].to_ascii_lowercase()).into()
}

fn gap_fields(entry: &[&str]) -> GapFields {
    let mut fields = GapFields::default();
    for (index, line) in entry.iter().enumerate() {
        let Some((key, value)) = gap_field_line(line, index == 0) else {
            continue;
        };
        let slot = match key.as_str() {
            "status" => &mut fields.status,
            "truth" => &mut fields.truth,
            "reason" => &mut fields.reason,
            "test" => &mut fields.test,
            _ => continue,
        };
        slot.get_or_insert(value);
    }
    fields
}

/// `rawGapEntryText`: the display fallback of an entry without a `truth:` field.
fn raw_gap_entry_text(entry: &[&str]) -> String {
    let joined: Vec<&str> = entry
        .iter()
        .enumerate()
        .map(|(index, line)| {
            if index == 0 {
                strip_hyphen_marker(line).unwrap_or(line)
            } else {
                js_trim(line)
            }
        })
        .collect();
    js_trim(&joined.join(" ")).to_owned()
}

/// `/^\d+$/`, as the test number a gap names (leading zeros dropped, as `parseInt`).
fn gap_test_number(value: Option<&str>) -> Option<String> {
    let value = value.filter(|v| !v.is_empty() && v.bytes().all(|b| b.is_ascii_digit()))?;
    let trimmed = value.trim_start_matches('0');
    Some(if trimmed.is_empty() { "0" } else { trimmed }.to_owned())
}

/// The body of the first level-2 `Gaps` section.
fn gaps_section_body(lines: &[&str]) -> Option<String> {
    markdown::collect_section(
        lines,
        |heading| heading.level == 2 && heading.text.eq_ignore_ascii_case("gaps"),
        true,
    )
    .map(|section| section.body)
}

/// One GFM table of a section body: its header row, if a delimiter row followed it,
/// and its data rows.
struct GapsTable {
    header: Option<Vec<String>>,
    rows: Vec<Vec<String>>,
}

/// `collectTableRows`: a header is the row a delimiter row follows, a prose or blank
/// line ends the table, a headerless table keeps every row.
fn collect_table_rows(body: &str) -> Vec<GapsTable> {
    let mut tables = Vec::new();
    let mut current: Option<GapsTable> = None;
    let mut pending: Option<Vec<String>> = None;
    fn ensure(current: &mut Option<GapsTable>) -> &mut GapsTable {
        current.get_or_insert_with(|| GapsTable {
            header: None,
            rows: Vec::new(),
        })
    }
    for raw in body.split('\n') {
        let line = js_trim(raw);
        if !line.starts_with('|') {
            if let Some(row) = pending.take() {
                ensure(&mut current).rows.push(row);
            }
            tables.extend(current.take());
            continue;
        }
        let cells = markdown::split_table_row(line);
        if markdown::is_delimiter_row(&cells) {
            ensure(&mut current).header = pending.take();
            continue;
        }
        if let Some(row) = pending.take() {
            ensure(&mut current).rows.push(row);
        }
        pending = Some(cells);
    }
    if let Some(row) = pending.take() {
        ensure(&mut current).rows.push(row);
    }
    tables.extend(current);
    tables
}

/// Column positions by canonical Gaps field (`GAPS_COLUMN_ALIASES`).
#[derive(Default)]
struct GapsColumns {
    truth: Option<usize>,
    status: Option<usize>,
    reason: Option<usize>,
    test: Option<usize>,
    /// Any column mapped, including fields the parsers do not read (`severity`).
    any: bool,
}

fn map_gaps_header(header: &[String]) -> Option<GapsColumns> {
    let mut columns = GapsColumns::default();
    for (index, cell) in header.iter().enumerate() {
        let name = js_trim(cell).to_lowercase().replace('*', "");
        let slot = match name.as_str() {
            "truth" | "gap" | "finding" | "item" | "description" | "issue" | "name" => {
                Some(&mut columns.truth)
            }
            "status" | "result" | "state" => Some(&mut columns.status),
            "reason" | "note" | "notes" | "detail" | "details" | "evidence" => {
                Some(&mut columns.reason)
            }
            "test" | "#" | "test #" | "test number" => Some(&mut columns.test),
            // Mapped upstream, never read.
            "severity" => {
                columns.any = true;
                None
            }
            // Own properties of a JavaScript object literal's prototype chain
            // resolve to a truthy value in upstream's alias lookup.
            "constructor" | "__proto__" => {
                columns.any = true;
                None
            }
            _ => None,
        };
        if let Some(slot) = slot {
            if slot.is_none() {
                *slot = Some(index);
            }
            columns.any = true;
        }
    }
    columns.any.then_some(columns)
}

/// `parseGapsTableItems`: one item per data row of the section's tables.
fn parse_gaps_table_items(body: &str) -> Vec<AuditItem> {
    let mut items = Vec::new();
    for table in collect_table_rows(body) {
        let columns = table.header.as_deref().and_then(map_gaps_header);
        for cells in &table.rows {
            let at = |column: Option<usize>| -> String {
                column
                    .and_then(|index| cells.get(index))
                    .map(|cell| js_trim(cell).to_owned())
                    .unwrap_or_default()
            };
            let status_column = columns.as_ref().and_then(|c| c.status);
            let raw_status = at(status_column);
            if raw_status.eq_ignore_ascii_case("resolved") {
                continue;
            }
            if status_column.is_none()
                && cells
                    .iter()
                    .any(|cell| js_trim(cell).eq_ignore_ascii_case("resolved"))
            {
                continue;
            }
            let truth = at(columns.as_ref().and_then(|c| c.truth));
            let reason = at(columns.as_ref().and_then(|c| c.reason));
            let test = at(columns.as_ref().and_then(|c| c.test));
            let name = if truth.is_empty() {
                cells
                    .iter()
                    .filter(|cell| !cell.is_empty())
                    .map(String::as_str)
                    .collect::<Vec<_>>()
                    .join(" \u{2014} ")
            } else {
                truth
            };
            if name.is_empty() {
                continue;
            }
            let status = if raw_status.is_empty() {
                "unknown".to_owned()
            } else {
                raw_status
            };
            let reason = non_empty(Some(reason));
            items.push(AuditItem {
                origin: Origin::GapTable,
                test: gap_test_number(Some(&test)),
                name,
                expected: None,
                category: categorize_item(&status, reason.as_deref(), None),
                result: status,
                reason,
                blocked_by: None,
            });
        }
    }
    items
}

/// The gap entries of a body, read once: the `## Gaps` items (bullets, then tables) and
/// the test numbers the section records as resolved.
struct GapsReading {
    items: Vec<AuditItem>,
    resolved_tests: std::collections::BTreeSet<String>,
}

/// `parseGapsItems` plus `isTestIssueResolvedInGaps`'s data.
fn read_gaps(lines: &[&str]) -> GapsReading {
    let mut reading = GapsReading {
        items: Vec::new(),
        resolved_tests: std::collections::BTreeSet::new(),
    };
    let Some(body) = gaps_section_body(lines) else {
        return reading;
    };
    for entry in split_gaps_entries(&body) {
        let fields = gap_fields(&entry);
        let raw_status = fields.status.as_deref().filter(|s| !s.is_empty());
        let resolved = raw_status.is_some_and(|s| s.to_lowercase() == "resolved");
        if resolved {
            reading
                .resolved_tests
                .extend(gap_test_number(fields.test.as_deref()));
            continue;
        }
        let status = raw_status.unwrap_or("unknown").to_owned();
        let reason = non_empty(fields.reason);
        let name = non_empty(fields.truth).unwrap_or_else(|| raw_gap_entry_text(&entry));
        reading.items.push(AuditItem {
            origin: Origin::Gap,
            test: gap_test_number(fields.test.as_deref()),
            name,
            expected: None,
            category: categorize_item(&status, reason.as_deref(), None),
            result: status,
            reason,
            blocked_by: None,
        });
    }
    reading.items.extend(parse_gaps_table_items(&body));
    reading
}

/// Parse the `### N.` rows and the `## Gaps` entries of a UAT body.
///
/// A row is a column-0 level-3 heading outside every fence whose text starts with
/// `N.`; its block runs to the next column-0 heading of any level. Delimiters of
/// wholly indented fences are neutralized first so an indented fence inside an
/// `expected: |` value cannot hide a later column-0 row.
pub(super) fn parse_uat_items_with_stats(text: &str) -> UatParse {
    let text = normalize_line_endings(text);
    let lines: Vec<&str> = text.split('\n').collect();
    let blanked = blank_indented_fence_delimiters(&lines);
    let blanked_refs: Vec<&str> = blanked.iter().map(String::as_str).collect();
    let headings: Vec<markdown::Heading> = markdown::tokenize_headings(&blanked_refs)
        .into_iter()
        .filter(|heading| lines[heading.line].starts_with('#'))
        .collect();
    let mut parse = UatParse::default();
    let rows = headings
        .iter()
        .filter(|heading| heading.level == 3 && parse_row_text(&heading.text).is_some())
        .count();
    // Row-shaped lines the tokenizer hid (a fence straddling a row), whole document on
    // both sides of the comparison.
    let shaped = lines
        .iter()
        .filter(|line| is_test_heading_line(line, false))
        .count();
    parse.headings_seen += shaped.saturating_sub(rows);
    parse.headings_seen += count_unattributed_indented_rows(&lines);
    if markdown::strip_fenced_code(&lines).1 {
        parse.headings_seen += 1;
    }
    let gaps = read_gaps(&lines);
    for (position, heading) in headings.iter().enumerate() {
        if heading.level != 3 {
            continue;
        }
        let Some((number, name)) = parse_row_text(&heading.text) else {
            continue;
        };
        let next = headings.get(position + 1).map(|next| next.line);
        let end = next.unwrap_or(lines.len());
        let mut block = lines[heading.line..end].join("\n");
        if next.is_some() {
            block.push('\n');
        }
        let (kept, unterminated) = markdown::strip_fenced_code(&lines[heading.line..end]);
        let visible = if unterminated {
            block.clone()
        } else {
            let mut joined = kept.join("\n");
            if next.is_some() {
                joined.push('\n');
            }
            joined
        };
        let Some(result) = visible.split('\n').find_map(result_token) else {
            parse.headings_seen += 1;
            continue;
        };
        if result == "pass" || result == "passed" {
            continue;
        }
        if result == "issue" && gaps.resolved_tests.contains(&number) {
            continue;
        }
        let expected = non_empty(extract_scalar_field(
            &clip_block_at_first_fence(&block),
            "expected",
        ));
        let reason = non_empty(extract_scalar_field(&visible, "reason"));
        let blocked_by = non_empty(extract_scalar_field(&visible, "blocked_by"));
        parse.items.push(AuditItem {
            origin: Origin::Row,
            test: Some(number),
            name,
            expected,
            category: categorize_item(&result, reason.as_deref(), blocked_by.as_deref()),
            result,
            reason,
            blocked_by,
        });
    }
    parse.items.extend(gaps.items);
    parse
}

#[cfg(test)]
mod tests {
    use super::*;

    fn cat(result: &str, reason: Option<&str>, blocked_by: Option<&str>) -> &'static str {
        categorize_item(result, reason, blocked_by).as_str()
    }

    #[test]
    fn categorize_blocked_and_each_blocked_by_keyword() {
        assert_eq!(cat("blocked", None, None), "blocked");
        assert_eq!(cat("BLOCKED", None, Some("")), "blocked");
        assert_eq!(
            cat("blocked", None, Some("the staging SERVER")),
            "server_blocked"
        );
        assert_eq!(
            cat("blocked", None, Some("a physical phone")),
            "device_needed"
        );
        assert_eq!(cat("blocked", None, Some("a Device lab")), "device_needed");
        assert_eq!(
            cat("blocked", None, Some("the release train")),
            "build_needed"
        );
        assert_eq!(
            cat("blocked", None, Some("a preview build")),
            "build_needed"
        );
        assert_eq!(
            cat("blocked", None, Some("Third-Party vendor")),
            "third_party"
        );
        assert_eq!(cat("blocked", None, Some("Twilio sandbox")), "third_party");
        assert_eq!(cat("blocked", None, Some("stripe keys")), "third_party");
        assert_eq!(cat("blocked", None, Some("something else")), "blocked");
        assert_eq!(cat("pending", None, Some("the server")), "server_blocked");
    }

    #[test]
    fn categorize_skipped_and_each_reason_keyword() {
        assert_eq!(cat("skipped", None, None), "skipped_unresolved");
        assert_eq!(
            cat("skipped", Some("Deferred follow-up: later"), None),
            "deferred"
        );
        assert_eq!(
            cat("skipped", Some("\"deferred follow-up\" x"), None),
            "deferred"
        );
        assert_eq!(
            cat("skipped", Some("deferred follow-ups are not it"), None),
            "skipped_unresolved"
        );
        assert_eq!(
            cat("skipped", Some("the app is not running"), None),
            "server_blocked"
        );
        assert_eq!(
            cat("skipped", Some("service not available"), None),
            "server_blocked"
        );
        assert_eq!(
            cat("skipped", Some("needs a simulator"), None),
            "device_needed"
        );
        assert_eq!(
            cat("skipped", Some("a physical thing"), None),
            "device_needed"
        );
        assert_eq!(
            cat("skipped", Some("on the release build"), None),
            "build_needed"
        );
        assert_eq!(
            cat(
                "skipped",
                Some("deferred follow-up on the release build"),
                None
            ),
            "deferred"
        );
        assert_eq!(cat("skipped", Some("no idea"), None), "skipped_unresolved");
    }

    #[test]
    fn categorize_the_remaining_results() {
        assert_eq!(cat("pending", None, None), "pending");
        assert_eq!(cat("human_needed", None, None), "human_uat");
        assert_eq!(cat("issue", None, None), "issue");
        assert_eq!(cat("failed", None, None), "unknown");
        assert_eq!(cat("partial", None, None), "unknown");
        assert_eq!(cat("", None, None), "unknown");
    }

    #[test]
    fn rows_keep_pending_and_drop_pass() {
        let text = "## Tests\n\n### 1. One\nexpected: a\nresult: pass\n\n### 2. Two\nexpected: b\nresult: pending\n\n### 3. Three\nresult: [pending]\n\n### 4. Four\nresult: PASSED\n";
        let parse = parse_uat_items_with_stats(text);
        let tests: Vec<_> = parse
            .items
            .iter()
            .map(|i| {
                (
                    i.test.as_deref().unwrap(),
                    i.result.as_str(),
                    i.name.as_str(),
                )
            })
            .collect();
        assert_eq!(tests, [("2", "pending", "Two"), ("3", "pending", "Three")]);
        assert_eq!(parse.items[0].expected.as_deref(), Some("b"));
        assert_eq!(parse.headings_seen, 0);
    }

    #[test]
    fn a_row_without_a_result_line_is_a_parse_gap() {
        let parse =
            parse_uat_items_with_stats("### 1. Draft\nexpected: x\n\n### 2. Done\nresult: pass\n");
        assert!(parse.items.is_empty());
        assert_eq!(parse.headings_seen, 1);
    }

    #[test]
    fn a_dotted_section_number_and_a_deeper_heading_are_not_rows() {
        let parse = parse_uat_items_with_stats(
            "### 1.2.3 Overview\nresult: pending\n\n#### 4. Deep\nresult: pending\n",
        );
        assert!(parse.items.is_empty());
        assert_eq!(parse.headings_seen, 0);
    }

    #[test]
    fn a_squished_or_nameless_heading_is_still_a_row() {
        let parse =
            parse_uat_items_with_stats("###\t3.Foo\nresult: pending\n\n### 4.\nresult: pending\n");
        let names: Vec<_> = parse.items.iter().map(|i| i.name.as_str()).collect();
        assert_eq!(names, ["Foo", "4."]);
    }

    #[test]
    fn a_block_runs_to_the_next_heading_of_any_level() {
        let text = "### 1. One\nresult: pending\n\n## Gaps\nreason: from the gaps section\n";
        let parse = parse_uat_items_with_stats(text);
        assert_eq!(parse.items[0].reason, None);
    }

    #[test]
    fn inline_fields_are_read_and_the_first_result_line_wins() {
        let text = "### 1. One\nresult: skipped # no device\nresult: pending\nreason: a physical device\nblocked_by: the server\nexpected: it works\n";
        let item = &parse_uat_items_with_stats(text).items[0];
        assert_eq!(item.result, "skipped");
        assert_eq!(item.reason.as_deref(), Some("a physical device"));
        assert_eq!(item.blocked_by.as_deref(), Some("the server"));
        assert_eq!(item.category, Category::ServerBlocked);
    }

    #[test]
    fn block_scalars_are_dedented_and_folded() {
        let text = "### 1. One\nresult: pending\nexpected: |\n  line one\n  line two\nreason: >\n  folded\n  text\n";
        let item = &parse_uat_items_with_stats(text).items[0];
        assert_eq!(item.expected.as_deref(), Some("line one\nline two"));
        assert_eq!(item.reason.as_deref(), Some("folded text"));
    }

    #[test]
    fn crlf_and_lone_cr_are_line_breaks() {
        let text = "### 1. One\r\nresult: pending\r### 2. Two\rresult: pending\r\n";
        assert_eq!(parse_uat_items_with_stats(text).items.len(), 2);
    }

    #[test]
    fn a_non_ascii_name_is_kept() {
        let item = &parse_uat_items_with_stats("### 1. Überprüfung ✓\nresult: pending\n").items[0];
        assert_eq!(item.name, "Überprüfung ✓");
    }

    #[test]
    fn a_row_heading_inside_a_fenced_example_is_not_a_row() {
        let text = "### 1. Real\nresult: pending\n\n```\n### 2. Fake\nresult: pending\n```\n\n### 3. After\nresult: pending\n";
        let parse = parse_uat_items_with_stats(text);
        let tests: Vec<_> = parse
            .items
            .iter()
            .map(|i| i.test.as_deref().unwrap())
            .collect();
        assert_eq!(tests, ["1", "3"]);
        let tilde = parse_uat_items_with_stats(
            "~~~\n### 1. Fake\nresult: pending\n~~~\n### 2. Real\nresult: pending\n",
        );
        assert_eq!(tilde.items.len(), 1);
        assert_eq!(tilde.items[0].test.as_deref(), Some("2"));
    }

    #[test]
    fn an_indented_row_heading_is_not_a_row() {
        let parse = parse_uat_items_with_stats(
            "    ### 1. Indented\nresult: pending\n### 2. Real\nresult: pending\n",
        );
        assert_eq!(parse.items.len(), 1);
        assert_eq!(parse.items[0].test.as_deref(), Some("2"));
    }

    #[test]
    fn an_indented_fence_in_a_value_does_not_hide_a_later_column_0_row() {
        let text = "### 1. One\nresult: pending\nexpected: |\n  ```\n  sample\n\n### 2. Two\nresult: pending\n";
        let parse = parse_uat_items_with_stats(text);
        let tests: Vec<_> = parse
            .items
            .iter()
            .map(|i| i.test.as_deref().unwrap())
            .collect();
        assert_eq!(tests, ["1", "2"]);
    }

    #[test]
    fn a_result_line_inside_a_fence_is_not_the_rows_result() {
        let text =
            "### 1. One\n```\nresult: pending\n```\nresult: skipped\nreason: a physical device\n";
        let item = &parse_uat_items_with_stats(text).items[0];
        assert_eq!(item.result, "skipped");
        assert_eq!(item.category, Category::DeviceNeeded);
    }

    fn tallies(text: &str) -> usize {
        parse_uat_items_with_stats(text).headings_seen
    }

    #[test]
    fn a_fence_straddling_two_rows_counts_the_hidden_row_once() {
        let text = "### 1. One\nresult: pending\n```\n### 2. Hidden\nresult: blocked\n```\n### 3. Three\nresult: pending\n";
        let parse = parse_uat_items_with_stats(text);
        assert_eq!(parse.items.len(), 2);
        assert_eq!(parse.headings_seen, 1);
    }

    #[test]
    fn an_indented_row_counts_unless_a_scalar_header_owns_it() {
        assert_eq!(tallies("prose\n  ### 5. Row\n  result: pending\n"), 1);
        assert_eq!(tallies("notes: |\n  ### 5. Row\n"), 0);
        assert_eq!(tallies("expected:\n  ### 5. Row\n"), 1);
        assert_eq!(tallies("notes:\n  ### 5. Row\n"), 1);
        assert_eq!(tallies("reason: >-\n\n  ### 5. Row\n"), 0);
    }

    #[test]
    fn an_unterminated_fence_and_a_row_without_a_result_each_count_once() {
        assert_eq!(tallies("### 1. One\nresult: pending\n```\nopen\n"), 1);
        assert_eq!(tallies("### 1. One\nexpected: x\n"), 1);
        assert_eq!(tallies("### 1. One\nresult: pending\n"), 0);
    }

    #[test]
    fn expected_stops_at_the_blocks_first_fence() {
        let text = "### 1. One\nresult: pending\nexpected: before\n```\nexpected: inside\n```\n";
        assert_eq!(
            parse_uat_items_with_stats(text).items[0]
                .expected
                .as_deref(),
            Some("before")
        );
        let late = "### 1. One\nresult: pending\n```\nexpected: inside\n```\nexpected: after\n";
        assert_eq!(
            parse_uat_items_with_stats(late).items[0]
                .expected
                .as_deref(),
            Some("after")
        );
        let hidden = "### 1. One\nresult: pending\n```\nexpected: inside\n```\n";
        assert_eq!(parse_uat_items_with_stats(hidden).items[0].expected, None);
    }

    #[test]
    fn a_multi_byte_row_and_gap_parse_without_slicing_a_character() {
        let text = "### 7. Überprüfung 確認 🔒\nresult: pending\nreason: 🔒 für Gäste\n\n## Gaps\n\n- truth: \"確認 ✓ fehlt\"\n  status: failed\n  test: 7\n";
        let parse = parse_uat_items_with_stats(text);
        assert_eq!(parse.items[0].name, "Überprüfung 確認 🔒");
        assert_eq!(parse.items[0].reason.as_deref(), Some("🔒 für Gäste"));
        assert_eq!(parse.items[1].name, "確認 ✓ fehlt");
        assert_eq!(parse.items[1].test.as_deref(), Some("7"));
    }

    fn gap_summary(text: &str) -> Vec<(Origin, Option<String>, String, String)> {
        parse_uat_items_with_stats(text)
            .items
            .into_iter()
            .filter(|i| i.origin != Origin::Row)
            .map(|i| (i.origin, i.test, i.name, i.result))
            .collect()
    }

    #[test]
    fn the_gaps_section_is_level_bounded_and_separators_are_skipped() {
        let text = "## Gaps\n\n<!-- template comment -->\n- truth: \"first\"\n  status: failed\n- - -\n- truth: \"second\"\n  status: open\n    - nested: item\n\n### Sub\n- truth: \"still inside\"\n  status: failed\n\n## After\n- truth: \"outside\"\n  status: failed\n";
        let names: Vec<_> = gap_summary(text).into_iter().map(|g| g.2).collect();
        assert_eq!(names, ["first", "second", "still inside"]);
    }

    #[test]
    fn gap_fields_unwrap_bold_keys_keep_the_first_and_strip_one_pair_of_quotes() {
        let text = "## gaps\n- **Status:** failed\n  truth: \"say \"hi\"\"\n  truth: second\n  test: 12\n- no fields here\n- truth: x\n  status: RESOLVED\n  test: 3\n";
        let got = gap_summary(text);
        assert_eq!(got.len(), 2);
        assert_eq!(got[0].2, "say \"hi\"");
        assert_eq!(got[0].1.as_deref(), Some("12"));
        assert_eq!(got[0].3, "failed");
        assert_eq!(got[1].2, "no fields here");
        assert_eq!(got[1].3, "unknown");
        assert_eq!(got[1].1, None);
    }

    #[test]
    fn a_non_numeric_test_value_is_no_test_number() {
        let text = "## Gaps\n- truth: a\n  status: failed\n  test: 2b\n";
        assert_eq!(gap_summary(text)[0].1, None);
    }

    #[test]
    fn a_resolved_gap_removes_only_the_matching_issue_row() {
        let text = "### 1. One\nresult: issue\n\n### 2. Two\nresult: issue\n\n### 3. Three\nresult: failed\n\n## Gaps\n- truth: a\n  status: resolved\n  test: 1\n- truth: b\n  status: resolved\n  test: 3\n- truth: c\n  status: open\n  test: 2\n";
        let parse = parse_uat_items_with_stats(text);
        let rows: Vec<_> = parse
            .items
            .iter()
            .filter(|i| i.origin == Origin::Row)
            .map(|i| i.test.as_deref().unwrap())
            .collect();
        assert_eq!(rows, ["2", "3"]);
        assert_eq!(gap_summary(text).len(), 1);
    }

    #[test]
    fn gap_tables_map_aliases_and_skip_resolved_rows() {
        let text = "## Gaps\n\n| **Finding** | State | Test # | Details |\n|---|---|---|---|\n| open one | failed | 4 | needs a physical device |\n| done one | Resolved | 5 | x |\n| no status | | | |\n";
        let parse = parse_uat_items_with_stats(text);
        assert_eq!(parse.items.len(), 2);
        assert_eq!(parse.items[0].origin, Origin::GapTable);
        assert_eq!(parse.items[0].test.as_deref(), Some("4"));
        assert_eq!(
            parse.items[0].reason.as_deref(),
            Some("needs a physical device")
        );
        assert_eq!(parse.items[1].result, "unknown");
        let headerless =
            parse_uat_items_with_stats("## Gaps\n| alpha | resolved |\n| beta | open |\n");
        assert_eq!(headerless.items.len(), 1);
        assert_eq!(headerless.items[0].name, "beta \u{2014} open");
    }

    #[test]
    fn bullet_and_table_gaps_are_unioned_bullets_first() {
        let text = "## Gaps\n| truth | status |\n|---|---|\n| tabled | failed |\n\n- truth: bulleted\n  status: failed\n";
        let origins: Vec<_> = gap_summary(text).into_iter().map(|g| g.0).collect();
        assert_eq!(origins, [Origin::Gap, Origin::GapTable]);
    }
}
