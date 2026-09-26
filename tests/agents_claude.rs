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
use gsd_meta_manager::agents::adapters::{
    registered_adapters, AdapterReport, AgentAdapter, CoreSnapshot,
};
use gsd_meta_manager::agents::waves::derive;
use gsd_meta_manager::agents::{
    scan_project_with, AgentLiveness, ProjectAgents, MAX_AGENT_AGE_SECS,
};
use gsd_meta_manager::state_reader::ProjectState;
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

// ---------------------------------------------------------------------------
// Orphans of an aborted run (25-07, CR-01, D-C08): past MAX_AGENT_AGE_SECS of
// inactivity an agent is over whatever its lock says. Its row is still listed
// (D-A03, D-C16); only the dashboard summary ignores it.
// ---------------------------------------------------------------------------

const THREE_DAYS: Duration = Duration::from_secs(3 * 86_400);

#[test]
fn a_released_lock_orphan_days_stale_reads_ended_and_is_not_active() {
    let Some(fx) = claude_fixture() else {
        return;
    };
    let wt = fx.add_agent_worktree(AGENT_ID, None);
    let now = SystemTime::now();
    fx.write_meta(AGENT_ID, &executor_meta(&wt));
    fx.write_transcript(AGENT_ID, now, THREE_DAYS);

    let scan = fx.scan(now);
    let row = only_row(&scan);
    assert!(!row.locked, "the runtime released the lock");
    assert_eq!(row.adapter, Some("claude-code"), "still enriched");
    assert_eq!(shown(&row.agent_type), Some("gsd-executor".to_string()));
    assert_eq!(row.commits_ahead, Some(0), "git's facts are still there");
    assert_eq!(row.dirty, Some(0));
    assert_eq!(row.liveness, AgentLiveness::Ended);

    let view = derive(&scan, &ProjectState::default());
    assert!(!view.is_active());
    assert_eq!(view.summary_forms(), Vec::<String>::new());
}

#[test]
fn a_locked_orphan_days_stale_reads_ended_not_stalled() {
    let Some(fx) = claude_fixture() else {
        return;
    };
    let wt = fx.add_agent_worktree(AGENT_ID, Some(std::process::id()));
    let now = SystemTime::now();
    fx.write_meta(AGENT_ID, &executor_meta(&wt));
    fx.write_transcript(AGENT_ID, now, THREE_DAYS);

    let scan = fx.scan(now);
    let row = only_row(&scan);
    assert!(row.locked, "a crashed session never released its lock");
    assert_eq!(row.adapter, Some("claude-code"));
    assert_eq!(row.commits_ahead, Some(0));
    assert_eq!(row.dirty, Some(0));
    assert_eq!(row.liveness, AgentLiveness::Ended);
    assert_eq!(
        derive(&scan, &ProjectState::default()).summary_forms(),
        Vec::<String>::new(),
        "no `1 stalled` for a days-old orphan"
    );

    // Inside the bound the crashed run still alerts. The exact second is
    // pinned by the unit test; mtime granularity makes a file test at the
    // boundary flaky.
    let Some(fx) = claude_fixture() else {
        return;
    };
    let wt = fx.add_agent_worktree(AGENT_ID, Some(std::process::id()));
    fx.write_meta(AGENT_ID, &executor_meta(&wt));
    fx.write_transcript(AGENT_ID, now, Duration::from_secs(MAX_AGENT_AGE_SECS - 60));
    let scan = fx.scan(now);
    assert_eq!(only_row(&scan).liveness, AgentLiveness::Stalled);
}

fn only_row(scan: &ProjectAgents) -> &gsd_meta_manager::agents::AgentRow {
    assert_eq!(scan.rows.len(), 1, "exactly one agent row: {:?}", scan.rows);
    &scan.rows[0]
}

// ---------------------------------------------------------------------------
// Live worktree-less subagents and the scan-cost bound (D-C07, D-C09 as
// amended by RESEARCH Pitfall 2)
// ---------------------------------------------------------------------------

/// A worktree-less meta: no `worktreePath`, no parent in any worktree.
fn loose_meta(agent_type: &str, description: &str) -> serde_json::Value {
    json!({
        "agentType": agent_type,
        "description": description,
        "spawnDepth": 1,
        "toolUseId": "toolu_0",
    })
}

/// The shown `(agentType, description)` of every worktree-less agent.
fn loose(scan: &ProjectAgents) -> Vec<(Option<String>, Option<String>)> {
    scan.worktreeless
        .iter()
        .map(|child| (shown(&child.agent_type), shown(&child.description)))
        .collect()
}

/// What the adapter itself reports, before the core filters anything.
fn adapter_report(fx: &ClaudeFixture, now: SystemTime) -> AdapterReport {
    let snap = CoreSnapshot {
        project_root: &fx.work,
        main_worktree: Some(&fx.work),
        worktrees: &[],
        now,
    };
    ClaudeCodeAdapter::new(fx.root.clone()).enrich(&snap)
}

#[test]
fn a_live_worktree_less_subagent_is_listed_and_a_stale_one_is_not() {
    let Some(fx) = claude_fixture() else {
        return;
    };
    let now = SystemTime::now();
    fx.write_meta(
        "aresearcher00000",
        &loose_meta("gsd-phase-researcher", "Research phase 26"),
    );
    fx.write_transcript("aresearcher00000", now, Duration::from_secs(30));
    fx.write_meta(
        "astale000000000",
        &loose_meta("gsd-planner", "Plan phase 26"),
    );
    fx.write_transcript("astale000000000", now, Duration::from_secs(121));
    // A stale transcript's meta is never read, so garbage there costs nothing.
    std::fs::write(
        fx.subagents.join("agent-astalegarbage00.meta.json"),
        b"\x00{not json",
    )
    .expect("garbage meta");
    fx.write_transcript("astalegarbage00", now, Duration::from_secs(121));

    let expected = vec![(
        Some("gsd-phase-researcher".to_string()),
        Some("Research phase 26".to_string()),
    )];
    assert_eq!(loose(&fx.scan(now)), expected);

    let report = adapter_report(&fx, now);
    assert_eq!(
        report.worktreeless.len(),
        1,
        "the adapter itself reports only the live agent, before any core filter: {:?}",
        report.worktreeless
    );
}

/// WR-05: two projects under one long parent share their encodings' 200-unit
/// prefix. A project with no Claude directory of its own must not adopt its
/// sibling's and report the sibling's live subagents as its own.
#[test]
fn a_long_path_project_never_adopts_a_siblings_claude_directory() {
    let Ok(tmp) = TempDir::new() else {
        return;
    };
    let root = tmp.path().join("claude");
    let parent = PathBuf::from(format!("/srv/{}", "p".repeat(220)));
    let with_dir = parent.join("beta");
    let without_dir = parent.join("alpha");
    let encoded = encode_project_dir(with_dir.to_str().expect("utf-8"));
    assert!(encoded.len() > 200, "the fixture is a long-path encoding");
    assert_eq!(
        encoded[..200],
        encode_project_dir(without_dir.to_str().expect("utf-8"))[..200],
        "the two projects share the 200-unit prefix"
    );
    let subagents = root
        .join("projects")
        .join(encoded)
        .join(SESSION)
        .join("subagents");
    std::fs::create_dir_all(&subagents).expect("fixture subagents dir");
    let now = SystemTime::now();
    write_meta_in(
        &subagents,
        "aresearcher00000",
        &loose_meta("gsd-phase-researcher", "Research phase 26"),
    );
    write_transcript_in(&subagents, "aresearcher00000", now - Duration::from_secs(5));

    let report_for = |project: &Path| {
        let snap = CoreSnapshot {
            project_root: project,
            main_worktree: Some(project),
            worktrees: &[],
            now,
        };
        ClaudeCodeAdapter::new(root.clone()).enrich(&snap)
    };
    assert_eq!(
        report_for(&with_dir).worktreeless.len(),
        1,
        "the owner still sees its live subagent"
    );
    assert!(
        report_for(&without_dir).worktreeless.is_empty(),
        "the sibling's subagent is not this project's"
    );
}

#[test]
fn agent_type_is_shown_verbatim_without_a_gsd_filter() {
    let Some(fx) = claude_fixture() else {
        return;
    };
    let now = SystemTime::now();
    fx.write_meta(
        "ageneral00000000",
        &loose_meta("general-purpose", "Survey crates"),
    );
    fx.write_transcript("ageneral00000000", now, Duration::from_secs(5));
    fx.write_meta(
        "aexplore00000000",
        &loose_meta("Explore", "Find the reader"),
    );
    fx.write_transcript("aexplore00000000", now, Duration::from_secs(5));

    let types: Vec<Option<String>> = loose(&fx.scan(now)).into_iter().map(|(t, _)| t).collect();
    assert_eq!(
        types,
        vec![
            Some("Explore".to_string()),
            Some("general-purpose".to_string())
        ],
        "every live agent, its type verbatim, sorted by type"
    );
}

#[test]
fn a_long_running_agent_in_an_old_subagents_dir_is_still_live() {
    let Some(fx) = claude_fixture() else {
        return;
    };
    let now = SystemTime::now();
    fx.write_meta(
        "alongrunner00000",
        &loose_meta("gsd-debugger", "Bisect the hang"),
    );
    fx.write_transcript("alongrunner00000", now, Duration::from_secs(5));
    // The directory's mtime moves only when an agent is spawned into it: this
    // agent was spawned ten minutes ago and is working right now.
    set_mtime(&fx.subagents, now - Duration::from_secs(600));

    assert_eq!(
        loose(&fx.scan(now)),
        vec![(
            Some("gsd-debugger".to_string()),
            Some("Bisect the hang".to_string())
        )],
        "a spawn clock ten minutes old must not hide a live agent"
    );
}

#[test]
fn a_day_old_session_is_skipped_for_worktree_less_agents_only() {
    let Some(fx) = claude_fixture() else {
        return;
    };
    let wt = fx.add_agent_worktree(AGENT_ID, Some(1));
    let now = SystemTime::now();
    let old_subagents = fx
        .subagents
        .parent()
        .and_then(Path::parent)
        .expect("project dir")
        .join("9a9a9a9a-2222-4333-8444-955566667777")
        .join("subagents");
    std::fs::create_dir_all(&old_subagents).expect("old session");
    write_meta_in(&old_subagents, AGENT_ID, &executor_meta(&wt));
    write_transcript_in(&old_subagents, AGENT_ID, now - Duration::from_secs(5));
    write_meta_in(
        &old_subagents,
        "aoldloose0000000",
        &loose_meta("general-purpose", "Old session"),
    );
    write_transcript_in(
        &old_subagents,
        "aoldloose0000000",
        now - Duration::from_secs(5),
    );
    set_mtime(
        &old_subagents,
        now - Duration::from_secs(MAX_AGENT_AGE_SECS + 60),
    );

    let scan = fx.scan(now);
    let row = only_row(&scan);
    assert_eq!(
        row.adapter,
        Some("claude-code"),
        "the per-id lookup is not bounded"
    );
    assert_eq!(shown(&row.agent_type), Some("gsd-executor".to_string()));
    assert_eq!(row.liveness, AgentLiveness::Live);
    assert!(
        scan.worktreeless.is_empty(),
        "a session with no spawn for a day is skipped for the worktree-less group: {:?}",
        scan.worktreeless
    );
}
