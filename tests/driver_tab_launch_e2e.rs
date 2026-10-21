#![cfg(target_os = "linux")]
// ============================================================================
// The Driver tab's relaunch and launch, through the REAL `driver launch` binary
// with the fake claude / bwrap / systemd-run shims (Phase 30, plan 30-05; D-19,
// D-21, D-26). Zero spend: no live model is called anywhere in this file.
//
// What this proves, and what it does not. The tab's `LaunchFormScreen` confirms a
// `LaunchArgs`; the App turns it into `LaunchOptions` with
// `driver_launch::launch_options_for` and hands them to the launcher library's
// `launch_detached`, which re-executes `current_exe`. A unit test cannot call that
// in-process (it would re-execute the TEST binary: RESEARCH Pitfall 5), so the
// App-level path is covered by the `LaunchSeams` tests in `src/app.rs` with a fake
// seam, and THIS file covers the other half: the argv `launch_argv` derives from the
// very options the tab builds is a command line the real binary accepts, and running
// it (minus the two daemon flags, which only change where it runs) launches the run.
// The in-process `launch_detached` call itself is therefore not exercised end to
// end; that is stated for the verifier rather than left to be discovered.
// ============================================================================

#[path = "launcher_support/mod.rs"]
mod launcher_support;

use std::ffi::OsString;
use std::fs;
use std::time::Duration;

use gsd_meta_manager::driver::direction::split_frontmatter;
use gsd_meta_manager::launcher::run::{launch_argv, LoopMode};
use gsd_meta_manager::state_reader::driver_frontmatter::{read_driver_run, DerivedStatus};
use gsd_meta_manager::ui::screens::driver_launch::{
    launch_kind_for, launch_options_for, relaunch_prefill, LaunchForm, LaunchKind,
};
use launcher_support::{wait_until, Fixture, ALIAS, BODY, RUN_ID};

/// Launch 1 of the relaunch test: start, signal readiness and hang until the test
/// kills it. No item is dispatched, so the crash leaves nothing to reconcile.
const CRASH_SCENARIO: &str = r#"
driverctl log start --mode headless --loop-mode loop
touch "$FAKE_CLAUDE_DIR/started"
while :; do sleep 1; done
"#;

/// A driver that starts and ends cleanly.
const CLEAN_SCENARIO: &str = r#"
driverctl log start --mode headless --loop-mode loop
driverctl log end --reason stop
printf '%s\n' '{"type":"result","subtype":"success"}'
"#;

/// The argv the tab's options produce, with the two daemon-only words dropped: what a
/// user would type to run the same launch in the foreground.
fn foreground_argv(argv: Vec<OsString>) -> Vec<OsString> {
    let mut argv = argv;
    let tail = argv.split_off(argv.len() - 2);
    assert_eq!(
        tail,
        [
            OsString::from("--foreground"),
            OsString::from("--daemon-child")
        ],
        "launch_argv ends with the daemon flags"
    );
    argv
}

#[test]
fn a_crashed_run_relaunches_with_the_argv_the_tab_builds() {
    let fx = Fixture::new();

    // Launch 1: the driver starts, then the launcher AND the driver are SIGKILLed.
    fx.write_scenario(CRASH_SCENARIO);
    let mut launcher = fx.spawn_launch(&[]);
    assert!(
        wait_until(Duration::from_secs(10), || fx.scenario_started()),
        "the launcher never reached its driver:\n{}",
        fx.log()
    );
    launcher.kill().expect("SIGKILL the launcher");
    launcher.wait().expect("reap the launcher");
    fx.kill_fixture_processes();
    assert!(
        wait_until(Duration::from_secs(10), || !fx.any_driver_alive()),
        "the killed driver is still alive"
    );

    // The tab reads the run: the lock is free and the file says running.
    assert!(
        wait_until(Duration::from_secs(10), || {
            read_driver_run(&fx.run_dir)
                .is_ok_and(|view| matches!(view.derived, DerivedStatus::Crashed { .. }))
        }),
        "the run never derived `crashed`:\n{}",
        fx.run_file("DIRECTION.md")
    );
    let view = read_driver_run(&fx.run_dir).expect("the run reads");
    assert_eq!(launch_kind_for(&view.derived), Ok(LaunchKind::Relaunch));

    // What `R` builds: the form prefilled from the run, the options for it, the argv.
    let form = relaunch_prefill(&view.frontmatter);
    let mut args = form.args.clone();
    args.project = ALIAS.to_string();
    args.run_id = RUN_ID.to_string();
    let options = launch_options_for(ALIAS, fx.project(), &args, false, None);
    assert_eq!(options.run_id, RUN_ID, "the SAME run id is relaunched");
    assert!(!options.unsandboxed);
    assert_eq!(options.loop_mode, LoopMode::Loop);
    let argv = foreground_argv(launch_argv(&options, None));

    // Launch 2: the real binary, the fake tools, the tab's argv.
    fx.write_scenario(CLEAN_SCENARIO);
    let out = fx
        .hermetic_command()
        .args(&argv)
        .arg("--foreground")
        .output()
        .expect("the binary is built and spawnable");
    assert!(
        out.status.success(),
        "the tab's argv was refused:\nstdout: {}\nstderr: {}\nargv: {argv:?}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );

    let log = fx.log();
    let crash = log
        .find("RUN | crash-detected")
        .unwrap_or_else(|| panic!("no crash-detected line:\n{log}"));
    let relaunch = log
        .find("RUN | launch | n=2")
        .unwrap_or_else(|| panic!("no second launch line:\n{log}"));
    assert!(
        crash < relaunch,
        "the crash is recorded before the relaunch:\n{log}"
    );
    assert!(
        fx.recorded("argv.txt").contains("--model"),
        "the fake claude ran under the launcher"
    );
}

#[test]
fn a_new_run_launches_with_the_argv_the_tab_builds() {
    let fx = Fixture::new();

    // `s` mints the run the way the tab does, then the user's editor replaces the body
    // (here: the fixture's lint-clean one) under the tool-written frontmatter.
    let now = chrono::Utc::now();
    let new_run = gsd_meta_manager::driver_templates::create_run_dir(fx.project(), "tab-e2e", now)
        .expect("create_run_dir mints the run");
    let direction = new_run.run_dir.join("DIRECTION.md");
    let created = fs::read_to_string(&direction).expect("DIRECTION.md");
    let block = split_frontmatter(&created)
        .0
        .expect("a tool block")
        .to_string();
    fs::write(&direction, format!("{block}{BODY}")).expect("the edited DIRECTION.md");

    let view = read_driver_run(&new_run.run_dir).expect("the new run reads");
    assert_eq!(view.derived, DerivedStatus::NotLaunched);
    assert_eq!(launch_kind_for(&view.derived), Ok(LaunchKind::Launch));

    // `R` on a never-launched run: the CLI defaults, nothing prefilled.
    let mut args = LaunchForm::fresh().args;
    args.project = ALIAS.to_string();
    args.run_id = new_run.run_id.clone();
    let options = launch_options_for(ALIAS, fx.project(), &args, false, None);
    let argv = foreground_argv(launch_argv(&options, None));
    let as_text: Vec<String> = argv
        .iter()
        .map(|a| a.to_string_lossy().into_owned())
        .collect();
    assert_eq!(
        &as_text[2..],
        [
            fx.project().display().to_string().as_str(),
            new_run.run_id.as_str(),
            "--model",
            "opus",
            "--loop-mode",
            "loop",
            "--max-budget-usd",
            "10",
            "--cap-dispatches",
            "10",
            "--cap-minutes",
            "60",
            "--cap-gap-rounds",
            "2",
        ],
        "the defaults, as the CLI would parse them"
    );

    fx.write_scenario(CLEAN_SCENARIO);
    let out = fx
        .hermetic_command()
        .args(&argv)
        .arg("--foreground")
        .output()
        .expect("the binary is built and spawnable");
    assert!(
        out.status.success(),
        "the tab's argv was refused:\nstdout: {}\nstderr: {}",
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr),
    );
    let log = fs::read_to_string(new_run.run_dir.join("LOG.md")).expect("LOG.md");
    assert!(log.contains("RUN | launch | n=1"), "{log}");
}
