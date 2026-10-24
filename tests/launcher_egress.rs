#![cfg(target_os = "linux")]
// ============================================================================
// The launcher-side egress proxy, end to end in CI (Phase 28.1, plan 01).
//
// Every test drives a real `EgressServer` over a real unix socket with raw
// HTTP/1 bytes, against an in-process plain-TCP stub standing in for
// `api.anthropic.com` (`Upstream::Plain`). There is no network and no bwrap.
// "The stub saw zero connections" is the assertion that matters for every
// refusal: a refused request must be decided before anything is dialled.
// ============================================================================

use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream};
use std::os::unix::net::{UnixListener, UnixStream};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use gsd_meta_manager::launcher::egress::upstream::Upstream;
use gsd_meta_manager::launcher::egress::{
    EgressHandle, EgressServer, EgressSpec, EgressStats, TokenProvider,
};
use tempfile::TempDir;

const REAL_TOKEN: &str = "REAL-TOKEN-SENTINEL";
const DUMMY_TOKEN: &str = "sk-ant-oat01-DUMMY-placeholder-not-a-real-token";

// ----------------------------------------------------------------- the stub

/// What the stub upstream received on one connection.
#[derive(Clone, Debug)]
struct Seen {
    head: String,
    body: Vec<u8>,
}

impl Seen {
    /// Every value of header `name` (case-insensitive), in order.
    fn header(&self, name: &str) -> Vec<String> {
        self.head
            .split("\r\n")
            .skip(1)
            .filter_map(|line| line.split_once(':'))
            .filter(|(n, _)| n.trim().eq_ignore_ascii_case(name))
            .map(|(_, v)| v.trim().to_owned())
            .collect()
    }

    fn request_line(&self) -> &str {
        self.head.split("\r\n").next().unwrap_or("")
    }
}

/// A stand-in for `api.anthropic.com`: records each request, answers `{}`.
struct Stub {
    addr: SocketAddr,
    seen: Arc<Mutex<Vec<Seen>>>,
    connections: Arc<AtomicUsize>,
    stop: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

impl Stub {
    fn start() -> Stub {
        let listener = TcpListener::bind("127.0.0.1:0").expect("bind the stub");
        listener.set_nonblocking(true).expect("nonblocking");
        let addr = listener.local_addr().expect("addr");
        let seen = Arc::new(Mutex::new(Vec::new()));
        let connections = Arc::new(AtomicUsize::new(0));
        let stop = Arc::new(AtomicBool::new(false));
        let (seen_t, connections_t, stop_t) = (
            Arc::clone(&seen),
            Arc::clone(&connections),
            Arc::clone(&stop),
        );
        let thread = std::thread::spawn(move || loop {
            match listener.accept() {
                Ok((stream, _)) => {
                    connections_t.fetch_add(1, Ordering::SeqCst);
                    let seen = Arc::clone(&seen_t);
                    std::thread::spawn(move || stub_connection(stream, &seen));
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
            seen,
            connections,
            stop,
            thread: Some(thread),
        }
    }

    /// Connections the stub accepted, counted at accept time.
    fn connections(&self) -> usize {
        self.connections.load(Ordering::SeqCst)
    }

    fn seen(&self) -> Vec<Seen> {
        self.seen.lock().expect("lock").clone()
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

fn stub_connection(mut stream: TcpStream, seen: &Mutex<Vec<Seen>>) {
    let _ = stream.set_nonblocking(false);
    let _ = stream.set_read_timeout(Some(Duration::from_secs(5)));
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
    seen.lock().expect("lock").push(Seen {
        head,
        body: buf[end..end + length].to_vec(),
    });
    let _ =
        stream.write_all(b"HTTP/1.1 200 OK\r\nContent-Length: 2\r\nConnection: close\r\n\r\n{}");
}

// -------------------------------------------------------------- the server

/// A running egress server with its socket and a scratch directory.
struct Rig {
    /// Keeps the scratch directory (and so the socket path) alive.
    _dir: TempDir,
    sock: PathBuf,
    handle: Option<EgressHandle>,
}

impl Rig {
    fn start(upstream: SocketAddr, token: TokenProvider, allow_hosts: &[&str]) -> Rig {
        let dir = tempfile::Builder::new()
            .prefix("e")
            .tempdir_in("/tmp")
            .expect("a short temp dir");
        let sock = dir.path().join("egress.sock");
        let listener = UnixListener::bind(&sock).expect("bind the egress socket");
        let handle = EgressServer::start(
            listener,
            EgressSpec {
                allow_hosts: allow_hosts.iter().map(|h| (*h).to_owned()).collect(),
                token,
                upstream: Upstream::Plain(upstream),
                log_path: dir.path().join("egress.log"),
            },
        )
        .expect("start the egress server");
        Rig {
            _dir: dir,
            sock,
            handle: Some(handle),
        }
    }

    fn with_static_token(stub: &Stub) -> Rig {
        Rig::start(stub.addr, TokenProvider::Static(REAL_TOKEN.to_owned()), &[])
    }

    /// Stop the server and return its counters.
    fn stop(mut self) -> EgressStats {
        self.handle.take().expect("running").stop()
    }

    /// Send `raw` and read the whole response (the server closes after one).
    fn exchange(&self, raw: &[u8]) -> String {
        let mut client = UnixStream::connect(&self.sock).expect("connect");
        client
            .set_read_timeout(Some(Duration::from_secs(10)))
            .expect("timeout");
        client.write_all(raw).expect("write the request");
        let mut out = Vec::new();
        let _ = client.read_to_end(&mut out);
        String::from_utf8_lossy(&out).into_owned()
    }
}

/// `POST /v1/messages?beta=true` the way Claude Code sends it, plus the three
/// credentials the proxy must strip.
fn inference_request(path: &str, body: &str) -> Vec<u8> {
    format!(
        "POST {path} HTTP/1.1\r\n\
         host: localhost:3128\r\n\
         authorization: Bearer {DUMMY_TOKEN}\r\n\
         x-api-key: {DUMMY_TOKEN}\r\n\
         cookie: session={DUMMY_TOKEN}\r\n\
         anthropic-version: 2023-06-01\r\n\
         anthropic-beta: oauth-2025-04-20\r\n\
         x-stainless-os: Linux\r\n\
         user-agent: claude-cli/2.1.310 (external, sdk-cli)\r\n\
         content-type: application/json\r\n\
         content-length: {}\r\n\
         connection: keep-alive\r\n\r\n{body}",
        body.len()
    )
    .into_bytes()
}

fn status_line(response: &str) -> &str {
    response.split("\r\n").next().unwrap_or("")
}

// ------------------------------------------------------------------- tests

#[test]
fn an_inference_request_is_forwarded_with_the_real_token() {
    let stub = Stub::start();
    let rig = Rig::with_static_token(&stub);
    let body = r#"{"model":"m","messages":[],"tools":[{"name":"Bash","input_schema":{}}]}"#;

    let response = rig.exchange(&inference_request("/v1/messages?beta=true", body));

    assert!(
        status_line(&response).starts_with("HTTP/1.1 200"),
        "{response}"
    );
    assert!(response.ends_with("{}"), "{response}");
    let seen = stub.seen();
    assert_eq!(seen.len(), 1, "exactly one upstream request");
    let req = &seen[0];
    assert_eq!(req.request_line(), "POST /v1/messages?beta=true HTTP/1.1");
    assert_eq!(
        req.header("authorization"),
        [format!("Bearer {REAL_TOKEN}")],
        "exactly one Authorization, the real one"
    );
    assert!(req.header("x-api-key").is_empty(), "{}", req.head);
    assert!(req.header("cookie").is_empty(), "{}", req.head);
    assert_eq!(req.header("host"), ["api.anthropic.com"]);
    assert_eq!(req.header("connection"), ["close"]);
    assert_eq!(req.header("content-length"), [body.len().to_string()]);
    assert_eq!(req.header("anthropic-version"), ["2023-06-01"]);
    assert_eq!(req.header("anthropic-beta"), ["oauth-2025-04-20"]);
    assert_eq!(req.header("x-stainless-os"), ["Linux"]);
    assert_eq!(
        req.header("user-agent"),
        ["claude-cli/2.1.310 (external, sdk-cli)"]
    );
    assert!(
        !req.head.contains(DUMMY_TOKEN),
        "the client's credential reached the upstream: {}",
        req.head
    );
    assert_eq!(req.body, body.as_bytes());
}

#[test]
fn account_paths_are_refused_without_reaching_upstream() {
    let stub = Stub::start();
    let rig = Rig::with_static_token(&stub);
    for target in [
        "/v1/code/sessions",
        "/v1/code/triggers",
        "/v1/mcp_servers?limit=1",
        "/v1/environments/bridge",
    ] {
        let raw = format!("GET {target} HTTP/1.1\r\nhost: x\r\nauthorization: Bearer d\r\n\r\n");
        let response = rig.exchange(raw.as_bytes());
        assert!(
            status_line(&response).starts_with("HTTP/1.1 403"),
            "{target}: {response}"
        );
        assert!(
            response.contains("permission_error"),
            "{target}: {response}"
        );
    }
    assert_eq!(stub.connections(), 0, "a refused request dialled upstream");
}

#[test]
fn absolute_form_targets_are_refused() {
    let stub = Stub::start();
    let rig = Rig::with_static_token(&stub);
    for raw in [
        "POST http://api.anthropic.com/v1/messages HTTP/1.1\r\nhost: api.anthropic.com\r\ncontent-length: 2\r\n\r\n{}",
        "GET http://evil.example/x HTTP/1.1\r\nhost: evil.example\r\n\r\n",
    ] {
        let response = rig.exchange(raw.as_bytes());
        assert!(
            status_line(&response).starts_with("HTTP/1.1 403"),
            "{raw}: {response}"
        );
    }
    assert_eq!(stub.connections(), 0, "a refused request dialled upstream");
}

#[test]
fn count_tokens_is_forwarded() {
    let stub = Stub::start();
    let rig = Rig::with_static_token(&stub);
    let body = r#"{"model":"m","messages":[]}"#;

    let response = rig.exchange(&inference_request("/v1/messages/count_tokens", body));

    assert!(
        status_line(&response).starts_with("HTTP/1.1 200"),
        "{response}"
    );
    let seen = stub.seen();
    assert_eq!(seen.len(), 1);
    assert_eq!(
        seen[0].request_line(),
        "POST /v1/messages/count_tokens HTTP/1.1"
    );
    assert_eq!(
        seen[0].header("authorization"),
        [format!("Bearer {REAL_TOKEN}")]
    );
    assert_eq!(rig.stop().inference_forwarded, 1);
}
