pub mod add_project;
pub mod create_project;
pub mod delete_confirm;
pub mod detail;
pub mod enqueue;
pub mod help;
pub mod normal;
pub mod queue_delete_confirm;

use crate::action::Action;
use crate::change_tracker::ChangeTracker;
use crate::config::Config;
use crate::state_reader::ProjectState;
use crate::state_reader::backlog::BacklogItem;
use crate::state_reader::git_ops::{GitLogEntry, GitDiffStat};
use crate::watcher::FileWatcher;
use crate::app::DetailSubView;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::widgets::TableState;
use ratatui::Frame;
use ratatui::layout::Rect;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::mpsc::UnboundedSender;

pub trait Screen {
    fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers, ctx: &mut AppContext) -> ScreenAction;
    fn render(&self, frame: &mut Frame, area: Rect, ctx: &AppContext);
    fn name(&self) -> &str;
}

pub enum ScreenAction {
    None,
    Push(Box<dyn Screen>),
    Pop,
    Quit,
    SetStatusMessage(String),
    /// Used by archive browser (Phase 12) to dispatch async load actions.
    #[allow(dead_code)]
    DispatchAction(Action),
}

pub struct ProjectViewCache {
    pub backlog_items: Vec<BacklogItem>,
    pub backlog_selected: usize,
    pub backlog_expanded: bool,
    pub git_entries: Vec<GitLogEntry>,
    pub git_selected: usize,
    pub git_planning_only: bool,
    pub git_diff_stat: Option<GitDiffStat>,
    pub loading_backlog: bool,
    pub loading_git: bool,
    pub loading_diff: bool,
    pub pipeline_selected: usize,
    pub queue_selected: usize,
    pub sessions_selected: usize,
}

impl Default for ProjectViewCache {
    fn default() -> Self {
        Self {
            backlog_items: Vec::new(),
            backlog_selected: 0,
            backlog_expanded: false,
            git_entries: Vec::new(),
            git_selected: 0,
            git_planning_only: false,
            git_diff_stat: None,
            loading_backlog: false,
            loading_git: false,
            loading_diff: false,
            pipeline_selected: 0,
            queue_selected: 0,
            sessions_selected: 0,
        }
    }
}

pub struct AppContext {
    pub config: Config,
    pub config_path: PathBuf,
    pub project_states: HashMap<String, ProjectState>,
    pub table_state: TableState,
    pub filtered_aliases: Vec<String>,
    pub filter_text: String,
    pub change_tracker: ChangeTracker,
    pub detail_sub_view_per_project: HashMap<String, DetailSubView>,
    pub view_cache: HashMap<String, ProjectViewCache>,
    pub status_message: Option<(String, std::time::Instant)>,
    pub error_message: Option<String>,
    pub event_tx: Option<UnboundedSender<Action>>,
    pub watcher: Option<FileWatcher>,
    pub last_refresh: HashMap<String, std::time::Instant>,
    pub detail_scroll_offset: u16,
    pub suggestion_index: usize,
    pub input_buffer: String,
    pub needs_redraw: bool,
    pub active_sessions: Vec<crate::session_detector::ClaudeSession>,
}

impl AppContext {
    /// Get sorted project aliases for consistent ordering in the table.
    pub fn sorted_aliases(&self) -> Vec<String> {
        let mut aliases: Vec<String> = self.config.projects.keys().cloned().collect();
        aliases.sort_by(|a, b| a.to_lowercase().cmp(&b.to_lowercase()));
        aliases
    }

    /// Get the alias of the currently selected project, if any.
    pub fn selected_alias(&self) -> Option<String> {
        self.table_state
            .selected()
            .and_then(|i| self.filtered_aliases.get(i).cloned())
    }

    pub fn recompute_filtered_aliases(&mut self) {
        use crate::app::{parse_filter, format_phase_display, FilterColumn};

        let all = self.sorted_aliases();
        if self.filter_text.is_empty() {
            self.filtered_aliases = all;
            return;
        }
        let (term, column) = parse_filter(&self.filter_text);
        let term_lower = term.to_lowercase();
        self.filtered_aliases = all
            .into_iter()
            .filter(|alias| {
                let state = self.project_states.get(alias);
                match column {
                    FilterColumn::Name => alias.to_lowercase().contains(&term_lower),
                    FilterColumn::Phase => state.map_or(false, |s| {
                        format_phase_display(s).to_lowercase().contains(&term_lower)
                    }),
                    FilterColumn::Status => {
                        state.map_or(false, |s| s.status.to_lowercase().contains(&term_lower))
                    }
                    FilterColumn::All => {
                        alias.to_lowercase().contains(&term_lower)
                            || state.map_or(false, |s| {
                                s.status.to_lowercase().contains(&term_lower)
                                    || format_phase_display(s)
                                        .to_lowercase()
                                        .contains(&term_lower)
                            })
                    }
                }
            })
            .collect();
    }
}
