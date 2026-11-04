//! `gsd-meta-manager task run|status|list|stop|send|budget|attach|land` (quick
//! 261103-f5y): the host verbs of one-off sandboxed task dispatch.
//!
//! Synchronous, dispatched in `main` after tracing and before the tokio runtime, like
//! `driver`. The return value is the exit code: `0` success, `1` every refusal or
//! failure. Any number of runs may be live on a project (quick 261103-hx7); `status`,
//! `stop`, `send` and `budget` take `--run-id` and refuse, listing the live ids, when
//! none is given and several runs may be live.

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
    use gsd_meta_manager::oneoff::cancel::{StopBy, StopKind};
    use gsd_meta_manager::oneoff::inbox::{
        send_message, set_soft_cap, wait_outcome, InboxError, InjectionOutcome,
    };
    use gsd_meta_manager::oneoff::land;
    use gsd_meta_manager::oneoff::launch::{
        run_detached, run_foreground, stop, OneoffOptions, StopOutcome, StopTarget,
    };
    use gsd_meta_manager::oneoff::resume::{self, ResumeFrom, ResumeSpec, ResumeStarted};
    use gsd_meta_manager::oneoff::session::validate_soft_cap;
    use gsd_meta_manager::oneoff::store::{
        project_dir_for, read_runs_in, resolve_run, BranchState, LandingKind, LiveState, MergeHow,
        OneoffView, ReportSource, RunPick,
    };
    use gsd_meta_manager::oneoff::stream::{
        meter_of_file, meter_text, rate_text, soft_cap_text, Follower, Meter,
    };
    use gsd_meta_manager::oneoff::todos::{title_from, valid_todo_file_name, PENDING_REL};
    use gsd_meta_manager::oneoff::{TaskSpec, STREAM_FILE};
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

    /// The first 12 characters of a recorded sha (char-safe: the record is untrusted).
    fn sha12(sha: &str) -> String {
        sha.chars().take(12).collect()
    }

    /// The run `verb` acts on (quick 261103-hx7, I-03): `run_id`, or the newest run, or
    /// with `refuse_several_live` the only live one. A refusal is printed and is exit 1.
    fn pick(
        verb: &str,
        project: &Path,
        run_id: Option<&str>,
        refuse_several_live: bool,
    ) -> Result<OneoffView, i32> {
        let Some(project_dir) = project_dir_for(project) else {
            err_line(&format!(
                "task {verb}: no one-off directory for this project"
            ));
            return Err(1);
        };
        resolve_run(&project_dir, run_id, refuse_several_live).map_err(|pick| {
            err_line(&shown(&pick.message(&format!("task {verb}"))));
            1
        })
    }

    pub fn run(action: TaskAction, config: Option<PathBuf>) -> i32 {
        match action {
            TaskAction::Run(args) => run_main(&args, config),
            TaskAction::Status {
                project,
                run_id,
                json,
            } => status_main(&project, run_id.as_deref(), json, config),
            TaskAction::List { project, json } => list_main(&project, json, config),
            TaskAction::Stop {
                project,
                run_id,
                wait_secs,
                kill,
                pause,
                resolver,
            } => {
                let kind = if kill {
                    StopKind::Kill
                } else if pause {
                    StopKind::Pause
                } else {
                    StopKind::Hard
                };
                let target = if resolver {
                    StopTarget::Landing
                } else {
                    StopTarget::Auto
                };
                stop_main(&project, run_id.as_deref(), wait_secs, kind, target, config)
            }
            TaskAction::Resume {
                project,
                run_id,
                from,
                budget,
                detach,
                unsandboxed,
            } => resume_main(
                &project,
                run_id.as_deref(),
                from,
                budget,
                detach,
                unsandboxed,
                config,
            ),
            TaskAction::Send {
                project,
                text,
                run_id,
                wait_secs,
            } => send_main(&project, &text, run_id.as_deref(), wait_secs, config),
            TaskAction::Budget {
                project,
                soft,
                clear,
                run_id,
                wait_secs,
            } => budget_main(
                &project,
                soft.filter(|_| !clear),
                run_id.as_deref(),
                wait_secs,
                config,
            ),
            TaskAction::Attach {
                project,
                run_id,
                no_input,
            } => attach_main(&project, run_id.as_deref(), no_input, config),
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
        // klf I-24: a resume takes its task from the old run's record.
        let resume = match &args.resume_from {
            Some(old_id) => {
                if !gsd_meta_manager::oneoff::store::valid_oneoff_run_id(old_id) {
                    err_line("task run: --resume-from is not a one-off run id");
                    return 1;
                }
                Some(ResumeSpec {
                    from_run: old_id.clone(),
                    start: args.resume_start.unwrap_or(ResumeFrom::Branch),
                })
            }
            None => None,
        };
        let resumed_task = match (&resume, project_dir_for(&project)) {
            (Some(spec), Some(dir)) => {
                match gsd_meta_manager::oneoff::store::read_view(&dir, &spec.from_run) {
                    Some(old) => Some(old.record.task),
                    None => {
                        err_line("task run: the run to resume was not found");
                        return 1;
                    }
                }
            }
            (Some(_), None) => {
                err_line("task run: no one-off directory for this project");
                return 1;
            }
            (None, _) => None,
        };
        let task = match (&args.todo, &args.text, &args.backlog) {
            _ if resumed_task.is_some() => resumed_task.unwrap_or(TaskSpec::Text(String::new())),
            (Some(file), None, None) => match todo_spec(&project, file) {
                Ok(spec) => spec,
                Err(message) => {
                    err_line(&format!("task run: {message}"));
                    return 1;
                }
            },
            (None, Some(text), None) => TaskSpec::Text(text.clone()),
            // 261103-ubn: the parent and the re-executed daemon both read the entry
            // from the project checkout (the todo pattern, TOCTOU-tolerant).
            (None, None, Some(number)) => {
                match gsd_meta_manager::oneoff::backlog::backlog_spec(&project, number) {
                    Ok(spec) => spec,
                    Err(message) => {
                        err_line(&format!("task run: {message}"));
                        return 1;
                    }
                }
            }
            _ => {
                err_line("task run: give exactly one of --todo, --text and --backlog");
                return 1;
            }
        };
        // o2r D-B: the parent scans; the daemon it starts never does (it would race the
        // directory its own parent just claimed).
        if !args.daemon_child {
            recycle(&project, "task run");
        }
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
            resume,
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
    fn summary_lines(view: &OneoffView, live: Option<&Meter>) -> Vec<String> {
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
        if let Some(from) = &record.resumed_from {
            lines.push(format!("resumed:   from {}", shown(from)));
        }
        if let Some(by) = &record.resumed_by {
            lines.push(format!("resumed:   as {}", shown(by)));
        }
        if let Some(merge) = &record.resume_merge {
            lines.push(match (&merge.failure, &merge.merge_sha) {
                (Some(why), _) => format!(
                    "resume:    merging {} failed: {}",
                    shown(&merge.branch),
                    shown(why)
                ),
                (None, Some(sha)) => format!(
                    "resume:    merged {} as {}{}",
                    shown(&merge.branch),
                    shown(&sha[..sha.len().min(12)]),
                    if merge.conflicted.is_empty() {
                        String::new()
                    } else {
                        format!(
                            " [resolved: {}]",
                            merge
                                .conflicted
                                .iter()
                                .map(|f| shown(f))
                                .collect::<Vec<_>>()
                                .join(", ")
                        )
                    }
                ),
                (None, None) => format!("resume:    {}", shown(&merge.branch)),
            });
        }
        if let Some(stop) = &record.stop {
            let pending = stop.ended_at.is_none() && view.live.may_be_live();
            let state = match (&stop.refused, pending, stop.kind) {
                (Some(why), _, _) => format!("pause refused ({})", shown(why)),
                (None, true, StopKind::Pause) => "pausing".to_owned(),
                (None, true, StopKind::Hard) => "stopping".to_owned(),
                (None, true, StopKind::Kill) => "killing".to_owned(),
                (None, false, kind) => kind.word().to_owned(),
            };
            lines.push(format!(
                "stop:      {state} ({} by {} at {}{})",
                stop.kind.word(),
                stop.by.word(),
                shown(&stop.requested_at),
                if stop.escalated { ", escalated" } else { "" }
            ));
        }
        lines.push(format!("target:    {}", shown(&record.land_target_text())));
        if let Some(pruned) = &record.pruned {
            lines.push(format!(
                "clone:     {} ({})",
                shown(&pruned.note),
                shown(&pruned.at)
            ));
        }
        lines.push(match record.branch_state() {
            BranchState::Tracked { name, head } => match head {
                Some(head) => format!(
                    "branch:    {} @ {}",
                    shown(&name),
                    shown(&head[..head.len().min(12)])
                ),
                None => format!("branch:    {}", shown(&name)),
            },
            BranchState::Gone { why } => format!("branch:    none ({})", shown(&why)),
            BranchState::Untracked { note } if note.is_empty() => "branch:    none".to_owned(),
            BranchState::Untracked { note } => format!("branch:    none ({})", shown(&note)),
        });
        if let Some(path) = &record.summary_path {
            let shown_path = if record.pruned.is_some() {
                view.run_dir
                    .join(gsd_meta_manager::oneoff::KEPT_SUMMARY_FILE)
            } else {
                view.run_dir.join("work").join(path)
            };
            lines.push(format!(
                "summary:   {}",
                shown(&shown_path.display().to_string())
            ));
        }
        for warning in &record.warnings {
            lines.push(format!("warning:   {}", shown(warning)));
        }
        if let Some(meter) = live {
            lines.push(format!(
                "spend:     ≥ ${:.2} so far of ${:.2} (live estimate, lower bound; exact at the end)",
                live_spend_usd(record.spend_usd, meter),
                record.max_budget_usd
            ));
            if let Some(soft) = record.soft_cap_usd {
                lines.push(format!(
                    "soft cap:  {}",
                    shown(&soft_cap_text(soft, record.soft_cap_hit_at.as_deref()))
                ));
            }
            if let Some(rate) = &meter.rate {
                lines.push(format!("rate:      {}", shown(&rate_text(rate))));
            }
        } else if let Some(spent) = record.spend_usd {
            lines.push(format!(
                "spend:     about ${spent:.2} of ${:.2} (including any conflict resolver)",
                record.max_budget_usd
            ));
        }
        for landing in &record.landed {
            if landing.kind == LandingKind::AlreadyLanded {
                lines.push(format!(
                    "landed:    {} {} {}",
                    landing.kind.as_str(),
                    shown(&landing.at),
                    shown(&landing.detail)
                ));
                if let Some(sha) = &landing.merge_sha {
                    let tip = landing.target_tip.as_deref().map_or(String::new(), |t| {
                        format!(" (target tip {} when detected)", shown(&sha12(t)))
                    });
                    lines.push(format!(
                        "           contained by {}{tip}",
                        shown(&sha12(sha))
                    ));
                }
                continue;
            }
            let how = landing
                .how
                .map_or(String::new(), |h| format!(" [{}]", h.as_str()));
            lines.push(format!(
                "landed:    {:?}{how} {} {}",
                landing.kind,
                shown(&landing.at),
                shown(&landing.detail)
            ));
            if let Some(range) = &landing.range {
                lines.push(format!("           range {}", shown(range)));
            }
            if let Some(sha) = &landing.merge_sha {
                lines.push(format!(
                    "           merge commit {}",
                    shown(&sha[..sha.len().min(12)])
                ));
                let project = shown(&record.project.display().to_string());
                match (landing.how, &landing.range) {
                    (Some(MergeHow::Merge | MergeHow::Resolved), _) => lines.push(format!(
                        "diff:      git -C {project} diff {}^1 {}",
                        shown(sha),
                        shown(sha)
                    )),
                    (Some(MergeHow::Ff), Some(range)) => {
                        lines.push(format!("diff:      git -C {project} diff {}", shown(range)));
                    }
                    _ => {}
                }
            }
            if !landing.conflicted.is_empty() {
                lines.push(format!(
                    "           resolved by agent; conflicted: {}",
                    landing
                        .conflicted
                        .iter()
                        .map(|f| shown(f))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
            if !landing.attempts.is_empty() {
                lines.push(format!(
                    "           resolver attempts: {}",
                    landing
                        .attempts
                        .iter()
                        .map(|a| shown(a))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
        }
        if let Some(failure) = &record.land_failure {
            lines.push(format!(
                "land failed: {} ({})",
                shown(&failure.reason),
                shown(&failure.at)
            ));
            if !failure.attempts.is_empty() {
                lines.push(format!(
                    "           attempts kept: {}",
                    failure
                        .attempts
                        .iter()
                        .map(|a| shown(a))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
            }
        }
        lines
    }

    fn status_main(target: &str, run_id: Option<&str>, json: bool, config: Option<PathBuf>) -> i32 {
        let (_, project, _) = match resolve_project(target, config) {
            Ok(resolved) => resolved,
            Err(message) => {
                err_line(&format!("task status: {message}"));
                return 1;
            }
        };
        let view = match pick("status", &project, run_id, true) {
            Ok(view) => view,
            Err(code) => return code,
        };
        let live = live_meter(&view);
        if json {
            let mut value = view_json(&view);
            if let (Some(meter), Some(object)) = (&live, value.as_object_mut()) {
                object.insert(
                    "live_spend_usd".to_owned(),
                    serde_json::json!(live_spend_usd(view.record.spend_usd, meter)),
                );
                object.insert("live_spend_is_lower_bound".to_owned(), true.into());
                object.insert(
                    "rate_limit".to_owned(),
                    meter.rate.as_ref().map_or(serde_json::Value::Null, |r| {
                        serde_json::json!({
                            "status": r.status,
                            "kind": r.kind,
                            "utilization": r.utilization,
                            "resets_at": r.resets_at,
                        })
                    }),
                );
            }
            out_line(&value.to_string());
        } else {
            for line in summary_lines(&view, live.as_ref()) {
                out_line(&line);
            }
        }
        0
    }

    /// The live run's stream folded once (D-05), or `None` for a run that is not live.
    /// Bounded: the fold goes through `StreamTail`'s per-line and per-poll limits.
    fn live_meter(view: &OneoffView) -> Option<Meter> {
        view.live
            .may_be_live()
            .then(|| meter_of_file(&view.run_dir.join(STREAM_FILE)))
    }

    /// The run's spend so far: what the record already holds (a resume merge's resolver
    /// spend) plus the live estimate of the stream.
    fn live_spend_usd(recorded: Option<f64>, meter: &Meter) -> f64 {
        recorded.unwrap_or(0.0) + meter.estimate
    }

    /// One run as the JSON `status --json` and `list --json` print.
    fn view_json(view: &OneoffView) -> serde_json::Value {
        serde_json::json!({
            "record": view.record,
            "live": view.live,
            "run_dir": view.run_dir,
        })
    }

    /// One line of `task list`.
    fn list_line(view: &OneoffView) -> String {
        let record = &view.record;
        let result = match (&record.report, &record.outcome) {
            (Some(report), _) => report.status.as_str(),
            (None, Some(outcome)) => outcome.word(),
            (None, None) => "-",
        };
        let landed = record
            .landed
            .last()
            .map_or_else(|| "-".to_owned(), |l| l.kind.as_str().to_owned());
        let resumed = match (&record.resumed_by, &record.resumed_from) {
            (Some(by), _) => format!("  resumed as {}", shown(by)),
            (None, Some(from)) => format!("  resumed from {}", shown(from)),
            (None, None) => String::new(),
        };
        let clone = if record.pruned.is_some() {
            "  clone removed"
        } else {
            ""
        };
        format!(
            "{}  {:<9}  {}  {:<8}  {} commit(s)  → {}  branch {}  landed {}{resumed}{clone}",
            shown(&record.run_id),
            view.live.as_str(),
            record.mode.as_str(),
            result,
            record.commits.unwrap_or(0),
            shown(&record.land_target_text()),
            match record.branch_state() {
                BranchState::Tracked { .. } => "yes",
                BranchState::Gone { .. } => "gone",
                BranchState::Untracked { .. } => "no",
            },
            landed
        )
    }

    /// Run the cleanup scan of `project` (quick 261103-o2r, D-B) and say on stderr what it
    /// did: stdout, and `--json`, stay untouched.
    fn recycle(project: &Path, verb: &str) {
        let report = gsd_meta_manager::oneoff::recycle::recycle_project(project);
        if let Some(summary) = report.summary() {
            err_line(&format!("{verb}: {}", shown(&summary)));
        }
        for warning in &report.warnings {
            err_line(&format!("{verb}: {}", shown(warning)));
        }
    }

    fn list_main(target: &str, json: bool, config: Option<PathBuf>) -> i32 {
        let (_, project, _) = match resolve_project(target, config) {
            Ok(resolved) => resolved,
            Err(message) => {
                err_line(&format!("task list: {message}"));
                return 1;
            }
        };
        let Some(project_dir) = project_dir_for(&project) else {
            err_line("task list: no one-off directory for this project");
            return 1;
        };
        // o2r D-B: put the project's run directories in order before listing them.
        recycle(&project, "task list");
        let views = read_runs_in(&project_dir, None);
        if json {
            let all: Vec<serde_json::Value> = views.iter().map(view_json).collect();
            out_line(&serde_json::Value::Array(all).to_string());
        } else if views.is_empty() {
            out_line("task list: no one-off runs");
        } else {
            for view in &views {
                out_line(&list_line(view));
            }
        }
        0
    }

    /// `task resume` (quick 261103-klf, C-02, I-17).
    fn resume_main(
        target: &str,
        run_id: Option<&str>,
        from: Option<ResumeFrom>,
        budget: Option<f64>,
        detach: bool,
        unsandboxed: bool,
        config: Option<PathBuf>,
    ) -> i32 {
        let (alias, project, registry) = match resolve_project(target, config.clone()) {
            Ok(resolved) => resolved,
            Err(message) => {
                err_line(&format!("task resume: {message}"));
                return 1;
            }
        };
        let egress_allow = match registry
            .projects
            .get(&alias)
            .map(gsd_meta_manager::config::RegisteredProject::driver_egress_allowlist)
            .transpose()
        {
            Ok(hosts) => hosts.unwrap_or_default(),
            Err(e) => {
                err_line(&format!("task resume: projects.{alias}.{e}"));
                return 1;
            }
        };
        // Ctrl-C / TERM stops a retried landing's resolver as `task land merge` does; the
        // handlers are installed only when a landing is actually retried.
        let land_allow = egress_allow.clone();
        let land_opts = move || land::LandOptions::with_signal_handlers(land_allow.clone());
        match resume::start(
            &project,
            &alias,
            run_id,
            from,
            budget,
            unsandboxed,
            egress_allow,
            detach,
            config.as_deref(),
            &land_opts,
        ) {
            Ok(ResumeStarted::Run {
                old,
                new,
                from,
                detached,
                outcome,
            }) => {
                out_line(&format!(
                    "task resume: {old} resumed as {new} (from {}){}",
                    from.word(),
                    match (detached, outcome) {
                        (true, _) => " in the background".to_owned(),
                        (false, Some(outcome)) => format!(": finished {outcome}"),
                        (false, None) => String::new(),
                    }
                ));
                0
            }
            Ok(ResumeStarted::Landing { message, .. }) => {
                for line in message.lines() {
                    out_line(&format!("task resume: {}", shown(line)));
                }
                0
            }
            Err(message) => {
                err_line(&format!("task resume: {}", shown(&message)));
                1
            }
        }
    }

    fn stop_main(
        target: &str,
        run_id: Option<&str>,
        wait_secs: u64,
        kind: StopKind,
        which: StopTarget,
        config: Option<PathBuf>,
    ) -> i32 {
        let (_, project, _) = match resolve_project(target, config) {
            Ok(resolved) => resolved,
            Err(message) => {
                err_line(&format!("task stop: {message}"));
                return 1;
            }
        };
        // Refuse a guess before anything is signalled: `stop` resolves the same way.
        let Some(project_dir) = project_dir_for(&project) else {
            err_line("task stop: no one-off directory for this project");
            return 1;
        };
        let picked = match resolve_run(&project_dir, run_id, true) {
            Ok(view) => view,
            Err(RunPick::NoRuns) => {
                out_line("task stop: no one-off is running on this project");
                return 0;
            }
            Err(refusal) => {
                err_line(&shown(&refusal.message("task stop")));
                return 1;
            }
        };
        let run_id = picked.record.run_id.clone();
        // Say which target was acted on when it was picked automatically (C-02).
        if which == StopTarget::Auto && !picked.live.may_be_live() && picked.landing {
            out_line(&format!(
                "task stop: {run_id} is not live but its landing is running; \
                 acting on the landing (--resolver)"
            ));
        }
        let word = kind.word();
        match stop(
            &project,
            Some(&run_id),
            kind,
            StopBy::Cli,
            which,
            Duration::from_secs(wait_secs),
        ) {
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
            Ok(outcome @ StopOutcome::PauseDelivered { .. }) => {
                out_line(&format!("task stop: {}", shown(&outcome.message())));
                0
            }
            Ok(outcome @ StopOutcome::PauseRefused { .. }) => {
                err_line(&format!("task stop: {}", shown(&outcome.message())));
                1
            }
            Ok(outcome @ StopOutcome::PauseUnconfirmed { .. }) => {
                err_line(&format!(
                    "task stop: {} (after {wait_secs}s)",
                    shown(&outcome.message())
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
            Ok(
                outcome @ (StopOutcome::LandingStopped { .. }
                | StopOutcome::LandingRequested { .. }),
            ) => {
                out_line(&format!(
                    "task stop: stopped the landing ({word}): {}",
                    outcome.message()
                ));
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

    fn send_main(
        target: &str,
        text: &str,
        run_id: Option<&str>,
        wait_secs: u64,
        config: Option<PathBuf>,
    ) -> i32 {
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
        let view = match pick("send", &project, run_id, true) {
            Ok(view) => view,
            Err(code) => return code,
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

    fn budget_main(
        target: &str,
        soft: Option<f64>,
        run_id: Option<&str>,
        wait_secs: u64,
        config: Option<PathBuf>,
    ) -> i32 {
        let (alias, project, _) = match resolve_project(target, config) {
            Ok(resolved) => resolved,
            Err(message) => {
                err_line(&format!("task budget: {message}"));
                return 1;
            }
        };
        let not_live = || {
            err_line(&format!(
                "task budget: {}'s one-off is not live",
                shown(&alias)
            ));
            1
        };
        let view = match pick("budget", &project, run_id, true) {
            Ok(view) => view,
            Err(code) => return code,
        };
        if view.live != LiveState::Running {
            return not_live();
        }
        if let Err(why) = validate_soft_cap(soft, view.record.max_budget_usd) {
            err_line(&format!("task budget: {why}"));
            return 1;
        }
        match set_soft_cap(&view.run_dir, soft) {
            Ok(id) => report_request("budget", &view.run_dir, &id, wait_secs),
            Err(InboxError::NotLive) => not_live(),
            Err(e) => {
                err_line(&format!("task budget: {}", shown(&e.to_string())));
                1
            }
        }
    }

    /// Follow a run's stream as plain text; each stdin line is an injected message.
    fn attach_main(
        target: &str,
        run_id: Option<&str>,
        no_input: bool,
        config: Option<PathBuf>,
    ) -> i32 {
        let (alias, project, _) = match resolve_project(target, config) {
            Ok(resolved) => resolved,
            Err(message) => {
                err_line(&format!("task attach: {message}"));
                return 1;
            }
        };
        let Some(project_dir) = project_dir_for(&project) else {
            err_line(&format!(
                "task attach: no one-off run for {}",
                shown(&alias)
            ));
            return 1;
        };
        let mut view = match resolve_run(&project_dir, run_id, true) {
            Ok(view) => view,
            Err(refusal) => {
                err_line(&shown(&refusal.message("task attach")));
                return 1;
            }
        };
        // Follow the picked run, not whichever is newest by then.
        let followed = view.record.run_id.clone();
        let read = || resolve_run(&project_dir, Some(&followed), false).ok();
        let run_dir = view.run_dir.clone();
        if !no_input {
            let dir = run_dir.clone();
            std::thread::spawn(move || {
                use std::io::BufRead;
                for line in std::io::stdin().lock().lines() {
                    let Ok(line) = line else { break };
                    let text = line.trim();
                    if text.is_empty() {
                        continue;
                    }
                    if let Err(e) = send_message(&dir, text) {
                        err_line(&format!("task attach: not sent: {}", shown(&e.to_string())));
                    }
                }
            });
        }
        let mut follower = Follower::new(&run_dir);
        let mut tick: u64 = 0;
        let meter = loop {
            let finished_before = !view.live.may_be_live();
            let batch = match follower.poll() {
                Ok(batch) => batch,
                Err(e) => {
                    err_line(&format!("task attach: {}", shown(&e.to_string())));
                    return 1;
                }
            };
            let drained = batch.lines.is_empty();
            for line in &batch.lines {
                out_line(&shown(line));
            }
            if finished_before && drained {
                break batch.meter;
            }
            tick += 1;
            if tick & 3 == 0 || !drained {
                if let Some(fresh) = read() {
                    view = fresh;
                }
            }
            if drained {
                std::thread::sleep(Duration::from_millis(250));
            }
        };
        out_line(&shown(&format!(
            "attach: {} {} - {}",
            view.record.run_id,
            view.live.as_str(),
            meter_text(
                &meter,
                view.record.max_budget_usd,
                view.record.soft_cap_usd,
                view.record.soft_cap_hit_at.as_deref(),
                view.live.may_be_live() || !meter.exact,
            )
        )));
        0
    }

    fn land_main(action: LandAction, config: Option<PathBuf>) -> i32 {
        let (verb, target, run_id) = match &action {
            LandAction::Merge {
                project, run_id, ..
            } => ("merge", project, run_id),
            LandAction::Push { project, run_id } => ("push", project, run_id),
            LandAction::Discard { project, run_id } => ("discard", project, run_id),
        };
        let (alias, project, registry) = match resolve_project(target, config) {
            Ok(resolved) => resolved,
            Err(message) => {
                err_line(&format!("task land {verb}: {message}"));
                return 1;
            }
        };
        let result = match action {
            LandAction::Merge { ref into, .. } => {
                // I-8: the project's `driver.egress` allowlist, for a conflict resolver
                // session, resolved as `task run` does.
                let egress_allow = match registry
                    .projects
                    .get(&alias)
                    .map(gsd_meta_manager::config::RegisteredProject::driver_egress_allowlist)
                    .transpose()
                {
                    Ok(hosts) => hosts.unwrap_or_default(),
                    Err(e) => {
                        err_line(&format!("task land merge: projects.{alias}.{e}"));
                        return 1;
                    }
                };
                // I-12: Ctrl-C / TERM stops a resolver the way a run stop does.
                match land::LandOptions::with_signal_handlers(egress_allow) {
                    Ok(mut opts) => {
                        opts.into = into.clone();
                        land::merge(&project, run_id, &opts)
                    }
                    Err(e) => Err(e),
                }
            }
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
