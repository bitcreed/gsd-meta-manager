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
//!    opted into anywhere in this module tree, and its absence is grepped for as
//!    a mechanical guard — the same discipline `src/executor/stream_json.rs:6-8`
//!    records for the wire model. Phase 20 is a known future emitter of new
//!    `kind` values, so the reader's `kind` stays a plain `String`.
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

pub mod reader;
pub mod redact;
pub mod writer;

use std::path::{Component, Path, PathBuf};
use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::executor::stream_json::StreamMessage;
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

/// The five paths that make up one run's on-disk footprint.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunPaths {
    /// `<planning>/meta-manager/runs/<run-id>/`.
    pub dir: PathBuf,
    /// The committed run record (D-06, D-07).
    pub run_json: PathBuf,
    /// The gitignored append-only journal.
    pub journal: PathBuf,
    /// `<planning>/meta-manager/runs/.gitignore`, shared by every run (D-08).
    pub gitignore: PathBuf,
    /// `<planning>/meta-manager/runs/active`, the cheap-to-poll run-id pointer.
    pub active: PathBuf,
}

/// The root of a project's run directories.
pub fn runs_root(planning_dir: &Path) -> PathBuf {
    planning_dir.join("meta-manager").join("runs")
}

/// The five paths for one run, derived purely from the planning dir and run id.
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
pub fn run_paths(planning_dir: &Path, run_id: &str) -> RunPaths {
    let root = runs_root(planning_dir);
    let dir = root.join(run_id);
    RunPaths {
        run_json: dir.join("run.json"),
        journal: dir.join("journal.jsonl"),
        gitignore: root.join(".gitignore"),
        active: root.join("active"),
        dir,
    }
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

/// A non-cryptographic identity digest of a spawned command line.
///
/// FNV-1a 64 over `argv` joined by the ASCII unit separator, rendered as
/// `fnv1a64:` plus 16 lowercase hex digits. Two facts about it are deliberate:
///
/// - **It is not a security control.** It exists so two runs can be compared for
///   "same command line" in a `run.json`, nothing more. Nothing authenticates,
///   authorises or trusts anything on the strength of this value.
/// - **It is implemented inline because this phase adds zero dependencies.**
///   No hashing crate is present in `Cargo.toml` and the digest does not warrant
///   adding one.
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
    /// A message the user injected into a running agent.
    ///
    /// **Schema only in this phase — Phase 18 emits it** (D-36).
    Interjected {
        /// The injected text.
        text: String,
        /// Whether it reached the agent, as opposed to being queued or dropped.
        delivered: bool,
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
    /// **Schema only in this phase — Phase 20 emits it** (D-36).
    Parked {
        /// Why the run parked.
        reason: String,
        /// What would unpark it.
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
/// **Forward constraint for Phase 20:** when a run becomes a multi-invocation
/// loop, the terminal write must still happen after the *last* agent
/// invocation exits, never between steps. Per-step state belongs in the
/// journal, which is gitignored.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunRecord {
    /// The run id, matching the directory name.
    pub run_id: String,
    /// The originating goal prompt, verbatim — the reason D-07 commits this file.
    pub goal: String,
    /// The GSD command the run was started with.
    pub gsd_command: String,
    /// The rendered execution target.
    pub target: String,
    /// The user's driver opt-in record.
    ///
    /// `Option` because populating it is **Phase 17's** job; this phase writes
    /// the field so that phase adds no schema migration.
    pub opt_in: Option<String>,
    /// RFC3339 UTC timestamp of the first write.
    pub started_at: String,
    /// The agent session UUID.
    pub session_id: String,
    /// The driver process id.
    pub pid: u32,
    /// The driver process group id.
    pub pgid: u32,
    /// The `claude_code_version` observed at `system/init`.
    pub claude_code_version: String,
    /// [`argv_digest`] of the spawned command line.
    pub argv_digest: String,
    /// RFC3339 UTC timestamp of the terminal transition.
    ///
    /// **`None` is precisely the signal Phase 17's crash reconciliation reads**
    /// (D-06, D-32): a run directory whose record has no `ended_at` is a run
    /// that never reached a terminal transition, and retention must never prune
    /// it.
    pub ended_at: Option<String>,
    /// The derived run outcome, rendered. `None` until the terminal write.
    pub outcome: Option<String>,
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
    "exec_finished",
    "events_dropped",
    "journal_truncated",
    "diagnostic",
    "run_ended",
];

/// The `kind` values that exist in the schema but that **this phase never
/// writes** (D-36).
///
/// `observed`, `decided` and `parked` are Phase 20's — the D-R-P-E-V router
/// that does not exist yet. `interjected` is Phase 18's — the TUI→driver
/// channel that likewise does not exist yet. They are modelled now so those
/// phases add no schema migration, and the reader tolerates them regardless
/// (D-30): a build that has never heard of a kind still carries its payload.
pub const RESERVED_KINDS: &[&str] = &["observed", "decided", "parked", "interjected"];

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
            // Phase 17 owns dry-run; until it exists every run is a real one.
            // The field is written now so that phase adds no schema migration.
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
/// 1. **No serialisation derive is added to [`StreamMessage`] or any other
///    Phase 15 wire type in order to journal it.** The debug rendering is the
///    projection available *without* widening a type this phase is told not to
///    widen; it passes through the redactor like every other string (D-22); and
///    Phase 18 may well want a richer projection once it has a surface to render
///    one into. Reaching for `#[derive(Serialize)]` here would spend a
///    permanent constraint on a temporary convenience.
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
            text: format!("{message:?}"),
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
            text: format!("{result:?}"),
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
        let paths = run_paths(planning, "RID");
        assert_eq!(paths.dir, Path::new("/p/.planning/meta-manager/runs/RID"));
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

    #[test]
    fn only_content_events_report_themselves_as_content() {
        assert!(JournalEvent::ExecEvent {
            stream: "assistant".to_string(),
            text: "hi".to_string(),
        }
        .is_content());
        assert!(JournalEvent::Interjected {
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
    }

    // ---- The run lifecycle and the executor mapping (plan 16-06, Task 1) ----

    const RUN_ID: &str = "2026-07-28T14-03-11Z-a3f9";

    fn run_record(run_id: &str) -> RunRecord {
        RunRecord {
            run_id: run_id.to_string(),
            goal: "close the run journal phase".to_string(),
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
        ] {
            let kind = kind_of(&event);
            assert!(EMITTED_KINDS.contains(&kind.as_str()), "{kind} undeclared");
            assert!(!RESERVED_KINDS.contains(&kind.as_str()));
        }

        // Every reserved kind is genuinely DECLARED in the schema — present as a
        // real variant Phase 18 or 20 can construct with no migration — rather
        // than being a string in a list nothing backs.
        let declared: Vec<String> = [
            JournalEvent::Observed {
                phase: "14".to_string(),
                drpev: vec!["Complete".to_string()],
            },
            JournalEvent::Decided {
                by: "policy".to_string(),
                command: "/gsd:execute-phase 14".to_string(),
                rationale: "next".to_string(),
            },
            JournalEvent::Parked {
                reason: "verification_gaps_found".to_string(),
                needs: "human".to_string(),
            },
            JournalEvent::Interjected {
                text: "stop".to_string(),
                delivered: true,
            },
        ]
        .iter()
        .map(kind_of)
        .collect();
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
