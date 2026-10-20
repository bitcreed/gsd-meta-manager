//! The sandboxed driver launcher (Phase 28; LNCH-01..08, D-01..D-34).
//!
//! `gsd-meta-manager driver launch` runs one headless `claude -p` driver inside
//! `bwrap` under `systemd-run --user --scope`, from a process that owns
//! everything the model must not be trusted with. The facts every later plan in
//! the phase builds against, each with the decision that settled it:
//!
//! 1. **The launcher is the host-side owner of the run (D-06).** It holds the run
//!    lock (`<run-dir>/launcher.lock`), embeds the Phase 27
//!    [`RunStore`](crate::driverctl::store::RunStore) and serves its socket from a
//!    thread, seeds the per-run `CLAUDE_CONFIG_DIR`, resolves the OAuth token and
//!    builds the sandbox. The driver inside the sandbox reaches the store only
//!    through the `driverctl` shim and the socket.
//! 2. **It is synchronous: std threads only, never called from an `async fn`.**
//!    The store takes `flock` and does file I/O (driverctl fact 5), and
//!    `tests/async_blocking_guard.rs` walks `src/` for the shape a violation
//!    would take. `main` dispatches `driver` before the tokio runtime exists.
//! 3. **Every process the launcher starts goes through [`process`] (RESEARCH §6).**
//!    That is the one launcher entry in the spawn allowlist
//!    (`tests/spawn_seam_guard.rs`); the agent is reached only through `bwrap`.
//! 4. **No pid file is ever written (D-07, LNCH-02).** The lock is the identity of
//!    a live launcher and `flock` is the enforcement; the pid recorded in the lock
//!    holder is advisory.
//! 5. **Lock first, lock last (D-05, R4).** The lock is taken before the launcher
//!    writes anything and is released last: the store drops after the socket is
//!    removed and after the terminal status is recorded, so the reader never sees
//!    a finished run with a free lock and a `running` status.
//! 6. **The argv builders are pure ([`sandbox`], D-20).** They take every host fact
//!    as a parameter, so the bwrap, systemd-run and claude command lines are
//!    golden-testable on any runner, one without bwrap or a user systemd
//!    included. `sandbox` is therefore the one submodule that is not Linux-gated.
//! 7. **The token never leaves the child's environment (D-19, D-22).** It is not
//!    in any argv, in `state.json`, in `LOG.md`, in the launch log or in any file
//!    the launcher writes; [`auth::LaunchToken`]'s `Debug` redacts it.
//! 8. **Headless, no TTY (LNCH-08).** The driver's stdin is `/dev/null` and its
//!    stdout is a file; there is no `tmux` argv and no terminal on the launch or
//!    control path.
//!
//! 9. **The run lifecycle is the launcher's (28-02).** `run` holds the lock across
//!    `fresh-exit` relaunches, stops the scope on a `SIGTERM` (`stop` is the
//!    sender), and detaches into its own session unless `--foreground`.
//!
//! 10. **No sandbox, no run, unless the launch says so (28-04, D-28..D-30).**
//!     [`host`] probes the host before the lock; a host that cannot sandbox is
//!     refused with a typed error naming what is missing and `--unsandboxed`. Only
//!     that flag, per launch, runs the driver as a supervised process group, and the
//!     run records `sandbox: degraded` so it is never shown as sandboxed.
//!
//! # What this module root leaves to later plans
//!
//! The TUI run selection (28-07) and the acceptance harness (28-09).
//!
//! 11. **The hard caps are enforced from outside the sandbox (28-05, D-12..D-16).**
//!     [`gate`] is the pure dispatch/gap-round decision installed on the embedded
//!     store; the monitor in [`run`] trips and kills (wall clock, gap rounds) and
//!     persists the active seconds.
//!
//! 12. **A run's commits leave the machine only through [`push`] (28-08, D-24..D-26).**
//!     The step runs on the host from a launcher-owned bare mirror whose push URL was
//!     pinned before any sandbox existed, sends one refspec into
//!     `refs/heads/gsd-auto/<alias>/<run-id>` after the SAFE-03 scan of the tip's
//!     tree, and opens a pull request only when the user enabled `driver.open_pr`.

pub mod prompt;
pub mod sandbox;

#[cfg(target_os = "linux")]
pub mod auth;
#[cfg(target_os = "linux")]
pub mod config_dir;
#[cfg(target_os = "linux")]
pub mod gate;
#[cfg(target_os = "linux")]
pub mod host;
#[cfg(target_os = "linux")]
pub mod outcome;
#[cfg(target_os = "linux")]
pub mod process;
#[cfg(target_os = "linux")]
pub mod push;
#[cfg(target_os = "linux")]
pub mod run;
#[cfg(target_os = "linux")]
pub mod stop;

/// Why a launch did not run (or could not continue).
///
/// Every variant but [`UnsupportedPlatform`](Self::UnsupportedPlatform) carries
/// the detail text; [`exit_code`](Self::exit_code) is the process exit code of
/// `driver launch`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LauncherError {
    /// The run id or the run directory is not acceptable (T-28-04).
    InvalidRun { detail: String },
    /// Another live launcher or `driverctl serve` holds the run lock (D-07).
    Locked { detail: String },
    /// No usable OAuth token (D-22).
    Auth { detail: String },
    /// A process the launcher needs could not be started.
    Spawn { detail: String },
    /// A file-system, thread or socket fault.
    Io { detail: String },
    /// The run store could not open or write.
    Store { detail: String },
    /// The launcher needs Linux (`flock`, `renameat2`, `bwrap`, `systemd-run`).
    UnsupportedPlatform,
    /// The detached launcher exited before it held the run lock (D-04). `code` is
    /// its own exit code (`3` for a run that is already launched); `detail` is
    /// what it wrote to `launcher.log`.
    DaemonExited { code: i32, detail: String },
    /// The host cannot sandbox the driver and the launch did not carry
    /// `--unsandboxed` (D-28). `missing` names each piece: `bwrap`, `systemd-run`,
    /// `user-systemd`.
    SandboxUnavailable { missing: Vec<String> },
    /// `claude` is absent or older than the hook `onFailure` minimum (D-28); refused
    /// in both modes.
    ClaudeVersion { detail: String },
    /// The run's active wall-clock cap is already used up (D-13): nothing was
    /// written and no driver was started.
    CapExhausted { detail: String },
}

impl LauncherError {
    /// The process exit code: `3` for a held lock (the spike launcher's refusal
    /// code), the detached launcher's own code for [`DaemonExited`](Self::DaemonExited),
    /// `1` for everything else.
    pub fn exit_code(&self) -> i32 {
        match self {
            LauncherError::Locked { .. } => 3,
            LauncherError::DaemonExited { code, .. } => *code,
            _ => 1,
        }
    }
}

impl std::fmt::Display for LauncherError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LauncherError::InvalidRun { detail } => write!(f, "invalid run: {detail}"),
            LauncherError::Locked { detail } => write!(f, "run is already launched: {detail}"),
            LauncherError::Auth { detail } => write!(f, "auth: {detail}"),
            LauncherError::Spawn { detail } => write!(f, "could not start a process: {detail}"),
            LauncherError::Io { detail } => write!(f, "I/O failure: {detail}"),
            LauncherError::Store { detail } => write!(f, "run store: {detail}"),
            LauncherError::UnsupportedPlatform => {
                f.write_str("unsupported platform (the launcher needs Linux)")
            }
            LauncherError::SandboxUnavailable { missing } => write!(
                f,
                "sandbox unavailable: missing {}; rerun with --unsandboxed to launch without \
                 bwrap and a systemd scope (degraded)",
                missing.join(", ")
            ),
            LauncherError::ClaudeVersion { detail } => write!(f, "claude version: {detail}"),
            LauncherError::CapExhausted { detail } => write!(f, "cap exhausted: {detail}"),
            LauncherError::DaemonExited { code, detail } => {
                write!(
                    f,
                    "the detached launcher exited with code {code} before it held the run lock"
                )?;
                if !detail.is_empty() {
                    write!(f, ":\n{detail}")?;
                }
                Ok(())
            }
        }
    }
}

impl std::error::Error for LauncherError {}

#[cfg(target_os = "linux")]
impl From<crate::driverctl::serve::DriverctlError> for LauncherError {
    fn from(err: crate::driverctl::serve::DriverctlError) -> Self {
        use crate::driverctl::serve::DriverctlError;

        match err {
            DriverctlError::InvalidRunDir { detail } => LauncherError::InvalidRun { detail },
            DriverctlError::Locked { detail } => LauncherError::Locked { detail },
            DriverctlError::SocketPath { detail } => LauncherError::Io {
                detail: format!("socket path: {detail}"),
            },
            DriverctlError::Io { detail } => LauncherError::Io { detail },
            DriverctlError::Store { detail } => LauncherError::Store { detail },
        }
    }
}

#[cfg(target_os = "linux")]
impl From<crate::driverctl::store::StoreError> for LauncherError {
    fn from(err: crate::driverctl::store::StoreError) -> Self {
        LauncherError::Store {
            detail: err.to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_held_lock_exits_3_and_everything_else_exits_1() {
        assert_eq!(
            LauncherError::Locked {
                detail: "x".to_owned()
            }
            .exit_code(),
            3
        );
        for other in [
            LauncherError::InvalidRun {
                detail: "x".to_owned(),
            },
            LauncherError::Auth {
                detail: "x".to_owned(),
            },
            LauncherError::Spawn {
                detail: "x".to_owned(),
            },
            LauncherError::Io {
                detail: "x".to_owned(),
            },
            LauncherError::Store {
                detail: "x".to_owned(),
            },
            LauncherError::UnsupportedPlatform,
            LauncherError::SandboxUnavailable {
                missing: vec!["bwrap".to_owned()],
            },
            LauncherError::ClaudeVersion {
                detail: "x".to_owned(),
            },
            LauncherError::CapExhausted {
                detail: "x".to_owned(),
            },
        ] {
            assert_eq!(other.exit_code(), 1, "{other}");
        }
    }

    #[test]
    fn a_detached_launcher_that_exited_passes_its_own_code_through() {
        for code in [1, 3, 17] {
            let error = LauncherError::DaemonExited {
                code,
                detail: "boom".to_owned(),
            };
            assert_eq!(error.exit_code(), code);
            assert!(error.to_string().contains("boom"), "{error}");
        }
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn driverctl_errors_map_by_variant() {
        use crate::driverctl::serve::DriverctlError;

        let locked = LauncherError::from(DriverctlError::Locked {
            detail: "held".to_owned(),
        });
        assert_eq!(locked.exit_code(), 3);
        assert!(matches!(
            LauncherError::from(DriverctlError::InvalidRunDir {
                detail: "bad".to_owned()
            }),
            LauncherError::InvalidRun { .. }
        ));
        assert!(matches!(
            LauncherError::from(DriverctlError::SocketPath {
                detail: "long".to_owned()
            }),
            LauncherError::Io { .. }
        ));
        assert!(matches!(
            LauncherError::from(DriverctlError::Store {
                detail: "corrupt".to_owned()
            }),
            LauncherError::Store { .. }
        ));
    }
}
