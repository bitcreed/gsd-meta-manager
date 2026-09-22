//! Which agent CLI drives a project, and the one enum the driver programs
//! against.
//!
//! Two runtimes exist: Claude Code (`claude -p` over duplex `stream-json`) and
//! OpenAI Codex (`codex exec --json`, one prompt, one turn per process). They
//! are not peers in capability, and nothing here pretends they are. Codex has
//! no stdin message protocol, no interrupt control channel, no capability
//! announcement and no USD cost (measured against codex-cli 0.155.1; see the
//! 260922-hdj research), so [`AgentExecutor::Codex`] satisfies `start`,
//! `cancel` and `is_running` and refuses the rest with a typed error.
//!
//! **Claude is the default and stays the default.** A project runs under Codex
//! only when the manager's own config names it (see [`resolve`]).

use serde_json::Value;

/// An agent CLI this manager can drive.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AgentRuntime {
    /// Claude Code. The default, and the only runtime before 260922-hdj.
    #[default]
    Claude,
    /// OpenAI Codex, driven through `codex exec --json`.
    Codex,
}

impl AgentRuntime {
    /// The config value and journal word for this runtime.
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
        }
    }

    /// The program found on `PATH` when no override is in force.
    ///
    /// Equal to [`Self::as_str`] today; kept as its own accessor because the
    /// two answer different questions (what the config says, what gets
    /// exec'd), and the argv digest reads this one.
    pub fn program(self) -> &'static str {
        match self {
            Self::Claude => "claude",
            Self::Codex => "codex",
        }
    }

    /// Read a runtime from a manager config value.
    ///
    /// **Exact lowercase strings only.** `"Codex"`, `" codex"`, a number or an
    /// object are all refused rather than guessed at: a value that selects
    /// which agent binary is executed is not a place for a lenient parse
    /// (T-hdj-07). `key` names the config key the value came from, so the
    /// refusal can say where to look.
    pub fn from_config_value(
        key: &'static str,
        value: &Value,
    ) -> Result<AgentRuntime, UnrecognizedRuntime> {
        match value.as_str() {
            Some("claude") => Ok(Self::Claude),
            Some("codex") => Ok(Self::Codex),
            Some(other) => Err(UnrecognizedRuntime {
                key,
                value: crate::text::Untrusted::from_untrusted_source(other.to_string()),
            }),
            None => Err(UnrecognizedRuntime {
                key,
                value: crate::text::Untrusted::from_untrusted_source(value.to_string()),
            }),
        }
    }
}

/// A runtime value in the manager config that names no runtime this build
/// knows.
///
/// Raised by `drive` before anything is created, so a typo costs nothing. The
/// value is carried as [`crate::text::Untrusted`] because it is user-authored
/// config text echoed back to a terminal.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnrecognizedRuntime {
    /// Which config key carried the value: `runtime` on a project entry, or
    /// `default_runtime` under `preferences`.
    pub key: &'static str,
    /// The value as found (a non-string is rendered as compact JSON).
    pub value: crate::text::Untrusted,
}

impl std::fmt::Display for UnrecognizedRuntime {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "the manager config key `{}` holds {:?}, which is not a known agent runtime. \
             Known values are \"claude\" and \"codex\" (exact, lowercase)",
            self.key,
            self.value.shown().to_string()
        )
    }
}

impl std::error::Error for UnrecognizedRuntime {}

/// The runtime a project is driven by: the project entry's own `runtime`, else
/// the global `default_runtime` preference, else Claude.
///
/// **GSD's own runtime keys are deliberately never consulted** — neither the
/// project's `.planning/config.json` `runtime` nor `~/.gsd/defaults.json`
/// (260922-hdj ID-2). On a machine whose GSD defaults say `codex`, adopting
/// them would silently move every Claude-driven project onto Codex, which is
/// exactly the unannounced behaviour change the manager must not make. A user
/// who wants Codex says so in the manager's config.
pub fn resolve(project: Option<AgentRuntime>, global: Option<AgentRuntime>) -> AgentRuntime {
    project.or(global).unwrap_or_default()
}

/// The command prefixes recognised as a GSD command, matched ASCII
/// case-insensitively. `$gsd-` is Codex's own skill-mention form, so an
/// already-translated command is normalised rather than double-prefixed.
const GSD_COMMAND_PREFIXES: [&str; 4] = ["/gsd-", "/gsd:", "gsd:", "$gsd-"];

/// Translate a canonical GSD command into Codex's skill-mention form.
///
/// `/gsd-execute-phase 3` becomes `$gsd-execute-phase 3`. Under Codex, GSD
/// commands are skills invoked by *mentioning* `$gsd-<name>`, and a live
/// `codex exec` resolved that form (research, "GSD skills under Codex").
///
/// Mirrors GSD's own `formatGsdSlash`: only the command token is lowercased and
/// the argument tail is kept byte for byte. **Unlike it, only prefixed input is
/// translated** — a bare word passes through verbatim, because this function's
/// input is a prompt and a goal sentence is not a command name (ID-7). A
/// prefix with no command token after it is also returned verbatim.
///
/// Called at the executor boundary and nowhere else: the driver, the bounds,
/// the router and the journal all compare canonical `/gsd-…` strings.
pub fn codex_command(canonical: &str) -> String {
    for prefix in GSD_COMMAND_PREFIXES {
        let Some(head) = canonical.get(..prefix.len()) else {
            continue;
        };
        if !head.eq_ignore_ascii_case(prefix) {
            continue;
        }
        let rest = &canonical[prefix.len()..];
        let split = rest.find(char::is_whitespace).unwrap_or(rest.len());
        let (token, tail) = rest.split_at(split);
        if token.is_empty() {
            return canonical.to_string();
        }
        return format!("$gsd-{}{tail}", token.to_lowercase());
    }
    canonical.to_string()
}

#[cfg(unix)]
pub use self::unix::AgentExecutor;

#[cfg(unix)]
mod unix {
    use std::ffi::OsString;
    use std::path::PathBuf;

    use tokio::sync::{mpsc, oneshot};

    use super::AgentRuntime;
    use crate::error::{SendError, SpawnError};
    use crate::executor::claude::ClaudeExecutor;
    use crate::executor::codex::CodexExecutor;
    use crate::executor::stream_json::UserMessage;
    use crate::executor::{
        BoxFuture, DrivableProject, ExecutionHandle, ExecutionOptions, Executor, InterruptAck,
        RunOutcome,
    };

    /// The executor the driver holds: one variant per runtime.
    ///
    /// An enum rather than a `Box<dyn Executor>` because the driver also needs
    /// the concrete builders (`observing_spawn`, `observing_replay_echoes`),
    /// which are deliberately not on the portable trait.
    #[derive(Debug, Clone)]
    pub enum AgentExecutor {
        /// Claude Code over duplex `stream-json`.
        Claude(ClaudeExecutor),
        /// Codex over `codex exec --json`.
        Codex(CodexExecutor),
    }

    impl AgentExecutor {
        /// Drive the runtime's default program found on `PATH`.
        pub fn new(runtime: AgentRuntime) -> Self {
            match runtime {
                AgentRuntime::Claude => Self::Claude(ClaudeExecutor::new()),
                AgentRuntime::Codex => Self::Codex(CodexExecutor::new()),
            }
        }

        /// Drive a stand-in program with fixed leading arguments — the test
        /// and debug-override path, exactly as each backend's own
        /// `with_program` is.
        pub fn with_program(
            runtime: AgentRuntime,
            program: impl Into<PathBuf>,
            leading_args: Vec<OsString>,
        ) -> Self {
            match runtime {
                AgentRuntime::Claude => {
                    Self::Claude(ClaudeExecutor::with_program(program, leading_args))
                }
                AgentRuntime::Codex => {
                    Self::Codex(CodexExecutor::with_program(program, leading_args))
                }
            }
        }

        /// Publish the agent's process group id on `tx` the instant the child
        /// exists. Both backends spawn a process-group leader, so both honour
        /// this.
        pub fn observing_spawn(self, tx: oneshot::Sender<u32>) -> Self {
            match self {
                Self::Claude(executor) => Self::Claude(executor.observing_spawn(tx)),
                Self::Codex(executor) => Self::Codex(executor.observing_spawn(tx)),
            }
        }

        /// Publish the raw line of every replay echo on `tx`.
        ///
        /// Codex has no replay echo, so its arm drops the sender: the driver's
        /// receiver then sees a closed channel, which it already treats as
        /// "no more echoes".
        pub fn observing_replay_echoes(self, tx: mpsc::UnboundedSender<String>) -> Self {
            match self {
                Self::Claude(executor) => Self::Claude(executor.observing_replay_echoes(tx)),
                Self::Codex(executor) => {
                    drop(tx);
                    Self::Codex(executor)
                }
            }
        }

        /// Which runtime this executor drives.
        pub fn runtime(&self) -> AgentRuntime {
            match self {
                Self::Claude(_) => AgentRuntime::Claude,
                Self::Codex(_) => AgentRuntime::Codex,
            }
        }
    }

    impl Executor for AgentExecutor {
        fn start<'a>(
            &'a self,
            project: &'a DrivableProject,
            command: String,
            options: ExecutionOptions,
        ) -> BoxFuture<'a, Result<ExecutionHandle, SpawnError>> {
            match self {
                Self::Claude(executor) => executor.start(project, command, options),
                Self::Codex(executor) => executor.start(project, command, options),
            }
        }

        fn send<'a>(
            &'a self,
            handle: &'a mut ExecutionHandle,
            message: UserMessage,
        ) -> BoxFuture<'a, Result<(), SendError>> {
            match self {
                Self::Claude(executor) => executor.send(handle, message),
                Self::Codex(executor) => executor.send(handle, message),
            }
        }

        fn interrupt<'a>(
            &'a self,
            handle: &'a mut ExecutionHandle,
        ) -> BoxFuture<'a, Result<InterruptAck, SendError>> {
            match self {
                Self::Claude(executor) => executor.interrupt(handle),
                Self::Codex(executor) => executor.interrupt(handle),
            }
        }

        fn cancel<'a>(&'a self, handle: &'a mut ExecutionHandle) -> BoxFuture<'a, RunOutcome> {
            match self {
                Self::Claude(executor) => executor.cancel(handle),
                Self::Codex(executor) => executor.cancel(handle),
            }
        }

        fn is_running(&self, handle: &ExecutionHandle) -> bool {
            match self {
                Self::Claude(executor) => executor.is_running(handle),
                Self::Codex(executor) => executor.is_running(handle),
            }
        }

        fn capabilities<'a>(&self, handle: &'a ExecutionHandle) -> &'a [String] {
            match self {
                Self::Claude(executor) => executor.capabilities(handle),
                Self::Codex(executor) => executor.capabilities(handle),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_exact_lowercase_values_name_a_runtime() {
        assert_eq!(
            AgentRuntime::from_config_value("runtime", &Value::from("claude")),
            Ok(AgentRuntime::Claude)
        );
        assert_eq!(
            AgentRuntime::from_config_value("runtime", &Value::from("codex")),
            Ok(AgentRuntime::Codex)
        );
        for refused in [
            Value::from("Codex"),
            Value::from(" codex"),
            Value::from("gemini"),
            Value::from(1),
            Value::Null,
        ] {
            let err = AgentRuntime::from_config_value("runtime", &refused)
                .expect_err("anything but an exact lowercase name is refused");
            assert_eq!(err.key, "runtime");
            assert!(err.to_string().contains("`runtime`"), "{err}");
        }
    }

    #[test]
    fn nothing_configured_resolves_to_claude_and_the_entry_outranks_the_preference() {
        assert_eq!(resolve(None, None), AgentRuntime::Claude);
        assert_eq!(resolve(None, Some(AgentRuntime::Codex)), AgentRuntime::Codex);
        assert_eq!(
            resolve(Some(AgentRuntime::Claude), Some(AgentRuntime::Codex)),
            AgentRuntime::Claude
        );
    }

    #[test]
    fn a_gsd_command_becomes_a_codex_skill_mention() {
        for (canonical, expected) in [
            ("/gsd-progress", "$gsd-progress"),
            ("/GSD:Plan-Phase 3 --X", "$gsd-plan-phase 3 --X"),
            ("gsd:execute-phase 4", "$gsd-execute-phase 4"),
            ("$gsd-progress", "$gsd-progress"),
            ("hello world", "hello world"),
            ("/gsd-", "/gsd-"),
            ("/gsd- tail", "/gsd- tail"),
            ("", ""),
            ("/g", "/g"),
        ] {
            assert_eq!(codex_command(canonical), expected, "{canonical:?}");
        }
    }

    #[test]
    fn the_argument_tail_is_kept_byte_for_byte() {
        assert_eq!(
            codex_command("/gsd-plan-phase  Ünïcode\targ \"Q\""),
            "$gsd-plan-phase  Ünïcode\targ \"Q\""
        );
    }
}
