//! Process evidence for agent worktrees, as pure logic over injected facts
//! (quick 260926-06g).
//!
//! `src/agents` still reads no process table itself (D-B01/D-B03, amended):
//! the caller takes a read-only snapshot through
//! [`crate::session_detector::ProcessProbe`] and hands it in as data. Off
//! Linux, or whenever the probe cannot answer, every verdict here is
//! [`ProcessEvidence::Unknown`] and liveness stays mtime-only.
//!
//! The lock-reason pid is the top-level session every agent shares (D-A05),
//! so a LIVE owner says nothing per-agent and never upgrades a row. Only its
//! DEATH is evidence: every agent of that session is over.

use std::path::PathBuf;
use std::sync::OnceLock;

use regex::Regex;

use super::worktrees::CoreWorktree;
use crate::session_detector::{PidObservation, ProcessProbe};

/// The owner a worktree lock reason names: `… (pid N start T)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LockOwner {
    /// The owning session's pid; always greater than 1.
    pub pid: u32,
    /// The owner's start time, when the reason carries one in procfs
    /// `starttime` units (all digits). A macOS `ps -o lstart=` date, or any
    /// other unit, is `None`: not comparable.
    pub start: Option<u64>,
}

/// Parse the owner out of a lock reason.
///
/// Claude Code (2.1.283) writes `claude agent agent-<id> (pid N start T)`, or
/// `(pid N)` with no start. The match is on shape only, anchored at the end of
/// the trimmed reason, and runtime-agnostic like D-A06's branch parse. A pid
/// of 0 or 1, one over `u32`, a missing close paren or trailing text after it
/// is `None`. The value is compared, never displayed, logged or spawned with.
pub fn parse_lock_owner(reason: &str) -> Option<LockOwner> {
    static OWNER: OnceLock<Regex> = OnceLock::new();
    let owner = OWNER.get_or_init(|| {
        Regex::new(r"\(pid ([0-9]{1,10})(?: start ([^()]{1,64}))?\)$")
            .expect("the lock-owner pattern is valid")
    });
    let caps = owner.captures(reason.trim())?;
    let pid: u32 = caps.get(1)?.as_str().parse().ok()?;
    if pid <= 1 {
        return None;
    }
    let start = caps
        .get(2)
        .map(|start| start.as_str())
        .filter(|start| {
            !start.is_empty() && start.len() <= 20 && start.bytes().all(|b| b.is_ascii_digit())
        })
        .and_then(|start| start.parse::<u64>().ok());
    Some(LockOwner { pid, start })
}

/// The owner of `wt`'s lock, when it is locked with a parseable reason.
pub(crate) fn lock_owner(wt: &CoreWorktree) -> Option<LockOwner> {
    match &wt.locked {
        // A parse is a comparison: question 1 of `src/text.rs`'s rule.
        Some(Some(reason)) => parse_lock_owner(reason.as_raw_for_logic_only()),
        _ => None,
    }
}

/// What the process table says about one agent worktree.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ProcessEvidence {
    /// Nothing was established: no probe, no parseable owner, or the probe
    /// could not say. Liveness is decided exactly as without a probe.
    #[default]
    Unknown,
    /// The lock owner is alive. Per D-A05 that says nothing per-agent, so it
    /// never changes the verdict either.
    OwnerAlive,
    /// The lock owner is gone — absent, replaced by another process under the
    /// same pid, or (with no comparable start) working outside the project.
    /// Every agent of that session is over: `Ended`.
    OwnerGone,
    /// A runtime process (`codex`) is working inside this worktree right now:
    /// `Live`, whatever else is known.
    RuntimeInside,
}

/// The verdict on a lock owner from what the probe observed of its pid.
///
/// `Unknown` → `Unknown`; `Absent` → `OwnerGone`. When the pid exists and both
/// start times are known, equal is `OwnerAlive` (exact identity, cwd
/// irrelevant) and different is `OwnerGone` (pid reuse). Otherwise the cwd
/// decides, component-wise: inside any of `roots` is `OwnerAlive`, outside all
/// is `OwnerGone`, unreadable is `Unknown`.
pub fn owner_verdict(
    obs: &PidObservation,
    lock_start: Option<u64>,
    roots: &[PathBuf],
) -> ProcessEvidence {
    match obs {
        PidObservation::Unknown => ProcessEvidence::Unknown,
        PidObservation::Absent => ProcessEvidence::OwnerGone,
        PidObservation::Present { start_time, cwd } => match (lock_start, start_time) {
            (Some(lock), Some(observed)) if lock == *observed => ProcessEvidence::OwnerAlive,
            (Some(_), Some(_)) => ProcessEvidence::OwnerGone,
            _ => match cwd {
                None => ProcessEvidence::Unknown,
                Some(cwd) if roots.iter().any(|root| cwd.starts_with(root)) => {
                    ProcessEvidence::OwnerAlive
                }
                Some(_) => ProcessEvidence::OwnerGone,
            },
        },
    }
}

/// The process evidence for one agent worktree.
///
/// `roots` are the project root (as given and canonical), the main worktree
/// and every worktree of the project: where a live owner may legitimately
/// work.
pub(crate) fn worktree_evidence(
    wt: &CoreWorktree,
    probe: &dyn ProcessProbe,
    roots: &[PathBuf],
) -> ProcessEvidence {
    let Some(owner) = lock_owner(wt) else {
        return ProcessEvidence::Unknown;
    };
    owner_verdict(&probe.observe(owner.pid), owner.start, roots)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::text::Untrusted;
    use std::collections::HashMap;

    /// A process table injected as data: no process, no procfs.
    #[derive(Default)]
    struct FakeProbe {
        pids: HashMap<u32, PidObservation>,
        codex: Option<Vec<PathBuf>>,
    }

    impl ProcessProbe for FakeProbe {
        fn observe(&self, pid: u32) -> PidObservation {
            self.pids
                .get(&pid)
                .cloned()
                .unwrap_or(PidObservation::Absent)
        }

        fn codex_cwds(&self) -> Option<&[PathBuf]> {
            self.codex.as_deref()
        }
    }

    fn owner(reason: &str) -> Option<(u32, Option<u64>)> {
        parse_lock_owner(reason).map(|o| (o.pid, o.start))
    }

    fn present(start: Option<u64>, cwd: Option<&str>) -> PidObservation {
        PidObservation::Present {
            start_time: start,
            cwd: cwd.map(PathBuf::from),
        }
    }

    fn roots() -> Vec<PathBuf> {
        vec![
            PathBuf::from("/repo"),
            PathBuf::from("/repo/.claude/worktrees/agent-a1"),
        ]
    }

    fn worktree(path: &str, locked: Option<Option<&str>>) -> CoreWorktree {
        CoreWorktree {
            path: PathBuf::from(path),
            locked: locked
                .map(|reason| reason.map(|r| Untrusted::from_untrusted_source(r.to_string()))),
            agent_pattern: true,
            ..CoreWorktree::default()
        }
    }

    #[test]
    fn the_claude_lock_reason_forms_parse() {
        assert_eq!(
            owner("claude agent agent-ab566f94e8a09dc6f (pid 29572 start 32844)"),
            Some((29572, Some(32844)))
        );
        assert_eq!(
            owner("claude agent agent-x (pid 29572)"),
            Some((29572, None))
        );
        assert_eq!(
            owner("claude agent agent-x (pid 29572 start 32844)\n"),
            Some((29572, Some(32844))),
            "trimmed before the anchor"
        );
        assert_eq!(
            owner("claude agent agent-x (pid 29572 start Sat Sep 26 00:09:07 2026)"),
            Some((29572, None)),
            "a macOS lstart date is not comparable"
        );
        assert_eq!(
            owner("x (pid 29572 start 123456789012345678901)"),
            Some((29572, None)),
            "21 digits is not a procfs starttime"
        );
    }

    #[test]
    fn malformed_lock_reasons_have_no_owner() {
        for reason in [
            "claude agent agent-x (pid 0 start 5)",
            "claude agent agent-x (pid 1)",
            "claude agent agent-x (pid 4294967296)",
            "claude agent agent-x (pid 12345678901)",
            "claude agent agent-x (pid 29572 start 32844",
            "claude agent agent-x (pid 29572 start 32844) trailing",
            "claude agent agent-x (pid -5)",
            "claude agent agent-x (pid )",
            "manual lock",
            "",
        ] {
            assert_eq!(owner(reason), None, "{reason:?}");
        }
        assert_eq!(owner("(pid 4294967295)"), Some((u32::MAX, None)));
    }

    #[test]
    fn only_a_locked_worktree_with_a_reason_has_an_owner() {
        assert_eq!(lock_owner(&worktree("/repo/wt", None)), None, "unlocked");
        assert_eq!(
            lock_owner(&worktree("/repo/wt", Some(None))),
            None,
            "locked without a reason"
        );
        assert_eq!(
            lock_owner(&worktree(
                "/repo/wt",
                Some(Some("claude agent a (pid 42 start 7)"))
            )),
            Some(LockOwner {
                pid: 42,
                start: Some(7)
            })
        );
    }

    #[test]
    fn owner_verdict_rules() {
        use ProcessEvidence::*;
        let r = roots();
        assert_eq!(
            owner_verdict(&PidObservation::Unknown, Some(1), &r),
            Unknown
        );
        assert_eq!(
            owner_verdict(&PidObservation::Absent, Some(1), &r),
            OwnerGone
        );
        assert_eq!(owner_verdict(&PidObservation::Absent, None, &r), OwnerGone);
        assert_eq!(
            owner_verdict(&present(Some(7), Some("/elsewhere")), Some(7), &r),
            OwnerAlive,
            "equal starts are exact identity; the cwd is irrelevant"
        );
        assert_eq!(
            owner_verdict(&present(Some(8), Some("/repo")), Some(7), &r),
            OwnerGone,
            "pid reuse"
        );
        // No comparable start: the cwd decides.
        for cwd in [
            "/repo",
            "/repo/src/deep",
            "/repo/.claude/worktrees/agent-a1",
        ] {
            assert_eq!(
                owner_verdict(&present(Some(8), Some(cwd)), None, &r),
                OwnerAlive,
                "{cwd}"
            );
            assert_eq!(
                owner_verdict(&present(None, Some(cwd)), Some(7), &r),
                OwnerAlive,
                "{cwd}"
            );
        }
        assert_eq!(
            owner_verdict(&present(None, Some("/repo2")), None, &r),
            OwnerGone,
            "a sibling prefix is outside, component-wise"
        );
        assert_eq!(
            owner_verdict(&present(None, Some("/home/me")), None, &r),
            OwnerGone
        );
        assert_eq!(
            owner_verdict(&present(None, None), None, &r),
            Unknown,
            "an unreadable cwd is never a death"
        );
        assert_eq!(
            owner_verdict(&present(None, Some("/repo")), None, &[]),
            OwnerGone,
            "no roots: nothing is inside"
        );
    }

    #[test]
    fn worktree_evidence_follows_the_lock_owner() {
        use ProcessEvidence::*;
        let r = roots();
        let locked =
            |reason: &str| worktree("/repo/.claude/worktrees/agent-a1", Some(Some(reason)));
        let mut probe = FakeProbe::default();
        probe.pids.insert(42, present(Some(7), Some("/repo")));

        assert_eq!(
            worktree_evidence(&locked("claude agent a (pid 42 start 7)"), &probe, &r),
            OwnerAlive
        );
        assert_eq!(
            worktree_evidence(&locked("claude agent a (pid 42 start 9)"), &probe, &r),
            OwnerGone
        );
        assert_eq!(
            worktree_evidence(&locked("claude agent a (pid 43 start 7)"), &probe, &r),
            OwnerGone,
            "the fake table does not hold 43"
        );
        assert_eq!(
            worktree_evidence(&worktree("/repo/wt", None), &probe, &r),
            Unknown,
            "no lock, no owner"
        );
        assert_eq!(
            worktree_evidence(&locked("manual"), &probe, &r),
            Unknown,
            "no parseable owner"
        );
        assert_eq!(
            worktree_evidence(
                &locked("claude agent a (pid 43 start 7)"),
                &crate::session_detector::NoProcessProbe,
                &r
            ),
            Unknown,
            "no probe, no verdict"
        );
    }
}
