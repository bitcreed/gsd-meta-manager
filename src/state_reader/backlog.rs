use std::path::{Path, PathBuf};

use crate::text::Untrusted;

/// One `999.*` directory under a project's `.planning/phases/`.
///
/// **Every text field is [`Untrusted`]** (D-21-19). Each of them is read off
/// disk from a repository the user cloned: `dir_name` is a directory name,
/// `number` and `description` are parsed out of it and out of the first
/// heading of a `.md` file inside it, and `content` is that file's body. None
/// of it was authored by this build, which is SAFE-07's own trust boundary.
///
/// The carrier is what makes that checkable rather than remembered: it
/// implements no `Display`, no `AsRef<str>`, no `Into<Cow<str>>`, so a render
/// site cannot interpolate one of these fields or hand it to a ratatui sink at
/// all. Retyping the struct is therefore what makes the COMPILER name every
/// consumer, instead of a reader working through a list of sites — which is how
/// `detail.rs:2882`'s raw `format!` survived nine rounds of review.
///
/// `path` stays a `PathBuf`: a path is not display text, and what may be done
/// with one is a different question with a different answer.
#[derive(Debug, Clone)]
pub struct BacklogItem {
    pub dir_name: Untrusted,
    pub number: Untrusted,
    pub description: Untrusted,
    pub content: Option<Untrusted>,
    /// Path to the first .md file in the backlog item directory.
    pub path: Option<PathBuf>,
}

/// Parse a backlog directory name like "999.3-queue-editor-and-reorder" into (number, slug).
/// Skips entries containing `{` or newlines (malformed slugs from broken JSON output).
pub fn parse_backlog_dir_name(name: &str) -> Option<(String, String)> {
    if name.contains('{') || name.contains('\n') {
        return None;
    }
    // Expected format: "999.N-slug-text"
    let rest = name.strip_prefix("999.")?;
    let dash_pos = rest.find('-')?;
    let number = format!("999.{}", &rest[..dash_pos]);
    let slug = rest[dash_pos + 1..].to_string();
    Some((number, slug))
}

/// Parse all backlog items from the `.planning/phases/` directory.
/// Reads 999.* directories, extracts number/slug, finds description from first heading.
/// Does NOT load content (leaves it as None).
pub fn parse_backlog_items(planning_dir: &Path) -> Vec<BacklogItem> {
    let phases_dir = planning_dir.join("phases");
    let entries = match std::fs::read_dir(&phases_dir) {
        Ok(e) => e,
        Err(_) => return Vec::new(),
    };

    let mut items: Vec<BacklogItem> = entries
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.file_name()
                .to_str()
                .map(|n| n.starts_with("999"))
                .unwrap_or(false)
                && e.file_type().map(|t| t.is_dir()).unwrap_or(false)
        })
        .filter_map(|e| {
            let dir_name = e.file_name().to_string_lossy().to_string();
            let (number, slug) = parse_backlog_dir_name(&dir_name)?;

            // Skip directories with no .md files (empty backlog placeholders)
            let md_path = find_first_md_file(&e.path())?;

            // Try to find description from first .md file's first heading
            let description = find_first_heading(&e.path()).unwrap_or_else(|| humanize_slug(&slug));

            // The ONE place these four values are created, so the ONE place
            // they are wrapped. Everything downstream inherits the carrier.
            Some(BacklogItem {
                dir_name: Untrusted::from_untrusted_source(dir_name),
                number: Untrusted::from_untrusted_source(number),
                description: Untrusted::from_untrusted_source(description),
                content: None,
                path: Some(md_path),
            })
        })
        .collect();

    // Sort by number ascending (999.1, 999.2, etc.)
    items.sort_by(|a, b| {
        // A sort key is a COMPARISON, not something a human reads, so the raw
        // bytes are the right answer here.
        let a_num: f64 = a
            .number
            .as_raw_for_logic_only()
            .strip_prefix("999.")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);
        let b_num: f64 = b
            .number
            .as_raw_for_logic_only()
            .strip_prefix("999.")
            .and_then(|s| s.parse().ok())
            .unwrap_or(0.0);
        a_num
            .partial_cmp(&b_num)
            .unwrap_or(std::cmp::Ordering::Equal)
    });

    items
}

/// Load the full content of a backlog item's first .md file.
pub fn load_backlog_content(planning_dir: &Path, dir_name: &str) -> Option<String> {
    let item_dir = planning_dir.join("phases").join(dir_name);
    let md_path = find_first_md_file(&item_dir)?;
    std::fs::read_to_string(md_path).ok()
}

/// Find the first .md file in a directory and extract its first `# heading`.
fn find_first_heading(dir: &Path) -> Option<String> {
    let md_path = find_first_md_file(dir)?;
    let content = std::fs::read_to_string(md_path).ok()?;
    for line in content.lines() {
        let trimmed = line.trim();
        if let Some(heading) = trimmed.strip_prefix("# ") {
            return Some(heading.trim().to_string());
        }
    }
    None
}

/// Find the first .md file in a directory (alphabetically).
fn find_first_md_file(dir: &Path) -> Option<std::path::PathBuf> {
    let mut entries: Vec<_> = std::fs::read_dir(dir)
        .ok()?
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().map(|ext| ext == "md").unwrap_or(false))
        .collect();
    entries.sort_by_key(|e| e.file_name());
    entries.first().map(|e| e.path())
}

/// Convert a slug like "queue-editor-and-reorder" to "Queue editor and reorder".
fn humanize_slug(slug: &str) -> String {
    let mut s = slug.replace('-', " ");
    if let Some(first) = s.get_mut(0..1) {
        first.make_ascii_uppercase();
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_backlog_dir_name_valid() {
        let result = parse_backlog_dir_name("999.3-queue-editor-and-reorder");
        assert_eq!(
            result,
            Some(("999.3".to_string(), "queue-editor-and-reorder".to_string()))
        );
    }

    #[test]
    fn test_parse_backlog_dir_name_with_brace() {
        assert_eq!(
            parse_backlog_dir_name("999.1-{\n  \"slug\": \"test\"\n}"),
            None
        );
    }

    #[test]
    fn test_parse_backlog_dir_name_no_prefix() {
        assert_eq!(parse_backlog_dir_name("05-state-reader"), None);
    }

    #[test]
    fn test_humanize_slug() {
        assert_eq!(
            humanize_slug("queue-editor-and-reorder"),
            "Queue editor and reorder"
        );
    }
}
