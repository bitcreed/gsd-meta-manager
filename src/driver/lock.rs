//! The single-execution lock: one driver per project, enforced by the kernel.
//!
//! This module is declared `#[cfg(unix)]` by its parent, so it carries no inner
//! attribute of its own — the portable surface lives in
//! [`crate::driver`](super) and only the implementation is gated (D-05).
//!
//! It is the **first and only file locking in this repository**. There is no
//! `nix`, no `libc`, no `fs4`, no `fs2` and no other `flock` anywhere in `src/`;
//! `NamedTempFile` + `persist` was the only on-disk concurrency primitive that
//! existed before this file. So there is no local idiom to follow, and the seven
//! facts below are each a way the naive version silently fails.
//!
//! 1. **It is `flock(2)`, never a PID file and never an in-memory flag (D-19).**
//!    An advisory lock is process-death-safe: the kernel releases it when the
//!    last descriptor referring to that open file description closes, *including*
//!    when the holder is `SIGKILL`ed. A PID file survives its owner and then
//!    either refuses forever or has to guess whether a recycled pid is the same
//!    process. An in-memory flag in the TUI is worse still, because the TUI is
//!    the process that is *expected* to exit while the run continues.
//! 2. **The driver acquires it, in its own process, after exec (D-20.1).** Not
//!    the TUI. The TUI can exit; the lock must not.
//! 3. **The descriptor is held for the run's duration (D-20.2).** `flock` is per
//!    *open file description*, so dropping the [`std::fs::File`] releases the
//!    lock — which is why [`acquire_at`] returns a guard that owns the file rather
//!    than a `()` that would release before it returned, and why [`RunLock`]
//!    must be stored in the driver's run state and not in a local that falls out
//!    of scope. Rust opens files `O_CLOEXEC`, so the descriptor is **not**
//!    inherited by the spawned agent; if it were, the lock would outlive a
//!    `SIGKILL`ed driver for as long as its grandchildren lived, and fact 1
//!    would quietly stop being true.
//! 4. **The loser opens read-only and never truncates (D-20.3).** An `O_TRUNC`
//!    on the losing path destroys the very information it came to read. A
//!    partial or empty read is reported as held by an *unknown* run and never as
//!    *not held*, because the alternative converts a lost race into a second
//!    concurrent run — see [`crate::error::LockError::HeldByUnknownRun`].
//! 5. **The acquire is non-blocking (D-20.4).** A blocking acquire turns a
//!    duplicate start into a hang, and a hang is the failure mode a user cannot
//!    diagnose. There is exactly one operation used here and it is
//!    [`FlockOperation::NonBlockingLockExclusive`] per *attempt* — see fact 7 for
//!    why an acquirer may nonetheless make several bounded attempts.
//! 6. **Readers probe with `LOCK_SH` (D-20, 27-RESEARCH E3).** A liveness probe
//!    that took `LOCK_EX` would make two concurrent readers see each other as a
//!    holder. [`probe_at`] opens read-only (never creating the file) and takes
//!    [`FlockOperation::NonBlockingLockShared`], so any number of readers agree
//!    the file is free while an exclusive holder still shows as held.
//! 7. **The acquirer retries for a bounded budget (D-20, 27-RESEARCH E6).** A
//!    probe's shared hold is momentary but real: an acquirer that happens to land
//!    inside it would be refused and report a false "already running" naming a
//!    stale pgid. [`acquire_at_with_retry`] therefore re-attempts with a
//!    doubling, capped backoff until a budget elapses. Each attempt is still
//!    non-blocking and the loop is bounded, so it never hangs; a real holder is
//!    still refused, just after the budget instead of instantly.

use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

use rustix::fs::{flock, FlockOperation};
use serde::{Deserialize, Serialize};

use crate::error::LockError;

/// The per-RUN lock file's name inside a run directory (D-20).
///
/// A run directory's `launcher.lock` guards one launcher. (Until Phase 31 a
/// per-project `run.lock` under `.planning/meta-manager/runs/` guarded the legacy
/// `drive`; that file is no longer written or read, and an old one is left on
/// disk untouched.)
pub const RUN_LOCK_FILE: &str = "launcher.lock";

/// How long [`acquire_at_with_retry`] keeps trying before it reports the refusal.
///
/// `[inferred — audit]`: 500 ms. The worst measured probe-induced refusal window
/// was 63 ms (27-RESEARCH E6), so 500 ms covers it by a wide margin while a real
/// holder is still reported within half a second.
pub const ACQUIRE_RETRY_BUDGET: Duration = Duration::from_millis(500);

/// Where a run directory's launcher lock lives (D-20).
pub fn run_lock_path(run_dir: &Path) -> PathBuf {
    run_dir.join(RUN_LOCK_FILE)
}

/// Who holds the lock, as the holder itself recorded it.
///
/// **Advisory metadata only, and it must never be promoted to an authority.**
/// The kernel's `flock` is the enforcement; this record exists so a loser can
/// answer *which* run holds the lock, which is the entire content of this
/// phase's success criterion #5. A forged or hand-edited record can change the
/// refusal *message* and can never grant a second lock, so the worst outcome of
/// tampering is a misleading string (T-17-08).
///
/// Serialised as pretty JSON, by the same argument
/// the legacy journal writer made for `run.json`: it is a document read by a
/// human at least as often as by the program.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LockHolder {
    /// The holding run's id.
    pub run_id: String,
    /// The holding driver's process group id. Equal to its pid, because
    /// `setpgid(0, 0)` ran at run entry (D-04).
    pub pgid: u32,
    /// When the lock was acquired, RFC3339 to second precision.
    pub started_at: String,
}

/// An acquired lock, alive for exactly as long as this value is.
///
/// **This type exists because of D-20.2 and for no other reason.** `flock` is
/// per open file description: the lock lasts until the last descriptor for that
/// description closes. A function that locked and returned `()` would release
/// the lock before it returned, and a guard bound to a local that falls out of
/// scope is the same bug one line later. The guard must be stored in the
/// launcher's run state (`launcher::run`).
///
/// There is deliberately **no `impl Drop`**. Closing the descriptor is what
/// releases the lock and the kernel does that when the [`File`] is dropped; an
/// explicit unlock would add a second release path with no third property to
/// buy, and two release paths for one resource is how a release comes to happen
/// twice or not at all. Please do not add one.
pub struct RunLock {
    /// The held descriptor. **Never read, and that is precisely its job**: the
    /// lock is alive for exactly as long as this file is open, so this field is
    /// the mechanism rather than bookkeeping. It is not renamed to `_file`,
    /// because an underscore reads as "leftover" and would invite the next
    /// reader to delete it.
    #[allow(dead_code)]
    file: File,
    /// The record this run wrote into the lock file after winning.
    holder: LockHolder,
}

impl RunLock {
    /// What this lock's own file says about it.
    pub fn holder(&self) -> &LockHolder {
        &self.holder
    }
}

/// Take the lock at `path`, or say who has it: one non-blocking attempt, no retry.
///
/// The order of the body is the decision:
///
/// 1. Open the lock path read-write, creating it if absent, and **never** with
///    truncate. Truncating on open would destroy the holder's record the moment
///    a second process merely *tried*; forbidding truncate-on-open on both the
///    winning and the losing path is cheaper than remembering which is which.
/// 2. `flock` with the **non-blocking** exclusive operation. A blocking acquire
///    is explicitly wrong here: it would turn a duplicate start into a hang
///    (D-20.4). On `EWOULDBLOCK` the losing path reads the holder's record and
///    reports it; any other errno is a real I/O fault and is reported as such.
/// 3. Only **after** winning, write the holder record.
/// 4. Return the guard holding the descriptor.
///
/// The caller owns making sure the directory exists.
pub fn acquire_at(path: &Path, run_id: &str, pgid: u32) -> Result<RunLock, LockError> {
    // `.truncate(false)` is written out rather than omitted: it is the explicit
    // form of "never `.truncate(true)` here" (step 1 above), and clippy's
    // `suspicious_open_options` asks for a truncate decision beside `create`
    // anyway — so the one place the answer matters states it.
    let mut file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(path)
        .map_err(|err| LockError::Unavailable {
            detail: format!("could not open {}: {err}", path.display()),
        })?;

    match flock(&file, FlockOperation::NonBlockingLockExclusive) {
        Ok(()) => {}
        // The one errno that means "somebody else has it". `EAGAIN` is the same
        // value as `EWOULDBLOCK` on every platform this binary targets; rustix
        // spells the constant the way `flock(2)` documents it.
        Err(errno) if errno == rustix::io::Errno::WOULDBLOCK => {
            return Err(match read_holder_at(path) {
                Some(holder) => LockError::HeldBy {
                    run_id: holder.run_id,
                    pgid: holder.pgid,
                    started_at: holder.started_at,
                },
                // Held, with an unreadable record. Never "not held" (D-20.3).
                None => LockError::HeldByUnknownRun,
            });
        }
        Err(errno) => {
            return Err(LockError::Unavailable {
                detail: format!("flock on {} failed: {errno}", path.display()),
            });
        }
    }

    let holder = LockHolder {
        run_id: run_id.to_string(),
        pgid,
        started_at: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
    };

    // Truncate-then-write **through the descriptor already held**. This looks
    // like opening with truncate and is not the same act: the lock is already
    // ours and no other process's record is being destroyed, only our own
    // predecessor's stale bytes.
    //
    // And emphatically NOT a temp file plus `persist`, which is the idiom
    // `write_run_record` uses two modules away and the single most likely way a
    // reader of it gets this wrong: a rename replaces the *inode*, so the lock
    // this process holds would go on protecting a file nobody can see while the
    // path it names is unlocked — silently handing the lock away.
    write_holder(&mut file, &holder).map_err(|err| LockError::Unavailable {
        detail: format!(
            "could not record the lock holder in {}: {err}",
            path.display()
        ),
    })?;

    Ok(RunLock { file, holder })
}

/// Replace the lock file's contents with `holder`, in place.
fn write_holder(file: &mut File, holder: &LockHolder) -> std::io::Result<()> {
    let json = serde_json::to_string_pretty(holder)
        .map_err(|err| std::io::Error::new(std::io::ErrorKind::InvalidData, err))?;

    file.set_len(0)?;
    file.rewind()?;
    file.write_all(json.as_bytes())?;
    file.write_all(b"\n")?;
    file.flush()?;

    // 0644 for the same reason `write_run_record` sets it: the loser may be a
    // different uid on the same host — a container mount, a shared CI runner —
    // and the one file whose whole purpose is being read by somebody else must
    // be readable by them.
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        file.set_permissions(std::fs::Permissions::from_mode(0o644))?;
    }

    Ok(())
}

/// Read whoever wrote the lock file last, for a caller that just lost the race.
///
/// **This function never opens for writing and never truncates.** It is the
/// losing path, and an `O_TRUNC` here would destroy the very information it came
/// to read (D-20.3) — after which every subsequent loser would be told the
/// holder is unknown.
///
/// Every failure — an absent file, an empty file, a short read, invalid JSON —
/// answers `None`. The **caller** turns `None` into
/// [`LockError::HeldByUnknownRun`], never into "not held": this function
/// answers "who", and the kernel has already answered "whether".
pub fn read_holder_at(path: &Path) -> Option<LockHolder> {
    let mut file = File::open(path).ok()?;
    let mut raw = String::new();
    file.read_to_string(&mut raw).ok()?;
    if raw.trim().is_empty() {
        return None;
    }
    serde_json::from_str(&raw).ok()
}

/// [`acquire_at`] with a bounded retry (fact 7, D-20).
///
/// Re-attempts on `HeldBy` / `HeldByUnknownRun` with a sleep that starts at 1 ms,
/// doubles, and is capped at 50 ms per sleep, until `budget` has elapsed; the
/// last refusal is then returned. `Unavailable` is a real fault and returns
/// immediately. Always makes at least one attempt, even for a zero budget.
pub fn acquire_at_with_retry(
    path: &Path,
    run_id: &str,
    pgid: u32,
    budget: Duration,
) -> Result<RunLock, LockError> {
    const MAX_SLEEP: Duration = Duration::from_millis(50);
    let started = Instant::now();
    let mut sleep = Duration::from_millis(1);
    loop {
        match acquire_at(path, run_id, pgid) {
            Ok(lock) => return Ok(lock),
            Err(refusal @ (LockError::HeldBy { .. } | LockError::HeldByUnknownRun)) => {
                let elapsed = started.elapsed();
                if elapsed >= budget {
                    return Err(refusal);
                }
                // Never sleep past the budget: the caller's bound is a bound.
                std::thread::sleep(sleep.min(budget - elapsed));
                sleep = (sleep * 2).min(MAX_SLEEP);
            }
            Err(other) => return Err(other),
        }
    }
}

/// What a read-only probe of a lock file found.
///
/// Carries the same posture as [`crate::driver::liveness::Liveness`]: `Unknown`
/// is an honest "could not tell" and **is never a synonym for `Free`**. A caller
/// that cannot distinguish `Unknown` from `Free` has converted an I/O fault into
/// a claim that nothing is running.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LockProbe {
    /// An exclusive holder is alive.
    Held,
    /// The file exists and nothing holds it exclusively.
    Free,
    /// The file does not exist (the probe never creates it).
    Absent,
    /// The probe could not tell; carries the errno that stopped it.
    Unknown(rustix::io::Errno),
}

/// Probe whether `path` is exclusively held, without ever disturbing it (fact 6).
///
/// `File::open` is `O_RDONLY` and so never creates the file; `NotFound` is
/// `Absent`. A `LOCK_SH | LOCK_NB` that succeeds means `Free` (the shared lock is
/// released when the `File` drops at the end of this function); `EWOULDBLOCK`
/// means `Held`; any other errno means `Unknown`.
pub fn probe_at(path: &Path) -> LockProbe {
    let file = match File::open(path) {
        Ok(file) => file,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return LockProbe::Absent,
        Err(err) => {
            return LockProbe::Unknown(
                rustix::io::Errno::from_io_error(&err).unwrap_or(rustix::io::Errno::IO),
            )
        }
    };
    match flock(&file, FlockOperation::NonBlockingLockShared) {
        Ok(()) => LockProbe::Free,
        Err(errno) if errno == rustix::io::Errno::WOULDBLOCK => LockProbe::Held,
        Err(errno) => LockProbe::Unknown(errno),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_holder_record_round_trips_through_the_lock_file() {
        let dir = tempfile::TempDir::new().expect("temp dir");
        let path = run_lock_path(dir.path());

        let lock = acquire_at(&path, "2026-07-29T12-00-00Z-a3f9", 4242)
            .expect("an uncontended acquire succeeds");

        assert_eq!(lock.holder().run_id, "2026-07-29T12-00-00Z-a3f9");
        assert_eq!(lock.holder().pgid, 4242);

        let read_back = read_holder_at(&path).expect("the record the winner wrote is readable");
        assert_eq!(
            &read_back,
            lock.holder(),
            "a loser must read back exactly what the winner recorded — that record IS \
             the answer to \"which run holds the lock\""
        );
    }

    #[test]
    fn an_empty_lock_file_reads_as_an_unknown_holder_not_as_unheld() {
        let dir = tempfile::TempDir::new().expect("temp dir");
        let path = run_lock_path(dir.path());
        std::fs::write(&path, b"").expect("an empty lock file");

        // There is a real window in which the lock is held and the file is still
        // empty: the holder writes its record in a second step after winning.
        // `None` here becomes `HeldByUnknownRun` at the call site, never "not
        // held" — that mapping is what stops a lost race becoming a second run.
        assert_eq!(
            read_holder_at(&path),
            None,
            "an empty record yields no holder identity"
        );
    }

    #[test]
    fn a_truncated_holder_record_reads_as_an_unknown_holder_not_as_unheld() {
        let dir = tempfile::TempDir::new().expect("temp dir");
        let path = run_lock_path(dir.path());
        // A deliberately truncated prefix of a real record — the shape a
        // half-completed write leaves behind.
        std::fs::write(
            &path,
            b"{\n  \"run_id\": \"2026-07-29T12-00-00Z-a3f9\",\n  \"pg",
        )
        .expect("a truncated lock file");

        assert_eq!(
            read_holder_at(&path),
            None,
            "a partial record parses as no holder, and the caller reports it as held \
             by an unknown run (D-20.3)"
        );
    }

    #[test]
    fn probe_at_reports_absent_free_and_held() {
        let dir = tempfile::TempDir::new().expect("temp dir");
        let path = run_lock_path(dir.path());

        assert_eq!(
            probe_at(&path),
            LockProbe::Absent,
            "a probe must report a missing file as Absent and must not create it"
        );
        assert!(!path.exists(), "probing never creates the lock file");

        std::fs::write(&path, b"").expect("an unheld lock file");
        assert_eq!(probe_at(&path), LockProbe::Free, "an unheld file is Free");

        let guard = acquire_at(&path, "r1", 1).expect("uncontended acquire");
        assert_eq!(
            probe_at(&path),
            LockProbe::Held,
            "a live exclusive holder must show as Held"
        );
        drop(guard);
        assert_eq!(
            probe_at(&path),
            LockProbe::Free,
            "dropping the guard releases the lock"
        );
    }

    #[test]
    fn run_lock_path_names_launcher_lock() {
        assert_eq!(
            run_lock_path(Path::new("/r/run-1")),
            Path::new("/r/run-1/launcher.lock")
        );
    }
}
