use super::{AppContext, Screen, ScreenAction};
use crate::state_reader::{self, queue_md};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub struct EnqueueScreen {
    pub alias: String,
}

impl EnqueueScreen {
    pub fn new(alias: String) -> Self {
        Self { alias }
    }
}

impl Screen for EnqueueScreen {
    fn handle_key(
        &mut self,
        code: KeyCode,
        _modifiers: KeyModifiers,
        ctx: &mut AppContext,
    ) -> ScreenAction {
        match code {
            KeyCode::Enter => {
                if !ctx.input_buffer.is_empty() {
                    if let Some(project) = ctx.config.projects.get(&self.alias) {
                        let planning_dir = project.path.join(".planning");
                        let mut actions = queue_md::load_queue(&planning_dir);
                        actions.push(queue_md::QueuedAction {
                            command: ctx.input_buffer.clone(),
                        });
                        if let Err(e) = queue_md::save_queue(&planning_dir, &actions) {
                            ctx.status_message =
                                Some((format!("Queue error: {}", e), std::time::Instant::now()));
                        } else {
                            ctx.status_message = Some((
                                format!("Queued: {}", ctx.input_buffer),
                                std::time::Instant::now(),
                            ));
                            // Reload project state so queued_actions is updated
                            let new_state = state_reader::parse_project_state(&planning_dir);
                            ctx.project_states.insert(self.alias.clone(), new_state);
                        }
                    }
                    ctx.input_buffer.clear();
                    ctx.needs_redraw = true;
                    ScreenAction::Pop
                } else {
                    ScreenAction::None
                }
            }
            KeyCode::Tab => {
                if ctx.config.projects.contains_key(&self.alias) {
                    if let Some(state) = ctx.project_states.get(&self.alias) {
                        let suggestions = queue_md::suggest_next_commands(state);
                        if !suggestions.is_empty() {
                            ctx.suggestion_index = (ctx.suggestion_index + 1) % suggestions.len();
                            ctx.input_buffer = suggestions[ctx.suggestion_index].clone();
                            ctx.needs_redraw = true;
                        }
                    }
                }
                ScreenAction::None
            }
            KeyCode::Esc => {
                ctx.input_buffer.clear();
                ctx.needs_redraw = true;
                ScreenAction::Pop
            }
            KeyCode::Backspace => {
                ctx.input_buffer.pop();
                ctx.suggestion_index = 0;
                ctx.needs_redraw = true;
                ScreenAction::None
            }
            KeyCode::Char(c) => {
                ctx.input_buffer.push(c);
                ctx.suggestion_index = 0;
                ctx.needs_redraw = true;
                ScreenAction::None
            }
            _ => ScreenAction::None,
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        // The enqueue screen renders over the detail view.
        // We delegate the main area to the detail screen's render but override the footer.
        // For simplicity, we render the detail view content + our enqueue footer.

        let chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(area);
        let footer_area = chunks[1];

        // Render the detail view content in main area
        // We reuse the detail screen rendering by creating a temporary one
        let detail = super::detail::DetailScreen::new(self.alias.clone());
        // Render detail main area only (we handle footer)
        let main_chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(area);
        detail.render_main_only(frame, main_chunks[0], ctx);

        // Enqueue input footer.
        //
        // Read, not queued: the command written to `.planning/queue.md` is
        // `ctx.input_buffer` raw. Only the echo is escaped — and this field is
        // not always something the operator typed, because `Tab` fills it from
        // `queue_md::suggest_next_commands`, a function of the project's parsed
        // `.planning/` state.
        let footer = Paragraph::new(Line::from(vec![
            Span::styled("  Enqueue> ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(crate::text::display_identity(&ctx.input_buffer)),
            Span::styled(
                "  [Tab] suggestions  [Enter] queue  [Esc] cancel",
                Style::default().fg(Color::DarkGray),
            ),
        ]));
        frame.render_widget(footer, footer_area);
    }

    fn name(&self) -> &str {
        "enqueue"
    }
}
