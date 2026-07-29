//! The `system/init` capability gate.
//!
//! A **pure function over a parsed message with no I/O**, so the whole gate can
//! be tested against synthetic init payloads without spawning anything (D-24).
//!
//! Feature detection is by the `capabilities[]` array, never by comparing
//! version strings (D-06). The check is a *subset* check, so a CLI advertising
//! extra capabilities passes; a renamed capability refuses a working CLI, which
//! is the intended fail-closed direction.
//!
//! The gate fires on the **first** `system/init` only. Every queued turn emits
//! its own init, and re-running the gate there would turn a start-time refusal
//! into a mid-run abort — precisely the "mysteriously context-free agent
//! mid-run" failure the guard exists to prevent (D-30). Enforcing that ordering
//! is the caller's job; this function is stateless.
//!
//! Plan 15-03 owns this file from here and adds the version floor (D-07) and
//! the `--bare` auth-source regression guard (D-08) beside the capability check.

use crate::error::CapabilityError;
use crate::executor::stream_json::InitMessage;

/// Capabilities a run requires before any turn may begin.
///
/// Observed on 2.1.220 as exactly these three, stable across every probe run:
/// `interrupt_receipt_v1` backs `interrupt()`, `interrupt_cancel_queued_v1`
/// backs the `still_queued` accounting, and `msg_lifecycle_v1` backs the
/// replay-echo delivery ack.
pub const REQUIRED_CAPABILITIES: [&str; 3] = [
    "interrupt_receipt_v1",
    "interrupt_cancel_queued_v1",
    "msg_lifecycle_v1",
];

/// What the gate validated, recorded onto the run handle.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GateFacts {
    /// The session UUID the CLI reported.
    pub session_id: Option<String>,
    /// Everything the CLI advertised.
    pub capabilities: Vec<String>,
    /// The observed CLI version, recorded so Phase 16 can journal it without a
    /// signature change (D-07).
    pub claude_code_version: Option<String>,
    /// `"none"` means the subscription/OAuth path is alive (D-08).
    pub api_key_source: Option<String>,
    /// Confirms `--permission-mode` took effect (D-15).
    pub permission_mode: Option<String>,
}

/// Validate a first `system/init`.
///
/// Returns the facts to record, or a typed error **naming every missing
/// capability**. An absent or empty `capabilities` array reaches here as an
/// empty vector and is refused with the full required set named — an empty
/// array is never read as "no requirements" (TRANS-04).
pub fn check_init(init: &InitMessage) -> Result<GateFacts, CapabilityError> {
    let missing: Vec<String> = REQUIRED_CAPABILITIES
        .iter()
        .filter(|required| !init.capabilities.iter().any(|c| c == *required))
        .map(|required| (*required).to_string())
        .collect();

    if !missing.is_empty() {
        return Err(CapabilityError::MissingCapabilities {
            missing,
            observed: init.capabilities.clone(),
        });
    }

    Ok(GateFacts {
        session_id: init.session_id.clone(),
        capabilities: init.capabilities.clone(),
        claude_code_version: init.claude_code_version.clone(),
        api_key_source: init.api_key_source.clone(),
        permission_mode: init.permission_mode.clone(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::stream_json::{parse_line, Envelope, StreamMessage, SystemMessage};

    fn init_with(capabilities: &[&str]) -> InitMessage {
        let caps = capabilities
            .iter()
            .map(|c| format!("\"{c}\""))
            .collect::<Vec<_>>()
            .join(",");
        let raw = format!(
            r#"{{"type":"system","subtype":"init","session_id":"s","capabilities":[{caps}],"apiKeySource":"none","claude_code_version":"2.1.220"}}"#
        );
        match parse_line(&raw) {
            Envelope::Parsed {
                msg: StreamMessage::System(SystemMessage::Init(init)),
                ..
            } => *init,
            other => panic!("fixture did not parse as a system/init: {other:?}"),
        }
    }

    #[test]
    fn a_cli_advertising_every_required_capability_passes() {
        let facts = check_init(&init_with(&REQUIRED_CAPABILITIES)).expect("gate should pass");
        assert_eq!(facts.claude_code_version.as_deref(), Some("2.1.220"));
        assert_eq!(facts.api_key_source.as_deref(), Some("none"));
    }

    #[test]
    fn extra_capabilities_do_not_refuse_a_working_cli() {
        let mut caps: Vec<&str> = REQUIRED_CAPABILITIES.to_vec();
        caps.push("some_future_capability_v2");
        assert!(
            check_init(&init_with(&caps)).is_ok(),
            "the check must be a subset check, not an equality check"
        );
    }

    #[test]
    fn a_missing_capability_is_named_in_the_error() {
        let err = check_init(&init_with(&["interrupt_receipt_v1", "msg_lifecycle_v1"]))
            .expect_err("gate should refuse");
        let rendered = err.to_string();
        assert!(
            rendered.contains("interrupt_cancel_queued_v1"),
            "the error must name the missing capability, got: {rendered}"
        );
    }

    #[test]
    fn an_empty_capabilities_array_is_refused_naming_every_requirement() {
        let err = check_init(&init_with(&[])).expect_err("gate should refuse");
        match &err {
            CapabilityError::MissingCapabilities { missing, observed } => {
                assert_eq!(
                    missing.len(),
                    REQUIRED_CAPABILITIES.len(),
                    "an empty array is not 'no requirements', got missing: {missing:?}"
                );
                assert!(observed.is_empty(), "observed should be empty, got: {observed:?}");
            }
        }
    }
}
