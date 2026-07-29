//! The run body: gate, journal start, executor spawn, event drain, journal
//! finish.
//!
//! This module is declared `#[cfg(unix)]` by its parent, so it carries no inner
//! attribute of its own — the portable surface lives in
//! [`crate::driver`](super) and only the implementation is gated (D-05).
//!
//! It is the **first production caller** of Phase 15's `ClaudeExecutor` and
//! Phase 16's `JournalRun`. Both were shipped complete and both were dead code
//! until this file existed; `src/journal/mod.rs` says so in as many words.

use std::path::PathBuf;

use crate::config::RegisteredProject;
use crate::driver::{lock, DriveArgs};
use crate::error::DriveError;
use crate::executor::claude::ClaudeExecutor;
use crate::executor::{
    DrivableProject, ExecutionHandle, ExecutionOptions, Executor, RunOutcome,
};
use crate::journal::{self, JournalEvent, JournalRun, RunRecord};

/// The diagnostic code the terminate-signal shutdown journals before its
/// terminal record.
///
/// A fixed identifier rather than a sentence, because it is what a later reader
/// greps for: a run that ends with the killed outcome could have been stopped
/// from the TUI, stopped from a shell, or stopped by a service manager, and this
/// record is the only thing that says a terminate signal is what arrived.
const TERMINATE_DIAGNOSTIC_CODE: &str = "terminate_signal_shutdown";

/// The state one driver process holds for the duration of one run.
pub struct DriverRun {
    /// The run's journal, open from `start` to `finish`.
    journal: JournalRun,
    /// The single-execution lock, held for the whole run (D-20.2).
    ///
    /// **This field is deliberately never read, and it is not bookkeeping.** An
    /// advisory `flock` is held per *open file description*, so dropping this
    /// value closes the descriptor and releases the lock — the field's only job
    /// is to keep it alive, and its placement on the run state rather than in a
    /// local is what makes "held for the run's duration" true. It is not renamed
    /// to `_lock`: an underscore reads as "leftover" and would invite the next
    /// reader to delete the lock along with it.
    #[allow(dead_code)]
    lock: lock::RunLock,
}

/// Map a derived outcome onto the short label the journal records.
///
/// Kept as its own function rather than inlined so plan 17-06 reuses it for the
/// killed path instead of inventing a second vocabulary for the same states.
fn outcome_label(outcome: &RunOutcome) -> &'static str {
    match outcome {
        RunOutcome::SucceededWithChanges { .. } => "succeeded_with_changes",
        RunOutcome::SucceededNoChanges { .. } => "succeeded_no_changes",
        RunOutcome::Failed { .. } => "failed",
        RunOutcome::PermissionDenied { .. } => "permission_denied",
        RunOutcome::Killed { .. } => "killed",
        RunOutcome::TimedOut { .. } => "timed_out",
        RunOutcome::Stalled { .. } => "stalled",
        RunOutcome::CapabilityRefused { .. } => "capability_refused",
        RunOutcome::SpawnFailed { .. } => "spawn_failed",
    }
}

/// Build the immutable half of `run.json`.
///
/// **`run.json` is written by the driver, not by the TUI before spawning.** The
/// record carries the driver's own pid and pgid, which only the driver knows. A
/// spawn failure is synchronous and reportable in the TUI, and correctly leaves
/// nothing at all on disk rather than a half-record (D-03).
///
/// **`pgid` is a parameter and not a second `std::process::id()` call, and that
/// is WR-01.** The two are equal for every run that reaches this point, but they
/// were equal here because the same expression was written twice rather than
/// because anything had been observed. On a failed `setpgid` — `EPERM` for a
/// session leader is the realistic one — the process does **not** lead its own
/// group, and the record asserted a leadership it did not hold. What that costs
/// downstream is concrete: `kill::resolve_signal_target` compares the recorded
/// group against the kernel's, so such a record makes every stop against that run
/// refuse, and the user's kill switch stops working for a reason nothing on
/// screen can explain. The caller passes what [`current_group`] reports.
fn make_run_record(
    run_id: String,
    args: &DriveArgs,
    entry: &RegisteredProject,
    options: &ExecutionOptions,
    argv_digest: String,
    pgid: u32,
) -> RunRecord {
    RunRecord {
        run_id,
        goal: args.goal.clone().unwrap_or_default(),
        gsd_command: args.command.clone(),
        target: format!("{:?}", options.target),
        // The field Phase 16 reserved at `src/journal/mod.rs:475` specifically
        // so this phase adds no migration.
        opt_in: entry
            .driver_opt_in
            .as_ref()
            .map(|record| record.opted_in_at.clone()),
        started_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
        session_id: options.session_id.to_string(),
        // D-04's arithmetic still holds — `pid == pgid` for every run that gets
        // here — but it now holds because the group was **observed** rather than
        // because the same expression was written on both lines. A record that
        // asserts a leadership the process does not hold is worse than one that
        // reports an inherited group: the first makes the kill switch refuse,
        // the second at least tells the truth about what to signal.
        pid: std::process::id(),
        pgid,
        // Empty until the first `system/init`. Record what is known; nothing
        // overwrites it, because `run.json` is written exactly twice.
        claude_code_version: String::new(),
        argv_digest,
        ended_at: None,
        outcome: None,
    }
}

/// The process group this process is **actually** in, straight from the kernel.
///
/// `getpgrp()`, which cannot fail: every process is in exactly one group.
///
/// **Split out of [`establish_own_group`] rather than inlined into it, and the
/// reason is testability without collateral damage.** A test that wanted to
/// cross-check the syscall against `liveness::process_group`'s `/proc` parse
/// could not call `establish_own_group`, because `setpgid` in a shared test
/// binary would move **the harness's own process group** — every other test in
/// the same process, and the runner's job control with them. This function
/// observes and changes nothing, so the cross-check costs nothing.
fn current_group() -> u32 {
    rustix::process::getpgrp().as_raw_nonzero().get() as u32
}

/// Become this process's own group leader, and report the group it ends up in.
///
/// `setpgid(0, 0)`. Under the TUI's spawn — which already applies
/// `process_group(0)` — this is a harmless no-op. It exists for the other launch
/// mode: a **hand-typed** `gsd-meta-manager drive`, which would otherwise
/// inherit the shell's job group and write a `pgid` naming a group it does not
/// lead. A later teardown signalling that pgid would signal the user's shell job
/// instead of the run (T-17-03).
///
/// An `EPERM` — the process is already a session leader — is downgraded to a
/// warning carrying the error **kind** only. It is not a reason to refuse a run.
///
/// **It returns [`current_group`] rather than nothing, and that return value is
/// WR-01.** The previous doc said this call *"is what makes the equality honest
/// rather than assumed"*, and that was true of the call and false of the record:
/// `make_run_record` wrote `std::process::id()` into `pgid` unconditionally, so
/// on the very failure path the `warn!` above describes the record still claimed
/// group leadership. Returning the group the kernel reports is what closes the
/// gap between the warning and the document — the record now says what happened,
/// including when what happened was not what was asked for.
fn establish_own_group() -> u32 {
    if let Err(err) = rustix::process::setpgid(None, None) {
        tracing::warn!(
            kind = ?err.kind(),
            "could not become process group leader; the recorded pgid may name an inherited group",
        );
    }
    current_group()
}

/// **Layers 2 and 3 of D-06's four-step stop**, in the order that makes them
/// work.
///
/// The stop the user presses is two-layer because there are **two process
/// groups, not one**: Phase 15 spawns `claude` with `ProcessGroup::leader()`, so
/// its pgid is distinct from this driver's and the TUI's
/// `kill(-driver_pgid, SIGTERM)` does not reach it. Layer 1 (the signal to this
/// process's group) and layer 4 (the grace, the escalation and the reap) belong
/// to [`crate::driver::kill`]; the two below belong here:
///
/// 1. **Layer 2 — `Executor::cancel`, and that single call is the whole of it.**
///    It is Phase 15's already-built sequence at `src/executor/claude.rs:1482-1549`:
///    SIGTERM to the `claude` process **group**, a ten-second grace, SIGKILL,
///    then an `wait()` that is deliberately unbounded and deliberately raced
///    against nothing, because that final wait is what stops the grandchildren
///    becoming zombies.
///
///    **A second teardown must not be written here, and the reason is specific
///    rather than stylistic** (D-06.2). `process-wrap`'s `start_kill()` and its
///    `kill()` convenience both send the **uncatchable** signal; `signal(15)` is
///    the only graceful path, and reading `kill()` as "terminate politely" is
///    natural and wrong. Re-implementing that distinction here is exactly how
///    the CLI's documented clean shutdown — the turn abort, the Bash-tree
///    teardown through its own handler, the `SessionEnd` hooks — gets silently
///    skipped, which is the failure mode the whole two-layer design exists to
///    prevent.
///
/// 2. **The reason, journaled before the ending.** A `Diagnostic` carrying
///    [`TERMINATE_DIAGNOSTIC_CODE`], written first so the *why* survives even if
///    the terminal write is the one that fails.
///
/// 3. **Layer 3 — the terminal record.** This is the write that makes a stopped
///    run distinguishable from a crashed one on disk: a crashed run has no
///    `ended_at` (that absence *is* Phase 16's crash contract), a stopped one has
///    `ended_at` plus the killed outcome. `src/driver/reconcile.rs` reads exactly
///    that difference, so skipping it would make every stop look like a crash.
///
/// The outcome label comes from the outcome `cancel` actually returned rather
/// than from a hard-coded `Killed`: the coordinator maps a cancelled run onto
/// [`RunOutcome::Killed`] at `src/executor/claude.rs:1223`, so this reports
/// `killed` by construction, and in the rare case a wall-clock or idle breach
/// was classified in the same pass it reports the breach truthfully instead of
/// overwriting it with a label that is merely expected.
///
/// The lock is **not** released here. Releasing it is dropping [`DriverRun`],
/// which happens after this returns, so the release lands after the last write
/// rather than in the middle of it (D-20.2). An explicit unlock would be a second
/// release path for one resource.
async fn shutdown_on_terminate(
    executor: &ClaudeExecutor,
    handle: &mut ExecutionHandle,
    journal: &mut JournalRun,
) {
    let outcome = executor.cancel(handle).await;

    if let Err(err) = journal.record(&JournalEvent::Diagnostic {
        code: TERMINATE_DIAGNOSTIC_CODE.to_string(),
        detail: "the driver received the terminate signal and tore down its agent process group"
            .to_string(),
    }) {
        // The error KIND only, never a message body (T-17-05). A journal that
        // cannot take the diagnostic must still be given the chance to take the
        // terminal record, which is the more important of the two.
        tracing::warn!(
            kind = ?err.kind(),
            "could not journal the terminate-signal diagnostic",
        );
    }

    if let Err(err) = journal.finish(outcome_label(&outcome)) {
        tracing::warn!(
            detail = %format!("{err:#}"),
            "could not close the journal after a terminate-signal shutdown",
        );
    }
}

/// Run one GSD command to completion and leave a complete run directory.
///
/// In order, and the order is the decision:
///
/// 1. Become our own process group leader, so D-04's `pid == pgid` invariant is
///    established rather than assumed.
/// 2. Take the single-execution lock, **after** the opt-in gate (which ran in
///    [`crate::driver::drive`]) and **before** the journal. Both boundaries are
///    the decision: a project the user never opted in must not get a lock file
///    in its tree (T-17-14), and a *losing* run must prune nothing, create no
///    run directory, write no `run.json` and clear nobody's `active` pointer —
///    D-12's evidence-preservation argument applies to a loser with exactly as
///    much force as to a crash.
/// 3. Start the journal — `run.json` write one of two, plus `run_started`.
/// 4. Spawn the agent. A spawn failure still calls `finish`, so a run that
///    started always has a terminal record (T-17-06).
/// 5. Journal the `claude` process group id **before draining a single event**:
///    a teardown handle recorded late is a teardown handle that can be missed.
/// 6. Drain the event stream into the journal, racing the terminate signal
///    **first** — see the `biased` `select!` below.
/// 7. Finish the journal with the derived outcome — `run.json` write two.
///
/// The lock guard lives on [`DriverRun`], which outlives the terminal
/// `finish` call, so the lock is released **after** the last write rather than
/// somewhere in the middle of it.
///
/// The `entry` parameter carries the opt-in record whose timestamp lands in
/// `RunRecord.opt_in`; the [`DrivableProject`] token proves the gate ran, but by
/// construction it carries only the alias and the root.
pub async fn execute_run(
    project: DrivableProject,
    args: &DriveArgs,
    entry: &RegisteredProject,
) -> Result<(), DriveError> {
    // Installed FIRST — before the group is established, before the lock, and
    // before a single byte lands on disk.
    //
    // **A driver that cannot observe the terminate signal is a driver that
    // cannot be stopped**, because layer 2 of D-06 is precisely this handler
    // calling `Executor::cancel`. Without it, the TUI's SIGTERM would reach the
    // driver's default disposition, the process would die *immediately*, and the
    // `claude` group — which is a different process group and never received
    // anything — would be orphaned along with its Bash grandchildren. That is
    // the exact failure CTRL-01 exists to prevent, so this refuses rather than
    // warning and continuing, and refusing here costs nothing: nothing has been
    // created yet, so there is no half-run to clean up.
    //
    // The refusal is `UnsupportedPlatform` rather than a new `DriveError`
    // variant. `tokio::signal::unix::signal` fails only for signals the kernel
    // does not let a process catch, so a build where this errors on SIGTERM is a
    // platform that cannot support driving at all — which is what that variant
    // already says (D-05). A variant of its own would widen an enum three later
    // plans in this phase also match on, to describe a branch no supported
    // platform reaches.
    let mut term = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
        .map_err(|err| DriveError::UnsupportedPlatform {
            detail: format!(
                "the terminate-signal handler could not be installed ({}), so a stop \
                 request could not tear the agent's process group down",
                err.kind()
            ),
        })?;

    let pgid = establish_own_group();

    let options = ExecutionOptions::default();

    // The TUI owns the id so it knows what to look for; the driver owns the
    // record (D-03).
    //
    // A **second** line of defence, not the tested one: `driver::drive` refuses
    // an absent run id before anything is created, and that refusal is what
    // `drive_refuses_a_real_run_that_carries_no_run_id_without_touching_disk`
    // exercises. This one exists so a future direct caller of `execute_run` —
    // there is none today — cannot reintroduce a run that `liveness::probe`
    // cannot see. The driver used to GENERATE an id here when none was supplied,
    // which produced a run whose id appeared on no argv: invisible to the probe,
    // reported crashed by every scan, and un-stoppable because a stop answered
    // already-gone without signalling (CR-04).
    let run_id = args.run_id.clone().ok_or(DriveError::RunIdRequired)?;

    // The executor's own generated argv is not reachable from here — the
    // builder is private to `src/executor/claude.rs` — so the digest covers the
    // driver's effective command line. That is enough for what the digest
    // promises: comparing two runs for "same command line". It authenticates
    // nothing (see `journal::argv_digest`).
    let program = args
        .claude_program
        .clone()
        .unwrap_or_else(|| PathBuf::from("claude"));
    let mut argv = vec![program.display().to_string()];
    argv.extend(
        args.claude_args
            .iter()
            .map(|arg| arg.to_string_lossy().into_owned()),
    );
    argv.push(args.command.clone());
    let argv_digest = journal::argv_digest(&argv);

    let record = make_run_record(run_id, args, entry, &options, argv_digest, pgid);

    let planning_dir = project.root().join(".planning");

    // The same observed group the record carries, and for the same reason
    // (WR-01): `setpgid(0, 0)` ran at entry, but its success is not something to
    // assume — the lock's holder record and `run.json` must name the same group,
    // or a stop resolved through one would refuse against the other. A second
    // `drive` against this project now refuses and names this run rather than
    // starting alongside it (CTRL-05).
    let lock = lock::acquire(&planning_dir, &record.run_id, pgid)?;

    let journal = JournalRun::start(&planning_dir, record).map_err(|err| DriveError::Journal {
        detail: format!("{err:#}"),
    })?;
    let mut run = DriverRun { journal, lock };

    // The only branch on the hidden development flags, and it lives here rather
    // than in `main` so the fixture never touches the production dispatch.
    let executor = match &args.claude_program {
        Some(program) => ClaudeExecutor::with_program(program, args.claude_args.clone()),
        None => ClaudeExecutor::new(),
    };

    let mut handle = match executor
        .start(&project, args.command.clone(), options)
        .await
    {
        Ok(handle) => handle,
        Err(err) => {
            // A run that started always has a terminal record, even when the
            // thing it was started for never launched (T-17-06).
            if let Err(journal_err) = run.journal.finish("spawn_failed") {
                tracing::warn!(
                    detail = %format!("{journal_err:#}"),
                    "could not close the journal after a failed spawn",
                );
            }
            return Err(DriveError::Spawn(err));
        }
    };

    run.journal.set_claude_pgid(handle.pgid);

    // One command means one message, so signal end-of-input immediately. EOF is
    // "no more input", not "stop": the CLI drains what is queued, finishes and
    // exits on its own. Without it a real `claude` would wait for a second turn
    // that this phase never sends.
    if let Err(err) = handle.close_input().await {
        tracing::warn!(kind = ?err, "could not signal end-of-input to the agent");
    }

    // `biased`, with the terminate arm FIRST, following `src/main_loop.rs:130`.
    //
    // Arm order is the decision, not a formality. Without `biased` the macro
    // picks a ready arm at random, and with a fast agent the event arm is
    // essentially always ready — so a stop request would lose the race for as
    // long as the stream kept producing, which is the entire duration of the run
    // the user is trying to stop. A stop that loses to a busy event queue is a
    // stop the user experiences as ignored (D-06.1).
    //
    // `tokio::select!` drops the other arms' futures before it runs the chosen
    // arm's body, which is what lets the terminate arm take `&mut handle` while
    // the event arm's future borrowed it.
    loop {
        tokio::select! {
            biased;

            _ = term.recv() => {
                shutdown_on_terminate(&executor, &mut handle, &mut run.journal).await;
                // Returning here drops `run`, and with it the `RunLock` — the
                // descriptor close IS the release (D-20.2). Nothing below this
                // point runs, so the terminal record written by the call above
                // is not followed by a second one.
                return Ok(());
            }

            event = handle.events.recv() => {
                match event {
                    Some(event) => {
                        if let Err(err) = run.journal.record_exec(&event) {
                            // The error KIND only. Never a message body, which
                            // could carry agent output (T-17-05).
                            tracing::warn!(kind = ?err.kind(), "journal write failed");
                        }
                    }
                    None => break,
                }
            }
        }
    }

    let outcome = handle.wait_outcome().await;
    run.journal
        .finish(outcome_label(&outcome))
        .map_err(|err| DriveError::Journal {
            detail: format!("{err:#}"),
        })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    // Aliased on import rather than called through its module path, and the
    // rename is load-bearing rather than stylistic. `tests/spawn_seam_guard.rs`
    // treats `process_group(` as a process-SPAWN marker, and its left word
    // boundary accepts a `::` — so an inline `liveness::process_group(…)` here
    // would report this module, which spawns nothing, as a spawn site. The
    // guard's own doc names the right fix for that class (`kill_process_group(`
    // is the same case) and names the wrong one: putting a file that spawns
    // nothing onto a spawn allowlist, which "quietly turns an audit into a list
    // of files somebody once had to add". An identifier character before the
    // marker is exactly what the boundary was built to accept.
    use crate::driver::liveness::process_group as kernel_process_group;

    /// A `DriveArgs` carrying nothing this module's own assertions vary.
    fn args() -> DriveArgs {
        DriveArgs {
            alias: "demo".to_string(),
            command: "/gsd-progress".to_string(),
            run_id: Some("2026-07-29T12-00-00Z-aaaa".to_string()),
            dry_run: false,
            goal: None,
            claude_program: None,
            claude_args: Vec::new(),
        }
    }

    /// A registry entry with no opt-in record; `make_run_record` only reads the
    /// opt-in timestamp, which is legitimately absent for a hand-typed run.
    fn entry() -> RegisteredProject {
        RegisteredProject {
            path: std::path::PathBuf::from("/nonexistent"),
            added: "2026-07-29T12:00:00Z".to_string(),
            driver_opt_in: None,
            extra: Default::default(),
        }
    }

    #[test]
    fn the_run_record_carries_the_group_it_was_given_and_not_a_second_copy_of_the_pid() {
        // A group this process cannot possibly be in, so an implementation that
        // ignored the argument and wrote `std::process::id()` twice — which is
        // what WR-01 describes — could not pass by coincidence.
        const SENTINEL_PGID: u32 = 4_242_424;

        let record = make_run_record(
            "2026-07-29T12-00-00Z-aaaa".to_string(),
            &args(),
            &entry(),
            &ExecutionOptions::default(),
            "fnv1a64:0000000000000000".to_string(),
            SENTINEL_PGID,
        );

        assert_eq!(
            record.pgid, SENTINEL_PGID,
            "the record must carry the group it was HANDED. Writing \
             std::process::id() here made the record assert a leadership a failed \
             setpgid means the process does not hold — and \
             kill::resolve_signal_target then refuses every stop against that run \
             (WR-01, D-04)"
        );
        assert_eq!(
            record.pid,
            std::process::id(),
            "the pid is still this process's own; the two fields are simply no \
             longer the same expression"
        );
    }

    #[test]
    fn the_current_group_agrees_with_the_proc_parse() {
        // Two independent sources of one fact: the `getpgrp` syscall and the
        // `/proc/<pid>/stat` field parse `kill::resolve_signal_target` compares
        // the record against. If either the syscall wrapper or the stat field
        // index were wrong, every stop in the product would refuse — and the two
        // would disagree here first.
        //
        // Deliberately NOT `establish_own_group()`: `setpgid` in a shared test
        // binary would move the harness's own process group, and with it every
        // other test in this process.
        let from_syscall = current_group();
        let from_proc = kernel_process_group(std::process::id())
            .expect("this process's own /proc/<pid>/stat is readable");

        assert_eq!(
            from_syscall, from_proc,
            "getpgrp() and the /proc pgrp field must name the same group. A \
             disagreement means one of the two is reading the wrong thing, and \
             the consequence is a kill switch that refuses every stop (D-04)"
        );
    }

    #[test]
    fn the_outcome_label_for_a_stopped_run_is_killed() {
        // One line, and it is what stops a later refactor silently relabelling a
        // stop as a failure. `src/driver/reconcile.rs` and the TUI both read the
        // `outcome` string off `run.json`, and a stop that arrives there as
        // `failed` tells the user their run broke when in fact they stopped it.
        assert_eq!(
            outcome_label(&RunOutcome::Killed { turns: Vec::new() }),
            "killed"
        );

        // The neighbour that would be reached if the terminate path ever stopped
        // going through `Executor::cancel` and started letting the run fall out
        // of its drain instead. Pinned so the two stay distinguishable.
        assert_ne!(
            outcome_label(&RunOutcome::Failed {
                reason: "x".to_string(),
                subtype: None,
                terminal_reason: None,
                exit_code: None,
            }),
            outcome_label(&RunOutcome::Killed { turns: Vec::new() })
        );
    }

    #[test]
    fn the_terminate_diagnostic_code_is_a_stable_grep_target() {
        // The code travels into `journal.jsonl` verbatim and is what a later
        // reader searches for, so it is snake_case and carries no timestamp, pid
        // or run id — those live in the record around it.
        assert_eq!(TERMINATE_DIAGNOSTIC_CODE, "terminate_signal_shutdown");
        assert!(TERMINATE_DIAGNOSTIC_CODE
            .chars()
            .all(|c| c.is_ascii_lowercase() || c == '_'));
    }
}
