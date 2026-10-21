//! The Driver tab's steering inputs (Phase 30, plan 30-04): the control chooser, and
//! the copy the App shows after a steering write.
//!
//! **A key handler here never touches the filesystem** (D-25). Each screen collects
//! one decision and dispatches an [`Action`] on `ctx.event_tx`; the App runs the
//! blocking write on `spawn_blocking` ([`crate::driver::steer::steer_run`], a
//! `write_body` closure under `DIRECTION.lock`) and answers with
//! [`Action::DriverSteered`]. The file contains no file-write call at all, and a test
//! pins that.
//!
//! **No copy here says a control or an answer took effect.** The driver has not
//! echoed it in the frontmatter yet, so "written to DIRECTION.md" is the strongest
//! thing true (D-28): never `paused`, `stopped` or `sent`, and no spinner.
//!
//! Every value drawn or interpolated that came from a file goes through
//! [`crate::text::render_for_terminal`].

use super::driver_confirm::dispatch;
use super::{AppContext, Screen, ScreenAction};
use crate::action::Action;
use crate::driver::steer::{SteerOp, SteerOutcome};
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// What every steering key says on a build that cannot write `DIRECTION.md` safely
/// (D-14): the writer is `renameat2(RENAME_EXCHANGE)`, which is Linux-only.
pub const STEERING_NEEDS_LINUX: &str = "read-only: steering needs Linux";

/// Whether this build can steer. A constant, not a probe: the write primitive exists
/// on Linux and nowhere else.
pub const fn can_steer() -> bool {
    cfg!(target_os = "linux")
}

fn bold() -> Style {
    Style::default().add_modifier(Modifier::BOLD)
}

fn muted() -> Style {
    Style::default().fg(Color::DarkGray)
}

/// The appended sentence when the write went ahead without `DIRECTION.lock` (27 D-20,
/// orchestrator decision 3 `[inferred — audit]`).
pub const LOCK_TIMEOUT_NOTE: &str =
    " Written without DIRECTION.lock (another writer held it); the exchange check kept every save.";

/// The status line (or error line) for a finished steering write, and whether it is
/// an error.
///
/// `live` is whether the run's lock was held when the result arrived: a write to a run
/// that is not running applies at its next launch, not before its next dispatch.
/// Nothing here claims the driver has applied anything (D-28).
pub fn steered_message(op: &SteerOp, outcome: &SteerOutcome, live: bool) -> (String, bool) {
    let render = |text: &str| crate::text::render_for_terminal(text).to_string();
    match (op, outcome) {
        (SteerOp::SetControl(word), SteerOutcome::Written { lock_timed_out }) => {
            let word = render(word);
            let mut text = if live {
                format!(
                    "control: {word} written to DIRECTION.md. The driver applies it before its next dispatch."
                )
            } else {
                format!("control: {word} written to DIRECTION.md. It applies at the next launch.")
            };
            if *lock_timed_out {
                text.push_str(LOCK_TIMEOUT_NOTE);
            }
            (text, false)
        }
        (SteerOp::SetControl(word), SteerOutcome::Unchanged) => (
            format!(
                "control is already {} in DIRECTION.md. Nothing was written.",
                render(word)
            ),
            false,
        ),
        (_, SteerOutcome::Refused(error)) => (
            format!("Nothing was written: {}.", render(&error.to_string())),
            true,
        ),
        (_, SteerOutcome::NotWritten(reason)) => (
            format!(
                "Could not write DIRECTION.md: {}. The file was not changed. Try again.",
                render(reason)
            ),
            true,
        ),
        (_, SteerOutcome::Failed(error)) => (
            format!(
                "Could not write DIRECTION.md: {}. The file was not changed. Try again.",
                render(error)
            ),
            true,
        ),
    }
}

// ── The control chooser (`c`, D-15) ─────────────────────────────────────────

/// `c`: the one-line chooser `control (now: {word})>  [r]un  [p]ause  [s]top
/// [Esc] cancel`.
///
/// `r` and `p` write at once; choosing the word the body already has writes nothing
/// and says so. `s` opens the polite-stop confirmation (the stop is the one choice
/// that ends the run, so it asks). `Esc` cancels.
pub struct ControlChooserScreen {
    alias: String,
    run_id: String,
    /// The control word the body holds now (the cached detail's `body_control`);
    /// `None` when the body has no `- control:` line.
    current: Option<String>,
}

impl ControlChooserScreen {
    pub fn new(alias: String, run_id: String, current: Option<String>) -> Self {
        Self {
            alias,
            run_id,
            current,
        }
    }

    /// `word` is what the body already says.
    fn is_current(&self, word: &str) -> bool {
        self.current
            .as_deref()
            .is_some_and(|current| current.eq_ignore_ascii_case(word))
    }

    /// Choose `word`: nothing is dispatched when the body already says it.
    fn choose(&self, word: &str, ctx: &mut AppContext) -> ScreenAction {
        if self.is_current(word) {
            let (text, _) = steered_message(
                &SteerOp::SetControl(word.to_string()),
                &SteerOutcome::Unchanged,
                false,
            );
            ctx.status_message = Some((text, std::time::Instant::now()));
            ctx.needs_redraw = true;
            return ScreenAction::Pop;
        }
        dispatch(
            ctx,
            Action::DriverSteerRequested {
                alias: self.alias.clone(),
                run_id: self.run_id.clone(),
                op: SteerOp::SetControl(word.to_string()),
            },
            &self.alias,
        );
        ScreenAction::Pop
    }
}

crate::ui::screens::adjudicate_screen!(
    ControlChooserScreen,
    crate::ui::screens::RENDERS_ATTACKER_INFLUENCED_IDENTITY,
    "Paints its body with `DetailScreen::render_main_only` (the registry alias and the \
     run list) and its footer names the control word the run's DIRECTION.md body holds \
     now, which is file content a sibling run or a cloned repository can set. The word \
     is drawn through `render_for_terminal`; the run id is carried for the dispatch and \
     never drawn. Fixture states: the hostile identity as the current word.",
);

impl Screen for ControlChooserScreen {
    fn handle_key(
        &mut self,
        code: KeyCode,
        _modifiers: KeyModifiers,
        ctx: &mut AppContext,
    ) -> ScreenAction {
        match code {
            KeyCode::Char('r') => self.choose("run", ctx),
            KeyCode::Char('p') => self.choose("pause", ctx),
            KeyCode::Char('s') => {
                // Task 2 wires the polite-stop confirmation; until then it is inert.
                ScreenAction::None
            }
            KeyCode::Esc => {
                ctx.needs_redraw = true;
                ScreenAction::Pop
            }
            _ => ScreenAction::None,
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        let chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(area);
        let detail = super::detail::DetailScreen::new(self.alias.clone());
        detail.render_main_only(frame, chunks[0], ctx);

        let now = match self.current.as_deref() {
            Some(word) => crate::text::render_for_terminal(word).to_string(),
            None => "no control line".to_string(),
        };
        let key = |key: &'static str, rest: &'static str| {
            [Span::styled(key, bold()), Span::styled(rest, muted())]
        };
        let mut spans = vec![Span::styled(format!("  control (now: {now})> "), bold())];
        spans.extend(key("[r]", "un  "));
        spans.extend(key("[p]", "ause  "));
        spans.extend(key("[s]", "top    "));
        spans.push(Span::styled("[Esc] cancel", muted()));
        frame.render_widget(Paragraph::new(Line::from(spans)), chunks[1]);
    }

    fn name(&self) -> &str {
        "driver_control"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::screens::driver_confirm::tests::{ctx_with_project, ALIAS};
    use tokio::sync::mpsc::UnboundedReceiver;

    const RUN: &str = "20261019-run0";

    fn chooser(
        current: Option<&str>,
    ) -> (ControlChooserScreen, AppContext, UnboundedReceiver<Action>) {
        let dir = tempfile::tempdir().expect("temp dir");
        let (ctx, rx) = ctx_with_project(dir.path());
        (
            ControlChooserScreen::new(
                ALIAS.to_string(),
                RUN.to_string(),
                current.map(str::to_string),
            ),
            ctx,
            rx,
        )
    }

    fn press(screen: &mut ControlChooserScreen, ctx: &mut AppContext, key: char) -> ScreenAction {
        screen.handle_key(KeyCode::Char(key), KeyModifiers::NONE, ctx)
    }

    #[test]
    fn launcher_steer_r_and_p_dispatch_a_set_control_and_pop() {
        for (key, word) in [('r', "run"), ('p', "pause")] {
            let (mut screen, mut ctx, mut rx) = chooser(Some("stop"));
            let action = press(&mut screen, &mut ctx, key);
            assert!(matches!(action, ScreenAction::Pop), "{key}");
            let Some(Action::DriverSteerRequested { alias, run_id, op }) = rx.try_recv().ok()
            else {
                panic!("{key} must dispatch a steering request");
            };
            assert_eq!(alias, ALIAS);
            assert_eq!(run_id, RUN);
            assert_eq!(op, SteerOp::SetControl(word.to_string()));
            assert!(rx.try_recv().is_err(), "exactly one request");
        }
    }

    #[test]
    fn launcher_steer_choosing_the_current_word_writes_nothing_and_says_so() {
        let (mut screen, mut ctx, mut rx) = chooser(Some("Pause"));
        let action = press(&mut screen, &mut ctx, 'p');
        assert!(matches!(action, ScreenAction::Pop));
        assert!(rx.try_recv().is_err(), "nothing is dispatched");
        let (text, _) = ctx.status_message.clone().expect("a status message");
        assert_eq!(
            text,
            "control is already pause in DIRECTION.md. Nothing was written."
        );
    }

    #[test]
    fn launcher_steer_escape_pops_and_other_keys_are_inert() {
        let (mut screen, mut ctx, mut rx) = chooser(None);
        for key in ['x', 'a', 'q'] {
            assert!(matches!(
                press(&mut screen, &mut ctx, key),
                ScreenAction::None
            ));
        }
        assert!(matches!(
            screen.handle_key(KeyCode::Esc, KeyModifiers::NONE, &mut ctx),
            ScreenAction::Pop
        ));
        assert!(rx.try_recv().is_err());
    }

    #[test]
    fn launcher_steer_the_copy_never_claims_the_driver_applied_it() {
        let op = SteerOp::SetControl("pause".to_string());
        let written = SteerOutcome::Written {
            lock_timed_out: false,
        };
        let (live, error) = steered_message(&op, &written, true);
        assert!(!error);
        assert_eq!(
            live,
            "control: pause written to DIRECTION.md. The driver applies it before its next dispatch."
        );
        let (idle, _) = steered_message(&op, &written, false);
        assert_eq!(
            idle,
            "control: pause written to DIRECTION.md. It applies at the next launch."
        );
        for text in [&live, &idle] {
            for word in ["paused", "stopped", "sent"] {
                assert!(!text.contains(word), "{text} says {word}");
            }
        }
    }

    #[test]
    fn launcher_steer_a_lock_timeout_is_surfaced_not_hidden() {
        let op = SteerOp::SetControl("stop".to_string());
        let (text, error) = steered_message(
            &op,
            &SteerOutcome::Written {
                lock_timed_out: true,
            },
            true,
        );
        assert!(!error);
        assert!(text.contains("Written without DIRECTION.lock"), "{text}");
    }

    #[test]
    fn launcher_steer_failures_name_the_problem_and_that_nothing_changed() {
        let op = SteerOp::SetControl("run".to_string());
        for outcome in [
            SteerOutcome::Failed("DIRECTION.md I/O failed: disk full".to_string()),
            SteerOutcome::NotWritten("the run has no DIRECTION.md".to_string()),
        ] {
            let (text, error) = steered_message(&op, &outcome, true);
            assert!(error, "{text}");
            assert!(text.starts_with("Could not write DIRECTION.md: "), "{text}");
            assert!(
                text.ends_with("The file was not changed. Try again."),
                "{text}"
            );
        }
    }

    #[test]
    fn launcher_steer_the_screens_never_write_a_file() {
        let source = include_str!("driver_steer.rs");
        let needle = ["fs::", "write("].concat();
        let create = ["File::", "create("].concat();
        let (code, _) = source
            .split_once("#[cfg(test)]")
            .expect("the file has a test module");
        assert!(!code.contains(&needle) && !code.contains(&create));
    }
}
