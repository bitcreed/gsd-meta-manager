//! The TUI event loop's testable core (D-17, TRANS-03).
//!
//! # Why this module exists at all
//!
//! The loop body used to live inside `run_tui_loop` in `src/main.rs`. That file
//! is the **binary**, not the library, so nothing defined there is reachable
//! from a `cargo test --lib` target, which means the loop's priority ordering
//! had no way to be tested. Moving the `select!` body into a library module is
//! the whole reason the tests at the bottom of this file can exist.
//!
//! # What the shape buys
//!
//! `run_tui_loop` becomes: sync the redraw flag, draw, [`pump`], the preserved
//! editor shell-out, the quit check. Everything interesting is in [`pump`], and
//! [`pump`] needs no terminal.
//!
//! # The two arms, in priority order
//!
//! `tokio::select!` is `biased;`, so **arm order is priority order**:
//!
//! 1. **Input and existing actions.** A control key can never queue behind
//!    other traffic.
//! 2. **The loop's own redraw timer.** Before this, the loop could only make
//!    progress when a message arrived, and was rescued only incidentally by the
//!    250ms `spawn_tick`.
//!
//! Until Phase 31 a third, bounded arm drained the in-process executor's event
//! channel between the two. Nothing has sent on that channel since the legacy
//! `drive` path went, so the arm and the channel are gone.

use crate::action::Action;
use crate::app::App;
use std::time::Duration;
use tokio::sync::mpsc;

/// How long [`pump`] waits with nothing to do before returning on its own.
///
/// ~60Hz. This is the arm that lets the loop make progress with no message at
/// all. Also an untuned starting value (A6).
pub const REDRAW_INTERVAL: Duration = Duration::from_millis(16);

/// What one [`pump`] iteration did.
///
/// Returned rather than swallowed so the loop (and the tests) can distinguish
/// "an action was handled" from "nothing happened for a whole redraw interval".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PumpOutcome {
    /// One [`Action`] was applied.
    Action,
    /// The redraw interval elapsed with the message arm idle.
    RedrawTick,
    /// Every arm was disabled: the action channel is closed. Unreachable in
    /// practice because the timer arm's pattern is irrefutable, but see
    /// [`pump`]'s Pitfall C note for why the branch exists anyway.
    Idle,
}

/// Run exactly one iteration of the TUI event loop's message handling.
///
/// Draws nothing and owns no terminal — the caller draws before calling this.
///
/// # Pitfall C: the arm that disables itself
///
/// `tokio::select!` **permanently disables** an arm whose pattern fails to
/// match, and a closed `mpsc` receiver yields `None`, which fails the
/// `Some(..)` pattern on arm 1. If every arm disables, `select!` panics. The
/// `else` branch below makes the panic impossible even if the action sender is
/// ever dropped.
///
/// Note that `select!` disables an arm only for the lifetime of *one* `select!`
/// expression. Because `pump` builds a fresh one per call, a disabled arm also
/// recovers on the next iteration.
pub async fn pump(app: &mut App, action_rx: &mut mpsc::UnboundedReceiver<Action>) -> PumpOutcome {
    tokio::select! {
        // Arm order IS priority order.
        biased;

        // 1. Input and existing actions win every iteration, so a control key
        //    can never queue behind other traffic (TRANS-03, D-17).
        //
        //    Because this arm is biased FIRST, routing anything high-volume
        //    through the `Action` FIFO would starve the redraw timer. Today's
        //    producers (keys, the 250ms tick, debounced watcher events) are
        //    nowhere near continuous, and that is a precondition of this
        //    design, not an accident of it.
        Some(action) = action_rx.recv() => {
            app.update(action);
            PumpOutcome::Action
        }

        // 2. The loop's own redraw timer. A fresh `sleep` per iteration is
        //    deliberately used instead of a long-lived `tokio::time::Interval`:
        //    `pump` is a free function, so it cannot own an `Interval` across
        //    calls, and an `Interval` reconstructed per call fires its first
        //    tick immediately, which would spin the loop. A per-iteration
        //    `sleep` is exactly `MissedTickBehavior::Delay` — the delay starts
        //    when the previous tick was consumed — so the two are behaviourally
        //    identical here, and this form is the one that survives extraction.
        _ = tokio::time::sleep(REDRAW_INTERVAL) => PumpOutcome::RedrawTick,

        // Pitfall C backstop. See the doc comment above.
        else => PumpOutcome::Idle,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn key(code: KeyCode) -> Action {
        Action::RawKey(KeyEvent::new(code, KeyModifiers::NONE))
    }

    /// A queued keypress is applied by the first `pump` iteration and reported
    /// as an `Action`, ahead of the redraw timer.
    #[tokio::test]
    async fn a_queued_keypress_is_applied_ahead_of_the_redraw_timer() {
        let (act_tx, mut act_rx) = mpsc::unbounded_channel();
        act_tx.send(key(KeyCode::Char('q'))).expect("send keypress");

        let mut app = App::new_for_test();
        assert!(!app.should_quit, "precondition: app starts not quitting");

        let outcome = pump(&mut app, &mut act_rx).await;

        assert!(app.should_quit, "the keypress was not applied");
        assert_eq!(outcome, PumpOutcome::Action);
    }

    /// With nothing queued the redraw timer fires, so the loop makes progress
    /// with no message at all.
    #[tokio::test]
    async fn an_idle_loop_returns_on_the_redraw_timer() {
        // Keep `_act_tx` alive: dropping it closes the channel, which would
        // disable arm 1 and leave the test proving something weaker.
        let (_act_tx, mut act_rx) = mpsc::unbounded_channel::<Action>();
        let mut app = App::new_for_test();

        assert_eq!(pump(&mut app, &mut act_rx).await, PumpOutcome::RedrawTick);
    }

    /// A closed action channel does not panic the loop (Pitfall C): the timer
    /// arm still answers.
    #[tokio::test]
    async fn a_closed_action_channel_still_lets_the_loop_make_progress() {
        let (act_tx, mut act_rx) = mpsc::unbounded_channel::<Action>();
        drop(act_tx);
        let mut app = App::new_for_test();

        assert_eq!(pump(&mut app, &mut act_rx).await, PumpOutcome::RedrawTick);
    }
}
