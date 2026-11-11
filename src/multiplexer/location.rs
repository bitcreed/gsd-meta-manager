//! Where a session lives: the read-only twin of focus (quick 261111-h9f).
//!
//! The Sessions tab shows each session's multiplexer location
//! (`tmux 0:1.2`, `herdr w2:p1 (via tmux 0:0.0)`). This module resolves it with
//! the SAME routing focus uses (`ProcFs::session_context` + [`plan_focus`]) and
//! the same per-backend joins ([`MultiplexerBackend::locate_session`] is the join
//! `focus` runs, minus the act), and it never selects or focuses anything. There
//! is no second detector: a backend that focus would not ask is not asked here.
//!
//! Resolution is **blocking** (it runs the tmux and herdr CLIs) and therefore
//! runs only off the render path: `app.rs` calls [`locate_sessions`] on
//! `spawn_blocking` and stores the result in a PID-keyed
//! [`SessionLocationCache`]. The Sessions tab only reads that cache.

use super::{
    plan_focus, reveal_targets, Detected, EnvSnapshot, MatchedBy, MultiplexerBackend, ProcFs,
    REGISTRY,
};
use crate::session_detector::ClaudeSession;
use crate::text::Untrusted;
use std::collections::HashMap;

/// One pane a session was found in. The target is tool output (a tmux session
/// name is whatever its owner typed), so it is [`Untrusted`] and only ever drawn
/// through [`Untrusted::shown`] (T-261111-h9f-01).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PaneLocation {
    /// The backend that found it (`"tmux"`, `"herdr"`).
    pub backend: &'static str,
    /// The backend's own address for the pane: tmux's `session:window.pane`,
    /// herdr's pane id.
    pub target: Untrusted,
    /// How the backend joined the session to the pane.
    pub by: MatchedBy,
}

impl PaneLocation {
    /// `"<backend> <target>"`, the target through [`Untrusted::shown`].
    fn shown(&self) -> String {
        format!("{} {}", self.backend, self.target.shown())
    }
}

/// Where a session lives: its pane, and, when that pane's multiplexer is itself
/// displayed in a pane of another one, that outer pane.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SessionLocation {
    pub pane: PaneLocation,
    /// The pane of another multiplexer that shows `pane`'s client (the reveal
    /// step's lookup, without the select).
    pub via: Option<PaneLocation>,
}

impl SessionLocation {
    /// The text of the Sessions row's location cell: `tmux 0:1.2`,
    /// `herdr w2:p1 (via tmux 0:0.0)`. A herdr pane matched by working directory
    /// only is a guess (focus says so in a note) and gets a trailing `?`.
    pub fn label(&self) -> String {
        let mut label = self.pane.shown();
        if self.pane.by == MatchedBy::Cwd {
            label.push('?');
        }
        if let Some(via) = &self.via {
            label.push_str(&format!(" (via {})", via.shown()));
        }
        label
    }
}

/// The outcome of resolving one session, with the session's start time so a
/// reused PID is never read as the earlier process.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LocatedEntry {
    pub start_time: Option<u64>,
    /// `None`: every backend in the plan answered and none found the session.
    pub location: Option<SessionLocation>,
}

/// One resolution run's output, keyed by session PID.
pub type ResolvedLocations = HashMap<u32, LocatedEntry>;

/// What the cache knows about one session.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LocationLookup<'a> {
    /// No resolution for this PID and start time has landed yet.
    Pending,
    /// Resolved, and no multiplexer finds the session.
    Absent,
    /// Resolved.
    At(&'a SessionLocation),
}

/// The PID-keyed cache the Sessions tab reads. Written only by the
/// `SessionLocationsResolved` handler; never resolved at render time.
#[derive(Debug, Clone, Default)]
pub struct SessionLocationCache {
    pub entries: ResolvedLocations,
    /// A resolution is running. Set before the spawn and cleared first in the
    /// handler, so a wedged CLI delays results but cannot stack threads.
    pub in_flight: bool,
}

impl SessionLocationCache {
    /// The cached answer for `session`. An entry whose start time differs from
    /// the session's belongs to an earlier process that had the same PID, so it
    /// reads as [`LocationLookup::Pending`], never as a stale location.
    pub fn lookup(&self, session: &ClaudeSession) -> LocationLookup<'_> {
        match self.entries.get(&session.pid) {
            Some(entry) if entry.start_time == session.start_time => match &entry.location {
                Some(location) => LocationLookup::At(location),
                None => LocationLookup::Absent,
            },
            _ => LocationLookup::Pending,
        }
    }
}

/// Where `session` lives, asked of `backends` in order (the first answer wins),
/// like `focus_with_procs` minus the act. A detection that is `inside` but not
/// `reachable` is skipped with its reason. Failures go to the `debug` log only.
pub(crate) fn locate_with_procs(
    backends: &[Detected],
    env: &EnvSnapshot,
    session: &ClaudeSession,
    procfs: &ProcFs,
) -> Option<SessionLocation> {
    for detected in backends {
        let name = detected.backend.name();
        if let Err(why) = &detected.detection.reachable {
            tracing::debug!(backend = name, reason = %why, "location: backend unreachable");
            continue;
        }
        match detected.backend.locate_session(env, session) {
            Ok(pane) => {
                let via = locate_via(detected, backends, env, procfs);
                return Some(SessionLocation { pane, via });
            }
            Err(e) => {
                tracing::debug!(backend = name, reason = %e, "location: session not found");
            }
        }
    }
    None
}

/// The pane of ANOTHER multiplexer that shows the `winner`'s client: the reveal
/// step's lookup, without the select. The same `reveal_targets` head, then the
/// first other backend (plan order) whose `locate_tty` answers. No client
/// terminal, no other backend, or no pane showing it: no `via`.
fn locate_via(
    winner: &Detected,
    backends: &[Detected],
    env: &EnvSnapshot,
    procfs: &ProcFs,
) -> Option<PaneLocation> {
    let (others, ttys) = reveal_targets(winner, backends, procfs)?;
    if ttys.is_empty() {
        return None;
    }
    others
        .iter()
        .find_map(|other| match other.backend.locate_tty(env, &ttys) {
            Ok(target) => Some(PaneLocation {
                backend: other.backend.name(),
                target,
                by: MatchedBy::Tty,
            }),
            Err(e) => {
                tracing::debug!(
                    backend = other.backend.name(),
                    reason = %e,
                    "location: client terminal not found"
                );
                None
            }
        })
}

/// [`locate_sessions`] with its inputs explicit: the `/proc` to read, the
/// registry, the TUI's own environment and pid. Per session it builds the same
/// plan `focus_message_in` builds.
pub(crate) fn locate_sessions_in(
    procfs: &ProcFs,
    registry: &[&'static dyn MultiplexerBackend],
    tui_env: &EnvSnapshot,
    tui_pid: u32,
    sessions: &[ClaudeSession],
) -> ResolvedLocations {
    let tui_ancestry = procfs.chain(tui_pid);
    sessions
        .iter()
        .map(|session| {
            let context = procfs.session_context(session.pid);
            let plan = plan_focus(registry, tui_env, &tui_ancestry, &context);
            let location = locate_with_procs(&plan.backends, &plan.env, session, procfs);
            (
                session.pid,
                LocatedEntry {
                    start_time: session.start_time,
                    location,
                },
            )
        })
        .collect()
}

/// Resolve the location of every session over the real environment. **The
/// real-environment entry: blocking, called only from `app.rs` on
/// `spawn_blocking`** (`tests/focus_entry_guard.rs` pins that).
pub fn locate_sessions(sessions: &[ClaudeSession]) -> ResolvedLocations {
    if sessions.is_empty() {
        return ResolvedLocations::new();
    }
    locate_sessions_in(
        &ProcFs::system(),
        REGISTRY,
        &EnvSnapshot::from_process(),
        std::process::id(),
        sessions,
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::multiplexer::{Detection, FocusOutcome, MuxError};
    use std::path::PathBuf;
    use std::sync::Mutex;

    fn pane(backend: &'static str, target: &str, by: MatchedBy) -> PaneLocation {
        PaneLocation {
            backend,
            target: Untrusted::from_untrusted_source(target.to_string()),
            by,
        }
    }

    fn session(pid: u32, start_time: Option<u64>) -> ClaudeSession {
        ClaudeSession {
            pid,
            kind: crate::session_detector::SessionKind::Claude,
            session_id: None,
            working_dir: PathBuf::from("/work/proj"),
            start_time,
            tty: Some("pts/3".to_string()),
        }
    }

    /// A backend that answers a configured `locate_session` and records every
    /// act it is asked to do.
    struct Fake {
        name: &'static str,
        answer: Result<PaneLocation, MuxError>,
        acts: Mutex<Vec<&'static str>>,
        asked: Mutex<u32>,
    }

    impl Fake {
        fn leak(name: &'static str, answer: Result<PaneLocation, MuxError>) -> &'static Fake {
            Box::leak(Box::new(Fake {
                name,
                answer,
                acts: Mutex::new(Vec::new()),
                asked: Mutex::new(0),
            }))
        }
    }

    impl MultiplexerBackend for Fake {
        fn name(&self) -> &'static str {
            self.name
        }
        fn detect(&self, _env: &EnvSnapshot) -> Option<Detection> {
            None
        }
        fn locate_session(
            &self,
            _env: &EnvSnapshot,
            _session: &ClaudeSession,
        ) -> Result<PaneLocation, MuxError> {
            *self.asked.lock().unwrap() += 1;
            self.answer.clone()
        }
        fn focus(
            &self,
            _env: &EnvSnapshot,
            _session: &ClaudeSession,
        ) -> Result<FocusOutcome, MuxError> {
            self.acts.lock().unwrap().push("focus");
            Err(MuxError::Unsupported("fake".to_string()))
        }
        fn open(
            &self,
            _env: &EnvSnapshot,
            _cwd: &std::path::Path,
            _argv: &crate::multiplexer::ExecArgv,
        ) -> Result<(), MuxError> {
            self.acts.lock().unwrap().push("open");
            Ok(())
        }
        fn focus_tty(
            &self,
            _env: &EnvSnapshot,
            _ttys: &[String],
        ) -> Result<FocusOutcome, MuxError> {
            self.acts.lock().unwrap().push("focus_tty");
            Err(MuxError::Unsupported("fake".to_string()))
        }
    }

    fn detected(backend: &'static Fake, reachable: Result<(), String>) -> Detected {
        Detected {
            backend,
            detection: Detection {
                inside: true,
                reachable,
            },
        }
    }

    #[test]
    fn locate_label_formats_tmux_and_herdr_via_tmux() {
        let tmux = SessionLocation {
            pane: pane("tmux", "0:1.2", MatchedBy::Tty),
            via: None,
        };
        assert_eq!(tmux.label(), "tmux 0:1.2");
        let herdr = SessionLocation {
            pane: pane("herdr", "w2:p1", MatchedBy::SessionId),
            via: Some(pane("tmux", "0:0.0", MatchedBy::Tty)),
        };
        assert_eq!(herdr.label(), "herdr w2:p1 (via tmux 0:0.0)");
    }

    #[test]
    fn locate_label_marks_a_directory_only_match() {
        let guess = SessionLocation {
            pane: pane("herdr", "w2:p1", MatchedBy::Cwd),
            via: None,
        };
        assert_eq!(guess.label(), "herdr w2:p1?");
    }

    #[test]
    fn locate_label_escapes_a_hostile_target() {
        let hostile = "evil\u{1b}[2J\u{202e}name";
        let location = SessionLocation {
            pane: pane("herdr", hostile, MatchedBy::SessionId),
            via: Some(pane("tmux", hostile, MatchedBy::Tty)),
        };
        let label = location.label();
        assert!(!label.contains('\u{1b}'), "{label:?}");
        assert!(!label.contains('\u{202e}'), "{label:?}");
        let shown = crate::text::render_for_terminal(hostile).to_string();
        assert_eq!(label, format!("herdr {shown} (via tmux {shown})"));
    }

    #[test]
    fn locate_lookup_is_pending_absent_or_at_and_guards_pid_reuse() {
        let mut cache = SessionLocationCache::default();
        let live = session(4242, Some(7));
        assert_eq!(cache.lookup(&live), LocationLookup::Pending, "missing pid");

        let at = SessionLocation {
            pane: pane("tmux", "0:1.2", MatchedBy::Tty),
            via: None,
        };
        cache.entries.insert(
            4242,
            LocatedEntry {
                start_time: Some(7),
                location: Some(at.clone()),
            },
        );
        assert_eq!(cache.lookup(&live), LocationLookup::At(&at));
        assert_eq!(
            cache.lookup(&session(4242, Some(8))),
            LocationLookup::Pending,
            "a reused pid is pending, never the earlier process's location"
        );

        cache.entries.insert(
            4242,
            LocatedEntry {
                start_time: Some(7),
                location: None,
            },
        );
        assert_eq!(cache.lookup(&live), LocationLookup::Absent);
    }

    #[test]
    fn locate_with_procs_takes_the_first_finder_and_never_focuses() {
        let first = Fake::leak("first", Err(MuxError::NotFound("nope".to_string())));
        let second = Fake::leak("second", Ok(pane("second", "x:1", MatchedBy::Tty)));
        let third = Fake::leak("third", Ok(pane("third", "never", MatchedBy::Tty)));
        let backends = [
            detected(first, Ok(())),
            detected(second, Ok(())),
            detected(third, Ok(())),
        ];
        let found = locate_with_procs(
            &backends,
            &EnvSnapshot::default(),
            &session(1, None),
            &ProcFs::none(),
        )
        .expect("the second backend finds it");
        assert_eq!(found.pane.backend, "second");
        assert_eq!(found.label(), "second x:1");
        assert_eq!(*third.asked.lock().unwrap(), 0, "first success wins");
        for fake in [first, second, third] {
            assert!(
                fake.acts.lock().unwrap().is_empty(),
                "locating must not act: {:?}",
                fake.acts.lock().unwrap()
            );
        }
    }

    #[test]
    fn locate_with_procs_skips_an_unreachable_backend() {
        let blocked = Fake::leak("blocked", Ok(pane("blocked", "no", MatchedBy::Tty)));
        let open = Fake::leak("open", Ok(pane("open", "yes", MatchedBy::Tty)));
        let backends = [
            detected(blocked, Err("socket unreachable".to_string())),
            detected(open, Ok(())),
        ];
        let found = locate_with_procs(
            &backends,
            &EnvSnapshot::default(),
            &session(1, None),
            &ProcFs::none(),
        )
        .expect("the reachable backend answers");
        assert_eq!(found.pane.backend, "open");
        assert_eq!(*blocked.asked.lock().unwrap(), 0, "never asked");
    }
}
