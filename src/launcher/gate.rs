//! The dispatch and gap-round gate (Phase 28; LNCH-03, D-12, D-14).
//!
//! [`decide_dispatch`] is the pure decision the launcher installs on the embedded
//! store with `RunStore::set_gate`. The store calls it, under its own mutex, for
//! every driver-level `PreToolUse Agent|Task` payload (no `agent_id`), and the
//! answer is `Allow` or a `Deny` whose reason the hook client prefixes with
//! `LAUNCHER CAP:` and hands back to Claude Code. Ported from the spike-003
//! launcher's `gate`, in the spike's order:
//!
//! 1. a tripped cap denies everything (`run stopped by launcher cap <cap>`), from
//!    the instant the monitor decided to kill, before the scope has even exited;
//! 2. the gap-round rule: with limit `gl`, a dispatch whose prompt names a phase
//!    `P` ([`mentions_phase`]) is denied once `per_phase[P] >= gl + 1`. The kill at
//!    `gl + 2` is the monitor's (`run.rs`), not this module's;
//! 3. the dispatch cap: with limit `lim`, the dispatch is denied once
//!    `hook_dispatches >= lim`, so dispatches `1..=lim` pass and `lim + 1` is the
//!    first denied.
//!
//! **Nothing here counts.** The gate reads only the [`RunState`] the store holds:
//! `hook_dispatches` is incremented by the store for a dispatch the gate allowed
//! (never from anything the driver reports) and `caps.gap_rounds.per_phase` is
//! the Phase 27 counter, fed by the on-disk VERIFICATION.md scan (T-28-14, the
//! plan's prohibition).

use std::sync::{Arc, Mutex, PoisonError};

use regex::Regex;

use crate::driverctl::protocol::HookPayload;
use crate::driverctl::store::{CapLimits, Gate, GateDecision, RunState};

/// The dispatch prompt is cut to this many characters before it reaches the
/// store (`driverctl_cli::HOOK_PROMPT_CHARS`, the spike's `[:4000]`). Applied again
/// here so the rule means the same thing whoever built the payload.
const PROMPT_CHARS: usize = 4000;

/// Whether a dispatch `prompt` names phase `phase` (port of the spike's
/// `mentions_phase`): `phase 1`, `phase 01`, `phase-1`, `phase_1`, `-phase 1` and
/// `1 --gaps` all name phase `1`; `phase 10` does not. Case-insensitive.
///
/// `phase` is a phase key as `phase_key` writes it (`1`, `26.1`): it is escaped, so
/// its `.` is a literal dot. An empty `phase` or an empty prompt never matches.
pub fn mentions_phase(prompt: &str, phase: &str) -> bool {
    if prompt.is_empty() || phase.is_empty() {
        return false;
    }
    let prompt: String = prompt.chars().take(PROMPT_CHARS).collect();
    let ph = regex::escape(phase);
    let pattern = format!(r"(?i)(phase[\s_-]*0*{ph}\b|-phase\s+0*{ph}\b|\b0*{ph}\s+--gaps)");
    Regex::new(&pattern).is_ok_and(|re| re.is_match(&prompt))
}

/// Decide one driver-level dispatch. Pure; see the module header for the order.
///
/// `tripped` is the cap the monitor has already tripped, if any.
pub fn decide_dispatch(
    payload: &HookPayload,
    state: &RunState,
    limits: &CapLimits,
    tripped: Option<&str>,
) -> GateDecision {
    if let Some(cap) = tripped {
        return GateDecision::Deny(format!("run stopped by launcher cap {cap}"));
    }
    let prompt = payload.prompt.as_deref().unwrap_or("");
    if let Some(gl) = limits.gap_rounds {
        // `per_phase` is a BTreeMap: with two phases over the cap the first key in
        // sorted order is named, deterministically.
        for (phase, &count) in &state.caps.gap_rounds.per_phase {
            if count > gl && mentions_phase(prompt, phase) {
                return GateDecision::Deny(format!(
                    "phase {phase} has failed verification {count} times (gap-round cap {gl}); \
                     no further dispatch for phase {phase}. Park the item with --rule gap-rounds."
                ));
            }
        }
    }
    if let Some(lim) = limits.dispatches {
        if state.hook_dispatches >= lim {
            return GateDecision::Deny(format!(
                "dispatch cap reached ({}/{lim}). Park the item with --rule launcher-cap \
                 and end the run.",
                state.hook_dispatches
            ));
        }
    }
    GateDecision::Allow
}

/// The cap that has been tripped for this run, shared between the gate (which
/// reads it under the store mutex) and the monitor (which sets it first, then
/// writes the trip and kills the scope).
pub type Tripped = Arc<Mutex<Option<String>>>;

/// A fresh, untripped marker.
pub fn new_tripped() -> Tripped {
    Arc::new(Mutex::new(None))
}

/// The cap that was tripped, if any.
pub fn tripped_cap(tripped: &Tripped) -> Option<String> {
    tripped
        .lock()
        .unwrap_or_else(PoisonError::into_inner)
        .clone()
}

/// Mark `cap` as tripped. Returns `false` (and changes nothing) when a cap had
/// already tripped: a cap trips at most once per run.
pub fn trip(tripped: &Tripped, cap: &str) -> bool {
    let mut slot = tripped.lock().unwrap_or_else(PoisonError::into_inner);
    if slot.is_some() {
        return false;
    }
    *slot = Some(cap.to_owned());
    true
}

/// [`decide_dispatch`] as the store's [`Gate`].
pub struct CapGate {
    limits: CapLimits,
    tripped: Tripped,
}

impl CapGate {
    pub fn new(limits: CapLimits, tripped: Tripped) -> Self {
        CapGate { limits, tripped }
    }

    /// The closure `RunStore::set_gate` takes.
    ///
    /// The `tripped` lock is taken inside the store's mutex (the gate runs from
    /// `handle_hook`) and only ever briefly; the monitor never holds it while it
    /// asks for the store, so the two cannot deadlock.
    pub fn into_gate(self) -> Gate {
        let CapGate { limits, tripped } = self;
        Box::new(move |payload, state| {
            let cap = tripped_cap(&tripped);
            decide_dispatch(payload, state, &limits, cap.as_deref())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dispatch(prompt: &str) -> HookPayload {
        HookPayload {
            hook_event_name: Some("PreToolUse".to_owned()),
            tool_name: Some("Agent".to_owned()),
            prompt: Some(prompt.to_owned()),
            ..HookPayload::default()
        }
    }

    fn state(hook_dispatches: u32, per_phase: &[(&str, u32)]) -> RunState {
        let mut state = RunState {
            hook_dispatches,
            ..RunState::default()
        };
        for (phase, count) in per_phase {
            state
                .caps
                .gap_rounds
                .per_phase
                .insert((*phase).to_owned(), *count);
        }
        state
    }

    fn limits(dispatches: Option<u32>, gap_rounds: Option<u32>) -> CapLimits {
        CapLimits {
            dispatches,
            wall_clock_min: None,
            gap_rounds,
        }
    }

    fn denied(decision: &GateDecision) -> &str {
        match decision {
            GateDecision::Deny(reason) => reason,
            GateDecision::Allow => panic!("expected a denial"),
        }
    }

    // -- encoding

    #[test]
    fn the_phase_forms_the_spike_accepts_all_name_phase_1() {
        for prompt in [
            "run phase 1 now",
            "run phase 01 now",
            "run phase-1 now",
            "run phase_1 now",
            "go -phase 1",
            "/gsd-plan-phase 1 --gaps",
            "1 --gaps",
            "01 --gaps",
            "PHASE 1",
            "Phase\t1",
        ] {
            assert!(mentions_phase(prompt, "1"), "{prompt:?}");
        }
    }

    #[test]
    fn phase_10_is_not_phase_1_and_other_prompts_do_not_match() {
        for prompt in [
            "run phase 10",
            "run phase 11 --gaps",
            "11 --gaps",
            "phase 2",
            "no phase here",
        ] {
            assert!(!mentions_phase(prompt, "1"), "{prompt:?}");
        }
    }

    #[test]
    fn a_decimal_phase_key_is_matched_literally() {
        assert!(mentions_phase("verify phase 26.1 again", "26.1"));
        assert!(!mentions_phase("verify phase 26x1 again", "26.1"));
        assert!(!mentions_phase("verify phase 26 again", "26.1"));
    }

    #[test]
    fn matching_is_applied_to_the_first_4000_characters_only() {
        let padding = "x".repeat(PROMPT_CHARS - "phase 1".chars().count());
        assert!(mentions_phase(&format!("{padding}phase 1"), "1"));
        assert!(!mentions_phase(&format!("{padding}xphase 1"), "1"));
        // Multi-byte characters count as one: 3999 of them leave room for one more.
        let wide = "é".repeat(PROMPT_CHARS - 1);
        assert!(!mentions_phase(&format!("{wide}phase 1"), "1"));
        assert!(mentions_phase(&format!("{}phase 1", "é".repeat(100)), "1"));
    }

    // -- empty

    #[test]
    fn an_empty_prompt_or_phase_never_matches() {
        assert!(!mentions_phase("", "1"));
        assert!(!mentions_phase("phase 1", ""));
        assert!(!mentions_phase("", ""));
    }

    #[test]
    fn an_empty_per_phase_map_allows_every_dispatch_through_the_gap_rule() {
        let decision = decide_dispatch(
            &dispatch("phase 1 --gaps"),
            &state(0, &[]),
            &limits(None, Some(0)),
            None,
        );
        assert_eq!(decision, GateDecision::Allow);
        let missing = HookPayload {
            prompt: None,
            ..dispatch("")
        };
        assert_eq!(
            decide_dispatch(
                &missing,
                &state(0, &[("1", 9)]),
                &limits(None, Some(0)),
                None
            ),
            GateDecision::Allow,
            "a payload with no prompt names no phase"
        );
    }

    // -- boundary

    #[test]
    fn the_dispatch_cap_allows_dispatch_l_and_denies_l_plus_1() {
        let lim = limits(Some(3), None);
        for used in 0..3 {
            assert_eq!(
                decide_dispatch(&dispatch("x"), &state(used, &[]), &lim, None),
                GateDecision::Allow,
                "dispatch {} of 3",
                used + 1
            );
        }
        let decision = decide_dispatch(&dispatch("x"), &state(3, &[]), &lim, None);
        assert!(
            denied(&decision).starts_with("dispatch cap reached (3/3)."),
            "{decision:?}"
        );
    }

    #[test]
    fn limit_zero_denies_the_first_dispatch() {
        let decision =
            decide_dispatch(&dispatch("x"), &state(0, &[]), &limits(Some(0), None), None);
        assert!(denied(&decision).starts_with("dispatch cap reached (0/0)."));
    }

    #[test]
    fn the_gap_cap_allows_at_count_l_and_denies_at_l_plus_1() {
        let lim = limits(None, Some(2));
        let prompt = dispatch("/gsd-plan-phase 1 --gaps");
        assert_eq!(
            decide_dispatch(&prompt, &state(0, &[("1", 2)]), &lim, None),
            GateDecision::Allow
        );
        let decision = decide_dispatch(&prompt, &state(0, &[("1", 3)]), &lim, None);
        assert_eq!(
            denied(&decision),
            "phase 1 has failed verification 3 times (gap-round cap 2); no further dispatch \
             for phase 1. Park the item with --rule gap-rounds."
        );
        // A dispatch naming another phase is untouched.
        assert_eq!(
            decide_dispatch(&dispatch("phase 2"), &state(0, &[("1", 3)]), &lim, None),
            GateDecision::Allow
        );
    }

    #[test]
    fn gap_limit_zero_denies_the_first_dispatch_naming_a_phase_at_count_1() {
        let lim = limits(None, Some(0));
        assert_eq!(
            decide_dispatch(&dispatch("phase 1"), &state(0, &[("1", 0)]), &lim, None),
            GateDecision::Allow
        );
        let decision = decide_dispatch(&dispatch("phase 1"), &state(0, &[("1", 1)]), &lim, None);
        assert!(denied(&decision).contains("failed verification 1 times (gap-round cap 0)"));
    }

    #[test]
    fn the_gap_rule_is_decided_before_the_dispatch_cap() {
        let decision = decide_dispatch(
            &dispatch("phase 1"),
            &state(5, &[("1", 3)]),
            &limits(Some(5), Some(2)),
            None,
        );
        assert!(denied(&decision).starts_with("phase 1 has failed verification"));
    }

    // -- tripped

    #[test]
    fn a_tripped_cap_denies_everything_first() {
        let decision = decide_dispatch(
            &dispatch("anything"),
            &state(0, &[]),
            &limits(None, None),
            Some("gap_rounds"),
        );
        assert_eq!(denied(&decision), "run stopped by launcher cap gap_rounds");
    }

    #[test]
    fn the_cap_gate_reads_the_shared_trip_and_a_cap_trips_once() {
        let tripped = new_tripped();
        let mut gate = CapGate::new(limits(Some(10), None), Tripped::clone(&tripped)).into_gate();
        let payload = dispatch("x");
        let state = state(0, &[]);
        assert_eq!(gate(&payload, &state), GateDecision::Allow);
        assert!(trip(&tripped, "wall_clock_min"));
        assert!(!trip(&tripped, "gap_rounds"), "the second trip is a no-op");
        assert_eq!(tripped_cap(&tripped).as_deref(), Some("wall_clock_min"));
        assert_eq!(
            gate(&payload, &state),
            GateDecision::Deny("run stopped by launcher cap wall_clock_min".to_owned())
        );
    }
}
