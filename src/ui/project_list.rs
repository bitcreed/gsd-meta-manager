use crate::app::{classify_status, format_phase_display, App, InputMode, StatusCategory};
use crate::state_reader::disk_status::{DiskInference, DiskStatus};
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table};
use ratatui::Frame;

fn status_color(status: &str) -> Color {
    match classify_status(status) {
        StatusCategory::Active => Color::Green,
        StatusCategory::Idle => Color::Yellow,
        StatusCategory::Blocked => Color::Red,
        StatusCategory::Complete => Color::DarkGray,
        StatusCategory::Unknown => Color::Magenta,
    }
}

/// Render the compact D-R-P-E-V pipeline for unfocused dashboard rows.
/// Each letter is colored: green (completed), yellow (current), dark gray (not reached).
fn compact_pipeline(status: &DiskStatus) -> Line<'static> {
    let stages: [(&str, DiskStatus); 5] = [
        ("D", DiskStatus::Discussed),
        ("R", DiskStatus::Researched),
        ("P", DiskStatus::Planned),
        ("E", DiskStatus::Partial),
        ("V", DiskStatus::Complete),
    ];

    let spans: Vec<Span> = stages
        .iter()
        .map(|(label, threshold)| {
            let color = if *status >= *threshold {
                Color::Green // completed stage
            } else if *status == prev_status(*threshold) {
                Color::Yellow // current stage (one step below threshold)
            } else {
                Color::DarkGray // not reached
            };
            Span::styled(format!(" {} ", label), Style::default().fg(color))
        })
        .collect();

    Line::from(spans)
}

/// Return the DiskStatus one step below the given threshold.
fn prev_status(threshold: DiskStatus) -> DiskStatus {
    match threshold {
        DiskStatus::Discussed => DiskStatus::Empty,
        DiskStatus::Researched => DiskStatus::Discussed,
        DiskStatus::Planned => DiskStatus::Researched,
        DiskStatus::Partial => DiskStatus::Planned,
        DiskStatus::Complete => DiskStatus::Partial,
        // NoDirectory and Empty have no meaningful "previous" for pipeline context
        _ => DiskStatus::NoDirectory,
    }
}

/// Render expanded status text for the focused/selected row.
fn expanded_status(inference: &DiskInference) -> String {
    match inference.status {
        DiskStatus::NoDirectory => "Not started".to_string(),
        DiskStatus::Empty => "Empty".to_string(),
        DiskStatus::Discussed => "Discussed".to_string(),
        DiskStatus::Researched => "Researched".to_string(),
        DiskStatus::Planned => format!("Planned ({} plans)", inference.plan_count),
        DiskStatus::Partial => format!(
            "Executing {}/{}",
            inference.summary_count, inference.plan_count
        ),
        DiskStatus::Complete => "Complete".to_string(),
    }
}

pub fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();

    // Minimum terminal size guard (NAV-04)
    if area.width < 40 || area.height < 8 {
        let msg = Paragraph::new("Terminal too small. Resize to at least 40x8.")
            .alignment(Alignment::Center);
        frame.render_widget(msg, area);
        return;
    }

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

        let paragraph = Paragraph::new(empty_text).alignment(Alignment::Center);

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

        // Adaptive column layout based on terminal width
        let (header_cells, widths) = if terminal_width >= 80 {
            // Full 5-column layout
            (
                vec!["Alias", "Phase", "Status", "Progress", "Backlog"],
                vec![
                    Constraint::Percentage(25),
                    Constraint::Percentage(30),
                    Constraint::Percentage(15),
                    Constraint::Percentage(15),
                    Constraint::Percentage(15),
                ],
            )
        } else if terminal_width >= 60 {
            // 4 columns -- hide Backlog
            (
                vec!["Alias", "Phase", "Status", "Progress"],
                vec![
                    Constraint::Percentage(30),
                    Constraint::Percentage(35),
                    Constraint::Percentage(20),
                    Constraint::Percentage(15),
                ],
            )
        } else {
            // 3 columns -- hide Backlog and Progress
            (
                vec!["Alias", "Phase", "Status"],
                vec![
                    Constraint::Percentage(35),
                    Constraint::Percentage(35),
                    Constraint::Percentage(30),
                ],
            )
        };

        let header = Row::new(header_cells)
            .style(Style::default().add_modifier(Modifier::BOLD))
            .bottom_margin(0);

        let selected_idx = app.table_state.selected();

        let rows: Vec<Row> = app
            .filtered_aliases
            .iter()
            .enumerate()
            .map(|(idx, alias)| {
                let state = app.project_states.get(alias);
                let is_selected = selected_idx == Some(idx);

                let status_str = match state {
                    Some(s) if !s.status.is_empty() => s.status.clone(),
                    _ => "unknown".to_string(),
                };

                let phase_cell = match state {
                    Some(s) => format_phase_display(s),
                    None => "?".to_string(),
                };

                let progress_cell = match state {
                    Some(s) => format!("{}/{} phases", s.completed_phases, s.total_phases),
                    None => "?".to_string(),
                };

                let backlog_cell = match state {
                    Some(s) if s.backlog_count > 0 => s.backlog_count.to_string(),
                    _ => "-".to_string(),
                };

                let row_color = status_color(&status_str);

                // Build status cell: D-03 milestone complete, D-02 pipeline, or expanded
                let is_milestone_complete = match state {
                    Some(s) => {
                        s.completed_phases >= s.total_phases
                            && s.total_phases > 0
                    }
                    None => false,
                };

                let status_cell: Line = if is_milestone_complete {
                    // D-03: Show milestone name for completed milestones
                    let milestone_text = match state {
                        Some(s) if !s.milestone.is_empty() => {
                            format!("{} Complete", s.milestone)
                        }
                        _ => "Complete".to_string(),
                    };
                    Line::from(Span::styled(
                        milestone_text,
                        Style::default().fg(Color::DarkGray),
                    ))
                } else if is_selected {
                    // Focused row: show expanded status text
                    match state.and_then(|s| s.current_phase_status.as_ref()) {
                        Some(inference) => Line::from(Span::styled(
                            expanded_status(inference),
                            Style::default().fg(row_color),
                        )),
                        None => Line::from(Span::styled(
                            status_str.clone(),
                            Style::default().fg(row_color),
                        )),
                    }
                } else {
                    // Unfocused row: show compact pipeline
                    match state.and_then(|s| s.current_phase_status.as_ref()) {
                        Some(inference) => compact_pipeline(&inference.status),
                        None => Line::from(Span::styled(
                            status_str.clone(),
                            Style::default().fg(row_color),
                        )),
                    }
                };

                let cells: Vec<Line> = if terminal_width >= 80 {
                    vec![
                        Line::from(alias.clone()),
                        Line::from(phase_cell),
                        status_cell,
                        Line::from(progress_cell),
                        Line::from(backlog_cell),
                    ]
                } else if terminal_width >= 60 {
                    vec![
                        Line::from(alias.clone()),
                        Line::from(phase_cell),
                        status_cell,
                        Line::from(progress_cell),
                    ]
                } else {
                    vec![
                        Line::from(alias.clone()),
                        Line::from(phase_cell),
                        status_cell,
                    ]
                };

                Row::new(cells).style(Style::default().fg(row_color))
            })
            .collect();

        let table = Table::new(rows, &widths)
            .header(header)
            .row_highlight_style(
                Style::default().add_modifier(Modifier::BOLD | Modifier::UNDERLINED),
            )
            .highlight_symbol("> ");

        frame.render_stateful_widget(table, inner, &mut app.table_state);
    }
}

fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
    match &app.input_mode {
        InputMode::Normal | InputMode::HelpOverlay => {
            // Check for status message first
            if let Some((msg, _)) = &app.status_message {
                let color = if msg.starts_with("Added") || msg.starts_with("Removed") {
                    Color::Green
                } else {
                    Color::default()
                };
                let line = Line::from(Span::styled(msg.clone(), Style::default().fg(color)));
                frame.render_widget(Paragraph::new(line), area);
            } else {
                render_normal_footer(frame, app, area);
            }
        }
        InputMode::Search => {
            render_search_footer(frame, app, area);
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
            let line = Line::from(Span::styled(prompt, Style::default().fg(Color::Red)));
            frame.render_widget(Paragraph::new(line), area);
        }
        InputMode::CreateName => {
            render_input_footer(frame, app, area, "Project name");
        }
        InputMode::CreatePath { .. } => {
            render_input_footer(frame, app, area, "Path (Tab to complete)");
        }
        InputMode::CreateConfirm { name, path } => {
            let prompt = format!(
                "Create \"{}\" at {}? [y/n]",
                name,
                path.display()
            );
            let line = Line::from(Span::styled(prompt, Style::default().fg(Color::Yellow)));
            frame.render_widget(Paragraph::new(line), area);
        }
        InputMode::DetailView { .. } | InputMode::EnqueueInput { .. } => {
            // Detail view renders its own footer; this arm should not be reached
        }
    }
}

fn render_normal_footer(frame: &mut Frame, app: &App, area: Rect) {
    // Left side: aggregate counts using icon shorthand (always from ALL projects)
    let all_count = app.config.projects.len();
    let mut active = 0u32;
    let mut blocked = 0u32;
    let mut idle = 0u32;
    let mut complete = 0u32;

    for state in app.project_states.values() {
        match classify_status(&state.status) {
            StatusCategory::Active => active += 1,
            StatusCategory::Blocked => blocked += 1,
            StatusCategory::Idle => idle += 1,
            StatusCategory::Complete => complete += 1,
            StatusCategory::Unknown => {} // not counted in shorthand
        }
    }

    let left_text = format!(
        "{} projects: {} > {} ! {} * {} +",
        all_count, active, blocked, idle, complete
    );

    // Right side: keybind hints
    let right_text = "[/]search [?]help [a]dd [c]reate [d]el [q]uit";

    let footer_chunks = Layout::horizontal([
        Constraint::Min(0),
        Constraint::Length(right_text.len() as u16 + 1),
    ])
    .split(area);

    let left = Paragraph::new(Line::from(Span::raw(left_text)));
    let right =
        Paragraph::new(Line::from(Span::raw(right_text))).alignment(Alignment::Right);

    frame.render_widget(left, footer_chunks[0]);
    frame.render_widget(right, footer_chunks[1]);
}

fn render_search_footer(frame: &mut Frame, app: &App, area: Rect) {
    let left_spans = vec![
        Span::raw("/ "),
        Span::styled(
            app.filter_text.clone(),
            Style::default().add_modifier(Modifier::UNDERLINED),
        ),
        Span::raw("_"),
    ];

    let right_text = "[Esc]clear [Enter]keep";

    let footer_chunks = Layout::horizontal([
        Constraint::Min(0),
        Constraint::Length(right_text.len() as u16 + 1),
    ])
    .split(area);

    let left = Paragraph::new(Line::from(left_spans));
    let right =
        Paragraph::new(Line::from(Span::raw(right_text))).alignment(Alignment::Right);

    frame.render_widget(left, footer_chunks[0]);
    frame.render_widget(right, footer_chunks[1]);
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
