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
//! ## Step B also shows the blast radius (D-26)
//!
//! While Step B is active the body's run-detail pane renders the dry-run report
//! for the committed command **in place of** the run detail: the GSD command
//! sequence, the working tree a commit would capture, and the push refspecs the
//! current state would produce. Zero new keys and zero new modes — it is
//! literally "before a start is confirmed", on the pane the user is already
//! looking at.
//!
//! The build goes through [`AppContext::schedule_dry_run_report`] and therefore
//! through `spawn_blocking`: it is two synchronous `git` shell-outs and one of
//! the named WR-10 call sites (D-28). **This screen surfaces the report; it does
//! not police it** — push allowlists, tool denial, pre-push hooks, secret
//! scanning and worktree isolation are Phase 19's.
//!
//! The preview is the phase's **designated cut**: the only surface item with no
//! requirement id. See [`super::driver::render_dry_run_preview`] for the rule.
//!
//! ## What this screen is not
//!
//! It picks **one** command. A Phase 18 run executes exactly one GSD command per
//! `drive` invocation; multi-command sequences and run bounds are **Phase 20's**,
//! so nothing here may imply a sequence.

use super::driver_confirm::{DriverConfirmScreen, DEFAULT_DRIVE_COMMAND};
use super::{AppContext, DryRunPreview, Screen, ScreenAction};
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

/// The refusal for a command that is not a slash command (WR-04).
///
/// **A refusal at the field, not a rewrite of what the user typed.** Silently
/// prefixing a `/` would run a command they did not ask for; running it anyway
/// would put a hyphen-led operand on the child's argv, where clap reports *"a
/// value is required for '--command <COMMAND>' but none was supplied"* and exits
/// non-zero into `/dev/null` — after the TUI has already said "Driving {alias}"
/// and inserted an optimistic `ObservedRun { liveness: Alive }` that vanishes at
/// the next scan with no error anywhere.
///
/// The rule is stated positively rather than as "must not start with `-`",
/// because a GSD command is a slash command and every suggestion this field
/// offers already is one.
pub const NOT_A_SLASH_COMMAND: &str =
    "A GSD command starts with `/`, e.g. `/gsd-progress`. Edit the command, or press [Esc].";

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

    /// Open the dry-run preview for the committed command and dispatch its
    /// build (D-26, the phase's designated cut).
    ///
    /// The preview goes in **loading** first and the build is scheduled second,
    /// which is the ordering that makes the loading state reachable rather than
    /// theoretical: `schedule_dry_run_report` returns immediately, so a pane
    /// that only learned about the preview when the report arrived would never
    /// render the idiom it is supposed to render.
    ///
    /// **It never reaches the blocking report builder in `driver::dry_run`.**
    /// That builder shells out to `git` twice and is one of the named WR-10 call
    /// sites, so it runs on `spawn_blocking` behind
    /// [`AppContext::schedule_dry_run_report`] and returns through
    /// `Action::DriverDryRunLoaded`. Reaching it from here would freeze the
    /// frame rather than raise an error, which is the failure mode with no
    /// symptom worth the name (D-28, T-18-62).
    fn open_dry_run_preview(&self, ctx: &mut AppContext) {
        let command = self.command.clone();
        let cache = ctx.view_cache.entry(self.alias.clone()).or_default();
        cache.driver_dry_run = Some(DryRunPreview {
            command: command.clone(),
            report: None,
        });
        ctx.schedule_dry_run_report(&self.alias, &command);
    }

    /// Close the preview, restoring the normal run detail.
    ///
    /// Called on **every** exit from Step B — back, forward and cancel — because
    /// the preview describes a start that is about to happen, and a preview left
    /// on screen after the flow has moved on describes one that already did.
    fn close_dry_run_preview(&self, ctx: &mut AppContext) {
        if let Some(cache) = ctx.view_cache.get_mut(&self.alias) {
            cache.driver_dry_run = None;
        }
    }
}

crate::ui::screens::adjudicate_screen!(
    DriverStartScreen,
    crate::ui::screens::RENDERS_ATTACKER_INFLUENCED_IDENTITY,
    "Paints its body with `DetailScreen::render_main_only`, and its two \
     wizard rows draw the command being typed (Step A) and the committed \
     command (Step B). Fixture states: one per sub-view at Step A, plus Step \
     B reached by driving the real key handler.",
);

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
                let committed = if ctx.input_buffer.is_empty() {
                    DEFAULT_DRIVE_COMMAND.to_string()
                } else {
                    ctx.input_buffer.clone()
                };

                // **The argv guard, at the field where it can still be fixed**
                // (WR-04). This phase turned an argv operand into a free-text
                // input, and a value beginning with `-` reaches the child's clap
                // as an option rather than an operand: the driver exits non-zero
                // into `/dev/null` while the TUI shows a run that never started.
                // Refusing here keeps the buffer and the cursor where they are,
                // so the user edits rather than retypes.
                if !committed.starts_with('/') {
                    ctx.status_message =
                        Some((NOT_A_SLASH_COMMAND.to_string(), std::time::Instant::now()));
                    ctx.needs_redraw = true;
                    return ScreenAction::None;
                }

                // `take` leaves the shared buffer empty for the goal field, and
                // happens only now that the value is accepted — a refusal above
                // must not eat what the user typed.
                ctx.input_buffer.clear();
                self.command = committed;
                ctx.suggestion_index = 0;
                self.step = StartStep::Goal;
                // Step B is where the preview lives: the user is one keystroke
                // from handing their repository to an autonomous agent, so this
                // is where "what would it touch" belongs.
                self.open_dry_run_preview(ctx);
                ctx.needs_redraw = true;
                ScreenAction::None
            }
            (StartStep::Command, KeyCode::Esc) => {
                ctx.input_buffer.clear();
                ctx.suggestion_index = 0;
                self.close_dry_run_preview(ctx);
                ctx.needs_redraw = true;
                ScreenAction::Pop
            }

            // ── Step B — goal (optional) ────────────────────────────────────
            (StartStep::Goal, KeyCode::Enter) => {
                // Empty *or* non-empty: the goal is optional, so there is
                // nothing to refuse here.
                let typed = std::mem::take(&mut ctx.input_buffer);
                let goal = if typed.is_empty() { None } else { Some(typed) };
                self.close_dry_run_preview(ctx);
                ctx.needs_redraw = true;
                // **`Replace`, not `Push`** (WR-03). The wizard has handed off
                // and has nothing left to do; staying on the stack put the user
                // back on Step B after they confirmed, with an empty buffer and
                // the preview closed — a screen looking exactly as it had before
                // they said yes. A second `Enter` there pushed a second
                // confirmation for the same command and a second `y` started a
                // second run of the same project, which `admit` does not refuse
                // and whose `flock` failure is invisible.
                ScreenAction::Replace(Box::new(DriverConfirmScreen::new_start(
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
                self.close_dry_run_preview(ctx);
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

        // Read, not committed: `handle_key` commits `ctx.input_buffer` and
        // `self.command` raw — the argv the child process receives is
        // byte-identical to what was typed. Only these two echo rows are
        // escaped, because a command with an invisible character in it is a
        // command the operator cannot tell apart from the one they meant.
        let (command_row, goal_row) = match self.step {
            StartStep::Command => (
                // Active: prompt BOLD, buffer default, caret, hints DarkGray.
                Line::from(vec![
                    Span::styled(COMMAND_PROMPT, bold),
                    Span::raw(crate::text::display_identity(&ctx.input_buffer)),
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
                    Span::styled(crate::text::display_identity(&self.command), hint),
                ]),
                Line::from(vec![
                    Span::styled(GOAL_PROMPT, bold),
                    Span::raw(crate::text::display_identity(&ctx.input_buffer)),
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
        // `Replace`, because the wizard must not survive the hand-off (WR-03).
        let ScreenAction::Replace(mut confirm) = pushed else {
            panic!("the goal step must REPLACE itself with a confirmation");
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

    /// WR-04, at the field: a hyphen-led command never reaches the child argv.
    ///
    /// This phase turned an argv operand into a free-text input. `--command`
    /// carries no `allow_hyphen_values`, so clap in the CHILD reports "a value
    /// is required for '--command <COMMAND>' but none was supplied" and exits
    /// non-zero — into `/dev/null`, after the TUI has already shown "Driving
    /// {alias}" and inserted an optimistic live `ObservedRun` that vanishes at
    /// the next scan with no error anywhere. A refusal at the field is the only
    /// place the user can still act on it.
    #[test]
    fn a_command_that_is_not_a_slash_command_is_refused_at_the_field() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut ctx, _rx) = ctx_with_project(dir.path());
        let mut screen = DriverStartScreen::new(ALIAS.to_string());

        for typed in ["-v2 migration", "--dry-run", "gsd-progress"] {
            ctx.input_buffer = typed.to_string();
            ctx.status_message = None;
            let action = press(&mut screen, &mut ctx, KeyCode::Enter);

            assert!(
                matches!(action, ScreenAction::None),
                "{typed:?} must not advance"
            );
            assert_eq!(
                screen.step(),
                StartStep::Command,
                "{typed:?} must leave the user on the field they can fix"
            );
            assert_eq!(
                ctx.input_buffer, typed,
                "and must NOT eat what they typed — a refusal that clears the \
                 buffer costs them the edit"
            );
            assert_eq!(
                ctx.status_message.as_ref().map(|(text, _)| text.as_str()),
                Some(NOT_A_SLASH_COMMAND),
                "the refusal must be visible rather than a silent no-op"
            );
            assert!(
                screen.command().is_empty(),
                "and nothing is committed: {:?}",
                screen.command()
            );
        }

        // The control arm: a real slash command still advances, so the guard is
        // not simply refusing everything.
        ctx.input_buffer = "/gsd-progress".to_string();
        press(&mut screen, &mut ctx, KeyCode::Enter);
        assert_eq!(screen.step(), StartStep::Goal);
        assert_eq!(screen.command(), "/gsd-progress");
        assert!(ctx.input_buffer.is_empty(), "the goal field starts empty");
    }

    /// WR-04's other half: a hyphen-led GOAL is legitimate free text and must
    /// survive the child's parser.
    ///
    /// `-- do not touch main` and `-v2 migration notes` are entirely plausible
    /// things to type into a field labelled "why". The goal is the last operand
    /// the argv builder emits, so terminating option parsing for it swallows no
    /// following flag.
    #[test]
    fn a_hyphen_led_goal_reaches_the_child_parser_as_a_goal() {
        use clap::Parser;

        const HYPHENATED: &str = "-- do not touch main";

        let argv = crate::driver::spawn::drive_argv(
            std::path::Path::new("/tmp/gsd-test/config.json"),
            "demo",
            "/gsd-progress",
            "2026-07-29T12-00-00Z-aaaa",
            Some(HYPHENATED),
        );
        let mut full = vec![std::ffi::OsString::from("gsd-meta-manager")];
        full.extend(argv);

        let cli = crate::cli::Cli::try_parse_from(&full).unwrap_or_else(|e| {
            panic!(
                "a hyphen-led goal must parse rather than exiting the child \
                 non-zero into /dev/null: {e}"
            )
        });
        match cli.command {
            Some(crate::cli::Commands::Drive { command, goal, .. }) => {
                assert_eq!(goal.as_deref(), Some(HYPHENATED), "verbatim");
                assert_eq!(
                    command.as_deref(),
                    Some("/gsd-progress"),
                    "and the goal must not have swallowed the command"
                );
            }
            _ => panic!("the argv must still parse back into a Drive command"),
        }
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

    // ── The dry-run preview at Step B (D-26, the designated cut) ───────────

    /// The preview parked in the view cache for `ALIAS`, if any.
    fn preview(ctx: &AppContext) -> Option<&DryRunPreview> {
        ctx.view_cache
            .get(ALIAS)
            .and_then(|cache| cache.driver_dry_run.as_ref())
    }

    /// An opted-in project, so `DrivableProject::from_registry` yields a token
    /// and the build is actually scheduled.
    ///
    /// Without the opt-in the constructor refuses and the scheduling path
    /// returns early — which is correct behaviour, and would also make every
    /// assertion below pass for the wrong reason.
    fn opted_in(ctx: &mut AppContext) {
        registry::record_opt_in(&mut ctx.config, ALIAS).expect("opt in");
    }

    #[tokio::test]
    async fn reaching_step_b_opens_a_loading_preview_for_the_committed_command() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut ctx, _rx) = ctx_with_project(dir.path());
        opted_in(&mut ctx);
        let mut s = DriverStartScreen::new(ALIAS.to_string());

        assert!(
            preview(&ctx).is_none(),
            "Step A shows the ordinary run detail"
        );

        type_str(&mut s, &mut ctx, "/gsd:execute-phase 18");
        press(&mut s, &mut ctx, KeyCode::Enter);

        let opened = preview(&ctx).expect("Step B opens the preview");
        assert_eq!(
            opened.command, "/gsd:execute-phase 18",
            "the preview is about the command that was actually committed"
        );
        assert_eq!(
            opened.report, None,
            "the preview goes in LOADING and the build is scheduled second — \
             the other order makes the loading idiom unreachable, because \
             `schedule_dry_run_report` returns immediately"
        );
        assert!(
            ctx.error_message.is_none(),
            "an opted-in project must schedule cleanly, got {:?}",
            ctx.error_message
        );
    }

    #[tokio::test]
    async fn keys_pressed_while_the_report_is_loading_are_inert_and_nothing_panics() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut ctx, mut rx) = ctx_with_project(dir.path());
        opted_in(&mut ctx);
        let mut s = DriverStartScreen::new(ALIAS.to_string());

        press(&mut s, &mut ctx, KeyCode::Enter);
        assert_eq!(s.step(), StartStep::Goal);

        // The preview owns no key, so every one of these belongs to the goal
        // field or to nothing at all. None may resolve, cancel or rebuild it.
        for code in [
            KeyCode::PageDown,
            KeyCode::PageUp,
            KeyCode::Up,
            KeyCode::Down,
            KeyCode::Tab,
            KeyCode::Left,
            KeyCode::Home,
            KeyCode::F(5),
        ] {
            let action = press(&mut s, &mut ctx, code);
            assert!(
                matches!(action, ScreenAction::None),
                "{code:?} must not move the flow while the preview is loading"
            );
        }

        let still = preview(&ctx).expect("the preview survives inert keys");
        assert_eq!(still.report, None, "no key resolves the report");
        assert_eq!(s.step(), StartStep::Goal);
        assert!(
            rx.try_recv().is_err(),
            "and nothing was started by looking at a preview"
        );
    }

    #[tokio::test]
    async fn leaving_step_b_in_any_direction_restores_the_normal_run_detail() {
        let dir = tempfile::tempdir().expect("temp dir");

        // Backwards: Esc returns to Step A.
        {
            let (mut ctx, _rx) = ctx_with_project(dir.path());
            opted_in(&mut ctx);
            let mut s = DriverStartScreen::new(ALIAS.to_string());
            type_str(&mut s, &mut ctx, "/gsd:plan-phase 19");
            press(&mut s, &mut ctx, KeyCode::Enter);
            assert!(preview(&ctx).is_some());

            press(&mut s, &mut ctx, KeyCode::Esc);
            assert_eq!(s.step(), StartStep::Command);
            assert!(
                preview(&ctx).is_none(),
                "a preview left up after the flow moved on describes a start \
                 that already happened"
            );
        }

        // Forwards: Enter pushes the confirmation.
        {
            let (mut ctx, _rx) = ctx_with_project(dir.path());
            opted_in(&mut ctx);
            let mut s = DriverStartScreen::new(ALIAS.to_string());
            type_str(&mut s, &mut ctx, "/gsd:plan-phase 19");
            press(&mut s, &mut ctx, KeyCode::Enter);
            press(&mut s, &mut ctx, KeyCode::Enter);
            assert!(preview(&ctx).is_none());
        }

        // Sideways: Esc at Step A pops the whole flow.
        {
            let (mut ctx, _rx) = ctx_with_project(dir.path());
            opted_in(&mut ctx);
            let mut s = DriverStartScreen::new(ALIAS.to_string());
            press(&mut s, &mut ctx, KeyCode::Enter);
            press(&mut s, &mut ctx, KeyCode::Esc);
            let action = press(&mut s, &mut ctx, KeyCode::Esc);
            assert!(matches!(action, ScreenAction::Pop));
            assert!(preview(&ctx).is_none());
        }
    }

    #[tokio::test]
    async fn a_project_that_never_opted_in_is_refused_visibly_and_shows_no_preview() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut ctx, mut rx) = ctx_with_project(dir.path());
        // Deliberately NOT opted in: the preview describes a drive that would
        // itself be refused, so building one would be a lie about what happens
        // next.
        let mut s = DriverStartScreen::new(ALIAS.to_string());
        type_str(&mut s, &mut ctx, "/gsd:progress");
        press(&mut s, &mut ctx, KeyCode::Enter);

        assert!(
            ctx.error_message.is_some(),
            "the refusal is visible, never silent (S4)"
        );
        assert_eq!(
            preview(&ctx).and_then(|p| p.report.clone()),
            None,
            "and no report ever arrives to fill it"
        );
        assert!(
            rx.try_recv().is_err(),
            "**and nothing was spawned.** This is not a second spawn gate — the \
             gate `tests/spawn_seam_guard.rs` pins is still the child's (D-16) — \
             but a preview must not become a side door either"
        );
    }
}
