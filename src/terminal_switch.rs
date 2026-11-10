//! Switch the user's terminal focus to a running Claude session.
//!
//! Tmux-first implementation modeled on `claudectl`'s tmux switcher:
//! we list every pane on the running tmux server, find the one whose
//! `#{pane_tty}`, with its leading "/dev/" stripped, EQUALS our session's
//! TTY (exact equality, never a substring match), then `select-window` +
//! `select-pane` onto that target.
//!
//! Outside tmux this is a no-op that returns a clear error so callers
//! can surface it via the status line — we don't yet wire up Kitty,
//! Ghostty, WezTerm, etc. (See `claudectl/src/terminals/*.rs` for the
//! reference implementations when we want to extend.)

use crate::session_detector::ClaudeSession;
use std::process::Command;

/// Are we running inside a tmux client?
///
/// Used by [`switch_to_session`] only; launching asks the backends in
/// [`crate::multiplexer`] instead.
pub fn in_tmux() -> bool {
    std::env::var_os("TMUX").is_some()
}
/// Remove exactly one leading `/dev/` from a TTY path, if present.
fn strip_dev_prefix(tty: &str) -> &str {
    tty.strip_prefix("/dev/").unwrap_or(tty)
}

/// Does a tmux pane's `#{pane_tty}` belong to the session with `session_tty`?
///
/// tmux prints `#{pane_tty}` as "/dev/pts/N"; the session detector stores
/// "pts/N". Both sides have one leading "/dev/" stripped and then must be
/// EXACTLY equal — no prefix, suffix or substring relation counts. A substring
/// test would let a session on "pts/1" focus the pane on "/dev/pts/12" (or
/// "/dev/pts/10", "/dev/pts/100"), sending the operator's keystrokes to an
/// unrelated session. An empty session TTY matches nothing.
fn pane_tty_matches(pane_tty: &str, session_tty: &str) -> bool {
    let session = strip_dev_prefix(session_tty);
    if session.is_empty() {
        return false;
    }
    strip_dev_prefix(pane_tty) == session
}

/// Pick the `session:window.pane` target of the first `tmux list-panes -a`
/// line (format `#{pane_tty} <target>`) whose pane TTY matches `session_tty`.
/// The target is the rest of the line after the first space, so a session
/// name containing spaces is kept whole. Lines without a target are skipped.
fn find_pane_target<'a>(list_panes_stdout: &'a str, session_tty: &str) -> Option<&'a str> {
    for line in list_panes_stdout.lines() {
        let mut parts = line.splitn(2, ' ');
        let pane_tty = parts.next().unwrap_or("");
        let target = match parts.next() {
            Some(t) => t,
            None => continue,
        };
        if pane_tty_matches(pane_tty, session_tty) {
            return Some(target);
        }
    }
    None
}

/// Switch the focused terminal to the given Claude session. Returns
/// Ok on success or a human-readable error string suitable for the
/// status bar.
pub fn switch_to_session(session: &ClaudeSession) -> Result<(), String> {
    if !in_tmux() {
        return Err("Tab-switching requires running inside tmux (no $TMUX set)".to_string());
    }

    let tty = session
        .tty
        .as_ref()
        .ok_or_else(|| "Session has no TTY recorded".to_string())?;

    let output = Command::new("tmux")
        .args([
            "list-panes",
            "-a",
            "-F",
            "#{pane_tty} #{session_name}:#{window_index}.#{pane_index}",
        ])
        .output()
        .map_err(|e| format!("tmux list-panes failed: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "tmux list-panes exited {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    if let Some(target) = find_pane_target(&stdout, tty) {
        // Best-effort: even if select-pane errors, select-window
        // is the primary effect the user notices.
        let _ = Command::new("tmux")
            .args(["select-window", "-t", target])
            .output();
        let _ = Command::new("tmux")
            .args(["select-pane", "-t", target])
            .output();
        return Ok(());
    }

    Err(format!(
        "TTY {} not found in any tmux pane (session may have exited)",
        tty
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    /// **The reported pane-misfocus bug, pinned** (T-LR8-01). tmux prints
    /// `#{pane_tty}` as "/dev/pts/N"; the detector stores "pts/N". A substring
    /// test lets a session on pts/1 match the pane on /dev/pts/12.
    #[test]
    fn pane_tty_match_is_exact_not_substring() {
        assert!(
            !pane_tty_matches("/dev/pts/12", "pts/1"),
            "session TTY pts/1 matched the pane on /dev/pts/12: Tab-switch would \
             focus an unrelated pane and send the operator's keystrokes there"
        );
        assert!(
            pane_tty_matches("/dev/pts/1", "pts/1"),
            "session TTY pts/1 did not match its own pane /dev/pts/1: Tab-switch \
             would report the session as gone instead of focusing it"
        );
    }

    /// **The end-to-end shape of the bug**: the longer TTY is listed FIRST, so
    /// the old first-substring-hit rule returned the wrong target.
    #[test]
    fn pane_tty_find_target_skips_a_longer_tty_listed_first() {
        assert_eq!(
            find_pane_target("/dev/pts/12 work:1.0\n/dev/pts/1 main:0.0\n", "pts/1"),
            Some("main:0.0"),
            "with /dev/pts/12 listed before /dev/pts/1, the session on pts/1 must \
             focus main:0.0; any other target misfocuses the operator's input"
        );
        assert_eq!(
            find_pane_target("/dev/pts/12 work:1.0\n", "pts/1"),
            None,
            "only /dev/pts/12 is listed, so pts/1 must find no pane (and reach the \
             not-found error) instead of focusing the unrelated work:1.0 pane"
        );
    }

    /// **The exact-match contract, row by row** (T-LR8-01, T-LR8-02). Only
    /// equality after one "/dev/" strip on each side counts.
    #[test]
    fn pane_tty_match_edge_cases() {
        let rows: Vec<(&str, &str, &str, bool)> = vec![
            ("tmux form vs detector form", "/dev/pts/1", "pts/1", true),
            ("a pane printed without /dev/", "pts/1", "pts/1", true),
            (
                "a session side carrying /dev/ is normalized",
                "/dev/pts/1",
                "/dev/pts/1",
                true,
            ),
            ("a non-pts console TTY", "/dev/tty1", "tty1", true),
            ("pts/10 is not pts/1", "/dev/pts/10", "pts/1", false),
            ("pts/100 is not pts/1", "/dev/pts/100", "pts/1", false),
            ("tty10 is not tty1", "/dev/tty10", "tty1", false),
            ("a suffix only is not a match", "/dev/pts/1", "ts/1", false),
            (
                "a longer session TTY is not a match",
                "/dev/pts/1",
                "pts/12",
                false,
            ),
            (
                "an empty session TTY matches nothing",
                "/dev/pts/1",
                "",
                false,
            ),
            ("empty against empty is not a match", "", "", false),
            (
                "a bare /dev/ on both sides is empty",
                "/dev/",
                "/dev/",
                false,
            ),
        ];

        for (label, pane_tty, session_tty, expected) in rows {
            assert_eq!(
                pane_tty_matches(pane_tty, session_tty),
                expected,
                "pane_tty_matches({pane_tty:?}, {session_tty:?}) wrong for the case: \
                 {label}. A false match focuses an unrelated pane"
            );
        }
    }

    /// **`list-panes` line parsing** around the exact-match predicate.
    #[test]
    fn pane_tty_find_target_parsing_edge_cases() {
        let rows: Vec<(&str, &str, &str, Option<&str>)> = vec![
            (
                "a target containing spaces is kept whole",
                "/dev/pts/1 my session:0.1\n",
                "pts/1",
                Some("my session:0.1"),
            ),
            (
                "a line without a target is skipped",
                "/dev/pts/1\n/dev/pts/1 main:0.0\n",
                "pts/1",
                Some("main:0.0"),
            ),
            ("empty list-panes output finds nothing", "", "pts/1", None),
            (
                "an empty session TTY focuses no pane",
                "/dev/pts/3 a:0.0\n/dev/pts/4 b:0.0\n",
                "",
                None,
            ),
        ];

        for (label, stdout, session_tty, expected) in rows {
            assert_eq!(
                find_pane_target(stdout, session_tty),
                expected,
                "find_pane_target wrong for the case: {label}"
            );
        }
    }
}
