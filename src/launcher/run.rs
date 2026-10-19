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
//! 5. stop the (empty) scope, stop serving, classify how the launch ended
//!    ([`classify_exit`], D-16) and record it with `set_terminal`, remove the
//!    socket, and drop the store LAST so the lock is released after the terminal
//!    status is on disk (R4).
//!
//! This plan runs one launch in the foreground of the caller. The relaunch loop,
//! `crash-detected` evidence across launches, stop and the detached default are
//! 28-02; the caps' enforcement is 28-05. The limits are recorded here, through
//! [`LaunchSpec::limits`], so `caps.*.limit` already shows them.

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
use serde_json::Value;

use super::auth::resolve_launch_token;
use super::config_dir::{driver_root, run_state_dir_in, seed, SeedSpec};
use super::process::{run_bounded, spawn_launch_child};
use super::sandbox::{
    bwrap_argv, child_env, claude_argv, render_prompt, scope_unit_name, systemd_run_argv,
    LaunchPaths, SandboxSpec,
};
use super::LauncherError;
use crate::driver::direction::{direction_lock_path, sweep_stale_temps};
use crate::driver::lock::{acquire_at_with_retry, run_lock_path, ACQUIRE_RETRY_BUDGET};
use crate::driverctl::counters::{classify_exit, outstanding_dispatch, ExitClass};
use crate::driverctl::log::parse_log;
use crate::driverctl::serve::{
    bind_run_socket, hash8, lock_store, socket_path, spawn_accept_loop, tick_once, valid_run_id,
    validate_run_dir, EmbeddedServer, DRIVERS_REL, STALE_TEMP_AGE, TICK,
};
use crate::driverctl::store::{
    CapLimits, DriverSpec, LaunchSpec, RunStore, TerminalStatus, LOG_FILE,
};
use crate::error::LockError;

/// The launcher connection's mode for a sandboxed launch (`RUN | launch | mode=`).
pub const LAUNCHER_MODE: &str = "bwrap-scope";

/// How long `systemctl --user stop <scope>` may take after the driver exited.
const SCOPE_STOP_BUDGET: Duration = Duration::from_secs(30);

/// How often the monitor loop looks at the child between ticks.
const CHILD_POLL: Duration = Duration::from_millis(100);

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
    /// The caps to record on the run (D-12..D-14); enforcement is 28-05's.
    pub limits: CapLimits,
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
}

/// How the driver process ended.
struct Exited {
    rc: i32,
    result_ok: bool,
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
    };
    run_one_launch(session, &ctx)
}

/// One launch: record it, run the driver, record how it ended.
fn run_one_launch(
    mut session: Session,
    ctx: &LaunchContext,
) -> Result<LaunchSummary, LauncherError> {
    let n = lock_store(&session.store)
        .state()
        .launches
        .saturating_add(1);
    let unit = scope_unit_name(&ctx.run_id, &hash8(&ctx.run_dir), n);
    let scope = format!("{unit}.scope");

    let begun = lock_store(&session.store).begin_launch(LaunchSpec {
        launcher_mode: LAUNCHER_MODE.to_owned(),
        limits: ctx.opts.limits.clone(),
        driver: Some(DriverSpec {
            model: Some(ctx.opts.model.clone()),
            mode: "headless".to_owned(),
        }),
        scope_unit: Some(scope.clone()),
    });
    if let Err(e) = begun {
        session.close();
        return Err(e.into());
    }

    let ended = supervise(&session, ctx, n, &unit, &scope);

    // No request writes to the store past this point.
    session.stop_serving();
    let outcome = match ended {
        Ok(exited) => record_end(&session, ctx, &exited),
        Err(e) => {
            if let Err(store_err) = lock_store(&session.store)
                .set_terminal(TerminalStatus::Stopped, "launcher:startup-failed")
            {
                eprintln!("driver launch: could not record the failed start: {store_err}");
            }
            Err(e)
        }
    };
    let summary = session.close();
    outcome.map(|()| summary)
}

/// Start the driver and wait for it, ticking the store.
fn supervise(
    session: &Session,
    ctx: &LaunchContext,
    n: u32,
    unit: &str,
    scope: &str,
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
    let spec = SandboxSpec::from_host(paths, &ctx.home);
    let argv = systemd_run_argv(unit, None, &bwrap_argv(&spec, &driver_argv));

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
            detail: format!("systemd-run: {e}"),
        })?;
    drop(env);

    let status = wait_ticking(session, &mut child, &ctx.project)?;
    // One last look: a verification written just before the exit still counts.
    tick_once(&session.store, &ctx.project);

    // The scope normally ended with its last process; this is the belt to
    // `--die-with-parent`'s braces, and it is best effort.
    let _ = run_bounded(
        "systemctl",
        &["--user".into(), "stop".into(), scope.into()],
        None,
        &[],
        SCOPE_STOP_BUDGET,
    );

    let rc = status
        .code()
        .or_else(|| status.signal().map(|signal| 128 + signal))
        .unwrap_or(-1);
    Ok(Exited {
        rc,
        result_ok: last_result_ok(&out_path),
    })
}

/// Wait for `child`, running [`tick_once`] about every [`TICK`].
fn wait_ticking(
    session: &Session,
    child: &mut Child,
    project: &Path,
) -> Result<std::process::ExitStatus, LauncherError> {
    loop {
        let slice_started = Instant::now();
        while slice_started.elapsed() < TICK {
            match child.try_wait() {
                Ok(Some(status)) => return Ok(status),
                Ok(None) => std::thread::sleep(CHILD_POLL),
                Err(e) => {
                    let _ = child.kill();
                    let _ = child.wait();
                    return Err(io_error("wait for the driver", e));
                }
            }
        }
        tick_once(&session.store, project);
    }
}

/// Record how the launch ended (D-16): a driver `RUN end` keeps the status the
/// driver set; otherwise [`classify_exit`] says `stalled` or `crashed`.
fn record_end(
    session: &Session,
    ctx: &LaunchContext,
    exited: &Exited,
) -> Result<(), LauncherError> {
    let log_text = fs::read_to_string(ctx.run_dir.join(LOG_FILE)).unwrap_or_default();
    let outstanding = outstanding_dispatch(&parse_log(&log_text)).map(|(item, _)| item);
    let ended_by_driver = lock_store(&session.store).state().ended_by_driver;
    let verdict = classify_exit(
        exited.rc,
        false,
        ended_by_driver,
        exited.result_ok,
        outstanding.as_deref(),
    );
    if let Some((class, reason)) = verdict {
        let status = match class {
            ExitClass::Stalled => TerminalStatus::Stalled,
            ExitClass::Crashed => TerminalStatus::Crashed,
        };
        lock_store(&session.store).set_terminal(status, &reason)?;
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

/// Whether the LAST `"type":"result"` line of a stream-json file has subtype
/// `success`. No result line, or an unreadable file, is `false` (spike
/// `classify_run_exit`).
///
/// Reads line by line with a bound per line: the file is written by the model's
/// process, so a single enormous line must not become a single enormous
/// allocation.
fn last_result_ok(path: &Path) -> bool {
    let Ok(file) = fs::File::open(path) else {
        return false;
    };
    let mut reader = std::io::BufReader::new(file);
    let mut line = Vec::new();
    let mut ok = false;
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
        if line.first() != Some(&b'{') {
            continue;
        }
        if let Ok(Value::Object(map)) = serde_json::from_slice::<Value>(&line) {
            if map.get("type").and_then(Value::as_str) == Some("result") {
                ok = map.get("subtype").and_then(Value::as_str) == Some("success");
            }
        }
    }
    ok
}

#[cfg(test)]
mod tests {
    use super::*;

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
        assert!(last_result_ok(&ok));
        let (_d, bad) = stream(&[
            r#"{"type":"result","subtype":"success"}"#,
            r#"{"type":"result","subtype":"error_during_execution"}"#,
        ]);
        assert!(!last_result_ok(&bad));
    }

    #[test]
    fn no_result_or_no_file_is_not_ok() {
        let (_d, none) = stream(&[r#"{"type":"assistant"}"#]);
        assert!(!last_result_ok(&none));
        assert!(!last_result_ok(Path::new("/nonexistent/launch-9.jsonl")));
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
            last_result_ok(&path),
            "the line after the giant one is still seen"
        );
    }

    #[test]
    fn loop_modes_render_the_prompt_words() {
        assert_eq!(LoopMode::Loop.as_str(), "loop");
        assert_eq!(LoopMode::FreshPerItem.as_str(), "fresh-per-item");
    }
}
