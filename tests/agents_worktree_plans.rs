// ============================================================================
// Quick 261014-0jv: phase plans that exist only in a linked worktree, against
// real repositories with real linked worktrees.
//
// A phase's `*-PLAN.md` files can live on a branch checked out in a linked
// worktree (`.claude/worktrees/lanes-15` on `gsd/phase-15`) while main's
// `.planning/phases/15-*/` is empty or absent. The scan resolves those plans
// (read-only, no extra git) so the Phases tab can browse them. Main always
// wins: a main dir holding any PLAN, or a main archive of the key, hides the
// worktree's copy.
//
// An integration test because building these worlds spawns git, and
// `src/agents/` is not on the spawn allowlist (tests/spawn_seam_guard.rs).
// The fixtures are MODELLED on mailbot's names (lanes-13 / lanes-15,
// gsd/phase-NN) without reading mailbot; the one `#[ignore]`d test at the end
// does read the real project, strictly read-only.
// ============================================================================

mod common;

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use common::git;
use gsd_meta_manager::agents::waves;
use gsd_meta_manager::agents::{scan_project_with, ProjectAgents};
use gsd_meta_manager::state_reader::ProjectState;
use tempfile::TempDir;

/// A repository with one commit (`tracked.txt`) and no linked worktree; `None`
/// when the sandbox forbids `git init` (every later step asserts).
fn plain_repo() -> Option<(TempDir, PathBuf)> {
    let tmp = TempDir::new().ok()?;
    let root = std::fs::canonicalize(tmp.path()).ok()?.join("project");
    std::fs::create_dir_all(&root).ok()?;
    if !git(&root, &["init", "--quiet"]) {
        return None;
    }
    assert!(git(&root, &["config", "user.email", "test@example.com"]));
    assert!(git(&root, &["config", "user.name", "Test User"]));
    assert!(git(&root, &["config", "commit.gpgsign", "false"]));
    std::fs::write(root.join("tracked.txt"), "one\n").expect("fixture write");
    assert!(git(&root, &["add", "tracked.txt"]), "git add");
    assert!(
        git(&root, &["commit", "-m", "initial", "--quiet"]),
        "git commit"
    );
    Some((tmp, root))
}

/// `git worktree add -b <branch> <path>` in `root`, asserting it worked.
fn add_worktree(root: &Path, branch: &str, path: &str) {
    assert!(
        git(root, &["worktree", "add", "--quiet", "-b", branch, path]),
        "git worktree add -b {branch} {path}"
    );
}

/// Write the nested `rel` under `dir` (creating parents). Untracked is fine:
/// the scan reads the filesystem, never git objects.
fn put(dir: &Path, rel: &str, body: &str) {
    let path = dir.join(rel);
    std::fs::create_dir_all(path.parent().expect("a parent")).expect("fixture mkdir");
    std::fs::write(&path, body).expect("fixture write");
}

fn scan(root: &Path) -> ProjectAgents {
    scan_project_with(root, &[], SystemTime::now())
}

const DIGEST: &str = ".planning/phases/15-digest";

/// Fill `holder`'s phase 15 dir: two waved plans and one summary.
fn fill_holder(holder: &Path) {
    put(
        holder,
        &format!("{DIGEST}/15-01-alpha-PLAN.md"),
        "---\nwave: 1\n---\n<objective>Alpha.</objective>\n",
    );
    put(
        holder,
        &format!("{DIGEST}/15-02-beta-PLAN.md"),
        "---\nwave: 2\n---\n<objective>Beta.</objective>\n",
    );
    put(holder, &format!("{DIGEST}/15-01-SUMMARY.md"), "summary\n");
}

/// A repo whose main has an EMPTY phase 15 dir and a `lanes-15` holder.
fn empty_main_with_holder() -> Option<(TempDir, PathBuf, PathBuf)> {
    let (tmp, root) = plain_repo()?;
    std::fs::create_dir_all(root.join(DIGEST)).ok()?;
    add_worktree(&root, "gsd/phase-15", ".claude/worktrees/lanes-15");
    let holder = root.join(".claude/worktrees/lanes-15");
    fill_holder(&holder);
    Some((tmp, root, holder))
}

#[test]
fn an_empty_main_phase_resolves_to_the_worktree_plans() {
    let Some((_tmp, root, holder)) = empty_main_with_holder() else {
        return;
    };
    let scan = scan(&root);
    assert_eq!(scan.worktree_plans.len(), 1, "{:?}", scan.worktree_plans);
    let wp = &scan.worktree_plans[0];
    assert_eq!(wp.key, "15");
    assert_eq!(wp.worktree_name.as_raw_for_logic_only(), "lanes-15");
    assert_eq!(
        wp.branch.as_ref().map(|b| b.as_raw_for_logic_only()),
        Some("gsd/phase-15")
    );
    assert_eq!(wp.short_ref.as_raw_for_logic_only(), "phase-15");
    assert_eq!(wp.others, 0);
    assert_eq!(wp.phase_dir, holder.join(DIGEST));
    assert_eq!(wp.inference.plan_count, 2);
    assert_eq!(wp.inference.summary_count, 1);
    let numbered: Vec<Option<u32>> = wp.inference.plan_waves.iter().map(|w| w.wave).collect();
    assert_eq!(numbered, vec![Some(1), Some(2)]);

    let view = waves::derive(&scan, &ProjectState::default());
    assert!(view.worktree_plans_for("15").is_some());
    assert!(view.worktree_plans_for("015").is_some(), "pad-insensitive");
    assert!(view.worktree_plans_for("16").is_none());
}

#[test]
fn a_phase_main_has_no_directory_for_also_resolves() {
    let Some((_tmp, root)) = plain_repo() else {
        return;
    };
    add_worktree(&root, "gsd/phase-15", ".claude/worktrees/lanes-15");
    fill_holder(&root.join(".claude/worktrees/lanes-15"));
    assert!(!root.join(DIGEST).exists());

    let scan = scan(&root);
    assert_eq!(scan.worktree_plans.len(), 1, "{:?}", scan.worktree_plans);
    assert_eq!(scan.worktree_plans[0].key, "15");
}

#[test]
fn main_plans_take_precedence_over_a_worktree_copy() {
    let Some((_tmp, root, _holder)) = empty_main_with_holder() else {
        return;
    };
    put(
        &root,
        &format!("{DIGEST}/15-01-alpha-PLAN.md"),
        "main copy\n",
    );
    assert!(
        scan(&root).worktree_plans.is_empty(),
        "main holding any plan hides the worktree's"
    );
}

#[test]
fn a_worktree_phase_dir_without_plans_is_not_listed() {
    let Some((_tmp, root)) = plain_repo() else {
        return;
    };
    add_worktree(&root, "gsd/phase-15", ".claude/worktrees/lanes-15");
    put(
        &root.join(".claude/worktrees/lanes-15"),
        &format!("{DIGEST}/15-CONTEXT.md"),
        "context\n",
    );
    assert!(scan(&root).worktree_plans.is_empty());
}

#[test]
fn a_project_without_linked_worktrees_lists_nothing() {
    let Some((_tmp, root)) = plain_repo() else {
        return;
    };
    fill_holder(&root);
    assert!(scan(&root).worktree_plans.is_empty());
}
