use crate::action::Action;
use crate::change_tracker::ChangeTracker;
use crate::config::{load_config, save_config, Config};
use crate::registry;
use crate::state_reader::{self, ProjectState};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::widgets::TableState;
use std::collections::HashMap;
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub enum InputMode {
    Normal,
    AddAlias,
    AddPath { alias: String },
    DeleteConfirm { alias: String },
    Search,
    HelpOverlay,
    DetailView { alias: String },
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
    pub config: Config,
    pub config_path: PathBuf,
    pub input_mode: InputMode,
    pub input_buffer: String,
    pub table_state: TableState,
    pub project_states: HashMap<String, ProjectState>,
    pub status_message: Option<(String, std::time::Instant)>,
    pub error_message: Option<String>,
    pub needs_redraw: bool,
    pub filter_text: String,
    pub filtered_aliases: Vec<String>,
    pub last_refresh: HashMap<String, std::time::Instant>,
    pub detail_scroll_offset: u16,
    pub change_tracker: ChangeTracker,
}

impl App {
    pub fn new(config_path: PathBuf) -> anyhow::Result<Self> {
        let config = load_config(&config_path)?;
        let mut table_state = TableState::default();
        if !config.projects.is_empty() {
            table_state.select(Some(0));
        }

        let mut app = App {
            should_quit: false,
            config,
            config_path,
            input_mode: InputMode::Normal,
            input_buffer: String::new(),
            table_state,
            project_states: HashMap::new(),
            status_message: None,
            error_message: None,
            needs_redraw: true,
            filter_text: String::new(),
            filtered_aliases: Vec::new(),
            last_refresh: HashMap::new(),
            detail_scroll_offset: 0,
            change_tracker: ChangeTracker::new(),
        };
        app.filtered_aliases = app.sorted_aliases();
        Ok(app)
    }

    /// Load project states for all registered projects synchronously.
    /// Acceptable for Phase 1 stub with small project counts.
    pub fn load_project_states(&mut self) {
        for (alias, project) in &self.config.projects {
            let planning_dir = project.path.join(".planning");
            let state = state_reader::parse_project_state(&planning_dir);
            self.project_states.insert(alias.clone(), state);
        }
        self.recompute_filtered_aliases();
    }

    /// Record initial snapshots for all loaded project states (for change detection).
    pub fn init_change_tracker(&mut self) {
        for (alias, state) in &self.project_states {
            self.change_tracker.record_initial(alias, state);
        }
    }

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

    pub fn update(&mut self, action: Action) {
        match action {
            Action::Tick => {
                // Expire status messages after 3 seconds
                if let Some((_, instant)) = &self.status_message {
                    if instant.elapsed() > std::time::Duration::from_secs(3) {
                        self.status_message = None;
                        self.needs_redraw = true;
                    }
                }
            }
            Action::Quit => {
                self.should_quit = true;
            }
            Action::RawKey(key_event) => {
                self.handle_key(key_event.code, key_event.modifiers);
            }
            Action::Resize => {
                self.needs_redraw = true;
            }
            Action::Noop => {}
            Action::AddProjectConfirm { alias, path } => {
                self.do_add_project(&alias, &path);
            }
            Action::RemoveProjectConfirm { alias } => {
                self.do_remove_project(&alias);
            }
            Action::FileChanged { project_path } => {
                // Find the alias matching this project path
                let alias = self
                    .config
                    .projects
                    .iter()
                    .find(|(_, proj)| proj.path == project_path)
                    .map(|(alias, _)| alias.clone());

                if let Some(alias) = alias {
                    // Dedup: skip if last refresh was less than 500ms ago
                    let now = std::time::Instant::now();
                    if let Some(last) = self.last_refresh.get(&alias) {
                        if now.duration_since(*last) < std::time::Duration::from_millis(500) {
                            return;
                        }
                    }

                    // Re-parse only the changed project
                    let planning_dir = project_path.join(".planning");
                    let new_state = state_reader::parse_project_state(&planning_dir);

                    // Detect changes before replacing the old state
                    if let Some(old_state) = self.project_states.get(&alias) {
                        self.change_tracker
                            .detect_changes(&alias, old_state, &new_state);
                    }

                    self.project_states.insert(alias.clone(), new_state);
                    self.last_refresh.insert(alias.clone(), now);
                    self.recompute_filtered_aliases();
                    self.status_message = Some((
                        format!("Updated: {}", alias),
                        std::time::Instant::now(),
                    ));
                    self.needs_redraw = true;
                }
            }
            Action::ProjectLoaded { alias, state } => {
                if let Some(s) = state {
                    self.project_states.insert(alias, s);
                }
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

        match &self.input_mode.clone() {
            InputMode::Normal => self.handle_normal_key(code),
            InputMode::AddAlias => self.handle_add_alias_key(code),
            InputMode::AddPath { alias } => {
                let alias = alias.clone();
                self.handle_add_path_key(code, &alias);
            }
            InputMode::DeleteConfirm { alias } => {
                let alias = alias.clone();
                self.handle_delete_confirm_key(code, &alias);
            }
            InputMode::Search => self.handle_search_key(code),
            InputMode::HelpOverlay => self.handle_help_key(code),
            InputMode::DetailView { .. } => self.handle_detail_key(code),
        }
    }

    fn handle_normal_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('q') => {
                self.should_quit = true;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.move_selection_down();
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.move_selection_up();
            }
            KeyCode::Char('a') => {
                self.input_mode = InputMode::AddAlias;
                self.input_buffer.clear();
                self.error_message = None;
                self.needs_redraw = true;
            }
            KeyCode::Char('d') => {
                if let Some(alias) = self.selected_alias() {
                    self.input_mode = InputMode::DeleteConfirm { alias };
                    self.needs_redraw = true;
                }
            }
            KeyCode::Enter => {
                if let Some(alias) = self.selected_alias() {
                    self.detail_scroll_offset = 0;
                    self.input_mode = InputMode::DetailView { alias };
                    self.needs_redraw = true;
                }
            }
            KeyCode::Char('/') => {
                self.input_mode = InputMode::Search;
                self.input_buffer.clear();
                self.filter_text.clear();
                self.recompute_filtered_aliases();
                self.needs_redraw = true;
            }
            KeyCode::Char('?') => {
                self.input_mode = InputMode::HelpOverlay;
                self.needs_redraw = true;
            }
            _ => {}
        }
    }

    fn handle_search_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char(c) => {
                self.filter_text.push(c);
                self.recompute_filtered_aliases();
                if !self.filtered_aliases.is_empty() {
                    self.table_state.select(Some(0));
                } else {
                    self.table_state.select(None);
                }
                self.needs_redraw = true;
            }
            KeyCode::Backspace => {
                self.filter_text.pop();
                self.recompute_filtered_aliases();
                if !self.filtered_aliases.is_empty() {
                    self.table_state.select(Some(0));
                } else {
                    self.table_state.select(None);
                }
                self.needs_redraw = true;
            }
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.filter_text.clear();
                self.recompute_filtered_aliases();
                if !self.filtered_aliases.is_empty() {
                    self.table_state.select(Some(0));
                } else {
                    self.table_state.select(None);
                }
                self.needs_redraw = true;
            }
            KeyCode::Enter => {
                // Confirm filter and return to normal mode (filter stays active)
                self.input_mode = InputMode::Normal;
                self.needs_redraw = true;
            }
            _ => {}
        }
    }

    fn handle_help_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char('?') | KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.needs_redraw = true;
            }
            _ => {} // Consume all other keys -- do NOT fall through
        }
    }

    fn handle_detail_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.input_mode = InputMode::Normal;
                self.detail_scroll_offset = 0;
                self.needs_redraw = true;
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.detail_scroll_offset = self.detail_scroll_offset.saturating_add(1);
                self.needs_redraw = true;
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.detail_scroll_offset = self.detail_scroll_offset.saturating_sub(1);
                self.needs_redraw = true;
            }
            KeyCode::Char('?') => {
                self.input_mode = InputMode::HelpOverlay;
                self.needs_redraw = true;
            }
            _ => {}
        }
    }

    fn handle_add_alias_key(&mut self, code: KeyCode) {
        match code {
            KeyCode::Char(c) => {
                self.input_buffer.push(c);
                self.error_message = None;
                self.needs_redraw = true;
            }
            KeyCode::Backspace => {
                self.input_buffer.pop();
                self.needs_redraw = true;
            }
            KeyCode::Enter => {
                let alias = self.input_buffer.trim().to_string();
                if alias.is_empty() {
                    self.error_message = Some("Alias cannot be empty.".to_string());
                    self.needs_redraw = true;
                    return;
                }
                if alias.contains(char::is_whitespace) {
                    self.error_message =
                        Some("Alias cannot contain whitespace.".to_string());
                    self.needs_redraw = true;
                    return;
                }
                if self.config.projects.contains_key(&alias) {
                    self.error_message = Some(format!(
                        "Alias \"{}\" already exists. Choose a different alias.",
                        alias
                    ));
                    self.needs_redraw = true;
                    return;
                }
                self.input_mode = InputMode::AddPath {
                    alias: alias.clone(),
                };
                self.input_buffer.clear();
                self.error_message = None;
                self.needs_redraw = true;
            }
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.input_buffer.clear();
                self.error_message = None;
                self.needs_redraw = true;
            }
            _ => {}
        }
    }

    fn handle_add_path_key(&mut self, code: KeyCode, alias: &str) {
        match code {
            KeyCode::Char(c) => {
                self.input_buffer.push(c);
                self.error_message = None;
                self.needs_redraw = true;
            }
            KeyCode::Backspace => {
                self.input_buffer.pop();
                self.needs_redraw = true;
            }
            KeyCode::Enter => {
                let path = PathBuf::from(self.input_buffer.trim());
                self.do_add_project(alias, &path);
            }
            KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.input_buffer.clear();
                self.error_message = None;
                self.needs_redraw = true;
            }
            _ => {}
        }
    }

    fn handle_delete_confirm_key(&mut self, code: KeyCode, alias: &str) {
        match code {
            KeyCode::Char('y') => {
                self.do_remove_project(alias);
            }
            KeyCode::Char('n') | KeyCode::Esc => {
                self.input_mode = InputMode::Normal;
                self.needs_redraw = true;
            }
            _ => {}
        }
    }

    fn do_add_project(&mut self, alias: &str, path: &PathBuf) {
        // Canonicalize the path
        let canonical = path.canonicalize().unwrap_or_else(|_| path.clone());

        match registry::add_project(&mut self.config, alias, &canonical) {
            Ok(()) => {
                if let Err(e) = save_config(&self.config, &self.config_path) {
                    self.error_message = Some(format!("Failed to save config: {}", e));
                    self.needs_redraw = true;
                    return;
                }
                // Load state for the new project
                let planning_dir = canonical.join(".planning");
                let state = state_reader::parse_project_state(&planning_dir);
                self.project_states.insert(alias.to_string(), state);

                self.status_message = Some((
                    format!("Added \"{}\"", alias),
                    std::time::Instant::now(),
                ));
                self.input_mode = InputMode::Normal;
                self.input_buffer.clear();
                self.error_message = None;

                // Recompute filtered aliases and select the newly added project
                self.recompute_filtered_aliases();
                if let Some(pos) = self.filtered_aliases.iter().position(|a| a == alias) {
                    self.table_state.select(Some(pos));
                }
                self.needs_redraw = true;
            }
            Err(e) => {
                self.error_message = Some(e.to_string());
                self.needs_redraw = true;
            }
        }
    }

    fn do_remove_project(&mut self, alias: &str) {
        match registry::remove_project(&mut self.config, alias) {
            Ok(()) => {
                if let Err(e) = save_config(&self.config, &self.config_path) {
                    self.error_message = Some(format!("Failed to save config: {}", e));
                    self.needs_redraw = true;
                    return;
                }
                self.project_states.remove(alias);
                self.status_message = Some((
                    format!("Removed \"{}\"", alias),
                    std::time::Instant::now(),
                ));
                self.input_mode = InputMode::Normal;

                // Recompute filtered aliases and adjust table selection
                self.recompute_filtered_aliases();
                if self.filtered_aliases.is_empty() {
                    self.table_state.select(None);
                } else {
                    let selected = self
                        .table_state
                        .selected()
                        .unwrap_or(0)
                        .min(self.filtered_aliases.len() - 1);
                    self.table_state.select(Some(selected));
                }
                self.needs_redraw = true;
            }
            Err(e) => {
                self.error_message = Some(e.to_string());
                self.needs_redraw = true;
            }
        }
    }

    fn move_selection_down(&mut self) {
        let count = self.filtered_aliases.len();
        if count == 0 {
            return;
        }
        let current = self.table_state.selected().unwrap_or(0);
        let next = if current >= count - 1 { 0 } else { current + 1 };
        self.table_state.select(Some(next));
        self.needs_redraw = true;
    }

    fn move_selection_up(&mut self) {
        let count = self.filtered_aliases.len();
        if count == 0 {
            return;
        }
        let current = self.table_state.selected().unwrap_or(0);
        let next = if current == 0 { count - 1 } else { current - 1 };
        self.table_state.select(Some(next));
        self.needs_redraw = true;
    }
}
