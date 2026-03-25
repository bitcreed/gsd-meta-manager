pub mod help_overlay;
pub mod project_list;

use crate::app::{App, InputMode};
use ratatui::Frame;

pub fn render(frame: &mut Frame, app: &mut App) {
    project_list::render(frame, app);

    // Draw help overlay on top if active
    if app.input_mode == InputMode::HelpOverlay {
        help_overlay::render(frame);
    }
}
