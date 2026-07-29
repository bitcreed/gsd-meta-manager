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

pub mod reader;
pub mod redact;
pub mod writer;

use std::path::{Component, Path, PathBuf};

use serde::{Deserialize, Serialize};

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
