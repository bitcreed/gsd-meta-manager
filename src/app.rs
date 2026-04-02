use crate::action::Action;
use crate::change_tracker::ChangeTracker;
use crate::config::{load_config, save_config};
use crate::registry;
use crate::session_detector::ClaudeSession;
use crate::state_reader::{self, ProjectState};
use crate::ui::screens::normal::NormalScreen;
use crate::ui::screens::{AppContext, Screen, ScreenAction};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::widgets::TableState;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::mpsc::UnboundedSender;

#[derive(Debug, Clone, PartialEq, Default)]
pub enum DetailSubView {
    #[default]
    PhaseList,
    RoadmapViz,
    Backlog,
    GitHistory,
    Pipeline,
    Queue,
    Sessions,
    Archive,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FilterColumn {
    All,
    Name,
    Phase,
    Status,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StatusCategory {
    Active,
    Idle,
    Blocked,
    Complete,
    Unknown,
}

pub fn parse_filter(input: &str) -> (String, FilterColumn) {
    if let Some(term) = input.strip_suffix("/p") {
        (term.to_string(), FilterColumn::Phase)
    } else if let Some(term) = input.strip_suffix("/n") {
        (term.to_string(), FilterColumn::Name)
    } else if let Some(term) = input.strip_suffix("/s") {
        (term.to_string(), FilterColumn::Status)
    } else {
        (input.to_string(), FilterColumn::All)
    }
}

pub fn classify_status(status: &str) -> StatusCategory {
    let s = status.to_lowercase();
    if s.contains("executing") || s.contains("active") || s.contains("in progress") {
        StatusCategory::Active
    } else if s.contains("blocked") {
        StatusCategory::Blocked
    } else if s.contains("complete") || s.contains("done") {
        StatusCategory::Complete
    } else if s.contains("idle") || s.contains("ready") || s.contains("plan") {
        StatusCategory::Idle
    } else {
        StatusCategory::Unknown
    }
}

pub fn format_phase_display(state: &ProjectState) -> String {
    if state.completed_phases >= state.total_phases && state.total_phases > 0 {
        if !state.milestone.is_empty() {
            return format!("{} Complete", state.milestone);
        }
        return "Complete".to_string();
    }
    let phase_num = state.completed_phases + 1;
    let phase_name = state
        .phases
        .iter()
        .find(|p| p.number == phase_num.to_string())
        .map(|p| p.name.as_str())
        .unwrap_or("Unknown");
    format!("P{}: {}", phase_num, phase_name)
}

pub struct App {
    pub should_quit: bool,
    pub needs_redraw: bool,
    pub ctx: AppContext,
    pub screen_stack: Vec<Box<dyn Screen>>,
    pub active_sessions: Vec<ClaudeSession>,
    pub session_poll_counter: u32,
    /// When set, the main loop should suspend the TUI and open this file in $EDITOR.
    pub pending_editor: Option<PathBuf>,
}

impl App {
    pub fn new(config_path: PathBuf) -> anyhow::Result<Self> {
        let config = load_config(&config_path)?;
        let mut table_state = TableState::default();
        if !config.projects.is_empty() {
            table_state.select(Some(0));
        }

        let mut ctx = AppContext {
            config,
            config_path,
            project_states: HashMap::new(),
            table_state,
            filtered_aliases: Vec::new(),
            filter_text: String::new(),
            change_tracker: ChangeTracker::new(),
            detail_sub_view_per_project: HashMap::new(),
            view_cache: HashMap::new(),
            status_message: None,
            error_message: None,
            event_tx: None,
            watcher: None,
            last_refresh: HashMap::new(),
            detail_scroll_offset: 0,
            suggestion_index: 0,
            input_buffer: String::new(),
            needs_redraw: true,
            active_sessions: Vec::new(),
            archive_cache: HashMap::new(),
        };
        ctx.filtered_aliases = ctx.sorted_aliases();

        Ok(App {
            should_quit: false,
            needs_redraw: true,
            ctx,
            screen_stack: vec![Box::new(NormalScreen::new())],
            active_sessions: Vec::new(),
            session_poll_counter: 0,
            pending_editor: None,
        })
    }

    /// Load project states for all registered projects.
    /// Uses spawn_blocking for async-safe file I/O when event_tx is available,
    /// falls back to synchronous loading for initial startup.
    pub fn load_project_states(&mut self) {
        for (alias, project) in &self.ctx.config.projects {
            let planning_dir = project.path.join(".planning");
            let state = state_reader::parse_project_state(&planning_dir);
            self.ctx.project_states.insert(alias.clone(), state);
        }
        self.ctx.recompute_filtered_aliases();
    }

    /// Record initial snapshots for all loaded project states (for change detection).
    pub fn init_change_tracker(&mut self) {
        for (alias, state) in &self.ctx.project_states {
            self.ctx.change_tracker.record_initial(alias, state);
        }
    }

    pub fn update(&mut self, action: Action) {
        match action {
            Action::Tick => {
                // Expire status messages after 3 seconds
                if let Some((_, instant)) = &self.ctx.status_message {
                    if instant.elapsed() > std::time::Duration::from_secs(3) {
                        self.ctx.status_message = None;
                        self.needs_redraw = true;
                    }
                }

                // Poll for Claude sessions every 20 ticks (~5s at 250ms interval)
                self.session_poll_counter += 1;
                if self.session_poll_counter >= 20 {
                    self.session_poll_counter = 0;
                    if let Some(ref tx) = self.ctx.event_tx {
                        let tx: UnboundedSender<Action> = tx.clone();
                        tokio::task::spawn_blocking(move || {
                            let sessions = crate::session_detector::detect_sessions();
                            let _ = tx.send(Action::SessionsDetected { sessions });
                        });
                    }
                }
            }
            Action::RawKey(key_event) => {
                self.handle_key(key_event.code, key_event.modifiers);
            }
            Action::Resize => {
                self.needs_redraw = true;
            }
            Action::FileChanged { project_path } => {
                // Find the alias matching this project path
                let alias = self
                    .ctx
                    .config
                    .projects
                    .iter()
                    .find(|(_, proj)| proj.path == project_path)
                    .map(|(alias, _)| alias.clone());

                if let Some(alias) = alias {
                    // Dedup: skip if last refresh was less than 500ms ago
                    let now = std::time::Instant::now();
                    if let Some(last) = self.ctx.last_refresh.get(&alias) {
                        if now.duration_since(*last) < std::time::Duration::from_millis(500) {
                            return;
                        }
                    }

                    // Use spawn_blocking for async file I/O
                    if let Some(tx) = &self.ctx.event_tx {
                        let tx = tx.clone();
                        let alias_for_task = alias.clone();
                        let planning_dir = project_path.join(".planning");
                        tokio::task::spawn_blocking(move || {
                            let state = state_reader::parse_project_state(&planning_dir);
                            let _ = tx.send(Action::ProjectStateLoaded {
                                alias: alias_for_task,
                                state,
                            });
                        });
                        self.ctx.last_refresh.insert(alias, now);
                    }

                    // Auto-start watcher if not yet watching
                    let planning_dir_check = project_path.join(".planning");
                    if planning_dir_check.is_dir() {
                        if let Some(ref mut watcher) = self.ctx.watcher {
                            let _ = watcher.watch(&planning_dir_check);
                        }
                    }
                }
            }
            Action::ProjectStateLoaded { alias, state } => {
                // Detect changes before replacing the old state
                if let Some(old_state) = self.ctx.project_states.get(&alias) {
                    self.ctx
                        .change_tracker
                        .detect_changes(&alias, old_state, &state);
                }
                self.ctx.project_states.insert(alias.clone(), state);
                self.ctx.recompute_filtered_aliases();
                self.ctx.status_message =
                    Some((format!("Updated: {}", alias), std::time::Instant::now()));
                self.needs_redraw = true;
            }
            Action::GitLogLoaded {
                alias,
                entries,
                planning_only,
            } => {
                let cache = self.ctx.view_cache.entry(alias).or_default();
                cache.git_entries = entries;
                cache.git_planning_only = planning_only;
                cache.loading_git = false;
                self.needs_redraw = true;
            }
            Action::GitDiffStatLoaded { alias, stat } => {
                let cache = self.ctx.view_cache.entry(alias).or_default();
                cache.git_diff_stat = Some(stat);
                cache.loading_diff = false;
                self.needs_redraw = true;
            }
            Action::SessionsDetected { sessions } => {
                self.active_sessions = sessions.clone();
                self.ctx.active_sessions = sessions;
                self.needs_redraw = true;
            }
            Action::CreateProjectResult {
                alias,
                path,
                success,
                error,
            } => {
                if success {
                    // Register the new project
                    if let Err(e) =
                        registry::add_project_unchecked(&mut self.ctx.config, &alias, &path)
                    {
                        self.ctx.error_message = Some(format!("Failed to register: {}", e));
                        self.needs_redraw = true;
                        return;
                    }
                    if let Err(e) = save_config(&self.ctx.config, &self.ctx.config_path) {
                        self.ctx.error_message = Some(format!("Failed to save config: {}", e));
                        self.needs_redraw = true;
                        return;
                    }

                    // Start file watcher on the new project
                    let planning_dir = path.join(".planning");
                    if planning_dir.is_dir() {
                        if let Some(ref mut watcher) = self.ctx.watcher {
                            let _ = watcher.watch(&planning_dir);
                        }
                    } else {
                        if let Some(ref tx) = self.ctx.event_tx {
                            let tx = tx.clone();
                            let planning_poll = planning_dir.clone();
                            tokio::spawn(async move {
                                for _ in 0..30 {
                                    if planning_poll.is_dir() {
                                        let _ = tx.send(Action::FileChanged {
                                            project_path: planning_poll
                                                .parent()
                                                .unwrap()
                                                .to_path_buf(),
                                        });
                                        break;
                                    }
                                    tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                                }
                            });
                        }
                    }

                    // Load project state for the new alias
                    let state = state_reader::parse_project_state(&planning_dir);
                    self.ctx.project_states.insert(alias.clone(), state);

                    self.ctx.status_message = Some((
                        format!("Created project \"{}\"", alias),
                        std::time::Instant::now(),
                    ));
                    // Pop back to normal screen if we're on the create screen
                    // The CreateProjectScreen already popped itself via ScreenAction::Pop
                    self.ctx.input_buffer.clear();
                    self.ctx.error_message = None;
                    self.ctx.recompute_filtered_aliases();
                    if let Some(pos) = self.ctx.filtered_aliases.iter().position(|a| a == &alias) {
                        self.ctx.table_state.select(Some(pos));
                    }
                    self.needs_redraw = true;
                } else {
                    self.ctx.error_message =
                        Some(error.unwrap_or_else(|| "Unknown error creating project".to_string()));
                    self.ctx.input_buffer.clear();
                    self.needs_redraw = true;
                }
            }
            Action::ArchiveMilestonesDiscovered { alias, milestones } => {
                let cache = self.ctx.view_cache.entry(alias).or_default();
                cache.archive_milestones = milestones;
                cache.archive_loading = false;
                self.needs_redraw = true;
            }
            Action::ArchiveLoaded {
                alias,
                milestone,
                data,
            } => {
                self.ctx.archive_cache.insert(milestone, data);
                let cache = self.ctx.view_cache.entry(alias).or_default();
                cache.archive_loading = false;
                self.needs_redraw = true;
            }
        }
    }

    fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers) {
        // Ctrl+C always quits regardless of mode
        if modifiers.contains(KeyModifiers::CONTROL) && code == KeyCode::Char('c') {
            self.should_quit = true;
            return;
        }

        if let Some(screen) = self.screen_stack.last_mut() {
            let action = screen.handle_key(code, modifiers, &mut self.ctx);
            self.process_screen_action(action);
        }
    }

    fn process_screen_action(&mut self, action: ScreenAction) {
        match action {
            ScreenAction::None => {}
            ScreenAction::Push(screen) => {
                self.screen_stack.push(screen);
                self.needs_redraw = true;
            }
            ScreenAction::Pop => {
                if self.screen_stack.len() > 1 {
                    self.screen_stack.pop();
                    self.needs_redraw = true;
                }
            }
            ScreenAction::Quit => {
                self.should_quit = true;
            }
            ScreenAction::SetStatusMessage(msg) => {
                self.ctx.status_message = Some((msg, std::time::Instant::now()));
                self.needs_redraw = true;
            }
            ScreenAction::SuspendAndEdit(path) => {
                self.pending_editor = Some(path);
            }
            ScreenAction::DispatchAction(action) => {
                if let Some(tx) = &self.ctx.event_tx {
                    let _ = tx.send(*action);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state_reader::roadmap_md::RoadmapPhase;

    #[test]
    fn test_format_phase_display_completed_milestone() {
        let state = ProjectState {
            completed_phases: 4,
            total_phases: 4,
            milestone: "v1.0".to_string(),
            phases: vec![],
            ..Default::default()
        };
        assert_eq!(format_phase_display(&state), "v1.0 Complete");
    }

    #[test]
    fn test_format_phase_display_completed_no_milestone() {
        let state = ProjectState {
            completed_phases: 4,
            total_phases: 4,
            milestone: "".to_string(),
            phases: vec![],
            ..Default::default()
        };
        assert_eq!(format_phase_display(&state), "Complete");
    }

    #[test]
    fn test_format_phase_display_in_progress() {
        let state = ProjectState {
            completed_phases: 1,
            total_phases: 4,
            milestone: "v1.0".to_string(),
            phases: vec![
                RoadmapPhase {
                    number: "1".to_string(),
                    name: "Foundation".to_string(),
                    description: String::new(),
                    completed: true,
                    total_plans: 0,
                    completed_plans: 0,
                },
                RoadmapPhase {
                    number: "2".to_string(),
                    name: "Dashboard".to_string(),
                    description: String::new(),
                    completed: false,
                    total_plans: 0,
                    completed_plans: 0,
                },
            ],
            ..Default::default()
        };
        assert_eq!(format_phase_display(&state), "P2: Dashboard");
    }
}
