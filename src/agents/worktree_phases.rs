//! Roadmap phases that exist only in a linked worktree (todo
//! 2026-10-01-show-worktree-only-roadmap-phases-in-project-view).
//!
//! **Read-only, names only** — the same contract as [`super::unmerged`]. A
//! phase is worktree-only when a non-main, non-prunable linked worktree's
//! `.planning/phases/` holds a `<number>-<slug>/` directory whose number main
//! holds in neither `.planning/phases/` nor `.planning/milestones/*-phases/`.
//! No file content is opened and no git is run, so a worktree's ROADMAP.md is
//! never parsed (a phase added to a worktree's ROADMAP but with no directory yet
//! is not shown; the directory is what `discuss-phase` creates first).
//!
//! The stage is a ladder over entry names: `*-SUMMARY.md` (executing),
//! `*-PLAN.md` (planned), `*-RESEARCH.md` (researched), `*-CONTEXT.md`
//! (context), else started. Main stays authoritative: the renderer additionally
//! drops any phase main's ROADMAP already lists. The directory slug is the name
//! and stays [`Untrusted`].

use std::collections::BTreeSet;
use std::path::PathBuf;

use super::unmerged::{item_state, names, short_ref, subdirs, UnmergedState};
use super::{worktrees, AgentRow};
use crate::state_reader::phase_num::{phase_key, PhaseNum};
use crate::text::Untrusted;

/// How far a worktree-only phase has progressed, from entry names alone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum WorktreeStage {
    Started,
    Context,
    Researched,
    Planned,
    Executing,
}

impl WorktreeStage {
    pub fn label(self) -> &'static str {
        match self {
            WorktreeStage::Started => "started",
            WorktreeStage::Context => "context",
            WorktreeStage::Researched => "researched",
            WorktreeStage::Planned => "planned",
            WorktreeStage::Executing => "executing",
        }
    }
}

/// One phase only a linked worktree holds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreePhase {
    /// The phase number as the directory spells it (`12`, `11.1`).
    pub id: String,
    /// The directory slug (`intake-throttle-output-test`), untrusted.
    pub name: Untrusted,
    pub stage: WorktreeStage,
    pub plans_done: u32,
    pub plans_total: u32,
    /// The first holder worktree (by path).
    pub worktree: PathBuf,
    pub short_ref: Untrusted,
    /// Whether the holder is still in use (same rule as unmerged work).
    pub state: UnmergedState,
}

/// Split `12-intake-throttle` into (`12`, `intake-throttle`); `None` when the
/// directory does not start with a numeric phase id followed by `-`.
fn split_dir(name: &str) -> Option<(&str, &str)> {
    let (id, slug) = name.split_once('-')?;
    PhaseNum::parse(id)?;
    Some((id, slug))
}

fn main_keys(main_planning: &std::path::Path) -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    let mut add = |root: &std::path::Path| {
        for (name, _) in subdirs(root) {
            if let Some((id, _)) = split_dir(&name) {
                keys.insert(phase_key(id));
            }
        }
    };
    add(&main_planning.join("phases"));
    for (name, dir) in subdirs(&main_planning.join("milestones")) {
        if name.ends_with("-phases") {
            add(&dir);
        }
    }
    keys
}

/// Classify a phase directory's entry names.
fn stage_of(entries: &[String]) -> (WorktreeStage, u32, u32) {
    let count = |suffix: &str| {
        let n = entries.iter().filter(|n| n.ends_with(suffix)).count();
        u32::try_from(n).unwrap_or(u32::MAX)
    };
    let plans = entries
        .iter()
        .filter(|n| n.ends_with("-PLAN.md") && !n.contains("-FIX-"))
        .count();
    let done = entries
        .iter()
        .filter(|n| n.ends_with("-SUMMARY.md") && !n.contains("-FIX-"))
        .count();
    let plans = u32::try_from(plans).unwrap_or(u32::MAX);
    let done = u32::try_from(done).unwrap_or(u32::MAX);
    let stage = if done > 0 {
        WorktreeStage::Executing
    } else if plans > 0 {
        WorktreeStage::Planned
    } else if count("-RESEARCH.md") > 0 {
        WorktreeStage::Researched
    } else if count("-CONTEXT.md") > 0 {
        WorktreeStage::Context
    } else {
        WorktreeStage::Started
    };
    (stage, done, plans)
}

/// Every worktree-only phase across a project's linked worktrees, one per
/// phase key (first holder by path), sorted by phase number. Empty without
/// reading main when no linked worktree exists.
pub fn detect(
    core: &worktrees::CoreScan,
    rows: &[AgentRow],
    project_root: &std::path::Path,
) -> Vec<WorktreePhase> {
    let main_root = core.main_worktree.as_deref().unwrap_or(project_root);
    let mut live: Vec<&worktrees::CoreWorktree> = core
        .worktrees
        .iter()
        .filter(|wt| !wt.prunable && wt.path != main_root)
        .collect();
    if live.is_empty() {
        return Vec::new();
    }
    live.sort_by(|a, b| a.path.cmp(&b.path));
    let main = main_keys(&main_root.join(".planning"));
    let agent_ids: Vec<&str> = core
        .worktrees
        .iter()
        .filter_map(|wt| wt.agent_id.as_deref())
        .collect();

    let mut found: Vec<(PhaseNum, WorktreePhase)> = Vec::new();
    for wt in live {
        for (dir_name, dir) in subdirs(&wt.path.join(".planning").join("phases")) {
            let Some((id, slug)) = split_dir(&dir_name) else {
                continue;
            };
            let key = phase_key(id);
            if main.contains(&key) || found.iter().any(|(_, p)| phase_key(&p.id) == key) {
                continue;
            }
            let Some(num) = PhaseNum::parse(id) else {
                continue;
            };
            let (stage, plans_done, plans_total) = stage_of(&names(&dir));
            let row = rows.iter().find(|r| r.path == wt.path);
            found.push((
                num,
                WorktreePhase {
                    id: id.to_string(),
                    name: Untrusted::from_untrusted_source(slug.to_string()),
                    stage,
                    plans_done,
                    plans_total,
                    worktree: wt.path.clone(),
                    short_ref: short_ref(wt.branch.as_ref(), &wt.path, &agent_ids),
                    state: item_state(row),
                },
            ));
        }
    }
    found.sort_by(|a, b| a.0.cmp(&b.0));
    if !found.is_empty() {
        // Counts only: never a branch, path or directory name (D-C13).
        tracing::debug!(phases = found.len(), "worktree-only phases detected");
    }
    found.into_iter().map(|(_, p)| p).collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn n(v: &[&str]) -> Vec<String> {
        v.iter().map(|s| s.to_string()).collect()
    }

    #[test]
    fn the_stage_ladder_reads_names() {
        assert_eq!(stage_of(&n(&["14-CONTEXT.md"])).0, WorktreeStage::Context);
        assert_eq!(
            stage_of(&n(&["13-CONTEXT.md", "13-RESEARCH.md"])).0,
            WorktreeStage::Researched
        );
        assert_eq!(
            stage_of(&n(&["12-RESEARCH.md", "12-01-PLAN.md", "12-02-PLAN.md"])),
            (WorktreeStage::Planned, 0, 2)
        );
        assert_eq!(
            stage_of(&n(&["12-01-PLAN.md", "12-01-SUMMARY.md"])),
            (WorktreeStage::Executing, 1, 1)
        );
        assert_eq!(stage_of(&n(&[])).0, WorktreeStage::Started);
    }

    #[test]
    fn dir_names_split_on_a_numeric_id() {
        assert_eq!(split_dir("11.1-hotfix"), Some(("11.1", "hotfix")));
        assert_eq!(split_dir("12-intake-throttle"), Some(("12", "intake-throttle")));
        assert_eq!(split_dir("notes-x"), None);
        assert_eq!(split_dir("12"), None);
    }

    #[test]
    fn main_keys_cover_phases_and_archives_and_pad_insensitive() {
        let tmp = tempfile::tempdir().unwrap();
        for rel in [
            "phases/09-a",
            "phases/11.1-b",
            "milestones/v1-phases/03-c",
            "milestones/v1-notes/04-d",
        ] {
            std::fs::create_dir_all(tmp.path().join(rel)).unwrap();
        }
        let keys = main_keys(tmp.path());
        assert!(keys.contains("9") && keys.contains("11.1") && keys.contains("3"));
        assert!(!keys.contains("4"));
    }
}
