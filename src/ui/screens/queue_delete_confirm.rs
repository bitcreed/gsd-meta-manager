use super::{AppContext, Screen, ScreenAction};
use crate::state_reader::{self, queue_md};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub struct QueueDeleteConfirmScreen {
    pub alias: String,
    pub index: usize,
    pub command_text: String,
}

impl QueueDeleteConfirmScreen {
    pub fn new(alias: String, index: usize, command_text: String) -> Self {
        Self {
            alias,
            index,
            command_text,
        }
    }
}

impl Screen for QueueDeleteConfirmScreen {
    fn handle_key(
        &mut self,
        code: KeyCode,
        _modifiers: KeyModifiers,
        ctx: &mut AppContext,
    ) -> ScreenAction {
        match code {
            KeyCode::Char('y') => {
                if let Some(project) = ctx.config.projects.get(&self.alias) {
                    let planning_dir = project.path.join(".planning");
                    let mut actions = queue_md::load_queue(&planning_dir);
                    if self.index < actions.len() {
                        actions.remove(self.index);
                        if let Err(e) = queue_md::save_queue(&planning_dir, &actions) {
                            ctx.status_message =
                                Some((format!("Queue error: {}", e), std::time::Instant::now()));
                        } else {
                            ctx.status_message = Some((
                                format!("Removed: {}", self.command_text),
                                std::time::Instant::now(),
                            ));
                            // Reload project state
                            let new_state = state_reader::parse_project_state(&planning_dir);
                            ctx.project_states.insert(self.alias.clone(), new_state);
                            // Clamp queue_selected
                            let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                            let new_len = actions.len();
                            if new_len == 0 {
                                cache.queue_selected = 0;
                            } else if cache.queue_selected >= new_len {
                                cache.queue_selected = new_len - 1;
                            }
                        }
                    }
                }
                ctx.needs_redraw = true;
                ScreenAction::Pop
            }
            KeyCode::Char('n') | KeyCode::Esc => {
                ctx.needs_redraw = true;
                ScreenAction::Pop
            }
            _ => ScreenAction::None,
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        let chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(area);
        let footer_area = chunks[1];

        // Render the detail view content in the background
        let detail = super::detail::DetailScreen::new(self.alias.clone());
        detail.render_main_only(frame, chunks[0], ctx);

        // Red confirmation prompt in footer
        let display_text = if self.command_text.len() > 50 {
            format!("{}...", &self.command_text[..47])
        } else {
            self.command_text.clone()
        };
        let prompt = format!("  Remove \"{}\" from queue? [y/n]", display_text,);
        let line = Line::from(Span::styled(
            prompt,
            Style::default().fg(Color::Red).add_modifier(Modifier::BOLD),
        ));
        frame.render_widget(Paragraph::new(line), footer_area);
    }

    fn name(&self) -> &str {
        "queue_delete_confirm"
    }
}
