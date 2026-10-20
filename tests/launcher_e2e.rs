#![cfg(target_os = "linux")]
// ============================================================================
// `gsd-meta-manager driver launch` end to end, through the REAL binary
// (Phase 28, plan 28-01; LNCH-01, LNCH-08, D-01..D-08, D-16, D-18, D-22).
//
// `systemd-run`, `bwrap`, `systemctl` and `claude` are fake executables first on
// `PATH` (`tests/launcher_support/mod.rs`), so this runs on any runner. It proves
// the launcher's architecture, not the kernel's isolation: the real-bwrap
// probes are 28-03's opt-in live tests.
// ============================================================================

#[path = "launcher_support/mod.rs"]
mod launcher_support;

use std::fs;
use std::process::Child;
use std::time::{Duration, Instant};

use gsd_meta_manager::driver::lock::{
    acquire_at, probe_at, read_holder_at, run_lock_path, LockProbe,
};
use gsd_meta_manager::launcher::prompt::{custom_version, MASTER_PROMPT};
use gsd_meta_manager::state_reader::driver_frontmatter::{
    read_driver_run, DerivedStatus, RunStatus,
};
use launcher_support::{
    wait_until, Fixture, BODY, RUN_ID, TOKEN, WAITING_SCENARIO, WELL_MANNERED_SCENARIO,
};

fn stderr(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

#[test]
fn a_dash_leading_template_reaches_the_driver_as_a_prompt_not_an_option() {
    let fx = Fixture::new();
    fx.write_scenario(WELL_MANNERED_SCENARIO);
    fs::write(
        fx.prompt_file(),
        "---\ntitle: front matter\n---\nMODE={MODE}\n",
    )
    .unwrap();

    let out = fx.launch(&[]);
    assert!(out.status.success(), "{}", stderr(&out));
    let prompt = fx.recorded("prompt.txt");
    assert!(
        prompt.starts_with("---\ntitle: front matter\n---"),
        "{prompt}"
    );
    assert!(prompt.contains("MODE=loop"), "{prompt}");
}

/// The `RUN | launch` line of launch number `n` in a run log.
fn launch_line(log: &str, n: u32) -> String {
    let marker = format!("RUN | launch | n={n} ");
    log.lines()
        .find(|l| l.contains(&marker))
        .unwrap_or_else(|| panic!("no launch {n} line in:\n{log}"))
        .to_owned()
}

#[test]
fn a_launch_without_a_prompt_file_runs_the_embedded_versioned_prompt() {
    let fx = Fixture::new();
    fx.write_scenario(WELL_MANNERED_SCENARIO);

    // PRMT-01 / D-04 / D-10: a run is launched by its id alone.
    let out = fx
        .launch_command_embedded(launcher_support::ALIAS, RUN_ID, &["--foreground"])
        .output()
        .expect("the binary is built and spawnable");
    assert!(out.status.success(), "{}", stderr(&out));

    let prompt = fx.recorded("prompt.txt");
    assert!(prompt.starts_with("<!-- driver-prompt/1.0.0"), "{prompt}");
    let run_dir = fs::canonicalize(&fx.run_dir).unwrap();
    let project = fs::canonicalize(fx.project()).unwrap();
    assert!(
        prompt.contains(&run_dir.display().to_string()),
        "the rendered RUN_DIR"
    );
    assert!(
        prompt.contains(&project.display().to_string()),
        "the rendered PROJECT_ROOT"
    );
    assert!(!prompt.contains("{RUN_DIR}"), "placeholders are filled");

    let log = fx.log();
    let line = launch_line(&log, 1);
    assert!(line.ends_with("prompt=driver-prompt/1.0.0"), "{line}");
    let view = read_driver_run(&fx.run_dir).expect("the run reads back");
    assert_eq!(
        view.frontmatter.prompt_version.as_deref(),
        Some("driver-prompt/1.0.0")
    );
}

#[test]
fn a_prompt_file_is_recorded_as_custom_with_the_hash_of_its_bytes() {
    let fx = Fixture::new();
    fx.write_scenario(WELL_MANNERED_SCENARIO);

    let out = fx.launch(&[]);
    assert!(out.status.success(), "{}", stderr(&out));

    let expected = custom_version(launcher_support::PROMPT_TEMPLATE.as_bytes());
    assert!(
        expected.starts_with("custom:") && expected.len() == "custom:".len() + 8,
        "{expected}"
    );
    let line = launch_line(&fx.log(), 1);
    assert!(line.ends_with(&format!("prompt={expected}")), "{line}");
    let view = read_driver_run(&fx.run_dir).expect("the run reads back");
    assert_eq!(
        view.frontmatter.prompt_version.as_deref(),
        Some(expected.as_str())
    );
}

#[test]
fn a_prompt_file_holding_the_embedded_bytes_is_still_custom() {
    // PRMT-01 adjacency edge: the source, not the content, decides the version.
    let fx = Fixture::new();
    fx.write_scenario(WELL_MANNERED_SCENARIO);
    fs::write(fx.prompt_file(), MASTER_PROMPT).unwrap();

    let out = fx.launch(&[]);
    assert!(out.status.success(), "{}", stderr(&out));

    let line = launch_line(&fx.log(), 1);
    let expected = custom_version(MASTER_PROMPT.as_bytes());
    assert!(line.ends_with(&format!("prompt={expected}")), "{line}");
    assert!(line.contains("prompt=custom:"), "{line}");
    assert!(!line.contains("driver-prompt/"), "{line}");
    let view = read_driver_run(&fx.run_dir).expect("the run reads back");
    assert_eq!(
        view.frontmatter.prompt_version.as_deref(),
        Some(expected.as_str())
    );
}

#[test]
fn every_launch_records_the_version_of_the_prompt_it_actually_used() {
    // Pitfall 1, and the invariant of the assumption-delta decision: launch 1 runs a
    // prompt file, launch 2 (after a crash) runs the embedded prompt. The frontmatter
    // holds the LATEST version; the log keeps both.
    let fx = Fixture::new();
    fx.write_scenario(WAITING_SCENARIO);
    let mut first = fx.spawn_launch(&[]);
    assert!(
        wait_until(Duration::from_secs(10), || fx.scenario_started()),
        "the launcher never reached its driver"
    );
    first.kill().expect("SIGKILL the launcher");
    first.wait().expect("reap the launcher");
    fx.kill_fixture_processes();

    fx.write_scenario(WELL_MANNERED_SCENARIO);
    let second = fx
        .launch_command_embedded(launcher_support::ALIAS, RUN_ID, &["--foreground"])
        .output()
        .expect("the binary is built and spawnable");
    assert!(second.status.success(), "{}", stderr(&second));

    let log = fx.log();
    let custom = custom_version(launcher_support::PROMPT_TEMPLATE.as_bytes());
    assert!(
        launch_line(&log, 1).ends_with(&format!("prompt={custom}")),
        "{log}"
    );
    assert!(
        launch_line(&log, 2).ends_with("prompt=driver-prompt/1.0.0"),
        "{log}"
    );
    let view = read_driver_run(&fx.run_dir).expect("the run reads back");
    assert_eq!(
        view.frontmatter.prompt_version.as_deref(),
        Some("driver-prompt/1.0.0")
    );
}

#[test]
fn an_empty_or_whitespace_prompt_file_is_refused_before_a_launch_is_logged() {
    for body in ["", "  \n\t\n"] {
        let fx = Fixture::new();
        fx.write_scenario(WELL_MANNERED_SCENARIO);
        fs::write(fx.prompt_file(), body).unwrap();

        let out = fx.launch(&[]);
        assert!(!out.status.success(), "{body:?} launched");
        assert!(
            stderr(&out).contains("the template is empty"),
            "{}",
            stderr(&out)
        );
        assert!(!fx.log().contains("RUN | launch"), "{}", fx.log());
        assert!(!fx.rec_dir().join("prompt.txt").exists(), "no driver ran");
    }
}

/// Assert a launch refused on DIRECTION.md left no trace: no lock record, no
/// `state.json`, no log, no per-run state directory and no `claude` invocation.
fn assert_nothing_was_written(fx: &Fixture, log_before: &str) {
    assert!(
        !run_lock_path(&fx.run_dir).exists(),
        "a lock record was written"
    );
    assert!(
        !fx.run_dir.join("state.json").exists(),
        "state.json was written"
    );
    assert_eq!(fx.log(), log_before, "LOG.md changed");
    assert!(!fx.state_dir().exists(), "the per-run state dir was made");
    assert_eq!(
        fs::read_dir(fx.rec_dir()).unwrap().count(),
        0,
        "claude was invoked"
    );
}

/// The `DIRECTION.md:<line>: ` lines on stderr.
fn direction_lines(out: &std::process::Output) -> Vec<String> {
    stderr(out)
        .lines()
        .filter(|l| l.contains("DIRECTION.md:"))
        .map(str::to_owned)
        .collect()
}

#[test]
fn a_direction_that_fails_the_lint_is_refused_before_anything_is_written() {
    let fx = Fixture::new();
    fx.write_scenario(WELL_MANNERED_SCENARIO);
    let body = BODY.replacen(
        "- done-when: Phase 1 has a passing VERIFICATION.md.\n",
        "",
        1,
    );
    assert_ne!(body, BODY, "the fixture body lost its done-when line");
    fs::write(fx.run_dir.join("DIRECTION.md"), &body).unwrap();
    let log_before = fx.log();

    let out = fx.launch(&[]);

    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    let lines = direction_lines(&out);
    assert_eq!(lines.len(), 1, "one problem, one line: {}", stderr(&out));
    assert!(
        lines[0].contains("I-01: missing `- done-when:`"),
        "{lines:?}"
    );
    assert_nothing_was_written(&fx, &log_before);
}

#[test]
fn the_detached_path_refuses_a_failing_direction_in_the_callers_terminal() {
    let fx = Fixture::new();
    fx.write_scenario(WELL_MANNERED_SCENARIO);
    let body = BODY.replacen(
        "- done-when: Phase 2 has a passing VERIFICATION.md.\n",
        "",
        1,
    );
    assert_ne!(body, BODY);
    fs::write(fx.run_dir.join("DIRECTION.md"), &body).unwrap();
    let log_before = fx.log();

    // No `--foreground`: the launcher would detach. The refusal happens before it.
    let child = fx
        .launch_command_embedded(launcher_support::ALIAS, RUN_ID, &[])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn the command");
    let out = finish(child, Duration::from_secs(15));

    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    assert!(
        direction_lines(&out)
            .iter()
            .any(|l| l.contains("I-02: missing `- done-when:`")),
        "{}",
        stderr(&out)
    );
    assert!(
        !stderr(&out).contains("detached launcher exited"),
        "no daemon was started: {}",
        stderr(&out)
    );
    assert_nothing_was_written(&fx, &log_before);
    assert!(
        !fx.state_dir().join("launcher.log").exists(),
        "no launcher.log: the refusal came first"
    );
}

#[test]
fn a_paused_or_stopped_direction_is_refused_at_launch() {
    // RESEARCH OQ2 [inferred - audit]: a launch on `control: pause|stop` would spend
    // the Opus startup cost just to be told to stop.
    for word in ["pause", "stop"] {
        let fx = Fixture::new();
        fx.write_scenario(WELL_MANNERED_SCENARIO);
        let body = BODY.replacen("- control: run\n", &format!("- control: {word}\n"), 1);
        assert_ne!(body, BODY);
        fs::write(fx.run_dir.join("DIRECTION.md"), &body).unwrap();
        let log_before = fx.log();

        let out = fx.launch(&[]);

        assert_eq!(out.status.code(), Some(1), "{word}: {}", stderr(&out));
        assert_eq!(
            direction_lines(&out),
            [format!(
                "DIRECTION.md:5: control is {word}; set - control: run to launch"
            )],
            "{}",
            stderr(&out)
        );
        assert_nothing_was_written(&fx, &log_before);
    }
}

#[test]
fn a_lint_warning_does_not_refuse_the_launch() {
    let fx = Fixture::new();
    fx.write_scenario(WELL_MANNERED_SCENARIO);
    let body = BODY.replacen(
        "- budget: dispatches=3\n",
        "- budget: dispatches=3, gap-rounds=3\n",
        1,
    );
    assert_ne!(body, BODY);
    fs::write(fx.run_dir.join("DIRECTION.md"), &body).unwrap();

    let out = fx.launch(&[]);

    assert!(out.status.success(), "{}", stderr(&out));
    assert!(fx.log().contains("RUN | launch | n=1"), "{}", fx.log());
}

#[test]
fn an_empty_or_oversized_template_is_refused_before_anything_is_written() {
    for (body, needle) in [
        (" \n".to_owned(), "empty"),
        ("x".repeat(130 * 1024), "over the"),
    ] {
        let fx = Fixture::new();
        fx.write_scenario(WELL_MANNERED_SCENARIO);
        fs::write(fx.prompt_file(), body).unwrap();
        let out = fx.launch(&[]);
        assert!(!out.status.success());
        assert!(stderr(&out).contains(needle), "{}", stderr(&out));
        assert!(
            !fx.rec_dir().join("prompt.txt").exists(),
            "no driver was started"
        );
    }
}

#[test]
fn launch_runs_the_driver_in_the_scope_and_records_its_end() {
    let fx = Fixture::new();
    fx.write_scenario(WELL_MANNERED_SCENARIO);

    let out = fx.launch(&[]);
    assert!(
        out.status.success(),
        "driver launch failed ({}): {}",
        out.status,
        stderr(&out)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("ended status=stopped reason=driver:stop"),
        "stdout: {stdout}"
    );

    // The run log: the launch (with the scope mode), the driver's start and end.
    let log = fx.log();
    assert!(
        log.contains("RUN | launch | n=1 | mode=bwrap-scope"),
        "{log}"
    );
    assert!(log.contains("RUN | start"), "{log}");
    assert!(log.contains("RUN | end"), "{log}");

    // The argv the shims captured: systemd-run > bwrap > claude.
    let shim = fx.shim_log();
    let unit_line = shim
        .lines()
        // The capability probes (`systemd-run --version`, 28-04) are logged first.
        .find(|l| l.starts_with("systemd-run --user"))
        .unwrap_or_else(|| panic!("no systemd-run line in:\n{shim}"));
    let prefix = "systemd-run --user --scope --quiet --collect --unit=gsdmm-driver-r28-";
    assert!(unit_line.starts_with(prefix), "{unit_line}");
    let hash = &unit_line[prefix.len()..prefix.len() + 8];
    assert!(hash.chars().all(|c| c.is_ascii_hexdigit()), "{unit_line}");
    assert!(
        unit_line[prefix.len() + 8..].starts_with("-1 "),
        "launch number 1: {unit_line}"
    );
    let canonical_run = fs::canonicalize(&fx.run_dir).unwrap();
    let run = canonical_run.display().to_string();
    assert!(shim.contains("--die-with-parent"), "{shim}");
    assert!(shim.contains("--unshare-pid"), "{shim}");
    assert!(
        shim.contains(&format!("--ro-bind {run} {run}")),
        "run dir read-only: {shim}"
    );
    for lock in ["launcher.lock", "DIRECTION.lock"] {
        assert!(
            shim.contains(&format!("--ro-bind /dev/null {run}/{lock}")),
            "{lock} masked: {shim}"
        );
    }
    assert!(shim.contains("claude -p "), "{shim}");
    assert!(
        shim.contains("--permission-mode bypassPermissions"),
        "{shim}"
    );

    // The scope unit is recorded on the run with its `.scope` suffix.
    let state = fx.state_json();
    let scope = state["connections"]
        .as_array()
        .unwrap()
        .iter()
        .find_map(|c| c["scope_unit"].as_str())
        .expect("a connection carries the scope unit");
    assert!(
        scope.starts_with("gsdmm-driver-r28-") && scope.ends_with("-1.scope"),
        "{scope}"
    );

    // The environment the driver saw: scrubbed, plus the launcher's own keys.
    let env = fx.recorded("env.txt");
    for key in [
        "CLAUDE_CONFIG_DIR=",
        "DRIVERCTL_SOCK=",
        "CLAUDE_CODE_OAUTH_TOKEN=",
        "DRIVER_LOCK_HELD=1",
    ] {
        assert!(env.lines().any(|l| l.starts_with(key)), "{key} in:\n{env}");
    }
    for stripped in ["SSH_AUTH_SOCK=", "GH_TOKEN=", "ANTHROPIC_API_KEY="] {
        assert!(
            !env.lines().any(|l| l.starts_with(stripped)),
            "{stripped} must be stripped:\n{env}"
        );
    }
    let config_dir = fx.config_dir();
    assert!(
        env.lines()
            .any(|l| l == format!("CLAUDE_CONFIG_DIR={}", config_dir.display())),
        "{env}"
    );
    assert!(
        env.lines()
            .find(|l| l.starts_with("PATH="))
            .is_some_and(|l| l.starts_with(&format!("PATH={}/bin:", config_dir.display()))),
        "the shim dir leads PATH:\n{env}"
    );
    assert_eq!(
        fx.recorded("token.txt"),
        TOKEN,
        "the stored token reaches the child"
    );

    // The token is in the environment only: no argv, no file the launcher wrote.
    assert!(!shim.contains(TOKEN), "token in a shim argv line");
    for name in ["LOG.md", "state.json", "DIRECTION.md"] {
        assert!(!fx.run_file(name).contains(TOKEN), "token in {name}");
    }
    assert!(
        !fs::read_to_string(config_dir.join("settings.json"))
            .unwrap()
            .contains(TOKEN),
        "token in settings.json"
    );
    assert!(
        !fs::read_to_string(fx.state_dir().join("launch-1.jsonl"))
            .unwrap()
            .contains(TOKEN),
        "token in the launch log"
    );

    // The prompt was rendered from the template (D-03).
    let prompt = fx.recorded("prompt.txt");
    assert!(prompt.contains(&format!("RUN_DIR={run}")), "{prompt}");
    assert!(
        prompt.contains(&format!(
            "PROJECT_ROOT={}",
            fs::canonicalize(fx.project()).unwrap().display()
        )),
        "{prompt}"
    );
    assert!(prompt.contains("MODE=loop"), "{prompt}");
    assert!(
        prompt.contains(&format!(
            "Begin now: run Startup, then the Loop, for RUN_DIR={run}."
        )),
        "{prompt}"
    );

    // The dispatch gate allowed the dispatch: an empty hook answer.
    assert_eq!(
        fx.recorded("hook_out.txt"),
        "",
        "an allowed dispatch prints nothing"
    );

    // How the run stands: stopped by the driver, lock released.
    let view = read_driver_run(&fx.run_dir).expect("the run reads back");
    assert_eq!(view.derived, DerivedStatus::AsWritten(RunStatus::Stopped));
    assert_eq!(probe_at(&run_lock_path(&fx.run_dir)), LockProbe::Free);

    // The seeded gate: PreToolUse[0] blocks on failure (hand-off 1).
    let settings: serde_json::Value =
        serde_json::from_str(&fs::read_to_string(config_dir.join("settings.json")).unwrap())
            .unwrap();
    let gate = &settings["hooks"]["PreToolUse"][0];
    assert_eq!(gate["matcher"], "Agent|Task");
    assert_eq!(gate["hooks"][0]["onFailure"], "block");
    assert!(config_dir.join("bin/driverctl").is_file());

    // The cap limits are recorded (D-12..D-14); enforcement is 28-05.
    assert_eq!(state["caps"]["dispatches"]["limit"], 10);
    assert_eq!(state["caps"]["wall_clock_min"]["limit"], 60.0);
    assert_eq!(state["caps"]["gap_rounds"]["limit"], 2);

    // LNCH-08: headless, no tmux anywhere, stdin is /dev/null.
    assert!(!shim.contains("tmux"), "tmux on the launch path: {shim}");
    assert_eq!(fx.recorded("stdin.txt").trim(), "/dev/null");

    // The socket was removed at shutdown.
    let socket_dir = fx.scratch.path().join("run/gsdmm-driverctl");
    let leftover: Vec<_> = fs::read_dir(&socket_dir)
        .map(|d| d.filter_map(Result::ok).collect())
        .unwrap_or_default();
    assert!(leftover.is_empty(), "sockets left behind: {leftover:?}");
}

#[test]
fn the_cap_flags_are_recorded_on_the_run() {
    let fx = Fixture::new();
    fx.write_scenario(WELL_MANNERED_SCENARIO);
    let out = fx.launch(&[
        "--cap-dispatches",
        "3",
        "--cap-minutes",
        "12.5",
        "--cap-gap-rounds",
        "0",
        "--model",
        "haiku",
        "--max-budget-usd",
        "1.5",
    ]);
    assert!(out.status.success(), "{}", stderr(&out));
    let state = fx.state_json();
    assert_eq!(state["caps"]["dispatches"]["limit"], 3);
    assert_eq!(state["caps"]["wall_clock_min"]["limit"], 12.5);
    assert_eq!(state["caps"]["gap_rounds"]["limit"], 0);
    let argv = fx.recorded("argv.txt");
    assert!(argv.lines().any(|l| l == "haiku"), "{argv}");
    assert!(argv.lines().any(|l| l == "1.5"), "{argv}");
}

#[test]
fn a_driver_that_dies_without_ending_is_recorded_crashed() {
    let fx = Fixture::new();
    fx.write_scenario("driverctl log start --mode headless --loop-mode loop\nexit 7\n");
    let out = fx.launch(&[]);
    assert!(out.status.success(), "{}", stderr(&out));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("status=crashed"), "{stdout}");
    assert!(
        fx.log().contains("driver exited rc=7 without RUN end"),
        "{}",
        fx.log()
    );
    assert_eq!(probe_at(&run_lock_path(&fx.run_dir)), LockProbe::Free);
}

#[test]
fn a_clean_exit_with_a_worker_outstanding_is_recorded_stalled() {
    let fx = Fixture::new();
    fx.write_scenario(
        "driverctl log start --mode headless --loop-mode loop\n\
         driverctl log dispatched --item I-01 --step plan\n\
         printf '%s\\n' '{\"type\":\"result\",\"subtype\":\"success\"}'\n",
    );
    let out = fx.launch(&[]);
    assert!(out.status.success(), "{}", stderr(&out));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("status=stalled"), "{stdout}\n{}", fx.log());
}

#[test]
fn a_held_run_lock_is_refused_with_exit_3_before_anything_is_written() {
    let fx = Fixture::new();
    fx.write_scenario(WELL_MANNERED_SCENARIO);
    let _held = acquire_at(&run_lock_path(&fx.run_dir), RUN_ID, std::process::id())
        .expect("the test takes the lock first");

    let out = fx.launch(&[]);
    assert_eq!(out.status.code(), Some(3), "{}", stderr(&out));
    assert!(
        stderr(&out).contains("already launched"),
        "{}",
        stderr(&out)
    );
    assert_eq!(fx.log(), "", "a refused launch writes no LOG");
    assert!(!fx.run_dir.join("state.json").exists(), "or state.json");
    // The capability probes run before the lock (28-04); a driver must not start.
    assert!(
        !fx.shim_log().contains("systemd-run --user") && !fx.shim_log().contains("claude -p"),
        "or starts a process: {}",
        fx.shim_log()
    );
    assert!(!fx.state_dir().exists(), "or a state directory");
}

#[test]
fn a_missing_token_is_refused_before_anything_is_written() {
    let fx = Fixture::new();
    fx.write_scenario(WELL_MANNERED_SCENARIO);
    fx.write_token(None);

    let out = fx.launch(&[]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        stderr(&out).contains("gsd-meta-manager driver auth setup"),
        "{}",
        stderr(&out)
    );
    assert!(!fx.run_dir.join("state.json").exists());
    assert!(
        !fx.run_dir.join("launcher.lock").exists(),
        "the lock is taken after the token"
    );
    assert!(fx.shim_log().is_empty());
}

#[test]
fn a_stored_token_open_to_others_is_refused() {
    use std::os::unix::fs::PermissionsExt;

    let fx = Fixture::new();
    fx.write_scenario(WELL_MANNERED_SCENARIO);
    fs::set_permissions(fx.token_path(), fs::Permissions::from_mode(0o644)).unwrap();

    let out = fx.launch(&[]);
    assert_eq!(out.status.code(), Some(1));
    assert!(
        stderr(&out).contains("must not be accessible"),
        "{}",
        stderr(&out)
    );
    assert!(fx.shim_log().is_empty());
}

#[test]
fn a_run_id_that_is_not_a_plain_component_is_refused() {
    let fx = Fixture::new();
    for bad in ["../r28", "a/b", ".hidden", "bad id"] {
        let out = fx.launch_command(bad, &[]).output().unwrap();
        assert_eq!(out.status.code(), Some(1), "{bad}: {}", stderr(&out));
    }
    assert!(fx.shim_log().is_empty());
}

#[test]
fn a_run_dir_that_does_not_exist_is_refused() {
    let fx = Fixture::new();
    let out = fx.launch_command("nope", &[]).output().unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(stderr(&out).contains("invalid run"), "{}", stderr(&out));
    assert!(fx.shim_log().is_empty());
}

#[test]
fn an_unknown_project_is_refused_through_the_registry() {
    let fx = Fixture::new();
    let out = fx
        .launch_command_for("nosuch", RUN_ID, &[])
        .output()
        .unwrap();
    assert_eq!(out.status.code(), Some(1));
    assert!(
        stderr(&out).contains("is not a registered project alias"),
        "{}",
        stderr(&out)
    );
    assert!(fx.shim_log().is_empty());
}

// ----------------------------------------------------------------------------
// Plan 28-02: the run lifecycle (LNCH-02, DRIVE-01 launcher half; D-04, D-05,
// D-07, D-16). Every test here goes through the real binary.
// ----------------------------------------------------------------------------

/// Wait for a spawned launcher, failing the test if it hangs.
fn finish(mut child: Child, within: Duration) -> std::process::Output {
    assert!(
        wait_until(within, || child.try_wait().expect("try_wait").is_some()),
        "the launcher did not finish within {within:?}"
    );
    child
        .wait_with_output()
        .expect("collect the launcher output")
}

/// `state.json` without the fields a running launcher rewrites on its own tick.
fn quiescent_state(fx: &Fixture) -> serde_json::Value {
    let mut state = fx.state_json();
    state.as_object_mut().unwrap().remove("updated_at");
    if let Some(connections) = state["connections"].as_array_mut() {
        for c in connections {
            c.as_object_mut().unwrap().remove("last_heartbeat");
        }
    }
    state
}

#[test]
fn second_launch_is_refused_with_rc_3() {
    let fx = Fixture::new();
    fx.write_scenario(WAITING_SCENARIO);
    let first = fx.spawn_launch(&[]);
    assert!(
        wait_until(Duration::from_secs(10), || fx.scenario_started()),
        "launcher A never reached its driver"
    );

    let log_before = fx.log();
    let state_before = quiescent_state(&fx);
    let started = Instant::now();
    let refused = fx.launch(&[]);
    assert_eq!(refused.status.code(), Some(3), "{}", stderr(&refused));
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "the refusal took {:?}",
        started.elapsed()
    );
    assert_eq!(
        fx.log(),
        log_before,
        "a refused launcher writes no LOG line"
    );
    assert_eq!(
        quiescent_state(&fx),
        state_before,
        "a refused launcher writes nothing to state.json"
    );

    fx.release();
    let out = finish(first, Duration::from_secs(20));
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert_eq!(fx.log().matches("RUN | launch | n=1").count(), 1);
}

#[test]
fn simultaneous_launches_leave_exactly_one_launch() {
    let fx = Fixture::new();
    fx.write_scenario(WAITING_SCENARIO);
    // Back to back, with no wait between the spawns: the lock decides.
    let mut a = fx.spawn_launch(&[]);
    let mut b = fx.spawn_launch(&[]);

    assert!(
        wait_until(Duration::from_secs(15), || {
            a.try_wait().unwrap().is_some() || b.try_wait().unwrap().is_some()
        }),
        "neither launcher gave up the race"
    );
    assert!(
        wait_until(Duration::from_secs(10), || fx.scenario_started()),
        "the winner never reached its driver"
    );
    fx.release();
    let out_a = finish(a, Duration::from_secs(20));
    let out_b = finish(b, Duration::from_secs(20));
    let mut codes = [out_a.status.code(), out_b.status.code()];
    codes.sort();
    assert_eq!(
        codes,
        [Some(0), Some(3)],
        "exactly one launcher wins: {} / {}",
        String::from_utf8_lossy(&out_a.stderr),
        String::from_utf8_lossy(&out_b.stderr)
    );
    let log = fx.log();
    assert_eq!(log.matches("RUN | launch | n=1").count(), 1, "{log}");
    assert_eq!(log.matches("RUN | launch |").count(), 1, "{log}");
}

#[test]
fn sigkill_then_relaunch_logs_crash_detected() {
    let fx = Fixture::new();
    fx.write_scenario(WAITING_SCENARIO);
    let mut first = fx.spawn_launch(&[]);
    assert!(
        wait_until(Duration::from_secs(10), || fx.scenario_started()),
        "the launcher never reached its driver"
    );
    first.kill().expect("SIGKILL the launcher");
    first.wait().expect("reap the launcher");
    // The fake driver is not tied to its parent; the real one is (die-with-parent).
    fx.kill_fixture_processes();

    let view = read_driver_run(&fx.run_dir).expect("the run reads back");
    assert!(
        matches!(
            view.derived,
            DerivedStatus::Crashed {
                written: RunStatus::Running
            }
        ),
        "{:?}",
        view.derived
    );
    assert_eq!(probe_at(&run_lock_path(&fx.run_dir)), LockProbe::Free);

    fx.write_scenario(WELL_MANNERED_SCENARIO);
    let second = fx.launch(&[]);
    assert!(second.status.success(), "{}", stderr(&second));
    let log = fx.log();
    let crash = log
        .find("RUN | crash-detected | previous_launch=1")
        .unwrap_or_else(|| panic!("no crash-detected line:\n{log}"));
    let relaunch = log
        .find("RUN | launch | n=2")
        .unwrap_or_else(|| panic!("no second launch line:\n{log}"));
    assert!(
        crash < relaunch,
        "crash-detected precedes the launch:\n{log}"
    );
}

#[test]
fn fresh_exit_relaunches_under_one_lock() {
    let fx = Fixture::new();
    fx.write_scenario(
        r#"
n=$(cat "$FAKE_CLAUDE_DIR/count" 2>/dev/null || echo 0)
n=$((n + 1))
echo "$n" > "$FAKE_CLAUDE_DIR/count"
echo "$PPID" >> "$FAKE_CLAUDE_DIR/launcher-pids"
driverctl log start --mode headless --loop-mode fresh-per-item
if [ "$n" = 1 ]; then
  driverctl log end --reason fresh-exit
else
  driverctl log end --reason all-done
fi
printf '%s\n' '{"type":"result","subtype":"success"}'
"#,
    );
    let child = fx.spawn_launch(&["--loop-mode", "fresh-per-item"]);
    let launcher_pid = child.id().to_string();
    let out = finish(child, Duration::from_secs(30));
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );

    let log = fx.log();
    assert!(log.contains("RUN | launch | n=1"), "{log}");
    assert!(log.contains("RUN | launch | n=2"), "{log}");
    assert!(
        !log.contains("crash-detected"),
        "no crash between launches:\n{log}"
    );
    assert_eq!(
        log.matches("RUN | launch-end |").count(),
        2,
        "one launch-end per launch:\n{log}"
    );

    // One process was the launcher of both drivers: the lock never changed hands.
    let pids = fx.recorded("launcher-pids");
    let pids: Vec<&str> = pids.lines().collect();
    assert_eq!(pids, [launcher_pid.as_str(); 2], "{pids:?}");

    let state = fx.state_json();
    assert_eq!(state["status"], "done");
    assert_eq!(state["launches"], 2);
    let view = read_driver_run(&fx.run_dir).expect("the run reads back");
    assert_eq!(view.derived, DerivedStatus::AsWritten(RunStatus::Done));
    assert_eq!(probe_at(&run_lock_path(&fx.run_dir)), LockProbe::Free);
}

#[test]
fn launch_end_is_recorded_when_the_driver_ended_the_run() {
    let fx = Fixture::new();
    fx.write_scenario(WELL_MANNERED_SCENARIO);
    let out = fx.launch(&[]);
    assert!(out.status.success(), "{}", stderr(&out));

    let log = fx.log();
    let last = log.lines().last().unwrap_or_default();
    assert!(
        last.contains("RUN | launch-end | rc=0 | status=stopped"),
        "the LOG ends with the launch end:\n{log}"
    );
    let state = fx.state_json();
    assert_eq!(state["status"], "stopped", "the status is unchanged");
    assert_eq!(state["status_reason"], "driver:stop");
    for c in state["connections"].as_array().unwrap() {
        assert_ne!(c["state"], "connected", "{c}");
    }
}

/// The session id of `pid` (`/proc/<pid>/stat` field 6).
fn session_of(pid: &str) -> u32 {
    let stat = fs::read_to_string(format!("/proc/{pid}/stat")).expect("read /proc stat");
    // The command name may contain spaces and parentheses: count after the last ')'.
    let rest = &stat[stat.rfind(')').expect("comm ends with ')'") + 1..];
    rest.split_whitespace()
        .nth(3)
        .expect("the session field")
        .parse()
        .expect("a numeric session id")
}

#[test]
fn detached_launch_returns_once_the_child_holds_the_lock() {
    let fx = Fixture::new();
    fx.write_scenario(WAITING_SCENARIO);
    let started = Instant::now();
    let mut parent = fx
        .launch_command_raw(launcher_support::ALIAS, RUN_ID, &[])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn the command");
    let parent_pid = parent.id();
    assert!(
        wait_until(Duration::from_secs(10), || parent
            .try_wait()
            .unwrap()
            .is_some()),
        "driver launch did not return"
    );
    let out = parent.wait_with_output().expect("collect output");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(started.elapsed() < Duration::from_secs(10));
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("started (pid "), "{stdout}");
    assert!(stdout.contains("launcher.log"), "{stdout}");

    // The run is live: the child holds the lock, and it is not the command we ran.
    assert_eq!(probe_at(&run_lock_path(&fx.run_dir)), LockProbe::Held);
    let holder = read_holder_at(&run_lock_path(&fx.run_dir)).expect("holder record");
    assert_ne!(
        holder.pgid, parent_pid,
        "the lock holder is the detached child"
    );
    let own_session = session_of("self");
    let holder_session = session_of(&holder.pgid.to_string());
    assert_ne!(holder_session, own_session, "the child has its own session");
    assert_eq!(
        holder_session, holder.pgid,
        "it leads that session (setsid)"
    );
    assert!(
        fx.state_dir().join("launcher.log").is_file(),
        "launcher.log under the run state dir"
    );

    // The run goes on to a terminal status without anybody attached.
    assert!(
        wait_until(Duration::from_secs(10), || fx.scenario_started()),
        "the detached launcher never reached its driver"
    );
    fx.release();
    assert!(
        wait_until(Duration::from_secs(20), || {
            matches!(
                read_driver_run(&fx.run_dir).map(|v| v.derived),
                Ok(DerivedStatus::AsWritten(RunStatus::Stopped))
            )
        }),
        "the detached run never reached a terminal status:\n{}",
        fx.log()
    );
}

#[test]
fn a_detached_launch_without_a_prompt_file_records_the_embedded_version() {
    // The daemon is re-executed with the parent's own argv: with no prompt file the
    // parent and the daemon both resolve the embedded asset and agree on its version.
    let fx = Fixture::new();
    fx.write_scenario(WELL_MANNERED_SCENARIO);
    let child = fx
        .launch_command_embedded(launcher_support::ALIAS, RUN_ID, &[])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn the command");
    let out = finish(child, Duration::from_secs(15));
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        wait_until(Duration::from_secs(20), || {
            matches!(
                read_driver_run(&fx.run_dir).map(|v| v.derived),
                Ok(DerivedStatus::AsWritten(RunStatus::Stopped))
            )
        }),
        "the detached run never reached a terminal status:\n{}",
        fx.log()
    );
    assert!(
        launch_line(&fx.log(), 1).ends_with("prompt=driver-prompt/1.0.0"),
        "{}",
        fx.log()
    );
    assert!(fx
        .recorded("prompt.txt")
        .starts_with("<!-- driver-prompt/1.0.0"));
}

/// A detached `driver launch` (no `--foreground`), bounded so a launcher that
/// never detaches fails the test instead of hanging it.
fn detached(fx: &Fixture) -> std::process::Output {
    let child = fx
        .launch_command_raw(launcher_support::ALIAS, RUN_ID, &[])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .spawn()
        .expect("spawn the command");
    finish(child, Duration::from_secs(15))
}

#[test]
fn detached_second_launch_returns_3() {
    let fx = Fixture::new();
    fx.write_scenario(WAITING_SCENARIO);
    let first = detached(&fx);
    assert!(
        first.status.success(),
        "{}",
        String::from_utf8_lossy(&first.stderr)
    );
    assert!(
        wait_until(Duration::from_secs(10), || fx.scenario_started()),
        "the first launcher never reached its driver"
    );

    let log_before = fx.log();
    let second = detached(&fx);
    assert_eq!(second.status.code(), Some(3), "{}", stderr(&second));
    assert_eq!(
        fx.log(),
        log_before,
        "the refused detached launch wrote no LOG line"
    );
    fx.release();
    assert!(wait_until(Duration::from_secs(20), || {
        probe_at(&run_lock_path(&fx.run_dir)) == LockProbe::Free
    }));
}
