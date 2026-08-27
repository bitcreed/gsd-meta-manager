use std::path::PathBuf;
use std::process::Command;

use crate::text::Untrusted;

/// One live `claude` process, as seen through `/proc`.
///
/// **`session_id` is [`Untrusted`]** (D-21-19). It is scraped verbatim out of
/// another process's `--resume` argument in [`read_session_id`], so nothing
/// about it was authored by this build and nothing constrains it to ASCII —
/// which is exactly why the Sessions tab's `sid[..8]` byte slice could panic
/// the whole TUI (T-21-25-05).
///
/// `tty` is deliberately NOT retyped: it is compared against tmux's
/// `#{pane_tty}` and is a `/dev/` path component the kernel produced, not
/// something read out of a repository.
#[derive(Debug, Clone)]
pub struct ClaudeSession {
    pub pid: u32,
    pub session_id: Option<Untrusted>,
    pub working_dir: PathBuf,
    pub start_time: Option<u64>,
    /// Controlling TTY in tmux-friendly form (e.g. "pts/3"). The leading
    /// "/dev/" is stripped so a `contains()` match against tmux's
    /// `#{pane_tty}` (which prints "/dev/pts/3") still hits.
    pub tty: Option<String>,
}

/// Detect active Claude Code sessions by inspecting the Linux /proc filesystem.
///
/// Uses `pgrep -x claude` to find PIDs, then reads /proc entries for each.
/// Silently skips any PID where reads fail (stale/exited processes).
/// Uses std::process::Command (not tokio) — called from spawn_blocking.
pub fn detect_sessions() -> Vec<ClaudeSession> {
    let pids = match get_claude_pids() {
        Some(pids) => pids,
        None => return Vec::new(),
    };

    pids.into_iter().filter_map(build_session).collect()
}

fn get_claude_pids() -> Option<Vec<u32>> {
    let output = Command::new("pgrep").args(["-x", "claude"]).output().ok()?;

    if !output.status.success() {
        return Some(Vec::new());
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let pids: Vec<u32> = stdout
        .lines()
        .filter_map(|line| line.trim().parse::<u32>().ok())
        .collect();

    Some(pids)
}

fn build_session(pid: u32) -> Option<ClaudeSession> {
    let proc_path = PathBuf::from(format!("/proc/{}", pid));

    // Read working directory from /proc/PID/cwd symlink
    let working_dir = std::fs::read_link(proc_path.join("cwd")).ok()?;

    // Read session_id from /proc/PID/cmdline (null-byte separated)
    let session_id = read_session_id(pid);

    // Read start_time from /proc/PID/stat field 22
    let start_time = read_start_time(pid);

    // Read TTY from /proc/PID/fd/0 — the controlling terminal symlinks
    // here for any interactive Claude session.
    let tty = read_tty(pid);

    Some(ClaudeSession {
        pid,
        session_id,
        working_dir,
        start_time,
        tty,
    })
}

fn read_tty(pid: u32) -> Option<String> {
    let link = std::fs::read_link(format!("/proc/{}/fd/0", pid)).ok()?;
    let s = link.to_string_lossy();
    // /dev/pts/3 → pts/3 ; /dev/tty1 → tty1 ; anything else passes through.
    let stripped = s.strip_prefix("/dev/").unwrap_or(&s);
    if stripped.is_empty() {
        None
    } else {
        Some(stripped.to_string())
    }
}

fn read_session_id(pid: u32) -> Option<Untrusted> {
    let cmdline = std::fs::read(format!("/proc/{}/cmdline", pid)).ok()?;
    let args: Vec<&[u8]> = cmdline.split(|&b| b == 0).collect();

    for window in args.windows(2) {
        if window[0] == b"--resume" {
            let val = String::from_utf8_lossy(window[1]);
            let val = val.trim();
            if !val.is_empty() {
                // The ONE place a session id enters this build, so the ONE
                // place it is wrapped.
                return Some(Untrusted::from_untrusted_source(val.to_string()));
            }
        }
    }

    None
}

fn read_start_time(pid: u32) -> Option<u64> {
    let stat = std::fs::read_to_string(format!("/proc/{}/stat", pid)).ok()?;
    // /proc/PID/stat fields are space-separated, but field 2 (comm) can contain spaces
    // and is enclosed in parentheses. Find the closing paren, then count from there.
    let after_comm = stat.find(')')?.checked_add(2)?;
    let rest = stat.get(after_comm..)?;
    // Field 22 (starttime) is at index 19 after the comm field (0-indexed from field 3)
    let field = rest.split_whitespace().nth(19)?;
    field.parse::<u64>().ok()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_sessions_returns_vec() {
        // Should not panic, returns empty vec if no claude processes
        let sessions = detect_sessions();
        // We can't assert specific content in CI, but it should not panic
        let _ = sessions;
    }

    #[test]
    fn test_read_session_id_nonexistent_pid() {
        // Should return None for non-existent PID
        assert!(read_session_id(999_999_999).is_none());
    }

    #[test]
    fn test_read_start_time_nonexistent_pid() {
        assert!(read_start_time(999_999_999).is_none());
    }
}
