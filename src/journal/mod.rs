//! The run journal: a durable, redacted, cheap-to-read record of one driver run.
//!
//! This module root carries the whole domain surface every later plan builds
//! against, so nothing downstream has to reopen it (D-35). The three submodules
//! divide cleanly: `redact` owns the capture-path filter and the [`RedactedLine`]
//! seam, `writer` owns the append-only NDJSON file, and `reader` owns the
//! byte-offset tail and the tolerant record parse.
//!
//! [`RedactedLine`]: redact::RedactedLine
//!
//! Five facts govern every type below. Each was settled by executed measurement
//! or by a decision recorded in `16-CONTEXT.md`, not inferred:
//!
//! 1. **`journal.jsonl` is append-only NDJSON (D-03).** One JSON object per
//!    line, one trailing newline, never a blank line. Every record carries `ts`,
//!    a **monotonic** `seq` starting at 1, and `kind`. `seq` exists so a tailing
//!    reader can detect gaps; a gap is reported, never treated as a parse
//!    failure.
//! 2. **There is no per-event durability syscall, and that is correct rather
//!    than an oversight (D-04).** RESEARCH §3 measured 8 of 8 SIGKILL runs with
//!    zero torn lines and zero `seq` gaps: a `write(2)` that has returned has
//!    handed its bytes to the kernel page cache, and process death cannot revoke
//!    pages the kernel already owns. Forcing the page cache to stable storage
//!    per event would buy survival of *machine* failure — which is not what
//!    OBS-01's success criterion asks for — at a cost that would dominate a
//!    write path firing every few seconds for hours.
//! 3. **Redaction happens in the capture path and is enforced by a type
//!    (D-21, D-22).** The writer accepts only
//!    [`RedactedLine`](redact::RedactedLine), whose sole constructor runs the
//!    redactor. There is no compilable path from a bare `String` to a written
//!    journal line, so the seam is held by the compiler and never by review
//!    discipline.
//! 4. **Parsing is tolerant by construction and never fatal (D-30).** Serde's
//!    strict unknown-field rejection attribute (`deny_unknown_fields`) is never
//!    opted into anywhere under `src/`, and **that absence is now enforced by a
//!    test rather than asserted by this paragraph**:
//!    `tests/spawn_seam_guard.rs::no_executable_line_in_src_opts_into_strict_unknown_field_rejection`
//!    walks every non-comment line in the tree and fails if the attribute
//!    appears. Phase 16 claimed a grep guard that was never written; Phase 17
//!    wrote it. Phase 20 is a known future emitter of new `kind` values, so the
//!    reader's `kind` stays a plain `String`.
//! 5. **Nothing in this tree logs event content (D-28).** Any `tracing` call
//!    inside `src/journal/` carries counts, paths that have already been through
//!    the redactor, or no event content at all. `src/executor/claude.rs:1260-1266`
//!    records the same discipline for the executor and names this phase as the
//!    one that makes it real.
//!
//! # What this phase deliberately does not do
//!
//! Four questions arrive at this module looking like they belong here. None
//! does, and each has a named owner, so a later reader can stop searching
//! instead of reopening this module to answer something it never held:
//!
//! - **Detached spawn, PID liveness, and crash *reconciliation* are Phase
//!   17's.** This phase writes the facts reconciliation reads — the pid and
//!   pgid in [`RunRecord`], and the absent [`RunRecord::ended_at`] that marks a
//!   run that never reached a terminal transition (D-06, D-32) — and it does no
//!   reconciling. Nothing here spawns a process; [`JournalRun`] is driven by a
//!   caller that hands it events.
//! - **The driver tab, the live stream, and the interjection channel are Phase
//!   18's.** Phase 16 ships **no UI** (D-36). The byte-offset tail in [`reader`]
//!   exists to keep watching *cheap*, not to display anything, and
//!   [`JournalEvent::Interjected`] is schema only until that phase exists.
//! - **The pre-push secret scan and the tool-boundary denial are Phase 19's.**
//!   [`redact`] is the *capture-path* half of SAFE-04 only. The complementary
//!   control fires **before** a secret enters context; neither substitutes for
//!   the other (D-25).
//! - **The decision router and a run's own bounds are Phase 20's.** The
//!   `observed`, `decided` and `parked` kinds are in the schema so that phase
//!   adds no migration, and [`RESERVED_KINDS`] states mechanically that this
//!   phase does not emit them (D-36).

pub mod inbox;
pub mod reader;
pub mod redact;
pub mod writer;

use std::path::{Component, Path, PathBuf};
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::executor::stream_json::{ResultMessage, StreamMessage, SystemMessage, TurnMessage};
use crate::executor::ExecutionEvent;

/// Where a project's run directories live, relative to its `.planning/` (D-01).
///
/// `meta-manager/` is an **existing** app namespace rather than a new one:
/// `src/state_reader/queue_md.rs:178` already owns
/// `.planning/meta-manager/QUEUE.md` and already writes it into third-party
/// repositories, so writing here inherits an established precedent and needs no
/// separate justification.
pub const RUNS_SUBDIR: &str = "meta-manager/runs";

/// Upper bound on any single string leaf inside one journal event (D-31).
///
/// 8 KiB. Phase 15's `tests/fixtures/transcripts/README.md` records base64
/// thinking-block signatures at **4-8 KB** and names them as the bulk of three
/// of its eight fixtures, so this admits one full signature and truncates a
/// pathological one. It is a defensible starting value with **no tuning data
/// behind it**; it is a named constant so tuning is a one-line change.
pub const MAX_EVENT_PAYLOAD_BYTES: usize = 8 * 1024;

/// Upper bound on one run's `journal.jsonl` before content events stop (D-31).
///
/// 64 MiB. At roughly 1 event/sec a four-hour run is ~14 400 events; at the
/// realistic ~1 KB average that is ~14 MB, and at the capped ~8 KiB worst case
/// ~115 MB. 64 MiB sits above the realistic case and below the pathological
/// one, which is where a circuit breaker belongs. Also untuned; see
/// [`RETAIN_RUNS`] for the disk budget it pairs with.
pub const MAX_RUN_JOURNAL_BYTES: u64 = 64 * 1024 * 1024;

/// How many complete runs are retained per project (D-32).
///
/// Ten. This constant and [`MAX_RUN_JOURNAL_BYTES`] are **one budget, not
/// two**: at the 64 MiB worst case they imply a 640 MB per-project ceiling.
/// Tuning either without the other moves that ceiling silently. Untuned
/// starting value.
pub const RETAIN_RUNS: usize = 10;

/// Upper bound on the bytes one tail read will consume (D-13).
///
/// 4 MiB — deliberately a wide margin above [`MAX_EVENT_PAYLOAD_BYTES`], because
/// a single tail legitimately batches many events. Given the per-event cap this
/// bound is unreachable in practice; the branch that handles it is pure defence
/// against a corrupted file, and without it a line that can never complete would
/// stall the cursor forever and make every later event invisible.
///
/// Like the three growth constants above it, this is a defensible starting
/// value with **no tuning data behind it** — it is a named constant so tuning is
/// a one-line change.
pub const MAX_TAIL_BYTES: u64 = 4 * 1024 * 1024;

/// Why an injected message can never be delivered: the agent's stdin was
/// already closed when the message reached the driver (D-10).
///
/// One of the two values [`JournalEvent::InterjectionMissed::reason`] may take,
/// and **fixed sentences rather than ones composed at the call site** — the
/// render layer selects a pinned gloss by matching on the exact string, so a
/// composed reason would land in the unrecognised branch and lose the
/// explanation the user needs.
///
/// They live here, beside the event that carries them, rather than in
/// [`crate::driver::run`] which writes them: that module is `#[cfg(unix)]` and
/// the render layer that must recognise them is not.
pub const MISSED_AFTER_CLOSE: &str =
    "the agent's stdin was already closed when this message reached the driver";

/// Why an injected message can never be delivered: the write to the agent's
/// stdin **failed** (D-10, CR-03).
///
/// The other terminal reason, and the one the four-state display was missing.
/// A message whose `Executor::send` returned `Err` was journaled
/// `interjected { delivered: false }` and nothing further; the cursor had
/// already moved past it, so it could never be re-read, never retried, and
/// never swept as missed. The render layer left it in `queued` — *"durably on
/// disk; nothing has read it yet"* — which is false twice over: the driver did
/// read it, and the write did fail. That is PITFALLS' undelivered-injection
/// failure inside the code written to prevent it.
pub const MISSED_SEND_FAILED: &str =
    "the write to the agent's stdin failed, so this message never reached it";

/// The six paths that make up one run's on-disk footprint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunPaths {
    /// `<planning>/meta-manager/runs/<run-id>/`.
    pub dir: PathBuf,
    /// The committed run record (D-06, D-07).
    pub run_json: PathBuf,
    /// The gitignored append-only journal.
    pub journal: PathBuf,
    /// The gitignored append-only **inbox**: the TUI→driver channel (D-03,
    /// D-05).
    ///
    /// **The one primitive the TUI and the detached driver share is the
    /// filesystem.** `ExecutionHandle::stdin_tx` lives inside the driver
    /// process, so the TUI cannot write to the agent's stdin at all; it appends
    /// a line here and the driver tails it. A design in which the TUI "sent"
    /// anywhere other than to a file could not survive a TUI restart and
    /// therefore could not satisfy STEER-03.
    ///
    /// No `.gitignore` change is needed for it: [`writer::RUNS_GITIGNORE_BODY`]
    /// re-includes exactly `!*/run.json`, so every other per-run file — this one
    /// included — is ignored by the catch-all (asserted by
    /// `tests/journal_gitignore.rs`).
    pub inbox: PathBuf,
    /// `<planning>/meta-manager/runs/.gitignore`, shared by every run (D-08).
    pub gitignore: PathBuf,
    /// `<planning>/meta-manager/runs/active`, the cheap-to-poll run-id pointer.
    pub active: PathBuf,
}

/// The root of a project's run directories.
pub fn runs_root(planning_dir: &Path) -> PathBuf {
    planning_dir.join("meta-manager").join("runs")
}

/// Whether `value` names **exactly one plain path component** (D-27, WR-02).
///
/// Non-empty, `Path::new(value).components()` yields exactly one item, that
/// item is a [`Component::Normal`], and its text is the whole of `value`.
///
/// **The property is general, and it now has two consumers** (D-03):
///
/// - a **run id**, joined under `<planning>/meta-manager/runs/`, and
/// - a **registry alias**, joined under the envelope directory by
///   [`crate::envelope::envelope_dir`].
///
/// Both are user-typed strings interpolated into a filesystem path, which is the
/// whole of what this predicate is about — the name says the property rather
/// than the first caller that needed it.
///
/// **The failure it exists to prevent was reproduced, not imagined.**
/// `--run-id '../../../../escaped'` created `run.json` and `journal.jsonl`
/// outside the project, in a directory with no `.gitignore`, **with exit 0**
/// (WR-02). This subsystem runs unattended with git and push rights.
///
/// **The alternative D-03 declined: a second, alias-specific validator.**
/// Keeping this one named for run ids and adding another for aliases is exactly
/// how the next caller escapes validation — the argument [`run_paths`]' own
/// signature already makes about infallible variants. Two predicates for one
/// property is one predicate plus a hole. So the predicate was **promoted** and
/// every caller swept in the same commit; the compiler enumerated the set.
///
/// **The final equality is not redundant belt-and-braces.** `components()`
/// silently normalises a leading `./` and a trailing `/` away, so `"./escape"`
/// and `"escape/"` both yield one `Normal` — comparing the component back
/// against the original string is what refuses a value whose written form is not
/// the plain name it resolves to.
///
/// Modelled on [`classify_change`], which already rejects every non-`Normal`
/// component and, like this, **refuses to normalise**: normalising means asking
/// the filesystem, and asking the filesystem is precisely what a traversal check
/// must not depend on. `..` is rejected as a *token*, not resolved.
///
/// The alternative that was declined: canonicalising the joined path and
/// checking it is a descendant of the runs root. That reads the filesystem
/// (so it answers differently for a path that does not exist yet, which is
/// every new run), it follows symlinks (so a symlink planted by the driven
/// agent decides the answer), and it cannot run on the `notify` callback
/// thread. A token check has none of those properties.
pub fn is_plain_path_component(value: &str) -> bool {
    if value.is_empty() {
        return false;
    }
    let mut components = Path::new(value).components();
    let Some(Component::Normal(name)) = components.next() else {
        return false;
    };
    if components.next().is_some() {
        return false;
    }
    name == std::ffi::OsStr::new(value)
}

/// The six paths for one run, or `None` for a run id that is not a plain name.
///
/// The layout's rationale lives here rather than at the call sites, following
/// the `queue_md.rs:170-176` precedent of putting the *why* of a path layout on
/// the path helper:
///
/// - `meta-manager/` is an existing app namespace (D-01); see [`RUNS_SUBDIR`].
/// - `run.json` is **committed** and everything else under `runs/` is ignored
///   (D-07). The committed record is the only artifact that survives a fresh
///   clone, which is what keeps the originating goal legible later; the journal
///   is a local transcript and has no business in anyone's history.
/// - The ignore file is a sibling of the run directories, not a child of one,
///   because it is written once at run-directory creation time and protects
///   every run (D-08). A log written before its protection lands is exactly the
///   class of mistake SAFE-04 exists to prevent.
/// - `inbox.jsonl` is the TUI→driver channel (D-03, D-05); see
///   [`RunPaths::inbox`].
///
/// **The `Option` is the whole of the WR-02 fix, and its shape was chosen so a
/// human does not have to find the call sites** (D-27). This function used to
/// `join` an unvalidated `run_id`, and both directions were *reproduced* against
/// the shipped tree: `--run-id '../../../../escaped'` created `run.json` and
/// `journal.jsonl` outside the project, in a directory with no `.gitignore` and
/// with exit 0; and `writer::read_active_run` returned whatever the `active`
/// file held — a file that lives inside the driven project, so **the agent
/// controls it**. This subsystem runs unattended with git and push rights.
///
/// A validation helper that merely *existed* would have been called at three of
/// the five sites, which is the failure mode this signature forecloses:
/// returning `Option` conscripts the compiler into enumerating every caller.
/// No infallible variant is kept alongside it, because keeping one is exactly
/// how the next caller escapes validation.
pub fn run_paths(planning_dir: &Path, run_id: &str) -> Option<RunPaths> {
    if !is_plain_path_component(run_id) {
        return None;
    }
    let root = runs_root(planning_dir);
    let dir = root.join(run_id);
    Some(RunPaths {
        run_json: dir.join("run.json"),
        journal: dir.join("journal.jsonl"),
        inbox: dir.join(inbox::INBOX_FILE),
        gitignore: root.join(".gitignore"),
        active: root.join("active"),
        dir,
    })
}

/// Build a run id of the form `2026-07-28T14-03-11Z-a3f9` (D-02).
///
/// RFC3339 UTC to second precision with every `:` replaced by `-` (colons are
/// legal on Linux but hostile on other filesystems and in shell paths), then a
/// `-` and the first four characters of the run's session UUID in its simple,
/// undashed hex form.
///
/// **The load-bearing property is that lexicographic sort equals chronological
/// sort.** That is what makes retention pruning (D-32) and "find the newest run"
/// a directory listing rather than a parse of every `run.json`. A format whose
/// ordering had to be recovered by parsing would make both operations O(runs)
/// file reads instead of one `read_dir`.
pub fn new_run_id(now: chrono::DateTime<chrono::Utc>, session_uuid: &uuid::Uuid) -> String {
    let stamp = now
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
        .replace(':', "-");
    let simple = session_uuid.simple().to_string();
    format!("{stamp}-{}", &simple[..4])
}

/// One run's list row, built from `run.json` and **nothing else** (OBS-05).
///
/// Every field here comes from the one committed per-run artifact, which is
/// what makes enumerating a project's runs cost one `read_dir` plus one small
/// read per run rather than a journal parse per run. A project holding the full
/// [`RETAIN_RUNS`] history would otherwise pay up to ten
/// [`MAX_RUN_JOURNAL_BYTES`]-bounded parses to draw a list.
///
/// **`outcome` is evidence, never prose.** It is [`RunRecord::outcome`], which
/// the driver derives from exit codes, envelope verdict fields and a disk
/// snapshot; nothing that renders a status word from a [`RunSummary`] may reach
/// for the agent's own summary of what it did (D-13).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunSummary {
    /// The run id, which is also the directory name.
    pub run_id: String,
    /// RFC3339 UTC timestamp of the run's first write.
    pub started_at: String,
    /// RFC3339 UTC timestamp of the terminal transition.
    ///
    /// **`None` means the run never reached one** — it is either live or it
    /// crashed, and telling those apart is a liveness probe's job, not this
    /// function's (D-06, D-32).
    pub ended_at: Option<String>,
    /// The originating goal prompt, verbatim (OBS-03). Empty when none was given.
    pub goal: String,
    /// The GSD command the run was started with.
    pub gsd_command: String,
    /// The derived run outcome, rendered. `None` until the terminal write.
    pub outcome: Option<String>,
}

/// Every run on disk for one project, newest first (OBS-05).
///
/// **This is blocking filesystem work — one `read_dir` and one small read per
/// run — and every caller is required to invoke it on
/// [`tokio::task::spawn_blocking`]** (D-28). That requirement is not
/// theoretical: `tests/driver_lock.rs:201-215` records a blocking call inside an
/// `async fn` defeating `tokio::time::timeout` on a current-thread runtime in
/// this very repository, and on the TUI side the same mistake is a frozen frame
/// rather than an error. No async wrapper is offered here on purpose — this
/// module has no runtime dependency today and should not gain one to enforce a
/// rule the call site is the right place to apply.
///
/// **It never reads a journal.** Neither [`reader::read_all`] nor
/// [`reader::tail_lines`] is reachable from here; a list row that cost a journal
/// parse would make entering the driver tab slow in exact proportion to how much
/// the project has been driven, which is backwards.
///
/// Skipping rules, and why each is not silent:
///
/// - A directory entry whose name is not a single plain path component is
///   refused by [`run_paths`] before anything is joined or read (D-27, WR-02).
/// - A run directory whose `run.json` is missing or unparseable is skipped with
///   a `tracing::warn!` naming the run id and the error **kind** — never file
///   content, per this module's logging rule. It is a real anomaly on a tree
///   this tool owns, so it must not vanish; it also must not abort the listing,
///   because one damaged record would then hide every healthy run beside it.
/// - A missing runs root is not an anomaly at all: it is a project that has
///   never been driven. It yields an empty list and logs nothing.
pub fn list_runs(planning_dir: &Path) -> Vec<RunSummary> {
    let root = runs_root(planning_dir);
    let entries = match std::fs::read_dir(&root) {
        Ok(entries) => entries,
        // Never driven: an empty list is the honest answer, not a warning.
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Vec::new(),
        Err(error) => {
            tracing::warn!(
                "Failed to list the runs root at {}: {:?}",
                root.display(),
                error.kind()
            );
            return Vec::new();
        }
    };

    let mut runs: Vec<RunSummary> = Vec::new();
    for entry in entries.filter_map(Result::ok) {
        if !entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
            continue;
        }
        // A non-UTF-8 name cannot be a run id: `new_run_id`'s format is ASCII by
        // construction. Converted with `into_string` rather than
        // `to_string_lossy` deliberately — a lossy conversion would hand a name
        // that is not the name on disk to the path join below.
        let Ok(name) = entry.file_name().into_string() else {
            continue;
        };
        if let Some(summary) = read_run_summary(planning_dir, &name) {
            runs.push(summary);
        }
    }

    sort_run_summaries_newest_first(&mut runs);
    runs
}

/// The outcome label of the most recent run on disk that **ended** (WR-02).
///
/// D-14 names a finished run whose outcome is `permission_denied`, `failed`,
/// `stalled` or `timed_out` as one of four evidence sources for the
/// needs-a-human badge, and the reconciliation scan cannot supply it:
/// `reconcile_one` returns `None` for an ended run, because there is nothing
/// left to *observe*. So the fact has to be read from the run list, where it has
/// been on disk all along as `RunRecord.outcome`.
///
/// **Newest ended, not newest.** A live or crashed run has no outcome to report,
/// and skipping past it to the newest run that does is what makes the answer a
/// fact rather than an absence. `needs_human` independently suppresses this arm
/// while a run is live, so the two rules cannot disagree in the direction that
/// summons a user to a project that is busy.
///
/// **Blocking filesystem work**, exactly like [`list_runs`] which it delegates
/// to — one `read_dir` and one small read per run, bounded by [`RETAIN_RUNS`] —
/// and every caller is required to invoke it on `tokio::task::spawn_blocking`
/// (D-28). It goes through `list_runs` rather than reaching for the newest
/// directory itself so there is exactly one traversal-refusal path (D-27,
/// WR-02) and one sort order.
pub fn last_ended_outcome(planning_dir: &Path) -> Option<String> {
    list_runs(planning_dir)
        .into_iter()
        .find(|run| run.ended_at.is_some())
        .and_then(|run| run.outcome)
}

/// One run's summary, or `None` for an id, a record or a document this cannot
/// safely read.
///
/// Split out of [`list_runs`] so both halves are testable: the traversal refusal
/// can be exercised against a real planted file **outside** the runs root, which
/// a `read_dir` walk can never produce and therefore can never prove.
fn read_run_summary(planning_dir: &Path, run_id: &str) -> Option<RunSummary> {
    // The fallible join is the whole traversal guard, and it comes first: a
    // hostile id is refused before any path is touched (D-27).
    let paths = run_paths(planning_dir, run_id)?;

    let raw = match std::fs::read_to_string(&paths.run_json) {
        Ok(raw) => raw,
        Err(error) => {
            tracing::warn!(
                "Skipping run {run_id}: run.json is unreadable ({:?})",
                error.kind()
            );
            return None;
        }
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&raw) else {
        tracing::warn!("Skipping run {run_id}: run.json did not parse as JSON");
        return None;
    };

    Some(run_summary_from_value(run_id, &value))
}

/// The field extraction, over a `Value` rather than through [`RunRecord`].
///
/// The same tolerance `writer::has_end_timestamp` already applies on the write
/// path (D-30): a record written by a schema this build has never seen must
/// still list, and every field this row needs is optional to it. Going through
/// [`RunRecord`] would make one added required field turn every older run
/// invisible in the UI.
fn run_summary_from_value(run_id: &str, value: &serde_json::Value) -> RunSummary {
    let string_field = |key: &str| -> String {
        value
            .get(key)
            .and_then(|field| field.as_str())
            .unwrap_or_default()
            .to_string()
    };
    let optional_field = |key: &str| -> Option<String> {
        value
            .get(key)
            .filter(|field| !field.is_null())
            .and_then(|field| field.as_str())
            .map(|field| field.to_string())
    };

    RunSummary {
        run_id: run_id.to_string(),
        started_at: string_field("started_at"),
        ended_at: optional_field("ended_at"),
        goal: string_field("goal"),
        gsd_command: string_field("gsd_command"),
        outcome: optional_field("outcome"),
    }
}

/// Sort run summaries **newest first**: lexicographic order on `run_id`,
/// descending.
///
/// **Lexicographic order is chronological order, and that is load-bearing
/// rather than a happy accident.** [`new_run_id`] produces
/// `2026-07-28T14-03-11Z-a3f9` — a fixed-width, zero-padded, UTC RFC3339 stamp
/// with a random suffix that only breaks ties — so byte order *is* time order,
/// and `run_id_sorts_lexicographically_in_chronological_order` pins it. This is
/// the same property on-disk retention (`writer::prune_runs`) and
/// `App::prune_driver_maps` already rely on to pick the newest runs without
/// opening a single file.
///
/// **The declined alternative is parsing `started_at` and sorting on the
/// timestamp.** It is worse twice over: it costs a parse per row for an ordering
/// the id already carries, and it *disagrees* with the id whenever a clock moves
/// — leaving the run list, the pruner and the map bound sorting a project's runs
/// three different ways.
pub fn sort_run_summaries_newest_first(runs: &mut [RunSummary]) {
    runs.sort_by(|left, right| right.run_id.cmp(&left.run_id));
}

/// A non-cryptographic identity digest of a spawned command line.
///
/// FNV-1a 64 over `argv` joined by the ASCII unit separator, rendered as
/// `fnv1a64:` plus 16 lowercase hex digits. Two facts about it are deliberate:
///
/// - **It is not a security control.** It exists so two runs can be compared for
///   "same command line" in a `run.json`, nothing more. Nothing authenticates,
///   authorises or trusts anything on the strength of this value.
/// - **It stays inline even though a hashing crate is now present.** Phase 21
///   added `sha2` to `Cargo.toml` for the driver opt-in's re-confirmation
///   digest, so the second half of this bullet's original claim — that no
///   hashing crate existed and none was warranted — stopped being true. This
///   function deliberately does **not** switch: a command-line identity
///   fingerprint has no adversary, and rewriting it would churn every
///   `fnv1a64:` value already sitting in a `run.json` for no gain. The function
///   that does need collision resistance is [`sha256_digest`], and it is a
///   sibling rather than a replacement so the two cannot be confused.
pub fn argv_digest(argv: &[String]) -> String {
    const OFFSET_BASIS: u64 = 0xcbf2_9ce4_8422_2325;
    const PRIME: u64 = 0x0000_0100_0000_01b3;

    let joined = argv.join("\u{1f}");
    let mut hash = OFFSET_BASIS;
    for byte in joined.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(PRIME);
    }
    format!("fnv1a64:{hash:016x}")
}

/// A collision-resistant digest of arbitrary bytes, rendered as `sha256:` plus
/// 64 lowercase hex digits.
///
/// **A sibling of [`argv_digest`], never a replacement.** The two exist for
/// opposite reasons and the prefixes are what keep them apart on disk:
///
/// - `argv_digest` answers "same command line?" for a human reading a
///   `run.json`. It has no adversary and says so.
/// - This one backs the driver opt-in's re-confirmation prompt, which **is** a
///   security affordance. The tool drives other people's cloned repositories,
///   whose `CLAUDE.md` can change under a `git pull` the user never read, so
///   the question "did these bytes change?" has to survive someone who wants
///   the answer to be no.
///
/// The prefix is not decoration. A recorded digest is compared as a whole
/// string, so a legacy `fnv1a64:` value can never accidentally compare equal to
/// a freshly computed `sha256:` one — the upgrade cannot be defeated by leaving
/// an old record in place, and no user's `config.json` needs a migration.
/// The hex rendering is written out by hand rather than through a `hex` crate:
/// `sha2` 0.11 returns a `hybrid_array::Array`, which — unlike 0.10's
/// `GenericArray` — does not implement `LowerHex`, and one `write!` per byte is
/// a smaller thing to own than another dependency.
pub fn sha256_digest(bytes: &[u8]) -> String {
    use sha2::{Digest, Sha256};
    use std::fmt::Write as _;

    let mut hasher = Sha256::new();
    hasher.update(bytes);

    let mut out = String::from("sha256:");
    for byte in hasher.finalize() {
        // `write!` to a String is infallible; the result is discarded rather
        // than unwrapped so this cannot panic on a path with no error to report.
        let _ = write!(out, "{byte:02x}");
    }
    out
}

/// The token recorded for a disclosed prompt input that did not exist.
///
/// A word rather than an omission, for `driver::escalate::UNNAMED`'s reason: a
/// digest built by simply leaving absent files out could not tell "the file was
/// not there" from "the file was not disclosed", and one of those is drift.
pub const ABSENT_INPUT_DIGEST: &str = "absent";

/// The identity of an approval: a digest over the approved plan **and** the
/// disclosed files that will enter the prompts.
///
/// # Why both halves, and why one digest over the two
///
/// Research Q4 asks that an approval bind to the plan *and* the disclosed files
/// and be re-checked at spawn. **An approval that does not cover the bytes that
/// will actually enter the prompt is not an approval of what will actually
/// run** — the plan can be identical while `CLAUDE.md` has been rewritten under
/// a `git pull` the user never read, and this tool drives other people's cloned
/// repositories, so that is the ordinary case rather than the exotic one.
///
/// One digest over both rather than two compared separately, because two
/// comparisons are two places a caller can check one and forget the other. The
/// *reason* a re-check failed is still recoverable, because
/// [`ApprovedPlan`] records the plan digest alongside this one and
/// [`recheck_approval`] compares that first — so "the plan changed" and "the
/// files changed" stay distinguishable to the person reading the refusal.
///
/// **How they stay distinguishable, concretely, because the paragraph above
/// used to promise it without a mechanism.** The plan half travels *with* the
/// approval, on argv, inside the token [`render_approval_token`] builds and
/// [`parse_approval_token`] reads. It is therefore the digest the approval
/// actually covered, and [`recheck_approval`] compares it against the digest of
/// the plan under test. An earlier build re-derived the "recorded" plan digest
/// from the plan under test and compared it against itself, which made
/// [`ApprovalRefusal::PlanChanged`] unreachable and reported every real
/// mismatch as a file change — telling the operator their `CLAUDE.md` moved on
/// the exact path where the model simply answered differently.
///
/// **SHA-256 on BOTH halves, and the "both" is the load-bearing word.** An
/// approval is exactly the affordance an adversary wants to defeat, so the outer
/// digest is the collision-resistant one — but an outer SHA-256 does not rescue a
/// weak inner digest, and an earlier version of this doc claimed it did.
///
/// That claim was wrong, and the reason it was wrong is worth keeping rather than
/// deleting: **hashing a weak digest under a strong one preserves the weak
/// digest's collision class exactly.** Two colliding inner values produce
/// byte-identical input to the outer hash, so the outer hash cannot distinguish
/// what the inner one already conflated. An inner FNV-1a-64 would have been a
/// second-preimage the attacker *constructs* rather than searches — the FNV round
/// is invertible mod 2^64 — over a token stream containing a `target_phase` that
/// a hostile `ROADMAP.md` authors.
///
/// So all three digests here are SHA-256: the plan half via
/// [`crate::driver::goal::plan_digest`], the file half via each
/// [`crate::config::PromptInput`]'s recorded digest from
/// `registry::current_prompt_inputs`, and the composition via this function.
/// [`argv_digest`] is not in this composition at all.
///
/// The file entries are **sorted**, so the digest is a fact about the disclosed
/// *set* rather than about the order a particular build happened to enumerate
/// it in.
pub fn approval_digest(plan_digest: &str, prompt_inputs: &[crate::config::PromptInput]) -> String {
    let mut lines = vec![format!("plan={plan_digest}")];
    let mut files: Vec<String> = prompt_inputs
        .iter()
        .map(|input| {
            format!(
                "file={}={}",
                input.path,
                input.digest.as_deref().unwrap_or(ABSENT_INPUT_DIGEST)
            )
        })
        .collect();
    files.sort();
    lines.extend(files);
    sha256_digest(lines.join("\n").as_bytes())
}

/// What joins the two halves of an approval token on argv.
///
/// **Why this character.** Both halves are `prefix:hexdigits` values produced by
/// [`sha256_digest`], so `+` occurs in neither half and
/// [`parse_approval_token`]'s split is unambiguous rather than heuristic. It
/// also needs no shell quoting, which matters because the value is copied out of
/// a refusal message and pasted onto a command line — a separator a shell
/// expanded would turn a correct paste into a malformed token.
///
/// **This is a user-visible CLI contract, not an implementation detail.**
/// Changing it is a breaking change on exactly the terms
/// [`crate::driver::dry_run::SECTION_REFSPECS`]'s doc establishes for the
/// preview's pinned headers: a token a user holds from an earlier build stops
/// parsing. The fail-closed direction is the saving grace — an old value is
/// refused by name rather than being read as a partial approval — but it is
/// still a break, and it belongs in the same commit as whatever falsified it.
pub const APPROVAL_TOKEN_SEPARATOR: &str = "+";

/// The single value a reviewer copies back onto argv: the plan they approved and
/// the disclosed files that approval covered, in one token.
///
/// **The only place the two halves are joined.** The refusal that *prints* a
/// token ([`crate::error::DriveError::PlanApprovalRequired`]) and the flag that
/// *accepts* one both route through here, so they cannot disagree about the
/// order — and the order is load-bearing, because both halves are `sha256:`
/// strings and a reversed pair would compare cleanly against the wrong things.
///
/// One flag rather than two, deliberately: with two flags "one supplied, the
/// other not" is a state the code has to classify anyway, and a reviewer has two
/// values to transcribe. With one token, a value that does not carry both halves
/// is simply malformed and the parse *is* the check.
pub fn render_approval_token(plan_digest: &str, approval_digest: &str) -> String {
    format!("{plan_digest}{APPROVAL_TOKEN_SEPARATOR}{approval_digest}")
}

/// Read a token back into its plan half and its approval half.
///
/// **An approval that cannot be parsed is an ABSENT approval.** Nothing here
/// repairs, trims or tolerates: a value with surrounding whitespace is the
/// caller's to fix, and silently normalising it would mean two spellings of one
/// approval whose digests then disagree about which was recorded. Every
/// malformed shape returns a named [`ApprovalTokenError`] and no shape returns a
/// half-approval.
pub fn parse_approval_token(raw: &str) -> Result<(String, String), ApprovalTokenError> {
    let mut halves = raw.split(APPROVAL_TOKEN_SEPARATOR);
    // `split` on a non-empty pattern always yields at least one element, so the
    // first `next` cannot be `None`; it is matched rather than unwrapped because
    // a panic on a parse of third-party argv is not a refusal.
    let (Some(plan), Some(approval)) = (halves.next(), halves.next()) else {
        return Err(ApprovalTokenError::SeparatorAbsent);
    };
    if halves.next().is_some() {
        // Three or more values concatenated must never be read as two: the
        // second half would silently be a prefix of what the caller meant.
        return Err(ApprovalTokenError::SeparatorRepeated);
    }
    if plan.is_empty() || approval.is_empty() {
        return Err(ApprovalTokenError::HalfEmpty);
    }
    Ok((plan.to_string(), approval.to_string()))
}

/// Why an `--approved-plan` value could not be read as a token at all.
///
/// A sibling taxonomy of [`ApprovalRefusal`], and deliberately **not** an arm of
/// it: "this token is malformed" and "this approval no longer covers what would
/// run" are different statements to the person reading the refusal, in exactly
/// the way [`ApprovalRefusal::Absent`] is different from the other two. A
/// malformed token is also not an absent one — an absent approval is a run
/// nobody approved, a malformed one is a run somebody tried to approve and got
/// wrong, and only the second is worth telling them how to re-transcribe.
///
/// No wildcard anywhere it is matched, so a fourth shape has to be given a
/// message by whoever adds it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ApprovalTokenError {
    /// No separator at all — including a legacy value carrying only one digest.
    SeparatorAbsent,
    /// More than one separator, so the value carries more than two halves.
    SeparatorRepeated,
    /// The separator is present but one of the halves is empty.
    HalfEmpty,
}

impl std::fmt::Display for ApprovalTokenError {
    /// Every arm names the flag **and** how to obtain a good value, because a
    /// refusal a caller cannot act on is a bug report rather than an error
    /// message ([`ApprovalRefusal::Absent`]'s register, copied deliberately).
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        // The offending value is never echoed. It arrives on argv from whoever
        // launched the run, and the caller already has it; printing it back
        // would put an unbounded third-party string on the terminal of the
        // person reading a refusal, which is the capability WR-05 closed
        // elsewhere in this phase.
        match self {
            ApprovalTokenError::SeparatorAbsent => write!(
                f,
                "the `--approved-plan` value carries no `{APPROVAL_TOKEN_SEPARATOR}`, \
                 so it is not an approval token. A token names BOTH halves — the plan \
                 that was approved and the disclosed files that approval covered — \
                 joined by `{APPROVAL_TOKEN_SEPARATOR}`. A value carrying one half is \
                 not half an approval; re-run this invocation WITHOUT \
                 `--approved-plan` to see the plan and the token that authorises it"
            ),
            ApprovalTokenError::SeparatorRepeated => write!(
                f,
                "the `--approved-plan` value carries more than one \
                 `{APPROVAL_TOKEN_SEPARATOR}`, so it is not two halves and is refused \
                 rather than read as the first two. Re-run this invocation WITHOUT \
                 `--approved-plan` to see the plan and the single token that \
                 authorises it"
            ),
            ApprovalTokenError::HalfEmpty => write!(
                f,
                "the `--approved-plan` value has an empty half, which is what a token \
                 truncated at a copy or a shell boundary looks like. An approval that \
                 cannot be parsed is an absent approval, so nothing here is treated as \
                 partially approved; re-run this invocation WITHOUT `--approved-plan` \
                 to see the plan and the whole token that authorises it"
            ),
        }
    }
}

/// The approval recorded on a run record: what was approved, and its identity.
///
/// **Approval is an explicit recorded act, never an inferred consent and never
/// a timeout-to-yes.** The absence of one of these on a goal-driven run is a
/// refusal with its own message ([`ApprovalRefusal::Absent`]), not a default
/// yes — a run that started because nobody said no is a run nobody approved.
///
/// It is a record of a decision rather than the decision itself: the human act
/// is supplying [`approval_digest`]'s value on argv, having seen the plan it
/// identifies. This type is what makes that act durable and re-checkable.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ApprovedPlan {
    /// The plan's steps, as typed `key=value` tokens in plan order.
    ///
    /// **Never a command line and never the model's prose.** Each entry is built
    /// from a validated `RouterAction`'s own verb, a roadmap-declared phase and
    /// a reduced terminal state; the seam's free-text `rationale` is excluded,
    /// because a record whose one-line-per-entry shape every reader depends on
    /// is the last place model-authored prose belongs.
    pub steps: Vec<String>,
    /// The phase the plan's **terminal** step targets — the phase the run drives
    /// toward. Never the first step's: a plan is an ordered traversal that may
    /// pass through prerequisites.
    pub target_phase: String,
    /// [`crate::driver::goal::plan_digest`] of the approved plan: a
    /// `sha256:`-prefixed digest of its step tokens in plan order.
    ///
    /// **A control rather than a hint.** It is SHA-256, so an attacker who
    /// authors a project's `ROADMAP.md` — and therefore the `target_phase`
    /// tokens hashed into it — cannot construct a second plan that shares this
    /// value with the one the user reviewed. It is recorded *separately* from
    /// [`Self::approval_digest`] so a failed re-check can name **which half**
    /// moved, the plan or the disclosed files, rather than only that something
    /// did.
    ///
    /// A record written before this became SHA-256 carries the legacy
    /// `fnv1a64:` prefix. Nothing migrates it and nothing needs to:
    /// [`recheck_approval`] compares the whole prefixed string, so the prefixes
    /// differ, the record re-checks as [`ApprovalRefusal::PlanChanged`], and the
    /// upgrade fails closed. That is what the prefixes are for, and
    /// `tests/driver_goal_seam.rs::a_recorded_approval_carrying_a_legacy_
    /// fnv1a64_plan_digest_re_checks_as_stale` proves it rather than leaving it
    /// as a claim in this paragraph.
    pub plan_digest: String,
    /// [`approval_digest`] over the plan digest and the disclosed prompt inputs
    /// as they stood when the approval was given.
    pub approval_digest: String,
    /// RFC3339 UTC timestamp of the recorded approval.
    pub approved_at: String,
    /// Every field of this record that this build does not model.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// Why a recorded approval no longer covers what would run.
///
/// Three arms, and the split between the first and the other two is the one the
/// tests turn on: **"nobody approved this" and "what was approved has changed"
/// are different statements to the person reading the refusal**, and a single
/// "mismatch" for both would report an unapproved run as a stale approval.
///
/// **[`Self::DisclosedFilesChanged`]'s message opens by asserting the plan is
/// unchanged, and that assertion is now backed by a comparison the code
/// actually performs.** It was not always: `driver::approve_plan` used to hand
/// [`recheck_approval`] the digest of the plan it had *just observed* as though
/// it were the recorded one, so the plan half compared equal by construction,
/// [`Self::PlanChanged`] was unreachable from production, and every real
/// mismatch fell through to this arm — announcing a file change to a user whose
/// files had not moved. The recorded plan digest now arrives from the caller's
/// token, so the sentence is true rather than aspirational.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ApprovalRefusal {
    /// No approval was recorded at all. Never a default yes.
    Absent,
    /// The plan changed since it was approved.
    PlanChanged {
        /// The plan digest the approval covered.
        approved: String,
        /// The plan digest observed now.
        observed: String,
    },
    /// The plan is identical but the disclosed files' bytes are not.
    DisclosedFilesChanged {
        /// The approval digest that was recorded.
        approved: String,
        /// The approval digest the current disclosed set produces.
        observed: String,
    },
}

impl std::fmt::Display for ApprovalRefusal {
    /// Each message names what happened **and** something the caller can do,
    /// because a refusal a caller cannot act on is a bug report rather than an
    /// error message (`driver::bounds::BoundsRefusal`'s rule).
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ApprovalRefusal::Absent => write!(
                f,
                "no plan approval was recorded for this run. Approval is an \
                 explicit act and never an inferred consent: run the same \
                 invocation with `--dry-run` to see the plan and the digest that \
                 identifies it, then pass that digest as `--approved-plan`"
            ),
            ApprovalRefusal::PlanChanged { approved, observed } => write!(
                f,
                "the plan this run decomposed ({observed}) is not the plan that \
                 was approved ({approved}). A model asked the same question twice \
                 may answer differently, and an approval covers one answer. \
                 Review the new plan with `--dry-run` and approve it explicitly"
            ),
            ApprovalRefusal::DisclosedFilesChanged { approved, observed } => write!(
                f,
                "the plan is unchanged but the disclosed files whose bytes reach \
                 a prompt are not: the approval covered {approved} and the files \
                 on disk now produce {observed}. An approval that does not cover \
                 the bytes that will enter the prompt is not an approval of what \
                 will actually run. Review and approve again"
            ),
        }
    }
}

/// Whether `recorded` still covers the plan and the files that would run.
///
/// **Pure, which is what makes both halves testable without a spawn.** The
/// caller reads the current prompt inputs and hands them in; nothing here opens
/// a file.
///
/// The plan half is compared **first**, so a run whose plan changed says so
/// rather than reporting the combined mismatch a file change would also
/// produce. Both refuse; the order is a legibility decision.
pub fn recheck_approval(
    recorded: Option<&ApprovedPlan>,
    plan_digest: &str,
    prompt_inputs: &[crate::config::PromptInput],
) -> Result<(), ApprovalRefusal> {
    let Some(recorded) = recorded else {
        return Err(ApprovalRefusal::Absent);
    };

    if recorded.plan_digest != plan_digest {
        return Err(ApprovalRefusal::PlanChanged {
            approved: recorded.plan_digest.clone(),
            observed: plan_digest.to_string(),
        });
    }

    let observed = approval_digest(plan_digest, prompt_inputs);
    if recorded.approval_digest != observed {
        return Err(ApprovalRefusal::DisclosedFilesChanged {
            approved: recorded.approval_digest.clone(),
            observed,
        });
    }

    Ok(())
}

/// What kind of write a watched path change represents (D-11).
///
/// `Hash` is load-bearing, not decorative: the watcher's debounce dedup becomes
/// per-`(root, classification)` rather than per-root, and this value is half of
/// that key (D-10).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ChangeKind {
    /// A write under `.planning/meta-manager/runs/<run-id>/` — the driver's own
    /// journal. Routes to a byte-offset tail, never to a full re-parse.
    DriverJournal {
        /// The run directory's name, which is the run id.
        run_id: String,
    },
    /// Everything else. Keeps today's re-parse behaviour.
    Planning,
}

/// Classify a changed path as a driver-journal write or a planning write.
///
/// **A pure function with no filesystem access (D-11).** It must not stat,
/// canonicalise or read anything, because it runs on the `notify` debouncer's
/// callback thread, which has no runtime and no `App`. Purity is also what lets
/// it be tested exhaustively against path shapes the way `extract_project_root`
/// already is at `src/watcher.rs:97-116`.
///
/// Two aspects of the rule are deliberate:
///
/// - **`Planning` is the default**, so any path shape this function does not
///   recognise keeps today's behaviour rather than silently losing a re-parse.
/// - **It anchors on the full three-component `.planning/meta-manager/runs/`
///   prefix, never on a bare `runs` component.** A project that happens to keep
///   `.planning/runs/` for its own purposes must not have its writes
///   misclassified as driver writes, because a misclassified planning write is a
///   dashboard that silently stops updating (RESEARCH Security Domain, tampering
///   row; T-16-07).
///
/// A path *directly* under `runs/` — the shared `.gitignore`, the `active`
/// pointer, or the run directory itself — is `Planning`: it names no run, so
/// there is no `run_id` to carry, and there is no journal to tail.
pub fn classify_change(project_root: &Path, changed_path: &Path) -> ChangeKind {
    let Ok(relative) = changed_path.strip_prefix(project_root) else {
        // Not under this project at all.
        return ChangeKind::Planning;
    };

    let mut parts: Vec<&str> = Vec::new();
    for component in relative.components() {
        match component {
            Component::Normal(name) => match name.to_str() {
                Some(name) => parts.push(name),
                // Non-UTF-8 component: unrecognised, so it takes the default.
                None => return ChangeKind::Planning,
            },
            // `..`, a root, or a prefix inside the relative remainder means the
            // path is not a plain descendant. Do not try to normalise it here —
            // normalising is exactly the filesystem access this function forbids.
            _ => return ChangeKind::Planning,
        }
    }

    if parts.len() >= 5
        && parts[0] == ".planning"
        && parts[1] == "meta-manager"
        && parts[2] == "runs"
    {
        return ChangeKind::DriverJournal {
            run_id: parts[3].to_string(),
        };
    }

    ChangeKind::Planning
}

/// One record in `journal.jsonl`, dispatched on the `kind` field (D-03).
///
/// **Struct variants only.** RESEARCH §9.1 verified that a tuple variant is a
/// compile error under an internally-tagged enum and that a newtype around a
/// scalar is a runtime serialisation error, so the shape is not a style choice.
///
/// This is the writer's *typed emission surface*. The reader deliberately does
/// not go through it — see [`reader::JournalRecord`], which keeps `kind` a plain
/// `String` so an unknown kind arrives with its payload intact (D-30).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum JournalEvent {
    /// The run began. Written once, immediately after `run.json`'s first write.
    RunStarted {
        /// The originating goal prompt, verbatim.
        goal: String,
        /// Whether this run was started in dry-run mode (Phase 17 populates it).
        dry_run: bool,
        /// The rendered [`ExecutionTarget`](crate::executor::ExecutionTarget).
        target: String,
    },
    /// What the driver observed about a project's state before deciding.
    ///
    /// **Schema only in this phase — Phase 20 emits it.** It is present now so
    /// that phase adds no schema migration (D-36).
    Observed {
        /// The phase number the observation is about.
        phase: String,
        /// The five D-R-P-E-V stage statuses, in order.
        drpev: Vec<String>,
    },
    /// What the driver decided to run next, and why.
    ///
    /// **Schema only in this phase — Phase 20 emits it** (D-36).
    Decided {
        /// `"policy"`, `"llm"`, or `"human"`.
        by: String,
        /// The GSD command the decision selected.
        command: String,
        /// Why this command, in one line.
        rationale: String,
    },
    /// An agent process was spawned.
    ExecStarted {
        /// The session UUID the CLI reported at `system/init`.
        session_id: String,
        /// [`argv_digest`] of the spawned command line.
        argv_digest: String,
        /// The `claude` process **group** id (D-09).
        ///
        /// `claude` is spawned as its own group leader, so its pgid is distinct
        /// from the driver's: a signal to the driver's group does not reach it.
        /// A driver killed with SIGKILL therefore skips its own teardown and
        /// would leave an **untraceable** `claude` tree behind — so the precise
        /// handle is journaled at the moment it becomes known.
        ///
        /// `Option` because [`from_exec_event`] is a pure per-event projection
        /// and `ExecutionEvent::SessionStarted` carries no pgid. The value is
        /// **stamped by the run**, exactly as [`JournalEvent::ExecFinished`]'s
        /// `cost_usd` and `duration_s` already are.
        claude_pgid: Option<u32>,
    },
    /// One observed thing on the agent's stream.
    ///
    /// **This is the field redaction earns its keep on.** `text` carries agent
    /// prose, tool results, and anything the agent read out of a config file or
    /// an environment dump on its way to a persistent local file (SAFE-04).
    ExecEvent {
        /// `"assistant"`, `"user"`, `"stderr"`, or another stream label.
        stream: String,
        /// The event's text, already through the redactor by the time it is
        /// written — see [`redact::RedactedLine`].
        text: String,
    },
    /// The running notional cost total as of the last turn boundary.
    ///
    /// Kept as a structured number rather than folded into a diagnostic string
    /// so a later reader can sum or chart it without parsing prose.
    Cost {
        /// `total_cost_usd`, which accumulates across turns.
        cumulative_usd: f64,
    },
    /// A message the user injected into a running agent — the **delivery**
    /// record (D-09, STEER-02).
    ///
    /// Written by the driver the moment it has tried to write the message to the
    /// agent's stdin, whether or not that write succeeded. `delivered` is
    /// therefore the answer to *"did `Executor::send` return `Ok`"* and nothing
    /// more: it is **not** an acknowledgement from the agent, which arrives
    /// later as [`JournalEvent::InterjectionActedOn`] and was measured 55
    /// seconds after the write. Rendering this as "received" or "read" would
    /// promise an observation the mechanism cannot make (D-07).
    Interjected {
        /// The client-generated correlation id from the inbox line (D-05).
        ///
        /// `Option` and `#[serde(default)]` because the variant shipped in Phase
        /// 16 without it, so a journal written by an older build still parses.
        /// **Text alone is not a correlation key** — a user may legitimately
        /// send the same sentence twice, and STEER-02's states are tracked per
        /// message rather than per string.
        #[serde(default)]
        id: Option<String>,
        /// The injected text.
        text: String,
        /// Whether it reached the agent, as opposed to being queued or dropped.
        delivered: bool,
    },
    /// The agent **dequeued** an injected message and is running it as its own
    /// turn (D-07, D-08, STEER-02).
    ///
    /// The `--replay-user-messages` echo, correlated by the driver — the only
    /// party that has parsed envelopes rather than a rendered projection.
    /// **Schema only in this plan; plan 18-02 emits it.**
    InterjectionActedOn {
        /// The correlation id of the message the echo matched.
        id: String,
    },
    /// An injected message that can never be delivered (D-10).
    ///
    /// The honest fourth state. A message appended after the driver closed the
    /// agent's stdin (D-11) has nowhere to go, and leaving it in `queued`
    /// forever is PITFALLS' undelivered-injection failure dressed up as a
    /// spinner. It is named, recorded, and **not retried**.
    InterjectionMissed {
        /// The correlation id of the message that will not be delivered.
        id: String,
        /// Why it cannot be delivered, in one line.
        reason: String,
    },
    /// The agent process exited.
    ExecFinished {
        /// The process exit code, or `None` if it died by signal.
        exit: Option<i32>,
        /// **Notional cost, not billed cost.** Phase 15-05 recorded this caveat
        /// and it is carried here verbatim rather than restated: the CLI reports
        /// a modelled figure, subscription runs are not billed per token, and
        /// nothing downstream should present this as an invoice.
        cost_usd: Option<f64>,
        /// Wall-clock duration of the run in whole seconds.
        duration_s: u64,
    },
    /// How many events Phase 15's bounded drain discarded (D-33).
    ///
    /// A run that silently lost 40 events is materially different from one that
    /// lost none, and this count is the only signal that distinguishes them.
    EventsDropped {
        /// Events discarded since the run began.
        count: u64,
    },
    /// The per-run byte cap was reached; content events stop here (D-31).
    ///
    /// Emitted exactly once. After it, only lifecycle, decision and outcome
    /// events are written — the run must always be able to write its terminal
    /// record, because silently dropping the ending is the one failure OBS-01
    /// cannot tolerate.
    JournalTruncated {
        /// Bytes written when the cap was reached.
        bytes_written: u64,
        /// The cap that was reached, so a reader need not guess the build's
        /// value of [`MAX_RUN_JOURNAL_BYTES`].
        cap: u64,
    },
    /// A non-fatal anomaly worth recording: a `seq` gap, a shrinking journal, a
    /// parent `.gitignore` that will keep `run.json` out of history.
    Diagnostic {
        /// A short stable identifier for the condition.
        code: String,
        /// One line of already-redacted detail.
        detail: String,
    },
    /// The run reached a terminal state. Always the last record.
    RunEnded {
        /// The rendered [`RunOutcome`](crate::executor::RunOutcome).
        outcome: String,
        /// RFC3339 UTC timestamp of the terminal transition.
        ended_at: String,
    },
    /// The run stopped short and needs a human.
    ///
    /// **Phase 19 is its first emitter** (D-24). The comment that stood here
    /// said "Schema only in this phase — Phase 20 emits it", and that stopped
    /// being true the moment the safety envelope started refusing operations:
    /// every envelope refusal parks the run, so the reason has to land somewhere
    /// a later reader can find it. Correcting the comment is part of the same
    /// change as the first emission, because a doc that describes the *previous*
    /// build is worse than no doc — it is read as current.
    ///
    /// **Phase 19 owns producing the reason; Phase 20 owns what happens to a
    /// parked run.** No resume, retry or backoff logic belongs beside this
    /// variant: a reason that also decided the recovery would have to be edited
    /// every time the recovery changed, and then the identifier is no longer
    /// stable.
    Parked {
        /// Why the run parked.
        ///
        /// A short stable identifier, in the same convention
        /// [`JournalEvent::Diagnostic`]'s `code` field documents. This string is
        /// always some taxonomy's `as_str()` and **never a fresh literal minted
        /// at the call site** — one list rather than two, so a reader who greps
        /// for `force_push_blocked` or `bounds_no_progress` finds the producer
        /// and the record together.
        ///
        /// **Five sanctioned taxonomies ride this one field, and naming only
        /// one of them would be the same quiet lie this record exists to
        /// prevent.** They are siblings, never extensions of each other:
        ///
        /// | Taxonomy | Prefix on disk | What it means |
        /// |---|---|---|
        /// | [`crate::envelope::policy::ParkReason`] | none (e.g. `force_push_blocked`) | The safety envelope refused an operation (Phase 19, D-24). **Seven arms, and it stays at seven** — a new detector is a new sibling enum, not an eighth arm here. |
        /// | `crate::driver::router::RouterReason` | `router_` **or** `gate_` | The deterministic router declined to choose a next command (Phase 20, DRIVE-06). Two prefixes on one enum, on purpose: `router_` marks a refusal about the rule table (no rule, unverified state, an unsatisfied dependency) and `gate_` a human-judgement gate the run reached and refused to answer (DRIVE-05). The split is what makes "how often does the always-park posture stop a run, and at which gate?" a grep over the journals rather than a memory. |
        /// | `crate::driver::bounds::BoundsReason` | `bounds_` | A run bound fired and the run halted itself before the next spawn (Phase 20, CTRL-06). |
        /// | `crate::driver::rate_limit::QuotaReason` | `quota_` | The transport reported a Claude subscription quota rejection and the run stopped without retrying (Phase 20, CTRL-07). Its own taxonomy rather than a fifth `BoundsReason` arm: the four bounds are facts about *this* run's budget, while a quota rejection is a fact about a budget shared with every other Claude surface the user has. |
        /// | `crate::driver::escalate::EscalationReason` | `escalation_` | The model seam's per-run budget was exhausted, or the seam's answer was refused (Phase 21, DRIVE-04). Its own taxonomy rather than a fifth `BoundsReason` arm, on the same axis the quota park is a sibling on: the four bounds are facts about *whether the run is making progress*, while an escalation count is a fact about *how much the model was consulted* — a run can burn its whole escalation budget while making excellent progress, and a stalled run can burn none. |
        ///
        /// The prefixes make the producer readable from the string alone, and
        /// they are a property of each enum's own `REASON_*` constants rather
        /// than something assembled here. All five reach a terminal record
        /// through the single `parked:` label prefix, so
        /// `run.json`'s `outcome` needs no second carrier and no second parse.
        reason: String,
        /// What would unpark it, in the same register as [`Self::reason`]: a
        /// short phrase naming the actor, not a sentence of advice.
        needs: String,
    },
}

impl JournalEvent {
    /// Whether this event carries run *content* as opposed to lifecycle facts.
    ///
    /// Plan 16-03 uses this to decide what survives the per-run byte cap: once
    /// [`MAX_RUN_JOURNAL_BYTES`] is breached, content stops and lifecycle,
    /// decision and outcome events continue, because D-31 is explicit that the
    /// run must always be able to write its terminal record.
    ///
    /// **[`JournalEvent::Interjected`] is content and the two interjection
    /// transitions are not**, and the split is deliberate rather than
    /// incidental: the delivery record carries the user's verbatim text, which
    /// is exactly the kind of arbitrarily large payload the cap exists to bound,
    /// while `interjection_acted_on` and `interjection_missed` carry only a
    /// correlation id and a fixed reason. Suppressing the *transitions* would
    /// strand a delivered message in `delivered` forever on a long run — the
    /// silent-abandonment failure the three-state display exists to prevent.
    pub fn is_content(&self) -> bool {
        matches!(
            self,
            JournalEvent::ExecEvent { .. } | JournalEvent::Interjected { .. }
        )
    }
}

/// The `run.json` document (D-06).
///
/// **This document is written exactly twice and nothing else ever rewrites it.**
/// Write one lands at run start, before any agent is spawned. Write two lands at
/// the terminal transition, after the last agent has exited. That immutability
/// is not tidiness — it is what makes committing the file safe against the Aider
/// garbage-commit hazard (D-07): the driven agent runs `git` inside the very
/// worktree that holds this file, so a record that changed on every status
/// update would leave the worktree perpetually dirty and let any `git add -A`
/// the agent issues sweep a mid-run snapshot into an unrelated commit.
///
/// **Forward constraint for Phase 20, now discharged:** when a run becomes a
/// multi-invocation loop, the terminal write must still happen after the *last*
/// agent invocation exits, never between steps. Per-step state belongs in the
/// journal, which is gitignored. Phase 20's iteration loop honours it — the
/// loop is strictly inside the two writes and adds neither a third nor a
/// per-iteration one.
///
/// # Every field states its scope, and that is a requirement rather than a
/// documentation habit
///
/// Phase 20 changed the driver's **unit of work from a command to a sequence**.
/// Every field below was singular because the unit of work was singular, so a
/// field left unclassified after that change is a field whose meaning moved
/// without saying so — a record that lies quietly to the separate process that
/// reads it (Phase 19 proved that reader exists). Each doc therefore names one
/// of two scopes explicitly:
///
/// * **Run-scoped** — one value for the whole run, true from the first write to
///   the last, no matter how many commands the run issued.
/// * **Iteration-scoped** — the value belongs to *one* command of a sequence.
///   No such field may stay on this record silently; either it is documented as
///   naming a specific iteration (as [`session_id`](Self::session_id) is), or
///   its per-iteration counterpart is named so a reader knows where the rest of
///   the truth lives.
///
/// `run_record_fields_all_declare_their_scope` parses this struct's own body out
/// of the source text and fails on a field declaration whose doc carries neither
/// word, so the classification cannot rot as fields are added.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunRecord {
    /// The run id, matching the directory name.
    ///
    /// **Run-scoped.** One directory per run, whatever the run issued.
    pub run_id: String,
    /// The originating goal prompt, verbatim — the reason D-07 commits this file.
    ///
    /// **Run-scoped.** The goal is what the *run* was started for; a routed run
    /// pursues one goal across every command it issues, and no iteration
    /// narrows it.
    pub goal: String,
    /// What the run issued, when that is one namable thing.
    ///
    /// **Run-scoped, and in routed mode it deliberately names no command at
    /// all.** In single-command mode this is the supplied `--command`, exactly
    /// as it was before Phase 20. In routed mode the run has no single command —
    /// that is the point of routing — so the field carries the marker
    /// [`crate::driver::ROUTED_RECORD_MARKER`] and the run's identity moves to
    /// [`target_phase`](Self::target_phase).
    ///
    /// **A marker rather than the goal text or an empty string, and both
    /// alternatives were rejected for a stated reason.** Recording the goal
    /// would put a non-command in a field whose name says command. Recording
    /// `""` would collide with a meaning this field already has: an absent
    /// field reads as empty on the tolerant path
    /// (`a_record_from_an_unknown_schema_still_produces_a_row`, D-30), so a
    /// routed run would be indistinguishable from a record this build could not
    /// parse. The marker is the only value that cannot be misread as either.
    ///
    /// The **sequence itself** is on the journal's `decided` records, one per
    /// iteration, naming each command in full and in order.
    pub gsd_command: String,
    /// The phase a routed run was driving toward, or `None` in command mode.
    ///
    /// **Run-scoped** — the target bounds the whole sequence and no iteration
    /// changes it; the *commands* chosen to reach it are the iteration-scoped
    /// part, and they are on the `decided` records.
    ///
    /// `Option` plus `#[serde(default)]` per the serde migration posture: a
    /// record written before Phase 20 has no such key and must still load, and
    /// `run.json` carries **no version discriminator** to hang a migration from,
    /// so tolerance on read is the only mechanism available.
    /// `a_pre_phase_20_record_still_loads_with_the_new_fields_defaulted` proves
    /// it against a byte literal of the old shape rather than against a record
    /// this build produced.
    #[serde(default)]
    pub target_phase: Option<String>,
    /// The run bounds actually in force, or `None` for a record written before
    /// they were recorded.
    ///
    /// **Run-scoped.** CTRL-06's caps bound the run as a whole; the per-iteration
    /// counter and elapsed clock they are compared against live in memory and
    /// reach disk as the `parked` reason when a detector fires.
    ///
    /// **The resolved value, never the constants.** A reader must be able to
    /// answer *"what was this run allowed to do?"* from the record, and a
    /// default written here would force them to infer it from which binary
    /// happened to run — which is exactly the inference a durable record exists
    /// to remove. An argv override is therefore what appears on disk.
    #[serde(default)]
    pub bounds: Option<RecordedBounds>,
    /// The plan the run's stated goal was decomposed into, and the approval that
    /// covers it. `None` for every run that was handed a `--command` or a
    /// `--target-phase` directly, which is every Phase 20 run.
    ///
    /// **Run-scoped.** The goal is set once, from a human, before the loop
    /// starts, and **no iteration narrows it** — the driver may never enqueue
    /// itself another goal from an artifact created during its own run, which is
    /// a type-level property of `driver::run::GoalDecomposition` rather than a
    /// promise made here. A run therefore has one plan or none, never a
    /// sequence of them, and this field is a value rather than a list for
    /// exactly that reason.
    ///
    /// **Approval is an explicit recorded act, never an inferred consent and
    /// never a timeout-to-yes.** A goal-driven run with no approval recorded here
    /// is refused before it starts, with its own message
    /// ([`ApprovalRefusal::Absent`]) rather than a mismatch; the absence of a
    /// record is a refusal and not a default yes.
    ///
    /// `Option` plus `#[serde(default)]` per the serde migration posture: a
    /// record written before Phase 21 has no such key and must still load, and
    /// `run.json` carries no version discriminator to hang a migration from.
    #[serde(default)]
    pub approved_plan: Option<ApprovedPlan>,
    /// How many model consultations this run was permitted, as resolved.
    ///
    /// **Run-scoped.** DRIVE-04's cap bounds the run as a whole — the
    /// decomposition above the loop and every ambiguity escalation inside it
    /// spend one budget between them — and no iteration has a cap of its own.
    ///
    /// **The resolved value, never the constant.** `escalate::resolve` refuses a
    /// supplied cap that could never bind and *reduces* an unsupplied default to
    /// what the step cap leaves room for, so a reader answering "how often could
    /// this run consult a model?" must not have to work out which binary produced
    /// the record and what its compiled-in default was. A one-step run records a
    /// cap of zero, which is a true statement about that run.
    #[serde(default)]
    pub escalation_cap: Option<u32>,
    /// How many model consultations this run actually spent, as a running total.
    ///
    /// **Run-scoped**, and a total rather than a per-iteration count for the
    /// same reason the cap is: the budget is the run's. The per-iteration
    /// evidence is the journal's `decided` records, whose `by` field reads `llm`
    /// for a command a seam named and `policy` for one the rule table chose.
    ///
    /// It **never under-reports**: the counter increments on the ask rather than
    /// on the answer, so a permitted consultation that then failed still counts.
    /// A number that reads as a safety property while under-reporting is worse
    /// than no number.
    ///
    /// It is stamped at write **two**, because the total is not known until the
    /// last iteration has run; write one records the cap alone.
    #[serde(default)]
    pub escalations_used: Option<u32>,
    /// The rendered execution target.
    ///
    /// **Run-scoped.** `ExecutionOptions::target` is rebuilt per iteration in a
    /// routed run, but from the same envelope settings every time, so the
    /// rendered value is invariant across the sequence.
    pub target: String,
    /// The user's driver opt-in record.
    ///
    /// **Run-scoped.** The opt-in authorises driving *this project*, not one
    /// command of a sequence.
    ///
    /// `Option` because populating it is **Phase 17's** job; this phase writes
    /// the field so that phase adds no schema migration.
    pub opt_in: Option<String>,
    /// RFC3339 UTC timestamp of the first write.
    ///
    /// **Run-scoped.** It stamps write one, which lands before the loop starts
    /// and before any agent is spawned. Per-iteration start times are the
    /// `exec_started` records' own timestamps.
    pub started_at: String,
    /// The agent session UUID of the **first** iteration, and of nothing else.
    ///
    /// **Iteration-scoped, and this field names iteration one.** It is written
    /// at the first write, before any command has run, so it can only ever be
    /// the first session's. It is *not* the run's session: Phase 20 starts a
    /// **fresh session per iteration** rather than resuming one (CONTEXT.md
    /// OQ5), so that a multi-hour routed run cannot hit a context limit for
    /// reasons unrelated to the work.
    ///
    /// The per-iteration ids ride the journal's **`exec_started`** events, one
    /// per spawn, each already carrying its own `session_id` — so the full set
    /// is recoverable without widening this record, which is why this stayed one
    /// field rather than becoming a list. (`decided` records the *command*
    /// chosen and its rationale; it carries no session id, because the decision
    /// precedes the spawn that creates one.)
    pub session_id: String,
    /// The driver process id.
    ///
    /// **Run-scoped.** One driver process performs the whole sequence; the
    /// agents it spawns are separate processes in their own group and are not
    /// recorded here.
    pub pid: u32,
    /// The driver process group id.
    ///
    /// **Run-scoped**, and the kill switch depends on it staying so: `kill`
    /// resolves a stop against this one group for the whole run.
    pub pgid: u32,
    /// The `claude_code_version` observed at `system/init`.
    ///
    /// **Iteration-scoped, naming iteration one**, for the same mechanical
    /// reason as [`session_id`](Self::session_id): it is empty at write one and
    /// nothing overwrites it, because the document is written exactly twice. In
    /// practice every iteration of a run spawns the same binary, so the value is
    /// invariant — but it is the *first* iteration's observation, and a reader
    /// wanting per-iteration proof reads the `exec_event` records.
    pub claude_code_version: String,
    /// [`argv_digest`] of the driver's own effective command line.
    ///
    /// **Run-scoped**, and deliberately the *driver's* argv rather than any
    /// agent's. For a routed run it digests the `--target-phase` invocation the
    /// user actually typed, which is what distinguishes two routed runs from one
    /// another; the per-iteration argv variation is on the `decided` records.
    /// The digest authenticates nothing (see [`argv_digest`]).
    pub argv_digest: String,
    /// RFC3339 UTC timestamp of the terminal transition.
    ///
    /// **Run-scoped.** It stamps write two, which lands after the **last**
    /// iteration exits and never between iterations.
    ///
    /// **`None` is precisely the signal Phase 17's crash reconciliation reads**
    /// (D-06, D-32): a run directory whose record has no `ended_at` is a run
    /// that never reached a terminal transition, and retention must never prune
    /// it.
    pub ended_at: Option<String>,
    /// The derived run outcome, rendered. `None` until the terminal write.
    ///
    /// **Run-scoped** — how the *run* ended, from the `Terminal` classification
    /// with no unclassified arm (DRIVE-06). A halted or parked run carries the
    /// `parked:<reason>` form, so the detector that stopped the sequence is
    /// readable here without a second read of the journal. Per-iteration
    /// outcomes are the `exec_finished` records.
    pub outcome: Option<String>,
    /// Every field of this record that this build does not model.
    ///
    /// **Run-scoped** in the trivial sense — it is the record's own overflow,
    /// not any iteration's.
    ///
    /// The same tolerance technique `JournalRecord.rest` and
    /// `RegisteredProject.extra` already use, applied to a file **two binary
    /// versions may share** and that users commit (T-20-08). Without it, an
    /// older binary deserialising a newer record and re-serialising it would
    /// silently delete every field it had never heard of — a destructive rewrite
    /// by omission. `#[serde(flatten)]` supplies default-when-absent for free: a
    /// record with no unknown fields deserialises to an empty map and serialises
    /// back to nothing at all.
    ///
    /// It complements rather than replaces the `Value`-based read paths
    /// ([`run_summary_from_value`], `reconcile`): those exist so one *added*
    /// field cannot make older runs invisible, and this exists so an *unknown*
    /// field survives a round trip through the typed struct.
    #[serde(flatten)]
    pub extra: serde_json::Map<String, serde_json::Value>,
}

/// The run bounds a run actually ran under, as written to `run.json`.
///
/// A record-side mirror of `crate::driver::bounds::RunBounds` rather than that
/// type itself, and the split is deliberate. `RunBounds` carries a
/// [`std::time::Duration`], which serde renders as a `{"secs":…,"nanos":…}`
/// object — a shape nothing reading this file wants and that a future change of
/// representation would silently alter. Seconds as a plain integer is what a
/// reader can act on, and keeping the mirror here means `journal` does not
/// depend on `driver` to describe its own document.
///
/// Both fields are **run-scoped**: they bound the sequence, not an iteration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecordedBounds {
    /// How many iterations the run was allowed, from
    /// `bounds::RunBounds::max_steps`.
    pub max_steps: u32,
    /// The run-level wall-clock cap in whole seconds, from
    /// `bounds::RunBounds::wall_clock_cap`.
    ///
    /// The run-level cap, not the per-iteration one: the two are different
    /// values on purpose (the iteration cap is strictly inside this one), and
    /// recording the wrong one would make the run-level wall-clock reason look
    /// unreachable to anyone checking the record against the detector.
    pub wall_clock_cap_secs: u64,
}

/// The `kind` values **this phase actually emits** (D-36).
///
/// D-36 requires Phase 16 to carry the kinds later phases will emit *and to say
/// which ones it can itself produce*. This constant is that statement, made
/// mechanical: [`RESERVED_KINDS`] is its complement, and
/// `every_reserved_kind_is_declared_and_none_is_emitted_by_this_phase` drives
/// every [`ExecutionEvent`] variant through [`from_exec_event`] to prove the
/// split is real rather than aspirational.
pub const EMITTED_KINDS: &[&str] = &[
    "run_started",
    "exec_started",
    "exec_event",
    "cost",
    "interjected",
    "interjection_acted_on",
    "interjection_missed",
    "exec_finished",
    "events_dropped",
    "journal_truncated",
    "diagnostic",
    "run_ended",
    // Phase 19's, moved out of RESERVED_KINDS in the same edit that gave it its
    // first emitter — every safety-envelope refusal parks the run (D-24).
    "parked",
    // Phase 20's two, moved out of RESERVED_KINDS in the same edit that gave
    // them their first emitter: the iteration loop journals what it observed and
    // what the deterministic router decided, once per routed iteration
    // (DRIVE-02, DRIVE-06). D-36 held: they were schema'd in Phase 16 and this
    // move needed no migration.
    "observed",
    "decided",
];

/// The `kind` values that exist in the schema but that **this phase never
/// writes** (D-36).
///
/// **Empty as of Phase 20, and an empty list here is a finished promise rather
/// than a missing one.** D-36's mechanism was that Phase 16 would model every
/// kind a later phase would emit, so no later phase would need a schema
/// migration; this list is what was owed, and every entry has now been drawn
/// down by the phase it was reserved for. `interjected` and its two transitions
/// left in **Phase 18** when the TUI→driver channel acquired a producer;
/// `parked` left in **Phase 19**, because the safety envelope refuses operations
/// and D-24 requires every refusal to park the run; `observed` and `decided`
/// left in **Phase 20**, when the iteration loop began journalling what it saw
/// and what the deterministic router chose. Not one of the four needed a
/// migration, which is the whole of what D-36 was buying.
///
/// It stays as a `pub const` rather than being deleted: the two lists are
/// complements, `every_reserved_kind_is_declared_and_none_is_emitted_by_this_phase`
/// is what proves it, and the next phase that wants to model a kind ahead of its
/// producer has the mechanism waiting rather than having to rebuild it. The
/// reader tolerates an unknown kind regardless (D-30): a build that has never
/// heard of one still carries its payload.
pub const RESERVED_KINDS: &[&str] = &[];

/// The one type a driver holds for the duration of a run (D-06, D-36).
///
/// It owns the run's paths, its open journal, its `run.json` document, the
/// instant the run began and the last cumulative cost the stream reported. One
/// `JournalRun` is one run, start to finish:
///
/// ```text
/// JournalRun::start(planning, record)   -> prune, mkdir+ignore, run.json #1, active, journal, run_started
/// JournalRun::record_exec(&event)       -> zero or more, one per observed ExecutionEvent
/// JournalRun::finish("completed")       -> run_ended, stamp, run.json #2, clear active
/// ```
///
/// **`run.json` is written exactly twice across that lifetime and nothing else
/// ever rewrites it** (D-06). [`Self::record_writes`] counts them and a debug
/// assertion in [`Self::finish`] pins the total at two, because the immutability
/// is what neutralises the Aider garbage-commit hazard (D-07) rather than being
/// mere tidiness.
///
/// **Phase 16 drives this from a synthetic event sequence.** Nothing in this
/// repository spawns a `ClaudeExecutor` yet; wiring a real one is Phase 17's
/// (D-36). The type is a library component ready for that producer, not a
/// half-built loop.
pub struct JournalRun {
    paths: RunPaths,
    writer: writer::JournalWriter,
    record: RunRecord,
    started: Instant,
    /// The last `total_cost_usd` the stream reported.
    ///
    /// **Notional cost, not billed cost.** Phase 15-05 recorded that caveat and
    /// it is carried rather than restated: the CLI reports a modelled figure,
    /// subscription runs are not billed per token, and nothing downstream
    /// should present this as an invoice.
    last_cost_usd: Option<f64>,
    /// The `claude` process group id, once the driver has one (D-09).
    ///
    /// Run-scoped rather than per-event, for the same reason the cost total is:
    /// no single [`ExecutionEvent`] carries it, and the run does.
    claude_pgid: Option<u32>,
    record_writes: usize,
}

impl JournalRun {
    /// Open a run: prune, create, record, point, journal, announce — in that
    /// order, and the order is the decision.
    ///
    /// 1. **Prune first** (D-32). Retention runs at run *start*, never at run
    ///    exit, because an exit-time prune is skipped by exactly the crash this
    ///    phase is built to survive. The new run's own id is passed as the
    ///    active one so a re-`start` on an existing id can never prune itself.
    /// 2. **Create the directory, which lands the ignore file** (D-08) before a
    ///    single journal byte exists. A log written before its protection lands
    ///    is the class of mistake SAFE-04 exists to prevent.
    /// 3. **Write `run.json` — write one of two** (D-06): the immutable facts,
    ///    landing before any agent is spawned.
    /// 4. **Write the `active` pointer**, which is cheap to poll and never
    ///    trusted over the directory listing.
    /// 5. **Open the journal** and append `run_started`.
    ///
    /// A failed prune does not stop a run from starting; [`writer::prune_runs`]
    /// already downgrades a removal failure to a warning, and the only `Err` it
    /// can return here is a runs-root listing failure, which is a real I/O
    /// fault worth surfacing.
    pub fn start(planning_dir: &Path, record: RunRecord) -> anyhow::Result<Self> {
        writer::prune_runs(planning_dir, RETAIN_RUNS, Some(&record.run_id))?;

        let paths = writer::create_run_dir(planning_dir, &record.run_id)?;
        writer::write_run_record(&paths, &record)?;
        writer::write_active_pointer(&runs_root(planning_dir), &record.run_id)?;

        let mut writer = writer::JournalWriter::open(&paths.journal)?;
        writer.append(&JournalEvent::RunStarted {
            goal: record.goal.clone(),
            // Dry-run exists as of plan 17-04, and the literal `false` is still
            // correct — for a different and now **permanent** reason. D-23
            // requires a preview to write nothing, so `drive()` renders its
            // report and returns before a run id is generated; a dry-run
            // therefore produces no journal at all and can never reach this
            // line. Every run that does reach it is a real one, by construction
            // rather than by "until it exists".
            dry_run: false,
            target: record.target.clone(),
        })?;

        Ok(Self {
            paths,
            writer,
            record,
            started: Instant::now(),
            last_cost_usd: None,
            claude_pgid: None,
            record_writes: 1,
        })
    }

    /// Record the `claude` process group id for this run (D-09).
    ///
    /// Call it the instant the spawn returns and **before** draining a single
    /// event: a teardown handle recorded late is a teardown handle that can be
    /// missed. It only affects `exec_started` records written after this call,
    /// which is why the ordering matters rather than being tidy.
    pub fn set_claude_pgid(&mut self, pgid: u32) {
        self.claude_pgid = Some(pgid);
    }

    /// Record how many model consultations this run spent (DRIVE-04).
    ///
    /// **It lands on write TWO and cannot land on write one**, because the total
    /// is not known until the last iteration has run — and `run.json` is written
    /// exactly twice, so there is no third write for it to arrive on. Call it
    /// immediately before [`finish`](Self::finish); the cap it is read against
    /// was stamped at write one.
    ///
    /// It mutates the in-memory record rather than appending an event, for the
    /// reason [`set_claude_pgid`](Self::set_claude_pgid) does: the field belongs
    /// to the record, and a per-iteration event carrying a running total would
    /// be N places a reader has to reconcile instead of one.
    pub fn set_escalations_used(&mut self, used: u32) {
        self.record.escalations_used = Some(used);
    }

    /// Close a run out: announce, stamp, record, unpoint.
    ///
    /// The `run_ended` event lands **before** the record write, so a crash
    /// between them leaves a journal that names the ending and a record that
    /// does not — which is exactly the shape Phase 17's reconciliation reads,
    /// rather than a record claiming an ending the journal never saw.
    ///
    /// **Forward constraint for Phase 20:** when a run becomes a
    /// multi-invocation loop, the terminal record write must still happen after
    /// the *last* agent invocation exits, never between steps. Per-step state
    /// belongs in the journal, which is ignored.
    pub fn finish(&mut self, outcome_label: &str) -> anyhow::Result<()> {
        let ended_at = chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true);

        self.writer.append(&JournalEvent::RunEnded {
            outcome: outcome_label.to_string(),
            ended_at: ended_at.clone(),
        })?;

        self.record.ended_at = Some(ended_at);
        self.record.outcome = Some(outcome_label.to_string());
        writer::write_run_record(&self.paths, &self.record)?;
        self.record_writes += 1;
        debug_assert_eq!(
            self.record_writes, 2,
            "run.json is written exactly twice and nothing else ever rewrites it (D-06)"
        );

        writer::clear_active_pointer(self.runs_root())?;
        Ok(())
    }

    /// Journal one observed [`ExecutionEvent`].
    ///
    /// Returns `Ok(())` for an event the mapping does not journal; see
    /// [`from_exec_event`] for why that arm exists and why nothing reaches it
    /// today.
    ///
    /// A cost update is *also* remembered here rather than only written, so the
    /// terminal `exec_finished` can carry the run's last cumulative figure
    /// without re-reading the file.
    pub fn record_exec(&mut self, ev: &ExecutionEvent) -> std::io::Result<()> {
        if let ExecutionEvent::Cost { cumulative_usd } = ev {
            self.last_cost_usd = Some(*cumulative_usd);
        }

        let Some(mut event) = from_exec_event(ev, &self.record.argv_digest) else {
            return Ok(());
        };

        // The run-scoped fields no single event can know. `from_exec_event` is a
        // pure per-event projection, so it leaves them empty and the run — which
        // owns the clock, the cost total and the spawned group's handle — stamps
        // them here.
        if let JournalEvent::ExecFinished {
            cost_usd,
            duration_s,
            ..
        } = &mut event
        {
            *cost_usd = self.last_cost_usd;
            *duration_s = self.started.elapsed().as_secs();
        }
        if let JournalEvent::ExecStarted { claude_pgid, .. } = &mut event {
            *claude_pgid = self.claude_pgid;
        }

        self.record(&event)
    }

    /// Append one journal event directly.
    ///
    /// The escape hatch for the events that are not projections of an executor
    /// event at all — a `Diagnostic` a caller raised, or Phase 20's decision
    /// records once it exists.
    ///
    /// The error type is `io::Error` rather than `anyhow::Error` because this is
    /// the per-event hot path: a caller draining a stream wants one cheap error
    /// type to react to, and the only thing that actually fails here is the
    /// write. The lifecycle calls, which are rare and want context, keep
    /// `anyhow`.
    pub fn record(&mut self, ev: &JournalEvent) -> std::io::Result<()> {
        self.writer
            .append(ev)
            .map(|_seq| ())
            .map_err(std::io::Error::other)
    }

    /// The five paths this run owns.
    pub fn paths(&self) -> &RunPaths {
        &self.paths
    }

    /// How many times `run.json` has been written. Exactly 2 after a
    /// started-then-finished run (D-06).
    pub fn record_writes(&self) -> usize {
        self.record_writes
    }

    /// The run record as it currently stands.
    pub fn record_document(&self) -> &RunRecord {
        &self.record
    }

    /// The runs root this run lives under, derived from its own paths.
    ///
    /// `gitignore` is `<root>/.gitignore` by construction ([`run_paths`]), so
    /// its parent is the root. The fallback is unreachable for any `RunPaths`
    /// this module built, and is inert if it ever were reached:
    /// [`writer::clear_active_pointer`] treats an absent pointer as success.
    fn runs_root(&self) -> &Path {
        self.paths
            .gitignore
            .parent()
            .unwrap_or_else(|| Path::new("."))
    }
}

/// The stream label for a parsed message: its variant name in snake case.
fn stream_label(message: &StreamMessage) -> &'static str {
    match message {
        StreamMessage::System(_) => "system",
        StreamMessage::Assistant(_) => "assistant",
        StreamMessage::User(_) => "user",
        StreamMessage::Result(_) => "result",
        StreamMessage::ControlResponse(_) => "control_response",
        StreamMessage::RateLimitEvent(_) => "rate_limit_event",
        StreamMessage::Unknown => "unknown",
    }
}

/// A readable text projection of one observed stream message (OBS-04).
///
/// **The agent's own words may be displayed as content and may never drive a
/// badge, a colour, a status word or a sort key.** That is D-13, and it is not
/// hypothetical: the incident it is drawn from is an agent that deleted a
/// production database during an explicit freeze, hid it, fabricated ~4000 fake
/// users and fake test results, and falsely claimed rollback was impossible.
/// Self-report is testimony, not telemetry. A run's status comes from
/// [`RunRecord::outcome`], [`JournalEvent::RunEnded`] and
/// [`JournalEvent::ExecFinished`]'s exit code — never from anything this
/// function returns. **Do not undo this by deriving anything from the string.**
///
/// **It deliberately does not sanitise, and that is a division of
/// responsibility rather than an omission.** Terminal-control stripping happens
/// at buffer-append time in the TUI, because `journal.jsonl` is *evidence* and
/// is read by tools other than that renderer: stripping the `ESC` bytes an
/// agent emitted at write time would destroy the record that it emitted them.
/// The renderer strips unconditionally before a byte reaches a display buffer.
/// Secret redaction is a separate control and already applies here, at the
/// [`redact::RedactedLine`] seam every journal write goes through.
///
/// Why this takes a whole [`StreamMessage`] rather than the [`TurnMessage`] it
/// mostly renders: the role is carried by the envelope's `type`, which *is* the
/// enum variant, so a turn alone cannot say whether it is the agent speaking or
/// the user. Every arm composes a real string; none falls back to a `Debug`
/// rendering, which is the whole point of the function.
fn exec_message_text(message: &StreamMessage) -> String {
    match message {
        StreamMessage::Assistant(turn) => compose_turn("assistant", turn),
        // The replay marker is protocol evidence off the parsed envelope, not
        // prose, so naming it here is honest. It is emitted at **dequeue**: the
        // correlation that turns it into an `acted-on` transition is the
        // driver's, from `is_replay`, never from this string (D-07, D-08).
        StreamMessage::User(turn) => {
            let role = if turn.is_replay {
                "user (replay)"
            } else {
                "user"
            };
            compose_turn(role, turn)
        }
        StreamMessage::System(SystemMessage::Init(init)) => format!(
            "system: init (claude {}, session {})",
            init.claude_code_version.as_deref().unwrap_or("unreported"),
            init.session_id.as_deref().unwrap_or("unreported"),
        ),
        StreamMessage::System(SystemMessage::Other) => {
            "system: a subtype this build does not model".to_string()
        }
        StreamMessage::Result(result) => turn_result_text(result),
        StreamMessage::ControlResponse(response) => format!(
            "control_response: {} (request {})",
            response.response.subtype, response.response.request_id
        ),
        // `Display` on a `serde_json::Value` is its JSON rendering — a real
        // string off the wire rather than a Rust struct dump.
        StreamMessage::RateLimitEvent(value) => format!("rate_limit_event: {value}"),
        StreamMessage::Unknown => "a message type this build does not model".to_string(),
    }
}

/// One turn as `role: text`, or the bare role when the turn rendered to nothing.
fn compose_turn(role: &str, turn: &TurnMessage) -> String {
    let text = turn.text_content();
    if text.is_empty() {
        format!("{role}:")
    } else {
        format!("{role}: {text}")
    }
}

/// A readable text projection of one `result` envelope — a **turn** boundary.
///
/// Composed from the facts the stream reported, in the spirit of the
/// `LineTruncated` arm below: the journal restates what the envelope said rather
/// than inventing a third rendering of it. The agent's own prose is appended as
/// content and carries no authority; see [`exec_message_text`] for the rule.
///
/// [`ResultMessage::result`] is an `Option` because it is **absent entirely on
/// error envelopes** — not empty, absent (Phase 15 D-32). Defaulting it to an
/// empty string would make an error envelope indistinguishable from a silent
/// success, so the absence is preserved by simply omitting the line.
fn turn_result_text(result: &ResultMessage) -> String {
    let subtype = if result.subtype.is_empty() {
        "no subtype reported"
    } else {
        result.subtype.as_str()
    };
    let mut text = format!("turn ended: {subtype}");
    if let Some(reason) = result.terminal_reason.as_deref() {
        text.push_str(&format!(" ({reason})"));
    }
    if result.is_error {
        text.push_str(" [error]");
    }
    if let Some(ms) = result.duration_ms {
        // Per-turn and resets (D-29), so it is labelled per-turn. A steered run
        // emits one `result` per turn.
        text.push_str(&format!("; {:.1}s this turn", ms as f64 / 1000.0));
    }
    if let Some(cost) = result.total_cost_usd {
        // Cumulative across turns, and **notional rather than billed** — the
        // caveat `JournalEvent::Cost` already carries, said where a human reads
        // it (D-12).
        text.push_str(&format!("; ${cost:.2} cumulative, notional"));
    }
    for error in &result.errors {
        text.push_str(&format!("\nerror: {error}"));
    }
    if let Some(prose) = result.result.as_deref() {
        if !prose.is_empty() {
            text.push('\n');
            text.push_str(prose);
        }
    }
    text
}

/// Project one [`ExecutionEvent`] onto the journal's vocabulary (D-36).
///
/// This is the whole of Phase 16's consumer wiring: the journal reads the
/// stream Phase 15 already produces and writes what it sees. Every field it
/// reaches for was **already recorded by Phase 15 for this purpose**, so no
/// executor signature is widened to make journalling possible (D-37) — the
/// session id comes off the event, the argv digest off the run record, the exit
/// code off the process status.
///
/// Two constraints on the projection, both deliberate:
///
/// 1. **No serialisation derive is added to [`StreamMessage`] or any other wire
///    type in order to journal it.** Phase 16 stored `format!("{message:?}")`
///    here for exactly that reason — a `Debug` rendering was the projection
///    available without widening a type it was told not to widen. The surface
///    that renders these strings now exists, so the projection is
///    [`exec_message_text`] and [`turn_result_text`]: readable prose composed
///    from a minimally-modelled message body. Reaching for
///    `#[derive(Serialize)]` here would still spend a permanent constraint on a
///    temporary convenience, and every text still passes through the redactor
///    like any other string (D-22).
/// 2. **The cost figure is notional, not billed.** Phase 15-05's caveat is
///    carried into [`JournalEvent::ExecFinished`]'s and [`JournalEvent::Cost`]'s
///    field docs rather than restated: the CLI reports a modelled number and
///    subscription runs are not billed per token.
///
/// A **turn boundary is not a run boundary**. `TurnCompleted` maps to an
/// `exec_event` on its own stream label rather than to `exec_finished`, because
/// conflating them would misreport one multi-turn run as several runs.
///
/// `None` is reserved for a future `ExecutionEvent` variant that carries nothing
/// worth a durable record. Every variant that exists today maps to `Some`, and
/// `every_reserved_kind_is_declared_and_none_is_emitted_by_this_phase` asserts
/// that the produced kinds all live in [`EMITTED_KINDS`].
pub fn from_exec_event(ev: &ExecutionEvent, argv_digest: &str) -> Option<JournalEvent> {
    let event = match ev {
        ExecutionEvent::SessionStarted { session_id, .. } => JournalEvent::ExecStarted {
            session_id: session_id.clone(),
            argv_digest: argv_digest.to_string(),
            // Stamped by `JournalRun::record_exec`, which owns the run's handle
            // on the spawned group. A caller using this function standalone gets
            // the session id and the digest and nothing else (D-09).
            claude_pgid: None,
        },
        ExecutionEvent::Message(message) => JournalEvent::ExecEvent {
            stream: stream_label(message).to_string(),
            text: exec_message_text(message),
        },
        ExecutionEvent::Unknown { raw } => JournalEvent::ExecEvent {
            stream: "unknown".to_string(),
            text: raw.clone(),
        },
        ExecutionEvent::Unparseable { raw, .. } => JournalEvent::ExecEvent {
            stream: "unparseable".to_string(),
            text: raw.clone(),
        },
        // Mirrors the field shape the executor already uses for this condition
        // (`ExecutionEvent::LineTruncated { bytes, prefix }`), so the journal
        // reports the same two facts the stream did rather than inventing a
        // third rendering of them.
        ExecutionEvent::LineTruncated { bytes, prefix } => JournalEvent::Diagnostic {
            code: "line_truncated".to_string(),
            detail: format!("{bytes} bytes; prefix: {prefix}"),
        },
        ExecutionEvent::Stderr(line) => JournalEvent::ExecEvent {
            stream: "stderr".to_string(),
            text: line.clone(),
        },
        ExecutionEvent::TurnCompleted(result) => JournalEvent::ExecEvent {
            stream: "turn_completed".to_string(),
            text: turn_result_text(result),
        },
        ExecutionEvent::Cost { cumulative_usd } => JournalEvent::Cost {
            cumulative_usd: *cumulative_usd,
        },
        // D-33: a run that silently lost 40 events is materially different from
        // one that lost none, and this count is the only signal that
        // distinguishes them.
        ExecutionEvent::EventsDropped { count } => JournalEvent::EventsDropped { count: *count },
        ExecutionEvent::Exited(status) => JournalEvent::ExecFinished {
            exit: status.code(),
            // Stamped by `JournalRun::record_exec`, which owns the run's clock
            // and its cost total. A caller using this function standalone gets
            // the exit code and nothing else.
            cost_usd: None,
            duration_s: 0,
        },
    };
    Some(event)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn utc(rfc3339: &str) -> chrono::DateTime<chrono::Utc> {
        chrono::DateTime::parse_from_rfc3339(rfc3339)
            .expect("test timestamp parses")
            .with_timezone(&chrono::Utc)
    }

    #[test]
    fn classify_change_names_a_journal_path_as_driver() {
        let root = Path::new("/home/user/projects/myapp");
        let run_id = "2026-07-28T14-03-11Z-a3f9";
        let cases = [
            ".planning/meta-manager/runs/2026-07-28T14-03-11Z-a3f9/journal.jsonl",
            ".planning/meta-manager/runs/2026-07-28T14-03-11Z-a3f9/run.json",
            ".planning/meta-manager/runs/2026-07-28T14-03-11Z-a3f9/inbox.jsonl",
            ".planning/meta-manager/runs/2026-07-28T14-03-11Z-a3f9/nested/run.json",
        ];
        for case in cases {
            assert_eq!(
                classify_change(root, &root.join(case)),
                ChangeKind::DriverJournal {
                    run_id: run_id.to_string()
                },
                "{case} must classify as a driver journal write"
            );
        }
    }

    #[test]
    fn classify_change_names_state_md_as_planning() {
        let root = Path::new("/home/user/projects/myapp");
        let cases = [
            ".planning/STATE.md",
            ".planning/ROADMAP.md",
            ".planning/phases/16-run-journal-state-substrate/16-01-PLAN.md",
            ".planning/meta-manager/QUEUE.md",
            "src/main.rs",
        ];
        for case in cases {
            assert_eq!(
                classify_change(root, &root.join(case)),
                ChangeKind::Planning,
                "{case} must classify as a planning write"
            );
        }
    }

    #[test]
    fn classify_change_does_not_treat_a_bare_runs_directory_as_driver() {
        let root = Path::new("/home/user/projects/myapp");
        let cases = [
            // A project that keeps its own `runs/` under `.planning/`. The
            // driver prefix is `meta-manager/runs/`, never a bare `runs`.
            ".planning/runs/2026-07-28T14-03-11Z-a3f9/journal.jsonl",
            ".planning/runs/anything",
            "meta-manager/runs/2026-07-28T14-03-11Z-a3f9/journal.jsonl",
            // Directly under `runs/`: names no run, so it carries no run id.
            ".planning/meta-manager/runs/.gitignore",
            ".planning/meta-manager/runs/active",
            ".planning/meta-manager/runs/2026-07-28T14-03-11Z-a3f9",
            ".planning/meta-manager/runs",
        ];
        for case in cases {
            assert_eq!(
                classify_change(root, &root.join(case)),
                ChangeKind::Planning,
                "{case} must not classify as a driver journal write"
            );
        }
    }

    #[test]
    fn classify_change_returns_planning_for_a_path_outside_the_project() {
        let root = Path::new("/home/user/projects/myapp");
        let cases = [
            "/home/user/projects/other/.planning/meta-manager/runs/RID/journal.jsonl",
            "/etc/passwd",
            "/home/user/projects/myapp-sibling/.planning/STATE.md",
        ];
        for case in cases {
            assert_eq!(
                classify_change(root, Path::new(case)),
                ChangeKind::Planning,
                "{case} is outside the project and must classify as planning"
            );
        }
    }

    #[test]
    fn run_id_sorts_lexicographically_in_chronological_order() {
        // Deliberately pairs the EARLIER timestamp with the LEXICALLY LARGEST
        // uuid suffix, so a format in which the suffix could decide the order
        // would fail this assertion.
        let earlier = new_run_id(
            utc("2026-07-28T14:03:11Z"),
            &uuid::Uuid::from_u128(u128::MAX),
        );
        let later = new_run_id(utc("2026-07-28T14:03:12Z"), &uuid::Uuid::from_u128(0));

        assert_eq!(earlier, "2026-07-28T14-03-11Z-ffff");
        assert_eq!(later, "2026-07-28T14-03-12Z-0000");
        assert!(
            earlier < later,
            "lexicographic order must equal chronological order: {earlier} !< {later}"
        );
        assert!(!earlier.contains(':'), "colons are hostile in paths (D-02)");

        // And across a day boundary, where naive formats usually break.
        let midnight = new_run_id(utc("2026-07-29T00:00:00Z"), &uuid::Uuid::from_u128(0));
        assert!(later < midnight, "{later} !< {midnight}");
    }

    #[test]
    fn run_paths_place_every_artifact_where_the_layout_says() {
        let planning = Path::new("/p/.planning");
        let paths = run_paths(planning, "RID").expect("a plain run id yields paths");
        assert_eq!(paths.dir, Path::new("/p/.planning/meta-manager/runs/RID"));
        assert_eq!(
            paths.inbox,
            Path::new("/p/.planning/meta-manager/runs/RID/inbox.jsonl")
        );
        assert_eq!(
            paths.run_json,
            Path::new("/p/.planning/meta-manager/runs/RID/run.json")
        );
        assert_eq!(
            paths.journal,
            Path::new("/p/.planning/meta-manager/runs/RID/journal.jsonl")
        );
        assert_eq!(
            paths.gitignore,
            Path::new("/p/.planning/meta-manager/runs/.gitignore")
        );
        assert_eq!(
            paths.active,
            Path::new("/p/.planning/meta-manager/runs/active")
        );
        assert_eq!(runs_root(planning), Path::new("/p/.planning/meta-manager/runs"));
    }

    #[test]
    fn argv_digest_is_stable_and_separator_safe() {
        let a = vec!["claude".to_string(), "-p".to_string(), "hello".to_string()];
        assert_eq!(argv_digest(&a), argv_digest(&a));
        assert!(argv_digest(&a).starts_with("fnv1a64:"));
        assert_eq!(argv_digest(&a).len(), "fnv1a64:".len() + 16);

        // The unit separator means an argv split cannot collide with an argv
        // that merely contains the joined text.
        let b = vec!["claude".to_string(), "-p hello".to_string()];
        assert_ne!(argv_digest(&a), argv_digest(&b));
    }

    // ========================================================================
    // The approval binding: the plan AND the bytes, together
    // ========================================================================

    /// The disclosed set as it would stand for a project with these digests.
    fn inputs(pairs: &[(&str, Option<&str>)]) -> Vec<crate::config::PromptInput> {
        pairs
            .iter()
            .map(|(path, digest)| crate::config::PromptInput {
                path: (*path).to_string(),
                digest: digest.map(str::to_string),
                extra: Default::default(),
            })
            .collect()
    }

    /// An approval covering `plan_digest` against `files`.
    fn approval(plan_digest: &str, files: &[crate::config::PromptInput]) -> ApprovedPlan {
        ApprovedPlan {
            steps: vec!["command=/gsd-plan-phase phase=21 terminal=verification_passed".to_string()],
            target_phase: "21".to_string(),
            plan_digest: plan_digest.to_string(),
            approval_digest: approval_digest(plan_digest, files),
            approved_at: "2026-08-19T12:00:00Z".to_string(),
            extra: serde_json::Map::new(),
        }
    }

    #[test]
    fn an_approval_digest_changes_when_the_disclosed_file_set_changes_under_an_identical_plan() {
        let plan = "fnv1a64:aaaaaaaaaaaaaaaa";

        let before = inputs(&[
            ("CLAUDE.md", Some("sha256:1111")),
            (".planning/STATE.md", None),
        ]);
        // The plan is byte-for-byte identical; only the bytes of a disclosed
        // file moved. **Against a binding that covered the plan alone this
        // assertion FAILS**, because the two digests are then equal and an
        // approval given before a `git pull` would still authorise the run
        // afterwards — which is the replay hazard research Q4 names.
        let after = inputs(&[
            ("CLAUDE.md", Some("sha256:2222")),
            (".planning/STATE.md", None),
        ]);
        assert_ne!(
            approval_digest(plan, &before),
            approval_digest(plan, &after),
            "an approval must not survive a change to the bytes that will enter \
             the prompt"
        );

        // A file APPEARING is drift too, which is why absent files are recorded
        // with a `None` digest rather than omitted from the list.
        let appeared = inputs(&[
            ("CLAUDE.md", Some("sha256:1111")),
            (".planning/STATE.md", Some("sha256:3333")),
        ]);
        assert_ne!(approval_digest(plan, &before), approval_digest(plan, &appeared));

        // And the same set in a different enumeration order is the same set: the
        // digest is a fact about which files carry which bytes, not about the
        // order a build happened to walk them in.
        let reordered = inputs(&[
            (".planning/STATE.md", None),
            ("CLAUDE.md", Some("sha256:1111")),
        ]);
        assert_eq!(approval_digest(plan, &before), approval_digest(plan, &reordered));

        // The control: a changed PLAN moves it too, so the assertions above are
        // about the file half rather than about a digest that changes for
        // everything.
        assert_ne!(
            approval_digest(plan, &before),
            approval_digest("fnv1a64:bbbbbbbbbbbbbbbb", &before)
        );
    }

    #[test]
    fn an_absent_approval_reports_itself_absent_rather_than_mismatched() {
        let files = inputs(&[("CLAUDE.md", Some("sha256:1111"))]);
        let refusal = recheck_approval(None, "fnv1a64:aaaaaaaaaaaaaaaa", &files)
            .expect_err("a run with no recorded approval must never spawn");

        assert_eq!(
            refusal,
            ApprovalRefusal::Absent,
            "absence of an approval is a refusal, never a default yes — and it \
             must be DISTINGUISHABLE from a mismatch, or the message sends the \
             user looking for a change that never happened"
        );
        let rendered = refusal.to_string();
        assert!(
            rendered.contains("no plan approval was recorded"),
            "the message must say the approval is absent; got: {rendered}"
        );
        assert!(
            rendered.contains("--approved-plan"),
            "and it must name the flag that supplies one, because a refusal a \
             caller cannot act on is a bug report; got: {rendered}"
        );
    }

    #[test]
    fn an_approval_bound_to_a_different_plan_is_refused_naming_the_plan_half() {
        let files = inputs(&[("CLAUDE.md", Some("sha256:1111"))]);
        let recorded = approval("fnv1a64:aaaaaaaaaaaaaaaa", &files);

        // The same files, a different plan: a model asked the same question
        // twice may answer differently, and an approval covers one answer.
        let refusal = recheck_approval(Some(&recorded), "fnv1a64:bbbbbbbbbbbbbbbb", &files)
            .expect_err("a plan the approval never covered must not run");
        assert!(
            matches!(refusal, ApprovalRefusal::PlanChanged { .. }),
            "got: {refusal:?}"
        );

        // The control arm: unchanged, it passes — so the refusal above is about
        // the plan rather than about a check that refuses everything.
        recheck_approval(Some(&recorded), "fnv1a64:aaaaaaaaaaaaaaaa", &files)
            .expect("an unchanged plan against unchanged files is approved");
    }

    #[test]
    fn an_approval_whose_disclosed_files_moved_is_refused_even_though_the_plan_is_identical() {
        // **Against the unfixed behaviour — an approval bound to the plan only —
        // this test FAILS**, because the plan digest still matches and the run
        // starts against files the approval never covered. That is the whole of
        // research Q4's second half, and the reason one mechanism has to cover
        // both.
        let plan = "fnv1a64:aaaaaaaaaaaaaaaa";
        let recorded = approval(plan, &inputs(&[("CLAUDE.md", Some("sha256:1111"))]));

        let rewritten = inputs(&[("CLAUDE.md", Some("sha256:2222"))]);
        let refusal = recheck_approval(Some(&recorded), plan, &rewritten)
            .expect_err("bytes that changed after the approval must not be run against");

        assert!(
            matches!(refusal, ApprovalRefusal::DisclosedFilesChanged { .. }),
            "and the refusal must say the FILES moved rather than the plan, or \
             the user goes looking in the wrong place; got: {refusal:?}"
        );
        assert!(
            refusal.to_string().contains("the plan is unchanged"),
            "got: {refusal}"
        );
    }

    // -----------------------------------------------------------------------
    // The approval TOKEN: one copy-pasteable value carrying both halves
    // -----------------------------------------------------------------------

    #[test]
    fn an_approval_token_round_trips_its_two_halves_in_plan_then_approval_order() {
        let plan = "sha256:1111111111111111111111111111111111111111111111111111111111111111";
        let approval = "sha256:2222222222222222222222222222222222222222222222222222222222222222";

        let token = render_approval_token(plan, approval);
        assert!(
            token.contains(APPROVAL_TOKEN_SEPARATOR),
            "the rendered token must carry the separator the parse splits on; \
             got: {token}"
        );

        let (parsed_plan, parsed_approval) = parse_approval_token(&token)
            .expect("a token this module rendered must parse back");
        assert_eq!(
            parsed_plan, plan,
            "the PLAN half comes first. Order is not a detail: the two halves are \
             both `sha256:`-prefixed strings, so a reversed parse would compare \
             the plan digest against the approval digest and report the wrong \
             half as the one that moved"
        );
        assert_eq!(parsed_approval, approval, "and the approval half second");

        // One producer, one consumer, and nothing in between: a caller that
        // assembles a token by concatenation is a second spelling of this
        // function, and two spellings are two things that can disagree.
        assert_eq!(
            token,
            format!("{plan}{APPROVAL_TOKEN_SEPARATOR}{approval}"),
            "the rendered shape is exactly the two halves joined by the \
             separator; got: {token}"
        );
    }

    #[test]
    fn a_token_that_does_not_carry_both_halves_is_refused_rather_than_read_as_one() {
        let half = "sha256:1111111111111111111111111111111111111111111111111111111111111111";
        let other = "sha256:2222222222222222222222222222222222222222222222222222222222222222";

        // **A legacy single-half value.** Before this became a two-half token,
        // `--approved-plan` took the approval digest alone. Such a value must
        // fail the parse rather than being read as a plan half with an empty
        // approval half — an approval that cannot be parsed is an ABSENT
        // approval, and the fail-closed direction is the only safe one.
        assert_eq!(
            parse_approval_token(half),
            Err(ApprovalTokenError::SeparatorAbsent),
            "a value carrying no separator is not half an approval; got: {:?}",
            parse_approval_token(half)
        );

        // Three values concatenated must not be read as two.
        let three = format!("{half}{APPROVAL_TOKEN_SEPARATOR}{other}{APPROVAL_TOKEN_SEPARATOR}{half}");
        assert_eq!(
            parse_approval_token(&three),
            Err(ApprovalTokenError::SeparatorRepeated),
            "got: {:?}",
            parse_approval_token(&three)
        );

        // An empty half on either side, which is what a token truncated at a
        // shell boundary or copied without one end looks like.
        for truncated in [
            format!("{APPROVAL_TOKEN_SEPARATOR}{other}"),
            format!("{half}{APPROVAL_TOKEN_SEPARATOR}"),
            APPROVAL_TOKEN_SEPARATOR.to_string(),
        ] {
            assert_eq!(
                parse_approval_token(&truncated),
                Err(ApprovalTokenError::HalfEmpty),
                "an empty half is not an approval of anything; got: {:?} for \
                 {truncated:?}",
                parse_approval_token(&truncated)
            );
        }

        // Nothing is trimmed. A value with surrounding whitespace is the
        // caller's to fix; accepting it silently would mean two spellings of one
        // approval, and the digests would then disagree about which is recorded.
        let padded = format!(" {half}{APPROVAL_TOKEN_SEPARATOR}{other}");
        assert_eq!(
            parse_approval_token(&padded),
            Ok((format!(" {half}"), other.to_string())),
            "the parse splits and does not trim, so a padded half stays padded \
             and fails the digest comparison by name rather than being quietly \
             normalised into a different value"
        );
    }

    #[test]
    fn every_approval_token_refusal_names_the_flag_and_says_how_to_obtain_a_token() {
        // No wildcard: a fourth arm added later has to be added here too, which
        // is the moment the decision about its message is cheap.
        for refusal in [
            ApprovalTokenError::SeparatorAbsent,
            ApprovalTokenError::SeparatorRepeated,
            ApprovalTokenError::HalfEmpty,
        ] {
            let rendered = refusal.to_string();
            assert!(
                rendered.contains("--approved-plan"),
                "a refusal a caller cannot act on is a bug report rather than an \
                 error message, and the flag is half of the action; got: \
                 {rendered}"
            );
            assert!(
                rendered.to_lowercase().contains("without"),
                "and the other half is HOW to obtain a good token: by re-running \
                 the same invocation WITHOUT the flag, which prints the plan and \
                 the token that authorises it; got: {rendered}"
            );
        }
    }

    #[test]
    fn the_three_new_run_record_fields_survive_a_round_trip_and_default_when_absent() {
        // A record written by a pre-Phase-21 binary has none of the three keys
        // and must still load — `run.json` carries no version discriminator, so
        // tolerance on read is the only mechanism available.
        let legacy = serde_json::to_value(run_record(RUN_ID)).expect("serialises");
        let mut stripped = legacy.as_object().expect("an object").clone();
        for key in ["approved_plan", "escalation_cap", "escalations_used"] {
            stripped.remove(key);
        }
        let loaded: RunRecord =
            serde_json::from_value(serde_json::Value::Object(stripped)).expect("a pre-Phase-21 record still loads");
        assert!(loaded.approved_plan.is_none());
        assert_eq!(loaded.escalation_cap, None);
        assert_eq!(loaded.escalations_used, None);

        // And a record carrying all three round-trips through the typed struct
        // without losing any of them.
        let files = inputs(&[("CLAUDE.md", Some("sha256:1111"))]);
        let mut record = run_record(RUN_ID);
        record.approved_plan = Some(approval("fnv1a64:aaaaaaaaaaaaaaaa", &files));
        record.escalation_cap = Some(3);
        record.escalations_used = Some(1);

        let round: RunRecord =
            serde_json::from_str(&serde_json::to_string(&record).expect("serialises"))
                .expect("deserialises");
        assert_eq!(round.escalation_cap, Some(3));
        assert_eq!(round.escalations_used, Some(1));
        assert_eq!(
            round.approved_plan.as_ref().map(|plan| plan.plan_digest.as_str()),
            Some("fnv1a64:aaaaaaaaaaaaaaaa")
        );
        assert_eq!(
            round.approved_plan.as_ref().map(|plan| plan.target_phase.as_str()),
            Some("21")
        );
    }

    #[test]
    fn sha256_digest_is_prefixed_and_is_sixty_four_lowercase_hex_digits() {
        // A known vector, so this fails if the function ever stops being
        // SHA-256 rather than merely stops being stable against itself.
        let digest = sha256_digest(b"abc");
        assert_eq!(
            digest,
            "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
            "the published SHA-256 vector for `abc`; comparing the function with \
             itself would not detect a swap to a different hash"
        );

        let hex = digest
            .strip_prefix("sha256:")
            .expect("the prefix is what keeps a legacy value from comparing equal");
        assert_eq!(hex.len(), 64, "SHA-256 renders as 64 hex digits");
        assert!(
            hex.chars()
                .all(|c| c.is_ascii_digit() || ('a'..='f').contains(&c)),
            "lowercase hex only, so two records of the same bytes compare as \
             strings without normalising. Got: {hex}"
        );
    }

    #[test]
    fn two_different_inputs_produce_different_sha256_digests() {
        assert_ne!(sha256_digest(b"one"), sha256_digest(b"two"));
        // A single flipped byte must move the digest — the whole point of the
        // upgrade is that a crafted near-identical file does not slip through.
        assert_ne!(sha256_digest(b"CLAUDE.md v1"), sha256_digest(b"CLAUDE.md v2"));
    }

    #[test]
    fn the_two_digest_functions_cannot_be_confused_for_the_same_input() {
        // Same logical input through both functions. If a caller ever reaches
        // for the wrong one, the prefix is what makes it visible on disk rather
        // than a silent downgrade to a non-security hash.
        let argv = vec!["abc".to_string()];
        let weak = argv_digest(&argv);
        let strong = sha256_digest(b"abc");

        assert_ne!(weak, strong);
        assert!(weak.starts_with("fnv1a64:"));
        assert!(strong.starts_with("sha256:"));
        assert!(
            !weak.starts_with("sha256:"),
            "a value that is not collision resistant must never wear the prefix \
             the re-confirmation gate treats as trustworthy"
        );
    }

    #[test]
    fn only_content_events_report_themselves_as_content() {
        assert!(JournalEvent::ExecEvent {
            stream: "assistant".to_string(),
            text: "hi".to_string(),
        }
        .is_content());
        assert!(JournalEvent::Interjected {
            id: Some("3f2a".to_string()),
            text: "stop".to_string(),
            delivered: true,
        }
        .is_content());
        assert!(!JournalEvent::RunEnded {
            outcome: "completed".to_string(),
            ended_at: "2026-07-28T14:03:11Z".to_string(),
        }
        .is_content());
        assert!(!JournalEvent::EventsDropped { count: 40 }.is_content());
        // The two interjection transitions carry an id and a fixed reason, so
        // they are lifecycle and must survive the per-run cap — a delivered
        // message stranded in `delivered` forever is the silent abandonment the
        // three-state display exists to prevent.
        assert!(!JournalEvent::InterjectionActedOn {
            id: "3f2a".to_string(),
        }
        .is_content());
        assert!(!JournalEvent::InterjectionMissed {
            id: "3f2a".to_string(),
            reason: "stdin closed".to_string(),
        }
        .is_content());
    }

    #[test]
    fn only_a_single_plain_component_is_accepted_as_a_run_id() {
        // WR-02's write side, at the predicate. Every rejected case below is a
        // shape the reproduction in `17-REVIEW.md` reached or a near neighbour
        // of it.
        assert!(is_plain_path_component("2026-07-28T14-03-11Z-a3f9"));
        assert!(is_plain_path_component("RID"));

        for hostile in [
            "",
            ".",
            "..",
            "../escaped",
            "../../../../escaped",
            "a/b",
            "/etc/passwd",
            "/",
            "./escaped",
            "escaped/",
            "a/../b",
        ] {
            assert!(
                !is_plain_path_component(hostile),
                "{hostile:?} must not be accepted as a run id"
            );
            assert!(
                run_paths(Path::new("/p/.planning"), hostile).is_none(),
                "{hostile:?} must yield no paths at all"
            );
        }
    }

    // ---- The run lifecycle and the executor mapping (plan 16-06, Task 1) ----

    const RUN_ID: &str = "2026-07-28T14-03-11Z-a3f9";

    fn run_record(run_id: &str) -> RunRecord {
        RunRecord {
            run_id: run_id.to_string(),
            goal: "close the run journal phase".to_string(),
            // Phase 21's three Run-scoped fields, absent on a fixture that
            // predates them — which is precisely the shape the tolerant read
            // path has to keep loading.
            approved_plan: None,
            escalation_cap: None,
            escalations_used: None,
            gsd_command: "/gsd:execute-phase 16".to_string(),
            target: "host".to_string(),
            opt_in: None,
            started_at: "2026-07-28T14:03:11Z".to_string(),
            session_id: "9f1c0e2a-0000-4000-8000-000000000000".to_string(),
            pid: 4242,
            pgid: 4242,
            claude_code_version: "2.1.0".to_string(),
            argv_digest: argv_digest(&["claude".to_string(), "-p".to_string()]),
            ended_at: None,
            outcome: None,
            // The single-command shape: this helper backs the pre-Phase-20
            // tests, and they must keep asserting what a command-mode run
            // records rather than quietly acquiring routed values.
            target_phase: None,
            bounds: None,
            extra: serde_json::Map::new(),
        }
    }

    /// A process exit status carrying `code`. `std::process::ExitStatus` has no
    /// portable constructor, so both platform extensions are used — the same
    /// shape `src/executor/outcome.rs:390-402` already uses for this.
    #[cfg(unix)]
    fn exit_status(code: i32) -> std::process::ExitStatus {
        use std::os::unix::process::ExitStatusExt;
        // The raw wait status packs the exit code into the high byte.
        std::process::ExitStatus::from_raw(code << 8)
    }

    #[cfg(windows)]
    fn exit_status(code: i32) -> std::process::ExitStatus {
        use std::os::windows::process::ExitStatusExt;
        std::process::ExitStatus::from_raw(code as u32)
    }

    fn message(json: serde_json::Value) -> ExecutionEvent {
        ExecutionEvent::Message(Box::new(
            serde_json::from_value(json).expect("the fixture is a valid stream message"),
        ))
    }

    /// Every `ExecutionEvent` variant, once each, in mapping order.
    ///
    /// Built from wire-shaped JSON rather than by hand-constructing the wire
    /// types, because those types are Phase 15's and this phase is forbidden to
    /// widen them (D-37) — including with a constructor added for a test's
    /// convenience.
    fn every_execution_event() -> Vec<ExecutionEvent> {
        vec![
            ExecutionEvent::SessionStarted {
                session_id: "9f1c0e2a-0000-4000-8000-000000000000".to_string(),
                capabilities: vec!["control".to_string()],
                claude_code_version: "2.1.0".to_string(),
                api_key_source: Some("none".to_string()),
                permission_mode: Some("dontAsk".to_string()),
            },
            message(serde_json::json!({ "type": "assistant" })),
            ExecutionEvent::Unknown {
                raw: r#"{"type":"telepathy"}"#.to_string(),
            },
            ExecutionEvent::Unparseable {
                raw: "{\"type\":".to_string(),
                error: "EOF while parsing".to_string(),
            },
            ExecutionEvent::LineTruncated {
                bytes: 262_144,
                prefix: "{\"type\":\"assistant\"".to_string(),
            },
            ExecutionEvent::Stderr("workspace trust warning".to_string()),
            ExecutionEvent::TurnCompleted(Box::new(
                serde_json::from_value(serde_json::json!({ "subtype": "success" }))
                    .expect("the fixture is a valid result envelope"),
            )),
            ExecutionEvent::Cost {
                cumulative_usd: 1.83,
            },
            ExecutionEvent::EventsDropped { count: 40 },
            ExecutionEvent::Exited(exit_status(0)),
        ]
    }

    fn kind_of(event: &JournalEvent) -> String {
        serde_json::to_value(event).expect("an event serialises")["kind"]
            .as_str()
            .expect("kind is a string")
            .to_string()
    }

    #[test]
    fn a_run_writes_its_record_exactly_twice() {
        let dir = tempfile::tempdir().expect("temp dir");
        let planning = dir.path().join(".planning");

        let mut run = JournalRun::start(&planning, run_record(RUN_ID)).expect("start the run");
        assert_eq!(run.record_writes(), 1, "write one lands at run start (D-06)");
        assert!(run.paths().run_json.is_file());
        assert!(run.paths().gitignore.is_file(), "the ignore file lands first");
        assert_eq!(
            writer::read_active_run(&planning).as_deref(),
            Some(RUN_ID),
            "the active pointer names the run while it runs"
        );

        // A hundred journal events must not touch the record.
        for _ in 0..100 {
            run.record(&JournalEvent::ExecEvent {
                stream: "assistant".to_string(),
                text: "working".to_string(),
            })
            .expect("append");
        }
        assert_eq!(run.record_writes(), 1, "journalling never rewrites run.json");

        run.finish("completed").expect("finish the run");
        assert_eq!(
            run.record_writes(),
            2,
            "exactly two — not `at least` two (D-06)"
        );
        assert_eq!(
            writer::read_active_run(&planning),
            None,
            "the pointer is cleared at the terminal transition"
        );

        // And the second write is what Phase 17's reconciliation reads.
        let stamped: RunRecord = serde_json::from_str(
            &std::fs::read_to_string(&run.paths().run_json).expect("read the record"),
        )
        .expect("the record deserialises");
        assert_eq!(stamped.outcome.as_deref(), Some("completed"));
        assert!(
            stamped.ended_at.is_some(),
            "an absent ended_at is the crash signal, so a finished run must carry one"
        );
        assert_eq!(stamped.goal, "close the run journal phase");
    }

    // ---- Phase 20: the record's scope classification and its new fields ----

    /// The `RunRecord` struct body, sliced out of this module's own source.
    ///
    /// Source-parsing rather than reflection, following the precedent
    /// `tests/spawn_seam_guard.rs` sets: a doc comment is not visible at
    /// runtime, so the only way to assert that every field *documents* its scope
    /// is to read the text that documents it.
    fn run_record_struct_body() -> &'static str {
        const SOURCE: &str = include_str!("mod.rs");
        let start = SOURCE
            .find("pub struct RunRecord {")
            .expect("the struct this test is named for must exist");
        let body = &SOURCE[start..];
        let end = body
            .find("\n}\n")
            .expect("the struct body must be brace-terminated at column zero");
        &body[..end]
    }

    #[test]
    fn run_record_fields_all_declare_their_scope() {
        let body = run_record_struct_body();

        // A field declaration is a `pub <name>:` line. Attributes and doc lines
        // are gathered as the *preceding* run of non-declaration lines, which is
        // where the doc comment for that field lives.
        let mut examined = 0usize;
        let mut undocumented: Vec<String> = Vec::new();
        let mut doc_block = String::new();

        for line in body.lines() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("pub ") && trimmed.contains(':') {
                examined += 1;
                let name = trimmed
                    .trim_start_matches("pub ")
                    .split(':')
                    .next()
                    .unwrap_or_default()
                    .to_string();
                let lowered = doc_block.to_lowercase();
                if !lowered.contains("run-scoped") && !lowered.contains("iteration-scoped") {
                    undocumented.push(name);
                }
                doc_block.clear();
            } else if trimmed.starts_with("///") {
                doc_block.push_str(trimmed);
                doc_block.push('\n');
            } else if trimmed.starts_with("#[") || trimmed.is_empty() {
                // Attributes and blank lines do not end a doc block.
            } else {
                doc_block.clear();
            }
        }

        // **The non-vacuity floor.** A parse that silently matched nothing would
        // pass this test with an empty `undocumented` list while proving
        // nothing at all — the exact failure mode a source-scanning guard is
        // most prone to.
        assert!(
            examined >= 10,
            "the scan examined only {examined} field declarations; the record has \
             far more than that, so the parse has broken rather than the struct \
             having shrunk"
        );

        assert!(
            undocumented.is_empty(),
            "Phase 20 made the driver's unit of work a SEQUENCE, so every field \
             that was singular because the unit was singular must now say which \
             scope it belongs to. These declare neither `run-scoped` nor \
             `iteration-scoped`: {undocumented:?}"
        );
    }

    #[test]
    fn a_pre_phase_20_record_still_loads_with_the_new_fields_defaulted() {
        // **A byte literal of the OLD shape, never a record built from the
        // current struct.** A fixture constructed from today's `RunRecord` would
        // acquire every field added since, and would therefore agree with itself
        // while proving nothing about a record already on disk. `run.json`
        // carries no version discriminator, so tolerance on read is the only
        // migration mechanism there is.
        const PRE_PHASE_20: &[u8] = br#"{
            "run_id": "2026-07-28T14-03-11Z-a3f9",
            "goal": "close the run journal phase",
            "gsd_command": "/gsd:execute-phase 16",
            "target": "host",
            "opt_in": null,
            "started_at": "2026-07-28T14:03:11Z",
            "session_id": "9f1c0e2a-0000-4000-8000-000000000000",
            "pid": 4242,
            "pgid": 4242,
            "claude_code_version": "2.1.0",
            "argv_digest": "fnv1a64:0000000000000000",
            "ended_at": null,
            "outcome": null
        }"#;

        let record: RunRecord =
            serde_json::from_slice(PRE_PHASE_20).expect("a pre-Phase-20 record must still load");

        assert_eq!(record.run_id, "2026-07-28T14-03-11Z-a3f9");
        assert_eq!(
            record.gsd_command, "/gsd:execute-phase 16",
            "a command-mode record keeps meaning exactly what it meant"
        );
        assert_eq!(
            record.target_phase, None,
            "the routed identity defaults to absent, which is the truth about a \
             record written before routing existed"
        );
        assert_eq!(
            record.bounds, None,
            "`None` says the bounds were not recorded — it must never be \
             fabricated as today's defaults, which would assert a cap the run \
             may never have run under"
        );
        assert!(
            record.extra.is_empty(),
            "the old shape carries no unknown fields"
        );
    }

    #[test]
    fn a_record_written_by_a_newer_binary_keeps_its_unknown_fields_through_a_round_trip() {
        // T-20-08: two binary versions share this file and users commit it. An
        // older build that loaded and re-saved a newer record while dropping the
        // fields it does not model would be a destructive rewrite by omission.
        const FROM_THE_FUTURE: &[u8] = br#"{
            "run_id": "2027-01-01T00-00-00Z-ffff",
            "goal": "g",
            "gsd_command": "/gsd-progress",
            "target": "host",
            "opt_in": null,
            "started_at": "2027-01-01T00:00:00Z",
            "session_id": "s",
            "pid": 1,
            "pgid": 1,
            "claude_code_version": "9.9.9",
            "argv_digest": "fnv1a64:0000000000000000",
            "ended_at": null,
            "outcome": null,
            "a_field_from_2027": {"nested": [1, 2, 3]}
        }"#;

        let record: RunRecord = serde_json::from_slice(FROM_THE_FUTURE).expect("it loads");
        assert_eq!(
            record.extra.len(),
            1,
            "the unknown field lands in the flattened overflow rather than being \
             dropped on the floor"
        );

        let round_tripped = serde_json::to_value(&record).expect("it serialises");
        assert_eq!(
            round_tripped["a_field_from_2027"]["nested"],
            serde_json::json!([1, 2, 3]),
            "and it survives the trip back out with its value intact"
        );
        assert!(
            round_tripped.get("target_phase").is_some(),
            "flattening the overflow must not stop this build writing its own \
             fields"
        );
    }

    #[test]
    fn the_recorded_bounds_round_trip_as_plain_numbers() {
        let mut record = run_record(RUN_ID);
        record.bounds = Some(RecordedBounds {
            max_steps: 7,
            wall_clock_cap_secs: 3600,
        });

        let value = serde_json::to_value(&record).expect("it serialises");
        assert_eq!(
            value["bounds"]["max_steps"], 7,
            "a reader gets a number, not a Duration's {{secs, nanos}} object"
        );
        assert_eq!(value["bounds"]["wall_clock_cap_secs"], 3600);

        let back: RunRecord = serde_json::from_value(value).expect("it loads");
        assert_eq!(back.bounds, record.bounds);
    }

    #[test]
    fn a_routed_run_of_two_iterations_still_writes_the_record_exactly_twice() {
        // The invariant the whole document depends on (D-06, D-07): the
        // iteration loop lives strictly *inside* the two writes and adds neither
        // a third nor a per-iteration one. Counted through the production
        // journal's own counter rather than by watching the file, so a write
        // that happened and was overwritten still counts.
        let dir = tempfile::tempdir().expect("temp dir");
        let planning = dir.path().join(".planning");

        let mut record = run_record(RUN_ID);
        record.gsd_command = crate::driver::ROUTED_RECORD_MARKER.to_string();
        record.target_phase = Some("20".to_string());
        record.bounds = Some(RecordedBounds {
            max_steps: 5,
            wall_clock_cap_secs: 4 * 60 * 60,
        });

        let mut run = JournalRun::start(&planning, record).expect("start the run");
        assert_eq!(run.record_writes(), 1, "write one lands at run start");

        // Two iterations' worth of the events a routed run emits per command.
        for command in ["/gsd-plan-phase 20", "/gsd-plan-phase 20"] {
            run.record(&JournalEvent::Decided {
                by: "policy".to_string(),
                command: command.to_string(),
                rationale: "the target phase is Discussed".to_string(),
            })
            .expect("the decision is journalled");
        }

        run.finish("parked:bounds_command_repeat")
            .expect("finish the run");

        assert_eq!(
            run.record_writes(),
            2,
            "exactly two for a routed run of two iterations — the loop adds no \
             third write and no per-iteration write"
        );
    }

    #[test]
    fn the_routed_record_marker_cannot_be_misread_as_absent_or_as_an_argv_fragment() {
        let marker = crate::driver::ROUTED_RECORD_MARKER;

        // The point of option-a is that the VALUE cannot be misread, so the
        // properties are asserted rather than only the literal.
        assert!(
            !marker.is_empty(),
            "`\"\"` already means `the field was absent` on the tolerant read \
             path (a_record_from_an_unknown_schema_still_produces_a_row, D-30); \
             a routed run must not be indistinguishable from an unparseable one"
        );
        assert!(
            !marker.contains("--"),
            "an argv fragment like `--target-phase 3` reads as a pasteable \
             command line and is not one — that is the shape this marker \
             replaced"
        );
        assert!(
            !marker.starts_with('/'),
            "a leading slash is what a real GSD command looks like; the marker \
             must not be mistakable for one"
        );
        assert!(
            !marker.starts_with('-'),
            "nor for a flag"
        );
        assert!(
            marker.contains("decided"),
            "the marker earns its place by pointing at where the real sequence \
             lives; a bare `(routed)` would say what is missing without saying \
             where to look"
        );

        // And a routed record is distinguishable from an absent-field one on the
        // tolerant path the TUI actually reads through.
        let summary = run_summary_from_value(
            RUN_ID,
            &serde_json::json!({
                "started_at": "2026-08-19T00:00:00Z",
                "goal": "g",
                "gsd_command": marker,
                "ended_at": serde_json::Value::Null,
            }),
        );
        assert_eq!(summary.gsd_command, marker);
        assert_ne!(
            summary.gsd_command, "",
            "which is exactly what an absent field would have produced"
        );
    }

    #[test]
    fn a_bounds_reason_reaches_the_terminal_record_through_the_one_parked_prefix() {
        // The park event's `reason` doc now names four sanctioned taxonomies.
        // This proves the bounds one rides the same single field and the same
        // single `parked:` prefix as the envelope one — one carrier, not four.
        let dir = tempfile::tempdir().expect("temp dir");
        let planning = dir.path().join(".planning");
        let mut run = JournalRun::start(&planning, run_record(RUN_ID)).expect("start the run");

        let reason = crate::driver::bounds::REASON_NO_PROGRESS;
        run.record(&JournalEvent::Parked {
            reason: reason.to_string(),
            needs: "human".to_string(),
        })
        .expect("the park is journalled");
        run.finish(&format!("parked:{reason}"))
            .expect("finish the run");

        let stamped: RunRecord = serde_json::from_str(
            &std::fs::read_to_string(&run.paths().run_json).expect("read the record"),
        )
        .expect("the record deserialises");

        assert_eq!(
            stamped.outcome.as_deref(),
            Some("parked:bounds_no_progress"),
            "a bounds reason reaches disk through the `parked:` prefix unchanged \
             — no second carrier was invented for the run bounds"
        );
    }

    #[test]
    fn an_executor_stream_becomes_a_journal_a_reader_can_replay() {
        let dir = tempfile::tempdir().expect("temp dir");
        let planning = dir.path().join(".planning");
        let mut run = JournalRun::start(&planning, run_record(RUN_ID)).expect("start the run");

        let events = every_execution_event();
        assert!(
            events.len() >= 6,
            "the sequence must exercise at least six distinct variants, got {}",
            events.len()
        );
        for event in &events {
            run.record_exec(event).expect("journal the event");
        }
        run.finish("succeeded_with_changes").expect("finish");

        // Read it back through the READER's whole-file API, so the writer and
        // the reader are exercised against each other rather than each against
        // a hand-written fixture.
        let (records, diagnostics) =
            reader::read_all(&run.paths().journal).expect("read the journal back");

        assert_eq!(
            diagnostics.gaps,
            Vec::<(u64, u64)>::new(),
            "a journal written in one process has no sequence gaps (D-29)"
        );
        assert_eq!(diagnostics.unparseable, 0, "every line the writer wrote parses");
        assert_eq!(
            records.len(),
            events.len() + 2,
            "one record per event, plus run_started and run_ended"
        );
        assert_eq!(diagnostics.last_seq, Some(records.len() as u64));

        let kinds: Vec<&str> = records.iter().map(|r| r.kind.as_str()).collect();
        assert_eq!(
            kinds,
            vec![
                "run_started",
                "exec_started",     // SessionStarted
                "exec_event",       // Message
                "exec_event",       // Unknown
                "exec_event",       // Unparseable
                "diagnostic",       // LineTruncated
                "exec_event",       // Stderr
                "exec_event",       // TurnCompleted — a turn is NOT the run
                "cost",             // Cost
                "events_dropped",   // EventsDropped
                "exec_finished",    // Exited
                "run_ended",
            ],
            "the kinds must appear in the order the stream produced them"
        );

        // The mapping's own details, asserted rather than assumed.
        let started = &records[1];
        assert_eq!(
            started.rest["session_id"], "9f1c0e2a-0000-4000-8000-000000000000",
            "the session id is read off the event Phase 15 already emits (D-37)"
        );
        assert_eq!(
            started.rest["argv_digest"],
            serde_json::Value::String(run.record_document().argv_digest.clone()),
            "the argv digest is read off the run record"
        );
        assert_eq!(records[6].rest["stream"], "stderr");
        assert_eq!(records[7].rest["stream"], "turn_completed");
        assert_eq!(records[5].rest["code"], "line_truncated");

        let finished = &records[10];
        assert_eq!(finished.rest["exit"], 0);
        assert_eq!(
            finished.rest["cost_usd"], 1.83,
            "the run stamps its last observed NOTIONAL cost onto the ending"
        );
        assert!(finished.rest["duration_s"].is_u64());
    }

    #[test]
    fn a_dropped_event_count_reaches_the_journal() {
        // D-33: a run that silently lost 40 events is materially different from
        // one that lost none, and this count is the only signal that
        // distinguishes them. Until this phase it reached a `tracing::warn!` and
        // nothing else.
        let dir = tempfile::tempdir().expect("temp dir");
        let planning = dir.path().join(".planning");
        let mut run = JournalRun::start(&planning, run_record(RUN_ID)).expect("start the run");

        run.record_exec(&ExecutionEvent::EventsDropped { count: 40 })
            .expect("journal the drop report");

        let (records, _) = reader::read_all(&run.paths().journal).expect("read");
        let dropped = records
            .iter()
            .find(|record| record.kind == "events_dropped")
            .expect("the drop report reached the journal");
        assert_eq!(
            dropped.rest["count"], 40,
            "the numeric count in the file must equal the count fed in"
        );
    }

    #[test]
    fn every_reserved_kind_is_declared_and_none_is_emitted_by_this_phase() {
        // A mechanical guard that Phase 16 did not start emitting Phase 20's
        // vocabulary early (D-36). Asserting it by review would decay the first
        // time a variant is added to the mapping.
        let digest = argv_digest(&["claude".to_string()]);
        for event in every_execution_event() {
            let mapped = from_exec_event(&event, &digest)
                .unwrap_or_else(|| panic!("every variant maps today: {event:?}"));
            let kind = kind_of(&mapped);
            assert!(
                !RESERVED_KINDS.contains(&kind.as_str()),
                "{kind} is reserved for a later phase and must not be emitted here"
            );
            assert!(
                EMITTED_KINDS.contains(&kind.as_str()),
                "{kind} is emitted but is not declared in EMITTED_KINDS"
            );
        }

        // The lifecycle kinds this phase writes outside the mapping.
        for event in [
            JournalEvent::RunStarted {
                goal: "g".to_string(),
                dry_run: false,
                target: "host".to_string(),
            },
            JournalEvent::RunEnded {
                outcome: "completed".to_string(),
                ended_at: "2026-07-28T18:00:00Z".to_string(),
            },
            JournalEvent::JournalTruncated {
                bytes_written: 1,
                cap: 1,
            },
            // Phase 18's three. `interjected` moved out of RESERVED_KINDS in
            // the same edit that added it here; the two lists are complements
            // and this loop plus the one below is what proves it.
            JournalEvent::Interjected {
                id: Some("3f2a".to_string()),
                text: "skip the UI review".to_string(),
                delivered: true,
            },
            JournalEvent::InterjectionActedOn {
                id: "3f2a".to_string(),
            },
            JournalEvent::InterjectionMissed {
                id: "3f2a".to_string(),
                reason: "the agent's stdin was already closed".to_string(),
            },
            // Phase 19's. `parked` moved out of RESERVED_KINDS in the same edit
            // that added it here, exactly as `interjected` did above; D-24 gave
            // it its first emitter (every envelope refusal parks the run).
            JournalEvent::Parked {
                reason: "force_push_blocked".to_string(),
                needs: "human".to_string(),
            },
            // Phase 20's two, moved out of RESERVED_KINDS in the same edit that
            // added them here — the iteration loop records what it observed and
            // what the router decided, once per routed iteration. Exactly the
            // route `interjected` and `parked` took before them.
            JournalEvent::Observed {
                phase: "20".to_string(),
                drpev: vec![
                    "yes".to_string(),
                    "yes".to_string(),
                    "0".to_string(),
                    "0".to_string(),
                    "no".to_string(),
                ],
            },
            JournalEvent::Decided {
                by: "policy".to_string(),
                command: "/gsd-plan-phase 20".to_string(),
                rationale: "next".to_string(),
            },
        ] {
            let kind = kind_of(&event);
            assert!(EMITTED_KINDS.contains(&kind.as_str()), "{kind} undeclared");
            assert!(!RESERVED_KINDS.contains(&kind.as_str()));
        }

        // Every reserved kind is genuinely DECLARED in the schema — present as a
        // real variant a later phase can construct with no migration — rather
        // than being a string in a list nothing backs.
        //
        // **The list is empty as of Phase 20**, because every kind D-36 reserved
        // has now been drawn down by the phase it was reserved for. The loop
        // below therefore proves a vacuous property today, and it is kept
        // deliberately: the mechanism is what the next phase reserving a kind
        // ahead of its producer needs, and a guard deleted the moment its list
        // empties is a guard that has to be rediscovered.
        let declared: Vec<String> = Vec::new();
        for kind in RESERVED_KINDS {
            assert!(
                declared.contains(&(*kind).to_string()),
                "{kind} is reserved but no schema variant produces it"
            );
        }
        assert_eq!(declared.len(), RESERVED_KINDS.len());
        for kind in EMITTED_KINDS {
            assert!(
                !RESERVED_KINDS.contains(kind),
                "{kind} cannot be both emitted and reserved"
            );
        }
    }

    // ---- Enumerating the runs on disk (plan 18-03, Task 2; OBS-05) ----

    /// A finished run on disk, written through the real writer.
    fn plant_run(planning: &Path, run_id: &str, goal: &str, command: &str) {
        let mut record = run_record(run_id);
        record.goal = goal.to_string();
        record.gsd_command = command.to_string();
        let mut run = JournalRun::start(planning, record).expect("start the run");
        run.finish("succeeded_with_changes")
            .expect("finish the run");
    }

    #[test]
    fn an_absent_or_empty_runs_root_lists_nothing() {
        let dir = tempfile::tempdir().expect("temp dir");
        let planning = dir.path().join(".planning");

        assert!(
            list_runs(&planning).is_empty(),
            "a project that has never been driven has no runs and is not an anomaly"
        );

        std::fs::create_dir_all(runs_root(&planning)).expect("create the runs root");
        assert!(list_runs(&planning).is_empty());
    }

    #[test]
    fn a_traversing_run_id_is_refused_before_any_record_is_read() {
        let dir = tempfile::tempdir().expect("temp dir");
        let planning = dir.path().join(".planning");
        plant_run(&planning, RUN_ID, "a real run", "/gsd:execute-phase 18");

        // A perfectly valid record, planted OUTSIDE the runs root — exactly what
        // WR-02's read side reached. `read_dir` can never yield a traversing
        // name, so the refusal is only provable by calling the reader directly.
        let escaped = planning.join("escaped");
        std::fs::create_dir_all(&escaped).expect("create the escape directory");
        std::fs::write(
            escaped.join("run.json"),
            serde_json::to_string(&run_record("2026-07-28T14-03-99Z-ffff"))
                .expect("serialise the record"),
        )
        .expect("plant the record");

        for hostile in ["../escaped", "../../escaped", "./escaped", "a/b", "..", ""] {
            assert!(
                read_run_summary(&planning, hostile).is_none(),
                "{hostile:?} must be refused before anything is joined or read"
            );
        }

        // The positive control: the guard refuses traversal, not reading.
        assert!(
            read_run_summary(&planning, RUN_ID).is_some(),
            "a plain run id must still be read, or the guard passes by being broken"
        );
    }

    #[test]
    fn a_directory_with_no_record_is_skipped_without_hiding_its_neighbours() {
        let dir = tempfile::tempdir().expect("temp dir");
        let planning = dir.path().join(".planning");
        plant_run(&planning, RUN_ID, "a real run", "/gsd:execute-phase 18");

        // A run directory whose record never landed, and a stray file that is
        // not a directory at all.
        std::fs::create_dir_all(runs_root(&planning).join("2026-07-29T09-00-00Z-dead"))
            .expect("create the recordless directory");
        std::fs::write(runs_root(&planning).join("2026-07-29T10-00-00Z-file"), "")
            .expect("write the stray file");

        let runs = list_runs(&planning);
        assert_eq!(
            runs.iter()
                .map(|run| run.run_id.as_str())
                .collect::<Vec<_>>(),
            vec![RUN_ID],
            "one damaged entry must not hide the healthy run beside it"
        );
    }

    #[test]
    fn three_runs_come_back_newest_first() {
        let dir = tempfile::tempdir().expect("temp dir");
        let planning = dir.path().join(".planning");

        // Planted OUT of order, and the earliest stamp deliberately carries the
        // lexically largest suffix — a sort the random tail could influence
        // would fail here.
        let middle = "2026-07-28T14-03-12Z-0000";
        let oldest = "2026-07-28T14-03-11Z-ffff";
        let newest = "2026-07-29T00-00-00Z-0001";
        for run_id in [middle, newest, oldest] {
            plant_run(&planning, run_id, "g", "/gsd:progress");
        }

        let runs = list_runs(&planning);
        assert_eq!(
            runs.iter()
                .map(|run| run.run_id.as_str())
                .collect::<Vec<_>>(),
            vec![newest, middle, oldest],
            "lexicographic descending is chronological newest-first (new_run_id's format)"
        );

        // And the sort is a pure function over a slice, so it is assertable
        // without a filesystem at all.
        let mut shuffled = runs.clone();
        shuffled.reverse();
        sort_run_summaries_newest_first(&mut shuffled);
        assert_eq!(shuffled, runs);
    }

    #[test]
    fn a_records_goal_and_command_surface_verbatim() {
        let dir = tempfile::tempdir().expect("temp dir");
        let planning = dir.path().join(".planning");
        let goal = "ship the driver tab — 起動 🚀";
        plant_run(&planning, RUN_ID, goal, "/gsd:execute-phase 18");

        let runs = list_runs(&planning);
        let run = runs.first().expect("the planted run lists");
        assert_eq!(
            run.goal, goal,
            "the goal is stored verbatim, never paraphrased"
        );
        assert_eq!(run.gsd_command, "/gsd:execute-phase 18");
        assert_eq!(run.started_at, "2026-07-28T14:03:11Z");
        assert_eq!(
            run.outcome.as_deref(),
            Some("succeeded_with_changes"),
            "the outcome is the driver's derivation, never the agent's prose (D-13)"
        );
        assert!(run.ended_at.is_some(), "a finished run carries its ending");
    }

    #[test]
    fn a_record_from_an_unknown_schema_still_produces_a_row() {
        // D-30's tolerance applied to the list: one added required field must
        // not make every older run invisible in the UI.
        let summary = run_summary_from_value(
            RUN_ID,
            &serde_json::json!({
                "started_at": "2026-07-28T14:03:11Z",
                "goal": "g",
                "ended_at": serde_json::Value::Null,
                "a_field_from_2027": 1,
            }),
        );
        assert_eq!(summary.run_id, RUN_ID);
        assert_eq!(summary.goal, "g");
        assert_eq!(
            summary.gsd_command, "",
            "an absent field is empty, not fatal"
        );
        assert_eq!(summary.ended_at, None, "an explicit null is not an ending");
        assert_eq!(summary.outcome, None);
    }

    // ---- The readable text projection (plan 18-03, Task 1; OBS-04) ----

    fn stream_message(json: serde_json::Value) -> StreamMessage {
        serde_json::from_value(json).expect("the fixture is a valid stream message")
    }

    fn result_message(json: serde_json::Value) -> ResultMessage {
        serde_json::from_value(json).expect("the fixture is a valid result envelope")
    }

    /// The text a journalled `ExecEvent` actually carries, through the real
    /// mapping rather than by calling the projection directly.
    fn journalled_text(event: &ExecutionEvent) -> String {
        match from_exec_event(event, "fnv1a64:0000000000000000") {
            Some(JournalEvent::ExecEvent { text, .. }) => text,
            other => panic!("expected an exec_event, got: {other:?}"),
        }
    }

    #[test]
    fn a_text_only_turn_journals_the_agents_words_not_a_debug_struct() {
        let text = journalled_text(&ExecutionEvent::Message(Box::new(stream_message(
            serde_json::json!({
                "type": "assistant",
                "message": { "role": "assistant", "content": [{ "type": "text", "text": "reading src/driver/run.rs" }] },
                "session_id": "s",
            }),
        ))));
        assert_eq!(text, "assistant: reading src/driver/run.rs");
        assert!(
            !text.contains("TurnMessage {"),
            "a pane full of Debug structs satisfies \"watch its output\" only in the letter"
        );
    }

    #[test]
    fn a_tool_use_turn_keeps_a_visible_label_in_the_journal() {
        let text = journalled_text(&ExecutionEvent::Message(Box::new(stream_message(
            serde_json::json!({
                "type": "assistant",
                "message": { "role": "assistant", "content": [
                    { "type": "text", "text": "checking the version" },
                    { "type": "tool_use", "id": "toolu_01", "name": "Read", "input": { "file_path": "/tmp/Cargo.toml" } },
                ] },
            }),
        ))));
        assert_eq!(text, "assistant: checking the version\n[tool_use: Read]");
    }

    #[test]
    fn a_turn_with_no_message_body_journals_a_short_string_and_does_not_panic() {
        let text = journalled_text(&ExecutionEvent::Message(Box::new(stream_message(
            serde_json::json!({ "type": "assistant", "session_id": "s" }),
        ))));
        assert_eq!(text, "assistant:");
    }

    #[test]
    fn an_error_result_envelope_with_no_result_field_projects_without_panicking() {
        // `result` is ABSENT on error envelopes, not empty (Phase 15 D-32).
        let text = journalled_text(&ExecutionEvent::TurnCompleted(Box::new(result_message(
            serde_json::json!({
                "type": "result",
                "subtype": "error_max_budget_usd",
                "is_error": true,
                "terminal_reason": "budget_exhausted",
                "duration_ms": 3568,
                "total_cost_usd": 1.83,
                "errors": ["budget of $1.00 exhausted"],
            }),
        ))));
        assert_eq!(
            text,
            "turn ended: error_max_budget_usd (budget_exhausted) [error]; 3.6s this turn; \
             $1.83 cumulative, notional\nerror: budget of $1.00 exhausted"
        );
        assert!(
            !text.contains("ResultMessage {"),
            "the turn boundary must read as facts, not as a struct dump"
        );
    }

    #[test]
    fn a_success_result_envelope_carries_its_prose_as_content_and_nothing_more() {
        let text = journalled_text(&ExecutionEvent::TurnCompleted(Box::new(result_message(
            serde_json::json!({
                "type": "result",
                "subtype": "success",
                "duration_ms": 1804,
                "total_cost_usd": 0.12,
                "result": "PONG",
            }),
        ))));
        assert_eq!(
            text,
            "turn ended: success; 1.8s this turn; $0.12 cumulative, notional\nPONG"
        );
    }

    #[test]
    fn a_multibyte_turn_reaches_the_journal_byte_identically() {
        // The projection slices nothing, so there is no byte boundary to split.
        let words = "🚀 起動しました — ✅";
        let text = journalled_text(&ExecutionEvent::Message(Box::new(stream_message(
            serde_json::json!({
                "type": "assistant",
                "message": { "role": "assistant", "content": [{ "type": "text", "text": words }] },
            }),
        ))));
        assert_eq!(text, format!("assistant: {words}"));
    }

    #[test]
    fn the_projection_does_not_sanitise_because_the_journal_is_evidence() {
        // Terminal-control stripping is the RENDERER's, at buffer-append time:
        // the journal is read by tools other than that renderer, and stripping
        // at write time would destroy the record that the agent emitted these
        // bytes at all. This test pins the division of responsibility so a
        // later reader cannot "fix" it here by accident.
        let hostile = "\u{1b}[2Jall tests passed";
        let text = journalled_text(&ExecutionEvent::Message(Box::new(stream_message(
            serde_json::json!({
                "type": "assistant",
                "message": { "role": "assistant", "content": [{ "type": "text", "text": hostile }] },
            }),
        ))));
        assert!(
            text.contains('\u{1b}'),
            "the escape byte is evidence and must survive to disk: {text:?}"
        );
    }

    #[test]
    fn a_replayed_user_turn_is_named_as_a_replay_in_its_projection() {
        let text = journalled_text(&ExecutionEvent::Message(Box::new(stream_message(
            serde_json::json!({
                "type": "user",
                "isReplay": true,
                "message": { "role": "user", "content": [{ "type": "text", "text": "skip the UI review" }] },
            }),
        ))));
        assert_eq!(text, "user (replay): skip the UI review");
    }

    #[test]
    fn journal_events_serialise_with_a_snake_case_kind_tag() {
        let value = serde_json::to_value(JournalEvent::ExecEvent {
            stream: "assistant".to_string(),
            text: "hi".to_string(),
        })
        .expect("event serialises");
        assert_eq!(value["kind"], "exec_event");
        assert_eq!(value["stream"], "assistant");

        let value = serde_json::to_value(JournalEvent::JournalTruncated {
            bytes_written: MAX_RUN_JOURNAL_BYTES,
            cap: MAX_RUN_JOURNAL_BYTES,
        })
        .expect("event serialises");
        assert_eq!(value["kind"], "journal_truncated");
    }
}
