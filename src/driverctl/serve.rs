//! `driverctl serve`: the standalone socket server over the library [`RunStore`]
//! (Phase 27, plan 27-06; DCTL-01, D-03, D-20, D-21, D-26).
//!
//! This module is Linux-only (the store's frontmatter splice is). The server is
//! synchronous: std threads for connections and the ticker, plus ONE thread that
//! hosts a current-thread tokio runtime for nothing but the signal wait. No
//! `async fn` here calls a blocking function.
//!
//! # Numbered facts
//!
//! 1. **It is a standalone entry point over the same library store the Phase 28
//!    launcher embeds (D-03).** `serve` registers one launch
//!    (`begin_launch`, launcher connection mode `standalone` plus a headless driver
//!    connection `[inferred — audit]`) and answers the spike protocol on a 0600 unix
//!    socket. It adds no second writer: every request is applied by
//!    [`RunStore::handle_json`] under one mutex.
//! 2. **Ordering invariant (D-20): the run lock comes first.** `serve` takes
//!    `<run-dir>/launcher.lock` ([`acquire_at_with_retry`]) before it creates,
//!    unlinks or binds any socket path and before it writes `running`. A refused
//!    second `serve` therefore returns [`DriverctlError::Locked`] (exit code 3)
//!    having touched nothing it did not create. On shutdown the terminal status is
//!    written and the socket is unlinked while the lock is still held; the lock is
//!    released LAST, when the store drops.
//! 3. **No socket theft (D-21).** Only the lock holder unlinks a stale socket, and
//!    only a path that `symlink_metadata` says is a socket; any other file type at
//!    the path is refused and left alone.
//! 4. **The default socket path is private (D-21, T-27-22).** Without `--sock` it is
//!    `$XDG_RUNTIME_DIR/gsdmm-driverctl/<run-id>-<sha256(canonical run dir)[:8]>.sock`.
//!    That directory is created 0700 and must be a real directory owned by the
//!    current uid with no group/other bits. An unset `XDG_RUNTIME_DIR`, or a path
//!    over [`MAX_SUN_PATH`] bytes, is a typed refusal. There is deliberately no
//!    `/tmp` fallback `[inferred — audit]`: a shared directory is exactly where a
//!    squatter pre-creates the path.
//! 5. **The run id is a path component and is validated as one (Pitfall 8,
//!    T-27-23).** It must match [`RUN_ID_RE`] and the canonical run dir must sit
//!    directly under the canonical `<project>/.planning/meta-manager/drivers/`.
//!    Anything else is refused before the lock is touched.
//! 6. **A stalled connection never blocks a gate hook (T-27-24).** Each connection
//!    gets its own thread (at most [`MAX_CONNECTIONS`]; beyond that the reply is
//!    `{ok:false,error:busy}`, which a gate client turns into a deny), with
//!    a [`CONN_TIMEOUT`] TOTAL read deadline (WR-01), a write timeout and a 1 MiB
//!    request cap. Reading
//!    happens before the store mutex is taken, so a trickling client holds no lock.
//!    A panic while handling one request is answered `internal: panic` and never
//!    ends the server; a poisoned mutex is recovered.
//! 7. **A 2 s ticker drives the launcher-side bookkeeping (D-26, D-11).** Every
//!    [`TICK`] it scans `*-VERIFICATION.md` on disk WITHOUT the store mutex (CR-01),
//!    then feeds the result to `observe_verifications` and calls `flush_heartbeat`
//!    (the trailing heartbeat flush) under the store mutex.
//! 8. **SIGTERM and SIGINT stop it gracefully.** The terminal status `stopped`
//!    (reason `serve:signal`, unless the driver logged `RUN end`) is written and the
//!    frontmatter regenerated, then the listener stops, the socket is unlinked and
//!    the lock is released.

use std::fs;
use std::io::{Read, Write};
use std::os::unix::ffi::OsStrExt;
use std::os::unix::fs::{DirBuilderExt, FileTypeExt, MetadataExt, PermissionsExt};
use std::os::unix::net::{UnixListener, UnixStream};
use std::panic::AssertUnwindSafe;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, LazyLock, Mutex, MutexGuard, PoisonError};
use std::thread::JoinHandle;
use std::time::{Duration, Instant};

use regex::Regex;
use sha2::{Digest, Sha256};

use super::protocol::{Response, MAX_REQUEST_BYTES};
use super::scan::scan_verifications;
use super::store::{DriverSpec, LaunchSpec, RunStore, StoreError, TerminalOutcome, TerminalStatus};
use crate::driver::direction::{direction_lock_path, sweep_stale_temps};
use crate::driver::lock::{acquire_at_with_retry, run_lock_path, ACQUIRE_RETRY_BUDGET};
use crate::error::LockError;

/// The per-user directory (under `XDG_RUNTIME_DIR`) the default sockets live in.
pub const SOCKET_DIR_NAME: &str = "gsdmm-driverctl";

/// The longest socket path `sockaddr_un` holds on Linux (108 bytes with the NUL).
pub const MAX_SUN_PATH: usize = 107;

/// How many connections may be in flight; the next one is answered `busy`.
pub const MAX_CONNECTIONS: usize = 32;

/// Read and write timeout of one connection.
pub const CONN_TIMEOUT: Duration = Duration::from_secs(10);

/// How often the ticker scans for gap rounds and flushes a trailing heartbeat.
pub const TICK: Duration = Duration::from_secs(2);

/// A `.DIRECTION.md.fm.*` temp older than this is swept at startup.
pub const STALE_TEMP_AGE: Duration = Duration::from_secs(60);

/// What a run id must look like (Pitfall 8): one path component, no leading
/// dash or dot, at most 80 characters.
pub const RUN_ID_RE: &str = r"^[A-Za-z0-9][A-Za-z0-9._-]{0,79}$";

/// Where run directories live, relative to the project root.
const DRIVERS_REL: &str = ".planning/meta-manager/drivers";

/// How long the accept loop and the ticker sleep between checks of the stop flag.
const POLL: Duration = Duration::from_millis(50);

static RUN_ID: LazyLock<Regex> = LazyLock::new(|| Regex::new(RUN_ID_RE).expect("static regex"));

/// Whether `id` is acceptable as a run id.
pub fn valid_run_id(id: &str) -> bool {
    RUN_ID.is_match(id)
}

/// Why `driverctl serve` did not run (or could not continue).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DriverctlError {
    /// The run directory or the project is not a valid pair (Pitfall 8).
    InvalidRunDir { detail: String },
    /// The socket path could not be resolved, created or taken.
    SocketPath { detail: String },
    /// Another live serve or launcher holds the run lock.
    Locked { detail: String },
    /// A file-system or thread fault.
    Io { detail: String },
    /// The run store could not open or write.
    Store { detail: String },
}

impl DriverctlError {
    /// The process exit code: `3` for a held lock (the spike launcher's refusal
    /// code) `[inferred — audit]`, `1` for everything else.
    pub fn exit_code(&self) -> i32 {
        match self {
            DriverctlError::Locked { .. } => 3,
            _ => 1,
        }
    }
}

impl std::fmt::Display for DriverctlError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DriverctlError::InvalidRunDir { detail } => write!(f, "invalid run dir: {detail}"),
            DriverctlError::SocketPath { detail } => write!(f, "socket path: {detail}"),
            DriverctlError::Locked { detail } => write!(f, "run is already served: {detail}"),
            DriverctlError::Io { detail } => write!(f, "I/O failure: {detail}"),
            DriverctlError::Store { detail } => write!(f, "run store: {detail}"),
        }
    }
}

impl std::error::Error for DriverctlError {}

impl From<StoreError> for DriverctlError {
    fn from(err: StoreError) -> Self {
        DriverctlError::Store {
            detail: err.to_string(),
        }
    }
}

fn io_error(context: &str, err: impl std::fmt::Display) -> DriverctlError {
    DriverctlError::Io {
        detail: format!("{context}: {err}"),
    }
}

/// What `driverctl serve` was asked to do.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ServeOptions {
    /// The run directory, `<project>/.planning/meta-manager/drivers/<run-id>`.
    pub run_dir: PathBuf,
    /// The project root the run belongs to.
    pub project: PathBuf,
    /// An explicit socket path; `None` uses [`socket_path`].
    pub sock: Option<PathBuf>,
}

// ---------------------------------------------------------------- run dir

/// Validate the run directory against the project (Pitfall 8, T-27-23) and return
/// its canonical path and run id.
///
/// The basename must match [`RUN_ID_RE`] and the canonical directory must sit
/// directly under the canonical `<project>/.planning/meta-manager/drivers/`. A
/// symlinked run dir pointing elsewhere is therefore refused.
pub fn validate_run_dir(
    project: &Path,
    run_dir: &Path,
) -> Result<(PathBuf, String), DriverctlError> {
    let bad = |detail: String| DriverctlError::InvalidRunDir { detail };

    let raw_id = run_dir
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| {
            bad(format!(
                "{} has no usable final component",
                run_dir.display()
            ))
        })?;
    if !valid_run_id(raw_id) {
        return Err(bad(format!("run id {raw_id:?} does not match {RUN_ID_RE}")));
    }

    let canonical_run = fs::canonicalize(run_dir)
        .map_err(|e| bad(format!("run dir {}: {e}", run_dir.display())))?;
    if !canonical_run.is_dir() {
        return Err(bad(format!(
            "{} is not a directory",
            canonical_run.display()
        )));
    }
    let run_id = canonical_run
        .file_name()
        .and_then(|n| n.to_str())
        .ok_or_else(|| bad(format!("{} has no usable name", canonical_run.display())))?
        .to_owned();
    if !valid_run_id(&run_id) {
        return Err(bad(format!("run id {run_id:?} does not match {RUN_ID_RE}")));
    }

    let canonical_project = fs::canonicalize(project)
        .map_err(|e| bad(format!("project {}: {e}", project.display())))?;
    let drivers = fs::canonicalize(canonical_project.join(DRIVERS_REL)).map_err(|e| {
        bad(format!(
            "{}/{DRIVERS_REL}: {e}",
            canonical_project.display()
        ))
    })?;
    if canonical_run.parent() != Some(drivers.as_path()) {
        return Err(bad(format!(
            "{} is not directly under {}",
            canonical_run.display(),
            drivers.display()
        )));
    }
    Ok((canonical_run, run_id))
}

// ---------------------------------------------------------------- socket path

/// The default socket path for a run (D-21), from `XDG_RUNTIME_DIR`.
///
/// `canonical_run_dir` must already be canonical: its hash keeps two projects
/// that use the same run id from colliding on one socket (T-27-25). Phase 28
/// passes this as `DRIVERCTL_SOCK`.
pub fn socket_path(run_id: &str, canonical_run_dir: &Path) -> Result<PathBuf, DriverctlError> {
    let runtime = std::env::var_os("XDG_RUNTIME_DIR")
        .filter(|v| !v.is_empty())
        .map(PathBuf::from);
    socket_path_in(runtime.as_deref(), run_id, canonical_run_dir)
}

/// [`socket_path`] with the runtime directory passed in (the testable core).
pub fn socket_path_in(
    runtime_dir: Option<&Path>,
    run_id: &str,
    canonical_run_dir: &Path,
) -> Result<PathBuf, DriverctlError> {
    let bad = |detail: String| DriverctlError::SocketPath { detail };

    let runtime = runtime_dir.ok_or_else(|| {
        bad("XDG_RUNTIME_DIR is not set; pass --sock explicitly (/tmp is never used)".to_owned())
    })?;
    if !runtime.is_absolute() {
        return Err(bad(format!(
            "XDG_RUNTIME_DIR {} is not an absolute path",
            runtime.display()
        )));
    }

    let digest = Sha256::digest(canonical_run_dir.as_os_str().as_bytes());
    let hash8: String = digest.iter().take(4).map(|b| format!("{b:02x}")).collect();
    let dir = runtime.join(SOCKET_DIR_NAME);
    let path = dir.join(format!("{run_id}-{hash8}.sock"));
    check_sun_len(&path)?;

    match fs::DirBuilder::new().mode(0o700).create(&dir) {
        Ok(()) => {}
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
        Err(e) => return Err(bad(format!("create {}: {e}", dir.display()))),
    }
    let meta = fs::symlink_metadata(&dir).map_err(|e| bad(format!("{}: {e}", dir.display())))?;
    if !meta.file_type().is_dir() {
        return Err(bad(format!("{} is not a real directory", dir.display())));
    }
    if meta.uid() != rustix::process::getuid().as_raw() {
        return Err(bad(format!(
            "{} is not owned by the current user",
            dir.display()
        )));
    }
    if meta.mode() & 0o077 != 0 {
        return Err(bad(format!(
            "{} must not be accessible to group or others (mode {:o})",
            dir.display(),
            meta.mode() & 0o777
        )));
    }
    Ok(path)
}

fn check_sun_len(path: &Path) -> Result<(), DriverctlError> {
    let len = path.as_os_str().len();
    if len > MAX_SUN_PATH {
        return Err(DriverctlError::SocketPath {
            detail: format!(
                "{} is {len} bytes, over the {MAX_SUN_PATH}-byte unix socket limit",
                path.display()
            ),
        });
    }
    Ok(())
}

/// Refuse a path that holds something other than a socket; report whether a
/// stale socket is there to replace.
fn existing_socket(path: &Path) -> Result<bool, DriverctlError> {
    match fs::symlink_metadata(path) {
        Ok(meta) if meta.file_type().is_socket() => Ok(true),
        Ok(_) => Err(DriverctlError::SocketPath {
            detail: format!(
                "{} exists and is not a socket; refusing to replace it",
                path.display()
            ),
        }),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(io_error(&format!("stat {}", path.display()), e)),
    }
}

/// Bind `path` with a private mode: the umask is 0o177 around the bind, then the
/// mode is set explicitly (T-27-26).
fn replace_and_bind(sock: &Path, stale: bool) -> Result<UnixListener, DriverctlError> {
    if stale {
        fs::remove_file(sock)
            .map_err(|e| io_error(&format!("remove stale socket {}", sock.display()), e))?;
    }
    bind_private(sock)
}

fn bind_private(path: &Path) -> Result<UnixListener, DriverctlError> {
    let old = rustix::process::umask(rustix::fs::Mode::from_raw_mode(0o177));
    let bound = UnixListener::bind(path);
    rustix::process::umask(old);
    let listener = bound.map_err(|e| io_error(&format!("bind {}", path.display()), e))?;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
        .map_err(|e| io_error(&format!("chmod {}", path.display()), e))?;
    listener
        .set_nonblocking(true)
        .map_err(|e| io_error("set the listener non-blocking", e))?;
    Ok(listener)
}

// ---------------------------------------------------------------- signals

/// Wait for SIGTERM or SIGINT, then set `stop`. An `async fn` with no blocking
/// call in it (`tests/async_blocking_guard.rs`).
async fn wait_for_stop_signal(stop: Arc<AtomicBool>, ready: mpsc::Sender<Result<(), String>>) {
    use tokio::signal::unix::{signal, SignalKind};

    let handlers = signal(SignalKind::terminate())
        .and_then(|term| signal(SignalKind::interrupt()).map(|int| (term, int)));
    let (mut term, mut int) = match handlers {
        Ok(pair) => pair,
        Err(e) => {
            let _ = ready.send(Err(e.to_string()));
            return;
        }
    };
    let _ = ready.send(Ok(()));
    tokio::select! {
        _ = term.recv() => {}
        _ = int.recv() => {}
    }
    stop.store(true, Ordering::SeqCst);
}

/// Install the SIGTERM/SIGINT handlers and return once they are registered, so a
/// signal sent after this returns is never lost.
fn install_signal_handlers(stop: &Arc<AtomicBool>) -> Result<(), DriverctlError> {
    let (tx, rx) = mpsc::channel();
    let stop = Arc::clone(stop);
    std::thread::Builder::new()
        .name("driverctl-signals".to_owned())
        .spawn(move || {
            let runtime = tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build();
            match runtime {
                Ok(runtime) => runtime.block_on(wait_for_stop_signal(stop, tx)),
                Err(e) => {
                    let _ = tx.send(Err(e.to_string()));
                }
            }
        })
        .map_err(|e| io_error("spawn the signal thread", e))?;
    match rx.recv_timeout(Duration::from_secs(5)) {
        Ok(Ok(())) => Ok(()),
        Ok(Err(e)) => Err(io_error("install signal handlers", e)),
        Err(e) => Err(io_error("install signal handlers", e)),
    }
}

// ---------------------------------------------------------------- connections

/// Take the store mutex, recovering a poisoned one: a panic while handling one
/// request must not end the server.
fn lock_store(store: &Mutex<RunStore>) -> MutexGuard<'_, RunStore> {
    store.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Run `f`, turning a panic into the `internal: panic` reply.
fn answer(f: impl FnOnce() -> Response) -> Response {
    std::panic::catch_unwind(AssertUnwindSafe(f))
        .unwrap_or_else(|_| Response::rejected("internal: panic"))
}

enum ReadError {
    TooLarge,
    /// Closed before any byte, or the read failed.
    Closed,
    TimedOut,
}

/// Read one request: up to the first `\n` or EOF, at most [`MAX_REQUEST_BYTES`],
/// within `total` of wall time.
///
/// WR-01: the bound is a TOTAL deadline, not a per-`read` socket timeout. Each read
/// is given only the time that is left, so a client that trickles one byte every few
/// seconds is timed out at the deadline instead of holding its handler slot for
/// `MAX_REQUEST_BYTES` reads.
fn read_request(stream: &mut UnixStream, total: Duration) -> Result<Vec<u8>, ReadError> {
    let deadline = Instant::now() + total;
    let mut buf: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
        let left = deadline.saturating_duration_since(Instant::now());
        if left.is_zero() {
            return Err(ReadError::TimedOut);
        }
        if stream.set_read_timeout(Some(left)).is_err() {
            return Err(ReadError::Closed);
        }
        match stream.read(&mut chunk) {
            Ok(0) => {
                return if buf.is_empty() {
                    Err(ReadError::Closed)
                } else {
                    Ok(buf)
                };
            }
            Ok(n) => {
                let data = &chunk[..n];
                if let Some(end) = data.iter().position(|b| *b == b'\n') {
                    buf.extend_from_slice(&data[..end]);
                    return if buf.len() > MAX_REQUEST_BYTES {
                        Err(ReadError::TooLarge)
                    } else {
                        Ok(buf)
                    };
                }
                buf.extend_from_slice(data);
                if buf.len() > MAX_REQUEST_BYTES {
                    return Err(ReadError::TooLarge);
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => {}
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                return Err(ReadError::TimedOut);
            }
            Err(_) => return Err(ReadError::Closed),
        }
    }
}

fn write_reply(stream: &mut UnixStream, reply: &Response) {
    let mut line = serde_json::to_string(reply)
        .unwrap_or_else(|_| r#"{"ok":false,"error":"internal: serialize"}"#.to_owned());
    line.push('\n');
    let _ = stream.write_all(line.as_bytes());
}

fn handle_connection(mut stream: UnixStream, store: &Mutex<RunStore>) {
    let _ = stream.set_nonblocking(false);
    let _ = stream.set_write_timeout(Some(CONN_TIMEOUT));
    let reply = match read_request(&mut stream, CONN_TIMEOUT) {
        Ok(bytes) => answer(|| lock_store(store).handle_json(&bytes)),
        Err(ReadError::TooLarge) => Response::rejected("request too large"),
        Err(ReadError::TimedOut) => Response::rejected("timed out reading the request"),
        Err(ReadError::Closed) => return,
    };
    write_reply(&mut stream, &reply);
}

/// Decrements the in-flight count when a handler thread ends, however it ends.
struct InFlight(Arc<AtomicUsize>);

impl Drop for InFlight {
    fn drop(&mut self) {
        self.0.fetch_sub(1, Ordering::SeqCst);
    }
}

/// Answer `busy` without reading the request and close.
fn refuse_busy(mut stream: UnixStream) {
    let _ = stream.set_nonblocking(false);
    let _ = stream.set_write_timeout(Some(Duration::from_secs(1)));
    write_reply(&mut stream, &Response::rejected("busy"));
}

fn accept_loop(
    listener: &UnixListener,
    store: &Arc<Mutex<RunStore>>,
    stop: &AtomicBool,
    in_flight: &Arc<AtomicUsize>,
) {
    while !stop.load(Ordering::SeqCst) {
        match listener.accept() {
            Ok((stream, _)) => {
                if in_flight.fetch_add(1, Ordering::SeqCst) >= MAX_CONNECTIONS {
                    in_flight.fetch_sub(1, Ordering::SeqCst);
                    refuse_busy(stream);
                    continue;
                }
                let guard = InFlight(Arc::clone(in_flight));
                let store = Arc::clone(store);
                let spawned = std::thread::Builder::new()
                    .name("driverctl-conn".to_owned())
                    .spawn(move || {
                        let _guard = guard;
                        handle_connection(stream, &store);
                        drop(store);
                    });
                if let Err(e) = spawned {
                    eprintln!("driverctl serve: could not spawn a handler thread: {e}");
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => std::thread::sleep(POLL),
            Err(e) => {
                eprintln!("driverctl serve: accept failed: {e}");
                std::thread::sleep(POLL);
            }
        }
    }
}

// ---------------------------------------------------------------- ticker

fn ticker(store: Arc<Mutex<RunStore>>, stop: Arc<AtomicBool>, project: PathBuf) {
    'outer: loop {
        let started = Instant::now();
        while started.elapsed() < TICK {
            if stop.load(Ordering::SeqCst) {
                break 'outer;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        // CR-01: the file scan runs BEFORE the store mutex is taken, so a slow
        // read of a driver-plantable file can never stall `handle_json`.
        let writes = scan_verifications(&project);
        let mut guard = lock_store(&store);
        if let Err(e) = guard.observe_verifications(&writes) {
            eprintln!("driverctl serve: gap scan failed: {e}");
        }
        guard.flush_heartbeat();
    }
}

// ---------------------------------------------------------------- serve

/// Serve one run until SIGTERM or SIGINT. See the module header for the order.
pub fn serve(opts: ServeOptions) -> Result<(), DriverctlError> {
    let (run_dir, run_id) = validate_run_dir(&opts.project, &opts.run_dir)?;
    let project = fs::canonicalize(&opts.project)
        .map_err(|e| io_error(&format!("project {}", opts.project.display()), e))?;

    // D-20: the run lock first. Nothing below touches a socket path before this.
    let lock = acquire_at_with_retry(
        &run_lock_path(&run_dir),
        &run_id,
        std::process::id(),
        ACQUIRE_RETRY_BUDGET,
    )
    .map_err(|e| match e {
        LockError::Unavailable { detail } => DriverctlError::Io {
            detail: format!("run lock: {detail}"),
        },
        held => DriverctlError::Locked {
            detail: held.to_string(),
        },
    })?;

    // Both lock files exist from here on (Phase 28 masks them inside bwrap).
    fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .open(direction_lock_path(&run_dir))
        .map_err(|e| io_error("create DIRECTION.lock", e))?;
    if let Err(e) = sweep_stale_temps(&run_dir, STALE_TEMP_AGE) {
        eprintln!("driverctl serve: could not sweep stale temps: {e}");
    }

    let stop = Arc::new(AtomicBool::new(false));
    install_signal_handlers(&stop)?;

    // Resolve and vet the socket path before `running` is written, so a refusal
    // leaves no half-registered launch behind.
    let sock = match &opts.sock {
        Some(path) => {
            check_sun_len(path)?;
            path.clone()
        }
        None => socket_path(&run_id, &run_dir)?,
    };
    let stale = existing_socket(&sock)?;

    let mut store = RunStore::open(&run_dir, &project, lock)?;
    store.begin_launch(LaunchSpec {
        launcher_mode: "standalone".to_owned(),
        driver: Some(DriverSpec {
            model: None,
            mode: "headless".to_owned(),
        }),
        ..LaunchSpec::default()
    })?;

    // We hold the lock: only now may a stale socket be replaced (D-21).
    let listener = replace_and_bind(&sock, stale);
    let listener = match listener {
        Ok(listener) => listener,
        Err(e) => {
            let _ = store.set_terminal(TerminalStatus::Stopped, "serve:bind-failed");
            return Err(e);
        }
    };

    let scan_root = store.project().to_owned();
    let store = Arc::new(Mutex::new(store));
    let ticker_handle: JoinHandle<()> = {
        let store = Arc::clone(&store);
        let stop = Arc::clone(&stop);
        std::thread::Builder::new()
            .name("driverctl-ticker".to_owned())
            .spawn(move || ticker(store, stop, scan_root))
            .map_err(|e| io_error("spawn the ticker", e))?
    };

    eprintln!("driverctl serve: listening on {}", sock.display());
    let in_flight = Arc::new(AtomicUsize::new(0));
    accept_loop(&listener, &store, &stop, &in_flight);

    // Shutdown (D-20): stop accepting, let handlers finish, write the terminal
    // status, unlink the socket, and release the lock LAST.
    drop(listener);
    let _ = ticker_handle.join();
    let deadline = Instant::now() + CONN_TIMEOUT + Duration::from_secs(1);
    while in_flight.load(Ordering::SeqCst) > 0 && Instant::now() < deadline {
        std::thread::sleep(POLL);
    }
    let terminal = lock_store(&store).set_terminal(TerminalStatus::Stopped, "serve:signal");
    match terminal {
        Ok(TerminalOutcome::Applied | TerminalOutcome::KeptDriverEnd) => {}
        Err(e) => eprintln!("driverctl serve: could not record the stop: {e}"),
    }
    let _ = fs::remove_file(&sock);
    match Arc::try_unwrap(store) {
        Ok(mutex) => drop(mutex.into_inner().unwrap_or_else(PoisonError::into_inner)),
        // A handler is still running past its timeouts: the process exits right
        // after this returns, and the kernel releases the lock then.
        Err(_still_shared) => {}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    /// A project with `drivers/<id>` for each id, returned with the project root.
    fn project_with_runs(ids: &[&str]) -> (TempDir, PathBuf) {
        let td = TempDir::new().unwrap();
        let drivers = td.path().join(DRIVERS_REL);
        fs::create_dir_all(&drivers).unwrap();
        for id in ids {
            fs::create_dir_all(drivers.join(id)).unwrap();
        }
        let project = td.path().to_path_buf();
        (td, project)
    }

    fn runtime_dir() -> TempDir {
        TempDir::new().unwrap()
    }

    // ---------------------------------------------------------------- run ids

    #[test]
    fn run_id_pattern_accepts_ordinary_ids() {
        for id in ["r27", "s003-r261a", "a", "run_1.b", &"a".repeat(80)] {
            assert!(valid_run_id(id), "{id:?} should be valid");
        }
    }

    #[test]
    fn run_id_pattern_refuses_path_shaped_and_oversized_ids() {
        for id in [
            "..",
            ".",
            "a b",
            "-x",
            ".hidden",
            "a/b",
            "",
            "a\nb",
            &"a".repeat(81),
        ] {
            assert!(!valid_run_id(id), "{id:?} should be refused");
        }
    }

    #[test]
    fn validate_run_dir_accepts_a_run_directly_under_drivers() {
        let (_td, project) = project_with_runs(&["r27"]);
        let run_dir = project.join(DRIVERS_REL).join("r27");
        let (canonical, id) = validate_run_dir(&project, &run_dir).unwrap();
        assert_eq!(id, "r27");
        assert_eq!(canonical, fs::canonicalize(&run_dir).unwrap());
    }

    #[test]
    fn validate_run_dir_refuses_bad_ids_before_looking_at_the_filesystem() {
        let (_td, project) = project_with_runs(&["a b", "-x"]);
        let drivers = project.join(DRIVERS_REL);
        let long = "a".repeat(81);
        fs::create_dir_all(drivers.join(&long)).unwrap();
        for id in ["..", "a b", "-x", long.as_str()] {
            let err = validate_run_dir(&project, &drivers.join(id)).unwrap_err();
            assert!(
                matches!(err, DriverctlError::InvalidRunDir { .. }),
                "{id:?}: {err:?}"
            );
            assert_eq!(err.exit_code(), 1);
        }
    }

    #[test]
    fn validate_run_dir_refuses_a_directory_outside_drivers() {
        let (td, project) = project_with_runs(&["r27"]);
        let outside = td.path().join("elsewhere/r27");
        fs::create_dir_all(&outside).unwrap();
        let err = validate_run_dir(&project, &outside).unwrap_err();
        assert!(err.to_string().contains("not directly under"), "{err}");

        // Two levels down is not "directly under" either.
        let nested = project.join(DRIVERS_REL).join("r27").join("inner");
        fs::create_dir_all(&nested).unwrap();
        assert!(validate_run_dir(&project, &nested).is_err());
    }

    #[test]
    fn validate_run_dir_refuses_a_symlink_that_leaves_drivers() {
        let (td, project) = project_with_runs(&[]);
        let outside = td.path().join("elsewhere/r99");
        fs::create_dir_all(&outside).unwrap();
        let link = project.join(DRIVERS_REL).join("linked");
        std::os::unix::fs::symlink(&outside, &link).unwrap();
        assert!(validate_run_dir(&project, &link).is_err());
    }

    #[test]
    fn validate_run_dir_refuses_a_missing_run_dir_and_a_missing_project() {
        let (_td, project) = project_with_runs(&[]);
        let missing = project.join(DRIVERS_REL).join("nope");
        assert!(validate_run_dir(&project, &missing).is_err());
        let (_td2, project2) = project_with_runs(&["r27"]);
        let run_dir = project2.join(DRIVERS_REL).join("r27");
        assert!(validate_run_dir(&project2.join("absent"), &run_dir).is_err());
    }

    // ---------------------------------------------------------------- socket path

    #[test]
    fn socket_path_names_the_run_and_a_stable_hash_of_the_run_dir() {
        let rt = runtime_dir();
        let run_dir = Path::new("/projects/a/.planning/meta-manager/drivers/r27");
        let first = socket_path_in(Some(rt.path()), "r27", run_dir).unwrap();
        let again = socket_path_in(Some(rt.path()), "r27", run_dir).unwrap();
        assert_eq!(first, again, "the hash is stable");

        let name = first.file_name().unwrap().to_str().unwrap();
        let (id, rest) = name.split_once('-').unwrap();
        assert_eq!(id, "r27");
        let hash = rest.strip_suffix(".sock").expect("a .sock suffix");
        assert_eq!(hash.len(), 8);
        assert!(hash.bytes().all(|b| b.is_ascii_hexdigit()), "{hash}");
        assert_eq!(first.parent().unwrap(), rt.path().join(SOCKET_DIR_NAME));

        // The same run id in another project gets another socket (T-27-25).
        let other = socket_path_in(
            Some(rt.path()),
            "r27",
            Path::new("/projects/b/.planning/meta-manager/drivers/r27"),
        )
        .unwrap();
        assert_ne!(first, other);
    }

    #[test]
    fn socket_path_creates_the_directory_0700() {
        let rt = runtime_dir();
        socket_path_in(Some(rt.path()), "r27", Path::new("/p/r27")).unwrap();
        let meta = fs::metadata(rt.path().join(SOCKET_DIR_NAME)).unwrap();
        assert_eq!(meta.mode() & 0o777, 0o700);
    }

    #[test]
    fn socket_path_without_a_runtime_dir_is_a_typed_refusal_naming_the_variable() {
        let err = socket_path_in(None, "r27", Path::new("/p/r27")).unwrap_err();
        assert!(matches!(err, DriverctlError::SocketPath { .. }));
        let text = err.to_string();
        assert!(text.contains("XDG_RUNTIME_DIR"), "{text}");
        assert!(text.contains("/tmp"), "{text}");
    }

    #[test]
    fn socket_path_refuses_a_relative_runtime_dir() {
        let err =
            socket_path_in(Some(Path::new("run/user")), "r27", Path::new("/p/r27")).unwrap_err();
        assert!(err.to_string().contains("absolute"), "{err}");
    }

    #[test]
    fn socket_path_refuses_a_group_or_other_accessible_directory() {
        for mode in [0o770, 0o750, 0o705, 0o707] {
            let rt = runtime_dir();
            let dir = rt.path().join(SOCKET_DIR_NAME);
            fs::create_dir(&dir).unwrap();
            fs::set_permissions(&dir, fs::Permissions::from_mode(mode)).unwrap();
            let err = socket_path_in(Some(rt.path()), "r27", Path::new("/p/r27")).unwrap_err();
            assert!(
                err.to_string().contains("group or others"),
                "mode {mode:o}: {err}"
            );
        }
    }

    #[test]
    fn socket_path_accepts_a_pre_existing_private_directory() {
        let rt = runtime_dir();
        let dir = rt.path().join(SOCKET_DIR_NAME);
        fs::create_dir(&dir).unwrap();
        fs::set_permissions(&dir, fs::Permissions::from_mode(0o700)).unwrap();
        assert!(socket_path_in(Some(rt.path()), "r27", Path::new("/p/r27")).is_ok());
    }

    #[test]
    fn socket_path_refuses_a_symlink_in_place_of_the_directory() {
        let rt = runtime_dir();
        let real = rt.path().join("real");
        fs::create_dir(&real).unwrap();
        fs::set_permissions(&real, fs::Permissions::from_mode(0o700)).unwrap();
        std::os::unix::fs::symlink(&real, rt.path().join(SOCKET_DIR_NAME)).unwrap();
        let err = socket_path_in(Some(rt.path()), "r27", Path::new("/p/r27")).unwrap_err();
        assert!(err.to_string().contains("not a real directory"), "{err}");
    }

    #[test]
    fn socket_path_refuses_a_path_over_the_unix_limit() {
        let rt = runtime_dir();
        let long_id = "r".repeat(80);
        let nested = rt.path().join("x".repeat(60));
        fs::create_dir(&nested).unwrap();
        let err = socket_path_in(Some(&nested), &long_id, Path::new("/p/r")).unwrap_err();
        assert!(err.to_string().contains("107"), "{err}");
        // Refused before the directory was created.
        assert!(!nested.join(SOCKET_DIR_NAME).exists());
    }

    #[test]
    fn a_path_of_exactly_107_bytes_passes_the_length_check() {
        let exact = PathBuf::from(format!("/{}", "a".repeat(MAX_SUN_PATH - 1)));
        assert_eq!(exact.as_os_str().len(), MAX_SUN_PATH);
        assert!(check_sun_len(&exact).is_ok());
        let over = PathBuf::from(format!("/{}", "a".repeat(MAX_SUN_PATH)));
        assert!(check_sun_len(&over).is_err());
    }

    // ---------------------------------------------------------------- misc

    #[test]
    fn only_a_held_lock_exits_3() {
        let locked = DriverctlError::Locked {
            detail: String::new(),
        };
        assert_eq!(locked.exit_code(), 3);
        for other in [
            DriverctlError::InvalidRunDir {
                detail: String::new(),
            },
            DriverctlError::SocketPath {
                detail: String::new(),
            },
            DriverctlError::Io {
                detail: String::new(),
            },
            DriverctlError::Store {
                detail: String::new(),
            },
        ] {
            assert_eq!(other.exit_code(), 1, "{other:?}");
        }
    }

    #[test]
    fn a_panic_while_handling_a_request_is_answered_internal() {
        let reply = answer(|| panic!("boom"));
        assert_eq!(reply, Response::rejected("internal: panic"));
        assert_eq!(answer(Response::ok), Response::ok());
    }

    #[test]
    fn wr01_a_trickling_client_hits_the_total_deadline() {
        let (mut server, mut client) = UnixStream::pair().unwrap();
        // One byte every 20 ms for up to ~3 s, never a newline. A per-read timeout
        // of 200 ms is reset by every byte and would never fire; the writer then
        // closes and the read would return Ok. Only a total deadline times out.
        let writer = std::thread::spawn(move || {
            for _ in 0..150 {
                if client.write_all(b"x").is_err() {
                    return;
                }
                std::thread::sleep(Duration::from_millis(20));
            }
        });
        let out = read_request(&mut server, Duration::from_millis(200));
        drop(server);
        let _ = writer.join();
        assert!(matches!(out, Err(ReadError::TimedOut)));
    }

    #[test]
    fn wr01_a_prompt_request_is_still_read() {
        let (mut server, mut client) = UnixStream::pair().unwrap();
        client.write_all(b"{\"op\":\"x\"}\nrest").unwrap();
        let got = read_request(&mut server, Duration::from_secs(30));
        assert!(matches!(got, Ok(ref b) if b == br#"{"op":"x"}"#));
    }

    #[test]
    fn existing_socket_only_replaces_a_real_socket() {
        let td = TempDir::new().unwrap();
        let missing = td.path().join("none");
        assert!(!existing_socket(&missing).unwrap());

        let file = td.path().join("file");
        fs::write(&file, "x").unwrap();
        assert!(existing_socket(&file).is_err());
        assert!(file.exists(), "a refused path is left alone");

        let sock = td.path().join("s.sock");
        drop(UnixListener::bind(&sock).unwrap());
        assert!(existing_socket(&sock).unwrap());

        let link = td.path().join("link");
        std::os::unix::fs::symlink(&sock, &link).unwrap();
        assert!(existing_socket(&link).is_err(), "a symlink is not a socket");
    }
}
