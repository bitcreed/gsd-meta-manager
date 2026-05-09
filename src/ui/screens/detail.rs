use super::enqueue::EnqueueScreen;
use super::help::HelpScreen;
use super::queue_delete_confirm::QueueDeleteConfirmScreen;
use super::{AppContext, Screen, ScreenAction};
use crate::action::Action;
use crate::app::{classify_status, DetailSubView, StatusCategory};
use crate::change_tracker::ChangeTracker;
use crate::state_reader::disk_status::{DiskInference, DiskStatus};
use crate::state_reader::git_ops;
use crate::state_reader::queue_md;
use crate::state_reader::{self, backlog};
use crate::ui::roadmap_widget::RoadmapWidget;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState, Paragraph, Tabs};
use ratatui::Frame;

const PAGE_SCROLL_LINES: u16 = 20;

const TAB_TITLES: [&str; 9] = [
    "1:Phases",
    "2:Roadmap",
    "3:Backlog",
    "4:Git",
    "5:Pipe",
    "6:Queue",
    "7:Sess",
    "8:Arch",
    "9:Cfg",
];

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
        DetailSubView::Pipeline => 4,
        DetailSubView::Queue => 5,
        DetailSubView::Sessions => 6,
        DetailSubView::Archive => 7,
        DetailSubView::Defaults => 8,
    }
}

fn sub_view_from_index(index: usize) -> DetailSubView {
    match index {
        0 => DetailSubView::PhaseList,
        1 => DetailSubView::RoadmapViz,
        2 => DetailSubView::Backlog,
        3 => DetailSubView::GitHistory,
        4 => DetailSubView::Pipeline,
        5 => DetailSubView::Queue,
        6 => DetailSubView::Sessions,
        7 => DetailSubView::Archive,
        8 => DetailSubView::Defaults,
        _ => DetailSubView::PhaseList,
    }
}

/// Find a terminal emulator to use for launching Claude sessions.
/// Tries $TERMINAL env var first, then common terminal emulators.
fn find_terminal() -> Option<String> {
    if let Ok(term) = std::env::var("TERMINAL") {
        if !term.is_empty() {
            return Some(term);
        }
    }
    for candidate in &["kitty", "alacritty", "gnome-terminal", "xterm"] {
        if std::process::Command::new("which")
            .arg(candidate)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
        {
            return Some(candidate.to_string());
        }
    }
    None
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

/// Compute disk-inferred status suffix spans for a phase line, e.g. " [Executing 2/3]".
/// When `show_badges` is true, appends a [verified] or [inferred] badge based on artifact presence.
fn disk_suffix_spans(
    phase_number: &str,
    phase_disk_statuses: &std::collections::HashMap<
        String,
        crate::state_reader::disk_status::DiskInference,
    >,
    show_badges: bool,
) -> Vec<Span<'static>> {
    let mut spans = Vec::new();
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

    if !disk_label.is_empty() {
        let label = if disk_label == "Executing" {
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
        };
        spans.push(Span::raw(label));
    }

    // Append verified/inferred badge when gsd_integration is enabled
    if show_badges {
        if let Some(inf) = phase_disk_statuses.get(phase_number) {
            if inf.has_summaries || inf.has_verification {
                spans.push(Span::styled(
                    " [verified]",
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::DIM),
                ));
            } else {
                spans.push(Span::styled(
                    " [inferred]",
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::DIM),
                ));
            }
        }
    }

    spans
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

    // Load config.json for defaults tab
    if new_view == DetailSubView::Defaults {
        let cache = ctx.view_cache.entry(alias.to_string()).or_default();
        if let Some(project) = ctx.config.projects.get(alias) {
            let config_path = project.path.join(".planning/config.json");
            if let Ok(content) = std::fs::read_to_string(&config_path) {
                cache.defaults_config =
                    crate::state_reader::config_json::parse_gsd_config(&content);
            } else {
                cache.defaults_config = None;
            }
        }
        cache.defaults_selected = 0;
        cache.defaults_editing = None;
        cache.defaults_dropdown_selected = 0;
    }

    // Load milestone list for archive tab on first visit
    if new_view == DetailSubView::Archive {
        let cache = ctx.view_cache.entry(alias.to_string()).or_default();
        if cache.archive_milestones.is_empty() && !cache.archive_loading {
            cache.archive_loading = true;
            if let (Some(project), Some(tx)) = (ctx.config.projects.get(alias), &ctx.event_tx) {
                let tx = tx.clone();
                let milestones_dir = project.path.join(".planning/milestones");
                let alias_owned = alias.to_string();
                tokio::task::spawn_blocking(move || {
                    let milestones = crate::archive::discover_milestones(&milestones_dir);
                    let _ = tx.send(Action::ArchiveMilestonesDiscovered {
                        alias: alias_owned,
                        milestones,
                    });
                });
            }
        }
    }

    ScreenAction::None
}

/// Load queue, apply a mutation, save back to disk, and reload project state.
/// Returns Ok(()) on success or an error message string.
fn queue_mutate_and_save(
    alias: &str,
    ctx: &mut AppContext,
    mutate: impl FnOnce(&mut Vec<queue_md::QueuedAction>),
) -> Result<(), String> {
    let project = ctx
        .config
        .projects
        .get(alias)
        .ok_or_else(|| "Project not found".to_string())?;
    let planning_dir = project.path.join(".planning");
    let mut actions = queue_md::load_queue(&planning_dir);
    mutate(&mut actions);
    queue_md::save_queue(&planning_dir, &actions).map_err(|e| format!("Queue error: {}", e))?;
    let new_state = state_reader::parse_project_state(&planning_dir);
    ctx.project_states.insert(alias.to_string(), new_state);
    Ok(())
}

impl Screen for DetailScreen {
    fn handle_key(
        &mut self,
        code: KeyCode,
        _modifiers: KeyModifiers,
        ctx: &mut AppContext,
    ) -> ScreenAction {
        let current_view = ctx
            .detail_sub_view_per_project
            .get(&self.alias)
            .cloned()
            .unwrap_or_default();
        let current_idx = tab_index(&current_view);

        match code {
            KeyCode::Esc | KeyCode::Char('q') => {
                // Archive: pop depth level before popping screen
                if current_view == DetailSubView::Archive {
                    let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                    use crate::archive::ArchiveDepth;
                    match &cache.archive_depth {
                        ArchiveDepth::MilestoneList => {
                            // At root level -- fall through to pop screen
                        }
                        ArchiveDepth::PhaseList { .. } => {
                            cache.archive_depth = ArchiveDepth::MilestoneList;
                            ctx.needs_redraw = true;
                            return ScreenAction::None;
                        }
                        ArchiveDepth::FileList { milestone, .. } => {
                            cache.archive_depth =
                                ArchiveDepth::PhaseList { milestone: milestone.clone() };
                            ctx.needs_redraw = true;
                            return ScreenAction::None;
                        }
                        ArchiveDepth::FileView {
                            milestone,
                            phase_idx,
                            ..
                        } => {
                            match phase_idx {
                                Some(idx) => {
                                    // Came from FileList -- return to FileList
                                    cache.archive_depth = ArchiveDepth::FileList {
                                        milestone: milestone.clone(),
                                        phase_idx: *idx,
                                    };
                                }
                                None => {
                                    // Came from PhaseList (top-level file) -- return to PhaseList
                                    cache.archive_depth = ArchiveDepth::PhaseList {
                                        milestone: milestone.clone(),
                                    };
                                }
                            }
                            cache.archive_file_content = None;
                            cache.archive_scroll_offset = 0;
                            ctx.needs_redraw = true;
                            return ScreenAction::None;
                        }
                    }
                }
                // If diff stat pane is showing on Git tab, dismiss it first
                if current_view == DetailSubView::GitHistory {
                    let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                    if cache.git_diff_stat.is_some() {
                        cache.git_diff_stat = None;
                        cache.loading_diff = false;
                        ctx.needs_redraw = true;
                        return ScreenAction::None;
                    }
                }
                // If a config dropdown is open on the Defaults tab, close it first
                if current_view == DetailSubView::Defaults {
                    let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                    if cache.defaults_editing.is_some() {
                        cache.defaults_editing = None;
                        cache.defaults_dropdown_selected = 0;
                        ctx.needs_redraw = true;
                        return ScreenAction::None;
                    }
                }
                self.scroll_offset = 0;
                ctx.needs_redraw = true;
                ScreenAction::Pop
            }
            KeyCode::Char('j') | KeyCode::Down => {
                match current_view {
                    DetailSubView::GitHistory => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        if !cache.git_entries.is_empty() {
                            let max = cache.git_entries.len().saturating_sub(1);
                            cache.git_selected = (cache.git_selected + 1).min(max);
                            cache.git_diff_stat = None;
                        }
                        ctx.needs_redraw = true;
                    }
                    DetailSubView::Backlog => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        if !cache.backlog_items.is_empty() {
                            let max = cache.backlog_items.len().saturating_sub(1);
                            cache.backlog_selected = (cache.backlog_selected + 1).min(max);
                            cache.backlog_expanded = false;
                        }
                        ctx.needs_redraw = true;
                    }
                    DetailSubView::Pipeline => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        if let Some(state) = ctx.project_states.get(&self.alias) {
                            if !state.phases.is_empty() {
                                let max = state.phases.len().saturating_sub(1);
                                cache.pipeline_selected = (cache.pipeline_selected + 1).min(max);
                            }
                        }
                        ctx.needs_redraw = true;
                    }
                    DetailSubView::Queue => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        if let Some(state) = ctx.project_states.get(&self.alias) {
                            if !state.queued_actions.is_empty() {
                                let max = state.queued_actions.len().saturating_sub(1);
                                cache.queue_selected = (cache.queue_selected + 1).min(max);
                            }
                        }
                        ctx.needs_redraw = true;
                    }
                    DetailSubView::Sessions => {
                        let session_count = ctx
                            .config
                            .projects
                            .get(&self.alias)
                            .map(|proj| {
                                ctx.active_sessions
                                    .iter()
                                    .filter(|s| s.working_dir == proj.path)
                                    .count()
                            })
                            .unwrap_or(0);
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        if session_count > 0 {
                            let max = session_count.saturating_sub(1);
                            cache.sessions_selected = (cache.sessions_selected + 1).min(max);
                        }
                        ctx.needs_redraw = true;
                    }
                    DetailSubView::Archive => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        use crate::archive::ArchiveDepth;
                        match &cache.archive_depth {
                            ArchiveDepth::MilestoneList => {
                                let max = cache.archive_milestones.len().saturating_sub(1);
                                cache.archive_selected[0] =
                                    (cache.archive_selected[0] + 1).min(max);
                            }
                            ArchiveDepth::PhaseList { milestone } => {
                                if let Some(data) = ctx.archive_cache.get(milestone) {
                                    let total = data.top_level_files.len() + data.phases.len();
                                    let max = total.saturating_sub(1);
                                    cache.archive_selected[1] =
                                        (cache.archive_selected[1] + 1).min(max);
                                }
                            }
                            ArchiveDepth::FileList { milestone, phase_idx } => {
                                if let Some(data) = ctx.archive_cache.get(milestone) {
                                    if let Some(phase) = data.phases.get(*phase_idx) {
                                        let max = phase.files.len().saturating_sub(1);
                                        cache.archive_selected[2] =
                                            (cache.archive_selected[2] + 1).min(max);
                                    }
                                }
                            }
                            ArchiveDepth::FileView { .. } => {
                                cache.archive_scroll_offset =
                                    cache.archive_scroll_offset.saturating_add(1);
                            }
                        }
                        ctx.needs_redraw = true;
                    }
                    DetailSubView::Defaults => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        if let Some(editing_idx) = cache.defaults_editing {
                            // Move dropdown cursor down
                            if let Some(ref config) = cache.defaults_config {
                                let entries = build_defaults_entries(config);
                                if let Some(entry) = entries.get(editing_idx) {
                                    let options = dropdown_options(&entry.kind);
                                    if !options.is_empty() {
                                        let max = options.len() - 1;
                                        cache.defaults_dropdown_selected =
                                            (cache.defaults_dropdown_selected + 1).min(max);
                                    }
                                }
                            }
                        } else {
                            let entry_count = defaults_entry_count(&cache.defaults_config);
                            if entry_count > 0 {
                                let max = entry_count.saturating_sub(1);
                                cache.defaults_selected = (cache.defaults_selected + 1).min(max);
                            }
                        }
                        ctx.needs_redraw = true;
                    }
                    _ => {
                        self.scroll_offset = self.scroll_offset.saturating_add(1);
                        ctx.needs_redraw = true;
                    }
                }
                ScreenAction::None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                match current_view {
                    DetailSubView::GitHistory => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        if !cache.git_entries.is_empty() {
                            cache.git_selected = cache.git_selected.saturating_sub(1);
                            cache.git_diff_stat = None;
                        }
                        ctx.needs_redraw = true;
                    }
                    DetailSubView::Backlog => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        cache.backlog_selected = cache.backlog_selected.saturating_sub(1);
                        cache.backlog_expanded = false;
                        ctx.needs_redraw = true;
                    }
                    DetailSubView::Pipeline => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        cache.pipeline_selected = cache.pipeline_selected.saturating_sub(1);
                        ctx.needs_redraw = true;
                    }
                    DetailSubView::Queue => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        cache.queue_selected = cache.queue_selected.saturating_sub(1);
                        ctx.needs_redraw = true;
                    }
                    DetailSubView::Sessions => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        cache.sessions_selected = cache.sessions_selected.saturating_sub(1);
                        ctx.needs_redraw = true;
                    }
                    DetailSubView::Archive => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        use crate::archive::ArchiveDepth;
                        match &cache.archive_depth {
                            ArchiveDepth::MilestoneList => {
                                cache.archive_selected[0] =
                                    cache.archive_selected[0].saturating_sub(1);
                            }
                            ArchiveDepth::PhaseList { .. } => {
                                cache.archive_selected[1] =
                                    cache.archive_selected[1].saturating_sub(1);
                            }
                            ArchiveDepth::FileList { .. } => {
                                cache.archive_selected[2] =
                                    cache.archive_selected[2].saturating_sub(1);
                            }
                            ArchiveDepth::FileView { .. } => {
                                cache.archive_scroll_offset =
                                    cache.archive_scroll_offset.saturating_sub(1);
                            }
                        }
                        ctx.needs_redraw = true;
                    }
                    DetailSubView::Defaults => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        if cache.defaults_editing.is_some() {
                            cache.defaults_dropdown_selected =
                                cache.defaults_dropdown_selected.saturating_sub(1);
                        } else {
                            cache.defaults_selected = cache.defaults_selected.saturating_sub(1);
                        }
                        ctx.needs_redraw = true;
                    }
                    _ => {
                        self.scroll_offset = self.scroll_offset.saturating_sub(1);
                        ctx.needs_redraw = true;
                    }
                }
                ScreenAction::None
            }
            KeyCode::PageDown => {
                match current_view {
                    DetailSubView::GitHistory => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        if !cache.git_entries.is_empty() {
                            let max = cache.git_entries.len().saturating_sub(1);
                            cache.git_selected = (cache.git_selected + PAGE_SCROLL_LINES as usize).min(max);
                            cache.git_diff_stat = None;
                        }
                        ctx.needs_redraw = true;
                    }
                    DetailSubView::Backlog => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        if !cache.backlog_items.is_empty() {
                            let max = cache.backlog_items.len().saturating_sub(1);
                            cache.backlog_selected = (cache.backlog_selected + PAGE_SCROLL_LINES as usize).min(max);
                            cache.backlog_expanded = false;
                        }
                        ctx.needs_redraw = true;
                    }
                    DetailSubView::Pipeline => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        if let Some(state) = ctx.project_states.get(&self.alias) {
                            if !state.phases.is_empty() {
                                let max = state.phases.len().saturating_sub(1);
                                cache.pipeline_selected = (cache.pipeline_selected + PAGE_SCROLL_LINES as usize).min(max);
                            }
                        }
                        ctx.needs_redraw = true;
                    }
                    DetailSubView::Queue => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        if let Some(state) = ctx.project_states.get(&self.alias) {
                            if !state.queued_actions.is_empty() {
                                let max = state.queued_actions.len().saturating_sub(1);
                                cache.queue_selected = (cache.queue_selected + PAGE_SCROLL_LINES as usize).min(max);
                            }
                        }
                        ctx.needs_redraw = true;
                    }
                    DetailSubView::Sessions => {
                        let session_count = ctx
                            .config
                            .projects
                            .get(&self.alias)
                            .map(|proj| {
                                ctx.active_sessions
                                    .iter()
                                    .filter(|s| s.working_dir == proj.path)
                                    .count()
                            })
                            .unwrap_or(0);
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        if session_count > 0 {
                            let max = session_count.saturating_sub(1);
                            cache.sessions_selected = (cache.sessions_selected + PAGE_SCROLL_LINES as usize).min(max);
                        }
                        ctx.needs_redraw = true;
                    }
                    DetailSubView::Archive => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        use crate::archive::ArchiveDepth;
                        match &cache.archive_depth {
                            ArchiveDepth::MilestoneList => {
                                let max = cache.archive_milestones.len().saturating_sub(1);
                                cache.archive_selected[0] =
                                    (cache.archive_selected[0] + PAGE_SCROLL_LINES as usize).min(max);
                            }
                            ArchiveDepth::PhaseList { milestone } => {
                                if let Some(data) = ctx.archive_cache.get(milestone) {
                                    let total = data.top_level_files.len() + data.phases.len();
                                    let max = total.saturating_sub(1);
                                    cache.archive_selected[1] =
                                        (cache.archive_selected[1] + PAGE_SCROLL_LINES as usize).min(max);
                                }
                            }
                            ArchiveDepth::FileList { milestone, phase_idx } => {
                                if let Some(data) = ctx.archive_cache.get(milestone) {
                                    if let Some(phase) = data.phases.get(*phase_idx) {
                                        let max = phase.files.len().saturating_sub(1);
                                        cache.archive_selected[2] =
                                            (cache.archive_selected[2] + PAGE_SCROLL_LINES as usize).min(max);
                                    }
                                }
                            }
                            ArchiveDepth::FileView { .. } => {
                                cache.archive_scroll_offset =
                                    cache.archive_scroll_offset.saturating_add(PAGE_SCROLL_LINES);
                            }
                        }
                        ctx.needs_redraw = true;
                    }
                    DetailSubView::Defaults => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        let entry_count = defaults_entry_count(&cache.defaults_config);
                        if entry_count > 0 {
                            let max = entry_count.saturating_sub(1);
                            cache.defaults_selected = (cache.defaults_selected + PAGE_SCROLL_LINES as usize).min(max);
                        }
                        ctx.needs_redraw = true;
                    }
                    _ => {
                        self.scroll_offset = self.scroll_offset.saturating_add(PAGE_SCROLL_LINES);
                        ctx.needs_redraw = true;
                    }
                }
                ScreenAction::None
            }
            KeyCode::PageUp => {
                match current_view {
                    DetailSubView::GitHistory => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        if !cache.git_entries.is_empty() {
                            cache.git_selected = cache.git_selected.saturating_sub(PAGE_SCROLL_LINES as usize);
                            cache.git_diff_stat = None;
                        }
                        ctx.needs_redraw = true;
                    }
                    DetailSubView::Backlog => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        cache.backlog_selected = cache.backlog_selected.saturating_sub(PAGE_SCROLL_LINES as usize);
                        cache.backlog_expanded = false;
                        ctx.needs_redraw = true;
                    }
                    DetailSubView::Pipeline => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        cache.pipeline_selected = cache.pipeline_selected.saturating_sub(PAGE_SCROLL_LINES as usize);
                        ctx.needs_redraw = true;
                    }
                    DetailSubView::Queue => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        cache.queue_selected = cache.queue_selected.saturating_sub(PAGE_SCROLL_LINES as usize);
                        ctx.needs_redraw = true;
                    }
                    DetailSubView::Sessions => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        cache.sessions_selected = cache.sessions_selected.saturating_sub(PAGE_SCROLL_LINES as usize);
                        ctx.needs_redraw = true;
                    }
                    DetailSubView::Archive => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        use crate::archive::ArchiveDepth;
                        match &cache.archive_depth {
                            ArchiveDepth::MilestoneList => {
                                cache.archive_selected[0] =
                                    cache.archive_selected[0].saturating_sub(PAGE_SCROLL_LINES as usize);
                            }
                            ArchiveDepth::PhaseList { .. } => {
                                cache.archive_selected[1] =
                                    cache.archive_selected[1].saturating_sub(PAGE_SCROLL_LINES as usize);
                            }
                            ArchiveDepth::FileList { .. } => {
                                cache.archive_selected[2] =
                                    cache.archive_selected[2].saturating_sub(PAGE_SCROLL_LINES as usize);
                            }
                            ArchiveDepth::FileView { .. } => {
                                cache.archive_scroll_offset =
                                    cache.archive_scroll_offset.saturating_sub(PAGE_SCROLL_LINES);
                            }
                        }
                        ctx.needs_redraw = true;
                    }
                    DetailSubView::Defaults => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        cache.defaults_selected = cache.defaults_selected.saturating_sub(PAGE_SCROLL_LINES as usize);
                        ctx.needs_redraw = true;
                    }
                    _ => {
                        self.scroll_offset = self.scroll_offset.saturating_sub(PAGE_SCROLL_LINES);
                        ctx.needs_redraw = true;
                    }
                }
                ScreenAction::None
            }
            // Tab switching via number keys
            KeyCode::Char('1') => switch_to_tab(&self.alias, 0, &mut self.scroll_offset, ctx),
            KeyCode::Char('2') => switch_to_tab(&self.alias, 1, &mut self.scroll_offset, ctx),
            KeyCode::Char('3') => switch_to_tab(&self.alias, 2, &mut self.scroll_offset, ctx),
            KeyCode::Char('4') => switch_to_tab(&self.alias, 3, &mut self.scroll_offset, ctx),
            KeyCode::Char('5') => switch_to_tab(&self.alias, 4, &mut self.scroll_offset, ctx),
            KeyCode::Char('6') => switch_to_tab(&self.alias, 5, &mut self.scroll_offset, ctx),
            KeyCode::Char('7') => switch_to_tab(&self.alias, 6, &mut self.scroll_offset, ctx),
            KeyCode::Char('8') => switch_to_tab(&self.alias, 7, &mut self.scroll_offset, ctx),
            KeyCode::Char('9') => switch_to_tab(&self.alias, 8, &mut self.scroll_offset, ctx),
            // Tab switching via arrow keys
            KeyCode::Left => {
                if current_idx > 0 {
                    switch_to_tab(&self.alias, current_idx - 1, &mut self.scroll_offset, ctx)
                } else {
                    ScreenAction::None
                }
            }
            KeyCode::Right => {
                if current_idx < TAB_TITLES.len() - 1 {
                    switch_to_tab(&self.alias, current_idx + 1, &mut self.scroll_offset, ctx)
                } else {
                    ScreenAction::None
                }
            }
            // Enter/Space: expand backlog item, load diff stat, or mark queue item done
            KeyCode::Enter | KeyCode::Char(' ') => {
                match current_view {
                    DetailSubView::Queue => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        let selected = cache.queue_selected;
                        // Get the command text before mutation
                        let command_text = ctx
                            .project_states
                            .get(&self.alias)
                            .and_then(|s| s.queued_actions.get(selected))
                            .map(|a| a.command.clone());
                        if let Some(cmd) = command_text {
                            match queue_mutate_and_save(&self.alias, ctx, |actions| {
                                if selected < actions.len() {
                                    actions.remove(selected);
                                }
                            }) {
                                Ok(()) => {
                                    // Clamp selection
                                    let cache =
                                        ctx.view_cache.entry(self.alias.clone()).or_default();
                                    let new_len = ctx
                                        .project_states
                                        .get(&self.alias)
                                        .map(|s| s.queued_actions.len())
                                        .unwrap_or(0);
                                    if new_len == 0 {
                                        cache.queue_selected = 0;
                                    } else if cache.queue_selected >= new_len {
                                        cache.queue_selected = new_len - 1;
                                    }
                                    ctx.status_message =
                                        Some((format!("Done: {}", cmd), std::time::Instant::now()));
                                }
                                Err(e) => {
                                    ctx.status_message = Some((e, std::time::Instant::now()));
                                }
                            }
                            ctx.needs_redraw = true;
                        }
                        ScreenAction::None
                    }
                    DetailSubView::Backlog => {
                        // Space does nothing on backlog; Enter toggles expand
                        if code == KeyCode::Char(' ') {
                            return ScreenAction::None;
                        }
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        if !cache.backlog_items.is_empty() {
                            cache.backlog_expanded = !cache.backlog_expanded;
                            // Load content if expanding and not yet loaded
                            if cache.backlog_expanded {
                                let selected = cache.backlog_selected;
                                if let Some(item) = cache.backlog_items.get(selected) {
                                    if item.content.is_none() {
                                        if let Some(project) = ctx.config.projects.get(&self.alias)
                                        {
                                            let planning_dir = project.path.join(".planning");
                                            let content = backlog::load_backlog_content(
                                                &planning_dir,
                                                &item.dir_name,
                                            );
                                            if let Some(content) = content {
                                                cache.backlog_items[selected].content =
                                                    Some(content);
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
                        if code == KeyCode::Char(' ') {
                            return ScreenAction::None;
                        }
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        if let Some(entry) = cache.git_entries.get(cache.git_selected) {
                            let hash = entry.hash.clone();
                            cache.loading_diff = true;
                            if let (Some(project), Some(tx)) =
                                (ctx.config.projects.get(&self.alias), &ctx.event_tx)
                            {
                                let tx = tx.clone();
                                let project_path = project.path.clone();
                                let alias = self.alias.clone();
                                tokio::spawn(async move {
                                    match git_ops::load_diff_stat(&project_path, &hash).await {
                                        Ok(stat) => {
                                            let _ =
                                                tx.send(Action::GitDiffStatLoaded { alias, stat });
                                        }
                                        Err(_) => {
                                            let _ = tx.send(Action::GitDiffStatLoaded {
                                                alias,
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
                    DetailSubView::Sessions => {
                        // Resume selected session in a new terminal
                        let filtered_sessions: Vec<_> = ctx
                            .config
                            .projects
                            .get(&self.alias)
                            .map(|proj| {
                                ctx.active_sessions
                                    .iter()
                                    .filter(|s| s.working_dir == proj.path)
                                    .cloned()
                                    .collect()
                            })
                            .unwrap_or_default();
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        if let Some(session) = filtered_sessions.get(cache.sessions_selected) {
                            if let Some(ref sid) = session.session_id {
                                match find_terminal() {
                                    Some(term) => {
                                        let short_id = if sid.len() > 8 { &sid[..8] } else { sid };
                                        match std::process::Command::new(&term)
                                            .args([
                                                "-e",
                                                "sh",
                                                "-c",
                                                &format!(
                                                    "cd '{}' && claude --resume '{}'",
                                                    session.working_dir.display(),
                                                    sid
                                                ),
                                            ])
                                            .spawn()
                                        {
                                            Ok(_) => {
                                                return ScreenAction::SetStatusMessage(format!(
                                                    "Resumed session {}",
                                                    short_id
                                                ))
                                            }
                                            Err(e) => {
                                                return ScreenAction::SetStatusMessage(format!(
                                                    "Failed to launch: {}",
                                                    e
                                                ))
                                            }
                                        }
                                    }
                                    None => {
                                        return ScreenAction::SetStatusMessage(
                                            "No terminal emulator found (set $TERMINAL)"
                                                .to_string(),
                                        )
                                    }
                                }
                            } else {
                                return ScreenAction::SetStatusMessage(
                                    "No session ID to resume".to_string(),
                                );
                            }
                        }
                        ScreenAction::None
                    }
                    DetailSubView::Archive => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        use crate::archive::ArchiveDepth;
                        match cache.archive_depth.clone() {
                            ArchiveDepth::MilestoneList => {
                                let selected = cache.archive_selected[0];
                                if let Some(milestone) =
                                    cache.archive_milestones.get(selected).cloned()
                                {
                                    cache.archive_depth = ArchiveDepth::PhaseList {
                                        milestone: milestone.clone(),
                                    };
                                    cache.archive_selected[1] = 0;
                                    // Trigger async load if not cached
                                    if !ctx.archive_cache.contains_key(&milestone) {
                                        cache.archive_loading = true;
                                        if let Some(project) = ctx.config.projects.get(&self.alias)
                                        {
                                            let milestones_dir =
                                                project.path.join(".planning/milestones");
                                            if let Some(tx) = &ctx.event_tx {
                                                let tx = tx.clone();
                                                let alias = self.alias.clone();
                                                let ms = milestone.clone();
                                                tokio::task::spawn_blocking(move || {
                                                    let data =
                                                        crate::archive::load_milestone_archive(
                                                            &milestones_dir,
                                                            &ms,
                                                        );
                                                    let _ = tx.send(Action::ArchiveLoaded {
                                                        alias,
                                                        milestone: ms,
                                                        data,
                                                    });
                                                });
                                            }
                                        }
                                    }
                                    ctx.needs_redraw = true;
                                }
                            }
                            ArchiveDepth::PhaseList { milestone } => {
                                let selected = cache.archive_selected[1];
                                if let Some(data) = ctx.archive_cache.get(&milestone) {
                                    let top_count = data.top_level_files.len();
                                    if selected < top_count {
                                        // Selected a top-level file
                                        let file = &data.top_level_files[selected];
                                        let path = file.path.clone();
                                        cache.archive_file_name = Some(file.name.clone());
                                        cache.archive_depth = ArchiveDepth::FileView {
                                            milestone: milestone.clone(),
                                            phase_idx: None,
                                            file_idx: selected,
                                        };
                                        cache.archive_scroll_offset = 0;
                                        cache.archive_file_content =
                                            Some(crate::archive::read_archive_file(&path));
                                    } else {
                                        // Selected a phase
                                        let phase_idx = selected - top_count;
                                        cache.archive_depth = ArchiveDepth::FileList {
                                            milestone: milestone.clone(),
                                            phase_idx,
                                        };
                                        cache.archive_selected[2] = 0;
                                    }
                                    ctx.needs_redraw = true;
                                }
                            }
                            ArchiveDepth::FileList {
                                milestone,
                                phase_idx,
                            } => {
                                let selected = cache.archive_selected[2];
                                if let Some(data) = ctx.archive_cache.get(&milestone) {
                                    if let Some(phase) = data.phases.get(phase_idx) {
                                        if let Some(file) = phase.files.get(selected) {
                                            let path = file.path.clone();
                                            cache.archive_file_name = Some(file.name.clone());
                                            cache.archive_depth = ArchiveDepth::FileView {
                                                milestone: milestone.clone(),
                                                phase_idx: Some(phase_idx),
                                                file_idx: selected,
                                            };
                                            cache.archive_scroll_offset = 0;
                                            cache.archive_file_content =
                                                Some(crate::archive::read_archive_file(&path));
                                            ctx.needs_redraw = true;
                                        }
                                    }
                                }
                            }
                            ArchiveDepth::FileView { .. } => {
                                // No action on Enter in file view
                            }
                        }
                        ScreenAction::None
                    }
                    DetailSubView::Defaults => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        if let Some(editing_idx) = cache.defaults_editing {
                            // Dropdown is open — Enter applies the selected option.
                            if let Some(ref mut config) = cache.defaults_config {
                                let entries = build_defaults_entries(config);
                                if let Some(entry) = entries.get(editing_idx) {
                                    let options = dropdown_options(&entry.kind);
                                    let dropdown_idx = cache.defaults_dropdown_selected.min(options.len().saturating_sub(1));
                                    if let Some(value) = options.get(dropdown_idx) {
                                        let applied = set_config_value(config, entry.key, value);
                                        if applied {
                                            if let Some(project) = ctx.config.projects.get(&self.alias) {
                                                let config_path = project.path.join(".planning/config.json");
                                                if let Ok(json) = crate::state_reader::config_json::serialize_gsd_config(config) {
                                                    let _ = std::fs::write(&config_path, &json);
                                                    ctx.status_message = Some(("Config saved".to_string(), std::time::Instant::now()));
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                            cache.defaults_editing = None;
                            cache.defaults_dropdown_selected = 0;
                        } else if let Some(ref config) = cache.defaults_config {
                            let entries = build_defaults_entries(config);
                            let selected = cache.defaults_selected;
                            if let Some(entry) = entries.get(selected) {
                                let options = dropdown_options(&entry.kind);
                                if !options.is_empty() {
                                    // Open dropdown, position cursor on current value
                                    let current_idx = options.iter().position(|o| o == &entry.value).unwrap_or(0);
                                    cache.defaults_editing = Some(selected);
                                    cache.defaults_dropdown_selected = current_idx;
                                } else if matches!(entry.kind, ConfigValueKind::Integer) {
                                    // Integers have no fixed list — keep cycling behavior.
                                    if let Some(ref mut config) = cache.defaults_config {
                                        let mutated = mutate_config_entry(config, entry.key, &entry.kind);
                                        if mutated {
                                            if let Some(project) = ctx.config.projects.get(&self.alias) {
                                                let config_path = project.path.join(".planning/config.json");
                                                if let Ok(json) = crate::state_reader::config_json::serialize_gsd_config(config) {
                                                    let _ = std::fs::write(&config_path, &json);
                                                    ctx.status_message = Some(("Config saved".to_string(), std::time::Instant::now()));
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                        ctx.needs_redraw = true;
                        ScreenAction::None
                    }
                    _ => ScreenAction::None,
                }
            }
            // 'r' key: reload config (Defaults tab only)
            KeyCode::Char('r') if current_view == DetailSubView::Defaults => {
                let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                if let Some(project) = ctx.config.projects.get(&self.alias) {
                    let config_path = project.path.join(".planning/config.json");
                    if let Ok(content) = std::fs::read_to_string(&config_path) {
                        cache.defaults_config =
                            crate::state_reader::config_json::parse_gsd_config(&content);
                    } else {
                        cache.defaults_config = None;
                    }
                }
                ctx.status_message = Some(("Config reloaded".to_string(), std::time::Instant::now()));
                ctx.needs_redraw = true;
                ScreenAction::None
            }
            // 'n' key: launch new Claude session (Sessions tab only)
            KeyCode::Char('n') if current_view == DetailSubView::Sessions => {
                if let Some(project) = ctx.config.projects.get(&self.alias) {
                    match find_terminal() {
                        Some(term) => {
                            match std::process::Command::new(&term)
                                .args([
                                    "-e",
                                    "sh",
                                    "-c",
                                    &format!("cd '{}' && claude", project.path.display()),
                                ])
                                .spawn()
                            {
                                Ok(_) => ScreenAction::SetStatusMessage(
                                    "Launched new Claude session".to_string(),
                                ),
                                Err(e) => ScreenAction::SetStatusMessage(format!(
                                    "Failed to launch: {}",
                                    e
                                )),
                            }
                        }
                        None => ScreenAction::SetStatusMessage(
                            "No terminal emulator found (set $TERMINAL)".to_string(),
                        ),
                    }
                } else {
                    ScreenAction::None
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
                if let (Some(project), Some(tx)) =
                    (ctx.config.projects.get(&self.alias), &ctx.event_tx)
                {
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
            // 'a' key: add new queue item (Queue tab only)
            KeyCode::Char('a') if current_view == DetailSubView::Queue => {
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
                    ctx.input_buffer.clear();
                    ctx.suggestion_index = 0;
                    ctx.needs_redraw = true;
                    ScreenAction::Push(Box::new(EnqueueScreen::new(alias)))
                }
            }
            // 'd' or 'x' key: delete queue item with confirmation (Queue tab only)
            KeyCode::Char('d') | KeyCode::Char('x') if current_view == DetailSubView::Queue => {
                let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                let selected = cache.queue_selected;
                let command_text = ctx
                    .project_states
                    .get(&self.alias)
                    .and_then(|s| s.queued_actions.get(selected))
                    .map(|a| a.command.clone());
                if let Some(cmd) = command_text {
                    ctx.needs_redraw = true;
                    ScreenAction::Push(Box::new(QueueDeleteConfirmScreen::new(
                        self.alias.clone(),
                        selected,
                        cmd,
                    )))
                } else {
                    ScreenAction::None
                }
            }
            // Shift+J: move queue item down
            KeyCode::Char('J') if current_view == DetailSubView::Queue => {
                let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                let selected = cache.queue_selected;
                let len = ctx
                    .project_states
                    .get(&self.alias)
                    .map(|s| s.queued_actions.len())
                    .unwrap_or(0);
                if len > 1 && selected < len - 1 {
                    match queue_mutate_and_save(&self.alias, ctx, |actions| {
                        if selected < actions.len() - 1 {
                            actions.swap(selected, selected + 1);
                        }
                    }) {
                        Ok(()) => {
                            let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                            cache.queue_selected = selected + 1;
                        }
                        Err(e) => {
                            ctx.status_message = Some((e, std::time::Instant::now()));
                        }
                    }
                    ctx.needs_redraw = true;
                }
                ScreenAction::None
            }
            // Shift+K: move queue item up
            KeyCode::Char('K') if current_view == DetailSubView::Queue => {
                let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                let selected = cache.queue_selected;
                if selected > 0 {
                    match queue_mutate_and_save(&self.alias, ctx, |actions| {
                        if selected > 0 && selected < actions.len() {
                            actions.swap(selected, selected - 1);
                        }
                    }) {
                        Ok(()) => {
                            let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                            cache.queue_selected = selected - 1;
                        }
                        Err(e) => {
                            ctx.status_message = Some((e, std::time::Instant::now()));
                        }
                    }
                    ctx.needs_redraw = true;
                }
                ScreenAction::None
            }
            KeyCode::Char('e') => {
                // Archive FileView: open file in $EDITOR (read-only for milestones)
                if current_view == DetailSubView::Archive {
                    let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                    use crate::archive::ArchiveDepth;
                    if let ArchiveDepth::FileView {
                        milestone,
                        phase_idx,
                        file_idx,
                    } = &cache.archive_depth
                    {
                        // Resolve the file path from archive cache
                        let file_path = ctx.archive_cache.get(milestone).and_then(|data| {
                            match phase_idx {
                                None => {
                                    // Top-level file (entered from PhaseList)
                                    data.top_level_files.get(*file_idx).map(|f| f.path.clone())
                                }
                                Some(idx) => {
                                    data.phases
                                        .get(*idx)
                                        .and_then(|p| p.files.get(*file_idx))
                                        .map(|f| f.path.clone())
                                }
                            }
                        });

                        if let Some(path) = file_path {
                            // Check if file is under milestones/ (archived = read-only)
                            let path_str = path.to_string_lossy();
                            if path_str.contains("/milestones/") {
                                ctx.needs_redraw = true;
                                return ScreenAction::SetStatusMessage(
                                    "Archived files are read-only".to_string(),
                                );
                            }
                            ctx.needs_redraw = true;
                            return ScreenAction::SuspendAndEdit(path);
                        }
                    }
                    // Not in FileView depth -- no-op for 'e'
                    return ScreenAction::None;
                }

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
                } else if current_view == DetailSubView::Queue {
                    // Queue tab: edit selected item (pre-fill + remove old)
                    let cache = ctx.view_cache.entry(alias.clone()).or_default();
                    let selected = cache.queue_selected;
                    let command_text = ctx
                        .project_states
                        .get(&alias)
                        .and_then(|s| s.queued_actions.get(selected))
                        .map(|a| a.command.clone());
                    if let Some(cmd) = command_text {
                        ctx.input_buffer = cmd.clone();
                        ctx.suggestion_index = 0;
                        // Remove the item first; EnqueueScreen will re-add on Enter
                        match queue_mutate_and_save(&alias, ctx, |actions| {
                            if selected < actions.len() {
                                actions.remove(selected);
                            }
                        }) {
                            Ok(()) => {
                                // Clamp selection after removal
                                let cache = ctx.view_cache.entry(alias.clone()).or_default();
                                let new_len = ctx
                                    .project_states
                                    .get(&alias)
                                    .map(|s| s.queued_actions.len())
                                    .unwrap_or(0);
                                if new_len == 0 {
                                    cache.queue_selected = 0;
                                } else if cache.queue_selected >= new_len {
                                    cache.queue_selected = new_len - 1;
                                }
                                ctx.status_message = Some((
                                    format!("Editing: {} (Esc cancels and removes)", cmd),
                                    std::time::Instant::now(),
                                ));
                            }
                            Err(e) => {
                                ctx.status_message = Some((e, std::time::Instant::now()));
                            }
                        }
                        ctx.needs_redraw = true;
                        ScreenAction::Push(Box::new(EnqueueScreen::new(alias)))
                    } else {
                        ScreenAction::None
                    }
                } else if current_view == DetailSubView::Backlog {
                    // Backlog tab: if expanded, open file in $EDITOR; otherwise enqueue
                    let cache = ctx.view_cache.entry(alias.clone()).or_default();
                    if cache.backlog_expanded {
                        if let Some(item) = cache.backlog_items.get(cache.backlog_selected) {
                            if let Some(ref path) = item.path {
                                ctx.needs_redraw = true;
                                return ScreenAction::SuspendAndEdit(path.clone());
                            }
                        }
                        return ScreenAction::SetStatusMessage(
                            "No file found for this backlog item".to_string(),
                        );
                    }
                    // Not expanded: enqueue as before
                    if let Some(item) = cache.backlog_items.get(cache.backlog_selected) {
                        ctx.input_buffer = format!("/gsd:review-backlog {}", item.dir_name);
                    } else {
                        ctx.input_buffer.clear();
                    }
                    ctx.suggestion_index = 0;
                    ctx.needs_redraw = true;
                    ScreenAction::Push(Box::new(EnqueueScreen::new(alias)))
                } else {
                    // Generic: use suggest_next_commands
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
            DetailSubView::Backlog => self.render_backlog_tab(frame, content_area, ctx),
            DetailSubView::GitHistory => self.render_git_tab(frame, content_area, ctx),
            DetailSubView::Pipeline => self.render_pipeline_tab(frame, content_area, ctx),
            DetailSubView::Queue => self.render_queue_tab(frame, content_area, ctx),
            DetailSubView::Sessions => self.render_sessions_tab(frame, content_area, ctx),
            DetailSubView::Archive => self.render_archive_tab(frame, content_area, ctx),
            DetailSubView::Defaults => self.render_defaults_tab(frame, content_area, ctx),
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

            if state.paused {
                let pause_line = if let Some(ref ctx_text) = state.pause_context {
                    Line::from(vec![
                        Span::styled(
                            "  Paused: ",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(ctx_text.as_str(), Style::default().fg(Color::Cyan)),
                    ])
                } else {
                    Line::from(Span::styled(
                        "  Paused (HANDOFF file present)",
                        Style::default().fg(Color::Cyan),
                    ))
                };
                lines.push(pause_line);
            }

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

                    let show_badges = ctx.config.preferences.gsd_integration;
                    let badge_spans =
                        disk_suffix_spans(&phase.number, &state.phase_disk_statuses, show_badges);

                    let line_text = format!(
                        "  {} P{}: {}  {}",
                        icon, phase.number, phase.name, plan_display
                    );

                    if is_current {
                        let cat = classify_status(&state.status);
                        let color = status_color(&cat);
                        let mut spans = vec![Span::styled(
                            line_text,
                            Style::default().fg(color).add_modifier(Modifier::BOLD),
                        )];
                        spans.extend(badge_spans);
                        lines.push(Line::from(spans));
                    } else if phase.completed {
                        let mut spans = vec![Span::styled(
                            line_text,
                            Style::default().fg(Color::DarkGray),
                        )];
                        spans.extend(badge_spans);
                        lines.push(Line::from(spans));
                    } else {
                        let mut spans = vec![Span::raw(line_text)];
                        spans.extend(badge_spans);
                        lines.push(Line::from(spans));
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
                        Span::styled(&action.command, Style::default().fg(Color::Cyan)),
                    ]));
                }
            }
        } else {
            lines.push(Line::from(""));
            lines.push(Line::from("  No state data available for this project."));
        }

        let block = Block::default().borders(Borders::ALL);
        // Clamp scroll so content can't scroll past the end
        let content_height = lines.len() as u16;
        let viewport_height = area.height.saturating_sub(2); // borders
        let max_scroll = content_height.saturating_sub(viewport_height);
        let clamped_offset = self.scroll_offset.min(max_scroll);

        let paragraph = Paragraph::new(lines)
            .block(block)
            .scroll((clamped_offset, 0));

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

            if state.paused {
                let pause_line = if let Some(ref ctx_text) = state.pause_context {
                    Line::from(vec![
                        Span::styled(
                            "  Paused: ",
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Span::styled(ctx_text.as_str(), Style::default().fg(Color::Cyan)),
                    ])
                } else {
                    Line::from(Span::styled(
                        "  Paused (HANDOFF file present)",
                        Style::default().fg(Color::Cyan),
                    ))
                };
                header_lines.push(pause_line);
            }

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

            let content_chunks =
                Layout::vertical([Constraint::Length(header_height), Constraint::Min(0)])
                    .split(area);

            let header_area = content_chunks[0];
            let roadmap_area = content_chunks[1];

            let header_block =
                Block::default().borders(Borders::TOP | Borders::LEFT | Borders::RIGHT);

            let header_paragraph = Paragraph::new(header_lines).block(header_block);
            frame.render_widget(header_paragraph, header_area);

            let current_phase_num = state.completed_phases + 1;
            // Clamp roadmap scroll — estimate content height from phase count
            let phase_block_h: u16 = 5; // BOX_HEIGHT(3) + connector(1) + spacing(1)
            let total_content = if state.phases.is_empty() {
                0
            } else {
                3 + (state.phases.len() as u16 - 1) * phase_block_h
            };
            let viewport_h = roadmap_area.height;
            let max_scroll = total_content.saturating_sub(viewport_h);
            let clamped_offset = self.scroll_offset.min(max_scroll);

            let roadmap_widget = RoadmapWidget {
                phases: &state.phases,
                current_phase_num,
                scroll_offset: clamped_offset,
            };
            frame.render_widget(roadmap_widget, roadmap_area);
        } else {
            let block = Block::default().borders(Borders::ALL);
            let paragraph = Paragraph::new("  No state data available for roadmap.").block(block);
            frame.render_widget(paragraph, area);
        }
    }

    /// Render the backlog tab with list selection and optional split-pane content preview.
    fn render_backlog_tab(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        let cache = ctx.view_cache.get(&self.alias);

        let block = Block::default().borders(Borders::ALL);
        let inner = block.inner(area);
        frame.render_widget(block, area);

        if inner.height < 3 || inner.width < 10 {
            return;
        }

        let cache = match cache {
            Some(c) => c,
            None => {
                let msg =
                    Paragraph::new("  Loading...").style(Style::default().fg(Color::DarkGray));
                frame.render_widget(msg, inner);
                return;
            }
        };

        if cache.loading_backlog {
            let msg = Paragraph::new("  Loading...").style(Style::default().fg(Color::DarkGray));
            frame.render_widget(msg, inner);
            return;
        }

        if cache.backlog_items.is_empty() {
            let msg = Paragraph::new("  No backlog items found.");
            frame.render_widget(msg, inner);
            return;
        }

        // Build list items
        let items: Vec<ListItem> = cache
            .backlog_items
            .iter()
            .map(|item| {
                ListItem::new(Line::from(format!(
                    "{} - {}",
                    item.number, item.description
                )))
            })
            .collect();

        let list = List::new(items)
            .highlight_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        let mut list_state = ListState::default();
        list_state.select(Some(cache.backlog_selected));

        if cache.backlog_expanded {
            // Split-pane: 50/50 list on top, content on bottom
            let chunks = Layout::vertical([Constraint::Percentage(50), Constraint::Percentage(50)])
                .split(inner);

            frame.render_stateful_widget(list, chunks[0], &mut list_state);

            // Content pane for selected item
            let selected_item = cache.backlog_items.get(cache.backlog_selected);
            let title = selected_item
                .map(|item| format!(" Content: {} ", item.dir_name))
                .unwrap_or_else(|| " Content ".to_string());
            let content_block = Block::default().borders(Borders::ALL).title(title);

            let content_text = selected_item
                .and_then(|item| item.content.as_deref())
                .unwrap_or("  Empty — no .md files in this backlog directory");

            let style = if selected_item
                .and_then(|item| item.content.as_ref())
                .is_none()
            {
                Style::default().fg(Color::DarkGray)
            } else {
                Style::default()
            };

            let content_paragraph = Paragraph::new(content_text)
                .style(style)
                .block(content_block);
            frame.render_widget(content_paragraph, chunks[1]);
        } else {
            // Full-height list, no content pane
            frame.render_stateful_widget(list, inner, &mut list_state);
        }
    }

    /// Render the git history tab with scrollable log, mode indicator, and diff stat pane.
    fn render_git_tab(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        let cache = ctx.view_cache.get(&self.alias);

        let block = Block::default().borders(Borders::ALL);
        let inner = block.inner(area);
        frame.render_widget(block, area);

        if inner.height < 3 || inner.width < 10 {
            return;
        }

        let cache = match cache {
            Some(c) => c,
            None => {
                let msg = Paragraph::new("  Press 4 to load git history")
                    .style(Style::default().fg(Color::DarkGray));
                frame.render_widget(msg, inner);
                return;
            }
        };

        if cache.loading_git {
            let msg = Paragraph::new("  Loading git history...")
                .style(Style::default().fg(Color::DarkGray));
            frame.render_widget(msg, inner);
            return;
        }

        if cache.git_entries.is_empty() {
            let msg = Paragraph::new("  No commits found (or not a git repository)");
            frame.render_widget(msg, inner);
            return;
        }

        // Mode indicator line
        let mode_line = if cache.git_planning_only {
            Line::from(Span::styled(
                "  [.planning/ only]  Press 'p' to show full repo",
                Style::default().fg(Color::Yellow),
            ))
        } else {
            Line::from(Span::styled(
                "  [Full repo]  Press 'p' to show .planning/ only",
                Style::default().fg(Color::DarkGray),
            ))
        };

        // Layout: mode indicator (1 line), then log (and optionally diff stat)
        let has_diff = cache.git_diff_stat.is_some() || cache.loading_diff;
        let content_chunks = if has_diff {
            Layout::vertical([
                Constraint::Length(1),
                Constraint::Percentage(60),
                Constraint::Percentage(40),
            ])
            .split(inner)
        } else {
            Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(inner)
        };

        let mode_area = content_chunks[0];
        let log_area = content_chunks[1];

        frame.render_widget(Paragraph::new(mode_line), mode_area);

        // Build log list items
        let items: Vec<ListItem> = cache
            .git_entries
            .iter()
            .map(|entry| {
                ListItem::new(Line::from(vec![
                    Span::styled(&entry.hash, Style::default().fg(Color::Yellow)),
                    Span::raw(" -- "),
                    Span::raw(&entry.date),
                    Span::raw(" -- "),
                    Span::raw(&entry.message),
                    Span::raw("  "),
                    Span::styled(&entry.author, Style::default().fg(Color::DarkGray)),
                ]))
            })
            .collect();

        let list = List::new(items)
            .highlight_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        let mut list_state = ListState::default();
        list_state.select(Some(cache.git_selected));
        frame.render_stateful_widget(list, log_area, &mut list_state);

        // Render diff stat pane if present
        if has_diff {
            let diff_area = content_chunks[2];
            if cache.loading_diff {
                let diff_block = Block::default()
                    .borders(Borders::ALL)
                    .title(" Diff: loading... ");
                let loading = Paragraph::new("  Loading diff...")
                    .style(Style::default().fg(Color::DarkGray))
                    .block(diff_block);
                frame.render_widget(loading, diff_area);
            } else if let Some(stat) = &cache.git_diff_stat {
                let selected_hash = cache
                    .git_entries
                    .get(cache.git_selected)
                    .map(|e| e.hash.as_str())
                    .unwrap_or("???");
                let diff_block = Block::default()
                    .borders(Borders::ALL)
                    .title(format!(" Diff: {} ", selected_hash));

                let mut diff_lines: Vec<Line> = stat
                    .file_stats
                    .iter()
                    .map(|s| Line::from(format!("  {}", s)))
                    .collect();

                // Summary line with colored insertions/deletions
                diff_lines.push(Line::from(vec![
                    Span::raw(format!("  {} files changed, ", stat.files_changed)),
                    Span::styled(
                        format!("+{}", stat.insertions),
                        Style::default().fg(Color::Green),
                    ),
                    Span::raw(" "),
                    Span::styled(
                        format!("-{}", stat.deletions),
                        Style::default().fg(Color::Red),
                    ),
                ]));

                let diff_paragraph = Paragraph::new(diff_lines).block(diff_block);
                frame.render_widget(diff_paragraph, diff_area);
            }
        }
    }

    fn render_pipeline_tab(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        let alias = &self.alias;
        let state = ctx.project_states.get(alias);
        let cache = ctx.view_cache.get(alias);

        let state = match state {
            Some(s) => s,
            None => {
                let block = Block::default().borders(Borders::ALL).title(" Pipeline ");
                let msg = Paragraph::new("  No state data available.").block(block);
                frame.render_widget(msg, area);
                return;
            }
        };

        if state.phases.is_empty() {
            let block = Block::default().borders(Borders::ALL).title(" Pipeline ");
            let msg = Paragraph::new("  No phases found").block(block);
            frame.render_widget(msg, area);
            return;
        }

        // Split into left (phase list) and right (pipeline detail)
        let panes = Layout::horizontal([Constraint::Percentage(40), Constraint::Percentage(60)])
            .split(area);
        let left_area = panes[0];
        let right_area = panes[1];

        let selected = cache
            .map(|c| {
                c.pipeline_selected
                    .min(state.phases.len().saturating_sub(1))
            })
            .unwrap_or(0);

        // Left pane: phase list
        let items: Vec<ListItem> = state
            .phases
            .iter()
            .map(|phase| ListItem::new(format!("P{}: {}", phase.number, phase.name)))
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::RIGHT).title(" Phases "))
            .highlight_style(
                Style::default()
                    .add_modifier(Modifier::BOLD)
                    .fg(Color::Cyan),
            )
            .highlight_symbol("> ");

        let mut list_state = ListState::default();
        list_state.select(Some(selected));
        frame.render_stateful_widget(list, left_area, &mut list_state);

        // Right pane: pipeline detail for selected phase
        let phase = &state.phases[selected];
        let inference = state.phase_disk_statuses.get(&phase.number);

        let right_block = Block::default().borders(Borders::NONE).title(" Pipeline ");
        let inner = right_block.inner(right_area);
        frame.render_widget(right_block, right_area);

        match inference {
            None => {
                let msg = Paragraph::new("  No disk data");
                frame.render_widget(msg, inner);
            }
            Some(inf) => {
                let stage_statuses = derive_all_stage_statuses(inf);
                let pipeline_line = build_pipeline_line(inf, &stage_statuses);
                let detail_lines = build_stage_detail_lines(inf, &stage_statuses);

                let mut lines: Vec<Line> = Vec::new();
                lines.push(Line::from(format!(
                    "  Phase {}: {}",
                    phase.number, phase.name
                )));
                lines.push(Line::from(""));
                lines.push(pipeline_line);
                lines.push(Line::from(""));
                for dl in detail_lines {
                    lines.push(dl);
                }

                let paragraph = Paragraph::new(lines);
                frame.render_widget(paragraph, inner);
            }
        }
    }

    /// Render the queue tab with selectable list of queued actions.
    fn render_queue_tab(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        let alias = &self.alias;
        let state = ctx.project_states.get(alias);
        let cache = ctx.view_cache.get(alias);

        let block = Block::default().borders(Borders::ALL);
        let inner = block.inner(area);
        frame.render_widget(block, area);

        if inner.height < 3 || inner.width < 10 {
            return;
        }

        let queued_actions = state.map(|s| &s.queued_actions).filter(|q| !q.is_empty());

        match queued_actions {
            None => {
                let msg = Paragraph::new("  Queue empty -- press 'a' to add").style(
                    Style::default()
                        .fg(Color::DarkGray)
                        .add_modifier(Modifier::DIM),
                );
                frame.render_widget(msg, inner);
            }
            Some(actions) => {
                let selected = cache.map(|c| c.queue_selected).unwrap_or(0);

                let items: Vec<ListItem> = actions
                    .iter()
                    .map(|action| ListItem::new(Line::from(format!("  > {}", action.command))))
                    .collect();

                let title = format!(" Queue ({} items) ", actions.len());
                let list_block = Block::default().borders(Borders::ALL).title(title);
                let list = List::new(items)
                    .block(list_block)
                    .highlight_style(
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD)
                            .add_modifier(Modifier::UNDERLINED),
                    )
                    .highlight_symbol("> ");

                let mut list_state = ListState::default();
                list_state.select(Some(selected));
                frame.render_stateful_widget(list, inner, &mut list_state);
            }
        }
    }

    /// Render the sessions tab listing active Claude sessions for this project.
    fn render_sessions_tab(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        let alias = &self.alias;

        let block = Block::default().borders(Borders::ALL);
        let inner = block.inner(area);
        frame.render_widget(block, area);

        if inner.height < 3 || inner.width < 10 {
            return;
        }

        // Filter sessions by project path
        let filtered_sessions: Vec<_> = ctx
            .config
            .projects
            .get(alias)
            .map(|proj| {
                ctx.active_sessions
                    .iter()
                    .filter(|s| s.working_dir == proj.path)
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();

        if filtered_sessions.is_empty() {
            let lines = vec![
                Line::from(""),
                Line::from("  No active Claude sessions"),
                Line::from(""),
                Line::from(Span::styled(
                    "  [n] Launch new session",
                    Style::default().fg(Color::DarkGray),
                )),
            ];
            let msg = Paragraph::new(lines);
            frame.render_widget(msg, inner);
            return;
        }

        let selected = ctx
            .view_cache
            .get(alias)
            .map(|c| {
                c.sessions_selected
                    .min(filtered_sessions.len().saturating_sub(1))
            })
            .unwrap_or(0);

        let items: Vec<ListItem> = filtered_sessions
            .iter()
            .map(|session| {
                let sid_display = session
                    .session_id
                    .as_ref()
                    .map(|sid| {
                        if sid.len() > 8 {
                            sid[..8].to_string()
                        } else {
                            sid.clone()
                        }
                    })
                    .unwrap_or_else(|| "new session".to_string());
                let time_display = session
                    .start_time
                    .map(|_| "active".to_string())
                    .unwrap_or_else(|| "active".to_string());
                ListItem::new(Line::from(format!(
                    "  PID {} | Session: {} | {}",
                    session.pid, sid_display, time_display
                )))
            })
            .collect();

        let title = format!(" Sessions ({}) ", filtered_sessions.len());
        let list_block = Block::default().borders(Borders::ALL).title(title);
        let list = List::new(items)
            .block(list_block)
            .highlight_style(
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            )
            .highlight_symbol("> ");

        let mut list_state = ListState::default();
        list_state.select(Some(selected));
        frame.render_stateful_widget(list, inner, &mut list_state);
    }

    /// Render the archive tab with 4-level drill-down navigation.
    fn render_archive_tab(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        use crate::archive::ArchiveDepth;

        let cache = if let Some(c) = ctx.view_cache.get(&self.alias) {
            c
        } else {
            let loading = Paragraph::new("Loading...")
                .style(Style::default().fg(Color::DarkGray));
            frame.render_widget(loading, area);
            return;
        };

        // Split: breadcrumb (1 row) + content (remaining)
        let chunks =
            Layout::vertical([Constraint::Length(1), Constraint::Min(0)]).split(area);
        let breadcrumb_area = chunks[0];
        let content_area = chunks[1];

        // Render breadcrumb
        let breadcrumb = Self::archive_breadcrumb(&cache.archive_depth, cache, ctx);
        frame.render_widget(Paragraph::new(breadcrumb), breadcrumb_area);

        if cache.archive_loading {
            let loading = Paragraph::new("Loading...")
                .style(Style::default().fg(Color::DarkGray));
            frame.render_widget(loading, content_area);
            return;
        }

        match &cache.archive_depth {
            ArchiveDepth::MilestoneList => {
                if cache.archive_milestones.is_empty() {
                    let empty = Paragraph::new(
                        "No archived milestones found. Complete a milestone to see it here.",
                    )
                    .style(Style::default().fg(Color::DarkGray));
                    frame.render_widget(empty, content_area);
                    return;
                }
                let items: Vec<ListItem> = cache
                    .archive_milestones
                    .iter()
                    .map(|v| {
                        ListItem::new(Line::from(Span::styled(
                            v.clone(),
                            Style::default().fg(Color::Yellow),
                        )))
                    })
                    .collect();
                let list = List::new(items)
                    .highlight_style(
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )
                    .highlight_symbol("> ");
                let mut list_state = ListState::default();
                list_state.select(Some(cache.archive_selected[0]));
                frame.render_stateful_widget(list, content_area, &mut list_state);
            }
            ArchiveDepth::PhaseList { milestone } => {
                if let Some(data) = ctx.archive_cache.get(milestone) {
                    let mut items: Vec<ListItem> = Vec::new();
                    // Top-level milestone files first
                    for f in &data.top_level_files {
                        items.push(ListItem::new(Line::from(Span::styled(
                            format!("  {}", f.name),
                            Style::default().fg(Color::DarkGray),
                        ))));
                    }
                    // Then phases
                    for phase in &data.phases {
                        items.push(ListItem::new(Line::from(Span::raw(
                            phase.display_name.clone(),
                        ))));
                    }
                    if items.is_empty() {
                        let empty =
                            Paragraph::new("No phase artifacts found for this milestone.")
                                .style(Style::default().fg(Color::DarkGray));
                        frame.render_widget(empty, content_area);
                        return;
                    }
                    let list = List::new(items)
                        .highlight_style(
                            Style::default()
                                .fg(Color::Cyan)
                                .add_modifier(Modifier::BOLD),
                        )
                        .highlight_symbol("> ");
                    let mut list_state = ListState::default();
                    list_state.select(Some(cache.archive_selected[1]));
                    frame.render_stateful_widget(list, content_area, &mut list_state);
                } else {
                    let loading = Paragraph::new("Loading...")
                        .style(Style::default().fg(Color::DarkGray));
                    frame.render_widget(loading, content_area);
                }
            }
            ArchiveDepth::FileList {
                milestone,
                phase_idx,
            } => {
                if let Some(data) = ctx.archive_cache.get(milestone) {
                    if let Some(phase) = data.phases.get(*phase_idx) {
                        if phase.files.is_empty() {
                            let empty = Paragraph::new("No files found for this phase.")
                                .style(Style::default().fg(Color::DarkGray));
                            frame.render_widget(empty, content_area);
                            return;
                        }
                        let items: Vec<ListItem> = phase
                            .files
                            .iter()
                            .map(|f| ListItem::new(Line::from(Span::raw(f.name.clone()))))
                            .collect();
                        let list = List::new(items)
                            .highlight_style(
                                Style::default()
                                    .fg(Color::Cyan)
                                    .add_modifier(Modifier::BOLD),
                            )
                            .highlight_symbol("> ");
                        let mut list_state = ListState::default();
                        list_state.select(Some(cache.archive_selected[2]));
                        frame.render_stateful_widget(list, content_area, &mut list_state);
                    }
                }
            }
            ArchiveDepth::FileView { .. } => {
                if let Some(content) = &cache.archive_file_content {
                    let styled_lines = crate::archive::render_markdown_lines(content);
                    let total_lines = styled_lines.len() as u16;
                    // Split content area into gutter + main content
                    let gutter_width = (total_lines as usize).max(1).to_string().len() as u16 + 1;
                    let file_chunks = Layout::horizontal([
                        Constraint::Length(gutter_width),
                        Constraint::Min(0),
                    ])
                    .split(content_area);
                    let gutter_area = file_chunks[0];
                    let text_area = file_chunks[1];

                    let visible_height = text_area.height;
                    let max_scroll = total_lines.saturating_sub(visible_height);
                    let scroll = cache.archive_scroll_offset.min(max_scroll);

                    // Render line number gutter
                    let gutter_lines = crate::archive::line_number_lines(
                        total_lines as usize,
                        scroll,
                        visible_height,
                    );
                    let gutter = Paragraph::new(gutter_lines);
                    frame.render_widget(gutter, gutter_area);

                    // Render styled markdown content
                    let paragraph = Paragraph::new(styled_lines).scroll((scroll, 0));
                    frame.render_widget(paragraph, text_area);
                } else {
                    let loading = Paragraph::new("Loading...")
                        .style(Style::default().fg(Color::DarkGray));
                    frame.render_widget(loading, content_area);
                }
            }
        }
    }

    /// Build breadcrumb line for the archive tab showing navigation path.
    fn archive_breadcrumb(
        depth: &crate::archive::ArchiveDepth,
        cache: &super::ProjectViewCache,
        ctx: &AppContext,
    ) -> Line<'static> {
        use crate::archive::ArchiveDepth;
        let mut spans = vec![Span::styled(
            "Archive".to_string(),
            Style::default().fg(Color::Cyan),
        )];

        match depth {
            ArchiveDepth::MilestoneList => {}
            ArchiveDepth::PhaseList { milestone } => {
                spans.push(Span::raw(" > "));
                spans.push(Span::styled(
                    milestone.clone(),
                    Style::default().fg(Color::Yellow),
                ));
            }
            ArchiveDepth::FileList {
                milestone,
                phase_idx,
            } => {
                spans.push(Span::raw(" > "));
                spans.push(Span::styled(
                    milestone.clone(),
                    Style::default().fg(Color::Yellow),
                ));
                if let Some(data) = ctx.archive_cache.get(milestone) {
                    if let Some(phase) = data.phases.get(*phase_idx) {
                        spans.push(Span::raw(" > "));
                        spans.push(Span::raw(phase.display_name.clone()));
                    }
                }
            }
            ArchiveDepth::FileView {
                milestone,
                phase_idx,
                ..
            } => {
                spans.push(Span::raw(" > "));
                spans.push(Span::styled(
                    milestone.clone(),
                    Style::default().fg(Color::Yellow),
                ));
                if let Some(idx) = phase_idx {
                    if let Some(data) = ctx.archive_cache.get(milestone) {
                        if let Some(phase) = data.phases.get(*idx) {
                            spans.push(Span::raw(" > "));
                            spans.push(Span::raw(phase.display_name.clone()));
                        }
                    }
                }
                if let Some(ref name) = cache.archive_file_name {
                    spans.push(Span::raw(" > "));
                    spans.push(Span::styled(
                        name.clone(),
                        Style::default().add_modifier(Modifier::BOLD),
                    ));
                }
            }
        }
        Line::from(spans)
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
        let chunks = Layout::vertical([Constraint::Length(3), Constraint::Min(0)]).split(main_area);
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
            DetailSubView::Backlog => self.render_backlog_tab(frame, content_area, ctx),
            DetailSubView::GitHistory => self.render_git_tab(frame, content_area, ctx),
            DetailSubView::Pipeline => self.render_pipeline_tab(frame, content_area, ctx),
            DetailSubView::Queue => self.render_queue_tab(frame, content_area, ctx),
            DetailSubView::Sessions => self.render_sessions_tab(frame, content_area, ctx),
            DetailSubView::Archive => self.render_archive_tab(frame, content_area, ctx),
            DetailSubView::Defaults => self.render_defaults_tab(frame, content_area, ctx),
        }
    }

    fn render_defaults_tab(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        let cache = ctx.view_cache.get(&self.alias);
        let config = cache.and_then(|c| c.defaults_config.as_ref());
        let selected = cache.map(|c| c.defaults_selected).unwrap_or(0);

        let config = match config {
            Some(c) => c,
            None => {
                let msg = Paragraph::new("  No config loaded (project may not have .planning/config.json)")
                    .style(Style::default().fg(Color::DarkGray));
                frame.render_widget(msg, area);
                return;
            }
        };

        let entries = build_defaults_entries(config);
        let items: Vec<ListItem> = entries
            .iter()
            .enumerate()
            .map(|(i, entry)| {
                let cat_span = if entry.show_category {
                    Span::styled(
                        format!("{:<10}", entry.category),
                        Style::default().fg(Color::DarkGray),
                    )
                } else {
                    Span::raw("          ")
                };
                let key_span = Span::styled(
                    format!("{:<30}", entry.key),
                    Style::default().fg(Color::White),
                );
                let val_style = match entry.kind {
                    ConfigValueKind::Bool => {
                        if entry.value == "true" {
                            Style::default().fg(Color::Green)
                        } else if entry.value == "false" {
                            Style::default().fg(Color::Red)
                        } else {
                            Style::default().fg(Color::DarkGray)
                        }
                    }
                    ConfigValueKind::Null => Style::default().fg(Color::DarkGray),
                    _ => Style::default().fg(Color::Yellow),
                };
                let val_span = Span::styled(&entry.value, val_style);
                let line = Line::from(vec![
                    Span::raw("  "),
                    cat_span,
                    Span::raw(" "),
                    key_span,
                    val_span,
                ]);
                let item = ListItem::new(line);
                if i == selected {
                    item.style(Style::default().bg(Color::DarkGray).fg(Color::Cyan))
                } else {
                    item
                }
            })
            .collect();

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::TOP)
                .title(" Config Settings "),
        );

        let mut list_state = ListState::default();
        list_state.select(Some(selected));
        frame.render_stateful_widget(list, area, &mut list_state);

        // Render dropdown overlay if editing
        if let Some(cache) = cache {
            if let Some(editing_idx) = cache.defaults_editing {
                if let Some(entry) = entries.get(editing_idx) {
                    let options = dropdown_options(&entry.kind);
                    if !options.is_empty() {
                        let dropdown_idx =
                            cache.defaults_dropdown_selected.min(options.len() - 1);
                        let title = format!(" {} ", entry.key);
                        let max_opt_width = options.iter().map(|s| s.len()).max().unwrap_or(0);
                        let inner_w = max_opt_width.max(title.len() + 2).max(20) as u16;
                        let popup_w = (inner_w + 4).min(area.width.saturating_sub(2));
                        let popup_h = (options.len() as u16 + 2).min(area.height.saturating_sub(2));
                        let popup_x =
                            area.x + (area.width.saturating_sub(popup_w)) / 2;
                        let popup_y =
                            area.y + (area.height.saturating_sub(popup_h)) / 2;
                        let popup_area = Rect {
                            x: popup_x,
                            y: popup_y,
                            width: popup_w,
                            height: popup_h,
                        };

                        // Clear underneath the popup so the list doesn't bleed through
                        frame.render_widget(ratatui::widgets::Clear, popup_area);

                        let opt_items: Vec<ListItem> = options
                            .iter()
                            .enumerate()
                            .map(|(i, opt)| {
                                let style = if i == dropdown_idx {
                                    Style::default()
                                        .bg(Color::Cyan)
                                        .fg(Color::Black)
                                        .add_modifier(Modifier::BOLD)
                                } else {
                                    Style::default().fg(Color::White)
                                };
                                let marker = if opt == &entry.value { "● " } else { "  " };
                                ListItem::new(Line::from(vec![
                                    Span::raw(marker),
                                    Span::raw(opt.clone()),
                                ]))
                                .style(style)
                            })
                            .collect();
                        let popup = List::new(opt_items).block(
                            Block::default()
                                .borders(Borders::ALL)
                                .border_style(Style::default().fg(Color::Cyan))
                                .title(title),
                        );
                        frame.render_widget(popup, popup_area);
                    }
                }
            }
        }
    }
}

// --- Pipeline stage status logic ---

#[derive(Debug, Clone, Copy, PartialEq)]
enum StageStatus {
    Complete,
    Current,
    Skipped,
    NotStarted,
}

/// Stage names for the 5 GSD pipeline stages.
const STAGE_LABELS: [&str; 5] = ["D", "R", "P", "E", "V"];
const STAGE_NAMES: [&str; 5] = ["Discuss", "Research", "Plan", "Execute", "Verify"];

/// Determine the status of each of the 5 pipeline stages from DiskInference.
fn derive_all_stage_statuses(inf: &DiskInference) -> [StageStatus; 5] {
    // Whether each stage's artifact is present
    let present = [
        inf.has_context,       // D: Discuss
        inf.has_research,      // R: Research
        inf.has_plans,         // P: Plan
        inf.summary_count > 0, // E: Execute (at least one)
        inf.has_verification,  // V: Verify
    ];

    // Execute is "complete" only if summary_count >= plan_count and plan_count > 0
    let execute_complete = inf.plan_count > 0 && inf.summary_count >= inf.plan_count;

    let mut statuses = [StageStatus::NotStarted; 5];

    for i in 0..5 {
        if present[i] {
            if i == 3 && !execute_complete {
                // Execute stage: present but not fully complete
                statuses[i] = StageStatus::Current;
            } else {
                statuses[i] = StageStatus::Complete;
            }
        } else {
            // Check if any later stage is present (skip detection per D-10)
            let any_later = present[i + 1..].iter().any(|&p| p);
            if any_later {
                statuses[i] = StageStatus::Skipped;
            } else {
                // Check if this is the next expected stage after the last complete one
                let last_complete = (0..i).rev().find(|&j| present[j]);
                if let Some(lc) = last_complete {
                    if lc == i - 1 {
                        // Immediately after a complete stage
                        statuses[i] = StageStatus::Current;
                    }
                } else if i == 0 {
                    // First stage, nothing complete yet -- it's the current one
                    // only if the overall status isn't NoDirectory/Empty
                    if inf.status != DiskStatus::NoDirectory && inf.status != DiskStatus::Empty {
                        statuses[i] = StageStatus::Current;
                    }
                }
            }
        }
    }

    statuses
}

fn stage_color(status: StageStatus) -> Color {
    match status {
        StageStatus::Complete => Color::Green,
        StageStatus::Current => Color::Yellow,
        StageStatus::Skipped => Color::Magenta,
        StageStatus::NotStarted => Color::DarkGray,
    }
}

/// Build the horizontal pipeline line: [D]---[R]---[P]---[E 2/3]---[V]
fn build_pipeline_line(inf: &DiskInference, statuses: &[StageStatus; 5]) -> Line<'static> {
    let mut spans: Vec<Span> = Vec::new();
    spans.push(Span::raw("  "));

    for (i, &status) in statuses.iter().enumerate() {
        let color = stage_color(status);
        let label = if status == StageStatus::Skipped {
            "[--]".to_string()
        } else if i == 3 && inf.plan_count > 0 {
            // Execute stage with plan fraction
            format!("[E {}/{}]", inf.summary_count, inf.plan_count)
        } else {
            format!("[{}]", STAGE_LABELS[i])
        };

        let style = if status == StageStatus::Skipped {
            Style::default().fg(color).add_modifier(Modifier::DIM)
        } else {
            Style::default().fg(color)
        };

        spans.push(Span::styled(label, style));

        if i < 4 {
            spans.push(Span::styled("---", Style::default().fg(Color::DarkGray)));
        }
    }

    Line::from(spans)
}

/// Build detail lines showing each stage's status text.
fn build_stage_detail_lines(
    inf: &DiskInference,
    statuses: &[StageStatus; 5],
) -> Vec<Line<'static>> {
    let mut lines = Vec::new();

    for (i, &status) in statuses.iter().enumerate() {
        let color = stage_color(status);
        let detail = match (i, status) {
            (3, StageStatus::Complete) => {
                format!("{}/{} complete", inf.summary_count, inf.plan_count)
            }
            (3, StageStatus::Current) => {
                format!("{}/{} complete", inf.summary_count, inf.plan_count)
            }
            (2, StageStatus::Complete) if inf.plan_count > 0 => format!("{} plans", inf.plan_count),
            (_, StageStatus::Complete) => "Complete".to_string(),
            (_, StageStatus::Current) => "Current".to_string(),
            (_, StageStatus::Skipped) => "Skipped".to_string(),
            (_, StageStatus::NotStarted) => "Not started".to_string(),
        };

        lines.push(Line::from(vec![
            Span::raw(format!("  {:<12}", format!("{}:", STAGE_NAMES[i]))),
            Span::styled(detail, Style::default().fg(color)),
        ]));
    }

    lines
}

/// Build the footer line with tab-appropriate key hints.
fn build_footer(sub_view: &DetailSubView) -> Paragraph<'static> {
    let b = Style::default().add_modifier(Modifier::BOLD);
    let mut spans = vec![
        Span::raw("  "),
        Span::styled("[Esc]", b),
        Span::raw("back  "),
        Span::styled("[1-9]", b),
        Span::raw("tabs  "),
        Span::styled("[j/k]", b),
        Span::raw("scroll  "),
    ];

    match sub_view {
        DetailSubView::Backlog => {
            spans.push(Span::styled("[Enter]", b));
            spans.push(Span::raw("xpand  "));
            spans.push(Span::styled("[e]", b));
            spans.push(Span::raw("nqueue  "));
        }
        DetailSubView::GitHistory => {
            spans.push(Span::styled("[p]", b));
            spans.push(Span::raw("lanning-only  "));
            spans.push(Span::styled("[Enter]", b));
            spans.push(Span::raw("diff  "));
        }
        DetailSubView::Queue => {
            spans.push(Span::styled("[a]", b));
            spans.push(Span::raw("dd  "));
            spans.push(Span::styled("[e]", b));
            spans.push(Span::raw("dit  "));
            spans.push(Span::styled("[d]", b));
            spans.push(Span::raw("el  "));
            spans.push(Span::styled("[Enter]", b));
            spans.push(Span::raw("done  "));
            spans.push(Span::styled("[J/K]", b));
            spans.push(Span::raw("reorder  "));
        }
        DetailSubView::Sessions => {
            spans.push(Span::styled("[Enter]", b));
            spans.push(Span::raw("resume  "));
            spans.push(Span::styled("[n]", b));
            spans.push(Span::raw("ew session  "));
        }
        DetailSubView::Archive => {
            spans.push(Span::styled("[Enter]", b));
            spans.push(Span::raw("open  "));
            spans.push(Span::styled("[e]", b));
            spans.push(Span::raw("dit  "));
        }
        DetailSubView::Defaults => {
            spans.push(Span::styled("[Enter]", b));
            spans.push(Span::raw("toggle/cycle  "));
            spans.push(Span::styled("[r]", b));
            spans.push(Span::raw("eload  "));
        }
        _ => {
            spans.push(Span::styled("[e]", b));
            spans.push(Span::raw("nqueue  "));
        }
    }

    spans.push(Span::styled("[?]", b));
    spans.push(Span::raw("help"));

    Paragraph::new(Line::from(spans))
}

// --- Defaults tab helpers ---

#[derive(Debug, Clone)]
enum ConfigValueKind {
    Bool,
    Enum(&'static [&'static str]),
    String,
    Integer,
    Null,
}

struct ConfigEntry {
    category: &'static str,
    key: &'static str,
    value: String,
    kind: ConfigValueKind,
    show_category: bool,
}

fn opt_bool_display(v: &Option<bool>) -> (String, ConfigValueKind) {
    match v {
        Some(b) => (b.to_string(), ConfigValueKind::Bool),
        None => ("(unset)".to_string(), ConfigValueKind::Null),
    }
}

fn opt_str_display(v: &Option<String>) -> (String, ConfigValueKind) {
    match v {
        Some(s) => (s.clone(), ConfigValueKind::String),
        None => ("(unset)".to_string(), ConfigValueKind::Null),
    }
}

fn opt_u32_display(v: &Option<u32>) -> (String, ConfigValueKind) {
    match v {
        Some(n) => (n.to_string(), ConfigValueKind::Integer),
        None => ("(unset)".to_string(), ConfigValueKind::Null),
    }
}

fn build_defaults_entries(config: &crate::state_reader::config_json::GsdConfig) -> Vec<ConfigEntry> {
    let mut entries = Vec::new();
    let mut push = |cat: &'static str, key: &'static str, value: String, kind: ConfigValueKind, first: bool| {
        entries.push(ConfigEntry {
            category: cat,
            key,
            value,
            kind,
            show_category: first,
        });
    };

    // General
    let cat = "General";
    push(cat, "mode", config.mode.clone(), ConfigValueKind::Enum(&["interactive", "yolo"]), true);
    push(cat, "granularity", config.granularity.clone(), ConfigValueKind::Enum(&["coarse", "standard", "fine"]), false);
    push(cat, "model_profile", config.model_profile.clone(), ConfigValueKind::Enum(&["quality", "balanced", "budget", "inherit"]), false);
    let (v, k) = opt_bool_display(&config.commit_docs);
    push(cat, "commit_docs", v, k, false);
    let (v, k) = opt_bool_display(&config.parallelization);
    push(cat, "parallelization", v, k, false);
    let (v, k) = opt_str_display(&config.project_code);
    push(cat, "project_code", v, k, false);
    let (v, k) = opt_str_display(&config.phase_naming);
    push(cat, "phase_naming", v, k, false);
    let (v, k) = opt_str_display(&config.response_language);
    push(cat, "response_language", v, k, false);

    // Search
    let cat = "Search";
    let (v, k) = opt_bool_display(&config.search_gitignored);
    push(cat, "search_gitignored", v, k, true);
    let (v, k) = opt_bool_display(&config.brave_search);
    push(cat, "brave_search", v, k, false);
    let (v, k) = opt_bool_display(&config.firecrawl);
    push(cat, "firecrawl", v, k, false);
    let (v, k) = opt_bool_display(&config.exa_search);
    push(cat, "exa_search", v, k, false);

    // Git
    let cat = "Git";
    let git = config.git.as_ref();
    let (v, k) = match git.and_then(|g| g.branching_strategy.as_ref()) {
        Some(s) => (s.clone(), ConfigValueKind::Enum(&["none", "phase", "milestone"])),
        None => ("(unset)".to_string(), ConfigValueKind::Null),
    };
    push(cat, "branching_strategy", v, k, true);
    let (v, k) = opt_str_display(&git.and_then(|g| g.base_branch.clone()));
    push(cat, "base_branch", v, k, false);
    let (v, k) = opt_str_display(&git.and_then(|g| g.phase_branch_template.clone()));
    push(cat, "phase_branch_template", v, k, false);
    let (v, k) = opt_str_display(&git.and_then(|g| g.milestone_branch_template.clone()));
    push(cat, "milestone_branch_template", v, k, false);
    let qbt_val = git.and_then(|g| g.quick_branch_template.as_ref()).map(|v| v.to_string()).unwrap_or("(unset)".to_string());
    let qbt_kind = if qbt_val == "(unset)" { ConfigValueKind::Null } else { ConfigValueKind::String };
    push(cat, "quick_branch_template", qbt_val, qbt_kind, false);

    // Workflow
    let cat = "Workflow";
    let wf = config.workflow.as_ref();
    let (v, k) = opt_bool_display(&wf.and_then(|w| w.research));
    push(cat, "research", v, k, true);
    let (v, k) = opt_bool_display(&wf.and_then(|w| w.plan_check));
    push(cat, "plan_check", v, k, false);
    let (v, k) = opt_bool_display(&wf.and_then(|w| w.verifier));
    push(cat, "verifier", v, k, false);
    let (v, k) = opt_bool_display(&wf.and_then(|w| w.nyquist_validation));
    push(cat, "nyquist_validation", v, k, false);
    let (v, k) = opt_bool_display(&wf.and_then(|w| w.auto_advance));
    push(cat, "auto_advance", v, k, false);
    let (v, k) = opt_bool_display(&wf.and_then(|w| w.node_repair));
    push(cat, "node_repair", v, k, false);
    let (v, k) = opt_u32_display(&wf.and_then(|w| w.node_repair_budget));
    push(cat, "node_repair_budget", v, k, false);
    let (v, k) = opt_bool_display(&wf.and_then(|w| w.ui_phase));
    push(cat, "ui_phase", v, k, false);
    let (v, k) = opt_bool_display(&wf.and_then(|w| w.ui_safety_gate));
    push(cat, "ui_safety_gate", v, k, false);
    let (v, k) = opt_bool_display(&wf.and_then(|w| w.text_mode));
    push(cat, "text_mode", v, k, false);
    let (v, k) = opt_bool_display(&wf.and_then(|w| w.research_before_questions));
    push(cat, "research_before_questions", v, k, false);
    let (v, k) = match wf.and_then(|w| w.discuss_mode.as_ref()) {
        Some(s) => (s.clone(), ConfigValueKind::Enum(&["discuss", "assumptions"])),
        None => ("(unset)".to_string(), ConfigValueKind::Null),
    };
    push(cat, "discuss_mode", v, k, false);
    let (v, k) = opt_bool_display(&wf.and_then(|w| w.skip_discuss));
    push(cat, "skip_discuss", v, k, false);
    let (v, k) = opt_bool_display(&wf.and_then(|w| w.auto_chain_active));
    push(cat, "auto_chain_active", v, k, false);
    let (v, k) = opt_bool_display(&wf.and_then(|w| w.use_worktrees));
    push(cat, "use_worktrees", v, k, false);
    let (v, k) = opt_u32_display(&wf.and_then(|w| w.subagent_timeout));
    push(cat, "subagent_timeout", v, k, false);

    // Hooks
    let cat = "Hooks";
    let (v, k) = opt_bool_display(&config.hooks.as_ref().and_then(|h| h.context_warnings));
    push(cat, "context_warnings", v, k, true);

    entries
}

fn defaults_entry_count(config: &Option<crate::state_reader::config_json::GsdConfig>) -> usize {
    match config {
        Some(c) => build_defaults_entries(c).len(),
        None => 0,
    }
}

/// Returns the list of selectable values for an entry's kind, or empty if the
/// list is not statically known (e.g. free-form strings, integers).
fn dropdown_options(kind: &ConfigValueKind) -> Vec<String> {
    match kind {
        ConfigValueKind::Bool => vec!["true".to_string(), "false".to_string()],
        ConfigValueKind::Enum(opts) => opts.iter().map(|s| s.to_string()).collect(),
        _ => Vec::new(),
    }
}

/// Set a bool/enum config field to a specific value (selected from a dropdown).
/// Returns true if the field was recognized and updated.
fn set_config_value(
    config: &mut crate::state_reader::config_json::GsdConfig,
    key: &str,
    value: &str,
) -> bool {
    use crate::state_reader::config_json::*;

    let parse_bool = |v: &str| match v {
        "true" => Some(true),
        "false" => Some(false),
        _ => None,
    };

    if let Some(b) = parse_bool(value) {
        match key {
            "commit_docs" => { config.commit_docs = Some(b); return true; }
            "parallelization" => { config.parallelization = Some(b); return true; }
            "search_gitignored" => { config.search_gitignored = Some(b); return true; }
            "brave_search" => { config.brave_search = Some(b); return true; }
            "firecrawl" => { config.firecrawl = Some(b); return true; }
            "exa_search" => { config.exa_search = Some(b); return true; }
            "research" => { config.workflow.get_or_insert_with(WorkflowConfig::default).research = Some(b); return true; }
            "plan_check" => { config.workflow.get_or_insert_with(WorkflowConfig::default).plan_check = Some(b); return true; }
            "verifier" => { config.workflow.get_or_insert_with(WorkflowConfig::default).verifier = Some(b); return true; }
            "nyquist_validation" => { config.workflow.get_or_insert_with(WorkflowConfig::default).nyquist_validation = Some(b); return true; }
            "auto_advance" => { config.workflow.get_or_insert_with(WorkflowConfig::default).auto_advance = Some(b); return true; }
            "node_repair" => { config.workflow.get_or_insert_with(WorkflowConfig::default).node_repair = Some(b); return true; }
            "ui_phase" => { config.workflow.get_or_insert_with(WorkflowConfig::default).ui_phase = Some(b); return true; }
            "ui_safety_gate" => { config.workflow.get_or_insert_with(WorkflowConfig::default).ui_safety_gate = Some(b); return true; }
            "text_mode" => { config.workflow.get_or_insert_with(WorkflowConfig::default).text_mode = Some(b); return true; }
            "research_before_questions" => { config.workflow.get_or_insert_with(WorkflowConfig::default).research_before_questions = Some(b); return true; }
            "skip_discuss" => { config.workflow.get_or_insert_with(WorkflowConfig::default).skip_discuss = Some(b); return true; }
            "auto_chain_active" => { config.workflow.get_or_insert_with(WorkflowConfig::default).auto_chain_active = Some(b); return true; }
            "use_worktrees" => { config.workflow.get_or_insert_with(WorkflowConfig::default).use_worktrees = Some(b); return true; }
            "context_warnings" => { config.hooks.get_or_insert_with(HooksConfig::default).context_warnings = Some(b); return true; }
            _ => {}
        }
    }

    // Enum string values
    match key {
        "mode" => { config.mode = value.to_string(); true }
        "granularity" => { config.granularity = value.to_string(); true }
        "model_profile" => { config.model_profile = value.to_string(); true }
        "branching_strategy" => {
            config.git.get_or_insert_with(GitConfig::default).branching_strategy = Some(value.to_string());
            true
        }
        "discuss_mode" => {
            config.workflow.get_or_insert_with(WorkflowConfig::default).discuss_mode = Some(value.to_string());
            true
        }
        _ => false,
    }
}

/// Mutate the config field identified by `key`. Returns true if a mutation was made.
fn mutate_config_entry(
    config: &mut crate::state_reader::config_json::GsdConfig,
    key: &str,
    kind: &ConfigValueKind,
) -> bool {
    use crate::state_reader::config_json::*;

    match kind {
        ConfigValueKind::Null => false, // Cannot toggle unset values without initializing parent
        ConfigValueKind::String => false, // String editing not supported via Enter
        ConfigValueKind::Bool => {
            // Toggle the boolean field
            match key {
                "commit_docs" => { config.commit_docs = Some(!config.commit_docs.unwrap_or(false)); true }
                "parallelization" => { config.parallelization = Some(!config.parallelization.unwrap_or(false)); true }
                "search_gitignored" => { config.search_gitignored = Some(!config.search_gitignored.unwrap_or(false)); true }
                "brave_search" => { config.brave_search = Some(!config.brave_search.unwrap_or(false)); true }
                "firecrawl" => { config.firecrawl = Some(!config.firecrawl.unwrap_or(false)); true }
                "exa_search" => { config.exa_search = Some(!config.exa_search.unwrap_or(false)); true }
                "research" => { let wf = config.workflow.get_or_insert_with(WorkflowConfig::default); wf.research = Some(!wf.research.unwrap_or(false)); true }
                "plan_check" => { let wf = config.workflow.get_or_insert_with(WorkflowConfig::default); wf.plan_check = Some(!wf.plan_check.unwrap_or(false)); true }
                "verifier" => { let wf = config.workflow.get_or_insert_with(WorkflowConfig::default); wf.verifier = Some(!wf.verifier.unwrap_or(false)); true }
                "nyquist_validation" => { let wf = config.workflow.get_or_insert_with(WorkflowConfig::default); wf.nyquist_validation = Some(!wf.nyquist_validation.unwrap_or(false)); true }
                "auto_advance" => { let wf = config.workflow.get_or_insert_with(WorkflowConfig::default); wf.auto_advance = Some(!wf.auto_advance.unwrap_or(false)); true }
                "node_repair" => { let wf = config.workflow.get_or_insert_with(WorkflowConfig::default); wf.node_repair = Some(!wf.node_repair.unwrap_or(false)); true }
                "ui_phase" => { let wf = config.workflow.get_or_insert_with(WorkflowConfig::default); wf.ui_phase = Some(!wf.ui_phase.unwrap_or(false)); true }
                "ui_safety_gate" => { let wf = config.workflow.get_or_insert_with(WorkflowConfig::default); wf.ui_safety_gate = Some(!wf.ui_safety_gate.unwrap_or(false)); true }
                "text_mode" => { let wf = config.workflow.get_or_insert_with(WorkflowConfig::default); wf.text_mode = Some(!wf.text_mode.unwrap_or(false)); true }
                "research_before_questions" => { let wf = config.workflow.get_or_insert_with(WorkflowConfig::default); wf.research_before_questions = Some(!wf.research_before_questions.unwrap_or(false)); true }
                "skip_discuss" => { let wf = config.workflow.get_or_insert_with(WorkflowConfig::default); wf.skip_discuss = Some(!wf.skip_discuss.unwrap_or(false)); true }
                "auto_chain_active" => { let wf = config.workflow.get_or_insert_with(WorkflowConfig::default); wf.auto_chain_active = Some(!wf.auto_chain_active.unwrap_or(false)); true }
                "use_worktrees" => { let wf = config.workflow.get_or_insert_with(WorkflowConfig::default); wf.use_worktrees = Some(!wf.use_worktrees.unwrap_or(false)); true }
                "context_warnings" => { let hooks = config.hooks.get_or_insert_with(HooksConfig::default); hooks.context_warnings = Some(!hooks.context_warnings.unwrap_or(false)); true }
                _ => false,
            }
        }
        ConfigValueKind::Enum(options) => {
            let cycle = |current: &str| -> String {
                let idx = options.iter().position(|&o| o == current).unwrap_or(0);
                let next = (idx + 1) % options.len();
                options[next].to_string()
            };
            match key {
                "mode" => { config.mode = cycle(&config.mode); true }
                "granularity" => { config.granularity = cycle(&config.granularity); true }
                "model_profile" => { config.model_profile = cycle(&config.model_profile); true }
                "branching_strategy" => {
                    let git = config.git.get_or_insert_with(GitConfig::default);
                    let current = git.branching_strategy.as_deref().unwrap_or("none");
                    git.branching_strategy = Some(cycle(current));
                    true
                }
                "discuss_mode" => {
                    let wf = config.workflow.get_or_insert_with(WorkflowConfig::default);
                    let current = wf.discuss_mode.as_deref().unwrap_or("discuss");
                    wf.discuss_mode = Some(cycle(current));
                    true
                }
                _ => false,
            }
        }
        ConfigValueKind::Integer => {
            match key {
                "node_repair_budget" => {
                    let wf = config.workflow.get_or_insert_with(WorkflowConfig::default);
                    let current = wf.node_repair_budget.unwrap_or(0);
                    wf.node_repair_budget = Some(if current >= 10 { 0 } else { current + 1 });
                    true
                }
                "subagent_timeout" => {
                    let wf = config.workflow.get_or_insert_with(WorkflowConfig::default);
                    let current = wf.subagent_timeout.unwrap_or(0);
                    wf.subagent_timeout = Some(if current >= 600 { 60 } else { current + 30 });
                    true
                }
                _ => false,
            }
        }
    }
}
