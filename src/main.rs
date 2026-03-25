mod action;
mod app;
mod cli;
mod config;
mod error;
mod event;
mod registry;
mod state_reader;
mod tui;
mod ui;
mod watcher;

use app::App;
use watcher::FileWatcher;
use clap::Parser;
use cli::{Cli, Commands};
use config::{load_config, save_config, Config};
use event::EventBus;
use registry::{add_project, list_projects, remove_project};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    color_eyre::install().map_err(|e| anyhow::anyhow!("{}", e))?;

    let cli = Cli::parse();
    let config_path = cli.config.unwrap_or_else(Config::default_path);

    match cli.command {
        Some(Commands::Add { alias, path }) => {
            let canonical_path = path.canonicalize().unwrap_or(path);
            let mut config = load_config(&config_path)?;
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
                println!("{:<20} {:<50} {}", "ALIAS", "PATH", "ADDED");
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
        None => {
            // TUI mode
            let mut terminal = tui::init();
            let mut app = App::new(config_path)?;
            app.load_project_states();

            let event_bus = EventBus::new();

            // Initialize file watcher for all registered projects
            let mut _watcher = FileWatcher::new(event_bus.tx.clone())?;
            for project in app.config.projects.values() {
                let planning_dir = project.path.join(".planning");
                if planning_dir.is_dir() {
                    if let Err(e) = _watcher.watch(&planning_dir) {
                        tracing::warn!(
                            "Could not watch {}: {}",
                            planning_dir.display(),
                            e
                        );
                    }
                }
            }

            event_bus.spawn_crossterm_reader();
            event_bus.spawn_tick(250);

            let mut rx = event_bus.rx;

            let result = run_tui_loop(&mut terminal, &mut app, &mut rx).await;

            tui::restore();

            result?;
        }
    }

    Ok(())
}

async fn run_tui_loop(
    terminal: &mut ratatui::DefaultTerminal,
    app: &mut App,
    rx: &mut tokio::sync::mpsc::UnboundedReceiver<action::Action>,
) -> anyhow::Result<()> {
    loop {
        if app.needs_redraw {
            terminal.draw(|frame| ui::render(frame, app))?;
            app.needs_redraw = false;
        }

        if let Some(action) = rx.recv().await {
            app.update(action);
        }

        if app.should_quit {
            break;
        }
    }

    Ok(())
}
