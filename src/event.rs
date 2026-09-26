use crossterm::event::{Event, EventStream, KeyEventKind};
use futures::StreamExt;
use gsd_meta_manager::action::Action;
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

    pub fn spawn_crossterm_reader(&self) {
        let tx = self.tx.clone();
        tokio::spawn(async move {
            let mut reader = EventStream::new();
            while let Some(Ok(event)) = reader.next().await {
                if let Some(action) = map_event_to_action(event) {
                    if tx.send(action).is_err() {
                        break;
                    }
                }
            }
        });
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
        Event::Mouse(m) => gsd_meta_manager::ui::mouse::routed(&m).map(|_| Action::Mouse(m)),
        _ => None,
    }
}
