//! YAML frontmatter of an audit file, read the way gsd-core 1.21.0 reads it.
//!
//! The fence is a port of upstream's `locateFrontmatterFence`: it opens only at
//! byte 0 (a leading BOM is tolerated) and `disk_status::leading_block`, which trims
//! the first line, is deliberately not reused because it would diverge from the
//! oracle. A block that never closes yields an empty mapping, so a truncated header
//! can neither assert a status nor hide a row. The closed block is parsed as a
//! `serde_yml::Value` (never a typed struct, so an odd value cannot fail the read)
//! with one ambiguous-colon repair retry, like `state_md`.
//!
//! A block that holds a YAML anchor or alias is refused before it is parsed
//! ([`refuses_anchors`]): `serde_yml` would expand an alias bomb for seconds, so the
//! block reads as an empty mapping, as upstream's `refuseAnchorsAndAliases` does.
//! Values are read FAILSAFE, as upstream's `FAILSAFE_SCHEMA` does: every scalar is
//! text ([`failsafe_str`]), [`fm_list`] flattens lists, and [`is_acknowledged`] ports
//! `isAuditItemAcknowledged`.

use chrono::{DateTime, Utc};
use serde_yml::{Mapping, Value};

/// Where a frontmatter block sits inside a text.
#[derive(Debug, PartialEq, Eq)]
struct Fence {
    /// Byte range of the body, between the opening and the closing fence.
    body: std::ops::Range<usize>,
    closed: bool,
}

/// Port of `locateFrontmatterFence` without the `allowPreamble` option.
fn locate_fence(text: &str) -> Option<Fence> {
    let start = if text.starts_with('\u{feff}') {
        '\u{feff}'.len_utf8()
    } else {
        0
    };
    let after = &text[start..];
    let open_len = if after.starts_with("---\r\n") {
        5
    } else if after.starts_with("---\n") {
        4
    } else {
        return None;
    };
    let open_end = start + open_len;
    let bytes = text.as_bytes();
    let strict = |line: &str| line.strip_prefix("---").is_some_and(only_blanks);
    let lenient = |line: &str| {
        let dashes = line.bytes().take_while(|b| *b == b'-').count();
        dashes >= 4 && only_blanks(&line[dashes..])
    };
    let closed_at = |line_start: usize| {
        let mut body_end = open_end;
        if line_start > open_end {
            body_end = line_start - 1;
            if body_end > open_end && bytes[body_end - 1] == b'\r' {
                body_end -= 1;
            }
        }
        Some(Fence {
            body: open_end..body_end,
            closed: true,
        })
    };
    let mut lenient_at: Option<usize> = None;
    let mut line_start = open_end;
    while line_start <= text.len() {
        let newline = text[line_start..].find('\n').map(|n| n + line_start);
        let line_end = match newline {
            Some(n) if n > line_start && bytes[n - 1] == b'\r' => n - 1,
            Some(n) => n,
            None => text.len(),
        };
        let line = &text[line_start..line_end];
        if strict(line) {
            return closed_at(line_start);
        }
        if lenient_at.is_none() && lenient(line) {
            lenient_at = Some(line_start);
        }
        match newline {
            Some(n) => line_start = n + 1,
            None => break,
        }
    }
    if let Some(line_start) = lenient_at {
        return closed_at(line_start);
    }
    Some(Fence {
        body: open_end..text.len(),
        closed: false,
    })
}

fn only_blanks(rest: &str) -> bool {
    rest.chars().all(|c| c == ' ' || c == '\t')
}

/// A closed frontmatter block whose YAML could not be read (after one repair attempt,
/// or because it carries an anchor or alias, which the reader refuses).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) struct Unparseable;

/// The frontmatter mapping of `text`: empty when the block is absent, unterminated,
/// not YAML (after one repair attempt) or not a mapping.
pub(super) fn extract(text: &str) -> Mapping {
    extract_checked(text).unwrap_or_default()
}

/// [`extract`], telling a block that is absent or not a mapping (`Ok`, empty) apart
/// from a closed block whose YAML did not parse (`Err`): a reader that must not read
/// a damaged file as clean needs the difference.
pub(super) fn extract_checked(text: &str) -> Result<Mapping, Unparseable> {
    let Some(fence) = locate_fence(text) else {
        return Ok(Mapping::new());
    };
    if !fence.closed {
        return Ok(Mapping::new());
    }
    // The body loses its last newline; a final `key:` without one is a YAML error.
    let body = format!("{}\n", &text[fence.body]);
    let body = body.as_str();
    if refuses_anchors(body) {
        return Err(Unparseable);
    }
    let parsed = serde_yml::from_str::<Value>(body).ok().or_else(|| {
        crate::state_reader::state_md::repair_frontmatter_block(body)
            .and_then(|(repaired, _)| serde_yml::from_str::<Value>(&repaired).ok())
    });
    match parsed {
        Some(Value::Mapping(map)) => Ok(map),
        Some(_) => Ok(Mapping::new()),
        None => Err(Unparseable),
    }
}

/// Whether a YAML anchor (`&name`) or alias (`*name`) sits where a node starts.
///
/// A conservative token scan, not a YAML parser: a node starts at the beginning of a
/// line, after `- `, `? `, `: `, a tag, and inside a flow collection after `[`, `{`
/// or `,`. Quoted scalars are skipped, a ` #` ends the line, and an indicator only
/// counts when an anchor-name character follows it, so `**bold**`, `R&D` and
/// `a * b` do not trip it. It may refuse text inside a block scalar; refusing too
/// much only empties the frontmatter, which every reader treats as absent.
fn refuses_anchors(body: &str) -> bool {
    let name_char = |c: char| c.is_ascii_alphanumeric() || c == '_' || c == '-';
    let mut depth = 0usize;
    for line in body.lines() {
        let chars: Vec<char> = line.chars().collect();
        let mut at_start = true;
        let mut i = 0;
        while i < chars.len() {
            let c = chars[i];
            let next = chars.get(i + 1).copied();
            let blank_next = next.is_none_or(|n| n == ' ' || n == '\t');
            if c == ' ' || c == '\t' {
                i += 1;
                continue;
            }
            if c == '#' && (i == 0 || matches!(chars[i - 1], ' ' | '\t')) {
                break;
            }
            if at_start {
                match c {
                    '&' | '*' if next.is_some_and(name_char) => return true,
                    '"' | '\'' => {
                        i = skip_quoted(&chars, i);
                        at_start = false;
                        continue;
                    }
                    '-' | '?' if blank_next => {}
                    '[' | '{' => depth += 1,
                    '!' => {
                        while i < chars.len() && chars[i] != ' ' && chars[i] != '\t' {
                            i += 1;
                        }
                        continue;
                    }
                    _ => at_start = false,
                }
            } else {
                match c {
                    ':' if blank_next => at_start = true,
                    ',' if depth > 0 => at_start = true,
                    '[' | '{' if depth > 0 => {
                        depth += 1;
                        at_start = true;
                    }
                    ']' | '}' => depth = depth.saturating_sub(1),
                    _ => {}
                }
            }
            i += 1;
        }
    }
    false
}

/// The index just past the quoted scalar that opens at `chars[start]`, or the end of
/// the line when it never closes.
fn skip_quoted(chars: &[char], start: usize) -> usize {
    let quote = chars[start];
    let mut i = start + 1;
    while i < chars.len() {
        match chars[i] {
            '\\' if quote == '"' => i += 1,
            c if c == quote => {
                if quote == '\'' && chars.get(i + 1) == Some(&'\'') {
                    i += 1;
                } else {
                    return i + 1;
                }
            }
            _ => {}
        }
        i += 1;
    }
    chars.len()
}

/// A FAILSAFE string for a scalar value: strings as they are, numbers and booleans
/// as their YAML text (a number loses its spelling, `1.10` reads as `1.1`, which
/// only matters for presence checks). Anything else (null, a list, a mapping) is
/// `None`.
pub(super) fn failsafe_str(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(flag) => Some(flag.to_string()),
        _ => None,
    }
}

/// The scalar text of `key`, or `None` when absent or not a scalar.
pub(super) fn fm_str(map: &Mapping, key: &str) -> Option<String> {
    map.get(key).and_then(failsafe_str)
}

/// The basenames of the `source` entry: a scalar, a flow list or a block list.
/// Each entry is reduced to the basename of its first whitespace-separated token.
pub(super) fn source(map: &Mapping) -> Vec<String> {
    let entries: Vec<String> = match map.get("source") {
        Some(Value::Sequence(items)) => items.iter().filter_map(failsafe_str).collect(),
        Some(other) => failsafe_str(other).into_iter().collect(),
        None => Vec::new(),
    };
    entries
        .iter()
        .filter_map(|entry| entry.split_whitespace().next())
        .map(|token| token.rsplit('/').next().unwrap_or(token).to_owned())
        .filter(|name| !name.is_empty())
        .collect()
}

/// The `updated` value as an RFC 3339 instant, or `None`.
pub(super) fn updated(map: &Mapping) -> Option<DateTime<Utc>> {
    let text = fm_str(map, "updated")?;
    DateTime::parse_from_rfc3339(text.trim())
        .ok()
        .map(|moment| moment.with_timezone(&Utc))
}

/// A display string for any value inside a list item: null is empty, a list is
/// `[a, b]`, a mapping is [`flatten_object_list_item`].
fn flatten_for_display(value: &Value) -> String {
    match value {
        Value::Null => String::new(),
        Value::Sequence(items) => format!(
            "[{}]",
            items
                .iter()
                .map(flatten_for_display)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Value::Mapping(map) => flatten_object_list_item(map),
        Value::Tagged(tagged) => flatten_for_display(tagged.value()),
        other => failsafe_str(other).unwrap_or_default(),
    }
}

/// Port of `flattenObjectListItem`: a mapping rendered as `key: value, key: value`
/// in insertion order, the one canonical string for every spelling of an object list
/// item.
pub(super) fn flatten_object_list_item(item: &Mapping) -> String {
    item.iter()
        .map(|(key, value)| format!("{}: {}", key, flatten_for_display(value)))
        .collect::<Vec<_>>()
        .join(", ")
}

/// The entries of the list at `key` as display strings: a scalar entry as its text,
/// null as an empty string, a mapping or a nested list flattened. A scalar value
/// reads as a one-entry list; an absent key, null or a mapping reads as empty.
#[allow(dead_code)] // first caller: the VERIFICATION and deferred parsers of plans 34-02 and 34-03
pub(super) fn fm_list(map: &Mapping, key: &str) -> Vec<String> {
    match map.get(key) {
        Some(Value::Sequence(items)) => items.iter().map(flatten_for_display).collect(),
        Some(other) => failsafe_str(other).into_iter().collect(),
        None => Vec::new(),
    }
}

/// Port of `isAuditItemAcknowledged` for a snapshot comparison: `audit_acknowledged`
/// is a mapping with a non-empty `milestone` and `at` (any scalar counts, as under
/// FAILSAFE) and a `snapshot_key` entry that is a YAML string equal to `current`.
/// Anything malformed acknowledges nothing, and the order of the mapping's keys is
/// irrelevant.
pub(super) fn is_acknowledged(fm: &Mapping, snapshot_key: &str, current: &str) -> bool {
    let Some(Value::Mapping(marker)) = fm.get("audit_acknowledged") else {
        return false;
    };
    let present = |key: &str| {
        marker
            .get(key)
            .and_then(failsafe_str)
            .is_some_and(|text| !text.is_empty())
    };
    present("milestone")
        && present("at")
        && matches!(marker.get(snapshot_key), Some(Value::String(text)) if text == current)
}

/// Port of `deriveUatGapSnapshotValue`: `<status>::scenarios=<n>`, where `n` counts
/// the case-insensitive `result:` + whitespace + `pending` or `[pending]` matches over
/// the whole text, fenced text included.
pub(super) fn uat_gap_snapshot(status: &str, text: &str) -> String {
    let lower = text.to_ascii_lowercase();
    let mut count = 0usize;
    let mut from = 0usize;
    while let Some(found) = lower[from..].find("result:") {
        let after = from + found + "result:".len();
        let rest = &lower[after..];
        let value = rest.trim_start_matches(super::uat::js_space);
        let skipped = rest.len() - value.len();
        let matched = if value.starts_with("pending") {
            Some("pending".len())
        } else if value.starts_with("[pending]") {
            Some("[pending]".len())
        } else {
            None
        };
        if let Some(len) = matched {
            count += 1;
            from = after + skipped + len;
        } else {
            from += found + 1;
        }
    }
    format!("{status}::scenarios={count}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_fence_opens_only_at_byte_zero() {
        assert!(extract("\n---\nstatus: x\n---\n").is_empty());
        assert!(extract(" ---\nstatus: x\n---\n").is_empty());
        assert_eq!(
            fm_str(&extract("---\nstatus: testing\n---\nbody"), "status").as_deref(),
            Some("testing")
        );
    }

    #[test]
    fn a_bom_and_crlf_are_tolerated() {
        let map = extract("\u{feff}---\r\nstatus: partial\r\n---\r\nbody");
        assert_eq!(fm_str(&map, "status").as_deref(), Some("partial"));
    }

    #[test]
    fn an_unterminated_block_is_empty() {
        assert!(extract("---\nstatus: complete\nno closing fence\n").is_empty());
    }

    #[test]
    fn a_lenient_closer_is_used_only_without_a_strict_one() {
        let map = extract("---\nstatus: a\n-----\nbody\n");
        assert_eq!(fm_str(&map, "status").as_deref(), Some("a"));
        let text = "---\na: 1\n-----\nb: 2\n---\n";
        let fence = locate_fence(text).unwrap();
        assert_eq!(
            &text[fence.body], "a: 1\n-----\nb: 2",
            "the strict closer wins"
        );
    }

    #[test]
    fn an_empty_block_is_an_empty_mapping() {
        assert!(extract("---\n---\nbody").is_empty());
    }

    #[test]
    fn scalars_are_read_failsafe() {
        let map = extract("---\ncount: 5\nflag: true\nq: \"gaps_found\"\nlist: [a]\nnone:\n---\n");
        assert_eq!(fm_str(&map, "count").as_deref(), Some("5"));
        assert_eq!(fm_str(&map, "flag").as_deref(), Some("true"));
        assert_eq!(fm_str(&map, "q").as_deref(), Some("gaps_found"));
        assert_eq!(fm_str(&map, "list"), None);
        assert_eq!(fm_str(&map, "none"), None);
    }

    #[test]
    fn an_ambiguous_colon_is_repaired_once() {
        let map = extract("---\nscore: 5/5: all verified\nstatus: complete\n---\n");
        assert_eq!(fm_str(&map, "status").as_deref(), Some("complete"));
    }

    #[test]
    fn source_accepts_a_scalar_a_flow_list_and_a_block_list() {
        let scalar = extract("---\nsource: phases/01-a/01-01-SUMMARY.md extra\n---\n");
        assert_eq!(source(&scalar), ["01-01-SUMMARY.md"]);
        let flow = extract("---\nsource: [01-01-SUMMARY.md, x/02-SUMMARY.md]\n---\n");
        assert_eq!(source(&flow), ["01-01-SUMMARY.md", "02-SUMMARY.md"]);
        let block = extract("---\nsource:\n  - a/01-SUMMARY.md\n  - 02-SUMMARY.md notes\n---\n");
        assert_eq!(source(&block), ["01-SUMMARY.md", "02-SUMMARY.md"]);
        assert!(source(&extract("---\nstatus: x\n---\n")).is_empty());
    }

    #[test]
    fn updated_parses_rfc_3339_only() {
        let map = extract("---\nupdated: 2026-01-10T12:00:00Z\n---\n");
        assert_eq!(
            updated(&map).unwrap().to_rfc3339(),
            "2026-01-10T12:00:00+00:00"
        );
        assert_eq!(updated(&extract("---\nupdated: yesterday\n---\n")), None);
        assert_eq!(updated(&extract("---\nstatus: x\n---\n")), None);
    }
    #[test]
    fn a_numeric_milestone_counts_as_present() {
        let map = extract("---\nmilestone: 1.10\n---\n");
        assert!(fm_str(&map, "milestone").is_some_and(|text| !text.is_empty()));
    }

    #[test]
    fn an_anchor_or_alias_refuses_the_whole_block() {
        assert!(extract("---\nx: &a 1\nstatus: complete\n---\n").is_empty());
        assert!(extract("---\nx: &a [1]\ny: *a\n---\n").is_empty());
        assert!(extract("---\nlist:\n  - &a one\n  - *a\n---\n").is_empty());
        assert!(extract("---\nflow: [*a, b]\n---\n").is_empty());
        assert!(extract("---\nk: !!str &a v\n---\n").is_empty());
    }

    #[test]
    fn prose_with_indicator_characters_is_not_an_anchor() {
        let body =
            "title: use **bold** here\nco: R&D\nexpr: a * b\nq: \"&a *b\"\nr: '*c'\nnote: x # &a\n";
        assert!(!refuses_anchors(body));
        let map = extract(&format!("---\n{body}---\n"));
        assert_eq!(fm_str(&map, "co").as_deref(), Some("R&D"));
        assert_eq!(fm_str(&map, "expr").as_deref(), Some("a * b"));
        assert_eq!(fm_str(&map, "q").as_deref(), Some("&a *b"));
        assert!(
            !refuses_anchors("a: \"x\\\" &y\"\n"),
            "an escaped quote stays inside"
        );
    }

    #[test]
    fn lists_are_flattened_for_display() {
        let map = extract(
            "---\nplain: [a, 1, true]\nobjs:\n  - test: a b\n    n: 2\n  - {test: c}\nholes:\n  -\n  - x\nnested: [[a, b], c]\none: single\n---\n",
        );
        assert_eq!(fm_list(&map, "plain"), ["a", "1", "true"]);
        assert_eq!(fm_list(&map, "objs"), ["test: a b, n: 2", "test: c"]);
        assert_eq!(fm_list(&map, "holes"), ["", "x"]);
        assert_eq!(fm_list(&map, "nested"), ["[a, b]", "c"]);
        assert_eq!(fm_list(&map, "one"), ["single"]);
        assert!(fm_list(&map, "absent").is_empty());
    }

    #[test]
    fn an_object_list_item_renders_in_insertion_order() {
        let map = extract("---\nitem:\n  zeta: 1\n  alpha: two\n  none:\n---\n");
        let Some(Value::Mapping(item)) = map.get("item") else {
            panic!("a mapping");
        };
        assert_eq!(
            flatten_object_list_item(item),
            "zeta: 1, alpha: two, none: "
        );
    }

    fn fm_with(marker: &str) -> Mapping {
        extract(&format!("---\nstatus: x\n{marker}---\n"))
    }

    const SNAP: &str = "diagnosed::scenarios=2";

    fn marker(body: &str) -> Mapping {
        fm_with(&format!("audit_acknowledged:\n{body}"))
    }

    #[test]
    fn the_acknowledgment_table() {
        let full = "  milestone: v1.0\n  at: \"2026-01-12T09:00:00Z\"\n  gap_snapshot: \"diagnosed::scenarios=2\"\n";
        assert!(
            is_acknowledged(&marker(full), "gap_snapshot", SNAP),
            "match"
        );
        for current in [
            "diagnosed::scenarios=1",
            "diagnosed::scenarios=3",
            "Diagnosed::scenarios=2",
            "diagnosed::scenarios=0",
        ] {
            assert!(
                !is_acknowledged(&marker(full), "gap_snapshot", current),
                "{current}"
            );
        }
        let reordered = "  gap_snapshot: \"diagnosed::scenarios=2\"\n  at: t\n  milestone: v1.0\n";
        assert!(
            is_acknowledged(&marker(reordered), "gap_snapshot", SNAP),
            "key order"
        );
        let no_milestone =
            "  milestone: \"\"\n  at: t\n  gap_snapshot: \"diagnosed::scenarios=2\"\n";
        assert!(!is_acknowledged(
            &marker(no_milestone),
            "gap_snapshot",
            SNAP
        ));
        let no_at = "  milestone: v1.0\n  gap_snapshot: \"diagnosed::scenarios=2\"\n";
        assert!(!is_acknowledged(&marker(no_at), "gap_snapshot", SNAP));
        let empty_at = "  milestone: v1.0\n  at:\n  gap_snapshot: \"diagnosed::scenarios=2\"\n";
        assert!(!is_acknowledged(&marker(empty_at), "gap_snapshot", SNAP));
        let precise =
            "  milestone: 1.10\n  at: 2026-11-13\n  gap_snapshot: \"diagnosed::scenarios=2\"\n";
        assert!(
            is_acknowledged(&marker(precise), "gap_snapshot", SNAP),
            "float and date count as present"
        );
        let number_snapshot = "  milestone: v1.0\n  at: t\n  gap_snapshot: 2\n";
        assert!(
            !is_acknowledged(&marker(number_snapshot), "gap_snapshot", "2"),
            "only a string snapshot counts"
        );
        let no_snapshot = "  milestone: v1.0\n  at: t\n";
        assert!(!is_acknowledged(&marker(no_snapshot), "gap_snapshot", SNAP));
    }

    #[test]
    fn a_malformed_or_absent_marker_acknowledges_nothing() {
        assert!(
            !is_acknowledged(&fm_with(""), "gap_snapshot", SNAP),
            "absent"
        );
        assert!(
            !is_acknowledged(&fm_with("audit_acknowledged:\n"), "gap_snapshot", SNAP),
            "null"
        );
        assert!(
            !is_acknowledged(
                &fm_with("audit_acknowledged: [a, b]\n"),
                "gap_snapshot",
                SNAP
            ),
            "list"
        );
        assert!(
            !is_acknowledged(&fm_with("audit_acknowledged: {}\n"), "gap_snapshot", SNAP),
            "empty map"
        );
        assert!(
            !is_acknowledged(&fm_with("audit_acknowledged: yes\n"), "gap_snapshot", SNAP),
            "scalar"
        );
    }

    #[test]
    fn the_gap_snapshot_counts_pending_results_over_the_whole_text() {
        assert_eq!(uat_gap_snapshot("diagnosed", ""), "diagnosed::scenarios=0");
        let text = "result: pending\nRESULT:   [Pending]\nresult:\n  pending\nresult: passed\nresult: pendingx\n```\nresult: pending\n```\nresult:[pending\n";
        assert_eq!(uat_gap_snapshot("testing", text), "testing::scenarios=5");
        assert_eq!(
            uat_gap_snapshot("unknown", "xresult: pending result: pending"),
            "unknown::scenarios=2"
        );
    }
}
