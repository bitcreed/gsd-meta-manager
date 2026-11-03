//! **The one-off live viewer** (quick 261103-gt1, C-20..C-24): `v` in the `T` popup
//! opens a full-screen view that replays a run's `stream.jsonl` from the start and then
//! follows it, with a meter (notional spend and the rate limit) and, while the run is
//! live, a one-line composer (`i`) that sends a message through the host-only inbox.
//!
//! Runtime-neutral: the lines were classified and rendered by `oneoff::stream`. This
//! screen reads only the view cache (the bounded [`OneoffStream`] ring) and dispatches
//! actions; the tail itself runs on `spawn_blocking` in `App` and stops when the screen
//! drops. Every file-derived string is drawn through [`render_for_terminal`].

use std::cell::Cell;
use std::collections::VecDeque;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use super::{AppContext, Screen, ScreenAction};
use crate::action::Action;
use crate::oneoff::inbox::{validate_message, InboxRequest};
use crate::oneoff::store::LiveState;
use crate::oneoff::stream::{meter_text, Meter};
use crate::oneoff::MAX_MESSAGE_CHARS;
use crate::text::render_for_terminal;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

/// The most rendered lines the viewer keeps; the oldest are dropped.
pub const ONEOFF_VIEWER_RING_LINES: usize = 10_000;

/// What the viewer knows about the run besides its lines.
#[derive(Debug, Clone, PartialEq)]
pub struct RunInfo {
    pub live: LiveState,
    pub hard_usd: f64,
    pub started_at: String,
    pub ended_at: Option<String>,
}

/// The bounded buffer of one viewed run, kept in the project's view cache.
#[derive(Debug, Clone)]
pub struct OneoffStream {
    pub run_id: String,
    /// Which `v` press filled this buffer: batches of an earlier viewer's loop that are
    /// still in flight are ignored.
    pub epoch: u64,
    pub run_dir: PathBuf,
    pub lines: VecDeque<String>,
    /// How many lines were dropped from the front of the ring.
    pub dropped: u64,
    pub meter: Meter,
    pub info: RunInfo,
    pub done: bool,
}

impl OneoffStream {
    pub fn new(run_id: String, epoch: u64, run_dir: PathBuf, info: RunInfo) -> Self {
        OneoffStream {
            run_id,
            epoch,
            run_dir,
            lines: VecDeque::new(),
            dropped: 0,
            meter: Meter::default(),
            info,
            done: false,
        }
    }

    /// Append rendered lines, dropping the oldest past [`ONEOFF_VIEWER_RING_LINES`].
    pub fn push(&mut self, lines: Vec<String>) {
        self.lines.extend(lines);
        while self.lines.len() > ONEOFF_VIEWER_RING_LINES {
            self.lines.pop_front();
            self.dropped += 1;
        }
    }

    /// The absolute index one past the last line.
    fn end(&self) -> u64 {
        self.dropped + self.lines.len() as u64
    }
}

/// The viewer of one project's run.
pub struct OneoffViewerScreen {
    alias: String,
    epoch: u64,
    /// The absolute index of the top visible line while not following.
    top: u64,
    follow: bool,
    composer: Option<String>,
    notice: Option<String>,
    /// The body's height as last drawn, for paging and for the follow anchor.
    height: Cell<usize>,
    /// Set on drop: the tail loop stops.
    cancel: Arc<AtomicBool>,
}

impl OneoffViewerScreen {
    pub fn new(alias: String, epoch: u64, cancel: Arc<AtomicBool>) -> Self {
        Self {
            alias,
            epoch,
            top: 0,
            follow: true,
            composer: None,
            notice: None,
            height: Cell::new(20),
            cancel,
        }
    }

    fn stream<'a>(&self, ctx: &'a AppContext) -> Option<&'a OneoffStream> {
        ctx.view_cache
            .get(&self.alias)
            .and_then(|c| c.oneoff_stream.as_ref())
            .filter(|s| s.epoch == self.epoch)
    }

    /// The absolute index of the first line drawn.
    fn effective_top(&self, stream: &OneoffStream) -> u64 {
        let bottom_top = stream.end().saturating_sub(self.height.get() as u64);
        let first = stream.dropped.saturating_sub(1).min(bottom_top);
        if self.follow {
            bottom_top
        } else {
            self.top.clamp(first, bottom_top)
        }
    }

    fn scroll_up(&mut self, stream: &OneoffStream, by: u64) {
        let top = self.effective_top(stream);
        self.top = top.saturating_sub(by);
        self.follow = false;
    }

    fn scroll_down(&mut self, stream: &OneoffStream, by: u64) {
        let top = self.effective_top(stream);
        self.top = top.saturating_add(by);
        self.follow = false;
    }

    fn live(&self, ctx: &AppContext) -> bool {
        self.stream(ctx)
            .is_some_and(|s| s.info.live == LiveState::Running)
    }

    fn composer_key(
        &mut self,
        code: KeyCode,
        modifiers: KeyModifiers,
        ctx: &mut AppContext,
    ) -> ScreenAction {
        let Some(buffer) = self.composer.as_mut() else {
            return ScreenAction::None;
        };
        match code {
            KeyCode::Esc => self.composer = None,
            KeyCode::Backspace => {
                buffer.pop();
            }
            KeyCode::Enter => {
                let text = buffer.trim().to_owned();
                self.composer = None;
                if text.is_empty() {
                    return ScreenAction::None;
                }
                if let Err(why) = validate_message(&text) {
                    self.notice = Some(why);
                    return ScreenAction::None;
                }
                let Some(stream) = self.stream(ctx) else {
                    return ScreenAction::None;
                };
                return ScreenAction::DispatchAction(Box::new(Action::OneoffInjectRequested {
                    alias: self.alias.clone(),
                    run_dir: stream.run_dir.clone(),
                    request: InboxRequest::Message { text },
                }));
            }
            KeyCode::Char(c)
                if !modifiers.contains(KeyModifiers::CONTROL)
                    && !modifiers.contains(KeyModifiers::ALT)
                    && buffer.chars().count() < MAX_MESSAGE_CHARS =>
            {
                buffer.push(c);
            }
            _ => {}
        }
        ScreenAction::None
    }
}

impl Drop for OneoffViewerScreen {
    fn drop(&mut self) {
        self.cancel.store(true, Ordering::SeqCst);
    }
}

crate::ui::screens::adjudicate_screen!(
    OneoffViewerScreen,
    crate::ui::screens::RENDERS_ATTACKER_INFLUENCED_IDENTITY,
    "Draws the run id, the registry key, the rate-limit words and every rendered stream \
     line (agent-written text), all through render_for_terminal. Fixture states: a hostile \
     run id and hostile stream lines, and the composer holding a hostile typed message.",
);

/// `text` escaped, on one line.
fn shown(text: &str) -> String {
    render_for_terminal(text).to_string()
}

fn elapsed_text(info: &RunInfo, now: chrono::DateTime<chrono::Utc>) -> String {
    let parse = |s: &str| {
        chrono::DateTime::parse_from_rfc3339(s)
            .ok()
            .map(|t| t.with_timezone(&chrono::Utc))
    };
    let Some(start) = parse(&info.started_at) else {
        return "elapsed ?".to_owned();
    };
    let end = match info.live {
        LiveState::Running | LiveState::Preparing => now,
        _ => info.ended_at.as_deref().and_then(parse).unwrap_or(now),
    };
    let secs = (end - start).num_seconds().max(0);
    format!("elapsed {}m{:02}s", secs / 60, secs % 60)
}

impl Screen for OneoffViewerScreen {
    fn handle_key(
        &mut self,
        code: KeyCode,
        modifiers: KeyModifiers,
        ctx: &mut AppContext,
    ) -> ScreenAction {
        ctx.needs_redraw = true;
        if self.composer.is_some() {
            return self.composer_key(code, modifiers, ctx);
        }
        self.notice = None;
        let page = self.height.get().max(2) as u64 - 1;
        let Some(stream) = self.stream(ctx).cloned() else {
            return match code {
                KeyCode::Esc | KeyCode::Char('q') => ScreenAction::Pop,
                _ => ScreenAction::None,
            };
        };
        match code {
            KeyCode::Esc | KeyCode::Char('q') => return ScreenAction::Pop,
            KeyCode::PageUp => self.scroll_up(&stream, page),
            KeyCode::PageDown => self.scroll_down(&stream, page),
            KeyCode::Char('k') | KeyCode::Up => self.scroll_up(&stream, 1),
            KeyCode::Char('j') | KeyCode::Down => self.scroll_down(&stream, 1),
            KeyCode::Char('f') => {
                if self.follow {
                    self.top = self.effective_top(&stream);
                    self.follow = false;
                } else {
                    self.follow = true;
                }
            }
            KeyCode::Char('G') | KeyCode::End => self.follow = true,
            KeyCode::Char('i') => {
                if self.live(ctx) {
                    self.composer = Some(String::new());
                } else {
                    self.notice = Some("run is not live".to_owned());
                }
            }
            _ => {}
        }
        ScreenAction::None
    }

    fn render(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        frame.render_widget(Clear, area);
        let title = format!(" one-off live view - {} ", shown(&self.alias));
        let block = Block::default().borders(Borders::ALL).title(title);
        let inner = block.inner(area);
        frame.render_widget(block, area);
        let dim = Style::default().fg(Color::DarkGray);
        let Some(stream) = self.stream(ctx) else {
            frame.render_widget(Paragraph::new("Loading the run's stream..."), inner);
            return;
        };

        let state = match stream.info.live {
            LiveState::Running | LiveState::Preparing => "live",
            LiveState::Finished => "finished",
            LiveState::Crashed => "crashed",
            LiveState::Unknown => "unknown",
        };
        let header = [
            Line::from(vec![
                Span::raw(shown(&stream.run_id)),
                Span::styled(format!("  {state}  "), Style::default().fg(Color::Cyan)),
                Span::raw(elapsed_text(&stream.info, chrono::Utc::now())),
            ]),
            Line::from(Span::raw(shown(&meter_text(
                &stream.meter,
                stream.info.hard_usd,
            )))),
        ];
        let footer_rows = 1_u16;
        let head_rows = header.len() as u16;
        let body_rows = inner.height.saturating_sub(head_rows + footer_rows);
        self.height.set(usize::from(body_rows).max(1));

        let top = self.effective_top(stream);
        let mut body: Vec<Line> = Vec::new();
        let first_virtual = stream.dropped.saturating_sub(1);
        for abs in top..top.saturating_add(u64::from(body_rows)) {
            if abs >= stream.end() {
                break;
            }
            if abs < stream.dropped {
                if abs >= first_virtual && stream.dropped > 0 {
                    body.push(Line::from(Span::styled(
                        format!("… {} earlier lines dropped", stream.dropped),
                        dim,
                    )));
                }
                continue;
            }
            let idx = usize::try_from(abs - stream.dropped).unwrap_or(usize::MAX);
            if let Some(line) = stream.lines.get(idx) {
                body.push(Line::from(shown(line)));
            }
        }

        let hint = match (&self.composer, &self.notice) {
            (Some(buffer), _) => Line::from(vec![
                Span::styled("message> ", Style::default().fg(Color::Yellow)),
                Span::raw(shown(buffer)),
                Span::raw("_"),
                Span::styled("   Enter send   Esc cancel", dim),
            ]),
            (None, Some(notice)) => Line::from(Span::styled(
                shown(notice),
                Style::default().fg(Color::Yellow),
            )),
            (None, None) => {
                let follow = if self.follow { " [following]" } else { "" };
                Line::from(Span::styled(
                    format!(
                        "PgUp/PgDn j/k scroll · f follow · G end · i message · Esc close{follow}"
                    ),
                    dim,
                ))
            }
        };

        let mut rows: Vec<Line> = header.to_vec();
        rows.extend(body);
        frame.render_widget(
            Paragraph::new(rows),
            Rect {
                height: inner.height.saturating_sub(footer_rows),
                ..inner
            },
        );
        frame.render_widget(
            Paragraph::new(hint),
            Rect {
                y: inner.y + inner.height.saturating_sub(footer_rows),
                height: footer_rows.min(inner.height),
                ..inner
            },
        );
    }

    fn name(&self) -> &str {
        "oneoff_viewer"
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::ui::screens::driver_confirm::tests::{ctx_with_project, ALIAS};

    pub(crate) fn info(live: LiveState) -> RunInfo {
        RunInfo {
            live,
            hard_usd: 10.0,
            started_at: "2026-11-03T10:00:00Z".to_owned(),
            ended_at: Some("2026-11-03T10:05:00Z".to_owned()),
        }
    }

    /// Seed `ctx` with a buffer of `lines` for `run_id` under `alias`, and return a
    /// viewer on it.
    pub(crate) fn seed(
        ctx: &mut AppContext,
        alias: &str,
        run_id: &str,
        lines: Vec<String>,
        live: LiveState,
    ) -> OneoffViewerScreen {
        let mut stream =
            OneoffStream::new(run_id.to_owned(), 7, PathBuf::from("/run/dir"), info(live));
        stream.push(lines);
        ctx.view_cache
            .entry(alias.to_owned())
            .or_default()
            .oneoff_stream = Some(stream);
        OneoffViewerScreen::new(alias.to_owned(), 7, Arc::new(AtomicBool::new(false)))
    }

    /// A context whose cache holds a buffer of `lines` for `run_id`, and a viewer on it.
    pub(crate) fn viewer_over(
        run_id: &str,
        lines: Vec<String>,
        live: LiveState,
    ) -> (AppContext, OneoffViewerScreen) {
        let dir = tempfile::tempdir().unwrap();
        let (mut ctx, _rx) = ctx_with_project(dir.path());
        let screen = seed(&mut ctx, ALIAS, run_id, lines, live);
        (ctx, screen)
    }

    fn text(screen: &OneoffViewerScreen, ctx: &AppContext, width: u16, height: u16) -> String {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;
        let mut t = Terminal::new(TestBackend::new(width, height)).unwrap();
        t.draw(|f| screen.render(f, f.area(), ctx)).unwrap();
        let buf = t.backend().buffer();
        (0..height)
            .map(|y| (0..width).map(|x| buf[(x, y)].symbol()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn key(screen: &mut OneoffViewerScreen, ctx: &mut AppContext, code: KeyCode) -> ScreenAction {
        screen.handle_key(code, KeyModifiers::NONE, ctx)
    }

    fn numbered(n: usize) -> Vec<String> {
        (0..n).map(|i| format!("line {i}")).collect()
    }

    #[test]
    fn the_header_the_meter_and_the_tail_are_drawn_following_by_default() {
        let (mut ctx, screen) =
            viewer_over("oneoff-20261103-100000", numbered(100), LiveState::Running);
        if let Some(s) = ctx
            .view_cache
            .get_mut(ALIAS)
            .and_then(|c| c.oneoff_stream.as_mut())
        {
            s.meter.estimate = 1.5;
        }
        let drawn = text(&screen, &ctx, 100, 20);
        assert!(drawn.contains("oneoff-20261103-100000"), "{drawn}");
        assert!(drawn.contains("live"), "{drawn}");
        assert!(drawn.contains("elapsed"), "{drawn}");
        assert!(
            drawn.contains("spend ≈ $1.50 / hard $10.00 (notional)"),
            "{drawn}"
        );
        assert!(drawn.contains("line 99"), "{drawn}");
        assert!(!drawn.contains("line 0 "), "{drawn}");
        assert!(drawn.contains("[following]"), "{drawn}");
    }

    #[test]
    fn paging_and_follow_keys_move_the_window() {
        let (mut ctx, mut screen) = viewer_over("oneoff-x", numbered(100), LiveState::Finished);
        text(&screen, &ctx, 100, 20);
        key(&mut screen, &mut ctx, KeyCode::PageUp);
        assert!(!screen.follow);
        let drawn = text(&screen, &ctx, 100, 20);
        assert!(!drawn.contains("line 99"), "{drawn}");
        assert!(!drawn.contains("[following]"), "{drawn}");
        let before = screen.effective_top(screen.stream(&ctx).unwrap());
        key(&mut screen, &mut ctx, KeyCode::Char('k'));
        assert_eq!(
            screen.effective_top(screen.stream(&ctx).unwrap()),
            before - 1
        );
        key(&mut screen, &mut ctx, KeyCode::Char('j'));
        assert_eq!(screen.effective_top(screen.stream(&ctx).unwrap()), before);
        key(&mut screen, &mut ctx, KeyCode::Char('G'));
        assert!(screen.follow);
        assert!(text(&screen, &ctx, 100, 20).contains("line 99"));
        key(&mut screen, &mut ctx, KeyCode::Char('f'));
        assert!(!screen.follow);
        key(&mut screen, &mut ctx, KeyCode::Char('f'));
        assert!(screen.follow);
    }

    #[test]
    fn a_scrolled_view_stays_put_while_lines_arrive() {
        let (mut ctx, mut screen) = viewer_over("oneoff-x", numbered(100), LiveState::Running);
        text(&screen, &ctx, 100, 20);
        key(&mut screen, &mut ctx, KeyCode::PageUp);
        let before = text(&screen, &ctx, 100, 20);
        ctx.view_cache
            .get_mut(ALIAS)
            .and_then(|c| c.oneoff_stream.as_mut())
            .unwrap()
            .push(numbered(5));
        assert_eq!(text(&screen, &ctx, 100, 20), before);
    }

    #[test]
    fn the_composer_owns_every_key_and_sends_through_the_inbox() {
        let (mut ctx, mut screen) = viewer_over("oneoff-x", numbered(3), LiveState::Running);
        key(&mut screen, &mut ctx, KeyCode::Char('i'));
        assert!(screen.composer.is_some());
        for c in "quit it".chars() {
            key(&mut screen, &mut ctx, KeyCode::Char(c));
        }
        key(&mut screen, &mut ctx, KeyCode::Backspace);
        assert!(text(&screen, &ctx, 100, 20).contains("message> quit i_"));
        match key(&mut screen, &mut ctx, KeyCode::Enter) {
            ScreenAction::DispatchAction(action) => match *action {
                Action::OneoffInjectRequested {
                    alias,
                    run_dir,
                    request,
                } => {
                    assert_eq!(alias, ALIAS);
                    assert_eq!(run_dir, PathBuf::from("/run/dir"));
                    assert_eq!(
                        request,
                        InboxRequest::Message {
                            text: "quit i".into()
                        }
                    );
                }
                other => panic!("{other:?}"),
            },
            _ => panic!("expected a dispatch"),
        }
        assert!(screen.composer.is_none());
        key(&mut screen, &mut ctx, KeyCode::Char('i'));
        key(&mut screen, &mut ctx, KeyCode::Esc);
        assert!(screen.composer.is_none(), "Esc cancels, it does not pop");
    }

    #[test]
    fn a_run_that_is_not_live_takes_no_message() {
        let (mut ctx, mut screen) = viewer_over("oneoff-x", numbered(3), LiveState::Finished);
        key(&mut screen, &mut ctx, KeyCode::Char('i'));
        assert!(screen.composer.is_none());
        assert!(text(&screen, &ctx, 100, 20).contains("run is not live"));
    }

    #[test]
    fn esc_and_q_pop_and_dropping_stops_the_tail() {
        let (mut ctx, mut screen) = viewer_over("oneoff-x", numbered(3), LiveState::Running);
        assert!(matches!(
            key(&mut screen, &mut ctx, KeyCode::Esc),
            ScreenAction::Pop
        ));
        assert!(matches!(
            key(&mut screen, &mut ctx, KeyCode::Char('q')),
            ScreenAction::Pop
        ));
        let cancel = Arc::clone(&screen.cancel);
        assert!(!cancel.load(Ordering::SeqCst));
        drop(screen);
        assert!(cancel.load(Ordering::SeqCst));
    }

    #[test]
    fn the_ring_is_bounded_and_says_what_it_dropped() {
        let (mut ctx, mut screen) = viewer_over("oneoff-x", numbered(10_005), LiveState::Finished);
        let stream = screen.stream(&ctx).unwrap();
        assert_eq!(stream.lines.len(), ONEOFF_VIEWER_RING_LINES);
        assert_eq!(stream.dropped, 5);
        text(&screen, &ctx, 100, 20);
        key(&mut screen, &mut ctx, KeyCode::PageUp);
        for _ in 0..900 {
            key(&mut screen, &mut ctx, KeyCode::PageUp);
        }
        let drawn = text(&screen, &ctx, 100, 20);
        assert!(drawn.contains("… 5 earlier lines dropped"), "{drawn}");
        assert!(drawn.contains("line 5"), "{drawn}");
    }

    #[test]
    fn hostile_text_is_drawn_escaped() {
        let hostile = "evil\u{1b}[31m\u{202e}txt".to_owned();
        let (ctx, screen) = viewer_over(&hostile, vec![hostile.clone()], LiveState::Running);
        let drawn = text(&screen, &ctx, 100, 20);
        assert!(!drawn.contains('\u{1b}'), "{drawn:?}");
        assert!(!drawn.contains('\u{202e}'), "{drawn:?}");
    }

    #[test]
    fn a_batch_of_another_viewer_is_not_shown() {
        let (mut ctx, screen) = viewer_over("oneoff-x", numbered(3), LiveState::Running);
        ctx.view_cache
            .get_mut(ALIAS)
            .and_then(|c| c.oneoff_stream.as_mut())
            .unwrap()
            .epoch = 8;
        assert!(text(&screen, &ctx, 100, 20).contains("Loading"));
    }
}
