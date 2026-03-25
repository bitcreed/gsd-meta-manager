use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};
use ratatui::Frame;

fn centered_rect(area: Rect, pct_width: u16, pct_height: u16) -> Rect {
    let width = (area.width as u32 * pct_width as u32 / 100) as u16;
    let height = (area.height as u32 * pct_height as u32 / 100) as u16;
    let x = area.x + (area.width.saturating_sub(width)) / 2;
    let y = area.y + (area.height.saturating_sub(height)) / 2;
    Rect::new(x, y, width, height)
}

pub fn render(frame: &mut Frame) {
    let area = centered_rect(frame.area(), 60, 70);
    frame.render_widget(Clear, area);

    let help_text = vec![
        Line::from(Span::styled(
            "Keybindings",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("  j / Down      Move down"),
        Line::from("  k / Up        Move up"),
        Line::from("  Enter         Open project detail"),
        Line::from("  /             Filter projects"),
        Line::from("  a             Add project"),
        Line::from("  d             Delete project"),
        Line::from("  ?             Toggle this help"),
        Line::from("  q / Esc       Quit / Back"),
        Line::from("  Ctrl+C        Force quit"),
        Line::from(""),
        Line::from(Span::styled(
            "Filter Syntax",
            Style::default().add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("  /term         Search all columns"),
        Line::from("  /term/n       Search name only"),
        Line::from("  /term/p       Search phase only"),
        Line::from("  /term/s       Search status only"),
        Line::from(""),
        Line::from(Span::styled(
            "Press ? or Esc to close",
            Style::default().add_modifier(Modifier::DIM),
        )),
    ];

    let block = Block::default()
        .borders(Borders::ALL)
        .title(" Help ");
    let paragraph = Paragraph::new(help_text).block(block);
    frame.render_widget(paragraph, area);
}
