//! The GUI-terminal fallback chain (D-02, D-10): which emulator to open when no
//! multiplexer applies or the innermost one failed, and the spawn itself.

use std::path::Path;
use std::process::Command;

/// The four facts the GUI choice is made from, gathered at the impure edge by
/// [`probe_terminals`] so that [`rank_gui`] can be a pure function over them.
///
/// Whether the user is inside a multiplexer is NOT a field here: that is the
/// backends' answer (`MultiplexerBackend::detect`), and it outranks all of
/// these (D-10).
pub struct TerminalProbes {
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

/// Rank the GUI terminals. **Pure**: no I/O, no environment reads, no spawning.
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
pub fn rank_gui(probes: &TerminalProbes) -> Option<String> {
    probes
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
        .or_else(|| probes.fallback_present.first().cloned())
}

/// Gather the facts [`rank_gui`] decides from. **The thin impure edge**: it
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

/// Spawn the GUI terminal `term` with `args`, working directory `cwd`.
///
/// **ARGV ELEMENTS, every one of them** — the kernel hands them to `execve`
/// unparsed; they are NOT program fragments. The earlier construction handed a
/// program string to an interpreter through `-c`, so a quote in a session id or
/// a registered project path ran arbitrary code (CR-01). The INTERPRETER is
/// gone; a parser is not: `claude`'s own option parser reads this argv, and its
/// resume option takes an OPTIONAL value, so the id travels FUSED as one
/// element which binds it as that option's value instead of letting a leading
/// `-` make it an option of its own (CWE-88). See the resume argv builder's doc
/// in `ui::screens::detail` for the measurement that chose fusion over a `--`
/// separator.
///
/// The working directory travels through [`Command::current_dir`], never as a
/// `cd` written into a program string (T-W0D-03).
pub fn spawn_terminal(term: &str, args: &[String], cwd: &Path) -> std::io::Result<()> {
    Command::new(term)
        .args(args)
        .current_dir(cwd)
        .spawn()
        .map(|_| ())
}
