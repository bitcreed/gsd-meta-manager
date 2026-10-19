//! The launch loop (Phase 28; LNCH-01, LNCH-08, D-05..D-08, D-16).
//!
//! [`launch`] is the host-side owner of one run for as long as the driver lives.
//! The order is the decision (it follows `driverctl serve`, whose shape this
//! loop extends):
//!
//! 1. validate the run directory and the run id (T-28-04), resolve the token and
//!    read the prompt, so a refusal there writes nothing;
//! 2. take the run lock (D-07) before anything is written, and map a held lock to
//!    [`LauncherError::Locked`] (exit code 3);
//! 3. open the embedded store, bind the run socket and serve it from a thread
//!    (D-06), seed the per-run config directory (D-18);
//! 4. record the launch (`RUN | launch | n=<n> | mode=bwrap-scope`, the scope unit
//!    and the cap limits), start the driver, tick the store every [`TICK`] until
//!    it exits;
//! 5. stop the (empty) scope and look at how the launch ended (D-16): one pure
//!    decision, [`outcome::decide`], first match wins. A `SIGTERM` or `SIGINT` to
//!    the launcher (`driver stop`, D-09) is the user stop: the monitor stops the
//!    scope through [`Containment::stop`], the run is recorded `stopped` with
//!    reason `stop:user`, and nothing relaunches. Otherwise:
//!    * a tripped cap or the driver's own terminal `RUN end` stands (`launch-end`
//!      is recorded without a status change);
//!    * the driver ended its context with `fresh-exit` (status still `running`):
//!      record `launch-end` with [`RunStore::end_launch`] and go back to step 4
//!      **in the same process, with the lock and the socket still held** (D-05,
//!      R3); that is what keeps every fresh-per-item run from reading as
//!      `crashed` between its launches;
//!    * an authentication failure (D-23) relaunches with the host's current token
//!      at most twice, or parks when the stored token was the one rejected;
//!    * an exhausted quota (D-15) parks the run, never retried;
//!    * anything else is classified ([`classify_exit`](crate::driverctl::counters::classify_exit))
//!      `stalled` or `crashed`;
//! 6. remove the socket and drop the store LAST so the lock is released after the
//!    terminal status is on disk (R4).
//!
//! [`launch`] runs in the foreground of the caller; [`launch_detached`] (the
//! default of `driver launch`, D-04) re-executes this binary as a daemon in its
//! own session and returns once that daemon holds the run lock.
//!
//! The hard caps (28-05, D-12..D-14) are enforced from here, outside the sandbox:
//! the dispatch / gap-round [`CapGate`] is installed on the store before the first
//! launch, and [`CapMonitor`] runs after every tick of [`wait_ticking`]. It sums the
//! scope-alive seconds across launches (`extra.active_seconds`, shown as
//! `caps.wall_clock_min.used`), and on `active > limit * 60` or a gap count of
//! `limit + 2` it marks the cap tripped (so the gate denies from that instant),
//! commits `cap-tripped` and only then stops the containment. Each sandboxed launch
//! also carries `RuntimeMaxSec = ceil(remaining) + 300` as the backstop.

use std::ffi::OsString;
use std::fs;
use std::io::{BufRead, Read};
use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
use std::os::unix::process::ExitStatusExt;
use std::path::{Path, PathBuf};
use std::process::Child;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex, PoisonError};
use std::time::{Duration, Instant};

use chrono::Utc;
use rustix::process::Signal;
use serde_json::Value;

use super::auth::{host_access_token, resolve_launch_token, AuthError};
use super::config_dir::{driver_root, run_state_dir_in, seed, SeedSpec};
use super::gate::{new_tripped, trip, tripped_cap, CapGate, Tripped};
use super::host::{decide, detect, Posture};
use super::outcome::{self, LaunchEnd, StreamAccumulator, StreamSummary, Verdict};
use super::process::{respawn_detached, run_bounded, spawn_launch_child};
use super::sandbox::{
    bwrap_argv, child_env, claude_argv, render_prompt, scope_unit_name, systemd_run_argv,
    LaunchPaths, SandboxSpec,
};
use super::LauncherError;
use crate::driver::direction::{direction_lock_path, sweep_stale_temps};
use crate::driver::kill::{signal_group, DRIVER_TEARDOWN_GRACE};
use crate::driver::lock::{
    acquire_at_with_retry, probe_at, read_holder_at, run_lock_path, LockProbe, ACQUIRE_RETRY_BUDGET,
};
use crate::driverctl::counters::outstanding_dispatch;
use crate::driverctl::log::parse_log;
use crate::driverctl::serve::{
    bind_run_socket, hash8, install_signal_handlers, lock_store, socket_path, spawn_accept_loop,
    tick_once, valid_run_id, validate_run_dir, EmbeddedServer, DRIVERS_REL, STALE_TEMP_AGE, TICK,
};
use crate::driverctl::store::{
    CapLimits, DriverSpec, LaunchSpec, RunState, RunStore, TerminalOutcome, TerminalStatus,
    EXTRA_ACTIVE_SECONDS, LOG_FILE, STATE_FILE,
};
use crate::error::LockError;

/// The launcher connection's mode for a sandboxed launch (`RUN | launch | mode=`).
/// A degraded launch records `degraded` instead ([`Posture::as_str`]).
pub const LAUNCHER_MODE: &str = "bwrap-scope";

/// How long `systemctl --user stop <scope>` may take after the driver exited.
const SCOPE_STOP_BUDGET: Duration = Duration::from_secs(30);

/// How long a user stop lets `systemctl --user stop <scope>` take. The scope
/// carries `TimeoutStopSec=15`, so systemd escalates to `SIGKILL` inside this.
const USER_STOP_BUDGET: Duration = Duration::from_secs(60);

/// How long a user stop waits for the driver child to exit once the scope is
/// stopped, before the child itself is killed.
const CHILD_EXIT_WAIT: Duration = Duration::from_secs(5);

/// How often the monitor loop looks at the child between ticks.
const CHILD_POLL: Duration = Duration::from_millis(100);

/// How long [`launch_detached`] waits for the daemon to hold the run lock.
const DETACH_BUDGET: Duration = Duration::from_secs(10);

/// How often [`launch_detached`] looks at the daemon while it waits.
const DETACH_POLL: Duration = Duration::from_millis(100);

/// How many lines of `launcher.log` a failed detach reports.
const DETACH_LOG_LINES: usize = 20;

/// `RuntimeMaxSec` is the remaining active wall clock plus this grace (spike value,
/// D-13): the monitor trips first, the systemd backstop only matters if it died.
const RUNTIME_MAX_GRACE_S: u64 = 300;

/// The monitor writes the summed active seconds at most this often (D-13).
const ACTIVE_PERSIST_EVERY: Duration = Duration::from_secs(15);

/// The `state.json` key counting auth relaunches (D-23).
const EXTRA_AUTH_RELAUNCHES: &str = "auth_relaunches";

/// A stream-json line longer than this is skipped when looking for the result.
const MAX_STREAM_LINE: u64 = 4 * 1024 * 1024;

/// The master prompt's `{MODE}` value (spike `--loop-mode`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoopMode {
    /// One driver context for the whole run.
    Loop,
    /// A fresh driver context per item (relaunch loop: 28-02).
    FreshPerItem,
}

impl LoopMode {
    pub fn as_str(self) -> &'static str {
        match self {
            LoopMode::Loop => "loop",
            LoopMode::FreshPerItem => "fresh-per-item",
        }
    }
}

/// What `driver launch` was asked to do.
#[derive(Debug, Clone, PartialEq)]
pub struct LaunchOptions {
    /// The project root (the registry path of the resolved alias).
    pub project: PathBuf,
    /// The registry alias the project was resolved from.
    pub alias: String,
    pub run_id: String,
    /// The master prompt template (`--prompt-file`, D-03).
    pub prompt_file: PathBuf,
    pub model: String,
    pub loop_mode: LoopMode,
    pub max_budget_usd: f64,
    /// The hard caps (D-12..D-14): recorded on the run, enforced by [`CapGate`] and
    /// [`CapMonitor`].
    pub limits: CapLimits,
    /// `--unsandboxed` (D-29): run the driver as a supervised process group with no
    /// `bwrap` and no scope. Per launch; there is no config default.
    pub unsandboxed: bool,
}

/// How the run stands when [`launch`] returns.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LaunchSummary {
    /// The run's status word (`stopped`, `done`, `crashed`, ...).
    pub status: String,
    pub reason: Option<String>,
    pub launches: u32,
}

fn io_error(context: &str, err: impl std::fmt::Display) -> LauncherError {
    LauncherError::Io {
        detail: format!("{context}: {err}"),
    }
}

/// The embedded store and its socket server (D-06).
struct Session {
    store: Arc<Mutex<RunStore>>,
    stop: Arc<AtomicBool>,
    server: Option<EmbeddedServer>,
    sock: PathBuf,
}

impl Session {
    /// Open the store (it takes the lock), bind `sock` and serve it. A refusal
    /// here has recorded no launch.
    fn start(
        run_dir: &Path,
        project: &Path,
        lock: crate::driver::lock::RunLock,
        sock: &Path,
    ) -> Result<Session, LauncherError> {
        let store = RunStore::open(run_dir, project, lock)?;
        let listener = bind_run_socket(sock)?;
        let store = Arc::new(Mutex::new(store));
        let stop = Arc::new(AtomicBool::new(false));
        let server = match spawn_accept_loop(listener, Arc::clone(&store), Arc::clone(&stop)) {
            Ok(server) => server,
            Err(e) => {
                let _ = fs::remove_file(sock);
                return Err(io_error("spawn the accept thread", e));
            }
        };
        Ok(Session {
            store,
            stop,
            server: Some(server),
            sock: sock.to_owned(),
        })
    }

    /// Stop accepting and let the handlers in flight finish: after this no
    /// request writes to the store.
    fn stop_serving(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(server) = self.server.take() {
            server.shutdown();
        }
    }

    /// Shut down, remove the socket, read the final standing, and drop the store
    /// LAST so the run lock is released after everything is on disk (R4).
    fn close(mut self) -> LaunchSummary {
        self.stop_serving();
        let _ = fs::remove_file(&self.sock);
        let summary = {
            let guard = lock_store(&self.store);
            let state = guard.state();
            LaunchSummary {
                status: state.status.as_str().to_owned(),
                reason: state.status_reason.clone(),
                launches: state.launches,
            }
        };
        match Arc::try_unwrap(self.store) {
            Ok(mutex) => drop(mutex.into_inner().unwrap_or_else(PoisonError::into_inner)),
            // A handler is still running past its timeouts: the process exits
            // right after this returns, and the kernel releases the lock then.
            Err(_still_shared) => {}
        }
        summary
    }
}

/// Everything one launch needs besides the store.
struct LaunchContext {
    opts: LaunchOptions,
    template: String,
    token: super::auth::LaunchToken,
    project: PathBuf,
    run_dir: PathBuf,
    run_id: String,
    state_dir: PathBuf,
    config_dir: PathBuf,
    sock: PathBuf,
    home: PathBuf,
    /// Set by `SIGTERM`/`SIGINT` (`driver stop`): the user wants the run stopped.
    user_stop: Arc<AtomicBool>,
    /// How the driver is contained, decided once from the host probes (D-28).
    posture: Posture,
    /// The cap the monitor has tripped, shared with the store's gate (D-12, T-28-15).
    tripped: Tripped,
}

/// What holds a launch's processes together, and so what stopping it means.
///
/// Two arms: the systemd scope of the sandboxed mode and the process group of the
/// explicit `--unsandboxed` mode (D-29).
enum Containment {
    Scope {
        /// The full unit name, `.scope` suffix included.
        unit: String,
    },
    /// The driver is a process-group leader (`process_group(0)`), so its group id is
    /// its pid. A process that leaves the group (`setsid`) escapes this: that is
    /// what the missing scope costs, and why the mode is opt-in.
    Group { pgid: u32 },
}

impl Containment {
    /// Stop everything the launch started and reap `child`.
    ///
    /// `systemctl --user stop <unit>` signals every process of the cgroup (a
    /// `setsid` grandchild included, which a process-group kill would miss) and
    /// systemd escalates to `SIGKILL` after the unit's `TimeoutStopSec`. It runs
    /// on a helper thread while this one keeps reaping `child`: the scope's main
    /// process is OUR child, a zombie stays in the cgroup until it is waited on,
    /// and a stop that waits for an empty cgroup would otherwise wait for us.
    /// If the child is still there a grace period after `systemctl` returned (the
    /// scope did not exist yet, or systemd is gone) it is killed directly:
    /// `bwrap` is its pid-namespace init, so that takes the sandbox with it.
    fn stop(&self, child: &mut Child) {
        let stopper = match self {
            Containment::Group { pgid } => {
                stop_group(*pgid, child);
                return;
            }
            Containment::Scope { unit } => {
                let unit = unit.clone();
                std::thread::Builder::new()
                    .name("scope-stop".to_owned())
                    .spawn(move || {
                        run_bounded(
                            "systemctl",
                            &["--user".into(), "stop".into(), unit.into()],
                            None,
                            &[],
                            USER_STOP_BUDGET,
                        )
                    })
                    .ok()
            }
        };
        let mut stopper_done_at: Option<Instant> = None;
        loop {
            if matches!(child.try_wait(), Ok(Some(_))) {
                break;
            }
            if stopper_done_at.is_none() && stopper.as_ref().is_none_or(|h| h.is_finished()) {
                stopper_done_at = Some(Instant::now());
            }
            if stopper_done_at.is_some_and(|at| at.elapsed() >= CHILD_EXIT_WAIT) {
                let _ = child.kill();
                break;
            }
            std::thread::sleep(CHILD_POLL);
        }
        if let Some(handle) = stopper {
            let _ = handle.join();
        }
    }
}

impl Containment {
    /// The belt to `--die-with-parent`'s braces after the driver exited by itself:
    /// stop whatever is left of the containment. Best effort.
    fn stop_leftovers(&self) {
        match self {
            // The leader is already reaped; this is best effort for the rest of
            // the group (a background job the driver left behind).
            Containment::Group { pgid } => {
                let _ = signal_group(*pgid, Signal::KILL);
            }
            Containment::Scope { unit } => {
                let _ = run_bounded(
                    "systemctl",
                    &["--user".into(), "stop".into(), unit.into()],
                    None,
                    &[],
                    SCOPE_STOP_BUDGET,
                );
            }
        }
    }
}

/// The synchronous twin of the Phase 17 kill escalation (`driver::kill`): `SIGTERM`
/// to the group, up to [`DRIVER_TEARDOWN_GRACE`] for the leader to exit, then
/// `SIGKILL` to whatever is left of the group. The caller reaps `child`.
///
/// The final `SIGKILL` also goes out when the leader exited inside the grace: a
/// descendant that ignored `SIGTERM` must not outlive the launch. If the leader
/// was already reaped and the group is empty, `kill(-pgid)` is an `ESRCH` to
/// ignore; a recycled pid inside that millisecond window would need a pid
/// wrap-around and is accepted `[inferred — audit]`.
fn stop_group(pgid: u32, child: &mut Child) {
    let _ = signal_group(pgid, Signal::TERM);
    let deadline = Instant::now() + DRIVER_TEARDOWN_GRACE;
    while Instant::now() < deadline {
        if !matches!(child.try_wait(), Ok(None)) {
            break;
        }
        std::thread::sleep(CHILD_POLL);
    }
    let _ = signal_group(pgid, Signal::KILL);
}

/// How the driver process ended.
struct Exited {
    rc: i32,
    /// What the launch's stream-json file says (last result, quota verdict).
    stream: StreamSummary,
    /// The launcher stopped it because the user asked (`driver stop`).
    user_stopped: bool,
    /// The cap the monitor tripped (and so killed the driver for), if any.
    tripped: Option<String>,
}

/// How the wait on the driver child ended.
struct Waited {
    status: std::process::ExitStatus,
    user_stopped: bool,
    /// The monitor tripped this cap and stopped the containment.
    tripped: Option<String>,
}

/// Run one headless driver for `opts.run_id` and block until it ends.
///
/// See the module header for the order. The driver's stdin is `/dev/null` and no
/// terminal or `tmux` is involved (LNCH-08).
pub fn launch(opts: LaunchOptions) -> Result<LaunchSummary, LauncherError> {
    if !valid_run_id(&opts.run_id) {
        return Err(LauncherError::InvalidRun {
            detail: format!("run id {:?} is not a plain run id", opts.run_id),
        });
    }
    let requested_run_dir = opts.project.join(DRIVERS_REL).join(&opts.run_id);
    let (run_dir, run_id) = validate_run_dir(&opts.project, &requested_run_dir)?;
    let project = fs::canonicalize(&opts.project)
        .map_err(|e| io_error(&format!("project {}", opts.project.display()), e))?;

    // Before the lock: a token refusal or an unreadable prompt writes nothing.
    let token = resolve_launch_token(Utc::now()).map_err(|e| LauncherError::Auth {
        detail: e.to_string(),
    })?;
    let template = fs::read_to_string(&opts.prompt_file)
        .map_err(|e| io_error(&format!("prompt file {}", opts.prompt_file.display()), e))?;
    let root = driver_root().ok_or_else(|| LauncherError::Io {
        detail: "no per-user data directory is available for the launcher state".to_owned(),
    })?;
    let home = dirs::home_dir().ok_or_else(|| LauncherError::Io {
        detail: "the home directory is unknown".to_owned(),
    })?;
    let state_dir = run_state_dir_in(&root, &project, &run_id);

    // D-28: the host probes, after every refusal that needs no process and before
    // the lock, so a host that cannot sandbox writes nothing.
    let posture = decide(&detect(), opts.unsandboxed)?;
    // D-13: a run whose active wall clock is used up is refused before anything is
    // written, the lock file included.
    check_wall_clock_budget(&run_dir, &opts.limits)?;

    // D-07: the run lock first. Nothing below writes before this.
    let lock = acquire_at_with_retry(
        &run_lock_path(&run_dir),
        &run_id,
        std::process::id(),
        ACQUIRE_RETRY_BUDGET,
    )
    .map_err(|e| match e {
        LockError::Unavailable { detail } => LauncherError::Io {
            detail: format!("run lock: {detail}"),
        },
        held => LauncherError::Locked {
            detail: held.to_string(),
        },
    })?;

    // D-09: from here a SIGTERM/SIGINT is the user's stop, not the end of this
    // process. Registered before `begin_launch`, as `serve` orders it, so a signal
    // sent once the lock is held is never lost.
    let user_stop = Arc::new(AtomicBool::new(false));
    install_signal_handlers(&user_stop)?;

    // Both lock files exist from here on: bwrap masks them with /dev/null.
    fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .open(direction_lock_path(&run_dir))
        .map_err(|e| io_error("create DIRECTION.lock", e))?;
    if let Err(e) = sweep_stale_temps(&run_dir, STALE_TEMP_AGE) {
        eprintln!("driver launch: could not sweep stale temps: {e}");
    }
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(&state_dir)
        .map_err(|e| io_error(&format!("create {}", state_dir.display()), e))?;

    let sock = socket_path(&run_id, &run_dir)?;
    let session = Session::start(&run_dir, &project, lock, &sock)?;

    let config_dir = state_dir.join("claude-config");
    let binary = std::env::current_exe().map_err(|e| io_error("locate this binary", e))?;
    let seeded = seed(&SeedSpec {
        config_dir: config_dir.clone(),
        host_claude_dir: home.join(".claude"),
        binary,
        run_dir: run_dir.clone(),
    });
    if let Err(e) = seeded {
        session.close();
        return Err(io_error("seed the per-run config directory", e));
    }

    // D-12, D-14: the gate is installed before the first spawn, so no dispatch is ever
    // decided by the default allow-all gate. The marker is shared with the monitor.
    let tripped = new_tripped();
    lock_store(&session.store)
        .set_gate(CapGate::new(opts.limits.clone(), Tripped::clone(&tripped)).into_gate());

    let ctx = LaunchContext {
        opts,
        template,
        token,
        project,
        run_dir,
        run_id,
        state_dir,
        config_dir,
        sock,
        home,
        user_stop,
        posture,
        tripped,
    };
    let mut session = session;
    let mut ctx = ctx;
    loop {
        match run_one_launch(&mut session, &mut ctx) {
            Ok(AfterLaunch::Relaunch) => {}
            Ok(AfterLaunch::Finished) => return Ok(session.close()),
            Err(e) => {
                session.close();
                return Err(e);
            }
        }
    }
}

/// What the run loop does after a launch ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AfterLaunch {
    /// The driver ended its context (`fresh-exit`): start the next launch under
    /// the same lock.
    Relaunch,
    /// The run is over; the caller closes the session.
    Finished,
}

/// One launch: record it, run the driver, record how it ended.
///
/// An `Err` leaves the session open for the caller to close; the failed launch's
/// verdict has already been recorded where one could be.
fn run_one_launch(
    session: &mut Session,
    ctx: &mut LaunchContext,
) -> Result<AfterLaunch, LauncherError> {
    if ctx.user_stop.load(Ordering::SeqCst) {
        // Asked to stop before this launch began (or between two): start nothing.
        session.stop_serving();
        record_user_stop(session, None)?;
        return Ok(AfterLaunch::Finished);
    }
    let n = lock_store(&session.store)
        .state()
        .launches
        .saturating_add(1);
    let unit = scope_unit_name(&ctx.run_id, &hash8(&ctx.run_dir), n);
    let scope = format!("{unit}.scope");

    // D-13: the active seconds the earlier launches used, and what is left of the
    // cap. A relaunch that finds nothing left is the cap tripping between launches.
    let prior_active = active_seconds(lock_store(&session.store).state());
    let remaining = wall_clock_remaining(ctx.opts.limits.wall_clock_min, prior_active);
    if exhausted(remaining) {
        return trip_between_launches(session, ctx, prior_active);
    }

    // A degraded launch has no scope to name.
    let scope_unit = match ctx.posture {
        Posture::BwrapScope => Some(scope.clone()),
        Posture::Degraded { .. } => None,
    };
    lock_store(&session.store).begin_launch(LaunchSpec {
        launcher_mode: ctx.posture.as_str().to_owned(),
        limits: ctx.opts.limits.clone(),
        driver: Some(DriverSpec {
            model: Some(ctx.opts.model.clone()),
            mode: "headless".to_owned(),
        }),
        scope_unit,
        sandbox: Some(ctx.posture.as_str().to_owned()),
        sandbox_missing: ctx.posture.missing().to_vec(),
    })?;

    let budget = Budget {
        prior_active,
        remaining,
    };
    match supervise(session, ctx, n, &unit, &scope, &budget) {
        Ok(exited) => finish_launch(session, ctx, &exited),
        Err(e) => {
            // No request writes to the store past this point.
            session.stop_serving();
            if let Err(store_err) = lock_store(&session.store)
                .set_terminal(TerminalStatus::Stopped, "launcher:startup-failed")
            {
                eprintln!("driver launch: could not record the failed start: {store_err}");
            }
            Err(e)
        }
    }
}

/// Decide what follows a launch that ran to its end (D-05, D-15, D-16, D-23): build
/// the facts, ask [`outcome::decide`], apply the verdict.
fn finish_launch(
    session: &mut Session,
    ctx: &mut LaunchContext,
    exited: &Exited,
) -> Result<AfterLaunch, LauncherError> {
    let end = {
        let guard = lock_store(&session.store);
        let state = guard.state();
        let log_text = fs::read_to_string(ctx.run_dir.join(LOG_FILE)).unwrap_or_default();
        LaunchEnd {
            rc: Some(exited.rc),
            // A stop that arrives while the driver is exiting by itself is still the
            // user's stop: nothing relaunches.
            stop_requested: exited.user_stopped || ctx.user_stop.load(Ordering::SeqCst),
            tripped: exited.tripped.clone(),
            status: state.status.clone(),
            ended_by_driver: state.ended_by_driver,
            stream: exited.stream.clone(),
            token_source: ctx.token.source,
            auth_relaunches: auth_relaunches(state),
            outstanding: outstanding_dispatch(&parse_log(&log_text)).map(|(item, _)| item),
        }
    };
    match outcome::decide(&end) {
        // The driver is gone (its scope was stopped and it was reaped), so the
        // socket can keep serving: nothing can write between here and the next
        // `begin_launch`.
        Verdict::FreshRelaunch => {
            lock_store(&session.store).end_launch(Some(exited.rc))?;
            Ok(AfterLaunch::Relaunch)
        }
        Verdict::AuthRelaunch => auth_relaunch(session, ctx, exited, end.auth_relaunches),
        // No request writes to the store past this point.
        Verdict::Keep => {
            session.stop_serving();
            lock_store(&session.store).end_launch(Some(exited.rc))?;
            Ok(AfterLaunch::Finished)
        }
        Verdict::Park(reason) => {
            session.stop_serving();
            record_terminal(session, TerminalStatus::Parked, &reason, Some(exited.rc))?;
            Ok(AfterLaunch::Finished)
        }
        Verdict::Terminal(status, reason) => {
            session.stop_serving();
            record_terminal(session, status, &reason, Some(exited.rc))?;
            Ok(AfterLaunch::Finished)
        }
    }
}

/// How many auth relaunches the run has made (`extra.auth_relaunches`).
fn auth_relaunches(state: &RunState) -> u32 {
    state
        .extra
        .get(EXTRA_AUTH_RELAUNCHES)
        .and_then(Value::as_u64)
        .and_then(|n| u32::try_from(n).ok())
        .unwrap_or(0)
}

/// The rejected host token is replaced by the host's current one and the run goes on
/// (D-23). The re-read comes first: a host token that cannot be used parks the run
/// without an `auth-relaunch` line for a relaunch that never happened.
fn auth_relaunch(
    session: &mut Session,
    ctx: &mut LaunchContext,
    exited: &Exited,
    made: u32,
) -> Result<AfterLaunch, LauncherError> {
    match host_access_token(&ctx.home.join(".claude"), Utc::now()) {
        Ok(token) => {
            ctx.token = token;
            let attempt = made.saturating_add(1);
            let mut store = lock_store(&session.store);
            store.end_launch(Some(exited.rc))?;
            store.log_launcher_event(
                "auth-relaunch",
                vec![
                    ("attempt", Some(attempt.to_string())),
                    ("source", Some("host".to_owned())),
                ],
            )?;
            store.set_launcher_key(EXTRA_AUTH_RELAUNCHES, serde_json::json!(attempt))?;
            Ok(AfterLaunch::Relaunch)
        }
        Err(e) => {
            let reason = match e {
                AuthError::Expiring { .. } => {
                    "auth: host token expires within 5 minutes; refresh the host login".to_owned()
                }
                other => format!("auth: {other}"),
            };
            session.stop_serving();
            record_terminal(session, TerminalStatus::Parked, &reason, Some(exited.rc))?;
            Ok(AfterLaunch::Finished)
        }
    }
}

// ---------------------------------------------------------------- hard caps (28-05)

/// What one launch knows about the active wall clock when it starts (D-13).
struct Budget {
    /// Active seconds the earlier launches of the run used.
    prior_active: f64,
    /// What is left of the wall-clock cap, or `None` without one.
    remaining: Option<f64>,
}

/// The active seconds the run has used so far (`extra.active_seconds`): `0` when
/// absent, negative or not a finite number.
fn active_seconds(state: &RunState) -> f64 {
    state
        .extra
        .get(EXTRA_ACTIVE_SECONDS)
        .and_then(Value::as_f64)
        .filter(|secs| secs.is_finite() && *secs > 0.0)
        .unwrap_or(0.0)
}

/// Seconds of wall clock left under `limit_min` after `prior_active` seconds, or
/// `None` without a limit. May be zero, negative or NaN: a caller refuses on
/// `is_nan() || <= 0`.
fn wall_clock_remaining(limit_min: Option<f64>, prior_active: f64) -> Option<f64> {
    limit_min.map(|minutes| minutes * 60.0 - prior_active)
}

fn exhausted(remaining: Option<f64>) -> bool {
    remaining.is_some_and(|left| left.is_nan() || left <= 0.0)
}

/// `RuntimeMaxSec` for a scope with `remaining` seconds of wall clock left: the
/// seconds rounded UP, plus [`RUNTIME_MAX_GRACE_S`] (D-13).
fn runtime_max_sec(remaining: f64) -> u64 {
    // `as` saturates, and a NaN is 0.
    (remaining.ceil() as u64).saturating_add(RUNTIME_MAX_GRACE_S)
}

/// The wall-clock minutes as `caps.wall_clock_min.used` shows them: one decimal,
/// half away from zero. Display only; the cap compares unrounded seconds.
fn display_minutes(active_secs: f64) -> f64 {
    (active_secs / 60.0 * 10.0).round() / 10.0
}

/// Refuse a launch whose run has no active wall clock left, before anything is
/// written (D-13): reads `state.json` directly, without the lock or the store.
fn check_wall_clock_budget(run_dir: &Path, limits: &CapLimits) -> Result<(), LauncherError> {
    let Some(minutes) = limits.wall_clock_min else {
        return Ok(());
    };
    let used = fs::read(run_dir.join(STATE_FILE))
        .ok()
        .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok())
        .and_then(|state| state.get(EXTRA_ACTIVE_SECONDS).and_then(Value::as_f64))
        .filter(|secs| secs.is_finite() && *secs > 0.0)
        .unwrap_or(0.0);
    if exhausted(wall_clock_remaining(Some(minutes), used)) {
        return Err(LauncherError::CapExhausted {
            detail: format!(
                "the run has used {} of its {minutes} active wall-clock minutes; raise \
                 --cap-minutes to continue it",
                display_minutes(used)
            ),
        });
    }
    Ok(())
}

/// Which cap the run has crossed, if any: `(cap, used, limit)` as the
/// `cap-tripped` line prints them. Pure; the wall clock is decided first (spike
/// order), on unrounded seconds, and only one cap is ever returned.
///
/// * `wall_clock_min`: `active > limit * 60` (exactly the limit does not trip);
/// * `gap_rounds`: the worst phase's count `>= limit + 2` (the deny at `limit + 1`
///   is the gate's, D-14).
fn decide_trip(
    limits: &CapLimits,
    active_secs: f64,
    worst_gap: u32,
) -> Option<(&'static str, String, String)> {
    if let Some(minutes) = limits.wall_clock_min {
        if active_secs > minutes * 60.0 {
            return Some((
                "wall_clock_min",
                display_minutes(active_secs).to_string(),
                minutes.to_string(),
            ));
        }
    }
    if let Some(gl) = limits.gap_rounds {
        if worst_gap >= gl.saturating_add(2) {
            return Some(("gap_rounds", worst_gap.to_string(), gl.to_string()));
        }
    }
    None
}

/// The highest gap-round count of any phase (the Phase 27 counter), or `0`.
fn worst_gap_count(state: &RunState) -> u32 {
    state
        .caps
        .gap_rounds
        .per_phase
        .values()
        .copied()
        .max()
        .unwrap_or(0)
}

/// The cap monitor of one launch (D-13, D-14): sums the active seconds, persists
/// them, and trips a cap at most once.
struct CapMonitor {
    limits: CapLimits,
    tripped: Tripped,
    prior_active: f64,
    started: Instant,
    last_persist: Instant,
}

impl CapMonitor {
    fn new(limits: CapLimits, tripped: Tripped, prior_active: f64) -> Self {
        let now = Instant::now();
        CapMonitor {
            limits,
            tripped,
            prior_active,
            started: now,
            last_persist: now,
        }
    }

    /// Active seconds so far: the earlier launches' plus this launch's scope-alive time.
    fn active_seconds(&self) -> f64 {
        self.prior_active + self.started.elapsed().as_secs_f64()
    }

    /// Write the active seconds to the store (best effort: a failed write is
    /// reported and the next one retries).
    fn persist(&mut self, session: &Session) {
        let secs = self.active_seconds();
        if let Err(e) = lock_store(&session.store).record_active_seconds(secs) {
            eprintln!("driver launch: could not record the active seconds: {e}");
        }
        self.last_persist = Instant::now();
    }

    /// Persist when due, then look at the caps. On a trip the order is the
    /// decision: the marker first (the gate denies from that instant), then
    /// `cap-tripped` under the store mutex; the CALLER stops the containment. A
    /// second trip in the same run is a no-op, so several caps crossing on one
    /// tick produce one line and one stop.
    fn after_tick(&mut self, session: &Session) -> Option<String> {
        if self.last_persist.elapsed() >= ACTIVE_PERSIST_EVERY {
            self.persist(session);
        }
        if tripped_cap(&self.tripped).is_some() {
            return None;
        }
        let active = self.active_seconds();
        let worst = worst_gap_count(lock_store(&session.store).state());
        let (cap, used, limit) = decide_trip(&self.limits, active, worst)?;
        if !trip(&self.tripped, cap) {
            return None;
        }
        let mut store = lock_store(&session.store);
        if let Err(e) = store.record_active_seconds(active) {
            eprintln!("driver launch: could not record the active seconds: {e}");
        }
        // The kill is enforced even if the record could not be written.
        if let Err(e) = store.trip_cap(cap, &used, &limit) {
            eprintln!("driver launch: could not record cap-tripped: {e}");
        }
        Some(cap.to_owned())
    }
}

/// A relaunch that finds no active wall clock left is the cap tripping between two
/// launches: record it, start nothing.
fn trip_between_launches(
    session: &mut Session,
    ctx: &LaunchContext,
    prior_active: f64,
) -> Result<AfterLaunch, LauncherError> {
    let limit = ctx.opts.limits.wall_clock_min.unwrap_or(0.0);
    if trip(&ctx.tripped, "wall_clock_min") {
        session.stop_serving();
        lock_store(&session.store).trip_cap(
            "wall_clock_min",
            &display_minutes(prior_active).to_string(),
            &limit.to_string(),
        )?;
    }
    Ok(AfterLaunch::Finished)
}

// ---------------------------------------------------------------- detached mode

/// The command line the detached launcher is re-executed with (D-04): every
/// `driver launch` flag the caller gave, then `--foreground --daemon-child`.
///
/// Pure. The paths are taken as they are in `opts`: the daemon's working
/// directory is `/`, so [`launch_detached`] hands this absolute ones. A cap that
/// is `None` is left off (the daemon's defaults then apply; the command line
/// always supplies all three).
pub fn launch_argv(opts: &LaunchOptions, config: Option<&Path>) -> Vec<OsString> {
    let mut argv: Vec<OsString> = Vec::new();
    if let Some(config) = config {
        argv.push("--config".into());
        argv.push(config.as_os_str().to_owned());
    }
    argv.extend(["driver", "launch"].map(OsString::from));
    argv.push(opts.project.as_os_str().to_owned());
    argv.push(OsString::from(&opts.run_id));
    argv.push("--prompt-file".into());
    argv.push(opts.prompt_file.as_os_str().to_owned());
    argv.extend(["--model".into(), OsString::from(&opts.model)]);
    argv.extend(["--loop-mode".into(), opts.loop_mode.as_str().into()]);
    argv.extend([
        "--max-budget-usd".into(),
        opts.max_budget_usd.to_string().into(),
    ]);
    if let Some(dispatches) = opts.limits.dispatches {
        argv.extend(["--cap-dispatches".into(), dispatches.to_string().into()]);
    }
    if let Some(minutes) = opts.limits.wall_clock_min {
        argv.extend(["--cap-minutes".into(), minutes.to_string().into()]);
    }
    if let Some(rounds) = opts.limits.gap_rounds {
        argv.extend(["--cap-gap-rounds".into(), rounds.to_string().into()]);
    }
    if opts.unsandboxed {
        argv.push("--unsandboxed".into());
    }
    argv.extend(["--foreground", "--daemon-child"].map(OsString::from));
    argv
}

/// Where the detached launcher's stdout and stderr go:
/// `<driver-root>/<hash8(project)>/<run-id>/launcher.log`.
pub fn detached_log_path(project: &Path, run_id: &str) -> Result<PathBuf, LauncherError> {
    let root = driver_root().ok_or_else(|| LauncherError::Io {
        detail: "no per-user data directory is available for the launcher state".to_owned(),
    })?;
    let project = fs::canonicalize(project)
        .map_err(|e| io_error(&format!("project {}", project.display()), e))?;
    Ok(run_state_dir_in(&root, &project, run_id).join("launcher.log"))
}

/// The last [`DETACH_LOG_LINES`] lines `launcher.log` gained past `offset`.
fn log_tail(path: &Path, offset: u64) -> String {
    let Ok(bytes) = fs::read(path) else {
        return String::new();
    };
    let start = usize::try_from(offset)
        .unwrap_or(usize::MAX)
        .min(bytes.len());
    let text = String::from_utf8_lossy(&bytes[start..]);
    let lines: Vec<&str> = text.lines().collect();
    lines[lines.len().saturating_sub(DETACH_LOG_LINES)..].join("\n")
}

/// Start the launcher as a daemon in its own session and return its pid once it
/// holds the run lock (D-04).
///
/// Validates the run directory and the prompt file, creates the run's state
/// directory and opens `launcher.log` for append, then re-executes this binary
/// with [`launch_argv`]. The daemon's first act is `setsid`, then the ordinary
/// foreground [`launch`]; this function polls every [`DETACH_POLL`] for up to
/// [`DETACH_BUDGET`]:
///
/// * the daemon exited first: [`LauncherError::DaemonExited`] with its exit code
///   and what it appended to `launcher.log` (a refusal such as "already
///   launched", exit code 3, is passed through unchanged); an exit code of `0`
///   means the whole run fit inside one poll, which is a success;
/// * the run lock is held and its holder record names the daemon: `Ok(pid)`. The
///   liveness test is the lock and the pid is only compared with the process we
///   started, never used to decide that anything is alive;
/// * neither within the budget: the daemon is killed (it has not taken the lock,
///   so no run is lost) and the call fails.
pub fn launch_detached(opts: &LaunchOptions, config: Option<&Path>) -> Result<u32, LauncherError> {
    if !valid_run_id(&opts.run_id) {
        return Err(LauncherError::InvalidRun {
            detail: format!("run id {:?} is not a plain run id", opts.run_id),
        });
    }
    let requested = opts.project.join(DRIVERS_REL).join(&opts.run_id);
    let (run_dir, run_id) = validate_run_dir(&opts.project, &requested)?;
    let project = fs::canonicalize(&opts.project)
        .map_err(|e| io_error(&format!("project {}", opts.project.display()), e))?;
    let prompt_file = fs::canonicalize(&opts.prompt_file)
        .map_err(|e| io_error(&format!("prompt file {}", opts.prompt_file.display()), e))?;
    if !prompt_file.is_file() {
        return Err(io_error(
            &format!("prompt file {}", prompt_file.display()),
            "not a regular file",
        ));
    }
    let config = config
        .map(std::path::absolute)
        .transpose()
        .map_err(|e| io_error("resolve --config", e))?;

    // D-28: refuse here, in the caller's terminal, before any state directory or
    // log exists. The daemon decides again for itself.
    decide(&detect(), opts.unsandboxed)?;
    check_wall_clock_budget(&run_dir, &opts.limits)?;

    let log_path = detached_log_path(&project, &run_id)?;
    let state_dir = log_path.parent().ok_or_else(|| LauncherError::Io {
        detail: "the launcher log has no directory".to_owned(),
    })?;
    fs::DirBuilder::new()
        .recursive(true)
        .mode(0o700)
        .create(state_dir)
        .map_err(|e| io_error(&format!("create {}", state_dir.display()), e))?;
    let log = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .mode(0o600)
        .open(&log_path)
        .map_err(|e| io_error(&format!("open {}", log_path.display()), e))?;
    let offset = log.metadata().map(|m| m.len()).unwrap_or(0);

    let absolute = LaunchOptions {
        project,
        prompt_file,
        ..opts.clone()
    };
    let argv = launch_argv(&absolute, config.as_deref());
    let mut child = respawn_detached(&argv, log).map_err(|e| LauncherError::Spawn {
        detail: format!("the detached launcher: {e}"),
    })?;

    let lock_path = run_lock_path(&run_dir);
    let deadline = Instant::now() + DETACH_BUDGET;
    loop {
        match child.try_wait() {
            Ok(Some(status)) if status.success() => return Ok(child.id()),
            Ok(Some(status)) => {
                return Err(LauncherError::DaemonExited {
                    code: status.code().unwrap_or(1),
                    detail: log_tail(&log_path, offset),
                });
            }
            Ok(None) => {}
            Err(e) => return Err(io_error("wait for the detached launcher", e)),
        }
        if probe_at(&lock_path) == LockProbe::Held
            && read_holder_at(&lock_path).is_some_and(|h| h.pgid == child.id())
        {
            return Ok(child.id());
        }
        if Instant::now() >= deadline {
            let _ = child.kill();
            let _ = child.wait();
            return Err(LauncherError::Io {
                detail: format!(
                    "the detached launcher did not take the run lock within {}s:\n{}",
                    DETACH_BUDGET.as_secs(),
                    log_tail(&log_path, offset)
                ),
            });
        }
        std::thread::sleep(DETACH_POLL);
    }
}

/// Start the driver and wait for it, ticking the store.
fn supervise(
    session: &Session,
    ctx: &LaunchContext,
    n: u32,
    unit: &str,
    scope: &str,
    budget: &Budget,
) -> Result<Exited, LauncherError> {
    let paths = LaunchPaths {
        project: ctx.project.clone(),
        run_dir: ctx.run_dir.clone(),
        config_dir: ctx.config_dir.clone(),
        socket: ctx.sock.clone(),
    };
    let prompt = render_prompt(
        &ctx.template,
        &ctx.run_dir,
        &ctx.project,
        ctx.opts.loop_mode.as_str(),
    );
    let driver_argv = claude_argv(&prompt, &ctx.opts.model, ctx.opts.max_budget_usd);
    // D-29: the degraded driver is `claude` itself: same env, config dir and socket,
    // but no bwrap and no scope, so none of the 28-03 isolation applies to it.
    let argv = match ctx.posture {
        Posture::BwrapScope => {
            let spec = SandboxSpec::from_host(paths, &ctx.home);
            // D-13: the backstop behind the monitor. A degraded launch has none (D-29).
            let runtime_max = budget.remaining.map(runtime_max_sec);
            systemd_run_argv(unit, runtime_max, &bwrap_argv(&spec, &driver_argv))
        }
        Posture::Degraded { .. } => driver_argv,
    };

    let mut path = ctx.config_dir.join("bin").into_os_string();
    if let Some(host_path) = std::env::var_os("PATH") {
        path.push(":");
        path.push(host_path);
    }
    let env = child_env(
        std::env::vars_os(),
        &[
            ("CLAUDE_CONFIG_DIR", ctx.config_dir.clone().into_os_string()),
            ("DRIVERCTL_SOCK", ctx.sock.clone().into_os_string()),
            ("DRIVER_LOCK_HELD", "1".into()),
            ("PATH", path),
            ("CLAUDE_CODE_OAUTH_TOKEN", ctx.token.value.clone().into()),
        ],
    );

    let out_path = ctx.state_dir.join(format!("launch-{n}.jsonl"));
    let out = fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(true)
        .mode(0o600)
        .open(&out_path)
        .map_err(|e| io_error(&format!("create {}", out_path.display()), e))?;

    let mut child =
        spawn_launch_child(&argv, &env, &ctx.project, out).map_err(|e| LauncherError::Spawn {
            detail: format!(
                "{}: {e}",
                match ctx.posture {
                    Posture::BwrapScope => "systemd-run",
                    Posture::Degraded { .. } => "claude",
                }
            ),
        })?;
    drop(env);

    let containment = match ctx.posture {
        Posture::BwrapScope => Containment::Scope {
            unit: scope.to_owned(),
        },
        // `spawn_launch_child` made the child a group leader: its pid is the pgid.
        Posture::Degraded { .. } => Containment::Group { pgid: child.id() },
    };
    let mut monitor = CapMonitor::new(
        ctx.opts.limits.clone(),
        Tripped::clone(&ctx.tripped),
        budget.prior_active,
    );
    let Waited {
        status,
        user_stopped,
        tripped,
    } = wait_ticking(
        session,
        &mut child,
        &ctx.project,
        &ctx.user_stop,
        &containment,
        &mut monitor,
    )?;
    // One last look: a verification written just before the exit still counts.
    tick_once(&session.store, &ctx.project);
    monitor.persist(session);

    // The scope normally ended with its last process; this is the belt to
    // `--die-with-parent`'s braces, and it is best effort. A degraded run has no
    // other containment, so the group is killed here. A user stop or a cap kill has
    // already stopped it.
    if !user_stopped && tripped.is_none() {
        containment.stop_leftovers();
    }

    let rc = status
        .code()
        .or_else(|| status.signal().map(|signal| 128 + signal))
        .unwrap_or(-1);
    Ok(Exited {
        rc,
        stream: summarize_stream_file(&out_path, Utc::now()),
        user_stopped,
        tripped: tripped.or_else(|| tripped_cap(&ctx.tripped)),
    })
}

/// Wait for `child`, running [`tick_once`] and then [`CapMonitor::after_tick`] about
/// every [`TICK`], and stop it through `containment` as soon as `user_stop` is set
/// or a cap trips.
fn wait_ticking(
    session: &Session,
    child: &mut Child,
    project: &Path,
    user_stop: &AtomicBool,
    containment: &Containment,
    monitor: &mut CapMonitor,
) -> Result<Waited, LauncherError> {
    loop {
        let slice_started = Instant::now();
        while slice_started.elapsed() < TICK {
            if user_stop.load(Ordering::SeqCst) {
                containment.stop(child);
                return match child.wait() {
                    Ok(status) => Ok(Waited {
                        status,
                        user_stopped: true,
                        tripped: None,
                    }),
                    Err(e) => Err(io_error("wait for the stopped driver", e)),
                };
            }
            match child.try_wait() {
                Ok(Some(status)) => {
                    return Ok(Waited {
                        status,
                        user_stopped: false,
                        tripped: None,
                    })
                }
                Ok(None) => std::thread::sleep(CHILD_POLL),
                Err(e) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(io_error("wait for the driver", e));
                }
            }
        }
        tick_once(&session.store, project);
        // A trip is already marked and committed when this returns: the gate denies
        // from that instant, and only now is the containment stopped (T-28-15/17).
        if let Some(cap) = monitor.after_tick(session) {
            containment.stop(child);
            return match child.wait() {
                Ok(status) => Ok(Waited {
                    status,
                    user_stopped: false,
                    tripped: Some(cap),
                }),
                Err(e) => Err(io_error("wait for the capped driver", e)),
            };
        }
    }
}

/// Record the user's stop (D-09): `stopped` with reason `stop:user`, for a stop that
/// arrived before a launch began.
fn record_user_stop(session: &Session, rc: Option<i32>) -> Result<(), LauncherError> {
    record_terminal(session, TerminalStatus::Stopped, "stop:user", rc)
}

/// Record a terminal verdict with [`RunStore::set_terminal`]. A terminal status the
/// driver had already recorded is kept (`RUN end` is the driver's word), and then
/// the launch end is logged without a status change like any other driver-ended
/// launch.
fn record_terminal(
    session: &Session,
    status: TerminalStatus,
    reason: &str,
    rc: Option<i32>,
) -> Result<(), LauncherError> {
    let mut store = lock_store(&session.store);
    if store.set_terminal(status, reason)? == TerminalOutcome::KeptDriverEnd {
        store.end_launch(rc)?;
    }
    Ok(())
}

/// Drop the rest of the current line from `reader`.
fn skip_line(reader: &mut impl BufRead) {
    loop {
        let (consumed, found) = match reader.fill_buf() {
            Ok([]) | Err(_) => return,
            Ok(buf) => match buf.iter().position(|b| *b == b'\n') {
                Some(at) => (at + 1, true),
                None => (buf.len(), false),
            },
        };
        reader.consume(consumed);
        if found {
            return;
        }
    }
}

/// Summarize a launch's stream-json file (see [`StreamAccumulator`]): the last
/// result, the quota verdict judged at `now`, the terminal reason. An unreadable
/// file is an empty summary (no result, quota allowed), as in the spike's
/// `classify_run_exit`.
///
/// Reads line by line with a bound per line: the file is written by the model's
/// process, so a single enormous line must not become a single enormous
/// allocation. A line over [`MAX_STREAM_LINE`] is skipped.
fn summarize_stream_file(path: &Path, now: chrono::DateTime<Utc>) -> StreamSummary {
    let mut acc = StreamAccumulator::default();
    let Ok(file) = fs::File::open(path) else {
        return acc.into_summary(now);
    };
    let mut reader = std::io::BufReader::new(file);
    let mut line = Vec::new();
    loop {
        line.clear();
        let Ok(read) = reader
            .by_ref()
            .take(MAX_STREAM_LINE)
            .read_until(b'\n', &mut line)
        else {
            break;
        };
        if read == 0 {
            break;
        }
        if !line.ends_with(b"\n") && read as u64 >= MAX_STREAM_LINE {
            skip_line(&mut reader);
            continue;
        }
        if let Ok(text) = std::str::from_utf8(&line) {
            acc.push(text);
        }
    }
    acc.into_summary(now)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Whether the stream file's last result is a success.
    fn result_ok(path: &Path) -> bool {
        summarize_stream_file(path, Utc::now()).result_ok()
    }

    fn stream(lines: &[&str]) -> (tempfile::TempDir, PathBuf) {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("launch-1.jsonl");
        fs::write(&path, lines.join("\n")).unwrap();
        (dir, path)
    }

    #[test]
    fn the_last_result_line_decides() {
        let (_d, ok) = stream(&[
            r#"{"type":"system"}"#,
            r#"{"type":"result","subtype":"error_max_turns"}"#,
            "not json",
            r#"{"type":"result","subtype":"success"}"#,
        ]);
        assert!(result_ok(&ok));
        let (_d, bad) = stream(&[
            r#"{"type":"result","subtype":"success"}"#,
            r#"{"type":"result","subtype":"error_during_execution"}"#,
        ]);
        assert!(!result_ok(&bad));
    }

    #[test]
    fn no_result_or_no_file_is_not_ok() {
        let (_d, none) = stream(&[r#"{"type":"assistant"}"#]);
        assert!(!result_ok(&none));
        assert!(!result_ok(Path::new("/nonexistent/launch-9.jsonl")));
    }

    #[test]
    fn an_oversized_line_is_skipped_not_allocated() {
        let dir = tempfile::TempDir::new().unwrap();
        let path = dir.path().join("big.jsonl");
        let mut body = String::from(r#"{"type":"result","subtype":"success"}"#);
        body.push('\n');
        body.push_str(&format!(
            "{{\"type\":\"assistant\",\"x\":\"{}\"}}\n",
            "a".repeat(MAX_STREAM_LINE as usize + 10)
        ));
        fs::write(&path, body).unwrap();
        assert!(
            result_ok(&path),
            "the line after the giant one is still seen"
        );
    }

    fn options() -> LaunchOptions {
        LaunchOptions {
            project: PathBuf::from("/work/proj"),
            alias: "p".to_owned(),
            run_id: "r28".to_owned(),
            prompt_file: PathBuf::from("/work/prompt.md"),
            model: "haiku".to_owned(),
            loop_mode: LoopMode::FreshPerItem,
            max_budget_usd: 1.5,
            limits: CapLimits {
                dispatches: Some(3),
                wall_clock_min: Some(12.5),
                gap_rounds: Some(0),
            },
            unsandboxed: false,
        }
    }

    fn words(argv: &[OsString]) -> Vec<String> {
        argv.iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn launch_argv_carries_every_flag_and_the_daemon_markers() {
        let argv = words(&launch_argv(&options(), None));
        assert_eq!(
            argv,
            [
                "driver",
                "launch",
                "/work/proj",
                "r28",
                "--prompt-file",
                "/work/prompt.md",
                "--model",
                "haiku",
                "--loop-mode",
                "fresh-per-item",
                "--max-budget-usd",
                "1.5",
                "--cap-dispatches",
                "3",
                "--cap-minutes",
                "12.5",
                "--cap-gap-rounds",
                "0",
                "--foreground",
                "--daemon-child",
            ]
        );
    }

    #[test]
    fn launch_argv_puts_the_config_first_and_omits_unset_caps() {
        let mut opts = options();
        opts.limits = CapLimits::default();
        let argv = words(&launch_argv(&opts, Some(Path::new("/etc/gmm.json"))));
        assert_eq!(&argv[..3], ["--config", "/etc/gmm.json", "driver"]);
        assert!(!argv.iter().any(|a| a.starts_with("--cap-")), "{argv:?}");
        assert_eq!(&argv[argv.len() - 2..], ["--foreground", "--daemon-child"]);
    }

    #[test]
    fn launch_argv_round_trips_through_the_real_parser() {
        use crate::cli::{Cli, Commands, DriverAction, DriverLoopMode};
        use clap::Parser;

        let mut argv = vec![OsString::from("gsd-meta-manager")];
        argv.extend(launch_argv(&options(), Some(Path::new("/etc/gmm.json"))));
        let cli = Cli::try_parse_from(argv).expect("the daemon's own command line parses");
        let Some(Commands::Driver {
            action: DriverAction::Launch(args),
        }) = cli.command
        else {
            panic!("not a driver launch");
        };
        assert_eq!(args.project, "/work/proj");
        assert_eq!(args.run_id, "r28");
        assert_eq!(args.loop_mode, DriverLoopMode::FreshPerItem);
        assert_eq!(args.cap_dispatches, 3);
        assert_eq!(args.cap_minutes, 12.5);
        assert_eq!(args.cap_gap_rounds, 0);
        assert!(args.foreground && args.daemon_child);
    }

    #[test]
    fn launch_argv_carries_unsandboxed_only_when_asked_and_it_parses_back() {
        use crate::cli::{Cli, Commands, DriverAction};
        use clap::Parser;

        assert!(!words(&launch_argv(&options(), None)).contains(&"--unsandboxed".to_owned()));
        let mut opts = options();
        opts.unsandboxed = true;
        let argv = words(&launch_argv(&opts, None));
        assert_eq!(
            &argv[argv.len() - 3..],
            ["--unsandboxed", "--foreground", "--daemon-child"]
        );

        let mut full = vec![OsString::from("gsd-meta-manager")];
        full.extend(launch_argv(&opts, None));
        let cli = Cli::try_parse_from(full).expect("the daemon's own command line parses");
        let Some(Commands::Driver {
            action: DriverAction::Launch(args),
        }) = cli.command
        else {
            panic!("not a driver launch");
        };
        assert!(args.unsandboxed);
    }

    fn limits(wall: Option<f64>, gap: Option<u32>) -> CapLimits {
        CapLimits {
            dispatches: None,
            wall_clock_min: wall,
            gap_rounds: gap,
        }
    }

    #[test]
    fn the_wall_clock_trips_one_second_past_the_limit_and_not_at_it() {
        let lim = limits(Some(10.0), None);
        assert_eq!(decide_trip(&lim, 599.0, 0), None);
        assert_eq!(decide_trip(&lim, 600.0, 0), None, "exactly the limit");
        assert_eq!(
            decide_trip(&lim, 601.0, 0),
            Some(("wall_clock_min", "10".to_owned(), "10".to_owned()))
        );
        assert!(decide_trip(&lim, 600.000_000_1, 0).is_some(), "unrounded");
    }

    #[test]
    fn the_wall_clock_compares_unrounded_seconds_and_displays_rounded_minutes() {
        // 0.05 min = 3 s. 3.0 s displays as 0.1 (half away from zero) yet does not trip.
        let lim = limits(Some(0.05), None);
        assert_eq!(display_minutes(3.0), 0.1);
        assert_eq!(decide_trip(&lim, 3.0, 0), None);
        assert_eq!(
            decide_trip(&lim, 4.2, 0),
            Some(("wall_clock_min", "0.1".to_owned(), "0.05".to_owned()))
        );
        assert_eq!(display_minutes(2.9), 0.0);
        assert_eq!(display_minutes(0.0), 0.0);
        assert_eq!(display_minutes(754.0), 12.6);
    }

    #[test]
    fn the_gap_cap_kills_at_limit_plus_two_and_not_before() {
        let lim = limits(None, Some(2));
        assert_eq!(decide_trip(&lim, 0.0, 3), None, "L + 1 is only a deny");
        assert_eq!(
            decide_trip(&lim, 0.0, 4),
            Some(("gap_rounds", "4".to_owned(), "2".to_owned()))
        );
        // Limit 0: the deny is at count 1, the kill at count 2.
        let zero = limits(None, Some(0));
        assert_eq!(decide_trip(&zero, 0.0, 1), None);
        assert!(decide_trip(&zero, 0.0, 2).is_some());
        // No limit never trips; the largest limit does not overflow.
        assert_eq!(decide_trip(&limits(None, None), 1e9, 1_000), None);
        assert_eq!(
            decide_trip(&limits(None, Some(u32::MAX)), 0.0, u32::MAX),
            Some(("gap_rounds", u32::MAX.to_string(), u32::MAX.to_string()))
        );
    }

    #[test]
    fn two_caps_crossing_on_one_tick_trip_exactly_one_and_the_wall_clock_wins() {
        let lim = limits(Some(1.0), Some(0));
        let tripped = decide_trip(&lim, 120.0, 9).expect("both are crossed");
        assert_eq!(tripped.0, "wall_clock_min");
        let marker = new_tripped();
        assert!(trip(&marker, tripped.0));
        assert!(!trip(&marker, "gap_rounds"), "a cap trips at most once");
        assert_eq!(tripped_cap(&marker).as_deref(), Some("wall_clock_min"));
    }

    #[test]
    fn runtime_max_sec_rounds_the_remaining_seconds_up_and_adds_the_grace() {
        assert_eq!(runtime_max_sec(3.0), 303);
        assert_eq!(runtime_max_sec(3.000_000_1), 304);
        assert_eq!(runtime_max_sec(0.4), 301);
        assert_eq!(runtime_max_sec(3600.0), 3900);
        assert_eq!(runtime_max_sec(f64::INFINITY), u64::MAX);
    }

    #[test]
    fn remaining_wall_clock_is_the_limit_less_the_active_seconds() {
        assert_eq!(wall_clock_remaining(None, 99.0), None);
        assert_eq!(wall_clock_remaining(Some(10.0), 0.0), Some(600.0));
        assert_eq!(wall_clock_remaining(Some(10.0), 590.5), Some(9.5));
        assert!(!exhausted(None));
        assert!(!exhausted(Some(0.001)));
        assert!(exhausted(Some(0.0)), "nothing left");
        assert!(exhausted(Some(-1.0)));
        assert!(exhausted(Some(f64::NAN)), "a NaN limit refuses");
        assert!(exhausted(wall_clock_remaining(Some(0.0), 0.0)), "limit 0");
        assert!(exhausted(wall_clock_remaining(Some(f64::NAN), 0.0)));
    }

    #[test]
    fn active_seconds_reads_the_launcher_key_and_ignores_garbage() {
        let mut state = RunState::default();
        assert_eq!(active_seconds(&state), 0.0);
        state
            .extra
            .insert(EXTRA_ACTIVE_SECONDS.to_owned(), serde_json::json!(12.5));
        assert_eq!(active_seconds(&state), 12.5);
        state
            .extra
            .insert(EXTRA_ACTIVE_SECONDS.to_owned(), serde_json::json!("many"));
        assert_eq!(active_seconds(&state), 0.0);
        state
            .extra
            .insert(EXTRA_ACTIVE_SECONDS.to_owned(), serde_json::json!(-4.0));
        assert_eq!(active_seconds(&state), 0.0);
    }

    #[test]
    fn the_worst_gap_count_is_the_maximum_over_phases() {
        let mut state = RunState::default();
        assert_eq!(worst_gap_count(&state), 0);
        state.caps.gap_rounds.per_phase.insert("1".to_owned(), 2);
        state.caps.gap_rounds.per_phase.insert("2".to_owned(), 5);
        state.caps.gap_rounds.per_phase.insert("3".to_owned(), 1);
        assert_eq!(worst_gap_count(&state), 5);
    }

    #[test]
    fn loop_modes_render_the_prompt_words() {
        assert_eq!(LoopMode::Loop.as_str(), "loop");
        assert_eq!(LoopMode::FreshPerItem.as_str(), "fresh-per-item");
    }
}
