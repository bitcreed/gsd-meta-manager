//! Pure decision functions over push refs and the reserved branch namespace.
//!
//! **No I/O, no processes, no network.** Everything here is a total function of
//! its arguments, which is what makes the push step's namespace decision
//! exhaustively unit-testable without a repository, a child process or a
//! credential - the same property `journal::is_plain_path_component` was written
//! for, and for the same reason: a decision that needs the world to answer cannot
//! be tested against the world's hostile cases.
//!
//! What survives Phase 31 is what `launcher/push.rs` calls: the namespace shape
//! rules ([`validate_namespace`], [`default_namespace`]), the destination
//! classifier ([`classify_push_ref`]), the park-reason taxonomy ([`ParkReason`])
//! and [`EnvelopePolicy`] with its pull-request caps. The argv classifier, the
//! tokenizer and everything that judged a shell command line went with the
//! Phase 19 guard.

use crate::config::{CredentialSource, DriverOptIn};

/// The reserved push namespace root (D-05).
///
/// GitHub Copilot's precedent is a flat `copilot/` prefix. The extra `<alias>`
/// segment is added because this tool drives N projects from one machine and one
/// credential, so an errant ref names the project it came from without a lookup.
pub const DEFAULT_NAMESPACE_ROOT: &str = "refs/heads/gsd-auto/";

/// The ref prefix every branch namespace must sit under.
const REFS_HEADS: &str = "refs/heads/";

/// First segments a namespace may never claim, at any casing git treats as
/// distinct. `HEAD` is included because `refs/heads/HEAD/` is a working shape
/// that would make every push look like a detached-head update.
const RESERVED_FIRST_SEGMENTS: &[&str] = &["main", "master", "HEAD"];

/// D-24's park reason for a push whose destination is outside the namespace.
pub const REASON_PUSH_OUTSIDE_NAMESPACE: &str = "push_outside_namespace";

/// D-24's park reason for a destructive git operation the envelope refuses.
pub const REASON_FORCE_PUSH_BLOCKED: &str = "force_push_blocked";

/// D-24's park reason for a command that would make an enforcement layer
/// unreachable.
pub const REASON_HOOK_BYPASS_BLOCKED: &str = "hook_bypass_blocked";

/// D-24's park reason for a credential found by the pre-push scan (SAFE-03).
pub const REASON_SECRET_DETECTED: &str = "secret_detected";

/// D-24's park reason for a PR attempt beyond the configured cap (SAFE-06).
pub const REASON_PR_CAP_EXCEEDED: &str = "pr_cap_exceeded";

/// D-24's park reason for a run that has no credential to push with (SAFE-05).
pub const REASON_CREDENTIAL_UNAVAILABLE: &str = "credential_unavailable";

/// D-24's park reason for an envelope input the enforcement point cannot read.
pub const REASON_ENVELOPE_ASSERTION_FAILED: &str = "envelope_assertion_failed";

/// The namespace an alias gets when the project configures none.
///
/// Always a prefix, always ending in `/`: see [`validate_namespace`] for why the
/// trailing slash is load-bearing rather than cosmetic.
pub fn default_namespace(alias: &str) -> String {
    format!("{DEFAULT_NAMESPACE_ROOT}{alias}/")
}

/// A configured namespace, or `None` if its shape would weaken the control.
///
/// D-05's rules, each of which exists to refuse a specific way of accidentally
/// disabling the boundary:
///
/// - **non-empty** and **under `refs/heads/`** — a namespace that is not a
///   branch prefix does not constrain a branch push at all;
/// - **at least two `/`-separated segments below `refs/heads/`** — a single
///   segment like `refs/heads/x/` is legal but leaves no room for the per-alias
///   segment the whole scheme is built on;
/// - **the first segment below `refs/heads/` is not `main`, `master` or
///   `HEAD`** — the branches the envelope exists to protect;
/// - **it ends in `/`** — so it is a *prefix* and not one branch. Without this,
///   a user configuring `refs/heads/` would silently allow every branch, which
///   is disabling the control by typo.
///
/// Returns the candidate unchanged on success. It deliberately does **not**
/// normalise: a namespace that has to be repaired before it is usable is a
/// namespace whose written form differs from its effect, and the whole point of
/// the shape rules is that what the user typed is what applies.
pub fn validate_namespace(candidate: &str) -> Option<String> {
    let below = candidate.strip_prefix(REFS_HEADS)?;
    if !below.ends_with('/') {
        return None;
    }
    let segments: Vec<&str> = below.trim_end_matches('/').split('/').collect();
    if segments.len() < 2 {
        return None;
    }
    if segments.iter().any(|segment| segment.is_empty()) {
        return None;
    }
    if RESERVED_FIRST_SEGMENTS.contains(&segments[0]) {
        return None;
    }
    Some(candidate.to_string())
}

/// What the envelope decided about one push destination.
///
/// `reason` is `&'static str` on purpose: every reason is a member of D-24's
/// fixed taxonomy, so a caller cannot invent one at the call site and a reader
/// can find every producer of a given reason by grepping the constant.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PushVerdict {
    /// The destination is inside the reserved namespace.
    Allow,
    /// The destination is outside it, or could not be read.
    Refuse {
        /// A member of D-24's reason taxonomy.
        reason: &'static str,
        /// The destination ref as git reported it, for a legible message.
        ref_name: String,
    },
}

/// Classify one push destination against a namespace prefix.
///
/// A plain prefix test, and that is the whole decision. It is written as a
/// separate function rather than inlined at the hook because this is the
/// sentence the tracer test flips to prove the refusal is load-bearing: making
/// this return [`PushVerdict::Allow`] unconditionally must turn the suite red.
pub fn classify_push_ref(dest_ref: &str, namespace: &str) -> PushVerdict {
    if dest_ref.starts_with(namespace) {
        PushVerdict::Allow
    } else {
        PushVerdict::Refuse {
            reason: REASON_PUSH_OUTSIDE_NAMESPACE,
            ref_name: dest_ref.to_string(),
        }
    }
}

/// The park-reason taxonomy this phase owns (D-24).
///
/// Modelled on [`crate::journal::JournalEvent::Diagnostic`]'s `code` field,
/// documented there as *"a short stable identifier for the condition"* — which
/// is exactly what a park reason is. Two conventions for one idea is how a
/// later reader ends up grepping for the wrong string.
///
/// **Phase 19 owns *producing* these reasons. Phase 20 owns what happens to a
/// run that carries one.** No resume, retry or backoff logic belongs next to
/// this type: a reason that also decides the recovery has to be edited every
/// time the recovery changes, and then the identifier is no longer stable.
///
/// The set is closed on purpose. When a refusal does not have its own reason —
/// `git stash`, `update-ref`, `filter-branch` — it is reported under
/// [`ParkReason::ForcePushBlocked`] as the destructive-git family, and the
/// specific command travels in the verdict's `detail`. That is stated here
/// rather than left for a reader to infer from a surprising string, because
/// inventing an eighth reason at a call site is how a taxonomy stops being one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ParkReason {
    /// A push whose destination is outside the reserved namespace (D-05).
    PushOutsideNamespace,
    /// A destructive git operation: force push, delete, or history rewrite.
    ForcePushBlocked,
    /// A command that would make an enforcement layer unreachable.
    HookBypassBlocked,
    /// The pre-push scan found a credential (SAFE-03).
    SecretDetected,
    /// A PR attempt beyond the configured cap (SAFE-06).
    PrCapExceeded,
    /// The run has no credential configured and cannot push (SAFE-05).
    CredentialUnavailable,
    /// An envelope input the enforcement point could not read.
    EnvelopeAssertionFailed,
}

impl ParkReason {
    /// The stable snake_case identifier a later reader greps for.
    ///
    /// Every arm returns one of the `REASON_*` constants above rather than a
    /// fresh literal, so the constant and the enum can never disagree and
    /// `grep REASON_FORCE_PUSH_BLOCKED` finds every producer.
    pub fn as_str(&self) -> &'static str {
        match self {
            ParkReason::PushOutsideNamespace => REASON_PUSH_OUTSIDE_NAMESPACE,
            ParkReason::ForcePushBlocked => REASON_FORCE_PUSH_BLOCKED,
            ParkReason::HookBypassBlocked => REASON_HOOK_BYPASS_BLOCKED,
            ParkReason::SecretDetected => REASON_SECRET_DETECTED,
            ParkReason::PrCapExceeded => REASON_PR_CAP_EXCEEDED,
            ParkReason::CredentialUnavailable => REASON_CREDENTIAL_UNAVAILABLE,
            ParkReason::EnvelopeAssertionFailed => REASON_ENVELOPE_ASSERTION_FAILED,
        }
    }
}

/// The default rolling-24h PR cap: **3**.
///
/// **Arbitrary, and recorded as arbitrary rather than presented as derived.**
/// PITFALLS suggests it explicitly as a starting number, not as a measurement.
/// A named constant with the reason beside it is what stops a later reader from
/// reverse-engineering a justification that never existed.
pub const DEFAULT_PR_CAP_PER_24H: u32 = 3;

/// The default per-run PR cap: **1**. Arbitrary in the same way as
/// [`DEFAULT_PR_CAP_PER_24H`], and for the same recorded reason.
pub const DEFAULT_PR_CAP_PER_RUN: u32 = 1;

/// Every envelope setting for one alias, with every default already decided.
///
/// **`Default` is deliberately NOT derived**, following the lesson
/// `config.rs:109-117` records for `Preferences::driver_max_concurrent`: a
/// derived `Default` would make each cap `u32::default()`, which is `0`, which
/// here means *"no PR may ever be opened"* — a total denial of service wearing
/// the costume of a default. The constants above are the defaults, and
/// [`EnvelopePolicy::resolve`] is the only place they are applied.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EnvelopePolicy {
    /// The validated namespace every push from this alias must sit under.
    pub namespace: String,
    /// PRs allowed in a rolling 24 hours.
    pub pr_cap_per_24h: u32,
    /// PRs allowed in a single run.
    pub pr_cap_per_run: u32,
    /// Where the run's credential comes from, or `None` for **no credential**,
    /// which is the default and means every push fails closed (D-18).
    pub credential: Option<CredentialSource>,
}

impl EnvelopePolicy {
    /// Resolve one project's envelope settings — **the only place an envelope
    /// default is decided** (D-30).
    ///
    /// That is the whole point of the function existing: with four
    /// `Option` fields and three consumers, a default applied at the point of
    /// use is a default applied differently at each point of use, and the first
    /// symptom is a run confined to a namespace the preview did not show.
    ///
    /// **An invalid configured namespace degrades to the safe default and
    /// warns; it never widens the control.** A user who types `refs/heads/` has
    /// written something that would allow every branch, and honouring it would
    /// be disabling the boundary by typo. Falling back is the only direction to
    /// be wrong in, and the warning is what keeps the fallback from being
    /// silent.
    pub fn resolve(alias: &str, opt_in: &DriverOptIn) -> EnvelopePolicy {
        let namespace = match opt_in.branch_namespace.as_deref() {
            Some(configured) => match validate_namespace(configured) {
                Some(valid) => valid,
                None => {
                    tracing::warn!(
                        alias,
                        configured,
                        "the configured branch namespace fails the shape rules that keep it a \
                         boundary, so the default applies instead",
                    );
                    default_namespace(alias)
                }
            },
            None => default_namespace(alias),
        };

        EnvelopePolicy {
            namespace,
            pr_cap_per_24h: opt_in.pr_cap_per_24h.unwrap_or(DEFAULT_PR_CAP_PER_24H),
            pr_cap_per_run: opt_in.pr_cap_per_run.unwrap_or(DEFAULT_PR_CAP_PER_RUN),
            credential: opt_in.credential.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A `DriverOptIn` with every envelope field absent — the shape a
    /// pre-Phase-19 config deserialises to.
    fn bare_opt_in() -> DriverOptIn {
        DriverOptIn {
            opted_in_at: "2026-08-18T00:00:00Z".to_string(),
            claude_md_digest: None,
            // Deliberately empty: this fixture is the pre-Phase-19 shape, and
            // these tests are about envelope field resolution rather than the
            // prompt-input disclosure.
            prompt_inputs: Vec::new(),
            extra: Default::default(),
            branch_namespace: None,
            credential: None,
            pr_cap_per_24h: None,
            pr_cap_per_run: None,
        }
    }

    #[test]
    fn an_unconfigured_project_resolves_to_the_default_namespace_and_the_default_caps() {
        let policy = EnvelopePolicy::resolve("demo", &bare_opt_in());

        assert_eq!(policy.namespace, "refs/heads/gsd-auto/demo/");
        assert_eq!(
            policy.pr_cap_per_24h, DEFAULT_PR_CAP_PER_24H,
            "a derived `Default` would make this 0, which means no PR may ever be opened \
             (config.rs:109-117's lesson, applied to a cap)"
        );
        assert_eq!(policy.pr_cap_per_run, DEFAULT_PR_CAP_PER_RUN);
        assert!(
            policy.credential.is_none(),
            "no credential is the default, and it must never resolve to the user's ambient \
             one — SAFE-05 names that substitution as the failure"
        );
    }

    #[test]
    fn pr_caps_default_to_three_and_one_from_an_absent_key_and_from_an_explicit_none() {
        // The two arms `driver_max_concurrent_defaults_to_one_from_default_and_from_empty_json`
        // established: the struct path and the JSON path can disagree, and only
        // exercising both catches it.
        let from_none = EnvelopePolicy::resolve("demo", &bare_opt_in());
        assert_eq!(from_none.pr_cap_per_24h, 3);
        assert_eq!(from_none.pr_cap_per_run, 1);

        let from_json: DriverOptIn = serde_json::from_str(
            r#"{"opted_in_at":"2026-08-18T00:00:00Z","claude_md_digest":null}"#,
        )
        .expect("a pre-Phase-19 opt-in record must still deserialise");
        let from_absent_key = EnvelopePolicy::resolve("demo", &from_json);
        assert_eq!(
            from_absent_key.pr_cap_per_24h, 3,
            "an absent key and an explicit `None` must resolve identically, or an old config \
             silently gets a different cap from a new one"
        );
        assert_eq!(from_absent_key.pr_cap_per_run, 1);
    }

    #[test]
    fn an_invalid_configured_namespace_falls_back_to_the_default_rather_than_widening() {
        for hostile in [
            "refs/heads/",
            "refs/heads/main/",
            "refs/heads/x/",
            "gsd-auto/demo/",
            "refs/heads/gsd-auto/demo",
        ] {
            let opt_in = DriverOptIn {
                branch_namespace: Some(hostile.to_string()),
                ..bare_opt_in()
            };
            assert_eq!(
                EnvelopePolicy::resolve("demo", &opt_in).namespace,
                "refs/heads/gsd-auto/demo/",
                "a configured namespace that fails the shape rules must degrade to the \
                 default; honouring {hostile:?} would disable the boundary by typo"
            );
        }
    }

    #[test]
    fn a_valid_configured_namespace_is_honoured_verbatim() {
        let opt_in = DriverOptIn {
            branch_namespace: Some("refs/heads/bots/nightly/".to_string()),
            ..bare_opt_in()
        };
        assert_eq!(
            EnvelopePolicy::resolve("demo", &opt_in).namespace,
            "refs/heads/bots/nightly/",
            "a namespace that passes the shape rules applies exactly as written — the \
             validator deliberately does not normalise"
        );
    }

    #[test]
    fn every_park_reason_has_a_distinct_stable_snake_case_identifier() {
        let all = [
            ParkReason::PushOutsideNamespace,
            ParkReason::ForcePushBlocked,
            ParkReason::HookBypassBlocked,
            ParkReason::SecretDetected,
            ParkReason::PrCapExceeded,
            ParkReason::CredentialUnavailable,
            ParkReason::EnvelopeAssertionFailed,
        ];
        let rendered: Vec<&str> = all.iter().map(|reason| reason.as_str()).collect();

        assert_eq!(
            rendered,
            vec![
                "push_outside_namespace",
                "force_push_blocked",
                "hook_bypass_blocked",
                "secret_detected",
                "pr_cap_exceeded",
                "credential_unavailable",
                "envelope_assertion_failed",
            ],
            "these seven strings are what Phase 20 reads out of a parked run; changing one \
             is a wire-format change, not a rename"
        );
    }

    #[test]
    fn the_default_namespace_of_any_plain_alias_satisfies_the_shape_rules() {
        for alias in ["demo", "gsd-meta-manager", "a.b_c-1"] {
            let namespace = default_namespace(alias);
            assert_eq!(
                validate_namespace(&namespace),
                Some(namespace.clone()),
                "the default namespace must pass the validator it ships with"
            );
        }
    }

    #[test]
    fn a_namespace_that_would_disable_the_control_is_refused() {
        for hostile in [
            "",
            "refs/heads/",
            "refs/heads",
            "gsd-auto/demo/",
            "refs/tags/gsd-auto/demo/",
            "refs/heads/demo/",
            "refs/heads/gsd-auto/demo",
            "refs/heads/main/demo/",
            "refs/heads/master/demo/",
            "refs/heads/HEAD/demo/",
            "refs/heads//demo/",
        ] {
            assert!(
                validate_namespace(hostile).is_none(),
                "{hostile:?} must not be accepted as a push namespace"
            );
        }
    }

    #[test]
    fn a_namespace_with_two_segments_below_refs_heads_is_accepted() {
        for ok in [
            "refs/heads/gsd-auto/demo/",
            "refs/heads/bots/nightly/",
            "refs/heads/bots/nightly/deep/",
        ] {
            assert_eq!(validate_namespace(ok), Some(ok.to_string()));
        }
    }

    #[test]
    fn only_refs_inside_the_namespace_are_allowed() {
        let namespace = default_namespace("demo");

        assert_eq!(
            classify_push_ref("refs/heads/gsd-auto/demo/tracer", &namespace),
            PushVerdict::Allow
        );

        for outside in [
            "refs/heads/main",
            "refs/heads/master",
            "refs/heads/gsd-auto/other/tracer",
            "refs/heads/gsd-auto-demo/tracer",
            "refs/tags/v1.0.0",
            "HEAD",
            "",
        ] {
            assert_eq!(
                classify_push_ref(outside, &namespace),
                PushVerdict::Refuse {
                    reason: REASON_PUSH_OUTSIDE_NAMESPACE,
                    ref_name: outside.to_string(),
                },
                "{outside:?} is outside {namespace} and must be refused"
            );
        }
    }

    #[test]
    fn a_one_character_difference_at_the_prefix_boundary_flips_the_verdict() {
        // The edge SAFE-01's flagged assumption names: a ref one character
        // outside the prefix is refused, a ref exactly at it is allowed.
        let namespace = default_namespace("demo");
        assert_eq!(
            classify_push_ref("refs/heads/gsd-auto/demo/x", &namespace),
            PushVerdict::Allow
        );
        assert!(matches!(
            classify_push_ref("refs/heads/gsd-auto/demox", &namespace),
            PushVerdict::Refuse { .. }
        ));
    }
}
