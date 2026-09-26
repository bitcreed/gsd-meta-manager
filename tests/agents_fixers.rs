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
use gsd_meta_manager::agents::fixers::FixerEstimate;
use gsd_meta_manager::agents::waves::derive;
use gsd_meta_manager::agents::{scan_project_with, AgentLiveness, ProjectAgents};
use gsd_meta_manager::state_reader::phase_num::PhaseNum;
use gsd_meta_manager::state_reader::ProjectState;
use gsd_meta_manager::text::Untrusted;
use tempfile::TempDir;

/// A Claude-style fixer worktree id: it carries no plan.
const FIXER_A: &str = "agent-a0123456789abcdef";

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
    std::fs::write(phase.join("12-01-PLAN.md"), "---\nphase: 12\nwave: 1\n---\n")
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
}

impl Scripted {
    /// A live `gsd-code-fixer` on phase 12's findings.
    fn fixer() -> Self {
        Scripted {
            agent_type: "gsd-code-fixer",
            description: Some("Fix phase 12 CLI findings"),
            age_secs: 0,
            ended: false,
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
                        agent_type: Some(Untrusted::from_untrusted_source(
                            self.agent_type.into(),
                        )),
                        description: self
                            .description
                            .map(|d| Untrusted::from_untrusted_source(d.into())),
                        last_activity: Some(snap.now - Duration::from_secs(self.age_secs)),
                        lock_released: Some(false),
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
