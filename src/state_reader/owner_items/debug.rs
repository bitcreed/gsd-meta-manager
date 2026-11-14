//! Layer A for debug sessions: a port of gsd-core 1.21.0's `scanDebugSessions`
//! (`audit.cjs` 272-340) for the `audit-open` oracle (Phase 35, D-02, D-10).
//!
//! # Numbered facts
//!
//! 1. **Root only.** The directory read is `<planning>/debug`, never a workstream's:
//!    upstream's `debugDir` is root-scoped, so a workstream cannot hold sessions of
//!    its own and none is listed twice.
//! 2. **Which files.** Regular files by `lstat` (a symlink and `resolved/` are
//!    skipped), named `*.md` (case-sensitive), never exactly `knowledge-base.md`.
//! 3. **Status.** The frontmatter `status` as a FAILSAFE string, `unknown` when absent
//!    or empty, lower-cased and **not trimmed** (`resolved ` stays open upstream).
//!    `resolved` and `complete` are dropped before the acknowledgment check.
//! 4. **Acknowledgment.** `audit_acknowledged` with a `status` snapshot equal to the
//!    status above ([`frontmatter::is_acknowledged`]).
//! 5. **Fail-safe, beyond upstream.** A file that cannot be read stays in the list
//!    marked unreadable (upstream skips it; Layer B turns it into Fix work, and the
//!    oracle comparison excludes it). A frontmatter fence that is closed but not YAML
//!    keeps the Layer A status `unknown` (as upstream) and additionally records a
//!    `recovered_status` from a column-0 `status:` line of the fence, which only
//!    Layer B reads.

use std::path::Path;
use std::sync::Arc;

use super::markdown;
use super::scan::{contained, open_checked, posix_relative, read_text};
use super::{frontmatter, DebugSession, OpenKind, ParseCache, OPEN_CACHE};

/// What the text of one session alone determines.
#[derive(Debug, Clone, PartialEq)]
pub(crate) struct ParsedSession {
    pub status: String,
    pub trigger: Option<String>,
    pub hypothesis: Option<String>,
    pub updated: Option<chrono::DateTime<chrono::Utc>>,
    pub acknowledged: bool,
    pub fm_unparseable: bool,
    pub recovered_status: Option<String>,
}

/// The column-0 `status:` line of a frontmatter body: value trimmed, one pair of
/// quotes dropped, empty is `None`.
fn recover_status(body: &str) -> Option<String> {
    let line = body.lines().find(|line| line.starts_with("status:"))?;
    let value = line["status:".len()..].trim();
    let value = ['"', '\'']
        .iter()
        .find_map(|quote| {
            value
                .strip_prefix(*quote)
                .and_then(|rest| rest.strip_suffix(*quote))
        })
        .unwrap_or(value)
        .trim();
    (!value.is_empty()).then(|| value.to_owned())
}

/// The first line of the `## Current Focus` section, at most 100 characters.
fn hypothesis_of(text: &str) -> Option<String> {
    let lines: Vec<&str> = text.split('\n').collect();
    let section = markdown::collect_section(
        &lines,
        |heading| {
            heading.level == 2
                && heading
                    .text
                    .trim()
                    .to_lowercase()
                    .starts_with("current focus")
        },
        true,
    )?;
    let first = section.body.trim().split('\n').next()?.trim().to_owned();
    let capped: String = first.chars().take(100).collect();
    (!capped.is_empty()).then_some(capped)
}

/// Parse the text of one session file.
pub(crate) fn parse_session(text: &str) -> ParsedSession {
    let checked = frontmatter::extract_checked(text);
    let fm_unparseable = checked.is_err();
    let fm = checked.unwrap_or_default();
    let status = frontmatter::fm_str(&fm, "status")
        .filter(|status| !status.is_empty())
        .unwrap_or_else(|| "unknown".to_owned())
        .to_lowercase();
    let acknowledged = frontmatter::is_acknowledged(&fm, "status", &status);
    let text_of = |key: &str| frontmatter::fm_str(&fm, key).filter(|value| !value.is_empty());
    let updated = text_of("updated")
        .or_else(|| text_of("date"))
        .and_then(|value| frontmatter::moment(&value));
    let recovered_status = if fm_unparseable {
        frontmatter::fence_body(text).and_then(recover_status)
    } else {
        None
    };
    ParsedSession {
        status,
        trigger: text_of("trigger"),
        hypothesis: hypothesis_of(text),
        updated,
        acknowledged,
        fm_unparseable,
        recovered_status,
    }
}

/// Whether Layer A drops a session with this status (`resolved`, `complete`).
fn is_terminal(status: &str) -> bool {
    matches!(status, "resolved" | "complete")
}

/// Every open session of `<planning_dir>/debug`, sorted by file name.
pub(super) fn scan_debug_sessions(planning_dir: &Path) -> Vec<DebugSession> {
    scan_with(&OPEN_CACHE, planning_dir)
}

fn scan_with(
    cache: &ParseCache<OpenKind, ParsedSession>,
    planning_dir: &Path,
) -> Vec<DebugSession> {
    let debug_dir = planning_dir.join("debug");
    let Ok(entries) = std::fs::read_dir(&debug_dir) else {
        return Vec::new();
    };
    let Ok(root_real) = planning_dir.canonicalize() else {
        return Vec::new();
    };
    let project_root = planning_dir.parent().unwrap_or(planning_dir);
    let mut names: Vec<String> = entries
        .flatten()
        .filter(|entry| entry.file_type().is_ok_and(|kind| kind.is_file()))
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.ends_with(".md") && name != "knowledge-base.md")
        .collect();
    names.sort();
    let mut sessions = Vec::new();
    for name in names {
        let path = debug_dir.join(&name);
        let Some(real) = contained(&root_real, &debug_dir, &path) else {
            continue;
        };
        let slug = name.trim_end_matches(".md").to_owned();
        let mut session = DebugSession {
            rel_path: posix_relative(project_root, &path),
            slug,
            status: "unknown".to_owned(),
            trigger: None,
            hypothesis: None,
            updated: None,
            modified: None,
            acknowledged: false,
            unreadable: None,
            fm_unparseable: false,
            recovered_status: None,
        };
        match open_checked(&real) {
            Ok(opened) => {
                session.modified = opened.modified;
                let parsed = match cache.get(&real, OpenKind::Debug, opened.key) {
                    Some(hit) => hit,
                    None => match read_text(opened.file) {
                        Ok(text) => {
                            let parsed = Arc::new(parse_session(&text));
                            cache.insert(real, OpenKind::Debug, opened.key, Arc::clone(&parsed));
                            parsed
                        }
                        Err(reason) => {
                            session.unreadable = Some(reason);
                            sessions.push(session);
                            continue;
                        }
                    },
                };
                let parsed = parsed.as_ref();
                if is_terminal(&parsed.status) {
                    continue;
                }
                session.status = parsed.status.clone();
                session.trigger = parsed.trigger.clone();
                session.hypothesis = parsed.hypothesis.clone();
                session.updated = parsed.updated;
                session.acknowledged = parsed.acknowledged;
                session.fm_unparseable = parsed.fm_unparseable;
                session.recovered_status = parsed.recovered_status.clone();
            }
            Err(reason) => session.unreadable = Some(reason),
        }
        sessions.push(session);
    }
    sessions
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use crate::state_reader::owner_items::{evict_project, CACHE_HITS};
    use crate::state_reader::owner_items::{DebugSession, Unreadable};

    fn write(path: &Path, text: &str) {
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(path, text).unwrap();
    }

    /// A project whose `.planning/debug/` holds the given (name, text) files.
    fn project(files: &[(&str, &str)]) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::tempdir().unwrap();
        let planning = dir.path().join(".planning");
        std::fs::create_dir_all(planning.join("debug")).unwrap();
        for (name, text) in files {
            write(&planning.join("debug").join(name), text);
        }
        (dir, planning)
    }

    fn scan(planning: &Path) -> Vec<DebugSession> {
        scan_with(&ParseCache::new(64), planning)
    }

    fn slugs(sessions: &[DebugSession]) -> Vec<&str> {
        sessions.iter().map(|s| s.slug.as_str()).collect()
    }

    fn status_file(status: &str) -> String {
        format!("---\nstatus: {status}\n---\n\n# Session\n")
    }

    #[test]
    fn ignored_names_are_never_read() {
        let awaiting = status_file("awaiting_human_verify");
        let (_dir, planning) = project(&[
            ("knowledge-base.md", &awaiting),
            ("resolved/old.md", &awaiting),
            ("notes.txt", &awaiting),
            ("NOTES.MD", &awaiting),
            ("real.md", &awaiting),
        ]);
        assert_eq!(slugs(&scan(&planning)), ["real"]);
    }

    #[test]
    fn a_missing_debug_directory_is_an_empty_list() {
        let dir = tempfile::tempdir().unwrap();
        let planning = dir.path().join(".planning");
        std::fs::create_dir_all(&planning).unwrap();
        assert!(scan(&planning).is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn a_symlinked_session_is_skipped() {
        let (dir, planning) = project(&[("real.md", &status_file("diagnosed"))]);
        let outside = dir.path().join("outside.md");
        write(&outside, &status_file("awaiting_human_verify"));
        std::os::unix::fs::symlink(&outside, planning.join("debug/link.md")).unwrap();
        assert_eq!(slugs(&scan(&planning)), ["real"]);
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn a_fifo_named_like_a_session_does_not_hang_the_scan() {
        use crate::state_reader::driver_frontmatter::fifo_test_support::{mkfifo, within_5s};
        let (_dir, planning) = project(&[("real.md", &status_file("diagnosed"))]);
        mkfifo(&planning.join("debug/pipe.md"));
        let sessions = within_5s(move || scan(&planning));
        assert_eq!(slugs(&sessions), ["real"]);
    }

    #[test]
    fn the_size_cap_is_one_mebibyte_inclusive() {
        let mut exact = status_file("diagnosed");
        exact.push_str(&"x".repeat((1usize << 20) - exact.len()));
        assert_eq!(exact.len(), 1 << 20);
        let over = format!("{exact}x");
        let (_dir, planning) = project(&[("exact.md", &exact), ("over.md", &over)]);
        let sessions = scan(&planning);
        assert_eq!(slugs(&sessions), ["exact", "over"]);
        assert_eq!(sessions[0].unreadable, None);
        assert_eq!(sessions[0].status, "diagnosed");
        assert_eq!(sessions[1].unreadable, Some(Unreadable::TooLarge));
    }

    #[test]
    fn status_is_lower_cased_not_trimmed_and_failsafe() {
        let (_dir, planning) = project(&[
            ("a.md", "---\nstatus: Awaiting-User\n---\n"),
            ("b.md", "---\nstatus: \"resolved \"\n---\n"),
            ("c.md", "---\nstatus: 7\n---\n"),
            ("d.md", "no frontmatter at all\n"),
        ]);
        let by_slug: Vec<(&str, String)> = scan(&planning)
            .iter()
            .map(|s| (if s.slug == "a" { "a" } else { "x" }, s.status.clone()))
            .collect();
        assert_eq!(
            by_slug.iter().map(|(_, s)| s.as_str()).collect::<Vec<_>>(),
            ["awaiting-user", "resolved ", "7", "unknown"]
        );
    }

    #[test]
    fn resolved_and_complete_are_dropped_and_never_acknowledged() {
        let acked = "---\nstatus: resolved\naudit_acknowledged:\n  milestone: v1\n  at: 2026-11-01\n  status: resolved\n---\n";
        let (_dir, planning) = project(&[
            ("a.md", &status_file("resolved")),
            ("b.md", &status_file("complete")),
            ("c.md", acked),
        ]);
        assert!(scan(&planning).is_empty());
    }

    #[test]
    fn a_lone_cr_file_parses() {
        let (_dir, planning) = project(&[(
            "a.md",
            "---\rstatus: awaiting_human_verify\rtrigger: Cr case\r---\r\r# Session\r",
        )]);
        let sessions = scan(&planning);
        assert_eq!(sessions[0].status, "awaiting_human_verify");
        assert_eq!(sessions[0].trigger.as_deref(), Some("Cr case"));
    }

    #[test]
    fn an_acknowledgment_matches_on_the_status_snapshot() {
        let acked = "---\nstatus: investigating\naudit_acknowledged:\n  milestone: v1\n  at: 2026-11-01\n  status: investigating\n---\n";
        let changed = acked.replace("status: investigating\naudit", "status: diagnosed\naudit");
        let (_dir, planning) = project(&[("a.md", acked), ("b.md", &changed)]);
        let sessions = scan(&planning);
        assert!(sessions[0].acknowledged);
        assert!(!sessions[1].acknowledged);
    }

    #[test]
    fn an_unparseable_fence_recovers_a_column_zero_status() {
        let broken = |line: &str| {
            format!("---\n{line}\ntrigger: \"Delta stops\" (owner, run 2)\n---\n\n# S\n")
        };
        let (_dir, planning) = project(&[
            ("a.md", &broken("status: awaiting_human_verify")),
            ("b.md", &broken("  status: awaiting_human_verify")),
            ("c.md", &broken("status: \"awaiting-user\"")),
            ("d.md", &broken("mood: calm")),
        ]);
        let sessions = scan(&planning);
        for session in &sessions {
            assert!(session.fm_unparseable, "{}", session.slug);
            assert_eq!(session.status, "unknown", "Layer A keeps upstream's value");
        }
        let recovered: Vec<Option<&str>> = sessions
            .iter()
            .map(|s| s.recovered_status.as_deref())
            .collect();
        assert_eq!(
            recovered,
            [
                Some("awaiting_human_verify"),
                None,
                Some("awaiting-user"),
                None
            ]
        );
    }

    #[test]
    fn the_hypothesis_is_the_first_line_of_current_focus() {
        let text = "---\nstatus: diagnosed\n---\n\n## Current Focus\n\n  first line here  \nsecond\n\n## Other\nno\n";
        let (_dir, planning) = project(&[("a.md", text)]);
        assert_eq!(
            scan(&planning)[0].hypothesis.as_deref(),
            Some("first line here")
        );
    }

    #[test]
    fn updated_falls_back_to_date() {
        let (_dir, planning) = project(&[
            ("a.md", "---\nstatus: x\ndate: 2026-11-10\n---\n"),
            (
                "b.md",
                "---\nstatus: x\nupdated: 2026-11-11T08:00:00Z\ndate: 2026-11-10\n---\n",
            ),
        ]);
        let sessions = scan(&planning);
        assert_eq!(
            sessions[0].updated.unwrap().to_rfc3339(),
            "2026-11-10T00:00:00+00:00"
        );
        assert_eq!(
            sessions[1].updated.unwrap().to_rfc3339(),
            "2026-11-11T08:00:00+00:00"
        );
    }

    #[test]
    fn a_second_scan_reads_from_the_cache_and_evict_project_drops_it() {
        let (dir, planning) = project(&[("a.md", &status_file("diagnosed"))]);
        let before = CACHE_HITS.with(std::cell::Cell::get);
        let first = scan_debug_sessions(&planning);
        assert_eq!(CACHE_HITS.with(std::cell::Cell::get), before, "cold scan");
        let second = scan_debug_sessions(&planning);
        assert_eq!(CACHE_HITS.with(std::cell::Cell::get) - before, 1);
        assert_eq!(first, second);
        evict_project(dir.path());
        let _ = scan_debug_sessions(&planning);
        assert_eq!(
            CACHE_HITS.with(std::cell::Cell::get) - before,
            1,
            "evicted, so the next scan parsed again"
        );
    }
}
