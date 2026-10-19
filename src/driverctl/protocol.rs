//! The driverctl wire protocol: request and response types and the pure
//! validation that runs before the store touches anything (D-01, D-04).
//!
//! The grammar is spike 002's, byte for byte on the wire:
//!
//! * a request is one JSON object `{"op": "log" | "hook" | "show", ...}`;
//! * `log` carries `event` and an `args` object keyed by the underscored argparse
//!   dests (`done_when`, `loop_mode`, ...), every value a string or `null`;
//! * a reply is `{"ok": true}`, `{"ok": true, "deny": reason}`,
//!   `{"ok": true, "frontmatter": text}` or `{"ok": false, "error": message}`.
//!
//! Everything here is pure and platform independent: [`parse_request`],
//! [`validate_log`] and [`validate_hook`] only look at their arguments, so a
//! rejection can be proven without a run directory. The store validates a whole
//! request first and applies it afterwards, which is why a refused request writes
//! nothing (D-04).
//!
//! # Tightenings over the spike `[inferred - audit]`
//!
//! None of these affects an invocation the MASTER-PROMPT makes (27-RESEARCH,
//! "Tightenings"): `RUN` is not an item on item-level events; `rule` is checked
//! whenever it is given; `phase`, `loop-mode` and `session` go through
//! [`one_line`]; a report body may not carry a control character other than tab;
//! unknown argument keys and non-string argument values are refused instead of
//! ignored or coerced.
//!
//! There is deliberately **no** op, event or argument here that can set a run
//! status of `crashed` or `stalled`, or write a counter: those are launcher-side
//! methods on the store (D-06, D-17), unreachable from the sandboxed driver.

use serde::de::DeserializeOwned;
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::Value;

use super::log::one_line;
use crate::state_reader::driver_frontmatter::{ItemState, RunStatus};

/// The longest request the store will parse (the 27-06 server enforces it on the
/// socket too, before it has read the whole line).
pub const MAX_REQUEST_BYTES: usize = 1 << 20;

/// A `report` keeps this many body lines; the rest are dropped and flagged.
pub const MAX_REPORT_LINES: usize = 15;

/// The park rules a driver may cite (spike `RULE_ENUM`).
pub const RULE_ENUM: &[&str] = &[
    "budget",
    "gap-rounds",
    "divergence",
    "no-progress",
    "needs-human",
    "scope",
    "run-budget",
    "launcher-cap",
];

/// `end --reason` and the run status each one records (spike `END_REASONS`).
/// `fresh-exit` keeps the run `running`: the driver ended its context, not the run.
pub const END_REASONS: &[(&str, RunStatus)] = &[
    ("all-done", RunStatus::Done),
    ("all-parked", RunStatus::Parked),
    ("pause", RunStatus::Paused),
    ("stop", RunStatus::Stopped),
    ("run-budget", RunStatus::Stopped),
    ("fresh-exit", RunStatus::Running),
];

/// `report --status`.
pub const REPORT_STATUSES: &[&str] = &["done", "partial", "blocked", "failed"];

/// `start --mode`.
pub const MODES: &[&str] = &["interactive", "headless"];

/// `recovered --action`.
pub const RECOVER_ACTIONS: &[&str] = &["done", "resume", "reset", "park"];

/// `verified --done-when`.
pub const DONE_WHEN: &[&str] = &["met", "not-met"];

/// Why a request was refused. The text goes to the driver as `{"ok": false,
/// "error": ..}` and to its stderr as `driverctl: REJECTED: ..` (rc 2).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Invalid(pub String);

impl std::fmt::Display for Invalid {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for Invalid {}

impl From<String> for Invalid {
    fn from(message: String) -> Self {
        Invalid(message)
    }
}

// ---------------------------------------------------------------- wire types

/// The `args` of a `log` request: the 17 argparse dests plus the report `body`.
///
/// Every value is `Option<String>`: the spike client sent strings only, so a JSON
/// number, bool or array is refused at the parse, and an unknown key is refused
/// by `deny_unknown_fields`.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct LogArgs {
    pub item: Option<String>,
    pub step: Option<String>,
    pub round: Option<String>,
    pub phase: Option<String>,
    pub status: Option<String>,
    pub done_when: Option<String>,
    pub evidence: Option<String>,
    pub state: Option<String>,
    pub rule: Option<String>,
    pub outcome: Option<String>,
    pub question: Option<String>,
    pub action: Option<String>,
    pub reason: Option<String>,
    pub found: Option<String>,
    pub mode: Option<String>,
    pub loop_mode: Option<String>,
    pub session: Option<String>,
    /// The worker report read from stdin (`report` only).
    pub body: Option<String>,
}

/// The eight fields the spike's hook client forwards from a Claude Code hook
/// payload, plus the (truncated) dispatch prompt.
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct HookPayload {
    pub hook_event_name: Option<String>,
    pub session_id: Option<String>,
    pub agent_id: Option<String>,
    pub agent_type: Option<String>,
    pub tool_name: Option<String>,
    pub source: Option<String>,
    pub reason: Option<String>,
    pub prompt: Option<String>,
}

impl HookPayload {
    /// Whether this payload is a driver-level dispatch the Gate must decide:
    /// `PreToolUse` of the `Agent` or `Task` tool with **no** `agent_id`. A payload
    /// with an `agent_id` comes from inside a subagent and is never counted (D-10).
    pub fn is_gate(&self) -> bool {
        self.hook_event_name.as_deref() == Some("PreToolUse")
            && matches!(self.tool_name.as_deref(), Some("Agent" | "Task"))
            && self.agent_id.is_none()
    }
}

/// `null` or an object, else an error; `null` is the type's default (the spike's
/// `r.get("args") or {}`). A bare array would otherwise deserialize as a tuple
/// struct, which is not a thing this protocol allows.
fn object_or_null<'de, D, T>(deserializer: D) -> Result<T, D::Error>
where
    D: Deserializer<'de>,
    T: Default + DeserializeOwned,
{
    match Value::deserialize(deserializer)? {
        Value::Null => Ok(T::default()),
        object @ Value::Object(_) => serde_json::from_value(object).map_err(serde::de::Error::custom),
        _ => Err(serde::de::Error::custom("expected an object or null")),
    }
}

/// One request. An unknown `op` is a serde error, so it is refused like any other
/// malformed request.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "op", rename_all = "lowercase")]
pub enum Request {
    Log {
        event: String,
        #[serde(default, deserialize_with = "object_or_null")]
        args: LogArgs,
    },
    Hook {
        #[serde(default, deserialize_with = "object_or_null")]
        payload: HookPayload,
    },
    Show,
}

/// One reply. Absent fields are omitted from the JSON, matching the spike.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Response {
    pub ok: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deny: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub frontmatter: Option<String>,
}

impl Response {
    /// `{"ok": true}`.
    pub fn ok() -> Self {
        Response {
            ok: true,
            error: None,
            deny: None,
            frontmatter: None,
        }
    }

    /// `{"ok": false, "error": message}`.
    pub fn rejected(message: impl Into<String>) -> Self {
        Response {
            ok: false,
            error: Some(message.into()),
            deny: None,
            frontmatter: None,
        }
    }

    /// `{"ok": true, "deny": reason}`: the Gate refused a dispatch.
    pub fn deny(reason: impl Into<String>) -> Self {
        Response {
            ok: true,
            error: None,
            deny: Some(reason.into()),
            frontmatter: None,
        }
    }

    /// `{"ok": true, "frontmatter": text}`.
    pub fn frontmatter(text: impl Into<String>) -> Self {
        Response {
            ok: true,
            error: None,
            deny: None,
            frontmatter: Some(text.into()),
        }
    }
}

/// Parse one request. Trailing ASCII whitespace (the spike client's `\n`, or none
/// at EOF) is ignored; anything else wrong is `malformed request: ..`.
pub fn parse_request(bytes: &[u8]) -> Result<Request, Invalid> {
    if bytes.len() > MAX_REQUEST_BYTES {
        return Err(Invalid(format!(
            "malformed request: larger than {MAX_REQUEST_BYTES} bytes"
        )));
    }
    let trimmed = bytes.trim_ascii_end();
    serde_json::from_slice(trimmed).map_err(|e| Invalid(format!("malformed request: {e}")))
}

// ---------------------------------------------------------------- validation

/// A `log` request that passed every check, with typed fields.
///
/// Holding one means the store can apply it without a further failure path of
/// its own (apart from I/O).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ValidatedEvent {
    Start {
        mode: String,
        loop_mode: Option<String>,
        session: Option<String>,
    },
    Reconcile {
        found: i64,
    },
    Dispatched {
        item: String,
        step: String,
        round: i64,
        phase: Option<String>,
    },
    Report {
        item: String,
        status: String,
        /// At most [`MAX_REPORT_LINES`] lines, each free of control characters.
        body: Vec<String>,
        truncated: bool,
        findings: Option<i64>,
    },
    Verified {
        item: String,
        done_when: String,
        evidence: String,
    },
    Status {
        item: String,
        state: ItemState,
        rule: Option<String>,
        outcome: Option<String>,
    },
    NeedsHuman {
        item: String,
        question: String,
    },
    Recovered {
        item: String,
        action: String,
        reason: String,
    },
    End {
        reason: String,
        status: RunStatus,
    },
}

/// A header value that is optional: absent or blank is `None`, anything else must
/// pass [`one_line`].
fn optional_line(name: &str, value: Option<&String>, limit: usize) -> Result<Option<String>, Invalid> {
    match value {
        Some(v) if !v.trim().is_empty() => Ok(Some(one_line(name, Some(v), limit)?)),
        _ => Ok(None),
    }
}

fn one_of(name: &str, value: Option<&String>, allowed: &[&str]) -> Result<String, Invalid> {
    match value {
        Some(v) if allowed.contains(&v.as_str()) => Ok(v.clone()),
        _ => Err(Invalid(format!("--{name} must be {}", allowed.join("|")))),
    }
}

/// Validate the whole `log` request against the current DIRECTION item ids.
///
/// Pure: the caller applies the returned event only after this returns `Ok`.
pub fn validate_log(
    event: &str,
    args: &LogArgs,
    items: &[String],
) -> Result<ValidatedEvent, Invalid> {
    let _ = items;
    match event {
        "start" => {
            let mode = one_of("mode", args.mode.as_ref(), MODES)?;
            Ok(ValidatedEvent::Start {
                mode,
                loop_mode: optional_line("loop-mode", args.loop_mode.as_ref(), 200)?,
                session: optional_line("session", args.session.as_ref(), 200)?,
            })
        }
        other => Err(Invalid(format!("unknown event {other:?}"))),
    }
}

/// Validate a hook payload. Only the values that reach a LOG header are checked:
/// `session_id` and `source` on `SessionStart`, `session_id` and `reason` on
/// `SessionEnd`. Gate and heartbeat payloads write no payload-derived text, so
/// they are never refused here (a refused gate payload would let a dispatch
/// through uncounted).
pub fn validate_hook(payload: &HookPayload) -> Result<(), Invalid> {
    match payload.hook_event_name.as_deref() {
        Some("SessionStart") => {
            optional_line("session", payload.session_id.as_ref(), 200)?;
            optional_line("source", payload.source.as_ref(), 200)?;
        }
        Some("SessionEnd") => {
            optional_line("session", payload.session_id.as_ref(), 200)?;
            optional_line("reason", payload.reason.as_ref(), 200)?;
        }
        _ => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn args(pairs: &[(&str, &str)]) -> LogArgs {
        let map: serde_json::Map<String, Value> = pairs
            .iter()
            .map(|(k, v)| ((*k).to_owned(), json!(v)))
            .collect();
        serde_json::from_value(Value::Object(map)).expect("test args")
    }

    fn items() -> Vec<String> {
        vec!["I-01".to_owned(), "I-02".to_owned()]
    }

    fn ok(event: &str, pairs: &[(&str, &str)]) -> ValidatedEvent {
        validate_log(event, &args(pairs), &items())
            .unwrap_or_else(|e| panic!("{event} {pairs:?} should be accepted: {e}"))
    }

    fn rejected(event: &str, pairs: &[(&str, &str)]) -> String {
        match validate_log(event, &args(pairs), &items()) {
            Ok(v) => panic!("{event} {pairs:?} should be rejected, got {v:?}"),
            Err(Invalid(m)) => m,
        }
    }

    #[test]
    fn start_accepts_a_known_mode_and_refuses_the_rest() {
        assert_eq!(
            ok("start", &[("mode", "headless")]),
            ValidatedEvent::Start {
                mode: "headless".into(),
                loop_mode: None,
                session: None
            }
        );
        assert_eq!(
            rejected("start", &[("mode", "bogus")]),
            "--mode must be interactive|headless"
        );
        rejected("start", &[]);
        // The A7 tightening: loop-mode and session reach a LOG header.
        rejected("start", &[("mode", "headless"), ("loop_mode", "a|b")]);
        rejected("start", &[("mode", "headless"), ("session", "a\nb")]);
        // A blank optional value is absent, not an error.
        assert_eq!(
            ok("start", &[("mode", "headless"), ("session", "  ")]),
            ValidatedEvent::Start {
                mode: "headless".into(),
                loop_mode: None,
                session: None
            }
        );
    }

    #[test]
    fn reconcile_parses_found_and_defaults_to_zero() {
        assert_eq!(ok("reconcile", &[]), ValidatedEvent::Reconcile { found: 0 });
        assert_eq!(
            ok("reconcile", &[("found", " 3 ")]),
            ValidatedEvent::Reconcile { found: 3 }
        );
        assert_eq!(
            rejected("reconcile", &[("found", "three")]),
            "--found must be an integer"
        );
    }

    #[test]
    fn dispatched_needs_a_known_item_a_step_and_an_integer_round() {
        assert_eq!(
            ok(
                "dispatched",
                &[
                    ("item", "I-01"),
                    ("step", "/gsd-plan-phase 1"),
                    ("round", "2"),
                    ("phase", "1")
                ]
            ),
            ValidatedEvent::Dispatched {
                item: "I-01".into(),
                step: "/gsd-plan-phase 1".into(),
                round: 2,
                phase: Some("1".into())
            }
        );
        assert_eq!(
            ok("dispatched", &[("item", "I-02"), ("step", "go")]),
            ValidatedEvent::Dispatched {
                item: "I-02".into(),
                step: "go".into(),
                round: 0,
                phase: None
            }
        );
        rejected("dispatched", &[("item", "I-01")]);
        rejected("dispatched", &[("item", "I-01"), ("step", "a|b")]);
        assert_eq!(
            rejected(
                "dispatched",
                &[("item", "I-01"), ("step", "go"), ("round", "x")]
            ),
            "--round must be an integer"
        );
        rejected(
            "dispatched",
            &[("item", "I-01"), ("step", "go"), ("phase", "1\n2")],
        );
    }

    #[test]
    fn known_item_rejects_run_and_unknown_ids_with_the_spike_message() {
        assert_eq!(
            rejected("dispatched", &[("item", "RUN"), ("step", "go")]),
            "--item 'RUN' is not an item in DIRECTION.md"
        );
        assert_eq!(
            rejected(
                "verified",
                &[("item", "I-99"), ("done_when", "met"), ("evidence", "x")]
            ),
            "--item 'I-99' is not an item in DIRECTION.md"
        );
        // No DIRECTION items at all: every item-level event is refused.
        let none: Vec<String> = Vec::new();
        assert!(validate_log(
            "needs-human",
            &args(&[("item", "I-01"), ("question", "why")]),
            &none
        )
        .is_err());
    }

    #[test]
    fn report_keeps_fifteen_lines_flags_truncation_and_reads_findings() {
        let long = (1..=16)
            .map(|n| format!("line {n}"))
            .collect::<Vec<_>>()
            .join("\n");
        let ValidatedEvent::Report {
            body,
            truncated,
            findings,
            ..
        } = ok(
            "report",
            &[("item", "I-01"), ("status", "done"), ("body", &long)],
        )
        else {
            panic!("not a report")
        };
        assert_eq!(body.len(), MAX_REPORT_LINES);
        assert!(truncated);
        assert_eq!(findings, None);

        let ValidatedEvent::Report {
            body,
            truncated,
            findings,
            ..
        } = ok(
            "report",
            &[
                ("item", "I-01"),
                ("status", "partial"),
                ("body", "FINDINGS: 4\ndetail\n\n"),
            ],
        )
        else {
            panic!("not a report")
        };
        assert_eq!(body, vec!["FINDINGS: 4".to_owned(), "detail".to_owned()]);
        assert!(!truncated);
        assert_eq!(findings, Some(4));

        rejected(
            "report",
            &[("item", "I-01"), ("status", "maybe"), ("body", "x")],
        );
        assert_eq!(
            rejected(
                "report",
                &[("item", "I-01"), ("status", "done"), ("body", "\n\n")]
            ),
            "report body (stdin) is empty; pipe the worker report verbatim"
        );
        rejected("report", &[("item", "I-01"), ("status", "done")]);
        rejected(
            "report",
            &[("item", "I-01"), ("status", "done"), ("body", "ok\u{1b}[31m")],
        );
        // A tab is the one control character a report may carry.
        ok(
            "report",
            &[("item", "I-01"), ("status", "done"), ("body", "a\tb")],
        );
    }

    #[test]
    fn verified_needs_a_done_when_verdict_and_a_one_line_evidence() {
        assert_eq!(
            ok(
                "verified",
                &[
                    ("item", "I-01"),
                    ("done_when", "not-met"),
                    ("evidence", "tests fail")
                ]
            ),
            ValidatedEvent::Verified {
                item: "I-01".into(),
                done_when: "not-met".into(),
                evidence: "tests fail".into()
            }
        );
        rejected(
            "verified",
            &[("item", "I-01"), ("done_when", "maybe"), ("evidence", "x")],
        );
        rejected("verified", &[("item", "I-01"), ("done_when", "met")]);
    }

    #[test]
    fn status_requires_a_rule_for_parked_and_an_outcome_unless_running() {
        assert_eq!(
            ok("status", &[("item", "I-01"), ("state", "running")]),
            ValidatedEvent::Status {
                item: "I-01".into(),
                state: ItemState::Running,
                rule: None,
                outcome: None
            }
        );
        assert_eq!(
            ok(
                "status",
                &[
                    ("item", "I-01"),
                    ("state", "parked"),
                    ("rule", "budget"),
                    ("outcome", "out of budget")
                ]
            ),
            ValidatedEvent::Status {
                item: "I-01".into(),
                state: ItemState::Parked,
                rule: Some("budget".into()),
                outcome: Some("out of budget".into())
            }
        );
        rejected(
            "status",
            &[("item", "I-01"), ("state", "bogus"), ("outcome", "x")],
        );
        rejected(
            "status",
            &[("item", "I-01"), ("state", "parked"), ("outcome", "x")],
        );
        rejected(
            "status",
            &[
                ("item", "I-01"),
                ("state", "parked"),
                ("rule", "bogus"),
                ("outcome", "x"),
            ],
        );
        // The tightening: a rule that is given is checked even when not parked.
        rejected(
            "status",
            &[("item", "I-01"), ("state", "running"), ("rule", "bogus")],
        );
        rejected("status", &[("item", "I-01"), ("state", "done")]);
        rejected(
            "status",
            &[("item", "I-01"), ("state", "done"), ("outcome", "a\nb")],
        );
    }

    #[test]
    fn needs_human_question_is_one_line_up_to_three_hundred() {
        ok(
            "needs-human",
            &[("item", "I-02"), ("question", &"q".repeat(300))],
        );
        rejected(
            "needs-human",
            &[("item", "I-02"), ("question", &"q".repeat(301))],
        );
        rejected("needs-human", &[("item", "I-02")]);
    }

    #[test]
    fn recovered_takes_a_known_action_and_a_reason() {
        assert_eq!(
            ok(
                "recovered",
                &[
                    ("item", "I-01"),
                    ("action", "resume"),
                    ("reason", "worker died")
                ]
            ),
            ValidatedEvent::Recovered {
                item: "I-01".into(),
                action: "resume".into(),
                reason: "worker died".into()
            }
        );
        rejected(
            "recovered",
            &[("item", "I-01"), ("action", "bogus"), ("reason", "x")],
        );
        rejected("recovered", &[("item", "I-01"), ("action", "done")]);
    }

    #[test]
    fn end_maps_each_reason_and_never_yields_crashed_or_stalled() {
        for (reason, status) in END_REASONS {
            assert_eq!(
                ok("end", &[("reason", reason)]),
                ValidatedEvent::End {
                    reason: (*reason).to_owned(),
                    status: status.clone()
                }
            );
            assert!(!matches!(status, RunStatus::Crashed | RunStatus::Stalled));
        }
        rejected("end", &[("reason", "crashed")]);
        rejected("end", &[("reason", "stalled")]);
        rejected("end", &[]);
    }

    #[test]
    fn unknown_event_is_refused() {
        let err = validate_log("explode", &LogArgs::default(), &[]).unwrap_err();
        assert!(err.0.contains("unknown event"), "{err}");
    }

    #[test]
    fn validate_hook_checks_only_values_that_reach_a_log_header() {
        let payload = |event: &str, session: &str| HookPayload {
            hook_event_name: Some(event.into()),
            session_id: Some(session.into()),
            ..HookPayload::default()
        };
        assert!(validate_hook(&payload("SessionStart", "abc-123")).is_ok());
        assert!(validate_hook(&payload("SessionStart", "a|b")).is_err());
        assert!(validate_hook(&payload("SessionEnd", "a\nb")).is_err());
        // A gate payload writes nothing derived from the payload, so it is never refused.
        assert!(validate_hook(&payload("PreToolUse", "a|b")).is_ok());
    }

    #[test]
    fn is_gate_needs_pretooluse_agent_or_task_and_no_agent_id() {
        let mut p = HookPayload {
            hook_event_name: Some("PreToolUse".into()),
            tool_name: Some("Agent".into()),
            ..HookPayload::default()
        };
        assert!(p.is_gate());
        p.tool_name = Some("Task".into());
        assert!(p.is_gate());
        p.agent_id = Some("sub-1".into());
        assert!(!p.is_gate());
        p.agent_id = None;
        p.tool_name = Some("Bash".into());
        assert!(!p.is_gate());
        p.tool_name = Some("Agent".into());
        p.hook_event_name = Some("PostToolUse".into());
        assert!(!p.is_gate());
    }

    #[test]
    fn parse_request_accepts_trailing_whitespace_and_a_missing_newline() {
        assert!(matches!(
            parse_request(br#"{"op":"show"}"#),
            Ok(Request::Show)
        ));
        assert!(matches!(
            parse_request(b"{\"op\":\"show\"}\n"),
            Ok(Request::Show)
        ));
        assert!(matches!(
            parse_request(b"{\"op\":\"show\"} \r\n\t\n"),
            Ok(Request::Show)
        ));
    }

    #[test]
    fn parse_request_refuses_an_unknown_op_a_bad_args_shape_and_a_stray_key() {
        for bad in [
            &br#"{"op":"set_terminal","status":"crashed"}"#[..],
            br#"{"op":"log","event":"start","args":5}"#,
            br#"{"op":"log","event":"start","args":[]}"#,
            br#"{"op":"log","event":"start","args":{"round":1}}"#,
            br#"{"op":"log","event":"start","args":{"bogus":"x"}}"#,
            br#"{"op":"hook","payload":{"tool_input":{}}}"#,
            br#"not json"#,
            b"",
        ] {
            let err = parse_request(bad).unwrap_err();
            assert!(
                err.0.starts_with("malformed request:"),
                "{err} for {}",
                String::from_utf8_lossy(bad)
            );
        }
        assert!(parse_request(br#"{"op":"log","event":"start","args":null}"#).is_ok());
        assert!(parse_request(br#"{"op":"log","event":"start"}"#).is_ok());
    }

    #[test]
    fn oversized_request_is_refused() {
        let big = vec![b' '; MAX_REQUEST_BYTES + 1];
        assert!(parse_request(&big).is_err());
    }

    #[test]
    fn response_json_omits_absent_fields() {
        assert_eq!(
            serde_json::to_string(&Response::ok()).unwrap(),
            r#"{"ok":true}"#
        );
        assert_eq!(
            serde_json::to_string(&Response::deny("cap")).unwrap(),
            r#"{"ok":true,"deny":"cap"}"#
        );
        assert_eq!(
            serde_json::to_string(&Response::rejected("no")).unwrap(),
            r#"{"ok":false,"error":"no"}"#
        );
        assert_eq!(
            serde_json::to_string(&Response::frontmatter("---\n")).unwrap(),
            r#"{"ok":true,"frontmatter":"---\n"}"#
        );
    }
}
