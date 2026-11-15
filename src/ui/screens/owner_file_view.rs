//! The read-only file viewer behind `o` (37-05, D-08).
//!
//! `o` on an inbox row or an Owner block row shows the group's source file inside
//! the app. It never starts `$EDITOR` or any other program (T-37-16): this screen
//! hands no file to an editor and nothing is spawned. The file is read
//! once, on the key press, by [`super::owner_actions::read_owner_file`] (contained
//! to the project's `.planning/`, at most 1 MiB); this screen only holds the
//! lines computed at push time, so `render` does no I/O (OWN-12).

use super::detail::{clamp_scroll, shown, ViewportMetrics};
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

pub struct OwnerFileViewScreen {
    title: String,
    /// The file as lines, escaped once at push time.
    lines: Vec<Line<'static>>,
    scroll_offset: u16,
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
        if lines.len() > MAX_LINES {
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
            scroll_offset: 0,
            viewport: Cell::new(ViewportMetrics::default()),
        }
    }

    #[cfg(test)]
    pub(super) fn scroll_offset(&self) -> u16 {
        self.scroll_offset
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
        self.scroll_offset = clamp_scroll(
            self.scroll_offset.saturating_add(by),
            vp.total_lines,
            vp.visible_height,
        );
    }

    fn up(&mut self, by: u16) {
        let vp = self.viewport.get();
        self.scroll_offset =
            clamp_scroll(self.scroll_offset, vp.total_lines, vp.visible_height).saturating_sub(by);
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
                self.scroll_offset = 0;
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
        // `new` caps the line count at `u16::MAX`, so this never saturates.
        let total = u16::try_from(self.lines.len()).unwrap_or(u16::MAX);
        self.viewport.set(ViewportMetrics {
            total_lines: total,
            visible_height: inner.height,
        });
        let offset = clamp_scroll(self.scroll_offset, total, inner.height);
        frame.render_widget(
            Paragraph::new(self.lines.clone())
                .block(block)
                .scroll((offset, 0)),
            area,
        );
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
            1 + 9,
            "a page is the height minus 1"
        );
        press(&mut screen, &mut ctx, KeyCode::End);
        assert_eq!(
            screen.scroll_offset(),
            100 - 10,
            "End clamps to the last page"
        );
        let text = draw(&screen, &ctx, 40, 12);
        assert!(text.contains("line 099"), "{text}");
        press(&mut screen, &mut ctx, KeyCode::Char('j'));
        assert_eq!(
            screen.scroll_offset(),
            90,
            "down past the end stays clamped"
        );
        press(&mut screen, &mut ctx, KeyCode::Char('k'));
        assert_eq!(screen.scroll_offset(), 89);
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
