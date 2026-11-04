//! Terminal setup and teardown, including mouse capture (quick 260926-dyf).
//!
//! Mouse-reporting modes are global terminal state that outlives the process,
//! so every path that hands the terminal back — normal exit, the panic hook and
//! the editor handoff ([`Handoff`]) — turns capture OFF first (D-05, T-dyf-02).

use crossterm::event::{DisableMouseCapture, EnableMouseCapture};
use crossterm::execute;
use gsd_meta_manager::input::TerminalHandoff;
use ratatui::DefaultTerminal;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Once;

/// Whether mouse capture is currently ON in the terminal (the APPLIED state;
/// `App::mouse_capture` is the desired one).
static MOUSE_CAPTURED: AtomicBool = AtomicBool::new(false);

static PANIC_HOOK: Once = Once::new();

/// Enter the alternate screen and raw mode, and install the capture-off panic
/// hook once.
///
/// `ratatui::init` installs its own restore panic hook on EVERY call (the
/// editor handoff calls this again); the `Once` keeps this hook single. It is
/// installed after ratatui's, so it runs FIRST — capture goes off before
/// ratatui restores the terminal — and then chains to the previous hook
/// (ratatui's restore, then color_eyre's report).
pub fn init() -> DefaultTerminal {
    let terminal = ratatui::init();
    PANIC_HOOK.call_once(|| {
        let previous = std::panic::take_hook();
        std::panic::set_hook(Box::new(move |info| {
            if MOUSE_CAPTURED.swap(false, Ordering::SeqCst) {
                let _ = execute!(std::io::stdout(), DisableMouseCapture);
            }
            previous(info);
        }));
    });
    terminal
}

/// Leave the TUI: capture off (when on), then ratatui's restore.
pub fn restore() {
    if MOUSE_CAPTURED.swap(false, Ordering::SeqCst) {
        let _ = execute!(std::io::stdout(), DisableMouseCapture);
    }
    ratatui::restore();
}

/// Turn terminal mouse capture on or off; the flag is recorded only on
/// success.
pub fn set_mouse_capture(on: bool) -> std::io::Result<()> {
    if on {
        execute!(std::io::stdout(), EnableMouseCapture)?;
    } else {
        execute!(std::io::stdout(), DisableMouseCapture)?;
    }
    MOUSE_CAPTURED.store(on, Ordering::SeqCst);
    Ok(())
}

/// Whether capture is currently on in the terminal.
pub fn mouse_captured() -> bool {
    MOUSE_CAPTURED.load(Ordering::SeqCst)
}

/// The production [`TerminalHandoff`]: gives the terminal to an interactive
/// child (the editor) and takes it back.
pub struct Handoff<'a> {
    terminal: &'a mut DefaultTerminal,
    mouse_was_on: bool,
}

impl<'a> Handoff<'a> {
    pub fn new(terminal: &'a mut DefaultTerminal) -> Self {
        Self {
            terminal,
            mouse_was_on: false,
        }
    }
}

impl TerminalHandoff for Handoff<'_> {
    /// D-05: mouse capture goes off before the terminal is handed to the child
    /// (`restore` disables it), and comes back on reclaim only if it was on.
    fn release_terminal(&mut self) {
        self.mouse_was_on = mouse_captured();
        restore();
    }

    fn reclaim_terminal(&mut self) {
        *self.terminal = init();
        if self.mouse_was_on {
            // Errors are left to the loop's sync at the top, which retries
            // and reports.
            let _ = set_mouse_capture(true);
        }
    }
}
