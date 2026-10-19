#![cfg(target_os = "linux")]
// ============================================================================
// `gsd-meta-manager driver stop` through the REAL binary (Phase 28, plan 28-02;
// LNCH-01 stop half, D-09). The fake `systemd-run` / `bwrap` / `systemctl` /
// `claude` of `tests/launcher_support` stand in for the host tools; the live
// proof that a real scope loses every descendant is `tests/launcher_live_scope.rs`.
// ============================================================================

#[path = "launcher_support/mod.rs"]
mod launcher_support;

use std::fs;
use std::time::Duration;

use gsd_meta_manager::driver::lock::{probe_at, run_lock_path, LockProbe};
use launcher_support::{wait_until, Fixture, RUN_ID, WAITING_SCENARIO};

fn stdout(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

#[test]
fn stop_signals_the_launcher_which_records_stop_user() {
    let fx = Fixture::new();
    // The driver never ends the run itself: only the stop can.
    fx.write_scenario(
        "driverctl log start --mode headless --loop-mode loop\n\
         touch \"$FAKE_CLAUDE_DIR/started\"\n\
         sleep 60\n",
    );
    let launcher = fx.spawn_launch(&[]);
    assert!(
        wait_until(Duration::from_secs(10), || fx.scenario_started()),
        "the launcher never reached its driver"
    );

    let stop = fx
        .stop_command(RUN_ID, &[])
        .output()
        .expect("spawn driver stop");
    assert!(stop.status.success(), "{}", stderr(&stop));
    assert!(
        stdout(&stop).contains("stopped the launcher"),
        "{}",
        stdout(&stop)
    );

    let out = launcher
        .wait_with_output()
        .expect("the launcher ends after the stop");
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stdout(&out).contains("ended status=stopped reason=stop:user"),
        "{}",
        stdout(&out)
    );

    let log = fx.log();
    let last = log.lines().last().unwrap_or_default();
    assert!(
        last.contains("RUN | launch-end | status=stopped | reason=stop:user"),
        "{log}"
    );
    assert!(!log.contains("RUN | launch | n=2"), "no relaunch:\n{log}");
    let shim = fx.shim_log();
    let stop_line = shim
        .lines()
        .find(|l| l.starts_with("systemctl --user stop gsdmm-driver-r28-"))
        .unwrap_or_else(|| panic!("no scope stop in:\n{shim}"));
    assert!(stop_line.ends_with("-1.scope"), "{stop_line}");
    assert_eq!(probe_at(&run_lock_path(&fx.run_dir)), LockProbe::Free);
    assert!(!fx.any_driver_alive(), "the driver is gone");
    assert_eq!(fx.state_json()["status"], "stopped");
}

#[test]
fn stop_after_launcher_death_stops_the_recorded_scope() {
    let fx = Fixture::new();
    fx.write_scenario(WAITING_SCENARIO);
    let mut launcher = fx.spawn_launch(&[]);
    assert!(
        wait_until(Duration::from_secs(10), || fx.scenario_started()),
        "the launcher never reached its driver"
    );
    launcher.kill().expect("SIGKILL the launcher");
    launcher.wait().expect("reap the launcher");
    assert_eq!(probe_at(&run_lock_path(&fx.run_dir)), LockProbe::Free);
    assert!(
        fx.any_driver_alive(),
        "the orphaned driver is still running"
    );

    let log_before = fx.log();
    let state_before = fs::read(fx.run_dir.join("state.json")).unwrap();

    let stop = fx
        .stop_command(RUN_ID, &[])
        .output()
        .expect("spawn driver stop");
    assert!(stop.status.success(), "{}", stderr(&stop));
    assert!(
        stdout(&stop).contains("stopped the scope"),
        "{}",
        stdout(&stop)
    );
    assert!(
        wait_until(Duration::from_secs(10), || !fx.any_driver_alive()),
        "the orphaned driver survived the scope stop"
    );
    assert!(
        fx.shim_log()
            .lines()
            .any(|l| l.starts_with("systemctl --user stop gsdmm-driver-r28-")),
        "{}",
        fx.shim_log()
    );

    // D-09: with the launcher gone the stop records nothing.
    assert_eq!(fx.log(), log_before, "LOG.md is untouched");
    assert_eq!(
        fs::read(fx.run_dir.join("state.json")).unwrap(),
        state_before,
        "state.json is untouched"
    );
}

#[test]
fn stop_of_an_idle_run_reports_not_running() {
    let fx = Fixture::new();
    let stop = fx
        .stop_command(RUN_ID, &[])
        .output()
        .expect("spawn driver stop");
    assert!(stop.status.success(), "{}", stderr(&stop));
    assert!(stdout(&stop).contains("not running"), "{}", stdout(&stop));
    assert!(!fx.run_dir.join("state.json").exists(), "nothing written");
    assert!(!fx.run_dir.join("LOG.md").exists(), "nothing written");
    assert!(
        !run_lock_path(&fx.run_dir).exists(),
        "the probe must not create the lock file"
    );
    assert!(fx.shim_log().is_empty(), "no tool was started");
}

#[test]
fn stop_of_a_finished_run_with_no_live_scope_reports_not_running() {
    let fx = Fixture::new();
    fx.write_scenario(launcher_support::WELL_MANNERED_SCENARIO);
    let out = fx.launch(&[]);
    assert!(out.status.success(), "{}", stderr(&out));
    let log_before = fx.log();

    let stop = fx
        .stop_command(RUN_ID, &[])
        .output()
        .expect("spawn driver stop");
    assert!(stop.status.success(), "{}", stderr(&stop));
    assert!(stdout(&stop).contains("not running"), "{}", stdout(&stop));
    assert_eq!(fx.log(), log_before);
}

#[test]
fn stop_refuses_a_run_id_that_is_not_a_plain_component() {
    let fx = Fixture::new();
    for bad in ["../r28", "a/b", "bad id"] {
        let stop = fx.stop_command(bad, &[]).output().unwrap();
        assert_eq!(stop.status.code(), Some(1), "{bad}: {}", stderr(&stop));
    }
    let missing = fx.stop_command("nope", &[]).output().unwrap();
    assert_eq!(missing.status.code(), Some(1));
    assert!(
        stderr(&missing).contains("invalid run"),
        "{}",
        stderr(&missing)
    );
}
