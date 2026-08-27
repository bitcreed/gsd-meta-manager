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
//! The badge legend interpolates the five `BADGE_*` constants `normal.rs`
//! renders and the injection legend interpolates the four `GLYPH_*`/`LABEL_*`
//! constants `driver.rs` renders. Retyping the glyphs as literals here would let
//! the documentation drift silently away from what the dashboard actually paints
//! (T-18-63); interpolating them means a renamed or re-pointed glyph fails a
//! test instead.
//!
//! **The injection gloss is a safety property, not copy** (D-07, D-10). The
//! words *sent*, *received*, *read* and *acknowledged* are forbidden for an
//! injected message: `delivered` means the write to the agent's stdin returned
//! without error and **it may be about a minute before the agent picks it up**;
//! `acted-on` means the agent **dequeued** it. Every legend row is a glyph plus
//! a text label, so no meaning is carried by colour alone, and there is no
//! spinner and no animation anywhere on this surface.

use super::detail::{clamp_scroll, ViewportMetrics, PAGE_SCROLL_LINES};
use super::driver::{
    GLYPH_ACTED_ON, GLYPH_DELIVERED, GLYPH_MISSED, GLYPH_QUEUED, LABEL_ACTED_ON, LABEL_DELIVERED,
    LABEL_MISSED, LABEL_QUEUED,
};
use super::normal::{
    BADGE_DRIVEN, BADGE_EXTERNAL_JOB, BADGE_NEEDS_HUMAN, BADGE_PAUSED, BADGE_SESSION,
};
use super::{AppContext, Screen, ScreenAction};
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

/// The five dashboard badges, in the priority order [`super::normal`] ranks
/// them, each paired with what it actually means.
///
/// The glyph column is the shipped constant rather than a retyped literal, so
/// the legend cannot drift from the dashboard (T-18-63).
const BADGE_LEGEND: [(&str, &str); 5] = [
    (BADGE_DRIVEN, "an agent is driving this repo right now"),
    (BADGE_NEEDS_HUMAN, "waiting on a human"),
    (BADGE_PAUSED, "paused - a non-empty HANDOFF"),
    (BADGE_EXTERNAL_JOB, "blocked on an external job, not stuck"),
    (BADGE_SESSION, "an active Claude session in this directory"),
];

/// The four states an injected message can be in, with the honest gloss.
///
/// The third column is wrapped by hand across the rows below rather than by a
/// `Paragraph::wrap`, because the popup's own scroll offset is measured in
/// rendered lines and soft wrapping would make the two disagree.
const INJECTION_LEGEND: [(&str, &str, &str); 4] = [
    (
        GLYPH_QUEUED,
        LABEL_QUEUED,
        "durably on disk; nothing has read it yet",
    ),
    (
        GLYPH_DELIVERED,
        LABEL_DELIVERED,
        "written to the agent's stdin; it may be a minute",
    ),
    (
        GLYPH_ACTED_ON,
        LABEL_ACTED_ON,
        "the agent dequeued it and is running it as its",
    ),
    (
        GLYPH_MISSED,
        LABEL_MISSED,
        // **Not "the run closed its input"** (CR-03). That is one of two
        // reasons a message can be undeliverable; the other is a stdin write
        // that failed while the run was still live. Naming only the first here
        // would contradict the gloss the pane prints on the message's own third
        // row, and would tell a user the run had stopped listening when it may
        // still be running.
        "undeliverable and never retried; the row under",
    ),
];

/// The continuation row for each legend entry above, or `""` for none.
///
/// Indexed in lockstep with [`INJECTION_LEGEND`]; a test joins the two and
/// asserts the resulting sentences, so a row split differently still passes and
/// a row whose gloss changes meaning does not.
const INJECTION_LEGEND_CONT: [&str; 4] = [
    "",
    "before the agent picks it up",
    "own turn",
    "the message says why",
];

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

fn heading(text: &str) -> Line<'static> {
    Line::from(Span::styled(
        text.to_string(),
        Style::default().add_modifier(Modifier::BOLD),
    ))
}

fn row(key: &str, description: &str) -> Line<'static> {
    Line::from(format!("  {key:<13} {description}"))
}

/// The whole help body, as a pure function of nothing.
///
/// **Extracted from `render` on purpose**, for the reason `footer_spans` was
/// split out of `build_footer`: a `Paragraph` exposes no public text accessor,
/// so content assertions against a rendered buffer cannot tell a missing row
/// from a clipped one. A `Vec<Line>` concatenates cleanly and every content test
/// below reads this instead.
pub(super) fn help_lines() -> Vec<Line<'static>> {
    let mut lines: Vec<Line<'static>> = vec![
        heading("Keybindings"),
        Line::from(""),
        row("j / Down", "Move down"),
        row("k / Up", "Move up"),
        row("Enter", "Open project detail"),
        row("/", "Filter projects"),
        row("s", "Toggle sort: alphabetical / attention first"),
        row("a", "Add project"),
        row("c", "Create new project"),
        row("d", "Delete project"),
        row("?", "Toggle this help"),
        row("r", "Start a driver run (dashboard)"),
        row("x", "Stop the live driver run (dashboard)"),
        row("o", "Toggle driver opt-in (dashboard)"),
        row("e", "Enqueue next action (detail view)"),
        row("r", "Toggle roadmap visualization (detail view)"),
        row("q / Esc", "Quit / Back"),
        row("Ctrl+C", "Force quit"),
        Line::from(""),
        // ── The Driver tab (Phase 18). There is deliberately no pane-focus
        // mode: the output pane's default is to follow its tail, so scrolling
        // is the exception rather than a second mode the user has to track.
        heading("Driver Tab (detail view)"),
        Line::from(""),
        row("Shift+D", "Jump to the Driver tab from any detail tab"),
        row("j / k", "Move the run selection"),
        row("PgUp / PgDn", "Scroll the output pane"),
        row("f", "Toggle following the live tail"),
        row("G", "Jump to the output tail and follow again"),
        row("i", "Inject a message into the live run"),
        row("s", "Start a run: command, then optional goal"),
        row("x", "Stop the live run (asks first)"),
        // Both surfaces bind `o` for the same action and the help lists both,
        // which is what makes the consistency visible instead of coincidental.
        // The two descriptions must not be byte-identical, or the whole-row
        // assertion below cannot tell which section it matched — the dashboard
        // row at the top is disambiguated by its trailing parenthetical, and
        // this one's wording differs throughout.
        row("o", "Toggle driver opt-in for this project"),
        Line::from(""),
        heading("Filter Syntax"),
        Line::from(""),
        row("/term", "Search all columns"),
        row("/term/n", "Search name only"),
        row("/term/p", "Search phase only"),
        row("/term/s", "Search status only"),
        row("/term/h", "Search all columns, needs-a-human rows only"),
        row("//h", "Every project waiting on a human"),
        Line::from(""),
        // ── The badge legend. Glyph plus text label on every row, so nothing
        // is carried by colour alone.
        heading("Dashboard Badges"),
        Line::from(""),
    ];

    for (glyph, meaning) in BADGE_LEGEND {
        lines.push(Line::from(format!("  {glyph} {meaning}")));
    }
    lines.push(Line::from(""));

    // ── The injection legend, with the honest gloss (D-07, D-10).
    lines.push(heading("Injected Message States"));
    lines.push(Line::from(""));
    for (index, (glyph, label, gloss)) in INJECTION_LEGEND.into_iter().enumerate() {
        lines.push(Line::from(format!("  {glyph} {label:<11} {gloss}")));
        let continuation = INJECTION_LEGEND_CONT[index];
        if !continuation.is_empty() {
            lines.push(Line::from(format!("  {:<14}{continuation}", "")));
        }
    }
    lines.push(Line::from(""));

    lines.push(Line::from(Span::styled(
        "Press ? or Esc to close   j/k, PgUp/PgDn scroll".to_string(),
        Style::default().add_modifier(Modifier::DIM),
    )));

    lines
}

crate::ui::screens::adjudicate_screen!(
    HelpScreen,
    crate::ui::screens::RENDERS_NO_ATTACKER_INFLUENCED_IDENTITY,
    "Draws `help_lines()`, which the module doc calls a pure function of \
     nothing: keybindings, the filter grammar and two legends, every byte of \
     it authored in this repository. It `Clear`s its popup area and paints \
     no background, so nothing from `AppContext` reaches a cell. Checked, \
     not claimed: the fixture registers the hostile identity and puts a \
     hostile project state behind it, and the probe asserts the clean stem \
     is absent from the buffer.",
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

    fn render(&self, frame: &mut Frame, area: Rect, _ctx: &AppContext) {
        // Help renders as an overlay on top of whatever is below
        let popup_area = centered_rect(area, POPUP_WIDTH_PCT, POPUP_HEIGHT_PCT);
        frame.render_widget(Clear, popup_area);

        let help_text = help_lines();
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
        help_lines()
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
            ("PgUp / PgDn", "Scroll the output pane"),
            ("f", "Toggle following the live tail"),
            ("G", "Jump to the output tail and follow again"),
            ("i", "Inject a message into the live run"),
            ("s", "Start a run: command, then optional goal"),
            ("x", "Stop the live run (asks first)"),
            ("o", "Toggle driver opt-in for this project"),
        ] {
            let expected = row(key, description).spans[0].content.to_string();
            assert!(
                text.lines().any(|line| line == expected),
                "the help screen is the ONLY place keys are documented, and \
                 the row {expected:?} is missing from it:\n{text}"
            );
        }
        // The dashboard sort toggle and the two needs-a-human filter forms.
        assert!(
            text.contains("Toggle sort"),
            "the dashboard sort toggle must be documented:\n{text}"
        );
        assert!(
            text.contains("/term/h") && text.contains("//h"),
            "both needs-a-human filter forms must be documented:\n{text}"
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
    fn the_injection_legend_names_all_four_states_and_carries_the_honest_gloss() {
        let text = body();
        let flow = flowed();

        for (glyph, label) in [
            (GLYPH_QUEUED, LABEL_QUEUED),
            (GLYPH_DELIVERED, LABEL_DELIVERED),
            (GLYPH_ACTED_ON, LABEL_ACTED_ON),
            (GLYPH_MISSED, LABEL_MISSED),
        ] {
            assert!(
                text.contains(glyph),
                "the injection legend must name the glyph {glyph:?}:\n{text}"
            );
            assert!(
                text.contains(label),
                "the injection legend must name the label {label:?}:\n{text}"
            );
        }

        assert!(
            flow.contains(
                "written to the agent's stdin; it may be a minute before the agent picks it up"
            ),
            "`delivered` must state honestly that the pickup may be about a \
             minute away — the echo arrives at DEQUEUE, measured roughly \
             fifty-five seconds after the write:\n{flow}"
        );
        assert!(
            flow.contains("the agent dequeued it and is running it as its own turn"),
            "`acted-on` must say DEQUEUED, which is the only observation the \
             protocol actually makes:\n{flow}"
        );
        assert!(
            flow.contains("undeliverable and never retried; the row under the message says why"),
            "`missed` must not pin itself to ONE cause (CR-03): a failed stdin \
             write on a run that is still live is missed too, and the pane \
             prints a different gloss for it:\n{flow}"
        );
        assert!(
            !flow.contains("the run closed its input"),
            "and the legend must not name the after-close cause as if it were \
             the only one:\n{flow}"
        );
    }

    #[test]
    fn the_help_screen_never_says_sent_received_read_or_acknowledged() {
        let text = flowed().to_lowercase();
        // Three words that would each promise an observation the protocol
        // cannot make: the echo arrives at DEQUEUE, roughly fifty-five seconds
        // after the stdin write.
        for forbidden in ["sent", "received", "acknowledged"] {
            assert!(
                !text.contains(forbidden),
                "the word {forbidden:?} is forbidden for an injected message \
                 (D-07, D-10):\n{text}"
            );
        }
        // `read` is allowed exactly once, in the queued gloss, where it says
        // that nothing has read the message — the opposite claim.
        assert_eq!(
            text.matches("read").count(),
            1,
            "`read` may appear only in the queued gloss:\n{text}"
        );
        assert!(
            text.contains("nothing has read it yet"),
            "and that one occurrence must be the queued gloss:\n{text}"
        );
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

    #[test]
    fn the_body_is_long_enough_that_the_scroll_work_was_required() {
        // The premise of the whole change, pinned: at an 80x24 terminal the
        // grown popup's inner height is 24*90/100 - 2 = 19 rows, and the body is
        // far longer than that. If this ever stops being true the scrolling is
        // still correct, but the claim in the module doc would not be.
        let total = help_lines().len();
        assert!(
            total > 19,
            "the help body is {total} rows; the 80x24 popup shows 19, so the \
             popup must scroll"
        );
    }
}
