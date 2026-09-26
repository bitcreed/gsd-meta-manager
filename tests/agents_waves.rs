// ============================================================================
// AGENT-02: the wave model, against real repositories with real linked
// worktrees — plan attribution in the scan, the worktree SUMMARY check, and
// `waves::derive` over the phase directory the main worktree actually holds.
//
// This is an integration test rather than an in-source one for the reason
// `tests/agents_scan.rs` gives: every proof here needs `git worktree add` and
// real commits, and `src/agents/` is deliberately NOT on the spawn allowlist in
// `tests/spawn_seam_guard.rs`, so even its `#[cfg(test)]` code may not spawn.
// The pure derivation is pinned in-source, in `src/agents/waves.rs`.
//
// It reuses `common::git` but never `common::fixture()`, which installs
// envelope hooks this suite has no business with.
// ============================================================================

mod common;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use common::git;
use gsd_meta_manager::agents::adapters::{AdapterReport, AgentAdapter, CoreSnapshot, Enrichment};
use gsd_meta_manager::agents::waves::derive;
use gsd_meta_manager::agents::{scan_project_with, AgentLiveness, ProjectAgents};
use gsd_meta_manager::state_reader::disk_status::infer_phase_status;
use gsd_meta_manager::state_reader::ProjectState;
use gsd_meta_manager::text::Untrusted;
use tempfile::TempDir;

/// The Codex-style branch of the tracer's executor: plan 13-02 (D-A06).
const P13_02: &str = "agent-p13-02-1790386422";

/// A repository whose one commit holds `.planning/phases/13-demo/` with
/// `13-01-PLAN.md` (wave 1), `13-02-PLAN.md` and `13-03-PLAN.md` (wave 2), and
/// `13-01-SUMMARY.md`.
///
/// `None` when the sandbox forbids `git init`; every later step asserts, so a
/// degraded fixture can never make a test pass vacuously.
fn phase_repo() -> Option<(TempDir, PathBuf)> {
    let tmp = TempDir::new().ok()?;
    // Canonical, because git reports canonical worktree paths.
    let root = std::fs::canonicalize(tmp.path()).ok()?.join("project");
    std::fs::create_dir_all(&root).ok()?;
    if !git(&root, &["init", "--quiet"]) {
        return None;
    }
    assert!(git(&root, &["config", "user.email", "test@example.com"]));
    assert!(git(&root, &["config", "user.name", "Test User"]));
    assert!(git(&root, &["config", "commit.gpgsign", "false"]));
    let phase = root.join(".planning/phases/13-demo");
    std::fs::create_dir_all(&phase).expect("fixture dir");
    let plan = |wave: u32| format!("---\nphase: 13\nwave: {wave}\n---\n\nbody\n");
    std::fs::write(phase.join("13-01-PLAN.md"), plan(1)).expect("fixture write");
    std::fs::write(phase.join("13-02-PLAN.md"), plan(2)).expect("fixture write");
    std::fs::write(phase.join("13-03-PLAN.md"), plan(2)).expect("fixture write");
    std::fs::write(phase.join("13-01-SUMMARY.md"), "done\n").expect("fixture write");
    assert!(git(&root, &["add", "."]), "git add");
    assert!(
        git(&root, &["commit", "-m", "phase 13 plans", "--quiet"]),
        "git commit"
    );
    Some((tmp, root))
}

/// `git worktree add -b worktree-<name> .claude/worktrees/<name>`, asserting.
fn add_agent_worktree(root: &Path, name: &str) -> PathBuf {
    let rel = format!(".claude/worktrees/{name}");
    assert!(
        git(
            root,
            &[
                "worktree",
                "add",
                "--quiet",
                "-b",
                &format!("worktree-{name}"),
                &rel
            ]
        ),
        "git worktree add {rel}"
    );
    root.join(rel)
}

/// Write `rel` under `dir` and commit it with `subject`.
fn commit_in(dir: &Path, rel: &str, body: &str, subject: &str) {
    let path = dir.join(rel);
    std::fs::create_dir_all(path.parent().expect("rel has a parent")).expect("fixture dir");
    std::fs::write(&path, body).expect("fixture write");
    assert!(git(dir, &["add", rel]), "git add {rel}");
    assert!(
        git(dir, &["commit", "-m", subject, "--quiet"]),
        "git commit {subject}"
    );
}

/// A test-only adapter reporting the same scripted facts for every worktree
/// the agent predicate matches.
struct Scripted {
    age_secs: u64,
    lock_released: Option<bool>,
    description: Option<&'static str>,
}

impl Scripted {
    fn live() -> Self {
        Scripted {
            age_secs: 0,
            lock_released: Some(false),
            description: None,
        }
    }
}

impl AgentAdapter for Scripted {
    fn name(&self) -> &'static str {
        "scripted"
    }

    fn enrich(&self, snap: &CoreSnapshot<'_>) -> AdapterReport {
        let per_worktree = snap
            .worktrees
            .iter()
            .enumerate()
            .filter(|(_, wt)| wt.agent_pattern)
            .map(|(index, _)| {
                (
                    index,
                    Enrichment {
                        agent_type: Some(Untrusted::from_untrusted_source("gsd-executor".into())),
                        description: self
                            .description
                            .map(|d| Untrusted::from_untrusted_source(d.into())),
                        last_activity: Some(snap.now - Duration::from_secs(self.age_secs)),
                        lock_released: self.lock_released,
                        ..Enrichment::default()
                    },
                )
            })
            .collect();
        AdapterReport {
            per_worktree,
            worktreeless: Vec::new(),
        }
    }
}

fn scan(root: &Path, adapter: Scripted) -> ProjectAgents {
    let adapters: Vec<Box<dyn AgentAdapter>> = vec![Box::new(adapter)];
    scan_project_with(root, &adapters, SystemTime::now())
}

/// The parsed state the refresh would hold for this project: phase 13's disk
/// inference, read from the main worktree.
fn phase_13_state(root: &Path) -> ProjectState {
    ProjectState {
        phase_disk_statuses: HashMap::from([(
            "13".to_string(),
            infer_phase_status(&root.join(".planning"), "13"),
        )]),
        ..ProjectState::default()
    }
}

// ---------------------------------------------------------------------------
// The tracer
// ---------------------------------------------------------------------------

#[test]
fn a_branch_attributed_executor_drives_the_wave_summary_end_to_end() {
    let Some((_tmp, root)) = phase_repo() else {
        return;
    };
    add_agent_worktree(&root, P13_02);

    let agents = scan(&root, Scripted::live());
    assert_eq!(agents.rows.len(), 1, "{:?}", agents.rows);
    let row = &agents.rows[0];
    assert_eq!(row.liveness, AgentLiveness::Live);
    assert_eq!(
        row.plan.as_ref().map(|p| p.label()),
        Some("13-02".to_string()),
        "attributed from the branch's p13-02 component"
    );

    let view = derive(&agents, &phase_13_state(&root));
    assert_eq!(view.current_wave, Some(2));
    assert_eq!(view.max_wave, Some(2));
    assert_eq!(
        view.summary_forms().first().map(String::as_str),
        Some("P13 \u{b7} w2/2 \u{b7} 1 run \u{b7} 1/3 done")
    );
}

// ---------------------------------------------------------------------------
// Attribution tiers against real worktrees
// ---------------------------------------------------------------------------

#[test]
fn commit_scopes_attribute_an_executor_whose_description_and_branch_do_not() {
    let Some((_tmp, root)) = phase_repo() else {
        return;
    };
    // A Claude-style id carries no plan, and the adapter reports no description.
    let wt = add_agent_worktree(&root, "agent-a0123456789abcdef");
    commit_in(&wt, "src/thing.txt", "thing\n", "feat(13-03): add thing");

    let agents = scan(&root, Scripted::live());
    assert_eq!(agents.rows.len(), 1, "{:?}", agents.rows);
    assert_eq!(agents.rows[0].branch_plan, None);
    assert_eq!(
        agents.rows[0].plan.as_ref().map(|p| p.label()),
        Some("13-03".to_string()),
        "the worktree's own commit scope names the plan"
    );
}

#[test]
fn a_description_outranks_the_branch() {
    let Some((_tmp, root)) = phase_repo() else {
        return;
    };
    add_agent_worktree(&root, P13_02);
    let agents = scan(
        &root,
        Scripted {
            description: Some("Execute plan 13-03 of phase 13"),
            ..Scripted::live()
        },
    );
    assert_eq!(agents.rows.len(), 1, "{:?}", agents.rows);
    assert_eq!(
        agents.rows[0].plan.as_ref().map(|p| p.label()),
        Some("13-03".to_string()),
        "tier 1 (description) wins over tier 3 (branch p13-02)"
    );
}

// ---------------------------------------------------------------------------
// Finished before the merge: the worktree SUMMARY check (Pitfalls 3 and 4)
// ---------------------------------------------------------------------------

/// A lock-released agent whose transcript is 30 s old: `Live` by age alone,
/// so only the worktree SUMMARY can make it `Finished`.
fn released_30s() -> Scripted {
    Scripted {
        age_secs: 30,
        lock_released: Some(true),
        description: None,
    }
}

/// A finished plan whose SUMMARY is only in its worktree reads `Finished`.
/// Alone it does not switch the ladder on (CR-01: `Finished` is done work,
/// not running work); once something on the phase runs, it counts toward done.
#[test]
fn a_summary_committed_in_the_worktree_reads_finished_before_the_merge() {
    let Some((_tmp, root)) = phase_repo() else {
        return;
    };
    let wt = add_agent_worktree(&root, P13_02);
    commit_in(
        &wt,
        ".planning/phases/13-demo/13-02-SUMMARY.md",
        "done\n",
        "docs(13-02): complete plan",
    );
    assert!(
        !root
            .join(".planning/phases/13-demo/13-02-SUMMARY.md")
            .exists(),
        "main has not merged it"
    );

    let agents = scan(&root, released_30s());
    assert_eq!(agents.rows.len(), 1, "{:?}", agents.rows);
    let row = &agents.rows[0];
    assert!(
        row.summary_in_worktree,
        "the worktree holds 13-02's SUMMARY"
    );
    assert_eq!(row.liveness, AgentLiveness::Finished);

    let view = derive(&agents, &phase_13_state(&root));
    assert_eq!((view.done, view.finished, view.running), (1, 1, 0));
    assert!(!view.is_active(), "a finished agent alone is not running");
    assert_eq!(view.summary_forms(), Vec::<String>::new());

    // 13-03's executor, running: no SUMMARY, 30 s old, so Live.
    add_agent_worktree(&root, "agent-p13-03-1790386423");
    let agents = scan(&root, released_30s());
    assert_eq!(agents.rows.len(), 2, "{:?}", agents.rows);
    let view = derive(&agents, &phase_13_state(&root));
    assert_eq!((view.done, view.finished, view.running), (1, 1, 1));
    assert_eq!(
        view.summary_forms().first().map(String::as_str),
        Some("P13 \u{b7} w2/2 \u{b7} 1 run \u{b7} 2/3 done"),
        "done shown to the user is main-done plus finished-unmerged"
    );
}

#[test]
fn a_held_lock_with_a_worktree_summary_is_live_but_its_plan_is_finished() {
    let Some((_tmp, root)) = phase_repo() else {
        return;
    };
    let wt = add_agent_worktree(&root, P13_02);
    commit_in(
        &wt,
        ".planning/phases/13-demo/13-02-SUMMARY.md",
        "done\n",
        "docs(13-02): complete plan",
    );

    let agents = scan(
        &root,
        Scripted {
            lock_released: Some(false),
            ..released_30s()
        },
    );
    assert_eq!(agents.rows.len(), 1, "{:?}", agents.rows);
    assert!(agents.rows[0].summary_in_worktree);
    assert_eq!(
        agents.rows[0].liveness,
        AgentLiveness::Live,
        "a held lock is not finished liveness"
    );
    let view = derive(&agents, &phase_13_state(&root));
    assert_eq!(
        (view.finished, view.running),
        (1, 0),
        "the SUMMARY alone marks the plan finished"
    );
}

#[test]
fn another_plans_summary_does_not_mark_this_plan() {
    let Some((_tmp, root)) = phase_repo() else {
        return;
    };
    let wt = add_agent_worktree(&root, P13_02);
    commit_in(
        &wt,
        ".planning/phases/13-demo/13-03-SUMMARY.md",
        "done\n",
        "docs(13-03): complete plan",
    );

    let agents = scan(&root, released_30s());
    assert_eq!(agents.rows.len(), 1, "{:?}", agents.rows);
    assert!(
        !agents.rows[0].summary_in_worktree,
        "13-03's SUMMARY says nothing about 13-02"
    );
    assert_eq!(agents.rows[0].liveness, AgentLiveness::Live);
}

#[test]
fn a_phase_missing_from_main_skips_the_summary_check() {
    let Some((_tmp, root)) = phase_repo() else {
        return;
    };
    let wt = add_agent_worktree(&root, "agent-a0123456789abcdef");
    // The worktree has a phase directory main does not; the check resolves the
    // directory from MAIN's listing only, so it never looks here.
    commit_in(
        &wt,
        ".planning/phases/14-new/14-01-SUMMARY.md",
        "done\n",
        "docs(14-01): complete plan",
    );

    let agents = scan(
        &root,
        Scripted {
            description: Some("Execute plan 14-01 of phase 14"),
            ..released_30s()
        },
    );
    assert_eq!(agents.rows.len(), 1, "{:?}", agents.rows);
    let row = &agents.rows[0];
    assert_eq!(
        row.plan.as_ref().map(|p| p.label()),
        Some("14-01".to_string())
    );
    assert!(!row.summary_in_worktree, "no directory in main, no stat");
    assert_eq!(row.liveness, AgentLiveness::Live);
}
