pub mod detail_view;
pub mod help_overlay;
pub mod project_list;

use crate::app::{App, InputMode};
use ratatui::Frame;

pub fn render(frame: &mut Frame, app: &mut App) {
    match &app.input_mode {
        InputMode::DetailView { .. } => {
            detail_view::render(frame, app);
        }
        _ => {
            project_list::render(frame, app);
            if app.input_mode == InputMode::HelpOverlay {
                help_overlay::render(frame);
            }
        }
    }
}
