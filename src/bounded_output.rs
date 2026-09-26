//! A read-only external command's stdout, within a wall-clock budget
//! (Phase 25 review WR-02).
//!
//! `std::process::Command::output` waits without a bound. On the background
//! scans that is a hang with no way out: one `git status` stuck on a dead mount
//! or a hanging `core.fsmonitor`, or one `pgrep` that never returns, keeps the
//! blocking closure from ever reporting back, and the scan's in-flight flag is
//! never cleared. [`stdout_within`] runs the command the way `output()` does,
//! but kills it at the deadline and reports `None`.
//!
//! **This module never constructs a command** — the caller builds it, from a
//! file that is on the spawn allowlist (`tests/spawn_seam_guard.rs`), and hands
//! it here to be run. The pattern is the one
//! `crate::envelope::advisory::run_client` established (spawn, `try_wait`
//! polling, kill at the deadline), with one addition: **stdout is drained on a
//! reader thread while the child runs.** A pipe holds about 64 KiB; a child that
//! writes more than that into an undrained pipe blocks on the write, never
//! exits, and would be killed at the deadline for no fault of its own — a large
//! `git log` or `git status` would read as a timeout.
//!
//! Stdin is null and stderr is discarded: both callers read stdout only, and an
//! undrained stderr pipe would be the same deadlock over again.

use std::io::Read;
use std::process::{Command, ExitStatus, Stdio};
use std::sync::mpsc;
use std::time::{Duration, Instant};

/// The first `try_wait` poll comes this soon after the spawn; each later one
/// doubles the gap, up to [`MAX_POLL`]. Most reads finish in a few
/// milliseconds, so a fixed coarse interval would add its whole length to every
/// call of a scan that makes dozens of them.
const FIRST_POLL: Duration = Duration::from_millis(1);

/// The widest gap between two `try_wait` polls.
const MAX_POLL: Duration = Duration::from_millis(25);

/// The least time the reader is given to reach end-of-file after the child
/// exits, even when the child exited at the very edge of its budget.
const DRAIN_GRACE: Duration = Duration::from_millis(100);

/// A finished command: its exit status and everything it wrote to stdout.
#[derive(Debug)]
pub struct BoundedOutput {
    pub status: ExitStatus,
    pub stdout: Vec<u8>,
}

/// Run `command` to completion within `budget` and return its exit status and
/// stdout.
///
/// `None` when the command could not be spawned, could not be waited on, did
/// not exit within `budget` (it is then killed and reaped), or its stdout did
/// not reach end-of-file within the same budget. A process the command left
/// behind that still holds the stdout pipe open cannot stall the caller: the
/// reader thread is abandoned rather than joined, and ends on its own when that
/// process closes the pipe.
///
/// Blocking; call it from a blocking context (`spawn_blocking`), never from an
/// `async fn`.
pub fn stdout_within(command: &mut Command, budget: Duration) -> Option<BoundedOutput> {
    let deadline = Instant::now() + budget;
    let mut child = command
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::null())
        .spawn()
        .ok()?;

    let (sender, receiver) = mpsc::channel();
    if let Some(mut pipe) = child.stdout.take() {
        std::thread::spawn(move || {
            let mut bytes = Vec::new();
            let read = pipe.read_to_end(&mut bytes).map(|_| bytes);
            // The receiver is gone when the caller already gave up; nothing to do.
            let _ = sender.send(read);
        });
    } else {
        let _ = sender.send(Ok(Vec::new()));
    }

    let mut gap = FIRST_POLL;
    let status = loop {
        match child.try_wait() {
            Ok(Some(status)) => break status,
            Ok(None) => {
                let now = Instant::now();
                if now >= deadline {
                    let _ = child.kill();
                    let _ = child.wait();
                    tracing::warn!(
                        budget_ms = u64::try_from(budget.as_millis()).unwrap_or(u64::MAX),
                        "an external read exceeded its budget and was stopped"
                    );
                    return None;
                }
                std::thread::sleep(gap.min(deadline - now));
                gap = (gap * 2).min(MAX_POLL);
            }
            Err(_) => {
                let _ = child.kill();
                let _ = child.wait();
                return None;
            }
        }
    };

    // A child that exited just inside the deadline still gets a moment for its
    // last bytes to reach the reader.
    let remaining = deadline
        .saturating_duration_since(Instant::now())
        .max(DRAIN_GRACE);
    let stdout = receiver.recv_timeout(remaining).ok()?.ok()?;
    Some(BoundedOutput { status, stdout })
}

// No in-source tests: a test here would have to build a command, and this file
// is not on the spawn allowlist — that holds for test code too. The behaviour
// is pinned by `tests/bounded_output.rs`, outside the audited `src/` tree.
