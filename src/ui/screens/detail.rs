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

const TAB_TITLES: [&str; 10] = [
    "1:Phases",
    "2:Roadmap",
    "3:Backlog",
    "4:Git",
    "5:Pipe",
    "6:Queue",
    "7:Sess",
    "8:Arch",
    "9:Cfg",
    "0:Docs",
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
        DetailSubView::Browse => 9,
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
        9 => DetailSubView::Browse,
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

    // Load config.json + ~/.gsd/defaults.json for defaults tab
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
        cache.defaults_user_config =
            crate::state_reader::config_json::load_user_defaults();
        cache.defaults_selected = 0;
        cache.defaults_editing = None;
        cache.defaults_dropdown_selected = 0;
        cache.defaults_text_buffer.clear();
    }

    // Lazy-init the docs browser on first visit: resolve the active phase
    // directory (or `.planning/` root if milestone complete) and populate
    // the entry list. Re-running this on subsequent visits would wipe any
    // navigation state, so we only run it when `browser_current_dir` is None.
    if new_view == DetailSubView::Browse {
        let project_path = ctx
            .config
            .projects
            .get(alias)
            .map(|p| p.path.clone());
        let project_state = ctx.project_states.get(alias).cloned();
        let cache = ctx.view_cache.entry(alias.to_string()).or_default();
        if cache.browser_current_dir.is_none() {
            if let (Some(path), Some(state)) = (project_path, project_state) {
                let planning_dir = path.join(".planning");
                let entry_dir = crate::browser::resolve_active_phase_dir(&planning_dir, &state);
                cache.browser_entries = crate::browser::list_dir(&entry_dir);
                cache.browser_root = Some(planning_dir);
                cache.browser_entry_dir = Some(entry_dir.clone());
                cache.browser_current_dir = Some(entry_dir);
                cache.browser_selected = 0;
                cache.browser_scroll_offset = 0;
                cache.browser_depth = crate::browser::BrowserDepth::List;
            }
        }
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

        // Text-input intercept: when the Defaults tab has a String entry being
        // edited, route all keystrokes to the input buffer so character keys
        // ('q', 'x', 'r', etc.) don't trigger their global shortcuts.
        if current_view == DetailSubView::Defaults {
            let editing_text_idx = {
                let cache = ctx.view_cache.get(&self.alias);
                let editing = cache.and_then(|c| c.defaults_editing);
                editing.and_then(|idx| {
                    cache
                        .map(entries_for_cache)
                        .and_then(|entries| entries.into_iter().nth(idx))
                        .filter(|e| matches!(e.kind, ConfigValueKind::String))
                        .map(|_| idx)
                })
            };
            if let Some(editing_idx) = editing_text_idx {
                return self.handle_text_input_key(code, ctx, editing_idx);
            }
        }

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
                // Browse: drop View→List, walk up one dir, or fall through to pop
                if current_view == DetailSubView::Browse {
                    let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                    use crate::browser::BrowserDepth;
                    match cache.browser_depth {
                        BrowserDepth::View => {
                            cache.browser_depth = BrowserDepth::List;
                            cache.browser_file_content = None;
                            cache.browser_file_name = None;
                            cache.browser_scroll_offset = 0;
                            ctx.needs_redraw = true;
                            return ScreenAction::None;
                        }
                        BrowserDepth::List => {
                            let current = cache.browser_current_dir.clone();
                            let root = cache.browser_root.clone();
                            if let (Some(current), Some(root)) = (current, root) {
                                if current != root {
                                    if let Some(parent) = current.parent() {
                                        let parent = parent.to_path_buf();
                                        cache.browser_entries = crate::browser::list_dir(&parent);
                                        cache.browser_current_dir = Some(parent);
                                        cache.browser_selected = 0;
                                        cache.browser_scroll_offset = 0;
                                        ctx.needs_redraw = true;
                                        return ScreenAction::None;
                                    }
                                }
                            }
                            // At the .planning/ root: fall through to pop screen
                        }
                    }
                }
                // If a config dropdown is open on the Defaults tab, close it first
                if current_view == DetailSubView::Defaults {
                    let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                    if cache.defaults_editing.is_some() {
                        cache.defaults_editing = None;
                        cache.defaults_dropdown_selected = 0;
                        cache.defaults_text_buffer.clear();
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
                            let entries = entries_for_cache(cache);
                            if let Some(entry) = entries.get(editing_idx) {
                                let options = dropdown_options(&entry.kind);
                                if !options.is_empty() {
                                    let max = options.len() - 1;
                                    cache.defaults_dropdown_selected =
                                        (cache.defaults_dropdown_selected + 1).min(max);
                                }
                            }
                        } else {
                            let entry_count = entries_count_for_cache(cache);
                            if entry_count > 0 {
                                let max = entry_count.saturating_sub(1);
                                cache.defaults_selected = (cache.defaults_selected + 1).min(max);
                            }
                        }
                        ctx.needs_redraw = true;
                    }
                    DetailSubView::Browse => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        use crate::browser::BrowserDepth;
                        match cache.browser_depth {
                            BrowserDepth::List => {
                                if !cache.browser_entries.is_empty() {
                                    let max = cache.browser_entries.len().saturating_sub(1);
                                    cache.browser_selected =
                                        (cache.browser_selected + 1).min(max);
                                }
                            }
                            BrowserDepth::View => {
                                cache.browser_scroll_offset =
                                    cache.browser_scroll_offset.saturating_add(1);
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
                    DetailSubView::Browse => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        use crate::browser::BrowserDepth;
                        match cache.browser_depth {
                            BrowserDepth::List => {
                                cache.browser_selected =
                                    cache.browser_selected.saturating_sub(1);
                            }
                            BrowserDepth::View => {
                                cache.browser_scroll_offset =
                                    cache.browser_scroll_offset.saturating_sub(1);
                            }
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
                        let entry_count = entries_count_for_cache(cache);
                        if entry_count > 0 {
                            let max = entry_count.saturating_sub(1);
                            cache.defaults_selected = (cache.defaults_selected + PAGE_SCROLL_LINES as usize).min(max);
                        }
                        ctx.needs_redraw = true;
                    }
                    DetailSubView::Browse => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        use crate::browser::BrowserDepth;
                        match cache.browser_depth {
                            BrowserDepth::List => {
                                if !cache.browser_entries.is_empty() {
                                    let max = cache.browser_entries.len().saturating_sub(1);
                                    cache.browser_selected = (cache.browser_selected
                                        + PAGE_SCROLL_LINES as usize)
                                        .min(max);
                                }
                            }
                            BrowserDepth::View => {
                                cache.browser_scroll_offset = cache
                                    .browser_scroll_offset
                                    .saturating_add(PAGE_SCROLL_LINES);
                            }
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
                    DetailSubView::Browse => {
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        use crate::browser::BrowserDepth;
                        match cache.browser_depth {
                            BrowserDepth::List => {
                                cache.browser_selected = cache
                                    .browser_selected
                                    .saturating_sub(PAGE_SCROLL_LINES as usize);
                            }
                            BrowserDepth::View => {
                                cache.browser_scroll_offset = cache
                                    .browser_scroll_offset
                                    .saturating_sub(PAGE_SCROLL_LINES);
                            }
                        }
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
            KeyCode::Char('0') => switch_to_tab(&self.alias, 9, &mut self.scroll_offset, ctx),
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
                        let project_path = ctx
                            .config
                            .projects
                            .get(&self.alias)
                            .map(|p| p.path.clone());
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        let target = cache.defaults_edit_target;
                        if let Some(editing_idx) = cache.defaults_editing {
                            // Dropdown is open — Enter applies the selected option.
                            let entries = entries_for_cache(cache);
                            if let Some(entry) = entries.get(editing_idx).cloned() {
                                let options = dropdown_options(&entry.kind);
                                let dropdown_idx = cache.defaults_dropdown_selected.min(options.len().saturating_sub(1));
                                if let Some(value) = options.get(dropdown_idx).cloned() {
                                    if let Some(active) = active_config_mut(cache) {
                                        if set_config_value(active, entry.key, &value) {
                                            persist_active_config(target, project_path.as_deref(), active, &mut ctx.status_message);
                                        }
                                    }
                                }
                            }
                            cache.defaults_editing = None;
                            cache.defaults_dropdown_selected = 0;
                        } else {
                            let entries = entries_for_cache(cache);
                            let selected = cache.defaults_selected;
                            if let Some(entry) = entries.get(selected).cloned() {
                                let options = dropdown_options(&entry.kind);
                                if !options.is_empty() {
                                    let current_idx = options.iter().position(|o| o == &entry.value).unwrap_or(0);
                                    cache.defaults_editing = Some(selected);
                                    cache.defaults_dropdown_selected = current_idx;
                                } else if matches!(entry.kind, ConfigValueKind::String) {
                                    cache.defaults_editing = Some(selected);
                                    cache.defaults_text_buffer =
                                        if entry.value == "(unset)" {
                                            String::new()
                                        } else {
                                            entry.value.clone()
                                        };
                                } else if matches!(entry.kind, ConfigValueKind::Integer) {
                                    if let Some(active) = active_config_mut(cache) {
                                        if mutate_config_entry(active, entry.key, &entry.kind) {
                                            persist_active_config(target, project_path.as_deref(), active, &mut ctx.status_message);
                                        }
                                    }
                                }
                            }
                        }
                        ctx.needs_redraw = true;
                        ScreenAction::None
                    }
                    DetailSubView::Browse => {
                        // Space does nothing; only Enter descends/opens
                        if code == KeyCode::Char(' ') {
                            return ScreenAction::None;
                        }
                        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                        if cache.browser_depth != crate::browser::BrowserDepth::List {
                            return ScreenAction::None;
                        }
                        let entry = cache
                            .browser_entries
                            .get(cache.browser_selected)
                            .cloned();
                        if let Some(entry) = entry {
                            if entry.is_dir {
                                cache.browser_entries = crate::browser::list_dir(&entry.path);
                                cache.browser_current_dir = Some(entry.path);
                                cache.browser_selected = 0;
                                cache.browser_scroll_offset = 0;
                            } else {
                                cache.browser_file_content =
                                    Some(crate::browser::read_md_file(&entry.path));
                                cache.browser_file_name = Some(entry.name.clone());
                                cache.browser_depth = crate::browser::BrowserDepth::View;
                                cache.browser_scroll_offset = 0;
                            }
                            ctx.needs_redraw = true;
                        }
                        ScreenAction::None
                    }
                    _ => ScreenAction::None,
                }
            }
            // Docs browser: 'g' jumps to .planning/ root
            KeyCode::Char('g') if current_view == DetailSubView::Browse => {
                let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                if let Some(root) = cache.browser_root.clone() {
                    cache.browser_entries = crate::browser::list_dir(&root);
                    cache.browser_current_dir = Some(root);
                    cache.browser_selected = 0;
                    cache.browser_scroll_offset = 0;
                    cache.browser_depth = crate::browser::BrowserDepth::List;
                    cache.browser_file_content = None;
                    cache.browser_file_name = None;
                    ctx.needs_redraw = true;
                }
                ScreenAction::None
            }
            // Docs browser: 'p' jumps back to the entry phase directory
            KeyCode::Char('p') if current_view == DetailSubView::Browse => {
                let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                if let Some(entry_dir) = cache.browser_entry_dir.clone() {
                    cache.browser_entries = crate::browser::list_dir(&entry_dir);
                    cache.browser_current_dir = Some(entry_dir);
                    cache.browser_selected = 0;
                    cache.browser_scroll_offset = 0;
                    cache.browser_depth = crate::browser::BrowserDepth::List;
                    cache.browser_file_content = None;
                    cache.browser_file_name = None;
                    ctx.needs_redraw = true;
                }
                ScreenAction::None
            }
            // 'x' key: clear (unset) the value of the selected config row
            KeyCode::Char('x') if current_view == DetailSubView::Defaults => {
                let project_path = ctx
                    .config
                    .projects
                    .get(&self.alias)
                    .map(|p| p.path.clone());
                let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                let target = cache.defaults_edit_target;
                if cache.defaults_editing.is_none() {
                    let entries = entries_for_cache(cache);
                    let selected = cache.defaults_selected;
                    if let Some(entry) = entries.get(selected).cloned() {
                        let key = entry.key;
                        if let Some(active) = active_config_mut(cache) {
                            let cleared = clear_config_value(active, key);
                            if cleared {
                                persist_active_config(
                                    target,
                                    project_path.as_deref(),
                                    active,
                                    &mut ctx.status_message,
                                );
                                ctx.status_message = Some((
                                    format!("Cleared {}", key),
                                    std::time::Instant::now(),
                                ));
                            } else {
                                ctx.status_message = Some((
                                    format!("{} cannot be cleared", key),
                                    std::time::Instant::now(),
                                ));
                            }
                        }
                    }
                }
                ctx.needs_redraw = true;
                ScreenAction::None
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
                cache.defaults_user_config =
                    crate::state_reader::config_json::load_user_defaults();
                ctx.status_message = Some(("Config reloaded".to_string(), std::time::Instant::now()));
                ctx.needs_redraw = true;
                ScreenAction::None
            }
            // Tab: switch the host terminal to a Claude session for this project.
            // From the Sessions tab, jump to the highlighted session.
            // From any other tab, jump to the first session matching the project's path.
            KeyCode::Tab => {
                let project_path = ctx
                    .config
                    .projects
                    .get(&self.alias)
                    .map(|p| p.path.clone());
                let session = if current_view == DetailSubView::Sessions {
                    let filtered: Vec<_> = project_path
                        .as_ref()
                        .map(|path| {
                            ctx.active_sessions
                                .iter()
                                .filter(|s| s.working_dir == *path)
                                .cloned()
                                .collect()
                        })
                        .unwrap_or_default();
                    let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                    filtered.get(cache.sessions_selected).cloned()
                } else {
                    project_path.and_then(|path| {
                        ctx.active_sessions
                            .iter()
                            .find(|s| s.working_dir == path)
                            .cloned()
                    })
                };
                let msg = match session {
                    Some(s) => match crate::terminal_switch::switch_to_session(&s) {
                        Ok(()) => format!("Switched to {}", self.alias),
                        Err(e) => e,
                    },
                    None => format!("No active Claude session for {}", self.alias),
                };
                ctx.status_message = Some((msg, std::time::Instant::now()));
                ctx.needs_redraw = true;
                ScreenAction::None
            }
            // 'd' key: toggle between editing project config and ~/.gsd/defaults.json
            KeyCode::Char('d') if current_view == DetailSubView::Defaults => {
                use super::DefaultsEditTarget;
                let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                if cache.defaults_editing.is_some() {
                    return ScreenAction::None;
                }
                cache.defaults_edit_target = match cache.defaults_edit_target {
                    DefaultsEditTarget::Project => DefaultsEditTarget::Global,
                    DefaultsEditTarget::Global => DefaultsEditTarget::Project,
                };
                // Bootstrap an empty global defaults so toggling into the
                // Global view always has something editable, even before
                // ~/.gsd/defaults.json exists on disk.
                if cache.defaults_edit_target == DefaultsEditTarget::Global
                    && cache.defaults_user_config.is_none()
                {
                    cache.defaults_user_config =
                        Some(crate::state_reader::config_json::GsdConfig::default());
                }
                cache.defaults_selected = 0;
                let label = match cache.defaults_edit_target {
                    DefaultsEditTarget::Project => "Editing project config",
                    DefaultsEditTarget::Global => "Editing ~/.gsd/defaults.json",
                };
                ctx.status_message = Some((label.to_string(), std::time::Instant::now()));
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
            DetailSubView::Browse => self.render_browser_tab(frame, content_area, ctx),
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
    /// Handle a keystroke while a text-input editor is open on the Defaults
    /// tab. Char/Backspace edit the buffer, Enter persists, Esc cancels.
    fn handle_text_input_key(
        &self,
        code: KeyCode,
        ctx: &mut AppContext,
        editing_idx: usize,
    ) -> ScreenAction {
        match code {
            KeyCode::Char(c) => {
                let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                cache.defaults_text_buffer.push(c);
                ctx.needs_redraw = true;
            }
            KeyCode::Backspace => {
                let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                cache.defaults_text_buffer.pop();
                ctx.needs_redraw = true;
            }
            KeyCode::Esc => {
                let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                cache.defaults_editing = None;
                cache.defaults_text_buffer.clear();
                ctx.needs_redraw = true;
            }
            KeyCode::Enter => {
                let project_path = ctx
                    .config
                    .projects
                    .get(&self.alias)
                    .map(|p| p.path.clone());
                let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
                let target = cache.defaults_edit_target;
                let buffer = std::mem::take(&mut cache.defaults_text_buffer);
                cache.defaults_editing = None;
                let entries = entries_for_cache(cache);
                if let Some(entry) = entries.get(editing_idx).cloned() {
                    let key = entry.key;
                    if let Some(active) = active_config_mut(cache) {
                        let applied = if buffer.is_empty() {
                            clear_config_value(active, key)
                        } else {
                            set_string_value(active, key, &buffer)
                        };
                        if applied {
                            persist_active_config(
                                target,
                                project_path.as_deref(),
                                active,
                                &mut ctx.status_message,
                            );
                        }
                    }
                }
                ctx.needs_redraw = true;
            }
            _ => {}
        }
        ScreenAction::None
    }

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
                let substage_lines = build_substage_lines(inf);

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
                if !substage_lines.is_empty() {
                    lines.push(Line::from(""));
                    for sl in substage_lines {
                        lines.push(sl);
                    }
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

    /// Render the docs browser tab: a drill-down list of `.planning/` entries
    /// (dirs first, then `.md` files) with a rendered markdown preview when a
    /// file is open.
    fn render_browser_tab(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        use crate::browser::BrowserDepth;

        let cache = match ctx.view_cache.get(&self.alias) {
            Some(c) => c,
            None => {
                let p = Paragraph::new("Loading...")
                    .style(Style::default().fg(Color::DarkGray));
                frame.render_widget(p, area);
                return;
            }
        };

        // Header: Docs > <relative path from .planning/ root>
        let root = cache.browser_root.as_deref();
        let current = cache.browser_current_dir.as_deref();
        let rel_path = match (root, current) {
            (Some(r), Some(c)) => c
                .strip_prefix(r)
                .ok()
                .map(|p| p.to_string_lossy().to_string())
                .unwrap_or_else(|| c.to_string_lossy().to_string()),
            _ => String::new(),
        };
        let mut header_spans = vec![
            Span::styled("Docs", Style::default().fg(Color::Cyan)),
            Span::raw(" > "),
            Span::styled(".planning", Style::default().fg(Color::Yellow)),
        ];
        if !rel_path.is_empty() {
            header_spans.push(Span::raw("/"));
            header_spans.push(Span::styled(
                rel_path,
                Style::default().fg(Color::Yellow),
            ));
        }
        if cache.browser_depth == BrowserDepth::View {
            if let Some(name) = &cache.browser_file_name {
                header_spans.push(Span::raw(" / "));
                header_spans.push(Span::styled(
                    name.clone(),
                    Style::default()
                        .fg(Color::Green)
                        .add_modifier(Modifier::BOLD),
                ));
            }
        }

        let chunks = Layout::vertical([
            Constraint::Length(1),
            Constraint::Min(0),
        ])
        .split(area);
        let header_area = chunks[0];
        let content_area = chunks[1];

        let header = Paragraph::new(Line::from(header_spans));
        frame.render_widget(header, header_area);

        match cache.browser_depth {
            BrowserDepth::List => {
                if cache.browser_entries.is_empty() {
                    let msg = Paragraph::new("(empty directory)")
                        .style(Style::default().fg(Color::DarkGray));
                    frame.render_widget(msg, content_area);
                    return;
                }
                let items: Vec<ListItem> = cache
                    .browser_entries
                    .iter()
                    .map(|e| {
                        if e.is_dir {
                            ListItem::new(Line::from(vec![
                                Span::styled(
                                    "[DIR] ",
                                    Style::default()
                                        .fg(Color::Blue)
                                        .add_modifier(Modifier::BOLD),
                                ),
                                Span::raw(e.name.clone()),
                            ]))
                        } else {
                            ListItem::new(Line::from(Span::raw(e.name.clone())))
                        }
                    })
                    .collect();
                let list = List::new(items)
                    .block(Block::default().borders(Borders::NONE))
                    .highlight_style(
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    )
                    .highlight_symbol("> ");
                let mut list_state = ListState::default();
                list_state.select(Some(cache.browser_selected));
                frame.render_stateful_widget(list, content_area, &mut list_state);
            }
            BrowserDepth::View => {
                if let Some(content) = &cache.browser_file_content {
                    let styled_lines = crate::archive::render_markdown_lines(content);
                    let total_lines = styled_lines.len() as u16;
                    let gutter_width =
                        (total_lines as usize).max(1).to_string().len() as u16 + 1;
                    let file_chunks = Layout::horizontal([
                        Constraint::Length(gutter_width),
                        Constraint::Min(0),
                    ])
                    .split(content_area);
                    let gutter_area = file_chunks[0];
                    let text_area = file_chunks[1];

                    let visible_height = text_area.height;
                    let max_scroll = total_lines.saturating_sub(visible_height);
                    let scroll = cache.browser_scroll_offset.min(max_scroll);

                    let gutter_lines = crate::archive::line_number_lines(
                        total_lines as usize,
                        scroll,
                        visible_height,
                    );
                    let gutter = Paragraph::new(gutter_lines);
                    frame.render_widget(gutter, gutter_area);

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
            DetailSubView::Browse => self.render_browser_tab(frame, content_area, ctx),
        }
    }

    fn render_defaults_tab(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        use super::DefaultsEditTarget;
        let cache = ctx.view_cache.get(&self.alias);
        let selected = cache.map(|c| c.defaults_selected).unwrap_or(0);
        let edit_target = cache
            .map(|c| c.defaults_edit_target)
            .unwrap_or_default();

        let entries = match cache {
            Some(c) => entries_for_cache(c),
            None => Vec::new(),
        };

        if entries.is_empty() {
            let msg_text = match edit_target {
                DefaultsEditTarget::Project => {
                    "  No config loaded (project may not have .planning/config.json)"
                }
                DefaultsEditTarget::Global => {
                    "  No global defaults loaded (~/.gsd/defaults.json missing — saving will create it)"
                }
            };
            let msg =
                Paragraph::new(msg_text).style(Style::default().fg(Color::DarkGray));
            frame.render_widget(msg, area);
            return;
        }

        let items: Vec<ListItem> = entries
            .iter()
            .enumerate()
            .map(|(i, entry)| {
                let cat_span = if entry.show_category {
                    Span::styled(
                        format!("{:<18}", entry.category),
                        Style::default().fg(Color::DarkGray),
                    )
                } else {
                    Span::raw("                  ")
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
                let val_span = Span::styled(entry.value.clone(), val_style);
                let mut spans = vec![
                    Span::raw("  "),
                    cat_span,
                    Span::raw(" "),
                    key_span,
                    val_span,
                ];
                if entry.from_defaults {
                    spans.push(Span::styled(
                        " *",
                        Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD),
                    ));
                }
                let line = Line::from(spans);
                let item = ListItem::new(line);
                if i == selected {
                    item.style(Style::default().bg(Color::DarkGray).fg(Color::Cyan))
                } else {
                    item
                }
            })
            .collect();

        let title = match edit_target {
            DefaultsEditTarget::Project => " Config Settings ".to_string(),
            DefaultsEditTarget::Global => " Global Defaults (~/.gsd/defaults.json) ".to_string(),
        };
        let list = List::new(items).block(
            Block::default()
                .borders(Borders::TOP)
                .title(title),
        );

        let mut list_state = ListState::default();
        list_state.select(Some(selected));
        frame.render_stateful_widget(list, area, &mut list_state);

        // Render dropdown OR text-input overlay if editing
        if let Some(cache) = cache {
            if let Some(editing_idx) = cache.defaults_editing {
                if let Some(entry) = entries.get(editing_idx) {
                    if matches!(entry.kind, ConfigValueKind::String) {
                        let title = format!(" {} (Enter to save, Esc to cancel) ", entry.key);
                        let buffer = &cache.defaults_text_buffer;
                        let inner_w = buffer.len().max(title.len()).max(30) as u16;
                        let popup_w = (inner_w + 4).min(area.width.saturating_sub(2));
                        let popup_h = 3u16;
                        let popup_x = area.x + (area.width.saturating_sub(popup_w)) / 2;
                        let popup_y = area.y + (area.height.saturating_sub(popup_h)) / 2;
                        let popup_area = Rect {
                            x: popup_x,
                            y: popup_y,
                            width: popup_w,
                            height: popup_h,
                        };
                        frame.render_widget(ratatui::widgets::Clear, popup_area);
                        let text = Paragraph::new(Line::from(vec![
                            Span::styled(buffer.clone(), Style::default().fg(Color::White)),
                            Span::styled("█", Style::default().fg(Color::Cyan)),
                        ]))
                        .block(
                            Block::default()
                                .borders(Borders::ALL)
                                .border_style(Style::default().fg(Color::Cyan))
                                .title(title),
                        );
                        frame.render_widget(text, popup_area);
                        return;
                    }
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

/// Build sub-stage status lines for the Plan and Execute parent stages.
/// Each sub-stage is detected by the presence of a specific artifact.
/// We only render a parent group's lines once that parent has any artifact
/// on disk — otherwise the section stays collapsed to avoid noise on
/// not-yet-touched phases.
fn build_substage_lines(inf: &DiskInference) -> Vec<Line<'static>> {
    let mut lines: Vec<Line> = Vec::new();

    let plan_touched = inf.has_plans
        || inf.has_patterns
        || inf.has_plan_check
        || inf.has_validation
        || inf.has_ui_spec
        || inf.has_ui_check
        || inf.has_ai_spec
        || inf.has_security;
    if plan_touched {
        lines.push(Line::from(Span::styled(
            "  Plan sub-stages:",
            Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD),
        )));
        push_substage(&mut lines, "Security", inf.has_security);
        push_substage(&mut lines, "Patterns", inf.has_patterns);
        push_substage(&mut lines, "UI-Spec", inf.has_ui_spec);
        push_substage(&mut lines, "AI-Spec", inf.has_ai_spec);
        push_substage(&mut lines, "Plan-Check", inf.has_plan_check);
        push_substage(&mut lines, "UI-Check", inf.has_ui_check);
        push_substage(&mut lines, "Nyquist", inf.has_validation);
    }

    let exec_touched = inf.summary_count > 0 || inf.has_review || inf.has_ui_review;
    if exec_touched {
        if plan_touched {
            lines.push(Line::from(""));
        }
        lines.push(Line::from(Span::styled(
            "  Execute sub-stages:",
            Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD),
        )));
        push_substage(&mut lines, "Code Review", inf.has_review);
        push_substage(&mut lines, "UI Review", inf.has_ui_review);
    }

    lines
}

fn push_substage(lines: &mut Vec<Line<'static>>, label: &'static str, present: bool) {
    let (marker, color) = if present {
        ("✓", Color::Green)
    } else {
        ("○", Color::DarkGray)
    };
    lines.push(Line::from(vec![
        Span::raw("    "),
        Span::styled(marker.to_string(), Style::default().fg(color)),
        Span::raw(" "),
        Span::styled(format!("{:<14}", label), Style::default().fg(Color::White)),
        Span::styled(
            if present { "done" } else { "not run" },
            Style::default().fg(color),
        ),
    ]));
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
            spans.push(Span::styled("[Tab]", b));
            spans.push(Span::raw("switch  "));
            spans.push(Span::styled("[n]", b));
            spans.push(Span::raw("ew session  "));
        }
        DetailSubView::Archive => {
            spans.push(Span::styled("[Enter]", b));
            spans.push(Span::raw("open  "));
            spans.push(Span::styled("[e]", b));
            spans.push(Span::raw("dit  "));
        }
        DetailSubView::Browse => {
            spans.push(Span::styled("[Enter]", b));
            spans.push(Span::raw("open  "));
            spans.push(Span::styled("[Esc]", b));
            spans.push(Span::raw("up  "));
            spans.push(Span::styled("[g]", b));
            spans.push(Span::raw("root  "));
            spans.push(Span::styled("[p]", b));
            spans.push(Span::raw("hase  "));
        }
        DetailSubView::Defaults => {
            spans.push(Span::styled("[Enter]", b));
            spans.push(Span::raw("edit  "));
            spans.push(Span::styled("[x]", b));
            spans.push(Span::raw(" clear  "));
            spans.push(Span::styled("[d]", b));
            spans.push(Span::raw(" defaults  "));
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

#[derive(Clone)]
struct ConfigEntry {
    category: &'static str,
    key: &'static str,
    value: String,
    kind: ConfigValueKind,
    show_category: bool,
    /// True when the value was inherited from ~/.gsd/defaults.json
    /// because the project config had it unset.
    from_defaults: bool,
}

fn opt_bool_layered(project: Option<bool>, defaults: Option<bool>) -> (String, ConfigValueKind, bool) {
    if let Some(v) = project {
        (v.to_string(), ConfigValueKind::Bool, false)
    } else if let Some(v) = defaults {
        (v.to_string(), ConfigValueKind::Bool, true)
    } else {
        ("(unset)".to_string(), ConfigValueKind::Null, false)
    }
}

fn opt_str_layered(project: Option<&str>, defaults: Option<&str>) -> (String, ConfigValueKind, bool) {
    if let Some(s) = project {
        (s.to_string(), ConfigValueKind::String, false)
    } else if let Some(s) = defaults {
        (s.to_string(), ConfigValueKind::String, true)
    } else {
        ("(unset)".to_string(), ConfigValueKind::Null, false)
    }
}

fn opt_u32_layered(project: Option<u32>, defaults: Option<u32>) -> (String, ConfigValueKind, bool) {
    if let Some(n) = project {
        (n.to_string(), ConfigValueKind::Integer, false)
    } else if let Some(n) = defaults {
        (n.to_string(), ConfigValueKind::Integer, true)
    } else {
        ("(unset)".to_string(), ConfigValueKind::Null, false)
    }
}

fn opt_enum_layered(
    project: Option<&str>,
    defaults: Option<&str>,
    options: &'static [&'static str],
) -> (String, ConfigValueKind, bool) {
    if let Some(s) = project {
        (s.to_string(), ConfigValueKind::Enum(options), false)
    } else if let Some(s) = defaults {
        (s.to_string(), ConfigValueKind::Enum(options), true)
    } else {
        ("(unset)".to_string(), ConfigValueKind::Null, false)
    }
}

fn build_defaults_entries(
    config: &crate::state_reader::config_json::GsdConfig,
    defaults: Option<&crate::state_reader::config_json::GsdConfig>,
) -> Vec<ConfigEntry> {
    use crate::state_reader::config_json::GsdConfig;

    let mut entries = Vec::new();
    let mut push = |cat: &'static str,
                    key: &'static str,
                    value: String,
                    kind: ConfigValueKind,
                    first: bool,
                    from_defaults: bool| {
        entries.push(ConfigEntry {
            category: cat,
            key,
            value,
            kind,
            show_category: first,
            from_defaults,
        });
    };

    // Layered accessors — fall back to ~/.gsd/defaults.json when the
    // project's Option is None.
    let bool_l = |proj: Option<bool>, def: Option<bool>| opt_bool_layered(proj, def);
    let u32_l = |proj: Option<u32>, def: Option<u32>| opt_u32_layered(proj, def);
    let str_l = |proj: Option<&str>, def: Option<&str>| opt_str_layered(proj, def);
    let enum_l = |proj: Option<&str>,
                  def: Option<&str>,
                  options: &'static [&'static str]| { opt_enum_layered(proj, def, options) };

    // ── Planning ───────────────────────────────────────────────
    let cat = "Planning";
    let pwf = config.workflow.as_ref();
    let dwf = defaults.and_then(|d: &GsdConfig| d.workflow.as_ref());
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.research), dwf.and_then(|w| w.research));
    push(cat, "research", v, k, true, fd);
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.plan_check), dwf.and_then(|w| w.plan_check));
    push(cat, "plan_check", v, k, false, fd);
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.pattern_mapper), dwf.and_then(|w| w.pattern_mapper));
    push(cat, "pattern_mapper", v, k, false, fd);
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.nyquist_validation), dwf.and_then(|w| w.nyquist_validation));
    push(cat, "nyquist_validation", v, k, false, fd);
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.ui_phase), dwf.and_then(|w| w.ui_phase));
    push(cat, "ui_phase", v, k, false, fd);
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.ui_safety_gate), dwf.and_then(|w| w.ui_safety_gate));
    push(cat, "ui_safety_gate", v, k, false, fd);
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.ai_integration_phase), dwf.and_then(|w| w.ai_integration_phase));
    push(cat, "ai_integration_phase", v, k, false, fd);
    let (v, k, fd) = u32_l(pwf.and_then(|w| w.subagent_timeout), dwf.and_then(|w| w.subagent_timeout));
    push(cat, "subagent_timeout", v, k, false, fd);

    // ── Execution ──────────────────────────────────────────────
    let cat = "Execution";
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.verifier), dwf.and_then(|w| w.verifier));
    push(cat, "verifier", v, k, true, fd);
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.tdd_mode), dwf.and_then(|w| w.tdd_mode));
    push(cat, "tdd_mode", v, k, false, fd);
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.code_review), dwf.and_then(|w| w.code_review));
    push(cat, "code_review", v, k, false, fd);
    let (v, k, fd) = enum_l(
        pwf.and_then(|w| w.code_review_depth.as_deref()),
        dwf.and_then(|w| w.code_review_depth.as_deref()),
        &["quick", "standard", "deep"],
    );
    push(cat, "code_review_depth", v, k, false, fd);
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.ui_review), dwf.and_then(|w| w.ui_review));
    push(cat, "ui_review", v, k, false, fd);
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.node_repair), dwf.and_then(|w| w.node_repair));
    push(cat, "node_repair", v, k, false, fd);
    let (v, k, fd) = u32_l(pwf.and_then(|w| w.node_repair_budget), dwf.and_then(|w| w.node_repair_budget));
    push(cat, "node_repair_budget", v, k, false, fd);

    // ── Docs & Output ─────────────────────────────────────────
    let cat = "Docs & Output";
    let (v, k, fd) = bool_l(config.commit_docs, defaults.and_then(|d| d.commit_docs));
    push(cat, "commit_docs", v, k, true, fd);
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.skip_discuss), dwf.and_then(|w| w.skip_discuss));
    push(cat, "skip_discuss", v, k, false, fd);
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.use_worktrees), dwf.and_then(|w| w.use_worktrees));
    push(cat, "use_worktrees", v, k, false, fd);
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.text_mode), dwf.and_then(|w| w.text_mode));
    push(cat, "text_mode", v, k, false, fd);
    let (v, k, fd) = str_l(
        config.response_language.as_deref(),
        defaults.and_then(|d| d.response_language.as_deref()),
    );
    push(cat, "response_language", v, k, false, fd);

    // ── Features ──────────────────────────────────────────────
    let cat = "Features";
    let (v, k, fd) = bool_l(
        config.intel.as_ref().and_then(|i| i.enabled),
        defaults.and_then(|d| d.intel.as_ref().and_then(|i| i.enabled)),
    );
    push(cat, "intel_enabled", v, k, true, fd);
    let (v, k, fd) = bool_l(
        config.graphify.as_ref().and_then(|g| g.enabled),
        defaults.and_then(|d| d.graphify.as_ref().and_then(|g| g.enabled)),
    );
    push(cat, "graphify_enabled", v, k, false, fd);
    let (v, k, fd) = u32_l(
        config.graphify.as_ref().and_then(|g| g.build_timeout),
        defaults.and_then(|d| d.graphify.as_ref().and_then(|g| g.build_timeout)),
    );
    push(cat, "graphify_build_timeout", v, k, false, fd);
    let (v, k, fd) = bool_l(config.brave_search, defaults.and_then(|d| d.brave_search));
    push(cat, "brave_search", v, k, false, fd);
    let (v, k, fd) = bool_l(config.firecrawl, defaults.and_then(|d| d.firecrawl));
    push(cat, "firecrawl", v, k, false, fd);
    let (v, k, fd) = bool_l(config.exa_search, defaults.and_then(|d| d.exa_search));
    push(cat, "exa_search", v, k, false, fd);

    // ── Model & Pipeline ──────────────────────────────────────
    let cat = "Model & Pipeline";
    push(cat, "mode", config.mode.clone(), ConfigValueKind::Enum(&["interactive", "yolo"]), true, false);
    push(cat, "granularity", config.granularity.clone(), ConfigValueKind::Enum(&["coarse", "standard", "fine"]), false, false);
    push(cat, "model_profile", config.model_profile.clone(), ConfigValueKind::Enum(&["quality", "balanced", "budget", "adaptive", "inherit"]), false, false);
    let (v, k, fd) = bool_l(config.parallelization, defaults.and_then(|d| d.parallelization));
    push(cat, "parallelization", v, k, false, fd);
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.auto_advance), dwf.and_then(|w| w.auto_advance));
    push(cat, "auto_advance", v, k, false, fd);
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.auto_chain_active), dwf.and_then(|w| w.auto_chain_active));
    push(cat, "auto_chain_active", v, k, false, fd);
    let pgit = config.git.as_ref();
    let dgit = defaults.and_then(|d| d.git.as_ref());
    let (v, k, fd) = enum_l(
        pgit.and_then(|g| g.branching_strategy.as_deref()),
        dgit.and_then(|g| g.branching_strategy.as_deref()),
        &["none", "phase", "milestone"],
    );
    push(cat, "branching_strategy", v, k, false, fd);
    let (v, k, fd) = str_l(
        pgit.and_then(|g| g.base_branch.as_deref()),
        dgit.and_then(|g| g.base_branch.as_deref()),
    );
    push(cat, "base_branch", v, k, false, fd);
    let (v, k, fd) = str_l(
        pgit.and_then(|g| g.phase_branch_template.as_deref()),
        dgit.and_then(|g| g.phase_branch_template.as_deref()),
    );
    push(cat, "phase_branch_template", v, k, false, fd);
    let (v, k, fd) = str_l(
        pgit.and_then(|g| g.milestone_branch_template.as_deref()),
        dgit.and_then(|g| g.milestone_branch_template.as_deref()),
    );
    push(cat, "milestone_branch_template", v, k, false, fd);
    let qbt_proj = pgit.and_then(|g| g.quick_branch_template.as_ref()).map(|v| v.to_string());
    let qbt_def = dgit.and_then(|g| g.quick_branch_template.as_ref()).map(|v| v.to_string());
    let (qbt_val, qbt_kind, qbt_fd) = match (qbt_proj, qbt_def) {
        (Some(s), _) => (s, ConfigValueKind::String, false),
        (None, Some(s)) => (s, ConfigValueKind::String, true),
        (None, None) => ("(unset)".to_string(), ConfigValueKind::Null, false),
    };
    push(cat, "quick_branch_template", qbt_val, qbt_kind, false, qbt_fd);

    // ── Misc ──────────────────────────────────────────────────
    let cat = "Misc";
    let (v, k, fd) = bool_l(
        config.hooks.as_ref().and_then(|h| h.context_warnings),
        defaults.and_then(|d| d.hooks.as_ref().and_then(|h| h.context_warnings)),
    );
    push(cat, "context_warnings", v, k, true, fd);
    let (v, k, fd) = bool_l(pwf.and_then(|w| w.research_before_questions), dwf.and_then(|w| w.research_before_questions));
    push(cat, "research_before_questions", v, k, false, fd);
    let (v, k, fd) = enum_l(
        pwf.and_then(|w| w.discuss_mode.as_deref()),
        dwf.and_then(|w| w.discuss_mode.as_deref()),
        &["discuss", "assumptions"],
    );
    push(cat, "discuss_mode", v, k, false, fd);
    let (v, k, fd) = bool_l(config.search_gitignored, defaults.and_then(|d| d.search_gitignored));
    push(cat, "search_gitignored", v, k, false, fd);
    let (v, k, fd) = str_l(config.project_code.as_deref(), defaults.and_then(|d| d.project_code.as_deref()));
    push(cat, "project_code", v, k, false, fd);
    let (v, k, fd) = str_l(config.phase_naming.as_deref(), defaults.and_then(|d| d.phase_naming.as_deref()));
    push(cat, "phase_naming", v, k, false, fd);

    entries
}

/// Build defaults entries respecting the cache's edit target.
/// When target=Project: project config is primary, defaults_user_config
/// fills in unset Optional fields. When target=Global: defaults_user_config
/// is primary with no fallback.
fn entries_for_cache(cache: &super::ProjectViewCache) -> Vec<ConfigEntry> {
    use super::DefaultsEditTarget;
    let (primary, fallback) = match cache.defaults_edit_target {
        DefaultsEditTarget::Project => {
            (cache.defaults_config.as_ref(), cache.defaults_user_config.as_ref())
        }
        DefaultsEditTarget::Global => (cache.defaults_user_config.as_ref(), None),
    };
    primary
        .map(|p| build_defaults_entries(p, fallback))
        .unwrap_or_default()
}

fn entries_count_for_cache(cache: &super::ProjectViewCache) -> usize {
    entries_for_cache(cache).len()
}

/// Get a mutable reference to whichever config the user is currently
/// editing (project or ~/.gsd/defaults.json).
fn active_config_mut(
    cache: &mut super::ProjectViewCache,
) -> Option<&mut crate::state_reader::config_json::GsdConfig> {
    use super::DefaultsEditTarget;
    match cache.defaults_edit_target {
        DefaultsEditTarget::Project => cache.defaults_config.as_mut(),
        DefaultsEditTarget::Global => cache.defaults_user_config.as_mut(),
    }
}

/// Persist the active config to disk and surface a status message.
fn persist_active_config(
    target: super::DefaultsEditTarget,
    project_path: Option<&std::path::Path>,
    config: &crate::state_reader::config_json::GsdConfig,
    status_message: &mut Option<(String, std::time::Instant)>,
) {
    use super::DefaultsEditTarget;
    let path = match target {
        DefaultsEditTarget::Project => project_path.map(|p| p.join(".planning/config.json")),
        DefaultsEditTarget::Global => crate::state_reader::config_json::user_defaults_path(),
    };
    let Some(path) = path else { return };
    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    if let Ok(json) = crate::state_reader::config_json::serialize_gsd_config(config) {
        if std::fs::write(&path, &json).is_ok() {
            let label = match target {
                DefaultsEditTarget::Project => "Config saved",
                DefaultsEditTarget::Global => "Defaults saved",
            };
            *status_message = Some((label.to_string(), std::time::Instant::now()));
        }
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
            "intel_enabled" => { config.intel.get_or_insert_with(IntelConfig::default).enabled = Some(b); return true; }
            "graphify_enabled" => { config.graphify.get_or_insert_with(GraphifyConfig::default).enabled = Some(b); return true; }
            "pattern_mapper" => { config.workflow.get_or_insert_with(WorkflowConfig::default).pattern_mapper = Some(b); return true; }
            "ai_integration_phase" => { config.workflow.get_or_insert_with(WorkflowConfig::default).ai_integration_phase = Some(b); return true; }
            "tdd_mode" => { config.workflow.get_or_insert_with(WorkflowConfig::default).tdd_mode = Some(b); return true; }
            "code_review" => { config.workflow.get_or_insert_with(WorkflowConfig::default).code_review = Some(b); return true; }
            "ui_review" => { config.workflow.get_or_insert_with(WorkflowConfig::default).ui_review = Some(b); return true; }
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
        "code_review_depth" => {
            config.workflow.get_or_insert_with(WorkflowConfig::default).code_review_depth = Some(value.to_string());
            true
        }
        _ => false,
    }
}

/// Set a String-kind config field by key. Returns true if recognized.
/// `value` is assumed to be non-empty (callers route empty input through
/// `clear_config_value` instead).
fn set_string_value(
    config: &mut crate::state_reader::config_json::GsdConfig,
    key: &str,
    value: &str,
) -> bool {
    use crate::state_reader::config_json::*;

    match key {
        "project_code" => { config.project_code = Some(value.to_string()); true }
        "phase_naming" => { config.phase_naming = Some(value.to_string()); true }
        "response_language" => { config.response_language = Some(value.to_string()); true }
        "base_branch" => {
            config.git.get_or_insert_with(GitConfig::default).base_branch = Some(value.to_string());
            true
        }
        "phase_branch_template" => {
            config.git.get_or_insert_with(GitConfig::default).phase_branch_template = Some(value.to_string());
            true
        }
        "milestone_branch_template" => {
            config.git.get_or_insert_with(GitConfig::default).milestone_branch_template = Some(value.to_string());
            true
        }
        "quick_branch_template" => {
            config.git.get_or_insert_with(GitConfig::default).quick_branch_template =
                Some(serde_json::Value::String(value.to_string()));
            true
        }
        _ => false,
    }
}

/// Clear (set to None / unset) the config field identified by `key`.
/// Required scalar fields (`mode`, `granularity`, `model_profile`) are NOT
/// clearable — those rows ignore the clear shortcut.
fn clear_config_value(
    config: &mut crate::state_reader::config_json::GsdConfig,
    key: &str,
) -> bool {
    use crate::state_reader::config_json::*;

    match key {
        // Required (non-Option) fields — refuse to clear.
        "mode" | "granularity" | "model_profile" => false,

        // Top-level Options
        "commit_docs" => { config.commit_docs = None; true }
        "parallelization" => { config.parallelization = None; true }
        "search_gitignored" => { config.search_gitignored = None; true }
        "brave_search" => { config.brave_search = None; true }
        "firecrawl" => { config.firecrawl = None; true }
        "exa_search" => { config.exa_search = None; true }
        "project_code" => { config.project_code = None; true }
        "phase_naming" => { config.phase_naming = None; true }
        "response_language" => { config.response_language = None; true }

        // Git
        "branching_strategy" => { config.git.get_or_insert_with(GitConfig::default).branching_strategy = None; true }
        "base_branch" => { config.git.get_or_insert_with(GitConfig::default).base_branch = None; true }
        "phase_branch_template" => { config.git.get_or_insert_with(GitConfig::default).phase_branch_template = None; true }
        "milestone_branch_template" => { config.git.get_or_insert_with(GitConfig::default).milestone_branch_template = None; true }
        "quick_branch_template" => { config.git.get_or_insert_with(GitConfig::default).quick_branch_template = None; true }

        // Workflow
        "research" => { config.workflow.get_or_insert_with(WorkflowConfig::default).research = None; true }
        "plan_check" => { config.workflow.get_or_insert_with(WorkflowConfig::default).plan_check = None; true }
        "verifier" => { config.workflow.get_or_insert_with(WorkflowConfig::default).verifier = None; true }
        "nyquist_validation" => { config.workflow.get_or_insert_with(WorkflowConfig::default).nyquist_validation = None; true }
        "auto_advance" => { config.workflow.get_or_insert_with(WorkflowConfig::default).auto_advance = None; true }
        "node_repair" => { config.workflow.get_or_insert_with(WorkflowConfig::default).node_repair = None; true }
        "node_repair_budget" => { config.workflow.get_or_insert_with(WorkflowConfig::default).node_repair_budget = None; true }
        "ui_phase" => { config.workflow.get_or_insert_with(WorkflowConfig::default).ui_phase = None; true }
        "ui_safety_gate" => { config.workflow.get_or_insert_with(WorkflowConfig::default).ui_safety_gate = None; true }
        "text_mode" => { config.workflow.get_or_insert_with(WorkflowConfig::default).text_mode = None; true }
        "research_before_questions" => { config.workflow.get_or_insert_with(WorkflowConfig::default).research_before_questions = None; true }
        "discuss_mode" => { config.workflow.get_or_insert_with(WorkflowConfig::default).discuss_mode = None; true }
        "skip_discuss" => { config.workflow.get_or_insert_with(WorkflowConfig::default).skip_discuss = None; true }
        "auto_chain_active" => { config.workflow.get_or_insert_with(WorkflowConfig::default).auto_chain_active = None; true }
        "use_worktrees" => { config.workflow.get_or_insert_with(WorkflowConfig::default).use_worktrees = None; true }
        "subagent_timeout" => { config.workflow.get_or_insert_with(WorkflowConfig::default).subagent_timeout = None; true }

        // Hooks
        "context_warnings" => { config.hooks.get_or_insert_with(HooksConfig::default).context_warnings = None; true }

        // Intel / Graphify
        "intel_enabled" => { config.intel.get_or_insert_with(IntelConfig::default).enabled = None; true }
        "graphify_enabled" => { config.graphify.get_or_insert_with(GraphifyConfig::default).enabled = None; true }
        "graphify_build_timeout" => { config.graphify.get_or_insert_with(GraphifyConfig::default).build_timeout = None; true }

        // Workflow toggles added with the /gsd-settings six-section layout
        "pattern_mapper" => { config.workflow.get_or_insert_with(WorkflowConfig::default).pattern_mapper = None; true }
        "ai_integration_phase" => { config.workflow.get_or_insert_with(WorkflowConfig::default).ai_integration_phase = None; true }
        "tdd_mode" => { config.workflow.get_or_insert_with(WorkflowConfig::default).tdd_mode = None; true }
        "code_review" => { config.workflow.get_or_insert_with(WorkflowConfig::default).code_review = None; true }
        "code_review_depth" => { config.workflow.get_or_insert_with(WorkflowConfig::default).code_review_depth = None; true }
        "ui_review" => { config.workflow.get_or_insert_with(WorkflowConfig::default).ui_review = None; true }

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
                "intel_enabled" => { let i = config.intel.get_or_insert_with(IntelConfig::default); i.enabled = Some(!i.enabled.unwrap_or(false)); true }
                "graphify_enabled" => { let g = config.graphify.get_or_insert_with(GraphifyConfig::default); g.enabled = Some(!g.enabled.unwrap_or(false)); true }
                "pattern_mapper" => { let wf = config.workflow.get_or_insert_with(WorkflowConfig::default); wf.pattern_mapper = Some(!wf.pattern_mapper.unwrap_or(false)); true }
                "ai_integration_phase" => { let wf = config.workflow.get_or_insert_with(WorkflowConfig::default); wf.ai_integration_phase = Some(!wf.ai_integration_phase.unwrap_or(false)); true }
                "tdd_mode" => { let wf = config.workflow.get_or_insert_with(WorkflowConfig::default); wf.tdd_mode = Some(!wf.tdd_mode.unwrap_or(false)); true }
                "code_review" => { let wf = config.workflow.get_or_insert_with(WorkflowConfig::default); wf.code_review = Some(!wf.code_review.unwrap_or(false)); true }
                "ui_review" => { let wf = config.workflow.get_or_insert_with(WorkflowConfig::default); wf.ui_review = Some(!wf.ui_review.unwrap_or(false)); true }
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
                "graphify_build_timeout" => {
                    let g = config.graphify.get_or_insert_with(GraphifyConfig::default);
                    let current = g.build_timeout.unwrap_or(0);
                    g.build_timeout = Some(if current >= 600 { 60 } else { current + 30 });
                    true
                }
                _ => false,
            }
        }
    }
}
