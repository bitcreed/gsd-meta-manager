//! **Surface 4 — the injection input** (STEER-01, D-22).
//!
//! `enqueue.rs` file-for-file, with three deliberate differences and one
//! prohibition.
//!
//! 1. **Different footer copy.** `  Inject> ` rather than `  Enqueue> `, and a
//!    caret. The four existing text inputs in this tree fake a caret with a
//!    trailing `Span::raw("_")`; `enqueue.rs` is the one that omits it, so the
//!    caret is taken from `project_list.rs:279` / `add_project.rs:212` rather
//!    than from the file this one is modelled on.
//! 2. **No `Tab` arm and no `[Tab] suggestions` hint.**
//!    `queue_md::suggest_next_commands` suggests **GSD commands**, and a
//!    steering message is free prose. Offering irrelevant completions would be
//!    worse than offering none.
//! 3. **`Enter` dispatches an [`Action`]; it does not write.** This is the one
//!    thing that must not be copied from `enqueue.rs`, whose `Enter` arm
//!    performs the queue write inline on the render thread.
//!
//! **The key handler never touches the filesystem** (D-06, D-22, D-28). The
//! durable append — the append-only write plus the disk sync that makes it
//! survive the writing process, spelled out in D-06 — is 18-05's
//! `schedule_inbox_append`, on `spawn_blocking`. Blocking the render thread on a
//! disk sync is the WR-10 failure mode whose symptom is a frozen frame rather
//! than an error. **A grep of this file for any filesystem API is an acceptance
//! criterion of the plan that created it**, which is why the API names are named
//! nowhere in it, not even in prose: no import of the standard library's
//! filesystem module belongs here and there must never be one.
//!
//! **The id is minted here, at queue time** (D-05). Text alone is not a
//! correlation key — a user may legitimately send the same sentence twice — and
//! the `queued` state has to be addressable before any other process has seen
//! the line.
//!
//! ## The screen claims no delivery outcome
//!
//! Nothing in this file says a message was **sent**, **received**, **read** or
//! **acknowledged**; those words are forbidden phase-wide for an injected
//! message (D-07), because `isReplay: true` arrives at DEQUEUE, measured ~55 s
//! after the write. The controller's success copy is
//! `Queued — waiting for the driver to pick it up.` and it deliberately promises
//! nothing about timing.
//!
//! ## The caller's contract
//!
//! This screen is only constructible with a **live** run's id, and the caller —
//! the Driver tab's `i` binding — is responsible for refusing otherwise, with
//! this exact copy:
//!
//! ```text
//! No live run on "{alias}" — nothing to inject into.
//! ```
//!
//! set on `ctx.status_message`, **without** opening the screen and **without**
//! dispatching anything. The copy lives here as well as at the call site so the
//! two cannot drift.

use super::{AppContext, Screen, ScreenAction};
use crate::action::Action;
use crate::journal::inbox;
use crossterm::event::{KeyCode, KeyModifiers};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

/// The footer hint, as a named constant so its **absences** are assertable.
///
/// There is no `[Tab] suggestions` here and there is no `Tab` arm in
/// [`DriverInjectScreen::handle_key`] — `queue_md::suggest_next_commands`
/// suggests GSD commands and a steering message is free prose. The two
/// guarantees are checked from opposite directions: a test asserts this string
/// offers no completion, and a grep asserts the key handler names no completion
/// key, so `Tab` provably falls through to the same do-nothing arm as any other
/// unbound key.
const FOOTER_HINT: &str = "   [Enter] queue  [Esc] cancel";

/// The refusal the caller must set when `alias` has no live run.
///
/// Formatted here rather than at the call site so the screen and its only
/// opener cannot describe the same condition two different ways.
pub fn no_live_run_message(alias: &str) -> String {
    format!("No live run on \"{alias}\" — nothing to inject into.")
}

/// A one-line free-prose input aimed at one live run.
pub struct DriverInjectScreen {
    alias: String,
    /// The id of the run the message is aimed at.
    ///
    /// **Contract:** the caller passes the id of a run that is live *now*. This
    /// screen does not re-check liveness — a run can finish between the
    /// keypress that opened the screen and the keypress that queues the
    /// message, and the honest answer to that race is the `missed` state
    /// observed by the driver, not a guess made here.
    run_id: String,
}

impl DriverInjectScreen {
    /// `run_id` must name a run that is live at the moment the screen is
    /// opened; see the field's own contract.
    pub fn new(alias: String, run_id: String) -> Self {
        Self { alias, run_id }
    }
}

impl Screen for DriverInjectScreen {
    fn handle_key(
        &mut self,
        code: KeyCode,
        _modifiers: KeyModifiers,
        ctx: &mut AppContext,
    ) -> ScreenAction {
        match code {
            KeyCode::Enter => {
                // Empty Enter is a no-op and the screen stays open, exactly as
                // `enqueue.rs` does. Unlike the start flow's command step there
                // is no sensible default for a steering message, so there is
                // nothing to accept.
                if ctx.input_buffer.is_empty() {
                    return ScreenAction::None;
                }

                let text = std::mem::take(&mut ctx.input_buffer);
                let action = Action::DriverInjectRequested {
                    alias: self.alias.clone(),
                    run_id: self.run_id.clone(),
                    id: inbox::new_message_id(),
                    text,
                };
                // `driver_confirm::dispatch` rather than a second mechanism: it
                // already reports visibly when there is no channel to reach the
                // seam through, and an action the user asked for that quietly
                // went nowhere is indistinguishable from a hang.
                super::driver_confirm::dispatch(ctx, action, &self.alias);
                ScreenAction::Pop
            }
            KeyCode::Esc => {
                ctx.input_buffer.clear();
                ctx.needs_redraw = true;
                ScreenAction::Pop
            }
            KeyCode::Backspace => {
                ctx.input_buffer.pop();
                ctx.needs_redraw = true;
                ScreenAction::None
            }
            KeyCode::Char(c) => {
                ctx.input_buffer.push(c);
                ctx.needs_redraw = true;
                ScreenAction::None
            }
            // The completion key falls through here with every other unbound
            // key, deliberately: no completion source is right for a free-prose
            // steering message.
            _ => ScreenAction::None,
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, ctx: &AppContext) {
        let chunks = Layout::vertical([Constraint::Min(0), Constraint::Length(1)]).split(area);

        // The body is the detail view the user was already looking at, so the
        // Driver tab stays visible behind the input.
        let detail = super::detail::DetailScreen::new(self.alias.clone());
        detail.render_main_only(frame, chunks[0], ctx);

        let footer = Paragraph::new(Line::from(vec![
            Span::styled("  Inject> ", Style::default().add_modifier(Modifier::BOLD)),
            Span::raw(&ctx.input_buffer),
            Span::raw("_"),
            Span::styled(FOOTER_HINT, Style::default().fg(Color::DarkGray)),
        ]));
        frame.render_widget(footer, chunks[1]);
    }

    fn name(&self) -> &str {
        "driver_inject"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ui::screens::driver_confirm::tests::{ctx_with_project, ALIAS};

    const RUN: &str = "run-abc";

    fn press(screen: &mut dyn Screen, ctx: &mut AppContext, code: KeyCode) -> ScreenAction {
        screen.handle_key(code, KeyModifiers::NONE, ctx)
    }

    fn type_str(screen: &mut dyn Screen, ctx: &mut AppContext, s: &str) {
        for c in s.chars() {
            press(screen, ctx, KeyCode::Char(c));
        }
    }

    fn screen() -> DriverInjectScreen {
        DriverInjectScreen::new(ALIAS.to_string(), RUN.to_string())
    }

    #[test]
    fn typing_appends_to_the_shared_buffer_and_backspace_pops() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut ctx, _rx) = ctx_with_project(dir.path());
        let mut s = screen();

        type_str(&mut s, &mut ctx, "abc");
        assert_eq!(ctx.input_buffer, "abc");

        let action = press(&mut s, &mut ctx, KeyCode::Backspace);
        assert!(matches!(action, ScreenAction::None));
        assert_eq!(ctx.input_buffer, "ab");
    }

    #[test]
    fn enter_on_a_non_empty_buffer_dispatches_one_injection_and_clears_the_buffer() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut ctx, mut rx) = ctx_with_project(dir.path());
        let mut s = screen();

        type_str(&mut s, &mut ctx, "skip the UI review");
        let action = press(&mut s, &mut ctx, KeyCode::Enter);
        assert!(matches!(action, ScreenAction::Pop));

        let sent = rx.try_recv().expect("a non-empty Enter must dispatch");
        let Action::DriverInjectRequested {
            alias,
            run_id,
            id,
            text,
        } = sent
        else {
            panic!("expected DriverInjectRequested, got {sent:?}");
        };
        assert_eq!(alias, ALIAS);
        assert_eq!(run_id, RUN);
        assert_eq!(text, "skip the UI review", "the text travels verbatim");
        assert!(
            !id.is_empty(),
            "the correlation id is minted at queue time (D-05) — without it the \
             queued state is not addressable and two identical sentences are \
             indistinguishable"
        );

        assert!(
            rx.try_recv().is_err(),
            "one keypress must queue exactly one message"
        );
        assert!(
            ctx.input_buffer.is_empty(),
            "the shared buffer must not leak into the next screen that uses it"
        );
    }

    #[test]
    fn enter_on_an_empty_buffer_dispatches_nothing_and_keeps_the_screen_open() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut ctx, mut rx) = ctx_with_project(dir.path());
        let mut s = screen();

        let action = press(&mut s, &mut ctx, KeyCode::Enter);
        assert!(
            matches!(action, ScreenAction::None),
            "an empty Enter must leave the screen open, as `enqueue.rs` does"
        );
        assert!(
            rx.try_recv().is_err(),
            "an empty Enter must dispatch NOTHING — a screen that returned `None` \
             and dispatched anyway would append a blank line to inbox.jsonl while \
             telling the user it did nothing"
        );
    }

    #[test]
    fn esc_clears_the_buffer_dispatches_nothing_and_pops() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut ctx, mut rx) = ctx_with_project(dir.path());
        let mut s = screen();

        type_str(&mut s, &mut ctx, "never mind");
        let action = press(&mut s, &mut ctx, KeyCode::Esc);
        assert!(matches!(action, ScreenAction::Pop));
        assert!(ctx.input_buffer.is_empty());
        assert!(
            rx.try_recv().is_err(),
            "cancelling must queue nothing — an abandoned message that reached \
             disk anyway is the opposite of cancelling"
        );
    }

    #[test]
    fn a_multi_byte_message_round_trips_through_the_dispatched_action_byte_identically() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut ctx, mut rx) = ctx_with_project(dir.path());
        let mut s = screen();

        let msg = "🚀 arrêter après l'étape 3 — 日本語も";
        type_str(&mut s, &mut ctx, msg);
        press(&mut s, &mut ctx, KeyCode::Enter);

        let sent = rx.try_recv().expect("dispatch");
        let Action::DriverInjectRequested { text, .. } = sent else {
            panic!("expected DriverInjectRequested, got {sent:?}");
        };
        assert_eq!(
            text.as_bytes(),
            msg.as_bytes(),
            "the human's words are the one input in this system that originates \
             with the human; they travel verbatim"
        );
    }

    /// The no-completion guarantee, from both directions.
    ///
    /// The handler names no completion key at all — a grep for one is an
    /// acceptance criterion — so the completion key reaches the same
    /// do-nothing arm this test exercises with an arbitrary unbound key. The
    /// second half asserts the hint offers no completion either, so the screen
    /// cannot advertise something it does not do.
    #[test]
    fn an_unbound_key_is_consumed_and_the_footer_advertises_no_completions() {
        let dir = tempfile::tempdir().expect("temp dir");
        let (mut ctx, mut rx) = ctx_with_project(dir.path());
        let mut s = screen();

        type_str(&mut s, &mut ctx, "st");
        let action = press(&mut s, &mut ctx, KeyCode::Down);
        assert!(matches!(action, ScreenAction::None));
        assert_eq!(
            ctx.input_buffer, "st",
            "an unbound key must not overwrite what the user typed"
        );
        assert!(rx.try_recv().is_err());

        assert!(
            !FOOTER_HINT.contains("Tab"),
            "`suggest_next_commands` suggests GSD commands; a steering message is \
             free prose, and offering irrelevant completions would be worse than \
             offering none. Got: {FOOTER_HINT}"
        );
        assert!(
            FOOTER_HINT.contains("[Enter] queue") && FOOTER_HINT.contains("[Esc] cancel"),
            "got: {FOOTER_HINT}"
        );
    }
}
