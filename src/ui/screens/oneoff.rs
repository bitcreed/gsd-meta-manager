//! **The one-off popup** (quick 261103-f5y, C-22..C-26; quick 261103-hx7, C-26..C-30):
//! the Driver tab's `t` (pick a pending todo or type a task, then `q` for /gsd-quick or
//! `f` for /gsd-fast, then confirm the model, spend ceiling and wall clock) and `T`
//! (the project's runs, newest first, live and finished, with `j`/`k` to select and
//! Enter for one run's strict summary).
//!
//! Every action names the SELECTED run: `e` SUMMARY.md, `m` merge, `p` push, `D`
//! discard and `x` kill (each asked first), `v` the run's live view, `r` its newest
//! conflict-resolver session and `d` the merge diff of a landed merge. Esc in a run's
//! summary returns to the list; Esc in the list closes it.
//!
//! One screen with internal modes (I-14). It reads only the view cache and dispatches
//! actions: every file read, git read, launch and landing runs on `spawn_blocking` in
//! `App`. Every todo title, typed task, report field, path, run id, conflicted file
//! name and diff line is drawn through [`render_for_terminal`]. The defaults shown come
//! from `cli::task_run_defaults` and the models from the runtime seam, never a copy.

use std::cell::Cell;

use super::{AppContext, Screen, ScreenAction};
use crate::action::Action;
use crate::oneoff::cancel::StopKind;
use crate::oneoff::resume::{self, KeyPlan, ResumeFrom};
use crate::oneoff::runtime::AgentRuntime;
use crate::oneoff::store::{
    run_counts, BranchState, LandTarget, Landing, LandingKind, LiveState, MergeHow, OneoffView,
    ReportSource,
};
use crate::oneoff::{prompt, Mode, OneoffCommand, TaskSpec};
use crate::text::render_for_terminal;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph, Wrap};
use ratatui::Frame;

/// The most characters of one drawn value.
const MAX_SHOWN: usize = 200;

/// The confirm popup's copy about the clone (C-23).
pub const UNCOMMITTED_COPY: &str =
    "Uncommitted changes in the project are not included: the agent works in a private clone.";

/// What the popup shows when the project has no one-off run.
pub const NO_RUN_COPY: &str = "No one-off task has run here yet; press t";

/// The picker row that opens the task input.
const TYPE_ROW: &str = "type a task...";

/// The headline's width in a run list row; longer headlines are cut.
const ROW_HEADLINE_CHARS: usize = 36;

/// The model a one-off starts with when none is chosen: the runtime's first model,
/// else `opus`. The confirm popup and the Backlog tab's one-key dispatch (quick
/// 261103-ubn) both use this, so the two never disagree.
pub(crate) fn default_model() -> String {
    AgentRuntime::default()
        .models()
        .first()
        .copied()
        .unwrap_or("opus")
        .to_owned()
}

/// `value` escaped and capped for one line.
fn shown(value: &str) -> String {
    let rendered = render_for_terminal(value).to_string();
    if rendered.chars().count() <= MAX_SHOWN {
        rendered
    } else {
        let mut cut: String = rendered.chars().take(MAX_SHOWN - 1).collect();
        cut.push('…');
        cut
    }
}

/// [`shown`] cut to `max` characters, for a table cell.
fn shown_to(value: &str, max: usize) -> String {
    let rendered = shown(value);
    if rendered.chars().count() <= max {
        rendered
    } else {
        let mut cut: String = rendered.chars().take(max.saturating_sub(1)).collect();
        cut.push('…');
        cut
    }
}

/// Which part of the flow the popup is in.
#[derive(Debug, Clone, PartialEq)]
enum OneoffMode {
    /// The todo list plus the `type a task...` row.
    Picker { selected: usize },
    /// The single-line task input.
    Input {
        mode: Mode,
        buffer: String,
        error: Option<String>,
    },
    /// Model, spend ceiling, wall clock; Enter launches.
    Confirm {
        mode: Mode,
        task: TaskSpec,
        model: String,
    },
    /// The project's runs, newest first. The selection is [`OneoffScreen::run_id`], so a
    /// refresh that reorders the list keeps it (quick 261103-hx7, C-27).
    List,
    /// One run's summary.
    Result,
    /// A landing or kill, asked first, about exactly the run that was asked about.
    Ask {
        prompt: Vec<String>,
        command: OneoffCommand,
        run_id: String,
        /// Where `n` and Esc return to.
        from: AskFrom,
    },
    /// `u` on a run that can be resumed: `b` from its branch, `M` merge the checkout's
    /// branch first, Esc cancels (quick 261103-klf, C-01).
    ResumeAsk { run_id: String, from: AskFrom },
    /// The landed merge's diff (`d`), scrolled.
    Diff { scroll: usize },
}

/// Where an [`OneoffMode::Ask`] returns to when it is declined.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AskFrom {
    List,
    Result,
    /// A viewer pushed the popup: popping it returns there (quick 261103-klf, I-16).
    Viewer,
}

/// What `x`, `X` and `z` ask for (quick 261103-klf, C-01).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CutKey {
    Stop,
    Kill,
    Pause,
}

impl CutKey {
    /// The key, if `code` is one of the three.
    pub fn from_key(code: KeyCode) -> Option<CutKey> {
        match code {
            KeyCode::Char('x') => Some(CutKey::Stop),
            KeyCode::Char('X') => Some(CutKey::Kill),
            KeyCode::Char('z') => Some(CutKey::Pause),
            _ => None,
        }
    }

    fn word(self) -> &'static str {
        match self {
            CutKey::Stop => "stop",
            CutKey::Kill => "kill",
            CutKey::Pause => "pause",
        }
    }

    /// The command and the confirmation text for the run (`landing`: its conflict
    /// resolver instead of its agent).
    pub fn ask(self, run_id: &str, landing: bool) -> (OneoffCommand, Vec<String>) {
        let id = shown(run_id);
        match (self, landing) {
            (CutKey::Stop, false) => (
                OneoffCommand::Stop,
                vec![format!(
                    "Stop one-off {id}? Its sandbox gets SIGTERM, then SIGKILL after 10 s \
                     if it is still there. Nothing lands."
                )],
            ),
            (CutKey::Kill, false) => (
                OneoffCommand::Kill,
                vec![format!(
                    "Kill one-off {id} now? Its whole sandbox gets SIGKILL at once: no SIGTERM, \
                     no wrap-up."
                )],
            ),
            (CutKey::Pause, false) => (
                OneoffCommand::Pause,
                vec![format!(
                    "Pause one-off {id}? The agent is asked to commit its work in progress, \
                     report partial and end; it is cut hard if it ignores that for 5 minutes. \
                     u resumes it afterwards."
                )],
            ),
            (CutKey::Stop, true) => (
                OneoffCommand::StopLanding,
                vec![format!(
                    "Stop the landing of {id}? Its conflict resolver is cut (SIGTERM, then \
                     SIGKILL); nothing lands and the run keeps its branch."
                )],
            ),
            (CutKey::Kill, true) => (
                OneoffCommand::KillLanding,
                vec![format!(
                    "Kill the landing of {id} now? Its conflict resolver gets SIGKILL at once; \
                     nothing lands and the run keeps its branch."
                )],
            ),
            (CutKey::Pause, true) => (
                OneoffCommand::PauseLanding,
                vec![format!(
                    "Pause the landing of {id}? The current resolver attempt may finish, but \
                     nothing lands and no further attempt starts. u retries the landing."
                )],
            ),
        }
    }

    /// The refusal when the run is neither live nor being landed.
    pub fn refusal(self, run_id: &str) -> String {
        format!(
            "One-off {} is not running, so there is nothing to {}.",
            shown(run_id),
            self.word()
        )
    }
}

/// The one-off popup of one project.
pub struct OneoffScreen {
    alias: String,
    mode: OneoffMode,
    /// The selected run (`None`: the newest).
    run_id: Option<String>,
    /// The diff body's height as last drawn, for paging.
    diff_height: Cell<usize>,
}

impl OneoffScreen {
    fn in_mode(alias: String, mode: OneoffMode) -> Self {
        Self {
            alias,
            mode,
            run_id: None,
            diff_height: Cell::new(20),
        }
    }

    /// The resume question for one run, pushed by the viewers (klf, I-16).
    pub fn resume_ask_for(alias: String, run_id: String) -> Self {
        Self::in_mode(
            alias,
            OneoffMode::ResumeAsk {
                run_id,
                from: AskFrom::Viewer,
            },
        )
    }

    /// A confirmation for one run, pushed by the viewers (quick 261103-klf, I-16): `y`
    /// dispatches and pops, `n` and Esc pop back to the viewer.
    pub fn ask_for(
        alias: String,
        run_id: String,
        command: OneoffCommand,
        prompt: Vec<String>,
    ) -> Self {
        Self::in_mode(
            alias,
            OneoffMode::Ask {
                prompt,
                command,
                run_id,
                from: AskFrom::Viewer,
            },
        )
    }

    /// The picker (`t`).
    pub fn picker(alias: String) -> Self {
        Self::in_mode(alias, OneoffMode::Picker { selected: 0 })
    }

    /// The run list (`T`).
    pub fn list(alias: String) -> Self {
        Self::in_mode(alias, OneoffMode::List)
    }

    fn todos<'a>(&self, ctx: &'a AppContext) -> &'a [crate::oneoff::todos::PendingTodo] {
        match ctx
            .view_cache
            .get(&self.alias)
            .and_then(|c| c.oneoff_todos.as_ref())
        {
            Some(Ok(todos)) => todos,
            _ => &[],
        }
    }

    fn runs<'a>(&self, ctx: &'a AppContext) -> &'a [OneoffView] {
        ctx.view_cache
            .get(&self.alias)
            .map_or(&[], |c| c.oneoff_runs.as_slice())
    }

    /// The index of the selected run in the list: the one named by `run_id`, else the
    /// newest.
    fn selected_index(&self, runs: &[OneoffView]) -> usize {
        self.run_id
            .as_deref()
            .and_then(|id| runs.iter().position(|v| v.record.run_id == id))
            .unwrap_or(0)
    }

    /// The selected run. In the list a run that vanished falls back to the newest; in a
    /// run's own summary it is `None`, never another run.
    fn selected<'a>(&self, ctx: &'a AppContext) -> Option<&'a OneoffView> {
        let runs = self.runs(ctx);
        match &self.run_id {
            None => runs.first(),
            Some(id) => runs.iter().find(|v| v.record.run_id == *id).or_else(|| {
                matches!(self.mode, OneoffMode::List)
                    .then(|| runs.first())
                    .flatten()
            }),
        }
    }

    fn confirm(mode: Mode, task: TaskSpec) -> OneoffMode {
        OneoffMode::Confirm {
            mode,
            task,
            model: default_model(),
        }
    }

    fn picker_key(&mut self, code: KeyCode, ctx: &mut AppContext) -> ScreenAction {
        let OneoffMode::Picker { selected } = self.mode else {
            return ScreenAction::None;
        };
        let todos = self.todos(ctx).to_vec();
        let rows = todos.len() + 1;
        let selected = selected.min(rows - 1);
        match code {
            KeyCode::Esc => return ScreenAction::Pop,
            KeyCode::Char('j') | KeyCode::Down => {
                self.mode = OneoffMode::Picker {
                    selected: (selected + 1).min(rows - 1),
                }
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.mode = OneoffMode::Picker {
                    selected: selected.saturating_sub(1),
                }
            }
            KeyCode::Char(key @ ('q' | 'f')) => {
                let mode = if key == 'q' { Mode::Quick } else { Mode::Fast };
                self.mode = match todos.get(selected) {
                    Some(todo) => Self::confirm(
                        mode,
                        TaskSpec::Todo {
                            file_name: todo.file_name.clone(),
                            title: todo.title.clone(),
                        },
                    ),
                    None => OneoffMode::Input {
                        mode,
                        buffer: String::new(),
                        error: None,
                    },
                };
            }
            _ => {}
        }
        ScreenAction::None
    }

    fn input_key(&mut self, code: KeyCode) -> ScreenAction {
        let OneoffMode::Input {
            mode,
            buffer,
            error,
        } = &mut self.mode
        else {
            return ScreenAction::None;
        };
        match code {
            KeyCode::Esc => {
                self.mode = OneoffMode::Picker { selected: 0 };
            }
            KeyCode::Backspace => {
                buffer.pop();
                *error = None;
            }
            KeyCode::Enter => match prompt::check_task_text(buffer) {
                Ok(()) => {
                    let task = TaskSpec::Text(buffer.clone());
                    self.mode = Self::confirm(*mode, task);
                }
                Err(why) => *error = Some(why),
            },
            KeyCode::Char(c) if !c.is_control() => {
                if buffer.chars().count() < prompt::MAX_TASK_TEXT {
                    buffer.push(c);
                }
                *error = None;
            }
            _ => {}
        }
        ScreenAction::None
    }

    fn confirm_key(&mut self, code: KeyCode, ctx: &mut AppContext) -> ScreenAction {
        let OneoffMode::Confirm { mode, task, model } = &mut self.mode else {
            return ScreenAction::None;
        };
        match code {
            KeyCode::Esc => ScreenAction::Pop,
            KeyCode::Char('m') => {
                let models = AgentRuntime::default().models();
                let at = models.iter().position(|m| m == model).unwrap_or(0);
                *model = models[(at + 1) % models.len()].to_owned();
                ScreenAction::None
            }
            KeyCode::Enter => {
                super::driver_confirm::dispatch(
                    ctx,
                    Action::OneoffRunRequested {
                        alias: self.alias.clone(),
                        mode: *mode,
                        task: task.clone(),
                        model: model.clone(),
                    },
                    &self.alias,
                );
                ScreenAction::Pop
            }
            _ => ScreenAction::None,
        }
    }

    /// The run list: `j`/`k` move the selection (kept by run id), Enter opens the run's
    /// summary, Esc closes, and the action keys act on the selected run.
    fn list_key(&mut self, code: KeyCode, ctx: &mut AppContext) -> ScreenAction {
        match code {
            KeyCode::Esc => ScreenAction::Pop,
            KeyCode::Char('j') | KeyCode::Down | KeyCode::Char('k') | KeyCode::Up => {
                let runs = self.runs(ctx);
                if runs.is_empty() {
                    return ScreenAction::None;
                }
                let at = self.selected_index(runs);
                let next = if matches!(code, KeyCode::Char('j') | KeyCode::Down) {
                    (at + 1).min(runs.len() - 1)
                } else {
                    at.saturating_sub(1)
                };
                self.run_id = Some(runs[next].record.run_id.clone());
                ScreenAction::None
            }
            KeyCode::Enter => {
                if let Some(id) = self.selected(ctx).map(|v| v.record.run_id.clone()) {
                    self.run_id = Some(id);
                    self.mode = OneoffMode::Result;
                }
                ScreenAction::None
            }
            other => self.run_action_key(other, ctx, true),
        }
    }

    fn result_key(&mut self, code: KeyCode, ctx: &mut AppContext) -> ScreenAction {
        if code == KeyCode::Esc {
            self.mode = OneoffMode::List;
            return ScreenAction::None;
        }
        self.run_action_key(code, ctx, false)
    }

    /// The keys that act on the selected run, in the list and in a run's summary.
    fn run_action_key(
        &mut self,
        code: KeyCode,
        ctx: &mut AppContext,
        from_list: bool,
    ) -> ScreenAction {
        let Some(view) = self.selected(ctx).cloned() else {
            return ScreenAction::None;
        };
        let run_id = view.record.run_id.clone();
        let id = shown(&run_id);
        let message = |ctx: &mut AppContext, text: String| {
            ctx.status_message = Some((text, std::time::Instant::now()));
            ScreenAction::None
        };
        let ask = |command: OneoffCommand, prompt: Vec<String>| OneoffMode::Ask {
            prompt,
            command,
            run_id: run_id.clone(),
            from: if from_list {
                AskFrom::List
            } else {
                AskFrom::Result
            },
        };
        match code {
            KeyCode::Char('e') => match &view.summary_file {
                Some(path) => ScreenAction::SuspendAndEdit(path.clone(), None),
                None => message(
                    ctx,
                    format!("One-off {id} has no SUMMARY.md to open (a /gsd-fast run writes none)"),
                ),
            },
            KeyCode::Char('m') => {
                if view.landing {
                    return message(ctx, format!("A landing of {id} is already in progress"));
                }
                let question = match merge_question(&view, &id) {
                    Ok(question) => question,
                    Err(refusal) => return message(ctx, refusal),
                };
                let mut prompt = vec![question];
                prompt.extend(merge_warning(&view));
                self.mode = ask(OneoffCommand::Merge, prompt);
                ScreenAction::None
            }
            KeyCode::Char('p') => {
                self.mode = ask(
                    OneoffCommand::Push,
                    vec![format!(
                        "Push {id} to gsd-auto/{}/{id} on the project's remote? It is scanned \
                         for credentials first.",
                        shown(&self.alias)
                    )],
                );
                ScreenAction::None
            }
            KeyCode::Char('D') => {
                self.mode = ask(
                    OneoffCommand::Discard,
                    vec![format!(
                        "Discard {id}? Its clone, its tracked branch and every commit not \
                         merged or pushed are deleted."
                    )],
                );
                ScreenAction::None
            }
            KeyCode::Char('v') => {
                self.run_id = Some(run_id.clone());
                ScreenAction::DispatchAction(Box::new(Action::OneoffViewerOpen {
                    alias: self.alias.clone(),
                    run_id,
                    attempt: None,
                }))
            }
            KeyCode::Char('r') => match view.land_attempts.last() {
                Some(&attempt) => {
                    self.run_id = Some(run_id.clone());
                    ScreenAction::DispatchAction(Box::new(Action::OneoffViewerOpen {
                        alias: self.alias.clone(),
                        run_id,
                        attempt: Some(attempt),
                    }))
                }
                None => message(ctx, format!("No resolver session for one-off {id}")),
            },
            KeyCode::Char('d') => {
                if view.record.last_merge_landing().is_none() {
                    return message(
                        ctx,
                        format!("No merge to show for one-off {id}: it was not landed by a merge"),
                    );
                }
                ctx.view_cache
                    .entry(self.alias.clone())
                    .or_default()
                    .oneoff_diff = None;
                self.run_id = Some(run_id.clone());
                self.mode = OneoffMode::Diff { scroll: 0 };
                ScreenAction::DispatchAction(Box::new(Action::OneoffDiffRequested {
                    alias: self.alias.clone(),
                    run_id,
                }))
            }
            // klf C-01: `u` retries a paused landing, or resumes a paused, stopped or
            // killed run (asks `b` / `M` first).
            KeyCode::Char('u') => match resume::key_plan(&view) {
                KeyPlan::Resume => {
                    self.mode = OneoffMode::ResumeAsk {
                        run_id: run_id.clone(),
                        from: if from_list {
                            AskFrom::List
                        } else {
                            AskFrom::Result
                        },
                    };
                    ScreenAction::None
                }
                KeyPlan::RetryLanding => self.run_action_key(KeyCode::Char('m'), ctx, from_list),
                KeyPlan::Refused(why) => message(
                    ctx,
                    format!("One-off {id} cannot be resumed: {}", shown(&why)),
                ),
            },
            // klf C-01: stop, kill now and pause; a run that is not live but whose
            // landing is running asks about its landing (I-16).
            KeyCode::Char('x' | 'X' | 'z') => {
                let key = CutKey::from_key(code).unwrap_or(CutKey::Stop);
                let live = matches!(view.live, LiveState::Running | LiveState::Preparing);
                if live || view.landing {
                    let (command, prompt) = key.ask(&run_id, !live);
                    self.mode = ask(command, prompt);
                    ScreenAction::None
                } else {
                    message(ctx, key.refusal(&run_id))
                }
            }
            _ => ScreenAction::None,
        }
    }

    /// `b` / `M` / Esc on the resume question.
    fn resume_ask_key(&mut self, code: KeyCode, ctx: &mut AppContext) -> ScreenAction {
        let OneoffMode::ResumeAsk { run_id, from } = &self.mode else {
            return ScreenAction::None;
        };
        let start = match code {
            KeyCode::Char('b') => Some(ResumeFrom::Branch),
            KeyCode::Char('M') => Some(ResumeFrom::Master),
            _ => None,
        };
        if let Some(start) = start {
            let action = Action::OneoffResumeRequested {
                alias: self.alias.clone(),
                run_id: run_id.clone(),
                from: start,
            };
            super::driver_confirm::dispatch(ctx, action, &self.alias);
            return ScreenAction::Pop;
        }
        if matches!(code, KeyCode::Esc | KeyCode::Char('n')) {
            return match from {
                AskFrom::Viewer => ScreenAction::Pop,
                AskFrom::List => {
                    self.mode = OneoffMode::List;
                    ScreenAction::None
                }
                AskFrom::Result => {
                    self.mode = OneoffMode::Result;
                    ScreenAction::None
                }
            };
        }
        ScreenAction::None
    }

    fn ask_key(&mut self, code: KeyCode, ctx: &mut AppContext) -> ScreenAction {
        let OneoffMode::Ask {
            command,
            run_id,
            from,
            ..
        } = &self.mode
        else {
            return ScreenAction::None;
        };
        match code {
            KeyCode::Char('y') => {
                let action = Action::OneoffCommandRequested {
                    alias: self.alias.clone(),
                    run_id: Some(run_id.clone()),
                    command: *command,
                };
                super::driver_confirm::dispatch(ctx, action, &self.alias);
                ScreenAction::Pop
            }
            KeyCode::Char('n') | KeyCode::Esc => match from {
                AskFrom::Viewer => ScreenAction::Pop,
                AskFrom::List => {
                    self.mode = OneoffMode::List;
                    ScreenAction::None
                }
                AskFrom::Result => {
                    self.mode = OneoffMode::Result;
                    ScreenAction::None
                }
            },
            _ => ScreenAction::None,
        }
    }

    /// The diff lines of the selected run, when they have loaded and are its own.
    fn diff_lines<'a>(&self, ctx: &'a AppContext) -> Option<&'a Result<Vec<String>, String>> {
        let id = self.selected(ctx).map(|v| v.record.run_id.as_str())?;
        ctx.view_cache
            .get(&self.alias)
            .and_then(|c| c.oneoff_diff.as_ref())
            .filter(|(run, _)| run == id)
            .map(|(_, result)| result)
    }

    /// `j`/`k`/PgUp/PgDn scroll the diff, Esc returns to the run's summary.
    fn diff_key(&mut self, code: KeyCode, ctx: &AppContext) -> ScreenAction {
        let OneoffMode::Diff { scroll } = self.mode else {
            return ScreenAction::None;
        };
        let total = match self.diff_lines(ctx) {
            Some(Ok(lines)) => lines.len(),
            _ => 0,
        };
        let page = self.diff_height.get().max(2) - 1;
        let bottom = total.saturating_sub(self.diff_height.get().max(1));
        let scroll = scroll.min(bottom);
        let next = match code {
            KeyCode::Esc | KeyCode::Char('q') => {
                self.mode = OneoffMode::Result;
                return ScreenAction::None;
            }
            KeyCode::Char('j') | KeyCode::Down => scroll.saturating_add(1),
            KeyCode::Char('k') | KeyCode::Up => scroll.saturating_sub(1),
            KeyCode::PageDown => scroll.saturating_add(page),
            KeyCode::PageUp => scroll.saturating_sub(page),
            KeyCode::Home | KeyCode::Char('g') => 0,
            KeyCode::End | KeyCode::Char('G') => bottom,
            // klf C-21: the full diff, written to a file and opened in $EDITOR. Only
            // while a merge diff is shown (there is a merge commit to export).
            KeyCode::Char('o') => {
                if let (Some(Ok(_)), Some(run_id)) = (
                    self.diff_lines(ctx),
                    self.selected(ctx).map(|v| v.record.run_id.clone()),
                ) {
                    return ScreenAction::DispatchAction(Box::new(
                        Action::OneoffDiffExportRequested {
                            alias: self.alias.clone(),
                            run_id,
                        },
                    ));
                }
                scroll
            }
            _ => scroll,
        };
        self.mode = OneoffMode::Diff {
            scroll: next.min(bottom),
        };
        ScreenAction::None
    }

    /// The diff popup's title and the lines of the visible window.
    fn diff_body(
        &self,
        ctx: &AppContext,
        scroll: usize,
        rows: usize,
    ) -> (String, Vec<Line<'static>>) {
        let id = self
            .selected(ctx)
            .map_or_else(String::new, |v| shown(&v.record.run_id));
        match self.diff_lines(ctx) {
            None => (
                format!(" Merge diff: {id} "),
                vec![Line::from("Reading the diff...")],
            ),
            Some(Err(why)) => (
                format!(" Merge diff: {id} "),
                vec![Line::from(format!(
                    "The diff could not be read: {}",
                    shown(why)
                ))],
            ),
            Some(Ok(lines)) => {
                let bottom = lines.len().saturating_sub(rows.max(1));
                let top = scroll.min(bottom);
                let end = (top + rows).min(lines.len());
                let title = format!(
                    " Merge diff: {id} - lines {}-{} of {} - j/k PgUp/PgDn g/G, o full diff, Esc back ",
                    if lines.is_empty() { 0 } else { top + 1 },
                    end,
                    lines.len()
                );
                let body = lines[top..end].iter().map(|line| diff_line(line)).collect();
                (title, body)
            }
        }
    }

    /// The popup's title and body for the current mode; `avail` is the rows the popup
    /// can show at most.
    fn body(&self, ctx: &AppContext, avail: usize) -> (String, Vec<Line<'static>>) {
        let alias = shown(&self.alias);
        let hint = |text: &str| {
            Line::from(Span::styled(
                text.to_owned(),
                Style::default().fg(Color::DarkGray),
            ))
        };
        match &self.mode {
            OneoffMode::Picker { selected } => {
                let mut lines = vec![
                    Line::from(format!("Run one sandboxed task on {alias}:")),
                    Line::from(""),
                ];
                let listing = ctx
                    .view_cache
                    .get(&self.alias)
                    .and_then(|c| c.oneoff_todos.as_ref());
                let todos = self.todos(ctx);
                match listing {
                    None => lines.push(hint("Loading the pending todos...")),
                    Some(Err(why)) => lines.push(Line::from(format!(
                        "The pending todos could not be listed: {}",
                        shown(why)
                    ))),
                    Some(Ok(list)) if list.is_empty() => lines.push(hint("No pending todos.")),
                    Some(Ok(_)) => {}
                }
                let rows = todos.len() + 1;
                let selected = (*selected).min(rows - 1);
                for (i, todo) in todos.iter().enumerate() {
                    lines.push(row_line(i == selected, &shown(&todo.title)));
                }
                lines.push(row_line(selected == todos.len(), TYPE_ROW));
                lines.push(Line::from(""));
                lines.push(hint("q /gsd-quick   f /gsd-fast   j/k move   Esc cancel"));
                (format!(" One-off task: {alias} "), lines)
            }
            OneoffMode::Input {
                mode,
                buffer,
                error,
            } => {
                let mut lines = vec![
                    Line::from(format!("{} task for {alias}:", mode.command())),
                    Line::from(format!("> {}_", shown(buffer))),
                ];
                if let Some(error) = error {
                    lines.push(Line::from(Span::styled(
                        shown(error),
                        Style::default().fg(Color::Red),
                    )));
                }
                lines.push(Line::from(""));
                lines.push(hint("Enter continue   Esc back"));
                (format!(" One-off task: {alias} "), lines)
            }
            OneoffMode::Confirm { mode, task, model } => {
                let defaults = crate::cli::task_run_defaults();
                let task_text = match task {
                    TaskSpec::Todo { file_name, title } => {
                        format!("{} ({})", shown(title), shown(file_name))
                    }
                    TaskSpec::Text(text) => shown(text),
                    TaskSpec::Backlog { number, title, .. } => {
                        format!("backlog {}: {}", shown(number), shown(title))
                    }
                };
                let lines = vec![
                    Line::from(format!("Command:       {}", mode.command())),
                    Line::from(format!("Task:          {task_text}")),
                    Line::from(format!("Model:         {} (m to change)", shown(model))),
                    Line::from(format!("Spend ceiling: ${:.2}", defaults.max_budget_usd)),
                    Line::from(format!("Wall clock:    {} min", defaults.cap_minutes)),
                    Line::from(""),
                    Line::from(Span::styled(
                        UNCOMMITTED_COPY,
                        Style::default().fg(Color::Yellow),
                    )),
                    Line::from(""),
                    hint("Enter launch   m model   Esc cancel"),
                ];
                (format!(" Start a one-off on {alias} "), lines)
            }
            OneoffMode::List => {
                let runs = self.runs(ctx);
                let lines = if runs.is_empty() {
                    vec![Line::from(NO_RUN_COPY)]
                } else {
                    list_lines(runs, self.selected_index(runs), avail)
                };
                (format!(" One-offs: {alias} "), lines)
            }
            OneoffMode::Result => {
                let lines = match self.selected(ctx) {
                    None => vec![Line::from(NO_RUN_COPY)],
                    Some(view) => result_lines(view),
                };
                (format!(" One-off: {alias} "), lines)
            }
            OneoffMode::Ask { prompt, .. } => {
                let mut lines: Vec<Line<'static>> =
                    prompt.iter().map(|l| Line::from(l.clone())).collect();
                lines.push(Line::from(""));
                lines.push(hint("y yes   n no"));
                (format!(" One-off: {alias} "), lines)
            }
            OneoffMode::ResumeAsk { run_id, .. } => {
                let id = shown(run_id);
                (
                    format!(" One-off: {alias} "),
                    vec![
                        Line::from(format!(
                            "Resume {id} as a new run? It starts a fresh session primed with the \
                             task, the report and the commits so far."
                        )),
                        Line::from(
                            "b from its branch   M merge the checkout's branch into it first \
                             (conflicts are resolved by a sandboxed agent)",
                        ),
                        Line::from(""),
                        hint("b branch   M merge master first   Esc cancel"),
                    ],
                )
            }
            OneoffMode::Diff { scroll } => self.diff_body(ctx, *scroll, avail),
        }
    }
}

fn row_line(selected: bool, text: &str) -> Line<'static> {
    if selected {
        Line::from(Span::styled(
            format!("> {text}"),
            Style::default().add_modifier(Modifier::BOLD),
        ))
    } else {
        Line::from(format!("  {text}"))
    }
}

/// One diff line, coloured by its first characters; escaped like everything drawn.
fn diff_line(line: &str) -> Line<'static> {
    let text = render_for_terminal(line).to_string();
    let style = if text.starts_with("+++") || text.starts_with("---") || text.starts_with("diff ") {
        Style::default().add_modifier(Modifier::BOLD)
    } else if text.starts_with('+') {
        Style::default().fg(Color::Green)
    } else if text.starts_with('-') {
        Style::default().fg(Color::Red)
    } else if text.starts_with("@@") {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default()
    };
    Line::from(Span::styled(text, style))
}

/// The landing column of a run list row: what landed it, `landing` while a landing is
/// in progress, `land failed` after a failed one, else `-`.
fn landing_label(view: &OneoffView) -> String {
    if view.landing {
        return "landing".to_owned();
    }
    match view.record.landed.last() {
        Some(Landing {
            kind: LandingKind::Pushed,
            ..
        }) => "pushed".to_owned(),
        Some(Landing {
            kind: LandingKind::AlreadyLanded,
            ..
        }) => "already".to_owned(),
        // A landing recorded before merge facts were kept was a fast-forward.
        Some(landing) => landing.how.unwrap_or(MergeHow::Ff).as_str().to_owned(),
        None if view.record.land_failure.is_some() => "land failed".to_owned(),
        None => "-".to_owned(),
    }
}

/// The run list: a header row, one row per run (id, state, mode, outcome or report
/// status, landing, headline) with the selected one marked, and the keys. At most
/// `avail` rows are drawn, windowed around the selection.
fn list_lines(runs: &[OneoffView], selected: usize, avail: usize) -> Vec<Line<'static>> {
    let hint = Line::from(Span::styled(
        "j/k select   Enter details   m merge   p push   D discard   x stop   X kill now   \
         z pause   u resume   e SUMMARY   v view   r resolver   d merge diff   Esc close",
        Style::default().fg(Color::DarkGray),
    ));
    // The title row, a blank line and the hint (which may wrap onto a second row).
    let fixed = 5;
    let window = avail.saturating_sub(fixed).max(1).min(runs.len());
    let top = selected.saturating_sub(window / 2).min(runs.len() - window);
    let mut lines = vec![Line::from(Span::styled(
        format!(
            "  {:<24} {:<9} {:<5} {:<11} {:<11} headline",
            "run", "state", "mode", "outcome", "landing"
        ),
        Style::default().fg(Color::DarkGray),
    ))];
    for (at, view) in runs.iter().enumerate().skip(top).take(window) {
        let record = &view.record;
        let outcome = match (&record.report, &record.outcome) {
            (Some(report), _) => report.status.as_str().to_owned(),
            (None, Some(outcome)) => outcome.word().to_owned(),
            (None, None) => "-".to_owned(),
        };
        let headline = record
            .report
            .as_ref()
            .map_or_else(String::new, |r| shown_to(&r.headline, ROW_HEADLINE_CHARS));
        let mode = record.mode.command().trim_start_matches("/gsd-");
        let row = format!(
            "{:<24} {:<9} {:<5} {:<11} {:<11} {}",
            shown_to(&record.run_id, 24),
            view.live.as_str(),
            mode,
            shown_to(&outcome, 11),
            shown_to(&landing_label(view), 11),
            headline
        );
        lines.push(row_line(at == selected, row.trim_end()));
    }
    lines.push(Line::from(""));
    lines.push(hint);
    lines
}

/// The merge prompt's sensitive-path warning (I-10).
fn merge_warning(view: &OneoffView) -> Vec<String> {
    #[cfg(any(target_os = "linux", target_os = "macos"))]
    {
        crate::oneoff::land::sensitive_warning(&view.record.changed_files)
            .iter()
            .map(|l| shown(l))
            .collect()
    }
    #[cfg(not(any(target_os = "linux", target_os = "macos")))]
    {
        let _ = view;
        Vec::new()
    }
}

/// The lines one recorded landing adds to a run's summary: where it landed, and for an
/// agent-resolved merge the `resolved by agent` marker with its conflicted files
/// (D-06, C-29).
fn landing_lines(landing: &Landing) -> Vec<Line<'static>> {
    if landing.kind == LandingKind::AlreadyLanded {
        let mut line = format!("Landed:    already in {}", shown(&landing.detail));
        if let Some(sha) = &landing.merge_sha {
            let short: String = sha.chars().take(12).collect();
            line.push_str(&format!(" (contained by {})", shown(&short)));
        }
        return vec![Line::from(line)];
    }
    let mut line = format!("Landed:    {:?} {}", landing.kind, shown(&landing.detail));
    if landing.kind == LandingKind::Merged {
        if let Some(sha) = &landing.merge_sha {
            let short: String = sha.chars().take(12).collect();
            line.push_str(&format!(" as {}", shown(&short)));
        }
        line.push_str(match landing.how {
            Some(MergeHow::Merge) => " (merge commit)",
            Some(MergeHow::Resolved) => " (merge commit, conflicts resolved)",
            Some(MergeHow::Ff) | None => " (fast-forward)",
        });
    }
    let mut lines = vec![Line::from(line)];
    if landing.kind == LandingKind::Merged {
        if let Some(range) = &landing.range {
            lines.push(Line::from(format!("Landed:    range {}", shown(range))));
        }
    }
    if landing.how == Some(MergeHow::Resolved) {
        lines.push(Line::from(Span::styled(
            "           resolved by agent - review it with d (merge diff) or r (its session)",
            Style::default().fg(Color::Yellow),
        )));
        lines.push(Line::from(format!(
            "           conflicted: {}",
            if landing.conflicted.is_empty() {
                "-".to_owned()
            } else {
                landing
                    .conflicted
                    .iter()
                    .map(|f| shown(f))
                    .collect::<Vec<_>>()
                    .join("; ")
            }
        )));
        if !landing.attempts.is_empty() {
            lines.push(Line::from(format!(
                "           attempts: {}",
                landing
                    .attempts
                    .iter()
                    .map(|a| shown(a))
                    .collect::<Vec<_>>()
                    .join("; ")
            )));
        }
    }
    lines
}

/// The question `m` asks, or why it asks nothing (quick 261103-o2r, D-C, I-16): the
/// merge names the branch it lands into. A run dispatched on a detached HEAD cannot be
/// landed from the TUI (it has no `--into` input).
pub(crate) fn merge_question(view: &OneoffView, id: &str) -> Result<String, String> {
    let how = "It fast-forwards when it can; otherwise it makes a merge commit in a private \
               clone, and conflicts are resolved by a sandboxed agent.";
    match view.record.land_target() {
        LandTarget::Branch(branch) => {
            let branch = shown(&branch);
            Ok(format!(
                "Merge {id} into {branch} (the branch HEAD pointed at when it was dispatched)? \
                 {how} If {branch} is checked out, only a clean checkout changes, at the very \
                 end; otherwise your working tree is not touched."
            ))
        }
        LandTarget::Legacy => Ok(format!(
            "Merge {id} into your checked-out branch? {how} Only a clean checkout changes, at \
             the very end."
        )),
        LandTarget::Detached { .. } => Err(format!(
            "{id} was dispatched on a detached HEAD; land it with `task land merge <project> \
             {id} --into <branch>`"
        )),
    }
}

/// The ~6 lines of a run's summary (C-25), the agent's report and the host's facts
/// labelled apart (I-18), then how it landed or why it did not.
pub fn result_lines(view: &OneoffView) -> Vec<Line<'static>> {
    let record = &view.record;
    let mut lines = vec![Line::from(format!(
        "Run {} ({}, {}, {})",
        shown(&record.run_id),
        view.live.as_str(),
        record.mode.command(),
        shown(&record.model)
    ))];
    let list = |items: &[String]| {
        if items.is_empty() {
            "-".to_owned()
        } else {
            items
                .iter()
                .map(|i| shown(i))
                .collect::<Vec<_>>()
                .join("; ")
        }
    };
    match &record.report {
        Some(report) => {
            let source = match &record.report_source {
                Some(ReportSource::Agent) => "agent-reported".to_owned(),
                Some(ReportSource::Synthesized { reason }) => {
                    format!("host-synthesized: {}", shown(reason))
                }
                None => "unknown source".to_owned(),
            };
            lines.push(Line::from(format!(
                "Status:    {} ({source})",
                report.status.as_str()
            )));
            lines.push(Line::from(format!(
                "Headline:  {}",
                shown(&report.headline)
            )));
            lines.push(Line::from(format!("Changed:   {}", list(&report.changed))));
            lines.push(Line::from(format!("Verified:  {}", list(&report.verified))));
            lines.push(Line::from(format!(
                "Followups: {}",
                list(&report.followups)
            )));
        }
        None => lines.push(Line::from("Status:    no report yet")),
    }
    lines.push(Line::from(format!(
        "Host:      outcome {}, {} commit(s), SUMMARY {}",
        record.outcome.as_ref().map_or("pending", |o| o.word()),
        record.commits.unwrap_or(0),
        if record.summary_path.is_some() {
            "present"
        } else {
            "absent"
        }
    )));
    if let Some(stop) = &record.stop {
        let pending = stop.ended_at.is_none() && view.live.may_be_live();
        let state = match (pending, stop.kind) {
            _ if stop.refused.is_some() => format!(
                "Pause refused ({})",
                shown(stop.refused.as_deref().unwrap_or_default())
            ),
            (true, StopKind::Pause) => "Pausing...".to_owned(),
            (true, StopKind::Hard) => "Stopping...".to_owned(),
            (true, StopKind::Kill) => "Killing...".to_owned(),
            (false, StopKind::Pause) => "Paused (pause)".to_owned(),
            (false, StopKind::Hard) => "Stopped (hard)".to_owned(),
            (false, StopKind::Kill) => "Killed (kill)".to_owned(),
        };
        lines.push(Line::from(format!(
            "Stop:      {state} by {} at {}{}",
            stop.by.word(),
            shown(&stop.requested_at),
            if stop.escalated { ", escalated" } else { "" }
        )));
    }
    if let Some(by) = &record.resumed_by {
        lines.push(Line::from(format!("Resumed as {}", shown(by))));
    }
    if let Some(from) = &record.resumed_from {
        lines.push(Line::from(format!("Resumed from {}", shown(from))));
    }
    if let Some(merge) = &record.resume_merge {
        let text = match (&merge.failure, &merge.merge_sha) {
            (Some(why), _) => format!(
                "Resume merge of {} failed: {}",
                shown(&merge.branch),
                shown(why)
            ),
            (None, Some(sha)) => format!(
                "Resume merge: {} merged as {}{}",
                shown(&merge.branch),
                shown(&sha.chars().take(12).collect::<String>()),
                if merge.conflicted.is_empty() {
                    String::new()
                } else {
                    format!(
                        " (resolved by agent: {})",
                        merge
                            .conflicted
                            .iter()
                            .map(|f| shown(f))
                            .collect::<Vec<_>>()
                            .join("; ")
                    )
                }
            ),
            (None, None) => format!("Resume merge: {}", shown(&merge.branch)),
        };
        lines.push(Line::from(text));
    }
    lines.push(Line::from(match &record.summary_path {
        Some(path) => format!("SUMMARY:   {}", shown(path)),
        None => "SUMMARY:   none".to_owned(),
    }));
    if let Some(spend) = record.spend_usd {
        lines.push(Line::from(format!(
            "Spend:     ${spend:.2} of ${:.2} (estimate, resolvers included)",
            record.max_budget_usd
        )));
    }
    // o2r D-C: where the run lands.
    lines.push(Line::from(format!(
        "Target:    {}",
        shown(&record.land_target_text())
    )));
    // klf C-17, C-18: only what the record normalises to; an old record whose merge
    // landing deleted the branch reads as gone.
    match record.branch_state() {
        BranchState::Tracked { name, head } => {
            let at = head
                .as_deref()
                .map(|h| format!(" @ {}", shown(&h.chars().take(12).collect::<String>())))
                .unwrap_or_default();
            lines.push(Line::from(format!(
                "Branch:    {}{at} (git branch --no-merged lists it)",
                shown(&name)
            )));
        }
        BranchState::Gone { why } => {
            lines.push(Line::from(format!("Branch:    none ({})", shown(&why))));
        }
        BranchState::Untracked { note } if !note.is_empty() => {
            lines.push(Line::from(format!("Branch:    none ({})", shown(&note))));
        }
        BranchState::Untracked { .. } => {}
    }
    if let Some(pruned) = &record.pruned {
        lines.push(Line::from(format!(
            "Clone:     {} ({})",
            shown(&pruned.note),
            shown(&pruned.at)
        )));
    }
    for landing in &record.landed {
        lines.extend(landing_lines(landing));
    }
    if view.landing {
        lines.push(Line::from(Span::styled(
            "Landing:   a merge landing of this run is in progress",
            Style::default().fg(Color::Yellow),
        )));
    }
    if let Some(failure) = &record.land_failure {
        lines.push(Line::from(Span::styled(
            format!("land failed: {}", shown(&failure.reason)),
            Style::default().fg(Color::Red),
        )));
        if !failure.conflicted.is_empty() {
            lines.push(Line::from(format!(
                "           conflicted: {}",
                list(&failure.conflicted)
            )));
        }
        if !failure.attempts.is_empty() {
            lines.push(Line::from(format!(
                "           attempts kept: {}",
                list(&failure.attempts)
            )));
        }
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "e SUMMARY   v live view   r resolver view   d merge diff   m merge   p push   \
         D discard   x stop   X kill now   z pause   u resume   Esc back",
        Style::default().fg(Color::DarkGray),
    )));
    lines
}

/// The Driver tab's one-line one-off status (C-24, quick 261103-hx7 C-26): the counts
/// of the project's listed runs, `one-offs: 2 running, 1 to land (T)`.
pub fn status_line(runs: &[OneoffView]) -> Line<'static> {
    Line::from(Span::styled(
        run_counts(runs).text(),
        Style::default().fg(Color::Cyan),
    ))
}

crate::ui::screens::adjudicate_screen!(
    OneoffScreen,
    crate::ui::screens::RENDERS_ATTACKER_INFLUENCED_IDENTITY,
    "Draws the registry key in every title, the pending todos' titles and file names \
     (repository text), the typed task, the model, the run list (run ids, report headlines), \
     one run's report fields (agent-written), SUMMARY path, conflicted file names and \
     land-failure reason, and the merge diff's lines (agent-written file contents), all \
     through render_for_terminal. Fixture states: picker with hostile todo titles, input \
     with a typed task, confirm with hostile text, list with hostile runs, result with a \
     hostile report and a resolved landing, diff with hostile lines.",
);

impl Screen for OneoffScreen {
    fn handle_key(
        &mut self,
        code: KeyCode,
        _modifiers: KeyModifiers,
        ctx: &mut AppContext,
    ) -> ScreenAction {
        ctx.needs_redraw = true;
        match self.mode {
            OneoffMode::Picker { .. } => self.picker_key(code, ctx),
            OneoffMode::Input { .. } => self.input_key(code),
            OneoffMode::Confirm { .. } => self.confirm_key(code, ctx),
            OneoffMode::List => self.list_key(code, ctx),
            OneoffMode::Result => self.result_key(code, ctx),
            OneoffMode::Ask { .. } => self.ask_key(code, ctx),
            OneoffMode::ResumeAsk { .. } => self.resume_ask_key(code, ctx),
            OneoffMode::Diff { .. } => self.diff_key(code, ctx),
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        super::detail::DetailScreen::new(self.alias.clone()).render_main_only(frame, area, ctx);
        super::confirm_popup::dim_background(frame, area);
        if matches!(self.mode, OneoffMode::Diff { .. }) {
            // The diff takes the screen: a window of its lines, no wrapping.
            let popup = Rect {
                x: area.x + 1,
                y: area.y,
                width: area.width.saturating_sub(2),
                height: area.height,
            };
            let rows = usize::from(popup.height.saturating_sub(2)).max(1);
            self.diff_height.set(rows);
            let (title, body) = self.body(ctx, rows);
            frame.render_widget(Clear, popup);
            frame.render_widget(
                Paragraph::new(body).block(Block::default().borders(Borders::ALL).title(title)),
                popup,
            );
            return;
        }
        let width = area.width.saturating_sub(4).min(100);
        let inner_width = width.saturating_sub(2).max(1);
        let avail = usize::from(area.height.saturating_sub(2)).max(1);
        let (title, body) = self.body(ctx, avail);
        let rows = super::confirm_popup::wrapped_rows(&body, inner_width);
        let height = rows.saturating_add(2).min(area.height);
        let popup = Rect {
            x: area.x + (area.width.saturating_sub(width)) / 2,
            y: area.y + (area.height.saturating_sub(height)) / 2,
            width,
            height,
        };
        frame.render_widget(Clear, popup);
        frame.render_widget(
            Paragraph::new(body)
                .wrap(Wrap { trim: false })
                .block(Block::default().borders(Borders::ALL).title(title)),
            popup,
        );
    }

    fn name(&self) -> &str {
        "oneoff"
    }
}

#[cfg(test)]
pub(crate) mod tests {
    use super::*;
    use crate::oneoff::report::{Report, ReportStatus};
    use crate::oneoff::store::{RecordStatus, RunRecord};
    use crate::oneoff::todos::PendingTodo;
    use crate::ui::screens::driver_confirm::tests::{ctx_with_project, ALIAS};
    use std::path::PathBuf;

    fn todo(name: &str, title: &str) -> PendingTodo {
        PendingTodo {
            file_name: name.to_owned(),
            title: title.to_owned(),
        }
    }

    pub(crate) fn finished_view(text: &str) -> OneoffView {
        let mut record = RunRecord::new(
            "oneoff-20261103-142233",
            ALIAS,
            PathBuf::from("/p"),
            Mode::Quick,
            TaskSpec::Text("x".to_owned()),
            "agent",
            "opus",
            10.0,
            60.0,
            chrono::Utc::now(),
        );
        record.status = RecordStatus::Finished;
        record.report = Some(Report {
            schema: 1,
            status: ReportStatus::Done,
            headline: format!("head {text}"),
            changed: vec![format!("changed {text}")],
            verified: vec![format!("verified: {text}")],
            followups: vec![format!("follow {text}")],
            at: "2026-11-03T10:00:00Z".to_owned(),
        });
        record.report_source = Some(ReportSource::Agent);
        record.summary_path = Some(format!(".planning/quick/{text}/x-SUMMARY.md"));
        record.commits = Some(2);
        record.head = Some("a".repeat(40));
        OneoffView {
            record,
            live: LiveState::Finished,
            run_dir: PathBuf::from("/data/oneoff/ab/oneoff-20261103-142233"),
            landing: false,
            land_attempts: Vec::new(),
            summary_file: None,
        }
    }

    const NEWEST: &str = "oneoff-20261103-150000";
    const MIDDLE: &str = "oneoff-20261103-142233";
    const OLDEST: &str = "oneoff-20261103-100000";

    fn landing(how: MergeHow, conflicted: &[&str]) -> Landing {
        Landing {
            kind: LandingKind::Merged,
            at: "2026-11-03T11:00:00Z".to_owned(),
            detail: "main".to_owned(),
            how: Some(how),
            merge_sha: Some("0123456789abcdef0123456789abcdef01234567".to_owned()),
            conflicted: conflicted.iter().map(|c| (*c).to_owned()).collect(),
            attempts: if how == MergeHow::Resolved {
                vec!["land/1".to_owned()]
            } else {
                Vec::new()
            },
            range: None,
            target_tip: None,
        }
    }

    /// Three runs, newest first: a live one, a finished one nothing has landed, and an
    /// older one an agent-resolved merge landed.
    fn three_runs() -> Vec<OneoffView> {
        let mut newest = finished_view("newest");
        newest.record.run_id = NEWEST.to_owned();
        newest.live = LiveState::Running;
        newest.record.report = None;
        let mut middle = finished_view("middle");
        middle.record.run_id = MIDDLE.to_owned();
        let mut oldest = finished_view("oldest");
        oldest.record.run_id = OLDEST.to_owned();
        oldest
            .record
            .landed
            .push(landing(MergeHow::Resolved, &["shared.txt"]));
        oldest.land_attempts = vec![1, 2];
        vec![newest, middle, oldest]
    }

    fn set_runs(ctx: &mut AppContext, runs: Vec<OneoffView>) {
        ctx.view_cache
            .entry(ALIAS.to_owned())
            .or_default()
            .oneoff_runs = runs;
    }

    fn list_over(runs: Vec<OneoffView>) -> (AppContext, OneoffScreen) {
        let dir = tempfile::tempdir().unwrap();
        let (mut ctx, _rx) = ctx_with_project(dir.path());
        set_runs(&mut ctx, runs);
        (ctx, OneoffScreen::list(ALIAS.to_owned()))
    }

    fn render_text(screen: &OneoffScreen, ctx: &AppContext) -> String {
        use ratatui::backend::TestBackend;
        use ratatui::Terminal;
        let mut t = Terminal::new(TestBackend::new(120, 40)).unwrap();
        t.draw(|f| screen.render(f, f.area(), ctx)).unwrap();
        let buf = t.backend().buffer();
        (0..40u16)
            .map(|y| {
                (0..120u16)
                    .map(|x| buf[(x, y)].symbol())
                    .collect::<String>()
            })
            .collect::<Vec<_>>()
            .join("\n")
    }

    fn key(screen: &mut OneoffScreen, ctx: &mut AppContext, code: KeyCode) -> ScreenAction {
        screen.handle_key(code, KeyModifiers::NONE, ctx)
    }

    fn dispatched(action: ScreenAction) -> Action {
        match action {
            ScreenAction::DispatchAction(action) => *action,
            _ => panic!("expected a dispatched action"),
        }
    }

    #[test]
    fn the_list_shows_runs_newest_first_with_their_outcome_and_landing() {
        let (ctx, screen) = list_over(three_runs());
        let text = render_text(&screen, &ctx);
        let at = |needle: &str| {
            text.find(needle)
                .unwrap_or_else(|| panic!("{needle}:\n{text}"))
        };
        assert!(at(NEWEST) < at(MIDDLE) && at(MIDDLE) < at(OLDEST), "{text}");
        // Finished runs show their report status, the landing column and the headline.
        for needle in [
            "running",
            "finished",
            "done",
            "resolved",
            "head oldest",
            "quick",
        ] {
            assert!(text.contains(needle), "{needle}:\n{text}");
        }
        // The first row is selected, and the keys are named.
        assert!(text.contains(&format!("> {NEWEST}")), "{text}");
        assert!(text.contains("Enter details") && text.contains("D discard"));
        assert!(render_text(&OneoffScreen::list(ALIAS.to_owned()), &ctx).contains(NEWEST));
    }

    #[test]
    fn an_empty_list_says_there_is_no_run() {
        let (ctx, screen) = list_over(Vec::new());
        assert!(render_text(&screen, &ctx).contains(NO_RUN_COPY));
    }

    #[test]
    fn j_and_k_move_clamp_and_a_reorder_keeps_the_selection_by_id() {
        let (mut ctx, mut screen) = list_over(three_runs());
        key(&mut screen, &mut ctx, KeyCode::Char('k'));
        assert_eq!(
            screen.selected_index(screen.runs(&ctx)),
            0,
            "clamped at the top"
        );
        key(&mut screen, &mut ctx, KeyCode::Char('j'));
        key(&mut screen, &mut ctx, KeyCode::Down);
        key(&mut screen, &mut ctx, KeyCode::Char('j'));
        assert_eq!(screen.run_id.as_deref(), Some(OLDEST), "clamped at the end");
        key(&mut screen, &mut ctx, KeyCode::Up);
        assert_eq!(screen.run_id.as_deref(), Some(MIDDLE));

        // A refresh puts a newer run on top: the selection stays on MIDDLE.
        let mut runs = three_runs();
        let mut newer = finished_view("newer");
        newer.record.run_id = "oneoff-20261103-160000".to_owned();
        runs.insert(0, newer);
        set_runs(&mut ctx, runs);
        assert_eq!(screen.selected_index(screen.runs(&ctx)), 2);
        assert_eq!(
            screen.selected(&ctx).map(|v| v.record.run_id.as_str()),
            Some(MIDDLE)
        );
        // A run that vanished falls back to the newest in the list.
        set_runs(&mut ctx, vec![three_runs().remove(0)]);
        assert_eq!(
            screen.selected(&ctx).map(|v| v.record.run_id.as_str()),
            Some(NEWEST)
        );
    }

    #[test]
    fn enter_shows_the_run_and_esc_walks_back_to_the_list_then_closes() {
        let (mut ctx, mut screen) = list_over(three_runs());
        key(&mut screen, &mut ctx, KeyCode::Char('j'));
        key(&mut screen, &mut ctx, KeyCode::Enter);
        assert_eq!(screen.mode, OneoffMode::Result);
        let text = render_text(&screen, &ctx);
        assert!(
            text.contains(&format!("Run {MIDDLE}")) && text.contains("head middle"),
            "{text}"
        );
        assert!(matches!(
            key(&mut screen, &mut ctx, KeyCode::Esc),
            ScreenAction::None
        ));
        assert_eq!(screen.mode, OneoffMode::List);
        assert_eq!(screen.run_id.as_deref(), Some(MIDDLE));
        assert!(matches!(
            key(&mut screen, &mut ctx, KeyCode::Esc),
            ScreenAction::Pop
        ));
    }

    #[test]
    fn v_opens_the_live_viewer_of_the_selected_run_only() {
        let dir = tempfile::tempdir().unwrap();
        let (mut ctx, _rx) = ctx_with_project(dir.path());
        let mut screen = OneoffScreen::list(ALIAS.to_owned());
        assert!(matches!(
            key(&mut screen, &mut ctx, KeyCode::Char('v')),
            ScreenAction::None
        ));
        set_runs(&mut ctx, three_runs());
        key(&mut screen, &mut ctx, KeyCode::Char('j'));
        match dispatched(key(&mut screen, &mut ctx, KeyCode::Char('v'))) {
            Action::OneoffViewerOpen {
                alias,
                run_id,
                attempt,
            } => {
                assert_eq!(alias, ALIAS);
                assert_eq!(run_id, MIDDLE);
                assert_eq!(attempt, None);
            }
            other => panic!("{other:?}"),
        }
        key(&mut screen, &mut ctx, KeyCode::Enter);
        assert!(render_text(&screen, &ctx).contains("v live view"));
    }

    #[test]
    fn r_opens_the_newest_resolver_session_or_says_there_is_none() {
        let (mut ctx, mut screen) = list_over(three_runs());
        // The newest run has no resolver session.
        key(&mut screen, &mut ctx, KeyCode::Char('r'));
        let message = ctx.status_message.take().expect("a notice").0;
        assert!(message.contains("No resolver session"), "{message}");
        key(&mut screen, &mut ctx, KeyCode::Char('j'));
        key(&mut screen, &mut ctx, KeyCode::Char('j'));
        match dispatched(key(&mut screen, &mut ctx, KeyCode::Char('r'))) {
            Action::OneoffViewerOpen {
                run_id, attempt, ..
            } => {
                assert_eq!(run_id, OLDEST);
                assert_eq!(attempt, Some(2), "the newest attempt");
            }
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn landing_keys_ask_first_and_name_the_selected_run() {
        let dir = tempfile::tempdir().unwrap();
        let (mut ctx, mut rx) = ctx_with_project(dir.path());
        set_runs(&mut ctx, three_runs());
        for (k, expected) in [
            ('m', OneoffCommand::Merge),
            ('p', OneoffCommand::Push),
            ('D', OneoffCommand::Discard),
        ] {
            // From the list: the second row is the one asked about.
            let mut screen = OneoffScreen::list(ALIAS.to_owned());
            key(&mut screen, &mut ctx, KeyCode::Char('j'));
            key(&mut screen, &mut ctx, KeyCode::Char(k));
            assert!(rx.try_recv().is_err(), "{k} must ask first");
            key(&mut screen, &mut ctx, KeyCode::Char('n'));
            assert!(rx.try_recv().is_err(), "n dispatches nothing");
            assert_eq!(screen.mode, OneoffMode::List, "n returns to the list");
            key(&mut screen, &mut ctx, KeyCode::Char(k));
            key(&mut screen, &mut ctx, KeyCode::Char('y'));
            let Ok(Action::OneoffCommandRequested {
                command, run_id, ..
            }) = rx.try_recv()
            else {
                panic!("y must dispatch {expected:?}");
            };
            assert_eq!(command, expected);
            assert_eq!(run_id.as_deref(), Some(MIDDLE));

            // From a run's own summary: n returns to the summary.
            let mut screen = OneoffScreen::list(ALIAS.to_owned());
            key(&mut screen, &mut ctx, KeyCode::Enter);
            key(&mut screen, &mut ctx, KeyCode::Char(k));
            key(&mut screen, &mut ctx, KeyCode::Esc);
            assert_eq!(screen.mode, OneoffMode::Result);
        }
        // The old discard key is not discard any more: `d` is the merge diff.
        let mut screen = OneoffScreen::list(ALIAS.to_owned());
        key(&mut screen, &mut ctx, KeyCode::Char('d'));
        assert!(!matches!(screen.mode, OneoffMode::Ask { .. }));

        // `x` on a finished run kills nothing...
        let mut screen = OneoffScreen::list(ALIAS.to_owned());
        key(&mut screen, &mut ctx, KeyCode::Char('j'));
        ctx.status_message = None;
        key(&mut screen, &mut ctx, KeyCode::Char('x'));
        assert_eq!(screen.mode, OneoffMode::List);
        assert!(ctx.status_message.is_some());
        // ...and on the live one it asks, then stops exactly that run.
        let mut screen = OneoffScreen::list(ALIAS.to_owned());
        key(&mut screen, &mut ctx, KeyCode::Char('x'));
        key(&mut screen, &mut ctx, KeyCode::Char('y'));
        match rx.try_recv() {
            Ok(Action::OneoffCommandRequested {
                command: OneoffCommand::Stop,
                run_id,
                ..
            }) => assert_eq!(run_id.as_deref(), Some(NEWEST)),
            other => panic!("{other:?}"),
        }
    }

    #[test]
    fn x_big_x_and_z_ask_about_a_live_run_and_dispatch_the_right_command() {
        let dir = tempfile::tempdir().unwrap();
        let (mut ctx, mut rx) = ctx_with_project(dir.path());
        set_runs(&mut ctx, three_runs());
        for (code, expected) in [
            ('X', OneoffCommand::Kill),
            ('z', OneoffCommand::Pause),
            ('x', OneoffCommand::Stop),
        ] {
            let mut screen = OneoffScreen::list(ALIAS.to_owned());
            key(&mut screen, &mut ctx, KeyCode::Char(code));
            match &screen.mode {
                OneoffMode::Ask {
                    command, run_id, ..
                } => {
                    assert_eq!(*command, expected);
                    assert_eq!(run_id, NEWEST);
                }
                other => panic!("{other:?}"),
            }
            // From the run's own summary the same keys ask the same thing.
            key(&mut screen, &mut ctx, KeyCode::Char('n'));
            assert_eq!(screen.mode, OneoffMode::List);
            key(&mut screen, &mut ctx, KeyCode::Char(code));
            key(&mut screen, &mut ctx, KeyCode::Char('y'));
            match rx.try_recv() {
                Ok(Action::OneoffCommandRequested {
                    command, run_id, ..
                }) => {
                    assert_eq!(command, expected);
                    assert_eq!(run_id.as_deref(), Some(NEWEST));
                }
                other => panic!("{other:?}"),
            }
        }
        // `z` and `X` on a finished run with no landing are refused with a message.
        let mut screen = OneoffScreen::list(ALIAS.to_owned());
        key(&mut screen, &mut ctx, KeyCode::Char('j'));
        for code in ['z', 'X'] {
            ctx.status_message = None;
            key(&mut screen, &mut ctx, KeyCode::Char(code));
            assert_eq!(screen.mode, OneoffMode::List);
            assert!(
                ctx.status_message
                    .as_ref()
                    .is_some_and(|(m, _)| m.contains("nothing to")),
                "{code}"
            );
        }
    }

    #[test]
    fn on_a_run_that_is_not_live_but_being_landed_the_keys_ask_about_its_landing() {
        let mut runs = three_runs();
        runs[1].landing = true;
        let dir = tempfile::tempdir().unwrap();
        let (mut ctx, mut rx) = ctx_with_project(dir.path());
        set_runs(&mut ctx, runs);
        for (code, expected) in [
            ('x', OneoffCommand::StopLanding),
            ('X', OneoffCommand::KillLanding),
            ('z', OneoffCommand::PauseLanding),
        ] {
            let mut screen = OneoffScreen::list(ALIAS.to_owned());
            key(&mut screen, &mut ctx, KeyCode::Char('j'));
            key(&mut screen, &mut ctx, KeyCode::Char(code));
            let OneoffMode::Ask {
                prompt, command, ..
            } = &screen.mode
            else {
                panic!("{:?}", screen.mode);
            };
            assert_eq!(*command, expected);
            assert!(prompt[0].contains("landing"), "{prompt:?}");
            key(&mut screen, &mut ctx, KeyCode::Char('y'));
            match rx.try_recv() {
                Ok(Action::OneoffCommandRequested {
                    command, run_id, ..
                }) => {
                    assert_eq!(command, expected);
                    assert_eq!(run_id.as_deref(), Some(MIDDLE));
                }
                other => panic!("{other:?}"),
            }
        }
    }

    #[test]
    fn the_summary_names_a_pending_and_a_finished_stop() {
        use crate::oneoff::cancel::{StopBy, StopRecord};
        let mut runs = three_runs();
        runs[0].record.stop = Some(StopRecord {
            kind: StopKind::Pause,
            requested_at: "2026-11-03T10:00:00Z".to_owned(),
            ended_at: None,
            escalated: false,
            by: StopBy::Tui,
            refused: None,
        });
        runs[1].record.stop = Some(StopRecord {
            kind: StopKind::Hard,
            requested_at: "2026-11-03T10:00:00Z".to_owned(),
            ended_at: Some("2026-11-03T10:00:12Z".to_owned()),
            escalated: true,
            by: StopBy::Cli,
            refused: None,
        });
        runs[2].record.stop = Some(StopRecord {
            kind: StopKind::Pause,
            requested_at: "2026-11-03T10:00:00Z".to_owned(),
            ended_at: Some("2026-11-03T10:00:01Z".to_owned()),
            escalated: false,
            by: StopBy::Cli,
            refused: Some("the session is ending".to_owned()),
        });
        let (mut ctx, mut screen) = list_over(runs);
        key(&mut screen, &mut ctx, KeyCode::Enter);
        let text = render_text(&screen, &ctx);
        assert!(text.contains("Pausing..."), "{text}");
        let mut screen = OneoffScreen::list(ALIAS.to_owned());
        key(&mut screen, &mut ctx, KeyCode::Char('j'));
        key(&mut screen, &mut ctx, KeyCode::Enter);
        let text = render_text(&screen, &ctx);
        assert!(text.contains("Stopped (hard) by cli"), "{text}");
        assert!(text.contains("escalated"), "{text}");
        let mut screen = OneoffScreen::list(ALIAS.to_owned());
        key(&mut screen, &mut ctx, KeyCode::Char('j'));
        key(&mut screen, &mut ctx, KeyCode::Char('j'));
        key(&mut screen, &mut ctx, KeyCode::Enter);
        let text = render_text(&screen, &ctx);
        assert!(
            text.contains("Pause refused (the session is ending) by cli"),
            "{text}"
        );
        assert!(!text.contains("Pausing..."), "{text}");
    }

    #[test]
    fn u_asks_b_or_big_m_for_a_paused_run_and_dispatches_the_resume() {
        use crate::oneoff::resume::ResumeFrom;
        use crate::oneoff::AgentOutcome;
        let mut runs = three_runs();
        runs[1].record.outcome = Some(AgentOutcome::Paused);
        runs[1].record.head = Some("a".repeat(40));
        runs[1].record.commits = Some(2);
        runs[1].record.tracked_branch = Some(format!("refs/heads/gsd-oneoff/{MIDDLE}"));
        let dir = tempfile::tempdir().unwrap();
        let (mut ctx, mut rx) = ctx_with_project(dir.path());
        set_runs(&mut ctx, runs);
        for (code, expected) in [('b', ResumeFrom::Branch), ('M', ResumeFrom::Master)] {
            let mut screen = OneoffScreen::list(ALIAS.to_owned());
            key(&mut screen, &mut ctx, KeyCode::Char('j'));
            key(&mut screen, &mut ctx, KeyCode::Char('u'));
            assert!(matches!(screen.mode, OneoffMode::ResumeAsk { .. }));
            assert!(render_text(&screen, &ctx).contains("M merge master first"));
            // Esc returns to the list.
            key(&mut screen, &mut ctx, KeyCode::Esc);
            assert_eq!(screen.mode, OneoffMode::List);
            key(&mut screen, &mut ctx, KeyCode::Char('u'));
            key(&mut screen, &mut ctx, KeyCode::Char(code));
            match rx.try_recv() {
                Ok(Action::OneoffResumeRequested { run_id, from, .. }) => {
                    assert_eq!(run_id, MIDDLE);
                    assert_eq!(from, expected);
                }
                other => panic!("{other:?}"),
            }
        }
    }

    #[test]
    fn u_on_a_paused_landing_asks_to_merge_and_on_a_finished_run_refuses() {
        use crate::oneoff::cancel::StopKind;
        let mut runs = three_runs();
        runs[1].record.land_failure = Some(crate::oneoff::store::LandFailure {
            at: "2026-11-03T11:00:00Z".to_owned(),
            reason: "paused".to_owned(),
            conflicted: Vec::new(),
            attempts: vec!["land/1".to_owned()],
            stop: Some(StopKind::Pause),
        });
        let (mut ctx, mut screen) = list_over(runs);
        key(&mut screen, &mut ctx, KeyCode::Char('j'));
        key(&mut screen, &mut ctx, KeyCode::Char('u'));
        assert!(matches!(
            screen.mode,
            OneoffMode::Ask {
                command: OneoffCommand::Merge,
                ..
            }
        ));
        // OLDEST finished on its own (and was landed): refused with a message.
        let mut screen = OneoffScreen::list(ALIAS.to_owned());
        key(&mut screen, &mut ctx, KeyCode::Char('j'));
        key(&mut screen, &mut ctx, KeyCode::Char('j'));
        ctx.status_message = None;
        key(&mut screen, &mut ctx, KeyCode::Char('u'));
        assert_eq!(screen.mode, OneoffMode::List);
        assert!(ctx
            .status_message
            .as_ref()
            .is_some_and(|(m, _)| m.contains("cannot be resumed")));
    }

    #[test]
    fn the_summary_names_the_resume_links_and_the_resume_merge() {
        use crate::oneoff::store::ResumeMerge;
        let mut runs = three_runs();
        runs[0].record.resumed_from = Some("oneoff-20261103-090000".to_owned());
        runs[0].record.resume_merge = Some(ResumeMerge {
            how: Some(MergeHow::Resolved),
            branch: "main".to_owned(),
            merge_sha: Some("0123456789abcdef0123456789abcdef01234567".to_owned()),
            conflicted: vec!["shared.txt".to_owned()],
            attempts: vec!["land/1".to_owned()],
            failure: None,
        });
        runs[1].record.resumed_by = Some(NEWEST.to_owned());
        let (mut ctx, mut screen) = list_over(runs);
        key(&mut screen, &mut ctx, KeyCode::Enter);
        let text = render_text(&screen, &ctx);
        assert!(
            text.contains("Resumed from oneoff-20261103-090000"),
            "{text}"
        );
        assert!(
            text.contains("Resume merge: main merged as 0123456789ab"),
            "{text}"
        );
        assert!(text.contains("resolved by agent: shared.txt"), "{text}");
        let mut screen = OneoffScreen::list(ALIAS.to_owned());
        key(&mut screen, &mut ctx, KeyCode::Char('j'));
        key(&mut screen, &mut ctx, KeyCode::Enter);
        assert!(render_text(&screen, &ctx).contains(&format!("Resumed as {NEWEST}")));
    }

    #[test]
    fn m_is_refused_while_a_landing_of_that_run_is_in_progress() {
        let mut runs = three_runs();
        runs[1].landing = true;
        let (mut ctx, mut screen) = list_over(runs);
        key(&mut screen, &mut ctx, KeyCode::Char('j'));
        key(&mut screen, &mut ctx, KeyCode::Char('m'));
        assert_eq!(screen.mode, OneoffMode::List);
        assert!(ctx
            .status_message
            .as_ref()
            .is_some_and(|(m, _)| m.contains("already in progress")));
        assert!(render_text(&screen, &ctx).contains("landing"));
    }

    #[test]
    fn the_summary_shows_the_target_and_a_pruned_clone() {
        let tip = "0123456789abcdef0123456789abcdef01234567".to_owned();
        let mut runs = three_runs();
        runs[1].record.dispatch_tip = Some(tip.clone());
        runs[1].record.dispatch_branch = Some("main".to_owned());
        runs[1].record.pruned = Some(crate::oneoff::store::Pruned {
            at: "2026-11-03T12:00:00Z".to_owned(),
            note: "removed after landing in main".to_owned(),
        });
        let (mut ctx, mut screen) = list_over(runs);
        key(&mut screen, &mut ctx, KeyCode::Char('j'));
        key(&mut screen, &mut ctx, KeyCode::Enter);
        let text = render_text(&screen, &ctx);
        assert!(text.contains("Target:    main"), "{text}");
        assert!(
            text.contains("Clone:     removed after landing in main (2026-11-03T12:00:00Z)"),
            "{text}"
        );
        // The other two shapes.
        let mut runs = three_runs();
        runs[1].record.dispatch_tip = Some(tip);
        runs[1].record.dispatch_detached = true;
        set_runs(&mut ctx, runs);
        let mut screen = OneoffScreen::list(ALIAS.to_owned());
        key(&mut screen, &mut ctx, KeyCode::Char('j'));
        key(&mut screen, &mut ctx, KeyCode::Enter);
        assert!(render_text(&screen, &ctx).contains("Target:    detached @0123456"));
        set_runs(&mut ctx, three_runs());
        let mut screen = OneoffScreen::list(ALIAS.to_owned());
        key(&mut screen, &mut ctx, KeyCode::Char('j'));
        key(&mut screen, &mut ctx, KeyCode::Enter);
        let text = render_text(&screen, &ctx);
        assert!(
            text.contains("Target:    checked-out branch (legacy)"),
            "{text}"
        );
        assert!(!text.contains("Clone:"), "{text}");
    }

    #[test]
    fn the_merge_question_names_the_target_and_a_detached_one_asks_nothing() {
        let mut runs = three_runs();
        runs[1].record.dispatch_tip = Some("a".repeat(40));
        runs[1].record.dispatch_branch = Some("release".to_owned());
        let (mut ctx, mut screen) = list_over(runs.clone());
        key(&mut screen, &mut ctx, KeyCode::Char('j'));
        key(&mut screen, &mut ctx, KeyCode::Char('m'));
        let text = render_text(&screen, &ctx);
        assert!(text.contains("into release"), "{text}");
        assert!(text.contains("working tree is not touched"), "{text}");
        assert!(matches!(screen.mode, OneoffMode::Ask { .. }));

        runs[1].record.dispatch_branch = None;
        runs[1].record.dispatch_detached = true;
        set_runs(&mut ctx, runs);
        let mut screen = OneoffScreen::list(ALIAS.to_owned());
        key(&mut screen, &mut ctx, KeyCode::Char('j'));
        key(&mut screen, &mut ctx, KeyCode::Char('m'));
        assert_eq!(screen.mode, OneoffMode::List, "nothing is asked");
        let refusal = ctx.status_message.as_ref().map(|(m, _)| m.clone());
        assert!(
            refusal.is_some_and(|m| m.contains("task land merge") && m.contains("--into")),
            "the refusal names the CLI"
        );

        let (mut ctx, mut screen) = list_over(three_runs());
        key(&mut screen, &mut ctx, KeyCode::Char('j'));
        key(&mut screen, &mut ctx, KeyCode::Char('m'));
        assert!(render_text(&screen, &ctx).contains("checked-out branch"));
    }

    #[test]
    fn an_already_landed_run_reads_already_in_the_list_and_contained_by_in_the_summary() {
        let mut view = finished_view("x");
        view.record.landed.clear();
        view.record.landed.push(Landing {
            merge_sha: Some("0123456789abcdef0123456789abcdef01234567".to_owned()),
            target_tip: Some("f".repeat(40)),
            ..Landing::plain(
                LandingKind::AlreadyLanded,
                "2026-11-03T11:00:00Z".to_owned(),
                "main".to_owned(),
            )
        });
        assert_eq!(landing_label(&view), "already");
        let lines: Vec<String> = landing_lines(&view.record.landed[0])
            .iter()
            .map(|l| l.to_string())
            .collect();
        assert_eq!(
            lines,
            ["Landed:    already in main (contained by 0123456789ab)"]
        );
    }

    #[test]
    fn a_resolved_landing_is_marked_and_a_failed_one_says_why() {
        let (mut ctx, mut screen) = list_over(three_runs());
        key(&mut screen, &mut ctx, KeyCode::Char('j'));
        key(&mut screen, &mut ctx, KeyCode::Char('j'));
        key(&mut screen, &mut ctx, KeyCode::Enter);
        let text = render_text(&screen, &ctx);
        for needle in [
            "resolved by agent",
            "conflicted: shared.txt",
            "0123456789ab",
            "attempts: land/1",
        ] {
            assert!(text.contains(needle), "{needle}:\n{text}");
        }
        let mut runs = three_runs();
        runs[1].record.land_failure = Some(crate::oneoff::store::LandFailure {
            at: "2026-11-03T11:00:00Z".to_owned(),
            reason: "the resolver's report was failed".to_owned(),
            conflicted: vec!["a.txt".to_owned()],
            attempts: vec!["land/1".to_owned()],
            stop: None,
        });
        set_runs(&mut ctx, runs);
        let mut screen = OneoffScreen::list(ALIAS.to_owned());
        key(&mut screen, &mut ctx, KeyCode::Char('j'));
        key(&mut screen, &mut ctx, KeyCode::Enter);
        let text = render_text(&screen, &ctx);
        assert!(
            text.contains("land failed: the resolver's report was failed"),
            "{text}"
        );
        assert!(text.contains("conflicted: a.txt"), "{text}");
        assert!(render_text(&OneoffScreen::list(ALIAS.to_owned()), &ctx).contains("land failed"));
    }

    #[test]
    fn the_summary_names_the_tracked_branch_until_a_merge_deletes_it() {
        let mut runs = three_runs();
        runs[1].record.tracked_branch = Some(format!("refs/heads/gsd-oneoff/{MIDDLE}"));
        runs[2].record.tracked_branch = Some(format!("refs/heads/gsd-oneoff/{OLDEST}"));
        let (mut ctx, mut screen) = list_over(runs);
        key(&mut screen, &mut ctx, KeyCode::Char('j'));
        key(&mut screen, &mut ctx, KeyCode::Enter);
        assert!(render_text(&screen, &ctx).contains("git branch --no-merged"));
        // OLDEST was merged: its branch is gone although the record still names it.
        let mut screen = OneoffScreen::list(ALIAS.to_owned());
        key(&mut screen, &mut ctx, KeyCode::Char('j'));
        key(&mut screen, &mut ctx, KeyCode::Char('j'));
        key(&mut screen, &mut ctx, KeyCode::Enter);
        assert!(!render_text(&screen, &ctx).contains("git branch --no-merged"));
    }

    #[test]
    fn d_shows_the_capped_merge_diff_of_a_landed_merge_and_scrolls() {
        let (mut ctx, mut screen) = list_over(three_runs());
        // A run no merge landed has nothing to show.
        key(&mut screen, &mut ctx, KeyCode::Char('d'));
        assert_eq!(screen.mode, OneoffMode::List);
        assert!(ctx
            .status_message
            .take()
            .is_some_and(|(m, _)| m.contains("No merge to show")));

        key(&mut screen, &mut ctx, KeyCode::Char('j'));
        key(&mut screen, &mut ctx, KeyCode::Char('j'));
        match dispatched(key(&mut screen, &mut ctx, KeyCode::Char('d'))) {
            Action::OneoffDiffRequested { alias, run_id } => {
                assert_eq!(alias, ALIAS);
                assert_eq!(run_id, OLDEST);
            }
            other => panic!("{other:?}"),
        }
        assert!(matches!(screen.mode, OneoffMode::Diff { scroll: 0 }));
        assert!(render_text(&screen, &ctx).contains("Reading the diff"));

        // Another run's diff is never shown for this one.
        ctx.view_cache.get_mut(ALIAS).unwrap().oneoff_diff =
            Some((MIDDLE.to_owned(), Ok(vec!["other".to_owned()])));
        assert!(render_text(&screen, &ctx).contains("Reading the diff"));

        let mut lines = vec!["conflicted: shared.txt".to_owned(), String::new()];
        lines.extend((0..100).map(|i| format!("+line {i}")));
        ctx.view_cache.get_mut(ALIAS).unwrap().oneoff_diff = Some((OLDEST.to_owned(), Ok(lines)));
        let text = render_text(&screen, &ctx);
        assert!(
            text.contains("conflicted: shared.txt") && text.contains("+line 0"),
            "{text}"
        );
        assert!(!text.contains("+line 99"), "{text}");
        key(&mut screen, &mut ctx, KeyCode::PageDown);
        let OneoffMode::Diff { scroll } = screen.mode else {
            panic!("still in the diff");
        };
        assert!(scroll > 1, "a page moved the window: {scroll}");
        key(&mut screen, &mut ctx, KeyCode::Char('G'));
        let text = render_text(&screen, &ctx);
        assert!(text.contains("+line 99"), "{text}");
        let OneoffMode::Diff { scroll: end } = screen.mode else {
            panic!("still in the diff");
        };
        key(&mut screen, &mut ctx, KeyCode::Char('j'));
        assert_eq!(screen.mode, OneoffMode::Diff { scroll: end }, "clamped");
        key(&mut screen, &mut ctx, KeyCode::Char('k'));
        key(&mut screen, &mut ctx, KeyCode::Home);
        assert_eq!(screen.mode, OneoffMode::Diff { scroll: 0 });
        key(&mut screen, &mut ctx, KeyCode::Esc);
        assert_eq!(screen.mode, OneoffMode::Result);
        assert!(render_text(&screen, &ctx).contains("resolved by agent"));
    }

    #[test]
    fn o_in_the_diff_exports_the_full_diff_only_while_a_diff_is_shown() {
        let (mut ctx, mut screen) = list_over(three_runs());
        key(&mut screen, &mut ctx, KeyCode::Char('j'));
        key(&mut screen, &mut ctx, KeyCode::Char('j'));
        key(&mut screen, &mut ctx, KeyCode::Char('d'));
        // Still reading: nothing to export yet.
        assert!(matches!(
            key(&mut screen, &mut ctx, KeyCode::Char('o')),
            ScreenAction::None
        ));
        // A failed read has no diff to export either.
        ctx.view_cache.get_mut(ALIAS).unwrap().oneoff_diff =
            Some((OLDEST.to_owned(), Err("no".to_owned())));
        assert!(matches!(
            key(&mut screen, &mut ctx, KeyCode::Char('o')),
            ScreenAction::None
        ));
        ctx.view_cache.get_mut(ALIAS).unwrap().oneoff_diff = Some((
            OLDEST.to_owned(),
            Ok(vec![
                "conflicted: none".to_owned(),
                "+1 -0  a\u{1b}[2J.txt".to_owned(),
                String::new(),
                "truncated — 2 more files, ~9 more lines".to_owned(),
            ]),
        ));
        let text = render_text(&screen, &ctx);
        assert!(text.contains("truncated — 2 more files"), "{text}");
        assert!(text.contains("o full diff"), "{text}");
        assert!(!text.contains('\u{1b}'), "{text:?}");
        match dispatched(key(&mut screen, &mut ctx, KeyCode::Char('o'))) {
            Action::OneoffDiffExportRequested { alias, run_id } => {
                assert_eq!(alias, ALIAS);
                assert_eq!(run_id, OLDEST);
            }
            other => panic!("{other:?}"),
        }
        assert!(
            matches!(screen.mode, OneoffMode::Diff { .. }),
            "stays in the diff"
        );
    }

    #[test]
    fn a_diff_that_could_not_be_read_says_why() {
        let (mut ctx, mut screen) = list_over(three_runs());
        key(&mut screen, &mut ctx, KeyCode::Char('j'));
        key(&mut screen, &mut ctx, KeyCode::Char('j'));
        key(&mut screen, &mut ctx, KeyCode::Char('d'));
        ctx.view_cache.get_mut(ALIAS).unwrap().oneoff_diff =
            Some((OLDEST.to_owned(), Err("git failed\u{1b}[2J".to_owned())));
        let text = render_text(&screen, &ctx);
        assert!(
            text.contains("The diff could not be read: git failed"),
            "{text}"
        );
        assert!(!text.contains('\u{1b}'), "{text:?}");
    }

    #[test]
    fn the_picker_moves_and_q_confirms_the_selected_todo() {
        let dir = tempfile::tempdir().unwrap();
        let (mut ctx, _rx) = ctx_with_project(dir.path());
        ctx.view_cache
            .entry(ALIAS.to_owned())
            .or_default()
            .oneoff_todos = Some(Ok(vec![todo("a.md", "A"), todo("b.md", "B")]));
        let mut screen = OneoffScreen::picker(ALIAS.to_owned());
        key(&mut screen, &mut ctx, KeyCode::Char('j'));
        key(&mut screen, &mut ctx, KeyCode::Char('q'));
        assert_eq!(
            screen.mode,
            OneoffMode::Confirm {
                mode: Mode::Quick,
                task: TaskSpec::Todo {
                    file_name: "b.md".to_owned(),
                    title: "B".to_owned()
                },
                model: "opus".to_owned()
            }
        );
    }

    #[test]
    fn the_type_row_opens_an_input_that_refuses_an_empty_task() {
        let dir = tempfile::tempdir().unwrap();
        let (mut ctx, _rx) = ctx_with_project(dir.path());
        ctx.view_cache
            .entry(ALIAS.to_owned())
            .or_default()
            .oneoff_todos = Some(Ok(vec![todo("a.md", "A")]));
        let mut screen = OneoffScreen::picker(ALIAS.to_owned());
        key(&mut screen, &mut ctx, KeyCode::Char('j'));
        key(&mut screen, &mut ctx, KeyCode::Char('f'));
        assert!(matches!(
            screen.mode,
            OneoffMode::Input {
                mode: Mode::Fast,
                ..
            }
        ));
        key(&mut screen, &mut ctx, KeyCode::Enter);
        let OneoffMode::Input { error, .. } = &screen.mode else {
            panic!("an empty task must stay in the input");
        };
        assert!(error.as_deref().unwrap_or("").contains("empty"));
        for c in "fix x".chars() {
            key(&mut screen, &mut ctx, KeyCode::Char(c));
        }
        key(&mut screen, &mut ctx, KeyCode::Enter);
        assert!(matches!(
            &screen.mode,
            OneoffMode::Confirm { mode: Mode::Fast, task: TaskSpec::Text(t), .. } if t == "fix x"
        ));
    }

    #[test]
    fn esc_walks_back_from_the_input_then_pops_the_picker() {
        let dir = tempfile::tempdir().unwrap();
        let (mut ctx, _rx) = ctx_with_project(dir.path());
        let mut screen = OneoffScreen::picker(ALIAS.to_owned());
        // While loading only the type row exists, so `q` opens the input.
        key(&mut screen, &mut ctx, KeyCode::Char('q'));
        assert!(matches!(screen.mode, OneoffMode::Input { .. }));
        key(&mut screen, &mut ctx, KeyCode::Esc);
        assert!(matches!(screen.mode, OneoffMode::Picker { .. }));
        assert!(render_text(&screen, &ctx).contains("Loading the pending todos"));
        assert!(matches!(
            key(&mut screen, &mut ctx, KeyCode::Esc),
            ScreenAction::Pop
        ));
    }

    #[test]
    fn the_confirm_popup_shows_the_caps_cycles_the_model_and_dispatches_on_enter() {
        let dir = tempfile::tempdir().unwrap();
        let (mut ctx, mut rx) = ctx_with_project(dir.path());
        let mut screen = OneoffScreen::in_mode(
            ALIAS.to_owned(),
            OneoffScreen::confirm(Mode::Quick, TaskSpec::Text("do it".to_owned())),
        );
        let text = render_text(&screen, &ctx);
        for needle in [
            "/gsd-quick",
            "do it",
            "opus",
            "$10.00",
            "60 min",
            "Uncommitted changes",
        ] {
            assert!(text.contains(needle), "{needle}:\n{text}");
        }
        key(&mut screen, &mut ctx, KeyCode::Char('m'));
        assert!(render_text(&screen, &ctx).contains("sonnet"));
        key(&mut screen, &mut ctx, KeyCode::Char('m'));
        assert!(matches!(&screen.mode, OneoffMode::Confirm { model, .. } if model == "opus"));
        key(&mut screen, &mut ctx, KeyCode::Char('m'));
        assert!(matches!(
            key(&mut screen, &mut ctx, KeyCode::Enter),
            ScreenAction::Pop
        ));
        let Ok(Action::OneoffRunRequested {
            mode, task, model, ..
        }) = rx.try_recv()
        else {
            panic!("Enter must dispatch the run");
        };
        assert_eq!(mode, Mode::Quick);
        assert_eq!(task, TaskSpec::Text("do it".to_owned()));
        assert_eq!(model, "sonnet");

        let mut screen = OneoffScreen::in_mode(
            ALIAS.to_owned(),
            OneoffScreen::confirm(Mode::Quick, TaskSpec::Text("do it".to_owned())),
        );
        assert!(matches!(
            key(&mut screen, &mut ctx, KeyCode::Esc),
            ScreenAction::Pop
        ));
        assert!(rx.try_recv().is_err(), "Esc dispatches nothing");
    }

    #[test]
    fn the_result_popup_renders_the_report_escaped_and_labelled() {
        let (mut ctx, mut screen) = list_over(Vec::new());
        key(&mut screen, &mut ctx, KeyCode::Enter);
        assert_eq!(screen.mode, OneoffMode::List, "nothing to open");
        set_runs(&mut ctx, vec![finished_view("evil\u{1b}[2J\u{202e}")]);
        key(&mut screen, &mut ctx, KeyCode::Enter);
        let text = render_text(&screen, &ctx);
        for needle in [
            "head evil",
            "changed evil",
            "verified: evil",
            "follow evil",
            "SUMMARY",
            "agent-reported",
            "Host:",
        ] {
            assert!(text.contains(needle), "{needle}:\n{text}");
        }
        assert!(
            !text.contains('\u{1b}') && !text.contains('\u{202e}'),
            "{text:?}"
        );
    }

    #[test]
    fn the_list_escapes_hostile_runs() {
        let mut view = finished_view("evil\u{1b}[2J\u{202e}");
        view.record.run_id = "oneoff-evil\u{1b}[2J\u{202e}".to_owned();
        let (ctx, screen) = list_over(vec![view]);
        let text = render_text(&screen, &ctx);
        assert!(text.contains("head evil"), "{text}");
        assert!(
            !text.contains('\u{1b}') && !text.contains('\u{202e}'),
            "{text:?}"
        );
    }

    #[test]
    fn e_opens_only_a_verified_summary() {
        let mut view = finished_view("x");
        let (mut ctx, mut screen) = list_over(vec![view.clone()]);
        assert!(matches!(
            key(&mut screen, &mut ctx, KeyCode::Char('e')),
            ScreenAction::None
        ));
        assert!(ctx.status_message.is_some());
        view.summary_file = Some(PathBuf::from("/w/work/s.md"));
        set_runs(&mut ctx, vec![view]);
        assert!(matches!(
            key(&mut screen, &mut ctx, KeyCode::Char('e')),
            ScreenAction::SuspendAndEdit(p, None) if p == std::path::Path::new("/w/work/s.md")
        ));
    }

    #[test]
    fn the_status_line_shows_the_counts() {
        let mut runs = three_runs();
        let text = status_line(&runs).to_string();
        // One live, one finished with commits nothing landed, one merged.
        assert_eq!(text, "one-offs: 1 running, 1 to land (T)");
        runs.remove(0);
        runs.remove(0);
        assert_eq!(status_line(&runs).to_string(), "one-offs: 1 finished (T)");
    }
}
