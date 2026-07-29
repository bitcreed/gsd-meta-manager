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
//! **That honest-failure posture is narrowed here rather than abandoned, and the
//! narrowing is the point of plan 17-08's CR-05 fix.** `session_detector` claims
//! nothing: it offers a session id or it does not, and a caller that gets `None`
//! knows only that it learned nothing. This module's `false` was being consumed
//! as a *safety verdict* — a stop that answered "already finished; nothing was
//! signalled", and a reconciliation that answered "your run died" — on a platform
//! where the probe had simply not looked. A limitation that is named is a
//! limitation; a limitation that is silent is a lie about the agent's state. So
//! the reads still fail softly, and [`probe`] reports that failure as
//! [`Liveness::Unknown`] instead of laundering it into [`Liveness::Dead`] (D-05,
//! D-10).
//!
//! Nothing in this module writes. Every function is a read of `/proc` and
//! answers `None` or `false` when the read fails, which is what makes it safe to
//! call over every registered project on a timer.

/// Whether the `/proc` technique this module is built on applies to the platform
/// the binary is running on.
///
/// **A `const` value rather than a `#[cfg]` block, and that is the whole design
/// decision.** Every downstream refusal CR-05 introduced — the stop that must not
/// answer already-gone, the reconciliation that must not report a crash, the
/// start-side gate that must not launch what it cannot stop — branches on the
/// *non-Linux* answer, which is unreachable at runtime on the only platform CI
/// runs. Expressed as an attribute those branches could never be executed, let
/// alone tested; expressed as a value they are pure functions that a Linux test
/// can hand `false` to. `the_platform_gate_refuses_a_real_run_where_liveness_cannot_be_determined`
/// in `src/driver/mod.rs` is what that buys, and it is the difference between a
/// named limitation and a silent one (D-05).
///
/// It is `target_os = "linux"` and not `unix`: macOS and the BSDs are Unix and
/// have no `/proc/<pid>/cmdline` in this form, so `unix` here would claim a
/// capability the reads do not have.
pub const LIVENESS_SUPPORTED: bool = cfg!(target_os = "linux");

/// What the probe could establish about a recorded pid — **three answers, not
/// two**.
///
/// The third answer is not pedantry. A `bool` forces "I could not look" to be
/// spelled as one of "it is running" or "it is gone", and both spellings are
/// dangerous in a different direction: reported live, a finished run can never be
/// cleared; reported dead, a *running* autonomous agent is told to the user as
/// finished and a stop reports success without sending a signal. `Unknown` is the
/// value that lets each consumer decide for itself, and every consumer in this
/// codebase decides the same way — refuse, and say why (CR-05, D-05).
///
/// Derives match [`crate::driver::reconcile::RunVerdict`]'s because this value
/// travels inside `ObservedRun`, which travels as an
/// [`Action`](crate::action::Action) payload and must stay `Clone`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Liveness {
    /// The pid exists **and** its cmdline still names this run. The double-check
    /// passed; this is the only value a signal may be sent on.
    Alive,
    /// The `/proc` read succeeded and said this pid is not this run's driver —
    /// either no such process, or a recycled pid belonging to something else.
    Dead,
    /// The technique does not apply here, so nothing was established. **Never a
    /// synonym for [`Dead`](Self::Dead).**
    Unknown,
}

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

/// Whether `pid`'s argument vector still names this run's driver (D-10).
///
/// Three conditions, all required:
///
/// 1. `/proc/<pid>/cmdline` is readable with a non-empty first element. That is
///    sufficient evidence the process exists and needs no separate signal probe
///    — the read is exactly `session_detector`'s liveness idiom, where a failed
///    read drops the pid.
/// 2. `argv[0]` names `gsd-meta-manager`. A recycled pid belonging to some other
///    program fails here.
/// 3. The argument list carries the run id, in **either** of the two spellings
///    clap accepts.
///
/// **Condition 3 is what makes this safe to hand to a kill switch.** Without it
/// a stale record plus a recycled pid signals an unrelated process group, and
/// the failure is silent, rare, and catastrophic — the class of bug that only
/// reproduces on the machine that has been up for a month.
///
/// **Both spellings, and the second one is CR-04.** `clap` accepts a long option
/// as an adjacent pair (`--run-id 2026-…`) *and* inline (`--run-id=2026-…`), and
/// they are indistinguishable to the caller who typed them. Matching only the
/// pair made a driver started the inline way invisible to every consumer of this
/// module at once: the reconciliation scan reported a perfectly healthy run as
/// crashed, `stop_run` returned already-gone **before sending any signal** while
/// the agent ran on, and `admit` under-counted so the concurrency cap could be
/// exceeded. One missed spelling, three CTRL failures.
fn cmdline_names_run(pid: u32, run_id: &str) -> bool {
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
    let inline = format!("--run-id={run_id}");
    args.iter().any(|arg| arg.as_slice() == inline.as_bytes())
        || args
            .windows(2)
            .any(|window| window[0] == b"--run-id" && window[1] == wanted)
}

/// What can be established about the driver of `run_id` at `pid` — the
/// **safety-decision surface** of this module (D-05, D-10).
///
/// Returns [`Liveness::Unknown`] immediately where [`LIVENESS_SUPPORTED`] is
/// false, because on such a platform every read below fails for a reason that has
/// nothing to do with the process being asked about. Otherwise it is the pid **and**
/// cmdline double-check, answering [`Liveness::Alive`] or [`Liveness::Dead`].
///
/// **`Unknown` is not a synonym for dead, and conflating them is the defect this
/// function exists to fix.** `session_detector`'s honest-failure posture — a
/// failed `/proc` read yields nothing — is honest precisely because nothing is
/// claimed on top of it. Here the same `false` was being consumed as a verdict:
/// a stop returned `AlreadyGone`, whose rendering is *"already finished; nothing
/// was signalled"*, about a run that was still going and had never been
/// signalled. Every caller making a safety decision goes through this function;
/// [`is_run_alive`] remains only for polling a pid already known to be live.
///
/// Note this reports a **zombie as dead**: a zombie's `/proc/<pid>/cmdline` reads
/// as empty, so condition 1 of [`cmdline_names_run`] rejects it. See [`is_zombie`]
/// where the distinction has to be made explicitly.
pub fn probe(pid: u32, run_id: &str) -> Liveness {
    if !LIVENESS_SUPPORTED {
        return Liveness::Unknown;
    }
    if cmdline_names_run(pid, run_id) {
        Liveness::Alive
    } else {
        Liveness::Dead
    }
}

/// Whether `pid` is observably still the driver for `run_id`.
///
/// **Its `false` conflates "gone" with "could not look", so it must never be the
/// basis of a safety verdict.** It is kept because polling *is* a legitimate use:
/// a caller that has already established a pid was alive and is waiting for it to
/// disappear wants a `bool`, and on such a path `Unknown` and `Dead` are
/// genuinely the same instruction (stop waiting). Anything that decides whether
/// to send a signal, whether to report a crash, or whether a run may start goes
/// through [`probe`] instead (CR-05).
pub fn is_run_alive(pid: u32, run_id: &str) -> bool {
    matches!(probe(pid, run_id), Liveness::Alive)
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

/// The process **group** `/proc/<pid>/stat` reports for `pid`, or `None`.
///
/// Field 5, `pgrp`, reached exactly the way [`process_state`] reaches field 3:
/// past the **last** `)` and the space after it, then `split_whitespace`, which
/// yields state, ppid, pgrp in that order. The last-`)` form is required for the
/// same reason it is required there — the `comm` field is unescaped and may
/// itself contain spaces and parentheses, so counting tokens from the start of
/// the line shifts every later field for a process whose name happens to contain
/// one. `session_detector::read_start_time`'s *first*-`)` form is deliberately
/// **not** copied; it has been correct in practice only because `claude` has no
/// parenthesis in its name, and this module is pointed at process names it does
/// not choose. `tests/driver_kill.rs::group_members` already extracts the same
/// field the same way, and the two must agree field for field.
///
/// **What makes this load-bearing rather than a convenience:** `run.json` lives
/// inside the project the autonomous agent is being run in, with ordinary write
/// access, so its recorded `pgid` is agent-writable. This function is the
/// kernel's answer, and `kill::resolve_signal_target` refuses to signal anything
/// the two do not agree on. A wrong field index here would therefore not read as
/// a parse bug — it would read as *the record disagrees with the kernel*, and
/// every stop in the product would refuse (D-04, D-08).
pub fn process_group(pid: u32) -> Option<u32> {
    let stat = std::fs::read_to_string(format!("/proc/{}/stat", pid)).ok()?;
    let after_comm = stat.rfind(')')?.checked_add(2)?;
    let rest = stat.get(after_comm..)?;
    // state, ppid, pgrp — the same three the group scan in `tests/driver_kill.rs`
    // walks, in the same order.
    rest.split_whitespace().nth(2)?.parse::<u32>().ok()
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
///
/// **The platform consequence, named rather than left implicit (CR-05).** Where
/// [`process_state`] always yields `None` — every platform for which
/// [`LIVENESS_SUPPORTED`] is false — this answers `false` **vacuously**, so the
/// zombie half of ROADMAP criterion #1 would pass by measuring nothing at all.
/// Two things make that survivable rather than a hole. `driver::platform_refusal`
/// refuses to *start* a real run where liveness is undeterminable, and you cannot
/// fail to detect a zombie in a run that cannot exist. And where the criterion is
/// actually measured — `tests/driver_kill.rs` and `tests/driver_kill_startup.rs`
/// — the test asserts [`LIVENESS_SUPPORTED`] before it asserts anything else, so
/// a suite that could only pass vacuously fails loudly instead.
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
    fn process_group_reads_the_kernels_group_for_a_child_that_leads_its_own() {
        use std::os::unix::process::CommandExt;

        let mut child = std::process::Command::new("sleep")
            .arg("30")
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            // The same call `driver::spawn::spawn_detached` makes, so the child
            // this test measures has exactly the shape a real driver has.
            .process_group(0)
            .spawn()
            .expect("`sleep` is on PATH");
        let pid = child.id();

        let group = process_group(pid);

        let _ = child.kill();
        let _ = child.wait();

        assert_eq!(
            group,
            Some(pid),
            "a child spawned with process_group(0) LEADS its own group, so the \
             kernel's pgrp field must equal its pid. This is the fact `stop_run` \
             checks the agent-writable recorded pgid against (D-04)"
        );
    }

    #[test]
    fn process_group_does_not_confuse_an_inherited_group_with_a_leaders_own() {
        // The control arm, and it is what makes the field index PROVEN rather
        // than assumed: a parse that returned the `pid` field by mistake would
        // pass the test above and fail this one, because this child does not
        // lead a group at all.
        let (mut child, pid) = spawn_sleeper();

        let group = process_group(pid);
        let ours = process_group(std::process::id());

        let _ = child.kill();
        let _ = child.wait();

        assert_ne!(
            group,
            Some(pid),
            "a child spawned WITHOUT process_group(0) inherits this process's \
             group, so its pgrp must not be its own pid"
        );
        assert_eq!(
            group, ours,
            "the inherited group must be this test harness's own group; anything \
             else means the parse is reading a different field than it claims"
        );
    }

    #[test]
    fn the_inline_run_id_spelling_is_not_invisible_to_the_probe() {
        use std::os::unix::process::CommandExt;

        const RUN_ID: &str = "2026-07-29T12-00-00Z-inline";

        // A live process whose `/proc/<pid>/cmdline` looks like a driver started
        // the inline way, synthesised without building a binary: `arg0` supplies
        // the argv[0] the probe matches on, and the trailing arguments supply the
        // spelling under test.
        //
        // **The trailing `; :` is load-bearing.** `dash` exec-optimises
        // `sh -c '<single command>'` into a bare `exec`, which would replace the
        // process image — and with it the entire cmdline this test inspects. A
        // second command defeats the optimisation and keeps the shell in place.
        let mut child = std::process::Command::new("/bin/sh")
            .arg0("gsd-meta-manager")
            .args(["-c", "sleep 300; :", "x", &format!("--run-id={RUN_ID}")])
            .stdin(std::process::Stdio::null())
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("/bin/sh exists");
        let pid = child.id();

        // Give the shell a moment to exec, or the cmdline read races the spawn.
        std::thread::sleep(std::time::Duration::from_millis(200));

        let matched = probe(pid, RUN_ID);
        let control = probe(pid, "2026-07-29T12-00-00Z-other");

        let _ = child.kill();
        let _ = child.wait();

        assert_eq!(
            matched,
            Liveness::Alive,
            "clap accepts `--run-id=VALUE` as one argv element and `--run-id VALUE` \
             as two, and the user cannot tell them apart. Matching only the pair \
             made such a run invisible: the scan reported it crashed, a stop \
             answered already-gone WITHOUT signalling, and the concurrency cap \
             under-counted (CR-04)"
        );
        assert_eq!(
            control,
            Liveness::Dead,
            "the inline match must still be a match on THIS run id — a probe that \
             accepted any inline --run-id would have re-opened the pid-reuse hole \
             it was widened to close (D-10)"
        );
    }

    #[test]
    fn liveness_supported_names_the_platform_the_proc_technique_actually_needs() {
        assert_eq!(
            LIVENESS_SUPPORTED,
            cfg!(target_os = "linux"),
            "every downstream refusal — the stop that must not answer already-gone, \
             the reconciliation that must not report a crash, the start-side gate — \
             is derived from this CONSTANT and not from the attribute. If the two \
             drift, those refusals silently start describing a different platform \
             than the one the /proc reads work on (D-05)"
        );
    }

    #[test]
    fn probe_answers_dead_for_an_impossible_pid_only_where_proc_applies() {
        // Written as one expression rather than two `#[cfg]` arms so the test is
        // honest on BOTH platform arms; a `#[cfg]`-ed test checks only the arm it
        // was compiled for and silently asserts nothing about the other.
        let expected = if LIVENESS_SUPPORTED {
            Liveness::Dead
        } else {
            Liveness::Unknown
        };
        assert_eq!(
            probe(IMPOSSIBLE_PID, "any-run-id"),
            expected,
            "where /proc applies, an impossible pid is a positive DEAD verdict; \
             where it does not, nothing was established and the answer must be \
             Unknown. Reporting Unknown as Dead is what told a user their still- \
             running agent had already finished (CR-05)"
        );
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
