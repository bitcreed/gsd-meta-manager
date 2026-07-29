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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::executor::ExecutionEvent;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;
    use std::time::Instant;

    // ── Why these tests flood with ten thousand events ────────────────────
    //
    // Normal operation will NOT stress this loop. The 15-01 spike's 68-second
    // turn emitted roughly 27 events in total — 22 `thinking_tokens`, 2
    // `assistant`, 1 `user`, 1 `init`, 1 `result` — i.e. well under one event
    // per second, because the partial-message flag that produces token-level
    // deltas is deliberately absent from D-01's argv baseline.
    //
    // That is exactly why the flood tests below use ten thousand events: a test
    // that replays realistic traffic proves nothing about a design guarding
    // against pressure that realistic traffic never applies. The test has to
    // manufacture the pressure itself.
    //
    // If a later phase wants token-level rendering (Phase 18), turning on
    // partial messages raises the event rate by orders of magnitude. The
    // bounded batch drain is what makes that safe, and these tests are what
    // prove it still holds.
    //
    // Deliberately NOT written: any wall-clock keypress-latency percentile
    // assertion. It passes on a developer laptop, fails on a loaded runner,
    // gets `#[ignore]`d within a month, and at that point TRANS-03 has no
    // verification at all. Properties 1 and 2 below are PRIORITY and PROGRESS
    // properties, which is what makes them deterministic.

    const FLOOD: usize = 10_000;

    /// A cheap stand-in for a real stream event. The loop routes envelopes and
    /// does not interpret them, so the variant is irrelevant to what is proven.
    fn dummy_event(alias: &str) -> ExecEvent {
        ExecEvent::new(alias, ExecutionEvent::Stderr("noise".to_string()))
    }

    fn key(code: KeyCode) -> Action {
        Action::RawKey(KeyEvent::new(code, KeyModifiers::NONE))
    }

    /// Property 1 (the real one, TRANS-03 / D-17): a keypress is never queued
    /// behind stream traffic.
    ///
    /// A naive FIFO drains the flood first and fails this. Removing `biased;`
    /// fails this. Merging the executor channel into the `Action` FIFO fails
    /// this. It is a priority property, so it is deterministic — no timing, no
    /// terminal, no flake.
    #[tokio::test]
    async fn keypress_is_handled_before_a_flood_of_executor_events() {
        let (act_tx, mut act_rx) = mpsc::unbounded_channel();
        let (exec_tx, mut exec_rx) = mpsc::channel(EXEC_CHANNEL_CAPACITY * 2);

        // Flood the executor channel FIRST.
        for _ in 0..FLOOD {
            exec_tx
                .try_send(dummy_event("acme"))
                .expect("executor channel must hold the whole flood");
        }
        // THEN enqueue exactly one quit keypress.
        act_tx.send(key(KeyCode::Char('q'))).expect("send keypress");

        let mut app = App::new_for_test();
        assert!(!app.should_quit, "precondition: app starts not quitting");

        // Exactly ONE pump iteration.
        let outcome = pump(&mut app, &mut act_rx, &mut exec_rx, EXEC_BATCH).await;

        assert!(
            app.should_quit,
            "keypress starved behind {} queued executor events (pump returned {:?})",
            exec_rx.len(),
            outcome,
        );
        assert_eq!(
            outcome,
            PumpOutcome::Action,
            "the first pump iteration must have serviced the Action arm, not {:?}",
            outcome,
        );
        assert_eq!(
            exec_rx.len(),
            FLOOD,
            "the keypress iteration must not have touched the executor queue; {} of {} events were consumed",
            FLOOD - exec_rx.len(),
            FLOOD,
        );
    }

    /// Property 2 (D-17): a burst is drained in bounded batches, so control
    /// returns to the loop head instead of draining everything.
    #[tokio::test]
    async fn executor_burst_is_drained_in_bounded_batches() {
        // Keep `_act_tx` alive: dropping it closes the Action channel, which
        // would disable arm 1 rather than leave it pending, and the test would
        // then be proving something weaker than it claims.
        let (_act_tx, mut act_rx) = mpsc::unbounded_channel::<Action>();
        let (exec_tx, mut exec_rx) = mpsc::channel(EXEC_CHANNEL_CAPACITY * 2);

        for _ in 0..FLOOD {
            exec_tx
                .try_send(dummy_event("acme"))
                .expect("executor channel must hold the whole flood");
        }

        let mut app = App::new_for_test();
        let outcome = pump(&mut app, &mut act_rx, &mut exec_rx, EXEC_BATCH).await;

        assert_eq!(
            outcome,
            PumpOutcome::ExecEvents(EXEC_BATCH),
            "one pump iteration must apply exactly EXEC_BATCH ({}) events, got {:?}",
            EXEC_BATCH,
            outcome,
        );
        assert_eq!(
            exec_rx.len(),
            FLOOD - EXEC_BATCH,
            "one pump iteration must drain at most EXEC_BATCH ({}) events, but {} of {} were consumed",
            EXEC_BATCH,
            FLOOD - exec_rx.len(),
            FLOOD,
        );
    }

    /// Property 3 (supporting, TRANS-03): frames keep rendering while the
    /// executor channel is saturated.
    ///
    /// Timing-flavoured but robust: the bound is generous, so it fails only if
    /// redraws stop entirely — which is the actual failure mode the criterion
    /// names.
    ///
    /// Two details make the load real rather than nominal. The runtime is
    /// **multi-threaded**, so the flooding task genuinely races the render loop
    /// the way a reader task races it in production; on the default
    /// single-threaded runtime the consumer simply takes turns with the
    /// producer and the queue never backs up past one batch. And the channel is
    /// **pre-filled to capacity** before the first frame, so saturation is a
    /// precondition of the measurement rather than something the test hopes
    /// will emerge during it. `peak_queue` is asserted for exactly that reason:
    /// without it, a harness that quietly failed to apply any load would still
    /// report a comfortable frame count.
    #[tokio::test(flavor = "multi_thread", worker_threads = 2)]
    async fn tui_renders_repeatedly_while_the_executor_channel_is_saturated() {
        const MEASURE: Duration = Duration::from_secs(2);
        const MIN_FRAMES: usize = 20;

        let mut terminal =
            Terminal::new(TestBackend::new(120, 40)).expect("TestBackend terminal");
        let (_act_tx, mut act_rx) = mpsc::unbounded_channel::<Action>();
        let (exec_tx, mut exec_rx) = mpsc::channel::<ExecEvent>(EXEC_CHANNEL_CAPACITY);

        // Saturate before the first frame is drawn.
        while exec_tx.try_send(dummy_event("acme")).is_ok() {}
        assert_eq!(
            exec_rx.len(),
            EXEC_CHANNEL_CAPACITY,
            "precondition: the channel must start full",
        );

        // Then keep topping it up for longer than we measure, so it is still
        // saturated when the last frame is counted.
        let flooder = tokio::spawn(async move {
            let until = Instant::now() + MEASURE + Duration::from_millis(500);
            while Instant::now() < until {
                if exec_tx.send(dummy_event("acme")).await.is_err() {
                    break;
                }
            }
        });

        let mut app = App::new_for_test();
        let mut draw_count = 0usize;
        let mut peak_queue = 0usize;

        let until = Instant::now() + MEASURE;
        while Instant::now() < until {
            // The same redraw-flag sync and draw the real loop performs.
            if app.ctx.needs_redraw {
                app.needs_redraw = true;
                app.ctx.needs_redraw = false;
            }
            if app.needs_redraw {
                terminal
                    .draw(|frame| crate::ui::render(frame, &mut app))
                    .expect("draw under load");
                app.needs_redraw = false;
                draw_count += 1;
            }
            peak_queue = peak_queue.max(exec_rx.len());
            pump(&mut app, &mut act_rx, &mut exec_rx, EXEC_BATCH).await;
        }

        flooder.abort();

        assert!(
            peak_queue > EXEC_BATCH,
            "the channel was never actually saturated (peak queue depth {peak_queue}); \
             the test proved nothing about behaviour under load",
        );
        assert!(
            draw_count > MIN_FRAMES,
            "only {draw_count} frames rendered in {:?} under a saturated executor channel \
             (peak queue depth {peak_queue}); expected more than {MIN_FRAMES}",
            MEASURE,
        );
    }
}
