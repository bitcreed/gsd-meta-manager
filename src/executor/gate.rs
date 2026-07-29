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

    /// The observed 2.1.220 auth source: the subscription/OAuth path is alive.
    const SUBSCRIPTION: Option<&str> = Some(SUBSCRIPTION_API_KEY_SOURCE);
    /// A version at the tested maximum, so version is never the reason a
    /// capability or auth-path test refuses.
    const OBSERVED_VERSION: Option<&str> = Some("2.1.220");

    /// Build an `InitMessage` from its three gate-relevant fields.
    ///
    /// `None` means the field is **absent from the JSON entirely**, which is a
    /// different thing from present-and-empty and is exactly what the
    /// fail-closed cases need to distinguish.
    fn init(
        capabilities: Option<&[&str]>,
        version: Option<&str>,
        api_key_source: Option<&str>,
    ) -> InitMessage {
        let mut obj = serde_json::Map::new();
        obj.insert("type".to_string(), "system".into());
        obj.insert("subtype".to_string(), "init".into());
        obj.insert("session_id".to_string(), "s".into());
        obj.insert("permissionMode".to_string(), "dontAsk".into());
        if let Some(caps) = capabilities {
            let values: Vec<serde_json::Value> =
                caps.iter().map(|c| serde_json::Value::from(*c)).collect();
            obj.insert("capabilities".to_string(), values.into());
        }
        if let Some(version) = version {
            obj.insert("claude_code_version".to_string(), version.into());
        }
        if let Some(source) = api_key_source {
            obj.insert("apiKeySource".to_string(), source.into());
        }

        let raw = serde_json::Value::Object(obj).to_string();
        match parse_line(&raw) {
            Envelope::Parsed {
                msg: StreamMessage::System(SystemMessage::Init(init)),
                ..
            } => *init,
            other => panic!("fixture did not parse as a system/init: {other:?}"),
        }
    }

    /// A healthy 2.1.220 init, varied one field at a time by the tests.
    fn healthy() -> InitMessage {
        init(
            Some(&REQUIRED_CAPABILITIES),
            OBSERVED_VERSION,
            SUBSCRIPTION,
        )
    }

    fn validate(init: &InitMessage) -> Result<GateOutcome, CapabilityError> {
        validate_first_init(init, PermissionMode::DontAsk)
    }

    // ========================================================================
    // Capabilities — a subset check on exact byte-equal strings (D-06)
    // ========================================================================

    #[test]
    fn a_cli_advertising_every_required_capability_passes() {
        let outcome = validate(&healthy()).expect("gate should pass");
        assert_eq!(outcome.claude_code_version.as_deref(), Some("2.1.220"));
        assert_eq!(outcome.api_key_source.as_deref(), Some("none"));
        assert_eq!(outcome.session_id.as_deref(), Some("s"));
        assert_eq!(outcome.capabilities.len(), REQUIRED_CAPABILITIES.len());
        assert_eq!(outcome.permission_mode.as_deref(), Some("dontAsk"));
        assert!(
            outcome.permission_mode_confirmed,
            "the init reflects the flag, so a matching mode is a free confirmation it took effect (D-15)"
        );
    }

    #[test]
    fn extra_capabilities_do_not_refuse_a_working_cli() {
        let mut caps: Vec<&str> = REQUIRED_CAPABILITIES.to_vec();
        caps.push("some_future_capability_v2");
        assert!(
            validate(&init(Some(&caps), OBSERVED_VERSION, SUBSCRIPTION)).is_ok(),
            "the check must be a subset check, not an equality check"
        );
    }

    #[test]
    fn a_missing_capability_is_named_in_the_error() {
        let err = validate(&init(
            Some(&["interrupt_receipt_v1", "msg_lifecycle_v1"]),
            OBSERVED_VERSION,
            SUBSCRIPTION,
        ))
        .expect_err("gate should refuse");
        let rendered = err.to_string();
        assert!(
            rendered.contains("interrupt_cancel_queued_v1"),
            "the error must name the missing capability, got: {rendered}"
        );
    }

    #[test]
    fn two_missing_capabilities_are_both_named() {
        let err = validate(&init(
            Some(&["interrupt_receipt_v1"]),
            OBSERVED_VERSION,
            SUBSCRIPTION,
        ))
        .expect_err("gate should refuse");
        let rendered = err.to_string();
        for expected in ["interrupt_cancel_queued_v1", "msg_lifecycle_v1"] {
            assert!(
                rendered.contains(expected),
                "the error must name every missing capability; {expected} is absent from: {rendered}"
            );
        }
    }

    #[test]
    fn an_empty_capabilities_array_is_refused_naming_every_requirement() {
        let err = validate(&init(Some(&[]), OBSERVED_VERSION, SUBSCRIPTION))
            .expect_err("gate should refuse");
        match &err {
            CapabilityError::MissingCapabilities { missing, observed } => {
                assert_eq!(
                    missing.len(),
                    REQUIRED_CAPABILITIES.len(),
                    "an empty array is not 'no requirements', got missing: {missing:?}"
                );
                assert!(
                    observed.is_empty(),
                    "observed should be empty, got: {observed:?}"
                );
            }
            other => panic!("expected a missing-capability refusal, got: {other:?}"),
        }
    }

    #[test]
    fn an_absent_capabilities_field_is_refused_exactly_like_an_empty_one() {
        let absent = validate(&init(None, OBSERVED_VERSION, SUBSCRIPTION))
            .expect_err("an absent capabilities field must refuse");
        let empty = validate(&init(Some(&[]), OBSERVED_VERSION, SUBSCRIPTION))
            .expect_err("an empty capabilities array must refuse");
        assert_eq!(
            absent, empty,
            "an absent array and an empty array must fail closed identically (TRANS-04)"
        );
    }

    #[test]
    fn capability_matching_is_exact_byte_equality_and_never_case_folded() {
        let shouted = [
            "INTERRUPT_RECEIPT_V1",
            "INTERRUPT_CANCEL_QUEUED_V1",
            "MSG_LIFECYCLE_V1",
        ];
        let err = validate(&init(Some(&shouted), OBSERVED_VERSION, SUBSCRIPTION))
            .expect_err("a capability differing only in case must not be accepted");
        match &err {
            CapabilityError::MissingCapabilities { missing, .. } => assert_eq!(
                missing.len(),
                REQUIRED_CAPABILITIES.len(),
                "matching is exact byte equality: no case folding, no Unicode normalization"
            ),
            other => panic!("expected a missing-capability refusal, got: {other:?}"),
        }
    }

    #[test]
    fn a_capability_matching_only_as_a_prefix_or_substring_is_not_accepted() {
        let near_misses = [
            "interrupt_receipt_v1_extended",
            "x_interrupt_cancel_queued_v1",
            "msg_lifecycle_v10",
        ];
        let err = validate(&init(Some(&near_misses), OBSERVED_VERSION, SUBSCRIPTION))
            .expect_err("substring and prefix matches must not satisfy the requirement");
        match &err {
            CapabilityError::MissingCapabilities { missing, .. } => assert_eq!(
                missing.len(),
                REQUIRED_CAPABILITIES.len(),
                "the match is whole-string equality, got missing: {missing:?}"
            ),
            other => panic!("expected a missing-capability refusal, got: {other:?}"),
        }
    }

    // ========================================================================
    // Version floor — componentwise, fail-closed (D-07, Pitfall F)
    // ========================================================================

    #[test]
    fn a_version_below_the_floor_is_refused_naming_the_observed_version() {
        let err = validate(&init(
            Some(&REQUIRED_CAPABILITIES),
            Some("2.1.213"),
            SUBSCRIPTION,
        ))
        .expect_err("a version below the floor must refuse");
        let rendered = err.to_string();
        assert!(
            rendered.contains("2.1.213"),
            "the error must name the observed version, got: {rendered}"
        );
        assert!(
            rendered.contains("2.1.214"),
            "the error must name the floor it failed, got: {rendered}"
        );
    }

    #[test]
    fn a_version_below_the_floor_on_an_earlier_component_is_also_refused() {
        for below in ["1.9.999", "2.0.999"] {
            assert!(
                validate(&init(Some(&REQUIRED_CAPABILITIES), Some(below), SUBSCRIPTION)).is_err(),
                "{below} is below the floor componentwise and must refuse"
            );
        }
    }

    #[test]
    fn a_version_exactly_at_the_floor_passes() {
        assert!(
            validate(&init(
                Some(&REQUIRED_CAPABILITIES),
                Some("2.1.214"),
                SUBSCRIPTION
            ))
            .is_ok(),
            "the floor itself is supported"
        );
    }

    #[test]
    fn a_version_above_the_tested_maximum_passes_rather_than_refusing() {
        // Feature detection is by capabilities[], so a newer CLI is never
        // refused on version alone — it warns and proceeds (D-07).
        for newer in ["2.1.221", "2.2.0", "3.0.0"] {
            assert!(
                validate(&init(Some(&REQUIRED_CAPABILITIES), Some(newer), SUBSCRIPTION)).is_ok(),
                "{newer} is above the tested maximum and must warn-and-proceed, not refuse"
            );
        }
    }

    #[test]
    fn an_absent_version_field_is_refused_rather_than_assumed_new_enough() {
        let err = validate(&init(Some(&REQUIRED_CAPABILITIES), None, SUBSCRIPTION))
            .expect_err("an absent version must fail closed (Pitfall F)");
        assert!(
            matches!(err, CapabilityError::VersionUnreadable { .. }),
            "expected an unreadable-version refusal, got: {err:?}"
        );
    }

    #[test]
    fn an_unparseable_version_string_is_refused() {
        for garbage in ["", "2.1", "2.1.x", "two.one.twenty", "2.1.220.1", "v2.1.220"] {
            let err = validate(&init(
                Some(&REQUIRED_CAPABILITIES),
                Some(garbage),
                SUBSCRIPTION,
            ))
            .unwrap_err();
            assert!(
                matches!(err, CapabilityError::VersionUnreadable { .. }),
                "{garbage:?} is not a readable version and must fail closed, got: {err:?}"
            );
        }
    }

    // ========================================================================
    // The auth-path regression guard (D-08)
    // ========================================================================

    #[test]
    fn an_auth_source_other_than_the_subscription_value_fires_the_guard() {
        let err = validate(&init(
            Some(&REQUIRED_CAPABILITIES),
            OBSERVED_VERSION,
            Some("ANTHROPIC_API_KEY"),
        ))
        .expect_err("a changed auth path must refuse");
        let rendered = err.to_string();
        assert!(
            rendered.contains("ANTHROPIC_API_KEY"),
            "the error must name the observed source, got: {rendered}"
        );
    }

    #[test]
    fn an_absent_auth_source_field_fires_the_guard_rather_than_passing() {
        let err = validate(&init(Some(&REQUIRED_CAPABILITIES), OBSERVED_VERSION, None))
            .expect_err("an absent auth source is the silent regression the guard exists to catch");
        assert!(
            matches!(err, CapabilityError::AuthPathChanged { observed: None }),
            "expected an auth-path refusal carrying an absent observation, got: {err:?}"
        );
    }

    // ========================================================================
    // Permission mode — a free confirmation, never a refusal (D-15)
    // ========================================================================

    #[test]
    fn a_permission_mode_mismatch_is_recorded_rather_than_refused() {
        let mut init = healthy();
        init.permission_mode = Some("default".to_string());
        let outcome = validate(&init)
            .expect("a permission-mode mismatch is a diagnostic, not a capability refusal");
        assert!(
            !outcome.permission_mode_confirmed,
            "the mismatch must be observable on the outcome"
        );
        assert_eq!(outcome.permission_mode.as_deref(), Some("default"));
    }
}
