//! The process-group signal and the teardown grace the launcher uses to stop a
//! run (D-06, D-08).
//!
//! **The fact that makes the design necessary: there are TWO process groups,
//! not one.** The agent leads a group of its own, distinct from the driver's, so
//! a signal aimed at the driver's group does not reach it. A stop built from one
//! signal *looks* like it works; the tell is `pgrep -x claude` still returning
//! processes afterwards.
//!
//! What is left here since Phase 31 retired the legacy `drive` stop path
//! (`stop_run`, which waited on a detached child and re-probed `/proc`):
//!
//! - [`signal_group`], the single place the group-zero refusal lives;
//! - [`DRIVER_TEARDOWN_GRACE`], the SIGTERM-to-SIGKILL grace that
//!   `launcher::run::stop_group` waits out.
//!
//! Nothing here spawns a process and nothing here writes to disk.

use std::time::Duration;

use rustix::process::{Pid, Signal};

/// How long the launcher waits after the terminate signal before escalating to
/// the uncatchable one (D-06 step 4).
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
/// **`pub(crate)` rather than private, because the launcher's teardown reaches
/// it** (`launcher::run::stop_group`). Routing every group signal through here
/// is precisely so the group-zero refusal above lives in exactly one place: a
/// second `kill_process_group` call site elsewhere in the tree would be a second
/// chance to get the sign convention or the zero case wrong, and the second one
/// would have no test in front of it (D-08).
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

    /// How long a SIGKILL is given to take effect, mirrored from the legacy stop
    /// path that owned the figure (it was `KILL_REAP_BOUND` in the retired
    /// `stop_run`). SIGKILL cannot be caught, so this is a reap window rather
    /// than a grace. Kept as a test mirror so the escalation budget below still
    /// has its second term.
    const KILL_REAP_WINDOW: Duration = Duration::from_secs(2);

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
            DRIVER_TEARDOWN_GRACE + KILL_REAP_WINDOW <= CRITERION_VERIFICATION_POINT,
            "the grace plus the SIGKILL reap window ({:?}) must still fit inside the \
             verification point ({CRITERION_VERIFICATION_POINT:?}): the escalation is \
             part of the stop, not something that happens after it",
            DRIVER_TEARDOWN_GRACE + KILL_REAP_WINDOW
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

    /// The positive half of `signal_group`, retargeted from the retired
    /// `tests/driver_kill*.rs` (Phase 31, D-10 lock/kill exception): those drove
    /// a real `drive` process and proved a stop reaches a process GROUP. Only the
    /// refusal half survived in the unit tests above, so this pins the behaviour
    /// the launcher's teardown relies on: a signal aimed at a leader's pgid
    /// reaches a member of that group, not just the leader.
    #[test]
    fn a_signal_to_a_process_group_reaches_a_member_of_it() {
        use std::os::unix::process::{CommandExt, ExitStatusExt};

        // `sh` leads a fresh group of its own and forks `sleep` into it; the
        // trailing `; :` stops dash replacing the shell image with the sleep.
        let mut leader = std::process::Command::new("/bin/sh")
            .arg("-c")
            .arg("sleep 30 & wait; :")
            .process_group(0)
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("`sh` is on PATH");
        let pgid = leader.id();

        signal_group(pgid, Signal::TERM).expect("a live group accepts the signal");

        let status = leader.wait().expect("the group leader is reaped");
        assert_eq!(
            status.signal(),
            Some(Signal::TERM.as_raw()),
            "the leader must have died of the SIGTERM sent to its group, got {status:?}"
        );
    }
}
