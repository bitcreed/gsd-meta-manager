//! `deferred-items.md`: the entries an executor logged as out of scope, ported from
//! gsd-core 1.21.0 (`parseDeferredItemsWithStatus`, `splitGapsEntriesCore` with the
//! deferred marker set, `splitDeferredHeadingEntriesDetailed`,
//! `extractGapEntryFields`, `rawGapEntryText`, `parseDeferredTableItems`).
//!
//! The file has no template. An entry is a list item (`-`, `*`, `+` or `1.`), a
//! heading with list items under it, or a table row; an entry whose `status:` field
//! reads `resolved` is closed and everything else is an open item. Code fences are
//! content, never entries or fields, and a thematic break ends the list.

use std::collections::{BTreeSet, HashSet};

use super::markdown::{self, scan_fenced_blocks};
use super::uat::{js_space, js_trim, line_terminator};
use super::{AuditItem, Category, Origin};

/// Port of `parseDeferredItems`: every entry that is not resolved, as an item with
/// result `unresolved`.
pub(super) fn parse_deferred_items(lines: &[&str]) -> Vec<AuditItem> {
    let section = markdown::collect_section(
        lines,
        |heading| heading.level == 2 && is_deferred_items_heading(&heading.text),
        true,
    );
    let body = section.map_or_else(|| lines.join("\n"), |found| found.body);
    let mut names: Vec<String> = Vec::new();
    for entry in entries_of(&body) {
        let text = raw_entry_text(&entry);
        if text.is_empty() {
            continue;
        }
        let resolved =
            entry_status(&entry).is_some_and(|status| status.to_lowercase() == "resolved");
        if !resolved {
            names.push(text);
        }
    }
    names.extend(table_items(&body));
    names
        .into_iter()
        .map(|name| AuditItem {
            origin: Origin::Deferred,
            test: None,
            name,
            expected: None,
            result: "unresolved".to_owned(),
            category: Category::Deferred,
            reason: None,
            blocked_by: None,
        })
        .collect()
}

/// `^deferred\s+items$` ignoring ASCII case, over an already trimmed heading text.
fn is_deferred_items_heading(text: &str) -> bool {
    let Some(head) = text.get(.."deferred".len()) else {
        return false;
    };
    if !head.eq_ignore_ascii_case("deferred") {
        return false;
    }
    let rest = &text["deferred".len()..];
    let after = rest.trim_start_matches(js_space);
    after.len() < rest.len() && after.eq_ignore_ascii_case("items")
}

/// One entry: its lines (carriage return removed) and, per line, whether the
/// splitter accepted it as a list opener.
#[derive(Debug, Clone, Default)]
struct Entry {
    lines: Vec<String>,
    opener: Vec<bool>,
    /// Set for entries of the heading path: the opener flags then decide which lines
    /// lose their marker before they are read as fields, and a flagged entry whose
    /// line 0 is no opener is a leaf heading (its line 0 is heading text). Unset for
    /// the list path, where only line 0 loses its marker.
    flagged: bool,
}

/// `entries` of `parseDeferredItemsWithStatus`: heading-delimited when the body has
/// headings, else list-delimited.
fn entries_of(body: &str) -> Vec<Entry> {
    let lines: Vec<&str> = body.split('\n').collect();
    split_heading_entries(&lines).unwrap_or_else(|| split_list_entries(&lines))
}

// ---------------------------------------------------------------------------------
// Markers
// ---------------------------------------------------------------------------------

/// `^(\s*)(?:[-*+]|\d{1,9}\.)[ \t]`: the leading whitespace and the marker token.
fn match_open(line: &str) -> Option<(&str, &str)> {
    let rest = line.trim_start_matches(js_space);
    let indent = &line[..line.len() - rest.len()];
    let token_len = match rest.as_bytes().first()? {
        b'-' | b'*' | b'+' => 1,
        b'0'..=b'9' => {
            let digits = rest.bytes().take_while(u8::is_ascii_digit).count();
            if digits > 9 || rest.as_bytes().get(digits) != Some(&b'.') {
                return None;
            }
            digits + 1
        }
        _ => return None,
    };
    matches!(rest.as_bytes().get(token_len), Some(b' ' | b'\t'))
        .then(|| (indent, &rest[..token_len]))
}

/// `line.replace(strip, '$2')`'s capture: `^(\s*)MARKER[ \t]+(.*?)\r?$`.
fn strip_marker(line: &str) -> Option<&str> {
    let (indent, token) = match_open(line)?;
    let rest = &line[indent.len() + token.len()..];
    let text = rest.trim_start_matches([' ', '\t']);
    let text = text.strip_suffix('\r').unwrap_or(text);
    (!text.contains(line_terminator)).then_some(text)
}

/// Leading width in CommonMark columns: a tab advances to the next multiple of four.
fn indent_of(indent: &str) -> usize {
    let mut column = 0;
    for ch in indent.chars() {
        match ch {
            ' ' => column += 1,
            '\t' => column += 4 - (column % 4),
            _ => break,
        }
    }
    column
}

/// `^[ \t]*([-*+_])(?:[ \t]*\1){2,}[ \t]*$`.
fn is_thematic_break(line: &str) -> bool {
    let rest = line.trim_start_matches([' ', '\t']);
    let Some(first) = rest
        .chars()
        .next()
        .filter(|c| matches!(c, '-' | '*' | '+' | '_'))
    else {
        return false;
    };
    let mut count = 0;
    for ch in rest.chars() {
        if ch == first {
            count += 1;
        } else if ch != ' ' && ch != '\t' {
            return false;
        }
    }
    count >= 3
}

/// `matchListOpener`: the opener's indent, refusing an ordered start above 1 where no
/// list is open at the line's level.
fn match_list_opener(line: &str, in_list: bool) -> Option<usize> {
    let (indent, token) = match_open(line)?;
    if token.as_bytes()[0].is_ascii_digit() && !in_list {
        let digits = &token[..token.len() - 1];
        if digits.parse::<u64>().map_or(true, |start| start > 1) {
            return None;
        }
    }
    Some(indent_of(indent))
}

/// `topLevelItemShape`.
fn top_level_item_shape(line: &str, base_indent: Option<usize>) -> bool {
    match_open(line)
        .is_some_and(|(indent, _)| base_indent.is_none_or(|base| indent_of(indent) <= base))
}

/// Per-indent list memory: whether a list is open at an indent.
#[derive(Default)]
struct ListRuns(BTreeSet<usize>);

impl ListRuns {
    fn at(&self, indent: usize) -> bool {
        self.0.contains(&indent)
    }
    fn opened(&mut self, indent: usize) {
        self.0.retain(|d| *d <= indent);
        self.0.insert(indent);
    }
    fn ended_at(&mut self, indent: usize) {
        self.0.retain(|d| *d < indent);
    }
    fn clear(&mut self) {
        self.0.clear();
    }
}

// ---------------------------------------------------------------------------------
// Fences
// ---------------------------------------------------------------------------------

/// `deindentedForFences`: the same lines without leading spaces and tabs.
fn deindented<'a>(lines: &[&'a str]) -> Vec<&'a str> {
    lines
        .iter()
        .map(|line| line.trim_start_matches([' ', '\t']))
        .collect()
}

/// `scanFencesFrom`'s result.
struct FenceScan {
    fenced: HashSet<usize>,
    openers: HashSet<usize>,
    unterminated_from: Option<usize>,
}

fn scan_fences_from(lines: &[&str], from: usize) -> FenceScan {
    let mut scan = FenceScan {
        fenced: HashSet::new(),
        openers: HashSet::new(),
        unterminated_from: None,
    };
    let tail = lines.get(from..).unwrap_or(&[]);
    for block in scan_fenced_blocks(&deindented(tail)) {
        let open = block.open_line + from;
        scan.openers.insert(open);
        match block.close_line {
            None => {
                scan.unterminated_from = Some(open);
                break;
            }
            Some(close) => scan.fenced.extend(open..=close + from),
        }
    }
    scan
}

/// `fencedLineSet`: the fenced lines of one entry, delimiters included.
fn fenced_line_set(lines: &[&str]) -> HashSet<usize> {
    let mut fenced = HashSet::new();
    for block in scan_fenced_blocks(&deindented(lines)) {
        let last = block.close_line.unwrap_or(lines.len().saturating_sub(1));
        fenced.extend(block.open_line..=last);
    }
    fenced
}

// ---------------------------------------------------------------------------------
// Splitting
// ---------------------------------------------------------------------------------

/// `splitGapsEntriesCore` with the deferred markers: entries on top-level list
/// openers.
fn split_list_entries(lines: &[&str]) -> Vec<Entry> {
    let mut entries = Vec::new();
    let mut current: Option<Entry> = None;
    let mut base_indent: Option<usize> = None;
    let mut runs = ListRuns::default();
    let mut scan = scan_fences_from(lines, 0);
    let mut blank_seen = false;
    for (idx, raw) in lines.iter().enumerate() {
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        let level_of = |base: Option<usize>| {
            let width = indent_of(&line[..line.len() - line.trim_start_matches([' ', '\t']).len()]);
            match base {
                Some(base) if width <= base => base,
                _ => width,
            }
        };
        if scan.unterminated_from.is_some_and(|from| idx > from)
            && top_level_item_shape(line, base_indent)
        {
            scan = scan_fences_from(lines, idx);
        }
        if scan.fenced.contains(&idx) || scan.unterminated_from.is_some_and(|from| idx >= from) {
            if scan.openers.contains(&idx) {
                runs.ended_at(level_of(base_indent));
            }
            if let Some(entry) = current.as_mut() {
                entry.lines.push(line.to_owned());
                entry.opener.push(false);
            }
            continue;
        }
        if is_thematic_break(line) {
            entries.extend(current.take());
            runs.clear();
            blank_seen = false;
            continue;
        }
        let opener = match_list_opener(line, runs.at(level_of(base_indent)));
        if let Some(indent) = opener {
            let base = *base_indent.get_or_insert(indent);
            runs.opened(level_of(base_indent));
            if indent <= base {
                entries.extend(current.take());
                current = Some(Entry {
                    lines: vec![line.to_owned()],
                    opener: vec![true],
                    flagged: false,
                });
                blank_seen = false;
                continue;
            }
        }
        if let Some(entry) = current.as_mut() {
            entry.lines.push(line.to_owned());
            entry.opener.push(opener.is_some());
            let blank = js_trim(line).is_empty();
            if !blank && blank_seen && opener.is_none() {
                runs.ended_at(level_of(base_indent));
            }
            blank_seen = opener.is_none() && blank;
        }
    }
    entries.extend(current);
    entries
}

/// Add a line to the open entry, or to the pending list-shaped region.
fn push_line(current: &mut Option<Entry>, pending: &mut Vec<String>, line: &str, opener: bool) {
    match current.as_mut() {
        Some(entry) => {
            entry.lines.push(line.to_owned());
            entry.opener.push(opener);
        }
        None => pending.push(line.to_owned()),
    }
}

/// `splitDeferredHeadingEntriesDetailed`: an entry per leaf heading that has a list
/// item under it, plus the list-shaped entries of the text outside any leaf. `None`
/// when the body has no headings.
fn split_heading_entries(lines: &[&str]) -> Option<Vec<Entry>> {
    let headings = markdown::tokenize_headings(lines);
    if headings.is_empty() {
        return None;
    }
    let mut heading_at = std::collections::HashMap::new();
    for (index, heading) in headings.iter().enumerate() {
        let container = headings
            .get(index + 1)
            .is_some_and(|next| next.level > heading.level);
        heading_at.insert(heading.line, (heading.text.clone(), container));
    }
    let mut entries = Vec::new();
    let mut current: Option<Entry> = None;
    let mut has_bullet = false;
    let mut pending: Vec<String> = Vec::new();
    let mut runs = ListRuns::default();
    let mut blank_seen = false;
    let mut body_base: Option<usize> = None;
    let mut scan = scan_fences_from(lines, 0);

    let flush_current =
        |entries: &mut Vec<Entry>, current: &mut Option<Entry>, has_bullet: &mut bool| {
            if let Some(entry) = current.take() {
                if *has_bullet {
                    entries.push(entry);
                }
            }
            *has_bullet = false;
        };
    let flush_pending = |entries: &mut Vec<Entry>, pending: &mut Vec<String>| {
        if !pending.is_empty() {
            let region = pending.join("\n");
            let region_lines: Vec<&str> = region.split('\n').collect();
            entries.extend(
                split_list_entries(&region_lines)
                    .into_iter()
                    .map(|entry| Entry {
                        flagged: true,
                        ..entry
                    }),
            );
        }
        pending.clear();
    };

    for (i, raw) in lines.iter().enumerate() {
        if let Some((text, container)) = heading_at.get(&i) {
            flush_current(&mut entries, &mut current, &mut has_bullet);
            flush_pending(&mut entries, &mut pending);
            runs.clear();
            blank_seen = false;
            body_base = None;
            scan = scan_fences_from(lines, i + 1);
            if !container {
                current = Some(Entry {
                    lines: vec![text.clone()],
                    opener: vec![false],
                    flagged: true,
                });
                has_bullet = false;
            }
            continue;
        }
        let line = raw.strip_suffix('\r').unwrap_or(raw);
        let level_of = |base: Option<usize>| {
            let width = indent_of(&line[..line.len() - line.trim_start_matches([' ', '\t']).len()]);
            match base {
                Some(base) if width <= base => base,
                _ => width,
            }
        };
        if scan.unterminated_from.is_some_and(|from| i > from)
            && top_level_item_shape(line, body_base)
        {
            scan = scan_fences_from(lines, i);
        }
        if scan.fenced.contains(&i) || scan.unterminated_from.is_some_and(|from| i >= from) {
            if scan.openers.contains(&i) {
                runs.ended_at(level_of(body_base));
            }
            push_line(&mut current, &mut pending, line, false);
            continue;
        }
        if is_thematic_break(line) {
            runs.clear();
            blank_seen = false;
            push_line(&mut current, &mut pending, line, false);
            continue;
        }
        if line.trim_start_matches(js_space).starts_with('|') {
            continue;
        }
        if current.is_some() {
            let opener = match_list_opener(line, runs.at(level_of(body_base)));
            push_line(&mut current, &mut pending, line, opener.is_some());
            if let Some(indent) = opener {
                has_bullet = true;
                body_base.get_or_insert(indent);
                runs.opened(level_of(body_base));
            } else if blank_seen && !js_trim(line).is_empty() {
                runs.ended_at(level_of(body_base));
            }
            blank_seen = js_trim(line).is_empty();
        } else {
            push_line(&mut current, &mut pending, line, false);
        }
    }
    flush_current(&mut entries, &mut current, &mut has_bullet);
    flush_pending(&mut entries, &mut pending);
    Some(entries)
}

// ---------------------------------------------------------------------------------
// Fields and names
// ---------------------------------------------------------------------------------

/// Whether line `idx` of an entry carries a marker to strip before it is read.
fn strips_marker_at(entry: &Entry, idx: usize) -> bool {
    if entry.flagged {
        entry.opener.get(idx).copied().unwrap_or(false)
    } else {
        idx == 0
    }
}

/// `parseGapEntryFieldLine`: the `key: value` of one line, a bolded key
/// (`**Status:**`) lower-cased, a double-quoted value unquoted.
fn field_of(raw: &str, strip: bool) -> Option<(String, String)> {
    let line = raw.strip_suffix('\r').unwrap_or(raw);
    let bare = if strip {
        strip_marker(line).unwrap_or_else(|| js_trim(line))
    } else {
        js_trim(line)
    };
    let stars = bare.bytes().take_while(|b| *b == b'*').count();
    let mut content = bare.to_owned();
    if stars > 0 {
        let rest = &bare[stars..];
        let key_len = key_len(rest);
        let after = &rest[key_len..];
        if key_len > 0 && after.starts_with(':') {
            let tail = after[1..].trim_start_matches('*');
            if after[1..].starts_with('*') {
                content = format!("{}:{}", rest[..key_len].to_lowercase(), tail);
            }
        }
    }
    let length = key_len(&content);
    if length == 0 || !content[length..].starts_with(':') {
        return None;
    }
    let value = js_trim(content[length + 1..].trim_start_matches(js_space));
    if value.contains(line_terminator) {
        return None;
    }
    let value = if value.len() >= 2 && value.starts_with('"') && value.ends_with('"') {
        &value[1..value.len() - 1]
    } else {
        value
    };
    Some((content[..length].to_owned(), value.to_owned()))
}

/// The length of a leading `[A-Za-z_][A-Za-z0-9_-]*`, 0 when there is none.
fn key_len(text: &str) -> usize {
    let bytes = text.as_bytes();
    if !bytes
        .first()
        .is_some_and(|b| b.is_ascii_alphabetic() || *b == b'_')
    {
        return 0;
    }
    bytes
        .iter()
        .take_while(|b| b.is_ascii_alphanumeric() || **b == b'_' || **b == b'-')
        .count()
}

/// `extractGapEntryFields(...).status`: the first `status` field outside any fence.
fn entry_status(entry: &Entry) -> Option<String> {
    let refs: Vec<&str> = entry.lines.iter().map(String::as_str).collect();
    let leaf = entry.flagged && entry.opener.first() == Some(&false);
    let fenced = if leaf {
        // A heading's text is not a Markdown line: it must not open a fence.
        let mut view = refs.clone();
        if let Some(first) = view.first_mut() {
            *first = "";
        }
        fenced_line_set(&view)
    } else {
        fenced_line_set(&refs)
    };
    for (idx, line) in entry.lines.iter().enumerate() {
        if fenced.contains(&idx) {
            continue;
        }
        if let Some((key, value)) = field_of(line, strips_marker_at(entry, idx)) {
            if key == "status" {
                return Some(value);
            }
        }
    }
    None
}

/// `rawGapEntryText`: the entry's lines joined by a space, the marker of an accepted
/// first line removed.
fn raw_entry_text(entry: &Entry) -> String {
    let parts: Vec<&str> = entry
        .lines
        .iter()
        .enumerate()
        .map(|(idx, line)| {
            if idx == 0 && strips_marker_at(entry, 0) {
                strip_marker(line).unwrap_or(line)
            } else {
                js_trim(line)
            }
        })
        .collect();
    js_trim(&parts.join(" ")).to_owned()
}

// ---------------------------------------------------------------------------------
// Tables
// ---------------------------------------------------------------------------------

/// `collectTableRows` + `parseDeferredTableItems`: the rows of every pipe table
/// (header and delimiter excluded when there is a delimiter row) whose cells do not
/// read `resolved`, `done` or `pass`, joined with ` — `.
fn table_items(body: &str) -> Vec<String> {
    let mut tables: Vec<Vec<Vec<String>>> = Vec::new();
    let mut rows: Option<Vec<Vec<String>>> = None;
    let mut pending: Option<Vec<String>> = None;
    for raw in body.split('\n') {
        let line = js_trim(raw.strip_suffix('\r').unwrap_or(raw));
        if !line.starts_with('|') {
            if let Some(cells) = pending.take() {
                rows.get_or_insert_with(Vec::new).push(cells);
            }
            tables.extend(rows.take());
            continue;
        }
        let cells = markdown::split_table_row(line);
        if markdown::is_delimiter_row(&cells) {
            rows.get_or_insert_with(Vec::new);
            pending = None;
            continue;
        }
        if let Some(previous) = pending.take() {
            rows.get_or_insert_with(Vec::new).push(previous);
        }
        pending = Some(cells);
    }
    if let Some(cells) = pending.take() {
        rows.get_or_insert_with(Vec::new).push(cells);
    }
    tables.extend(rows.take());
    let mut names = Vec::new();
    for row in tables.into_iter().flatten() {
        if row.iter().any(|cell| {
            ["resolved", "done", "pass"]
                .iter()
                .any(|word| cell.eq_ignore_ascii_case(word))
        }) {
            continue;
        }
        let name = row
            .iter()
            .filter(|cell| !cell.is_empty())
            .map(String::as_str)
            .collect::<Vec<_>>()
            .join(" \u{2014} ");
        if !name.is_empty() {
            names.push(name);
        }
    }
    names
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> Vec<String> {
        let lines: Vec<&str> = text.split('\n').collect();
        parse_deferred_items(&lines)
            .into_iter()
            .map(|item| item.name)
            .collect()
    }

    #[test]
    fn every_marker_opens_an_entry() {
        let text = "## Deferred Items\n\n- dash one\n* star two\n+ plus three\n1. ordered four\n2. ordered five\n";
        assert_eq!(
            parse(text),
            [
                "dash one",
                "star two",
                "plus three",
                "ordered four",
                "ordered five"
            ]
        );
    }

    #[test]
    fn the_section_is_used_when_there_is_one_else_the_whole_file() {
        assert_eq!(
            parse("- outside\n\n## Deferred Items\n\n- inside\n\n## Other\n\n- elsewhere\n"),
            ["inside"]
        );
        assert_eq!(parse("- one\n- two\n"), ["one", "two"]);
        assert!(parse("").is_empty());
        assert!(parse("# Notes\n\nprose only\n").is_empty());
    }

    #[test]
    fn a_thematic_break_ends_the_list_and_prose_ordinals_are_not_items() {
        assert_eq!(parse("- one\n---\n- two\n"), ["one", "two"]);
        assert_eq!(parse("- - -\n- real\n"), ["real"]);
        assert!(parse("2026. was a long year\n").is_empty());
    }

    #[test]
    fn a_resolved_status_closes_an_entry() {
        let text = "- fixed thing\n  status: resolved\n- open thing\n  status: pending\n- quoted\n  status: \"Resolved\"\n";
        assert_eq!(parse(text), ["open thing status: pending"]);
    }

    #[test]
    fn heading_entries_read_fields_from_sibling_bullets() {
        let text = "## Deferred Items\n\n### Closed one\n\n- **Status:** resolved\n- **Note:** done\n\n### Open one\n\n- **Status:** open\n- **Note:** later\n";
        let names = parse(text);
        assert_eq!(names.len(), 1);
        assert!(names[0].starts_with("Open one"), "{names:?}");
    }

    #[test]
    fn a_status_inside_a_fence_does_not_resolve_an_entry() {
        let text = "- keeps open\n  ```\n  status: resolved\n  ```\n";
        assert_eq!(parse(text).len(), 1);
    }

    #[test]
    fn table_rows_are_items_unless_a_cell_says_resolved_done_or_pass() {
        let text = "| Test | Seeds | State |\n|------|-------|-------|\n| a | 1, 2 | open |\n| b | 3 | Done |\n| c | 4 | PASS |\n| d |  |  |\n";
        assert_eq!(parse(text), ["a \u{2014} 1, 2 \u{2014} open", "d"]);
    }

    #[test]
    fn the_items_are_unresolved_deferred() {
        let lines = ["- one"];
        let items = parse_deferred_items(&lines);
        assert_eq!(items.len(), 1);
        assert_eq!(items[0].result, "unresolved");
        assert_eq!(items[0].category, Category::Deferred);
        assert_eq!(items[0].origin, Origin::Deferred);
    }

    #[test]
    fn two_hundred_entries_are_two_hundred_items() {
        let mut text = String::from("## Deferred Items\n\n");
        for n in 1..=200 {
            text.push_str(&format!("- item {n}: something\n"));
        }
        assert_eq!(parse(&text).len(), 200);
    }
}
