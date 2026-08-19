mod event;
mod tui;

use gsd_meta_manager::app::App;
use gsd_meta_manager::watcher::FileWatcher;
use clap::Parser;
use gsd_meta_manager::cli::{Cli, Commands, EnvelopeAction};
use gsd_meta_manager::config::{load_config, save_config, Config};
use gsd_meta_manager::driver::{drive, DriveArgs};
use event::EventBus;
use gsd_meta_manager::main_loop::{
    pump, ExecEvent, PumpOutcome, EXEC_BATCH, EXEC_CHANNEL_CAPACITY,
};
use gsd_meta_manager::registry::{add_project, list_projects, remove_project};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    // Initialize tracing subscriber with file appender before anything else
    let log_dir = dirs::data_local_dir()
        .map(|d| d.join("gsd-meta-manager"))
        .unwrap_or_else(|| std::path::PathBuf::from("/tmp"));
    let file_appender = tracing_appender::rolling::daily(&log_dir, "gsd-meta-manager.log");
    tracing_subscriber::fmt()
        .with_writer(file_appender)
        .with_ansi(false)
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    color_eyre::install().map_err(|e| anyhow::anyhow!("{}", e))?;

    let cli = Cli::parse();
    let config_path = cli.config.unwrap_or_else(Config::default_path);

    match cli.command {
        Some(Commands::Add { path, alias }) => {
            let canonical_path = path.canonicalize().unwrap_or(path);
            let alias = alias.unwrap_or_else(|| {
                canonical_path
                    .file_name()
                    .and_then(|n| n.to_str())
                    .unwrap_or("unnamed")
                    .to_string()
            });
            let config = load_config(&config_path)?;
            if config.projects.contains_key(&alias) {
                eprintln!(
                    "Error: alias '{}' already exists. Provide an explicit alias: gsd-manager add {} <alias>",
                    alias,
                    canonical_path.display()
                );
                std::process::exit(1);
            }
            let mut config = config;
            add_project(&mut config, &alias, &canonical_path)?;
            save_config(&config, &config_path)?;
            println!("Added project '{}' at {}", alias, canonical_path.display());
        }
        Some(Commands::Remove { alias }) => {
            let mut config = load_config(&config_path)?;
            remove_project(&mut config, &alias)?;
            save_config(&config, &config_path)?;
            println!("Removed project '{}'", alias);
        }
        Some(Commands::List) => {
            let config = load_config(&config_path)?;
            let projects = list_projects(&config);
            if projects.is_empty() {
                println!("No projects registered.");
            } else {
                println!("{:<20} {:<50} ADDED", "ALIAS", "PATH");
                println!("{}", "-".repeat(90));
                for (alias, project) in projects {
                    println!(
                        "{:<20} {:<50} {}",
                        alias,
                        project.path.display(),
                        project.added
                    );
                }
            }
        }
        // The two `#[cfg(debug_assertions)]` fields below are D-30/WR-16: the
        // agent-override flags have no parser entry in a release build, so this
        // arm must not name them there either. The attribute rides the pattern
        // field and the struct-expression field rather than forking this arm in
        // two — one arm that goes out of sync with the other is exactly the bug
        // a cfg is supposed to prevent.
        Some(Commands::Drive {
            alias,
            command,
            target_phase,
            max_steps,
            wall_clock_cap_secs,
            run_id,
            dry_run,
            goal,
            #[cfg(debug_assertions)]
            claude_program,
            #[cfg(debug_assertions)]
            claude_args,
        }) => {
            // This arm is by construction BEFORE `tui::init()` in the `None`
            // arm below, which is the whole of "the driver never touches
            // ratatui" — no new mechanism, just the position in this match.
            let config = load_config(&config_path)?;
            let args = DriveArgs {
                alias,
                command,
                target_phase,
                max_steps,
                wall_clock_cap_secs,
                run_id,
                dry_run,
                goal,
                #[cfg(debug_assertions)]
                claude_program,
                #[cfg(debug_assertions)]
                claude_args,
            };
            if let Err(err) = drive(args, &config).await {
                // The `Add` arm's house shape: user-facing refusals in this
                // binary print and exit, they do not bubble as an anyhow chain.
                eprintln!("Error: {}", err);
                std::process::exit(1);
            }
        }
        // Like the `Drive` arm above, this one is BEFORE `tui::init()` by
        // construction — and here the position is doing more work than there. A
        // git hook's stdout and stderr belong to the pushing git process; a
        // terminal put into raw mode by ratatui on the way past would corrupt
        // the very output that carries the refusal.
        Some(Commands::Envelope { action }) => match action {
            EnvelopeAction::PrePush { alias, hook_path } => {
                let stdin = std::io::stdin();
                // git runs a hook with the working directory at the top of the
                // worktree, which is what makes the scan's root and the path
                // checks' root the same root without a flag to get wrong.
                let repo_root = match std::env::current_dir() {
                    Ok(dir) => dir,
                    Err(err) => {
                        eprintln!("Error: cannot resolve the repository root: {err}");
                        std::process::exit(1);
                    }
                };
                match gsd_meta_manager::envelope::hooks::pre_push(
                    &alias,
                    stdin.lock(),
                    &hook_path,
                    &repo_root,
                ) {
                    // The exit code IS the control (D-25): git blocks the push
                    // on any non-zero exit, and nothing downstream has to parse
                    // a message to learn that it was blocked.
                    Ok(code) => std::process::exit(code),
                    Err(err) => {
                        // The `Add` arm's house shape, and fail-closed: a hook
                        // that could not do its job must not let the push
                        // through.
                        eprintln!("Error: {}", err);
                        std::process::exit(1);
                    }
                }
            }
            EnvelopeAction::PreCommit { alias, hook_path } => {
                let repo_root = match std::env::current_dir() {
                    Ok(dir) => dir,
                    Err(err) => {
                        eprintln!("Error: cannot resolve the repository root: {err}");
                        std::process::exit(1);
                    }
                };
                match gsd_meta_manager::envelope::hooks::pre_commit(
                    &alias,
                    &hook_path,
                    &repo_root,
                ) {
                    Ok(code) => std::process::exit(code),
                    Err(err) => {
                        // Fail-closed, exactly as the push hook does: a hook
                        // that could not do its job must not let the commit
                        // through.
                        eprintln!("Error: {}", err);
                        std::process::exit(1);
                    }
                }
            }
            EnvelopeAction::Askpass {
                alias,
                host,
                prompt,
            } => {
                // The credential goes to stdout and the refusal goes to stderr,
                // both inside the handler — nothing is printed here, because a
                // second print site is a second place a secret could be echoed.
                match gsd_meta_manager::envelope::cred::askpass_with_config(
                    &config_path,
                    &alias,
                    &prompt,
                    &host,
                ) {
                    Ok(code) => std::process::exit(code),
                    Err(err) => {
                        // Fail-closed, and redacted: an askpass that cannot do
                        // its job emits no credential, and its diagnostic passes
                        // through the already-shipped redaction path rather than
                        // a forked one.
                        eprintln!(
                            "Error: {}",
                            gsd_meta_manager::journal::redact::redact(&err.to_string())
                        );
                        std::process::exit(1);
                    }
                }
            }
            EnvelopeAction::Guard { alias } => {
                // Before `tui::init()` for the same reason the hook arms are:
                // this process's stdout carries the permission decision the
                // agent CLI reads, and a terminal put into raw mode on the way
                // past would corrupt the very answer that does the blocking.
                let stdin = std::io::stdin();
                match gsd_meta_manager::envelope::hooks::guard(&alias, stdin.lock()) {
                    // The exit code is the second carrier (2 blocks); the JSON
                    // on stdout is the first. Neither depends on the other.
                    Ok(code) => std::process::exit(code),
                    Err(err) => {
                        // Fail-closed and redacted: a guard that cannot do its
                        // job denies, and its diagnostic passes through the
                        // already-shipped redaction path rather than a forked
                        // one — the request it could not judge is a command
                        // line, which is a thing that can carry a token.
                        eprintln!(
                            "Error: {}",
                            gsd_meta_manager::journal::redact::redact(&err.to_string())
                        );
                        std::process::exit(2);
                    }
                }
            }
            EnvelopeAction::Scan { alias, root } => {
                if !gsd_meta_manager::journal::is_plain_path_component(&alias) {
                    eprintln!(
                        "Error: alias {alias:?} is not a plain path component, so no \
                         envelope sanctions a scan for it"
                    );
                    std::process::exit(1);
                }
                let report = gsd_meta_manager::envelope::scan::scan_with_external(
                    &root,
                    gsd_meta_manager::envelope::scan::ScanLimits::default(),
                );
                // Stdout, because this entry point is read by a human. The
                // report is safe to print by construction: it carries file,
                // line and rule, and has nowhere to hold a matched secret.
                println!("alias={alias} root={}", root.display());
                print!("{}", report.render());
                // The exit code IS the control (D-25), here as much as in the
                // hook: a finding exits non-zero.
                std::process::exit(if report.is_clean() { 0 } else { 1 });
            }
        },
        None => {
            // TUI mode
            let mut terminal = tui::init();
            let mut app = App::new(config_path)?;
            app.load_project_states();

            app.init_change_tracker();

            let event_bus = EventBus::new();

            // The executor event channel is created ONCE for the process
            // lifetime, is SEPARATE from the Action FIFO, and is BOUNDED
            // (D-17). Each of those three properties guards a distinct failure:
            //
            // * once-for-the-lifetime + the sender clone stashed below means it
            //   never closes, so its `select!` arm can never permanently
            //   disable itself between runs (Pitfall C, T-15-27);
            // * separate means bulk stream traffic cannot starve control keys
            //   behind the biased-first Action arm (TRANS-03, T-15-25);
            // * bounded means a render loop parked in the blocking editor
            //   shell-out applies backpressure to the reader instead of growing
            //   without limit (T-15-26).
            let (exec_tx, mut exec_rx) =
                tokio::sync::mpsc::channel::<ExecEvent>(EXEC_CHANNEL_CAPACITY);

            // Store event_tx on App context so creation flow can send actions back
            app.ctx.event_tx = Some(event_bus.tx.clone());
            app.ctx.exec_tx = Some(exec_tx.clone());

            // Initialize file watcher for all registered projects
            let mut watcher = FileWatcher::new(event_bus.tx.clone())?;
            for project in app.ctx.config.projects.values() {
                let planning_dir = project.path.join(".planning");
                if planning_dir.is_dir() {
                    if let Err(e) = watcher.watch(&planning_dir) {
                        tracing::warn!(
                            "Could not watch {}: {}",
                            planning_dir.display(),
                            e
                        );
                    }
                }
            }

            // Store watcher on App context so new projects can be watched dynamically
            app.ctx.watcher = Some(watcher);

            // Startup scan: auto-register any active Claude sessions whose
            // working_dir is an unregistered GSD project. The session poll
            // tick handles the same logic ongoing (every ~5s), but this
            // closes the gap between launch and the first poll.
            let initial_sessions = gsd_meta_manager::session_detector::detect_sessions();
            app.active_sessions = initial_sessions.clone();
            app.ctx.active_sessions = initial_sessions;
            app.auto_register_new_sessions();

            // Startup scan, second half: find driver runs that outlived a
            // previous TUI session. The 20-tick block handles the same logic
            // ongoing (every ~5s), but this closes the gap between launch and
            // the first poll — which is the whole of "reopening the TUI shows
            // that run still live" (CTRL-04, D-13).
            //
            // Synchronous for the same reason the session scan above is: the TUI
            // is not yet in its loop, so there is no render thread to block.
            //
            // The scan writes nothing (D-12). A run whose driver died without an
            // `ended_at` comes back with `live == false` and its record is left
            // exactly as the dead driver left it.
            app.ctx.observed_runs =
                gsd_meta_manager::driver::reconcile::reconcile_all(&app.ctx.config.projects)
                    .into_iter()
                    .map(|run| (run.alias.clone(), run))
                    .collect();
            // Its sibling, read at the same moment and for the same reason
            // (WR-02): D-14's finished-run arm has to be answerable on the very
            // first frame, or a project whose last run failed shows no badge
            // until the first 20-tick scan lands five seconds later.
            app.ctx.last_outcomes =
                gsd_meta_manager::driver::reconcile::last_ended_outcomes(&app.ctx.config.projects);

            event_bus.spawn_crossterm_reader();
            // Keep the 250ms tick. It drives the 20-tick session poll and the
            // 3s status-message expiry; `pump`'s 16ms redraw interval is a
            // separate concern and purely additive. Removing this would
            // silently break session detection.
            event_bus.spawn_tick(250);

            let mut rx = event_bus.rx;

            let result = run_tui_loop(&mut terminal, &mut app, &mut rx, &mut exec_rx).await;

            tui::restore();

            // Held to here on purpose: this binding plus the clone on
            // `app.ctx` are what keep the executor channel open for the whole
            // process lifetime.
            drop(exec_tx);

            result?;
        }
    }

    Ok(())
}

/// Draw, [`pump`], the editor shell-out, the quit check.
///
/// Everything that used to be a bare `rx.recv().await` here now lives in
/// `gsd_meta_manager::main_loop::pump`, in the **library**, because this file is
/// the binary and nothing in it is reachable from a `cargo test --lib` target.
/// That placement is what makes TRANS-03 testable at all.
async fn run_tui_loop(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut App,
    rx: &mut tokio::sync::mpsc::UnboundedReceiver<gsd_meta_manager::action::Action>,
    exec_rx: &mut tokio::sync::mpsc::Receiver<ExecEvent>,
) -> anyhow::Result<()> {
    loop {
        // Sync needs_redraw from ctx (screens set ctx.needs_redraw)
        if app.ctx.needs_redraw {
            app.needs_redraw = true;
            app.ctx.needs_redraw = false;
        }

        if app.needs_redraw {
            terminal.draw(|frame| gsd_meta_manager::ui::render(frame, app))?;
            app.needs_redraw = false;
        }

        if pump(app, rx, exec_rx, EXEC_BATCH).await == PumpOutcome::Idle {
            // Every arm disabled: both channels are closed, so no further
            // message can ever arrive and continuing would spin. Unreachable
            // while the redraw timer arm exists, but breaking is the only
            // non-spinning response if that ever changes.
            tracing::warn!("event loop: all message sources closed, exiting");
            break;
        }

        // Check if a screen action requested an editor launch
        if let Some(path) = app.pending_editor.take() {
            // Suspend the TUI
            ratatui::restore();

            // Determine editor: $VISUAL > $EDITOR > vi
            let editor = std::env::var("VISUAL")
                .or_else(|_| std::env::var("EDITOR"))
                .unwrap_or_else(|_| "vi".to_string());

            // Spawn editor and wait for it to exit
            let status = std::process::Command::new(&editor).arg(&path).status();

            match status {
                Ok(s) if s.success() => {
                    app.ctx.status_message = Some((
                        format!(
                            "Editor closed: {}",
                            path.file_name()
                                .unwrap_or_default()
                                .to_string_lossy()
                        ),
                        std::time::Instant::now(),
                    ));
                }
                Ok(s) => {
                    app.ctx.status_message = Some((
                        format!("Editor exited with: {}", s),
                        std::time::Instant::now(),
                    ));
                }
                Err(e) => {
                    app.ctx.status_message = Some((
                        format!("Failed to launch {}: {}", editor, e),
                        std::time::Instant::now(),
                    ));
                }
            }

            // Re-initialize TUI
            *terminal = ratatui::init();
            app.needs_redraw = true;
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}
