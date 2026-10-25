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
//! **Phase 31 trimmed this module to what live code still reads.** The legacy
//! `drive` run lister, the plan-approval digest, the stdin inbox and the
//! executor-event projection went with the code that called them. What stays is
//! the change classifier the watcher uses, the path and digest helpers the
//! registry and the envelope use, the redactor, the tail reader the Driver tab
//! uses, and the writer, [`RunPaths`], [`JournalEvent`], [`RunRecord`] and
//! [`JournalRun`] that the envelope's park path still touches. Those last items
//! are deleted together with the envelope guard they serve.
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
//!   adds no migration (D-36).

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
///
/// **Two shapes it used to accept, and the correction rides the commit that
/// falsified the old text** (per the `src/driver/dry_run.rs:78-83` precedent):
///
/// * **Whitespace-only.** Only the *empty* string was short-circuited, so
///   `"   "` yielded exactly one [`Component::Normal`] whose text was the whole
///   of the value and the predicate returned `true`. `--run-id '   '` therefore
///   passed the seam and named a run directory made of spaces. Worse, the
///   round-3 review cited this predicate as the reason a blank `--target-phase`
///   needed no check of its own — a justification that was simply false, and
///   the exemption it bought is what let round-4's Critical through.
/// * **Embedded control characters.** `"a\nEVIL"` is one `Normal` component, so
///   it passed, and the dry-run render then printed the newline verbatim inside
///   what reads as a pasteable command line.
/// * **Zero-width and format characters only**, which `str::trim` cannot see. A
///   value of one `U+200B` passed every check above and was reproduced end to
///   end by pass 5: a **complete run** whose directory name was one invisible
///   character, and two visually identical registry aliases resolving to two
///   different envelope paths. The blank half of this predicate now delegates to
///   [`crate::text::carries_visible_content`] — the one production spelling of
///   the judgment — instead of carrying a second, weaker one of its own. Refused
///   as of that commit; the correction rides it.
/// * **EMBEDDED invisible formatting** — a value that renders exactly like a
///   sibling and carries different bytes. The shape above closes values that are
///   *wholly* invisible; this one carries visible content too, so
///   [`crate::text::carries_visible_content`] accepts it **by construction** and
///   the blank half above can never see it. Pass 5 named the harm ("two visually
///   identical aliases resolve to two different envelope and credential paths"),
///   round 5 claimed to close it with the emptiness judgment alone, and pass 6
///   reproduced it unchanged: `"demo"` and `"demo\u{200b}"` both accepted, two
///   envelope roots rendering as one; `"…-aaaa"` and `"…-aaaa\u{200b}"` both
///   accepted, two run directories, one rendering — a terminal record no human
///   can tell from its sibling. Refused as of D-17-1 by delegating to
///   [`crate::text::carries_invisible_formatting`], the second judgment over the
///   same class spelling; the correction rides that commit.
///
/// * **ANY value outside the identity alphabet** — the fifth shape, and the one
///   that subsumes every character shape above it by construction (D-19-2,
///   Premise 8). The four corrections above were all deny-list widenings, and
///   this phase widened the deny-list at three successive levels while each new
///   level stayed a subset: pass 7 measured `"demo\u{202e}"`, `"2\u{202e}0"`,
///   `"2\u{e0041}0"` and `"2\u{ad}0"` all ACCEPTED here, and `run_paths` handing
///   back a directory for them. An identity needs no script at all, so the
///   direction inverts: the accepted set is the finite, printable
///   `[A-Za-z0-9._-]` ([`crate::text::is_identity_char`]), which cannot be one
///   code point short. Bidi controls, the tag block, variation selectors and
///   homoglyphs all fall out of one clause. The recorded product trade lives on
///   `is_identity_char`.
///
/// All five are now refused. None was ever a legitimate run id, alias, or phase
/// name; every value the suite pins as acceptable (`"20"`, `"2.1"`, real
/// timestamped run ids, registered aliases) still passes.
///
/// **The clause layering, stated honestly rather than tidied.** The blank,
/// control and invisible-formatting clauses below are now REDUNDANT for
/// refusal — nothing they reject survives the alphabet clause. They are kept
/// anyway, and not out of caution: message-bearing consumers read them
/// (`crate::registry::Alias::new` gives `NotVisible` and `InvisibleFormatting`
/// their own honest wording by asking the same questions in the same order), and
/// the two-judgments drift pins in `crate::text` assert on them directly.
/// Deleting them would couple those consumers to this function's internals,
/// which is the coupling this module was split up to remove.
///
/// **The named trade (D-17-1).** An identity may not carry ZWJ/ZWNJ, which are
/// load-bearing in some scripts. Free text — goals, commands — is unaffected,
/// because only identity seams consult this predicate; a directory whose *name*
/// carries format characters registers under an explicitly chosen alias.
pub fn is_plain_path_component(value: &str) -> bool {
    // Blank rather than merely empty, and judged by the ONE production spelling
    // of blankness rather than by a `trim` of this function's own. A component
    // with nothing visible in it names nothing a reader could identify, and it
    // reached a run directory.
    if !crate::text::carries_visible_content(value) {
        return false;
    }
    // A control character has no business in a path component, and one that
    // reaches a rendered preview breaks the line the preview appears to be.
    //
    // **This is a SEPARATE question from the one above and is kept.** Visibility
    // asks whether anything can be seen at all; this asks whether a control
    // character is EMBEDDED in otherwise-visible text — `"a\nEVIL"` carries
    // plenty of visible content and is still not a name.
    if value.chars().any(|c| c.is_control()) {
        return false;
    }
    // Identity, not emptiness — a THIRD question, and the one the two above
    // structurally cannot ask. Both of them are satisfied by `"demo\u{200b}"`:
    // it carries visible content and no control character. What it also carries
    // is a byte that renders as nothing, so it names the same thing on screen as
    // `"demo"` while naming a different directory on disk (D-17-1).
    if crate::text::carries_invisible_formatting(value) {
        return false;
    }
    // The FINITE direction, and the one that structurally cannot be one code
    // point short (D-19-2). Everything above this line is a deny-list; this is
    // the allow-list. It subsumes them all for refusal purposes — see the doc
    // above for why they are nonetheless kept.
    if !value.chars().all(crate::text::is_identity_char) {
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
        gitignore: root.join(".gitignore"),
        active: root.join("active"),
        dir,
    })
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

/// What kind of write a watched path change represents (D-11).
///
/// `Hash` is load-bearing, not decorative: the watcher's debounce dedup becomes
/// per-`(root, classification)` rather than per-root, and this value is half of
/// that key (D-10).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum ChangeKind {
    /// A change to a launcher run's own files under
    /// `.planning/meta-manager/drivers/<run-id>/` (30-01, D-24): the run directory
    /// itself appearing or disappearing, its `DIRECTION.md`, or its `LOG.md`.
    /// Routes to a launcher-run refresh for that run, never to a project re-parse
    /// (OBS-06): the launcher writes these files often, and none of them is
    /// planning state.
    DriverRun {
        /// The run directory's name, which is the run id.
        run_id: String,
    },
    /// A path under `.planning/meta-manager/drivers/` that no consumer reads: the
    /// splice temps `.DIRECTION.md.fm.*`, `.DIRECTION.md.edit`, `state.json`,
    /// `launcher.lock`, `DIRECTION.lock`, editor swap files, anything deeper, and
    /// the bare `drivers` directory itself. Dropped before an action is built
    /// (T-30-03): the splice alone rewrites its temp and `state.json` on every
    /// heartbeat, and a re-parse per heartbeat is the denial of service this
    /// variant exists to stop.
    ///
    /// **Also every path at or below `.planning/meta-manager/runs/`** (Phase 31, D-03):
    /// the legacy `drive` journal-run directory, which nothing in the TUI reads any
    /// more. Runs a previous version left on disk stay there (they are not migrated or
    /// deleted) and are inert: they trigger neither a journal tail nor a re-parse.
    Ignored,
    /// Everything else. Keeps today's re-parse behaviour.
    Planning,
}

/// Classify a changed path as a launcher-run write, an ignorable write or a planning
/// write.
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
///   misclassified as ignorable, because a misclassified planning write is a
///   dashboard that silently stops updating (RESEARCH Security Domain, tampering
///   row; T-16-07).
///
/// **`.planning/meta-manager/runs/` is inert (Phase 31, D-03).** It held the legacy
/// `drive` journal runs, which nothing in the TUI reads any more, and previous
/// versions' runs stay on disk. Every path at or below it is [`ChangeKind::Ignored`]:
/// the shared `.gitignore`, the `active` pointer, `run.lock`, the run directory
/// itself and everything deeper. Before this, writes under it routed to a journal
/// tail, and the shallower ones to a full re-parse.
///
/// **`.planning/meta-manager/drivers/` is an ALLOWLIST (30-01, D-24).** The
/// run directory (`drivers/<id>`), `drivers/<id>/DIRECTION.md` and
/// `drivers/<id>/LOG.md` are [`ChangeKind::DriverRun`]; every other shape under
/// `drivers/` is [`ChangeKind::Ignored`], so a file added to a run directory later
/// is silent until someone decides it should not be. The same three-component
/// anchor applies: a project's own `.planning/drivers/` stays `Planning`.
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

    if parts.len() >= 3
        && parts[0] == ".planning"
        && parts[1] == "meta-manager"
        && parts[2] == "runs"
    {
        return ChangeKind::Ignored;
    }

    if parts.len() >= 3
        && parts[0] == ".planning"
        && parts[1] == "meta-manager"
        && parts[2] == "drivers"
    {
        return match parts.as_slice() {
            [_, _, _, run_id] | [_, _, _, run_id, "DIRECTION.md" | "LOG.md"] => {
                ChangeKind::DriverRun {
                    run_id: (*run_id).to_string(),
                }
            }
            // The bare `drivers` directory, a splice temp, `state.json`, a lock,
            // an editor swap file, a deeper path: nothing reads them.
            _ => ChangeKind::Ignored,
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
        /// for `force_push_blocked` or `quota_rejected` finds the producer
        /// and the record together.
        ///
        /// **Three sanctioned taxonomies ride this one field (the run bounds and
        /// the model-escalation budget were two more until Phase 31 retired the
        /// code that produced them), and naming only one of them would be the
        /// same quiet lie this record exists to prevent.** They are siblings,
        /// never extensions of each other:
        ///
        /// | Taxonomy | Prefix on disk | What it means |
        /// |---|---|---|
        /// | [`crate::envelope::policy::ParkReason`] | none (e.g. `force_push_blocked`) | The safety envelope refused an operation (Phase 19, D-24). **Seven arms, and it stays at seven** — a new detector is a new sibling enum, not an eighth arm here. |
        /// | `crate::driver::router::RouterReason` | `router_` **or** `gate_` | The deterministic router declined to choose a next command (Phase 20, DRIVE-06). Two prefixes on one enum, on purpose: `router_` marks a refusal about the rule table (no rule, unverified state, an unsatisfied dependency) and `gate_` a human-judgement gate the run reached and refused to answer (DRIVE-05). The split is what makes "how often does the always-park posture stop a run, and at which gate?" a grep over the journals rather than a memory. |
        /// | `crate::driver::rate_limit::QuotaReason` | `quota_` | The transport reported a Claude subscription quota rejection and the run stopped without retrying (Phase 20, CTRL-07). Its own taxonomy: a quota rejection is a fact about a budget shared with every other Claude surface the user has. |
        ///
        /// The prefixes make the producer readable from the string alone, and
        /// they are a property of each enum's own `REASON_*` constants rather
        /// than something assembled here. All three reach a terminal record
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
    /// `(routed: see the decided journal records)` and the run's identity moves to
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

/// The one type a driver holds for the duration of a run (D-06, D-36).
///
/// It owns the run's paths, its open journal and its `run.json` document. One
/// `JournalRun` is one run, start to finish:
///
/// ```text
/// JournalRun::start(planning, record)   -> prune, mkdir+ignore, run.json #1, active, journal, run_started
/// JournalRun::record(&event)            -> zero or more, one per journal event
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
            record_writes: 1,
        })
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_change_ignores_every_legacy_runs_path() {
        let root = Path::new("/home/user/projects/myapp");
        let run = ".planning/meta-manager/runs/2026-07-28T14-03-11Z-a3f9";
        let cases = [
            ".planning/meta-manager/runs".to_string(),
            ".planning/meta-manager/runs/.gitignore".to_string(),
            ".planning/meta-manager/runs/active".to_string(),
            ".planning/meta-manager/runs/run.lock".to_string(),
            run.to_string(),
            format!("{run}/journal.jsonl"),
            format!("{run}/run.json"),
            format!("{run}/inbox/x"),
            format!("{run}/inbox.jsonl"),
            format!("{run}/nested/run.json"),
        ];
        for case in cases {
            assert_eq!(
                classify_change(root, &root.join(&case)),
                ChangeKind::Ignored,
                "{case} is a legacy run path and must be ignored"
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
    fn classify_change_does_not_treat_a_bare_runs_directory_as_inert() {
        let root = Path::new("/home/user/projects/myapp");
        let cases = [
            // A project that keeps its own `runs/` under `.planning/`. The inert
            // prefix is `meta-manager/runs/`, never a bare `runs`.
            ".planning/runs/2026-07-28T14-03-11Z-a3f9/journal.jsonl",
            ".planning/runs/anything",
            ".planning/runs/x",
            "meta-manager/runs/2026-07-28T14-03-11Z-a3f9/journal.jsonl",
        ];
        for case in cases {
            assert_eq!(
                classify_change(root, &root.join(case)),
                ChangeKind::Planning,
                "{case} must keep re-parsing the project"
            );
        }
    }

    #[test]
    fn classify_change_routes_driver_run_paths() {
        let root = Path::new("/home/user/projects/myapp");
        let run_id = "20261020-driver-tab";
        let cases = [
            // The run directory appearing or disappearing.
            ".planning/meta-manager/drivers/20261020-driver-tab",
            ".planning/meta-manager/drivers/20261020-driver-tab/DIRECTION.md",
            ".planning/meta-manager/drivers/20261020-driver-tab/LOG.md",
        ];
        for case in cases {
            assert_eq!(
                classify_change(root, &root.join(case)),
                ChangeKind::DriverRun {
                    run_id: run_id.to_string()
                },
                "{case} must classify as a driver run change"
            );
        }
    }

    #[test]
    fn classify_change_ignores_every_other_drivers_path() {
        let root = Path::new("/home/user/projects/myapp");
        let run = ".planning/meta-manager/drivers/20261020-driver-tab";
        let cases = [
            format!("{run}/state.json"),
            format!("{run}/.DIRECTION.md.fm.abc"),
            format!("{run}/.DIRECTION.md.edit"),
            format!("{run}/launcher.lock"),
            format!("{run}/DIRECTION.lock"),
            format!("{run}/.DIRECTION.md.edit.swp"),
            format!("{run}/a/b/c"),
            // Deeper paths named like the allowed files are still deeper paths.
            format!("{run}/a/DIRECTION.md"),
            format!("{run}/LOG.md/x"),
            // The bare drivers directory itself (inferred: it names no run).
            ".planning/meta-manager/drivers".to_string(),
        ];
        for case in cases {
            assert_eq!(
                classify_change(root, &root.join(&case)),
                ChangeKind::Ignored,
                "{case} must be ignored"
            );
        }
    }

    #[test]
    fn classify_change_keeps_a_project_owned_drivers_dir_as_planning() {
        let root = Path::new("/home/user/projects/myapp");
        // No `meta-manager` component: the project's own directory, whose writes
        // must keep re-parsing the project.
        for case in [
            ".planning/drivers/x/DIRECTION.md",
            ".planning/drivers/x",
            ".planning/runs/x",
            "drivers/x/DIRECTION.md",
            ".planning/meta-manager/QUEUE.md",
        ] {
            assert_eq!(
                classify_change(root, &root.join(case)),
                ChangeKind::Planning,
                "{case} is not the launcher's drivers directory"
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
    fn run_paths_place_every_artifact_where_the_layout_says() {
        let planning = Path::new("/p/.planning");
        let paths = run_paths(planning, "RID").expect("a plain run id yields paths");
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
        assert_eq!(
            runs_root(planning),
            Path::new("/p/.planning/meta-manager/runs")
        );
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

    // -----------------------------------------------------------------------
    // The approval TOKEN: one copy-pasteable value carrying both halves
    // -----------------------------------------------------------------------

    #[test]
    fn sha256_digest_is_prefixed_and_is_sixty_four_lowercase_hex_digits() {
        // A known vector, so this fails if the function ever stops being
        // SHA-256 rather than merely stops being stable against itself.
        let digest = sha256_digest(b"abc");
        assert_eq!(
            digest, "sha256:ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad",
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
        assert_ne!(
            sha256_digest(b"CLAUDE.md v1"),
            sha256_digest(b"CLAUDE.md v2")
        );
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

        // **Both directions, and the acceptances come first on purpose.** The
        // predicate was tightened twice in this commit; a tightening that
        // regressed a legitimate value would be a worse defect than the two it
        // closes, so every shape the tree actually passes through this seam —
        // a bare phase number, a decimal phase, a timestamped run id, a
        // registered alias — is pinned as accepted.
        for legitimate in ["20", "2.1", "2026-08-19T12-00-00Z-aaaa", "demo", "99"] {
            assert!(
                is_plain_path_component(legitimate),
                "{legitimate:?} is a shape the tree passes through this seam and \
                 must keep passing"
            );
        }

        // **The blank half consumes the SHARED const, and the structural half
        // stays literal.** Pass 5 found this pin carrying a hand-copied
        // `["   ", "\t", "\n  \n"]` — three of the six blank shapes defined in
        // the very commit that defined six — so the two zero-width shapes were
        // never asserted at the one seam where they were reachable end to end,
        // and `--run-id '\u{200b}'` drove a complete run. Every blank-shape pin
        // in the tree now reads `test_support::DEGENERATE`, so a NEW shape lands
        // here without anybody remembering to copy it — which is how D-19-4's
        // four outside-the-old-ranges payloads arrived at this seam. The
        // traversal,
        // separator and embedded-control hostiles below are NOT blank shapes and
        // stay written out.
        let structural_hostiles = [
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
            // A control character embedded in an otherwise plain component.
            // `"a\nEVIL"` reached the dry-run render verbatim, breaking the line
            // the preview appears to be. It carries plenty of VISIBLE content,
            // so it is refused by the control-character half rather than by the
            // blankness half — which is why that half is composed with the
            // shared predicate rather than replaced by it.
            "a\nEVIL",
        ];
        for hostile in crate::test_support::DEGENERATE
            .iter()
            .chain(structural_hostiles.iter())
            .copied()
        {
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

    /// Two identities that render identically must not both resolve.
    ///
    /// **The tracer for pass-6 gap 1.** Pass 5 wrote the sentence "two visually
    /// identical aliases resolve to two different envelope and credential
    /// paths", 21-15 truth 5 claimed to close it, and the fix structurally could
    /// not: [`crate::text::carries_visible_content`] asks "is anything visible?",
    /// which refuses `"\u{200b}"` and accepts `"demo\u{200b}"`. Every payload in
    /// `DEGENERATE` is wholly invisible, so no fixture in the tree could express
    /// the harm — hence [`crate::test_support::LOOK_ALIKE_PAIRS`], consumed here
    /// rather than hand-spelled.
    ///
    /// The four seams asserted are the four a look-alike reached at pass 6: the
    /// predicate itself, the run directory, the envelope root, and (through the
    /// envelope root) the credential scope.
    ///
    /// **Red arm, observed verbatim against the unfixed tree** (this commit;
    /// the `#[ignore]` comes off in the fix commit, so the committed tree stays
    /// green while the red evidence is in history):
    ///
    /// ```text
    /// running 1 test
    /// test journal::tests::a_look_alike_identity_never_resolves_beside_its_visible_twin ... FAILED
    ///
    /// ---- journal::tests::a_look_alike_identity_never_resolves_beside_its_visible_twin stdout ----
    ///
    /// thread 'journal::tests::a_look_alike_identity_never_resolves_beside_its_visible_twin' (123442) panicked at src/journal/mod.rs:2801:13:
    /// "demo\u{200b}" renders exactly as "demo" and carries different bytes; accepting both is how two identities that no reader can distinguish both resolve
    ///
    /// test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1031 filtered out; finished in 0.00s
    /// ```
    #[test]
    fn a_look_alike_identity_never_resolves_beside_its_visible_twin() {
        let planning = Path::new("/p/.planning");
        let envelope_root = Path::new("/data/envelope");

        for (visible, look_alike) in crate::test_support::LOOK_ALIKE_PAIRS {
            assert!(
                is_plain_path_component(visible),
                "{visible:?} is the visible member of a look-alike pair and must \
                 keep being accepted — a refusal that swallowed both members \
                 would close the harm by making the tool unusable"
            );
            assert!(
                !is_plain_path_component(look_alike),
                "{look_alike:?} renders exactly as {visible:?} and carries \
                 different bytes; accepting both is how two identities that no \
                 reader can distinguish both resolve"
            );
            assert!(
                crate::envelope::envelope_dir_in(envelope_root, look_alike).is_none(),
                "{look_alike:?} must not name an envelope root beside \
                 {visible:?}'s — pass 6 reproduced exactly two roots rendering \
                 as one"
            );
            assert!(
                crate::envelope::envelope_dir_in(envelope_root, visible).is_some(),
                "{visible:?} must still name its envelope root"
            );
        }

        // The run-id half needs a run-id-SHAPED composite, because a run id is
        // never a bare alias: this is the pinned legitimate shape with one
        // invisible character appended.
        let visible_run_id = "2026-08-19T12-00-00Z-aaaa";
        let look_alike_run_id = "2026-08-19T12-00-00Z-aaaa\u{200b}";
        assert!(
            run_paths(planning, visible_run_id).is_some(),
            "the pinned legitimate run id must keep resolving"
        );
        assert!(
            run_paths(planning, look_alike_run_id).is_none(),
            "a run id that renders identically to {visible_run_id:?} must yield \
             no paths at all — pass 6 reproduced two run directories, one \
             rendering, a record no human can tell from its sibling"
        );
    }

    /// **The finite direction at this seam, and every value pass 7 measured
    /// ACCEPTED here** (D-19-2).
    ///
    /// The look-alike sweep above refuses by deny-list, and pass 7 measured what
    /// that costs: with the class spelled as three literal ranges, this
    /// predicate returned `true` for `"demo\u{202e}"`, `"2\u{202e}0"`,
    /// `"2\u{e0041}0"` and `"2\u{ad}0"`, and `run_paths` handed back a real
    /// directory for each. Every one of those is outside the ranges and inside
    /// the class the doc claimed. The alphabet ends that regress: the accepted
    /// set is finite, so there is no "next code point" for a later pass to find.
    ///
    /// The phase-token case is the sharpest. `"2\u{202e}0"` was safe only
    /// because no roadmap happens to declare it — a fact about roadmap
    /// *contents* that the next roadmap could falsify. Here it is a property of
    /// the value (21-17 truth 6, SAFE-08).
    #[test]
    fn an_identity_outside_the_alphabet_names_nothing_at_any_seam() {
        let planning = Path::new("/p/.planning");
        let envelope_root = Path::new("/data/envelope");

        for outside in [
            "demo\u{202e}",
            "2\u{202e}0",
            "2\u{e0041}0",
            "2\u{ad}0",
            "\u{434}\u{435}\u{43c}\u{43e}", // Cyrillic — no invisible byte at all
            "d\u{e9}mo",                    // visible accented Latin
        ] {
            assert!(
                !is_plain_path_component(outside),
                "{outside:?} is outside the identity alphabet and must not name \
                 a run directory, envelope root, credential scope or phase token"
            );
            assert!(
                run_paths(planning, outside).is_none(),
                "{outside:?} must yield no run paths — pass 7 measured \
                 `run_paths` handing back a real directory for values like this"
            );
            assert!(
                crate::envelope::envelope_dir_in(envelope_root, outside).is_none(),
                "{outside:?} must not name an envelope root"
            );
        }

        // The accepting direction, unchanged. Every value the suite pins as a
        // legitimate identity is drawn from the alphabet already, which is why
        // the inversion costs the tool nothing it was using.
        for accepted in [
            "20",
            "2.1",
            "2026-08-19T12-00-00Z-aaaa",
            "demo",
            "99",
            "RID",
        ] {
            assert!(
                is_plain_path_component(accepted),
                "{accepted:?} is a pinned legitimate identity and must keep \
                 being accepted — a narrowing that swallowed these would close \
                 the harm by making the tool unusable"
            );
        }
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
            // The single-command shape: this helper backs the pre-Phase-20
            // tests, and they must keep asserting what a command-mode run
            // records rather than quietly acquiring routed values.
            target_phase: None,
            extra: serde_json::Map::new(),
        }
    }

    #[test]
    fn a_run_writes_its_record_exactly_twice() {
        let dir = tempfile::tempdir().expect("temp dir");
        let planning = dir.path().join(".planning");

        let mut run = JournalRun::start(&planning, run_record(RUN_ID)).expect("start the run");
        assert_eq!(
            run.record_writes(),
            1,
            "write one lands at run start (D-06)"
        );
        assert!(run.paths().run_json.is_file());
        assert!(
            run.paths().gitignore.is_file(),
            "the ignore file lands first"
        );
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
        assert_eq!(
            run.record_writes(),
            1,
            "journalling never rewrites run.json"
        );

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
        assert!(
            record.extra.is_empty(),
            "the old shape carries no unknown fields"
        );
    }

    #[test]
    fn a_record_carrying_the_retired_goal_layer_fields_still_loads_and_keeps_them() {
        // Phase 31 deleted `bounds`, `approved_plan`, `escalation_cap` and
        // `escalations_used` from `RunRecord` with the drive path that wrote
        // them. A `run.json` an older build left on disk still carries them, and
        // the record must load (users' run dirs are left in place, D-03) and
        // must not silently delete them on a rewrite: they land in `extra`.
        const LEGACY: &[u8] = br#"{
            "run_id": "2026-09-30T10-00-00Z-aaaa",
            "goal": "g",
            "gsd_command": "(routed: see the decided journal records)",
            "target_phase": "21",
            "bounds": {"max_steps": 5, "wall_clock_cap_secs": 3600},
            "approved_plan": {"plan": "fnv1a64:aaaaaaaaaaaaaaaa", "files": "fnv1a64:bbbbbbbbbbbbbbbb"},
            "escalation_cap": 3,
            "escalations_used": 1,
            "target": "host",
            "opt_in": null,
            "started_at": "2026-09-30T10:00:00Z",
            "session_id": "s",
            "pid": 1,
            "pgid": 1,
            "claude_code_version": "2.1.0",
            "argv_digest": "fnv1a64:0000000000000000",
            "ended_at": null,
            "outcome": null
        }"#;

        let record: RunRecord = serde_json::from_slice(LEGACY).expect("a legacy record loads");
        assert_eq!(record.target_phase.as_deref(), Some("21"));
        for key in [
            "bounds",
            "approved_plan",
            "escalation_cap",
            "escalations_used",
        ] {
            assert!(
                record.extra.contains_key(key),
                "{key} must survive in the flattened overflow, not be dropped"
            );
        }
        let again = serde_json::to_value(&record).expect("it serialises");
        assert_eq!(again["bounds"]["max_steps"], 5);
        assert_eq!(again["escalation_cap"], 3);
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
