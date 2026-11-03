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
use std::path::Path;

use chrono::Utc;

use crate::driver::lock::{probe_at, LockProbe};
use crate::launcher::push;

use super::land::{git_relevant_planted, land_lock_path, take_land_lock};
use super::store::{
    project_dir_for, read_view, rfc3339, verified_summary, write_record, LandingKind, OneoffView,
    Pruned, RecordStatus,
};
use super::stream::spend_of_file;
use super::{
    INJECTIONS_FILE, KEPT_SUMMARY_FILE, LAND_DIR, LAND_LOCK_FILE, LAUNCH_LOG, REPORT_FILE,
    REPORT_SUBDIR, RUN_FILE, RUN_LOCK_FILE, STREAM_FILE, WORK_DIR,
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

/// Slim the run `run_id` of `project` after a landing: take the run's land lock, slim,
/// unlink `land.lock` while it is still held (I-15) and release it. `note` is the reason
/// recorded in `pruned`. Never an error: the outcome says what happened.
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
    // I-15: unlinked only by the process that holds it.
    if matches!(outcome, SlimOutcome::Slimmed { .. }) {
        let _ = fs::remove_file(land_lock_path(&run_dir));
    }
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
    let merged = record.landed.iter().any(|l| l.kind == LandingKind::Merged);
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
}
