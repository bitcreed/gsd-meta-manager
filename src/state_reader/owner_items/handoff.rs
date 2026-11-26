//! Layer A for the pause files: `HANDOFF.json` / `HANDOFF.md` per scope and
//! `phases/<p>/.continue-here.md` checkpoints (Phase 35, D-05, D-06, D-10).
//!
//! gsd-core has no audit reader for these files, so there is no oracle to port: the
//! field set follows gsd-core's `pause-work` template (`status`, `next_action`,
//! `timestamp`, `phase`, `human_actions_pending`, `blockers`) and the reader is
//! fixture- and unit-tested only.
//!
//! # Numbered facts
//!
//! 1. **Native and capped.** Every file goes through the same containment check and
//!    `open_checked` (no final symlink, regular file, 1 MiB cap) as Layer A's other
//!    readers. A file that cannot be read is kept as an unreadable record, never
//!    dropped (fail-safe).
//! 2. **Handoffs.** Per scope (the project and each workstream) `HANDOFF.json` wins
//!    over `HANDOFF.md`; an empty or whitespace-only file is no handoff, as in
//!    `detect_handoff`. Staleness is not decided here: the record keeps its written
//!    time and Layer B compares it with the scope's `stale_handoff` and the caller's
//!    `stale_days`.
//! 3. **Checkpoints.** Only phase directories of the current milestone are read. A
//!    checkpoint of a ROADMAP-complete phase, or whose `status` is terminal, is not a
//!    record.
//! 4. **Untrusted text.** Every string is project text; Layer B passes it on and the
//!    renderers escape it. The phase is reduced to a command-safe token or `""`.

use std::path::Path;

use chrono::{DateTime, Utc};
use serde_json::Value;

use super::rules::is_command_token;
use super::scan::{
    contained, current_phase_dirs, dir_token, open_checked, posix_relative, read_text,
};
use super::{frontmatter, Unreadable};
use crate::state_reader::ProjectState;

/// The most entries read from one `human_actions_pending` or `blockers` list.
const MAX_ENTRIES: usize = 200;

/// Frontmatter statuses that end a checkpoint ([INFERRED] A8).
const TERMINAL_STATUSES: [&str; 4] = ["complete", "completed", "resolved", "done"];

/// One entry of a handoff list: its name and the text that explains it.
pub(super) type Entry = (String, Option<String>);

/// One non-empty `HANDOFF.json` or `HANDOFF.md`.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct HandoffRecord {
    /// POSIX path relative to the project root.
    pub rel_path: String,
    /// The workstream directory name, `None` for the project's own tree.
    pub workstream: Option<String>,
    /// Lower-cased `status`, `unknown` when absent (always for HANDOFF.md).
    pub status: String,
    /// `next_action` (JSON) or the first content line (MD).
    pub next_action: Option<String>,
    /// The `phase` when it is a command-safe token, else `""`.
    pub phase: String,
    pub actions: Vec<Entry>,
    pub blockers: Vec<Entry>,
    /// The JSON `timestamp`.
    pub written: Option<DateTime<Utc>>,
    pub modified: Option<DateTime<Utc>>,
    pub unreadable: bool,
}

/// One `.continue-here.md` of an open phase.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct CheckpointRecord {
    /// POSIX path relative to the project root.
    pub rel_path: String,
    pub workstream: Option<String>,
    /// The phase directory's leading token.
    pub phase: String,
    pub phase_dir: String,
    /// Lower-cased `status`, `unknown` when absent.
    pub status: String,
    /// Frontmatter `phase`, else the directory name.
    pub name: String,
    pub written: Option<DateTime<Utc>>,
    pub modified: Option<DateTime<Utc>>,
    pub unreadable: bool,
}

/// A file's text and modification time.
type FileText = (String, Option<DateTime<Utc>>);

/// The text of one file under `tree`, or why it is not available. `Ok(None)` means
/// the file is absent or escapes the tree (treated as absent).
fn read_contained(root_real: &Path, tree: &Path, path: &Path) -> Result<Option<FileText>, ()> {
    let Some(real) = contained(root_real, tree, path) else {
        return Ok(None);
    };
    let opened = open_checked(&real).map_err(|_: Unreadable| ())?;
    let modified = opened.modified;
    let text = read_text(opened.file).map_err(|_: Unreadable| ())?;
    Ok(Some((text, modified)))
}

fn entries(value: Option<&Value>, name_key: &str, reason_key: &str) -> Vec<Entry> {
    let Some(Value::Array(list)) = value else {
        return Vec::new();
    };
    list.iter()
        .filter_map(|entry| match entry {
            Value::String(text) => {
                let text = text.trim();
                (!text.is_empty()).then(|| (text.to_owned(), None))
            }
            Value::Object(map) => {
                let field = |key: &str| {
                    map.get(key)
                        .and_then(Value::as_str)
                        .map(str::trim)
                        .filter(|text| !text.is_empty())
                        .map(str::to_owned)
                };
                field(name_key).map(|name| (name, field(reason_key)))
            }
            _ => None,
        })
        .take(MAX_ENTRIES)
        .collect()
}

/// The `phase` value as a command-safe token, or `""`.
fn phase_token(value: Option<&Value>) -> String {
    let text = match value {
        Some(Value::String(text)) => text.trim().to_owned(),
        Some(Value::Number(number)) => number.to_string(),
        _ => return String::new(),
    };
    if is_command_token(&text) {
        text
    } else {
        String::new()
    }
}

fn unreadable_handoff(rel_path: String, workstream: Option<String>) -> HandoffRecord {
    HandoffRecord {
        rel_path,
        workstream,
        status: "unknown".to_owned(),
        next_action: None,
        phase: String::new(),
        actions: Vec::new(),
        blockers: Vec::new(),
        written: None,
        modified: None,
        unreadable: true,
    }
}

fn parse_json(
    text: &str,
    rel_path: String,
    workstream: Option<String>,
    modified: Option<DateTime<Utc>>,
) -> HandoffRecord {
    let Ok(value) = serde_json::from_str::<Value>(text) else {
        return unreadable_handoff(rel_path, workstream);
    };
    let text_of = |key: &str| {
        value
            .get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(str::to_owned)
    };
    HandoffRecord {
        status: text_of("status").map_or_else(|| "unknown".to_owned(), |s| s.to_lowercase()),
        next_action: text_of("next_action"),
        phase: phase_token(value.get("phase")),
        actions: entries(value.get("human_actions_pending"), "action", "context"),
        blockers: entries(value.get("blockers"), "description", "workaround"),
        written: text_of("timestamp").and_then(|stamp| frontmatter::moment(&stamp)),
        rel_path,
        workstream,
        modified,
        unreadable: false,
    }
}

fn parse_markdown(
    text: &str,
    rel_path: String,
    workstream: Option<String>,
    modified: Option<DateTime<Utc>>,
) -> HandoffRecord {
    HandoffRecord {
        rel_path,
        workstream,
        status: "unknown".to_owned(),
        next_action: text
            .lines()
            .map(str::trim)
            .find(|line| !line.is_empty() && !line.starts_with('#'))
            .map(str::to_owned),
        phase: String::new(),
        actions: Vec::new(),
        blockers: Vec::new(),
        written: None,
        modified,
        unreadable: false,
    }
}

/// The scope trees of the project: the planning directory and each workstream's.
fn trees<'a>(
    planning_dir: &'a Path,
    state: &'a ProjectState,
) -> impl Iterator<Item = (Option<&'a str>, std::path::PathBuf, &'a ProjectState)> {
    state.scopes().map(move |(workstream, scope)| {
        let tree = match workstream {
            None => planning_dir.to_path_buf(),
            Some(stream) => planning_dir.join("workstreams").join(&stream.name),
        };
        (workstream.map(|stream| stream.name.as_str()), tree, scope)
    })
}

/// D-05, D-10: the handoff of the project's own tree and of each workstream.
pub(super) fn read_handoffs(planning_dir: &Path, state: &ProjectState) -> Vec<HandoffRecord> {
    let root = planning_dir.parent().unwrap_or(planning_dir);
    let Ok(root_real) = planning_dir.canonicalize() else {
        return Vec::new();
    };
    let mut records = Vec::new();
    for (workstream, tree, _) in trees(planning_dir, state) {
        let workstream = workstream.map(str::to_owned);
        for (name, json) in [("HANDOFF.json", true), ("HANDOFF.md", false)] {
            let path = tree.join(name);
            let rel_path = posix_relative(root, &path);
            match read_contained(&root_real, &tree, &path) {
                Ok(None) => {}
                Ok(Some((text, modified))) => {
                    if text.trim().is_empty() {
                        continue;
                    }
                    records.push(if json {
                        parse_json(&text, rel_path, workstream.clone(), modified)
                    } else {
                        parse_markdown(&text, rel_path, workstream.clone(), modified)
                    });
                    break;
                }
                Err(()) => {
                    // The file exists but cannot be read: it is still a pause.
                    records.push(unreadable_handoff(rel_path, workstream.clone()));
                    break;
                }
            }
        }
    }
    records
}

/// D-06: the open `.continue-here.md` checkpoints of the current milestone's phases.
pub(super) fn read_checkpoints(planning_dir: &Path, state: &ProjectState) -> Vec<CheckpointRecord> {
    let root = planning_dir.parent().unwrap_or(planning_dir);
    let Ok(root_real) = planning_dir.canonicalize() else {
        return Vec::new();
    };
    let mut records = Vec::new();
    for (workstream, tree, scope) in trees(planning_dir, state) {
        let phases_dir = tree.join("phases");
        for dir in current_phase_dirs(&phases_dir, scope) {
            let phase = dir_token(&dir);
            if scope
                .roadmap_phase(&phase)
                .is_some_and(|roadmap| roadmap.completed)
            {
                continue;
            }
            let phase_path = phases_dir.join(&dir);
            let path = phase_path.join(".continue-here.md");
            let rel_path = posix_relative(root, &path);
            let mut record = CheckpointRecord {
                rel_path,
                workstream: workstream.map(str::to_owned),
                phase,
                phase_dir: dir.clone(),
                status: "unknown".to_owned(),
                name: dir,
                written: None,
                modified: None,
                unreadable: false,
            };
            match read_contained(&root_real, &phase_path, &path) {
                Ok(None) => continue,
                Err(()) => record.unreadable = true,
                Ok(Some((text, modified))) => {
                    record.modified = modified;
                    let map = frontmatter::extract(&text);
                    if let Some(status) = frontmatter::fm_str(&map, "status")
                        .map(|s| s.trim().to_lowercase())
                        .filter(|s| !s.is_empty())
                    {
                        record.status = status;
                    }
                    if TERMINAL_STATUSES.contains(&record.status.as_str()) {
                        continue;
                    }
                    if let Some(name) = frontmatter::fm_str(&map, "phase")
                        .map(|s| s.trim().to_owned())
                        .filter(|s| !s.is_empty())
                    {
                        record.name = name;
                    }
                    record.written = frontmatter::fm_str(&map, "last_updated")
                        .and_then(|stamp| frontmatter::moment(&stamp));
                }
            }
            records.push(record);
        }
    }
    records
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_reader::parse_project_state;
    use std::fs;

    fn write(path: &Path, text: &str) {
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn project() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        let planning = dir.path().join(".planning");
        write(
            &planning.join("ROADMAP.md"),
            "# Roadmap\n\n## Phases\n\n- [x] **Phase 01: Done** - x\n- [ ] **Phase 02: Open** - y\n\n### Phase 01: Done\n\n**Goal**: g\n\n### Phase 02: Open\n\n**Goal**: g\n",
        );
        write(
            &planning.join("STATE.md"),
            "---\nstatus: in_progress\n---\n",
        );
        dir
    }

    fn handoffs(root: &Path) -> Vec<HandoffRecord> {
        let planning = root.join(".planning");
        let state = parse_project_state(&planning);
        read_handoffs(&planning, &state)
    }

    fn checkpoints(root: &Path) -> Vec<CheckpointRecord> {
        let planning = root.join(".planning");
        let state = parse_project_state(&planning);
        read_checkpoints(&planning, &state)
    }

    #[test]
    fn json_wins_over_markdown_and_an_empty_file_is_nothing() {
        let dir = project();
        let planning = dir.path().join(".planning");
        write(&planning.join("HANDOFF.json"), "  \n");
        write(&planning.join("HANDOFF.md"), "# Handoff\n\nResume here\n");
        let found = handoffs(dir.path());
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].rel_path, ".planning/HANDOFF.md");
        assert_eq!(found[0].next_action.as_deref(), Some("Resume here"));
        assert_eq!(found[0].status, "unknown");
        write(&planning.join("HANDOFF.json"), "{\"status\": \"Paused\"}");
        let found = handoffs(dir.path());
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].rel_path, ".planning/HANDOFF.json");
        assert_eq!(found[0].status, "paused");
    }

    #[test]
    fn invalid_json_is_an_unreadable_record() {
        let dir = project();
        write(&dir.path().join(".planning/HANDOFF.json"), "{not json");
        let found = handoffs(dir.path());
        assert_eq!(found.len(), 1);
        assert!(found[0].unreadable);
        assert!(found[0].actions.is_empty());
    }

    #[test]
    fn the_size_cap_is_one_mebibyte_inclusive() {
        let dir = project();
        let path = dir.path().join(".planning/HANDOFF.json");
        let mut text = String::from("{\"status\":\"paused\"}");
        text.push_str(&" ".repeat((1 << 20) - text.len()));
        assert_eq!(text.len(), 1 << 20);
        write(&path, &text);
        let found = handoffs(dir.path());
        assert!(!found[0].unreadable);
        assert_eq!(found[0].status, "paused");
        text.push(' ');
        write(&path, &text);
        let found = handoffs(dir.path());
        assert_eq!(found.len(), 1);
        assert!(found[0].unreadable);
        assert_eq!(found[0].status, "unknown");
    }

    #[cfg(unix)]
    #[test]
    fn a_symlinked_handoff_is_never_followed_out_of_the_tree() {
        let dir = project();
        let outside = tempfile::tempdir().unwrap();
        write(
            &outside.path().join("secret.json"),
            "{\"status\":\"paused\"}",
        );
        std::os::unix::fs::symlink(
            outside.path().join("secret.json"),
            dir.path().join(".planning/HANDOFF.json"),
        )
        .unwrap();
        assert!(handoffs(dir.path()).is_empty());
    }

    #[test]
    fn the_phase_is_a_token_or_empty() {
        for (raw, expect) in [
            ("4", "4"),
            ("\"04\"", "04"),
            ("\"../x\"", ""),
            ("\"Phase 12\"", ""),
            ("null", ""),
        ] {
            let dir = project();
            write(
                &dir.path().join(".planning/HANDOFF.json"),
                &format!("{{\"phase\": {raw}}}"),
            );
            assert_eq!(handoffs(dir.path())[0].phase, expect, "{raw}");
        }
    }

    #[test]
    fn actions_and_blockers_read_objects_and_strings() {
        let dir = project();
        write(
            &dir.path().join(".planning/HANDOFF.json"),
            r#"{"human_actions_pending":[{"action":"Pair it","context":"bench"},"Call the vendor",7],
                "blockers":[], "timestamp":"2026-05-14T09:04:46.076Z"}"#,
        );
        let found = &handoffs(dir.path())[0];
        assert_eq!(
            found.actions,
            vec![
                ("Pair it".to_owned(), Some("bench".to_owned())),
                ("Call the vendor".to_owned(), None)
            ]
        );
        assert!(found.blockers.is_empty());
        assert!(found.written.is_some());
        write(
            &dir.path().join(".planning/HANDOFF.json"),
            r#"{"blockers":[{"description":"No key","workaround":"sandbox"}],"timestamp":"2026-05-14"}"#,
        );
        let found = &handoffs(dir.path())[0];
        assert_eq!(
            found.blockers,
            vec![("No key".to_owned(), Some("sandbox".to_owned()))]
        );
        assert_eq!(
            found.written.unwrap().to_rfc3339(),
            "2026-05-14T00:00:00+00:00"
        );
    }

    #[test]
    fn checkpoint_statuses_and_milestone_filter() {
        let dir = project();
        let phases = dir.path().join(".planning/phases");
        write(
            &phases.join("01-done/.continue-here.md"),
            "---\nstatus: x\n---\n",
        );
        write(
            &phases.join("02-open/.continue-here.md"),
            "---\nphase: 02-open\nstatus: planning_incomplete\nlast_updated: 2026-11-01T00:00:00Z\n---\n",
        );
        write(
            &phases.join("07-outside/.continue-here.md"),
            "---\nstatus: x\n---\n",
        );
        let found = checkpoints(dir.path());
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].phase, "02");
        assert_eq!(found[0].status, "planning_incomplete");
        assert!(found[0].written.is_some());
        for status in ["complete", "Completed", "resolved", "done"] {
            write(
                &phases.join("02-open/.continue-here.md"),
                &format!("---\nstatus: {status}\n---\n"),
            );
            assert!(checkpoints(dir.path()).is_empty(), "{status}");
        }
        write(
            &phases.join("02-open/.continue-here.md"),
            "no frontmatter\n",
        );
        let found = checkpoints(dir.path());
        assert_eq!(found[0].status, "unknown");
        assert_eq!(found[0].name, "02-open");
    }

    #[test]
    fn an_unreadable_checkpoint_is_kept() {
        let dir = project();
        let path = dir
            .path()
            .join(".planning/phases/02-open/.continue-here.md");
        write(&path, &"x".repeat((1 << 20) + 1));
        let found = checkpoints(dir.path());
        assert_eq!(found.len(), 1);
        assert!(found[0].unreadable);
    }
}
