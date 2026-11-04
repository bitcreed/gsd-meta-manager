//! The backlog items a one-off can pick (quick 261103-ubn).
//!
//! A dispatchable backlog item is one `999.N` directory of a project's
//! `.planning/phases/` ([`crate::state_reader::backlog`]) whose entry (its ROADMAP.md
//! section, else its directory's `.md` files) is the task. All of it is untrusted
//! repository text (D-21-19, SAFE-07): the number is checked by
//! [`valid_backlog_number`], the title is one sanitised line and the entry is stripped
//! of control and invisible formatting characters and capped at [`MAX_ENTRY_BYTES`] by
//! [`clean_entry`]. The prompt builder and the launcher re-apply both to whatever spec
//! they are given, fresh or read back from a stored record.

use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;

use crate::state_reader::backlog::{backlog_dispatch_entry, parse_backlog_items};
use crate::text::carries_invisible_formatting;

use super::todos::sanitize_title;
use super::TaskSpec;

/// The most bytes of an entry kept.
///
/// The budget, against `MAX_PROMPT_BYTES` (120 KiB): this entry, the title (800 bytes
/// at most, 200 four-byte characters), the fixed instructions (about 2 KiB) and the
/// largest `build_resume` continuation (20 follow-ups and 50 commit lines of 200
/// four-byte characters, a 400-character note) stay under it. INFERRED: 48 KiB.
pub const MAX_ENTRY_BYTES: usize = 48 * 1024;

/// The note a cut entry ends with. ASCII, so its length is its byte count.
const TRUNCATION_NOTE: &str =
    "\n[entry truncated by the dispatcher; the rest is in .planning/ROADMAP.md]";

static BACKLOG_NUMBER: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^999\.[0-9]{1,9}$").expect("static regex"));

/// Whether `number` is a backlog number: `999.` and one to nine digits, nothing else.
pub fn valid_backlog_number(number: &str) -> bool {
    BACKLOG_NUMBER.is_match(number)
}

/// `entry` with `\r`, every other control character except `\n` and `\t`, and every
/// invisible formatting character dropped; trailing whitespace trimmed; and, above
/// [`MAX_ENTRY_BYTES`], cut on a character boundary with a visible note, the note
/// included in the bound. A second application changes nothing.
pub fn clean_entry(entry: &str) -> String {
    let kept: String = entry
        .chars()
        .filter(|&c| {
            (c == '\n' || c == '\t' || !c.is_control())
                && !carries_invisible_formatting(c.encode_utf8(&mut [0; 4]))
        })
        .collect();
    let kept = kept.trim_end();
    if kept.len() <= MAX_ENTRY_BYTES {
        return kept.to_owned();
    }
    let mut end = MAX_ENTRY_BYTES - TRUNCATION_NOTE.len();
    while !kept.is_char_boundary(end) {
        end -= 1;
    }
    // Trimming the cut keeps the result a fixed point: the note starts with a newline.
    let mut cut = kept[..end].trim_end().to_owned();
    cut.push_str(TRUNCATION_NOTE);
    cut
}

/// The task for backlog item `number` of `project`, read from the checkout.
///
/// Refuses (with a message that never echoes a malformed number) a number that is not
/// `999.<digits>`, a number that is no item of the project, and an item with no entry
/// to dispatch. The number is only compared against the items' numbers, never joined
/// into a path (T-ubn-03).
pub fn backlog_spec(project: &Path, number: &str) -> Result<TaskSpec, String> {
    if !valid_backlog_number(number) {
        return Err("not a backlog number: expected 999.<digits>".to_owned());
    }
    let planning = project.join(".planning");
    let item = parse_backlog_items(&planning)
        .into_iter()
        .find(|item| item.number.as_raw_for_logic_only() == number)
        .ok_or_else(|| format!("backlog {number} is not an item of this project"))?;
    let entry = backlog_dispatch_entry(&planning, item.dir_name.as_raw_for_logic_only())
        .map(|raw| clean_entry(&raw))
        .filter(|entry| !entry.trim().is_empty())
        .ok_or_else(|| format!("backlog {number} has no ROADMAP.md entry to dispatch"))?;
    // INFERRED: the title is the Backlog tab's own label for the item, so what is
    // selected is what is dispatched.
    let title = sanitize_title(item.description.as_raw_for_logic_only());
    let title = if title.is_empty() {
        number.to_owned()
    } else {
        title
    };
    Ok(TaskSpec::Backlog {
        number: number.to_owned(),
        title,
        entry,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn project() -> (tempfile::TempDir, std::path::PathBuf) {
        let td = tempfile::TempDir::new().unwrap();
        let root = td.path().to_path_buf();
        for dir in ["999.1-demo-item", "999.2-other-item"] {
            let path = root.join(".planning/phases").join(dir);
            std::fs::create_dir_all(&path).unwrap();
            std::fs::write(path.join(".gitkeep"), "").unwrap();
        }
        std::fs::write(
            root.join(".planning/ROADMAP.md"),
            "## Backlog\n\n### Phase 999.1: Demo item (BACKLOG)\n\n**Goal:** Do it.\n\n- [ ] TBD\n\n\
             ### Phase 999.2: Other item (BACKLOG)\n\nOTHER BODY\n",
        )
        .unwrap();
        (td, root)
    }

    #[test]
    fn numbers_are_999_dot_digits_and_nothing_else() {
        for ok in ["999.1", "999.12", "999.123456789"] {
            assert!(valid_backlog_number(ok), "{ok}");
        }
        for bad in [
            "",
            "999",
            "999.",
            "999.x",
            "999.1.2",
            "../999.1",
            "999.1 ",
            " 999.1",
            "-999.1",
            "999.1\n",
            "999.1234567890",
            "998.1",
        ] {
            assert!(!valid_backlog_number(bad), "{bad:?}");
        }
    }

    #[test]
    fn clean_entry_drops_control_and_invisible_characters_but_keeps_newlines_and_tabs() {
        let cleaned = clean_entry("a\u{7}b\u{1b}[2Jc\r\nd\te\u{202E}f  \n\n");
        assert_eq!(cleaned, "ab[2Jc\nd\tef");
    }

    #[test]
    fn a_long_entry_is_cut_on_a_char_boundary_with_a_visible_note_within_the_bound() {
        let long = "ü€😀".repeat(MAX_ENTRY_BYTES);
        let cleaned = clean_entry(&long);
        assert!(cleaned.len() <= MAX_ENTRY_BYTES, "{}", cleaned.len());
        assert!(cleaned.len() > MAX_ENTRY_BYTES - 8 - TRUNCATION_NOTE.len());
        assert!(
            cleaned.ends_with("is in .planning/ROADMAP.md]"),
            "{cleaned:?}"
        );
        assert!(cleaned.contains("truncated"));
        assert_eq!(clean_entry(&cleaned), cleaned, "idempotent");
        let exact = "x".repeat(MAX_ENTRY_BYTES);
        assert_eq!(clean_entry(&exact), exact, "at the bound is untouched");
        assert_eq!(clean_entry(&format!("{exact}y")).len(), MAX_ENTRY_BYTES);
    }

    #[test]
    fn clean_entry_is_idempotent_on_plain_text() {
        let text = "### Phase 999.1: x\n\n\tbody\n";
        let once = clean_entry(text);
        assert_eq!(clean_entry(&once), once);
    }

    #[test]
    fn backlog_spec_carries_the_title_and_the_cleaned_section() {
        let (_td, root) = project();
        match backlog_spec(&root, "999.1").unwrap() {
            TaskSpec::Backlog {
                number,
                title,
                entry,
            } => {
                assert_eq!(number, "999.1");
                assert_eq!(title, "Demo item");
                assert!(entry.starts_with("### Phase 999.1: Demo item (BACKLOG)"));
                assert!(entry.contains("- [ ] TBD") && !entry.contains("OTHER BODY"));
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn the_title_falls_back_to_the_number_when_blank() {
        let (_td, root) = project();
        // A directory with an empty slug has a blank label.
        std::fs::create_dir_all(root.join(".planning/phases/999.5-")).unwrap();
        std::fs::write(
            root.join(".planning/ROADMAP.md"),
            "## Backlog\n\n### Phase 999.5: Whatever (BACKLOG)\n\nBODY\n",
        )
        .unwrap();
        match backlog_spec(&root, "999.5").unwrap() {
            TaskSpec::Backlog { title, .. } => assert_eq!(title, "999.5"),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn the_refusals_say_why_and_never_echo_a_malformed_number() {
        let (_td, root) = project();
        let unknown = backlog_spec(&root, "999.9").unwrap_err();
        assert!(unknown.contains("not an item"), "{unknown}");
        let malformed = backlog_spec(&root, "../etc\u{1b}[2J").unwrap_err();
        assert!(!malformed.contains("etc") && !malformed.contains('\u{1b}'));
        std::fs::write(root.join(".planning/ROADMAP.md"), "## Backlog\n").unwrap();
        let empty = backlog_spec(&root, "999.1").unwrap_err();
        assert!(
            empty.contains("has no ROADMAP.md entry to dispatch"),
            "{empty}"
        );
    }

    #[test]
    fn a_dir_md_stands_in_for_a_missing_roadmap_section() {
        let (_td, root) = project();
        std::fs::remove_file(root.join(".planning/ROADMAP.md")).unwrap();
        std::fs::write(
            root.join(".planning/phases/999.1-demo-item/999.1-NOTE.md"),
            "# Real title\n\nFrom the directory.\n",
        )
        .unwrap();
        match backlog_spec(&root, "999.1").unwrap() {
            TaskSpec::Backlog { title, entry, .. } => {
                assert_eq!(title, "Real title");
                assert!(entry.contains("From the directory."));
            }
            other => panic!("{other:?}"),
        }
    }
}
