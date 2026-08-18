//! The append-only NDJSON journal writer (OBS-01, D-03, D-04, D-29).
//!
//! One [`JournalWriter`] owns one run's `journal.jsonl` for the run's lifetime.
//! Four properties of this file are decisions, not incidental implementation:
//!
//! 1. **One record is one `write_all` of one buffer that already ends in `\n`.**
//!    Never a write of the line followed by a write of the newline: two calls
//!    reintroduce exactly the torn-line window that one call closes.
//! 2. **The handle stays open for the run.** Reopening per append costs an
//!    open/close syscall pair per event and buys nothing under `O_APPEND`.
//! 3. **No userspace buffering wrapper sits in front of the file.** Such a
//!    wrapper's entire effect would be to hold already-serialised events in the
//!    dying process's memory — which is precisely the data D-29 promises is on
//!    disk. There is likewise no per-event durability syscall; see the module
//!    root for the measurement that shows it buys nothing against process death.
//! 4. **`append` takes `&mut self`.** D-29's ordering contract — an event is
//!    written before the driver proceeds past the action it describes — is
//!    therefore satisfied *structurally*: exactly one owner can write at a time
//!    and records land in call order. A fire-and-forget spawn per event is the
//!    shape D-29 forbids, and exclusive `&mut` ownership is a stronger guarantee
//!    than an ordered channel, so no channel is introduced here for a producer
//!    that does not exist until Phase 17.
//!
//! One cosmetic consequence worth writing down so nobody "fixes" it:
//! `serde_json::Map` is a `BTreeMap` in this project — the ordering-preserving
//! feature is off — so a record's keys serialise alphabetically and a line reads
//! `kind`, `seq`, `ts`, then its payload rather than in the sketch order. That
//! is deliberate and entirely irrelevant to framing.

use std::fs::{File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::Context;
use serde_json::Value;
use tempfile::NamedTempFile;

use super::redact::RedactedLine;
use super::{run_paths, runs_root, JournalEvent, RunPaths, RunRecord, MAX_RUN_JOURNAL_BYTES};

/// The value [`JournalWriter::append`] returns for an event the per-run cap
/// suppressed (D-31).
///
/// Zero is unambiguous: `seq` starts at 1 and is monotonic, so no written
/// record ever carries it.
pub const SUPPRESSED_SEQ: u64 = 0;

/// An open append-only journal for one run.
pub struct JournalWriter {
    file: File,
    path: PathBuf,
    next_seq: u64,
    bytes_written: u64,
    cap: u64,
    truncated: bool,
    suppressed_content: u64,
}

impl JournalWriter {
    /// Open (creating if absent) the run's journal and keep the handle.
    ///
    /// The run directory must already exist; creating it is the run-start
    /// sequence's job ([`create_run_dir`]), which also writes the ignore file
    /// that protects this journal before a single byte of it is produced
    /// (D-08).
    pub fn open(journal_path: &Path) -> anyhow::Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(journal_path)
            .with_context(|| {
                format!("Failed to open journal at {}", journal_path.display())
            })?;

        Ok(Self {
            file,
            path: journal_path.to_path_buf(),
            next_seq: 1,
            bytes_written: 0,
            cap: MAX_RUN_JOURNAL_BYTES,
            truncated: false,
            suppressed_content: 0,
        })
    }

    /// Override the per-run byte cap.
    ///
    /// D-31 says the cap is a defensible starting value that is made
    /// configurable, and this is that seam. It is also how the tests drive the
    /// breach without writing 64 MiB, and how
    /// [`crash_test_writer_loop`] keeps producing bytes for its whole kill
    /// window.
    pub fn with_cap(mut self, cap: u64) -> Self {
        self.cap = cap;
        self
    }

    /// Append one event and return the `seq` it was written with, or
    /// [`SUPPRESSED_SEQ`] if the per-run cap suppressed it.
    ///
    /// **The asymmetry at the cap is the decision (D-31).** On the first append
    /// at or past [`MAX_RUN_JOURNAL_BYTES`] the writer emits exactly one
    /// [`JournalEvent::JournalTruncated`] and then stops writing *content*
    /// events — but keeps writing every lifecycle, decision, diagnostic and
    /// outcome event forever, because the run must always be able to write its
    /// terminal record, and silently dropping the ending is the one failure
    /// OBS-01 cannot tolerate.
    ///
    /// The per-*event* payload bound is a different failure mode and lives
    /// inside [`RedactedLine`]; it is deliberately not duplicated here.
    pub fn append(&mut self, event: &JournalEvent) -> anyhow::Result<u64> {
        if !self.truncated && self.bytes_written >= self.cap {
            // Set the flag first: the notice itself is not a content event, so
            // it writes through the raw path regardless, and the flag makes the
            // "exactly once" structural rather than a comment.
            self.truncated = true;
            let notice = JournalEvent::JournalTruncated {
                bytes_written: self.bytes_written,
                cap: self.cap,
            };
            self.write_event(&notice)?;
        }

        if self.truncated && event.is_content() {
            self.suppressed_content += 1;
            return Ok(SUPPRESSED_SEQ);
        }

        self.write_event(event)
    }

    /// Whether the per-run cap has been reached and announced.
    pub fn truncated(&self) -> bool {
        self.truncated
    }

    /// How many content events the cap has suppressed so far.
    ///
    /// A count is content-free by construction (D-28), so it is safe to carry
    /// anywhere — but it stays out of every log line's body and is emitted once
    /// at run end by [`Self::append_suppressed_diagnostic`].
    pub fn suppressed_content_events(&self) -> u64 {
        self.suppressed_content
    }

    /// Emit the suppressed-content count as a single [`JournalEvent::Diagnostic`]
    /// if it is non-zero. Returns the count it reported.
    ///
    /// The same kind of signal as plan 16-02's dropped-event count: a run that
    /// silently lost content is materially different from one that did not, and
    /// this is the only record that distinguishes them.
    pub fn append_suppressed_diagnostic(&mut self) -> anyhow::Result<Option<u64>> {
        if self.suppressed_content == 0 {
            return Ok(None);
        }
        let count = self.suppressed_content;
        self.append(&JournalEvent::Diagnostic {
            code: "journal_content_suppressed".to_string(),
            detail: format!("{count} content events suppressed after the per-run cap"),
        })?;
        Ok(Some(count))
    }

    /// Serialise, stamp, redact and write one event unconditionally.
    fn write_event(&mut self, event: &JournalEvent) -> anyhow::Result<u64> {
        let mut value =
            serde_json::to_value(event).context("Failed to render a journal event as JSON")?;

        let seq = self.next_seq;
        {
            let object = value
                .as_object_mut()
                .context("a journal event always serialises to a JSON object")?;
            object.insert(
                "ts".to_string(),
                Value::String(
                    chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Millis, true),
                ),
            );
            object.insert("seq".to_string(), Value::from(seq));
        }

        let line = RedactedLine::new(value);
        let mut buffer = String::with_capacity(line.as_line().len() + 1);
        buffer.push_str(line.as_line());
        buffer.push('\n');

        self.file
            .write_all(buffer.as_bytes())
            .with_context(|| format!("Failed to append to {}", self.path.display()))?;

        self.next_seq += 1;
        self.bytes_written += buffer.len() as u64;
        Ok(seq)
    }

    /// The `seq` the **next** append will use.
    ///
    /// Starts at 1 and increments monotonically, so a tailing reader can detect
    /// gaps; a gap is a reported diagnostic, never a parse failure (D-03).
    pub fn seq(&self) -> u64 {
        self.next_seq
    }

    /// Bytes this writer has appended. Plan 16-03 reads it for the per-run cap.
    pub fn bytes_written(&self) -> u64 {
        self.bytes_written
    }

    /// The journal path this writer holds open.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

// The body below is an **interface**, not an implementation detail: it is
// written into other people's repositories, changing it later does not
// retroactively update the files already written, and a user may already have
// committed one. Two facts the RESEARCH §1.1 transcript established against git
// 2.43.0 — both of which a reader would otherwise get wrong:
//
// 1. The directory re-inclusion is what lets the record re-inclusion reach
//    anything at all. Git will not re-include a file that lives inside an
//    excluded directory, so without un-excluding directories first, the
//    run-record rule matches nothing and D-07's goal-legibility rationale
//    evaporates silently.
// 2. A record one directory level deeper — `runs/<id>/nested/run.json` — stays
//    ignored, because the re-inclusion matches exactly one level. That is
//    desirable rather than a gap: only the per-run record at the documented
//    depth is meant to be committed.
/// The exact contents of `<planning>/meta-manager/runs/.gitignore` (D-08).
///
/// Six lines: two comments saying why the split exists, then the catch-all
/// exclusion, the directory re-inclusion, the ignore-file re-inclusion, and the
/// one-level run-record re-inclusion. Every one of the four patterns is
/// load-bearing; see the comment above this constant.
pub const RUNS_GITIGNORE_BODY: &str = "\
# Written by gsd-meta-manager. Driver transcripts are local-only; the per-run
# run.json (goal + outcome) is committed so the goal stays legible later.
*
!*/
!.gitignore
!*/run.json
";

/// Where our own ignore file sits relative to a project root, used to tell our
/// pattern apart from a driven repository's own (see
/// [`parent_excludes_run_record`]).
const OWN_GITIGNORE_SUFFIX: &str = "meta-manager/runs/.gitignore";

/// Write [`RUNS_GITIGNORE_BODY`] **only if the file is absent**.
///
/// Returns whether it wrote. Idempotent by construction, so the second run in a
/// project does not churn the file — and so a user who has deliberately edited
/// theirs keeps their edit.
///
/// Private on purpose: [`ensure_runs_root`] is the **only** entry point that
/// decides when writing it is correct (D-08), and both callers — the run
/// directory and the driver's lock file — go through it. Exposing this directly
/// would let a third caller create a protected byte without its protection.
fn write_runs_gitignore(gitignore_path: &Path) -> anyhow::Result<bool> {
    if gitignore_path.exists() {
        return Ok(false);
    }
    if let Some(parent) = gitignore_path.parent() {
        std::fs::create_dir_all(parent).with_context(|| {
            format!("Failed to create the runs root at {}", parent.display())
        })?;
    }
    std::fs::write(gitignore_path, RUNS_GITIGNORE_BODY).with_context(|| {
        format!(
            "Failed to write the runs ignore file at {}",
            gitignore_path.display()
        )
    })?;
    Ok(true)
}

/// Create the runs root with its ignore file already in place, and return it.
///
/// **The timing is the decision (D-08), and this function is where it is made
/// exactly once.** The ignore entry is written at run-*directory* creation time
/// and not at opt-in time, because a protected file that exists before its
/// protection is exactly the class of mistake SAFE-04 exists to prevent.
///
/// It became callable from outside `create_run_dir` for a concrete reason: the
/// driver's `flock` file (`runs/run.lock`, plan 17-02, D-19/D-20) is now the
/// **first** file to land in this directory, before any run directory exists.
/// If the lock file could be created without going through here, there would be
/// a window in which a file inside a driven repository is neither ignored nor
/// committed on purpose — the same window, one file earlier.
///
/// Idempotent: `create_dir_all` tolerates an existing root and
/// [`write_runs_gitignore`] writes only if the file is absent, so a user's own
/// edit of it survives.
pub fn ensure_runs_root(planning_dir: &Path) -> anyhow::Result<PathBuf> {
    let root = runs_root(planning_dir);

    // `create_dir_all` then write, the shape `save_queue` uses for
    // `meta-manager/` (`queue_md.rs:218-225`).
    std::fs::create_dir_all(&root)
        .with_context(|| format!("Failed to create the runs root at {}", root.display()))?;

    write_runs_gitignore(&root.join(".gitignore"))?;

    Ok(root)
}

/// Create one run's directory with its ignore posture already in place.
///
/// The two steps that used to live here — create the root, write the ignore
/// file — now live in [`ensure_runs_root`], which is called **first** so the
/// protection is on disk before the run directory it protects even exists.
/// There remains exactly one place that decides when the ignore file lands.
///
/// The parent-exclusion diagnostic runs here for the same reason it always did:
/// this is the one moment at which the layout is known and nothing has been
/// written into it yet. It never fails creation; the diagnostic behind it is
/// [`parent_excludes_run_record`].
///
/// **A run id that is not a single plain path component is refused here, before
/// anything is created** (D-27, WR-02). This is the write side of the traversal
/// hole, and the refusal is placed ahead of [`ensure_runs_root`] on purpose: a
/// hostile id must not so much as create the runs root, let alone a run
/// directory outside the project. The failure is loud — it propagates through
/// `JournalRun::start` into `DriveError::Journal` — rather than a silent skip,
/// because a run that quietly wrote nothing would look identical to a run that
/// worked.
pub fn create_run_dir(planning_dir: &Path, run_id: &str) -> anyhow::Result<RunPaths> {
    let Some(paths) = run_paths(planning_dir, run_id) else {
        anyhow::bail!(
            "the run id is not a single plain path component, so no run directory was created"
        );
    };

    ensure_runs_root(planning_dir)?;

    std::fs::create_dir_all(&paths.dir)
        .with_context(|| format!("Failed to create the run directory {}", paths.dir.display()))?;

    // The project root is `.planning/`'s parent. A planning dir with no parent
    // is not a shape this tool produces, and the diagnostic is optional anyway.
    if let Some(project_root) = planning_dir.parent() {
        warn_if_parent_excludes(project_root, &paths.run_json);
    }

    Ok(paths)
}

/// Whether a `.gitignore` **other than ours** will keep this run record out of
/// the index (RESEARCH §1.3).
///
/// **The polarity looks backwards on purpose.** `git check-ignore` answers
/// *"did a pattern match?"*, **not** *"is this file ignored?"*, and a negation
/// counts as a match — so a bare quiet-mode invocation exits 0 for a file our
/// own `!*/run.json` rule deliberately re-includes, reporting the precise
/// inverse of the truth. A first pass of RESEARCH §1.2 made exactly that
/// mistake before the ground-truth check corrected it. That is why this
/// function inspects the *reported pattern* rather than the exit status.
///
/// True only when a match is reported, from a different ignore file, by a
/// pattern with no leading negation marker. Everything else — git absent, a
/// non-repository, a non-zero exit, or a match from our own file — is `false`.
pub fn parent_excludes_run_record(project_root: &Path, run_json: &Path) -> bool {
    let output = match std::process::Command::new("git")
        .arg("check-ignore")
        .arg("-v")
        .arg("--")
        .arg(run_json)
        .current_dir(project_root)
        .output()
    {
        Ok(output) => output,
        // git is not installed, or is not executable here. Not our problem.
        Err(_) => return false,
    };

    // Exit 1 means no pattern matched at all; 128 means "not a repository".
    if !output.status.success() {
        return false;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let Some(line) = stdout.lines().next() else {
        return false;
    };
    // `-v` prints `<source>:<lineno>:<pattern>\t<pathname>`.
    let Some((reported, _pathname)) = line.split_once('\t') else {
        return false;
    };
    let mut fields = reported.splitn(3, ':');
    let (Some(source), Some(_lineno), Some(pattern)) =
        (fields.next(), fields.next(), fields.next())
    else {
        return false;
    };

    // Our own file re-including the record is the expected, healthy answer.
    if Path::new(source).ends_with(Path::new(OWN_GITIGNORE_SUFFIX)) {
        return false;
    }
    // A negation is a re-inclusion, which is the opposite of an exclusion.
    !pattern.starts_with('!')
}

/// Report — never fail on — a driven repository that has opted its own
/// `.planning/meta-manager/` out (RESEARCH §1.3).
///
/// Git does not descend into an excluded directory, so a nested ignore file
/// inside one never runs: the run record is then silently never committable and
/// D-07's goal-legibility rationale is lost in exactly the third-party
/// repositories this tool exists for. Detecting it costs one command.
///
/// **This is a diagnostic only.** It must never fail run-directory creation,
/// because the journal still works; the only thing lost is the committed
/// record. See [`parent_excludes_run_record`] for why the check reads the
/// reported pattern instead of the exit status — that predicate is the public
/// half of this pair, because a `tracing::warn!` cannot be asserted on from an
/// integration test.
fn warn_if_parent_excludes(project_root: &Path, run_json: &Path) {
    if parent_excludes_run_record(project_root, run_json) {
        tracing::warn!(
            "{} has excluded .planning/meta-manager/ in its own .gitignore: the run record \
             at {} will not be committable, so this run's goal will not survive a fresh clone",
            project_root.display(),
            run_json.display()
        );
    }
}

/// Write the `active` pointer: the run id and one newline.
///
/// **The directory listing is authoritative when the two disagree.** The
/// pointer exists because it is cheap to poll and useful to Phase 17's
/// reconciliation, but D-02's run-id format makes lexicographic sort equal
/// chronological sort, so a listing answers "which run is newest" without
/// reading anything — and a listing cannot go stale after a crash, whereas this
/// file can. That is the resolution of CONTEXT's open discretion on this file:
/// write it, but never trust it over the directory it names.
pub fn write_active_pointer(runs_root: &Path, run_id: &str) -> anyhow::Result<()> {
    std::fs::create_dir_all(runs_root)
        .with_context(|| format!("Failed to create the runs root at {}", runs_root.display()))?;
    let active = runs_root.join("active");
    std::fs::write(&active, format!("{run_id}\n"))
        .with_context(|| format!("Failed to write the active pointer at {}", active.display()))
}

/// Remove the `active` pointer if it is present.
///
/// A missing pointer is the normal steady state, so its absence is not an
/// error — this is idempotent for the same reason the ignore-file write is.
pub fn clear_active_pointer(runs_root: &Path) -> anyhow::Result<()> {
    let active = runs_root.join("active");
    match std::fs::remove_file(&active) {
        Ok(()) => Ok(()),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error)
            .with_context(|| format!("Failed to clear the active pointer at {}", active.display())),
    }
}

/// The run id the `active` pointer names, **only if that run directory exists**.
///
/// The existence check is the pointer's authority rule made mechanical: a
/// pointer naming a directory that is not there is stale — a crash between the
/// directory's removal and the pointer's clear, or a hand-edited file — and the
/// listing wins. A disagreement is logged, because it is a real anomaly even
/// though it is recoverable.
///
/// **The component check runs before the existence check, and the read side is
/// the half that matters more** (D-27, WR-02). The `active` file lives inside
/// the driven project, so **the agent controls it** — and this subsystem runs
/// unattended with git and push rights. The write side needs a hostile operator
/// or a hostile script; this side needs only the agent the user already asked to
/// run. `is_dir()` was the sole guard and a traversing path satisfies it
/// happily, after which `reconcile_one` read `run.json` from anywhere on the
/// filesystem and `App::schedule_journal_tail` tailed anything into a render
/// surface. Ordering the checks the other way would ask the filesystem about the
/// hostile path before refusing it, which is a smaller hole rather than none.
///
/// The signature does not change: this already answered `Option<String>`, and a
/// pointer that names something other than a run directory is exactly the "no
/// active run" this function is for.
pub fn read_active_run(planning_dir: &Path) -> Option<String> {
    let root = runs_root(planning_dir);
    let raw = std::fs::read_to_string(root.join("active")).ok()?;
    let run_id = raw.trim();
    if run_id.is_empty() {
        return None;
    }
    if !super::is_plain_path_component(run_id) {
        // The same register as the stale-pointer warning below, and content-free
        // for the same reason every log line in this tree is: the refused value
        // is the untrusted one (D-28).
        tracing::warn!(
            "the active pointer under {} names a path that is not a single directory \
             component; it is refused rather than followed",
            root.display()
        );
        return None;
    }
    if !root.join(run_id).is_dir() {
        tracing::warn!(
            "the active pointer under {} names a run directory that does not exist; \
             the directory listing is authoritative, so the pointer is ignored",
            root.display()
        );
        return None;
    }
    Some(run_id.to_string())
}

/// Write `run.json` atomically (D-05, D-06, D-07).
///
/// **This document is written exactly twice and nothing else ever rewrites
/// it.** Write one lands at run start, before any agent is spawned, carrying
/// the immutable facts: run id, goal prompt, GSD command, execution target, the
/// opt-in record, the start timestamp, session id, pid and pgid, the CLI version
/// and the argv digest. Write two lands at the terminal transition, after the
/// last agent has exited, adding the end timestamp and the derived outcome.
///
/// That immutability is what neutralises the Aider garbage-commit hazard
/// (D-07). The driven agent runs `git` inside the very worktree that holds this
/// file, so a record that changed on every status update would leave the
/// worktree perpetually dirty and let any `git add -A` the agent issues sweep a
/// **mid-run** snapshot into an unrelated commit, recording a running status in
/// history forever.
///
/// **Forward constraint for Phase 20:** when a run becomes a multi-invocation
/// loop, the terminal write must still happen after the *last* invocation
/// exits, never between steps. Per-step state belongs in the journal, which is
/// ignored.
///
/// Two deliberate departures from the `src/config.rs:71-87` idiom this mirrors:
/// the record is serialised *pretty*, because it is a document a human reads
/// rather than an NDJSON line; and under unix its mode is set to 0644 before
/// the persist, because `persist` preserves the temp file's 0600 and a second
/// uid on the same host — a container mount, a shared CI runner — otherwise
/// cannot read the one file whose entire purpose is being read by somebody else
/// later (RESEARCH §1.4).
///
/// `queue_md.rs`'s fixed-name-plus-rename write is deliberately **not** copied:
/// a fixed temporary name collides if two writers ever race, and RESEARCH §1.1
/// additionally measured that [`NamedTempFile`] produces a dot-prefixed
/// basename, which the ignore body's catch-all already covers — so an
/// interrupted persist leaves no untracked litter either.
pub fn write_run_record(paths: &RunPaths, record: &RunRecord) -> anyhow::Result<()> {
    let mut handle = NamedTempFile::new_in(&paths.dir).with_context(|| {
        format!(
            "Failed to create a temporary file for the run record in {}",
            paths.dir.display()
        )
    })?;

    let json = serde_json::to_string_pretty(record).context("Failed to serialize the run record")?;
    handle
        .write_all(json.as_bytes())
        .context("Failed to write the run record to its temporary file")?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        handle
            .as_file()
            .set_permissions(std::fs::Permissions::from_mode(0o644))
            .context("Failed to make the run record world-readable")?;
    }

    handle
        .persist(&paths.run_json)
        .with_context(|| format!("Failed to persist the run record to {}", paths.run_json.display()))?;

    Ok(())
}

/// Whether a directory name is a run id in D-02's format.
///
/// Parsing rather than pattern-matching: the stamp is fed to the same calendar
/// that produced it, so `2026-13-45T99-99-99Z-0000` is rejected and a directory
/// a user dropped in by hand is never mistaken for a run.
fn parses_as_run_id(name: &str) -> bool {
    let Some((stamp, suffix)) = name.rsplit_once('-') else {
        return false;
    };
    if suffix.len() != 4 || !suffix.chars().all(|c| c.is_ascii_alphanumeric()) {
        return false;
    }
    chrono::NaiveDateTime::parse_from_str(stamp, "%Y-%m-%dT%H-%M-%SZ").is_ok()
}

/// Whether this run directory's record carries an end timestamp.
///
/// Read as a `Value` rather than through [`RunRecord`] deliberately: a record
/// written by a later schema must still be prunable, and the only field this
/// question needs is `ended_at` (D-30's tolerance, applied to a write path).
/// A missing, unreadable or unparseable record answers `false`, which keeps it.
fn has_end_timestamp(run_dir: &Path) -> bool {
    let Ok(raw) = std::fs::read_to_string(run_dir.join("run.json")) else {
        return false;
    };
    let Ok(value) = serde_json::from_str::<Value>(&raw) else {
        return false;
    };
    value
        .get("ended_at")
        .map(|ended| !ended.is_null())
        .unwrap_or(false)
}

/// Keep the newest `retain` complete runs and remove the rest (D-32).
///
/// **The timing is the decision.** Pruning happens at **run start**, never at
/// run exit, because an exit-time prune is skipped by exactly the crash this
/// phase is built to survive.
///
/// Three runs are never touched, and the last two are the ones that matter:
///
/// - the **active** run, named by `active_run_id`;
/// - any run whose `run.json` is missing, unreadable, or carries **no end
///   timestamp** — that record is precisely what Phase 17's crash reconciliation
///   reads, so pruning it would destroy the evidence of the crash;
/// - any directory whose name is not a run id, which is somebody else's.
///
/// `retain` therefore counts *complete, inactive* runs; unfinished runs are
/// outside the budget rather than consuming it.
///
/// Sorting is lexicographic, which **equals** chronological by D-02's
/// construction — and that equality is the whole reason retention is a directory
/// listing rather than a parse of every record.
///
/// Returns the ids actually removed. A removal failure is a `tracing::warn!` and
/// never propagates: a failed prune must not stop a run from starting.
pub fn prune_runs(
    planning_dir: &Path,
    retain: usize,
    active_run_id: Option<&str>,
) -> anyhow::Result<Vec<String>> {
    let root = runs_root(planning_dir);
    let entries = match std::fs::read_dir(&root) {
        Ok(entries) => entries,
        // No runs root yet: the first run in this project has nothing to prune.
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(error) => {
            return Err(error)
                .with_context(|| format!("Failed to list the runs root at {}", root.display()))
        }
    };

    let mut candidates: Vec<String> = Vec::new();
    for entry in entries.filter_map(Result::ok) {
        if !entry.file_type().map(|kind| kind.is_dir()).unwrap_or(false) {
            continue;
        }
        let name = entry.file_name().to_string_lossy().into_owned();
        if !parses_as_run_id(&name) {
            continue;
        }
        if Some(name.as_str()) == active_run_id {
            continue;
        }
        if !has_end_timestamp(&root.join(&name)) {
            continue;
        }
        candidates.push(name);
    }
    candidates.sort();

    let mut removed = Vec::new();
    if candidates.len() > retain {
        let doomed = candidates.len() - retain;
        for run_id in candidates.into_iter().take(doomed) {
            let dir = root.join(&run_id);
            match std::fs::remove_dir_all(&dir) {
                Ok(()) => removed.push(run_id),
                Err(error) => {
                    tracing::warn!("Failed to prune run directory {}: {}", dir.display(), error)
                }
            }
        }
    }

    Ok(removed)
}

/// An Anthropic-shaped key the crash-test child plants. Planted, never real.
///
/// Exposed so the crash harness asserts against the value the child actually
/// wrote rather than a retyped copy that could drift out of agreement with it.
pub const CRASH_TEST_PLANTED_KEY: &str = "sk-ant-api03-AbCdEf012345_-XyZ";

/// Claude Code's dash-encoded session-directory form of a fictional user's home
/// — the WR-15 shape that historically slipped past a sweep checking only
/// `/home/<user>`. Planted, never real.
pub const CRASH_TEST_PLANTED_HOME: &str = "-home-fakeuser-projects-secretrepo";

/// **Test support only.** An unkillable-by-design writer loop for the crash
/// harness.
///
/// This is `pub` and deliberately **not** `#[cfg(test)]`, which is the whole
/// point: plan 16-06's crash test re-execs the integration-test binary and the
/// child must run the *real* writer, which no shell fixture can. A `cfg(test)`
/// function is invisible to that child.
///
/// It reaches the file through the ordinary [`JournalWriter::append`] path, so
/// it does **not** reopen the [`RedactedLine`] seam: the planted key and home
/// path below travel exactly the route a production event travels, which is why
/// asserting redaction on the bytes that survive a `SIGKILL` proves anything at
/// all (D-27).
///
/// The per-run cap is disabled here because the loop must keep producing bytes
/// for the entire kill window; a suppressed content event would spin the loop
/// with no I/O at all.
pub fn crash_test_writer_loop(journal_path: &Path) -> ! {
    let mut writer = match JournalWriter::open(journal_path) {
        Ok(writer) => writer.with_cap(u64::MAX),
        Err(error) => {
            eprintln!("crash-test child could not open the journal: {error}");
            std::process::exit(2);
        }
    };

    loop {
        let event = JournalEvent::ExecEvent {
            stream: "assistant".to_string(),
            text: format!("cwd {CRASH_TEST_PLANTED_HOME} key {CRASH_TEST_PLANTED_KEY} working"),
        };
        if let Err(error) = writer.append(&event) {
            eprintln!("crash-test child could not append: {error}");
            std::process::exit(3);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::journal::reader::{parse_line, tail_lines, ParsedLine, TailCursor};
    use crate::journal::run_paths;

    /// An Anthropic-shaped key. Planted, never real.
    const PLANTED_KEY: &str = "sk-ant-api03-AbCdEf012345_-XyZ";
    /// Claude Code's dash-encoded session-directory form of a fictional user's
    /// home. This is the WR-15 shape — the one that historically slipped past a
    /// sweep that checked only `/home/<user>`.
    const PLANTED_DASH_HOME: &str = "-home-fakeuser-projects-secretrepo";
    /// What the redactor must have put in their place, in the file's bytes.
    const EXPECTED_KEY_LITERAL: &str = "[REDACTED:anthropic-key]";
    const EXPECTED_HOME_LITERAL: &str = "-home-redacted-project";

    fn open_in(dir: &Path) -> (JournalWriter, PathBuf) {
        let paths = run_paths(&dir.join(".planning"), "2026-07-28T14-03-11Z-a3f9")
            .expect("a plain run id yields paths");
        std::fs::create_dir_all(&paths.dir).expect("create the run directory");
        let writer = JournalWriter::open(&paths.journal).expect("open the journal");
        (writer, paths.journal)
    }

    fn planted_event() -> JournalEvent {
        JournalEvent::ExecEvent {
            stream: "assistant".to_string(),
            text: format!("cwd {PLANTED_DASH_HOME} key {PLANTED_KEY} done"),
        }
    }

    #[test]
    fn a_planted_credential_is_already_redacted_in_the_file_bytes() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut writer, journal) = open_in(dir.path());
        writer.append(&planted_event()).expect("append");

        // The assertion that closes the loop reads the BYTES ON DISK (D-27).
        // An in-memory assertion is the same category of vacuous check as
        // grepping filtered `cargo` output for `warning:`.
        let bytes = std::fs::read_to_string(&journal).expect("read the journal back");

        assert!(
            !bytes.contains(PLANTED_KEY),
            "the planted credential survived to disk"
        );
        assert!(
            !bytes.contains(PLANTED_DASH_HOME),
            "the planted dash-encoded home path survived to disk"
        );
        assert!(bytes.contains(EXPECTED_KEY_LITERAL), "got: {bytes}");
        assert!(bytes.contains(EXPECTED_HOME_LITERAL), "got: {bytes}");

        // Then read it back the way a live tail would, from a zero cursor.
        let read = tail_lines(&journal, TailCursor::default()).expect("tail");
        assert_eq!(read.lines.len(), 1, "one appended event is one tailed line");
        assert_eq!(
            read.cursor.offset,
            journal.metadata().expect("stat").len(),
            "the cursor must advance to the end of a complete file"
        );
        assert!(!read.restarted);

        match parse_line(&read.lines[0]) {
            ParsedLine::Record(record) => {
                assert_eq!(record.seq, 1);
                assert_eq!(record.kind, "exec_event");
                assert_eq!(record.rest["stream"], "assistant");
                assert!(!record.ts.is_empty());
            }
            ParsedLine::Unparseable { raw, error } => {
                panic!("the writer produced an unparseable line: {error} in {raw}")
            }
        }
    }

    #[test]
    fn every_appended_record_is_one_complete_line_with_one_newline() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut writer, journal) = open_in(dir.path());

        writer.append(&planted_event()).expect("append");
        let bytes = std::fs::read_to_string(&journal).expect("read");
        assert_eq!(bytes.matches('\n').count(), 1, "exactly one newline");
        assert!(bytes.ends_with('\n'), "the record is newline-terminated");
        assert!(
            !bytes.trim_end_matches('\n').contains('\n'),
            "no embedded newline may break the NDJSON framing"
        );

        // Three more, including one whose payload is full of newlines and
        // carriage returns: compact serialisation escapes them, so framing
        // cannot be forged from a payload.
        writer
            .append(&JournalEvent::ExecEvent {
                stream: "user".to_string(),
                text: "line one\nline two\r\n{\"kind\":\"forged\"}".to_string(),
            })
            .expect("append");
        writer
            .append(&JournalEvent::EventsDropped { count: 40 })
            .expect("append");
        writer
            .append(&JournalEvent::RunEnded {
                outcome: "completed".to_string(),
                ended_at: "2026-07-28T14:10:00Z".to_string(),
            })
            .expect("append");

        let bytes = std::fs::read_to_string(&journal).expect("read");
        assert_eq!(bytes.matches('\n').count(), 4);
        assert!(bytes.ends_with('\n'));
        assert!(!bytes.contains("\n\n"), "no blank lines (D-03)");
        for line in bytes.lines() {
            serde_json::from_str::<Value>(line).expect("every written line reparses as JSON");
        }
        assert!(
            !bytes.contains(r#"{"kind":"forged"}"#),
            "an embedded object literal must stay escaped inside its string"
        );
    }

    #[test]
    fn seq_starts_at_one_and_increments_monotonically() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut writer, journal) = open_in(dir.path());

        assert_eq!(writer.seq(), 1, "the first append uses seq 1");
        for expected in 1..=5u64 {
            assert_eq!(writer.append(&planted_event()).expect("append"), expected);
        }
        assert_eq!(writer.seq(), 6);
        assert!(writer.bytes_written() > 0);

        let bytes = std::fs::read_to_string(&journal).expect("read");
        assert_eq!(
            writer.bytes_written(),
            bytes.len() as u64,
            "the running byte count must match the file"
        );
        let seqs: Vec<u64> = bytes
            .lines()
            .map(|line| match parse_line(line) {
                ParsedLine::Record(record) => record.seq,
                ParsedLine::Unparseable { error, .. } => panic!("unparseable: {error}"),
            })
            .collect();
        assert_eq!(seqs, vec![1, 2, 3, 4, 5]);
    }

    // ---- Layout, ignore posture and the run record (plan 16-03, Task 1) ----

    const RID: &str = "2026-07-28T14-03-11Z-a3f9";

    fn sample_record(run_id: &str) -> RunRecord {
        RunRecord {
            run_id: run_id.to_string(),
            goal: "ship the run journal".to_string(),
            gsd_command: "/gsd:execute-phase".to_string(),
            target: "claude".to_string(),
            opt_in: None,
            started_at: "2026-07-28T14:03:11Z".to_string(),
            session_id: "9f1c0e2a-0000-4000-8000-000000000000".to_string(),
            pid: 4242,
            pgid: 4242,
            claude_code_version: "2.1.0".to_string(),
            argv_digest: crate::journal::argv_digest(&["claude".to_string()]),
            ended_at: None,
            outcome: None,
        }
    }

    #[test]
    fn the_ignore_body_is_exactly_the_four_verified_patterns() {
        // The literal lives HERE, not in a reference to the constant: a change
        // to the constant must fail this test, and a test that compared the
        // constant with itself could not do that.
        let expected = ["*", "!*/", "!.gitignore", "!*/run.json"];

        let patterns: Vec<&str> = RUNS_GITIGNORE_BODY
            .lines()
            .filter(|line| !line.starts_with('#'))
            .collect();
        assert_eq!(
            patterns, expected,
            "the ignore body must match D-08 character for character"
        );
        assert_eq!(
            RUNS_GITIGNORE_BODY.lines().count(),
            6,
            "two comment lines plus four patterns"
        );
        assert!(
            RUNS_GITIGNORE_BODY.ends_with('\n'),
            "the file must end in a newline like every other text file"
        );
    }

    #[test]
    fn the_ignore_file_is_written_once_and_not_rewritten() {
        let dir = tempfile::tempdir().expect("temp dir");
        let planning = dir.path().join(".planning");

        let paths = create_run_dir(&planning, RID).expect("create the run directory");
        assert!(paths.dir.is_dir(), "the run directory must exist");
        assert!(paths.gitignore.is_file(), "the ignore file must exist");
        let first = std::fs::read_to_string(&paths.gitignore).expect("read the ignore file");
        assert_eq!(first, RUNS_GITIGNORE_BODY);
        let first_mtime = paths
            .gitignore
            .metadata()
            .expect("stat")
            .modified()
            .expect("mtime");

        // A hand edit, then a second run in the same project.
        std::fs::write(&paths.gitignore, "# edited by the user\n*\n!*/\n!*/run.json\n")
            .expect("edit the ignore file");
        let edited = std::fs::read_to_string(&paths.gitignore).expect("read");
        let edited_mtime = paths
            .gitignore
            .metadata()
            .expect("stat")
            .modified()
            .expect("mtime");

        assert!(
            !write_runs_gitignore(&paths.gitignore).expect("second write attempt"),
            "a present ignore file must not be rewritten"
        );
        assert_eq!(
            std::fs::read_to_string(&paths.gitignore).expect("read"),
            edited,
            "the user's edit must survive a second run"
        );
        assert_eq!(
            paths
                .gitignore
                .metadata()
                .expect("stat")
                .modified()
                .expect("mtime"),
            edited_mtime,
            "an unwritten file must not be touched"
        );
        let _ = first_mtime;

        // And a second create_run_dir for a different run is likewise quiet.
        let other = create_run_dir(&planning, "2026-07-29T01-00-00Z-b111")
            .expect("create a second run directory");
        assert_eq!(other.gitignore, paths.gitignore, "one file serves every run");
        assert_eq!(
            std::fs::read_to_string(&paths.gitignore).expect("read"),
            edited
        );
    }

    #[test]
    fn a_run_record_is_written_atomically_and_is_world_readable() {
        let dir = tempfile::tempdir().expect("temp dir");
        let planning = dir.path().join(".planning");
        let paths = create_run_dir(&planning, RID).expect("create the run directory");

        let record = sample_record(RID);
        write_run_record(&paths, &record).expect("write one");

        let raw = std::fs::read_to_string(&paths.run_json).expect("read the record back");
        let round_tripped: RunRecord = serde_json::from_str(&raw).expect("the record deserialises");
        assert_eq!(round_tripped.run_id, record.run_id);
        assert_eq!(round_tripped.goal, record.goal);
        assert_eq!(round_tripped.ended_at, None);
        assert!(
            raw.contains('\n'),
            "the record is a document, so it is pretty-printed"
        );

        // Write two: the terminal transition. Same document, stamped.
        let mut ended = record.clone();
        ended.ended_at = Some("2026-07-28T18:00:00Z".to_string());
        ended.outcome = Some("completed".to_string());
        write_run_record(&paths, &ended).expect("write two");
        let round_tripped: RunRecord =
            serde_json::from_str(&std::fs::read_to_string(&paths.run_json).expect("read"))
                .expect("the record deserialises");
        assert_eq!(round_tripped.ended_at.as_deref(), Some("2026-07-28T18:00:00Z"));

        // No temporary litter survives either write.
        let stray: Vec<String> = std::fs::read_dir(&paths.dir)
            .expect("list the run directory")
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| name != "run.json")
            .collect();
        assert!(stray.is_empty(), "a persisted write leaves nothing behind: {stray:?}");

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            // `persist` preserves the temp file's 0600, which would make the one
            // file whose purpose is being read later unreadable to a second uid
            // (RESEARCH §1.4). This assertion is what stops that returning.
            let mode = paths.run_json.metadata().expect("stat").permissions().mode();
            assert_eq!(mode & 0o777, 0o644, "got mode {:o}", mode & 0o777);
        }
    }

    #[test]
    fn the_active_pointer_is_written_cleared_and_never_trusted_over_the_listing() {
        let dir = tempfile::tempdir().expect("temp dir");
        let planning = dir.path().join(".planning");
        let paths = create_run_dir(&planning, RID).expect("create the run directory");
        let root = crate::journal::runs_root(&planning);

        assert_eq!(read_active_run(&planning), None, "no pointer yet");

        write_active_pointer(&root, RID).expect("write the pointer");
        assert_eq!(
            std::fs::read_to_string(&paths.active).expect("read"),
            format!("{RID}\n"),
            "the id and exactly one newline"
        );
        assert_eq!(read_active_run(&planning).as_deref(), Some(RID));

        // A pointer naming a directory that is not there is stale, and the
        // listing wins.
        std::fs::remove_dir_all(&paths.dir).expect("remove the run directory");
        assert_eq!(
            read_active_run(&planning),
            None,
            "a stale pointer must not be believed"
        );

        clear_active_pointer(&root).expect("clear");
        assert!(!paths.active.exists());
        clear_active_pointer(&root).expect("clearing twice is not an error");
    }

    // ---- Growth bounds and retention (plan 16-03, Task 2) ----

    /// A journal opened with a test-sized cap, so the breach is reachable
    /// without writing 64 MiB.
    fn open_capped(dir: &Path, cap: u64) -> (JournalWriter, PathBuf) {
        let paths = run_paths(&dir.join(".planning"), RID).expect("a plain run id yields paths");
        std::fs::create_dir_all(&paths.dir).expect("create the run directory");
        let writer = JournalWriter::open(&paths.journal)
            .expect("open the journal")
            .with_cap(cap);
        (writer, paths.journal)
    }

    fn kinds_in(journal: &Path) -> Vec<String> {
        std::fs::read_to_string(journal)
            .expect("read the journal")
            .lines()
            .map(|line| match parse_line(line) {
                ParsedLine::Record(record) => record.kind,
                ParsedLine::Unparseable { error, .. } => panic!("unparseable: {error}"),
            })
            .collect()
    }

    #[test]
    fn the_per_run_cap_emits_one_truncation_notice_then_only_lifecycle_events() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut writer, journal) = open_capped(dir.path(), 200);

        while writer.bytes_written() < 200 {
            writer.append(&planted_event()).expect("append");
        }
        assert!(
            !writer.truncated(),
            "the cap fires on the next append, not on the one that filled it"
        );

        // Ten further content events, every one of them past the cap.
        for _ in 0..10 {
            assert_eq!(
                writer.append(&planted_event()).expect("append"),
                SUPPRESSED_SEQ,
                "a suppressed event consumes no seq"
            );
        }
        assert!(writer.truncated());
        assert_eq!(writer.suppressed_content_events(), 10);

        // Lifecycle, diagnostic and outcome events keep flowing regardless.
        writer
            .append(&JournalEvent::EventsDropped { count: 40 })
            .expect("append");
        assert_eq!(
            writer.append_suppressed_diagnostic().expect("diagnostic"),
            Some(10)
        );

        let kinds = kinds_in(&journal);
        assert_eq!(
            kinds.iter().filter(|kind| *kind == "journal_truncated").count(),
            1,
            "exactly one truncation notice, not merely at least one: {kinds:?}"
        );
        let notice_at = kinds
            .iter()
            .position(|kind| kind == "journal_truncated")
            .expect("the notice is present");
        assert!(
            !kinds[notice_at + 1..].iter().any(|kind| kind == "exec_event"),
            "no content event may follow the notice: {kinds:?}"
        );
        assert!(kinds[notice_at + 1..].iter().any(|kind| kind == "events_dropped"));
        assert!(kinds[notice_at + 1..].iter().any(|kind| kind == "diagnostic"));

        // The notice carries the numbers a reader would otherwise have to guess.
        let notice: Value = serde_json::from_str(
            std::fs::read_to_string(&journal)
                .expect("read")
                .lines()
                .nth(notice_at)
                .expect("the notice line"),
        )
        .expect("the notice parses");
        assert_eq!(notice["cap"], 200);
        assert!(notice["bytes_written"].as_u64().expect("a number") >= 200);
    }

    #[test]
    fn a_terminal_record_is_still_written_after_the_cap_is_reached() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut writer, journal) = open_capped(dir.path(), 200);

        while writer.bytes_written() < 200 {
            writer.append(&planted_event()).expect("append");
        }
        for _ in 0..5 {
            writer.append(&planted_event()).expect("append");
        }
        writer
            .append(&JournalEvent::RunEnded {
                outcome: "completed".to_string(),
                ended_at: "2026-07-28T18:00:00Z".to_string(),
            })
            .expect("append the terminal record");

        let bytes = std::fs::read_to_string(&journal).expect("read");
        let last = bytes.lines().next_back().expect("a last line");
        match parse_line(last) {
            ParsedLine::Record(record) => {
                assert_eq!(record.kind, "run_ended", "the ending must be the last line");
                assert_eq!(record.rest["outcome"], "completed");
            }
            ParsedLine::Unparseable { error, .. } => panic!("unparseable: {error}"),
        }
    }

    fn make_run(planning: &Path, run_id: &str, ended: bool) -> RunPaths {
        let paths = create_run_dir(planning, run_id).expect("create the run directory");
        let mut record = sample_record(run_id);
        if ended {
            record.ended_at = Some("2026-07-28T18:00:00Z".to_string());
            record.outcome = Some("completed".to_string());
        }
        write_run_record(&paths, &record).expect("write the record");
        paths
    }

    const R1: &str = "2026-07-28T14-03-11Z-a3f9";
    const R2: &str = "2026-07-28T15-00-00Z-b111";
    const R3: &str = "2026-07-28T16-00-00Z-c222";
    const R4: &str = "2026-07-29T01-00-00Z-d333";
    const R5: &str = "2026-07-30T02-00-00Z-e444";

    #[test]
    fn pruning_keeps_the_newest_runs_and_never_the_active_one() {
        let dir = tempfile::tempdir().expect("temp dir");
        let planning = dir.path().join(".planning");
        for run_id in [R1, R2, R3, R4, R5] {
            make_run(&planning, run_id, true);
        }
        // A directory that is not a run, to prove the listing is filtered.
        let stranger = crate::journal::runs_root(&planning).join("notes");
        std::fs::create_dir_all(&stranger).expect("create a stranger directory");

        // The OLDEST run is the active one, so a prune that ignored the active
        // id would remove it first.
        let removed = prune_runs(&planning, 2, Some(R1)).expect("prune");
        assert_eq!(
            removed,
            vec![R2.to_string(), R3.to_string()],
            "the return value must name exactly what was removed"
        );

        let root = crate::journal::runs_root(&planning);
        assert!(root.join(R1).is_dir(), "the active run is never pruned");
        assert!(!root.join(R2).is_dir());
        assert!(!root.join(R3).is_dir());
        assert!(root.join(R4).is_dir(), "the newest survivors are kept");
        assert!(root.join(R5).is_dir());
        assert!(stranger.is_dir(), "a directory that is not a run is not ours");
        assert!(root.join(".gitignore").is_file(), "the ignore file survives");

        // Idempotent: a second prune with the same budget removes nothing.
        assert!(prune_runs(&planning, 2, Some(R1))
            .expect("prune again")
            .is_empty());
    }

    #[test]
    fn pruning_never_removes_a_run_with_no_end_timestamp() {
        let dir = tempfile::tempdir().expect("temp dir");
        let planning = dir.path().join(".planning");
        make_run(&planning, R1, true);
        // R2 crashed: its record was written at run start and never stamped.
        make_run(&planning, R2, false);
        make_run(&planning, R3, true);
        make_run(&planning, R4, true);
        // R5 has no record at all — equally unreconciled, equally untouchable.
        create_run_dir(&planning, R5).expect("create a recordless run");

        let removed = prune_runs(&planning, 1, None).expect("prune");
        assert_eq!(removed, vec![R1.to_string(), R3.to_string()]);

        let root = crate::journal::runs_root(&planning);
        assert!(
            root.join(R2).is_dir(),
            "a run with no end timestamp is Phase 17's crash evidence"
        );
        assert!(root.join(R5).is_dir(), "a run with no record at all is likewise kept");
        assert!(root.join(R4).is_dir(), "the newest complete run is retained");
        assert!(!root.join(R1).is_dir());
        assert!(!root.join(R3).is_dir());
    }

    #[test]
    fn pruning_a_project_with_no_runs_root_is_not_an_error() {
        let dir = tempfile::tempdir().expect("temp dir");
        let planning = dir.path().join(".planning");
        assert!(prune_runs(&planning, 10, None)
            .expect("prune a project that has never run")
            .is_empty());
    }

    #[test]
    fn only_a_d02_shaped_directory_name_parses_as_a_run_id() {
        assert!(parses_as_run_id("2026-07-28T14-03-11Z-a3f9"));
        assert!(!parses_as_run_id("2026-13-45T99-99-99Z-0000"), "not a calendar date");
        assert!(!parses_as_run_id("notes"));
        assert!(!parses_as_run_id("2026-07-28T14-03-11Z"), "no uuid suffix");
        assert!(!parses_as_run_id("2026-07-28T14:03:11Z-a3f9"), "colons are not the format");
    }
}
