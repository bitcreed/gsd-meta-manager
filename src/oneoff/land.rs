//! Landing a finished one-off: merge, push or discard (C-18..C-21, I-4, I-10, I-11).
//!
//! * **merge** lands the recorded head in the user's checkout, only when the checkout is
//!   clean and on a branch, and only after the planted-file gate found nothing
//!   git-relevant in the clone (the fetch starts `upload-pack` inside the agent's
//!   repository). The fetch is hardened. When the checkout's HEAD is an ancestor of the
//!   head it is the user's own `git merge --ff-only`, hooks included (C-19). Otherwise
//!   the merge is made in a private clone and, when it conflicts, resolved by a sandboxed
//!   agent session ([`super::resolve`], quick 261103-hx7, D-04, D-05); the checkout
//!   changes only at the final fast-forward. A per-run land lock admits one landing of a
//!   run at a time (I-13).
//! * **push** sends the head only to `refs/heads/gsd-auto/<alias>/<run-id>`, from
//!   the launcher's mirror whose URL was pinned before any sandbox, after the SAFE-03
//!   scan, through the store-free core the driver's push uses. No pull request (I-11).
//! * **discard** deletes the run dir once the run is not live, and the run's tracked
//!   branch `gsd-oneoff/<run-id>` while it still points at the recorded head.
//!
//! Quick 261103-hx7: the daemon tracks every finished run that made commits as
//! `refs/heads/gsd-oneoff/<run-id>` in the project ([`track_branch`], after the same
//! planted-file gate), and a landing deletes that branch with a safe `git branch -d`.
//!
//! Every refusal changes nothing in the checkout.

use std::ffi::OsString;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::time::Duration;

use chrono::Utc;

use crate::driver::lock::{acquire_at_with_retry, probe_at, LockProbe, RunLock};
use crate::error::LockError;
use crate::launcher::process::run_bounded_with_stderr;
use crate::launcher::push::{
    self, check_destination, dest_ref, mirror_path, pinned_push_url, planted, push_tip, TipPush,
    TipPushed,
};

use super::runtime::AgentRuntime;
use super::store::{
    project_dir_for, read_view, rfc3339, write_record, Landing, LandingKind, MergeHow, OneoffView,
    RecordStatus, RunRecord, NOTE_ALREADY_GONE, NOTE_DELETED_AFTER_LANDING,
    NOTE_DELETED_BY_DISCARD,
};
use super::BRANCH_PREFIX;
use super::LAND_LOCK_FILE;
use super::WORK_DIR;

/// The paths whose change the merge confirmation calls out (I-10), besides the
/// agent's own project config dir ([`AgentRuntime::project_config_dir`]).
const SENSITIVE_DIRS: &[&str] = &[".github/", ".husky/"];
const SENSITIVE_FILES: &[&str] = &[".gitattributes", ".gitmodules", ".envrc"];

/// The plain `git merge --ff-only` budget.
const MERGE_BUDGET: Duration = Duration::from_secs(300);

/// How long a second landing of the same run waits for the land lock before refusing.
const LAND_LOCK_WAIT: Duration = Duration::from_millis(250);

/// What a landing needs besides the project and the run (quick 261103-hx7, I-12).
#[derive(Debug, Clone)]
pub struct LandOptions {
    /// The project's `driver.egress` CONNECT allowlist, for a conflict resolver session.
    pub egress_allow: Vec<String>,
    /// Set by the caller to stop a resolver session (the CLI's signal handlers). The TUI
    /// passes a flag that is never set: if it exits, the resolver dies with it.
    pub user_stop: Arc<AtomicBool>,
}

impl LandOptions {
    /// Options with a never-set stop flag.
    pub fn new(egress_allow: Vec<String>) -> Self {
        LandOptions {
            egress_allow,
            user_stop: Arc::new(AtomicBool::new(false)),
        }
    }

    /// Options whose stop flag Ctrl-C and TERM set, as they do for a run (the CLI).
    pub fn with_signal_handlers(egress_allow: Vec<String>) -> Result<Self, String> {
        let opts = Self::new(egress_allow);
        crate::driverctl::serve::install_signal_handlers(&opts.user_stop)
            .map_err(|e| e.to_string())?;
        Ok(opts)
    }
}

/// The per-run land lock inside `run_dir` (I-13).
pub(crate) fn land_lock_path(run_dir: &Path) -> PathBuf {
    run_dir.join(LAND_LOCK_FILE)
}

/// Whether a landing of the run in `run_dir` is in progress right now.
pub fn landing_in_progress(run_dir: &Path) -> bool {
    matches!(
        probe_at(&land_lock_path(run_dir)),
        LockProbe::Held | LockProbe::Unknown(_)
    )
}

/// Take the land lock of the run, or say why not.
fn take_land_lock(run_dir: &Path, run_id: &str) -> Result<RunLock, String> {
    acquire_at_with_retry(
        &land_lock_path(run_dir),
        run_id,
        std::process::id(),
        LAND_LOCK_WAIT,
    )
    .map_err(|e| match e {
        LockError::HeldBy { .. } | LockError::HeldByUnknownRun => {
            "a landing of this run is already in progress".to_owned()
        }
        LockError::Unavailable { detail } => format!("land lock: {detail}"),
    })
}

/// Everything the merge decision looks at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MergeFacts {
    /// The run may still be live.
    pub live: bool,
    /// The run is finished and its head was recorded.
    pub finished_with_head: bool,
    pub commits: u32,
    /// Git-relevant planted findings in the clone.
    pub planted: usize,
    /// `git status --porcelain` printed something.
    pub dirty: bool,
    /// The checkout's branch; `None` on a detached HEAD.
    pub branch: Option<String>,
    /// The checkout's HEAD is an ancestor of the recorded head: the landing is a
    /// fast-forward. Otherwise it is a merge.
    pub ancestor: bool,
}

/// Whether the merge may run (pure). Each refusal has its own reason.
pub fn merge_decision(facts: &MergeFacts) -> Result<(), String> {
    if facts.live {
        return Err("the run is still live; stop it or wait for it to finish".to_owned());
    }
    if !facts.finished_with_head {
        return Err("the run has no recorded head to merge".to_owned());
    }
    if facts.commits == 0 {
        return Err("the run made no commits; there is nothing to merge".to_owned());
    }
    if facts.planted > 0 {
        return Err(format!(
            "{} git-relevant planted finding(s) in the run's clone; it is not landed (discard it)",
            facts.planted
        ));
    }
    if facts.dirty {
        return Err(
            "the checkout has uncommitted or untracked changes; commit or stash them first"
                .to_owned(),
        );
    }
    if facts.branch.is_none() {
        return Err("the checkout's HEAD is detached; check out a branch first".to_owned());
    }
    // `facts.ancestor` is not a refusal any more: it chooses between the fast-forward
    // and the merge landing (quick 261103-hx7, C-10).
    Ok(())
}

/// The changed paths the merge confirmation calls out (I-10).
pub fn sensitive_paths(paths: &[String]) -> Vec<String> {
    paths
        .iter()
        .filter(|p| {
            p.starts_with(AgentRuntime::default().project_config_dir())
                || SENSITIVE_DIRS.iter().any(|d| p.starts_with(d))
                || SENSITIVE_FILES
                    .iter()
                    .any(|f| p.as_str() == *f || p.ends_with(&format!("/{f}")))
        })
        .cloned()
        .collect()
}

/// The warning lines for `paths`.
pub fn sensitive_warning(paths: &[String]) -> Vec<String> {
    let found = sensitive_paths(paths);
    if found.is_empty() {
        return Vec::new();
    }
    let mut lines = vec![format!(
        "warning: the run changed {} sensitive path(s); review them before you rely on them:",
        found.len()
    )];
    lines.extend(found.into_iter().map(|p| format!("  {p}")));
    lines
}

/// The view of `run_id`, or why not.
fn view_of(project: &Path, run_id: &str) -> Result<(PathBuf, OneoffView), String> {
    if !super::store::valid_oneoff_run_id(run_id) {
        return Err(format!("{run_id:?} is not a one-off run id"));
    }
    let project_dir = project_dir_for(project)
        .ok_or_else(|| "no one-off directory for this project".to_owned())?;
    let view = read_view(&project_dir, run_id)
        .ok_or_else(|| format!("no one-off run {run_id} for this project"))?;
    Ok((project_dir, view))
}

/// Git-relevant planted findings in `work` against the baseline saved in `run_dir`
/// (strict without one). File reads only, so it may run before any git touches the
/// clone (quick 261103-hx7, C-05).
pub(crate) fn git_relevant_planted(run_dir: &Path, work: &Path) -> Vec<planted::Finding> {
    let baseline = planted::load_baseline(run_dir).ok().flatten();
    planted::findings(baseline.as_ref(), &planted::snapshot(work), work)
        .into_iter()
        .filter(planted::Finding::is_git_relevant)
        .collect()
}

/// Git-relevant planted findings in the clone against its launch baseline (strict
/// without one). File reads only.
fn planted_count(view: &OneoffView) -> usize {
    let work = view.run_dir.join(WORK_DIR);
    let found = git_relevant_planted(&view.run_dir, &work);
    for line in planted::render_bounded(&found) {
        eprintln!("task land: planted: {line}");
    }
    found.len()
}

/// The full ref a run's work is tracked under in the user's project.
pub(crate) fn tracked_ref(run_id: &str) -> String {
    format!("refs/heads/{BRANCH_PREFIX}{run_id}")
}

/// The commit `reference` names in `project`, if it exists.
fn ref_commit(project: &Path, reference: &str) -> Option<String> {
    let found = git_in(
        project,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("{reference}^{{commit}}"),
        ],
    );
    (found.success && push::is_hex_sha(&found.stdout)).then_some(found.stdout)
}

/// Track the run's `head` in `project` as `refs/heads/gsd-oneoff/<run-id>` so that
/// `git branch --no-merged` shows unmerged work (quick 261103-hx7, D-02, C-06).
///
/// `Ok(ref)` when the branch was made or already pointed at `head`; `Err(note)` says why
/// not. A branch that points elsewhere is never overwritten (the refspec has no `+`),
/// and the fetch is the hardened one. The caller has already run the planted-file gate.
pub(crate) fn track_branch(
    project: &Path,
    work: &Path,
    run_id: &str,
    head: &str,
) -> Result<String, String> {
    let reference = tracked_ref(run_id);
    if let Some(existing) = ref_commit(project, &reference) {
        return if existing == head {
            Ok(reference)
        } else {
            Err(format!(
                "a branch {BRANCH_PREFIX}{run_id} already exists at {}; not overwritten",
                &existing[..existing.len().min(12)]
            ))
        };
    }
    let fetch = push::git(
        vec![
            "-C".into(),
            project.as_os_str().to_owned(),
            "fetch".into(),
            "--quiet".into(),
            "--no-tags".into(),
            "--no-recurse-submodules".into(),
            work.as_os_str().to_owned(),
            format!("{reference}:{reference}").into(),
        ],
        push::GIT_FETCH,
    );
    if !fetch.success {
        return Err(format!(
            "could not fetch the run's branch into the project: {}",
            push::redact_urls(&fetch.last_error)
        ));
    }
    match ref_commit(project, &reference) {
        Some(sha) if sha == head => Ok(reference),
        Some(sha) => Err(format!(
            "the fetched branch is at {}, not the recorded head; not tracked",
            &sha[..sha.len().min(12)]
        )),
        None => Err("the fetch made no branch; not tracked".to_owned()),
    }
}

fn git_in(dir: &Path, args: &[&str]) -> push::GitRun {
    push::git_in(dir, args, push::GIT_QUICK)
}

/// The longest `git diff` of a landed merge may run.
const DIFF_BUDGET: Duration = Duration::from_secs(60);

/// The longest the full diff export may run.
const EXPORT_BUDGET: Duration = Duration::from_secs(120);

/// The `git diff` arguments every read of a merge's diff shares, so the shortstat, the
/// numstat, the hunks and the export agree (klf, I-03): no external driver, no
/// textconv, no colour, no rename detection, first parent against the merge.
fn merge_diff_args(project: &Path, flags: &[&str], merge_sha: &str) -> Vec<OsString> {
    let mut args: Vec<OsString> = vec!["-C".into(), project.as_os_str().to_owned(), "diff".into()];
    args.extend(
        [
            "--no-ext-diff",
            "--no-textconv",
            "--no-color",
            "--no-renames",
        ]
        .iter()
        .map(OsString::from),
    );
    args.extend(flags.iter().map(OsString::from));
    args.push(format!("{merge_sha}^1").into());
    args.push(merge_sha.into());
    args
}

/// The diff of a landed merge commit against its first parent, for the TUI's `d`
/// (quick 261103-klf, L-6, C-19, C-20, I-03). Three bounded reads through hardened git
/// in the project, none of which holds more than its cap in memory: the totals
/// (`--shortstat`), the per-file summary (`--numstat -z`, 2000 files / 1 MiB) and the
/// hunks (5000 lines / 512 KiB, after which git is killed). The view is headed by the
/// conflicted files and, when the hunks were cut, closed by a `truncated — ` marker.
/// Blocking: the caller runs it on `spawn_blocking`.
pub fn merge_diff(
    project: &Path,
    merge_sha: &str,
    conflicted: &[String],
) -> Result<Vec<String>, String> {
    use super::merge::{
        diff_view, parse_numstat_z, parse_shortstat, DIFF_MAX_BYTES, DIFF_MAX_FILES,
        DIFF_MAX_LINES, NUMSTAT_MAX_BYTES,
    };
    if !push::is_hex_sha(merge_sha) {
        return Err("the recorded merge commit is not a commit id".to_owned());
    }
    let fail =
        |what: &str, why: &str| format!("git could not diff {} ({what}): {why}", &merge_sha[..12]);
    let stat = push::git_capped(
        merge_diff_args(project, &["--shortstat"], merge_sha),
        4096,
        16,
        DIFF_BUDGET,
    );
    if !stat.success {
        return Err(fail("totals", &stat.last_error));
    }
    let numstat = push::git_capped(
        merge_diff_args(project, &["--numstat", "-z"], merge_sha),
        NUMSTAT_MAX_BYTES,
        usize::MAX,
        DIFF_BUDGET,
    );
    if !numstat.success {
        return Err(fail("files", &numstat.last_error));
    }
    let hunks = push::git_capped(
        merge_diff_args(project, &[], merge_sha),
        DIFF_MAX_BYTES,
        DIFF_MAX_LINES,
        DIFF_BUDGET,
    );
    if !hunks.success {
        return Err(fail("hunks", &hunks.last_error));
    }
    let rows = parse_numstat_z(&numstat.stdout);
    let rows_truncated = numstat.truncated || rows.len() > DIFF_MAX_FILES;
    Ok(diff_view(
        conflicted,
        &parse_shortstat(&stat.stdout),
        &rows,
        rows_truncated,
        &hunks.stdout,
        hunks.truncated,
    ))
}

/// Write the full diff of a landed merge commit to
/// `<run-dir>/land/merge-<sha12>.diff` and return the path (klf, C-21, I-04, T-klf-02).
///
/// git's stdout goes straight into the file, never through memory. `land/` is created
/// `0700` if missing; the file is opened `O_NOFOLLOW | O_CREAT | O_TRUNC` with mode
/// `0600`, so a symlink planted at that path fails the export instead of being followed.
/// Blocking: the caller runs it on `spawn_blocking`.
pub fn export_merge_diff(
    project: &Path,
    run_dir: &Path,
    merge_sha: &str,
) -> Result<PathBuf, String> {
    use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
    if !push::is_hex_sha(merge_sha) {
        return Err("the recorded merge commit is not a commit id".to_owned());
    }
    let land = run_dir.join(super::LAND_DIR);
    match fs::DirBuilder::new().mode(0o700).create(&land) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(format!("create {}: {e}", land.display())),
    }
    let path = land.join(format!("merge-{}.diff", &merge_sha[..12]));
    let file = fs::OpenOptions::new()
        .write(true)
        .create(true)
        .truncate(true)
        .mode(0o600)
        .custom_flags(i32::try_from(rustix::fs::OFlags::NOFOLLOW.bits()).unwrap_or(0))
        .open(&path)
        .map_err(|e| format!("open {}: {e}", path.display()))?;
    let run = push::git_to_file(
        merge_diff_args(project, &[], merge_sha),
        file,
        EXPORT_BUDGET,
    );
    if run.success {
        Ok(path)
    } else {
        Err(format!(
            "git could not write the diff to {}: {}",
            path.display(),
            run.last_error
        ))
    }
}

/// Record a landing on the run.
fn record_landing(view: &OneoffView, landing: Landing) {
    let mut record = view.record.clone();
    record.landed.push(landing);
    // A later success clears the latest failure (I-19).
    record.land_failure = None;
    if let Err(e) = write_record(&view.run_dir, &record) {
        eprintln!("task land: the landing succeeded but could not be recorded: {e}");
    }
}

/// Land the run's recorded head in the project's checkout: a fast-forward when the
/// checkout's HEAD is an ancestor of it, otherwise a merge commit made in a private clone
/// (with a sandboxed agent session resolving conflicts). Never a rebase (D-04).
pub fn merge(project: &Path, run_id: &str, opts: &LandOptions) -> Result<String, String> {
    let project = fs::canonicalize(project).map_err(|e| format!("project: {e}"))?;
    let (_, first_view) = view_of(&project, run_id)?;
    // I-13: one landing of a run at a time; the view is read again under the lock.
    let _land_lock = take_land_lock(&first_view.run_dir, run_id)?;
    let (_, view) = view_of(&project, run_id)?;
    let record = &view.record;
    let head = record.head.clone();
    let mut facts = MergeFacts {
        live: view.live.may_be_live(),
        finished_with_head: record.status == RecordStatus::Finished && head.is_some(),
        commits: record.commits.unwrap_or(0),
        planted: 0,
        dirty: false,
        branch: Some(String::new()),
        ancestor: true,
    };
    merge_decision(&facts)?;
    let head = head.unwrap_or_default();
    // I-4: the planted gate reads files only, and runs before any git touches the clone.
    facts.planted = planted_count(&view);
    let status = git_in(&project, &["status", "--porcelain"]);
    if !status.success {
        return Err(format!("git status failed: {}", status.last_error));
    }
    facts.dirty = !status.stdout.is_empty();
    let branch = git_in(&project, &["symbolic-ref", "-q", "--short", "HEAD"]);
    facts.branch =
        (branch.success && push::plausible_branch(&branch.stdout)).then_some(branch.stdout);
    merge_decision(&facts)?;

    let work = view.run_dir.join(WORK_DIR);
    let branch_ref = format!("refs/heads/{BRANCH_PREFIX}{run_id}");
    let fetch = push::git(
        vec![
            "-C".into(),
            project.as_os_str().to_owned(),
            "fetch".into(),
            "--quiet".into(),
            "--no-tags".into(),
            "--no-recurse-submodules".into(),
            work.as_os_str().to_owned(),
            branch_ref.into(),
        ],
        push::GIT_FETCH,
    );
    if !fetch.success {
        return Err(format!(
            "could not fetch the run's branch: {}",
            fetch.last_error
        ));
    }
    let fetched = git_in(
        &project,
        &["rev-parse", "--verify", "--quiet", "FETCH_HEAD^{commit}"],
    );
    if !fetched.success || fetched.stdout != head {
        return Err("the fetched tip is not the recorded head; nothing was merged".to_owned());
    }
    let ancestor = git_in(&project, &["merge-base", "--is-ancestor", "HEAD", &head]);
    facts.ancestor = ancestor.success;
    merge_decision(&facts)?;
    if !facts.ancestor {
        let target = facts.branch.clone().unwrap_or_default();
        // I-24: a head the target already contains would make a `--no-ff` merge that
        // creates nothing.
        if git_in(&project, &["merge-base", "--is-ancestor", &head, "HEAD"]).success {
            return Err(format!(
                "the run's head is already in {target}; nothing to merge (discard the run)"
            ));
        }
        return super::resolve::land_by_merge(&project, &view, &target, opts);
    }

    // klf I-08: the checkout's HEAD before the fast-forward starts the landed range.
    let old_head = ref_commit(&project, "HEAD");

    // C-19: the user's own git, hooks included.
    let args: Vec<OsString> = [
        "-C",
        &project.to_string_lossy(),
        "merge",
        "--ff-only",
        "--quiet",
        &head,
    ]
    .iter()
    .map(OsString::from)
    .collect();
    match run_bounded_with_stderr(
        "git",
        &args,
        None,
        &[("GIT_TERMINAL_PROMPT", "0")],
        MERGE_BUDGET,
    ) {
        Some((out, _)) if out.status.success() => {}
        Some((_, line)) => {
            return Err(format!(
                "git merge --ff-only failed: {}",
                push::redact_urls(&line)
            ))
        }
        None => return Err("git merge did not finish".to_owned()),
    }
    let branch = facts.branch.unwrap_or_default();
    let mut lines = vec![format!(
        "merged {} commit(s) of {run_id} into {branch} (now at {})",
        facts.commits,
        &head[..head.len().min(12)]
    )];
    // klf I-06: the delete first, then ONE write with the landing and the branch fields.
    let mut updated = view.record.clone();
    updated.landed.push(Landing {
        how: Some(MergeHow::Ff),
        merge_sha: Some(head.clone()),
        range: old_head.map(|old| format!("{old}..{head}")),
        ..Landing::plain(LandingKind::Merged, rfc3339(Utc::now()), branch.clone())
    });
    updated.land_failure = None;
    if let Some(line) = clear_tracking_after_landing(&project, run_id, &head, &mut updated) {
        lines.push(line);
    }
    if let Err(e) = write_record(&view.run_dir, &updated) {
        eprintln!("task land: the landing succeeded but could not be recorded: {e}");
    }
    lines.extend(sensitive_warning(&record.changed_files));
    Ok(lines.join("\n"))
}

/// What deleting the tracked branch after a landing came to (klf, I-06).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TrackedDelete {
    /// `git branch -d` removed it.
    Deleted,
    /// There was no such branch, or it points elsewhere (nothing of ours to delete).
    Absent,
    /// The delete failed or was refused; the ref stays, and this line says why.
    Left(String),
}

/// After a landing the tracked branch is merged and stays reachable from the target, so
/// a safe `git branch -d` removes it (C-07). Only a branch that points at `head` is
/// touched; a failure is one line, never an error.
pub(crate) fn delete_tracked_branch_after_landing(
    project: &Path,
    run_id: &str,
    head: &str,
) -> TrackedDelete {
    let reference = tracked_ref(run_id);
    if ref_commit(project, &reference).as_deref() != Some(head) {
        return TrackedDelete::Absent;
    }
    let name = format!("{BRANCH_PREFIX}{run_id}");
    let deleted = git_in(project, &["branch", "--delete", "--quiet", &name]);
    if deleted.success {
        TrackedDelete::Deleted
    } else {
        TrackedDelete::Left(format!(
            "the tracked branch was not deleted: {}",
            push::redact_urls(&deleted.last_error)
        ))
    }
}

/// Delete the tracked branch after a landing and bring `record`'s branch fields in line
/// with what happened (klf, C-15, I-06). The caller writes the record once afterwards.
/// Returns the one line to print when the branch was left in place.
pub(crate) fn clear_tracking_after_landing(
    project: &Path,
    run_id: &str,
    head: &str,
    record: &mut RunRecord,
) -> Option<String> {
    match delete_tracked_branch_after_landing(project, run_id, head) {
        TrackedDelete::Deleted => {
            record.tracked_branch = None;
            record.tracking_note = Some(NOTE_DELETED_AFTER_LANDING.to_owned());
            None
        }
        TrackedDelete::Absent => {
            if record.tracked_branch.is_some() {
                record.tracked_branch = None;
                record.tracking_note = Some(NOTE_ALREADY_GONE.to_owned());
            }
            None
        }
        TrackedDelete::Left(line) => Some(line),
    }
}

/// Push the run's recorded head to `refs/heads/gsd-auto/<alias>/<run-id>`.
pub fn push(
    project: &Path,
    alias: &str,
    run_id: &str,
    driver_root: &Path,
) -> Result<String, String> {
    let project = fs::canonicalize(project).map_err(|e| format!("project: {e}"))?;
    let (_, view) = view_of(&project, run_id)?;
    let record = &view.record;
    if view.live.may_be_live() {
        return Err("the run is still live; stop it or wait for it to finish".to_owned());
    }
    if record.status != RecordStatus::Finished || record.head.is_none() {
        return Err("the run has no recorded head to push".to_owned());
    }
    if record.commits.unwrap_or(0) == 0 {
        return Err("the run made no commits; there is nothing to push".to_owned());
    }
    let mirror = mirror_path(driver_root, &project);
    let url = pinned_push_url(&mirror).ok_or_else(|| {
        "no push remote was pinned when the mirror was created (the project had no \
         remote.origin.url when the run started)"
            .to_owned()
    })?;
    let dest = dest_ref(alias, run_id);
    let private_ref = format!("refs/gsdmm/{run_id}");
    for name in [&dest, &private_ref] {
        let check = push::git_mirror(&mirror, &["check-ref-format", name], push::GIT_QUICK);
        if !check.success {
            return Err(format!("{name:?} is not a valid ref name"));
        }
    }
    check_destination(&dest, alias).map_err(|reason| format!("{dest} is refused: {reason}"))?;
    let planted = planted_count(&view);
    if planted > 0 {
        return Err(format!(
            "{planted} git-relevant planted finding(s) in the run's clone; nothing was pushed (discard it)"
        ));
    }
    let work = view.run_dir.join(WORK_DIR);
    let branch = format!("{BRANCH_PREFIX}{run_id}");
    let prior = record.last_pushed_sha();
    let scan_dir = mirror
        .parent()
        .unwrap_or(driver_root)
        .join(format!("scan-{run_id}"));
    match push_tip(&TipPush {
        mirror: &mirror,
        url: &url,
        checkout: &work,
        branch: &branch,
        private_ref: &private_ref,
        dest: &dest,
        start_sha: record.base.as_deref(),
        prior_pushed_sha: prior.as_deref(),
        scan_dir: &scan_dir,
    }) {
        TipPushed::Pushed { sha } => {
            record_landing(
                &view,
                Landing::plain(
                    LandingKind::Pushed,
                    rfc3339(Utc::now()),
                    format!("{dest} {sha}"),
                ),
            );
            Ok(format!("pushed {} to {dest}", &sha[..sha.len().min(12)]))
        }
        TipPushed::AlreadyPushed => Ok(format!("already pushed: {dest} has this run's head")),
        TipPushed::NothingToPush => Ok("nothing to push: the branch is at its start".to_owned()),
        TipPushed::SecretFound { findings, rendered } => {
            eprintln!("{rendered}");
            Err(format!(
                "the credential scan found {findings} finding(s) in the tip; nothing was pushed"
            ))
        }
        TipPushed::FetchFailed { detail }
        | TipPushed::ScanFailed { detail }
        | TipPushed::PushFailed { detail } => Err(detail),
    }
}

/// Delete the run's directory, clone included, once the run is not live.
pub fn discard(project: &Path, run_id: &str) -> Result<String, String> {
    let (project_dir, view) = view_of(project, run_id)?;
    if view.live.may_be_live() {
        return Err("the run is still live; stop it first".to_owned());
    }
    if landing_in_progress(&view.run_dir) {
        return Err("a landing of this run is in progress; wait for it to end".to_owned());
    }
    let run_dir = project_dir.join(run_id);
    let meta = fs::symlink_metadata(&run_dir).map_err(|e| format!("{}: {e}", run_dir.display()))?;
    if !meta.file_type().is_dir() {
        return Err(format!("{} is not a directory", run_dir.display()));
    }
    let canonical = fs::canonicalize(&run_dir).map_err(|e| e.to_string())?;
    let parent = fs::canonicalize(&project_dir).map_err(|e| e.to_string())?;
    if canonical.parent() != Some(parent.as_path()) {
        return Err(format!(
            "{} does not sit directly under the project's one-off directory",
            run_dir.display()
        ));
    }
    // C-08, I-09: the tracked branch goes first, by an atomic compare-and-delete that
    // only succeeds while it still points at the recorded head.
    let mut lines = Vec::new();
    if let Some(head) = &view.record.head {
        let reference = tracked_ref(run_id);
        if ref_commit(project, &reference).is_some() {
            let deleted = git_in(project, &["update-ref", "-d", &reference, head]);
            // klf I-07: record the branch's fate BEFORE the directory goes, so a removal
            // that fails half-way leaves no stale ref in run.json.
            let mut updated = view.record.clone();
            if deleted.success {
                updated.tracked_branch = None;
                updated.tracking_note = Some(NOTE_DELETED_BY_DISCARD.to_owned());
                lines.push(format!("deleted branch {BRANCH_PREFIX}{run_id}"));
            } else {
                updated.tracking_note =
                    Some("left by discard: it no longer points at the run's head".to_owned());
                lines.push(format!(
                    "left branch {BRANCH_PREFIX}{run_id}: it no longer points at the run's head"
                ));
            }
            if let Err(e) = write_record(&view.run_dir, &updated) {
                eprintln!("task land: could not record the branch's fate: {e}");
            }
        }
    }
    fs::remove_dir_all(&canonical).map_err(|e| format!("remove {}: {e}", canonical.display()))?;
    lines.insert(0, format!("discarded {run_id}"));
    Ok(lines.join("\n"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ok_facts() -> MergeFacts {
        MergeFacts {
            live: false,
            finished_with_head: true,
            commits: 2,
            planted: 0,
            dirty: false,
            branch: Some("main".to_owned()),
            ancestor: true,
        }
    }

    #[test]
    fn a_clean_branch_ancestor_with_commits_merges() {
        assert!(merge_decision(&ok_facts()).is_ok());
    }

    #[test]
    fn a_non_ancestor_passes_the_decision_and_takes_the_merge_path() {
        let facts = MergeFacts {
            ancestor: false,
            ..ok_facts()
        };
        assert!(merge_decision(&facts).is_ok());
        // The other refusals still win over it.
        let dirty = MergeFacts {
            ancestor: false,
            dirty: true,
            ..ok_facts()
        };
        assert!(merge_decision(&dirty).is_err());
    }

    #[test]
    fn each_refusal_has_its_own_reason() {
        let cases: Vec<(MergeFacts, &str)> = vec![
            (
                MergeFacts {
                    live: true,
                    ..ok_facts()
                },
                "live",
            ),
            (
                MergeFacts {
                    commits: 0,
                    ..ok_facts()
                },
                "no commits",
            ),
            (
                MergeFacts {
                    dirty: true,
                    ..ok_facts()
                },
                "uncommitted",
            ),
            (
                MergeFacts {
                    branch: None,
                    ..ok_facts()
                },
                "detached",
            ),
            (
                MergeFacts {
                    planted: 1,
                    ..ok_facts()
                },
                "planted",
            ),
            (
                MergeFacts {
                    finished_with_head: false,
                    ..ok_facts()
                },
                "no recorded head",
            ),
        ];
        for (facts, needle) in cases {
            let why = merge_decision(&facts).unwrap_err();
            assert!(why.contains(needle), "{needle}: {why}");
        }
    }

    #[test]
    fn sensitive_paths_are_called_out() {
        let agent_settings = format!(
            "{}settings.json",
            AgentRuntime::default().project_config_dir()
        );
        let paths: Vec<String> = [
            agent_settings.as_str(),
            ".github/workflows/x.yml",
            ".gitattributes",
            ".envrc",
            "src/a.rs",
            ".husky/pre-commit",
            "sub/.gitmodules",
        ]
        .iter()
        .map(|s| (*s).to_owned())
        .collect();
        let found = sensitive_paths(&paths);
        assert!(!found.contains(&"src/a.rs".to_owned()));
        assert_eq!(found.len(), 6, "{found:?}");
    }
}
