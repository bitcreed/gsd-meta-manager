mod action;
mod cli;
mod config;
mod error;
mod registry;
mod state_reader;

use clap::Parser;
use cli::{Cli, Commands};
use config::{load_config, save_config, Config};
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
            println!("TUI mode not yet implemented. Use --help for CLI commands.");
        }
    }

    Ok(())
}
