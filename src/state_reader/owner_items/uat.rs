//! The UAT row parser and `categorize_item`, ported from gsd-core 1.21.0's
//! `uat.cjs` (`parseUatItemsWithStats`, `categorizeItem`).
//!
//! Rows are column-0 `### N.` headings found fence-aware (through
//! [`markdown::tokenize_headings`]), with inline and block-scalar `result:`,
//! `reason:`, `blocked_by:` and `expected:` fields. Plan 34-02 adds the `## Gaps`
//! bullets and tables, the resolved-gap skip and the shortfall tallies.
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

/// Parse the `### N.` rows of a UAT body.
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
        let expected = non_empty(extract_scalar_field(&block, "expected"));
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
}
