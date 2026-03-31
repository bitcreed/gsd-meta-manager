use super::{AppContext, Screen, ScreenAction};
use crate::config::save_config;
use crate::registry;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

pub struct DeleteConfirmScreen {
    pub alias: String,
}

impl DeleteConfirmScreen {
    pub fn new(alias: String) -> Self {
        Self { alias }
    }
}

impl Screen for DeleteConfirmScreen {
    fn handle_key(
        &mut self,
        code: KeyCode,
        _modifiers: KeyModifiers,
        ctx: &mut AppContext,
    ) -> ScreenAction {
        match code {
            KeyCode::Char('y') => {
                do_remove_project(ctx, &self.alias);
                ScreenAction::Pop
            }
            KeyCode::Char('n') | KeyCode::Esc => {
                ctx.needs_redraw = true;
                ScreenAction::Pop
            }
            _ => ScreenAction::None,
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, _ctx: &AppContext) {
        let chunks = ratatui::layout::Layout::vertical([
            ratatui::layout::Constraint::Min(0),
            ratatui::layout::Constraint::Length(1),
        ])
        .split(area);

        // Render project list background
        use ratatui::widgets::{Block, Borders};
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" GSD Manager ");
        frame.render_widget(block, chunks[0]);

        // Footer with delete confirmation
        let prompt = format!(
            "Remove \"{}\"? This only unregisters it \u{2014} project files are not deleted. [y/n]",
            self.alias
        );
        let line = Line::from(Span::styled(prompt, Style::default().fg(Color::Red)));
        frame.render_widget(Paragraph::new(line), chunks[1]);
    }

    fn name(&self) -> &str {
        "delete_confirm"
    }
}

fn do_remove_project(ctx: &mut AppContext, alias: &str) {
    let project_path = ctx.config.projects.get(alias).map(|p| p.path.clone());

    match registry::remove_project(&mut ctx.config, alias) {
        Ok(()) => {
            if let Err(e) = save_config(&ctx.config, &ctx.config_path) {
                ctx.error_message = Some(format!("Failed to save config: {}", e));
                ctx.needs_redraw = true;
                return;
            }

            // Unwatch the project's .planning/ directory
            if let Some(ref path) = project_path {
                if let Some(ref mut watcher) = ctx.watcher {
                    let planning_dir = path.join(".planning");
                    let _ = watcher.unwatch(&planning_dir);
                }
            }

            ctx.project_states.remove(alias);
            ctx.detail_sub_view_per_project.remove(alias);
            ctx.last_refresh.remove(alias);

            ctx.status_message =
                Some((format!("Removed \"{}\"", alias), std::time::Instant::now()));

            ctx.recompute_filtered_aliases();
            if ctx.filtered_aliases.is_empty() {
                ctx.table_state.select(None);
            } else {
                let selected = ctx
                    .table_state
                    .selected()
                    .unwrap_or(0)
                    .min(ctx.filtered_aliases.len() - 1);
                ctx.table_state.select(Some(selected));
            }
            ctx.needs_redraw = true;
        }
        Err(e) => {
            ctx.error_message = Some(e.to_string());
            ctx.needs_redraw = true;
        }
    }
}
