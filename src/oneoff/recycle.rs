//! Slimming a landed run and recycling dead ones (quick 261103-o2r, D-A, D-B).
//!
//! A run that landed (merge or push) no longer needs its ~100 MB `--no-hardlinks` clone,
//! its resolver clones or the per-run config dir. [`slim_run`] removes them and keeps a
//! small record: `run.json` (with `pruned {at, note}` and the spend folded in), the
//! agent's report, `injections.jsonl`, a copy of the run's SUMMARY.md, and the tails of
//! `stream.jsonl` and `launch.log`.
//!
//! One rule guards every deletion of a clone that is not an explicit `task land discard`
//! (I-04): after a clean planted-file gate, the clone's `HEAD` and every branch tip that
//! is not an ancestor of the run's base must each be contained in a branch of the
//! project ([`clone_preserved`]). Anything else keeps the clone and says why. `pruned`
//! is written to `run.json` before anything is deleted (I-03), so a record with it set
//! always lands from the tracked branch and a later scan finishes an interrupted slim.
//!
//! Slimming is an allowlist (I-02): everything not named below is deleted, with
//! `symlink_metadata` and `remove_dir_all`/`remove_file`, so a symlink is removed as a
//! link and never followed.

use std::ffi::OsString;
use std::fs;
use std::io::{self, Read, Seek, SeekFrom, Write};
use std::os::unix::fs::{OpenOptionsExt, PermissionsExt};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use chrono::Utc;

use crate::driver::lock::{probe_at, LockProbe};
use crate::driverctl::serve::hash8;
use crate::launcher::process::run_bounded;
use crate::launcher::push;

use super::land::{
    git_relevant_planted, land_lock_path, record_already_landed, ref_commit, take_land_lock,
    track_branch, tracked_ref, try_land_lock,
};
use super::launch::settle_work;
use super::report::read_validated;
use super::store::{
    project_dir_for, read_runs_in, read_view, resume_predecessors, resume_successors, rfc3339,
    run_ids_newest_first, synthesize, verified_summary, write_record, LandingKind, LiveState,
    OneoffView, Pruned, RecordStatus, ReportSource,
};
use super::stream::spend_of_file;
use super::{
    AgentOutcome, INBOX_DIR, INJECTIONS_FILE, KEPT_SUMMARY_FILE, LAND_DIR, LAND_LOCK_FILE,
    LAUNCH_LOG, REPORT_FILE, REPORT_SUBDIR, RUN_FILE, RUN_LOCK_FILE, STREAM_FILE, WORK_DIR,
};

/// How much of `stream.jsonl` a slimmed run keeps (D-A).
pub const STREAM_KEEP_BYTES: u64 = 256 * 1024;
/// How much of `launch.log` a slimmed run keeps.
pub const LAUNCH_KEEP_BYTES: u64 = 64 * 1024;
/// The same two for the resolver attempts a landing names (I-02).
pub const ATTEMPT_STREAM_KEEP_BYTES: u64 = 64 * 1024;
pub const ATTEMPT_LAUNCH_KEEP_BYTES: u64 = 16 * 1024;
/// The largest report or SUMMARY copy that is kept (I-13).
const SMALL_FILE_MAX: u64 = 256 * 1024;

/// The longest a git read in a clone may run.
const READ_BUDGET: std::time::Duration = push::GIT_QUICK;

/// What slimming a run came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SlimOutcome {
    /// The clone and the bulky files are gone; `kept_bytes` is what the run dir holds
    /// now and `notes` are lines about anything deliberately left (a resolver clone the
    /// preservation rule kept).
    Slimmed { kept_bytes: u64, notes: Vec<String> },
    /// The preservation rule kept the clone, and nothing was deleted.
    Kept(String),
    /// Nothing was done (a landing in progress, a live run, a failure); a later scan
    /// retries.
    Skipped(String),
}

impl SlimOutcome {
    /// The line the landing prints for this outcome.
    pub fn line(&self) -> String {
        match self {
            SlimOutcome::Slimmed { kept_bytes, notes } => {
                let mut line = format!(
                    "removed the run's clone; kept its record ({})",
                    human_size(*kept_bytes)
                );
                for note in notes {
                    line.push('\n');
                    line.push_str(note);
                }
                line
            }
            SlimOutcome::Kept(reason) => format!("kept the run's clone: {reason}"),
            SlimOutcome::Skipped(reason) => format!(
                "warning: the run's clone was not removed ({reason}); the next `task list` retries"
            ),
        }
    }
}

/// `bytes` as `123 KiB` / `1.2 MiB`.
pub fn human_size(bytes: u64) -> String {
    const KIB: u64 = 1024;
    const MIB: u64 = 1024 * 1024;
    if bytes >= MIB {
        format!("{:.1} MiB", bytes as f64 / MIB as f64)
    } else {
        format!("{} KiB", bytes.div_ceil(KIB).max(1))
    }
}

/// The total size of everything under `path`, links counted as links.
pub fn tree_size(path: &Path) -> u64 {
    let Ok(meta) = fs::symlink_metadata(path) else {
        return 0;
    };
    let mut total = meta.len();
    if meta.file_type().is_dir() {
        for entry in fs::read_dir(path).into_iter().flatten().flatten() {
            total += tree_size(&entry.path());
        }
    }
    total
}

/// Cut a regular file down to its last `keep` bytes, at a line boundary, behind one
/// marker line `{"type":"gsdmm_truncated","dropped_bytes":N}` (I-14). The cut file is
/// written to a temp file in the same directory and renamed over the original, 0600.
/// Returns the number of bytes dropped, or `None` when the file is short enough or is
/// not a regular file (a symlink is never followed).
pub fn tail_cut(path: &Path, keep: u64) -> io::Result<Option<u64>> {
    let meta = fs::symlink_metadata(path)?;
    if !meta.file_type().is_file() {
        return Ok(None);
    }
    let len = meta.len();
    if len <= keep {
        return Ok(None);
    }
    let mut file = fs::OpenOptions::new()
        .read(true)
        .custom_flags(i32::try_from(rustix::fs::OFlags::NOFOLLOW.bits()).unwrap_or(0))
        .open(path)?;
    // One byte before the cut tells whether the cut falls on a line start.
    file.seek(SeekFrom::Start(len - keep - 1))?;
    let mut window = Vec::with_capacity(usize::try_from(keep + 1).unwrap_or(0));
    file.take(keep + 1).read_to_end(&mut window)?;
    if window.is_empty() {
        return Ok(None);
    }
    let body = if window[0] == b'\n' {
        1
    } else {
        window[1..]
            .iter()
            .position(|b| *b == b'\n')
            .map_or(window.len(), |at| at + 2)
    };
    let kept = &window[body.min(window.len())..];
    let dropped = len - kept.len() as u64;
    let marker = format!("{{\"type\":\"gsdmm_truncated\",\"dropped_bytes\":{dropped}}}\n");
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let mut tmp = tempfile::NamedTempFile::new_in(parent)?;
    tmp.write_all(marker.as_bytes())?;
    tmp.write_all(kept)?;
    fs::set_permissions(tmp.path(), fs::Permissions::from_mode(0o600))?;
    tmp.persist(path).map_err(|e| e.error)?;
    Ok(Some(dropped))
}

/// `git -C <clone> <args>` in an agent-written clone: hooks off, the host's global and
/// system config off (the same hardening `finalize` reads the clone with).
fn clone_git(clone: &Path, args: &[&str]) -> push::GitRun {
    let mut full: Vec<OsString> = vec!["-C".into(), clone.as_os_str().to_owned()];
    full.extend(args.iter().map(OsString::from));
    push::git_mirror_only(full, READ_BUDGET)
}

/// Whether deleting `clone` loses nothing (I-04): the planted-file gate is clean, and
/// the clone's `HEAD` and every `refs/heads/*` tip is either an ancestor of `base` (the
/// work the run started from) or contained in a branch of `project`. `Err` says what is
/// not preserved. File reads first, then hardened reads, exactly as `finalize` does.
pub fn clone_preserved(
    project: &Path,
    run_dir: &Path,
    clone: &Path,
    base: Option<&str>,
) -> Result<(), String> {
    let findings = git_relevant_planted(run_dir, clone);
    if !findings.is_empty() {
        return Err(format!(
            "{} git-relevant planted finding(s) in the clone; its commits cannot be checked safely",
            findings.len()
        ));
    }
    let mut tips: Vec<String> = Vec::new();
    let head = clone_git(
        clone,
        &["rev-parse", "--verify", "--quiet", "HEAD^{commit}"],
    );
    if head.success && push::is_hex_sha(&head.stdout) {
        tips.push(head.stdout);
    }
    let refs = clone_git(
        clone,
        &["for-each-ref", "--format=%(objectname)", "refs/heads/"],
    );
    if !refs.success {
        return Err(format!(
            "could not list the clone's branches: {}",
            refs.last_error
        ));
    }
    for line in refs.stdout.lines() {
        let sha = line.trim();
        if push::is_hex_sha(sha) && !tips.iter().any(|t| t == sha) {
            tips.push(sha.to_owned());
        }
    }
    for tip in tips {
        if let Some(base) = base {
            if clone_git(clone, &["merge-base", "--is-ancestor", &tip, base]).success {
                continue;
            }
        }
        let contained = push::git_in(
            project,
            &[
                "for-each-ref",
                "--contains",
                &tip,
                "--count=1",
                "--format=%(refname)",
                "refs/heads/",
            ],
            push::GIT_QUICK,
        );
        if !contained.success || contained.stdout.trim().is_empty() {
            return Err(format!(
                "commit {} in the clone is not in any branch of the project",
                &tip[..tip.len().min(12)]
            ));
        }
    }
    Ok(())
}

/// Remove `path` without following a symlink.
fn remove_entry(path: &Path) -> io::Result<()> {
    let meta = fs::symlink_metadata(path)?;
    if meta.file_type().is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    }
}

/// Keep only the regular, small `report.json` of a `report/` directory.
fn slim_report_dir(dir: &Path) -> io::Result<()> {
    let meta = fs::symlink_metadata(dir)?;
    if !meta.file_type().is_dir() {
        return remove_entry(dir);
    }
    for entry in fs::read_dir(dir)?.flatten() {
        let path = entry.path();
        let keep = entry.file_name() == REPORT_FILE
            && fs::symlink_metadata(&path)
                .is_ok_and(|m| m.file_type().is_file() && m.len() <= SMALL_FILE_MAX);
        if !keep {
            remove_entry(&path)?;
        }
    }
    Ok(())
}

/// Slim one resolver attempt dir: its small files stay (stream tail, launch log tail,
/// report), everything else (the land clone included) goes.
fn slim_attempt(dir: &Path) -> io::Result<()> {
    for entry in fs::read_dir(dir)?.flatten() {
        let path = entry.path();
        let name = entry.file_name();
        let regular = fs::symlink_metadata(&path).is_ok_and(|m| m.file_type().is_file());
        if name == STREAM_FILE && regular {
            tail_cut(&path, ATTEMPT_STREAM_KEEP_BYTES)?;
        } else if name == LAUNCH_LOG && regular {
            tail_cut(&path, ATTEMPT_LAUNCH_KEEP_BYTES)?;
        } else if name == REPORT_SUBDIR {
            slim_report_dir(&path)?;
        } else {
            remove_entry(&path)?;
        }
    }
    Ok(())
}

/// Copy the run's SUMMARY from its clone to `<run-dir>/summary.md` (at most 256 KiB,
/// 0600, temp file + rename), unless a copy is already there.
fn keep_summary(view: &OneoffView) -> io::Result<()> {
    let target = view.run_dir.join(KEPT_SUMMARY_FILE);
    if fs::symlink_metadata(&target).is_ok() {
        return Ok(());
    }
    let Some(rel) = view.record.summary_path.as_deref() else {
        return Ok(());
    };
    // `verified_summary` falls back to the kept copy, which does not exist yet.
    let Some(source) = verified_summary(&view.run_dir, rel) else {
        return Ok(());
    };
    let mut bytes = Vec::new();
    fs::File::open(&source)?
        .take(SMALL_FILE_MAX)
        .read_to_end(&mut bytes)?;
    let mut tmp = tempfile::NamedTempFile::new_in(&view.run_dir)?;
    tmp.write_all(&bytes)?;
    fs::set_permissions(tmp.path(), fs::Permissions::from_mode(0o600))?;
    tmp.persist(&target).map_err(|e| e.error)?;
    Ok(())
}

/// Slim the run `run_id` of `project` after a landing: take the run's land lock, slim and
/// release it (the guard unlinks `land.lock` while still holding it, I-15, pd7 D-05).
/// `note` is the reason recorded in `pruned`. Never an error: the outcome says what
/// happened.
pub fn slim_run(project: &Path, run_id: &str, note: &str) -> SlimOutcome {
    let Some(project_dir) = project_dir_for(project) else {
        return SlimOutcome::Skipped("no one-off directory for this project".to_owned());
    };
    if !super::store::valid_oneoff_run_id(run_id) {
        return SlimOutcome::Skipped(format!("{run_id:?} is not a one-off run id"));
    }
    let run_dir = project_dir.join(run_id);
    let lock = match take_land_lock(&run_dir, run_id) {
        Ok(lock) => lock,
        Err(why) => return SlimOutcome::Skipped(why),
    };
    let outcome = match read_view(&project_dir, run_id) {
        Some(view) => slim_locked(project, &view, note, true),
        None => SlimOutcome::Skipped("the run's record could not be read".to_owned()),
    };
    // Dropping the guard unlinks the lock file, only while it is held (I-15).
    drop(lock);
    outcome
}

/// [`slim_run`]'s work. The caller holds the run's land lock. `require_landing` demands a
/// recorded merge or push (a landed run); crash recovery passes `false`.
pub(crate) fn slim_locked(
    project: &Path,
    view: &OneoffView,
    note: &str,
    require_landing: bool,
) -> SlimOutcome {
    let record = &view.record;
    if record.status != RecordStatus::Finished || view.live.may_be_live() {
        return SlimOutcome::Skipped("the run may still be live".to_owned());
    }
    let merged = record.landed.iter().any(|l| l.kind.puts_work_on_target());
    if require_landing && record.landed.is_empty() {
        return SlimOutcome::Skipped("the run has not landed".to_owned());
    }
    let run_dir = &view.run_dir;

    // I-04: the run's clone is deleted only when nothing in it is lost.
    let clone = run_dir.join(WORK_DIR);
    let has_clone = fs::symlink_metadata(&clone).is_ok();
    if has_clone {
        if !fs::symlink_metadata(&clone).is_ok_and(|m| m.file_type().is_dir()) {
            return SlimOutcome::Kept("the run's clone is not a directory".to_owned());
        }
        if let Err(why) = clone_preserved(project, run_dir, &clone, record.base.as_deref()) {
            return SlimOutcome::Kept(why);
        }
    }

    // Nothing is derived from the stream after the cut: fold the spend first (T-o2r-10).
    let mut updated = record.clone();
    if updated.spend_usd.is_none() {
        updated.spend_usd = Some(spend_of_file(&run_dir.join(STREAM_FILE)));
    }
    if has_clone {
        if let Err(e) = keep_summary(view) {
            return SlimOutcome::Skipped(format!("could not keep the SUMMARY: {e}"));
        }
    }
    // I-03: `pruned` first, then the deletions.
    if updated.pruned.is_none() {
        updated.pruned = Some(Pruned {
            at: rfc3339(Utc::now()),
            note: note.to_owned(),
        });
    }
    if let Err(e) = write_record(run_dir, &updated) {
        return SlimOutcome::Skipped(format!("could not write run.json: {e}"));
    }

    let mut notes: Vec<String> = Vec::new();
    let failed = |what: &str, e: io::Error| SlimOutcome::Skipped(format!("{what}: {e}"));
    for (file, keep) in [
        (STREAM_FILE, STREAM_KEEP_BYTES),
        (LAUNCH_LOG, LAUNCH_KEEP_BYTES),
    ] {
        if let Err(e) = tail_cut(&run_dir.join(file), keep) {
            if e.kind() != io::ErrorKind::NotFound {
                return failed(&format!("cut {file}"), e);
            }
        }
    }

    // Resolver attempts: the ones the latest resolved landing names keep their small
    // files; with a merge landing the rest are superseded and go entirely; without one
    // (a push) a clone goes only when the preservation rule allows it (I-11).
    let named: Vec<String> = record
        .landed
        .iter()
        .rev()
        .find(|l| l.kind == LandingKind::Merged)
        .map(|l| l.attempts.clone())
        .unwrap_or_default();
    let land = run_dir.join(LAND_DIR);
    if fs::symlink_metadata(&land).is_ok_and(|m| m.file_type().is_dir()) {
        for entry in fs::read_dir(&land).into_iter().flatten().flatten() {
            let path = entry.path();
            let name = entry.file_name().to_string_lossy().into_owned();
            let is_attempt = name.parse::<u32>().is_ok()
                && fs::symlink_metadata(&path).is_ok_and(|m| m.file_type().is_dir());
            let result = if !is_attempt {
                remove_entry(&path)
            } else if named.iter().any(|n| *n == format!("{LAND_DIR}/{name}")) {
                slim_attempt(&path)
            } else if merged {
                remove_entry(&path)
            } else {
                let attempt_clone = path.join(WORK_DIR);
                let safe = !attempt_clone.exists()
                    || clone_preserved(project, run_dir, &attempt_clone, updated.head.as_deref())
                        .map_err(|why| {
                            notes.push(format!("kept the resolver clone {LAND_DIR}/{name}: {why}"));
                        })
                        .is_ok();
                if safe {
                    slim_attempt(&path)
                } else {
                    Ok(())
                }
            };
            if let Err(e) = result {
                return failed(&format!("clean {LAND_DIR}/{name}"), e);
            }
        }
    }

    // The clone goes before anything else: its planted-file baseline (deleted below)
    // is what a retried slim would need to judge it.
    if has_clone {
        if let Err(e) = remove_entry(&clone) {
            return failed("remove the clone", e);
        }
    }
    // The top level is an allowlist (I-02).
    let entries = match fs::read_dir(run_dir) {
        Ok(entries) => entries,
        Err(e) => return failed("read the run dir", e),
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().into_owned();
        let regular = fs::symlink_metadata(&path).is_ok_and(|m| m.file_type().is_file());
        let result = match name.as_str() {
            REPORT_SUBDIR => slim_report_dir(&path),
            RUN_FILE | LAND_DIR | LAND_LOCK_FILE | RUN_LOCK_FILE => Ok(()),
            INJECTIONS_FILE | KEPT_SUMMARY_FILE | STREAM_FILE | LAUNCH_LOG if regular => Ok(()),
            _ => remove_entry(&path),
        };
        if let Err(e) = result {
            return failed(&format!("remove {name}"), e);
        }
    }
    // I-15: a free run lock of a finished record is unlinked.
    if probe_at(&run_dir.join(RUN_LOCK_FILE)) == LockProbe::Free {
        let _ = fs::remove_file(run_dir.join(RUN_LOCK_FILE));
    }
    SlimOutcome::Slimmed {
        kept_bytes: tree_size(run_dir),
        notes,
    }
}

// ---------------------------------------------------------------- the recycle scan

/// How long a finished record is kept at the latest (I-01).
pub const RETENTION_DAYS: i64 = 30;
/// How many finished records a project keeps at the most (I-01).
pub const RETENTION_MAX_FINISHED: usize = 50;
/// An orphan run dir younger than this is never touched: the daemon creates its dir
/// before it takes its lock (T-o2r-04).
pub const ORPHAN_MIN_AGE: Duration = Duration::from_secs(3600);
/// How long `systemctl --user is-active` may take.
const SCOPE_PROBE_BUDGET: Duration = Duration::from_secs(10);

/// What the run's systemd scope unit says (I-09).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ScopeState {
    /// The record names no scope (an unsandboxed run, or one that died before it).
    NoScope,
    Inactive,
    Active,
    /// The probe did not answer, or the recorded unit is not the one this run would
    /// have: never read as inactive.
    Unprobeable,
}

/// Whether a run is dead and may be recovered (pure, D-B): the record says live, its
/// run lock is free, no landing holds its land lock and its scope, if it has one, is
/// not active. A lock probe that could not tell is [`LiveState::Unknown`] and never
/// qualifies.
pub(crate) fn is_dead(live: LiveState, land_lock_free: bool, scope: ScopeState) -> bool {
    live == LiveState::Crashed
        && land_lock_free
        && matches!(scope, ScopeState::NoScope | ScopeState::Inactive)
}

/// The scope state `systemctl --user is-active` printed (`None`: it did not answer).
pub(crate) fn scope_state_from(printed: Option<&str>) -> ScopeState {
    match printed.map(str::trim) {
        Some("active" | "activating" | "deactivating" | "reloading") => ScopeState::Active,
        Some("inactive" | "failed") => ScopeState::Inactive,
        _ => ScopeState::Unprobeable,
    }
}

/// Probe the scope the record names. Only a unit equal to the one rebuilt from the run
/// id and the project hash is passed to `systemctl` (I-09, T-o2r-08).
fn probe_scope(project: &Path, record: &super::store::RunRecord) -> ScopeState {
    let Some(unit) = &record.scope_unit else {
        return ScopeState::NoScope;
    };
    let expected = format!(
        "{}.scope",
        super::launch::scope_unit(&record.run_id, &hash8(project))
    );
    if *unit != expected {
        return ScopeState::Unprobeable;
    }
    let printed = run_bounded(
        "systemctl",
        &["--user".into(), "is-active".into(), unit.into()],
        None,
        &[],
        SCOPE_PROBE_BUDGET,
    )
    .map(|out| String::from_utf8_lossy(&out.stdout).into_owned());
    scope_state_from(printed.as_deref())
}

/// A finished record as the retention policy sees it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Aging {
    pub run_id: String,
    /// Terminal and clean: finished, not live, not landing, no clone and no
    /// `gsd-oneoff/<id>` branch. Only such a record ever ages out.
    pub clean: bool,
    pub age_days: i64,
}

/// The ids to remove (pure, I-01): of the finished records, newest first, the clean ones
/// older than `days` or beyond the newest `max`.
pub(crate) fn retention_picks(newest_first: &[Aging], days: i64, max: usize) -> Vec<String> {
    newest_first
        .iter()
        .enumerate()
        .filter(|(rank, aging)| aging.clean && (aging.age_days > days || *rank >= max))
        .map(|(_, aging)| aging.run_id.clone())
        .collect()
}

/// Why a finished run is slimmed now, if it is (pure, D-B): it landed (before this
/// change, or its slim was cut short), or a successor continues it and its work is in
/// `gsd-oneoff/<id>`.
pub(crate) fn leftover_slim_note(
    record: &super::store::RunRecord,
    clone_present: bool,
    tracked_at_head: bool,
) -> Option<String> {
    if record.pruned.is_some() && !clone_present {
        return None;
    }
    if let Some(landing) = record.landed.last() {
        return Some(match landing.kind {
            LandingKind::Merged | LandingKind::AlreadyLanded => {
                format!("removed after landing in {}", landing.detail)
            }
            LandingKind::Pushed => "removed after push".to_owned(),
        });
    }
    (record.resumed_by.is_some() && tracked_at_head)
        .then(|| "removed after it was resumed".to_owned())
}

/// What one scan did.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct RecycleReport {
    /// Dead runs finished as `crashed`.
    pub recovered: usize,
    /// Runs whose clone and bulky files were removed.
    pub slimmed: usize,
    /// Finished runs found already contained in their target branch and recorded as
    /// landed (quick 261103-pd7, D-03).
    pub already_landed: usize,
    /// Runs whose clone was kept because something in it is not in a project branch.
    pub kept_clones: usize,
    /// Orphan run dirs removed (their work, if any, preserved first).
    pub orphans_removed: usize,
    /// Orphan run dirs whose work was tracked as `gsd-oneoff/<dirname>`.
    pub orphans_preserved: usize,
    /// Stale temp, request, inbox and lock files removed.
    pub stale_files: usize,
    /// Finished records aged out.
    pub aged_out: usize,
    /// Lines for the user: what was kept and why, what failed.
    pub warnings: Vec<String>,
}

impl RecycleReport {
    /// One line naming what was done, or `None` when nothing was.
    pub fn summary(&self) -> Option<String> {
        let parts: Vec<String> = [
            (self.recovered, "recovered {} crashed run(s)"),
            (
                self.already_landed,
                "recorded {} run(s) already in their target as landed",
            ),
            (self.slimmed, "removed the clone of {} landed run(s)"),
            (self.orphans_removed, "removed {} orphan run dir(s)"),
            (self.orphans_preserved, "tracked the work of {} orphan(s)"),
            (self.stale_files, "removed {} stale file(s)"),
            (self.aged_out, "aged out {} old record(s)"),
            (self.kept_clones, "kept {} clone(s) with unpreserved work"),
        ]
        .iter()
        .filter(|(n, _)| *n > 0)
        .map(|(n, text)| text.replace("{}", &n.to_string()))
        .collect();
        (!parts.is_empty()).then(|| format!("one-off cleanup: {}", parts.join(", ")))
    }

    /// Fold `other` into `self` (the TUI scans every project).
    pub fn absorb(&mut self, other: RecycleReport) {
        self.recovered += other.recovered;
        self.slimmed += other.slimmed;
        self.already_landed += other.already_landed;
        self.kept_clones += other.kept_clones;
        self.orphans_removed += other.orphans_removed;
        self.orphans_preserved += other.orphans_preserved;
        self.stale_files += other.stale_files;
        self.aged_out += other.aged_out;
        self.warnings.extend(other.warnings);
    }
}

/// Remove the run dir `<project_dir>/<id>`: a real directory (never a symlink) whose
/// canonical parent is the project's one-off directory (T-o2r-03), as `discard` checks.
fn remove_run_dir(project_dir: &Path, id: &str) -> Result<(), String> {
    if !super::store::valid_oneoff_run_id(id) {
        return Err(format!("{id:?} is not a one-off run id"));
    }
    let run_dir = project_dir.join(id);
    let meta = fs::symlink_metadata(&run_dir).map_err(|e| e.to_string())?;
    if !meta.file_type().is_dir() {
        return Err(format!("{} is not a directory", run_dir.display()));
    }
    let canonical = fs::canonicalize(&run_dir).map_err(|e| e.to_string())?;
    let parent = fs::canonicalize(project_dir).map_err(|e| e.to_string())?;
    if canonical.parent() != Some(parent.as_path()) {
        return Err(format!(
            "{} does not sit directly under the project's one-off directory",
            run_dir.display()
        ));
    }
    fs::remove_dir_all(&canonical).map_err(|e| format!("remove {}: {e}", canonical.display()))
}

/// Whether a lock file may be treated as free: absent or not held.
fn lock_is_free(path: &Path) -> bool {
    matches!(probe_at(path), LockProbe::Free | LockProbe::Absent)
}

/// Scan one project's one-off directory and put it in order (quick 261103-o2r, D-B):
/// finish dead runs as `crashed` and preserve their work, slim landed runs that still
/// carry a clone, remove stale files, orphan dirs and aged-out records. Blocking and
/// cheap by default (a directory listing and lock probes; git only for a run that needs
/// it). It never touches a run whose lock could not be probed, whose land lock is held
/// or whose scope may be active, and it deletes no clone whose commits are not in a
/// branch of the project ([`clone_preserved`]).
pub fn recycle_project(project: &Path) -> RecycleReport {
    let mut report = RecycleReport::default();
    let Ok(project) = fs::canonicalize(project) else {
        return report;
    };
    let Some(project_dir) = project_dir_for(&project) else {
        return report;
    };
    if !fs::symlink_metadata(&project_dir).is_ok_and(|m| m.file_type().is_dir()) {
        return report;
    }
    for id in run_ids_newest_first(&project_dir) {
        match read_view(&project_dir, &id) {
            Some(view) => tend_run(&project, &project_dir, &view, &mut report),
            None => tend_orphan(&project, &project_dir, &id, &mut report),
        }
    }
    age_out(&project, &project_dir, &mut report);
    report
}

/// A run with a valid record: recover it when it is dead, otherwise put a finished one
/// in order.
fn tend_run(project: &Path, project_dir: &Path, view: &OneoffView, report: &mut RecycleReport) {
    let id = view.record.run_id.clone();
    let run_dir = &view.run_dir;
    if view.live == LiveState::Crashed {
        let scope = probe_scope(project, &view.record);
        if is_dead(view.live, lock_is_free(&land_lock_path(run_dir)), scope) {
            recover_dead(project, project_dir, &id, report);
        }
        return;
    }
    if view.live != LiveState::Finished || view.landing {
        return;
    }
    if let Some(branch) = already_landed_candidate(&view.record) {
        if tend_contained(project, project_dir, &id, &branch, report) {
            return;
        }
    }
    let clone_present = fs::symlink_metadata(run_dir.join(WORK_DIR)).is_ok();
    let tracked_at_head = view
        .record
        .head
        .as_deref()
        .is_some_and(|head| ref_commit(project, &tracked_ref(&id)).as_deref() == Some(head));
    if let Some(note) = leftover_slim_note(&view.record, clone_present, tracked_at_head) {
        match slim_run(project, &id, &note) {
            SlimOutcome::Slimmed { notes, .. } => {
                report.slimmed += 1;
                report.warnings.extend(notes);
            }
            SlimOutcome::Kept(why) => {
                report.kept_clones += 1;
                report.warnings.push(format!("{id}: kept the clone: {why}"));
            }
            SlimOutcome::Skipped(why) => report.warnings.push(format!("{id}: {why}")),
        }
        return;
    }
    tidy_finished(project, view, report);
}

/// The branch a finished run's head may already be in, if the scan should look (pure,
/// quick 261103-pd7, D-03): a finished run with commits and a recorded head, not paused,
/// not continued by another run, with no landing that put its work on the target, and a
/// recorded target branch. A detached or legacy target has no branch to look in.
pub(crate) fn already_landed_candidate(record: &super::store::RunRecord) -> Option<String> {
    if record.status != RecordStatus::Finished
        || record.head.is_none()
        || record.commits.unwrap_or(0) == 0
        || record.resumed_by.is_some()
        || matches!(record.outcome, Some(AgentOutcome::Paused))
        || record.landed.iter().any(|l| l.kind.puts_work_on_target())
    {
        return None;
    }
    match record.land_target() {
        super::store::LandTarget::Branch(branch) => Some(branch),
        super::store::LandTarget::Detached { .. } | super::store::LandTarget::Legacy => None,
    }
}

/// The branch a resumed run's work reached through a run that continues it, if the scan
/// should look (pure, quick 261103-qqm, I-02): a finished run with a recorded head that a
/// successor continues, no landing of its own that put work on the target, and a run later
/// in its chain (`successors`, in chain order) with such a landing. Returns the branch of
/// the first such landing's latest entry, when it is a plausible branch name. Every link is
/// confirmed from both sides. A pause and a commit count of 0 do not disqualify the run: once
/// the chain has landed, its clone is as dead as any landed run's. A push puts nothing on a
/// project branch and so never qualifies.
pub(crate) fn successor_landing_branch(
    record: &super::store::RunRecord,
    successors: &[super::store::RunRecord],
) -> Option<String> {
    if record.status != RecordStatus::Finished
        || record.head.is_none()
        || record.landed.iter().any(|l| l.kind.puts_work_on_target())
    {
        return None;
    }
    let mut expected = record.resumed_by.clone()?;
    let mut previous = record.run_id.as_str();
    for next in successors {
        if next.run_id != expected || next.resumed_from.as_deref() != Some(previous) {
            return None;
        }
        if let Some(landing) = next
            .landed
            .iter()
            .rev()
            .find(|l| l.kind.puts_work_on_target())
        {
            return push::plausible_branch(&landing.detail).then(|| landing.detail.clone());
        }
        expected = next.resumed_by.clone()?;
        previous = next.run_id.as_str();
    }
    None
}

/// The branch the run's head may already be in, by either rule: a plain run
/// ([`already_landed_candidate`]) or a predecessor whose successor landed
/// ([`successor_landing_branch`]).
pub(crate) fn contained_candidate(
    project_dir: &Path,
    record: &super::store::RunRecord,
) -> Option<String> {
    already_landed_candidate(record)
        .or_else(|| successor_landing_branch(record, &resume_successors(project_dir, record)))
}

/// After the run `run_id` landed, close out every predecessor in its resume chain whose
/// head is in the branch a later run landed in (quick 261103-qqm, I-04): the same routine
/// the scan uses, with each predecessor's land lock taken without waiting (a held lock
/// leaves that predecessor to the next scan). Returns the lines to print. Never an error.
pub fn close_out_predecessors(project: &Path, run_id: &str) -> Vec<String> {
    let mut report = RecycleReport::default();
    let Some(project_dir) = project_dir_for(project) else {
        return report.warnings;
    };
    let Some(view) = read_view(&project_dir, run_id) else {
        return report.warnings;
    };
    for predecessor in resume_predecessors(&project_dir, &view.record) {
        let id = predecessor.record.run_id.clone();
        let Some(branch) = contained_candidate(&project_dir, &predecessor.record) else {
            continue;
        };
        let slimmed = report.slimmed;
        if tend_contained(project, &project_dir, &id, &branch, &mut report)
            && report.slimmed > slimmed
        {
            report.warnings.push(format!("{id}: removed its clone"));
        }
    }
    report.warnings
}

/// Record the run `id` as already landed when its head is an ancestor of `branch`'s tip,
/// then slim it and delete its tracked branch (pd7 D-03). Takes the land lock without
/// waiting. Returns `true` when the run was dealt with this scan (recorded, or its lock is
/// held by a landing), `false` when it is not contained and the normal tidy applies.
fn tend_contained(
    project: &Path,
    project_dir: &Path,
    id: &str,
    branch: &str,
    report: &mut RecycleReport,
) -> bool {
    let Some(view) = read_view(project_dir, id) else {
        return false;
    };
    let Some(head) = view.record.head.clone() else {
        return false;
    };
    let Some(tip) = ref_commit(project, &format!("refs/heads/{branch}")) else {
        return false;
    };
    if !push::git_in(
        project,
        &["merge-base", "--is-ancestor", &head, &tip],
        push::GIT_QUICK,
    )
    .success
    {
        return false;
    }
    let Some(lock) = try_land_lock(&view.run_dir, id) else {
        return true;
    };
    // Judged again under the lock: the run and the branch may have changed.
    let Some(view) = read_view(project_dir, id) else {
        return true;
    };
    let tip = ref_commit(project, &format!("refs/heads/{branch}")).unwrap_or(tip);
    if view.live != LiveState::Finished
        || contained_candidate(project_dir, &view.record).as_deref() != Some(branch)
    {
        return true;
    }
    let mut record = view.record.clone();
    if let Err(why) = record_already_landed(project, &view.run_dir, &mut record, branch, &tip) {
        report.warnings.push(format!("{id}: {why}"));
        return true;
    }
    report.already_landed += 1;
    report.warnings.push(format!(
        "{id}: its head is already in {branch}; recorded as landed"
    ));
    if let Some(view) = read_view(project_dir, id) {
        match slim_locked(
            project,
            &view,
            &format!("removed after landing in {branch}"),
            true,
        ) {
            SlimOutcome::Slimmed { notes, .. } => {
                report.slimmed += 1;
                report.warnings.extend(notes);
            }
            SlimOutcome::Kept(why) => {
                report.kept_clones += 1;
                report.warnings.push(format!("{id}: kept the clone: {why}"));
            }
            SlimOutcome::Skipped(why) => report.warnings.push(format!("{id}: {why}")),
        }
    }
    drop(lock);
    true
}

/// Stale small files of a finished run that keeps its clone, and the resolver clones
/// of its failed attempts whose commits are all preserved (I-11). Under the land lock.
fn tidy_finished(project: &Path, view: &OneoffView, report: &mut RecycleReport) {
    let run_dir = &view.run_dir;
    let id = &view.record.run_id;
    let Ok(_lock) = take_land_lock(run_dir, id) else {
        return;
    };
    let removed = std::cell::Cell::new(0_usize);
    let remove = |path: &Path| {
        if fs::symlink_metadata(path).is_ok() && remove_entry(path).is_ok() {
            removed.set(removed.get() + 1);
        }
    };
    if let Ok(entries) = fs::read_dir(run_dir) {
        for entry in entries.flatten() {
            if entry.file_name().to_string_lossy().starts_with(".tmp") {
                remove(&entry.path());
            }
        }
    }
    remove(&run_dir.join(INBOX_DIR));
    // A stop request left in a finished run's dir is stale: nothing is left to stop.
    if super::cancel::read_request(run_dir)
        .ok()
        .flatten()
        .is_some()
    {
        super::cancel::clear_request(run_dir);
        removed.set(removed.get() + 1);
    }
    let land = run_dir.join(LAND_DIR);
    if super::cancel::read_request(&land).ok().flatten().is_some() {
        super::cancel::clear_request(&land);
        removed.set(removed.get() + 1);
    }
    if probe_at(&run_dir.join(RUN_LOCK_FILE)) == LockProbe::Free {
        remove(&run_dir.join(RUN_LOCK_FILE));
    }
    for entry in fs::read_dir(&land).into_iter().flatten().flatten() {
        let clone = entry.path().join(WORK_DIR);
        let is_attempt = entry.file_name().to_string_lossy().parse::<u32>().is_ok()
            && fs::symlink_metadata(entry.path()).is_ok_and(|m| m.file_type().is_dir());
        if is_attempt && fs::symlink_metadata(&clone).is_ok_and(|m| m.file_type().is_dir()) {
            match clone_preserved(project, run_dir, &clone, view.record.head.as_deref()) {
                Ok(()) => remove(&clone),
                Err(why) => report.warnings.push(format!(
                    "{id}: kept the resolver clone {}: {why}",
                    entry.file_name().to_string_lossy()
                )),
            }
        }
    }
    report.stale_files += removed.get();
}

/// Finish a dead run as `crashed`, preserve its work and slim it (D-B). Everything runs
/// under the run's land lock, and the death is judged again under it.
fn recover_dead(project: &Path, project_dir: &Path, id: &str, report: &mut RecycleReport) {
    let run_dir = project_dir.join(id);
    let Ok(lock) = take_land_lock(&run_dir, id) else {
        return;
    };
    let Some(view) = read_view(project_dir, id) else {
        return;
    };
    // `view.landing` is our own lock; the run lock and the scope are what matter.
    if !is_dead(view.live, true, probe_scope(project, &view.record)) {
        return;
    }
    let mut record = view.record.clone();
    let was = if record.status == RecordStatus::Preparing {
        "preparing"
    } else {
        "running"
    };
    let outcome = AgentOutcome::Crashed {
        detail: format!("the launcher died while the run was {was}"),
    };
    match read_validated(&run_dir.join(REPORT_SUBDIR).join(REPORT_FILE)) {
        Ok(agent_report) => {
            record.report = Some(agent_report);
            record.report_source = Some(ReportSource::Agent);
        }
        Err(_) => {
            record.report = Some(synthesize(
                &outcome,
                record.max_budget_usd,
                record.cap_minutes,
            ));
            record.report_source = Some(ReportSource::Synthesized {
                reason: "the launcher died before the run finished".to_owned(),
            });
        }
    }
    record.outcome = Some(outcome);
    record.status = RecordStatus::Finished;
    record.ended_at = Some(rfc3339(Utc::now()));
    record
        .warnings
        .push("the launcher died before the run finished; recovered by a later scan".to_owned());
    record.spend_usd =
        Some(record.spend_usd.unwrap_or(0.0) + spend_of_file(&run_dir.join(STREAM_FILE)));
    let work = run_dir.join(WORK_DIR);
    if fs::symlink_metadata(&work).is_ok_and(|m| m.file_type().is_dir()) {
        settle_work(project, &run_dir, &work, id, record.mode, &mut record);
    } else {
        record.tracking_note = Some("no clone: nothing to track".to_owned());
    }
    if let Err(e) = write_record(&run_dir, &record) {
        report
            .warnings
            .push(format!("{id}: could not record the crash: {e}"));
        return;
    }
    report.recovered += 1;
    let Some(view) = read_view(project_dir, id) else {
        return;
    };
    match slim_locked(project, &view, "removed after crash recovery", false) {
        SlimOutcome::Slimmed { notes, .. } => {
            report.slimmed += 1;
            report.warnings.extend(notes);
        }
        SlimOutcome::Kept(why) => {
            report.kept_clones += 1;
            report.warnings.push(format!("{id}: kept the clone: {why}"));
        }
        SlimOutcome::Skipped(why) => report.warnings.push(format!("{id}: {why}")),
    }
    drop(lock);
}

/// The newest of the modification times of `paths` that exist.
fn newest_mtime(paths: &[PathBuf]) -> Option<SystemTime> {
    paths
        .iter()
        .filter_map(|p| fs::symlink_metadata(p).ok()?.modified().ok())
        .max()
}

/// A run-id-named directory without a valid `run.json` (I-10): left alone while any lock
/// in it is held or it is younger than an hour; otherwise its work is preserved as
/// `gsd-oneoff/<dirname>` (behind the planted-file gate) and the directory removed. A
/// directory whose work cannot be preserved is kept and a warning says why.
fn tend_orphan(project: &Path, project_dir: &Path, id: &str, report: &mut RecycleReport) {
    let dir = project_dir.join(id);
    if !fs::symlink_metadata(&dir).is_ok_and(|m| m.file_type().is_dir()) {
        return;
    }
    let locks = [
        dir.join(RUN_LOCK_FILE),
        dir.join(LAND_LOCK_FILE),
        project_dir.join(super::LOCK_FILE),
    ];
    // The legacy per-project lock judges only a dir without a run lock of its own.
    let judged = if fs::symlink_metadata(&locks[0]).is_ok() {
        &locks[..2]
    } else {
        &locks[1..2]
    };
    if !judged.iter().all(|l| lock_is_free(l)) {
        return;
    }
    let newest = newest_mtime(&[dir.clone(), dir.join(RUN_LOCK_FILE), dir.join(LAUNCH_LOG)]);
    let old_enough = newest
        .and_then(|t| SystemTime::now().duration_since(t).ok())
        .is_some_and(|age| age > ORPHAN_MIN_AGE);
    if !old_enough {
        return;
    }
    let work = dir.join(WORK_DIR);
    if fs::symlink_metadata(&work).is_ok_and(|m| m.file_type().is_dir()) {
        let findings = git_relevant_planted(&dir, &work);
        if !findings.is_empty() {
            report.warnings.push(format!(
                "{id}: kept the orphan dir: {} git-relevant planted finding(s) in its clone",
                findings.len()
            ));
            return;
        }
        if clone_preserved(project, &dir, &work, None).is_err() {
            let tip = clone_git(
                &work,
                &[
                    "rev-parse",
                    "--verify",
                    "--quiet",
                    &format!("{}^{{commit}}", tracked_ref(id)),
                ],
            );
            if tip.success && push::is_hex_sha(&tip.stdout) {
                match track_branch(project, &work, id, &tip.stdout) {
                    Ok(_) => report.orphans_preserved += 1,
                    Err(why) => {
                        report
                            .warnings
                            .push(format!("{id}: kept the orphan dir: {why}"));
                        return;
                    }
                }
            }
            if let Err(why) = clone_preserved(project, &dir, &work, None) {
                report
                    .warnings
                    .push(format!("{id}: kept the orphan dir: {why}"));
                return;
            }
        }
    }
    match remove_run_dir(project_dir, id) {
        Ok(()) => report.orphans_removed += 1,
        Err(why) => report
            .warnings
            .push(format!("{id}: could not remove the orphan dir: {why}")),
    }
}

/// Retention (I-01): finished records that are terminal and clean age out after
/// [`RETENTION_DAYS`] days or beyond the newest [`RETENTION_MAX_FINISHED`].
fn age_out(project: &Path, project_dir: &Path, report: &mut RecycleReport) {
    let now = Utc::now();
    let agings: Vec<Aging> = read_runs_in(project_dir, None)
        .into_iter()
        .filter(|v| v.live == LiveState::Finished)
        .map(|v| {
            let id = v.record.run_id.clone();
            let clone = fs::symlink_metadata(v.run_dir.join(WORK_DIR)).is_ok();
            let branch = ref_commit(project, &tracked_ref(&id)).is_some();
            let stamp = v.record.ended_at.as_deref().unwrap_or(&v.record.started_at);
            let age_days = chrono::DateTime::parse_from_rfc3339(stamp)
                .map(|t| (now - t.with_timezone(&Utc)).num_days())
                .unwrap_or(0);
            Aging {
                run_id: id,
                clean: !v.landing && !clone && !branch,
                age_days,
            }
        })
        .collect();
    for id in retention_picks(&agings, RETENTION_DAYS, RETENTION_MAX_FINISHED) {
        let run_dir = project_dir.join(&id);
        let Ok(_lock) = take_land_lock(&run_dir, &id) else {
            continue;
        };
        // Judged again under the lock: nothing may have appeared since the listing.
        let still_clean = read_view(project_dir, &id).is_some_and(|v| {
            v.live == LiveState::Finished
                && !run_dir.join(WORK_DIR).exists()
                && ref_commit(project, &tracked_ref(&id)).is_none()
        });
        if still_clean {
            match remove_run_dir(project_dir, &id) {
                Ok(()) => report.aged_out += 1,
                Err(why) => report.warnings.push(format!("{id}: {why}")),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn the_tail_cut_keeps_the_last_bytes_from_a_line_start_behind_a_marker() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("stream.jsonl");
        let lines: Vec<String> = (0..50).map(|i| format!("line-{i:03}-xxxxxxxx")).collect();
        let text = format!("{}\n", lines.join("\n"));
        fs::write(&path, &text).unwrap();
        let dropped = tail_cut(&path, 100).unwrap().expect("cut");
        let cut = fs::read_to_string(&path).unwrap();
        let mut rows = cut.lines();
        let marker = rows.next().unwrap();
        assert_eq!(
            marker,
            format!("{{\"type\":\"gsdmm_truncated\",\"dropped_bytes\":{dropped}}}")
        );
        let kept: Vec<&str> = rows.collect();
        assert!(kept.iter().all(|l| l.starts_with("line-")), "{kept:?}");
        assert_eq!(kept.last().copied(), Some(lines[49].as_str()));
        let kept_bytes: usize = kept.iter().map(|l| l.len() + 1).sum();
        assert!(kept_bytes <= 100, "{kept_bytes}");
        assert_eq!(dropped as usize + kept_bytes, text.len());
        assert_eq!(
            fs::metadata(&path).unwrap().permissions().mode() & 0o777,
            0o600
        );
    }

    #[test]
    fn a_short_file_or_a_symlink_is_left_alone() {
        let dir = tempfile::TempDir::new().unwrap();
        let short = dir.path().join("short.log");
        fs::write(&short, "a\nb\n").unwrap();
        assert_eq!(tail_cut(&short, 100).unwrap(), None);
        assert_eq!(fs::read_to_string(&short).unwrap(), "a\nb\n");
        let big = dir.path().join("big.log");
        fs::write(&big, "x\n".repeat(500)).unwrap();
        let link = dir.path().join("link.log");
        std::os::unix::fs::symlink(&big, &link).unwrap();
        assert_eq!(tail_cut(&link, 10).unwrap(), None);
        assert_eq!(fs::read_to_string(&big).unwrap(), "x\n".repeat(500));
    }

    /// The repo's own hardened git, with a fixed identity (no `Command` here: process
    /// spawns are confined to the launcher's seam).
    fn git(dir: &Path, args: &[&str]) -> String {
        let mut full = vec!["-c", "user.name=t", "-c", "user.email=t@example.com"];
        full.extend_from_slice(args);
        let run = push::git_in(dir, &full, push::GIT_QUICK);
        assert!(run.success, "git {args:?}: {}", run.last_error);
        run.stdout
    }

    /// A project with one commit, a clone of it in `<run>/work`, and the run dir.
    fn fixture() -> (tempfile::TempDir, PathBuf, PathBuf, PathBuf, String) {
        let root = tempfile::TempDir::new().unwrap();
        let project = root.path().join("project");
        fs::create_dir(&project).unwrap();
        git(&project, &["init", "-q", "-b", "main"]);
        fs::write(project.join("a.txt"), "a").unwrap();
        git(&project, &["add", "-A"]);
        git(&project, &["commit", "-qm", "init"]);
        let base = git(&project, &["rev-parse", "HEAD"]);
        let run_dir = root.path().join("run");
        fs::create_dir(&run_dir).unwrap();
        let clone = run_dir.join("work");
        git(
            root.path(),
            &[
                "clone",
                "-q",
                "--no-hardlinks",
                &project.display().to_string(),
                &clone.display().to_string(),
            ],
        );
        // A baseline of the clone, so the planted gate runs in normal (not strict) mode.
        push::planted::save_baseline_if_absent(&run_dir, &push::planted::snapshot(&clone)).unwrap();
        (root, project, run_dir, clone, base)
    }

    #[test]
    fn a_clone_with_a_commit_outside_the_project_is_kept_and_a_contained_one_is_not() {
        let (_root, project, run_dir, clone, base) = fixture();
        // Nothing beyond the base: preserved.
        assert_eq!(
            clone_preserved(&project, &run_dir, &clone, Some(&base)),
            Ok(())
        );
        // A commit on the clone's work branch that no project branch has.
        git(&clone, &["switch", "-q", "-c", "gsd-oneoff/x"]);
        fs::write(clone.join("b.txt"), "b").unwrap();
        git(&clone, &["add", "-A"]);
        git(&clone, &["commit", "-qm", "work"]);
        let why = clone_preserved(&project, &run_dir, &clone, Some(&base)).unwrap_err();
        assert!(why.contains("not in any branch of the project"), "{why}");
        // Tracked in the project: preserved.
        git(
            &project,
            &[
                "fetch",
                "-q",
                &clone.display().to_string(),
                "refs/heads/gsd-oneoff/x:refs/heads/gsd-oneoff/x",
            ],
        );
        assert_eq!(
            clone_preserved(&project, &run_dir, &clone, Some(&base)),
            Ok(())
        );
    }

    #[test]
    fn a_planted_file_keeps_the_clone() {
        let (_root, project, run_dir, clone, base) = fixture();
        fs::write(clone.join(".gitattributes"), "* filter=x\n").unwrap();
        let why = clone_preserved(&project, &run_dir, &clone, Some(&base)).unwrap_err();
        assert!(why.contains("planted"), "{why}");
    }

    #[test]
    fn sizes_read_in_kib_and_mib() {
        assert_eq!(human_size(0), "1 KiB");
        assert_eq!(human_size(2048), "2 KiB");
        assert_eq!(human_size(3 * 1024 * 1024), "3.0 MiB");
    }

    // ------------------------------------------------------------ the recycle scan

    #[test]
    fn only_a_dead_run_is_recovered() {
        let free = |live, land_free, scope| is_dead(live, land_free, scope);
        assert!(free(LiveState::Crashed, true, ScopeState::NoScope));
        assert!(free(LiveState::Crashed, true, ScopeState::Inactive));
        // A probe that could not tell, a held land lock, an active, unprobeable or
        // mismatched scope, and a finished run are never touched.
        assert!(!free(LiveState::Unknown, true, ScopeState::NoScope));
        assert!(!free(LiveState::Crashed, false, ScopeState::NoScope));
        assert!(!free(LiveState::Crashed, true, ScopeState::Active));
        assert!(!free(LiveState::Crashed, true, ScopeState::Unprobeable));
        assert!(!free(LiveState::Finished, true, ScopeState::NoScope));
        assert!(!free(LiveState::Running, true, ScopeState::NoScope));
        assert!(!free(LiveState::Preparing, true, ScopeState::NoScope));
    }

    #[test]
    fn the_scope_probe_reads_systemctl_words_and_distrusts_anything_else() {
        for active in ["active", "activating\n", "deactivating", "reloading"] {
            assert_eq!(
                scope_state_from(Some(active)),
                ScopeState::Active,
                "{active}"
            );
        }
        for gone in ["inactive\n", "failed"] {
            assert_eq!(scope_state_from(Some(gone)), ScopeState::Inactive, "{gone}");
        }
        assert_eq!(scope_state_from(Some("")), ScopeState::Unprobeable);
        assert_eq!(scope_state_from(Some("what")), ScopeState::Unprobeable);
        assert_eq!(scope_state_from(None), ScopeState::Unprobeable);
    }

    #[test]
    fn a_scope_unit_that_is_not_this_runs_is_never_probed() {
        let mut record = super::super::store::tests::record();
        assert_eq!(probe_scope(Path::new("/p"), &record), ScopeState::NoScope);
        record.scope_unit = Some("--user-evil.scope".to_owned());
        assert_eq!(
            probe_scope(Path::new("/p"), &record),
            ScopeState::Unprobeable
        );
    }

    fn aging(id: &str, clean: bool, age_days: i64) -> Aging {
        Aging {
            run_id: id.to_owned(),
            clean,
            age_days,
        }
    }

    #[test]
    fn retention_ages_out_only_clean_records_by_age_or_rank() {
        let list = [
            aging("a", true, 1),
            aging("b", false, 90),
            aging("c", true, 31),
            aging("d", true, 2),
            aging("e", false, 3),
            aging("f", true, 4),
        ];
        // Old and clean: c. Beyond the newest 3 and clean: d?, f (index 3 and 5), never e.
        assert_eq!(retention_picks(&list, 30, 100), ["c"]);
        assert_eq!(retention_picks(&list, 30, 3), ["c", "d", "f"]);
        // A record with a clone or a branch (not clean) is never picked, however old.
        assert!(retention_picks(&[aging("x", false, 400)], 30, 0).is_empty());
    }

    #[test]
    fn a_landed_run_that_still_has_a_clone_is_slimmed_and_an_unlanded_one_is_not() {
        let mut record = super::super::store::tests::record();
        assert_eq!(leftover_slim_note(&record, true, false), None, "unlanded");
        record.landed.push(super::super::store::Landing::plain(
            LandingKind::Merged,
            "2026-11-03T10:00:00Z".to_owned(),
            "main".to_owned(),
        ));
        assert_eq!(
            leftover_slim_note(&record, true, false).as_deref(),
            Some("removed after landing in main")
        );
        // Landed before this change: no clone, never pruned -> the record is marked.
        assert!(leftover_slim_note(&record, false, false).is_some());
        // Pruned and clean: nothing to do; pruned with leftovers: finish the slim.
        record.pruned = Some(Pruned {
            at: "t".to_owned(),
            note: "n".to_owned(),
        });
        assert_eq!(leftover_slim_note(&record, false, false), None);
        assert!(leftover_slim_note(&record, true, false).is_some());
        // A resumed run is slimmed only when its work is in the tracked branch.
        let mut resumed = super::super::store::tests::record();
        resumed.resumed_by = Some("oneoff-20261103-150000".to_owned());
        assert_eq!(leftover_slim_note(&resumed, true, false), None);
        assert!(leftover_slim_note(&resumed, true, true).is_some());
    }

    #[test]
    fn the_report_names_what_was_done_and_nothing_when_idle() {
        let mut report = RecycleReport::default();
        assert_eq!(report.summary(), None);
        report.recovered = 1;
        report.slimmed = 2;
        report.kept_clones = 1;
        let text = report.summary().unwrap();
        assert!(text.contains("recovered 1 crashed run(s)"), "{text}");
        assert!(
            text.contains("removed the clone of 2 landed run(s)"),
            "{text}"
        );
        assert!(text.contains("kept 1 clone(s)"), "{text}");
        let other = RecycleReport {
            aged_out: 3,
            warnings: vec!["w".to_owned()],
            ..RecycleReport::default()
        };
        report.absorb(other);
        assert_eq!(report.aged_out, 3);
        assert_eq!(report.warnings, ["w"]);
    }

    /// A finished, landed record of a run whose clone is `<run_dir>/work`.
    fn landed_view(run_dir: &Path, base: &str) -> OneoffView {
        let mut record = super::super::store::tests::record();
        record.status = RecordStatus::Finished;
        record.base = Some(base.to_owned());
        record.head = Some(base.to_owned());
        record.landed.push(super::super::store::Landing::plain(
            LandingKind::Merged,
            "2026-11-03T10:00:00Z".to_owned(),
            "main".to_owned(),
        ));
        OneoffView {
            record,
            live: LiveState::Finished,
            run_dir: run_dir.to_path_buf(),
            landing: false,
            land_attempts: Vec::new(),
            summary_file: None,
        }
    }

    #[test]
    fn slimming_refuses_a_clone_whose_commits_are_not_in_the_project() {
        let (_root, project, run_dir, clone, base) = fixture();
        git(&clone, &["switch", "-q", "-c", "gsd-oneoff/x"]);
        fs::write(clone.join("b.txt"), "b").unwrap();
        git(&clone, &["add", "-A"]);
        git(&clone, &["commit", "-qm", "work"]);
        fs::write(run_dir.join("stream.jsonl"), "{}\n").unwrap();
        let view = landed_view(&run_dir, &base);
        let outcome = slim_locked(&project, &view, "removed after landing in main", true);
        assert!(matches!(outcome, SlimOutcome::Kept(_)), "{outcome:?}");
        assert!(clone.join("b.txt").is_file(), "nothing was deleted");
        assert!(!run_dir.join("run.json").exists(), "nothing was written");
        // Once the work is in a project branch the same call slims the run.
        git(
            &project,
            &[
                "fetch",
                "-q",
                &clone.display().to_string(),
                "refs/heads/gsd-oneoff/x:refs/heads/gsd-oneoff/x",
            ],
        );
        let outcome = slim_locked(&project, &view, "removed after landing in main", true);
        assert!(
            matches!(outcome, SlimOutcome::Slimmed { .. }),
            "{outcome:?}"
        );
        assert!(!clone.exists());
        assert!(run_dir.join("stream.jsonl").is_file());
        let kept = super::super::store::read_record(&run_dir).unwrap();
        assert!(kept.pruned.is_some());
    }

    #[test]
    fn a_run_that_has_not_landed_is_not_slimmed_unless_recovering() {
        let (_root, project, run_dir, _clone, base) = fixture();
        let mut view = landed_view(&run_dir, &base);
        view.record.landed.clear();
        assert!(matches!(
            slim_locked(&project, &view, "n", true),
            SlimOutcome::Skipped(_)
        ));
        assert!(matches!(
            slim_locked(&project, &view, "n", false),
            SlimOutcome::Slimmed { .. }
        ));
    }

    #[test]
    fn a_resolver_clone_of_an_unlanded_run_goes_only_when_its_commits_are_preserved() {
        let (_root, project, run_dir, _clone, base) = fixture();
        let mut view = landed_view(&run_dir, &base);
        view.record.landed.clear();
        let attempt = run_dir.join("land/1");
        fs::create_dir_all(&attempt).unwrap();
        let land_clone = attempt.join("work");
        git(
            &run_dir,
            &[
                "clone",
                "-q",
                "--no-hardlinks",
                &project.display().to_string(),
                &land_clone.display().to_string(),
            ],
        );
        push::planted::save_baseline_if_absent(
            &run_dir,
            &push::planted::snapshot(&run_dir.join("work")),
        )
        .unwrap();
        // The attempt's merge commit exists only in the land clone.
        fs::write(land_clone.join("m.txt"), "merge").unwrap();
        git(&land_clone, &["add", "-A"]);
        git(&land_clone, &["commit", "-qm", "merge"]);
        let mut report = RecycleReport::default();
        tidy_finished(&project, &view, &mut report);
        assert!(land_clone.is_dir(), "an unpreserved commit keeps the clone");
        assert!(report.warnings.iter().any(|w| w.contains("resolver clone")));
        // Reset the attempt to the project's commit: nothing is lost, the clone goes
        // and the attempt dir stays.
        git(&land_clone, &["reset", "-q", "--hard", &base]);
        git(&land_clone, &["reflog", "expire", "--expire=now", "--all"]);
        tidy_finished(&project, &view, &mut report);
        assert!(!land_clone.exists());
        assert!(attempt.is_dir());
    }

    #[test]
    fn a_resolver_clone_goes_once_its_commit_is_on_a_project_branch() {
        // pd7 D-04: the verdict is judged afresh on every scan, never stored.
        let (_root, project, run_dir, _clone, base) = fixture();
        let mut view = landed_view(&run_dir, &base);
        view.record.landed.clear();
        let attempt = run_dir.join("land/1");
        fs::create_dir_all(&attempt).unwrap();
        let land_clone = attempt.join("work");
        git(
            &run_dir,
            &[
                "clone",
                "-q",
                "--no-hardlinks",
                &project.display().to_string(),
                &land_clone.display().to_string(),
            ],
        );
        push::planted::save_baseline_if_absent(
            &run_dir,
            &push::planted::snapshot(&run_dir.join("work")),
        )
        .unwrap();
        fs::write(land_clone.join("m.txt"), "merge").unwrap();
        git(&land_clone, &["add", "-A"]);
        git(&land_clone, &["commit", "-qm", "merge"]);
        let mut first = RecycleReport::default();
        tidy_finished(&project, &view, &mut first);
        assert!(land_clone.is_dir(), "an unreachable commit keeps the clone");
        assert!(first.warnings.iter().any(|w| w.contains("resolver clone")));

        // The commit reaches a project branch (what a fast-forward of the target does).
        git(
            &project,
            &[
                "fetch",
                "-q",
                &land_clone.display().to_string(),
                "HEAD:refs/heads/reached",
            ],
        );
        let mut second = RecycleReport::default();
        tidy_finished(&project, &view, &mut second);
        assert!(!land_clone.exists(), "the preservation is obsolete");
        assert!(attempt.is_dir(), "the attempt dir stays");
        assert!(second.warnings.is_empty(), "{:?}", second.warnings);
    }

    #[test]
    fn already_landed_candidates_are_finished_branch_targeted_runs_with_work() {
        use super::super::store::{Landing, RunRecord};
        let (_root, _project, run_dir, _clone, base) = fixture();
        let candidate = |edit: &dyn Fn(&mut RunRecord)| {
            let mut record = landed_view(&run_dir, &base).record;
            record.landed.clear();
            record.commits = Some(2);
            record.dispatch_branch = Some("main".to_owned());
            record.dispatch_tip = Some(base.clone());
            record.dispatch_detached = false;
            edit(&mut record);
            already_landed_candidate(&record)
        };
        assert_eq!(candidate(&|_| {}), Some("main".to_owned()));
        assert_eq!(candidate(&|r| r.commits = Some(0)), None);
        assert_eq!(candidate(&|r| r.commits = None), None);
        assert_eq!(candidate(&|r| r.head = None), None);
        assert_eq!(candidate(&|r| r.outcome = Some(AgentOutcome::Paused)), None);
        assert_eq!(candidate(&|r| r.status = RecordStatus::Running), None);
        assert_eq!(
            candidate(&|r| r.landed.push(Landing::plain(
                LandingKind::Merged,
                "t".to_owned(),
                "main".to_owned()
            ))),
            None
        );
        assert_eq!(
            candidate(&|r| r.landed.push(Landing::plain(
                LandingKind::AlreadyLanded,
                "t".to_owned(),
                "main".to_owned()
            ))),
            None
        );
        assert_eq!(
            candidate(&|r| r.landed.push(Landing::plain(
                LandingKind::Pushed,
                "t".to_owned(),
                "o abc".to_owned()
            ))),
            Some("main".to_owned()),
            "a push does not put the work on the target"
        );
        assert_eq!(candidate(&|r| r.dispatch_detached = true), None);
        assert_eq!(
            candidate(&|r| {
                r.dispatch_branch = None;
                r.dispatch_tip = None;
            }),
            None,
            "a legacy record has no branch to look in"
        );
        assert_eq!(
            candidate(&|r| r.resumed_by = Some("oneoff-x".to_owned())),
            None
        );
    }
}
