use clap::{Parser, Subcommand};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "gsd-meta-manager",
    version,
    about = "TUI command center for GSD projects"
)]
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
        /// Path to the project root (must contain .planning/)
        path: PathBuf,
        /// Optional alias (defaults to last folder component of path)
        alias: Option<String>,
    },
    /// Remove a project from the registry
    Remove {
        /// Alias of the project to remove
        alias: String,
    },
    /// List all registered projects
    List,
}
