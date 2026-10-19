//! The `driver` subcommand: the launcher's command line (Phase 28; D-01, D-02).
//!
//! `driver launch <alias|path> <run-id> --prompt-file <file>` resolves the project
//! through the registry exactly as the positional launch target does
//! ([`launch_target::resolve_launch_target`](gsd_meta_manager::launch_target)),
//! then hands the project root to [`gsd_meta_manager::launcher::run::launch`],
//! which blocks for the run's life when `--foreground` is given. Without it the
//! launcher re-executes itself as a daemon (D-04): the hidden `--daemon-child`
//! flag marks that re-executed process, whose first act is `setsid`.
//!
//! It runs synchronously in `main`, after tracing is initialised and before the
//! tokio runtime exists: the launcher is std threads and blocking I/O, and must
//! never run inside an `async fn`. The return value is the process exit code
//! (`3` for a run that is already launched, `1` for every other refusal).

use std::io::Write;
use std::path::PathBuf;

use gsd_meta_manager::cli::{DriverAction, LaunchArgs};
use gsd_meta_manager::config::{load_config, Config};
use gsd_meta_manager::launch_target::resolve_launch_target;
use gsd_meta_manager::launcher::LauncherError;

/// Write one line to stderr, ignoring failure (a closed pipe must not panic).
fn err_line(text: &str) {
    let mut stderr = std::io::stderr().lock();
    let _ = writeln!(stderr, "{text}");
}

/// Write one line to stdout, ignoring failure.
#[cfg(target_os = "linux")]
fn out_line(text: &str) {
    let mut stdout = std::io::stdout().lock();
    let _ = writeln!(stdout, "{text}");
}

/// Run one `driver` verb and return the process exit code.
pub fn run(action: DriverAction, config: Option<PathBuf>) -> i32 {
    match action {
        DriverAction::Launch(args) => launch_main(&args, config),
        DriverAction::Stop {
            project,
            run_id,
            wait_secs,
        } => stop_main(&project, &run_id, wait_secs, config),
    }
}

/// The project root a `driver launch` target names, through the registry.
#[cfg(target_os = "linux")]
fn resolve_project(target: &str, config: Option<PathBuf>) -> Result<(String, PathBuf), String> {
    let config_path = config.unwrap_or_else(Config::default_path);
    let config = load_config(&config_path).map_err(|e| format!("{e:#}"))?;
    let cwd =
        std::env::current_dir().map_err(|e| format!("cannot read the working directory: {e}"))?;
    let alias = resolve_launch_target(&config, target, &cwd)
        .map_err(|e| format!("{e}\n{}", e.suggestion()))?;
    let project = config
        .projects
        .get(&alias)
        .map(|p| p.path.clone())
        .ok_or_else(|| format!("project {alias:?} is not in the registry"))?;
    Ok((alias, project))
}

#[cfg(target_os = "linux")]
fn launch_main(args: &LaunchArgs, config: Option<PathBuf>) -> i32 {
    use gsd_meta_manager::cli::DriverLoopMode;
    use gsd_meta_manager::driverctl::store::CapLimits;
    use gsd_meta_manager::launcher::run::{
        detached_log_path, launch, launch_detached, LaunchOptions, LoopMode,
    };

    if args.daemon_child {
        // First, before anything is opened or written: the daemon leaves the
        // caller's session so a closed terminal cannot reach it.
        if let Err(e) = rustix::process::setsid() {
            err_line(&format!("driver launch: setsid failed: {e}"));
            return 1;
        }
    }
    let (alias, project) = match resolve_project(&args.project, config.clone()) {
        Ok(resolved) => resolved,
        Err(message) => {
            err_line(&format!("driver launch: {message}"));
            return 1;
        }
    };
    let options = LaunchOptions {
        project,
        alias,
        run_id: args.run_id.clone(),
        prompt_file: args.prompt_file.clone(),
        model: args.model.clone(),
        loop_mode: match args.loop_mode {
            DriverLoopMode::Loop => LoopMode::Loop,
            DriverLoopMode::FreshPerItem => LoopMode::FreshPerItem,
        },
        max_budget_usd: args.max_budget_usd,
        limits: CapLimits {
            dispatches: Some(args.cap_dispatches),
            wall_clock_min: Some(args.cap_minutes),
            gap_rounds: Some(args.cap_gap_rounds),
        },
        unsandboxed: args.unsandboxed,
    };
    if !args.foreground {
        return match launch_detached(&options, config.as_deref()) {
            Ok(pid) => {
                let log = detached_log_path(&options.project, &args.run_id)
                    .map(|p| p.display().to_string())
                    .unwrap_or_default();
                out_line(&format!(
                    "driver launch: run {} started (pid {pid}); log: {log}",
                    args.run_id
                ));
                0
            }
            Err(error) => report("launch", &error),
        };
    }
    match launch(options) {
        Ok(summary) => {
            out_line(&format!(
                "driver launch: run {} ended status={} reason={}",
                args.run_id,
                summary.status,
                summary.reason.as_deref().unwrap_or("none")
            ));
            0
        }
        Err(error) => report("launch", &error),
    }
}

/// `driver stop`: signal the launcher (or stop an orphaned scope) and say what
/// happened. `1` when the launcher was signalled but still holds the run.
#[cfg(target_os = "linux")]
fn stop_main(target: &str, run_id: &str, wait_secs: u64, config: Option<PathBuf>) -> i32 {
    use gsd_meta_manager::launcher::stop::{stop_run, StopReport};

    let (_alias, project) = match resolve_project(target, config) {
        Ok(resolved) => resolved,
        Err(message) => {
            err_line(&format!("driver stop: {message}"));
            return 1;
        }
    };
    match stop_run(&project, run_id, std::time::Duration::from_secs(wait_secs)) {
        Ok(report) => {
            out_line(&format!("driver stop: run {run_id}: {report}"));
            i32::from(matches!(report, StopReport::StillRunning { .. }))
        }
        Err(error) => report("stop", &error),
    }
}

#[cfg(not(target_os = "linux"))]
fn stop_main(_target: &str, _run_id: &str, _wait_secs: u64, _config: Option<PathBuf>) -> i32 {
    report("stop", &LauncherError::UnsupportedPlatform)
}

/// The launcher needs `flock`, `renameat2`, `bwrap` and `systemd-run`: Linux only.
///
/// D-31: without `--unsandboxed` the refusal names what a sandboxed run needs
/// (`bwrap` and `systemd-run`); with it the run store is what is missing, because
/// the degraded mode is functional only on Linux in this phase. Both exit 1.
#[cfg(not(target_os = "linux"))]
fn launch_main(args: &LaunchArgs, _config: Option<PathBuf>) -> i32 {
    if args.unsandboxed {
        err_line(
            "driver launch: unsupported platform: the run store needs Linux \
             (no renamex_np splice yet)",
        );
        return 1;
    }
    report(
        "launch",
        &LauncherError::SandboxUnavailable {
            missing: vec!["bwrap".to_owned(), "systemd-run".to_owned()],
        },
    )
}

fn report(verb: &str, error: &LauncherError) -> i32 {
    err_line(&format!("driver {verb}: {error}"));
    error.exit_code()
}
