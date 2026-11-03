//! One-off sandboxed task dispatch (quick task 261103-f5y).
//!
//! A one-off is ONE headless agent session that runs `/gsd-quick <task>` or
//! `/gsd-fast <task>` on a registered project, in a private clone, inside the
//! launcher's sandbox, and reports back through the strict `taskctl report` CLI.
//! The facts this module is built on:
//!
//! 1. **No driver (C-01).** One session whose prompt is the GSD command plus the
//!    reporting instruction ([`prompt`]); no DIRECTION.md, LOG.md, master prompt, gap
//!    rules or hook gate. The caps are a wall clock and the agent's own spend ceiling.
//! 2. **Launcher pieces, composed here (C-02).** The sandbox argv builders, the host
//!    probe, the token, the config seeding, egress, the process seam, stop and push
//!    are reused from `crate::launcher`; `launcher::run::launch` is never called (it is
//!    bound to the driver's run store).
//! 3. **One runtime seam (C-03, C-10).** Every agent-specific fact (argv, env, config
//!    seeding, credential, egress upstream, stream parsing, capability flags, models)
//!    lives in [`runtime`]. Every other file here is runtime-neutral, which
//!    `tests/oneoff_runtime_seam_guard.rs` enforces.
//! 4. **Synchronous std threads, every spawn through `launcher::process` (C-04).**
//!
//! # The trust split
//!
//! The agent writes exactly two things: its clone (`<run-dir>/work`) and
//! `<run-dir>/report/report.json`. The host writes `run.json`, `stream.jsonl` and
//! `launch.log`, none of which is mounted into the sandbox. The host never trusts
//! `report.json`: it reads it capped, refuses a symlink and re-validates it with the
//! CLI's own validator ([`report::read_validated`]). The commit range, the commit
//! count and the SUMMARY path are derived on the host from git, never from the report.
//!
//! The operator writes only `<run-dir>/inbox/` (0700, host-only, never mounted); the
//! host writes `injections.jsonl`, the record of every message it delivered or refused.
//! The agent reads its prompt and every injected message as stream-json lines on its
//! stdin ([`session`], [`inbox`]).

use serde::{Deserialize, Serialize};

pub mod inbox;
pub mod prompt;
pub mod report;
pub mod runtime;
pub mod session;
pub mod store;
pub mod stream;
pub mod todos;

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub mod land;
#[cfg(any(target_os = "linux", target_os = "macos"))]
pub mod launch;

/// The environment variable that names the report directory inside the sandbox.
pub const REPORT_DIR_ENV: &str = "GSDMM_ONEOFF_REPORT_DIR";
/// The agent's report, inside [`REPORT_SUBDIR`].
pub const REPORT_FILE: &str = "report.json";
/// The host's record of the run.
pub const RUN_FILE: &str = "run.json";
/// The agent's raw stream-json output, as the redacting capture pump wrote it.
pub const STREAM_FILE: &str = "stream.jsonl";
/// The detached daemon's own stdout and stderr.
pub const LAUNCH_LOG: &str = "launch.log";
/// The private clone the agent works in.
pub const WORK_DIR: &str = "work";
/// The one directory of the run dir the sandbox may write besides the clone.
pub const REPORT_SUBDIR: &str = "report";
/// The per-project lock, held by the daemon for the run's life.
pub const LOCK_FILE: &str = "oneoff.lock";
/// The branch prefix of the clone's work branch.
pub const BRANCH_PREFIX: &str = "gsd-oneoff/";
/// The host-only directory operator requests are put in (never mounted).
pub const INBOX_DIR: &str = "inbox";
/// The host's record of every injection, delivered or refused.
pub const INJECTIONS_FILE: &str = "injections.jsonl";
/// The most injections the launcher accepts per run.
pub const MAX_INJECTIONS: usize = 200;
/// The longest injected message, in characters.
pub const MAX_MESSAGE_CHARS: usize = 4000;
/// The prefix of every one-off run id.
pub const RUN_ID_PREFIX: &str = "oneoff-";

/// Which GSD command the session runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, clap::ValueEnum)]
#[serde(rename_all = "lowercase")]
pub enum Mode {
    /// `/gsd-quick`: planned, atomic commits, a SUMMARY.md
    Quick,
    /// `/gsd-fast`: inline, no SUMMARY.md
    Fast,
}

impl Mode {
    /// The slash command the prompt starts with.
    pub fn command(self) -> &'static str {
        match self {
            Mode::Quick => "/gsd-quick",
            Mode::Fast => "/gsd-fast",
        }
    }

    /// The CLI word (`--mode quick|fast`).
    pub fn as_str(self) -> &'static str {
        match self {
            Mode::Quick => "quick",
            Mode::Fast => "fast",
        }
    }
}

/// What the session works on.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", content = "spec", rename_all = "snake_case")]
pub enum TaskSpec {
    /// A pending todo, by file name under `.planning/todos/pending/`.
    Todo { file_name: String, title: String },
    /// A typed task, verbatim.
    Text(String),
}

/// How the agent session ended, as the runtime seam reads it (C-10, C-14).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum AgentOutcome {
    /// The session ended with a successful result.
    Finished,
    /// The session failed; `detail` is host-written, never stream text.
    Failed { detail: String },
    /// The agent's quota was exhausted.
    RateLimited { detail: String },
    /// The agent hit its own spend ceiling.
    OverBudget,
    /// The host's wall clock stopped the session.
    TimedOut,
    /// The user stopped the session.
    Stopped,
}

impl AgentOutcome {
    /// The one word `task status` and the TUI show.
    pub fn word(&self) -> &'static str {
        match self {
            AgentOutcome::Finished => "finished",
            AgentOutcome::Failed { .. } => "failed",
            AgentOutcome::RateLimited { .. } => "rate_limited",
            AgentOutcome::OverBudget => "over_budget",
            AgentOutcome::TimedOut => "timed_out",
            AgentOutcome::Stopped => "stopped",
        }
    }
}

/// A landing or control verb the TUI asks for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OneoffCommand {
    Merge,
    Push,
    Discard,
    Stop,
}

impl OneoffCommand {
    /// The verb as the user reads it.
    pub fn verb(self) -> &'static str {
        match self {
            OneoffCommand::Merge => "merge",
            OneoffCommand::Push => "push",
            OneoffCommand::Discard => "discard",
            OneoffCommand::Stop => "stop",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn modes_name_their_commands() {
        assert_eq!(Mode::Quick.command(), "/gsd-quick");
        assert_eq!(Mode::Fast.command(), "/gsd-fast");
    }

    #[test]
    fn task_specs_and_outcomes_round_trip_through_json() {
        for spec in [
            TaskSpec::Todo {
                file_name: "a.md".to_owned(),
                title: "A".to_owned(),
            },
            TaskSpec::Text("fix x".to_owned()),
        ] {
            let text = serde_json::to_string(&spec).unwrap();
            assert_eq!(serde_json::from_str::<TaskSpec>(&text).unwrap(), spec);
        }
        for outcome in [
            AgentOutcome::Finished,
            AgentOutcome::Failed {
                detail: "x".to_owned(),
            },
            AgentOutcome::RateLimited {
                detail: "y".to_owned(),
            },
            AgentOutcome::OverBudget,
            AgentOutcome::TimedOut,
            AgentOutcome::Stopped,
        ] {
            let text = serde_json::to_string(&outcome).unwrap();
            assert!(text.contains(outcome.word()), "{text}");
            assert_eq!(
                serde_json::from_str::<AgentOutcome>(&text).unwrap(),
                outcome
            );
        }
    }
}
