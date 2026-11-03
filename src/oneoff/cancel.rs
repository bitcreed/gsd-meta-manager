//! Stopping, killing and pausing a one-off run or its conflict resolver (quick
//! 261103-klf, L-1, L-2, L-4, C-01..C-08, C-13, C-14, I-09..I-13).
//!
//! A requester (the CLI's `task stop`, the TUI's `x`/`X`/`z`) writes a small request
//! file, `<run-dir>/stop.request` for the run's agent or `<run-dir>/land/stop.request`
//! for a landing, and only the run's own launcher daemon (or the landing process)
//! acts on it: TERM then KILL after a grace, an immediate KILL, or a pause message and
//! a longer grace. The files sit outside every sandbox bind, so neither the agent nor a
//! resolver can write them.
//!
//! This file holds the request types and the I/O of the request file (linux/macos), the
//! two graces and the pure state machine ([`Cancel`]) the supervised wait loop drives
//! with injected instants. Nothing here names the agent.

use std::time::{Duration, Instant};

use serde::{Deserialize, Serialize};

/// The request file inside a run dir (the run's agent) or its `land/` dir (a landing).
pub const STOP_REQUEST_FILE: &str = "stop.request";

/// The most bytes of a request file that are read (I-09).
pub const MAX_REQUEST_BYTES: u64 = 4096;

/// Seconds between TERM and KILL for a hard cut: the environment variable that shortens
/// it (tests only; 1..=600). Read by the host daemon at session start.
pub const STOP_GRACE_ENV: &str = "GSDMM_ONEOFF_STOP_GRACE_SECS";
/// The default TERM to KILL grace (C-03).
pub const STOP_GRACE: Duration = Duration::from_secs(10);

/// Seconds a pause may take before it is cut hard: the environment variable that
/// shortens it (tests only; 1..=600).
pub const PAUSE_GRACE_ENV: &str = "GSDMM_ONEOFF_PAUSE_GRACE_SECS";
/// The default pause grace (C-14).
pub const PAUSE_GRACE: Duration = Duration::from_secs(300);

/// What a requester wants (C-02). Ordered by strength: kill > hard > pause.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StopKind {
    /// Ask the agent to commit its work in progress and report, then end.
    Pause,
    /// SIGTERM the scope or group, SIGKILL after the grace.
    Hard,
    /// SIGKILL at once.
    Kill,
}

impl StopKind {
    fn rank(self) -> u8 {
        match self {
            StopKind::Pause => 0,
            StopKind::Hard => 1,
            StopKind::Kill => 2,
        }
    }

    /// Whether `self` is strictly stronger than `other`.
    pub fn outranks(self, other: StopKind) -> bool {
        self.rank() > other.rank()
    }

    /// The stronger of two kinds.
    pub fn stronger(self, other: StopKind) -> StopKind {
        if other.rank() > self.rank() {
            other
        } else {
            self
        }
    }

    /// The word recorded and shown.
    pub fn word(self) -> &'static str {
        match self {
            StopKind::Pause => "pause",
            StopKind::Hard => "hard",
            StopKind::Kill => "kill",
        }
    }
}

/// Who asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StopBy {
    Tui,
    Cli,
    /// A TERM or Ctrl-C arrived with no request file (an older CLI, a terminal).
    Signal,
}

impl StopBy {
    pub fn word(self) -> &'static str {
        match self {
            StopBy::Tui => "tui",
            StopBy::Cli => "cli",
            StopBy::Signal => "signal",
        }
    }
}

/// The request file's content.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StopRequest {
    pub kind: StopKind,
    pub requested_at: String,
    pub by: StopBy,
}

/// What `run.json` keeps of a stop (C-07, I-13): written with `ended_at: None` when the
/// daemon first sees the request, completed when the run is finalized.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct StopRecord {
    pub kind: StopKind,
    pub requested_at: String,
    #[serde(default)]
    pub ended_at: Option<String>,
    /// The daemon had to escalate: a pause that ran out its grace, or TERM that was not
    /// enough.
    #[serde(default)]
    pub escalated: bool,
    pub by: StopBy,
}

impl StopRecord {
    /// A record for `request`, not yet ended.
    pub fn begun(request: &StopRequest) -> Self {
        StopRecord {
            kind: request.kind,
            requested_at: request.requested_at.clone(),
            ended_at: None,
            escalated: false,
            by: request.by,
        }
    }
}

/// `pending` with `new` folded in: the stronger kind wins and a requester never
/// downgrades a pending request (I-09).
pub fn merge_request(pending: Option<StopRequest>, new: StopRequest) -> StopRequest {
    match pending {
        Some(old) if old.kind.rank() >= new.kind.rank() => old,
        _ => new,
    }
}

/// Parse a request file's text: at most [`MAX_REQUEST_BYTES`], a JSON object with a
/// known kind and requester.
pub fn parse_request(text: &str) -> Result<StopRequest, String> {
    if text.len() as u64 > MAX_REQUEST_BYTES {
        return Err("the stop request is too large".to_owned());
    }
    serde_json::from_str(text).map_err(|e| format!("the stop request is not valid: {e}"))
}

/// A grace from an environment value: an integer 1..=600 seconds, else `default`.
pub fn grace_with_default(value: Option<&str>, default: Duration) -> Duration {
    value
        .and_then(|v| v.trim().parse::<u64>().ok())
        .filter(|secs| (1..=600).contains(secs))
        .map_or(default, Duration::from_secs)
}

/// The TERM to KILL grace in force.
pub fn stop_grace_from_env() -> Duration {
    grace_with_default(std::env::var(STOP_GRACE_ENV).ok().as_deref(), STOP_GRACE)
}

/// The pause grace in force.
pub fn pause_grace_from_env() -> Duration {
    grace_with_default(std::env::var(PAUSE_GRACE_ENV).ok().as_deref(), PAUSE_GRACE)
}

/// What the wait loop must do this tick.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CancelStep {
    /// Nothing, or a grace still running.
    Wait,
    /// Inject the pause message (once).
    InjectPause,
    /// SIGTERM the scope or group.
    Term,
    /// SIGKILL the scope or group.
    Kill,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Phase {
    Idle,
    Pausing { deadline: Instant },
    Terming { deadline: Instant, from_pause: bool },
    Killing,
}

/// How a cut ended, once the child is gone.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CutEnd {
    /// TERM was enough for a hard request.
    Terminated,
    /// KILL was sent; `escalated` says the daemon, not the user, escalated to it.
    Killed { escalated: bool },
    /// A pause whose grace ran out: cut, even though TERM may have sufficed (C-05).
    PauseCut,
}

/// The pure cancel state machine, SoftCap's shape (S-01): the wait loop feeds it the
/// strongest request it knows and an instant, and does what it answers.
#[derive(Debug, Clone)]
pub struct Cancel {
    stop_grace: Duration,
    pause_grace: Duration,
    phase: Phase,
    /// KILL was reached by the daemon's own escalation.
    escalated: bool,
    /// The pause grace ran out.
    pause_cut: bool,
}

impl Cancel {
    pub fn new(stop_grace: Duration, pause_grace: Duration) -> Self {
        Cancel {
            stop_grace,
            pause_grace,
            phase: Phase::Idle,
            escalated: false,
            pause_cut: false,
        }
    }

    /// One tick: `wanted` is the strongest request known (`None` while there is none).
    pub fn tick(&mut self, wanted: Option<StopKind>, now: Instant) -> CancelStep {
        match (self.phase, wanted) {
            (_, None) | (Phase::Killing, _) => CancelStep::Wait,
            (Phase::Idle, Some(StopKind::Pause)) => {
                self.phase = Phase::Pausing {
                    deadline: now + self.pause_grace,
                };
                CancelStep::InjectPause
            }
            (Phase::Idle | Phase::Pausing { .. }, Some(StopKind::Hard)) => {
                self.phase = Phase::Terming {
                    deadline: now + self.stop_grace,
                    from_pause: false,
                };
                CancelStep::Term
            }
            (Phase::Idle | Phase::Pausing { .. } | Phase::Terming { .. }, Some(StopKind::Kill)) => {
                self.phase = Phase::Killing;
                CancelStep::Kill
            }
            (Phase::Pausing { deadline }, Some(StopKind::Pause)) => {
                if now >= deadline {
                    self.pause_cut = true;
                    self.phase = Phase::Terming {
                        deadline: now + self.stop_grace,
                        from_pause: true,
                    };
                    CancelStep::Term
                } else {
                    CancelStep::Wait
                }
            }
            (Phase::Terming { deadline, .. }, Some(_)) => {
                if now >= deadline {
                    self.escalated = true;
                    self.phase = Phase::Killing;
                    CancelStep::Kill
                } else {
                    CancelStep::Wait
                }
            }
        }
    }

    /// Whether the daemon has started a cut (TERM or KILL) the child must now die of.
    pub fn cutting(&self) -> bool {
        matches!(self.phase, Phase::Terming { .. } | Phase::Killing)
    }

    /// How the cut ended, if one was made.
    pub fn end(&self) -> Option<CutEnd> {
        match self.phase {
            Phase::Idle | Phase::Pausing { .. } => None,
            _ if self.pause_cut => Some(CutEnd::PauseCut),
            Phase::Terming { .. } => Some(CutEnd::Terminated),
            Phase::Killing => Some(CutEnd::Killed {
                escalated: self.escalated,
            }),
        }
    }

    /// Whether the record's `escalated` flag is set: the daemon cut a pause or
    /// escalated TERM to KILL.
    pub fn escalated(&self) -> bool {
        self.escalated || self.pause_cut
    }
}

#[cfg(any(target_os = "linux", target_os = "macos"))]
pub use self::io::*;

#[cfg(any(target_os = "linux", target_os = "macos"))]
mod io {
    use std::fs;
    use std::io::Read;
    use std::os::unix::fs::{DirBuilderExt, OpenOptionsExt};
    use std::path::Path;
    use std::time::Duration;

    use chrono::Utc;

    use super::{
        merge_request, parse_request, StopBy, StopKind, StopRequest, MAX_REQUEST_BYTES,
        STOP_REQUEST_FILE,
    };
    use crate::launcher::process::run_bounded;
    use crate::oneoff::store::rfc3339;

    /// A request for `kind` made now by `by`.
    pub fn request_now(kind: StopKind, by: StopBy) -> StopRequest {
        StopRequest {
            kind,
            requested_at: rfc3339(Utc::now()),
            by,
        }
    }

    /// The request in `dir`, if any. `O_NOFOLLOW`, bounded to 4 KiB; a file that is
    /// there but unreadable, oversized or not a request is an error (T-klf-03).
    pub fn read_request(dir: &Path) -> Result<Option<StopRequest>, String> {
        let path = dir.join(STOP_REQUEST_FILE);
        let flags = rustix::fs::OFlags::NOFOLLOW.bits() | rustix::fs::OFlags::NONBLOCK.bits();
        let file = match fs::OpenOptions::new()
            .read(true)
            .custom_flags(i32::try_from(flags).unwrap_or(0))
            .open(&path)
        {
            Ok(file) => file,
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(None),
            Err(e) => return Err(format!("read {}: {e}", path.display())),
        };
        let meta = file.metadata().map_err(|e| e.to_string())?;
        if !meta.file_type().is_file() {
            return Err("the stop request is not a regular file".to_owned());
        }
        let mut text = String::new();
        file.take(MAX_REQUEST_BYTES + 1)
            .read_to_string(&mut text)
            .map_err(|e| format!("read {}: {e}", path.display()))?;
        parse_request(&text).map(Some)
    }

    /// Write `request` into `dir` (created 0700 when missing) for the daemon to read:
    /// merged with a pending request so the stronger kind wins, to a temp file in the
    /// same dir, 0600, then renamed (I-09). Returns the request that is now pending.
    pub fn write_request(dir: &Path, request: StopRequest) -> Result<StopRequest, String> {
        match fs::DirBuilder::new().mode(0o700).create(dir) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(format!("create {}: {e}", dir.display())),
        }
        let pending = read_request(dir).ok().flatten();
        let merged = merge_request(pending, request);
        let text = serde_json::to_string(&merged).map_err(|e| e.to_string())?;
        let temp = dir.join(format!("{STOP_REQUEST_FILE}.tmp-{}", std::process::id()));
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create(true)
            .truncate(true)
            .mode(0o600)
            .open(&temp)
            .map_err(|e| format!("write {}: {e}", temp.display()))?;
        std::io::Write::write_all(&mut file, text.as_bytes())
            .map_err(|e| format!("write {}: {e}", temp.display()))?;
        drop(file);
        fs::rename(&temp, dir.join(STOP_REQUEST_FILE))
            .map_err(|e| format!("write {}: {e}", dir.join(STOP_REQUEST_FILE).display()))?;
        Ok(merged)
    }

    /// Remove a stale request in `dir` (a landing does it right after taking its lock).
    pub fn clear_request(dir: &Path) {
        let _ = fs::remove_file(dir.join(STOP_REQUEST_FILE));
    }

    /// Signal the scope `unit` by name: `systemctl --user kill --signal=<sig> <unit>`.
    /// The caller has checked the name is a one-off scope.
    pub fn signal_scope(unit: &str, signal: &str) {
        let _ = run_bounded(
            "systemctl",
            &[
                "--user".into(),
                "kill".into(),
                format!("--signal={signal}").into(),
                unit.into(),
            ],
            None,
            &[],
            Duration::from_secs(30),
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn at(base: Instant, secs: u64) -> Instant {
        base + Duration::from_secs(secs)
    }

    fn req(kind: StopKind) -> StopRequest {
        StopRequest {
            kind,
            requested_at: "2026-11-03T10:00:00Z".to_owned(),
            by: StopBy::Cli,
        }
    }

    #[test]
    fn kill_beats_hard_beats_pause_and_a_request_never_downgrades() {
        assert_eq!(StopKind::Pause.stronger(StopKind::Hard), StopKind::Hard);
        assert_eq!(StopKind::Kill.stronger(StopKind::Hard), StopKind::Kill);
        assert_eq!(StopKind::Hard.stronger(StopKind::Pause), StopKind::Hard);
        let merged = merge_request(Some(req(StopKind::Hard)), req(StopKind::Pause));
        assert_eq!(merged.kind, StopKind::Hard);
        let merged = merge_request(Some(req(StopKind::Pause)), req(StopKind::Kill));
        assert_eq!(merged.kind, StopKind::Kill);
        assert_eq!(
            merge_request(None, req(StopKind::Pause)).kind,
            StopKind::Pause
        );
    }

    #[test]
    fn a_request_round_trips_and_garbage_is_an_error() {
        let text = serde_json::to_string(&req(StopKind::Kill)).unwrap();
        assert!(text.contains(r#""kind":"kill""#), "{text}");
        assert_eq!(parse_request(&text).unwrap(), req(StopKind::Kill));
        assert!(parse_request("not json").is_err());
        assert!(parse_request(r#"{"kind":"explode","requested_at":"x","by":"cli"}"#).is_err());
        assert!(parse_request(r#"{"kind":"hard","requested_at":"x","by":"nobody"}"#).is_err());
        let big = format!(
            r#"{{"kind":"hard","requested_at":"{}","by":"cli"}}"#,
            "x".repeat(5000)
        );
        assert!(parse_request(&big).unwrap_err().contains("too large"));
    }

    #[test]
    fn both_graces_parse_like_the_soft_cap_grace() {
        let d = |s| Duration::from_secs(s);
        assert_eq!(grace_with_default(None, STOP_GRACE), d(10));
        assert_eq!(grace_with_default(None, PAUSE_GRACE), d(300));
        for ok in ["1", " 600 ", "42"] {
            assert_eq!(
                grace_with_default(Some(ok), STOP_GRACE),
                d(ok.trim().parse().unwrap())
            );
        }
        for bad in ["0", "601", "ten", "-3", ""] {
            assert_eq!(grace_with_default(Some(bad), STOP_GRACE), d(10), "{bad:?}");
        }
    }

    #[test]
    fn a_pause_injects_once_waits_then_terms_then_kills() {
        let t = Instant::now();
        let mut c = Cancel::new(Duration::from_secs(10), Duration::from_secs(300));
        assert_eq!(c.tick(None, t), CancelStep::Wait);
        assert_eq!(c.tick(Some(StopKind::Pause), t), CancelStep::InjectPause);
        assert_eq!(c.tick(Some(StopKind::Pause), at(t, 1)), CancelStep::Wait);
        assert_eq!(c.tick(Some(StopKind::Pause), at(t, 299)), CancelStep::Wait);
        assert!(!c.cutting());
        assert_eq!(c.end(), None);
        assert_eq!(c.tick(Some(StopKind::Pause), at(t, 300)), CancelStep::Term);
        assert!(c.cutting());
        assert_eq!(c.tick(Some(StopKind::Pause), at(t, 305)), CancelStep::Wait);
        assert_eq!(c.tick(Some(StopKind::Pause), at(t, 310)), CancelStep::Kill);
        assert!(c.escalated());
        assert_eq!(c.end(), Some(CutEnd::PauseCut));
        assert_eq!(c.tick(Some(StopKind::Pause), at(t, 400)), CancelStep::Wait);
    }

    #[test]
    fn a_pause_cut_is_a_pause_cut_even_when_term_sufficed() {
        let t = Instant::now();
        let mut c = Cancel::new(Duration::from_secs(10), Duration::from_secs(5));
        c.tick(Some(StopKind::Pause), t);
        assert_eq!(c.tick(Some(StopKind::Pause), at(t, 5)), CancelStep::Term);
        assert_eq!(c.end(), Some(CutEnd::PauseCut));
        assert!(c.escalated());
    }

    #[test]
    fn a_hard_request_terms_waits_the_grace_then_kills() {
        let t = Instant::now();
        let mut c = Cancel::new(Duration::from_secs(10), Duration::from_secs(300));
        assert_eq!(c.tick(Some(StopKind::Hard), t), CancelStep::Term);
        assert_eq!(c.end(), Some(CutEnd::Terminated));
        assert!(!c.escalated());
        assert_eq!(c.tick(Some(StopKind::Hard), at(t, 9)), CancelStep::Wait);
        assert_eq!(c.tick(Some(StopKind::Hard), at(t, 10)), CancelStep::Kill);
        assert_eq!(c.end(), Some(CutEnd::Killed { escalated: true }));
    }

    #[test]
    fn a_kill_request_kills_at_once_without_escalation() {
        let t = Instant::now();
        let mut c = Cancel::new(Duration::from_secs(10), Duration::from_secs(300));
        assert_eq!(c.tick(Some(StopKind::Kill), t), CancelStep::Kill);
        assert_eq!(c.end(), Some(CutEnd::Killed { escalated: false }));
        assert!(!c.escalated());
    }

    #[test]
    fn a_hard_request_during_a_pause_terms_at_once_and_kill_overrides_a_term() {
        let t = Instant::now();
        let mut c = Cancel::new(Duration::from_secs(10), Duration::from_secs(300));
        assert_eq!(c.tick(Some(StopKind::Pause), t), CancelStep::InjectPause);
        assert_eq!(c.tick(Some(StopKind::Hard), at(t, 1)), CancelStep::Term);
        assert_eq!(c.tick(Some(StopKind::Kill), at(t, 2)), CancelStep::Kill);
        assert_eq!(c.end(), Some(CutEnd::Killed { escalated: false }));
    }
}
