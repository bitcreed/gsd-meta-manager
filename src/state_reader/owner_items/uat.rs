//! The UAT row parser and `categorize_item`, ported from gsd-core 1.21.0's
//! `uat.cjs` (`parseUatItemsWithStats`, `categorizeItem`).
//!
//! This is the first form of the parser: column-0 `### N.` row headings, inline and
//! block-scalar `result:`, `reason:`, `blocked_by:` and `expected:` fields. Plan
//! 34-06 makes heading detection fence-aware and plan 34-02 adds the `## Gaps`
//! bullets and tables, the resolved-gap skip and the shortfall tallies.
//!
//! JavaScript's character classes are reproduced where they decide an outcome:
//! `\w` is ASCII, `\s` is [`js_space`], and `.` excludes four line terminators.

use super::{AuditItem, Category, Origin};

/// What one parse of a UAT body produced.
#[derive(Debug, Default, PartialEq)]
pub(super) struct UatParse {
    pub items: Vec<AuditItem>,
    /// Row headings that yielded neither an item nor a recognized pass (upstream's
    /// `headingsSeen`).
    pub headings_seen: usize,
}

/// JavaScript's `\s` (and `String.prototype.trim`'s whitespace).
fn js_space(c: char) -> bool {
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

fn js_trim(text: &str) -> &str {
    text.trim_matches(js_space)
}

/// A JavaScript `.` does not match these.
fn line_terminator(c: char) -> bool {
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

/// An ATX heading line at column 0: one to six `#` then a blank or the end.
fn is_heading_line(line: &str) -> bool {
    let hashes = line.bytes().take_while(|b| *b == b'#').count();
    (1..=6).contains(&hashes)
        && line[hashes..]
            .chars()
            .next()
            .is_none_or(|c| c == ' ' || c == '\t')
}

/// The text of a `### N. Name` row heading as `(number, name)`, or `None` when the
/// line is not a test-row heading (`TEST_HEADING_LINE_RE` plus
/// `^\d+\.(?!\d)` on the heading text).
fn parse_row_heading(line: &str) -> Option<(String, String)> {
    let rest = line.strip_prefix("###")?;
    if rest.starts_with('#') {
        return None;
    }
    let after_blank = rest.trim_start_matches([' ', '\t']);
    if after_blank.len() == rest.len() {
        return None;
    }
    let digits = after_blank.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return None;
    }
    let after_digits = after_blank[digits..].strip_prefix('.')?;
    if after_digits
        .bytes()
        .next()
        .is_some_and(|b| b.is_ascii_digit())
    {
        return None;
    }
    let text = heading_text(after_blank);
    let name = js_trim(&text[digits + 1..]);
    let name = if name.is_empty() {
        text.clone()
    } else {
        name.to_owned()
    };
    let number = after_blank[..digits].trim_start_matches('0');
    let number = if number.is_empty() { "0" } else { number };
    Some((number.to_owned(), name))
}

/// A heading's text: trimmed, without an ATX closing `###` sequence (which must be
/// preceded by a blank).
fn heading_text(text: &str) -> String {
    let trimmed = js_trim(text);
    let head = trimmed.trim_end_matches('#');
    if head.len() < trimmed.len() && head.ends_with([' ', '\t']) {
        js_trim(head).to_owned()
    } else {
        trimmed.to_owned()
    }
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
pub(super) fn parse_uat_items_with_stats(text: &str) -> UatParse {
    let text = normalize_line_endings(text);
    let lines: Vec<&str> = text.split('\n').collect();
    let mut parse = UatParse::default();
    let mut index = 0;
    while index < lines.len() {
        let Some((number, name)) = parse_row_heading(lines[index]) else {
            index += 1;
            continue;
        };
        let mut end = index + 1;
        while end < lines.len() && !is_heading_line(lines[end]) {
            end += 1;
        }
        let mut block = lines[index..end].join("\n");
        if end < lines.len() {
            block.push('\n');
        }
        index = end;
        let Some(result) = block.split('\n').find_map(result_token) else {
            parse.headings_seen += 1;
            continue;
        };
        if result == "pass" || result == "passed" {
            continue;
        }
        let expected = non_empty(extract_scalar_field(&block, "expected"));
        let reason = non_empty(extract_scalar_field(&block, "reason"));
        let blocked_by = non_empty(extract_scalar_field(&block, "blocked_by"));
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
}
