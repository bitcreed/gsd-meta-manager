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
    /// The Driver tab: run list, run detail, live output (D-15, OBS-04).
    ///
    /// **Index 10, and it is deliberately not yet reachable.** This plan adds
    /// the variant and the two arms `detail.rs` needs to compile; the tab title
    /// vector, the `Shift+D` binding, the footer hints and the rendering are
    /// plan 18-09's. Until those land, `TAB_TITLES` still has ten entries, so
    /// `Right` stops at index 9 and nothing selects this — read the gap as a
    /// staged landing rather than as a tab that is half-wired by accident.
    Driver,
}

#[derive(Debug, Clone, PartialEq)]
pub enum FilterColumn {
    All,
    Name,
    Phase,
    Status,
    /// Rows satisfying [`crate::ui::screens::needs_human`] (OBS-07, D-25).
    ///
    /// Unlike the three column filters this one is a **predicate**, not a
    /// column: the term still matches across every column and this narrows what
    /// survives. `/h` alone (an empty term) is therefore "every project waiting
    /// on a human" with no special case in the grammar.
    NeedsHuman,
}

#[derive(Debug, Clone, PartialEq)]
pub enum StatusCategory {
    Active,
    Idle,
    Blocked,
    Complete,
    Unknown,
}

/// Split a filter string into its term and the column or predicate it narrows.
///
/// The suffix grammar is `term/x`. `h` joins `n`, `p` and `s` for "needs a
/// human" (OBS-07): the other three letters were taken and `h` is both free and
/// mnemonic. On screen the search prompt supplies its own leading `/`, so the
/// all-rows form renders as `//h` while the stored `filter_text` is `/h`.
pub fn parse_filter(input: &str) -> (String, FilterColumn) {
    if let Some(term) = input.strip_suffix("/p") {
        (term.to_string(), FilterColumn::Phase)
    } else if let Some(term) = input.strip_suffix("/n") {
        (term.to_string(), FilterColumn::Name)
    } else if let Some(term) = input.strip_suffix("/s") {
        (term.to_string(), FilterColumn::Status)
    } else if let Some(term) = input.strip_suffix("/h") {
        (term.to_string(), FilterColumn::NeedsHuman)
    } else {
        (input.to_string(), FilterColumn::All)
    }
}

/// Map a goal buffer to the `Option<&str>` `App::start_driver_run` takes, so an
/// empty one arrives as `None` and never as `Some("")` (D-23, OBS-03).
///
/// **The trap is live rather than theoretical.** The start flow's goal field is
/// optional and Step B accepts an empty buffer on `Enter`, so the common case is
/// a user who typed nothing. `drive_argv` omits `--goal` entirely for `None`,
/// but for `Some("")` it pushes the flag with an empty operand — and the driver
/// then records an empty goal into `RunRecord.goal` **as though one had been
/// given**. Those are different facts, and the display distinguishes them: no
/// goal renders `(none given)`, while a recorded empty goal renders as a blank
/// line that looks like a goal the reader simply cannot see.
///
/// Emptiness is tested on the **trimmed** text while the value returned is the
/// **untrimmed** original. A goal of three spaces is no goal; a goal that was
/// given is stored verbatim and never paraphrased, so nothing here rewrites what
/// the user typed (FEATURES table stakes, D-23).
///
/// Pure, which is what makes the branch that matters testable without spawning a
/// driver (S6).
pub fn goal_or_none(goal: Option<&str>) -> Option<&str> {
    goal.filter(|text| !text.trim().is_empty())
}

/// Project one journal record into a live-output line, or `None` if it belongs
/// somewhere other than the output pane (D-20, OBS-04).
///
/// The five classes are the UI-SPEC's event-kind rendering table, and the
/// mapping is on `record.kind` — a plain `String`, because the reader
/// deliberately does not go through the typed `JournalEvent`, so a kind this
/// build has never seen still arrives with its payload intact (D-30).
///
/// **What is deliberately not buffered, and why that is not "losing lines".**
/// `run_started`, `exec_started` and `cost` return `None`: they are the run
/// header's data — the goal, the command, the started time, the cumulative cost
/// — and they are rendered there rather than in the scrolling pane. Putting them
/// in both would make the pane's first rows a duplicate of the header above it.
/// The reserved kinds (`observed`, `decided`, `parked`) return `None` too, for a
/// different reason: nothing emits them before Phase 20, and their surface is
/// that phase's step timeline rather than this buffer.
///
/// **Every kind that can carry a diagnostic IS buffered**, because a pane that
/// silently loses a `journal_truncated` or an `events_dropped` is exactly the
/// "looks done but isn't" failure this phase enumerates by name.
///
/// **The text is composed, never `Debug`-rendered**, and it is not sanitised
/// here: `DriverOutput::push_record` is the one append-time gate every string
/// from disk passes through, and stripping twice would be two places to keep in
/// agreement.
///
/// Pure, which is what makes the whole table assertable without a terminal (S6).
pub fn driver_line_for_record(
    record: &crate::journal::reader::JournalRecord,
) -> Option<(crate::ui::screens::DriverLineKind, String)> {
    use crate::ui::screens::DriverLineKind;

    let text = |key: &str| -> String {
        record
            .rest
            .get(key)
            .and_then(|value| value.as_str())
            .unwrap_or_default()
            .to_string()
    };
    let number = |key: &str| -> u64 {
        record
            .rest
            .get(key)
            .and_then(|value| value.as_u64())
            .unwrap_or_default()
    };

    match record.kind.as_str() {
        "exec_event" => {
            // `stderr` is present but secondary; every other stream — assistant,
            // user, turn_completed, unknown, unparseable — is the common case
            // and gets no marker, so injections and diagnostics stand out.
            let kind = if text("stream") == "stderr" {
                DriverLineKind::Stderr
            } else {
                DriverLineKind::Output
            };
            Some((kind, text("text")))
        }
        // The human's own words, echoed back into the stream they were steering.
        "interjected" => Some((DriverLineKind::Injection, text("text"))),
        // These two carry an id rather than the text — the driver correlates,
        // the TUI does not (D-08) — so the line names the transition and the
        // renderer pairs it with the message body it already holds.
        "interjection_acted_on" => Some((
            DriverLineKind::Injection,
            format!("interjection acted on: {}", text("id")),
        )),
        "interjection_missed" => Some((
            DriverLineKind::Injection,
            format!(
                "interjection missed: {} ({})",
                text("id"),
                text("reason")
            ),
        )),
        "diagnostic" => Some((
            DriverLineKind::Diagnostic,
            format!("{}: {}", text("code"), text("detail")),
        )),
        "events_dropped" => Some((
            DriverLineKind::Diagnostic,
            format!("{} stream events were dropped", number("count")),
        )),
        "journal_truncated" => Some((
            DriverLineKind::Diagnostic,
            format!(
                "journal truncated at {} bytes (cap {})",
                number("bytes_written"),
                number("cap")
            ),
        )),
        // The visual full stop. `RunEnded.outcome` is the four-source
        // derivation's answer and never the agent's prose (D-13).
        "run_ended" => Some((
            DriverLineKind::Terminal,
            format!("run ended: {}", text("outcome")),
        )),
        "exec_finished" => {
            let exit = record
                .rest
                .get("exit")
                .and_then(|value| value.as_i64())
                .map_or_else(|| "no exit status".to_string(), |code| format!("exit {code}"));
            Some((
                DriverLineKind::Terminal,
                format!("agent finished: {exit} after {}s", number("duration_s")),
            ))
        }
        _ => None,
    }
}

/// Fold one batch of journal records into the Driver tab's running tally.
///
/// The two facts here are the ones the run header and the step timeline need
/// that the committed `run.json` does not carry — the cumulative cost and the
/// number of turn boundaries — so they are read from the records as they arrive
/// rather than by re-parsing a journal at render time.
///
/// **The tally is reset when the run id changes**, which is what stops one run's
/// cost being displayed against another's. Both facts are evidence: `cost`
/// records carry `cumulative_usd`, and a turn boundary is an `exec_event` whose
/// stream is `turn_completed`. Neither is inferred from the agent's prose (D-13).
///
/// Pure, so the whole fold is assertable without a terminal (S6).
pub fn update_driver_tally(
    cache: &mut crate::ui::screens::ProjectViewCache,
    run_id: &str,
    records: &[crate::journal::reader::JournalRecord],
) {
    use crate::ui::screens::DriverRunTally;

    let tally = match cache.driver_tally.as_mut() {
        Some(tally) if tally.run_id == run_id => tally,
        _ => {
            cache.driver_tally = Some(DriverRunTally {
                run_id: run_id.to_string(),
                ..DriverRunTally::default()
            });
            cache
                .driver_tally
                .as_mut()
                .expect("just assigned")
        }
    };

    for record in records {
        match record.kind.as_str() {
            "cost" => {
                if let Some(usd) = record
                    .rest
                    .get("cumulative_usd")
                    .and_then(|value| value.as_f64())
                {
                    tally.cumulative_cost_usd = Some(usd);
                }
            }
            "exec_event" => {
                let is_boundary = record
                    .rest
                    .get("stream")
                    .and_then(|value| value.as_str())
                    .is_some_and(|stream| stream == "turn_completed");
                if is_boundary {
                    tally.turn_boundaries = tally.turn_boundaries.saturating_add(1);
                }
            }
            _ => {}
        }
    }
}

/// The exact copy for an observed journal sequence gap (Copywriting Contract).
///
/// A gap means records the writer emitted were never read — the pane is missing
/// lines and cannot recover them. **Surfacing it is the point.** Before this it
/// reached a `tracing::warn!` and nothing else, i.e. a log file the user of a
/// TUI never opens, while the pane presented itself as complete.
pub fn journal_gap_line(gaps: usize) -> String {
    format!("journal gap: {gaps} record(s) not read")
}

/// Whether this tick should repaint for the elapsed-time counter (D-21).
///
/// **All three conditions are required, and the gate is the whole point.**
/// Elapsed time is `now − started_at`, so it changes every tick forever; an
/// unconditional per-tick redraw would make an idle fleet dashboard burn CPU for
/// the life of the process, which is the exact opposite of this tool's pitch.
/// The counter is only *visible* when a detail view is on top, its active
/// sub-view is the Driver tab, and the selected project actually has a live run
/// — a finished run's elapsed figure is frozen and needs no frame at all.
///
/// **No second timer.** This rides the existing 250 ms `Action::Tick`, which
/// `App::update`'s 20-tick block forbids duplicating in as many words: two
/// timers at slightly different phases would make "how stale can the dashboard
/// be?" a question with two answers.
///
/// Pure, so the truth table is assertable without a terminal (S6).
pub fn driver_elapsed_redraw_wanted(
    top_screen_name: &str,
    sub_view: Option<&DetailSubView>,
    run: Option<&crate::driver::reconcile::ObservedRun>,
) -> bool {
    top_screen_name == crate::ui::screens::detail::DetailScreen::NAME
        && matches!(sub_view, Some(DetailSubView::Driver))
        && run.is_some_and(|run| run.is_live())
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
            driver_output: HashMap::new(),
            sort_mode: crate::ui::screens::SortMode::default(),
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
        // No tail is scheduled for a run id that is not a plain path component
        // (D-27, WR-02). The id reaches here from the driven project's own
        // `active` file, so it is attacker-controlled for threat-modelling
        // purposes, and without this guard a traversing id would have the TUI
        // tailing an arbitrary file on the user's disk into a render surface.
        // The refusal is logged by kind only — never the path, which is the
        // untrusted value itself (D-28, PATTERNS §S3).
        let Some(paths) = crate::journal::run_paths(&planning_dir, run_id) else {
            tracing::warn!(
                alias = %alias,
                kind = "run_id_not_a_plain_component",
                "journal tail refused: the run id does not name a single directory component",
            );
            return;
        };
        let journal = paths.journal;

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

    // ── The Driver surface's three schedulers (D-28) ──────────────────
    //
    // All three are `pub` for the reason `start_driver_run` and
    // `stop_driver_run` are: they are the seams the UI layer reaches, and a
    // `Screen` only ever receives `&mut AppContext`, so the route from a key to
    // one of these is an `Action` this file dispatches. Two of them — the run
    // list and the dry-run preview — have no in-file caller yet because their
    // callers are the Driver tab's own key handling, which is a later plan's.
    //
    // All three follow the shipped `spawn_blocking` → `Action` shape: a
    // `let … else` guard on `event_tx`, a cloned sender, owned data moved in.
    // **No file I/O on the render thread** — that phrase is the searchable
    // marker this file already uses for the boundary (S1).

    /// Gather the three facts [`driver_elapsed_redraw_wanted`] decides on.
    ///
    /// The alias comes from `ctx.selected_alias()` rather than from the screen,
    /// because the stack holds `Box<dyn Screen>` and cannot hand back a
    /// `DetailScreen`'s field. That is not a workaround: `DetailScreen::new` is
    /// constructed from exactly this value at `normal.rs:279`, and nothing moves
    /// the dashboard's selection while a detail view is on top of the stack, so
    /// the two are the same alias by construction.
    fn driver_elapsed_redraw_due(&self) -> bool {
        let Some(top) = self.screen_stack.last() else {
            return false;
        };
        let Some(alias) = self.ctx.selected_alias() else {
            return false;
        };
        driver_elapsed_redraw_wanted(
            top.name(),
            self.ctx.detail_sub_view_per_project.get(&alias),
            self.ctx.observed_runs.get(&alias),
        )
    }

    /// Resolve one alias's `.planning/` directory, or refuse visibly (S4).
    ///
    /// The schedulers below begin with this lookup, and they all refuse the
    /// same way: a message in `ctx.error_message` and a redraw, never a bare
    /// `return`. Sharing it is what stops the refusals drifting into several
    /// different wordings for one condition.
    fn planning_dir_for(&mut self, alias: &str) -> Option<PathBuf> {
        match self.ctx.config.projects.get(alias) {
            Some(project) => Some(project.path.join(".planning")),
            None => {
                self.ctx.error_message = Some(format!("No registered project named '{alias}'"));
                self.needs_redraw = true;
                None
            }
        }
    }

    /// Schedule the durable append of one injected message (STEER-01, STEER-03).
    ///
    /// **The key handler never touches the filesystem, and that is the whole
    /// shape of this function.** `journal::inbox::append` pays a `sync_data()`
    /// — deliberately, because STEER-03's criterion is that the bytes were on
    /// disk before any reader existed — and a `sync_data()` on the render thread
    /// is the WR-10 failure mode whose symptom is a frozen frame rather than an
    /// error (D-06, D-28). So the write runs on `spawn_blocking` with its result
    /// returned as an `Action` on a cloned sender, following the idiom this file
    /// already uses for the re-parse, for session detection and for the journal
    /// tail. **No file I/O on the render thread.**
    ///
    /// `id` is the caller's, minted at queue time before any other process saw
    /// the line, which is what makes the `queued` state addressable at all
    /// (D-05). It is **not** re-minted here: `InboxMessage::new` would generate
    /// a fresh one, and the message the UI is tracking would then have an id no
    /// journal record ever references.
    ///
    /// The status the UI shows is set by the `DriverInjectWritten` handler and
    /// never here — nothing may read `queued` before the write returns `Ok`.
    pub fn schedule_inbox_append(
        &mut self,
        alias: &str,
        run_id: &str,
        id: String,
        text: String,
    ) {
        let Some(planning_dir) = self.planning_dir_for(alias) else {
            return;
        };

        // The same fallible join `schedule_journal_tail` uses, and for the same
        // reason: the run id reaches the TUI from the driven project's own
        // `active` file, so it is attacker-controlled for threat-modelling
        // purposes (D-27, WR-02). Without this a traversing id would have the
        // TUI appending an agent-authored line to an arbitrary file on the
        // user's disk. Logged by kind only — never the path, which IS the
        // untrusted value (S3).
        let Some(paths) = crate::journal::run_paths(&planning_dir, run_id) else {
            tracing::warn!(
                alias = %alias,
                kind = "run_id_not_a_plain_component",
                "inbox append refused: the run id does not name a single directory component",
            );
            self.ctx.error_message = Some(
                "Could not queue message: that run id does not name a single directory \
                 component. Nothing was written — try again."
                    .to_string(),
            );
            self.needs_redraw = true;
            return;
        };

        let Some(tx) = &self.ctx.event_tx else {
            self.ctx.error_message = Some(
                "Could not queue message: this session has no event channel. Nothing was \
                 written — try again."
                    .to_string(),
            );
            self.needs_redraw = true;
            return;
        };
        let tx = tx.clone();
        let inbox = paths.inbox;
        let alias_for_task = alias.to_string();
        let run_id_for_task = run_id.to_string();

        tokio::task::spawn_blocking(move || {
            // Built by hand rather than through `InboxMessage::new` so the
            // caller's id survives; `cap_chars` is applied explicitly because
            // that is the half of `new` this path still needs.
            let message = crate::journal::inbox::InboxMessage {
                id: id.clone(),
                ts: chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true),
                text: crate::journal::inbox::cap_chars(
                    &text,
                    crate::journal::inbox::MAX_INBOX_TEXT_CHARS,
                ),
            };

            // The error **kind** and nothing else. An `io::Error`'s `Display`
            // can carry an OS path, and this path's operand is a run directory
            // under a project the agent writes to (S3, D-28).
            let error = crate::journal::inbox::append(&inbox, &message)
                .err()
                .map(|e| format!("{:?}", e.kind()));
            if let Some(kind) = &error {
                tracing::warn!(
                    alias = %alias_for_task,
                    run_id = %run_id_for_task,
                    kind = %kind,
                    "inbox append failed",
                );
            }

            let _ = tx.send(Action::DriverInjectWritten {
                alias: alias_for_task,
                run_id: run_id_for_task,
                id,
                error,
            });
        });
    }

    /// Schedule the run-list directory scan and the selected run's inbox read
    /// (OBS-05, STEER-02).
    ///
    /// Both halves are blocking filesystem work — a `read_dir` plus one small
    /// `run.json` per run, then a byte-offset tail — and **this is one of the
    /// phase's own new blocking paths**. Wrapping only the calls the Phase 17
    /// review listed while leaving the new ones on the render thread is the
    /// named half-fix (D-28), so this follows the same `spawn_blocking` →
    /// `Action` idiom. **No file I/O on the render thread.**
    ///
    /// The two reads share one task rather than taking one each, because the
    /// second depends on the first: which run's inbox to read is decided by
    /// indexing the freshly-listed runs. Splitting them would mean either a
    /// round trip through the event loop between them or a stale index.
    ///
    /// The inbox is read from offset zero every time, and that is deliberate:
    /// the payload is the whole inbox rather than a delta, so a message removed
    /// from the file is expressed by its absence — the same reason
    /// [`Action::RunsReconciled`] carries the whole scan.
    ///
    /// **The body lives on [`AppContext`] since plan 18-09**, and this is a
    /// delegation rather than a second implementation. The Driver tab's
    /// `switch_to_tab` arm has to schedule the same scan on first visit, and a
    /// `Screen` is handed an `&mut AppContext` and never an `&mut App` — so the
    /// choice was one function reachable from both or two copies of a
    /// `spawn_blocking` closure that would drift. `ctx.needs_redraw` is synced
    /// into `App::needs_redraw` by the main loop, so the refusal path is
    /// unchanged in effect.
    pub fn schedule_run_list_scan(&mut self, alias: &str, project_path: &Path) {
        self.ctx.schedule_run_list_scan(alias, project_path);
    }

    /// Schedule the dry-run preview for `alias` and `command` (D-26).
    ///
    /// **The body lives on [`AppContext`] since plan 18-11**, and this is a
    /// delegation rather than a second implementation, for the reason
    /// [`Self::schedule_run_list_scan`] is one: `DriverStartScreen` has to
    /// schedule the same build at Step B, and a `Screen` is handed an
    /// `&mut AppContext` and never an `&mut App` — so the choice was one
    /// function reachable from both or two copies of a `spawn_blocking` closure
    /// that would drift. `ctx.needs_redraw` is synced into `App::needs_redraw`
    /// by the main loop, so the refusal paths are unchanged in effect.
    #[cfg(unix)]
    pub fn schedule_dry_run_report(&mut self, alias: &str, command: &str) {
        self.ctx.schedule_dry_run_report(alias, command);
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

                // The elapsed-time counter (D-21). It rides THIS tick and gets
                // no interval of its own, for the same reason the reconciliation
                // probe below does not — see the comment on that block, which
                // forbids a second timer in as many words.
                //
                // **Gated, and the gate is not an optimisation.** Elapsed time
                // is `now − started_at`, so it differs every tick forever; an
                // unconditional redraw here would repaint an idle fleet
                // dashboard four times a second for the life of the process.
                // The predicate is a pure function so its truth table is
                // assertable without a terminal.
                if self.driver_elapsed_redraw_due() {
                    self.needs_redraw = true;
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
            // One tail read landed. **This is the seam Phase 16 left open, and
            // this handler is what closes it** (D-20).
            //
            // Until now it stored its cursor, counted `seq` gaps into a
            // `tracing::warn!`, and dropped `records` on the floor — deliberately,
            // because there was no surface to render them onto and a redraw would
            // have scheduled a frame that could not differ from the one on
            // screen. The comment here said so, and named Phase 18 as what adds
            // the surface and the flag together. It does, so that comment is
            // gone rather than left standing above code it no longer describes.
            //
            // What it does now:
            //
            // * projects each record into a `DriverLineKind` and pushes it into
            //   the alias's bounded ring buffer, where `push_record` sanitises
            //   it and enforces the cap;
            // * turns an observed sequence gap into a **visible** diagnostic
            //   line as well as a log line, because a pane that silently loses
            //   records is the named "looks done but isn't" failure; and
            // * sets `needs_redraw`, because the frame genuinely can differ now.
            //
            // What it still deliberately does **not** do, and the omissions are
            // as load-bearing as they were before:
            //
            // * It does not touch `ProjectState` (D-18). That type derives
            //   `PartialEq` and `app.rs` uses the derived equality to suppress
            //   "Updated: {alias}" status spam; live output arrives every few
            //   seconds, so a field there would flood the status bar for an
            //   entire multi-hour run.
            // * It does not touch `last_refresh` (D-14). Sharing the 500 ms
            //   dedup map would let a journal append suppress a genuine
            //   `STATE.md` re-parse, trading a performance bug for a correctness
            //   one.
            // * It does not schedule a re-parse (OBS-06).
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

                // The buffer for this alias, created on first append. The map is
                // pruned in `prune_driver_maps` — see the negative
                // carry-forward the phase discharges there.
                let output = self.ctx.driver_output.entry(key.0.clone()).or_default();

                // The gap goes in FIRST and as a line the user can see. The
                // records that follow are the ones that arrived; the gap is what
                // came before them and never will, so it belongs above them.
                if gaps > 0 {
                    output.push_record(
                        crate::ui::screens::DriverLineKind::Diagnostic,
                        &journal_gap_line(gaps),
                    );
                }

                for record in &records {
                    if let Some((line_kind, text)) = driver_line_for_record(record) {
                        output.push_record(line_kind, &text);
                    }
                }

                // The two header facts that live in journal records rather than
                // in the committed `run.json`: the cumulative cost and the turn
                // count. `driver_line_for_record` deliberately returns `None`
                // for `cost` because it is header data and not a pane line —
                // this is the header reading it (D-12).
                let cache = self.ctx.view_cache.entry(key.0.clone()).or_default();
                update_driver_tally(cache, &key.1, &records);

                // An injected message's state is a set intersection over disk
                // records, and the newest of those records arrive here. They are
                // kept **as records** rather than as rendered lines because the
                // correlation id lives in a typed field, and reconstructing a
                // protocol state by parsing a rendered string is the
                // screen-scraping D-01 forbids in another guise (D-08).
                //
                // Guarded by run id for the reason the tally is: appending the
                // tailed run's transitions to a journal the user is reviewing
                // from last week would report one run's steering as another's.
                if let Some(journal) = cache
                    .driver_journal
                    .as_mut()
                    .filter(|journal| journal.run_id == key.1)
                {
                    journal.injections.extend(
                        records
                            .iter()
                            .filter(|record| {
                                crate::ui::screens::driver::INJECTION_KINDS
                                    .contains(&record.kind.as_str())
                            })
                            .cloned(),
                    );
                }

                self.ctx.journal_cursors.insert(key, cursor);
                // The pane can now show a frame that differs from the one on
                // screen, which is precisely the condition that was absent
                // before (D-20).
                self.needs_redraw = true;
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
            // The goal travels from the sender and is recorded into
            // `RunRecord.goal` verbatim, interpreting nothing (Phase 21 owns
            // goal decomposition). `None` and `Some("")` are deliberately kept
            // distinct all the way down: `drive_argv` omits the flag entirely
            // for `None`, where an empty string would record an empty goal as
            // though one had been given.
            //
            // That reasoning was written when the handler passed a hard-coded
            // `None`, because there was no screen to type a goal into — Phase
            // 18's start flow (D-23) is what changes. The handler now passes the
            // variant's own `goal` through, and the reasoning it replaces is not
            // obsolete but **load-bearing**: the goal picker's Step B accepts an
            // empty buffer, so the empty-string trap is live rather than
            // theoretical, and [`goal_or_none`] is what keeps it shut.
            Action::DriverStartRequested {
                alias,
                command,
                goal,
            } => {
                #[cfg(unix)]
                self.start_driver_run(&alias, &command, goal_or_none(goal.as_deref()));
                // Off Unix there is no detached spawn to reach, so the request
                // has nowhere to go. The bindings are consumed explicitly
                // rather than left to an `unused_variables` allow.
                #[cfg(not(unix))]
                let _ = (alias, command, goal);
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
            // The stop already happened; this is the report, and **the maps are
            // mutated only when the run is actually gone** (WR-15, D-29).
            //
            // When the run IS gone the entry is dropped rather than edited,
            // because `observed_runs` is a projection of the scan and a
            // hand-edited entry would be overwritten by the next one anyway.
            // Dropping it is what the scan itself will do five seconds later,
            // done now so the dashboard does not show a stopped run as live in
            // the meantime.
            //
            // When it may still be live, nothing is dropped. `SignalFailed`
            // means the signal was **never delivered** and `AlreadyGone` means
            // **nothing was signalled** — in neither case has anything
            // established that the run ended, and dropping on either cost two
            // distinct things:
            //
            // * the dashboard showed **no run** for up to five seconds, until
            //   the next reconciliation scan put back a run that never left; and
            // * the `session_spawned_runs` entry was gone **permanently** —
            //   nothing ever re-inserts it, because the disk cannot say who a
            //   driver's parent was — so a later stop took the `Adopted` reaping
            //   arm for a run this session did spawn, waiting on a `/proc`
            //   re-probe instead of the reaping task that actually owns the
            //   `wait()`.
            //
            // **Phase 18 is what makes the contradiction visible:** the Driver
            // tab renders a "no run" pane directly beside a status line reading
            // *"the stop signal could not be delivered"*. The status message is
            // still set on both arms — the user is told what happened either
            // way; what changes is that the dashboard no longer forgets a run on
            // the strength of a stop that stopped nothing.
            Action::DriverStopped {
                alias,
                run_id,
                outcome,
                disposition,
            } => {
                if disposition == crate::action::StopDisposition::RunGone {
                    self.ctx.observed_runs.remove(&alias);
                    self.ctx.session_spawned_runs.remove(&run_id);
                }
                self.ctx.status_message =
                    Some((format!("{alias}: {outcome}"), std::time::Instant::now()));
                self.needs_redraw = true;
            }
            // ── The Driver surface's four messages ────────────────────
            //
            // Four explicit arms rather than a catch-all: a `_ =>` would let a
            // later variant be added and silently ignored, which is the
            // opposite of what this match's exhaustiveness is for.
            //
            // Nothing is logged from `text`, `report` or any other message
            // body: log lines in this subsystem carry the error **kind** and
            // counts only, because a body can carry agent output (S3).

            // The request arrives from a key handler that touched no file, and
            // it leaves this arm still having touched none (D-22, D-28).
            Action::DriverInjectRequested {
                alias,
                run_id,
                id,
                text,
            } => {
                self.schedule_inbox_append(&alias, &run_id, id, text);
            }
            // The durable append reported. **`queued` is set here and nowhere
            // earlier**, because STEER-03's criterion is survival of the
            // writing process: a buffered write that dies with the TUI
            // satisfies the UI and fails the criterion, so nothing may claim
            // the message is queued before the `sync_data()`-backed write
            // returned `Ok` (D-06).
            //
            // On failure the error copy states that **nothing was written** —
            // the honest thing to say, and the thing that tells the user
            // retrying is safe rather than a way to send the same steer twice.
            Action::DriverInjectWritten {
                alias,
                run_id,
                id,
                error,
            } => {
                match error {
                    Some(kind) => {
                        tracing::warn!(
                            %alias,
                            %run_id,
                            %id,
                            kind = %kind,
                            "injection append failed",
                        );
                        self.ctx.error_message = Some(format!(
                            "Could not queue message: {kind}. Nothing was written — try again."
                        ));
                    }
                    None => {
                        // Deliberately promises nothing about timing. The
                        // dequeue ack was measured at 55 seconds, so any word
                        // implying imminence would be a lie about the protocol
                        // (D-07).
                        self.ctx.status_message = Some((
                            "Queued — waiting for the driver to pick it up.".to_string(),
                            std::time::Instant::now(),
                        ));
                    }
                }
                self.needs_redraw = true;
            }
            // Both payloads replace rather than merge, for the reason
            // `RunsReconciled` does: each read is authoritative, and a run
            // directory or a message no longer on disk is expressed by its
            // absence and by nothing else.
            Action::DriverRunsListed {
                alias,
                runs,
                inbox,
                journal,
            } => {
                let cache = self.ctx.view_cache.entry(alias).or_default();
                // Clamped rather than reset: a scan that lands while the user is
                // on a run must not move the selection off it, and a selection
                // past the end of a shortened list must not dangle.
                if !runs.is_empty() {
                    cache.driver_selected_run = cache.driver_selected_run.min(runs.len() - 1);
                }
                cache.driver_runs = runs;
                cache.driver_inbox = inbox;
                // The journal carries its own run id, so the renderer can refuse
                // to show it under a run it is not about.
                cache.driver_journal = journal;
                self.needs_redraw = true;
            }
            // The report was built off the render thread and arrived (D-26).
            //
            // The load-bearing half of D-26 is the *scheduling*:
            // `build_report` shells out to `git` twice and is one of the WR-10
            // call sites the Phase 17 review named, so it must never run on the
            // render thread. `AppContext::schedule_dry_run_report` discharges
            // that in full, and it is the half with a threat-register row
            // (T-18-25, T-18-62).
            //
            // This arm is the other half, landed by plan 18-11: the string is
            // parked in `ProjectViewCache::driver_dry_run`, which the start
            // screen created when it reached Step B.
            //
            // **Guarded twice, and neither guard is belt-and-braces.** A report
            // is stored only when a preview is actually open — a report that
            // arrives after the user pressed Esc has nowhere to go, and
            // recreating the preview for it would repaint a pane the user has
            // left — and only when the command it was built for is still the
            // command the preview is about. Without the second guard, a user who
            // went back to Step A and retyped would see the previous command's
            // blast radius under the new command's name, which is the display
            // disagreeing with the disk in the direction that flatters the run.
            //
            // Logged by count, never by body: the report interpolates a working
            // tree's file names (S3).
            Action::DriverDryRunLoaded {
                alias,
                command,
                report,
            } => {
                tracing::debug!(
                    %alias,
                    %command,
                    bytes = report.len(),
                    "dry-run report built off the render thread",
                );
                if let Some(preview) = self
                    .ctx
                    .view_cache
                    .get_mut(&alias)
                    .and_then(|cache| cache.driver_dry_run.as_mut())
                    .filter(|preview| preview.command == command)
                {
                    preview.report = Some(report);
                    self.needs_redraw = true;
                }
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
        // The ring buffer joins the pass, and this line is the phase's only
        // remaining Phase 16 carry-forward — a negative one. `driver_output` is
        // keyed by alias and inserted on every journal append; without this it
        // would grow for the life of the process, holding up to
        // `DRIVER_OUTPUT_RING_LINES` lines for every project ever driven and
        // then unregistered. That is the Phase 16 leak reintroduced under a new
        // name, which is exactly what the carry-forward exists to prevent.
        //
        // No per-run second pass, because the map has no run dimension: one
        // alias holds one buffer, already bounded by its own cap.
        self.ctx
            .driver_output
            .retain(|alias, _| registered.contains_key(alias));
        // `view_cache` joins the pass too, and plan 18-09 is what makes that
        // necessary rather than tidy. It is keyed by alias and was never pruned,
        // which cost little while it held browse listings and git logs for
        // projects the user had merely visited — but this phase moved three
        // driver payloads into it: `driver_runs` (a `RunSummary` per run on
        // disk), `driver_inbox` (every queued message for the selected run) and
        // `driver_tally`. Those are exactly the per-alias driver state the
        // phase's carry-forward is about, so leaving them in an unpruned map
        // would satisfy the obligation for the maps it names while breaking it
        // for the one it does not.
        //
        // Dropping the whole entry for an unregistered alias is correct rather
        // than merely convenient: the project is gone, so every view state it
        // held — driver and otherwise — describes something the user can no
        // longer open.
        self.ctx
            .view_cache
            .retain(|alias, _| registered.contains_key(alias));
        // The last two alias-keyed maps on `AppContext`, added by plan 18-11 so
        // that "every per-alias map is pruned" is literally true rather than
        // true of the driver maps and quietly false of two neighbours.
        //
        // Neither predates this pass by accident: `last_refresh` holds one
        // `Instant` per alias ever watched and `archive_cache` holds a whole
        // parsed milestone archive per alias ever browsed, and neither has a
        // removal site. They are small and slow-growing rather than harmless —
        // "small leak" is how the Phase 16 leak was described before it was
        // measured. Both are caches, so dropping an entry costs at most one
        // re-derivation for a project that no longer exists.
        self.ctx
            .last_refresh
            .retain(|alias, _| registered.contains_key(alias));
        self.ctx
            .archive_cache
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
        //
        // **It refuses visibly** (WR-11, S4). A bare `return` here left the user
        // pressing the stop key against a live run and being told nothing at
        // all, which reads as "the key does not work" — and the run keeps
        // driving their repository meanwhile. The condition is not reachable in
        // production (the channel is installed at startup and held for the
        // process lifetime), which is exactly why the silent arm survived: it
        // only fires in a test or after a wiring regression, and both are cases
        // where silence costs the most.
        let Some(tx) = &self.ctx.event_tx else {
            self.ctx.error_message = Some(format!(
                "Could not stop the run on '{alias}': this session has no event channel."
            ));
            self.needs_redraw = true;
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
                // The disposition is derived here, at the seam that still holds
                // the typed `StopOutcome`, and travels beside the rendered
                // text. Recovering it downstream by matching on that text would
                // be screen-scraping this project's own output (D-29).
                disposition: crate::action::StopDisposition::from(&outcome),
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

    /// A `RunSummary` with only the field these assertions read varied.
    fn run_summary(run_id: &str) -> crate::journal::RunSummary {
        crate::journal::RunSummary {
            run_id: run_id.to_string(),
            started_at: "2026-07-29T12:02:00Z".to_string(),
            ended_at: None,
            goal: "ship it".to_string(),
            gsd_command: "/gsd-progress".to_string(),
            outcome: None,
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
            disposition,
        } = action
        else {
            panic!("the dispatched stop must report through DriverStopped, got {action:?}");
        };
        assert_eq!(alias, OBS_ALIAS);
        assert_eq!(run_id, "run-x");
        assert!(!outcome.is_empty(), "the outcome must render as something");
        // The pid belongs to no driver, so the dispatched task took the
        // `AlreadyGone` path — which establishes nothing about whether a run
        // ended, and therefore reports `MayStillBeLive` (WR-15/D-29).
        assert_eq!(disposition, crate::action::StopDisposition::MayStillBeLive);

        // And on a report that the run really IS gone, the handler drops the
        // entry rather than editing it, so the dashboard does not show a
        // stopped run as live until the next scan. The disposition is supplied
        // by hand rather than reused from the dispatch above, because that
        // dispatch legitimately produced the other one — reusing it would make
        // this assertion silently test nothing.
        app.update(Action::DriverStopped {
            alias: OBS_ALIAS.to_string(),
            run_id: "run-x".to_string(),
            outcome,
            disposition: crate::action::StopDisposition::RunGone,
        });
        assert!(!app.ctx.observed_runs.contains_key(OBS_ALIAS));
        assert!(!app.ctx.session_spawned_runs.contains("run-x"));
    }

    // ── WR-15 / D-29: a stop that stopped nothing must forget nothing ────
    //
    // One test per `StopOutcome` variant, driven through the real
    // `StopDisposition::from` conversion rather than through a hand-picked
    // disposition — otherwise the test would assert the handler's behaviour
    // for a mapping the send site might not actually produce, and the two
    // halves of the fix could drift apart silently.
    //
    // **The load-bearing half of each is the second assertion**, following the
    // discipline `driver_confirm.rs:367-372` records: an implementation that
    // set the status message and mutated the maps anyway would pass a test that
    // only checked the message, and the user would be shown the truth beside a
    // dashboard that had already forgotten the run.

    /// One `App` holding a live observed run this session spawned, ready to
    /// receive a stop report for it.
    #[cfg(unix)]
    fn app_with_a_spawned_run(
        root: &std::path::Path,
    ) -> (App, tokio::sync::mpsc::UnboundedReceiver<Action>) {
        let (mut app, rx) = obs_app(root);
        app.ctx
            .observed_runs
            .insert(OBS_ALIAS.to_string(), observed(OBS_ALIAS, "run-x", ALIVE));
        app.ctx.session_spawned_runs.insert("run-x".to_string());
        (app, rx)
    }

    /// Apply a stop report derived from `outcome` and return the two facts the
    /// assertions below care about: whether each map kept its entry.
    #[cfg(unix)]
    fn report_stop(
        app: &mut App,
        outcome: crate::driver::kill::StopOutcome,
    ) -> (bool, bool, Option<String>) {
        app.update(Action::DriverStopped {
            alias: OBS_ALIAS.to_string(),
            run_id: "run-x".to_string(),
            disposition: crate::action::StopDisposition::from(&outcome),
            outcome: outcome.to_string(),
        });
        (
            app.ctx.observed_runs.contains_key(OBS_ALIAS),
            app.ctx.session_spawned_runs.contains("run-x"),
            app.ctx.status_message.as_ref().map(|(text, _)| text.clone()),
        )
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_run_that_exited_on_terminate_is_dropped_from_both_maps() {
        use crate::driver::kill::StopOutcome;

        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = app_with_a_spawned_run(dir.path());

        let (observed_kept, spawned_kept, message) =
            report_stop(&mut app, StopOutcome::ExitedOnTerminate);

        assert!(
            !observed_kept,
            "the driver was observed gone, so the dashboard must not keep showing \
             its run as live until the next scan"
        );
        assert!(
            !spawned_kept,
            "a run that is gone needs no reaping arm, so its session record goes too"
        );
        let message = message.expect("the outcome must still reach the status line");
        assert!(message.contains(OBS_ALIAS), "got: {message}");
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_run_that_exited_after_the_uncatchable_signal_is_dropped_from_both_maps() {
        use crate::driver::kill::StopOutcome;

        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = app_with_a_spawned_run(dir.path());

        let (observed_kept, spawned_kept, message) =
            report_stop(&mut app, StopOutcome::ExitedAfterKill);

        assert!(
            !observed_kept,
            "an escalated stop is still a stop that was observed to work"
        );
        assert!(!spawned_kept);
        let message = message.expect("the outcome must still reach the status line");
        assert!(
            message.contains("orphaned"),
            "an escalation must still tell the user its `claude` group may have \
             been orphaned, got: {message}"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_stop_that_signalled_nothing_keeps_both_map_entries() {
        use crate::driver::kill::StopOutcome;

        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = app_with_a_spawned_run(dir.path());

        let (observed_kept, spawned_kept, message) =
            report_stop(&mut app, StopOutcome::AlreadyGone);

        // The user is told, either way. That half was never broken.
        let message = message.expect("the outcome must reach the status line");
        assert!(message.contains(OBS_ALIAS), "got: {message}");

        // The half that was. `AlreadyGone` means NOTHING WAS SIGNALLED — the
        // probe simply did not recognise the pid as this run's driver — so it
        // establishes nothing about whether the run ended.
        assert!(
            observed_kept,
            "WR-15: dropping the observed run here shows 'no run' for up to five \
             seconds beside a status line saying nothing was signalled"
        );
        assert!(
            spawned_kept,
            "WR-15: this entry is never re-inserted — the disk cannot say who a \
             driver's parent was — so dropping it makes a later stop take the \
             `Adopted` arm for a run this session did spawn"
        );
    }

    #[cfg(unix)]
    #[tokio::test]
    async fn a_stop_whose_signal_was_never_delivered_keeps_both_map_entries() {
        use crate::driver::kill::StopOutcome;

        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = app_with_a_spawned_run(dir.path());

        let (observed_kept, spawned_kept, message) = report_stop(
            &mut app,
            StopOutcome::SignalFailed {
                detail: "PermissionDenied".to_string(),
            },
        );

        let message = message.expect("the outcome must reach the status line");
        assert!(
            message.contains("could not be delivered"),
            "the user must be told the signal never landed, got: {message}"
        );

        // This is the arm the Driver tab makes impossible to miss: a "no run"
        // pane beside a status line saying the signal could not be delivered.
        assert!(
            observed_kept,
            "WR-15: the signal was never delivered, so the run is most likely \
             still driving the user's repository"
        );
        assert!(spawned_kept);
    }

    // ── D-20: the seam, the gap diagnostic, the tick gate, the prune ─────

    /// One journal record of `kind`, carrying `fields` as its payload.
    fn journal_record(
        seq: u64,
        kind: &str,
        fields: &[(&str, serde_json::Value)],
    ) -> crate::journal::reader::JournalRecord {
        let mut rest = serde_json::Map::new();
        for (key, value) in fields {
            rest.insert((*key).to_string(), value.clone());
        }
        crate::journal::reader::JournalRecord {
            ts: "2026-07-29T12:00:00Z".to_string(),
            seq,
            kind: kind.to_string(),
            rest,
        }
    }

    /// An `exec_event` on `stream` carrying `text`.
    fn exec_event(seq: u64, stream: &str, text: &str) -> crate::journal::reader::JournalRecord {
        journal_record(
            seq,
            "exec_event",
            &[("stream", stream.into()), ("text", text.into())],
        )
    }

    /// Every buffered line for `alias`, as `(kind, text)` pairs.
    fn buffered(app: &App, alias: &str) -> Vec<(crate::ui::screens::DriverLineKind, String)> {
        app.ctx
            .driver_output
            .get(alias)
            .map(|output| {
                output
                    .lines()
                    .map(|line| (line.kind, line.text.clone()))
                    .collect()
            })
            .unwrap_or_default()
    }

    /// The header's two record-borne facts — the cumulative cost and the turn
    /// count — are read from the tail as it arrives, and both are evidence:
    /// `cost.cumulative_usd` and an `exec_event` on the `turn_completed` stream.
    #[tokio::test]
    async fn a_journal_batch_tallies_the_cumulative_cost_and_the_turn_boundaries() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());

        app.update(Action::DriverJournalAppended {
            alias: OBS_ALIAS.to_string(),
            run_id: OBS_RUN.to_string(),
            records: vec![
                exec_event(1, "assistant", "working"),
                exec_event(2, "turn_completed", "turn ended: success"),
                journal_record(3, "cost", &[("cumulative_usd", 0.42.into())]),
                exec_event(4, "turn_completed", "turn ended: success"),
                journal_record(5, "cost", &[("cumulative_usd", 1.83.into())]),
            ],
            cursor: cursor_at(120, 5),
        });

        let tally = app
            .ctx
            .view_cache
            .get(OBS_ALIAS)
            .and_then(|cache| cache.driver_tally.clone())
            .expect("a tally after a batch");
        assert_eq!(tally.run_id, OBS_RUN);
        // The LAST cumulative value wins; it is a running total, not a delta.
        assert_eq!(tally.cumulative_cost_usd, Some(1.83));
        assert_eq!(tally.turn_boundaries, 2);

        // A batch from a DIFFERENT run resets the tally rather than adding to
        // it — one run's cost shown against another's is a figure the evidence
        // does not support.
        app.update(Action::DriverJournalAppended {
            alias: OBS_ALIAS.to_string(),
            run_id: "2026-07-29T22-00-00Z-beef".to_string(),
            records: vec![exec_event(1, "assistant", "a different run")],
            cursor: cursor_at(20, 1),
        });
        let tally = app
            .ctx
            .view_cache
            .get(OBS_ALIAS)
            .and_then(|cache| cache.driver_tally.clone())
            .expect("a tally after the second batch");
        assert_eq!(tally.run_id, "2026-07-29T22-00-00Z-beef");
        assert_eq!(
            tally.cumulative_cost_usd, None,
            "the previous run's cost must not carry over"
        );
        assert_eq!(tally.turn_boundaries, 0);
    }

    #[tokio::test]
    async fn a_journal_batch_reaches_the_ring_buffer_and_requests_a_redraw() {
        use crate::ui::screens::DriverLineKind;

        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());

        app.needs_redraw = false;
        app.update(Action::DriverJournalAppended {
            alias: OBS_ALIAS.to_string(),
            run_id: OBS_RUN.to_string(),
            records: vec![
                exec_event(1, "assistant", "reading src/driver/run.rs"),
                exec_event(2, "stderr", "a warning from the tool"),
                journal_record(3, "run_ended", &[("outcome", "succeeded".into())]),
            ],
            cursor: cursor_at(120, 3),
        });

        let lines = buffered(&app, OBS_ALIAS);
        assert_eq!(
            lines.len(),
            3,
            "every record with an output-pane class must be buffered, got: {lines:?}"
        );
        assert_eq!(
            lines[0],
            (
                DriverLineKind::Output,
                "reading src/driver/run.rs".to_string()
            )
        );
        assert_eq!(lines[1].0, DriverLineKind::Stderr, "stderr is its own class");
        assert_eq!(
            lines[2].0,
            DriverLineKind::Terminal,
            "the ending is the visual full stop"
        );
        assert!(
            lines[2].1.contains("succeeded"),
            "the terminal line carries the derived outcome, got: {}",
            lines[2].1
        );

        assert!(
            app.needs_redraw,
            "D-20: the handler that adds the surface adds the flag with it — \
             without this the Driver tab renders a frame that can never change"
        );

        // And the omissions the rewritten comment claims are still real.
        assert!(
            app.ctx.last_refresh.is_empty(),
            "the driver route must still not touch the 500ms dedup map (D-14)"
        );
        assert_eq!(
            app.ctx.reparse_dispatches, 0,
            "a journal append must still schedule no full re-parse (OBS-06)"
        );
    }

    #[tokio::test]
    async fn a_sequence_gap_becomes_a_line_the_user_can_see() {
        use crate::ui::screens::DriverLineKind;

        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());

        // Seed a cursor at seq 4, then deliver a batch starting at 9. Five
        // records the writer emitted were never read, and the pane cannot
        // recover them — so it has to say so.
        let key = (OBS_ALIAS.to_string(), OBS_RUN.to_string());
        app.ctx.journal_cursors.insert(key, cursor_at(10, 4));

        app.update(Action::DriverJournalAppended {
            alias: OBS_ALIAS.to_string(),
            run_id: OBS_RUN.to_string(),
            records: vec![exec_event(9, "assistant", "still working")],
            cursor: cursor_at(80, 9),
        });

        let lines = buffered(&app, OBS_ALIAS);
        assert_eq!(lines.len(), 2, "the gap line plus the record: {lines:?}");
        assert_eq!(
            lines[0].0,
            DriverLineKind::Diagnostic,
            "a gap is a diagnostic, never silently swallowed"
        );
        assert!(
            lines[0].1.contains("journal gap") && lines[0].1.contains('1'),
            "the diagnostic must name the count, got: {}",
            lines[0].1
        );
        assert_eq!(
            lines[1].0,
            DriverLineKind::Output,
            "the gap goes ABOVE the records that did arrive, because it is what \
             came before them"
        );

        // The control arm: a contiguous batch adds no diagnostic, so the
        // assertion above is not passing because every batch gets one.
        app.update(Action::DriverJournalAppended {
            alias: OBS_ALIAS.to_string(),
            run_id: OBS_RUN.to_string(),
            records: vec![exec_event(10, "assistant", "carrying on")],
            cursor: cursor_at(120, 10),
        });
        let lines = buffered(&app, OBS_ALIAS);
        assert_eq!(lines.len(), 3, "no second gap line: {lines:?}");
    }

    /// The record → line projection, across every class the pane renders.
    #[test]
    fn every_diagnostic_bearing_record_kind_reaches_the_pane() {
        use crate::ui::screens::DriverLineKind;

        let cases: Vec<(crate::journal::reader::JournalRecord, DriverLineKind, &str)> = vec![
            (
                journal_record(1, "events_dropped", &[("count", 40.into())]),
                DriverLineKind::Diagnostic,
                "40 stream events were dropped",
            ),
            (
                journal_record(
                    2,
                    "journal_truncated",
                    &[("bytes_written", 1024.into()), ("cap", 2048.into())],
                ),
                DriverLineKind::Diagnostic,
                "2048",
            ),
            (
                journal_record(
                    3,
                    "diagnostic",
                    &[("code", "seq_gap".into()), ("detail", "one record".into())],
                ),
                DriverLineKind::Diagnostic,
                "seq_gap",
            ),
            (
                journal_record(
                    4,
                    "interjected",
                    &[("id", "3f2a".into()), ("text", "skip the UI review".into())],
                ),
                DriverLineKind::Injection,
                "skip the UI review",
            ),
            (
                journal_record(5, "interjection_acted_on", &[("id", "3f2a".into())]),
                DriverLineKind::Injection,
                "3f2a",
            ),
            (
                journal_record(
                    6,
                    "interjection_missed",
                    &[("id", "3f2a".into()), ("reason", "stdin closed".into())],
                ),
                DriverLineKind::Injection,
                "stdin closed",
            ),
            (
                journal_record(7, "exec_finished", &[("exit", 0.into()), ("duration_s", 12.into())]),
                DriverLineKind::Terminal,
                "exit 0",
            ),
        ];

        for (record, expected_kind, expected_substring) in cases {
            let (kind, text) = driver_line_for_record(&record)
                .unwrap_or_else(|| panic!("`{}` must reach the pane", record.kind));
            assert_eq!(kind, expected_kind, "wrong class for `{}`", record.kind);
            assert!(
                text.contains(expected_substring),
                "`{}` rendered as {text:?}, which does not name {expected_substring:?}",
                record.kind
            );
            assert!(
                !text.contains('{'),
                "`{}` must compose a real string, never a Debug rendering: {text:?}",
                record.kind
            );
        }

        // The header's own data is deliberately NOT buffered — it renders above
        // the pane, and duplicating it would make the first rows a copy of the
        // header. This is not "losing lines"; the assertions above are what
        // guarantee that every diagnostic-bearing kind does arrive.
        for kind in ["run_started", "exec_started", "cost", "parked"] {
            assert!(
                driver_line_for_record(&journal_record(1, kind, &[])).is_none(),
                "`{kind}` belongs to the header or to a later phase's timeline"
            );
        }
    }

    /// D-21's truth table: three conditions, and all three are required.
    #[test]
    fn the_elapsed_redraw_gate_is_true_for_exactly_one_combination() {
        use crate::ui::screens::detail::DetailScreen;

        let live = observed(OBS_ALIAS, "run-x", ALIVE);
        let dead = observed(OBS_ALIAS, "run-x", DEAD);

        // The one combination that repaints.
        assert!(
            driver_elapsed_redraw_wanted(
                DetailScreen::NAME,
                Some(&DetailSubView::Driver),
                Some(&live)
            ),
            "a live run on the Driver tab is the case the counter exists for"
        );

        // The dashboard. This is the one that matters most: an idle fleet
        // dashboard repainting four times a second forever is the exact
        // opposite of this tool's pitch.
        assert!(
            !driver_elapsed_redraw_wanted("normal", Some(&DetailSubView::Driver), Some(&live)),
            "the counter is not on screen from the dashboard"
        );

        // A detail view on another tab. The counter is not rendered there
        // either.
        assert!(
            !driver_elapsed_redraw_wanted(
                DetailScreen::NAME,
                Some(&DetailSubView::Pipeline),
                Some(&live)
            ),
            "no other tab shows an elapsed counter"
        );

        // The Driver tab with nothing running. A finished run's elapsed figure
        // is frozen, so a frame would be identical to the one already up.
        assert!(
            !driver_elapsed_redraw_wanted(
                DetailScreen::NAME,
                Some(&DetailSubView::Driver),
                Some(&dead)
            ),
            "a run that is not live has a frozen elapsed figure"
        );
        assert!(
            !driver_elapsed_redraw_wanted(DetailScreen::NAME, Some(&DetailSubView::Driver), None),
            "a project with no observed run at all has nothing to count"
        );
    }

    /// The gate wired through a real `Action::Tick`, not just the predicate.
    #[tokio::test]
    async fn a_tick_repaints_only_when_the_driver_tab_is_watching_a_live_run() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());
        app.ctx.recompute_filtered_aliases();
        app.ctx.table_state.select(Some(0));
        app.ctx
            .observed_runs
            .insert(OBS_ALIAS.to_string(), observed(OBS_ALIAS, "run-x", ALIVE));

        // The dashboard is on top and no tab is selected: an idle tick.
        app.needs_redraw = false;
        app.update(Action::Tick);
        assert!(
            !app.needs_redraw,
            "an idle tick must not repaint — this is the CPU burn D-21 names"
        );

        // Now a detail view on the Driver tab, over the same live run.
        app.screen_stack.push(Box::new(
            crate::ui::screens::detail::DetailScreen::new(OBS_ALIAS.to_string()),
        ));
        app.ctx
            .detail_sub_view_per_project
            .insert(OBS_ALIAS.to_string(), DetailSubView::Driver);

        app.needs_redraw = false;
        app.update(Action::Tick);
        assert!(
            app.needs_redraw,
            "the elapsed counter must advance where it can actually be seen"
        );
    }

    /// D-27's negative carry-forward, discharged for `driver_output`.
    #[tokio::test]
    async fn pruning_drops_the_output_buffer_for_an_unregistered_alias() {
        use crate::ui::screens::DriverLineKind;

        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());

        // OBS_ALIAS is registered by the fixture; "gone" never was.
        app.ctx
            .driver_output
            .entry("gone".to_string())
            .or_default()
            .push_record(DriverLineKind::Output, "output from a project that left");
        app.ctx
            .driver_output
            .entry(OBS_ALIAS.to_string())
            .or_default()
            .push_record(DriverLineKind::Output, "output from a project that stayed");

        app.prune_driver_maps();

        assert!(
            !app.ctx.driver_output.contains_key("gone"),
            "a buffer for an unregistered alias holds up to the full ring cap and \
             nothing can ever append to it again — leaving it reintroduces the \
             Phase 16 leak under a new name"
        );

        // The control arm: a registered alias keeps its buffer AND its
        // contents, so the assertion above is not passing because the prune
        // emptied the map.
        let kept = app
            .ctx
            .driver_output
            .get(OBS_ALIAS)
            .expect("a registered alias keeps its buffer");
        assert_eq!(
            kept.len(),
            1,
            "the prune must not clear a live buffer's contents"
        );
    }

    /// Plan 18-09 moved three driver payloads onto `ProjectViewCache`, so the
    /// map that holds it joins the prune pass. Without this, `driver_runs`,
    /// `driver_inbox` and `driver_tally` for a project the user unregistered
    /// would sit in memory for the life of the process — the same leak
    /// `driver_output` was added to the pass to prevent, under a third name.
    #[tokio::test]
    async fn pruning_drops_the_view_cache_for_an_unregistered_alias() {
        use crate::ui::screens::DriverRunTally;

        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());

        for alias in ["gone", OBS_ALIAS] {
            let cache = app.ctx.view_cache.entry(alias.to_string()).or_default();
            cache.driver_selected_run = 2;
            cache.driver_tally = Some(DriverRunTally {
                run_id: OBS_RUN.to_string(),
                cumulative_cost_usd: Some(1.83),
                turn_boundaries: 3,
            });
        }

        app.prune_driver_maps();

        assert!(
            !app.ctx.view_cache.contains_key("gone"),
            "view state for an unregistered project describes something the user \
             can no longer open"
        );

        // The control arm: a registered alias keeps its cache AND its contents,
        // so the assertion above is not passing because the prune emptied the
        // map.
        let kept = app
            .ctx
            .view_cache
            .get(OBS_ALIAS)
            .expect("a registered alias keeps its view cache");
        assert_eq!(kept.driver_selected_run, 2);
        assert_eq!(
            kept.driver_tally.as_ref().and_then(|t| t.cumulative_cost_usd),
            Some(1.83),
            "the prune must not clear a live cache's contents"
        );
    }

    // ── The three schedulers, the goal, and the sub-view ─────────────────

    /// A dispatch with no `event_tx` refuses visibly and sends nothing.
    ///
    /// **The second assertion is the load-bearing one.** A scheduler that set
    /// the message and dispatched anyway would pass a test that only checked
    /// `error_message`, and the write would happen regardless of what the user
    /// was told — the `driver_confirm.rs:367-372` discipline, applied here.
    #[tokio::test]
    async fn an_injection_with_no_event_channel_refuses_visibly_and_writes_nothing() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, mut rx) = obs_app(dir.path());
        app.ctx.event_tx = None;

        app.schedule_inbox_append(
            OBS_ALIAS,
            OBS_RUN,
            "3f2a".to_string(),
            "skip the UI review".to_string(),
        );

        let refusal = app
            .ctx
            .error_message
            .as_deref()
            .expect("a queue that cannot report its result must say so, not fail silently");
        assert!(
            refusal.contains("Nothing was written"),
            "the copy must state that nothing reached disk, so the user knows a \
             retry is safe rather than a way to steer twice, got: {refusal}"
        );
        assert!(
            rx.try_recv().is_err(),
            "a refused injection must dispatch NOTHING"
        );

        // And the file was never created, which is the claim the copy makes.
        let inbox = crate::journal::run_paths(&dir.path().join(".planning"), OBS_RUN)
            .expect("a plain run id resolves")
            .inbox;
        assert!(!inbox.exists(), "nothing was written must mean nothing was written");
    }

    /// A traversing run id is refused before any path is joined (T-18-27).
    #[tokio::test]
    async fn a_traversing_run_id_never_reaches_the_inbox_append() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, mut rx) = obs_app(dir.path());

        app.schedule_inbox_append(
            OBS_ALIAS,
            "../../../../escaped",
            "3f2a".to_string(),
            "steer".to_string(),
        );

        assert!(
            app.ctx.error_message.is_some(),
            "the refusal must be visible, not a silent no-op"
        );
        assert!(rx.try_recv().is_err(), "nothing may be dispatched");

        // The positive control: a plain id does reach the scheduler, so the
        // assertion above is not passing because the function refuses
        // everything.
        app.ctx.error_message = None;
        app.schedule_inbox_append(OBS_ALIAS, OBS_RUN, "b1".to_string(), "steer".to_string());
        assert!(
            app.ctx.error_message.is_none(),
            "a plain run id must not be refused: {:?}",
            app.ctx.error_message
        );
    }

    /// The whole injection round trip, off the render thread (STEER-01/03).
    #[tokio::test]
    async fn an_injection_is_written_off_the_render_thread_and_reports_back() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, mut rx) = obs_app(dir.path());

        let paths = crate::journal::run_paths(&dir.path().join(".planning"), OBS_RUN)
            .expect("a plain run id resolves");
        std::fs::create_dir_all(&paths.dir).expect("the run dir must exist for an append");

        app.schedule_inbox_append(
            OBS_ALIAS,
            OBS_RUN,
            "3f2a".to_string(),
            "skip the UI review".to_string(),
        );

        let action = tokio::time::timeout(std::time::Duration::from_secs(5), rx.recv())
            .await
            .expect("the append must report back promptly, not block the loop")
            .expect("the channel is open");
        let Action::DriverInjectWritten {
            alias, id, error, ..
        } = action
        else {
            panic!("the append must report through DriverInjectWritten, got {action:?}");
        };
        assert_eq!(alias, OBS_ALIAS);
        assert_eq!(id, "3f2a", "the caller's id must survive, not be re-minted");
        assert!(error.is_none(), "unexpected append failure: {error:?}");

        // The bytes are on disk with the caller's id, which is what makes the
        // `queued` state addressable and STEER-03 hold across a restart.
        let written = std::fs::read_to_string(&paths.inbox).expect("the inbox must exist");
        assert!(written.contains("\"id\":\"3f2a\""), "got: {written}");
        assert!(written.contains("skip the UI review"), "got: {written}");
    }

    /// A failed append sets `error_message` and **not** `status_message`.
    ///
    /// The load-bearing half is the second assertion: showing `queued` beside an
    /// error is the exact failure D-06 exists to prevent — the UI satisfied, the
    /// criterion failed.
    #[tokio::test]
    async fn a_failed_injection_write_never_reports_the_message_as_queued() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());

        app.update(Action::DriverInjectWritten {
            alias: OBS_ALIAS.to_string(),
            run_id: OBS_RUN.to_string(),
            id: "3f2a".to_string(),
            error: Some("PermissionDenied".to_string()),
        });

        let failure = app
            .ctx
            .error_message
            .as_deref()
            .expect("a failed append must be visible");
        assert!(failure.contains("PermissionDenied"), "got: {failure}");
        assert!(
            failure.contains("Nothing was written"),
            "the copy must state that nothing reached disk, got: {failure}"
        );
        assert!(
            app.ctx.status_message.is_none(),
            "a message that was never written must never be shown as queued"
        );

        // The control arm: a successful write does set the queued status, and
        // it promises nothing about timing — the dequeue ack was measured at 55
        // seconds, so a word implying imminence would lie about the protocol.
        app.ctx.error_message = None;
        app.update(Action::DriverInjectWritten {
            alias: OBS_ALIAS.to_string(),
            run_id: OBS_RUN.to_string(),
            id: "3f2a".to_string(),
            error: None,
        });
        let queued = app
            .ctx
            .status_message
            .as_ref()
            .map(|(text, _)| text.clone())
            .expect("a successful write must confirm");
        assert!(queued.contains("Queued"), "got: {queued}");
        assert!(
            !queued.to_lowercase().contains("sent"),
            "the word 'sent' is forbidden for an injected message, got: {queued}"
        );
    }

    /// `DriverRunsListed` populates the view cache and requests a redraw.
    #[tokio::test]
    async fn a_run_list_result_populates_the_cache_and_requests_a_redraw() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());

        app.needs_redraw = false;
        app.update(Action::DriverRunsListed {
            alias: OBS_ALIAS.to_string(),
            runs: vec![run_summary("2026-07-29T12-02-00Z-a1b2")],
            inbox: vec![crate::journal::inbox::InboxMessage {
                id: "3f2a".to_string(),
                ts: "2026-07-29T21:40:02Z".to_string(),
                text: "steer".to_string(),
            }],
            journal: None,
        });

        let cache = app
            .ctx
            .view_cache
            .get(OBS_ALIAS)
            .expect("the handler must create the cache entry");
        assert_eq!(cache.driver_runs.len(), 1);
        assert_eq!(cache.driver_runs[0].run_id, "2026-07-29T12-02-00Z-a1b2");
        assert_eq!(cache.driver_inbox.len(), 1);
        assert!(
            app.needs_redraw,
            "a run list that changed under the pane must be painted"
        );
    }

    /// A shortened list clamps the selection rather than leaving it dangling.
    #[tokio::test]
    async fn a_shorter_run_list_clamps_the_selection_instead_of_dangling() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());

        app.ctx
            .view_cache
            .entry(OBS_ALIAS.to_string())
            .or_default()
            .driver_selected_run = 4;

        app.update(Action::DriverRunsListed {
            alias: OBS_ALIAS.to_string(),
            runs: vec![run_summary("2026-07-29T12-02-00Z-a1b2")],
            inbox: Vec::new(),
            journal: None,
        });

        assert_eq!(
            app.ctx.view_cache[OBS_ALIAS].driver_selected_run, 0,
            "a selection past the end of a shortened list must be clamped, or the \
             detail pane indexes a run that is not there"
        );
    }

    /// D-23 / OBS-03: an empty goal buffer reaches `start_driver_run` as `None`.
    ///
    /// Asserted at two levels, because the pure mapping alone would not prove
    /// the consequence: the second half drives the real `drive_argv` and shows
    /// that `None` omits the flag entirely while `Some("")` would push it with
    /// an empty operand — which is what makes the driver record an empty goal
    /// **as though one had been given**.
    #[test]
    fn an_empty_goal_buffer_reaches_start_driver_run_as_none() {
        assert_eq!(goal_or_none(None), None);
        assert_eq!(goal_or_none(Some("")), None, "an empty buffer is no goal");
        assert_eq!(
            goal_or_none(Some("   ")),
            None,
            "a buffer of spaces is no goal either"
        );
        assert_eq!(
            goal_or_none(Some("ship the driver tab")),
            Some("ship the driver tab"),
            "a goal that was given is returned verbatim and never paraphrased"
        );
        assert_eq!(
            goal_or_none(Some("  ship it  ")),
            Some("  ship it  "),
            "emptiness is tested on the trimmed text; the value returned is the \
             untrimmed original, because the goal is stored verbatim"
        );

        #[cfg(unix)]
        {
            use crate::driver::spawn::drive_argv;
            let config = std::path::Path::new("/tmp/config.json");

            let empty = drive_argv(config, "proj", "/gsd-progress", "run-1", goal_or_none(Some("")));
            assert!(
                !empty.iter().any(|arg| arg == "--goal"),
                "an empty goal must omit the flag entirely, got: {empty:?}"
            );

            // The control: a real goal still reaches the child.
            let given = drive_argv(
                config,
                "proj",
                "/gsd-progress",
                "run-1",
                goal_or_none(Some("ship it")),
            );
            let position = given
                .iter()
                .position(|arg| arg == "--goal")
                .expect("a goal that was given must reach the child");
            assert_eq!(given[position + 1], "ship it");
        }
    }

    /// The `DriverStartRequested` handler threads the variant's own goal.
    ///
    /// The refusal is what is observed: `start_driver_run` refuses an unknown
    /// alias before it spawns anything, so this asserts the handler reached the
    /// seam at all rather than dropping the request — the goal's own mapping is
    /// pinned by the test above.
    #[cfg(unix)]
    #[tokio::test]
    async fn a_start_request_carries_its_goal_to_the_spawn_seam() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());

        app.update(Action::DriverStartRequested {
            alias: "nosuchalias".to_string(),
            command: "/gsd-progress".to_string(),
            goal: Some("ship the driver tab".to_string()),
        });

        let refusal = app
            .ctx
            .error_message
            .as_deref()
            .expect("the request must reach `start_driver_run`, which refuses visibly");
        assert!(refusal.contains("nosuchalias"), "got: {refusal}");
    }

    /// The Driver sub-view round-trips through both `detail.rs` mappings.
    #[test]
    fn the_driver_sub_view_is_index_ten_in_both_directions() {
        use crate::ui::screens::detail::{sub_view_from_index, tab_index};

        // Index 10 per D-15, and the two mappings must agree — a tab whose
        // index does not round-trip lands on a different tab than the one the
        // user asked for.
        assert_eq!(tab_index(&DetailSubView::Driver), 10);
        assert_eq!(sub_view_from_index(10), DetailSubView::Driver);
        // The fallback is unchanged: an out-of-range index still lands on the
        // first tab and never on the newest one.
        assert_eq!(sub_view_from_index(11), DetailSubView::PhaseList);
    }

    /// A stop with nowhere to report its outcome refuses **visibly** (WR-11).
    #[cfg(unix)]
    #[tokio::test]
    async fn a_stop_with_no_event_channel_refuses_visibly_rather_than_silently() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = app_with_a_spawned_run(dir.path());
        app.ctx.event_tx = None;

        app.stop_driver_run(OBS_ALIAS);

        let refusal = app
            .ctx
            .error_message
            .as_deref()
            .expect("a stop that cannot report its outcome must say so, not fail silently");
        assert!(refusal.contains(OBS_ALIAS), "got: {refusal}");
        assert!(
            app.ctx.observed_runs.contains_key(OBS_ALIAS),
            "a stop that never dispatched must not have disturbed the observed map"
        );
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

    // ── The phase's negative carry-forward, discharged mechanically ────────

    /// Every per-alias and per-run map on `AppContext` is covered by
    /// `prune_driver_maps` (D-27, the Phase 16 carry-forward).
    ///
    /// **This is the durable form of a promise that was previously kept by
    /// remembering.** The sibling tests above each prove one map is pruned; this
    /// one proves the *set* is complete, and the exhaustive destructuring below
    /// is what makes it stay complete: adding a field to `AppContext` fails to
    /// compile here until someone has looked at this list and decided whether
    /// the new field is alias-keyed. A new map that is not pruned reintroduces
    /// the Phase 16 leak under a new name, which is precisely what the
    /// carry-forward exists to prevent.
    #[tokio::test]
    async fn every_per_alias_driver_map_is_pruned() {
        use crate::executor::RunState;
        use crate::journal::reader::JournalCursor;
        use crate::ui::screens::{DriverLineKind, DryRunPreview};

        const GONE: &str = "gone";

        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());

        // Populate every alias-keyed map for BOTH a registered alias and an
        // alias the registry has never heard of, so each assertion below has a
        // control arm and none of them can pass by emptying the map.
        for alias in [GONE, OBS_ALIAS] {
            app.ctx.journal_cursors.insert(
                (alias.to_string(), OBS_RUN.to_string()),
                JournalCursor::default(),
            );
            app.ctx
                .run_states
                .insert(alias.to_string(), RunState::Running);
            app.ctx.observed_runs.insert(
                alias.to_string(),
                observed(alias, OBS_RUN, crate::driver::liveness::Liveness::Dead),
            );
            app.ctx
                .driver_output
                .entry(alias.to_string())
                .or_default()
                .push_record(DriverLineKind::Output, "a line");
            let cache = app.ctx.view_cache.entry(alias.to_string()).or_default();
            cache.driver_selected_run = 1;
            // The field plan 18-11 added. It is inside `view_cache`, so it
            // inherits that map's prune rather than needing its own — and this
            // test is where that inheritance is checked rather than assumed.
            cache.driver_dry_run = Some(DryRunPreview {
                command: "/gsd:progress".to_string(),
                report: None,
            });
            app.ctx
                .last_refresh
                .insert(alias.to_string(), std::time::Instant::now());
            app.ctx.archive_cache.insert(
                alias.to_string(),
                crate::archive::MilestoneArchive {
                    version: "v1.0".to_string(),
                    top_level_files: Vec::new(),
                    phases: Vec::new(),
                },
            );
        }

        app.prune_driver_maps();

        // ── The enumeration. Destructured exhaustively so a new `AppContext`
        // field is a compile error here until it has been classified.
        let crate::ui::screens::AppContext {
            // Alias-keyed or (alias, run_id)-keyed: every one of these must be
            // pruned, and each is asserted below.
            journal_cursors,
            run_states,
            observed_runs,
            driver_output,
            view_cache,
            last_refresh,
            archive_cache,

            // Alias-keyed but deliberately NOT pruned, with the reason on the
            // field: `project_states` is the registry's own mirror and is
            // rebuilt wholesale by `load_project_states`;
            // `detail_sub_view_per_project` holds one enum per alias and is
            // rewritten on every tab switch.
            project_states: _,
            detail_sub_view_per_project: _,

            // Not alias-keyed at all. `session_spawned_runs` is a set of run
            // ids that grows by one per run this session starts, with
            // `driver_max_concurrent` defaulting to one; the rest are scalars,
            // handles and single values.
            session_spawned_runs: _,
            config: _,
            config_path: _,
            table_state: _,
            filtered_aliases: _,
            filter_text: _,
            change_tracker: _,
            status_message: _,
            error_message: _,
            event_tx: _,
            exec_tx: _,
            reparse_dispatches: _,
            sort_mode: _,
            watcher: _,
            detail_scroll_offset: _,
            suggestion_index: _,
            input_buffer: _,
            needs_redraw: _,
            active_sessions: _,
        } = &app.ctx;

        assert!(
            journal_cursors.keys().all(|(alias, _)| alias != GONE),
            "journal_cursors is keyed (alias, run_id) and must lose every entry \
             for an unregistered alias"
        );
        assert!(!run_states.contains_key(GONE), "run_states leaked");
        assert!(!observed_runs.contains_key(GONE), "observed_runs leaked");
        assert!(!driver_output.contains_key(GONE), "driver_output leaked");
        assert!(!view_cache.contains_key(GONE), "view_cache leaked");
        assert!(!last_refresh.contains_key(GONE), "last_refresh leaked");
        assert!(!archive_cache.contains_key(GONE), "archive_cache leaked");

        // The control arm for all seven at once: the registered alias kept
        // everything, so none of the assertions above passed because the prune
        // cleared the map.
        assert!(journal_cursors.keys().any(|(alias, _)| alias == OBS_ALIAS));
        assert!(run_states.contains_key(OBS_ALIAS));
        assert!(observed_runs.contains_key(OBS_ALIAS));
        assert!(driver_output.contains_key(OBS_ALIAS));
        assert!(last_refresh.contains_key(OBS_ALIAS));
        assert!(archive_cache.contains_key(OBS_ALIAS));
        let kept = view_cache
            .get(OBS_ALIAS)
            .expect("a registered alias keeps its view cache");
        assert_eq!(kept.driver_selected_run, 1);
        assert!(
            kept.driver_dry_run.is_some(),
            "and keeps the preview it had open"
        );
    }

    /// A dry-run report is parked only under the preview it was built for
    /// (D-26).
    #[tokio::test]
    async fn a_dry_run_report_is_stored_only_under_the_preview_it_was_built_for() {
        use crate::ui::screens::DryRunPreview;

        let dir = tempfile::tempdir().expect("temp dir");
        let (mut app, _rx) = obs_app(dir.path());

        // Nothing open: a report that arrives after the user pressed Esc has
        // nowhere to go, and recreating the preview for it would repaint a pane
        // the user has left.
        app.update(Action::DriverDryRunLoaded {
            alias: OBS_ALIAS.to_string(),
            command: "/gsd:progress".to_string(),
            report: "orphaned".to_string(),
        });
        assert!(
            app.ctx
                .view_cache
                .get(OBS_ALIAS)
                .and_then(|cache| cache.driver_dry_run.as_ref())
                .is_none(),
            "a report with no open preview must not create one"
        );

        app.ctx
            .view_cache
            .entry(OBS_ALIAS.to_string())
            .or_default()
            .driver_dry_run = Some(DryRunPreview {
            command: "/gsd:progress".to_string(),
            report: None,
        });

        // Stale: built for a command the user has since retyped. Storing it
        // would show one command's blast radius under another command's name —
        // the display disagreeing with the disk in the direction that flatters
        // the run.
        app.needs_redraw = false;
        app.update(Action::DriverDryRunLoaded {
            alias: OBS_ALIAS.to_string(),
            command: "/gsd:execute-phase 99".to_string(),
            report: "the wrong report".to_string(),
        });
        assert_eq!(
            app.ctx.view_cache[OBS_ALIAS]
                .driver_dry_run
                .as_ref()
                .and_then(|p| p.report.as_deref()),
            None,
            "a report for a different command must be discarded"
        );
        assert!(!app.needs_redraw, "and must not repaint the frame");

        // Matching: stored, and the pane is repainted.
        app.update(Action::DriverDryRunLoaded {
            alias: OBS_ALIAS.to_string(),
            command: "/gsd:progress".to_string(),
            report: "== Push refspecs this state would produce ==".to_string(),
        });
        assert_eq!(
            app.ctx.view_cache[OBS_ALIAS]
                .driver_dry_run
                .as_ref()
                .and_then(|p| p.report.as_deref()),
            Some("== Push refspecs this state would produce =="),
        );
        assert!(app.needs_redraw);
    }
}
