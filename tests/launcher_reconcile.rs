#![cfg(target_os = "linux")]
// ============================================================================
// The relaunch reconcile rule and the mid-dispatch pause, through the REAL
// `driver launch` with the fake claude / bwrap / systemd shims (Phase 29, plan
// 29-05; PRMT-04, D-17, D-18, D-19 deterministic layer).
//
// The model-facing half of SC4 (does a real Haiku driver follow the prompt?) is
// plan 29-08's paid run; this file proves for free that the MECHANISM holds: a
// scripted driver that dispatched I-01 is SIGKILLed mid-dispatch, the relaunch
// refuses every `dispatched` until exactly one `recovered` for I-01 is logged,
// and a `control: pause` saved while a worker runs is honoured before the next
// dispatch.
//
// D-18 wording difference, flagged for the verifier. SC4 says the run "ends after
// the current item"; PRMT-04 says the control edit is honoured "before the next
// dispatch". This file pins the STRICTER PRMT-04 reading `[operator-confirmed
// 2026-10-19]`: the in-flight worker's report is still recorded, then the next
// driver-level dispatch is denied and the run ends `reason=pause`. The two
// readings agree on every scripted path here (the pause lands while I-01 runs).
// ============================================================================

#[path = "launcher_support/mod.rs"]
mod launcher_support;

use std::fs;
use std::process::{Child, Output};
use std::time::Duration;

use launcher_support::{wait_until, Fixture};
use rustix::process::{kill_process_group, Pid, Signal};

/// A lint-clean DIRECTION body: every item carries intent, done-when, a dispatch
/// budget, `after: none` and `hold: false`, so the launch preflight of plan 29-06
/// accepts it. Written over the fixture's own body before each launch.
const RECONCILE_BODY: &str = "# Driver direction: reconcile test

## Direction

- control: run
- goal: Exercise the relaunch reconcile rule.

## Items

### I-01 First slice

- intent: Phase 1 is planned, executed and verified.
- done-when: Phase 1 has a passing VERIFICATION.md.
- budget: dispatches=3
- after: none
- hold: false

### I-02 Second slice

- intent: Phase 2 is planned, executed and verified.
- done-when: Phase 2 has a passing VERIFICATION.md.
- budget: dispatches=3
- after: none
- hold: false
";

/// Launch 1 of the crash tests: start, dispatch I-01 (the gate allows it), signal
/// readiness and hang until the test kills it.
const CRASH_SCENARIO: &str = r#"
D="$FAKE_CLAUDE_DIR"
driverctl log start --mode headless --loop-mode loop
driverctl log dispatched --item I-01 --step "/gsd-plan-phase 1" --round 0 --phase 1
printf '%s' '{"hook_event_name":"PreToolUse","tool_name":"Agent","session_id":"s1","tool_input":{"prompt":"run phase 1"}}' | driverctl hook > "$D/crash_hook.json"
touch "$D/started"
while :; do sleep 1; done
"#;

/// The relaunch driver for one recovery `action`. It records every driverctl exit
/// code in `$FAKE_CLAUDE_DIR/rc_*` and the first refusal text in `dispatch_1.err`.
fn relaunch_scenario(action: &str) -> String {
    let followup = match action {
        "done" => {
            r#"driverctl log verified --item I-01 --done-when met --evidence "VERIFICATION.md passed"
driverctl log status --item I-01 --state done --outcome "finished before the crash"
"#
        }
        "reset" => {
            r#"driverctl log status --item I-01 --state pending --outcome "reset after the crash"
"#
        }
        "park" => {
            r#"driverctl log needs-human --item I-01 --question "the worker died; is the plan still wanted?"
driverctl log status --item I-01 --state parked --rule needs-human --outcome "parked after the crash"
"#
        }
        // resume: the same step is dispatched again, below.
        _ => "",
    };
    let redispatch = if action == "resume" {
        r#"driverctl log dispatched --item I-01 --step "/gsd-plan-phase 1" --round 0 --phase 1
echo $? > "$D/rc_dispatch_2"
"#
    } else {
        ""
    };
    format!(
        r#"
D="$FAKE_CLAUDE_DIR"
driverctl log start --mode headless --loop-mode loop
driverctl log dispatched --item I-01 --step "/gsd-plan-phase 1" --round 0 --phase 1 2> "$D/dispatch_1.err"
echo $? > "$D/rc_dispatch_1"
driverctl log recovered --item I-01 --action {action} --reason "worker died" 2> "$D/recover_1.err"
echo $? > "$D/rc_recover_1"
driverctl log recovered --item I-01 --action {action} --reason "again" 2> "$D/recover_2.err"
echo $? > "$D/rc_recover_2"
{followup}driverctl log reconcile --found 1
{redispatch}driverctl log end --reason stop
printf '%s\n' '{{"type":"result","subtype":"success"}}'
"#
    )
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// Wait for a spawned launcher, failing the test if it hangs.
fn finish(mut child: Child, within: Duration) -> Output {
    assert!(
        wait_until(within, || child.try_wait().expect("try_wait").is_some()),
        "the launcher did not finish within {within:?}"
    );
    child
        .wait_with_output()
        .expect("collect the launcher output")
}

/// A fixture whose DIRECTION.md is the lint-clean [`RECONCILE_BODY`].
fn fixture() -> Fixture {
    let fx = Fixture::new();
    fs::write(fx.run_dir.join("DIRECTION.md"), RECONCILE_BODY).expect("DIRECTION.md");
    fx
}

/// Whether `pid` belongs to this fixture: its environment carries this fixture's
/// `FAKE_SHIM_LOG`. A pid file can outlive its process; a recycled pid must never
/// be signalled (T-29-19).
fn is_ours(fx: &Fixture, pid: u32) -> bool {
    let marker = format!("FAKE_SHIM_LOG={}", fx.shim_log_path().display());
    fs::read(format!("/proc/{pid}/environ")).is_ok_and(|raw| {
        raw.split(|byte| *byte == 0)
            .any(|entry| entry == marker.as_bytes())
    })
}

/// SIGKILL only the fake driver's process group, leaving the launcher alive.
fn sigkill_the_driver(fx: &Fixture) {
    let pids = fx.driver_pids();
    assert!(!pids.is_empty(), "no driver pid file was recorded");
    for pid in pids {
        if !is_ours(fx, pid) {
            continue;
        }
        if let Some(pid) = i32::try_from(pid).ok().and_then(Pid::from_raw) {
            let _ = kill_process_group(pid, Signal::KILL);
        }
    }
}

/// Launch 1: the driver dispatches I-01, then is SIGKILLed. With `with_launcher`
/// the launcher process dies with it (the Phase 28 L5 path); otherwise the
/// launcher survives, records the crash and ends the launch.
fn crash_first_launch(fx: &Fixture, with_launcher: bool) {
    fx.write_scenario(CRASH_SCENARIO);
    let mut launcher = fx.spawn_launch(&[]);
    assert!(
        wait_until(Duration::from_secs(10), || fx.scenario_started()),
        "the launcher never reached its driver:\n{}",
        fx.log()
    );
    if with_launcher {
        launcher.kill().expect("SIGKILL the launcher");
        launcher.wait().expect("reap the launcher");
        // The fake driver is not tied to its parent; the real one is (die-with-parent).
        fx.kill_fixture_processes();
    } else {
        sigkill_the_driver(fx);
        let out = finish(launcher, Duration::from_secs(20));
        assert!(out.status.success(), "{}", stderr(&out));
    }
    assert!(
        wait_until(Duration::from_secs(10), || !fx.any_driver_alive()),
        "the killed driver is still alive"
    );
    assert_eq!(
        fx.state_json()["items"]["I-01"]["state"],
        "running",
        "the crash must leave I-01 running:\n{}",
        fx.log()
    );
}

/// Relaunch with the relaunch driver for `action` and return the launch output.
fn relaunch(fx: &Fixture, action: &str) -> Output {
    fx.write_scenario(&relaunch_scenario(action));
    let child = fx.spawn_launch(&[]);
    finish(child, Duration::from_secs(30))
}

/// The exit code a scenario recorded under `name`.
fn rc(fx: &Fixture, name: &str) -> String {
    fx.recorded(name).trim().to_owned()
}

/// The part of LOG.md from launch 2's launch line on.
fn launch_two(log: &str) -> &str {
    log.split_once("RUN | launch | n=2")
        .unwrap_or_else(|| panic!("no second launch line:\n{log}"))
        .1
}

fn count(haystack: &str, needle: &str) -> usize {
    haystack.matches(needle).count()
}

/// The D-19 property for `action`: refused dispatch, exactly one accepted
/// `recovered`, a refused second one, and the LOG holds exactly one recovered line
/// for I-01 after `RUN | launch | n=2` and before launch 2's first dispatch.
fn assert_recovered_exactly_once(fx: &Fixture, action: &str, expected_state: &str) {
    let log = fx.log();
    assert_eq!(
        rc(fx, "rc_dispatch_1"),
        "2",
        "dispatch over a crashed worker:\n{log}"
    );
    let refusal = fx.recorded("dispatch_1.err");
    assert!(
        refusal.contains("reconcile first: I-01"),
        "wrong refusal: {refusal}"
    );
    assert_eq!(
        rc(fx, "rc_recover_1"),
        "0",
        "{}",
        fx.recorded("recover_1.err")
    );
    assert_eq!(
        rc(fx, "rc_recover_2"),
        "2",
        "second recovered must be refused:\n{log}"
    );
    assert!(
        fx.recorded("recover_2.err")
            .contains("already recovered in this launch: I-01"),
        "wrong second refusal: {}",
        fx.recorded("recover_2.err")
    );

    assert_eq!(
        count(&log, "| I-01 | recovered |"),
        1,
        "exactly one recovered line:\n{log}"
    );
    let second = launch_two(&log);
    assert_eq!(count(second, "| I-01 | recovered |"), 1, "{log}");
    let recovered_at = second.find("| I-01 | recovered |").expect("recovered line");
    if let Some(dispatched_at) = second.find("| I-01 | dispatched |") {
        assert!(
            recovered_at < dispatched_at,
            "no dispatch before the recovery in launch 2:\n{log}"
        );
    }
    assert!(
        second.contains(&format!("action={action}")) || second.contains(action),
        "the recovered line names the action {action}:\n{log}"
    );

    if action == "resume" {
        assert_eq!(rc(fx, "rc_dispatch_2"), "0", "{log}");
        assert_eq!(count(second, "| I-01 | dispatched |"), 1, "{log}");
    } else {
        assert_eq!(count(second, "| I-01 | dispatched |"), 0, "{log}");
    }
    assert_eq!(
        fx.state_json()["items"]["I-01"]["state"],
        expected_state,
        "follow-up item state for {action}:\n{log}"
    );
}

fn killed_dispatch_recovered_exactly_once(action: &str, expected_state: &str) {
    let fx = fixture();
    crash_first_launch(&fx, false);
    let out = relaunch(&fx, action);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_recovered_exactly_once(&fx, action, expected_state);
}

#[test]
fn a_killed_dispatch_is_recovered_exactly_once_before_the_next_dispatch() {
    killed_dispatch_recovered_exactly_once("resume", "running");
}

#[test]
fn a_killed_dispatch_is_recovered_exactly_once_before_the_next_dispatch_done() {
    killed_dispatch_recovered_exactly_once("done", "done");
}

#[test]
fn a_killed_dispatch_is_recovered_exactly_once_before_the_next_dispatch_reset() {
    killed_dispatch_recovered_exactly_once("reset", "pending");
}

#[test]
fn a_killed_dispatch_is_recovered_exactly_once_before_the_next_dispatch_park() {
    killed_dispatch_recovered_exactly_once("park", "parked");
}

#[test]
fn a_launcher_and_driver_crash_is_reconciled_on_relaunch() {
    let fx = fixture();
    crash_first_launch(&fx, true);
    let out = relaunch(&fx, "resume");
    assert!(out.status.success(), "{}", stderr(&out));

    let log = fx.log();
    let crash = log
        .find("RUN | crash-detected | previous_launch=1")
        .unwrap_or_else(|| panic!("no crash-detected line:\n{log}"));
    let relaunch_at = log
        .find("RUN | launch | n=2")
        .unwrap_or_else(|| panic!("no second launch line:\n{log}"));
    assert!(
        crash < relaunch_at,
        "crash-detected precedes the launch:\n{log}"
    );
    assert_recovered_exactly_once(&fx, "resume", "running");
}
