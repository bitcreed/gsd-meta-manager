//! The append-only pull-request ledger and the rolling-window cap (SAFE-06,
//! D-19, D-20).
//!
//! One NDJSON file per alias, at `<envelope>/<alias>/pr-ledger.ndjson`, opened
//! `create(true).append(true)` — the same discipline
//! [`crate::journal::writer::JournalWriter::open`] uses, and for the same
//! reason: an append-only file survives a torn write with at most the last line
//! damaged, and no reader has to trust a writer that died mid-record.
//!
//! ## Where it lives, and what that placement prevents
//!
//! **Outside the driven repository**, under the alias's envelope directory. A
//! cap the driven agent can reset is not a cap: a ledger inside the worktree
//! would be removable by the very `Bash` tool call the cap exists to bound, and
//! `git checkout -- .` would do it without even naming the file. The placement
//! is asserted by a test rather than only stated here.
//!
//! ## Rolling, not calendar
//!
//! The window is the 24 hours preceding the attempt, not "today". A calendar day
//! lets a run at 23:50 open three pull requests and three more ten minutes
//! later — six in twenty minutes, every one of them inside the cap.
//!
//! ## The boundary is inclusive, and that is a decision
//!
//! An entry stamped **exactly** 24 hours before the attempt counts toward the
//! window. The requirement does not fix the tie-break, so the direction that
//! over-counts was chosen, consistent with [D-20](#the-ordering-is-the-decision)
//! below. Comparison is integer seconds throughout: there is no floating-point
//! arithmetic anywhere in this decision, because a cap that rounds is a cap that
//! is off by one at exactly the moment somebody looks.
//!
//! ## The ordering is the decision (D-20)
//!
//! See [`record_and_check`]. The entry is appended **before** the verdict is
//! returned, never after the tool call succeeds.

use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{anyhow, Context};
use serde::{Deserialize, Serialize};

use super::policy::{EnvelopePolicy, ParkReason};

/// The ledger's filename inside the alias's envelope directory.
const LEDGER_FILE: &str = "pr-ledger.ndjson";

/// The rolling window, in seconds. Integer, and integer for a reason: see the
/// module doc.
const WINDOW_SECS: i64 = 24 * 60 * 60;

/// The largest ledger [`record_and_check_in`] will read before refusing
/// (`T-19-117`).
///
/// # WHY A BOUND AT ALL, AND WHY IT REFUSES RATHER THAN READS LESS
///
/// [`record_and_check_in`] reads this file WHOLE and [`tally`] walks every line,
/// on the guard's registered critical path against
/// [`super::hooks::GUARD_TIMEOUT_SECS`]. **The file is append-only and a command
/// the guard PERMITS can inflate it** — an append whose target arrives after a
/// redirection operator was exit 0 before plan 19-29, and is exit 0 still at
/// every spelling that rule gets no rule for. Measured on this machine:
///
/// ```text
///   ledger bytes     `gh pr create`     `git push --force`
///   0                        49 ms                  44 ms
///   1,010,000                96 ms                  47 ms
///   20,200,000            1,008 ms                  46 ms
///   202,000,000           7,900 ms                  46 ms      <- past a 5 s deadline
/// ```
///
/// **The forge/non-forge split is the shape of the finding**: the cost falls only
/// on commands that reach this function, so the remedy belongs here and not in
/// the guard at large.
///
/// **IT MUST FAIL CLOSED, AND A TAIL READ IS THE WRONG SHAPE.** [`tally`]'s own
/// doc states the invariant — *"a line this function cannot parse is COUNTED,
/// not skipped … under-counting is the one failure this file exists to
/// prevent"*. A tail read, a line cap and sampling all UNDER-COUNT by
/// construction, which violates that invariant **in the direction that
/// permits**. Refusing preserves it: the guard says it cannot establish the
/// count rather than establishing a smaller one.
///
/// **The contrast is three functions below.** [`ends_mid_line`] already refuses a
/// whole-file read for a one-byte question, on exactly this reasoning — *"a
/// whole-file read to answer a one-byte question is the sort of thing that turns
/// a guard into a hang"* — and the read beneath it was unbounded until now.
///
/// # THE DERIVATION: FROM THE DEADLINE, NEVER FROM THE CAPS
///
/// From the curve above, [`tally`] costs roughly **41 ms per megabyte**. At
/// **8 MiB** that is about **340 ms**, under **7%** of a five-second deadline,
/// which leaves the rest of the budget for the rest of the guard.
///
/// **A CAP-DERIVED BOUND WOULD BE A BOUND THE AGENT STEERS.** `pr_cap_per_24h`
/// and `pr_cap_per_run` are UNCLAMPED and are read from a file the driven agent
/// can write (`C-15`), so deriving from them would hand the value to the party
/// the value bounds. They are not clamped here and this constant is not
/// computed from them. For SCALE rather than for derivation: at the default cap
/// of three per 24 hours and about 101 bytes per entry, 8 MiB is roughly
/// seventy-five years of maximal legitimate appending — which is why a
/// long-lived envelope does not reach it and a deliberately inflated one does.
///
/// # THE OVER-REFUSAL, DISCLOSED FROM BOTH SIDES
///
/// A ledger past this bound refuses **the forge commands the cap governs** —
/// `gh pr create` and its siblings — and **nothing else**: every command that
/// does not reach this function is unaffected, including every `git` command and
/// every ordinary shell call. The refusal names the file and says it can be
/// archived, so it is one a human or an agent can act on (AR-19-11). A ledger
/// just UNDER the bound still permits **and still counts**, which is the half
/// that keeps this a size bound rather than a disarmed cap.
const MAX_LEDGER_BYTES: u64 = 8 * 1024 * 1024;

/// One recorded pull-request attempt.
///
/// Serialised as one JSON object per line. The field names are the on-disk
/// format an already-written ledger is read back with, so renaming one means
/// tolerating both spellings forever — which is why the shape is fixed by D-19
/// rather than chosen here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LedgerEntry {
    /// RFC3339 UTC at **second** precision.
    ///
    /// Produced by `chrono::Utc::now().to_rfc3339_opts(SecondsFormat::Secs,
    /// true)` — the same call `JournalRun::finish` and
    /// [`crate::config::DriverOptIn::opted_in_at`] use, so a ledger stamp and a
    /// run stamp compare directly without normalising.
    pub at: String,
    /// The run this attempt belongs to, which is what the per-run cap counts.
    pub run_id: String,
    /// The **classified shape** of the attempt — `gh pr create`, `gh api POST
    /// …/pulls`, `glab mr create` — and deliberately never the command line.
    ///
    /// The same rule [`super::policy::GitVerdict::Refuse`]'s `detail` records: a
    /// field that quotes the command back is a field that can carry a token into
    /// a file on disk (SAFE-04). This ledger is not a transcript.
    pub command: String,
    /// The forge the attempt was aimed at, for a human reading the file later.
    pub platform: String,
}

/// Where this alias's ledger lives, or `None` for a hostile alias.
///
/// The `Option` does the job it does in [`super::envelope_dir`]: it conscripts
/// the compiler into making every caller decide about an alias that is not a
/// plain path component, rather than leaving a validation helper that merely
/// exists to be called at some of the sites.
pub fn ledger_path(alias: &str) -> Option<PathBuf> {
    super::envelope_dir(alias).map(|dir| dir.join(LEDGER_FILE))
}

/// [`ledger_path`] against an explicit envelope root.
///
/// Split out for the same reason [`super::hooks::install_in`] is: a test may not
/// write into the developer's real `~/.local/share`.
pub fn ledger_path_in(root: &Path, alias: &str) -> Option<PathBuf> {
    super::envelope_dir_in(root, alias).map(|dir| dir.join(LEDGER_FILE))
}

/// What the cap decided about one pull-request attempt.
///
/// Both arms carry the counts, because a refusal that only says "no" leaves a
/// human unable to tell a per-run bound from a rolling-window one — and those
/// two have completely different remedies (wait, versus start a new run).
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CapVerdict {
    /// Within both bounds. The counts **include** this attempt.
    Permit {
        /// Attempts inside the rolling window, including this one.
        used_24h: u32,
        /// Attempts by this run, including this one.
        used_run: u32,
    },
    /// Beyond one of the bounds. The entry is already on disk regardless (D-20).
    Refuse {
        /// Always [`ParkReason::PrCapExceeded`]; carried rather than implied so
        /// a caller writes `reason.as_str()` and cannot invent a string.
        reason: ParkReason,
        /// Attempts inside the rolling window, including this one.
        used_24h: u32,
        /// The configured rolling-window cap.
        cap_24h: u32,
        /// Attempts by this run, including this one.
        used_run: u32,
        /// The configured per-run cap.
        cap_run: u32,
    },
}

impl CapVerdict {
    /// The human-facing refusal line, or `None` for a permit.
    ///
    /// **It states the over-count bias out loud**, because D-20's bias is
    /// exactly the kind of thing a user otherwise discovers by having a failed
    /// attempt cost a slot and concluding the tool is broken. A limitation that
    /// is stated is a limitation; a limitation that is discovered is a bug
    /// report.
    pub fn refusal_detail(&self) -> Option<String> {
        match self {
            CapVerdict::Permit { .. } => None,
            CapVerdict::Refuse {
                used_24h,
                cap_24h,
                used_run,
                cap_run,
                ..
            } => {
                let bound = if used_run > cap_run {
                    format!("the per-run cap ({used_run} of {cap_run} used by this run)")
                } else {
                    format!(
                        "the rolling 24-hour cap ({used_24h} of {cap_24h} used in the last 24 hours)"
                    )
                };
                Some(format!(
                    "pull-request creation is refused: this attempt is beyond {bound}. \
                     The count is deliberately biased to over-count — every attempt is \
                     recorded BEFORE it is permitted, so an attempt that was recorded and \
                     then failed still costs a slot. Under-counting is the failure that \
                     matters, so that is the correct direction to be wrong in."
                ))
            }
        }
    }
}

/// Record one pull-request attempt and return the cap's verdict.
///
/// **The ordering is the decision (D-20), and it is the reverse of the obvious
/// one.** The entry is appended *before* the verdict is returned, not after the
/// tool call succeeds. The guard is the only point that observes the attempt at
/// all: a pull request opened through a path the guard never saw — a script the
/// agent wrote, a nested shell, an evasion of the argv split — is one this
/// ledger cannot know about. So the cap is deliberately biased to **over**
/// count: a recorded-but-failed `gh pr create` costs one slot out of three.
/// Under-counting is the failure that matters, which makes over-counting the
/// correct direction to be wrong in, and [`CapVerdict::refusal_detail`] says so
/// in the refusal itself rather than leaving it to be discovered.
///
/// This is the same shape of recorded asymmetry
/// `crate::journal::writer::JournalWriter::append` documents for its own cap:
/// the trade is named at the function, not left in a commit message.
pub fn record_and_check(
    alias: &str,
    entry: &LedgerEntry,
    policy: &EnvelopePolicy,
) -> anyhow::Result<CapVerdict> {
    let root = super::envelope_root().ok_or_else(|| {
        anyhow!(
            "no application data directory is resolvable, and the envelope refuses to keep \
             a pull-request ledger in a directory that could sit inside a repository"
        )
    })?;
    record_and_check_in(&root, alias, entry, policy)
}

/// [`record_and_check`] against an explicit envelope root.
pub fn record_and_check_in(
    root: &Path,
    alias: &str,
    entry: &LedgerEntry,
    policy: &EnvelopePolicy,
) -> anyhow::Result<CapVerdict> {
    let path = ledger_path_in(root, alias).ok_or_else(|| {
        anyhow!("alias {alias:?} is not a plain path component, so it has no ledger")
    })?;

    // The attempt's own stamp is the window's origin. An attempt whose stamp
    // cannot be parsed is an attempt whose window cannot be computed, and the
    // only safe answer to "I cannot compute the bound" is to refuse — which is
    // what the `Err` becomes at the guard.
    let now = parse_stamp(&entry.at).ok_or_else(|| {
        anyhow!(
            "the attempt's own timestamp is not RFC3339, so no rolling window can be \
             computed for it and the attempt is refused rather than guessed at"
        )
    })?;

    // **THE SIZE BOUND, BEFORE THE APPEND AND BEFORE THE READ** (`T-19-117`).
    // A ledger this large is one whose count cannot be established inside the
    // guard's registered deadline, so the guard says so rather than establishing
    // a smaller count from a partial read — see `MAX_LEDGER_BYTES` for the
    // derivation, why it refuses rather than reads less, and the over-refusal
    // disclosed from both sides. The check is a `stat`, not a read: one syscall
    // against a file that may be hundreds of megabytes, which is the same
    // reasoning `ends_mid_line` below is built on. A file that cannot be stat'd
    // is not a file this check can refuse on — the append that follows reports
    // the real error with the real context rather than this probe guessing.
    if let Ok(size) = std::fs::metadata(&path).map(|meta| meta.len()) {
        if size > MAX_LEDGER_BYTES {
            // An `Err` rather than a `CapVerdict::Refuse`, and the distinction is
            // D-24: `CapVerdict::Refuse` carries `ParkReason::PrCapExceeded`,
            // which names a cap that FIRED — and this ledger was never tallied.
            // The guard's `Err` arm parks at
            // `ParkReason::EnvelopeAssertionFailed`, the general unresolvable
            // identifier the sibling refusals already carry, which is the honest
            // attribution: an error reaching a verdict is not a verdict.
            return Err(anyhow!(
                "the pull-request ledger at {} is {size} bytes, past the {MAX_LEDGER_BYTES}-byte \
                 bound this guard can count inside its registered {}-second deadline, so how \
                 many pull requests this run has already opened cannot be established and the \
                 attempt is refused rather than counted from a partial read. To proceed: \
                 archive or remove that file — it is the envelope's own record and no other \
                 command is affected by this refusal",
                path.display(),
                super::hooks::GUARD_TIMEOUT_SECS
            ));
        }
    }

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("failed to create {}", parent.display()))?;
    }

    append_entry(&path, entry)?;

    let bytes = std::fs::read(&path)
        .with_context(|| format!("failed to read the ledger at {}", path.display()))?;
    let tally = tally(&bytes, now, &entry.run_id);

    if tally.used_24h > policy.pr_cap_per_24h || tally.used_run > policy.pr_cap_per_run {
        return Ok(CapVerdict::Refuse {
            reason: ParkReason::PrCapExceeded,
            used_24h: tally.used_24h,
            cap_24h: policy.pr_cap_per_24h,
            used_run: tally.used_run,
            cap_run: policy.pr_cap_per_run,
        });
    }

    Ok(CapVerdict::Permit {
        used_24h: tally.used_24h,
        used_run: tally.used_run,
    })
}

/// Append one line, `create(true).append(true)`.
///
/// One `write_all` of the whole line including its newline, so the kernel is
/// asked for one append rather than two: a line and its terminator written
/// separately are two chances to be interrupted between them.
///
/// **The line is prefixed with a newline when the existing file does not end in
/// one**, and that detail is load-bearing rather than tidy. A torn write leaves
/// a final line with no terminator; appending straight onto it would fuse the
/// torn record and the new one into a single unparseable line, which counts as
/// **one** attempt instead of two — so the very failure the append-only format
/// is meant to bound would swallow the attempt being recorded. Terminating the
/// torn line first keeps it counted as the one damaged attempt it is, and keeps
/// the new attempt countable as its own.
fn append_entry(path: &Path, entry: &LedgerEntry) -> anyhow::Result<()> {
    let mut line = serde_json::to_string(entry)
        .context("failed to render a ledger entry as JSON")?;
    line.push('\n');

    if ends_mid_line(path) {
        line.insert(0, '\n');
    }

    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("failed to open the ledger at {}", path.display()))?;
    file.write_all(line.as_bytes())
        .with_context(|| format!("failed to append to the ledger at {}", path.display()))?;
    file.flush()
        .with_context(|| format!("failed to flush the ledger at {}", path.display()))?;
    Ok(())
}

/// Whether the file exists, is non-empty and does **not** end with a newline —
/// the signature of a torn final record.
///
/// Reads one byte from the end rather than the whole file: this runs on the
/// agent's critical path inside the guard, and a whole-file read to answer a
/// one-byte question is the sort of thing that turns a guard into a hang.
/// An unreadable file answers `false`, because the append that follows will
/// report the real error with the real context rather than this probe guessing.
fn ends_mid_line(path: &Path) -> bool {
    use std::io::{Read, Seek, SeekFrom};

    let Ok(mut file) = std::fs::File::open(path) else {
        return false;
    };
    if file.seek(SeekFrom::End(-1)).is_err() {
        // A zero-length file cannot be seeked to -1, and has no torn tail.
        return false;
    }
    let mut last = [0u8; 1];
    match file.read_exact(&mut last) {
        Ok(()) => last[0] != b'\n',
        Err(_) => false,
    }
}

/// How many attempts the ledger accounts for.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
struct Tally {
    used_24h: u32,
    used_run: u32,
}

/// Add one, without ever wrapping.
///
/// **Saturating rather than wrapping is the whole point.** A `u32` that wraps
/// turns an implausibly long ledger into a count of zero, and a count of zero is
/// a permit. Saturation makes the impossible case refuse instead, and it is
/// asserted at this function rather than by writing four billion lines.
fn saturating_bump(count: u32) -> u32 {
    count.saturating_add(1)
}

/// Count the ledger's lines against the window and the run.
///
/// **A line this function cannot parse is COUNTED, not skipped**, and that
/// inverts the disposition `crate::journal::reader` takes for the run journal.
/// The inversion is deliberate and the reason is the consumer, not the format:
/// a malformed line in a *journal* is skipped because dropping one observation
/// is cheaper than failing a read, while a malformed line in a *cap ledger* is
/// an attempt that demonstrably happened — forgetting it under-counts, and
/// under-counting is the one failure this file exists to prevent.
///
/// An unparseable line counts toward the **window** only, never toward the
/// per-run bound. A line whose run cannot be read cannot be attributed to one,
/// and attributing it to every run would make a single torn write refuse the
/// first pull request of every run forever — a permanent denial of service
/// wearing the costume of caution. Counting it in the window is bounded: it
/// expires after 24 hours, exactly like the attempt it stands for.
fn tally(bytes: &[u8], now: i64, run_id: &str) -> Tally {
    let mut tally = Tally::default();

    for raw in bytes.split(|byte| *byte == b'\n') {
        if raw.is_empty() {
            continue;
        }
        // A non-UTF-8 fragment is an attempt that happened and cannot be read.
        let Ok(text) = std::str::from_utf8(raw) else {
            tally.used_24h = saturating_bump(tally.used_24h);
            continue;
        };
        if text.trim().is_empty() {
            continue;
        }
        // A torn final line — the one a killed writer leaves behind.
        let Ok(entry) = serde_json::from_str::<LedgerEntry>(text) else {
            tally.used_24h = saturating_bump(tally.used_24h);
            continue;
        };
        // The per-run bound is deliberately NOT time-scoped: a run is a bounded
        // thing, and letting its slots expire mid-run would silently turn the
        // per-run cap into a second rolling-window cap.
        if entry.run_id == run_id {
            tally.used_run = saturating_bump(tally.used_run);
        }
        let Some(at) = parse_stamp(&entry.at) else {
            tally.used_24h = saturating_bump(tally.used_24h);
            continue;
        };
        // Integer seconds, inclusive at the boundary. A stamp in the future
        // yields a negative difference and is inside the window, which is the
        // over-counting direction and therefore the right one.
        if now.saturating_sub(at) <= WINDOW_SECS {
            tally.used_24h = saturating_bump(tally.used_24h);
        }
    }

    tally
}

/// One RFC3339 stamp as whole seconds since the epoch, or `None`.
///
/// `timestamp()` is integer seconds by construction, which is how the whole
/// decision stays free of floating-point arithmetic: there is no sub-second
/// component to round, so two stamps written a millisecond apart compare as
/// equal rather than as an ordering nobody specified.
fn parse_stamp(stamp: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(stamp)
        .ok()
        .map(|parsed| parsed.timestamp())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{Duration, SecondsFormat, Utc};

    fn policy(cap_24h: u32, cap_run: u32) -> EnvelopePolicy {
        EnvelopePolicy {
            namespace: "refs/heads/gsd-auto/alpha/".to_string(),
            pr_cap_per_24h: cap_24h,
            pr_cap_per_run: cap_run,
            credential: None,
        }
    }

    fn entry(at: &str, run_id: &str) -> LedgerEntry {
        LedgerEntry {
            at: at.to_string(),
            run_id: run_id.to_string(),
            command: "gh pr create".to_string(),
            platform: "github".to_string(),
        }
    }

    /// A stamp `secs` seconds before the fixed reference `now` below.
    fn ago(secs: i64) -> String {
        (Utc::now() - Duration::seconds(secs)).to_rfc3339_opts(SecondsFormat::Secs, true)
    }

    fn now_stamp() -> String {
        Utc::now().to_rfc3339_opts(SecondsFormat::Secs, true)
    }

    #[test]
    fn the_first_attempt_in_an_empty_ledger_is_permitted() {
        let root = tempfile::tempdir().unwrap();
        let verdict =
            record_and_check_in(root.path(), "alpha", &entry(&now_stamp(), "run-1"), &policy(3, 1))
                .unwrap();
        assert_eq!(
            verdict,
            CapVerdict::Permit {
                used_24h: 1,
                used_run: 1
            }
        );
    }

    #[test]
    fn a_second_attempt_in_the_same_run_is_refused_even_though_the_window_has_capacity() {
        let root = tempfile::tempdir().unwrap();
        let policy = policy(3, 1);
        record_and_check_in(root.path(), "alpha", &entry(&now_stamp(), "run-1"), &policy).unwrap();
        let verdict =
            record_and_check_in(root.path(), "alpha", &entry(&now_stamp(), "run-1"), &policy)
                .unwrap();

        match verdict {
            CapVerdict::Refuse {
                reason,
                used_24h,
                used_run,
                ..
            } => {
                assert_eq!(reason, ParkReason::PrCapExceeded);
                assert_eq!(used_run, 2, "the per-run bound is what was exceeded");
                assert_eq!(used_24h, 2, "and the window still had capacity for it");
            }
            other => panic!("the per-run cap is a separate bound: {other:?}"),
        }
    }

    #[test]
    fn three_distinct_runs_are_permitted_in_the_window_and_the_fourth_is_refused() {
        let root = tempfile::tempdir().unwrap();
        let policy = policy(3, 1);

        for (index, run) in ["run-1", "run-2", "run-3"].iter().enumerate() {
            let verdict =
                record_and_check_in(root.path(), "alpha", &entry(&now_stamp(), run), &policy)
                    .unwrap();
            assert_eq!(
                verdict,
                CapVerdict::Permit {
                    used_24h: index as u32 + 1,
                    used_run: 1
                },
                "attempt {} of three must be permitted — an envelope that refuses \
                 everything passes every refusal criterion",
                index + 1
            );
        }

        let verdict =
            record_and_check_in(root.path(), "alpha", &entry(&now_stamp(), "run-4"), &policy)
                .unwrap();
        assert!(
            matches!(
                verdict,
                CapVerdict::Refuse {
                    reason: ParkReason::PrCapExceeded,
                    used_24h: 4,
                    ..
                }
            ),
            "the fourth attempt in the window is refused: {verdict:?}"
        );
    }

    #[test]
    fn an_entry_one_second_inside_the_window_counts_toward_it() {
        let root = tempfile::tempdir().unwrap();
        let policy = policy(1, 9);
        record_and_check_in(root.path(), "alpha", &entry(&ago(WINDOW_SECS - 1), "old"), &policy)
            .unwrap();

        let verdict =
            record_and_check_in(root.path(), "alpha", &entry(&now_stamp(), "new"), &policy)
                .unwrap();
        assert!(
            matches!(verdict, CapVerdict::Refuse { used_24h: 2, .. }),
            "23h59m59s ago is inside the window: {verdict:?}"
        );
    }

    #[test]
    fn an_entry_exactly_at_the_boundary_counts_because_the_window_is_inclusive() {
        let root = tempfile::tempdir().unwrap();
        let policy = policy(1, 9);
        record_and_check_in(root.path(), "alpha", &entry(&ago(WINDOW_SECS), "old"), &policy)
            .unwrap();

        let verdict =
            record_and_check_in(root.path(), "alpha", &entry(&now_stamp(), "new"), &policy)
                .unwrap();
        assert!(
            matches!(verdict, CapVerdict::Refuse { used_24h: 2, .. }),
            "exactly 24h is INSIDE the window — the boundary is inclusive, which is the \
             over-counting direction: {verdict:?}"
        );
    }

    #[test]
    fn an_entry_one_second_past_the_boundary_does_not_count() {
        let root = tempfile::tempdir().unwrap();
        let policy = policy(1, 9);
        record_and_check_in(root.path(), "alpha", &entry(&ago(WINDOW_SECS + 1), "old"), &policy)
            .unwrap();

        let verdict =
            record_and_check_in(root.path(), "alpha", &entry(&now_stamp(), "new"), &policy)
                .unwrap();
        assert_eq!(
            verdict,
            CapVerdict::Permit {
                used_24h: 1,
                used_run: 1
            },
            "24h00m01s ago has fallen out of the window; a cap that never releases a \
             slot is not a rolling window"
        );
    }

    #[test]
    fn a_torn_final_line_is_counted_toward_the_cap_rather_than_skipped() {
        let root = tempfile::tempdir().unwrap();
        let policy = policy(1, 9);
        let path = ledger_path_in(root.path(), "alpha").unwrap();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        // A writer killed mid-record: valid JSON up to the cut, no newline.
        std::fs::write(&path, br#"{"at":"2026-08-18T10:00:00Z","run_id":"run-0","comm"#).unwrap();

        let verdict =
            record_and_check_in(root.path(), "alpha", &entry(&now_stamp(), "new"), &policy)
                .unwrap();
        assert!(
            matches!(verdict, CapVerdict::Refuse { used_24h: 2, .. }),
            "a line the reader cannot parse is an attempt that happened; skipping it \
             under-counts, which is the failure this ledger exists to prevent: {verdict:?}"
        );
    }

    #[test]
    fn appending_onto_a_torn_tail_does_not_fuse_the_two_records_into_one() {
        let root = tempfile::tempdir().unwrap();
        let path = ledger_path_in(root.path(), "alpha").unwrap();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(&path, br#"{"at":"2026-08-18T10:00:00Z","run_id":"run-0","comm"#).unwrap();

        record_and_check_in(root.path(), "alpha", &entry(&now_stamp(), "new"), &policy(9, 9))
            .unwrap();

        let text = std::fs::read_to_string(&path).unwrap();
        let lines: Vec<&str> = text.lines().filter(|l| !l.trim().is_empty()).collect();
        assert_eq!(
            lines.len(),
            2,
            "a torn record and the record appended after it must stay two lines; fused \
             they parse as one, which counts ONE attempt where two happened: {text}"
        );
        assert!(serde_json::from_str::<LedgerEntry>(lines[1]).is_ok());
    }

    #[test]
    fn a_line_whose_timestamp_cannot_be_parsed_is_counted_toward_the_cap() {
        let root = tempfile::tempdir().unwrap();
        let policy = policy(1, 9);
        let path = ledger_path_in(root.path(), "alpha").unwrap();
        std::fs::create_dir_all(path.parent().unwrap()).unwrap();
        std::fs::write(
            &path,
            b"{\"at\":\"yesterday afternoon\",\"run_id\":\"run-0\",\"command\":\"gh pr create\",\
              \"platform\":\"github\"}\n",
        )
        .unwrap();

        let verdict =
            record_and_check_in(root.path(), "alpha", &entry(&now_stamp(), "new"), &policy)
                .unwrap();
        assert!(
            matches!(verdict, CapVerdict::Refuse { used_24h: 2, .. }),
            "an unparseable timestamp cannot be shown to be outside the window, so it \
             counts: {verdict:?}"
        );
    }

    #[test]
    fn the_running_count_saturates_rather_than_wrapping() {
        assert_eq!(
            saturating_bump(u32::MAX),
            u32::MAX,
            "a wrapping counter turns an implausibly long ledger into a count of zero, \
             and a count of zero is a permit"
        );
        assert_eq!(saturating_bump(0), 1);
    }

    #[test]
    fn the_ledger_sits_under_the_envelope_and_carries_no_component_of_the_repository() {
        let envelope_root = tempfile::tempdir().unwrap();
        let repo_root = tempfile::tempdir().unwrap();

        let path = ledger_path_in(envelope_root.path(), "alpha").unwrap();
        assert!(
            path.starts_with(envelope_root.path()),
            "{} is not under the envelope root",
            path.display()
        );
        assert!(
            !path.starts_with(repo_root.path()),
            "a cap the driven agent can delete with its own file tools is not a cap: {}",
            path.display()
        );
        assert_eq!(path.file_name().unwrap(), LEDGER_FILE);
    }

    #[test]
    fn a_hostile_alias_has_no_ledger_at_all() {
        for hostile in ["", ".", "..", "../escaped", "a/b", "/etc/passwd"] {
            assert!(ledger_path_in(Path::new("/data/envelope"), hostile).is_none());
        }
    }

    #[test]
    fn the_refused_attempt_is_on_disk_because_the_entry_is_written_before_it_is_permitted() {
        let root = tempfile::tempdir().unwrap();
        let policy = policy(1, 9);
        record_and_check_in(root.path(), "alpha", &entry(&now_stamp(), "run-1"), &policy).unwrap();

        let refused = entry(&now_stamp(), "run-2");
        let verdict = record_and_check_in(root.path(), "alpha", &refused, &policy).unwrap();
        assert!(matches!(verdict, CapVerdict::Refuse { .. }));

        let text = std::fs::read_to_string(ledger_path_in(root.path(), "alpha").unwrap()).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        assert_eq!(
            lines.len(),
            2,
            "the refused attempt must be recorded too (D-20): {text}"
        );
        assert!(
            lines[1].contains("run-2"),
            "the refused attempt is the one on the second line: {text}"
        );
    }

    #[test]
    fn the_refusal_states_the_over_count_bias_rather_than_leaving_it_to_be_discovered() {
        let refusal = CapVerdict::Refuse {
            reason: ParkReason::PrCapExceeded,
            used_24h: 4,
            cap_24h: 3,
            used_run: 1,
            cap_run: 1,
        };
        let detail = refusal.refusal_detail().expect("a refusal has a detail");
        assert!(detail.contains("over-count"), "{detail}");
        assert!(detail.contains("BEFORE it is permitted"), "{detail}");
        assert!(detail.contains("rolling 24-hour cap"), "{detail}");
        assert_eq!(
            CapVerdict::Permit {
                used_24h: 1,
                used_run: 1
            }
            .refusal_detail(),
            None
        );
    }

    #[test]
    fn a_refusal_names_the_per_run_bound_when_that_is_the_one_that_was_exceeded() {
        let refusal = CapVerdict::Refuse {
            reason: ParkReason::PrCapExceeded,
            used_24h: 2,
            cap_24h: 3,
            used_run: 2,
            cap_run: 1,
        };
        let detail = refusal.refusal_detail().unwrap();
        assert!(
            detail.contains("per-run cap"),
            "waiting does not clear a per-run bound, so the two must be distinguishable: \
             {detail}"
        );
    }

    #[test]
    fn an_attempt_whose_own_timestamp_is_unparseable_is_an_error_rather_than_a_guess() {
        let root = tempfile::tempdir().unwrap();
        let bad = entry("not a timestamp", "run-1");
        assert!(
            record_and_check_in(root.path(), "alpha", &bad, &policy(3, 1)).is_err(),
            "no window can be computed from an unparseable origin, and the guard turns \
             this error into a deny"
        );
    }
}
