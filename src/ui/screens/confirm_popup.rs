//! The one modal popup every destructive y/n confirmation goes through.
//!
//! Replaces the per-screen footer line (`... [y/n]`), which looked identical to
//! routine status chatter and, on the project-delete screen, sat under an empty
//! bordered block that had wiped the dashboard.
//!
//! Contract (todo 2026-08-18; INFERRED decisions are flagged in the quick
//! summary for audit):
//! - centred bordered popup over a DIMMED but still rendered background — the
//!   caller paints the background, then calls [`dim_background`], then
//!   [`render_confirm_popup`];
//! - two buttons, `[ Yes ]` `[ No ]`, with **No focused by default** so a stray
//!   Enter is safe;
//! - `y` / `n` / `Esc` keep working as accelerators; Left/Right/Tab/BackTab (and
//!   `h`/`l`) move focus; Enter activates the focused button;
//! - a body the area cannot show in full is **clipped** (UI D5 `[inferred — audit]`):
//!   [`render_confirm_popup`] reports it in [`PopupLayout::clipped`], the popup's hint
//!   row then reads [`CLIPPED_HINT`], and until the next render says otherwise `y`,
//!   Enter on Yes and a click on Yes do nothing. `n`, `Esc` and a click on No still
//!   cancel. A decision whose text is cut off is never confirmable.
//!
//! This module draws only text its caller hands it. Callers pass prompt bodies
//! that are already escaped through `crate::text::render_for_terminal`, so the
//! render-escape census still holds: nothing is re-derived from raw input here.

use crate::ui::mouse::MouseInput;
use crossterm::event::KeyCode;
use ratatui::layout::{Alignment, Position, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;
use std::cell::Cell;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ConfirmFocus {
    Yes,
    /// The safe default.
    #[default]
    No,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConfirmOutcome {
    Confirm,
    Cancel,
    /// Focus moved, or the key meant nothing.
    Pending,
}

/// Where the two buttons were drawn, recorded at render time (D-04).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConfirmButtons {
    pub yes: Rect,
    pub no: Rect,
}

/// What [`render_confirm_popup`] drew: where the buttons landed (`None` when the popup
/// is too short to draw them) and whether the body was cut off.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PopupLayout {
    pub buttons: Option<ConfirmButtons>,
    pub clipped: bool,
}

/// The bottom row of a clipped popup, in place of the key hint.
pub const CLIPPED_HINT: &str = "Cut off: enlarge the terminal to confirm \u{b7} Esc cancel";

#[derive(Debug, Clone, Default)]
pub struct ConfirmPopup {
    pub focus: ConfirmFocus,
    /// Written by `render` (which takes `&self`), read by `on_mouse`.
    buttons: Cell<Option<ConfirmButtons>>,
    /// Written by `render`, read by `on_key` / `on_mouse`.
    clipped: Cell<bool>,
}

impl ConfirmPopup {
    /// A left click on a button is that button's outcome; anything else —
    /// a click elsewhere, a wheel step — is inert. Deliberate reversal of the
    /// old "a click never confirms a dialog" default (INFERRED, requested): a
    /// press must land on a labelled button, and the stray-Enter safety is
    /// untouched.
    pub fn on_mouse(&self, input: MouseInput) -> ConfirmOutcome {
        let (MouseInput::Click { column, row, .. }, Some(b)) = (input, self.buttons.get()) else {
            return ConfirmOutcome::Pending;
        };
        let at = Position::new(column, row);
        if b.yes.contains(at) {
            // A cut-off body is not a decision the user has read.
            self.confirm_unless_clipped()
        } else if b.no.contains(at) {
            ConfirmOutcome::Cancel
        } else {
            ConfirmOutcome::Pending
        }
    }

    /// Remember what `render_confirm_popup` drew.
    pub fn record(&self, layout: PopupLayout) {
        self.buttons.set(layout.buttons);
        self.clipped.set(layout.clipped);
    }

    /// Whether the last render cut the body off.
    pub fn clipped(&self) -> bool {
        self.clipped.get()
    }

    /// `Confirm`, unless the body was cut off: then nothing happens until the terminal
    /// is enlarged (UI D5). Every way of saying yes goes through here.
    fn confirm_unless_clipped(&self) -> ConfirmOutcome {
        if self.clipped.get() {
            ConfirmOutcome::Pending
        } else {
            ConfirmOutcome::Confirm
        }
    }

    pub fn on_key(&mut self, code: KeyCode) -> ConfirmOutcome {
        match code {
            KeyCode::Char('y') | KeyCode::Char('Y') => self.confirm_unless_clipped(),
            KeyCode::Char('n') | KeyCode::Char('N') | KeyCode::Esc => ConfirmOutcome::Cancel,
            KeyCode::Enter => match self.focus {
                ConfirmFocus::Yes => self.confirm_unless_clipped(),
                ConfirmFocus::No => ConfirmOutcome::Cancel,
            },
            KeyCode::Left
            | KeyCode::Right
            | KeyCode::Tab
            | KeyCode::BackTab
            | KeyCode::Char('h')
            | KeyCode::Char('l') => {
                self.focus = match self.focus {
                    ConfirmFocus::Yes => ConfirmFocus::No,
                    ConfirmFocus::No => ConfirmFocus::Yes,
                };
                ConfirmOutcome::Pending
            }
            _ => ConfirmOutcome::Pending,
        }
    }
}

/// A click on a button, as the accelerator key a screen already handles
/// (`y` / `n`), so click and key share one code path. `None` for any other
/// input.
pub fn click_as_key(popup: &ConfirmPopup, input: MouseInput) -> Option<KeyCode> {
    match popup.on_mouse(input) {
        ConfirmOutcome::Confirm => Some(KeyCode::Char('y')),
        ConfirmOutcome::Cancel => Some(KeyCode::Char('n')),
        ConfirmOutcome::Pending => None,
    }
}

/// Drop the legacy footer key hint so the popup body does not repeat the
/// buttons. The prompt builders keep the suffix (tests pin it).
pub fn strip_key_hint(prompt: &str) -> &str {
    prompt.trim_end().trim_end_matches("[y/n]").trim()
}

/// Dim everything already painted in `area`.
pub fn dim_background(frame: &mut Frame, area: Rect) {
    frame
        .buffer_mut()
        .set_style(area, Style::default().add_modifier(Modifier::DIM));
}

fn popup_width(area: Rect) -> u16 {
    area.width.saturating_sub(4).clamp(1, 72).min(area.width)
}

/// The popup rectangle for a body of `body_rows` wrapped rows.
fn popup_rect(area: Rect, body_rows: u16) -> Rect {
    let width = popup_width(area);
    // borders (2) + body + spacer (1) + buttons (1) + hint (1) - spacer shared
    let height = body_rows.saturating_add(5).min(area.height);
    Rect::new(
        area.x + (area.width - width) / 2,
        area.y + (area.height - height) / 2,
        width,
        height,
    )
}

/// The rows `lines` take once word-wrapped to `inner_width` cells.
///
/// This asks ratatui's own wrapper (`Paragraph::line_count` with
/// `Wrap { trim: false }`) rather than dividing a character count by the width: word
/// wrap moves a whole word to the next row, so the count is routinely larger than
/// `ceil(chars / width)`, and a popup sized by the smaller number lost its last words
/// (the polite-stop prompt's "Nothing is killed.", D-22). It is the same wrap
/// [`render_confirm_popup`] draws the body with, so the two cannot disagree.
pub fn wrapped_rows(lines: &[Line<'_>], inner_width: u16) -> u16 {
    let rows = Paragraph::new(lines.to_vec())
        .wrap(Wrap { trim: false })
        .line_count(inner_width.max(1));
    u16::try_from(rows).unwrap_or(u16::MAX)
}

fn button(label: &'static str, focused: bool, accent: Color) -> Span<'static> {
    let style = if focused {
        Style::default()
            .fg(accent)
            .add_modifier(Modifier::REVERSED | Modifier::BOLD)
    } else {
        Style::default().fg(Color::Gray)
    };
    Span::styled(label, style)
}

const YES_LABEL: &str = "[ Yes ]";
const NO_LABEL: &str = "[ No ]";
const BUTTON_GAP: u16 = 3;

/// The button rects `Alignment::Center` gives the buttons line in `row`.
fn button_rects(row: Rect) -> ConfirmButtons {
    let (yw, nw) = (YES_LABEL.len() as u16, NO_LABEL.len() as u16);
    let total = yw + BUTTON_GAP + nw;
    let x0 = row.x + row.width.saturating_sub(total) / 2;
    let clip = |x: u16, w: u16| Rect::new(x, row.y, w, 1).intersection(row);
    ConfirmButtons {
        yes: clip(x0, yw),
        no: clip(x0 + yw + BUTTON_GAP, nw),
    }
}

/// Draw the popup. `title` is static text; `body` lines are pre-escaped.
///
/// Returns where the buttons landed (`None` when the popup is too short to
/// draw them) and whether the body was cut off, so the caller can hand it to
/// [`ConfirmPopup::record`].
pub fn render_confirm_popup(
    frame: &mut Frame,
    area: Rect,
    title: &str,
    body: Vec<Line<'static>>,
    accent: Color,
    focus: ConfirmFocus,
) -> PopupLayout {
    let inner_w = popup_width(area).saturating_sub(2);
    let body_rows = wrapped_rows(&body, inner_w);
    let popup = popup_rect(area, body_rows);
    frame.render_widget(Clear, popup);
    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(accent))
        .title(Span::styled(
            format!(" {title} "),
            Style::default().fg(accent).add_modifier(Modifier::BOLD),
        ));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);
    if inner.height == 0 {
        // Not even a row to draw in: certainly cut off.
        return PopupLayout {
            buttons: None,
            clipped: true,
        };
    }

    // Bottom three inner rows: buttons, spacer-free hint. Body gets the rest.
    let body_h = inner.height.saturating_sub(3);
    frame.render_widget(
        Paragraph::new(body).wrap(Wrap { trim: false }),
        Rect::new(inner.x, inner.y, inner.width, body_h),
    );

    let mut rects = None;
    if inner.height >= 2 {
        let buttons = Line::from(vec![
            button(YES_LABEL, focus == ConfirmFocus::Yes, accent),
            Span::raw(" ".repeat(BUTTON_GAP as usize)),
            button(NO_LABEL, focus == ConfirmFocus::No, accent),
        ]);
        let row = Rect::new(inner.x, inner.y + inner.height - 2, inner.width, 1);
        frame.render_widget(Paragraph::new(buttons).alignment(Alignment::Center), row);
        rects = Some(button_rects(row));
    }
    // The body is clipped when it needs more rows than the popup has, or when the popup
    // has no room for its buttons at all. A clipped popup cannot be confirmed, and says
    // so where the key hint would be.
    let clipped = body_rows > body_h || rects.is_none();
    let hint = if clipped {
        Span::styled(CLIPPED_HINT, Style::default().fg(Color::Yellow))
    } else {
        Span::styled(
            "y/n \u{b7} \u{2190}/\u{2192} move \u{b7} Enter select \u{b7} Esc cancel",
            Style::default().fg(Color::DarkGray),
        )
    };
    frame.render_widget(
        Paragraph::new(Line::from(hint)).alignment(Alignment::Center),
        Rect::new(inner.x, inner.y + inner.height - 1, inner.width, 1),
    );
    PopupLayout {
        buttons: rects,
        clipped,
    }
}

/// `(inner width, most body rows)` a popup drawn in `area` can show.
pub fn body_room(area: Rect) -> (u16, u16) {
    (
        popup_width(area).saturating_sub(2),
        area.height.saturating_sub(5),
    )
}

/// The longest prefix of `items` whose wrapped rows, plus one muted `  … {k} more` row for
/// the items left out, fit `rows`. When everything fits, `items` is returned unchanged.
/// When not even the `… {k} more` row fits, just that row is returned, so the caller's
/// popup is taller than `rows` and clips (and is then not confirmable).
pub fn fit_list(items: Vec<Line<'static>>, rows: u16, inner_width: u16) -> Vec<Line<'static>> {
    let costs: Vec<usize> = items
        .iter()
        .map(|item| usize::from(wrapped_rows(std::slice::from_ref(item), inner_width)))
        .collect();
    let budget = usize::from(rows);
    if costs.iter().sum::<usize>() <= budget {
        return items;
    }
    let more = |left: usize| {
        Line::from(Span::styled(
            format!("  \u{2026} {left} more"),
            Style::default().fg(Color::DarkGray),
        ))
    };
    let mut kept = 0usize;
    let mut used = 0usize;
    for cost in &costs {
        let next = used + cost;
        let left = costs.len() - (kept + 1);
        let more_cost = if left > 0 {
            usize::from(wrapped_rows(&[more(left)], inner_width))
        } else {
            0
        };
        if next + more_cost > budget {
            break;
        }
        used = next;
        kept += 1;
    }
    let left = costs.len() - kept;
    let mut out: Vec<Line<'static>> = items.into_iter().take(kept).collect();
    out.push(more(left));
    out
}

/// Column and row of the first `needle` in newline-joined screen `text`.
#[cfg(test)]
pub(crate) fn locate(text: &str, needle: &str) -> Option<(u16, u16)> {
    text.lines().enumerate().find_map(|(y, line)| {
        let chars: Vec<char> = line.chars().collect();
        let n: Vec<char> = needle.chars().collect();
        chars
            .windows(n.len())
            .position(|w| w == n.as_slice())
            .map(|x| (x as u16, y as u16))
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn rows_of(t: &Terminal<TestBackend>, w: u16, h: u16) -> Vec<String> {
        let buf = t.backend().buffer();
        (0..h)
            .map(|y| (0..w).map(|x| buf[(x, y)].symbol().to_string()).collect())
            .collect()
    }

    fn draw(focus: ConfirmFocus, w: u16, h: u16) -> Vec<String> {
        let mut t = Terminal::new(TestBackend::new(w, h)).unwrap();
        t.draw(|f| {
            // A recognisable background that the popup must dim, not erase.
            let bg = Paragraph::new("BACKGROUND-ROW\n".repeat(h as usize));
            f.render_widget(bg, f.area());
            dim_background(f, f.area());
            render_confirm_popup(
                f,
                f.area(),
                "Remove project",
                vec![Line::from("Remove \"demo\"? Files are not deleted.")],
                Color::Red,
                focus,
            );
        })
        .unwrap();
        rows_of(&t, w, h)
    }

    #[test]
    fn popup_shows_title_body_and_both_buttons_over_a_retained_background() {
        let rows = draw(ConfirmFocus::No, 80, 24);
        let text = rows.join("\n");
        assert!(text.contains("Remove project"));
        assert!(text.contains("Remove \"demo\"? Files are not deleted."));
        assert!(text.contains("[ Yes ]") && text.contains("[ No ]"));
        assert!(rows[0].contains("BACKGROUND-ROW"), "background erased");
    }

    #[test]
    fn popup_is_centred() {
        let rows = draw(ConfirmFocus::No, 80, 24);
        let first = rows.iter().position(|r| r.contains('\u{250c}')).unwrap();
        let last = rows.iter().rposition(|r| r.contains('\u{2514}')).unwrap();
        let mid = (first + last) as i32 / 2;
        assert!((mid - 11).abs() <= 1, "vertically centred, got {mid}");
        let col = rows[first].find('\u{250c}').unwrap();
        assert!(col > 0, "horizontally inset");
    }

    #[test]
    fn focus_is_marked_by_reverse_video_on_exactly_the_focused_button() {
        for focus in [ConfirmFocus::Yes, ConfirmFocus::No] {
            let mut t = Terminal::new(TestBackend::new(80, 24)).unwrap();
            t.draw(|f| {
                let _ = render_confirm_popup(
                    f,
                    f.area(),
                    "T",
                    vec![Line::from("x")],
                    Color::Red,
                    focus,
                );
            })
            .unwrap();
            let rows = rows_of(&t, 80, 24);
            let buf = t.backend().buffer();
            let (mut yes_rev, mut no_rev) = (false, false);
            for (y, row) in rows.iter().enumerate() {
                // Rows are ASCII-or-single-cell symbols, so byte offset == column
                // only up to the first multi-byte border glyph; search by cells.
                let cells: Vec<&str> = (0..80).map(|x| buf[(x, y as u16)].symbol()).collect();
                let joined = cells.concat();
                if joined.contains("[ Yes ]") {
                    let c = cells.iter().position(|s| *s == "[").unwrap();
                    yes_rev = buf[(c as u16, y as u16)]
                        .modifier
                        .contains(Modifier::REVERSED);
                    let c2 = cells.iter().rposition(|s| *s == "[").unwrap();
                    no_rev = buf[(c2 as u16, y as u16)]
                        .modifier
                        .contains(Modifier::REVERSED);
                }
                let _ = row;
            }
            assert_eq!(yes_rev, focus == ConfirmFocus::Yes);
            assert_eq!(no_rev, focus == ConfirmFocus::No);
        }
    }

    #[test]
    fn wrapped_rows_counts_the_rows_word_wrap_draws() {
        // Word wrap moves `bbbbbbb` whole onto the next row; a char count says 2.
        assert_eq!(wrapped_rows(&[Line::from("aaaaaaa bbbbbbb ccc")], 10), 3);
        let bodies: [Vec<Line<'static>>; 3] = [
            vec![Line::from(
                "Set control: stop for 20261019-0042-abcd? The driver dispatches nothing new, records the in-flight report, then ends. Nothing is killed.",
            )],
            vec![
                Line::from("driver check found 3 problem(s) in your edit:"),
                Line::from("  DIRECTION.md:7: the goal is still a placeholder"),
                Line::from(""),
                Line::from("Apply it anyway?"),
            ],
            vec![Line::from(
                "supercalifragilisticexpialidocious pneumonoultramicroscopicsilicovolcanoconiosis x",
            )],
        ];
        for body in &bodies {
            for width in 10..=80u16 {
                let expected = Paragraph::new(body.clone())
                    .wrap(Wrap { trim: false })
                    .line_count(width);
                assert_eq!(
                    usize::from(wrapped_rows(body, width)),
                    expected,
                    "width {width}"
                );
            }
        }
    }

    fn draw_popup(popup: &ConfirmPopup, body: Vec<Line<'static>>, w: u16, h: u16) -> Vec<String> {
        let mut t = Terminal::new(TestBackend::new(w, h)).unwrap();
        t.draw(|f| {
            popup.record(render_confirm_popup(
                f,
                f.area(),
                "T",
                body.clone(),
                Color::Red,
                popup.focus,
            ));
        })
        .unwrap();
        rows_of(&t, w, h)
    }

    fn six_lines() -> Vec<Line<'static>> {
        (1..=6).map(|n| Line::from(format!("line {n}"))).collect()
    }

    #[test]
    fn a_clipped_body_disarms_yes_and_says_so() {
        let mut popup = ConfirmPopup::default();
        // Nothing drawn yet: never clipped, so key-only tests keep working.
        assert!(!popup.clipped());
        draw_popup(&popup, six_lines(), 40, 6);
        assert!(popup.clipped(), "six lines cannot fit a 40x6 area");
        let b = popup.buttons.get().expect("buttons are still drawn");
        assert_eq!(popup.on_key(KeyCode::Char('y')), ConfirmOutcome::Pending);
        assert_eq!(popup.on_key(KeyCode::Char('Y')), ConfirmOutcome::Pending);
        popup.focus = ConfirmFocus::Yes;
        assert_eq!(popup.on_key(KeyCode::Enter), ConfirmOutcome::Pending);
        assert_eq!(
            popup.on_mouse(click(b.yes.x, b.yes.y)),
            ConfirmOutcome::Pending
        );
        assert_eq!(click_as_key(&popup, click(b.yes.x, b.yes.y)), None);
        // Declining still works from every route.
        assert_eq!(popup.on_key(KeyCode::Char('n')), ConfirmOutcome::Cancel);
        assert_eq!(popup.on_key(KeyCode::Char('N')), ConfirmOutcome::Cancel);
        assert_eq!(popup.on_key(KeyCode::Esc), ConfirmOutcome::Cancel);
        assert_eq!(
            popup.on_mouse(click(b.no.x, b.no.y)),
            ConfirmOutcome::Cancel
        );
        // Focus still moves, and Enter on No still cancels.
        assert_eq!(popup.on_key(KeyCode::Tab), ConfirmOutcome::Pending);
        assert_eq!(popup.focus, ConfirmFocus::No);
        assert_eq!(popup.on_key(KeyCode::Enter), ConfirmOutcome::Cancel);

        // The bottom row says why Yes does nothing (a 60-wide area fits the sentence).
        let rows = draw_popup(&popup, six_lines(), 60, 6);
        let hint = &rows[4];
        assert!(
            hint.contains("Cut off: enlarge the terminal to confirm"),
            "{rows:#?}"
        );
        assert!(!hint.contains("y/n"), "{hint}");
        assert!(CLIPPED_HINT.starts_with("Cut off: enlarge the terminal to confirm"));
    }

    #[test]
    fn a_body_that_fits_keeps_yes_armed() {
        let mut popup = ConfirmPopup::default();
        let rows = draw_popup(&popup, vec![Line::from("Remove it?")], 80, 24);
        assert!(!popup.clipped());
        assert!(rows.join("\n").contains("y/n"), "the key hint is kept");
        assert!(!rows.join("\n").contains("Cut off"));
        let b = popup.buttons.get().expect("buttons");
        assert_eq!(popup.on_key(KeyCode::Char('y')), ConfirmOutcome::Confirm);
        assert_eq!(
            popup.on_mouse(click(b.yes.x, b.yes.y)),
            ConfirmOutcome::Confirm
        );
        popup.focus = ConfirmFocus::Yes;
        assert_eq!(popup.on_key(KeyCode::Enter), ConfirmOutcome::Confirm);
        // A never-rendered popup is not clipped.
        let mut fresh = ConfirmPopup::default();
        assert_eq!(fresh.on_key(KeyCode::Char('y')), ConfirmOutcome::Confirm);
    }

    fn text_of(line: &Line<'_>) -> String {
        line.spans.iter().map(|s| s.content.as_ref()).collect()
    }

    #[test]
    fn fit_list_keeps_the_rows_that_fit_and_counts_the_rest() {
        let items =
            || -> Vec<Line<'static>> { (1..=6).map(|n| Line::from(format!("item {n}"))).collect() };
        let texts = |lines: &[Line<'_>]| -> Vec<String> { lines.iter().map(text_of).collect() };
        let cut = fit_list(items(), 4, 40);
        assert_eq!(
            texts(&cut),
            ["item 1", "item 2", "item 3", "  \u{2026} 3 more"]
        );
        let none = fit_list(items(), 0, 40);
        assert_eq!(texts(&none), ["  \u{2026} 6 more"]);
        for rows in [6, 7, 40] {
            assert_eq!(texts(&fit_list(items(), rows, 40)).len(), 6, "rows {rows}");
        }
        assert_eq!(
            texts(&fit_list(items(), 5, 40)).last().unwrap(),
            "  \u{2026} 2 more"
        );
        // A wrapped item costs its wrapped rows.
        let wide = vec![
            Line::from("aaaa bbbb cccc dddd"),
            Line::from("x"),
            Line::from("y"),
        ];
        assert_eq!(
            texts(&fit_list(wide, 3, 16)),
            ["aaaa bbbb cccc dddd", "  \u{2026} 2 more"]
        );
    }

    #[test]
    fn tiny_terminals_do_not_panic() {
        for (w, h) in [(1, 1), (10, 3), (20, 5), (30, 8)] {
            let _ = draw(ConfirmFocus::No, w, h);
        }
    }

    #[test]
    fn keys_default_to_no_and_accelerators_work() {
        let mut p = ConfirmPopup::default();
        assert_eq!(p.focus, ConfirmFocus::No);
        assert_eq!(p.on_key(KeyCode::Enter), ConfirmOutcome::Cancel);
        assert_eq!(p.on_key(KeyCode::Char('y')), ConfirmOutcome::Confirm);
        assert_eq!(p.on_key(KeyCode::Char('n')), ConfirmOutcome::Cancel);
        assert_eq!(p.on_key(KeyCode::Esc), ConfirmOutcome::Cancel);
        assert_eq!(p.on_key(KeyCode::Char('z')), ConfirmOutcome::Pending);
        assert_eq!(p.on_key(KeyCode::Tab), ConfirmOutcome::Pending);
        assert_eq!(p.focus, ConfirmFocus::Yes);
        assert_eq!(p.on_key(KeyCode::Enter), ConfirmOutcome::Confirm);
        assert_eq!(p.on_key(KeyCode::Left), ConfirmOutcome::Pending);
        assert_eq!(p.focus, ConfirmFocus::No);
    }

    #[test]
    fn strip_key_hint_removes_only_the_suffix() {
        assert_eq!(strip_key_hint("  Remove \"a\"? [y/n]"), "Remove \"a\"?");
        assert_eq!(strip_key_hint("no hint"), "no hint");
    }

    fn click(column: u16, row: u16) -> MouseInput {
        MouseInput::Click {
            column,
            row,
            double: false,
        }
    }

    #[test]
    fn recorded_button_rects_sit_exactly_on_the_drawn_labels() {
        for (w, h) in [(80, 24), (40, 12), (110, 40)] {
            let mut t = Terminal::new(TestBackend::new(w, h)).unwrap();
            let mut got = PopupLayout::default();
            t.draw(|f| {
                got = render_confirm_popup(
                    f,
                    f.area(),
                    "T",
                    vec![Line::from("x")],
                    Color::Red,
                    ConfirmFocus::No,
                );
            })
            .unwrap();
            let b = got.buttons.expect("buttons drawn");
            let text = rows_of(&t, w, h).join("\n");
            let (yx, yy) = locate(&text, "[ Yes ]").unwrap();
            let (nx, ny) = locate(&text, "[ No ]").unwrap();
            assert_eq!(b.yes, Rect::new(yx, yy, 7, 1), "{w}x{h}");
            assert_eq!(b.no, Rect::new(nx, ny, 6, 1), "{w}x{h}");
        }
    }

    #[test]
    fn clicks_map_to_buttons_and_everything_else_is_inert() {
        let mut t = Terminal::new(TestBackend::new(80, 24)).unwrap();
        let popup = ConfirmPopup::default();
        // Nothing recorded yet: inert.
        assert_eq!(popup.on_mouse(click(40, 12)), ConfirmOutcome::Pending);
        t.draw(|f| {
            popup.record(render_confirm_popup(
                f,
                f.area(),
                "T",
                vec![Line::from("x")],
                Color::Red,
                popup.focus,
            ));
        })
        .unwrap();
        let b = popup.buttons.get().unwrap();
        let on = |r: Rect| click(r.x + r.width - 1, r.y);
        assert_eq!(
            popup.on_mouse(click(b.yes.x, b.yes.y)),
            ConfirmOutcome::Confirm
        );
        assert_eq!(popup.on_mouse(on(b.yes)), ConfirmOutcome::Confirm);
        assert_eq!(
            popup.on_mouse(click(b.no.x, b.no.y)),
            ConfirmOutcome::Cancel
        );
        assert_eq!(popup.on_mouse(on(b.no)), ConfirmOutcome::Cancel);
        // The gap between the buttons, the row above, and a far corner.
        assert_eq!(
            popup.on_mouse(click(b.yes.right(), b.yes.y)),
            ConfirmOutcome::Pending
        );
        assert_eq!(
            popup.on_mouse(click(b.yes.x, b.yes.y - 1)),
            ConfirmOutcome::Pending
        );
        assert_eq!(popup.on_mouse(click(0, 0)), ConfirmOutcome::Pending);
        // A wheel step never activates.
        let wheel = MouseInput::Wheel {
            column: b.yes.x,
            row: b.yes.y,
            down: true,
        };
        assert_eq!(popup.on_mouse(wheel), ConfirmOutcome::Pending);
        assert_eq!(
            click_as_key(&popup, click(b.yes.x, b.yes.y)),
            Some(KeyCode::Char('y'))
        );
        assert_eq!(
            click_as_key(&popup, click(b.no.x, b.no.y)),
            Some(KeyCode::Char('n'))
        );
        assert_eq!(click_as_key(&popup, click(0, 0)), None);
    }
}
