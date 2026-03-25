use crate::app::{App, InputMode};
use crate::registry;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table};
use ratatui::Frame;

pub fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    // Split into main area and footer
    let chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(area);

    let main_area = chunks[0];
    let footer_area = chunks[1];

    render_main(frame, app, main_area);
    render_footer(frame, app, footer_area);
}

fn render_main(frame: &mut Frame, app: &mut App, area: Rect) {
    let outer_block = Block::default()
        .borders(Borders::ALL)
        .title(" GSD Manager ");

    if app.config.projects.is_empty() && app.input_mode == InputMode::Normal {
        // Empty state
        let inner = outer_block.inner(area);
        frame.render_widget(outer_block, area);

        let empty_text = vec![
            Line::from(Span::styled(
                "No projects registered",
                Style::default().add_modifier(Modifier::BOLD),
            )),
            Line::from(""),
            Line::from("Press [a] to add your first GSD project"),
        ];

        let paragraph = Paragraph::new(empty_text)
            .alignment(ratatui::layout::Alignment::Center);

        // Center vertically
        let y_offset = if inner.height > 3 {
            (inner.height - 3) / 2
        } else {
            0
        };
        let centered_area = Rect {
            x: inner.x,
            y: inner.y + y_offset,
            width: inner.width,
            height: 3.min(inner.height),
        };

        frame.render_widget(paragraph, centered_area);
    } else {
        // Project table
        let inner = outer_block.inner(area);
        frame.render_widget(outer_block, area);

        let terminal_width = area.width;
        let show_path = terminal_width >= 60;

        let header_cells = if show_path {
            vec!["Alias", "Phase", "Status", "Path"]
        } else {
            vec!["Alias", "Phase", "Status"]
        };

        let header = Row::new(header_cells)
            .style(Style::default().add_modifier(Modifier::BOLD))
            .bottom_margin(0);

        let widths = if show_path {
            vec![
                Constraint::Percentage(20),
                Constraint::Percentage(15),
                Constraint::Percentage(15),
                Constraint::Percentage(50),
            ]
        } else {
            vec![
                Constraint::Percentage(40),
                Constraint::Percentage(30),
                Constraint::Percentage(30),
            ]
        };

        let aliases = app.sorted_aliases();
        let rows: Vec<Row> = aliases
            .iter()
            .map(|alias| {
                let project = &app.config.projects[alias];
                let state = app.project_states.get(alias);

                let phase_cell = match state {
                    Some(s) => format!("{} of {}", s.completed_phases, s.total_phases),
                    None => "?".to_string(),
                };

                let status_cell = match state {
                    Some(s) if !s.status.is_empty() => s.status.clone(),
                    _ => "unknown".to_string(),
                };

                if show_path {
                    Row::new(vec![
                        alias.clone(),
                        phase_cell,
                        status_cell,
                        project.path.display().to_string(),
                    ])
                } else {
                    Row::new(vec![alias.clone(), phase_cell, status_cell])
                }
            })
            .collect();

        let table = Table::new(rows, &widths)
            .header(header)
            .highlight_style(Style::default().add_modifier(Modifier::REVERSED))
            .highlight_symbol("> ");

        frame.render_stateful_widget(table, inner, &mut app.table_state);
    }
}

fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
    match &app.input_mode {
        InputMode::Normal => {
            // Check for status message first
            if let Some((msg, _)) = &app.status_message {
                let color = if msg.starts_with("Added") {
                    Color::Green
                } else if msg.starts_with("Removed") {
                    Color::Green
                } else {
                    Color::default()
                };
                let line = Line::from(Span::styled(msg.clone(), Style::default().fg(color)));
                frame.render_widget(Paragraph::new(line), area);
            } else {
                // Show keybind hints and project count
                let project_count = app.config.projects.len();
                let hints = format!("[a]dd  [d]elete  [q]uit");
                let count_text = format!("{} project(s) tracked", project_count);

                // Split footer into left and right
                let footer_chunks = Layout::horizontal([
                    Constraint::Min(0),
                    Constraint::Length(count_text.len() as u16 + 1),
                ])
                .split(area);

                let left = Paragraph::new(Line::from(Span::raw(hints)));
                let right = Paragraph::new(Line::from(Span::raw(count_text)))
                    .alignment(ratatui::layout::Alignment::Right);

                frame.render_widget(left, footer_chunks[0]);
                frame.render_widget(right, footer_chunks[1]);
            }
        }
        InputMode::AddAlias => {
            render_input_footer(frame, app, area, "Alias");
        }
        InputMode::AddPath { .. } => {
            render_input_footer(frame, app, area, "Path");
        }
        InputMode::DeleteConfirm { alias } => {
            let prompt = format!(
                "Remove \"{}\"? This only unregisters it \u{2014} project files are not deleted. [y/n]",
                alias
            );
            let line = Line::from(Span::styled(
                prompt,
                Style::default().fg(Color::Red),
            ));
            frame.render_widget(Paragraph::new(line), area);
        }
    }
}

fn render_input_footer(frame: &mut Frame, app: &App, area: Rect, label: &str) {
    let mut spans = vec![
        Span::raw(format!("{}: ", label)),
        Span::styled(
            app.input_buffer.clone(),
            Style::default().add_modifier(Modifier::UNDERLINED),
        ),
        Span::raw("_"),
    ];

    if let Some(err) = &app.error_message {
        spans.push(Span::raw("  "));
        spans.push(Span::styled(
            err.clone(),
            Style::default().fg(Color::Red),
        ));
    }

    let line = Line::from(spans);
    frame.render_widget(Paragraph::new(line), area);
}
