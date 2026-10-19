#![cfg(target_os = "linux")]
// ============================================================================
// The honest fallback (Phase 28, plan 28-04; LNCH-07, D-28..D-31): a host that
// cannot sandbox is REFUSED unless the launch carries `--unsandboxed`, and an
// unsandboxed driver is a process group the launcher supervises and kills.
//
// Everything runs through the REAL binary with the fake tools of
// `tests/launcher_support`; the failing variants (a `bwrap` that exits 1, a
// `systemctl` that says `offline`, a `claude` that is too old) are written in this
// file. The fakes log every call, the capability probes included, so the
// assertions about "no sandbox was started" look for the launch lines
// (`systemd-run --user ...`, a `bwrap` line that runs `claude`), never for an empty
// log.
// ============================================================================

#[path = "launcher_support/mod.rs"]
mod launcher_support;

use std::collections::BTreeMap;
use std::fs;
use std::path::Path;
use std::time::Duration;

use gsd_meta_manager::driver::lock::{probe_at, run_lock_path, LockProbe};
use gsd_meta_manager::state_reader::driver_frontmatter::read_driver_run;
use launcher_support::{
    wait_until, write_executable, Fixture, RUN_ID, WAITING_SCENARIO, WELL_MANNERED_SCENARIO,
};

fn stderr(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn stdout(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// A `bwrap` that cannot create a sandbox (a copied or AppArmor-blocked binary).
fn break_bwrap(fx: &Fixture) {
    write_executable(&fx.shims(), "bwrap", "#!/bin/sh\nexit 1\n");
}

/// Every file under `dir` with its bytes, for a "nothing was written" comparison.
fn snapshot(dir: &Path) -> BTreeMap<String, Vec<u8>> {
    fn walk(base: &Path, dir: &Path, out: &mut BTreeMap<String, Vec<u8>>) {
        for entry in fs::read_dir(dir).into_iter().flatten().flatten() {
            let path = entry.path();
            if path.is_dir() {
                walk(base, &path, out);
            } else {
                let rel = path.strip_prefix(base).unwrap().display().to_string();
                out.insert(rel, fs::read(&path).unwrap_or_default());
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(dir, dir, &mut out);
    out
}

/// The shim lines that START a driver: a `systemd-run --user` scope, or a `bwrap`
/// that goes on to run `claude`.
fn launch_lines(fx: &Fixture) -> Vec<String> {
    fx.shim_log()
        .lines()
        .filter(|l| {
            l.starts_with("systemd-run --user") || (l.starts_with("bwrap") && l.contains("claude"))
        })
        .map(str::to_owned)
        .collect()
}

/// Assert a refusal wrote nothing: not to the run dir, not a lock, not a state dir,
/// and started no driver.
fn assert_refused_cleanly(
    fx: &Fixture,
    before: &BTreeMap<String, Vec<u8>>,
    out: &std::process::Output,
) {
    assert_eq!(out.status.code(), Some(1), "{}", stderr(out));
    assert_eq!(
        &snapshot(&fx.run_dir),
        before,
        "the run dir is byte-identical"
    );
    assert!(!run_lock_path(&fx.run_dir).exists(), "no lock was taken");
    assert!(!fx.state_dir().exists(), "no state directory was made");
    assert_eq!(
        launch_lines(fx),
        Vec::<String>::new(),
        "no driver was started"
    );
}

#[test]
fn launch_refuses_without_a_working_bwrap() {
    let fx = Fixture::new();
    fx.write_scenario(WELL_MANNERED_SCENARIO);
    break_bwrap(&fx);
    let before = snapshot(&fx.run_dir);

    let out = fx.launch(&[]);
    let err = stderr(&out);
    assert_refused_cleanly(&fx, &before, &out);
    assert!(err.contains("bwrap"), "names the missing piece: {err}");
    assert!(err.contains("--unsandboxed"), "names the way out: {err}");
    assert!(!err.contains("systemd-run,"), "only what is missing: {err}");
}

#[test]
fn a_detached_launch_is_refused_in_the_callers_terminal() {
    let fx = Fixture::new();
    fx.write_scenario(WELL_MANNERED_SCENARIO);
    break_bwrap(&fx);
    let before = snapshot(&fx.run_dir);

    let out = fx
        .launch_command_raw(launcher_support::ALIAS, RUN_ID, &[])
        .output()
        .expect("spawn");
    assert_refused_cleanly(&fx, &before, &out);
    assert!(stderr(&out).contains("--unsandboxed"), "{}", stderr(&out));
}

#[test]
fn launch_refuses_when_the_user_manager_is_offline() {
    let fx = Fixture::new();
    fx.write_scenario(WELL_MANNERED_SCENARIO);
    write_executable(
        &fx.shims(),
        "systemctl",
        "#!/bin/sh\necho offline\nexit 1\n",
    );
    let before = snapshot(&fx.run_dir);

    let out = fx.launch(&[]);
    assert_refused_cleanly(&fx, &before, &out);
    let err = stderr(&out);
    assert!(
        err.contains("user-systemd"),
        "names the user manager: {err}"
    );
    assert!(err.contains("--unsandboxed"), "{err}");
}

#[test]
fn a_degraded_user_manager_is_usable() {
    // `degraded` exits 1 and is still a manager that can run a scope (RESEARCH §2).
    let fx = Fixture::new();
    fx.write_scenario(WELL_MANNERED_SCENARIO);
    let shim = fs::read_to_string(fx.shims().join("systemctl")).unwrap();
    write_executable(
        &fx.shims(),
        "systemctl",
        &shim.replace(
            "is-system-running) echo running; exit 0;;",
            "is-system-running) echo degraded; exit 1;;",
        ),
    );
    let out = fx.launch(&[]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(fx.log().contains("RUN | launch | n=1 | mode=bwrap-scope"));
}

#[test]
fn a_missing_systemd_run_is_named() {
    let fx = Fixture::new();
    fx.write_scenario(WELL_MANNERED_SCENARIO);
    write_executable(&fx.shims(), "systemd-run", "#!/bin/sh\nexit 127\n");
    let before = snapshot(&fx.run_dir);

    let out = fx.launch(&[]);
    assert_refused_cleanly(&fx, &before, &out);
    assert!(stderr(&out).contains("systemd-run"), "{}", stderr(&out));
}

#[test]
fn launch_refuses_an_old_claude() {
    for unsandboxed in [false, true] {
        let fx = Fixture::new();
        fx.write_scenario(WELL_MANNERED_SCENARIO);
        write_executable(
            &fx.shims(),
            "claude",
            "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo '2.1.200 (Claude Code)'; exit 0; fi\nexit 0\n",
        );
        let before = snapshot(&fx.run_dir);

        let args: &[&str] = if unsandboxed { &["--unsandboxed"] } else { &[] };
        let out = fx.launch(args);
        assert_refused_cleanly(&fx, &before, &out);
        let err = stderr(&out);
        assert!(err.contains("2.1.295"), "unsandboxed={unsandboxed}: {err}");
        assert!(err.contains("2.1.200"), "unsandboxed={unsandboxed}: {err}");
    }
}

#[test]
fn an_unreadable_claude_version_is_refused() {
    let fx = Fixture::new();
    fx.write_scenario(WELL_MANNERED_SCENARIO);
    write_executable(
        &fx.shims(),
        "claude",
        "#!/bin/sh\nif [ \"$1\" = \"--version\" ]; then echo garbage; exit 0; fi\nexit 0\n",
    );
    let before = snapshot(&fx.run_dir);

    let out = fx.launch(&["--unsandboxed"]);
    assert_refused_cleanly(&fx, &before, &out);
    assert!(stderr(&out).contains("2.1.295"), "{}", stderr(&out));
}

#[test]
fn unsandboxed_launch_runs_without_bwrap_and_records_degraded() {
    let fx = Fixture::new();
    fx.write_scenario(WELL_MANNERED_SCENARIO);
    break_bwrap(&fx);

    let out = fx.launch(&["--unsandboxed"]);
    assert!(
        out.status.success(),
        "driver launch failed ({}): {}",
        out.status,
        stderr(&out)
    );
    assert!(
        stdout(&out).contains("ended status=stopped reason=driver:stop"),
        "{}",
        stdout(&out)
    );

    let log = fx.log();
    assert!(log.contains("RUN | launch | n=1 | mode=degraded"), "{log}");
    assert!(!log.contains("mode=bwrap-scope"), "{log}");

    // No bwrap and no scope were used to run the driver.
    assert_eq!(launch_lines(&fx), Vec::<String>::new(), "{}", fx.shim_log());
    assert!(
        !fx.shim_log().contains("systemctl --user stop"),
        "there is no scope to stop: {}",
        fx.shim_log()
    );

    // The driver ran, with the launcher's config dir and socket, and the gate hook
    // worked through the same driverctl socket.
    let env = fx.recorded("env.txt");
    assert!(env.contains("CLAUDE_CONFIG_DIR="), "{env}");
    assert!(env.contains("DRIVERCTL_SOCK="), "{env}");
    assert!(env.contains("CLAUDE_CODE_OAUTH_TOKEN=REDACTED"), "{env}");
    assert!(
        !env.contains("SSH_AUTH_SOCK"),
        "the env scrub still applies: {env}"
    );
    assert!(!env.contains("GH_TOKEN"), "{env}");
    assert!(fx.rec_dir().join("hook_out.txt").exists(), "the hook ran");
    assert_eq!(
        fx.state_json()["hook_dispatches"],
        1,
        "the gate saw the dispatch"
    );

    // The connection carries no scope unit and says how it ran.
    let state = fx.state_json();
    let launcher = state["connections"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["role"] == "launcher")
        .expect("a launcher connection");
    assert_eq!(launcher["mode"], "degraded");
    assert!(launcher["scope_unit"].is_null(), "{launcher}");
    assert_eq!(probe_at(&run_lock_path(&fx.run_dir)), LockProbe::Free);
}

#[test]
fn the_flag_is_never_implied_by_a_failed_probe() {
    // The prohibition: no automatic fallback. A host that fails every probe and a
    // launch without the flag starts nothing, in either launch mode.
    let fx = Fixture::new();
    fx.write_scenario(WELL_MANNERED_SCENARIO);
    break_bwrap(&fx);
    write_executable(&fx.shims(), "systemd-run", "#!/bin/sh\nexit 127\n");
    write_executable(&fx.shims(), "systemctl", "#!/bin/sh\nexit 1\n");

    for out in [
        fx.launch(&[]),
        fx.launch_command_raw(launcher_support::ALIAS, RUN_ID, &[])
            .output()
            .unwrap(),
    ] {
        assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
        let err = stderr(&out);
        for piece in ["bwrap", "systemd-run", "user-systemd", "--unsandboxed"] {
            assert!(err.contains(piece), "{piece}: {err}");
        }
    }
    assert!(!fx.run_dir.join("state.json").exists());
    assert_eq!(fx.log(), "");
    assert_eq!(launch_lines(&fx), Vec::<String>::new());
    assert!(!fx.any_driver_alive());
}

/// Whether `pid` is a live (non-zombie) process.
fn alive(pid: u32) -> bool {
    fs::read_to_string(format!("/proc/{pid}/stat")).is_ok_and(|stat| {
        stat.rsplit_once(") ")
            .is_some_and(|(_, rest)| !rest.starts_with('Z'))
    })
}

fn pid_file(fx: &Fixture, name: &str) -> Option<u32> {
    fs::read_to_string(fx.rec_dir().join(name))
        .ok()
        .and_then(|text| text.trim().parse().ok())
}

/// The driver pid (the group leader) as the lock holder record names the launcher,
/// not the driver: the fake driver writes its own pid.
const GROUP_SCENARIO: &str = r#"
driverctl log start --mode headless --loop-mode loop
echo $$ > "$FAKE_CLAUDE_DIR/leader.pid"
sleep 600 &
echo $! > "$FAKE_CLAUDE_DIR/bg.pid"
touch "$FAKE_CLAUDE_DIR/started"
wait
"#;

#[test]
fn unsandboxed_stop_kills_the_group() {
    let fx = Fixture::new();
    fx.write_scenario(GROUP_SCENARIO);
    break_bwrap(&fx);
    let launcher = fx.spawn_launch(&["--unsandboxed"]);
    assert!(
        wait_until(Duration::from_secs(10), || fx.scenario_started()),
        "the launcher never reached its driver"
    );
    let leader = pid_file(&fx, "leader.pid").expect("the driver wrote its pid");
    let background = pid_file(&fx, "bg.pid").expect("the driver started a background job");
    assert!(alive(leader) && alive(background));

    let stop = fx.stop_command(RUN_ID, &[]).output().expect("driver stop");
    assert!(stop.status.success(), "{}", stderr(&stop));
    assert!(
        stdout(&stop).contains("stopped the launcher"),
        "{}",
        stdout(&stop)
    );
    let out = launcher.wait_with_output().expect("the launcher ends");
    assert!(out.status.success(), "{}", stderr(&out));

    assert!(
        wait_until(Duration::from_secs(5), || !alive(leader)
            && !alive(background)),
        "leader alive={} background alive={}",
        alive(leader),
        alive(background)
    );
    let log = fx.log();
    assert!(
        log.lines()
            .last()
            .unwrap_or_default()
            .contains("RUN | launch-end | status=stopped | reason=stop:user"),
        "{log}"
    );
    assert_eq!(fx.state_json()["status"], "stopped");
    assert_eq!(fx.state_json()["status_reason"], "stop:user");
    assert_eq!(probe_at(&run_lock_path(&fx.run_dir)), LockProbe::Free);
}

#[test]
fn unsandboxed_stop_escalates_to_kill_for_a_driver_that_ignores_term() {
    let fx = Fixture::new();
    fx.write_scenario(
        r#"
trap '' TERM
driverctl log start --mode headless --loop-mode loop
echo $$ > "$FAKE_CLAUDE_DIR/leader.pid"
touch "$FAKE_CLAUDE_DIR/started"
while :; do sleep 1; done
"#,
    );
    break_bwrap(&fx);
    let launcher = fx.spawn_launch(&["--unsandboxed"]);
    assert!(
        wait_until(Duration::from_secs(10), || fx.scenario_started()),
        "the launcher never reached its driver"
    );
    let leader = pid_file(&fx, "leader.pid").expect("the driver wrote its pid");

    let started = std::time::Instant::now();
    let stop = fx.stop_command(RUN_ID, &[]).output().expect("driver stop");
    assert!(stop.status.success(), "{}", stderr(&stop));
    let out = launcher.wait_with_output().expect("the launcher ends");
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        started.elapsed() >= Duration::from_secs(10),
        "TERM is given its grace before KILL: {:?}",
        started.elapsed()
    );
    assert!(
        wait_until(Duration::from_secs(5), || !alive(leader)),
        "a SIGTERM-proof driver survived the escalation"
    );
    assert_eq!(fx.state_json()["status_reason"], "stop:user");
}

#[test]
fn unsandboxed_normal_exit_leaves_no_group_descendant() {
    let fx = Fixture::new();
    fx.write_scenario(
        r#"
driverctl log start --mode headless --loop-mode loop
sleep 600 &
echo $! > "$FAKE_CLAUDE_DIR/bg.pid"
driverctl log end --reason stop
printf '%s\n' '{"type":"result","subtype":"success"}'
"#,
    );
    break_bwrap(&fx);
    let out = fx.launch(&["--unsandboxed"]);
    assert!(out.status.success(), "{}", stderr(&out));
    let background = pid_file(&fx, "bg.pid").expect("the driver started a background job");
    assert!(
        wait_until(Duration::from_secs(5), || !alive(background)),
        "a descendant outlived the launch"
    );
}

#[test]
fn unsandboxed_run_holds_the_lock_like_a_sandboxed_one() {
    let fx = Fixture::new();
    fx.write_scenario(WAITING_SCENARIO);
    break_bwrap(&fx);
    let launcher = fx.spawn_launch(&["--unsandboxed"]);
    assert!(
        wait_until(Duration::from_secs(10), || fx.scenario_started()),
        "the launcher never reached its driver"
    );
    assert_eq!(probe_at(&run_lock_path(&fx.run_dir)), LockProbe::Held);
    let second = fx.launch(&["--unsandboxed"]);
    assert_eq!(second.status.code(), Some(3), "{}", stderr(&second));
    fx.release();
    let out = launcher.wait_with_output().expect("the launcher ends");
    assert!(out.status.success(), "{}", stderr(&out));
}

// ---------------------------------------------------------------- the recorded posture (D-30)

#[test]
fn unsandboxed_launch_records_degraded_posture() {
    let fx = Fixture::new();
    fx.write_scenario(WELL_MANNERED_SCENARIO);
    break_bwrap(&fx);

    let out = fx.launch(&["--unsandboxed"]);
    assert!(out.status.success(), "{}", stderr(&out));

    let view = read_driver_run(&fx.run_dir).expect("the run reads back");
    assert_eq!(view.frontmatter.sandbox.as_deref(), Some("degraded"));
    assert_eq!(
        view.frontmatter.sandbox_missing,
        Some(vec!["bwrap".to_owned()])
    );
    let state = fx.state_json();
    assert_eq!(state["sandbox"], "degraded");
    assert_eq!(state["sandbox_missing"], serde_json::json!(["bwrap"]));
    let direction = fx.run_file("DIRECTION.md");
    assert!(
        direction.contains("\nsandbox: \"degraded\"\n"),
        "{direction}"
    );
    assert!(
        direction.contains("\nsandbox_missing: [\"bwrap\"]\n"),
        "{direction}"
    );
}

#[test]
fn unsandboxed_on_a_complete_host_is_degraded_with_nothing_missing() {
    // The flag is the operator's choice (D-29): it degrades a host that could sandbox,
    // and the run says so without a misleading missing list.
    let fx = Fixture::new();
    fx.write_scenario(WELL_MANNERED_SCENARIO);

    let out = fx.launch(&["--unsandboxed"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(launch_lines(&fx), Vec::<String>::new(), "{}", fx.shim_log());
    let view = read_driver_run(&fx.run_dir).expect("the run reads back");
    assert_eq!(view.frontmatter.sandbox.as_deref(), Some("degraded"));
    assert_eq!(view.frontmatter.sandbox_missing, None);
    assert!(fx.log().contains("RUN | launch | n=1 | mode=degraded"));
}

#[test]
fn sandboxed_launch_records_bwrap_scope_posture() {
    let fx = Fixture::new();
    fx.write_scenario(WELL_MANNERED_SCENARIO);

    let out = fx.launch(&[]);
    assert!(out.status.success(), "{}", stderr(&out));

    let view = read_driver_run(&fx.run_dir).expect("the run reads back");
    assert_eq!(view.frontmatter.sandbox.as_deref(), Some("bwrap-scope"));
    assert_eq!(view.frontmatter.sandbox_missing, None);
    let direction = fx.run_file("DIRECTION.md");
    assert!(
        direction.contains("\nsandbox: \"bwrap-scope\"\n"),
        "{direction}"
    );
    assert!(!direction.contains("sandbox_missing"), "{direction}");
    assert_eq!(fx.state_json()["sandbox"], "bwrap-scope");
}
