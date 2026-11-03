//! The session lifecycle of a one-off run (quick 261103-gt1).
//!
//! In stream-json input mode the agent stays alive until its stdin reaches EOF, so the
//! launcher must close stdin to end the session the way a plain `-p` run ends: when
//! the agent finishes its turn with nothing left to read. [`SessionLifecycle`] is the
//! pure decision (C-06); [`Supervisor`] is one wait-loop tick of the daemon: follow
//! the agent's own `stream.jsonl`, take the inbox, write the messages, apply the
//! lifecycle.

use std::time::{Duration, Instant};

use super::stream::StreamEvent;

/// How long a `result` may wait for the replay of a message the launcher wrote before
/// stdin is closed anyway (C-06). A fixed fallback for an agent that never replays.
pub const REPLAY_SETTLE: Duration = Duration::from_secs(30);

/// What the lifecycle asks the daemon to do with the agent's stdin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleStep {
    Keep,
    CloseStdin,
    /// Close it, and record that a written message was never seen replayed.
    CloseStdinWithWarning,
}

/// Counts the messages written and replayed, and decides when the turn is over.
///
/// Replays are matched to writes by COUNT, in order, never by text: the capture pump
/// may redact the replayed text (I-2).
#[derive(Debug, Clone)]
pub struct SessionLifecycle {
    written: u32,
    replayed: u32,
    stdin_open: bool,
    /// When a `result` was last seen with no replay after it.
    result_since: Option<Instant>,
}

impl Default for SessionLifecycle {
    fn default() -> Self {
        Self::new()
    }
}

impl SessionLifecycle {
    pub fn new() -> Self {
        SessionLifecycle {
            written: 0,
            replayed: 0,
            stdin_open: true,
            result_since: None,
        }
    }

    /// A message was queued for the agent. A `result` that is still waiting restarts
    /// its settle clock: the agent has just been given something to read.
    pub fn wrote(&mut self, now: Instant) {
        self.written += 1;
        if self.result_since.is_some() {
            self.result_since = Some(now);
        }
    }

    /// Feed one classified stream event.
    pub fn saw(&mut self, event: &StreamEvent, now: Instant) {
        match event {
            StreamEvent::UserReplay { .. } => {
                self.replayed += 1;
                // A replay after a result is a new turn: wait for the next result.
                self.result_since = None;
            }
            StreamEvent::TurnResult { .. } => self.result_since = Some(now),
            _ => {}
        }
    }

    /// The step for `now`. Returning a close also marks the input closed.
    pub fn decide(&mut self, now: Instant) -> LifecycleStep {
        if !self.stdin_open {
            return LifecycleStep::Keep;
        }
        let Some(since) = self.result_since else {
            return LifecycleStep::Keep;
        };
        if self.replayed >= self.written {
            self.stdin_open = false;
            return LifecycleStep::CloseStdin;
        }
        if now.saturating_duration_since(since) >= REPLAY_SETTLE {
            self.stdin_open = false;
            return LifecycleStep::CloseStdinWithWarning;
        }
        LifecycleStep::Keep
    }

    /// The input was closed by someone else (a stop path).
    pub fn closed(&mut self) {
        self.stdin_open = false;
    }

    /// Whether a message may still be written.
    pub fn can_write(&self) -> bool {
        self.stdin_open
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub use self::supervisor::*;

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod supervisor {
    use std::path::{Path, PathBuf};
    use std::time::Instant;

    use chrono::Utc;

    use super::{LifecycleStep, SessionLifecycle};
    use crate::launcher::process::ChildInput;
    use crate::oneoff::inbox::{
        self, InboxRequest, InjectionKind, InjectionOutcome, InjectionRecord, Taken,
    };
    use crate::oneoff::runtime::AgentRuntime;
    use crate::oneoff::store::rfc3339;
    use crate::oneoff::stream::StreamTail;
    use crate::oneoff::{MAX_INJECTIONS, STREAM_FILE};

    /// The daemon's per-tick view of its own session.
    pub struct Supervisor {
        run_dir: PathBuf,
        runtime: AgentRuntime,
        tail: StreamTail,
        lifecycle: SessionLifecycle,
        input: ChildInput,
        injections: usize,
        warnings: Vec<String>,
    }

    impl Supervisor {
        pub fn new(run_dir: &Path, input: ChildInput) -> Self {
            Supervisor {
                run_dir: run_dir.to_owned(),
                runtime: AgentRuntime::default(),
                tail: StreamTail::new(&run_dir.join(STREAM_FILE)),
                lifecycle: SessionLifecycle::new(),
                input,
                injections: 0,
                warnings: Vec::new(),
            }
        }

        /// Write the initial prompt as the first message. It is not an injection and is
        /// not recorded in `injections.jsonl`.
        pub fn send_prompt(&mut self, prompt: &str, now: Instant) {
            let line = self.runtime.user_message_line(prompt);
            self.lifecycle.wrote(now);
            if self.input.send(line.into_bytes()).is_err() {
                self.warnings
                    .push("the agent's input closed before the prompt was written".to_owned());
            }
        }

        fn record(&self, record: InjectionRecord) {
            let _ = inbox::append_injection(&self.run_dir, &record);
        }

        fn refused(&self, id: &str, kind: InjectionKind, text: Option<String>, reason: &str) {
            self.record(InjectionRecord {
                at: rfc3339(Utc::now()),
                id: id.to_owned(),
                kind,
                text,
                usd: None,
                outcome: InjectionOutcome::Refused,
                reason: Some(reason.to_owned()),
            });
        }

        fn handle(&mut self, taken: Taken, now: Instant) {
            let Taken { id, request } = taken;
            match request {
                Err(reason) => self.refused(&id, InjectionKind::Message, None, &reason),
                Ok(InboxRequest::SoftCap { .. }) => {
                    self.refused(&id, InjectionKind::SoftCap, None, "not supported yet");
                }
                Ok(InboxRequest::Message { text }) => {
                    let reason = if !self.lifecycle.can_write() {
                        Some("the session is ending")
                    } else if self.injections >= MAX_INJECTIONS {
                        Some("too many injections")
                    } else if !self.input.is_open() {
                        Some("the agent's input is closed")
                    } else {
                        None
                    };
                    if let Some(reason) = reason {
                        self.refused(&id, InjectionKind::Message, Some(text), reason);
                        return;
                    }
                    // Recorded BEFORE the write (C-11).
                    self.record(InjectionRecord {
                        at: rfc3339(Utc::now()),
                        id: id.clone(),
                        kind: InjectionKind::Message,
                        text: Some(text.clone()),
                        usd: None,
                        outcome: InjectionOutcome::Delivered,
                        reason: None,
                    });
                    self.injections += 1;
                    self.lifecycle.wrote(now);
                    if self
                        .input
                        .send(self.runtime.user_message_line(&text).into_bytes())
                        .is_err()
                    {
                        self.warnings
                            .push(format!("message {id} was recorded but not written"));
                    }
                }
            }
        }

        /// One wait-loop tick: follow the stream, take the inbox, apply the lifecycle.
        pub fn tick(&mut self, now: Instant) {
            if let Ok(lines) = self.tail.poll() {
                for line in lines {
                    for event in self.runtime.classify(&line) {
                        self.lifecycle.saw(&event, now);
                    }
                }
            }
            for taken in inbox::take(&self.run_dir) {
                self.handle(taken, now);
            }
            match self.lifecycle.decide(now) {
                LifecycleStep::Keep => {}
                LifecycleStep::CloseStdin => self.input.close(),
                LifecycleStep::CloseStdinWithWarning => {
                    self.warnings.push(
                        "a message was written but never seen replayed; the agent's input was closed anyway"
                            .to_owned(),
                    );
                    self.input.close();
                }
            }
        }

        /// The input is closed (a stop path); nothing more is written.
        pub fn close_input(&mut self) {
            self.lifecycle.closed();
            self.input.close();
        }

        /// The session ended: refuse whatever is still in the inbox and hand back the
        /// warnings for `run.json`.
        pub fn finish(mut self) -> Vec<String> {
            self.close_input();
            for taken in inbox::take(&self.run_dir) {
                let kind = match taken.request {
                    Ok(InboxRequest::SoftCap { .. }) => InjectionKind::SoftCap,
                    _ => InjectionKind::Message,
                };
                self.refused(&taken.id, kind, None, "the session ended");
            }
            self.warnings
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn replay() -> StreamEvent {
        StreamEvent::UserReplay { text: "x".into() }
    }

    fn result() -> StreamEvent {
        StreamEvent::TurnResult {
            subtype: "success".into(),
            cost_usd: None,
            turns: None,
            is_error: false,
        }
    }

    #[test]
    fn the_prompts_turn_ends_the_session() {
        let t0 = Instant::now();
        let mut l = SessionLifecycle::new();
        l.wrote(t0);
        assert_eq!(l.decide(t0), LifecycleStep::Keep, "no result yet");
        l.saw(&replay(), t0);
        l.saw(&result(), t0);
        assert_eq!(l.decide(t0), LifecycleStep::CloseStdin);
        assert!(!l.can_write());
        assert_eq!(l.decide(t0), LifecycleStep::Keep, "closed once");
    }

    #[test]
    fn a_result_before_the_injections_replay_keeps_stdin_open() {
        let t0 = Instant::now();
        let mut l = SessionLifecycle::new();
        l.wrote(t0);
        l.wrote(t0);
        l.saw(&replay(), t0);
        l.saw(&result(), t0);
        assert_eq!(l.decide(t0), LifecycleStep::Keep);
        l.saw(&replay(), t0);
        l.saw(&result(), t0);
        assert_eq!(l.decide(t0), LifecycleStep::CloseStdin);
    }

    #[test]
    fn a_replay_that_never_comes_closes_after_the_settle_window() {
        let t0 = Instant::now();
        let mut l = SessionLifecycle::new();
        l.wrote(t0);
        l.saw(&result(), t0);
        assert_eq!(l.decide(t0), LifecycleStep::Keep);
        assert_eq!(l.decide(t0 + Duration::from_secs(29)), LifecycleStep::Keep);
        assert_eq!(
            l.decide(t0 + Duration::from_secs(30)),
            LifecycleStep::CloseStdinWithWarning
        );
        assert!(!l.can_write());
    }

    #[test]
    fn a_replay_after_a_result_restarts_the_wait() {
        let t0 = Instant::now();
        let mut l = SessionLifecycle::new();
        l.wrote(t0);
        l.wrote(t0);
        l.saw(&replay(), t0);
        l.saw(&result(), t0);
        l.saw(&replay(), t0);
        // The second turn has not ended: even past the settle window, keep.
        assert_eq!(l.decide(t0 + Duration::from_secs(120)), LifecycleStep::Keep);
        l.saw(&result(), t0 + Duration::from_secs(121));
        assert_eq!(
            l.decide(t0 + Duration::from_secs(121)),
            LifecycleStep::CloseStdin
        );
    }

    #[test]
    fn a_write_after_a_result_is_not_lost_to_the_close() {
        let t0 = Instant::now();
        let mut l = SessionLifecycle::new();
        l.wrote(t0);
        l.saw(&replay(), t0);
        l.saw(&result(), t0);
        l.wrote(t0);
        assert_eq!(l.decide(t0), LifecycleStep::Keep, "the new message is owed");
    }
}
