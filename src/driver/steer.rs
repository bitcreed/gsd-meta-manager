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
//! Pure on purpose: no file, no clock, no platform facility, so the module compiles
//! and is tested everywhere (the `write_body` it will later feed is Linux-only).

use crate::driver::direction::{blank_comments, item_re, section_heading, Section};

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
    name.trim()
        .eq_ignore_ascii_case(key)
        .then(|| value.trim())
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
        let body = "## Direction\n\n### I-05 misplaced\n- hold: true\n\n## Items\n\n### I-01 real\n";
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
        assert_eq!(items[0].answer, None, "a non-item `###` heading ends the item");
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
}
