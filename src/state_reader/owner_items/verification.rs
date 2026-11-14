//! `*-VERIFICATION.md` files: the status gate and the open checks a verifier left for
//! a human, ported from gsd-core 1.21.0 (`reportStatusOf`, `parseVerificationItems`,
//! `parseHumanVerificationItems`, `parseVerificationGapsItems`,
//! `normalizeHumanVerificationEntry`).
//!
//! # The gate
//!
//! Only a report whose frontmatter `status` is exactly `human_needed` or `gaps_found`
//! is read for items. `passed` and an absent, blank or unparseable status contribute
//! nothing. Any other value, including `Human_Needed` (no case folding) or a number,
//! is outside the closed set: upstream aborts the whole run on it, this port marks
//! that one file unreadable ([`Gate::OutOfSet`]) and keeps reading the rest.

use serde_yml::{Mapping, Value};

use super::uat::{categorize_item, js_space, js_trim, line_terminator};
use super::{frontmatter, markdown, AuditItem, Category, EntryFields, Origin};

/// What the `status` key of a report says about reading it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(super) enum Gate {
    /// `passed`, or no usable status: the file contributes nothing.
    Omit,
    /// `human_needed` or `gaps_found`: read its items.
    Open(String),
    /// A value outside the closed set, spelled as found.
    OutOfSet(String),
}

/// Port of `reportStatusOf`: the exact-match judgement of a report's `status`.
pub(super) fn report_status(fm: &Mapping) -> Gate {
    match fm.get("status") {
        None | Some(Value::Null) => Gate::Omit,
        Some(Value::String(raw)) => {
            let trimmed = js_trim(raw);
            match trimmed {
                "" | "passed" => Gate::Omit,
                "human_needed" | "gaps_found" => Gate::Open(trimmed.to_owned()),
                other => Gate::OutOfSet(other.to_owned()),
            }
        }
        Some(other) => {
            Gate::OutOfSet(frontmatter::failsafe_str(other).unwrap_or_else(|| "?".to_owned()))
        }
    }
}

/// Port of `parseVerificationItems`: the open checks of a report with `status`.
pub(super) fn parse_verification_items(
    lines: &[&str],
    fm: &Mapping,
    status: &str,
) -> Vec<AuditItem> {
    let mut items = Vec::new();
    match status {
        "gaps_found" => {
            items.extend(human_verification_items(lines, fm));
            items.extend(gap_items(fm));
        }
        "human_needed" => items.extend(human_verification_items(lines, fm)),
        _ => {}
    }
    items
}

fn human_needed(test: Option<String>, name: String) -> AuditItem {
    AuditItem {
        origin: Origin::HumanVerification,
        test,
        name,
        expected: None,
        result: "human_needed".to_owned(),
        category: Category::HumanUat,
        reason: None,
        blocked_by: None,
        fields: None,
    }
}

/// `entry[key]` as upstream's `entryField`: a non-blank string verbatim, a number or
/// boolean as its text, anything else absent.
fn entry_field(entry: &Mapping, key: &str) -> Option<String> {
    match entry.get(key)? {
        Value::String(text) => (!js_trim(text).is_empty()).then(|| text.clone()),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(flag) => Some(flag.to_string()),
        _ => None,
    }
}

/// The structured fields of a mapping entry (D-14), read as upstream's `entryField`.
fn entry_fields(entry: &Mapping) -> EntryFields {
    EntryFields {
        test: entry_field(entry, "test"),
        truth: entry_field(entry, "truth"),
        name: entry_field(entry, "name"),
        description: entry_field(entry, "description"),
        expected: entry_field(entry, "expected"),
        why_human: entry_field(entry, "why_human"),
    }
}

/// `frontmatterEntryStatus`: the trimmed lower-cased `status` string, `None` when
/// absent, blank or not a string.
fn entry_status(entry: &Mapping) -> Option<String> {
    match entry.get("status")? {
        Value::String(text) => {
            let trimmed = js_trim(text);
            (!trimmed.is_empty()).then(|| trimmed.to_lowercase())
        }
        _ => None,
    }
}

/// `isHumanVerificationEntryResolved`: `status` decides where readable, otherwise a
/// non-blank `resolution` closes the entry.
fn human_entry_resolved(entry: &Mapping) -> bool {
    if let Some(status) = entry_status(entry) {
        return status == "resolved";
    }
    matches!(entry.get("resolution"), Some(Value::String(text)) if !js_trim(text).is_empty())
}

/// `normalizeHumanVerificationEntry` for a display string: wrapping quotes trimmed,
/// the trimmed original when nothing is left.
fn normalize_entry(raw: &str) -> String {
    let trimmed = js_trim(raw);
    let unquoted = js_trim(trimmed.trim_matches(['"', '\'']));
    if unquoted.is_empty() {
        trimmed.to_owned()
    } else {
        unquoted.to_owned()
    }
}

/// `parseVerificationGapsItems`: the open entries of the frontmatter `gaps:` list.
fn gap_items(fm: &Mapping) -> Vec<AuditItem> {
    let Some(Value::Sequence(entries)) = fm.get("gaps") else {
        return Vec::new();
    };
    let displays = frontmatter::fm_list(fm, "gaps");
    let mut items = Vec::new();
    for (entry, display) in entries.iter().zip(&displays) {
        let Value::Mapping(entry) = entry else {
            items.push(AuditItem {
                origin: Origin::VerificationGap,
                test: None,
                name: normalize_entry(display),
                expected: None,
                result: "unknown".to_owned(),
                category: categorize_item("unknown", None, None),
                reason: None,
                blocked_by: None,
                fields: None,
            });
            continue;
        };
        if entry_status(entry).as_deref() == Some("resolved") {
            continue;
        }
        let result = entry_field(entry, "status").unwrap_or_else(|| "unknown".to_owned());
        let reason = entry_field(entry, "reason");
        let name = entry_field(entry, "truth")
            .unwrap_or_else(|| frontmatter::flatten_object_list_item(entry));
        items.push(AuditItem {
            origin: Origin::VerificationGap,
            test: None,
            name,
            expected: None,
            category: categorize_item(&result, reason.as_deref(), None),
            result,
            reason,
            blocked_by: None,
            fields: Some(entry_fields(entry)),
        });
    }
    items
}

/// `parseHumanVerificationItems`: the frontmatter `human_verification` list when it
/// has entries, else the `## Human Verification` section of the body.
fn human_verification_items(lines: &[&str], fm: &Mapping) -> Vec<AuditItem> {
    if let Some(Value::Sequence(entries)) = fm.get("human_verification") {
        if !entries.is_empty() {
            let displays = frontmatter::fm_list(fm, "human_verification");
            return entries
                .iter()
                .zip(&displays)
                .enumerate()
                .filter(|(_, (entry, _))| {
                    !matches!(entry, Value::Mapping(map) if human_entry_resolved(map))
                })
                .map(|(index, (entry, display))| {
                    let mut item =
                        human_needed(Some((index + 1).to_string()), normalize_entry(display));
                    if let Value::Mapping(map) = entry {
                        item.fields = Some(entry_fields(map));
                    }
                    item
                })
                .collect();
        }
    }
    body_items(lines)
}

/// `^human\s+verification` ignoring ASCII case.
fn is_human_verification_heading(text: &str) -> bool {
    let Some(head) = text.get(..5) else {
        return false;
    };
    if !head.eq_ignore_ascii_case("human") {
        return false;
    }
    let rest = &text[5..];
    let after = rest.trim_start_matches(js_space);
    after.len() < rest.len()
        && after
            .get(.."verification".len())
            .is_some_and(|word| word.eq_ignore_ascii_case("verification"))
}

/// A decimal test number as `parseInt` would print it.
fn number_text(digits: &str) -> String {
    let trimmed = digits.trim_start_matches('0');
    if trimmed.is_empty() { "0" } else { trimmed }.to_owned()
}

/// `\s+(.+)` after a marker, as JavaScript backtracks it: the text up to the first
/// line terminator, or the last whitespace character when only whitespace follows.
fn ws_then_rest(after: &str) -> Option<&str> {
    let rest = after.trim_start_matches(js_space);
    let skipped = &after[..after.len() - rest.len()];
    if skipped.is_empty() {
        return None;
    }
    if !rest.is_empty() {
        let end = rest.find(line_terminator).unwrap_or(rest.len());
        return Some(&rest[..end]);
    }
    let mut chars = skipped.char_indices().rev();
    let (last, ch) = chars.next()?;
    (skipped.chars().count() >= 2 && !line_terminator(ch)).then(|| &skipped[last..])
}

/// The numbered, bulleted and tabular items of a `## Human Verification` section,
/// plus its `### N. Name` headings followed by a bold-led paragraph.
fn body_items(lines: &[&str]) -> Vec<AuditItem> {
    let Some(section) = markdown::collect_section(
        lines,
        |heading| heading.level == 2 && is_human_verification_heading(&heading.text),
        true,
    ) else {
        return Vec::new();
    };
    let mut items = Vec::new();
    let body_lines: Vec<&str> = section.body.split('\n').collect();
    for line in &body_lines {
        let trimmed = js_trim(line);
        let mut table = None;
        if trimmed.starts_with('|') {
            let cells = markdown::split_table_row(trimmed);
            if cells.len() >= 2
                && !cells[0].is_empty()
                && cells[0].bytes().all(|b| b.is_ascii_digit())
            {
                table = Some(cells);
            }
        }
        let bullet = line.strip_prefix(['-', '*']).and_then(ws_then_rest);
        let numbered = {
            let digits = line.bytes().take_while(u8::is_ascii_digit).count();
            line[digits..]
                .strip_prefix('.')
                .filter(|_| digits > 0)
                .and_then(ws_then_rest)
                .map(|rest| (&line[..digits], rest))
        };
        if let Some(cells) = table {
            let passed = cells.iter().skip(2).any(|cell| {
                cell.eq_ignore_ascii_case("pass") || cell.eq_ignore_ascii_case("resolved")
            });
            if !passed {
                items.push(human_needed(Some(number_text(&cells[0])), cells[1].clone()));
            }
        } else if let Some((digits, rest)) = numbered {
            items.push(human_needed(
                Some(number_text(digits)),
                js_trim(rest).to_owned(),
            ));
        } else if let Some(rest) = bullet {
            if rest.encode_utf16().count() > 10 {
                items.push(human_needed(None, js_trim(rest).to_owned()));
            }
        }
    }
    let headings: Vec<markdown::Heading> = markdown::tokenize_headings(&body_lines)
        .into_iter()
        .filter(|heading| heading.level == 3 && numbered_heading(&heading.text).is_some())
        .collect();
    for (position, heading) in headings.iter().enumerate() {
        let end = headings
            .get(position + 1)
            .map_or(body_lines.len(), |next| next.line);
        let after = body_lines[(heading.line + 1).min(end)..end].join("\n");
        if !after.trim_start_matches(js_space).starts_with("**") {
            continue;
        }
        if let Some((digits, name)) = numbered_heading(&heading.text) {
            items.push(human_needed(
                Some(number_text(digits)),
                js_trim(name).to_owned(),
            ));
        }
    }
    items
}

/// `^(\d+)\.\s+(.+)$` over a heading's text.
fn numbered_heading(text: &str) -> Option<(&str, &str)> {
    let digits = text.bytes().take_while(u8::is_ascii_digit).count();
    if digits == 0 {
        return None;
    }
    let rest = text[digits..].strip_prefix('.')?;
    let name = rest.trim_start_matches(js_space);
    if name.len() == rest.len() || name.is_empty() || name.contains(line_terminator) {
        return None;
    }
    Some((&text[..digits], name))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str, status: &str) -> Vec<AuditItem> {
        let fm = frontmatter::extract(text);
        let lines: Vec<&str> = text.split('\n').collect();
        parse_verification_items(&lines, &fm, status)
    }

    fn names(items: &[AuditItem]) -> Vec<&str> {
        items.iter().map(|item| item.name.as_str()).collect()
    }

    fn tests_of(items: &[AuditItem]) -> Vec<Option<&str>> {
        items.iter().map(|item| item.test.as_deref()).collect()
    }

    #[test]
    fn a_frontmatter_list_of_strings_numbers_the_checks() {
        let items = parse(
            "---\nstatus: human_needed\nhuman_verification:\n  - \"check one\"\n  - check two\n  - 'check three'\n---\n",
            "human_needed",
        );
        assert_eq!(names(&items), ["check one", "check two", "check three"]);
        assert_eq!(tests_of(&items), [Some("1"), Some("2"), Some("3")]);
        assert!(items.iter().all(|item| item.result == "human_needed"));
        assert!(items.iter().all(|item| item.category == Category::HumanUat));
    }

    #[test]
    fn an_object_entry_is_flattened_and_a_resolved_one_is_skipped_keeping_the_index() {
        let items = parse(
            "---\nhuman_verification:\n  - test: open it\n    expected: it opens\n  - test: closed one\n    status: resolved\n  - test: done by note\n    resolution: checked on a device\n  - test: still open\n    status: failed\n    resolution: attempted a retry\n---\n",
            "human_needed",
        );
        assert_eq!(names(&items).len(), 2);
        assert_eq!(items[0].name, "test: open it, expected: it opens");
        assert_eq!(
            items[1].name,
            "test: still open, status: failed, resolution: attempted a retry"
        );
        assert_eq!(tests_of(&items), [Some("1"), Some("4")]);
    }

    #[test]
    fn mapping_entries_carry_their_fields_and_scalars_do_not() {
        let items = parse(
            "---\nhuman_verification:\n  - test: Weekly summary sends\n    expected: Mail arrives\n    why_human: Needs a real inbox\n  - plain scalar\n  - test: closed\n    status: resolved\n  - truth: Footer shows the year\n---\n",
            "human_needed",
        );
        assert_eq!(items.len(), 3);
        let first = items[0].fields.as_ref().expect("a mapping has fields");
        assert_eq!(first.test.as_deref(), Some("Weekly summary sends"));
        assert_eq!(first.expected.as_deref(), Some("Mail arrives"));
        assert_eq!(first.why_human.as_deref(), Some("Needs a real inbox"));
        assert_eq!(items[1].fields, None);
        assert_eq!(items[1].name, "plain scalar");
        assert_eq!(items[2].test.as_deref(), Some("4"));
        assert_eq!(
            items[2].fields.as_ref().and_then(|f| f.truth.as_deref()),
            Some("Footer shows the year")
        );
        assert_eq!(
            items[0].name,
            "test: Weekly summary sends, expected: Mail arrives, why_human: Needs a real inbox"
        );
    }

    #[test]
    fn a_gap_mapping_without_truth_carries_its_description() {
        let items = parse(
            "---\nstatus: gaps_found\ngaps:\n  - description: Export skips the header\n    status: failed\n---\n",
            "gaps_found",
        );
        assert_eq!(items.len(), 1);
        let fields = items[0].fields.as_ref().expect("fields");
        assert_eq!(
            fields.description.as_deref(),
            Some("Export skips the header")
        );
        assert_eq!(fields.truth, None);
    }

    #[test]
    fn the_body_section_is_read_when_the_frontmatter_has_no_list() {
        let text = "---\nstatus: human_needed\n---\n\n## Human Verification\n\n| # | Check | Result |\n|---|-------|--------|\n| 1 | first table check | |\n| 2 | passed table check | pass |\n| 3 | resolved table check | RESOLVED |\n\n4. numbered check\n- a bullet that is long enough\n- short\n\n### 5. Heading check\n\n**Test:** do it\n\n### 6. Not a check\n\nplain text\n\n## Next Section\n\n7. not in the section\n";
        let items = parse(text, "human_needed");
        assert_eq!(
            names(&items),
            [
                "first table check",
                "numbered check",
                "a bullet that is long enough",
                "Heading check"
            ]
        );
        assert_eq!(tests_of(&items), [Some("1"), Some("4"), None, Some("5")]);
    }

    #[test]
    fn gaps_found_adds_the_open_frontmatter_gaps() {
        let text = "---\nstatus: gaps_found\nhuman_verification:\n  - one human check\ngaps:\n  - truth: a missing feature\n    status: failed\n    reason: not built yet\n  - truth: already fixed\n    status: resolved\n  - truth: no status here\n  - a bare string gap\n---\n";
        let items = parse(text, "gaps_found");
        assert_eq!(items.len(), 4);
        assert_eq!(items[0].origin, Origin::HumanVerification);
        assert_eq!(items[1].name, "a missing feature");
        assert_eq!(items[1].result, "failed");
        assert_eq!(items[1].reason.as_deref(), Some("not built yet"));
        assert_eq!(items[1].test, None);
        assert_eq!(items[2].result, "unknown");
        assert_eq!(items[3].name, "a bare string gap");
        assert_eq!(items[3].result, "unknown");
        assert!(items[1..]
            .iter()
            .all(|i| i.origin == Origin::VerificationGap));
    }

    #[test]
    fn human_needed_does_not_read_gaps() {
        let text = "---\nstatus: human_needed\nhuman_verification:\n  - one\ngaps:\n  - truth: ignored\n---\n";
        assert_eq!(parse(text, "human_needed").len(), 1);
    }

    #[test]
    fn the_status_gate_is_an_exact_match() {
        let gate = |text: &str| report_status(&frontmatter::extract(text));
        assert_eq!(gate("---\nstatus: passed\n---\n"), Gate::Omit);
        assert_eq!(gate("---\nstatus: \"\"\n---\n"), Gate::Omit);
        assert_eq!(gate("---\nother: 1\n---\n"), Gate::Omit);
        assert_eq!(gate("no frontmatter"), Gate::Omit);
        assert_eq!(
            gate("---\nstatus: \"human_needed\"\n---\n"),
            Gate::Open("human_needed".into())
        );
        assert_eq!(
            gate("---\nstatus: gaps_found\n---\n"),
            Gate::Open("gaps_found".into())
        );
        assert_eq!(
            gate("---\nstatus: Human_Needed\n---\n"),
            Gate::OutOfSet("Human_Needed".into())
        );
        assert_eq!(
            gate("---\nstatus: complete\n---\n"),
            Gate::OutOfSet("complete".into())
        );
        assert_eq!(gate("---\nstatus: 7\n---\n"), Gate::OutOfSet("7".into()));
    }

    #[test]
    fn a_bullet_of_exactly_ten_characters_is_not_a_check() {
        let text = "## Human Verification\n\n- 0123456789\n- 01234567890\n";
        let items = parse(text, "human_needed");
        assert_eq!(names(&items), ["01234567890"]);
    }
}
