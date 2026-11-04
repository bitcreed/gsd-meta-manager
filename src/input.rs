//! The TUI's terminal input reader, and the one helper that hands the terminal
//! to an interactive child (quick 261103-tn4).
//!
//! Root cause this module exists for: crossterm's `EventStream` runs a
//! background std thread that loops a blocking poll and READS tty bytes, and it
//! is never joined (dropping the stream only flags the thread). An editor
//! launched from the TUI therefore raced that thread for every keystroke, and
//! each byte the TUI won was swallowed, so the editor saw about every other key.
//!
//! The reader here is app-owned instead: one std thread that polls crossterm
//! with a short timeout. The invariant is that once [`InputReader::suspend`]
//! returns, no thread in this process is inside a tty poll or read, and none
//! starts until [`InputReader::resume`]. Every in-TUI shell-out that gives the
//! terminal to a child goes through [`run_with_terminal_suspended`].

use crate::action::Action;
use crossterm::event::{Event, KeyEventKind};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;

/// How long one `poll` may block. It bounds how long [`InputReader::suspend`]
/// can wait for the thread to come back to its park point.
const POLL_TIMEOUT: Duration = Duration::from_millis(50);

/// Where the reader gets terminal events. A seam so a test can stand in for the
/// tty and count every poll/read.
pub trait InputSource: Send + 'static {
    /// Wait up to `timeout` for an event; `true` when one is ready to `read`.
    fn poll(&mut self, timeout: Duration) -> std::io::Result<bool>;
    /// Read the ready event.
    fn read(&mut self) -> std::io::Result<Event>;
}

/// The real tty, through crossterm's global reader.
pub struct CrosstermSource;

impl InputSource for CrosstermSource {
    fn poll(&mut self, timeout: Duration) -> std::io::Result<bool> {
        crossterm::event::poll(timeout)
    }

    fn read(&mut self) -> std::io::Result<Event> {
        crossterm::event::read()
    }
}

fn map_event_to_action(event: Event) -> Option<Action> {
    match event {
        Event::Key(key_event) => {
            // Only handle key press events, not release or repeat
            if key_event.kind == KeyEventKind::Press {
                Some(Action::RawKey(key_event))
            } else {
                None
            }
        }
        Event::Resize(_w, _h) => Some(Action::Resize),
        // Mouse capture turns on any-motion tracking, and `pump` drains this
        // FIFO on a biased-first arm: a continuous producer there starves every
        // other arm (see `main_loop::pump`'s doc). So only a left press and
        // vertical wheel steps are forwarded; everything else is dropped here
        // (quick 260926-dyf, T-dyf-01).
        Event::Mouse(m) => crate::ui::mouse::routed(&m).map(|_| Action::Mouse(m)),
        _ => None,
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Mode {
    Run,
    Suspend,
    Stop,
}

struct Control {
    mode: Mode,
    /// The thread is waiting at its park point, outside any poll/read.
    parked: bool,
    /// The thread has ended (normal return, error or panic).
    exited: bool,
}

type Shared = Arc<(Mutex<Control>, Condvar)>;

fn lock(m: &Mutex<Control>) -> MutexGuard<'_, Control> {
    m.lock().unwrap_or_else(PoisonError::into_inner)
}

/// Sets `exited` when the reader thread ends, however it ends, so `suspend`
/// never waits on a dead thread.
struct ExitGuard(Shared);

impl Drop for ExitGuard {
    fn drop(&mut self) {
        lock(&self.0 .0).exited = true;
        self.0 .1.notify_all();
    }
}

/// Handle to the reader thread.
pub struct InputReader {
    shared: Shared,
}

impl InputReader {
    /// Start the reader thread; mapped events are sent on `tx`.
    pub fn spawn<S: InputSource>(
        mut source: S,
        tx: UnboundedSender<Action>,
    ) -> std::io::Result<InputReader> {
        let shared: Shared = Arc::new((
            Mutex::new(Control {
                mode: Mode::Run,
                parked: false,
                exited: false,
            }),
            Condvar::new(),
        ));
        let thread_shared = Arc::clone(&shared);
        std::thread::Builder::new()
            .name("gsdmm-input".to_string())
            .spawn(move || {
                let _guard = ExitGuard(Arc::clone(&thread_shared));
                let (mutex, cvar) = &*thread_shared;
                loop {
                    {
                        let mut ctl = lock(mutex);
                        loop {
                            match ctl.mode {
                                Mode::Stop => return,
                                Mode::Suspend => {
                                    ctl.parked = true;
                                    cvar.notify_all();
                                    ctl = cvar.wait(ctl).unwrap_or_else(PoisonError::into_inner);
                                }
                                Mode::Run => {
                                    ctl.parked = false;
                                    break;
                                }
                            }
                        }
                    }
                    match source.poll(POLL_TIMEOUT) {
                        Ok(false) => {}
                        Ok(true) => match source.read() {
                            Ok(event) => {
                                if let Some(action) = map_event_to_action(event) {
                                    if tx.send(action).is_err() {
                                        return;
                                    }
                                }
                            }
                            Err(e) => {
                                tracing::warn!("input reader: read failed, stopping: {e}");
                                return;
                            }
                        },
                        Err(e) => {
                            tracing::warn!("input reader: poll failed, stopping: {e}");
                            return;
                        }
                    }
                }
            })?;
        Ok(InputReader { shared })
    }

    /// Park the reader and wait until it is outside any poll/read (or has
    /// exited). After this returns nothing in the process reads the tty until
    /// [`InputReader::resume`].
    pub fn suspend(&self) {
        let (mutex, cvar) = &*self.shared;
        let mut ctl = lock(mutex);
        ctl.mode = Mode::Suspend;
        cvar.notify_all();
        while !(ctl.parked || ctl.exited) {
            ctl = cvar.wait(ctl).unwrap_or_else(PoisonError::into_inner);
        }
    }

    /// Let the reader poll again. Does not wait, and does not drain what
    /// crossterm buffered.
    pub fn resume(&self) {
        let (mutex, cvar) = &*self.shared;
        lock(mutex).mode = Mode::Run;
        cvar.notify_all();
    }
}

impl Drop for InputReader {
    fn drop(&mut self) {
        let (mutex, cvar) = &*self.shared;
        lock(mutex).mode = Mode::Stop;
        cvar.notify_all();
    }
}

/// Hands the real terminal to a child and takes it back.
pub trait TerminalHandoff {
    /// Leave raw mode / the alternate screen (and mouse capture).
    fn release_terminal(&mut self);
    /// Re-enter them.
    fn reclaim_terminal(&mut self);
}

/// Run `child` with the terminal released and the input reader parked.
///
/// Every in-TUI shell-out that hands the terminal to a child must use this. The
/// reader is suspended BEFORE raw mode is left and resumed only AFTER raw mode is
/// re-entered. It blocks the calling thread for at most one poll interval plus
/// the child, the same way the editor's `status()` already blocks the runtime
/// thread.
pub fn run_with_terminal_suspended<R>(
    input: &InputReader,
    terminal: &mut impl TerminalHandoff,
    child: impl FnOnce() -> R,
) -> R {
    input.suspend();
    terminal.release_terminal();
    let result = child();
    terminal.reclaim_terminal();
    input.resume();
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
    use std::collections::VecDeque;
    use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
    use std::time::Instant;
    use tokio::sync::mpsc::{self, UnboundedReceiver};

    /// State shared between the test and the fake tty.
    #[derive(Default)]
    struct Fake {
        calls: AtomicUsize,
        released: AtomicBool,
        violations: AtomicUsize,
        queue: Mutex<VecDeque<Event>>,
        in_poll: AtomicBool,
        poll_delay_ms: AtomicU64,
        fail: AtomicBool,
    }

    impl Fake {
        fn new() -> Arc<Fake> {
            let f = Fake::default();
            f.poll_delay_ms.store(1, Ordering::SeqCst);
            Arc::new(f)
        }
        fn calls(&self) -> usize {
            self.calls.load(Ordering::SeqCst)
        }
        fn violations(&self) -> usize {
            self.violations.load(Ordering::SeqCst)
        }
        fn check_released(&self) {
            if self.released.load(Ordering::SeqCst) {
                self.violations.fetch_add(1, Ordering::SeqCst);
            }
        }
    }

    struct FakeSource(Arc<Fake>);

    impl InputSource for FakeSource {
        fn poll(&mut self, _timeout: Duration) -> std::io::Result<bool> {
            let f = &self.0;
            f.calls.fetch_add(1, Ordering::SeqCst);
            f.check_released();
            if f.fail.load(Ordering::SeqCst) {
                return Err(std::io::Error::other("fake tty gone"));
            }
            let ready = !f.queue.lock().unwrap().is_empty();
            if !ready {
                f.in_poll.store(true, Ordering::SeqCst);
                std::thread::sleep(Duration::from_millis(
                    f.poll_delay_ms.load(Ordering::SeqCst),
                ));
                f.in_poll.store(false, Ordering::SeqCst);
            }
            f.check_released();
            Ok(ready)
        }

        fn read(&mut self) -> std::io::Result<Event> {
            let f = &self.0;
            f.calls.fetch_add(1, Ordering::SeqCst);
            f.check_released();
            f.queue
                .lock()
                .unwrap()
                .pop_front()
                .ok_or_else(|| std::io::Error::other("read with empty queue"))
        }
    }

    struct FakeHandoff(Arc<Fake>);

    impl TerminalHandoff for FakeHandoff {
        fn release_terminal(&mut self) {
            self.0.released.store(true, Ordering::SeqCst);
            std::thread::sleep(Duration::from_millis(20));
        }
        fn reclaim_terminal(&mut self) {
            std::thread::sleep(Duration::from_millis(20));
            self.0.released.store(false, Ordering::SeqCst);
        }
    }

    fn start(fake: &Arc<Fake>) -> (InputReader, UnboundedReceiver<Action>) {
        let (tx, rx) = mpsc::unbounded_channel();
        let reader = InputReader::spawn(FakeSource(Arc::clone(fake)), tx).unwrap();
        (reader, rx)
    }

    fn wait_until(what: &str, mut cond: impl FnMut() -> bool) {
        let deadline = Instant::now() + Duration::from_secs(2);
        while !cond() {
            assert!(Instant::now() < deadline, "timed out waiting for {what}");
            std::thread::sleep(Duration::from_millis(2));
        }
    }

    fn recv_key(rx: &mut UnboundedReceiver<Action>) -> bool {
        let deadline = Instant::now() + Duration::from_secs(2);
        while Instant::now() < deadline {
            if let Ok(Action::RawKey(_)) = rx.try_recv() {
                return true;
            }
            std::thread::sleep(Duration::from_millis(2));
        }
        false
    }

    fn key_a() -> Event {
        Event::Key(KeyEvent::new(KeyCode::Char('a'), KeyModifiers::NONE))
    }

    const CHILD: Duration = Duration::from_millis(200);

    #[test]
    fn no_tty_read_while_a_shelled_out_child_owns_the_terminal() {
        let fake = Fake::new();
        let (reader, mut rx) = start(&fake);
        wait_until("first poll", || fake.calls() > 0);

        let mut handoff = FakeHandoff(Arc::clone(&fake));
        let (at_start, at_end, saw_released) =
            run_with_terminal_suspended(&reader, &mut handoff, || {
                let at_start = fake.calls();
                let saw_released = fake.released.load(Ordering::SeqCst);
                std::thread::sleep(CHILD);
                (at_start, fake.calls(), saw_released)
            });

        assert!(saw_released, "the child must see the terminal released");
        assert_eq!(
            at_start, at_end,
            "tty poll/read calls while the child owned the terminal"
        );
        assert_eq!(
            fake.violations(),
            0,
            "a poll/read overlapped the released-terminal window"
        );

        // Input comes back after the helper returns.
        let before = fake.calls();
        fake.queue.lock().unwrap().push_back(key_a());
        assert!(recv_key(&mut rx), "a key typed after resume must arrive");
        assert!(fake.calls() > before, "polling must resume");
    }

    #[test]
    fn the_harness_sees_tty_reads_when_a_shell_out_skips_the_reader() {
        // The old main.rs order: release -> child -> reclaim, reader untouched.
        let fake = Fake::new();
        let (_reader, _rx) = start(&fake);
        wait_until("first poll", || fake.calls() > 0);

        let mut handoff = FakeHandoff(Arc::clone(&fake));
        handoff.release_terminal();
        let at_start = fake.calls();
        std::thread::sleep(CHILD);
        let at_end = fake.calls();
        handoff.reclaim_terminal();

        assert!(
            at_end > at_start,
            "the harness must see tty reads from a reader nobody suspended"
        );
        assert!(fake.violations() > 0);
    }
}
