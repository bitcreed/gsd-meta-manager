use clap::{Parser, Subcommand};
use std::ffi::OsString;
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
    // Every `///` below is rendered verbatim by clap as help text, so the
    // rationale for each field lives in `//` comments like this one and only the
    // one-line description a user needs is a doc comment.
    //
    // The variant carries no `#[cfg]` on purpose (D-05): driving is Unix-only,
    // but the subcommand parses on every platform and the *handler* returns a
    // typed unsupported-platform error. An accepted limitation that surfaces as
    // "unknown subcommand" is indistinguishable from a bug.
    /// Run one GSD command against a project that has opted in to being driven
    Drive {
        /// Alias of the project to drive; it must carry a driver opt-in record
        alias: String,
        // Exactly one command. The decision router is Phase 20's, so there is
        // deliberately no way to pass a sequence.
        /// The single GSD command to run, e.g. `/gsd-progress`
        #[arg(long)]
        command: String,
        // The TUI supplies it so it knows what to look for afterwards (D-03).
        //
        // It is **not** generated when absent, and this comment used to say it
        // was. A comment that describes a mode the code refuses is the next
        // reader's bug: a driver-generated id appears on no caller's argv, which
        // is exactly what made a run invisible to `driver::liveness::probe` —
        // reported crashed by every scan, and un-stoppable because a stop
        // answered already-gone without signalling (CR-04). `driver::drive`
        // refuses a real run with no id before anything is created.
        // It must also be a **single plain path component**, and that refusal
        // likewise lives in `driver::drive` rather than in a `value_parser`
        // here: the id reaches the run body from a hand-typed invocation, from
        // the TUI's argv builder, and from `writer::read_active_run` — a
        // validator wired to this one flag would guard the one path that is
        // already the least interesting (D-27, WR-02).
        /// Run id to record this run under; required unless `--dry-run`
        #[arg(long)]
        run_id: Option<String>,
        /// Report what the run would do without executing anything
        #[arg(long)]
        dry_run: bool,
        // Interpreting a goal is Phase 21's; this phase only records it.
        /// Free-text goal recorded into the run record, never interpreted
        #[arg(long)]
        goal: Option<String>,
        /// Test and development only: the program to spawn instead of `claude`
        ///
        /// It exists because the driver is a separate process that constructs
        /// its own executor, so `ClaudeExecutor::with_program` does not reach
        /// it. The TUI's own spawn argv never emits this flag. A hidden flag
        /// rather than an environment variable is deliberate: an env var is
        /// inherited by children, so a stray `GSD_*` in the user's shell would
        /// silently reach a TUI-spawned driver — the same class of leak
        /// `src/executor/claude.rs:333-352` already scrubs `CLAUDE*` for —
        /// whereas a flag must be passed on purpose by a caller whose argv
        /// builder is itself unit-tested.
        #[arg(long, hide = true)]
        claude_program: Option<PathBuf>,
        /// Test and development only: leading arguments for `--claude-program`
        ///
        /// Repeatable. Placed before the executor's own generated argv, which is
        /// how the checked-in shell stand-ins receive their transcript and exit
        /// code. Hidden for the same inheritance reason as `--claude-program`.
        #[arg(long, hide = true)]
        claude_args: Vec<OsString>,
    },
}
