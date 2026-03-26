use super::{AppContext, Screen, ScreenAction};
use super::enqueue::EnqueueScreen;
use super::help::HelpScreen;
use crate::app::{classify_status, DetailSubView, StatusCategory};
use crate::state_reader::disk_status::DiskStatus;
use crate::change_tracker::ChangeTracker;
use crate::state_reader::queue_md;
use crate::ui::roadmap_widget::RoadmapWidget;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

pub struct DetailScreen {
    pub alias: String,
    pub scroll_offset: u16,
}

impl DetailScreen {
    pub fn new(alias: String) -> Self {
        Self {
            alias,
            scroll_offset: 0,
        }
    }
}

fn status_color(category: &StatusCategory) -> Color {
    match category {
        StatusCategory::Active => Color::Green,
        StatusCategory::Idle => Color::Yellow,
        StatusCategory::Blocked => Color::Red,
        StatusCategory::Complete => Color::DarkGray,
        StatusCategory::Unknown => Color::Magenta,
    }
}

/// Compute a disk-inferred status suffix for a phase line, e.g. " [Executing 2/3]".
fn disk_suffix(phase_number: &str, phase_disk_statuses: &std::collections::HashMap<String, crate::state_reader::disk_status::DiskInference>) -> String {
    let disk_inf = phase_disk_statuses.get(phase_number);
    let disk_label = match disk_inf {
        Some(inf) => match inf.status {
            DiskStatus::NoDirectory => "Not started",
            DiskStatus::Empty => "Empty",
            DiskStatus::Discussed => "Discussed",
            DiskStatus::Researched => "Researched",
            DiskStatus::Planned => "Planned",
            DiskStatus::Partial => "Executing",
            DiskStatus::Complete => "Complete",
        },
        None => "",
    };

    if disk_label.is_empty() {
        String::new()
    } else if disk_label == "Executing" {
        match disk_inf {
            Some(inf) if inf.plan_count > 0 => {
                format!(" [Executing {}/{}]", inf.summary_count, inf.plan_count)
            }
            _ => format!(" [{}]", disk_label),
        }
    } else if disk_label == "Planned" {
        match disk_inf {
            Some(inf) if inf.plan_count > 0 => {
                format!(" [Planned ({} plans)]", inf.plan_count)
            }
            _ => format!(" [{}]", disk_label),
        }
    } else {
        format!(" [{}]", disk_label)
    }
}

impl Screen for DetailScreen {
    fn handle_key(&mut self, code: KeyCode, _modifiers: KeyModifiers, ctx: &mut AppContext) -> ScreenAction {
        match code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.scroll_offset = 0;
                ctx.needs_redraw = true;
                ScreenAction::Pop
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.scroll_offset = self.scroll_offset.saturating_add(1);
                ctx.needs_redraw = true;
                ScreenAction::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.scroll_offset = self.scroll_offset.saturating_sub(1);
                ctx.needs_redraw = true;
                ScreenAction::None
            }
            KeyCode::Char('r') => {
                let current = ctx
                    .detail_sub_view_per_project
                    .get(&self.alias)
                    .cloned()
                    .unwrap_or_default();
                let next = match current {
                    DetailSubView::PhaseList => DetailSubView::RoadmapViz,
                    DetailSubView::RoadmapViz => DetailSubView::PhaseList,
                };
                ctx.detail_sub_view_per_project
                    .insert(self.alias.clone(), next);
                self.scroll_offset = 0;
                ctx.needs_redraw = true;
                ScreenAction::None
            }
            KeyCode::Char('e') => {
                let alias = self.alias.clone();
                let has_planning = ctx
                    .config
                    .projects
                    .get(&alias)
                    .map(|p| p.path.join(".planning").is_dir())
                    .unwrap_or(false);
                if !has_planning {
                    ctx.needs_redraw = true;
                    ScreenAction::SetStatusMessage(
                        "Run GSD in this project first to enable queue".to_string(),
                    )
                } else {
                    // Pre-populate with first suggestion if available
                    if let Some(state) = ctx.project_states.get(&alias) {
                        let suggestions = queue_md::suggest_next_commands(state);
                        if !suggestions.is_empty() {
                            ctx.input_buffer = suggestions[0].clone();
                        } else {
                            ctx.input_buffer.clear();
                        }
                    } else {
                        ctx.input_buffer.clear();
                    }
                    ctx.suggestion_index = 0;
                    ctx.needs_redraw = true;
                    ScreenAction::Push(Box::new(EnqueueScreen::new(alias)))
                }
            }
            KeyCode::Char('?') => {
                ctx.needs_redraw = true;
                ScreenAction::Push(Box::new(HelpScreen))
            }
            _ => ScreenAction::None,
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        let alias = &self.alias;

        // Split into main content and footer
        let chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(area);
        let main_area = chunks[0];
        let footer_area = chunks[1];

        let state = ctx.project_states.get(alias);
        let project_path = ctx
            .config
            .projects
            .get(alias)
            .map(|p| p.path.display().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let sub_view = ctx
            .detail_sub_view_per_project
            .get(alias)
            .cloned()
            .unwrap_or_default();

        let show_roadmap = sub_view == DetailSubView::RoadmapViz && state.is_some();

        if show_roadmap {
            let state = state.unwrap();

            let mut header_lines: Vec<Line> = Vec::new();
            header_lines.push(Line::from(vec![
                Span::styled("  Path: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&project_path),
            ]));

            let cat = classify_status(&state.status);
            let color = status_color(&cat);
            header_lines.push(Line::from(vec![
                Span::styled("  Status: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(&state.status, Style::default().fg(color)),
                Span::raw("    "),
                Span::styled("Milestone: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&state.milestone),
            ]));

            header_lines.push(Line::from(""));

            if let Some(event) = ctx.change_tracker.latest_change(alias) {
                let elapsed = ChangeTracker::format_elapsed(event.timestamp);
                let banner = format!("  [ {} -- {} ]", event.description, elapsed);
                header_lines.push(Line::from(Span::styled(
                    banner,
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )));
                header_lines.push(Line::from(""));
            }

            let header_height = header_lines.len() as u16 + 2;

            let content_chunks = Layout::vertical([
                Constraint::Length(header_height),
                Constraint::Min(0),
            ])
            .split(main_area);

            let header_area = content_chunks[0];
            let roadmap_area = content_chunks[1];

            let header_block = Block::default()
                .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
                .title(format!(" Project: {} (Roadmap) ", alias));

            let header_paragraph = Paragraph::new(header_lines).block(header_block);
            frame.render_widget(header_paragraph, header_area);

            let current_phase_num = state.completed_phases + 1;
            let roadmap_widget = RoadmapWidget {
                phases: &state.phases,
                current_phase_num,
                scroll_offset: self.scroll_offset,
            };
            frame.render_widget(roadmap_widget, roadmap_area);
        } else {
            // Original phase list rendering
            let mut lines: Vec<Line> = Vec::new();

            lines.push(Line::from(vec![
                Span::styled("  Path: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&project_path),
            ]));

            if let Some(state) = state {
                let cat = classify_status(&state.status);
                let color = status_color(&cat);
                lines.push(Line::from(vec![
                    Span::styled("  Status: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::styled(&state.status, Style::default().fg(color)),
                    Span::raw("    "),
                    Span::styled("Milestone: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(&state.milestone),
                ]));

                lines.push(Line::from(""));

                if let Some(event) = ctx.change_tracker.latest_change(alias) {
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

                lines.push(Line::from(Span::styled(
                    "  Phases:",
                    Style::default().add_modifier(Modifier::BOLD),
                )));

                if state.phases.is_empty() {
                    lines.push(Line::from("  No roadmap data available"));
                } else {
                    let current_phase_num = (state.completed_phases + 1).to_string();

                    for phase in &state.phases {
                        let (icon, is_current) = if phase.completed {
                            ("+", false)
                        } else if phase.number == current_phase_num {
                            ("*", true)
                        } else {
                            ("o", false)
                        };

                        let plan_display = if phase.total_plans == 0 {
                            "0/? plans".to_string()
                        } else {
                            format!("{}/{} plans", phase.completed_plans, phase.total_plans)
                        };

                        let ds = disk_suffix(&phase.number, &state.phase_disk_statuses);

                        let line_text = format!(
                            "  {} P{}: {}  {}{}",
                            icon, phase.number, phase.name, plan_display, ds
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

                lines.push(Line::from(""));

                if state.backlog_count > 0 {
                    lines.push(Line::from(format!(
                        "  Backlog: {} items",
                        state.backlog_count
                    )));
                }

                if !state.queued_actions.is_empty() {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "  Queued:",
                        Style::default().add_modifier(Modifier::BOLD),
                    )));
                    for (i, action) in state.queued_actions.iter().enumerate() {
                        lines.push(Line::from(vec![
                            Span::raw(format!("    {}. ", i + 1)),
                            Span::styled(
                                &action.command,
                                Style::default().fg(Color::Cyan),
                            ),
                        ]));
                    }
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
                .scroll((self.scroll_offset, 0));

            frame.render_widget(paragraph, main_area);
        }

        // Footer
        let toggle_hint = if show_roadmap { "phases" } else { "roadmap" };
        let footer = Paragraph::new(Line::from(vec![
            Span::raw("  "),
            Span::styled("[Esc]", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("back  "),
            Span::styled("[j/k]", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("scroll  "),
            Span::styled("[r]", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(toggle_hint),
            Span::raw("  "),
            Span::styled("[e]", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("enqueue  "),
            Span::styled("[?]", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw("help"),
        ]));
        frame.render_widget(footer, footer_area);
    }

    fn name(&self) -> &str {
        "detail"
    }
}

impl DetailScreen {
    /// Render just the main content area (without footer), used by EnqueueScreen overlay.
    pub fn render_main_only(&self, frame: &mut Frame, main_area: Rect, ctx: &AppContext) {
        let alias = &self.alias;
        let state = ctx.project_states.get(alias);
        let project_path = ctx
            .config
            .projects
            .get(alias)
            .map(|p| p.path.display().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        let sub_view = ctx
            .detail_sub_view_per_project
            .get(alias)
            .cloned()
            .unwrap_or_default();

        let show_roadmap = sub_view == DetailSubView::RoadmapViz && state.is_some();

        if show_roadmap {
            let state = state.unwrap();

            let mut header_lines: Vec<Line> = Vec::new();
            header_lines.push(Line::from(vec![
                Span::styled("  Path: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&project_path),
            ]));

            let cat = classify_status(&state.status);
            let color = status_color(&cat);
            header_lines.push(Line::from(vec![
                Span::styled("  Status: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::styled(&state.status, Style::default().fg(color)),
                Span::raw("    "),
                Span::styled("Milestone: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&state.milestone),
            ]));

            header_lines.push(Line::from(""));

            if let Some(event) = ctx.change_tracker.latest_change(alias) {
                let elapsed = ChangeTracker::format_elapsed(event.timestamp);
                let banner = format!("  [ {} -- {} ]", event.description, elapsed);
                header_lines.push(Line::from(Span::styled(
                    banner,
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )));
                header_lines.push(Line::from(""));
            }

            let header_height = header_lines.len() as u16 + 2;

            let content_chunks = Layout::vertical([
                Constraint::Length(header_height),
                Constraint::Min(0),
            ])
            .split(main_area);

            let header_area = content_chunks[0];
            let roadmap_area = content_chunks[1];

            let header_block = Block::default()
                .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT)
                .title(format!(" Project: {} (Roadmap) ", alias));

            let header_paragraph = Paragraph::new(header_lines).block(header_block);
            frame.render_widget(header_paragraph, header_area);

            let current_phase_num = state.completed_phases + 1;
            let roadmap_widget = RoadmapWidget {
                phases: &state.phases,
                current_phase_num,
                scroll_offset: self.scroll_offset,
            };
            frame.render_widget(roadmap_widget, roadmap_area);
        } else {
            let mut lines: Vec<Line> = Vec::new();

            lines.push(Line::from(vec![
                Span::styled("  Path: ", Style::default().add_modifier(Modifier::BOLD)),
                Span::raw(&project_path),
            ]));

            if let Some(state) = state {
                let cat = classify_status(&state.status);
                let color = status_color(&cat);
                lines.push(Line::from(vec![
                    Span::styled("  Status: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::styled(&state.status, Style::default().fg(color)),
                    Span::raw("    "),
                    Span::styled("Milestone: ", Style::default().add_modifier(Modifier::BOLD)),
                    Span::raw(&state.milestone),
                ]));

                lines.push(Line::from(""));

                if let Some(event) = ctx.change_tracker.latest_change(alias) {
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

                lines.push(Line::from(Span::styled(
                    "  Phases:",
                    Style::default().add_modifier(Modifier::BOLD),
                )));

                if state.phases.is_empty() {
                    lines.push(Line::from("  No roadmap data available"));
                } else {
                    let current_phase_num = (state.completed_phases + 1).to_string();

                    for phase in &state.phases {
                        let (icon, is_current) = if phase.completed {
                            ("+", false)
                        } else if phase.number == current_phase_num {
                            ("*", true)
                        } else {
                            ("o", false)
                        };

                        let plan_display = if phase.total_plans == 0 {
                            "0/? plans".to_string()
                        } else {
                            format!("{}/{} plans", phase.completed_plans, phase.total_plans)
                        };

                        let ds = disk_suffix(&phase.number, &state.phase_disk_statuses);

                        let line_text = format!(
                            "  {} P{}: {}  {}{}",
                            icon, phase.number, phase.name, plan_display, ds
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

                lines.push(Line::from(""));

                if state.backlog_count > 0 {
                    lines.push(Line::from(format!(
                        "  Backlog: {} items",
                        state.backlog_count
                    )));
                }

                if !state.queued_actions.is_empty() {
                    lines.push(Line::from(""));
                    lines.push(Line::from(Span::styled(
                        "  Queued:",
                        Style::default().add_modifier(Modifier::BOLD),
                    )));
                    for (i, action) in state.queued_actions.iter().enumerate() {
                        lines.push(Line::from(vec![
                            Span::raw(format!("    {}. ", i + 1)),
                            Span::styled(
                                &action.command,
                                Style::default().fg(Color::Cyan),
                            ),
                        ]));
                    }
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
                .scroll((self.scroll_offset, 0));

            frame.render_widget(paragraph, main_area);
        }
    }
}
