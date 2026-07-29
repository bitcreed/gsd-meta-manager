pub mod add_project;
pub mod create_project;
pub mod delete_confirm;
pub mod detail;
pub mod enqueue;
pub mod help;
pub mod normal;
pub mod queue_delete_confirm;

use crate::action::Action;
use crate::app::DetailSubView;
use crate::change_tracker::ChangeTracker;
use crate::config::Config;
use crate::executor::RunState;
use crate::main_loop::ExecEvent;
use crate::state_reader::backlog::BacklogItem;
use crate::state_reader::git_ops::{GitDiffStat, GitLogEntry};
use crate::state_reader::ProjectState;
use crate::watcher::FileWatcher;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::widgets::TableState;
use ratatui::Frame;
use std::collections::HashMap;
use std::path::PathBuf;
use tokio::sync::mpsc::{Sender, UnboundedSender};

pub trait Screen {
    fn handle_key(
        &mut self,
        code: KeyCode,
        modifiers: KeyModifiers,
        ctx: &mut AppContext,
    ) -> ScreenAction;
    fn render(&self, frame: &mut Frame, area: Rect, ctx: &AppContext);
    fn name(&self) -> &str;
}

pub enum ScreenAction {
    None,
    Push(Box<dyn Screen>),
    Pop,
    Quit,
    SetStatusMessage(String),
    /// Suspend the TUI and open a file in $EDITOR.
    SuspendAndEdit(std::path::PathBuf),
    /// Used by archive browser (Phase 12) to dispatch async load actions.
    #[allow(dead_code)]
    DispatchAction(Box<Action>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DefaultsEditTarget {
    #[default]
    Project,
    Global,
}

#[derive(Default)]
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
    pub archive_depth: crate::archive::ArchiveDepth,
    pub archive_milestones: Vec<String>,
    pub archive_selected: [usize; 4],
    pub archive_scroll_offset: u16,
    pub archive_loading: bool,
    pub archive_file_content: Option<String>,
    pub archive_file_name: Option<String>,
    pub defaults_config: Option<crate::state_reader::config_json::GsdConfig>,
    /// Parsed contents of ~/.gsd/defaults.json — layered under
    /// `defaults_config` for display and editable via the [d] toggle.
    pub defaults_user_config: Option<crate::state_reader::config_json::GsdConfig>,
    /// Which file the Defaults tab is currently editing.
    pub defaults_edit_target: DefaultsEditTarget,
    pub defaults_selected: usize,
    /// When `Some(entry_idx)`, a dropdown OR text input is open for that entry.
    /// The render code decides which UI to show based on the entry's kind.
    pub defaults_editing: Option<usize>,
    /// Cursor position inside the open dropdown (only used for Bool/Enum).
    pub defaults_dropdown_selected: usize,
    /// In-progress text the user is typing for a String-kind entry.
    pub defaults_text_buffer: String,
    // ── Docs browser tab state ────────────────────────────────────────
    pub browser_depth: crate::browser::BrowserDepth,
    /// `None` until the user first activates the Docs tab; then set to the
    /// initial active-phase dir (or `.planning/` root if milestone complete).
    pub browser_current_dir: Option<PathBuf>,
    /// Root directory of the browse session (the project's `.planning/`).
    /// Used as the boundary for "navigate up" — Backspace at this dir is a no-op.
    pub browser_root: Option<PathBuf>,
    /// Entry directory the browser opened on (active phase or root). `g` jumps
    /// to `browser_root`; `p` jumps back here.
    pub browser_entry_dir: Option<PathBuf>,
    pub browser_entries: Vec<crate::browser::BrowserEntry>,
    pub browser_selected: usize,
    pub browser_scroll_offset: u16,
    pub browser_file_content: Option<String>,
    pub browser_file_name: Option<String>,
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
    /// Long-lived sender for the **separate, bounded** executor channel (D-17).
    ///
    /// Two things about this field are load-bearing, and neither is stylistic:
    ///
    /// * It is **separate** from `event_tx`. High-volume stream traffic must
    ///   never travel through the `Action` FIFO, because `pump`'s biased
    ///   `select!` polls that FIFO first and would starve everything else.
    /// * It is a clone held for the **process lifetime**. The executor channel
    ///   legitimately has no producer between runs, and a channel with no live
    ///   sender closes — which permanently disables its `select!` arm
    ///   (Pitfall C). Keeping this clone alive is what stops that.
    pub exec_tx: Option<Sender<ExecEvent>>,
    /// Per-alias live driver state (D-19).
    ///
    /// A **sibling map**, shaped exactly like `last_refresh` and
    /// `archive_cache` above. Driver state deliberately does NOT live on
    /// `ProjectState`: that type derives `PartialEq`, and `app.rs` uses the
    /// derived equality to suppress the "Updated: {alias}" status message.
    /// Driver state changes every few seconds — one 68-second spike turn
    /// emitted 22 `thinking_tokens` events — so putting it there would flood
    /// the status bar for an entire multi-hour run.
    pub run_states: HashMap<String, RunState>,
    /// Count of full `parse_project_state` dispatches this process has issued.
    ///
    /// * **Load-bearing for OBS-06.** A driver journal append must never
    ///   increment it. `App::schedule_reparse` is the only writer, which is
    ///   what makes the assertion "zero re-parses" mean anything.
    /// * It is deliberately **not** `#[cfg(test)]`-gated, for two reasons: a
    ///   gated counter means the test exercises a different binary than
    ///   production, and the count is a legitimate diagnostic in its own right
    ///   that Phase 18's driver surface may want to render.
    pub reparse_dispatches: u64,
    /// Byte offset into each run journal, keyed by `(alias, run_id)` (D-13).
    ///
    /// * A **sibling map**, shaped exactly like `run_states` above and
    ///   `last_refresh` / `archive_cache` below. Phase 16 extends that
    ///   neighbourhood rather than recreating it (D-19).
    /// * It must not live on `ProjectState`: that type derives `PartialEq` and
    ///   `app.rs` uses the derived equality to suppress the "Updated: {alias}"
    ///   status message. A journal offset moves every few seconds, so a field
    ///   there would flood the status bar for an entire multi-hour run (D-18).
    /// * It holds offsets, run ids and counts — **never a file handle or a
    ///   join handle**. `Action` derives `Clone` and a handle is not `Clone`
    ///   (D-20).
    pub journal_cursors: HashMap<(String, String), crate::journal::reader::TailCursor>,
    /// Per-alias observed driver run, from the reconciliation scan (D-13, D-25).
    ///
    /// * A **sibling map**, shaped exactly like `run_states` and
    ///   `journal_cursors` above. Phase 17 extends that neighbourhood rather
    ///   than recreating it.
    /// * It must not live on `ProjectState`: that type derives `PartialEq` and
    ///   `app.rs` uses the derived equality to suppress the "Updated: {alias}"
    ///   status message. A driver's observed state moves every few seconds, so a
    ///   field there would flood the status bar for an entire multi-hour run —
    ///   defeating a deliberate v1.4 feature for the whole duration of the thing
    ///   it is meant to report (D-25, ARCHITECTURE AP1).
    /// * It holds ids, counts and strings — **never a file handle or a join
    ///   handle**. `Action` derives `Clone` and a handle is not `Clone` (D-20).
    /// * An entry is **"observed", not "reattached" and not "streaming"** (D-11).
    ///   Once the TUI has exited, the driver's stdout pipe is gone and live
    ///   re-streaming is physically impossible; what this map carries is a
    ///   journal-tail handle and a pgid to signal.
    pub observed_runs: HashMap<String, crate::driver::reconcile::ObservedRun>,
    pub watcher: Option<FileWatcher>,
    pub last_refresh: HashMap<String, std::time::Instant>,
    pub detail_scroll_offset: u16,
    pub suggestion_index: usize,
    pub input_buffer: String,
    pub needs_redraw: bool,
    pub active_sessions: Vec<crate::session_detector::ClaudeSession>,
    pub archive_cache: HashMap<String, crate::archive::MilestoneArchive>,
}

impl AppContext {
    /// Get sorted project aliases for consistent ordering in the table.
    pub fn sorted_aliases(&self) -> Vec<String> {
        let mut aliases: Vec<String> = self.config.projects.keys().cloned().collect();
        aliases.sort_by_key(|a| a.to_lowercase());
        aliases
    }

    /// Get the alias of the currently selected project, if any.
    pub fn selected_alias(&self) -> Option<String> {
        self.table_state
            .selected()
            .and_then(|i| self.filtered_aliases.get(i).cloned())
    }

    pub fn recompute_filtered_aliases(&mut self) {
        use crate::app::{format_phase_display, parse_filter, FilterColumn};

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
                    FilterColumn::Phase => state.is_some_and(|s| {
                        format_phase_display(s).to_lowercase().contains(&term_lower)
                    }),
                    FilterColumn::Status => {
                        state.is_some_and(|s| s.status.to_lowercase().contains(&term_lower))
                    }
                    FilterColumn::All => {
                        alias.to_lowercase().contains(&term_lower)
                            || state.is_some_and(|s| {
                                s.status.to_lowercase().contains(&term_lower)
                                    || format_phase_display(s).to_lowercase().contains(&term_lower)
                            })
                    }
                }
            })
            .collect();
    }
}
