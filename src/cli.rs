use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(name = "gsd-manager", about = "TUI command center for GSD projects")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Path to config file (overrides default location)
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Add a GSD project to the registry
    Add {
        /// Alias for the project (displayed in TUI)
        alias: String,
        /// Path to the project root (must contain .planning/)
        path: PathBuf,
    },
    /// Remove a project from the registry
    Remove {
        /// Alias of the project to remove
        alias: String,
    },
    /// List all registered projects
    List,
}
