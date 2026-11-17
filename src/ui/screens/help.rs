//! The help overlay — **the only place keys are documented**, and therefore the
//! only place a key this project binds is discoverable at all.
//!
//! ## Why this screen scrolls (Phase 18, UI-SPEC `## Keybindings`)
//!
//! It was a hardcoded `Vec<Line>` in a `centered_rect(area, 60, 70)` overlay
//! **with no scrolling**. It already held around thirty rows, and at an 80×24
//! terminal that popup is 48×16 — so roughly half of it was already invisible
//! before Phase 18 added a Driver key section, the sort toggle, two filter rows
//! and two legends. Adding rows to a screen that cannot show them is a
//! documentation change with no reader, which is why the scroll work is
//! required rather than nice to have.
//!
//! Three changes, all mandated: the popup grows to `centered_rect(area, 80, 90)`,
//! it scrolls with the **UIFIX-04 clamp ordering**, and it shows a more-content
//! indicator on the last visible row while content extends below the viewport.
//!
//! **One copy of the clamp formula, not three.** [`clamp_scroll`] and
//! [`ViewportMetrics`] are imported from [`super::detail`], which is where the
//! Driver output pane and the markdown file views already clamp. A third
//! hand-rolled `total - visible` expression here is exactly how UIFIX-04 came
//! back the first time, so there is deliberately no arithmetic of that shape in
//! this file.
//!
//! ## Why the legends are built from constants
//!
//! The badge legend interpolates the `BADGE_*` constants `normal.rs`
//! renders. Retyping the glyphs as literals here would let
//! the documentation drift silently away from what the dashboard actually paints
//! (T-18-63); interpolating them means a renamed or re-pointed glyph fails a
//! test instead.
//!
//! **The Phase 18 injection legend is gone** (30-04, D-02): no key on the Driver tab
//! can produce an injected message any more, so the `i` row and the `Injected Message
//! States` legend are removed, with the flag on and off. Every legend row is a glyph
//! plus a text label, so no meaning is carried by colour alone, and there is no
//! spinner and no animation anywhere on this surface.

use super::detail::{
    clamp_scroll, ViewportMetrics, PAGE_SCROLL_LINES, WAVES_GLYPH_DONE, WAVES_GLYPH_RUNNING,
    WAVES_GLYPH_STALLED,
};
use super::normal::{
    BADGE_CRASHED, BADGE_DRIVEN, BADGE_EXTERNAL_JOB, BADGE_NEEDS_HUMAN, BADGE_PAUSED,
    BADGE_SESSION, BADGE_STALE, BADGE_UNKNOWN,
};
use super::{AppContext, Screen, ScreenAction};
use crate::agents::unmerged::{GLYPH_UNMERGED, LABEL_ACTIVE, LABEL_UNMERGED};
use crate::ui::roadmap_graph::GLYPH_DONE;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;
use std::cell::Cell;

/// The indicator the last visible row carries while content extends below it.
///
/// **Not decoration.** A popup that silently hides half its rows is the same
/// "looks complete but isn't" failure the Driver pane's ring-overflow notice
/// exists to prevent, one surface over.
pub(super) const MORE_INDICATOR: &str = "\u{25BE} more";

/// The popup's share of the frame, as percentages (UI-SPEC `## Keybindings`).
///
/// Grown from 60×70 in Phase 18. At an 80×24 terminal this is 64×21 rather than
/// 48×16 — five more content rows and sixteen more columns, which is what keeps
/// the injection gloss on one line at ordinary widths.
const POPUP_WIDTH_PCT: u16 = 80;
const POPUP_HEIGHT_PCT: u16 = 90;

/// The dashboard badges, in the priority order [`super::normal`] ranks them,
/// each paired with what it actually means.
///
/// The glyph column is the shipped constant rather than a retyped literal, so
/// the legend cannot drift from the dashboard (T-18-63). The glyphs in
/// [`DRIVER_ONLY_BADGES`] are the driver surface's own and are left out with the
/// experimental features off.
const BADGE_LEGEND: [(&str, &str); 8] = [
    (
        BADGE_CRASHED,
        "driver run crashed - its lock is free, never finished",
    ),
    (BADGE_DRIVEN, "an agent is driving this repo right now"),
    (
        BADGE_NEEDS_HUMAN,
        "waiting on a human (a parked driver run too)",
    ),
    (BADGE_STALE, "driver heartbeat stale - alive but silent"),
    (
        BADGE_UNKNOWN,
        "driver run liveness unknown - lock probe failed",
    ),
    (BADGE_PAUSED, "paused - a current (non-stale) HANDOFF"),
    (BADGE_EXTERNAL_JOB, "blocked on an external job, not stuck"),
    (
        BADGE_SESSION,
        "an active Claude or Codex session in this directory",
    ),
];

/// The badge glyphs only the driver surface can paint: with the experimental
/// features off `row_badge` can never yield them, so documenting them would name
/// a glyph nothing renders. (`❢` stays: the legacy needs-a-human predicate paints
/// it with the flag off too.)
const DRIVER_ONLY_BADGES: [&str; 4] = [BADGE_CRASHED, BADGE_DRIVEN, BADGE_STALE, BADGE_UNKNOWN];

pub struct HelpScreen {
    /// How far the popup is scrolled, in rendered lines.
    ///
    /// Clamped against [`Self::viewport`] with the UIFIX-04 ordering: the
    /// down-direction arms add **then** clamp, the up-direction arms clamp
    /// **first** and subtract second.
    scroll_offset: u16,
    /// What the last render pass could show. Zero before the first pass, which
    /// yields a max scroll of zero — the correct pre-first-render floor.
    viewport: Cell<ViewportMetrics>,
}

impl Default for HelpScreen {
    fn default() -> Self {
        Self::new()
    }
}

impl HelpScreen {
    pub fn new() -> Self {
        Self {
            scroll_offset: 0,
            viewport: Cell::new(ViewportMetrics::default()),
        }
    }

    /// Help opened scrolled to the section headed `heading` (the screen's own
    /// section); the top when there is no such section.
    pub fn at_section(heading: &str, experimental: bool) -> Self {
        let mut screen = Self::new();
        if let Some(at) = help_lines(experimental).iter().position(|l| {
            l.spans
                .iter()
                .map(|s| s.content.as_ref())
                .collect::<String>()
                == heading
        }) {
            screen.scroll_offset = at as u16;
        }
        screen
    }

    /// The offset a test can read without rendering a frame.
    #[cfg(test)]
    pub(super) fn scroll_offset(&self) -> u16 {
        self.scroll_offset
    }
}

fn centered_rect(area: Rect, pct_width: u16, pct_height: u16) -> Rect {
    let width = (area.width as u32 * pct_width as u32 / 100) as u16;
    let height = (area.height as u32 * pct_height as u32 / 100) as u16;
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width, height)
}

/// The `Work State` legend (quick 260929-szq, I-12): what finished, merged,
/// unmerged, running and stalled work looks like on the Waves pane, the
/// Roadmap and the dashboard. Every glyph and word is interpolated from the
/// constants those surfaces render (T-18-63), never retyped.
fn work_state_legend() -> Vec<(String, String)> {
    vec![
        (
            format!("{WAVES_GLYPH_DONE} {GLYPH_DONE} done"),
            "merged into the main branch".to_string(),
        ),
        (
            format!("{GLYPH_UNMERGED} {LABEL_UNMERGED}"),
            "finished on a worktree branch, not yet in main".to_string(),
        ),
        (
            format!("{WAVES_GLYPH_RUNNING} {LABEL_ACTIVE}"),
            format!(
                "after a {GLYPH_UNMERGED} hint: its worktree is still locked or has a live agent"
            ),
        ),
        (
            format!("{WAVES_GLYPH_RUNNING} running"),
            "a live agent is working on a worktree".to_string(),
        ),
        (
            format!("{WAVES_GLYPH_STALLED} stalled"),
            "locked/dirty worktree, no recent activity".to_string(),
        ),
        (
            "@a0973c1".to_string(),
            "worktree branch (agent id, 7 chars); +N commits ahead, ~N dirty paths".to_string(),
        ),
        (
            format!("Dashboard {GLYPH_UNMERGED}N"),
            "N plans or quick tasks waiting to be merged".to_string(),
        ),
    ]
}

/// The `s` row: the sort toggle and what attention first ranks (36 D-13).
const SORT_ROW: &str = "Toggle sort: alphabetical / attention first (waiting on a human, then owner work, then driven)";

fn heading(text: &str) -> Line<'static> {
    Line::from(Span::styled(
        text.to_string(),
        Style::default().add_modifier(Modifier::BOLD),
    ))
}

fn row(key: &str, description: &str) -> Line<'static> {
    Line::from(format!("  {key:<13} {description}"))
}

/// The whole help body, as a pure function of one startup-resolved flag.
///
/// **Extracted from `render` on purpose**, for the reason `footer_spans` was
/// split out of `build_footer`: a `Paragraph` exposes no public text accessor,
/// so content assertions against a rendered buffer cannot tell a missing row
/// from a clipped one. A `Vec<Line>` concatenates cleanly and every content test
/// below reads this instead.
///
/// `experimental` decides whether the driver is documented here at all
/// (260917-fko D2/D3). **The help screen is the only place keys are documented**,
/// which is exactly why it has to be gated: leaving the driver rows in while the
/// surface is hidden would teach a user about a tab they cannot open and keys
/// that do nothing, and leaving the badge and injection legends in would name
/// glyphs the dashboard can no longer paint. With the flag on, the Driver
/// section's heading carries the EXPERIMENTAL marker (D3).
pub(super) fn help_lines(experimental: bool) -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = vec![
        heading("Keybindings"),
        Line::from(""),
        row("j / Down", "Move down"),
        row("k / Up", "Move up"),
        row("Enter", "Open project detail"),
        // Directly under `Enter`, because the two are the same action differing
        // only in which tab it lands on. The wording must not be byte-identical
        // to any other row in this body: the whole-row assertions below cannot
        // tell two identical rows apart, which the `o` rows' comment records.
        row("b", "Open project detail on the backlog tab"),
        row("/", "Filter projects"),
        row("s", SORT_ROW),
        row(
            "i",
            "Owner inbox: one list of what waits on you across every project (decide, test, fix)",
        ),
        row("a", "Add project"),
        row("c", "Create new project"),
        row("d", "Delete project"),
        row("G", "Global settings: the GSD global template"),
        row("?", "Toggle this help"),
    ];

    // The dashboard's `r`, `x` and `o` driver keys were retired in Phase 31 (D-03): the
    // Driver tab below documents the `x` kill and `o` opt-in keys that remain.

    lines.extend([
        row("e", "Enqueue next action (detail view)"),
        // The Roadmap tab's cursor keys (phase 24-05, D-A10). They replace a
        // stale `r` row: `r` toggles nothing on the Roadmap and is bound only
        // on the Config tab. Each wording is unique in this body, so the
        // whole-row assertions can tell them from the dashboard's `Enter`.
        row(
            "j/k g/G",
            "Roadmap tab: move the phase cursor / top / bottom (detail view)",
        ),
        row(
            "h / l",
            "Roadmap tab: jump to a dependency / to a phase it unblocks (detail view)",
        ),
        row(
            "[ / ]",
            "Roadmap tab: previous / next phase in the same wave (detail view)",
        ),
        row(
            "Space",
            "Roadmap tab: fold / unfold a milestone (detail view)",
        ),
        row(
            "Enter",
            "Roadmap tab: open the phase in Phases (detail view)",
        ),
        row(
            "Enter",
            "Backlog tab: open / close the item's content pane (detail view)",
        ),
        row(
            "j/k PgUp/PgDn",
            "Backlog tab: scroll the open content pane (detail view)",
        ),
        row(
            "e",
            "Backlog tab: edit the open item where it lives, e.g. ROADMAP.md (detail view)",
        ),
    ]);
    // Quick 261103-ubn: the Backlog tab's one-key one-off. Documented only with the
    // experimental flag on, because the key is inert without it (the 260917-fko
    // discovery gate: a help row for a dead key would advertise it).
    if experimental {
        lines.push(row(
            "t",
            "Backlog tab: start the selected item as a one-off /gsd-quick run (experimental)",
        ));
    }
    lines.extend([
        // Phase 24-07 (D-B04) made `m` the Docs / Sessions sub-tab switch;
        // since quick 260926-1t1 it is an alias of the arrows and `[` / `]`,
        // documented here only (the strip and footer no longer show it).
        row("m", "Docs / Sessions tab: switch sub-tab (alias of Left / Right) (detail view)"),
        // Quick 260927-t3s (D-01): `g` switches the Config tab's editing
        // scope. `d`, the old key, is a working alias documented here only
        // (I-2), as `m` is.
        row("g", "Config tab: switch between project and global ($GSD_HOME/.gsd/defaults.json; GSD_HOME defaults to ~) settings; d is an alias (detail view)"),
        // Still true on the dashboard; the detail view's split of the two is
        // documented in the navigation block below.
        row("q / Esc", "Quit / Back"),
        row("Ctrl+C", "Force quit"),
        Line::from(""),
        // ── The detail view's navigation model (quick 260926-1t1, D-07): a
        // tab-bar level above each tab's content. Every description is unique
        // in this body, so the whole-row assertions can tell the rows apart
        // from the dashboard's `Enter` and the Roadmap's `[ / ]`. Ends with
        // exactly one blank row, whichever section follows.
        heading("Detail View Navigation"),
        Line::from(""),
        row("1-8", "Detail view: jump straight into a tab's content"),
        row(
            "Left / Right",
            "Detail view: previous / next tab on the tab bar; sub-tab inside Sessions / Docs",
        ),
        row("[ / ]", "Sessions / Docs tab: previous / next sub-tab (detail view)"),
        // Quick 260929-g9u (I-9): the Config scope strip is a focus level of
        // its own, where the arrows switch scope.
        row(
            "Left / Right",
            "Config tab scope strip: Project / Global (Down from the tab bar, Up from the first row)",
        ),
        row("Down / Enter", "Detail view tab bar: enter the tab's content (no action)"),
        row("Up", "Detail view: back to the tab bar from the first row"),
        row("Esc", "Detail view: close a pane or level; then tab bar; then dashboard"),
        row("q", "Detail view: back to the dashboard from anywhere"),
        row("Tab", "Detail view: switch the terminal to this project's session"),
        row("Enter", "Sessions tab: resume the selected session (detail view)"),
        Line::from(""),
        // ── The Phases tab's Waves pane and the Agents cross-jump (quick
        // 260926-2l4, D-01, D-05). Every description is unique in this body,
        // so the whole-row assertions can tell them from the Roadmap's
        // `j/k g/G` and the Driver's `PgUp / PgDn`. Ends with one blank row.
        heading("Phases Waves Pane (detail view)"),
        Line::from(""),
        row("Right / Enter", "Phases tab: focus the Waves pane (detail view)"),
        row("j/k g/G", "Waves pane: move the row cursor / top / bottom"),
        row("PgUp / PgDn", "Waves pane: page the rows"),
        row("Enter / Space", "Waves pane: fold / unfold a wave; on a plan, jump to its agent"),
        row("e", "Waves pane: edit the plan's PLAN.md at its objective"),
        row("Left / Esc", "Waves pane: back to the phase list"),
        row("Enter", "Agents sub-tab: open the agent's plan in the Phases Waves pane"),
        Line::from(""),
        // ── The Phases tab's Owner block (36-04, D-21; 37-07). Expanding and
        // folding, plus the four inbox actions `v o A y` on the selected group
        // (37-03, 37-05, 37-06, 37-07), listed as one row. Every description is
        // unique in this body and ends in `(detail view)`. Ends with one blank row.
        heading("Phases Owner Block (detail view)"),
        Line::from(""),
        row("Up / k", "Phases tab: from the first phase, enter the Owner block above it (detail view)"),
        row("j/k g/G", "Owner block: move the row cursor / top / bottom (detail view)"),
        row("PgUp / PgDn", "Owner block: page the rows (detail view)"),
        row("Enter / Space", "Owner block: expand or collapse the selected group or fold (detail view)"),
        row("Left / Right", "Owner block: move between the folds on the fold line (detail view)"),
        row("v o A y", "Owner block: run, open, acknowledge or copy the selected group, as in the inbox (detail view)"),
        row("Esc", "Owner block: back to the phase list (detail view)"),
        Line::from(""),
        // ── The mouse (quick 260926-dyf, D-07). Every description is unique
        // in this body. Ends with one blank row.
        heading("Mouse"),
        Line::from(""),
        // Quick 260926-kes: row clicks on every tab, the Config value click,
        // the Roadmap fold glyph (escaped, per the house rule), the band
        // double-click, and the wheel over text panes.
        row("Click", "Mouse: switch tab / sub-tab; select a row on any tab and focus its pane"),
        row("Click value", "Mouse: Config tab: on the selected row, the same as Enter"),
        row("Click \u{25B8}/\u{25BE}", "Mouse: Roadmap: fold / unfold that milestone band"),
        row("Double-click", "Mouse: open the row, the same as Enter (Space on a Roadmap band)"),
        row("Wheel", "Mouse: move the selection, or scroll the pane, under the pointer"),
        row("M", "Toggle mouse capture on / off (dashboard and detail view)"),
        row(
            "Shift+drag",
            "Select and copy text while mouse capture is on (terminal-dependent)",
        ),
        Line::from(""),
    ]);

    // ── The Driver tab (Phase 18). There is deliberately no pane-focus
    // mode: the output pane's default is to follow its tail, so scrolling
    // is the exception rather than a second mode the user has to track.
    //
    // The whole section — heading, blank line, its rows and its trailing
    // blank — goes together when the experimental surfaces are off; the
    // blank line above is the one that survives, so the Filter Syntax
    // heading below has exactly one blank before it either way.
    if experimental {
        lines.extend([
            // D3: the literal word EXPERIMENTAL on the help screen's Driver
            // heading, one of the two places it must appear. The em dash is
            // the separator `app.rs` already uses in its status strings.
            heading("Driver Tab (detail view) \u{2014} EXPERIMENTAL"),
            Line::from(""),
            row("Shift+D", "Jump to the Driver tab from any detail tab"),
            row("j / k", "Move the run selection"),
            row("[ / ]", "Move the item cursor in the selected run"),
            row("PgUp / PgDn", "Scroll the LOG.md tail"),
            row("f", "Toggle following the LOG.md tail"),
            row("G", "Jump to the LOG.md tail and follow again"),
            row("c", "Set control: run, pause or stop (writes DIRECTION.md)"),
            row(
                "a",
                "Answer the item under the cursor (also sets hold: false)",
            ),
            row("u", "Take the item under the cursor off hold"),
            row("e", "Edit this run's DIRECTION.md in $EDITOR"),
            // The continuation of the `e` row (RESEARCH Pitfall 7, `[inferred —
            // audit]`): a GUI editor that returns at once leaves the file unchanged.
            row("", "$VISUAL/$EDITOR must be a terminal editor or"),
            row("", "a blocking wrapper script; a GUI editor that"),
            row("", "returns at once sees an unchanged file."),
            row(
                "s",
                "Start a new run: name it, edit it, check it, launch it",
            ),
            row(
                "R",
                "Launch or relaunch this run (asks first, shows the spend ceiling)",
            ),
            row("x", "Kill this run's sandbox now (asks first)"),
            // The Driver tab's `o` is the only opt-in key (the dashboard's was retired
            // in Phase 31, D-03).
            row("o", "Toggle driver opt-in for this project"),
            // Quick 261103-f5y: one-off sandboxed tasks, no driver run needed.
            row(
                "t",
                "One-off task: pick a todo or type one; q = /gsd-quick, f = /gsd-fast (asks first); starts another run even while one is live",
            ),
            row(
                "T",
                "One-off runs: j/k select, Enter details; m merge, p push, D discard, x stop (TERM, then KILL), X kill now, z pause, u resume (b from its branch, M merge master first), e SUMMARY, d merge diff (o: full diff in $EDITOR)",
            ),
            row(
                "v",
                "In T: v live view of the selected run (r: its newest conflict-resolver session; PgUp/PgDn, j/k, f, G; Esc closes)",
            ),
            row(
                "x",
                "In T or the live view: x stop (TERM, then KILL after 10 s), X kill now, z pause (commit, report partial, end), u resume a paused, stopped or killed run (or retry a paused landing); each asks first. On a run being landed x/X/z act on its conflict resolver",
            ),
            row("i", "In the live view: send a message to the running agent"),
            row(
                "b",
                "In the live view: set or clear the soft spend cap (judged against the lower-bound estimate; asks the agent to wrap up, then stops it)",
            ),
            Line::from(""),
            // ── The run-state legend (30-01, D-04): glyph, word and gloss for every
            // status the run list can show, glyphs from the `driver_run` constants.
            heading("Driver Run States"),
            Line::from(""),
        ]);
        for (glyph, word, gloss) in super::driver_run::RUN_STATES_LEGEND {
            lines.push(Line::from(format!("  {glyph} {word:<16} {gloss}")));
        }
        lines.push(Line::from(""));
        // ── The item-state legend (30-02, D-06): every state an item row can show,
        // glyphs from the `driver_run` constants.
        lines.push(heading("Driver Item States"));
        lines.push(Line::from(""));
        for (glyph, word, gloss) in super::driver_run::ITEM_STATES_LEGEND {
            lines.push(Line::from(format!("  {glyph} {word:<16} {gloss}")));
        }
        lines.push(Line::from(""));
    }

    lines.extend([
        heading("Filter Syntax"),
        Line::from(""),
        row("/term", "Search all columns"),
        row("/term/n", "Search name only"),
        row("/term/p", "Search phase only"),
        row("/term/s", "Search status only"),
        row("/term/h", "Search all columns, needs-a-human rows only"),
        row("//h", "Every project waiting on a human"),
        row("/term/y", "Search all columns, owner-work rows only"),
        row("//y", "Every project with owner work (decide, test or fix)"),
        row(
            "//y test",
            "Owner work in one lane: test, decide or fix (short: //yt //yd //yf)",
        ),
        Line::from(""),
        // ── What the You column and the summary line say (36 D-21). Only
        // surfaces this build has; the inbox screen's own section follows.
        heading("Owner Work"),
        Line::from(""),
        row(
            "You",
            "Dashboard column, 100+ columns wide: what waits on you in the current milestone",
        ),
        row(
            "decide N",
            "N rulings wait on you (confirmations, handoffs, open questions)",
        ),
        row(
            "test N",
            "N checks only you can run; \u{2192} closes M = M phases that close by testing alone",
        ),
        row(
            "fix N ph",
            "N phases need code (failed checks, gaps, unreadable checks); without ph, a debug investigation counts too",
        ),
        row(
            "\u{b7}",
            "Nothing waits on you here (blocked work is not yours to act on)",
        ),
        row("(+N old)", "N more test, decide or fix items in archived milestones (detail Old debt adds blocked ones)"),
        row("(blank)", "No owner data: not loaded yet, or the folder or its .planning/ is gone"),
        Line::from(""),
        // ── The inbox screen (37-07, D-20). Keys in the overlay's ASCII
        // vocabulary (the glyph scan forbids raw arrows under src/ui/). Every
        // description is unique in this body, so the whole-row assertions can
        // tell a row from a same-key row elsewhere. Ends with one blank row.
        heading("Owner Inbox"),
        Line::from(""),
        row("1 2 3 4", "Show one lane: Test, Decide, Fix, Blocked (press the key again for the default view)"),
        row("a", "Show all four lanes, Blocked included"),
        row("j/k Up/Down", "Move the cursor; PgUp/PgDn page, Home/End jump to the first or last row"),
        row("Enter", "Expand or collapse the selected group or fold (inbox)"),
        row("Left / Right", "On the fold line: move between Old debt, Stale, Acknowledged and Triage (inbox)"),
        row("g", "Go to the project: detail view, Phases tab, on the same group when visible there"),
        row("o", "Open the group's source file, read-only (never your editor)"),
        row("v", "Run the group's command in its project through your multiplexer; a live session is focused instead and you type the command there"),
        row("A", "Acknowledge the group's file via gsd-core: file-level, uncommitted, asks first"),
        row("y", "Copy the group as a plain checklist (wl-copy, xclip, xsel, pbcopy, clip.exe, else OSC 52)"),
        row("Stale", "Fold of phases idle for more than 90 days (preferences.owner.fold_days); still counted, Enter lists them"),
        row("decision?", "Label only: the text reads like a ruling; looks resolved: it says fixed; a label never moves or hides a row"),
        row("Esc / q", "Back to the dashboard (the dashboard selection is unchanged)"),
        Line::from(""),
        // ── The badge legend. Glyph plus text label on every row, so nothing
        // is carried by colour alone.
        heading("Dashboard Badges"),
        Line::from(""),
    ]);

    for (glyph, meaning) in BADGE_LEGEND {
        // The driven, crashed, stale and unknown badges are the dashboard's
        // driver surface; with the experimental features off `row_badge` can
        // never yield them, so documenting them would name a glyph nothing
        // paints.
        if !experimental && DRIVER_ONLY_BADGES.contains(&glyph) {
            continue;
        }
        lines.push(Line::from(format!("  {glyph} {meaning}")));
    }
    lines.push(Line::from(""));

    // ── The work-state legend (quick 260929-szq): merged vs unmerged work.
    lines.push(heading("Work State"));
    lines.push(Line::from(""));
    for (key, meaning) in work_state_legend() {
        lines.push(row(&key, &meaning));
    }
    lines.push(Line::from(""));

    lines.push(Line::from(Span::styled(
        "Press ? or Esc to close   j/k, PgUp/PgDn scroll".to_string(),
        Style::default().add_modifier(Modifier::DIM),
    )));

    wrap_owner_sections(lines)
}

/// The widest help row the Owner sections may have: the popup is 80% of a
/// 100-column terminal, 78 cells inside its border, minus a little slack.
pub(super) const HELP_ROW_MAX: usize = 76;

/// The cells a description may use after the two-space indent and the key column.
const HELP_DESC_MAX: usize = HELP_ROW_MAX - 2 - 13 - 1;

/// `description` hand-wrapped at spaces to [`HELP_DESC_MAX`] cells: the first row
/// carries `key`, every continuation hangs under the description with an empty key
/// (the same convention as the Driver rows above). One `String` per row.
pub(super) fn wrapped_row(key: &str, description: &str) -> Vec<String> {
    let mut parts: Vec<String> = Vec::new();
    let mut current = String::new();
    for word in description.split(' ') {
        let fits = current.is_empty()
            || super::owner_block::cells(&current) + 1 + super::owner_block::cells(word)
                <= HELP_DESC_MAX;
        if !fits {
            parts.push(std::mem::take(&mut current));
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    parts.push(current);
    parts
        .iter()
        .enumerate()
        .map(|(i, part)| {
            let key = if i == 0 { key } else { "" };
            format!("  {key:<13} {part}")
        })
        .collect()
}

/// Wrap every owner-related row wider than [`HELP_ROW_MAX`] (the "Owner Work" and
/// "Owner Inbox" sections, and any row that documents the Owner block or the
/// owner inbox), so no row of the owner screens is cut at 100 columns. Other
/// rows keep their authored text.
fn wrap_owner_sections(lines: Vec<Line<'static>>) -> Vec<Line<'static>> {
    let text = |l: &Line<'static>| {
        l.spans
            .iter()
            .map(|s| s.content.as_ref())
            .collect::<String>()
    };
    let start = lines.iter().position(|l| text(l) == "Owner Work");
    let end = lines.iter().position(|l| text(l) == "Dashboard Badges");
    let mut out = Vec::with_capacity(lines.len() + 24);
    for (i, line) in lines.into_iter().enumerate() {
        let t = text(&line);
        let in_section = matches!((start, end), (Some(s), Some(e)) if i > s && i < e);
        let owner_row = in_section || t.contains("Owner block") || t.contains("Owner inbox");
        if owner_row && t.starts_with("  ") && super::owner_block::cells(&t) > HELP_ROW_MAX {
            let key = t.get(2..15).unwrap_or("").trim_end().to_owned();
            let desc = t.get(16..).unwrap_or("").to_owned();
            out.extend(wrapped_row(&key, &desc).into_iter().map(Line::from));
        } else {
            out.push(line);
        }
    }
    out
}

crate::ui::screens::adjudicate_screen!(
    HelpScreen,
    crate::ui::screens::RENDERS_NO_ATTACKER_INFLUENCED_IDENTITY,
    "Draws `help_lines(ctx.experimental)`: keybindings, the filter grammar \
     and two legends, every byte of it authored in this repository. The one \
     value it takes from `AppContext` is a startup-resolved bool that selects \
     between two authored bodies — no borrowed text, no identity, no project \
     state. It `Clear`s its popup area and paints no background, so nothing \
     attacker-influenced reaches a cell. Checked, not claimed: the fixture \
     registers the hostile identity and puts a hostile project state behind \
     it, and the probe asserts the clean stem is absent from the buffer.",
);

impl Screen for HelpScreen {
    fn handle_key(
        &mut self,
        code: KeyCode,
        _modifiers: KeyModifiers,
        ctx: &mut AppContext,
    ) -> ScreenAction {
        let vp = self.viewport.get();
        match code {
            KeyCode::Char('?') | KeyCode::Esc => {
                ctx.needs_redraw = true;
                ScreenAction::Pop
            }
            // Down-direction: add the delta, **then** clamp (UIFIX-04).
            KeyCode::Char('j') | KeyCode::Down => {
                self.scroll_offset = clamp_scroll(
                    self.scroll_offset.saturating_add(1),
                    vp.total_lines,
                    vp.visible_height,
                );
                ctx.needs_redraw = true;
                ScreenAction::None
            }
            KeyCode::PageDown => {
                self.scroll_offset = clamp_scroll(
                    self.scroll_offset.saturating_add(PAGE_SCROLL_LINES),
                    vp.total_lines,
                    vp.visible_height,
                );
                ctx.needs_redraw = true;
                ScreenAction::None
            }
            // Up-direction: clamp **first**, subtract second. The other order is
            // the UIFIX-04 defect — an offset parked past the end swallows the
            // first press and the viewport appears frozen.
            KeyCode::Char('k') | KeyCode::Up => {
                self.scroll_offset =
                    clamp_scroll(self.scroll_offset, vp.total_lines, vp.visible_height)
                        .saturating_sub(1);
                ctx.needs_redraw = true;
                ScreenAction::None
            }
            KeyCode::PageUp => {
                self.scroll_offset =
                    clamp_scroll(self.scroll_offset, vp.total_lines, vp.visible_height)
                        .saturating_sub(PAGE_SCROLL_LINES);
                ctx.needs_redraw = true;
                ScreenAction::None
            }
            _ => ScreenAction::None, // Consume all other keys
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        // Help renders as an overlay on top of whatever is below
        let popup_area = centered_rect(area, POPUP_WIDTH_PCT, POPUP_HEIGHT_PCT);
        frame.render_widget(Clear, popup_area);

        let help_text = help_lines(ctx.experimental);
        let block = Block::default().borders(Borders::ALL).title(" Help ");
        let inner = block.inner(popup_area);

        // Record for the key handler, which cannot see this pass.
        let total_lines = help_text.len() as u16;
        self.viewport.set(ViewportMetrics {
            total_lines,
            visible_height: inner.height,
        });
        let offset = clamp_scroll(self.scroll_offset, total_lines, inner.height);

        let paragraph = Paragraph::new(help_text).block(block).scroll((offset, 0));
        frame.render_widget(paragraph, popup_area);

        if more_below(offset, total_lines, inner.height) && inner.height > 0 {
            let indicator_row = Rect::new(inner.x, inner.y + inner.height - 1, inner.width, 1);
            frame.render_widget(
                Paragraph::new(Line::from(Span::styled(
                    MORE_INDICATOR,
                    Style::default().fg(Color::DarkGray),
                ))),
                indicator_row,
            );
        }
    }

    fn name(&self) -> &str {
        "help"
    }
}

/// Whether content extends below the viewport at `offset`.
///
/// Split out so the indicator's condition is assertable without a frame, and
/// expressed as a comparison rather than as a second max-scroll subtraction —
/// the one arithmetic shape this file must not grow a copy of.
pub(super) fn more_below(offset: u16, total_lines: u16, visible_height: u16) -> bool {
    offset.saturating_add(visible_height) < total_lines
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::screens::driver_confirm::tests::ctx_with_project;

    /// Every rendered row, joined — the form a content assertion can search
    /// without caring which line a phrase landed on.
    fn body() -> String {
        body_with(true)
    }

    /// The same, at a chosen `experimental` flag — the form the 260917-fko
    /// flag-off assertions read.
    fn body_with(experimental: bool) -> String {
        help_lines(experimental)
            .iter()
            .map(|line| {
                line.spans
                    .iter()
                    .map(|span| span.content.as_ref())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    /// The body with every run of whitespace collapsed, so a gloss wrapped
    /// across two rows still reads as one sentence.
    fn flowed() -> String {
        body().split_whitespace().collect::<Vec<_>>().join(" ")
    }

    fn press(screen: &mut HelpScreen, ctx: &mut AppContext, code: KeyCode) {
        screen.handle_key(code, KeyModifiers::NONE, ctx);
    }

    /// A screen whose viewport says the content is `total` lines in a `visible`
    /// row window, as a render pass would have recorded.
    fn screen_with_viewport(total: u16, visible: u16) -> HelpScreen {
        let screen = HelpScreen::new();
        screen.viewport.set(ViewportMetrics {
            total_lines: total,
            visible_height: visible,
        });
        screen
    }

    /// The Backlog tab's one-off row (quick 261103-ubn), built through `row()`.
    fn backlog_t_row() -> String {
        row(
            "t",
            "Backlog tab: start the selected item as a one-off /gsd-quick run (experimental)",
        )
        .spans[0]
            .content
            .to_string()
    }

    #[test]
    fn the_backlog_one_off_row_is_documented_only_with_the_flag_on() {
        let on = body_with(true);
        assert_eq!(
            on.lines().filter(|line| *line == backlog_t_row()).count(),
            1,
            "{on}"
        );
        let off = body_with(false);
        assert!(
            !off.lines().any(|line| line == backlog_t_row()),
            "a row for a key that is inert with the flag off:\n{off}"
        );
        assert!(!off.contains("start the selected item as a one-off"));
    }

    #[test]
    fn help_lines_documents_every_key_this_phase_binds() {
        let text = body();
        // Asserted as **whole rows**, not as substrings. `text.contains("x")`
        // is satisfied by the word "next" and `contains("s")` by almost
        // anything, so a substring check on a one-letter key is vacuous —
        // exactly the shape of gate this phase exists to stop shipping.
        // Building the expected row through the same `row()` helper the body
        // uses means the assertion is "this key is documented, with this
        // meaning, on its own row".
        for (key, description) in [
            ("Shift+D", "Jump to the Driver tab from any detail tab"),
            ("j / k", "Move the run selection"),
            ("[ / ]", "Move the item cursor in the selected run"),
            ("PgUp / PgDn", "Scroll the LOG.md tail"),
            ("f", "Toggle following the LOG.md tail"),
            ("G", "Jump to the LOG.md tail and follow again"),
            ("c", "Set control: run, pause or stop (writes DIRECTION.md)"),
            (
                "a",
                "Answer the item under the cursor (also sets hold: false)",
            ),
            ("u", "Take the item under the cursor off hold"),
            ("e", "Edit this run's DIRECTION.md in $EDITOR"),
            (
                "s",
                "Start a new run: name it, edit it, check it, launch it",
            ),
            (
                "R",
                "Launch or relaunch this run (asks first, shows the spend ceiling)",
            ),
            ("x", "Kill this run's sandbox now (asks first)"),
            ("o", "Toggle driver opt-in for this project"),
            (
                "t",
                "One-off task: pick a todo or type one; q = /gsd-quick, f = /gsd-fast (asks first); starts another run even while one is live",
            ),
            (
                "T",
                "One-off runs: j/k select, Enter details; m merge, p push, D discard, x stop (TERM, then KILL), X kill now, z pause, u resume (b from its branch, M merge master first), e SUMMARY, d merge diff (o: full diff in $EDITOR)",
            ),
            (
                "v",
                "In T: v live view of the selected run (r: its newest conflict-resolver session; PgUp/PgDn, j/k, f, G; Esc closes)",
            ),
            (
                "x",
                "In T or the live view: x stop (TERM, then KILL after 10 s), X kill now, z pause (commit, report partial, end), u resume a paused, stopped or killed run (or retry a paused landing); each asks first. On a run being landed x/X/z act on its conflict resolver",
            ),
            ("i", "In the live view: send a message to the running agent"),
            (
                "b",
                "In the live view: set or clear the soft spend cap (judged against the lower-bound estimate; asks the agent to wrap up, then stops it)",
            ),
        ] {
            let expected = row(key, description).spans[0].content.to_string();
            assert!(
                text.lines().any(|line| line == expected),
                "the help screen is the ONLY place keys are documented, and \
                 the row {expected:?} is missing from it:\n{text}"
            );
        }
        // 37-07 D-20: the inbox screen's section and the Owner block's `v o A y`
        // row, each asserted as a whole row.
        let heading_at = text
            .lines()
            .position(|line| line == "Owner Inbox")
            .expect("the Owner Inbox heading");
        let after_work = text
            .lines()
            .position(|line| line == "Owner Work")
            .expect("the Owner Work heading");
        assert!(
            after_work < heading_at,
            "the inbox section follows Owner Work"
        );
        for (key, description) in [
            (
                "1 2 3 4",
                "Show one lane: Test, Decide, Fix, Blocked (press the key again for the default view)",
            ),
            ("a", "Show all four lanes, Blocked included"),
            (
                "j/k Up/Down",
                "Move the cursor; PgUp/PgDn page, Home/End jump to the first or last row",
            ),
            (
                "Enter",
                "Expand or collapse the selected group or fold (inbox)",
            ),
            (
                "Left / Right",
                "On the fold line: move between Old debt, Stale, Acknowledged and Triage (inbox)",
            ),
            (
                "g",
                "Go to the project: detail view, Phases tab, on the same group when visible there",
            ),
            ("o", "Open the group's source file, read-only (never your editor)"),
            (
                "v",
                "Run the group's command in its project through your multiplexer; a live session is focused instead and you type the command there",
            ),
            (
                "A",
                "Acknowledge the group's file via gsd-core: file-level, uncommitted, asks first",
            ),
            (
                "y",
                "Copy the group as a plain checklist (wl-copy, xclip, xsel, pbcopy, clip.exe, else OSC 52)",
            ),
            (
                "Stale",
                "Fold of phases idle for more than 90 days (preferences.owner.fold_days); still counted, Enter lists them",
            ),
            (
                "decision?",
                "Label only: the text reads like a ruling; looks resolved: it says fixed; a label never moves or hides a row",
            ),
            (
                "Esc / q",
                "Back to the dashboard (the dashboard selection is unchanged)",
            ),
            (
                "v o A y",
                "Owner block: run, open, acknowledge or copy the selected group, as in the inbox (detail view)",
            ),
        ] {
            let lines: Vec<&str> = text.lines().collect();
            for expected in wrapped_row(key, description) {
                assert_eq!(
                    lines.iter().filter(|line| **line == expected).count(),
                    1,
                    "the row {expected:?} must appear exactly once:\n{text}"
                );
            }
            assert!(key.chars().count() <= 13, "{key:?} fits the key column");
        }
        // The dashboard sort toggle and the two needs-a-human filter forms.
        assert!(
            text.contains("Toggle sort"),
            "the dashboard sort toggle must be documented:\n{text}"
        );
        // The dashboard's backlog shortcut (260916-vqz). Asserted beside the
        // sort toggle rather than appended to the list above, which is the
        // Driver TAB's section — `b` is a dashboard key and widening that list
        // would misfile it. Still a WHOLE ROW: `text.contains("b")` is
        // satisfied by the word "backlog" itself, by "Toggle", and by most of
        // this file.
        let backlog_row = row("b", "Open project detail on the backlog tab").spans[0]
            .content
            .to_string();
        assert!(
            text.lines().any(|line| line == backlog_row),
            "the dashboard's backlog shortcut must be documented, and the row \
             {backlog_row:?} is missing:\n{text}"
        );
        // Quick 261103-ubn: the Backlog tab's one-off key, flag-on, and directly
        // under the Backlog tab's `e` row.
        let lines: Vec<&str> = text.lines().collect();
        let edit = row(
            "e",
            "Backlog tab: edit the open item where it lives, e.g. ROADMAP.md (detail view)",
        )
        .spans[0]
            .content
            .to_string();
        let at = lines
            .iter()
            .position(|line| *line == edit)
            .expect("the Backlog `e` row");
        assert_eq!(lines[at + 1], backlog_t_row(), "{text}");
        assert!(
            text.contains("/term/h") && text.contains("//h"),
            "both needs-a-human filter forms must be documented:\n{text}"
        );
    }

    #[test]
    fn help_documents_the_owner_block_keys() {
        let text = body();
        let rows: [(&str, &str); 7] = [
            (
                "Up / k",
                "Phases tab: from the first phase, enter the Owner block above it (detail view)",
            ),
            (
                "j/k g/G",
                "Owner block: move the row cursor / top / bottom (detail view)",
            ),
            ("PgUp / PgDn", "Owner block: page the rows (detail view)"),
            (
                "Enter / Space",
                "Owner block: expand or collapse the selected group or fold (detail view)",
            ),
            (
                "Left / Right",
                "Owner block: move between the folds on the fold line (detail view)",
            ),
            (
                "v o A y",
                "Owner block: run, open, acknowledge or copy the selected group, as in the inbox (detail view)",
            ),
            ("Esc", "Owner block: back to the phase list (detail view)"),
        ];
        let lines: Vec<&str> = text.lines().collect();
        for (key, description) in rows {
            let wrapped = wrapped_row(key, description);
            for expected in &wrapped {
                assert!(
                    lines.iter().any(|line| line == expected),
                    "missing the whole row {expected:?}:\n{text}"
                );
            }
            assert_eq!(
                lines.iter().filter(|l| **l == wrapped[0]).count(),
                1,
                "{description:?} must be unique in the body"
            );
        }
        // Only what the block binds (37-07, OWN-23): nothing this phase does not
        // build is advertised for it (no mouse action, no injection into a session,
        // no per-item result, no native acknowledge writer).
        let block: String = lines
            .iter()
            .filter(|l| l.contains("Owner block"))
            .copied()
            .collect::<Vec<_>>()
            .join("\n")
            .to_lowercase();
        for forbidden in ["mouse", "click", "inject", "pass", "fail", "native"] {
            assert!(!block.contains(forbidden), "{forbidden}: {block}");
        }
    }

    #[test]
    fn owner_sections_are_hand_wrapped_and_nothing_is_cut_at_100_columns() {
        let lines = help_lines(false);
        let text: Vec<String> = lines
            .iter()
            .map(|l| l.spans.iter().map(|s| s.content.as_ref()).collect())
            .collect();
        let start = text.iter().position(|l| l == "Owner Work").unwrap();
        let end = text.iter().position(|l| l == "Dashboard Badges").unwrap();
        for l in &text[start..end] {
            assert!(
                crate::ui::screens::owner_block::cells(l) <= 78,
                "cut at 100 columns: {l:?}"
            );
        }
        // A continuation hangs under the description.
        let v = text
            .iter()
            .position(|l| l.starts_with(&format!("  {:<13} Run the group", "v")))
            .expect("v row");
        assert!(
            text[v + 1].starts_with(&format!("  {:<13} ", "")),
            "{:?}",
            text[v + 1]
        );
        // Nothing is lost: the wrapped words equal the authored description.
        let joined: String = text[v..]
            .iter()
            .enumerate()
            .take_while(|(i, l)| *i == 0 || l.starts_with("                "))
            .map(|(_, l)| l)
            .map(|l| l.get(16..).unwrap_or("").to_owned())
            .collect::<Vec<_>>()
            .join(" ");
        assert!(joined.ends_with("you type the command there"), "{joined}");
        // The overlay opened from the inbox starts at the Owner Inbox heading.
        let screen = HelpScreen::at_section("Owner Inbox", false);
        assert_eq!(
            usize::from(screen.scroll_offset()),
            text.iter().position(|l| l == "Owner Inbox").unwrap()
        );
        assert_eq!(
            HelpScreen::at_section("No such heading", false).scroll_offset(),
            0
        );
    }

    #[test]
    fn help_opened_from_the_inbox_shows_the_inbox_section_at_100x30() {
        use ratatui::{backend::TestBackend, Terminal};
        let (ctx, _rx) = ctx_with_project(std::path::Path::new("/tmp/x"));
        let screen = HelpScreen::at_section("Owner Inbox", ctx.experimental);
        let mut terminal = Terminal::new(TestBackend::new(100, 30)).expect("backend");
        terminal
            .draw(|frame| {
                let area = frame.area();
                Screen::render(&screen, frame, area, &ctx);
            })
            .expect("draw");
        let buffer = terminal.backend().buffer().clone();
        let text: String = (0..30)
            .map(|y| {
                (0..100)
                    .map(|x| buffer.cell((x, y)).expect("cell").symbol().to_owned())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n");
        assert!(text.contains("Owner Inbox"), "{text}");
        assert!(text.contains("Show one lane"), "{text}");
        assert!(!text.contains("Open project detail"), "{text}");
    }

    #[test]
    fn help_documents_the_owner_work_column_sort_and_filter() {
        let text = body();
        let rows: [(&str, &str); 12] = [
            (
                "s",
                SORT_ROW,
            ),
            (
                "i",
                "Owner inbox: one list of what waits on you across every project (decide, test, fix)",
            ),
            ("/term/y", "Search all columns, owner-work rows only"),
            ("//y", "Every project with owner work (decide, test or fix)"),
            (
                "//y test",
                "Owner work in one lane: test, decide or fix (short: //yt //yd //yf)",
            ),
            (
                "You",
                "Dashboard column, 100+ columns wide: what waits on you in the current milestone",
            ),
            (
                "decide N",
                "N rulings wait on you (confirmations, handoffs, open questions)",
            ),
            (
                "test N",
                "N checks only you can run; \u{2192} closes M = M phases that close by testing alone",
            ),
            (
                "fix N ph",
                "N phases need code (failed checks, gaps, unreadable checks); without ph, a debug investigation counts too",
            ),
            (
                "\u{b7}",
                "Nothing waits on you here (blocked work is not yours to act on)",
            ),
            ("(+N old)", "N more test, decide or fix items in archived milestones (detail Old debt adds blocked ones)"),
            ("(blank)", "No owner data: not loaded yet, or the folder or its .planning/ is gone"),
        ];
        let lines: Vec<&str> = text.lines().collect();
        for (key, description) in rows {
            // Rows that are not owner rows keep their authored single line.
            let authored = row(key, description).spans[0].content.to_string();
            let wrapped = if lines.contains(&authored.as_str()) {
                vec![authored]
            } else {
                wrapped_row(key, description)
            };
            for expected in &wrapped {
                assert!(
                    lines.iter().any(|line| line == expected),
                    "missing the whole row {expected:?}:\n{text}"
                );
            }
            assert_eq!(
                lines.iter().filter(|l| **l == wrapped[0]).count(),
                1,
                "{description:?} must be unique in the body"
            );
        }
        let at = |heading: &str| {
            lines
                .iter()
                .position(|l| *l == heading)
                .unwrap_or_else(|| panic!("no {heading} heading:\n{text}"))
        };
        assert!(at("Owner Work") > at("Filter Syntax"));
        assert!(at("Owner Work") < at("Dashboard Badges"));
    }

    #[test]
    fn the_session_badge_legend_names_both_agents() {
        // 260923-lr9: a Codex session lights the same badge as a Claude one.
        assert!(
            flowed().contains("Claude or Codex session"),
            "the session badge legend must name both agents"
        );
    }

    #[test]
    fn the_badge_legend_names_all_five_dashboard_glyphs_by_their_constants() {
        let text = body();
        // Compared against the exported constants rather than against literals
        // retyped here: a renamed or re-pointed glyph must fail this test
        // instead of silently making the documentation wrong (T-18-63).
        for glyph in [
            BADGE_DRIVEN,
            BADGE_NEEDS_HUMAN,
            BADGE_PAUSED,
            BADGE_EXTERNAL_JOB,
            BADGE_SESSION,
        ] {
            assert!(
                text.contains(glyph.trim()),
                "the badge legend must name {glyph:?}, which the dashboard \
                 actually paints:\n{text}"
            );
        }
        // Glyph plus text label on every row: nothing is carried by colour
        // alone, which a monochrome terminal is the test for.
        for (_, meaning) in BADGE_LEGEND {
            assert!(text.contains(meaning), "missing the meaning {meaning:?}");
        }
    }

    #[test]
    fn the_work_state_legend_is_built_from_the_rendered_glyphs() {
        for experimental in [false, true] {
            let text = body_with(experimental);
            let lines: Vec<&str> = text.lines().collect();
            let at = lines
                .iter()
                .position(|l| *l == "Work State")
                .unwrap_or_else(|| panic!("no Work State heading:\n{text}"));
            let badges = lines
                .iter()
                .position(|l| *l == "Dashboard Badges")
                .expect("the badge legend");
            assert!(at > badges, "Work State follows Dashboard Badges");
            // Whole rows, built from the same constants the surfaces paint
            // (T-18-63): a re-pointed glyph fails here, never silently.
            for (key, meaning) in [
                (
                    format!("{WAVES_GLYPH_DONE} {GLYPH_DONE} done"),
                    "merged into the main branch".to_string(),
                ),
                (
                    format!("{GLYPH_UNMERGED} {LABEL_UNMERGED}"),
                    "finished on a worktree branch, not yet in main".to_string(),
                ),
                (
                    format!("{WAVES_GLYPH_RUNNING} {LABEL_ACTIVE}"),
                    format!(
                        "after a {GLYPH_UNMERGED} hint: its worktree is still locked or has a live agent"
                    ),
                ),
                (
                    format!("{WAVES_GLYPH_RUNNING} running"),
                    "a live agent is working on a worktree".to_string(),
                ),
                (
                    format!("{WAVES_GLYPH_STALLED} stalled"),
                    "locked/dirty worktree, no recent activity".to_string(),
                ),
                (
                    "@a0973c1".to_string(),
                    "worktree branch (agent id, 7 chars); +N commits ahead, ~N dirty paths"
                        .to_string(),
                ),
                (
                    format!("Dashboard {GLYPH_UNMERGED}N"),
                    "N plans or quick tasks waiting to be merged".to_string(),
                ),
            ] {
                let expected = row(&key, &meaning).spans[0].content.to_string();
                assert!(
                    lines[at..].iter().any(|l| *l == expected),
                    "missing {expected:?}:\n{text}"
                );
                assert_eq!(
                    text.matches(meaning.as_str()).count(),
                    1,
                    "{meaning:?} must be unique in the body"
                );
            }
        }
    }

    /// Phase 31 retired the Driver tab's read-only legacy runs section (D-03): the help
    /// names neither the section nor its `L` toggle, with the experimental features on
    /// or off.
    #[test]
    fn the_help_names_no_legacy_runs_section_and_no_toggle_key() {
        for experimental in [true, false] {
            let text = body_with(experimental);
            assert!(
                !text.to_lowercase().contains("legacy runs"),
                "experimental={experimental}:\n{text}"
            );
            let toggle_row = row("L", "Show or hide legacy runs (read-only)").spans[0]
                .content
                .to_string();
            assert!(
                !text.lines().any(|line| line == toggle_row),
                "experimental={experimental}:\n{text}"
            );
            assert!(!text.contains("Legacy runs are"), "{text}");
        }
    }

    /// Phase 30 retired message injection (D-02): there is no `i` row and no
    /// `Injected Message States` legend, with the experimental features on or off.
    #[test]
    fn launcher_steer_the_help_has_no_injection_row_and_no_injected_message_legend() {
        use crate::ui::glyphs::{BALLOT_X, CIRCLE_FILLED, CIRCLE_OPEN, INCREMENT};
        for experimental in [true, false] {
            let text = body_with(experimental);
            assert!(
                !text.contains("Injected Message States"),
                "experimental={experimental}:\n{text}"
            );
            let inject_row = row("i", "Inject a message into the live run").spans[0]
                .content
                .to_string();
            assert!(
                !text.lines().any(|line| line == inject_row),
                "experimental={experimental}:\n{text}"
            );
            assert!(!text.contains("Inject a message"), "{text}");
            for (glyph, label) in [
                (CIRCLE_OPEN, "queued"),
                (INCREMENT, "delivered"),
                (CIRCLE_FILLED, "acted-on"),
                (BALLOT_X, "missed"),
            ] {
                let legend_form = format!("{glyph} {label}");
                assert!(
                    !text.contains(&legend_form),
                    "the legend row {legend_form:?} survived (experimental={experimental}):\n{text}"
                );
            }
        }
    }

    /// The Driver section documents the steering keys, in the UI-SPEC order, with the
    /// flag on, and none of them with it off.
    #[test]
    fn launcher_steer_the_help_documents_c_a_and_u_only_with_the_flag_on() {
        let on = body_with(true);
        let lines: Vec<&str> = on.lines().collect();
        let at = |description: &str| {
            lines
                .iter()
                .position(|l| l.contains(description))
                .unwrap_or_else(|| panic!("no row says {description:?}:\n{on}"))
        };
        let g = at("Jump to the LOG.md tail and follow again");
        let c = at("Set control: run, pause or stop (writes DIRECTION.md)");
        let a = at("Answer the item under the cursor (also sets hold: false)");
        let u = at("Take the item under the cursor off hold");
        let e = at("Edit this run's DIRECTION.md in $EDITOR");
        let s = at("Start a new run: name it, edit it, check it, launch it");
        let r = at("Launch or relaunch this run (asks first, shows the spend ceiling)");
        assert!(
            g < c && c < a && a < u && u < e && e < s && s < r,
            "UI-SPEC order:\n{on}"
        );
        // The old start row is gone: `s` no longer asks for a command and a goal.
        assert!(
            !on.contains("Start a run: command, then optional goal"),
            "{on}"
        );
        // The continuation rows warn that the editor must block (Pitfall 7).
        let flow = flowed();
        assert!(
            flow.contains("terminal editor or a blocking wrapper script"),
            "{flow}"
        );
        assert!(flow.contains("GUI editor that returns at once"), "{flow}");
        let off = body_with(false);
        for description in [
            "Set control: run, pause or stop",
            "Answer the item under the cursor",
            "Take the item under the cursor off hold",
            "Edit this run's DIRECTION.md in $EDITOR",
            "Start a new run: name it, edit it, check it, launch it",
            "Launch or relaunch this run",
        ] {
            assert!(!off.contains(description), "{description}:\n{off}");
        }
    }

    #[test]
    fn the_help_screen_never_says_sent_received_or_acknowledged() {
        // `Acknowledged` is also the name of the inbox's fold (37-07), so the fold
        // list is taken out before the word check.
        let text = flowed()
            .to_lowercase()
            .replace("old debt, stale, acknowledged and triage", "");
        // The steering rows must not promise an observation the protocol cannot
        // make: a write to DIRECTION.md is not the driver having applied it (D-28).
        for forbidden in ["sent", "received", "acknowledged"] {
            assert!(
                !text.contains(forbidden),
                "the word {forbidden:?} is forbidden on this surface (D-28):\n{text}"
            );
        }
    }

    #[test]
    fn page_down_then_page_up_returns_to_the_top() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut ctx, _rx) = ctx_with_project(dir.path());
        let mut screen = screen_with_viewport(200, 20);

        press(&mut screen, &mut ctx, KeyCode::PageDown);
        assert_eq!(screen.scroll_offset(), PAGE_SCROLL_LINES);
        press(&mut screen, &mut ctx, KeyCode::PageUp);
        assert_eq!(
            screen.scroll_offset(),
            0,
            "one page down and one page up is a round trip"
        );
    }

    #[test]
    fn page_up_at_the_top_stays_at_the_top() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut ctx, _rx) = ctx_with_project(dir.path());
        let mut screen = screen_with_viewport(200, 20);

        press(&mut screen, &mut ctx, KeyCode::PageUp);
        press(&mut screen, &mut ctx, KeyCode::Char('k'));
        assert_eq!(screen.scroll_offset(), 0, "u16 must not wrap under zero");
    }

    #[test]
    fn scrolling_past_the_end_clamps_and_the_first_page_up_still_moves() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut ctx, _rx) = ctx_with_project(dir.path());
        // 50 lines in a 20-row window: the last page starts at 30.
        let mut screen = screen_with_viewport(50, 20);

        for _ in 0..10 {
            press(&mut screen, &mut ctx, KeyCode::PageDown);
        }
        assert_eq!(
            screen.scroll_offset(),
            30,
            "the offset clamps to total - visible and does not grow past it"
        );

        // The UIFIX-04 backstop: because the up arm clamps FIRST and subtracts
        // second, the very first PageUp after repeated over-scrolling moves.
        press(&mut screen, &mut ctx, KeyCode::PageUp);
        assert_eq!(screen.scroll_offset(), 10);
    }

    #[test]
    fn the_more_indicator_appears_only_while_content_extends_below_the_viewport() {
        assert!(
            more_below(0, 60, 20),
            "sixty lines in a twenty-row window has more below"
        );
        assert!(
            !more_below(40, 60, 20),
            "at the last page there is nothing below, so the indicator must go"
        );
        assert!(
            !more_below(0, 12, 20),
            "content shorter than the viewport never shows the indicator"
        );
        assert!(
            !more_below(0, 20, 20),
            "content exactly filling the viewport never shows the indicator"
        );
    }

    // ── 260917-fko group 5: the help screen in both flag states ───────────
    //
    // The help screen is the ONLY place keys are documented, which is why it
    // is gated rather than trimmed: with the driver hidden, a driver row would
    // document a tab the user cannot open.

    /// Every driver row this screen carries, built through the same `row()`
    /// helper the body uses — so the flag-off assertion is the absence of
    /// WHOLE ROWS, never the absence of a one-letter key. `text.contains("x")`
    /// is satisfied by the word "next"; a letter-level check here would be
    /// vacuous in exactly the way this repo already refuses.
    fn driver_rows() -> Vec<String> {
        [
            ("Shift+D", "Jump to the Driver tab from any detail tab"),
            ("j / k", "Move the run selection"),
            ("[ / ]", "Move the item cursor in the selected run"),
            ("PgUp / PgDn", "Scroll the LOG.md tail"),
            ("f", "Toggle following the LOG.md tail"),
            ("G", "Jump to the LOG.md tail and follow again"),
            ("c", "Set control: run, pause or stop (writes DIRECTION.md)"),
            (
                "a",
                "Answer the item under the cursor (also sets hold: false)",
            ),
            ("u", "Take the item under the cursor off hold"),
            ("e", "Edit this run's DIRECTION.md in $EDITOR"),
            (
                "s",
                "Start a new run: name it, edit it, check it, launch it",
            ),
            (
                "R",
                "Launch or relaunch this run (asks first, shows the spend ceiling)",
            ),
            ("x", "Kill this run's sandbox now (asks first)"),
            ("o", "Toggle driver opt-in for this project"),
            (
                "t",
                "One-off task: pick a todo or type one; q = /gsd-quick, f = /gsd-fast (asks first); starts another run even while one is live",
            ),
            (
                "T",
                "One-off runs: j/k select, Enter details; m merge, p push, D discard, x stop (TERM, then KILL), X kill now, z pause, u resume (b from its branch, M merge master first), e SUMMARY, d merge diff (o: full diff in $EDITOR)",
            ),
            (
                "v",
                "In T: v live view of the selected run (r: its newest conflict-resolver session; PgUp/PgDn, j/k, f, G; Esc closes)",
            ),
            (
                "x",
                "In T or the live view: x stop (TERM, then KILL after 10 s), X kill now, z pause (commit, report partial, end), u resume a paused, stopped or killed run (or retry a paused landing); each asks first. On a run being landed x/X/z act on its conflict resolver",
            ),
            ("i", "In the live view: send a message to the running agent"),
            (
                "b",
                "In the live view: set or clear the soft spend cap (judged against the lower-bound estimate; asks the agent to wrap up, then stops it)",
            ),
        ]
        .into_iter()
        .map(|(key, description)| row(key, description).spans[0].content.to_string())
        .collect()
    }

    #[test]
    fn the_help_screen_documents_no_driver_row_when_experimental_is_off() {
        let text = body_with(false);
        for expected in driver_rows() {
            assert!(
                !text.lines().any(|line| line == expected),
                "the row {expected:?} documents a surface the user cannot \
                 reach with the flag off:\n{text}"
            );
        }
    }

    /// 30-01: the Driver tab's run-state legend names every status the run list
    /// can show, with its glyph, and the glyphs are the `driver_run` constants.
    #[test]
    fn the_driver_run_states_legend_names_every_run_status_with_its_glyph() {
        use crate::ui::screens::driver_run::{
            GLYPH_RUN_CRASHED, GLYPH_RUN_DONE, GLYPH_RUN_LIVE, GLYPH_RUN_PARKED, GLYPH_RUN_UNKNOWN,
            GLYPH_RUN_UNLAUNCHED, RUN_STATES_LEGEND,
        };
        let text = body_with(true);
        assert!(
            text.lines().any(|line| line == "Driver Run States"),
            "the legend heading is missing:\n{text}"
        );
        for (glyph, word, _) in RUN_STATES_LEGEND {
            let form = format!("  {glyph} {word:<16} ");
            assert!(
                text.lines().any(|line| line.starts_with(&form)),
                "the legend row for {word:?} is missing:\n{text}"
            );
        }
        // The rows the failing and the waiting states live on, by glyph and word.
        for (glyph, word) in [
            (GLYPH_RUN_LIVE, "running"),
            (GLYPH_RUN_CRASHED, "crashed"),
            (GLYPH_RUN_UNKNOWN, "liveness unknown"),
            (GLYPH_RUN_UNLAUNCHED, "not launched"),
            (GLYPH_RUN_PARKED, "parked"),
            (GLYPH_RUN_DONE, "done"),
        ] {
            assert!(
                text.contains(&format!("{glyph} {word}")),
                "{glyph} {word}:\n{text}"
            );
        }
        // Glyph, word and gloss: no row carries its meaning in a glyph alone.
        assert!(RUN_STATES_LEGEND
            .iter()
            .all(|(_, word, gloss)| !word.is_empty() && !gloss.is_empty()));
    }

    /// 30-02: the item-state legend names every state an item row can show, with the
    /// glyph the row draws, and goes with the Driver section when the flag is off.
    #[test]
    fn the_driver_item_states_legend_names_every_item_state_with_its_glyph() {
        use crate::ui::screens::driver_run::ITEM_STATES_LEGEND;
        let text = body_with(true);
        assert!(
            text.lines().any(|line| line == "Driver Item States"),
            "the legend heading is missing:\n{text}"
        );
        assert_eq!(
            ITEM_STATES_LEGEND.len(),
            7,
            "the seven item rows of the UI-SPEC"
        );
        for (glyph, word, gloss) in ITEM_STATES_LEGEND {
            assert!(!word.is_empty() && !gloss.is_empty());
            let form = format!("  {glyph} {word:<16} ");
            assert!(
                text.lines().any(|line| line.starts_with(&form)),
                "the legend row for {word:?} is missing:\n{text}"
            );
        }
        for word in ["pending", "running", "done", "parked", "failed", "unseen"] {
            assert!(
                ITEM_STATES_LEGEND.iter().any(|(_, w, _)| *w == word),
                "{word}"
            );
        }
        let off = body_with(false);
        assert!(
            !off.contains("Driver Item States"),
            "the item legend documents a tab the user cannot open:\n{off}"
        );
    }

    #[test]
    fn the_help_screen_has_no_driver_run_states_legend_when_off() {
        let text = body_with(false);
        assert!(
            !text.contains("Driver Run States"),
            "the run-state legend documents a tab the user cannot open:\n{text}"
        );
        assert!(
            !text.contains("liveness unknown") && !text.contains("not launched"),
            "a legend row survived:\n{text}"
        );
    }

    #[test]
    fn the_help_screen_has_no_driver_heading_and_no_injection_legend_when_off() {
        let text = body_with(false);
        assert!(
            !text.contains("Driver Tab"),
            "the Driver section heading must go with its rows:\n{text}"
        );
        assert!(
            !text.contains("Injected Message States"),
            "no injection legend exists any more:\n{text}"
        );
        assert!(
            !text.contains(BADGE_DRIVEN.trim()),
            "the driven badge cannot be painted with the flag off, so naming \
             it in the legend would document a glyph nothing renders:\n{text}"
        );
        // The four badges that survive, so the skip above removed exactly one.
        for glyph in [
            BADGE_NEEDS_HUMAN,
            BADGE_PAUSED,
            BADGE_EXTERNAL_JOB,
            BADGE_SESSION,
        ] {
            assert!(
                text.contains(glyph.trim()),
                "{glyph:?} is not a driver badge and must stay:\n{text}"
            );
        }
    }

    /// D3's help-screen half: with the flag ON the Driver heading carries the
    /// literal word EXPERIMENTAL.
    #[test]
    fn the_driver_heading_is_marked_experimental_when_the_flag_is_on() {
        let text = body_with(true);
        let heading_line = text
            .lines()
            .find(|line| line.contains("Driver Tab"))
            .unwrap_or_else(|| panic!("the Driver section must be present:\n{text}"));
        assert!(
            heading_line.contains("EXPERIMENTAL"),
            "the Driver section heading must say EXPERIMENTAL in so many \
             letters (D3), and reads {heading_line:?}"
        );
    }

    /// The gating must not leave the seam it cut visible: no run of two blank
    /// lines, and no trailing blank run before the closing hint.
    #[test]
    fn neither_flag_state_leaves_a_double_blank_line_where_a_section_was_cut() {
        for experimental in [false, true] {
            let lines = help_lines(experimental);
            let text: Vec<String> = lines
                .iter()
                .map(|line| {
                    line.spans
                        .iter()
                        .map(|span| span.content.as_ref())
                        .collect::<String>()
                })
                .collect();
            for pair in text.windows(2) {
                assert!(
                    !(pair[0].is_empty() && pair[1].is_empty()),
                    "experimental={experimental}: a removed section left two \
                     blank rows behind:\n{text:#?}"
                );
            }
            assert!(
                !text.last().expect("a non-empty body").is_empty(),
                "experimental={experimental}: the body must not end on a blank \
                 row:\n{text:#?}"
            );
        }
    }

    /// The flag-off body is genuinely shorter — the negative assertions above
    /// cannot all be passing because `body_with` returned nothing.
    #[test]
    fn the_flag_off_help_body_is_shorter_but_still_documents_the_other_keys() {
        let off = help_lines(false).len();
        let on = help_lines(true).len();
        assert!(
            off < on,
            "the flag-off body ({off} rows) must be shorter than the flag-on \
             one ({on} rows)"
        );
        let text = body_with(false);
        for (key, description) in [
            ("j / Down", "Move down"),
            ("b", "Open project detail on the backlog tab"),
            ("e", "Enqueue next action (detail view)"),
            (
                "Space",
                "Roadmap tab: fold / unfold a milestone (detail view)",
            ),
            ("q / Esc", "Quit / Back"),
        ] {
            let expected = row(key, description).spans[0].content.to_string();
            assert!(
                text.lines().any(|line| line == expected),
                "{expected:?} is not a driver row and must survive the \
                 gate:\n{text}"
            );
        }
        assert!(text.contains("Filter Syntax") && text.contains("Dashboard Badges"));
    }

    #[test]
    fn the_body_is_long_enough_that_the_scroll_work_was_required() {
        // The premise of the whole change, pinned: at an 80x24 terminal the
        // grown popup's inner height is 24*90/100 - 2 = 19 rows, and the body is
        // far longer than that. If this ever stops being true the scrolling is
        // still correct, but the claim in the module doc would not be.
        let total = help_lines(true).len();
        assert!(
            total > 19,
            "the help body is {total} rows; the 80x24 popup shows 19, so the \
             popup must scroll"
        );
    }

    /// Phase 24-05: every Roadmap cursor key is documented as a WHOLE row,
    /// exactly once, with the flag on and off (they are not driver keys).
    #[test]
    fn the_roadmap_cursor_keys_are_documented_as_whole_rows() {
        for experimental in [true, false] {
            let text = body_with(experimental);
            for (key, description) in [
                (
                    "j/k g/G",
                    "Roadmap tab: move the phase cursor / top / bottom (detail view)",
                ),
                (
                    "h / l",
                    "Roadmap tab: jump to a dependency / to a phase it unblocks (detail view)",
                ),
                (
                    "[ / ]",
                    "Roadmap tab: previous / next phase in the same wave (detail view)",
                ),
                (
                    "Space",
                    "Roadmap tab: fold / unfold a milestone (detail view)",
                ),
                (
                    "Enter",
                    "Roadmap tab: open the phase in Phases (detail view)",
                ),
            ] {
                let expected = row(key, description).spans[0].content.to_string();
                assert_eq!(
                    text.lines().filter(|line| *line == expected).count(),
                    1,
                    "experimental={experimental}: the row {expected:?} must appear \
                     exactly once:\n{text}"
                );
            }
        }
    }

    /// Phase 24-07 (D-B04) and 25-05 (D-C15): `m` switches sub-view on the
    /// Docs tab (Files / Milestones) and on the Sessions tab (Sessions /
    /// Agents) — since quick 260926-1t1 as an alias of the arrows. One shared
    /// row documents both, as a WHOLE row, exactly once, with the flag on and
    /// off (it is not a driver key).
    /// Quick 260927-t3s (D-01, I-2): `g` switches the Config tab between
    /// project and global settings; `d` is its alias, named only here. One
    /// WHOLE row, exactly once, with the flag on and off.
    #[test]
    fn the_g_config_scope_switch_is_documented() {
        let expected = row(
            "g",
            "Config tab: switch between project and global ($GSD_HOME/.gsd/defaults.json; GSD_HOME defaults to ~) settings; d is an alias (detail view)",
        )
        .spans[0]
            .content
            .to_string();
        for experimental in [true, false] {
            let text = body_with(experimental);
            assert_eq!(
                text.lines().filter(|line| *line == expected).count(),
                1,
                "experimental={experimental}: the row {expected:?} must appear \
                 exactly once:\n{text}"
            );
        }
    }

    /// Quick 260929-g9u (I-9): the Config scope strip's arrows. One WHOLE
    /// row, exactly once, with the flag on and off.
    #[test]
    fn the_config_scope_strip_arrows_are_documented() {
        let expected = row(
            "Left / Right",
            "Config tab scope strip: Project / Global (Down from the tab bar, Up from the first row)",
        )
        .spans[0]
            .content
            .to_string();
        for experimental in [true, false] {
            let text = body_with(experimental);
            assert_eq!(
                text.lines().filter(|line| *line == expected).count(),
                1,
                "experimental={experimental}: the row {expected:?} must appear \
                 exactly once:\n{text}"
            );
        }
    }

    #[test]
    fn the_m_sub_view_switch_is_documented() {
        let expected = row(
            "m",
            "Docs / Sessions tab: switch sub-tab (alias of Left / Right) (detail view)",
        )
        .spans[0]
            .content
            .to_string();
        for experimental in [true, false] {
            let text = body_with(experimental);
            assert_eq!(
                text.lines().filter(|line| *line == expected).count(),
                1,
                "experimental={experimental}: the row {expected:?} must appear \
                 exactly once:\n{text}"
            );
        }
    }

    /// Quick 260926-1t1 (D-07): the detail view's navigation model — digits,
    /// the arrows, `[ / ]`, Up / Down / Enter between levels, the Esc vs q
    /// split and Tab — is documented under its own heading as WHOLE rows,
    /// each exactly once, with the flag on and off (none is a driver key).
    #[test]
    fn the_detail_navigation_block_is_documented_as_whole_rows() {
        for experimental in [true, false] {
            let text = body_with(experimental);
            assert_eq!(
                text.lines()
                    .filter(|line| *line == "Detail View Navigation")
                    .count(),
                1,
                "experimental={experimental}: the heading must appear once:\n{text}"
            );
            for (key, description) in [
                ("1-8", "Detail view: jump straight into a tab's content"),
                (
                    "Left / Right",
                    "Detail view: previous / next tab on the tab bar; sub-tab inside Sessions / Docs",
                ),
                ("[ / ]", "Sessions / Docs tab: previous / next sub-tab (detail view)"),
                ("Down / Enter", "Detail view tab bar: enter the tab's content (no action)"),
                ("Up", "Detail view: back to the tab bar from the first row"),
                ("Esc", "Detail view: close a pane or level; then tab bar; then dashboard"),
                ("q", "Detail view: back to the dashboard from anywhere"),
                ("Tab", "Detail view: switch the terminal to this project's session"),
                ("Enter", "Sessions tab: resume the selected session (detail view)"),
            ] {
                let expected = row(key, description).spans[0].content.to_string();
                assert_eq!(
                    text.lines().filter(|line| *line == expected).count(),
                    1,
                    "experimental={experimental}: the row {expected:?} must appear \
                     exactly once:\n{text}"
                );
            }
        }
    }

    /// Phase 24-05: the stale `r` row ("Toggle roadmap visualization") is
    /// gone — `r` toggles nothing on the Roadmap tab.
    #[test]
    fn the_stale_roadmap_toggle_row_is_gone() {
        for line in help_lines(true) {
            let text: String = line.spans.iter().map(|s| s.content.as_ref()).collect();
            let Some(rest) = text.strip_prefix("  r ") else {
                continue;
            };
            assert!(
                !rest.to_lowercase().contains("roadmap"),
                "a stale `r` Roadmap row survives: {text:?}"
            );
        }
        assert!(!body().contains("Toggle roadmap visualization"));
    }

    /// Quick 260926-2l4 (D-01, D-05): the Phases Waves pane's keys and the
    /// Agents `Enter` cross-jump are documented under their own heading as
    /// WHOLE rows, each exactly once, with the flag on and off.
    #[test]
    fn the_waves_pane_block_is_documented_as_whole_rows() {
        for experimental in [true, false] {
            let text = body_with(experimental);
            assert_eq!(
                text.lines()
                    .filter(|line| *line == "Phases Waves Pane (detail view)")
                    .count(),
                1,
                "experimental={experimental}: the heading must appear once:\n{text}"
            );
            for (key, description) in [
                (
                    "Right / Enter",
                    "Phases tab: focus the Waves pane (detail view)",
                ),
                ("j/k g/G", "Waves pane: move the row cursor / top / bottom"),
                ("PgUp / PgDn", "Waves pane: page the rows"),
                (
                    "Enter / Space",
                    "Waves pane: fold / unfold a wave; on a plan, jump to its agent",
                ),
                ("e", "Waves pane: edit the plan's PLAN.md at its objective"),
                ("Left / Esc", "Waves pane: back to the phase list"),
                (
                    "Enter",
                    "Agents sub-tab: open the agent's plan in the Phases Waves pane",
                ),
            ] {
                let expected = row(key, description).spans[0].content.to_string();
                assert_eq!(
                    text.lines().filter(|line| *line == expected).count(),
                    1,
                    "experimental={experimental}: the row {expected:?} must appear \
                     exactly once:\n{text}"
                );
            }
        }
    }

    /// quick 260926-dyf D-07: the mouse block, as whole rows, in both flag
    /// states.
    #[test]
    fn mouse_help_block_is_documented_as_whole_rows() {
        for experimental in [true, false] {
            let text = body_with(experimental);
            assert_eq!(
                text.lines().filter(|line| *line == "Mouse").count(),
                1,
                "experimental={experimental}: the heading must appear once:\n{text}"
            );
            for (key, description) in [
                (
                    "Click",
                    "Mouse: switch tab / sub-tab; select a row on any tab and focus its pane",
                ),
                (
                    "Click value",
                    "Mouse: Config tab: on the selected row, the same as Enter",
                ),
                (
                    "Click \u{25B8}/\u{25BE}",
                    "Mouse: Roadmap: fold / unfold that milestone band",
                ),
                (
                    "Double-click",
                    "Mouse: open the row, the same as Enter (Space on a Roadmap band)",
                ),
                (
                    "Wheel",
                    "Mouse: move the selection, or scroll the pane, under the pointer",
                ),
                (
                    "M",
                    "Toggle mouse capture on / off (dashboard and detail view)",
                ),
                (
                    "Shift+drag",
                    "Select and copy text while mouse capture is on (terminal-dependent)",
                ),
            ] {
                let expected = row(key, description).spans[0].content.to_string();
                assert_eq!(
                    text.lines().filter(|line| *line == expected).count(),
                    1,
                    "experimental={experimental}: the row {expected:?} must appear \
                     exactly once:\n{text}"
                );
                assert_eq!(
                    text.matches(description).count(),
                    1,
                    "{description:?} must be unique in the body"
                );
            }
            assert!(text.contains("terminal-dependent"));
        }
    }

    /// 30-03: the legend names the crashed, stale and unknown launcher badges by their
    /// constants with the flag on, and none of the three glyphs appears with it off.
    #[test]
    fn launcher_marker_the_badge_legend_gains_three_rows_gated_on_the_flag() {
        let on = body_with(true);
        let off = body_with(false);
        for glyph in [BADGE_CRASHED, BADGE_STALE, BADGE_UNKNOWN] {
            assert!(on.contains(glyph.trim()), "{glyph:?} missing:\n{on}");
            assert!(
                !off.contains(glyph.trim()),
                "{glyph:?} documented with the flag off:\n{off}"
            );
        }
        // The shipped flag glyph stays: the legacy predicate paints it either way.
        assert!(off.contains(BADGE_NEEDS_HUMAN.trim()));
    }
}
