//! Serde model for the `claude -p --output-format stream-json` NDJSON protocol.
//!
//! Parsing is **tolerant by construction and never fails a run** (D-09). Every
//! line arriving here is untrusted input from a process that itself consumed
//! untrusted repository content, so:
//!
//! - Serde's strict unknown-field rejection attribute is never opted into
//!   anywhere in this file, and its absence is grepped for as a mechanical
//!   guard. Nine new
//!   `system/init` fields and an entirely new `system` subtype appeared between
//!   the committed research pass and the phase spike, on the same CLI version
//!   line. A run of the real 2.1.220 CLI was measured at 41% undocumented
//!   subtypes — an exhaustive match would have failed that run outright.
//! - Unknown message `type`s and unknown `subtype`s absorb into catch-all
//!   variants and are carried, never fatal.
//! - `subtype` and `terminal_reason` are plain `String`s, never enums. This CLI
//!   shipped three new values on one version line; a typed enum would need a
//!   catch-all on each and would still lose the actual string.
//! - No panicking accessor exists in the non-test region of this file.
//!
//! `#[serde(other)]` is only accepted on a **unit** variant of an
//! internally-tagged enum, so the enum itself cannot carry the raw body. The
//! raw line is therefore preserved outside the enum, by [`Envelope`], in the
//! reader task that already owns the `String`. That also separates two cases a
//! single catch-all would conflate: an unknown *message type* (forward-compat,
//! carry it) from a *malformed line* (a torn write — a real diagnostic).
//!
//! Casing in `system/init` is **mixed within one object**: `apiKeySource` and
//! `permissionMode` are camelCase while `claude_code_version`, `session_id` and
//! `output_style` are snake_case. A blanket `rename_all = "camelCase"` would
//! silently deserialise `claude_code_version` as `None` and make the version
//! gate pass everything, so per-field renames are used instead.

use serde::{Deserialize, Serialize};

/// One message off the `stream-json` stream, dispatched on the `type` field.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type")]
#[serde(rename_all = "snake_case")]
pub enum StreamMessage {
    /// `system/*` — init, hook events, thinking-token counters, task progress.
    System(SystemMessage),
    /// `result` — closes a **turn**, not the run (D-29). Boxed because the
    /// payload dwarfs every other variant (`clippy::large_enum_variant` is a
    /// `-D warnings` build gate here, per the `src/action.rs` precedent).
    Result(Box<ResultMessage>),
    /// `control_response` — the reply to a `control_request` we wrote.
    ControlResponse(ControlResponse),
    /// A message type no version we have observed emits. Carried as
    /// forward-compat; the raw line is preserved by [`Envelope`].
    #[serde(other)]
    Unknown,
}

/// A `system` message, dispatched on the `subtype` field.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "subtype")]
#[serde(rename_all = "snake_case")]
pub enum SystemMessage {
    /// `system/init` — the capability, version and auth-source announcement.
    /// Boxed for the same large-variant reason as `StreamMessage::Result`.
    Init(Box<InitMessage>),
    /// Every other subtype: `hook_started`, `hook_response`, `thinking_tokens`,
    /// `api_retry`, `task_progress`, `vcs_state_changed`, and whatever ships
    /// next. Absorbed without error, never a diagnostic (D-32).
    #[serde(other)]
    Other,
}

/// The `system/init` payload — the raw material for the capability gate.
#[derive(Debug, Clone, Deserialize)]
pub struct InitMessage {
    /// The session UUID. We generate this pre-spawn via `--session-id`, so the
    /// observed value should echo it back.
    #[serde(default)]
    pub session_id: Option<String>,
    /// Protocol capabilities the CLI advertises. Absent or empty parses fine —
    /// the gate is what refuses it, not the parser (TRANS-04).
    #[serde(default)]
    pub capabilities: Vec<String>,
    /// `"none"` means subscription/OAuth auth. The `--bare` regression guard
    /// asserts against this (D-08). camelCase on the wire.
    #[serde(default, rename = "apiKeySource")]
    pub api_key_source: Option<String>,
    /// Reflects the `--permission-mode` flag actually taking effect (D-15).
    /// camelCase on the wire.
    #[serde(default, rename = "permissionMode")]
    pub permission_mode: Option<String>,
    /// snake_case on the wire — do not "fix" this with a blanket rename.
    #[serde(default)]
    pub claude_code_version: Option<String>,
}

/// The `result` payload. One per **turn** (D-29).
#[derive(Debug, Clone, Deserialize)]
pub struct ResultMessage {
    /// Observed: `success`, `error_max_turns`, `error_during_execution`,
    /// `error_max_budget_usd`. Deliberately a `String`.
    #[serde(default)]
    pub subtype: String,
    /// Whether the CLI classified this turn as an error.
    #[serde(default)]
    pub is_error: bool,
    /// Observed: `completed`, `aborted_tools`, `budget_exhausted`,
    /// `aborted_streaming`. Deliberately a `String`.
    #[serde(default)]
    pub terminal_reason: Option<String>,
    /// Stable across every turn of one process.
    #[serde(default)]
    pub session_id: Option<String>,
    /// **Per-turn and resets** — it is not a run-level counter (D-29).
    #[serde(default)]
    pub num_turns: Option<u64>,
    /// **Cumulative across turns.** Under subscription auth this is a notional
    /// price for work that is not billed per call — never present it as "what
    /// this run cost" (D-16).
    #[serde(default)]
    pub total_cost_usd: Option<f64>,
}

/// A `control_response` envelope. Note the **double nesting** of `response`.
#[derive(Debug, Clone, Deserialize)]
pub struct ControlResponse {
    /// The response body.
    pub response: ControlResponseBody,
}

/// The inner body of a `control_response`.
#[derive(Debug, Clone, Deserialize)]
pub struct ControlResponseBody {
    /// `"success"` here means **the request was accepted**, not that the thing
    /// you meant was cancelled. Correlate on `request_id` and read
    /// `still_queued` as authoritative (D-31).
    #[serde(default)]
    pub subtype: String,
    /// Correlates back to the `control_request` we wrote.
    #[serde(default)]
    pub request_id: String,
    /// The doubly-nested payload, carrying `{"still_queued": [...]}`. Kept as
    /// an unmodelled value following the escape-hatch idiom in
    /// `state_reader/config_json.rs`.
    #[serde(default)]
    pub response: Option<serde_json::Value>,
}

impl ControlResponseBody {
    /// The authoritative statement of what remains queued after an interrupt.
    ///
    /// Returns an empty slice when the field is absent or is not an array —
    /// never panics, never unwraps (V5).
    pub fn still_queued(&self) -> Vec<serde_json::Value> {
        self.response
            .as_ref()
            .and_then(|v| v.get("still_queued"))
            .and_then(|v| v.as_array())
            .cloned()
            .unwrap_or_default()
    }
}

/// The outbound user message written to stdin, in the exact observed wire
/// shape: `{"type":"user","message":{"role":"user","content":[{"type":"text",
/// "text":"…"}]}}` (TRANS-01).
#[derive(Debug, Clone, Serialize)]
pub struct UserMessage {
    #[serde(rename = "type")]
    kind: &'static str,
    message: UserMessageBody,
}

#[derive(Debug, Clone, Serialize)]
struct UserMessageBody {
    role: &'static str,
    content: Vec<TextBlock>,
}

#[derive(Debug, Clone, Serialize)]
struct TextBlock {
    #[serde(rename = "type")]
    kind: &'static str,
    text: String,
}

impl UserMessage {
    /// Build a user message carrying one text block.
    pub fn text(text: impl Into<String>) -> Self {
        Self {
            kind: "user",
            message: UserMessageBody {
                role: "user",
                content: vec![TextBlock {
                    kind: "text",
                    text: text.into(),
                }],
            },
        }
    }
}

/// The outbound `control_request` envelope.
///
/// The bare `{"type":"interrupt"}` form does nothing on 2.1.220 and must never
/// be used (D-31, community issue #41665).
#[derive(Debug, Clone, Serialize)]
pub struct ControlRequest {
    #[serde(rename = "type")]
    kind: &'static str,
    request_id: String,
    request: ControlRequestBody,
}

#[derive(Debug, Clone, Serialize)]
struct ControlRequestBody {
    subtype: &'static str,
}

impl ControlRequest {
    /// Build an interrupt request correlated by `request_id`.
    pub fn interrupt(request_id: impl Into<String>) -> Self {
        Self {
            kind: "control_request",
            request_id: request_id.into(),
            request: ControlRequestBody {
                subtype: "interrupt",
            },
        }
    }
}

/// What the reader task yields for one observed line.
///
/// Parsing **never** fails a run: a line either parsed into a carried message
/// or is reported as unparseable, and both keep the run going (D-09).
#[derive(Debug, Clone)]
pub enum Envelope {
    /// The line parsed. `msg` may still be a forward-compat `Unknown`.
    Parsed {
        /// The raw line exactly as observed.
        raw: String,
        /// The parsed message.
        msg: StreamMessage,
    },
    /// The line did not parse — a torn write, a truncation, or invalid JSON.
    /// Distinct from `StreamMessage::Unknown`, which is a *known-good* line of
    /// an unknown type.
    Unparseable {
        /// The raw line exactly as observed.
        raw: String,
        /// The serde error, rendered.
        error: String,
    },
}

/// Parse one NDJSON line. Never panics, never fails a run.
pub fn parse_line(raw: &str) -> Envelope {
    match serde_json::from_str::<StreamMessage>(raw) {
        Ok(msg) => Envelope::Parsed {
            raw: raw.to_string(),
            msg,
        },
        Err(e) => Envelope::Unparseable {
            raw: raw.to_string(),
            error: e.to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE_01: &str = include_str!("../../tests/fixtures/transcripts/01-success-textonly.ndjson");

    fn parse_all(transcript: &str) -> Vec<Envelope> {
        transcript
            .lines()
            .filter(|l| !l.trim().is_empty())
            .map(parse_line)
            .collect()
    }

    #[test]
    fn every_line_of_the_clean_baseline_parses() {
        for env in parse_all(FIXTURE_01) {
            assert!(
                matches!(env, Envelope::Parsed { .. }),
                "expected a parsed envelope, got: {env:?}"
            );
        }
    }

    #[test]
    fn first_line_of_the_clean_baseline_is_a_system_init() {
        let first = parse_all(FIXTURE_01).into_iter().next();
        match first {
            Some(Envelope::Parsed {
                msg: StreamMessage::System(SystemMessage::Init(init)),
                ..
            }) => {
                assert_eq!(
                    init.capabilities.len(),
                    3,
                    "expected the three 2.1.220 capabilities, got: {:?}",
                    init.capabilities
                );
                assert_eq!(
                    init.api_key_source.as_deref(),
                    Some("none"),
                    "expected subscription auth, got: {:?}",
                    init.api_key_source
                );
                assert_eq!(
                    init.claude_code_version.as_deref(),
                    Some("2.1.220"),
                    "a blanket rename_all would null this field out (Pitfall F), got: {:?}",
                    init.claude_code_version
                );
            }
            other => panic!("expected a parsed system/init, got: {other:?}"),
        }
    }

    #[test]
    fn a_torn_line_is_unparseable_and_not_unknown() {
        let env = parse_line(r#"{"type":"result","subtype":"suc"#);
        assert!(
            matches!(env, Envelope::Unparseable { .. }),
            "expected Unparseable, got: {env:?}"
        );
    }

    #[test]
    fn an_unknown_message_type_is_carried_not_fatal() {
        let env = parse_line(r#"{"type":"totally_new_message_type_from_2_2_0","x":1}"#);
        assert!(
            matches!(
                env,
                Envelope::Parsed {
                    msg: StreamMessage::Unknown,
                    ..
                }
            ),
            "expected a carried Unknown, got: {env:?}"
        );
    }

    #[test]
    fn the_outbound_user_message_matches_the_observed_wire_shape() {
        let json = serde_json::to_string(&UserMessage::text("hello")).expect("serialise");
        assert_eq!(
            json,
            r#"{"type":"user","message":{"role":"user","content":[{"type":"text","text":"hello"}]}}"#,
            "outbound wire shape drifted"
        );
    }
}
