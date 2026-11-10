//! The terminal-multiplexer backend seam (Phase 33, D-01, D-02).
//!
//! Where a Claude session is focused or opened used to be hardcoded to tmux in
//! `terminal_switch`. This module replaces that with one object-safe trait,
//! [`MultiplexerBackend`], a static [`REGISTRY`] of backends, and a small set of
//! seam functions the UI calls instead of spawning anything itself:
//!
//! * [`launch`] opens a new or resumed session where the user is: the
//!   innermost detected backend, then the GUI terminal chain ([`gui`]).
//! * [`focus_message`] focuses an existing session: each detected backend is
//!   tried innermost-first and the first success wins.
//!
//! # Pure core, impure edges (D-04)
//!
//! Detection and ordering are pure functions over an injected [`EnvSnapshot`]
//! ([`order_innermost`], [`plan_launch`], [`launch_with`]). Commands and the
//! herdr control socket run only at each backend's own edge. No command-runner
//! abstraction is shared across backends: tmux's `list-panes` / `new-window` and
//! herdr's socket request share nothing but the question "are we inside it?".
//!
//! # One fail-closed argv sink (D-05)
//!
//! Every backend's [`MultiplexerBackend::open`] takes an [`ExecArgv`], and an
//! `ExecArgv` can only be built by [`ExecArgv::new`], which refuses a vector a
//! multiplexer would hand to a command interpreter. The refusal therefore holds
//! for every backend, present and future, rather than by a rule each one has to
//! remember.
//!
//! The longer design note (backend responsibilities, nesting precedence, the
//! herdr launch decision) is written alongside the herdr focus work in the
//! Phase 33 follow-up plan (D-14).

pub mod gui;
pub mod herdr;
pub mod tmux;

use crate::session_detector::ClaudeSession;
use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

/// Why a backend did not do what it was asked (D-03).
///
/// The four variants fall into three classes:
///
/// * **Not mine / not found** ([`MuxError::NotFound`], [`MuxError::Unsupported`]):
///   this backend has nothing to act on, so the caller tries the next one
///   ([`MuxError::is_try_next`]).
/// * **Refused for safety** ([`MuxError::RefusedForSafety`]): the request itself
///   would put a command interpreter in the path. It is never retried through a
///   path that adds one.
/// * **Failed** ([`MuxError::Failed`]): the backend tried and the attempt broke.
///
/// `Display` prints only the inner reason, so the reasons tmux already produced
/// stay byte-identical in status lines. The backend name is added by the caller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MuxError {
    /// The thing to act on does not exist here (no pane, no tty recorded).
    NotFound(String),
    /// This backend does not implement the operation.
    Unsupported(String),
    /// Refused before any process or socket was touched, for safety.
    RefusedForSafety(String),
    /// The backend ran and failed.
    Failed(String),
}

impl MuxError {
    /// The reason, without the variant.
    fn reason(&self) -> &str {
        match self {
            Self::NotFound(r)
            | Self::Unsupported(r)
            | Self::RefusedForSafety(r)
            | Self::Failed(r) => r,
        }
    }

    /// Should a caller move on to the next backend? True for the two "not
    /// mine / not found" classes.
    pub fn is_try_next(&self) -> bool {
        matches!(self, Self::NotFound(_) | Self::Unsupported(_))
    }
}

impl fmt::Display for MuxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.reason())
    }
}

impl std::error::Error for MuxError {}

/// A program vector a multiplexer may exec directly. **The only way to hand a
/// backend something to run.**
///
/// The private field, the absence of `Default` and the absence of any
/// `From<Vec<String>>` mean a value of this type exists only if
/// [`ExecArgv::new`] accepted it.
///
/// # The fail-closed direct-exec boundary (D-03, D-05, T-W0D-01, T-W0D-04)
///
/// `man tmux`, verbatim: *"the new-window ... and respawn-pane commands allow
/// shell-command to be given as multiple arguments and executed directly
/// (without 'sh -c'). This can avoid issues with shell quoting."* A
/// shell-command given as a **single** argument still goes through `sh -c`.
///
/// So the arity of this vector decides whether a command interpreter is in the
/// path, and an interpreter in this path is precisely the defect CR-01 /
/// T-21-27-01 / T-21-27-02 closed at the GUI spawn sites. **This is the
/// boundary at which that fix could silently regress**, so the check lives on
/// the type every future caller must pass through, rather than on each builder
/// where a new builder could simply not inherit it. Both refusals happen BEFORE
/// any process or socket is touched, which is also what lets them be tested on a
/// machine with no multiplexer at all.
///
/// The in-tree builders make `len() >= 2` true *by construction* by beginning
/// every vector with the literal `env`, so this refusal is unreachable from
/// them, and that is the intent. `env` is not a command interpreter: it has no
/// program-string mode, it consumes leading `NAME=VALUE`/option words and then
/// `execvp`s the first non-option word with the remainder passed through
/// **unparsed**, so no byte of a fused resume element can be reinterpreted by
/// it.
///
/// A head beginning with `-` is refused for a second, independent reason:
/// tmux's own option parser would consume it as one of ITS options rather than
/// treating it as the program.
///
/// The working directory travels outside this vector: as tmux's `-c` **argv
/// element**, as herdr's JSON `cwd` field, or through the GUI spawn's
/// `current_dir`; never as a `cd` written into a program string (T-W0D-03).
///
/// # Why the messages still name tmux [inferred]
///
/// The refusals used to live on tmux's `open_new_window` and now sit on a type
/// every backend's `open` must take (D-05). The message text deliberately keeps
/// naming "a tmux window": it is byte-identical for SC2, and the arity rule
/// originates in tmux. The substrings `directly` and `option` that the tests
/// pin are unchanged.
#[derive(Debug, Clone)]
pub struct ExecArgv(Vec<String>);

impl ExecArgv {
    /// Validate and wrap a program vector.
    pub fn new(argv: Vec<String>) -> Result<ExecArgv, MuxError> {
        if argv.len() < 2 {
            return Err(MuxError::RefusedForSafety(format!(
                "refusing to open a tmux window from a {}-element program vector: \
                 tmux execs a shell-command given as MULTIPLE arguments directly, \
                 but hands a SINGLE argument to a command interpreter — which would \
                 put a parser back in a path that deliberately has none",
                argv.len()
            )));
        }
        if argv[0].starts_with('-') {
            return Err(MuxError::RefusedForSafety(format!(
                "refusing to open a tmux window whose program vector begins with \
                 {:?}: tmux's own option parser would consume it as one of its \
                 options instead of treating it as the program",
                argv[0]
            )));
        }
        Ok(ExecArgv(argv))
    }

    /// The validated elements, in order.
    pub fn as_slice(&self) -> &[String] {
        &self.0
    }
}

/// The environment keys detection reads. A fixed list, so the snapshot is a
/// complete record of what the decisions below depend on.
pub const ENV_KEYS: &[&str] = &[
    "TMUX",
    "HERDR_ENV",
    "HERDR_SOCKET_PATH",
    "HERDR_WORKSPACE_ID",
];

/// The values of [`ENV_KEYS`] at one moment. A set-but-empty value is kept
/// distinct from an unset one.
///
/// [`EnvSnapshot::from_process`] is the only place detection reads
/// `std::env` (the impure edge). Everything after it is a pure function of the
/// snapshot, which is also what lets a fixture describe an environment nobody
/// is running in. Markers are compared as exact strings with no normalization.
#[derive(Debug, Clone, Default)]
pub struct EnvSnapshot {
    values: BTreeMap<String, String>,
}

impl EnvSnapshot {
    /// Read [`ENV_KEYS`] from the process environment.
    pub fn from_process() -> Self {
        let mut values = BTreeMap::new();
        for key in ENV_KEYS {
            if let Some(value) = std::env::var_os(key) {
                values.insert((*key).to_string(), value.to_string_lossy().into_owned());
            }
        }
        Self { values }
    }

    /// Build a fixture from `(key, value)` pairs.
    pub fn from_pairs(pairs: &[(&str, &str)]) -> Self {
        Self {
            values: pairs
                .iter()
                .map(|(k, v)| ((*k).to_string(), (*v).to_string()))
                .collect(),
        }
    }

    /// The value of `key`: `None` when unset, `Some("")` when set but empty.
    pub fn get(&self, key: &str) -> Option<&str> {
        self.values.get(key).map(String::as_str)
    }
}

/// What a backend says about the current environment. "Inside" and "reachable"
/// are separate states (D-02): the TUI can be inside herdr (the marker is set)
/// while its control socket is not reachable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Detection {
    /// The environment says the TUI runs inside this multiplexer.
    pub inside: bool,
    /// Whether the backend can be driven from here; `Err` carries the reason.
    pub reachable: Result<(), String>,
}

/// A backend that detected itself, with what it detected.
#[derive(Clone)]
pub struct Detected {
    pub backend: &'static dyn MultiplexerBackend,
    pub detection: Detection,
}

impl fmt::Debug for Detected {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Detected")
            .field("backend", &self.backend.name())
            .field("detection", &self.detection)
            .finish()
    }
}

/// How a focus request found its target.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MatchedBy {
    SessionId,
    Tty,
    Pid,
    Cwd,
}

/// A successful focus.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FocusOutcome {
    pub by: MatchedBy,
    /// The human note when focus landed short of the exact pane.
    pub partial: Option<String>,
}

/// One terminal multiplexer. Object-safe, stateless, registered statically.
///
/// `focus` and `open` default to [`MuxError::Unsupported`], the same
/// "knows nothing" default shape as `session_detector::ProcessProbe`. The
/// snapshot is passed back into `focus` and `open` so that a stateless static
/// registry entry acts on the environment it was detected from [inferred
/// refinement of D-02's method list].
pub trait MultiplexerBackend: Sync {
    /// The name shown in status lines (`"tmux: <reason>"`).
    fn name(&self) -> &'static str;

    /// `Some` when this backend has anything to say about `env`.
    fn detect(&self, env: &EnvSnapshot) -> Option<Detection>;

    /// Focus the pane or tab running `session`.
    fn focus(
        &self,
        _env: &EnvSnapshot,
        _session: &ClaudeSession,
    ) -> Result<FocusOutcome, MuxError> {
        Err(MuxError::Unsupported(format!(
            "{} cannot focus a session",
            self.name()
        )))
    }

    /// Open `argv` as a new window or tab in `cwd`.
    fn open(&self, _env: &EnvSnapshot, _cwd: &Path, _argv: &ExecArgv) -> Result<(), MuxError> {
        Err(MuxError::Unsupported(format!(
            "{} cannot open a session",
            self.name()
        )))
    }
}

/// Every backend, in the static tie-break order: tmux first (D-07, SC2).
pub static REGISTRY: &[&dyn MultiplexerBackend] = &[&tmux::TMUX, &herdr::HERDR];

/// The backends that detected themselves as "inside", in `registry` order, each
/// at most once. **Pure.**
///
/// Detection returns an ordered list, innermost first, because focus and launch
/// use the order differently (D-08): focus tries each in turn, launch takes the
/// first. The detection layer is now shared, superseding the old note that only
/// `in_tmux()` was shared between focusing and launching (D-04). Two detected
/// backends are never merged and none is duplicated.
pub fn order_innermost(
    registry: &[&'static dyn MultiplexerBackend],
    env: &EnvSnapshot,
) -> Vec<Detected> {
    let mut found: Vec<Detected> = Vec::new();
    for backend in registry {
        if found.iter().any(|d| d.backend.name() == backend.name()) {
            continue;
        }
        if let Some(detection) = backend.detect(env) {
            if detection.inside {
                found.push(Detected {
                    backend: *backend,
                    detection,
                });
            }
        }
    }
    found
}

/// Snapshot the process environment and order [`REGISTRY`] over it. **The
/// impure edge**; only the production seam functions call it.
pub fn detect_current() -> (EnvSnapshot, Vec<Detected>) {
    let env = EnvSnapshot::from_process();
    let backends = order_innermost(REGISTRY, &env);
    (env, backends)
}

/// What to do about a launch request.
///
/// **It carries BOTH answers on purpose.** The launch order is "the innermost
/// multiplexer, and on failure a GUI terminal", so a caller that gets an `Err`
/// back from a backend must be able to fall through without re-deriving
/// anything — and re-deriving is exactly where a second, differently-ordered
/// copy of the decision would grow. Computing the GUI answer unconditionally is
/// also what makes the whole order testable as a pure function: a plan can be
/// asserted for an environment nobody is running in.
pub struct LaunchPlan {
    /// The environment the backends were detected from.
    pub env: EnvSnapshot,
    /// Detected backends, innermost first. Launch tries only the first.
    pub backends: Vec<Detected>,
    /// The GUI terminal to use — either because no backend was detected, or
    /// because the innermost one was tried and failed.
    pub gui: Option<String>,
}

/// Decide where a Claude session should be opened. **Pure**: no I/O, no
/// environment reads, no spawning.
///
/// # D-01 — a multiplexer OUTRANKS `$TERMINAL`
///
/// `$TERMINAL` expresses which *GUI emulator* the user prefers, not *where the
/// session should appear*. It is commonly exported globally from a shell
/// profile, so it is rarely a per-invocation intent. If it outranked tmux,
/// every user with `$TERMINAL` set would keep hitting exactly the reported bug:
/// a GUI window opening while they sit inside a tmux session.
///
/// The D-01 argument applies to every detected multiplexer, not just tmux, so
/// `$TERMINAL` ranks below any of them (D-10).
///
/// **Residual, stated rather than fixed:** a user sitting inside tmux can no
/// longer force a GUI window by setting `$TERMINAL`. No new environment
/// variable and no config key is added to recover that. `$TERMINAL` is not
/// lost — the backend branch falls through to it whenever no multiplexer is
/// detected, no server is reachable, or the open call fails.
///
/// # D-02 — the GUI order, with the reason each rank is where it is
///
/// See [`gui::rank_gui`].
pub fn plan_launch(
    env: EnvSnapshot,
    backends: Vec<Detected>,
    probes: &gui::TerminalProbes,
) -> LaunchPlan {
    LaunchPlan {
        env,
        backends,
        gui: gui::rank_gui(probes),
    }
}

/// The GUI spawner [`launch_with`] calls: `(terminal, argv, cwd)`.
pub type GuiSpawner<'a> = dyn FnMut(&str, &[String], &Path) -> std::io::Result<()> + 'a;

/// Open a session where the user is, and report what happened. **Pure over its
/// injected collaborators**: the argv result, the GUI argv builder and the GUI
/// spawner are all arguments, so the whole decision runs in a test without a
/// multiplexer or a terminal (D-10).
///
/// 1. Take the innermost detected backend and **never any later entry**: a
///    session must not open in an outer multiplexer the user is not looking at.
///    If it exists, derive a reason it did not launch: its detection's
///    unreachable text, else the argv's refusal (with `open` not called), else
///    call `open`. `open` succeeding returns `success`; failing gives the
///    error's text as the reason.
/// 2. Then the GUI: with `plan.gui` set, spawn it with `gui_argv(term)` in
///    `cwd`. A fall-through is REPORTED rather than silent, as
///    `"<backend>: <reason> — opened in <term>"`; the user asked for a session
///    where they are and got one somewhere else.
/// 3. With no GUI, `"<backend>: <reason>; no terminal emulator found (set
///    $TERMINAL)"`, or the bare `"No terminal emulator found (set $TERMINAL)"`
///    when no backend was involved.
pub fn launch_with(
    plan: &LaunchPlan,
    cwd: &Path,
    argv: Result<ExecArgv, MuxError>,
    gui_argv: &dyn Fn(&str) -> Vec<String>,
    spawn_gui: &mut GuiSpawner<'_>,
    success: &str,
) -> String {
    let mut fallthrough: Option<(&'static str, String)> = None;
    if let Some(innermost) = plan.backends.first() {
        let reason = match &innermost.detection.reachable {
            Err(why) => why.clone(),
            Ok(()) => match &argv {
                Err(refused) => refused.to_string(),
                Ok(argv) => match innermost.backend.open(&plan.env, cwd, argv) {
                    Ok(()) => return success.to_string(),
                    Err(e) => e.to_string(),
                },
            },
        };
        fallthrough = Some((innermost.backend.name(), reason));
    }

    match &plan.gui {
        Some(term) => match spawn_gui(term, &gui_argv(term), cwd) {
            Ok(()) => match fallthrough {
                Some((name, reason)) => format!("{name}: {reason} — opened in {term}"),
                None => success.to_string(),
            },
            Err(e) => format!("Failed to launch: {e}"),
        },
        None => match fallthrough {
            Some((name, reason)) => {
                format!("{name}: {reason}; no terminal emulator found (set $TERMINAL)")
            }
            None => "No terminal emulator found (set $TERMINAL)".to_string(),
        },
    }
}

/// [`launch_with`] with the real GUI spawner. **The real-environment entry,
/// called only from `detail.rs`.** A test that called it inside herdr would open
/// a real tab, so tests use [`launch_with`] with injected backends.
pub fn launch(
    plan: &LaunchPlan,
    cwd: &Path,
    argv: Result<ExecArgv, MuxError>,
    gui_argv: &dyn Fn(&str) -> Vec<String>,
    success: &str,
) -> String {
    launch_with(
        plan,
        cwd,
        argv,
        gui_argv,
        &mut |term, args, dir| gui::spawn_terminal(term, args, dir),
        success,
    )
}

/// Focus the pane or tab running `session` and report what happened (D-09).
/// **Pure over `backends`**: tests pass fakes.
///
/// * With no backend detected: `"Tab-switching requires running inside <names>
///   (none detected)"`, where `<names>` is the [`REGISTRY`] names joined with
///   `" or "`, so a future backend appears automatically.
/// * Otherwise each backend is tried innermost-first and **the first success
///   wins**. Not found is not final: the session may live in an outer
///   multiplexer. A detection that is `inside` but not `reachable` counts as
///   that backend's failure with its reason, and its `focus` is not called.
/// * On success: `"Switched to <label>"`, or `"Switched to <label> (<note>)"`
///   when the focus landed short of the exact pane. The matched identity goes to
///   the log only, which keeps the success text byte-identical to the tmux-only
///   days.
/// * When all fail: a single tried backend gives its reason verbatim (so tmux
///   failures are byte-identical); several give `"<name>: <reason>"` joined with
///   `"; "`, innermost first.
pub fn focus_with(
    backends: &[Detected],
    env: &EnvSnapshot,
    session: &ClaudeSession,
    label: &str,
) -> String {
    if backends.is_empty() {
        let names: Vec<&str> = REGISTRY.iter().map(|b| b.name()).collect();
        return format!(
            "Tab-switching requires running inside {} (none detected)",
            names.join(" or ")
        );
    }

    let mut failures: Vec<(&'static str, String)> = Vec::new();
    for detected in backends {
        let name = detected.backend.name();
        if let Err(why) = &detected.detection.reachable {
            failures.push((name, why.clone()));
            continue;
        }
        match detected.backend.focus(env, session) {
            Ok(outcome) => {
                tracing::debug!(backend = name, matched_by = ?outcome.by, "focused session");
                return match outcome.partial {
                    Some(note) => format!("Switched to {label} ({note})"),
                    None => format!("Switched to {label}"),
                };
            }
            Err(e) => failures.push((name, e.to_string())),
        }
    }

    match failures.as_slice() {
        [(_, reason)] => reason.clone(),
        _ => failures
            .iter()
            .map(|(name, reason)| format!("{name}: {reason}"))
            .collect::<Vec<_>>()
            .join("; "),
    }
}

/// [`focus_with`] over the detected process environment. **The
/// real-environment entry, called only from `normal.rs` and `detail.rs`.**
pub fn focus_message(session: &ClaudeSession, label: &str) -> String {
    let (env, backends) = detect_current();
    focus_with(&backends, &env, session, label)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;
    use std::sync::{Arc, Mutex};

    /// A test-only backend that records the calls it receives. Leaked so it can
    /// be `&'static`, as a registry entry must be.
    struct FakeBackend {
        name: &'static str,
        open_result: Result<(), MuxError>,
        opened: Mutex<Vec<(PathBuf, Vec<String>)>>,
        focus_result: Result<FocusOutcome, MuxError>,
        log: Arc<Mutex<Vec<String>>>,
    }

    impl FakeBackend {
        fn leak_focus(
            name: &'static str,
            focus_result: Result<FocusOutcome, MuxError>,
            log: &Arc<Mutex<Vec<String>>>,
        ) -> &'static FakeBackend {
            Box::leak(Box::new(FakeBackend {
                name,
                open_result: Ok(()),
                opened: Mutex::new(Vec::new()),
                focus_result,
                log: Arc::clone(log),
            }))
        }

        fn leak(
            name: &'static str,
            open_result: Result<(), MuxError>,
            log: &Arc<Mutex<Vec<String>>>,
        ) -> &'static FakeBackend {
            Box::leak(Box::new(FakeBackend {
                name,
                open_result,
                opened: Mutex::new(Vec::new()),
                focus_result: Err(MuxError::Unsupported("fake cannot focus".to_string())),
                log: Arc::clone(log),
            }))
        }

        fn open_calls(&self) -> usize {
            self.opened.lock().unwrap().len()
        }
    }

    impl MultiplexerBackend for FakeBackend {
        fn name(&self) -> &'static str {
            self.name
        }
        fn detect(&self, _env: &EnvSnapshot) -> Option<Detection> {
            Some(Detection {
                inside: true,
                reachable: Ok(()),
            })
        }
        fn focus(
            &self,
            _env: &EnvSnapshot,
            _session: &ClaudeSession,
        ) -> Result<FocusOutcome, MuxError> {
            self.log
                .lock()
                .unwrap()
                .push(format!("focus:{}", self.name));
            self.focus_result.clone()
        }
        fn open(&self, _env: &EnvSnapshot, cwd: &Path, argv: &ExecArgv) -> Result<(), MuxError> {
            self.log.lock().unwrap().push(format!("open:{}", self.name));
            self.opened
                .lock()
                .unwrap()
                .push((cwd.to_path_buf(), argv.as_slice().to_vec()));
            self.open_result.clone()
        }
    }

    fn new_log() -> Arc<Mutex<Vec<String>>> {
        Arc::new(Mutex::new(Vec::new()))
    }

    fn detected(backend: &'static dyn MultiplexerBackend) -> Detected {
        Detected {
            backend,
            detection: Detection {
                inside: true,
                reachable: Ok(()),
            },
        }
    }

    fn plan_with(backends: Vec<Detected>, gui: Option<&str>) -> LaunchPlan {
        LaunchPlan {
            env: EnvSnapshot::default(),
            backends,
            gui: gui.map(str::to_string),
        }
    }

    fn argv_ok() -> Result<ExecArgv, MuxError> {
        ExecArgv::new(vec!["env".to_string(), "claude".to_string()])
    }

    fn gui_argv_for(term: &str) -> Vec<String> {
        vec![format!("--sep-for-{term}"), "claude".to_string()]
    }

    /// **The fail-closed sink for the direct-exec boundary** (D-03, T-W0D-01),
    /// moved from the tmux `open_new_window` test with its assertions kept.
    /// Reached without any multiplexer, because the refusals happen before a
    /// process or socket is touched.
    #[test]
    fn exec_argv_refuses_a_vector_tmux_would_hand_to_an_interpreter() {
        let one = vec!["claude".to_string()];
        let err = ExecArgv::new(one)
            .expect_err("a ONE-element program vector must be refused: tmux runs it under `sh -c`");
        assert!(
            err.to_string().contains("directly"),
            "the refusal must name the direct-exec boundary it is protecting; got {err:?}"
        );
        assert!(matches!(err, MuxError::RefusedForSafety(_)));

        assert!(
            ExecArgv::new(Vec::new()).is_err(),
            "an EMPTY program vector must be refused"
        );

        let dashed = vec!["-d".to_string(), "claude".to_string()];
        let err = ExecArgv::new(dashed)
            .expect_err("a program vector whose head starts with `-` must be refused");
        assert!(
            err.to_string().contains("option"),
            "the refusal must name tmux's own option parser as the reason; got {err:?}"
        );

        let ok = vec!["env".to_string(), "claude".to_string()];
        let built = ExecArgv::new(ok.clone()).expect("env + program is accepted");
        assert_eq!(built.as_slice(), ok.as_slice());
    }

    /// An argv element with spaces, `;`, `$HOME` and `*` is still ONE element.
    #[test]
    fn exec_argv_preserves_elements_and_order_byte_for_byte() {
        let hostile = "lit a b;touch X $HOME *".to_string();
        let v = vec!["env".to_string(), hostile.clone(), "z".to_string()];
        let built = ExecArgv::new(v.clone()).unwrap();
        assert_eq!(built.as_slice(), v.as_slice());
        assert_eq!(built.as_slice()[1], hostile);
    }

    /// One row of the launch-order table: label, inside tmux, probes, expected
    /// backend names, expected GUI.
    type LaunchRow<'a> = (
        &'a str,
        bool,
        gui::TerminalProbes,
        Vec<&'a str>,
        Option<&'a str>,
    );

    fn probes(
        terminal_env: Option<&str>,
        xdg_ok: bool,
        x_term: bool,
        fallback: &[&str],
    ) -> gui::TerminalProbes {
        gui::TerminalProbes {
            terminal_env: terminal_env.map(str::to_string),
            xdg_terminal_exec_ok: xdg_ok,
            x_terminal_emulator_present: x_term,
            fallback_present: fallback.iter().map(|s| (*s).to_string()).collect(),
        }
    }

    fn plan_for(tmux_inside: bool, p: &gui::TerminalProbes) -> LaunchPlan {
        let env = if tmux_inside {
            EnvSnapshot::from_pairs(&[("TMUX", "/tmp/tmux-1000/default,1,0")])
        } else {
            EnvSnapshot::default()
        };
        let backends = order_innermost(REGISTRY, &env);
        plan_launch(env, backends, p)
    }

    fn backend_names(plan: &LaunchPlan) -> Vec<&'static str> {
        plan.backends.iter().map(|d| d.backend.name()).collect()
    }

    /// **The whole launch order, as a pure function** (D-01, D-02).
    ///
    /// This table spawns nothing, reads no environment variable and does not
    /// depend on which terminals happen to be installed on the machine running
    /// it — which is the entire point of `plan_launch` taking its facts as an
    /// argument instead of reading them.
    #[test]
    fn plan_launch_ranks_tmux_first_and_then_the_discovered_default() {
        let rows: Vec<LaunchRow> = vec![
            (
                "inside tmux, tmux WINS over $TERMINAL — and the GUI answer is \
                 still computed so the caller can fall through without \
                 re-deriving the decision (D-01)",
                true,
                probes(Some("kitty"), true, true, &["gnome-terminal"]),
                vec!["tmux"],
                Some("kitty"),
            ),
            (
                "outside tmux, $TERMINAL is the top GUI rank",
                false,
                probes(Some("kitty"), true, false, &[]),
                vec![],
                Some("kitty"),
            ),
            (
                "an EMPTY $TERMINAL is ignored, as it always was",
                false,
                probes(Some(""), true, false, &["gnome-terminal"]),
                vec![],
                Some("xdg-terminal-exec"),
            ),
            (
                "with no $TERMINAL, xdg-terminal-exec beats BOTH lower ranks \
                 (D-02)",
                false,
                probes(None, true, true, &["gnome-terminal"]),
                vec![],
                Some("xdg-terminal-exec"),
            ),
            (
                "x-terminal-emulator is the next rank once xdg cannot resolve",
                false,
                probes(None, false, true, &["gnome-terminal"]),
                vec![],
                Some("x-terminal-emulator"),
            ),
            (
                "the fallback list is last resort, first PRESENT entry in the \
                 caller's order",
                false,
                probes(None, false, false, &["gnome-terminal", "xterm"]),
                vec![],
                Some("gnome-terminal"),
            ),
            (
                "nothing present at all is None, not a guess",
                false,
                probes(None, false, false, &[]),
                vec![],
                None,
            ),
        ];

        for (label, in_tmux, probe, expected_backends, expected_gui) in rows {
            let plan = plan_for(in_tmux, &probe);
            assert_eq!(
                backend_names(&plan),
                expected_backends,
                "backends wrong for the case: {label}"
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
        let plan = plan_for(
            true,
            &probes(None, true, true, &["ptyxis", "gnome-terminal", "xterm"]),
        );
        assert_eq!(
            backend_names(&plan),
            vec!["tmux"],
            "the plan declined to try tmux in the exact environment the bug was \
             reported from, so a GUI terminal ({:?}) would be spawned while the \
             user is sitting inside a tmux session. That is the reported defect \
             (D-01).",
            plan.gui
        );
    }

    #[test]
    fn order_innermost_over_env_fixtures() {
        let names = |pairs: &[(&str, &str)]| -> Vec<&'static str> {
            order_innermost(REGISTRY, &EnvSnapshot::from_pairs(pairs))
                .iter()
                .map(|d| d.backend.name())
                .collect()
        };
        assert_eq!(names(&[("TMUX", "/tmp/t,1,0")]), vec!["tmux"]);
        assert_eq!(
            names(&[("HERDR_ENV", "1"), ("HERDR_SOCKET_PATH", "/tmp/h.sock")]),
            vec!["herdr"]
        );
        assert_eq!(
            names(&[
                ("TMUX", "/tmp/t,1,0"),
                ("HERDR_ENV", "1"),
                ("HERDR_SOCKET_PATH", "/tmp/h.sock")
            ]),
            vec!["tmux", "herdr"],
            "with no ordering signal the order is the static REGISTRY order"
        );
        assert!(names(&[("HERDR_ENV", "0")]).is_empty());
        assert!(names(&[("HERDR_ENV", "true")]).is_empty());
        assert_eq!(
            names(&[("TMUX", "")]),
            vec!["tmux"],
            "a set-but-empty $TMUX still counts, as in_tmux() did"
        );
        assert!(names(&[]).is_empty());
    }

    #[test]
    fn order_innermost_returns_each_backend_at_most_once() {
        let env = EnvSnapshot::from_pairs(&[("TMUX", "x")]);
        let doubled: [&'static dyn MultiplexerBackend; 3] =
            [&tmux::TMUX, &tmux::TMUX, &herdr::HERDR];
        let found = order_innermost(&doubled, &env);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].backend.name(), "tmux");
    }

    #[test]
    fn herdr_without_a_socket_is_inside_but_not_reachable() {
        let env = EnvSnapshot::from_pairs(&[("HERDR_ENV", "1")]);
        let detection = herdr::HERDR.detect(&env).expect("HERDR_ENV=1 is a marker");
        assert!(detection.inside);
        let why = detection
            .reachable
            .expect_err("no socket path, not reachable");
        assert!(why.contains("HERDR_SOCKET_PATH"), "{why}");
    }

    fn run_launch(
        plan: &LaunchPlan,
        argv: Result<ExecArgv, MuxError>,
        spawn: &mut Vec<(String, Vec<String>, PathBuf)>,
        spawn_result: Result<(), String>,
    ) -> String {
        launch_with(
            plan,
            Path::new("/work/proj"),
            argv,
            &gui_argv_for,
            &mut |term, args, cwd| {
                spawn.push((term.to_string(), args.to_vec(), cwd.to_path_buf()));
                spawn_result
                    .clone()
                    .map_err(|m| std::io::Error::new(std::io::ErrorKind::NotFound, m))
            },
            "Launched new Claude session",
        )
    }

    #[test]
    fn launch_with_reports_success_when_the_backend_opens() {
        let log = new_log();
        let fake = FakeBackend::leak("tmux", Ok(()), &log);
        let plan = plan_with(vec![detected(fake)], Some("kitty"));
        let mut spawned = Vec::new();
        let msg = run_launch(&plan, argv_ok(), &mut spawned, Ok(()));
        assert_eq!(msg, "Launched new Claude session");
        assert!(spawned.is_empty(), "the GUI is not touched on success");
        assert_eq!(fake.open_calls(), 1);
    }

    #[test]
    fn launch_with_falls_through_to_the_gui_and_names_the_backend() {
        let log = new_log();
        let fake = FakeBackend::leak("tmux", Err(MuxError::Failed("boom".into())), &log);
        let plan = plan_with(vec![detected(fake)], Some("kitty"));
        let mut spawned = Vec::new();
        let msg = run_launch(&plan, argv_ok(), &mut spawned, Ok(()));
        assert_eq!(msg, "tmux: boom — opened in kitty");
        assert_eq!(
            spawned,
            vec![(
                "kitty".to_string(),
                gui_argv_for("kitty"),
                PathBuf::from("/work/proj")
            )]
        );
    }

    #[test]
    fn launch_with_without_a_gui_reports_the_reason_and_the_hint() {
        let log = new_log();
        let fake = FakeBackend::leak("tmux", Err(MuxError::Failed("boom".into())), &log);
        let plan = plan_with(vec![detected(fake)], None);
        let mut spawned = Vec::new();
        let msg = run_launch(&plan, argv_ok(), &mut spawned, Ok(()));
        assert_eq!(
            msg,
            "tmux: boom; no terminal emulator found (set $TERMINAL)"
        );
    }

    #[test]
    fn launch_with_a_failing_spawner_reports_failed_to_launch() {
        let log = new_log();
        let fake = FakeBackend::leak("tmux", Err(MuxError::Failed("boom".into())), &log);
        let plan = plan_with(vec![detected(fake)], Some("kitty"));
        let mut spawned = Vec::new();
        let msg = run_launch(&plan, argv_ok(), &mut spawned, Err("no such file".into()));
        assert!(msg.starts_with("Failed to launch: "), "{msg}");
    }

    #[test]
    fn launch_with_no_backend_goes_straight_to_the_gui() {
        let plan = plan_with(Vec::new(), Some("kitty"));
        let mut spawned = Vec::new();
        let msg = run_launch(&plan, argv_ok(), &mut spawned, Ok(()));
        assert_eq!(msg, "Launched new Claude session");
        assert_eq!(spawned.len(), 1);

        let plan = plan_with(Vec::new(), None);
        let msg = run_launch(&plan, argv_ok(), &mut Vec::new(), Ok(()));
        assert_eq!(msg, "No terminal emulator found (set $TERMINAL)");
    }

    #[test]
    fn launch_with_a_refused_argv_never_calls_open() {
        let log = new_log();
        let fake = FakeBackend::leak("tmux", Ok(()), &log);
        let plan = plan_with(vec![detected(fake)], Some("kitty"));
        let refused = ExecArgv::new(vec!["claude".to_string()]);
        let mut spawned = Vec::new();
        let msg = run_launch(&plan, refused, &mut spawned, Ok(()));
        assert!(
            msg.starts_with(
                "tmux: refusing to open a tmux window from a 1-element program vector: "
            ),
            "{msg}"
        );
        assert!(msg.ends_with(" — opened in kitty"), "{msg}");
        assert_eq!(fake.open_calls(), 0);
    }

    #[test]
    fn launch_with_two_detected_backends_only_tries_the_first() {
        let log = new_log();
        let first = FakeBackend::leak("herdr", Err(MuxError::Failed("nope".into())), &log);
        let second = FakeBackend::leak("tmux", Ok(()), &log);
        let plan = plan_with(vec![detected(first), detected(second)], None);
        let mut spawned = Vec::new();
        let msg = run_launch(&plan, argv_ok(), &mut spawned, Ok(()));
        assert_eq!(
            msg,
            "herdr: nope; no terminal emulator found (set $TERMINAL)"
        );
        assert_eq!(first.open_calls(), 1);
        assert_eq!(
            second.open_calls(),
            0,
            "an outer multiplexer the user is not looking at is never tried (D-10)"
        );
    }

    #[test]
    fn launch_with_an_unreachable_detection_never_calls_open() {
        let log = new_log();
        let fake = FakeBackend::leak("herdr", Ok(()), &log);
        let plan = plan_with(
            vec![Detected {
                backend: fake,
                detection: Detection {
                    inside: true,
                    reachable: Err("HERDR_SOCKET_PATH is unset".into()),
                },
            }],
            Some("kitty"),
        );
        let mut spawned = Vec::new();
        let msg = run_launch(&plan, argv_ok(), &mut spawned, Ok(()));
        assert_eq!(msg, "herdr: HERDR_SOCKET_PATH is unset — opened in kitty");
        assert_eq!(fake.open_calls(), 0);
    }

    #[test]
    fn mux_error_classes_map_to_try_next() {
        assert!(MuxError::NotFound("x".into()).is_try_next());
        assert!(MuxError::Unsupported("x".into()).is_try_next());
        assert!(!MuxError::RefusedForSafety("x".into()).is_try_next());
        assert!(!MuxError::Failed("x".into()).is_try_next());
    }

    #[test]
    fn a_backend_that_overrides_nothing_cannot_open() {
        struct Bare;
        impl MultiplexerBackend for Bare {
            fn name(&self) -> &'static str {
                "bare"
            }
            fn detect(&self, _env: &EnvSnapshot) -> Option<Detection> {
                None
            }
        }
        let err = Bare
            .open(&EnvSnapshot::default(), Path::new("/"), &argv_ok().unwrap())
            .unwrap_err();
        assert_eq!(
            err,
            MuxError::Unsupported("bare cannot open a session".into())
        );
    }

    // ---- focus (D-09) -------------------------------------------------------

    fn session() -> ClaudeSession {
        ClaudeSession {
            pid: 4242,
            kind: crate::session_detector::SessionKind::Claude,
            session_id: None,
            working_dir: PathBuf::from("/work/proj"),
            start_time: None,
            tty: Some("pts/3".to_string()),
        }
    }

    fn run_focus(backends: &[Detected]) -> String {
        focus_with(backends, &EnvSnapshot::default(), &session(), "alpha")
    }

    #[test]
    fn focus_with_reports_the_none_detected_text() {
        assert_eq!(
            run_focus(&[]),
            "Tab-switching requires running inside tmux or herdr (none detected)"
        );
    }

    #[test]
    fn a_single_backend_failure_is_reported_verbatim() {
        let log = new_log();
        let tmux = FakeBackend::leak_focus(
            "tmux",
            Err(MuxError::NotFound(
                "TTY pts/3 not found in any tmux pane (session may have exited)".to_string(),
            )),
            &log,
        );
        assert_eq!(
            run_focus(&[detected(tmux)]),
            "TTY pts/3 not found in any tmux pane (session may have exited)"
        );
    }

    #[test]
    fn focus_tries_the_next_backend_when_the_first_does_not_find_it() {
        let log = new_log();
        let herdr = FakeBackend::leak_focus("herdr", Err(MuxError::NotFound("a".into())), &log);
        let tmux = FakeBackend::leak_focus(
            "tmux",
            Ok(FocusOutcome {
                by: MatchedBy::Tty,
                partial: None,
            }),
            &log,
        );
        assert_eq!(
            run_focus(&[detected(herdr), detected(tmux)]),
            "Switched to alpha"
        );
        assert_eq!(
            *log.lock().unwrap(),
            vec!["focus:herdr".to_string(), "focus:tmux".to_string()],
            "innermost first, and both were called (not found is not final)"
        );
    }

    #[test]
    fn focus_failures_of_several_backends_are_prefixed_with_their_names() {
        let log = new_log();
        let herdr = FakeBackend::leak_focus("herdr", Err(MuxError::NotFound("a".into())), &log);
        let tmux = FakeBackend::leak_focus("tmux", Err(MuxError::NotFound("b".into())), &log);
        assert_eq!(
            run_focus(&[detected(herdr), detected(tmux)]),
            "herdr: a; tmux: b"
        );
    }

    #[test]
    fn a_partial_focus_appends_its_note() {
        let log = new_log();
        let herdr = FakeBackend::leak_focus(
            "herdr",
            Ok(FocusOutcome {
                by: MatchedBy::Cwd,
                partial: Some("tab only".to_string()),
            }),
            &log,
        );
        assert_eq!(
            run_focus(&[detected(herdr)]),
            "Switched to alpha (tab only)"
        );
    }

    #[test]
    fn an_unreachable_detection_reports_its_reason_and_skips_focus() {
        let log = new_log();
        let herdr = FakeBackend::leak_focus(
            "herdr",
            Ok(FocusOutcome {
                by: MatchedBy::Pid,
                partial: None,
            }),
            &log,
        );
        let msg = run_focus(&[Detected {
            backend: herdr,
            detection: Detection {
                inside: true,
                reachable: Err("HERDR_SOCKET_PATH is unset".to_string()),
            },
        }]);
        assert_eq!(msg, "HERDR_SOCKET_PATH is unset");
        assert!(log.lock().unwrap().is_empty(), "focus was not called");
    }

    #[test]
    fn a_backend_that_does_not_override_focus_is_unsupported() {
        struct Bare;
        impl MultiplexerBackend for Bare {
            fn name(&self) -> &'static str {
                "bare"
            }
            fn detect(&self, _env: &EnvSnapshot) -> Option<Detection> {
                None
            }
        }
        static BARE: Bare = Bare;
        assert_eq!(run_focus(&[detected(&BARE)]), "bare cannot focus a session");
    }

    // ---- the tracer's end-to-end check: a fake herdr socket ----------------

    /// Serve ONE request on a Unix socket in a tempdir: read a line, hand it to
    /// the test, then write `reply` (when `Some`) and close.
    #[cfg(unix)]
    fn serve_once(
        reply: Option<&'static str>,
    ) -> (
        tempfile::TempDir,
        PathBuf,
        std::sync::mpsc::Receiver<String>,
        std::thread::JoinHandle<()>,
    ) {
        use std::io::{BufRead, BufReader, Write};
        use std::os::unix::net::UnixListener;

        let dir = tempfile::tempdir().expect("tempdir");
        let path = dir.path().join("h.sock");
        let listener = UnixListener::bind(&path).expect("bind");
        let (tx, rx) = std::sync::mpsc::channel();
        let handle = std::thread::spawn(move || {
            let (stream, _) = listener.accept().expect("accept");
            let mut line = String::new();
            BufReader::new(&stream).read_line(&mut line).expect("read");
            tx.send(line).ok();
            if let Some(reply) = reply {
                let mut stream = &stream;
                stream.write_all(reply.as_bytes()).ok();
                stream.write_all(b"\n").ok();
            }
        });
        (dir, path, rx, handle)
    }

    #[cfg(unix)]
    fn herdr_plan(socket: &Path, gui: Option<&str>) -> LaunchPlan {
        let env = EnvSnapshot::from_pairs(&[
            ("HERDR_ENV", "1"),
            ("HERDR_SOCKET_PATH", socket.to_str().unwrap()),
            ("HERDR_WORKSPACE_ID", "wT"),
        ]);
        let backends = order_innermost(REGISTRY, &env);
        assert_eq!(backends.len(), 1, "only herdr is detected");
        LaunchPlan {
            env,
            backends,
            gui: gui.map(str::to_string),
        }
    }

    /// **The tracer**: the real herdr backend, a fake socket, one
    /// `layout.apply` request whose pane command equals the `ExecArgv` element
    /// for element, no shell and no GUI spawn (MUX-05, D-12, D-13).
    #[cfg(unix)]
    #[test]
    fn launch_inside_herdr_opens_a_tab_through_layout_apply() {
        let (_dir, socket, rx, handle) = serve_once(Some(
            r#"{"id":"gsdmm-open","result":{"type":"layout_apply"}}"#,
        ));
        let plan = herdr_plan(&socket, Some("kitty"));
        let mut spawned = Vec::new();
        let msg = run_launch(&plan, argv_ok(), &mut spawned, Ok(()));
        handle.join().unwrap();

        assert_eq!(msg, "Launched new Claude session");
        assert!(
            spawned.is_empty(),
            "the GUI spawner is never called: {spawned:?}"
        );

        let line = rx.recv().expect("the listener saw a request");
        let req: serde_json::Value = serde_json::from_str(line.trim()).expect("one JSON line");
        assert_eq!(req["method"], "layout.apply");
        assert_eq!(req["params"]["workspace_id"], "wT");
        assert_eq!(req["params"]["focus"], true);
        assert_eq!(req["params"]["root"]["type"], "pane");
        assert_eq!(req["params"]["root"]["cwd"], "/work/proj");
        assert_eq!(
            req["params"]["root"]["command"],
            serde_json::json!(["env", "claude"])
        );
    }

    #[cfg(unix)]
    #[test]
    fn launch_falls_back_to_the_gui_when_herdr_refuses_the_request() {
        let (_dir, socket, _rx, handle) = serve_once(Some(
            r#"{"id":"gsdmm-open","error":{"code":"layout_apply_failed","message":"Unable to spawn env"}}"#,
        ));
        let plan = herdr_plan(&socket, Some("kitty"));
        let mut spawned = Vec::new();
        let msg = run_launch(&plan, argv_ok(), &mut spawned, Ok(()));
        handle.join().unwrap();
        assert_eq!(
            msg,
            "herdr: herdr layout.apply failed (layout_apply_failed): Unable to spawn env — opened in kitty"
        );
        assert_eq!(spawned.len(), 1);
    }
}
