//! The tmux backend (MUX-03): detection on `$TMUX`, and a new window through
//! `tmux new-window`. Moved from `terminal_switch` with behaviour unchanged.

use super::{Detection, EnvSnapshot, ExecArgv, MultiplexerBackend, MuxError};
use std::path::Path;
use std::process::Command;

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
