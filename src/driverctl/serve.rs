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
//!    [`CONN_TIMEOUT`] read and write timeouts and a 1 MiB request cap. Reading
//!    happens before the store mutex is taken, so a trickling client holds no lock.
//!    A panic while handling one request is answered `internal: panic` and never
//!    ends the server; a poisoned mutex is recovered.
//! 7. **A 2 s ticker drives the launcher-side bookkeeping (D-26, D-11).** Every
//!    [`TICK`] it calls `scan_tick` (gap rounds from `*-VERIFICATION.md` on disk)
//!    and `flush_heartbeat` (the trailing heartbeat flush) under the store mutex.
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

/// Read one request: up to the first `\n` or EOF, at most [`MAX_REQUEST_BYTES`].
fn read_request(stream: &mut UnixStream) -> Result<Vec<u8>, ReadError> {
    let mut buf: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 8192];
    loop {
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
    let _ = stream.set_read_timeout(Some(CONN_TIMEOUT));
    let _ = stream.set_write_timeout(Some(CONN_TIMEOUT));
    let reply = match read_request(&mut stream) {
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

fn ticker(store: Arc<Mutex<RunStore>>, stop: Arc<AtomicBool>) {
    'outer: loop {
        let started = Instant::now();
        while started.elapsed() < TICK {
            if stop.load(Ordering::SeqCst) {
                break 'outer;
            }
            std::thread::sleep(Duration::from_millis(100));
        }
        let mut guard = lock_store(&store);
        if let Err(e) = guard.scan_tick() {
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

    let store = Arc::new(Mutex::new(store));
    let ticker_handle: JoinHandle<()> = {
        let store = Arc::clone(&store);
        let stop = Arc::clone(&stop);
        std::thread::Builder::new()
            .name("driverctl-ticker".to_owned())
            .spawn(move || ticker(store, stop))
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
