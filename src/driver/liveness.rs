//! Is the process a run recorded still *that* run's process? (D-10)
//!
//! **Liveness here is a pid AND cmdline double-check, and the second half is not
//! optional.** A recorded pid outlives the process that owned it: the kernel
//! recycles pids, and on a busy long-lived host it recycles them within hours.
//! A bare existence check against `RunRecord.pid` therefore answers "some
//! process exists" and not "my driver exists" — and in this phase the difference
//! is the difference between reporting a run live and sending SIGTERM to a
//! stranger's process group. Requiring the pid's `/proc/<pid>/cmdline` to carry
//! both `gsd-meta-manager` and the matching `--run-id` is the standard defence,
//! and it is the same shape `session_detector::read_session_id` already uses to
//! recover a `--resume` argument.
//!
//! **This module is Linux-specific and deliberately not `cfg`-gated.** It is a
//! second consumer of `src/session_detector.rs`'s technique, and that module
//! carries no platform attributes at all: its `/proc` reads simply fail off
//! Linux and the functions yield `None`/`false`. Matching that honest failure
//! mode keeps the behaviour of the copy identical to the behaviour of the
//! original, and it keeps the whole reconciliation path compiling and testable
//! on every platform — a Unix attribute here would gate a module that performs
//! no platform call, only file reads.
//!
//! Nothing in this module writes. Every function is a read of `/proc` and
//! answers `None` or `false` when the read fails, which is what makes it safe to
//! call over every registered project on a timer.

/// Every NUL-separated element of `/proc/<pid>/cmdline`, as raw bytes.
///
/// Bytes rather than `String`: an argument vector is not required to be UTF-8,
/// and a lossy conversion at this level would let a mangled argument compare
/// equal to a well-formed one. Callers convert only the elements they mean to
/// show or compare as text.
///
/// This is `session_detector::read_session_id`'s first two lines generalised by
/// one step, so both callers split the same way.
fn cmdline_args(pid: u32) -> Option<Vec<Vec<u8>>> {
    let cmdline = std::fs::read(format!("/proc/{}/cmdline", pid)).ok()?;
    Some(cmdline.split(|&b| b == 0).map(|arg| arg.to_vec()).collect())
}

/// Whether `pid` is still the driver process for `run_id` (D-10).
///
/// Three conditions, all required:
///
/// 1. `/proc/<pid>/cmdline` is readable with a non-empty first element. That is
///    sufficient evidence the process exists and needs no separate signal probe
///    — the read is exactly `session_detector`'s liveness idiom, where a failed
///    read drops the pid.
/// 2. `argv[0]` names `gsd-meta-manager`. A recycled pid belonging to some other
///    program fails here.
/// 3. The argument list carries `--run-id` immediately followed by `run_id`. A
///    recycled pid belonging to *another driver* fails here, and so does a
///    driver for a different run on the same project.
///
/// **Condition 3 is what makes this safe to hand to a kill switch.** Without it
/// a stale record plus a recycled pid signals an unrelated process group, and
/// the failure is silent, rare, and catastrophic — the class of bug that only
/// reproduces on the machine that has been up for a month.
///
/// Note this reports a **zombie as alive**: a zombie's `/proc/<pid>/cmdline`
/// reads as empty, so condition 1 already rejects it. See [`is_zombie`] for the
/// case where the distinction has to be made explicitly.
pub fn is_run_alive(pid: u32, run_id: &str) -> bool {
    let Some(args) = cmdline_args(pid) else {
        return false;
    };
    let Some(argv0) = args.first() else {
        return false;
    };
    if argv0.is_empty() {
        return false;
    }
    if !String::from_utf8_lossy(argv0).contains("gsd-meta-manager") {
        return false;
    }

    let wanted = run_id.as_bytes();
    args.windows(2)
        .any(|window| window[0] == b"--run-id" && window[1] == wanted)
}

/// The process state character from `/proc/<pid>/stat`, or `None`.
///
/// **The `comm` field is the hazard here and a naive `split_whitespace` over the
/// whole line is the classic bug.** Field 2 is the executable name in
/// parentheses, it is *not* escaped, and it may itself contain spaces and
/// parentheses — a process called `my (weird) name` shifts every later field by
/// two if you count tokens from the start of the line. The parse therefore finds
/// the **last** `)` in the line, skips it and the space after it, and only then
/// splits: everything after the final `)` is guaranteed to be the fixed-width
/// numeric tail.
///
/// This differs by one character from `session_detector::read_start_time`, which
/// uses the *first* `)`. That call site has been correct in practice because
/// `claude` has no parenthesis in its name; the last-match form is correct for
/// any name, and this module is called against process names it does not choose.
pub fn process_state(pid: u32) -> Option<char> {
    let stat = std::fs::read_to_string(format!("/proc/{}/stat", pid)).ok()?;
    let after_comm = stat.rfind(')')?.checked_add(2)?;
    let rest = stat.get(after_comm..)?;
    rest.split_whitespace().next()?.chars().next()
}

/// Whether `pid` is a **zombie** — exited, but not yet reaped by its parent.
///
/// This exists because the repository's existing liveness helper cannot answer
/// the question. `tests/executor_lifecycle.rs`'s `alive()` uses the zero signal
/// (`kill -0`), which succeeds for a zombie: a zombie still owns a pid and still
/// has a process table entry, so `kill -0` reports it **alive**. ROADMAP success
/// criterion #1 requires that stopping a run leaves *"no zombie behind — verified
/// 15s later"*, and that half of the criterion is therefore untestable with
/// `alive()`. Plan 17-06's kill test is the consumer.
///
/// Do not extend or reuse `alive()` for this. The two helpers answer two
/// different questions and collapsing them would make the criterion pass on an
/// implementation that leaves zombies.
pub fn is_zombie(pid: u32) -> bool {
    process_state(pid) == Some('Z')
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A pid no process can plausibly own. Linux's default `pid_max` is 4194304,
    /// well below this.
    const IMPOSSIBLE_PID: u32 = 999_999_999;

    /// Spawn a real, long-lived child and return it with its pid.
    fn spawn_sleeper() -> (std::process::Child, u32) {
        let child = std::process::Command::new("sleep")
            .arg("30")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("`sleep` is on PATH");
        let pid = child.id();
        (child, pid)
    }

    #[test]
    fn a_live_pid_whose_cmdline_lacks_the_run_id_is_reported_dead() {
        // The pid-reuse defence, and the single most load-bearing test in this
        // module: it spawns a REAL process, takes its REAL live pid, and asserts
        // the probe says dead. Every other test here passes on an implementation
        // that only checks existence; this one does not.
        let (mut child, pid) = spawn_sleeper();

        let verdict = is_run_alive(pid, "2026-07-29T12-00-00Z-aaaa");

        // Reap before asserting so a failure does not also leak a process.
        let _ = child.kill();
        let _ = child.wait();

        assert!(
            !verdict,
            "a live pid belonging to an unrelated process must be reported DEAD. \
             A bare existence check passes this only by luck, and in production it \
             would eventually point a SIGTERM at a stranger's process group (D-10)"
        );
    }

    #[test]
    fn a_pid_that_does_not_exist_is_reported_dead() {
        assert!(!is_run_alive(IMPOSSIBLE_PID, "any-run-id"));
        assert!(process_state(IMPOSSIBLE_PID).is_none());
        assert!(!is_zombie(IMPOSSIBLE_PID));
    }

    #[test]
    fn the_process_state_of_a_running_child_is_not_the_zombie_state() {
        let (mut child, pid) = spawn_sleeper();

        let state = process_state(pid);
        let zombie = is_zombie(pid);

        let _ = child.kill();
        let _ = child.wait();

        // `S` (interruptible sleep) is what a `sleep 30` should report, but the
        // assertion is the weaker, non-flaky one: whatever it is, it is not `Z`.
        // Pinning `S` would make this test a scheduler observation.
        assert!(
            state.is_some(),
            "a live pid must have a readable /proc/<pid>/stat"
        );
        assert_ne!(state, Some('Z'), "a running process is not a zombie");
        assert!(!zombie, "is_zombie must agree with process_state");
    }

    #[test]
    fn this_process_reports_its_own_state_as_running_or_sleeping() {
        // A positive control for the comm-field parse: if the parse were wrong
        // it would return a digit or `None` rather than a state letter, and the
        // three tests above would all still pass.
        let state = process_state(std::process::id()).expect("our own stat is readable");
        assert!(
            state.is_ascii_alphabetic(),
            "the parsed field must be the state LETTER, not a number from further \
             along the line, got {state:?}"
        );
    }
}
