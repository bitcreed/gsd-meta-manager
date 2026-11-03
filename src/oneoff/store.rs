//! Where a one-off run lives and what the host records about it (C-06, C-13, C-14).
//!
//! Layout: `<data_local_dir>/gsd-meta-manager/oneoff/<hash8(canonical project)>/`
//! holds the per-project lock ([`super::LOCK_FILE`]) and one directory per run,
//! `<run-id>/`, which holds `run.json` (this module's [`RunRecord`]), `stream.jsonl`,
//! `launch.log`, the clone `work/`, the agent's `report/` and the per-run config dir.
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

/// One landing of the run's work.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Landing {
    pub kind: LandingKind,
    pub at: String,
    /// The branch merged into, or `<dest> <sha>` for a push.
    pub detail: String,
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
    /// The SUMMARY.md the TUI may open (T-f5y-13): `<run-dir>/work/<summary_path>`,
    /// checked off the render thread to be a regular file whose canonical path sits
    /// under the clone. `None` when there is no such file.
    pub summary_file: Option<PathBuf>,
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
        derive_live, summary_path_from, valid_oneoff_run_id, LockSeen, OneoffView, RunRecord,
    };
    use crate::oneoff::{LOCK_FILE, RUN_FILE, WORK_DIR};

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
        let live = derive_live(&record, &probe(&lock_path(project_dir)));
        let summary_file = record
            .summary_path
            .as_deref()
            .and_then(|rel| verified_summary(&run_dir, rel));
        Some(OneoffView {
            record,
            live,
            run_dir,
            summary_file,
        })
    }

    /// The latest run of `project`, if any.
    pub fn read_latest(project: &Path) -> Option<OneoffView> {
        let project_dir = project_dir_for(project)?;
        let run_id = latest_run_id(&project_dir)?;
        read_view(&project_dir, &run_id)
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
        record.landed.push(Landing {
            kind: LandingKind::Pushed,
            at: "2026-11-03T10:00:00Z".to_owned(),
            detail: "refs/heads/gsd-auto/p/x abc123".to_owned(),
        });
        let text = serde_json::to_string(&record).unwrap();
        let back: RunRecord = serde_json::from_str(&text).unwrap();
        assert_eq!(back, record);
        assert_eq!(back.last_pushed_sha().as_deref(), Some("abc123"));
    }
}
