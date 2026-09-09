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
use std::path::Path;
use std::process::Command;

/// Are we running inside a tmux client?
///
/// **The ONLY thing factored between focus-switching and launching** (D-06).
/// [`switch_to_session`] and the launch path share exactly this one question;
/// their `tmux list-panes` and `tmux new-window` invocations share nothing but
/// the binary name, so they are deliberately NOT merged behind a common runner.
pub fn in_tmux() -> bool {
    std::env::var_os("TMUX").is_some()
}

/// The five facts the launch decision is made from, gathered at the impure
/// edge by [`probe_terminals`] so that [`plan_launch`] can be a pure function
/// over them.
pub struct TerminalProbes {
    /// `$TMUX` is set — see [`in_tmux`].
    pub in_tmux: bool,
    /// `$TERMINAL`, already emptiness-filtered by the collector.
    pub terminal_env: Option<String>,
    /// `xdg-terminal-exec --print-id` resolved a terminal AND exited 0.
    pub xdg_terminal_exec_ok: bool,
    /// `x-terminal-emulator` exists on `PATH`.
    pub x_terminal_emulator_present: bool,
    /// The subset of the caller's fallback candidate list that exists, in the
    /// caller's order.
    pub fallback_present: Vec<String>,
}

/// What to do about a launch request.
///
/// **It carries BOTH answers on purpose.** The launch order is "the tmux
/// server, and on failure a GUI terminal", so a caller that gets an `Err` back
/// from tmux must be able to fall through without re-deriving anything — and
/// re-deriving is exactly where a second, differently-ordered copy of the
/// decision would grow. Computing the GUI answer unconditionally is also what
/// makes the whole order testable as a pure function: a plan can be asserted
/// for an environment nobody is running in.
pub struct LaunchPlan {
    /// Try the running tmux server first.
    pub try_tmux: bool,
    /// The GUI terminal to use — either because tmux was not applicable, or
    /// because it was tried and failed.
    pub gui: Option<String>,
}

/// Decide where a Claude session should be opened. **Pure**: no I/O, no
/// environment reads, no spawning.
///
/// # D-01 — tmux OUTRANKS `$TERMINAL`
///
/// `$TERMINAL` expresses which *GUI emulator* the user prefers, not *where the
/// session should appear*. It is commonly exported globally from a shell
/// profile, so it is rarely a per-invocation intent. If it outranked tmux,
/// every user with `$TERMINAL` set would keep hitting exactly the reported bug:
/// a GUI window opening while they sit inside a tmux session.
///
/// **Residual, stated rather than fixed:** a user sitting inside tmux can no
/// longer force a GUI window by setting `$TERMINAL`. No new environment
/// variable and no config key is added to recover that. `$TERMINAL` is not
/// lost — the tmux branch falls through to it whenever `$TMUX` is unset, no
/// tmux server is reachable, or the `tmux new-window` call fails.
///
/// # D-02 — the GUI order, with the reason each rank is where it is
///
/// 1. `$TERMINAL`, when non-empty.
/// 2. `xdg-terminal-exec` — the reference implementation of the freedesktop
///    Default Terminal Specification, i.e. the thing that actually knows which
///    terminal the desktop designates as default.
/// 3. `x-terminal-emulator` (Debian alternatives). **This ordering is
///    load-bearing:** on the machine the bug was reported from,
///    `/usr/bin/x-terminal-emulator` points at `gnome-terminal.wrapper` — i.e.
///    at exactly the wrong answer. Probed first, the reported bug would
///    persist.
/// 4. The caller's hardcoded fallback list, first present entry in list order.
///    Order-dependence is precisely how the reported bug happened, so this rank
///    is last.
pub fn plan_launch(probes: &TerminalProbes) -> LaunchPlan {
    let gui = probes
        .terminal_env
        .as_ref()
        .filter(|term| !term.is_empty())
        .cloned()
        .or_else(|| {
            probes
                .xdg_terminal_exec_ok
                .then(|| "xdg-terminal-exec".to_string())
        })
        .or_else(|| {
            probes
                .x_terminal_emulator_present
                .then(|| "x-terminal-emulator".to_string())
        })
        .or_else(|| probes.fallback_present.first().cloned());

    LaunchPlan {
        try_tmux: probes.in_tmux,
        gui,
    }
}

/// Gather the facts [`plan_launch`] decides from. **The thin impure edge**: it
/// holds no ordering of its own, which is why the order is testable without it.
///
/// `xdg-terminal-exec --print-id` is BOTH the existence check and the
/// capability check: a missing binary makes `.output()` an `Err`, and a present
/// binary that cannot resolve a terminal exits non-zero. Per D-02 `--print-id`
/// resolves the desktop's default-terminal association WITHOUT spawning a
/// terminal, which is what lets the launcher fall through — a bare `spawn()`
/// fails asynchronously, after which there is nothing left to fall through to.
/// It is run with no untrusted argument (T-W0D-06).
pub fn probe_terminals(fallback_candidates: &[&str]) -> TerminalProbes {
    let on_path = |program: &str| -> bool {
        Command::new("which")
            .arg(program)
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false)
    };

    TerminalProbes {
        in_tmux: in_tmux(),
        terminal_env: std::env::var("TERMINAL")
            .ok()
            .filter(|term| !term.is_empty()),
        xdg_terminal_exec_ok: Command::new("xdg-terminal-exec")
            .arg("--print-id")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false),
        x_terminal_emulator_present: on_path("x-terminal-emulator"),
        fallback_present: fallback_candidates
            .iter()
            .filter(|candidate| on_path(candidate))
            .map(|candidate| (*candidate).to_string())
            .collect(),
    }
}

/// Open `program_argv` as a new window on the running tmux server, in `cwd`.
///
/// # The fail-closed direct-exec boundary (D-03, T-W0D-01, T-W0D-04)
///
/// `man tmux`, verbatim: *"the new-window ... and respawn-pane commands allow
/// shell-command to be given as multiple arguments and executed directly
/// (without 'sh -c'). This can avoid issues with shell quoting."* A
/// shell-command given as a **single** argument still goes through `sh -c`.
///
/// So the arity of this vector decides whether a command interpreter is in the
/// path — and an interpreter in this path is precisely the defect CR-01 /
/// T-21-27-01 / T-21-27-02 closed at the GUI spawn sites. **This is the
/// boundary at which that fix could silently regress**, so the check lives
/// here, on the sink every future caller must pass through, rather than on each
/// builder where a new builder could simply not inherit it. Both refusals
/// happen BEFORE any [`std::process::Command`] is constructed, which is also
/// what lets them be tested on a machine with no tmux at all.
///
/// The in-tree builders make `len() >= 2` true *by construction* by beginning
/// every vector with the literal `env`, so this refusal is unreachable from
/// them — that is the intent. `env` is not a command interpreter: it has no
/// program-string mode, it consumes leading `NAME=VALUE`/option words and then
/// `execvp`s the first non-option word with the remainder passed through
/// **unparsed**, so no byte of a fused `--resume=<id>` element can be
/// reinterpreted by it.
///
/// A head beginning with `-` is refused for a second, independent reason:
/// tmux's own option parser would consume it as one of ITS options rather than
/// treating it as the program.
///
/// The working directory travels as the `-c` **argv element** below, never as a
/// `cd` written into a program string (T-W0D-03) — the tmux-side mirror of the
/// GUI path's [`std::process::Command::current_dir`].
pub fn open_new_window(cwd: &Path, program_argv: &[String]) -> Result<(), String> {
    if program_argv.len() < 2 {
        return Err(format!(
            "refusing to open a tmux window from a {}-element program vector: \
             tmux execs a shell-command given as MULTIPLE arguments directly, \
             but hands a SINGLE argument to a command interpreter — which would \
             put a parser back in a path that deliberately has none",
            program_argv.len()
        ));
    }
    if program_argv[0].starts_with('-') {
        return Err(format!(
            "refusing to open a tmux window whose program vector begins with \
             {:?}: tmux's own option parser would consume it as one of its \
             options instead of treating it as the program",
            program_argv[0]
        ));
    }

    // No `--` before the program vector: the first element is a non-option
    // word, which is where tmux's parser already stops. No `-d` either — the
    // user is inside tmux and wants to land in the new window.
    let output = Command::new("tmux")
        .arg("new-window")
        .arg("-c")
        .arg(cwd)
        .args(program_argv)
        .output()
        .map_err(|e| format!("tmux new-window failed: {e}"))?;

    if !output.status.success() {
        return Err(format!(
            "tmux new-window exited {}: {}",
            output.status,
            String::from_utf8_lossy(&output.stderr).trim()
        ));
    }
    Ok(())
}

/// Switch the focused terminal to the given Claude session. Returns
/// Ok on success or a human-readable error string suitable for the
/// status bar.
pub fn switch_to_session(session: &ClaudeSession) -> Result<(), String> {
    if !in_tmux() {
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

#[cfg(test)]
mod tests {
    use super::*;

    /// Build a `TerminalProbes` from the five facts, so each row of the table
    /// below reads as the environment it describes rather than as a struct
    /// literal.
    fn probes(
        in_tmux: bool,
        terminal_env: Option<&str>,
        xdg_ok: bool,
        x_term: bool,
        fallback: &[&str],
    ) -> TerminalProbes {
        TerminalProbes {
            in_tmux,
            terminal_env: terminal_env.map(str::to_string),
            xdg_terminal_exec_ok: xdg_ok,
            x_terminal_emulator_present: x_term,
            fallback_present: fallback.iter().map(|s| (*s).to_string()).collect(),
        }
    }

    /// **The whole launch order, as a pure function** (D-01, D-02).
    ///
    /// This table spawns nothing, reads no environment variable and does not
    /// depend on which terminals happen to be installed on the machine running
    /// it — which is the entire point of `plan_launch` taking its facts as an
    /// argument instead of reading them.
    #[test]
    fn plan_launch_ranks_tmux_first_and_then_the_discovered_default() {
        let rows: Vec<(&str, TerminalProbes, bool, Option<&str>)> = vec![
            (
                "inside tmux, tmux WINS over $TERMINAL — and the GUI answer is \
                 still computed so the caller can fall through without \
                 re-deriving the decision (D-01)",
                probes(true, Some("kitty"), true, true, &["gnome-terminal"]),
                true,
                Some("kitty"),
            ),
            (
                "outside tmux, $TERMINAL is the top GUI rank",
                probes(false, Some("kitty"), true, false, &[]),
                false,
                Some("kitty"),
            ),
            (
                "an EMPTY $TERMINAL is ignored, as it always was",
                probes(false, Some(""), true, false, &["gnome-terminal"]),
                false,
                Some("xdg-terminal-exec"),
            ),
            (
                "with no $TERMINAL, xdg-terminal-exec beats BOTH lower ranks \
                 (D-02)",
                probes(false, None, true, true, &["gnome-terminal"]),
                false,
                Some("xdg-terminal-exec"),
            ),
            (
                "x-terminal-emulator is the next rank once xdg cannot resolve",
                probes(false, None, false, true, &["gnome-terminal"]),
                false,
                Some("x-terminal-emulator"),
            ),
            (
                "the fallback list is last resort, first PRESENT entry in the \
                 caller's order",
                probes(false, None, false, false, &["gnome-terminal", "xterm"]),
                false,
                Some("gnome-terminal"),
            ),
            (
                "nothing present at all is None, not a guess",
                probes(false, None, false, false, &[]),
                false,
                None,
            ),
        ];

        for (label, probe, expected_tmux, expected_gui) in rows {
            let plan = plan_launch(&probe);
            assert_eq!(
                plan.try_tmux, expected_tmux,
                "try_tmux wrong for the case: {label}"
            );
            assert_eq!(
                plan.gui.as_deref(),
                expected_gui,
                "gui wrong for the case: {label}"
            );
        }
    }

    /// **The reported bug, pinned with the user's exact measured environment.**
    ///
    /// `$TERMINAL` unset, `$TMUX` set, `xdg-terminal-exec` resolving,
    /// `x-terminal-emulator` present, and `ptyxis`/`gnome-terminal`/`xterm` all
    /// installed. The TUI opened a **gnome-terminal** window while the user was
    /// sitting inside tmux. Nothing in this environment may produce a plan that
    /// declines to try tmux.
    #[test]
    fn a_gui_terminal_must_not_be_chosen_while_the_user_is_sitting_in_tmux() {
        let plan = plan_launch(&probes(
            true,
            None,
            true,
            true,
            &["ptyxis", "gnome-terminal", "xterm"],
        ));
        assert!(
            plan.try_tmux,
            "the plan declined to try tmux in the exact environment the bug was \
             reported from, so a GUI terminal ({:?}) would be spawned while the \
             user is sitting inside a tmux session. That is the reported defect \
             (D-01).",
            plan.gui
        );
    }

    /// **The fail-closed sink for the tmux direct-exec boundary** (D-03,
    /// T-W0D-01). Reached without a tmux server, because the refusals happen
    /// before any `std::process::Command` is constructed.
    #[test]
    fn open_new_window_refuses_a_vector_tmux_would_hand_to_an_interpreter() {
        let cwd = std::path::Path::new("/tmp");

        let one = vec!["claude".to_string()];
        let err = open_new_window(cwd, &one)
            .expect_err("a ONE-element program vector must be refused: tmux runs it under `sh -c`");
        assert!(
            err.contains("directly"),
            "the refusal must name the direct-exec boundary it is protecting; got {err:?}"
        );

        let none: Vec<String> = Vec::new();
        assert!(
            open_new_window(cwd, &none).is_err(),
            "an EMPTY program vector must be refused"
        );

        let dashed = vec!["-d".to_string(), "claude".to_string()];
        let err = open_new_window(cwd, &dashed)
            .expect_err("a program vector whose head starts with `-` must be refused");
        assert!(
            err.contains("option"),
            "the refusal must name tmux's own option parser as the reason; got {err:?}"
        );
    }

    /// **The direct-exec boundary against a LIVE tmux**, on a private socket.
    ///
    /// `#[ignore]`d because CI has no tmux. It exists so the measurement that
    /// D-03 rests on is RE-RUNNABLE rather than a fact recorded once in a plan:
    /// a two-element vector whose second element carries a `;` and a `$HOME`
    /// must arrive literally, with no word splitting, no expansion and no
    /// execution.
    ///
    /// Run with: `cargo test --lib terminal_switch -- --ignored --nocapture`
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
