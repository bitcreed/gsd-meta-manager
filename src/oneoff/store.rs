//! Where a one-off run lives and what the host records about it (C-06, C-13, C-14).
//!
//! Layout: `<data_local_dir>/gsd-meta-manager/oneoff/<hash8(canonical project)>/`
//! holds one directory per run, `<run-id>/`, which holds `run.lock` (the per-run lock,
//! [`super::RUN_LOCK_FILE`]), `run.json` (this module's [`RunRecord`]), `stream.jsonl`,
//! `launch.log`, the clone `work/`, the agent's `report/` and the per-run config dir.
//! Any number of runs may be live at once (quick 261103-hx7). A run dir made before
//! per-run locks has no `run.lock`; it is judged by the legacy per-project lock
//! ([`super::LOCK_FILE`], probed only) exactly as before ([`live_seen`]).
//!
//! Everything in `run.json` is host-written. The commit range, the commit count, the
//! changed files and the SUMMARY path are derived on the host from git
//! ([`summary_path_from`]); the agent never supplies them.

use std::path::PathBuf;
use std::sync::LazyLock;

use chrono::{DateTime, Utc};
use regex::Regex;
use serde::{Deserialize, Serialize};

use super::report::{Report, ReportStatus, MAX_HEADLINE};
use super::{AgentOutcome, Mode, TaskSpec, RUN_ID_PREFIX};

/// The record schema version.
pub const RECORD_SCHEMA: u32 = 1;
/// The most changed file names recorded.
pub const MAX_CHANGED_FILES: usize = 200;

/// Where a run is in its life, as the host last recorded it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RecordStatus {
    Preparing,
    Running,
    Finished,
}

/// Where the final report came from.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ReportSource {
    /// The agent's own `report.json`, re-validated.
    Agent,
    /// Synthesized from the outcome; `reason` says why the agent's report was not used.
    Synthesized { reason: String },
}

/// How a run's work left the clone.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LandingKind {
    Merged,
    Pushed,
}

/// How a merge landing got into the target branch (quick 261103-hx7, C-21).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum MergeHow {
    /// The target was an ancestor of the run's head: the user's own `git merge --ff-only`.
    Ff,
    /// A clean `--no-ff` merge commit made by the host.
    Merge,
    /// A merge whose conflicts a sandboxed agent session resolved.
    Resolved,
}

impl MergeHow {
    /// The word `task status` and the TUI show.
    pub fn as_str(self) -> &'static str {
        match self {
            MergeHow::Ff => "ff",
            MergeHow::Merge => "merge",
            MergeHow::Resolved => "resolved",
        }
    }
}

/// The `tracking_note` words that say the branch existed and was removed (klf, I-06,
/// I-07).
pub const NOTE_DELETED_AFTER_LANDING: &str = "deleted after landing";
pub const NOTE_ALREADY_GONE: &str = "already gone at landing";
pub const NOTE_DELETED_BY_DISCARD: &str = "deleted by discard";
const BRANCH_GONE_NOTES: &[&str] = &[
    NOTE_DELETED_AFTER_LANDING,
    NOTE_ALREADY_GONE,
    NOTE_DELETED_BY_DISCARD,
];

/// A run's tracked branch as it is displayed (quick 261103-klf, C-17, I-05).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BranchState {
    /// `gsd-oneoff/<id>` (no `refs/heads/`) at the run's head.
    Tracked { name: String, head: Option<String> },
    /// The branch existed and a landing or a discard removed it.
    Gone { why: String },
    /// Never tracked; `note` is the host's reason, if it kept one.
    Untracked { note: String },
}

/// One landing of the run's work.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Landing {
    pub kind: LandingKind,
    pub at: String,
    /// The branch merged into, or `<dest> <sha>` for a push.
    pub detail: String,
    /// How a merge landed; `None` for a push and for a landing recorded before
    /// quick 261103-hx7 (those were all fast-forwards).
    #[serde(default)]
    pub how: Option<MergeHow>,
    /// The commit the target branch was advanced to by a merge landing.
    #[serde(default)]
    pub merge_sha: Option<String>,
    /// The files that conflicted and were resolved (control-cleaned, capped).
    #[serde(default)]
    pub conflicted: Vec<String>,
    /// The resolver attempt dirs under the run dir (`land/<n>`).
    #[serde(default)]
    pub attempts: Vec<String>,
    /// The git range the landing advanced the target over, `<old>..<new>`: for a
    /// fast-forward `<checkout HEAD before>..<head>`, for a merge or resolved landing
    /// `<target tip>..<merge_sha>` (quick 261103-klf, C-15). `None` for a push and for
    /// a landing recorded before klf.
    #[serde(default)]
    pub range: Option<String>,
}

impl Landing {
    /// A landing with no merge facts (a push, or a fast-forward before they were kept).
    pub fn plain(kind: LandingKind, at: String, detail: String) -> Self {
        Landing {
            kind,
            at,
            detail,
            how: None,
            merge_sha: None,
            conflicted: Vec::new(),
            attempts: Vec::new(),
            range: None,
        }
    }
}

/// Why the latest merge landing of a run did not land (quick 261103-hx7, D-07, I-19).
/// Only the latest failure is kept; a later success clears it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LandFailure {
    pub at: String,
    pub reason: String,
    #[serde(default)]
    pub conflicted: Vec<String>,
    /// Every attempt dir the landing made (`land/<n>`), kept on disk.
    #[serde(default)]
    pub attempts: Vec<String>,
}

/// The sandbox posture the run had.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct SandboxRecord {
    pub posture: String,
    #[serde(default)]
    pub missing: Vec<String>,
}

/// `run.json`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RunRecord {
    pub schema: u32,
    pub run_id: String,
    pub alias: String,
    pub project: PathBuf,
    pub mode: Mode,
    pub task: TaskSpec,
    pub runtime: String,
    pub model: String,
    pub max_budget_usd: f64,
    pub cap_minutes: f64,
    #[serde(default)]
    pub sandbox: Option<SandboxRecord>,
    #[serde(default)]
    pub scope_unit: Option<String>,
    #[serde(default)]
    pub base: Option<String>,
    pub branch: String,
    pub started_at: String,
    #[serde(default)]
    pub ended_at: Option<String>,
    pub status: RecordStatus,
    #[serde(default)]
    pub exit_code: Option<i32>,
    #[serde(default)]
    pub outcome: Option<AgentOutcome>,
    #[serde(default)]
    pub report: Option<Report>,
    #[serde(default)]
    pub report_source: Option<ReportSource>,
    #[serde(default)]
    pub head: Option<String>,
    #[serde(default)]
    pub commits: Option<u32>,
    #[serde(default)]
    pub changed_files: Vec<String>,
    #[serde(default)]
    pub summary_path: Option<String>,
    #[serde(default)]
    pub warnings: Vec<String>,
    #[serde(default)]
    pub landed: Vec<Landing>,
    /// The soft spend cap in force when the run ended or as last set (quick 261103-gt1).
    #[serde(default)]
    pub soft_cap_usd: Option<f64>,
    /// When the spend estimate first reached the soft cap.
    #[serde(default)]
    pub soft_cap_hit_at: Option<String>,
    /// The full ref (`refs/heads/gsd-oneoff/<run-id>`) the host tracks the run's work
    /// under in the user's project, when it was made at finish or already matched
    /// (quick 261103-hx7, C-05, C-06).
    #[serde(default)]
    pub tracked_branch: Option<String>,
    /// Why there is no tracked branch, or why it was refused.
    #[serde(default)]
    pub tracking_note: Option<String>,
    /// The run's spend in notional dollars: the session's estimate at finish plus every
    /// conflict resolver's (quick 261103-hx7, C-23, I-17). `None` for a record written
    /// before it was kept; the landing folds it once from `stream.jsonl`.
    #[serde(default)]
    pub spend_usd: Option<f64>,
    /// Why the latest merge landing failed, if it did.
    #[serde(default)]
    pub land_failure: Option<LandFailure>,
}

impl RunRecord {
    /// A fresh `preparing` record.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        run_id: &str,
        alias: &str,
        project: PathBuf,
        mode: Mode,
        task: TaskSpec,
        runtime: &str,
        model: &str,
        max_budget_usd: f64,
        cap_minutes: f64,
        now: DateTime<Utc>,
    ) -> Self {
        RunRecord {
            schema: RECORD_SCHEMA,
            run_id: run_id.to_owned(),
            alias: alias.to_owned(),
            project,
            mode,
            task,
            runtime: runtime.to_owned(),
            model: model.to_owned(),
            max_budget_usd,
            cap_minutes,
            sandbox: None,
            scope_unit: None,
            base: None,
            branch: format!("{}{run_id}", super::BRANCH_PREFIX),
            started_at: rfc3339(now),
            ended_at: None,
            status: RecordStatus::Preparing,
            exit_code: None,
            outcome: None,
            report: None,
            report_source: None,
            head: None,
            commits: None,
            changed_files: Vec::new(),
            summary_path: None,
            warnings: Vec::new(),
            landed: Vec::new(),
            soft_cap_usd: None,
            soft_cap_hit_at: None,
            tracked_branch: None,
            tracking_note: None,
            spend_usd: None,
            land_failure: None,
        }
    }

    /// The latest merge landing that made a merge commit (a clean `merge` or a
    /// `resolved` one) and recorded its sha: the commit the TUI's `d` diffs against its
    /// first parent (quick 261103-hx7, D-06, C-30).
    pub fn last_merge_landing(&self) -> Option<&Landing> {
        self.landed.iter().rev().find(|l| {
            l.kind == LandingKind::Merged
                && matches!(l.how, Some(MergeHow::Merge | MergeHow::Resolved))
                && l.merge_sha.is_some()
        })
    }

    /// What the record says about the run's tracked branch, normalised for display
    /// (quick 261103-klf, C-17, I-05). An old record whose merge landing deleted the
    /// branch still carries the ref; that reads as gone here, so no migration rewrite
    /// is needed. Every display uses this, never the raw field.
    pub fn branch_state(&self) -> BranchState {
        let landed = self.landed.iter().any(|l| l.kind == LandingKind::Merged);
        match &self.tracked_branch {
            Some(_) if landed => BranchState::Gone {
                why: "deleted after landing".to_owned(),
            },
            Some(reference) => BranchState::Tracked {
                name: reference
                    .strip_prefix("refs/heads/")
                    .unwrap_or(reference)
                    .to_owned(),
                head: self.head.clone(),
            },
            None => match self.tracking_note.as_deref() {
                Some(note) if BRANCH_GONE_NOTES.contains(&note) => BranchState::Gone {
                    why: note.to_owned(),
                },
                Some(note) => BranchState::Untracked {
                    note: note.to_owned(),
                },
                None if landed => BranchState::Gone {
                    why: "deleted after landing".to_owned(),
                },
                None => BranchState::Untracked {
                    note: String::new(),
                },
            },
        }
    }

    /// The sha of the latest push landing, if any.
    pub fn last_pushed_sha(&self) -> Option<String> {
        self.landed
            .iter()
            .rev()
            .find(|l| l.kind == LandingKind::Pushed)
            .and_then(|l| l.detail.split_whitespace().nth(1).map(str::to_owned))
    }
}

/// `now` as RFC 3339 to the second.
pub fn rfc3339(now: DateTime<Utc>) -> String {
    now.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
}

static SUMMARY_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^\.planning/quick/(?P<dir>[A-Za-z0-9][A-Za-z0-9._-]*)/(?P<id>[A-Za-z0-9][A-Za-z0-9._-]*)-SUMMARY\.md$",
    )
    .expect("static regex")
});

/// The SUMMARY path among `changes` (`(status letter, path)` from
/// `git diff --name-status`), per I-5: a path `.planning/quick/<dir>/<id>-SUMMARY.md`
/// whose `<dir>` starts with `<id>-`. Only added (`A`) and modified (`M`) paths count;
/// an added one beats a modified one, and among equals the last in byte order wins.
pub fn summary_path_from(changes: &[(char, String)]) -> Option<String> {
    changes
        .iter()
        .filter(|(status, _)| matches!(status, 'A' | 'M'))
        .filter(|(_, path)| {
            SUMMARY_RE.captures(path).is_some_and(|caps| {
                let id = &caps["id"];
                caps["dir"].starts_with(&format!("{id}-"))
            })
        })
        .max_by(|a, b| (a.0 == 'A', a.1.as_bytes()).cmp(&(b.0 == 'A', b.1.as_bytes())))
        .map(|(_, path)| path.clone())
}

static RUN_ID_RE: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^oneoff-[0-9]{8}-[0-9]{6}(-[0-9]{1,4})?$").expect("static regex")
});

/// The run id minted at `now` (UTC): `oneoff-YYYYMMDD-HHMMSS` (I-2). The caller adds
/// `-2`, `-3`... on a collision.
pub fn mint_run_id(now: DateTime<Utc>) -> String {
    format!("{RUN_ID_PREFIX}{}", now.format("%Y%m%d-%H%M%S"))
}

/// Whether `id` is a one-off run id (one safe path component, T-f5y-10).
pub fn valid_oneoff_run_id(id: &str) -> bool {
    RUN_ID_RE.is_match(id)
}

/// `text` cut to `max` characters, with control characters replaced by spaces.
fn fit(text: &str, max: usize) -> String {
    let cleaned: String = text
        .chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .collect();
    if cleaned.chars().count() <= max {
        return cleaned;
    }
    let mut cut: String = cleaned.chars().take(max - 1).collect();
    cut.push('…');
    cut
}

/// The report a run without a valid agent report ends with (C-14, I-6). It passes
/// [`Report::validate`].
pub fn synthesize(outcome: &AgentOutcome, max_budget_usd: f64, cap_minutes: f64) -> Report {
    let (status, headline) = match outcome {
        AgentOutcome::Finished => (
            ReportStatus::Partial,
            "The agent finished without reporting; check the commits".to_owned(),
        ),
        AgentOutcome::Failed { detail } => (
            ReportStatus::Failed,
            format!("The run failed without a report: {detail}"),
        ),
        AgentOutcome::RateLimited { detail } => (
            ReportStatus::Blocked,
            format!("Rate limited before the agent reported: {detail}"),
        ),
        AgentOutcome::OverBudget => (
            ReportStatus::Failed,
            format!("The run hit its ${max_budget_usd:.2} spend ceiling before reporting"),
        ),
        AgentOutcome::TimedOut => (
            ReportStatus::Failed,
            format!("The run hit its {cap_minutes} min wall clock before reporting"),
        ),
        AgentOutcome::Stopped => (
            ReportStatus::Failed,
            "The run was stopped before the agent reported".to_owned(),
        ),
    };
    let headline = fit(&headline, MAX_HEADLINE);
    Report::validate(status, &headline, &[], &[], &[]).unwrap_or_else(|_| {
        Report::validate(status, outcome.word(), &[], &[], &[])
            .expect("a fixed one-word headline is valid")
    })
}

/// What the per-project lock says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockSeen {
    /// Held by the run with this id.
    HeldBy(String),
    /// Nothing holds it.
    Free,
    /// The probe could not tell.
    Unknown,
}

/// Whether a run is live, derived from its record and the lock.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LiveState {
    Preparing,
    Running,
    Finished,
    /// The record says live but nothing holds the lock: the daemon is gone.
    Crashed,
    /// The lock could not be probed. Never treated as finished.
    Unknown,
}

impl LiveState {
    /// Whether the run may still be live (landing and discard are refused).
    pub fn may_be_live(self) -> bool {
        matches!(
            self,
            LiveState::Preparing | LiveState::Running | LiveState::Unknown
        )
    }

    pub fn as_str(self) -> &'static str {
        match self {
            LiveState::Preparing => "preparing",
            LiveState::Running => "running",
            LiveState::Finished => "finished",
            LiveState::Crashed => "crashed",
            LiveState::Unknown => "unknown",
        }
    }
}

/// The live state of `record` given what the lock says (pure).
pub fn derive_live(record: &RunRecord, seen: &LockSeen) -> LiveState {
    match seen {
        LockSeen::Unknown => LiveState::Unknown,
        LockSeen::HeldBy(id) if *id == record.run_id => match record.status {
            RecordStatus::Preparing => LiveState::Preparing,
            // The lock is released last, so a finished record under a held lock is
            // still wrapping up.
            RecordStatus::Running | RecordStatus::Finished => LiveState::Running,
        },
        LockSeen::HeldBy(_) | LockSeen::Free => match record.status {
            RecordStatus::Finished => LiveState::Finished,
            RecordStatus::Preparing | RecordStatus::Running => LiveState::Crashed,
        },
    }
}

/// The latest run of a project as the TUI and `task status` see it.
#[derive(Debug, Clone, PartialEq)]
pub struct OneoffView {
    pub record: RunRecord,
    pub live: LiveState,
    pub run_dir: PathBuf,
    /// A landing of this run is in progress: its `land.lock` is held (or could not be
    /// probed, which is never read as free) (quick 261103-hx7, I-21).
    pub landing: bool,
    /// The resolver attempt dirs `land/<n>` that exist, ascending (D-08, C-25).
    pub land_attempts: Vec<u32>,
    /// The SUMMARY.md the TUI may open (T-f5y-13): `<run-dir>/work/<summary_path>`,
    /// checked off the render thread to be a regular file whose canonical path sits
    /// under the clone. `None` when there is no such file.
    pub summary_file: Option<PathBuf>,
}

/// The most runs the Driver tab reads and lists (quick 261103-hx7, I-21).
pub const ONEOFF_RUNS_LISTED: usize = 20;

/// What the Driver tab's one-off status line counts (quick 261103-hx7, C-26).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunCounts {
    /// Runs that may still be live.
    pub running: usize,
    /// Runs with a landing in progress.
    pub landing: usize,
    /// Finished runs with commits that no merge has landed yet.
    pub to_land: usize,
    /// Every run counted.
    pub total: usize,
}

impl RunCounts {
    /// `one-offs: 2 running, 1 to land (T)`, zero parts omitted; `3 finished` when
    /// there is nothing else to say.
    pub fn text(&self) -> String {
        let mut parts = Vec::new();
        if self.running > 0 {
            parts.push(format!("{} running", self.running));
        }
        if self.landing > 0 {
            parts.push(format!("{} landing", self.landing));
        }
        if self.to_land > 0 {
            parts.push(format!("{} to land", self.to_land));
        }
        if parts.is_empty() {
            parts.push(format!("{} finished", self.total));
        }
        format!("one-offs: {} (T)", parts.join(", "))
    }
}

/// The counts of `runs` (pure). A run is *to land* when it is not live, has a head and
/// commits, has no merge landing recorded and is not being landed right now.
pub fn run_counts(runs: &[OneoffView]) -> RunCounts {
    let mut counts = RunCounts {
        running: 0,
        landing: 0,
        to_land: 0,
        total: runs.len(),
    };
    for view in runs {
        if view.live.may_be_live() {
            counts.running += 1;
        } else if view.landing {
            counts.landing += 1;
        } else if view.record.head.is_some()
            && view.record.commits.unwrap_or(0) > 0
            && !view
                .record
                .landed
                .iter()
                .any(|l| l.kind == LandingKind::Merged)
        {
            counts.to_land += 1;
        }
    }
    counts
}

/// Why a run could not be picked (quick 261103-hx7, I-03).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RunPick {
    /// The project has no run at all.
    NoRuns,
    /// The id is not a run of this project (or is not a run id).
    NotFound(String),
    /// Several runs may be live and no id was given: the live ids, newest first.
    SeveralLive(Vec<String>),
}

impl RunPick {
    /// The refusal without a verb (the ids are the host's own validated names, a
    /// user-given one is escaped by its debug form).
    pub fn detail(&self) -> String {
        match self {
            RunPick::NoRuns => "this project has no one-off runs".to_owned(),
            RunPick::NotFound(id) => format!("no one-off run {id:?} for this project"),
            RunPick::SeveralLive(ids) => format!(
                "several one-offs may be live; pass --run-id: {}",
                ids.join(", ")
            ),
        }
    }

    /// The refusal as one line for `verb`.
    pub fn message(&self, verb: &str) -> String {
        format!("{verb}: {}", self.detail())
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub use self::fs_store::*;

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod fs_store {
    use std::fs;
    use std::io;
    use std::path::{Path, PathBuf};

    use crate::driver::lock::{probe_at, read_holder_at, LockProbe};
    use crate::driverctl::serve::hash8;
    use crate::state_reader::driver_frontmatter::read_run_file_capped;

    use super::{
        derive_live, summary_path_from, valid_oneoff_run_id, LockSeen, OneoffView, RunPick,
        RunRecord,
    };
    use crate::oneoff::{LAND_DIR, LAND_LOCK_FILE, LOCK_FILE, RUN_FILE, RUN_LOCK_FILE, WORK_DIR};

    /// `dirs::data_local_dir()/gsd-meta-manager/oneoff`, or `None` without a data dir.
    /// Never a `/tmp` fallback.
    pub fn oneoff_root() -> Option<PathBuf> {
        dirs::data_local_dir().map(|d| d.join("gsd-meta-manager").join("oneoff"))
    }

    /// `<root>/<hash8(canonical project)>`.
    pub fn project_dir_in(root: &Path, canonical_project: &Path) -> PathBuf {
        root.join(hash8(canonical_project))
    }

    /// The per-project lock under `project_dir`.
    pub fn lock_path(project_dir: &Path) -> PathBuf {
        project_dir.join(LOCK_FILE)
    }

    /// The per-run lock inside `run_dir` (quick 261103-hx7, C-01).
    pub fn run_lock_path(run_dir: &Path) -> PathBuf {
        run_dir.join(RUN_LOCK_FILE)
    }

    /// What the lock that judges the run in `run_dir` says (I-02): its own `run.lock`
    /// when that exists, otherwise the LEGACY per-project lock under `project_dir`, so a
    /// run dir made before per-run locks reads exactly as it did.
    pub fn live_seen(project_dir: &Path, run_dir: &Path) -> LockSeen {
        let own = run_lock_path(run_dir);
        if fs::symlink_metadata(&own).is_ok() {
            probe(&own)
        } else {
            probe(&lock_path(project_dir))
        }
    }

    /// The lock file [`live_seen`] judges the run in `run_dir` by: the same choice,
    /// for callers that must signal the holder (`stop`).
    pub fn live_lock_path(project_dir: &Path, run_dir: &Path) -> PathBuf {
        let own = run_lock_path(run_dir);
        if fs::symlink_metadata(&own).is_ok() {
            own
        } else {
            lock_path(project_dir)
        }
    }

    /// Write `record` to `<run_dir>/run.json` atomically, 0600.
    pub fn write_record(run_dir: &Path, record: &RunRecord) -> io::Result<()> {
        use std::io::Write;
        use std::os::unix::fs::PermissionsExt;

        let rendered = serde_json::to_vec_pretty(record)
            .map_err(|e| io::Error::new(io::ErrorKind::InvalidData, e))?;
        let mut tmp = tempfile::NamedTempFile::new_in(run_dir)?;
        tmp.write_all(&rendered)?;
        tmp.write_all(b"\n")?;
        fs::set_permissions(tmp.path(), fs::Permissions::from_mode(0o600))?;
        tmp.persist(run_dir.join(RUN_FILE)).map_err(|e| e.error)?;
        Ok(())
    }

    /// Read `<run_dir>/run.json` capped (symlinks refused).
    pub fn read_record(run_dir: &Path) -> Result<RunRecord, String> {
        let path = run_dir.join(RUN_FILE);
        let bytes = read_run_file_capped(&path).map_err(|e| format!("{}: {e}", path.display()))?;
        serde_json::from_slice(&bytes).map_err(|e| format!("{}: {e}", path.display()))
    }

    /// The greatest valid run id under `project_dir` (ids sort by time).
    pub fn latest_run_id(project_dir: &Path) -> Option<String> {
        fs::read_dir(project_dir)
            .ok()?
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|name| valid_oneoff_run_id(name))
            .max_by(|a, b| run_id_order(a).cmp(&run_id_order(b)))
    }

    /// Sort key: the timestamp, then the numeric collision suffix.
    fn run_id_order(id: &str) -> (String, u32) {
        let base: String = id.chars().take("oneoff-YYYYMMDD-HHMMSS".len()).collect();
        let n = id
            .get(base.len()..)
            .and_then(|rest| rest.strip_prefix('-'))
            .and_then(|n| n.parse().ok())
            .unwrap_or(1);
        (base, n)
    }

    /// What the lock at `lock` says.
    pub fn probe(lock: &Path) -> LockSeen {
        match probe_at(lock) {
            LockProbe::Held => match read_holder_at(lock) {
                Some(holder) => LockSeen::HeldBy(holder.run_id),
                None => LockSeen::Unknown,
            },
            LockProbe::Free | LockProbe::Absent => LockSeen::Free,
            LockProbe::Unknown(_) => LockSeen::Unknown,
        }
    }

    /// The project dir of `project` under the one-off root, `None` without a data dir
    /// or when `project` cannot be canonicalized.
    pub fn project_dir_for(project: &Path) -> Option<PathBuf> {
        let canonical = fs::canonicalize(project).ok()?;
        Some(project_dir_in(&oneoff_root()?, &canonical))
    }

    /// `<run_dir>/work/<rel>` when `rel` has the SUMMARY shape and names a regular
    /// file (never a symlink) whose canonical path sits under the canonical clone.
    pub fn verified_summary(run_dir: &Path, rel: &str) -> Option<PathBuf> {
        summary_path_from(&[('A', rel.to_owned())])?;
        let work = run_dir.join(WORK_DIR);
        let candidate = work.join(rel);
        let meta = fs::symlink_metadata(&candidate).ok()?;
        if !meta.file_type().is_file() {
            return None;
        }
        let canonical = fs::canonicalize(&candidate).ok()?;
        let root = fs::canonicalize(&work).ok()?;
        canonical.starts_with(&root).then_some(candidate)
    }

    /// The view of one run.
    pub fn read_view(project_dir: &Path, run_id: &str) -> Option<OneoffView> {
        if !valid_oneoff_run_id(run_id) {
            return None;
        }
        let run_dir = project_dir.join(run_id);
        let record = read_record(&run_dir).ok()?;
        let live = derive_live(&record, &live_seen(project_dir, &run_dir));
        let summary_file = record
            .summary_path
            .as_deref()
            .and_then(|rel| verified_summary(&run_dir, rel));
        let landing = landing_held(&run_dir);
        let land_attempts = land_attempts_of(&run_dir);
        Some(OneoffView {
            record,
            live,
            run_dir,
            landing,
            land_attempts,
            summary_file,
        })
    }

    /// Whether the run's `land.lock` is held. A lock that cannot be probed counts as
    /// held, never as free (the same rule as `land::landing_in_progress`).
    fn landing_held(run_dir: &Path) -> bool {
        matches!(
            probe_at(&run_dir.join(LAND_LOCK_FILE)),
            LockProbe::Held | LockProbe::Unknown(_)
        )
    }

    /// The numbered attempt dirs under `<run_dir>/land`, ascending. Symlinks and
    /// non-numeric names are ignored.
    pub fn land_attempts_of(run_dir: &Path) -> Vec<u32> {
        let mut found: Vec<u32> = fs::read_dir(run_dir.join(LAND_DIR))
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
            .filter_map(|e| e.file_name().into_string().ok())
            .filter_map(|name| name.parse::<u32>().ok())
            .collect();
        found.sort_unstable();
        found
    }

    /// Every valid run id under `project_dir`, newest first.
    pub fn run_ids_newest_first(project_dir: &Path) -> Vec<String> {
        let mut ids: Vec<String> = fs::read_dir(project_dir)
            .into_iter()
            .flatten()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_ok_and(|t| t.is_dir()))
            .filter_map(|e| e.file_name().into_string().ok())
            .filter(|name| valid_oneoff_run_id(name))
            .collect();
        ids.sort_by_key(|id| std::cmp::Reverse(run_id_order(id)));
        ids
    }

    /// The views of the runs under `project_dir` that have a readable `run.json`,
    /// newest first, at most `limit`.
    pub fn read_runs_in(project_dir: &Path, limit: Option<usize>) -> Vec<OneoffView> {
        let views = run_ids_newest_first(project_dir)
            .into_iter()
            .filter_map(|id| read_view(project_dir, &id));
        match limit {
            Some(n) => views.take(n).collect(),
            None => views.collect(),
        }
    }

    /// [`read_runs_in`] for `project`.
    pub fn read_runs(project: &Path, limit: Option<usize>) -> Vec<OneoffView> {
        project_dir_for(project)
            .map(|dir| read_runs_in(&dir, limit))
            .unwrap_or_default()
    }

    /// The run a verb acts on (quick 261103-hx7, I-03). `Some(id)` is that run, or
    /// `NotFound`. `None` is the newest run, except when `refuse_several_live` is set:
    /// then exactly one live run is that run (even when a newer one has finished), and
    /// more than one run that may be live is `SeveralLive` (newest first), never a
    /// guess `[inferred — audit]`.
    pub fn resolve_run(
        project_dir: &Path,
        run_id: Option<&str>,
        refuse_several_live: bool,
    ) -> Result<OneoffView, RunPick> {
        if let Some(id) = run_id {
            return read_view(project_dir, id).ok_or_else(|| RunPick::NotFound(id.to_owned()));
        }
        let views = read_runs_in(project_dir, None);
        if refuse_several_live {
            let live: Vec<&OneoffView> = views.iter().filter(|v| v.live.may_be_live()).collect();
            match live.len() {
                0 => {}
                1 => return Ok(live[0].clone()),
                _ => {
                    return Err(RunPick::SeveralLive(
                        live.iter().map(|v| v.record.run_id.clone()).collect(),
                    ))
                }
            }
        }
        views.into_iter().next().ok_or(RunPick::NoRuns)
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use tempfile::TempDir;

        #[test]
        fn the_latest_run_id_honours_the_collision_suffix() {
            let root = TempDir::new().unwrap();
            for id in [
                "oneoff-20261103-100000",
                "oneoff-20261103-100000-2",
                "oneoff-20261103-100000-10",
                "oneoff-20261102-235959",
                "not-a-run",
            ] {
                fs::create_dir(root.path().join(id)).unwrap();
            }
            assert_eq!(
                latest_run_id(root.path()).as_deref(),
                Some("oneoff-20261103-100000-10")
            );
        }

        #[test]
        fn only_a_regular_summary_file_inside_the_clone_is_offered() {
            let root = TempDir::new().unwrap();
            let rel = ".planning/quick/261103-abc-x/261103-abc-SUMMARY.md";
            let dir = root.path().join("work/.planning/quick/261103-abc-x");
            fs::create_dir_all(&dir).unwrap();
            assert_eq!(verified_summary(root.path(), rel), None, "missing");
            fs::write(dir.join("261103-abc-SUMMARY.md"), "s").unwrap();
            assert_eq!(
                verified_summary(root.path(), rel),
                Some(root.path().join("work").join(rel))
            );
            fs::remove_file(dir.join("261103-abc-SUMMARY.md")).unwrap();
            let outside = root.path().join("secret.txt");
            fs::write(&outside, "x").unwrap();
            std::os::unix::fs::symlink(&outside, dir.join("261103-abc-SUMMARY.md")).unwrap();
            assert_eq!(verified_summary(root.path(), rel), None, "symlink");
            assert_eq!(verified_summary(root.path(), "../secret.txt"), None);
        }

        use crate::driver::lock::acquire_at;

        fn running(run_id: &str) -> RunRecord {
            let mut record = super::super::tests::record();
            record.run_id = run_id.to_owned();
            record.status = crate::oneoff::store::RecordStatus::Running;
            record
        }

        /// A run dir under `project_dir` with a record and, when `held`, a held lock.
        fn run_in(
            project_dir: &Path,
            id: &str,
            held: bool,
        ) -> Option<crate::driver::lock::RunLock> {
            let run_dir = project_dir.join(id);
            fs::create_dir(&run_dir).unwrap();
            let lock = held.then(|| acquire_at(&run_lock_path(&run_dir), id, 1).unwrap());
            write_record(&run_dir, &running(id)).unwrap();
            lock
        }

        #[test]
        fn a_run_lock_is_preferred_and_the_legacy_project_lock_is_the_fallback() {
            let project_dir = TempDir::new().unwrap();
            let legacy = "oneoff-20261103-100000";
            let modern = "oneoff-20261103-100001";
            // A legacy run: no run.lock, judged by the project lock.
            let legacy_dir = project_dir.path().join(legacy);
            fs::create_dir(&legacy_dir).unwrap();
            write_record(&legacy_dir, &running(legacy)).unwrap();
            let project_lock =
                acquire_at(&lock_path(project_dir.path()), legacy, 1).expect("legacy lock");
            assert_eq!(
                read_view(project_dir.path(), legacy).unwrap().live,
                crate::oneoff::store::LiveState::Running
            );
            // A modern run whose own lock is free is Crashed even while the project
            // lock is held by the legacy run.
            let _ = run_in(project_dir.path(), modern, false);
            fs::write(project_dir.path().join(modern).join("run.lock"), "").unwrap();
            assert_eq!(
                read_view(project_dir.path(), modern).unwrap().live,
                crate::oneoff::store::LiveState::Crashed
            );
            drop(project_lock);
            // A process another test forks at this very moment holds a copy of the
            // lock's file description until its exec, so a read right after the drop
            // can still see the lock held: poll briefly instead of asserting once.
            let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
            let mut seen = read_view(project_dir.path(), legacy).unwrap().live;
            while seen != crate::oneoff::store::LiveState::Crashed
                && std::time::Instant::now() < deadline
            {
                std::thread::sleep(std::time::Duration::from_millis(20));
                seen = read_view(project_dir.path(), legacy).unwrap().live;
            }
            assert_eq!(seen, crate::oneoff::store::LiveState::Crashed);
        }

        #[test]
        fn resolve_run_picks_the_newest_and_refuses_when_several_are_live() {
            use crate::oneoff::store::RunPick;
            let project_dir = TempDir::new().unwrap();
            let (a, b, c) = (
                "oneoff-20261103-100000",
                "oneoff-20261103-100001",
                "oneoff-20261103-100002",
            );
            assert_eq!(
                resolve_run(project_dir.path(), None, true).unwrap_err(),
                RunPick::NoRuns
            );
            let _lock_a = run_in(project_dir.path(), a, true);
            let picked = resolve_run(project_dir.path(), None, true).unwrap();
            assert_eq!(picked.record.run_id, a, "the only live run");
            let _lock_b = run_in(project_dir.path(), b, true);
            assert_eq!(
                resolve_run(project_dir.path(), None, true).unwrap_err(),
                RunPick::SeveralLive(vec![b.to_owned(), a.to_owned()])
            );
            // Without the refusal the newest run is picked.
            assert_eq!(
                resolve_run(project_dir.path(), None, false)
                    .unwrap()
                    .record
                    .run_id,
                b
            );
            // An id picks that run, and an unknown or invalid one is NotFound.
            assert_eq!(
                resolve_run(project_dir.path(), Some(a), true)
                    .unwrap()
                    .record
                    .run_id,
                a
            );
            for bad in [c, "../x", "r1"] {
                assert_eq!(
                    resolve_run(project_dir.path(), Some(bad), true).unwrap_err(),
                    RunPick::NotFound(bad.to_owned())
                );
            }
            let listed: Vec<String> = read_runs_in(project_dir.path(), None)
                .into_iter()
                .map(|v| v.record.run_id)
                .collect();
            assert_eq!(listed, [b, a]);
            assert_eq!(read_runs_in(project_dir.path(), Some(1)).len(), 1);
        }

        #[test]
        fn a_record_round_trips_on_disk_and_an_absent_lock_is_free() {
            let root = TempDir::new().unwrap();
            let record = super::super::tests::record();
            write_record(root.path(), &record).unwrap();
            assert_eq!(read_record(root.path()).unwrap(), record);
            assert_eq!(probe(&root.path().join("oneoff.lock")), LockSeen::Free);
        }
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;

    pub(crate) fn record() -> RunRecord {
        RunRecord::new(
            "oneoff-20261103-142233",
            "p28",
            PathBuf::from("/work/p"),
            Mode::Quick,
            TaskSpec::Text("fix x".to_owned()),
            "agent",
            "opus",
            10.0,
            60.0,
            DateTime::from_timestamp(1_800_000_000, 0).unwrap(),
        )
    }

    fn changes(list: &[(char, &str)]) -> Vec<(char, String)> {
        list.iter().map(|(c, p)| (*c, (*p).to_owned())).collect()
    }

    #[test]
    fn the_summary_path_follows_the_quick_layout() {
        let path = ".planning/quick/261103-em6-slug/261103-em6-SUMMARY.md";
        assert_eq!(
            summary_path_from(&changes(&[('M', "src/a.rs"), ('A', path)])).as_deref(),
            Some(path)
        );
        let irregular = ".planning/quick/261006-badge-x/261006-SUMMARY.md";
        assert_eq!(
            summary_path_from(&changes(&[('A', irregular)])).as_deref(),
            Some(irregular)
        );
    }

    #[test]
    fn an_added_summary_beats_a_modified_one() {
        let added = ".planning/quick/261103-aaa-x/261103-aaa-SUMMARY.md";
        let modified = ".planning/quick/261103-zzz-x/261103-zzz-SUMMARY.md";
        assert_eq!(
            summary_path_from(&changes(&[('A', added), ('M', modified)])).as_deref(),
            Some(added)
        );
        let later = ".planning/quick/261103-bbb-x/261103-bbb-SUMMARY.md";
        assert_eq!(
            summary_path_from(&changes(&[('A', later), ('A', added)])).as_deref(),
            Some(later)
        );
    }

    #[test]
    fn no_match_or_an_odd_path_gives_none() {
        for odd in [
            "src/SUMMARY.md",
            ".planning/quick/../261103-abc-SUMMARY.md",
            ".planning/quick/x y/x-SUMMARY.md",
            ".planning/quick/261103-abc-x/261103-abd-SUMMARY.md",
            ".planning/quick/261103-abc-x/sub/261103-abc-SUMMARY.md",
        ] {
            assert_eq!(summary_path_from(&changes(&[('A', odd)])), None, "{odd}");
        }
        let deleted = ".planning/quick/261103-abc-x/261103-abc-SUMMARY.md";
        assert_eq!(summary_path_from(&changes(&[('D', deleted)])), None);
    }

    #[test]
    fn run_ids_are_minted_and_checked() {
        let now = DateTime::parse_from_rfc3339("2026-11-03T14:22:33Z")
            .unwrap()
            .with_timezone(&Utc);
        let id = mint_run_id(now);
        assert_eq!(id, "oneoff-20261103-142233");
        assert!(valid_oneoff_run_id(&id));
        assert!(valid_oneoff_run_id("oneoff-20261103-142233-2"));
        for bad in [
            "oneoff-..",
            "oneoff-",
            "r1",
            "oneoff-20261103-142233/x",
            "../oneoff-20261103-142233",
        ] {
            assert!(!valid_oneoff_run_id(bad), "{bad}");
        }
    }

    #[test]
    fn live_state_is_derived_from_record_and_lock() {
        let mut running = record();
        running.status = RecordStatus::Running;
        let held = LockSeen::HeldBy(running.run_id.clone());
        assert_eq!(derive_live(&running, &held), LiveState::Running);
        assert_eq!(derive_live(&running, &LockSeen::Free), LiveState::Crashed);
        let mut finished = record();
        finished.status = RecordStatus::Finished;
        assert_eq!(derive_live(&finished, &LockSeen::Free), LiveState::Finished);
        for r in [&running, &finished] {
            assert_eq!(derive_live(r, &LockSeen::Unknown), LiveState::Unknown);
        }
        assert!(LiveState::Unknown.may_be_live());
        assert!(!LiveState::Finished.may_be_live());
    }

    #[test]
    fn every_synthesized_report_is_valid_and_carries_its_status() {
        for (outcome, status) in [
            (AgentOutcome::Finished, ReportStatus::Partial),
            (
                AgentOutcome::Failed {
                    detail: "x".repeat(400),
                },
                ReportStatus::Failed,
            ),
            (
                AgentOutcome::RateLimited {
                    detail: "window=five_hour\nresets".to_owned(),
                },
                ReportStatus::Blocked,
            ),
            (AgentOutcome::OverBudget, ReportStatus::Failed),
            (AgentOutcome::TimedOut, ReportStatus::Failed),
            (AgentOutcome::Stopped, ReportStatus::Failed),
        ] {
            let report = synthesize(&outcome, 10.0, 60.0);
            assert_eq!(report.status, status, "{outcome:?}");
            assert!(
                Report::validate(
                    report.status,
                    &report.headline,
                    &report.changed,
                    &report.verified,
                    &report.followups
                )
                .is_ok(),
                "{report:?}"
            );
        }
        assert!(synthesize(&AgentOutcome::OverBudget, 10.0, 60.0)
            .headline
            .contains("$10.00"));
    }

    #[test]
    fn a_record_round_trips_through_serde() {
        let mut record = record();
        record.outcome = Some(AgentOutcome::OverBudget);
        record.report_source = Some(ReportSource::Synthesized {
            reason: "none".to_owned(),
        });
        record.landed.push(Landing::plain(
            LandingKind::Pushed,
            "2026-11-03T10:00:00Z".to_owned(),
            "refs/heads/gsd-auto/p/x abc123".to_owned(),
        ));
        let text = serde_json::to_string(&record).unwrap();
        let back: RunRecord = serde_json::from_str(&text).unwrap();
        assert_eq!(back, record);
        assert_eq!(back.last_pushed_sha().as_deref(), Some("abc123"));
    }

    #[test]
    fn the_tracking_fields_are_optional_on_read_and_round_trip() {
        let mut value = serde_json::to_value(record()).unwrap();
        let object = value.as_object_mut().unwrap();
        object.remove("tracked_branch");
        object.remove("tracking_note");
        let old: RunRecord = serde_json::from_value(value).unwrap();
        assert_eq!(old.tracked_branch, None);
        assert_eq!(old.tracking_note, None);
        let mut tracked = record();
        tracked.tracked_branch = Some("refs/heads/gsd-oneoff/x".to_owned());
        tracked.tracking_note = Some("note".to_owned());
        let back: RunRecord =
            serde_json::from_str(&serde_json::to_string(&tracked).unwrap()).unwrap();
        assert_eq!(back, tracked);
    }

    #[test]
    fn the_soft_cap_fields_are_optional_on_read_and_round_trip() {
        let mut value = serde_json::to_value(record()).unwrap();
        let object = value.as_object_mut().unwrap();
        object.remove("soft_cap_usd");
        object.remove("soft_cap_hit_at");
        let old: RunRecord = serde_json::from_value(value).unwrap();
        assert_eq!(old.soft_cap_usd, None);
        assert_eq!(old.soft_cap_hit_at, None);
        let mut capped = record();
        capped.soft_cap_usd = Some(1.5);
        capped.soft_cap_hit_at = Some("2026-11-03T10:00:00Z".to_owned());
        let back: RunRecord =
            serde_json::from_str(&serde_json::to_string(&capped).unwrap()).unwrap();
        assert_eq!(back, capped);
    }

    #[test]
    fn the_landing_and_spend_fields_are_optional_on_read_and_round_trip() {
        let mut value = serde_json::to_value(record()).unwrap();
        let object = value.as_object_mut().unwrap();
        object.remove("spend_usd");
        object.remove("land_failure");
        let old: RunRecord = serde_json::from_value(value).unwrap();
        assert_eq!(old.spend_usd, None);
        assert_eq!(old.land_failure, None);

        let legacy: Landing = serde_json::from_str(
            r#"{"kind":"merged","at":"2026-11-03T10:00:00Z","detail":"main"}"#,
        )
        .unwrap();
        assert_eq!(legacy.how, None);
        assert_eq!(legacy.merge_sha, None);
        assert!(legacy.conflicted.is_empty() && legacy.attempts.is_empty());

        let mut full = record();
        full.spend_usd = Some(2.5);
        full.land_failure = Some(LandFailure {
            at: "2026-11-03T10:00:00Z".to_owned(),
            reason: "the resolver's report was failed".to_owned(),
            conflicted: vec!["shared.txt".to_owned()],
            attempts: vec!["land/1".to_owned()],
        });
        full.landed.push(Landing {
            kind: LandingKind::Merged,
            at: "2026-11-03T10:00:00Z".to_owned(),
            detail: "main".to_owned(),
            how: Some(MergeHow::Resolved),
            merge_sha: Some("abc".to_owned()),
            conflicted: vec!["shared.txt".to_owned()],
            attempts: vec!["land/1".to_owned()],
            range: Some("aaa..abc".to_owned()),
        });
        let text = serde_json::to_string(&full).unwrap();
        assert!(text.contains(r#""how":"resolved""#), "{text}");
        let back: RunRecord = serde_json::from_str(&text).unwrap();
        assert_eq!(back, full);
    }
    fn view_of(live: LiveState, commits: u32, landed: bool, landing: bool) -> OneoffView {
        let mut record = record();
        record.head = Some("a".repeat(40));
        record.commits = Some(commits);
        if landed {
            record.landed.push(Landing::plain(
                LandingKind::Merged,
                "2026-11-03T10:00:00Z".to_owned(),
                "main".to_owned(),
            ));
        }
        OneoffView {
            record,
            live,
            run_dir: PathBuf::from("/r"),
            landing,
            land_attempts: Vec::new(),
            summary_file: None,
        }
    }

    #[test]
    fn the_counts_name_running_landing_and_to_land_runs() {
        let runs = [
            view_of(LiveState::Running, 0, false, false),
            view_of(LiveState::Preparing, 0, false, false),
            view_of(LiveState::Finished, 2, false, false),
            view_of(LiveState::Finished, 2, false, true),
            view_of(LiveState::Finished, 2, true, false),
            view_of(LiveState::Finished, 0, false, false),
        ];
        let counts = run_counts(&runs);
        assert_eq!(
            counts,
            RunCounts {
                running: 2,
                landing: 1,
                to_land: 1,
                total: 6
            }
        );
        assert_eq!(
            counts.text(),
            "one-offs: 2 running, 1 landing, 1 to land (T)"
        );
    }

    #[test]
    fn the_counts_omit_zero_parts_and_fall_back_to_finished() {
        let runs = [
            view_of(LiveState::Running, 0, false, false),
            view_of(LiveState::Running, 0, false, false),
            view_of(LiveState::Finished, 3, false, false),
        ];
        assert_eq!(
            run_counts(&runs).text(),
            "one-offs: 2 running, 1 to land (T)"
        );
        let done = [
            view_of(LiveState::Finished, 1, true, false),
            view_of(LiveState::Finished, 0, false, false),
            view_of(LiveState::Crashed, 0, false, false),
        ];
        assert_eq!(run_counts(&done).text(), "one-offs: 3 finished (T)");
    }

    /// A literal `run.json` as hx7 wrote it: a Merged `merge` landing with its sha and
    /// no `range`, `tracked_branch` still set, none of the klf fields.
    const PRE_KLF_RECORD: &str = r#"{
        "schema": 1, "run_id": "oneoff-20261103-142233", "alias": "p",
        "project": "/p", "mode": "quick",
        "task": {"kind": "text", "spec": "do it"},
        "runtime": "agent", "model": "m", "max_budget_usd": 5.0, "cap_minutes": 30.0,
        "branch": "gsd-oneoff/oneoff-20261103-142233",
        "started_at": "2026-11-03T10:00:00Z", "status": "finished",
        "head": "0123456789abcdef0123456789abcdef01234567", "commits": 2,
        "landed": [{"kind": "merged", "at": "2026-11-03T11:00:00Z", "detail": "main",
                    "how": "merge",
                    "merge_sha": "abcdef0123456789abcdef0123456789abcdef01"}],
        "tracked_branch": "refs/heads/gsd-oneoff/oneoff-20261103-142233"
    }"#;

    #[test]
    fn a_pre_klf_record_reads_as_landed_with_no_live_branch() {
        let record: RunRecord = serde_json::from_str(PRE_KLF_RECORD).expect("pre-klf parses");
        assert!(record.landed[0].range.is_none());
        assert_eq!(
            record.branch_state(),
            BranchState::Gone {
                why: NOTE_DELETED_AFTER_LANDING.to_owned()
            }
        );
    }

    #[test]
    fn the_branch_state_names_a_tracked_branch_and_a_gone_one() {
        let mut r = record();
        r.head = Some("a".repeat(40));
        r.tracked_branch = Some(format!("refs/heads/gsd-oneoff/{}", r.run_id));
        assert_eq!(
            r.branch_state(),
            BranchState::Tracked {
                name: format!("gsd-oneoff/{}", r.run_id),
                head: Some("a".repeat(40)),
            }
        );
        r.tracked_branch = None;
        r.tracking_note = Some(NOTE_DELETED_AFTER_LANDING.to_owned());
        assert!(matches!(r.branch_state(), BranchState::Gone { .. }));
        r.tracking_note = Some(NOTE_DELETED_BY_DISCARD.to_owned());
        assert!(matches!(r.branch_state(), BranchState::Gone { .. }));
        r.tracking_note = Some("the run made no commits; no branch tracked".to_owned());
        assert!(matches!(r.branch_state(), BranchState::Untracked { .. }));
        r.tracking_note = None;
        assert_eq!(
            r.branch_state(),
            BranchState::Untracked {
                note: String::new()
            }
        );
    }
}
