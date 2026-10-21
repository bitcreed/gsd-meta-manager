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

use gsd_meta_manager::cli::{AuthAction, DriverAction, LaunchArgs};
#[cfg(any(target_os = "linux", target_os = "macos"))]
use gsd_meta_manager::config::{load_config, Config};
#[cfg(any(target_os = "linux", target_os = "macos"))]
use gsd_meta_manager::launch_target::resolve_launch_target;
use gsd_meta_manager::launcher::LauncherError;

/// Write one line to stderr, ignoring failure (a closed pipe must not panic).
fn err_line(text: &str) {
    let mut stderr = std::io::stderr().lock();
    let _ = writeln!(stderr, "{text}");
}

/// Write one line to stdout, ignoring failure.
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn out_line(text: &str) {
    let mut stdout = std::io::stdout().lock();
    let _ = writeln!(stdout, "{text}");
}

/// `text` as one terminal-safe shell word for a command line printed to be copied
/// (WR-03): control and bidi characters are escaped first, then a word with anything
/// outside the plain path/alias set is single-quoted.
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn shell_word(text: &str) -> String {
    let shown = gsd_meta_manager::text::render_for_terminal(text).to_string();
    let plain = !shown.is_empty()
        && shown
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "_./:@%+=,-".contains(c));
    if plain {
        shown
    } else {
        format!("'{}'", shown.replace('\'', "'\\''"))
    }
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
        DriverAction::Push { project, run_id } => push_main(&project, &run_id, config),
        DriverAction::New { project, slug } => new_main(&project, &slug, config),
        DriverAction::Check { project, run_id } => check_main(&project, &run_id, config),
        DriverAction::Auth { action } => auth_main(action),
    }
}

/// `driver auth setup|status` (D-21).
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn auth_main(action: AuthAction) -> i32 {
    match action {
        AuthAction::Setup => auth_setup(),
        AuthAction::Status => auth_status_main(),
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn auth_main(_action: AuthAction) -> i32 {
    report("auth", &LauncherError::UnsupportedPlatform)
}

/// Explain, run `claude setup-token` on the user's terminal, read the pasted
/// token from one line of stdin and store it. The value is never printed.
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn auth_setup() -> i32 {
    use gsd_meta_manager::launcher::auth::{
        has_expected_prefix, store_token, token_path, validate_pasted_token, EXPECTED_TOKEN_PREFIX,
        MAX_PASTED_LEN,
    };
    use gsd_meta_manager::launcher::process::run_interactive;
    use std::io::{BufRead, Read};

    let Some(path) = token_path() else {
        err_line(
            "driver auth setup: this platform has no user config directory to store a token in",
        );
        return 1;
    };
    out_line(
        "driver auth setup: a long-lived token from `claude setup-token` keeps driver runs \
         independent of your host login.\n\
         Copying the host's own token instead would log the host out the first time either side \
         refreshes it.\n\
         `claude setup-token` runs now on this terminal; follow its instructions.",
    );
    match run_interactive("claude", &["setup-token"]) {
        Ok(status) if status.success() => {}
        Ok(status) => {
            err_line(&format!(
                "driver auth setup: `claude setup-token` did not succeed ({status}); nothing was stored"
            ));
            return 1;
        }
        Err(e) => {
            err_line(&format!(
                "driver auth setup: could not run `claude setup-token`: {e}; nothing was stored"
            ));
            return 1;
        }
    }
    {
        let mut stdout = std::io::stdout().lock();
        let _ = write!(stdout, "Paste the token printed above, then press Enter: ");
        let _ = stdout.flush();
    }
    let mut line = String::new();
    // One line, bounded: a stray file piped in cannot grow this without limit.
    let stdin = std::io::stdin();
    let read = (&mut stdin.lock())
        .take(MAX_PASTED_LEN as u64 * 2)
        .read_line(&mut line);
    match read {
        Ok(0) => {
            err_line("driver auth setup: nothing was pasted (end of input); nothing was stored");
            return 1;
        }
        Ok(_) => {}
        Err(e) => {
            err_line(&format!(
                "driver auth setup: could not read the pasted token: {e}; nothing was stored"
            ));
            return 1;
        }
    }
    if more_input_pending(&stdin) {
        err_line(
            "driver auth setup: more than one line was pasted; paste only the token; \
             nothing was stored",
        );
        return 1;
    }
    let token = match validate_pasted_token(&line) {
        Ok(token) => token,
        Err(e) => {
            err_line(&format!("driver auth setup: {e}; nothing was stored"));
            return 1;
        }
    };
    if !has_expected_prefix(&token) {
        err_line(&format!(
            "driver auth setup: warning: the token does not start with `{EXPECTED_TOKEN_PREFIX}`; \
             storing it anyway"
        ));
    }
    match store_token(&path, &token) {
        Ok(()) => {
            out_line(&format!("stored at {} (0600)", path.display()));
            0
        }
        Err(e) => {
            err_line(&format!("driver auth setup: {e}; nothing was stored"));
            1
        }
    }
}

/// Whether stdin already holds more than whitespace after the line just read.
///
/// A multi-line paste arrives as several lines, and `read_line` takes only the
/// first; storing that first line silently would store half of whatever was
/// pasted. The check must not block (a terminal has nothing more to give until
/// the user types), so stdin goes non-blocking for the one `fill_buf` and its
/// flags are restored on every path.
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn more_input_pending(stdin: &std::io::Stdin) -> bool {
    use std::io::BufRead;

    let Ok(flags) = rustix::fs::fcntl_getfl(stdin) else {
        return false;
    };
    if rustix::fs::fcntl_setfl(stdin, flags | rustix::fs::OFlags::NONBLOCK).is_err() {
        return false;
    }
    let pending = {
        let mut lock = stdin.lock();
        match lock.fill_buf() {
            Ok(buf) => buf.iter().any(|b| !b.is_ascii_whitespace()),
            Err(_) => false,
        }
    };
    let _ = rustix::fs::fcntl_setfl(stdin, flags);
    pending
}

/// Report presence, path, mode and the host token's expiry. Never a value.
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn auth_status_main() -> i32 {
    use gsd_meta_manager::launcher::auth::auth_status;

    let now = chrono::Utc::now();
    let status = auth_status(now);
    let path = status.path.as_ref().map_or_else(
        || "<no config directory>".to_owned(),
        |p| p.display().to_string(),
    );
    match (&status.stored, &status.stored_problem) {
        (Some((_, mode)), None) => out_line(&format!(
            "stored token: present at {path} (mode {mode:04o})"
        )),
        (Some((_, mode)), Some(problem)) => out_line(&format!(
            "stored token: present at {path} (mode {mode:04o}) but REFUSED at launch: {problem}"
        )),
        (None, _) => out_line(&format!("stored token: none (would be stored at {path})")),
    }
    match (status.host_expires_at, status.host_minutes_left) {
        (Some(at), Some(minutes)) if minutes >= 0 => out_line(&format!(
            "host token: expires {} (in {minutes} min)",
            at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
        )),
        (Some(at), _) => out_line(&format!(
            "host token: expired {}",
            at.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
        )),
        (None, _) => out_line("host token: none found"),
    }
    out_line(&format!("next launch uses: {}", status.next_launch()));
    0
}

/// The alias, project root and loaded registry a `driver` target names.
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn resolve_project(
    target: &str,
    config: Option<PathBuf>,
) -> Result<(String, PathBuf, Config), String> {
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
    Ok((alias, project, config))
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
fn launch_main(args: &LaunchArgs, config: Option<PathBuf>) -> i32 {
    use gsd_meta_manager::cli::DriverLoopMode;
    use gsd_meta_manager::driverctl::store::CapLimits;
    use gsd_meta_manager::launcher::push::policy_for;
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
    let (alias, project, registry) = match resolve_project(&args.project, config.clone()) {
        Ok(resolved) => resolved,
        Err(message) => {
            err_line(&format!("driver launch: {message}"));
            return 1;
        }
    };
    let pr_policy = policy_for(
        &alias,
        registry
            .projects
            .get(&alias)
            .and_then(|p| p.driver_opt_in.as_ref()),
    );
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
        // D-26: read from the registry here, and again by the detached child, which
        // runs this same function with the same `--config`.
        open_pr: registry.preferences.driver_open_pr(),
        pr_policy,
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
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn stop_main(target: &str, run_id: &str, wait_secs: u64, config: Option<PathBuf>) -> i32 {
    use gsd_meta_manager::launcher::stop::{stop_run, StopReport};

    let (_alias, project, _registry) = match resolve_project(target, config) {
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

/// `driver new`: mint a run directory from the embedded templates (D-08 .. D-10).
///
/// Prints where DIRECTION.md is and the exact next commands; opens no editor.
/// `1` for every refusal, and nothing is created on a refusal.
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn new_main(target: &str, slug: &str, config: Option<PathBuf>) -> i32 {
    use gsd_meta_manager::driver_templates::create_run_dir;

    let (_alias, project, _registry) = match resolve_project(target, config) {
        Ok(resolved) => resolved,
        Err(message) => {
            err_line(&format!("driver new: {message}"));
            return 1;
        }
    };
    match create_run_dir(&project, slug, chrono::Utc::now()) {
        Ok(run) => {
            out_line(&format!("driver new: created run {}", run.run_id));
            out_line(&format!("  direction: {}", run.direction.display()));
            let target = shell_word(target);
            out_line(&format!(
                "  check:  gsd-meta-manager driver check {target} {}",
                run.run_id
            ));
            out_line(&format!(
                "  launch: gsd-meta-manager driver launch {target} {}",
                run.run_id
            ));
            0
        }
        Err(error) => {
            err_line(&format!("driver new: {error}"));
            1
        }
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn new_main(_target: &str, _slug: &str, _config: Option<PathBuf>) -> i32 {
    report("new", &LauncherError::UnsupportedPlatform)
}

/// `driver check`: lint a run's DIRECTION.md and print every problem (D-11, DRIVE-03).
///
/// Read-only. Problems then warnings, one `DIRECTION.md:<line>: ...` line each;
/// nothing is printed for a clean file. `0` without problems, `1` with any, and `1`
/// for a run that cannot be read.
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn check_main(target: &str, run_id: &str, config: Option<PathBuf>) -> i32 {
    use gsd_meta_manager::driver_templates::lint::lint_direction;
    use gsd_meta_manager::driverctl::serve::{valid_run_id, validate_run_dir, RUN_ID_RE};
    use gsd_meta_manager::state_reader::driver_frontmatter::DRIVER_RUNS_REL;

    let (_alias, project, _registry) = match resolve_project(target, config) {
        Ok(resolved) => resolved,
        Err(message) => {
            err_line(&format!("driver check: {message}"));
            return 1;
        }
    };
    if !valid_run_id(run_id) {
        err_line(&format!(
            "driver check: run id {run_id:?} does not match {RUN_ID_RE}"
        ));
        return 1;
    }
    let run_dir = match validate_run_dir(&project, &project.join(DRIVER_RUNS_REL).join(run_id)) {
        Ok((run_dir, _id)) => run_dir,
        Err(error) => {
            err_line(&format!("driver check: {error}"));
            return 1;
        }
    };
    let direction = run_dir.join("DIRECTION.md");
    let text = match std::fs::read_to_string(&direction) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            err_line(&format!(
                "driver check: DIRECTION.md not found in {}; run driver new",
                run_dir.display()
            ));
            return 1;
        }
        Err(e) => {
            err_line(&format!(
                "driver check: cannot read {}: {e}",
                direction.display()
            ));
            return 1;
        }
    };
    let report = lint_direction(&text);
    for problem in &report.problems {
        out_line(&problem.to_string());
    }
    for warning in &report.warnings {
        out_line(&warning.as_warning());
    }
    i32::from(!report.is_clean())
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn check_main(_target: &str, _run_id: &str, _config: Option<PathBuf>) -> i32 {
    report("check", &LauncherError::UnsupportedPlatform)
}

/// `driver push`: the push step for a finished run whose launcher is gone.
/// `0` for a push, an already-done push or nothing to push; `1` for a refusal or
/// a failure.
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn push_main(target: &str, run_id: &str, config: Option<PathBuf>) -> i32 {
    use gsd_meta_manager::launcher::config_dir::driver_root;
    use gsd_meta_manager::launcher::push::{policy_for, push_finished_run, PushContext};

    let (alias, project, registry) = match resolve_project(target, config) {
        Ok(resolved) => resolved,
        Err(message) => {
            err_line(&format!("driver push: {message}"));
            return 1;
        }
    };
    let Some(driver_root) = driver_root() else {
        err_line("driver push: no per-user data directory is available for the launcher state");
        return 1;
    };
    let opt_in = registry
        .projects
        .get(&alias)
        .and_then(|p| p.driver_opt_in.as_ref());
    let ctx = PushContext {
        project,
        policy: policy_for(&alias, opt_in),
        alias,
        run_id: run_id.to_owned(),
        driver_root,
        open_pr: registry.preferences.driver_open_pr(),
    };
    match push_finished_run(&ctx) {
        Ok(outcome) => {
            if outcome.is_success() {
                out_line(&format!("driver push: run {run_id}: {outcome}"));
                0
            } else {
                err_line(&format!("driver push: run {run_id}: {outcome}"));
                1
            }
        }
        Err(error) => report("push", &error),
    }
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn push_main(_target: &str, _run_id: &str, _config: Option<PathBuf>) -> i32 {
    report("push", &LauncherError::UnsupportedPlatform)
}

#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn stop_main(_target: &str, _run_id: &str, _wait_secs: u64, _config: Option<PathBuf>) -> i32 {
    report("stop", &LauncherError::UnsupportedPlatform)
}

/// The launcher is supported on Linux and macOS only; this stub is every other
/// platform (Windows).
///
/// D-31: without `--unsandboxed` the refusal names what a sandboxed run needs
/// (`bwrap` and `systemd-run`); with it the platform itself is what is missing,
/// because the degraded mode runs on Linux and macOS only. Both exit 1.
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn launch_main(args: &LaunchArgs, _config: Option<PathBuf>) -> i32 {
    if args.unsandboxed {
        err_line("driver launch: unsupported platform: the launcher needs Linux or macOS");
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
    match error {
        // Already `DIRECTION.md:<line>: <message>` lines: a verb prefix would make the
        // first one a different shape from the rest.
        LauncherError::DirectionInvalid { .. } => err_line(&error.to_string()),
        _ => err_line(&format!("driver {verb}: {error}")),
    }
    error.exit_code()
}

#[cfg(all(test, any(target_os = "linux", target_os = "macos")))]
mod tests {
    use super::shell_word;

    #[test]
    fn a_plain_alias_or_path_is_printed_as_is() {
        assert_eq!(shell_word("proj-1"), "proj-1");
        assert_eq!(shell_word("/home/me/proj_a/x.y"), "/home/me/proj_a/x.y");
    }

    #[test]
    fn anything_else_is_single_quoted_and_free_of_control_characters() {
        assert_eq!(shell_word("my proj"), "'my proj'");
        assert_eq!(shell_word("it's"), "'it'\\''s'");
        assert_eq!(shell_word(""), "''");
        let shown = shell_word("a\u{1b}[31m b\u{202e}");
        assert!(shown.starts_with('\'') && shown.ends_with('\''), "{shown}");
        assert!(
            !shown.chars().any(|c| c.is_control() || c == '\u{202e}'),
            "{shown:?}"
        );
    }
}
