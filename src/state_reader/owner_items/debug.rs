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
