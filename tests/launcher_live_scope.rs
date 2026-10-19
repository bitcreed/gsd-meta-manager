#![cfg(target_os = "linux")]
// ============================================================================
// LIVE proofs of the launcher's stop and wall-clock backstop (Phase 28, plan
// 28-02; D-09, D-10, D-13). Opt-in: every test is `#[ignore]` because it needs
// a working `bwrap` and a user systemd manager, neither of which a CI runner has.
//
//   cargo test --test launcher_live_scope -- --ignored --nocapture
//
// Run it OUTSIDE any tool sandbox. A test that cannot run prints a `SKIP:` line
// and returns; a `SKIP:` line is no evidence, it means this host proved nothing.
//
// Unlike the CI tests, `bwrap`, `systemd-run` and `systemctl` here are the
// host's real binaries; only `claude` is fake (no API spend). The fixture lives
// under `CARGO_TARGET_TMPDIR` because bwrap's `--tmpfs /tmp` hides `/tmp`.
// ============================================================================

#[path = "launcher_support/mod.rs"]
mod launcher_support;

use std::fs;
use std::path::PathBuf;
use std::process::{Command, Stdio};
use std::time::{Duration, Instant};

use gsd_meta_manager::driver::lock::{probe_at, run_lock_path, LockProbe};
use gsd_meta_manager::launcher::sandbox::systemd_run_argv;
use launcher_support::{wait_until, write_executable, Fixture, RUN_ID};

/// The fake driver: answers `--version`, then starts a `setsid` grandchild and a
/// `bash -c` grandchild inside the sandbox and waits. Neither is in the driver's
/// process group, which is the point: only the cgroup reaches them.
const LIVE_CLAUDE: &str = r#"#!/bin/sh
if [ "$1" = "--version" ]; then echo "2.1.303 (Claude Code)"; exit 0; fi
setsid sleep 600 &
bash -c 'sleep 600' &
touch "$CLAUDE_CONFIG_DIR/live-started"
wait
"#;

fn run_ok(program: &str, args: &[&str]) -> Option<String> {
    let out = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .output()
        .ok()?;
    Some(String::from_utf8_lossy(&out.stdout).trim().to_owned())
}

/// Why this host cannot run the live proofs, if it cannot.
fn skip_reason() -> Option<String> {
    let bwrap = Command::new("bwrap")
        .args(["--ro-bind", "/", "/", "--dev", "/dev", "--", "true"])
        .stdin(Stdio::null())
        .output();
    match bwrap {
        Ok(out) if out.status.success() => {}
        Ok(out) => {
            return Some(format!(
                "bwrap cannot run a trivial command here ({}): {}",
                out.status,
                String::from_utf8_lossy(&out.stderr).trim()
            ))
        }
        Err(e) => return Some(format!("bwrap is not runnable: {e}")),
    }
    match run_ok("systemctl", &["--user", "is-system-running"]).as_deref() {
        Some("running" | "degraded") => None,
        other => Some(format!(
            "the user systemd manager is not usable (is-system-running: {other:?})"
        )),
    }
}

fn target_tmp() -> PathBuf {
    PathBuf::from(env!("CARGO_TARGET_TMPDIR")).join("launcher-live")
}

fn real_runtime_dir() -> PathBuf {
    PathBuf::from(std::env::var_os("XDG_RUNTIME_DIR").expect("XDG_RUNTIME_DIR is set"))
}

fn unit_state(unit: &str) -> String {
    run_ok("systemctl", &["--user", "is-active", unit]).unwrap_or_default()
}

/// The host pids of every process in the unit's cgroup.
fn cgroup_pids(unit: &str) -> Vec<u32> {
    let Some(group) = run_ok(
        "systemctl",
        &["--user", "show", "-p", "ControlGroup", "--value", unit],
    ) else {
        return Vec::new();
    };
    if group.is_empty() {
        return Vec::new();
    }
    fs::read_to_string(format!("/sys/fs/cgroup{group}/cgroup.procs"))
        .unwrap_or_default()
        .lines()
        .filter_map(|l| l.trim().parse().ok())
        .collect()
}

/// Stops the scope on drop, so a failing assertion leaves no `sleep 600` behind.
struct ScopeGuard(String);

impl Drop for ScopeGuard {
    fn drop(&mut self) {
        let _ = Command::new("systemctl")
            .args(["--user", "stop", &self.0])
            .stdin(Stdio::null())
            .output();
    }
}

/// Whether `pid` is a live (non-zombie) process.
fn process_alive(pid: u32) -> bool {
    match fs::read_to_string(format!("/proc/{pid}/stat")) {
        Ok(stat) => {
            let after = stat.rfind(')').map_or("", |i| &stat[i + 1..]);
            after.split_whitespace().next() != Some("Z")
        }
        Err(_) => false,
    }
}

#[test]
#[ignore = "needs bwrap and a user systemd; run with -- --ignored --nocapture"]
fn stop_kills_every_process_of_the_scope() {
    if let Some(reason) = skip_reason() {
        println!("SKIP: {reason}");
        return;
    }
    let mut fx = Fixture::new_in(&target_tmp());
    fx.runtime_dir = real_runtime_dir();
    // Only `claude` is fake: the host's real systemd-run, bwrap and systemctl run.
    for fake in ["systemd-run", "bwrap", "systemctl"] {
        fs::remove_file(fx.shims().join(fake)).expect("remove a fake tool");
    }
    write_executable(&fx.shims(), "claude", LIVE_CLAUDE);

    let launcher = fx.spawn_launch(&[]);
    let started = fx.config_dir().join("live-started");
    assert!(
        wait_until(Duration::from_secs(30), || started.exists()),
        "the sandboxed driver never started; launcher log: {}",
        fs::read_to_string(fx.state_dir().join("launch-1.jsonl")).unwrap_or_default()
    );

    let unit = fx.state_json()["connections"]
        .as_array()
        .and_then(|cs| cs.iter().find_map(|c| c["scope_unit"].as_str()))
        .expect("the scope unit is recorded on the run")
        .to_owned();
    let _guard = ScopeGuard(unit.clone());
    let mut pids = Vec::new();
    assert!(
        wait_until(Duration::from_secs(10), || {
            pids = cgroup_pids(&unit);
            pids.len() >= 3
        }),
        "the scope holds fewer than 3 processes: {pids:?}"
    );
    println!("LIVE unit: {unit}");
    println!("LIVE state before stop: {}", unit_state(&unit));
    println!("LIVE cgroup pids before stop: {pids:?}");
    for pid in &pids {
        let comm = fs::read_to_string(format!("/proc/{pid}/comm")).unwrap_or_default();
        println!("LIVE   pid {pid}: {}", comm.trim());
    }
    assert_eq!(unit_state(&unit), "active");

    let stop = fx
        .stop_command(RUN_ID, &[])
        .output()
        .expect("spawn driver stop");
    println!(
        "LIVE driver stop: {}",
        String::from_utf8_lossy(&stop.stdout)
    );
    assert!(
        stop.status.success(),
        "driver stop failed: {}",
        String::from_utf8_lossy(&stop.stderr)
    );
    let out = launcher.wait_with_output().expect("the launcher ends");
    assert!(
        out.status.success(),
        "the launcher failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    println!("LIVE state after stop: {}", unit_state(&unit));
    assert_ne!(unit_state(&unit), "active", "the scope is still active");
    assert!(
        wait_until(Duration::from_secs(10), || !pids
            .iter()
            .any(|p| process_alive(*p))),
        "processes of the scope survived the stop: {:?}",
        pids.iter()
            .filter(|p| process_alive(**p))
            .collect::<Vec<_>>()
    );
    println!("LIVE no pid of {pids:?} is alive");
    assert!(
        cgroup_pids(&unit).is_empty(),
        "the cgroup still has members"
    );
    assert_eq!(probe_at(&run_lock_path(&fx.run_dir)), LockProbe::Free);
    let log = fx.log();
    assert!(
        log.lines()
            .last()
            .unwrap_or_default()
            .contains("RUN | launch-end | status=stopped | reason=stop:user"),
        "{log}"
    );
}

#[test]
#[ignore = "needs a user systemd; run with -- --ignored --nocapture"]
fn runtime_max_sec_backstop_stops_a_scope() {
    if let Some(reason) = skip_reason() {
        println!("SKIP: {reason}");
        return;
    }
    let unit = format!("gsdmm-driver-live-rtmax-{}-1", std::process::id());
    let argv = systemd_run_argv(&unit, Some(3), &["sleep".into(), "60".into()]);
    let (program, args) = argv.split_first().expect("a command line");
    let _guard = ScopeGuard(format!("{unit}.scope"));
    let started = Instant::now();
    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn systemd-run");
    let finished = wait_until(Duration::from_secs(15), || {
        child.try_wait().expect("try_wait").is_some()
    });
    let elapsed = started.elapsed();
    if !finished {
        let _ = Command::new("systemctl")
            .args(["--user", "stop", &format!("{unit}.scope")])
            .output();
        let _ = child.kill();
        let _ = child.wait();
    }
    println!("LIVE RuntimeMaxSec=3 scope ended after {elapsed:?}");
    println!("LIVE state after: {}", unit_state(&format!("{unit}.scope")));
    assert!(
        finished,
        "RuntimeMaxSec=3 did not stop the scope within 15 s"
    );
    assert!(
        elapsed >= Duration::from_secs(2),
        "the scope ended before its limit: {elapsed:?}"
    );
    assert_ne!(unit_state(&format!("{unit}.scope")), "active");
}
