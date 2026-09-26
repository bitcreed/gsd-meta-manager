// ============================================================================
// AGENT-04 / AGENT-03: the Claude Code adapter, against real repositories with
// real linked worktrees and a FAKE Claude config root.
//
// An integration test rather than an in-source one for the reason
// `tests/agents_scan.rs` gives: building a real worktree means spawning git,
// and `src/agents/` is not on the spawn allowlist in `tests/spawn_seam_guard.rs`,
// test code included. `tests/` is not walked by that guard.
//
// Every test roots the adapter at a tempdir through `ClaudeCodeAdapter::new`
// and injects its clock; none calls `from_env()` or reads the real home
// directory (D-C17, RESEARCH Pitfall 10). Transcript mtimes are set with
// `File::set_modified`, never by sleeping.
//
// It reuses `common::git` but never `common::fixture()`, which installs
// envelope hooks this suite has no business with.
// ============================================================================

mod common;

use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use common::git;
use gsd_meta_manager::agents::adapters::claude::{encode_project_dir, ClaudeCodeAdapter};
use gsd_meta_manager::agents::adapters::{registered_adapters, AgentAdapter};
use gsd_meta_manager::agents::{scan_project_with, AgentLiveness, ProjectAgents};
use serde_json::json;
use tempfile::TempDir;

/// The Claude-style agent id of the fixture's executor worktree.
const AGENT_ID: &str = "a0123456789abcdef";

/// The session every fixture files its subagents under.
const SESSION: &str = "0b6f7a3e-1111-4222-8333-944455556666";

/// A real repository plus a fake Claude config root that knows about it.
struct ClaudeFixture {
    /// Held for its Drop; every path below lives inside it.
    _tmp: TempDir,
    /// The registered project (the main worktree).
    work: PathBuf,
    /// The fake Claude config root.
    root: PathBuf,
    /// `<root>/projects/<encoded work>/<SESSION>/subagents`.
    subagents: PathBuf,
}

impl ClaudeFixture {
    /// `git worktree add -b worktree-agent-<id> .claude/worktrees/agent-<id>`,
    /// locked with Claude Code's reason when `lock_pid` is given.
    fn add_agent_worktree(&self, id: &str, lock_pid: Option<u32>) -> PathBuf {
        let rel = format!(".claude/worktrees/agent-{id}");
        assert!(
            git(
                &self.work,
                &[
                    "worktree",
                    "add",
                    "--quiet",
                    "-b",
                    &format!("worktree-agent-{id}"),
                    &rel
                ]
            ),
            "git worktree add {rel}"
        );
        let path = self.work.join(&rel);
        if let Some(pid) = lock_pid {
            let reason = format!("claude agent agent-{id} (pid {pid} start 2)");
            assert!(
                git(&self.work, &["worktree", "lock", "--reason", &reason, &rel]),
                "git worktree lock {rel}"
            );
        }
        path
    }

    /// Write `agent-<id>.meta.json` in the session's `subagents/`.
    fn write_meta(&self, id: &str, meta: &serde_json::Value) {
        write_meta_in(&self.subagents, id, meta);
    }

    /// Write `agent-<id>.jsonl` with its mtime `age` before `now`.
    fn write_transcript(&self, id: &str, now: SystemTime, age: Duration) {
        write_transcript_in(&self.subagents, id, now - age);
    }

    fn adapters(&self) -> Vec<Box<dyn AgentAdapter>> {
        vec![Box::new(ClaudeCodeAdapter::new(self.root.clone()))]
    }

    fn scan(&self, now: SystemTime) -> ProjectAgents {
        scan_project_with(&self.work, &self.adapters(), now)
    }
}

fn write_meta_in(subagents: &Path, id: &str, meta: &serde_json::Value) {
    std::fs::write(
        subagents.join(format!("agent-{id}.meta.json")),
        serde_json::to_vec(meta).expect("serializable"),
    )
    .expect("fixture meta write");
}

fn write_transcript_in(subagents: &Path, id: &str, modified: SystemTime) {
    let path = subagents.join(format!("agent-{id}.jsonl"));
    std::fs::write(&path, "{\"type\":\"user\"}\n").expect("fixture transcript write");
    set_mtime(&path, modified);
}

/// Set a file's or directory's mtime without writing its contents.
fn set_mtime(path: &Path, modified: SystemTime) {
    std::fs::File::open(path)
        .and_then(|f| f.set_modified(modified))
        .expect("set mtime");
}

/// A repository with one commit at `<tmp>/work`, and an empty session at
/// `<tmp>/claude/projects/<encode_project_dir(work)>/<SESSION>/subagents`.
///
/// `None` when the sandbox forbids `git init`, so the suite skips rather than
/// failing for a reason that is not about the code. Every later step asserts:
/// a fixture that silently degraded would make every test here a vacuous pass.
fn claude_fixture() -> Option<ClaudeFixture> {
    let tmp = TempDir::new().ok()?;
    // Canonical, because git reports canonical worktree paths.
    let base = std::fs::canonicalize(tmp.path()).ok()?;
    let work = base.join("work");
    std::fs::create_dir_all(&work).ok()?;
    if !git(&work, &["init", "--quiet"]) {
        return None;
    }
    assert!(git(&work, &["config", "user.email", "test@example.com"]));
    assert!(git(&work, &["config", "user.name", "Test User"]));
    assert!(git(&work, &["config", "commit.gpgsign", "false"]));
    std::fs::write(work.join("tracked.txt"), "one\n").expect("fixture write");
    assert!(git(&work, &["add", "tracked.txt"]), "git add");
    assert!(
        git(&work, &["commit", "-m", "initial", "--quiet"]),
        "git commit"
    );

    let root = base.join("claude");
    let subagents = root
        .join("projects")
        .join(encode_project_dir(work.to_str().expect("utf-8 tempdir")))
        .join(SESSION)
        .join("subagents");
    std::fs::create_dir_all(&subagents).expect("fixture subagents dir");
    Some(ClaudeFixture {
        _tmp: tmp,
        work,
        root,
        subagents,
    })
}

fn shown(value: &Option<gsd_meta_manager::text::Untrusted>) -> Option<String> {
    value.as_ref().map(|v| v.shown().to_string())
}

// ---------------------------------------------------------------------------
// The tracer
// ---------------------------------------------------------------------------

#[test]
fn a_live_claude_executor_is_enriched_end_to_end() {
    let Some(fx) = claude_fixture() else {
        return;
    };
    let wt = fx.add_agent_worktree(AGENT_ID, Some(1));
    let now = SystemTime::now();
    fx.write_meta(
        AGENT_ID,
        &json!({
            "agentType": "gsd-executor",
            "description": "Execute plan 13-02 of phase 13",
            "worktreePath": wt,
            "spawnDepth": 2,
            "requestShape": {"unknown": ["to", "this", "adapter"]},
        }),
    );
    fx.write_transcript(AGENT_ID, now, Duration::from_secs(10));

    let scan = fx.scan(now);
    assert_eq!(scan.rows.len(), 1, "one agent row: {:?}", scan.rows);
    let row = &scan.rows[0];
    assert_eq!(row.path, wt);
    assert_eq!(row.adapter, Some("claude-code"));
    assert_eq!(shown(&row.agent_type), Some("gsd-executor".to_string()));
    assert_eq!(
        shown(&row.description),
        Some("Execute plan 13-02 of phase 13".to_string())
    );
    assert_eq!(row.last_activity, Some(now - Duration::from_secs(10)));
    assert_eq!(row.liveness, AgentLiveness::Live);
}

#[test]
fn registered_adapters_includes_claude_code() {
    let names: Vec<&'static str> = registered_adapters().iter().map(|a| a.name()).collect();
    assert!(names.contains(&"claude-code"), "{names:?}");
}

// ---------------------------------------------------------------------------
// Lifecycle facts the core classifies (D-C08, D-A05, RESEARCH Pitfall 4)
// ---------------------------------------------------------------------------

/// A plain executor meta joined to `wt` by path.
fn executor_meta(wt: &Path) -> serde_json::Value {
    json!({
        "agentType": "gsd-executor",
        "description": "Execute plan 13-02 of phase 13",
        "worktreePath": wt,
        "spawnDepth": 2,
    })
}

#[test]
fn a_released_lock_with_a_stale_transcript_reads_finished() {
    let Some(fx) = claude_fixture() else {
        return;
    };
    let wt = fx.add_agent_worktree(AGENT_ID, None);
    let now = SystemTime::now();
    fx.write_meta(AGENT_ID, &executor_meta(&wt));
    fx.write_transcript(AGENT_ID, now, Duration::from_secs(121));

    let scan = fx.scan(now);
    let row = only_row(&scan);
    assert!(!row.locked, "the runtime released the lock");
    assert_eq!(row.adapter, Some("claude-code"));
    assert_eq!(row.liveness, AgentLiveness::Finished);
}

#[test]
fn a_locked_worktree_with_a_stale_transcript_reads_stalled_even_with_a_live_pid() {
    let Some(fx) = claude_fixture() else {
        return;
    };
    // The lock reason names THIS process — as alive as a pid can be. It is
    // the session's pid, shared by every agent, so it proves nothing (D-A05).
    let wt = fx.add_agent_worktree(AGENT_ID, Some(std::process::id()));
    let now = SystemTime::now();
    fx.write_meta(AGENT_ID, &executor_meta(&wt));
    fx.write_transcript(AGENT_ID, now, Duration::from_secs(601));

    let scan = fx.scan(now);
    let row = only_row(&scan);
    assert!(row.locked);
    assert_eq!(row.adapter, Some("claude-code"));
    assert_eq!(row.liveness, AgentLiveness::Stalled);
}

#[test]
fn a_stopped_by_user_meta_reads_ended() {
    let Some(fx) = claude_fixture() else {
        return;
    };
    let wt = fx.add_agent_worktree(AGENT_ID, Some(1));
    let now = SystemTime::now();
    let mut meta = executor_meta(&wt);
    meta["stoppedByUser"] = json!(true);
    fx.write_meta(AGENT_ID, &meta);
    fx.write_transcript(AGENT_ID, now, Duration::from_secs(5));

    let scan = fx.scan(now);
    assert_eq!(only_row(&scan).liveness, AgentLiveness::Ended);
}

#[test]
fn a_worktree_without_metadata_degrades_to_the_core_row() {
    let Some(fx) = claude_fixture() else {
        return;
    };
    let wt = fx.add_agent_worktree(AGENT_ID, Some(1));
    let now = SystemTime::now();

    let scan = fx.scan(now);
    let row = only_row(&scan);
    assert_eq!(row.path, wt);
    assert_eq!(row.adapter, None);
    assert_eq!(row.agent_type, None);
    assert_eq!(row.liveness, AgentLiveness::Unknown);
    assert_eq!(row.commits_ahead, Some(0), "git's facts are still there");
    assert_eq!(row.dirty, Some(0));
}

fn only_row(scan: &ProjectAgents) -> &gsd_meta_manager::agents::AgentRow {
    assert_eq!(scan.rows.len(), 1, "exactly one agent row: {:?}", scan.rows);
    &scan.rows[0]
}
