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

/// How long the session may stay alive after the soft cap's wrap-up message (C-18).
pub const SOFT_CAP_GRACE: Duration = Duration::from_secs(300);

/// The environment variable that shortens the grace (tests only; 1..=600 seconds). It
/// is read by the host daemon at session start; the sandboxed agent cannot set it.
pub const SOFT_CAP_GRACE_ENV: &str = "GSDMM_ONEOFF_SOFT_CAP_GRACE_SECS";

/// The grace in force: [`SOFT_CAP_GRACE_ENV`] when it is an integer 1..=600, else
/// [`SOFT_CAP_GRACE`].
pub fn grace_from_env() -> Duration {
    grace_from(std::env::var(SOFT_CAP_GRACE_ENV).ok().as_deref())
}

/// [`grace_from_env`] with the value given.
pub fn grace_from(value: Option<&str>) -> Duration {
    super::cancel::grace_with_default(value, SOFT_CAP_GRACE)
}

/// Judge a soft cap against the hard ceiling: `None` clears; a value must be finite,
/// above 0 and below `hard_usd`.
pub fn validate_soft_cap(usd: Option<f64>, hard_usd: f64) -> Result<(), String> {
    match usd {
        None => Ok(()),
        Some(v) if v.is_finite() && v > 0.0 && v < hard_usd => Ok(()),
        Some(_) => Err(format!(
            "the soft cap must be above $0 and below the hard ceiling ${hard_usd:.2}"
        )),
    }
}

/// What the soft cap asks of the daemon this tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SoftCapStep {
    None,
    /// The estimate just reached the cap: send the one wrap-up message.
    WrapUp,
    /// In the grace.
    Wait,
    /// The grace is over and the session is still alive: stop it.
    Stop,
}

/// The soft spend cap: off by default, set and cleared live (C-17, C-18, I-11).
#[derive(Debug, Clone)]
pub struct SoftCap {
    cap: Option<f64>,
    hard: f64,
    wrapped_up: bool,
    hit: bool,
    deadline: Option<Instant>,
    grace: Duration,
}

impl SoftCap {
    pub fn new(hard: f64, grace: Duration) -> Self {
        SoftCap {
            cap: None,
            hard,
            wrapped_up: false,
            hit: false,
            deadline: None,
            grace,
        }
    }

    /// The cap in force.
    pub fn cap(&self) -> Option<f64> {
        self.cap
    }

    /// Whether the estimate ever reached the cap.
    pub fn hit(&self) -> bool {
        self.hit
    }

    /// Set (`Some`) or clear (`None`) the cap. Raising or clearing it above the
    /// estimate cancels a pending stop; the wrap-up message that was sent stays sent.
    pub fn set(&mut self, usd: Option<f64>, estimate: f64) -> Result<(), String> {
        validate_soft_cap(usd, self.hard)?;
        self.cap = usd;
        if self.deadline.is_some() && usd.is_none_or(|cap| estimate < cap) {
            self.deadline = None;
        }
        Ok(())
    }

    /// One tick at `estimate` and `now`.
    pub fn tick(&mut self, estimate: f64, now: Instant) -> SoftCapStep {
        let Some(cap) = self.cap else {
            return SoftCapStep::None;
        };
        if estimate < cap {
            return SoftCapStep::None;
        }
        self.hit = true;
        match self.deadline {
            None => {
                self.deadline = Some(now + self.grace);
                if self.wrapped_up {
                    SoftCapStep::Wait
                } else {
                    self.wrapped_up = true;
                    SoftCapStep::WrapUp
                }
            }
            Some(deadline) if now >= deadline => SoftCapStep::Stop,
            Some(_) => SoftCapStep::Wait,
        }
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub use self::supervisor::*;

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod supervisor {
    use std::path::{Path, PathBuf};
    use std::time::{Duration, Instant};

    use chrono::Utc;

    use super::{LifecycleStep, SessionLifecycle, SoftCap, SoftCapStep};
    use crate::launcher::process::ChildInput;
    use crate::oneoff::inbox::{
        self, InboxRequest, InjectionKind, InjectionOutcome, InjectionRecord, Taken,
    };
    use crate::oneoff::runtime::AgentRuntime;
    use crate::oneoff::store::rfc3339;
    use crate::oneoff::stream::{SpendEstimate, StreamTail};
    use crate::oneoff::{MAX_INJECTIONS, STREAM_FILE};

    /// What one tick asks of the wait loop.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
    pub struct TickOutcome {
        /// Stop the session: the soft cap's grace ran out.
        pub stop_soft_cap: bool,
        /// The soft cap or its hit time changed: write `run.json`.
        pub persist: bool,
    }

    /// What the supervisor hands back when the session ends.
    #[derive(Debug, Clone, PartialEq)]
    pub struct SessionEnd {
        pub warnings: Vec<String>,
        pub soft_cap_usd: Option<f64>,
        pub soft_cap_hit_at: Option<String>,
        /// The session's spend estimate as the supervisor last saw it (quick
        /// 261103-hx7, I-17).
        pub spend_usd: f64,
    }

    /// The daemon's per-tick view of its own session.
    pub struct Supervisor {
        run_dir: PathBuf,
        runtime: AgentRuntime,
        tail: StreamTail,
        lifecycle: SessionLifecycle,
        input: ChildInput,
        injections: usize,
        warnings: Vec<String>,
        estimate: SpendEstimate,
        soft_cap: SoftCap,
        soft_cap_hit_at: Option<String>,
    }

    impl Supervisor {
        pub fn new(run_dir: &Path, input: ChildInput, hard_usd: f64, grace: Duration) -> Self {
            Supervisor {
                run_dir: run_dir.to_owned(),
                runtime: AgentRuntime::default(),
                tail: StreamTail::new(&run_dir.join(STREAM_FILE)),
                lifecycle: SessionLifecycle::new(),
                input,
                injections: 0,
                warnings: Vec::new(),
                estimate: SpendEstimate::default(),
                soft_cap: SoftCap::new(hard_usd, grace),
                soft_cap_hit_at: None,
            }
        }

        /// The soft cap in force.
        pub fn soft_cap_usd(&self) -> Option<f64> {
            self.soft_cap.cap()
        }

        /// When the estimate first reached the soft cap.
        pub fn soft_cap_hit_at(&self) -> Option<&str> {
            self.soft_cap_hit_at.as_deref()
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

        fn record_of(
            id: &str,
            kind: InjectionKind,
            text: Option<String>,
            usd: Option<f64>,
            outcome: InjectionOutcome,
            reason: Option<&str>,
        ) -> InjectionRecord {
            InjectionRecord {
                at: rfc3339(Utc::now()),
                id: id.to_owned(),
                kind,
                text,
                usd,
                outcome,
                reason: reason.map(str::to_owned),
            }
        }

        fn refused(&self, id: &str, kind: InjectionKind, text: Option<String>, reason: &str) {
            self.record(Self::record_of(
                id,
                kind,
                text,
                None,
                InjectionOutcome::Refused,
                Some(reason),
            ));
        }

        /// Write `text` to the agent as a user message, recorded BEFORE the write.
        /// `count` says whether it counts against the injection limit.
        fn deliver(&mut self, id: &str, kind: InjectionKind, text: &str, now: Instant) -> bool {
            self.record(Self::record_of(
                id,
                kind,
                Some(text.to_owned()),
                None,
                InjectionOutcome::Delivered,
                None,
            ));
            self.lifecycle.wrote(now);
            let sent = self
                .input
                .send(self.runtime.user_message_line(text).into_bytes())
                .is_ok();
            if !sent {
                self.warnings
                    .push(format!("message {id} was recorded but not written"));
            }
            sent
        }

        /// Deliver a host-made message (the pause message): recorded BEFORE the write,
        /// not counted against [`MAX_INJECTIONS`], refused and recorded when the
        /// session is ending or its input is closed (quick 261103-klf, S-01).
        pub fn inject_host(&mut self, kind: InjectionKind, text: &str, now: Instant) -> bool {
            let id = format!("pause-{}", Utc::now().timestamp_millis());
            if self.lifecycle.can_write() && self.input.is_open() {
                self.deliver(&id, kind, text, now)
            } else {
                self.refused(&id, kind, Some(text.to_owned()), "the session is ending");
                false
            }
        }

        fn handle(&mut self, taken: Taken, now: Instant, outcome: &mut TickOutcome) {
            let Taken { id, request } = taken;
            match request {
                Err(reason) => self.refused(&id, InjectionKind::Message, None, &reason),
                Ok(InboxRequest::SoftCap { usd }) => {
                    match self.soft_cap.set(usd, self.estimate.usd()) {
                        Ok(()) => {
                            self.record(Self::record_of(
                                &id,
                                InjectionKind::SoftCap,
                                None,
                                usd,
                                InjectionOutcome::Delivered,
                                None,
                            ));
                            outcome.persist = true;
                        }
                        Err(reason) => self.refused(&id, InjectionKind::SoftCap, None, &reason),
                    }
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
                    self.injections += 1;
                    self.deliver(&id, InjectionKind::Message, &text, now);
                }
            }
        }

        /// One wait-loop tick: follow the stream, take the inbox, apply the soft cap
        /// and the lifecycle.
        pub fn tick(&mut self, now: Instant) -> TickOutcome {
            let mut outcome = TickOutcome::default();
            if let Ok(lines) = self.tail.poll() {
                for line in lines {
                    for event in self.runtime.classify(&line) {
                        self.estimate.apply(&event);
                        self.lifecycle.saw(&event, now);
                    }
                }
            }
            for taken in inbox::take(&self.run_dir) {
                self.handle(taken, now, &mut outcome);
            }
            match self.soft_cap.tick(self.estimate.usd(), now) {
                SoftCapStep::None | SoftCapStep::Wait => {}
                SoftCapStep::WrapUp => {
                    if self.soft_cap_hit_at.is_none() {
                        self.soft_cap_hit_at = Some(rfc3339(Utc::now()));
                        outcome.persist = true;
                    }
                    let id = format!("wrap-up-{}", Utc::now().timestamp_millis());
                    let message = self.runtime.wrap_up_message();
                    if self.lifecycle.can_write() && self.input.is_open() {
                        self.deliver(&id, InjectionKind::WrapUp, message, now);
                    } else {
                        self.refused(
                            &id,
                            InjectionKind::WrapUp,
                            Some(message.to_owned()),
                            "the session is ending",
                        );
                    }
                }
                SoftCapStep::Stop => outcome.stop_soft_cap = true,
            }
            if self.soft_cap.hit() && self.soft_cap_hit_at.is_none() {
                self.soft_cap_hit_at = Some(rfc3339(Utc::now()));
                outcome.persist = true;
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
            outcome
        }

        /// The input is closed (a stop path); nothing more is written.
        pub fn close_input(&mut self) {
            self.lifecycle.closed();
            self.input.close();
        }

        /// The session ended: refuse whatever is still in the inbox and hand back the
        /// warnings and the soft-cap facts for `run.json`.
        pub fn finish(mut self) -> SessionEnd {
            self.close_input();
            for taken in inbox::take(&self.run_dir) {
                let kind = match taken.request {
                    Ok(InboxRequest::SoftCap { .. }) => InjectionKind::SoftCap,
                    _ => InjectionKind::Message,
                };
                self.refused(&taken.id, kind, None, "the session ended");
            }
            SessionEnd {
                warnings: self.warnings,
                soft_cap_usd: self.soft_cap.cap(),
                soft_cap_hit_at: self.soft_cap_hit_at,
                spend_usd: self.estimate.usd(),
            }
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

    #[test]
    fn a_soft_cap_is_judged_against_the_hard_ceiling() {
        let g = Duration::from_secs(300);
        let mut cap = SoftCap::new(10.0, g);
        for bad in [0.0, -1.0, f64::NAN, f64::INFINITY, 10.0, 11.0] {
            assert!(cap.set(Some(bad), 0.0).is_err(), "{bad}");
        }
        assert!(cap.set(Some(9.99), 0.0).is_ok());
        assert_eq!(cap.cap(), Some(9.99));
        assert!(cap.set(None, 0.0).is_ok());
        assert_eq!(cap.cap(), None);
    }

    #[test]
    fn crossing_the_cap_wraps_up_once_then_waits_then_stops() {
        let t0 = Instant::now();
        let g = Duration::from_secs(300);
        let mut cap = SoftCap::new(10.0, g);
        assert_eq!(cap.tick(2.0, t0), SoftCapStep::None, "off by default");
        cap.set(Some(1.0), 2.0).unwrap();
        assert_eq!(cap.tick(2.0, t0), SoftCapStep::WrapUp);
        assert!(cap.hit());
        assert_eq!(
            cap.tick(2.0, t0 + Duration::from_secs(299)),
            SoftCapStep::Wait
        );
        assert_eq!(cap.tick(2.0, t0 + g), SoftCapStep::Stop);
    }

    #[test]
    fn raising_or_clearing_the_cap_cancels_the_stop_and_never_wraps_up_twice() {
        let t0 = Instant::now();
        let g = Duration::from_secs(300);
        let mut cap = SoftCap::new(10.0, g);
        cap.set(Some(1.0), 2.0).unwrap();
        assert_eq!(cap.tick(2.0, t0), SoftCapStep::WrapUp);
        cap.set(Some(5.0), 2.0).unwrap();
        assert_eq!(cap.tick(2.0, t0 + g), SoftCapStep::None, "cancelled");
        // Crossing the raised cap restarts the grace, with no second message.
        let t1 = t0 + Duration::from_secs(400);
        assert_eq!(cap.tick(6.0, t1), SoftCapStep::Wait);
        assert_eq!(cap.tick(6.0, t1 + g), SoftCapStep::Stop);
        cap.set(None, 6.0).unwrap();
        assert_eq!(cap.tick(6.0, t1 + g), SoftCapStep::None, "cleared");
    }

    #[test]
    fn the_grace_override_is_bounded() {
        assert_eq!(grace_from(Some("1")), Duration::from_secs(1));
        assert_eq!(grace_from(Some(" 600 ")), Duration::from_secs(600));
        for bad in [None, Some("0"), Some("601"), Some("x"), Some("")] {
            assert_eq!(grace_from(bad), SOFT_CAP_GRACE, "{bad:?}");
        }
    }
}
