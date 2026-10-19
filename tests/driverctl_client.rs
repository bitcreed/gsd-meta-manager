#![cfg(unix)]
// ============================================================================
// The sandbox-side `driverctl` client, through the REAL binary (Phase 27, plan
// 27-04; DCTL-01 client half, D-01, D-02, D-04, D-25).
//
// Every test spawns `gsd-meta-manager driverctl ...` as a child process against a
// test-local recording server, and every spawn carries
// `XDG_DATA_HOME=/proc/nonexistent-gsdmm`. That variable is the regression guard
// for T-27-11: the data dir is unwritable, so a client that initialised tracing
// first would panic (rc 101) and, as a PreToolUse hook, fail OPEN. The binary is
// the unit under test on purpose; an in-process call to the client would not
// exercise `main`'s dispatch order, which is the property that matters.
//
// The recording server answers each connection with the next scripted reply line
// (the last one repeats) and records the request line it received, so a test can
// compare the exact wire request the spike client would have sent.
// ============================================================================

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use serde_json::{json, Value};
use tempfile::TempDir;

/// The unwritable data dir every spawn runs under (D-25, T-27-11).
const UNWRITABLE_DATA_HOME: &str = "/proc/nonexistent-gsdmm";

/// A unix-socket server that records requests and plays scripted replies.
struct Recorder {
    _dir: TempDir,
    sock: PathBuf,
    requests: Arc<Mutex<Vec<String>>>,
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl Recorder {
    /// Serve `replies` in order (the last repeats). Each is one JSON line.
    fn start(replies: &[&str]) -> Recorder {
        let dir = TempDir::new().expect("temp dir for the socket");
        let sock = dir.path().join("driverctl.sock");
        let listener = UnixListener::bind(&sock).expect("bind the recording socket");
        listener.set_nonblocking(true).expect("non-blocking accept");
        let requests = Arc::new(Mutex::new(Vec::<String>::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let replies: Vec<String> = replies.iter().map(|r| r.to_string()).collect();

        let handle = {
            let requests = Arc::clone(&requests);
            let stop = Arc::clone(&stop);
            std::thread::spawn(move || {
                let mut served = 0usize;
                while !stop.load(Ordering::SeqCst) {
                    match listener.accept() {
                        Ok((stream, _)) => {
                            stream.set_nonblocking(false).expect("blocking connection");
                            stream
                                .set_read_timeout(Some(Duration::from_secs(5)))
                                .expect("read timeout");
                            let mut reader = BufReader::new(stream);
                            let mut line = String::new();
                            if reader.read_line(&mut line).is_ok() && !line.is_empty() {
                                requests.lock().expect("requests").push(line);
                                let reply = replies
                                    .get(served)
                                    .or_else(|| replies.last())
                                    .cloned()
                                    .unwrap_or_else(|| r#"{"ok": true}"#.to_string());
                                served += 1;
                                let _ = reader.get_mut().write_all(format!("{reply}\n").as_bytes());
                            }
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            std::thread::sleep(Duration::from_millis(5));
                        }
                        Err(_) => break,
                    }
                }
            })
        };
        Recorder {
            _dir: dir,
            sock,
            requests,
            stop,
            handle: Some(handle),
        }
    }

    /// Every request line received so far, parsed.
    fn requests(&self) -> Vec<Value> {
        self.requests
            .lock()
            .expect("requests")
            .iter()
            .map(|line| serde_json::from_str(line).expect("the client sent one JSON object"))
            .collect()
    }

    /// The raw request lines, for newline-framing assertions.
    fn raw_requests(&self) -> Vec<String> {
        self.requests.lock().expect("requests").clone()
    }
}

impl Drop for Recorder {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

/// `gsd-meta-manager driverctl ...` with the hostile environment of the sandbox.
fn driverctl(sock: Option<&Path>) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_gsd-meta-manager"));
    cmd.env("XDG_DATA_HOME", UNWRITABLE_DATA_HOME)
        // The executor's own Claude session sets this; a test that reads it would
        // assert against the wrong session.
        .env_remove("CLAUDE_CODE_SESSION_ID")
        .env_remove("DRIVERCTL_SOCK");
    if let Some(sock) = sock {
        cmd.env("DRIVERCTL_SOCK", sock);
    }
    cmd.arg("driverctl");
    cmd
}

/// Run `cmd` to completion with `stdin` (empty when `None`) and capture it.
fn run(cmd: &mut Command, stdin: Option<&str>) -> Output {
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the binary is built and spawnable");
    {
        let mut pipe = child.stdin.take().expect("piped stdin");
        if let Some(text) = stdin {
            pipe.write_all(text.as_bytes()).expect("write stdin");
        }
    }
    child.wait_with_output().expect("wait for the client")
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// A socket path that nothing listens on.
fn dead_socket() -> (TempDir, PathBuf) {
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join("nobody-home.sock");
    (dir, path)
}

#[test]
fn log_start_sends_the_spike_wire_request_and_prints_ok() {
    let server = Recorder::start(&[r#"{"ok": true}"#]);
    let out = run(
        driverctl(Some(&server.sock)).args([
            "log",
            "start",
            "--mode",
            "headless",
            "--loop-mode",
            "loop",
        ]),
        None,
    );

    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    assert_eq!(stdout(&out), "ok\n");
    assert!(
        !stderr(&out).contains("panicked"),
        "the client must not panic under an unwritable data dir: {}",
        stderr(&out)
    );

    let requests = server.requests();
    assert_eq!(requests.len(), 1, "exactly one request per connection");
    assert_eq!(
        requests[0],
        json!({"op": "log", "event": "start",
               "args": {"mode": "headless", "loop_mode": "loop", "session": null}})
    );
    let raw = server.raw_requests();
    assert!(raw[0].ends_with('\n'), "the request is framed by a newline");
    assert_eq!(
        raw[0].matches('\n').count(),
        1,
        "one line, no embedded newline"
    );
}

#[test]
fn rejected_reply_exits_2_with_the_spike_message() {
    let server = Recorder::start(&[r#"{"ok": false, "error": "bad rule: nope"}"#]);
    let out = run(
        driverctl(Some(&server.sock))
            .args(["log", "status", "--item", "I-01", "--state", "parked"]),
        None,
    );

    assert_eq!(out.status.code(), Some(2));
    assert_eq!(stdout(&out), "", "a rejection prints nothing on stdout");
    assert_eq!(stderr(&out), "driverctl: REJECTED: bad rule: nope\n");
}

#[test]
fn transport_error_exits_1_not_101() {
    let (_dir, dead) = dead_socket();
    let out = run(
        driverctl(Some(&dead)).args(["log", "end", "--reason", "stop"]),
        None,
    );

    assert_eq!(out.status.code(), Some(1), "stderr: {}", stderr(&out));
    assert!(stderr(&out).starts_with("driverctl: "), "{}", stderr(&out));
    assert!(!stderr(&out).contains("panicked"), "{}", stderr(&out));
}

#[test]
fn unset_socket_env_exits_1() {
    let out = run(
        driverctl(None).args(["log", "end", "--reason", "stop"]),
        None,
    );

    assert_eq!(out.status.code(), Some(1));
    assert!(
        stderr(&out).contains("DRIVERCTL_SOCK is not set"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn usage_errors_exit_2() {
    let server = Recorder::start(&[r#"{"ok": true}"#]);
    let out = run(
        driverctl(Some(&server.sock)).args(["log", "start", "--bogus", "x"]),
        None,
    );

    assert_eq!(
        out.status.code(),
        Some(2),
        "argparse parity for usage errors"
    );
    assert!(server.requests().is_empty(), "a usage error sends nothing");
}

#[test]
fn report_body_comes_from_stdin() {
    let server = Recorder::start(&[r#"{"ok": true}"#]);
    let body = "STATUS: done\nITEM: I-01\n";
    let out = run(
        driverctl(Some(&server.sock)).args(["log", "report", "--item", "I-01", "--status", "done"]),
        Some(body),
    );

    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    assert_eq!(stdout(&out), "ok\n");
    assert_eq!(
        server.requests()[0],
        json!({"op": "log", "event": "report",
               "args": {"item": "I-01", "status": "done", "body": body}})
    );
}

#[test]
fn start_session_defaults_from_the_claude_env() {
    let server = Recorder::start(&[r#"{"ok": true}"#]);
    let out = run(
        driverctl(Some(&server.sock))
            .env("CLAUDE_CODE_SESSION_ID", "sess-from-env")
            .args(["log", "start", "--mode", "headless"]),
        None,
    );
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    assert_eq!(
        server.requests()[0]["args"]["session"],
        json!("sess-from-env")
    );

    // An explicit flag wins over the environment.
    let out = run(
        driverctl(Some(&server.sock))
            .env("CLAUDE_CODE_SESSION_ID", "sess-from-env")
            .args(["log", "start", "--session", "sess-flag"]),
        None,
    );
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    assert_eq!(server.requests()[1]["args"]["session"], json!("sess-flag"));

    // The default applies to `start` only.
    let out = run(
        driverctl(Some(&server.sock))
            .env("CLAUDE_CODE_SESSION_ID", "sess-from-env")
            .args(["log", "end", "--reason", "stop"]),
        None,
    );
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    assert!(
        server.requests()[2]["args"].get("session").is_none(),
        "only `start` defaults the session"
    );
}
