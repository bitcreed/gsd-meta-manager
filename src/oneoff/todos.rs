//! The pending todos a one-off can pick (C-23).
//!
//! A todo is a regular `.md` file directly under `.planning/todos/pending/` whose name
//! is one plain path component ([`valid_todo_file_name`], T-f5y-10). Its title is the
//! frontmatter `title:`, else the first `# ` heading, else the file stem, cut to one
//! line of at most [`MAX_TITLE`] characters with control characters dropped. The title
//! is still untrusted repository text: every renderer draws it through
//! `text::render_for_terminal`.

use std::io::{self, Read};
use std::path::Path;
use std::sync::LazyLock;

use regex::Regex;
use serde::{Deserialize, Serialize};

use crate::text::carries_invisible_formatting;

/// Where pending todos live, relative to the project root.
pub const PENDING_REL: &str = ".planning/todos/pending";
/// Where a done todo is moved, relative to the project root.
pub const COMPLETED_REL: &str = ".planning/todos/completed";
/// The longest title kept, in characters.
pub const MAX_TITLE: usize = 200;
/// The most todos listed.
pub const MAX_TODOS: usize = 200;
/// How much of each todo file is read for its title.
const READ_PREFIX: u64 = 8 * 1024;

static TODO_NAME: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z0-9][A-Za-z0-9._-]{0,150}\.md$").expect("static regex"));

/// One pending todo.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PendingTodo {
    pub file_name: String,
    pub title: String,
}

/// Whether `name` is an acceptable todo file name: one component, `.md`, not hidden.
pub fn valid_todo_file_name(name: &str) -> bool {
    TODO_NAME.is_match(name)
}

/// `text` cut to its first line, with control and invisible formatting characters
/// dropped, trimmed, and at most [`MAX_TITLE`] characters.
pub fn sanitize_title(text: &str) -> String {
    let line = text.lines().next().unwrap_or("");
    let cleaned: String = line
        .chars()
        .filter(|c| !c.is_control() && !carries_invisible_formatting(c.encode_utf8(&mut [0; 4])))
        .collect();
    cleaned
        .trim()
        .chars()
        .take(MAX_TITLE)
        .collect::<String>()
        .trim()
        .to_owned()
}

/// The frontmatter `title:` of `text`, if it has one.
fn frontmatter_title(text: &str) -> Option<String> {
    let rest = text.strip_prefix("---")?;
    let rest = rest
        .strip_prefix("\r\n")
        .or_else(|| rest.strip_prefix('\n'))?;
    for line in rest.lines() {
        if line.trim_end() == "---" {
            break;
        }
        if let Some(value) = line.strip_prefix("title:") {
            let value = value.trim();
            let unquoted = value
                .strip_prefix('"')
                .and_then(|v| v.strip_suffix('"'))
                .or_else(|| value.strip_prefix('\'').and_then(|v| v.strip_suffix('\'')))
                .unwrap_or(value);
            return Some(unquoted.to_owned());
        }
    }
    None
}

/// The title of a todo whose text is `text` (pure).
pub fn title_from(text: &str, file_name: &str) -> String {
    let candidates = [
        frontmatter_title(text),
        text.lines()
            .find_map(|line| line.strip_prefix("# ").map(str::to_owned)),
    ];
    for candidate in candidates.into_iter().flatten() {
        let title = sanitize_title(&candidate);
        if !title.is_empty() {
            return title;
        }
    }
    let stem = file_name.strip_suffix(".md").unwrap_or(file_name);
    sanitize_title(stem)
}

/// The pending todos of `project`, sorted by file name. An absent directory is an
/// empty list. Only regular files (never a symlink) with a valid name are listed, at
/// most [`MAX_TODOS`], each read for its first 8 KiB only.
pub fn list_pending(project: &Path) -> io::Result<Vec<PendingTodo>> {
    let dir = project.join(PENDING_REL);
    let entries = match std::fs::read_dir(&dir) {
        Ok(entries) => entries,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(e),
    };
    let mut names: Vec<String> = entries
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| valid_todo_file_name(name))
        .collect();
    names.sort();
    let mut todos = Vec::new();
    for name in names {
        if todos.len() >= MAX_TODOS {
            break;
        }
        let path = dir.join(&name);
        let Ok(meta) = std::fs::symlink_metadata(&path) else {
            continue;
        };
        if !meta.file_type().is_file() {
            continue;
        }
        let mut prefix = Vec::new();
        if let Ok(file) = std::fs::File::open(&path) {
            let _ = file.take(READ_PREFIX).read_to_end(&mut prefix);
        }
        let text = String::from_utf8_lossy(&prefix);
        todos.push(PendingTodo {
            title: title_from(&text, &name),
            file_name: name,
        });
    }
    Ok(todos)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn titles_come_from_frontmatter_then_heading_then_stem() {
        assert_eq!(title_from("---\ntitle: \"X\"\n---\n# H\n", "f.md"), "X");
        assert_eq!(title_from("---\ntitle: Y\n---\n", "f.md"), "Y");
        assert_eq!(title_from("intro\n# Heading\nbody", "f.md"), "Heading");
        assert_eq!(title_from("just text", "my-todo.md"), "my-todo");
    }

    #[test]
    fn titles_are_one_clean_line_of_at_most_200_chars() {
        let long = format!("---\ntitle: {}\n---\n", "t".repeat(300));
        assert_eq!(title_from(&long, "f.md").chars().count(), MAX_TITLE);
        assert_eq!(title_from("# a\u{1b}[31mb\u{202E}c", "f.md"), "a[31mbc");
    }

    #[test]
    fn file_names_are_one_plain_md_component() {
        assert!(valid_todo_file_name("2026-11-03-fix.md"));
        for bad in ["../x.md", ".x.md", "a/b.md", "x.txt", "x.md/"] {
            assert!(!valid_todo_file_name(bad), "{bad}");
        }
    }

    #[test]
    fn an_absent_dir_is_an_empty_list() {
        let root = TempDir::new().unwrap();
        assert!(list_pending(root.path()).unwrap().is_empty());
    }

    #[test]
    fn the_list_skips_odd_entries_and_is_sorted() {
        let root = TempDir::new().unwrap();
        let dir = root.path().join(PENDING_REL);
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(dir.join("b.md"), "# Bee\n").unwrap();
        std::fs::write(dir.join("a.md"), "---\ntitle: Ay\n---\n").unwrap();
        std::fs::write(dir.join("notes.txt"), "# no").unwrap();
        std::fs::write(dir.join(".hidden.md"), "# no").unwrap();
        std::fs::create_dir(dir.join("sub.md")).unwrap();
        #[cfg(unix)]
        std::os::unix::fs::symlink(dir.join("a.md"), dir.join("link.md")).unwrap();
        let todos = list_pending(root.path()).unwrap();
        assert_eq!(
            todos,
            vec![
                PendingTodo {
                    file_name: "a.md".to_owned(),
                    title: "Ay".to_owned()
                },
                PendingTodo {
                    file_name: "b.md".to_owned(),
                    title: "Bee".to_owned()
                },
            ]
        );
    }
}
