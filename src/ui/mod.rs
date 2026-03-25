pub mod project_list;

use crate::app::App;
use ratatui::Frame;

pub fn render(frame: &mut Frame, app: &mut App) {
    project_list::render(frame, app);
}
