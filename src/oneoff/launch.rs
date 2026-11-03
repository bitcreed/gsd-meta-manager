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

use super::inbox::ensure_inbox;
use super::land::{git_relevant_planted, track_branch};
use super::report::read_validated;
use super::runtime::{AgentRuntime, HostEnd};
use super::session::{grace_from_env, Supervisor};
use super::store::{
    live_lock_path, mint_run_id, oneoff_root, project_dir_for, project_dir_in, resolve_run,
    rfc3339, run_lock_path, summary_path_from, synthesize, valid_oneoff_run_id, write_record,
    RecordStatus, ReportSource, RunPick, RunRecord, SandboxRecord, MAX_CHANGED_FILES,
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
                "the prompt is {} bytes; at most {MAX_PROMPT_BYTES} fit in one message",
                prompt.len()
            ),
        });
    }
    let agent_argv = ctx.runtime.argv(&opts.model, opts.max_budget_usd);
    ensure_inbox(run_dir).map_err(|e| io_error("create the inbox", e))?;

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
            &bwrap_argv_masking(
                &spec,
                RunDirMode::ReportWritable,
                &app_root_masks(ctx.home),
                &bridged,
            ),
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
    let (mut child, capture, input) =
        spawn_launch_child_piped(&argv, &env, &work, out).map_err(|e| LauncherError::Spawn {
            detail: format!("the agent session: {e}"),
        })?;
    drop(env);
    // C-05: the prompt is the first message on the agent's stdin.
    let mut supervisor = Supervisor::new(run_dir, input, opts.max_budget_usd, grace_from_env());
    supervisor.send_prompt(&prompt, Instant::now());
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
            supervisor.close_input();
            break (child.wait(), HostEnd::UserStop);
        }
        if started.elapsed() >= cap {
            containment.stop(&mut child);
            supervisor.close_input();
            break (child.wait(), HostEnd::WallClock);
        }
        let tick = supervisor.tick(Instant::now());
        if tick.persist {
            record.soft_cap_usd = supervisor.soft_cap_usd();
            record.soft_cap_hit_at = supervisor.soft_cap_hit_at().map(str::to_owned);
            if let Err(e) = write_record(run_dir, record) {
                eprintln!("task run: could not record the soft cap: {e}");
            }
        }
        if tick.stop_soft_cap {
            containment.stop(&mut child);
            supervisor.close_input();
            break (child.wait(), HostEnd::SoftCap);
        }
        match child.try_wait() {
            Ok(Some(status)) => break (Ok(status), HostEnd::Exited),
            Ok(None) => {}
            Err(e) => {
                containment.stop(&mut child);
                supervisor.close_input();
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
    let end_of_session = supervisor.finish();
    record.warnings.extend(end_of_session.warnings);
    record.soft_cap_usd = end_of_session.soft_cap_usd;
    record.soft_cap_hit_at = end_of_session.soft_cap_hit_at;
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
    // C-05: the planted-file gate reads files only, so it runs before any git touches
    // the clone (the tracking fetch starts `upload-pack` inside the agent's repository).
    let planted_findings = git_relevant_planted(ctx.run_dir, work).len();
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
            match track_branch(ctx.project, work, ctx.run_id, &head) {
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
            StopOutcome::NotRunning => "that one-off is not running".to_owned(),
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

/// Stop a one-off of `project`, waiting up to `wait` for its lock to go free.
///
/// `run_id` names the run; without it the run is picked like every other verb does
/// ([`resolve_run`] with `refuse_several_live`): the only live run, or a refusal that
/// lists the live ids when several may be live (quick 261103-hx7, I-03).
pub fn stop(
    project: &Path,
    run_id: Option<&str>,
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
            // A legacy project lock may be held by another run of the old kind; only
            // the selected run's own holder is signalled.
            if holder.run_id != view.record.run_id {
                return Ok(StopOutcome::NotRunning);
            }
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
            // The selected run's own record, not the latest's.
            let record = view.record;
            if record.status != RecordStatus::Finished
                && record.scope_unit.as_deref().is_some_and(plausible_scope)
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
            } else {
                Ok(StopOutcome::NotRunning)
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
}
