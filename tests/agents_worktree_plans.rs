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

// ── several holders (ID-4): newest PLAN mtime, then smallest path ────────────

/// Set `path`'s mtime to `secs` after the epoch (test code only: `src/agents`
/// stays write-free).
fn set_mtime(path: &Path, secs: u64) {
    let file = std::fs::File::options()
        .write(true)
        .open(path)
        .expect("open for set_modified");
    file.set_modified(SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(secs))
        .expect("set_modified");
}

const PLAN_15_01: &str = ".planning/phases/15-digest/15-01-alpha-PLAN.md";

/// Two holders `lanes-15a` / `lanes-15b`, each with a 15-01 PLAN at the given
/// mtimes (seconds after the epoch).
fn two_holders(a_secs: u64, b_secs: u64) -> Option<(TempDir, PathBuf)> {
    let (tmp, root) = plain_repo()?;
    for (name, secs) in [("lanes-15a", a_secs), ("lanes-15b", b_secs)] {
        add_worktree(
            &root,
            &format!("gsd/phase-{}", name.trim_start_matches("lanes-")),
            &format!(".claude/worktrees/{name}"),
        );
        let holder = root.join(".claude/worktrees").join(name);
        put(
            &holder,
            PLAN_15_01,
            "---\nwave: 1\n---\n<objective>x</objective>\n",
        );
        set_mtime(&holder.join(PLAN_15_01), secs);
    }
    Some((tmp, root))
}

fn winner_name(scan: &ProjectAgents) -> String {
    assert_eq!(scan.worktree_plans.len(), 1, "{:?}", scan.worktree_plans);
    scan.worktree_plans[0]
        .worktree_name
        .as_raw_for_logic_only()
        .to_string()
}

#[test]
fn the_newest_plan_wins_among_several_worktrees() {
    let Some((_tmp, root)) = two_holders(1_000, 2_000) else {
        return;
    };
    let scan_b = scan(&root);
    assert_eq!(winner_name(&scan_b), "lanes-15b");
    assert_eq!(scan_b.worktree_plans[0].others, 1);
    assert_eq!(
        scan_b.worktree_plans[0].short_ref.as_raw_for_logic_only(),
        "phase-15b"
    );

    // Flip the mtimes: a becomes the winner.
    set_mtime(
        &root.join(".claude/worktrees/lanes-15a").join(PLAN_15_01),
        3_000,
    );
    let scan_a = self::scan(&root);
    assert_eq!(winner_name(&scan_a), "lanes-15a");
    assert_eq!(scan_a.worktree_plans[0].others, 1);
}

#[test]
fn equal_plan_mtimes_fall_back_to_the_worktree_path() {
    let Some((_tmp, root)) = two_holders(5_000, 5_000) else {
        return;
    };
    let first = scan(&root);
    assert_eq!(winner_name(&first), "lanes-15a", "the smaller path");
    let second = scan(&root);
    assert_eq!(
        winner_name(&second),
        winner_name(&first),
        "identical across scans"
    );
}

#[test]
fn a_superseded_only_winner_falls_through_to_the_next_worktree() {
    let Some((_tmp, root)) = two_holders(1_000, 2_000) else {
        return;
    };
    // b is newest, but its only PLAN is superseded: a must win.
    put(
        &root.join(".claude/worktrees/lanes-15b"),
        PLAN_15_01,
        "---\nstatus: superseded\nwave: 1\n---\n<objective>x</objective>\n",
    );
    set_mtime(
        &root.join(".claude/worktrees/lanes-15b").join(PLAN_15_01),
        2_000,
    );
    let scan = scan(&root);
    assert_eq!(winner_name(&scan), "lanes-15a");
    assert_eq!(scan.worktree_plans[0].inference.plan_count, 1);
}

#[test]
fn a_phase_main_archived_under_milestones_is_not_listed() {
    let Some((_tmp, root, _holder)) = empty_main_with_holder() else {
        return;
    };
    std::fs::create_dir_all(root.join(".planning/milestones/v1-phases/15-digest"))
        .expect("archive dir");
    assert!(scan(&root).worktree_plans.is_empty());
}

// ── the real mailbot project (read-only; run on demand) ──────────────────────

/// Phases 13 and 15 of mailbot keep their plans in `lanes-13` / `lanes-15`.
/// Reads only: `scan_project_with` runs `git worktree list --porcelain` with
/// `--no-optional-locks` and opens files, nothing else. Prints SKIP and returns
/// when the project, a holder or the unmerged state is not as recorded.
///
/// Run once, on demand:
/// `cargo test --test agents_worktree_plans -- --ignored real_mailbot`
#[test]
#[ignore = "reads ~/projects/python/mailbot; run on demand"]
fn real_mailbot_phases_13_and_15_resolve_to_worktree_plans() {
    let Some(home) = std::env::var_os("HOME") else {
        println!("SKIP: HOME unset");
        return;
    };
    let path = PathBuf::from(home).join("projects/python/mailbot");
    let holders = path.join(".claude/worktrees");
    if !path.is_dir() || !holders.join("lanes-13").is_dir() || !holders.join("lanes-15").is_dir() {
        println!("SKIP: mailbot or its lanes-13 / lanes-15 worktrees are absent");
        return;
    }
    let main_has_plan = |n: &str| {
        std::fs::read_dir(path.join(".planning/phases"))
            .into_iter()
            .flatten()
            .flatten()
            .filter(|e| {
                e.file_name()
                    .to_string_lossy()
                    .starts_with(&format!("{n}-"))
            })
            .any(|e| {
                std::fs::read_dir(e.path())
                    .into_iter()
                    .flatten()
                    .flatten()
                    .any(|f| f.file_name().to_string_lossy().ends_with("PLAN.md"))
            })
    };
    if main_has_plan("13") || main_has_plan("15") {
        println!("SKIP: main now holds phase 13/15 plans (merged since)");
        return;
    }

    let scan = scan(&path);
    let entry = |key: &str| {
        scan.worktree_plans
            .iter()
            .find(|wp| wp.key == key)
            .unwrap_or_else(|| {
                panic!(
                    "no worktree plans for phase {key}: {:?}",
                    scan.worktree_plans
                )
            })
    };
    for (key, name, branch, plans) in [
        ("13", "lanes-13", "gsd/phase-13", 5),
        ("15", "lanes-15", "gsd/phase-15", 7),
    ] {
        let wp = entry(key);
        assert_eq!(wp.worktree_name.as_raw_for_logic_only(), name);
        assert_eq!(
            wp.branch.as_ref().map(|b| b.as_raw_for_logic_only()),
            Some(branch)
        );
        assert_eq!(wp.inference.plan_count, plans, "phase {key} plan count");
    }
    println!("PASS: phases 13 and 15 resolve to their worktree plans");
}
