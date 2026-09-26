//! **This module performs zero disk writes** (D-B01).
//!
//! The agent observer answers one question — which GSD agents are running in a
//! project's worktrees right now, and how far each has got — while those agents
//! are running. Everything here is subordinate to not disturbing them:
//!
//! * **No write of any kind**: no file created, no directory made, no
//!   timestamp touched. Enforced by
//!   `tests/agents_scan.rs::no_file_under_src_agents_writes_reads_proc_or_spawns`,
//!   which walks this directory at runtime.
//! * **No `worktree prune`, no fetch, no gc, no ref write, no index refresh.**
//!   Every git call goes through `git_ops::git_read_raw`, which takes no
//!   optional lock (D-B02). A `prunable` worktree is reported, never pruned.
//! * **`src/agents` performs no process inspection itself, and no runtime
//!   invocation** (D-B03): state comes from git reads and file stats, never
//!   from running Claude, Codex or GSD. On Linux an optional process snapshot,
//!   taken by `crate::session_detector` and injected through
//!   `crate::session_detector::ProcessProbe`, contributes two facts: the death
//!   of a worktree lock's owner session, and a `codex` process working inside
//!   a worktree (quick 260926-06g, [`processes`]). Off Linux, or whenever the
//!   probe cannot answer (procfs not mounted, permission denied), liveness is
//!   mtime-only exactly as D-B03 requires.
//! * **A transcript's contents are never read**, only its mtime.
//! * **Never `$TMPDIR`, never `WAVE_WORKTREE_MANIFEST`** (D-A09): the manifest
//!   GSD writes there is a transient implementation detail of one orchestrator
//!   run, not an interface.
//! * **Never GSD's `waves.json`** (D-A10): wave membership comes from each
//!   PLAN.md's `wave:` frontmatter, read by the wave model.
//!
//! The split is two layers. [`worktrees`] is the runtime-agnostic core
//! (D-A01a): git alone, for any runtime. [`adapters`] is the seam (D-A01b)
//! through which a runtime reports facts about those worktrees. [`processes`]
//! turns an injected process snapshot into per-worktree evidence, as pure
//! logic over data. Liveness is classified here, once, by
//! [`classify_observed`] ([`classify_liveness`] is its mtime-only form).
//!
//! [`AgentLiveness`] is **not** the driver's `crate::driver::liveness::Liveness`
//! (which answers "is this pid our driver"). Never glob-import one beside the
//! other.

pub mod adapters;
pub mod fixers;
pub mod processes;
pub mod waves;
pub mod worktrees;

use std::collections::HashMap;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use adapters::{AdapterReport, AgentAdapter, ChildAgent, CoreSnapshot, Enrichment};
use processes::ProcessEvidence;
use worktrees::BranchPlan;

use crate::session_detector::{NoProcessProbe, ProcessProbe};
use crate::state_reader::disk_status;
use crate::state_reader::phase_num::PhaseNum;
use crate::text::Untrusted;

/// What the core concluded about one agent from its adapter's facts.
///
/// `Unknown` means "nothing was established" — no adapter data, or no activity
/// timestamp — and is never a synonym for dead or stalled (the doctrine
/// `src/driver/liveness.rs` states for its own `Unknown`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AgentLiveness {
    /// Activity within [`LIVE_SECS`].
    Live,
    /// Activity within [`IDLE_SECS`], but not within [`LIVE_SECS`].
    Idle,
    /// The runtime released the worktree lock and either a SUMMARY exists in
    /// the worktree or activity stopped more than [`LIVE_SECS`] ago.
    Finished,
    /// No activity for more than [`IDLE_SECS`] and nothing says it finished.
    Stalled,
    /// The runtime recorded the agent as ended, **or** its last activity is
    /// older than [`MAX_AGENT_AGE_SECS`]: an aborted or crashed run's leftover
    /// worktree, which is over whatever its lock says (CR-01). The row is still
    /// listed; only the dashboard summary ignores it.
    Ended,
    /// Nothing was established.
    #[default]
    Unknown,
}

impl AgentLiveness {
    /// Whether the agent is running: `Live` or `Idle`, nothing else (CR-01).
    ///
    /// A running agent is the only thing that switches the dashboard summary
    /// on, wins the first tier of the active-phase vote, selects the executor
    /// ladder, and is counted by the `N agents` form
    /// ([`waves::AgentView::summary_forms`]). It is also what switches the
    /// fixer estimate on ([`fixers::estimate`]).
    ///
    /// `Finished` is done work. It counts as a finished plan inside a ladder
    /// something running switched on, and as a fixer inside a running fix run,
    /// but never switches either on by itself: an aborted run's leftovers must
    /// not stand in for the project's real status.
    pub fn is_running(self) -> bool {
        matches!(self, AgentLiveness::Live | AgentLiveness::Idle)
    }
}

// The liveness thresholds, in one block (D-C08). Changing what "live" or
// "stalled" means is an edit here and nowhere else.

/// Activity at most this many seconds old is `Live`.
pub const LIVE_SECS: u64 = 120;
/// Activity at most this many seconds old (and older than [`LIVE_SECS`]) is `Idle`.
pub const IDLE_SECS: u64 = 600;
/// Activity older than this (a day) is over. The core classifies such an
/// agent `Ended` ([`classify_liveness`], CR-01), and the Claude adapter's
/// worktree-less pass skips sessions with no spawn this recent. The bound is on
/// **inactivity**: an agent that keeps writing its transcript is never aged
/// out, however long it runs.
pub const MAX_AGENT_AGE_SECS: u64 = 86_400;

/// Seconds from `then` to `now`; a `then` in the future is age 0.
fn age_secs(then: SystemTime, now: SystemTime) -> u64 {
    now.duration_since(then).map(|d| d.as_secs()).unwrap_or(0)
}

/// The one classifier every row and child goes through.
fn classify_facts(
    ended: bool,
    last_activity: Option<SystemTime>,
    lock_released: Option<bool>,
    summary_in_worktree: bool,
    evidence: ProcessEvidence,
    now: SystemTime,
) -> AgentLiveness {
    // A runtime process working in the worktree right now is the freshest
    // evidence there is, ahead of every recorded fact.
    if evidence == ProcessEvidence::RuntimeInside {
        return AgentLiveness::Live;
    }
    if ended {
        return AgentLiveness::Ended;
    }
    // The session that owned the lock is gone, so every agent of it is over,
    // whatever its transcript mtime, SUMMARY or lock says (the CR-01
    // precedent). A LIVE owner never reaches here as a verdict (D-A05).
    if evidence == ProcessEvidence::OwnerGone {
        return AgentLiveness::Ended;
    }
    let Some(last) = last_activity else {
        return AgentLiveness::Unknown;
    };
    let age = age_secs(last, now);
    // CR-01: an aborted or crashed run's leftover is over, whatever its lock
    // says. Strictly greater, like the LIVE/IDLE "at most" boundaries.
    if age > MAX_AGENT_AGE_SECS {
        return AgentLiveness::Ended;
    }
    if lock_released == Some(true) && (summary_in_worktree || age > LIVE_SECS) {
        return AgentLiveness::Finished;
    }
    if age <= LIVE_SECS {
        AgentLiveness::Live
    } else if age <= IDLE_SECS {
        AgentLiveness::Idle
    } else {
        AgentLiveness::Stalled
    }
}

/// Classify one agent from its adapter's facts (D-C08).
///
/// In order: no facts → `Unknown`; `ended` → `Ended`; no `last_activity` →
/// `Unknown`; activity older than [`MAX_AGENT_AGE_SECS`] → `Ended`, whatever
/// the lock says (CR-01); a released lock plus (a SUMMARY in the worktree, or
/// activity older than [`LIVE_SECS`]) → `Finished`; activity within [`LIVE_SECS`] →
/// `Live`; within [`IDLE_SECS`] → `Idle`; otherwise `Stalled`. A future mtime
/// is age 0.
///
/// This is [`classify_observed`] with no process evidence
/// ([`ProcessEvidence::Unknown`]): the mtime-only answer.
pub fn classify_liveness(
    facts: Option<&Enrichment>,
    summary_in_worktree: bool,
    now: SystemTime,
) -> AgentLiveness {
    classify_observed(facts, summary_in_worktree, ProcessEvidence::Unknown, now)
}

/// Classify one agent from its adapter's facts plus what the process table
/// said about its worktree ([`processes::worktree_evidence`], quick
/// 260926-06g).
///
/// The full rule order, with or without facts where noted:
///
/// 1. [`ProcessEvidence::RuntimeInside`] → `Live` (with or without facts).
/// 2. `ended` → `Ended`.
/// 3. [`ProcessEvidence::OwnerGone`] → `Ended` (with or without facts).
/// 4. No facts, or no `last_activity` → `Unknown`.
/// 5. Activity older than [`MAX_AGENT_AGE_SECS`] → `Ended` (CR-01).
/// 6. A released lock plus (a SUMMARY in the worktree, or activity older than
///    [`LIVE_SECS`]) → `Finished`.
/// 7. Otherwise `Live`, `Idle` or `Stalled` by age.
///
/// [`ProcessEvidence::Unknown`] and [`ProcessEvidence::OwnerAlive`] never
/// change the result (D-A05): it is exactly [`classify_liveness`]'s.
pub fn classify_observed(
    facts: Option<&Enrichment>,
    summary_in_worktree: bool,
    evidence: ProcessEvidence,
    now: SystemTime,
) -> AgentLiveness {
    let Some(facts) = facts else {
        return match evidence {
            ProcessEvidence::RuntimeInside => AgentLiveness::Live,
            ProcessEvidence::OwnerGone => AgentLiveness::Ended,
            ProcessEvidence::Unknown | ProcessEvidence::OwnerAlive => AgentLiveness::Unknown,
        };
    };
    classify_facts(
        facts.ended,
        facts.last_activity,
        facts.lock_released,
        summary_in_worktree,
        evidence,
        now,
    )
}

/// A child agent classified with the same rules; a child has no lock of its own.
fn classify_child(mut child: ChildAgent, now: SystemTime) -> ChildAgent {
    child.liveness = classify_facts(
        child.ended,
        child.last_activity,
        None,
        false,
        ProcessEvidence::Unknown,
        now,
    );
    child
}

/// One agent worktree, with git's counts and whatever an adapter added.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct AgentRow {
    /// The worktree's path, as git reported it.
    pub path: PathBuf,
    /// The branch short name.
    pub branch: Option<Untrusted>,
    /// The validated agent id from the path or branch.
    pub agent_id: Option<String>,
    /// Plan id and spawn time from a Codex-style branch (D-A06).
    pub branch_plan: Option<BranchPlan>,
    /// Plan id from GSD's ledger file (confirmation only).
    pub ledger_plan: Option<String>,
    /// Whether git reports the worktree locked.
    pub locked: bool,
    /// Whether git reports the worktree prunable (its directory is gone).
    pub prunable: bool,
    /// Commits the worktree's HEAD carries beyond the main worktree's HEAD;
    /// `None` renders as `?`.
    pub commits_ahead: Option<u32>,
    /// Paths `git status --porcelain` reports; `None` renders as `?`.
    pub dirty: Option<u32>,
    /// The name of the adapter whose facts this row carries.
    pub adapter: Option<&'static str>,
    /// The runtime's agent type.
    pub agent_type: Option<Untrusted>,
    /// The runtime's task description.
    pub description: Option<Untrusted>,
    /// The most recent observed activity.
    pub last_activity: Option<SystemTime>,
    /// The core's verdict, from [`classify_liveness`].
    pub liveness: AgentLiveness,
    /// Sub-agents inside this worktree, each classified.
    pub children: Vec<ChildAgent>,
    /// The plan this agent executes, from [`waves::attribute`] at scan time;
    /// `None` when no tier yielded a valid plan id (quick tasks, fixers).
    pub plan: Option<waves::PlanRef>,
    /// Whether this worktree's own `.planning/` holds this plan's SUMMARY —
    /// finished, but not yet merged into main (RESEARCH Pitfall 3).
    pub summary_in_worktree: bool,
}

/// One project's agents, as of one scan.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ProjectAgents {
    /// Agent worktree rows, sorted by path.
    pub rows: Vec<AgentRow>,
    /// Live agents with no worktree of their own, sorted by type then description.
    pub worktreeless: Vec<ChildAgent>,
    /// The main worktree's path.
    pub main_worktree: Option<PathBuf>,
    /// The base commit counts are measured from.
    pub base_sha: Option<String>,
    /// When the scan ran.
    pub scanned_at: Option<SystemTime>,
    /// The code-review fix-run estimate ([`fixers::estimate`]); `None` unless
    /// a running (`Live` or `Idle`) unattributed `gsd-code-fixer` row exists.
    pub fixer_estimate: Option<fixers::FixerEstimate>,
}

/// Run one adapter, turning a panic into an empty report.
fn run_adapter(
    adapter: &dyn AgentAdapter,
    snap: &CoreSnapshot<'_>,
) -> (&'static str, AdapterReport) {
    let name = catch_unwind(AssertUnwindSafe(|| adapter.name())).unwrap_or("<unnamed adapter>");
    match catch_unwind(AssertUnwindSafe(|| adapter.enrich(snap))) {
        Ok(report) => (name, report),
        Err(_) => {
            tracing::warn!(
                adapter = name,
                "agent adapter panicked; its facts are dropped for this scan"
            );
            (name, AdapterReport::default())
        }
    }
}

/// Whether `phase_dir` (a worktree's copy of a phase directory main also has)
/// holds `plan`'s SUMMARY — the plan finished in that worktree, unmerged.
///
/// Only entry NAMES are read, never contents. A name matches when it ends in
/// `-SUMMARY.md` and its stem's plan index equals `plan` (`13-2-SUMMARY.md`
/// matches 13-02); FIX and GAPCLOSURE summaries are never plan partners, the
/// same exclusion the main-worktree pairing makes. An unreadable or missing
/// directory is `false`.
fn worktree_holds_summary(phase_dir: &Path, plan: &waves::PlanRef) -> bool {
    let Ok(entries) = std::fs::read_dir(phase_dir) else {
        return false;
    };
    entries.flatten().any(|entry| {
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            return false;
        };
        if name.contains("-FIX-") || name.ends_with("-GAPCLOSURE-SUMMARY.md") {
            return false;
        }
        name.strip_suffix("-SUMMARY.md")
            .and_then(disk_status::plan_index)
            .is_some_and(|(phase, number)| phase == plan.phase && number == plan.plan)
    })
}

/// Scan one project with an explicit adapter list.
///
/// Core scan → [`CoreSnapshot`] → each adapter's `enrich` (panics caught) →
/// claims accepted in registration order, first claim wins, out-of-range
/// indexes ignored → a row for every worktree matching the agent predicate or
/// claimed by an adapter → git counts per row (skipped for a `prunable`
/// worktree) → plan attribution per row ([`waves::attribute`]; one `git log`
/// only for a row the free tiers leave unattributed) → one worktree SUMMARY
/// check per attributed row → liveness per row and child → only `Live`
/// worktree-less agents kept (D-C07).
///
/// Public so an adapter's own tests — and the D-A07 proof in
/// `tests/agents_scan.rs` — can feed a test-only adapter through the real core.
/// A non-git project still runs its adapters, over an empty worktree list.
///
/// No process evidence: this is [`scan_project_with_probe`] with
/// [`NoProcessProbe`], the mtime-only scan every non-Linux build runs.
pub fn scan_project_with(
    project_root: &Path,
    adapters: &[Box<dyn AgentAdapter>],
    now: SystemTime,
) -> ProjectAgents {
    scan_project_with_probe(project_root, adapters, &NoProcessProbe, now)
}

/// [`scan_project_with`], plus one [`ProcessEvidence`] per row from `probe`
/// ([`processes::worktree_evidence`]), computed before the row's liveness is
/// classified by [`classify_observed`] (quick 260926-06g).
///
/// The roots a live lock owner may legitimately work in are built once per
/// project: `project_root`, its canonical form, the main worktree and every
/// worktree git reported. `probe` is consulted only for agent rows, so a
/// project with none costs it nothing.
pub fn scan_project_with_probe(
    project_root: &Path,
    adapters: &[Box<dyn AgentAdapter>],
    probe: &dyn ProcessProbe,
    now: SystemTime,
) -> ProjectAgents {
    let core = worktrees::scan_worktrees(project_root);
    let mut roots: Vec<PathBuf> = vec![project_root.to_path_buf()];
    if let Ok(canonical) = std::fs::canonicalize(project_root) {
        roots.push(canonical);
    }
    roots.extend(core.main_worktree.iter().cloned());
    roots.extend(core.worktrees.iter().map(|wt| wt.path.clone()));
    let snap = CoreSnapshot {
        project_root,
        main_worktree: core.main_worktree.as_deref(),
        worktrees: &core.worktrees,
        now,
    };

    let mut claims: Vec<Option<(&'static str, Enrichment)>> = vec![None; core.worktrees.len()];
    let mut worktreeless = Vec::new();
    for adapter in adapters {
        let (name, report) = run_adapter(adapter.as_ref(), &snap);
        for (index, facts) in report.per_worktree {
            if let Some(slot) = claims.get_mut(index) {
                if slot.is_none() {
                    *slot = Some((name, facts));
                }
            }
        }
        worktreeless.extend(report.worktreeless);
    }

    let base_sha = core.base_sha.as_deref();
    let planning = project_root.join(".planning");
    // Phase → its directory relative to `<project>/.planning`, or `None` when
    // main has no directory for it. `find_phase_dir` runs at most once per
    // distinct phase per scan.
    let mut phase_dirs: HashMap<PhaseNum, Option<PathBuf>> = HashMap::new();
    let mut rows: Vec<AgentRow> = core
        .worktrees
        .iter()
        .zip(claims)
        .filter(|(wt, claim)| wt.agent_pattern || claim.is_some())
        .map(|(wt, claim)| {
            let (commits_ahead, dirty) = worktrees::worktree_counts(wt, base_sha);
            let facts = claim.as_ref().map(|(_, facts)| facts);
            // Attribution BEFORE classification: the plan is what the SUMMARY
            // check (and so `Finished`) is keyed on.
            let description = facts
                .and_then(|f| f.description.as_ref())
                .map(Untrusted::as_raw_for_logic_only);
            let branch_plan = wt.branch_plan.as_ref().map(|bp| bp.plan.as_str());
            // Tiers 1-3 cost nothing; only a row they leave unattributed pays
            // for one `git log` to read its commit scopes (tier 4) before the
            // ledger is consulted (tier 5).
            let plan = waves::attribute(description, branch_plan, None, None).or_else(|| {
                let scope = base_sha
                    .map(|base| worktrees::commit_subjects(wt, base))
                    .and_then(|subjects| waves::commit_scope_plan(&subjects));
                waves::attribute(
                    description,
                    branch_plan,
                    scope.as_deref(),
                    wt.ledger_plan.as_deref(),
                )
            });
            // One `read_dir` per attributed row; the phase directory is
            // resolved once per distinct phase, from MAIN's own listing.
            let summary_in_worktree = plan.as_ref().is_some_and(|plan| {
                let relative = phase_dirs.entry(plan.phase.clone()).or_insert_with(|| {
                    disk_status::find_phase_dir(&planning, &plan.phase.padded())
                        .and_then(|dir| dir.strip_prefix(&planning).ok().map(Path::to_path_buf))
                });
                !wt.prunable
                    && relative.as_deref().is_some_and(|relative| {
                        worktree_holds_summary(&wt.path.join(".planning").join(relative), plan)
                    })
            });
            let evidence = processes::worktree_evidence(wt, probe, &roots);
            let liveness = classify_observed(facts, summary_in_worktree, evidence, now);
            let mut row = AgentRow {
                plan,
                summary_in_worktree,
                path: wt.path.clone(),
                branch: wt.branch.clone(),
                agent_id: wt.agent_id.clone(),
                branch_plan: wt.branch_plan.clone(),
                ledger_plan: wt.ledger_plan.clone(),
                locked: wt.locked.is_some(),
                prunable: wt.prunable,
                commits_ahead,
                dirty,
                liveness,
                ..AgentRow::default()
            };
            if let Some((name, facts)) = claim {
                row.adapter = Some(name);
                row.agent_type = facts.agent_type;
                row.description = facts.description;
                row.last_activity = facts.last_activity;
                row.children = facts
                    .children
                    .into_iter()
                    .map(|child| classify_child(child, now))
                    .collect();
            }
            row
        })
        .collect();
    rows.sort_by(|a, b| a.path.cmp(&b.path));

    let raw = |value: &Option<Untrusted>| {
        value
            .as_ref()
            .map(|v| v.as_raw_for_logic_only().to_string())
    };
    let mut worktreeless: Vec<ChildAgent> = worktreeless
        .into_iter()
        .map(|child| classify_child(child, now))
        .filter(|child| child.liveness == AgentLiveness::Live)
        .collect();
    worktreeless.sort_by(|a, b| {
        (raw(&a.agent_type), raw(&a.description)).cmp(&(raw(&b.agent_type), raw(&b.description)))
    });

    let fixer_estimate = fixers::estimate(
        project_root,
        core.main_worktree.as_deref(),
        core.base_sha.as_deref(),
        &rows,
    );

    ProjectAgents {
        rows,
        worktreeless,
        main_worktree: core.main_worktree.clone(),
        base_sha: core.base_sha.clone(),
        scanned_at: Some(now),
        fixer_estimate,
    }
}

/// Scan every registered project with the registered adapters, one project's
/// failure never costing another project its rows (D-A03).
///
/// [`registered_adapters`](adapters::registered_adapters) is called once per
/// call. Each project is scanned inside `catch_unwind`: a panic yields
/// [`ProjectAgents::default()`] for that alias plus a `tracing::warn!` naming
/// the alias only. The result is sorted by alias, so two scans of the same
/// state compare equal.
///
/// **What the unwinding guard does and does not buy (Pitfall 6).** It keeps
/// the scan — and therefore its caller's in-flight flag, which is cleared only
/// when a result arrives — alive through a panicking adapter. It cannot undo
/// the process-wide panic hook `ratatui::init` installs, which restores the
/// terminal *before* unwinding starts: a caught panic can still leave the TUI
/// visibly disturbed. So adapters must be panic-free by construction — failure
/// as data, no `unwrap` on external input — and this guard is defence in
/// depth, not a licence.
///
/// `probe` is the one process snapshot for this whole scan (quick
/// 260926-06g): the caller builds it with
/// `crate::session_detector::process_probe()` inside the same blocking task,
/// and every project's rows consult it through [`scan_project_with_probe`]. Its
/// memo means one read per distinct lock pid per scan. [`NoProcessProbe`]
/// gives the mtime-only scan.
pub fn scan_projects_guarded(
    projects: &[(String, PathBuf)],
    probe: &dyn ProcessProbe,
    now: SystemTime,
) -> Vec<(String, ProjectAgents)> {
    let adapters = catch_unwind(adapters::registered_adapters).unwrap_or_else(|_| {
        tracing::warn!("constructing the agent adapters panicked; scanning with none");
        Vec::new()
    });
    let mut scans: Vec<(String, ProjectAgents)> = projects
        .iter()
        .map(|(alias, path)| {
            let scan = catch_unwind(AssertUnwindSafe(|| {
                scan_project_with_probe(path, &adapters, probe, now)
            }))
            .unwrap_or_else(|_| {
                    tracing::warn!(alias = %alias, "agent scan panicked; this project reports no agents this scan");
                    ProjectAgents::default()
                });
            (alias.clone(), scan)
        })
        .collect();
    scans.sort_by(|a, b| a.0.cmp(&b.0));
    scans
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    fn at(now: SystemTime, age_secs: i64) -> Option<SystemTime> {
        Some(if age_secs >= 0 {
            now - Duration::from_secs(age_secs.unsigned_abs())
        } else {
            now + Duration::from_secs(age_secs.unsigned_abs())
        })
    }

    fn facts(now: SystemTime, age_secs: i64, lock_released: Option<bool>) -> Enrichment {
        Enrichment {
            last_activity: at(now, age_secs),
            lock_released,
            ..Enrichment::default()
        }
    }

    #[test]
    fn liveness_thresholds_at_and_one_second_past_each_boundary() {
        let now = SystemTime::now();
        let c = |age| classify_liveness(Some(&facts(now, age, None)), false, now);
        assert_eq!(c(0), AgentLiveness::Live);
        assert_eq!(c(120), AgentLiveness::Live);
        assert_eq!(c(121), AgentLiveness::Idle);
        assert_eq!(c(600), AgentLiveness::Idle);
        assert_eq!(c(601), AgentLiveness::Stalled);
        assert_eq!(LIVE_SECS, 120);
        assert_eq!(IDLE_SECS, 600);
    }

    #[test]
    fn a_future_mtime_is_age_zero() {
        let now = SystemTime::now();
        assert_eq!(
            classify_liveness(Some(&facts(now, -30, None)), false, now),
            AgentLiveness::Live
        );
    }

    #[test]
    fn ended_wins_over_every_other_fact() {
        let now = SystemTime::now();
        let mut f = facts(now, 0, Some(true));
        f.ended = true;
        assert_eq!(classify_liveness(Some(&f), true, now), AgentLiveness::Ended);
    }

    #[test]
    fn finished_needs_a_released_lock_and_a_summary_or_staleness() {
        let now = SystemTime::now();
        let c = |age, lock, summary| classify_liveness(Some(&facts(now, age, lock)), summary, now);
        assert_eq!(
            c(30, Some(true), false),
            AgentLiveness::Live,
            "fresh, no SUMMARY"
        );
        assert_eq!(
            c(30, Some(true), true),
            AgentLiveness::Finished,
            "SUMMARY in worktree"
        );
        assert_eq!(
            c(121, Some(true), false),
            AgentLiveness::Finished,
            "stale after release"
        );
        assert_eq!(
            c(601, Some(false), false),
            AgentLiveness::Stalled,
            "lock held"
        );
        assert_eq!(c(601, None, false), AgentLiveness::Stalled, "lock unknown");
    }

    /// CR-01 (D-C08): an aborted or crashed run's leftover worktree is over once
    /// its last activity is older than [`MAX_AGENT_AGE_SECS`], whatever its lock
    /// says. At exactly the bound it still reads `Finished` or `Stalled`.
    #[test]
    fn an_agent_silent_past_the_age_bound_reads_ended() {
        assert_eq!(MAX_AGENT_AGE_SECS, 86_400);
        let now = SystemTime::now();
        let bound = i64::try_from(MAX_AGENT_AGE_SECS).expect("the bound fits an i64");
        let c = |age, lock, summary| classify_liveness(Some(&facts(now, age, lock)), summary, now);

        assert_eq!(c(bound, Some(true), false), AgentLiveness::Finished);
        assert_eq!(c(bound, Some(true), true), AgentLiveness::Finished);
        assert_eq!(c(bound, Some(false), false), AgentLiveness::Stalled);
        assert_eq!(c(bound, None, false), AgentLiveness::Stalled);

        let past = bound + 1;
        assert_eq!(c(past, Some(true), false), AgentLiveness::Ended, "released");
        assert_eq!(
            c(past, Some(true), true),
            AgentLiveness::Ended,
            "released, SUMMARY in worktree"
        );
        assert_eq!(c(past, Some(false), false), AgentLiveness::Ended, "held");
        assert_eq!(c(past, None, false), AgentLiveness::Ended, "lock unknown");

        let child = ChildAgent {
            last_activity: at(now, past),
            ..ChildAgent::default()
        };
        assert_eq!(classify_child(child, now).liveness, AgentLiveness::Ended);
    }

    /// CR-01: only a `Live` or `Idle` agent is running.
    #[test]
    fn only_live_and_idle_are_running() {
        use AgentLiveness::*;
        for (liveness, running) in [
            (Live, true),
            (Idle, true),
            (Finished, false),
            (Stalled, false),
            (Ended, false),
            (Unknown, false),
        ] {
            assert_eq!(liveness.is_running(), running, "{liveness:?}");
        }
    }

    /// Every (facts, summary) combination of the process-evidence grid.
    fn evidence_grid(now: SystemTime) -> Vec<(Option<Enrichment>, bool)> {
        let mut grid = Vec::new();
        for summary in [false, true] {
            grid.push((None, summary));
            grid.push((Some(Enrichment::default()), summary));
            for age in [0, 120, 121, 600, 601, 86_400, 86_401] {
                for lock in [None, Some(false), Some(true)] {
                    grid.push((Some(facts(now, age, lock)), summary));
                    let mut ended = facts(now, age, lock);
                    ended.ended = true;
                    grid.push((Some(ended), summary));
                }
            }
        }
        grid
    }

    /// D-A05: an unknown or live owner never changes the mtime-only answer.
    #[test]
    fn unknown_and_owner_alive_equal_the_mtime_only_classifier() {
        let now = SystemTime::now();
        for (f, summary) in evidence_grid(now) {
            let expected = classify_liveness(f.as_ref(), summary, now);
            for evidence in [ProcessEvidence::Unknown, ProcessEvidence::OwnerAlive] {
                assert_eq!(
                    classify_observed(f.as_ref(), summary, evidence, now),
                    expected,
                    "{evidence:?} {f:?} summary={summary}"
                );
            }
        }
    }

    /// A gone owner ends every agent of its session, whatever the mtime says.
    #[test]
    fn owner_gone_is_ended_for_every_combination() {
        let now = SystemTime::now();
        for (f, summary) in evidence_grid(now) {
            assert_eq!(
                classify_observed(f.as_ref(), summary, ProcessEvidence::OwnerGone, now),
                AgentLiveness::Ended,
                "{f:?} summary={summary}"
            );
        }
    }

    /// A process working in the worktree right now reads `Live`, whatever the
    /// recorded facts say.
    #[test]
    fn runtime_inside_is_live_for_every_combination() {
        let now = SystemTime::now();
        for (f, summary) in evidence_grid(now) {
            assert_eq!(
                classify_observed(f.as_ref(), summary, ProcessEvidence::RuntimeInside, now),
                AgentLiveness::Live,
                "{f:?} summary={summary}"
            );
        }
        let stale = facts(now, 86_401, None);
        let mut ended = facts(now, 0, None);
        ended.ended = true;
        let released = facts(now, 30, Some(true));
        for (name, f, summary) in [
            ("no facts", None, false),
            ("stale past the age bound", Some(&stale), false),
            ("ended", Some(&ended), false),
            ("released lock with a SUMMARY", Some(&released), true),
        ] {
            assert_eq!(
                classify_observed(f, summary, ProcessEvidence::RuntimeInside, now),
                AgentLiveness::Live,
                "{name}"
            );
        }
    }

    #[test]
    fn no_adapter_data_is_unknown_never_stalled() {
        let now = SystemTime::now();
        assert_eq!(classify_liveness(None, false, now), AgentLiveness::Unknown);
        assert_eq!(
            classify_liveness(Some(&Enrichment::default()), false, now),
            AgentLiveness::Unknown,
            "facts without a last_activity establish nothing"
        );
        assert_eq!(AgentLiveness::default(), AgentLiveness::Unknown);
    }
}
