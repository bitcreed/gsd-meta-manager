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

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{Duration, SystemTime};

use common::git;
use gsd_meta_manager::agents::adapters::{AdapterReport, AgentAdapter, CoreSnapshot, Enrichment};
use gsd_meta_manager::agents::worktrees::BranchPlan;
use gsd_meta_manager::agents::{scan_project_with, AgentLiveness, ProjectAgents};
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

// ---------------------------------------------------------------------------
// Which worktrees are agent rows, and what degraded ones look like
// ---------------------------------------------------------------------------

/// Scan with no adapter: what git alone yields.
fn scan_bare(root: &Path) -> ProjectAgents {
    scan_project_with(root, &[], SystemTime::now())
}

/// `git -C dir <args>` stdout, trimmed; panics on failure.
fn git_out(dir: &Path, args: &[&str]) -> String {
    let out = Command::new("git")
        .arg("-C")
        .arg(dir)
        .args(args)
        .output()
        .expect("git is runnable");
    assert!(out.status.success(), "git {args:?} failed");
    String::from_utf8_lossy(&out.stdout).trim().to_string()
}

#[test]
fn the_main_worktree_and_a_plain_user_worktree_are_not_agent_rows() {
    let Some((_tmp, root)) = agents_fixture() else {
        return;
    };
    add_worktree(&root, "feature", "../side");
    let side = root.parent().unwrap().join("side");
    assert!(side.is_dir(), "the user worktree exists");

    let scan = scan_bare(&root);
    let paths: Vec<&PathBuf> = scan.rows.iter().map(|r| &r.path).collect();
    assert_eq!(
        paths,
        vec![&agent_worktree(&root)],
        "only the agent worktree"
    );
    assert_eq!(scan.main_worktree.as_deref(), Some(root.as_path()));
}

#[test]
fn a_codex_style_branch_yields_its_plan_and_spawn_time() {
    let Some((_tmp, root)) = plain_repo() else {
        return;
    };
    add_worktree(
        &root,
        "worktree-agent-p13-02-1790386422",
        ".claude/worktrees/agent-p13-02-1790386422",
    );

    let scan = scan_bare(&root);
    assert_eq!(scan.rows.len(), 1, "{:?}", scan.rows);
    let row = &scan.rows[0];
    assert_eq!(
        row.branch_plan,
        Some(BranchPlan {
            plan: "13-02".to_string(),
            spawned_unix: 1_790_386_422,
        })
    );
    assert_eq!(row.agent_id.as_deref(), Some("p13-02-1790386422"));
}

#[test]
fn a_worktree_deleted_from_disk_keeps_its_row_with_unknown_counts() {
    let Some((_tmp, root)) = agents_fixture() else {
        return;
    };
    let wt = agent_worktree(&root);
    std::fs::remove_dir_all(&wt).expect("delete the worktree directory, keep the admin dir");

    let scan = scan_bare(&root);
    assert_eq!(scan.rows.len(), 1, "the row survives: {:?}", scan.rows);
    let row = &scan.rows[0];
    assert_eq!(row.path, wt);
    assert!(row.prunable, "git reports it prunable");
    assert_eq!(row.commits_ahead, None);
    assert_eq!(row.dirty, None);
    assert!(
        root.join(".git").join("worktrees").is_dir(),
        "and nothing was pruned"
    );
}

#[test]
fn a_ledger_file_confirms_the_plan() {
    let Some((_tmp, root)) = agents_fixture() else {
        return;
    };
    add_worktree(
        &root,
        "worktree-agent-bbbbbbbbbbbbbbbb",
        ".claude/worktrees/agent-bbbbbbbbbbbbbbbb",
    );

    let admin = |wt: &Path| PathBuf::from(git_out(wt, &["rev-parse", "--absolute-git-dir"]));
    let first = agent_worktree(&root);
    let second = root.join(".claude/worktrees/agent-bbbbbbbbbbbbbbbb");
    std::fs::write(
        admin(&first).join("gsd-plan-head-before-13-13"),
        "deadbeef\n",
    )
    .unwrap();
    std::fs::write(
        admin(&second).join("gsd-plan-head-before-13x"),
        "deadbeef\n",
    )
    .unwrap();

    let scan = scan_bare(&root);
    let ledger = |path: &Path| {
        scan.rows
            .iter()
            .find(|r| r.path == path)
            .map(|r| r.ledger_plan.clone())
            .expect("row present")
    };
    assert_eq!(ledger(&first), Some("13-13".to_string()));
    assert_eq!(
        ledger(&second),
        None,
        "a ledger name failing the plan-id rule"
    );
}

#[test]
fn a_non_git_project_scans_to_no_worktree_rows() {
    let tmp = TempDir::new().unwrap();
    let scan = scan_bare(tmp.path());
    assert!(scan.rows.is_empty());
    assert!(scan.worktreeless.is_empty());
    assert_eq!(scan.main_worktree, None);

    // A repository with no linked worktree has no `.git/worktrees`, so the
    // listing is skipped outright (D-C09 cost bound). The base is resolved only
    // from that listing, so its absence is the observable proof it never ran.
    let Some((_tmp, root)) = plain_repo() else {
        return;
    };
    assert!(!root.join(".git").join("worktrees").exists());
    let scan = scan_bare(&root);
    assert!(scan.rows.is_empty());
    assert_eq!(scan.base_sha, None, "worktree list was not run");
}

#[test]
fn an_agent_worktree_just_spawned_reads_zero_commits_and_zero_dirty() {
    let Some((_tmp, root)) = agents_fixture() else {
        return;
    };
    let scan = scan_bare(&root);
    assert_eq!(scan.rows.len(), 1);
    let row = &scan.rows[0];
    assert_eq!(row.commits_ahead, Some(0));
    assert_eq!(row.dirty, Some(0));
    assert_eq!(
        row.liveness,
        AgentLiveness::Unknown,
        "zero counts are not a liveness verdict; with no adapter nothing is established"
    );
}

// ---------------------------------------------------------------------------
// Non-intrusion (RESEARCH Pitfall 8, D-B02)
// ---------------------------------------------------------------------------

/// The index file's content digest and mtime.
///
/// A digest, not a length: `tests/driver_dry_run.rs` records that the most
/// likely unwanted write — an opportunistic index refresh — leaves the file
/// exactly the same size, so a length-only check would miss it.
fn index_fingerprint(index: &Path) -> (u64, SystemTime) {
    let bytes = std::fs::read(index).expect("the index exists");
    let mut hasher = DefaultHasher::new();
    bytes.hash(&mut hasher);
    let mtime = std::fs::metadata(index)
        .and_then(|m| m.modified())
        .expect("index mtime");
    (hasher.finish(), mtime)
}

#[test]
fn scanning_a_live_agent_worktree_never_touches_its_index() {
    let Some((_tmp, root)) = agents_fixture() else {
        return;
    };
    let wt = agent_worktree(&root);
    let index = {
        let reported = PathBuf::from(git_out(&wt, &["rev-parse", "--git-path", "index"]));
        if reported.is_absolute() {
            reported
        } else {
            wt.join(reported)
        }
    };
    let lock = index.with_extension("lock");

    // Make `tracked.txt` stat-dirty but content-identical: exactly the entry a
    // refreshing read would rewrite the index for.
    let tracked = wt.join("tracked.txt");
    let bytes = std::fs::read(&tracked).unwrap();
    std::fs::write(&tracked, &bytes).unwrap();
    let past = SystemTime::now() - Duration::from_secs(100);
    std::fs::OpenOptions::new()
        .write(true)
        .open(&tracked)
        .and_then(|f| f.set_modified(past))
        .expect("set the tracked file's mtime into the past");

    let before = index_fingerprint(&index);
    let scan = scan_bare(&root);
    assert_eq!(scan.rows.len(), 1);
    assert_eq!(scan.rows[0].dirty, Some(0), "content-identical is clean");
    assert_eq!(
        index_fingerprint(&index),
        before,
        "the scan rewrote the index"
    );
    assert!(!lock.exists(), "the scan left an index.lock behind");

    // Positive control: a plain `git status` (no lock flag) DOES refresh this
    // index, so the fixture is able to detect the write the scan must not make.
    let status = Command::new("git")
        .arg("-C")
        .arg(&wt)
        .args(["status", "--porcelain"])
        .env_remove("GIT_OPTIONAL_LOCKS")
        .output()
        .expect("git status runs");
    assert!(status.status.success());
    assert_ne!(
        index_fingerprint(&index),
        before,
        "the control did not refresh the index, so this fixture proves nothing"
    );
}
