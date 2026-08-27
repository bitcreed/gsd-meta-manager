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
/// **Every argv-derived string field is a [`payload::NonBlank`], and the domain
/// is the STRUCT rather than a chosen subset of it** (21-15). Four gap-closure
/// cycles each protected a hand-enumerated subset of these values and each
/// enumeration was one item short: three `CommandSource` arms one at a time,
/// then three of the six fields below. The enumeration is now the compiler's:
/// the type refuses a blank in every position, and [`DriveArgs::from_argv`]'s
/// exhaustive destructure makes a seventh field a compile error until somebody
/// classifies it.
#[derive(Debug, Clone)]
pub struct DriveArgs {
    /// The registry alias to drive.
    ///
    /// A [`payload::NonBlank`], so no registry lookup can be asked to name a
    /// value nobody can see. It used to rest on the lookup's incidental
    /// behaviour — an alias of spaces matched no key and so was refused as
    /// *unknown*, which is an accident of the map's contents rather than a
    /// property (pass-5 T-21-15-04).
    pub alias: payload::NonBlank,
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
    ///
    /// **The blankness rule lives in the TYPE, not at a downstream seam.** This
    /// doc used to point at `command_source` as the place a blank `--command`
    /// was refused; as of 21-15 a blank one cannot be represented here at all,
    /// and `command_source`'s check is gone rather than duplicated. The refusal
    /// happens once, at [`DriveArgs::from_argv`].
    pub command: Option<payload::NonBlank>,
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
    ///
    /// The `is_plain_path_component` check named above is **structural** —
    /// traversal tokens, separators, embedded control characters — and it stays
    /// where it is. Blankness is no longer part of it here: a blank value cannot
    /// reach this field (21-15).
    pub target_phase: Option<payload::NonBlank>,
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
    ///
    /// A [`payload::NonBlank`] as of 21-15. A blank token used to rest on
    /// `parse_approval_token`'s incidental behaviour — it refuses a value with
    /// no separator, and a blank one has none — which is a property of the token
    /// grammar rather than of blankness (pass-5 T-21-15-04). It is now refused
    /// at the boundary and the parse remains as defence in depth.
    pub approved_plan: Option<payload::NonBlank>,
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
    ///
    /// A [`payload::NonBlank`] as of 21-15, which is CR-02's structural half: a
    /// run id of one `U+200B` survived `is_plain_path_component`'s old
    /// `str::trim` and named a run directory of one invisible character, in a
    /// run that completed.
    pub run_id: Option<payload::NonBlank>,
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
    ///
    /// **A supplied-but-blank goal is a REFUSAL, not a dropped field** (D-15-2,
    /// 21-15). It used to be recorded verbatim into `RunRecord.goal` through an
    /// `unwrap_or_default`, including when supplied *beside* a visible
    /// `--command`: pass 5 reproduced `--goal '   '` persisting after the lock,
    /// the journal and the run directory. Dropping it silently would fabricate
    /// "no goal was given"; recording it fabricates a goal nobody can read.
    /// Refusal is the only answer that states what happened.
    pub goal: Option<payload::NonBlank>,
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

/// One `drive` invocation exactly as argv supplied it, before anything has been
/// judged.
///
/// **The raw side of the parse boundary.** Clap's own struct fields are `String`
/// and `Option<String>` and stay that way — clap parses the command line, it
/// does not judge payloads — so this is the shape that crosses from the parser
/// into the driver. [`DriveArgs::from_argv`] is the only way across, and what
/// comes out the other side cannot hold a blank in any argv-derived field.
#[derive(Debug, Clone)]
pub struct RawDriveArgs {
    /// The registry alias to drive, unjudged.
    pub alias: String,
    /// `--command`, unjudged.
    pub command: Option<String>,
    /// `--target-phase`, unjudged.
    pub target_phase: Option<String>,
    /// `--max-steps`; a number, so there is no blankness to judge.
    pub max_steps: Option<u32>,
    /// `--wall-clock-cap-secs`; a number.
    pub wall_clock_cap_secs: Option<u64>,
    /// `--max-escalations`; a number.
    pub max_escalations: Option<u32>,
    /// `--approved-plan`, unjudged.
    pub approved_plan: Option<String>,
    /// `--run-id`, unjudged.
    pub run_id: Option<String>,
    /// `--dry-run`; a flag.
    pub dry_run: bool,
    /// `--goal`, unjudged.
    pub goal: Option<String>,
    /// Debug builds only, as on [`DriveArgs`] (D-30, WR-16).
    #[cfg(debug_assertions)]
    pub claude_program: Option<PathBuf>,
    /// Debug builds only, as on [`DriveArgs`] (D-30, WR-16).
    #[cfg(debug_assertions)]
    pub claude_args: Vec<OsString>,
}

/// Judge one supplied argv value, or produce this position's own refusal.
///
/// `None` stays `None` — an absent flag is not a blank one, and the two are
/// different facts a caller downstream still has to tell apart. A supplied value
/// carrying visible content becomes `Some(NonBlank)`. A supplied value carrying
/// none is `refuse`d, with the raw text handed back so the refusal can echo what
/// the caller typed.
///
/// One helper rather than five copies of the same three-line match: five copies
/// is five places a later field can be given the wrong one, and the shape of
/// this phase's whole failure history is per-site rules that drift.
///
/// **`refuse` receives a [`crate::text::Untrusted`], not a `String`** (`21-24`).
/// This is the ONE place a refused argv value is handed to a refusal on this
/// path, so wrapping it HERE means every position that already routes through
/// this helper — and every position added to it later — carries the value in a
/// type that cannot be interpolated. Wrapping at each of the five closures
/// instead would be a list of five sites that would be six next round, which is
/// the shape this phase has paid for six times.
fn argv_visible(
    value: Option<String>,
    refuse: impl FnOnce(crate::text::Untrusted) -> DriveError,
) -> Result<Option<payload::NonBlank>, DriveError> {
    match value {
        None => Ok(None),
        Some(raw) => match payload::NonBlank::new(&raw) {
            Some(payload) => Ok(Some(payload)),
            None => Err(refuse(crate::text::Untrusted::from_untrusted_source(raw))),
        },
    }
}

impl DriveArgs {
    /// **THE PARSE BOUNDARY.** The single production conversion from raw argv
    /// strings into the type every downstream consumer reads.
    ///
    /// **The exhaustive destructure below IS the anti-recurrence mechanism, and
    /// it must keep no `..` rest-pattern.** Four gap-closure cycles each
    /// hand-enumerated which values to protect and each enumeration was exactly
    /// one item short — three `CommandSource` arms one at a time, then three of
    /// `DriveArgs`'s six string fields, with all three of the next round's
    /// Criticals landing in the other three. A thirteenth field on
    /// [`RawDriveArgs`] now refuses to compile until this function classifies
    /// it, and a new field on [`DriveArgs`] refuses to compile until this
    /// function constructs it. That is the enumeration moved from a human to the
    /// compiler, which is the whole of round 5. A builder, a `Default`, a
    /// `..raw` rest-pattern or a struct-update syntax would each restore the
    /// silent default this destructure exists to forbid.
    ///
    /// **Each position keeps its OWN refusal**, because the flag a user
    /// mistyped is what they need told:
    ///
    /// * `--command`, `--target-phase`, `--goal` → [`DriveError::NoCommandSource`],
    ///   whose `Display` already says a value made only of invisible characters
    ///   counts as absent. The goal clause is D-15-2: a blank `--goal` supplied
    ///   *beside* a visible source is refused rather than dropped or recorded.
    /// * `--run-id` → [`DriveError::RunIdInvalid`], the refusal the seam already
    ///   raised for a run id that is not a plain name.
    /// * `--approved-plan` → the malformed-token refusal, derived by running the
    ///   real [`journal::parse_approval_token`] on the blank value rather than
    ///   by asserting which variant it would produce. A pin below fixes that
    ///   "guaranteed `Err`" as a checked fact rather than an assumption.
    /// * the alias → [`DriveError::AliasNotVisible`]. **This line used to say
    ///   "the unknown-alias refusal `drive` already reports, because no registry
    ///   can honestly name a value nobody can see", and that was false**: pass 6
    ///   measured that an invisible alias CAN be registered by an older build,
    ///   so the borrowed message narrated the registry's contents rather than
    ///   the value's shape (WR-06, D-17-4). The correction rides the commit that
    ///   falsifies it.
    ///
    /// **Pure.** It opens no file and starts no process, so every refusal above
    /// costs nothing and creates nothing — and, because it runs before a
    /// `DriveArgs` exists at all, it cannot consult `dry_run`. Preview/real
    /// symmetry for these refusals is therefore a construction-time fact rather
    /// than two branches that have to agree (this plan's fourth prohibition).
    pub fn from_argv(raw: RawDriveArgs) -> Result<DriveArgs, DriveError> {
        // NO `..` REST-PATTERN. See the doc above: this listing is the
        // mechanism, and a rest-pattern would let a new argv field arrive
        // unclassified and unprotected, which is the five-time losing bet.
        let RawDriveArgs {
            alias,
            command,
            target_phase,
            max_steps,
            wall_clock_cap_secs,
            max_escalations,
            approved_plan,
            run_id,
            dry_run,
            goal,
            #[cfg(debug_assertions)]
            claude_program,
            #[cfg(debug_assertions)]
            claude_args,
        } = raw;

        // **This used to borrow `OptInError::UnknownAlias`, and the borrowed
        // sentence was false** (WR-06, D-17-4). Its message reads "no project
        // is registered under the alias `…`" — a claim about durable state that
        // this pure, file-free boundary has not checked and cannot know. Pass 6
        // measured that an invisible alias CAN be registered by an older build,
        // which makes the claim not merely unproven but wrong. The correction
        // rides the commit that falsifies it: same site, same purity, same
        // ordering, a refusal that describes the value instead of the registry.
        // Wrapped at construction, as every other refusal on this boundary now
        // is: `alias` is the raw argv string and the refusal echoes it.
        let alias = payload::NonBlank::new(&alias).ok_or_else(|| DriveError::AliasNotVisible {
            alias: crate::text::Untrusted::from_untrusted_source(alias.clone()),
        })?;

        let command = argv_visible(command, |_| DriveError::NoCommandSource)?;
        let target_phase = argv_visible(target_phase, |_| DriveError::NoCommandSource)?;
        let goal = argv_visible(goal, |_| DriveError::NoCommandSource)?;
        let run_id = argv_visible(run_id, |run_id| DriveError::RunIdInvalid { run_id })?;
        let approved_plan = argv_visible(approved_plan, |raw| {
            // The refusal is DERIVED by running the real parser on the blank
            // value, never asserted. `parse_approval_token` refuses every blank
            // shape — `a_blank_approval_token_is_refused_by_the_real_parser`
            // pins that, so "guaranteed" is checked rather than believed — and
            // the `Ok` arm below is a typed refusal too, never a fabricated
            // value, because an unreachable arm answered with a manufactured
            // value is this phase's most-repeated defect.
            //
            // `as_raw_for_logic_only` and not `shown()`: this value is being
            // PARSED, not read by a human, and parsing an escaped copy would
            // derive the refusal from a string the caller never typed.
            match journal::parse_approval_token(raw.as_raw_for_logic_only()) {
                Err(err) => DriveError::PlanApprovalMalformed(err),
                Ok(_) => DriveError::PlanApprovalMalformed(
                    journal::ApprovalTokenError::SeparatorAbsent,
                ),
            }
        })?;

        Ok(DriveArgs {
            alias,
            command,
            target_phase,
            max_steps,
            wall_clock_cap_secs,
            max_escalations,
            approved_plan,
            run_id,
            dry_run,
            goal,
            #[cfg(debug_assertions)]
            claude_program,
            #[cfg(debug_assertions)]
            claude_args,
        })
    }
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

/// The payload half of [`CommandSource`], made blank-proof **by type**.
///
/// **The private field IS the mechanism, and the NESTING is what makes it one.**
/// Rust field privacy is scoped to the defining module *and its descendants*,
/// never its ancestors. Declaring [`NonBlank`](payload::NonBlank) inside this
/// nested module — rather than beside [`CommandSource`] at `driver`'s own top
/// level — is precisely what stops the rest of `driver/mod.rs`, its
/// `#[cfg(test)] mod tests` included, from fabricating a blank payload with a
/// tuple-struct literal. Flattening this module back out would leave the type in
/// place and the guarantee gone.
///
/// **Why a type rather than a fourth per-arm check.** Three consecutive
/// gap-closure cycles each fixed one [`CommandSource`] arm's blankness and each
/// left the next arm bare: `Goal` in 21-07, `Command` in 21-11, and `Routed`
/// still unguarded when round-4 verification reproduced `--target-phase '   '`
/// previewing cleanly at exit 0 and then, on a real run, creating `run.lock`, a
/// run directory, `journal.jsonl` and a committed `run.json` carrying
/// `"target_phase": "   "` — a value that reads as *field absent* on the tolerant
/// read path (D-30), so the record stops being evidence of what ran. Every fix
/// was correct and every *scope* was the defect (`21-PREMISES.md`, Premise 1).
/// With a `String` payload each arm — and each FUTURE arm — has to independently
/// remember the invariant, and the compiler enforces nothing. With this type the
/// invariant is written **once**, in [`NonBlank::new`](payload::NonBlank::new),
/// and a blank payload is unrepresentable because there is no other route in.
///
/// This is the type-level completion of [`CommandSource`]'s own argument, which
/// already says that resolving once and matching exhaustively is what makes a
/// fourth source a compile error at every consumer. That argument covered the
/// *variant* half; this covers the *payload* half (round-4 CR-01).
/// **The module and the type are `pub`; the FIELD is not, and that distinction
/// is the whole mechanism** (D-15-5). The visibility widened in 21-15 because
/// [`DriveArgs`]'s own `pub` fields now carry this type — a `pub(crate)` type in
/// a `pub` field trips `private_interfaces` — and because integration-test
/// fixtures build payloads. What did NOT widen is the tuple field, which is
/// private to this module and therefore unreachable from every ancestor,
/// `driver/mod.rs`'s own `#[cfg(test)] mod tests` included. There is still
/// exactly one route to a `NonBlank` and it refuses the invisible. Adding a
/// `pub` field, a `From<String>`, or an `into_inner` would give up the
/// guarantee while leaving the type in place.
pub mod payload {
    /// An argv payload carrying at least one character a reader could see.
    ///
    /// Built only by [`NonBlank::new`]; the field is private to this module, so
    /// no ancestor of `driver::payload` — production code or test code — can
    /// write past the constructor.
    #[derive(Debug, Clone, PartialEq, Eq)]
    pub struct NonBlank(String);

    impl NonBlank {
        /// `Some` when `raw` carries a visible instruction, `None` otherwise.
        ///
        /// **"Blank" means NO VISIBLE INSTRUCTION, deliberately wider than
        /// `str::trim`** (D-13-2). Trimming answers only for whitespace, and a
        /// payload of `U+200B` (zero-width space) or `U+FEFF` survives it while
        /// rendering as visually empty and recording as field-absent — the same
        /// corruption a run of spaces causes, spelled differently. Refused: the
        /// empty string, and any value whose every character is whitespace, a
        /// control character, or a zero-width/format character.
        ///
        /// **The judgment is DELEGATED to [`crate::text::carries_visible_content`]
        /// and is not spelled here.** It used to be an inline character-class
        /// closure, which made this the second production spelling of blankness
        /// beside `journal::is_plain_path_component`'s `str::trim` — and pass 5
        /// reproduced the two disagreeing about `U+200B` end to end. The
        /// invariant this type carries is *enforcement*, which is the private
        /// field; the *definition* of blank belongs in one module that every
        /// judge of it reads.
        ///
        /// An `Option` rather than a typed error, so the calling seam owns the
        /// refusal it reports: `command_source` maps `None` onto
        /// [`crate::error::DriveError::NoCommandSource`], which is already the
        /// name for a run with nothing to do.
        pub fn new(raw: &str) -> Option<Self> {
            if crate::text::carries_visible_content(raw) {
                Some(Self(raw.to_string()))
            } else {
                None
            }
        }

        /// The payload as written, for the renderers and the record writers.
        pub fn as_str(&self) -> &str {
            &self.0
        }
    }
}

/// Which of the three legal command sources this invocation names.
///
/// **A resolved value rather than three `Option`s re-matched at each consumer,
/// and Phase 21 is what forced the promotion.** The old shape had every consumer
/// re-derive the source from `(command, target_phase, goal)`, which meant adding
/// a third source was something one could do *beside* the other two rather than
/// *through* them: `--goal` became legal at the refusal and the preview was
/// never told, so a goal-only `--dry-run` fell through to the command-mode
/// renderer and printed an empty command as "the complete and honest sequence"
/// (CR-01). Resolving once and matching exhaustively is what makes a fourth
/// source a **compile error** at every consumer instead of a silent
/// fall-through.
///
/// **Owned rather than borrowed**, because the value is moved into the
/// `spawn_blocking` closure on the dry-run path and that requires `'static`.
/// `Clone` because the inline join-failure fallback needs a second copy, and
/// building it by re-resolving would be two places that can disagree about which
/// source won.
///
/// **Every payload is a [`payload::NonBlank`], not a `String`, and that is
/// round-4's structural fix.** A `String` can hold `"   "`, so each arm had to
/// remember the blankness rule independently and three consecutive cycles proved
/// that one arm always forgets. The payload type carries the rule instead: see
/// [`payload`] for what the private field buys and why the module is nested.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum CommandSource {
    /// One supplied `--command`: the whole of what the run would issue.
    Command(payload::NonBlank),
    /// A `--target-phase` the decision router drives toward, choosing each
    /// iteration's command from what the previous one left on disk.
    Routed(payload::NonBlank),
    /// A `--goal` stated in plain language, which decomposes into a plan above
    /// the run and resolves *into* the routed model rather than being a fourth
    /// execution model of its own.
    Goal(payload::NonBlank),
}

/// Resolve the one command source a caller named, or refuse.
///
/// **Pure**, and the refusals are unchanged from the `Option<DriveError>` shape
/// this replaced; only the success case grew a value, because the callers below
/// needed the source itself rather than merely its legality.
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
///
/// **Precedence is exactly what it was**: a supplied `command` **that carries a
/// visible instruction** wins, otherwise such a `target_phase`, otherwise such a
/// `goal`, otherwise [`DriveError::NoCommandSource`]. Command-plus-phase is still
/// [`DriveError::AmbiguousCommandSource`]; a goal beside either is recorded prose
/// and must not turn a legal invocation into an ambiguous one.
///
/// **Blankness is not this function's rule to remember, and as of 21-15 it is
/// not this function's rule at all.** The inputs are already
/// [`payload::NonBlank`]s — a blank one is unrepresentable, refused at
/// [`DriveArgs::from_argv`] before a `DriveArgs` exists — so the `NonBlank::new`
/// mapping the round-4 arms carried is *gone* rather than duplicated. A
/// vestigial check on a guaranteed-visible value would be a second predicate
/// that can drift, which is the rule `crate::text` states.
///
/// What remains here is exactly three things, and nothing else: **precedence**
/// (command, then phase, then goal), **ambiguity** (command and phase together),
/// and **absence** (none of the three).
pub(crate) fn command_source(
    command: Option<&payload::NonBlank>,
    target_phase: Option<&payload::NonBlank>,
    goal: Option<&payload::NonBlank>,
) -> Result<CommandSource, DriveError> {
    match (command, target_phase) {
        // First, and it must STAY first. Two sources were named, and blankness
        // must not demote an ambiguous invocation into a legal one: whichever of
        // the two won would be a mode the caller did not choose, with the other
        // mode's bounds left unenforced.
        (Some(_), Some(_)) => Err(DriveError::AmbiguousCommandSource),
        // A supplied command wins. It cannot be blank — the type says so, and
        // `DriveArgs::from_argv` said so before this function was entered — so
        // the fall-through-to-`goal` hazard the round-4 arm guarded against
        // cannot arise here either: there is no blank `command` left to demote
        // an invocation whose precedence the caller already chose.
        //
        // **Spelled as a `match` with the variant constructed inside it rather
        // than as `.map(CommandSource::Command)`**, and this comment must
        // outlive every rewrite of these arms: `tests/spawn_seam_guard.rs`'s
        // guard eight scans for the PARENTHESISED variant spellings, and a
        // point-free construction's source text is `CommandSource::Command)` —
        // which contains no `CommandSource::Command(` and so silently stops
        // matching. The guard would keep passing while auditing less than it
        // claims.
        (Some(command), None) => Ok(CommandSource::Command(command.clone())),
        // The routed arm, in the same shape and under the same needle rule.
        (None, Some(target_phase)) => Ok(CommandSource::Routed(target_phase.clone())),
        // Absence, and the ONLY remaining source of `NoCommandSource` in this
        // function: no command, no phase, no goal is a run with nothing to do.
        //
        // **Spelled as an explicit `match` rather than
        // `goal.cloned().map(CommandSource::Goal).ok_or(...)`** for exactly the
        // needle reason the `Command` arm above records — the point-free form's
        // source text is `CommandSource::Goal)`, which guard eight's
        // `CommandSource::Goal(` needle does not contain.
        (None, None) => match goal {
            Some(goal) => Ok(CommandSource::Goal(goal.clone())),
            None => Err(DriveError::NoCommandSource),
        },
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
/// **The three modes make three different honesty claims, so they render
/// through three entry points** rather than through one that would have to
/// hedge: a supplied command is the complete sequence, a routed run shows the
/// router's first selection with the fact that it continues stated plainly, and
/// a stated goal shows no command at all because the plan does not exist until a
/// model is consulted.
///
/// The source is **resolved once** by [`command_source`], above, rather than
/// re-derived here from the `Option`s on `args`. That is what makes the match
/// exhaustive over a three-variant type with no fall-through arm — and it is the
/// property, not a comment, that stops a fourth source being added beside these
/// three without the preview learning about it.
fn preview_text(project: &DrivableProject, source: &CommandSource) -> String {
    match source {
        CommandSource::Command(command) => {
            dry_run::render(&dry_run::build_report(project, command.as_str()))
        }
        CommandSource::Routed(target_phase) => {
            dry_run::render_scoped(&dry_run::build_routed_report(project, target_phase.as_str()))
        }
        CommandSource::Goal(goal) => {
            dry_run::render_scoped(&dry_run::build_goal_report(project, goal.as_str()))
        }
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
///    source or two, or a `--command` made of nothing (`command_source`); a
///    `--target-phase` that is not a single plain path component
///    ([`DriveError::TargetPhaseInvalid`]); caps that cannot be honoured
///    ([`bounds::resolve`] and, on the adjacent line and against the step cap the
///    first of them returned, [`escalate::resolve`]); and an `--approved-plan`
///    that is not a token at all ([`journal::parse_approval_token`],
///    [`DriveError::PlanApprovalMalformed`]). All **four** sit **above** the
///    dry-run branch, because each is answered identically whether or not the run
///    is real and a preview that answered them differently would be previewing
///    something the user cannot run (WR-09). Every one of them is **pure**: it
///    opens no file and starts no process, so each is refused for free — which is
///    the whole argument for the approval parse being here rather than below the
///    decomposition seam, where a typo cost a live model consultation
///    (review-CR-01).
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
        .get(args.alias.as_str())
        // Wrapped at construction (`21-24`): this is the argv alias, a string
        // this build did not author, and the refusal that echoes it is the one
        // pass 9 measured printing a live ANSI introducer to a terminal.
        .ok_or_else(|| OptInError::UnknownAlias {
            alias: crate::text::Untrusted::from_untrusted_source(args.alias.as_str().to_string()),
        })?;

    let project = DrivableProject::from_registry(args.alias.as_str(), entry)?;

    // **The four refusals about WHAT WAS ASKED FOR, before the dry-run branch**
    // — unlike the ones about *running*, which sit below it. The run-id and
    // platform refusals are about a run: a preview creates nothing to identify
    // and starts no process to stop, so neither is about anything a preview
    // does. These four are about the invocation itself, they are answered
    // identically whether or not the run is real, and a preview that answers
    // them differently is answering a different question from the one the user
    // asked (WR-09).
    //
    // In order: the command source (`command_source`), the `--target-phase`
    // plain-component check, the two caps (`bounds::resolve` then
    // `escalate::resolve`), and the `--approved-plan` token parse
    // (`journal::parse_approval_token`). The fourth joined the group in 21-11;
    // it used to sit inside `approve_plan`, below both the dry-run branch and the
    // decomposition seam, which is review-CR-01.
    //
    // All four are pure and none creates anything on disk — no file is opened
    // and no process is started — which is why each can be answered before the
    // preview branch at no cost, and why a malformed token now costs zero seam
    // spawns rather than one.
    //
    // Still after the capability gate, so `from_registry` keeps its single
    // production call site and an unregistered alias is refused first.

    // A preview of nothing has nothing to show, and a preview of two
    // conflicting sources cannot say which it previewed.
    //
    // **Resolved into a value here rather than merely checked**, so every
    // consumer below reads the source one place decided instead of re-deriving
    // it from three `Option`s. Re-deriving is how CR-01 happened.
    let source = command_source(
        args.command.as_ref(),
        args.target_phase.as_ref(),
        args.goal.as_ref(),
    )?;

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
    if let Some(target_phase) = args.target_phase.as_ref() {
        if !journal::is_plain_path_component(target_phase.as_str()) {
            return Err(DriveError::TargetPhaseInvalid {
                target_phase: crate::text::Untrusted::from_untrusted_source(
                    target_phase.as_str().to_string(),
                ),
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

    // **The approval token, PARSED FIRST — before the dry-run branch, before the
    // decomposition seam, and before anything is created (review-CR-01).**
    //
    // A value that is not a token cannot approve anything, so nothing may be
    // computed on the strength of it and no path may treat a half-supplied value
    // as a partial approval. This paragraph used to live inside `approve_plan`,
    // where it was true of that function's body and false of the ordering that
    // reached it: `drive` called `GoalDecomposition::decompose` — a real process
    // spawn into the driven repository and a real model consultation out of
    // `budget` — *before* `approve_plan` ran, so a typo in a token cost a live
    // consultation before a pure string check refused it. A real run with a
    // garbage token was reproduced returning `PlanApprovalMalformed` **after
    // exactly one seam spawn was recorded on disk**.
    //
    // **Last of the invocation-shape refusals**, so the four above keep their
    // documented order and an invocation malformed in two ways still reports the
    // refusal it already reported.
    //
    // **Above the dry-run branch**, because the answer does not depend on whether
    // the run is real: `--dry-run` returns at the branch below, so the preview
    // never reached this parse at all and exited `Ok(())` on an invocation the
    // real run refuses. A preview that refuses less than the run it previews is
    // previewing something the user cannot run (WR-09).
    //
    // **The widening this ordering bought, named rather than left implicit
    // (IN-03).** Because the parse now sits here, a malformed `--approved-plan`
    // is refused on EVERY invocation — including a `--command` or
    // `--target-phase` run, which has no plan for a token to approve and where
    // the flag was previously never looked at. A well-formed but irrelevant
    // token on such a run is accepted and then simply ignored. That is the
    // intended trade: refusing a value that is not a token at all costs nothing
    // and cannot surprise anyone, while a flag silently unread on three of four
    // invocation shapes is how a user learns to trust a check that did not run.
    // `src/cli.rs`'s help for the flag says the same thing.
    //
    // **Malformation only, never absence.** `parse_approval_token` runs only when
    // the flag is present. An absent approval stays `None` and is refused far
    // below by `approve_plan` with [`DriveError::PlanApprovalRequired`], which
    // carries the token the reviewer copies back — a refusal that needs a plan to
    // exist before it can be written. A goal-only preview has no plan, so there
    // is nothing here to require an approval of.
    //
    // Unused on non-Unix, where `decomposed` and `approved_plan` are both
    // hard-coded `None`; annotated rather than `#[cfg(unix)]`-gated, following
    // `budget` above, because the refusal must be answered identically on every
    // platform. A platform that silently accepted a malformed token until
    // `dispatch` refused the run for an unrelated reason is precisely the
    // asymmetry this move removes.
    #[cfg_attr(not(unix), allow(unused_variables))]
    let recorded_approval: Option<(String, String)> = match args.approved_plan.as_ref() {
        Some(raw) => Some(
            journal::parse_approval_token(raw.as_str())
                .map_err(DriveError::PlanApprovalMalformed)?,
        ),
        None => None,
    };

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
        //
        // The **resolved source** is cloned rather than the three `Option`s that
        // produced it, so the closure and the fallback below preview the same
        // mode by construction instead of by two agreeing re-derivations.
        //
        // **The mode is passed through rather than flattened to a string**, and
        // the pinned `dry_run::SECTION_COMMANDS` prose has been corrected to
        // match (research Pitfall 6). It used to say a routed sequence was a
        // single honest command; the preview now shows the router's own first
        // selection and states plainly that the run continues past it — and, as
        // of Phase 21, says of a stated goal that no command can be shown at all.
        //
        // **The closure keeps its braced body** rather than collapsing to a
        // one-line expression, and that is a guard property rather than a style
        // preference: `tests/async_blocking_guard.rs` suppresses reporting from
        // the line a `spawn_blocking` appears on until the brace depth it opened
        // closes. A braced closure closes that scope before the `Err` arm, so the
        // inline fallback below is still *reported* and still suppressed by the
        // deliberate `("src/driver/mod.rs", "preview_text(")` allowlist entry. A
        // one-line closure leaves the match block open instead, and the fallback
        // becomes invisible to the scanner — the entry goes stale and the
        // exemption stops being a decision anybody made.
        let cloned_source = source.clone();
        let cloned = project.clone();
        let rendered = match tokio::task::spawn_blocking(move || {
            preview_text(&cloned, &cloned_source)
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
                preview_text(&project, &source)
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
    let Some(run_id) = args.run_id.as_ref() else {
        return Err(DriveError::RunIdRequired);
    };
    if !journal::is_plain_path_component(run_id.as_str()) {
        return Err(DriveError::RunIdInvalid {
            run_id: crate::text::Untrusted::from_untrusted_source(run_id.as_str().to_string()),
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
        // The already-parsed halves are threaded down rather than re-read from
        // argv: `approve_plan` no longer touches `args` at all, so there is no
        // second parse that could disagree with the one above about whether the
        // value was a token.
        Some(plan) => Some(approve_plan(&project, recorded_approval.as_ref(), plan)?),
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
    //
    // **The rewrite goes through the payload constructor and refuses rather
    // than defaulting.** It is `Some` in practice for every plan that reaches
    // here — `goal::legality` already applied a *stricter* check than the argv
    // seam, `is_plain_path_component` plus a roadmap declaration — so the
    // `ok_or_else` is the arm nothing should reach. It is nonetheless a TYPED
    // refusal rather than a panic or an `unwrap_or_default`: a detached driver
    // that panicked here would leave no terminal record, and a default would
    // manufacture the blank value this whole plan exists to make
    // unrepresentable.
    if let Some(plan) = approved_plan.as_ref() {
        args.target_phase = Some(payload::NonBlank::new(&plan.target_phase).ok_or_else(|| {
            // The value is a MODEL-selected target phase read back out of an
            // approval token, so it is third-party content by the strictest
            // reading of the term — exactly what the carrier is for.
            DriveError::TargetPhaseInvalid {
                target_phase: crate::text::Untrusted::from_untrusted_source(
                    plan.target_phase.clone(),
                ),
            }
        })?);
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
/// The four outcomes are the four the caller needs:
///
/// * **no `--approved-plan` at all** → [`DriveError::PlanApprovalRequired`],
///   carrying the token the user would approve, so the refusal is actionable in
///   one step rather than being a bug report. This one is raised *here*, because
///   it needs a decomposed plan to name and a token to print, and neither exists
///   before the seam has been consulted;
/// * **a value that is not a token at all** → [`DriveError::PlanApprovalMalformed`],
///   raised **by the caller**, in [`drive`]'s invocation-shape group above the
///   dry-run branch and above the decomposition seam. This function is entered
///   only with halves that have already parsed, and `recorded` carries them.
///   That position is review-CR-01: the parse used to live in this body, which
///   made the claim "before any of the work below" true of this function and
///   false of the ordering that reached it — the seam had already spawned a
///   process and spent a model consultation by the time a typo was refused;
/// * **a token that covers a different plan** → the plan half changed;
/// * **a token whose plan half matches and whose file half does not** → the
///   disclosed bytes changed under the approval.
///
/// The last two share a variant carrying [`journal::ApprovalRefusal`], which is
/// what keeps *which half* readable without a second error type.
///
/// **The third of those used to be unreachable, and that was WR-01.** This
/// function built a throwaway [`journal::ApprovedPlan`] whose `plan_digest` was
/// the digest of the plan it had *just decomposed*, then asked
/// [`journal::recheck_approval`] to compare that against the same value — a
/// comparison of a value against itself, reported as a check. Every real
/// mismatch therefore fell through to the files-changed arm, whose message
/// opens *"the plan is unchanged but the disclosed files … are not"*, and the
/// **common** case is the one it lied about: the review flow re-decomposes the
/// goal through a non-deterministic model, so a differing plan is expected, and
/// the user was told to go looking for a `git pull` that never happened. The
/// recorded plan digest now arrives from the caller's token, where the approval
/// put it.
#[cfg(unix)]
fn approve_plan(
    project: &DrivableProject,
    recorded: Option<&(String, String)>,
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

    let Some((recorded_plan_digest, recorded_approval_digest)) = recorded else {
        // Absent, not malformed. A malformed value never reaches this function —
        // it is refused in `drive` above — so this arm is unambiguously "nobody
        // approved this", which is the statement the token below answers.
        // The token the reviewer copies back: one value carrying both halves,
        // rendered by the one function that joins them, so what the refusal
        // prints and what the flag accepts cannot drift apart.
        return Err(DriveError::PlanApprovalRequired {
            token: journal::render_approval_token(&plan_digest, &digest),
            steps,
        });
    };

    // **The substitution that fixes WR-01.** The recorded plan digest is the one
    // the caller's token carried — the plan the approval covered — and it is
    // compared against the plan just decomposed. It is no longer the same value
    // on both sides, so `PlanChanged` is a distinction the code can actually
    // draw. One predicate, used here and again at the spawn gate, rather than an
    // equality written twice.
    journal::recheck_approval(
        Some((recorded_plan_digest, recorded_approval_digest)),
        &plan_digest,
        &prompt_inputs,
    )
    .map_err(DriveError::PlanApprovalStale)?;

    // **The model seam's route to the same corruption the argv seam refuses.**
    //
    // `legality` refuses an empty plan, so the last step is believed always
    // present — and this line used to spell that belief as an unwrap-or-default,
    // which is not a refusal but a *fabrication*: reached, it would write `""`
    // into `ApprovedPlan.target_phase`, and the empty string already means
    // "field absent" on the tolerant read path (D-30). That is the identical
    // D-30 corruption `command_source` refuses on argv, arriving through the
    // model seam instead — the same defect class, merely with a different
    // provenance, which is why 21-PREMISES.md Premise 6 adjudicated it IN
    // rather than letting the scoping bet that lost three times run a fourth
    // time.
    //
    // It refuses with the SAME typed error `legality` raises for a stepless
    // plan, so the refusal a user reads matches the invariant that was broken.
    // Refused rather than `unwrap`ped, because a detached driver that panicked
    // here would leave no terminal record at all.
    //
    // A step that IS present cannot carry a blank phase either: `legality`
    // passes every step's `target_phase` through
    // `journal::is_plain_path_component`, which since 21-13 refuses blank and
    // control-carrying values.
    let target_phase = run::plan_target_phase(plan)
        .ok_or_else(|| DriveError::from(goal::GoalRefusal::empty_plan()))?
        .to_string();

    Ok(journal::ApprovedPlan {
        steps,
        target_phase,
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

    /// A payload from a literal a reader can see.
    ///
    /// Every fixture below builds its argv values through this rather than
    /// through a struct literal, because a struct literal is exactly what the
    /// private field forbids. It `expect`s: a visible literal yields `Some` by
    /// construction, and a fixture whose own literal is invisible is a fixture
    /// bug rather than a case under test.
    fn visible(raw: &str) -> payload::NonBlank {
        payload::NonBlank::new(raw).expect("a visible test literal is a payload")
    }

    fn args(alias: &str) -> DriveArgs {
        DriveArgs {
            alias: visible(alias),
            command: Some(visible("/gsd-progress")),
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
        // An `Option<NonBlank>`, never a `Vec<_>`. A routed run's sequence is
        // DERIVED per iteration from what the previous one left on disk, so a
        // field that accepted a supplied list would be a third execution model
        // that neither the router nor the bounds know about.
        assert_eq!(
            args.command.as_ref().map(payload::NonBlank::as_str),
            Some("/gsd-progress")
        );
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

    /// **What is left of `command_source` to test: absence only.**
    ///
    /// Every blank-payload row this test used to carry is now *inexpressible* —
    /// the inputs are `Option<&payload::NonBlank>`, and there is no route to a
    /// blank `NonBlank`. Those rows did not vanish; they moved UP, to the parse
    /// boundary where the values are still raw strings, and they are asserted in
    /// [`every_argv_position_refuses_every_degenerate_payload_at_the_parse_boundary`]
    /// against `DriveArgs::from_argv`. Asserting them here too would be a second
    /// place answering a question the boundary already answers, on a type that
    /// cannot represent the failing input.
    #[test]
    fn a_run_with_no_command_source_at_all_is_refused_before_anything_is_created() {
        assert!(
            matches!(
                command_source(None, None, None),
                Err(DriveError::NoCommandSource)
            ),
            "clap used to make this unrepresentable by requiring --command. The \
             moment --target-phase became an alternative, 'exactly one of these' \
             stopped being expressible in the parser, and a run with nothing to do \
             would otherwise reach the lock and the journal before anybody noticed"
        );

        // The non-vacuity control: one visible character in the first position
        // resolves, so the refusal above is about ABSENCE rather than about a
        // resolver that refuses everything.
        assert_eq!(
            command_source(Some(&visible("x")), None, None).ok(),
            Some(CommandSource::Command(visible("x"))),
            "one visible character is a command; a resolver that refused this too \
             would be refusing on length rather than on absence"
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
        assert_eq!(
            command_source(None, None, Some(&visible("get phase 22 verified"))).ok(),
            Some(CommandSource::Goal(visible("get phase 22 verified"))),
            "a stated goal alone must reach the decomposition rather than be \
             refused as a run with nothing to do — and it must resolve to the \
             GOAL variant, because a goal that resolved to anything else is CR-01 \
             again with a different spelling"
        );
    }

    #[test]
    fn a_run_naming_both_command_sources_is_refused_rather_than_resolved() {
        assert!(
            matches!(
                command_source(Some(&visible("/gsd-progress")), Some(&visible("20")), None),
                Err(DriveError::AmbiguousCommandSource)
            ),
            "a precedence rule would let one source win SILENTLY, and whichever it \
             was the run's terminal record would name a mode the caller did not \
             choose while the other mode's bounds went unenforced. An unattended \
             run whose stopping conditions are not the ones asked for is the \
             failure CTRL-06 exists to prevent"
        );

        assert_eq!(
            command_source(Some(&visible("/gsd-progress")), None, None).ok(),
            Some(CommandSource::Command(visible("/gsd-progress"))),
            "single-command mode must stay transparent"
        );
        assert_eq!(
            command_source(None, Some(&visible("20")), None).ok(),
            Some(CommandSource::Routed(visible("20"))),
            "routed mode must stay transparent"
        );

        // A goal supplied ALONGSIDE either source is recorded prose and nothing
        // more — both of those sources are already machine-checkable, so there
        // is nothing to decompose and no seam fires. A goal must not turn a
        // legal invocation into an ambiguous one, and it must not WIN over one
        // either: precedence is command, then phase, then goal.
        assert_eq!(
            command_source(Some(&visible("/gsd-progress")), None, Some(&visible("a goal"))).ok(),
            Some(CommandSource::Command(visible("/gsd-progress"))),
            "a goal beside --command is recorded text, not a second source"
        );
        assert_eq!(
            command_source(None, Some(&visible("20")), Some(&visible("a goal"))).ok(),
            Some(CommandSource::Routed(visible("20"))),
            "a goal beside --target-phase is recorded text, not a second source"
        );

        // **Blankness cannot demote an ambiguous invocation into a legal one,
        // and the reason has MOVED** (T-21-11-05). It used to be that the
        // ambiguity arm had to match before the emptiness guard, so
        // `--command '' --target-phase 20` reported ambiguity rather than
        // resolving to a routed run the caller did not choose. Both halves of
        // that pairing are now unrepresentable here: a blank `--command` never
        // reaches this function, because `DriveArgs::from_argv` refuses it with
        // `NoCommandSource` before a `DriveArgs` exists. The invocation is still
        // refused and still creates nothing — the variant changed, the legality
        // did not — and the pairing is asserted at the boundary, in
        // `a_blank_payload_beside_a_visible_one_is_still_refused_at_the_boundary`.
    }

    #[tokio::test]
    async fn a_target_phase_that_is_not_a_plain_path_component_is_refused_without_touching_disk() {
        let root = tempfile::TempDir::new().expect("temp dir");
        let config = opted_in(root.path());

        let mut args = args("demo");
        args.command = None;
        args.target_phase = Some(visible("../../../../escaped"));
        args.run_id = Some(visible("2026-08-19T12-00-00Z-aaaa"));

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

    /// **The `--target-phase` seam's own look-alike pin** (pass-7 missing item
    /// 3).
    ///
    /// The seam consumes `journal::is_plain_path_component` and nothing else —
    /// no roadmap lookup happens here — so this certifies the WIRING, not a new
    /// mechanism. It had no look-alike coverage of its own: the traversal pin
    /// above is the only thing that ever exercised it, and a build that dropped
    /// the check for look-alikes specifically would have kept that pin green.
    ///
    /// **THE VISIBLE HALF IS A PREDICATE ASSERTION, NOT A DRIVE OUTCOME. Do not
    /// "fix" it into the stronger-looking claim, which is false.** `drive` with
    /// `--target-phase demo` does NOT succeed: the value is used downstream as a
    /// key into `ProjectState::phase_disk_statuses`, so a visible member of
    /// `LOOK_ALIKE_PAIRS` ("demo", "run", "x", "abc") is a perfectly legal path
    /// component that is not a phase in any fixture roadmap. Asserting success
    /// would fail for a reason with nothing to do with this seam. So the visible
    /// half is asserted two ways, both true by construction: the predicate
    /// directly, and — where the pair is driven end to end — only that the error
    /// is NOT `TargetPhaseInvalid`. That second one is the DISCRIMINATING claim:
    /// the seam refused the look-alike for being a look-alike, and not for
    /// something both members share.
    #[tokio::test]
    async fn a_target_phase_that_renders_as_another_is_refused_at_the_seam() {
        for (visible_member, look_alike) in crate::test_support::LOOK_ALIKE_PAIRS {
            // The visible half, direct: the predicate accepts it, so the refusal
            // below is about the invisible bytes rather than about the pair.
            assert!(
                journal::is_plain_path_component(visible_member),
                "{visible_member:?} is the visible member and must pass the \
                 predicate this seam consumes"
            );

            let root = tempfile::TempDir::new().expect("temp dir");
            let config = opted_in(root.path());

            let mut hostile_args = args("demo");
            hostile_args.command = None;
            hostile_args.target_phase = Some(visible(look_alike));
            hostile_args.run_id = Some(visible("2026-08-19T12-00-00Z-aaaa"));

            let err = drive(hostile_args, &config)
                .await
                .expect_err("a look-alike target phase must be refused at the seam");

            assert!(
                matches!(err, DriveError::TargetPhaseInvalid { .. }),
                "{look_alike:?} renders exactly as {visible_member:?} and must \
                 be refused by the typed seam refusal, got: {err:?}"
            );
            assert!(
                !root.path().join(".planning/meta-manager").exists(),
                "and a refused run must have created NOTHING"
            );

            // The discriminating half: the visible twin, driven end to end,
            // fails for some OTHER reason — never TargetPhaseInvalid.
            let visible_root = tempfile::TempDir::new().expect("temp dir");
            let visible_config = opted_in(visible_root.path());
            let mut visible_args = args("demo");
            visible_args.command = None;
            visible_args.target_phase = Some(visible(visible_member));
            visible_args.run_id = Some(visible("2026-08-19T12-00-00Z-aaaa"));

            if let Err(err) = drive(visible_args, &visible_config).await {
                assert!(
                    !matches!(err, DriveError::TargetPhaseInvalid { .. }),
                    "{visible_member:?} is a legal path component, so whatever \
                     stops it downstream must not be THIS seam — otherwise the \
                     refusal above was about something both members share \
                     rather than about the look-alike. Got: {err:?}"
                );
            }
        }

        // And a token from OUTSIDE the pre-round-7 ranges, so this pin cannot
        // pass by re-confirming the three literal ranges pass 7 found short.
        let root = tempfile::TempDir::new().expect("temp dir");
        let config = opted_in(root.path());
        let mut outside_args = args("demo");
        outside_args.command = None;
        outside_args.target_phase = Some(visible("2\u{e0041}0"));
        outside_args.run_id = Some(visible("2026-08-19T12-00-00Z-aaaa"));

        let err = drive(outside_args, &config)
            .await
            .expect_err("a tag-character target phase must be refused at the seam");
        assert!(
            matches!(err, DriveError::TargetPhaseInvalid { .. }),
            "a U+E0041 tag character is outside the old literal ranges and was \
             ACCEPTED here at HEAD; got: {err:?}"
        );
    }

    /// **The `--run-id` blank pin MOVED to the parse boundary** (21-15).
    ///
    /// It used to drive `drive` with a blank `--run-id` and assert
    /// `RunIdInvalid` with nothing on disk, over a hand-copied
    /// `["   ", "\t", "\n  \n"]` — three of the six blank shapes defined in the
    /// very commit that defined six, which is how `--run-id '\u{200b}'` reached
    /// a completed run. That row is now one of seven positions in
    /// [`every_argv_position_refuses_every_degenerate_payload_at_the_parse_boundary`],
    /// swept over the whole shared `test_support::DEGENERATE` const, and the
    /// refusal happens before a `DriveArgs` exists rather than partway down
    /// `drive`.
    ///
    /// Its **legitimate-id control arm** was the part worth keeping and it did
    /// not move here: `journal::mod`'s
    /// `only_a_single_plain_component_is_accepted_as_a_run_id` pins
    /// `"2026-08-19T12-00-00Z-aaaa"`, `"20"`, `"2.1"`, `"demo"` and `"99"` as
    /// accepted, in both directions, at the predicate the seam consults.
    #[test]
    fn a_legitimate_run_id_still_passes_the_seams_predicate() {
        for legitimate in ["2026-08-19T12-00-00Z-aaaa", "RID", "20"] {
            assert!(
                journal::is_plain_path_component(legitimate),
                "a real run id must still pass the predicate — one that refused \
                 every id would satisfy every blank-id assertion in this tree \
                 forever; {legitimate:?} was refused"
            );
        }
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
            args.target_phase = Some(visible("20"));
            args.run_id = Some(visible("2026-08-19T12-00-00Z-aaaa"));
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
            args.target_phase = Some(visible("20"));
            args.run_id = Some(visible("2026-08-19T12-00-00Z-aaaa"));
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
        args.target_phase = Some(visible("20"));
        args.run_id = Some(visible("2026-08-19T12-00-00Z-aaaa"));
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
        args.target_phase = Some(visible("20"));
        args.run_id = Some(visible("2026-08-19T12-00-00Z-aaaa"));
        args.max_steps = Some(4);
        args.max_escalations = Some(3);
        args.dry_run = true;

        drive(args, &config)
            .await
            .expect("a cap one below the step cap binds, so the preview renders");
    }

    /// **review-CR-01, as an assertion rather than as a review finding.**
    ///
    /// `--dry-run` returned above the whole decompose/approve region, so
    /// `journal::parse_approval_token` was never reached on the preview path at
    /// all: a preview carrying a garbage `--approved-plan` exited `Ok(())` with a
    /// clean preview and no mention of the token, while the *same* invocation run
    /// for real was refused. Against that build this test FAILS on the first
    /// assertion — `drive` returns `Ok(())`.
    ///
    /// A preview that refuses LESS than the run it previews is previewing
    /// something the user cannot run (WR-09), and the person rehearsing a run on
    /// an unfamiliar repository is exactly the person a preview exists for.
    #[tokio::test]
    async fn a_malformed_approval_token_is_refused_in_a_preview_exactly_as_a_real_run_would_be() {
        let root = tempfile::TempDir::new().expect("temp dir");
        let config = opted_in(root.path());

        let mut args = args("demo");
        args.command = None;
        args.goal = Some(visible("get phase 22 verified"));
        args.dry_run = true;
        args.approved_plan = Some(visible("total-garbage-no-separator"));

        let err = drive(args, &config).await.expect_err(
            "a value that is not a token cannot approve anything, and a preview \
             must answer that identically to the run it previews",
        );

        assert!(
            matches!(
                err,
                DriveError::PlanApprovalMalformed(journal::ApprovalTokenError::SeparatorAbsent)
            ),
            "the refusal must name WHICH malformation, so the user knows to add \
             the separator rather than to go looking for a changed plan; got: {err:?}"
        );
        assert!(
            !root.path().join(".planning/meta-manager").exists(),
            "the refusal is a pure string check above everything that creates \
             anything, so nothing may be left behind"
        );
    }

    /// The control arm for the test above, and the arm a careless fix breaks.
    ///
    /// **Absence is not malformation.** A goal-only preview has no plan yet, so
    /// there is nothing for an approval to cover;
    /// [`DriveError::PlanApprovalRequired`] is raised inside `approve_plan`,
    /// after a plan exists, and it carries the token the user copies back. A
    /// refusal moved up here would refuse every goal-only preview — the one
    /// output that explains what the user is being asked to approve.
    #[tokio::test]
    async fn an_absent_approval_is_not_refused_above_the_dry_run_branch() {
        let root = tempfile::TempDir::new().expect("temp dir");
        let config = opted_in(root.path());

        let mut args = args("demo");
        args.command = None;
        args.goal = Some(visible("get phase 22 verified"));
        args.dry_run = true;
        args.approved_plan = None;

        drive(args, &config).await.expect(
            "absence is a question about a plan that does not exist yet; \
             malformation is a question about a string on argv, and only the \
             second is answerable here",
        );
    }

    /// The new refusal joins the invocation-shape group **last**, so an
    /// invocation malformed in two ways still reports the refusal it already
    /// reported. Re-ordering an existing refusal would change the message a user
    /// has already learned to read.
    #[tokio::test]
    async fn the_target_phase_refusal_keeps_its_position_above_the_approval_parse() {
        let root = tempfile::TempDir::new().expect("temp dir");
        let config = opted_in(root.path());

        let mut args = args("demo");
        args.command = None;
        args.target_phase = Some(visible("../../../escaped"));
        args.approved_plan = Some(visible("total-garbage-no-separator"));
        args.run_id = Some(visible("2026-08-19T12-00-00Z-aaaa"));

        let err = drive(args, &config)
            .await
            .expect_err("an invocation malformed in two ways is still refused");

        assert!(
            matches!(err, DriveError::TargetPhaseInvalid { .. }),
            "the target-phase refusal is older and sits above the approval parse; \
             got: {err:?}"
        );
    }

    /// A `DrivableProject` for the preview tests, built through the **production**
    /// constructor.
    ///
    /// Never `for_testing_bypassing_opt_in`: `tests/spawn_seam_guard.rs` requires
    /// that identifier to appear on exactly one executable line under `src/` —
    /// its own definition — and the guard's line filter drops comments but not
    /// `#[cfg(test)]` modules, so an in-source test that used the hatch would
    /// break the audit rather than the audit catching a real bypass. Going
    /// through `from_registry` with a genuine opt-in is the same route
    /// `dry_run.rs`'s own in-module test takes.
    fn previewable(root: &std::path::Path) -> DrivableProject {
        let config = opted_in(root);
        let entry = config
            .projects
            .get("demo")
            .expect("the fixture registers `demo`");
        DrivableProject::from_registry("demo", entry).expect("an opted-in real directory")
    }

    // The payloads that carry no instruction at all were **lifted out of this
    // module in 21-15**. They live at `crate::test_support::DEGENERATE`, read by
    // every blank-shape pin in the tree — because pass 5 found the `--run-id`
    // pin carrying a hand-copied three-of-six subset added in the very commit
    // that defined six. A const each seam copies from is a const each seam can
    // copy from incompletely. Every consumer below names it by its full path so
    // there is no local alias that could quietly go stale.

    /// The expected `CommandSource` variant names, in ONE place (round-3 IN-02).
    ///
    /// Both consumers read this const rather than each spelling the list out:
    /// the preview enumeration's per-variant sweep and the matrix's coverage
    /// assertion. Duplicated literals were IN-02's finding — two lists that must
    /// agree are two lists that can drift, and the one that drifts silently
    /// narrows a sweep.
    ///
    /// It is anchored by [`variant_name`], whose match has no wildcard arm: a
    /// fourth variant fails to compile there first, and then fails these
    /// assertions **by name** rather than by count.
    const ALL_VARIANT_NAMES: [&str; 3] = ["Command", "Routed", "Goal"];

    /// A **total** classification of [`CommandSource`], with no wildcard arm.
    ///
    /// **The absence of a wildcard is the mechanism, and it is the whole point of
    /// this function existing at all.** A fourth `CommandSource` variant is a
    /// compile error *here, in the test file*, which means nobody can add a
    /// fourth command source without opening this module — which is precisely
    /// what did not happen when `--goal` was added beside `--command` and
    /// `--target-phase`, and CR-01 is what that cost.
    ///
    /// Deliberately **not** `std::mem::discriminant` and deliberately no new
    /// derive: a hash-based or opaque identity would keep compiling when a fourth
    /// variant appeared, and a mechanism that keeps compiling is not a mechanism.
    /// The `&'static str` is what lets the coverage assertion below name the
    /// variants it expects, so a matrix that quietly stopped producing one of
    /// them fails by name rather than by count.
    fn variant_name(source: &CommandSource) -> &'static str {
        match source {
            CommandSource::Command(_) => "Command",
            CommandSource::Routed(_) => "Routed",
            CommandSource::Goal(_) => "Goal",
        }
    }

    /// Every [`CommandSource`] variant, constructed once, paired with its name.
    ///
    /// **The SECOND anchor direction for [`ALL_VARIANT_NAMES`]** (pass-5 warning
    /// 4). [`variant_name`] anchors the const from one side: a fourth variant
    /// fails to compile there, so nobody can add one without opening this module.
    /// It does not anchor the other side — a const that had quietly gone stale,
    /// or a variant produced by no argv position, left the coverage assertion
    /// green with a whole variant unexercised.
    ///
    /// **This doc used to claim a fourth variant is "a compile error HERE too,
    /// in two ways: the array's declared length and the missing construction".
    /// That was false, and the correction rides the commit that falsifies it**
    /// (D-18-1, pass-6 WR-01). The verifier built a fourth variant, added its
    /// single arm to `variant_name`, and measured this pin PASSING with the
    /// variant unswept and **zero** compile errors. An array literal of
    /// constructed values carries no exhaustiveness obligation: nothing in Rust
    /// requires a `[T; N]` to mention every variant of `T`, and the declared
    /// length only forces the author of a fourth ENTRY to update it — not the
    /// author of a fourth VARIANT to add one.
    ///
    /// **What is actually measured, and by what.** The only compile-time anchor
    /// on `CommandSource`'s arity is [`variant_name`]'s wildcard-free match. It
    /// forces CLASSIFICATION — somebody must open this module and name the new
    /// variant — and that is all it forces. It does not force sweep growth.
    /// The set equality in the pin below is a runtime check between two lists
    /// that must agree; it catches a const that went stale relative to what this
    /// function builds, which is a real and different thing.
    ///
    /// **The residual, named rather than papered over.** A fourth variant
    /// classified in `variant_name` but never constructed here passes this pin
    /// with the variant unswept by the matrix's coverage assertion. What catches
    /// that is nothing mechanical — it is the reviewer reading this sentence,
    /// which is why the sentence is here. A `named(source)` destructuring helper
    /// would not change it: that too forces classification, not collection
    /// growth, and a second mechanism that overpromises is the exact defect
    /// class this round closes.
    fn one_of_each() -> [(&'static str, CommandSource); 3] {
        [
            ("Command", CommandSource::Command(visible("x"))),
            ("Routed", CommandSource::Routed(visible("x"))),
            ("Goal", CommandSource::Goal(visible("x"))),
        ]
    }

    #[test]
    fn all_variant_names_matches_the_variant_set_in_both_directions() {
        let built = one_of_each();

        assert_eq!(
            built.len(),
            ALL_VARIANT_NAMES.len(),
            "`ALL_VARIANT_NAMES` has {} entries and `one_of_each` builds {} \
             variants. A fourth `CommandSource` variant is a compile error in \
             `variant_name` ALONE — pass 6 measured that constructing values in \
             `one_of_each` carries no exhaustiveness obligation, so a variant \
             classified there and built nowhere passes this pin unswept. What \
             this assertion catches is the const and this function DISAGREEING; \
             the residual is named in `one_of_each`'s doc.",
            ALL_VARIANT_NAMES.len(),
            built.len()
        );

        // The label each entry carries must be the one `variant_name` derives
        // from the value beside it, or the pairing is decorative.
        for (label, source) in &built {
            assert_eq!(
                variant_name(source),
                *label,
                "`one_of_each` pairs {label:?} with a value `variant_name` calls \
                 {:?}; the pairing is what makes the set comparison below mean \
                 anything",
                variant_name(source)
            );
        }

        let mut names: Vec<&str> = built.iter().map(|(name, _)| *name).collect();
        names.sort_unstable();
        let mut expected = ALL_VARIANT_NAMES.to_vec();
        expected.sort_unstable();
        assert_eq!(
            names, expected,
            "the variants `one_of_each` constructs and the names \
             `ALL_VARIANT_NAMES` lists must be the same set. They are read by \
             different consumers — the preview enumeration and the matrix's \
             coverage assertion — and two lists that must agree are two lists \
             that can drift."
        );
    }

    /// The first line of `rendered` that is a number, a dot, and nothing a
    /// reader could see.
    ///
    /// The exact shape `build_report(project, "")` produced for a goal under
    /// CR-01, and for a blank `--command` under review-CR-02. **One detector
    /// shared by both tests below** rather than two copies: two places that answer
    /// the same question are two places that can disagree, which is the shape
    /// `CommandSource`'s own promotion was made to remove.
    ///
    /// **Its judgment of "visibly empty" is spelled out here rather than
    /// delegated to [`crate::text::carries_visible_content`], and the reason is
    /// narrower than the old doc claimed** (pass-5 WR-02). The old text credited
    /// this detector with breaking the trim tautology round-3 WR-03 found. It
    /// does not — and the replacement claim, that
    /// [`crate::test_support::DEGENERATE`] breaks it instead, was ALSO wrong and
    /// is corrected here (pass-7 gap 2, D-19-4). A hand-written enumeration is
    /// not independent of the predicate merely by being literal: every payload
    /// in that array was drawn from inside the class the implementation already
    /// covered, so it agreed with the implementation by construction for six
    /// rounds. What breaks the tautology is
    /// `crate::text::tests::every_format_character_the_standard_names_is_inside_the_class`,
    /// which sweeps all code points against an independently maintained
    /// derivation of the standard. No degenerate payload ever reaches this
    /// detector at all, because every one of them is refused at the parse
    /// boundary before anything renders.
    ///
    /// What this detector is genuinely for is the **realistic** half: a renderer
    /// that printed an invisible numbered entry beneath a *visible* payload. Its
    /// independently-DERIVED character class matters there — if
    /// [`payload::NonBlank`] were ever loosened to admit a zero-width payload,
    /// this detector would still judge a `U+200B` entry visibly empty and the
    /// matrix would go red — and the
    /// direct pins in
    /// [`the_visibly_empty_detector_is_falsifiable_on_its_own`] make that claim
    /// checkable rather than asserted. Do not replace this with a call to
    /// `carries_visible_content` or with `tail.trim()`.
    ///
    /// **Its class is no longer hand-written** (D-19-3). It used to spell the
    /// same three literal ranges production spelled, so "independent" bought
    /// nothing: pass 7 found the class short in both at once. See the inner
    /// `invisible` helper for what it derives from and what that does and does
    /// not buy.
    fn visibly_empty_numbered_entry(rendered: &str) -> Option<&str> {
        /// The detector's own answer to "is this character invisible?", derived
        /// INDEPENDENTLY OF PRODUCTION (D-19-3).
        ///
        /// It used to spell three literal ranges. Those ranges were the same
        /// 22-code-point subset production spelled, written twice, so the
        /// "independent" oracle and the thing it checked shared a mistake —
        /// which is why pass 7 found the class short in both places at once.
        /// It now reads the `unicode-properties` dev-dependency (unicode-rs)
        /// while production reads `icu_properties` (ICU4X), so the two derive
        /// the same standard from different data.
        ///
        /// The `Cf` half comes from the oracle; the default-ignorable half is
        /// named members, because the dev-dependency does not expose that
        /// property. Same disclosed gap as
        /// `text::tests::named_default_ignorable_members_beyond_cf_are_inside_the_class`,
        /// and deliberately the same member list — these two are the second and
        /// third consumers of one oracle, so a defect in `unicode-properties`
        /// deceives both. What neither can be deceived by is a defect in
        /// production's derivation, which is the only property that was ever
        /// load-bearing here. Do NOT replace this with a call to
        /// `crate::text::carries_visible_content` or with `tail.trim()`.
        fn invisible(c: char) -> bool {
            use unicode_properties::{GeneralCategory, UnicodeGeneralCategory};

            c.general_category() == GeneralCategory::Format
                || matches!(
                    c,
                    '\u{034f}'
                        | '\u{115f}'
                        | '\u{1160}'
                        | '\u{17b4}'
                        | '\u{17b5}'
                        | '\u{180b}'
                        | '\u{180d}'
                        | '\u{3164}'
                        | '\u{fe00}'..='\u{fe0f}'
                        | '\u{ffa0}'
                        | '\u{e0100}'..='\u{e01ef}'
                )
        }

        fn visible(text: &str) -> bool {
            text.chars()
                .any(|c| !(c.is_whitespace() || c.is_control() || invisible(c)))
        }

        rendered.lines().find(|line| {
            let trimmed = line.trim();
            trimmed.split_once('.').is_some_and(|(head, tail)| {
                !head.is_empty() && head.chars().all(|c| c.is_ascii_digit()) && !visible(tail)
            })
        })
    }

    /// **The detector, falsifiable on its own** (pass-5 WR-02).
    ///
    /// Every other consumer of [`visibly_empty_numbered_entry`] asserts it finds
    /// NOTHING, which a detector that had stopped working would satisfy forever —
    /// the shape this phase has now paid for three times. These four pins are the
    /// other direction: two entries it must report, two it must not.
    #[test]
    fn the_visibly_empty_detector_is_falsifiable_on_its_own() {
        assert_eq!(
            visibly_empty_numbered_entry("1. \u{200b}"),
            Some("1. \u{200b}"),
            "a numbered entry whose tail is one zero-width space is visibly \
             empty — it reads to a user as a command the run would issue, with \
             nothing after the number"
        );
        assert_eq!(
            visibly_empty_numbered_entry("1. \u{feff}\u{2060}"),
            Some("1. \u{feff}\u{2060}"),
            "and the same for a tail of a byte-order mark and a word joiner — \
             both survive `str::trim` untouched, which is why this detector \
             spells its own character classes"
        );
        assert_eq!(
            visibly_empty_numbered_entry("1. \u{202e}"),
            Some("1. \u{202e}"),
            "and for a tail of one RIGHT-TO-LEFT OVERRIDE — a character OUTSIDE \
             the three literal ranges this detector used to spell. Against the \
             pre-D-19-3 detector this assertion fails, which is the point: the \
             old 'independent' oracle carried production's own subset"
        );
        assert_eq!(
            visibly_empty_numbered_entry("1. x"),
            None,
            "a numbered entry with a visible command is not visibly empty; a \
             detector that reported this would make the matrix unsatisfiable"
        );
        assert_eq!(
            visibly_empty_numbered_entry("1. \u{65e5}"),
            None,
            "and a tail of one CJK ideograph is a visible command — the \
             over-detection bound, past ASCII, so a wrong oracle that swallowed \
             real script is caught here rather than silently reporting every \
             non-Latin entry as empty"
        );
        assert_eq!(
            visibly_empty_numbered_entry("no numbered entry here"),
            None,
            "and a line that is not a numbered entry at all is not one"
        );
    }

    /// **CR-01, as an assertion rather than as a review finding.**
    ///
    /// `--goal X --dry-run` used to fall through `preview_text`'s `(None, None)`
    /// arm to `build_report(project, "")`, which renders `commands: vec![""]`
    /// under `PreviewScope::Complete` — a `1 command in the sequence:` total and
    /// an empty numbered entry, printed beneath a header promising *the complete
    /// and honest sequence*. Against that build this test FAILS on the first
    /// assertion.
    ///
    /// It calls `preview_text` rather than only the renderer on purpose: CR-01
    /// was a **wiring** defect, and a renderer-only test would have passed
    /// against the broken tree.
    #[test]
    fn a_goal_only_preview_never_claims_an_empty_command_is_the_honest_sequence() {
        let root = tempfile::TempDir::new().expect("temp dir");
        let project = previewable(root.path());

        let rendered = preview_text(
            &project,
            &CommandSource::Goal(
                payload::NonBlank::new("get phase 22 verified")
                    .expect("a realistic goal is a payload"),
            ),
        );

        assert!(
            !rendered.contains("1 command in the sequence:"),
            "a goal has not been decomposed yet, so there is no total to state — \
             and `1 command in the sequence` beneath a header promising the \
             COMPLETE and honest sequence invites a user to authorise a run on a \
             claim the tool never checked; got:\n{rendered}"
        );
        assert!(
            !rendered.lines().any(|line| line.trim() == "1."),
            "an empty numbered entry reads as a command the run would issue; \
             got:\n{rendered}"
        );
        assert!(
            rendered.contains("get phase 22 verified"),
            "the preview must name the goal it is a preview OF; got:\n{rendered}"
        );
        assert!(
            rendered.contains(dry_run::SECTION_COMMANDS),
            "the pinned commands section still renders — a blank section reads as \
             a missing one; got:\n{rendered}"
        );
    }

    /// **The invariant a fourth command source has to be added to.**
    ///
    /// CR-01 was not a mistake in a branch; it was a third source added
    /// *beside* two others without the preview being told. This walks every
    /// `CommandSource` variant, renders each through `preview_text`, and asserts
    /// of every rendering that it carries the pinned section and shows no empty
    /// numbered entry.
    ///
    /// **The array literal is the mechanism.** It is written as an exhaustive
    /// list of constructed variants rather than as a helper that generates them,
    /// so a fifth arm on the enum is a change somebody has to make *here* — and
    /// the per-variant sweep below is what makes forgetting to extend the array a
    /// failure rather than a silently narrower sweep.
    ///
    /// **This test carries REALISTIC payloads only, and that is deliberate now
    /// rather than accidental.** As written by 21-07 it enumerated
    /// `CommandSource::Command("/gsd:progress")` and nothing blanker, so it
    /// passed vacuously against the one source that already had review-CR-02's
    /// bug. The degenerate payloads are enumerated in
    /// [`every_command_source_refuses_or_previews_cleanly_for_every_degenerate_payload`]
    /// below instead, and they are enumerated *there* because that is where they
    /// are production-reachable: on argv, through [`command_source`], which is
    /// the single production constructor of this type. Defending the renderer
    /// against a hand-constructed `CommandSource::Command(String::new())` would
    /// be a second place answering a question the seam already answers, and two
    /// such places are two places that can disagree.
    #[test]
    fn every_command_source_renders_a_preview_with_no_empty_numbered_command() {
        let root = tempfile::TempDir::new().expect("temp dir");
        let project = previewable(root.path());

        let sources = [
            CommandSource::Command(
                payload::NonBlank::new("/gsd:progress").expect("a realistic command"),
            ),
            CommandSource::Routed(
                payload::NonBlank::new("20").expect("a realistic target phase"),
            ),
            CommandSource::Goal(
                payload::NonBlank::new("get phase 22 verified").expect("a realistic goal"),
            ),
        ];

        // Non-vacuity, in the register `tests/spawn_seam_guard.rs` uses: an
        // enumeration that had quietly stopped covering a variant would pass for
        // the wrong reason.
        //
        // **Keyed on the VARIANT rather than on the array index.** The sweep this
        // replaced compared `expected == 0/1/2` against the array position, which
        // is a per-position check wearing a per-variant check's name: it proved
        // the array had three entries in a fixed order, and would have kept
        // passing if two of those entries had been the same variant. `variant_name`
        // is a `match` with no wildcard, so a fourth variant is a compile error
        // and a duplicated one is a count of 2 here.
        for expected in ALL_VARIANT_NAMES {
            let present = sources
                .iter()
                .filter(|source| variant_name(source) == expected)
                .count();
            assert_eq!(
                present, 1,
                "each command source must appear exactly once in the enumeration; \
                 `{expected}` appeared {present} times"
            );
        }

        for source in &sources {
            let rendered = preview_text(&project, source);

            assert!(
                rendered.contains(dry_run::SECTION_COMMANDS),
                "every source renders the pinned commands section; {source:?} did \
                 not:\n{rendered}"
            );
            let empty_entry = visibly_empty_numbered_entry(&rendered);
            assert!(
                empty_entry.is_none(),
                "no preview may render a visibly empty numbered entry — it reads \
                 as a command the run would issue, and printing one for a source \
                 the renderer did not recognise is exactly how CR-01 shipped. \
                 {source:?} produced {empty_entry:?} in:\n{rendered}"
            );
        }
    }

    /// One argv position, as a builder over the RAW record the boundary takes.
    ///
    /// A higher-ranked fn pointer rather than a boxed closure so the table stays
    /// a literal and the borrow is the payload's own.
    type PositionBuilder = for<'a> fn(&'a str) -> RawDriveArgs;

    /// Whether a refusal is the one this position owns.
    ///
    /// A fn pointer rather than a variant, because the seven positions produce
    /// four different `DriveError` variants and two of them carry fields; a
    /// predicate is what lets each row name its own refusal BY NAME rather than
    /// the matrix settling for "some error happened".
    type PositionRefusal = fn(&DriveError) -> bool;

    /// One row of the matrix: the flag, a realistic payload, the builder that
    /// places a payload into that position, and the refusal that position owns.
    type Position = (&'static str, &'static str, PositionBuilder, PositionRefusal);

    /// A `RawDriveArgs` whose every field is a legitimate value, for a builder to
    /// place one payload into.
    ///
    /// The remainder has to be VALID, or a row could pass for the wrong reason:
    /// a matrix whose baseline was itself refused would report every cell `Err`
    /// no matter what the position under test contained.
    fn raw_baseline() -> RawDriveArgs {
        RawDriveArgs {
            alias: "demo".to_string(),
            command: Some("/gsd:progress".to_string()),
            target_phase: None,
            max_steps: None,
            wall_clock_cap_secs: None,
            max_escalations: None,
            approved_plan: None,
            run_id: Some("2026-08-19T12-00-00Z-aaaa".to_string()),
            dry_run: false,
            goal: None,
            #[cfg(debug_assertions)]
            claude_program: None,
            #[cfg(debug_assertions)]
            claude_args: Vec::new(),
        }
    }

    /// The seven invocation positions, each with the refusal its own flag owns.
    ///
    /// **The row TABLE is hand-maintained and that is disclosed rather than
    /// glossed.** What is mechanical is the *type*: `DriveArgs::from_argv`'s
    /// exhaustive destructure means a seventh argv field cannot compile until
    /// somebody classifies it, and 21-16's guard nine additionally refuses a
    /// raw-`String` argv field declared in this tree's style. Neither forces a
    /// ROW here. A seventh field correctly typed `NonBlank` but given no row
    /// would leave this matrix at 7×6 silently. That residual is a COVERAGE gap,
    /// not a blank-payload route — the type still refuses the blank — and it is
    /// stated here rather than claimed closed.
    fn positions() -> Vec<Position> {
        vec![
            (
                "--alias",
                "demo",
                (|payload| RawDriveArgs {
                    alias: payload.to_string(),
                    ..raw_baseline()
                }) as PositionBuilder,
                // Corrected in 21-17 from `OptIn(UnknownAlias)`: a blankness
                // refusal must not assert that no project is registered under
                // the value, because pass 6 measured that one can be (WR-06).
                (|err| matches!(err, DriveError::AliasNotVisible { .. })) as PositionRefusal,
            ),
            (
                "--command",
                "/gsd:progress",
                |payload| RawDriveArgs {
                    command: Some(payload.to_string()),
                    ..raw_baseline()
                },
                |err| matches!(err, DriveError::NoCommandSource),
            ),
            (
                "--target-phase",
                "20",
                |payload| RawDriveArgs {
                    command: None,
                    target_phase: Some(payload.to_string()),
                    ..raw_baseline()
                },
                |err| matches!(err, DriveError::NoCommandSource),
            ),
            (
                "--goal (sole source)",
                "get phase 22 verified",
                |payload| RawDriveArgs {
                    command: None,
                    goal: Some(payload.to_string()),
                    ..raw_baseline()
                },
                |err| matches!(err, DriveError::NoCommandSource),
            ),
            // **The multi-flag row pass 5 proved unexpressible in the old
            // shape.** The matrix used to be a table of `command_source`'s three
            // arguments, so "a blank `--goal` BESIDE a visible `--command`" was
            // not a cell it could hold — and that is precisely the invocation
            // IN-01 reproduced writing `"goal": "   "` into a committed record
            // after the lock, the journal and the run directory existed.
            (
                "--goal beside a visible --command",
                "get phase 22 verified",
                |payload| RawDriveArgs {
                    goal: Some(payload.to_string()),
                    ..raw_baseline()
                },
                |err| matches!(err, DriveError::NoCommandSource),
            ),
            (
                "--run-id beside a visible --command",
                "2026-08-19T12-00-00Z-aaaa",
                |payload| RawDriveArgs {
                    run_id: Some(payload.to_string()),
                    ..raw_baseline()
                },
                |err| matches!(err, DriveError::RunIdInvalid { .. }),
            ),
            (
                "--approved-plan beside a visible --command",
                "plan-digest+approval-digest",
                |payload| RawDriveArgs {
                    approved_plan: Some(payload.to_string()),
                    ..raw_baseline()
                },
                |err| matches!(err, DriveError::PlanApprovalMalformed(_)),
            ),
        ]
    }

    /// **The enumeration that would have caught review-CR-02, CR-01 before it,
    /// and pass 5's three Criticals after them — without anybody having to pick
    /// the right payload or the right flag.**
    ///
    /// Every degenerate payload, in every argv position, driven through the
    /// **production parse boundary** [`DriveArgs::from_argv`] rather than through
    /// a hand-constructed value.
    ///
    /// **The rule is UNIFORM and there is no exemption.** Every
    /// [`crate::test_support::DEGENERATE`] payload in every position is an `Err`
    /// carrying that position's own named variant — one nested loop, no `Ok`
    /// branch for a degenerate cell, no per-position hand-written sweep. The
    /// shape this replaces offered each cell a *disjunction* and then re-asserted
    /// the strict rule by name for some columns; a Critical walked between them.
    ///
    /// **What moved, and why the axis changed.** The predecessor's column axis
    /// was tied to `command_source`'s arity through a 3-tuple `PositionBuilder`,
    /// so a fourth argv *field* that resolved to an existing variant added a
    /// column the matrix did not have — which is exactly what `--run-id`,
    /// `--approved-plan` and the alias always were. The axis is now
    /// `DriveArgs::from_argv`, so the columns are argv POSITIONS rather than
    /// resolver parameters, and a new argv field breaks `from_argv`'s exhaustive
    /// destructure at compile time. Giving it a row here is the classification
    /// that commit must contain; see [`positions`] for what that does and does
    /// not force.
    #[test]
    fn every_argv_position_refuses_every_degenerate_payload_at_the_parse_boundary() {
        for (position, realistic, build, expected) in positions() {
            // **The degenerate half: one rule, every cell, no branch.** Not a
            // `match` offering an `Ok` arm — there is no acceptable `Ok` for a
            // degenerate payload in any position, so the test's shape cannot
            // express one.
            for payload in crate::test_support::DEGENERATE {
                let outcome = DriveArgs::from_argv(build(payload));
                let err = match outcome {
                    Err(err) => err,
                    Ok(_) => panic!(
                        "every degenerate payload in every argv position is \
                         refused at the parse boundary, uniformly and with no \
                         exemption — a value carrying no visible instruction is \
                         nothing a run can act on whichever flag carried it, and \
                         the four cycles this matrix has now outlived each ended \
                         with one position quietly excused. {position} with \
                         payload {payload:?} was ACCEPTED"
                    ),
                };
                assert!(
                    expected(&err),
                    "{position} with payload {payload:?} must be refused with \
                     that position's own named variant, so the user is told \
                     which flag to fix; got {err:?}"
                );
            }

            // **The realistic half**, which is also the non-vacuity control: a
            // boundary that passed the loop above by refusing everything fails
            // here.
            let accepted = DriveArgs::from_argv(build(realistic)).unwrap_or_else(|err| {
                panic!(
                    "a realistic {position} payload {realistic:?} must still be \
                     accepted — a boundary that refuses everything is not a \
                     boundary; got {err:?}"
                )
            });
            // And the payload arrives INTACT: the boundary judges, it never
            // rewrites what the user typed.
            let _ = accepted;
        }
    }

    /// **The blankness boundary, pinned on the OTHER side, in every position.**
    ///
    /// Zero visible characters is refused above; ONE visible character must be
    /// accepted, or the refusal is a length rule wearing an emptiness rule's
    /// name. Each payload is padded with the exact whitespace and zero-width
    /// characters `DEGENERATE` is made of, so what is pinned is *visibility*,
    /// not brevity.
    ///
    /// **The `--alias`/`--run-id` exemption is DELETED and the rationale that
    /// carried it was false** (D-18-3, pass-6 IN-01). It read: "both are composed
    /// into path components downstream, where `is_plain_path_component` answers a
    /// stricter structural question that is not this test's subject." The verifier
    /// measured that `from_argv` never calls `is_plain_path_component` — for
    /// either field — so the sentence justified a narrowing with a fact about a
    /// function this boundary does not invoke. That is the cycle-3 defect
    /// recurring inside the acceptance matrix, and hand-picked exemptions are how
    /// five rounds each left one shape uncovered. All SEVEN positions now take all
    /// FOUR padded payloads.
    ///
    /// **What is true instead.** `from_argv` judges VISIBILITY. One visible
    /// character is an instruction in every position, `--alias` and `--run-id`
    /// included, and this boundary accepts a padded look-alike in those positions
    /// **by design** — a `NonBlank` says something can be seen, not that the bytes
    /// name exactly one thing. The identity question is answered DOWNSTREAM, at
    /// the seams: `journal::is_plain_path_component` at the run-id and
    /// target-phase seams (since 21-17's D-17-1 clause) and `registry::Alias::new`
    /// at registration, each pinned with `test_support::LOOK_ALIKE_PAIRS` in its
    /// own suite. The drive-level consequence, stated so it is not discovered: a
    /// look-alike `--run-id` passes THIS boundary and is refused by `drive` at the
    /// seam, before anything is created.
    #[test]
    fn one_visible_character_is_accepted_in_every_argv_position() {
        for (position, _, build, _) in positions() {
            let padded: &[&str] = &["x", " x ", "\u{200b}x", "x\u{feff}"];
            for payload in padded {
                let outcome = DriveArgs::from_argv(build(payload));
                assert!(
                    outcome.is_ok(),
                    "one visible character is an instruction; {position} with \
                     payload {payload:?} must be accepted, got {:?}",
                    outcome.err()
                );
            }
        }
    }

    /// **The pairing the old ambiguity pins covered, at its new home.**
    ///
    /// `--command '' --target-phase 20` used to report `AmbiguousCommandSource`
    /// from `command_source`; it is now refused at the boundary with
    /// `NoCommandSource`, before a `DriveArgs` exists. The variant changed and
    /// the legality did not: the invocation is still refused, and refused
    /// earlier, creating nothing. What T-21-11-05 forbids — a blank payload
    /// DEMOTING an ambiguous invocation into a legal one — remains impossible,
    /// which is what this asserts.
    #[test]
    fn a_blank_payload_beside_a_visible_one_is_still_refused_at_the_boundary() {
        for blank in crate::test_support::DEGENERATE {
            for (label, raw) in [
                (
                    "a blank --command beside a --target-phase",
                    RawDriveArgs {
                        command: Some(blank.to_string()),
                        target_phase: Some("20".to_string()),
                        ..raw_baseline()
                    },
                ),
                (
                    "a blank --command beside a --goal",
                    RawDriveArgs {
                        command: Some(blank.to_string()),
                        goal: Some("a real goal".to_string()),
                        ..raw_baseline()
                    },
                ),
                (
                    "a blank --target-phase beside a --goal",
                    RawDriveArgs {
                        command: None,
                        target_phase: Some(blank.to_string()),
                        goal: Some("a real goal".to_string()),
                        ..raw_baseline()
                    },
                ),
            ] {
                assert!(
                    matches!(
                        DriveArgs::from_argv(raw),
                        Err(DriveError::NoCommandSource)
                    ),
                    "{label} must be refused, never promoted into a source the \
                     caller's precedence did not choose; blank={blank:?}"
                );
            }
        }
    }

    /// The "guaranteed `Err`" that `DriveArgs::from_argv`'s `--approved-plan`
    /// refusal derives its error from, pinned as a CHECKED fact.
    ///
    /// The boundary runs the real [`journal::parse_approval_token`] on a blank
    /// token and wraps its `Err`. That is only honest if the parser really does
    /// refuse every blank shape — so this asserts it rather than assuming it. A
    /// parser that started accepting one would fail here, loudly, instead of
    /// silently routing a blank token through the boundary's unreachable arm.
    #[test]
    fn a_blank_approval_token_is_refused_by_the_real_parser() {
        for blank in crate::test_support::DEGENERATE {
            assert!(
                journal::parse_approval_token(blank).is_err(),
                "{blank:?} carries neither half of an approval token and must be \
                 refused by the parser the boundary derives its refusal from"
            );
        }
        // Non-vacuity: a well-formed token still parses, so the refusals above
        // are about blankness rather than about a parser that refuses
        // everything.
        assert!(
            journal::parse_approval_token("plan-digest+approval-digest").is_ok(),
            "a well-formed token must still parse, or the loop above is \
             satisfied by a parser that has stopped working"
        );
    }
}
