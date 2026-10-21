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
        Self::with_run_id(RUN_ID)
    }

    /// A fixture whose run directory is named `run_id`.
    fn with_run_id(run_id: &str) -> Self {
        let project_dir = TempDir::new().expect("project dir");
        let scratch = TempDir::new().expect("scratch dir");
        let run_dir = project_dir
            .path()
            .join(".planning/meta-manager/drivers")
            .join(run_id);
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

// ---------------------------------------------------------------- helpers (task 2)

use std::os::unix::fs::{MetadataExt, PermissionsExt};
use std::os::unix::net::UnixStream;

/// Bytes of every file a rejected request must leave alone.
fn run_files(fx: &Fixture) -> [Vec<u8>; 3] {
    ["LOG.md", "state.json", "DIRECTION.md"]
        .map(|name| fs::read(fx.run_dir.join(name)).unwrap_or_default())
}

/// A PreToolUse `Agent` payload with no `agent_id`: the dispatch gate's input.
const GATE_PAYLOAD: &str = r#"{"hook_event_name":"PreToolUse","tool_name":"Agent","tool_input":{"description":"x","prompt":"y"},"session_id":"s-1"}"#;

/// `serve --run-dir <run_dir> --project <project> --sock <sock>` for a run dir
/// that is not the fixture's own.
fn serve_command_for(fx: &Fixture, run_dir: &Path, sock: &Path) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_gsd-meta-manager"));
    cmd.args(["driverctl", "serve", "--run-dir"])
        .arg(run_dir)
        .arg("--project")
        .arg(fx.project())
        .arg("--sock")
        .arg(sock)
        .env("XDG_DATA_HOME", UNWRITABLE_DATA_HOME)
        .env_remove("XDG_RUNTIME_DIR");
    cmd
}

// ---------------------------------------------------------------- D-21 test 6

#[test]
fn second_serve_is_refused_and_does_not_steal_the_socket() {
    let fx = Fixture::new();
    let sock_a = fx.sock("a.sock");
    let served_a = spawn_serve(&fx, &sock_a, "a");
    let inode_before = fs::metadata(&sock_a).expect("A's socket").ino();

    // B on a different socket path: refused quickly, creates nothing.
    let sock_b = fx.sock("b.sock");
    let started = Instant::now();
    let mut b = spawn_captured(&fx, serve_command(&fx, Some(&sock_b)), "b");
    let status = b.wait_exit(Duration::from_secs(10));
    assert_eq!(
        status.code(),
        Some(3),
        "a second serve exits 3\nstderr: {}",
        b.stderr_text()
    );
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "the refusal is quick, not a hang"
    );
    assert!(!sock_b.exists(), "the refused serve created nothing");
    assert!(
        b.stderr_text().contains("already served"),
        "stderr names the refusal: {}",
        b.stderr_text()
    );

    // B on A's socket path: refused, and A's socket is the same inode.
    let mut b2 = spawn_captured(&fx, serve_command(&fx, Some(&sock_a)), "b2");
    let status = b2.wait_exit(Duration::from_secs(10));
    assert_eq!(status.code(), Some(3), "stderr: {}", b2.stderr_text());
    assert_eq!(
        fs::metadata(&sock_a).expect("A's socket survives").ino(),
        inode_before,
        "the refused serve neither unlinked nor rebound A's socket"
    );
    let show = client(&sock_a, &["show"], None);
    assert_eq!(
        show.status.code(),
        Some(0),
        "A still answers: {}",
        stderr(&show)
    );
    drop(served_a);
}

// ---------------------------------------------------------------- D-20 test 7

#[test]
fn sigkill_reads_crashed_and_restart_logs_crash_detected() {
    let fx = Fixture::new();
    let sock = fx.sock("k.sock");
    let mut a = spawn_serve(&fx, &sock, "a");
    let start = client(&sock, &["log", "start", "--mode", "headless"], None);
    assert_eq!(start.status.code(), Some(0), "start: {}", stderr(&start));

    a.signal(rustix::process::Signal::KILL);
    let status = a.wait_exit(Duration::from_secs(10));
    assert_eq!(status.code(), None, "SIGKILL leaves no exit code");

    // The kernel released the lock; nobody wrote a terminal status.
    let view = read_driver_run(&fx.run_dir).expect("reader parses the killed run");
    assert_eq!(
        view.derived,
        DerivedStatus::Crashed {
            written: RunStatus::Running
        }
    );
    assert!(sock.exists(), "a SIGKILLed serve leaves its socket behind");

    // A restart takes the stale socket (it holds the lock) and logs the crash.
    let b = spawn_serve(&fx, &sock, "b");
    let log = fx.log();
    assert!(
        log.contains(
            "RUN | crash-detected | previous_launch=1 | evidence=status=running and no RUN end; lock was free"
        ),
        "LOG.md: {log}"
    );
    let view = read_driver_run(&fx.run_dir).expect("reader parses the restarted run");
    assert_eq!(view.derived, DerivedStatus::AsWritten(RunStatus::Running));
    assert_eq!(view.frontmatter.launches, 2);
    drop(b);
}

// ---------------------------------------------------------------- D-04

#[test]
fn malformed_event_through_the_binary_exits_2_and_writes_nothing() {
    let fx = Fixture::new();
    let sock = fx.sock("m.sock");
    let _served = spawn_serve(&fx, &sock, "m");
    let start = client(&sock, &["log", "start", "--mode", "headless"], None);
    assert_eq!(start.status.code(), Some(0), "start: {}", stderr(&start));

    let before = run_files(&fx);
    let bad = client(
        &sock,
        &["log", "status", "--item", "I-01", "--state", "bogus"],
        None,
    );
    assert_eq!(bad.status.code(), Some(2), "stderr: {}", stderr(&bad));
    assert!(
        stderr(&bad).contains("driverctl: REJECTED:"),
        "stderr: {}",
        stderr(&bad)
    );
    assert_eq!(
        run_files(&fx),
        before,
        "LOG.md, state.json and DIRECTION.md are byte-identical after a rejection"
    );
}

// ---------------------------------------------------------------- D-26

#[test]
fn gap_scan_tick_observes_verification_writes() {
    let fx = Fixture::new();
    let sock = fx.sock("g.sock");
    let _served = spawn_serve(&fx, &sock, "g");

    let phase_dir = fx.project().join(".planning/phases/01-x");
    fs::create_dir_all(&phase_dir).expect("phase dir");
    fs::write(
        phase_dir.join("01-VERIFICATION.md"),
        "---\nstatus: gaps_found\n---\nbody\n",
    )
    .expect("verification");

    let started = Instant::now();
    loop {
        if fx.log().contains("RUN | gap-observed | phase=1 | count=1") {
            break;
        }
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "no gap-observed line within 5 s\nLOG.md: {}",
            fx.log()
        );
        std::thread::sleep(Duration::from_millis(100));
    }
    let show = client(&sock, &["show"], None);
    let fm = parse_driver_frontmatter(&stdout(&show)).expect("show parses");
    assert_eq!(fm.caps.gap_rounds.used, 1, "show reports gap_rounds used 1");
}

// ---------------------------------------------------------------- D-21 paths

/// sha256(canonical run dir)[:8] as hex, computed independently of the server.
fn expected_hash8(run_dir: &Path) -> String {
    use sha2::{Digest, Sha256};
    use std::os::unix::ffi::OsStrExt;

    let canonical = fs::canonicalize(run_dir).expect("canonical run dir");
    Sha256::digest(canonical.as_os_str().as_bytes())
        .iter()
        .take(4)
        .map(|b| format!("{b:02x}"))
        .collect()
}

/// The fixed-length socket stem, computed independently of the server:
/// sha256(run id, 0x00, canonical run dir)[:8] as 16 lowercase hex characters.
fn expected_fixed_stem(run_id: &str, run_dir: &Path) -> String {
    use sha2::{Digest, Sha256};
    use std::os::unix::ffi::OsStrExt;

    let canonical = fs::canonicalize(run_dir).expect("canonical run dir");
    let mut hasher = Sha256::new();
    hasher.update(run_id.as_bytes());
    hasher.update([0u8]);
    hasher.update(canonical.as_os_str().as_bytes());
    hasher
        .finalize()
        .iter()
        .take(8)
        .map(|b| format!("{b:02x}"))
        .collect()
}

#[test]
fn default_socket_path_gives_a_long_run_id_the_fixed_length_name() {
    // An 80-character run id makes the D-21 name far longer than the unix
    // socket limit; the default path falls back to a fixed-length name in the
    // same private directory.
    let run_id = format!("20261021-{}", "a".repeat(71));
    assert_eq!(run_id.len(), 80);
    let fx = Fixture::with_run_id(&run_id);
    let runtime = TempDir::new().expect("runtime dir");
    let mut cmd = serve_command(&fx, None);
    cmd.env("XDG_RUNTIME_DIR", runtime.path());
    let mut served = spawn_captured(&fx, cmd, "long");
    wait_listening(&mut served);

    let dir = runtime.path().join("gsdmm-driverctl");
    let expected = dir.join(format!(
        "{}.sock",
        expected_fixed_stem(&run_id, &fx.run_dir)
    ));
    assert!(
        expected.exists(),
        "fixed-length socket at {}",
        expected.display()
    );
    let sock_mode = fs::metadata(&expected).expect("socket metadata").mode();
    assert_eq!(sock_mode & 0o777, 0o600, "the socket is 0600");
    let d21 = dir.join(format!("{run_id}-{}.sock", expected_hash8(&fx.run_dir)));
    assert!(!d21.exists(), "no D-21-shaped socket at {}", d21.display());

    let show = client(&expected, &["show"], None);
    assert_eq!(show.status.code(), Some(0), "show: {}", stderr(&show));
    drop(served);
}

#[test]
fn default_socket_path_requires_xdg_runtime_dir() {
    // No --sock and no XDG_RUNTIME_DIR: refused, naming the variable, before
    // anything is written.
    let fx = Fixture::new();
    // TMPDIR is set to a real absolute directory and still ignored on Linux
    // (D-21: no shared-dir fallback; only macOS consults it, D-31).
    let tmpdir = TempDir::new().expect("tmpdir");
    let mut none_cmd = serve_command(&fx, None);
    none_cmd.env("TMPDIR", tmpdir.path());
    let mut none = spawn_captured(&fx, none_cmd, "none");
    let status = none.wait_exit(Duration::from_secs(10));
    assert_eq!(status.code(), Some(1), "stderr: {}", none.stderr_text());
    assert!(
        none.stderr_text().contains("XDG_RUNTIME_DIR"),
        "stderr names the variable: {}",
        none.stderr_text()
    );
    assert!(
        !fx.run_dir.join("LOG.md").exists() && !fx.run_dir.join("state.json").exists(),
        "a refused serve registers no launch"
    );

    // XDG_RUNTIME_DIR set: the socket is <dir>/gsdmm-driverctl/<id>-<hash8>.sock.
    let runtime = TempDir::new().expect("runtime dir");
    let mut cmd = serve_command(&fx, None);
    cmd.env("XDG_RUNTIME_DIR", runtime.path());
    let mut served = spawn_captured(&fx, cmd, "xdg");
    wait_listening(&mut served);
    let expected = runtime
        .path()
        .join("gsdmm-driverctl")
        .join(format!("{RUN_ID}-{}.sock", expected_hash8(&fx.run_dir)));
    assert!(
        expected.exists(),
        "default socket at {}",
        expected.display()
    );
    let dir_mode = fs::metadata(expected.parent().expect("socket dir"))
        .expect("socket dir metadata")
        .mode();
    assert_eq!(dir_mode & 0o777, 0o700, "the socket dir is 0700");
    let sock_mode = fs::metadata(&expected).expect("socket metadata").mode();
    assert_eq!(sock_mode & 0o777, 0o600, "the socket is 0600");
    let show = client(&expected, &["show"], None);
    assert_eq!(show.status.code(), Some(0), "show: {}", stderr(&show));
    drop(served);

    // A pre-existing group-writable socket dir is refused (T-27-22).
    let hostile = TempDir::new().expect("hostile runtime dir");
    let dir = hostile.path().join("gsdmm-driverctl");
    fs::create_dir(&dir).expect("pre-created dir");
    fs::set_permissions(&dir, fs::Permissions::from_mode(0o770)).expect("chmod");
    let fx2 = Fixture::new();
    let mut cmd = serve_command(&fx2, None);
    cmd.env("XDG_RUNTIME_DIR", hostile.path());
    let mut refused = spawn_captured(&fx2, cmd, "hostile");
    let status = refused.wait_exit(Duration::from_secs(10));
    assert_eq!(status.code(), Some(1), "stderr: {}", refused.stderr_text());
    assert!(
        refused.stderr_text().contains("group or others"),
        "stderr: {}",
        refused.stderr_text()
    );
    assert!(
        !fx2.run_dir.join("state.json").exists(),
        "a refused serve registers no launch"
    );
}

// ---------------------------------------------------------------- Pitfall 8

#[test]
fn run_dir_outside_drivers_is_refused() {
    let fx = Fixture::new();

    // A directory that is not under .planning/meta-manager/drivers/.
    let outside = fx.project().join("elsewhere").join(RUN_ID);
    fs::create_dir_all(&outside).expect("outside dir");
    fs::write(outside.join("DIRECTION.md"), BODY).expect("DIRECTION.md");
    let cmd = serve_command_for(&fx, &outside, &fx.sock("o.sock"));
    let mut refused = spawn_captured(&fx, cmd, "outside");
    let status = refused.wait_exit(Duration::from_secs(10));
    assert_eq!(status.code(), Some(1), "stderr: {}", refused.stderr_text());
    assert!(
        refused.stderr_text().contains("not directly under"),
        "stderr: {}",
        refused.stderr_text()
    );
    assert!(
        !outside.join("launcher.lock").exists(),
        "the lock is never touched for a refused run dir"
    );

    // A symlink under drivers/ that points outside is refused as well.
    let link = fx
        .project()
        .join(".planning/meta-manager/drivers")
        .join("linked");
    std::os::unix::fs::symlink(&outside, &link).expect("symlink");
    let cmd = serve_command_for(&fx, &link, &fx.sock("l.sock"));
    let mut refused = spawn_captured(&fx, cmd, "linked");
    let status = refused.wait_exit(Duration::from_secs(10));
    assert_eq!(status.code(), Some(1), "stderr: {}", refused.stderr_text());
    assert!(!outside.join("launcher.lock").exists());
}

// ---------------------------------------------------------------- stale sockets

#[test]
fn stale_socket_is_replaced_only_by_the_lock_holder() {
    // A stale socket file is replaced by the lock holder.
    let fx = Fixture::new();
    let sock = fx.sock("s.sock");
    drop(std::os::unix::net::UnixListener::bind(&sock).expect("make a stale socket"));
    assert!(sock.exists());
    let served = spawn_serve(&fx, &sock, "stale");
    let show = client(&sock, &["show"], None);
    assert_eq!(show.status.code(), Some(0), "show: {}", stderr(&show));
    drop(served);

    // A regular file at --sock is refused and left intact.
    let fx2 = Fixture::new();
    let file = fx2.sock("plain");
    fs::write(&file, "not a socket").expect("regular file");
    let mut refused = spawn_captured(&fx2, serve_command(&fx2, Some(&file)), "plain");
    let status = refused.wait_exit(Duration::from_secs(10));
    assert_eq!(status.code(), Some(1), "stderr: {}", refused.stderr_text());
    assert_eq!(
        fs::read_to_string(&file).expect("file survives"),
        "not a socket"
    );
    assert!(
        !fx2.run_dir.join("state.json").exists(),
        "a refused serve registers no launch"
    );
}

#[test]
fn wr05_another_runs_live_socket_is_not_stolen_through_a_shared_sock() {
    // Two DIFFERENT runs (two projects, two run locks) pointed at one --sock. The
    // run lock cannot protect the path; the live-socket probe must.
    use std::os::unix::fs::MetadataExt;
    let fx_a = Fixture::new();
    let sock = fx_a.sock("shared.sock");
    let served_a = spawn_serve(&fx_a, &sock, "a");
    let inode_before = fs::metadata(&sock).expect("A's socket").ino();

    let fx_b = Fixture::new();
    let mut b = spawn_captured(&fx_b, serve_command(&fx_b, Some(&sock)), "b");
    let status = b.wait_exit(Duration::from_secs(15));
    assert_eq!(status.code(), Some(1), "stderr: {}", b.stderr_text());
    assert!(
        b.stderr_text().contains("live socket"),
        "stderr names the refusal: {}",
        b.stderr_text()
    );
    assert_eq!(
        fs::metadata(&sock).expect("A's socket survives").ino(),
        inode_before,
        "B neither unlinked nor rebound A's socket"
    );
    let show = client(&sock, &["show"], None);
    assert_eq!(
        show.status.code(),
        Some(0),
        "A still answers: {}",
        stderr(&show)
    );
    drop(served_a);
}

// ---------------------------------------------------------------- concurrency

#[test]
fn concurrent_clients_are_all_answered() {
    let fx = Fixture::new();
    let sock = fx.sock("c.sock");
    let _served = spawn_serve(&fx, &sock, "c");

    let handles: Vec<_> = (0..16)
        .map(|_| {
            let sock = sock.clone();
            std::thread::spawn(move || client(&sock, &["log", "reconcile", "--found", "1"], None))
        })
        .collect();
    for handle in handles {
        let out = handle.join().expect("client thread");
        assert_eq!(out.status.code(), Some(0), "client: {}", stderr(&out));
    }

    let log = fx.log();
    let whole: Vec<&str> = log
        .lines()
        .filter(|l| l.contains(" | RUN | reconcile | "))
        .collect();
    assert_eq!(whole.len(), 16, "16 whole reconcile lines in: {log}");
    assert!(
        whole
            .iter()
            .all(|l| l.ends_with("RUN | reconcile | found=1")),
        "every reconcile line is whole: {whole:?}"
    );
}

#[test]
fn stalled_connection_does_not_block_the_gate() {
    let fx = Fixture::new();
    let sock = fx.sock("t.sock");
    let _served = spawn_serve(&fx, &sock, "t");

    // One raw connection, sending nothing, held open.
    let _stalled = UnixStream::connect(&sock).expect("connect the stalled client");
    std::thread::sleep(Duration::from_millis(200));

    let out = client(&sock, &["hook"], Some(GATE_PAYLOAD));
    assert_eq!(out.status.code(), Some(0), "hook: {}", stderr(&out));
    // WR-09: no wall-clock ceiling. The property is that the gate was answered
    // WHILE the stalled connection still occupied its handler slot, i.e. before the
    // server's read deadline closed it; a still-open peer shows as WouldBlock.
    _stalled.set_nonblocking(true).expect("non-blocking probe");
    let mut probe = [0u8; 1];
    match std::io::Read::read(&mut &_stalled, &mut probe) {
        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {}
        other => panic!("the stalled connection was already closed ({other:?}): the gate did not answer behind it"),
    }
    assert!(
        !stdout(&out).contains("deny"),
        "the gate allows the dispatch: {}",
        stdout(&out)
    );
}

#[test]
fn a_flood_of_idle_connections_is_answered_busy_and_the_gate_denies() {
    let fx = Fixture::new();
    let sock = fx.sock("f.sock");
    let _served = spawn_serve(&fx, &sock, "f");

    let idle: Vec<UnixStream> = (0..32)
        .map(|_| UnixStream::connect(&sock).expect("idle connection"))
        .collect();
    std::thread::sleep(Duration::from_millis(500));

    let out = client(&sock, &["hook"], Some(GATE_PAYLOAD));
    assert_eq!(out.status.code(), Some(0), "hook: {}", stderr(&out));
    assert!(
        stdout(&out).contains("deny"),
        "past 32 in flight the gate client denies: {}",
        stdout(&out)
    );
    assert!(
        stdout(&out).contains("busy"),
        "the deny reason carries the server's answer: {}",
        stdout(&out)
    );
    drop(idle);
}
