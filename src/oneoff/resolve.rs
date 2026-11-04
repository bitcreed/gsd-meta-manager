//! The non-fast-forward landing: a merge in a private clone, a sandboxed resolver session
//! for conflicts, the host's own checks, and the final fast-forward (quick 261103-hx7,
//! D-04, D-05, D-07, D-08, C-09..C-25, I-10..I-20).
//!
//! One attempt, from a fresh `--no-hardlinks` clone of the project at the target tip in
//! `<run-dir>/land/<n>/work`:
//!
//! 1. fetch the run's branch from the run's own clone (hardened, verified to be the
//!    recorded head) and `git merge --no-ff` it, hooks off and the host's global git
//!    config off, with the project's identity passed on the command line (I-10);
//! 2. a clean merge is committed by git with the host's subject (C-12). A conflicted one
//!    goes to ONE sandboxed resolver session that shares the run's session runner
//!    ([`super::launch::run_sandboxed_session`]), with the run's remaining budget and a
//!    15 minute wall clock (C-22, C-24);
//! 3. the host never trusts the resolver's report alone (C-15): the planted-file gate on
//!    the land clone, a valid `done` report with a `--verified` item, HEAD's parents
//!    exactly `[target tip, run head]`, a clean tree and no conflict marker left in the
//!    conflicted files;
//! 4. the merge commit is fetched into the project and, while the checkout is still
//!    clean, on the same branch and at the target tip, the user's own
//!    `git merge --ff-only` lands it (C-17). A moved target redoes the whole merge in a
//!    new attempt dir, at most [`MAX_LAND_ATTEMPTS`] in total (C-18).
//!
//! The user's tree changes only at that last step. Every failure leaves the checkout
//! and the tracked branch as they were, keeps every attempt dir and records
//! `land_failure` in `run.json` (D-07, I-15). Agent facts come only from
//! [`AgentRuntime`]; this file names none.

use std::ffi::OsString;
use std::fs;
use std::io::{Read, Write};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::path::{Path, PathBuf};
use std::time::Duration;

use chrono::Utc;

use crate::launcher::host::{detect, Posture};
use crate::launcher::process::run_bounded_with_stderr;
use crate::launcher::push::{self, planted};
use crate::state_reader::driver_frontmatter::read_run_file_capped;

use super::cancel::{read_request, StopKind};
use super::land::{
    clear_tracking_after_landing, git_relevant_planted, record_already_landed, sensitive_warning,
    tracked_ref, update_target, LandOptions, Target,
};
use super::launch::{run_sandboxed_session, SessionEvent, SessionSpec};
use super::merge::{
    clean_text, has_conflict_markers, landed_line, merge_message, merge_parents_ok, parse_unmerged,
    recorded_names, resolver_budget, test_command_from, MARKER_READ_CAP, MAX_LAND_ATTEMPTS,
    RESOLVER_CAP_MINUTES,
};
use super::prompt::{resolver, ResolverPrompt};
use super::report::{read_validated, ReportStatus};
use super::runtime::{AgentRuntime, HostEnd};
use super::store::{
    rfc3339, write_record, LandFailure, Landing, LandingKind, MergeHow, OneoffView, ResumeMerge,
    RunRecord,
};
use super::stream::spend_of_file;
use super::{LAND_DIR, LAUNCH_LOG, REPORT_FILE, REPORT_SUBDIR, STREAM_FILE, WORK_DIR};

/// The clone of a large repository may take a while.
const CLONE_BUDGET: Duration = Duration::from_secs(600);

/// The merge itself, and the user's final fast-forward.
const MERGE_BUDGET: Duration = Duration::from_secs(300);

/// The most conflicted files the host will check and the resolver be asked to resolve.
const MAX_RESOLVABLE_FILES: usize = 1000;

/// The ref in the land clone that pins the merge commit while it is fetched out.
const MERGE_REF: &str = "refs/gsdmm/land";

/// What one attempt came to.
enum Step {
    /// The target branch now points at the merge commit.
    Landed {
        merge_sha: String,
        how: MergeHow,
        conflicted: Vec<String>,
        /// The target tip the attempt merged onto: the landed range starts here.
        tip: String,
    },
    /// The run's head is already an ancestor of the target's tip (the target moved onto
    /// it while the landing ran): record it as landed instead of merging (quick
    /// 261103-pd7, D-02).
    AlreadyLanded { tip: String },
    /// The target moved while the merge was made: redo it on the new tip.
    Moved,
    /// Nothing landed, and trying again cannot help.
    Failed(String),
    /// The user stopped, killed or paused the landing (klf, C-13, C-14, I-15): nothing
    /// landed and no further attempt starts.
    Stopped(StopKind),
}

/// The stop the user asked of this landing, if any (klf, I-15): the request file in
/// `<run-dir>/land/`, or a hard stop when the caller's own stop flag is set (Ctrl-C or
/// TERM of `task land merge`, the TUI's per-run flag).
fn stop_wanted(land: &Land<'_>) -> Option<StopKind> {
    let from_file = read_request(&land.run_dir.join(LAND_DIR))
        .ok()
        .flatten()
        .map(|r| r.kind);
    let from_flag = land
        .opts
        .user_stop
        .load(std::sync::atomic::Ordering::SeqCst)
        .then_some(StopKind::Hard);
    match (from_file, from_flag) {
        (Some(a), Some(b)) => Some(a.stronger(b)),
        (a, b) => a.or(b),
    }
}

/// The `land_failure` reason of a landing the user ended (klf, C-13, C-14).
fn stop_reason(kind: StopKind) -> &'static str {
    match kind {
        StopKind::Hard => "stopped by user",
        StopKind::Kill => "killed by user",
        StopKind::Pause => "paused",
    }
}

/// `git -C <work> <args>` with hooks off and the host's global and system config off.
fn wgit(work: &Path, args: &[&str]) -> push::GitRun {
    let mut full = vec![OsString::from("-C"), work.as_os_str().to_owned()];
    full.extend(args.iter().map(OsString::from));
    push::git_mirror_only(full, push::GIT_QUICK)
}

/// `git -C <project> <args>` (hardened, the user's configuration on).
fn pgit(project: &Path, args: &[&str]) -> push::GitRun {
    push::git_in(project, args, push::GIT_QUICK)
}

/// The commit a `git rev-parse --verify` run printed, if it succeeded.
fn rev_of(run: &push::GitRun) -> Option<String> {
    (run.success && push::is_hex_sha(&run.stdout)).then(|| run.stdout.clone())
}

/// Append one line to `<dir>/launch.log`, the host's step log of an attempt (I-18).
fn log_line(dir: &Path, text: &str) {
    let opened = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(dir.join(LAUNCH_LOG));
    if let Ok(mut file) = opened {
        let _ = writeln!(file, "{} {text}", rfc3339(Utc::now()));
    }
}

/// Claim `<run-dir>/land/<n>` with an atomic, non-recursive `mkdir`: the next free
/// number, never one a kept attempt already has (I-15).
pub(crate) fn claim_attempt(run_dir: &Path) -> Result<(u32, PathBuf), String> {
    let land = run_dir.join(LAND_DIR);
    match fs::DirBuilder::new().mode(0o700).create(&land) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(format!("create {}: {e}", land.display())),
    }
    for n in 1..1000_u32 {
        let dir = land.join(n.to_string());
        match fs::DirBuilder::new().mode(0o700).create(&dir) {
            Ok(()) => return Ok((n, dir)),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(format!("create {}: {e}", dir.display())),
        }
    }
    Err("too many landing attempts of this run".to_owned())
}

/// The project's merge identity, from its effective git config (I-10).
pub(crate) fn merge_identity(project: &Path) -> Result<(String, String), String> {
    let read = |key: &str| {
        let got = pgit(project, &["config", key]);
        (got.success && !got.stdout.is_empty() && !got.stdout.chars().any(char::is_control))
            .then_some(got.stdout)
    };
    match (read("user.name"), read("user.email")) {
        (Some(name), Some(email)) => Ok((name, email)),
        _ => Err("git has no user.name/user.email for this project".to_owned()),
    }
}

/// `workflow.test_command` of the LAND clone's `.planning/config.json` (the target
/// side), when usable (I-20).
fn test_command_of(work: &Path) -> Option<String> {
    let bytes = read_run_file_capped(&work.join(".planning/config.json")).ok()?;
    test_command_from(&String::from_utf8_lossy(&bytes))
}

/// Why `names` still conflict: the first conflicted file that is not a regular file, is
/// too large to check or still holds a marker (I-16). A deleted file passes.
fn first_unresolved(work: &Path, names: &[String]) -> Option<String> {
    let canonical_work = fs::canonicalize(work).ok()?;
    for name in names {
        let shown = clean_text(name, 200);
        let relative = Path::new(name);
        if relative.is_absolute()
            || relative
                .components()
                .any(|c| matches!(c, std::path::Component::ParentDir))
        {
            return Some(format!("{shown}: not a path inside the repository"));
        }
        let path = work.join(relative);
        let meta = match fs::symlink_metadata(&path) {
            Ok(meta) => meta,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
            Err(e) => return Some(format!("{shown}: {e}")),
        };
        if !meta.file_type().is_file() {
            return Some(format!("{shown}: not a regular file"));
        }
        if meta.len() > MARKER_READ_CAP {
            return Some(format!("{shown}: too large to check"));
        }
        let inside = path
            .parent()
            .and_then(|parent| fs::canonicalize(parent).ok())
            .is_some_and(|parent| parent.starts_with(&canonical_work));
        if !inside {
            return Some(format!("{shown}: not a path inside the repository"));
        }
        let mut bytes = Vec::new();
        let read = fs::File::open(&path)
            .and_then(|file| file.take(MARKER_READ_CAP + 1).read_to_end(&mut bytes));
        if let Err(e) = read {
            return Some(format!("{shown}: {e}"));
        }
        if has_conflict_markers(&String::from_utf8_lossy(&bytes)) {
            return Some(format!("{shown}: conflict markers remain"));
        }
    }
    None
}

/// Write the record, noting a failure to stderr (the landing outcome stands either way).
fn persist(run_dir: &Path, record: &RunRecord) {
    if let Err(e) = write_record(run_dir, record) {
        eprintln!("task land: could not update run.json: {e}");
    }
}

/// How the final step ended when it did not fail.
enum Forwarded {
    /// The user's `git merge --ff-only` advanced the checkout.
    Landed,
    /// The target branch moved before the fast-forward: redo the merge.
    Moved,
}

/// Which way a conflicted merge runs (klf, C-11).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Direction {
    /// A landing: the run's head is merged into the target branch's tip (parents
    /// `[tip, head]`).
    IntoTarget,
    /// A resume from master: the checkout's tip is merged into the run's head (parents
    /// `[head, tip]`).
    IntoRun,
}

/// What one resolver session needs, for a landing and for a resume merge alike.
pub(crate) struct ResolveJob<'a> {
    pub project: &'a Path,
    /// The run dir that holds the stream used for spend, and the record that is written.
    pub run_dir: &'a Path,
    pub run_id: &'a str,
    /// The checkout's branch.
    pub target_branch: &'a str,
    /// The checkout branch's tip.
    pub tip: &'a str,
    /// The run's head.
    pub head: &'a str,
    pub direction: Direction,
    pub egress_allow: &'a [String],
    pub user_stop: &'a std::sync::atomic::AtomicBool,
    /// Where a stop request is read from while the resolver runs.
    pub control: &'a Path,
}

/// Everything one attempt shares.
struct Land<'a> {
    project: &'a Path,
    run_dir: &'a Path,
    run_id: &'a str,
    head: &'a str,
    target_branch: &'a str,
    /// The target branch is the project checkout's current branch (quick 261103-o2r,
    /// D-C): the final step is the user's `git merge --ff-only`. Otherwise it is an
    /// `update-ref` with the expected old value and the working tree is never touched.
    checked_out: bool,
    /// Where the run's work is fetched from: the run's clone, or the project itself when
    /// the clone was removed and `gsd-oneoff/<id>` holds the work (o2r, D-A).
    source: &'a Path,
    opts: &'a LandOptions,
}

/// Land the run's `head` into `target_branch` of `project` with a merge commit, a
/// resolver session resolving conflicts when there are any. `view` is the finished,
/// landable run; the caller holds the run's land lock and has made every precondition
/// check (the checkout is clean and on `target_branch` when `target.checked_out`, the
/// planted gate is clean when the work comes from the run's clone).
pub(crate) fn land_by_merge(
    project: &Path,
    view: &OneoffView,
    target: &Target,
    source: &Path,
    opts: &LandOptions,
) -> Result<String, String> {
    let mut record = view.record.clone();
    let head = record
        .head
        .clone()
        .ok_or_else(|| "the run has no recorded head to merge".to_owned())?;
    let land = Land {
        project,
        run_dir: &view.run_dir,
        run_id: &view.record.run_id,
        head: &head,
        target_branch: &target.branch,
        checked_out: target.checked_out,
        source,
        opts,
    };
    let mut attempts: Vec<String> = Vec::new();
    let mut conflicted: Vec<String> = Vec::new();

    let failure = 'tries: {
        for _ in 0..MAX_LAND_ATTEMPTS {
            match attempt(&land, &mut record, &mut attempts, &mut conflicted) {
                Step::Landed {
                    merge_sha,
                    how,
                    conflicted,
                    tip,
                } => {
                    return Ok(landed(
                        &land,
                        &mut record,
                        attempts,
                        &merge_sha,
                        how,
                        conflicted,
                        &tip,
                    ))
                }
                Step::AlreadyLanded { tip } => {
                    match record_already_landed(
                        project,
                        land.run_dir,
                        &mut record,
                        land.target_branch,
                        &tip,
                    ) {
                        Ok(line) => return Ok(line),
                        Err(reason) => break 'tries (reason, None),
                    }
                }
                Step::Moved => {}
                Step::Failed(reason) => break 'tries (reason, None),
                Step::Stopped(kind) => {
                    break 'tries (
                        format!(
                            "the landing was {}; nothing was merged",
                            match kind {
                                StopKind::Pause => "paused",
                                StopKind::Hard => "stopped",
                                StopKind::Kill => "killed",
                            }
                        ),
                        Some(kind),
                    )
                }
            }
        }
        (
            "the target kept moving; nothing was merged".to_owned(),
            None,
        )
    };

    let (failure, stop) = failure;
    let reason = match stop {
        Some(kind) => stop_reason(kind).to_owned(),
        None => clean_text(&failure, 400),
    };
    record.land_failure = Some(LandFailure {
        at: rfc3339(Utc::now()),
        reason: reason.clone(),
        conflicted,
        attempts: attempts.clone(),
        stop,
    });
    persist(land.run_dir, &record);
    let kept = if attempts.is_empty() {
        String::new()
    } else {
        format!(" ({} kept under land/)", attempts.join(", "))
    };
    if stop.is_some() {
        return Err(format!("{failure}{kept}"));
    }
    Err(format!("{reason}{kept}"))
}

/// Record the landing and render the success lines.
fn landed(
    land: &Land<'_>,
    record: &mut RunRecord,
    attempts: Vec<String>,
    merge_sha: &str,
    how: MergeHow,
    conflicted: Vec<String>,
    tip: &str,
) -> String {
    let mut extra_line = None;
    // klf I-06: the delete runs BEFORE the one write, so the record never says the
    // branch is still there once it is not.
    record.landed.push(Landing {
        kind: LandingKind::Merged,
        target_tip: None,
        at: rfc3339(Utc::now()),
        detail: land.target_branch.to_owned(),
        how: Some(how),
        merge_sha: Some(merge_sha.to_owned()),
        conflicted: conflicted.clone(),
        attempts: if how == MergeHow::Resolved {
            attempts.clone()
        } else {
            Vec::new()
        },
        range: Some(format!("{tip}..{merge_sha}")),
    });
    // A later success clears the latest failure (I-19).
    record.land_failure = None;
    // C-07: the merge made the run's head reachable from the target.
    if let Some(line) = clear_tracking_after_landing(land.project, land.run_id, land.head, record) {
        extra_line = Some(line);
    }
    persist(land.run_dir, record);
    let mut lines = vec![landed_line(
        land.run_id,
        land.target_branch,
        merge_sha,
        how == MergeHow::Resolved,
        &conflicted,
    )];
    if !land.checked_out {
        lines.push(not_checked_out_line(land.target_branch));
    }
    lines.extend(extra_line);
    lines.extend(sensitive_warning(&record.changed_files));
    lines.join("\n")
}

/// The line a landing into a branch that is not checked out ends with (o2r, D-C).
pub(crate) fn not_checked_out_line(branch: &str) -> String {
    format!("{branch} is not checked out: it was advanced by update-ref; the working tree was not touched")
}

/// One attempt, start to finish (steps 1-4 of the module header).
fn attempt(
    land: &Land<'_>,
    record: &mut RunRecord,
    attempts: &mut Vec<String>,
    conflicted_out: &mut Vec<String>,
) -> Step {
    let project = land.project;
    let fail = |why: String| Step::Failed(why);

    // klf I-15: a stop between attempts ends the landing here.
    if let Some(kind) = stop_wanted(land) {
        return Step::Stopped(kind);
    }

    // The target branch and its tip, read again for every attempt. A checked-out target
    // is the checkout's current branch; any other is read from its ref (o2r, D-C).
    let tip = if land.checked_out {
        let branch = pgit(project, &["symbolic-ref", "-q", "--short", "HEAD"]);
        if !branch.success || branch.stdout != land.target_branch {
            return fail("the checkout switched branches; nothing was merged".to_owned());
        }
        let Some(tip) = rev_of(&pgit(
            project,
            &["rev-parse", "--verify", "--quiet", "HEAD^{commit}"],
        )) else {
            return fail("could not read the checkout's HEAD".to_owned());
        };
        tip
    } else {
        let reference = format!("refs/heads/{}", land.target_branch);
        let Some(tip) = rev_of(&pgit(
            project,
            &[
                "rev-parse",
                "--verify",
                "--quiet",
                &format!("{reference}^{{commit}}"),
            ],
        )) else {
            return fail(format!(
                "the target branch {} no longer exists; nothing was merged",
                land.target_branch
            ));
        };
        tip
    };
    // pd7 D-02: the target moved onto the head while the landing ran.
    if pgit(project, &["merge-base", "--is-ancestor", land.head, &tip]).success {
        return Step::AlreadyLanded { tip };
    }
    let (name, email) = match merge_identity(project) {
        Ok(identity) => identity,
        Err(why) => return fail(why),
    };

    let (n, dir) = match claim_attempt(land.run_dir) {
        Ok(claimed) => claimed,
        Err(why) => return fail(why),
    };
    let label = format!("{LAND_DIR}/{n}");
    attempts.push(label.clone());
    let work = dir.join(WORK_DIR);
    log_line(
        &dir,
        &format!("attempt {n}: target {} at {tip}", land.target_branch),
    );

    // C-11: a fresh clone of the target tip.
    let clone = push::git(
        vec![
            "clone".into(),
            "--no-hardlinks".into(),
            "--quiet".into(),
            project.as_os_str().to_owned(),
            work.as_os_str().to_owned(),
        ],
        CLONE_BUDGET,
    );
    if !clone.success {
        return fail(format!("could not clone the project: {}", clone.last_error));
    }
    let switch = wgit(
        &work,
        &["checkout", "--quiet", "-B", land.target_branch, &tip],
    );
    if !switch.success {
        return fail(format!(
            "could not check out the target tip: {}",
            switch.last_error
        ));
    }

    // The run's branch, from the run's own clone; the planted gate on that clone has
    // passed before the landing started.
    let reference = tracked_ref(land.run_id);
    let fetch = push::git(
        vec![
            "-C".into(),
            work.as_os_str().to_owned(),
            "fetch".into(),
            "--quiet".into(),
            "--no-tags".into(),
            "--no-recurse-submodules".into(),
            land.source.as_os_str().to_owned(),
            format!("{reference}:{reference}").into(),
        ],
        push::GIT_FETCH,
    );
    if !fetch.success {
        return fail(format!(
            "could not fetch the run's branch: {}",
            fetch.last_error
        ));
    }
    if rev_of(&wgit(
        &work,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("{reference}^{{commit}}"),
        ],
    ))
    .as_deref()
        != Some(land.head)
    {
        return fail("the fetched tip is not the recorded head; nothing was merged".to_owned());
    }

    // C-11, C-12: the merge. Git commits a clean one with the host's subject.
    let headline = record
        .report
        .as_ref()
        .map(|r| r.headline.as_str())
        .unwrap_or_default();
    let subject = merge_message(land.run_id, headline);
    let merge = wgit(
        &work,
        &[
            "-c",
            &format!("user.name={name}"),
            "-c",
            &format!("user.email={email}"),
            "merge",
            "--no-ff",
            "--no-edit",
            "-m",
            &subject,
            land.head,
        ],
    );
    let how;
    let names;
    if merge.success {
        let head_now = rev_of(&wgit(
            &work,
            &["rev-parse", "--verify", "--quiet", "HEAD^{commit}"],
        ));
        if head_now.as_deref() == Some(tip.as_str()) {
            return Step::AlreadyLanded { tip };
        }
        how = MergeHow::Merge;
        names = Vec::new();
        log_line(&dir, "the merge was clean");
    } else {
        let unmerged = wgit(&work, &["ls-files", "-u", "-z"]);
        let found = if unmerged.success {
            parse_unmerged(&unmerged.stdout)
        } else {
            Vec::new()
        };
        if found.is_empty() {
            return fail(format!("the merge failed: {}", merge.last_error));
        }
        if found.len() > MAX_RESOLVABLE_FILES {
            return fail(format!(
                "{} files conflict; that is too many to resolve in one session",
                found.len()
            ));
        }
        *conflicted_out = recorded_names(&found);
        log_line(&dir, &format!("{} file(s) conflict", found.len()));
        let control = land.run_dir.join(LAND_DIR);
        let job = ResolveJob {
            project: land.project,
            run_dir: land.run_dir,
            run_id: land.run_id,
            target_branch: land.target_branch,
            tip: &tip,
            head: land.head,
            direction: Direction::IntoTarget,
            egress_allow: &land.opts.egress_allow,
            user_stop: &land.opts.user_stop,
            control: &control,
        };
        if let Err(why) = resolve_conflicts(&job, record, n, &dir, &work, &found) {
            // A stop the user asked for explains the failure better than the session's.
            return match stop_wanted(land) {
                Some(kind) => Step::Stopped(kind),
                None => fail(why),
            };
        }
        how = MergeHow::Resolved;
        names = found;
    }

    // C-15 for both kinds: HEAD is a merge of exactly [target tip, run head], so the
    // final fast-forward lands only what was reviewed.
    let parents = wgit(&work, &["rev-list", "--parents", "-n", "1", "HEAD"]);
    if !parents.success || !merge_parents_ok(&parents.stdout, &tip, land.head) {
        return fail(
            "the merge commit does not have exactly the target tip and the run's head as parents"
                .to_owned(),
        );
    }
    let Some(merge_sha) = parents
        .stdout
        .split_whitespace()
        .next()
        .filter(|sha| push::is_hex_sha(sha))
        .map(str::to_owned)
    else {
        return fail("could not read the merge commit".to_owned());
    };
    if how == MergeHow::Resolved {
        if let Some(why) = first_unresolved(&work, &names) {
            return fail(format!("the resolution is not clean: {why}"));
        }
    }
    log_line(&dir, &format!("merge commit {merge_sha}"));

    // klf I-15: nothing lands once a stop was asked, even a merge that resolved or was
    // clean; the attempt dir is kept.
    if let Some(kind) = stop_wanted(land) {
        return Step::Stopped(kind);
    }
    match fast_forward(land, &work, &label, &tip, &merge_sha) {
        Ok(Forwarded::Landed) => Step::Landed {
            merge_sha,
            how,
            conflicted: recorded_names(&names),
            tip,
        },
        Ok(Forwarded::Moved) => {
            log_line(&dir, "the target moved; starting a new attempt");
            Step::Moved
        }
        Err(why) => fail(why),
    }
}

/// What a `--from master` resume needs (quick 261103-klf, C-11, I-20).
pub(crate) struct ResumeMergeJob<'a> {
    pub project: &'a Path,
    /// The NEW run's dir (its `land/<n>` holds the one resolver attempt).
    pub run_dir: &'a Path,
    pub run_id: &'a str,
    pub old_run_id: &'a str,
    /// The new run's clone, switched to its branch at the old head.
    pub work: &'a Path,
    pub old_head: &'a str,
    /// The checkout's branch and its tip, merged into the old head.
    pub branch: &'a str,
    pub tip: &'a str,
    pub egress_allow: &'a [String],
    pub user_stop: &'a std::sync::atomic::AtomicBool,
}

/// Merge the checkout's tip into the old head inside the new run's clone, with parents
/// `[old head, tip]` (quick 261103-klf, C-11, I-20). `Ok(None)`: the tip is already in
/// the old head, nothing to merge. `Ok(Some(..))`: a host-made merge commit, or ONE
/// resolver attempt whose result the host re-checked (planted gate, valid `done` report
/// with `--verified`, exact parents, clean tree, no markers). `Err(..)`: the failure is
/// set and the caller ends the run before the agent session.
#[allow(clippy::result_large_err)]
pub(crate) fn merge_master_into_run(
    job: &ResumeMergeJob<'_>,
    record: &mut RunRecord,
) -> Result<Option<ResumeMerge>, ResumeMerge> {
    let failed = |why: String, attempts: Vec<String>, conflicted: Vec<String>| ResumeMerge {
        how: None,
        branch: clean_text(job.branch, 200),
        merge_sha: None,
        conflicted,
        attempts,
        failure: Some(clean_text(&why, 400)),
    };
    let work = job.work;
    let (name, email) =
        merge_identity(job.project).map_err(|why| failed(why, Vec::new(), Vec::new()))?;
    if wgit(work, &["merge-base", "--is-ancestor", job.tip, "HEAD"]).success {
        return Ok(None);
    }
    let subject = format!(
        "Merge {} into one-off {} (resume {})",
        clean_text(job.branch, 120),
        job.old_run_id,
        job.run_id
    );
    let merge = wgit(
        work,
        &[
            "-c",
            &format!("user.name={name}"),
            "-c",
            &format!("user.email={email}"),
            "merge",
            "--no-ff",
            "--no-edit",
            "-m",
            &subject,
            job.tip,
        ],
    );
    let mut attempts = Vec::new();
    let mut conflicted = Vec::new();
    let how;
    let mut names = Vec::new();
    if merge.success {
        how = MergeHow::Merge;
    } else {
        let unmerged = wgit(work, &["ls-files", "-u", "-z"]);
        let found = if unmerged.success {
            parse_unmerged(&unmerged.stdout)
        } else {
            Vec::new()
        };
        if found.is_empty() {
            return Err(failed(
                format!("the merge of {} failed: {}", job.branch, merge.last_error),
                attempts,
                conflicted,
            ));
        }
        if found.len() > MAX_RESOLVABLE_FILES {
            return Err(failed(
                format!(
                    "{} files conflict; that is too many to resolve in one session",
                    found.len()
                ),
                attempts,
                conflicted,
            ));
        }
        conflicted = recorded_names(&found);
        let (n, dir) = match claim_attempt(job.run_dir) {
            Ok(claimed) => claimed,
            Err(why) => return Err(failed(why, attempts, conflicted)),
        };
        attempts.push(format!("{LAND_DIR}/{n}"));
        log_line(
            &dir,
            &format!("resume merge: {} file(s) conflict", found.len()),
        );
        let resolve = ResolveJob {
            project: job.project,
            run_dir: job.run_dir,
            run_id: job.run_id,
            target_branch: job.branch,
            tip: job.tip,
            head: job.old_head,
            direction: Direction::IntoRun,
            egress_allow: job.egress_allow,
            user_stop: job.user_stop,
            control: job.run_dir,
        };
        if let Err(why) = resolve_conflicts(&resolve, record, n, &dir, work, &found) {
            return Err(failed(why, attempts, conflicted));
        }
        how = MergeHow::Resolved;
        names = found;
    }
    let parents = wgit(work, &["rev-list", "--parents", "-n", "1", "HEAD"]);
    if !parents.success || !merge_parents_ok(&parents.stdout, job.old_head, job.tip) {
        return Err(failed(
            "the merge commit does not have exactly the run's head and the checkout's tip as parents"
                .to_owned(),
            attempts,
            conflicted,
        ));
    }
    let Some(merge_sha) = parents
        .stdout
        .split_whitespace()
        .next()
        .filter(|sha| push::is_hex_sha(sha))
        .map(str::to_owned)
    else {
        return Err(failed(
            "could not read the merge commit".to_owned(),
            attempts,
            conflicted,
        ));
    };
    if how == MergeHow::Resolved {
        if let Some(why) = first_unresolved(work, &names) {
            return Err(failed(
                format!("the resolution is not clean: {why}"),
                attempts,
                conflicted,
            ));
        }
    }
    Ok(Some(ResumeMerge {
        how: Some(how),
        branch: clean_text(job.branch, 200),
        merge_sha: Some(merge_sha),
        conflicted,
        attempts,
        failure: None,
    }))
}

/// The resolver session of one attempt and the host's checks on what it left.
fn resolve_conflicts(
    job: &ResolveJob<'_>,
    record: &mut RunRecord,
    n: u32,
    dir: &Path,
    work: &Path,
    names: &[String],
) -> Result<(), String> {
    let tip = job.tip;
    // C-22: the run's remaining budget, folded from a legacy stream when unrecorded.
    let spent = record
        .spend_usd
        .unwrap_or_else(|| spend_of_file(&job.run_dir.join(STREAM_FILE)));
    let budget = resolver_budget(record.max_budget_usd, spent)?;

    // The planted baseline of the land clone, before any sandbox exists.
    if let Err(e) = planted::save_baseline_if_absent(dir, &planted::snapshot(work)) {
        log_line(
            dir,
            &format!("no planted-file baseline ({e}); the gate runs in strict mode"),
        );
    }

    // I-11: the resolver never runs unsandboxed.
    let runtime = AgentRuntime::default();
    let credential = runtime
        .credential(Utc::now())
        .map_err(|e| format!("no credential for the conflict resolver: {e}"))?;
    let posture = runtime
        .decide_posture(&detect(), false)
        .map_err(|e| format!("the conflict resolver needs the sandbox: {e}"))?;
    if !matches!(posture, Posture::BwrapScope) {
        return Err("the conflict resolver needs the sandbox".to_owned());
    }
    let home = dirs::home_dir().ok_or_else(|| "the home directory is unknown".to_owned())?;

    let recorded = recorded_names(names);
    let test_command = test_command_of(work);
    let headline = record
        .report
        .as_ref()
        .map(|r| r.headline.clone())
        .unwrap_or_default();
    let prompt = resolver(&ResolverPrompt {
        run_id: job.run_id,
        target_branch: job.target_branch,
        target_tip: tip,
        run_head: job.head,
        headline: &headline,
        conflicted: &recorded,
        test_command: test_command.as_deref(),
        into_run: job.direction == Direction::IntoRun,
    })?;

    let report_dir = dir.join(REPORT_SUBDIR);
    fs::DirBuilder::new()
        .mode(0o700)
        .create(&report_dir)
        .map_err(|e| format!("create {}: {e}", report_dir.display()))?;
    let unit_id = format!("{}-land{n}", job.run_id);
    log_line(
        dir,
        &format!(
            "resolver session {unit_id}: budget ${budget:.2}, {RESOLVER_CAP_MINUTES} min wall clock"
        ),
    );
    let spec = SessionSpec {
        runtime,
        session_dir: dir,
        work,
        report_dir: &report_dir,
        unit_id: &unit_id,
        prompt: &prompt,
        model: &record.model,
        budget_usd: budget,
        cap_minutes: RESOLVER_CAP_MINUTES,
        egress_allow: job.egress_allow,
        posture: &posture,
        credential: &credential,
        overlay: detect().overlay,
        home: &home,
        project: job.project,
        user_stop: job.user_stop,
        inbox: false,
        // klf C-14: the resolver is not asked to pause; its attempt may finish within
        // the pause grace and is then not landed.
        control: Some(job.control),
        pause_message: None,
    };
    let session = run_sandboxed_session(&spec, &mut |event| {
        if let SessionEvent::ScopeUnit(Some(unit)) = event {
            log_line(dir, &format!("scope unit {unit}"));
        }
    });

    // C-23, D-08: the resolver's spend counts against the run, kept even when the
    // session failed to run.
    let session_spend = match &session {
        Ok(run) => run.spend_usd,
        Err(_) => spend_of_file(&dir.join(STREAM_FILE)),
    };
    record.spend_usd = Some(spent + session_spend);
    persist(job.run_dir, record);
    let session = session.map_err(|e| format!("the resolver session could not run: {e}"))?;
    log_line(
        dir,
        &format!(
            "resolver ended: {:?}, exit {:?}, spend ${:.2}",
            session.end, session.rc, session_spend
        ),
    );
    match session.end {
        HostEnd::Exited => {}
        HostEnd::UserStop => return Err("the landing was stopped; nothing was merged".to_owned()),
        HostEnd::Killed { .. } => {
            return Err("the landing was killed; nothing was merged".to_owned())
        }
        HostEnd::Paused => return Err("the landing was paused; nothing was merged".to_owned()),
        HostEnd::WallClock => {
            return Err(format!(
                "the resolver hit its {RESOLVER_CAP_MINUTES} minute wall clock; nothing was merged"
            ))
        }
        HostEnd::SoftCap => {
            return Err("the resolver hit its spend limit; nothing was merged".to_owned())
        }
    }

    // C-15, in this order. The planted gate reads files only.
    let planted_found = git_relevant_planted(dir, work);
    if !planted_found.is_empty() {
        return Err(format!(
            "{} git-relevant planted finding(s) in the land clone; nothing was merged",
            planted_found.len()
        ));
    }
    let report_file = report_dir.join(REPORT_FILE);
    let report = match read_validated(&report_file) {
        Ok(report) => report,
        Err(why) => {
            return Err(if report_file.symlink_metadata().is_err() {
                "the resolver wrote no report".to_owned()
            } else {
                format!(
                    "the resolver's report was refused: {}",
                    clean_text(&why, 200)
                )
            })
        }
    };
    if report.status != ReportStatus::Done {
        return Err(format!(
            "the resolver's report says {}: {}",
            report.status.as_str(),
            clean_text(&report.headline, 200)
        ));
    }
    if report.verified.is_empty() {
        return Err("the resolver's report has no --verified item".to_owned());
    }
    let parents = wgit(work, &["rev-list", "--parents", "-n", "1", "HEAD"]);
    let (first, second) = match job.direction {
        Direction::IntoTarget => (tip, job.head),
        Direction::IntoRun => (job.head, tip),
    };
    if !parents.success || !merge_parents_ok(&parents.stdout, first, second) {
        return Err("the resolver did not commit a merge of the two heads it was given".to_owned());
    }
    let status = wgit(work, &["status", "--porcelain", "--untracked-files=all"]);
    if !status.success || !status.stdout.is_empty() {
        return Err("the resolver left uncommitted or untracked changes".to_owned());
    }
    Ok(())
}

/// The final step (C-17, C-19, I-14): fetch the merge commit into the project, re-check
/// the checkout, and run the user's own `git merge --ff-only`.
///
/// `Err` means nothing landed and no retry helps.
fn fast_forward(
    land: &Land<'_>,
    work: &Path,
    label: &str,
    tip: &str,
    merge_sha: &str,
) -> Result<Forwarded, String> {
    let project = land.project;
    let pinned = wgit(work, &["update-ref", MERGE_REF, merge_sha]);
    if !pinned.success {
        return Err(format!(
            "could not pin the merge commit: {}",
            pinned.last_error
        ));
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
            MERGE_REF.into(),
        ],
        push::GIT_FETCH,
    );
    if !fetch.success {
        return Err(format!(
            "could not fetch the merge commit: {}",
            fetch.last_error
        ));
    }
    if rev_of(&pgit(
        project,
        &["rev-parse", "--verify", "--quiet", "FETCH_HEAD^{commit}"],
    ))
    .as_deref()
        != Some(merge_sha)
    {
        return Err("the fetched merge commit is not the one that was checked".to_owned());
    }

    if !land.checked_out {
        // o2r D-C, I-05: the branch is moved by `update-ref` with the expected old value.
        // A tip that changed, or a worktree that took the branch, refuses with nothing
        // changed; there is no retry.
        return update_target(project, land.target_branch, merge_sha, tip, land.run_id)
            .map(|()| Forwarded::Landed);
    }
    let branch = pgit(project, &["symbolic-ref", "-q", "--short", "HEAD"]);
    if !branch.success || branch.stdout != land.target_branch {
        return Err("the checkout switched branches; nothing was merged".to_owned());
    }
    let status = pgit(project, &["status", "--porcelain"]);
    if !status.success {
        return Err(format!("git status failed: {}", status.last_error));
    }
    if !status.stdout.is_empty() {
        return Err(format!(
            "the checkout has uncommitted changes; the merge is kept in {label}"
        ));
    }
    let head_of = || {
        rev_of(&pgit(
            project,
            &["rev-parse", "--verify", "--quiet", "HEAD^{commit}"],
        ))
    };
    if head_of().as_deref() != Some(tip) {
        return Ok(Forwarded::Moved);
    }

    // C-17: the user's own git, hooks included.
    let args: Vec<OsString> = [
        "-C",
        &project.to_string_lossy(),
        "merge",
        "--ff-only",
        "--quiet",
        merge_sha,
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
        Some((out, _)) if out.status.success() => Ok(Forwarded::Landed),
        Some((_, line)) => {
            // I-14: a failed fast-forward after which HEAD moved counts as a move.
            if head_of().as_deref() == Some(tip) {
                Err(format!(
                    "git merge --ff-only failed: {}",
                    push::redact_urls(&line)
                ))
            } else {
                Ok(Forwarded::Moved)
            }
        }
        None => Err("git merge did not finish".to_owned()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::os::unix::fs::symlink;

    #[test]
    fn a_resolved_file_passes_and_a_leftover_marker_fails() {
        let dir = tempfile::TempDir::new().unwrap();
        let work = dir.path();
        fs::write(work.join("ok.txt"), "merged\nTitle\n=======\n").unwrap();
        fs::write(
            work.join("bad.txt"),
            "a\n<<<<<<< HEAD\nx\n=======\ny\n>>>>>>> b\n",
        )
        .unwrap();
        let names = |list: &[&str]| list.iter().map(|s| (*s).to_owned()).collect::<Vec<_>>();
        assert_eq!(first_unresolved(work, &names(&["ok.txt"])), None);
        let why = first_unresolved(work, &names(&["ok.txt", "bad.txt"])).unwrap();
        assert!(
            why.contains("bad.txt") && why.contains("conflict markers"),
            "{why}"
        );
        // A deleted file passes.
        assert_eq!(first_unresolved(work, &names(&["gone.txt"])), None);
    }

    #[test]
    fn a_symlink_a_directory_and_an_escape_fail() {
        let dir = tempfile::TempDir::new().unwrap();
        let work = dir.path().join("w");
        fs::create_dir(&work).unwrap();
        let outside = dir.path().join("outside.txt");
        fs::write(&outside, "x").unwrap();
        symlink(&outside, work.join("link.txt")).unwrap();
        fs::create_dir(work.join("sub")).unwrap();
        let one = |name: &str| first_unresolved(&work, &[name.to_owned()]).unwrap();
        assert!(one("link.txt").contains("not a regular file"));
        assert!(one("sub").contains("not a regular file"));
        assert!(one("../outside.txt").contains("not a path inside"));
        assert!(one("/etc/passwd").contains("not a path inside"));
        // A parent that is a symlink out of the clone is refused too.
        symlink(dir.path(), work.join("up")).unwrap();
        assert!(one("up/outside.txt").contains("not a path inside"));
    }

    #[test]
    fn a_file_over_the_cap_is_too_large_to_check() {
        let dir = tempfile::TempDir::new().unwrap();
        let big = fs::File::create(dir.path().join("big.txt")).unwrap();
        big.set_len(MARKER_READ_CAP + 1).unwrap();
        let why = first_unresolved(dir.path(), &["big.txt".to_owned()]).unwrap();
        assert!(why.contains("too large to check"), "{why}");
    }

    #[test]
    fn attempt_dirs_are_numbered_and_never_reused() {
        let run = tempfile::TempDir::new().unwrap();
        let (one, first) = claim_attempt(run.path()).unwrap();
        let (two, second) = claim_attempt(run.path()).unwrap();
        assert_eq!((one, two), (1, 2));
        assert!(first.is_dir() && second.is_dir());
        fs::remove_dir(&first).unwrap();
        // A removed number is still skipped only if it exists; the next free one is used.
        let (next, _) = claim_attempt(run.path()).unwrap();
        assert_eq!(next, 1, "the lowest free number, never an existing one");
    }
}
