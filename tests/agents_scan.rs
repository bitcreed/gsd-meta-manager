// ============================================================================
// AGENT-01 / AGENT-03: the agent observer's core scan and its adapter seam,
// against real repositories with real linked worktrees.
//
// This is an integration test rather than an in-source one for two reasons.
// Every proof here needs the real world — `git worktree add`, a real index to
// fingerprint, a worktree directory to delete from under git — and building
// that world means spawning git. `src/agents/` is deliberately NOT on the spawn
// allowlist in `tests/spawn_seam_guard.rs`, so even its `#[cfg(test)]` code may
// not spawn; `tests/` is not walked by that guard.
//
// It reuses `common::git` but never `common::fixture()`, which installs
// envelope hooks this suite has no business with.
// ============================================================================

mod common;

use std::path::{Path, PathBuf};
use std::time::SystemTime;

use common::git;
use gsd_meta_manager::agents::adapters::{AdapterReport, AgentAdapter, CoreSnapshot, Enrichment};
use gsd_meta_manager::agents::{scan_project_with, AgentLiveness};
use gsd_meta_manager::text::Untrusted;
use tempfile::TempDir;

/// The Claude-style agent id every fixture's first agent worktree carries.
const AGENT_ID: &str = "a0123456789abcdef";

/// A repository with one commit and one agent worktree at
/// `.claude/worktrees/agent-<AGENT_ID>` on branch `worktree-agent-<AGENT_ID>`.
///
/// `None` when the sandbox forbids `git init`, so the suite skips rather than
/// failing for a reason that is not about the code. Every step AFTER a
/// successful `git init` asserts instead: a fixture that silently degraded to
/// `None` there would turn every test in this file into a vacuous pass.
fn agents_fixture() -> Option<(TempDir, PathBuf)> {
    let (tmp, root) = plain_repo()?;
    add_worktree(
        &root,
        &format!("worktree-agent-{AGENT_ID}"),
        &format!(".claude/worktrees/agent-{AGENT_ID}"),
    );
    Some((tmp, root))
}

/// A repository with one commit (`tracked.txt`) and no linked worktree.
fn plain_repo() -> Option<(TempDir, PathBuf)> {
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

/// The fixture's agent worktree path.
fn agent_worktree(root: &Path) -> PathBuf {
    root.join(".claude")
        .join("worktrees")
        .join(format!("agent-{AGENT_ID}"))
}

/// Write `name` in `dir` and commit it.
fn commit_file(dir: &Path, name: &str, body: &str) {
    std::fs::write(dir.join(name), body).expect("fixture write");
    assert!(git(dir, &["add", name]), "git add {name}");
    assert!(
        git(dir, &["commit", "-m", &format!("add {name}"), "--quiet"]),
        "git commit {name}"
    );
}

/// A test-only adapter: reports `gsd-executor`, active now, for the worktree
/// whose agent id is [`AGENT_ID`]. Defined here, outside the crate, which is
/// the D-A07 point — the core needs no edit to accept it.
struct FakeAdapter;

impl AgentAdapter for FakeAdapter {
    fn name(&self) -> &'static str {
        "fake"
    }

    fn enrich(&self, snap: &CoreSnapshot<'_>) -> AdapterReport {
        let per_worktree = snap
            .worktrees
            .iter()
            .enumerate()
            .filter(|(_, wt)| wt.agent_id.as_deref() == Some(AGENT_ID))
            .map(|(index, _)| {
                (
                    index,
                    Enrichment {
                        agent_type: Some(Untrusted::from_untrusted_source("gsd-executor".into())),
                        last_activity: Some(snap.now),
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

// ---------------------------------------------------------------------------
// The tracer
// ---------------------------------------------------------------------------

#[test]
fn one_agent_worktree_is_found_with_its_commit_and_dirty_counts() {
    let Some((_tmp, root)) = agents_fixture() else {
        return;
    };
    let wt = agent_worktree(&root);
    commit_file(&wt, "a.txt", "a\n");
    commit_file(&wt, "b.txt", "b\n");
    std::fs::write(wt.join("tracked.txt"), "modified\n").unwrap();
    std::fs::write(wt.join("untracked.txt"), "new\n").unwrap();

    let now = SystemTime::now();
    let adapters: Vec<Box<dyn AgentAdapter>> = vec![Box::new(FakeAdapter)];
    let scan = scan_project_with(&root, &adapters, now);

    assert_eq!(scan.rows.len(), 1, "exactly one agent row: {:?}", scan.rows);
    let row = &scan.rows[0];
    assert_eq!(row.path, wt);
    assert_eq!(row.commits_ahead, Some(2), "two commits beyond main's HEAD");
    assert_eq!(row.dirty, Some(2), "one modified plus one untracked");
    assert_eq!(row.agent_id.as_deref(), Some(AGENT_ID));
    assert_eq!(row.adapter, Some("fake"));
    assert_eq!(
        row.agent_type.as_ref().map(|t| t.shown().to_string()),
        Some("gsd-executor".to_string())
    );
    assert_eq!(row.liveness, AgentLiveness::Live);
    assert_eq!(scan.scanned_at, Some(now));
    assert!(
        scan.base_sha.is_some(),
        "the main worktree's HEAD is the base"
    );
}
