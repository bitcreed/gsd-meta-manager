#![cfg(target_os = "linux")]
// ============================================================================
// The in-sandbox bridge, end to end in CI (Phase 28.1, plan 05).
//
// Every test runs the REAL `gsd-meta-manager bridge` subcommand as a child
// process, against a real in-process `EgressServer` on a real unix socket and a
// plain-TCP stub standing in for `api.anthropic.com` (`Upstream::Plain`). There
// is no bwrap and no network: the bridge is what is under test, not the sandbox.
//
// Every wait is bounded and every bridge is killed (with its whole process
// group, so a `sleep 30` child does not outlive the test) when its guard drops,
// so a regression fails a test instead of hanging the suite.
// ============================================================================

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::os::unix::net::UnixListener;
use std::os::unix::process::CommandExt;
use std::path::{Path, PathBuf};
use std::process::{Child, Command, Stdio};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use gsd_meta_manager::launcher::egress::upstream::Upstream;
use gsd_meta_manager::launcher::egress::{EgressHandle, EgressServer, EgressSpec, TokenProvider};
use tempfile::TempDir;

const REAL_TOKEN: &str = "BRIDGE-REAL-TOKEN-SENTINEL";
const DUMMY_TOKEN: &str = "sk-ant-oat01-DUMMY-placeholder-not-a-real-token";

/// The bound on every wait in this file.
const BOUND: Duration = Duration::from_secs(5);

// ----------------------------------------------------------------- the stub

/// A stand-in for `api.anthropic.com`: records each request head, answers `{}`.
struct Stub {
    addr: SocketAddr,
    heads: Arc<Mutex<Vec<String>>>,
    stop: Arc<AtomicBool>,
    connections: Arc<AtomicUsize>,
    thread: Option<JoinHandle<()>>,
}

impl Stub {
    fn start() -> Stub {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind the stub");
        listener.set_nonblocking(true).expect("nonblocking");
        let addr = listener.local_addr().expect("addr");
        let heads = Arc::new(Mutex::new(Vec::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let connections = Arc::new(AtomicUsize::new(0));
        let (heads_t, stop_t, connections_t) = (
            Arc::clone(&heads),
            Arc::clone(&stop),
            Arc::clone(&connections),
        );
        let thread = std::thread::spawn(move || loop {
            match listener.accept() {
                Ok((stream, _)) => {
                    connections_t.fetch_add(1, Ordering::SeqCst);
                    let heads = Arc::clone(&heads_t);
                    std::thread::spawn(move || stub_connection(stream, &heads));
                }
                Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                    if stop_t.load(Ordering::SeqCst) {
                        break;
                    }
                    std::thread::sleep(Duration::from_millis(2));
                }
                Err(_) => break,
            }
        });
        Stub {
            addr,
            heads,
            stop,
            connections,
            thread: Some(thread),
        }
    }

    /// The request heads the stub received, in order.
    fn heads(&self) -> Vec<String> {
        self.heads.lock().expect("lock").clone()
    }

    fn connections(&self) -> usize {
        self.connections.load(Ordering::SeqCst)
    }
}

impl Drop for Stub {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn stub_connection(mut stream: TcpStream, heads: &Mutex<Vec<String>>) {
    let _ = stream.set_nonblocking(false);
    let _ = stream.set_read_timeout(Some(BOUND));
    let mut buf = Vec::new();
    let mut chunk = [0u8; 4096];
    let end = loop {
        if let Some(i) = buf.windows(4).position(|w| w == b"\r\n\r\n") {
            break i + 4;
        }
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => return,
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
        }
    };
    let head = String::from_utf8_lossy(&buf[..end]).into_owned();
    let length = head
        .split("\r\n")
        .filter_map(|l| l.split_once(':'))
        .find(|(n, _)| n.trim().eq_ignore_ascii_case("content-length"))
        .and_then(|(_, v)| v.trim().parse::<usize>().ok())
        .unwrap_or(0);
    while buf.len() < end + length {
        match stream.read(&mut chunk) {
            Ok(0) | Err(_) => return,
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
        }
    }
    heads.lock().expect("lock").push(head);
    let _ =
        stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}");
}

// --------------------------------------------------------- the egress server

/// A running egress server on a short socket path in a scratch directory.
struct Egress {
    /// Keeps the scratch directory (and so the socket path) alive.
    _dir: TempDir,
    sock: PathBuf,
    handle: Option<EgressHandle>,
}

/// A fresh short scratch directory (a unix socket path is limited to ~108 bytes).
fn scratch() -> TempDir {
    tempfile::Builder::new()
        .prefix("b")
        .tempdir_in("/tmp")
        .expect("a short temp dir")
}

fn start_egress_at(sock: &Path, stub: &Stub, log: PathBuf) -> EgressHandle {
    let listener = UnixListener::bind(sock).expect("bind the egress socket");
    EgressServer::start(
        listener,
        EgressSpec {
            allow_hosts: Vec::new(),
            token: TokenProvider::Static(REAL_TOKEN.to_owned()),
            upstream: Upstream::Plain(stub.addr),
            log_path: log,
        },
    )
    .expect("start the egress server")
}

impl Egress {
    fn start(stub: &Stub) -> Egress {
        let dir = scratch();
        let sock = dir.path().join("egress.sock");
        let handle = start_egress_at(&sock, stub, dir.path().join("egress.log"));
        Egress {
            _dir: dir,
            sock,
            handle: Some(handle),
        }
    }
}

impl Drop for Egress {
    fn drop(&mut self) {
        if let Some(handle) = self.handle.take() {
            let _ = handle.stop();
        }
    }
}

// ------------------------------------------------------------- the bridge

/// A free loopback port: bind `:0`, read it, drop the listener.
fn free_port() -> u16 {
    TcpListener::bind("127.0.0.1:0")
        .expect("bind :0")
        .local_addr()
        .expect("addr")
        .port()
}

/// A running `gsd-meta-manager bridge`, killed with its process group on drop.
struct Bridge {
    child: Child,
    port: u16,
}

/// What a finished bridge left behind.
struct Finished {
    code: Option<i32>,
    stdout: String,
    stderr: String,
}

impl Bridge {
    /// Start the real subcommand with `command` after `--`. The bridge leads its
    /// own process group so [`Drop`] can take its child down with it.
    fn start(port: u16, sock: &Path, command: &[&str]) -> Bridge {
        let child = Command::new(env!("CARGO_BIN_EXE_gsd-meta-manager"))
            .arg("bridge")
            .arg("--port")
            .arg(port.to_string())
            .arg("--sock")
            .arg(sock)
            .arg("--")
            .args(command)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .process_group(0)
            .spawn()
            .expect("start the bridge");
        Bridge { child, port }
    }

    /// Connect to the bridge's port, retrying for up to [`BOUND`] while it binds.
    fn connect(&self) -> TcpStream {
        let deadline = Instant::now() + BOUND;
        loop {
            match TcpStream::connect(("127.0.0.1", self.port)) {
                Ok(stream) => {
                    stream.set_read_timeout(Some(BOUND)).expect("timeout");
                    return stream;
                }
                Err(e) if Instant::now() < deadline => {
                    let _ = e;
                    std::thread::sleep(Duration::from_millis(20));
                }
                Err(e) => panic!("the bridge never listened on {}: {e}", self.port),
            }
        }
    }

    /// Wait up to [`BOUND`] for the bridge to exit; `None` if it is still running.
    fn wait_bounded(&mut self) -> Option<std::process::ExitStatus> {
        let deadline = Instant::now() + BOUND;
        loop {
            if let Some(status) = self.child.try_wait().expect("try_wait") {
                return Some(status);
            }
            if Instant::now() >= deadline {
                return None;
            }
            std::thread::sleep(Duration::from_millis(20));
        }
    }

    /// Wait for the bridge to exit on its own and collect its output. A bridge
    /// still running after [`BOUND`] fails the test (and is killed on drop).
    fn finish(mut self) -> Finished {
        let status = self
            .wait_bounded()
            .unwrap_or_else(|| panic!("the bridge was still running after {BOUND:?}"));
        Finished {
            code: status.code().or_else(|| {
                use std::os::unix::process::ExitStatusExt;
                status.signal().map(|s| 128 + s)
            }),
            stdout: read_all(self.child.stdout.take()),
            stderr: read_all(self.child.stderr.take()),
        }
    }

    /// Kill the bridge and its child, then collect what was written to stdout.
    fn kill_and_stdout(mut self) -> String {
        self.kill_group();
        let _ = self.child.wait();
        read_all(self.child.stdout.take())
    }

    fn kill_group(&mut self) {
        let pgid = self.child.id();
        let _ = Command::new("kill")
            .args(["-KILL", "--"])
            .arg(format!("-{pgid}"))
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status();
        let _ = self.child.kill();
    }
}

impl Drop for Bridge {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            self.kill_group();
            let _ = self.child.wait();
        }
    }
}

fn read_all<R: Read>(pipe: Option<R>) -> String {
    let mut out = String::new();
    if let Some(mut pipe) = pipe {
        let mut bytes = Vec::new();
        let _ = pipe.read_to_end(&mut bytes);
        out = String::from_utf8_lossy(&bytes).into_owned();
    }
    out
}

/// `POST /v1/messages` the way Claude Code sends it: the dummy credential the
/// egress server replaces.
fn inference_request() -> Vec<u8> {
    let body = r#"{"model":"m","messages":[]}"#;
    format!(
        "POST /v1/messages HTTP/1.1\r\n\
         host: localhost:3128\r\n\
         authorization: Bearer {DUMMY_TOKEN}\r\n\
         anthropic-version: 2023-06-01\r\n\
         content-type: application/json\r\n\
         content-length: {}\r\n\
         connection: close\r\n\r\n{body}",
        body.len()
    )
    .into_bytes()
}

/// Send `request` on `client` and read until the far side closes.
fn exchange(mut client: TcpStream, request: &[u8]) -> String {
    client.write_all(request).expect("write the request");
    let mut out = Vec::new();
    let _ = client.read_to_end(&mut out);
    String::from_utf8_lossy(&out).into_owned()
}

fn status_line(response: &str) -> &str {
    response.split("\r\n").next().unwrap_or("")
}

// ------------------------------------------------------------------- tests

#[test]
fn the_bridge_pumps_tcp_to_the_egress_socket() {
    let stub = Stub::start();
    let egress = Egress::start(&stub);
    let port = free_port();
    let bridge = Bridge::start(port, &egress.sock, &["sh", "-c", "sleep 30"]);

    let response = exchange(bridge.connect(), &inference_request());

    assert!(
        status_line(&response).starts_with("HTTP/1.1 200"),
        "{response}"
    );
    assert!(response.ends_with("{}"), "{response}");
    let heads = stub.heads();
    assert_eq!(heads.len(), 1, "exactly one upstream request: {heads:?}");
    assert!(
        heads[0].contains(&format!("Bearer {REAL_TOKEN}")),
        "the stub did not see the injected token: {}",
        heads[0]
    );
    assert!(
        !heads[0].contains(DUMMY_TOKEN),
        "the client's credential reached the upstream: {}",
        heads[0]
    );
    drop(bridge);
}

#[test]
fn the_bridge_exits_with_its_childs_code() {
    let dir = scratch();
    let bridge = Bridge::start(
        free_port(),
        &dir.path().join("no-socket"),
        &["sh", "-c", "exit 7"],
    );

    let finished = bridge.finish();

    assert_eq!(finished.code, Some(7), "stderr: {}", finished.stderr);
}
