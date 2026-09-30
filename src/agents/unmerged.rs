//! Completed-but-unmerged worktree work (quick 260929-szq).
//!
//! **Read-only, names only.** This module lists directory entries with
//! `std::fs::read_dir` and nothing else: no file is written, created or
//! opened for its contents, no process is spawned and no git is run. The only
//! git facts it uses (commits ahead, dirty paths) come from the existing
//! [`worktrees::worktree_counts`], which goes through `git_ops` with no
//! optional lock, and only for a worktree that holds unmerged work (I-3).
//!
//! **The signal is path-level (I-1).** A plan is unmerged when a linked
//! worktree's `.planning/phases/*/` holds its `*-SUMMARY.md` and main's
//! `.planning/phases/*/` and `.planning/milestones/*-phases/*/` do not. A quick
//! task is unmerged when a worktree's `.planning/quick/<id>-*/` holds a
//! `*-SUMMARY.md` and no main quick dir with that id does (I-5). Commit
//! ancestry is never consulted: a squash- or rebase-merged branch stays
//! "ahead" of main forever, while a SUMMARY that reached main — however it got
//! there — clears the item.
//!
//! **Known limits (I-6).** A SUMMARY main deleted outright (rather than
//! archiving it) still flags in a stale worktree, and a renumbered phase whose
//! key collides with an archived one reads as merged.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use super::waves::PlanRef;
use super::{worktrees, AgentLiveness, AgentRow};
use crate::state_reader::disk_status;
use crate::text::Untrusted;

/// The glyph every surface draws for unmerged work (U+25D0, one cell).
pub const GLYPH_UNMERGED: &str = "\u{25D0}";
/// The word every surface pairs with [`GLYPH_UNMERGED`].
pub const LABEL_UNMERGED: &str = "unmerged";
/// The word the Waves pane appends when the holder worktree is still in use
/// ([`UnmergedState::InProgress`], I-13).
pub const LABEL_ACTIVE: &str = "active";

/// What finished on a worktree: a plan, or a quick task by its `YYMMDD-xxx` id.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum UnmergedKey {
    Plan(PlanRef),
    Quick(String),
}

/// Whether the worktree holding unmerged work is still in use (I-4).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum UnmergedState {
    /// Unlocked (or its lock is stale) and no live agent: waiting to merge.
    AwaitingMerge,
    /// Locked by an agent that is not `Ended`, or its agent is `Live`/`Idle`.
    InProgress,
}

/// One unmerged key, with the first worktree (by path) that holds it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnmergedItem {
    pub key: UnmergedKey,
    pub worktree: PathBuf,
    /// The worktree's branch short name, as git reported it.
    pub branch: Option<Untrusted>,
    /// The display ref ([`short_ref`]); drawn only through `shown()`.
    pub short_ref: Untrusted,
    pub commits_ahead: Option<u32>,
    pub dirty: Option<u32>,
    pub state: UnmergedState,
}

/// Every entry name of `dir`, as UTF-8; an unreadable dir is empty.
fn names(dir: &Path) -> Vec<String> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter_map(|e| e.file_name().to_str().map(str::to_string))
        .collect()
}

/// Sub-directory entries of `dir` (following symlinks), with their names.
fn subdirs(dir: &Path) -> Vec<(String, PathBuf)> {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return Vec::new();
    };
    entries
        .flatten()
        .filter(|e| e.path().is_dir())
        .filter_map(|e| {
            let name = e.file_name().to_str()?.to_string();
            Some((name, e.path()))
        })
        .collect()
}

/// The plan a SUMMARY file name completes. FIX and GAPCLOSURE summaries are
/// never plan partners — `worktree_holds_summary`'s exclusion — and a
/// phase-level or remediation summary has no plan index.
fn summary_plan(name: &str) -> Option<PlanRef> {
    if name.contains("-FIX-") || name.ends_with("-GAPCLOSURE-SUMMARY.md") {
        return None;
    }
    let (phase, plan) = name
        .strip_suffix("-SUMMARY.md")
        .and_then(disk_status::plan_index)?;
    Some(PlanRef { phase, plan })
}

/// The `YYMMDD-xxx` id a quick dir name starts with: six digits, a dash, three
/// `[a-z0-9]`, then the end or a dash (I-5). Anything else is not a quick id.
fn quick_id(dir_name: &str) -> Option<&str> {
    let b = dir_name.as_bytes();
    if b.len() < 10 {
        return None;
    }
    let ok = b[..6].iter().all(u8::is_ascii_digit)
        && b[6] == b'-'
        && b[7..10]
            .iter()
            .all(|c| c.is_ascii_digit() || c.is_ascii_lowercase())
        && (b.len() == 10 || b[10] == b'-');
    ok.then(|| &dir_name[..10])
}

/// Whether a quick dir holds any `*-SUMMARY.md`.
fn holds_summary(dir: &Path) -> bool {
    names(dir).iter().any(|n| n.ends_with("-SUMMARY.md"))
}

/// The plan keys every SUMMARY in `phases_root/*/` completes.
fn phase_keys(phases_root: &Path, out: &mut BTreeSet<UnmergedKey>) {
    for (_, dir) in subdirs(phases_root) {
        out.extend(
            names(&dir)
                .iter()
                .filter_map(|n| summary_plan(n))
                .map(UnmergedKey::Plan),
        );
    }
}

/// Main's done keys: plan SUMMARYs under `phases/*/` and
/// `milestones/*-phases/*/`, and quick ids whose `quick/<id>-*/` dir holds a
/// SUMMARY (I-1, I-5, I-6).
pub fn main_done_keys(main_planning: &Path) -> BTreeSet<UnmergedKey> {
    let mut keys = BTreeSet::new();
    phase_keys(&main_planning.join("phases"), &mut keys);
    for (name, dir) in subdirs(&main_planning.join("milestones")) {
        if name.ends_with("-phases") {
            phase_keys(&dir, &mut keys);
        }
    }
    for (name, dir) in subdirs(&main_planning.join("quick")) {
        if let Some(id) = quick_id(&name) {
            if holds_summary(&dir) {
                keys.insert(UnmergedKey::Quick(id.to_string()));
            }
        }
    }
    keys
}

/// A worktree's done keys that `main` does not hold, sorted and distinct.
/// Only `phases/*/` and `quick/*/` are read; a quick dir whose id main already
/// holds is skipped without listing it (I-5's cost prune).
pub fn worktree_done_keys(wt_planning: &Path, main: &BTreeSet<UnmergedKey>) -> Vec<UnmergedKey> {
    let mut keys = BTreeSet::new();
    phase_keys(&wt_planning.join("phases"), &mut keys);
    for (name, dir) in subdirs(&wt_planning.join("quick")) {
        let Some(id) = quick_id(&name) else {
            continue;
        };
        let key = UnmergedKey::Quick(id.to_string());
        if main.contains(&key) || keys.contains(&key) {
            continue;
        }
        if holds_summary(&dir) {
            keys.insert(key);
        }
    }
    keys.retain(|k| !main.contains(k));
    keys.into_iter().collect()
}

/// The shortest prefix of `id` (at least 7 chars) no other id shares.
fn unique_prefix<'a>(id: &'a str, agent_ids: &[&str]) -> &'a str {
    let mut n = 7.min(id.len());
    while n < id.len()
        && agent_ids
            .iter()
            .any(|other| *other != id && other.starts_with(&id[..n]))
    {
        n += 1;
    }
    &id[..n]
}

/// The display ref of a worktree (I-10): the branch with `refs/heads/`,
/// `worktree-` and `gsd/` stripped (the directory name for a detached HEAD);
/// an `agent-<id>` name becomes the id's first 7 chars, extended until unique
/// among `agent_ids`. Stored untrusted: a hostile clone chooses the name.
pub fn short_ref(branch: Option<&Untrusted>, worktree: &Path, agent_ids: &[&str]) -> Untrusted {
    let name = match branch {
        Some(b) => {
            let raw = b.as_raw_for_logic_only();
            let raw = raw.strip_prefix("refs/heads/").unwrap_or(raw);
            let raw = raw.strip_prefix("worktree-").unwrap_or(raw);
            raw.strip_prefix("gsd/").unwrap_or(raw).to_string()
        }
        None => worktree
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_default(),
    };
    let name = match name.strip_prefix("agent-") {
        Some(id) if worktrees::valid_agent_id(id) => unique_prefix(id, agent_ids).to_string(),
        _ => name,
    };
    Untrusted::from_untrusted_source(name)
}

/// The state one holder worktree's agent row implies (I-4): a running agent,
/// or a lock whose owner is not `Ended`, is `InProgress`; anything else —
/// including a worktree with no agent row, locked or not — awaits merge.
fn item_state(row: Option<&AgentRow>) -> UnmergedState {
    match row {
        Some(r) if r.liveness.is_running() => UnmergedState::InProgress,
        Some(r) if r.locked && r.liveness != AgentLiveness::Ended => UnmergedState::InProgress,
        _ => UnmergedState::AwaitingMerge,
    }
}

/// Every unmerged key across a project's non-prunable linked worktrees, one
/// item per key, sorted by key.
///
/// Empty — without reading main — when no such worktree exists (I-2). Main is
/// `core.main_worktree`, else `project_root`. The item's worktree is the first
/// holder by path; its state is `InProgress` when ANY holder is (I-4). Counts
/// come from the holder's agent row, else from one `worktree_counts` call per
/// holder that is not an agent row (I-3).
pub fn detect(core: &worktrees::CoreScan, rows: &[AgentRow], project_root: &Path) -> Vec<UnmergedItem> {
    let mut live: Vec<&worktrees::CoreWorktree> =
        core.worktrees.iter().filter(|wt| !wt.prunable).collect();
    if live.is_empty() {
        return Vec::new();
    }
    live.sort_by(|a, b| a.path.cmp(&b.path));
    let main_root = core.main_worktree.as_deref().unwrap_or(project_root);
    let main = main_done_keys(&main_root.join(".planning"));
    let agent_ids: Vec<&str> = core
        .worktrees
        .iter()
        .filter_map(|wt| wt.agent_id.as_deref())
        .collect();

    let mut items: Vec<UnmergedItem> = Vec::new();
    for wt in live {
        let keys = worktree_done_keys(&wt.path.join(".planning"), &main);
        if keys.is_empty() {
            continue;
        }
        let row = rows.iter().find(|r| r.path == wt.path);
        let state = item_state(row);
        let (commits_ahead, dirty) = match row {
            Some(r) => (r.commits_ahead, r.dirty),
            None => worktrees::worktree_counts(wt, core.base_sha.as_deref()),
        };
        let short = short_ref(wt.branch.as_ref(), &wt.path, &agent_ids);
        for key in keys {
            if let Some(existing) = items.iter_mut().find(|i| i.key == key) {
                if state == UnmergedState::InProgress {
                    existing.state = UnmergedState::InProgress;
                }
                continue;
            }
            items.push(UnmergedItem {
                key,
                worktree: wt.path.clone(),
                branch: wt.branch.clone(),
                short_ref: short.clone(),
                commits_ahead,
                dirty,
                state,
            });
        }
    }
    items.sort_by(|a, b| a.key.cmp(&b.key));
    if !items.is_empty() {
        // Counts only: never a branch, path or directory name (D-C13).
        tracing::debug!(items = items.len(), "unmerged worktree work detected");
    }
    items
}

#[cfg(test)]
mod tests {
    use super::*;

    fn u(s: &str) -> Untrusted {
        Untrusted::from_untrusted_source(s.to_string())
    }

    fn raw(value: Untrusted) -> String {
        value.as_raw_for_logic_only().to_string()
    }

    #[test]
    fn short_refs_strip_prefixes_and_shorten_agent_ids() {
        let id = "a486395e992a58454";
        assert_eq!(
            raw(short_ref(
                Some(&u(&format!("worktree-agent-{id}"))),
                Path::new("/x"),
                &[id]
            )),
            "a486395"
        );
        assert_eq!(
            raw(short_ref(Some(&u("refs/heads/gsd/foo")), Path::new("/x"), &[])),
            "foo"
        );
        assert_eq!(
            raw(short_ref(Some(&u("feature/x")), Path::new("/x"), &[])),
            "feature/x"
        );
        assert_eq!(
            raw(short_ref(None, Path::new("/tmp/wts/hotfix-dir"), &[])),
            "hotfix-dir",
            "a detached HEAD uses its directory name"
        );
        let twin = "a486395f00000000";
        assert_eq!(
            raw(short_ref(
                Some(&u(&format!("worktree-agent-{id}"))),
                Path::new("/x"),
                &[id, twin]
            )),
            "a486395e",
            "two ids sharing 7 chars extend to 8"
        );
    }

    #[test]
    fn quick_ids_are_validated() {
        assert_eq!(quick_id("260929-abc-slug"), Some("260929-abc"));
        assert_eq!(quick_id("260929-a0c"), Some("260929-a0c"));
        assert_eq!(quick_id("2609-abc"), None);
        assert_eq!(quick_id("260929-ABC"), None);
        assert_eq!(quick_id("260929-abcd"), None);
        assert_eq!(quick_id("../../etc"), None);
    }

    #[test]
    fn fix_and_gapclosure_summaries_are_never_keys() {
        assert!(summary_plan("13-01-FIX-SUMMARY.md").is_none());
        assert!(summary_plan("13-01-GAPCLOSURE-SUMMARY.md").is_none());
        assert!(summary_plan("13-SUMMARY.md").is_none());
        assert!(summary_plan("14-REMEDIATION-SUMMARY.md").is_none());
        assert!(summary_plan("13-01-PLAN.md").is_none());
        assert_eq!(summary_plan("13-1-SUMMARY.md"), PlanRef::from_id("13-01"));
    }

    fn row(locked: bool, liveness: AgentLiveness) -> AgentRow {
        AgentRow {
            locked,
            liveness,
            ..AgentRow::default()
        }
    }

    #[test]
    fn the_state_rule_reads_the_holder_row() {
        use AgentLiveness::*;
        use UnmergedState::*;
        assert_eq!(item_state(Some(&row(false, Live))), InProgress);
        assert_eq!(item_state(Some(&row(false, Idle))), InProgress);
        assert_eq!(item_state(Some(&row(true, Stalled))), InProgress);
        assert_eq!(item_state(Some(&row(true, Unknown))), InProgress);
        assert_eq!(item_state(Some(&row(true, Ended))), AwaitingMerge);
        assert_eq!(item_state(Some(&row(false, Finished))), AwaitingMerge);
        assert_eq!(item_state(Some(&row(false, Stalled))), AwaitingMerge);
        assert_eq!(item_state(None), AwaitingMerge);
    }

    /// Name-only fixtures in a temp dir (std::fs in test code; no spawn).
    #[test]
    fn main_and_worktree_key_sets_read_names_only() {
        let tmp = tempfile::tempdir().expect("temp dir");
        let touch = |rel: &str| {
            let p = tmp.path().join(rel);
            std::fs::create_dir_all(p.parent().unwrap()).unwrap();
            std::fs::write(p, "").unwrap();
        };
        touch("main/phases/13-x/13-01-SUMMARY.md");
        touch("main/phases/13-x/13-02-PLAN.md");
        touch("main/milestones/v1.0-phases/04-y/04-03-SUMMARY.md");
        touch("main/milestones/v1.0-notes/05-01-SUMMARY.md");
        touch("main/quick/260929-abc-a/260929-abc-SUMMARY.md");
        touch("main/quick/260929-def-b/260929-def-PLAN.md");
        touch("wt/phases/13-x/13-01-SUMMARY.md");
        touch("wt/phases/13-x/13-02-SUMMARY.md");
        touch("wt/phases/13-x/13-REVIEW.md");
        touch("wt/phases/04-y/04-03-SUMMARY.md");
        touch("wt/quick/260929-abc-a/260929-abc-SUMMARY.md");
        touch("wt/quick/260929-def-b/260929-def-SUMMARY.md");
        touch("wt/quick-batches/260929-ghi/260929-ghi-SUMMARY.md");

        let main = main_done_keys(&tmp.path().join("main"));
        assert!(main.contains(&UnmergedKey::Plan(PlanRef::from_id("13-01").unwrap())));
        assert!(main.contains(&UnmergedKey::Plan(PlanRef::from_id("04-03").unwrap())));
        assert!(!main.contains(&UnmergedKey::Plan(PlanRef::from_id("05-01").unwrap())));
        assert!(main.contains(&UnmergedKey::Quick("260929-abc".into())));
        assert!(!main.contains(&UnmergedKey::Quick("260929-def".into())));

        assert_eq!(
            worktree_done_keys(&tmp.path().join("wt"), &main),
            vec![
                UnmergedKey::Plan(PlanRef::from_id("13-02").unwrap()),
                UnmergedKey::Quick("260929-def".into()),
            ]
        );
    }
}
