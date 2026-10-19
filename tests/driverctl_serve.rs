#![cfg(target_os = "linux")]
// ============================================================================
// `driverctl serve` end to end, through the REAL binary (Phase 27, plan 27-06;
// DCTL-01, DCTL-03, DCTL-04, D-03, D-04, D-20, D-21, D-26).
//
// Every process here is the built `gsd-meta-manager`: one `driverctl serve` per
// run, and one `driverctl <verb>` client per request. The server's stdio goes to
// FILES (never pipes nothing drains, never `Stdio::null()`, which hides why a
// child died), every spawn runs under `XDG_DATA_HOME=/proc/nonexistent-gsdmm`
// (the sandbox data dir is unwritable, T-27-11) and the clients run without
// `CLAUDE_CODE_SESSION_ID` so a `start` records what the test sent.
//
// The server is ready when it has printed `listening on`, which it does after
// the socket is bound and the launch is recorded.
// ============================================================================

use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

use gsd_meta_manager::driver::lock::{probe_at, run_lock_path, LockProbe};
use gsd_meta_manager::state_reader::driver_frontmatter::{
    parse_driver_frontmatter, read_driver_run, DerivedStatus, RunStatus,
};
use tempfile::TempDir;

/// The unwritable data dir every spawn runs under (T-27-11).
const UNWRITABLE_DATA_HOME: &str = "/proc/nonexistent-gsdmm";

/// The run id every test serves.
const RUN_ID: &str = "r27";

/// The human-owned DIRECTION body: two items, I-01 with a dispatch budget of 3.
const BODY: &str = "# Driver direction: serve test

## Direction

- control: run
- goal: Exercise the server.

## Items

### I-01 First slice

- intent: Phase 1 is planned, executed and verified.
- budget: dispatches=3

### I-02 Second slice

- intent: Phase 2 is planned, executed and verified.
";

/// A project holding one run directory, plus a scratch directory for sockets and
/// the server's captured stdio.
struct Fixture {
    project_dir: TempDir,
    scratch: TempDir,
    run_dir: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let project_dir = TempDir::new().expect("project dir");
        let scratch = TempDir::new().expect("scratch dir");
        let run_dir = project_dir
            .path()
            .join(".planning/meta-manager/drivers")
            .join(RUN_ID);
        fs::create_dir_all(&run_dir).expect("run dir");
        fs::write(run_dir.join("DIRECTION.md"), BODY).expect("DIRECTION.md");
        Fixture {
            project_dir,
            scratch,
            run_dir,
        }
    }

    fn project(&self) -> &Path {
        self.project_dir.path()
    }

    /// A socket path in the scratch dir (short: the unix limit is 107 bytes).
    fn sock(&self, name: &str) -> PathBuf {
        self.scratch.path().join(name)
    }

    fn read(&self, name: &str) -> String {
        fs::read_to_string(self.run_dir.join(name)).unwrap_or_default()
    }

    fn log(&self) -> String {
        self.read("LOG.md")
    }
}

/// A running `driverctl serve` child with its stdio captured to files.
struct Served {
    child: Child,
    stdout: PathBuf,
    stderr: PathBuf,
}

impl Served {
    fn stderr_text(&self) -> String {
        fs::read_to_string(&self.stderr).unwrap_or_default()
    }

    fn stdout_text(&self) -> String {
        fs::read_to_string(&self.stdout).unwrap_or_default()
    }

    /// Wait for the child to exit, killing it (and failing) after `limit`.
    fn wait_exit(&mut self, limit: Duration) -> std::process::ExitStatus {
        let started = Instant::now();
        loop {
            if let Some(status) = self.child.try_wait().expect("try_wait") {
                return status;
            }
            if started.elapsed() > limit {
                let _ = self.child.kill();
                let _ = self.child.wait();
                panic!(
                    "serve did not exit within {limit:?}\nstderr: {}",
                    self.stderr_text()
                );
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    fn signal(&self, signal: rustix::process::Signal) {
        let pid = rustix::process::Pid::from_child(&self.child);
        rustix::process::kill_process(pid, signal).expect("signal the server");
    }
}

impl Drop for Served {
    fn drop(&mut self) {
        // A test that failed midway must not leave a server running.
        if matches!(self.child.try_wait(), Ok(None)) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

/// The `serve` command line, not yet spawned.
fn serve_command(fx: &Fixture, sock: Option<&Path>) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_gsd-meta-manager"));
    cmd.args(["driverctl", "serve", "--run-dir"])
        .arg(&fx.run_dir)
        .arg("--project")
        .arg(fx.project());
    if let Some(sock) = sock {
        cmd.arg("--sock").arg(sock);
    }
    cmd.env("XDG_DATA_HOME", UNWRITABLE_DATA_HOME)
        .env_remove("XDG_RUNTIME_DIR")
        .env_remove("DRIVERCTL_SOCK")
        .env_remove("CLAUDE_CODE_SESSION_ID");
    cmd
}

/// Spawn `cmd` with stdio to fresh files named `tag` in the scratch dir.
fn spawn_captured(fx: &Fixture, mut cmd: Command, tag: &str) -> Served {
    let stdout = fx.scratch.path().join(format!("{tag}.out"));
    let stderr = fx.scratch.path().join(format!("{tag}.err"));
    cmd.stdin(Stdio::null())
        .stdout(fs::File::create(&stdout).expect("stdout file"))
        .stderr(fs::File::create(&stderr).expect("stderr file"));
    let child = cmd.spawn().expect("the binary is built and spawnable");
    Served {
        child,
        stdout,
        stderr,
    }
}

/// Wait until the server has printed `listening on`, or fail with its stderr.
fn wait_listening(served: &mut Served) {
    let started = Instant::now();
    loop {
        if served.stderr_text().contains("listening on") {
            return;
        }
        if let Some(status) = served.child.try_wait().expect("try_wait") {
            panic!(
                "serve exited early ({status})\nstderr: {}",
                served.stderr_text()
            );
        }
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "serve never reported listening\nstderr: {}",
            served.stderr_text()
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

/// `serve` with an explicit socket, ready to answer.
fn spawn_serve(fx: &Fixture, sock: &Path, tag: &str) -> Served {
    let mut served = spawn_captured(fx, serve_command(fx, Some(sock)), tag);
    wait_listening(&mut served);
    assert!(sock.exists(), "the socket exists once serve is listening");
    served
}

/// `gsd-meta-manager driverctl <args>` against `sock`, with `stdin` if given.
fn client(sock: &Path, args: &[&str], stdin: Option<&str>) -> Output {
    use std::io::Write;

    let mut cmd = Command::new(env!("CARGO_BIN_EXE_gsd-meta-manager"));
    cmd.arg("driverctl")
        .args(args)
        .env("DRIVERCTL_SOCK", sock)
        .env("XDG_DATA_HOME", UNWRITABLE_DATA_HOME)
        .env_remove("CLAUDE_CODE_SESSION_ID")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    let mut child = cmd.spawn().expect("spawn the client");
    {
        let mut pipe = child.stdin.take().expect("client stdin");
        if let Some(text) = stdin {
            pipe.write_all(text.as_bytes()).expect("client stdin write");
        }
    }
    child.wait_with_output().expect("client output")
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

// ---------------------------------------------------------------- tracer

#[test]
fn serve_log_show_and_reader_end_to_end() {
    let fx = Fixture::new();
    let sock = fx.sock("serve.sock");
    let mut served = spawn_serve(&fx, &sock, "tracer");

    let start = client(
        &sock,
        &["log", "start", "--mode", "headless", "--loop-mode", "loop"],
        None,
    );
    assert_eq!(
        start.status.code(),
        Some(0),
        "log start: {}",
        stderr(&start)
    );

    let show = client(&sock, &["show"], None);
    assert_eq!(show.status.code(), Some(0), "show: {}", stderr(&show));
    let fm = parse_driver_frontmatter(&stdout(&show)).expect("show prints a parsable frontmatter");
    assert_eq!(fm.status, RunStatus::Running);
    assert_eq!(fm.launches, 1);

    // The TUI reader sees the live server: the run lock is held.
    let view = read_driver_run(&fx.run_dir).expect("reader parses the run");
    assert_eq!(view.derived, DerivedStatus::AsWritten(RunStatus::Running));

    served.signal(rustix::process::Signal::TERM);
    let status = served.wait_exit(Duration::from_secs(10));
    assert_eq!(
        status.code(),
        Some(0),
        "a signalled serve exits 0\nstderr: {}",
        served.stderr_text()
    );

    let view = read_driver_run(&fx.run_dir).expect("reader parses the stopped run");
    assert_eq!(view.derived, DerivedStatus::AsWritten(RunStatus::Stopped));
    assert_eq!(
        view.frontmatter.status_reason.as_deref(),
        Some("serve:signal")
    );
    assert_eq!(
        probe_at(&run_lock_path(&fx.run_dir)),
        LockProbe::Free,
        "the lock is released on exit"
    );
    assert!(!sock.exists(), "the socket is unlinked on exit");

    let log = fx.log();
    assert!(log.contains("RUN | launch"), "LOG.md: {log}");
    assert!(log.contains("RUN | start"), "LOG.md: {log}");
    assert!(
        log.contains("RUN | launch-end | status=stopped | reason=serve:signal"),
        "LOG.md: {log}"
    );
    assert_eq!(
        served.stdout_text(),
        "",
        "serve writes nothing to stdout; diagnostics go to stderr"
    );
}
