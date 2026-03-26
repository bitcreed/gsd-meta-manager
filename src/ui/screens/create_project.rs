use super::{AppContext, Screen, ScreenAction};
use crate::action::Action;
use crate::project_creator;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use std::path::PathBuf;

enum CreatePhase {
    Name,
    Path { name: String },
    Confirm { name: String, path: PathBuf },
}

pub struct CreateProjectScreen {
    phase: CreatePhase,
}

impl CreateProjectScreen {
    pub fn new_name() -> Self {
        Self {
            phase: CreatePhase::Name,
        }
    }
}

impl Screen for CreateProjectScreen {
    fn handle_key(&mut self, code: KeyCode, _modifiers: KeyModifiers, ctx: &mut AppContext) -> ScreenAction {
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
                let prompt = format!(
                    "Create \"{}\" at {}? [y/n]",
                    name,
                    path.display()
                );
                let line = Line::from(Span::styled(prompt, Style::default().fg(Color::Yellow)));
                frame.render_widget(Paragraph::new(line), chunks[1]);
            }
        }
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
        path: &PathBuf,
    ) -> ScreenAction {
        match code {
            KeyCode::Enter | KeyCode::Char('y') => {
                let alias = name.to_lowercase().replace(' ', "-");
                let hooks = ctx.config.preferences.hooks.clone();
                let path_clone = path.clone();
                let name_clone = name.to_string();
                let alias_clone = alias.clone();

                if let Some(tx) = ctx.event_tx.clone() {
                    tokio::task::spawn_blocking(move || {
                        let result = project_creator::create_project(
                            &name_clone,
                            &path_clone,
                            &hooks,
                        );
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
                    ctx.error_message =
                        Some("Event channel not available.".to_string());
                    ctx.needs_redraw = true;
                    ScreenAction::None
                }
            }
            KeyCode::Char('n') | KeyCode::Esc => {
                ctx.input_buffer.clear();
                ctx.error_message = None;
                ctx.needs_redraw = true;
                ScreenAction::Pop
            }
            _ => ScreenAction::None,
        }
    }
}

fn render_input_footer(frame: &mut Frame, area: Rect, ctx: &AppContext, label: &str) {
    let mut spans = vec![
        Span::raw(format!("{}: ", label)),
        Span::styled(
            ctx.input_buffer.clone(),
            Style::default().add_modifier(Modifier::UNDERLINED),
        ),
        Span::raw("_"),
    ];

    if let Some(err) = &ctx.error_message {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            err.clone(),
            Style::default().fg(Color::Red),
        ));
    }

    let line = Line::from(spans);
    frame.render_widget(Paragraph::new(line), area);
}
