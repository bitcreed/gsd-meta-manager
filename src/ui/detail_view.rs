use crate::app::{classify_status, App, InputMode, StatusCategory};
use crate::change_tracker::ChangeTracker;
use ratatui::layout::{Constraint, Layout};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

fn status_color(category: &StatusCategory) -> Color {
    match category {
        StatusCategory::Active => Color::Green,
        StatusCategory::Idle => Color::Yellow,
        StatusCategory::Blocked => Color::Red,
        StatusCategory::Complete => Color::DarkGray,
        StatusCategory::Unknown => Color::Magenta,
    }
}

pub fn render(frame: &mut Frame, app: &mut App) {
    let alias = match &app.input_mode {
        InputMode::DetailView { alias } => alias.clone(),
        _ => return,
    };

    let area = frame.area();

    // Split into main content and footer
    let chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(area);
    let main_area = chunks[0];
    let footer_area = chunks[1];

    let state = app.project_states.get(&alias);
    let project_path = app
        .config
        .projects
        .get(&alias)
        .map(|p| p.path.display().to_string())
        .unwrap_or_else(|| "unknown".to_string());

    let mut lines: Vec<Line> = Vec::new();

    // Path line
    lines.push(Line::from(vec![
        Span::styled("  Path: ", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw(&project_path),
    ]));

    if let Some(state) = state {
        // Status + Milestone line
        let cat = classify_status(&state.status);
        let color = status_color(&cat);
        lines.push(Line::from(vec![
            Span::styled("  Status: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::styled(&state.status, Style::default().fg(color)),
            Span::raw("    "),
            Span::styled("Milestone: ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(&state.milestone),
        ]));

        // Blank line
        lines.push(Line::from(""));

        // Change banner (per D-04)
        if let Some(event) = app.change_tracker.latest_change(&alias) {
            let elapsed = ChangeTracker::format_elapsed(event.timestamp);
            let banner = format!("  [ {} -- {} ]", event.description, elapsed);
            lines.push(Line::from(Span::styled(
                banner,
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )));
            lines.push(Line::from(""));
        }

        // Phases header
        lines.push(Line::from(Span::styled(
            "  Phases:",
            Style::default().add_modifier(Modifier::BOLD),
        )));

        if state.phases.is_empty() {
            lines.push(Line::from("  No roadmap data available"));
        } else {
            // Determine current phase number from completed_phases
            let current_phase_num = (state.completed_phases + 1).to_string();

            for phase in &state.phases {
                // Determine phase status
                let (icon, is_current) = if phase.completed {
                    ("+", false)
                } else if phase.number == current_phase_num {
                    ("*", true)
                } else {
                    ("o", false)
                };

                // Plan count display
                let plan_display = if phase.total_plans == 0 {
                    "0/? plans".to_string()
                } else {
                    format!("{}/{} plans", phase.completed_plans, phase.total_plans)
                };

                let line_text = format!(
                    "  {} P{}: {}  {}",
                    icon, phase.number, phase.name, plan_display
                );

                if is_current {
                    let cat = classify_status(&state.status);
                    let color = status_color(&cat);
                    lines.push(Line::from(Span::styled(
                        line_text,
                        Style::default()
                            .fg(color)
                            .add_modifier(Modifier::BOLD),
                    )));
                } else if phase.completed {
                    lines.push(Line::from(Span::styled(
                        line_text,
                        Style::default().fg(Color::DarkGray),
                    )));
                } else {
                    lines.push(Line::from(Span::raw(line_text)));
                }
            }
        }

        // Blank line
        lines.push(Line::from(""));

        // Backlog line (if > 0)
        if state.backlog_count > 0 {
            lines.push(Line::from(format!(
                "  Backlog: {} items",
                state.backlog_count
            )));
        }
    } else {
        lines.push(Line::from(""));
        lines.push(Line::from("  No state data available for this project."));
    }

    let block = Block::default()
        .borders(Borders::ALL)
        .title(format!(" Project: {} ", alias));

    let paragraph = Paragraph::new(lines)
        .block(block)
        .scroll((app.detail_scroll_offset, 0));

    frame.render_widget(paragraph, main_area);

    // Footer
    let footer = Paragraph::new(Line::from(vec![
        Span::raw("  "),
        Span::styled("[Esc]", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("back  "),
        Span::styled("[j/k]", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("scroll  "),
        Span::styled("[?]", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("help"),
    ]));
    frame.render_widget(footer, footer_area);
}
