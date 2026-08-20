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
//! [`decide`] indexes that map by the target key and never iterates it; the
//! dependency predicates below walk `ProjectState::phases`, which is a `Vec` in
//! roadmap order, and index the map for each entry they need.
//!
//! **The reason vocabulary is closed and greppable.** Each [`RouterReason`] arm
//! maps through [`RouterReason::as_str`] to one `pub const REASON_*`, in the
//! shape `src/envelope/policy.rs` established for `ParkReason`. Those seven
//! safety arms are untouched: these are a **sibling** taxonomy, and both flow
//! through the one `JournalEvent::Parked` record and the one `parked:` terminal
//! label, so a reader greps one place for *why did this run end*.
//!
//! Two prefixes ride this one enum on purpose. `router_` marks a refusal about
//! the *table* — no rule, unverified state, an unsatisfied dependency — and
//! `gate_` marks a human-judgement gate the run reached and refused to answer
//! (DRIVE-05). Splitting them is what makes "how often does the always-park
//! posture actually stop a run, and at which gate?" answerable by grepping the
//! journals rather than by remembering.
//!
//! **A state the rules do not cover parks, naming what was observed.** It never
//! falls back to a model call (that is Phase 21's goal layer, deliberately not
//! this), never defaults to `/gsd-progress` (a router selecting a router is how a
//! run re-selects the same command forever), and never guesses. An uncovered
//! state is a gap in the rule table, and [`Decision::NoRule`] is how that gap
//! becomes visible on disk instead of becoming a plausible-looking command.
//!
//! ## Two things this module deliberately does not model
//!
//! **The waiting-signal contract (`WAITING.json`) is a written non-goal, not an
//! omission.** GSD 1.10.0 still ships `state signal-waiting` / `signal-resume`
//! and `init.manager` still reads the file back — but nothing in the whole
//! runtime *writes* it (a grep across every workflow, reference and skill
//! returns zero call sites), and the writer and the reader disagree about its
//! location: the writer prefers `.gsd/WAITING.json` when a `.gsd/` directory
//! exists, while the reader hardcodes `.planning/WAITING.json`. A rule that read
//! an always-absent file to decide nothing is a rule with no state behind it,
//! and [`RouterReason::NoRule`] already covers the day the runtime revives it.
//! Recorded here so a later reader finds a decision rather than a gap.
//!
//! **`--research-phase` is excluded from the command alphabet on purpose.** The
//! roadmap's own phase entries say *"Research: yes — `/gsd-plan-phase
//! --research-phase`"*, which reads like a selectable command and is not one: it
//! is research-**only** mode, exiting before the planner runs and producing no
//! plans. A rule that selected it would satisfy no artifact postcondition, would
//! leave the phase in the same observed state, and would therefore be re-selected
//! on the next iteration — tripping the command-repeat detector for a reason that
//! is a bug rather than a stall. The alphabet holds bare verbs and every emitted
//! command is `"{verb} {phase}"`, so no flag is constructible.

use crate::state_reader::disk_status::{DiskInference, DiskStatus};
use crate::state_reader::state_md;
use crate::state_reader::ProjectState;

/// The rules do not cover the observed state.
pub const REASON_NO_RULE: &str = "router_no_rule";
/// The roadmap does not corroborate the target phase, so its disk inference is
/// inferred-only state and the router refuses to act on it.
pub const REASON_STATE_UNVERIFIED: &str = "router_state_unverified";
/// A dependency the target phase declares is not satisfied.
pub const REASON_DEPENDENCY_UNSATISFIED: &str = "router_dependency_unsatisfied";

/// G1: the target's verification concluded a human is needed.
pub const REASON_GATE_VERIFICATION_HUMAN_NEEDED: &str = "gate_verification_human_needed";
/// G2: the target's verification found gaps.
///
/// **Its own reason rather than a fold into a generic gate reason, and that is
/// the point of it.** CONTEXT.md's always-park resolution accepts, in as many
/// words, that a fully autonomous run may park at nearly every phase boundary;
/// upstream GSD routes this state to a concrete next command
/// (`/gsd-plan-phase <N> --gaps`) and this router deliberately does not. A
/// separately-greppable reason is what turns "is the always-park posture making
/// the driver useless?" into a count over the journals instead of a memory.
pub const REASON_GATE_VERIFICATION_GAPS_FOUND: &str = "gate_verification_gaps_found";
/// G3: the target's verification artifact is older than its summaries.
pub const REASON_GATE_VERIFICATION_STALE: &str = "gate_verification_stale";
/// G4: the target has no verification artifact at all.
pub const REASON_GATE_VERIFICATION_MISSING: &str = "gate_verification_missing";
/// G5: the target's verification status is outside the routing table.
pub const REASON_GATE_VERIFICATION_UNKNOWN: &str = "gate_verification_unknown";
/// G6: the staleness check could not be run, which is not the same as running
/// it and finding nothing stale.
pub const REASON_GATE_STALE_CHECK_INDETERMINATE: &str = "gate_stale_check_indeterminate";
/// G7: the target's UAT status is one of the outstanding set.
pub const REASON_GATE_UAT_OUTSTANDING: &str = "gate_uat_outstanding";
/// G10: a non-empty `.planning/.continue-here.md` at the project root.
pub const REASON_GATE_CONTINUE_HERE_PROJECT: &str = "gate_continue_here_project";
/// G13: the target's phase directory carries a blocking-severity
/// `.continue-here.md` row.
pub const REASON_GATE_CONTINUE_HERE_PHASE_BLOCKING: &str = "gate_continue_here_phase_blocking";
/// G11: STATE.md declares the project itself to be in `error` or `failed`.
pub const REASON_GATE_STATE_ERROR: &str = "gate_state_error";
/// G15: STATE.md's Deferred Verification table names the target phase.
pub const REASON_GATE_DEFERRED_VERIFICATION: &str = "gate_deferred_verification";
/// G14-adjacent: the target has summaries but fewer than its plan count.
pub const REASON_GATE_PHASE_PARTIAL: &str = "gate_phase_partial";

/// Why the router refused to choose a command.
///
/// A sibling of `crate::envelope::policy::ParkReason`, never an extension of it:
/// that enum's seven arms are the safety envelope's taxonomy and are closed.
/// Both reach a reader through the same `parked:` terminal label.
///
/// **The gate arms are a closed set, and an un-named case does not get an
/// improvised thirteenth.** A human-judgement gate this build cannot observe
/// leaves the target in some state the rule table either covers or does not; if
/// it does not, the run parks under [`RouterReason::NoRule`] naming the observed
/// state. Minting a reason at a call site is how a taxonomy stops being one, and
/// it is why every arm below returns a `REASON_*` constant rather than a fresh
/// literal.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouterReason {
    /// No rule covers the observed state.
    NoRule,
    /// The target phase is not corroborated by the roadmap.
    StateUnverified,
    /// A declared dependency of the target phase is not satisfied.
    DependencyUnsatisfied,
    /// G1. The definitional DRIVE-05 gate.
    GateVerificationHumanNeeded,
    /// G2. Its own arm, so the cost of always-parking is measurable.
    GateVerificationGapsFound,
    /// G3.
    GateVerificationStale,
    /// G4.
    GateVerificationMissing,
    /// G5.
    GateVerificationUnknown,
    /// G6. "Could not check" is not "checked, nothing is stale".
    GateStaleCheckIndeterminate,
    /// G7.
    GateUatOutstanding,
    /// G10.
    GateContinueHereProject,
    /// G13.
    GateContinueHerePhaseBlocking,
    /// G11.
    GateStateError,
    /// G15.
    GateDeferredVerification,
    /// G14-adjacent.
    GatePhasePartial,
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
            RouterReason::GateVerificationHumanNeeded => REASON_GATE_VERIFICATION_HUMAN_NEEDED,
            RouterReason::GateVerificationGapsFound => REASON_GATE_VERIFICATION_GAPS_FOUND,
            RouterReason::GateVerificationStale => REASON_GATE_VERIFICATION_STALE,
            RouterReason::GateVerificationMissing => REASON_GATE_VERIFICATION_MISSING,
            RouterReason::GateVerificationUnknown => REASON_GATE_VERIFICATION_UNKNOWN,
            RouterReason::GateStaleCheckIndeterminate => REASON_GATE_STALE_CHECK_INDETERMINATE,
            RouterReason::GateUatOutstanding => REASON_GATE_UAT_OUTSTANDING,
            RouterReason::GateContinueHereProject => REASON_GATE_CONTINUE_HERE_PROJECT,
            RouterReason::GateContinueHerePhaseBlocking => {
                REASON_GATE_CONTINUE_HERE_PHASE_BLOCKING
            }
            RouterReason::GateStateError => REASON_GATE_STATE_ERROR,
            RouterReason::GateDeferredVerification => REASON_GATE_DEFERRED_VERIFICATION,
            RouterReason::GatePhasePartial => REASON_GATE_PHASE_PARTIAL,
        }
    }

    /// Every arm, in declaration order.
    ///
    /// Declared as data so a test can enumerate the taxonomy rather than sample
    /// it. A new arm that nobody adds here is caught by
    /// `every_router_reason_is_listed_in_the_declared_set`, which compares this
    /// list against the arm count [`RouterReason::as_str`] can produce.
    pub const ALL: &'static [RouterReason] = &[
        RouterReason::NoRule,
        RouterReason::StateUnverified,
        RouterReason::DependencyUnsatisfied,
        RouterReason::GateVerificationHumanNeeded,
        RouterReason::GateVerificationGapsFound,
        RouterReason::GateVerificationStale,
        RouterReason::GateVerificationMissing,
        RouterReason::GateVerificationUnknown,
        RouterReason::GateStaleCheckIndeterminate,
        RouterReason::GateUatOutstanding,
        RouterReason::GateContinueHereProject,
        RouterReason::GateContinueHerePhaseBlocking,
        RouterReason::GateStateError,
        RouterReason::GateDeferredVerification,
        RouterReason::GatePhasePartial,
    ];
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
    /// **Goal-met is the target phase's verification status being `passed`**
    /// (CONTEXT.md OQ4), which is a fact about bytes the verifier wrote into
    /// frontmatter — read by the one shared reader, handed here as a typed
    /// value. Nothing the agent *says* can produce it: the result envelope's
    /// prose field is read by no branch of the outcome derivation and by no
    /// branch here (D-10, T-20-20).
    ///
    /// It is deliberately checked **after** every gate. A target whose
    /// verification passed while a human-judgement gate is still open is a
    /// contradiction on disk, and reporting victory over an open gate is the
    /// precise failure DRIVE-05 exists to prevent — so the gate wins and the run
    /// parks naming it.
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

/// The rationale carried by the discuss row.
pub const RATIONALE_READY_TO_DISCUSS: &str =
    "the phase has no artifacts yet and its declared dependencies are satisfied, \
     so gathering context is the next step";

/// The rationale carried by the plan rows.
///
/// A named constant rather than an inline literal because it is journalled onto
/// `JournalEvent::Decided.rationale`, which is a durable record a later reader
/// greps — and because a `&'static str` from a closed set is what keeps agent
/// output out of that field by construction.
pub const RATIONALE_READY_TO_PLAN: &str =
    "the phase has context or research on disk and no plans, so planning is the next step";

/// The rationale carried by the execute row.
pub const RATIONALE_READY_TO_EXECUTE: &str =
    "the phase has plans and no summaries and its declared dependencies are \
     satisfied, so executing is the next step";

/// The bare command verb for gathering context.
pub const COMMAND_DISCUSS_PHASE: &str = "/gsd-discuss-phase";
/// The bare command verb for planning.
pub const COMMAND_PLAN_PHASE: &str = "/gsd-plan-phase";
/// The bare command verb for executing.
pub const COMMAND_EXECUTE_PHASE: &str = "/gsd-execute-phase";

/// Every command this router may ever emit, declared as data.
///
/// **Membership is taken from GSD's own router** (`init.cjs:2022-2079`), not
/// from prose: these are the three forward-motion commands its table emits for a
/// phase that is not yet complete. The fourth command that table can emit —
/// `/gsd-verify-work <N>` for an `executed` phase — is **deliberately absent**,
/// and its absence is a decision rather than an oversight. CONTEXT.md's OQ7
/// resolution is that reaching a human-judgement gate parks the run; the
/// verification statuses that put a phase in the `executed` state are exactly
/// that gate set, and `verify-work` is the command a *human* runs to unpark it.
/// Auto-selecting it would be the router answering the question the gate exists
/// to ask.
///
/// **The forbidden half of the alphabet is enforced structurally rather than by
/// enumeration.** Nothing outside this list is constructible as an emitted
/// command: [`RouterAction`] has three arms, each returns one member below, and
/// every emitted command is `format!("{verb} {phase}")`. So
/// `/gsd-complete-milestone` (ends in an interactive merge-strategy prompt and
/// rewrites ROADMAP.md by judgement), `/gsd-cleanup` (archives and prunes behind
/// a confirmation), `/gsd-audit-milestone` (its outcomes are themselves gates),
/// `/gsd-autonomous` (the driver driving a driver), `/gsd-progress` and
/// `/gsd-next` (routers — a router selecting a router is how a run re-selects
/// the same command forever) and `/gsd-ship` (push and PR side effects that
/// belong to the envelope's explicit paths) are unreachable by construction, not
/// by a denylist somebody has to maintain.
///
/// `tests/driver_router_table.rs` proves the list is *exactly* what the router
/// can emit, failing in **both** directions: a command-shaped literal in this
/// module that is not listed is a violation, and a listed command no reachable
/// rule row emits is equally one — an allowlist wider than the truth it
/// describes is the failure that shape exists to catch (T-20-17).
pub const SAFE_COMMAND_ALPHABET: &[&str] = &[
    COMMAND_DISCUSS_PHASE,
    COMMAND_PLAN_PHASE,
    COMMAND_EXECUTE_PHASE,
];

/// A forward-motion action, and the one command verb it builds.
///
/// The indirection is what makes the alphabet a *constructive* guarantee rather
/// than a documented intention: a rule row names an action, an action names a
/// verb from [`SAFE_COMMAND_ALPHABET`], and there is no path from a row to a
/// string that does not pass through here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RouterAction {
    /// Gather context for a phase with no artifacts.
    Discuss,
    /// Plan a phase whose context or research is on disk.
    Plan,
    /// Execute a phase whose plans are on disk.
    Execute,
}

impl RouterAction {
    /// The bare verb, always a member of [`SAFE_COMMAND_ALPHABET`].
    ///
    /// Exhaustive with no wildcard, so a fourth action is a compile error here
    /// and therefore a deliberate widening of the alphabet.
    pub fn verb(self) -> &'static str {
        match self {
            RouterAction::Discuss => COMMAND_DISCUSS_PHASE,
            RouterAction::Plan => COMMAND_PLAN_PHASE,
            RouterAction::Execute => COMMAND_EXECUTE_PHASE,
        }
    }

    /// The full command for `phase`.
    ///
    /// `phase` arrived on argv and was validated at the seam as a single plain
    /// path component (`driver::mod`'s `--target-phase` refusal), so this
    /// interpolation cannot smuggle a flag, a second command or a path.
    ///
    /// **On every path that reaches here, including the dry-run preview.** That
    /// refusal used to sit *below* the preview branch, so the sentence above was
    /// false for `--dry-run --target-phase '../../../escaped'`, which rendered
    /// the unvalidated token into something that reads as a pasteable command
    /// line (WR-09).
    ///
    /// **`pub(crate)` since Phase 21, and the widening is the point rather than
    /// a concession.** The ambiguity seam at [`Decision::NoRule`] produces a
    /// [`RouterAction`] that survived `driver::goal::parse_action` — the SAFE-08
    /// re-parse — and has to turn it into the iteration's command. Keeping this
    /// private would have meant a second `format!("{verb} {phase}")` written at
    /// that call site, and a second composition site is exactly how "there is one
    /// path from an action to a string" stops being true.
    ///
    /// Neither input can carry model bytes: `self` is a typed variant of a
    /// three-arm enum, and `phase` is the run's own `--target-phase`, validated
    /// at the seam. **This is not a widening of what may be built, only of where
    /// the one builder may be called from**, and `tests/driver_router_table.rs`
    /// still proves the emitted set is exactly [`SAFE_COMMAND_ALPHABET`].
    pub(crate) fn command_for(self, phase: &str) -> String {
        format!("{} {phase}", self.verb())
    }

    /// Every arm, in declaration order — the enumeration the both-directions
    /// guard walks.
    pub const ALL: &'static [RouterAction] = &[
        RouterAction::Discuss,
        RouterAction::Plan,
        RouterAction::Execute,
    ];
}

/// One row of the declared rule table.
///
/// **Data, not a match arm, and that is the whole reason the table is shaped
/// this way.** CONTEXT.md requires that a rule with no test row *and* a test row
/// with no rule both fail the build. A table a test can enumerate makes that a
/// property of one list; match arms a test can only sample make it a property of
/// whoever remembered to add both.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RuleRow {
    /// The observed disk status this row matches. Exactly one row per status:
    /// `no_two_rows_accept_the_same_observed_state` proves it, which is what
    /// makes "the same state always yields the same choice" a property of the
    /// table rather than a consequence of arm ordering.
    pub observed: DiskStatus,
    /// The forward action this row selects.
    pub action: RouterAction,
    /// Whether the action is withheld when the target's declared dependencies
    /// are not all complete.
    ///
    /// Taken from upstream verbatim: `init.cjs:2037` gates `execute` on
    /// `deps_satisfied` and `:2056-2064` gates `discuss` on `is_next_to_discuss`
    /// (which is `deps_satisfied` plus the empty/no-directory test this row
    /// already applies). The `plan` rows carry **no** dependency condition
    /// upstream and carry none here — planning a phase whose dependency is
    /// unfinished is not the same hazard as executing one.
    pub requires_dependencies: bool,
    /// Why this command, in one line, journalled onto `Decided.rationale`.
    pub rationale: &'static str,
}

/// The complete forward-motion rule table.
///
/// Every row's condition and command is GSD's own (`init.cjs:2022-2079`),
/// transcribed rather than derived from prose, because a hand-written table
/// drifts from the runtime the driver is actually driving.
/// `tests/driver_router_conformance.rs` proves the agreement against that
/// runtime over fixture trees rather than trusting this comment.
///
/// | Observed | Action | Dependency-gated |
/// |---|---|---|
/// | `no_directory` | discuss | yes |
/// | `empty` | discuss | yes |
/// | `discussed` | plan | no |
/// | `researched` | plan | no |
/// | `planned` | execute | yes |
///
/// The three statuses with no row here are not gaps: `partial` and `executed`
/// are resolved by [`gate_for`] before this table is consulted (upstream emits
/// no action for `partial` and routes `executed` to a verification command this
/// router will not auto-select), and `complete` is the goal-met state.
pub const RULE_TABLE: &[RuleRow] = &[
    RuleRow {
        observed: DiskStatus::NoDirectory,
        action: RouterAction::Discuss,
        requires_dependencies: true,
        rationale: RATIONALE_READY_TO_DISCUSS,
    },
    RuleRow {
        observed: DiskStatus::Empty,
        action: RouterAction::Discuss,
        requires_dependencies: true,
        rationale: RATIONALE_READY_TO_DISCUSS,
    },
    RuleRow {
        observed: DiskStatus::Discussed,
        action: RouterAction::Plan,
        requires_dependencies: false,
        rationale: RATIONALE_READY_TO_PLAN,
    },
    RuleRow {
        observed: DiskStatus::Researched,
        action: RouterAction::Plan,
        requires_dependencies: false,
        rationale: RATIONALE_READY_TO_PLAN,
    },
    RuleRow {
        observed: DiskStatus::Planned,
        action: RouterAction::Execute,
        requires_dependencies: true,
        rationale: RATIONALE_READY_TO_EXECUTE,
    },
];

/// The stable identifier for a disk status.
///
/// Exhaustive with no wildcard, so a new `DiskStatus` variant is a compile error
/// here rather than an unnamed state in a park record. That is not hypothetical:
/// `Executed` below arrived exactly that way, as a compile error at every site
/// that had to classify it.
///
/// Every token is GSD's own spelling (`init.cjs:1875-1888`), so a park record
/// naming `executed` names the state the runtime names.
pub fn status_token(status: DiskStatus) -> &'static str {
    match status {
        DiskStatus::NoDirectory => "no_directory",
        DiskStatus::Empty => "empty",
        DiskStatus::Discussed => "discussed",
        DiskStatus::Researched => "researched",
        DiskStatus::Planned => "planned",
        DiskStatus::Partial => "partial",
        DiskStatus::Executed => "executed",
        DiskStatus::Complete => "complete",
    }
}

/// The observed token for a target the roadmap declares but the reader recorded
/// no inference for.
///
/// **Deliberately distinct from `no_directory`.** The two are easy to conflate —
/// the old code did, with an `unwrap_or_default()` — and they are opposite
/// facts. `no_directory` is an *observation*: the reader looked and there is no
/// phase directory, which is a state GSD's own table routes forward from. An
/// absent map entry is the *absence of an observation*: `parse_project_state`
/// inserts one inference per roadmap phase, so a missing key means the value did
/// not come from that reader at all. Routing forward from it would be acting on
/// state nothing observed, which is the one thing CONTEXT.md's
/// refuse-to-act-on-inferred-only-state rule forbids outright.
pub const OBSERVED_NO_INFERENCE: &str = "no_disk_inference";

/// Whether the target phase's verification passed — the goal-met predicate.
///
/// **Deterministic, machine-checkable, and derived from project state alone**
/// (CONTEXT.md OQ4, DRIVE-06, T-20-20). The input is
/// [`VerificationStatus`](crate::state_reader::disk_status::VerificationStatus),
/// parsed by the one shared reader from the leading frontmatter of the phase's
/// `*-VERIFICATION.md` — bytes GSD's verifier wrote. Nothing the driven agent
/// *asserts* reaches this function, because the only thing it takes is a typed
/// enum.
///
/// Spelled as a named predicate rather than an inline comparison so that
/// "what does goal-met mean?" has exactly one answer in the tree, and so a test
/// can assert the negative arms against the same definition the router uses.
pub fn is_goal_met(inference: &DiskInference) -> bool {
    inference.verification_status.is_passed()
}

/// The human-judgement gate observed on `target_phase`, if any.
///
/// # The order is fixed, documented, and the first hit wins
///
/// Every arm below parks, so precedence decides only **which reason lands on the
/// terminal record** — never whether the run stops. That is worth stating
/// plainly, because it is what makes the ordering a legibility decision rather
/// than a safety one, and it is why the most *specific* statement about the
/// target outranks the more general ones:
///
/// 1. **G11 project status `error`/`failed`** — `next.md:60-69`'s hard stop. A
///    project that declares itself broken is not a project to route inside.
/// 2. **G10 root `.continue-here.md`** — `next.md:46-58`'s hard stop, whose only
///    bypass upstream is `--force`, which this router does not have and must not
///    invent.
/// 3. **The verification gates on the target**, in upstream's own precedence
///    (`verification.cjs:333-351`): `gaps_found` first, because the source is
///    explicit that it *"takes priority over stale"* and short-circuits the
///    staleness check; then staleness; then ordinary table routing over the
///    remaining statuses. These are consulted **only for an `executed` target**,
///    which is the state in which GSD itself consults its verification routing
///    table. For every earlier state a missing verification artifact means
///    *"not yet"*, not *"gate"* — reading it as a gate would park every phase in
///    the project forever, before a single plan was ever written.
/// 4. **G14-adjacent `partial`** — upstream emits no action for it, and
///    `next.md`'s resume prompt for an incomplete phase defaults to **Stop**.
/// 5. **G13 phase-directory blocking `.continue-here.md`.**
/// 6. **G7 outstanding UAT on the target.**
/// 7. **G15 the target named in STATE.md's Deferred Verification table.**
///
/// # What the detail carries, and what it never carries
///
/// A phase number that arrived on argv and was validated at the seam, or a
/// closed-vocabulary token chosen by an explicit comparison. Never the raw
/// STATE.md status string, never a line out of a `.continue-here.md`, never any
/// bytes the driven agent wrote (SAFE-04, T-20-18) — the journal this reason
/// lands in is a file users commit.
fn gate_for(
    state: &ProjectState,
    target_phase: &str,
    inference: &DiskInference,
) -> Option<(RouterReason, String)> {
    // 1. G11. `is_error_status` matches `error` or `failed` exactly, so the
    //    detail below is one of two known tokens chosen here rather than the
    //    project's raw status text echoed back.
    if state_md::is_error_status(&state.status) {
        let token = if state.status.trim().eq_ignore_ascii_case("failed") {
            "failed"
        } else {
            "error"
        };
        return Some((RouterReason::GateStateError, token.to_string()));
    }

    // 2. G10.
    if state.continue_here_present {
        return Some((
            RouterReason::GateContinueHereProject,
            // A fixed token, not the marker's contents.
            "project_root".to_string(),
        ));
    }

    // 3. The verification gates, for an `executed` target only.
    if inference.status == DiskStatus::Executed {
        if let Some(reason) = verification_gate(inference) {
            return Some((reason, target_phase.to_string()));
        }
    }

    // 4. G14-adjacent.
    if inference.status == DiskStatus::Partial {
        return Some((RouterReason::GatePhasePartial, target_phase.to_string()));
    }

    // 5. G13. A blocking-severity ROW, never the file's existence: this
    //    repository's own phase-19 directory carries a stale marker whose every
    //    severity reads `advisory`, and an existence check would park every run
    //    against this project forever. The reader owns that distinction.
    if inference.continue_here_blocking {
        return Some((
            RouterReason::GateContinueHerePhaseBlocking,
            target_phase.to_string(),
        ));
    }

    // 6. G7.
    if inference.uat_status.is_outstanding() {
        return Some((RouterReason::GateUatOutstanding, target_phase.to_string()));
    }

    // 7. G15.
    //
    // **Both sides normalised through the roadmap parser's own identifier
    // extraction** (WR-06). The cell is whatever STATE.md's table author wrote —
    // `19`, `Phase 19`, `**19**`, `19-gitsafe-git-blast-radius-envelope` — and
    // `target_phase` is whatever arrived on argv. Raw equality matched this
    // repository's own spelling (bare numbers, which is why every test passed)
    // and silently never fired on any other, routing a run straight past a phase
    // that explicitly owes a verification. A gate that never fires is worse than
    // an absent one: it also answers "how often did this stop a run?" with a
    // zero that has nothing to do with the posture.
    let target_id = phase_identity(target_phase);
    if state
        .deferred_verification_phases
        .iter()
        .any(|phase| phase_identity(phase) == target_id)
    {
        return Some((
            RouterReason::GateDeferredVerification,
            target_phase.to_string(),
        ));
    }

    None
}

/// One written phase reference reduced to the identifier it names, for
/// comparison against another (WR-06).
///
/// The extraction is [`crate::state_reader::roadmap_md::extract_phase_id`] — the
/// same one the roadmap parser's dependency line uses, rather than a second set
/// of accepted spellings maintained here. Text that names no identifier at all
/// keeps its own bytes, so two unparseable cells still compare as themselves and
/// nothing is silently equated with a guess.
fn phase_identity(text: &str) -> String {
    crate::state_reader::roadmap_md::extract_phase_id(text).unwrap_or_else(|| text.to_string())
}

/// The verification gate for an `executed` target, or `None` when its status is
/// `passed`.
///
/// Exhaustive over
/// [`VerificationStatus`](crate::state_reader::disk_status::VerificationStatus)
/// with no wildcard, so a seventh status is a compile error here rather than a
/// state that quietly takes the `passed` branch.
///
/// **The `missing` split is the one piece of judgement in this function, and it
/// is where G6 lives.** Upstream distinguishes *"checked; nothing is stale"*
/// from *"could not check"* with its own `staleCheckIndeterminate` flag
/// (`verification.cjs:343-357`), raised when the staleness scan itself fails on
/// an fs, scan or clock error. This repository's reader has no such flag: a
/// verification artifact it cannot open, or whose leading frontmatter carries no
/// `status`, degrades to
/// [`VerificationStatus::Missing`](crate::state_reader::disk_status::VerificationStatus::Missing).
/// So the two cases are recovered from a fact the reader *does* record —
/// `has_verification`:
///
/// - **no artifact at all** → G4 `missing`. Nothing was checked because there
///   was nothing to check.
/// - **an artifact exists but yielded no status** → G6 *could not check*. The
///   scan ran and produced no conclusion, which is precisely the case the
///   upstream source insists is not the same as a clean result.
///
/// Both park, so nothing turns on getting the split wrong; the split exists so
/// a reader of the journal can tell an unwritten verification from an unreadable
/// one without opening the tree.
fn verification_gate(inference: &DiskInference) -> Option<RouterReason> {
    use crate::state_reader::disk_status::VerificationStatus;

    match &inference.verification_status {
        // Upstream checks this FIRST and short-circuits staleness with it.
        VerificationStatus::GapsFound => Some(RouterReason::GateVerificationGapsFound),
        VerificationStatus::Stale => Some(RouterReason::GateVerificationStale),
        VerificationStatus::HumanNeeded => Some(RouterReason::GateVerificationHumanNeeded),
        VerificationStatus::Missing => Some(if inference.has_verification {
            RouterReason::GateStaleCheckIndeterminate
        } else {
            RouterReason::GateVerificationMissing
        }),
        VerificationStatus::Unknown(_) => Some(RouterReason::GateVerificationUnknown),
        // The only status that is not a gate. It is also the goal-met predicate,
        // and an `executed` phase cannot carry it — the reader's own derivation
        // promotes such a phase to `complete` — so this arm is the reader's
        // invariant restated where a router rule can rely on it.
        VerificationStatus::Passed => None,
    }
}

/// Whether `number` names a phase the project has finished.
///
/// Upstream's `completedNums` is the union of two sources
/// (`init.cjs:1958-1971`): phases whose computed completion is `phase_complete`,
/// **and** phases the roadmap ticks off with a `- [x]` checkbox. Both are read
/// here for the same reason upstream reads both — an archived or hand-ticked
/// phase has no artifacts left to infer completion from, and treating it as
/// unfinished would make every dependent phase permanently unroutable.
fn phase_is_complete(state: &ProjectState, number: &str) -> bool {
    let ticked = state
        .phases
        .iter()
        .any(|phase| phase.number == number && phase.completed);
    let inferred = state
        .phase_disk_statuses
        .get(number)
        .is_some_and(|inference| inference.status == DiskStatus::Complete);
    ticked || inferred
}

/// The first declared dependency of `target_phase` that is not complete.
///
/// **Declared, never inferred.** The dependencies come from the roadmap entry's
/// own `**Depends on**:` line, parsed by the reader; deriving them from phase
/// numbering ("20 depends on 19") is the kind of guess that drifts from the
/// runtime the driver is driving. An absent line means *no declared
/// dependencies*, not *unknown*, and yields `None` — which is upstream's rule
/// verbatim (`init.cjs:1988-1992`).
///
/// A dependency naming a phase the roadmap does not declare is treated as
/// **unsatisfied**, not as absent. A dependency on something nobody can point
/// at is not a satisfied dependency, and fail-closed is the only defensible
/// reading when the alternative is spawning an agent.
///
/// Walks `phases` (a `Vec`, in roadmap order) and indexes the status map, so the
/// answer does not depend on `HashMap` iteration order.
fn unsatisfied_dependency(state: &ProjectState, target_phase: &str) -> Option<String> {
    let target = state
        .phases
        .iter()
        .find(|phase| phase.number == target_phase)?;

    target
        .depends_on
        .iter()
        .find(|dependency| !phase_is_complete(state, dependency))
        .cloned()
}

/// Whether a dependency path runs from `from` to `to`, following declared
/// `depends_on` edges transitively.
///
/// `visited` is the cycle guard, and it is not decoration: a roadmap is
/// hand-written prose and two phases naming each other is a typo away. Upstream
/// carries the identical guard (`init.cjs:1971-1985`).
fn reaches(state: &ProjectState, from: &str, to: &str, visited: &mut Vec<String>) -> bool {
    if visited.iter().any(|seen| seen == from) {
        return false;
    }
    visited.push(from.to_string());

    let Some(phase) = state.phases.iter().find(|phase| phase.number == from) else {
        return false;
    };
    if phase.depends_on.iter().any(|dependency| dependency == to) {
        return true;
    }
    phase
        .depends_on
        .iter()
        .any(|dependency| reaches(state, dependency, to, visited))
}

/// Whether `a` and `b` are related by a declared dependency path in **either**
/// direction.
///
/// Both directions, because upstream checks both (`init.cjs:1986-1988`) and the
/// reverse direction is the one that is easy to miss: a phase that *depends on*
/// the target and is itself half-executed is exactly as much of a collision as a
/// dependency of the target being half-executed.
fn has_dependency_relationship(state: &ProjectState, a: &str, b: &str) -> bool {
    reaches(state, a, b, &mut Vec::new()) || reaches(state, b, a, &mut Vec::new())
}

/// The first partially-executed phase that collides with `target_phase`.
///
/// # What this models, and the half it deliberately does not
///
/// Upstream's collision filter (`init.cjs:2067-2079`) withholds an `execute`
/// action while any phase related to the target is `partial` **or** is `planned`
/// *and active*, and withholds a `plan` action while any related phase is
/// *active* and `discussed`/`researched`. `is_active` there means *"a file in
/// that phase's directory was modified in the last five minutes"*
/// (`init.cjs:1901-1903`).
///
/// **The recency half is not modelled, and that is a decision with a reason.**
/// [`decide`] is a pure function with no clock, by design — the property that
/// makes DRIVE-02's determinism claim a value property rather than a filesystem
/// experiment. Worse, the recency signal would be *self-referential* here: the
/// driven agent is the thing writing into those phase directories, so a driver
/// that withheld actions on five-minute mtimes would be reacting to its own
/// output. The `partial` half needs no clock, catches the collision that
/// actually matters (an interrupted execution), and is what is implemented.
///
/// The narrowing errs toward emitting an action upstream would withhold. That is
/// bounded by everything around it: the target's own `partial` state is already
/// a gate, its declared dependencies must be complete, and
/// `tests/driver_router_conformance.rs` compares against the real runtime over
/// fixture trees, so a divergence that matters shows up as a failing build
/// rather than as this paragraph.
fn colliding_partial_phase(state: &ProjectState, target_phase: &str) -> Option<String> {
    state
        .phases
        .iter()
        .find(|phase| {
            phase.number != target_phase
                && state
                    .phase_disk_statuses
                    .get(&phase.number)
                    .is_some_and(|inference| inference.status == DiskStatus::Partial)
                && has_dependency_relationship(state, target_phase, &phase.number)
        })
        .map(|phase| phase.number.clone())
}

/// Choose the next GSD command for `target_phase`, or refuse.
///
/// **Pure: no I/O, no model call, no clock.** `state` is the value the caller
/// already read, and every answer below is a function of it.
///
/// The evaluation order is fixed and is itself part of the contract:
///
/// 1. **Roadmap corroboration.** A phase the roadmap does not declare parks as
///    [`RouterReason::StateUnverified`]. CONTEXT.md's *"refuse to act on
///    inferred-only state"* made mechanical: a disk inference nothing declared
///    is a directory somebody made, not a phase the project committed to.
/// 2. **An observation exists at all.** An absent map entry parks under
///    [`OBSERVED_NO_INFERENCE`] rather than defaulting to `no_directory` and
///    routing forward from it.
/// 3. **Human-judgement gates** ([`gate_for`]), before any forward-motion rule
///    is consulted, in the documented precedence.
/// 4. **Goal-met** ([`is_goal_met`]) — after the gates, so a passing status can
///    never route past an open one.
/// 5. **The rule table** ([`RULE_TABLE`]), with the dependency conditions the
///    matched row declares.
/// 6. **No rule.** An observed state with no row parks naming itself. There is
///    no default arm anywhere in this function: not a model call, not
///    `/gsd-progress`, not a guess.
///
/// `phase_disk_statuses` is indexed by key and **never iterated** — it is a
/// `HashMap` with undefined iteration order, and a routing decision that read it
/// in map order would falsify DRIVE-02's determinism claim in a way no single
/// test run reveals.
/// `decide_is_insensitive_to_the_insertion_order_of_the_phase_status_map`
/// asserts that rather than this sentence claiming it.
pub fn decide(state: &ProjectState, target_phase: &str) -> Decision {
    // 1. The roadmap is the corroborating source.
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

    // 2. Indexed, never iterated — and an absent entry is the absence of an
    //    observation rather than an observation of absence.
    let Some(inference) = state.phase_disk_statuses.get(target_phase) else {
        return Decision::NoRule {
            observed: OBSERVED_NO_INFERENCE.to_string(),
        };
    };

    // 3. Gates first. Answering one, or routing past it, is the failure DRIVE-05
    //    exists to prevent.
    if let Some((reason, detail)) = gate_for(state, target_phase, inference) {
        return Decision::Park { reason, detail };
    }

    // 4. Goal-met is a fact about the verification frontmatter, never a claim.
    if is_goal_met(inference) {
        return Decision::GoalMet;
    }

    // 5. The declared table. Exactly one row can match (proved by
    //    `no_two_rows_accept_the_same_observed_state`), so `find` and a
    //    hypothetical "best match" are the same answer.
    let Some(row) = RULE_TABLE
        .iter()
        .find(|row| row.observed == inference.status)
    else {
        // 6. No row. Park naming what was seen.
        return Decision::NoRule {
            observed: status_token(inference.status).to_string(),
        };
    };

    if row.requires_dependencies {
        if let Some(dependency) = unsatisfied_dependency(state, target_phase) {
            return Decision::Park {
                reason: RouterReason::DependencyUnsatisfied,
                detail: dependency,
            };
        }
    }

    if row.action == RouterAction::Execute {
        if let Some(colliding) = colliding_partial_phase(state, target_phase) {
            return Decision::Park {
                reason: RouterReason::DependencyUnsatisfied,
                detail: colliding,
            };
        }
    }

    Decision::Run {
        command: row.action.command_for(target_phase),
        rationale: row.rationale,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_reader::disk_status::{DiskInference, UatStatus, VerificationStatus};
    use crate::state_reader::roadmap_md::RoadmapPhase;

    /// A `ProjectState` whose roadmap declares `numbers` and whose disk
    /// inference map receives `statuses` in the order given.
    ///
    /// The insertion order is a parameter precisely so the determinism test can
    /// vary it; every other test passes one order and never looks at it.
    fn state_with(numbers: &[&str], statuses: &[(&str, DiskStatus)]) -> ProjectState {
        let mut state = ProjectState {
            phases: numbers.iter().map(|number| phase(number, &[])).collect(),
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

    /// A roadmap entry for `number` declaring `depends_on`.
    fn phase(number: &str, depends_on: &[&str]) -> RoadmapPhase {
        RoadmapPhase {
            number: number.to_string(),
            name: String::new(),
            description: String::new(),
            completed: false,
            total_plans: 0,
            completed_plans: 0,
            depends_on: depends_on.iter().map(|d| (*d).to_string()).collect(),
        }
    }

    /// A state whose single target phase is `Executed` with `verification`.
    fn executed_with(verification: VerificationStatus, has_verification: bool) -> ProjectState {
        let mut state = state_with(&["20"], &[]);
        state.phase_disk_statuses.insert(
            "20".to_string(),
            DiskInference {
                status: DiskStatus::Executed,
                verification_status: verification,
                has_verification,
                ..Default::default()
            },
        );
        state
    }

    /// The park reason `decide` returned, or a panic naming what it returned
    /// instead — every gate test wants the same unwrap.
    fn park_reason(decision: &Decision) -> RouterReason {
        match decision {
            Decision::Park { reason, .. } => *reason,
            other => panic!("expected a park, got {other:?}"),
        }
    }

    // ---------------------------------------------------------------- the table

    #[test]
    fn every_covered_state_routes_to_the_command_gsd_s_own_router_emits() {
        for (status, expected) in [
            (DiskStatus::NoDirectory, "/gsd-discuss-phase 20"),
            (DiskStatus::Empty, "/gsd-discuss-phase 20"),
            (DiskStatus::Discussed, "/gsd-plan-phase 20"),
            (DiskStatus::Researched, "/gsd-plan-phase 20"),
            (DiskStatus::Planned, "/gsd-execute-phase 20"),
        ] {
            let state = state_with(&["20"], &[("20", status)]);
            match decide(&state, "20") {
                Decision::Run { command, .. } => assert_eq!(
                    command, expected,
                    "the rule table is transcribed from GSD's own router \
                     (init.cjs:2022-2079); a command that disagrees with it means \
                     this router is driving a runtime that does not exist ({status:?})"
                ),
                other => panic!("{status:?} must route forward, got {other:?}"),
            }
        }
    }

    #[test]
    fn no_two_rows_accept_the_same_observed_state() {
        for (index, row) in RULE_TABLE.iter().enumerate() {
            for other in RULE_TABLE.iter().skip(index + 1) {
                assert_ne!(
                    row.observed, other.observed,
                    "two rows accepting one observed state makes 'the same state always \
                     yields the same choice' a property of arm ORDER rather than of the \
                     table, which is exactly the guarantee DRIVE-02 asks for"
                );
            }
        }
    }

    #[test]
    fn every_rule_row_produces_a_run_and_every_action_is_produced_by_a_row() {
        let mut produced: Vec<RouterAction> = Vec::new();

        for row in RULE_TABLE {
            let state = state_with(&["20"], &[("20", row.observed)]);
            match decide(&state, "20") {
                Decision::Run { command, rationale } => {
                    assert_eq!(
                        command,
                        row.action.command_for("20"),
                        "a row must route to its own action's command"
                    );
                    assert_eq!(rationale, row.rationale, "a row must carry its own rationale");
                    produced.push(row.action);
                }
                other => panic!(
                    "row {:?} produced no command: {other:?}. A row that produces nothing is \
                     a table wider than the truth it describes",
                    row.observed
                ),
            }
        }

        for action in RouterAction::ALL {
            assert!(
                produced.contains(action),
                "{action:?} is a declared action no rule row produces. An action reachable \
                 from no row is dead weight in the alphabet, and the guard fails in this \
                 direction precisely so the two lists cannot drift apart"
            );
        }
    }

    #[test]
    fn every_alphabet_member_is_the_verb_of_exactly_one_action() {
        let verbs: Vec<&str> = RouterAction::ALL.iter().map(|a| a.verb()).collect();
        assert_eq!(
            verbs.len(),
            SAFE_COMMAND_ALPHABET.len(),
            "the alphabet and the action set must be the same size, or one of them is \
             describing commands the other cannot produce"
        );
        for member in SAFE_COMMAND_ALPHABET {
            assert!(
                verbs.contains(member),
                "{member} is declared safe but no action emits it"
            );
        }
        for verb in &verbs {
            assert!(
                SAFE_COMMAND_ALPHABET.contains(verb),
                "{verb} is emitted by an action but is not declared safe"
            );
        }
    }

    #[test]
    fn no_emitted_command_can_carry_a_flag_or_a_second_token() {
        for action in RouterAction::ALL {
            let command = action.command_for("20");
            assert_eq!(
                command.split_whitespace().count(),
                2,
                "an emitted command is exactly a verb and a phase number: {command}"
            );
            assert!(
                !command.contains("--"),
                "no emitted command may carry a flag. `--research-phase` in particular is \
                 research-ONLY mode: it produces no plans, leaves the phase in the same \
                 observed state, and would therefore be re-selected forever — tripping the \
                 command-repeat detector for a bug rather than a stall. Got: {command}"
            );
        }
    }

    // --------------------------------------------------------- fail-closed arms

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
    fn an_empty_phases_vector_parks_rather_than_routing() {
        let state = ProjectState::default();
        assert_eq!(
            decide(&state, "20"),
            Decision::Park {
                reason: RouterReason::StateUnverified,
                detail: "20".to_string(),
            },
            "a project whose roadmap declares nothing corroborates nothing"
        );
    }

    #[test]
    fn a_declared_phase_with_no_disk_entry_at_all_parks_rather_than_panicking() {
        for state in [
            // The map is empty.
            state_with(&["20"], &[]),
            // The map holds someone else's key.
            state_with(&["19", "20"], &[("19", DiskStatus::Complete)]),
        ] {
            assert_eq!(
                decide(&state, "20"),
                Decision::NoRule {
                    observed: OBSERVED_NO_INFERENCE.to_string(),
                },
                "an absent map entry is the ABSENCE of an observation, not an observation \
                 of `no_directory`. `parse_project_state` inserts one inference per roadmap \
                 phase, so a missing key means the value did not come from that reader — and \
                 routing forward from it would be a command chosen from nothing. It must \
                 also never be an index panic: the router runs inside a detached process \
                 whose whole value is surviving its parent"
            );
        }
    }

    #[test]
    fn a_state_the_table_does_not_cover_parks_as_no_rule_naming_it() {
        // `Complete` with a non-passing verification violates the reader's own
        // invariant (`Complete` MEANS verification passed). It is expressible,
        // so a hand-edited or foreign tree can produce it, and there is no rule
        // for it.
        let mut state = state_with(&["20"], &[]);
        state.phase_disk_statuses.insert(
            "20".to_string(),
            DiskInference {
                status: DiskStatus::Complete,
                verification_status: VerificationStatus::HumanNeeded,
                has_verification: true,
                ..Default::default()
            },
        );

        assert_eq!(
            decide(&state, "20"),
            Decision::NoRule {
                observed: "complete".to_string(),
            },
            // The progress command is named in prose rather than spelled as a
            // literal on purpose: `tests/driver_router_table.rs` scans every
            // executable line of this module for command-shaped tokens and
            // compares the set against the declared alphabet. A forbidden
            // command written into an assertion MESSAGE is still a forbidden
            // command on an executable line, and weakening the scan to spare it
            // would weaken it for the arm that mattered.
            "an uncovered state must park NAMING itself. A router that guessed here — or \
             defaulted to the progress command, which is itself a router — would hide the \
             gap in the rule table behind a plausible command that re-selects forever"
        );
    }

    // ----------------------------------------------------------- dependencies

    #[test]
    fn an_unsatisfied_declared_dependency_parks_rather_than_routing_forward() {
        let mut state = state_with(&[], &[]);
        state.phases = vec![phase("19", &[]), phase("20", &["19"])];
        state
            .phase_disk_statuses
            .insert("19".to_string(), DiskInference::default());
        state.phase_disk_statuses.insert(
            "20".to_string(),
            DiskInference {
                status: DiskStatus::Planned,
                ..Default::default()
            },
        );

        assert_eq!(
            decide(&state, "20"),
            Decision::Park {
                reason: RouterReason::DependencyUnsatisfied,
                detail: "19".to_string(),
            },
            "widening the goal to cover a dependency is a human's call, not a router's. \
             Upstream gates the execute action on deps_satisfied (init.cjs:2037) and so \
             does this row"
        );
    }

    #[test]
    fn a_satisfied_dependency_routes_forward_from_either_completion_source() {
        // Source one: the dependency's own disk status is Complete.
        let mut inferred = state_with(&[], &[]);
        inferred.phases = vec![phase("19", &[]), phase("20", &["19"])];
        inferred.phase_disk_statuses.insert(
            "19".to_string(),
            DiskInference {
                status: DiskStatus::Complete,
                verification_status: VerificationStatus::Passed,
                ..Default::default()
            },
        );
        inferred.phase_disk_statuses.insert(
            "20".to_string(),
            DiskInference {
                status: DiskStatus::Planned,
                ..Default::default()
            },
        );

        // Source two: the roadmap ticks it off, with no artifacts left to infer
        // from — the archived-phase case.
        let mut ticked = inferred.clone();
        ticked.phases[0].completed = true;
        ticked
            .phase_disk_statuses
            .insert("19".to_string(), DiskInference::default());

        for (label, state) in [("inferred", inferred), ("roadmap checkbox", ticked)] {
            match decide(&state, "20") {
                Decision::Run { command, .. } => assert_eq!(command, "/gsd-execute-phase 20"),
                other => panic!(
                    "a dependency completed via the {label} source must satisfy the \
                     condition — upstream reads BOTH (init.cjs:1958-1971), and treating an \
                     archived phase as unfinished makes every dependent phase permanently \
                     unroutable. Got {other:?}"
                ),
            }
        }
    }

    #[test]
    fn a_dependency_naming_a_phase_the_roadmap_does_not_declare_is_unsatisfied() {
        let mut state = state_with(&[], &[]);
        state.phases = vec![phase("20", &["99"])];
        state.phase_disk_statuses.insert(
            "20".to_string(),
            DiskInference {
                status: DiskStatus::Planned,
                ..Default::default()
            },
        );

        assert_eq!(
            park_reason(&decide(&state, "20")),
            RouterReason::DependencyUnsatisfied,
            "a dependency on something nobody can point at is not a satisfied dependency. \
             Fail-closed is the only defensible reading when the alternative is spawning \
             an agent"
        );
    }

    #[test]
    fn a_related_partial_phase_withholds_the_execute_action() {
        let mut state = state_with(&[], &[]);
        // 21 depends on 20 — the REVERSE direction, which `deps_satisfied`
        // alone never sees.
        state.phases = vec![phase("20", &[]), phase("21", &["20"])];
        state.phase_disk_statuses.insert(
            "20".to_string(),
            DiskInference {
                status: DiskStatus::Planned,
                ..Default::default()
            },
        );
        state.phase_disk_statuses.insert(
            "21".to_string(),
            DiskInference {
                status: DiskStatus::Partial,
                ..Default::default()
            },
        );

        assert_eq!(
            decide(&state, "20"),
            Decision::Park {
                reason: RouterReason::DependencyUnsatisfied,
                detail: "21".to_string(),
            },
            "upstream's collision filter checks the dependency relationship in BOTH \
             directions (init.cjs:1986-1988): a phase that depends on the target and is \
             itself half-executed is exactly as much of a collision as a dependency of the \
             target being half-executed"
        );
    }

    #[test]
    fn an_unrelated_partial_phase_does_not_withhold_the_execute_action() {
        let mut state = state_with(&[], &[]);
        state.phases = vec![phase("20", &[]), phase("21", &[])];
        state.phase_disk_statuses.insert(
            "20".to_string(),
            DiskInference {
                status: DiskStatus::Planned,
                ..Default::default()
            },
        );
        state.phase_disk_statuses.insert(
            "21".to_string(),
            DiskInference {
                status: DiskStatus::Partial,
                ..Default::default()
            },
        );

        match decide(&state, "20") {
            Decision::Run { command, .. } => assert_eq!(command, "/gsd-execute-phase 20"),
            other => panic!(
                "the collision filter is scoped to phases with a DECLARED dependency \
                 relationship; withholding on every partial phase anywhere in the project \
                 would stop the driver on any half-finished unrelated work. Got {other:?}"
            ),
        }
    }

    #[test]
    fn a_dependency_cycle_terminates_rather_than_recursing_forever() {
        let mut state = state_with(&[], &[]);
        // A roadmap is hand-written prose and two phases naming each other is a
        // typo away.
        state.phases = vec![phase("20", &["21"]), phase("21", &["20"])];
        state.phase_disk_statuses.insert(
            "20".to_string(),
            DiskInference {
                status: DiskStatus::Planned,
                ..Default::default()
            },
        );
        state.phase_disk_statuses.insert(
            "21".to_string(),
            DiskInference {
                status: DiskStatus::Partial,
                ..Default::default()
            },
        );

        assert_eq!(
            park_reason(&decide(&state, "20")),
            RouterReason::DependencyUnsatisfied,
            "the visited guard must make a cycle terminate; a stack overflow inside a \
             detached driver is a run that leaves no terminal record at all"
        );
    }

    #[test]
    fn the_plan_rows_carry_no_dependency_condition() {
        let mut state = state_with(&[], &[]);
        state.phases = vec![phase("19", &[]), phase("20", &["19"])];
        state
            .phase_disk_statuses
            .insert("19".to_string(), DiskInference::default());
        state.phase_disk_statuses.insert(
            "20".to_string(),
            DiskInference {
                status: DiskStatus::Discussed,
                ..Default::default()
            },
        );

        match decide(&state, "20") {
            Decision::Run { command, .. } => assert_eq!(command, "/gsd-plan-phase 20"),
            other => panic!(
                "upstream gates `execute` and `discuss` on deps_satisfied and gates `plan` \
                 on nothing (init.cjs:2046-2054). Adding a condition upstream does not have \
                 is drift in the direction that looks safe and is still drift. Got {other:?}"
            ),
        }
    }

    // ----------------------------------------------------------------- gates

    #[test]
    fn every_verification_status_outside_passed_yields_its_own_gate_reason() {
        let cases = [
            (
                VerificationStatus::HumanNeeded,
                true,
                RouterReason::GateVerificationHumanNeeded,
            ),
            (
                VerificationStatus::GapsFound,
                true,
                RouterReason::GateVerificationGapsFound,
            ),
            (
                VerificationStatus::Stale,
                true,
                RouterReason::GateVerificationStale,
            ),
            (
                VerificationStatus::Missing,
                false,
                RouterReason::GateVerificationMissing,
            ),
            (
                VerificationStatus::Missing,
                true,
                RouterReason::GateStaleCheckIndeterminate,
            ),
            (
                VerificationStatus::Unknown("invented".to_string()),
                true,
                RouterReason::GateVerificationUnknown,
            ),
        ];

        for (status, has_verification, expected) in cases {
            let state = executed_with(status.clone(), has_verification);
            assert_eq!(
                decide(&state, "20"),
                Decision::Park {
                    reason: expected,
                    detail: "20".to_string(),
                },
                "an executed phase whose verification is {status:?} (artifact present: \
                 {has_verification}) must park under its own reason. Folding gates \
                 together is how the always-park posture stops being measurable"
            );
        }
    }

    #[test]
    fn the_gaps_found_reason_is_distinct_from_every_other_gate_reason() {
        for reason in RouterReason::ALL {
            if *reason == RouterReason::GateVerificationGapsFound {
                continue;
            }
            assert_ne!(
                reason.as_str(),
                REASON_GATE_VERIFICATION_GAPS_FOUND,
                "gaps_found parks under its OWN reason so the cost of the always-park \
                 posture is a count over the journals rather than a memory. Upstream routes \
                 this state to a concrete command and this router deliberately does not; \
                 that choice has to be measurable"
            );
        }
    }

    #[test]
    fn the_stale_check_indeterminate_condition_parks_rather_than_proceeding() {
        // A verification artifact exists and yielded no status: the scan ran and
        // produced no conclusion.
        let state = executed_with(VerificationStatus::Missing, true);
        let decision = decide(&state, "20");

        assert_eq!(
            park_reason(&decision),
            RouterReason::GateStaleCheckIndeterminate,
            "'could not check' is not 'checked, nothing is stale' — the upstream source \
             says so in as many words (verification.cjs:343-357), and CONTEXT.md's refusal \
             to act on uncorroborated state settles it in favour of parking"
        );
        assert!(
            !matches!(decision, Decision::Run { .. }),
            "an indeterminate staleness check must never yield a forward command"
        );
    }

    #[test]
    fn a_gate_outranks_the_forward_rule_for_the_same_phase() {
        // `partial` is a state upstream emits no action for, and next.md's
        // resume prompt for an incomplete phase defaults to Stop.
        let state = state_with(&["20"], &[("20", DiskStatus::Partial)]);
        assert_eq!(
            park_reason(&decide(&state, "20")),
            RouterReason::GatePhasePartial,
            "an interrupted execution is a human's call to resume or abandon"
        );
    }

    #[test]
    fn each_project_level_and_phase_level_gate_yields_its_own_reason() {
        // G11.
        let mut error = state_with(&["20"], &[("20", DiskStatus::Discussed)]);
        error.status = "error".to_string();
        assert_eq!(
            decide(&error, "20"),
            Decision::Park {
                reason: RouterReason::GateStateError,
                detail: "error".to_string(),
            }
        );

        let mut failed = state_with(&["20"], &[("20", DiskStatus::Discussed)]);
        failed.status = "Failed".to_string();
        assert_eq!(
            decide(&failed, "20"),
            Decision::Park {
                reason: RouterReason::GateStateError,
                detail: "failed".to_string(),
            },
            "the detail is a token chosen by an explicit comparison, never the project's \
             raw status text echoed into a journal users commit (SAFE-04)"
        );

        // G10.
        let mut root_marker = state_with(&["20"], &[("20", DiskStatus::Discussed)]);
        root_marker.continue_here_present = true;
        assert_eq!(
            decide(&root_marker, "20"),
            Decision::Park {
                reason: RouterReason::GateContinueHereProject,
                detail: "project_root".to_string(),
            }
        );

        // G13.
        let mut phase_marker = state_with(&["20"], &[]);
        phase_marker.phase_disk_statuses.insert(
            "20".to_string(),
            DiskInference {
                status: DiskStatus::Discussed,
                continue_here_blocking: true,
                ..Default::default()
            },
        );
        assert_eq!(
            park_reason(&decide(&phase_marker, "20")),
            RouterReason::GateContinueHerePhaseBlocking
        );

        // G7.
        let mut uat = state_with(&["20"], &[]);
        uat.phase_disk_statuses.insert(
            "20".to_string(),
            DiskInference {
                status: DiskStatus::Discussed,
                uat_status: UatStatus::Blocked,
                ..Default::default()
            },
        );
        assert_eq!(
            park_reason(&decide(&uat, "20")),
            RouterReason::GateUatOutstanding
        );

        // G15.
        let mut deferred = state_with(&["20"], &[("20", DiskStatus::Discussed)]);
        deferred.deferred_verification_phases = vec!["20".to_string()];
        assert_eq!(
            park_reason(&decide(&deferred, "20")),
            RouterReason::GateDeferredVerification
        );
    }

    #[test]
    fn a_uat_status_outside_the_outstanding_set_is_not_a_gate() {
        let mut state = state_with(&["20"], &[]);
        // This repository's own phase 19 carries `status: deferred`, which is a
        // human's explicit decision to proceed, not an unanswered question.
        state.phase_disk_statuses.insert(
            "20".to_string(),
            DiskInference {
                status: DiskStatus::Discussed,
                uat_status: UatStatus::Other("deferred".to_string()),
                ..Default::default()
            },
        );
        assert!(
            matches!(decide(&state, "20"), Decision::Run { .. }),
            "a `deferred` UAT must not park the run; the outstanding set is \
             uat-predicate.cjs:30-32's and nothing wider"
        );
    }

    #[test]
    fn a_deferred_verification_row_naming_another_phase_is_not_this_phase_s_gate() {
        let mut state = state_with(&["20"], &[("20", DiskStatus::Discussed)]);
        state.deferred_verification_phases = vec!["19".to_string()];
        assert!(
            matches!(decide(&state, "20"), Decision::Run { .. }),
            "G15 names WHICH phase owes a verification; parking phase 20 on phase 19's row \
             would make one deferred verification stop every phase in the project"
        );
    }

    #[test]
    fn the_deferred_verification_gate_fires_on_every_spelling_a_state_table_uses() {
        // **WR-06.** The cell is whatever the table's author wrote and
        // `target_phase` is whatever arrived on argv. Raw equality matched this
        // repository's bare-number spelling — which is why every test passed —
        // and silently never fired on any other, routing a run straight past a
        // phase that explicitly owes a verification.
        for cell in [
            "19",
            "Phase 19",
            "phase 19",
            "**19**",
            "#19",
            "19-gitsafe-git-blast-radius-envelope",
        ] {
            let mut state = state_with(&["19"], &[("19", DiskStatus::Discussed)]);
            state.deferred_verification_phases = vec![cell.to_string()];
            assert_eq!(
                park_reason(&decide(&state, "19")),
                RouterReason::GateDeferredVerification,
                "a G15 row written as `{cell}` names phase 19 and must gate it. A \
                 gate that fires only on one spelling fails OPEN on every other, \
                 which is worse than an absent gate: it also answers `how often \
                 did this stop a run?` with a zero unrelated to the posture"
            );
        }

        // The negative half, unchanged: normalising must not start equating
        // different phases.
        let mut other = state_with(&["20"], &[("20", DiskStatus::Discussed)]);
        other.deferred_verification_phases = vec!["Phase 19".to_string()];
        assert!(
            matches!(decide(&other, "20"), Decision::Run { .. }),
            "phase 19's row, however spelled, is not phase 20's gate"
        );
    }

    #[test]
    fn the_documented_gate_precedence_decides_when_two_gates_are_observable_at_once() {
        // The project-level hard stop outranks the phase-level gate.
        let mut both = state_with(&["20"], &[]);
        both.continue_here_present = true;
        both.deferred_verification_phases = vec!["20".to_string()];
        both.phase_disk_statuses.insert(
            "20".to_string(),
            DiskInference {
                status: DiskStatus::Executed,
                verification_status: VerificationStatus::HumanNeeded,
                has_verification: true,
                uat_status: UatStatus::Pending,
                ..Default::default()
            },
        );
        assert_eq!(
            park_reason(&decide(&both, "20")),
            RouterReason::GateContinueHereProject,
            "a project-wide hard stop outranks anything about one phase (next.md:46-58)"
        );

        // With the project-level stop cleared, the target's verification gate
        // outranks its UAT and deferred-verification gates.
        both.continue_here_present = false;
        assert_eq!(
            park_reason(&decide(&both, "20")),
            RouterReason::GateVerificationHumanNeeded,
            "the verification status is the most specific statement about the target, and \
             every arm here parks — so precedence chooses the REASON on the terminal \
             record, never whether the run stops"
        );

        // `gaps_found` is checked before the staleness arm, mirroring upstream's
        // own short-circuit. The two cannot be observed at once in this build
        // (both are values of ONE frontmatter field, where upstream derives
        // staleness from mtimes), so the ordering is proved on the pair that CAN
        // co-occur.
        let mut gaps = executed_with(VerificationStatus::GapsFound, true);
        gaps.deferred_verification_phases = vec!["20".to_string()];
        assert_eq!(
            park_reason(&decide(&gaps, "20")),
            RouterReason::GateVerificationGapsFound,
        );
    }

    // -------------------------------------------------------------- goal-met

    #[test]
    fn a_target_phase_whose_verification_passed_is_goal_met() {
        let mut state = state_with(&["20"], &[]);
        state.phase_disk_statuses.insert(
            "20".to_string(),
            DiskInference {
                status: DiskStatus::Complete,
                verification_status: VerificationStatus::Passed,
                has_verification: true,
                ..Default::default()
            },
        );

        assert_eq!(
            decide(&state, "20"),
            Decision::GoalMet,
            "goal-met is the target phase's verification status being `passed` — a fact \
             about bytes the verifier wrote, read by the one shared reader (CONTEXT.md \
             OQ4). It is the ONLY producer of this arm"
        );
    }

    #[test]
    fn no_verification_status_other_than_passed_can_produce_goal_met() {
        for status in [
            VerificationStatus::HumanNeeded,
            VerificationStatus::GapsFound,
            VerificationStatus::Stale,
            VerificationStatus::Missing,
            VerificationStatus::Unknown("invented".to_string()),
        ] {
            for disk in [
                DiskStatus::NoDirectory,
                DiskStatus::Empty,
                DiskStatus::Discussed,
                DiskStatus::Researched,
                DiskStatus::Planned,
                DiskStatus::Partial,
                DiskStatus::Executed,
                DiskStatus::Complete,
            ] {
                let mut state = state_with(&["20"], &[]);
                state.phase_disk_statuses.insert(
                    "20".to_string(),
                    DiskInference {
                        status: disk,
                        verification_status: status.clone(),
                        has_verification: true,
                        ..Default::default()
                    },
                );
                assert_ne!(
                    decide(&state, "20"),
                    Decision::GoalMet,
                    "only a passing verification is a met goal. Anything else is either a \
                     forward command or a gate park ({disk:?} / {status:?})"
                );
            }
        }
    }

    #[test]
    fn an_open_gate_outranks_a_passing_verification() {
        let mut state = state_with(&["20"], &[]);
        state.deferred_verification_phases = vec!["20".to_string()];
        state.phase_disk_statuses.insert(
            "20".to_string(),
            DiskInference {
                status: DiskStatus::Complete,
                verification_status: VerificationStatus::Passed,
                has_verification: true,
                ..Default::default()
            },
        );

        assert_eq!(
            park_reason(&decide(&state, "20")),
            RouterReason::GateDeferredVerification,
            "a passing verification beside an open human-judgement gate is a contradiction \
             on disk. Reporting victory over the gate is precisely the failure DRIVE-05 \
             exists to prevent, so the gate wins and the run parks naming it"
        );
    }

    // ------------------------------------------------------------ determinism

    #[test]
    fn decide_is_insensitive_to_the_insertion_order_of_the_phase_status_map() {
        // The same four entries, inserted in two different orders. `HashMap`
        // iteration order differs between these two values; a router that read
        // the map in iteration order would disagree with itself.
        let forward = state_with(
            &["18", "19", "20", "21"],
            &[
                ("18", DiskStatus::Complete),
                ("19", DiskStatus::Executed),
                ("20", DiskStatus::Discussed),
                ("21", DiskStatus::Empty),
            ],
        );
        let reversed = state_with(
            &["18", "19", "20", "21"],
            &[
                ("21", DiskStatus::Empty),
                ("20", DiskStatus::Discussed),
                ("19", DiskStatus::Executed),
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

    // ------------------------------------------------------------- the taxonomy

    #[test]
    fn every_router_reason_carries_its_own_stable_identifier() {
        let mut seen: Vec<&str> = RouterReason::ALL.iter().map(RouterReason::as_str).collect();
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
                identifier.starts_with("router_") || identifier.starts_with("gate_"),
                "a reason must be greppable by producer: `router_` marks a refusal about \
                 the rule table and `gate_` a human-judgement gate the run refused to \
                 answer. Got: {identifier}"
            );
        }
    }

    #[test]
    fn every_gate_reason_is_reachable_from_some_observable_state() {
        // Every arm this test does not reach through `decide` is an arm the
        // taxonomy declares and nothing produces — the stub shape this phase
        // exists to close.
        let mut produced: Vec<RouterReason> = Vec::new();
        let mut record = |decision: Decision| {
            if let Decision::Park { reason, .. } = decision {
                produced.push(reason);
            }
        };

        record(decide(&ProjectState::default(), "20"));

        for (status, has_verification) in [
            (VerificationStatus::HumanNeeded, true),
            (VerificationStatus::GapsFound, true),
            (VerificationStatus::Stale, true),
            (VerificationStatus::Missing, false),
            (VerificationStatus::Missing, true),
            (VerificationStatus::Unknown("x".to_string()), true),
        ] {
            record(decide(&executed_with(status, has_verification), "20"));
        }

        record(decide(
            &state_with(&["20"], &[("20", DiskStatus::Partial)]),
            "20",
        ));

        let mut error = state_with(&["20"], &[("20", DiskStatus::Discussed)]);
        error.status = "error".to_string();
        record(decide(&error, "20"));

        let mut root = state_with(&["20"], &[("20", DiskStatus::Discussed)]);
        root.continue_here_present = true;
        record(decide(&root, "20"));

        let mut blocking = state_with(&["20"], &[]);
        blocking.phase_disk_statuses.insert(
            "20".to_string(),
            DiskInference {
                status: DiskStatus::Discussed,
                continue_here_blocking: true,
                ..Default::default()
            },
        );
        record(decide(&blocking, "20"));

        let mut uat = state_with(&["20"], &[]);
        uat.phase_disk_statuses.insert(
            "20".to_string(),
            DiskInference {
                status: DiskStatus::Discussed,
                uat_status: UatStatus::Failed,
                ..Default::default()
            },
        );
        record(decide(&uat, "20"));

        let mut deferred = state_with(&["20"], &[("20", DiskStatus::Discussed)]);
        deferred.deferred_verification_phases = vec!["20".to_string()];
        record(decide(&deferred, "20"));

        let mut dependency = state_with(&[], &[]);
        dependency.phases = vec![phase("19", &[]), phase("20", &["19"])];
        dependency
            .phase_disk_statuses
            .insert("19".to_string(), DiskInference::default());
        dependency.phase_disk_statuses.insert(
            "20".to_string(),
            DiskInference {
                status: DiskStatus::Planned,
                ..Default::default()
            },
        );
        record(decide(&dependency, "20"));

        for reason in RouterReason::ALL {
            // `NoRule` reaches a reader through `Decision::NoRule` rather than
            // through a `Park`, and its producer is asserted by
            // `a_state_the_table_does_not_cover_parks_as_no_rule_naming_it`.
            if *reason == RouterReason::NoRule {
                continue;
            }
            assert!(
                produced.contains(reason),
                "{reason:?} is declared in the taxonomy and produced by no observable \
                 state. A reason with no producer is a stub wearing a constant's clothes, \
                 and it makes `grep {}` in the journals silently answer zero",
                reason.as_str()
            );
        }
    }
}
