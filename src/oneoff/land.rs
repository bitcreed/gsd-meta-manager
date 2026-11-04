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

use crate::driver::lock::{acquire_at, acquire_at_with_retry, probe_at, LockProbe, RunLock};
use crate::error::LockError;
use crate::launcher::process::run_bounded_with_stderr;
use crate::launcher::push::{
    self, check_destination, dest_ref, mirror_path, pinned_push_url, planted, push_tip, TipPush,
    TipPushed,
};

use super::runtime::AgentRuntime;
use super::store::{
    project_dir_for, read_view, rfc3339, write_record, LandTarget, Landing, LandingKind, MergeHow,
    OneoffView, RecordStatus, RunRecord, NOTE_ALREADY_GONE, NOTE_DELETED_AFTER_LANDING,
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
    /// `--into <branch>`: land into this existing local branch instead of the run's
    /// recorded dispatch branch (quick 261103-o2r, D-C).
    pub into: Option<String>,
}

impl LandOptions {
    /// Options with a never-set stop flag.
    pub fn new(egress_allow: Vec<String>) -> Self {
        LandOptions {
            egress_allow,
            user_stop: Arc::new(AtomicBool::new(false)),
            into: None,
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

/// The held land lock of a run. Dropping it releases the lock and removes the lock file
/// (quick 261103-pd7, D-05): every taker went through `acquire_at`, which creates the
/// file, and nothing but a successful slim ever unlinked it, so each landing, refusal and
/// scan left a `land.lock` behind.
///
/// This `Drop` releases nothing: it only unlinks. The release stays the fd close that
/// [`RunLock`] relies on. The unlink happens while the lock is still held, and only when
/// the path still names the inode this guard holds ([`RunLock::is_at`]), so a holder never
/// removes another holder's file; a taker that locked an inode somebody unlinked meanwhile
/// notices in [`take_land_lock`] and retries.
pub(crate) struct LandLock {
    path: PathBuf,
    lock: RunLock,
}

impl Drop for LandLock {
    fn drop(&mut self) {
        if self.lock.is_at(&self.path) {
            let _ = fs::remove_file(&self.path);
        }
    }
}

/// How many times a take re-opens the lock path after locking an unlinked inode.
const LAND_LOCK_REOPENS: usize = 16;

/// Take the land lock of the run, or say why not.
pub(crate) fn take_land_lock(run_dir: &Path, run_id: &str) -> Result<LandLock, String> {
    let path = land_lock_path(run_dir);
    for _ in 0..LAND_LOCK_REOPENS {
        let lock = acquire_at_with_retry(&path, run_id, std::process::id(), LAND_LOCK_WAIT)
            .map_err(land_lock_refusal)?;
        if lock.is_at(&path) {
            return Ok(LandLock { path, lock });
        }
        // The holder before us unlinked the file this descriptor was opened on; locking
        // it excludes nobody. Drop it (without unlinking) and take the new file.
    }
    Err("land lock: the lock file kept changing under the take".to_owned())
}

/// Take the land lock of the run without waiting: `None` when it is held or cannot be
/// taken (quick 261103-pd7, D-03, the scan).
pub(crate) fn try_land_lock(run_dir: &Path, run_id: &str) -> Option<LandLock> {
    let path = land_lock_path(run_dir);
    for _ in 0..LAND_LOCK_REOPENS {
        let lock = acquire_at(&path, run_id, std::process::id()).ok()?;
        if lock.is_at(&path) {
            return Some(LandLock { path, lock });
        }
    }
    None
}

fn land_lock_refusal(e: LockError) -> String {
    match e {
        LockError::HeldBy { .. } | LockError::HeldByUnknownRun => {
            "a landing of this run is already in progress".to_owned()
        }
        LockError::Unavailable { detail } => format!("land lock: {detail}"),
    }
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
    /// The landing target is the branch the project's checkout has checked out (quick
    /// 261103-o2r, D-C). Only then do a dirty checkout and a detached HEAD refuse: any
    /// other target is moved by `update-ref` and the working tree is never touched.
    pub target_checked_out: bool,
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
    if facts.target_checked_out && facts.dirty {
        return Err(
            "the checkout has uncommitted or untracked changes; commit or stash them first"
                .to_owned(),
        );
    }
    if facts.target_checked_out && facts.branch.is_none() {
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
pub(crate) fn ref_commit(project: &Path, reference: &str) -> Option<String> {
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

/// The branch a landing moves, and whether the project's checkout has it checked out.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Target {
    pub branch: String,
    pub checked_out: bool,
}

/// Which branch a landing aims at before it is looked up (pure, quick 261103-o2r, D-C).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum TargetChoice {
    /// A named local branch (`--into`, or the one recorded at dispatch).
    Branch { name: String, explicit: bool },
    /// A legacy record: the checkout's current branch, as before.
    Checkout,
}

/// `--into` beats the recorded dispatch branch; a legacy record lands into the checkout's
/// branch; a detached dispatch needs `--into`.
pub(crate) fn choose_target(
    into: Option<&str>,
    recorded: &LandTarget,
) -> Result<TargetChoice, String> {
    match (into, recorded) {
        (Some(name), _) => Ok(TargetChoice::Branch {
            name: name.to_owned(),
            explicit: true,
        }),
        (None, LandTarget::Branch(name)) => Ok(TargetChoice::Branch {
            name: name.clone(),
            explicit: false,
        }),
        (None, LandTarget::Legacy) => Ok(TargetChoice::Checkout),
        (None, LandTarget::Detached { tip }) => Err(format!(
            "the run was dispatched on a detached HEAD (@{}); pass --into <branch> to say \
             where to land it",
            &tip[..tip.len().min(7)]
        )),
    }
}

/// One entry of `git worktree list --porcelain`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Worktree {
    pub path: PathBuf,
    /// The full ref the worktree has checked out; `None` when detached or bare.
    pub branch: Option<String>,
}

/// Parse `git worktree list --porcelain` output, with `-z` (NUL-separated fields) or
/// without (newline-separated). Pure.
pub(crate) fn parse_worktrees(text: &str) -> Vec<Worktree> {
    let mut found: Vec<Worktree> = Vec::new();
    for field in text.split(['\0', '\n']) {
        if let Some(path) = field.strip_prefix("worktree ") {
            found.push(Worktree {
                path: PathBuf::from(path),
                branch: None,
            });
        } else if let (Some(reference), Some(last)) =
            (field.strip_prefix("branch "), found.last_mut())
        {
            last.branch = Some(reference.to_owned());
        }
    }
    found
}

/// The path of a worktree other than the project's own that has `branch` checked out
/// (I-06). A listing that cannot be read refuses: moving a branch another worktree has
/// checked out would desync that worktree's index.
fn checked_out_elsewhere(project: &Path, branch: &str) -> Result<Option<PathBuf>, String> {
    let mut listed = git_in(project, &["worktree", "list", "--porcelain", "-z"]);
    if !listed.success {
        listed = git_in(project, &["worktree", "list", "--porcelain"]);
    }
    if !listed.success {
        return Err(format!(
            "could not list the project's worktrees: {}",
            listed.last_error
        ));
    }
    let wanted = format!("refs/heads/{branch}");
    let own = fs::canonicalize(project).unwrap_or_else(|_| project.to_path_buf());
    Ok(parse_worktrees(&listed.stdout)
        .into_iter()
        .filter(|w| w.branch.as_deref() == Some(wanted.as_str()))
        .find(|w| fs::canonicalize(&w.path).unwrap_or_else(|_| w.path.clone()) != own)
        .map(|w| w.path))
}

/// A branch name `--into` or a record names, checked before it reaches any git argv.
fn check_branch_name(project: &Path, name: &str) -> Result<(), String> {
    if name.starts_with('-') || !push::plausible_branch(name) {
        return Err(format!("{name:?} is not a usable branch name"));
    }
    let checked = git_in(project, &["check-ref-format", "--branch", name]);
    if !checked.success || checked.stdout != name {
        return Err(format!("{name:?} is not a usable branch name"));
    }
    Ok(())
}

/// Look the chosen target up in `project` (D-C): the name is validated, a branch that
/// is gone refuses with the `--into` hint, and a branch another worktree has checked
/// out refuses naming that worktree.
fn resolve_target(
    project: &Path,
    choice: TargetChoice,
    checkout_branch: Option<&str>,
) -> Result<Target, String> {
    match choice {
        TargetChoice::Checkout => Ok(Target {
            branch: checkout_branch.unwrap_or_default().to_owned(),
            checked_out: true,
        }),
        TargetChoice::Branch { name, explicit } => {
            check_branch_name(project, &name)?;
            if checkout_branch == Some(name.as_str()) {
                return Ok(Target {
                    branch: name,
                    checked_out: true,
                });
            }
            if ref_commit(project, &format!("refs/heads/{name}")).is_none() {
                return Err(if explicit {
                    format!("there is no local branch {name:?} to land into")
                } else {
                    format!(
                        "the branch {name:?} the run was dispatched from no longer exists; pass \
                         --into <branch>"
                    )
                });
            }
            if let Some(path) = checked_out_elsewhere(project, &name)? {
                return Err(format!(
                    "{name:?} is checked out in the worktree {}; land from that worktree or \
                     check out another branch there first",
                    path.display()
                ));
            }
            Ok(Target {
                branch: name,
                checked_out: false,
            })
        }
    }
}

/// Move `branch` of `project` from `old` to `new` with `git update-ref` and the expected
/// old value, never touching a working tree or an index (quick 261103-o2r, D-C, I-05).
/// The tip is read again and the worktree list checked right before; any change refuses
/// with nothing changed.
pub(crate) fn update_target(
    project: &Path,
    branch: &str,
    new: &str,
    old: &str,
    run_id: &str,
) -> Result<(), String> {
    let reference = format!("refs/heads/{branch}");
    let moved = || format!("the target branch {branch} moved during the landing; nothing changed");
    if ref_commit(project, &reference).as_deref() != Some(old) {
        return Err(moved());
    }
    let current = git_in(project, &["symbolic-ref", "-q", "--short", "HEAD"]);
    if current.success && current.stdout == branch {
        return Err(format!(
            "the target branch {branch} was checked out during the landing; nothing changed"
        ));
    }
    if let Some(path) = checked_out_elsewhere(project, branch)? {
        return Err(format!(
            "{branch:?} is checked out in the worktree {}; nothing changed",
            path.display()
        ));
    }
    let message = format!("gsd-meta-manager: land {run_id}");
    let updated = git_in(
        project,
        &["update-ref", "-m", &message, &reference, new, old],
    );
    if updated.success {
        Ok(())
    } else {
        Err(moved())
    }
}

/// Where a landing takes the run's work from (quick 261103-o2r, D-A).
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum RunSource {
    /// The run's own clone, behind the planted-file gate.
    Clone(PathBuf),
    /// The clone was removed: `refs/heads/gsd-oneoff/<id>` in the project holds the work.
    Tracked,
}

/// What [`choose_source`] looks at.
pub(crate) struct SourceFacts<'a> {
    pub run_id: &'a str,
    /// The record says the clone was removed.
    pub pruned_note: Option<&'a str>,
    /// `<run-dir>/work` is a real directory.
    pub clone_present: bool,
    /// `refs/heads/gsd-oneoff/<id>` in the project is at the recorded head.
    pub tracked_at_head: bool,
    /// The branch the latest merge landing went into, if there is one.
    pub merged_into: Option<&'a str>,
}

/// Choose where the work comes from (pure). The clone only for a record that was not
/// pruned and still has it; otherwise the tracked branch at the head; otherwise a refusal
/// that says where the work is.
pub(crate) fn choose_source(facts: &SourceFacts<'_>, clone: &Path) -> Result<RunSource, String> {
    if facts.pruned_note.is_none() && facts.clone_present {
        return Ok(RunSource::Clone(clone.to_path_buf()));
    }
    if facts.tracked_at_head {
        return Ok(RunSource::Tracked);
    }
    Err(match (facts.merged_into, facts.pruned_note) {
        (Some(target), _) => format!(
            "the run's clone was removed after it landed in {target}; its work is in {target}"
        ),
        (None, Some(note)) => format!(
            "the run's clone was removed ({note}) and {BRANCH_PREFIX}{} is not at its head; \
             there is nothing to land",
            facts.run_id
        ),
        (None, None) => format!(
            "the run's clone is gone and {BRANCH_PREFIX}{} is not at its head; there is \
             nothing to land",
            facts.run_id
        ),
    })
}

/// [`choose_source`] for `view`, with the facts read from disk and the project.
fn run_source(project: &Path, view: &OneoffView, head: &str) -> Result<RunSource, String> {
    let record = &view.record;
    let clone = view.run_dir.join(WORK_DIR);
    let clone_present = fs::symlink_metadata(&clone).is_ok_and(|m| m.file_type().is_dir());
    let merged_into = record
        .landed
        .iter()
        .rev()
        .find(|l| l.kind.puts_work_on_target())
        .map(|l| l.detail.as_str());
    choose_source(
        &SourceFacts {
            run_id: &record.run_id,
            pruned_note: record.pruned.as_ref().map(|p| p.note.as_str()),
            clone_present,
            tracked_at_head: ref_commit(project, &tracked_ref(&record.run_id)).as_deref()
                == Some(head),
            merged_into,
        },
        &clone,
    )
}

/// Land the run's recorded head in the branch the run was dispatched from (or `--into`):
/// a fast-forward when that branch's tip is an ancestor of it, otherwise a merge commit
/// made in a private clone (with a sandboxed agent session resolving conflicts). Never a
/// rebase (D-04). A target that is not checked out is moved by `update-ref` with the
/// expected old value; the working tree is never touched (quick 261103-o2r, D-C). When
/// the landing succeeds the run's clone and bulky files are removed (D-A); a cleanup
/// problem is a line of the output, never a failure.
pub fn merge(project: &Path, run_id: &str, opts: &LandOptions) -> Result<String, String> {
    let project = fs::canonicalize(project).map_err(|e| format!("project: {e}"))?;
    // The land lock is released when `merge_locked` returns; the slim takes it again.
    let (message, branch) = merge_locked(&project, run_id, opts)?;
    let slim = super::recycle::slim_run(
        &project,
        run_id,
        &format!("removed after landing in {branch}"),
    );
    Ok(format!("{message}\n{}", slim.line()))
}

/// [`merge`] under the run's land lock. Returns the output and the branch landed into.
fn merge_locked(
    project: &Path,
    run_id: &str,
    opts: &LandOptions,
) -> Result<(String, String), String> {
    let (_, first_view) = view_of(project, run_id)?;
    // I-13: one landing of a run at a time; the view is read again under the lock.
    let _land_lock = take_land_lock(&first_view.run_dir, run_id)?;
    // klf I-09: a stop request left by an earlier landing must not stop this one.
    super::cancel::clear_request(&first_view.run_dir.join(super::LAND_DIR));
    let (_, view) = view_of(project, run_id)?;
    let record = &view.record;
    let head = record.head.clone();
    let mut facts = MergeFacts {
        live: view.live.may_be_live(),
        finished_with_head: record.status == RecordStatus::Finished && head.is_some(),
        commits: record.commits.unwrap_or(0),
        planted: 0,
        dirty: false,
        target_checked_out: true,
        branch: Some(String::new()),
        ancestor: true,
    };
    merge_decision(&facts)?;
    let head = head.unwrap_or_default();

    // D-C: where to land, looked up in the project.
    let checkout = git_in(project, &["symbolic-ref", "-q", "--short", "HEAD"]);
    let checkout_branch =
        (checkout.success && push::plausible_branch(&checkout.stdout)).then_some(checkout.stdout);
    let choice = choose_target(opts.into.as_deref(), &record.land_target())?;
    let target = resolve_target(project, choice, checkout_branch.as_deref())?;
    facts.target_checked_out = target.checked_out;
    facts.branch = checkout_branch;

    // pd7 D-02: a head the target already contains is recorded as landed. This reads only
    // the project's own objects (a head never fetched fails the ancestry test), so it
    // needs neither the clone nor the planted gate, and a dirty checkout does not block it:
    // nothing in the checkout changes.
    if !target.branch.is_empty()
        && !view
            .record
            .landed
            .iter()
            .any(|l| l.kind.puts_work_on_target())
    {
        let tip_ref = if target.checked_out {
            "HEAD".to_owned()
        } else {
            format!("refs/heads/{}", target.branch)
        };
        if let Some(tip) = ref_commit(project, &tip_ref) {
            if git_in(project, &["merge-base", "--is-ancestor", &head, &tip]).success {
                let mut updated = view.record.clone();
                let message = record_already_landed(
                    project,
                    &view.run_dir,
                    &mut updated,
                    &target.branch,
                    &tip,
                )?;
                return Ok((message, target.branch));
            }
        }
    }

    // D-A: the run's clone, or, once it was removed, the tracked branch.
    let source = run_source(project, &view, &head)?;
    // I-4: the planted gate reads files only, and runs before any git touches the clone.
    if matches!(source, RunSource::Clone(_)) {
        facts.planted = planted_count(&view);
    }
    if target.checked_out {
        let status = git_in(project, &["status", "--porcelain"]);
        if !status.success {
            return Err(format!("git status failed: {}", status.last_error));
        }
        facts.dirty = !status.stdout.is_empty();
    }
    merge_decision(&facts)?;

    let source_path = match &source {
        RunSource::Clone(path) => path.clone(),
        RunSource::Tracked => project.to_path_buf(),
    };
    let branch_ref = tracked_ref(run_id);
    let fetch = push::git(
        vec![
            "-C".into(),
            project.as_os_str().to_owned(),
            "fetch".into(),
            "--quiet".into(),
            "--no-tags".into(),
            "--no-recurse-submodules".into(),
            source_path.as_os_str().to_owned(),
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
        project,
        &["rev-parse", "--verify", "--quiet", "FETCH_HEAD^{commit}"],
    );
    if !fetched.success || fetched.stdout != head {
        return Err("the fetched tip is not the recorded head; nothing was merged".to_owned());
    }
    let tip_ref = if target.checked_out {
        "HEAD".to_owned()
    } else {
        format!("refs/heads/{}", target.branch)
    };
    let Some(tip) = ref_commit(project, &tip_ref) else {
        return Err(format!("could not read {}", target.branch));
    };
    let ancestor = git_in(project, &["merge-base", "--is-ancestor", &tip, &head]);
    facts.ancestor = ancestor.success;
    merge_decision(&facts)?;
    let branch = target.branch.clone();
    if !facts.ancestor {
        // I-24: a head the target already contains would make a `--no-ff` merge that
        // creates nothing.
        if git_in(project, &["merge-base", "--is-ancestor", &head, &tip]).success {
            return Err(format!(
                "the run already landed in {branch}; nothing to merge"
            ));
        }
        let message = super::resolve::land_by_merge(project, &view, &target, &source_path, opts)?;
        return Ok((message, branch));
    }

    // klf I-08: the target's tip before the fast-forward starts the landed range.
    let old_head = Some(tip.clone());
    if target.checked_out {
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
    } else {
        // D-C, I-05: not checked out, so nothing but the ref moves.
        update_target(project, &target.branch, &head, &tip, run_id)?;
    }
    // mnr D-04: name what actually landed. The target advanced from `old_head` to
    // `head`, which can be more than the agent's own commits (a resumed run carries the
    // host merge commit and the commits it was resumed from).
    let landed = old_head.as_deref().and_then(|old| {
        let counted = git_in(project, &["rev-list", "--count", &format!("{old}..{head}")]);
        counted
            .success
            .then(|| counted.stdout.trim().parse::<u32>().ok())
            .flatten()
    });
    let mut lines = vec![landed_message(
        landed,
        facts.commits,
        run_id,
        &branch,
        &head[..head.len().min(12)],
    )];
    if !target.checked_out {
        lines.push(super::resolve::not_checked_out_line(&branch));
    }
    // klf I-06: the delete first, then ONE write with the landing and the branch fields.
    let mut updated = view.record.clone();
    updated.landed.push(Landing {
        how: Some(MergeHow::Ff),
        merge_sha: Some(head.clone()),
        range: old_head.map(|old| format!("{old}..{head}")),
        ..Landing::plain(LandingKind::Merged, rfc3339(Utc::now()), branch.clone())
    });
    updated.land_failure = None;
    if let Some(line) = clear_tracking_after_landing(project, run_id, &head, &mut updated) {
        lines.push(line);
    }
    if let Err(e) = write_record(&view.run_dir, &updated) {
        eprintln!("task land: the landing succeeded but could not be recorded: {e}");
    }
    lines.extend(sensitive_warning(&record.changed_files));
    Ok((lines.join("\n"), branch))
}

/// The commit on `tip`'s first-parent history that brought `head` in: the oldest commit
/// of `head..tip` along the first-parent line that has `head` as an ancestor (the merge
/// commit of a `--no-ff` merge). `head` itself when it sits on that line (a fast-forward,
/// or a head that is the tip). `None` when git cannot say (quick 261103-pd7, D-02).
pub(crate) fn containing_commit(project: &Path, head: &str, tip: &str) -> Option<String> {
    let range = format!("{head}..{tip}");
    let first = git_in(project, &["rev-list", "--first-parent", &range]);
    let path = git_in(project, &["rev-list", "--ancestry-path", &range]);
    if !first.success || !path.success {
        return None;
    }
    let on_path: std::collections::HashSet<&str> = path.stdout.lines().collect();
    let oldest = first.stdout.lines().rev().find(|c| on_path.contains(c));
    let Some(oldest) = oldest else {
        return Some(head.to_owned());
    };
    let parent = git_in(project, &["rev-parse", "--verify", &format!("{oldest}^1")]);
    if !parent.success {
        return None;
    }
    if parent.stdout == head {
        return Some(head.to_owned());
    }
    Some(oldest.to_owned())
}

/// Record that the run's head is already in `branch` (whose tip is `tip`): an
/// `already-landed` landing, `land_failure` cleared, the tracked branch deleted, and one
/// write of the record. The caller holds the land lock and slims afterwards. The ancestry
/// is checked again here, on the project's own objects. Returns the line to print.
pub(crate) fn record_already_landed(
    project: &Path,
    run_dir: &Path,
    record: &mut RunRecord,
    branch: &str,
    tip: &str,
) -> Result<String, String> {
    let head = record
        .head
        .clone()
        .ok_or_else(|| "the run has no recorded head".to_owned())?;
    if !git_in(project, &["merge-base", "--is-ancestor", &head, tip]).success {
        return Err(format!("the run's head is not in {branch}"));
    }
    let merge_sha = containing_commit(project, &head, tip).unwrap_or_else(|| tip.to_owned());
    record.landed.push(Landing {
        merge_sha: Some(merge_sha.clone()),
        target_tip: Some(tip.to_owned()),
        ..Landing::plain(
            LandingKind::AlreadyLanded,
            rfc3339(Utc::now()),
            branch.to_owned(),
        )
    });
    record.land_failure = None;
    let run_id = record.run_id.clone();
    let mut lines = vec![format!(
        "the run's head is already in {branch} ({}); recorded as landed",
        &merge_sha[..merge_sha.len().min(7)]
    )];
    if let Some(line) = clear_tracking_after_landing(project, &run_id, &head, record) {
        lines.push(line);
    }
    if let Err(e) = write_record(run_dir, record) {
        eprintln!("task land: the landing was found but could not be recorded: {e}");
    }
    Ok(lines.join("\n"))
}

/// The line a fast-forward landing prints (mnr, D-04): `landed` is what the branch
/// advanced by (`git rev-list --count old..new`), `agent` the run's own commit count.
/// Without a count the agent's is all that is known, and the line makes no claim beyond
/// it.
pub(crate) fn landed_message(
    landed: Option<u32>,
    agent: u32,
    run_id: &str,
    branch: &str,
    short: &str,
) -> String {
    match landed {
        Some(n) if n == agent => {
            format!("landed {n} commit(s) of {run_id} on {branch} (now at {short})")
        }
        Some(n) => format!(
            "landed {n} commit(s) of {run_id} on {branch} ({agent} by the agent; now at {short})"
        ),
        None => format!(
            "landed {agent} commit(s) by the agent of {run_id} on {branch} (now at {short})"
        ),
    }
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
    // o2r D-A: the run's clone, or, once it was removed, the project's tracked branch
    // (the same branch name; the work was gated before that branch was made).
    let head = record.head.clone().unwrap_or_default();
    let source = run_source(&project, &view, &head)?;
    let work = match &source {
        RunSource::Clone(path) => {
            let planted = planted_count(&view);
            if planted > 0 {
                return Err(format!(
                    "{planted} git-relevant planted finding(s) in the run's clone; nothing was pushed (discard it)"
                ));
            }
            path.clone()
        }
        RunSource::Tracked => project.clone(),
    };
    let branch = format!("{BRANCH_PREFIX}{run_id}");
    let prior = record.last_pushed_sha();
    let scan_dir = mirror
        .parent()
        .unwrap_or(driver_root)
        .join(format!("scan-{run_id}"));
    let pushed = push_tip(&TipPush {
        mirror: &mirror,
        url: &url,
        checkout: &work,
        branch: &branch,
        private_ref: &private_ref,
        dest: &dest,
        start_sha: record.base.as_deref(),
        prior_pushed_sha: prior.as_deref(),
        scan_dir: &scan_dir,
    });
    let slim = |dest: &str| {
        // o2r D-A: the work is on the remote and in `gsd-oneoff/<id>`; the clone goes
        // when the preservation rule agrees, otherwise it stays and the line says why.
        super::recycle::slim_run(&project, run_id, &format!("removed after push to {dest}")).line()
    };
    match pushed {
        TipPushed::Pushed { sha } => {
            record_landing(
                &view,
                Landing::plain(
                    LandingKind::Pushed,
                    rfc3339(Utc::now()),
                    format!("{dest} {sha}"),
                ),
            );
            Ok(format!(
                "pushed {} to {dest}\n{}",
                &sha[..sha.len().min(12)],
                slim(&dest)
            ))
        }
        TipPushed::AlreadyPushed => Ok(format!(
            "already pushed: {dest} has this run's head\n{}",
            slim(&dest)
        )),
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
    #[test]
    fn the_landing_message_counts_what_landed() {
        use super::landed_message;
        let sha = "0123456789ab";
        assert_eq!(
            landed_message(Some(3), 1, "r", "main", sha),
            "landed 3 commit(s) of r on main (1 by the agent; now at 0123456789ab)"
        );
        assert_eq!(
            landed_message(Some(2), 2, "r", "main", sha),
            "landed 2 commit(s) of r on main (now at 0123456789ab)"
        );
        assert_eq!(
            landed_message(None, 4, "r", "main", sha),
            "landed 4 commit(s) by the agent of r on main (now at 0123456789ab)"
        );
    }

    use super::*;

    fn ok_facts() -> MergeFacts {
        MergeFacts {
            live: false,
            finished_with_head: true,
            commits: 2,
            planted: 0,
            dirty: false,
            target_checked_out: true,
            branch: Some("main".to_owned()),
            ancestor: true,
        }
    }

    #[test]
    fn a_target_that_is_not_checked_out_ignores_a_dirty_or_detached_checkout() {
        let elsewhere = MergeFacts {
            target_checked_out: false,
            dirty: true,
            branch: None,
            ..ok_facts()
        };
        assert!(merge_decision(&elsewhere).is_ok());
        // The other refusals still apply.
        let planted = MergeFacts {
            planted: 1,
            ..elsewhere.clone()
        };
        assert!(merge_decision(&planted).unwrap_err().contains("planted"));
        let live = MergeFacts {
            live: true,
            ..elsewhere
        };
        assert!(merge_decision(&live).is_err());
    }

    #[test]
    fn the_target_is_into_then_the_recorded_branch_then_the_checkout() {
        let branch = |name: &str, explicit: bool| TargetChoice::Branch {
            name: name.to_owned(),
            explicit,
        };
        let recorded = LandTarget::Branch("main".to_owned());
        assert_eq!(choose_target(None, &recorded), Ok(branch("main", false)));
        assert_eq!(
            choose_target(Some("dev"), &recorded),
            Ok(branch("dev", true))
        );
        assert_eq!(
            choose_target(None, &LandTarget::Legacy),
            Ok(TargetChoice::Checkout)
        );
        assert_eq!(
            choose_target(Some("dev"), &LandTarget::Legacy),
            Ok(branch("dev", true))
        );
        let detached = LandTarget::Detached {
            tip: "0123456789abcdef".to_owned(),
        };
        let why = choose_target(None, &detached).unwrap_err();
        assert!(why.contains("--into") && why.contains("@0123456"), "{why}");
        assert_eq!(
            choose_target(Some("dev"), &detached),
            Ok(branch("dev", true))
        );
    }

    #[test]
    fn worktree_listings_parse_with_and_without_nul_separators() {
        let z = "worktree /p\0HEAD aaa\0branch refs/heads/side\0\0\
                 worktree /other wt\0HEAD bbb\0branch refs/heads/main\0\0\
                 worktree /detached\0HEAD ccc\0detached\0\0";
        let plain = "worktree /p\nHEAD aaa\nbranch refs/heads/side\n\n\
                     worktree /other wt\nHEAD bbb\nbranch refs/heads/main\n\n\
                     worktree /detached\nHEAD ccc\ndetached\n";
        for text in [z, plain] {
            let found = parse_worktrees(text);
            assert_eq!(found.len(), 3, "{found:?}");
            assert_eq!(found[0].path, PathBuf::from("/p"));
            assert_eq!(found[0].branch.as_deref(), Some("refs/heads/side"));
            assert_eq!(found[1].path, PathBuf::from("/other wt"));
            assert_eq!(found[1].branch.as_deref(), Some("refs/heads/main"));
            assert_eq!(found[2].branch, None);
        }
    }

    #[test]
    fn the_run_source_is_the_clone_the_tracked_branch_or_a_refusal() {
        let clone = Path::new("/r/work");
        let facts = |pruned: Option<&'static str>, present, tracked, merged| SourceFacts {
            run_id: "oneoff-1",
            pruned_note: pruned,
            clone_present: present,
            tracked_at_head: tracked,
            merged_into: merged,
        };
        // An existing clone of a record that was not pruned wins, even with a branch.
        assert_eq!(
            choose_source(&facts(None, true, true, None), clone),
            Ok(RunSource::Clone(clone.to_path_buf()))
        );
        // A pruned record never reads a (half-deleted) clone.
        assert_eq!(
            choose_source(&facts(Some("after push"), true, true, None), clone),
            Ok(RunSource::Tracked)
        );
        assert_eq!(
            choose_source(&facts(None, false, true, None), clone),
            Ok(RunSource::Tracked)
        );
        let why = choose_source(&facts(Some("after push"), false, false, None), clone).unwrap_err();
        assert!(why.contains("gsd-oneoff/oneoff-1"), "{why}");
        let merged = choose_source(
            &facts(Some("after landing"), false, false, Some("main")),
            clone,
        )
        .unwrap_err();
        assert!(merged.contains("its work is in main"), "{merged}");
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

    /// The module's hardened git with a fixed identity and default branch.
    fn tgit(dir: &Path, args: &[&str]) -> String {
        let mut full = vec!["-c", "user.name=t", "-c", "user.email=t@example.com"];
        full.extend_from_slice(args);
        let run = push::git_in(dir, &full, push::GIT_QUICK);
        assert!(run.success, "git {args:?}: {}", run.last_error);
        run.stdout
    }

    /// A repo with `main` holding one commit, and a side branch `run` one commit ahead.
    fn two_branch_repo() -> (tempfile::TempDir, PathBuf, String) {
        let root = tempfile::TempDir::new().unwrap();
        let repo = root.path().join("repo");
        fs::create_dir(&repo).unwrap();
        tgit(&repo, &["init", "-q", "-b", "main"]);
        fs::write(repo.join("a.txt"), "a").unwrap();
        tgit(&repo, &["add", "-A"]);
        tgit(&repo, &["commit", "-qm", "init"]);
        tgit(&repo, &["switch", "-q", "-c", "run"]);
        fs::write(repo.join("b.txt"), "b").unwrap();
        tgit(&repo, &["add", "-A"]);
        tgit(&repo, &["commit", "-qm", "run work"]);
        let head = tgit(&repo, &["rev-parse", "HEAD"]);
        tgit(&repo, &["switch", "-q", "main"]);
        (root, repo, head)
    }

    fn commit_file(repo: &Path, name: &str) -> String {
        fs::write(repo.join(name), name).unwrap();
        tgit(repo, &["add", "-A"]);
        tgit(repo, &["commit", "-qm", name]);
        tgit(repo, &["rev-parse", "HEAD"])
    }

    #[test]
    fn containing_commit_names_the_merge_that_brought_the_head_in() {
        let (_root, repo, head) = two_branch_repo();
        commit_file(&repo, "c.txt");
        tgit(&repo, &["merge", "--no-ff", "-q", "-m", "m", "run"]);
        let merge = tgit(&repo, &["rev-parse", "HEAD"]);
        assert_eq!(containing_commit(&repo, &head, &merge), Some(merge.clone()));
        // More commits after the merge: still the merge.
        commit_file(&repo, "d.txt");
        let tip = commit_file(&repo, "e.txt");
        assert_eq!(containing_commit(&repo, &head, &tip), Some(merge));
    }

    #[test]
    fn containing_commit_is_the_head_when_it_sits_on_the_first_parent_line() {
        let (_root, repo, head) = two_branch_repo();
        // The tip is the head.
        tgit(&repo, &["merge", "--ff-only", "-q", "run"]);
        assert_eq!(containing_commit(&repo, &head, &head), Some(head.clone()));
        // Later commits on top of the head.
        let tip = commit_file(&repo, "later.txt");
        assert_eq!(containing_commit(&repo, &head, &tip), Some(head));
    }

    #[test]
    fn containing_commit_is_none_for_an_unknown_head() {
        let (_root, repo, _) = two_branch_repo();
        let tip = tgit(&repo, &["rev-parse", "HEAD"]);
        assert_eq!(containing_commit(&repo, &"f".repeat(40), &tip), None);
    }

    #[test]
    fn the_land_lock_file_is_unlinked_when_released() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = land_lock_path(dir.path());
        let held = take_land_lock(dir.path(), "oneoff-x").unwrap();
        assert!(path.exists(), "the file exists while the lock is held");
        assert!(landing_in_progress(dir.path()));
        // A second take waits its bounded time and refuses; try_ refuses at once.
        let started = std::time::Instant::now();
        assert!(try_land_lock(dir.path(), "oneoff-x").is_none());
        assert!(started.elapsed() < Duration::from_millis(100));
        assert!(take_land_lock(dir.path(), "oneoff-x").is_err());
        assert!(
            path.exists(),
            "a refused take leaves the holder's file alone"
        );
        drop(held);
        assert!(!path.exists(), "the file is gone after the release");
        assert!(!landing_in_progress(dir.path()), "a probe never creates it");
        assert!(!path.exists());
        // A try that wins unlinks too, and the path can be taken again.
        drop(try_land_lock(dir.path(), "oneoff-x").expect("free"));
        assert!(!path.exists());
        drop(take_land_lock(dir.path(), "oneoff-x").unwrap());
        assert!(!path.exists());
    }

    #[test]
    fn a_guard_whose_file_was_replaced_does_not_unlink_the_new_one() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = land_lock_path(dir.path());
        let held = take_land_lock(dir.path(), "oneoff-x").unwrap();
        fs::remove_file(&path).unwrap();
        fs::write(&path, b"").unwrap();
        drop(held);
        assert!(path.exists(), "somebody else's file is not ours to unlink");
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
