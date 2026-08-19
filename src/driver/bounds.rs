//! The run bounds: the four detectors that make an unattended run stop itself.
//!
//! **This module performs no I/O of its own**, in the same register and for the
//! same reason as [`super::router`]: the caller captures the
//! [`RunSnapshot`](crate::executor::outcome::RunSnapshot) and reads the clock,
//! and every answer below is a function of the values it hands in. That is what
//! makes CTRL-06's thresholds testable at the boundary and one step either side
//! without a filesystem, a process, or a four-hour wait.
//!
//! **CTRL-06's four detectors are the only stopping condition an unattended run
//! has**, so there is deliberately no argv value, config key or build flag that
//! disables all four at once, and no cap so large that it is a disablement in
//! disguise: [`MAX_WALL_CLOCK_CAP_SECS`] is a compiled-in ceiling and a value
//! above it is refused at the seam, before a run directory exists.
//!
//! **The reason vocabulary is closed and greppable**, one `pub const REASON_*`
//! per [`BoundsReason`] arm through [`BoundsReason::as_str`], mirroring
//! `src/envelope/policy.rs`'s `ParkReason` shape. Those seven safety arms are
//! untouched — these are a sibling taxonomy, and both reach a reader through the
//! one `JournalEvent::Parked` record and the one `parked:` terminal label.
//!
//! **The no-progress signal is `DiskDelta::between`, never a digest.**
//! `ProjectState::phase_disk_statuses` is a `HashMap` with undefined iteration
//! order, so a hash that iterated it would be nondeterministic and would falsify
//! DRIVE-02's determinism claim in a way no single test run reveals.
//! `DiskDelta::between` already performs exactly the comparison intended, by
//! value equality, with the unknown-versus-unchanged distinction already
//! correct: an unknown git half contributes **nothing** rather than asserting
//! that nothing moved.

use std::time::Duration;

use crate::executor::outcome::{DiskDelta, RunSnapshot};

/// Two consecutive iterations changed nothing observable.
pub const REASON_NO_PROGRESS: &str = "bounds_no_progress";
/// The router selected the same command two iterations running.
pub const REASON_COMMAND_REPEAT: &str = "bounds_command_repeat";
/// The run performed as many iterations as it was allowed.
pub const REASON_STEP_CAP: &str = "bounds_step_cap";
/// The run exceeded its own wall-clock cap.
pub const REASON_WALL_CLOCK: &str = "bounds_wall_clock";

/// Why a run halted itself.
///
/// A sibling of `crate::envelope::policy::ParkReason` and of
/// [`super::router::RouterReason`], never an extension of either.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundsReason {
    /// Two consecutive iterations left the project unchanged.
    NoProgress,
    /// The same command was selected twice in a row.
    CommandRepeat,
    /// The step cap was reached.
    StepCap,
    /// The run-level wall-clock cap was reached.
    WallClock,
}

impl BoundsReason {
    /// The stable snake_case identifier a later reader greps for.
    ///
    /// One arm per variant, no wildcard: a new detector is a compile error here
    /// rather than a halt that borrows somebody else's reason string.
    pub fn as_str(&self) -> &'static str {
        match self {
            BoundsReason::NoProgress => REASON_NO_PROGRESS,
            BoundsReason::CommandRepeat => REASON_COMMAND_REPEAT,
            BoundsReason::StepCap => REASON_STEP_CAP,
            BoundsReason::WallClock => REASON_WALL_CLOCK,
        }
    }
}

/// What [`evaluate`] answers.
///
/// **The type cannot express more than one reason, and that is the point.**
/// CTRL-06's criterion asks *which detector fired*; a list of detectors is not
/// that. When several conditions hold at once the fixed evaluation order decides
/// and the first hit returns immediately.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundVerdict {
    /// No bound fired; the next iteration may spawn.
    Continue,
    /// A bound fired. The run halts **before** spawning.
    Halt(BoundsReason),
}

/// How many iterations a run performs unless argv says otherwise.
///
/// Twenty, per CONTEXT.md. Overridable per run and recorded in the run's own
/// bounds, so the cap in force is readable after the fact rather than inferred
/// from the binary's defaults.
pub const DEFAULT_MAX_STEPS: u32 = 20;

/// How long a whole driven run may take unless argv says otherwise.
///
/// Four hours, per CONTEXT.md — this is the user-facing number, so it keeps the
/// value the user was told.
pub const DEFAULT_RUN_WALL_CLOCK_CAP: Duration = Duration::from_secs(4 * 60 * 60);

/// The wall-clock cap the driver hands the executor for **one iteration**.
///
/// **Three hours, and the arithmetic is the whole reason this constant exists.**
/// `ExecutionOptions` defaults its own `wall_clock_cap` to exactly four hours —
/// the same value [`DEFAULT_RUN_WALL_CLOCK_CAP`] carries — so with the two left
/// equal a run that ran out of time would report the *executor's* `timed_out`
/// and the run-level [`BoundsReason::WallClock`] would be unreportable: CTRL-06
/// asks a run that exceeds **its** cap to say so, distinguishably from an agent
/// that hung. Three hours is strictly less than four, so the per-iteration cap
/// always fires first for a single long iteration and the run-level cap always
/// fires first for an accumulation across several.
///
/// `the_iteration_wall_clock_cap_is_strictly_inside_the_run_level_cap` asserts
/// that relationship rather than this comment claiming it — the precedent is
/// `the_startup_stop_budget_fits_inside_the_driver_teardown_grace`.
pub const ITERATION_WALL_CLOCK_CAP: Duration = Duration::from_secs(3 * 60 * 60);

/// The largest wall-clock cap argv may ask for, in seconds.
///
/// **Twenty-four hours, and it is a safety control rather than a preference.**
/// CTRL-06's prohibition is that nothing may disable all four detectors at once,
/// and a cap of `u64::MAX` seconds is the wall-clock detector switched off while
/// still appearing to be configured. A ceiling is what makes "there is no
/// disablement" a property of the parser rather than of the caller's restraint.
/// It also keeps the deadline arithmetic in range by construction: a day fits in
/// a [`Duration`] and in an `Instant` on every supported platform, so no accepted
/// value can wrap.
pub const MAX_WALL_CLOCK_CAP_SECS: u64 = 24 * 60 * 60;

/// How many consecutive unchanged iterations halt a run.
///
/// Two, per CONTEXT.md. **One is not enough and the distinction is not
/// pedantry:** a legitimate GSD command can read state, decide there is nothing
/// to do and exit having written nothing, and halting on that single observation
/// would halt a healthy run on its first quiet step.
pub const NO_PROGRESS_THRESHOLD: u32 = 2;

/// The caps in force for one run.
///
/// A value rather than two loose parameters, so the run records what bounded it
/// and a test constructs the boundary case in one expression.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct RunBounds {
    /// How many iterations this run may perform.
    pub max_steps: u32,
    /// How long this run may take, end to end.
    pub wall_clock_cap: Duration,
}

impl Default for RunBounds {
    fn default() -> Self {
        Self {
            max_steps: DEFAULT_MAX_STEPS,
            wall_clock_cap: DEFAULT_RUN_WALL_CLOCK_CAP,
        }
    }
}

/// Why a proposed set of bounds was refused before the run existed.
///
/// Each maps to one typed `DriveError` variant at the seam in `driver::drive`,
/// mirroring the `run_id` precedent: the refusal happens where a bad value first
/// arrives, not inside a `value_parser` wired to one of the three paths that can
/// supply it (a hand-typed invocation, the TUI's argv builder, a re-read record).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BoundsRefusal {
    /// A step cap of zero, which is a run that can never take a step.
    ZeroStepCap,
    /// A wall-clock cap of zero, which is the same defect on the other axis.
    ZeroWallClockCap,
    /// A wall-clock cap above [`MAX_WALL_CLOCK_CAP_SECS`].
    WallClockOutOfRange {
        /// What was asked for, verbatim.
        secs: u64,
    },
}

impl std::fmt::Display for BoundsRefusal {
    /// Each message names the flag **and** what to pass instead, because a
    /// refusal a caller cannot act on is a bug report rather than an error.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            BoundsRefusal::ZeroStepCap => write!(
                f,
                "`--max-steps 0` is a run that can never take a step. Pass at least \
                 1, or omit the flag for the default of {DEFAULT_MAX_STEPS}"
            ),
            BoundsRefusal::ZeroWallClockCap => write!(
                f,
                "`--wall-clock-cap-secs 0` halts the run before its first command. \
                 Pass at least 1, or omit the flag for the default of {} seconds",
                DEFAULT_RUN_WALL_CLOCK_CAP.as_secs()
            ),
            BoundsRefusal::WallClockOutOfRange { secs } => write!(
                f,
                "`--wall-clock-cap-secs {secs}` exceeds the compiled-in ceiling of \
                 {MAX_WALL_CLOCK_CAP_SECS} seconds. A cap that large is the \
                 wall-clock detector switched off while still looking configured, \
                 and an unattended run's only stopping conditions are its detectors"
            ),
        }
    }
}

/// Resolve argv's optional overrides into the caps a run will be bounded by, or
/// refuse.
///
/// **Pure, and refusing here is what keeps CTRL-06 unbypassable.** A zero step
/// cap is a run that performs no iteration at all — the *appearance* of a driven
/// run with none of the work — and a cap above the compiled-in ceiling is the
/// wall-clock detector disabled while still looking configured. Both are refused
/// before a run directory exists, so a refused run leaves nothing behind.
///
/// The addition is [`Duration::from_secs`] over a value already proven to be at
/// most a day, so no accepted value can overflow a `Duration`, and
/// `a_cap_at_the_compiled_in_ceiling_yields_a_deadline_in_the_future` proves the
/// resulting deadline is genuinely ahead of the instant it is added to rather
/// than an already-elapsed one.
pub fn resolve(
    max_steps: Option<u32>,
    wall_clock_cap_secs: Option<u64>,
) -> Result<RunBounds, BoundsRefusal> {
    let max_steps = max_steps.unwrap_or(DEFAULT_MAX_STEPS);
    if max_steps == 0 {
        return Err(BoundsRefusal::ZeroStepCap);
    }

    let wall_clock_cap = match wall_clock_cap_secs {
        None => DEFAULT_RUN_WALL_CLOCK_CAP,
        Some(0) => return Err(BoundsRefusal::ZeroWallClockCap),
        Some(secs) if secs > MAX_WALL_CLOCK_CAP_SECS => {
            return Err(BoundsRefusal::WallClockOutOfRange { secs })
        }
        Some(secs) => Duration::from_secs(secs),
    };

    Ok(RunBounds {
        max_steps,
        wall_clock_cap,
    })
}

/// What the run has done so far, as the detectors need to see it.
///
/// Owned by the run body and folded forward one iteration at a time through
/// [`BoundsState::observe`] and [`BoundsState::record_step`], so [`evaluate`]
/// stays a pure function of a value.
#[derive(Debug, Clone, Default)]
pub struct BoundsState {
    /// How many iterations have run to completion.
    ///
    /// A `u32` compared with `>=`, never narrowed and never cast: an iteration
    /// index that lost its top bits would compare small forever, which is the
    /// step cap silently switched off.
    pub completed_steps: u32,
    /// The snapshot the previous [`BoundsState::observe`] captured, if any.
    pub previous_snapshot: Option<RunSnapshot>,
    /// How many consecutive observations have shown nothing changed.
    pub consecutive_unchanged: u32,
    /// The command the previous iteration selected, if any.
    pub previous_command: Option<String>,
}

impl BoundsState {
    /// Fold a freshly captured snapshot into the no-progress evidence.
    ///
    /// **The comparison is [`DiskDelta::between`] and nothing else** — no digest
    /// is computed here or anywhere under this module, because
    /// `ProjectState::phase_disk_statuses` is a `HashMap` whose iteration order
    /// is undefined and a hash over it would be nondeterministic.
    /// `the_no_progress_path_uses_the_shipped_delta_and_computes_no_digest` in
    /// `tests/driver_iteration_loop.rs` scans this file's executable lines and
    /// fails if that stops being true.
    ///
    /// **An unknown git half contributes nothing rather than asserting that
    /// nothing moved.** `DiskDelta::between` already draws that distinction —
    /// `head_sha` or `dirty` being `None` on either side leaves the
    /// corresponding signal quiet — so a project with no git still reports its
    /// artifact changes honestly and a half-captured pair never fabricates a
    /// delta in either direction.
    ///
    /// The **first** observation of a run has nothing to compare against and so
    /// is never evidence of no progress.
    pub fn observe(&mut self, snapshot: RunSnapshot) {
        match &self.previous_snapshot {
            None => self.consecutive_unchanged = 0,
            Some(previous) => {
                if DiskDelta::between(previous, &snapshot).made_changes() {
                    self.consecutive_unchanged = 0;
                } else {
                    self.consecutive_unchanged = self.consecutive_unchanged.saturating_add(1);
                }
            }
        }
        self.previous_snapshot = Some(snapshot);
    }

    /// Record that an iteration ran `command` to completion.
    ///
    /// `saturating_add` rather than `+`: a run that somehow reached `u32::MAX`
    /// iterations must stay pinned at the top rather than wrap to zero and
    /// escape the step cap.
    pub fn record_step(&mut self, command: String) {
        self.completed_steps = self.completed_steps.saturating_add(1);
        self.previous_command = Some(command);
    }
}

/// Whether the next iteration may spawn.
///
/// **The evaluation order is fixed, and this doc comment is the contract:**
///
/// 1. wall-clock cap
/// 2. step cap
/// 3. no progress
/// 4. command repeat
///
/// The first hit returns immediately, so **exactly one** detector is ever
/// reported. CTRL-06 asks which detector fired; a list of detectors is not that,
/// and [`BoundVerdict`] cannot express one.
///
/// The order is not arbitrary. The two *budget* detectors go first because they
/// are facts about the run as a whole and are true regardless of what the router
/// chose; the two *stall* detectors follow because they are inferences about the
/// work, and an inference reported in place of an exhausted budget would send a
/// reader looking for a stall that was really a deadline. Within each pair the
/// coarser signal precedes the finer one.
///
/// `elapsed` is supplied by the caller rather than read from a clock here, which
/// is what lets the boundary be tested at nanosecond resolution instead of by
/// waiting four hours.
pub fn evaluate(
    bounds: &RunBounds,
    state: &BoundsState,
    elapsed: Duration,
    next_command: &str,
) -> BoundVerdict {
    if elapsed >= bounds.wall_clock_cap {
        return BoundVerdict::Halt(BoundsReason::WallClock);
    }

    // `u32` against `u32` with `>=`, no cast in either direction. With
    // `max_steps` of N: before iteration 1 the count is 0, so iterations 1..=N
    // run; before iteration N+1 the count is N and this fires. Iteration N runs;
    // iteration N+1 is refused.
    if state.completed_steps >= bounds.max_steps {
        return BoundVerdict::Halt(BoundsReason::StepCap);
    }

    if state.consecutive_unchanged >= NO_PROGRESS_THRESHOLD {
        return BoundVerdict::Halt(BoundsReason::NoProgress);
    }

    if state.previous_command.as_deref() == Some(next_command) {
        return BoundVerdict::Halt(BoundsReason::CommandRepeat);
    }

    BoundVerdict::Continue
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_reader::ProjectState;

    /// Bounds with a step cap of `max_steps` and a wall-clock cap far enough
    /// away that only the detector under test can fire.
    fn bounds(max_steps: u32) -> RunBounds {
        RunBounds {
            max_steps,
            wall_clock_cap: Duration::from_secs(60 * 60),
        }
    }

    /// A snapshot whose artifact half carries `status`, with both git halves
    /// known and equal.
    fn snapshot(status: &str) -> RunSnapshot {
        RunSnapshot {
            head_sha: Some("a".repeat(40)),
            dirty: Some(false),
            project_state: ProjectState {
                status: status.to_string(),
                ..Default::default()
            },
        }
    }

    #[test]
    fn the_iteration_wall_clock_cap_is_strictly_inside_the_run_level_cap() {
        assert!(
            ITERATION_WALL_CLOCK_CAP < DEFAULT_RUN_WALL_CLOCK_CAP,
            "the per-iteration executor cap ({ITERATION_WALL_CLOCK_CAP:?}) must be \
             STRICTLY less than the run-level cap ({DEFAULT_RUN_WALL_CLOCK_CAP:?}). \
             Left equal — which is what both default to on their own — a run that \
             ran out of time reports the executor's timed_out and the run-level \
             wall-clock reason becomes unreportable, so CTRL-06's 'halt and report \
             that as the reason' cannot be satisfied at all"
        );
    }

    #[test]
    fn the_step_cap_fires_at_the_cap_and_not_one_step_below_it() {
        // The boundary and both sides in one body, so N-1, N and N+1 are proven
        // together rather than sampled.
        const N: u32 = 5;

        let below = BoundsState {
            completed_steps: N - 1,
            ..Default::default()
        };
        let at = BoundsState {
            completed_steps: N,
            ..Default::default()
        };
        let above = BoundsState {
            completed_steps: N + 1,
            ..Default::default()
        };

        assert_eq!(
            evaluate(&bounds(N), &below, Duration::ZERO, "/gsd-plan-phase 20"),
            BoundVerdict::Continue,
            "with {} of {N} steps done the run has one left; halting here loses an \
             iteration the user paid for",
            N - 1
        );
        assert_eq!(
            evaluate(&bounds(N), &at, Duration::ZERO, "/gsd-plan-phase 20"),
            BoundVerdict::Halt(BoundsReason::StepCap),
            "with all {N} steps done the run has performed everything it was \
             allowed; a run that spawned a {}th iteration would exceed the only \
             bound standing between an unattended run and an unbounded one",
            N + 1
        );
        assert_eq!(
            evaluate(&bounds(N), &above, Duration::ZERO, "/gsd-plan-phase 20"),
            BoundVerdict::Halt(BoundsReason::StepCap),
            "past the cap the detector must stay latched; a bound that fires only \
             on exact equality is a bound a single miscount switches off"
        );
    }

    #[test]
    fn the_wall_clock_detector_fires_at_the_cap_and_not_one_nanosecond_below_it() {
        let cap = Duration::from_secs(90 * 60);
        let run = RunBounds {
            max_steps: DEFAULT_MAX_STEPS,
            wall_clock_cap: cap,
        };
        let state = BoundsState::default();

        assert_eq!(
            evaluate(
                &run,
                &state,
                cap - Duration::from_nanos(1),
                "/gsd-plan-phase 20"
            ),
            BoundVerdict::Continue,
            "one nanosecond inside the cap is still inside it; a detector that \
             rounded here would shorten every run it bounded by an unstated amount"
        );
        assert_eq!(
            evaluate(&run, &state, cap, "/gsd-plan-phase 20"),
            BoundVerdict::Halt(BoundsReason::WallClock),
            "at the cap the run is out of time. A strict > here would leave a run \
             that landed exactly on its deadline running forever at the boundary"
        );
    }

    #[test]
    fn the_no_progress_detector_needs_two_consecutive_unchanged_iterations() {
        let mut state = BoundsState::default();

        state.observe(snapshot("planning"));
        assert_eq!(
            evaluate(
                &bounds(DEFAULT_MAX_STEPS),
                &state,
                Duration::ZERO,
                "/gsd-plan-phase 20"
            ),
            BoundVerdict::Continue,
            "the first observation has nothing to compare against, so it can never \
             be evidence that nothing moved"
        );

        state.observe(snapshot("planning"));
        assert_eq!(
            state.consecutive_unchanged, 1,
            "one unchanged pair is one observation, not two"
        );
        assert_eq!(
            evaluate(
                &bounds(DEFAULT_MAX_STEPS),
                &state,
                Duration::ZERO,
                "/gsd-plan-phase 20"
            ),
            BoundVerdict::Continue,
            "a single quiet iteration is not a stall: a legitimate GSD command can \
             read state, decide there is nothing to do and exit having written \
             nothing. Halting here halts healthy runs on their first quiet step"
        );

        state.observe(snapshot("planning"));
        assert_eq!(
            evaluate(
                &bounds(DEFAULT_MAX_STEPS),
                &state,
                Duration::ZERO,
                "/gsd-plan-phase 20"
            ),
            BoundVerdict::Halt(BoundsReason::NoProgress),
            "two consecutive unchanged iterations is the documented threshold; \
             without the halt an unattended run repeats a no-op until its step cap, \
             burning a shared quota to produce nothing"
        );
    }

    #[test]
    fn a_change_between_iterations_resets_the_no_progress_evidence() {
        let mut state = BoundsState::default();
        state.observe(snapshot("planning"));
        state.observe(snapshot("planning"));
        assert_eq!(state.consecutive_unchanged, 1);

        state.observe(snapshot("executing"));
        assert_eq!(
            state.consecutive_unchanged, 0,
            "progress must clear the evidence outright rather than decrementing it: \
             a run that alternated between working and idling would otherwise \
             accumulate its way to a spurious stall halt"
        );
    }

    #[test]
    fn an_unknown_git_half_contributes_nothing_in_either_direction() {
        // `head_sha` unknown on one side, artifacts identical. The git signal
        // must stay quiet rather than assert movement.
        let known = snapshot("planning");
        let unknown = RunSnapshot {
            head_sha: None,
            ..snapshot("planning")
        };

        let delta = DiskDelta::between(&known, &unknown);
        assert!(
            !delta.head_moved,
            "an unknown head sha is inconclusive, not a commit. Reading it as \
             movement would keep a genuinely stalled run alive forever on a project \
             that is not a git repository"
        );
        assert!(
            !delta.artifacts_changed,
            "the artifact halves are equal here; this arm isolates the git signal"
        );

        // The same unknown half, this time with the artifacts genuinely
        // different: the unknown git half must not suppress the real signal.
        let moved = RunSnapshot {
            head_sha: None,
            ..snapshot("executing")
        };
        assert!(
            DiskDelta::between(&known, &moved).made_changes(),
            "an unknown git half must not veto an artifact change; a run that made \
             real progress would otherwise be halted as stalled"
        );

        // And the counter agrees: the unchanged pair counts once, the changed
        // pair clears it, so the threshold is never reached on the strength of
        // the git halves alone.
        let mut state = BoundsState::default();
        state.observe(known.clone());
        state.observe(unknown);
        assert_eq!(
            state.consecutive_unchanged, 1,
            "the unchanged verdict here rests on the artifact comparison alone"
        );
        state.observe(moved);
        assert_eq!(
            state.consecutive_unchanged, 0,
            "an unknown git half beside a real artifact change is progress"
        );
    }

    #[test]
    fn the_command_repeat_detector_fires_on_a_then_a_and_not_on_a_then_b_then_a() {
        let mut state = BoundsState::default();
        state.record_step("/gsd-plan-phase 20".to_string());

        assert_eq!(
            evaluate(
                &bounds(DEFAULT_MAX_STEPS),
                &state,
                Duration::ZERO,
                "/gsd-plan-phase 20"
            ),
            BoundVerdict::Halt(BoundsReason::CommandRepeat),
            "the same command twice in a row is the router re-selecting from a state \
             the previous run of that command did not change — the loop CTRL-06 \
             exists to break"
        );

        state.record_step("/gsd-execute-phase 20".to_string());
        assert_eq!(
            evaluate(
                &bounds(DEFAULT_MAX_STEPS),
                &state,
                Duration::ZERO,
                "/gsd-plan-phase 20"
            ),
            BoundVerdict::Continue,
            "A, B, A is legitimate forward motion — a plan, an execution, then a \
             replan — and halting it would make the detector fire on the ordinary \
             shape of GSD work"
        );
    }

    #[test]
    fn the_documented_evaluation_order_decides_when_every_condition_holds_at_once() {
        let cap = Duration::from_secs(60);
        let all = RunBounds {
            max_steps: 1,
            wall_clock_cap: cap,
        };
        let mut state = BoundsState {
            completed_steps: 9,
            ..Default::default()
        };
        state.observe(snapshot("planning"));
        state.observe(snapshot("planning"));
        state.observe(snapshot("planning"));
        state.previous_command = Some("/gsd-plan-phase 20".to_string());

        // 1. Wall clock first: every one of the four conditions is true.
        assert_eq!(
            evaluate(&all, &state, cap, "/gsd-plan-phase 20"),
            BoundVerdict::Halt(BoundsReason::WallClock),
            "with all four conditions true the run is out of time, and time is the \
             coarsest fact about it. Reporting a stall in place of an exhausted \
             deadline sends a reader looking for a bug that is not there"
        );

        // 2. Remove the wall-clock condition; the step cap must be next.
        assert_eq!(
            evaluate(&all, &state, Duration::ZERO, "/gsd-plan-phase 20"),
            BoundVerdict::Halt(BoundsReason::StepCap),
            "second in the documented order; if a stall detector answered here the \
             order would be sampled rather than pinned"
        );

        // 3. Remove the step-cap condition; no progress must be next.
        let roomy = RunBounds {
            max_steps: DEFAULT_MAX_STEPS,
            wall_clock_cap: cap,
        };
        assert_eq!(
            evaluate(&roomy, &state, Duration::ZERO, "/gsd-plan-phase 20"),
            BoundVerdict::Halt(BoundsReason::NoProgress),
            "third in the documented order"
        );

        // 4. Remove the no-progress condition; command repeat is what is left.
        let mut moving = state.clone();
        moving.consecutive_unchanged = 0;
        assert_eq!(
            evaluate(&roomy, &moving, Duration::ZERO, "/gsd-plan-phase 20"),
            BoundVerdict::Halt(BoundsReason::CommandRepeat),
            "last in the documented order, and reached only once the three above it \
             are quiet"
        );

        // 5. And with nothing true at all, the run continues.
        moving.previous_command = Some("/gsd-execute-phase 20".to_string());
        assert_eq!(
            evaluate(&roomy, &moving, Duration::ZERO, "/gsd-plan-phase 20"),
            BoundVerdict::Continue,
            "a detector that fired with no condition true would halt every run on \
             its first iteration"
        );
    }

    #[test]
    fn a_zero_step_cap_is_refused_before_a_run_exists() {
        assert_eq!(
            resolve(Some(0), None),
            Err(BoundsRefusal::ZeroStepCap),
            "a step cap of zero is a run that can never take a step — the whole \
             appearance of a driven run with none of the work. Refusing it at the \
             seam is what keeps a zero-iteration run from reaching disk"
        );
        assert_eq!(
            resolve(Some(1), None).map(|bounds| bounds.max_steps),
            Ok(1),
            "one is the smallest honest cap and must be accepted"
        );
    }

    #[test]
    fn a_wall_clock_cap_outside_the_compiled_in_range_is_refused_at_the_seam() {
        assert_eq!(
            resolve(None, Some(0)),
            Err(BoundsRefusal::ZeroWallClockCap),
            "a zero wall-clock cap halts before the first spawn, which is the same \
             zero-iteration run a zero step cap would produce"
        );
        assert_eq!(
            resolve(None, Some(MAX_WALL_CLOCK_CAP_SECS + 1)),
            Err(BoundsRefusal::WallClockOutOfRange {
                secs: MAX_WALL_CLOCK_CAP_SECS + 1,
            }),
            "a cap above the ceiling is the wall-clock detector switched off while \
             still looking configured, and CTRL-06 forbids any value that disables a \
             detector"
        );
    }

    #[test]
    fn a_cap_at_the_compiled_in_ceiling_yields_a_deadline_in_the_future() {
        let bounds = resolve(None, Some(MAX_WALL_CLOCK_CAP_SECS))
            .expect("the ceiling itself is accepted; only values above it are refused");
        assert_eq!(
            bounds.wall_clock_cap,
            Duration::from_secs(MAX_WALL_CLOCK_CAP_SECS)
        );

        let start = std::time::Instant::now();
        let deadline = start
            .checked_add(bounds.wall_clock_cap)
            .expect("no accepted cap value may overflow an Instant");
        assert!(
            deadline > start,
            "the largest accepted cap must still produce a deadline AHEAD of the \
             instant it was added to. An already-elapsed deadline would halt the run \
             on its first evaluation with a wall-clock reason that was never true"
        );
    }

    #[test]
    fn the_defaults_are_the_documented_ones() {
        let bounds = resolve(None, None).expect("the defaults are always acceptable");
        assert_eq!(bounds.max_steps, DEFAULT_MAX_STEPS);
        assert_eq!(bounds.wall_clock_cap, DEFAULT_RUN_WALL_CLOCK_CAP);
        assert_eq!(
            bounds,
            RunBounds::default(),
            "the Default impl and the resolver must agree, or the cap recorded for a \
             run would depend on which one built it"
        );
    }

    #[test]
    fn every_bounds_reason_carries_its_own_stable_identifier() {
        let reasons = [
            BoundsReason::NoProgress,
            BoundsReason::CommandRepeat,
            BoundsReason::StepCap,
            BoundsReason::WallClock,
        ];
        let mut seen: Vec<&str> = reasons.iter().map(BoundsReason::as_str).collect();
        seen.sort_unstable();
        let before = seen.len();
        seen.dedup();
        assert_eq!(
            seen.len(),
            before,
            "two detectors sharing one identifier makes 'which detector fired' \
             unanswerable from the terminal record"
        );
        for identifier in seen {
            assert!(
                identifier.starts_with("bounds_"),
                "a bounds reason must be greppable as one: {identifier}"
            );
        }
    }
}
