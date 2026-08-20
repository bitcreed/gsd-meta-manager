//! The goal layer: a plain-language goal becomes a plan over the router's own
//! alphabet, or it is refused.
//!
//! **This module performs no I/O and reads no clock**, in the same register and
//! for the same reason as [`super::bounds`], [`super::router`] and
//! [`super::rate_limit`]: the caller makes the model call and hands the returned
//! payload in, so every predicate below is testable against a literal without a
//! spawn, a network or a binary anywhere in the test.
//!
//! # The schema is defence in depth; the Rust re-parse is the control
//!
//! [`escalation_schema`] constrains the model's answer on the wire, and its
//! `command` enum is populated **from** [`router::SAFE_COMMAND_ALPHABET`] rather
//! than restated as a literal list, so widening the alphabet without widening
//! the schema is not expressible. That is worth having and it is **not** the
//! control, for three concrete reasons: the CLI derives its strict-schema
//! variant on a best-effort basis and logs a documented fallback to a non-strict
//! one; its validation retry loop can exhaust; and a future CLI version could
//! change the tool. [`parse_action`] is the control — it walks
//! [`router::RouterAction::ALL`] and compares [`router::RouterAction::verb`], so
//! a fourth action is a compile error there and therefore a deliberate widening.
//!
//! # The named-but-unmatched string is evidence, never a command
//!
//! When the payload names something outside the alphabet, [`parse_action`]
//! returns that string so the caller can record it verbatim — an injection
//! attempt that is silently dropped teaches nobody that the repository is
//! hostile. It is **never** interpolated into a command string, a log format or
//! a preview. The precedent is `router::RouterAction::command_for`, which is
//! private precisely because WR-09 rendered an unvalidated token into something
//! that reads as a pasteable command line through the *preview* path.
//!
//! # OQ1, answered empirically before the rest of the phase was committed to
//!
//! The question was: **does goal decomposition need file bodies?** If it did,
//! the empty-tool seam would be untenable and the untrusted surface would grow
//! from six enumerated strings to arbitrary repository content. Two live arms
//! were run against the installed binary, both with the seam profile's argv and
//! environment, and both asserting `tools == ["StructuredOutput"]` and
//! `mcp_servers == []` on the init envelope first — so the measurement cannot be
//! confounded by a seam that quietly had file access. Recorded in the register
//! `21-RESEARCH.md` uses for its spikes; the arms live in
//! `tests/driver_model_seam.rs` and can be re-run.
//!
//! **Binary: `claude` 2.1.236.** Goal, both arms, stated once in plain language:
//! *"get the container execution target phase to a state where its verification
//! has passed"*. The phase a human would choose is **22**
//! (`Container Execution Target`). Both arms returned a four-step plan; the
//! `terminal_state` prose is quoted here only where it carries the finding.
//!
//! - **Arm A — typed state only** (phase numbers, disk-status tokens,
//!   verification-status tokens, phase names as short labels; **no file bodies
//!   at all**). Terminal step targets phase **`"22"`**. Steps, verbatim:
//!   1. `/gsd-execute-phase 21` — *"Phase 21 (llm-goal-layer) executed against
//!      its existing plan; disk_status=verified and verification_status=passed,
//!      clearing the last incomplete phase ahead of 22."*
//!   2. `/gsd-discuss-phase 22`
//!   3. `/gsd-plan-phase 22`
//!   4. `/gsd-execute-phase 22` — *"Phase 22 executed and verified:
//!      disk_status=verified, verification_status=passed — the stated goal."*
//!
//! - **Arm B — the same typed state plus this project's own ROADMAP prose**
//!   passed through [`super::untrusted::untrusted_block`] as one boundary block.
//!   Terminal step targets phase **`"22"`** — the same choice, and the same
//!   four-step shape, opening on `/gsd-execute-phase 21` with the rationale
//!   *"unblocking work on the next phase"*.
//!
//! Both arms were schema-conformant and **every command in both plans re-parsed
//! to a [`RouterAction`](router::RouterAction)**.
//!
//! **The answer is no: goal decomposition does not need file bodies.** Arm A
//! succeeded on typed state alone, so the empty-tool seam survives and the
//! untrusted surface stays at the six enumerated strings in
//! [`super::untrusted::THIRD_PARTY_STRINGS`]. Arm B agreeing is a secondary
//! finding and is explicitly **not** what the design rests on — the boundary
//! block is built and available, but nothing requires it for decomposition.
//!
//! ## What the arms corrected about the plan type
//!
//! Both arms opened their plan on phase **21**, not on the phase the goal named.
//! That is a *correct* reading of the typed state — 21 was `planned` and
//! unfinished, and it stands between the run and 22 — and it is the reason the
//! goal's target phase is the **last** step's phase rather than the first's. The
//! first version of the arms asserted on the first step and failed both arms
//! against a seam that had answered well; the assertion was wrong, not the seam.
//! A plan is an ordered traversal that may pass through prerequisite phases, and
//! anything reading "which phase is this goal about" off step zero will be wrong
//! the moment a prerequisite exists.
//!
//! ## The retry pin, exercised rather than assumed
//!
//! `MAX_STRUCTURED_OUTPUT_RETRIES` was read out of the binary's registry and had
//! never been exercised. A third live arm supplies an **unsatisfiable** schema (a
//! string required to be both at least 50 and at most 10 characters), which Ajv
//! compiles and no answer can satisfy, and counts the structured-output tool-use
//! blocks in the drained events. **Observed: exactly 1.** The pin is honoured.
//! Had it not been, one driver-side escalation would silently be up to five model
//! turns and the DRIVE-04 per-run count would under-report by up to fivefold.
//!
//! ## The honest caveat
//!
//! All three arms are **single samples of a non-deterministic system**, so they
//! establish capability rather than a success rate. That is the design question
//! OQ1 asked, and it is deliberately all the design rests on: an answer that is
//! wrong, or hostile, is refused by [`parse_action`] regardless of how it was
//! produced.

use serde_json::Value;

use super::router;

/// The wire field naming the command a step selects.
pub const FIELD_COMMAND: &str = "command";
/// The wire field naming the phase a step targets.
pub const FIELD_PHASE: &str = "phase";
/// The wire field naming the terminal state that would satisfy a step.
pub const FIELD_TERMINAL_STATE: &str = "terminal_state";
/// The wire field carrying the ordered steps.
pub const FIELD_STEPS: &str = "steps";

/// The longest a wire-supplied phase token may be.
///
/// Eight characters, which is generous against the two-digit phase numbers this
/// tree actually uses and tight enough that the schema rejects a paragraph
/// before the Rust validator has to. It is a schema hint, not a control: the
/// control is `journal::is_plain_path_component`, applied in [`legality`].
pub const MAX_PHASE_CHARS: u64 = 8;

/// The longest a wire-supplied terminal-state description may be.
pub const MAX_TERMINAL_STATE_CHARS: u64 = 200;

/// The most steps the schema will accept in one plan.
///
/// A wire-level backstop only. The binding limit is the run's **resolved** step
/// cap, checked against the value `bounds::resolve` returned — never against
/// `bounds::DEFAULT_MAX_STEPS`, because comparing two constants is a check that
/// cannot fail.
pub const SCHEMA_MAX_STEPS: u64 = 20;

/// The JSON Schema for a decomposed plan.
///
/// The `command` enum is built **from** [`router::SAFE_COMMAND_ALPHABET`] and is
/// never restated as a literal list here. That is the whole reason this function
/// exists rather than a `const &str` of JSON: a literal would be a second place
/// the alphabet is written down, and two places that can disagree is exactly how
/// an allowlist comes to be wider than the truth it describes.
///
/// `additionalProperties: false` throughout, so a field the driver does not
/// model cannot ride along inside a validated payload.
pub fn escalation_schema() -> Value {
    let alphabet: Vec<Value> = router::SAFE_COMMAND_ALPHABET
        .iter()
        .map(|verb| Value::from(*verb))
        .collect();

    serde_json::json!({
        "type": "object",
        "additionalProperties": false,
        "properties": {
            FIELD_STEPS: {
                "type": "array",
                "minItems": 1,
                "maxItems": SCHEMA_MAX_STEPS,
                "items": {
                    "type": "object",
                    "additionalProperties": false,
                    "properties": {
                        FIELD_COMMAND: { "type": "string", "enum": alphabet },
                        FIELD_PHASE: { "type": "string", "maxLength": MAX_PHASE_CHARS },
                        FIELD_TERMINAL_STATE: {
                            "type": "string",
                            "maxLength": MAX_TERMINAL_STATE_CHARS
                        }
                    },
                    "required": [FIELD_COMMAND, FIELD_PHASE, FIELD_TERMINAL_STATE]
                }
            }
        },
        "required": [FIELD_STEPS]
    })
}

/// A command string the wire named that is not in the alphabet.
///
/// Carried as an owned value so the caller can record it verbatim on a park
/// record. **It is never rendered into a command string.** [`Display`] on this
/// type quotes it and says what it is, so a message built from it cannot be
/// mistaken for — or pasted as — a command line.
///
/// [`Display`]: std::fmt::Display
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UnknownCommand {
    named: String,
}

impl UnknownCommand {
    /// The string the wire named, verbatim and unmodified.
    ///
    /// Verbatim because a refusal that mangles its evidence is a refusal nobody
    /// can audit. Bound it with [`super::untrusted::bounded`] at the point it is
    /// rendered into a record, never here.
    pub fn named(&self) -> &str {
        &self.named
    }
}

impl std::fmt::Display for UnknownCommand {
    /// Quoted and prefixed, deliberately.
    ///
    /// The rendering must not contain a space-separated command line assembled
    /// from the named value: `"/gsd-ship 21"` reproduced bare in an error is a
    /// string a reader can paste, and producing one from model output is the
    /// thing CONTEXT.md forbids outright. Quoting it and naming it as a refusal
    /// is what keeps it evidence.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "the model named a command outside the safe alphabet: {:?} (refused; \
             the alphabet is {:?})",
            self.named,
            router::SAFE_COMMAND_ALPHABET
        )
    }
}

/// Reduce a wire-named command to a [`router::RouterAction`], or refuse.
///
/// **This is the SAFE-08 control.** It walks [`router::RouterAction::ALL`] and
/// compares [`router::RouterAction::verb`], so the set it accepts is exactly the
/// set the router can emit — the same enumeration
/// `tests/driver_router_table.rs` already guards in both directions — and a
/// fourth action is a compile error at `verb` rather than a silent widening
/// here.
///
/// Nothing is repaired. A near-miss (`gsd-plan-phase` without the leading slash,
/// `/gsd-plan-phase-2`) is refused, not corrected: repairing model output is how
/// a validator becomes a second producer of commands.
pub fn parse_action(named: &str) -> Result<router::RouterAction, UnknownCommand> {
    router::RouterAction::ALL
        .iter()
        .copied()
        .find(|action| action.verb() == named)
        .ok_or_else(|| UnknownCommand {
            named: named.to_string(),
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The schema's `command` enum, as a sorted list of strings.
    fn schema_command_enum() -> Vec<String> {
        let schema = escalation_schema();
        let members = schema["properties"][FIELD_STEPS]["items"]["properties"][FIELD_COMMAND]
            ["enum"]
            .as_array()
            .expect("the schema declares a command enum")
            .clone();
        let mut out: Vec<String> = members
            .iter()
            .map(|value| {
                value
                    .as_str()
                    .expect("every enum member is a string")
                    .to_string()
            })
            .collect();
        out.sort();
        out
    }

    #[test]
    fn the_schema_command_enum_and_the_safe_alphabet_match_in_both_directions() {
        let observed = schema_command_enum();
        assert!(
            !observed.is_empty(),
            "the schema's command enum is empty, so the diff below would pass \
             vacuously"
        );

        let mut declared: Vec<String> = router::SAFE_COMMAND_ALPHABET
            .iter()
            .map(|verb| verb.to_string())
            .collect();
        declared.sort();

        let widened: Vec<&String> = observed.iter().filter(|c| !declared.contains(c)).collect();
        assert!(
            widened.is_empty(),
            "the schema offers a command the safe alphabet does not contain — an \
             allowlist wider than the truth it describes is the failure that shape \
             exists to catch (T-20-17). Extra: {widened:?}"
        );

        let missing: Vec<&String> = declared.iter().filter(|c| !observed.contains(c)).collect();
        assert!(
            missing.is_empty(),
            "the safe alphabet contains a command the schema does not offer, so a \
             legal plan step would be rejected on the wire before the Rust \
             validator ever saw it. Missing: {missing:?}"
        );
    }

    #[test]
    fn every_alphabet_member_reparses_to_the_action_that_emits_it() {
        for action in router::RouterAction::ALL {
            assert_eq!(
                parse_action(action.verb()),
                Ok(*action),
                "the verb {:?} must reduce back to the action that produced it",
                action.verb()
            );
        }
    }

    #[test]
    fn a_command_outside_the_alphabet_is_refused_and_names_the_string_verbatim() {
        // The injection shape: a real GSD command that is deliberately NOT in
        // the alphabet, with an argument, so the refusal has something
        // command-line-shaped to leak if it is going to.
        let hostile = "/gsd-ship 21 --force";
        let refusal = parse_action(hostile).expect_err("a command outside the alphabet is refused");

        assert_eq!(
            refusal.named(),
            hostile,
            "the refusal must carry the named string verbatim, or an injection \
             attempt stops being evidence on disk"
        );

        let rendered = refusal.to_string();
        assert!(
            !rendered.contains(&format!(" {hostile} ")) && !rendered.contains(&format!("{hostile} ")),
            "the rendering must not reproduce the named value as a bare, \
             space-separated command line a reader could paste (WR-09); got: {rendered}"
        );
        assert!(
            rendered.contains(&format!("{hostile:?}")),
            "the rendering must quote the named value so it reads as evidence \
             rather than as an instruction; got: {rendered}"
        );
    }

    #[test]
    fn a_near_miss_is_refused_rather_than_repaired() {
        // The control arm for "refuses, never repairs". Without it, a
        // `parse_action` doing prefix matching or trimming would pass every
        // assertion above.
        for near_miss in [
            "gsd-plan-phase",
            "/gsd-plan-phase ",
            "/gsd-plan-phase-2",
            "/GSD-PLAN-PHASE",
            "",
        ] {
            assert!(
                parse_action(near_miss).is_err(),
                "{near_miss:?} is not a member of the alphabet and must be \
                 refused rather than corrected"
            );
        }
    }

    #[test]
    fn the_schema_forbids_fields_the_driver_does_not_model() {
        let schema = escalation_schema();
        assert_eq!(schema["additionalProperties"], Value::Bool(false));
        assert_eq!(
            schema["properties"][FIELD_STEPS]["items"]["additionalProperties"],
            Value::Bool(false),
            "a step carrying an unmodelled field would be a validated payload \
             the driver reads only part of"
        );
    }
}
