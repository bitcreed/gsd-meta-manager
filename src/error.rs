// Error types for gsd-meta-manager.
//
// The rest of the codebase uses `anyhow::Result` + `.with_context()` at every
// seam existing callers touch, and that stays true (see `config.rs`,
// `registry.rs`, `git_ops.rs`). Only the run lock's refusal below uses a typed
// error: a launcher caller needs a *state* to match on, not a message.
//
// `Display` and `std::error::Error` are hand-written rather than derived. One
// small enum does not justify an error-derive dependency.
//
// (Until Phase 31 this module also held the in-process executor's start, send,
// capability and opt-in errors, plus one deliberate exception: the legacy drive
// error enum's `Display` reached for `crate::ui::screens::sanitize_render_line`.
// All of those went with the executor transport they described.)

use std::fmt;

/// Why the single-execution lock could not be taken (D-19, D-20).
///
/// **The lock itself is `flock(2)` on a held descriptor; these variants only
/// describe the refusal.** The kernel is the enforcement — a variant here is
/// what the *user* is told, and criterion #5 of this phase is entirely about
/// that message: "a second attempt reports **which run** holds the lock instead
/// of starting a second one". A refusal that says only "locked" has not
/// delivered it, which is why [`LockError::HeldBy`] carries all three fields of
/// the holder's record and renders every one of them.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LockError {
    /// Another live run holds the lock, and its own record was readable.
    ///
    /// The three fields are read out of the lock file, which the holder wrote
    /// **after** winning. They are advisory metadata: a hand-edited record can
    /// change this message but cannot grant a second lock, because the kernel's
    /// `flock` and not the file's contents is what refuses (T-17-08).
    HeldBy {
        /// The holding run's id, as the holder recorded it.
        run_id: String,
        /// The holding driver's process group id — the teardown handle.
        pgid: u32,
        /// When the holder acquired the lock, RFC3339.
        started_at: String,
    },
    /// The lock is **held**, but the holder's record was absent, empty, short or
    /// unparseable.
    ///
    /// **D-20.3's rule in full: a partial or empty read is reported as held by
    /// an unknown run, never as not held.** The holder writes its record in a
    /// second step after winning the lock, so there is a real window in which
    /// the lock is taken and the file is still empty. Reading that window as
    /// "not held" would convert a lost race into a second concurrent run, which
    /// is the exact outcome CTRL-05 exists to prevent. Losing the identity of
    /// the holder costs a worse message; losing the refusal costs the invariant.
    HeldByUnknownRun,
    /// The lock file could not be opened, or the runs root could not be created.
    ///
    /// A real I/O fault — a read-only filesystem, a permissions problem, a
    /// vanished path — and deliberately distinct from contention, because the
    /// two call for opposite responses from the user.
    Unavailable {
        /// The underlying failure, rendered.
        detail: String,
    },
}

impl fmt::Display for LockError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::HeldBy {
                run_id,
                pgid,
                started_at,
            } => write!(
                f,
                "another run is already driving this project: run `{run_id}` \
                 (process group {pgid}, started {started_at}). \
                 Only one driver may run against a project at a time (CTRL-05)"
            ),
            Self::HeldByUnknownRun => write!(
                f,
                "another run is already driving this project, but its lock record \
                 could not be read, so the holding run id is unknown. \
                 The lock is held; this is not a reason to start a second run (CTRL-05)"
            ),
            Self::Unavailable { detail } => {
                write!(f, "the run lock could not be taken: {detail}")
            }
        }
    }
}

impl std::error::Error for LockError {}
