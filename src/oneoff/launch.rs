//! Launch, supervise and stop one one-off run (C-01, C-02, C-06..C-09, C-11, C-13,
//! C-14, C-16, C-17).
//!
//! The order mirrors the driver launcher's (`launcher::run`'s module header):
//! validate; the credential, the host probe and the posture (a refusal writes
//! nothing); the run dir, claimed by an atomic mkdir (any number of runs per project,
//! quick 261103-hx7); the per-run lock `<run-dir>/run.lock`, held for the run's life;
//! the signal handlers;
//! `run.json` as `preparing`; the private clone `<run-dir>/work` (a `--no-hardlinks`
//! clone on branch `gsd-oneoff/<run-id>` at the project's HEAD, recorded as `base`);
//! the planted-file baseline of the clone; the push mirror (pinned before any
//! sandbox); `report/`; the binary snapshot; the `$HOME` allowlist; the per-run config
//! dir; the prompt; the egress server and the sandbox argv; the spawn through the
//! launcher's process seam, whose redacting pump writes `stream.jsonl`; the wait with
//! the host wall clock; then the host-side finalize (outcome, re-validated report,
//! commit range, SUMMARY path) and `run.json` as `finished`. The lock is dropped last.
//!
//! Every agent fact comes from [`AgentRuntime::default`]; this file names none.

use std::ffi::OsString;
use std::fs;
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, LazyLock};
use std::time::{Duration, Instant};

use chrono::Utc;
use regex::Regex;

use crate::driver::lock::{
    acquire_at_with_retry, probe_at, read_holder_at, LockProbe, ACQUIRE_RETRY_BUDGET,
};
use crate::driverctl::serve::{hash8, install_signal_handlers, socket_path};
use crate::error::LockError;
use crate::launcher::config_dir::{app_root_masks, driver_root, resolve_current_exe, snapshot_exe};
use crate::launcher::egress::socket_path_for;
use crate::launcher::home::HomeSetup;
use crate::launcher::host::{detect, Posture};
use crate::launcher::process::{
    respawn_detached, run_bounded, spawn_launch_child_piped, CAPTURE_FINISH_BUDGET,
};
use crate::launcher::push::{self, planted};
use crate::launcher::run::{
    log_tail, runtime_max_sec, Containment, EgressGuard, DETACH_BUDGET, DETACH_POLL,
};
use crate::launcher::sandbox::{
    bridged_command, bwrap_argv_masking, child_env, systemd_run_argv, LaunchPaths, RunDirMode,
    SandboxSpec, BRIDGE_PORT, MAX_PROMPT_BYTES,
};
use crate::launcher::stop::signal_target;
use crate::launcher::LauncherError;

use super::cancel::{
    pause_grace_from_env, read_request, request_now, signal_scope, stop_grace_from_env,
    write_request, Cancel, CancelStep, CutEnd, StopBy, StopKind, StopRecord, StopRequest,
};
use super::inbox::{ensure_inbox, pause_outcome_since, InjectionKind, InjectionOutcome};
use super::land::{git_relevant_planted, landing_in_progress, track_branch};
use super::merge::MIN_RESOLVER_BUDGET_USD;
use super::report::{read_validated, ReportStatus};
use super::resolve::{merge_master_into_run, ResumeMergeJob};
use super::resume::{ResumeFrom, ResumeSpec};
use super::runtime::{AgentRuntime, HostEnd};
use super::session::{grace_from_env, idle_settle_from_env, Supervisor};
use super::store::{
    live_lock_path, mint_run_id, oneoff_root, project_dir_for, project_dir_in, resolve_run,
    rfc3339, run_lock_path, summary_path_from, synthesize, valid_oneoff_run_id, write_record,
    RecordStatus, ReportSource, RunPick, RunRecord, SandboxRecord, MAX_CHANGED_FILES,
};
use super::stream::spend_of_file;
use super::todos::{valid_todo_file_name, PENDING_REL};
use super::{
    prompt, Mode, TaskSpec, BRANCH_PREFIX, LAND_DIR, LAUNCH_LOG, REPORT_DIR_ENV, REPORT_FILE,
    REPORT_SUBDIR, STREAM_FILE, WORK_DIR,
};

/// The first word of every one-off scope unit name (I-16).
pub const SCOPE_PREFIX: &str = "gsdmm-oneoff";

/// How often the wait loop looks at the child.
const WAIT_POLL: Duration = Duration::from_millis(100);

/// The clone of a large repository may take a while.
const CLONE_BUDGET: Duration = Duration::from_secs(600);

/// Range reads in the clone.
const READ_BUDGET: Duration = Duration::from_secs(30);

/// How long `stop` waits for a holder record that is being written.
const HOLDER_WAIT: Duration = Duration::from_secs(2);

static MODEL_RE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^[A-Za-z0-9][A-Za-z0-9._\[\]-]{0,63}$").expect("static regex"));

/// What a one-off run is asked to do.
#[derive(Debug, Clone, PartialEq)]
pub struct OneoffOptions {
    /// The project root (canonicalized again here).
    pub project: PathBuf,
    pub alias: String,
    pub mode: Mode,
    pub task: TaskSpec,
    pub model: String,
    pub max_budget_usd: f64,
    pub cap_minutes: f64,
    /// `--unsandboxed` (CLI only, C-09).
    pub unsandboxed: bool,
    /// The id the detached parent minted; `None` mints one here.
    pub run_id: Option<String>,
    /// The project's `driver.egress` CONNECT allowlist (I-8).
    pub egress_allow: Vec<String>,
    /// Set when this run continues a paused, stopped or killed one (quick 261103-klf,
    /// C-09). Its task, mode, model and wall clock are the old run's; the budget is the
    /// caller's.
    pub resume: Option<ResumeSpec>,
}

/// A detached run that holds its lock.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Started {
    pub run_id: String,
    pub pid: u32,
    pub log: PathBuf,
}

/// `gsdmm-oneoff-<run-id>-<hash8>` (I-16). systemd appends `.scope`.
pub fn scope_unit(run_id: &str, hash8: &str) -> String {
    format!("{SCOPE_PREFIX}-{run_id}-{hash8}")
}

/// `RuntimeMaxSec` for a `cap_minutes` wall clock: the seconds rounded up plus the
/// launcher's grace (I-16).
pub fn runtime_max_sec_for(cap_minutes: f64) -> u64 {
    runtime_max_sec(cap_minutes * 60.0)
}

fn io_error(context: &str, e: impl std::fmt::Display) -> LauncherError {
    LauncherError::Io {
        detail: format!("{context}: {e}"),
    }
}

/// Judge the options before anything is probed or written.
fn validate(opts: &OneoffOptions) -> Result<(), LauncherError> {
    let invalid = |detail: String| LauncherError::InvalidRun { detail };
    if let Some(id) = &opts.run_id {
        if !valid_oneoff_run_id(id) {
            return Err(invalid(format!("run id {id:?} is not a one-off run id")));
        }
    }
    if !MODEL_RE.is_match(&opts.model) {
        return Err(invalid(format!(
            "model {:?} is not a model name",
            opts.model
        )));
    }
    let bounded = |v: f64, max: f64| v.is_finite() && v > 0.0 && v <= max;
    if !bounded(opts.max_budget_usd, crate::cli::MAX_BUDGET_USD) {
        return Err(invalid("the budget is out of bounds".to_owned()));
    }
    if !bounded(opts.cap_minutes, crate::cli::MAX_CAP_MINUTES) {
        return Err(invalid("the wall clock is out of bounds".to_owned()));
    }
    match &opts.task {
        TaskSpec::Text(text) => prompt::check_task_text(text).map_err(invalid)?,
        TaskSpec::Todo { file_name, .. } => {
            if !valid_todo_file_name(file_name) {
                return Err(invalid(format!("{file_name:?} is not a todo file name")));
            }
        }
    }
    Ok(())
}

/// The canonical project, and its one-off project dir created 0700.
fn project_dirs(project: &Path) -> Result<(PathBuf, PathBuf), LauncherError> {
    let project = fs::canonicalize(project)
        .map_err(|e| io_error(&format!("project {}", project.display()), e))?;
    let root = oneoff_root().ok_or_else(|| LauncherError::Io {
        detail: "no per-user data directory is available for one-off runs".to_owned(),
    })?;
    let project_dir = project_dir_in(&root, &project);
    create_private_dir(&project_dir)?;
    Ok((project, project_dir))
}

fn create_private_dir(dir: &Path) -> Result<(), LauncherError> {
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(dir)
        .map_err(|e| io_error(&format!("create {}", dir.display()), e))
}

/// Claim a fresh run id and dir for `project` (a resume claims it under the old run's
/// land lock, quick 261103-klf, I-18); the run then starts with `run_id: Some(id)`.
pub(crate) fn claim_new_run(project: &Path) -> Result<(String, PathBuf), LauncherError> {
    let (project, project_dir) = project_dirs(project)?;
    let _ = project;
    claim_run_dir(&project_dir)
}

/// Claim a fresh run directory under `project_dir` with an atomic, non-recursive
/// `mkdir` (quick 261103-hx7, I-01): the id minted now, `-2`, `-3`... when the name is
/// taken. `AlreadyExists` is never treated as success, so two launches at the same
/// second get distinct ids and distinct directories.
fn claim_run_dir(project_dir: &Path) -> Result<(String, PathBuf), LauncherError> {
    claim_run_dir_at(project_dir, &mint_run_id(Utc::now()))
}

/// [`claim_run_dir`] for the base id `base`.
fn claim_run_dir_at(project_dir: &Path, base: &str) -> Result<(String, PathBuf), LauncherError> {
    for n in 1..10_000_u32 {
        let id = if n == 1 {
            base.to_owned()
        } else {
            format!("{base}-{n}")
        };
        let dir = project_dir.join(&id);
        match fs::DirBuilder::new().mode(0o700).create(&dir) {
            Ok(()) => return Ok((id, dir)),
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(io_error(&format!("create {}", dir.display()), e)),
        }
    }
    Err(io_error(
        "claim a run directory",
        "too many runs started in the same second",
    ))
}

/// The refusal for a held per-run lock.
fn locked(holder: &str) -> LauncherError {
    LauncherError::Locked {
        detail: format!("{holder} is already live"),
    }
}

/// Run one one-off in this process and block until it ends.
pub fn run_foreground(opts: OneoffOptions) -> Result<RunRecord, LauncherError> {
    validate(&opts)?;
    let runtime = AgentRuntime::default();
    let credential = runtime.credential(Utc::now())?;
    let caps = detect();
    let posture = runtime.decide_posture(&caps, opts.unsandboxed)?;
    let home = dirs::home_dir().ok_or_else(|| LauncherError::Io {
        detail: "the home directory is unknown".to_owned(),
    })?;
    let (project, project_dir) = project_dirs(&opts.project)?;
    // I-01: the daemon child opens the dir its parent claimed; a foreground run claims
    // its own.
    let (run_id, run_dir) = match &opts.run_id {
        Some(id) => {
            let dir = project_dir.join(id);
            create_private_dir(&dir)?;
            (id.clone(), dir)
        }
        None => claim_run_dir(&project_dir)?,
    };

    // C-01: the per-run lock first, before `run.json` exists. Nothing below writes
    // before this.
    let lock = acquire_at_with_retry(
        &run_lock_path(&run_dir),
        &run_id,
        std::process::id(),
        ACQUIRE_RETRY_BUDGET,
    )
    .map_err(|e| match e {
        LockError::Unavailable { detail } => LauncherError::Io {
            detail: format!("one-off lock: {detail}"),
        },
        LockError::HeldBy { run_id, .. } => locked(&run_id),
        LockError::HeldByUnknownRun => locked(&run_id),
    })?;
    let user_stop = Arc::new(AtomicBool::new(false));
    install_signal_handlers(&user_stop)?;

    let mut record = RunRecord::new(
        &run_id,
        &opts.alias,
        project.clone(),
        opts.mode,
        opts.task.clone(),
        runtime.name(),
        &opts.model,
        opts.max_budget_usd,
        opts.cap_minutes,
        Utc::now(),
    );
    record.sandbox = Some(SandboxRecord {
        posture: posture.as_str().to_owned(),
        missing: posture.missing().to_vec(),
    });
    write_record(&run_dir, &record).map_err(|e| io_error("write run.json", e))?;

    let ctx = Ctx {
        opts: &opts,
        runtime,
        credential: &credential,
        posture: &posture,
        overlay: caps.overlay,
        home: &home,
        project: &project,
        run_dir: &run_dir,
        run_id: &run_id,
        user_stop: &user_stop,
    };
    let result = run_session(&ctx, &mut record);
    if let Err(e) = &result {
        let outcome = super::AgentOutcome::Failed {
            detail: format!("launcher: {e}"),
        };
        record.report = Some(synthesize(&outcome, opts.max_budget_usd, opts.cap_minutes));
        record.report_source = Some(ReportSource::Synthesized {
            reason: "the launcher failed before the session ended".to_owned(),
        });
        record.outcome = Some(outcome);
        record.status = RecordStatus::Finished;
        record.ended_at = Some(rfc3339(Utc::now()));
        let _ = write_record(&run_dir, &record);
    }
    // Lock last: the record is final before the lock goes free.
    drop(lock);
    result.map(|()| record)
}

/// The facts one session runs with.
struct Ctx<'a> {
    opts: &'a OneoffOptions,
    runtime: AgentRuntime,
    credential: &'a crate::launcher::auth::LaunchToken,
    posture: &'a Posture,
    overlay: bool,
    home: &'a Path,
    project: &'a Path,
    run_dir: &'a Path,
    run_id: &'a str,
    user_stop: &'a AtomicBool,
}

/// One hardened `git` in the clone, with the host's global and system config off.
fn git_read(work: &Path, args: &[&str]) -> push::GitRun {
    let mut full = vec![OsString::from("-C"), work.as_os_str().to_owned()];
    full.extend(args.iter().map(OsString::from));
    push::git_mirror_only(full, READ_BUDGET)
}

/// Clone, sandbox, spawn, wait and finalize. Errors are recorded by the caller.
fn run_session(ctx: &Ctx<'_>, record: &mut RunRecord) -> Result<(), LauncherError> {
    let opts = ctx.opts;
    let run_dir = ctx.run_dir;
    let step = |what: &str| {
        let what = what.to_owned();
        move |e: String| LauncherError::Io {
            detail: format!("{what}: {e}"),
        }
    };

    // klf C-09: a resume continues from the old run's recorded head.
    let old = match &opts.resume {
        Some(spec) => Some(read_old_run(ctx.project, spec)?),
        None => None,
    };
    // C-07: the clone, cut at the project's HEAD (a resume: at the old run's head).
    let base = match &old {
        Some(old) => {
            // o2r D-C: a resume lands where the run it continues was dispatched from.
            record.dispatch_branch = old.record.dispatch_branch.clone();
            record.dispatch_tip = old.record.dispatch_tip.clone();
            record.dispatch_detached = old.record.dispatch_detached;
            old.record
                .head
                .clone()
                .ok_or_else(|| LauncherError::InvalidRun {
                    detail: "the run to resume has no recorded head".to_owned(),
                })?
        }
        None => {
            let (branch, sha) = push::current_branch_and_sha(ctx.project).ok_or_else(|| {
                LauncherError::InvalidRun {
                    detail: "the project has no commit to start from".to_owned(),
                }
            })?;
            // o2r D-C: the branch HEAD pointed at and the commit the clone starts from,
            // taken from the one read that yields `base`.
            record.dispatch_detached = branch.is_none();
            record.dispatch_branch = branch;
            record.dispatch_tip = Some(sha.clone());
            sha
        }
    };
    record.base = Some(base.clone());
    record.resumed_from = old.as_ref().map(|o| o.record.run_id.clone());
    let work = run_dir.join(WORK_DIR);
    let clone = push::git(
        vec![
            "clone".into(),
            "--no-hardlinks".into(),
            "--quiet".into(),
            ctx.project.as_os_str().to_owned(),
            work.as_os_str().to_owned(),
        ],
        CLONE_BUDGET,
    );
    if !clone.success {
        return Err(step("clone the project")(clone.last_error));
    }
    let branch = format!("{BRANCH_PREFIX}{}", ctx.run_id);
    let switch = git_read(&work, &["switch", "--quiet", "-c", &branch, &base]);
    if !switch.success {
        return Err(step("create the work branch")(switch.last_error));
    }

    // I-3: a todo must be committed, so the clone has it. A resume skips the check: the
    // old run passed it and may have moved the todo (klf, I-19).
    if let (None, TaskSpec::Todo { file_name, .. }) = (&old, &opts.task) {
        let todo = work.join(PENDING_REL).join(file_name);
        if !fs::symlink_metadata(&todo).is_ok_and(|m| m.file_type().is_file()) {
            return Err(LauncherError::InvalidRun {
                detail: format!(
                    "{PENDING_REL}/{file_name} is not in the project's last commit; commit the todo first"
                ),
            });
        }
    }

    let report_dir = run_dir.join(REPORT_SUBDIR);
    create_private_dir(&report_dir)?;

    // klf C-11, I-20: `--from master` merges the checkout's tip into the old head here,
    // in the new run's own clone, before the agent session.
    if let (Some(old), Some(spec)) = (&old, &opts.resume) {
        if spec.start == ResumeFrom::Master {
            let ended = resume_master(ctx, record, old, &work, &report_dir)?;
            if ended {
                return write_record(run_dir, record).map_err(|e| io_error("write run.json", e));
            }
        }
    }

    // D-22 reused: the planted-file baseline of the clone, before any sandbox exists.
    if let Err(e) = planted::save_baseline_if_absent(run_dir, &planted::snapshot(&work)) {
        record.warnings.push(format!(
            "no planted-file baseline ({e}); landing runs in strict mode"
        ));
    }
    // C-20: the push mirror and its pinned URL, before any sandbox exists.
    if let Some(root) = driver_root() {
        if let Err(e) = push::prepare_mirror(&root, ctx.project) {
            record.warnings.push(format!("no push mirror: {e}"));
        }
    }
    // klf I-20: what the resume merge cost comes off the budget the agent session gets.
    let budget_usd = opts.max_budget_usd - record.spend_usd.unwrap_or(0.0);
    if budget_usd < MIN_RESOLVER_BUDGET_USD {
        return Err(LauncherError::InvalidRun {
            detail: "no budget left after the resume merge".to_owned(),
        });
    }
    let prompt = match &old {
        Some(old) => resume_prompt(ctx, record, old, &work),
        None => prompt::build(opts.mode, &opts.task, ctx.run_id),
    }
    .map_err(|detail| LauncherError::InvalidRun { detail })?;
    let spec = SessionSpec {
        runtime: ctx.runtime,
        session_dir: run_dir,
        work: &work,
        report_dir: &report_dir,
        unit_id: ctx.run_id,
        prompt: &prompt,
        model: &opts.model,
        budget_usd,
        cap_minutes: opts.cap_minutes,
        egress_allow: &opts.egress_allow,
        posture: ctx.posture,
        credential: ctx.credential,
        overlay: ctx.overlay,
        home: ctx.home,
        project: ctx.project,
        user_stop: ctx.user_stop,
        inbox: true,
        control: Some(run_dir),
        pause_message: Some(ctx.runtime.pause_message()),
    };
    let mut on_event = |event: SessionEvent<'_>| match event {
        SessionEvent::ScopeUnit(unit) => record.scope_unit = unit.map(str::to_owned),
        SessionEvent::Running => {
            record.status = RecordStatus::Running;
            if let Err(e) = write_record(run_dir, record) {
                eprintln!("task run: could not record the running state: {e}");
            }
        }
        SessionEvent::SoftCap { usd, hit_at } => {
            record.soft_cap_usd = usd;
            record.soft_cap_hit_at = hit_at.map(str::to_owned);
            if let Err(e) = write_record(run_dir, record) {
                eprintln!("task run: could not record the soft cap: {e}");
            }
        }
        SessionEvent::Stop(stop) => {
            record.stop = Some(stop.clone());
            if let Err(e) = write_record(run_dir, record) {
                eprintln!("task run: could not record the stop request: {e}");
            }
        }
    };
    let session = run_sandboxed_session(&spec, &mut on_event)?;
    record.warnings.extend(session.warnings);
    record.soft_cap_usd = session.soft_cap_usd;
    record.soft_cap_hit_at = session.soft_cap_hit_at;
    // I-17: the session's estimate is recorded, so a later resolver knows what is left.
    // klf I-24: a resume merge's resolver spend is already on the record; add to it.
    record.spend_usd = Some(record.spend_usd.unwrap_or(0.0) + session.spend_usd);
    if session.stop.is_some() {
        record.stop = session.stop;
    }

    // mnr D-02: the agent's report decides whether a delivered pause ended `paused`.
    let report_status = read_validated(&report_dir.join(REPORT_FILE))
        .ok()
        .map(|r| r.status);
    finalize(
        ctx,
        record,
        &work,
        &run_dir.join(STREAM_FILE),
        &report_dir,
        session.rc,
        pause_end_with_report(session.end, report_status),
    );
    write_record(run_dir, record).map_err(|e| io_error("write run.json", e))
}

/// A delivered pause ends the run `Paused` only when the agent answered it with a
/// `partial` report; a `done` report means it finished anyway, and a missing, `blocked`
/// or `failed` one falls back to the ordinary classification. Every other end passes
/// through unchanged (mnr, D-02).
fn pause_end_with_report(end: HostEnd, report: Option<ReportStatus>) -> HostEnd {
    match (end, report) {
        (HostEnd::Paused, Some(ReportStatus::Partial)) => HostEnd::Paused,
        (HostEnd::Paused, _) => HostEnd::Exited,
        (other, _) => other,
    }
}

/// The run a resume continues, read from disk.
fn read_old_run(
    project: &Path,
    spec: &ResumeSpec,
) -> Result<super::store::OneoffView, LauncherError> {
    let dir = project_dir_for(project).ok_or_else(|| LauncherError::Io {
        detail: "no one-off directory for this project".to_owned(),
    })?;
    super::store::read_view(&dir, &spec.from_run).ok_or_else(|| LauncherError::InvalidRun {
        detail: format!("no one-off run {} to resume", spec.from_run),
    })
}

/// The stop the user asked of the run before its agent session exists (klf, I-20): the
/// run's request file, or a signal.
fn pending_stop(ctx: &Ctx<'_>) -> Option<StopRequest> {
    let from_file = read_request(ctx.run_dir).ok().flatten();
    if ctx.user_stop.load(Ordering::SeqCst) {
        return Some(match from_file {
            Some(file) if file.kind != StopKind::Pause => file,
            Some(file) => StopRequest {
                kind: StopKind::Hard,
                ..file
            },
            None => request_now(StopKind::Hard, StopBy::Signal),
        });
    }
    from_file
}

/// The `--from master` merge of a resume, in the new run's clone. Returns `true` when
/// the user stopped the run during it and the run is already finalized (no agent
/// session follows); `Err` when the merge failed (the run then ends `failed`).
fn resume_master(
    ctx: &Ctx<'_>,
    record: &mut RunRecord,
    old: &super::store::OneoffView,
    work: &Path,
    report_dir: &Path,
) -> Result<bool, LauncherError> {
    let invalid = |detail: String| LauncherError::InvalidRun { detail };
    let old_head = old.record.head.clone().unwrap_or_default();
    let (branch, tip) = match push::current_branch_and_sha(ctx.project) {
        Some((Some(branch), tip)) => (branch, tip),
        _ => {
            return Err(invalid(
                "the checkout's HEAD is detached; check out a branch to merge first".to_owned(),
            ))
        }
    };
    let job = ResumeMergeJob {
        project: ctx.project,
        run_dir: ctx.run_dir,
        run_id: ctx.run_id,
        old_run_id: &old.record.run_id,
        work,
        old_head: &old_head,
        branch: &branch,
        tip: &tip,
        egress_allow: &ctx.opts.egress_allow,
        user_stop: ctx.user_stop,
    };
    let merged = merge_master_into_run(&job, record);
    // A stop asked during the merge ends the run with that request's outcome.
    if let Some(request) = pending_stop(ctx) {
        let mut stop = StopRecord::begun(&request);
        stop.ended_at = Some(rfc3339(Utc::now()));
        record.stop = Some(stop);
        match &merged {
            Ok(Some(m)) | Err(m) => record.resume_merge = Some(m.clone()),
            Ok(None) => {}
        }
        let end = match request.kind {
            StopKind::Hard => HostEnd::UserStop,
            StopKind::Kill => HostEnd::Killed { escalated: false },
            StopKind::Pause => HostEnd::Paused,
        };
        finalize(
            ctx,
            record,
            work,
            &ctx.run_dir.join(STREAM_FILE),
            report_dir,
            None,
            end,
        );
        return Ok(true);
    }
    match merged {
        Ok(None) => {
            record.warnings.push(format!(
                "the checkout's tip on {} is already in the run's head; nothing was merged",
                clean(&branch, 100)
            ));
            Ok(false)
        }
        Ok(Some(merge)) => {
            if let Some(sha) = &merge.merge_sha {
                record.base = Some(sha.clone());
            }
            record.resume_merge = Some(merge);
            if let Err(e) = write_record(ctx.run_dir, record) {
                eprintln!("task run: could not record the resume merge: {e}");
            }
            Ok(false)
        }
        Err(failure) => {
            let why = failure.failure.clone().unwrap_or_default();
            record.resume_merge = Some(failure);
            Err(invalid(format!(
                "the merge of {} into the run's branch failed before the session: {why}",
                clean(&branch, 100)
            )))
        }
    }
}

/// The prompt of a resumed run (klf, C-10, I-21).
fn resume_prompt(
    ctx: &Ctx<'_>,
    record: &RunRecord,
    old: &super::store::OneoffView,
    work: &Path,
) -> Result<String, String> {
    let old_record = &old.record;
    let commit_lines: Vec<String> = match (&old_record.base, &old_record.head) {
        (Some(base), Some(head)) => {
            let log = git_read(work, &["log", "--format=%h %s", &format!("{base}..{head}")]);
            if log.success {
                log.stdout.lines().take(50).map(str::to_owned).collect()
            } else {
                Vec::new()
            }
        }
        _ => Vec::new(),
    };
    let followups = old_record
        .report
        .as_ref()
        .map(|r| r.followups.clone())
        .unwrap_or_default();
    let master_note = record.resume_merge.as_ref().and_then(|m| {
        let sha = m.merge_sha.as_deref()?;
        let how = match m.how {
            Some(super::store::MergeHow::Resolved) => {
                format!("conflicts in {} were resolved", m.conflicted.join(", "))
            }
            _ => "it merged cleanly".to_owned(),
        };
        Some(format!(
            "The checkout's branch {} was merged into your branch first ({how}; merge commit {}).",
            m.branch,
            &sha[..sha.len().min(12)]
        ))
    });
    prompt::build_resume(&prompt::ResumePrompt {
        mode: ctx.opts.mode,
        task: &ctx.opts.task,
        run_id: ctx.run_id,
        old_run_id: &old_record.run_id,
        outcome_word: old_record.outcome.as_ref().map_or("ended", |o| o.word()),
        headline: old_record.report.as_ref().map(|r| r.headline.as_str()),
        followups: &followups,
        commit_lines: &commit_lines,
        master_note: master_note.as_deref(),
    })
}

/// The facts one sandboxed agent session runs with: a run's own session and a conflict
/// resolver's share this one description, so they cannot drift apart (quick 261103-hx7,
/// D-05, "the same launcher sandbox and the same `AgentRuntime` seam").
pub(crate) struct SessionSpec<'a> {
    pub runtime: AgentRuntime,
    /// Holds `stream.jsonl`, `launch.log`, the per-run config dir, the gitconfig and the
    /// egress log: the run dir, or a landing attempt's dir.
    pub session_dir: &'a Path,
    /// The clone the agent works in; the sandbox's project bind.
    pub work: &'a Path,
    /// The directory the agent may write its report in.
    pub report_dir: &'a Path,
    /// The scope unit's and the sockets' name (`<run-id>`, or `<run-id>-land<n>`).
    pub unit_id: &'a str,
    pub prompt: &'a str,
    pub model: &'a str,
    pub budget_usd: f64,
    pub cap_minutes: f64,
    pub egress_allow: &'a [String],
    pub posture: &'a Posture,
    pub credential: &'a crate::launcher::auth::LaunchToken,
    pub overlay: bool,
    pub home: &'a Path,
    /// The user's project, whose hash names the scope unit.
    pub project: &'a Path,
    pub user_stop: &'a AtomicBool,
    /// Create the operator's inbox (a run has one; a resolver does not).
    pub inbox: bool,
    /// The directory a stop request file is read from (quick 261103-klf, I-09): the
    /// run dir for a run, `<run-dir>/land/` for a resolver.
    pub control: Option<&'a Path>,
    /// The message injected when the user pauses (a run has one; a resolver is not
    /// asked to pause, its attempt just finishes within the grace).
    pub pause_message: Option<&'a str>,
}

/// What the session tells its caller while it runs.
#[derive(Debug, Clone, Copy)]
pub(crate) enum SessionEvent<'a> {
    /// The scope unit the session runs in, before the spawn.
    ScopeUnit(Option<&'a str>),
    /// The agent was spawned.
    Running,
    /// The soft cap or its hit time changed.
    SoftCap {
        usd: Option<f64>,
        hit_at: Option<&'a str>,
    },
    /// A stop request was first seen, or came in stronger (klf, I-13).
    Stop(&'a StopRecord),
}

/// How a sandboxed session ended.
#[derive(Debug)]
pub(crate) struct SessionRun {
    pub rc: Option<i32>,
    pub end: HostEnd,
    pub warnings: Vec<String>,
    pub soft_cap_usd: Option<f64>,
    pub soft_cap_hit_at: Option<String>,
    /// The session's spend in notional dollars, read from the whole stream after the
    /// capture pump drained (the supervisor's last tick can miss the final lines).
    pub spend_usd: f64,
    /// How the user stopped, killed or paused the session, completed (klf, C-07).
    pub stop: Option<StopRecord>,
}

/// Run one agent session: config dir, snapshot, `$HOME` allowlist, seed, egress, the
/// sandbox argv, env, `stream.jsonl`, the spawn, the supervised wait (stop, wall clock,
/// soft cap), then the egress and capture finish. Errors are the caller's to record.
pub(crate) fn run_sandboxed_session(
    spec: &SessionSpec<'_>,
    on_event: &mut dyn FnMut(SessionEvent<'_>),
) -> Result<SessionRun, LauncherError> {
    let session_dir = spec.session_dir;
    let work = spec.work;

    let config_dir = spec.runtime.config_dir_in(session_dir);
    let binary = resolve_current_exe()
        .and_then(|exe| snapshot_exe(&exe, spec.project, &config_dir))
        .map_err(|e| io_error("locate or snapshot this binary", e))?;
    let home_setup = HomeSetup::for_host(spec.home, &binary, spec.overlay, session_dir)
        .map_err(|e| io_error("derive the sandbox's $HOME allowlist", e))?;
    let sandboxed = matches!(spec.posture, Posture::BwrapScope);
    spec.runtime
        .seed_config(
            &config_dir,
            spec.home,
            &binary,
            spec.report_dir,
            sandboxed,
            &home_setup.keep_roots,
        )
        .map_err(|e| io_error("seed the per-run config directory", e))?;

    if spec.prompt.len() > MAX_PROMPT_BYTES {
        return Err(LauncherError::InvalidRun {
            detail: format!(
                "the prompt is {} bytes; at most {MAX_PROMPT_BYTES} fit in one message",
                spec.prompt.len()
            ),
        });
    }
    let agent_argv = spec.runtime.argv(spec.model, spec.budget_usd);
    if spec.inbox {
        ensure_inbox(session_dir).map_err(|e| io_error("create the inbox", e))?;
    }

    let mut egress_guard = None;
    let mut unit = None;
    let argv = if sandboxed {
        let sock = socket_path(spec.unit_id, session_dir)?;
        let egress_sock = socket_path_for(&sock, session_dir);
        egress_guard = Some(EgressGuard::start_with(
            &egress_sock,
            spec.runtime.egress_token(spec.credential, spec.home),
            spec.egress_allow.to_vec(),
            spec.runtime.upstream(),
            session_dir.join("egress.log"),
        )?);
        let sandbox = SandboxSpec::from_host(
            LaunchPaths {
                project: work.to_path_buf(),
                run_dir: spec.report_dir.to_path_buf(),
                config_dir: config_dir.clone(),
                socket: sock,
                egress_socket: egress_sock.clone(),
            },
            spec.home,
            &home_setup,
        );
        let name = scope_unit(spec.unit_id, &hash8(spec.project));
        let bridged = bridged_command(&binary, BRIDGE_PORT, &egress_sock, &agent_argv);
        let argv = systemd_run_argv(
            &name,
            Some(runtime_max_sec_for(spec.cap_minutes)),
            &bwrap_argv_masking(
                &sandbox,
                RunDirMode::ReportWritable,
                &app_root_masks(spec.home),
                &bridged,
            ),
        );
        unit = Some(format!("{name}.scope"));
        argv
    } else {
        agent_argv
    };
    on_event(SessionEvent::ScopeUnit(unit.as_deref()));

    let mut path = config_dir.join("bin").into_os_string();
    if let Some(host_path) = std::env::var_os("PATH") {
        path.push(":");
        path.push(host_path);
    }
    let mut sets = spec
        .runtime
        .env_sets(&config_dir, spec.posture, spec.credential);
    sets.push(("PATH", path));
    sets.push((REPORT_DIR_ENV, spec.report_dir.as_os_str().to_owned()));
    let env = child_env(std::env::vars_os(), &sets);
    drop(sets);

    // C-11: the raw stream, appended through the redacting, line-bounded pump.
    let stream_path = session_dir.join(STREAM_FILE);
    let out = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(&stream_path)
        .map_err(|e| io_error(&format!("open {}", stream_path.display()), e))?;
    let (mut child, capture, input) =
        spawn_launch_child_piped(&argv, &env, work, out).map_err(|e| LauncherError::Spawn {
            detail: format!("the agent session: {e}"),
        })?;
    drop(env);
    // C-05: the prompt is the first message on the agent's stdin.
    // D-01: the agent's report file says it is done; the idle settle covers one that
    // never writes it.
    let mut supervisor = Supervisor::new(
        session_dir,
        &spec.report_dir.join(REPORT_FILE),
        input,
        spec.budget_usd,
        grace_from_env(),
        idle_settle_from_env(),
    );
    supervisor.send_prompt(spec.prompt, Instant::now());
    let containment = match &unit {
        Some(unit) => Containment::Scope { unit: unit.clone() },
        None => Containment::Group { pgid: child.id() },
    };
    on_event(SessionEvent::Running);

    // I-17: the wall clock starts at the spawn.
    let started = Instant::now();
    let cap = Duration::from_secs_f64(spec.cap_minutes * 60.0);
    let mut cancel = Cancel::new(stop_grace_from_env(), pause_grace_from_env());
    // klf: the strongest stop the user asked for, and what `run.json` keeps of it.
    let mut stop: Option<StopRequest> = None;
    let mut stop_record: Option<StopRecord> = None;
    // mnr D-02: a pause only counts once the supervisor DELIVERED the message.
    let mut pause_delivered = false;
    let (status, end) = loop {
        // klf I-09, I-13: a request file, or a signal with or without one.
        let mut wanted = None;
        if let Some(dir) = spec.control {
            wanted = read_request(dir).ok().flatten();
        }
        if spec.user_stop.load(Ordering::SeqCst) {
            wanted = Some(match wanted {
                Some(file) if file.kind != StopKind::Pause => file,
                Some(file) => StopRequest {
                    kind: StopKind::Hard,
                    ..file
                },
                None => request_now(StopKind::Hard, StopBy::Signal),
            });
        }
        if let Some(request) = wanted {
            if stop.as_ref().is_none_or(|s| request.kind.outranks(s.kind)) {
                let record = StopRecord::begun(&request);
                on_event(SessionEvent::Stop(&record));
                stop_record = Some(record);
                stop = Some(request);
            }
        }
        match cancel.tick(stop.as_ref().map(|s| s.kind), Instant::now()) {
            CancelStep::Wait => {}
            CancelStep::InjectPause => {
                if let Some(message) = spec.pause_message {
                    match supervisor.inject_host(InjectionKind::Pause, message, Instant::now()) {
                        Ok(()) => pause_delivered = true,
                        Err(reason) => {
                            // Withdrawn: no grace cut follows a pause that never
                            // reached the agent, and the record says why.
                            cancel.withdraw_pause();
                            if let Some(record) = stop_record.as_mut() {
                                record.refused = Some(reason);
                                record.ended_at = Some(rfc3339(Utc::now()));
                                on_event(SessionEvent::Stop(record));
                            }
                        }
                    }
                }
            }
            CancelStep::Term => send_signal(&containment, "TERM"),
            CancelStep::Kill => {
                send_signal(&containment, "KILL");
                let _ = child.kill();
                supervisor.close_input();
                break (child.wait(), end_of_cut(&cancel));
            }
        }
        if started.elapsed() >= cap {
            containment.stop(&mut child);
            supervisor.close_input();
            break (child.wait(), HostEnd::WallClock);
        }
        let tick = supervisor.tick(Instant::now());
        if tick.persist {
            on_event(SessionEvent::SoftCap {
                usd: supervisor.soft_cap_usd(),
                hit_at: supervisor.soft_cap_hit_at(),
            });
        }
        if tick.stop_soft_cap {
            containment.stop(&mut child);
            supervisor.close_input();
            break (child.wait(), HostEnd::SoftCap);
        }
        match child.try_wait() {
            Ok(Some(status)) => {
                let end = if cancel.cutting() {
                    end_of_cut(&cancel)
                } else {
                    // The agent ended by itself: a pause the request file still carries
                    // and a clean exit is a paused run (C-06, I-13). A resolver is not
                    // asked to pause, so it never ends `Paused`.
                    let late = spec
                        .control
                        .and_then(|dir| read_request(dir).ok().flatten());
                    if let Some(late) = &late {
                        if stop_record.is_none() {
                            stop_record = Some(StopRecord::begun(late));
                        }
                    }
                    match (late.map(|l| l.kind), spec.pause_message.is_some()) {
                        (Some(StopKind::Pause), true) if status.success() && pause_delivered => {
                            HostEnd::Paused
                        }
                        (Some(StopKind::Hard), _) => HostEnd::UserStop,
                        (Some(StopKind::Kill), _) => HostEnd::Killed { escalated: false },
                        _ => HostEnd::Exited,
                    }
                };
                break (Ok(status), end);
            }
            Ok(None) => {}
            Err(e) => {
                containment.stop(&mut child);
                supervisor.close_input();
                return Err(io_error("wait for the agent session", e));
            }
        }
        std::thread::sleep(WAIT_POLL);
    };
    if !matches!(end, HostEnd::WallClock | HostEnd::SoftCap) {
        containment.stop_leftovers();
    }
    let stop = stop_record.map(|mut record| {
        record.ended_at.get_or_insert_with(|| rfc3339(Utc::now()));
        record.escalated = cancel.escalated();
        record
    });
    if let Some(guard) = egress_guard {
        guard.finish();
    }
    let end_of_session = supervisor.finish();
    capture.finish(CAPTURE_FINISH_BUDGET);
    let rc = status
        .ok()
        .and_then(|s| s.code().or_else(|| s.signal().map(|signal| 128 + signal)));
    let spend_usd = spend_of_file(&stream_path).max(end_of_session.spend_usd);

    Ok(SessionRun {
        rc,
        end,
        warnings: end_of_session.warnings,
        soft_cap_usd: end_of_session.soft_cap_usd,
        soft_cap_hit_at: end_of_session.soft_cap_hit_at,
        spend_usd,
        stop,
    })
}

/// The `HostEnd` of a session the daemon cut (klf, C-05, I-13).
fn end_of_cut(cancel: &Cancel) -> HostEnd {
    match cancel.end() {
        Some(CutEnd::Terminated) | None => HostEnd::UserStop,
        Some(CutEnd::Killed { escalated }) => HostEnd::Killed { escalated },
        Some(CutEnd::PauseCut) => HostEnd::Killed { escalated: true },
    }
}

/// Send `signal` to everything the session runs in: the scope by name, or the process
/// group (klf, I-12). The caller reaps the child.
fn send_signal(containment: &Containment, signal: &str) {
    match containment {
        Containment::Scope { unit } => signal_scope(unit, signal),
        Containment::Group { pgid } => {
            let sig = if signal == "KILL" {
                rustix::process::Signal::KILL
            } else {
                rustix::process::Signal::TERM
            };
            let _ = crate::driver::kill::signal_group(*pgid, sig);
        }
    }
}

/// `text` with control characters replaced and cut to `max` characters.
fn clean(text: &str, max: usize) -> String {
    text.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .take(max)
        .collect()
}

/// The host-side end of a run (C-13, C-14): the outcome, the re-validated report or
/// a synthesized one, and the commit range read from the clone with hardened git.
fn finalize(
    ctx: &Ctx<'_>,
    record: &mut RunRecord,
    work: &Path,
    stream_path: &Path,
    report_dir: &Path,
    rc: Option<i32>,
    end: HostEnd,
) {
    let opts = ctx.opts;
    let outcome = ctx.runtime.outcome(stream_path, rc, end);
    match read_validated(&report_dir.join(REPORT_FILE)) {
        Ok(report) => {
            record.report = Some(report);
            record.report_source = Some(ReportSource::Agent);
        }
        Err(why) => {
            let reason = if report_dir.join(REPORT_FILE).symlink_metadata().is_err() {
                "the agent wrote no report".to_owned()
            } else {
                format!("the agent's report was refused: {}", clean(&why, 200))
            };
            record.report = Some(synthesize(&outcome, opts.max_budget_usd, opts.cap_minutes));
            record.report_source = Some(ReportSource::Synthesized { reason });
        }
    }
    record.outcome = Some(outcome);
    record.exit_code = rc;

    // The planted-file gate, the work facts and the tracked branch (shared with crash
    // recovery, quick 261103-o2r, D-B).
    settle_work(
        ctx.project,
        ctx.run_dir,
        work,
        ctx.run_id,
        opts.mode,
        record,
    );
    record.status = RecordStatus::Finished;
    record.ended_at = Some(rfc3339(Utc::now()));
}

/// Derive the work facts of a run from its clone and track its branch in the project:
/// the planted-file gate (file reads only, before any git touches the clone), then the
/// tip, the commit count, the changed files and the SUMMARY path read with hardened git,
/// then `track_branch` for a run with commits and a clean gate. A tracking failure is a
/// `tracking_note`, never an error. Used by `finalize` and by crash recovery (quick
/// 261103-o2r, D-B); the caller sets `status` and `ended_at`.
pub(crate) fn settle_work(
    project: &Path,
    run_dir: &Path,
    work: &Path,
    run_id: &str,
    mode: Mode,
    record: &mut RunRecord,
) {
    // C-05: the planted-file gate reads files only, so it runs before any git touches
    // the clone (the tracking fetch starts `upload-pack` inside the agent's repository).
    let planted_findings = git_relevant_planted(run_dir, work).len();
    // I-9: only the tip of the work branch is recorded and landed.
    let branch_ref = format!("refs/heads/{BRANCH_PREFIX}{}", run_id);
    let head = git_read(
        work,
        &[
            "rev-parse",
            "--verify",
            "--quiet",
            &format!("{branch_ref}^{{commit}}"),
        ],
    );
    if head.success && push::is_hex_sha(&head.stdout) {
        let head = head.stdout;
        if let Some(base) = record.base.clone() {
            let range = format!("{base}..{head}");
            let count = git_read(work, &["rev-list", "--count", &range]);
            record.commits = count.stdout.trim().parse().ok().filter(|_| count.success);
            let diff = git_read(
                work,
                &["diff", "--name-status", "--no-renames", "-z", &base, &head],
            );
            let mut changes: Vec<(char, String)> = Vec::new();
            if diff.success {
                let mut fields = diff.stdout.split('\0').filter(|f| !f.is_empty());
                while let (Some(status), Some(path)) = (fields.next(), fields.next()) {
                    let letter = status.chars().next().unwrap_or('?');
                    changes.push((letter, path.to_owned()));
                }
            }
            record.summary_path = match mode {
                Mode::Quick => summary_path_from(&changes),
                Mode::Fast => None,
            };
            if changes.len() > MAX_CHANGED_FILES {
                record.warnings.push(format!(
                    "{} files changed; only the first {MAX_CHANGED_FILES} are recorded",
                    changes.len()
                ));
            }
            record.changed_files = changes
                .into_iter()
                .take(MAX_CHANGED_FILES)
                .map(|(_, path)| clean(&path, 512))
                .collect();
        }
        let current = git_read(work, &["rev-parse", "--verify", "--quiet", "HEAD^{commit}"]);
        if current.success && current.stdout != head {
            record.warnings.push(format!(
                "the clone's HEAD is not the tip of {BRANCH_PREFIX}{}; only the branch tip is recorded",
                run_id
            ));
        }
        record.head = Some(head.clone());
        // C-05, C-06: the branch is tracked in the project only for a run with commits
        // and a clean planted gate. A tracking failure is never a run failure.
        if planted_findings > 0 {
            record.tracking_note = Some(format!(
                "{planted_findings} git-relevant planted finding(s) in the clone; not tracked"
            ));
        } else if record.commits.unwrap_or(0) == 0 {
            record.tracking_note = Some("the run made no commits; no branch tracked".to_owned());
        } else {
            match track_branch(project, work, run_id, &head) {
                Ok(reference) => record.tracked_branch = Some(reference),
                Err(note) => record.tracking_note = Some(clean(&note, 400)),
            }
        }
    } else {
        record.tracking_note =
            Some("no head: the work branch is gone; no branch tracked".to_owned());
        record.warnings.push(format!(
            "the work branch {branch_ref} is gone; nothing can be landed"
        ));
    }
}

/// The argv the detached parent re-executes this binary with.
pub fn daemon_argv(opts: &OneoffOptions, run_id: &str, config: Option<&Path>) -> Vec<OsString> {
    let mut argv: Vec<OsString> = Vec::new();
    if let Some(config) = config {
        argv.push("--config".into());
        argv.push(config.as_os_str().to_owned());
    }
    argv.extend(["task", "run"].map(OsString::from));
    argv.push(opts.project.as_os_str().to_owned());
    argv.extend(["--run-id", run_id, "--mode", opts.mode.as_str()].map(OsString::from));
    match (&opts.resume, &opts.task) {
        // klf I-24: a resume carries the old run's id instead of its task; the daemon
        // reads the task from the old record.
        (Some(resume), _) => {
            argv.push("--resume-from".into());
            argv.push(resume.from_run.clone().into());
            argv.push("--resume-start".into());
            argv.push(resume.start.word().into());
        }
        (None, TaskSpec::Todo { file_name, .. }) => {
            argv.push("--todo".into());
            argv.push(file_name.into());
        }
        (None, TaskSpec::Text(text)) => {
            argv.push("--text".into());
            argv.push(text.into());
        }
    }
    argv.push("--model".into());
    argv.push(opts.model.clone().into());
    argv.push("--max-budget-usd".into());
    argv.push(opts.max_budget_usd.to_string().into());
    argv.push("--cap-minutes".into());
    argv.push(opts.cap_minutes.to_string().into());
    if opts.unsandboxed {
        argv.push("--unsandboxed".into());
    }
    argv.push("--foreground".into());
    argv.push("--daemon-child".into());
    argv
}

/// Start a one-off as a daemon in its own session and return once it holds the run's
/// own lock (the driver's `launch_detached` pattern).
pub fn run_detached(opts: &OneoffOptions, config: Option<&Path>) -> Result<Started, LauncherError> {
    validate(opts)?;
    let runtime = AgentRuntime::default();
    // Refusals in the caller's terminal, before any directory or log exists. The
    // daemon decides again for itself.
    runtime.credential(Utc::now())?;
    runtime.decide_posture(&detect(), opts.unsandboxed)?;
    let (project, project_dir) = project_dirs(&opts.project)?;
    // C-03: no per-project lock and no cap; every launch claims its own run dir.
    let config = config
        .map(std::path::absolute)
        .transpose()
        .map_err(|e| io_error("resolve --config", e))?;
    let (run_id, run_dir) = match &opts.run_id {
        Some(id) => {
            let dir = project_dir.join(id);
            create_private_dir(&dir)?;
            (id.clone(), dir)
        }
        None => claim_run_dir(&project_dir)?,
    };
    let lock_file = run_lock_path(&run_dir);
    let log_path = run_dir.join(LAUNCH_LOG);
    let log = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(&log_path)
        .map_err(|e| io_error(&format!("open {}", log_path.display()), e))?;
    let absolute = OneoffOptions {
        project,
        ..opts.clone()
    };
    let argv = daemon_argv(&absolute, &run_id, config.as_deref());
    let mut child = respawn_detached(&argv, log).map_err(|e| LauncherError::Spawn {
        detail: format!("the detached one-off: {e}"),
    })?;

    let started = |child: std::process::Child| {
        let pid = child.id();
        // Reap the daemon whenever it ends, so a long-lived caller (the TUI) keeps no
        // zombie.
        let mut child = child;
        std::thread::spawn(move || {
            let _ = child.wait();
        });
        Ok(Started {
            run_id: run_id.clone(),
            pid,
            log: log_path.clone(),
        })
    };
    let deadline = Instant::now() + DETACH_BUDGET;
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return started(child),
            Ok(Some(status)) => {
                let detail = log_tail(&log_path, 0);
                // A daemon that refused before writing anything leaves only its log.
                if !run_dir.join(super::RUN_FILE).exists() {
                    let _ = fs::remove_dir_all(&run_dir);
                }
                return Err(LauncherError::DaemonExited {
                    code: status.code().unwrap_or(1),
                    detail,
                });
            }
            Ok(None) => {}
            Err(e) => return Err(io_error("wait for the detached one-off", e)),
        }
        if probe_at(&lock_file) == LockProbe::Held
            && read_holder_at(&lock_file)
                .is_some_and(|h| h.pgid == child.id() && h.run_id == run_id)
        {
            return started(child);
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(LauncherError::Io {
                detail: format!(
                    "the detached one-off did not take the lock within {}s:\n{}",
                    DETACH_BUDGET.as_secs(),
                    log_tail(&log_path, 0)
                ),
            });
        }
        std::thread::sleep(DETACH_POLL);
    }
}

/// What `stop` did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StopOutcome {
    /// The daemon was asked and the lock went free.
    Stopped { run_id: String, pid: u32 },
    /// The daemon was signalled but still holds the lock.
    StillRunning { run_id: String, pid: u32 },
    /// A pause was asked, the launcher delivered it to the agent (its
    /// `injections.jsonl` says so) and the run is still winding down; the launcher cuts
    /// it after the pause grace (quick 261103-klf, I-14; mnr D-02).
    PauseDelivered { run_id: String, pid: u32 },
    /// The launcher refused the pause (for example the session was already ending after
    /// the agent reported). It is withdrawn: nothing cuts the run for it. `ended` says
    /// the run's lock was free by the time we looked (mnr, D-02).
    PauseRefused {
        run_id: String,
        reason: String,
        ended: bool,
    },
    /// The pause was requested but the launcher's answer did not appear within the wait.
    PauseUnconfirmed { run_id: String, pid: u32 },
    /// No daemon; the recorded scope was stopped by name.
    OrphanScopeStopped { unit: String },
    /// Nothing is running.
    NotRunning,
    /// The run's landing (its conflict resolver) was asked to `kind` and has ended
    /// (klf, C-13).
    LandingStopped { run_id: String, kind: StopKind },
    /// The landing was asked to `kind` and is still running; it acts on the request
    /// itself.
    LandingRequested { run_id: String, kind: StopKind },
}

impl StopOutcome {
    /// Whether the outcome means a requested pause did NOT happen as asked (refused, or
    /// not confirmed): a failure for the CLI exit code and the TUI status line.
    pub fn is_refusal(&self) -> bool {
        matches!(
            self,
            StopOutcome::PauseRefused { .. } | StopOutcome::PauseUnconfirmed { .. }
        )
    }

    /// The outcome as one line.
    pub fn message(&self) -> String {
        match self {
            StopOutcome::Stopped { run_id, .. } => format!("{run_id} stopped"),
            StopOutcome::StillRunning { run_id, pid } => {
                format!("{run_id} (pid {pid}) was signalled but is still running")
            }
            StopOutcome::PauseDelivered { run_id, .. } => format!(
                "{run_id}: the pause was delivered; the agent is winding down (the launcher cuts \
                 it after its pause grace)"
            ),
            StopOutcome::PauseRefused {
                run_id,
                reason,
                ended: false,
            } => format!(
                "the pause could not be delivered to {run_id} ({reason}); it keeps running - use \
                 'task stop' (without --pause) to stop it"
            ),
            StopOutcome::PauseRefused {
                run_id,
                reason,
                ended: true,
            } => format!(
                "the pause could not be delivered to {run_id} ({reason}); the run has ended on \
                 its own"
            ),
            StopOutcome::PauseUnconfirmed { run_id, .. } => format!(
                "the pause of {run_id} was requested but its delivery is not confirmed yet; \
                 check 'task status' (the launcher may still deliver it)"
            ),
            StopOutcome::OrphanScopeStopped { unit } => {
                format!("no live one-off launcher; stopped the leftover scope {unit}")
            }
            StopOutcome::NotRunning => "that one-off is not running".to_owned(),
            StopOutcome::LandingStopped { run_id, kind } => {
                format!(
                    "the landing of {run_id} ended after a {} request; nothing landed",
                    kind.word()
                )
            }
            StopOutcome::LandingRequested { run_id, kind } => format!(
                "the landing of {run_id} was asked to {} and is still ending; nothing will land",
                kind.word()
            ),
        }
    }
}

/// Whether `unit` is a one-off scope this launcher could have created.
fn plausible_scope(unit: &str) -> bool {
    unit.starts_with(&format!("{SCOPE_PREFIX}-"))
        && unit.ends_with(".scope")
        && unit
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_.".contains(c))
}

/// Whom a stop is aimed at (klf, C-02, I-10).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StopTarget {
    /// The run's agent session.
    Run,
    /// The run's in-progress landing (its conflict resolver): `--resolver`.
    Landing,
    /// The agent when the run is live; otherwise the landing when its lock is held.
    Auto,
}

/// Stop, kill or pause a one-off of `project`, waiting up to `wait` for the targeted
/// lock to go free.
///
/// `run_id` names the run; without it the run is picked like every other verb does
/// ([`resolve_run`] with `refuse_several_live`): the only live run, or a refusal that
/// lists the live ids when several may be live (quick 261103-hx7, I-03).
///
/// A request is written only while the target's lock is held (`run.lock` for the run,
/// `land.lock` for a landing); the run's daemon is signalled (TERM, for hard and kill)
/// only through its run lock's holder, and a landing's lock holder is NEVER signalled:
/// it may be this very process (the TUI), so the landing polls its request file
/// instead (klf, I-10, T-klf-04).
pub fn stop(
    project: &Path,
    run_id: Option<&str>,
    kind: StopKind,
    by: StopBy,
    target: StopTarget,
    wait: Duration,
) -> Result<StopOutcome, LauncherError> {
    let project_dir = project_dir_for(project).ok_or_else(|| LauncherError::Io {
        detail: "no one-off directory for this project".to_owned(),
    })?;
    let view = match resolve_run(&project_dir, run_id, true) {
        Ok(view) => view,
        Err(RunPick::NoRuns) => return Ok(StopOutcome::NotRunning),
        Err(pick) => {
            return Err(LauncherError::InvalidRun {
                detail: pick.detail(),
            })
        }
    };
    let lock_file = live_lock_path(&project_dir, &view.run_dir);
    let probe = probe_at(&lock_file);
    let landing = match target {
        StopTarget::Run => false,
        StopTarget::Landing => true,
        StopTarget::Auto => probe != LockProbe::Held && landing_in_progress(&view.run_dir),
    };
    if landing {
        return stop_landing(&view, kind, by, wait);
    }
    match probe {
        LockProbe::Held => {
            let deadline = Instant::now() + HOLDER_WAIT;
            let holder = loop {
                if let Some(holder) = read_holder_at(&lock_file) {
                    break holder;
                }
                if Instant::now() >= deadline {
                    return Err(LauncherError::Io {
                        detail: "the one-off lock is held but its holder record is unreadable"
                            .to_owned(),
                    });
                }
                std::thread::sleep(Duration::from_millis(50));
            };
            // A legacy project lock may be held by another run of the old kind; only
            // the selected run's own holder is signalled.
            if holder.run_id != view.record.run_id {
                return Ok(StopOutcome::NotRunning);
            }
            let target = signal_target(holder.pgid)?;
            // Request and signal only while the lock is still held, so the pid is the
            // daemon's.
            let mut written = None;
            if probe_at(&lock_file) == LockProbe::Held {
                written = Some(
                    write_request(&view.run_dir, request_now(kind, by))
                        .map_err(|e| io_error("could not write the stop request", e))?,
                );
                if kind != StopKind::Pause {
                    match rustix::process::kill_process(target, rustix::process::Signal::TERM) {
                        Ok(()) | Err(rustix::io::Errno::SRCH) => {}
                        Err(errno) => {
                            return Err(io_error(
                                &format!("could not signal pid {}", holder.pgid),
                                errno,
                            ))
                        }
                    }
                }
            }
            let deadline = Instant::now() + wait;
            // mnr D-02: a pause is confirmed by the launcher's own record of it, never
            // assumed. A merged request stronger than a pause waits like a hard stop.
            let pause_since = written
                .as_ref()
                .filter(|r| r.kind == StopKind::Pause)
                .map(|r| r.requested_at.clone());
            loop {
                let free = probe_at(&lock_file) != LockProbe::Held;
                if let Some(since) = &pause_since {
                    // The lock was probed first: a record the daemon wrote before it
                    // exited is already on disk when `free` is true.
                    match pause_outcome_since(&view.run_dir, since) {
                        Some(record) if record.outcome == InjectionOutcome::Delivered => {
                            return Ok(if free {
                                StopOutcome::Stopped {
                                    run_id: holder.run_id,
                                    pid: holder.pgid,
                                }
                            } else {
                                StopOutcome::PauseDelivered {
                                    run_id: holder.run_id,
                                    pid: holder.pgid,
                                }
                            });
                        }
                        Some(record) => {
                            return Ok(StopOutcome::PauseRefused {
                                run_id: holder.run_id,
                                reason: record
                                    .reason
                                    .unwrap_or_else(|| "the session is ending".to_owned()),
                                ended: free,
                            });
                        }
                        None if free => {
                            return Ok(StopOutcome::PauseRefused {
                                run_id: holder.run_id,
                                reason: "the run ended before the pause reached it".to_owned(),
                                ended: true,
                            });
                        }
                        None => {}
                    }
                } else if free {
                    return Ok(StopOutcome::Stopped {
                        run_id: holder.run_id,
                        pid: holder.pgid,
                    });
                }
                if Instant::now() >= deadline {
                    return Ok(if pause_since.is_some() {
                        StopOutcome::PauseUnconfirmed {
                            run_id: holder.run_id,
                            pid: holder.pgid,
                        }
                    } else {
                        StopOutcome::StillRunning {
                            run_id: holder.run_id,
                            pid: holder.pgid,
                        }
                    });
                }
                std::thread::sleep(WAIT_POLL);
            }
        }
        LockProbe::Free | LockProbe::Absent => {
            // The selected run's own record, not the latest's.
            let record = view.record;
            if record.status != RecordStatus::Finished
                && record.scope_unit.as_deref().is_some_and(plausible_scope)
            {
                if kind == StopKind::Pause {
                    return Err(LauncherError::InvalidRun {
                        detail: "no live launcher to pause it; use stop or kill".to_owned(),
                    });
                }
                let unit = record.scope_unit.unwrap_or_default();
                if kind == StopKind::Kill {
                    signal_scope(&unit, "KILL");
                }
                let _ = run_bounded(
                    "systemctl",
                    &["--user".into(), "stop".into(), unit.clone().into()],
                    None,
                    &[],
                    Duration::from_secs(60),
                );
                Ok(StopOutcome::OrphanScopeStopped { unit })
            } else {
                Ok(StopOutcome::NotRunning)
            }
        }
        LockProbe::Unknown(errno) => Err(io_error("could not probe the one-off lock", errno)),
    }
}

/// Ask the run's in-progress landing to stop, be killed or pause (klf, C-13, C-14).
fn stop_landing(
    view: &super::store::OneoffView,
    kind: StopKind,
    by: StopBy,
    wait: Duration,
) -> Result<StopOutcome, LauncherError> {
    let run_dir = &view.run_dir;
    let run_id = view.record.run_id.clone();
    if !landing_in_progress(run_dir) {
        return Err(LauncherError::InvalidRun {
            detail: "no landing of this run is in progress".to_owned(),
        });
    }
    write_request(&run_dir.join(LAND_DIR), request_now(kind, by))
        .map_err(|e| io_error("could not write the landing's stop request", e))?;
    // X on a resolver kills its scope directly (C-13). The unit name is rebuilt from the
    // run id, the attempt number and the project hash, and checked for its shape.
    if kind == StopKind::Kill {
        if let Some(n) = view.land_attempts.last() {
            let name = format!(
                "{}.scope",
                scope_unit(&format!("{run_id}-land{n}"), &hash8(&view.record.project))
            );
            if plausible_scope(&name) && name.contains(&format!("-land{n}-")) {
                signal_scope(&name, "KILL");
            }
        }
    }
    let deadline = Instant::now() + wait;
    loop {
        if !landing_in_progress(run_dir) {
            return Ok(StopOutcome::LandingStopped { run_id, kind });
        }
        if Instant::now() >= deadline {
            return Ok(StopOutcome::LandingRequested { run_id, kind });
        }
        std::thread::sleep(WAIT_POLL);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn opts(task: TaskSpec) -> OneoffOptions {
        OneoffOptions {
            project: PathBuf::from("/work/p"),
            alias: "p".to_owned(),
            mode: Mode::Quick,
            task,
            model: "opus".to_owned(),
            max_budget_usd: 10.0,
            cap_minutes: 60.0,
            unsandboxed: false,
            run_id: None,
            egress_allow: Vec::new(),
            resume: None,
        }
    }

    fn strings(argv: &[OsString]) -> Vec<String> {
        argv.iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn the_daemon_argv_carries_every_option_and_the_config_first() {
        let todo = opts(TaskSpec::Todo {
            file_name: "t.md".to_owned(),
            title: "T".to_owned(),
        });
        assert_eq!(
            strings(&daemon_argv(&todo, "oneoff-x", Some(Path::new("/c.json")))),
            [
                "--config",
                "/c.json",
                "task",
                "run",
                "/work/p",
                "--run-id",
                "oneoff-x",
                "--mode",
                "quick",
                "--todo",
                "t.md",
                "--model",
                "opus",
                "--max-budget-usd",
                "10",
                "--cap-minutes",
                "60",
                "--foreground",
                "--daemon-child"
            ]
        );
        let mut text = opts(TaskSpec::Text("fix x".to_owned()));
        text.unsandboxed = true;
        text.mode = Mode::Fast;
        let argv = strings(&daemon_argv(&text, "oneoff-x", None));
        assert_eq!(argv[0], "task");
        assert!(argv.windows(2).any(|w| w == ["--text", "fix x"]));
        assert!(argv.windows(2).any(|w| w == ["--mode", "fast"]));
        assert!(argv.contains(&"--unsandboxed".to_owned()));
    }

    #[test]
    fn claiming_a_run_dir_never_reuses_an_existing_one() {
        let root = tempfile::TempDir::new().unwrap();
        let base = "oneoff-20261103-100000";
        let (first, first_dir) = claim_run_dir_at(root.path(), base).unwrap();
        assert_eq!(first, base);
        assert!(first_dir.is_dir());
        let (second, second_dir) = claim_run_dir_at(root.path(), base).unwrap();
        assert_eq!(second, format!("{base}-2"));
        assert!(second_dir.is_dir());
        let (third, _) = claim_run_dir_at(root.path(), base).unwrap();
        assert_eq!(third, format!("{base}-3"));
        assert!(valid_oneoff_run_id(&third));
    }

    #[test]
    fn caps_and_unit_names() {
        assert_eq!(runtime_max_sec_for(60.0), 3900);
        assert_eq!(
            scope_unit("oneoff-x", "ab12cd34"),
            "gsdmm-oneoff-oneoff-x-ab12cd34"
        );
        assert!(plausible_scope("gsdmm-oneoff-oneoff-x-ab12cd34.scope"));
        assert!(!plausible_scope("gsdmm-driver-r1-ab12cd34-1.scope"));
        assert!(!plausible_scope("gsdmm-oneoff-x;rm.scope"));
    }

    #[test]
    fn validation_refuses_bad_options() {
        let mut bad = opts(TaskSpec::Text("x".to_owned()));
        bad.model = "--danger".to_owned();
        assert!(validate(&bad).is_err());
        let mut bad = opts(TaskSpec::Text("x".to_owned()));
        bad.run_id = Some("../x".to_owned());
        assert!(validate(&bad).is_err());
        let bad = opts(TaskSpec::Todo {
            file_name: "../x.md".to_owned(),
            title: "t".to_owned(),
        });
        assert!(validate(&bad).is_err());
        assert!(validate(&opts(TaskSpec::Text(String::new()))).is_err());
        assert!(validate(&opts(TaskSpec::Text("ok".to_owned()))).is_ok());
    }

    #[test]
    fn a_resume_hands_the_daemon_the_old_run_instead_of_its_task() {
        let mut resumed = opts(TaskSpec::Text("fix x".to_owned()));
        resumed.resume = Some(ResumeSpec {
            from_run: "oneoff-20261103-100000".to_owned(),
            start: ResumeFrom::Master,
        });
        let argv = strings(&daemon_argv(&resumed, "oneoff-new", None));
        assert!(argv
            .windows(2)
            .any(|w| w == ["--resume-from", "oneoff-20261103-100000"]));
        assert!(argv.windows(2).any(|w| w == ["--resume-start", "master"]));
        assert!(!argv.contains(&"--text".to_owned()));
        assert!(!argv.contains(&"--todo".to_owned()));
    }

    #[test]
    fn the_report_decides_whether_a_delivered_pause_is_paused() {
        use ReportStatus::{Blocked, Done, Failed, Partial};
        assert_eq!(
            pause_end_with_report(HostEnd::Paused, Some(Partial)),
            HostEnd::Paused
        );
        for report in [Some(Done), Some(Blocked), Some(Failed), None] {
            assert_eq!(
                pause_end_with_report(HostEnd::Paused, report),
                HostEnd::Exited,
                "{report:?}"
            );
        }
        for end in [
            HostEnd::UserStop,
            HostEnd::Killed { escalated: true },
            HostEnd::WallClock,
            HostEnd::SoftCap,
            HostEnd::Exited,
        ] {
            for report in [Some(Done), Some(Partial), None] {
                assert_eq!(pause_end_with_report(end, report), end);
            }
        }
    }

    #[test]
    fn pause_refusals_are_failures_with_their_reason() {
        let refused = StopOutcome::PauseRefused {
            run_id: "oneoff-x".to_owned(),
            reason: "the session is ending".to_owned(),
            ended: false,
        };
        assert!(refused.is_refusal());
        let message = refused.message();
        assert!(message.contains("oneoff-x"), "{message}");
        assert!(message.contains("the session is ending"), "{message}");
        assert!(
            message.contains("'task stop' (without --pause)"),
            "{message}"
        );
        let ended = StopOutcome::PauseRefused {
            run_id: "oneoff-x".to_owned(),
            reason: "the run ended before the pause reached it".to_owned(),
            ended: true,
        };
        assert!(ended.is_refusal());
        assert!(ended.message().contains("has ended on its own"));
        let unconfirmed = StopOutcome::PauseUnconfirmed {
            run_id: "oneoff-x".to_owned(),
            pid: 7,
        };
        assert!(unconfirmed.is_refusal());
        assert!(unconfirmed.message().contains("not confirmed"));
        let delivered = StopOutcome::PauseDelivered {
            run_id: "oneoff-x".to_owned(),
            pid: 7,
        };
        assert!(!delivered.is_refusal());
        assert!(delivered.message().contains("delivered"));
    }
}
