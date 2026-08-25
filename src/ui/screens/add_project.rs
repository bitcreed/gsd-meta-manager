use super::{AppContext, Screen, ScreenAction};
use crate::config::save_config;
use crate::registry;
use crate::state_reader;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;
use std::path::Path;
use std::path::PathBuf;

enum AddPhase {
    Alias,
    Path { alias: String },
}

pub struct AddProjectScreen {
    phase: AddPhase,
}

impl AddProjectScreen {
    pub fn new_alias() -> Self {
        Self {
            phase: AddPhase::Alias,
        }
    }
}

impl Screen for AddProjectScreen {
    fn handle_key(
        &mut self,
        code: KeyCode,
        _modifiers: KeyModifiers,
        ctx: &mut AppContext,
    ) -> ScreenAction {
        match &self.phase {
            AddPhase::Alias => self.handle_alias_key(code, ctx),
            AddPhase::Path { alias } => {
                let alias = alias.clone();
                self.handle_path_key(code, ctx, &alias)
            }
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        // This screen renders only a footer - the main area shows the normal screen behind
        // Since we're on the stack, the normal screen won't render. We need to render footer only.
        // Actually, for add_project, the old code showed the project list with a footer prompt.
        // The simplest approach: render just the footer area in the last row.
        // The parent NormalScreen is not visible (screen stack renders only top).
        // So we render the project list ourselves + our footer.

        // For simplicity, delegate main rendering to the normal screen pattern
        // but just render the footer prompt.
        let chunks = ratatui::layout::Layout::vertical([
            ratatui::layout::Constraint::Min(0),
            ratatui::layout::Constraint::Length(1),
        ])
        .split(area);

        // Render a minimal project-list block in the main area.
        //
        // This used to say "the old code rendered project_list in the
        // background", citing `src/ui/project_list.rs`. That file was deleted in
        // 21-21 — it had been orphaned from the module tree since `c297631` (the
        // commit that introduced the `Screen` trait and `screens/normal.rs`) and
        // the build never compiled it, so the "old code" being deferred to had
        // not shipped for the whole of its citation's life.
        render_project_list_background(frame, chunks[0], ctx);

        // Footer
        let label = match &self.phase {
            AddPhase::Alias => "Alias",
            AddPhase::Path { .. } => "Path",
        };
        render_input_footer(frame, chunks[1], ctx, label);
    }

    fn name(&self) -> &str {
        "add_project"
    }
}

impl AddProjectScreen {
    fn handle_alias_key(&mut self, code: KeyCode, ctx: &mut AppContext) -> ScreenAction {
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
                let alias = ctx.input_buffer.trim().to_string();
                if alias.is_empty() {
                    ctx.error_message = Some("Alias cannot be empty.".to_string());
                    ctx.needs_redraw = true;
                    return ScreenAction::None;
                }
                if alias.contains(char::is_whitespace) {
                    ctx.error_message = Some("Alias cannot contain whitespace.".to_string());
                    ctx.needs_redraw = true;
                    return ScreenAction::None;
                }
                if ctx.config.projects.contains_key(&alias) {
                    ctx.error_message = Some(format!(
                        "Alias \"{}\" already exists. Choose a different alias.",
                        alias
                    ));
                    ctx.needs_redraw = true;
                    return ScreenAction::None;
                }
                self.phase = AddPhase::Path { alias };
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

    fn handle_path_key(
        &mut self,
        code: KeyCode,
        ctx: &mut AppContext,
        alias: &str,
    ) -> ScreenAction {
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
                let path = PathBuf::from(ctx.input_buffer.trim());
                do_add_project(ctx, alias, &path);
                ScreenAction::Pop
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
}

fn do_add_project(ctx: &mut AppContext, alias: &str, path: &Path) {
    let canonical = path.canonicalize().unwrap_or_else(|_| path.to_path_buf());

    // The TUI registration path gets the SAME refusal as the CLI, with the same
    // message (D-17-2). A screen that admitted an alias the command line refuses
    // would be the second registration route, which is how a fourth predicate
    // came to exist in the first place.
    let judged = match registry::Alias::new(alias) {
        Ok(judged) => judged,
        Err(refusal) => {
            ctx.error_message = Some(refusal.to_string());
            ctx.needs_redraw = true;
            return;
        }
    };

    match registry::add_project(&mut ctx.config, &judged, &canonical) {
        Ok(()) => {
            if let Err(e) = save_config(&ctx.config, &ctx.config_path) {
                ctx.error_message = Some(format!("Failed to save config: {}", e));
                ctx.needs_redraw = true;
                return;
            }
            let planning_dir = canonical.join(".planning");
            let state = state_reader::parse_project_state(&planning_dir);
            ctx.project_states.insert(alias.to_string(), state);

            ctx.status_message = Some((format!("Added \"{}\"", alias), std::time::Instant::now()));
            ctx.input_buffer.clear();
            ctx.error_message = None;

            ctx.recompute_filtered_aliases();
            if let Some(pos) = ctx.filtered_aliases.iter().position(|a| a == alias) {
                ctx.table_state.select(Some(pos));
            }
            ctx.needs_redraw = true;
        }
        Err(e) => {
            ctx.error_message = Some(e.to_string());
            ctx.needs_redraw = true;
        }
    }
}

fn render_project_list_background(frame: &mut Frame, area: Rect, _ctx: &AppContext) {
    use ratatui::widgets::{Block, Borders};

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" GSD Manager ");
    frame.render_widget(block, area);
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
        spans.push(Span::styled(err.clone(), Style::default().fg(Color::Red)));
    }

    let line = Line::from(spans);
    frame.render_widget(Paragraph::new(line), area);
}
