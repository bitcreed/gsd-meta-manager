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
use gsd_meta_manager::agents::{
    scan_project_with, scan_projects_guarded, AgentLiveness, ProjectAgents,
};
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

// ---------------------------------------------------------------------------
// The adapter seam under hostile adapters (D-A03, D-A07)
// ---------------------------------------------------------------------------

/// One claim: a named adapter reporting `agent_type` for every worktree index
/// `select` picks, plus any extra raw indexes (in range or not).
struct Claimer {
    name: &'static str,
    agent_type: &'static str,
    select: fn(&gsd_meta_manager::agents::worktrees::CoreWorktree) -> bool,
    extra_indexes: &'static [usize],
}

impl AgentAdapter for Claimer {
    fn name(&self) -> &'static str {
        self.name
    }

    fn enrich(&self, snap: &CoreSnapshot<'_>) -> AdapterReport {
        let facts = || Enrichment {
            agent_type: Some(Untrusted::from_untrusted_source(self.agent_type.into())),
            last_activity: Some(snap.now),
            ..Enrichment::default()
        };
        let mut per_worktree: Vec<(usize, Enrichment)> = snap
            .worktrees
            .iter()
            .enumerate()
            .filter(|(_, wt)| (self.select)(wt))
            .map(|(index, _)| (index, facts()))
            .collect();
        per_worktree.extend(self.extra_indexes.iter().map(|&index| (index, facts())));
        AdapterReport {
            per_worktree,
            worktreeless: Vec::new(),
        }
    }
}

/// An adapter that panics mid-scan.
struct Panicker;

impl AgentAdapter for Panicker {
    fn name(&self) -> &'static str {
        "panicker"
    }

    fn enrich(&self, _snap: &CoreSnapshot<'_>) -> AdapterReport {
        panic!("a hostile adapter panicked (expected by this test)");
    }
}

fn shown_type(scan: &ProjectAgents, index: usize) -> Option<String> {
    scan.rows[index]
        .agent_type
        .as_ref()
        .map(|t| t.shown().to_string())
}

#[test]
fn a_test_only_adapter_claims_a_worktree_outside_the_predicate() {
    let Some((_tmp, root)) = plain_repo() else {
        return;
    };
    add_worktree(&root, "feature", "../side");
    assert!(
        scan_bare(&root).rows.is_empty(),
        "a plain user worktree is not an agent row by itself"
    );

    let adapters: Vec<Box<dyn AgentAdapter>> = vec![Box::new(Claimer {
        name: "claimer",
        agent_type: "custom-runtime",
        select: |wt| wt.branch.as_ref().map(|b| b.as_raw_for_logic_only()) == Some("feature"),
        extra_indexes: &[],
    })];
    let scan = scan_project_with(&root, &adapters, SystemTime::now());
    assert_eq!(
        scan.rows.len(),
        1,
        "the claim makes it a row: {:?}",
        scan.rows
    );
    assert_eq!(scan.rows[0].path, root.parent().unwrap().join("side"));
    assert_eq!(scan.rows[0].adapter, Some("claimer"));
    assert_eq!(shown_type(&scan, 0), Some("custom-runtime".to_string()));
    assert_eq!(scan.rows[0].liveness, AgentLiveness::Live);
}

#[test]
fn the_first_adapter_to_claim_a_worktree_wins() {
    let Some((_tmp, root)) = agents_fixture() else {
        return;
    };
    let adapters: Vec<Box<dyn AgentAdapter>> = vec![
        Box::new(Claimer {
            name: "first",
            agent_type: "first-type",
            select: |_| true,
            extra_indexes: &[99, usize::MAX],
        }),
        Box::new(Claimer {
            name: "second",
            agent_type: "second-type",
            select: |_| true,
            extra_indexes: &[],
        }),
    ];
    let scan = scan_project_with(&root, &adapters, SystemTime::now());
    assert_eq!(scan.rows.len(), 1, "out-of-range indexes add no row");
    assert_eq!(scan.rows[0].adapter, Some("first"));
    assert_eq!(shown_type(&scan, 0), Some("first-type".to_string()));
}

#[test]
fn a_panicking_adapter_leaves_the_core_rows_intact() {
    let Some((_tmp, root)) = agents_fixture() else {
        return;
    };
    let alone: Vec<Box<dyn AgentAdapter>> = vec![Box::new(Panicker)];
    let scan = scan_project_with(&root, &alone, SystemTime::now());
    assert_eq!(scan.rows.len(), 1, "the core row survives the panic");
    assert_eq!(scan.rows[0].adapter, None);
    assert_eq!(scan.rows[0].liveness, AgentLiveness::Unknown);
    assert_eq!(
        scan.rows[0].commits_ahead,
        Some(0),
        "git facts still present"
    );

    let then_fake: Vec<Box<dyn AgentAdapter>> = vec![Box::new(Panicker), Box::new(FakeAdapter)];
    let scan = scan_project_with(&root, &then_fake, SystemTime::now());
    assert_eq!(scan.rows.len(), 1);
    assert_eq!(
        scan.rows[0].adapter,
        Some("fake"),
        "the next adapter still runs"
    );
    assert_eq!(scan.rows[0].liveness, AgentLiveness::Live);
}

#[test]
fn a_projects_scan_is_sorted_by_alias_and_isolates_failures() {
    let Some((_tmp_c, with_agent)) = agents_fixture() else {
        return;
    };
    let Some((_tmp_a, plain)) = plain_repo() else {
        return;
    };
    let missing = with_agent.parent().unwrap().join("does-not-exist");
    let projects = vec![
        ("charlie".to_string(), with_agent),
        ("bravo".to_string(), missing),
        ("alpha".to_string(), plain),
    ];

    let scans = scan_projects_guarded(&projects, SystemTime::now());
    let aliases: Vec<&str> = scans.iter().map(|(alias, _)| alias.as_str()).collect();
    assert_eq!(
        aliases,
        vec!["alpha", "bravo", "charlie"],
        "sorted by alias"
    );
    assert!(scans[0].1.rows.is_empty(), "alpha has no linked worktree");
    assert!(
        scans[1].1.rows.is_empty(),
        "a missing path scans to nothing"
    );
    assert!(scans[1].1.worktreeless.is_empty());
    assert_eq!(
        scans[2].1.rows.len(),
        1,
        "charlie's agent row is unaffected"
    );
}

// ---------------------------------------------------------------------------
// The zero-write guard (D-B01, D-B03)
// ---------------------------------------------------------------------------

/// Every forbidden token, split in two so this file's own text never contains
/// one whole — the technique `src/driver/reconcile.rs`'s own guard uses.
const FORBIDDEN_HALVES: &[(&str, &str)] = &[
    // Writes.
    ("fs::", "write"),
    ("File::", "create"),
    ("create_", "dir"),
    ("remove_", "file"),
    ("remove_", "dir"),
    ("ren", "ame("),
    ("Open", "Options"),
    ("set_", "modified"),
    ("set_", "permissions"),
    ("write_", "all("),
    // Process inspection.
    ("/pr", "oc"),
    // Process spawns.
    ("Command::", "new("),
    ("CommandWrap::", "with_new("),
    ("process_", "group("),
    ("process::", "Command"),
];

/// Every `.rs` file under `dir`, recursively.
fn rust_files(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            rust_files(&path, out);
        } else if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path);
        }
    }
}

#[test]
fn no_file_under_src_agents_writes_reads_proc_or_spawns() {
    let root = Path::new(concat!(env!("CARGO_MANIFEST_DIR"), "/src/agents"));
    let mut files = Vec::new();
    rust_files(root, &mut files);
    files.sort();
    assert!(
        files.len() >= 3,
        "the walk found too few files to be a real guard: {files:?}"
    );

    let tokens: Vec<String> = FORBIDDEN_HALVES
        .iter()
        .map(|(head, tail)| format!("{head}{tail}"))
        .collect();

    let mut offenders = Vec::new();
    for file in &files {
        let text = std::fs::read_to_string(file).expect("readable source");
        // Production text only: everything before the first `#[cfg(test)]`.
        for (number, line) in text
            .lines()
            .enumerate()
            .take_while(|(_, line)| line.trim() != "#[cfg(test)]")
        {
            if line.trim_start().starts_with("//") {
                continue;
            }
            if let Some(token) = tokens.iter().find(|t| line.contains(t.as_str())) {
                offenders.push(format!(
                    "{}:{}: `{token}` in {:?}",
                    file.display(),
                    number + 1,
                    line.trim()
                ));
            }
        }
    }
    assert!(
        offenders.is_empty(),
        "src/agents/ must not write, read the process table, or spawn (D-B01, \
         D-B03):\n{}",
        offenders.join("\n")
    );
}
