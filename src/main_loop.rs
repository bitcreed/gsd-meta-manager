//! The TUI event loop's testable core (D-17, TRANS-03).
//!
//! # Why this module exists at all
//!
//! The loop body used to live inside `run_tui_loop` in `src/main.rs`. That file
//! is the **binary**, not the library, so nothing defined there is reachable
//! from a `cargo test --lib` target — which means the loop's priority ordering,
//! the property TRANS-03 actually asks about, had no way to be tested. Moving
//! the `select!` body into a library module is the whole reason the three
//! property tests at the bottom of this file can exist.
//!
//! # What the shape buys
//!
//! `run_tui_loop` becomes: sync the redraw flag, draw, [`pump`], the preserved
//! editor shell-out, the quit check. Everything interesting is in [`pump`], and
//! [`pump`] needs no terminal.
//!
//! # The three arms, in priority order
//!
//! `tokio::select!` is `biased;`, so **arm order is priority order**:
//!
//! 1. **Input and existing actions.** A control key can never queue behind bulk
//!    stream traffic — that is precisely the failure TRANS-03 forbids.
//! 2. **Executor events, drained in a bounded batch.** A burst returns control
//!    to the loop head after at most [`EXEC_BATCH`] events instead of starving
//!    the redraw.
//! 3. **The loop's own redraw timer.** Before this, the loop could only make
//!    progress when a message arrived, and was rescued only incidentally by the
//!    250ms `spawn_tick`.
//!
//! The executor channel is deliberately **separate from and bounded** unlike the
//! `Action` FIFO. See [`pump`]'s arm comments for why both halves of that
//! sentence are load-bearing.

use crate::action::Action;
use crate::app::App;
use crate::executor::ExecutionEvent;
use std::time::Duration;
use tokio::sync::mpsc;

/// The maximum number of executor events a single [`pump`] iteration applies.
///
/// This bound is what stops a burst starving a redraw: once `EXEC_BATCH` events
/// have been applied, control returns to the loop head, which draws a frame and
/// re-polls the input arm.
///
/// 64 is a defensible starting value with no tuning data behind it (RESEARCH
/// assumption A6, D-13). It is a named constant so tuning is a one-line change.
pub const EXEC_BATCH: usize = 64;

/// How long [`pump`] waits with nothing to do before returning on its own.
///
/// ~60Hz. This is the arm that lets the loop make progress with no message at
/// all. Also an untuned starting value (A6).
pub const REDRAW_INTERVAL: Duration = Duration::from_millis(16);

/// Capacity of the process-lifetime executor channel.
///
/// Generous on purpose. The channel is bounded so that a render loop blocked in
/// the editor shell-out applies **backpressure** to the reader task rather than
/// growing without limit (T-15-26) — but backpressure on the reader risks
/// Claude's thirty-second exit drain, so the buffer wants a lot of headroom
/// before that backpressure is ever felt.
pub const EXEC_CHANNEL_CAPACITY: usize = 8192;

/// One executor event, tagged with the project alias whose run produced it.
///
/// The channel is created **once for the process lifetime** and is shared by
/// every alias that is ever driven, so a bare [`ExecutionEvent`] would be
/// unroutable: [`App::apply_exec_event`] keys the driver state map by alias and
/// has no other way to learn which run an event came from.
#[derive(Debug, Clone)]
pub struct ExecEvent {
    /// The registered project alias this event belongs to.
    pub alias: String,
    /// The observed event.
    pub event: ExecutionEvent,
}

impl ExecEvent {
    /// Tag an executor event with the alias whose run produced it.
    pub fn new(alias: impl Into<String>, event: ExecutionEvent) -> Self {
        Self {
            alias: alias.into(),
            event,
        }
    }
}

/// What one [`pump`] iteration did.
///
/// Returned rather than swallowed so the loop (and the tests) can distinguish
/// "an action was handled" from "a burst was partially drained" from "nothing
/// happened for a whole redraw interval".
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PumpOutcome {
    /// One [`Action`] was applied.
    Action,
    /// This many executor events were applied — never more than the `batch`
    /// argument, which is the bounded-drain guarantee.
    ExecEvents(usize),
    /// The redraw interval elapsed with both message arms idle.
    RedrawTick,
    /// Every arm was disabled: both channels are closed. Unreachable in
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
/// `Some(..)` patterns on arms 1 and 2. If every arm disables, `select!`
/// panics. The executor channel legitimately has no producer between runs, so
/// this is guarded twice over:
///
/// * the channel is created **once for the process lifetime** and a long-lived
///   sender clone is stashed on `AppContext`, so it never closes; **and**
/// * the `else` branch below makes the panic impossible even if that clone is
///   ever dropped.
///
/// Note that `select!` disables an arm only for the lifetime of *one* `select!`
/// expression. Because `pump` builds a fresh one per call, a disabled arm also
/// recovers on the next iteration — belt, braces, and a spare pair of braces.
pub async fn pump(
    app: &mut App,
    action_rx: &mut mpsc::UnboundedReceiver<Action>,
    exec_rx: &mut mpsc::Receiver<ExecEvent>,
    batch: usize,
) -> PumpOutcome {
    tokio::select! {
        // Arm order IS priority order.
        biased;

        // 1. Input and existing actions win every iteration, so a control key
        //    can never queue behind bulk stream traffic (TRANS-03, D-17).
        //
        //    The flip side, which is the reason this comment exists: because
        //    this arm is biased FIRST, routing anything high-volume through the
        //    `Action` FIFO in a future phase would starve arms 2 and 3. D-17's
        //    rule that per-token stream output must not travel through the
        //    `Action` FIFO is load-bearing, not stylistic. Today's producers —
        //    keys, the 250ms tick, debounced watcher events — are nowhere near
        //    continuous, and that is a precondition of this design, not an
        //    accident of it.
        Some(action) = action_rx.recv() => {
            app.update(action);
            PumpOutcome::Action
        }

        // 2. Executor events, drained in a BOUNDED batch. Handle the received
        //    event, then take up to `batch - 1` more with non-blocking receives,
        //    stopping at the first empty-or-closed result. The bound is what
        //    stops a burst starving a redraw.
        Some(ev) = exec_rx.recv() => {
            app.apply_exec_event(ev);
            let mut applied = 1usize;
            while applied < batch {
                match exec_rx.try_recv() {
                    Ok(ev) => {
                        app.apply_exec_event(ev);
                        applied += 1;
                    }
                    Err(_) => break,
                }
            }
            PumpOutcome::ExecEvents(applied)
        }

        // 3. The loop's own redraw timer. A fresh `sleep` per iteration is
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
