//! Launch, supervise and stop one one-off run (C-01, C-02, C-06..C-09, C-11, C-13,
//! C-14, C-16, C-17).
//!
//! The order mirrors the driver launcher's (`launcher::run`'s module header):
//! validate; the credential, the host probe and the posture (a refusal writes
//! nothing); the per-project lock, held for the run's life; the signal handlers;
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
use crate::launcher::config_dir::{driver_root, resolve_current_exe, snapshot_exe};
use crate::launcher::egress::socket_path_for;
use crate::launcher::home::HomeSetup;
use crate::launcher::host::{detect, Posture};
use crate::launcher::process::{
    respawn_detached, run_bounded, spawn_launch_child, CAPTURE_FINISH_BUDGET,
};
use crate::launcher::push::{self, planted};
use crate::launcher::run::{
    log_tail, runtime_max_sec, Containment, EgressGuard, DETACH_BUDGET, DETACH_POLL,
};
use crate::launcher::sandbox::{
    bridged_command, bwrap_argv_with, child_env, systemd_run_argv, LaunchPaths, RunDirMode,
    SandboxSpec, BRIDGE_PORT, MAX_PROMPT_BYTES,
};
use crate::launcher::stop::signal_target;
use crate::launcher::LauncherError;

use super::report::read_validated;
use super::runtime::{AgentRuntime, HostEnd};
use super::store::{
    lock_path, mint_run_id, oneoff_root, probe, project_dir_for, project_dir_in, rfc3339,
    summary_path_from, synthesize, valid_oneoff_run_id, write_record, LockSeen, RecordStatus,
    ReportSource, RunRecord, SandboxRecord, MAX_CHANGED_FILES,
};
use super::todos::{valid_todo_file_name, PENDING_REL};
use super::{
    prompt, Mode, TaskSpec, BRANCH_PREFIX, LAUNCH_LOG, REPORT_DIR_ENV, REPORT_FILE, REPORT_SUBDIR,
    STREAM_FILE, WORK_DIR,
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

/// A fresh run id under `project_dir`: minted now, `-2`, `-3`... on a collision.
fn unique_run_id(project_dir: &Path) -> String {
    let base = mint_run_id(Utc::now());
    if !project_dir.join(&base).exists() {
        return base;
    }
    (2..10_000)
        .map(|n| format!("{base}-{n}"))
        .find(|id| !project_dir.join(id).exists())
        .unwrap_or(base)
}

/// The refusal for a held lock.
fn locked(holder: &str) -> LauncherError {
    LauncherError::Locked {
        detail: format!("a one-off is already running on this project: {holder}"),
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
    let run_id = opts
        .run_id
        .clone()
        .unwrap_or_else(|| unique_run_id(&project_dir));

    // C-17: the per-project lock first. Nothing below writes before this.
    let lock_file = lock_path(&project_dir);
    let lock = acquire_at_with_retry(
        &lock_file,
        &run_id,
        std::process::id(),
        ACQUIRE_RETRY_BUDGET,
    )
    .map_err(|e| match e {
        LockError::Unavailable { detail } => LauncherError::Io {
            detail: format!("one-off lock: {detail}"),
        },
        LockError::HeldBy { run_id, .. } => locked(&run_id),
        LockError::HeldByUnknownRun => locked("an unknown run"),
    })?;
    let user_stop = Arc::new(AtomicBool::new(false));
    install_signal_handlers(&user_stop)?;

    let run_dir = project_dir.join(&run_id);
    create_private_dir(&run_dir)?;
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

    // C-07: the clone, cut at the project's HEAD.
    let (_, base) =
        push::current_branch_and_sha(ctx.project).ok_or_else(|| LauncherError::InvalidRun {
            detail: "the project has no commit to start from".to_owned(),
        })?;
    record.base = Some(base.clone());
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

    // I-3: a todo must be committed, so the clone has it.
    if let TaskSpec::Todo { file_name, .. } = &opts.task {
        let todo = work.join(PENDING_REL).join(file_name);
        if !fs::symlink_metadata(&todo).is_ok_and(|m| m.file_type().is_file()) {
            return Err(LauncherError::InvalidRun {
                detail: format!(
                    "{PENDING_REL}/{file_name} is not in the project's last commit; commit the todo first"
                ),
            });
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
    let report_dir = run_dir.join(REPORT_SUBDIR);
    create_private_dir(&report_dir)?;

    let config_dir = ctx.runtime.config_dir_in(run_dir);
    let binary = resolve_current_exe()
        .and_then(|exe| snapshot_exe(&exe, ctx.project, &config_dir))
        .map_err(|e| io_error("locate or snapshot this binary", e))?;
    let home_setup = HomeSetup::for_host(ctx.home, &binary, ctx.overlay, run_dir)
        .map_err(|e| io_error("derive the sandbox's $HOME allowlist", e))?;
    let sandboxed = matches!(ctx.posture, Posture::BwrapScope);
    ctx.runtime
        .seed_config(
            &config_dir,
            ctx.home,
            &binary,
            &report_dir,
            sandboxed,
            &home_setup.keep_roots,
        )
        .map_err(|e| io_error("seed the per-run config directory", e))?;

    let prompt = prompt::build(opts.mode, &opts.task, ctx.run_id)
        .map_err(|detail| LauncherError::InvalidRun { detail })?;
    if prompt.len() > MAX_PROMPT_BYTES {
        return Err(LauncherError::InvalidRun {
            detail: format!(
                "the prompt is {} bytes; at most {MAX_PROMPT_BYTES} fit in one argument",
                prompt.len()
            ),
        });
    }
    let agent_argv = ctx.runtime.argv(&prompt, &opts.model, opts.max_budget_usd);

    let mut egress_guard = None;
    let mut unit = None;
    let argv = if sandboxed {
        let sock = socket_path(ctx.run_id, run_dir)?;
        let egress_sock = socket_path_for(&sock, run_dir);
        egress_guard = Some(EgressGuard::start_with(
            &egress_sock,
            ctx.runtime.egress_token(ctx.credential, ctx.home),
            opts.egress_allow.clone(),
            ctx.runtime.upstream(),
            run_dir.join("egress.log"),
        )?);
        let spec = SandboxSpec::from_host(
            LaunchPaths {
                project: work.clone(),
                run_dir: report_dir.clone(),
                config_dir: config_dir.clone(),
                socket: sock,
                egress_socket: egress_sock.clone(),
            },
            ctx.home,
            &home_setup,
        );
        let name = scope_unit(ctx.run_id, &hash8(ctx.project));
        let bridged = bridged_command(&binary, BRIDGE_PORT, &egress_sock, &agent_argv);
        let argv = systemd_run_argv(
            &name,
            Some(runtime_max_sec_for(opts.cap_minutes)),
            &bwrap_argv_with(&spec, RunDirMode::ReportWritable, &bridged),
        );
        unit = Some(format!("{name}.scope"));
        argv
    } else {
        agent_argv
    };
    record.scope_unit.clone_from(&unit);

    let mut path = config_dir.join("bin").into_os_string();
    if let Some(host_path) = std::env::var_os("PATH") {
        path.push(":");
        path.push(host_path);
    }
    let mut sets = ctx
        .runtime
        .env_sets(&config_dir, ctx.posture, ctx.credential);
    sets.push(("PATH", path));
    sets.push((REPORT_DIR_ENV, report_dir.clone().into_os_string()));
    let env = child_env(std::env::vars_os(), &sets);
    drop(sets);

    // C-11: the raw stream, appended through the redacting, line-bounded pump.
    let stream_path = run_dir.join(STREAM_FILE);
    let out = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(&stream_path)
        .map_err(|e| io_error(&format!("open {}", stream_path.display()), e))?;
    let (mut child, capture) =
        spawn_launch_child(&argv, &env, &work, out).map_err(|e| LauncherError::Spawn {
            detail: format!("the agent session: {e}"),
        })?;
    drop(env);
    let containment = match &unit {
        Some(unit) => Containment::Scope { unit: unit.clone() },
        None => Containment::Group { pgid: child.id() },
    };
    record.status = RecordStatus::Running;
    if let Err(e) = write_record(run_dir, record) {
        eprintln!("task run: could not record the running state: {e}");
    }

    // I-17: the wall clock starts at the spawn.
    let started = Instant::now();
    let cap = Duration::from_secs_f64(opts.cap_minutes * 60.0);
    let (status, end) = loop {
        if ctx.user_stop.load(Ordering::SeqCst) {
            containment.stop(&mut child);
            break (child.wait(), HostEnd::UserStop);
        }
        if started.elapsed() >= cap {
            containment.stop(&mut child);
            break (child.wait(), HostEnd::WallClock);
        }
        match child.try_wait() {
            Ok(Some(status)) => break (Ok(status), HostEnd::Exited),
            Ok(None) => {}
            Err(e) => {
                containment.stop(&mut child);
                return Err(io_error("wait for the agent session", e));
            }
        }
        std::thread::sleep(WAIT_POLL);
    };
    if end == HostEnd::Exited {
        containment.stop_leftovers();
    }
    if let Some(guard) = egress_guard {
        guard.finish();
    }
    capture.finish(CAPTURE_FINISH_BUDGET);
    let rc = status
        .ok()
        .and_then(|s| s.code().or_else(|| s.signal().map(|signal| 128 + signal)));

    finalize(ctx, record, &work, &stream_path, &report_dir, rc, end);
    write_record(run_dir, record).map_err(|e| io_error("write run.json", e))
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

    // I-9: only the tip of the work branch is recorded and landed.
    let branch_ref = format!("refs/heads/{BRANCH_PREFIX}{}", ctx.run_id);
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
            record.summary_path = match opts.mode {
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
                ctx.run_id
            ));
        }
        record.head = Some(head);
    } else {
        record.warnings.push(format!(
            "the work branch {branch_ref} is gone; nothing can be landed"
        ));
    }
    record.status = RecordStatus::Finished;
    record.ended_at = Some(rfc3339(Utc::now()));
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
    match &opts.task {
        TaskSpec::Todo { file_name, .. } => {
            argv.push("--todo".into());
            argv.push(file_name.into());
        }
        TaskSpec::Text(text) => {
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

/// Start a one-off as a daemon in its own session and return once it holds the
/// project's lock (the driver's `launch_detached` pattern).
pub fn run_detached(opts: &OneoffOptions, config: Option<&Path>) -> Result<Started, LauncherError> {
    validate(opts)?;
    let runtime = AgentRuntime::default();
    // Refusals in the caller's terminal, before any directory or log exists. The
    // daemon decides again for itself.
    runtime.credential(Utc::now())?;
    runtime.decide_posture(&detect(), opts.unsandboxed)?;
    let (project, project_dir) = project_dirs(&opts.project)?;
    let lock_file = lock_path(&project_dir);
    match probe(&lock_file) {
        LockSeen::HeldBy(id) => return Err(locked(&id)),
        LockSeen::Unknown => {
            return Err(LauncherError::Io {
                detail: "the one-off lock could not be probed".to_owned(),
            })
        }
        LockSeen::Free => {}
    }
    let config = config
        .map(std::path::absolute)
        .transpose()
        .map_err(|e| io_error("resolve --config", e))?;
    let run_id = match &opts.run_id {
        Some(id) => id.clone(),
        None => unique_run_id(&project_dir),
    };
    let run_dir = project_dir.join(&run_id);
    create_private_dir(&run_dir)?;
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
    /// The daemon was signalled and the lock went free.
    Stopped { run_id: String, pid: u32 },
    /// The daemon was signalled but still holds the lock.
    StillRunning { run_id: String, pid: u32 },
    /// No daemon; the recorded scope was stopped by name.
    OrphanScopeStopped { unit: String },
    /// Nothing is running.
    NotRunning,
}

impl StopOutcome {
    /// The outcome as one line.
    pub fn message(&self) -> String {
        match self {
            StopOutcome::Stopped { run_id, .. } => format!("{run_id} stopped"),
            StopOutcome::StillRunning { run_id, pid } => {
                format!("{run_id} (pid {pid}) was signalled but is still running")
            }
            StopOutcome::OrphanScopeStopped { unit } => {
                format!("no live one-off launcher; stopped the leftover scope {unit}")
            }
            StopOutcome::NotRunning => "no one-off is running on this project".to_owned(),
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

/// Stop the live one-off of `project`, waiting up to `wait` for its lock to go free.
pub fn stop(project: &Path, wait: Duration) -> Result<StopOutcome, LauncherError> {
    let project_dir = project_dir_for(project).ok_or_else(|| LauncherError::Io {
        detail: "no one-off directory for this project".to_owned(),
    })?;
    let lock_file = lock_path(&project_dir);
    match probe_at(&lock_file) {
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
            let target = signal_target(holder.pgid)?;
            // Signal only while the lock is still held, so the pid is the daemon's.
            if probe_at(&lock_file) == LockProbe::Held {
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
            let deadline = Instant::now() + wait;
            loop {
                if probe_at(&lock_file) != LockProbe::Held {
                    return Ok(StopOutcome::Stopped {
                        run_id: holder.run_id,
                        pid: holder.pgid,
                    });
                }
                if Instant::now() >= deadline {
                    return Ok(StopOutcome::StillRunning {
                        run_id: holder.run_id,
                        pid: holder.pgid,
                    });
                }
                std::thread::sleep(WAIT_POLL);
            }
        }
        LockProbe::Free | LockProbe::Absent => {
            let latest = super::store::latest_run_id(&project_dir)
                .and_then(|id| super::store::read_record(&project_dir.join(id)).ok());
            match latest {
                Some(record)
                    if record.status != RecordStatus::Finished
                        && record.scope_unit.as_deref().is_some_and(plausible_scope) =>
                {
                    let unit = record.scope_unit.unwrap_or_default();
                    let _ = run_bounded(
                        "systemctl",
                        &["--user".into(), "stop".into(), unit.clone().into()],
                        None,
                        &[],
                        Duration::from_secs(60),
                    );
                    Ok(StopOutcome::OrphanScopeStopped { unit })
                }
                _ => Ok(StopOutcome::NotRunning),
            }
        }
        LockProbe::Unknown(errno) => Err(io_error("could not probe the one-off lock", errno)),
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
}
