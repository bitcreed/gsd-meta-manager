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
//! This plan writes [`extract`], [`fm_str`], [`source`] and [`updated`]; plan 34-06
//! adds the FAILSAFE lists, the anchor and alias refusal and the acknowledgment.

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

/// The frontmatter mapping of `text`: empty when the block is absent, unterminated,
/// not YAML (after one repair attempt) or not a mapping.
pub(super) fn extract(text: &str) -> Mapping {
    let Some(fence) = locate_fence(text) else {
        return Mapping::new();
    };
    if !fence.closed {
        return Mapping::new();
    }
    // The body loses its last newline; a final `key:` without one is a YAML error.
    let body = format!("{}\n", &text[fence.body]);
    let body = body.as_str();
    let parsed = serde_yml::from_str::<Value>(body).ok().or_else(|| {
        crate::state_reader::state_md::repair_frontmatter_block(body)
            .and_then(|(repaired, _)| serde_yml::from_str::<Value>(&repaired).ok())
    });
    match parsed {
        Some(Value::Mapping(map)) => map,
        _ => Mapping::new(),
    }
}

/// A FAILSAFE string for a scalar value: strings as they are, numbers and booleans
/// as their YAML text. Anything else (null, a list, a mapping) is `None`.
fn scalar_text(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Number(number) => Some(number.to_string()),
        Value::Bool(flag) => Some(flag.to_string()),
        _ => None,
    }
}

/// The scalar text of `key`, or `None` when absent or not a scalar.
pub(super) fn fm_str(map: &Mapping, key: &str) -> Option<String> {
    map.get(key).and_then(scalar_text)
}

/// The basenames of the `source` entry: a scalar, a flow list or a block list.
/// Each entry is reduced to the basename of its first whitespace-separated token.
pub(super) fn source(map: &Mapping) -> Vec<String> {
    let entries: Vec<String> = match map.get("source") {
        Some(Value::Sequence(items)) => items.iter().filter_map(scalar_text).collect(),
        Some(other) => scalar_text(other).into_iter().collect(),
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
}
