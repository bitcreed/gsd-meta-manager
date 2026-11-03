//! The session lifecycle of a one-off run (quick 261103-gt1, 261103-mnr).
//!
//! In stream-json input mode the agent stays alive until its stdin reaches EOF, so the
//! launcher must close stdin to end the session. It must not do so while the agent
//! still has work: a `result` ends a TURN, not the session, because the agent may have
//! background subagents running that start a new turn on each of their notifications.
//! [`SessionLifecycle`] is the pure decision (C-06, D-01); [`Supervisor`] is one
//! wait-loop tick of the daemon: follow the agent's own `stream.jsonl`, take the inbox,
//! write the messages, apply the lifecycle.

use std::collections::BTreeSet;
use std::time::{Duration, Instant};

use super::stream::StreamEvent;

/// How long a `result` may wait for the replay of a message the launcher wrote before
/// stdin is closed anyway (C-06). A fixed fallback for an agent that never replays.
pub const REPLAY_SETTLE: Duration = Duration::from_secs(30);

/// How long the stream may stay quiet after a turn ended with nothing outstanding and
/// no report written before stdin is closed anyway (D-01). It leaves the operator room
/// to inject a message and covers an agent that forgets to report.
pub const IDLE_SETTLE: Duration = Duration::from_secs(120);

/// The environment variable that shortens the idle settle (tests only; 1..=600
/// seconds). It is read by the host daemon at session start; the sandboxed agent cannot
/// set it.
pub const IDLE_SETTLE_ENV: &str = "GSDMM_ONEOFF_IDLE_SETTLE_SECS";

/// The idle settle in force: [`IDLE_SETTLE_ENV`] when it is an integer 1..=600, else
/// [`IDLE_SETTLE`].
pub fn idle_settle_from_env() -> Duration {
    idle_settle_from(std::env::var(IDLE_SETTLE_ENV).ok().as_deref())
}

/// [`idle_settle_from_env`] with the value given.
pub fn idle_settle_from(value: Option<&str>) -> Duration {
    super::cancel::grace_with_default(value, IDLE_SETTLE)
}

/// The most started-not-ended task ids the lifecycle remembers (T-mnr-01).
const MAX_TRACKED_TASKS: usize = 1024;

/// What the lifecycle asks the daemon to do with the agent's stdin.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LifecycleStep {
    Keep,
    CloseStdin,
    /// Close it, and record that a written message was never seen replayed.
    CloseStdinWithWarning,
    /// Close it, and record that the agent went quiet without writing its report.
    CloseStdinIdle,
}

/// Counts the messages written and replayed, tracks the agent's outstanding work and
/// decides when the session is over.
///
/// The session ends when a turn has ended since the last turn start, every written
/// message was replayed, no background task is outstanding, no turn is queued, and
/// either the agent's report exists (close at once) or the stream has been idle for the
/// idle settle (close with a warning).
///
/// Replays are matched to writes by COUNT, in order, never by text: the capture pump
/// may redact the replayed text (I-2).
#[derive(Debug, Clone)]
pub struct SessionLifecycle {
    written: u32,
    replayed: u32,
    stdin_open: bool,
    /// When a `result` was last seen with no turn start after it.
    result_since: Option<Instant>,
    idle_settle: Duration,
    /// When the last stream event was seen.
    last_event: Option<Instant>,
    /// How many `init` events were seen (every one after the first starts a turn).
    inits: u32,
    /// The size of the latest background-task set the agent reported.
    background: Option<u32>,
    /// Task ids started and not ended.
    tasks: BTreeSet<String>,
    /// The latest result's queued turns and still-running subagents.
    queued_turns: u32,
    result_outstanding: u32,
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
            idle_settle: IDLE_SETTLE,
            last_event: None,
            inits: 0,
            background: None,
            tasks: BTreeSet::new(),
            queued_turns: 0,
            result_outstanding: 0,
        }
    }

    /// The same lifecycle with another idle settle.
    pub fn with_idle_settle(mut self, idle_settle: Duration) -> Self {
        self.idle_settle = idle_settle;
        self
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
        self.last_event = Some(now);
        match event {
            StreamEvent::UserReplay { .. } => {
                self.replayed += 1;
                // A replay after a result is a new turn: wait for the next result.
                self.result_since = None;
            }
            StreamEvent::System { subtype } if subtype == "init" => {
                self.inits += 1;
                // The agent starts a new turn on each task notification.
                if self.inits > 1 {
                    self.result_since = None;
                }
            }
            StreamEvent::TurnResult {
                queued_turns,
                subagents_outstanding,
                ..
            } => {
                self.result_since = Some(now);
                self.queued_turns = *queued_turns;
                self.result_outstanding = *subagents_outstanding;
            }
            StreamEvent::BackgroundTasks { count } => self.background = Some(*count),
            StreamEvent::TaskStarted { task_id } => {
                if self.tasks.len() < MAX_TRACKED_TASKS {
                    self.tasks.insert(task_id.clone());
                }
            }
            StreamEvent::TaskEnded { task_id } => {
                self.tasks.remove(task_id);
            }
            _ => {}
        }
    }

    /// The background work the agent still has: the latest background-task set when
    /// one was ever reported (else the started-not-ended task ids), or what the
    /// latest result's statistics say, whichever is more.
    pub fn outstanding(&self) -> u32 {
        let tracked = self
            .background
            .unwrap_or_else(|| u32::try_from(self.tasks.len()).unwrap_or(u32::MAX));
        tracked.max(self.result_outstanding)
    }

    /// The step for `now`. `report_present` says whether the agent's report exists.
    /// Returning a close also marks the input closed.
    pub fn decide(&mut self, now: Instant, report_present: bool) -> LifecycleStep {
        if !self.stdin_open {
            return LifecycleStep::Keep;
        }
        let Some(since) = self.result_since else {
            return LifecycleStep::Keep;
        };
        if self.outstanding() > 0 || self.queued_turns > 0 {
            return LifecycleStep::Keep;
        }
        if self.replayed < self.written {
            if now.saturating_duration_since(since) >= REPLAY_SETTLE {
                self.stdin_open = false;
                return LifecycleStep::CloseStdinWithWarning;
            }
            return LifecycleStep::Keep;
        }
        if report_present {
            self.stdin_open = false;
            return LifecycleStep::CloseStdin;
        }
        let quiet_since = self.last_event.map_or(since, |last| last.max(since));
        if now.saturating_duration_since(quiet_since) >= self.idle_settle {
            self.stdin_open = false;
            return LifecycleStep::CloseStdinIdle;
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
        /// The agent's report file: its presence says the agent is done (D-01).
        report_file: PathBuf,
        idle_settle: Duration,
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
        pub fn new(
            run_dir: &Path,
            report_file: &Path,
            input: ChildInput,
            hard_usd: f64,
            grace: Duration,
            idle_settle: Duration,
        ) -> Self {
            Supervisor {
                run_dir: run_dir.to_owned(),
                report_file: report_file.to_owned(),
                idle_settle,
                runtime: AgentRuntime::default(),
                tail: StreamTail::new(&run_dir.join(STREAM_FILE)),
                lifecycle: SessionLifecycle::new().with_idle_settle(idle_settle),
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
            // Presence only, no symlink follow: the report is re-validated at finalize.
            let report_present = self.report_file.symlink_metadata().is_ok();
            match self.lifecycle.decide(now, report_present) {
                LifecycleStep::Keep => {}
                LifecycleStep::CloseStdin => self.input.close(),
                LifecycleStep::CloseStdinIdle => {
                    self.warnings.push(format!(
                        "closed the agent's input after {}s idle with no report",
                        self.idle_settle.as_secs()
                    ));
                    self.input.close();
                }
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
            queued_turns: 0,
            subagents_outstanding: 0,
        }
    }

    #[test]
    fn the_prompts_turn_ends_the_session() {
        let t0 = Instant::now();
        let mut l = SessionLifecycle::new();
        l.wrote(t0);
        assert_eq!(l.decide(t0, true), LifecycleStep::Keep, "no result yet");
        l.saw(&replay(), t0);
        l.saw(&result(), t0);
        assert_eq!(l.decide(t0, true), LifecycleStep::CloseStdin);
        assert!(!l.can_write());
        assert_eq!(l.decide(t0, true), LifecycleStep::Keep, "closed once");
    }

    #[test]
    fn a_result_before_the_injections_replay_keeps_stdin_open() {
        let t0 = Instant::now();
        let mut l = SessionLifecycle::new();
        l.wrote(t0);
        l.wrote(t0);
        l.saw(&replay(), t0);
        l.saw(&result(), t0);
        assert_eq!(l.decide(t0, true), LifecycleStep::Keep);
        l.saw(&replay(), t0);
        l.saw(&result(), t0);
        assert_eq!(l.decide(t0, true), LifecycleStep::CloseStdin);
    }

    #[test]
    fn a_replay_that_never_comes_closes_after_the_settle_window() {
        let t0 = Instant::now();
        let mut l = SessionLifecycle::new();
        l.wrote(t0);
        l.saw(&result(), t0);
        assert_eq!(l.decide(t0, true), LifecycleStep::Keep);
        assert_eq!(
            l.decide(t0 + Duration::from_secs(29), true),
            LifecycleStep::Keep
        );
        assert_eq!(
            l.decide(t0 + Duration::from_secs(30), true),
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
        assert_eq!(
            l.decide(t0 + Duration::from_secs(120), true),
            LifecycleStep::Keep
        );
        l.saw(&result(), t0 + Duration::from_secs(121));
        assert_eq!(
            l.decide(t0 + Duration::from_secs(121), true),
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
        assert_eq!(
            l.decide(t0, true),
            LifecycleStep::Keep,
            "the new message is owed"
        );
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

    fn init() -> StreamEvent {
        StreamEvent::System {
            subtype: "init".into(),
        }
    }

    fn result_with(queued_turns: u32, subagents_outstanding: u32) -> StreamEvent {
        StreamEvent::TurnResult {
            subtype: "success".into(),
            cost_usd: None,
            turns: None,
            is_error: false,
            queued_turns,
            subagents_outstanding,
        }
    }

    #[test]
    fn background_work_keeps_stdin_open_past_a_result() {
        let t0 = Instant::now();
        let mut l = SessionLifecycle::new();
        l.wrote(t0);
        l.saw(&init(), t0);
        l.saw(&replay(), t0);
        l.saw(&StreamEvent::BackgroundTasks { count: 2 }, t0);
        l.saw(&result(), t0);
        assert_eq!(l.outstanding(), 2);
        // Even with a report present, two tasks outstanding is not the end.
        assert_eq!(l.decide(t0, true), LifecycleStep::Keep);
        assert_eq!(
            l.decide(t0 + Duration::from_secs(600), true),
            LifecycleStep::Keep
        );
        assert!(l.can_write());
        // The set drains; the agent starts a turn on the notification and ends it.
        l.saw(&StreamEvent::BackgroundTasks { count: 0 }, t0);
        l.saw(&init(), t0);
        assert_eq!(l.decide(t0, true), LifecycleStep::Keep, "a turn is running");
        l.saw(&result(), t0);
        assert_eq!(l.decide(t0, true), LifecycleStep::CloseStdin);
    }

    #[test]
    fn task_events_count_when_no_background_event_was_seen() {
        let t0 = Instant::now();
        let mut l = SessionLifecycle::new();
        l.wrote(t0);
        l.saw(&replay(), t0);
        for id in ["t1", "t2"] {
            l.saw(&StreamEvent::TaskStarted { task_id: id.into() }, t0);
        }
        l.saw(&result(), t0);
        assert_eq!(l.outstanding(), 2);
        assert_eq!(l.decide(t0, true), LifecycleStep::Keep);
        l.saw(
            &StreamEvent::TaskEnded {
                task_id: "t1".into(),
            },
            t0,
        );
        assert_eq!(l.outstanding(), 1);
        l.saw(
            &StreamEvent::TaskEnded {
                task_id: "t2".into(),
            },
            t0,
        );
        assert_eq!(l.outstanding(), 0);
        assert_eq!(l.decide(t0, true), LifecycleStep::CloseStdin);
    }

    #[test]
    fn the_task_id_set_is_bounded() {
        let t0 = Instant::now();
        let mut l = SessionLifecycle::new();
        for n in 0..(MAX_TRACKED_TASKS + 50) {
            l.saw(
                &StreamEvent::TaskStarted {
                    task_id: format!("t{n}"),
                },
                t0,
            );
        }
        assert_eq!(l.outstanding() as usize, MAX_TRACKED_TASKS);
    }

    #[test]
    fn subagent_stats_and_queued_turns_hold_the_session_open() {
        let t0 = Instant::now();
        let mut l = SessionLifecycle::new();
        l.wrote(t0);
        l.saw(&replay(), t0);
        l.saw(&result_with(0, 3), t0);
        assert_eq!(l.outstanding(), 3);
        assert_eq!(l.decide(t0, true), LifecycleStep::Keep, "subagents running");
        l.saw(&result_with(1, 0), t0);
        assert_eq!(l.decide(t0, true), LifecycleStep::Keep, "a turn is queued");
        l.saw(&result_with(0, 0), t0);
        assert_eq!(l.decide(t0, true), LifecycleStep::CloseStdin);
    }

    #[test]
    fn a_later_init_starts_a_new_turn() {
        let t0 = Instant::now();
        let mut l = SessionLifecycle::new();
        l.wrote(t0);
        l.saw(&init(), t0);
        l.saw(&replay(), t0);
        l.saw(&result(), t0);
        // A second init (a task notification) restarts the wait for a result.
        l.saw(&init(), t0);
        assert_eq!(
            l.decide(t0 + Duration::from_secs(500), true),
            LifecycleStep::Keep
        );
        l.saw(&result(), t0 + Duration::from_secs(501));
        assert_eq!(
            l.decide(t0 + Duration::from_secs(501), true),
            LifecycleStep::CloseStdin
        );
    }

    #[test]
    fn the_first_init_alone_does_not_start_a_new_turn() {
        let t0 = Instant::now();
        let mut l = SessionLifecycle::new();
        l.wrote(t0);
        l.saw(&replay(), t0);
        l.saw(&result(), t0);
        l.saw(&init(), t0);
        // That was the first init: the result stands.
        assert_eq!(l.decide(t0, true), LifecycleStep::CloseStdin);
    }

    #[test]
    fn without_a_report_the_session_closes_after_the_idle_settle() {
        let t0 = Instant::now();
        let mut l = SessionLifecycle::new();
        l.wrote(t0);
        l.saw(&replay(), t0);
        l.saw(&result(), t0);
        assert_eq!(l.decide(t0, false), LifecycleStep::Keep);
        assert_eq!(
            l.decide(t0 + Duration::from_secs(119), false),
            LifecycleStep::Keep
        );
        assert_eq!(
            l.decide(t0 + Duration::from_secs(120), false),
            LifecycleStep::CloseStdinIdle
        );
        assert!(!l.can_write());

        // An event at 60 s moves the idle clock.
        let mut l = SessionLifecycle::new();
        l.wrote(t0);
        l.saw(&replay(), t0);
        l.saw(&result(), t0);
        l.saw(&StreamEvent::Other, t0 + Duration::from_secs(60));
        assert_eq!(
            l.decide(t0 + Duration::from_secs(120), false),
            LifecycleStep::Keep
        );
        assert_eq!(
            l.decide(t0 + Duration::from_secs(179), false),
            LifecycleStep::Keep
        );
        assert_eq!(
            l.decide(t0 + Duration::from_secs(180), false),
            LifecycleStep::CloseStdinIdle
        );

        // A report that appears closes at once.
        let mut l = SessionLifecycle::new();
        l.wrote(t0);
        l.saw(&replay(), t0);
        l.saw(&result(), t0);
        assert_eq!(
            l.decide(t0 + Duration::from_secs(5), false),
            LifecycleStep::Keep
        );
        assert_eq!(
            l.decide(t0 + Duration::from_secs(6), true),
            LifecycleStep::CloseStdin
        );
    }

    #[test]
    fn a_shorter_idle_settle_is_honoured() {
        let t0 = Instant::now();
        let mut l = SessionLifecycle::new().with_idle_settle(Duration::from_secs(1));
        l.wrote(t0);
        l.saw(&replay(), t0);
        l.saw(&result(), t0);
        assert_eq!(l.decide(t0, false), LifecycleStep::Keep);
        assert_eq!(
            l.decide(t0 + Duration::from_secs(1), false),
            LifecycleStep::CloseStdinIdle
        );
    }

    #[test]
    fn the_replay_settle_waits_for_background_work() {
        let t0 = Instant::now();
        let mut l = SessionLifecycle::new();
        l.wrote(t0);
        l.saw(&result_with(0, 1), t0);
        // Written but never replayed, and a background task is outstanding: no close.
        assert_eq!(
            l.decide(t0 + Duration::from_secs(300), true),
            LifecycleStep::Keep
        );
        l.saw(&StreamEvent::BackgroundTasks { count: 0 }, t0);
        l.saw(&result_with(0, 0), t0 + Duration::from_secs(300));
        assert_eq!(
            l.decide(t0 + Duration::from_secs(330), true),
            LifecycleStep::CloseStdinWithWarning
        );
    }

    #[test]
    fn the_idle_settle_override_is_bounded() {
        assert_eq!(idle_settle_from(Some("1")), Duration::from_secs(1));
        assert_eq!(idle_settle_from(Some(" 600 ")), Duration::from_secs(600));
        for bad in [None, Some("0"), Some("601"), Some("x"), Some("")] {
            assert_eq!(idle_settle_from(bad), IDLE_SETTLE, "{bad:?}");
        }
    }

    const BG_RUN: &str = include_str!("../../tests/fixtures/oneoff/real-bg-subagents.stream.jsonl");
    const RESUMED_RUN: &str = include_str!("../../tests/fixtures/oneoff/real-resumed.stream.jsonl");

    /// Feed a recorded stream through the lifecycle, one line per second. Returns, per
    /// line index, the events and what `decide(.., report)` answered after each result.
    fn replay_run(
        run: &str,
        report: bool,
    ) -> (SessionLifecycle, Vec<(usize, StreamEvent, LifecycleStep)>) {
        let t0 = Instant::now();
        let rt = crate::oneoff::runtime::AgentRuntime::default();
        let mut l = SessionLifecycle::new();
        l.wrote(t0);
        let mut results = Vec::new();
        for (i, line) in run.lines().enumerate() {
            let now = t0 + Duration::from_secs(i as u64);
            for event in rt.classify(line) {
                l.saw(&event, now);
                if matches!(event, StreamEvent::TurnResult { .. }) {
                    let step = l.decide(now, report);
                    results.push((i, event.clone(), step));
                    if step != LifecycleStep::Keep {
                        return (l, results);
                    }
                }
            }
        }
        (l, results)
    }

    #[test]
    fn the_recorded_background_run_closes_only_at_its_final_result() {
        let t0 = Instant::now();
        let rt = crate::oneoff::runtime::AgentRuntime::default();
        let lines: Vec<&str> = BG_RUN.lines().collect();
        assert_eq!(lines.len(), 304);

        // The line whose background set first reached 0 after having been non-empty.
        let mut peak = false;
        let mut drained_at = None;
        for (i, line) in lines.iter().enumerate() {
            for event in rt.classify(line) {
                if let StreamEvent::BackgroundTasks { count } = event {
                    if count > 0 {
                        peak = true;
                    } else if peak && drained_at.is_none() {
                        drained_at = Some(i);
                    }
                }
            }
        }
        let drained_at = drained_at.expect("the set drains");

        let mut l = SessionLifecycle::new();
        l.wrote(t0);
        let mut first_result = None;
        let mut first_close = None;
        for (i, line) in lines.iter().enumerate() {
            let now = t0 + Duration::from_secs(i as u64);
            for event in rt.classify(line) {
                l.saw(&event, now);
                if !matches!(event, StreamEvent::TurnResult { .. }) {
                    continue;
                }
                let step = l.decide(now, true);
                if first_result.is_none() {
                    first_result = Some((i, now));
                    assert_eq!(
                        step,
                        LifecycleStep::Keep,
                        "line {i}: four tasks outstanding"
                    );
                    assert_eq!(l.outstanding(), 4);
                    // Quiet for five minutes and no report: still not the end.
                    assert_eq!(
                        l.decide(now + Duration::from_secs(300), false),
                        LifecycleStep::Keep
                    );
                } else if step != LifecycleStep::Keep && first_close.is_none() {
                    first_close = Some((i, step));
                }
            }
            if first_close.is_some() {
                break;
            }
        }
        let (first_at, _) = first_result.expect("a first result");
        assert!(first_at < drained_at, "the first result precedes the drain");
        let (close_at, step) = first_close.expect("the run closes");
        assert!(
            close_at > drained_at,
            "closed at {close_at}, drained at {drained_at}"
        );
        assert_eq!(step, LifecycleStep::CloseStdin);
        // It is the first result after the drain.
        let first_after = lines
            .iter()
            .enumerate()
            .skip(drained_at)
            .find(|(_, line)| {
                rt.classify(line)
                    .iter()
                    .any(|e| matches!(e, StreamEvent::TurnResult { .. }))
            })
            .map(|(i, _)| i);
        assert_eq!(Some(close_at), first_after);
    }

    #[test]
    fn the_recorded_resumed_run_closes_at_its_result_once_reported() {
        let (_, results) = replay_run(RESUMED_RUN, true);
        let (_, _, step) = results.last().expect("a result");
        assert_eq!(*step, LifecycleStep::CloseStdin);

        // Without a report it keeps, then closes after the idle settle.
        let rt = crate::oneoff::runtime::AgentRuntime::default();
        let t0 = Instant::now();
        let mut l = SessionLifecycle::new();
        l.wrote(t0);
        let mut at = t0;
        for (i, line) in RESUMED_RUN.lines().enumerate() {
            at = t0 + Duration::from_secs(i as u64);
            for event in rt.classify(line) {
                l.saw(&event, at);
            }
        }
        assert_eq!(l.decide(at, false), LifecycleStep::Keep);
        assert_eq!(
            l.decide(at + Duration::from_secs(119), false),
            LifecycleStep::Keep
        );
        assert_eq!(
            l.decide(at + IDLE_SETTLE, false),
            LifecycleStep::CloseStdinIdle
        );
    }
}
