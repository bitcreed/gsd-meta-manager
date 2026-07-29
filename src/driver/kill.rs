//! The TUI's half of the kill switch: signal the driver's group, wait, escalate,
//! confirm (D-06 steps 1 and 4, D-07, D-08).
//!
//! **The fact that makes this design necessary: there are TWO process groups,
//! not one.** Phase 15 spawns `claude` with `ProcessGroup::leader()`
//! (`src/executor/claude.rs:351`), so the agent leads a group whose id is
//! distinct from the driver's — and `kill(-driver_pgid, SIGTERM)` therefore does
//! **not** reach it. A stop built from this module alone *looks* like it works;
//! the tell is `pgrep -x claude` still returning processes afterwards, with the
//! agent's Bash grandchildren still holding files, ports and quota the user
//! cannot see.
//!
//! D-06's four steps, and who owns each:
//!
//! 1. **SIGTERM to the driver's process group** — this module, by the pgid the
//!    driver recorded on its own `run.json` and the reconciliation scan carries
//!    on [`ObservedRun`](crate::driver::reconcile::ObservedRun).
//! 2. **The driver's terminate handler calls `Executor::cancel`** — that is
//!    `src/driver/run.rs`, and layer 2 is a **call into Phase 15's existing
//!    SIGTERM → 10s grace → SIGKILL → unconditional `wait()` sequence**, never a
//!    second teardown.
//! 3. **The driver finishes its journal, releases its lock, exits** — also
//!    `src/driver/run.rs`. It is what makes a stopped run distinguishable on disk
//!    from a crashed one.
//! 4. **Grace, escalation to the uncatchable signal, and the reap** — this
//!    module again, through whichever of D-07's two arms applies.
//!
//! Nothing here spawns a process and nothing here writes to disk. It sends two
//! signals and reads `/proc`.

use std::time::Duration;

use rustix::process::{Pid, Signal};

use crate::driver::liveness::{self, Liveness};

/// How long the TUI waits after the terminate signal before escalating to the
/// uncatchable one (D-06 step 4).
///
/// **This value is wedged between two other numbers and both bounds are
/// asserted by unit tests rather than left as a comment**, because changing
/// either neighbour re-opens this one:
///
/// * **It must exceed the `claude` group's own grace** — ten seconds,
///   `src/executor/claude.rs:107` — **plus teardown slack.** The driver spends
///   that entire grace *inside* `Executor::cancel` before it can journal its
///   ending and exit, so escalating sooner would SIGKILL the driver mid-teardown
///   and orphan the very `claude` tree the stop exists to remove. That is the
///   failure the two-layer design was built to avoid, reintroduced by a number.
/// * **It must be strictly less than fifteen seconds**, which is where ROADMAP
///   success criterion #1 verifies — *"leaves no `claude` process, no grandchild
///   build or server process, and no zombie behind — verified 15 seconds
///   later"*. Even the escalation path has to have completed and been reaped
///   before that check, or the criterion is unverifiable by construction.
///
/// Twelve seconds satisfies both with two seconds of slack below and three above.
pub const DRIVER_TEARDOWN_GRACE: Duration = Duration::from_secs(12);

/// How often death is re-probed while waiting out a grace.
///
/// A signal is delivered asynchronously and a just-signalled process is briefly
/// still a pid, so liveness has to be "gone soon" rather than "gone now". A
/// tenth of a second is far below every bound here and costs one `/proc` read.
const DEATH_POLL_INTERVAL: Duration = Duration::from_millis(100);

/// How long the uncatchable signal is given to take effect before this returns.
///
/// SIGKILL cannot be caught, blocked or ignored, so this is a reap window rather
/// than a grace. It is short on purpose: it must fit inside the gap between
/// [`DRIVER_TEARDOWN_GRACE`] and criterion #1's fifteen seconds.
const KILL_REAP_BOUND: Duration = Duration::from_secs(2);

/// Which of D-07's two reaping arms applies to this run.
///
/// **Both arms exist because the TUI is only *sometimes* the parent**, and a
/// design with one arm fails success criterion #1 in exactly the case this phase
/// exists for.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReapArm {
    /// This TUI session spawned the driver, so it is the parent.
    ///
    /// The `wait()` that reaps it is owned by the detached task
    /// `src/driver/spawn.rs::spawn_detached` created for exactly that purpose,
    /// and **that task is the single `wait()` owner**. This module deliberately
    /// does not also wait: two waiters on one child is a race, and the loser
    /// gets `ECHILD` from a child that was reaped a microsecond earlier. So this
    /// arm confirms death the same way [`Adopted`](Self::Adopted) does — by
    /// re-probing `/proc` — and relies on that task for the reap itself. Saying
    /// so plainly is better than pretending to `wait()` from two places.
    Parent,
    /// The TUI restarted and rediscovered this run through the reconciliation
    /// scan, so the driver was reparented to init when its original parent
    /// exited.
    ///
    /// `wait()` from this process would return `ECHILD` — it is not our child —
    /// so liveness is confirmed **only** by re-probing `/proc/<pid>` until it
    /// disappears, and init performs the reap (D-07).
    Adopted,
}

impl ReapArm {
    /// Who performs the `wait()` that turns the exited driver into a reaped one.
    fn reaper(self) -> &'static str {
        match self {
            Self::Parent => "the spawn-side reaping task",
            Self::Adopted => "init",
        }
    }
}

/// How a stop ended.
///
/// A **state**, not a message: `src/error.rs`'s module header states the house
/// rule that a driver UI needs something it can render rather than a string it
/// must parse, and this is the surface the stop returns through an `Action`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StopOutcome {
    /// The driver was gone within [`DRIVER_TEARDOWN_GRACE`] of the terminate
    /// signal — the clean path, which means its own layer-2 teardown ran and the
    /// `claude` group went with it.
    ExitedOnTerminate,
    /// The driver ignored or outlasted the terminate signal and had to be sent
    /// the uncatchable one.
    ///
    /// **This is not a clean stop.** A SIGKILLed driver skips D-06 layer 2 by
    /// definition, so its `claude` group may survive it. The `claude` pgid
    /// journaled on `exec_started` (D-09) is what makes such an orphan traceable,
    /// and Phase 18's orphan sweep is what cleans it.
    ExitedAfterKill,
    /// There was nothing to stop: the pid was not this run's driver when the
    /// stop arrived. **No signal was sent.**
    AlreadyGone,
    /// The signal itself could not be delivered — no such process group, or no
    /// permission to signal it. Carries the error kind and nothing else.
    SignalFailed {
        /// The `std::io::ErrorKind`, rendered. Never a path and never a message
        /// body.
        detail: String,
    },
}

impl std::fmt::Display for StopOutcome {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::ExitedOnTerminate => write!(f, "stopped"),
            Self::ExitedAfterKill => write!(
                f,
                "stopped, but only after the uncatchable signal — the agent's process \
                 group may have been orphaned"
            ),
            Self::AlreadyGone => write!(f, "already finished; nothing was signalled"),
            Self::SignalFailed { detail } => {
                write!(f, "the stop signal could not be delivered: {detail}")
            }
        }
    }
}

/// Send `sig` to every process in the group led by `pgid`.
///
/// **Through `rustix`, never a shell-out to `/bin/kill` and never a hand-written
/// negative-pid call** (D-08). Both alternatives are ways to get the sign
/// convention wrong silently — `kill(pid, …)` and `kill(-pid, …)` differ by one
/// character and by the entire blast radius — and the repository already took
/// `rustix` as a direct dependency in plan 17-01 for exactly this call plus the
/// `flock` in plan 17-02.
///
/// **A pgid of zero, or one that does not fit a `pid_t`, is a hard refusal and
/// never a fall-through.** This is the single most dangerous mistake reachable
/// from this function: `kill(0, sig)` signals *the caller's own process group*,
/// which under a TUI is the user's terminal session — so a stop against a run
/// whose record carried a zero pgid would kill the TUI, its shell, and everything
/// else sharing that session, rather than the run. `Pid::from_raw` answers `None`
/// for zero, and that `None` returns an error here.
///
/// **`pub(crate)` rather than private, because the driver's own startup teardown
/// reaches it** — `src/driver/run.rs::shutdown_during_startup`, plan 17-08. That
/// path also has to signal a process group, and routing it through here is
/// precisely so the group-zero refusal above lives in exactly one place: a second
/// `kill_process_group` call site elsewhere in the tree would be a second chance
/// to get the sign convention or the zero case wrong, and the second one would
/// have no test in front of it (D-08).
pub(crate) fn signal_group(pgid: u32, sig: Signal) -> std::io::Result<()> {
    let raw = i32::try_from(pgid).map_err(|_| {
        std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "the recorded process group id does not fit a pid",
        )
    })?;

    let Some(group) = Pid::from_raw(raw) else {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "refusing to signal process group 0, which is the caller's own group",
        ));
    };

    rustix::process::kill_process_group(group, sig).map_err(std::io::Error::from)
}

/// What a liveness answer permits a stop to do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StopDecision {
    /// The pid is this run's driver. Signal it.
    Signal,
    /// The `/proc` read positively established the driver is not there.
    AlreadyGone,
    /// Nothing was established, so nothing may be concluded.
    Undeterminable,
}

/// Map a liveness answer onto what the stop is allowed to do about it (CR-05).
///
/// **This is a function rather than an inline `match` for one reason, and it is
/// worth the indirection:** [`Liveness::Unknown`] is unreachable at runtime on
/// Linux, which is the only platform CI runs, so the only way to test the
/// non-Linux answer at all is to hand it to a pure function. An inline match
/// inside [`stop_run`] would leave the branch that matters most — the one that
/// decides whether a user on an unsupported platform is told the truth — as the
/// single untested line in the kill switch.
///
/// The mapping that matters is the third arm. Answering
/// [`StopOutcome::AlreadyGone`] on `Unknown` renders as *"already finished;
/// nothing was signalled"* — a true second half and a fabricated first half,
/// about an autonomous agent that is still running with git and push rights.
fn stop_decision(liveness: Liveness) -> StopDecision {
    match liveness {
        Liveness::Alive => StopDecision::Signal,
        Liveness::Dead => StopDecision::AlreadyGone,
        Liveness::Unknown => StopDecision::Undeterminable,
    }
}

/// Which process group this stop may signal, given the record and the kernel
/// (CR-02, D-04, D-08).
///
/// **The threat, in one sentence:** `run.json` lives inside the project the
/// autonomous agent is being driven in, with ordinary write access, so an
/// unvalidated recorded `pgid` lets the agent being stopped choose which process
/// group the user's TUI sends SIGTERM and then SIGKILL to.
///
/// `Ok` requires the kernel to confirm the recorded group for the pid whose
/// liveness was just checked; anything else — a disagreement, or a `/proc` read
/// that yielded nothing — is an `Err` and **no signal is sent at all**. Refusing
/// is strictly safer than guessing: a refusal costs the user a visible error and
/// a second attempt, while a guess costs somebody else's process group.
///
/// **The `Ok` arm returns the kernel's value, not the recorded one, even though
/// the two are equal by construction there.** That is the point rather than a
/// pedantry: the value that leaves this function came from `/proc`, so a future
/// edit that loosens the equality check cannot silently start signalling the
/// agent-writable number.
fn resolve_signal_target(recorded_pgid: u32, kernel_pgid: Option<u32>) -> Result<u32, String> {
    match kernel_pgid {
        Some(kernel) if kernel == recorded_pgid => Ok(kernel),
        Some(kernel) => Err(format!(
            "the recorded process group ({recorded_pgid}) disagrees with the group \
             the kernel reports for this pid ({kernel}), so nothing was signalled. \
             D-04's invariant is that the recorded group is ESTABLISHED, not \
             assumed, and run.json is writable by the agent being driven"
        )),
        None => Err(format!(
            "the process group for this pid could not be read from /proc, so the \
             recorded group ({recorded_pgid}) could not be confirmed and nothing \
             was signalled (D-04)"
        )),
    }
}

/// Poll until this run's driver is gone, giving up after `limit`.
///
/// The probe is [`liveness::is_run_alive`], which is a pid **and** cmdline
/// double-check — so a pid recycled by an unrelated process while this loop runs
/// reads as gone rather than keeping the caller waiting on a stranger.
async fn gone_within(pid: u32, run_id: &str, limit: Duration) -> bool {
    let deadline = tokio::time::Instant::now() + limit;
    loop {
        if !liveness::is_run_alive(pid, run_id) {
            return true;
        }
        if tokio::time::Instant::now() >= deadline {
            return false;
        }
        tokio::time::sleep(DEATH_POLL_INTERVAL).await;
    }
}

/// Stop the run led by `pgid`, and confirm it is gone (D-06 steps 1 and 4).
///
/// In order, and the order is the decision:
///
/// 1. **Liveness first, before any signal**, and through [`liveness::probe`]
///    rather than the `bool` convenience. If `pid` is positively not this run's
///    driver, return [`StopOutcome::AlreadyGone`] having signalled nothing; if
///    liveness could not be *determined*, return
///    [`StopOutcome::SignalFailed`] — see [`stop_decision`] for why those two
///    must not collapse. This is the pid-reuse guard applied at the moment it
///    matters most: the kernel recycles pids, and signalling a *recycled* pid's
///    **group** is how a stop comes to kill a stranger's processes — silently,
///    rarely, and only on a host that has been up long enough (D-10).
/// 2. **Resolve the target from the kernel**, through [`resolve_signal_target`].
///    The recorded `pgid` arrives from a file inside the driven project, which
///    the agent can write; the group actually signalled is the one `/proc`
///    reports, and a disagreement sends nothing at all (CR-02, D-04).
/// 3. **SIGTERM to that group**, through [`signal_group`].
/// 4. **Wait out [`DRIVER_TEARDOWN_GRACE`]**, re-probing `/proc`. Both of D-07's
///    arms poll the same way and differ in who reaps — see [`ReapArm`].
/// 5. **Escalate** to the uncatchable signal if the grace expired, with a
///    `tracing::warn!`, because an escalation means the driver did not shut down
///    cleanly and its `claude` tree may now be orphaned.
///
/// **Never call this on the render thread.** The grace is twelve seconds; the
/// caller dispatches it on a task and takes the result back as an `Action`
/// (TRANS-03).
pub async fn stop_run(pid: u32, pgid: u32, run_id: &str, arm: ReapArm) -> StopOutcome {
    match stop_decision(liveness::probe(pid, run_id)) {
        StopDecision::AlreadyGone => return StopOutcome::AlreadyGone,
        StopDecision::Undeterminable => {
            return StopOutcome::SignalFailed {
                detail: "liveness cannot be determined on this platform".to_string(),
            }
        }
        StopDecision::Signal => {}
    }

    // The signal target is the kernel's answer, never the record's. Resolved
    // once and reused by both the terminate and the escalation below, so the two
    // cannot drift onto different groups.
    let target = match resolve_signal_target(pgid, liveness::process_group(pid)) {
        Ok(target) => target,
        Err(detail) => return StopOutcome::SignalFailed { detail },
    };

    if let Err(err) = signal_group(target, Signal::TERM) {
        return StopOutcome::SignalFailed {
            detail: format!("{}", err.kind()),
        };
    }

    if gone_within(pid, run_id, DRIVER_TEARDOWN_GRACE).await {
        return StopOutcome::ExitedOnTerminate;
    }

    // The signalled group and the arm, and nothing else — no goal string, no run
    // id body. The group logged is the resolved one, because that is the group
    // that was actually signalled and therefore the one an operator needs.
    tracing::warn!(
        pgid = target,
        reaped_by = arm.reaper(),
        "the driver outlasted its terminate grace; escalating to the uncatchable \
         signal. Its agent process group may now be orphaned",
    );

    if let Err(err) = signal_group(target, Signal::KILL) {
        return StopOutcome::SignalFailed {
            detail: format!("{}", err.kind()),
        };
    }

    // The result is deliberately not branched on: SIGKILL cannot be caught, so a
    // pid still present after this bound is one whose reap has not been observed
    // yet, not one that survived. Reporting it as anything other than
    // `ExitedAfterKill` would invent a state.
    let _ = gone_within(pid, run_id, KILL_REAP_BOUND).await;
    StopOutcome::ExitedAfterKill
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The `claude` process group's own SIGTERM→SIGKILL grace, as an
    /// **observable** copy of a constant that is private to the executor
    /// (`src/executor/claude.rs:107`).
    ///
    /// The convention is `tests/executor_lifecycle.rs:43-46`'s: a private
    /// constant whose value a caller must reason about is mirrored where the
    /// reasoning happens, and the mirror is what the bound is asserted against.
    /// If the executor's value moves,
    /// `the_driver_grace_exceeds_the_claude_group_grace_plus_slack` is where the
    /// consequence surfaces.
    const CLAUDE_GROUP_GRACE: Duration = Duration::from_secs(10);

    /// The point at which ROADMAP success criterion #1 verifies, verbatim from
    /// the criterion: *"verified 15 seconds later"*.
    const CRITERION_VERIFICATION_POINT: Duration = Duration::from_secs(15);

    #[test]
    fn the_driver_grace_exceeds_the_claude_group_grace_plus_slack() {
        assert!(
            DRIVER_TEARDOWN_GRACE > CLAUDE_GROUP_GRACE,
            "the driver's grace ({DRIVER_TEARDOWN_GRACE:?}) must exceed the claude \
             group's own grace ({CLAUDE_GROUP_GRACE:?}): the driver spends that whole \
             grace inside Executor::cancel before it can journal and exit, so \
             escalating sooner SIGKILLs the driver mid-teardown and ORPHANS the very \
             claude tree the stop exists to remove (D-06)"
        );
        assert!(
            DRIVER_TEARDOWN_GRACE >= CLAUDE_GROUP_GRACE + Duration::from_secs(2),
            "the margin above the claude group's grace must be slack for the journal \
             finish and the process exit, not a rounding difference: {DRIVER_TEARDOWN_GRACE:?} \
             vs {CLAUDE_GROUP_GRACE:?}"
        );
    }

    #[test]
    fn the_driver_grace_is_strictly_below_the_fifteen_second_verification_point() {
        assert!(
            DRIVER_TEARDOWN_GRACE < CRITERION_VERIFICATION_POINT,
            "the grace ({DRIVER_TEARDOWN_GRACE:?}) must be STRICTLY below the point at \
             which criterion #1 verifies ({CRITERION_VERIFICATION_POINT:?}), or even the \
             escalation path has not finished when the check runs and the criterion is \
             unverifiable by construction"
        );
        assert!(
            DRIVER_TEARDOWN_GRACE + KILL_REAP_BOUND <= CRITERION_VERIFICATION_POINT,
            "the grace plus the SIGKILL reap window ({:?}) must still fit inside the \
             verification point ({CRITERION_VERIFICATION_POINT:?}): the escalation is \
             part of the stop, not something that happens after it",
            DRIVER_TEARDOWN_GRACE + KILL_REAP_BOUND
        );
    }

    #[tokio::test]
    async fn stopping_a_pid_that_is_already_gone_reports_already_gone() {
        // A REAL pid that has been reaped, not a number chosen to be implausible:
        // the case this guards is a stop arriving a moment after the run ended,
        // which is a race a user hits by pressing stop as a run completes.
        let mut child = std::process::Command::new("sleep")
            .arg("30")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("`sleep` is on PATH");
        let pid = child.id();
        let _ = child.kill();
        let _ = child.wait();

        let outcome = stop_run(pid, pid, "2026-07-29T12-00-00Z-aaaa", ReapArm::Adopted).await;

        assert_eq!(
            outcome,
            StopOutcome::AlreadyGone,
            "a pid that is no longer this run's driver must be reported gone WITHOUT \
             a signal being sent. Signalling a recycled pid's whole process group is \
             how a stop kills a stranger's processes (D-10)"
        );
    }

    #[test]
    fn a_zero_process_group_is_refused_rather_than_signalled() {
        // The single most dangerous mistake reachable from this module: process
        // group 0 means the CALLER's own group, so under a TUI this would
        // terminate the user's whole terminal session instead of the run.
        let err = signal_group(0, Signal::TERM)
            .expect_err("process group 0 must never be signalled (T-17-37)");
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);

        // And a value that cannot be a pid at all, which is the other way the
        // conversion could quietly produce a wrong group.
        let err = signal_group(u32::MAX, Signal::TERM)
            .expect_err("a pgid that does not fit a pid must be refused");
        assert_eq!(err.kind(), std::io::ErrorKind::InvalidInput);
    }

    #[test]
    fn an_undeterminable_liveness_never_answers_already_gone() {
        assert_eq!(stop_decision(Liveness::Alive), StopDecision::Signal);
        assert_eq!(stop_decision(Liveness::Dead), StopDecision::AlreadyGone);
        assert_eq!(
            stop_decision(Liveness::Unknown),
            StopDecision::Undeterminable,
            "an undeterminable liveness must NOT map onto already-gone. That \
             mapping is what told a macOS user 'already finished; nothing was \
             signalled' about a run that was still going and had never been \
             signalled — a true second half attached to a fabricated first half \
             (CR-05, D-05)"
        );
    }

    #[test]
    fn a_recorded_group_that_disagrees_with_the_kernel_is_refused_without_a_signal() {
        // Pure, so the three arms are checked without a process: the point is
        // the DECISION, and the end-to-end proof that no signal follows a refusal
        // is the decoy test below.
        let disagreement = resolve_signal_target(4242, Some(99))
            .expect_err("a recorded group the kernel does not confirm must be refused");
        assert!(
            disagreement.contains("D-04"),
            "the refusal must name the invariant it is enforcing, or the next \
             reader deletes it as over-caution, got: {disagreement}"
        );

        let unreadable = resolve_signal_target(4242, None)
            .expect_err("an unreadable kernel group is not a licence to trust the record");
        assert!(unreadable.contains("D-04"), "got: {unreadable}");

        assert_eq!(
            resolve_signal_target(4242, Some(4242)),
            Ok(4242),
            "agreement is the ordinary case and must be transparent — every real \
             driver records the group it actually leads (D-04)"
        );
    }

    #[tokio::test]
    async fn a_stop_whose_record_names_another_groups_pgid_signals_nothing_and_the_decoy_survives() {
        use std::os::unix::process::CommandExt;

        const RUN_ID: &str = "2026-07-29T12-00-00Z-decoy";

        // A stand-in that satisfies the liveness double-check without building a
        // binary: `arg0` supplies the argv[0] the probe matches, and the run id
        // is passed as the adjacent pair. The trailing `; :` defeats dash's
        // exec-optimisation of `sh -c '<single command>'`, which would replace
        // the process image and with it the cmdline being inspected.
        let mut stand_in = std::process::Command::new("/bin/sh")
            .arg0("gsd-meta-manager")
            .args(["-c", "sleep 300; :", "x", "--run-id", RUN_ID])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("/bin/sh exists");
        let stand_in_pid = stand_in.id();

        // An unrelated process leading a group of its OWN — the group a
        // tampered-with `run.json` would name. Nothing connects it to the run.
        let mut decoy = std::process::Command::new("sleep")
            .arg("300")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .process_group(0)
            .spawn()
            .expect("`sleep` is on PATH");
        let decoy_pid = decoy.id();

        tokio::time::sleep(Duration::from_millis(200)).await;

        // The non-vacuity precondition. Without it a stand-in that failed to come
        // up would take the already-gone path and this test would "prove" a
        // refusal it never reached.
        let precondition = liveness::probe(stand_in_pid, RUN_ID);

        // A record that names the decoy's group beside the stand-in's pid —
        // exactly the shape an agent gets by editing the `pgid` field of the
        // `run.json` sitting in the project it is being driven in.
        let outcome = stop_run(stand_in_pid, decoy_pid, RUN_ID, ReapArm::Adopted).await;

        // Read the decoy's survival BEFORE the teardown, then reap both children
        // so a failure does not also leak two processes.
        let decoy_present = liveness::process_state(decoy_pid).is_some();
        let _ = stand_in.kill();
        let _ = stand_in.wait();
        let _ = decoy.kill();
        let _ = decoy.wait();

        assert_eq!(
            precondition,
            Liveness::Alive,
            "precondition: the stand-in must read as this run's live driver before \
             the stop, or the refusal below is not the branch being exercised"
        );
        assert!(
            matches!(outcome, StopOutcome::SignalFailed { .. }),
            "a stop whose recorded group disagrees with the kernel must refuse, \
             got {outcome:?}"
        );
        assert!(
            decoy_present,
            "the decoy in the unrelated process group {decoy_pid} was SIGNALLED. \
             run.json is writable by the agent being driven, so an unvalidated \
             recorded pgid lets that agent choose which process group the user's \
             TUI terminates (CR-02, T-17-08-01)"
        );
    }

    #[test]
    fn both_reaping_arms_name_who_performs_the_wait() {
        // D-07's distinction, pinned: the parent case has a `wait()` owner in
        // this process and the adopted case does not, and conflating them is
        // what leaves a zombie in one direction and races a reap in the other.
        assert_ne!(ReapArm::Parent.reaper(), ReapArm::Adopted.reaper());
        assert_eq!(ReapArm::Adopted.reaper(), "init");
    }
}
