// ============================================================================
// Quick 260929-szq: completed-but-unmerged worktree work, against real
// repositories with real linked worktrees.
//
// A plan (or quick task) is unmerged when its SUMMARY exists in a linked
// worktree and in none of main's `.planning/phases/*/`,
// `.planning/milestones/*-phases/*/` or `.planning/quick/<id>-*/` dirs (I-1,
// I-5, I-6). Commit ancestry is never the signal: a squash-merged branch stays
// "ahead" forever, and fixture (b) proves the path-level rule clears it.
//
// An integration test because building these worlds spawns git, and
// `src/agents/` is not on the spawn allowlist (tests/spawn_seam_guard.rs).
// Fixture (h) is MODELLED on the mailbot snapshot's file names only; it never
// reads the snapshot or mailbot itself.
// ============================================================================

mod common;

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use common::git;
use gsd_meta_manager::agents::unmerged::{UnmergedKey, UnmergedState};
use gsd_meta_manager::agents::waves::{self, PlanRef};
use gsd_meta_manager::agents::{scan_project_with, scan_projects_guarded, ProjectAgents};
use gsd_meta_manager::session_detector::NoProcessProbe;
use gsd_meta_manager::state_reader::ProjectState;
use tempfile::TempDir;

const AGENT_ID: &str = "a0123456789abcdef";
const MAILBOT_ID: &str = "a486395e992a58454";

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

/// Write the nested `rel` under `dir` (creating parents) and, when `commit`,
/// add and commit it.
fn put(dir: &Path, rel: &str, body: &str, commit: bool) {
    let path = dir.join(rel);
    std::fs::create_dir_all(path.parent().expect("a parent")).expect("fixture mkdir");
    std::fs::write(&path, body).expect("fixture write");
    if commit {
        assert!(git(dir, &["add", rel]), "git add {rel}");
        assert!(
            git(dir, &["commit", "-m", &format!("add {rel}"), "--quiet"]),
            "git commit {rel}"
        );
    }
}

const PLAN_13_01: &str = ".planning/phases/13-demo/13-01-PLAN.md";
const SUMMARY_13_01: &str = ".planning/phases/13-demo/13-01-SUMMARY.md";

/// Main commits phase 13's PLAN, then the agent worktree is added.
fn phase_13_with_agent() -> Option<(TempDir, PathBuf, PathBuf)> {
    let (tmp, root) = plain_repo()?;
    put(&root, PLAN_13_01, "plan\n", true);
    let rel = format!(".claude/worktrees/agent-{AGENT_ID}");
    add_worktree(&root, &format!("worktree-agent-{AGENT_ID}"), &rel);
    let wt = root.join(rel);
    Some((tmp, root, wt))
}

fn plan_key(id: &str) -> UnmergedKey {
    UnmergedKey::Plan(PlanRef::from_id(id).expect("valid id"))
}

fn scan(root: &Path) -> ProjectAgents {
    scan_project_with(root, &[], SystemTime::now())
}

#[test]
fn a_summary_committed_only_on_a_worktree_is_flagged() {
    let Some((_tmp, root, wt)) = phase_13_with_agent() else {
        return;
    };
    put(&wt, SUMMARY_13_01, "summary\n", true);

    let scan = scan(&root);
    assert_eq!(scan.unmerged.len(), 1, "{:?}", scan.unmerged);
    let item = &scan.unmerged[0];
    assert_eq!(item.key, plan_key("13-01"));
    assert_eq!(item.worktree, wt);
    assert_eq!(item.short_ref.as_raw_for_logic_only(), "a012345");
    assert_eq!(item.commits_ahead, Some(1));
    assert_eq!(item.state, UnmergedState::AwaitingMerge);
}

#[test]
fn a_squash_merged_summary_is_cleared_although_the_branch_stays_ahead() {
    let Some((_tmp, root, wt)) = phase_13_with_agent() else {
        return;
    };
    put(&wt, SUMMARY_13_01, "summary\n", true);
    // The squash: main gains the same file under its own commit.
    put(&root, SUMMARY_13_01, "summary\n", true);

    let scan = scan(&root);
    assert!(scan.unmerged.is_empty(), "{:?}", scan.unmerged);
    assert_eq!(
        scan.rows[0].commits_ahead,
        Some(1),
        "control: the worktree is still ahead by ancestry"
    );
}

#[test]
fn an_untracked_worktree_summary_is_flagged() {
    let Some((_tmp, root, wt)) = phase_13_with_agent() else {
        return;
    };
    put(&wt, SUMMARY_13_01, "summary\n", false);

    let scan = scan(&root);
    assert_eq!(scan.unmerged.len(), 1, "{:?}", scan.unmerged);
    assert_eq!(scan.unmerged[0].key, plan_key("13-01"));
    assert!(scan.unmerged[0].dirty >= Some(1), "{:?}", scan.unmerged[0]);
}

#[test]
fn a_stale_worktree_holding_only_a_plan_main_deleted_is_not_flagged() {
    let Some((_tmp, root)) = plain_repo() else {
        return;
    };
    let plan = ".planning/phases/15-x/15-10-PLAN.md";
    put(&root, plan, "plan\n", true);
    let rel = format!(".claude/worktrees/agent-{AGENT_ID}");
    add_worktree(&root, &format!("worktree-agent-{AGENT_ID}"), &rel);
    assert!(git(&root, &["rm", "--quiet", plan]));
    assert!(git(&root, &["commit", "-m", "replan", "--quiet"]));

    let scan = scan(&root);
    assert_eq!(scan.rows.len(), 1, "control: the stale worktree is scanned");
    assert!(scan.unmerged.is_empty(), "{:?}", scan.unmerged);
}

#[test]
fn a_quick_summary_is_flagged_until_main_holds_it() {
    let Some((_tmp, root, wt)) = phase_13_with_agent() else {
        return;
    };
    put(
        &wt,
        ".planning/quick/260929-abc-slug/260929-abc-SUMMARY.md",
        "summary\n",
        true,
    );
    let scan1 = scan(&root);
    assert_eq!(scan1.unmerged.len(), 1, "{:?}", scan1.unmerged);
    assert_eq!(
        scan1.unmerged[0].key,
        UnmergedKey::Quick("260929-abc".to_string())
    );

    put(
        &root,
        ".planning/quick/260929-abc-other-slug/260929-abc-SUMMARY.md",
        "summary\n",
        true,
    );
    assert!(scan(&root).unmerged.is_empty());
}

#[test]
fn a_summary_main_archived_under_milestones_is_not_flagged() {
    let Some((_tmp, root, wt)) = phase_13_with_agent() else {
        return;
    };
    put(&wt, SUMMARY_13_01, "summary\n", true);
    put(
        &root,
        ".planning/milestones/v1.0-phases/13-demo/13-01-SUMMARY.md",
        "summary\n",
        true,
    );
    assert!(scan(&root).unmerged.is_empty());
}

#[test]
fn a_human_feature_worktree_is_flagged_and_awaits_merge_even_when_locked() {
    let Some((tmp, root)) = plain_repo() else {
        return;
    };
    put(&root, PLAN_13_01, "plan\n", true);
    let sibling = std::fs::canonicalize(tmp.path())
        .expect("canonical tmp")
        .join("feature-x");
    add_worktree(&root, "feature/x", sibling.to_str().expect("utf-8 path"));
    put(&sibling, SUMMARY_13_01, "summary\n", true);

    let scan1 = scan(&root);
    assert!(scan1.rows.is_empty(), "a human worktree has no agent row");
    assert_eq!(scan1.unmerged.len(), 1, "{:?}", scan1.unmerged);
    assert_eq!(scan1.unmerged[0].short_ref.as_raw_for_logic_only(), "feature/x");
    assert_eq!(scan1.unmerged[0].state, UnmergedState::AwaitingMerge);
    assert_eq!(scan1.unmerged[0].commits_ahead, Some(1));

    assert!(git(&root, &["worktree", "lock", sibling.to_str().unwrap()]));
    let scan2 = scan(&root);
    assert_eq!(scan2.unmerged.len(), 1);
    assert_eq!(scan2.unmerged[0].state, UnmergedState::AwaitingMerge);
}

/// Mailbot's phase 05 as the snapshot shows it: five SUMMARYs plus the review
/// and verification docs only in a LOCKED agent worktree with a dirty
/// REQUIREMENTS.md.
#[test]
fn mailbot_modelled_phase_05_is_in_progress_while_locked_then_awaits_merge() {
    let Some((_tmp, root)) = plain_repo() else {
        return;
    };
    let dir = ".planning/phases/05-attachment-export";
    put(&root, ".planning/REQUIREMENTS.md", "reqs\n", true);
    for n in 1..=5 {
        put(&root, &format!("{dir}/05-0{n}-PLAN.md"), "plan\n", true);
    }
    let rel = format!(".claude/worktrees/agent-{MAILBOT_ID}");
    add_worktree(&root, &format!("worktree-agent-{MAILBOT_ID}"), &rel);
    let wt = root.join(&rel);
    for n in 1..=5 {
        put(&wt, &format!("{dir}/05-0{n}-SUMMARY.md"), "summary\n", true);
    }
    for doc in ["REVIEW", "REVIEW-FIX", "VALIDATION", "VERIFICATION"] {
        put(&wt, &format!("{dir}/05-{doc}.md"), "doc\n", true);
    }
    put(&wt, ".planning/REQUIREMENTS.md", "reqs, edited\n", false);
    assert!(git(
        &root,
        &[
            "worktree",
            "lock",
            "--reason",
            &format!("claude agent agent-{MAILBOT_ID} (pid 1 start 1)"),
            wt.to_str().unwrap(),
        ]
    ));

    let expected: Vec<UnmergedKey> = (1..=5).map(|n| plan_key(&format!("05-0{n}"))).collect();
    let locked = scan(&root);
    assert_eq!(
        locked.unmerged.iter().map(|i| i.key.clone()).collect::<Vec<_>>(),
        expected
    );
    for item in &locked.unmerged {
        assert_eq!(item.state, UnmergedState::InProgress, "{item:?}");
        assert_eq!(item.short_ref.as_raw_for_logic_only(), "a486395");
        assert_eq!(item.dirty, Some(1), "{item:?}");
        assert_eq!(item.worktree, wt);
    }
    assert_eq!(
        waves::derive(&locked, &ProjectState::default()).summary_forms(),
        vec!["\u{25D0}5 unmerged".to_string(), "\u{25D0}5".to_string()]
    );

    assert!(git(&root, &["worktree", "unlock", wt.to_str().unwrap()]));
    let unlocked = scan(&root);
    assert_eq!(unlocked.unmerged.len(), 5);
    assert!(unlocked
        .unmerged
        .iter()
        .all(|i| i.state == UnmergedState::AwaitingMerge));
    assert_eq!(
        waves::derive(&unlocked, &ProjectState::default()).summary_forms(),
        vec!["\u{25D0}5 unmerged".to_string(), "\u{25D0}5".to_string()]
    );
}

#[test]
fn the_guarded_scan_carries_unmerged_work_to_the_summary_forms() {
    let Some((_tmp, root, wt)) = phase_13_with_agent() else {
        return;
    };
    put(&wt, SUMMARY_13_01, "summary\n", true);

    let scans = scan_projects_guarded(
        &[("demo".to_string(), root.clone())],
        &NoProcessProbe,
        SystemTime::now(),
    );
    assert_eq!(scans.len(), 1);
    let view = waves::derive(&scans[0].1, &ProjectState::default());
    assert_eq!(
        view.summary_forms(),
        vec!["\u{25D0}1 unmerged".to_string(), "\u{25D0}1".to_string()]
    );
}

#[test]
fn a_project_without_worktrees_has_no_unmerged_work() {
    let Some((_tmp, root)) = plain_repo() else {
        return;
    };
    put(&root, PLAN_13_01, "plan\n", true);
    assert!(scan(&root).unmerged.is_empty());
}
