// ============================================================================
// AGENT-07: the code-review fix-run estimate, against real repositories with
// real linked worktrees — a phase's REVIEW.md total, the `fix(NN): …` subjects
// on the fixer worktrees and on main, through the real scan and `derive`.
//
// This is an integration test rather than an in-source one for the reason
// `tests/agents_scan.rs` gives: every proof here needs `git worktree add` and
// real commits, and `src/agents/` is deliberately NOT on the spawn allowlist in
// `tests/spawn_seam_guard.rs`, so even its `#[cfg(test)]` code may not spawn.
// The pure parsing (`finding_ids`, `fixer_phase`) is pinned in-source, in
// `src/agents/fixers.rs`.
//
// It reuses `common::git` but never `common::fixture()`, which installs
// envelope hooks this suite has no business with.
// ============================================================================

mod common;

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use common::git;
use gsd_meta_manager::agents::adapters::{AdapterReport, AgentAdapter, CoreSnapshot, Enrichment};
use gsd_meta_manager::agents::fixers::{self, FixerEstimate};
use gsd_meta_manager::agents::waves::derive;
use gsd_meta_manager::agents::waves::PlanRef;
use gsd_meta_manager::agents::{scan_project_with, AgentLiveness, AgentRow, ProjectAgents};
use gsd_meta_manager::state_reader::phase_num::PhaseNum;
use gsd_meta_manager::state_reader::ProjectState;
use gsd_meta_manager::text::Untrusted;
use tempfile::TempDir;

/// A Claude-style fixer worktree id: it carries no plan.
const FIXER_A: &str = "agent-a0123456789abcdef";
const FIXER_B: &str = "agent-b0123456789abcdef";
const FIXER_C: &str = "agent-c0123456789abcdef";
const FIXER_D: &str = "agent-d0123456789abcdef";

/// The phase-12 review frontmatter GSD writes: a `findings:` block whose
/// `total` is the denominator.
const REVIEW_48: &str = "---\nphase: 12\nfindings:\n  critical: 2\n  warning: 22\n  info: 24\n  total: 48\n---\n\n# Review\n";

/// A repository whose first commit holds `.planning/phases/12-cli/` with
/// `review` as `12-REVIEW.md` (none when `review` is `None`).
///
/// `None` when the sandbox forbids `git init`; every later step asserts, so a
/// degraded fixture can never make a test pass vacuously.
fn review_repo(review: Option<&str>) -> Option<(TempDir, PathBuf)> {
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
    let phase = root.join(".planning/phases/12-cli");
    std::fs::create_dir_all(&phase).expect("fixture dir");
    std::fs::write(
        phase.join("12-01-PLAN.md"),
        "---\nphase: 12\nwave: 1\n---\n",
    )
    .expect("fixture write");
    if let Some(review) = review {
        std::fs::write(phase.join("12-REVIEW.md"), review).expect("fixture write");
    }
    assert!(git(&root, &["add", "."]), "git add");
    assert!(
        git(&root, &["commit", "-m", "docs(12): review", "--quiet"]),
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

/// Write a file unique to `subject` under `dir` and commit it with `subject`.
fn commit(dir: &Path, subject: &str) {
    let rel = format!(
        "src/{}.txt",
        subject
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect::<String>()
    );
    let path = dir.join(&rel);
    std::fs::create_dir_all(path.parent().expect("rel has a parent")).expect("fixture dir");
    std::fs::write(&path, subject).expect("fixture write");
    assert!(git(dir, &["add", &rel]), "git add {rel}");
    assert!(
        git(dir, &["commit", "-m", subject, "--quiet"]),
        "git commit {subject}"
    );
}

/// A test-only adapter reporting the same scripted facts for every worktree
/// the agent predicate matches.
struct Scripted {
    agent_type: &'static str,
    description: Option<&'static str>,
    age_secs: u64,
    ended: bool,
    lock_released: Option<bool>,
}

impl Scripted {
    /// A live `gsd-code-fixer` on phase 12's findings, its lock held.
    fn fixer() -> Self {
        Scripted {
            agent_type: "gsd-code-fixer",
            description: Some("Fix phase 12 CLI findings"),
            age_secs: 0,
            ended: false,
            lock_released: Some(false),
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
                        agent_type: Some(Untrusted::from_untrusted_source(self.agent_type.into())),
                        description: self
                            .description
                            .map(|d| Untrusted::from_untrusted_source(d.into())),
                        last_activity: Some(snap.now - Duration::from_secs(self.age_secs)),
                        lock_released: self.lock_released,
                        ended: self.ended,
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

/// The estimate's counts: `(fixers, fixed, total)`.
fn counts(agents: &ProjectAgents) -> Option<(u32, Option<u32>, Option<u32>)> {
    agents
        .fixer_estimate
        .as_ref()
        .map(|e| (e.fixers, e.fixed, e.total))
}

// ---------------------------------------------------------------------------
// The tracer
// ---------------------------------------------------------------------------

#[test]
fn a_code_fixer_run_shows_an_estimated_fixed_over_total() {
    let Some((_tmp, root)) = review_repo(Some(REVIEW_48)) else {
        return;
    };
    commit(&root, "fix(12): WR-01 tidy");
    let wt = add_agent_worktree(&root, FIXER_A);
    commit(&wt, "fix(12): WR-08 x");
    commit(&wt, "fix(12): CR-01 y");
    commit(&wt, "fix(12-sec): IN-03 z");

    let agents = scan(&root, Scripted::fixer());
    assert_eq!(agents.rows.len(), 1, "{:?}", agents.rows);
    assert_eq!(agents.rows[0].liveness, AgentLiveness::Live);
    assert_eq!(agents.rows[0].plan, None, "a fixer is never attributed");
    assert_eq!(
        agents.fixer_estimate,
        Some(FixerEstimate {
            fixers: 1,
            phase: PhaseNum::parse("12"),
            fixed: Some(4),
            total: Some(48),
        }),
        "WR-01 on main plus WR-08, CR-01 and IN-03 on the fixer, of 48"
    );

    let view = derive(&agents, &ProjectState::default());
    assert_eq!(
        view.summary_forms().first().map(String::as_str),
        Some("1 fixer \u{b7} ~4/48 fixed")
    );
}

// ---------------------------------------------------------------------------
// Edge cases
// ---------------------------------------------------------------------------

#[test]
fn finding_ids_dedupe_across_worktrees_and_main() {
    let Some((_tmp, root)) = review_repo(Some(REVIEW_48)) else {
        return;
    };
    commit(&root, "fix(12): WR-08 on main");
    commit(&root, "fix(13): WR-09 another phase");
    let a = add_agent_worktree(&root, FIXER_A);
    let b = add_agent_worktree(&root, FIXER_B);
    let c = add_agent_worktree(&root, FIXER_C);
    commit(&a, "fix(12): WR-08 on a");
    commit(&b, "fix(12): WR-08 on b");
    commit(&c, "fix(12): CR-02 on c");

    let agents = scan(&root, Scripted::fixer());
    assert_eq!(agents.rows.len(), 3, "{:?}", agents.rows);
    assert_eq!(
        counts(&agents),
        Some((3, Some(2), Some(48))),
        "WR-08 (three times) and CR-02; phase 13's WR-09 never counts"
    );
    assert_eq!(
        derive(&agents, &ProjectState::default()).summary_forms(),
        vec![
            "3 fixers \u{b7} ~2/48 fixed".to_string(),
            "3 fixers \u{b7} ~2/48".to_string(),
            "3fix ~2/48".to_string(),
            "3fix".to_string(),
        ]
    );
}

/// WR-04: a `*-REVIEW-FIX.md` left by an earlier run, or by `--auto`
/// iteration 1, no longer hides the count of a run that is still going —
/// whether the run is over is the running-fixer gate's call. (Until WR-04 this
/// test pinned the opposite: any REVIEW-FIX.md meant "no counts".)
#[test]
fn a_leftover_review_fix_report_does_not_hide_a_running_estimate() {
    let Some((_tmp, root)) = review_repo(Some(REVIEW_48)) else {
        return;
    };
    let dir = root.join(".planning/phases/12-cli");
    std::fs::write(dir.join("12-REVIEW-FIX.md"), "---\nfixed: 30\n---\n").expect("fixture write");
    assert!(git(&root, &["add", "."]), "git add");
    assert!(
        git(
            &root,
            &["commit", "-m", "docs(12): review fix report", "--quiet"]
        ),
        "git commit"
    );
    for (n, name) in [FIXER_A, FIXER_B, FIXER_C].into_iter().enumerate() {
        let wt = add_agent_worktree(&root, name);
        commit(&wt, &format!("fix(12): WR-0{} fixed", n + 1));
    }

    let agents = scan(&root, Scripted::fixer());
    assert_eq!(
        agents.fixer_estimate,
        Some(FixerEstimate {
            fixers: 3,
            phase: PhaseNum::parse("12"),
            fixed: Some(3),
            total: Some(48),
        }),
        "three fixers are running: the counts stay"
    );
    assert_eq!(
        derive(&agents, &ProjectState::default())
            .summary_forms()
            .first()
            .map(String::as_str),
        Some("3 fixers \u{b7} ~3/48 fixed")
    );
}

/// WR-04: ids an earlier review's run fixed are not this review's findings,
/// and `fixed` never exceeds `total`.
#[test]
fn only_the_current_reviews_findings_count_and_fixed_never_exceeds_total() {
    let review = "---\nphase: 12\nfindings:\n  total: 2\n---\n\n### CR-01: a\n\n### WR-01: b\n";
    let Some((_tmp, root)) = review_repo(Some(review)) else {
        return;
    };
    // An earlier review of phase 12, fixed and merged: WR-05 and IN-02 are
    // not findings of the current review.
    commit(&root, "fix(12): WR-05 earlier review");
    commit(&root, "fix(12): IN-02 earlier review");
    let wt = add_agent_worktree(&root, FIXER_A);
    commit(&wt, "fix(12): CR-01 x");
    commit(&wt, "fix(12): WR-1 y");

    let agents = scan(&root, Scripted::fixer());
    assert_eq!(counts(&agents), Some((1, Some(2), Some(2))));

    // A review without finding headings: every id counts, but at most the total.
    let Some((_tmp2, headless)) = review_repo(Some("---\nfindings:\n  total: 2\n---\n")) else {
        return;
    };
    let wt = add_agent_worktree(&headless, FIXER_A);
    commit(&wt, "fix(12): CR-01 WR-01 WR-02 x");
    let agents = scan(&headless, Scripted::fixer());
    assert_eq!(counts(&agents), Some((1, Some(2), Some(2))), "clamped to the total");
}

/// WR-03: the denominator is the code review's, even when an eval review and a
/// UI review sit beside it — `12-EVAL-REVIEW.md` sorts before `12-REVIEW.md`.
#[test]
fn eval_and_ui_reviews_never_supply_the_total() {
    let Some((_tmp, root)) = review_repo(Some(REVIEW_48)) else {
        return;
    };
    let dir = root.join(".planning/phases/12-cli");
    let other = "---\nfindings:\n  total: 5\n---\n";
    std::fs::write(dir.join("12-EVAL-REVIEW.md"), other).expect("fixture write");
    std::fs::write(dir.join("12-UI-REVIEW.md"), other).expect("fixture write");
    let wt = add_agent_worktree(&root, FIXER_A);
    commit(&wt, "fix(12): WR-08 x");

    let agents = scan(&root, Scripted::fixer());
    assert_eq!(counts(&agents), Some((1, Some(1), Some(48))));
}

#[test]
fn a_missing_or_unreadable_review_total_gives_no_count() {
    let no_total = "---\nfindings:\n  critical: 2\n  warning: 22\n---\n";
    let not_a_number = "---\nfindings:\n  critical: 2\n  total: many\n---\n";
    let nested_too_deep = "---\nfindings:\n  by_kind:\n    total: 48\n---\n";
    for (label, review) in [
        ("no REVIEW.md", None),
        ("findings without total", Some(no_total)),
        ("total: many", Some(not_a_number)),
        ("total one level too deep", Some(nested_too_deep)),
    ] {
        let Some((_tmp, root)) = review_repo(review) else {
            return;
        };
        let wt = add_agent_worktree(&root, FIXER_A);
        commit(&wt, "fix(12): WR-08 x");

        let agents = scan(&root, Scripted::fixer());
        assert_eq!(
            agents.fixer_estimate,
            Some(FixerEstimate {
                fixers: 1,
                phase: PhaseNum::parse("12"),
                fixed: None,
                total: None,
            }),
            "{label}"
        );
        assert_eq!(
            derive(&agents, &ProjectState::default()).summary_forms(),
            vec!["1 fixer".to_string(), "1fix".to_string()],
            "{label}"
        );
    }
}

#[test]
fn attributed_or_inactive_fixers_are_not_counted() {
    let Some((_tmp, root)) = review_repo(Some(REVIEW_48)) else {
        return;
    };
    let wt = add_agent_worktree(&root, FIXER_A);
    commit(&wt, "fix(12): WR-08 x");

    // A fixer the scan attributes to a plan is executing that plan.
    let attributed = scan(
        &root,
        Scripted {
            description: Some("Execute plan 12-01 of phase 12"),
            ..Scripted::fixer()
        },
    );
    assert!(attributed.rows[0].plan.is_some(), "{:?}", attributed.rows);
    assert_eq!(attributed.fixer_estimate, None, "attributed");

    // Stalled, and ended: not active.
    let stalled = scan(
        &root,
        Scripted {
            age_secs: 700,
            ..Scripted::fixer()
        },
    );
    assert_eq!(stalled.rows[0].liveness, AgentLiveness::Stalled);
    assert_eq!(stalled.fixer_estimate, None, "stalled");
    let ended = scan(
        &root,
        Scripted {
            ended: true,
            ..Scripted::fixer()
        },
    );
    assert_eq!(ended.rows[0].liveness, AgentLiveness::Ended);
    assert_eq!(ended.fixer_estimate, None, "ended");

    // A live agent of another type is not a fixer.
    let executor = scan(
        &root,
        Scripted {
            agent_type: "gsd-executor",
            ..Scripted::fixer()
        },
    );
    assert_eq!(executor.rows[0].liveness, AgentLiveness::Live);
    assert_eq!(executor.fixer_estimate, None, "not a fixer");

    // Unknown (no adapter facts) and attributed rows, built by hand. The
    // estimate returns before any read when no row is an active fixer, so a
    // project root and a main worktree that do not exist are never touched.
    let fixer_type = || Some(Untrusted::from_untrusted_source("gsd-code-fixer".into()));
    let rows = vec![
        AgentRow {
            path: root.join(".claude/worktrees").join(FIXER_A),
            agent_type: fixer_type(),
            liveness: AgentLiveness::Unknown,
            ..AgentRow::default()
        },
        AgentRow {
            path: root.join(".claude/worktrees").join(FIXER_B),
            agent_type: fixer_type(),
            liveness: AgentLiveness::Live,
            plan: PlanRef::from_id("12-01"),
            ..AgentRow::default()
        },
    ];
    let gone = root.join("does-not-exist");
    assert_eq!(
        fixers::estimate(
            &gone,
            Some(&gone),
            Some("0123456789abcdef0123456789abcdef01234567"),
            &rows
        ),
        None
    );
    assert_eq!(
        fixers::estimate(&root, None, None, &[]),
        None,
        "no rows at all"
    );
}

/// 25-07 (CR-01, D-C12): the estimate needs a running (Live or Idle) fixer. An
/// orphaned fix run whose fixers all finished shows no `N fixers`, while a
/// running run still counts its finished fixers and their commits.
#[test]
fn a_finished_fixer_run_yields_no_estimate() {
    let Some((_tmp, root)) = review_repo(Some(REVIEW_48)) else {
        return;
    };
    let a = add_agent_worktree(&root, FIXER_A);
    commit(&a, "fix(12): WR-08 x");

    // Lock released, 200 s silent: Finished.
    let finished = scan(
        &root,
        Scripted {
            age_secs: 200,
            lock_released: Some(true),
            ..Scripted::fixer()
        },
    );
    assert_eq!(finished.rows[0].liveness, AgentLiveness::Finished);
    assert_eq!(finished.fixer_estimate, None, "a finished run");
    assert_eq!(
        derive(&finished, &ProjectState::default()).summary_forms(),
        Vec::<String>::new()
    );

    // Three days silent: Ended.
    let aged = scan(
        &root,
        Scripted {
            age_secs: 3 * 86_400,
            ..Scripted::fixer()
        },
    );
    assert_eq!(aged.rows[0].liveness, AgentLiveness::Ended);
    assert_eq!(aged.fixer_estimate, None, "an aged-out run");

    // Two fixers; the base sha and the rows are git's own, then the
    // liveness is edited by hand.
    let b = add_agent_worktree(&root, FIXER_B);
    commit(&b, "fix(12): CR-01 y");
    let live = scan(&root, Scripted::fixer());
    assert_eq!(live.rows.len(), 2, "{:?}", live.rows);
    let with = |a_liveness, b_liveness| -> Option<FixerEstimate> {
        let rows: Vec<AgentRow> = live
            .rows
            .iter()
            .cloned()
            .map(|mut row| {
                row.liveness = if row.path.ends_with(FIXER_A) {
                    a_liveness
                } else {
                    b_liveness
                };
                row
            })
            .collect();
        fixers::estimate(
            &root,
            live.main_worktree.as_deref(),
            live.base_sha.as_deref(),
            &rows,
        )
    };
    assert_eq!(
        with(AgentLiveness::Finished, AgentLiveness::Finished),
        None,
        "every fixer finished"
    );
    assert_eq!(
        with(AgentLiveness::Finished, AgentLiveness::Live),
        Some(FixerEstimate {
            fixers: 2,
            phase: PhaseNum::parse("12"),
            fixed: Some(2),
            total: Some(48),
        }),
        "a finished fixer still counts inside a running run"
    );

    // The gate runs before any git call or file read.
    let gone = root.join("does-not-exist");
    let row = AgentRow {
        path: gone.join(FIXER_A),
        agent_type: Some(Untrusted::from_untrusted_source("gsd-code-fixer".into())),
        liveness: AgentLiveness::Finished,
        ..AgentRow::default()
    };
    assert_eq!(
        fixers::estimate(
            &gone,
            Some(&gone),
            Some("0123456789abcdef0123456789abcdef01234567"),
            &[row]
        ),
        None
    );
}

/// WR-06: finished orphan fixers of an aborted run on phase 12 do not outvote
/// the one fixer running on phase 13, and are not part of its run: neither its
/// `N fixers` nor its `fixed` count.
#[test]
fn a_running_fixer_outvotes_finished_orphans_of_another_phase() {
    let Some((_tmp, root)) = review_repo(Some(REVIEW_48)) else {
        return;
    };
    let phase13 = root.join(".planning/phases/13-api");
    std::fs::create_dir_all(&phase13).expect("fixture dir");
    std::fs::write(
        phase13.join("13-REVIEW.md"),
        "---\nfindings:\n  total: 5\n---\n\n### WR-01: a\n\n### WR-02: b\n",
    )
    .expect("fixture write");
    for (n, name) in [FIXER_A, FIXER_B, FIXER_C].into_iter().enumerate() {
        let wt = add_agent_worktree(&root, name);
        commit(&wt, &format!("fix(12): WR-0{} orphan", n + 1));
    }
    let d = add_agent_worktree(&root, FIXER_D);
    commit(&d, "fix(13): WR-01 running");

    // No description, so each fixer's phase comes from its own commits; then
    // the three phase-12 fixers are marked finished by hand.
    let scanned = scan(
        &root,
        Scripted {
            description: None,
            ..Scripted::fixer()
        },
    );
    assert_eq!(scanned.rows.len(), 4, "{:?}", scanned.rows);
    let rows: Vec<AgentRow> = scanned
        .rows
        .iter()
        .cloned()
        .map(|mut row| {
            if !row.path.ends_with(FIXER_D) {
                row.liveness = AgentLiveness::Finished;
            }
            row
        })
        .collect();
    let estimate = fixers::estimate(
        &root,
        scanned.main_worktree.as_deref(),
        scanned.base_sha.as_deref(),
        &rows,
    );
    assert_eq!(
        estimate,
        Some(FixerEstimate {
            fixers: 1,
            phase: PhaseNum::parse("13"),
            fixed: Some(1),
            total: Some(5),
        }),
        "the running fixer's phase 13, not the orphans' 3-to-1 phase 12"
    );

    // Once the phase-13 fixer finishes too, the run is over (CR-01).
    let all_finished: Vec<AgentRow> = rows
        .iter()
        .cloned()
        .map(|mut row| {
            row.liveness = AgentLiveness::Finished;
            row
        })
        .collect();
    assert_eq!(
        fixers::estimate(
            &root,
            scanned.main_worktree.as_deref(),
            scanned.base_sha.as_deref(),
            &all_finished,
        ),
        None
    );
}

#[test]
fn the_phase_comes_from_the_fixers_own_commits_when_the_description_has_none() {
    let Some((_tmp, root)) = review_repo(Some(REVIEW_48)) else {
        return;
    };
    let wt = add_agent_worktree(&root, FIXER_A);
    let without_phase = || Scripted {
        description: Some("Fix the review findings"),
        ..Scripted::fixer()
    };

    // Before its first commit, nothing names a phase: the fixer count only.
    let before = scan(&root, without_phase());
    assert_eq!(
        before.fixer_estimate,
        Some(FixerEstimate {
            fixers: 1,
            ..FixerEstimate::default()
        })
    );
    assert_eq!(
        derive(&before, &ProjectState::default()).summary_forms(),
        vec!["1 fixer".to_string(), "1fix".to_string()]
    );

    commit(&wt, "fix(12): WR-02 y");
    let after = scan(&root, without_phase());
    assert_eq!(
        after.fixer_estimate,
        Some(FixerEstimate {
            fixers: 1,
            phase: PhaseNum::parse("12"),
            fixed: Some(1),
            total: Some(48),
        }),
        "the fixer's own fix(12) scope names the phase"
    );
}
