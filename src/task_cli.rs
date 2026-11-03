//! `gsd-meta-manager task run|status|stop|land` (quick 261103-f5y): the host verbs of
//! one-off sandboxed task dispatch.
//!
//! Synchronous, dispatched in `main` after tracing and before the tokio runtime, like
//! `driver`. The return value is the exit code: `0` success, `3` a one-off is already
//! running on the project, `1` every other refusal or failure.

use std::io::Write;

/// Write one line to stderr, ignoring failure.
fn err_line(text: &str) {
    let mut stderr = std::io::stderr().lock();
    let _ = writeln!(stderr, "{text}");
}

/// Run one `task` verb and return the exit code.
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
pub fn run(action: gsd_meta_manager::cli::TaskAction, config: Option<std::path::PathBuf>) -> i32 {
    let _ = (action, config);
    err_line("task: one-off runs need Linux or macOS");
    1
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub use unix::run;

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod unix {
    use std::path::{Path, PathBuf};
    use std::time::Duration;

    use gsd_meta_manager::cli::{LandAction, TaskAction, TaskRunArgs};
    use gsd_meta_manager::launcher::LauncherError;
    use gsd_meta_manager::oneoff::inbox::{
        send_message, wait_outcome, InboxError, InjectionOutcome,
    };
    use gsd_meta_manager::oneoff::land;
    use gsd_meta_manager::oneoff::launch::{
        run_detached, run_foreground, stop, OneoffOptions, StopOutcome,
    };
    use gsd_meta_manager::oneoff::store::{read_latest, LiveState, OneoffView, ReportSource};
    use gsd_meta_manager::oneoff::todos::{title_from, valid_todo_file_name, PENDING_REL};
    use gsd_meta_manager::oneoff::TaskSpec;
    use gsd_meta_manager::text::render_for_terminal;

    use super::err_line;
    use crate::driver_cli::resolve_project;

    fn out_line(text: &str) {
        use std::io::Write;
        let mut stdout = std::io::stdout().lock();
        let _ = writeln!(stdout, "{text}");
    }

    /// A launcher refusal, prefixed with the verb.
    fn report(verb: &str, error: &LauncherError) -> i32 {
        err_line(&format!("task {verb}: {error}"));
        error.exit_code()
    }

    /// A shown string, escaped for the terminal.
    fn shown(text: &str) -> String {
        render_for_terminal(text).to_string()
    }

    pub fn run(action: TaskAction, config: Option<PathBuf>) -> i32 {
        match action {
            TaskAction::Run(args) => run_main(&args, config),
            TaskAction::Status { project, json } => status_main(&project, json, config),
            TaskAction::Stop { project, wait_secs } => stop_main(&project, wait_secs, config),
            TaskAction::Send {
                project,
                text,
                wait_secs,
            } => send_main(&project, &text, wait_secs, config),
            TaskAction::Land { action } => land_main(action, config),
        }
    }

    /// The todo's title as the project's checkout shows it (the file name when it is
    /// unreadable; the run itself checks that the clone has it).
    fn todo_spec(project: &Path, file_name: &str) -> Result<TaskSpec, String> {
        if !valid_todo_file_name(file_name) {
            return Err(format!(
                "{} is not a todo file name under {PENDING_REL}",
                shown(file_name)
            ));
        }
        let path = project.join(PENDING_REL).join(file_name);
        let text = match std::fs::symlink_metadata(&path) {
            Ok(meta) if meta.file_type().is_file() => {
                let mut bytes = std::fs::read(&path).unwrap_or_default();
                bytes.truncate(8 * 1024);
                String::from_utf8_lossy(&bytes).into_owned()
            }
            _ => String::new(),
        };
        Ok(TaskSpec::Todo {
            file_name: file_name.to_owned(),
            title: title_from(&text, file_name),
        })
    }

    fn run_main(args: &TaskRunArgs, config: Option<PathBuf>) -> i32 {
        if args.daemon_child {
            // First, before anything is opened or written: leave the caller's session.
            if let Err(e) = rustix::process::setsid() {
                err_line(&format!("task run: setsid failed: {e}"));
                return 1;
            }
        }
        let (alias, project, registry) = match resolve_project(&args.project, config.clone()) {
            Ok(resolved) => resolved,
            Err(message) => {
                err_line(&format!("task run: {message}"));
                return 1;
            }
        };
        // I-8: the project's `driver.egress` allowlist, resolved as `driver launch` does.
        let egress_allow = match registry
            .projects
            .get(&alias)
            .map(gsd_meta_manager::config::RegisteredProject::driver_egress_allowlist)
            .transpose()
        {
            Ok(hosts) => hosts.unwrap_or_default(),
            Err(e) => {
                err_line(&format!("task run: projects.{alias}.{e}"));
                return 1;
            }
        };
        let task = match (&args.todo, &args.text) {
            (Some(file), None) => match todo_spec(&project, file) {
                Ok(spec) => spec,
                Err(message) => {
                    err_line(&format!("task run: {message}"));
                    return 1;
                }
            },
            (None, Some(text)) => TaskSpec::Text(text.clone()),
            _ => {
                err_line("task run: give exactly one of --todo and --text");
                return 1;
            }
        };
        let options = OneoffOptions {
            project,
            alias,
            mode: args.mode,
            task,
            model: args.model.clone(),
            max_budget_usd: args.max_budget_usd,
            cap_minutes: args.cap_minutes,
            unsandboxed: args.unsandboxed,
            run_id: args.run_id.clone(),
            egress_allow,
        };
        if !args.foreground {
            return match run_detached(&options, config.as_deref()) {
                Ok(started) => {
                    out_line(&format!(
                        "task run: {} started (pid {}); log: {}",
                        started.run_id,
                        started.pid,
                        started.log.display()
                    ));
                    0
                }
                Err(e) => report("run", &e),
            };
        }
        match run_foreground(options) {
            Ok(record) => {
                let outcome = record.outcome.as_ref().map_or("unknown", |o| o.word());
                out_line(&format!("task run: {} finished: {outcome}", record.run_id));
                if let Some(report) = &record.report {
                    out_line(&format!(
                        "task run: {}: {}",
                        report.status.as_str(),
                        shown(&report.headline)
                    ));
                }
                0
            }
            Err(e) => report("run", &e),
        }
    }

    /// The ~8-line summary (I-18): the agent's report and the host's facts, labelled.
    fn summary_lines(view: &OneoffView) -> Vec<String> {
        let record = &view.record;
        let mut lines = vec![format!(
            "run:       {} ({}, {} {})",
            shown(&record.run_id),
            view.live.as_str(),
            record.mode.command(),
            shown(&record.model)
        )];
        let source = match &record.report_source {
            Some(ReportSource::Agent) => "agent-reported".to_owned(),
            Some(ReportSource::Synthesized { reason }) => {
                format!("synthesized by the host: {}", shown(reason))
            }
            None => "no report yet".to_owned(),
        };
        if let Some(report) = &record.report {
            lines.push(format!("status:    {} ({source})", report.status.as_str()));
            lines.push(format!("headline:  {}", shown(&report.headline)));
            let list = |items: &[String]| {
                if items.is_empty() {
                    "-".to_owned()
                } else {
                    items
                        .iter()
                        .map(|i| shown(i))
                        .collect::<Vec<_>>()
                        .join("; ")
                }
            };
            lines.push(format!("changed:   {}", list(&report.changed)));
            lines.push(format!("verified:  {}", list(&report.verified)));
            lines.push(format!("followups: {}", list(&report.followups)));
        }
        lines.push(format!(
            "host:      outcome {}, {} commit(s), SUMMARY {}",
            record.outcome.as_ref().map_or("pending", |o| o.word()),
            record.commits.unwrap_or(0),
            if record.summary_path.is_some() {
                "present"
            } else {
                "absent"
            }
        ));
        if let Some(path) = &record.summary_path {
            lines.push(format!(
                "summary:   {}",
                shown(&view.run_dir.join("work").join(path).display().to_string())
            ));
        }
        for warning in &record.warnings {
            lines.push(format!("warning:   {}", shown(warning)));
        }
        for landing in &record.landed {
            lines.push(format!(
                "landed:    {:?} {} {}",
                landing.kind,
                shown(&landing.at),
                shown(&landing.detail)
            ));
        }
        lines
    }

    fn status_main(target: &str, json: bool, config: Option<PathBuf>) -> i32 {
        let (alias, project, _) = match resolve_project(target, config) {
            Ok(resolved) => resolved,
            Err(message) => {
                err_line(&format!("task status: {message}"));
                return 1;
            }
        };
        let Some(view) = read_latest(&project) else {
            err_line(&format!(
                "task status: no one-off run for {}",
                shown(&alias)
            ));
            return 1;
        };
        if json {
            let value = serde_json::json!({
                "record": view.record,
                "live": view.live,
                "run_dir": view.run_dir,
            });
            out_line(&value.to_string());
        } else {
            for line in summary_lines(&view) {
                out_line(&line);
            }
        }
        0
    }

    fn stop_main(target: &str, wait_secs: u64, config: Option<PathBuf>) -> i32 {
        let (_, project, _) = match resolve_project(target, config) {
            Ok(resolved) => resolved,
            Err(message) => {
                err_line(&format!("task stop: {message}"));
                return 1;
            }
        };
        match stop(&project, Duration::from_secs(wait_secs)) {
            Ok(StopOutcome::Stopped { run_id, pid }) => {
                out_line(&format!("task stop: {run_id} stopped (pid {pid})"));
                0
            }
            Ok(StopOutcome::StillRunning { run_id, pid }) => {
                err_line(&format!(
                    "task stop: {run_id} (pid {pid}) was signalled but is still running after {wait_secs}s"
                ));
                1
            }
            Ok(StopOutcome::OrphanScopeStopped { unit }) => {
                out_line(&format!(
                    "task stop: no live one-off launcher; stopped the leftover scope {unit}"
                ));
                0
            }
            Ok(StopOutcome::NotRunning) => {
                out_line("task stop: no one-off is running on this project");
                0
            }
            Err(e) => report("stop", &e),
        }
    }

    /// Wait for the launcher's record of request `id` and report it like `verb` says.
    fn report_request(verb: &str, run_dir: &Path, id: &str, wait_secs: u64) -> i32 {
        match wait_outcome(run_dir, id, Duration::from_secs(wait_secs)) {
            Some(record) if record.outcome == InjectionOutcome::Delivered => {
                out_line(&format!("task {verb}: delivered ({id})"));
                0
            }
            Some(record) => {
                err_line(&format!(
                    "task {verb}: refused: {}",
                    shown(record.reason.as_deref().unwrap_or("no reason given"))
                ));
                1
            }
            None => {
                err_line(&format!("task {verb}: not picked up within {wait_secs}s"));
                1
            }
        }
    }

    fn send_main(target: &str, text: &str, wait_secs: u64, config: Option<PathBuf>) -> i32 {
        let (alias, project, _) = match resolve_project(target, config) {
            Ok(resolved) => resolved,
            Err(message) => {
                err_line(&format!("task send: {message}"));
                return 1;
            }
        };
        let not_live = || {
            err_line(&format!(
                "task send: {}'s one-off is not live",
                shown(&alias)
            ));
            1
        };
        let Some(view) = read_latest(&project) else {
            return not_live();
        };
        if view.live != LiveState::Running {
            return not_live();
        }
        match send_message(&view.run_dir, text) {
            Ok(id) => report_request("send", &view.run_dir, &id, wait_secs),
            Err(InboxError::NotLive) => not_live(),
            Err(e) => {
                err_line(&format!("task send: {}", shown(&e.to_string())));
                1
            }
        }
    }

    fn land_main(action: LandAction, config: Option<PathBuf>) -> i32 {
        let (verb, target, run_id) = match &action {
            LandAction::Merge { project, run_id } => ("merge", project, run_id),
            LandAction::Push { project, run_id } => ("push", project, run_id),
            LandAction::Discard { project, run_id } => ("discard", project, run_id),
        };
        let (alias, project, _) = match resolve_project(target, config) {
            Ok(resolved) => resolved,
            Err(message) => {
                err_line(&format!("task land {verb}: {message}"));
                return 1;
            }
        };
        let result = match action {
            LandAction::Merge { .. } => land::merge(&project, run_id),
            LandAction::Push { .. } => {
                match gsd_meta_manager::launcher::config_dir::driver_root() {
                    Some(root) => land::push(&project, &alias, run_id, &root),
                    None => Err("no per-user data directory is available".to_owned()),
                }
            }
            LandAction::Discard { .. } => land::discard(&project, run_id),
        };
        match result {
            Ok(message) => {
                for line in message.lines() {
                    out_line(&format!("task land {verb}: {}", shown(line)));
                }
                0
            }
            Err(message) => {
                err_line(&format!("task land {verb}: {}", shown(&message)));
                1
            }
        }
    }
}
