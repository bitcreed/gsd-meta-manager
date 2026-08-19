//! The deterministic decision router: observed project state → next GSD command.
//!
//! **This module performs no I/O of its own, and that property is the whole
//! design (D-11).** The caller reads the project with
//! [`state_reader::parse_project_state`](crate::state_reader::parse_project_state)
//! — the one reader the dashboard and the driver already share — and hands the
//! resulting value in. Nothing here opens a file, shells out, or contacts a
//! model. That is what makes DRIVE-02's *"the same project state always yields
//! the same choice"* testable as a **value property** rather than as a
//! filesystem experiment: two `ProjectState` values that compare equal produce
//! [`Decision`] values that compare equal, and a test proves it in microseconds
//! with no fixture on disk.
//!
//! It is also what keeps determinism honest against a hazard that no single test
//! run would reveal. `ProjectState::phase_disk_statuses` is a `HashMap` with no
//! defined iteration order, so a router that *iterated* it would return
//! different answers on different runs of the same binary over the same bytes.
//! [`decide`] indexes that map by the target key and never iterates it.
//!
//! **The reason vocabulary is closed and greppable.** Each [`RouterReason`] arm
//! maps through [`RouterReason::as_str`] to one `pub const REASON_*`, in the
//! shape `src/envelope/policy.rs` established for `ParkReason`. Those seven
//! safety arms are untouched: these are a **sibling** taxonomy, and both flow
//! through the one `JournalEvent::Parked` record and the one `parked:` terminal
//! label, so a reader greps one place for *why did this run end*.
//!
//! **A state the rules do not cover parks, naming what was observed.** It never
//! falls back to a model call (that is Phase 21's goal layer, deliberately not
//! this), never defaults to `/gsd-progress` (a router selecting a router is how a
//! run re-selects the same command forever), and never guesses. An uncovered
//! state is a gap in the rule table, and [`Decision::NoRule`] is how that gap
//! becomes visible on disk instead of becoming a plausible-looking command.

use crate::state_reader::disk_status::DiskStatus;
use crate::state_reader::ProjectState;

/// The rules do not cover the observed state.
pub const REASON_NO_RULE: &str = "router_no_rule";
/// The roadmap does not corroborate the target phase, so its disk inference is
/// inferred-only state and the router refuses to act on it.
pub const REASON_STATE_UNVERIFIED: &str = "router_state_unverified";
/// A dependency the target phase declares is not satisfied.
pub const REASON_DEPENDENCY_UNSATISFIED: &str = "router_dependency_unsatisfied";

/// Why the router refused to choose a command.
///
/// A sibling of `crate::envelope::policy::ParkReason`, never an extension of it:
/// that enum's seven arms are the safety envelope's taxonomy and are closed.
/// Both reach a reader through the same `parked:` terminal label.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouterReason {
    /// No rule covers the observed state.
    NoRule,
    /// The target phase is not corroborated by the roadmap.
    StateUnverified,
    /// A declared dependency of the target phase is not satisfied.
    DependencyUnsatisfied,
}

impl RouterReason {
    /// The stable snake_case identifier a later reader greps for.
    ///
    /// One arm per variant with no wildcard, so a new reason is a compile error
    /// here rather than a silent fallthrough onto somebody else's string.
    pub fn as_str(&self) -> &'static str {
        match self {
            RouterReason::NoRule => REASON_NO_RULE,
            RouterReason::StateUnverified => REASON_STATE_UNVERIFIED,
            RouterReason::DependencyUnsatisfied => REASON_DEPENDENCY_UNSATISFIED,
        }
    }
}

/// What the router decided to do next.
///
/// Four arms and **no wildcard anywhere it is matched**, following the
/// compile-time-gate discipline `DriveError::source` and
/// `driver::reconcile::RunVerdict` already use: an arm added later is a compile
/// error at every site that has to classify it, rather than a state that quietly
/// takes somebody else's branch.
///
/// **Every payload is one token derived from typed state** — a phase number, a
/// status name — and never free text read out of an artifact. That is the same
/// constraint `envelope::policy::GitVerdict::Refuse` documents for its `detail`,
/// and here it is what keeps agent-authored content out of a command choice
/// (D-10, T-20-01). Phase 21 owns prompt-injection hardening; this phase's
/// contribution is that no prose reaches a routing arm at all.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Decision {
    /// Run this GSD command next.
    Run {
        /// The command, e.g. `/gsd-plan-phase 20`.
        command: String,
        /// Why this command, in one line. A `&'static str` from a closed set,
        /// never composed from anything the agent wrote (SAFE-04, T-20-05).
        rationale: &'static str,
    },
    /// Stop, and name the reason from the closed taxonomy.
    Park {
        /// The taxonomy member.
        reason: RouterReason,
        /// One token naming what was observed — a phase number or a status
        /// name. Never a sentence and never artifact content.
        detail: String,
    },
    /// The declared target has been reached.
    ///
    /// **Declared here, produced by no branch in this plan**, and that is
    /// deliberate rather than an omission. Goal-met is defined as the target
    /// phase's verification status being `passed` (CONTEXT.md OQ4), and this
    /// repository's `DiskInference` records only the *presence* of a
    /// `*-VERIFICATION.md`, never its frontmatter `status` (research Pitfall 2).
    /// Extending the one reader is a separate plan's; minting a goal-met arm
    /// that guessed from presence would step straight past the `human_needed`
    /// gate DRIVE-05 exists to park at.
    GoalMet,
    /// No rule covers the observed state.
    ///
    /// Distinct from [`Decision::Park`] with [`RouterReason::NoRule`] at the
    /// call site only in that this arm carries the observed status verbatim as
    /// its own field; both land on the same reason string.
    NoRule {
        /// The observed disk status, as its stable identifier.
        observed: String,
    },
}

/// The rationale carried by this plan's single forward rule.
///
/// A named constant rather than an inline literal because it is journalled onto
/// `JournalEvent::Decided.rationale`, which is a durable record a later reader
/// greps — and because a `&'static str` from a closed set is what keeps agent
/// output out of that field by construction.
pub const RATIONALE_READY_TO_PLAN: &str =
    "the phase has context or research on disk and no plans, so planning is the next step";

/// The stable identifier for a disk status.
///
/// Exhaustive with no wildcard, so a new `DiskStatus` variant — the `Executed`
/// state GSD's own vocabulary has and this repository's does not yet (research
/// Pitfall 1) — is a compile error here rather than an unnamed state in a park
/// record.
fn status_token(status: DiskStatus) -> &'static str {
    match status {
        DiskStatus::NoDirectory => "no_directory",
        DiskStatus::Empty => "empty",
        DiskStatus::Discussed => "discussed",
        DiskStatus::Researched => "researched",
        DiskStatus::Planned => "planned",
        DiskStatus::Partial => "partial",
        DiskStatus::Complete => "complete",
    }
}

/// Choose the next GSD command for `target_phase`, or refuse.
///
/// **Pure: no I/O, no model call, no clock.** `state` is the value the caller
/// already read, and every answer below is a function of it.
///
/// The rule table this plan mints is deliberately one row wide, because the
/// phase's architectural risk is the per-run/per-iteration split rather than the
/// breadth of the table, and a wide table proven on no path is worth less than a
/// narrow one proven end to end:
///
/// | Observed | Answer |
/// |---|---|
/// | target phase absent from `state.phases` | [`Decision::Park`] / [`RouterReason::StateUnverified`] |
/// | `DiskStatus::Discussed` \| `DiskStatus::Researched` | [`Decision::Run`] `/gsd-plan-phase <N>` |
/// | anything else | [`Decision::NoRule`] naming the observed status |
///
/// The first row is CONTEXT.md's *"refuse to act on inferred-only state"* made
/// mechanical: a disk inference the roadmap does not corroborate is not
/// corroborated state, and v1.1's verified/inferred badge is a safety input to
/// the router rather than decoration.
///
/// `phase_disk_statuses` is indexed by `target_phase` and **never iterated** —
/// it is a `HashMap` with undefined iteration order, and a routing decision that
/// read it in map order would falsify DRIVE-02's determinism claim in a way no
/// single test run reveals.
/// `decide_is_insensitive_to_the_insertion_order_of_the_phase_status_map`
/// asserts that rather than this sentence claiming it.
pub fn decide(state: &ProjectState, target_phase: &str) -> Decision {
    // The roadmap is the corroborating source. A phase that exists on disk but
    // not in ROADMAP.md is a directory somebody made, not a phase the project
    // declared, and acting on it is acting on inferred-only state.
    if !state
        .phases
        .iter()
        .any(|phase| phase.number == target_phase)
    {
        return Decision::Park {
            reason: RouterReason::StateUnverified,
            // The phase number, which arrived on argv and was validated as a
            // single plain path component at the seam. One token, not prose.
            detail: target_phase.to_string(),
        };
    }

    // Indexed, never iterated. An absent entry is `NoDirectory` — the same
    // default `DiskInference` carries — rather than a second refusal shape for
    // the same fact.
    let status = state
        .phase_disk_statuses
        .get(target_phase)
        .map(|inference| inference.status)
        .unwrap_or_default();

    match status {
        DiskStatus::Discussed | DiskStatus::Researched => Decision::Run {
            command: format!("/gsd-plan-phase {target_phase}"),
            rationale: RATIONALE_READY_TO_PLAN,
        },
        // Every other observed state is a gap in the table, and parking is how
        // that gap becomes visible. Spelled out arm by arm rather than as `_`,
        // so a new `DiskStatus` variant has to be classified here on purpose.
        DiskStatus::NoDirectory
        | DiskStatus::Empty
        | DiskStatus::Planned
        | DiskStatus::Partial
        | DiskStatus::Complete => Decision::NoRule {
            observed: status_token(status).to_string(),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_reader::disk_status::DiskInference;
    use crate::state_reader::roadmap_md::RoadmapPhase;

    /// A `ProjectState` whose roadmap declares `numbers` and whose disk
    /// inference map receives `statuses` in the order given.
    ///
    /// The insertion order is a parameter precisely so the determinism test can
    /// vary it; every other test passes one order and never looks at it.
    fn state_with(numbers: &[&str], statuses: &[(&str, DiskStatus)]) -> ProjectState {
        let mut state = ProjectState {
            phases: numbers
                .iter()
                .map(|number| RoadmapPhase {
                    number: (*number).to_string(),
                    name: String::new(),
                    description: String::new(),
                    completed: false,
                    total_plans: 0,
                    completed_plans: 0,
                })
                .collect(),
            ..Default::default()
        };
        for (number, status) in statuses {
            state.phase_disk_statuses.insert(
                (*number).to_string(),
                DiskInference {
                    status: *status,
                    ..Default::default()
                },
            );
        }
        state
    }

    #[test]
    fn a_discussed_or_researched_target_phase_routes_to_the_plan_command() {
        for status in [DiskStatus::Discussed, DiskStatus::Researched] {
            let state = state_with(&["20"], &[("20", status)]);
            assert_eq!(
                decide(&state, "20"),
                Decision::Run {
                    command: "/gsd-plan-phase 20".to_string(),
                    rationale: RATIONALE_READY_TO_PLAN,
                },
                "a phase with context or research on disk and no plans is ready to \
                 plan; routing it anywhere else would leave the one state this \
                 plan's rule table covers uncovered ({status:?})"
            );
        }
    }

    #[test]
    fn a_target_phase_the_roadmap_does_not_declare_parks_as_unverified() {
        // Present on disk, absent from the roadmap: inferred-only state.
        let state = state_with(&["19"], &[("20", DiskStatus::Discussed)]);

        assert_eq!(
            decide(&state, "20"),
            Decision::Park {
                reason: RouterReason::StateUnverified,
                detail: "20".to_string(),
            },
            "a disk inference the roadmap does not corroborate must never produce a \
             Run. Acting on it is acting on state nothing declared, which is the \
             'refuse to act on inferred-only state' rule this arm exists for"
        );
    }

    #[test]
    fn an_uncovered_status_parks_as_no_rule_and_names_what_was_observed() {
        for (status, token) in [
            (DiskStatus::NoDirectory, "no_directory"),
            (DiskStatus::Empty, "empty"),
            (DiskStatus::Planned, "planned"),
            (DiskStatus::Partial, "partial"),
            (DiskStatus::Complete, "complete"),
        ] {
            let state = state_with(&["20"], &[("20", status)]);
            assert_eq!(
                decide(&state, "20"),
                Decision::NoRule {
                    observed: token.to_string(),
                },
                "an uncovered state must park NAMING itself. A router that guessed \
                 here — or defaulted to /gsd-progress, which is itself a router — \
                 would hide the gap in the rule table behind a plausible command \
                 that re-selects forever ({status:?})"
            );
        }
    }

    #[test]
    fn a_declared_phase_with_no_disk_entry_at_all_parks_rather_than_panicking() {
        let state = state_with(&["20"], &[]);
        assert_eq!(
            decide(&state, "20"),
            Decision::NoRule {
                observed: "no_directory".to_string(),
            },
            "an absent map entry is the default inference, not an index panic: the \
             router runs inside a detached process whose whole value is surviving \
             its parent"
        );
    }

    #[test]
    fn decide_is_insensitive_to_the_insertion_order_of_the_phase_status_map() {
        // The same four entries, inserted in two different orders. `HashMap`
        // iteration order differs between these two values; a router that read
        // the map in iteration order would disagree with itself.
        let forward = state_with(
            &["18", "19", "20", "21"],
            &[
                ("18", DiskStatus::Complete),
                ("19", DiskStatus::Partial),
                ("20", DiskStatus::Discussed),
                ("21", DiskStatus::Empty),
            ],
        );
        let reversed = state_with(
            &["18", "19", "20", "21"],
            &[
                ("21", DiskStatus::Empty),
                ("20", DiskStatus::Discussed),
                ("19", DiskStatus::Partial),
                ("18", DiskStatus::Complete),
            ],
        );

        let expected = Decision::Run {
            command: "/gsd-plan-phase 20".to_string(),
            rationale: RATIONALE_READY_TO_PLAN,
        };

        for iteration in 0..100 {
            assert_eq!(
                decide(&forward, "20"),
                expected,
                "iteration {iteration}: an undefined-order map iterated by a routing \
                 decision falsifies the determinism criterion in a way no single test \
                 run reveals — the same binary over the same bytes would choose \
                 differently on different days"
            );
            assert_eq!(
                decide(&reversed, "20"),
                expected,
                "iteration {iteration}: two maps holding the same entries in different \
                 insertion orders must route identically; if they do not, the router \
                 is reading the map in iteration order"
            );
        }
    }

    #[test]
    fn every_router_reason_carries_its_own_stable_identifier() {
        let reasons = [
            RouterReason::NoRule,
            RouterReason::StateUnverified,
            RouterReason::DependencyUnsatisfied,
        ];
        let mut seen: Vec<&str> = reasons.iter().map(RouterReason::as_str).collect();
        seen.sort_unstable();
        let before = seen.len();
        seen.dedup();
        assert_eq!(
            seen.len(),
            before,
            "two reasons sharing one identifier makes the terminal record ambiguous \
             about which one fired, which is exactly what the parked: carrier exists \
             to answer"
        );
        for identifier in seen {
            assert!(
                identifier.starts_with("router_"),
                "a router reason must be greppable as one: {identifier}"
            );
        }
    }
}
