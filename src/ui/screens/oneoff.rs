//! **The one-off popup** (quick 261103-f5y, C-22..C-26): the Driver tab's `t`
//! (pick a pending todo or type a task, then `q` for /gsd-quick or `f` for /gsd-fast,
//! then confirm the model, spend ceiling and wall clock) and `T` (the latest run's
//! strict summary, with `e` SUMMARY.md, `m` merge, `p` push, `d` discard and `x` kill,
//! each asked first).
//!
//! One screen with internal modes (I-14). It reads only the view cache and dispatches
//! actions: every file read, launch and landing runs on `spawn_blocking` in `App`.
//! Every todo title, typed task, report field, path and run id is drawn through
//! [`render_for_terminal`]. The defaults shown come from `cli::task_run_defaults` and
//! the models from the runtime seam, never a copy.

use super::{AppContext, Screen, ScreenAction};
use crate::action::Action;
use crate::oneoff::runtime::AgentRuntime;
use crate::oneoff::store::{LiveState, OneoffView, ReportSource};
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
    /// The latest run's summary.
    Result,
    /// A landing or kill, asked first.
    Ask {
        prompt: Vec<String>,
        command: OneoffCommand,
    },
}

/// The one-off popup of one project.
pub struct OneoffScreen {
    alias: String,
    mode: OneoffMode,
}

impl OneoffScreen {
    /// The picker (`t`).
    pub fn picker(alias: String) -> Self {
        Self {
            alias,
            mode: OneoffMode::Picker { selected: 0 },
        }
    }

    /// The result popup (`T`).
    pub fn result(alias: String) -> Self {
        Self {
            alias,
            mode: OneoffMode::Result,
        }
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

    fn view<'a>(&self, ctx: &'a AppContext) -> Option<&'a OneoffView> {
        ctx.view_cache
            .get(&self.alias)
            .and_then(|c| c.oneoff.as_deref())
    }

    fn confirm(mode: Mode, task: TaskSpec) -> OneoffMode {
        let model = AgentRuntime::default()
            .models()
            .first()
            .copied()
            .unwrap_or("opus")
            .to_owned();
        OneoffMode::Confirm { mode, task, model }
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

    fn result_key(&mut self, code: KeyCode, ctx: &mut AppContext) -> ScreenAction {
        if code == KeyCode::Esc {
            return ScreenAction::Pop;
        }
        let Some(view) = self.view(ctx).cloned() else {
            return ScreenAction::None;
        };
        let id = shown(&view.record.run_id);
        let message = |ctx: &mut AppContext, text: String| {
            ctx.status_message = Some((text, std::time::Instant::now()));
            ScreenAction::None
        };
        let ask = |command: OneoffCommand, prompt: Vec<String>| OneoffMode::Ask { prompt, command };
        match code {
            KeyCode::Char('e') => match &view.summary_file {
                Some(path) => ScreenAction::SuspendAndEdit(path.clone(), None),
                None => message(
                    ctx,
                    format!("One-off {id} has no SUMMARY.md to open (a /gsd-fast run writes none)"),
                ),
            },
            KeyCode::Char('m') => {
                let mut prompt = vec![format!(
                    "Merge {id} into your checkout? It fast-forwards when it can; otherwise it \
                     makes a merge commit in a private clone, and conflicts are resolved by a \
                     sandboxed agent. Only a clean checkout changes, at the very end."
                )];
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
            KeyCode::Char('d') => {
                self.mode = ask(
                    OneoffCommand::Discard,
                    vec![format!(
                        "Discard {id}? Its clone and every commit not merged or pushed are deleted."
                    )],
                );
                ScreenAction::None
            }
            KeyCode::Char('v') => {
                ScreenAction::DispatchAction(Box::new(Action::OneoffViewerOpen {
                    alias: self.alias.clone(),
                }))
            }
            KeyCode::Char('x') => {
                if matches!(view.live, LiveState::Running | LiveState::Preparing) {
                    self.mode = ask(
                        OneoffCommand::Stop,
                        vec![format!(
                            "Kill one-off {id} now? Its sandbox is stopped mid-task."
                        )],
                    );
                    ScreenAction::None
                } else {
                    message(
                        ctx,
                        format!("One-off {id} is not running, so there is nothing to kill."),
                    )
                }
            }
            _ => ScreenAction::None,
        }
    }

    fn ask_key(&mut self, code: KeyCode, ctx: &mut AppContext) -> ScreenAction {
        let OneoffMode::Ask { command, .. } = &self.mode else {
            return ScreenAction::None;
        };
        match code {
            KeyCode::Char('y') => {
                let run_id = self.view(ctx).map(|v| v.record.run_id.clone());
                super::driver_confirm::dispatch(
                    ctx,
                    Action::OneoffCommandRequested {
                        alias: self.alias.clone(),
                        run_id,
                        command: *command,
                    },
                    &self.alias,
                );
                ScreenAction::Pop
            }
            KeyCode::Char('n') | KeyCode::Esc => {
                self.mode = OneoffMode::Result;
                ScreenAction::None
            }
            _ => ScreenAction::None,
        }
    }

    /// The popup's title and body for the current mode.
    fn body(&self, ctx: &AppContext) -> (String, Vec<Line<'static>>) {
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
            OneoffMode::Result => {
                let lines = match self.view(ctx) {
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

/// The ~6 lines of a run's summary (C-25), the agent's report and the host's facts
/// labelled apart (I-18).
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
    lines.push(Line::from(match &record.summary_path {
        Some(path) => format!("SUMMARY:   {}", shown(path)),
        None => "SUMMARY:   none".to_owned(),
    }));
    for landing in &record.landed {
        lines.push(Line::from(format!(
            "Landed:    {:?} {}",
            landing.kind,
            shown(&landing.detail)
        )));
    }
    lines.push(Line::from(""));
    lines.push(Line::from(Span::styled(
        "e SUMMARY   v live view   m merge   p push   d discard   x kill   Esc close",
        Style::default().fg(Color::DarkGray),
    )));
    lines
}

/// The Driver tab's one-line one-off status (C-24).
pub fn status_line(view: &OneoffView, now: chrono::DateTime<chrono::Utc>) -> Line<'static> {
    let record = &view.record;
    let id = shown(&record.run_id);
    let text = match view.live {
        LiveState::Running | LiveState::Preparing => {
            let minutes = chrono::DateTime::parse_from_rfc3339(&record.started_at)
                .map(|at| (now - at.with_timezone(&chrono::Utc)).num_minutes().max(0))
                .unwrap_or(0);
            format!(
                "One-off {id}: running {minutes}m - {} (T)",
                record.mode.command()
            )
        }
        LiveState::Finished => match &record.report {
            Some(report) => format!(
                "One-off {id}: finished - {}: {} (T)",
                report.status.as_str(),
                shown(&report.headline)
            ),
            None => format!("One-off {id}: finished (T)"),
        },
        LiveState::Crashed => format!("One-off {id}: crashed - the launcher is gone (T)"),
        LiveState::Unknown => format!("One-off {id}: state unknown (T)"),
    };
    Line::from(Span::styled(text, Style::default().fg(Color::Cyan)))
}

crate::ui::screens::adjudicate_screen!(
    OneoffScreen,
    crate::ui::screens::RENDERS_ATTACKER_INFLUENCED_IDENTITY,
    "Draws the registry key in every title, the pending todos' titles and file names \
     (repository text), the typed task, the model, and the latest run's report fields \
     (agent-written), SUMMARY path and run id. Fixture states: picker with hostile todo \
     titles, input with a typed task, confirm with hostile text, result with a hostile \
     report.",
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
            OneoffMode::Result => self.result_key(code, ctx),
            OneoffMode::Ask { .. } => self.ask_key(code, ctx),
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        super::detail::DetailScreen::new(self.alias.clone()).render_main_only(frame, area, ctx);
        super::confirm_popup::dim_background(frame, area);
        let (title, body) = self.body(ctx);
        let width = area.width.saturating_sub(4).min(100);
        let inner_width = width.saturating_sub(2).max(1);
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
        OneoffView {
            record,
            live: LiveState::Finished,
            run_dir: PathBuf::from("/data/oneoff/ab/oneoff-20261103-142233"),
            summary_file: None,
        }
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

    #[test]
    fn v_opens_the_live_viewer_only_when_there_is_a_run() {
        let dir = tempfile::tempdir().unwrap();
        let (mut ctx, _rx) = ctx_with_project(dir.path());
        let mut screen = OneoffScreen::result(ALIAS.to_owned());
        assert!(matches!(
            key(&mut screen, &mut ctx, KeyCode::Char('v')),
            ScreenAction::None
        ));
        ctx.view_cache.entry(ALIAS.to_owned()).or_default().oneoff =
            Some(Box::new(finished_view("x")));
        match key(&mut screen, &mut ctx, KeyCode::Char('v')) {
            ScreenAction::DispatchAction(action) => match *action {
                Action::OneoffViewerOpen { alias } => assert_eq!(alias, ALIAS),
                other => panic!("{other:?}"),
            },
            _ => panic!("expected a dispatch"),
        }
        assert!(render_text(&screen, &ctx).contains("v live view"));
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
        let mut screen = OneoffScreen {
            alias: ALIAS.to_owned(),
            mode: OneoffScreen::confirm(Mode::Quick, TaskSpec::Text("do it".to_owned())),
        };
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

        let mut screen = OneoffScreen {
            alias: ALIAS.to_owned(),
            mode: OneoffScreen::confirm(Mode::Quick, TaskSpec::Text("do it".to_owned())),
        };
        assert!(matches!(
            key(&mut screen, &mut ctx, KeyCode::Esc),
            ScreenAction::Pop
        ));
        assert!(rx.try_recv().is_err(), "Esc dispatches nothing");
    }

    #[test]
    fn the_result_popup_renders_the_report_escaped_and_labelled() {
        let dir = tempfile::tempdir().unwrap();
        let (mut ctx, _rx) = ctx_with_project(dir.path());
        let screen = OneoffScreen::result(ALIAS.to_owned());
        assert!(render_text(&screen, &ctx).contains(NO_RUN_COPY));
        ctx.view_cache.entry(ALIAS.to_owned()).or_default().oneoff =
            Some(Box::new(finished_view("evil\u{1b}[2J\u{202e}")));
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
    fn e_opens_only_a_verified_summary() {
        let dir = tempfile::tempdir().unwrap();
        let (mut ctx, _rx) = ctx_with_project(dir.path());
        let mut view = finished_view("x");
        ctx.view_cache.entry(ALIAS.to_owned()).or_default().oneoff = Some(Box::new(view.clone()));
        let mut screen = OneoffScreen::result(ALIAS.to_owned());
        assert!(matches!(
            key(&mut screen, &mut ctx, KeyCode::Char('e')),
            ScreenAction::None
        ));
        assert!(ctx.status_message.is_some());
        view.summary_file = Some(PathBuf::from("/w/work/s.md"));
        ctx.view_cache.entry(ALIAS.to_owned()).or_default().oneoff = Some(Box::new(view));
        assert!(matches!(
            key(&mut screen, &mut ctx, KeyCode::Char('e')),
            ScreenAction::SuspendAndEdit(p, None) if p == std::path::Path::new("/w/work/s.md")
        ));
    }

    #[test]
    fn landing_keys_ask_first() {
        let dir = tempfile::tempdir().unwrap();
        let (mut ctx, mut rx) = ctx_with_project(dir.path());
        ctx.view_cache.entry(ALIAS.to_owned()).or_default().oneoff =
            Some(Box::new(finished_view("x")));
        for (k, expected) in [
            ('m', OneoffCommand::Merge),
            ('p', OneoffCommand::Push),
            ('d', OneoffCommand::Discard),
        ] {
            let mut screen = OneoffScreen::result(ALIAS.to_owned());
            key(&mut screen, &mut ctx, KeyCode::Char(k));
            assert!(rx.try_recv().is_err(), "{k} must ask first");
            key(&mut screen, &mut ctx, KeyCode::Char('n'));
            assert!(rx.try_recv().is_err(), "n dispatches nothing");
            assert!(matches!(screen.mode, OneoffMode::Result));
            key(&mut screen, &mut ctx, KeyCode::Char(k));
            key(&mut screen, &mut ctx, KeyCode::Char('y'));
            let Ok(Action::OneoffCommandRequested {
                command, run_id, ..
            }) = rx.try_recv()
            else {
                panic!("y must dispatch {expected:?}");
            };
            assert_eq!(command, expected);
            assert_eq!(run_id.as_deref(), Some("oneoff-20261103-142233"));
        }
        // `x` on a finished run kills nothing.
        let mut screen = OneoffScreen::result(ALIAS.to_owned());
        ctx.status_message = None;
        key(&mut screen, &mut ctx, KeyCode::Char('x'));
        assert!(matches!(screen.mode, OneoffMode::Result));
        assert!(ctx.status_message.is_some());
        // ...and on a live one it asks, then stops.
        let mut live = finished_view("x");
        live.live = LiveState::Running;
        ctx.view_cache.entry(ALIAS.to_owned()).or_default().oneoff = Some(Box::new(live));
        key(&mut screen, &mut ctx, KeyCode::Char('x'));
        key(&mut screen, &mut ctx, KeyCode::Char('y'));
        assert!(matches!(
            rx.try_recv(),
            Ok(Action::OneoffCommandRequested {
                command: OneoffCommand::Stop,
                ..
            })
        ));
    }

    #[test]
    fn the_status_line_names_the_state() {
        let now = chrono::Utc::now();
        let finished = finished_view("x");
        let text = status_line(&finished, now).to_string();
        assert!(text.contains("finished - done: head x (T)"), "{text}");
        let mut running = finished_view("x");
        running.live = LiveState::Running;
        assert!(status_line(&running, now)
            .to_string()
            .contains("running 0m - /gsd-quick"));
        let mut crashed = finished_view("x");
        crashed.live = LiveState::Crashed;
        assert!(status_line(&crashed, now).to_string().contains("crashed"));
    }
}
