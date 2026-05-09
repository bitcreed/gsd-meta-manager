//! Switch the user's terminal focus to a running Claude session.
//!
//! Tmux-first implementation modeled on `claudectl`'s tmux switcher:
//! we list every pane on the running tmux server, find the one whose
//! `#{pane_tty}` matches our session's TTY, then `select-window` +
//! `select-pane` onto that target.
//!
//! Outside tmux this is a no-op that returns a clear error so callers
//! can surface it via the status line — we don't yet wire up Kitty,
//! Ghostty, WezTerm, etc. (See `claudectl/src/terminals/*.rs` for the
//! reference implementations when we want to extend.)

use crate::session_detector::ClaudeSession;
use std::process::Command;

/// Switch the focused terminal to the given Claude session. Returns
/// Ok on success or a human-readable error string suitable for the
/// status bar.
pub fn switch_to_session(session: &ClaudeSession) -> Result<(), String> {
    if std::env::var_os("TMUX").is_none() {
        return Err(
            "Tab-switching requires running inside tmux (no $TMUX set)".to_string(),
        );
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
    for line in stdout.lines() {
        let mut parts = line.splitn(2, ' ');
        let pane_tty = parts.next().unwrap_or("");
        let target = match parts.next() {
            Some(t) => t,
            None => continue,
        };
        if pane_tty.contains(tty) {
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
    }

    Err(format!(
        "TTY {} not found in any tmux pane (session may have exited)",
        tty
    ))
}
