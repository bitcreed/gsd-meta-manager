//! The Driver tab's shared pane helpers: the two-pane split, the follow indicator,
//! the scrolling pane's header rule, the styles and the capped identity composition.
//!
//! **Module-layout decision, recorded here because this is where a reader will
//! ask the question** (the register of `src/driver/liveness.rs:35-41`). Every
//! other detail sub-tab renders inline in `detail.rs`; the Driver tab renders from
//! `driver_run.rs` and `detail.rs` delegates. What lives HERE is what the launcher
//! view (`driver_run.rs`), the steering screens and the launch form share.
//!
//! **What this module used to hold, and where it went (Phase 31, D-03).** The
//! legacy run list, the run header, the step timeline, the output pane, the
//! injection widget and the dry-run preview rendered Phase 17/18 journal runs.
//! Nothing in the TUI reads those runs any more, so that renderer is gone. The
//! launcher's own run list, header, items and LOG tail are `driver_run.rs`.
//!
//! **The Replit rule governs everything here (D-13).** Every status word, glyph,
//! colour and sort key on this surface derives from recorded evidence — the run
//! lock, the written status, the heartbeat — and **none of it is ever computed by
//! inspecting the agent's own prose.** The agent's words may appear as content in
//! a pane and may carry no authority anywhere else. The incident behind the rule:
//! an agent deleted a production database during an explicit freeze, hid it,
//! fabricated roughly four thousand fake users and fake test results, and falsely
//! claimed rollback was impossible. A tool that repeats an agent's account of
//! itself as fact makes that class of failure invisible to the human who is
//! accountable for the repository.

use chrono::{DateTime, Local, Utc};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};

use super::{sanitize_render_line, DRIVER_OUTPUT_RING_LINES};

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

// ── Copy (Copywriting Contract, exact strings) ─────────────────────────────

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
pub(super) fn ring_overflow_notice(dropped: u64) -> String {
    format!("\u{2026} {dropped} earlier lines dropped (buffer holds {DRIVER_OUTPUT_RING_LINES})")
}

/// The shipped async-load idiom, reused verbatim (Phase 12, UI-SPEC `## Colour`).
///
/// A second wording for "we are waiting on a blocking read" would be a second
/// thing for the reader to learn, and this surface has no spinner and no
/// animation to distinguish it with.
pub(super) const DRY_RUN_LOADING: &str = "Loading...";

// ── Styles ─────────────────────────────────────────────────────────────────

/// The Label role: a field name, a section rule, a timestamp. `Modifier::DIM` on
/// the terminal's own default foreground, with no fg colour set [inferred — audit]
/// (quick 261021-1id).
///
/// DIM dims whatever foreground the terminal uses, so it adapts to light and dark
/// themes. Bright-black (`Color::DarkGray`) equals the background in
/// Solarized-Dark-family palettes, and ANSI white (`Color::Gray`) nearly vanishes
/// on light themes. A terminal without SGR 2 shows the label at full intensity,
/// which is still legible. The inert-state colour (unlaunched, stopped, pending,
/// unseen, `[not live]`, killed) stays `Color::DarkGray`: a state's colour is one
/// third of its (glyph, word, colour) triple.
pub(super) fn label_style() -> Style {
    Style::default().add_modifier(Modifier::DIM)
}

/// The Muted role: secondary copy. Renders the same as [`label_style`] (DIM on the
/// default foreground, see there); muted rows stay distinguishable by their
/// parenthesised copy.
pub(super) fn muted_style() -> Style {
    Style::default().add_modifier(Modifier::DIM)
}

// ── The terminal-state label vocabulary (OBS-05, D-13) ─────────────────────

/// The outcome states a run record's `outcome` label can name, as a **typed** value.
///
/// **What is left of it (Phase 31, D-03).** The Driver tab no longer renders legacy
/// journal runs, so the glyph/word table and the live-verdict arms that fed it are
/// gone. What stays is the label parser, because `driver::run`'s label-vocabulary
/// test still asserts that every word `spawn_failure_label` writes is one this
/// parser recognises. It goes with that test when `driver::run` is retired
/// (`[inferred — audit]`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalState {
    /// `RunOutcome::SucceededWithChanges`.
    Succeeded,
    /// `RunOutcome::SucceededNoChanges`. **Its own state, not a flavour of
    /// success**: the envelope said success while nothing moved on disk.
    SucceededNoChanges,
    /// `RunOutcome::Failed`.
    Failed,
    /// `RunOutcome::PermissionDenied`.
    PermissionDenied,
    /// `RunOutcome::TimedOut` — the wall-clock cap.
    TimedOut,
    /// `RunOutcome::Stalled` — the idle cap; the stuck detector proper.
    Stalled,
    /// `RunOutcome::CapabilityRefused` — the CLI failed the capability gate.
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
    /// The state an outcome **label** names.
    ///
    /// The label is what `run.json` and `JournalEvent::RunEnded` actually carry,
    /// written by `driver::run::outcome_label` from the derived outcome. A label
    /// this build has never seen is [`Unrecorded`](Self::Unrecorded) rather than
    /// a guess.
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
}

fn parse_rfc3339(raw: &str) -> Option<DateTime<Utc>> {
    DateTime::parse_from_rfc3339(raw)
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
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
/// (21-25 T2). `sanitize_render_line` answers only the CONTROL class
/// (`ESC` / C0 / `DEL` / C1). The invisible-formatting class —
/// `General_Category=Cf` union `Default_Ignorable_Code_Point` — is `Cf`, not
/// `Cc`, so `U+202E`, `U+00AD` and the `U+E0000..U+E007F` tag block passed
/// through untouched into a `Paragraph`. The empty-state copy this module then
/// carried (`no_runs_lines`, replaced by `driver_run`'s empty state in 30-01)
/// already stated that neither class subsumes the other and composed the two.
///
/// It was invisible for as long as it was because `probe_ctx` left
/// `driver_runs` empty, so the Driver tab rendered that empty state — the ONE
/// site that was already composed — on every probe run. The red is quoted in
/// `render_escape_guard::probe_ctx`'s doc.
///
/// # IN-01 — the completeness claim REPLACED by the census that checks it
///
/// **REMOVED 2026-08-27 (21-28).** This doc used to open with the sentence:
///
/// > *"Every site below used `sanitize_render_line` alone, which answers only
/// > the CONTROL class …; what was missing was every OTHER site on this path
/// > doing the same."*
///
/// It read as a statement that the conversion was complete. It was not: three
/// sites in THIS file were never converted — the output pane at `output_line`,
/// the injection rows, and the dry-run preview — plus two in
/// `driver_confirm.rs`. A completeness claim is exactly the sentence that stops
/// a reader re-checking, which is how those five survived a round whose whole
/// subject was this composition.
///
/// **What a reader can CHECK instead**, replacing what they were asked to
/// believe:
/// [`no_executable_control_class_call_in_these_two_files_stands_outside_a_composition`](tests::no_executable_control_class_call_in_these_two_files_stands_outside_a_composition)
/// is a committed census over `driver.rs` and `driver_confirm.rs`. It asserts
/// an EQUALITY on a count of zero: no executable call to `sanitize_render_line`
/// in either file stands outside a composition with `display_identity` or
/// `render_for_terminal`. It was observed RED by planting an unconverted call,
/// and its needle is assembled at runtime from two halves so it cannot report
/// its own source.
///
/// **Its residual, with the direction.** It is a source scan over two NAMED
/// files, so a composition assembled across separate statements, or a third
/// file added to this path tomorrow, is invisible to it. **Under-detection,
/// silent.** What bounds THAT is the render-escape probe — which now renders
/// all four of this path's sites — and not this census.
///
/// The cap is `sanitize_render_line`'s, deliberately: this draws agent prose
/// and a run's own goal, which is exactly what that cap exists for. The
/// uncapped composition is `crate::text::render_for_terminal`, and the two
/// agree below the cap (pinned by
/// `ui::screens::tests::the_capped_and_uncapped_compositions_agree_below_the_cap`).
pub(super) fn shown_capped(value: &str) -> String {
    crate::text::display_identity(&sanitize_render_line(value))
}

/// Split the tab area into the run list and the run detail.
///
/// The detail pane takes `Constraint::Min(DRIVER_DETAIL_MIN_CELLS)` and **never
/// a bare percentage complement of the list's 40%** — the discipline that keeps
/// the reused D-R-P-E-V line from losing its trailing `[V]`. A percentage-only
/// constraint on that pane is the CR-01 / UIFIX-02 defect, verbatim. Below
/// [`DRIVER_TWO_PANE_MIN_CELLS`] the list is dropped entirely and `None` is
/// returned for it; the selection still moves, only its *rendering* is given up.
pub(super) fn driver_panes(area: Rect) -> (Option<Rect>, Rect) {
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

/// The Driver pane's identity title, carrying D3's EXPERIMENTAL marker.
///
/// **Unconditional, and that is correct**: since quick task 260917-fko this
/// whole function is reachable only when `AppContext.experimental` is set, so
/// anyone who can read this title is by construction in an experimental
/// session. A second flag threaded down here would be a bool that can only ever
/// hold one value.
///
/// One constant for both title sites — the empty-runs early return and the
/// two-pane detail block — so a project with no runs yet is marked too. The
/// narrow tier deliberately keeps the selected run's own title (`narrow_title` in
/// `driver_run.rs`): that tier exists to give the detail pane the selected run's
/// identity, and overwriting it with this marker would cost the user the
/// information the tier was added for.
///
/// The em dash is written as a `\u{…}` escape, matching the `DRIVER_LIVE_MARKER`
/// convention in `detail.rs`.
pub(super) const DRIVER_PANE_TITLE: &str = " Driver \u{2014} EXPERIMENTAL ";

/// What the Driver tab drew that the mouse can hit (quick 260926-kes,
/// [inferred I-11]): the run list's item rows, and the selected run's LOG pane.
#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub(super) struct DriverTabRegions {
    pub list: Option<crate::ui::mouse::ListRegion>,
    pub output: Option<Rect>,
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

/// A scrolling pane's header: a DarkGray `── {name} ───` rule with the follow
/// indicator on its right. [`output_header_line`] is this with the name `output`;
/// the launcher run's LOG.md pane (30-02) is the same rule with the name `log`, so
/// the two panes share one construction and one narrow-width rule.
pub(super) fn pane_header_line(
    name: &str,
    width: u16,
    indicator: &str,
    color: Color,
) -> Line<'static> {
    let head = format!("\u{2500}\u{2500} {name} ");
    let tail_cells = indicator.chars().count() + 4; // ` ` + indicator + ` ──`
    let pad = usize::from(width)
        .saturating_sub(head.chars().count())
        .saturating_sub(tail_cells);
    Line::from(vec![
        Span::styled(format!("{head}{}", "\u{2500}".repeat(pad)), label_style()),
        Span::raw(" "),
        Span::styled(indicator.to_string(), Style::default().fg(color)),
        Span::styled(" \u{2500}\u{2500}", label_style()),
    ])
}

#[cfg(test)]
mod tests {
    use super::super::detail::ViewportMetrics;
    use super::super::{DriverLineKind, DriverOutput, ProjectViewCache};
    use super::*;
    use ratatui::Frame;
    use std::cell::Cell;

    /// The Driver tab as the detail screen draws it: the launcher-first composition.
    fn render_driver_tab(
        frame: &mut Frame,
        area: Rect,
        alias: &str,
        cache: Option<&ProjectViewCache>,
        viewport: &Cell<ViewportMetrics>,
        list_offset: &Cell<usize>,
    ) -> DriverTabRegions {
        super::super::driver_run::render_launcher_tab(
            frame,
            area,
            alias,
            cache,
            viewport,
            list_offset,
        )
    }

    /// The visible text of a `Line`, spans concatenated.
    fn text(line: &Line<'static>) -> String {
        line.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    /// The two halves of the census needle, **meaningless apart**.
    ///
    /// Neither const's own declaration line contains the assembled needle, so
    /// this module's source cannot become a hit in its own census — the
    /// anti-self-match property `text.rs`'s alphabet census establishes, and it
    /// is ASSERTED below rather than argued here.
    const CENSUS_NEEDLE_HEAD: &str = "sanitize_";
    const CENSUS_NEEDLE_TAIL: &str = "render_line(";

    /// The two files the census speaks for.
    const CENSUS_FILES: [&str; 2] = [
        "src/ui/screens/driver.rs",
        "src/ui/screens/driver_confirm.rs",
    ];

    /// A composition marker: naming either of these on the same logical line
    /// means the CONTROL-class call is composed with the invisible-class one.
    ///
    /// `shown_capped`'s own body needs no special case — it *is*
    /// `display_identity(&sanitize_render_line(value))`, so it satisfies the
    /// first marker by construction rather than by exemption.
    const CENSUS_COMPOSERS: [&str; 2] = ["display_identity", "render_for_terminal"];

    /// Split source into LOGICAL lines, each paired with the number of its FIRST
    /// physical line.
    ///
    /// A logical line ends where Rust's delimiters balance and the text closes a
    /// statement or a block — so `foo(\n  bar(baz),\n)` is one unit rather than
    /// three, and a call wrapped across five lines is judged whole.
    ///
    /// **Why this replaces the obvious "look at nearby lines" version.** The
    /// first draft of this census joined FORWARD and BACKWARD until it found a
    /// composer. That version did not go red against a planted unconverted call:
    /// `render_disclosure` composes correctly two statements above the plant, so
    /// the backward walk found `display_identity` belonging to a DIFFERENT
    /// statement and reported the plant as composed. A census that cannot go red
    /// is the exact failure this census exists to replace, so proximity was
    /// dropped for structure.
    ///
    /// Characters inside string literals are not counted as delimiters; escapes
    /// are honoured. Comment-only lines are dropped rather than accumulated.
    fn logical_lines(source: &str) -> Vec<(usize, String)> {
        let mut out: Vec<(usize, String)> = Vec::new();
        let mut current = String::new();
        let mut first = 0usize;
        let mut depth: i32 = 0;
        let mut in_string = false;
        let mut escaped = false;

        for (index, raw) in source.lines().enumerate() {
            let trimmed = raw.trim();
            if current.is_empty() && (trimmed.is_empty() || trimmed.starts_with("//")) {
                continue;
            }
            if trimmed.starts_with("//") {
                continue;
            }
            if current.is_empty() {
                first = index + 1;
            } else {
                current.push(' ');
            }
            current.push_str(trimmed);

            for c in trimmed.chars() {
                if escaped {
                    escaped = false;
                    continue;
                }
                // **Parens and brackets only — braces are BOUNDARIES, not
                // continuations.** Counting `{` here was the second defect this
                // census had: `fn f(..) {` left the depth above zero, so every
                // statement in a function body accumulated into ONE logical
                // line, which then contained some composer belonging to another
                // statement and swallowed the planted call. Found by planting,
                // not by reading.
                match c {
                    '\\' if in_string => escaped = true,
                    '"' => in_string = !in_string,
                    '(' | '[' if !in_string => depth += 1,
                    ')' | ']' if !in_string => depth -= 1,
                    _ => {}
                }
            }

            let closes = trimmed.ends_with(';') || trimmed.ends_with('{') || trimmed.ends_with('}');
            if depth <= 0 && !in_string && closes {
                out.push((first, std::mem::take(&mut current)));
                depth = 0;
            }
        }
        if !current.is_empty() {
            out.push((first, current));
        }
        out
    }

    /// Every executable call to `sanitize_render_line` in [`CENSUS_FILES`] that
    /// stands OUTSIDE a composition, as `file:line`.
    ///
    /// Comment lines are dropped and wrapped logical lines are joined, keeping
    /// the FIRST physical line's number for the report — the idiom
    /// `crate::text`'s alphabet census established.
    ///
    /// **Production lines only.** The walk stops at each file's `#[cfg(test)]`
    /// marker. The claim is about RENDER sites, and the composition-equality pin
    /// in this very module deliberately calls `sanitize_render_line` bare in
    /// order to compare the two compositions — counting that would make the
    /// census red for the test that proves the thing it is checking.
    fn composition_census() -> Vec<String> {
        let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let mut sites = Vec::new();

        for relative in CENSUS_FILES {
            let source = std::fs::read_to_string(base.join(relative))
                .unwrap_or_else(|e| panic!("the census must be able to read {relative}: {e}"));
            sites.extend(census_sites_in(relative, &production_slice(&source)));
        }
        sites
    }

    /// Every line index at which `source` carries a **column-zero**
    /// `#[cfg(test)]` whose following declaration declares a MODULE.
    ///
    /// The truncation used to cut at the first line whose TRIMMED start was the
    /// test attribute. That is a silent kill switch: a mid-file test-attributed
    /// helper drops every line after it from the scan, and nothing says so. The
    /// hazard is **live in this tree** — `src/ui/screens/detail.rs` carries two
    /// column-zero test attributes, at `:5342` (a `pub(super) fn` helper) and
    /// `:5764` (`mod tests`) — so adding that file to [`CENSUS_FILES`] under the
    /// old rule would have silently dropped more than two thousand lines.
    ///
    /// A test-attributed item that is NOT a module now stays INSIDE the scanned
    /// slice. That is the stricter direction, and therefore the safe one.
    ///
    /// # Observed RED by planting a second marker
    ///
    /// `#[cfg(test)] mod planted_second_test_module {}` appended to this file,
    /// then removed and `git status --porcelain` confirmed clean afterwards
    /// (` M src/ui/screens/driver.rs` alone):
    ///
    /// ```text
    /// thread 'ui::screens::driver::tests::no_executable_control_class_call_in_these_two_files_stands_outside_a_composition' (2985743) panicked at src/ui/screens/driver.rs:2457:13:
    /// assertion `left == right` failed: src/ui/screens/driver.rs carries 2 column-zero `#[cfg(test)]` markers whose following declaration is a MODULE, at line(s) [2030, 4199]. The census cuts at the FIRST one, so a second means every line after it is dropped from the scan without anything saying so [..]
    ///   left: 2
    ///  right: 1
    /// ```
    ///
    /// The failure NAMES both marker lines, which is the difference between a
    /// kill switch and a control: the reader is sent to the two markers rather
    /// than to the census.
    fn test_module_marker_lines(source: &str) -> Vec<usize> {
        let lines: Vec<&str> = source.lines().collect();
        let mut markers = Vec::new();
        for (index, line) in lines.iter().enumerate() {
            // `starts_with` at index 0 IS the column-zero test.
            if !line.starts_with("#[cfg(test)]") {
                continue;
            }
            let follows_a_module = lines[index + 1..]
                .iter()
                .find(|later| !later.trim().is_empty())
                .is_some_and(|declaration| declares_a_module(declaration));
            if follows_a_module {
                markers.push(index);
            }
        }
        markers
    }

    /// Does `line` declare a module, with or without a visibility qualifier?
    fn declares_a_module(line: &str) -> bool {
        let trimmed = line.trim_start();
        let rest = trimmed.strip_prefix("pub").map_or(trimmed, |after| {
            let after = if after.starts_with('(') {
                after.split_once(')').map_or(after, |(_, tail)| tail)
            } else {
                after
            };
            after.trim_start()
        });
        rest.starts_with("mod ")
    }

    /// `source` above its test module — the slice the census actually walks.
    fn production_slice(source: &str) -> String {
        let lines: Vec<&str> = source.lines().collect();
        let end = test_module_marker_lines(source)
            .first()
            .copied()
            .unwrap_or(lines.len());
        lines[..end].join("\n")
    }

    /// The census's verdict over one already-sliced source text.
    ///
    /// Extracted from [`composition_census`] so the live assertion and the
    /// laundering control consume the SAME code. Pass 11 found this census's
    /// defect by re-implementing its published algorithm in Python; a control
    /// that exercises a re-implementation certifies the re-implementation, so
    /// the control below drives this function instead.
    fn census_sites_in(label: &str, production: &str) -> Vec<String> {
        let needle = format!("{CENSUS_NEEDLE_HEAD}{CENSUS_NEEDLE_TAIL}");
        let mut sites = Vec::new();
        for (number, logical) in logical_lines(production) {
            if !logical.contains(&needle) {
                continue;
            }
            // **The verdict is taken at each OCCURRENCE, never over the unit.**
            // `CENSUS_COMPOSERS.iter().any(|c| logical.contains(c))` used to
            // stand here and it laundered: one composed arm of a joined `match`
            // answered for its bare sibling. See
            // `a_composed_arm_does_not_launder_its_un_composed_sibling`.
            if logical
                .match_indices(&needle)
                .all(|(at, _)| occurrence_is_composed(&logical[..at]))
            {
                continue;
            }
            sites.push(format!("{label}:{number}"));
        }
        sites
    }

    /// Is the needle occurrence that FOLLOWS `before` wrapped in a composer?
    ///
    /// The whole verdict, and it looks only at the characters immediately
    /// preceding one occurrence: strip a trailing run of path-qualifier
    /// segments (`super::`, `crate::text::`, repeated), trim trailing
    /// whitespace, and the occurrence is composed only if what remains ends
    /// with a composer name, an opening parenthesis and an optional reference
    /// marker.
    ///
    /// So `crate::text::display_identity(&super::` + needle is composed and
    /// `vec![` + needle is not, whichever arm of whatever expression each sits
    /// in. **There is no window here to over-join**, which is why this repair
    /// is terminal: no future round can find a bigger unit that launders
    /// something, because no unit is consulted.
    ///
    /// # Residual, with its direction — it FLIPPED
    ///
    /// A composition assembled across separate statements — the sanitiser bound
    /// to a local on one line, wrapped three lines later — reads as
    /// UN-composed. Under the old whole-unit verdict that same case was
    /// under-detection and silent; here it is **over-detection, and LOUD**: it
    /// fails the build and names the file and line. Loud is the safe direction.
    /// The repair for a loud false positive is to inline the composition, or to
    /// record an exemption with a reason — never to soften this rule.
    fn occurrence_is_composed(before: &str) -> bool {
        let trimmed = strip_trailing_path_qualifiers(before).trim_end();
        CENSUS_COMPOSERS.iter().any(|composer| {
            let open = format!("{composer}(");
            trimmed.ends_with(&open) || trimmed.ends_with(&format!("{open}&"))
        })
    }

    /// `text` with any trailing run of `identifier::` segments removed.
    ///
    /// `..display_identity(&super::` becomes `..display_identity(&`, so the
    /// composer check does not have to know which spelling of the path a given
    /// call site used. Both spellings are live in the census files: this module
    /// calls the sanitiser unqualified, `driver_confirm.rs` reaches it through
    /// `super::`.
    fn strip_trailing_path_qualifiers(text: &str) -> &str {
        let mut cut = text;
        while let Some(head) = cut.strip_suffix("::") {
            let mut boundary = head.len();
            for (index, character) in head.char_indices().rev() {
                if character.is_alphanumeric() || character == '_' {
                    boundary = index;
                } else {
                    break;
                }
            }
            if boundary == head.len() {
                return cut;
            }
            cut = &head[..boundary];
        }
        cut
    }

    /// A two-arm `match` whose arms land in ONE logical unit: one arm calls the
    /// control-class sanitiser bare, its sibling wraps it in a composer.
    ///
    /// Built at runtime from [`CENSUS_NEEDLE_HEAD`], [`CENSUS_NEEDLE_TAIL`] and
    /// [`CENSUS_COMPOSERS`]. Spelled whole, this fixture would be an executable
    /// hit in this file — in its own census, and in the per-file needle
    /// distribution `21-33` pins across `src/ui/`, in a file `21-33` cannot
    /// edit. So no token here is a raw string literal.
    fn laundering_fixture() -> String {
        let needle = format!("{CENSUS_NEEDLE_HEAD}{CENSUS_NEEDLE_TAIL}");
        let composer = CENSUS_COMPOSERS[0];
        format!(
            "fn render(action: Action) -> Vec<String> {{\n\
             match action {{\n\
             Action::Bare => vec![{needle}value)],\n\
             Action::Composed => vec![{composer}(&{needle}other))],\n\
             }}\n\
             }}\n"
        )
    }

    /// **A composed arm must not launder its un-composed sibling** (21-32 T3,
    /// T-21-32-03, pass 11 gaps[2]).
    ///
    /// [`logical_lines`] joins sibling `match` arms into one unit — correctly,
    /// because a call wrapped across five physical lines has to be judged
    /// whole. The defect was the VERDICT being taken over that unit: "does this
    /// unit contain a composer anywhere". One composed arm then answered for
    /// its bare sibling, and the census silently missed the exact construction
    /// it exists to catch.
    ///
    /// The repair deletes the window from the decision rather than making the
    /// join smarter. A smarter join is still a window and a window can still
    /// over-join; taking the verdict at the INNERMOST call leaves nothing to
    /// over-join.
    ///
    /// # Observed RED against the committed whole-unit verdict
    ///
    /// ```text
    /// thread 'ui::screens::driver::tests::a_composed_arm_does_not_launder_its_un_composed_sibling' (2971679) panicked at src/ui/screens/driver.rs:2255:9:
    /// assertion `left == right` failed: the census did not report the BARE arm of a two-arm match whose sibling is composed. [..]
    ///   left: []
    ///  right: ["fixture.rs:3"]
    /// ```
    ///
    /// `left: []` IS the laundering: the committed rule looked at the joined
    /// unit, found `display_identity` belonging to the OTHER arm, and called the
    /// bare call clean. The unit-shape assertion below passed in that same run,
    /// so the fixture really did join both arms — the red is the verdict's, not
    /// the join's.
    #[test]
    fn a_composed_arm_does_not_launder_its_un_composed_sibling() {
        let fixture = laundering_fixture();
        let needle = format!("{CENSUS_NEEDLE_HEAD}{CENSUS_NEEDLE_TAIL}");

        // The fixture really does put both arms in ONE logical unit — otherwise
        // this control would pass for the wrong reason.
        let units = logical_lines(&fixture);
        let joined: Vec<&(usize, String)> = units
            .iter()
            .filter(|(_, text)| text.matches(&needle).count() == 2)
            .collect();
        assert_eq!(
            joined.len(),
            1,
            "the fixture must join both arms into exactly one logical unit, or \
             the laundering it reproduces is not the laundering that was \
             measured. Units: {units:?}"
        );

        let sites = census_sites_in("fixture.rs", &fixture);
        assert_eq!(
            sites,
            vec!["fixture.rs:3".to_string()],
            "the census did not report the BARE arm of a two-arm match whose \
             sibling is composed. A verdict taken over the joined unit answers \
             a question about the unit, not about the calls inside it, so the \
             composed arm launders its sibling and `U+202E`, `U+00AD` and the \
             `U+E0000..U+E007F` tag block reach a terminal cell from a site the \
             census calls clean. The verdict has to be taken at each occurrence, \
             from the characters immediately preceding it."
        );
    }

    /// **IN-01: `shown_capped`'s completeness claim, replaced by the census that
    /// CHECKS it** (21-28 T3).
    ///
    /// `shown_capped`'s doc used to assert that every other site on this path had
    /// been converted. Three sites in the same file had not been, and that
    /// sentence is precisely what stopped a reader re-checking the file for a
    /// whole round. A claim no committed control can falsify is the failure this
    /// phase exists to end, so the sentence is gone and this is what stands in
    /// its place.
    ///
    /// **The zero is not vacuous.** `composition_census` is asserted to have
    /// actually read both files and to find the needle **in the slice it walks**
    /// — the arms below — so a walk that silently read nothing, a needle that
    /// matched nothing, or a truncation that quietly ate the file, is red rather
    /// than green.
    ///
    /// **Observed RED by planting**, an unconverted call added to
    /// `driver_confirm.rs`; the panic naming the planted file and line is quoted
    /// in `21-28-SUMMARY.md`. 21-32 added two further reds, both quoted in
    /// `21-32-SUMMARY.md`: the two-arm laundering fixture against the old
    /// whole-unit verdict, and a planted second column-zero test module against
    /// the truncation assertion below.
    ///
    /// # Residuals, each with its direction
    ///
    /// 1. **A composition assembled across separate statements** — the sanitiser
    ///    bound to a local on one line, wrapped three lines later — reads as
    ///    UN-composed and is reported. Under the old whole-unit verdict this was
    ///    under-detection and silent; since 21-32 moved the verdict to the
    ///    innermost call it is **over-detection, and LOUD** — it fails the build
    ///    and names the file and line. Loud is the safe direction, and the
    ///    direction changed on purpose. The repair for a loud false positive is
    ///    to inline the composition, or to record an exemption with a reason —
    ///    **never to soften the rule**.
    /// 2. **A third file added to this path tomorrow is simply not looked at.**
    ///    [`CENSUS_FILES`] is two named files, and nothing here notices a new
    ///    one. **Under-detection, silent.** What bounds that is the
    ///    render-escape probe, which renders all four of this path's sites, and
    ///    NOT this census.
    ///
    /// The census's only job is to stop an unconverted call being re-introduced
    /// into these two files unnoticed, and it is deliberately not sold as more
    /// than that.
    #[test]
    fn no_executable_control_class_call_in_these_two_files_stands_outside_a_composition() {
        let needle = format!("{CENSUS_NEEDLE_HEAD}{CENSUS_NEEDLE_TAIL}");

        // ANTI-SELF-MATCH, asserted directly rather than argued: neither half
        // spells the needle, so the two const declaration lines above cannot be
        // hits in the census that reads this very file.
        assert!(
            !CENSUS_NEEDLE_HEAD.contains(&needle) && !CENSUS_NEEDLE_TAIL.contains(&needle),
            "the needle halves must be meaningless apart, or this census reports \
             its own source"
        );
        let base = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let own = std::fs::read_to_string(base.join(CENSUS_FILES[0])).expect("own source");
        for line in own.lines() {
            if line.contains("CENSUS_NEEDLE_HEAD:") || line.contains("CENSUS_NEEDLE_TAIL:") {
                assert!(
                    !line.contains(&needle),
                    "a needle-half declaration line spells the whole needle, so \
                     the census can report itself: {line:?}"
                );
            }
        }

        // NON-VACUITY 1: the walk read real source.
        for relative in CENSUS_FILES {
            let source = std::fs::read_to_string(base.join(relative))
                .unwrap_or_else(|e| panic!("the census must be able to read {relative}: {e}"));
            assert!(
                source.lines().count() > 100,
                "{relative} read as {} lines, so a clean census would be a walk \
                 that never looked",
                source.lines().count()
            );
        }

        // TRUNCATION: exactly one column-zero test MODULE marker per file, so
        // the cut cannot become a silent kill switch. Observed RED by planting
        // a second one; the panic is quoted in `21-32-SUMMARY.md`.
        for relative in CENSUS_FILES {
            let source = std::fs::read_to_string(base.join(relative))
                .unwrap_or_else(|e| panic!("the census must be able to read {relative}: {e}"));
            let markers = test_module_marker_lines(&source);
            let numbered: Vec<usize> = markers.iter().map(|index| index + 1).collect();
            assert_eq!(
                markers.len(),
                1,
                "{relative} carries {} column-zero `#[cfg(test)]` markers whose \
                 following declaration is a MODULE, at line(s) {numbered:?}. The \
                 census cuts at the FIRST one, so a second means every line \
                 after it is dropped from the scan without anything saying so — \
                 the silent kill switch this assertion exists to make loud. \
                 Either fold the modules together or state, here, which one the \
                 census is meant to stop at and why.",
                markers.len()
            );
        }

        // NON-VACUITY 2: the needle matches something **in the slice the census
        // actually walks**. It used to be counted over the WHOLE file, so its
        // "there was something to find" guarantee was about a different set
        // than the zero below — measured at 7 whole-file needle lines against 6
        // in the production slices, the difference being `driver.rs`'s own
        // test-module call at `:2349`, which the census never scans. If every
        // production call vanished, the zero below would mean "nothing to find"
        // rather than "all composed".
        //
        // Measured over the slice at the time of writing: 2 needle-bearing
        // logical units carrying 3 occurrences — `driver.rs` 1 unit / 1
        // occurrence, `driver_confirm.rs` 1 unit / 2 occurrences (the opt-in
        // disclosure's path-and-digest line carries two). All 3 are judged
        // composed by the innermost verdict, which is what makes the zero below
        // non-vacuous.
        //
        // Moved from (5, 6, 7) by 30-01: the empty-state copy `no_runs_lines`,
        // which carried one composed call, was replaced by `driver_run`'s empty
        // state, and that draws the registry key through `shown_capped` (which
        // is itself defined here and is the one remaining call).
        //
        // Moved from (4, 5, 6) to (2, 3, 4) by 31-02: the start prompt's command
        // call and the goal row's call were removed with `DriverAction::Start`
        // and `goal_row`, so `driver_confirm.rs` lost 2 units / 2 occurrences, and
        // the whole-file count lost the same 2 lines. Two sites removed, none moved.
        //
        // 31-03 re-measured after the legacy run renderer left this file: (2, 3, 4)
        // UNCHANGED. That renderer carried no needle call (`shown_capped` is the one
        // production call here), so deleting it moves none of the three numbers.
        const PRODUCTION_NEEDLE_UNITS: usize = 2;
        const PRODUCTION_NEEDLE_OCCURRENCES: usize = 3;
        const WHOLE_FILE_NEEDLE_LINES: usize = 4;

        let mut units = 0usize;
        let mut occurrences = 0usize;
        let mut whole_file = 0usize;
        for relative in CENSUS_FILES {
            let source = std::fs::read_to_string(base.join(relative))
                .unwrap_or_else(|e| panic!("the census must be able to read {relative}: {e}"));
            whole_file += source
                .lines()
                .filter(|l| !l.trim_start().starts_with("//") && l.contains(&needle))
                .count();
            for (_, logical) in logical_lines(&production_slice(&source)) {
                let found = logical.matches(&needle).count();
                if found > 0 {
                    units += 1;
                    occurrences += found;
                }
            }
        }
        assert!(
            occurrences > 0,
            "the census found NO executable call to the control-class function \
             in the production slice of either file, so its zero below says \
             nothing about composition"
        );
        assert_eq!(
            (units, occurrences, whole_file),
            (
                PRODUCTION_NEEDLE_UNITS,
                PRODUCTION_NEEDLE_OCCURRENCES,
                WHOLE_FILE_NEEDLE_LINES
            ),
            "the census's non-vacuity totals moved. These are MEASURED numbers, \
             not targets: re-measure them, say in this comment why they changed, \
             and check that the change is a site being added or removed rather \
             than the slice quietly shifting under the census."
        );

        let sites = composition_census();
        assert_eq!(
            sites.len(),
            0,
            "{} executable call(s) to the control-class sanitiser in these two \
             files stand OUTSIDE a composition. Sites: {sites:?}. That function \
             answers only the CONTROL class — `ESC`/C0/`DEL`/C1 — so a site \
             using it alone lets `U+202E`, `U+00AD` and the `U+E0000..U+E007F` \
             tag block reach a terminal cell. Compose it with `display_identity` \
             (or use `shown_capped` / `render_for_terminal`). This is IN-01: the \
             claim that every site was converted is checked here rather than \
             asserted in a doc comment.",
            sites.len()
        );
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
            let via_carrier = crate::text::Untrusted::from_untrusted_source(
                super::super::sanitize_render_line(&raw),
            )
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

    /// Below the two-pane floor the run list is not rendered; the selection's
    /// identity moves into the pane title (`narrow_title` in `driver_run.rs`)
    /// rather than vanishing.
    #[test]
    fn the_sub_sixty_tier_returns_a_single_pane() {
        let narrow = Rect::new(0, 0, DRIVER_TWO_PANE_MIN_CELLS - 1, 24);
        let (list, detail) = driver_panes(narrow);
        assert!(
            list.is_none(),
            "the run list must be dropped below 60 columns"
        );
        assert_eq!(detail, narrow);

        let wide = Rect::new(0, 0, DRIVER_TWO_PANE_MIN_CELLS, 24);
        let (list, detail) = driver_panes(wide);
        assert!(list.is_some(), "two panes at exactly 60 columns");
        assert!(
            detail.width >= DRIVER_DETAIL_MIN_CELLS,
            "the detail pane fell below its floor at 60 columns: {}",
            detail.width
        );
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
        output.push_record(
            DriverLineKind::Terminal,
            "run ended: succeeded_with_changes",
        );
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
        assert_eq!(
            output.len(),
            1,
            "an unchanged run id must not clear the ring"
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
            let rendered = text(&pane_header_line(
                "output",
                width,
                INDICATOR_FOLLOWING,
                Color::Green,
            ));
            assert!(
                rendered.contains(INDICATOR_FOLLOWING),
                "the indicator was dropped at {width} columns: {rendered:?}"
            );
            assert!(
                rendered.starts_with("\u{2500}\u{2500} output"),
                "{rendered:?}"
            );
        }
    }

    // ── The terminal-state label vocabulary (OBS-05, D-13) ─────────────────

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
            assert_ne!(
                TerminalState::from_label(Some(label)),
                TerminalState::Unrecorded,
                "the renderer has never been taught the label {label:?}"
            );
        }
    }

    // ── 260917-fko group 4: the EXPERIMENTAL marker on the pane ───────────

    /// A launcher row that finished (`done`), built off the spike fixture.
    fn launcher_row(id: &str) -> crate::state_reader::driver_frontmatter::DriverRunEntry {
        use crate::state_reader::driver_frontmatter::{
            parse_driver_frontmatter, DerivedStatus, DriverRunView, RunStatus,
        };
        let mut frontmatter = parse_driver_frontmatter(include_str!(
            "../../../tests/fixtures/driverctl/direction/s003-run1b.md"
        ))
        .expect("the spike fixture parses");
        frontmatter.run_id = id.to_string();
        (
            id.to_string(),
            Ok(DriverRunView {
                frontmatter,
                derived: DerivedStatus::AsWritten(RunStatus::Done),
            }),
        )
    }

    /// D3's tab half, read out of a **real render buffer** rather than out of
    /// the constant.
    ///
    /// Asserting `DRIVER_PANE_TITLE.contains("EXPERIMENTAL")` would pass on a
    /// constant nothing draws; what D3 requires is that the word reaches the
    /// terminal. Both title sites are exercised — a project with runs and a
    /// project with none — because the marker matters most on the empty pane,
    /// which is the one a user reaches first.
    #[test]
    fn the_driver_pane_title_carries_the_experimental_marker_with_and_without_runs() {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;

        let dir = tempfile::tempdir().expect("temp dir");
        let alias = super::super::driver_confirm::tests::ALIAS;

        for (label, runs) in [
            ("no runs yet", Vec::new()),
            ("one finished run", vec![launcher_row("20261020-a")]),
        ] {
            let (mut ctx, _rx) = super::super::driver_confirm::tests::ctx_with_project(dir.path());
            let cache = ctx.view_cache.entry(alias.to_string()).or_default();
            cache.launcher_runs = runs;
            cache.launcher_listed = true;

            let viewport = Cell::default();
            let mut terminal =
                Terminal::new(TestBackend::new(100, 20)).expect("TestBackend terminal");
            terminal
                .draw(|frame| {
                    let area = frame.area();
                    render_driver_tab(
                        frame,
                        area,
                        alias,
                        ctx.view_cache.get(alias),
                        &viewport,
                        &Cell::new(0),
                    );
                })
                .expect("draw the driver tab");

            let buffer = terminal.backend().buffer().clone();
            let scraped: Vec<String> = (0..20)
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
                .collect();

            assert!(
                scraped.iter().any(|row| row.contains("EXPERIMENTAL")),
                "the Driver pane ({label}) must carry the EXPERIMENTAL marker \
                 where the user can read it (D3): {scraped:#?}"
            );
            assert!(
                scraped.iter().any(|row| row.contains("Driver")),
                "and it must still say which pane it is: {scraped:#?}"
            );
        }
    }
}
