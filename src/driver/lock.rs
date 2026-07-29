//! The single-execution lock: one driver per project, enforced by the kernel.
//!
//! This module is declared `#[cfg(unix)]` by its parent, so it carries no inner
//! attribute of its own — the portable surface lives in
//! [`crate::driver`](super) and only the implementation is gated (D-05).
//!
//! It is the **first and only file locking in this repository**. There is no
//! `nix`, no `libc`, no `fs4`, no `fs2` and no other `flock` anywhere in `src/`;
//! `NamedTempFile` + `persist` was the only on-disk concurrency primitive that
//! existed before this file. So there is no local idiom to follow, and the five
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
//!    lock — which is why [`acquire`] returns a guard that owns the file rather
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
//!    [`FlockOperation::NonBlockingLockExclusive`].

use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};

use rustix::fs::{flock, FlockOperation};
use serde::{Deserialize, Serialize};

use crate::error::LockError;
use crate::journal::{runs_root, writer};

/// The lock file's name inside the runs root.
const LOCK_FILE_NAME: &str = "run.lock";

/// Where a project's run lock lives.
///
/// `<project>/.planning/meta-manager/runs/run.lock`, and **not**
/// `.planning/meta-manager/run.lock`, which is the literal placement PITFALLS
/// suggested. The rationale lives here rather than at the call sites, following
/// the `queue_md.rs:170-176` precedent of putting the *why* of a layout on the
/// path helper. Two reasons, both about this repository rather than about locks
/// in general:
///
/// * [`writer::RUNS_GITIGNORE_BODY`] already excludes everything directly under
///   `runs/` except `.gitignore` and `*/run.json`. The lock file is therefore
///   kept out of a driven repository's index by a rule that **already exists**,
///   rather than by a new rule someone would have to remember to add — and it is
///   excluded from the moment the directory exists, because
///   [`writer::ensure_runs_root`] writes the ignore file first.
/// * It sits next to `active`, which is the other run-scoped pointer file, so
///   the run neighbourhood stays one directory rather than two.
pub fn lock_path(planning_dir: &Path) -> PathBuf {
    runs_root(planning_dir).join(LOCK_FILE_NAME)
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
/// [`writer::write_run_record`] makes for `run.json`: it is a document read by a
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
/// driver's run state, which is what `src/driver/run.rs` does.
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

/// Take the project's run lock, or say who has it.
///
/// The order of the body is the decision:
///
/// 1. [`writer::ensure_runs_root`] — the ignore file lands **before** the lock
///    file can exist, so there is never a moment at which a file inside a driven
///    repository is unprotected (T-17-12).
/// 2. Open the lock path read-write, creating it if absent, and **never** with
///    truncate. Truncating on open would destroy the holder's record the moment
///    a second process merely *tried*; forbidding truncate-on-open on both the
///    winning and the losing path is cheaper than remembering which is which.
/// 3. `flock` with the **non-blocking** exclusive operation. A blocking acquire
///    is explicitly wrong here: it would turn a duplicate start into a hang
///    (D-20.4). On `EWOULDBLOCK` the losing path reads the holder's record and
///    reports it; any other errno is a real I/O fault and is reported as such.
/// 4. Only **after** winning, write the holder record.
/// 5. Return the guard holding the descriptor.
///
/// `pgid` is the caller's own [`std::process::id`], because `setpgid(0, 0)` ran
/// at run entry and the driver is therefore its own group leader (D-04).
pub fn acquire(planning_dir: &Path, run_id: &str, pgid: u32) -> Result<RunLock, LockError> {
    let root = writer::ensure_runs_root(planning_dir).map_err(|err| LockError::Unavailable {
        detail: format!("{err:#}"),
    })?;
    let path = root.join(LOCK_FILE_NAME);

    // `.truncate(false)` is written out rather than omitted: it is the explicit
    // form of "never `.truncate(true)` here" (step 2 above), and clippy's
    // `suspicious_open_options` asks for a truncate decision beside `create`
    // anyway — so the one place the answer matters states it.
    let mut file = OpenOptions::new()
        .create(true)
        .read(true)
        .write(true)
        .truncate(false)
        .open(&path)
        .map_err(|err| LockError::Unavailable {
            detail: format!("could not open {}: {err}", path.display()),
        })?;

    match flock(&file, FlockOperation::NonBlockingLockExclusive) {
        Ok(()) => {}
        // The one errno that means "somebody else has it". `EAGAIN` is the same
        // value as `EWOULDBLOCK` on every platform this binary targets; rustix
        // spells the constant the way `flock(2)` documents it.
        Err(errno) if errno == rustix::io::Errno::WOULDBLOCK => {
            return Err(match read_holder(planning_dir) {
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
        detail: format!("could not record the lock holder in {}: {err}", path.display()),
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
/// holder is unknown, and the byte-identity assertion in `tests/driver_lock.rs`
/// exists to catch exactly that.
///
/// Every failure — an absent file, an empty file, a short read, invalid JSON —
/// answers `None`. The **caller** turns `None` into
/// [`LockError::HeldByUnknownRun`], never into "not held": this function
/// answers "who", and the kernel has already answered "whether".
pub fn read_holder(planning_dir: &Path) -> Option<LockHolder> {
    let mut file = File::open(lock_path(planning_dir)).ok()?;
    let mut raw = String::new();
    file.read_to_string(&mut raw).ok()?;
    if raw.trim().is_empty() {
        return None;
    }
    serde_json::from_str(&raw).ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// A planning dir under a fresh temp root.
    fn planning() -> tempfile::TempDir {
        let dir = tempfile::TempDir::new().expect("temp dir");
        std::fs::create_dir_all(dir.path().join(".planning")).expect("scratch .planning");
        dir
    }

    #[test]
    fn the_lock_path_sits_under_the_gitignored_runs_root() {
        let root = Path::new("/nonexistent/project/.planning");
        let path = lock_path(root);

        assert_eq!(
            path,
            runs_root(root).join("run.lock"),
            "the lock lives beside `active` in the runs root, not at the meta-manager root"
        );

        // It is excluded by a rule that already exists: the body's catch-all
        // `*`, with nothing re-including it. The two re-inclusions are
        // `.gitignore` and `*/run.json` — one level deeper, and a different
        // name. If a future edit to the body re-includes it, this fails.
        let body = writer::RUNS_GITIGNORE_BODY;
        assert!(
            body.lines().any(|line| line == "*"),
            "the catch-all is what excludes the lock file; the body is now: {body}"
        );
        let name = path
            .file_name()
            .and_then(|n| n.to_str())
            .expect("the lock path has a file name");
        for negation in body.lines().filter(|line| line.starts_with('!')) {
            assert_ne!(
                negation,
                format!("!{name}"),
                "nothing may re-include the lock file into a driven repository's index"
            );
        }
    }

    #[test]
    fn a_holder_record_round_trips_through_the_lock_file() {
        let dir = planning();
        let planning_dir = dir.path().join(".planning");

        let lock = acquire(&planning_dir, "2026-07-29T12-00-00Z-a3f9", 4242)
            .expect("an uncontended acquire succeeds");

        assert_eq!(lock.holder().run_id, "2026-07-29T12-00-00Z-a3f9");
        assert_eq!(lock.holder().pgid, 4242);

        let read_back = read_holder(&planning_dir).expect("the record the winner wrote is readable");
        assert_eq!(
            &read_back,
            lock.holder(),
            "a loser must read back exactly what the winner recorded — that record IS \
             the answer to \"which run holds the lock\""
        );

        // The ignore file landed before the lock file could exist (T-17-12).
        assert!(
            runs_root(&planning_dir).join(".gitignore").is_file(),
            "the runs root's ignore file must exist by the time the lock file does"
        );
    }

    #[test]
    fn an_empty_lock_file_reads_as_an_unknown_holder_not_as_unheld() {
        let dir = planning();
        let planning_dir = dir.path().join(".planning");
        let root = writer::ensure_runs_root(&planning_dir).expect("runs root");
        std::fs::write(root.join("run.lock"), b"").expect("an empty lock file");

        // There is a real window in which the lock is held and the file is still
        // empty: the holder writes its record in a second step after winning.
        // `None` here becomes `HeldByUnknownRun` at the call site, never "not
        // held" — that mapping is what stops a lost race becoming a second run.
        assert_eq!(
            read_holder(&planning_dir),
            None,
            "an empty record yields no holder identity"
        );
    }

    #[test]
    fn a_truncated_holder_record_reads_as_an_unknown_holder_not_as_unheld() {
        let dir = planning();
        let planning_dir = dir.path().join(".planning");
        let root = writer::ensure_runs_root(&planning_dir).expect("runs root");
        // A deliberately truncated prefix of a real record — the shape a
        // half-completed write leaves behind.
        std::fs::write(
            root.join("run.lock"),
            b"{\n  \"run_id\": \"2026-07-29T12-00-00Z-a3f9\",\n  \"pg",
        )
        .expect("a truncated lock file");

        assert_eq!(
            read_holder(&planning_dir),
            None,
            "a partial record parses as no holder, and the caller reports it as held \
             by an unknown run (D-20.3)"
        );
    }
}
