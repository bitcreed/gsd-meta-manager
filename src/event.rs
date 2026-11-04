use gsd_meta_manager::action::Action;
use gsd_meta_manager::input::{CrosstermSource, InputReader};
use std::time::Duration;
use tokio::sync::mpsc;

pub struct EventBus {
    pub rx: mpsc::UnboundedReceiver<Action>,
    pub tx: mpsc::UnboundedSender<Action>,
}

impl EventBus {
    pub fn new() -> Self {
        let (tx, rx) = mpsc::unbounded_channel();
        Self { tx, rx }
    }

    /// Start the app-owned tty reader (see `gsd_meta_manager::input`). The
    /// returned handle suspends it around interactive shell-outs.
    pub fn spawn_input_reader(&self) -> std::io::Result<InputReader> {
        InputReader::spawn(CrosstermSource, self.tx.clone())
    }

    pub fn spawn_tick(&self, interval_ms: u64) {
        let tx = self.tx.clone();
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(Duration::from_millis(interval_ms));
            loop {
                interval.tick().await;
                if tx.send(Action::Tick).is_err() {
                    break;
                }
            }
        });
    }
}
