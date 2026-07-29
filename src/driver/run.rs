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

use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::time::Duration;

use rustix::process::Signal;
use tokio::sync::{mpsc, oneshot};

use crate::config::RegisteredProject;
use crate::driver::{kill, liveness, lock, DriveArgs};
use crate::error::DriveError;
use crate::executor::claude::ClaudeExecutor;
use crate::executor::stream_json::UserMessage;
use crate::executor::{
    DrivableProject, ExecutionEvent, ExecutionHandle, ExecutionOptions, Executor, RunOutcome,
};
use crate::journal::inbox::{self, InboxMessage};
use crate::journal::reader::TailCursor;
use crate::journal::{self, JournalEvent, JournalRun, RunRecord};

/// The diagnostic code the terminate-signal shutdown journals before its
/// terminal record.
///
/// A fixed identifier rather than a sentence, because it is what a later reader
/// greps for: a run that ends with the killed outcome could have been stopped
/// from the TUI, stopped from a shell, or stopped by a service manager, and this
/// record is the only thing that says a terminate signal is what arrived.
const TERMINATE_DIAGNOSTIC_CODE: &str = "terminate_signal_shutdown";

/// How long the agent's group is given between the terminate signal and the
/// uncatchable one **when the stop arrives during startup**.
///
/// **Five seconds, not the drain path's ten, and the difference is the state the
/// agent is in.** The drain path's ten seconds exist for a working agent: a turn
/// to abort, a Bash tree mid-command with its own signal handler, and a
/// `SessionEnd` hook chain that is allowed to finish. During startup the agent is
/// parked at — or before — its first `system/init`. There is no turn, no
/// grandchild mid-build, and no hook chain worth ten seconds of a user's stop.
///
/// The upper bound is not taste. The whole startup teardown has to fit inside
/// [`crate::driver::kill::DRIVER_TEARDOWN_GRACE`], the twelve seconds the TUI
/// gives the driver before it escalates to SIGKILL — and a driver SIGKILLed
/// mid-teardown orphans the agent group, which is the exact failure this path
/// exists to prevent. Five plus [`STARTUP_REAP_BOUND`] plus the journal's two is
/// nine, which fits with three to spare, and
/// `the_startup_stop_budget_fits_inside_the_driver_teardown_grace` asserts that
/// rather than this comment claiming it. The drain path does not fit and does not
/// need to: those twelve seconds were sized *for* it, by
/// `the_driver_grace_exceeds_the_claude_group_grace_plus_slack`.
const STARTUP_AGENT_GRACE: Duration = Duration::from_secs(5);

/// How long the uncatchable signal is given to take effect on the startup path.
///
/// SIGKILL cannot be caught, so this is a reap window rather than a grace, and it
/// mirrors `kill::KILL_REAP_BOUND` for the same reason: two seconds is enough to
/// observe a `/proc` entry disappear and short enough to leave room in the
/// budget above.
const STARTUP_REAP_BOUND: Duration = Duration::from_secs(2);

/// How often the agent group is re-probed while waiting out either bound above.
///
/// A signal is delivered asynchronously and a just-signalled process is briefly
/// still a pid, so the wait is "gone soon" rather than "gone now" — the same
/// reasoning, and the same tenth of a second, as `kill::DEATH_POLL_INTERVAL`.
const STARTUP_POLL_INTERVAL: Duration = Duration::from_millis(100);

/// How often the drain loop looks in this run's `inbox.jsonl` for a message the
/// user queued from the TUI (D-04, STEER-01).
///
/// **Polling, deliberately, rather than a `notify` watcher.** The driver is a
/// detached process that has no watcher today, and adding one to save half a
/// second is not a trade this phase needs to make: the unit of work here is
/// *minutes* — a GSD command running an agent turn — so a sub-second injection
/// latency buys nothing a user can perceive. A watcher would also put a second
/// event source and a second failure mode inside a process whose entire value is
/// being simple enough to survive its parent.
///
/// 750 ms is a defensible starting value with **no tuning data behind it** — it
/// is a named constant so tuning is a one-line change, following the
/// `main_loop.rs:41-49` idiom.
const INBOX_POLL_INTERVAL: Duration = Duration::from_millis(750);

/// The reason recorded on every message the run could not deliver (D-10).
///
/// A fixed sentence rather than one composed at the call site, so the four-state
/// display renders one string and a later reader greps for one thing.
const MISSED_AFTER_CLOSE: &str =
    "the agent's stdin was already closed when this message reached the driver";

/// The wire name of the marker that makes a `user` envelope a **replay echo**.
///
/// camelCase on the wire, and **absent rather than `false`** on an ordinary
/// message — so the test is "is this exactly `true`", never "is this not
/// `false`". The parsed model spells the same field `is_replay`
/// (`src/executor/stream_json.rs:113-122`) and
/// [`ClaudeExecutor::observing_replay_echoes`](crate::executor::claude::ClaudeExecutor::observing_replay_echoes)
/// filters on it before a line ever reaches this module; re-reading it here is
/// the second of two independent checks, and it is cheap.
const REPLAY_MARKER: &str = "isReplay";

/// Messages written to the agent's stdin that have not yet been echoed back.
///
/// A FIFO of `(id, text as sent)`, and both halves are load-bearing. The **id**
/// is what the journal record names, because text alone is not a correlation
/// key — a user may legitimately send the same sentence twice, and STEER-02's
/// states are per message rather than per string. The **text** is the only thing
/// the echo carries that can be matched against, because the CLI's replay echo
/// reproduces the body and mints its own `uuid`.
///
/// Nothing prunes this except a matched echo and the end of the run, and that is
/// correct: it holds at most one entry per message the user injected, which is a
/// number bounded by how fast a human types.
#[derive(Debug, Default)]
struct PendingAcks {
    entries: VecDeque<(String, String)>,
}

impl PendingAcks {
    /// Record that `text` was written to stdin under `id`.
    ///
    /// Called at exactly one moment: immediately after `Executor::send` returned
    /// `Ok`. A message whose write failed is never pushed, because it will never
    /// be echoed and would sit here shadowing a later identical message.
    fn push_delivered(&mut self, id: String, text: String) {
        self.entries.push_back((id, text));
    }

    /// The id of the message `echoed_text` acks, if any.
    fn match_echo(&mut self, echoed_text: &str) -> Option<String> {
        match_replay_echo(&mut self.entries, echoed_text)
    }

    /// Take every id still waiting for an echo when the run ends.
    ///
    /// **These messages were delivered and are not `missed`.** Each keeps its
    /// `interjected` record and simply never gains an `interjection_acted_on`,
    /// which is the honest answer: the run ended before the agent dequeued them,
    /// and fabricating the transition would assert an observation the driver
    /// never made. The ids are returned so the count can be logged; they are
    /// deliberately not journaled as anything.
    fn drain_undelivered(&mut self) -> Vec<String> {
        self.entries.drain(..).map(|(id, _)| id).collect()
    }
}

/// The id of the first pending message whose text is **exactly** `echoed_text`.
///
/// **`is_replay: true` is emitted at DEQUEUE, not at receipt.** Measured against
/// CLI 2.1.220: a message written at t=12s was echoed at t=68s, 45 ms after the
/// *previous* turn's `result` (D-07, Phase 15 D-31). The echo therefore means
/// *"the agent has started processing this"*, and the only correct word for the
/// state it establishes is **acted-on**. **"received", "read" and "acknowledged"
/// are forbidden renderings**: each promises an observation 55 seconds earlier
/// than the one the protocol actually supports, and the whole point of the
/// three-state display is that each state names evidence that exists.
///
/// Matching is **exact `String` equality on the UTF-8 text as sent** — no
/// trimming, no Unicode normalisation, no case folding. Anything looser would
/// let a message the user did not send ack one they did. The scan runs **front
/// to back and removes the first match**, so two legitimately identical messages
/// are acked in delivery order; matching the newest first would let the second
/// echo re-ack the first message and leave the second showing `delivered`
/// forever.
///
/// **Pure, and that is what makes it testable at all** — the register of
/// `driver/mod.rs:129-135`. Exercising this against a real agent would need a
/// process, a dequeue delay and two messages with the same body; fed a deque
/// directly it is four assertions.
///
/// **The declined alternative was correlating in the TUI** from
/// `ExecEvent.text`. That was rejected because `ExecEvent.text` is a *projection*
/// written for a human to read, and reconstructing protocol semantics from a
/// rendered string is exactly the screen-scraping D-01 forbids, wearing a
/// different hat. Only the driver has parsed envelopes, so only the driver can
/// answer this honestly (D-08).
fn match_replay_echo(pending: &mut VecDeque<(String, String)>, echoed_text: &str) -> Option<String> {
    let index = pending.iter().position(|(_, text)| text == echoed_text)?;
    pending.remove(index).map(|(id, _)| id)
}

/// The text an echoed `user` line carries, or `None` if it is not an echo.
///
/// The driver re-reads the `is_replay` marker off the raw line rather than
/// trusting the executor's filter alone, and then walks the body itself. Both
/// halves are deliberate: this is the party D-08 makes responsible for the
/// protocol reading, and a `user` line **without** the marker is a tool result —
/// not an echo of anything the driver sent, and never allowed to ack an injected
/// message.
///
/// Tolerant by construction, following `src/executor/stream_json.rs`'s whole
/// premise: a body shape no version we know emits yields `None` and a logged
/// non-event, never a parse failure and never a run-ending error. `content` is
/// accepted both as an array of blocks and as a bare string, because the CLI has
/// shipped both shapes; only `text` blocks contribute, and they are concatenated
/// in order so a multi-block echo of a single-block send still compares equal.
fn replay_echo_text(raw: &str) -> Option<String> {
    let value: serde_json::Value = serde_json::from_str(raw).ok()?;

    if value.get(REPLAY_MARKER).and_then(serde_json::Value::as_bool) != Some(true) {
        return None;
    }

    match value.get("message")?.get("content")? {
        serde_json::Value::String(text) => Some(text.clone()),
        serde_json::Value::Array(blocks) => {
            let mut text = String::new();
            for block in blocks {
                if block.get("type").and_then(serde_json::Value::as_str) == Some("text") {
                    if let Some(chunk) = block.get("text").and_then(serde_json::Value::as_str) {
                        text.push_str(chunk);
                    }
                }
            }
            Some(text)
        }
        _ => None,
    }
}

/// Turn one observed replay echo into an `interjection_acted_on` record.
///
/// An **unmatched** echo is logged by kind and otherwise ignored. That is the
/// right answer rather than a lenient one: the agent echoes every `user` message
/// it dequeues, including the run's own command prompt, and a hostile or merely
/// unexpected echo carrying text the driver never sent must not be able to
/// invent a transition. It is informational in exactly the way a later
/// `system/init` is (Phase 15 D-30), and it must never abort a run.
///
/// **No log line ever carries the echoed text.** It is either the user's own
/// words or the agent's, and both are message bodies (T-18-12, `PATTERNS` §S3).
fn correlate_replay_echo(pending: &mut PendingAcks, journal: &mut JournalRun, raw: &str) {
    let Some(text) = replay_echo_text(raw) else {
        tracing::debug!("a replay echo carried no readable text body");
        return;
    };

    let Some(id) = pending.match_echo(&text) else {
        tracing::debug!("a replay echo matched no message this driver delivered");
        return;
    };

    if let Err(err) = journal.record(&JournalEvent::InterjectionActedOn { id }) {
        tracing::warn!(kind = ?err.kind(), "journal write failed");
    }
}

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

/// Whether the agent process has left, counting a **zombie as gone**.
///
/// The `'Z'` arm is the load-bearing half. This driver is the agent's parent and
/// is about to exit without reaping it — the whole point of
/// [`shutdown_during_startup`] is that the process leaves immediately afterwards
/// — so a zombie here is an *exited* process waiting for init to adopt and reap
/// it. Treating it as still-running would burn the remaining grace waiting for
/// something that has already happened, and on the escalation path that wait is
/// subtracted directly from the budget the TUI is counting down.
fn agent_has_exited(pid: u32) -> bool {
    matches!(liveness::process_state(pid), None | Some('Z'))
}

/// Poll until `pid` has left, giving up after `limit`. Reports whether it did.
async fn agent_gone_within(pid: u32, limit: Duration) -> bool {
    let deadline = tokio::time::Instant::now() + limit;
    loop {
        if agent_has_exited(pid) {
            return true;
        }
        if tokio::time::Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(STARTUP_POLL_INTERVAL).await;
    }
}

/// The stop that arrives while the agent is still starting (CR-01).
///
/// **The window this closes, precisely.** `tokio::signal::unix::signal` replaces
/// SIGTERM's default disposition the moment it is constructed, at the very top of
/// [`execute_run`] — so from that instant the driver survives a terminate signal.
/// Until plan 17-08 nothing polled `term.recv()` until the drain loop two hundred
/// lines later, which meant a stop issued while `Executor::start` was in flight
/// was **swallowed**: the driver ignored the TUI's SIGTERM, the TUI escalated to
/// SIGKILL after its twelve-second grace, and the `claude` group — a *different*
/// process group that had never been signalled — was orphaned with its
/// grandchildren. The documented `SessionStart` hook hang
/// (`src/executor/mod.rs:225-239`) makes that window minutes rather than
/// microseconds.
///
/// **Why this tears the group down itself rather than trusting the Coordinator.**
/// Dropping the `start` future does drop `cancel_tx`, and the Coordinator's
/// cancellation arm does tear the agent group down. But that teardown is
/// unobservable from here — there is no handle, no outcome and no channel back —
/// and it dies with the tokio runtime the instant this process exits, which is
/// the next thing that happens. The driver must not return until it has *seen*
/// the group go, so it signals the group itself and waits.
///
/// It reaches the group through [`kill::signal_group`] and never through a direct
/// `rustix` call, so the "process group 0 is the caller's own group" refusal
/// exists in exactly one place in the tree (D-08).
async fn shutdown_during_startup(pgid_rx: &mut oneshot::Receiver<u32>, journal: &mut JournalRun) {
    // A non-blocking read, and it has to be: by the time this body runs
    // `tokio::select!` has already dropped the `start` future, so nothing further
    // will ever be sent on this channel and an `.await` here would hang until the
    // sender dropped. `Ok` means the agent child exists and its group is known;
    // any `Err` means the spawn had not reached `child.id()` yet, so there is no
    // agent and nothing to tear down.
    let agent_pgid = pgid_rx.try_recv().ok();

    if let Some(agent_pgid) = agent_pgid {
        if let Err(err) = kill::signal_group(agent_pgid, Signal::TERM) {
            // The error KIND only, never a message body (T-17-05).
            tracing::warn!(
                kind = ?err.kind(),
                "could not send the terminate signal to the agent process group during startup",
            );
        }

        if !agent_gone_within(agent_pgid, STARTUP_AGENT_GRACE).await {
            tracing::warn!(
                agent_pgid,
                "the agent group outlasted its startup grace; escalating to the \
                 uncatchable signal",
            );
            if let Err(err) = kill::signal_group(agent_pgid, Signal::KILL) {
                tracing::warn!(
                    kind = ?err.kind(),
                    "could not send the uncatchable signal to the agent process group",
                );
            }
            let _ = agent_gone_within(agent_pgid, STARTUP_REAP_BOUND).await;
        }
    }

    // The same diagnostic CODE the drain-loop path writes, deliberately: it is
    // what a later reader greps for, and a second code for "stopped, but earlier"
    // would split one question across two searches. The *detail* carries the
    // difference, including whether there was an agent group at all.
    if let Err(err) = journal.record(&JournalEvent::Diagnostic {
        code: TERMINATE_DIAGNOSTIC_CODE.to_string(),
        detail: match agent_pgid {
            Some(_) => "the driver received the terminate signal while the agent was still \
                        starting, and tore down the agent process group"
                .to_string(),
            None => "the driver received the terminate signal before the agent process \
                     existed, so there was no agent process group to tear down"
                .to_string(),
        },
    }) {
        tracing::warn!(
            kind = ?err.kind(),
            "could not journal the terminate-signal diagnostic",
        );
    }

    // Hard-coded, unlike `shutdown_on_terminate`, and the difference is not an
    // inconsistency. That path has an `ExecutionHandle` and therefore a
    // `RunOutcome` to derive a label from — including the rare case where a
    // wall-clock or idle breach was classified in the same pass. Here there is no
    // handle and no outcome: the run was stopped before one could exist, and
    // "killed" is the only truthful thing to write.
    if let Err(err) = journal.finish("killed") {
        tracing::warn!(
            detail = %format!("{err:#}"),
            "could not close the journal after a startup terminate-signal shutdown",
        );
    }
}

/// Read every complete inbox line after `cursor`, off the async worker.
///
/// **The read runs on `tokio::task::spawn_blocking` and never inline** (D-28,
/// WR-10). It is synchronous filesystem work, and this repository has *observed*
/// what a blocking syscall inside an `async fn` costs: `tests/driver_lock.rs`
/// records a blocking `flock` defeating `tokio::time::timeout` outright on a
/// current-thread runtime. A blocking read on the driver's one poll thread would
/// stall the terminate arm — the arm whose whole job is to be reachable.
///
/// An I/O failure is a warning and an empty batch, never a run-ending error: an
/// unreadable inbox costs the user their steering, and killing the run over it
/// would cost them the run as well. Every log line carries the error **kind**
/// only — never a path and never a message body, because a body is text the user
/// typed (T-18-03, PATTERNS §S3).
async fn read_inbox(inbox_path: &Path, cursor: &mut TailCursor) -> Vec<InboxMessage> {
    let path = inbox_path.to_path_buf();
    let start = *cursor;

    let read = match tokio::task::spawn_blocking(move || inbox::tail(&path, start)).await {
        Ok(Ok(read)) => read,
        Ok(Err(err)) => {
            tracing::warn!(kind = ?err.kind(), "inbox tail failed");
            return Vec::new();
        }
        Err(_) => {
            tracing::warn!("the inbox tail task did not run to completion");
            return Vec::new();
        }
    };

    *cursor = read.cursor;

    // Both diagnostic flags are surfaced rather than swallowed, following
    // `App::schedule_journal_tail`. Under this design's own invariants an inbox
    // is never truncated in place, so `restarted` can only mean an invariant
    // broke — and its consequence is re-delivery of messages already sent.
    if read.restarted {
        tracing::warn!(
            restarted = true,
            "inbox tail: the file shrank and the cursor was reset",
        );
    }
    if read.skipped_oversize {
        tracing::warn!(
            skipped_oversize = true,
            "inbox tail: stepped over a line that exceeded the read bound",
        );
    }
    if read.unparseable > 0 {
        // A count, never the line (D-28).
        tracing::warn!(
            count = read.unparseable,
            "inbox tail: complete lines did not parse as messages",
        );
    }

    read.messages
}

/// Deliver every queued message to the agent's stdin, journalling each attempt.
///
/// Returns how many were **delivered**, not how many were drained, and the
/// difference is load-bearing: the caller uses the answer to decide whether to
/// expect another turn. A message that was drained but whose write failed
/// produces no new turn, so counting it would park the run waiting for a
/// `result` that is never coming.
///
/// `delivered` on the journal record is exactly *"`Executor::send` returned
/// `Ok`"* and is never rendered as an acknowledgement from the agent (D-07);
/// that transition is `interjection_acted_on`, measured 55 seconds later.
///
/// **There is no turn-boundary flush buffer here, and there must never be one**
/// (D-02). ARCHITECTURE's AP3 — buffer a mid-turn message and flush it at the
/// `result` boundary — was *refuted* empirically against CLI 2.1.220 by Phase
/// 15's spike: a message written mid-turn is queued by the CLI and executed as
/// its own turn, so a driver-side buffer would duplicate the CLI's own queue and
/// make queued-message accounting incoherent.
/// `tests/executor_transport.rs::a_message_sent_mid_turn_is_not_buffered_by_the_driver`
/// is the regression guard that fails anyone who adds one.
async fn deliver_pending_inbox(
    executor: &ClaudeExecutor,
    handle: &mut ExecutionHandle,
    journal: &mut JournalRun,
    inbox_path: &Path,
    cursor: &mut TailCursor,
    pending: &mut PendingAcks,
) -> usize {
    let mut delivered_count = 0usize;

    for message in read_inbox(inbox_path, cursor).await {
        let delivered = match executor
            .send(handle, UserMessage::text(message.text.clone()))
            .await
        {
            Ok(()) => {
                delivered_count += 1;
                // The `{id → text}` D-08 requires, recorded at the only moment
                // it is true: the write returned `Ok`, so an echo of this text
                // can now legitimately arrive. Pushing before the write would
                // let a failed send shadow a later identical message.
                pending.push_delivered(message.id.clone(), message.text.clone());
                true
            }
            Err(err) => {
                // The error KIND only, never the message body (T-18-03).
                tracing::warn!(
                    kind = ?err,
                    "could not write an injected message to the agent's stdin",
                );
                false
            }
        };

        if let Err(err) = journal.record(&JournalEvent::Interjected {
            id: Some(message.id),
            text: message.text,
            delivered,
        }) {
            tracing::warn!(kind = ?err.kind(), "journal write failed");
        }
    }

    delivered_count
}

/// Journal every remaining inbox message as undeliverable (D-10).
///
/// **The honest fourth state.** Once the agent's stdin is closed it can never be
/// reopened, so a message that arrives afterwards has nowhere to go. Leaving it
/// in `queued` forever is PITFALLS' undelivered-injection failure dressed up as
/// a spinner, and it is the one thing the user's own steering intent must never
/// suffer: every message reaches a named terminal state the user can see.
///
/// It is reached from **two** places and classifies in **one**: the drain
/// loop's inbox poll once stdin has closed, and a final pass after the event
/// stream has ended. The two are the same question asked at two moments — "can
/// this still be delivered?" — and the answer is `no` from `close_input()`
/// onwards. Keeping the classification in [`journal_as_missed`] rather than at
/// each branch is what stops the two answers drifting apart.
///
/// It is deliberately **not** a retry: stdin cannot be reopened.
async fn sweep_inbox_as_missed(
    journal: &mut JournalRun,
    inbox_path: &Path,
    cursor: &mut TailCursor,
) -> usize {
    journal_as_missed(journal, read_inbox(inbox_path, cursor).await)
}

/// Write the terminal `missed` record for each of `messages`. **The only
/// emission site.**
///
/// One message, one record, no retry. Each message arrives here exactly once
/// because the cursor has already advanced past it, which is what makes
/// `interjection_missed` and `interjected` mutually exclusive for one id: a
/// message the drain loop delivered was consumed by [`deliver_pending_inbox`]
/// and can never be read a second time.
///
/// The `reason` is [`MISSED_AFTER_CLOSE`], a fixed machine-readable string; the
/// human-readable gloss belongs to the render layer, which must not have to
/// parse prose written here.
fn journal_as_missed(journal: &mut JournalRun, messages: Vec<InboxMessage>) -> usize {
    let count = messages.len();

    for message in messages {
        if let Err(err) = journal.record(&JournalEvent::InterjectionMissed {
            id: message.id,
            reason: MISSED_AFTER_CLOSE.to_string(),
        }) {
            tracing::warn!(kind = ?err.kind(), "journal write failed");
        }
    }

    count
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
/// **The terminate signal is raced against every await between the handler's
/// installation and step 7 — not only the drain loop.** This doc used to say the
/// signal was raced "first", which was true of the drain loop and false of
/// everything before it, and CR-01 is what that cost. The awaits in the window
/// were `lock::acquire`, `JournalRun::start`, `executor.start` and
/// `close_input`; the last two are long — `start` blocks on the capability gate,
/// which the documented hook hang can park for minutes — and each is now the
/// second arm of a `biased` `select!` whose first arm is `term.recv()`. The
/// synchronous steps in that window need no arm of their own, because tokio
/// **buffers** a signal delivered before the first `recv()`: whichever poll comes
/// first acts on it. `establish_own_group` and the two shorter awaits are covered
/// by that buffering, which is why the driver responds to a stop issued before it
/// has finished starting rather than after.
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

    // The agent's process group, published the instant the child exists rather
    // than only on the `ExecutionHandle` (CR-01). A stop that lands while `start`
    // is still awaiting the capability gate never receives a handle, so without
    // this channel it would have no way to reach the agent's group — which is a
    // *different* group from this driver's, and therefore the one that survives a
    // signal aimed here (D-06, D-09).
    let (pgid_tx, mut pgid_rx) = oneshot::channel::<u32>();

    // The raw wire line of every `user` replay echo, which is the **only**
    // evidence the protocol offers that the agent has started on an injected
    // message (D-07, D-08). Unbounded on purpose: this channel must never be
    // able to park the executor's coordinator, which owns the caps, the cancel
    // and the teardown. Its depth is bounded by the number of user messages one
    // run sends, and the loop below drains it on every pass.
    let (echo_tx, mut echo_rx) = mpsc::unbounded_channel::<String>();

    // The only branch on the hidden development flags, and it lives here rather
    // than in `main` so the fixture never touches the production dispatch.
    let executor = match &args.claude_program {
        Some(program) => ClaudeExecutor::with_program(program, args.claude_args.clone()),
        None => ClaudeExecutor::new(),
    }
    .observing_spawn(pgid_tx)
    .observing_replay_echoes(echo_tx);

    // `biased`, terminate arm FIRST — the same discipline as the drain loop
    // below, for a sharper reason. There the cost of losing the race is a
    // *delayed* stop; here it is a **swallowed** one. `Executor::start` blocks on
    // the capability gate, and the documented `SessionStart` hook hang parks it
    // for minutes, during which the driver would ignore the TUI's SIGTERM, get
    // SIGKILLed at the end of the twelve-second grace, and orphan the agent group
    // it never signalled. That is CR-01, and `tests/driver_kill_startup.rs` is
    // the three-process proof.
    //
    // Losing the race also drops the `start` future, which drops `cancel_tx` and
    // fires the Coordinator's own cancellation — but that teardown is
    // unobservable from here and dies with the runtime, which is why
    // `shutdown_during_startup` tears the group down explicitly instead of
    // relying on it.
    let started = tokio::select! {
        biased;

        _ = term.recv() => {
            shutdown_during_startup(&mut pgid_rx, &mut run.journal).await;
            // Returning drops `run` and with it the `RunLock` — the descriptor
            // close IS the release (D-20.2). The terminal record was written by
            // the call above, so nothing below runs and no second one follows.
            return Ok(());
        }

        result = executor.start(&project, args.command.clone(), options) => result,
    };

    let mut handle = match started {
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

    // The inbox this run is steered through, and the driver's own cursor into it
    // (D-03, D-04). The cursor lives here, in the run's own state, because the
    // inbox is per-run and nothing outside this loop consumes it.
    let inbox_path = run.journal.paths().inbox.clone();
    let mut inbox_cursor = TailCursor::default();

    // The first tick fires immediately, which is what makes a message queued
    // *before* the driver existed arrive without waiting out a full interval.
    // `Delay` rather than the default burst behaviour: a poll the loop was too
    // busy to service is worth doing once, not N times in a row.
    let mut inbox_poll = tokio::time::interval(INBOX_POLL_INTERVAL);
    inbox_poll.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Delay);

    // Whether the agent can still be written to. It starts `true`, and **that
    // is the change that makes steering physically possible** (D-11).
    let mut stdin_open = true;

    // Every message written to stdin that has not yet been echoed back, and the
    // state the acted-on transition is derived from (D-08).
    let mut pending_acks = PendingAcks::default();

    // Whether the echo channel can still produce. It cannot close while the run
    // is live — `executor` owns the sender and outlives this loop — so this flag
    // exists purely so a future refactor that *does* drop it early cannot turn a
    // closed channel into a permanently ready arm spinning the poll thread.
    let mut echo_open = true;

    // `biased`, with the terminate arm FIRST, following `src/main_loop.rs:130`.
    //
    // Arm order is the decision, not a formality. Without `biased` the macro
    // picks a ready arm at random, and with a fast agent the event arm is
    // essentially always ready — so a stop request would lose the race for as
    // long as the stream kept producing, which is the entire duration of the run
    // the user is trying to stop. A stop that loses to a busy event queue is a
    // stop the user experiences as ignored (D-06.1). The inbox poll goes **last**
    // for the mirror-image reason: it is the only arm whose work can wait, and
    // the only one that touches the filesystem. The replay-echo arm sits between
    // them — its work is a string compare against a deque, and the state it
    // records happened 55 seconds ago (D-07), so it is neither urgent nor
    // expensive.
    //
    // `tokio::select!` drops the other arms' futures before it runs the chosen
    // arm's body, which is what lets the terminate arm take `&mut handle` while
    // the event arm's future borrowed it.
    //
    // **When stdin closes, and why it is here rather than after the spawn**
    // (D-11). Until this plan, `close_input()` ran immediately after the spawn
    // with the comment *"one command means one message"* — correct for Phase 17
    // and fatal for Phase 18, because the writer task breaks its loop on
    // `Close` and every later `Executor::send` returns `WriterGone`. **While that
    // line stood, STEER-01/02/03 were not merely unimplemented but physically
    // impossible.** The rule that replaces it is four steps:
    //
    // 1. Do **not** close stdin after spawn.
    // 2. On each `ExecutionEvent::TurnCompleted` — a `result`, which closes a
    //    TURN and not the run (Phase 15 D-29) — drain the inbox one final time.
    // 3. If a message was delivered, the agent runs it as a new turn and the
    //    loop repeats from step 2. This supports N human-steered turns for free.
    // 4. If nothing was delivered, `close_input()`. EOF is "no more input", not
    //    "stop": the CLI drains what is queued, finishes, and **exits 0**.
    //
    // The final drain at step 2 is what resolves the common race in the user's
    // favour; anything arriving after the close is `missed`, named, and not
    // retried (D-10) — see the sweep below the loop. No new bound is needed:
    // `ExecutionOptions`' idle cap and wall-clock cap remain the backstop for an
    // agent that goes quiet with stdin open.
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
                        let turn_boundary = matches!(event, ExecutionEvent::TurnCompleted(_));

                        if let Err(err) = run.journal.record_exec(&event) {
                            // The error KIND only. Never a message body, which
                            // could carry agent output (T-17-05).
                            tracing::warn!(kind = ?err.kind(), "journal write failed");
                        }

                        if turn_boundary && stdin_open {
                            let delivered = deliver_pending_inbox(
                                &executor,
                                &mut handle,
                                &mut run.journal,
                                &inbox_path,
                                &mut inbox_cursor,
                                &mut pending_acks,
                            )
                            .await;

                            if delivered == 0 {
                                // Raced the same way and for the same reason as
                                // every other await in this file: by this point
                                // there **is** a handle, so a stop here takes
                                // the ordinary layer-2 path through
                                // `Executor::cancel` and no second teardown is
                                // written (D-06.2).
                                tokio::select! {
                                    biased;

                                    _ = term.recv() => {
                                        shutdown_on_terminate(
                                            &executor,
                                            &mut handle,
                                            &mut run.journal,
                                        )
                                        .await;
                                        return Ok(());
                                    }

                                    result = handle.close_input() => {
                                        if let Err(err) = result {
                                            tracing::warn!(
                                                kind = ?err,
                                                "could not signal end-of-input to the agent",
                                            );
                                        }
                                    }
                                }
                                stdin_open = false;
                            }
                        }
                    }
                    None => break,
                }
            }

            // The acted-on transition, and the only arm that produces it. It
            // sits after the event arm because an echo is never urgent — the
            // state it establishes happened 55 seconds ago (D-07) — and before
            // the inbox poll because it is the cheaper of the two: a string
            // compare against a deque, with no filesystem call at all.
            echo = echo_rx.recv(), if echo_open => {
                match echo {
                    Some(raw) => correlate_replay_echo(
                        &mut pending_acks,
                        &mut run.journal,
                        &raw,
                    ),
                    // Unreachable while `executor` is alive; see `echo_open`.
                    None => echo_open = false,
                }
            }

            _ = inbox_poll.tick() => {
                if stdin_open {
                    deliver_pending_inbox(
                        &executor,
                        &mut handle,
                        &mut run.journal,
                        &inbox_path,
                        &mut inbox_cursor,
                        &mut pending_acks,
                    )
                    .await;
                } else {
                    // **The arm keeps polling after the close, and that is the
                    // point** (D-10). Nothing read here can ever be delivered —
                    // stdin cannot be reopened — so each message is journaled
                    // `missed` the moment it is seen rather than at the end of
                    // the run. The difference is what the user watches: a
                    // message that reports its fate within a poll interval,
                    // versus one that sits in `queued` for however long the
                    // agent takes to finish, indistinguishable from a slow
                    // agent. That indistinguishability is PITFALLS' Pitfall 11
                    // wearing a spinner.
                    sweep_inbox_as_missed(
                        &mut run.journal,
                        &inbox_path,
                        &mut inbox_cursor,
                    )
                    .await;
                }
            }
        }
    }

    // The stream has ended, so every echo that will ever arrive has arrived.
    // Drained without awaiting: a `recv()` here would park until the executor's
    // sender dropped, which happens after this function returns.
    while let Ok(raw) = echo_rx.try_recv() {
        correlate_replay_echo(&mut pending_acks, &mut run.journal, &raw);
    }

    // Whatever is left was **delivered and never dequeued**. It keeps its
    // `interjected` record and gains no `interjection_acted_on`, because the
    // driver never observed one and inventing it would be the lie the whole
    // three-state display exists to prevent. A count only — never an id and
    // never a body — because this is the ordinary end of a run and not a fault.
    let unacked = pending_acks.drain_undelivered().len();
    if unacked > 0 {
        tracing::debug!(
            count = unacked,
            "the run ended before the agent dequeued every delivered message",
        );
    }

    // The stream has ended, so nothing further can be delivered. Anything still
    // in the inbox reaches its own named terminal state instead of sitting in
    // `queued` forever (D-10).
    sweep_inbox_as_missed(&mut run.journal, &inbox_path, &mut inbox_cursor).await;

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

    /// What the startup teardown still has to do after the agent group is gone:
    /// the diagnostic append, the terminal `run.json` write, and this process's
    /// own exit.
    ///
    /// A named budget rather than a fudge factor, following
    /// `tests/executor_lifecycle.rs:43-46`'s convention of mirroring a value
    /// where the reasoning about it happens.
    const JOURNAL_BUDGET: Duration = Duration::from_secs(2);

    #[test]
    fn the_startup_stop_budget_fits_inside_the_driver_teardown_grace() {
        let budget = STARTUP_AGENT_GRACE + STARTUP_REAP_BOUND + JOURNAL_BUDGET;
        assert!(
            budget <= crate::driver::kill::DRIVER_TEARDOWN_GRACE,
            "the whole startup teardown ({budget:?}) must fit inside the grace the \
             TUI gives the driver ({:?}). A breach is not a slow stop: the TUI \
             SIGKILLs the driver mid-teardown, which orphans the agent process \
             group and its grandchildren — the exact failure this path exists to \
             prevent, reintroduced by a number",
            crate::driver::kill::DRIVER_TEARDOWN_GRACE
        );

        // And the startup grace is deliberately SHORTER than the drain path's,
        // which is the claim the constant's doc makes. Pinned so a later edit
        // that "harmonises" the two has to face the budget above.
        assert!(
            STARTUP_AGENT_GRACE < Duration::from_secs(10),
            "the startup grace must stay below the drain path's ten seconds: \
             during startup there is no turn to abort, no Bash tree mid-command \
             and no SessionEnd chain, and the ten-second version does not fit the \
             budget asserted above"
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

    /// The other side of D-11's boundary, and the reason it is testable here
    /// rather than end to end.
    ///
    /// The *delivered* side is proved by `tests/driver_inbox.rs` against a real
    /// child process. The *missed* side cannot be reached that way without a
    /// sleep: it needs a message to land after the driver has closed stdin, and
    /// "after" in a live run is a race no assertion can pin. So the sweep is
    /// exercised directly against a real `JournalRun` and a real inbox file,
    /// which is the whole of what runs once the event stream has ended.
    ///
    /// The load-bearing assertion is that the ids come back. A sweep that
    /// journalled a *count* would satisfy "nothing is silently abandoned" in
    /// prose and leave the four-state display unable to say **which** message
    /// was lost, which is the only version of that answer a user can act on.
    #[tokio::test]
    async fn a_message_left_in_the_inbox_when_the_stream_ends_is_journaled_as_missed() {
        let dir = tempfile::tempdir().expect("temp dir");
        let planning = dir.path().join(".planning");

        let record = make_run_record(
            "2026-07-29T12-00-00Z-aaaa".to_string(),
            &args(),
            &entry(),
            &ExecutionOptions::default(),
            journal::argv_digest(&["claude".to_string()]),
            4242,
        );
        let mut journal = JournalRun::start(&planning, record).expect("start the run");
        let inbox_path = journal.paths().inbox.clone();

        let first = InboxMessage::new("this one was delivered");
        let second = InboxMessage::new("this one arrived too late");
        inbox::append(&inbox_path, &first).expect("append");

        // Consume the first message the way the drain loop would, so the cursor
        // sits exactly where the close left it.
        let mut cursor = TailCursor::default();
        assert_eq!(read_inbox(&inbox_path, &mut cursor).await.len(), 1);

        inbox::append(&inbox_path, &second).expect("append after the close");
        assert_eq!(
            sweep_inbox_as_missed(&mut journal, &inbox_path, &mut cursor).await,
            1,
            "only the message that arrived after the cursor may be swept"
        );
        journal.finish("succeeded_no_changes").expect("finish");

        let (records, _) =
            crate::journal::reader::read_all(&journal.paths().journal).expect("read");
        let missed: Vec<_> = records
            .iter()
            .filter(|record| record.kind == "interjection_missed")
            .collect();
        assert_eq!(missed.len(), 1, "one message, one terminal state");
        assert_eq!(
            missed[0].rest["id"], second.id,
            "the record must name WHICH message was lost, not merely that one was"
        );
        assert_eq!(missed[0].rest["reason"], MISSED_AFTER_CLOSE);
        assert!(
            !records
                .iter()
                .any(|record| record.kind == "interjection_missed"
                    && record.rest["id"] == first.id),
            "a message already past the cursor must never be re-reported as missed"
        );
    }

    #[test]
    fn the_inbox_poll_interval_is_sized_for_a_unit_of_work_measured_in_minutes() {
        // The bound in both directions is the decision, not the number. Too
        // short and a detached process burns syscalls forever for a latency no
        // human perceives; too long and the injection the user just typed feels
        // dropped. CONTEXT fixes the order at 500ms-1s and prefers polling to a
        // `notify` watcher outright.
        assert!(INBOX_POLL_INTERVAL >= Duration::from_millis(500));
        assert!(INBOX_POLL_INTERVAL <= Duration::from_secs(1));
    }

    // ========================================================================
    // The replay-echo correlator (D-07, D-08, STEER-02)
    //
    // Exhaustive here rather than end to end, and that is what the free
    // function bought: reaching the duplicate-text case against a real agent
    // would need a process, a dequeue delay and two messages with the same
    // body, and the assertion would still be about the order of two records.
    // ========================================================================

    /// A pending FIFO built from `(id, text)` literals.
    fn pending(entries: &[(&str, &str)]) -> VecDeque<(String, String)> {
        entries
            .iter()
            .map(|(id, text)| (id.to_string(), text.to_string()))
            .collect()
    }

    #[test]
    fn an_echo_against_an_empty_deque_matches_nothing() {
        let mut deque = pending(&[]);
        assert_eq!(match_replay_echo(&mut deque, "anything"), None);
        assert!(deque.is_empty());
    }

    #[test]
    fn an_echo_matches_the_pending_message_with_the_same_text() {
        let mut deque = pending(&[("id-1", "skip the UI review")]);
        assert_eq!(
            match_replay_echo(&mut deque, "skip the UI review"),
            Some("id-1".to_string())
        );
        assert!(
            deque.is_empty(),
            "a matched message must leave the queue, or its echo could ack it \
             twice"
        );
    }

    #[test]
    fn two_identical_texts_are_matched_in_delivery_order() {
        // The case the whole FIFO exists for. Two identical messages are
        // legitimate — a user may say "continue" twice — and matching the
        // NEWEST first would let the second echo re-ack the first message,
        // leaving the second showing `delivered` for the rest of the run.
        let mut deque = pending(&[("first", "continue"), ("second", "continue")]);

        assert_eq!(
            match_replay_echo(&mut deque, "continue"),
            Some("first".to_string()),
            "the FIRST delivered message is acked first"
        );
        assert_eq!(
            match_replay_echo(&mut deque, "continue"),
            Some("second".to_string()),
            "and the second echo acks the second message, not the first again"
        );
        assert!(deque.is_empty());
    }

    #[test]
    fn a_non_matching_echo_leaves_the_deque_untouched() {
        // The run's own command prompt is echoed exactly like an injected
        // message is, so this is the common case and not an edge one. An
        // implementation that popped the front on any echo would ack a message
        // the agent has not started, which is the failure D-07 names.
        let mut deque = pending(&[("id-1", "skip the UI review")]);

        assert_eq!(match_replay_echo(&mut deque, "/gsd-progress"), None);
        assert_eq!(
            deque.len(),
            1,
            "an unmatched echo must consume nothing: the message it did not \
             name is still waiting for its own"
        );

        // And the comparison is exact. Each of these differs from the stored
        // text only by something a lenient matcher would forgive, and each must
        // still miss (no trimming, no case folding, no normalisation).
        for near_miss in [
            " skip the UI review",
            "skip the UI review ",
            "Skip the UI review",
            "skip  the UI review",
        ] {
            assert_eq!(
                match_replay_echo(&mut deque, near_miss),
                None,
                "{near_miss:?} is not the text that was sent"
            );
        }
        assert_eq!(deque.len(), 1);
    }

    #[test]
    fn the_echo_text_comes_out_of_the_wire_body_only_when_the_replay_marker_is_true() {
        const TEXT: &str = "skip the UI review \u{1F680}";

        let echo = format!(
            r#"{{"type":"user","message":{{"role":"user","content":[{{"type":"text","text":{}}}]}},"isReplay":true,"uuid":"echo-1"}}"#,
            serde_json::to_string(TEXT).expect("the text encodes")
        );
        assert_eq!(
            replay_echo_text(&echo).as_deref(),
            Some(TEXT),
            "a multi-byte body must come back byte for byte, or exact equality \
             can never match what was sent"
        );

        // A `user` message WITHOUT the marker is a tool result. Acking an
        // injected message from one would report the agent as having started on
        // the user's steering when it was in fact reporting a Bash exit code.
        let tool_result = format!(
            r#"{{"type":"user","message":{{"role":"user","content":[{{"type":"text","text":{}}}]}}}}"#,
            serde_json::to_string(TEXT).expect("the text encodes")
        );
        assert_eq!(replay_echo_text(&tool_result), None);

        // Tolerant by construction: a shape no version we know emits yields
        // `None`, never a panic and never a parse failure that could end a run.
        assert_eq!(replay_echo_text("{not json"), None);
        assert_eq!(replay_echo_text(r#"{"isReplay":true}"#), None);
        assert_eq!(
            replay_echo_text(r#"{"isReplay":true,"message":{"content":"bare string"}}"#)
                .as_deref(),
            Some("bare string"),
            "`content` has shipped as a bare string as well as an array"
        );
    }

    #[test]
    fn a_delivered_message_awaiting_its_echo_is_never_reported_as_missed() {
        // The distinction Task 2's doc turns on, pinned as a test because the
        // two states are one word apart and mean opposite things: `missed` says
        // the message NEVER reached the agent, while an un-acked pending entry
        // says it reached the agent and the run ended before the agent got to
        // it. `drain_undelivered` therefore journals nothing at all.
        let mut acks = PendingAcks::default();
        acks.push_delivered("id-1".to_string(), "continue".to_string());
        acks.push_delivered("id-2".to_string(), "and again".to_string());

        assert_eq!(acks.match_echo("continue"), Some("id-1".to_string()));
        assert_eq!(
            acks.drain_undelivered(),
            vec!["id-2".to_string()],
            "only the message that never came back is left, and it is returned \
             for a COUNT — nothing here writes a journal record"
        );
    }

    /// D-11's idle interaction, asserted where the driver actually decides it.
    ///
    /// **The failure mode this guards is a run that hangs for fifteen minutes
    /// looking healthy.** Once stdin stays open for the life of a steerable run,
    /// an agent that goes quiet is held only by `ExecutionOptions`' idle cap —
    /// and the driver's terminal record is derived from the outcome that cap
    /// produces, through [`outcome_label`] and nothing else. So the property to
    /// pin is that the derivation reports the breach rather than laundering it
    /// into a success.
    ///
    /// The breach itself — an idle cap firing and producing
    /// [`RunOutcome::Stalled`] — is proved against a real silent child at
    /// `tests/executor_lifecycle.rs:435`. Reproducing that here would need a
    /// per-run idle-cap knob on `DriveArgs`, which is the new code path this
    /// test was asked *not* to add.
    #[test]
    fn a_run_idle_at_the_empty_inbox_step_is_reported_stalled_not_succeeded() {
        let stalled = outcome_label(&RunOutcome::Stalled {
            idle_for: Duration::from_secs(900),
        });

        assert_eq!(stalled, "stalled");
        for success in [
            outcome_label(&RunOutcome::SucceededWithChanges {
                turns: Vec::new(),
                total_cost_usd: None,
            }),
            outcome_label(&RunOutcome::SucceededNoChanges {
                turns: Vec::new(),
                total_cost_usd: None,
            }),
        ] {
            assert_ne!(
                stalled, success,
                "a run parked at the empty-inbox step for the idle cap must not \
                 reach the journal wearing a success label: `reconcile.rs` and \
                 the TUI both read this string, and a hang reported as a success \
                 is a hang nobody investigates"
            );
        }
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
