use super::{AppContext, Screen, ScreenAction};
use super::enqueue::EnqueueScreen;
use super::help::HelpScreen;
use crate::action::Action;
use crate::app::{classify_status, DetailSubView, StatusCategory};
use crate::state_reader::backlog;
use crate::state_reader::disk_status::DiskStatus;
use crate::state_reader::git_ops;
use crate::change_tracker::ChangeTracker;
use crate::state_reader::queue_md;
use crate::ui::roadmap_widget::RoadmapWidget;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Tabs};
use ratatui::Frame;

const TAB_TITLES: [&str; 4] = ["1:Phases", "2:Roadmap", "3:Backlog", "4:Git"];

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

fn tab_index(sub_view: &DetailSubView) -> usize {
    match sub_view {
        DetailSubView::PhaseList => 0,
        DetailSubView::RoadmapViz => 1,
        DetailSubView::Backlog => 2,
        DetailSubView::GitHistory => 3,
    }
}

fn sub_view_from_index(index: usize) -> DetailSubView {
    match index {
        0 => DetailSubView::PhaseList,
        1 => DetailSubView::RoadmapViz,
        2 => DetailSubView::Backlog,
        3 => DetailSubView::GitHistory,
        _ => DetailSubView::PhaseList,
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

/// Switch to a new tab, handling scroll reset and data loading for backlog/git tabs.
fn switch_to_tab(
    alias: &str,
    new_index: usize,
    scroll_offset: &mut u16,
    ctx: &mut AppContext,
) -> ScreenAction {
    let new_view = sub_view_from_index(new_index);
    ctx.detail_sub_view_per_project
        .insert(alias.to_string(), new_view.clone());
    *scroll_offset = 0;
    ctx.needs_redraw = true;

    // Load data for backlog tab synchronously (fast filesystem reads)
    if new_view == DetailSubView::Backlog {
        let cache = ctx.view_cache.entry(alias.to_string()).or_default();
        if cache.backlog_items.is_empty() && !cache.loading_backlog {
            if let Some(project) = ctx.config.projects.get(alias) {
                let planning_dir = project.path.join(".planning");
                let items = backlog::parse_backlog_items(&planning_dir);
                cache.backlog_items = items;
            }
        }
    }

    // Load data for git tab asynchronously
    if new_view == DetailSubView::GitHistory {
        let cache = ctx.view_cache.entry(alias.to_string()).or_default();
        if cache.git_entries.is_empty() && !cache.loading_git {
            cache.loading_git = true;
            if let (Some(project), Some(tx)) = (ctx.config.projects.get(alias), &ctx.event_tx) {
                let tx = tx.clone();
                let project_path = project.path.clone();
                let alias_owned = alias.to_string();
                let planning_only = cache.git_planning_only;
                tokio::spawn(async move {
                    match git_ops::load_git_log(&project_path, planning_only, 50).await {
                        Ok(entries) => {
                            let _ = tx.send(Action::GitLogLoaded {
                                alias: alias_owned,
                                entries,
                                planning_only,
                            });
                        }
                        Err(_) => {
                            let _ = tx.send(Action::GitLogLoaded {
                                alias: alias_owned,
                                entries: Vec::new(),
                                planning_only,
                            });
                        }
                    }
                });
            }
        }
    }

    ScreenAction::None
}

impl Screen for DetailScreen {
    fn handle_key(&mut self, code: KeyCode, _modifiers: KeyModifiers, ctx: &mut AppContext) -> ScreenAction {
        let current_view = ctx
            .detail_sub_view_per_project
            .get(&self.alias)
            .cloned()
            .unwrap_or_default();
        let current_idx = tab_index(&current_view);

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
            // Tab switching via number keys
            KeyCode::Char('1') => switch_to_tab(&self.alias, 0, &mut self.scroll_offset, ctx),
            KeyCode::Char('2') => switch_to_tab(&self.alias, 1, &mut self.scroll_offset, ctx),
            KeyCode::Char('3') => switch_to_tab(&self.alias, 2, &mut self.scroll_offset, ctx),
            KeyCode::Char('4') => switch_to_tab(&self.alias, 3, &mut self.scroll_offset, ctx),
            // Tab switching via arrow keys
            KeyCode::Left => {
                if current_idx > 0 {
                    switch_to_tab(&self.alias, current_idx - 1, &mut self.scroll_offset, ctx)
                } else {
                    ScreenAction::None
                }
            }
            KeyCode::Right => {
                if current_idx < 3 {
                    switch_to_tab(&self.alias, current_idx + 1, &mut self.scroll_offset, ctx)
                } else {
                    ScreenAction::None
                }
            }
            // Enter: expand backlog item or load diff stat
            KeyCode::Enter => {
                match current_view {
                    DetailSubView::Backlog => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        if !cache.backlog_items.is_empty() {
                            cache.backlog_expanded = !cache.backlog_expanded;
                            // Load content if expanding and not yet loaded
                            if cache.backlog_expanded {
                                let selected = cache.backlog_selected;
                                if let Some(item) = cache.backlog_items.get(selected) {
                                    if item.content.is_none() {
                                        if let Some(project) = ctx.config.projects.get(&self.alias) {
                                            let planning_dir = project.path.join(".planning");
                                            let content = backlog::load_backlog_content(&planning_dir, &item.dir_name);
                                            if let Some(content) = content {
                                                cache.backlog_items[selected].content = Some(content);
                                            }
                                        }
                                    }
                                }
                            }
                            ctx.needs_redraw = true;
                        }
                        ScreenAction::None
                    }
                    DetailSubView::GitHistory => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        if let Some(entry) = cache.git_entries.get(cache.git_selected) {
                            let hash = entry.hash.clone();
                            cache.loading_diff = true;
                            if let (Some(project), Some(tx)) = (ctx.config.projects.get(&self.alias), &ctx.event_tx) {
                                let tx = tx.clone();
                                let project_path = project.path.clone();
                                let alias = self.alias.clone();
                                tokio::spawn(async move {
                                    match git_ops::load_diff_stat(&project_path, &hash).await {
                                        Ok(stat) => {
                                            let _ = tx.send(Action::GitDiffStatLoaded {
                                                alias,
                                                hash,
                                                stat,
                                            });
                                        }
                                        Err(_) => {
                                            let _ = tx.send(Action::GitDiffStatLoaded {
                                                alias,
                                                hash,
                                                stat: Default::default(),
                                            });
                                        }
                                    }
                                });
                            }
                            ctx.needs_redraw = true;
                        }
                        ScreenAction::None
                    }
                    _ => ScreenAction::None,
                }
            }
            // Toggle planning-only filter for git tab
            KeyCode::Char('p') if current_view == DetailSubView::GitHistory => {
                let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                cache.git_planning_only = !cache.git_planning_only;
                cache.git_entries.clear();
                cache.git_diff_stat = None;
                cache.git_selected = 0;
                cache.loading_git = true;
                if let (Some(project), Some(tx)) = (ctx.config.projects.get(&self.alias), &ctx.event_tx) {
                    let tx = tx.clone();
                    let project_path = project.path.clone();
                    let alias = self.alias.clone();
                    let planning_only = cache.git_planning_only;
                    tokio::spawn(async move {
                        match git_ops::load_git_log(&project_path, planning_only, 50).await {
                            Ok(entries) => {
                                let _ = tx.send(Action::GitLogLoaded {
                                    alias,
                                    entries,
                                    planning_only,
                                });
                            }
                            Err(_) => {
                                let _ = tx.send(Action::GitLogLoaded {
                                    alias,
                                    entries: Vec::new(),
                                    planning_only,
                                });
                            }
                        }
                    });
                }
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

        let sub_view = ctx
            .detail_sub_view_per_project
            .get(alias)
            .cloned()
            .unwrap_or_default();
        let tab_idx = tab_index(&sub_view);

        // Split into: tab bar, content, footer
        let chunks = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);
        let tab_area = chunks[0];
        let content_area = chunks[1];
        let footer_area = chunks[2];

        // Render tab bar
        let titles: Vec<Line> = TAB_TITLES.iter().map(|t| Line::from(*t)).collect();
        let tabs_widget = Tabs::new(titles)
            .select(tab_idx)
            .highlight_style(
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(Color::Cyan),
            )
            .divider("|");
        let tab_block = Block::default()
            .borders(Borders::BOTTOM)
            .title(format!(" Project: {} ", alias));
        frame.render_widget(tabs_widget.block(tab_block), tab_area);

        // Render content based on active tab
        match sub_view {
            DetailSubView::PhaseList => self.render_phase_list(frame, content_area, ctx),
            DetailSubView::RoadmapViz => self.render_roadmap(frame, content_area, ctx),
            DetailSubView::Backlog => self.render_backlog_placeholder(frame, content_area, ctx),
            DetailSubView::GitHistory => self.render_git_placeholder(frame, content_area, ctx),
        }

        // Render footer with tab-appropriate hints
        let footer = build_footer(&sub_view);
        frame.render_widget(footer, footer_area);
    }

    fn name(&self) -> &str {
        "detail"
    }
}

impl DetailScreen {
    /// Render the phase list tab content.
    fn render_phase_list(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        let alias = &self.alias;
        let state = ctx.project_states.get(alias);
        let project_path = ctx
            .config
            .projects
            .get(alias)
            .map(|p| p.path.display().to_string())
            .unwrap_or_else(|| "unknown".to_string());

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
            lines.push(Line::from(Span::styled(
                "  Legend: + done  * current  o future  [stage] = disk-inferred  (N plans) = plan count",
                Style::default().fg(Color::DarkGray),
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

        let block = Block::default().borders(Borders::ALL);

        let paragraph = Paragraph::new(lines)
            .block(block)
            .scroll((self.scroll_offset, 0));

        frame.render_widget(paragraph, area);
    }

    /// Render the roadmap visualization tab content.
    fn render_roadmap(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        let alias = &self.alias;
        let state = ctx.project_states.get(alias);
        let project_path = ctx
            .config
            .projects
            .get(alias)
            .map(|p| p.path.display().to_string())
            .unwrap_or_else(|| "unknown".to_string());

        if let Some(state) = state {
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
            .split(area);

            let header_area = content_chunks[0];
            let roadmap_area = content_chunks[1];

            let header_block = Block::default()
                .borders(Borders::TOP | Borders::LEFT | Borders::RIGHT);

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
            let block = Block::default().borders(Borders::ALL);
            let paragraph = Paragraph::new("  No state data available for roadmap.")
                .block(block);
            frame.render_widget(paragraph, area);
        }
    }

    /// Render the backlog tab placeholder content.
    fn render_backlog_placeholder(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        let cache = ctx.view_cache.get(&self.alias);

        let mut lines: Vec<Line> = Vec::new();

        if let Some(cache) = cache {
            if cache.loading_backlog {
                lines.push(Line::from(Span::styled(
                    "  Loading...",
                    Style::default().fg(Color::DarkGray),
                )));
            } else if cache.backlog_items.is_empty() {
                lines.push(Line::from("  No backlog items found."));
            } else {
                lines.push(Line::from(format!(
                    "  Backlog items: {}",
                    cache.backlog_items.len()
                )));
                lines.push(Line::from(""));
                for (i, item) in cache.backlog_items.iter().enumerate() {
                    let prefix = if i == cache.backlog_selected { "> " } else { "  " };
                    let style = if i == cache.backlog_selected {
                        Style::default().add_modifier(Modifier::BOLD).fg(Color::Cyan)
                    } else {
                        Style::default()
                    };
                    lines.push(Line::from(Span::styled(
                        format!("{}  {} - {}", prefix, item.number, item.description),
                        style,
                    )));
                }
            }
        } else {
            lines.push(Line::from(Span::styled(
                "  Loading...",
                Style::default().fg(Color::DarkGray),
            )));
        }

        let block = Block::default().borders(Borders::ALL);
        let paragraph = Paragraph::new(lines)
            .block(block)
            .scroll((self.scroll_offset, 0));
        frame.render_widget(paragraph, area);
    }

    /// Render the git history tab placeholder content.
    fn render_git_placeholder(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        let cache = ctx.view_cache.get(&self.alias);

        let mut lines: Vec<Line> = Vec::new();

        if let Some(cache) = cache {
            if cache.git_planning_only {
                lines.push(Line::from(Span::styled(
                    "  [.planning/ only]",
                    Style::default().fg(Color::Yellow),
                )));
                lines.push(Line::from(""));
            }

            if cache.loading_git {
                lines.push(Line::from(Span::styled(
                    "  Loading...",
                    Style::default().fg(Color::DarkGray),
                )));
            } else if cache.git_entries.is_empty() {
                lines.push(Line::from("  No git log entries found."));
            } else {
                lines.push(Line::from(format!(
                    "  Git log: {} entries",
                    cache.git_entries.len()
                )));
                lines.push(Line::from(""));
                for (i, entry) in cache.git_entries.iter().enumerate() {
                    let prefix = if i == cache.git_selected { "> " } else { "  " };
                    let style = if i == cache.git_selected {
                        Style::default().add_modifier(Modifier::BOLD)
                    } else {
                        Style::default()
                    };
                    lines.push(Line::from(vec![
                        Span::styled(format!("{}  ", prefix), style),
                        Span::styled(&entry.hash, Style::default().fg(Color::Yellow)),
                        Span::styled(
                            format!(" {} ", entry.date),
                            Style::default().fg(Color::DarkGray),
                        ),
                        Span::styled(&entry.message, style),
                    ]));
                }
            }
        } else {
            lines.push(Line::from(Span::styled(
                "  Loading...",
                Style::default().fg(Color::DarkGray),
            )));
        }

        let block = Block::default().borders(Borders::ALL);
        let paragraph = Paragraph::new(lines)
            .block(block)
            .scroll((self.scroll_offset, 0));
        frame.render_widget(paragraph, area);
    }

    /// Render just the main content area (without footer), used by EnqueueScreen overlay.
    pub fn render_main_only(&self, frame: &mut Frame, main_area: Rect, ctx: &AppContext) {
        let alias = &self.alias;

        let sub_view = ctx
            .detail_sub_view_per_project
            .get(alias)
            .cloned()
            .unwrap_or_default();
        let tab_idx = tab_index(&sub_view);

        // Split into tab bar and content
        let chunks = Layout::vertical([
            Constraint::Length(3),
            Constraint::Min(0),
        ])
        .split(main_area);
        let tab_area = chunks[0];
        let content_area = chunks[1];

        // Render tab bar
        let titles: Vec<Line> = TAB_TITLES.iter().map(|t| Line::from(*t)).collect();
        let tabs_widget = Tabs::new(titles)
            .select(tab_idx)
            .highlight_style(
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(Color::Cyan),
            )
            .divider("|");
        let tab_block = Block::default()
            .borders(Borders::BOTTOM)
            .title(format!(" Project: {} ", alias));
        frame.render_widget(tabs_widget.block(tab_block), tab_area);

        // Render content based on active tab
        match sub_view {
            DetailSubView::PhaseList => self.render_phase_list(frame, content_area, ctx),
            DetailSubView::RoadmapViz => self.render_roadmap(frame, content_area, ctx),
            DetailSubView::Backlog => self.render_backlog_placeholder(frame, content_area, ctx),
            DetailSubView::GitHistory => self.render_git_placeholder(frame, content_area, ctx),
        }
    }
}

/// Build the footer line with tab-appropriate key hints.
fn build_footer(sub_view: &DetailSubView) -> Paragraph<'static> {
    let mut spans = vec![
        Span::raw("  "),
        Span::styled("[Esc]", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("back  "),
        Span::styled("[1-4]", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("tabs  "),
        Span::styled("[j/k]", Style::default().add_modifier(Modifier::BOLD)),
        Span::raw("scroll  "),
    ];

    match sub_view {
        DetailSubView::Backlog => {
            spans.push(Span::styled("[Enter]", Style::default().add_modifier(Modifier::BOLD)));
            spans.push(Span::raw("expand  "));
            spans.push(Span::styled("[e]", Style::default().add_modifier(Modifier::BOLD)));
            spans.push(Span::raw("enqueue  "));
        }
        DetailSubView::GitHistory => {
            spans.push(Span::styled("[p]", Style::default().add_modifier(Modifier::BOLD)));
            spans.push(Span::raw("planning-only  "));
            spans.push(Span::styled("[Enter]", Style::default().add_modifier(Modifier::BOLD)));
            spans.push(Span::raw("diff  "));
        }
        _ => {
            spans.push(Span::styled("[e]", Style::default().add_modifier(Modifier::BOLD)));
            spans.push(Span::raw("enqueue  "));
        }
    }

    spans.push(Span::styled("[?]", Style::default().add_modifier(Modifier::BOLD)));
    spans.push(Span::raw("help"));

    Paragraph::new(Line::from(spans))
}
