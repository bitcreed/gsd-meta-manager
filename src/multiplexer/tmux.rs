//! The tmux backend (MUX-03): detection on `$TMUX`, focus by exact pane tty,
//! and a new window through `tmux new-window`. Moved from `terminal_switch`
//! with behaviour unchanged.
//!
//! Focus is modeled on `claudectl`'s tmux switcher: list every pane on the
//! running tmux server, find the one whose `#{pane_tty}`, with its leading
//! "/dev/" stripped, EQUALS the session's TTY (exact equality, never a
//! substring match), then `select-window` + `select-pane` onto that target.

use super::{
    Detection, EnvSnapshot, ExecArgv, FocusOutcome, MatchedBy, MultiplexerBackend, MuxError,
};
use crate::session_detector::ClaudeSession;
use std::path::Path;
use std::process::Command;

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
/// unrelated session. An empty session TTY matches nothing (T-LR8-01).
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

/// The tmux backend.
pub struct TmuxBackend;

/// The registry entry for tmux.
pub static TMUX: TmuxBackend = TmuxBackend;

impl MultiplexerBackend for TmuxBackend {
    fn name(&self) -> &'static str {
        "tmux"
    }

    /// Inside when `$TMUX` is present with ANY value, exactly the old
    /// `in_tmux()`: a set-but-empty value counts, so behaviour is unchanged
    /// (SC2) [inferred]. tmux is reachable whenever it is inside.
    fn detect(&self, env: &EnvSnapshot) -> Option<Detection> {
        env.get("TMUX").map(|_| Detection {
            inside: true,
            reachable: Ok(()),
        })
    }

    /// Switch the focused tmux pane to the given Claude session. Detection
    /// already gated on `$TMUX`, so there is no `in_tmux` check here. Every
    /// reason string is byte-identical to the one `switch_to_session` produced.
    fn focus(&self, _env: &EnvSnapshot, session: &ClaudeSession) -> Result<FocusOutcome, MuxError> {
        let tty = session
            .tty
            .as_ref()
            .ok_or_else(|| MuxError::NotFound("Session has no TTY recorded".to_string()))?;

        let output = Command::new("tmux")
            .args([
                "list-panes",
                "-a",
                "-F",
                "#{pane_tty} #{session_name}:#{window_index}.#{pane_index}",
            ])
            .output()
            .map_err(|e| MuxError::Failed(format!("tmux list-panes failed: {e}")))?;

        if !output.status.success() {
            return Err(MuxError::Failed(format!(
                "tmux list-panes exited {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            )));
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
            return Ok(FocusOutcome {
                by: MatchedBy::Tty,
                partial: None,
            });
        }

        Err(MuxError::NotFound(format!(
            "TTY {} not found in any tmux pane (session may have exited)",
            tty
        )))
    }

    /// Open `argv` as a new window on the running tmux server, in `cwd`.
    ///
    /// The fail-closed direct-exec refusals are on [`ExecArgv`], so by the time
    /// this runs the vector is already known to be exec-direct. The working
    /// directory travels as the `-c` **argv element** below, never as a `cd`
    /// written into a program string (T-W0D-03), the tmux-side mirror of the
    /// GUI path's `current_dir`.
    fn open(&self, _env: &EnvSnapshot, cwd: &Path, argv: &ExecArgv) -> Result<(), MuxError> {
        // No `--` before the program vector: the first element is a non-option
        // word, which is where tmux's parser already stops. No `-d` either — the
        // user is inside tmux and wants to land in the new window.
        let output = Command::new("tmux")
            .arg("new-window")
            .arg("-c")
            .arg(cwd)
            .args(argv.as_slice())
            .output()
            .map_err(|e| MuxError::Failed(format!("tmux new-window failed: {e}")))?;

        if !output.status.success() {
            return Err(MuxError::Failed(format!(
                "tmux new-window exited {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        Ok(())
    }
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

    /// **The direct-exec boundary against a LIVE tmux**, on a private socket.
    ///
    /// `#[ignore]`d because CI has no tmux. It exists so the measurement that
    /// D-03 rests on is RE-RUNNABLE rather than a fact recorded once in a plan:
    /// a two-element vector whose second element carries a `;` and a `$HOME`
    /// must arrive literally, with no word splitting, no expansion and no
    /// execution.
    ///
    /// Run with: `cargo test --lib multiplexer::tmux -- --ignored --nocapture`
    #[test]
    #[ignore = "requires a live tmux server; CI has none"]
    fn the_tmux_direct_exec_boundary_holds_against_a_live_server() {
        let socket = format!("gsd-mm-probe-{}", std::process::id());
        let payload = "--resume=--version;whoami $HOME";

        // Kill the server on EVERY exit path, including a panicking assert.
        struct Server(String);
        impl Drop for Server {
            fn drop(&mut self) {
                let _ = Command::new("tmux")
                    .args(["-L", &self.0, "kill-server"])
                    .output();
            }
        }

        let started = Command::new("tmux")
            .args([
                "-L",
                &socket,
                "new-session",
                "-d",
                "-s",
                "probe",
                "-x",
                "200",
                "-y",
                "50",
            ])
            .output();
        let Ok(started) = started else {
            eprintln!("tmux not runnable; skipping");
            return;
        };
        assert!(started.status.success(), "could not start the probe server");
        let _guard = Server(socket.clone());

        // The verdict is read off the FILESYSTEM rather than off the pane: a
        // pane whose program has exited is reaped, and capture-pane's timing is
        // not a property this test is trying to measure. `touch` creating a
        // file whose name IS the payload is a deterministic, race-free witness
        // that the bytes arrived as one argv element.
        let probe_dir = std::env::temp_dir().join(format!("gsd-mm-probe-{}", std::process::id()));
        std::fs::create_dir_all(&probe_dir).expect("probe dir");
        let expected = probe_dir.join(payload);

        // The vector under test: >= 2 elements, head is a literal that does not
        // begin with `-`.
        let program: Vec<String> = vec![
            "env".to_string(),
            "touch".to_string(),
            "--".to_string(),
            expected.to_string_lossy().into_owned(),
        ];
        let mut cmd = Command::new("tmux");
        cmd.args(["-L", &socket, "new-window", "-d", "-t", "probe:", "-c"])
            .arg(&probe_dir)
            .args(&program);
        let run = cmd.output().expect("new-window runs");
        assert!(
            run.status.success(),
            "new-window failed: {}",
            String::from_utf8_lossy(&run.stderr)
        );

        let mut arrived = false;
        for _ in 0..50 {
            if expected.exists() {
                arrived = true;
                break;
            }
            std::thread::sleep(std::time::Duration::from_millis(40));
        }
        let siblings: Vec<String> = std::fs::read_dir(&probe_dir)
            .map(|entries| {
                entries
                    .filter_map(Result::ok)
                    .map(|e| e.file_name().to_string_lossy().into_owned())
                    .collect()
            })
            .unwrap_or_default();
        let _ = std::fs::remove_dir_all(&probe_dir);

        assert!(
            arrived,
            "the payload did not arrive LITERALLY: no file named {payload:?} \
             was created. tmux either split it on the space, expanded $HOME, or \
             ran the `;` — all three mean an interpreter got into the path. \
             What the directory does contain: {siblings:?}"
        );
    }
}
