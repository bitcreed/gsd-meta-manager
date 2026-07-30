//! **Surface 5 — the start flow: a command, then an optional goal** (D-23).
//!
//! Two fields built out of the input model that already exists, rather than a
//! new widget. `ctx.input_buffer` is a **single shared `String`**, so a two-step
//! wizard holds the committed command on the screen struct and lets the buffer
//! serve only the active field. That is the minimal extension: it adds no
//! `AppContext` field, which matters because every full-field `AppContext`
//! fixture in the test suite breaks on one.
//!
//! **No movable cursor and no rich text widget** (D-22). All the existing text
//! inputs in this tree fake a caret with a trailing `Span::raw("_")`; a fifth,
//! different input model for one field is churn, and `tui-textarea` — already a
//! dependency for something else — deliberately gains no call site here.
//!
//! ## The goal is stored verbatim
//!
//! Whatever the user types reaches `RunRecord.goal` byte-identically. This
//! screen **interprets nothing**: goal decomposition and prompt-injection
//! hardening are **Phase 21's**, and paraphrasing a human's stated intent into a
//! record they will later be judged against is exactly the thing that must not
//! happen by accident. Only the *rendering* of a goal is sanitised, in
//! [`super::driver_confirm`].
//!
//! An empty goal becomes `None`, never `Some("")`: `drive_argv` omits the flag
//! entirely for `None`, whereas an empty string would be recorded verbatim as a
//! goal the user gave. "No goal" and "an empty goal" are different facts and the
//! second one is worse.
//!
//! ## Two screens from one idiom that behave differently on `Enter`
//!
//! `Enter` on an empty buffer **accepts the default command and advances** here,
//! while the same key on an empty buffer in [`super::driver_inject`] does
//! nothing at all. That asymmetry is deliberate and it is recorded here because
//! it is exactly the sort of thing a later reader will "fix": there **is** a
//! sensible default for which command to run ([`DEFAULT_DRIVE_COMMAND`], a
//! read-only progress command), and there is no sensible default for what a
//! human wants to say to a running agent.
//!
//! ## What this screen is not
//!
//! It picks **one** command. A Phase 18 run executes exactly one GSD command per
//! `drive` invocation; multi-command sequences and run bounds are **Phase 20's**,
//! so nothing here may imply a sequence.

use super::driver_confirm::{DriverConfirmScreen, DEFAULT_DRIVE_COMMAND};
use super::{AppContext, Screen, ScreenAction};
use crate::state_reader::queue_md;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// Which of the two fields is taking keystrokes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StartStep {
    /// Step A — which GSD command this run issues.
    Command,
    /// Step B — why, in the human's own words. Optional.
    Goal,
}

const COMMAND_PROMPT: &str = "  Command> ";
const COMMAND_HINT: &str = "   [Tab] suggestions  [Enter] next  [Esc] cancel";
const GOAL_PROMPT: &str = "  Goal (optional)> ";
/// `[Esc] back`, not `[Esc] cancel`, and the hint says so because the behaviour
/// says so: losing a typed command to a stray keystroke is the kind of thing
/// that makes a user stop using a flow.
const GOAL_HINT: &str = "   [Enter] start  [Esc] back";

/// The two-field wizard that precedes a driver run's confirmation.
pub struct DriverStartScreen {
    alias: String,
    step: StartStep,
    /// The command committed at Step A. Empty until then; `ctx.input_buffer`
    /// serves whichever field is active.
    command: String,
}

impl DriverStartScreen {
    pub fn new(alias: String) -> Self {
        Self {
            alias,
            step: StartStep::Command,
            command: String::new(),
        }
    }

    /// Which field is active. Exposed so the step machine is assertable
    /// without rendering.
    pub fn step(&self) -> StartStep {
        self.step
    }

    /// The command committed so far — empty while Step A is still active.
    pub fn command(&self) -> &str {
        &self.command
    }

    /// Step A's `Tab`: cycle the **same** suggestion source that already backs
    /// `EnqueueScreen`'s completion (D-23).
    ///
    /// "What should I run next" is not re-derived here. `suggest_next_commands`
    /// prefers `gsd-tools smart-entry` and falls back to a keyword heuristic;
    /// duplicating either would create a second answer that drifts from the one
    /// the queue screen gives for the same project.
    fn cycle_suggestion(&self, ctx: &mut AppContext) {
        let Some(state) = ctx.project_states.get(&self.alias) else {
            return;
        };
        let suggestions = queue_md::suggest_next_commands(state);
        if suggestions.is_empty() {
            return;
        }
        ctx.suggestion_index = (ctx.suggestion_index + 1) % suggestions.len();
        ctx.input_buffer = suggestions[ctx.suggestion_index].clone();
        ctx.needs_redraw = true;
    }
}

impl Screen for DriverStartScreen {
    fn handle_key(
        &mut self,
        code: KeyCode,
        _modifiers: KeyModifiers,
        ctx: &mut AppContext,
    ) -> ScreenAction {
        match (self.step, code) {
            // ── Step A — command ────────────────────────────────────────────
            (StartStep::Command, KeyCode::Tab) => {
                self.cycle_suggestion(ctx);
                ScreenAction::None
            }
            (StartStep::Command, KeyCode::Enter) => {
                // Enter on an empty buffer accepts the default rather than
                // doing nothing — see the module doc on the asymmetry with the
                // injection screen.
                self.command = if ctx.input_buffer.is_empty() {
                    DEFAULT_DRIVE_COMMAND.to_string()
                } else {
                    // `take` leaves the shared buffer empty for the goal field.
                    std::mem::take(&mut ctx.input_buffer)
                };
                ctx.suggestion_index = 0;
                self.step = StartStep::Goal;
                ctx.needs_redraw = true;
                ScreenAction::None
            }
            (StartStep::Command, KeyCode::Esc) => {
                ctx.input_buffer.clear();
                ctx.suggestion_index = 0;
                ctx.needs_redraw = true;
                ScreenAction::Pop
            }

            // ── Step B — goal (optional) ────────────────────────────────────
            (StartStep::Goal, KeyCode::Enter) => {
                // Empty *or* non-empty: the goal is optional, so there is
                // nothing to refuse here.
                let typed = std::mem::take(&mut ctx.input_buffer);
                let goal = if typed.is_empty() { None } else { Some(typed) };
                ctx.needs_redraw = true;
                ScreenAction::Push(Box::new(DriverConfirmScreen::new_start(
                    self.alias.clone(),
                    self.command.clone(),
                    goal,
                )))
            }
            (StartStep::Goal, KeyCode::Esc) => {
                // Back, not cancel. The committed command is put back in the
                // buffer so the user sees and can edit what they already typed.
                ctx.input_buffer = self.command.clone();
                self.step = StartStep::Command;
                ctx.needs_redraw = true;
                ScreenAction::None
            }

            // ── Editing, shared by both fields ──────────────────────────────
            (_, KeyCode::Backspace) => {
                ctx.input_buffer.pop();
                ctx.suggestion_index = 0;
                ctx.needs_redraw = true;
                ScreenAction::None
            }
            (_, KeyCode::Char(c)) => {
                ctx.input_buffer.push(c);
                ctx.suggestion_index = 0;
                ctx.needs_redraw = true;
                ScreenAction::None
            }
            // Includes `Tab` on the goal field: a goal is free prose, and the
            // suggestion source suggests GSD commands.
            _ => ScreenAction::None,
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        let chunks = Layout::vertical([
            Constraint::Min(0),
            Constraint::Length(1),
            Constraint::Length(1),
        ])
        .split(area);

        // The body is the detail view behind the wizard, as `enqueue.rs` does.
        let detail = super::detail::DetailScreen::new(self.alias.clone());
        detail.render_main_only(frame, chunks[0], ctx);

        let bold = Style::default().add_modifier(Modifier::BOLD);
        let dim = Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::DIM);
        let hint = Style::default().fg(Color::DarkGray);

        let (command_row, goal_row) = match self.step {
            StartStep::Command => (
                // Active: prompt BOLD, buffer default, caret, hints DarkGray.
                Line::from(vec![
                    Span::styled(COMMAND_PROMPT, bold),
                    Span::raw(&ctx.input_buffer),
                    Span::raw("_"),
                    Span::styled(COMMAND_HINT, hint),
                ]),
                // Not yet reached: the whole row DIM, so it reads as a field
                // that exists rather than one that is waiting.
                Line::from(Span::styled(GOAL_PROMPT, dim)),
            ),
            StartStep::Goal => (
                // Already answered: DarkGray, showing the committed command.
                Line::from(vec![
                    Span::styled(COMMAND_PROMPT, hint),
                    Span::styled(self.command.as_str(), hint),
                ]),
                Line::from(vec![
                    Span::styled(GOAL_PROMPT, bold),
                    Span::raw(&ctx.input_buffer),
                    Span::raw("_"),
                    Span::styled(GOAL_HINT, hint),
                ]),
            ),
        };

        frame.render_widget(Paragraph::new(command_row), chunks[1]);
        frame.render_widget(Paragraph::new(goal_row), chunks[2]);
    }

    fn name(&self) -> &str {
        "driver_start"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::Action;
    use crate::registry;
    use crate::state_reader::ProjectState;
    use crate::ui::screens::driver_confirm::tests::{ctx_with_project, ALIAS};

    fn press(screen: &mut dyn Screen, ctx: &mut AppContext, code: KeyCode) -> ScreenAction {
        screen.handle_key(code, KeyModifiers::NONE, ctx)
    }

    fn type_str(screen: &mut dyn Screen, ctx: &mut AppContext, s: &str) {
        for c in s.chars() {
            press(screen, ctx, KeyCode::Char(c));
        }
    }

    /// Give the fixture a project state so the suggestion source has something
    /// to derive from. A default `ProjectState` has an empty `project_root`, so
    /// `suggest_next_commands` skips the `smart-entry` subprocess and returns
    /// the deterministic keyword list.
    fn with_state(ctx: &mut AppContext) -> Vec<String> {
        let state = ProjectState::default();
        let suggestions = queue_md::suggest_next_commands(&state);
        ctx.project_states.insert(ALIAS.to_string(), state);
        suggestions
    }

    /// Drive the confirmation the wizard pushed, and read what it dispatched.
    ///
    /// Asserting through the *pushed screen* rather than through an accessor is
    /// what makes this an end-to-end check of T-18-41: the command the
    /// confirmation would name and the command it dispatches are the same
    /// value, and this test only ever sees the second one.
    fn confirm_and_take_action(
        pushed: ScreenAction,
        ctx: &mut AppContext,
        rx: &mut tokio::sync::mpsc::UnboundedReceiver<Action>,
    ) -> (String, Option<String>) {
        let ScreenAction::Push(mut confirm) = pushed else {
            panic!("the goal step must push a confirmation");
        };
        assert_eq!(confirm.name(), "driver_confirm");
        registry::record_opt_in(&mut ctx.config, ALIAS).expect("opt in");
        confirm.handle_key(KeyCode::Char('y'), KeyModifiers::NONE, ctx);

        let sent = rx.try_recv().expect("a confirmed start must dispatch");
        let Action::DriverStartRequested { command, goal, .. } = sent else {
            panic!("expected DriverStartRequested, got {sent:?}");
        };
        (command, goal)
    }

    #[test]
    fn tab_cycles_the_shared_suggestion_source_and_wraps() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut ctx, _rx) = ctx_with_project(dir.path());
        let expected = with_state(&mut ctx);
        assert!(
            expected.len() > 1,
            "the fixture must offer more than one suggestion or the wrap \
             assertion is vacuous"
        );

        let mut s = DriverStartScreen::new(ALIAS.to_string());

        // `suggestion_index` starts at 0 and is incremented before use, exactly
        // as `enqueue.rs` does, so the first Tab shows the second suggestion.
        for i in 1..=expected.len() {
            press(&mut s, &mut ctx, KeyCode::Tab);
            assert_eq!(
                ctx.input_buffer,
                expected[i % expected.len()],
                "Tab #{i} must show suggestion {}",
                i % expected.len()
            );
        }
        assert_eq!(
            ctx.input_buffer, expected[0],
            "the list must wrap rather than stick at the end"
        );
    }

    #[test]
    fn enter_on_an_empty_command_commits_the_default_and_advances() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut ctx, _rx) = ctx_with_project(dir.path());
        let mut s = DriverStartScreen::new(ALIAS.to_string());

        let action = press(&mut s, &mut ctx, KeyCode::Enter);
        assert!(matches!(action, ScreenAction::None));
        assert_eq!(
            s.command(),
            DEFAULT_DRIVE_COMMAND,
            "unlike the injection screen, Enter-on-empty accepts the default \
             here — there IS a sensible default for which command to run"
        );
        assert_eq!(s.step(), StartStep::Goal);
        assert!(
            ctx.input_buffer.is_empty(),
            "the shared buffer must be handed to the goal field empty"
        );
    }

    #[test]
    fn enter_on_a_typed_command_advances_with_that_command() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut ctx, _rx) = ctx_with_project(dir.path());
        let mut s = DriverStartScreen::new(ALIAS.to_string());

        type_str(&mut s, &mut ctx, "/gsd:execute-phase 18");
        press(&mut s, &mut ctx, KeyCode::Enter);

        assert_eq!(s.command(), "/gsd:execute-phase 18");
        assert_eq!(s.step(), StartStep::Goal);
        assert!(ctx.input_buffer.is_empty());
    }

    #[test]
    fn esc_on_the_goal_step_returns_to_the_command_step_without_cancelling() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut ctx, mut rx) = ctx_with_project(dir.path());
        let mut s = DriverStartScreen::new(ALIAS.to_string());

        type_str(&mut s, &mut ctx, "/gsd:plan-phase 19");
        press(&mut s, &mut ctx, KeyCode::Enter);
        type_str(&mut s, &mut ctx, "half a goal");

        let action = press(&mut s, &mut ctx, KeyCode::Esc);
        assert!(
            matches!(action, ScreenAction::None),
            "Esc on the goal step must NOT pop — losing a typed command to a \
             stray keystroke is the kind of thing that makes a user stop using \
             a flow"
        );
        assert_eq!(s.step(), StartStep::Command);
        assert_eq!(
            ctx.input_buffer, "/gsd:plan-phase 19",
            "the committed command comes back into the buffer so the user can \
             see and edit what they already typed"
        );
        assert_eq!(s.command(), "/gsd:plan-phase 19");
        assert!(rx.try_recv().is_err(), "going back must dispatch nothing");
    }

    #[test]
    fn esc_on_the_command_step_pops_without_dispatching() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut ctx, mut rx) = ctx_with_project(dir.path());
        let mut s = DriverStartScreen::new(ALIAS.to_string());

        type_str(&mut s, &mut ctx, "/gsd:quick");
        let action = press(&mut s, &mut ctx, KeyCode::Esc);
        assert!(matches!(action, ScreenAction::Pop));
        assert!(
            ctx.input_buffer.is_empty(),
            "an abandoned command must not leak into the next screen sharing \
             the buffer"
        );
        assert!(
            rx.try_recv().is_err(),
            "cancelling must start nothing — this is the last step before an \
             autonomous agent runs on the user's repo"
        );
    }

    #[test]
    fn an_empty_goal_becomes_none_rather_than_an_empty_string() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut ctx, mut rx) = ctx_with_project(dir.path());
        let mut s = DriverStartScreen::new(ALIAS.to_string());

        type_str(&mut s, &mut ctx, "/gsd:verify-work 18");
        press(&mut s, &mut ctx, KeyCode::Enter);
        let pushed = press(&mut s, &mut ctx, KeyCode::Enter);

        let (command, goal) = confirm_and_take_action(pushed, &mut ctx, &mut rx);
        assert_eq!(command, "/gsd:verify-work 18");
        assert_eq!(
            goal, None,
            "`drive_argv` omits the flag entirely for None; Some(\"\") would be \
             recorded verbatim as a goal the user never gave"
        );
    }

    #[test]
    fn a_typed_multi_byte_goal_reaches_the_confirmation_byte_identically() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut ctx, mut rx) = ctx_with_project(dir.path());
        let mut s = DriverStartScreen::new(ALIAS.to_string());

        let typed = "terminer la phase 18 🚀 — 日本語も";
        type_str(&mut s, &mut ctx, "/gsd:execute-phase 18");
        press(&mut s, &mut ctx, KeyCode::Enter);
        type_str(&mut s, &mut ctx, typed);
        let pushed = press(&mut s, &mut ctx, KeyCode::Enter);

        let (command, goal) = confirm_and_take_action(pushed, &mut ctx, &mut rx);
        assert_eq!(command, "/gsd:execute-phase 18");
        assert_eq!(
            goal.as_deref().map(str::as_bytes),
            Some(typed.as_bytes()),
            "stored verbatim, never paraphrased — interpretation is Phase 21's"
        );
        assert!(
            ctx.input_buffer.is_empty(),
            "the goal must not stay in the shared buffer after it is committed"
        );
    }

    #[test]
    fn the_goal_field_offers_no_completion_and_neither_prompt_mentions_a_cursor() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut ctx, _rx) = ctx_with_project(dir.path());
        with_state(&mut ctx);
        let mut s = DriverStartScreen::new(ALIAS.to_string());

        press(&mut s, &mut ctx, KeyCode::Enter);
        assert_eq!(s.step(), StartStep::Goal);
        type_str(&mut s, &mut ctx, "why");
        press(&mut s, &mut ctx, KeyCode::Tab);
        assert_eq!(
            ctx.input_buffer, "why",
            "the suggestion source suggests GSD commands; a goal is free prose, \
             so Tab must not overwrite it"
        );

        assert!(
            !GOAL_HINT.contains("Tab") && GOAL_HINT.contains("[Esc] back"),
            "the goal hint must say `back`, because Esc goes back. Got: {GOAL_HINT}"
        );
        assert!(
            COMMAND_HINT.contains("[Tab] suggestions") && COMMAND_HINT.contains("[Esc] cancel"),
            "got: {COMMAND_HINT}"
        );
    }
}
