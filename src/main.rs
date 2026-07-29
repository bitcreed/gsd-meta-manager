mod event;
mod tui;

use gsd_meta_manager::app::App;
use gsd_meta_manager::watcher::FileWatcher;
use clap::Parser;
use gsd_meta_manager::cli::{Cli, Commands};
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
        Some(Commands::Drive {
            alias,
            command,
            run_id,
            dry_run,
            goal,
            claude_program,
            claude_args,
        }) => {
            // This arm is by construction BEFORE `tui::init()` in the `None`
            // arm below, which is the whole of "the driver never touches
            // ratatui" — no new mechanism, just the position in this match.
            let config = load_config(&config_path)?;
            let args = DriveArgs {
                alias,
                command,
                run_id,
                dry_run,
                goal,
                claude_program,
                claude_args,
            };
            if let Err(err) = drive(args, &config).await {
                // The `Add` arm's house shape: user-facing refusals in this
                // binary print and exit, they do not bubble as an anyhow chain.
                eprintln!("Error: {}", err);
                std::process::exit(1);
            }
        }
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
