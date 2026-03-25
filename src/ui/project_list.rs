use crate::app::App;
use ratatui::Frame;
use ratatui::widgets::Paragraph;

/// Stub rendering — will be fully implemented in Task 2.
pub fn render(frame: &mut Frame, _app: &mut App) {
    let area = frame.area();
    let text = Paragraph::new("GSD Manager - stub rendering");
    frame.render_widget(text, area);
}
