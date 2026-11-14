//! The pending-todo count and the age of the oldest one (Phase 35, D-08, OWN-10).
//!
//! Todos have no upstream audit reader, so this is native and fixture-tested. The
//! enumeration is [`crate::oneoff::todos::pending_paths`], the same one the todo
//! listing uses, so the count and the list cannot drift (regular files with a valid
//! name, at most 200).
//!
//! # Numbered facts
//!
//! 1. **Age.** Whole days, floored, between the todo's frontmatter `created:` (an
//!    RFC 3339 instant, quoted or not, or a bare date read as midnight UTC) and the
//!    caller's `now`; a todo without a readable `created:` ages by its file time.
//!    A `created:` in the future is age 0. The oldest is the maximum.
//! 2. **Fail-safe.** When the pending folder cannot be listed (it is a regular file,
//!    unreadable, ...) the count is 0 and `unreadable` is true, so a failure is never
//!    reported as a clean list (34-REVIEW IN-05).
//! 3. **Bounded reads.** Only the first 8 KiB of each file is read.

use std::io::Read;
use std::path::Path;

use chrono::{DateTime, Utc};

use super::frontmatter;
use crate::oneoff::todos::{pending_paths, READ_PREFIX};

/// What the pending folder holds, as the inbox's `hidden` object reports it.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub(super) struct TodoStats {
    pub count: usize,
    pub oldest_days: Option<i64>,
    pub unreadable: bool,
}

/// The moment a todo was created: its `created:` value, else its file time.
fn created_at(path: &Path) -> Option<DateTime<Utc>> {
    let mut prefix = Vec::new();
    if let Ok(file) = std::fs::File::open(path) {
        let _ = file.take(READ_PREFIX).read_to_end(&mut prefix);
    }
    let text = String::from_utf8_lossy(&prefix);
    let map = frontmatter::extract(&text);
    frontmatter::fm_str(&map, "created")
        .and_then(|value| frontmatter::moment(&value))
        .or_else(|| {
            std::fs::symlink_metadata(path)
                .and_then(|meta| meta.modified())
                .ok()
                .map(DateTime::<Utc>::from)
        })
}

/// The pending todos of the project at `project_root` as of `now`.
pub(super) fn todo_stats(project_root: &Path, now: DateTime<Utc>) -> TodoStats {
    let Ok(paths) = pending_paths(project_root) else {
        return TodoStats {
            count: 0,
            oldest_days: None,
            unreadable: true,
        };
    };
    let oldest_days = paths
        .iter()
        .filter_map(|path| created_at(path))
        .map(|moment| (now - moment).num_days().max(0))
        .max();
    TodoStats {
        count: paths.len(),
        oldest_days,
        unreadable: false,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::oneoff::todos::PENDING_REL;

    fn now() -> DateTime<Utc> {
        "2026-11-14T00:00:00Z".parse().unwrap()
    }

    fn project(todos: &[(&str, &str)]) -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let pending = dir.path().join(PENDING_REL);
        std::fs::create_dir_all(&pending).unwrap();
        for (name, text) in todos {
            std::fs::write(pending.join(name), text).unwrap();
        }
        dir
    }

    fn oldest(text: &str) -> Option<i64> {
        todo_stats(project(&[("a.md", text)]).path(), now()).oldest_days
    }

    #[test]
    fn created_is_read_exactly_in_each_spelling() {
        // 2026-07-27T07:28:24.815Z to 2026-11-14T00:00:00Z is 109 whole days.
        assert_eq!(
            oldest("---\ncreated: 2026-07-27T07:28:24.815Z\n---\n"),
            Some(109)
        );
        assert_eq!(
            oldest("---\ncreated: \"2026-08-01T00:00:00Z\"\n---\n"),
            Some(105)
        );
        assert_eq!(oldest("---\ncreated: 2026-08-15\n---\n"), Some(91));
    }

    #[test]
    fn a_missing_created_ages_by_the_file_time_and_a_future_one_is_zero() {
        let dir = project(&[("a.md", "# no frontmatter\n")]);
        let path = dir.path().join(PENDING_REL).join("a.md");
        let file = std::fs::File::options().write(true).open(&path).unwrap();
        file.set_modified(
            "2026-06-01T00:00:00Z"
                .parse::<DateTime<Utc>>()
                .unwrap()
                .into(),
        )
        .unwrap();
        assert_eq!(todo_stats(dir.path(), now()).oldest_days, Some(166));
        assert_eq!(oldest("---\ncreated: 2027-01-01\n---\n"), Some(0));
    }

    #[test]
    fn the_oldest_is_the_maximum_and_no_todos_is_none() {
        let dir = project(&[
            ("a.md", "---\ncreated: 2026-11-01\n---\n"),
            ("b.md", "---\ncreated: 2026-10-01\n---\n"),
        ]);
        let stats = todo_stats(dir.path(), now());
        assert_eq!((stats.count, stats.oldest_days), (2, Some(44)));
        let empty = todo_stats(project(&[]).path(), now());
        assert_eq!(empty, TodoStats::default());
        let absent = todo_stats(tempfile::tempdir().unwrap().path(), now());
        assert_eq!(absent, TodoStats::default());
    }

    #[test]
    fn a_pending_path_that_is_a_file_is_flagged_not_clean() {
        let dir = tempfile::tempdir().unwrap();
        let pending = dir.path().join(PENDING_REL);
        std::fs::create_dir_all(pending.parent().unwrap()).unwrap();
        std::fs::write(&pending, "not a directory").unwrap();
        let stats = todo_stats(dir.path(), now());
        assert_eq!(
            stats,
            TodoStats {
                count: 0,
                oldest_days: None,
                unreadable: true
            }
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_symlinked_todo_is_not_counted() {
        let dir = project(&[("a.md", "# a\n")]);
        let pending = dir.path().join(PENDING_REL);
        std::os::unix::fs::symlink(pending.join("a.md"), pending.join("link.md")).unwrap();
        assert_eq!(todo_stats(dir.path(), now()).count, 1);
    }

    #[test]
    fn the_count_is_capped_at_the_listing_cap() {
        let dir = tempfile::tempdir().unwrap();
        let pending = dir.path().join(PENDING_REL);
        std::fs::create_dir_all(&pending).unwrap();
        for n in 0..(crate::oneoff::todos::MAX_TODOS + 5) {
            std::fs::write(pending.join(format!("t{n:04}.md")), "# t\n").unwrap();
        }
        assert_eq!(
            todo_stats(dir.path(), now()).count,
            crate::oneoff::todos::MAX_TODOS
        );
    }
}
