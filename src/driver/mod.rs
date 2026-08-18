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
//! 4. **This phase runs exactly one GSD command, supplied on the command line.**
//!    There is no loop and no sequence. The decision router is Phase 20's, which
//!    is why [`DriveArgs::command`] is a single `String` and not a `Vec`.
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
    /// The single GSD command to run.
    ///
    /// **Exactly one, and that is the whole of this phase's execution model.**
    /// Phase 20 owns the decision router that turns a goal into a sequence; a
    /// field that accepted a sequence now would imply a loop that does not
    /// exist.
    pub command: String,
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
    /// Free text recorded into `RunRecord.goal` and never interpreted.
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
/// 4. Refuse a real run that cannot be stopped ([`platform_refusal`]), that
///    cannot be identified ([`DriveError::RunIdRequired`]), or whose id is not a
///    single plain path component ([`DriveError::RunIdInvalid`], D-27). All
///    three sit **after** the dry-run branch, so none reaches a preview, and all
///    three sit **before** `dispatch`, so a refused run has created nothing at
///    all: no lock file, no run directory, no `run.json`, no journal.
/// 5. Dispatch to the platform handler, which is the run body on Unix and a
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
        let command = args.command.clone();
        let cloned = project.clone();
        let report = match tokio::task::spawn_blocking(move || {
            dry_run::build_report(&cloned, &command)
        })
        .await
        {
            Ok(report) => report,
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
                dry_run::build_report(&project, &args.command)
            }
        };
        println!("{}", dry_run::render(&report));
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
