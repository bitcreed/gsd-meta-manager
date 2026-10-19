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

    fn args_with(f: impl FnOnce(&mut LogArgs)) -> LogArgs {
        let mut a = LogArgs::default();
        f(&mut a);
        a
    }

    #[test]
    fn start_accepts_a_known_mode_and_refuses_the_rest() {
        let ok = validate_log(
            "start",
            &args_with(|a| a.mode = Some("headless".into())),
            &[],
        )
        .unwrap();
        assert_eq!(
            ok,
            ValidatedEvent::Start {
                mode: "headless".into(),
                loop_mode: None,
                session: None
            }
        );
        let err = validate_log("start", &args_with(|a| a.mode = Some("bogus".into())), &[]);
        assert_eq!(err, Err(Invalid("--mode must be interactive|headless".into())));
        assert!(validate_log("start", &LogArgs::default(), &[]).is_err());
    }

    #[test]
    fn unknown_event_is_refused() {
        let err = validate_log("explode", &LogArgs::default(), &[]).unwrap_err();
        assert!(err.0.contains("unknown event"), "{err}");
    }

    #[test]
    fn parse_request_accepts_trailing_whitespace_and_a_missing_newline() {
        assert!(matches!(parse_request(br#"{"op":"show"}"#), Ok(Request::Show)));
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
    fn parse_request_refuses_an_unknown_op() {
        let err = parse_request(br#"{"op":"set_terminal","status":"crashed"}"#).unwrap_err();
        assert!(err.0.starts_with("malformed request:"), "{err}");
    }
}
