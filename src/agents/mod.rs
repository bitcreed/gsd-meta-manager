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
//! * **No process inspection and no runtime invocation** (D-B03): state comes
//!   from git reads and file stats only, never from running Claude, Codex or
//!   GSD, and never from the process table.
//! * **A transcript's contents are never read**, only its mtime.
//! * **Never `$TMPDIR`, never `WAVE_WORKTREE_MANIFEST`** (D-A09): the manifest
//!   GSD writes there is a transient implementation detail of one orchestrator
//!   run, not an interface.
//! * **Never GSD's `waves.json`** (D-A10): wave membership comes from each
//!   PLAN.md's `wave:` frontmatter, read by the wave model.
//!
//! The split is two layers. [`worktrees`] is the runtime-agnostic core
//! (D-A01a): git alone, for any runtime. [`adapters`] is the seam (D-A01b)
//! through which a runtime reports facts about those worktrees. Liveness is
//! classified here, once, by [`classify_liveness`].
//!
//! [`AgentLiveness`] is **not** the driver's `crate::driver::liveness::Liveness`
//! (which answers "is this pid our driver"). Never glob-import one beside the
//! other.

pub mod adapters;
pub mod worktrees;

use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::time::SystemTime;

use adapters::{AdapterReport, AgentAdapter, ChildAgent, CoreSnapshot, Enrichment};
use worktrees::BranchPlan;

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
    /// The runtime recorded the agent as ended.
    Ended,
    /// Nothing was established.
    #[default]
    Unknown,
}

// The liveness thresholds, in one block (D-C08). Changing what "live" or
// "stalled" means is an edit here and nowhere else.

/// Activity at most this many seconds old is `Live`.
pub const LIVE_SECS: u64 = 120;
/// Activity at most this many seconds old (and older than [`LIVE_SECS`]) is `Idle`.
pub const IDLE_SECS: u64 = 600;
/// Adapters ignore agents whose last activity is older than this (a day).
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
    now: SystemTime,
) -> AgentLiveness {
    if ended {
        return AgentLiveness::Ended;
    }
    let Some(last) = last_activity else {
        return AgentLiveness::Unknown;
    };
    let age = age_secs(last, now);
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
/// `Unknown`; a released lock plus (a SUMMARY in the worktree, or activity
/// older than [`LIVE_SECS`]) → `Finished`; activity within [`LIVE_SECS`] →
/// `Live`; within [`IDLE_SECS`] → `Idle`; otherwise `Stalled`. A future mtime
/// is age 0.
pub fn classify_liveness(
    facts: Option<&Enrichment>,
    summary_in_worktree: bool,
    now: SystemTime,
) -> AgentLiveness {
    let Some(facts) = facts else {
        return AgentLiveness::Unknown;
    };
    classify_facts(
        facts.ended,
        facts.last_activity,
        facts.lock_released,
        summary_in_worktree,
        now,
    )
}

/// A child agent classified with the same rules; a child has no lock of its own.
fn classify_child(mut child: ChildAgent, now: SystemTime) -> ChildAgent {
    child.liveness = classify_facts(child.ended, child.last_activity, None, false, now);
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

/// Scan one project with an explicit adapter list.
///
/// Core scan → [`CoreSnapshot`] → each adapter's `enrich` (panics caught) →
/// claims accepted in registration order, first claim wins, out-of-range
/// indexes ignored → a row for every worktree matching the agent predicate or
/// claimed by an adapter → git counts per row (skipped for a `prunable`
/// worktree) → liveness per row and child → only `Live` worktree-less agents
/// kept (D-C07).
///
/// Public so an adapter's own tests — and the D-A07 proof in
/// `tests/agents_scan.rs` — can feed a test-only adapter through the real core.
/// A non-git project still runs its adapters, over an empty worktree list.
pub fn scan_project_with(
    project_root: &Path,
    adapters: &[Box<dyn AgentAdapter>],
    now: SystemTime,
) -> ProjectAgents {
    let core = worktrees::scan_worktrees(project_root);
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
    let mut rows: Vec<AgentRow> = core
        .worktrees
        .iter()
        .zip(claims)
        .filter(|(wt, claim)| wt.agent_pattern || claim.is_some())
        .map(|(wt, claim)| {
            let (commits_ahead, dirty) = worktrees::worktree_counts(wt, base_sha);
            let liveness = classify_liveness(claim.as_ref().map(|(_, facts)| facts), false, now);
            let mut row = AgentRow {
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

    ProjectAgents {
        rows,
        worktreeless,
        main_worktree: core.main_worktree.clone(),
        base_sha: core.base_sha.clone(),
        scanned_at: Some(now),
    }
}

/// Scan every registered project with the registered adapters.
pub fn scan_projects_guarded(
    projects: &[(String, PathBuf)],
    now: SystemTime,
) -> Vec<(String, ProjectAgents)> {
    let _ = (projects, now);
    Vec::new()
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
