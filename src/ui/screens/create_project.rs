use super::confirm_popup::{self, ConfirmOutcome, ConfirmPopup};
use super::{AppContext, Screen, ScreenAction};
use crate::action::Action;
use crate::project_creator;
use crate::ui::mouse::MouseInput;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use std::path::{Path, PathBuf};

enum CreatePhase {
    Name,
    Path { name: String },
    Confirm { name: String, path: PathBuf },
}

pub struct CreateProjectScreen {
    phase: CreatePhase,
    /// The Confirm phase's Yes/No popup. Focus starts on Yes: creating a
    /// folder is non-destructive and Enter used to mean "create" here
    /// (INFERRED; the destructive popups default to No).
    popup: ConfirmPopup,
}

impl CreateProjectScreen {
    fn new_popup() -> ConfirmPopup {
        let mut popup = ConfirmPopup::default();
        popup.focus = confirm_popup::ConfirmFocus::Yes;
        popup
    }

    pub fn new_name() -> Self {
        Self {
            phase: CreatePhase::Name,
            popup: Self::new_popup(),
        }
    }
}

crate::ui::screens::adjudicate_screen!(
    CreateProjectScreen,
    crate::ui::screens::RENDERS_ATTACKER_INFLUENCED_IDENTITY,
    "Draws the project name the operator is typing, `ctx.error_message`, and \
     in its Confirm phase the chosen name and path. The name becomes a \
     directory, so it is an identity in the full sense. Fixture states: the \
     name field, and the name field with an error echoed beside it.",
);

impl Screen for CreateProjectScreen {
    fn handle_key(
        &mut self,
        code: KeyCode,
        _modifiers: KeyModifiers,
        ctx: &mut AppContext,
    ) -> ScreenAction {
        match &self.phase {
            CreatePhase::Name => self.handle_name_key(code, ctx),
            CreatePhase::Path { name } => {
                let name = name.clone();
                self.handle_path_key(code, ctx, &name)
            }
            CreatePhase::Confirm { name, path } => {
                let name = name.clone();
                let path = path.clone();
                self.handle_confirm_key(code, ctx, &name, &path)
            }
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        let chunks = ratatui::layout::Layout::vertical([
            ratatui::layout::Constraint::Min(0),
            ratatui::layout::Constraint::Length(1),
        ])
        .split(area);
        self.popup.record(None);

        // Render background
        use ratatui::widgets::{Block, Borders};
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" GSD Manager ");
        frame.render_widget(block, chunks[0]);

        // Footer
        match &self.phase {
            CreatePhase::Name => {
                render_input_footer(frame, chunks[1], ctx, "Project name");
            }
            CreatePhase::Path { .. } => {
                render_input_footer(frame, chunks[1], ctx, "Path (Tab to complete)");
            }
            CreatePhase::Confirm { name, path } => {
                // The footer stays an empty line; the question is a modal
                // popup over the dimmed frame.
                confirm_popup::dim_background(frame, area);
                self.popup.record(confirm_popup::render_confirm_popup(
                    frame,
                    area,
                    "Create project",
                    vec![Line::from(Span::styled(
                        confirm_popup::strip_key_hint(&prompt_text(name, path)).to_string(),
                        Style::default().fg(Color::Yellow),
                    ))],
                    Color::Yellow,
                    self.popup.focus,
                ));
            }
        }
    }

    fn handle_mouse(&mut self, input: MouseInput, ctx: &mut AppContext) -> ScreenAction {
        confirm_popup::click_as_key(&self.popup, input)
            .map_or(ScreenAction::None, |code| self.handle_key(code, KeyModifiers::NONE, ctx))
    }

    fn name(&self) -> &str {
        "create_project"
    }
}

impl CreateProjectScreen {
    fn handle_name_key(&mut self, code: KeyCode, ctx: &mut AppContext) -> ScreenAction {
        match code {
            KeyCode::Char(c) => {
                ctx.input_buffer.push(c);
                ctx.error_message = None;
                ctx.needs_redraw = true;
                ScreenAction::None
            }
            KeyCode::Backspace => {
                ctx.input_buffer.pop();
                ctx.needs_redraw = true;
                ScreenAction::None
            }
            KeyCode::Enter => {
                let name = ctx.input_buffer.trim().to_string();
                if name.is_empty() {
                    ctx.error_message = Some("Name cannot be empty.".to_string());
                    ctx.needs_redraw = true;
                    return ScreenAction::None;
                }
                let alias = name.to_lowercase().replace(' ', "-");
                if ctx.config.projects.contains_key(&alias) {
                    ctx.error_message = Some(format!(
                        "Alias \"{}\" already exists. Choose a different name.",
                        alias
                    ));
                    ctx.needs_redraw = true;
                    return ScreenAction::None;
                }
                self.phase = CreatePhase::Path { name };
                ctx.input_buffer.clear();
                ctx.error_message = None;
                ctx.needs_redraw = true;
                ScreenAction::None
            }
            KeyCode::Esc => {
                ctx.input_buffer.clear();
                ctx.error_message = None;
                ctx.needs_redraw = true;
                ScreenAction::Pop
            }
            _ => ScreenAction::None,
        }
    }

    fn handle_path_key(&mut self, code: KeyCode, ctx: &mut AppContext, name: &str) -> ScreenAction {
        match code {
            KeyCode::Char(c) => {
                ctx.input_buffer.push(c);
                ctx.error_message = None;
                ctx.needs_redraw = true;
                ScreenAction::None
            }
            KeyCode::Backspace => {
                ctx.input_buffer.pop();
                ctx.needs_redraw = true;
                ScreenAction::None
            }
            KeyCode::Tab => {
                let completions = project_creator::tab_complete_path(&ctx.input_buffer);
                if completions.len() == 1 {
                    ctx.input_buffer = completions[0].clone();
                    ctx.needs_redraw = true;
                } else if completions.len() > 1 {
                    ctx.status_message = Some((
                        format!("{} matches", completions.len()),
                        std::time::Instant::now(),
                    ));
                    ctx.needs_redraw = true;
                }
                ScreenAction::None
            }
            KeyCode::Enter => {
                let path = project_creator::resolve_path(&ctx.input_buffer);
                if path.is_dir() {
                    ctx.status_message = Some((
                        "Directory exists; will git-init in it.".to_string(),
                        std::time::Instant::now(),
                    ));
                }
                self.phase = CreatePhase::Confirm {
                    name: name.to_string(),
                    path,
                };
                self.popup = Self::new_popup();
                ctx.input_buffer.clear();
                ctx.error_message = None;
                ctx.needs_redraw = true;
                ScreenAction::None
            }
            KeyCode::Esc => {
                ctx.input_buffer.clear();
                ctx.error_message = None;
                ctx.needs_redraw = true;
                ScreenAction::Pop
            }
            _ => ScreenAction::None,
        }
    }

    fn handle_confirm_key(
        &mut self,
        code: KeyCode,
        ctx: &mut AppContext,
        name: &str,
        path: &Path,
    ) -> ScreenAction {
        match self.popup.on_key(code) {
            ConfirmOutcome::Confirm => {
                let alias = name.to_lowercase().replace(' ', "-");
                let hooks = ctx.config.preferences.hooks.clone();
                let path_clone = path.to_path_buf();
                let name_clone = name.to_string();
                let alias_clone = alias.clone();

                if let Some(tx) = ctx.event_tx.clone() {
                    tokio::task::spawn_blocking(move || {
                        let result =
                            project_creator::create_project(&name_clone, &path_clone, &hooks);
                        let (success, error): (bool, Option<String>) = match result {
                            Ok(()) => (true, None),
                            Err(e) => (false, Some(format!("{}", e))),
                        };
                        let _ = tx.send(Action::CreateProjectResult {
                            alias: alias_clone,
                            path: path_clone,
                            success,
                            error,
                        });
                    });

                    ctx.status_message = Some((
                        format!("Creating project \"{}\"...", alias),
                        std::time::Instant::now(),
                    ));
                    ctx.needs_redraw = true;
                    ScreenAction::Pop
                } else {
                    ctx.error_message = Some("Event channel not available.".to_string());
                    ctx.needs_redraw = true;
                    ScreenAction::None
                }
            }
            ConfirmOutcome::Cancel => {
                ctx.input_buffer.clear();
                ctx.error_message = None;
                ctx.needs_redraw = true;
                ScreenAction::Pop
            }
            ConfirmOutcome::Pending => {
                ctx.needs_redraw = true;
                ScreenAction::None
            }
        }
    }
}

/// The exact question the Confirm popup draws.
///
/// Read, not created: `name` and `path` are still used verbatim by
/// `handle_confirm_key` to create the directory; only this last-chance prompt
/// is escaped.
fn prompt_text(name: &str, path: &Path) -> String {
    format!(
        "Create \"{}\" at {}? [y/n]",
        crate::text::render_for_terminal(name),
        crate::text::render_for_terminal(&path.display().to_string())
    )
}

fn render_input_footer(frame: &mut Frame, area: Rect, ctx: &AppContext, label: &str) {
    // The split, at the site: the project name and the path a HUMAN READS are
    // escaped; the bytes handed to `project_creator` and to the filesystem are
    // `ctx.input_buffer` raw, unchanged. The name becomes a directory, so an
    // invisible character in it is an identity difference the operator cannot
    // see at the one moment they could still refuse it.
    //
    // Both halves, through the ONE composition (WR-05). See `crate::ui::tests`.
    let mut spans = vec![
        Span::raw(format!("{}: ", label)),
        Span::styled(
            crate::text::render_for_terminal(&ctx.input_buffer),
            Style::default().add_modifier(Modifier::UNDERLINED),
        ),
        Span::raw("_"),
    ];

    if let Some(err) = &ctx.error_message {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            crate::text::render_for_terminal(err),
            Style::default().fg(Color::Red),
        ));
    }

    let line = Line::from(spans);
    frame.render_widget(Paragraph::new(line), area);
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    fn type_str(screen: &mut CreateProjectScreen, ctx: &mut AppContext, text: &str) {
        for c in text.chars() {
            screen.handle_key(KeyCode::Char(c), KeyModifiers::NONE, ctx);
        }
        screen.handle_key(KeyCode::Enter, KeyModifiers::NONE, ctx);
    }

    /// A screen in its Confirm phase for `name` at `/tmp/where`.
    fn confirm_screen(name: &str) -> (CreateProjectScreen, AppContext) {
        let mut ctx = crate::ui::screens::tests::ctx_with_aliases(&["demo"]);
        let mut screen = CreateProjectScreen::new_name();
        type_str(&mut screen, &mut ctx, name);
        type_str(&mut screen, &mut ctx, "/tmp/where");
        assert!(matches!(screen.phase, CreatePhase::Confirm { .. }));
        (screen, ctx)
    }

    fn rows(screen: &CreateProjectScreen, ctx: &AppContext) -> String {
        let mut t = Terminal::new(TestBackend::new(100, 30)).unwrap();
        t.draw(|f| screen.render(f, f.area(), ctx)).unwrap();
        let buf = t.backend().buffer();
        (0..30u16)
            .map(|y| (0..100u16).map(|x| buf[(x, y)].symbol()).collect::<String>())
            .collect::<Vec<_>>()
            .join("\n")
    }

    #[test]
    fn confirm_phase_renders_a_popup_with_buttons_not_a_footer() {
        let (screen, ctx) = confirm_screen("Fresh App");
        let text = rows(&screen, &ctx);
        assert!(text.contains("Create project"), "popup title missing");
        assert!(text.contains("Create \"Fresh App\" at /tmp/where?"));
        assert!(text.contains("[ Yes ]") && text.contains("[ No ]"));
        assert!(!text.contains("[y/n]"), "legacy footer hint must be gone");
    }

    #[test]
    fn prompt_text_escapes_hostile_name_and_path() {
        let hostile = "ev\u{1b}[31mil\u{9b}\u{200b}";
        let p = prompt_text(hostile, Path::new("/tmp/a\u{1b}b"));
        assert!(!p.contains('\u{1b}') && !p.contains('\u{9b}'), "got {p:?}");
        assert!(p.starts_with("Create \"") && p.ends_with("? [y/n]"));
        let (screen, ctx) = confirm_screen(hostile);
        let text = rows(&screen, &ctx);
        assert!(!text.contains('\u{1b}') && !text.contains('\u{9b}'));
    }

    #[test]
    fn enter_defaults_to_yes_and_reaches_creation() {
        // No event channel in the fixture, so a confirmed create reports that
        // instead of spawning: proof that Enter took the Yes path.
        let (mut screen, mut ctx) = confirm_screen("fresh");
        assert_eq!(screen.popup.focus, confirm_popup::ConfirmFocus::Yes);
        let act = screen.handle_key(KeyCode::Enter, KeyModifiers::NONE, &mut ctx);
        assert!(matches!(act, ScreenAction::None));
        assert_eq!(ctx.error_message.as_deref(), Some("Event channel not available."));
    }

    #[test]
    fn y_n_esc_and_focus_moves_keep_working() {
        let (mut screen, mut ctx) = confirm_screen("fresh");
        screen.handle_key(KeyCode::Char('y'), KeyModifiers::NONE, &mut ctx);
        assert_eq!(ctx.error_message.as_deref(), Some("Event channel not available."));

        for code in [KeyCode::Char('n'), KeyCode::Esc] {
            let (mut screen, mut ctx) = confirm_screen("fresh");
            let act = screen.handle_key(code, KeyModifiers::NONE, &mut ctx);
            assert!(matches!(act, ScreenAction::Pop), "{code:?}");
            assert!(ctx.error_message.is_none());
        }

        // Tab moves focus to No; Enter then cancels.
        let (mut screen, mut ctx) = confirm_screen("fresh");
        let act = screen.handle_key(KeyCode::Tab, KeyModifiers::NONE, &mut ctx);
        assert!(matches!(act, ScreenAction::None));
        let act = screen.handle_key(KeyCode::Enter, KeyModifiers::NONE, &mut ctx);
        assert!(matches!(act, ScreenAction::Pop));
        assert!(ctx.error_message.is_none());
    }

    #[test]
    fn clicking_buttons_confirms_or_cancels_and_stray_clicks_are_inert() {
        let click = |screen: &CreateProjectScreen, ctx: &AppContext, label: &str| {
            let (column, row) =
                confirm_popup::locate(&rows(screen, ctx), label).expect("button drawn");
            MouseInput::Click { column, row, double: false }
        };
        let (mut screen, mut ctx) = confirm_screen("fresh");
        let stray = MouseInput::Click { column: 0, row: 0, double: false };
        assert!(matches!(screen.handle_mouse(stray, &mut ctx), ScreenAction::None));
        let c = click(&screen, &ctx, "[ No ]");
        assert!(matches!(screen.handle_mouse(c, &mut ctx), ScreenAction::Pop));
        assert!(ctx.error_message.is_none());

        let (mut screen, mut ctx) = confirm_screen("fresh");
        let c = click(&screen, &ctx, "[ Yes ]");
        screen.handle_mouse(c, &mut ctx);
        assert_eq!(ctx.error_message.as_deref(), Some("Event channel not available."));
    }

    #[test]
    fn name_phase_ignores_clicks() {
        let mut ctx = crate::ui::screens::tests::ctx_with_aliases(&["demo"]);
        let mut screen = CreateProjectScreen::new_name();
        let c = MouseInput::Click { column: 5, row: 5, double: false };
        assert!(matches!(screen.handle_mouse(c, &mut ctx), ScreenAction::None));
        assert!(matches!(screen.phase, CreatePhase::Name));
    }
}
