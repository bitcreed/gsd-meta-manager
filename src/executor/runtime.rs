//! Which agent CLI a project is configured for.
//!
//! Two runtimes are named: Claude Code and OpenAI Codex. Since Phase 31 this
//! module selects nothing to drive: the in-process executors that consumed the
//! value went with the legacy `drive` path, and the sandboxed launcher runs
//! Claude only. [`AgentRuntime`] stays because it is a parsed config value: the
//! per-project `runtime` key and the `default_runtime` preference still load,
//! and a config naming `"codex"` is still accepted rather than refused.

use serde_json::Value;

/// An agent CLI a project's config can name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AgentRuntime {
    /// Claude Code. The default, and the only runtime before 260922-hdj.
    #[default]
    Claude,
    /// OpenAI Codex. A parsed config value only: nothing in this build drives it.
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
/// Raised while the manager config is read, so a typo costs nothing. The
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
        assert_eq!(
            resolve(None, Some(AgentRuntime::Codex)),
            AgentRuntime::Codex
        );
        assert_eq!(
            resolve(Some(AgentRuntime::Claude), Some(AgentRuntime::Codex)),
            AgentRuntime::Claude
        );
    }
}
