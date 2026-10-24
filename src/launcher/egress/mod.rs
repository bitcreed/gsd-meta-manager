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
//! - anything else is refused with a 403 and the upstream is never contacted.
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
use std::os::unix::net::UnixListener as StdUnixListener;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::UnixStream;
use tokio::sync::{oneshot, OwnedSemaphorePermit, Semaphore};
use tokio::task::JoinSet;

use self::http1::{Head, ReadError};
use self::policy::{BodyVerdict, Denial, Target};
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
                        runtime.block_on(run(listener, spec, stop_rx, thread_counters, ready_tx));
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

/// Everything a connection handler reads.
struct Ctx {
    token: TokenProvider,
    connector: Connector,
    counters: Arc<Counters>,
}

impl Ctx {
    /// The token to inject now.
    async fn current_token(&self) -> String {
        match &self.token {
            TokenProvider::Static(token) => token.clone(),
            TokenProvider::Host { initial, .. } => initial.clone(),
        }
    }
}

async fn run(
    listener: StdUnixListener,
    spec: EgressSpec,
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
    let ctx = Arc::new(Ctx {
        token: spec.token,
        connector,
        counters,
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

/// Write `bytes` to the client within [`REPLY_TIMEOUT`]; a failure is the
/// client's loss and is ignored.
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
            if let Some((status, reason)) = read_error_reply(&e) {
                send(&mut client, &http1::error_response(status, reason)).await;
            }
            return;
        }
    };
    match policy::classify_target(&head.method, &head.target) {
        Target::Inference { path, query } => {
            let target = match &query {
                Some(query) => format!("{path}?{query}"),
                None => path,
            };
            inference(&mut client, &ctx, &head, leftover, &target).await;
        }
        Target::Connect { .. } => {
            // The CONNECT proxy is Task 3 of this plan; until then it is refused.
            deny(&mut client, &ctx, 403, "CONNECT is not available").await;
        }
        Target::Refuse(reason) => deny(&mut client, &ctx, 403, &reason).await,
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

async fn deny(client: &mut UnixStream, ctx: &Ctx, status: u16, reason: &str) {
    ctx.counters.inference_denied.fetch_add(1, Ordering::SeqCst);
    send(client, &http1::error_response(status, reason)).await;
}

async fn refuse(client: &mut UnixStream, ctx: &Ctx, denial: &Denial) {
    deny(client, ctx, denial.status(), denial.reason()).await;
}

async fn inference(
    client: &mut UnixStream,
    ctx: &Ctx,
    head: &Head,
    leftover: Vec<u8>,
    target: &str,
) {
    let framing = match policy::framing_verdict(&head.headers) {
        Ok(framing) => framing,
        Err(denial) => return refuse(client, ctx, &denial).await,
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
    let body = match http1::read_body(client, leftover, framing).await {
        Ok(body) => body,
        Err(e) => {
            if let Some((status, reason)) = read_error_reply(&e) {
                deny(client, ctx, status, reason).await;
            }
            return;
        }
    };
    if let BodyVerdict::Deny(reason) = policy::judge_body(&body) {
        return deny(client, ctx, 403, &reason).await;
    }
    let token = ctx.current_token().await;
    let headers = policy::forwarded_headers(&head.headers);
    let Some(request) = http1::request_bytes("POST", target, &headers, &token, &body) else {
        return deny(client, ctx, 502, "no usable credential for the upstream").await;
    };
    let mut up = match ctx.connector.connect().await {
        Ok(up) => up,
        Err(_) => return deny(client, ctx, 502, "the upstream is unreachable").await,
    };
    if up.write_all(&request).await.is_err() || up.flush().await.is_err() {
        return deny(client, ctx, 502, "the upstream refused the request").await;
    }
    relay_response(&mut up, client, ctx).await;
}

/// Pump the upstream's response to the client until the upstream ends it.
///
/// The bytes are not interpreted beyond the status line, which is read only to
/// count the request as forwarded: the upstream was asked for `Connection: close`,
/// so its end of stream is the end of the response whatever framing it used, and
/// the client parses that framing itself.
async fn relay_response(up: &mut upstream::UpstreamStream, client: &mut UnixStream, ctx: &Ctx) {
    let mut chunk = [0u8; 8192];
    let mut first: Vec<u8> = Vec::new();
    // The first read(s) up to the end of the status line, so a non-HTTP answer is
    // not passed on as one.
    while !first.contains(&b'\n') && first.len() < chunk.len() {
        match up.read(&mut chunk).await {
            Ok(0) => break,
            Ok(n) => first.extend_from_slice(&chunk[..n]),
            Err(e) if e.kind() == io::ErrorKind::UnexpectedEof => break,
            Err(_) => break,
        }
    }
    if !first.starts_with(b"HTTP/1.") {
        return deny(client, ctx, 502, "the upstream gave no valid response").await;
    }
    ctx.counters
        .inference_forwarded
        .fetch_add(1, Ordering::SeqCst);
    if client.write_all(&first).await.is_err() {
        return;
    }
    loop {
        match up.read(&mut chunk).await {
            Ok(0) => break,
            Ok(n) => {
                if client.write_all(&chunk[..n]).await.is_err() {
                    return;
                }
            }
            // A TLS peer that closes without close_notify: the end of the stream.
            Err(_) => break,
        }
    }
    let _ = client.shutdown().await;
}

/// RED stub: the tunnel pump lands in the GREEN commit.
pub async fn tunnel<C, U>(
    _client: &mut C,
    _upstream: &mut U,
    _leftover: &[u8],
) -> io::Result<(u64, u64)>
where
    C: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
    U: tokio::io::AsyncRead + tokio::io::AsyncWrite + Unpin,
{
    Err(io::Error::other("RED stub"))
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
}
