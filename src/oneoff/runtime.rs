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

use serde::Serialize;
use serde_json::Value;

use super::stream::{StreamEvent, UsageTokens};

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

    /// The agent's command line: headless, stream-json on both ends, replaying the
    /// user messages it reads. There is NO positional prompt and no `--`: the prompt
    /// is the first message on stdin ([`user_message_line`](Self::user_message_line)).
    /// The driver keeps `launcher::sandbox::claude_argv`, untouched (C-03).
    pub fn argv(self, model: &str, max_budget_usd: f64) -> Vec<OsString> {
        match self {
            AgentRuntime::Claude => {
                let mut argv: Vec<OsString> = ["claude", "-p", "--setting-sources", "user"]
                    .iter()
                    .map(OsString::from)
                    .collect();
                argv.extend(["--model", model].iter().map(OsString::from));
                argv.extend(
                    [
                        "--input-format",
                        "stream-json",
                        "--output-format",
                        "stream-json",
                        "--verbose",
                        "--replay-user-messages",
                        "--forward-subagent-text",
                        "--permission-mode",
                        "bypassPermissions",
                    ]
                    .iter()
                    .map(OsString::from),
                );
                if self.caps().budget_cap {
                    argv.push("--max-budget-usd".into());
                    argv.push(max_budget_usd.to_string().into());
                }
                argv
            }
        }
    }

    /// One user message as the agent reads it on stdin: a single JSON line ending in
    /// a newline (C-04). Serialized from typed structs so the key order is pinned.
    pub fn user_message_line(self, text: &str) -> String {
        #[derive(Serialize)]
        struct Block<'a> {
            r#type: &'static str,
            text: &'a str,
        }
        #[derive(Serialize)]
        struct Message<'a> {
            role: &'static str,
            content: [Block<'a>; 1],
        }
        #[derive(Serialize)]
        struct Line<'a> {
            r#type: &'static str,
            message: Message<'a>,
            parent_tool_use_id: Option<&'static str>,
        }
        let line = Line {
            r#type: "user",
            message: Message {
                role: "user",
                content: [Block {
                    r#type: "text",
                    text,
                }],
            },
            parent_tool_use_id: None,
        };
        let mut rendered = serde_json::to_string(&line).unwrap_or_default();
        rendered.push('\n');
        rendered
    }

    /// The wrap-up message the launcher injects when a soft spend cap is reached.
    pub fn wrap_up_message(self) -> &'static str {
        match self {
            AgentRuntime::Claude => WRAP_UP_MESSAGE,
        }
    }
}

/// Notional USD per million tokens for one model family.
#[derive(Debug, Clone, Copy, PartialEq)]
struct Prices {
    input: f64,
    output: f64,
    cache_write: f64,
    cache_read: f64,
}

const fn prices(input: f64, output: f64, cache_write: f64, cache_read: f64) -> Prices {
    Prices {
        input,
        output,
        cache_write,
        cache_read,
    }
}

/// Notional first-party list prices in USD per MTok, from the claude-api skill's model
/// table (cached 2026-10-06). Cache write is 1.25 x input. Subscription tokens are not
/// billed per token, so every figure derived from this table is notional. The longest
/// model-id prefix wins.
const PRICES: &[(&str, Prices)] = &[
    ("claude-fable-5-1", prices(10.0, 50.0, 12.5, 0.25)),
    ("claude-fable-5", prices(10.0, 50.0, 12.5, 1.0)),
    ("claude-opus-5-5", prices(4.0, 20.0, 5.0, 0.20)),
    ("claude-opus-5", prices(5.0, 25.0, 6.25, 0.50)),
    ("claude-opus-4-8", prices(5.0, 25.0, 6.25, 0.50)),
    ("claude-opus-4-7", prices(5.0, 25.0, 6.25, 0.50)),
    ("claude-opus-4-6", prices(5.0, 25.0, 6.25, 0.50)),
    ("claude-sonnet-5-5", prices(2.0, 10.0, 2.5, 0.20)),
    ("claude-sonnet-5", prices(2.0, 10.0, 2.5, 0.20)),
    ("claude-sonnet-4-6", prices(3.0, 15.0, 3.75, 0.30)),
    ("claude-haiku-5-5", prices(0.10, 0.50, 0.125, 0.01)),
    ("claude-haiku-4-5", prices(1.0, 5.0, 1.25, 0.10)),
];

/// Family fallbacks for an id no prefix matches.
const FAMILY_FALLBACK: &[(&str, &str)] = &[
    ("fable", "claude-fable-5-1"),
    ("opus", "claude-opus-5-5"),
    ("sonnet", "claude-sonnet-5-5"),
    ("haiku", "claude-haiku-5-5"),
];

/// The price of the unknown.
const DEFAULT_PRICES: Prices = prices(5.0, 25.0, 6.25, 0.50);

fn prices_for(model: &str) -> Prices {
    let by_prefix = PRICES
        .iter()
        .filter(|(prefix, _)| model.starts_with(prefix))
        .max_by_key(|(prefix, _)| prefix.len());
    if let Some((_, found)) = by_prefix {
        return *found;
    }
    for (family, row) in FAMILY_FALLBACK {
        if model.contains(family) {
            if let Some((_, found)) = PRICES.iter().find(|(prefix, _)| prefix == row) {
                return *found;
            }
        }
    }
    DEFAULT_PRICES
}

impl AgentRuntime {
    /// The notional dollar cost of `tokens` on `model` (C-15, I-8).
    #[allow(clippy::cast_precision_loss)]
    pub fn notional_usd(self, model: &str, tokens: &UsageTokens) -> f64 {
        match self {
            AgentRuntime::Claude => {
                let p = prices_for(model);
                (tokens.input as f64 * p.input
                    + tokens.output as f64 * p.output
                    + tokens.cache_creation as f64 * p.cache_write
                    + tokens.cache_read as f64 * p.cache_read)
                    / 1_000_000.0
            }
        }
    }
}

/// The one message the launcher injects at the soft spend cap (C-18).
const WRAP_UP_MESSAGE: &str = "Budget reached: commit your work now, call `gsd-meta-manager taskctl report` with your status, then stop.";

/// The longest text a classified event keeps (characters).
const EVENT_TEXT_MAX: usize = 32 * 1024;
/// The longest one-line string a classified event keeps (characters).
const EVENT_LINE_MAX: usize = 200;
/// The longest tool argument summary (characters).
const TOOL_SUMMARY_MAX: usize = 100;

/// `text` on one line: control characters become spaces, cut to `max` characters.
fn one_line(text: &str, max: usize) -> String {
    text.chars()
        .map(|c| if c.is_control() { ' ' } else { c })
        .take(max)
        .collect()
}

/// `text` cut to `max` characters, newlines and tabs kept, other control characters
/// replaced.
fn bounded_text(text: &str, max: usize) -> String {
    text.chars()
        .map(|c| {
            if c.is_control() && c != '\n' && c != '\t' {
                ' '
            } else {
                c
            }
        })
        .take(max)
        .collect()
}

/// A short `[a-z_]` word, else `other` (agent-written text is never copied).
fn lower_word(value: Option<&Value>) -> Option<String> {
    let word = value?.as_str()?;
    Some(
        if !word.is_empty()
            && word.len() <= 40
            && word.chars().all(|c| c.is_ascii_lowercase() || c == '_')
        {
            word.to_owned()
        } else {
            "other".to_owned()
        },
    )
}

/// The tool's argument that says what it is doing, on one line (C-08).
fn summarize_tool(name: &str, input: &Value) -> String {
    let key = match name {
        "Bash" => Some("command"),
        "Read" | "Edit" | "Write" | "NotebookEdit" => Some("file_path"),
        "Grep" | "Glob" => Some("pattern"),
        "Task" | "Agent" => Some("description"),
        "Skill" => Some("skill"),
        _ => None,
    };
    let chosen = key
        .and_then(|k| input.get(k))
        .and_then(Value::as_str)
        .or_else(|| {
            input
                .as_object()?
                .values()
                .filter_map(Value::as_str)
                .find(|s| s.chars().count() <= 200)
        });
    chosen.map_or_else(String::new, |s| one_line(s, TOOL_SUMMARY_MAX))
}

/// The text of a message `content` that is a string or an array of text blocks.
fn content_text(content: &Value) -> Option<String> {
    match content {
        Value::String(s) => Some(s.clone()),
        Value::Array(blocks) => {
            let parts: Vec<&str> = blocks
                .iter()
                .filter(|b| b.get("type").and_then(Value::as_str) == Some("text"))
                .filter_map(|b| b.get("text").and_then(Value::as_str))
                .collect();
            (!parts.is_empty()).then(|| parts.join("\n"))
        }
        _ => None,
    }
}

impl AgentRuntime {
    /// Read one `stream.jsonl` line into runtime-neutral events (C-08, I-1, I-2).
    /// One line can carry several content blocks plus a `usage`. A line that is not
    /// JSON, or that this reader does not know, is `[Other]`. Thinking blocks are not
    /// kept.
    pub fn classify(self, line: &str) -> Vec<StreamEvent> {
        match self {
            AgentRuntime::Claude => classify_claude(line),
        }
    }
}

fn classify_claude(line: &str) -> Vec<StreamEvent> {
    let Ok(value) = serde_json::from_str::<Value>(line) else {
        return vec![StreamEvent::Other];
    };
    let sub = value
        .get("parent_tool_use_id")
        .is_some_and(|p| !p.is_null());
    let kind = value.get("type").and_then(Value::as_str).unwrap_or("");
    match kind {
        "assistant" => {
            let Some(message) = value.get("message") else {
                return vec![StreamEvent::Other];
            };
            let mut events = Vec::new();
            if let Some(blocks) = message.get("content").and_then(Value::as_array) {
                for block in blocks {
                    match block.get("type").and_then(Value::as_str) {
                        Some("text") => {
                            if let Some(text) = block.get("text").and_then(Value::as_str) {
                                events.push(StreamEvent::AssistantText {
                                    sub,
                                    text: bounded_text(text, EVENT_TEXT_MAX),
                                });
                            }
                        }
                        Some("tool_use") => {
                            let name = one_line(
                                block.get("name").and_then(Value::as_str).unwrap_or("tool"),
                                40,
                            );
                            let input = block.get("input").unwrap_or(&Value::Null);
                            events.push(StreamEvent::ToolCall {
                                sub,
                                summary: summarize_tool(&name, input),
                                name,
                            });
                        }
                        _ => {}
                    }
                }
            }
            if let (Some(id), Some(usage)) = (
                message.get("id").and_then(Value::as_str),
                message.get("usage").filter(|u| u.is_object()),
            ) {
                let n = |key: &str| usage.get(key).and_then(Value::as_u64).unwrap_or(0);
                events.push(StreamEvent::Usage {
                    sub,
                    message_id: one_line(id, 80),
                    model: one_line(
                        message.get("model").and_then(Value::as_str).unwrap_or(""),
                        80,
                    ),
                    tokens: UsageTokens {
                        input: n("input_tokens"),
                        output: n("output_tokens"),
                        cache_creation: n("cache_creation_input_tokens"),
                        cache_read: n("cache_read_input_tokens"),
                    },
                });
            }
            if events.is_empty() {
                events.push(StreamEvent::Other);
            }
            events
        }
        "user" => {
            let Some(content) = value.get("message").and_then(|m| m.get("content")) else {
                return vec![StreamEvent::Other];
            };
            let replay = value.get("isReplay").and_then(Value::as_bool) == Some(true);
            if replay && !sub {
                return match content_text(content) {
                    Some(text) => vec![StreamEvent::UserReplay {
                        text: bounded_text(&text, EVENT_TEXT_MAX),
                    }],
                    None => vec![StreamEvent::Other],
                };
            }
            let mut events = Vec::new();
            if let Some(blocks) = content.as_array() {
                for block in blocks {
                    if block.get("type").and_then(Value::as_str) != Some("tool_result") {
                        continue;
                    }
                    let text = block
                        .get("content")
                        .and_then(content_text)
                        .unwrap_or_default();
                    events.push(StreamEvent::ToolResult {
                        sub,
                        is_error: block.get("is_error").and_then(Value::as_bool) == Some(true),
                        first_line: one_line(text.lines().next().unwrap_or(""), EVENT_LINE_MAX),
                        lines: text.lines().count(),
                    });
                }
            }
            if events.is_empty() {
                events.push(StreamEvent::Other);
            }
            events
        }
        "result" => vec![StreamEvent::TurnResult {
            subtype: lower_word(value.get("subtype")).unwrap_or_else(|| "none".to_owned()),
            cost_usd: value
                .get("total_cost_usd")
                .and_then(Value::as_f64)
                .filter(|c| c.is_finite() && *c >= 0.0),
            turns: value
                .get("num_turns")
                .and_then(Value::as_u64)
                .and_then(|n| u32::try_from(n).ok()),
            is_error: value.get("is_error").and_then(Value::as_bool) == Some(true),
        }],
        "rate_limit_event" => {
            let info = value.get("rate_limit_info").unwrap_or(&Value::Null);
            vec![StreamEvent::RateLimit {
                status: lower_word(info.get("status")).unwrap_or_else(|| "other".to_owned()),
                kind: lower_word(info.get("rateLimitType")),
                utilization: info
                    .get("utilization")
                    .and_then(Value::as_f64)
                    .filter(|u| u.is_finite() && *u >= 0.0),
                resets_at: info.get("resetsAt").and_then(Value::as_i64),
            }]
        }
        "system" => vec![StreamEvent::System {
            subtype: lower_word(value.get("subtype")).unwrap_or_else(|| "none".to_owned()),
        }],
        _ => vec![StreamEvent::Other],
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

    fn argv_strings(argv: &[OsString]) -> Vec<String> {
        argv.iter()
            .map(|a| a.to_string_lossy().into_owned())
            .collect()
    }

    #[test]
    fn the_argv_reads_stream_json_on_stdin_and_has_no_prompt() {
        let rt = AgentRuntime::default();
        assert_eq!(
            argv_strings(&rt.argv("opus", 10.0)).join(" "),
            "claude -p --setting-sources user --model opus --input-format stream-json \
             --output-format stream-json --verbose --replay-user-messages \
             --forward-subagent-text --permission-mode bypassPermissions --max-budget-usd 10"
        );
        // The driver's command is untouched.
        let driver = argv_strings(&crate::launcher::sandbox::claude_argv("x", "opus", 10.0));
        assert_eq!(&driver[driver.len() - 2..], ["--", "x"]);
        assert!(!driver.iter().any(|a| a == "--input-format"));
    }

    #[test]
    fn a_user_message_is_one_json_line_with_pinned_key_order() {
        let line = AgentRuntime::default().user_message_line("a\n\"b\"");
        assert_eq!(
            line,
            "{\"type\":\"user\",\"message\":{\"role\":\"user\",\"content\":[{\"type\":\"text\",\"text\":\"a\\n\\\"b\\\"\"}]},\"parent_tool_use_id\":null}\n"
        );
        assert_eq!(line.matches('\n').count(), 1);
    }

    fn classify(line: &str) -> Vec<StreamEvent> {
        AgentRuntime::default().classify(line)
    }

    #[test]
    fn a_replay_is_a_user_replay() {
        for content in [r#"[{"type":"text","text":"hi"}]"#, r#""hi""#] {
            let line = format!(
                r#"{{"type":"user","isReplay":true,"message":{{"role":"user","content":{content}}},"parent_tool_use_id":null}}"#
            );
            assert_eq!(
                classify(&line),
                [StreamEvent::UserReplay { text: "hi".into() }]
            );
        }
    }

    #[test]
    fn a_tool_result_carries_its_first_line_and_line_count() {
        let line = r#"{"type":"user","message":{"role":"user","content":[{"type":"tool_result","tool_use_id":"t","content":"one\ntwo\nthree","is_error":true}]},"parent_tool_use_id":"t1"}"#;
        assert_eq!(
            classify(line),
            [StreamEvent::ToolResult {
                sub: true,
                is_error: true,
                first_line: "one".into(),
                lines: 3
            }]
        );
    }

    #[test]
    fn an_assistant_line_yields_text_tool_calls_and_usage_in_order() {
        let line = r#"{"type":"assistant","message":{"id":"m1","model":"claude-opus-5-5","content":[{"type":"thinking","thinking":"hmm"},{"type":"text","text":"on it"},{"type":"tool_use","id":"u","name":"Bash","input":{"command":"cargo test"}}],"usage":{"input_tokens":3,"output_tokens":4,"cache_creation_input_tokens":5,"cache_read_input_tokens":6}},"parent_tool_use_id":null}"#;
        assert_eq!(
            classify(line),
            [
                StreamEvent::AssistantText {
                    sub: false,
                    text: "on it".into()
                },
                StreamEvent::ToolCall {
                    sub: false,
                    name: "Bash".into(),
                    summary: "cargo test".into()
                },
                StreamEvent::Usage {
                    sub: false,
                    message_id: "m1".into(),
                    model: "claude-opus-5-5".into(),
                    tokens: UsageTokens {
                        input: 3,
                        output: 4,
                        cache_creation: 5,
                        cache_read: 6
                    }
                },
            ]
        );
        let sub = line.replace(
            r#""parent_tool_use_id":null"#,
            r#""parent_tool_use_id":"t1""#,
        );
        assert!(classify(&sub).iter().all(|e| match e {
            StreamEvent::AssistantText { sub, .. }
            | StreamEvent::ToolCall { sub, .. }
            | StreamEvent::Usage { sub, .. } => *sub,
            _ => false,
        }));
    }

    #[test]
    fn results_rate_limits_and_system_lines_are_sanitised() {
        assert_eq!(
            classify(
                r#"{"type":"result","subtype":"success","total_cost_usd":0.42,"num_turns":3,"is_error":false}"#
            ),
            [StreamEvent::TurnResult {
                subtype: "success".into(),
                cost_usd: Some(0.42),
                turns: Some(3),
                is_error: false
            }]
        );
        assert_eq!(
            classify(r#"{"type":"result","subtype":"\u001b[31mevil"}"#),
            [StreamEvent::TurnResult {
                subtype: "other".into(),
                cost_usd: None,
                turns: None,
                is_error: false
            }]
        );
        assert_eq!(
            classify(
                r#"{"type":"rate_limit_event","rate_limit_info":{"status":"allowed_warning","rateLimitType":"five_hour","utilization":0.85,"resetsAt":1800003600}}"#
            ),
            [StreamEvent::RateLimit {
                status: "allowed_warning".into(),
                kind: Some("five_hour".into()),
                utilization: Some(0.85),
                resets_at: Some(1_800_003_600)
            }]
        );
        assert_eq!(
            classify(
                r#"{"type":"rate_limit_event","rate_limit_info":{"status":"Evil!","rateLimitType":"x y"}}"#
            ),
            [StreamEvent::RateLimit {
                status: "other".into(),
                kind: Some("other".into()),
                utilization: None,
                resets_at: None
            }]
        );
        assert_eq!(
            classify(r#"{"type":"system","subtype":"init"}"#),
            [StreamEvent::System {
                subtype: "init".into()
            }]
        );
        assert_eq!(classify("not json"), [StreamEvent::Other]);
        assert_eq!(classify(r#"{"type":"mystery"}"#), [StreamEvent::Other]);
    }

    #[test]
    fn a_tool_is_summarised_by_the_argument_that_says_what_it_does() {
        let s =
            |name: &str, input: &str| summarize_tool(name, &serde_json::from_str(input).unwrap());
        assert_eq!(s("Bash", r#"{"command":"ls -la","timeout":5}"#), "ls -la");
        assert_eq!(s("Read", r#"{"file_path":"/a/b"}"#), "/a/b");
        assert_eq!(
            s("Edit", r#"{"file_path":"/a/c","old_string":"x"}"#),
            "/a/c"
        );
        assert_eq!(s("Grep", r#"{"pattern":"foo","path":"."}"#), "foo");
        assert_eq!(s("Task", r#"{"description":"scan","prompt":"p"}"#), "scan");
        assert_eq!(s("Skill", r#"{"skill":"gsd-fast"}"#), "gsd-fast");
        assert_eq!(s("Weird", r#"{"n":1,"q":"first string"}"#), "first string");
        assert_eq!(s("Bash", r#"{"command":"a\nb\u001b[0m"}"#), "a b [0m");
        let long = format!(r#"{{"command":"{}"}}"#, "x".repeat(500));
        assert_eq!(s("Bash", &long).chars().count(), 100);
    }

    #[test]
    fn the_price_table_is_notional_and_the_longest_prefix_wins() {
        let rt = AgentRuntime::default();
        let one = |model: &str, field: &str| {
            let mut t = UsageTokens::default();
            match field {
                "in" => t.input = 1_000_000,
                "out" => t.output = 1_000_000,
                "cr" => t.cache_read = 1_000_000,
                _ => t.cache_creation = 1_000_000,
            }
            rt.notional_usd(model, &t)
        };
        let close = |a: f64, b: f64| (a - b).abs() < 1e-9;
        assert!(close(one("claude-opus-5-5", "in"), 4.0));
        assert!(close(one("claude-opus-5-5", "out"), 20.0));
        assert!(close(one("claude-opus-5-5", "cr"), 0.20));
        assert!(close(one("claude-opus-5-5", "cw"), 5.0));
        assert!(close(one("claude-opus-5-5-20270101", "in"), 4.0));
        assert!(close(one("claude-opus-5", "in"), 5.0));
        assert!(close(one("claude-opus-5", "out"), 25.0));
        assert!(close(one("claude-sonnet-5-5", "in"), 2.0));
        assert!(close(one("claude-sonnet-5-5", "out"), 10.0));
        assert!(close(one("claude-sonnet-4-6", "in"), 3.0));
        assert!(close(one("claude-sonnet-4-6", "out"), 15.0));
        assert!(close(one("claude-haiku-4-5", "in"), 1.0));
        assert!(close(one("claude-haiku-4-5", "out"), 5.0));
        assert!(close(one("some-opus-thing", "in"), 4.0), "family fallback");
        assert!(close(one("mystery", "in"), 5.0));
        assert!(close(one("mystery", "out"), 25.0));
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
