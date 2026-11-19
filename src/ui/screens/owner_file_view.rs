//! The read-only file viewer behind `o` (37-05, D-08).
//!
//! `o` on an inbox row or an Owner block row shows the group's source file inside
//! the app, from the top. Enter on a check or a `file PATH` line opens it at a
//! line instead (quick 261118-u4e, [`OwnerFileViewScreen::at_line`]): the target
//! line carries a gutter marker and REVERSED+BOLD, and sits two rows below the
//! top. It never starts `$EDITOR` or any other program (T-37-16): this screen
//! hands no file to an editor and nothing is spawned. The file is read
//! once, on the key press, by [`super::owner_actions::read_owner_file`] (contained
//! to the project's `.planning/`, at most 1 MiB); this screen only holds the
//! lines computed at push time, so `render` does no I/O (OWN-12).

use super::detail::{clamp_scroll, shown, ViewportMetrics};
use super::owner_block::cells;
use super::{AppContext, Screen, ScreenAction};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;
use std::cell::Cell;

/// The title: the alias and the project-relative path, both escaped.
fn title_for(alias: &str, path: &str) -> String {
    format!(" {} \u{b7} {} ", shown(alias), shown(path))
}

/// The most lines the viewer holds: the scroll offset and `Paragraph::scroll` are
/// `u16`, so a line past this could never be reached (WR-06).
const MAX_LINES: usize = u16::MAX as usize;

/// The key hint for a footer `width` cells wide that also holds `position_w`
/// cells of scroll position: the longest hint that fits.
fn footer_keys(width: usize, position_w: usize) -> String {
    let room = width.saturating_sub(position_w + 2);
    for hint in [
        "[Esc]back  [j/k]scroll  [PgUp/PgDn]page  [Home/End]",
        "[Esc]back  [j/k]scroll  [PgUp/PgDn]page",
        "[Esc]back  [j/k]scroll",
        "[Esc]back",
    ] {
        if cells(hint) <= room {
            return hint.to_owned();
        }
    }
    String::new()
}

/// `line` cut into rows of at most `width` cells, breaking at the last space in
/// the row when there is one and at the cell boundary otherwise, each span's
/// style kept. An empty line stays one empty row.
fn wrap_line(line: &Line<'static>, width: usize) -> Vec<Line<'static>> {
    let width = width.max(1);
    let chars: Vec<(char, Style)> = line
        .spans
        .iter()
        .flat_map(|span| span.content.chars().map(move |c| (c, span.style)))
        .collect();
    let cw = |c: char| cells(&c.to_string());
    let total: usize = chars.iter().map(|(c, _)| cw(*c)).sum();
    let regroup = |part: &[(char, Style)]| -> Line<'static> {
        let mut spans: Vec<Span<'static>> = Vec::new();
        let mut text = String::new();
        let mut style = None;
        for (c, st) in part {
            if style.is_some_and(|s| s != *st) {
                spans.push(Span::styled(
                    std::mem::take(&mut text),
                    style.unwrap_or_default(),
                ));
            }
            style = Some(*st);
            text.push(*c);
        }
        if !text.is_empty() {
            spans.push(Span::styled(text, style.unwrap_or_default()));
        }
        Line::from(spans)
    };
    if total <= width {
        return vec![line.clone()];
    }
    let mut out = Vec::new();
    let mut start = 0;
    while start < chars.len() {
        let mut used = 0;
        let mut end = start;
        let mut last_space = None;
        while end < chars.len() && used + cw(chars[end].0) <= width {
            if chars[end].0 == ' ' && end > start {
                last_space = Some(end);
            }
            used += cw(chars[end].0);
            end += 1;
        }
        if end == chars.len() {
            out.push(regroup(&chars[start..end]));
            break;
        }
        if end == start {
            // A glyph wider than the row: take it alone so the loop advances.
            end = start + 1;
            out.push(regroup(&chars[start..end]));
            start = end;
            continue;
        }
        match last_space {
            Some(space) => {
                out.push(regroup(&chars[start..space]));
                start = space + 1;
            }
            None => {
                out.push(regroup(&chars[start..end]));
                start = end;
            }
        }
    }
    out
}

/// The rows between the top of the viewer and a freshly opened target line.
const TARGET_ROWS_FROM_TOP: usize = 2;

/// Cells the target gutter takes: the marker and a space.
const GUTTER_CELLS: u16 = 2;

/// `row` with the target gutter in front: the marker and a space on the target's
/// first row, two blanks elsewhere. The target's rows are REVERSED+BOLD, never
/// color alone. The marker is a registry glyph, never file text (T-u4e-04).
fn with_gutter(row: Line<'static>, is_target: bool, first: bool) -> Line<'static> {
    let mark = Style::default().add_modifier(Modifier::REVERSED | Modifier::BOLD);
    let gutter = if is_target && first {
        format!("{} ", crate::ui::glyphs::POINTER_RIGHT)
    } else {
        " ".repeat(usize::from(GUTTER_CELLS))
    };
    let mut spans = vec![Span::raw(gutter)];
    spans.extend(row.spans);
    if is_target {
        for span in &mut spans {
            span.style = span.style.patch(mark);
        }
    }
    Line::from(spans)
}

pub struct OwnerFileViewScreen {
    title: String,
    /// The file as lines, escaped once at push time.
    lines: Vec<Line<'static>>,
    /// The last line is the "more lines not shown" notice, not file text.
    notice: bool,
    /// Written by `render` (the opening jump depends on the wrapped width) and by
    /// the keys, so a `Cell`.
    scroll_offset: Cell<u16>,
    /// The line [`at_line`](Self::at_line) marks, an index into `lines`.
    target: Option<usize>,
    /// The first `render` still has to scroll to the target.
    jump_pending: Cell<bool>,
    /// Written by `render`, read by the key handler (the help.rs idiom).
    viewport: Cell<ViewportMetrics>,
}

impl OwnerFileViewScreen {
    /// A viewer over `text`: markdown files through the Docs renderer, anything
    /// else (`HANDOFF.json`) as one escaped line per text line.
    pub fn new(alias: &str, path: &str, text: &str) -> Self {
        let mut lines = if path.to_ascii_lowercase().ends_with(".md") {
            crate::archive::render_markdown_lines(text)
        } else {
            text.lines()
                .map(|line| Line::from(Span::raw(shown(line))))
                .collect()
        };
        let notice = lines.len() > MAX_LINES;
        if notice {
            // Say so instead of letting a long file look complete.
            let hidden = lines.len() - (MAX_LINES - 1);
            lines.truncate(MAX_LINES - 1);
            lines.push(Line::from(Span::styled(
                format!("\u{2026} {hidden} more lines not shown"),
                Style::default().add_modifier(Modifier::DIM),
            )));
        }
        Self {
            title: title_for(alias, path),
            lines,
            notice,
            scroll_offset: Cell::new(0),
            target: None,
            jump_pending: Cell::new(false),
            viewport: Cell::new(ViewportMetrics::default()),
        }
    }

    /// Open at `line` (a zero-based index into the file's `str::lines`): the line
    /// gets a gutter marker plus REVERSED+BOLD, and the first render scrolls it to
    /// two rows below the top. A line past the end, or the truncation notice, is
    /// dropped: the viewer then opens exactly as [`new`](Self::new) does.
    pub fn at_line(mut self, line: Option<usize>) -> Self {
        let held = self.lines.len() - usize::from(self.notice);
        self.target = line.filter(|at| *at < held);
        self.jump_pending.set(self.target.is_some());
        self
    }

    #[cfg(test)]
    pub(super) fn scroll_offset(&self) -> u16 {
        self.scroll_offset.get()
    }

    #[cfg(test)]
    pub(super) fn title(&self) -> &str {
        &self.title
    }

    #[cfg(test)]
    pub(super) fn line_count(&self) -> usize {
        self.lines.len()
    }

    fn page(&self) -> u16 {
        self.viewport.get().visible_height.saturating_sub(1).max(1)
    }

    /// Down: add, then clamp. Up: clamp, then subtract (UIFIX-04, as help.rs).
    fn down(&mut self, by: u16) {
        let vp = self.viewport.get();
        self.scroll_offset.set(clamp_scroll(
            self.scroll_offset.get().saturating_add(by),
            vp.total_lines,
            vp.visible_height,
        ));
    }

    fn up(&mut self, by: u16) {
        let vp = self.viewport.get();
        self.scroll_offset.set(
            clamp_scroll(self.scroll_offset.get(), vp.total_lines, vp.visible_height)
                .saturating_sub(by),
        );
    }
}

crate::ui::screens::adjudicate_screen!(
    OwnerFileViewScreen,
    crate::ui::screens::RENDERS_ATTACKER_INFLUENCED_IDENTITY,
    "Draws the file's text, which a project controls, and the registered alias in \
     the title. Markdown goes through `archive::render_markdown_lines`, which \
     escapes every line; any other file and the title go through `shown`. \
     Fixture state: the viewer over a hostile alias, path and file text.",
);

impl Screen for OwnerFileViewScreen {
    fn handle_key(
        &mut self,
        code: KeyCode,
        _modifiers: KeyModifiers,
        ctx: &mut AppContext,
    ) -> ScreenAction {
        ctx.needs_redraw = true;
        match code {
            KeyCode::Esc | KeyCode::Char('q') => ScreenAction::Pop,
            KeyCode::Char('?') => ScreenAction::Push(Box::new(super::help::HelpScreen::new())),
            KeyCode::Char('j') | KeyCode::Down => {
                self.down(1);
                ScreenAction::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.up(1);
                ScreenAction::None
            }
            KeyCode::PageDown => {
                self.down(self.page());
                ScreenAction::None
            }
            KeyCode::PageUp => {
                self.up(self.page());
                ScreenAction::None
            }
            KeyCode::Home => {
                self.scroll_offset.set(0);
                ScreenAction::None
            }
            KeyCode::End => {
                self.down(u16::MAX);
                ScreenAction::None
            }
            _ => ScreenAction::None,
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, _ctx: &AppContext) {
        if area.width == 0 || area.height == 0 {
            return;
        }
        let block = Block::default().borders(Borders::ALL).title(Span::styled(
            self.title.clone(),
            Style::default().add_modifier(Modifier::BOLD),
        ));
        let inner = block.inner(area);
        // The last inner row is the key hint and the scroll position, when the
        // frame has a row to spare for it.
        let footer = inner.height >= 3;
        let body_h = inner.height.saturating_sub(u16::from(footer));
        // Long lines wrap at the inner width instead of being cut by the border.
        // With a target every line gets a gutter, so the wrap is two cells
        // narrower.
        let gutter = if self.target.is_some() {
            GUTTER_CELLS
        } else {
            0
        };
        let wrap_width = usize::from(inner.width.saturating_sub(gutter)).max(1);
        let mut wrapped: Vec<Line<'static>> = Vec::with_capacity(self.lines.len());
        let mut target_row = None;
        for (index, line) in self.lines.iter().enumerate() {
            let rows = wrap_line(line, wrap_width);
            if self.target.is_none() {
                wrapped.extend(rows);
            } else {
                let is_target = self.target == Some(index);
                if is_target {
                    target_row = Some(wrapped.len());
                }
                for (n, row) in rows.into_iter().enumerate() {
                    wrapped.push(with_gutter(row, is_target, n == 0));
                }
            }
            if wrapped.len() > MAX_LINES {
                wrapped.truncate(MAX_LINES);
                break;
            }
        }
        let total = u16::try_from(wrapped.len()).unwrap_or(u16::MAX);
        self.viewport.set(ViewportMetrics {
            total_lines: total,
            visible_height: body_h,
        });
        if let Some(row) = target_row.filter(|_| self.jump_pending.replace(false)) {
            let top = u16::try_from(row.saturating_sub(TARGET_ROWS_FROM_TOP)).unwrap_or(u16::MAX);
            self.scroll_offset.set(clamp_scroll(top, total, body_h));
        }
        let offset = clamp_scroll(self.scroll_offset.get(), total, body_h);
        frame.render_widget(block, area);
        frame.render_widget(
            Paragraph::new(wrapped).scroll((offset, 0)),
            Rect::new(inner.x, inner.y, inner.width, body_h),
        );
        if footer {
            let last = usize::from(offset) + usize::from(body_h);
            let position = if total == 0 {
                "empty".to_owned()
            } else {
                format!(
                    "{}-{} of {}",
                    usize::from(offset) + 1,
                    last.min(usize::from(total)),
                    total
                )
            };
            let keys = footer_keys(usize::from(inner.width), cells(&position));
            let pad = usize::from(inner.width).saturating_sub(cells(&keys) + cells(&position));
            let dim = Style::default().add_modifier(Modifier::DIM);
            frame.render_widget(
                Paragraph::new(Line::from(vec![
                    Span::styled(keys, dim),
                    Span::raw(" ".repeat(pad)),
                    Span::styled(position, dim),
                ])),
                Rect::new(inner.x, inner.y + body_h, inner.width, 1),
            );
        }
    }

    fn name(&self) -> &str {
        "owner_file_view"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::screens::tests::ctx_with_aliases;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn draw(screen: &OwnerFileViewScreen, ctx: &AppContext, w: u16, h: u16) -> String {
        let mut terminal = Terminal::new(TestBackend::new(w, h)).unwrap();
        terminal
            .draw(|frame| screen.render(frame, frame.area(), ctx))
            .unwrap();
        let buf = terminal.backend().buffer().clone();
        (0..h)
            .map(|y| {
                (0..w)
                    .map(|x| buf[(x, y)].symbol().to_owned())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn many_lines(n: usize) -> String {
        (0..n)
            .map(|i| format!("line {i:03}"))
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn the_viewer_shows_a_key_hint_a_position_and_wraps_long_lines() {
        let ctx = ctx_with_aliases(&["alder-api"]);
        let long = format!("{} tail-marker", "word ".repeat(40));
        let text = format!("{long}\n{}", many_lines(30));
        let screen = OwnerFileViewScreen::new("alder-api", ".planning/HANDOFF.json", &text);
        let out = draw(&screen, &ctx, 60, 14);
        assert!(out.contains("[Esc]back  [j/k]scroll"), "{out}");
        assert!(out.contains("1-11 of "), "{out}");
        assert!(!out.contains('\u{2026}'), "nothing is cut: {out}");
        // The tail of the long line is on screen, wrapped, not lost at the border.
        assert!(out.contains("tail-marker"), "{out}");
        let rows: Vec<&str> = out.lines().collect();
        for row in &rows[1..rows.len() - 1] {
            assert!(
                row.starts_with('\u{2502}') && row.ends_with('\u{2502}'),
                "{row:?}"
            );
        }
        // At 100 columns the same file wraps less and the total changes.
        let wide = draw(&screen, &ctx, 100, 14);
        assert!(wide.contains("[PgUp/PgDn]page"), "{wide}");
        assert!(wide.contains("tail-marker"), "{wide}");
    }

    #[test]
    fn wrap_line_keeps_every_cell_and_breaks_at_spaces() {
        let line = Line::from(vec![
            Span::raw("alpha beta "),
            Span::styled(
                "gamma-delta-epsilon",
                Style::default().add_modifier(Modifier::BOLD),
            ),
        ]);
        let rows = wrap_line(&line, 12);
        let flat: Vec<String> = rows
            .iter()
            .map(|l| l.spans.iter().map(|s| s.content.as_ref()).collect())
            .collect();
        assert_eq!(flat, ["alpha beta", "gamma-delta-", "epsilon"]);
        assert!(rows.iter().all(|l| cells(
            &l.spans
                .iter()
                .map(|s| s.content.as_ref())
                .collect::<String>()
        ) <= 12));
        assert_eq!(wrap_line(&Line::from(""), 5).len(), 1);
    }

    #[test]
    fn scrolls_and_pops() {
        let mut ctx = ctx_with_aliases(&["alder-api"]);
        let mut screen =
            OwnerFileViewScreen::new("alder-api", ".planning/HANDOFF.json", &many_lines(100));
        let text = draw(&screen, &ctx, 40, 12);
        assert!(
            text.contains("alder-api \u{b7} .planning/HANDOFF.json"),
            "{text}"
        );
        assert!(text.contains("line 000"));
        let press = |s: &mut OwnerFileViewScreen, ctx: &mut AppContext, code| {
            s.handle_key(code, KeyModifiers::NONE, ctx)
        };
        press(&mut screen, &mut ctx, KeyCode::Char('j'));
        assert_eq!(screen.scroll_offset(), 1);
        press(&mut screen, &mut ctx, KeyCode::PageDown);
        assert_eq!(
            screen.scroll_offset(),
            1 + 8,
            "a page is the body height (the hint row is not body) minus 1"
        );
        press(&mut screen, &mut ctx, KeyCode::End);
        assert_eq!(
            screen.scroll_offset(),
            100 - 9,
            "End clamps to the last page"
        );
        let text = draw(&screen, &ctx, 40, 12);
        assert!(text.contains("line 099"), "{text}");
        press(&mut screen, &mut ctx, KeyCode::Char('j'));
        assert_eq!(
            screen.scroll_offset(),
            91,
            "down past the end stays clamped"
        );
        press(&mut screen, &mut ctx, KeyCode::Char('k'));
        assert_eq!(screen.scroll_offset(), 90);
        press(&mut screen, &mut ctx, KeyCode::Home);
        assert_eq!(screen.scroll_offset(), 0);
        press(&mut screen, &mut ctx, KeyCode::Char('k'));
        assert_eq!(screen.scroll_offset(), 0, "up at the top stays at the top");
        assert!(matches!(
            press(&mut screen, &mut ctx, KeyCode::Esc),
            ScreenAction::Pop
        ));
        assert!(matches!(
            press(&mut screen, &mut ctx, KeyCode::Char('q')),
            ScreenAction::Pop
        ));
        assert!(matches!(
            press(&mut screen, &mut ctx, KeyCode::Char('e')),
            ScreenAction::None
        ));
    }

    /// WR-06: a file with more lines than a `u16` scroll can reach says so.
    #[test]
    fn a_file_past_the_scroll_range_ends_with_a_notice() {
        let ctx = ctx_with_aliases(&["alder-api"]);
        let text = many_lines(70_000);
        let mut screen = OwnerFileViewScreen::new("alder-api", ".planning/HANDOFF.json", &text);
        assert_eq!(screen.line_count(), usize::from(u16::MAX));
        let mut ctx = ctx;
        let _ = draw(&screen, &ctx, 60, 6);
        screen.handle_key(KeyCode::End, KeyModifiers::NONE, &mut ctx);
        let shown = draw(&screen, &ctx, 60, 6);
        let hidden = 70_000 - (usize::from(u16::MAX) - 1);
        assert!(
            shown.contains(&format!("\u{2026} {hidden} more lines not shown")),
            "{shown}"
        );
        // A file that fits gets no notice.
        let fits = OwnerFileViewScreen::new("alder-api", ".planning/a.json", &many_lines(10));
        assert_eq!(fits.line_count(), 10);
    }

    /// Quick 261118-u4e (I-5): the target gutter, the REVERSED+BOLD row and the
    /// two-rows-below-the-top jump, inside the borders at 80 columns.
    #[test]
    fn at_line_marks_the_target_and_scrolls_it_two_rows_below_the_top() {
        let mut ctx = ctx_with_aliases(&["alder-api"]);
        let mut screen =
            OwnerFileViewScreen::new("alder-api", ".planning/HANDOFF.json", &many_lines(100))
                .at_line(Some(50));
        let mut terminal = Terminal::new(TestBackend::new(80, 24)).unwrap();
        terminal
            .draw(|frame| screen.render(frame, frame.area(), &ctx))
            .unwrap();
        assert_eq!(screen.scroll_offset(), 48);
        let buf = terminal.backend().buffer().clone();
        let row = |y: u16| -> String {
            (0..80u16)
                .map(|x| buf[(x, y)].symbol().to_owned())
                .collect()
        };
        // Body row 0 is screen row 1: line 48 is at the top, the target 2 below.
        assert!(row(1).starts_with("\u{2502}  line 048"), "{}", row(1));
        let target = row(3);
        let marker = crate::ui::glyphs::POINTER_RIGHT;
        assert!(
            target.starts_with(&format!("\u{2502}{marker} line 050")),
            "{target}"
        );
        for y in 1..23u16 {
            let line = row(y);
            assert!(
                line.starts_with('\u{2502}') && line.ends_with('\u{2502}'),
                "{line:?}"
            );
            let modifier = buf[(5, y)].modifier;
            let marked = modifier.contains(Modifier::REVERSED | Modifier::BOLD);
            assert_eq!(marked, y == 3, "row {y}");
            if y != 3 {
                assert!(!line.contains(marker), "{line}");
            }
        }
        // j moves on from the jumped offset.
        screen.handle_key(KeyCode::Char('j'), KeyModifiers::NONE, &mut ctx);
        assert_eq!(screen.scroll_offset(), 49);
        // A later frame keeps the owner's scroll: the jump happens once.
        terminal
            .draw(|frame| screen.render(frame, frame.area(), &ctx))
            .unwrap();
        assert_eq!(screen.scroll_offset(), 49);
    }

    #[test]
    fn at_line_none_or_out_of_range_renders_like_new() {
        let ctx = ctx_with_aliases(&["alder-api"]);
        let text = many_lines(100);
        let plain = OwnerFileViewScreen::new("alder-api", ".planning/HANDOFF.json", &text);
        let want = draw(&plain, &ctx, 80, 24);
        for at in [None, Some(100), Some(5000)] {
            let screen =
                OwnerFileViewScreen::new("alder-api", ".planning/HANDOFF.json", &text).at_line(at);
            assert_eq!(draw(&screen, &ctx, 80, 24), want, "{at:?}");
            assert_eq!(screen.scroll_offset(), 0);
        }
        // The truncation notice is not a target.
        let long = many_lines(70_000);
        let screen = OwnerFileViewScreen::new("alder-api", ".planning/HANDOFF.json", &long)
            .at_line(Some(usize::from(u16::MAX) - 1));
        assert!(screen.target.is_none());
        let last_real = OwnerFileViewScreen::new("alder-api", ".planning/HANDOFF.json", &long)
            .at_line(Some(usize::from(u16::MAX) - 2));
        assert!(last_real.target.is_some());
    }

    #[test]
    fn a_target_near_the_top_or_with_wrapping_stays_inside_the_borders() {
        let ctx = ctx_with_aliases(&["alder-api"]);
        let long = format!("target {}", "word ".repeat(40));
        let text = format!("one\ntwo\n{long}\nfour\n");
        let screen =
            OwnerFileViewScreen::new("alder-api", ".planning/HANDOFF.json", &text).at_line(Some(2));
        let out = draw(&screen, &ctx, 40, 12);
        assert_eq!(
            screen.scroll_offset(),
            0,
            "two rows from the top is the top"
        );
        let marker = crate::ui::glyphs::POINTER_RIGHT;
        assert_eq!(
            out.matches(marker).count(),
            1,
            "one marker for a wrapped line:\n{out}"
        );
        for row in out.lines().skip(1).take(10) {
            assert!(
                row.starts_with('\u{2502}') && row.ends_with('\u{2502}'),
                "{row:?}"
            );
        }
    }

    #[test]
    fn escapes_hostile_content() {
        let ctx = ctx_with_aliases(&["alder-api"]);
        let hostile = "a\u{1b}[31mred\u{7}\u{202e}rtl\u{200b}z\u{85}end";
        for path in [".planning/a.md", ".planning/HANDOFF.json"] {
            let screen =
                OwnerFileViewScreen::new(hostile, path, &format!("{hostile}\n```\n{hostile}\n```"));
            let mut terminal = Terminal::new(TestBackend::new(80, 10)).unwrap();
            terminal
                .draw(|frame| screen.render(frame, frame.area(), &ctx))
                .unwrap();
            let buf = terminal.backend().buffer().clone();
            for cell in buf.content() {
                assert!(
                    !cell.symbol().chars().any(char::is_control),
                    "raw control cell in {path}: {:?}",
                    cell.symbol()
                );
            }
        }
        let screen = OwnerFileViewScreen::new("alder-api", ".planning/x.md", "a\nb");
        assert_eq!(screen.line_count(), 2);
        assert_eq!(screen.title(), " alder-api \u{b7} .planning/x.md ");
    }
}
