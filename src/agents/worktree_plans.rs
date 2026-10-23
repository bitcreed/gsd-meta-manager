//! Phase plans that exist only in a linked worktree (quick 261014-0jv).
//!
//! A project can keep a phase's `*-PLAN.md` files on a branch checked out in a
//! linked worktree (`.claude/worktrees/lanes-15` on `gsd/phase-15`) while main's
//! `.planning/phases/15-*/` is empty or absent. The Phases tab would then say
//! "no plans" for a phase with seven executed plans. This module finds those
//! plans so the screen can browse them.
//!
//! **Contract.**
//!
//! * Read-only, and it runs no git: it consumes the caller's [`CoreScan`], the
//!   one `git worktree list` the scan already ran.
//! * File content is read ONLY through
//!   [`crate::state_reader::disk_status::infer_disk_status`], on a winning
//!   holder's phase directory (the first agents detector that reads content —
//!   plan titles, waves and tokens need it). Candidate discovery and the main
//!   check read entry names only.
//! * Display-only: the result lives on `ProjectAgents` / `AgentView` and is
//!   never merged into `ProjectState` or `phase_disk_statuses`, so the driver
//!   router and the DRIVE-05 gates cannot act on unmerged worktree content.
//! * Main precedence: a phase key is skipped when main's `phases/<id>-*` dir
//!   holds any `PLAN.md` / `*-PLAN.md`, or when main archives the key under
//!   `milestones/*-phases/` (an archived phase is "held" by main).
//!
//! The same rules apply inside a workstream's planning directory
//! ([`detect_in`] with `.planning/workstreams/<name>`).

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use super::unmerged::{names, short_ref, subdirs};
use super::worktree_phases::split_dir;
use super::worktrees;
use crate::state_reader::disk_status::{infer_disk_status, DiskInference};
use crate::state_reader::phase_num::{phase_key, PhaseNum};
use crate::text::Untrusted;

/// One phase whose plans only a linked worktree holds, with that worktree's
/// own disk inference.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct WorktreePlans {
    /// The `phase_key` of the directory's numeric id (`15`, `11.1`); reader
    /// generated, never third-party text.
    pub key: String,
    /// The holder worktree's root.
    pub worktree: PathBuf,
    /// The holder's `.planning/phases/<dir>`.
    pub phase_dir: PathBuf,
    /// The holder path's last component (`lanes-15`), untrusted.
    pub worktree_name: Untrusted,
    /// git's short branch name (`gsd/phase-15`); `None` for a detached holder.
    pub branch: Option<Untrusted>,
    /// The `@ref` form (`phase-15`) the Roadmap badge uses.
    pub short_ref: Untrusted,
    /// How many OTHER holders also had plans for this key and lost (see
    /// [`detect`] for the winner rule: newest PLAN mtime, then smallest path,
    /// superseded-only holders falling through). The pane title shows it as
    /// ` +N more`.
    pub others: u32,
    /// `infer_disk_status` of [`WorktreePlans::phase_dir`]: the same plans,
    /// waves, summarized plans and tokens a main phase would carry.
    pub inference: DiskInference,
}

/// `PLAN.md` or `<anything>-PLAN.md`.
fn is_plan_name(name: &str) -> bool {
    name == "PLAN.md" || name.ends_with("-PLAN.md")
}

/// The phase keys main holds: every `phases/` subdirectory whose entry names
/// include a plan file, plus every key under `milestones/*-phases/` regardless
/// of content (archived means main holds it).
pub(crate) fn main_plan_keys(main_planning: &Path) -> BTreeSet<String> {
    let mut keys = BTreeSet::new();
    for (name, dir) in subdirs(&main_planning.join("phases")) {
        if let Some((id, _)) = split_dir(&name) {
            if names(&dir).iter().any(|n| is_plan_name(n)) {
                keys.insert(phase_key(id));
            }
        }
    }
    for (name, dir) in subdirs(&main_planning.join("milestones")) {
        if name.ends_with("-phases") {
            for (phase, _) in subdirs(&dir) {
                if let Some((id, _)) = split_dir(&phase) {
                    keys.insert(phase_key(id));
                }
            }
        }
    }
    keys
}

/// The newest `*-PLAN.md` mtime in `dir` (an unreadable mtime ranks oldest).
fn newest_plan_mtime(dir: &Path, entries: &[String]) -> SystemTime {
    entries
        .iter()
        .filter(|n| is_plan_name(n))
        .filter_map(|n| std::fs::metadata(dir.join(n)).ok()?.modified().ok())
        .max()
        .unwrap_or(SystemTime::UNIX_EPOCH)
}

/// One live worktree's phase dir that holds plans for a key main lacks.
struct Candidate<'a> {
    wt: &'a worktrees::CoreWorktree,
    dir: PathBuf,
    newest: SystemTime,
}

/// Every phase main holds no plans for but a linked worktree does, one entry
/// per phase key, sorted by phase number. Empty without reading main when no
/// live linked worktree exists.
///
/// **Winner rule [inferred ID-4].** Several worktrees can hold plans for one
/// key. They are ranked by the newest `*-PLAN.md` mtime in their phase dir
/// (descending; an unreadable mtime ranks oldest), then by ascending worktree
/// path, so two scans of the same disk agree. The first holder whose
/// [`DiskInference::plan_count`] is above zero wins: a holder whose plans are
/// all superseded falls through to the next. [`WorktreePlans::others`] counts
/// the remaining candidates, which the pane title shows as ` +N more`.
pub fn detect(core: &worktrees::CoreScan, project_root: &Path) -> Vec<WorktreePlans> {
    detect_in(core, project_root, Path::new(".planning"))
}

/// [`detect`] over one scope's planning directory: `planning_rel` is joined
/// onto the main root and onto each worktree path (`.planning` is the project,
/// `.planning/workstreams/<name>` one workstream; quick 261023-05i).
pub fn detect_in(
    core: &worktrees::CoreScan,
    project_root: &Path,
    planning_rel: &Path,
) -> Vec<WorktreePlans> {
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
    let main = main_plan_keys(&main_root.join(planning_rel));
    let agent_ids: Vec<&str> = core
        .worktrees
        .iter()
        .filter_map(|wt| wt.agent_id.as_deref())
        .collect();

    // Names-only discovery: every holder's candidate dirs, grouped by key.
    let mut by_key: BTreeMap<PhaseNum, (String, Vec<Candidate<'_>>)> = BTreeMap::new();
    for wt in live {
        for (dir_name, dir) in subdirs(&wt.path.join(planning_rel).join("phases")) {
            let Some((id, _)) = split_dir(&dir_name) else {
                continue;
            };
            let key = phase_key(id);
            if main.contains(&key) {
                continue;
            }
            let entries = names(&dir);
            if !entries.iter().any(|n| is_plan_name(n)) {
                continue;
            }
            let Some(num) = PhaseNum::parse(id) else {
                continue;
            };
            let newest = newest_plan_mtime(&dir, &entries);
            by_key
                .entry(num)
                .or_insert_with(|| (key, Vec::new()))
                .1
                .push(Candidate { wt, dir, newest });
        }
    }

    let mut out = Vec::new();
    for (key, mut candidates) in by_key.into_values() {
        // `live` is path-sorted and the sort is stable, so equal mtimes keep
        // ascending path order.
        candidates.sort_by(|a, b| {
            b.newest
                .cmp(&a.newest)
                .then_with(|| a.wt.path.cmp(&b.wt.path))
        });
        let total = candidates.len();
        for c in candidates {
            let inference = infer_disk_status(&c.dir);
            if inference.plan_count == 0 {
                continue;
            }
            out.push(WorktreePlans {
                key,
                worktree: c.wt.path.clone(),
                phase_dir: c.dir,
                worktree_name: Untrusted::from_untrusted_source(
                    c.wt.path
                        .file_name()
                        .map(|n| n.to_string_lossy().into_owned())
                        .unwrap_or_default(),
                ),
                branch: c.wt.branch.clone(),
                short_ref: short_ref(c.wt.branch.as_ref(), &c.wt.path, &agent_ids),
                others: u32::try_from(total - 1).unwrap_or(u32::MAX),
                inference,
            });
            break;
        }
    }
    if !out.is_empty() {
        // Counts only: never a branch, path or directory name (D-C13).
        tracing::debug!(phases = out.len(), "worktree plans detected");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plan_names_are_plan_md_or_suffix() {
        assert!(is_plan_name("PLAN.md"));
        assert!(is_plan_name("15-01-alpha-PLAN.md"));
        assert!(!is_plan_name("15-01-SUMMARY.md"));
        assert!(!is_plan_name("15-CONTEXT.md"));
        assert!(!is_plan_name("PLAN.md.bak"));
    }

    #[test]
    fn main_plan_keys_cover_plans_and_archives_and_pad_insensitive() {
        let tmp = tempfile::tempdir().unwrap();
        let touch = |rel: &str| {
            let p = tmp.path().join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, "x").unwrap();
        };
        // empty dir: not held
        std::fs::create_dir_all(tmp.path().join("phases/15-empty")).unwrap();
        // context only: not held
        touch("phases/16-ctx/16-CONTEXT.md");
        // plan dir, zero-padded: held under the unpadded key
        touch("phases/09-a/09-01-PLAN.md");
        // standalone PLAN.md
        touch("phases/11.1-b/PLAN.md");
        // archived: held regardless of content
        std::fs::create_dir_all(tmp.path().join("milestones/v1-phases/03-c")).unwrap();
        std::fs::create_dir_all(tmp.path().join("milestones/v1-notes/04-d")).unwrap();

        let keys = main_plan_keys(tmp.path());
        assert!(keys.contains("9") && keys.contains("11.1") && keys.contains("3"));
        assert!(!keys.contains("15") && !keys.contains("16") && !keys.contains("4"));
    }
}
