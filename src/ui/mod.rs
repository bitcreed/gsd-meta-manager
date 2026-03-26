pub mod roadmap_widget;
pub mod screens;

use crate::app::App;
use ratatui::Frame;

pub fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    if let Some(screen) = app.screen_stack.last() {
        screen.render(frame, area, &app.ctx);
    }
}
