//! The in-sandbox bridge: the sandbox's one way out (Phase 28.1, D-08).
//!
//! Inside the network-isolated bwrap profile the driver has no route anywhere.
//! `gsd-meta-manager bridge --port <p> --sock <unix> -- <cmd...>` is bwrap's
//! command: it listens on `127.0.0.1:<p>`, relays every accepted connection to the
//! bound-in egress socket of the launcher-side [`egress`](super::egress) server,
//! and runs the driver as its child.
//!
//! # Numbered facts
//!
//! 1. **The bridge is bwrap's command and the driver is its child**
//!    (`[inferred — audit]` RESEARCH Pattern 4): no fork-exec of our own and no
//!    `libc`. The exit code of `run` is the driver's, so bwrap hands it back to
//!    the launcher unchanged.
//! 2. **Bind first, then start the child.** A port the bridge cannot bind fails
//!    with one stderr line and exit 70, and the driver never starts: a driver
//!    that came up with no way out would only fail later, less legibly.
//! 3. **It relays bytes and decides nothing.** Policy and the run token live in
//!    the launcher-side server (RESEARCH anti-pattern 1), so a driver that kills
//!    or bypasses the bridge gains nothing (T-28.1-24).
//! 4. **One dead connection costs one connection.** A failed connect to the
//!    egress socket drops that TCP stream and the accept loop keeps serving.
//! 5. **It is silent on stdout and writes no file.** `claude`'s stream-json shares
//!    stdout, and the sandbox's data directory may be unwritable, so the bridge
//!    never initialises tracing (`main` dispatches it first) and reports only on
//!    stderr, one `gsd-meta-manager bridge: ...` line per failure.
//! 6. **Exit codes follow the shell.** The driver's own code; `128 + signal` when a
//!    signal ended it; `127` when the program does not exist and `126` when it
//!    exists but cannot be run; `70` for a port that cannot be bound. A caller
//!    (bwrap, then the launcher) can tell "no way out" from "the driver failed".
//! 7. **Synchronous: std threads only** (launcher fact 2). Every connection is two
//!    detached pump threads, ended by EOF on either side.

use std::ffi::OsString;
use std::io::ErrorKind;
use std::net::{Shutdown, TcpListener, TcpStream};
use std::os::unix::net::UnixStream;
use std::os::unix::process::ExitStatusExt;
use std::path::Path;
use std::thread;
use std::time::Duration;

use super::process::spawn_bridged_child;

/// The exit code when the loopback port cannot be bound (`EX_SOFTWARE`).
const EXIT_BIND_FAILED: i32 = 70;

/// The exit code when the driver program does not exist (the shell's 127).
const EXIT_NOT_FOUND: i32 = 127;

/// The exit code when the driver program exists but cannot be run (the shell's 126).
const EXIT_NOT_RUNNABLE: i32 = 126;

/// The exit code when the bridge itself fails after the port is bound.
const EXIT_FAILED: i32 = 1;

/// How long the accept loop waits after an `accept` fault before trying again, so
/// a persistent fault (out of file descriptors) does not spin a core.
const ACCEPT_RETRY: Duration = Duration::from_millis(50);

/// Bind `127.0.0.1:port`, relay it to `sock`, run `command` and return its exit
/// code. See the module docs for the contract.
pub fn run(port: u16, sock: &Path, command: &[OsString]) -> i32 {
    let listener = match TcpListener::bind(("127.0.0.1", port)) {
        Ok(listener) => listener,
        Err(e) => {
            eprintln!("gsd-meta-manager bridge: cannot bind 127.0.0.1:{port}: {e}");
            return EXIT_BIND_FAILED;
        }
    };
    let sock = sock.to_path_buf();
    let accept = thread::Builder::new()
        .name("gsdmm-bridge-accept".to_owned())
        .spawn(move || accept_loop(&listener, &sock));
    if let Err(e) = accept {
        eprintln!("gsd-meta-manager bridge: cannot start the accept thread: {e}");
        return EXIT_FAILED;
    }

    let mut child = match spawn_bridged_child(command) {
        Ok(child) => child,
        Err(e) => {
            eprintln!("gsd-meta-manager bridge: cannot start the driver: {e}");
            return if e.kind() == ErrorKind::NotFound {
                EXIT_NOT_FOUND
            } else {
                EXIT_NOT_RUNNABLE
            };
        }
    };
    match child.wait() {
        Ok(status) => status
            .code()
            .or_else(|| status.signal().map(|signal| 128 + signal))
            .unwrap_or(EXIT_FAILED),
        Err(e) => {
            eprintln!("gsd-meta-manager bridge: cannot wait for the driver: {e}");
            EXIT_FAILED
        }
    }
}

/// Accept connections for ever; the process ending ends this thread.
fn accept_loop(listener: &TcpListener, sock: &Path) {
    loop {
        match listener.accept() {
            Ok((tcp, _)) => {
                let sock = sock.to_path_buf();
                // A thread that cannot start costs this one connection (the
                // stream is dropped with the closure); the loop carries on.
                let _ = thread::Builder::new()
                    .name("gsdmm-bridge-conn".to_owned())
                    .spawn(move || serve_connection(tcp, &sock));
            }
            Err(e) if e.kind() == ErrorKind::Interrupted => {}
            Err(_) => thread::sleep(ACCEPT_RETRY),
        }
    }
}

/// Relay one connection both ways. A failed connect to the egress socket drops
/// `tcp` and nothing else.
fn serve_connection(tcp: TcpStream, sock: &Path) {
    let Ok(unix) = UnixStream::connect(sock) else {
        return;
    };
    let (Ok(tcp_read), Ok(unix_write)) = (tcp.try_clone(), unix.try_clone()) else {
        return;
    };
    let upstream = thread::Builder::new()
        .name("gsdmm-bridge-up".to_owned())
        .spawn(move || {
            let mut from = tcp_read;
            let mut to = unix_write;
            let _ = std::io::copy(&mut from, &mut to);
            let _ = to.shutdown(Shutdown::Write);
        });
    if upstream.is_err() {
        return;
    }
    let mut from = unix;
    let mut to = tcp;
    let _ = std::io::copy(&mut from, &mut to);
    let _ = to.shutdown(Shutdown::Write);
}
