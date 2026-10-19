//! `driver stop`: end a run from the CLI or the TUI (Phase 28; LNCH-01 stop half,
//! D-09, D-10).
//!
//! One library function, [`stop_run`], shared by the command line and the Driver
//! screen. There are two situations and the **run lock** tells them apart, never
//! a pid:
//!
//! * **The lock is held** (a live launcher). The lock-holder record names the
//!   launcher's pid and that pid is used for exactly one thing: it is the target
//!   of a `SIGTERM`. The launcher then stops the sandbox scope with
//!   `systemctl --user stop` (systemd signals every process of the cgroup and
//!   escalates to `SIGKILL` itself), records `stopped` with reason `stop:user`
//!   and releases the lock last. This function waits for the lock to go free.
//! * **The lock is free** (no launcher). If the last recorded scope is nevertheless
//!   still active, the launcher died and orphaned it: the scope is stopped by name
//!   and **nothing is written** (D-09). The reader keeps deriving `crashed` for
//!   such a run, which is the truth.
//!
//! The pid is never a liveness signal and no pid file is read or written
//! (LNCH-02, T-28-05): a recycled pid can at worst receive one `SIGTERM` in the
//! instant between the lock probe and the signal, and pid `0`, `1` and our own
//! are refused outright.

use std::fs;
use std::path::Path;
use std::time::{Duration, Instant};

use rustix::process::{kill_process, Pid, Signal};

use super::process::run_bounded;
use super::LauncherError;
use crate::driver::lock::{probe_at, read_holder_at, run_lock_path, LockProbe};
use crate::driverctl::serve::{valid_run_id, validate_run_dir, DRIVERS_REL};
use crate::driverctl::store::RunState;
use crate::state_reader::driver_frontmatter::{read_driver_run, DerivedStatus};

/// How long `systemctl --user is-active` may take.
const PROBE_BUDGET: Duration = Duration::from_secs(10);

/// How long `systemctl --user stop` may take (the unit carries
/// `TimeoutStopSec=15`, so systemd escalates to `SIGKILL` well inside this).
const STOP_BUDGET: Duration = Duration::from_secs(60);

/// How long to wait for the holder record of a lock that was just seen held.
const HOLDER_WAIT: Duration = Duration::from_secs(2);

/// How often the lock is probed while waiting for the launcher to finish.
const POLL: Duration = Duration::from_millis(100);

/// What [`stop_run`] did.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StopReport {
    /// The launcher was signalled and the lock was released within the wait.
    SignalledLauncher { pid: u32, final_status: String },
    /// The launcher was signalled but still holds the lock after the wait.
    StillRunning { pid: u32 },
    /// No launcher holds the run, but its recorded scope was alive; it was
    /// stopped by name and nothing was recorded.
    ScopeStopped { unit: String },
    /// Nothing to stop.
    NotRunning,
}

impl std::fmt::Display for StopReport {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            StopReport::SignalledLauncher { pid, final_status } => write!(
                f,
                "stopped the launcher (pid {pid}); the run is now {final_status}"
            ),
            StopReport::StillRunning { pid } => write!(
                f,
                "signalled the launcher (pid {pid}) but it is still running; \
                 it is stopping the sandbox, check again shortly"
            ),
            StopReport::ScopeStopped { unit } => write!(
                f,
                "no launcher holds the run; stopped the scope {unit} (nothing was recorded)"
            ),
            StopReport::NotRunning => f.write_str("the run is not running"),
        }
    }
}

fn io_error(context: &str, err: impl std::fmt::Display) -> LauncherError {
    LauncherError::Io {
        detail: format!("{context}: {err}"),
    }
}

/// Stop the run `run_id` of `project`, waiting up to `wait` for a live launcher
/// to finish.
pub fn stop_run(project: &Path, run_id: &str, wait: Duration) -> Result<StopReport, LauncherError> {
    if !valid_run_id(run_id) {
        return Err(LauncherError::InvalidRun {
            detail: format!("run id {run_id:?} is not a plain run id"),
        });
    }
    let requested = project.join(DRIVERS_REL).join(run_id);
    let (run_dir, _) = validate_run_dir(project, &requested)?;
    let lock = run_lock_path(&run_dir);
    match probe_at(&lock) {
        LockProbe::Held => signal_launcher(&run_dir, &lock, wait),
        LockProbe::Free | LockProbe::Absent => stop_orphaned_scope(&run_dir),
        LockProbe::Unknown(errno) => Err(io_error("could not probe the run lock", errno)),
    }
}

/// The pid a `SIGTERM` may be sent to, or why not.
fn signal_target(pid: u32) -> Result<Pid, LauncherError> {
    if pid <= 1 || pid == std::process::id() {
        return Err(LauncherError::InvalidRun {
            detail: format!("the run lock names pid {pid}, which is not a launcher"),
        });
    }
    i32::try_from(pid)
        .ok()
        .and_then(Pid::from_raw)
        .ok_or_else(|| LauncherError::InvalidRun {
            detail: format!("the run lock names pid {pid}, which is not a process id"),
        })
}

/// A live launcher: `SIGTERM` it and wait for the lock to go free.
fn signal_launcher(
    run_dir: &Path,
    lock: &Path,
    wait: Duration,
) -> Result<StopReport, LauncherError> {
    // The holder record is written right after the lock is taken; give a launcher
    // caught in that instant a moment rather than reporting a missing record.
    let deadline = Instant::now() + HOLDER_WAIT;
    let holder = loop {
        if let Some(holder) = read_holder_at(lock) {
            break holder;
        }
        if Instant::now() >= deadline {
            return Err(LauncherError::Io {
                detail: "the run lock is held but its holder record is unreadable".to_owned(),
            });
        }
        std::thread::sleep(Duration::from_millis(50));
    };
    let pid = holder.pgid;
    let target = signal_target(pid)?;

    // T-28-05: signal only while the lock is still held, so the pid is the
    // launcher's and not a stranger's that reused it.
    if probe_at(lock) != LockProbe::Held {
        return Ok(finished(run_dir, pid));
    }
    match kill_process(target, Signal::TERM) {
        Ok(()) => {}
        // It exited between the probe and the signal: that is the outcome we want.
        Err(rustix::io::Errno::SRCH) => return Ok(finished(run_dir, pid)),
        Err(errno) => return Err(io_error(&format!("could not signal pid {pid}"), errno)),
    }

    let deadline = Instant::now() + wait;
    loop {
        if probe_at(lock) != LockProbe::Held {
            return Ok(finished(run_dir, pid));
        }
        if Instant::now() >= deadline {
            return Ok(StopReport::StillRunning { pid });
        }
        std::thread::sleep(POLL);
    }
}

/// The launcher is gone: report where the run ended up.
fn finished(run_dir: &Path, pid: u32) -> StopReport {
    let final_status = match read_driver_run(run_dir).map(|view| view.derived) {
        Ok(DerivedStatus::AsWritten(status)) => status.as_str().to_owned(),
        Ok(DerivedStatus::Crashed { .. }) => "crashed".to_owned(),
        Ok(DerivedStatus::NotLaunched) => "starting".to_owned(),
        Ok(_) | Err(_) => "unknown".to_owned(),
    };
    StopReport::SignalledLauncher { pid, final_status }
}

/// Whether `unit` is a scope this launcher could have created: the name goes to
/// `systemctl` as an argument, so nothing option-shaped may reach it.
fn plausible_scope(unit: &str) -> bool {
    // A string rather than char arms: `crate::text` owns the one executable
    // spelling of the identity alphabet's punctuation clause, and this is a
    // different (systemd unit name) alphabet.
    const UNIT_PUNCTUATION: &str = "-_.";
    unit.starts_with("gsdmm-driver-")
        && unit.ends_with(".scope")
        && unit
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || UNIT_PUNCTUATION.contains(c))
}

/// No launcher: stop the last recorded scope by name if it is still alive.
fn stop_orphaned_scope(run_dir: &Path) -> Result<StopReport, LauncherError> {
    let state_path = run_dir.join("state.json");
    let text = match fs::read_to_string(&state_path) {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(StopReport::NotRunning),
        Err(e) => return Err(io_error(&format!("read {}", state_path.display()), e)),
    };
    let state: RunState = serde_json::from_str(&text).map_err(|e| LauncherError::Store {
        detail: format!("{} does not parse: {e}", state_path.display()),
    })?;
    let Some(unit) = state
        .connections
        .iter()
        .rev()
        .find(|c| c.role == "launcher")
        .and_then(|c| c.scope_unit.clone())
    else {
        return Ok(StopReport::NotRunning);
    };
    if !plausible_scope(&unit) {
        return Err(LauncherError::Store {
            detail: format!("the recorded scope unit {unit:?} is not a launcher scope"),
        });
    }

    let active = run_bounded(
        "systemctl",
        &["--user".into(), "is-active".into(), unit.clone().into()],
        None,
        &[],
        PROBE_BUDGET,
    )
    .is_some_and(|out| String::from_utf8_lossy(&out.stdout).trim() == "active");
    if !active {
        return Ok(StopReport::NotRunning);
    }
    let stopped = run_bounded(
        "systemctl",
        &["--user".into(), "stop".into(), unit.clone().into()],
        None,
        &[],
        STOP_BUDGET,
    );
    match stopped {
        Some(out) if out.status.success() => Ok(StopReport::ScopeStopped { unit }),
        Some(out) => Err(LauncherError::Spawn {
            detail: format!("systemctl --user stop {unit} exited with {}", out.status),
        }),
        None => Err(LauncherError::Spawn {
            detail: format!("systemctl --user stop {unit} did not finish in {STOP_BUDGET:?}"),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_a_real_launcher_pid_is_a_signal_target() {
        for refused in [0, 1, std::process::id()] {
            assert!(
                matches!(
                    signal_target(refused),
                    Err(LauncherError::InvalidRun { .. })
                ),
                "{refused}"
            );
        }
        assert!(matches!(
            signal_target(u32::MAX),
            Err(LauncherError::InvalidRun { .. })
        ));
        assert!(signal_target(4242).is_ok());
    }

    #[test]
    fn only_launcher_shaped_scope_names_reach_systemctl() {
        assert!(plausible_scope("gsdmm-driver-r28-0123abcd-1.scope"));
        for bad in [
            "--user",
            "-h.scope",
            "gsdmm-driver-r28-1",
            "other.scope",
            "gsdmm-driver-a b.scope",
            "gsdmm-driver-x;rm.scope",
            "",
        ] {
            assert!(!plausible_scope(bad), "{bad:?}");
        }
    }

    #[test]
    fn every_report_arm_names_what_happened() {
        let signalled = StopReport::SignalledLauncher {
            pid: 7,
            final_status: "stopped".to_owned(),
        };
        assert!(signalled.to_string().contains("stopped the launcher"));
        assert!(signalled.to_string().contains("stopped"));
        assert!(StopReport::StillRunning { pid: 7 }
            .to_string()
            .contains("still running"));
        assert!(StopReport::ScopeStopped {
            unit: "u.scope".to_owned()
        }
        .to_string()
        .contains("stopped the scope u.scope"));
        assert!(StopReport::NotRunning.to_string().contains("not running"));
    }
}
