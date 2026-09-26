// ============================================================================
// Quick 260926-06g: the Linux-only process signal for the running-agents view,
// against real repositories with real locked worktrees.
//
// An integration test for the same reason `tests/agents_scan.rs` is one: every
// proof needs `git worktree add` and `git worktree lock`, and `src/agents/` may
// not spawn even in its `#[cfg(test)]` code. `tests/` is not walked by the
// spawn guard.
//
// The real-probe cases are `#[cfg(target_os = "linux")]`: they drive the real
// `session_detector::process_probe()` against this test process's own pid and
// against a pid that cannot exist. Everything else is platform-independent and
// pins that the probe-less scan is exactly the pre-change, mtime-only scan.
//
// It reuses `common::git` but never `common::fixture()`.
// ============================================================================

mod common;

use std::path::PathBuf;

use common::git;
use gsd_meta_manager::agents::adapters::{AdapterReport, AgentAdapter, CoreSnapshot, Enrichment};
use gsd_meta_manager::agents::{scan_project_with, scan_project_with_probe, AgentLiveness};
use gsd_meta_manager::session_detector::{NoProcessProbe, PidObservation, ProcessProbe};
use std::time::SystemTime;
use tempfile::TempDir;

/// The Claude-style agent id every fixture's agent worktree carries.
const AGENT_ID: &str = "a0123456789abcdef";

/// A repository with one commit and no linked worktree; `None` when the
/// sandbox forbids `git init`.
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

/// A repository with one agent worktree, locked with `reason` when given.
fn locked_fixture(reason: Option<&str>) -> Option<(TempDir, PathBuf)> {
    let (tmp, root) = plain_repo()?;
    let path = format!(".claude/worktrees/agent-{AGENT_ID}");
    assert!(
        git(
            &root,
            &[
                "worktree",
                "add",
                "--quiet",
                "-b",
                &format!("worktree-agent-{AGENT_ID}"),
                &path
            ]
        ),
        "git worktree add"
    );
    if let Some(reason) = reason {
        assert!(
            git(&root, &["worktree", "lock", "--reason", reason, &path]),
            "git worktree lock --reason {reason:?}"
        );
    }
    Some((tmp, root))
}

/// A test-only adapter: reports a transcript written `now` for the agent
/// worktree, so the mtime-only answer is always `Live`.
struct FreshAdapter;

impl AgentAdapter for FreshAdapter {
    fn name(&self) -> &'static str {
        "fresh"
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

fn fresh() -> Vec<Box<dyn AgentAdapter>> {
    vec![Box::new(FreshAdapter)]
}

/// The single agent row's liveness.
fn only_liveness(scan: &gsd_meta_manager::agents::ProjectAgents) -> AgentLiveness {
    assert_eq!(scan.rows.len(), 1, "one agent row: {:?}", scan.rows);
    scan.rows[0].liveness
}

/// The three lock reasons of the real-probe cases, keyed by name.
fn reasons() -> Vec<(&'static str, String)> {
    let own = std::process::id();
    vec![
        (
            "dead owner",
            format!("claude agent agent-{AGENT_ID} (pid 4294967295 start 1)"),
        ),
        (
            "own pid, no start",
            format!("claude agent agent-{AGENT_ID} (pid {own})"),
        ),
        (
            "own pid, wrong start",
            format!("claude agent agent-{AGENT_ID} (pid {own} start 1)"),
        ),
    ]
}

// ---------------------------------------------------------------------------
// Platform-independent: no probe is the pre-change scan
// ---------------------------------------------------------------------------

#[test]
fn without_a_probe_every_lock_owner_reads_as_the_mtime_says() {
    for (name, reason) in reasons() {
        let Some((_tmp, root)) = locked_fixture(Some(&reason)) else {
            return;
        };
        let now = SystemTime::now();
        let plain = scan_project_with(&root, &fresh(), now);
        assert_eq!(only_liveness(&plain), AgentLiveness::Live, "{name}");
        assert_eq!(
            scan_project_with_probe(&root, &fresh(), &NoProcessProbe, now),
            plain,
            "{name}: NoProcessProbe is exactly scan_project_with"
        );
    }
}

/// A process table injected as data: every pid unknown, and a fixed list of
/// `codex` cwds.
struct CodexProbe(Vec<PathBuf>);

impl ProcessProbe for CodexProbe {
    fn observe(&self, _pid: u32) -> PidObservation {
        PidObservation::Unknown
    }

    fn codex_cwds(&self) -> Option<&[PathBuf]> {
        Some(&self.0)
    }
}

/// A Codex executor has no adapter (D-A08): with no probe its worktree reads
/// `Unknown`, and a `codex` working inside it makes it `Live`.
#[test]
fn a_codex_process_inside_an_agent_worktree_reads_live() {
    let Some((_tmp, root)) = locked_fixture(None) else {
        return;
    };
    let wt = root
        .join(".claude")
        .join("worktrees")
        .join(format!("agent-{AGENT_ID}"));
    let now = SystemTime::now();
    let no_adapters: Vec<Box<dyn AgentAdapter>> = Vec::new();

    let before = scan_project_with_probe(&root, &no_adapters, &NoProcessProbe, now);
    assert_eq!(only_liveness(&before), AgentLiveness::Unknown);
    assert_eq!(before, scan_project_with(&root, &no_adapters, now));

    for cwd in [wt.clone(), wt.join("src")] {
        let probe = CodexProbe(vec![PathBuf::from("/elsewhere"), cwd.clone()]);
        let scan = scan_project_with_probe(&root, &no_adapters, &probe, now);
        assert_eq!(only_liveness(&scan), AgentLiveness::Live, "{cwd:?}");
    }

    let main_only = CodexProbe(vec![root.clone()]);
    assert_eq!(
        only_liveness(&scan_project_with_probe(
            &root,
            &no_adapters,
            &main_only,
            now
        )),
        AgentLiveness::Unknown,
        "a codex in the main worktree is not inside the agent worktree"
    );
}

// ---------------------------------------------------------------------------
// Linux: the real procfs probe
// ---------------------------------------------------------------------------

#[cfg(target_os = "linux")]
mod linux {
    use super::*;
    use gsd_meta_manager::session_detector::{process_probe, PidObservation};
    use std::path::Path;

    /// The fixture's agent worktree path.
    fn agent_worktree(root: &Path) -> PathBuf {
        root.join(".claude")
            .join("worktrees")
            .join(format!("agent-{AGENT_ID}"))
    }

    /// This process's own procfs start time, as the real probe sees it.
    fn own_start() -> u64 {
        match process_probe().observe(std::process::id()) {
            PidObservation::Present {
                start_time: Some(start),
                ..
            } => start,
            other => panic!("the real probe must see this process: {other:?}"),
        }
    }

    fn scan_real(root: &Path) -> AgentLiveness {
        let probe = process_probe();
        only_liveness(&scan_project_with_probe(
            root,
            &fresh(),
            probe.as_ref(),
            SystemTime::now(),
        ))
    }

    #[test]
    fn a_dead_lock_owner_reads_ended_despite_a_fresh_transcript() {
        let reason = format!("claude agent agent-{AGENT_ID} (pid 4294967295 start 1)");
        let Some((_tmp, root)) = locked_fixture(Some(&reason)) else {
            return;
        };
        assert!(agent_worktree(&root).is_dir());
        assert_eq!(scan_real(&root), AgentLiveness::Ended);
    }

    #[test]
    fn a_live_lock_owner_with_its_own_start_changes_nothing() {
        let reason = format!(
            "claude agent agent-{AGENT_ID} (pid {} start {})",
            std::process::id(),
            own_start()
        );
        let Some((_tmp, root)) = locked_fixture(Some(&reason)) else {
            return;
        };
        assert_eq!(scan_real(&root), AgentLiveness::Live);
    }

    #[test]
    fn a_reused_pid_reads_ended() {
        let reason = format!(
            "claude agent agent-{AGENT_ID} (pid {} start {})",
            std::process::id(),
            own_start() + 1
        );
        let Some((_tmp, root)) = locked_fixture(Some(&reason)) else {
            return;
        };
        assert_eq!(scan_real(&root), AgentLiveness::Ended);
    }

    /// With no start in the lock, the owner's cwd decides. This test process
    /// works in the package root, outside the temporary project: the shape of
    /// a pid reused by an unrelated process.
    #[test]
    fn an_owner_working_outside_the_project_reads_ended() {
        let cwd = std::env::current_dir().expect("the test process has a cwd");
        let reason = format!("claude agent agent-{AGENT_ID} (pid {})", std::process::id());
        let Some((_tmp, root)) = locked_fixture(Some(&reason)) else {
            return;
        };
        assert!(!cwd.starts_with(&root), "the fixture is outside the cwd");
        assert_eq!(scan_real(&root), AgentLiveness::Ended);
    }

    #[test]
    fn an_unlocked_worktree_is_untouched_by_the_probe() {
        let Some((_tmp, root)) = locked_fixture(None) else {
            return;
        };
        assert_eq!(scan_real(&root), AgentLiveness::Live);
    }
}
