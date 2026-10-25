//! The launcher-side egress server (Phase 28.1, plan 01; D-04, D-05, D-06, D-09).
//!
//! The sandbox keeps no network (28.1-10); the only path out is a unix socket this
//! module serves from the HOST side of the boundary. One socket, one dispatch on
//! the request-target form (RESEARCH Pattern 1):
//!
//! - an origin-form `POST /v1/messages[...]` is the **inference reverse proxy**:
//!   the request is parsed, judged by [`policy`], re-serialised with the real run
//!   token injected, sent to the upstream with `Connection: close`, and the
//!   response is pumped back unchanged. The token exists only here, in
//!   [`TokenProvider`]; whatever credential the driver sent is dropped.
//! - a `CONNECT host:443` is the **tunnel proxy**: deny by default, allowed only
//!   for names on the run's allowlist whose DNS answer is entirely public. The
//!   name is resolved once, every denied address is dropped, and the vetted
//!   address is dialled (no re-resolve), so a name cannot be rebound onto the
//!   host, the LAN or a metadata service. `api.anthropic.com` is never tunnelled.
//! - anything else is refused with a 403 and nothing is dialled.
//!
//! Every request leaves one header-free line in `egress.log` ([`log`]), written
//! BEFORE the client sees the answer.
//!
//! Every decision is a pure function in [`policy`]; this module only does I/O and
//! acts on the verdict. The upstream is a parameter ([`upstream::Upstream`]), so
//! the whole path runs in CI against a plain-TCP stub of `api.anthropic.com`.
//!
//! The server runs on its own thread (`gsdmm-egress`) with a current-thread tokio
//! runtime, the shape of `driverctl::serve::install_signal_handlers`: the launcher
//! is synchronous std threads and never enters an async context. No `async fn` here
//! holds a blocking call (`tests/async_blocking_guard.rs`).

pub mod http1;
pub mod log;
pub mod policy;
pub mod upstream;

use std::fmt;
use std::io;
use std::net::SocketAddr;
use std::os::unix::net::UnixListener as StdUnixListener;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpStream, UnixStream};
use tokio::sync::{oneshot, OwnedSemaphorePermit, Semaphore};
use tokio::task::JoinSet;

use self::http1::{Head, ReadError};
use self::log::{EgressLog, Kind};
use self::policy::{BodyVerdict, Target};
use self::upstream::{Connector, Upstream};

/// The one host inference requests go to, and the host `CONNECT` never tunnels.
pub const INFERENCE_HOST: &str = "api.anthropic.com";

/// Connections served at once; the next one is answered `503` (RESEARCH Pitfall 6).
pub const MAX_CONNECTIONS: usize = 64;

/// How long [`EgressServer::start`] waits for the listener to be live.
const READY_TIMEOUT: Duration = Duration::from_secs(5);

/// How long a response to the client may take to write. A client that does not
/// read its error does not hold a permit forever.
const REPLY_TIMEOUT: Duration = Duration::from_secs(10);

/// The most memory the request bodies in flight may hold, in "weighted bytes" (WR-02): a
/// body is weighed at [`BODY_WEIGHT`] times its length, covering the second copy the
/// forwarded request needs and the `serde_json::Value` the judge builds. The proxy runs
/// in the launcher, outside the scope's `MemoryMax`, so without this a driver could
/// make 64 connections hold 32 MiB each and have the judge expand each of them.
/// 512 MiB `[inferred — audit]`: about 4 maximum-size bodies, or hundreds of ordinary ones.
pub const BODY_MEMORY_BUDGET: u32 = 512 * 1024 * 1024;

/// The weight of one body byte against [`BODY_MEMORY_BUDGET`] `[inferred — audit]`.
const BODY_WEIGHT: usize = 4;

/// How long a request waits for body memory before it is answered `503`.
const BODY_BUDGET_WAIT: Duration = Duration::from_secs(10);

/// The longest a `CONNECT` tunnel may carry no byte in either direction (WR-02)
/// `[inferred — audit]`.
pub const TUNNEL_IDLE: Duration = Duration::from_secs(300);

/// The longest a `CONNECT` tunnel may stay open (WR-02) `[inferred — audit]`.
pub const TUNNEL_LIFETIME: Duration = Duration::from_secs(3600);

/// The longest the upstream may stay silent before or during a response (WR-02). A
/// non-streaming completion sends nothing until it is done, which can take minutes, so
/// this is generous `[inferred — audit]`.
const UPSTREAM_IDLE: Duration = Duration::from_secs(600);

/// How long one write of the response to the client may take. A client that stopped
/// reading does not pin a connection slot (WR-02).
const CLIENT_WRITE_TIMEOUT: Duration = Duration::from_secs(60);

/// The longest one relayed response may take in all (WR-02) `[inferred — audit]`.
const RELAY_LIFETIME: Duration = Duration::from_secs(7200);

/// The run token as the proxy sees it. The driver never does.
#[derive(Clone)]
pub enum TokenProvider {
    /// A token that does not change (the stored `claude setup-token` token).
    Static(String),
    /// The host's access token: `initial` until a request finds a newer one in
    /// `<claude_dir>/.credentials.json`.
    Host {
        claude_dir: PathBuf,
        initial: String,
    },
}

impl fmt::Debug for TokenProvider {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenProvider::Static(_) => f.write_str("Static(<redacted>)"),
            TokenProvider::Host { claude_dir, .. } => write!(
                f,
                "Host {{ claude_dir: {claude_dir:?}, initial: <redacted> }}"
            ),
        }
    }
}

/// What one run's egress server is configured with.
#[derive(Clone)]
pub struct EgressSpec {
    /// Hosts a `CONNECT` may name (D-07: empty by default).
    pub allow_hosts: Vec<String>,
    pub token: TokenProvider,
    pub upstream: Upstream,
    /// Where the header-free `egress.log` goes.
    pub log_path: PathBuf,
}

impl fmt::Debug for EgressSpec {
    // Hand-written so the token can never be printed. Not `debug_struct`: its
    // `.finish()` is the spelling `tests/spawn_seam_guard.rs` reserves for the
    // journal's terminal write.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "EgressSpec {{ allow_hosts: {:?}, token: {:?}, upstream: {:?}, log_path: {:?} }}",
            self.allow_hosts, self.token, self.upstream, self.log_path
        )
    }
}

/// What the server did over its life, read when it stops (D-14).
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct EgressStats {
    /// `/v1/messages`-family requests whose upstream status line arrived.
    pub inference_forwarded: u64,
    pub inference_denied: u64,
    pub connect_allowed: u64,
    pub connect_denied: u64,
}

#[derive(Default)]
struct Counters {
    inference_forwarded: AtomicU64,
    inference_denied: AtomicU64,
    connect_allowed: AtomicU64,
    connect_denied: AtomicU64,
}

impl Counters {
    fn snapshot(&self) -> EgressStats {
        EgressStats {
            inference_forwarded: self.inference_forwarded.load(Ordering::SeqCst),
            inference_denied: self.inference_denied.load(Ordering::SeqCst),
            connect_allowed: self.connect_allowed.load(Ordering::SeqCst),
            connect_denied: self.connect_denied.load(Ordering::SeqCst),
        }
    }
}

/// Starts the server. A unit type: the state lives in the [`EgressHandle`].
pub struct EgressServer;

impl EgressServer {
    /// Serve `listener` on a dedicated thread and return once it is accepting.
    ///
    /// `listener` is an already-bound unix listener (the launcher binds it 0600
    /// with `driverctl::serve::bind_run_socket`; tests bind their own). The
    /// caller blocks for at most five seconds for the readiness handshake, so the
    /// socket is live before the sandbox that connects to it starts.
    pub fn start(listener: StdUnixListener, spec: EgressSpec) -> io::Result<EgressHandle> {
        listener.set_nonblocking(true)?;
        let log = EgressLog::open(&spec.log_path)?;
        let counters = Arc::new(Counters::default());
        let (stop_tx, stop_rx) = oneshot::channel::<()>();
        let (ready_tx, ready_rx) = std::sync::mpsc::channel::<Result<(), String>>();
        let thread_counters = Arc::clone(&counters);
        let thread = std::thread::Builder::new()
            .name("gsdmm-egress".to_owned())
            .spawn(move || {
                let runtime = tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build();
                match runtime {
                    Ok(runtime) => {
                        runtime.block_on(run(
                            listener,
                            spec,
                            log,
                            stop_rx,
                            thread_counters,
                            ready_tx,
                        ));
                    }
                    Err(e) => {
                        let _ = ready_tx.send(Err(e.to_string()));
                    }
                }
            })?;
        let handle = EgressHandle {
            stop: Some(stop_tx),
            thread: Some(thread),
            counters,
        };
        match ready_rx.recv_timeout(READY_TIMEOUT) {
            Ok(Ok(())) => Ok(handle),
            Ok(Err(e)) => Err(io::Error::other(format!("start the egress server: {e}"))),
            Err(e) => Err(io::Error::other(format!("start the egress server: {e}"))),
        }
    }
}

/// A running egress server. Dropping it stops the server too.
pub struct EgressHandle {
    stop: Option<oneshot::Sender<()>>,
    thread: Option<std::thread::JoinHandle<()>>,
    counters: Arc<Counters>,
}

impl EgressHandle {
    /// Stop the server and return its counters.
    ///
    /// Open connections are cut: no request outlives the handle. Returns once
    /// the server thread has ended.
    pub fn stop(mut self) -> EgressStats {
        self.shutdown();
        self.counters.snapshot()
    }

    fn shutdown(&mut self) {
        if let Some(stop) = self.stop.take() {
            let _ = stop.send(());
        }
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl Drop for EgressHandle {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// `<dir of the driverctl socket>/egress-<hash8(run dir)>.sock`: the egress socket
/// sits beside the driverctl socket, in the same private directory.
/// `canonical_run_dir` must already be canonical for the hash to be stable.
pub fn socket_path_for(driverctl_sock: &Path, canonical_run_dir: &Path) -> PathBuf {
    let dir = driverctl_sock
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    dir.join(format!(
        "egress-{}.sock",
        crate::driverctl::serve::hash8(canonical_run_dir)
    ))
}

/// The token as the handlers keep it.
enum TokenState {
    Static(String),
    Host {
        claude_dir: PathBuf,
        /// The newest good token seen; the one used when the file is unreadable.
        last_good: tokio::sync::Mutex<String>,
    },
}

/// Everything a connection handler reads.
struct Ctx {
    token: TokenState,
    allow_hosts: Vec<String>,
    connector: Connector,
    counters: Arc<Counters>,
    log: EgressLog,
    /// What is left of [`BODY_MEMORY_BUDGET`].
    body_budget: Arc<Semaphore>,
}

impl Ctx {
    /// The token to inject now.
    ///
    /// A host token is re-read from `<claude_dir>/.credentials.json` on every
    /// request (a small file the host's own `claude` keeps fresh), so a long run
    /// outlives the access token it started with. An unreadable, malformed,
    /// missing or nearly expired file keeps the last good value (RESEARCH
    /// Pitfall 4). The read is a blocking one and goes through `spawn_blocking`.
    async fn current_token(&self) -> String {
        match &self.token {
            TokenState::Static(token) => token.clone(),
            TokenState::Host {
                claude_dir,
                last_good,
            } => {
                let dir = claude_dir.clone();
                let fresh = tokio::task::spawn_blocking(move || {
                    crate::launcher::auth::host_access_token(&dir, chrono::Utc::now())
                })
                .await;
                let mut last_good = last_good.lock().await;
                if let Ok(Ok(token)) = fresh {
                    *last_good = token.value;
                }
                last_good.clone()
            }
        }
    }

    /// Log the request as denied, THEN answer the client, so the log line exists
    /// by the time the client sees the answer.
    async fn deny(&self, client: &mut UnixStream, req: &Req, status: u16, reason: &str) {
        self.log
            .record(
                req.kind,
                &req.subject,
                &req.path,
                status,
                &format!("denied: {reason}"),
            )
            .await;
        send(client, &http1::error_response(status, reason)).await;
    }

    /// [`Ctx::deny`] for an inference request, counting a policy refusal (400,
    /// 403, 413) in `inference_denied`; a timeout or an upstream fault is not one.
    async fn deny_inference(&self, client: &mut UnixStream, req: &Req, status: u16, reason: &str) {
        if matches!(status, 400 | 403 | 413) {
            self.counters
                .inference_denied
                .fetch_add(1, Ordering::SeqCst);
        }
        self.deny(client, req, status, reason).await;
    }
}

/// What a log line says about the request in hand.
struct Req {
    kind: Kind,
    /// The method, or `CONNECT host:port`.
    subject: String,
    /// The path without its query, or `-`.
    path: String,
}

async fn run(
    listener: StdUnixListener,
    spec: EgressSpec,
    log: EgressLog,
    mut stop: oneshot::Receiver<()>,
    counters: Arc<Counters>,
    ready: std::sync::mpsc::Sender<Result<(), String>>,
) {
    let listener = match tokio::net::UnixListener::from_std(listener) {
        Ok(listener) => listener,
        Err(e) => {
            let _ = ready.send(Err(e.to_string()));
            return;
        }
    };
    let connector = match Connector::new(spec.upstream) {
        Ok(connector) => connector,
        Err(e) => {
            let _ = ready.send(Err(e.to_string()));
            return;
        }
    };
    let token = match spec.token {
        TokenProvider::Static(token) => TokenState::Static(token),
        TokenProvider::Host {
            claude_dir,
            initial,
        } => TokenState::Host {
            claude_dir,
            last_good: tokio::sync::Mutex::new(initial),
        },
    };
    let ctx = Arc::new(Ctx {
        token,
        allow_hosts: spec.allow_hosts,
        connector,
        counters,
        log,
        body_budget: Arc::new(Semaphore::new(BODY_MEMORY_BUDGET as usize)),
    });
    let _ = ready.send(Ok(()));

    let permits = Arc::new(Semaphore::new(MAX_CONNECTIONS));
    let mut tasks: JoinSet<()> = JoinSet::new();
    loop {
        tokio::select! {
            _ = &mut stop => break,
            accepted = listener.accept() => match accepted {
                Ok((stream, _)) => admit(stream, &permits, &ctx, &mut tasks),
                Err(_) => tokio::time::sleep(Duration::from_millis(50)).await,
            },
            Some(_) = tasks.join_next(), if !tasks.is_empty() => {}
        }
    }
    tasks.abort_all();
    while tasks.join_next().await.is_some() {}
}

/// Serve `stream` if a connection slot is free, else answer `503` and close.
fn admit(stream: UnixStream, permits: &Arc<Semaphore>, ctx: &Arc<Ctx>, tasks: &mut JoinSet<()>) {
    match Arc::clone(permits).try_acquire_owned() {
        Ok(permit) => {
            let ctx = Arc::clone(ctx);
            tasks.spawn(handle(stream, ctx, permit));
        }
        Err(_) => {
            // A fresh socket's send buffer is empty: one small non-blocking
            // write lands or is dropped, and the accept loop never waits on it.
            let _ = stream.try_write(&http1::error_response(
                503,
                "too many concurrent connections",
            ));
        }
    }
}

/// Write `bytes` to the client within [`REPLY_TIMEOUT`] and close; a failure is
/// the client's loss and is ignored.
async fn send(client: &mut UnixStream, bytes: &[u8]) {
    let _ = tokio::time::timeout(REPLY_TIMEOUT, async {
        client.write_all(bytes).await?;
        client.shutdown().await
    })
    .await;
}

async fn handle(mut client: UnixStream, ctx: Arc<Ctx>, _permit: OwnedSemaphorePermit) {
    let (head, leftover) = match http1::read_head(&mut client).await {
        Ok(read) => read,
        Err(e) => {
            // A connection that never produced a complete head is not a request:
            // it is answered where there is anyone to answer, and not logged.
            if let Some((status, reason)) = read_error_reply(&e) {
                let req = Req {
                    kind: Kind::Refused,
                    subject: "-".to_owned(),
                    path: "-".to_owned(),
                };
                ctx.deny(&mut client, &req, status, reason).await;
            }
            return;
        }
    };
    match policy::classify_target(&head.method, &head.target) {
        Target::Inference { path, query } => {
            inference(&mut client, &ctx, &head, leftover, path, query).await;
        }
        Target::Connect { host, port } => {
            connect(&mut client, &ctx, leftover, host, port).await;
        }
        Target::Refuse(reason) => {
            let req = Req {
                kind: Kind::Refused,
                subject: head.method.clone(),
                path: head.target.split('?').next().unwrap_or("").to_owned(),
            };
            ctx.deny(&mut client, &req, 403, &reason).await;
        }
    }
}

/// The status and reason to answer a failed read with, or `None` when the
/// connection is already gone and there is nobody to answer.
fn read_error_reply(e: &ReadError) -> Option<(u16, &'static str)> {
    match e {
        ReadError::Timeout => Some((408, "the request was not completed in time")),
        ReadError::HeadTooLarge => Some((431, "request head is too large")),
        ReadError::BodyTooLarge => Some((413, "request body is too large")),
        ReadError::Malformed(_) => Some((400, "request is not valid HTTP/1")),
        ReadError::Closed | ReadError::Io(_) => None,
    }
}

async fn inference(
    client: &mut UnixStream,
    ctx: &Ctx,
    head: &Head,
    leftover: Vec<u8>,
    path: String,
    query: Option<String>,
) {
    let req = Req {
        kind: Kind::Inference,
        subject: "POST".to_owned(),
        path: path.clone(),
    };
    let target = match &query {
        Some(query) => format!("{path}?{query}"),
        None => path,
    };
    let framing = match policy::framing_verdict(&head.headers) {
        Ok(framing) => framing,
        Err(denial) => {
            return ctx
                .deny_inference(client, &req, denial.status(), denial.reason())
                .await
        }
    };
    if policy::wants_continue(&head.headers) {
        // Answered here: the interim response never comes from the upstream.
        let sent = tokio::time::timeout(
            REPLY_TIMEOUT,
            client.write_all(b"HTTP/1.1 100 Continue\r\n\r\n"),
        )
        .await;
        if !matches!(sent, Ok(Ok(()))) {
            return;
        }
    }
    // WR-02: the memory the body will take is reserved before it is read, so the bodies
    // in flight stay within `BODY_MEMORY_BUDGET`.
    let weight = body_weight(framing);
    let reserved = tokio::time::timeout(
        BODY_BUDGET_WAIT,
        Arc::clone(&ctx.body_budget).acquire_many_owned(weight),
    )
    .await;
    let Ok(Ok(reservation)) = reserved else {
        return ctx
            .deny_inference(client, &req, 503, "too many request bodies in flight")
            .await;
    };
    let body = match http1::read_body(client, leftover, framing).await {
        Ok(body) => body,
        Err(e) => {
            return match read_error_reply(&e) {
                Some((status, reason)) => ctx.deny_inference(client, &req, status, reason).await,
                None => {
                    ctx.log
                        .record(
                            req.kind,
                            &req.subject,
                            &req.path,
                            0,
                            "aborted: the client closed before the body was complete",
                        )
                        .await;
                }
            };
        }
    };
    if let BodyVerdict::Deny(reason) = policy::judge_body(&body) {
        return ctx.deny_inference(client, &req, 403, &reason).await;
    }
    let token = ctx.current_token().await;
    let headers = policy::forwarded_headers(&head.headers);
    let Some(request) = http1::request_bytes("POST", &target, &headers, &token, &body) else {
        return ctx
            .deny_inference(client, &req, 502, "no usable credential for the upstream")
            .await;
    };
    drop(body);
    let mut up = match ctx.connector.connect().await {
        Ok(up) => up,
        Err(_) => {
            return ctx
                .deny_inference(client, &req, 502, "the upstream is unreachable")
                .await
        }
    };
    if up.write_all(&request).await.is_err() || up.flush().await.is_err() {
        return ctx
            .deny_inference(client, &req, 502, "the upstream refused the request")
            .await;
    }
    // The request is on the wire: its memory is free for the next body while the
    // response, which can take minutes, is relayed.
    drop(request);
    drop(reservation);
    relay_response(&mut up, client, ctx, &req).await;
}

/// What a body reserves of [`BODY_MEMORY_BUDGET`] before it is read: its announced
/// length (the maximum for a chunked body, whose length is unknown) times
/// [`BODY_WEIGHT`], at least 1 and at most the whole budget.
fn body_weight(framing: policy::Framing) -> u32 {
    let announced = match framing {
        policy::Framing::Length(len) => usize::try_from(len).unwrap_or(usize::MAX),
        policy::Framing::Chunked => http1::BODY_MAX_BYTES,
    };
    let weight = announced
        .min(http1::BODY_MAX_BYTES)
        .saturating_mul(BODY_WEIGHT)
        .clamp(1, BODY_MEMORY_BUDGET as usize);
    u32::try_from(weight).unwrap_or(BODY_MEMORY_BUDGET)
}

/// The status code of an HTTP/1.x status line, or `None`.
fn parse_status(first: &[u8]) -> Option<u16> {
    let code = first.get(9..12)?;
    if first.get(8) != Some(&b' ') || !code.iter().all(u8::is_ascii_digit) {
        return None;
    }
    std::str::from_utf8(code).ok()?.parse().ok()
}

/// Pump the upstream's response to the client until the upstream ends it.
///
/// The bytes are not interpreted beyond the status line, which is read to log and
/// count the request as forwarded: the upstream was asked for `Connection: close`,
/// so its end of stream is the end of the response whatever framing it used, and
/// the client parses that framing itself. The log line is written before the first
/// byte reaches the client.
async fn relay_response(
    up: &mut upstream::UpstreamStream,
    client: &mut UnixStream,
    ctx: &Ctx,
    req: &Req,
) {
    let mut chunk = [0u8; 8192];
    let mut first: Vec<u8> = Vec::new();
    // The first read(s) up to the end of the status line, so a non-HTTP answer is
    // not passed on as one.
    while !first.contains(&b'\n') && first.len() < chunk.len() {
        match tokio::time::timeout(UPSTREAM_IDLE, up.read(&mut chunk)).await {
            Ok(Ok(0) | Err(_)) => break,
            Ok(Ok(n)) => first.extend_from_slice(&chunk[..n]),
            Err(_) => {
                return ctx
                    .deny_inference(client, req, 504, "the upstream did not answer in time")
                    .await
            }
        }
    }
    if !first.starts_with(b"HTTP/1.") {
        return ctx
            .deny_inference(client, req, 502, "the upstream gave no valid response")
            .await;
    }
    ctx.counters
        .inference_forwarded
        .fetch_add(1, Ordering::SeqCst);
    ctx.log
        .record(
            req.kind,
            &req.subject,
            &req.path,
            parse_status(&first).unwrap_or(0),
            "forwarded",
        )
        .await;
    // WR-02: no read, write or whole-response wait is unbounded, so a stalled upstream
    // or a client that stopped reading ends here instead of pinning a connection slot.
    let relayed = tokio::time::timeout(RELAY_LIFETIME, async {
        if !write_to_client(client, &first).await {
            return;
        }
        loop {
            match tokio::time::timeout(UPSTREAM_IDLE, up.read(&mut chunk)).await {
                Ok(Ok(0)) => break,
                Ok(Ok(n)) => {
                    if !write_to_client(client, &chunk[..n]).await {
                        return;
                    }
                }
                // A TLS peer that closes without close_notify: the end of the stream.
                Ok(Err(_)) => break,
                Err(_) => return,
            }
        }
        let _ = tokio::time::timeout(CLIENT_WRITE_TIMEOUT, client.shutdown()).await;
    })
    .await;
    drop(relayed);
}

/// One write of response bytes to the client within [`CLIENT_WRITE_TIMEOUT`]; false
/// when it failed or timed out.
async fn write_to_client(client: &mut UnixStream, bytes: &[u8]) -> bool {
    matches!(
        tokio::time::timeout(CLIENT_WRITE_TIMEOUT, client.write_all(bytes)).await,
        Ok(Ok(()))
    )
}

/// How long resolving an allowlisted name may take.
const RESOLVE_TIMEOUT: Duration = Duration::from_secs(10);

/// How long dialling one vetted address may take.
const DIAL_TIMEOUT: Duration = Duration::from_secs(15);

/// The `CONNECT` proxy (D-06): verdict on the name, ONE resolution, every denied
/// address dropped, a dial to the first vetted address that accepts (never a
/// second resolution), then a byte tunnel.
///
/// A refusal for an address names its CLASS (`loopback`) and never the address, in
/// the response and in the log alike.
async fn connect(client: &mut UnixStream, ctx: &Ctx, leftover: Vec<u8>, host: String, port: u16) {
    let req = Req {
        kind: Kind::Connect,
        subject: format!("CONNECT {host}:{port}"),
        path: "-".to_owned(),
    };
    if let Err(denial) = policy::connect_verdict(&host, port, &ctx.allow_hosts) {
        ctx.counters.connect_denied.fetch_add(1, Ordering::SeqCst);
        return ctx
            .deny(client, &req, denial.status(), denial.reason())
            .await;
    }
    let resolved = tokio::time::timeout(
        RESOLVE_TIMEOUT,
        tokio::net::lookup_host((host.as_str(), port)),
    )
    .await;
    let addrs: Vec<SocketAddr> = match resolved {
        Ok(Ok(addrs)) => addrs.collect(),
        _ => {
            return ctx
                .deny(client, &req, 502, "the host could not be resolved")
                .await
        }
    };
    let mut first_class = None;
    let mut vetted = Vec::new();
    for addr in addrs {
        match policy::ip_denied(addr.ip()) {
            Some(class) => {
                first_class.get_or_insert(class);
            }
            None => vetted.push(addr),
        }
    }
    if vetted.is_empty() {
        return match first_class {
            Some(class) => {
                ctx.counters.connect_denied.fetch_add(1, Ordering::SeqCst);
                let reason = format!("the host resolves only to a {class} address");
                ctx.deny(client, &req, 403, &reason).await;
            }
            None => {
                ctx.deny(client, &req, 502, "the host did not resolve")
                    .await
            }
        };
    }
    let mut upstream = None;
    for addr in vetted {
        if let Ok(Ok(stream)) = tokio::time::timeout(DIAL_TIMEOUT, TcpStream::connect(addr)).await {
            upstream = Some(stream);
            break;
        }
    }
    let Some(mut upstream) = upstream else {
        return ctx.deny(client, &req, 502, "the host is unreachable").await;
    };
    ctx.counters.connect_allowed.fetch_add(1, Ordering::SeqCst);
    ctx.log
        .record(req.kind, &req.subject, &req.path, 200, "allowed")
        .await;
    let accepted = tokio::time::timeout(
        REPLY_TIMEOUT,
        client.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n"),
    )
    .await;
    if matches!(accepted, Ok(Ok(()))) {
        let _ = tunnel(client, &mut upstream, &leftover).await;
    }
}

/// The post-handshake pump of an allowed `CONNECT`: `leftover` (bytes the client
/// sent past its request head, a pipelined first flight) goes to the upstream
/// first, then both directions are copied until either side ends. Returns the
/// bytes copied to the upstream and to the client after the leftover. Generic over the two streams
/// so the positive path is unit-tested over `tokio::io::duplex` pairs.
///
/// A tunnel that carries no byte for [`TUNNEL_IDLE`], or stays open for
/// [`TUNNEL_LIFETIME`], ends with a `TimedOut` error (WR-02).
pub async fn tunnel<C, U>(
    client: &mut C,
    upstream: &mut U,
    leftover: &[u8],
) -> io::Result<(u64, u64)>
where
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    U: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    tunnel_within(client, upstream, leftover, TUNNEL_IDLE, TUNNEL_LIFETIME).await
}

fn timed_out(what: &str) -> io::Error {
    io::Error::new(io::ErrorKind::TimedOut, what.to_owned())
}

/// [`tunnel`] with the two limits as parameters (the test seam).
pub async fn tunnel_within<C, U>(
    client: &mut C,
    upstream: &mut U,
    leftover: &[u8],
    idle: Duration,
    lifetime: Duration,
) -> io::Result<(u64, u64)>
where
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    U: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    let both = async {
        if !leftover.is_empty() {
            tokio::time::timeout(idle, upstream.write_all(leftover))
                .await
                .map_err(|_| timed_out("the tunnel stalled"))??;
        }
        let (mut client_read, mut client_write) = tokio::io::split(client);
        let (mut up_read, mut up_write) = tokio::io::split(upstream);
        tokio::try_join!(
            pump(&mut client_read, &mut up_write, idle),
            pump(&mut up_read, &mut client_write, idle)
        )
    };
    match tokio::time::timeout(lifetime, both).await {
        Ok(result) => result,
        Err(_) => Err(timed_out("the tunnel reached its lifetime")),
    }
}

/// Copy `from` to `to` until `from` ends, then shut `to` down (a half-close, as
/// `copy_bidirectional` does). No read or write waits longer than `idle`.
async fn pump<R, W>(from: &mut R, to: &mut W, idle: Duration) -> io::Result<u64>
where
    R: tokio::io::AsyncRead + Unpin,
    W: tokio::io::AsyncWrite + Unpin,
{
    let mut buf = [0u8; 16 * 1024];
    let mut total = 0u64;
    loop {
        let n = tokio::time::timeout(idle, from.read(&mut buf))
            .await
            .map_err(|_| timed_out("the tunnel was idle"))??;
        if n == 0 {
            let _ = tokio::time::timeout(idle, to.shutdown()).await;
            return Ok(total);
        }
        tokio::time::timeout(idle, async {
            to.write_all(&buf[..n]).await?;
            to.flush().await
        })
        .await
        .map_err(|_| timed_out("the tunnel stalled"))??;
        total += n as u64;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{duplex, AsyncReadExt, AsyncWriteExt};

    #[tokio::test]
    async fn the_tunnel_sends_the_leftover_first_then_pumps_both_directions() {
        let (mut client_end, mut client_far) = duplex(1024);
        let (mut up_end, mut up_far) = duplex(1024);
        let pump =
            tokio::spawn(async move { tunnel(&mut client_end, &mut up_end, b"HELLO").await });

        let mut seen = [0u8; 5];
        up_far
            .read_exact(&mut seen)
            .await
            .expect("leftover reaches the upstream");
        assert_eq!(&seen, b"HELLO");

        client_far.write_all(b"ping").await.expect("client write");
        let mut seen = [0u8; 4];
        up_far
            .read_exact(&mut seen)
            .await
            .expect("client bytes reach the upstream");
        assert_eq!(&seen, b"ping");

        up_far.write_all(b"pong").await.expect("upstream write");
        client_far
            .read_exact(&mut seen)
            .await
            .expect("upstream bytes reach the client");
        assert_eq!(&seen, b"pong");

        drop(client_far);
        drop(up_far);
        let (to_upstream, to_client) = pump.await.expect("join").expect("a clean tunnel");
        assert_eq!((to_upstream, to_client), (4, 4));
    }

    #[tokio::test]
    async fn the_tunnel_ends_when_either_side_closes() {
        let (mut client_end, client_far) = duplex(64);
        let (mut up_end, mut up_far) = duplex(64);
        let pump = tokio::spawn(async move { tunnel(&mut client_end, &mut up_end, b"").await });
        drop(client_far);
        let mut rest = Vec::new();
        up_far
            .read_to_end(&mut rest)
            .await
            .expect("the upstream sees EOF");
        assert!(rest.is_empty());
        drop(up_far);
        pump.await.expect("join").expect("a clean close");
    }

    #[test]
    fn a_body_reserves_a_weighted_share_of_the_budget() {
        use policy::Framing;
        assert_eq!(body_weight(Framing::Length(1000)), 4000);
        assert_eq!(body_weight(Framing::Length(0)), 1);
        let max = http1::BODY_MAX_BYTES as u32 * 4;
        assert_eq!(body_weight(Framing::Chunked), max);
        // A lying Content-Length reserves at most the cap's worth.
        assert_eq!(body_weight(Framing::Length(u64::MAX)), max);
        // Four maximum-size bodies fit the budget; a fifth waits.
        assert_eq!(BODY_MEMORY_BUDGET / max, 4);
    }

    #[tokio::test]
    async fn a_silent_tunnel_ends_at_its_idle_limit() {
        let (mut client_end, _client_far) = duplex(64);
        let (mut up_end, _up_far) = duplex(64);
        let err = tunnel_within(
            &mut client_end,
            &mut up_end,
            b"",
            Duration::from_millis(60),
            Duration::from_secs(30),
        )
        .await
        .expect_err("idle");
        assert_eq!(err.kind(), io::ErrorKind::TimedOut);
    }

    #[tokio::test]
    async fn a_busy_tunnel_still_ends_at_its_lifetime() {
        let (mut client_end, mut client_far) = duplex(64);
        let (mut up_end, _up_far) = duplex(64);
        let pump = tokio::spawn(async move {
            tunnel_within(
                &mut client_end,
                &mut up_end,
                b"",
                Duration::from_secs(5),
                Duration::from_millis(150),
            )
            .await
        });
        // A byte every 20 ms never lets the idle limit fire.
        let feeder = tokio::spawn(async move {
            for _ in 0..40 {
                if client_far.write_all(b"x").await.is_err() {
                    break;
                }
                tokio::time::sleep(Duration::from_millis(20)).await;
            }
        });
        let err = pump.await.expect("join").expect_err("lifetime");
        assert_eq!(err.kind(), io::ErrorKind::TimedOut);
        feeder.abort();
    }
}
