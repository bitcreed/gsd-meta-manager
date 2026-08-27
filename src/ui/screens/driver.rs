//! The Driver tab: the run list, the run header, the reused D-R-P-E-V row and
//! the step timeline.
//!
//! **Module-layout decision, recorded here because this is where a reader will
//! ask the question** (the register of `src/driver/liveness.rs:35-41`). Every
//! other detail sub-tab renders inline in `detail.rs`; this one renders from its
//! own module and `detail.rs` delegates. The reason is size and shape rather
//! than taste: every other sub-tab is about forty lines of layout, while this
//! one is eight width and height tiers, a bounded ring-buffer renderer and a
//! four-state injection widget — and `detail.rs` is already the largest file in
//! the repository at over five thousand lines.
//!
//! **The declined alternative was to render inline like every sibling.** It was
//! declined only for that reason; the cost of the split is that a reader looking
//! for "where does the Driver tab render" finds a delegation rather than a body,
//! which this doc and the two `DetailSubView::Driver` dispatch arms answer.
//!
//! **The Replit rule governs everything here (D-13).** Every status word,
//! glyph, colour and sort key on this surface derives from `RunRecord.outcome`,
//! `JournalEvent::RunEnded.outcome` or `ExecFinished.exit` — evidence Phase 15
//! computes from `is_error`, `terminal_reason`, the permission-denial list, the
//! exit code and a git/disk snapshot. **None of it is ever computed by
//! inspecting the agent's own prose.** The agent's words may appear as content
//! in the output pane and may carry no authority anywhere else. The incident
//! behind the rule: an agent deleted a production database during an explicit
//! freeze, hid it, fabricated roughly four thousand fake users and fake test
//! results, and falsely claimed rollback was impossible. A tool that repeats an
//! agent's account of itself as fact makes that class of failure invisible to
//! the human who is accountable for the repository.
//!
//! **The injection vocabulary is a safety property, not a style choice**
//! (D-07, D-10). An injected message has exactly four states and each has one
//! authoritative observer: `queued` is durably in `inbox.jsonl` with nothing
//! having read it; `delivered` means the write to the agent's stdin returned
//! without error; `acted-on` means the agent **dequeued** it and is running it
//! as its own turn; `missed` means it was appended after the agent's input was
//! closed and can never be delivered.
//!
//! The word for the stdin write is *delivered* and nothing else. The word for
//! the dequeue echo is *acted-on*. **The word "sent" is forbidden here, and so
//! are "received", "read" and "acknowledged"** — the echo arrives at DEQUEUE,
//! measured roughly fifty-five seconds after the write, so every one of those
//! words would promise an observation the protocol cannot make. The filling
//! shape `○ → ◐ → ●` is the whole design: it reads correctly across a
//! minute-long gap. **There is no spinner, no animation and no implied
//! imminence anywhere on this surface**, and a spinner idiom here would be
//! actively dishonest rather than merely decorative.

use std::cell::Cell;

use chrono::{DateTime, Local, Utc};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph};
use ratatui::Frame;

use super::detail::{clamp_scroll, tail_offset, ViewportMetrics};
use super::{
    sanitize_render_line, AppContext, DriverLineKind, DriverOutput, DriverOutputLine,
    DryRunPreview, ProjectViewCache, DRIVER_OUTPUT_RING_LINES,
};
use crate::driver::reconcile::RunVerdict;
use crate::journal::inbox::InboxMessage;
use crate::journal::reader::JournalRecord;
use crate::journal::RunSummary;

// ── Measured layout constants (the `STATUS_COLUMN_MIN_CELLS` discipline) ────

/// The widest line `detail::build_pipeline_line` can produce, in cells.
///
/// **Derived from the render, not chosen.** Its worst case is the two-cell left
/// indent, then `[D]`, `---`, `[R]`, `---`, `[P]`, `---`, `[E 123/456]`, `---`,
/// `[V]` — `2 + 3 + 3 + 3 + 3 + 3 + 3 + 11 + 3 + 3 = 37`.
///
/// The number exists because of a bug with a name. A percentage-only constraint
/// on the pane holding that line is exactly what clipped its trailing `[V]`
/// (UIFIX-02 / CR-01), and a fully verified project then rendered identically to
/// a mid-pipeline one — the display disagreeing with the disk in the direction
/// that flatters the run.
pub const PIPELINE_LINE_MAX_CELLS: u16 = 37;

/// The floor the run-detail pane is given, in cells.
///
/// [`PIPELINE_LINE_MAX_CELLS`] plus the pane's own two-cell left indent. It is a
/// `Constraint::Min` and never a bare `Constraint::Percentage`, for the reason
/// spelled out above: the reused D-R-P-E-V line must keep its trailing `[V]` at
/// every width where two panes render at all, and only a floor guarantees that.
pub const DRIVER_DETAIL_MIN_CELLS: u16 = PIPELINE_LINE_MAX_CELLS + 2;

/// Below this outer width the run list is not rendered at all and the run detail
/// gets the whole area.
///
/// Derived: at 59 columns a 40% list takes 23 cells and leaves 36 for the
/// detail, which is below [`DRIVER_DETAIL_MIN_CELLS`]. Rendering both anyway
/// would clip the pipeline line, so the list — whose information moves into the
/// detail pane's block title — is what gives way.
pub const DRIVER_TWO_PANE_MIN_CELLS: u16 = 60;

/// Inner width at or above which a run-list row carries its state word.
///
/// Derived: the fixed row is 18 cells, the `"> "` highlight symbol is 2, and the
/// gap before the word is 2 — so 22 cells are spoken for before a word can
/// start, and 26 is the first width that leaves room for a short one.
pub const RUN_LIST_WORD_MIN_CELLS: u16 = 26;

/// The fixed part of a run-list row, in cells:
/// `glyph(1) space MM-DD(5) space HH:MM(5) space suffix(4)`.
pub const RUN_LIST_FIXED_CELLS: usize = 18;

// ── Run-state glyphs (fixed `&'static str`, `\u{…}` per PATTERNS S7) ────────
//
// A fixed `&'static str` is the *mechanism* enforcing "no glyph is derived from
// file content": no byte read off disk can be assigned to one of these, so agent
// prose cannot reach a run row through its glyph (D-13). The escape form keeps
// the source readable in editors that render these codepoints ambiguously.

/// `◆` — a run this scan positively knows is running.
const GLYPH_LIVE: &str = "\u{25C6}";
/// `◇` — the platform cannot say whether the driver lives. **Never a synonym
/// for dead**; that collapse was CR-05 and the tri-state fix must not be
/// re-flattened at the render layer.
const GLYPH_LIVENESS_UNKNOWN: &str = "\u{25C7}";
/// `●` — succeeded, with changes on disk.
const GLYPH_SUCCEEDED: &str = "\u{25CF}";
/// `○` — succeeded while nothing moved on disk. Its own state, not a flavour of
/// success: the envelope said success and the working tree disagreed.
const GLYPH_SUCCEEDED_NO_CHANGES: &str = "\u{25CB}";
/// `✗` — failed, denied, timed out, stalled, refused, crashed.
const GLYPH_FAILED: &str = "\u{2717}";
/// `■` — the user stopped it.
const GLYPH_KILLED: &str = "\u{25A0}";

// ── Output-pane markers (fixed `&'static str`, `\u{…}` per PATTERNS S7) ─────
//
// A two-cell left marker column, and the same mechanism argument as the run
// glyphs above: no byte read off disk can be assigned to one of these, so agent
// prose cannot reach the marker column (D-13).

/// The common case — assistant, user, turn-boundary and unparseable streams —
/// gets **no** marker, so injections and diagnostics stand out against it.
const MARKER_OUTPUT: &str = "  ";
/// `· ` — the agent's stderr: present, and deliberately secondary.
const MARKER_STDERR: &str = "\u{00B7} ";
/// `» ` — a human-injected message. The one Cyan use in this pane.
const MARKER_INJECTION: &str = "\u{00BB} ";
/// `! ` — a diagnostic. **Never silently swallowed**: a pane that loses lines
/// without saying so is exactly the "looks done but isn't" failure this phase
/// enumerates by name.
const MARKER_DIAGNOSTIC: &str = "! ";
/// `= ` — the terminal record, in the terminal-state colour and BOLD. The
/// visual full stop, and it renders last.
const MARKER_TERMINAL: &str = "= ";

// ── Injection state: glyphs, labels and the kinds that carry them ──────────

/// The three journal kinds that carry an injected message's state (D-07, D-09).
///
/// Named here, beside the derivation that reads them, and referenced by the
/// scan that collects them — so the set of kinds that matter exists once.
pub const INJECTION_KINDS: [&str; 3] = [
    "interjected",
    "interjection_acted_on",
    "interjection_missed",
];

/// `○` — durably on disk in `inbox.jsonl`; **nothing has read it**.
pub(super) const GLYPH_QUEUED: &str = "\u{25CB}";
/// `◐` — written to the agent's stdin without error. Half-filled, because half
/// of what matters has happened: the write landed and the agent has not yet
/// picked it up.
pub(super) const GLYPH_DELIVERED: &str = "\u{25D0}";
/// `●` — the agent **dequeued** it and is running it as its own turn. Full,
/// because this is as far as the protocol can see. The same codepoint the run
/// glyphs use for "succeeded": both mean *this reached its end*, and the two
/// live in different columns of different widgets, so no row shows both.
pub(super) const GLYPH_ACTED_ON: &str = "\u{25CF}";
/// `✗` — appended after the agent's input was closed. Undeliverable, named,
/// and **never retried**.
pub(super) const GLYPH_MISSED: &str = "\u{2717}";

/// The exact label for a message durably queued and unread.
pub(super) const LABEL_QUEUED: &str = "queued";
/// The exact label for a write to the agent's stdin that returned without
/// error. **This word belongs to the stdin write and to nothing else.**
pub(super) const LABEL_DELIVERED: &str = "delivered";
/// The exact label for the dequeue echo. Deliberately not "received", "read" or
/// "acknowledged": the echo says the agent *started processing*, roughly a
/// minute after the write, and every one of those three words would claim an
/// earlier and stronger observation than the protocol supports.
pub(super) const LABEL_ACTED_ON: &str = "acted-on";
/// The exact label for the honest fourth state (D-10).
pub(super) const LABEL_MISSED: &str = "missed";

/// Why a missed message is missed, in one line: it arrived after the close.
///
/// **Leaving it in `queued` forever would be the undelivered-injection failure
/// dressed up as a spinner.** It is named instead.
const MISSED_GLOSS: &str = "(the run closed its input before this was delivered)";

/// Why a missed message is missed, in one line: the write itself failed (CR-03).
///
/// The second reason, and the one that had no gloss because it had no state.
/// It says what happened and, by saying "never reached", refuses to imply the
/// agent might still pick the message up.
const MISSED_GLOSS_SEND_FAILED: &str = "(the write to the agent failed; it never reached the run)";

/// Why a missed message is missed when the journal names a reason this build
/// does not recognise.
///
/// A journal written by a newer build can carry a reason string this one has no
/// gloss for. The honest answer is to say the message is undeliverable and that
/// the recorded reason is not one this build knows — **never** to fall back to
/// one of the two glosses above, which would attribute a cause the evidence
/// does not support.
const MISSED_GLOSS_UNRECOGNISED: &str = "(undeliverable; the journal gives a reason this build does not recognise)";

// ── Copy (Copywriting Contract, exact strings) ─────────────────────────────

/// The goal field for a run that was started without one.
///
/// **Never a fabricated or paraphrased summary** (D-13, D-23). A goal that was
/// given is stored and shown verbatim; a goal that was not given says so.
const GOAL_NONE_GIVEN: &str = "(none given)";

const NO_RUNS_OPTED_IN: &str = "No runs yet for";
const NO_RUNS_OPTED_IN_NEXT: &str = "Press [s] to start one.";

/// The next step for a project that has not been given permission to be driven.
///
/// It names no other surface, because `o` is now bound on **this** tab
/// (`detail.rs`, the `Char('o') if current_view == DetailSubView::Driver` arm)
/// and opens the same confirmation the dashboard's `o` opens. The sentence
/// points at where the reader is already standing; sending them back to the
/// dashboard to press a key that works right here was a navigation round-trip
/// that existed for no reason.
const NOT_OPTED_IN_NEXT: &str = "Press [o] to allow it.";
const NO_JOURNAL_ENTRIES: &str = "No journal entries yet.";

/// The right-aligned indicator while the pane is following a live tail.
const INDICATOR_FOLLOWING: &str = "[following]";

/// The right-aligned indicator for a run this session did not spawn.
///
/// **It does not say "live"**, because there is nothing live about it: once the
/// TUI exits, the child's stdout pipe is gone, so reattachment is read-only and
/// journal-based (Phase 17 D-11).
const INDICATOR_JOURNAL_ONLY: &str = "[journal only]";

/// The largest scrolled-below count rendered as a figure; past it the indicator
/// says `+999`, because the exact number stops carrying information long before
/// then and a widening field would move the rule under the reader's eye.
const SCROLLED_DISPLAY_MAX: usize = 999;

/// The notice the pane's first row carries once the ring has dropped lines.
///
/// **Not cosmetic.** The buffer is bounded, so on a long run it *will* drop the
/// beginning of the output; a pane that silently loses lines is the named
/// "looks done but isn't" failure. The count is monotonic over the session, so
/// the sentence stays true after any number of wraps.
fn ring_overflow_notice(dropped: u64) -> String {
    format!("\u{2026} {dropped} earlier lines dropped (buffer holds {DRIVER_OUTPUT_RING_LINES})")
}

/// The notice for a record cut at the per-record line cap.
///
/// One pathological record could otherwise flush the whole ring, so the cap
/// exists — and saying that it fired is the other half of it.
const RECORD_TRUNCATED_NOTICE: &str = "    \u{2026} record truncated";

/// The pinned first row for a run this session did not spawn (Phase 17 D-11).
///
/// **It says the live output is gone, and that is the only thing it may say.**
/// Once the TUI exits the child's stdout pipe is gone, so reattachment is
/// read-only and journal-based; live re-streaming after a restart is impossible
/// by construction. **Nothing in code, copy, status text, footer hint or doc
/// comment may promise it** — a promise the mechanism cannot keep is the named
/// "looks done but isn't" failure, and this sentence is what stands in its
/// place.
const ADOPTED_RUN_NOTICE: &str = "This run started before the current TUI session. \
     Its live output is gone \u{2014} showing the journal on disk.";

/// The four-cell indent every injection body row carries.
///
/// A constant rather than a literal at three call sites: the state column's
/// alignment is the whole reason the rendering is always two rows, and three
/// copies of an indent is three chances for one of them to drift.
const INJECTION_INDENT: &str = "    ";

/// The mandatory honest note under the one-row step timeline.
///
/// **Required, not decorative.** In this phase a run executes exactly one GSD
/// command plus whatever turns the human's own interjections add, so the
/// timeline has one decided row — and a one-row timeline with no explanation
/// implies a computed sequence of steps that does not exist yet. Phase 20's
/// router is what makes the row count mean something.
const STEPS_HONEST_NOTE: &str = "This version runs one command per run.";

/// The label that makes the cost figure readable rather than misleading.
///
/// **Mandatory** (D-12): `total_cost_usd` accumulates across the turns of one
/// run while `num_turns` and `duration_ms` reset per turn, so an unlabelled
/// figure invites the reader to take a running total for a per-turn one. Any
/// per-turn figure rendered anywhere on this surface must be labelled as such.
const COST_CUMULATIVE: &str = "cumulative";

/// What the cost line says before any `cost` record has been read for this run.
///
/// A run's cumulative cost is only knowable from its journal, and the run list
/// is deliberately built from each run's small committed `run.json` without
/// opening the journal beside it. So for a run that is not the one being tailed
/// the figure is genuinely unknown — and saying so is the only honest option. A
/// zero here would be a number the mechanism cannot back.
const COST_NOT_REPORTED: &str = "not yet reported";

/// The shipped async-load idiom, reused verbatim (Phase 12, UI-SPEC `## Colour`).
///
/// A second wording for "we are waiting on a blocking read" would be a second
/// thing for the reader to learn, and this surface has no spinner and no
/// animation to distinguish it with.
pub(super) const DRY_RUN_LOADING: &str = "Loading...";

// ── Styles ─────────────────────────────────────────────────────────────────

fn label_style() -> Style {
    Style::default().fg(Color::DarkGray)
}

fn muted_style() -> Style {
    Style::default()
        .fg(Color::DarkGray)
        .add_modifier(Modifier::DIM)
}

/// The glyph, run-list word and colour for one run's state.
///
/// **Every arm reads evidence and nothing else** (D-13): `verdict` is
/// [`crate::driver::reconcile::ObservedRun::verdict`], derived from the record's
/// `ended_at` and a pid/cmdline liveness probe; `outcome` is
/// `RunRecord.outcome`, the rendered form of Phase 15's four-source derivation
/// over `is_error`, `terminal_reason`, the permission-denial list, the exit code
/// and a git/disk snapshot. No agent prose is inspected, and no string
/// comparison against agent prose decides whether a run succeeded.
///
/// `None` for both is **not** treated as a crash. A run with no terminal record
/// and no observation is a run nothing can currently speak for, which is what
/// `◇ ?` says; reporting it as dead would manufacture a crash out of an absence
/// of evidence, which is the CR-05 defect in a new place.
pub fn run_state_glyph(
    verdict: Option<RunVerdict>,
    outcome: Option<&str>,
) -> (&'static str, &'static str, Color) {
    match verdict {
        Some(RunVerdict::Live) => (GLYPH_LIVE, "live", Color::Magenta),
        Some(RunVerdict::LivenessUnknown) => (GLYPH_LIVENESS_UNKNOWN, "?", Color::Yellow),
        Some(RunVerdict::CrashedWithoutEnding) => (GLYPH_FAILED, "crash", Color::Red),
        Some(RunVerdict::Ended) | None => match outcome {
            Some("succeeded_with_changes") => (GLYPH_SUCCEEDED, "ok", Color::Green),
            Some("succeeded_no_changes") => {
                (GLYPH_SUCCEEDED_NO_CHANGES, "no-chg", Color::Yellow)
            }
            Some("failed") => (GLYPH_FAILED, "fail", Color::Red),
            Some("permission_denied") => (GLYPH_FAILED, "denied", Color::Red),
            Some("timed_out") => (GLYPH_FAILED, "t/out", Color::Red),
            Some("stalled") => (GLYPH_FAILED, "stall", Color::Red),
            Some("capability_refused") => (GLYPH_FAILED, "refuse", Color::Red),
            Some("spawn_failed") => (GLYPH_FAILED, "spawn", Color::Red),
            Some("killed") => (GLYPH_KILLED, "killed", Color::DarkGray),
            // A label this build has never seen, or none at all.
            _ => (GLYPH_LIVENESS_UNKNOWN, "?", Color::Yellow),
        },
    }
}

// ── The terminal-state table (OBS-05, D-13) ────────────────────────────────

/// Every state a run can be in, as a **typed** value.
///
/// It exists so [`terminal_state_cell`] can match exhaustively with no wildcard
/// arm, which is what makes a future state a compile error rather than a silent
/// fallthrough to a word that happens to be wrong. Twelve of the thirteen are a
/// [`RunVerdict`] or a [`crate::executor::RunOutcome`]; the thirteenth is the
/// honest answer when neither is on disk.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalState {
    /// `RunVerdict::Live` — the record has no `ended_at` and the pid is still
    /// this run's driver.
    Live,
    /// `RunVerdict::LivenessUnknown`. **Never a synonym for dead**: that
    /// collapse was CR-05, and re-flattening the tri-state at the render layer
    /// would print "your run died" about every healthy run on a platform where
    /// the probe does not apply.
    LivenessUnknown,
    /// `RunVerdict::CrashedWithoutEnding` — no terminal record and the pid is
    /// gone.
    CrashedWithoutEnding,
    /// `RunOutcome::SucceededWithChanges`.
    Succeeded,
    /// `RunOutcome::SucceededNoChanges`. **Its own state, not a flavour of
    /// success**: the envelope said success while nothing moved on disk, which
    /// is exactly the disagreement TRANS-02 exists to surface.
    SucceededNoChanges,
    /// `RunOutcome::Failed`.
    Failed,
    /// `RunOutcome::PermissionDenied`.
    PermissionDenied,
    /// `RunOutcome::TimedOut` — the wall-clock cap.
    TimedOut,
    /// `RunOutcome::Stalled` — the idle cap; the stuck detector proper.
    Stalled,
    /// `RunOutcome::CapabilityRefused` — no turn ever started.
    CapabilityRefused,
    /// `RunOutcome::SpawnFailed`.
    SpawnFailed,
    /// `RunOutcome::Killed` — the user stopped it.
    Killed,
    /// Neither an observation nor a recognised outcome label is on disk.
    ///
    /// **Not a crash and not a success.** Manufacturing either out of an absence
    /// of evidence is the CR-05 defect in a new place.
    Unrecorded,
}

impl TerminalState {
    /// The state a derived [`crate::executor::RunOutcome`] names.
    ///
    /// **The match is exhaustive with no wildcard on purpose**: a new
    /// `RunOutcome` variant must fail to compile here rather than fall through
    /// to a word that is quietly wrong about what happened.
    pub fn from_outcome(outcome: &crate::executor::RunOutcome) -> Self {
        use crate::executor::RunOutcome;
        match outcome {
            RunOutcome::SucceededWithChanges { .. } => Self::Succeeded,
            RunOutcome::SucceededNoChanges { .. } => Self::SucceededNoChanges,
            RunOutcome::Failed { .. } => Self::Failed,
            RunOutcome::PermissionDenied { .. } => Self::PermissionDenied,
            RunOutcome::Killed { .. } => Self::Killed,
            RunOutcome::TimedOut { .. } => Self::TimedOut,
            RunOutcome::Stalled { .. } => Self::Stalled,
            RunOutcome::CapabilityRefused { .. } => Self::CapabilityRefused,
            RunOutcome::SpawnFailed { .. } => Self::SpawnFailed,
        }
    }

    /// The state an outcome **label** names.
    ///
    /// The label is what `run.json` and `JournalEvent::RunEnded` actually carry,
    /// written by `driver::run::outcome_label` from the derived outcome. A label
    /// this build has never seen is [`Unrecorded`](Self::Unrecorded) rather than
    /// a guess: the two vocabularies are kept in agreement by a test that maps
    /// every `RunOutcome` through the driver's own labeller and back.
    pub fn from_label(label: Option<&str>) -> Self {
        match label {
            Some("succeeded_with_changes") => Self::Succeeded,
            Some("succeeded_no_changes") => Self::SucceededNoChanges,
            Some("failed") => Self::Failed,
            Some("permission_denied") => Self::PermissionDenied,
            Some("timed_out") => Self::TimedOut,
            Some("stalled") => Self::Stalled,
            Some("capability_refused") => Self::CapabilityRefused,
            Some("spawn_failed") => Self::SpawnFailed,
            Some("killed") => Self::Killed,
            _ => Self::Unrecorded,
        }
    }

    /// The state the two pieces of evidence together support.
    ///
    /// An observation wins while there is one, because a live or crashed run has
    /// no terminal record to read; once the run has ended the label speaks.
    pub fn observed(verdict: Option<RunVerdict>, outcome: Option<&str>) -> Self {
        match verdict {
            Some(RunVerdict::Live) => Self::Live,
            Some(RunVerdict::LivenessUnknown) => Self::LivenessUnknown,
            Some(RunVerdict::CrashedWithoutEnding) => Self::CrashedWithoutEnding,
            Some(RunVerdict::Ended) | None => Self::from_label(outcome),
        }
    }
}

/// The glyph, the run-detail word and the colour for one terminal state (D-13).
///
/// **The Replit rule governs every arm.** Each state arrives from evidence and
/// from nothing else — `RunRecord.outcome`, `JournalEvent::RunEnded.outcome` or
/// the exec exit, which is the four-source derivation Phase 15 computes from
/// `is_error`, `terminal_reason`, the permission-denial list, the exit code and
/// a git/disk snapshot — and **never from inspecting the agent's own prose**.
/// The agent's words may be displayed as content in the output pane and may
/// carry no authority here or anywhere else. The incident behind the rule: an
/// agent deleted a production database during an explicit freeze, hid it,
/// fabricated roughly four thousand fake users and fake test results, and
/// falsely claimed rollback was impossible. A tool that repeats an agent's
/// account of itself as fact makes that class of failure invisible to the human
/// who is accountable for the repository.
///
/// **The match has no wildcard arm**, so a state this table has not been taught
/// is a compile error rather than a word that is silently wrong.
///
/// The word carries **no interpolated value**, and that is a fact about the
/// evidence rather than a simplification: what is on disk for a finished run is
/// the outcome *label*, so a reason, a denial count or a cap duration is simply
/// not there to render. Inventing one is what D-13 forbids. The return type is
/// an owned `String` so a later source of a sanitised, `…`-truncated detail can
/// be added without touching a caller.
///
/// **D-13 in one line, so it survives a reader who starts here:** every word
/// below comes from evidence and never from the agent's prose. The incident is a
/// production database deleted during an explicit freeze, hidden, and papered
/// over with fabricated users and fabricated test results.
pub fn terminal_state_cell(state: TerminalState) -> (&'static str, String, Color) {
    let (glyph, word, color) = match state {
        TerminalState::Live => (GLYPH_LIVE, "live", Color::Magenta),
        TerminalState::LivenessUnknown => {
            (GLYPH_LIVENESS_UNKNOWN, "liveness unknown", Color::Yellow)
        }
        TerminalState::CrashedWithoutEnding => (
            GLYPH_FAILED,
            "crashed without a terminal record",
            Color::Red,
        ),
        TerminalState::Succeeded => (GLYPH_SUCCEEDED, "succeeded", Color::Green),
        TerminalState::SucceededNoChanges => (
            GLYPH_SUCCEEDED_NO_CHANGES,
            "succeeded, no changes on disk",
            Color::Yellow,
        ),
        TerminalState::Failed => (GLYPH_FAILED, "failed", Color::Red),
        TerminalState::PermissionDenied => (GLYPH_FAILED, "permission denied", Color::Red),
        TerminalState::TimedOut => (GLYPH_FAILED, "timed out", Color::Red),
        TerminalState::Stalled => (GLYPH_FAILED, "stalled", Color::Red),
        TerminalState::CapabilityRefused => (GLYPH_FAILED, "capability refused", Color::Red),
        TerminalState::SpawnFailed => (GLYPH_FAILED, "spawn failed", Color::Red),
        TerminalState::Killed => (GLYPH_KILLED, "killed", Color::DarkGray),
        TerminalState::Unrecorded => {
            (GLYPH_LIVENESS_UNKNOWN, "outcome not recorded", Color::Yellow)
        }
    };
    (glyph, word.to_string(), color)
}

/// How long a run has been going, or how long it took.
///
/// `+MM:SS` while it is still going and `ran MM:SS` once it has ended, both
/// widening to `H:MM:SS` past an hour. An unparseable `started_at` yields
/// `unknown` rather than a plausible-looking zero: a duration the code cannot
/// compute is not a duration of nothing.
pub fn elapsed_label(started_at: &str, now: DateTime<Utc>, ended_at: Option<&str>) -> String {
    let Some(start) = parse_rfc3339(started_at) else {
        return "unknown".to_string();
    };
    let ended = ended_at.and_then(parse_rfc3339);
    let end = ended.unwrap_or(now);
    let seconds = (end - start).num_seconds().max(0);
    let rendered = hms(seconds);
    match ended {
        Some(_) => format!("ran {rendered}"),
        None => format!("+{rendered}"),
    }
}

fn parse_rfc3339(raw: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

fn hms(seconds: i64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;
    if hours > 0 {
        format!("{hours}:{minutes:02}:{secs:02}")
    } else {
        format!("{minutes:02}:{secs:02}")
    }
}

/// `MM-DD` and `HH:MM` in the reader's local zone, or placeholders.
fn local_date_time(started_at: &str) -> (String, String) {
    match parse_rfc3339(started_at) {
        Some(dt) => {
            let local = dt.with_timezone(&Local);
            (
                local.format("%m-%d").to_string(),
                local.format("%H:%M").to_string(),
            )
        }
        None => ("??-??".to_string(), "??:??".to_string()),
    }
}

/// The capped composition of BOTH classes, for the prose this tab draws.
///
/// **The half that was missing, found by the probe rather than by reading**
/// (21-25 T2). Every site below used `sanitize_render_line` alone, which
/// answers only the CONTROL class (`ESC` / C0 / `DEL` / C1). The
/// invisible-formatting class — `General_Category=Cf` union
/// `Default_Ignorable_Code_Point` — is `Cf`, not `Cc`, so `U+202E`, `U+00AD`
/// and the `U+E0000..U+E007F` tag block passed through untouched into a
/// `Paragraph`. This file's own `no_runs_lines` at the bottom of this module
/// already states that neither class subsumes the other and composes the two;
/// what was missing was every OTHER site on this path doing the same.
///
/// It was invisible for as long as it was because `probe_ctx` left
/// `driver_runs` empty, so the Driver tab rendered `no_runs_lines` — the ONE
/// site that was already composed — on every probe run. The red is quoted in
/// `render_escape_guard::probe_ctx`'s doc.
///
/// The cap is `sanitize_render_line`'s, deliberately: this draws agent prose
/// and a run's own goal, which is exactly what that cap exists for. The
/// uncapped composition is `crate::text::render_for_terminal`, and the two
/// agree below the cap (pinned by
/// `ui::screens::tests::the_capped_and_uncapped_compositions_agree_below_the_cap`).
fn shown_capped(value: &str) -> String {
    crate::text::display_identity(&sanitize_render_line(value))
}

/// The last four characters of a run id — enough to tell two runs apart in a
/// list, since `new_run_id` puts a random tie-break suffix there.
///
/// Already a CHARACTER operation, unlike the session-id truncation 21-25 fixed
/// in `detail.rs` — noted so a reader does not assume the family is everywhere.
fn run_id_suffix(run_id: &str) -> String {
    let chars: Vec<char> = run_id.chars().collect();
    let start = chars.len().saturating_sub(4);
    let mut suffix: String = chars[start..].iter().collect();
    while suffix.chars().count() < 4 {
        suffix.insert(0, ' ');
    }
    suffix
}

/// One row of the run list.
///
/// The fixed part is [`RUN_LIST_FIXED_CELLS`] cells at every width — one form,
/// not a set of width variants — and the state word is appended only when the
/// pane's inner width reaches [`RUN_LIST_WORD_MIN_CELLS`]. Glyph, word and
/// colour travel together so no meaning is carried by colour alone.
pub fn run_list_row(
    summary: &RunSummary,
    verdict: Option<RunVerdict>,
    inner_width: u16,
) -> Line<'static> {
    let (glyph, word, color) = run_state_glyph(verdict, summary.outcome.as_deref());
    let (date, time) = local_date_time(&summary.started_at);
    let mut spans = vec![
        Span::styled(glyph, Style::default().fg(color)),
        // READ BY A HUMAN: the run id is generated by this build but read back
        // out of a run directory name on disk, which any process can create.
        Span::raw(format!(
            " {date} {time} {}",
            shown_capped(&run_id_suffix(&summary.run_id))
        )),
    ];
    if inner_width >= RUN_LIST_WORD_MIN_CELLS {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(word, Style::default().fg(color)));
    }
    Line::from(spans)
}

/// The identity of the selected run, for the narrow tier's pane title.
fn run_list_title(summary: &RunSummary, index: usize, total: usize) -> String {
    let (date, time) = local_date_time(&summary.started_at);
    // READ BY A HUMAN, through `Block::title` — the family that preserves the
    // invisible class most completely of the four measured in 21-23.
    format!(
        " {date} {time} {}  ({}/{}) ",
        shown_capped(&run_id_suffix(&summary.run_id)),
        index + 1,
        total
    )
}

/// Split the tab area into the run list and the run detail.
///
/// The detail pane takes `Constraint::Min(DRIVER_DETAIL_MIN_CELLS)` and **never
/// a bare percentage complement of the list's 40%** — the discipline that keeps
/// the reused D-R-P-E-V line from losing its trailing `[V]`. A percentage-only
/// constraint on that pane is the CR-01 / UIFIX-02 defect, verbatim. Below
/// [`DRIVER_TWO_PANE_MIN_CELLS`] the list is dropped entirely and `None` is
/// returned for it; the selection still moves, only its *rendering* is given up.
fn driver_panes(area: Rect) -> (Option<Rect>, Rect) {
    if area.width < DRIVER_TWO_PANE_MIN_CELLS {
        return (None, area);
    }
    let panes = Layout::horizontal([
        Constraint::Percentage(40),
        Constraint::Min(DRIVER_DETAIL_MIN_CELLS),
    ])
    .split(area);
    (Some(panes[0]), panes[1])
}

/// Wrap `text` to at most `max_rows` rows of `width` cells, ellipsing the last
/// row when there is more.
///
/// Greedy word wrapping with a hard split for a word longer than the row, and
/// `char`-based throughout: byte slicing panics on a multibyte boundary, and a
/// goal is arbitrary user text.
fn wrap_rows(text: &str, width: usize, max_rows: usize) -> Vec<String> {
    let width = width.max(1);
    let mut rows: Vec<String> = Vec::new();
    let mut current = String::new();
    let mut current_cells = 0usize;

    let mut push_row = |row: &mut String, cells: &mut usize| {
        rows.push(std::mem::take(row));
        *cells = 0;
    };

    for word in text.split_whitespace() {
        let word_cells = word.chars().count();
        if current_cells > 0 && current_cells + 1 + word_cells > width {
            push_row(&mut current, &mut current_cells);
        }
        if word_cells > width {
            // A single word wider than the row: hard-split it by chars.
            for ch in word.chars() {
                if current_cells == width {
                    push_row(&mut current, &mut current_cells);
                }
                current.push(ch);
                current_cells += 1;
            }
            continue;
        }
        if current_cells > 0 {
            current.push(' ');
            current_cells += 1;
        }
        current.push_str(word);
        current_cells += word_cells;
    }
    if current_cells > 0 {
        rows.push(current);
    }

    if rows.len() > max_rows {
        rows.truncate(max_rows);
        if let Some(last) = rows.last_mut() {
            if last.chars().count() >= width {
                let kept: String = last.chars().take(width.saturating_sub(1)).collect();
                *last = kept;
            }
            last.push('\u{2026}');
        }
    }
    rows
}

/// Truncate to `width` cells by `char`, with a trailing ellipsis when cut.
fn ellipsize(text: &str, width: usize) -> String {
    if text.chars().count() <= width {
        return text.to_string();
    }
    let mut out: String = text.chars().take(width.saturating_sub(1)).collect();
    out.push('\u{2026}');
    out
}

/// Keep the tail of a path when it does not fit — the leading directories are
/// the part a reader can reconstruct, the run id is not.
fn left_truncate(text: &str, width: usize) -> String {
    let cells = text.chars().count();
    if cells <= width {
        return text.to_string();
    }
    let keep = width.saturating_sub(1);
    let skip = cells.saturating_sub(keep);
    let mut out = String::from("\u{2026}");
    out.extend(text.chars().skip(skip));
    out
}

/// The goal rows: verbatim, sanitised, wrapped to at most three, then ellipsed.
///
/// **The goal is never fabricated, paraphrased or summarised** (D-13, D-23). It
/// is the one field on this surface that is the human's own words, and the whole
/// value of OBS-03 is that it comes back exactly as it was given. When no goal
/// was given the field says [`GOAL_NONE_GIVEN`] and shows nothing else — an
/// invented one-line summary of what the run "seems to be doing" would be the
/// agent's account of itself wearing the user's voice.
fn goal_lines(goal: &str, inner_width: u16) -> Vec<Line<'static>> {
    const PREFIX: &str = "  goal: ";
    const CONTINUATION: &str = "        ";

    // The one production spelling of blankness, not a `trim` of this renderer's
    // own. This surface renders values re-read from run records, which this
    // build's parse boundary never saw: a goal of zero-width characters survives
    // `str::trim` and used to render as a blank line wearing a goal's label —
    // which reads as a goal the user simply cannot see rather than as no goal.
    if !crate::text::carries_visible_content(goal) {
        return vec![Line::from(vec![
            Span::styled(PREFIX, label_style()),
            Span::styled(GOAL_NONE_GIVEN, muted_style()),
        ])];
    }

    // READ BY A HUMAN: the goal is re-read from a run record this build's
    // parse boundary never saw. BOTH classes, composed — `sanitize_render_line`
    // alone left `U+202E`, `U+00AD` and the tag block intact here.
    let sanitised = shown_capped(goal);
    let body_width = usize::from(inner_width).saturating_sub(PREFIX.chars().count());
    let rows = wrap_rows(&sanitised, body_width, 3);
    rows.into_iter()
        .enumerate()
        .map(|(i, row)| {
            Line::from(vec![
                Span::styled(if i == 0 { PREFIX } else { CONTINUATION }, label_style()),
                Span::raw(row),
            ])
        })
        .collect()
}

/// How many rows [`goal_lines`] will take, so the vertical layout can budget.
fn goal_rows(goal: &str, inner_width: u16) -> u16 {
    goal_lines(goal, inner_width).len() as u16
}

/// The run header: goal, command, times, cumulative cost, and the run directory.
///
/// **Nothing here is derived from the agent's prose** (D-13). The goal and the
/// command are the human's own input echoed back; the timestamps come from the
/// run record; the cost comes from a `cost` journal record's `cumulative_usd`.
/// The agent's `ResultMessage.result` may be displayed as content in the output
/// pane below and has no authority over any word in this header.
///
/// **This header renders no status word of its own, and takes no verdict**
/// (WR-11). The state word belongs to the run list and to the step timeline;
/// saying it three times would invite three renderings of one fact. That
/// intention used to be spelled as a `verdict` parameter whose only statement
/// was `let _ = verdict;`, which is a parameter the next edit will silently
/// mis-wire — the compiler cannot tell a future change that forgot to use it
/// from one that deliberately did not. The intent is a sentence, so it is one.
fn render_run_header(
    summary: &RunSummary,
    cost_usd: Option<f64>,
    run_dir: Option<&str>,
    inner_width: u16,
    now: DateTime<Utc>,
) -> Vec<Line<'static>> {
    let mut lines = goal_lines(&summary.goal, inner_width);

    let command = ellipsize(
        &shown_capped(&summary.gsd_command),
        usize::from(inner_width).saturating_sub(30).max(8),
    );
    let (_, started) = local_date_time(&summary.started_at);
    let mut cmd_spans = vec![
        Span::styled("  cmd:  ", label_style()),
        Span::raw(command),
        Span::styled("    started ", label_style()),
        Span::raw(started),
        Span::styled("    ", label_style()),
    ];
    // While a run is live the label reads `elapsed`; once it has ended the value
    // itself carries the `ran` verb, so a second label would read "elapsed ran".
    if summary.ended_at.is_none() {
        cmd_spans.push(Span::styled("elapsed ", label_style()));
    }
    cmd_spans.push(Span::raw(elapsed_label(
        &summary.started_at,
        now,
        summary.ended_at.as_deref(),
    )));
    if let Some(ended_at) = summary.ended_at.as_deref() {
        let (_, ended) = local_date_time(ended_at);
        cmd_spans.push(Span::styled("    ended ", label_style()));
        cmd_spans.push(Span::raw(ended));
    }
    lines.push(Line::from(cmd_spans));

    lines.push(Line::from(match cost_usd {
        Some(usd) => vec![
            Span::styled("  cost  ", label_style()),
            Span::raw(format!("${usd:.2} ")),
            Span::styled(COST_CUMULATIVE, label_style()),
        ],
        None => vec![
            Span::styled("  cost  ", label_style()),
            Span::styled(COST_NOT_REPORTED, muted_style()),
        ],
    }));

    if let Some(dir) = run_dir {
        lines.push(Line::from(Span::styled(
            format!(
                "  {}",
                left_truncate(
                    &shown_capped(dir),
                    usize::from(inner_width).saturating_sub(2).max(8),
                )
            ),
            label_style(),
        )));
    }

    lines
}

/// The copy for a project with no runs, in its two opt-in variants.
///
/// Both name the next key, because an empty state that does not is a dead end.
/// The two are genuinely different situations: one project is ready and has not
/// been asked to do anything, the other has not been given permission.
fn no_runs_lines(alias: &str, opted_in: bool) -> Vec<Line<'static>> {
    // Two classes, two predicates, composed (CR-01): `sanitize_render_line`
    // answers the ESC / C0 / DEL *control* question, `display_identity` answers
    // the invisible-formatting one (`General_Category=Cf` union
    // `Default_Ignorable_Code_Point`). Neither subsumes the other, and this is
    // the only place a registry key is drawn on this path.
    let alias = crate::text::display_identity(&sanitize_render_line(alias));
    if opted_in {
        vec![
            Line::from(format!("  {NO_RUNS_OPTED_IN} \"{alias}\".")),
            Line::from(Span::styled(
                format!("  {NO_RUNS_OPTED_IN_NEXT}"),
                label_style(),
            )),
        ]
    } else {
        vec![
            Line::from(format!("  \"{alias}\" is not opted in to driving.")),
            Line::from(Span::styled(
                format!("  {NOT_OPTED_IN_NEXT}"),
                label_style(),
            )),
        ]
    }
}

/// Render the Driver tab (TRANS-05, OBS-03, OBS-05).
///
/// The layout is two panes — the run list and the run detail — on the 40/60
/// split the Pipeline tab already uses, with the detail pane given a
/// `Constraint::Min` floor rather than a percentage. Below
/// [`DRIVER_TWO_PANE_MIN_CELLS`] the list is dropped and its identity moves into
/// the detail pane's block title.
pub(super) fn render_driver_tab(
    frame: &mut Frame,
    area: Rect,
    ctx: &AppContext,
    alias: &str,
    cache: Option<&ProjectViewCache>,
    viewport: &Cell<ViewportMetrics>,
) {
    let runs: &[RunSummary] = cache.map_or(&[], |c| c.driver_runs.as_slice());

    // The "no state / no data" early return, in the shape `render_pipeline_tab`
    // established: one bordered block, one message, nothing else painted.
    if runs.is_empty() {
        let block = Block::default().borders(Borders::ALL).title(" Driver ");
        // A project with no runs at all is the *most* likely place to be
        // starting one, so the preview has to survive this branch. Showing "no
        // runs yet" over the top of a start the user is one keystroke from
        // confirming would hide the blast radius exactly when it is newest.
        if let Some(preview) = cache.and_then(|c| c.driver_dry_run.as_ref()) {
            let inner = block.inner(area);
            frame.render_widget(block, area);
            render_dry_run_preview(frame, inner, preview);
            return;
        }
        let opted_in = ctx
            .config
            .projects
            .get(alias)
            .is_some_and(|project| project.driver_opt_in.is_some());
        let message = Paragraph::new(no_runs_lines(alias, opted_in)).block(block);
        frame.render_widget(message, area);
        return;
    }

    let selected = cache
        .map_or(0, |c| c.driver_selected_run)
        .min(runs.len() - 1);
    let summary = &runs[selected];

    // The verdict applies only to the run the reconciliation scan is observing;
    // every other row in the list is a finished run whose own `outcome` speaks
    // for it. Applying an observed run's verdict to the wrong row would report
    // one run's liveness as another's.
    let observed = ctx.observed_runs.get(alias);
    let verdict_for = |run: &RunSummary| -> Option<RunVerdict> {
        observed
            .filter(|run_observed| run_observed.run_id == run.run_id)
            .map(|run_observed| run_observed.verdict())
    };

    let (list_area, detail_area) = driver_panes(area);

    if let Some(list_area) = list_area {
        let list_block = Block::default().borders(Borders::RIGHT).title(" Runs ");
        let inner_width = list_block.inner(list_area).width;
        let items: Vec<ListItem> = runs
            .iter()
            .map(|run| ListItem::new(run_list_row(run, verdict_for(run), inner_width)))
            .collect();
        let list = List::new(items)
            .block(list_block)
            .highlight_style(
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(Color::Cyan),
            )
            .highlight_symbol("> ");
        let mut list_state = ListState::default();
        list_state.select(Some(selected));
        frame.render_stateful_widget(list, list_area, &mut list_state);
    }

    // At the narrow tier the selection has nowhere else to go, so it becomes the
    // pane's title. `j`/`k` still move it; only the list's rendering is dropped.
    let detail_block = if list_area.is_some() {
        Block::default().borders(Borders::NONE).title(" Driver ")
    } else {
        Block::default()
            .borders(Borders::NONE)
            .title(run_list_title(summary, selected, runs.len()))
    };
    let inner = detail_block.inner(detail_area);
    frame.render_widget(detail_block, detail_area);

    // The dry-run preview replaces the run detail while the start flow is at
    // Step B, and replaces nothing else (D-26). See `render_dry_run_preview`.
    match cache.and_then(|c| c.driver_dry_run.as_ref()) {
        Some(preview) => render_dry_run_preview(frame, inner, preview),
        None => render_run_detail(
            frame,
            inner,
            ctx,
            alias,
            cache,
            summary,
            verdict_for(summary),
            viewport,
        ),
    }
}

/// Render the dry-run preview in place of the run detail (D-26).
///
/// **What this is.** While `DriverStartScreen` is on the stack at Step B, the
/// body's run-detail pane shows what the run about to be confirmed would touch:
/// the GSD command sequence, the working tree a commit would capture, and the
/// push refspecs the current state would produce. Zero new keys and zero new
/// modes — it is the pane the user is already looking at, and it is literally
/// "before a start is confirmed". This is the only place in v2.0 where a user
/// sees blast radius **before** an autonomous agent with push rights starts.
///
/// **The cut rule (D-26).** This is the phase's *designated cut*: it is the only
/// surface item in Phase 18 with no requirement id, and if it ever competes for
/// space it is the first thing to drop. If it is dropped, the body simply shows
/// the normal run detail again and **nothing else changes** — the one call site
/// above is a `match` arm, and the preview owns no key, no mode and no state
/// outside `ProjectViewCache::driver_dry_run`.
///
/// **The fence (Phase 19).** Phase 18 *surfaces* the report and does not police
/// it. Push allowlists, `--disallowedTools`, pre-push hooks, secret scanning and
/// worktree isolation are **Phase 19's**, and nothing here may imply that seeing
/// a refspec prevents pushing to it.
///
/// **Untrusted input.** The report interpolates paths and branch names read from
/// the project, so every row goes through the shared
/// [`sanitize_render_line`] before rendering — otherwise a branch named with an
/// escape sequence could repaint the screen or forge a status line (T-18-61).
/// Nothing on this pane is derived from an agent's prose (D-13).
///
/// **Loading.** Until the `spawn_blocking` build returns, the pane shows the
/// shipped [`DRY_RUN_LOADING`] idiom in DarkGray. Keys stay inert and nothing
/// panics; there is no spinner, because there is no honest thing for one to say.
///
/// ---
///
/// **D-26 in one paragraph, kept adjacent to the signature so it survives a
/// skim:** this preview is the phase's *designated cut* — the only surface item
/// with no requirement id. Drop it and the body shows the normal run detail,
/// with nothing else changing. And the **Phase 19 fence**: Phase 18 surfaces
/// this report, it does not police it — push allowlists, tool denial, pre-push
/// hooks, secret scanning and worktree isolation all belong to Phase 19.
pub(super) fn render_dry_run_preview(frame: &mut Frame, area: Rect, preview: &DryRunPreview) {
    let mut lines: Vec<Line<'static>> = match preview.report.as_deref() {
        // Unknown is not "nothing would happen".
        None => vec![Line::from(Span::styled(DRY_RUN_LOADING, muted_style()))],
        // The report interpolates paths and branch names read from the project
        // — a repository the operator cloned, not one this build authored. Both
        // classes, composed through `shown_capped`.
        //
        // **This site is held by a CALL, not by the type.** `DryRunPreview` is
        // built in `src/app.rs`, outside this wave's fence, so `report` is
        // still `Option<String>` and a NEW render of it would not fail to
        // compile. What bounds that is the `Driver tab, dry-run preview` probe
        // state, not the compiler — see LIMIT 1's residual (D-21-39).
        Some(report) => report
            .lines()
            .map(|raw| Line::from(shown_capped(raw)))
            .collect(),
    };

    // The report is longer than a short pane, and a pane that silently drops the
    // refspec section is precisely the PITFALLS:69 failure this preview exists
    // to prevent — so say so, with the same indicator and the same predicate the
    // help popup uses rather than a second copy of either.
    let total = lines.len() as u16;
    if super::help::more_below(0, total, area.height) && area.height > 0 {
        let last = area.height as usize - 1;
        lines.truncate(last);
        lines.push(Line::from(Span::styled(
            super::help::MORE_INDICATOR,
            Style::default().fg(Color::DarkGray),
        )));
    }

    frame.render_widget(Paragraph::new(lines), area);
}

/// The vertical layout of the run-detail pane, in three height tiers.
///
/// **No section is ever allocated zero rows**: a section that cannot fit is
/// dropped by the tier rather than squeezed to nothing, because a one-row
/// section rule with no content under it reads as a section that failed to load.
#[allow(clippy::too_many_arguments)]
fn render_run_detail(
    frame: &mut Frame,
    area: Rect,
    ctx: &AppContext,
    alias: &str,
    cache: Option<&ProjectViewCache>,
    summary: &RunSummary,
    verdict: Option<RunVerdict>,
    viewport: &Cell<ViewportMetrics>,
) {
    let now = Utc::now();
    let tally = cache.and_then(|c| c.driver_tally.as_ref());
    let cost_usd = tally
        .filter(|tally| tally.run_id == summary.run_id)
        .and_then(|tally| tally.cumulative_cost_usd);
    let turns = tally
        .filter(|tally| tally.run_id == summary.run_id)
        .map(|tally| tally.turn_boundaries + 1);

    let run_dir = ctx.config.projects.get(alias).and_then(|project| {
        crate::journal::run_paths(&project.path.join(".planning"), &summary.run_id)
            .map(|paths| paths.dir.display().to_string())
    });

    let inference = ctx
        .project_states
        .get(alias)
        .and_then(|state| state.current_phase_status.as_ref());

    let goal_row_count = goal_rows(&summary.goal, area.width);
    let header_rows = goal_row_count + 3;
    // The section rule, the one decided row this phase has (D-12), and the
    // mandatory honest note.
    let step_rows = 3;

    let chunks: Vec<Rect> = if area.height >= 14 {
        Layout::vertical([
            Constraint::Length(header_rows),
            Constraint::Length(2),
            Constraint::Length(step_rows),
            Constraint::Min(3),
        ])
        .split(area)
        .to_vec()
    } else if area.height >= 8 {
        // Every section is still present, at one row each. **No section is ever
        // allocated zero rows** — a section that cannot fit is dropped by the
        // tier, never squeezed to nothing, because an empty section rule reads
        // as a section that failed to load.
        Layout::vertical([
            Constraint::Length(2),
            Constraint::Length(1),
            Constraint::Length(1),
            Constraint::Min(3),
        ])
        .split(area)
        .to_vec()
    } else {
        Layout::vertical([Constraint::Length(1), Constraint::Min(1)])
            .split(area)
            .to_vec()
    };

    if area.height < 8 {
        // The shortest tier: a one-row identity header and the output pane.
        let (date, time) = local_date_time(&summary.started_at);
        frame.render_widget(
            Paragraph::new(Line::from(Span::styled(
                format!(
                    "  {date} {time} {}",
                    shown_capped(&run_id_suffix(&summary.run_id))
                ),
                label_style(),
            ))),
            chunks[0],
        );
        render_output_section(
            frame, chunks[1], ctx, alias, cache, summary, verdict, now, viewport,
        );
        return;
    }

    let header = render_run_header(summary, cost_usd, run_dir.as_deref(), area.width, now);
    let header_budget = usize::from(chunks[0].height);
    frame.render_widget(
        Paragraph::new(header.into_iter().take(header_budget).collect::<Vec<_>>()),
        chunks[0],
    );

    render_pipeline_row(frame, chunks[1], inference);
    render_steps(frame, chunks[2], summary, verdict, turns);
    render_output_section(
        frame, chunks[3], ctx, alias, cache, summary, verdict, now, viewport,
    );
}

/// The D-R-P-E-V pipeline row — **called, never re-implemented** (D-17).
///
/// [`super::detail::derive_all_stage_statuses`] and
/// [`super::detail::build_pipeline_line`] are free functions over
/// `&DiskInference`, invoked here exactly as `render_pipeline_tab` invokes them,
/// with their existing two-cell indent, their existing per-stage colours and
/// their existing `[--]` rendering for a skipped stage. Nothing about the widget
/// is modified, wrapped, restyled or duplicated: **a second progress display is
/// a named anti-feature** and D-17 makes reuse mandatory.
///
/// `ARCHITECTURE` M5 proposes lifting those functions out of `detail.rs` into
/// `state_reader/`. That is **explicitly declined for this phase**, and the
/// decline is recorded at the call site so a later reader knows it was
/// considered rather than missed: the driver process does not render, Phase 20's
/// `decide()` is the first genuine second consumer, and moving 130 lines now
/// would churn the largest file in the repository to buy nothing.
///
/// The inference is the project's **current phase**'s, which is what makes this
/// row answer the question a driven repository raises — how far through the
/// pipeline is the thing the agent is working on.
fn render_pipeline_row(
    frame: &mut Frame,
    area: Rect,
    inference: Option<&crate::state_reader::disk_status::DiskInference>,
) {
    let mut lines: Vec<Line<'static>> = vec![Line::from("")];
    match inference {
        Some(inf) => {
            let statuses = super::detail::derive_all_stage_statuses(inf);
            lines.push(super::detail::build_pipeline_line(inf, &statuses));
        }
        None => lines.push(Line::from(Span::styled("  No disk data", label_style()))),
    }
    frame.render_widget(Paragraph::new(lines), area);
}

/// The step timeline: **exactly one decided row in this phase**, plus the note
/// that says so.
///
/// **Turns are turns, not commands** (D-12, Phase 15 D-29). A steered run emits
/// several `system/init` and `result` pairs inside one process; the timeline
/// shows the one command that was *decided* and the counter shows the turns. A
/// later `system/init` is informational and is never rendered as a restart. The
/// section carries N rows structurally so Phase 20's router needs no re-layout,
/// while this phase supplies one — and says so, because a one-row timeline with
/// no explanation implies a computed sequence that does not exist.
///
/// **The state word comes from evidence only, never from the agent's prose**
/// (D-13). [`run_state_glyph`] reads `RunVerdict` — the record's `ended_at` plus
/// a pid/cmdline liveness probe — and `RunRecord.outcome`, the rendered form of
/// the four-source derivation Phase 15 computes from `is_error`,
/// `terminal_reason`, the permission-denial list, the exit code and a git/disk
/// snapshot. **No string comparison against what the agent said about itself
/// decides what this row says.** The agent's own account may appear as content
/// in the output pane below and carries no authority here.
fn render_steps(
    frame: &mut Frame,
    area: Rect,
    summary: &RunSummary,
    verdict: Option<RunVerdict>,
    turns: Option<u32>,
) {
    frame.render_widget(
        Paragraph::new(steps_lines(summary, verdict, turns, area.width)),
        area,
    );
}

/// The step section's lines, as a pure function so the whole thing is assertable
/// without a terminal (S6).
fn steps_lines(
    summary: &RunSummary,
    verdict: Option<RunVerdict>,
    turns: Option<u32>,
    width: u16,
) -> Vec<Line<'static>> {
    let live = matches!(verdict, Some(RunVerdict::Live));
    // **The run detail's word, not the run list's.** The list row is 18 cells at
    // every width and carries the abbreviation; this row is the one place the
    // full evidence-derived word fits, and it is what makes a finished run
    // reviewable after the fact rather than merely glanceable (OBS-05).
    let (_, word, color) = terminal_state_cell(TerminalState::observed(
        verdict,
        summary.outcome.as_deref(),
    ));
    let (_, started) = local_date_time(&summary.started_at);

    let command_style = if live {
        Style::default().add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };

    let mut lines: Vec<Line<'static>> = vec![section_rule("steps", width)];

    // N rows structurally, one row supplied — Phase 20's router adds the rest
    // without re-laying-out this section.
    let decided: [&RunSummary; 1] = [summary];
    for step in decided {
        let mut spans = vec![
            Span::styled(
                format!("  {}", shown_capped(&step.gsd_command)),
                command_style,
            ),
            Span::styled(format!("   {started}   "), label_style()),
            Span::styled(
                if live {
                    "running".to_string()
                } else {
                    word.clone()
                },
                Style::default().fg(color),
            ),
        ];
        if live {
            if let Some(turns) = turns {
                spans.push(Span::styled(format!("   turn {turns}"), label_style()));
            }
        }
        lines.push(Line::from(spans));
    }

    lines.push(Line::from(Span::styled(
        format!("  {STEPS_HONEST_NOTE}"),
        muted_style(),
    )));

    lines
}

/// The right-aligned state of the output pane, as a word and a colour.
///
/// Four states, and each says something the others do not:
///
/// * **following** — the pane is live and pinned to the tail.
/// * **scrolled +N** — the pane is live and the user has moved off the tail; `N`
///   is how many buffered lines sit below the viewport. It deliberately does
///   **not** say *paused*: `paused` already means *"a non-empty HANDOFF is
///   present"* everywhere else in this tool, and spending the word on a scroll
///   state would make the vocabulary lie.
/// * **journal only** — this run started before the current TUI session. Its
///   live output is gone and what is shown came off disk.
/// * **ended HH:MM** — the run is over and the journal has stopped growing.
///
/// Adopted only qualifies an ended run: while an adopted run is still going its
/// journal *is* being tailed, so the follow bit is meaningful and the pane's
/// first row carries the provenance instead.
pub fn follow_indicator(
    live: bool,
    following: bool,
    below: usize,
    ended_at: Option<&str>,
    adopted: bool,
) -> (String, Color) {
    if live {
        return if following {
            (INDICATOR_FOLLOWING.to_string(), Color::Green)
        } else if below > SCROLLED_DISPLAY_MAX {
            (format!("[scrolled +{SCROLLED_DISPLAY_MAX}]"), Color::Yellow)
        } else {
            (format!("[scrolled +{below}]"), Color::Yellow)
        };
    }
    if adopted {
        return (INDICATOR_JOURNAL_ONLY.to_string(), Color::DarkGray);
    }
    match ended_at {
        Some(ended_at) => {
            let (_, time) = local_date_time(ended_at);
            (format!("[ended {time}]"), Color::DarkGray)
        }
        // A run with no terminal record and no observation. Saying `[ended]`
        // would assert a fact nothing on disk supports.
        None => (INDICATOR_JOURNAL_ONLY.to_string(), Color::DarkGray),
    }
}

/// The output pane's header: a DarkGray rule with the indicator on its right.
///
/// The rule is the pane's own row rather than the first line of the scrolling
/// paragraph, so the indicator does not scroll away the moment it becomes
/// interesting. At a width too narrow for both, the dashes give way first and
/// the indicator is what survives.
fn output_header_line(width: u16, indicator: &str, color: Color) -> Line<'static> {
    let head = "\u{2500}\u{2500} output ";
    let tail_cells = indicator.chars().count() + 4; // ` ` + indicator + ` ──`
    let pad = usize::from(width)
        .saturating_sub(head.chars().count())
        .saturating_sub(tail_cells);
    Line::from(vec![
        Span::styled(
            format!("{head}{}", "\u{2500}".repeat(pad)),
            label_style(),
        ),
        Span::raw(" "),
        Span::styled(indicator.to_string(), Style::default().fg(color)),
        Span::styled(" \u{2500}\u{2500}", label_style()),
    ])
}

// ── The four-state injection display (STEER-02, D-07, D-10) ────────────────

/// What the evidence on disk supports about one injected message.
///
/// Four states, one authoritative observer each, and **the vocabulary is a
/// safety property rather than a style choice**: the word for a write to the
/// agent's stdin is *delivered* and nothing else, the word for the dequeue echo
/// is *acted-on*, and "sent", "received", "read" and "acknowledged" are all
/// forbidden — the echo arrives roughly fifty-five seconds after the write, so
/// each of those four would claim an observation the protocol cannot make. The
/// filling shape `○ → ◐ → ●` is the whole design: it reads correctly across a
/// minute-long gap with **no spinner, no animation and no implied imminence**.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InjectionState {
    /// Durably in `inbox.jsonl` and nothing has read it.
    Queued,
    /// The write to the agent's stdin returned without error.
    Delivered,
    /// The agent dequeued it and is running it as its own turn.
    ActedOn,
    /// Undeliverable, and never retried (D-10). The payload is **why**, which
    /// selects the pinned gloss on the message's third row.
    Missed(MissedReason),
}

/// Why a message is undeliverable, as a value rather than as a string to parse.
///
/// The state and its explanation travel together for the same reason the glyph,
/// the label and the colour do in [`InjectionState::cell`]: a reader must never
/// have to pair two independently-derived facts to learn one thing. There are
/// exactly two reasons a driver can record, and a third for a journal that names
/// a reason this build does not know — which is a case the render layer answers
/// honestly rather than by guessing one of the other two.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MissedReason {
    /// The message reached the driver after the agent's stdin was closed. Stdin
    /// cannot be reopened, so there was nowhere to write it.
    AfterClose,
    /// The write to the agent's stdin returned an error — the writer task was
    /// gone, or the agent was no longer running. The driver **did** read the
    /// message; the write is what failed (CR-03).
    SendFailed,
    /// The journal named a reason this build has no gloss for.
    Unrecognised,
}

impl MissedReason {
    /// The reason a journal record's `reason` field names.
    ///
    /// Matching on the exact pinned string, never on a substring: the two
    /// reasons are constants in [`crate::journal`] precisely so this is an
    /// equality test rather than prose parsing (D-01's rule in another guise).
    fn from_reason(reason: &str) -> Self {
        match reason {
            crate::journal::MISSED_AFTER_CLOSE => Self::AfterClose,
            crate::journal::MISSED_SEND_FAILED => Self::SendFailed,
            _ => Self::Unrecognised,
        }
    }

    /// The pinned one-line gloss for this reason.
    pub fn gloss(self) -> &'static str {
        match self {
            Self::AfterClose => MISSED_GLOSS,
            Self::SendFailed => MISSED_GLOSS_SEND_FAILED,
            Self::Unrecognised => MISSED_GLOSS_UNRECOGNISED,
        }
    }
}

impl InjectionState {
    /// How far along the progression a state sits, so **a later state wins**.
    ///
    /// `Missed` outranks `Delivered` and is outranked by `ActedOn`. The driver
    /// cannot produce that contradiction — it writes `missed` only once stdin is
    /// closed and `acted_on` only from the agent's own echo — but if a journal
    /// ever carried both, the record naming the agent's own dequeue is the
    /// stronger evidence and an undeliverability claim must not overrule it.
    fn rank(self) -> u8 {
        match self {
            Self::Queued => 0,
            Self::Delivered => 1,
            Self::Missed(_) => 2,
            Self::ActedOn => 3,
        }
    }

    /// The glyph, the **exact** label and the colour, travelling together so no
    /// meaning is ever carried by colour alone.
    pub fn cell(self) -> (&'static str, &'static str, Color) {
        match self {
            Self::Queued => (GLYPH_QUEUED, LABEL_QUEUED, Color::DarkGray),
            Self::Delivered => (GLYPH_DELIVERED, LABEL_DELIVERED, Color::Yellow),
            Self::ActedOn => (GLYPH_ACTED_ON, LABEL_ACTED_ON, Color::Green),
            // One glyph and one word for both reasons: the *state* is the same
            // — undeliverable, terminal, never retried — and only the
            // explanation on the row below differs. A second label would be a
            // fifth state in the four-state vocabulary.
            Self::Missed(_) => (GLYPH_MISSED, LABEL_MISSED, Color::Red),
        }
    }
}

/// One injected message, the state disk supports for it, and when that state was
/// last observed.
///
/// A tuple rather than a struct because every one of the three is read at the
/// one call site that renders it, and the timestamp is `None` exactly when the
/// transition's own `ts` did not parse — which is the case the elapsed counter
/// must omit rather than fill with a zero.
pub type InjectionEntry = (InboxMessage, InjectionState, Option<DateTime<Utc>>);

/// The state of every injected message, as a **pure function of disk**.
///
/// The rule is a set intersection and nothing more: an id present in
/// `inbox.jsonl` and in no journal record is [`InjectionState::Queued`]; present
/// in an `interjected` record is [`InjectionState::Delivered`]; present in an
/// `interjection_acted_on` record is [`InjectionState::ActedOn`]; present in an
/// `interjection_missed` record is [`InjectionState::Missed`]. Later states win
/// over earlier ones ([`InjectionState::rank`]).
///
/// **The one qualification is a refusal to overstate — in BOTH directions**
/// (CR-03). `interjected` carries a `delivered` flag which is exactly *"did
/// `Executor::send` return `Ok`"*, and the driver writes the record either way —
/// so a record with `delivered: false` is the journal saying the write
/// **failed**. Promoting it to `delivered` would assert a state the evidence
/// contradicts, which is the one thing this surface must never do. But leaving
/// it in `queued` — glossed *"durably on disk; nothing has read it yet"* — is
/// the same error pointing the other way, and it is the worse of the two: the
/// driver **did** read the message, the write **did** fail, and the cursor has
/// already moved past it so nothing will ever read it again. A user shown
/// "waiting to be picked up" about a message that will never be picked up is
/// PITFALLS' undelivered-injection failure exactly. `delivered: false` is
/// therefore positive evidence of a failed write and resolves to
/// [`InjectionState::Missed`] with [`MissedReason::SendFailed`].
///
/// The driver writes a matching `interjection_missed { reason }` record for the
/// same id, so this is normally the second of two independent witnesses. It is
/// kept as its own rule rather than delegated to that record because the
/// driver's own journal write can fail — it warns and continues — and a message
/// must not fall back into `queued` when it does.
///
/// **Nothing is held in memory that a restart would lose.** That is precisely
/// what makes STEER-03 hold across a TUI restart with no extra persistence: the
/// answer is recomputed from the two files every time, so there is no cache to
/// go stale and none to rebuild.
///
/// Pure, so the whole table is testable without a terminal or a process — the
/// register of `driver/mod.rs:129-135`.
pub fn derive_injection_states(
    inbox: &[InboxMessage],
    records: &[JournalRecord],
) -> Vec<InjectionEntry> {
    inbox
        .iter()
        .map(|message| {
            let mut state = InjectionState::Queued;
            let mut at: Option<DateTime<Utc>> = None;
            for record in records {
                // The correlation id lives in a **typed field**. Reconstructing
                // a protocol state by parsing a rendered string is the
                // screen-scraping D-01 forbids in another guise (D-08).
                let id = record.rest.get("id").and_then(|value| value.as_str());
                if id != Some(message.id.as_str()) {
                    continue;
                }
                let observed = match record.kind.as_str() {
                    "interjected" => {
                        let written = record
                            .rest
                            .get("delivered")
                            .and_then(|value| value.as_bool())
                            .unwrap_or(false);
                        if written {
                            InjectionState::Delivered
                        } else {
                            InjectionState::Missed(MissedReason::SendFailed)
                        }
                    }
                    "interjection_acted_on" => InjectionState::ActedOn,
                    "interjection_missed" => InjectionState::Missed(MissedReason::from_reason(
                        record
                            .rest
                            .get("reason")
                            .and_then(|value| value.as_str())
                            .unwrap_or_default(),
                    )),
                    _ => continue,
                };
                if observed.rank() >= state.rank() {
                    state = observed;
                    at = parse_rfc3339(&record.ts);
                }
            }
            (message.clone(), state, at)
        })
        .collect()
}

/// `HH:MM:SS` in the reader's local zone, or a placeholder.
fn local_time_of_day(ts: &str) -> String {
    match parse_rfc3339(ts) {
        Some(dt) => dt.with_timezone(&Local).format("%H:%M:%S").to_string(),
        None => "??:??:??".to_string(),
    }
}

/// `M:SS`, widening to `H:MM:SS` past an hour.
///
/// The elapsed counter's own form rather than [`hms`]'s, because this figure is
/// read as a *duration since* rather than as a clock time and a leading `00:`
/// on a forty-seven-second gap reads as a stopped counter.
fn injection_elapsed(seconds: i64) -> String {
    let hours = seconds / 3600;
    let minutes = (seconds % 3600) / 60;
    let secs = seconds % 60;
    if hours > 0 {
        format!("{hours}:{minutes:02}:{secs:02}")
    } else {
        format!("{minutes}:{secs:02}")
    }
}

/// One injected message: **always two rows, three when missed, at every width**.
///
/// ```text
/// » ◐ delivered  21:40:02  (+0:47)
///     skip the UI review
/// ```
///
/// Row one is the Cyan+BOLD injection marker, the state glyph, the exact state
/// label and the queue timestamp; row two is a four-cell-indented, sanitised,
/// verbatim copy of the message text. The delivered state adds an elapsed
/// counter to row one and the missed state adds a third row carrying the pinned
/// gloss.
///
/// **There is no width parameter and that is the design**: always two rows means
/// no width branch, a perfectly aligned state column, and no right-alignment
/// arithmetic that could clip in the narrowest pane this tab can reach.
///
/// The elapsed counter is the honest answer to the fifty-five-second gap — it
/// shows time passing **without predicting an arrival**, which a spinner cannot
/// do. It is omitted rather than shown as a zero when the transition's own
/// timestamp did not parse: a duration the code cannot compute is not a duration
/// of nothing.
pub fn injection_rows(entry: &InjectionEntry, now: DateTime<Utc>) -> Vec<Line<'static>> {
    let (message, state, at) = entry;
    let (glyph, label, color) = state.cell();

    let mut head = vec![
        Span::styled(
            MARKER_INJECTION,
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!("{glyph} {label}"), Style::default().fg(color)),
        Span::styled(format!("  {}", local_time_of_day(&message.ts)), label_style()),
    ];
    if *state == InjectionState::Delivered {
        if let Some(at) = at {
            let seconds = (now - *at).num_seconds().max(0);
            head.push(Span::styled(
                format!("  (+{})", injection_elapsed(seconds)),
                label_style(),
            ));
        }
    }

    let mut rows = vec![
        Line::from(head),
        // `message.text` is `InboxMessage::text`: the user's text stored
        // VERBATIM and deliberately un-redacted, read back from `inbox.jsonl`
        // on disk — a file this build does not exclusively own. Both classes,
        // composed through `shown_capped`.
        //
        // **This site is held by a CALL, not by the type.** `InboxMessage`
        // lives in `src/journal/inbox.rs`, outside this wave's fence, so its
        // `text` field is still a bare `String` and a NEW render of it would
        // not fail to compile. What bounds that is the probe fixture, not the
        // compiler — see LIMIT 1's residual and its failure direction (D-21-39).
        Line::from(Span::raw(format!(
            "{INJECTION_INDENT}{}",
            shown_capped(&message.text)
        ))),
    ];
    if let InjectionState::Missed(reason) = state {
        rows.push(Line::from(Span::styled(
            format!("{INJECTION_INDENT}{}", reason.gloss()),
            muted_style(),
        )));
    }
    rows
}

/// Every injected message's rows, in queue order.
///
/// **A run with no injected messages produces nothing at all** — not an empty
/// queued placeholder, which would invent a message the user never typed.
fn injection_block(entries: &[InjectionEntry], now: DateTime<Utc>) -> Vec<Line<'static>> {
    entries
        .iter()
        .flat_map(|entry| injection_rows(entry, now))
        .collect()
}

/// One buffered line, with its two-cell marker in its own style.
///
/// The four visual classes are the UI-SPEC's event-kind table. `terminal_color`
/// is the run's terminal-state colour, which is derived from evidence and never
/// from the agent's prose (D-13) — see [`run_state_glyph`].
fn output_line(line: &DriverOutputLine, terminal_color: Color) -> Line<'static> {
    let injection_style = Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD);
    let terminal_style = Style::default()
        .fg(terminal_color)
        .add_modifier(Modifier::BOLD);
    let (marker, marker_style, text_style) = match line.kind {
        DriverLineKind::Output => (MARKER_OUTPUT, Style::default(), Style::default()),
        DriverLineKind::Stderr => (MARKER_STDERR, label_style(), label_style()),
        DriverLineKind::Injection => (MARKER_INJECTION, injection_style, Style::default()),
        DriverLineKind::Diagnostic => (
            MARKER_DIAGNOSTIC,
            Style::default().fg(Color::Yellow),
            Style::default().fg(Color::Yellow),
        ),
        DriverLineKind::Terminal => (MARKER_TERMINAL, terminal_style, terminal_style),
    };
    Line::from(vec![
        Span::styled(marker, marker_style),
        // `line.text` is `crate::text::Untrusted` — the agent's own prose, read
        // back off disk. `shown()` is not a courtesy here: it is the only way
        // this value can reach a `Span` at all, because the carrier implements
        // none of the string conversions (21-28 T1, CR-02).
        Span::styled(line.text.shown(), text_style),
    ])
}

/// The scrolling body of the output pane, as a pure function (S6).
///
/// Order, and every part of it is load-bearing:
///
/// 1. The **adopted-run notice**, when this session did not spawn the run. It
///    is above the drop notice because it describes where every row below came
///    from, while the drop notice describes only the buffer's own shortfall.
/// 2. The **ring-overflow notice**, when the buffer has dropped lines. It
///    describes what is missing from everything below it.
/// 3. The buffered lines, in order, each with its marker — with the terminal
///    record held back, and with the injection records **superseded** by the
///    four-state widget at the position of the first of them.
/// 4. The **record-truncation notice**, when one record was cut at the
///    per-record cap.
/// 5. The terminal record, which renders **last**: the visual full stop.
///
/// The injection substitution is why the widget is spliced rather than appended:
/// an `interjected` record's own line carries the text and the two that follow
/// it carry a bare correlation id, so leaving them in beside the widget would
/// print the same message twice and the id twice more. When there are **no**
/// entries to render — an unreadable inbox, or a journal from a build that
/// recorded no ids — the raw lines stay exactly where they are, because a
/// transition that cannot be paired with a message is still evidence and this
/// pane never silently swallows one.
///
/// A run whose journal has no records **and** no queued messages renders the
/// pinned no-entries copy and nothing else.
fn output_body_lines(
    output: Option<&DriverOutput>,
    entries: &[InjectionEntry],
    now: DateTime<Utc>,
    adopted: bool,
    terminal_color: Color,
) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = Vec::new();

    if adopted {
        lines.push(Line::from(Span::styled(ADOPTED_RUN_NOTICE, muted_style())));
    }

    let Some(output) = output.filter(|output| !output.is_empty()) else {
        if entries.is_empty() {
            lines.push(Line::from(Span::styled(
                format!("  {NO_JOURNAL_ENTRIES}"),
                label_style(),
            )));
        } else {
            // A run whose journal has not been written yet but whose inbox
            // already holds a message: the message is real and durably on disk,
            // so it renders. Saying "no journal entries" over it would hide the
            // one thing that is there.
            lines.extend(injection_block(entries, now));
        }
        return lines;
    };

    if output.dropped() > 0 {
        lines.push(Line::from(Span::styled(
            ring_overflow_notice(output.dropped()),
            muted_style(),
        )));
    }

    let mut terminal: Vec<Line<'static>> = Vec::new();
    let mut injections_placed = entries.is_empty();
    for line in output.lines() {
        match line.kind {
            DriverLineKind::Terminal => terminal.push(output_line(line, terminal_color)),
            DriverLineKind::Injection if !entries.is_empty() => {
                if !injections_placed {
                    lines.extend(injection_block(entries, now));
                    injections_placed = true;
                }
            }
            _ => lines.push(output_line(line, terminal_color)),
        }
    }
    // Every message is still queued, so the journal carries no transition for
    // the widget to land beside. The tail is where it belongs: a queued message
    // is the newest thing that has happened.
    if !injections_placed {
        lines.extend(injection_block(entries, now));
    }

    if output.record_truncated() {
        lines.push(Line::from(Span::styled(
            RECORD_TRUNCATED_NOTICE,
            muted_style(),
        )));
    }

    lines.extend(terminal);
    lines
}

/// Which buffer speaks for the selected run.
///
/// The live ring on `AppContext` is **per alias** and holds the run this session
/// is tailing; `ProjectViewCache::driver_journal` is the *selected* run's
/// journal as the scan read it whole off disk. Rendering the tailed run's ring
/// under a run the user is reviewing from last week would attribute one run's
/// output to another — the same error the run id inside `DriverRunTally` exists
/// to prevent — so the choice is made by run id and by nothing else, and a
/// journal that is about a different run is not shown at all.
///
/// **The ring is matched on its own run id, not on `observed_runs`** (CR-02).
/// The reconciliation map answers "which run is this session tailing", which is
/// a different question from "which run do these lines belong to" and stops
/// being the same answer the moment a run ends: `reconcile_one` drops an ended
/// run from the map while its lines are still the newest thing on the surface.
/// [`DriverOutput::run_id`] is the buffer's own account of itself and cannot go
/// stale relative to the lines beside it.
///
/// **The live ring is a preference, not a short-circuit** (WR-07). An empty ring
/// falls through to the journal rather than returning `None`, because an adopted
/// run — or any run after a TUI restart — has an empty ring until the first
/// watcher event fires, and short-circuiting there paints "No journal entries
/// yet." directly under the notice saying the journal on disk is what is being
/// shown. The scan already read that journal whole for exactly this case.
fn output_for_run<'a>(
    ctx: &'a AppContext,
    alias: &str,
    cache: Option<&'a ProjectViewCache>,
    run_id: &str,
) -> Option<&'a DriverOutput> {
    let live = ctx
        .driver_output
        .get(alias)
        .filter(|output| output.run_id() == run_id)
        .filter(|output| !output.is_empty());
    if live.is_some() {
        return live;
    }
    cache
        .and_then(|cache| cache.driver_journal.as_deref())
        .filter(|journal| journal.run_id == run_id)
        .map(|journal| &journal.output)
}

/// The live output pane (OBS-04, OBS-05).
///
/// **This function renders live output for a run this session spawned and a
/// journal that has stopped growing for one it did not, and it never promises
/// the first for the second.** Once the TUI exits the child's stdout pipe is
/// gone, so reattachment is read-only and journal-based (Phase 17 D-11); the
/// adopted run's first row says so and the indicator reads journal-only. That is
/// a rule on this function rather than an observation about it.
///
/// The viewer is the Browse file viewer's shape: build the whole `Vec<Line>`,
/// take `total_lines` from **that vector** after it is fully built, record the
/// metrics into the shared `Cell<ViewportMetrics>` during the render pass, and
/// render `Paragraph::new(lines).scroll((offset, 0))` with **wrapping off** —
/// `Wrap` breaks the `total_lines` ↔ offset correspondence the clamp depends on.
///
/// **The follow bit is resolved here and nowhere else.** While it is set the
/// pane shows the tail, expressed as `u16::MAX` through the one shared
/// [`clamp_scroll`]; a second `total_lines - visible_height` written out
/// anywhere is how UIFIX-04 would come back in a new place (D-19).
#[allow(clippy::too_many_arguments)]
fn render_output_section(
    frame: &mut Frame,
    area: Rect,
    ctx: &AppContext,
    alias: &str,
    cache: Option<&ProjectViewCache>,
    summary: &RunSummary,
    verdict: Option<RunVerdict>,
    now: DateTime<Utc>,
    viewport: &Cell<ViewportMetrics>,
) {
    let rows = Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(area);
    let (rule_area, body_area) = (rows[0], rows[1]);

    let (_, _, terminal_color) = run_state_glyph(verdict, summary.outcome.as_deref());
    let adopted = !ctx.session_spawned_runs.contains(&summary.run_id);

    // **One derivation, read from disk, never a second copy.** The inbox and the
    // journal both came off disk in the same scan, and the state is recomputed
    // from them here rather than cached anywhere — which is what makes the
    // display survive a TUI restart with no persisted state (STEER-03).
    let entries = cache.map_or_else(Vec::new, |cache| {
        let records = cache
            .driver_journal
            .as_deref()
            .filter(|journal| journal.run_id == summary.run_id)
            .map_or(&[][..], |journal| journal.injections.as_slice());
        derive_injection_states(&cache.driver_inbox, records)
    });

    let body = output_body_lines(
        output_for_run(ctx, alias, cache, &summary.run_id),
        &entries,
        now,
        adopted,
        terminal_color,
    );

    // `total_lines` is taken **after** the vector is fully built, so the offset
    // always clamps against what is actually rendered.
    let total_lines = body.len() as u16;
    let metrics = ViewportMetrics {
        total_lines,
        visible_height: body_area.height,
    };
    viewport.set(metrics);

    let following = cache.is_some_and(|c| c.driver_follow);
    // `u16::MAX` means "the tail, whatever it is"; the shared clamp resolves it.
    let requested = if following {
        u16::MAX
    } else {
        cache.map_or(0, |c| c.driver_scroll_offset)
    };
    let scroll = clamp_scroll(requested, total_lines, body_area.height);
    let below = usize::from(tail_offset(metrics).saturating_sub(scroll));

    let live = matches!(verdict, Some(RunVerdict::Live));
    let (indicator, color) = follow_indicator(
        live,
        following,
        below,
        summary.ended_at.as_deref(),
        adopted,
    );
    frame.render_widget(
        Paragraph::new(output_header_line(rule_area.width, &indicator, color)),
        rule_area,
    );
    frame.render_widget(Paragraph::new(body).scroll((scroll, 0)), body_area);
}

/// A DarkGray section rule: `── name ─────…` padded to the pane width.
fn section_rule(name: &str, width: u16) -> Line<'static> {
    let head = format!("\u{2500}\u{2500} {name} ");
    let pad = usize::from(width).saturating_sub(head.chars().count());
    Line::from(Span::styled(
        format!("{head}{}", "\u{2500}".repeat(pad)),
        label_style(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn summary(run_id: &str, outcome: Option<&str>) -> RunSummary {
        RunSummary {
            run_id: run_id.to_string(),
            started_at: "2026-07-29T21:40:00Z".to_string(),
            ended_at: None,
            goal: "ship the driver tab".to_string(),
            gsd_command: "/gsd:execute-phase 18".to_string(),
            outcome: outcome.map(str::to_string),
        }
    }

    /// The visible text of a `Line`, spans concatenated.
    fn text(line: &Line<'static>) -> String {
        line.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    /// Every input the composition-equality pin is measured over: both members
    /// of every `LOOK_ALIKE_PAIRS` entry, the control-class fixtures this tree
    /// already carries, and two above-the-cap inputs so the cap's placement is
    /// inside the comparison rather than beside it.
    ///
    /// Hostile characters are drawn BY IMPORT from `LOOK_ALIKE_PAIRS` and never
    /// respelled (D-21-6).
    fn composition_fixtures() -> Vec<String> {
        let mut out: Vec<String> = Vec::new();
        for (clean, hostile) in crate::test_support::LOOK_ALIKE_PAIRS {
            out.push(clean.to_string());
            out.push(hostile.to_string());
        }
        // The CONTROL class, spelled as the tests in `super::super` spell it:
        // ESC-introduced CSI, the three C1 introducers, NUL, TAB and DEL.
        for control in [
            "\u{1b}[31mred\u{1b}[0m",
            "\u{9b}31m",
            "\u{9d}0;title\u{9c}",
            "\u{90}payload\u{9c}",
            "a\u{0}b",
            "tab\there",
            "del\u{7f}gone",
        ] {
            out.push(control.to_string());
        }
        // Clean prose, so the pin also covers the case prohibition 1 protects.
        out.push("an ordinary line of agent prose".to_string());
        // Above the cap, with and without a hostile character past it — the two
        // inputs where "cap then escape" and "escape then cap" could diverge.
        let over = super::super::DRIVER_OUTPUT_LINE_CELLS + 50;
        out.push("x".repeat(over));
        out.push(format!("{}\u{e0041}", "y".repeat(over)));
        out.push(format!("\u{e0041}{}", "z".repeat(over)));
        out
    }

    /// **The retype is behaviour-preserving, and that is PINNED in both
    /// directions rather than assumed** (21-28 T1, D-21-38).
    ///
    /// `DriverOutputLine::text` became `crate::text::Untrusted`, which moved the
    /// invisible-class escape from the render CALL (`shown_capped`) to the
    /// carrier's `shown()`. Those are two different orderings of the same two
    /// operations plus a cap, and nothing would have gone red if they disagreed:
    /// the pane would simply have started showing something else.
    ///
    /// So this asserts the equality directly, over every fixture in
    /// [`composition_fixtures`] — both classes, above and below the cap.
    ///
    /// **The non-vacuity arm is why this is not two identity functions agreeing
    /// forever.** An equality between two functions that both return their input
    /// passes on every fixture ever added. The second assertion names a specific
    /// fixture the composition CHANGES and pins what it changes into, so a
    /// future refactor that made both sides the identity is red here rather than
    /// silently green.
    ///
    /// If the two ever diverge this reports the raw input, the carrier's answer
    /// and `shown_capped`'s answer side by side — which of the two moved is then
    /// a diff away, and the pane's visible behaviour is the thing that changed.
    #[test]
    fn the_wrapped_line_composition_equals_shown_capped() {
        for raw in composition_fixtures() {
            let via_carrier =
                crate::text::Untrusted::from_untrusted_source(super::super::sanitize_render_line(
                    &raw,
                ))
                .shown()
                .to_string();
            let via_call = shown_capped(&raw);
            assert_eq!(
                via_carrier, via_call,
                "the buffered line's carrier and the render-site call must agree \
                 on every input, or the retype silently changed what the output \
                 pane shows. input={raw:?}"
            );
        }

        // NON-VACUITY. `LOOK_ALIKE_PAIRS[4].1` is `demo` followed by the tag
        // character `U+E0041` — the carrier this phase is named for, and the one
        // the probe caught reaching a cell in this very pane.
        let (_, tag_bearing) = crate::test_support::LOOK_ALIKE_PAIRS[4];
        assert_ne!(
            shown_capped(tag_bearing),
            tag_bearing,
            "the composition must CHANGE at least one fixture, or the equality \
             above is two identity functions agreeing and would pass forever"
        );
        assert_eq!(
            shown_capped(tag_bearing),
            "demoU+E0041",
            "and what it changes it into is the escaped spelling, not merely \
             something different"
        );
    }

    /// **The ring's accounting is unmoved by the retype** (21-28 T1 step (f)).
    ///
    /// The retype touched the type of what `push_record` stores, and it must not
    /// have touched what the buffer COUNTS. `dropped` is the operator's only
    /// evidence that earlier output existed at all, and `record_truncated` is
    /// the only evidence a single record was cut — a retype that quietly moved
    /// either would remove the admission while leaving the pane looking right.
    ///
    /// The two numbers below are the pre-retype values, computed the same way
    /// the pre-retype code computed them: the ring holds
    /// `DRIVER_OUTPUT_RING_LINES` and drops the overshoot, and a record with
    /// more lines than `DRIVER_OUTPUT_RECORD_MAX_LINES` is cut and says so.
    #[test]
    fn the_retype_left_the_rings_accounting_where_it_was() {
        use super::super::{
            DriverLineKind, DriverOutput, DRIVER_OUTPUT_RECORD_MAX_LINES, DRIVER_OUTPUT_RING_LINES,
        };

        let overshoot = 37usize;
        let mut buf = DriverOutput::default();
        for i in 0..(DRIVER_OUTPUT_RING_LINES + overshoot) {
            buf.push_record(DriverLineKind::Output, &format!("line {i}"));
        }
        assert_eq!(
            buf.len(),
            DRIVER_OUTPUT_RING_LINES,
            "the ring is still bounded at its cap after the retype"
        );
        assert_eq!(
            buf.dropped(),
            overshoot as u64,
            "dropped must still count every evicted line: the count is the whole \
             mechanism the render layer has for admitting the loss"
        );
        assert!(
            !buf.record_truncated(),
            "no single record here exceeded the per-record cap"
        );

        let mut cut = DriverOutput::default();
        let long: String = (0..(DRIVER_OUTPUT_RECORD_MAX_LINES + 10))
            .map(|i| format!("row {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        cut.push_record(DriverLineKind::Output, &long);
        assert!(
            cut.record_truncated(),
            "record_truncated must still fire when one record is cut at the \
             per-record cap"
        );
        assert_eq!(
            cut.len(),
            DRIVER_OUTPUT_RECORD_MAX_LINES,
            "and the cut must still be at exactly the per-record cap"
        );
    }

    #[test]
    fn a_run_list_row_is_eighteen_cells_before_the_state_word() {
        let row = run_list_row(&summary("2026-07-29T21-40-00Z-3f2a", None), None, 0);
        assert_eq!(
            text(&row).chars().count(),
            RUN_LIST_FIXED_CELLS,
            "the fixed part of a run row must be one form at every width: {:?}",
            text(&row)
        );
    }

    #[test]
    fn the_state_word_appears_at_twenty_six_cells_and_not_at_twenty_five() {
        let run = summary("2026-07-29T21-40-00Z-3f2a", Some("succeeded_with_changes"));
        let narrow = run_list_row(&run, None, RUN_LIST_WORD_MIN_CELLS - 1);
        let wide = run_list_row(&run, None, RUN_LIST_WORD_MIN_CELLS);
        assert!(!text(&narrow).contains("ok"), "{:?}", text(&narrow));
        assert!(text(&wide).ends_with("  ok"), "{:?}", text(&wide));
    }

    #[test]
    fn elapsed_formats_under_and_over_an_hour_and_switches_form_when_ended() {
        let start = "2026-07-29T21:40:00Z";
        let now = parse_rfc3339("2026-07-29T21:44:12Z").expect("fixture parses");
        assert_eq!(elapsed_label(start, now, None), "+04:12");

        let later = parse_rfc3339("2026-07-29T23:05:07Z").expect("fixture parses");
        assert_eq!(elapsed_label(start, later, None), "+1:25:07");

        // A finished run reports what it took, not what it is taking.
        assert_eq!(
            elapsed_label(start, later, Some("2026-07-29T21:52:41Z")),
            "ran 12:41"
        );

        // A duration the code cannot compute is not a duration of nothing.
        assert_eq!(elapsed_label("not a timestamp", now, None), "unknown");
    }

    #[test]
    fn the_empty_run_list_copy_differs_by_opt_in_state() {
        let opted_in: Vec<String> = no_runs_lines("meta-mgr", true).iter().map(text).collect();
        let not_opted_in: Vec<String> =
            no_runs_lines("meta-mgr", false).iter().map(text).collect();

        assert!(opted_in[0].contains("No runs yet for \"meta-mgr\"."));
        assert!(opted_in[1].contains("[s]"));
        assert!(not_opted_in[0].contains("is not opted in to driving"));
        assert!(not_opted_in[1].contains("[o]"));
        assert!(
            !not_opted_in[1].contains("dashboard"),
            "the line must not send the reader to another surface to press a \
             key that works right here: an empty state whose only instruction \
             is to go somewhere else is the dead end this change removes"
        );
        assert_ne!(opted_in, not_opted_in);
    }

    /// T-18-49: an escape sequence in a goal must not reach a `Span`.
    #[test]
    fn a_goal_bearing_an_escape_sequence_is_sanitised_before_it_becomes_a_line() {
        let hostile = "\u{1b}[2Jdeploy \u{1b}]0;pwned\u{7}now";
        let rendered: String = goal_lines(hostile, 80).iter().map(text).collect();
        assert!(
            !rendered.contains('\u{1b}'),
            "an ESC survived into the header: {rendered:?}"
        );
        // The prose itself is still shown — the rule must not pass by deleting
        // everything.
        assert!(rendered.contains("deploy"), "{rendered:?}");
        assert!(rendered.contains("now"), "{rendered:?}");
    }

    #[test]
    fn an_absent_goal_renders_the_pinned_copy_and_nothing_else() {
        // The shared blank-shape set rather than a hand-picked list, so the
        // zero-width shapes `str::trim` cannot see are covered here too: this
        // surface renders values re-read from run records, which this build's
        // parse boundary never saw, and a goal of one `U+200B` used to render as
        // a blank line wearing a goal's label.
        for empty in crate::test_support::DEGENERATE
            .iter()
            .chain(["\n\t "].iter())
            .copied()
        {
            let rendered: String = goal_lines(empty, 80).iter().map(text).collect();
            assert!(
                rendered.contains(GOAL_NONE_GIVEN),
                "an absent goal must say so; {empty:?} gave: {rendered:?}"
            );
        }
        // A goal that WAS given is never replaced by the placeholder.
        let given: String = goal_lines("ship the driver tab", 80).iter().map(text).collect();
        assert!(!given.contains(GOAL_NONE_GIVEN), "{given:?}");
        assert!(given.contains("ship the driver tab"), "{given:?}");
    }

    #[test]
    fn a_long_goal_wraps_to_at_most_three_rows_then_ellipses() {
        let long = "ship ".repeat(200);
        let lines = goal_lines(&long, 60);
        assert_eq!(lines.len(), 3);
        assert!(text(&lines[2]).ends_with('\u{2026}'), "{:?}", text(&lines[2]));
    }

    /// Below the two-pane floor the run list is not rendered, and the selection
    /// is still honoured — it moves into the pane title rather than vanishing.
    #[test]
    fn the_sub_sixty_tier_returns_a_single_pane_and_still_honours_the_selection() {
        let narrow = Rect::new(0, 0, DRIVER_TWO_PANE_MIN_CELLS - 1, 24);
        let (list, detail) = driver_panes(narrow);
        assert!(list.is_none(), "the run list must be dropped below 60 columns");
        assert_eq!(detail, narrow);

        let wide = Rect::new(0, 0, DRIVER_TWO_PANE_MIN_CELLS, 24);
        let (list, detail) = driver_panes(wide);
        assert!(list.is_some(), "two panes at exactly 60 columns");
        assert!(
            detail.width >= DRIVER_DETAIL_MIN_CELLS,
            "the detail pane fell below its floor at 60 columns: {}",
            detail.width
        );

        // The selection still identifies a run at the narrow tier.
        let title = run_list_title(&summary("2026-07-29T21-40-00Z-3f2a", None), 1, 7);
        assert!(title.contains("3f2a"), "{title:?}");
        assert!(title.contains("(2/7)"), "{title:?}");
    }

    /// D-13: every state word, glyph and colour comes from evidence, and the two
    /// "nothing is known" answers are never collapsed into "it died".
    #[test]
    fn every_run_state_maps_from_evidence_and_unknown_is_not_death() {
        assert_eq!(
            run_state_glyph(Some(RunVerdict::Live), None),
            (GLYPH_LIVE, "live", Color::Magenta)
        );
        assert_eq!(
            run_state_glyph(Some(RunVerdict::LivenessUnknown), None),
            (GLYPH_LIVENESS_UNKNOWN, "?", Color::Yellow)
        );
        assert_eq!(
            run_state_glyph(Some(RunVerdict::CrashedWithoutEnding), None),
            (GLYPH_FAILED, "crash", Color::Red)
        );
        for (label, expected) in [
            ("succeeded_with_changes", (GLYPH_SUCCEEDED, "ok", Color::Green)),
            (
                "succeeded_no_changes",
                (GLYPH_SUCCEEDED_NO_CHANGES, "no-chg", Color::Yellow),
            ),
            ("failed", (GLYPH_FAILED, "fail", Color::Red)),
            ("permission_denied", (GLYPH_FAILED, "denied", Color::Red)),
            ("timed_out", (GLYPH_FAILED, "t/out", Color::Red)),
            ("stalled", (GLYPH_FAILED, "stall", Color::Red)),
            ("capability_refused", (GLYPH_FAILED, "refuse", Color::Red)),
            ("spawn_failed", (GLYPH_FAILED, "spawn", Color::Red)),
            ("killed", (GLYPH_KILLED, "killed", Color::DarkGray)),
        ] {
            assert_eq!(run_state_glyph(None, Some(label)), expected, "{label}");
        }
        // No terminal record and no observation: nothing is known, and that is
        // not a crash report (CR-05).
        assert_eq!(
            run_state_glyph(None, None),
            (GLYPH_LIVENESS_UNKNOWN, "?", Color::Yellow)
        );
        // `SucceededNoChanges` is its own state, not a flavour of success.
        assert_ne!(
            run_state_glyph(None, Some("succeeded_no_changes")),
            run_state_glyph(None, Some("succeeded_with_changes"))
        );
    }

    /// The cost figure is labelled cumulative, and an unknown cost says so
    /// rather than rendering a zero the mechanism cannot back.
    #[test]
    fn the_cost_line_is_labelled_cumulative_and_never_fabricates_a_figure() {
        let now = parse_rfc3339("2026-07-29T21:44:12Z").expect("fixture parses");
        let run = summary("2026-07-29T21-40-00Z-3f2a", None);

        let known: String = render_run_header(&run, Some(1.83), None, 80, now)
            .iter()
            .map(text)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(known.contains("$1.83"), "{known}");
        assert!(known.contains(COST_CUMULATIVE), "{known}");

        let unknown: String = render_run_header(&run, None, None, 80, now)
            .iter()
            .map(text)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(unknown.contains(COST_NOT_REPORTED), "{unknown}");
        assert!(!unknown.contains("$0.00"), "{unknown}");
    }

    /// T-18-50: the run directory row is left-truncated, keeping the run id —
    /// the part a reader cannot reconstruct — rather than the leading path.
    #[test]
    fn the_run_directory_row_keeps_its_tail_when_it_does_not_fit() {
        let path = "/home/blk/projects/rust/gsd-meta-manager/.planning/meta-manager/runs/2026-07-29T21-40-00Z-3f2a";
        let shown = left_truncate(path, 40);
        assert_eq!(shown.chars().count(), 40);
        assert!(shown.starts_with('\u{2026}'), "{shown:?}");
        assert!(shown.ends_with("3f2a"), "{shown:?}");
    }

    // ── The live output pane (OBS-04, D-19) ────────────────────────────────

    /// A fixed `now`, so an elapsed counter is a value rather than a race.
    fn fixed_now() -> DateTime<Utc> {
        parse_rfc3339("2026-07-29T21:40:49Z").expect("fixture parses")
    }

    /// The body of a run this session spawned, with nothing injected — the
    /// shape every pre-injection pane test asserts against.
    fn body(output: Option<&DriverOutput>) -> Vec<Line<'static>> {
        output_body_lines(output, &[], fixed_now(), false, Color::Green)
    }

    #[test]
    fn an_empty_journal_renders_the_no_entries_copy() {
        let rendered: String = body(None).iter().map(text).collect();
        assert!(rendered.contains(NO_JOURNAL_ENTRIES), "{rendered:?}");

        // A buffer that exists but holds nothing is the same situation.
        let empty = DriverOutput::default();
        let rendered: String = body(Some(&empty)).iter().map(text).collect();
        assert!(rendered.contains(NO_JOURNAL_ENTRIES), "{rendered:?}");
    }

    #[test]
    fn a_buffer_that_dropped_lines_states_the_count_in_its_first_row() {
        let mut output = DriverOutput::default();
        for n in 0..(DRIVER_OUTPUT_RING_LINES + 5) {
            output.push_record(DriverLineKind::Output, &format!("line {n}"));
        }
        assert_eq!(output.dropped(), 5, "the ring dropped what it was asked to");

        let lines = body(Some(&output));
        let first = text(&lines[0]);
        assert!(
            first.contains("5 earlier lines dropped"),
            "the drop count must be the FIRST row: {first:?}"
        );
        assert!(
            first.contains(&DRIVER_OUTPUT_RING_LINES.to_string()),
            "the notice names the capacity so the count is readable: {first:?}"
        );
    }

    #[test]
    fn a_diagnostic_record_renders_as_a_diagnostic_row() {
        let mut output = DriverOutput::default();
        output.push_record(DriverLineKind::Output, "ordinary output");
        output.push_record(DriverLineKind::Stderr, "a warning on stderr");
        output.push_record(DriverLineKind::Diagnostic, "journal gap: 3 record(s) not read");
        output.push_record(DriverLineKind::Terminal, "run ended: succeeded_with_changes");

        let lines = body(Some(&output));
        let rendered: Vec<String> = lines.iter().map(text).collect();

        let diagnostic = lines
            .iter()
            .find(|line| text(line).contains("journal gap"))
            .unwrap_or_else(|| panic!("the diagnostic was swallowed: {rendered:#?}"));
        assert_eq!(diagnostic.spans[0].content.as_ref(), MARKER_DIAGNOSTIC);
        assert_eq!(diagnostic.spans[0].style.fg, Some(Color::Yellow));
        assert_eq!(diagnostic.spans[1].style.fg, Some(Color::Yellow));

        // The four visual classes are actually distinct, and the terminal record
        // is the visual full stop.
        assert_eq!(lines[0].spans[0].content.as_ref(), MARKER_OUTPUT);
        assert_eq!(lines[1].spans[0].content.as_ref(), MARKER_STDERR);
        let last = lines.last().expect("a terminal row");
        assert_eq!(last.spans[0].content.as_ref(), MARKER_TERMINAL);
        assert!(text(last).contains("run ended"), "{:?}", text(last));
        assert_eq!(last.spans[0].style.fg, Some(Color::Green));
    }

    #[test]
    fn a_truncated_record_says_so_rather_than_losing_the_lines_silently() {
        let mut output = DriverOutput::default();
        let giant = "row\n".repeat(super::super::DRIVER_OUTPUT_RECORD_MAX_LINES + 10);
        output.push_record(DriverLineKind::Output, &giant);
        assert!(output.record_truncated());

        let rendered: String = body(Some(&output))
            .iter()
            .map(text)
            .collect::<Vec<_>>()
            .join("\n");
        assert!(rendered.contains("record truncated"), "{rendered}");
    }

    // ── The ring belongs to ONE run (CR-02, WR-07) ─────────────────────────

    /// A ring holding `lines` rows already attributed to `run_id`.
    fn ring_for(run_id: &str, lines: &[&str]) -> DriverOutput {
        let mut output = DriverOutput::for_run(run_id);
        for line in lines {
            output.push_record(DriverLineKind::Output, line);
        }
        output
    }

    /// CR-02, at the buffer.
    ///
    /// The map is keyed by alias, so the ring outlives every run in a project.
    /// Retargeting must empty it — **including both overflow counters**, because
    /// `dropped` and `record_truncated` describe this buffer's own shortfall and
    /// carrying run A's into run B reports one run's loss as another's.
    #[test]
    fn retargeting_the_ring_at_a_new_run_drops_the_previous_runs_lines_and_counters() {
        let mut output = ring_for("run-a", &[]);
        for n in 0..(DRIVER_OUTPUT_RING_LINES + 5) {
            output.push_record(DriverLineKind::Output, &format!("line {n}"));
        }
        output.push_record(
            DriverLineKind::Output,
            &"row\n".repeat(super::super::DRIVER_OUTPUT_RECORD_MAX_LINES + 10),
        );
        output.push_record(DriverLineKind::Terminal, "run ended: succeeded_with_changes");
        assert!(output.dropped() > 0 && output.record_truncated() && !output.is_empty());

        output.retarget("run-b");

        assert_eq!(output.run_id(), "run-b");
        assert!(
            output.is_empty(),
            "run A's lines — its terminal record above all, which renders LAST \
             and in run B's colour — must not survive into run B's pane"
        );
        assert_eq!(output.dropped(), 0, "run B has dropped nothing");
        assert!(!output.record_truncated(), "and truncated nothing");

        // Retargeting at the SAME run is not a reset: the tail arrives in
        // batches and each batch retargets before its first push.
        output.push_record(DriverLineKind::Output, "run B says something");
        output.retarget("run-b");
        assert_eq!(output.len(), 1, "an unchanged run id must not clear the ring");
    }

    /// CR-02, at the chooser.
    ///
    /// The ring is matched on its **own** run id and never on `observed_runs`:
    /// the reconciliation map answers "which run is this session tailing", which
    /// stops being the same question the moment a run ends.
    #[test]
    fn a_ring_filled_by_one_run_is_never_handed_to_another() {
        let dir = tempfile::tempdir().expect("temp dir");
        let alias = super::super::driver_confirm::tests::ALIAS;
        let (mut ctx, _rx) = super::super::driver_confirm::tests::ctx_with_project(dir.path());
        ctx.driver_output.insert(
            alias.to_string(),
            ring_for("run-a", &["run A said this"]),
        );

        assert!(
            output_for_run(&ctx, alias, None, "run-a").is_some(),
            "its own run still gets the ring"
        );
        assert!(
            output_for_run(&ctx, alias, None, "run-b").is_none(),
            "and the next run in the same project does not — with no journal on \
             disk for it yet, the honest answer is nothing at all"
        );
    }

    /// WR-07: a live run whose ring is empty falls back to the journal.
    ///
    /// After a TUI restart the ring for an adopted run is empty until the first
    /// watcher event fires. Short-circuiting on `tailed` painted "No journal
    /// entries yet." directly under the notice saying the journal on disk was
    /// what was being shown — two lines that contradict each other, over a
    /// journal the scan had already read whole.
    #[test]
    fn an_adopted_live_run_with_an_empty_ring_renders_the_journal_on_disk() {
        let dir = tempfile::tempdir().expect("temp dir");
        let alias = super::super::driver_confirm::tests::ALIAS;
        let (mut ctx, _rx) = super::super::driver_confirm::tests::ctx_with_project(dir.path());
        ctx.driver_output
            .insert(alias.to_string(), DriverOutput::for_run("run-a"));

        let cache = ProjectViewCache {
            driver_journal: Some(Box::new(super::super::DriverRunJournal {
                run_id: "run-a".to_string(),
                output: ring_for("run-a", &["what the journal on disk holds"]),
                injections: Vec::new(),
            })),
            ..ProjectViewCache::default()
        };

        let chosen = output_for_run(&ctx, alias, Some(&cache), "run-a")
            .expect("the journal is on disk and must be shown");
        // `as_raw_for_logic_only`: this asks WHICH BUFFER was chosen, not what
        // it renders as, so the raw stored bytes are the right question.
        let rendered: String = chosen
            .lines()
            .map(|line| line.text.as_raw_for_logic_only())
            .collect();
        assert!(
            rendered.contains("what the journal on disk holds"),
            "an empty live ring must fall THROUGH to the journal, not \
             short-circuit to a no-entries placeholder: {rendered:?}"
        );
    }

    /// The four indicator states, each with its own word and colour — and the
    /// scrolled state deliberately does not spend the word `paused`, which
    /// already means "a non-empty HANDOFF is present" everywhere else.
    #[test]
    fn the_follow_indicator_says_which_of_the_four_states_the_pane_is_in() {
        let (word, color) = follow_indicator(true, true, 0, None, false);
        assert_eq!(word, INDICATOR_FOLLOWING);
        assert_eq!(color, Color::Green);

        let (word, color) = follow_indicator(true, false, 42, None, false);
        assert_eq!(word, "[scrolled +42]");
        assert_eq!(color, Color::Yellow);
        assert!(!word.contains("paused"));

        // A wildly scrolled pane stops widening the field.
        let (word, _) = follow_indicator(true, false, 100_000, None, false);
        assert_eq!(word, "[scrolled +999]");

        let (word, color) = follow_indicator(false, false, 0, Some("2026-07-29T21:52:00Z"), false);
        assert!(word.starts_with("[ended "), "{word:?}");
        assert_eq!(color, Color::DarkGray);

        let (word, color) = follow_indicator(false, false, 0, Some("2026-07-29T21:52:00Z"), true);
        assert_eq!(word, INDICATOR_JOURNAL_ONLY);
        assert_eq!(color, Color::DarkGray);
    }

    /// The header rule keeps the indicator at every width the pane can reach:
    /// the dashes are what give way, never the state word.
    #[test]
    fn the_output_header_keeps_its_indicator_at_every_width() {
        for width in [20u16, 39, 60, 80, 120] {
            let rendered = text(&output_header_line(width, INDICATOR_FOLLOWING, Color::Green));
            assert!(
                rendered.contains(INDICATOR_FOLLOWING),
                "the indicator was dropped at {width} columns: {rendered:?}"
            );
            assert!(rendered.starts_with("\u{2500}\u{2500} output"), "{rendered:?}");
        }
    }

    // ── After-the-fact review: the terminal-state table (OBS-05, D-13) ─────

    /// The whole evidence-derived table, asserted as a table.
    ///
    /// The **exhaustiveness** half is not carried by this test — it is carried
    /// by the two wildcard-free matches, which make a new state a compile error.
    /// What this asserts is that no two states collapse into one cell, which a
    /// compiler cannot see.
    #[test]
    fn every_terminal_state_maps_to_its_own_cell_from_evidence_alone() {
        use std::collections::HashSet;

        let all = [
            TerminalState::Live,
            TerminalState::LivenessUnknown,
            TerminalState::CrashedWithoutEnding,
            TerminalState::Succeeded,
            TerminalState::SucceededNoChanges,
            TerminalState::Failed,
            TerminalState::PermissionDenied,
            TerminalState::TimedOut,
            TerminalState::Stalled,
            TerminalState::CapabilityRefused,
            TerminalState::SpawnFailed,
            TerminalState::Killed,
            TerminalState::Unrecorded,
        ];

        let cells: HashSet<(&str, String, Color)> =
            all.into_iter().map(terminal_state_cell).collect();
        assert_eq!(
            cells.len(),
            all.len(),
            "two states collapsed into one cell: {cells:#?}"
        );

        // The run list's short word and this table's long one must at least
        // agree on the glyph and the colour, or one surface says a run failed
        // in red while the other says it succeeded in green.
        for label in [
            "succeeded_with_changes",
            "succeeded_no_changes",
            "failed",
            "permission_denied",
            "timed_out",
            "stalled",
            "capability_refused",
            "spawn_failed",
            "killed",
        ] {
            let (list_glyph, _, list_color) = run_state_glyph(None, Some(label));
            let (glyph, word, color) =
                terminal_state_cell(TerminalState::from_label(Some(label)));
            assert_eq!(glyph, list_glyph, "{label}");
            assert_eq!(color, list_color, "{label}");
            assert!(!word.is_empty(), "{label}");
        }

        // A label this build has never seen is not guessed at.
        assert_eq!(
            TerminalState::from_label(Some("a_label_from_a_later_build")),
            TerminalState::Unrecorded
        );
        assert_eq!(TerminalState::from_label(None), TerminalState::Unrecorded);
    }

    /// The vocabulary written by the driver and the vocabulary read by the
    /// renderer are a **string protocol across a process boundary**. Spelling
    /// the labels out a second time here would produce a test that agrees with
    /// itself while disagreeing with disk, so this maps every `RunOutcome`
    /// through the driver's own labeller.
    #[cfg(unix)]
    #[test]
    fn the_render_vocabulary_is_the_one_the_driver_actually_writes() {
        use crate::executor::RunOutcome;
        use std::time::Duration;

        let outcomes = [
            RunOutcome::SucceededWithChanges {
                turns: Vec::new(),
                total_cost_usd: None,
            },
            RunOutcome::SucceededNoChanges {
                turns: Vec::new(),
                total_cost_usd: None,
            },
            RunOutcome::Failed {
                reason: "the agent exited non-zero".to_string(),
                subtype: None,
                terminal_reason: None,
                exit_code: Some(1),
            },
            RunOutcome::PermissionDenied {
                denials: Vec::new(),
            },
            RunOutcome::Killed { turns: Vec::new() },
            RunOutcome::TimedOut {
                after: Duration::from_secs(60),
            },
            RunOutcome::Stalled {
                idle_for: Duration::from_secs(60),
            },
            RunOutcome::CapabilityRefused {
                missing: vec!["stream-json".to_string()],
            },
            RunOutcome::SpawnFailed {
                reason: "no such file".to_string(),
            },
        ];

        for outcome in &outcomes {
            let label = crate::driver::run::outcome_label(outcome);
            assert_eq!(
                TerminalState::from_label(Some(label)),
                TerminalState::from_outcome(outcome),
                "the label {label:?} does not round-trip to {outcome:?}"
            );
            assert_ne!(
                TerminalState::from_label(Some(label)),
                TerminalState::Unrecorded,
                "the renderer has never been taught the label {label:?}"
            );
        }
    }

    /// CR-05, at the render layer. The tri-state fix must not be re-flattened
    /// here: on a platform where the probe does not apply, collapsing unknown
    /// into dead would print "your run died" about every healthy run, on every
    /// scan, for as long as the run lasted.
    #[test]
    fn liveness_unknown_is_not_rendered_as_dead() {
        let unknown = terminal_state_cell(TerminalState::LivenessUnknown);
        let crashed = terminal_state_cell(TerminalState::CrashedWithoutEnding);
        let killed = terminal_state_cell(TerminalState::Killed);
        let failed = terminal_state_cell(TerminalState::Failed);

        assert_ne!(unknown, crashed);
        assert_ne!(unknown, killed);
        assert_ne!(unknown, failed);
        assert_ne!(
            unknown.2, crashed.2,
            "unknown must not even share the colour of a death report"
        );
        assert!(!unknown.1.contains("crash"), "{:?}", unknown.1);
        assert!(!unknown.1.contains("died"), "{:?}", unknown.1);

        // And the verdict routes there rather than to a terminal outcome, even
        // when a stale label is sitting beside it.
        assert_eq!(
            TerminalState::observed(Some(RunVerdict::LivenessUnknown), Some("failed")),
            TerminalState::LivenessUnknown
        );
    }

    /// TRANS-02's whole point: the envelope said success while nothing moved on
    /// disk, and that disagreement is the thing this tool exists to surface. A
    /// flavour of success would bury it.
    #[test]
    fn succeeded_no_changes_is_its_own_state_and_not_a_flavour_of_success() {
        let ok = terminal_state_cell(TerminalState::Succeeded);
        let no_changes = terminal_state_cell(TerminalState::SucceededNoChanges);

        assert_ne!(ok.0, no_changes.0, "a different glyph");
        assert_ne!(ok.1, no_changes.1, "a different word");
        assert_ne!(ok.2, no_changes.2, "a different colour");
        assert!(
            no_changes.1.contains("no changes"),
            "the word must say what did not happen: {:?}",
            no_changes.1
        );
    }

    /// Phase 17 D-11: once the TUI exits the child's stdout pipe is gone, so a
    /// run this session did not spawn can only be shown from disk — and the
    /// pane says so in its first row rather than leaving the reader to infer it.
    #[test]
    fn the_adopted_notice_renders_only_for_a_run_this_session_did_not_spawn() {
        let mut output = DriverOutput::default();
        output.push_record(DriverLineKind::Output, "ordinary output");

        let adopted =
            output_body_lines(Some(&output), &[], fixed_now(), true, Color::Green);
        assert!(
            text(&adopted[0]).contains(ADOPTED_RUN_NOTICE),
            "the notice must be the FIRST row: {:?}",
            text(&adopted[0])
        );
        assert!(
            text(&adopted[0]).contains("gone"),
            "and it must say the live output is gone, never promise it: {:?}",
            text(&adopted[0])
        );

        let spawned_here = body(Some(&output));
        let rendered: String = spawned_here.iter().map(text).collect();
        assert!(
            !rendered.contains(ADOPTED_RUN_NOTICE),
            "a run this session spawned must not carry the notice: {rendered:?}"
        );

        // The indicator agrees with the notice rather than contradicting it.
        let (indicator, _) = follow_indicator(false, false, 0, None, true);
        assert_eq!(indicator, INDICATOR_JOURNAL_ONLY);
    }

    // ── The four-state injection display (STEER-02, D-07, D-10) ────────────

    fn message(id: &str, text: &str) -> InboxMessage {
        InboxMessage {
            id: id.to_string(),
            ts: "2026-07-29T21:40:02Z".to_string(),
            text: text.to_string(),
        }
    }

    /// One journal record, built the way the reader hands it over: a `kind` and
    /// a flattened payload map.
    fn record(kind: &str, payload: serde_json::Value) -> JournalRecord {
        let map = match payload {
            serde_json::Value::Object(map) => map,
            other => panic!("a record payload must be an object, got {other:?}"),
        };
        JournalRecord {
            ts: "2026-07-29T21:40:02Z".to_string(),
            seq: 1,
            kind: kind.to_string(),
            rest: map,
        }
    }

    fn interjected(id: &str, delivered: bool) -> JournalRecord {
        record(
            "interjected",
            serde_json::json!({ "id": id, "text": "skip the UI review", "delivered": delivered }),
        )
    }

    fn acted_on(id: &str) -> JournalRecord {
        record("interjection_acted_on", serde_json::json!({ "id": id }))
    }

    /// The after-close terminal record, carrying the **pinned** reason string.
    ///
    /// Deliberately the constant and not a paraphrase: the render layer selects
    /// its gloss by exact match, so a hand-written reason here would assert the
    /// unrecognised branch while claiming to assert the after-close one.
    fn missed(id: &str) -> JournalRecord {
        missed_with(id, crate::journal::MISSED_AFTER_CLOSE)
    }

    /// The same, for a named `reason`.
    fn missed_with(id: &str, reason: &str) -> JournalRecord {
        record(
            "interjection_missed",
            serde_json::json!({ "id": id, "reason": reason }),
        )
    }

    fn state_of(inbox: &[InboxMessage], records: &[JournalRecord]) -> Vec<InjectionState> {
        derive_injection_states(inbox, records)
            .into_iter()
            .map(|(_, state, _)| state)
            .collect()
    }

    /// The whole table, in one place: each state derives from its own record
    /// set and from nothing else.
    #[test]
    fn each_of_the_four_states_derives_from_the_record_set_that_supports_it() {
        let inbox = vec![message("aaa", "skip the UI review")];

        assert_eq!(state_of(&inbox, &[]), vec![InjectionState::Queued]);
        assert_eq!(
            state_of(&inbox, &[interjected("aaa", true)]),
            vec![InjectionState::Delivered]
        );
        assert_eq!(
            state_of(&inbox, &[interjected("aaa", true), acted_on("aaa")]),
            vec![InjectionState::ActedOn]
        );
        assert_eq!(
            state_of(&inbox, &[missed("aaa")]),
            vec![InjectionState::Missed(MissedReason::AfterClose)]
        );

        // A record about a DIFFERENT message moves nothing: the correlation is
        // by id and never by proximity.
        assert_eq!(
            state_of(&inbox, &[interjected("bbb", true), acted_on("bbb")]),
            vec![InjectionState::Queued]
        );
    }

    /// CR-03: `interjected { delivered: false }` must never leave a message in
    /// `queued`, and must never be promoted to `delivered` either.
    ///
    /// Both errors are the display disagreeing with the disk; they differ only
    /// in direction. `delivered: false` is the journal saying the write FAILED,
    /// which is positive evidence and not an absence of it — the driver read the
    /// message, the write failed, and the inbox cursor has already moved past it
    /// so nothing will ever read it again. Rendering `○ queued` — glossed
    /// *"durably on disk; nothing has read it yet"* — is false twice over and is
    /// PITFALLS' undelivered-injection failure in the code written to prevent it.
    #[test]
    fn a_failed_stdin_write_is_missed_and_never_queued_or_delivered() {
        let inbox = vec![message("aaa", "skip the UI review")];

        let states = state_of(&inbox, &[interjected("aaa", false)]);
        assert_eq!(
            states,
            vec![InjectionState::Missed(MissedReason::SendFailed)],
            "a failed stdin write is terminal, and its reason is the write"
        );
        assert_ne!(
            states[0],
            InjectionState::Queued,
            "`queued` says nothing has read it; the driver HAS read it"
        );
        assert_ne!(
            states[0],
            InjectionState::Delivered,
            "and `delivered` says the write returned Ok, which it did not"
        );

        // The driver writes a matching terminal record for the same id. The two
        // witnesses must agree rather than fight over the rank.
        assert_eq!(
            state_of(
                &inbox,
                &[
                    interjected("aaa", false),
                    missed_with("aaa", crate::journal::MISSED_SEND_FAILED),
                ]
            ),
            vec![InjectionState::Missed(MissedReason::SendFailed)],
        );

        // The gloss names the write, not the close: a user told "the run closed
        // its input" about a run that is still live and still listening would
        // draw exactly the wrong conclusion.
        let rendered: String = injection_rows(
            &derive_injection_states(&inbox, &[interjected("aaa", false)])[0],
            fixed_now(),
        )
        .iter()
        .map(text)
        .collect::<Vec<_>>()
        .join("\n");
        assert!(rendered.contains(MISSED_GLOSS_SEND_FAILED), "{rendered}");
        assert!(!rendered.contains(MISSED_GLOSS), "{rendered}");
        assert!(!rendered.contains(LABEL_QUEUED), "{rendered}");
    }

    /// A reason string this build has no gloss for is answered honestly rather
    /// than by borrowing one of the two it knows.
    #[test]
    fn an_unrecognised_missed_reason_gets_its_own_gloss_and_still_reads_missed() {
        let inbox = vec![message("aaa", "skip the UI review")];
        let records = [missed_with("aaa", "some future reason")];

        assert_eq!(
            state_of(&inbox, &records),
            vec![InjectionState::Missed(MissedReason::Unrecognised)]
        );

        let rendered: String = injection_rows(
            &derive_injection_states(&inbox, &records)[0],
            fixed_now(),
        )
        .iter()
        .map(text)
        .collect::<Vec<_>>()
        .join("\n");
        assert!(rendered.contains(LABEL_MISSED), "{rendered}");
        assert!(rendered.contains(MISSED_GLOSS_UNRECOGNISED), "{rendered}");
        assert!(
            !rendered.contains(MISSED_GLOSS) && !rendered.contains(MISSED_GLOSS_SEND_FAILED),
            "an unknown reason must not be attributed to a known cause: {rendered}"
        );
    }

    /// D-07's whole point: delivered and acted-on are **two** states set by two
    /// different records, not one state with two labels. The tell that they were
    /// collapsed is that nothing reads the second record.
    #[test]
    fn a_message_with_both_records_derives_acted_on_rather_than_delivered() {
        let inbox = vec![message("aaa", "skip the UI review")];
        let both = [interjected("aaa", true), acted_on("aaa")];
        assert_eq!(state_of(&inbox, &both), vec![InjectionState::ActedOn]);

        // And the order the records arrive in does not decide it — the later
        // state wins on its rank, not on its position.
        let reversed = [acted_on("aaa"), interjected("aaa", true)];
        assert_eq!(state_of(&inbox, &reversed), vec![InjectionState::ActedOn]);
    }

    #[test]
    fn a_message_present_only_in_the_inbox_derives_queued() {
        let inbox = vec![message("aaa", "skip the UI review")];
        // A journal full of other kinds must not disturb it.
        let noise = [
            record("exec_event", serde_json::json!({ "stream": "assistant", "text": "hi" })),
            record("cost", serde_json::json!({ "cumulative_usd": 1.83 })),
        ];
        assert_eq!(state_of(&inbox, &noise), vec![InjectionState::Queued]);
    }

    /// The honest fourth state (D-10). Leaving it in `queued` forever is the
    /// undelivered-injection failure dressed up as a spinner.
    #[test]
    fn a_message_present_only_in_interjection_missed_derives_missed() {
        let inbox = vec![message("aaa", "skip the UI review")];
        assert_eq!(
            state_of(&inbox, &[missed("aaa")]),
            vec![InjectionState::Missed(MissedReason::AfterClose)]
        );

        let rendered: String = injection_rows(
            &derive_injection_states(&inbox, &[missed("aaa")])[0],
            fixed_now(),
        )
        .iter()
        .map(text)
        .collect::<Vec<_>>()
        .join("\n");
        assert!(rendered.contains(MISSED_GLOSS), "{rendered}");
    }

    /// D-05: text alone is not a correlation key. A user may legitimately send
    /// the same sentence twice, and STEER-02's states are per message.
    #[test]
    fn two_messages_with_identical_text_but_different_ids_derive_independently() {
        let inbox = vec![
            message("aaa", "skip the UI review"),
            message("bbb", "skip the UI review"),
        ];
        assert_eq!(
            state_of(&inbox, &[interjected("aaa", true), acted_on("aaa")]),
            vec![InjectionState::ActedOn, InjectionState::Queued],
            "the second message must not inherit the first's state"
        );
    }

    /// Always two rows, three when missed — **at every width**, which this
    /// function guarantees structurally by taking no width at all. The strongest
    /// available statement of it is that the count does not move for a message
    /// text of any length.
    #[test]
    fn the_rendered_rows_are_exactly_two_and_three_when_missed_at_every_width() {
        for length in [0usize, 1, 40, 200, 4_000] {
            let text_of_length = "x".repeat(length);
            let inbox = vec![message("aaa", &text_of_length)];

            for (records, expected) in [
                (Vec::new(), 2usize),
                (vec![interjected("aaa", true)], 2),
                (vec![interjected("aaa", true), acted_on("aaa")], 2),
                (vec![missed("aaa")], 3),
            ] {
                let entries = derive_injection_states(&inbox, &records);
                let rows = injection_rows(&entries[0], fixed_now());
                assert_eq!(
                    rows.len(),
                    expected,
                    "a {length}-character message in {:?} rendered {} rows",
                    entries[0].1,
                    rows.len()
                );
            }
        }

        // And a run with NO injected messages renders no injection rows at all
        // — not an empty queued placeholder.
        assert!(injection_block(&[], fixed_now()).is_empty());
    }

    /// The elapsed counter is the honest answer to the fifty-five-second gap: it
    /// shows time passing **without predicting an arrival**, which is why it is
    /// on the delivered row and on no other. A queued message has nothing to
    /// count from, and a terminal state has stopped counting.
    #[test]
    fn only_the_delivered_row_carries_an_elapsed_counter() {
        let inbox = vec![message("aaa", "skip the UI review")];

        let delivered = derive_injection_states(&inbox, &[interjected("aaa", true)]);
        let head = text(&injection_rows(&delivered[0], fixed_now())[0]);
        assert!(
            head.contains("(+0:47)"),
            "the delivered row must count the gap: {head:?}"
        );

        for records in [
            Vec::new(),
            vec![interjected("aaa", true), acted_on("aaa")],
            vec![missed("aaa")],
        ] {
            let entries = derive_injection_states(&inbox, &records);
            let head = text(&injection_rows(&entries[0], fixed_now())[0]);
            assert!(
                !head.contains("(+"),
                "{:?} must not count: {head:?}",
                entries[0].1
            );
        }

        // A transition whose own timestamp did not parse omits the counter
        // rather than fabricating a zero.
        let mut broken = interjected("aaa", true);
        broken.ts = "not a timestamp".to_string();
        let entries = derive_injection_states(&inbox, &[broken]);
        assert_eq!(entries[0].1, InjectionState::Delivered);
        let head = text(&injection_rows(&entries[0], fixed_now())[0]);
        assert!(!head.contains("(+"), "{head:?}");
    }

    /// **The vocabulary is a safety property** (D-07, D-10). The dequeue echo
    /// arrives roughly fifty-five seconds after the stdin write, so every one of
    /// these four words would promise an observation the protocol cannot make.
    ///
    /// The list is held **here, in the test module**, and never in the render
    /// module — the same discipline `tests/spawn_seam_guard.rs` uses by walking
    /// `src/` only. A list the rendering module owned could be softened in the
    /// same edit that softened the copy, and the assertion would go quiet
    /// instead of failing.
    #[test]
    fn no_rendered_injection_string_uses_a_word_that_implies_receipt() {
        const FORBIDDEN: [&str; 4] = ["sent", "received", "read", "acknowledged"];

        // The message text is the user's own words and is echoed verbatim; the
        // rule governs the tool's copy, so the fixture says nothing that would
        // make the assertion about the wrong string.
        let inbox = vec![message("aaa", "skip the UI review")];
        let mut rendered: Vec<String> = Vec::new();
        for records in [
            Vec::new(),
            vec![interjected("aaa", true)],
            vec![interjected("aaa", true), acted_on("aaa")],
            vec![missed("aaa")],
            // Every missed reason, so a gloss added with a new reason cannot
            // slip past the rule.
            vec![interjected("aaa", false)],
            vec![missed_with("aaa", "a reason this build does not know")],
        ] {
            for entry in derive_injection_states(&inbox, &records) {
                rendered.extend(injection_rows(&entry, fixed_now()).iter().map(text));
            }
        }
        // The labels themselves, in case a state ever stops being rendered.
        rendered.extend(
            [
                InjectionState::Queued,
                InjectionState::Delivered,
                InjectionState::ActedOn,
                InjectionState::Missed(MissedReason::AfterClose),
                InjectionState::Missed(MissedReason::SendFailed),
                InjectionState::Missed(MissedReason::Unrecognised),
            ]
            .into_iter()
            .map(|state| state.cell().1.to_string()),
        );
        rendered.extend(
            [
                MissedReason::AfterClose,
                MissedReason::SendFailed,
                MissedReason::Unrecognised,
            ]
            .into_iter()
            .map(|reason| reason.gloss().to_string()),
        );

        for line in &rendered {
            // Tokenised rather than substring-matched, so `already` and
            // `threads` do not count as `read` — a substring test would be so
            // noisy it would have to be weakened, and a weakened test is how the
            // real word gets back in.
            for token in line
                .to_lowercase()
                .split(|c: char| !c.is_alphanumeric())
                .filter(|token| !token.is_empty())
            {
                assert!(
                    !FORBIDDEN.contains(&token),
                    "an injection string used {token:?}, which implies an \
                     observation the protocol cannot make: {line:?}"
                );
            }
        }

        // The list must actually be able to fail, or the loop above proves
        // nothing about it.
        assert!(FORBIDDEN.contains(&"sent"));
    }

    /// The widget is spliced **into** the pane in place of the raw injection
    /// records, which is what makes the derivation feed the render path rather
    /// than sit beside it as a second, duplicated one.
    #[test]
    fn the_injection_widget_supersedes_the_raw_records_in_the_pane() {
        let inbox = vec![message("aaa", "skip the UI review")];
        let entries = derive_injection_states(&inbox, &[interjected("aaa", true), acted_on("aaa")]);

        let mut output = DriverOutput::default();
        output.push_record(DriverLineKind::Output, "ordinary output");
        output.push_record(DriverLineKind::Injection, "skip the UI review");
        output.push_record(DriverLineKind::Injection, "interjection acted on: aaa");
        output.push_record(DriverLineKind::Terminal, "run ended: succeeded_with_changes");

        let lines = output_body_lines(Some(&output), &entries, fixed_now(), false, Color::Green);
        let rendered: Vec<String> = lines.iter().map(text).collect();
        let joined = rendered.join("\n");

        assert!(
            joined.contains(LABEL_ACTED_ON),
            "the widget must render in the pane: {joined}"
        );
        assert!(
            !joined.contains("interjection acted on: aaa"),
            "the bare correlation id is superseded by the widget: {joined}"
        );
        assert_eq!(
            joined.matches("skip the UI review").count(),
            1,
            "the message text must appear exactly once: {joined}"
        );
        // The ordinary output above it and the terminal full stop below it are
        // untouched.
        assert!(rendered[0].contains("ordinary output"), "{joined}");
        assert!(
            rendered.last().expect("a terminal row").contains("run ended"),
            "{joined}"
        );
    }

    // ── The reused pipeline row and the one-row step timeline (D-17, D-12) ──

    fn mid_pipeline_inference() -> crate::state_reader::disk_status::DiskInference {
        use crate::state_reader::disk_status::{DiskInference, DiskStatus};
        DiskInference {
            status: DiskStatus::Partial,
            plan_count: 3,
            summary_count: 2,
            has_plans: true,
            has_summaries: true,
            has_context: true,
            has_research: true,
            ..DiskInference::default()
        }
    }

    /// D-17: the Driver tab **calls** the D-R-P-E-V widget rather than growing a
    /// second progress display. The strongest available statement of that is
    /// that the two produce the identical line for the same inference — a
    /// reimplementation, however faithful at the moment it was written, would
    /// drift the first time either side changed.
    #[test]
    fn the_driver_tab_pipeline_line_matches_the_pipeline_tabs() {
        let inf = mid_pipeline_inference();

        // What `render_pipeline_tab` builds, at its own call site's shape.
        let statuses = super::super::detail::derive_all_stage_statuses(&inf);
        let pipeline_tab_line = super::super::detail::build_pipeline_line(&inf, &statuses);

        // What the Driver tab renders: the second line of the pipeline row,
        // under its one blank spacer.
        let mut driver_lines: Vec<Line<'static>> = vec![Line::from("")];
        let driver_statuses = super::super::detail::derive_all_stage_statuses(&inf);
        driver_lines.push(super::super::detail::build_pipeline_line(
            &inf,
            &driver_statuses,
        ));

        assert_eq!(text(&driver_lines[1]), text(&pipeline_tab_line));
        assert_eq!(
            driver_lines[1].spans.len(),
            pipeline_tab_line.spans.len(),
            "the same spans, so the same per-stage colours"
        );
        for (a, b) in driver_lines[1]
            .spans
            .iter()
            .zip(pipeline_tab_line.spans.iter())
        {
            assert_eq!(a.style, b.style, "a stage was restyled: {:?}", a.content);
        }
    }

    #[test]
    fn the_steps_section_has_exactly_one_command_row_and_always_carries_the_note() {
        for (verdict, outcome) in [
            (Some(RunVerdict::Live), None),
            (None, Some("succeeded_with_changes")),
            (None, Some("failed")),
            (Some(RunVerdict::CrashedWithoutEnding), None),
        ] {
            let run = summary("2026-07-29T21-40-00Z-3f2a", outcome);
            let lines = steps_lines(&run, verdict, Some(3), 60);
            assert_eq!(
                lines.len(),
                3,
                "the section rule, ONE decided row and the note: {:?}",
                lines.iter().map(text).collect::<Vec<_>>()
            );
            let command_rows = lines
                .iter()
                .filter(|line| text(line).contains("/gsd:execute-phase 18"))
                .count();
            assert_eq!(command_rows, 1, "one decided command row in this phase");
            assert!(
                text(&lines[2]).contains(STEPS_HONEST_NOTE),
                "the honest note is mandatory whenever the section renders: {:?}",
                text(&lines[2])
            );
        }
    }

    /// The turn counter is a live-run affordance: a finished run's turn count is
    /// history that belongs to nothing on this row, and showing it beside a
    /// terminal state word would read as "still going".
    #[test]
    fn the_turn_counter_appears_only_while_the_run_is_live() {
        let run = summary("2026-07-29T21-40-00Z-3f2a", None);
        let live: String = steps_lines(&run, Some(RunVerdict::Live), Some(3), 60)
            .iter()
            .map(text)
            .collect();
        assert!(live.contains("turn 3"), "{live:?}");
        assert!(live.contains("running"), "{live:?}");

        let ended = summary("2026-07-29T21-40-00Z-3f2a", Some("succeeded_with_changes"));
        let finished: String = steps_lines(&ended, Some(RunVerdict::Ended), Some(3), 60)
            .iter()
            .map(text)
            .collect();
        assert!(!finished.contains("turn 3"), "{finished:?}");
        assert!(!finished.contains("running"), "{finished:?}");
        // The **run-detail** word, not the run list's abbreviation: this row is
        // the one place the full evidence-derived word fits (Surface 8).
        assert!(finished.contains("succeeded"), "{finished:?}");
    }

    /// Held-out render-buffer backstop (UI-SPEC `## UI Considerations`, the
    /// D-R-P-E-V overflow row). **A real buffer assertion, not a width
    /// calculation**, because this is the CR-01 / UIFIX-02 clip in a new
    /// location and the original defect was invisible to every calculation the
    /// code had: the trailing `[V]` fell off the right edge of a
    /// percentage-sized pane while every number involved looked right.
    #[test]
    fn the_pipeline_line_keeps_its_verify_stage_at_sixty_eighty_and_one_hundred_twenty_columns() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        // The `pub(crate)` fixture the three driver screens already share,
        // rather than a sixth full-field `AppContext` construction.
        let dir = tempfile::tempdir().expect("temp dir");
        let alias = super::super::driver_confirm::tests::ALIAS;

        for width in [60u16, 80, 120] {
            let (mut ctx, _rx) =
                super::super::driver_confirm::tests::ctx_with_project(dir.path());
            ctx.project_states.insert(
                alias.to_string(),
                crate::state_reader::ProjectState {
                    current_phase_status: Some(mid_pipeline_inference()),
                    ..crate::state_reader::ProjectState::default()
                },
            );
            let cache = ctx.view_cache.entry(alias.to_string()).or_default();
            cache.driver_runs = vec![summary("2026-07-29T21-40-00Z-3f2a", None)];

            let viewport = Cell::default();
            let mut terminal =
                Terminal::new(TestBackend::new(width, 30)).expect("TestBackend terminal");
            terminal
                .draw(|frame| {
                    let area = frame.area();
                    render_driver_tab(
                        frame,
                        area,
                        &ctx,
                        alias,
                        ctx.view_cache.get(alias),
                        &viewport,
                    );
                })
                .expect("draw the driver tab");

            let buffer = terminal.backend().buffer().clone();
            let scraped: Vec<String> = (0..30)
                .map(|y| {
                    (0..width)
                        .map(|x| {
                            buffer
                                .cell((x, y))
                                .map(|cell| cell.symbol())
                                .unwrap_or(" ")
                                .to_string()
                        })
                        .collect::<String>()
                })
                .collect();

            let pipeline_row = scraped
                .iter()
                .find(|row| row.contains("[D]"))
                .unwrap_or_else(|| {
                    panic!("no pipeline row rendered at {width} columns: {scraped:#?}")
                });

            assert!(
                pipeline_row.contains("[E 2/3]"),
                "the execute stage lost its plan fraction at {width} columns: \
                 {pipeline_row:?}"
            );
            assert!(
                pipeline_row.contains("[V]"),
                "the trailing verify stage was clipped at {width} columns — this is \
                 CR-01 / UIFIX-02 in a new place: {pipeline_row:?}"
            );
            // The note is asserted by prefix rather than whole. At exactly 60
            // columns the detail pane is `DRIVER_DETAIL_MIN_CELLS` (39) wide and
            // the note plus its two-cell indent is 40, so its final period is
            // clipped — the sentence stays legible and the floor protects the
            // element it was derived from, which is the pipeline line above.
            let note_prefix = &STEPS_HONEST_NOTE[..30];
            assert!(
                scraped.iter().any(|row| row.contains(note_prefix)),
                "the honest note is missing at {width} columns: {scraped:#?}"
            );
        }
    }

    // ── The dry-run preview (D-26, the designated cut) ─────────────────────

    /// Paint `preview` into a `width` x `height` frame and scrape it back.
    ///
    /// A real buffer rather than an inspection of the `Vec<Line>`, because the
    /// property under test on the sanitiser row is *what reaches the terminal*.
    fn scrape_preview(preview: &DryRunPreview, width: u16, height: u16) -> Vec<String> {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let mut terminal =
            Terminal::new(TestBackend::new(width, height)).expect("TestBackend terminal");
        terminal
            .draw(|frame| {
                let area = frame.area();
                render_dry_run_preview(frame, area, preview);
            })
            .expect("draw the dry-run preview");

        let buffer = terminal.backend().buffer().clone();
        (0..height)
            .map(|y| {
                (0..width)
                    .map(|x| {
                        buffer
                            .cell((x, y))
                            .map(|cell| cell.symbol())
                            .unwrap_or(" ")
                            .to_string()
                    })
                    .collect::<String>()
            })
            .collect()
    }

    fn loaded(report: &str) -> DryRunPreview {
        DryRunPreview {
            command: "/gsd:progress".to_string(),
            report: Some(report.to_string()),
        }
    }

    #[test]
    fn the_preview_pane_renders_the_loading_idiom_before_the_report_resolves() {
        let pending = DryRunPreview {
            command: "/gsd:progress".to_string(),
            report: None,
        };
        // The whole point of the loading state is that it is reachable: the
        // build is scheduled, returns immediately, and the report arrives a
        // couple of `git` invocations later.
        let scraped = scrape_preview(&pending, 60, 12);
        assert!(
            scraped.iter().any(|row| row.contains(DRY_RUN_LOADING)),
            "an unresolved preview must show the shipped loading idiom, not an \
             empty report — an empty preview of a run about to touch the user's \
             repository is the most dangerous thing this pane could imply: \
             {scraped:#?}"
        );
    }

    #[test]
    fn a_loaded_report_renders_its_three_pinned_section_headers() {
        use crate::driver::dry_run::{
            render, DryRunReport, SECTION_COMMANDS, SECTION_DIFFSTAT, SECTION_REFSPECS,
        };
        use crate::state_reader::git_ops::{PushPreview, WorkingTreeStat};

        let report = render(&DryRunReport {
            commands: vec!["/gsd:execute-phase 18".to_string()],
            diffstat: WorkingTreeStat {
                stat_lines: vec![" src/lib.rs | 2 +-".to_string()],
                untracked: Vec::new(),
            },
            push: PushPreview {
                remote: Some("origin".to_string()),
                url: Some("https://example.invalid/demo.git".to_string()),
                refspecs: vec!["refs/heads/main:refs/heads/main".to_string()],
                note: None,
            },
            protection: crate::envelope::advisory::not_probed(),
        });
        // Tall enough that the more-indicator does not eat a header row.
        let scraped = scrape_preview(&loaded(&report), 100, 40).join("\n");

        for section in [SECTION_COMMANDS, SECTION_DIFFSTAT, SECTION_REFSPECS] {
            // The first line of each pinned header. The refspec section is the
            // one PITFALLS:69 names as THE warning sign when it goes missing.
            let headline = section.lines().next().expect("a non-empty header");
            assert!(
                scraped.contains(headline),
                "the preview dropped the pinned section {headline:?}:\n{scraped}"
            );
        }
        assert!(
            scraped.contains("/gsd:execute-phase 18"),
            "the command itself must appear:\n{scraped}"
        );
        assert!(
            scraped.contains("refs/heads/main:refs/heads/main"),
            "the refspec list is the blast radius; a preview without it tells \
             the user nothing:\n{scraped}"
        );
    }

    #[test]
    fn a_report_carrying_an_escape_sequence_is_sanitised_before_rendering() {
        // A branch name or a path is project-controlled, and an OSC sequence in
        // one would otherwise repaint the screen, forge a status line or set the
        // window title (T-18-61).
        let hostile = "== Push refspecs this state would produce ==\n  \
             refs/heads/\u{1b}]0;pwned\u{7}evil:refs/heads/evil\n";
        let scraped = scrape_preview(&loaded(hostile), 80, 10).join("");

        assert!(
            !scraped.contains('\u{1b}'),
            "ESC must be stripped unconditionally — the single highest-value \
             rule in the untrusted-input section: {scraped:?}"
        );
        assert!(
            !scraped.contains('\u{7}'),
            "no C0 control may reach the terminal: {scraped:?}"
        );
        assert!(
            scraped.contains('\u{00B7}'),
            "the stripped control must be replaced by the middle dot rather \
             than silently deleted: {scraped:?}"
        );
        assert!(
            scraped.contains("evil:refs/heads/evil"),
            "sanitising must not eat the content the user needs to read: \
             {scraped:?}"
        );
    }

    #[test]
    fn a_report_taller_than_the_pane_says_so_rather_than_clipping_silently() {
        let long = (0..40)
            .map(|i| format!("line {i}"))
            .collect::<Vec<_>>()
            .join("\n");
        let scraped = scrape_preview(&loaded(&long), 40, 8);
        let last = scraped.last().expect("eight rows were painted");
        assert!(
            last.contains(super::super::help::MORE_INDICATOR),
            "a pane that silently drops the refspec section is the PITFALLS:69 \
             failure this preview exists to prevent: {scraped:#?}"
        );
    }

    #[test]
    fn the_preview_replaces_the_run_detail_and_restores_it_when_it_is_cleared() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let dir = tempfile::tempdir().expect("temp dir");
        let alias = super::super::driver_confirm::tests::ALIAS;
        let (mut ctx, _rx) = super::super::driver_confirm::tests::ctx_with_project(dir.path());
        ctx.view_cache
            .entry(alias.to_string())
            .or_default()
            .driver_runs = vec![summary("2026-07-29T21-40-00Z-3f2a", None)];

        let scrape = |ctx: &AppContext| -> String {
            let viewport = Cell::default();
            let mut terminal =
                Terminal::new(TestBackend::new(100, 30)).expect("TestBackend terminal");
            terminal
                .draw(|frame| {
                    let area = frame.area();
                    render_driver_tab(frame, area, ctx, alias, ctx.view_cache.get(alias), &viewport);
                })
                .expect("draw the driver tab");
            let buffer = terminal.backend().buffer().clone();
            (0..30)
                .map(|y| {
                    (0..100)
                        .map(|x| {
                            buffer
                                .cell((x, y))
                                .map(|cell| cell.symbol())
                                .unwrap_or(" ")
                                .to_string()
                        })
                        .collect::<String>()
                })
                .collect::<Vec<_>>()
                .join("\n")
        };

        let before = scrape(&ctx);
        assert!(
            !before.contains(DRY_RUN_LOADING),
            "with no preview open the pane is the ordinary run detail:\n{before}"
        );

        ctx.view_cache
            .entry(alias.to_string())
            .or_default()
            .driver_dry_run = Some(DryRunPreview {
            command: "/gsd:progress".to_string(),
            report: None,
        });
        let during = scrape(&ctx);
        assert!(
            during.contains(DRY_RUN_LOADING),
            "the preview replaces the run detail while Step B is open:\n{during}"
        );

        // The cut rule, exercised: clear the one field and the body is the
        // normal run detail again, with nothing else changed.
        ctx.view_cache
            .entry(alias.to_string())
            .or_default()
            .driver_dry_run = None;
        assert_eq!(
            scrape(&ctx),
            before,
            "leaving Step B must restore the normal run detail exactly"
        );
    }
}
