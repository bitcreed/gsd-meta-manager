use super::{AppContext, Screen, ScreenAction};
use super::detail::DetailScreen;
use super::add_project::AddProjectScreen;
use super::create_project::CreateProjectScreen;
use super::delete_confirm::DeleteConfirmScreen;
use super::help::HelpScreen;
use crate::app::{classify_status, format_phase_display, StatusCategory};
use crate::state_reader::disk_status::{DiskInference, DiskStatus};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::{Alignment, Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Row, Table};
use ratatui::Frame;

pub struct NormalScreen {
    pub searching: bool,
}

impl NormalScreen {
    pub fn new() -> Self {
        Self { searching: false }
    }
}

fn status_color(status: &str) -> Color {
    match classify_status(status) {
        StatusCategory::Active => Color::Green,
        StatusCategory::Idle => Color::Yellow,
        StatusCategory::Blocked => Color::Red,
        StatusCategory::Complete => Color::DarkGray,
        StatusCategory::Unknown => Color::Magenta,
    }
}

/// Return the DiskStatus one step below the given threshold.
fn prev_status(threshold: DiskStatus) -> DiskStatus {
    match threshold {
        DiskStatus::Discussed => DiskStatus::Empty,
        DiskStatus::Researched => DiskStatus::Discussed,
        DiskStatus::Planned => DiskStatus::Researched,
        DiskStatus::Partial => DiskStatus::Planned,
        DiskStatus::Complete => DiskStatus::Partial,
        _ => DiskStatus::NoDirectory,
    }
}

/// Render the compact D-R-P-E-V pipeline for unfocused dashboard rows.
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
                Color::Green
            } else if *status == prev_status(*threshold) {
                Color::Yellow
            } else {
                Color::DarkGray
            };
            Span::styled(format!(" {} ", label), Style::default().fg(color))
        })
        .collect();

    Line::from(spans)
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

impl Screen for NormalScreen {
    fn handle_key(&mut self, code: KeyCode, _modifiers: KeyModifiers, ctx: &mut AppContext) -> ScreenAction {
        if self.searching {
            return self.handle_search_key(code, ctx);
        }

        match code {
            KeyCode::Char('q') => ScreenAction::Quit,
            KeyCode::Char('j') | KeyCode::Down => {
                move_selection_down(ctx);
                ctx.needs_redraw = true;
                ScreenAction::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                move_selection_up(ctx);
                ctx.needs_redraw = true;
                ScreenAction::None
            }
            KeyCode::Char('a') => {
                ctx.input_buffer.clear();
                ctx.error_message = None;
                ctx.needs_redraw = true;
                ScreenAction::Push(Box::new(AddProjectScreen::new_alias()))
            }
            KeyCode::Char('c') => {
                ctx.input_buffer.clear();
                ctx.error_message = None;
                ctx.needs_redraw = true;
                ScreenAction::Push(Box::new(CreateProjectScreen::new_name()))
            }
            KeyCode::Char('d') => {
                if let Some(alias) = ctx.selected_alias() {
                    ctx.needs_redraw = true;
                    ScreenAction::Push(Box::new(DeleteConfirmScreen::new(alias)))
                } else {
                    ScreenAction::None
                }
            }
            KeyCode::Enter => {
                if let Some(alias) = ctx.selected_alias() {
                    ctx.detail_scroll_offset = 0;
                    ctx.needs_redraw = true;
                    ScreenAction::Push(Box::new(DetailScreen::new(alias)))
                } else {
                    ScreenAction::None
                }
            }
            KeyCode::Char('/') => {
                self.searching = true;
                ctx.input_buffer.clear();
                ctx.filter_text.clear();
                ctx.recompute_filtered_aliases();
                ctx.needs_redraw = true;
                ScreenAction::None
            }
            KeyCode::Char('?') => {
                ctx.needs_redraw = true;
                ScreenAction::Push(Box::new(HelpScreen))
            }
            _ => ScreenAction::None,
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        // Minimum terminal size guard
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

        self.render_main(frame, main_area, ctx);
        self.render_footer(frame, footer_area, ctx);
    }

    fn name(&self) -> &str {
        "normal"
    }
}

impl NormalScreen {
    fn handle_search_key(&mut self, code: KeyCode, ctx: &mut AppContext) -> ScreenAction {
        match code {
            KeyCode::Char(c) => {
                ctx.filter_text.push(c);
                ctx.recompute_filtered_aliases();
                if !ctx.filtered_aliases.is_empty() {
                    ctx.table_state.select(Some(0));
                } else {
                    ctx.table_state.select(None);
                }
                ctx.needs_redraw = true;
                ScreenAction::None
            }
            KeyCode::Backspace => {
                ctx.filter_text.pop();
                ctx.recompute_filtered_aliases();
                if !ctx.filtered_aliases.is_empty() {
                    ctx.table_state.select(Some(0));
                } else {
                    ctx.table_state.select(None);
                }
                ctx.needs_redraw = true;
                ScreenAction::None
            }
            KeyCode::Esc => {
                self.searching = false;
                ctx.filter_text.clear();
                ctx.recompute_filtered_aliases();
                if !ctx.filtered_aliases.is_empty() {
                    ctx.table_state.select(Some(0));
                } else {
                    ctx.table_state.select(None);
                }
                ctx.needs_redraw = true;
                ScreenAction::None
            }
            KeyCode::Enter => {
                // Confirm filter and return to normal mode (filter stays active)
                self.searching = false;
                ctx.needs_redraw = true;
                ScreenAction::None
            }
            _ => ScreenAction::None,
        }
    }

    fn render_main(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        let outer_block = Block::default()
            .borders(Borders::ALL)
            .title(" GSD Manager ");

        if ctx.config.projects.is_empty() && !self.searching {
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

            let (header_cells, widths) = if terminal_width >= 80 {
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

            let selected_idx = ctx.table_state.selected();

            let rows: Vec<Row> = ctx
                .filtered_aliases
                .iter()
                .enumerate()
                .map(|(idx, alias)| {
                    let state = ctx.project_states.get(alias);
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

                    // Build status cell: milestone complete, pipeline, or expanded
                    let is_milestone_complete = match state {
                        Some(s) => s.completed_phases >= s.total_phases && s.total_phases > 0,
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

            // We need a mutable table_state for rendering
            let mut table_state = ctx.table_state.clone();
            frame.render_stateful_widget(table, inner, &mut table_state);
            // Note: table_state selection is managed by ctx directly through handle_key
        }
    }

    fn render_footer(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        if self.searching {
            render_search_footer(frame, area, ctx);
            return;
        }

        // Check for status message first
        if let Some((msg, _)) = &ctx.status_message {
            let color = if msg.starts_with("Added") || msg.starts_with("Removed") {
                Color::Green
            } else {
                Color::default()
            };
            let line = Line::from(Span::styled(msg.clone(), Style::default().fg(color)));
            frame.render_widget(Paragraph::new(line), area);
        } else {
            render_normal_footer(frame, area, ctx);
        }
    }
}

fn render_normal_footer(frame: &mut Frame, area: Rect, ctx: &AppContext) {
    let all_count = ctx.config.projects.len();
    let mut active = 0u32;
    let mut blocked = 0u32;
    let mut idle = 0u32;
    let mut complete = 0u32;

    for state in ctx.project_states.values() {
        match classify_status(&state.status) {
            StatusCategory::Active => active += 1,
            StatusCategory::Blocked => blocked += 1,
            StatusCategory::Idle => idle += 1,
            StatusCategory::Complete => complete += 1,
            StatusCategory::Unknown => {}
        }
    }

    let left_text = format!(
        "{} projects: {} > {} ! {} * {} +",
        all_count, active, blocked, idle, complete
    );

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

fn render_search_footer(frame: &mut Frame, area: Rect, ctx: &AppContext) {
    let left_spans = vec![
        Span::raw("/ "),
        Span::styled(
            ctx.filter_text.clone(),
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

fn move_selection_down(ctx: &mut AppContext) {
    let count = ctx.filtered_aliases.len();
    if count == 0 {
        return;
    }
    let current = ctx.table_state.selected().unwrap_or(0);
    let next = if current >= count - 1 { 0 } else { current + 1 };
    ctx.table_state.select(Some(next));
}

fn move_selection_up(ctx: &mut AppContext) {
    let count = ctx.filtered_aliases.len();
    if count == 0 {
        return;
    }
    let current = ctx.table_state.selected().unwrap_or(0);
    let next = if current == 0 { count - 1 } else { current - 1 };
    ctx.table_state.select(Some(next));
}
