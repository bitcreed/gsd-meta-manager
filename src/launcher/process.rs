//! The one launcher spawn file (Phase 28; RESEARCH §6, LNCH-08).
//!
//! Every process the launcher starts is built here: `systemd-run` (which runs
//! `bwrap`, which runs the agent), `systemctl`, and, in later plans, `git` for
//! the push mirror, `claude setup-token` and `gh`. That is why the spawn
//! allowlist (`tests/spawn_seam_guard.rs`) carries ONE entry for the launcher: a
//! spawn site nobody had to think about is how an agent comes to be launched
//! outside its sandbox. The agent is reached only through `bwrap` (or, in 28-04,
//! the explicit degraded spawn).
//!
//! Five shapes:
//!
//! * [`spawn_launch_child`], the long-lived driver child: a process-group leader
//!   with stdin `/dev/null` (no TTY, no `tmux`), stdout and stderr to a file, and
//!   EXACTLY the environment it is given.
//! * [`run_bounded`], a short host command whose stdout is read within a budget
//!   ([`crate::bounded_output::stdout_within`]).
//! * [`respawn_detached`], this binary again as the detached launcher (D-04).
//! * [`run_interactive`], a host command that talks to the user's terminal
//!   (`claude setup-token`, 28-06).
//! * [`run_bounded_with_stderr`], [`run_bounded`] that also keeps the last line of
//!   the command's stderr (the push step's `git` and `gh`, 28-08).

use std::collections::BTreeMap;
use std::ffi::OsString;
use std::fs::File;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::{Child, Command, ExitStatus, Stdio};
use std::time::Duration;

use crate::bounded_output::{stdout_within, stdout_within_with_stderr, BoundedOutput};

/// Start the driver command `argv` in `cwd`, headless.
///
/// * The environment is cleared and then set to `env`: the full child
///   environment is decided by the pure [`super::sandbox::child_env`], so the
///   scrub is golden-testable rather than a loop around a command builder.
///   `[inferred — audit]`: this deviates from the `env_remove` analog in
///   `src/executor/claude.rs`, which removes keys from an inherited environment.
/// * `stdin` is `/dev/null` and `stdout`/`stderr` are `out` (LNCH-08).
/// * `process_group(0)` makes the child a group leader, so the launcher can
///   signal the whole group (and `die-with-parent` is not the only defence).
///
/// With `PATH` in `env`, the program lookup uses the child's `PATH`.
pub fn spawn_launch_child(
    argv: &[OsString],
    env: &BTreeMap<OsString, OsString>,
    cwd: &Path,
    out: File,
) -> std::io::Result<Child> {
    let (program, args) = argv.split_first().ok_or_else(|| {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, "an empty command line")
    })?;
    let err = out.try_clone()?;
    let mut cmd = Command::new(program);
    cmd.args(args)
        .env_clear()
        .envs(env)
        .current_dir(cwd)
        .stdin(Stdio::null())
        .stdout(out)
        .stderr(err)
        .process_group(0);
    cmd.spawn()
}

/// Run a short host command and return its status and stdout, within `budget`.
///
/// The host environment is inherited, plus `extra_env`. `None` when the command
/// could not be started or did not finish in time (it is then killed).
pub fn run_bounded(
    program: &str,
    args: &[OsString],
    cwd: Option<&Path>,
    extra_env: &[(&str, &str)],
    budget: Duration,
) -> Option<BoundedOutput> {
    let mut cmd = Command::new(program);
    cmd.args(args);
    if let Some(cwd) = cwd {
        cmd.current_dir(cwd);
    }
    for (key, value) in extra_env {
        cmd.env(key, value);
    }
    stdout_within(&mut cmd, budget)
}

/// Start this binary again with `args` as the detached launcher (D-04).
///
/// * `stdin` is `/dev/null`, `stdout` and `stderr` are `log`, and the working
///   directory is `/`, so nothing of the caller's terminal or directory is kept
///   alive by the daemon.
/// * **No `process_group(0)`, deliberately:** a process-group leader cannot call
///   `setsid(2)` (`EPERM`), and the child's first act is `setsid` (the hidden
///   `--daemon-child` flag of `driver launch`). A fresh child that stays in the
///   caller's group is not a leader, so it can.
/// * The environment is inherited: the daemon is the same launcher the caller
///   would have run in the foreground.
pub fn respawn_detached(args: &[OsString], log: File) -> std::io::Result<Child> {
    let program = std::env::current_exe()?;
    let err = log.try_clone()?;
    let mut cmd = Command::new(program);
    cmd.args(args)
        .current_dir("/")
        .stdin(Stdio::null())
        .stdout(log)
        .stderr(err);
    cmd.spawn()
}

/// Run `program args` with the caller's own stdin, stdout and stderr, and wait
/// for it (28-06: `claude setup-token` asks the user to open a browser and
/// prints the token for them to copy).
///
/// Nothing is captured, so the launcher never holds what the program prints; the
/// user pastes the token back themselves (D-21). The environment is inherited:
/// this is the user's own login on the host, not a sandboxed run.
pub fn run_interactive(program: &str, args: &[&str]) -> std::io::Result<ExitStatus> {
    Command::new(program).args(args).status()
}

/// How much of the end of a command's stderr is read back.
const STDERR_TAIL_BYTES: u64 = 4096;

/// The `GIT_*` variables that would point a host `git` at a repository other than
/// the one the argv names. Removed from every `git` this module starts: the push
/// step names its repositories explicitly (`--git-dir`, `-C`), and an inherited
/// `GIT_DIR` would silently override both.
const GIT_REPO_ENV: &[&str] = &[
    "GIT_DIR",
    "GIT_WORK_TREE",
    "GIT_INDEX_FILE",
    "GIT_OBJECT_DIRECTORY",
    "GIT_ALTERNATE_OBJECT_DIRECTORIES",
    "GIT_COMMON_DIR",
    "GIT_NAMESPACE",
];

/// [`run_bounded`], plus the last non-empty line of the command's stderr.
///
/// stderr goes to a private temporary file (never a pipe, so it cannot fill and
/// block the child), and only its final [`STDERR_TAIL_BYTES`] are read back. The
/// caller decides what to do with the line: it can carry a URL with credentials in
/// it, so it must be redacted before it is printed or logged. For `git`, the
/// repository-selecting `GIT_*` variables are removed from the inherited
/// environment first.
pub fn run_bounded_with_stderr(
    program: &str,
    args: &[OsString],
    cwd: Option<&Path>,
    extra_env: &[(&str, &str)],
    budget: Duration,
) -> Option<(BoundedOutput, String)> {
    use std::io::{Read, Seek, SeekFrom};

    let mut sink = tempfile::tempfile().ok()?;
    let mut cmd = Command::new(program);
    cmd.args(args);
    if let Some(cwd) = cwd {
        cmd.current_dir(cwd);
    }
    if program == "git" {
        for key in GIT_REPO_ENV {
            cmd.env_remove(key);
        }
    }
    for (key, value) in extra_env {
        cmd.env(key, value);
    }
    let output = stdout_within_with_stderr(&mut cmd, budget, Stdio::from(sink.try_clone().ok()?))?;

    let len = sink.seek(SeekFrom::End(0)).ok()?;
    sink.seek(SeekFrom::Start(len.saturating_sub(STDERR_TAIL_BYTES)))
        .ok()?;
    let mut tail = Vec::new();
    sink.take(STDERR_TAIL_BYTES).read_to_end(&mut tail).ok()?;
    let line = String::from_utf8_lossy(&tail)
        .lines()
        .rev()
        .map(str::trim)
        .find(|line| !line.is_empty())
        .unwrap_or_default()
        .to_owned();
    Some((output, line))
}
