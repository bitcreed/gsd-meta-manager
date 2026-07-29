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
//! `output_style` are snake_case. A blanket camelCase rename-all would
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
    /// `assistant` — a model turn.
    Assistant(TurnMessage),
    /// `user` — either a prompt echo (`isReplay`) or a tool result.
    User(TurnMessage),
    /// `result` — closes a **turn**, not the run (D-29). Boxed because the
    /// payload dwarfs every other variant (`clippy::large_enum_variant` is a
    /// `-D warnings` build gate here, per the `src/action.rs` precedent).
    Result(Box<ResultMessage>),
    /// `control_response` — the reply to a `control_request` we wrote.
    ControlResponse(ControlResponse),
    /// `rate_limit_event`. The payload is carried unmodelled, following the
    /// escape-hatch idiom in `state_reader/config_json.rs`: nothing in this
    /// phase reads it, and modelling a shape we do not consume would only add
    /// a second thing to keep in sync with the CLI's patch cadence.
    RateLimitEvent(serde_json::Value),
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

/// An `assistant` or `user` turn message.
///
/// Both types share one shape. The message body itself is deliberately not
/// modelled: this phase routes envelopes, and the content blocks are Phase 18's
/// rendering concern.
#[derive(Debug, Clone, Deserialize)]
pub struct TurnMessage {
    /// Stable across the process's turns.
    #[serde(default)]
    pub session_id: Option<String>,
    /// The `--replay-user-messages` echo marker — the only delivery ack the
    /// design has.
    ///
    /// It is camelCase on the wire **and absent rather than `false`** on
    /// non-replay messages, so it needs both a rename and a default. It is also
    /// emitted at *dequeue*, not at receipt: a message written mid-turn is
    /// echoed after the preceding turn's `result`, which makes this a "started
    /// processing" ack rather than a "received" ack (D-31).
    #[serde(default, rename = "isReplay")]
    pub is_replay: bool,
    /// Non-null on subagent messages.
    #[serde(default)]
    pub parent_tool_use_id: Option<String>,
    /// Per-message identity.
    #[serde(default)]
    pub uuid: Option<String>,
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
    /// The agent's own summary of the turn.
    ///
    /// `Option` because it is **absent entirely** on error envelopes — not
    /// empty, absent (D-32). Defaulting it to an empty string would make an
    /// error envelope indistinguishable from a silent success. Nothing derives
    /// an outcome from this field: outcome comes from the envelope's verdict
    /// fields, the exit code and the disk delta, never from prose.
    #[serde(default)]
    pub result: Option<String>,
    /// Present only on error envelopes, hence the default.
    #[serde(default)]
    pub errors: Vec<String>,
    /// Denials the CLI recorded. Populated when `--permission-mode dontAsk`
    /// blocked something; carried unmodelled.
    #[serde(default)]
    pub permission_denials: Vec<serde_json::Value>,
    /// Success-only; absent on error envelopes.
    #[serde(default)]
    pub api_error_status: Option<serde_json::Value>,
    /// Why the model stopped, as distinct from why the turn ended.
    #[serde(default)]
    pub stop_reason: Option<String>,
    /// Per-envelope identity. Differs between the turns of one session, which
    /// is what makes two `result`s from one process distinguishable (D-29).
    #[serde(default)]
    pub uuid: Option<String>,
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

    // Compile-time fixture loading: the tests do no filesystem I/O and need no
    // temporary directory. Every one of these is a real 2.1.220 capture, so a
    // failure here means the model is wrong, not that the test is wrong.
    const T01: &str = include_str!("../../tests/fixtures/transcripts/01-success-textonly.ndjson");
    const T02: &str = include_str!("../../tests/fixtures/transcripts/02-budget-exhausted.ndjson");
    const T03: &str =
        include_str!("../../tests/fixtures/transcripts/03-tooluse-success-settingsources.ndjson");
    const T04: &str =
        include_str!("../../tests/fixtures/transcripts/04-hookhang-aborted-tools.ndjson");
    const T05: &str =
        include_str!("../../tests/fixtures/transcripts/05-queued-injection-two-turns.ndjson");
    const T06: &str =
        include_str!("../../tests/fixtures/transcripts/06-interrupt-aborted-streaming.ndjson");
    const T07: &str = include_str!("../../tests/fixtures/transcripts/07-interrupt-early.ndjson");
    const T08: &str =
        include_str!("../../tests/fixtures/transcripts/08-tooluse-queued-two-turns.ndjson");

    const ALL_TRANSCRIPTS: [(&str, &str); 8] = [
        ("01-success-textonly", T01),
        ("02-budget-exhausted", T02),
        ("03-tooluse-success-settingsources", T03),
        ("04-hookhang-aborted-tools", T04),
        ("05-queued-injection-two-turns", T05),
        ("06-interrupt-aborted-streaming", T06),
        ("07-interrupt-early", T07),
        ("08-tooluse-queued-two-turns", T08),
    ];

    fn parse_all(transcript: &str) -> Vec<Envelope> {
        transcript
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(parse_line)
            .collect()
    }

    fn messages(transcript: &str) -> Vec<StreamMessage> {
        parse_all(transcript)
            .into_iter()
            .filter_map(|env| match env {
                Envelope::Parsed { msg, .. } => Some(msg),
                Envelope::Unparseable { .. } => None,
            })
            .collect()
    }

    fn results(transcript: &str) -> Vec<Box<ResultMessage>> {
        messages(transcript)
            .into_iter()
            .filter_map(|msg| match msg {
                StreamMessage::Result(result) => Some(result),
                _ => None,
            })
            .collect()
    }

    fn inits(transcript: &str) -> Vec<Box<InitMessage>> {
        messages(transcript)
            .into_iter()
            .filter_map(|msg| match msg {
                StreamMessage::System(SystemMessage::Init(init)) => Some(init),
                _ => None,
            })
            .collect()
    }

    // ========================================================================
    // Tolerance across every golden transcript (D-09)
    // ========================================================================

    #[test]
    fn every_line_of_every_golden_transcript_parses_to_a_carried_envelope() {
        for (name, transcript) in ALL_TRANSCRIPTS {
            for (index, env) in parse_all(transcript).into_iter().enumerate() {
                assert!(
                    matches!(env, Envelope::Parsed { .. }),
                    "{name} line {} did not parse: {env:?}",
                    index + 1
                );
            }
        }
    }

    #[test]
    fn no_golden_transcript_line_lands_in_the_forward_compat_unknown_variant() {
        for (name, transcript) in ALL_TRANSCRIPTS {
            for (index, msg) in messages(transcript).into_iter().enumerate() {
                assert!(
                    !matches!(msg, StreamMessage::Unknown),
                    "{name} line {} fell through to Unknown; the model is missing a type: {msg:?}",
                    index + 1
                );
            }
        }
    }

    #[test]
    fn a_thinking_tokens_line_absorbs_into_the_catch_all_subtype() {
        let raw = r#"{"type":"system","subtype":"thinking_tokens","estimated_tokens":50,"estimated_tokens_delta":50,"uuid":"u","session_id":"s"}"#;
        match parse_line(raw) {
            Envelope::Parsed {
                msg: StreamMessage::System(SystemMessage::Other),
                ..
            } => {}
            other => panic!("thinking_tokens must never be an error, got: {other:?}"),
        }
    }

    #[test]
    fn hook_and_retry_subtypes_absorb_into_the_catch_all_too() {
        for subtype in ["hook_started", "hook_response", "api_retry", "task_progress"] {
            let raw = format!(r#"{{"type":"system","subtype":"{subtype}","session_id":"s"}}"#);
            match parse_line(&raw) {
                Envelope::Parsed {
                    msg: StreamMessage::System(SystemMessage::Other),
                    ..
                } => {}
                other => panic!("{subtype} must absorb without error, got: {other:?}"),
            }
        }
    }

    #[test]
    fn an_injected_unknown_field_on_system_init_is_ignored() {
        let raw = r#"{"type":"system","subtype":"init","session_id":"s","capabilities":["a"],"brand_new_field":1,"claude_code_version":"2.1.220"}"#;
        match parse_line(raw) {
            Envelope::Parsed {
                msg: StreamMessage::System(SystemMessage::Init(init)),
                ..
            } => assert_eq!(init.claude_code_version.as_deref(), Some("2.1.220")),
            other => panic!("an unknown field must be ignored, got: {other:?}"),
        }
    }

    #[test]
    fn a_message_type_from_no_known_version_is_carried_as_forward_compat() {
        let env = parse_line(r#"{"type":"totally_new_message_type_from_2_2_0","x":1}"#);
        assert!(
            matches!(
                env,
                Envelope::Parsed {
                    msg: StreamMessage::Unknown,
                    ..
                }
            ),
            "a future message type must be carried, not fatal: {env:?}"
        );
    }

    #[test]
    fn a_torn_line_is_unparseable_and_distinct_from_unknown() {
        let env = parse_line(r#"{"type":"result","subtype":"suc"#);
        assert!(
            matches!(env, Envelope::Unparseable { .. }),
            "a torn line is a diagnostic, not a forward-compat unknown: {env:?}"
        );
    }

    // ========================================================================
    // result is a TURN boundary, not a run terminator (D-29, D-30)
    // ========================================================================

    #[test]
    fn fixture_05_yields_two_system_inits_and_two_result_envelopes() {
        assert_eq!(
            inits(T05).len(),
            2,
            "every queued turn emits its own system/init (D-30)"
        );
        assert_eq!(
            results(T05).len(),
            2,
            "result closes a turn, not the run — one process, two results (D-29)"
        );
    }

    #[test]
    fn fixture_05_results_share_a_session_and_differ_per_message() {
        let results = results(T05);
        let first = results.first().expect("first result");
        let second = results.get(1).expect("second result");

        assert_eq!(
            first.session_id, second.session_id,
            "the session id is stable across turns"
        );
        assert!(
            first.uuid.is_some() && second.uuid.is_some(),
            "each envelope carries its own uuid"
        );
        assert_ne!(
            first.uuid, second.uuid,
            "the per-message uuid differs per envelope"
        );
    }

    #[test]
    fn fixture_05_num_turns_resets_while_cost_accumulates() {
        let results = results(T05);
        let first = results.first().expect("first result");
        let second = results.get(1).expect("second result");

        assert_eq!(
            first.num_turns, second.num_turns,
            "num_turns is per-turn and resets; it is not a run-level counter (D-29)"
        );

        let first_cost = first.total_cost_usd.expect("first cost");
        let second_cost = second.total_cost_usd.expect("second cost");
        assert!(
            second_cost > first_cost,
            "total_cost_usd accumulates across turns: {first_cost} then {second_cost}"
        );
    }

    // ========================================================================
    // Error envelopes carry different fields from success envelopes (D-32)
    // ========================================================================

    #[test]
    fn fixture_02_budget_envelope_has_no_result_and_a_populated_errors_array() {
        let results = results(T02);
        let envelope = results.first().expect("the budget transcript has one result");

        assert_eq!(envelope.subtype, "error_max_budget_usd");
        assert_eq!(envelope.terminal_reason.as_deref(), Some("budget_exhausted"));
        assert!(
            envelope.result.is_none(),
            "result is absent on error envelopes and must not default to empty: {:?}",
            envelope.result
        );
        assert!(
            !envelope.errors.is_empty(),
            "errors is populated only on error envelopes, got: {:?}",
            envelope.errors
        );
    }

    #[test]
    fn a_success_envelope_carries_a_result_and_no_errors() {
        let results = results(T01);
        let envelope = results.first().expect("the clean baseline has one result");

        assert_eq!(envelope.subtype, "success");
        assert_eq!(envelope.result.as_deref(), Some("PONG"));
        assert!(
            envelope.errors.is_empty(),
            "errors is absent on success envelopes, got: {:?}",
            envelope.errors
        );
    }

    // ========================================================================
    // The replay marker is the only delivery ack the design has (D-31)
    // ========================================================================

    #[test]
    fn a_user_message_carrying_the_replay_marker_parses_with_it_true() {
        let raw = r#"{"type":"user","message":{"role":"user","content":[]},"session_id":"s","isReplay":true,"uuid":"u"}"#;
        match parse_line(raw) {
            Envelope::Parsed {
                msg: StreamMessage::User(turn),
                ..
            } => assert!(turn.is_replay, "the camelCase marker must be read"),
            other => panic!("expected a user turn message, got: {other:?}"),
        }
    }

    #[test]
    fn a_user_message_omitting_the_replay_marker_parses_with_it_false() {
        let raw = r#"{"type":"user","message":{"role":"user","content":[]},"session_id":"s","uuid":"u"}"#;
        match parse_line(raw) {
            Envelope::Parsed {
                msg: StreamMessage::User(turn),
                ..
            } => assert!(
                !turn.is_replay,
                "the marker is ABSENT rather than false on non-replay messages, so it needs a default"
            ),
            other => panic!("expected a user turn message, got: {other:?}"),
        }
    }

    #[test]
    fn assistant_messages_parse_as_turn_messages() {
        let count = messages(T03)
            .into_iter()
            .filter(|msg| matches!(msg, StreamMessage::Assistant(_)))
            .count();
        assert!(
            count > 0,
            "the tool-use transcript contains assistant turns; none parsed as one"
        );
    }

    // ========================================================================
    // control_response correlation (D-31)
    // ========================================================================

    #[test]
    fn fixture_07_exposes_still_queued_through_the_doubly_nested_response() {
        let responses: Vec<ControlResponse> = messages(T07)
            .into_iter()
            .filter_map(|msg| match msg {
                StreamMessage::ControlResponse(response) => Some(response),
                _ => None,
            })
            .collect();

        let response = responses
            .first()
            .expect("the early-interrupt transcript carries a control_response");

        assert!(
            !response.response.request_id.is_empty(),
            "the request id must be reachable for correlation"
        );
        assert_eq!(
            response.response.subtype, "success",
            "success here means the request was ACCEPTED, not that anything was cancelled"
        );
        assert!(
            response.response.still_queued().is_empty(),
            "fixture 07 cancelled nothing meaningful, so still_queued is empty: {:?}",
            response.response.still_queued()
        );
    }

    // ========================================================================
    // Rate limiting, and the gate rather than the parser refusing a bad init
    // ========================================================================

    #[test]
    fn a_rate_limit_event_parses_into_its_own_carried_variant() {
        let count = messages(T01)
            .into_iter()
            .filter(|msg| matches!(msg, StreamMessage::RateLimitEvent(_)))
            .count();
        assert!(
            count > 0,
            "the clean baseline includes a rate_limit_event in the happy path"
        );
    }

    #[test]
    fn a_system_init_with_no_capabilities_parses_so_the_gate_can_refuse_it() {
        for raw in [
            r#"{"type":"system","subtype":"init","session_id":"s"}"#,
            r#"{"type":"system","subtype":"init","session_id":"s","capabilities":[]}"#,
        ] {
            match parse_line(raw) {
                Envelope::Parsed {
                    msg: StreamMessage::System(SystemMessage::Init(init)),
                    ..
                } => assert!(
                    init.capabilities.is_empty(),
                    "an absent or empty array parses to empty; refusing it is the gate's job"
                ),
                other => panic!("the parser must not refuse this, got: {other:?}"),
            }
        }
    }

    #[test]
    fn the_mixed_casing_of_system_init_is_read_field_by_field() {
        let init = inits(T01);
        let init = init.first().expect("the clean baseline opens with an init");
        assert_eq!(
            init.claude_code_version.as_deref(),
            Some("2.1.220"),
            "a blanket camelCase rename would null this field out and make the version gate pass everything (Pitfall F)"
        );
        assert_eq!(init.api_key_source.as_deref(), Some("none"));
        assert_eq!(init.permission_mode.as_deref(), Some("default"));
    }

    // ========================================================================
    // Outbound wire shape (TRANS-01)
    // ========================================================================

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
