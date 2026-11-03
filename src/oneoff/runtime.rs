//! The agent runtime seam (C-10): the ONLY one-off module that names the agent.
//!
//! `AgentRuntime` has one variant, Claude Code. Every agent-specific fact a one-off
//! needs is a method here: the argv, the environment, the config-dir seeding, the
//! sandbox posture decision (the version check), the credential, the egress token and
//! upstream, the stream-json reading into the common [`AgentOutcome`], the capability
//! flags and the models. Runtime-neutral code obtains the runtime only through
//! [`AgentRuntime::default`] and never spells a variant name or a Claude path;
//! `tests/oneoff_runtime_seam_guard.rs` enforces that. There is no config surface.

use std::ffi::OsString;

/// The agent CLI a one-off runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AgentRuntime {
    /// Claude Code (`claude -p`, stream-json).
    #[default]
    Claude,
}

/// What a runtime supports.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuntimeCaps {
    /// It enforces a spend ceiling itself (`--max-budget-usd`).
    pub budget_cap: bool,
    /// It runs user hooks from its settings.
    pub hooks: bool,
}

/// How the host ended the session, which outranks whatever the stream says.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HostEnd {
    /// The session exited by itself.
    Exited,
    /// The user stopped it.
    UserStop,
    /// The host's wall clock stopped it.
    WallClock,
}

/// The models the TUI cycles through, first is the default.
const CLAUDE_MODELS: &[&str] = &["opus", "sonnet"];

impl AgentRuntime {
    /// The runtime's name, as `run.json` records it.
    pub fn name(self) -> &'static str {
        match self {
            AgentRuntime::Claude => "claude",
        }
    }

    pub fn caps(self) -> RuntimeCaps {
        match self {
            AgentRuntime::Claude => RuntimeCaps {
                budget_cap: true,
                hooks: true,
            },
        }
    }

    /// The agent's per-project config directory, relative to the project root, with a
    /// trailing `/`. A merge that changes anything under it is called out (I-10).
    pub fn project_config_dir(self) -> &'static str {
        match self {
            AgentRuntime::Claude => ".claude/",
        }
    }

    /// The models a user may pick, the default first.
    pub fn models(self) -> &'static [&'static str] {
        match self {
            AgentRuntime::Claude => CLAUDE_MODELS,
        }
    }

    /// The agent's command line: headless, stream-json, the prompt last after `--`.
    pub fn argv(self, prompt: &str, model: &str, max_budget_usd: f64) -> Vec<OsString> {
        match self {
            AgentRuntime::Claude => {
                let mut argv = crate::launcher::sandbox::claude_argv(prompt, model, max_budget_usd);
                if !self.caps().budget_cap {
                    if let Some(i) = argv.iter().position(|a| a == "--max-budget-usd") {
                        argv.drain(i..i + 2);
                    }
                }
                argv
            }
        }
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod host_side {
    use std::ffi::OsString;
    use std::io;
    use std::path::{Path, PathBuf};

    use chrono::{DateTime, Utc};
    use serde_json::Value;

    use super::{AgentRuntime, HostEnd};
    use crate::driver::rate_limit::{self, QuotaVerdict, QuotaWindow};
    use crate::launcher::auth::{resolve_launch_token, LaunchToken, TokenSource};
    use crate::launcher::config_dir::{seed_oneoff, OneoffSeedSpec, HOST_MANAGED_DIR};
    use crate::launcher::egress::upstream::Upstream;
    use crate::launcher::egress::{TokenProvider, INFERENCE_HOST};
    use crate::launcher::host::{self, HostCaps, Posture};
    use crate::launcher::outcome::is_auth_failure;
    use crate::launcher::run::summarize_stream_file;
    use crate::launcher::sandbox::{common_env_sets, egress_env_sets, BRIDGE_PORT};
    use crate::launcher::LauncherError;
    use crate::oneoff::AgentOutcome;

    /// A result subtype as recorded: a short `[a-z_]` word, else `other` (the stream is
    /// agent-written).
    fn subtype_word(result: Option<&Value>) -> String {
        match result
            .and_then(|r| r.get("subtype"))
            .and_then(Value::as_str)
        {
            Some(word)
                if !word.is_empty()
                    && word.len() <= 40
                    && word.chars().all(|c| c.is_ascii_lowercase() || c == '_') =>
            {
                word.to_owned()
            }
            Some(_) => "other".to_owned(),
            None => "none".to_owned(),
        }
    }

    impl AgentRuntime {
        /// How the session ended: the host end first, then the stream at
        /// `stream_path` (read bounded per line), then the exit code.
        pub fn outcome(self, stream_path: &Path, rc: Option<i32>, end: HostEnd) -> AgentOutcome {
            self.outcome_at(stream_path, rc, end, Utc::now())
        }

        /// [`outcome`](Self::outcome) with the clock given.
        pub fn outcome_at(
            self,
            stream_path: &Path,
            rc: Option<i32>,
            end: HostEnd,
            now: DateTime<Utc>,
        ) -> AgentOutcome {
            match end {
                HostEnd::UserStop => return AgentOutcome::Stopped,
                HostEnd::WallClock => return AgentOutcome::TimedOut,
                HostEnd::Exited => {}
            }
            let summary = summarize_stream_file(stream_path, now);
            if is_auth_failure(summary.result.as_ref()) {
                return AgentOutcome::Failed {
                    detail: "auth: the agent's token was rejected".to_owned(),
                };
            }
            match &summary.rate_limit {
                QuotaVerdict::Rejected { window, resets_at } => {
                    return AgentOutcome::RateLimited {
                        detail: rate_limit::park_detail(window, *resets_at),
                    }
                }
                QuotaVerdict::Allowed => {
                    if rate_limit::terminal_reason_names_a_rate_limit(
                        summary.terminal_reason.as_deref(),
                    ) {
                        return AgentOutcome::RateLimited {
                            detail: rate_limit::park_detail(&QuotaWindow::Unknown(None), None),
                        };
                    }
                }
            }
            let subtype = subtype_word(summary.result.as_ref());
            if subtype == "error_max_budget_usd" {
                return AgentOutcome::OverBudget;
            }
            if summary.result_ok() {
                return AgentOutcome::Finished;
            }
            let code = rc.map_or_else(|| "none".to_owned(), |rc| rc.to_string());
            AgentOutcome::Failed {
                detail: format!("the agent exited with code {code} (result: {subtype})"),
            }
        }

        /// The sandbox posture for this host, with the runtime's own version check.
        pub fn decide_posture(
            self,
            caps: &HostCaps,
            unsandboxed: bool,
        ) -> Result<Posture, LauncherError> {
            host::decide(caps, unsandboxed)
        }

        /// The credential for one run.
        pub fn credential(self, now: DateTime<Utc>) -> Result<LaunchToken, LauncherError> {
            resolve_launch_token(now).map_err(|e| LauncherError::Auth {
                detail: e.to_string(),
            })
        }

        /// What the launcher-side egress server injects.
        pub fn egress_token(self, credential: &LaunchToken, home: &Path) -> TokenProvider {
            match credential.source {
                TokenSource::Stored => TokenProvider::Static(credential.value.clone()),
                TokenSource::Host => TokenProvider::Host {
                    claude_dir: home.join(".claude"),
                    initial: credential.value.clone(),
                },
            }
        }

        /// Where inference requests go.
        pub fn upstream(self) -> Upstream {
            Upstream::Tls {
                host: INFERENCE_HOST.to_owned(),
                port: 443,
            }
        }

        /// The per-run config dir inside `run_dir`.
        pub fn config_dir_in(self, run_dir: &Path) -> PathBuf {
            run_dir.join("claude-config")
        }

        /// Seed the per-run config dir (no gate, no driverctl shim; a `taskctl`-only
        /// shim with `report_dir` pinned).
        pub fn seed_config(
            self,
            config_dir: &Path,
            home: &Path,
            binary: &Path,
            report_dir: &Path,
            managed: bool,
            keep_roots: &[PathBuf],
        ) -> io::Result<()> {
            let keep_roots = if self.caps().hooks {
                keep_roots.to_vec()
            } else {
                Vec::new()
            };
            seed_oneoff(&OneoffSeedSpec {
                config_dir: config_dir.to_owned(),
                host_claude_dir: home.join(".claude"),
                binary: binary.to_owned(),
                report_dir: report_dir.to_owned(),
                managed,
                host_managed_dir: PathBuf::from(HOST_MANAGED_DIR),
                keep_roots,
            })
        }

        /// The runtime's environment sets for the child: its config dir, the common
        /// sets, then the bridge (sandboxed) or the credential itself (degraded).
        pub fn env_sets(
            self,
            config_dir: &Path,
            posture: &Posture,
            credential: &LaunchToken,
        ) -> Vec<(&'static str, OsString)> {
            let mut sets: Vec<(&'static str, OsString)> =
                vec![("CLAUDE_CONFIG_DIR", config_dir.as_os_str().to_owned())];
            sets.extend(common_env_sets());
            match posture {
                Posture::BwrapScope => sets.extend(egress_env_sets(BRIDGE_PORT)),
                Posture::Degraded { .. } => {
                    sets.push(("CLAUDE_CODE_OAUTH_TOKEN", credential.value.clone().into()));
                }
            }
            sets
        }
    }

    #[cfg(test)]
    mod tests {
        use super::*;
        use tempfile::TempDir;

        fn outcome_of(lines: &[&str], rc: Option<i32>, end: HostEnd) -> AgentOutcome {
            let dir = TempDir::new().unwrap();
            let path = dir.path().join("stream.jsonl");
            std::fs::write(&path, lines.join("\n")).unwrap();
            AgentRuntime::default().outcome_at(
                &path,
                rc,
                end,
                DateTime::from_timestamp(1_800_000_000, 0).unwrap(),
            )
        }

        #[test]
        fn the_stream_maps_to_the_common_outcome() {
            assert_eq!(
                outcome_of(
                    &[r#"{"type":"result","subtype":"success","is_error":false}"#],
                    Some(0),
                    HostEnd::Exited
                ),
                AgentOutcome::Finished
            );
            assert_eq!(
                outcome_of(
                    &[r#"{"type":"result","subtype":"error_max_budget_usd"}"#],
                    Some(1),
                    HostEnd::Exited
                ),
                AgentOutcome::OverBudget
            );
            assert!(matches!(
                outcome_of(
                    &[
                        r#"{"type":"rate_limit_event","rate_limit_info":{"status":"rejected","rateLimitType":"five_hour","resetsAt":1800003600}}"#,
                        r#"{"type":"result","subtype":"error_during_execution","is_error":true}"#
                    ],
                    Some(1),
                    HostEnd::Exited
                ),
                AgentOutcome::RateLimited { .. }
            ));
            match outcome_of(
                &[
                    r#"{"type":"result","subtype":"success","is_error":true,"result":"Failed to authenticate. API Error: 401"}"#,
                ],
                Some(1),
                HostEnd::Exited,
            ) {
                AgentOutcome::Failed { detail } => assert!(detail.contains("auth"), "{detail}"),
                other => panic!("{other:?}"),
            }
            match outcome_of(&["not json"], Some(1), HostEnd::Exited) {
                AgentOutcome::Failed { detail } => {
                    assert!(detail.contains("code 1"), "{detail}");
                    assert!(detail.contains("result: none"), "{detail}");
                }
                other => panic!("{other:?}"),
            }
        }

        #[test]
        fn the_host_end_outranks_the_stream() {
            let success = [r#"{"type":"result","subtype":"success"}"#];
            assert_eq!(
                outcome_of(&success, Some(0), HostEnd::WallClock),
                AgentOutcome::TimedOut
            );
            assert_eq!(
                outcome_of(&success, Some(0), HostEnd::UserStop),
                AgentOutcome::Stopped
            );
        }

        #[test]
        fn a_hostile_subtype_is_not_copied() {
            match outcome_of(
                &[r#"{"type":"result","subtype":"\u001b[31mevil"}"#],
                Some(2),
                HostEnd::Exited,
            ) {
                AgentOutcome::Failed { detail } => assert!(detail.ends_with("(result: other)")),
                other => panic!("{other:?}"),
            }
        }

        #[test]
        fn env_sets_point_at_the_config_dir_and_never_carry_the_token_when_sandboxed() {
            let credential = LaunchToken {
                value: "secret-token".to_owned(),
                source: TokenSource::Stored,
                expires_at: None,
            };
            let rt = AgentRuntime::default();
            let sets = rt.env_sets(Path::new("/cfg"), &Posture::BwrapScope, &credential);
            assert_eq!(sets[0].0, "CLAUDE_CONFIG_DIR");
            assert!(!sets.iter().any(|(_, v)| v == "secret-token"));
            assert!(!sets.iter().any(|(k, _)| *k == "DRIVERCTL_SOCK"));
            let degraded = rt.env_sets(
                Path::new("/cfg"),
                &Posture::Degraded { missing: vec![] },
                &credential,
            );
            assert!(degraded.iter().any(|(_, v)| v == "secret-token"));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_argv_is_the_launchers_headless_command() {
        let rt = AgentRuntime::default();
        assert_eq!(
            rt.argv("/gsd-quick x", "opus", 10.0),
            crate::launcher::sandbox::claude_argv("/gsd-quick x", "opus", 10.0)
        );
    }

    #[test]
    fn caps_models_and_name() {
        let rt = AgentRuntime::default();
        assert_eq!(
            rt.caps(),
            RuntimeCaps {
                budget_cap: true,
                hooks: true
            }
        );
        assert_eq!(rt.models(), ["opus", "sonnet"]);
        assert_eq!(rt.name(), "claude");
    }
}
