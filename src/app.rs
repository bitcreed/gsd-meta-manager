use crate::action::Action;
use crate::change_tracker::ChangeTracker;
use crate::config::{load_config, save_config, Config};
use crate::registry;
use crate::session_detector::ClaudeSession;
use crate::state_reader::{self, ProjectState};
use crate::ui::screens::normal::NormalScreen;
use crate::ui::screens::{AppContext, Screen, ScreenAction};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::widgets::TableState;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
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
    Defaults,
    Browse,
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
    // ADR-2207 status vocabulary takes precedence over the generic keyword
    // matching below. Milestone-terminal statuses are truly Complete; the
    // intermediate `All phases complete` is NOT (the milestone awaits
    // `/gsd:complete-milestone`) — classify it as Idle so it doesn't fall
    // through to the `contains("complete")` branch.
    if state_reader::state_md::is_milestone_terminal(status) {
        return StatusCategory::Complete;
    }
    if state_reader::state_md::is_all_phases_complete(status) {
        return StatusCategory::Idle;
    }
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
    // ADR-2207: an explicit `current_phase_name` from STATE.md frontmatter is
    // the most accurate label; prefer it over the count-derived fallback.
    if !state.current_phase_name.is_empty() {
        return state.current_phase_name.clone();
    }
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
        Ok(Self::from_config(config, config_path))
    }

    /// Build an `App` with an empty config and no disk access.
    ///
    /// `App::new` reads `config.json` from disk, which makes it useless in a
    /// unit test: the result depends on whatever the developer or the CI runner
    /// happens to have registered. This constructor is what makes the
    /// `main_loop` property tests possible without a config file.
    pub fn new_for_test() -> Self {
        Self::from_config(Config::new(), PathBuf::from("/nonexistent/config.json"))
    }

    fn from_config(config: Config, config_path: PathBuf) -> Self {
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
            exec_tx: None,
            run_states: HashMap::new(),
            reparse_dispatches: 0,
            journal_cursors: HashMap::new(),
            observed_runs: HashMap::new(),
            session_spawned_runs: std::collections::HashSet::new(),
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

        App {
            should_quit: false,
            needs_redraw: true,
            ctx,
            screen_stack: vec![Box::new(NormalScreen::new())],
            active_sessions: Vec::new(),
            session_poll_counter: 0,
            pending_editor: None,
        }
    }

    /// Apply one executor event to the per-alias driver state (D-17, D-19).
    ///
    /// This is the executor arm's whole handler. It touches only the sibling
    /// map on `AppContext` and the redraw flag — never `ProjectState`, whose
    /// derived equality suppresses status-bar spam.
    ///
    /// Note what it deliberately does *not* do: `ExecutionEvent::Exited` moves
    /// the alias to `Stopping`, **not** to `Finished(outcome)`. A `RunOutcome`
    /// cannot be derived from an exit status alone — D-26's matrix needs the
    /// collected turns and the disk delta as well — so fabricating one here
    /// would be a lie the UI then renders. The driver that owns the run
    /// (Phase 17) is what closes the state out.
    ///
    /// Nor does `ExecutionEvent::EventsDropped` move the state at all. The
    /// count is carried for the journal (plan 16-06) and for a future Phase 18
    /// surface, and it is deliberately not allowed to move a run's state: it is
    /// emitted in the terminal path, after the process has stopped producing,
    /// so letting it reach the catch-all below would let a diagnostic
    /// manufacture liveness (D-33).
    pub fn apply_exec_event(&mut self, event: crate::main_loop::ExecEvent) {
        use crate::executor::{ExecutionEvent, RunState};

        let crate::main_loop::ExecEvent { alias, event } = event;
        let state = self.ctx.run_states.entry(alias).or_default();

        match event {
            ExecutionEvent::SessionStarted { .. } => *state = RunState::Running,
            ExecutionEvent::Exited(_) => *state = RunState::Stopping,
            // A diagnostic about what the stream *lost*, not evidence that
            // anything is still running. Explicit rather than left to the
            // catch-all, which would promote an idle alias on reasoning that
            // does not apply to a terminal-path event (D-19, D-33).
            ExecutionEvent::EventsDropped { .. } => {}
            // Any other observed line means the process is alive and talking.
            // If we somehow never saw the gated `system/init` (a torn first
            // line, say), record that a run is at least under way rather than
            // leaving the alias reading `Idle` while output streams.
            _ => {
                if *state == RunState::Idle {
                    *state = RunState::Starting;
                }
            }
        }

        self.needs_redraw = true;
    }

    /// **The only place a full project re-parse is scheduled.**
    ///
    /// That claim is not decoration — the OBS-06 test asserts
    /// `ctx.reparse_dispatches` does not move across five hundred journal
    /// appends, and the assertion is only as strong as this being the sole
    /// writer of that counter. If a second dispatch site ever appears, route it
    /// through here rather than duplicating the body.
    ///
    /// `parse_project_state` reads STATE.md, ROADMAP.md, QUEUE.md, HANDOFF,
    /// every phase directory's disk inference and every workstream, and shells
    /// out to git for last-activity. It is the bill "free watching" was quietly
    /// running up.
    ///
    /// The counter is incremented **before** the `spawn_blocking` so the count
    /// is synchronous and cannot race the blocking task (RESEARCH §7.3).
    fn schedule_reparse(&mut self, alias: &str, project_path: &Path) {
        let Some(tx) = &self.ctx.event_tx else {
            return;
        };
        let tx = tx.clone();
        let alias_for_task = alias.to_string();
        let planning_dir = project_path.join(".planning");

        self.ctx.reparse_dispatches += 1;
        tokio::task::spawn_blocking(move || {
            let state = state_reader::parse_project_state(&planning_dir);
            let _ = tx.send(Action::ProjectStateLoaded {
                alias: alias_for_task,
                state: Box::new(state),
            });
        });
        self.ctx
            .last_refresh
            .insert(alias.to_string(), std::time::Instant::now());
    }

    /// Schedule a byte-offset tail of one run's journal (D-12, D-16).
    ///
    /// Note the three things this deliberately does **not** do, because they
    /// are the literal content of OBS-06:
    ///
    /// * It does **not** call `parse_project_state`. A tail costs
    ///   bytes-appended-since-the-last-read, so its cost is proportional to
    ///   actual new data rather than to project size — that proportionality is
    ///   the whole difference (D-15).
    /// * It does **not** touch `last_refresh`. Sharing the 500 ms dedup map
    ///   would let a journal append suppress a genuine `STATE.md` re-parse for
    ///   half a second, trading a performance bug for a correctness bug (D-14).
    /// * It does **not** increment `reparse_dispatches`.
    ///
    /// The read runs on `spawn_blocking` with its result returned as an
    /// `Action` on a cloned sender, following the idiom this file already uses
    /// for the re-parse and for session detection. No file I/O on the render
    /// thread (D-16).
    fn schedule_journal_tail(&mut self, alias: &str, project_path: &Path, run_id: &str) {
        let Some(tx) = &self.ctx.event_tx else {
            return;
        };
        let tx = tx.clone();

        // `.copied()` — and `JournalCursor` staying `Copy` is what keeps this
        // line unchanged now that the value carries a `seq` as well as an offset
        // (D-28, PATTERNS note 4).
        let key = (alias.to_string(), run_id.to_string());
        let stored = self.ctx.journal_cursors.get(&key).copied().unwrap_or_default();
        let (alias_for_task, run_id_for_task) = key;

        let planning_dir = project_path.join(".planning");
        let journal = crate::journal::run_paths(&planning_dir, run_id).journal;

        tokio::task::spawn_blocking(move || {
            let read = match crate::journal::reader::tail_lines(&journal, stored.cursor) {
                Ok(read) => read,
                Err(e) => {
                    // The error KIND only. Neither the path nor the message
                    // body is logged (D-28).
                    tracing::warn!(
                        alias = %alias_for_task,
                        run_id = %run_id_for_task,
                        kind = ?e.kind(),
                        "journal tail failed",
                    );
                    return;
                }
            };

            if read.restarted {
                tracing::warn!(
                    alias = %alias_for_task,
                    run_id = %run_id_for_task,
                    restarted = true,
                    "journal tail: the file shrank and the cursor was reset",
                );
            }
            if read.skipped_oversize {
                tracing::warn!(
                    alias = %alias_for_task,
                    run_id = %run_id_for_task,
                    skipped_oversize = true,
                    "journal tail: stepped over a line that exceeded the read bound",
                );
            }

            let mut records = Vec::with_capacity(read.lines.len());
            let mut unparseable: u64 = 0;
            for line in &read.lines {
                match crate::journal::reader::parse_line(line) {
                    crate::journal::reader::ParsedLine::Record(record) => records.push(record),
                    // A count, never the line (D-28).
                    crate::journal::reader::ParsedLine::Unparseable { .. } => unparseable += 1,
                }
            }
            if unparseable > 0 {
                tracing::warn!(
                    alias = %alias_for_task,
                    run_id = %run_id_for_task,
                    count = unparseable,
                    "journal tail: lines did not parse",
                );
            }

            // An EMPTY batch retains the previous `last_seq` rather than
            // resetting it. A tail that finds nothing new is the common case —
            // the watcher fires on a directory before the first append, and a
            // torn trailing line yields no complete record — and resetting to
            // zero on any of those would silently disarm the boundary check for
            // the next batch that does carry records (D-28).
            let last_seq = records.last().map_or(stored.last_seq, |record| record.seq);

            let _ = tx.send(Action::DriverJournalAppended {
                alias: alias_for_task,
                run_id: run_id_for_task,
                records,
                cursor: crate::journal::reader::JournalCursor {
                    cursor: read.cursor,
                    last_seq,
                },
            });
        });
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

    /// Auto-register any active Claude sessions whose working_dir is an
    /// unregistered GSD project. Persists config, starts the file watcher,
    /// loads initial project state, and surfaces a status message per
    /// newly-added project.
    pub fn auto_register_new_sessions(&mut self) {
        let added =
            registry::auto_register_from_sessions(&mut self.ctx.config, &self.active_sessions);

        if added.is_empty() {
            return;
        }

        for (alias, path) in &added {
            if let Err(e) = save_config(&self.ctx.config, &self.ctx.config_path) {
                tracing::warn!(
                    alias = %alias,
                    error = %e,
                    "auto-register: save_config failed",
                );
                // Continue; the in-memory registration still helps this session.
            }

            let planning_dir = path.join(".planning");
            if let Some(ref mut watcher) = self.ctx.watcher {
                if let Err(e) = watcher.watch(&planning_dir) {
                    tracing::warn!(
                        alias = %alias,
                        path = %planning_dir.display(),
                        error = %e,
                        "auto-register: watcher.watch failed",
                    );
                }
            }

            let state = state_reader::parse_project_state(&planning_dir);
            self.ctx
                .change_tracker
                .record_initial(alias, &state);
            self.ctx.project_states.insert(alias.clone(), state);

            tracing::info!(
                alias = %alias,
                path = %path.display(),
                "Auto-registered GSD project from active Claude session",
            );
            self.ctx.status_message = Some((
                format!("Auto-registered: {}", alias),
                std::time::Instant::now(),
            ));
        }

        self.ctx.recompute_filtered_aliases();
        self.needs_redraw = true;
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

                    // The driver reconciliation probe rides THIS counter and
                    // must never get one of its own (D-13, ARCHITECTURE §4.4(c)
                    // are both explicit). Two timers polling `/proc` and
                    // `run.json` at slightly different phases would double the
                    // syscall load for no extra freshness and would make "how
                    // stale can the dashboard be?" a question with two answers.
                    // Do not tidy this into its own interval.
                    //
                    // `spawn_blocking` for the same reason as the line above:
                    // `/proc` reads and `run.json` reads are synchronous fs work
                    // and this file's idiom for that is unambiguous.
                    if let Some(ref tx) = self.ctx.event_tx {
                        let tx: UnboundedSender<Action> = tx.clone();
                        let projects = self.ctx.config.projects.clone();
                        tokio::task::spawn_blocking(move || {
                            let runs = crate::driver::reconcile::reconcile_all(&projects);
                            let _ = tx.send(Action::RunsReconciled { runs });
                        });
                    }

                    // The prune rides the SAME counter, for the same reason the
                    // reconciliation probe above does (D-27 says so explicitly).
                    // It is pure in-memory map work — no syscall, no file read —
                    // so it runs inline rather than on a blocking task.
                    self.prune_driver_maps();
                }
            }
            Action::RawKey(key_event) => {
                self.handle_key(key_event.code, key_event.modifiers);
            }
            Action::Resize => {
                self.needs_redraw = true;
            }
            Action::FileChanged {
                project_path,
                changed_path,
            } => {
                // Find the alias matching this project path
                let alias = self
                    .ctx
                    .config
                    .projects
                    .iter()
                    .find(|(_, proj)| proj.path == project_path)
                    .map(|(alias, _)| alias.clone());

                if let Some(alias) = alias {
                    // Classify BEFORE the dedup check. D-14 is explicit that
                    // the two routes must not share `last_refresh`: if the
                    // driver route went through the 500ms map, one journal
                    // append could suppress a genuine STATE.md re-parse for
                    // half a second — a performance bug traded for a
                    // correctness bug.
                    match crate::journal::classify_change(&project_path, &changed_path) {
                        crate::journal::ChangeKind::DriverJournal { run_id } => {
                            // No extra throttle here, deliberately (D-15). A
                            // tail read costs bytes-appended-since-the-last-
                            // read, so its cost tracks actual new data rather
                            // than project size — which is the whole
                            // difference from `parse_project_state`, and the
                            // 200ms watcher debounce already floors the rate.
                            self.schedule_journal_tail(&alias, &project_path, &run_id);
                        }
                        crate::journal::ChangeKind::Planning => {
                            // Dedup: skip if last refresh was less than 500ms ago
                            let now = std::time::Instant::now();
                            if let Some(last) = self.ctx.last_refresh.get(&alias) {
                                if now.duration_since(*last)
                                    < std::time::Duration::from_millis(500)
                                {
                                    return;
                                }
                            }

                            self.schedule_reparse(&alias, &project_path);

                            // Auto-start watcher if not yet watching
                            let planning_dir_check = project_path.join(".planning");
                            if planning_dir_check.is_dir() {
                                if let Some(ref mut watcher) = self.ctx.watcher {
                                    let _ = watcher.watch(&planning_dir_check);
                                }
                            }
                        }
                    }
                }
            }
            Action::ProjectStateLoaded { alias, state } => {
                // Detect changes before replacing the old state.
                // `changed` is true if there was no prior state (first load) or
                // the freshly-parsed snapshot differs from the cached one.
                // Watcher events that don't actually move state (e.g. unrelated
                // .planning/ writes) skip the user-facing notification.
                let changed = match self.ctx.project_states.get(&alias) {
                    Some(old_state) => {
                        self.ctx
                            .change_tracker
                            .detect_changes(&alias, old_state, &state);
                        *old_state != *state
                    }
                    None => true,
                };
                self.ctx.project_states.insert(alias.clone(), *state);
                if changed {
                    self.ctx.recompute_filtered_aliases();
                    self.ctx.status_message =
                        Some((format!("Updated: {}", alias), std::time::Instant::now()));
                    self.needs_redraw = true;
                }
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
                self.auto_register_new_sessions();
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
                                            // The `.planning` directory itself
                                            // is what was just observed. It
                                            // classifies as `Planning`, which
                                            // is the re-parse this poll exists
                                            // to trigger.
                                            changed_path: planning_poll.clone(),
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
            // One tail read landed. This handler touches its own cursor entry
            // and nothing else — modelled on `apply_exec_event`, which
            // likewise records what it deliberately does *not* do.
            //
            // It does NOT set `needs_redraw`. This phase ships no surface that
            // renders journal content (D-36), so a redraw here would schedule
            // a frame that cannot differ from the one already on screen. Read
            // the omission as a choice, not as a bug — Phase 18 is what adds
            // the surface and the flag together.
            //
            // It also does NOT touch `ProjectState` (D-18) or `last_refresh`
            // (D-14).
            Action::DriverJournalAppended {
                alias,
                run_id,
                records,
                cursor,
            } => {
                // `seq` is monotonic from 1 and exists so a tailing reader can
                // detect gaps (D-03). A gap is reported as a COUNT and never as
                // a parse failure, and never with a record body (D-28, D-30).
                //
                // This calls the shared reader function rather than
                // re-implementing its `windows(2)` filter, which is what it used
                // to do. Two copies of a comparison are two things to keep in
                // agreement, and they had already diverged in the way that
                // matters: the inline copy could not be seeded, so it could only
                // ever see a gap that fell inside one batch.
                let key = (alias, run_id);
                let previous_last_seq = self
                    .ctx
                    .journal_cursors
                    .get(&key)
                    .map_or(0, |stored| stored.last_seq);
                let gaps =
                    crate::journal::reader::seq_gaps_from(previous_last_seq, &records).len();
                if gaps > 0 {
                    tracing::warn!(
                        alias = %key.0,
                        run_id = %key.1,
                        count = gaps,
                        "journal tail: sequence gaps observed",
                    );
                }

                self.ctx.journal_cursors.insert(key, cursor);
            }
            // The scan is authoritative, so the map is **replaced** and never
            // merged (D-12, D-13). A merge would keep a run in the map after its
            // project was unregistered, or after it ended — the absence of an
            // entry is how the scan says both of those things, and a merge
            // discards exactly that information.
            Action::RunsReconciled { runs } => {
                let observed: HashMap<String, crate::driver::reconcile::ObservedRun> = runs
                    .into_iter()
                    .map(|run| (run.alias.clone(), run))
                    .collect();

                // Equality-guarded redraw, following this file's existing
                // discipline: a scan lands every ~5s for the whole life of the
                // process, and an unconditional redraw here would repaint the
                // frame twelve times a minute forever with nothing changed.
                if self.ctx.observed_runs != observed {
                    // Counts only. A goal string is user content and a run id is
                    // not worth a line every five seconds (D-28).
                    tracing::debug!(
                        observed = observed.len(),
                        live = observed.values().filter(|run| run.is_live()).count(),
                        "driver reconciliation scan applied",
                    );
                    self.ctx.observed_runs = observed;
                    self.needs_redraw = true;
                }
            }
            // The goal is `None`: Phase 17 records a goal into `RunRecord.goal`
            // when one is supplied on the command line and interprets nothing
            // (Phase 21 owns goal decomposition), and there is no screen to type
            // one into — that is Phase 18's. An empty string is deliberately not
            // passed instead, because `drive_argv` omits the flag entirely for
            // `None` and would otherwise record an empty goal verbatim.
            Action::DriverStartRequested { alias, command } => {
                #[cfg(unix)]
                self.start_driver_run(&alias, &command, None);
                // Off Unix there is no detached spawn to reach, so the request
                // has nowhere to go. Both bindings are consumed explicitly
                // rather than left to an `unused_variables` allow.
                #[cfg(not(unix))]
                let _ = (alias, command);
            }
            Action::DriverStopRequested { alias } => {
                #[cfg(unix)]
                self.stop_driver_run(&alias);
                // Off Unix there is no way to have started a run in the first
                // place, so there is nothing to stop. The binding is consumed
                // explicitly rather than left to an `unused_variables` allow.
                #[cfg(not(unix))]
                let _ = alias;
            }
            // The stop already happened; this is the report. The entry is
            // dropped from the observed map rather than edited, because the map
            // is a projection of the scan and a hand-edited entry would be
            // overwritten by the next one anyway. Dropping it is what the scan
            // itself will do five seconds later, done now so the dashboard does
            // not show a stopped run as live in the meantime.
            Action::DriverStopped {
                alias,
                run_id,
                outcome,
            } => {
                self.ctx.observed_runs.remove(&alias);
                self.ctx.session_spawned_runs.remove(&run_id);
                self.ctx.status_message =
                    Some((format!("{alias}: {outcome}"), std::time::Instant::now()));
                self.needs_redraw = true;
            }
        }
    }

    /// Drop driver state the registry no longer justifies, and bound what stays
    /// (D-27).
    ///
    /// **The leak this closes.** `journal_cursors` is keyed `(alias, run_id)`,
    /// inserted in exactly one place — `Action::DriverJournalAppended` — and
    /// removed in none, so it grew for the whole life of the process: one entry
    /// per run of every project ever tailed, forever. `run_states` and
    /// `observed_runs` had the same shape for an unregistered alias, because
    /// `registry::remove_project` touches neither (it has no access to
    /// `AppContext`; see its doc, which names both cleanup sites).
    ///
    /// Two passes:
    ///
    /// 1. **Unregistered aliases go.** A project that is no longer a key in
    ///    `config.projects` cannot produce another journal append, another run
    ///    state, or another observation, so every entry it owns is dead weight.
    /// 2. **At most [`RETAIN_RUNS`](crate::journal::RETAIN_RUNS) run ids survive
    ///    per alias**, newest kept. Aligning to that constant rather than
    ///    picking a second number is deliberate: it is the on-disk retention
    ///    bound, so the in-memory bound and the disk bound move together instead
    ///    of drifting into two unrelated budgets.
    ///
    /// **"Newest" is a sort on the run-id string and needs no file read.** Run
    /// ids sort lexicographically in chronological order by construction — that
    /// is `new_run_id`'s documented load-bearing property, and the same one
    /// on-disk pruning relies on. A reader might otherwise reach for `run.json`
    /// timestamps, which would turn a map operation into O(runs) file reads.
    ///
    /// Called from the **existing** 20-tick block, never from a timer of its own
    /// (D-13, D-27).
    fn prune_driver_maps(&mut self) {
        let registered = &self.ctx.config.projects;
        self.ctx
            .journal_cursors
            .retain(|(alias, _), _| registered.contains_key(alias));
        self.ctx
            .run_states
            .retain(|alias, _| registered.contains_key(alias));
        self.ctx
            .observed_runs
            .retain(|alias, _| registered.contains_key(alias));

        let mut runs_by_alias: HashMap<&str, Vec<&str>> = HashMap::new();
        for (alias, run_id) in self.ctx.journal_cursors.keys() {
            runs_by_alias
                .entry(alias.as_str())
                .or_default()
                .push(run_id.as_str());
        }

        let mut evict: Vec<(String, String)> = Vec::new();
        for (alias, mut run_ids) in runs_by_alias {
            if run_ids.len() <= crate::journal::RETAIN_RUNS {
                continue;
            }
            run_ids.sort_unstable();
            let excess = run_ids.len() - crate::journal::RETAIN_RUNS;
            for run_id in run_ids.into_iter().take(excess) {
                evict.push((alias.to_string(), run_id.to_string()));
            }
        }
        for key in evict {
            self.ctx.journal_cursors.remove(&key);
        }
    }

    /// Start a driver run against `alias`, or refuse visibly (D-03, D-18).
    ///
    /// The order of the body is the decision:
    ///
    /// 1. Look the alias up, refusing an unknown one through `ctx.error_message`.
    /// 2. [`admit`](crate::driver::spawn::admit) against
    ///    `preferences.driver_max_concurrent`, counting the live entries of the
    ///    reconciliation scan's own result. Refuse through `ctx.error_message`.
    /// 3. Generate the run id **here** — the TUI owns the id so it knows what to
    ///    look for afterwards, and the driver owns the record because `run.json`
    ///    carries the driver's own pid and pgid, which only the driver knows
    ///    (D-03).
    /// 4. Build the argv, spawn detached, report either way.
    ///
    /// **There is deliberately no opt-in check here, and the absence is the
    /// point** (D-16). The gate lives in the child, at `driver::drive`, which is
    /// what makes CTRL-03's "never" literally true for both entry points: a user
    /// typing `gsd-meta-manager drive foo` by hand is refused by the same code
    /// path as this one. A second check here would put the gate at two call
    /// sites, and "exactly one call site" is a property
    /// `tests/spawn_seam_guard.rs` can check while "every branch remembered to
    /// gate" is not.
    ///
    /// Unix-only, like the rest of the driver (D-05). The TUI itself stays
    /// cross-platform; a Windows build simply has no way to start a run.
    #[cfg(unix)]
    pub fn start_driver_run(&mut self, alias: &str, command: &str, goal: Option<&str>) {
        use crate::driver::spawn::{admit, drive_argv, spawn_detached};

        let Some(project) = self.ctx.config.projects.get(alias) else {
            self.ctx.error_message = Some(format!("No registered project named '{alias}'"));
            self.needs_redraw = true;
            return;
        };
        let project_root = project.path.clone();
        // Cloned beside `project_root` and for the same borrow-checker reason:
        // `project` borrows `self.ctx.config`, and the `admit` call below needs
        // `self.ctx` again. It is the config the TUI itself is using, and handing
        // it to the child is what stops the spawned driver resolving this alias in
        // a different registry (CR-03).
        let config_path = self.ctx.config_path.clone();

        let live = self
            .ctx
            .observed_runs
            .values()
            .filter(|run| run.is_live())
            .count();
        if let Err(refusal) = admit(live, self.ctx.config.preferences.driver_max_concurrent) {
            self.ctx.error_message = Some(refusal.to_string());
            self.needs_redraw = true;
            return;
        }

        let run_id = crate::journal::new_run_id(chrono::Utc::now(), &uuid::Uuid::new_v4());
        let argv = drive_argv(&config_path, alias, command, &run_id, goal);

        match spawn_detached(&project_root, &argv) {
            Ok(pid) => {
                // This session is the driver's parent, so its `wait()` belongs
                // to the reaping task `spawn_detached` just created. Recording
                // the run id here is what lets a later stop pick D-07's parent
                // arm; a run absent from this set was adopted after a restart
                // and can only be confirmed dead through `/proc`.
                self.ctx.session_spawned_runs.insert(run_id.clone());
                self.ctx.status_message = Some((
                    format!("Driving {alias} — run {run_id}"),
                    std::time::Instant::now(),
                ));
                // An optimistic entry so the dashboard does not wait up to five
                // seconds for the first scan to notice. The next scan replaces
                // it wholesale from disk, which is what corrects it if the
                // driver refused at its own gate and exited immediately.
                self.ctx.observed_runs.insert(
                    alias.to_string(),
                    crate::driver::reconcile::ObservedRun {
                        alias: alias.to_string(),
                        run_id,
                        pid,
                        // The child is its own group leader, so pgid == pid
                        // (D-04). The scan reads the driver's own record a few
                        // seconds later and would correct this if it were wrong.
                        pgid: pid,
                        started_at: chrono::Utc::now()
                            .to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                        goal: goal.unwrap_or_default().to_string(),
                        gsd_command: command.to_string(),
                        // `Alive` and not the probe's answer: the spawn just
                        // succeeded, so this entry asserts what this process
                        // knows first-hand rather than what /proc could say
                        // about a pid that is microseconds old.
                        liveness: crate::driver::liveness::Liveness::Alive,
                    },
                );
            }
            Err(e) => {
                // A spawn failure is synchronous and leaves genuinely nothing on
                // disk, which is the correct state rather than a lost run (D-03).
                self.ctx.error_message =
                    Some(format!("Could not start a driver for '{alias}': {e}"));
            }
        }
        self.needs_redraw = true;
    }

    /// Stop the observed run on `alias`, or refuse visibly (CTRL-01, D-06, D-07).
    ///
    /// The counterpart to [`App::start_driver_run`], and the order of the body is
    /// the decision:
    ///
    /// 1. Read the `ObservedRun` for the alias **out of the scan's own result**.
    ///    The pid and the pgid are taken here, at dispatch, and never carried in
    ///    the key event that asked for the stop: a pgid captured earlier is a
    ///    pgid that may already name a different run's group, and this is the one
    ///    value in the codebase where being stale means signalling a stranger.
    ///    No entry is a visible refusal through `ctx.error_message`.
    /// 2. Choose D-07's arm from `ctx.session_spawned_runs` — `Parent` if this
    ///    session spawned it, `Adopted` otherwise. That set exists rather than a
    ///    flag on `ObservedRun` because the observed map is replaced wholesale by
    ///    every scan and the disk cannot say who a driver's parent was.
    /// 3. Dispatch on a task and take the result back as
    ///    [`Action::DriverStopped`]. **Nothing here blocks the render thread**:
    ///    the grace alone is twelve seconds, and a loop that stopped painting for
    ///    twelve seconds while stopping a run would look exactly like the hang
    ///    the stop exists to end (TRANS-03).
    ///
    /// Unix-only, like the rest of the driver (D-05).
    #[cfg(unix)]
    pub fn stop_driver_run(&mut self, alias: &str) {
        use crate::driver::kill::ReapArm;

        let Some(run) = self.ctx.observed_runs.get(alias) else {
            self.ctx.error_message = Some(format!("No run is being observed for '{alias}'"));
            self.needs_redraw = true;
            return;
        };
        let pid = run.pid;
        let pgid = run.pgid;
        let run_id = run.run_id.clone();

        let arm = if self.ctx.session_spawned_runs.contains(&run_id) {
            ReapArm::Parent
        } else {
            ReapArm::Adopted
        };

        // The same guard `schedule_journal_tail` uses: without a channel there
        // is nowhere to return the outcome, and a stop whose result cannot be
        // reported is a stop the user cannot tell happened.
        let Some(tx) = &self.ctx.event_tx else {
            return;
        };
        let tx = tx.clone();
        let alias_for_task = alias.to_string();
        let run_id_for_task = run_id.clone();

        tokio::spawn(async move {
            let outcome =
                crate::driver::kill::stop_run(pid, pgid, &run_id_for_task, arm).await;
            let _ = tx.send(Action::DriverStopped {
                alias: alias_for_task,
                run_id: run_id_for_task,
                outcome: outcome.to_string(),
            });
        });

        self.ctx.status_message = Some((
            format!("Stopping {alias} — run {run_id}"),
            std::time::Instant::now(),
        ));
        self.needs_redraw = true;
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

    #[test]
    fn test_classify_status_milestone_terminal_is_complete() {
        assert_eq!(
            classify_status("v1.5.0 milestone complete"),
            StatusCategory::Complete
        );
        assert_eq!(
            classify_status("Awaiting next milestone"),
            StatusCategory::Complete
        );
    }

    #[test]
    fn test_classify_status_all_phases_complete_is_idle() {
        // Intermediate ADR-2207 state — NOT Complete.
        assert_eq!(
            classify_status("All phases complete"),
            StatusCategory::Idle
        );
    }

    #[test]
    fn test_classify_status_ordinary_unchanged() {
        assert_eq!(classify_status("executing"), StatusCategory::Active);
        assert_eq!(classify_status("blocked"), StatusCategory::Blocked);
        assert_eq!(classify_status("ready to plan"), StatusCategory::Idle);
        assert_eq!(classify_status("shipped"), StatusCategory::Unknown);
        // A plain phase-completion status still classifies as Complete.
        assert_eq!(classify_status("phase complete"), StatusCategory::Complete);
    }

    #[test]
    fn test_format_phase_display_prefers_current_phase_name() {
        let state = ProjectState {
            current_phase_name: "Live State".to_string(),
            completed_phases: 2,
            total_phases: 4,
            phases: vec![],
            ..Default::default()
        };
        assert_eq!(format_phase_display(&state), "Live State");
    }

    #[test]
    fn a_drop_report_does_not_move_a_runs_state() {
        use crate::executor::{ExecutionEvent, RunState};
        use crate::main_loop::ExecEvent;

        let mut app = App::new_for_test();
        app.ctx
            .run_states
            .insert("busy".to_string(), RunState::Running);

        app.apply_exec_event(ExecEvent::new(
            "busy",
            ExecutionEvent::EventsDropped { count: 40 },
        ));

        assert_eq!(
            app.ctx.run_states.get("busy"),
            Some(&RunState::Running),
            "a drop report is a diagnostic; it must not disturb a state the \
             stream already established"
        );

        // The load-bearing half. The catch-all arm promotes an `Idle` alias to
        // `Starting` on the reasoning that any observed line means the process
        // is alive and talking — but this report is emitted in the terminal
        // path, *after* the process stopped producing. If this assertion ever
        // reads `Starting`, the catch-all has reclaimed the variant and a
        // diagnostic is manufacturing liveness again (D-19, D-33).
        app.apply_exec_event(ExecEvent::new(
            "quiet",
            ExecutionEvent::EventsDropped { count: 40 },
        ));

        assert_eq!(
            app.ctx.run_states.get("quiet"),
            Some(&RunState::Idle),
            "a drop report must not promote an idle alias to Starting"
        );
    }

    // ── OBS-06: counting the re-parses ───────────────────────────────────
    //
    // The seam is a plain `u64` on `AppContext`, bumped in `schedule_reparse`.
    // A `static AtomicUsize` inside `parse_project_state` would be more literal
    // and is deliberately REJECTED: cargo runs a crate's tests as threads
    // inside one process, so every other test that constructs project state
    // would increment the same static concurrently, and the failure would be
    // intermittent. Per-`App` state is the right scope — please do not
    // "improve" this seam back into a flaky one (RESEARCH §7.4).
    //
    // Deliberately NOT written: any wall-clock timing assertion. `main_loop.rs`
    // already records the reasoning for the same repository (its tests module,
    // "it passes on a developer laptop, fails on a loaded runner, gets
    // `#[ignore]`d within a month, and at that point the requirement has no
    // verification at all"). D-17 permits a generous comparative timing number
    // as corroboration, but the counter is the load-bearing evidence and a
    // timing number must never be the only evidence. Read this absence as a
    // choice.
    //
    // `#[tokio::test]` is required throughout: both schedulers call
    // `spawn_blocking`, which panics without a runtime.

    const OBS_ALIAS: &str = "proj";
    const OBS_RUN: &str = "2026-07-29T09-00-00Z-a3f9";

    /// The two liveness answers these tests build fixtures from.
    ///
    /// Named locally so the assertions below read as "a live run" and "a crashed
    /// run" rather than as a path repeated forty times, and so the third answer
    /// — `Liveness::Unknown`, which no `App`-level assertion here depends on — is
    /// conspicuously absent rather than silently unused.
    const ALIVE: crate::driver::liveness::Liveness = crate::driver::liveness::Liveness::Alive;
    const DEAD: crate::driver::liveness::Liveness = crate::driver::liveness::Liveness::Dead;

    /// One `App` with one registered project at `root`, and a live `event_tx`.
    ///
    /// Every OBS-06 test below builds through this one helper, so a handler
    /// change cannot satisfy the zero-arm by breaking the control arm.
    /// `App::new_for_test` leaves `event_tx` as `None`, and `schedule_reparse`
    /// returns early without a sender — so without this the counter would read
    /// zero for the wrong reason.
    fn obs_app(root: &std::path::Path) -> (App, tokio::sync::mpsc::UnboundedReceiver<Action>) {
        use crate::config::RegisteredProject;

        let mut app = App::new_for_test();
        app.ctx.config.projects.insert(
            OBS_ALIAS.to_string(),
            RegisteredProject {
                path: root.to_path_buf(),
                added: "2026-07-29".to_string(),
                driver_opt_in: None,
                extra: Default::default(),
            },
        );
        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        app.ctx.event_tx = Some(tx);
        (app, rx)
    }

    fn journal_change(root: &std::path::Path) -> Action {
        Action::FileChanged {
            project_path: root.to_path_buf(),
            changed_path: root.join(format!(
                ".planning/meta-manager/runs/{OBS_RUN}/journal.jsonl"
            )),
        }
    }

    fn planning_change(root: &std::path::Path) -> Action {
        Action::FileChanged {
            project_path: root.to_path_buf(),
            changed_path: root.join(".planning/STATE.md"),
        }
    }

    #[tokio::test]
    async fn journal_appends_never_trigger_a_full_reparse() {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = dir.path();
        let (mut app, _rx) = obs_app(root);

        let before = app.ctx.reparse_dispatches;
        for _ in 0..500 {
            app.update(journal_change(root));
        }

        assert_eq!(
            app.ctx.reparse_dispatches, before,
            "OBS-06: five hundred journal appends leaked {} full re-parses",
            app.ctx.reparse_dispatches - before
        );
    }

    // The control arm, and it is NOT optional. A test that only asserts zero
    // would pass against a handler that dropped `FileChanged` entirely — which
    // would break the dashboard outright while satisfying OBS-06 (RESEARCH
    // §7.3).
    #[tokio::test]
    async fn a_planning_write_still_triggers_a_reparse() {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = dir.path();
        let (mut app, _rx) = obs_app(root);

        let before = app.ctx.reparse_dispatches;
        app.update(planning_change(root));

        assert_eq!(
            app.ctx.reparse_dispatches,
            before + 1,
            "a planning write must still schedule exactly one re-parse, got {}",
            app.ctx.reparse_dispatches
        );
    }

    #[tokio::test]
    async fn the_driver_route_leaves_the_refresh_dedup_map_untouched() {
        let dir = tempfile::tempdir().expect("temp dir");
        let root = dir.path();
        let (mut app, _rx) = obs_app(root);

        for _ in 0..10 {
            app.update(journal_change(root));
        }
        assert!(
            app.ctx.last_refresh.is_empty(),
            "D-14: the driver route must not touch the 500ms dedup map, or a \
             journal append could suppress a genuine state refresh"
        );

        // And the planning route still records into it, so the assertion above
        // is not passing because the map went unused everywhere.
        app.update(planning_change(root));
        assert!(app.ctx.last_refresh.contains_key(OBS_ALIAS));
    }

    /// A `JournalCursor` with both of its fields varied, so an assertion cannot
    /// pass by comparing only the byte offset.
    fn cursor_at(offset: u64, last_seq: u64) -> crate::journal::reader::JournalCursor {
        crate::journal::reader::JournalCursor {
            cursor: crate::journal::reader::TailCursor { offset },
            last_seq,
        }
    }

    /// An `ObservedRun` with only the fields these assertions read varied.
    fn observed(
        alias: &str,
        run_id: &str,
        liveness: crate::driver::liveness::Liveness,
    ) -> crate::driver::reconcile::ObservedRun {
        crate::driver::reconcile::ObservedRun {
            alias: alias.to_string(),
            run_id: run_id.to_string(),
            pid: 4242,
            pgid: 4242,
            started_at: "2026-07-29T09:00:00Z".to_string(),
            goal: "ship it".to_string(),
            gsd_command: "/gsd-progress".to_string(),
            liveness,
        }
    }

    #[tokio::test]
    async fn a_reconciliation_result_replaces_the_observed_map_rather_than_merging_it() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());

        // A run the previous scan saw, whose project has since been
        // unregistered — or whose run has since ended. Either way the new scan
        // expresses that by NOT returning it, and a merge would keep it forever.
        app.ctx
            .observed_runs
            .insert("gone".to_string(), observed("gone", "run-old", ALIVE));

        app.update(Action::RunsReconciled {
            runs: vec![observed(OBS_ALIAS, "run-new", ALIVE)],
        });

        assert_eq!(
            app.ctx.observed_runs.len(),
            1,
            "the scan is authoritative: a stale entry must be dropped, not merged"
        );
        assert!(!app.ctx.observed_runs.contains_key("gone"));
        assert_eq!(app.ctx.observed_runs[OBS_ALIAS].run_id, "run-new");

        // An empty scan clears the map outright, which is how "every run ended"
        // is expressed.
        app.update(Action::RunsReconciled { runs: Vec::new() });
        assert!(app.ctx.observed_runs.is_empty());
    }

    #[tokio::test]
    async fn an_unchanged_reconciliation_result_does_not_request_a_redraw() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());

        app.update(Action::RunsReconciled {
            runs: vec![observed(OBS_ALIAS, "run-a", ALIVE)],
        });

        // The scan lands every ~5s for the whole life of the process. Without
        // the equality guard this would repaint twelve times a minute forever
        // with nothing on screen changed.
        app.needs_redraw = false;
        app.update(Action::RunsReconciled {
            runs: vec![observed(OBS_ALIAS, "run-a", ALIVE)],
        });
        assert!(
            !app.needs_redraw,
            "an identical scan result must not request a redraw"
        );

        // The control arm: a real change still does.
        app.update(Action::RunsReconciled {
            runs: vec![observed(OBS_ALIAS, "run-a", DEAD)],
        });
        assert!(
            app.needs_redraw,
            "a run going from live to crashed must request a redraw, or the guard \
             above is suppressing genuine updates too"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_driver_run_is_refused_when_the_live_count_already_meets_the_cap() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());

        assert_eq!(app.ctx.config.preferences.driver_max_concurrent, 1);
        app.ctx
            .observed_runs
            .insert("busy".to_string(), observed("busy", "run-live", ALIVE));

        app.start_driver_run(OBS_ALIAS, "/gsd-progress", None);

        let refusal = app
            .ctx
            .error_message
            .as_deref()
            .expect("the refusal must be visible to the user, not silent");
        assert!(
            refusal.contains("driver_max_concurrent"),
            "the refusal must name the setting that governs it, got: {refusal}"
        );
        assert!(
            !app.ctx.observed_runs.contains_key(OBS_ALIAS),
            "a refused spawn must add no optimistic entry"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_driver_run_against_an_unknown_alias_is_refused_visibly() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());

        app.start_driver_run("nosuchalias", "/gsd-progress", None);

        let refusal = app.ctx.error_message.as_deref().expect("a visible refusal");
        assert!(refusal.contains("nosuchalias"), "got: {refusal}");
        assert!(app.ctx.observed_runs.is_empty());
    }

    /// A crashed run does NOT count against the cap.
    ///
    /// It is still surfaced — a run that died without an ending is exactly what
    /// a user needs to be told about — but it consumes no quota, so counting it
    /// would leave a project permanently unstartable after one crash, with no
    /// way out but editing `config.json`.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_crashed_run_does_not_consume_a_concurrency_slot() {
        use crate::driver::spawn::admit;

        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());
        app.ctx
            .observed_runs
            .insert("dead".to_string(), observed("dead", "run-crashed", DEAD));

        let live = app
            .ctx
            .observed_runs
            .values()
            .filter(|run| run.is_live())
            .count();
        assert_eq!(live, 0);
        assert!(admit(live, app.ctx.config.preferences.driver_max_concurrent).is_ok());
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn stopping_an_alias_with_no_observed_run_is_refused_visibly() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());

        app.stop_driver_run(OBS_ALIAS);

        let refusal = app
            .ctx
            .error_message
            .as_deref()
            .expect("a stop with nothing to stop must say so, not fail silently");
        assert!(refusal.contains(OBS_ALIAS), "got: {refusal}");
    }

    /// A run the scan found but this session did not start takes D-07's
    /// **adopted** arm.
    ///
    /// The distinction cannot be read off disk — `run.json` records the driver's
    /// own pid and never who its parent was — so it lives in a set that survives
    /// the scan replacing `observed_runs` wholesale every five seconds. This
    /// asserts the set stays empty for a run that arrived through a scan, which
    /// is what makes the arm `Adopted` and the `/proc` re-probe the only way to
    /// confirm death.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_run_this_session_did_not_spawn_is_never_recorded_as_our_child() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());

        app.update(Action::RunsReconciled {
            runs: vec![observed(OBS_ALIAS, "run-adopted", ALIVE)],
        });

        assert!(
            app.ctx.observed_runs.contains_key(OBS_ALIAS),
            "the scan's own entry must be there, or the assertion below is vacuous"
        );
        assert!(
            app.ctx.session_spawned_runs.is_empty(),
            "a run rediscovered by the scan was reparented to init when its \
             original parent exited; claiming it as our child would make a stop \
             wait on a `wait()` that returns ECHILD (D-07)"
        );
    }

    /// The stop is dispatched off the render thread and its result comes back as
    /// an `Action`.
    ///
    /// The twelve-second grace is why this shape is not optional: a stop that
    /// blocked the loop would freeze the frame for twelve seconds while stopping
    /// a run, which looks exactly like the hang the stop exists to end
    /// (TRANS-03). The pid here belongs to no driver, so the dispatched task
    /// takes the `AlreadyGone` path and returns immediately — what is being
    /// asserted is the seam, not the teardown.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_stop_returns_its_outcome_as_an_action_rather_than_blocking() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, mut rx) = obs_app(dir.path());
        app.ctx
            .observed_runs
            .insert(OBS_ALIAS.to_string(), observed(OBS_ALIAS, "run-x", ALIVE));

        app.stop_driver_run(OBS_ALIAS);
        assert!(
            app.ctx.error_message.is_none(),
            "a stop against an observed run is not a refusal"
        );

        let action = tokio::time::timeout(std::time::Duration::from_secs(5), rx.recv())
            .await
            .expect("the stop must report back promptly, not block the loop")
            .expect("the channel is open");

        let Action::DriverStopped {
            alias,
            run_id,
            outcome,
        } = action
        else {
            panic!("the dispatched stop must report through DriverStopped, got {action:?}");
        };
        assert_eq!(alias, OBS_ALIAS);
        assert_eq!(run_id, "run-x");
        assert!(!outcome.is_empty(), "the outcome must render as something");

        // And the handler drops the entry rather than editing it, so the
        // dashboard does not show a stopped run as live until the next scan.
        app.update(Action::DriverStopped {
            alias: OBS_ALIAS.to_string(),
            run_id: "run-x".to_string(),
            outcome,
        });
        assert!(!app.ctx.observed_runs.contains_key(OBS_ALIAS));
        assert!(!app.ctx.session_spawned_runs.contains("run-x"));
    }

    #[tokio::test]
    async fn a_tail_result_advances_only_its_own_cursor() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());

        let mine = (OBS_ALIAS.to_string(), "run-a".to_string());
        let theirs = (OBS_ALIAS.to_string(), "run-b".to_string());
        app.ctx.journal_cursors.insert(mine.clone(), cursor_at(10, 1));
        app.ctx
            .journal_cursors
            .insert(theirs.clone(), cursor_at(20, 7));

        app.update(Action::DriverJournalAppended {
            alias: OBS_ALIAS.to_string(),
            run_id: "run-a".to_string(),
            records: Vec::new(),
            cursor: cursor_at(99, 4),
        });

        assert_eq!(
            app.ctx.journal_cursors.get(&mine),
            Some(&cursor_at(99, 4)),
            "the tail's own cursor must advance, seq and all"
        );
        assert_eq!(
            app.ctx.journal_cursors.get(&theirs),
            Some(&cursor_at(20, 7)),
            "a sibling run's cursor must not move"
        );
    }

    /// A run id whose lexicographic order equals its chronological order, which
    /// is `new_run_id`'s documented property and what the retention sort relies
    /// on. Built by hand rather than through `new_run_id` so the ordering under
    /// test is visible in the test itself.
    fn run_id_at(minute: u32) -> String {
        format!("2026-07-29T12-{minute:02}-00Z-a1b2")
    }

    /// D-27, first pass: an alias the registry no longer knows about.
    #[tokio::test]
    async fn pruning_drops_cursors_for_an_unregistered_alias() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());

        // OBS_ALIAS is registered by the fixture; "gone" never was.
        app.ctx
            .journal_cursors
            .insert((OBS_ALIAS.to_string(), run_id_at(1)), cursor_at(5, 1));
        app.ctx
            .journal_cursors
            .insert(("gone".to_string(), run_id_at(2)), cursor_at(6, 2));
        app.ctx
            .run_states
            .insert("gone".to_string(), crate::executor::RunState::Running);
        app.ctx
            .run_states
            .insert(OBS_ALIAS.to_string(), crate::executor::RunState::Running);
        app.ctx
            .observed_runs
            .insert("gone".to_string(), observed("gone", "run-gone", ALIVE));
        app.ctx.observed_runs.insert(
            OBS_ALIAS.to_string(),
            observed(OBS_ALIAS, "run-here", ALIVE),
        );

        app.prune_driver_maps();

        assert!(
            !app.ctx
                .journal_cursors
                .contains_key(&("gone".to_string(), run_id_at(2))),
            "a cursor for an unregistered alias is dead weight for the whole \
             life of the process — nothing can ever append to it again"
        );
        assert!(!app.ctx.run_states.contains_key("gone"));
        assert!(!app.ctx.observed_runs.contains_key("gone"));

        // The control arm: a registered alias keeps everything, so the
        // assertions above are not passing because the prune emptied the maps.
        assert!(app
            .ctx
            .journal_cursors
            .contains_key(&(OBS_ALIAS.to_string(), run_id_at(1))));
        assert!(app.ctx.run_states.contains_key(OBS_ALIAS));
        assert!(app.ctx.observed_runs.contains_key(OBS_ALIAS));
    }

    /// D-27, second pass: the per-alias retention bound.
    ///
    /// Asserts the count **and** which ids survived. A prune that kept the right
    /// number of the wrong runs — the oldest — would satisfy a count-only
    /// assertion while discarding exactly the cursors a live run needs.
    #[tokio::test]
    async fn pruning_retains_at_most_the_newest_runs_per_alias() {
        use crate::journal::RETAIN_RUNS;

        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());

        let total = RETAIN_RUNS + 5;
        for minute in 0..total {
            app.ctx.journal_cursors.insert(
                (OBS_ALIAS.to_string(), run_id_at(minute as u32)),
                cursor_at(minute as u64, minute as u64),
            );
        }
        assert_eq!(app.ctx.journal_cursors.len(), total);

        app.prune_driver_maps();

        assert_eq!(
            app.ctx.journal_cursors.len(),
            RETAIN_RUNS,
            "the in-memory bound must match the on-disk one rather than being a \
             second unrelated number"
        );

        let mut survivors: Vec<String> = app
            .ctx
            .journal_cursors
            .keys()
            .map(|(_, run_id)| run_id.clone())
            .collect();
        survivors.sort();
        let expected: Vec<String> = (total - RETAIN_RUNS..total)
            .map(|minute| run_id_at(minute as u32))
            .collect();
        assert_eq!(
            survivors, expected,
            "the NEWEST ids must survive; run ids sort lexicographically in \
             chronological order by construction, so this is a string sort and \
             needs no file read"
        );

        // Under the bound, nothing is evicted.
        app.ctx.journal_cursors.clear();
        for minute in 0..RETAIN_RUNS {
            app.ctx.journal_cursors.insert(
                (OBS_ALIAS.to_string(), run_id_at(minute as u32)),
                cursor_at(0, 0),
            );
        }
        app.prune_driver_maps();
        assert_eq!(app.ctx.journal_cursors.len(), RETAIN_RUNS);
    }

    /// The interactive removal path cleans the driver maps immediately, rather
    /// than leaving them to the periodic backstop.
    #[tokio::test]
    async fn removing_a_project_interactively_clears_its_driver_maps() {
        use crate::ui::screens::delete_confirm::DeleteConfirmScreen;
        use crate::ui::screens::Screen;
        use crossterm::event::{KeyCode, KeyModifiers};

        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());
        // `save_config` must land somewhere real, or the removal reports a save
        // failure and returns before it reaches the cleanup block.
        app.ctx.config_path = dir.path().join("config.json");

        app.ctx
            .journal_cursors
            .insert((OBS_ALIAS.to_string(), run_id_at(1)), cursor_at(5, 1));
        app.ctx
            .journal_cursors
            .insert((OBS_ALIAS.to_string(), run_id_at(2)), cursor_at(9, 3));
        app.ctx
            .run_states
            .insert(OBS_ALIAS.to_string(), crate::executor::RunState::Running);
        // `DEAD`, and the choice is load-bearing since plan 17-08: the removal
        // path now REFUSES a project whose observed run is not known to be
        // finished (CR-06), so a live run here would make this test assert the
        // refusal rather than the D-27 cleanup it is about. A crashed run is
        // nothing left to abandon, so it is removable — and it still exercises
        // the sibling-map cleanup, which is this test's actual subject.
        app.ctx
            .observed_runs
            .insert(OBS_ALIAS.to_string(), observed(OBS_ALIAS, "run-here", DEAD));

        let mut screen = DeleteConfirmScreen::new(OBS_ALIAS.to_string());
        screen.handle_key(KeyCode::Char('y'), KeyModifiers::NONE, &mut app.ctx);

        assert!(
            !app.ctx.config.projects.contains_key(OBS_ALIAS),
            "the removal itself must have happened, or the rest is vacuous"
        );
        assert!(
            app.ctx.error_message.is_none(),
            "unexpected failure: {:?}",
            app.ctx.error_message
        );
        assert!(
            !app.ctx.run_states.contains_key(OBS_ALIAS),
            "run_states leaked past an interactive removal"
        );
        assert!(
            !app.ctx.observed_runs.contains_key(OBS_ALIAS),
            "observed_runs leaked past an interactive removal"
        );
        assert!(
            app.ctx
                .journal_cursors
                .keys()
                .all(|(alias, _)| alias != OBS_ALIAS),
            "every cursor for the removed alias must go, not just one — the map \
             is keyed (alias, run_id) and a project may have several"
        );
    }
}
