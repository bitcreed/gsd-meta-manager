//! Run one GSD command against an opted-in project, as a detached process.
//!
//! This module root carries the surface every later plan in the phase builds
//! against, so nothing downstream has to reopen it. Four facts govern everything
//! below, each with the decision that settled it:
//!
//! 1. **The driver is a detached child of the same binary (D-01).** One build,
//!    one version, one binary — `gsd-meta-manager drive <alias> --command <c>`
//!    is spawned into its own process group with all three stdio handles
//!    nulled, and it reuses `state_reader::parse_project_state` verbatim so the
//!    dashboard and the driver can never drift in how they read a project. It is
//!    deliberately **not** an in-process tokio task (which cannot survive the
//!    TUI closing), not `systemd-run`, and not tmux.
//! 2. **Driving needs two platform facilities, not one, and both refusals are
//!    typed rather than silent (D-05).** The implementation submodules carry the
//!    `#[cfg(unix)]`; the `Drive` subcommand does **not**, so on a non-Unix
//!    platform it still parses and [`drive`] returns
//!    [`DriveError::UnsupportedPlatform`] naming the missing detachment
//!    facility — an accepted limitation that surfaces as "unknown subcommand" is
//!    indistinguishable from a bug. A **real** run additionally requires a
//!    determinable liveness, which is the `/proc` probe the kill switch and the
//!    reconciliation scan are both built on; where that is unavailable
//!    [`platform_refusal`] refuses with the same typed variant and a detail that
//!    names the probe. Both mirror the posture at `src/executor/mod.rs:22-28`.
//!    Neither applies to `--dry-run`.
//! 3. **The driver's working directory is the project root itself, never a
//!    worktree (D-21).** OQ4 was resolved empirically — `claude --worktree`
//!    exists and works — and declined for five concrete reasons, chief among
//!    them that it writes *inside* the repo working directory, where
//!    `watcher.rs::extract_project_root` would resolve the worktree's
//!    `.planning/` as a phantom project and mis-route every file event. Phase 19
//!    owns the decision and the safe fallback (a worktree created *outside* the
//!    repo).
//! 4. **A run is either a single supplied command or a routed sequence, and
//!    exactly one of the two (D-20.2, CTRL-06).** Phase 17 shipped only the
//!    first: `--command` ran once and there was no loop at all. Phase 20 builds
//!    the loop. `--target-phase` puts the run under [`router`], which chooses
//!    each iteration's command from observed project state with **no model
//!    call**, and under [`bounds`], whose four detectors are the only stopping
//!    condition an unattended run has. [`DriveArgs::command`] is therefore an
//!    `Option<String>` rather than a `Vec<String>`: the sequence is *derived*
//!    per iteration, never supplied, so a field that accepted a list would
//!    describe a mode neither half of the code has. The lock, the journal, the
//!    envelope and the terminate handler are established once and held across
//!    every iteration; only spawn, drain and outcome repeat.
//! 5. **Stopping a run is two-layer, because there are TWO process groups, not
//!    one (D-06).** Phase 15 spawns `claude` with `ProcessGroup::leader()`, so
//!    the agent leads a group of its own and a signal aimed at the *driver's*
//!    group never reaches it. Layer 1 signals the driver's group from the TUI
//!    ([`kill`]); layer 2 is the driver's own terminate handler calling
//!    `Executor::cancel` on the agent's group ([`run`]) — and that second layer
//!    is a **call into code that already exists**, never a reimplementation. A
//!    stop built from layer 1 alone looks like it works, and the tell is
//!    `pgrep -x claude` still returning processes afterwards.
//!
//! [`lock`] landed in plan 17-02, [`dry_run`] in 17-04, [`liveness`] and
//! [`reconcile`] in 17-05, and [`kill`] in 17-06. Nothing is stubbed ahead of
//! time: an empty module for a later phase is a promise the compiler cannot
//! keep.

// Deliberately **outside** the `#[cfg(unix)]` block below. Point 2 above makes
// the *running* of an agent Unix-only; a preview is git reads and string
// building, so it compiles and is testable on every platform. Gating it would
// make the one mode that needs no platform facility the one mode a Windows
// build could not even check.
pub mod dry_run;
// Outside the block for a third reason, and the plainest one: both modules are
// **pure**. They open no file, spawn no process and read no clock — the caller
// captures the state and hands it in (D-11) — so there is nothing platform-
// specific in either to gate, and gating them would make the two modules whose
// determinism is the phase's central claim the two a non-Unix build could not
// even type-check.
pub mod bounds;
pub mod router;
// Third in the same block and for the identical reason: [`rate_limit`] reads no
// clock and opens no file — the driver retains the payload off the stream and
// hands in the instant to judge a reset time against — so there is nothing
// platform-specific in it to gate, and gating it would put the classification
// that decides whether a run parks on a shared quota out of reach of a non-Unix
// build's type checker.
pub mod rate_limit;
// Fourth and fifth in the same block, and for the identical reason once more:
// both are pure. [`untrusted`] builds a string out of values the caller hands
// in, and [`goal`] validates a payload the caller retained off the stream —
// neither opens a file, spawns a process or reads a clock. Gating them would put
// the boundary construction and the SAFE-08 re-parse, which are the phase's two
// central safety claims, out of reach of a non-Unix build's type checker.
pub mod goal;
pub mod untrusted;
// Sixth in the same block, and for the identical reason a sixth time:
// [`escalate`] opens no file, spawns no process and reads no clock — the caller
// makes the model call and hands in what it retained. Gating it would put the
// per-run model-consultation budget, which is DRIVE-04's whole control, out of
// reach of a non-Unix build's type checker.
pub mod escalate;
// Also outside the block, and for a related but distinct reason. Both modules
// are `/proc` and `run.json` **reads**, and D-10 is explicit that they should
// carry `src/session_detector.rs`'s honest-failure posture — that module has no
// platform attributes at all and its reads simply yield nothing off Linux —
// rather than diverging from the module they copy. A `cfg` here would also gate
// the reconciliation scan out of the TUI, which is cross-platform.
pub mod liveness;
pub mod reconcile;
#[cfg(unix)]
pub mod kill;
#[cfg(unix)]
pub mod lock;
#[cfg(unix)]
pub mod run;
#[cfg(unix)]
pub mod spawn;

// Both are named only by the debug-only override fields below, so the imports
// carry the same gate those fields do (D-30).
#[cfg(debug_assertions)]
use std::ffi::OsString;
#[cfg(debug_assertions)]
use std::path::PathBuf;

use crate::config::{Config, RegisteredProject};
use crate::envelope::{self, policy::ParkReason};
use crate::error::{DriveError, OptInError};
use crate::executor::DrivableProject;
use crate::journal;

/// Everything one `drive` invocation was asked to do.
///
/// A plain owned record rather than a borrow of the CLI enum, so the TUI's
/// spawn path (plan 17-05) and a hand-typed invocation build the same value.
#[derive(Debug, Clone)]
pub struct DriveArgs {
    /// The registry alias to drive.
    pub alias: String,
    /// The single GSD command to run, in single-command mode.
    ///
    /// **Mutually exclusive with [`target_phase`](Self::target_phase), and
    /// exactly one of the two is required for a real run.** `Some` selects
    /// single-command mode: one iteration, the existing terminal label, the
    /// existing `run.json` — byte-for-byte the Phase 17 behaviour. `None`
    /// requires `target_phase`, and puts the run under the decision router,
    /// which derives each iteration's command from observed state.
    ///
    /// It is not a `Vec<String>` and never will be: a routed sequence is
    /// *derived* per iteration from what the previous one left on disk, so a
    /// supplied list would be a third execution model neither the router nor the
    /// bounds know about.
    pub command: Option<String>,
    /// The phase a routed run is driving toward.
    ///
    /// **Mutually exclusive with [`command`](Self::command).** It is used only
    /// as a map key into `ProjectState::phase_disk_statuses` and is **never
    /// composed into a path**, but it is nonetheless validated with
    /// `journal::is_plain_path_component` at the seam in [`drive`], mirroring
    /// the `run_id` refusal that closed WR-02: the value arrives from a
    /// hand-typed invocation, from the TUI's argv builder and from a re-read run
    /// record, and a validator wired to one flag guards the least interesting of
    /// those three (D-27).
    pub target_phase: Option<String>,
    /// How many iterations this run may perform, overriding
    /// [`bounds::DEFAULT_MAX_STEPS`].
    ///
    /// Zero is refused at the seam by [`bounds::resolve`]: it is a run that can
    /// never take a step, which is the appearance of a driven run with none of
    /// the work.
    pub max_steps: Option<u32>,
    /// How long this run may take end to end, in seconds, overriding
    /// [`bounds::DEFAULT_RUN_WALL_CLOCK_CAP`].
    ///
    /// Carried as seconds rather than as a `Duration` because that is the shape
    /// argv supplies, and converting at the seam is what lets the refusal name
    /// the number the caller typed. Zero and anything above
    /// [`bounds::MAX_WALL_CLOCK_CAP_SECS`] are refused there — CTRL-06 forbids
    /// any value that disables a detector, and a cap of `u64::MAX` is the
    /// wall-clock detector switched off while still looking configured.
    pub wall_clock_cap_secs: Option<u64>,
    /// How many times this run may consult the model, overriding
    /// [`escalate::DEFAULT_MAX_ESCALATIONS`].
    ///
    /// Zero and any value **at or above this run's resolved step cap** are
    /// refused at the seam by [`escalate::resolve`], in the same register and for
    /// the same reason as [`max_steps`](Self::max_steps): a zero cap is a seam
    /// that can never fire while still looking configured, and a cap at or above
    /// the step cap can never bind, because the run halts on its step cap first.
    /// DRIVE-04's cap is only a control if exceeding it is possible and changes
    /// what the run does.
    ///
    /// The comparison is against the value [`bounds::resolve`] **returned** for
    /// this run, never [`bounds::DEFAULT_MAX_STEPS`]: under a default of 20 a cap
    /// of 2 and a cap of 3 would both be accepted by a run bounded at two steps,
    /// which is the exact shape of the Critical Phase 20's review found.
    pub max_escalations: Option<u32>,
    /// The digest identifying the plan the user reviewed and approved.
    ///
    /// **Required for a `--goal`-only run, and its absence is a refusal rather
    /// than a default yes.** Approval is an explicit recorded act: this value is
    /// [`journal::approval_digest`] over the decomposed plan **and** the
    /// disclosed files, so it identifies both the plan that was reviewed and the
    /// bytes that will actually enter the prompts. An approval that does not
    /// cover the second half is not an approval of what will actually run
    /// (research Q4).
    ///
    /// It is re-checked at spawn as well as here, because time and other
    /// processes pass between the two: the decomposition happens above the run
    /// and the first agent starts after the lock, the journal and the envelope,
    /// and a `git pull` in that window rewrites the very files the approval
    /// covered.
    ///
    /// Ignored when the run supplies a `--command` or a `--target-phase`, both
    /// of which are already machine-checkable and decompose nothing.
    pub approved_plan: Option<String>,
    /// The run id to record under. **Required for a real run**; `None` is legal
    /// only with [`dry_run`](Self::dry_run).
    ///
    /// It is supplied by the caller and never generated here: the TUI supplies it
    /// so it knows what to look for afterwards, and the driver owns the record,
    /// because `run.json` carries the driver's own pid and pgid which only the
    /// driver knows (D-03). A real run with `None` is refused by [`drive`] with
    /// [`DriveError::RunIdRequired`] before anything is created — an id the
    /// driver invented for itself would appear on no caller's argv, and that is
    /// precisely what made a run invisible to `liveness::probe` (CR-04).
    pub run_id: Option<String>,
    /// Preview only, execute nothing (D-24). Plan 17-04 implements it.
    pub dry_run: bool,
    /// Free text recorded into `RunRecord.goal`, and the input [`goal`]
    /// decomposes into a plan over [`router::SAFE_COMMAND_ALPHABET`].
    ///
    /// **This doc used to claim the goal was recorded and nothing more, and
    /// Phase 21 is where that stopped being true.** The correction rides the
    /// commit that
    /// falsified it, per the `src/driver/dry_run.rs:78-83` precedent. What is
    /// true now: the goal is decomposed **once, before the loop**, through the
    /// model seam, and the decomposition is **refused rather than repaired** when
    /// it is not machine-checkable — a command outside the alphabet, a target
    /// phase the roadmap does not declare, or a terminal state that does not
    /// reduce to [`router::is_goal_met`] each produce a `goal_`-prefixed refusal
    /// rather than a repaired plan. Repairing a model's answer would make the
    /// re-parse advisory, and the re-parse is the control (SAFE-08).
    ///
    /// The text itself is still never treated as an instruction: it is a
    /// third-party string, and what the driver acts on is the validated
    /// [`goal::GoalPlan`] rather than the prose.
    pub goal: Option<String>,
    /// Test and development only: the program to spawn instead of `claude`.
    ///
    /// **Debug builds only (D-30, WR-16).** `src/cli.rs` carries the full
    /// reasoning and the accepted consequence; the gate is repeated here rather
    /// than stopping at the parser because a field that survives into the
    /// release binary keeps `ClaudeExecutor::with_program` reachable from
    /// anything that can build a `DriveArgs`, and "the only caller today is the
    /// CLI" is a fact about today, not a property of the type.
    #[cfg(debug_assertions)]
    pub claude_program: Option<PathBuf>,
    /// Test and development only: leading arguments for `claude_program`.
    ///
    /// Debug builds only, for the same reason and by the same decision as
    /// [`claude_program`](Self::claude_program).
    #[cfg(debug_assertions)]
    pub claude_args: Vec<OsString>,
}

/// Whether a **real** run may be started on a platform whose liveness
/// determinability is `liveness_supported` (CR-05).
///
/// **The principle in one line: it must be impossible to start what cannot be
/// stopped.** Where `driver::liveness`'s `/proc` technique does not apply, the
/// kill switch cannot tell a running agent from a finished one, the
/// reconciliation scan cannot tell a live run from a crashed one, and the
/// concurrency cap cannot count. A run started there is an autonomous agent with
/// git and push rights that the tool has no honest way to observe or terminate.
/// The refusal is the only answer that does not lie about that.
///
/// **Pure, and that is what makes it testable at all.**
/// [`liveness::LIVENESS_SUPPORTED`] is `true` on the only platform CI runs, so a
/// gate written as a `#[cfg]` block or an inline read of the constant could never
/// have its refusal branch executed — the one branch whose whole job is to be
/// honest with a user this project cannot otherwise reach. Fed the non-Linux
/// answer directly, it is one assertion.
///
/// `dry_run` is exempt, and the exemption is not an oversight: a preview starts
/// no process, so there is nothing to stop and nothing to observe. A preview that
/// stopped working on the platform where a run cannot run would be useless
/// exactly where it is most useful (D-22, D-24).
fn platform_refusal(liveness_supported: bool, dry_run: bool) -> Option<DriveError> {
    if liveness_supported || dry_run {
        return None;
    }
    Some(DriveError::UnsupportedPlatform {
        detail: "the /proc liveness probe that the kill switch and the reconciliation \
                 scan are both built on is unavailable here, so a run could be started \
                 but neither observed nor stopped"
            .to_string(),
    })
}

/// Whether the command sources a caller supplied name exactly one run.
///
/// **Pure, and shaped as `Option<DriveError>` to match [`platform_refusal`]
/// beside it:** the chain in [`drive`] reads as a sequence of refusals, and a
/// function returning "the refusal, if any" reads the same way at the call site
/// as the one above it.
///
/// Both directions are refused, and neither is pedantry:
///
/// * **Neither present** is a run with nothing to do. Clap used to make this
///   unrepresentable by requiring `--command`; the moment `--target-phase`
///   became an alternative, "required" stopped being expressible in the parser
///   and became this function's job.
/// * **Both present** is ambiguous, and the ambiguity is dangerous rather than
///   merely untidy. One of the two would have to win silently, and whichever it
///   was, the run's terminal record would name a mode the caller did not choose
///   — while the *other* mode's bounds went unenforced.
///
/// **`goal` became a third source in Phase 21, and this doc used to describe
/// two.** The correction rides the commit that falsified it, per the
/// `src/driver/dry_run.rs:78-83` precedent. A goal supplied *alongside* either
/// of the other two is recorded prose and nothing more — both of those sources
/// are already machine-checkable, so there is nothing to decompose. A goal
/// supplied **alone** is the DRIVE-01 invocation: the plan
/// [`goal`](crate::driver::goal) decomposes it into is what tells the router
/// which phase the run is driving toward, and its terminal step's phase becomes
/// the `--target-phase` the loop would otherwise have been given directly. It is
/// not a fourth execution model — it resolves *into* the routed one, above the
/// run, before anything is created.
fn command_source_refusal(
    command: Option<&str>,
    target_phase: Option<&str>,
    goal: Option<&str>,
) -> Option<DriveError> {
    match (command, target_phase) {
        (Some(_), None) | (None, Some(_)) => None,
        (None, None) if goal.is_some_and(|goal| !goal.trim().is_empty()) => None,
        (None, None) => Some(DriveError::NoCommandSource),
        (Some(_), Some(_)) => Some(DriveError::AmbiguousCommandSource),
    }
}

/// The routed-mode marker written into `run.json`'s `gsd_command`.
///
/// **Exactly one routed literal exists in the tree, and this is it.** There were
/// briefly two — this one and a `ROUTED_PREVIEW` the dry-run used in place of a
/// command it declined to compute. The preview now renders the router's *own*
/// first selection alongside a scope that refuses to call it a sequence
/// ([`dry_run::PreviewScope`]), so it has a real command to show and needs no
/// marker at all. Collapsing to one constant removes the drift risk that having
/// two of them created.
///
/// It names the journal records that hold the actual sequence, because the whole
/// purpose of writing a marker rather than a command is to send the reader
/// somewhere that is not a guess.
///
/// **It may never be the empty string or an argv fragment.**
/// `the_routed_record_marker_cannot_be_misread_as_absent_or_as_an_argv_fragment`
/// pins that rather than pinning the literal alone: `""` already means "field
/// absent" on the tolerant read path (D-30), and a `--target-phase 3`-shaped
/// value reads as a pasteable command line while being nothing of the kind.
pub const ROUTED_RECORD_MARKER: &str = "(routed: see the decided journal records)";

/// The rendered preview for whichever execution model this invocation names.
///
/// [`command_source_refusal`] has already established that exactly one of the
/// two is present, so the last arm is unreachable; it is spelled out rather than
/// `unwrap`ped because a preview is a foreground command and a panic here would
/// tell the user nothing about what was wrong with their invocation.
///
/// **The two modes make different honesty claims, so they render through
/// different entry points** rather than through one that would have to hedge:
/// a supplied command is the complete sequence, and a routed run shows the
/// router's first selection with the fact that it continues stated plainly.
fn preview_text(
    project: &DrivableProject,
    command: Option<&str>,
    target_phase: Option<&str>,
) -> String {
    match (command, target_phase) {
        (Some(command), _) => dry_run::render(&dry_run::build_report(project, command)),
        (None, Some(target_phase)) => {
            dry_run::render_routed(&dry_run::build_routed_report(project, target_phase))
        }
        (None, None) => dry_run::render(&dry_run::build_report(project, "")),
    }
}

/// Run one GSD command against `alias`, or refuse.
///
/// The order of the body is the decision:
///
/// 1. Look the alias up, refusing an unregistered one with
///    [`OptInError::UnknownAlias`].
/// 2. Pass the entry through the **single production call** to the capability
///    constructor. That call site is unique on purpose, and the uniqueness is a
///    property `tests/spawn_seam_guard.rs` can check while "every branch
///    remembers to gate" is not.
/// 3. Refuse an invocation that is malformed **as an invocation**: no command
///    source or two (`command_source_refusal`), a `--target-phase` that is not
///    a single plain path component ([`DriveError::TargetPhaseInvalid`]), or
///    caps that cannot be honoured ([`bounds::resolve`] and, on the adjacent
///    line and against the step cap the first of them returned,
///    [`escalate::resolve`]). All three sit **above**
///    the dry-run branch, because each is answered identically whether or not
///    the run is real and a preview that answered them differently would be
///    previewing something the user cannot run (WR-09).
/// 4. Only then branch on anything else — including the dry-run branch plan
///    17-04 adds. **Gating before the preview branch is stricter than CTRL-03
///    requires, and it is deliberate:** one gate call site is mechanically
///    verifiable, two are an invitation to add a third.
/// 5. Refuse a real run that cannot be stopped ([`platform_refusal`]), that
///    cannot be identified ([`DriveError::RunIdRequired`]), or whose id is not a
///    single plain path component ([`DriveError::RunIdInvalid`], D-27). All
///    three sit **after** the dry-run branch, because a preview creates no run
///    to identify and starts no process to stop; all three sit **before**
///    `dispatch`, so a refused run has created nothing at all: no lock file, no
///    run directory, no `run.json`, no journal.
/// 6. Dispatch to the platform handler, which is the run body on Unix and a
///    typed refusal everywhere else (D-05).
pub async fn drive(mut args: DriveArgs, config: &Config) -> Result<(), DriveError> {
    let entry = config
        .projects
        .get(&args.alias)
        .ok_or_else(|| OptInError::UnknownAlias {
            alias: args.alias.clone(),
        })?;

    let project = DrivableProject::from_registry(&args.alias, entry)?;

    // **The three refusals about WHAT WAS ASKED FOR, before the dry-run branch**
    // — unlike the ones about *running*, which sit below it. The run-id and
    // platform refusals are about a run: a preview creates nothing to identify
    // and starts no process to stop, so neither is about anything a preview
    // does. These three are about the invocation itself, they are answered
    // identically whether or not the run is real, and a preview that answers
    // them differently is answering a different question from the one the user
    // asked (WR-09).
    //
    // All three are pure and none creates anything on disk.
    //
    // Still after the capability gate, so `from_registry` keeps its single
    // production call site and an unregistered alias is refused first.

    // A preview of nothing has nothing to show, and a preview of two
    // conflicting sources cannot say which it previewed.
    if let Some(refusal) = command_source_refusal(
        args.command.as_deref(),
        args.target_phase.as_deref(),
        args.goal.as_deref(),
    ) {
        return Err(refusal);
    }

    // The same question about `--target-phase` the run id is asked below, at the
    // same seam and for the same reason (D-27, T-20-03). The value is used
    // **only** as a map key into `ProjectState::phase_disk_statuses` and the
    // iteration loop composes no path from it — but "no caller composes a path
    // from it today" is a fact about today, not a property of the type, and the
    // run id's history is exactly what that distinction cost:
    // `--run-id '../../../../escaped'` was reproduced against the shipped tree
    // writing outside the project with exit 0. One refusal at the seam is
    // cheaper than auditing every future use.
    //
    // **It sits above the preview because the preview renders the value**
    // (WR-09). `--dry-run --target-phase '../../../escaped'` used to print that
    // token verbatim inside a pasteable command line, and
    // `RouterAction::command_for`'s doc asserts the phase "arrived on argv and
    // was validated at the seam" — a claim that was false on exactly this path.
    if let Some(target_phase) = args.target_phase.as_deref() {
        if !journal::is_plain_path_component(target_phase) {
            return Err(DriveError::TargetPhaseInvalid {
                target_phase: target_phase.to_string(),
            });
        }
    }

    // The caps, resolved and refused before anything is created, so a run asked
    // for with a cap that disables a detector leaves nothing on disk. The
    // resolved value is recomputed in the run body rather than threaded through
    // `dispatch`: `bounds::resolve` is a pure function of two `Option`s on
    // `args`, so a second call cannot disagree with this one, and threading it
    // would widen a signature three later plans in this phase also touch.
    //
    // **Also above the preview** (WR-09). `--dry-run --max-steps 0` used to
    // render a clean preview of an invocation that would be *refused* if run for
    // real, and a preview whose whole purpose is "what would happen" answering
    // anything but that is worse than no preview.
    let run_bounds =
        bounds::resolve(args.max_steps, args.wall_clock_cap_secs).map_err(DriveError::from)?;

    // The model seam's budget, on the adjacent line and in the same group, for
    // the identical reason: DRIVE-04's cap is a control only if a value that
    // cannot bind is refused, and refusing it here means a run asked for with
    // such a cap leaves nothing on disk.
    //
    // **It is called with the step cap `bounds::resolve` just returned** — never
    // with `args.max_steps`, which is an `Option` that may be `None`, and never
    // with `bounds::DEFAULT_MAX_STEPS`. Comparing against the constant is the
    // exact shape of the Critical Phase 20's review found: under a default of 20
    // a cap of 2 and a cap of 3 are both accepted, so a run bounded at two steps
    // carries an escalation cap that can never fire while looking configured.
    // That is why the resolved value is now bound rather than discarded.
    //
    // **The returned budget is now bound rather than discarded**, and that is
    // Phase 21: the goal decomposition below spends one consultation out of it
    // and the ambiguity seam inside the loop spends the rest, so one budget has
    // to survive from here to the last iteration. A second `resolve` call in the
    // run body would be a second thing that can disagree about how many
    // consultations have already happened.
    #[cfg_attr(not(unix), allow(unused_mut))]
    let mut budget =
        escalate::resolve(args.max_escalations, run_bounds.max_steps).map_err(DriveError::from)?;

    if args.dry_run {
        // Positioned **after** the gate and **before** anything Unix-only, and
        // both halves of that sentence are decisions:
        //
        // Gating first means `--dry-run` against a non-opted-in project is
        // refused too. That is stricter than CTRL-03 requires, and it keeps
        // `from_registry` at exactly one call site — a property a test can check,
        // while "every branch remembers to gate" is not.
        //
        // Branching before the lock means a preview never contends with a live
        // run, which is what a user previewing a busy project expects. Nothing
        // below this point runs: no lock is acquired, no journal is started, no
        // executor is constructed, and no run id is even generated (D-23).
        //
        // stdout, not the journal and not the TUI (D-24). A detached driver's
        // stdio is null, so a dry-run is by definition a foreground invocation.
        //
        // **No blocking syscall inside an `async fn`** (D-28, WR-10).
        // `build_report` makes two synchronous `std::process::Command` calls to
        // `git`, and a `git` invocation on a large or cold repository is not
        // microseconds. **The deadlock this discipline prevents was OBSERVED,
        // not theorised** — `tests/driver_lock.rs:201-215` records a blocking
        // `flock` inside an `async fn` defeating `tokio::time::timeout` outright
        // on a current-thread runtime, because `Timeout::poll` polls its inner
        // future inline and a parked thread polls nothing at all.
        //
        // The preview is a foreground CLI invocation today, so the cost of
        // getting this wrong is only a stalled process. It is wrapped anyway
        // because Phase 18 puts a TUI-side caller on this path, and there the
        // same stall is a frozen frame the user cannot escape.
        //
        // The token is **cloned** into the closure rather than moved, and that
        // is not fussiness: the fallback below needs one too, and building a
        // second one would mean a second `DrivableProject::from_registry` call
        // site — the exact uniqueness `tests/spawn_seam_guard.rs` exists to
        // check, and a property a comment cannot hold.
        // The refusal above guarantees exactly one of the two is present, so the
        // `Routed` arm is the only case where `command` is absent.
        //
        // **The mode is passed through rather than flattened to a string**, and
        // the pinned `dry_run::SECTION_COMMANDS` prose has been corrected to
        // match (research Pitfall 6). It used to say a routed sequence was a
        // single honest command; the preview now shows the router's own first
        // selection and states plainly that the run continues past it.
        let command = args.command.clone();
        let target_phase = args.target_phase.clone();
        let cloned = project.clone();
        let rendered = match tokio::task::spawn_blocking(move || {
            preview_text(&cloned, command.as_deref(), target_phase.as_deref())
        })
        .await
        {
            Ok(rendered) => rendered,
            // Reachable only if the closure panicked or the runtime is shutting
            // down — `build_report` does neither, and `git_ops` reports a failed
            // shell-out as data rather than by unwinding. Re-running inline is
            // `ClaudeExecutor::capture_snapshot`'s answer to the same question
            // and this repository's established one: it keeps the preview
            // honest on a path no healthy run reaches, at the cost of a blocking
            // call in a process that is already ending anyway.
            Err(err) => {
                tracing::warn!(
                    panicked = err.is_panic(),
                    "the dry-run report task did not run to completion",
                );
                preview_text(&project, args.command.as_deref(), args.target_phase.as_deref())
            }
        };
        println!("{rendered}");
        return Ok(());
    }

    // Both refusals below are positioned, and the position is the decision.
    //
    // **After the gate**, so `from_registry` keeps its single production call
    // site and an unregistered or non-opted-in alias is still refused first
    // (D-16). **After the dry-run branch**, because a preview creates no run to
    // identify and starts no process to stop, so neither refusal is about
    // anything a preview does (D-22, D-24). And **before `dispatch`**, so a
    // refused run has created nothing at all: no lock file, no run directory, no
    // `run.json`, no journal.
    if let Some(refusal) = platform_refusal(liveness::LIVENESS_SUPPORTED, args.dry_run) {
        return Err(refusal);
    }

    // Two questions about the same field, in the order they can be answered:
    // is there an id at all, and is the id a name rather than a path (D-27).
    // The second refusal is here, at the seam where a bad id first arrives,
    // rather than only inside `journal::run_paths` — that helper's `Option` is
    // what makes the hole *unreachable*, and this is what makes the common case
    // fail **loudly and non-zero** instead of quietly yielding a run that did
    // nothing. `--run-id '../../../../escaped'` was reproduced against the
    // shipped tree writing outside the project with exit 0.
    let Some(run_id) = args.run_id.as_deref() else {
        return Err(DriveError::RunIdRequired);
    };
    if !journal::is_plain_path_component(run_id) {
        return Err(DriveError::RunIdInvalid {
            run_id: run_id.to_string(),
        });
    }

    // The envelope assertion, and its position is the decision in three clauses
    // (D-24).
    //
    // **After the capability gate**, so `DrivableProject::from_registry` keeps
    // its single production call site and a non-opted-in alias is still refused
    // first — a project the user never opted in must not get so far as having an
    // envelope reasoned about.
    //
    // **After the dry-run branch**, because a preview starts no process and
    // creates no run to protect; refusing a preview because an envelope could
    // not be established would refuse the one output that explains why.
    //
    // **Before `dispatch`**, so a refused run has created nothing at all: no
    // lock file, no run directory, no run record, no journal.
    //
    // That last clause is also why this refusal writes **no**
    // `journal::JournalEvent::Parked` event, even though its reason is
    // `envelope_assertion_failed` and every other producer of that reason does.
    // There is no journal for the run at this point — there is no run — so the
    // evidence D-25 requires is the typed error, the non-zero process exit and
    // the stderr line `src/main.rs` prints, all three of which are readable
    // without consulting anything a model said. **Forward constraint:** should
    // this assertion ever become reachable after a journal has been opened, the
    // appender is `envelope::park`, the same one the hook and guard re-entries
    // call — one appender, not a second one written here speculatively.
    if let Some(refusal) = envelope_refusal(&project) {
        return Err(refusal);
    }

    // **THE FIRST OF THE TWO MODEL SEAMS: goal decomposition, once, above the
    // run.** The capability is constructed here and consumed here, on the same
    // expression, and this is its only construction site under `src/`.
    //
    // **The move is the mechanism, not the comment.** `decompose` takes
    // `self` by value, so after this line there is no capability left — a
    // second decomposition anywhere, and in particular inside `run`'s
    // `'iterations` loop, does not compile. That is what makes "the driver may
    // never enqueue itself a goal from an artifact created during its own run"
    // a type-level property rather than a control-flow habit somebody has to
    // keep. `tests/spawn_seam_guard.rs` additionally scans for a construction
    // inside the loop's label scope and for a `Clone`/`Copy` derive that would
    // make the move a formality.
    //
    // **Positioned below the dry-run branch, unlike the refusals above it, and
    // the position is the decision.** Every refusal above this line is pure —
    // it opens no file and starts no process — which is why each is answered
    // identically for a preview and for a real run. A decomposition *spawns a
    // process*, and D-23 is explicit that a preview does none. A preview of a
    // goal-only invocation therefore renders without consulting a model at all.
    //
    // Positioned above `dispatch` for the reason every refusal in this function
    // is: a refused decomposition has created nothing at all — no lock file, no
    // run directory, no `run.json`, no journal — because none of those exist
    // until `execute_run` builds them.
    //
    // **Unix-gated because the seam is the executor's spawn seam**, which is
    // Unix by construction (D-05). A non-Unix build refuses at `dispatch`
    // immediately below regardless, so nothing is skipped that would otherwise
    // have run.
    #[cfg(unix)]
    let decomposed = match run::GoalDecomposition::from_argv_goal(&args) {
        None => None,
        Some(decomposition) => Some(
            decomposition
                .decompose(&project, &args, &mut budget, run_bounds.max_steps)
                .await?,
        ),
    };
    #[cfg(not(unix))]
    let decomposed: Option<goal::GoalPlan> = None;

    // **The approval, bound to the plan AND to the bytes that will enter the
    // prompts, and refused when either half is unmatched or absent.**
    //
    // Approval is an explicit recorded act. There is deliberately no path here
    // that infers it, defaults it, or times out into it: a `--goal`-only run
    // whose `--approved-plan` is absent is refused with a message that says
    // *absent* rather than reporting a mismatch, because "nobody approved this"
    // and "what was approved has changed" are different statements to the person
    // reading the refusal.
    //
    // The digest covers `goal::plan_digest` **and** the disclosed prompt inputs
    // together, through one mechanism, because two comparisons are two places a
    // caller can check one and forget the other. Plan 21-03 closed the files
    // half at the spawn gate; this closes the plan half and binds the two.
    #[cfg(unix)]
    let approved_plan = match decomposed.as_ref() {
        None => None,
        Some(plan) => Some(approve_plan(&project, &args, plan)?),
    };
    #[cfg(not(unix))]
    let approved_plan: Option<journal::ApprovedPlan> = None;

    // The decomposed plan's terminal step is the phase the run drives toward,
    // written back onto `args` so every consumer below — the run record, the
    // argv digest, the `CommandSource` — reads one field rather than each
    // deciding for itself which of two places the target lives in.
    //
    // **It is the LAST step's phase, never the first's** (see
    // `run::plan_target_phase`), and it has already passed a *stricter* check
    // than the `--target-phase` seam above: `goal::legality` applies
    // `journal::is_plain_path_component` **and** requires the roadmap to declare
    // it, where the argv seam applies only the first.
    if let Some(plan) = approved_plan.as_ref() {
        args.target_phase = Some(plan.target_phase.clone());
    }

    dispatch(project, &args, entry, approved_plan, budget).await
}

/// Record the approval for `plan`, or refuse the run.
///
/// **Pure of decisions and impure only in the one way it has to be**: it reads
/// the disclosed files' current digests, because an approval that does not cover
/// the bytes that will enter the prompt is not an approval of what will actually
/// run. Everything it *judges* is handed to [`journal::recheck_approval`], which
/// opens nothing and is therefore testable on either side of every boundary.
///
/// The three outcomes are the three the caller needs:
///
/// * **no `--approved-plan` at all** → [`DriveError::PlanApprovalRequired`],
///   carrying the digest the user would approve, so the refusal is actionable in
///   one step rather than being a bug report;
/// * **a digest that covers a different plan** → the plan half changed;
/// * **a digest whose plan half matches and whose file half does not** → the
///   disclosed bytes changed under the approval.
///
/// The last two share a variant carrying [`journal::ApprovalRefusal`], which is
/// what keeps *which half* readable without a second error type.
#[cfg(unix)]
fn approve_plan(
    project: &DrivableProject,
    args: &DriveArgs,
    plan: &goal::GoalPlan,
) -> Result<journal::ApprovedPlan, DriveError> {
    let plan_digest = goal::plan_digest(plan);
    let prompt_inputs = crate::registry::current_prompt_inputs(project.root());
    let digest = journal::approval_digest(&plan_digest, &prompt_inputs);

    // The plan rendered as typed tokens, built once here and used for both the
    // refusal the user reads and the record the run carries — one producer, so
    // what the reviewer approved and what the record says cannot disagree.
    let steps: Vec<String> = plan
        .steps
        .iter()
        .map(|step| {
            format!(
                "command={} phase={} terminal={}",
                step.command.verb(),
                step.target_phase,
                step.terminal_state.as_str(),
            )
        })
        .collect();

    let recorded = args.approved_plan.as_deref().map(|approved| {
        // The record built from what the caller approved, so the comparison
        // below is `recheck_approval`'s — one predicate, used here and again at
        // spawn, rather than an equality written twice.
        journal::ApprovedPlan {
            steps: Vec::new(),
            target_phase: String::new(),
            // The plan half is the digest of the plan we just decomposed: the
            // caller approved a digest, not a plan, so a mismatch on the whole
            // value is what "this is not what you approved" means. Recording the
            // observed plan digest here lets `recheck_approval` report WHICH half
            // moved rather than only that something did.
            plan_digest: plan_digest.clone(),
            approval_digest: approved.to_string(),
            approved_at: String::new(),
            extra: serde_json::Map::new(),
        }
    });

    if recorded.is_none() {
        return Err(DriveError::PlanApprovalRequired { digest, steps });
    }

    journal::recheck_approval(recorded.as_ref(), &plan_digest, &prompt_inputs)
        .map_err(DriveError::PlanApprovalStale)?;

    Ok(journal::ApprovedPlan {
        steps,
        // `legality` refuses an empty plan, so the last step is always present.
        // Spelled out rather than `unwrap`ped because a detached driver that
        // panicked here would leave no terminal record at all.
        target_phase: run::plan_target_phase(plan).unwrap_or_default().to_string(),
        plan_digest,
        approval_digest: digest,
        approved_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        extra: serde_json::Map::new(),
    })
}

/// Whether an envelope can be established for `project` at all.
///
/// **An assertion, not the establishment.** Establishment writes four files and
/// runs a probe, and it happens at the single production `ExecutionOptions`
/// construction site in [`run`] where its results are consumed. This answers the
/// cheaper question that can be answered before anything is created: is there a
/// root to put an envelope under, and is this alias a name that may have one?
/// Both are pure — no filesystem write, no process, no network — which is what
/// lets the refusal sit in the ordered chain above rather than after the lock.
///
/// Shaped as `Option<DriveError>` rather than `Result<(), _>` deliberately, to
/// match [`platform_refusal`] beside it: this chain reads as a sequence of
/// refusals, and a function that returns "the refusal, if any" reads the same
/// way at the call site as the one above it.
///
/// The **later** failure — an establishment that gets a root and an alias and
/// still cannot write — returns the identical [`DriveError::EnvelopeAssertionFailed`]
/// variant from `run::execute_run`, before the lock and before the journal. Two
/// positions, one refusal, one park reason.
fn envelope_refusal(project: &DrivableProject) -> Option<DriveError> {
    if envelope::envelope_root().is_none() {
        return Some(DriveError::EnvelopeAssertionFailed {
            reason: ParkReason::EnvelopeAssertionFailed,
            detail: "no application data directory is resolvable, and the envelope refuses \
                     to fall back to a directory that could sit inside a repository"
                .to_string(),
        });
    }
    if envelope::envelope_dir(project.alias()).is_none() {
        // The alias is not echoed. It came from the registry rather than from an
        // argument, and every other refusal in this file that echoes a value
        // echoes one the caller just typed.
        return Some(DriveError::EnvelopeAssertionFailed {
            reason: ParkReason::EnvelopeAssertionFailed,
            detail: "this alias is not a single plain path component, so no envelope \
                     directory can be sanctioned for it"
                .to_string(),
        });
    }
    None
}

/// The Unix run body.
#[cfg(unix)]
async fn dispatch(
    project: DrivableProject,
    args: &DriveArgs,
    entry: &RegisteredProject,
    approved_plan: Option<journal::ApprovedPlan>,
    budget: escalate::EscalationBudget,
) -> Result<(), DriveError> {
    run::execute_run(project, args, entry, approved_plan, budget).await
}

/// The typed refusal every non-Unix platform gets (D-05).
#[cfg(not(unix))]
async fn dispatch(
    project: DrivableProject,
    args: &DriveArgs,
    entry: &RegisteredProject,
    approved_plan: Option<journal::ApprovedPlan>,
    budget: escalate::EscalationBudget,
) -> Result<(), DriveError> {
    let _ = (project, args, entry, approved_plan, budget);
    Err(DriveError::UnsupportedPlatform {
        detail: "process-group detachment is a Unix facility and has no portable equivalent"
            .to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(alias: &str) -> DriveArgs {
        DriveArgs {
            alias: alias.to_string(),
            command: Some("/gsd-progress".to_string()),
            target_phase: None,
            max_steps: None,
            wall_clock_cap_secs: None,
            max_escalations: None,
            approved_plan: None,
            run_id: None,
            dry_run: false,
            goal: None,
            #[cfg(debug_assertions)]
            claude_program: None,
            #[cfg(debug_assertions)]
            claude_args: Vec::new(),
        }
    }

    #[tokio::test]
    async fn drive_refuses_an_unknown_alias_without_touching_disk() {
        let scratch = tempfile::TempDir::new().expect("temp dir");
        let config = Config::new();

        let err = drive(args("nosuchalias"), &config)
            .await
            .expect_err("an unregistered alias must never reach a spawn");

        assert!(
            err.to_string().contains("nosuchalias"),
            "the refusal must name the alias so it is actionable, got: {err}"
        );
        assert_eq!(
            std::fs::read_dir(scratch.path())
                .expect("the scratch dir is readable")
                .count(),
            0,
            "a refused drive writes nothing at all (CTRL-03)"
        );
    }

    /// A project root with `.planning/`, plus a config whose entry carries a
    /// driver opt-in — so the gate at the top of [`drive`] passes and the
    /// refusals below are the branch under test rather than an incidental one.
    fn opted_in(root: &std::path::Path) -> Config {
        std::fs::create_dir_all(root.join(".planning")).expect("scratch .planning");
        let mut config = Config::new();
        config.projects.insert(
            "demo".to_string(),
            RegisteredProject {
                path: root.to_path_buf(),
                added: "2026-07-29T12:00:00Z".to_string(),
                driver_opt_in: Some(crate::config::DriverOptIn {
                    opted_in_at: "2026-07-29T11:59:00Z".to_string(),
                    claude_md_digest: None,
                    // See `dry_run.rs`: the real disclosed set, so this fixture
                    // is not refused by the spawn gate's drift check.
                    prompt_inputs: crate::registry::current_prompt_inputs(root),
                    extra: Default::default(),
                    branch_namespace: None,
                    credential: None,
                    pr_cap_per_24h: None,
                    pr_cap_per_run: None,
                }),
                extra: Default::default(),
            },
        );
        config
    }

    #[tokio::test]
    async fn drive_refuses_a_real_run_that_carries_no_run_id_without_touching_disk() {
        let root = tempfile::TempDir::new().expect("temp dir");
        let config = opted_in(root.path());

        let mut args = args("demo");
        args.run_id = None;

        let err = drive(args, &config)
            .await
            .expect_err("a real run with no run id must be refused");

        assert!(
            matches!(err, DriveError::RunIdRequired),
            "the refusal must be the typed one, got: {err:?}"
        );
        assert!(
            err.to_string().contains("--run-id"),
            "the refusal must name the flag, or the caller cannot act on it, got: {err}"
        );
        assert!(
            !root.path().join(".planning/meta-manager").exists(),
            "a refused run must have created NOTHING. A refusal that left a runs \
             root, a lock file or a journal behind would mean something had already \
             started before the check ran (CR-04)"
        );
    }

    #[tokio::test]
    async fn drive_still_previews_when_there_is_no_run_id_because_a_preview_creates_no_run() {
        let root = tempfile::TempDir::new().expect("temp dir");
        let config = opted_in(root.path());

        let mut args = args("demo");
        args.run_id = None;
        args.dry_run = true;

        drive(args, &config)
            .await
            .expect("a preview creates no run to identify, so the run-id refusal must not reach it");

        // The inertness control, over the one directory this call could have
        // created. D-23's full zero-write property — including the git reflog —
        // is `tests/driver_dry_run.rs`'s and is not duplicated here.
        assert!(
            !root.path().join(".planning/meta-manager").exists(),
            "a preview writes nothing (D-23)"
        );
    }

    #[test]
    fn the_platform_gate_refuses_a_real_run_where_liveness_cannot_be_determined() {
        let refusal = platform_refusal(false, false)
            .expect("a real run must be refused where liveness cannot be determined");
        assert!(
            matches!(refusal, DriveError::UnsupportedPlatform { .. }),
            "the refusal reuses the accepted-limitation variant rather than adding \
             a second one, got: {refusal:?}"
        );
        assert!(
            refusal.to_string().contains("/proc"),
            "the detail must name the facility concretely, in the style D-05 \
             requires — 'not supported' with no noun is indistinguishable from a \
             bug, got: {refusal}"
        );

        assert!(
            platform_refusal(true, false).is_none(),
            "the ordinary case must be transparent"
        );
        assert!(
            platform_refusal(false, true).is_none(),
            "a PREVIEW must survive the refusal, and the exemption is deliberate \
             rather than an oversight: a preview starts no process, so there is \
             nothing to stop and nothing to observe. A preview that stopped \
             working on the platform where a run cannot run would be useless \
             exactly where it is most useful (D-22, D-24)"
        );
    }

    #[test]
    fn drive_args_carry_exactly_one_command_source_and_never_a_supplied_sequence() {
        // This test replaced `drive_args_carry_the_single_command_the_router_
        // phase_will_replace`, which asserted the Phase 17 shape it was named
        // for. Phase 20 built the router that test was waiting on, so the claim
        // it pinned stopped being true and the assertion moved with the code
        // rather than being deleted.
        let args = args("demo");
        // An `Option<String>`, never a `Vec<String>`. A routed run's sequence is
        // DERIVED per iteration from what the previous one left on disk, so a
        // field that accepted a supplied list would be a third execution model
        // that neither the router nor the bounds know about.
        assert_eq!(args.command.as_deref(), Some("/gsd-progress"));
        assert_eq!(
            args.target_phase, None,
            "the two sources are mutually exclusive; a fixture carrying both \
             would exercise the refusal rather than the mode it names"
        );
        assert!(
            !args.dry_run,
            "a real run is the default; the preview is opt-in"
        );
    }

    #[test]
    fn a_run_with_no_command_source_at_all_is_refused_before_anything_is_created() {
        assert!(
            matches!(
                command_source_refusal(None, None, None),
                Some(DriveError::NoCommandSource)
            ),
            "clap used to make this unrepresentable by requiring --command. The \
             moment --target-phase became an alternative, 'exactly one of these' \
             stopped being expressible in the parser, and a run with nothing to do \
             would otherwise reach the lock and the journal before anybody noticed"
        );

        // A goal made of nothing but whitespace is nothing to do either. The
        // trim is what stops `--goal ' '` from becoming a third command source
        // that decomposes an empty string, which the seam would answer somehow
        // and `legality` would then be asked to judge.
        assert!(
            matches!(
                command_source_refusal(None, None, Some("   ")),
                Some(DriveError::NoCommandSource)
            ),
            "a blank goal is not a command source"
        );
    }

    #[test]
    fn a_stated_goal_alone_is_a_command_source_because_the_plan_supplies_the_target() {
        // DRIVE-01's invocation: the user says what they want once, in plain
        // language, and the decomposed plan's terminal step is what tells the
        // router which phase the run is driving toward. Against the UNFIXED
        // behaviour — a two-source refusal that knows nothing about goals — this
        // FAILS with `NoCommandSource`, and the run the requirement describes is
        // unrepresentable.
        assert!(
            command_source_refusal(None, None, Some("get phase 22 verified")).is_none(),
            "a stated goal alone must reach the decomposition rather than be \
             refused as a run with nothing to do"
        );
    }

    #[test]
    fn a_run_naming_both_command_sources_is_refused_rather_than_resolved() {
        assert!(
            matches!(
                command_source_refusal(Some("/gsd-progress"), Some("20"), None),
                Some(DriveError::AmbiguousCommandSource)
            ),
            "a precedence rule would let one source win SILENTLY, and whichever it \
             was the run's terminal record would name a mode the caller did not \
             choose while the other mode's bounds went unenforced. An unattended \
             run whose stopping conditions are not the ones asked for is the \
             failure CTRL-06 exists to prevent"
        );

        assert!(
            command_source_refusal(Some("/gsd-progress"), None, None).is_none(),
            "single-command mode must stay transparent"
        );
        assert!(
            command_source_refusal(None, Some("20"), None).is_none(),
            "routed mode must stay transparent"
        );

        // A goal supplied ALONGSIDE either source is recorded prose and nothing
        // more — both of those sources are already machine-checkable, so there
        // is nothing to decompose and no seam fires. A goal must not turn a
        // legal invocation into an ambiguous one.
        assert!(
            command_source_refusal(Some("/gsd-progress"), None, Some("a goal")).is_none(),
            "a goal beside --command is recorded text, not a second source"
        );
        assert!(
            command_source_refusal(None, Some("20"), Some("a goal")).is_none(),
            "a goal beside --target-phase is recorded text, not a second source"
        );
    }

    #[tokio::test]
    async fn a_target_phase_that_is_not_a_plain_path_component_is_refused_without_touching_disk() {
        let root = tempfile::TempDir::new().expect("temp dir");
        let config = opted_in(root.path());

        let mut args = args("demo");
        args.command = None;
        args.target_phase = Some("../../../../escaped".to_string());
        args.run_id = Some("2026-08-19T12-00-00Z-aaaa".to_string());

        let err = drive(args, &config)
            .await
            .expect_err("a traversing target phase must be refused at the seam");

        assert!(
            matches!(err, DriveError::TargetPhaseInvalid { .. }),
            "the refusal must be the typed one, got: {err:?}"
        );
        assert!(
            !root.path().join(".planning/meta-manager").exists(),
            "a refused run must have created NOTHING — the same property the \
             run-id refusal beside it holds, and for the same reason: \
             --run-id '../../../../escaped' was reproduced writing outside the \
             project with exit 0"
        );
    }

    #[tokio::test]
    async fn a_cap_that_would_disable_a_detector_is_refused_without_touching_disk() {
        for (max_steps, wall_clock_cap_secs) in [
            (Some(0), None),
            (None, Some(0)),
            (None, Some(bounds::MAX_WALL_CLOCK_CAP_SECS + 1)),
        ] {
            let root = tempfile::TempDir::new().expect("temp dir");
            let config = opted_in(root.path());

            let mut args = args("demo");
            args.command = None;
            args.target_phase = Some("20".to_string());
            args.run_id = Some("2026-08-19T12-00-00Z-aaaa".to_string());
            args.max_steps = max_steps;
            args.wall_clock_cap_secs = wall_clock_cap_secs;

            let err = drive(args, &config).await.expect_err(
                "a cap that switches a detector off must never reach a spawn \
                 (max_steps={max_steps:?}, wall_clock_cap_secs={wall_clock_cap_secs:?})",
            );

            assert!(
                matches!(err, DriveError::BoundsRefused(_)),
                "the refusal must carry the bounds taxonomy rather than a fresh \
                 string, got: {err:?}"
            );
            assert!(
                !root.path().join(".planning/meta-manager").exists(),
                "CTRL-06's detectors are an unattended run's only stopping \
                 condition, so a run asked for with one of them disabled must \
                 leave nothing at all behind"
            );
        }
    }

    /// **The above-the-run half of the phase's most load-bearing guard.**
    ///
    /// The invocation is `--max-steps 2 --max-escalations 2`: a cap equal to the
    /// run's resolved step cap, which can never bind. Against a resolution that
    /// compared the supplied cap with `bounds::DEFAULT_MAX_STEPS` this FAILS,
    /// because 2 is comfortably below 20, the refusal never happens and the run
    /// starts — creating the run directory this test asserts does not exist.
    #[tokio::test]
    async fn an_escalation_cap_that_can_never_bind_is_refused_before_the_run_exists() {
        for max_escalations in [Some(2), Some(3), Some(0)] {
            let root = tempfile::TempDir::new().expect("temp dir");
            let config = opted_in(root.path());

            let mut args = args("demo");
            args.command = None;
            args.target_phase = Some("20".to_string());
            args.run_id = Some("2026-08-19T12-00-00Z-aaaa".to_string());
            args.max_steps = Some(2);
            args.max_escalations = max_escalations;

            let err = drive(args, &config)
                .await
                .expect_err("a cap that can never bind must never reach a spawn");

            assert!(
                matches!(err, DriveError::EscalationRefused(_)),
                "the refusal must carry the escalation taxonomy rather than a \
                 fresh string, got: {err:?}"
            );
            assert!(
                !root.path().join(".planning/meta-manager").exists(),
                "a refused run creates NOTHING — no run directory, no journal, \
                 no run.json"
            );
        }
    }

    /// The refusal names both numbers **and** something the caller can do, on
    /// `BoundsRefusal::Display`'s terms: a refusal a caller cannot act on is a
    /// bug report rather than an error message.
    #[tokio::test]
    async fn the_escalation_refusal_names_both_numbers_and_an_action() {
        let root = tempfile::TempDir::new().expect("temp dir");
        let config = opted_in(root.path());

        let mut args = args("demo");
        args.command = None;
        args.target_phase = Some("20".to_string());
        args.run_id = Some("2026-08-19T12-00-00Z-aaaa".to_string());
        args.max_steps = Some(4);
        args.max_escalations = Some(9);

        let rendered = drive(args, &config)
            .await
            .expect_err("a cap of 9 under a step cap of 4 can never bind")
            .to_string();

        assert!(
            rendered.contains('9'),
            "the message must name the cap the caller supplied; got: {rendered}"
        );
        assert!(
            rendered.contains('4'),
            "the message must name the RESOLVED step cap it was judged against — \
             not the compiled-in default, which is what the Phase 20 Critical \
             compared with; got: {rendered}"
        );
        assert!(
            rendered.contains("--max-steps") || rendered.contains("Pass at most"),
            "the message must name at least one concrete action; got: {rendered}"
        );
    }

    /// The control arm for the two above: a run whose escalation cap CAN bind is
    /// not refused at this seam, so the refusals are about the cap rather than
    /// about `--target-phase` or the opt-in.
    ///
    /// It still fails afterwards (there is no agent to spawn in a unit test), so
    /// the assertion is on the *kind* of error rather than on success.
    #[tokio::test]
    async fn a_cap_that_can_bind_passes_this_seam() {
        let root = tempfile::TempDir::new().expect("temp dir");
        let config = opted_in(root.path());

        let mut args = args("demo");
        args.command = None;
        args.target_phase = Some("20".to_string());
        args.run_id = Some("2026-08-19T12-00-00Z-aaaa".to_string());
        args.max_steps = Some(4);
        args.max_escalations = Some(3);
        args.dry_run = true;

        drive(args, &config)
            .await
            .expect("a cap one below the step cap binds, so the preview renders");
    }
}
