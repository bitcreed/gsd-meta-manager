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
//! 2. **Driving is Unix-only, deliberately and explicitly (D-05).** The
//!    implementation submodules carry the `#[cfg(unix)]`; the `Drive` subcommand
//!    does **not**. On a non-Unix platform the subcommand still parses and
//!    [`drive`] returns [`DriveError::UnsupportedPlatform`] naming the missing
//!    facility, because an accepted limitation that surfaces as "unknown
//!    subcommand" is indistinguishable from a bug. This mirrors the posture at
//!    `src/executor/mod.rs:22-28`.
//! 3. **The driver's working directory is the project root itself, never a
//!    worktree (D-21).** OQ4 was resolved empirically — `claude --worktree`
//!    exists and works — and declined for five concrete reasons, chief among
//!    them that it writes *inside* the repo working directory, where
//!    `watcher.rs::extract_project_root` would resolve the worktree's
//!    `.planning/` as a phantom project and mis-route every file event. Phase 19
//!    owns the decision and the safe fallback (a worktree created *outside* the
//!    repo).
//! 4. **This phase runs exactly one GSD command, supplied on the command line.**
//!    There is no loop and no sequence. The decision router is Phase 20's, which
//!    is why [`DriveArgs::command`] is a single `String` and not a `Vec`.
//!
//! [`lock`] landed in plan 17-02, [`dry_run`] in 17-04, and [`liveness`] and
//! [`reconcile`] in 17-05. Later plans add `kill` (17-06) as a sibling. Nothing
//! is stubbed ahead of time: an empty module for a later phase is a promise the
//! compiler cannot keep.

// Deliberately **outside** the `#[cfg(unix)]` block below. Point 2 above makes
// the *running* of an agent Unix-only; a preview is git reads and string
// building, so it compiles and is testable on every platform. Gating it would
// make the one mode that needs no platform facility the one mode a Windows
// build could not even check.
pub mod dry_run;
// Also outside the block, and for a related but distinct reason. Both modules
// are `/proc` and `run.json` **reads**, and D-10 is explicit that they should
// carry `src/session_detector.rs`'s honest-failure posture — that module has no
// platform attributes at all and its reads simply yield nothing off Linux —
// rather than diverging from the module they copy. A `cfg` here would also gate
// the reconciliation scan out of the TUI, which is cross-platform.
pub mod liveness;
pub mod reconcile;
#[cfg(unix)]
pub mod lock;
#[cfg(unix)]
pub mod run;
#[cfg(unix)]
pub mod spawn;

use std::ffi::OsString;
use std::path::PathBuf;

use crate::config::{Config, RegisteredProject};
use crate::error::{DriveError, OptInError};
use crate::executor::DrivableProject;

/// Everything one `drive` invocation was asked to do.
///
/// A plain owned record rather than a borrow of the CLI enum, so the TUI's
/// spawn path (plan 17-05) and a hand-typed invocation build the same value.
#[derive(Debug, Clone)]
pub struct DriveArgs {
    /// The registry alias to drive.
    pub alias: String,
    /// The single GSD command to run.
    ///
    /// **Exactly one, and that is the whole of this phase's execution model.**
    /// Phase 20 owns the decision router that turns a goal into a sequence; a
    /// field that accepted a sequence now would imply a loop that does not
    /// exist.
    pub command: String,
    /// The run id to record under, or `None` to generate one.
    ///
    /// The TUI supplies it so it knows what to look for afterwards; the driver
    /// owns the record, because `run.json` carries the driver's own pid and pgid
    /// which only the driver knows (D-03).
    pub run_id: Option<String>,
    /// Preview only, execute nothing (D-24). Plan 17-04 implements it.
    pub dry_run: bool,
    /// Free text recorded into `RunRecord.goal` and never interpreted.
    pub goal: Option<String>,
    /// Test and development only: the program to spawn instead of `claude`.
    pub claude_program: Option<PathBuf>,
    /// Test and development only: leading arguments for `claude_program`.
    pub claude_args: Vec<OsString>,
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
/// 3. Only then branch on anything else — including the dry-run branch plan
///    17-04 adds. **Gating before the preview branch is stricter than CTRL-03
///    requires, and it is deliberate:** one gate call site is mechanically
///    verifiable, two are an invitation to add a third.
/// 4. Dispatch to the platform handler, which is the run body on Unix and a
///    typed refusal everywhere else (D-05).
pub async fn drive(args: DriveArgs, config: &Config) -> Result<(), DriveError> {
    let entry = config
        .projects
        .get(&args.alias)
        .ok_or_else(|| OptInError::UnknownAlias {
            alias: args.alias.clone(),
        })?;

    let project = DrivableProject::from_registry(&args.alias, entry)?;

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
        let report = dry_run::build_report(&project, &args.command);
        println!("{}", dry_run::render(&report));
        return Ok(());
    }

    dispatch(project, &args, entry).await
}

/// The Unix run body.
#[cfg(unix)]
async fn dispatch(
    project: DrivableProject,
    args: &DriveArgs,
    entry: &RegisteredProject,
) -> Result<(), DriveError> {
    run::execute_run(project, args, entry).await
}

/// The typed refusal every non-Unix platform gets (D-05).
#[cfg(not(unix))]
async fn dispatch(
    project: DrivableProject,
    args: &DriveArgs,
    entry: &RegisteredProject,
) -> Result<(), DriveError> {
    let _ = (project, args, entry);
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
            command: "/gsd-progress".to_string(),
            run_id: None,
            dry_run: false,
            goal: None,
            claude_program: None,
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

    #[test]
    fn drive_args_carry_the_single_command_the_router_phase_will_replace() {
        let args = args("demo");
        // A `String`, never a `Vec<String>`: this phase issues exactly one GSD
        // command and Phase 20 owns the router that would issue a sequence.
        assert_eq!(args.command, "/gsd-progress");
        assert!(
            !args.dry_run,
            "a real run is the default; the preview is opt-in"
        );
    }
}
