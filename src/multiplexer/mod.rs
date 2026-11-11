//! The terminal-multiplexer backend seam (Phase 33, D-01, D-02).
//!
//! Where a Claude session is focused or opened used to be hardcoded to tmux in
//! `terminal_switch`. This module replaces that with one object-safe trait,
//! [`MultiplexerBackend`], a static [`REGISTRY`] of backends, and a small set of
//! seam functions the UI calls instead of spawning anything itself:
//!
//! * [`launch`] opens a new or resumed session where the user is: the
//!   innermost detected backend, then the GUI terminal chain ([`gui`]).
//! * [`focus_message`] focuses an existing session: the backends the SESSION
//!   itself lives in are tried first, innermost-first, then the TUI's own, and
//!   the first success wins ([`plan_focus`]).
//!
//! # Launch follows the TUI, focus follows the session
//!
//! Launching opens a session where the user is looking, so it targets the
//! TUI's innermost multiplexer ([`launch_with`], D-10). Focusing goes to a
//! session that already exists, wherever it lives, so it is routed by the
//! session's own `/proc/<pid>/environ` markers and ancestry
//! ([`SessionContext`]), never by where the TUI happens to run: a TUI in tmux
//! focuses a session under a herdr server, a TUI in herdr focuses a session in
//! tmux (on the tmux server named by the SESSION's `$TMUX`), and both then
//! bring forward the terminal that displays the result (`reveal_client`).
//! This supersedes Phase 33's "controlling a multiplexer the TUI is not running
//! inside" exclusion, for focus only. Both UI paths (the Sessions panel and the
//! tab-switch) call the one [`focus_message`].
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
//! # Design note: the trait against every surveyed tool (MUX-01, D-14)
//!
//! This is the canonical copy of the design record. The trait has three
//! operations, and each column below is what one operation needs from a tool:
//! **detect** (is the TUI inside it, and is it drivable from here), **focus**
//! (find the pane that runs a session by some join key, then select it) and
//! **open** (run a program vector in a directory as a new window or tab).
//! Confidence tags are research's: \[VERIFIED\] was run on this machine,
//! \[CITED\] comes from the tool's documentation, \[ASSUMED\] is unconfirmed.
//!
//! | Tool | detect (env gate; ancestor comm) | focus (join key) | open in cwd with argv | exec-direct? | status |
//! |---|---|---|---|---|---|
//! | tmux | `$TMUX` set; comm `tmux: server` \[VERIFIED\] | `list-panes -a -F '#{pane_tty} ...'`, exact tty, then `select-window` and `select-pane` \[VERIFIED\] | `new-window -c <cwd> <argv...>` | yes with 2 or more args \[CITED: man tmux\] | implemented |
//! | herdr TUI 0.9.5 | `HERDR_ENV=1` and `HERDR_SOCKET_PATH`; comm `herdr` \[VERIFIED\] | `pane list`, then `agent_session.value`, else pid via `pane process-info`, else a unique same-cwd pane herdr reports running the session's agent (reported as a directory-only match), then `agent focus <pane_id>` with `tab focus` as fallback \[VERIFIED\] | socket `layout.apply` with `{root:{type:pane,cwd,command:[...]}}` \[VERIFIED\] | `layout.apply` yes; `pane run` no, it types into a shell \[VERIFIED\] | implemented |
//! | herdr-core / `hc` | Unsupported until verified: a separate server, not the TUI's \[CITED: secondary sources\] | `hc ls --json` \[CITED\] | `hc run --json --cwd <dir> -- <cmd>` \[CITED\] | unknown \[ASSUMED: argv after `--`\] | deferred, MUX-F06 |
//! | Rex | Unsupported until verified: detection unknown, closed-source public beta | `rex` CLI with `--json` \[CITED: secondary sources\] | the CLI creates sessions and runs commands \[CITED: secondary sources\] | unknown | deferred, MUX-F05 |
//! | zellij | `$ZELLIJ`, `$ZELLIJ_SESSION_NAME` \[ASSUMED\]; `$ZELLIJ_PANE_ID` \[CITED\] | `zellij action list-panes -j`, then `focus-pane-id <id>` \[CITED\] | `zellij action new-pane --cwd <cwd> -- <argv>` \[CITED\] | likely \[ASSUMED\] | deferred, MUX-F01 |
//! | WezTerm mux | `$WEZTERM_PANE` \[CITED\] | `wezterm cli list --format json`, then `activate-pane --pane-id` \[ASSUMED\] | `wezterm cli spawn --cwd <cwd> -- <argv>` \[CITED\] | likely \[ASSUMED\] | deferred, MUX-F02 |
//! | kitty remote control | `$KITTY_WINDOW_ID` \[CITED\]; drivable only with `allow_remote_control` and `KITTY_LISTEN_ON` \[CITED\], so "inside" differs from "reachable" | `kitten @ ls` (id, cwd, pid, cmdline per window), then `focus-window --match id:<n>` \[CITED\] | `kitten @ launch --type=tab --cwd <cwd> <argv>` \[ASSUMED for `--cwd`\] | likely \[ASSUMED\] | deferred, MUX-F03 |
//! | GNU screen | `$STY` \[CITED: man screen\] | `screen -Q windows` (weak: no tty or pid), then `-X select <n>` | open is Unsupported: `-X chdir` then `-X screen <cmd>` is re-tokenized by screen's own command parser \[ASSUMED\], which would put a parser in the path | no \[ASSUMED\] | deferred, MUX-F04, focus only |
//!
//! ## How each trait method maps onto the table
//!
//! * [`MultiplexerBackend::detect`] is the "detect" column, asked of the
//!   SESSION's environment for focus and of the TUI's for launch. It returns
//!   [`Detection`], which keeps `inside` (the marker is set) apart from
//!   `reachable` (the tool can be driven from here), because kitty shows the two
//!   differ and herdr can carry its marker without a socket (D-02). The
//!   "ancestor comm" half of the column is [`MultiplexerBackend::is_server_comm`],
//!   used only to order a nesting ([`order_innermost`]).
//! * [`MultiplexerBackend::focus`] is the "focus" column: the join key is what
//!   the tool exposes about a pane. tmux exposes a tty, herdr exposes an agent
//!   session id and pids but no tty, GNU screen exposes neither, which is why
//!   its focus is weak. A tool that cannot satisfy the join answers
//!   [`MuxError::Unsupported`], the trait default.
//! * [`MultiplexerBackend::locate_session`] is the same "focus" column without
//!   the select: the join that finds the pane, used by the Sessions tab's
//!   location column ([`location`]). `focus` is built on it.
//! * [`MultiplexerBackend::client_ttys`] and [`MultiplexerBackend::focus_tty`]
//!   are the reveal step after a focus: a headless server's pane (herdr) is on
//!   a terminal only its CLIENT process shows, and that terminal is usually a
//!   pane of another multiplexer (tmux), found by tty. Defaults: no client
//!   concept, and [`MuxError::Unsupported`].
//! * [`MultiplexerBackend::open`] is the "open" column and takes an
//!   [`ExecArgv`], so the "exec-direct?" column is enforced for every backend by
//!   the one sink (D-05). A tool whose only path re-parses text (GNU screen,
//!   herdr's `pane run`) answers [`MuxError::Unsupported`] or refuses; it is
//!   never given a quoted command line.
//!
//! ## The five call sites that use the seam
//!
//! `ui::screens::detail` calls [`launch`] twice (resume and new session) and
//! [`focus_message`] once, and builds its plan through [`plan_launch`] over
//! [`detect_current`]; `ui::screens::normal` calls [`focus_message`] once. No UI
//! code spawns a multiplexer command itself, and `tests/focus_entry_guard.rs`
//! pins that these two are the only focus callers and that nothing outside
//! this module reaches a backend.
//!
//! ## Recorded but unused, and superseded
//!
//! * `TERM_PROGRAM` names the innermost multiplexer in both nestings
//!   (`tmux` and `herdr`, measured) and would work off Linux too. It is a
//!   candidate signal that is deliberately NOT used, so as not to invent a
//!   second heuristic beside the env gate plus ancestry (D-07). It is also
//!   fragile: tmux older than 3.2 does not set it.
//! * D-04 supersedes the old note that only `in_tmux()` was shared between
//!   focusing and launching: the detection layer is now shared, the command
//!   runners are not.
//! * D-11's tty step is replaced by a pid step in herdr's focus, because herdr
//!   exposes no tty (`herdr.rs` module docs). The exact-match and refuse-on-
//!   ambiguity rules are unchanged.

// Every user of these fixtures is a `#[cfg(unix)]` test module (they run
// `#!/bin/sh` scripts), so the module is gated to exactly that cfg.
#[cfg(all(test, unix))]
pub(crate) mod fixtures;
pub mod gui;
pub mod herdr;
pub mod location;
pub mod tmux;

pub use location::{
    locate_sessions, LocatedEntry, LocationLookup, PaneLocation, ResolvedLocations,
    SessionLocation, SessionLocationCache,
};

use crate::session_detector::ClaudeSession;
use std::collections::BTreeMap;
use std::fmt;
use std::path::Path;

/// Why a backend did not do what it was asked (D-03).
///
/// The four variants fall into three classes. **The class describes why the
/// backend did not act; it does not decide what the seam does next.** Both seam
/// functions treat every error alike: a failing backend is reported and the next
/// step is tried ([`focus_with`] moves on to the next detected backend,
/// [`launch_with`] falls through to the GUI terminal chain), and the first
/// success wins \[inferred, for audit: review WR-03 found the previous docs
/// promising retry rules the seam never applied\].
///
/// * **Not mine / not found** ([`MuxError::NotFound`], [`MuxError::Unsupported`]):
///   this backend has nothing to act on.
/// * **Refused for safety** ([`MuxError::RefusedForSafety`]): the request itself
///   would put a command interpreter in the path, or the backend would have to
///   guess. Falling through is safe because no later step reuses the refused
///   vector: the GUI argv is built separately, and a later backend runs its own
///   join. A future step that did reuse it would have to stop here instead.
/// * **Failed** ([`MuxError::Failed`]): the backend tried and the attempt broke.
///
/// `Display` prints only the inner reason, so the reasons tmux already produced
/// stay byte-identical in focus status lines (the launch line shows
/// [`MuxError::launch_phrase`] instead). The backend name is added by the
/// caller.
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

    /// A short, human phrase for the launch status line (review UI-01). The
    /// full [`MuxError`] text can carry socket paths, exit statuses and herdr's
    /// own error JSON, which a status line should not show; it goes to the log
    /// instead. The phrase depends on the class only.
    pub fn launch_phrase(&self) -> &'static str {
        match self {
            Self::NotFound(_) => "no target found",
            Self::Unsupported(_) => "cannot open sessions",
            Self::RefusedForSafety(_) => "refused for safety",
            Self::Failed(_) => "could not open a session",
        }
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
/// # Why the messages are backend-neutral \[inferred, for audit\]
///
/// The refusals used to live on tmux's `open_new_window` and named "a tmux
/// window". They now sit on a type every backend's `open` must take (D-05), and
/// `launch_with` adds the backend name itself, so a herdr launch would have read
/// `herdr: refusing to open a tmux window ...` (review IN-02). The text now says
/// "a new window"; the substrings `directly` and `option` that the tests pin are
/// unchanged, and the tmux status line no longer shows this text anyway (UI-01).
#[derive(Debug, Clone)]
pub struct ExecArgv(Vec<String>);

impl ExecArgv {
    /// Validate and wrap a program vector.
    pub fn new(argv: Vec<String>) -> Result<ExecArgv, MuxError> {
        if argv.len() < 2 {
            return Err(MuxError::RefusedForSafety(format!(
                "refusing to open a new window from a {}-element program vector: \
                 a multiplexer execs a shell-command given as MULTIPLE arguments directly, \
                 but may hand a SINGLE argument to a command interpreter — which would \
                 put a parser back in a path that deliberately has none",
                argv.len()
            )));
        }
        if argv[0].starts_with('-') {
            return Err(MuxError::RefusedForSafety(format!(
                "refusing to open a new window whose program vector begins with \
                 {:?}: the multiplexer's own option parser would consume it as one of its \
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

    /// Read [`ENV_KEYS`] from another process's `/proc/<pid>/environ` block
    /// (`KEY=VALUE` entries separated by NUL). **Pure.**
    ///
    /// Only the [`ENV_KEYS`] are kept. The FIRST entry for a key wins, as
    /// `getenv` would have it; an entry without `=`, with a key that is not
    /// UTF-8, or for another key is skipped. Values are read lossily.
    ///
    /// This is untrusted input: another process's environment. A control
    /// socket path is accepted only when it is an absolute path
    /// (`accept_session_value`); anything else is dropped, so it can neither
    /// make a backend a candidate nor reach a command line.
    pub fn from_environ_block(block: &[u8]) -> Self {
        let mut first: BTreeMap<&'static str, &[u8]> = BTreeMap::new();
        for entry in block.split(|byte| *byte == 0) {
            let Some(eq) = entry.iter().position(|byte| *byte == b'=') else {
                continue;
            };
            let Ok(key) = std::str::from_utf8(&entry[..eq]) else {
                continue;
            };
            if let Some(known) = ENV_KEYS.iter().find(|k| **k == key) {
                first.entry(known).or_insert(&entry[eq + 1..]);
            }
        }
        let values = first
            .into_iter()
            .map(|(key, value)| (key, String::from_utf8_lossy(value).into_owned()))
            .filter(|(key, value)| accept_session_value(key, value))
            .map(|(key, value)| (key.to_string(), value))
            .collect();
        Self { values }
    }

    /// `self` with every key `over` sets replaced by `over`'s value. Keys `over`
    /// does not set keep `self`'s. **Pure.**
    pub fn overlaid_with(&self, over: &EnvSnapshot) -> Self {
        let mut values = self.values.clone();
        values.extend(over.values.iter().map(|(k, v)| (k.clone(), v.clone())));
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

/// May `value` of `key`, read from ANOTHER process's environment, be believed?
///
/// A control socket named there is something this build would connect a CLI to,
/// so it must be an absolute path: a relative one would resolve against this
/// process's working directory instead of the session's. `TMUX` is
/// `<socket>,<server pid>,<session index>` and its socket is the part before the
/// first comma ([`tmux::socket_of`]). Every other key is only a marker.
fn accept_session_value(key: &str, value: &str) -> bool {
    match key {
        "HERDR_SOCKET_PATH" => Path::new(value).is_absolute(),
        "TMUX" => tmux::socket_of(value).is_some(),
        _ => true,
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

/// One process as `/proc` shows it: the facts the reveal step needs to tell a
/// multiplexer's CLIENT (the process that displays it in a terminal) from its
/// server and from short-lived CLI calls.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProcEntry {
    pub pid: u32,
    /// The command name from `/proc/<pid>/stat`.
    pub comm: String,
    /// `/proc/<pid>/cmdline`, split on NUL. Empty when it could not be read.
    pub argv: Vec<String>,
    /// The terminal on the process's stdin in `pts/N` / `ttyN` form (see
    /// `terminal_name`). `None` when stdin is not a terminal or unreadable.
    pub tty: Option<String>,
}

/// Look up the processes whose command name EQUALS `comm`. Handed to
/// [`MultiplexerBackend::client_ttys`] so that a backend with no client concept
/// never causes a `/proc` walk. The real implementation is `ProcFs::scan`.
pub type ProcScan<'a> = dyn Fn(&str) -> Vec<ProcEntry> + 'a;

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

    /// Find the pane running `session` and select nothing: `focus`'s join
    /// without the act. The Sessions tab's location column is built on it, and
    /// `focus` is rebuilt on the same join, so the two cannot disagree. Like
    /// `focus`, it answers [`MuxError::NotFound`] when the session is not here
    /// and [`MuxError::Unsupported`] by default.
    fn locate_session(
        &self,
        _env: &EnvSnapshot,
        _session: &ClaudeSession,
    ) -> Result<PaneLocation, MuxError> {
        Err(MuxError::Unsupported(format!(
            "{} cannot locate a session",
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

    /// The terminals (`pts/N`) of this backend's own CLIENT processes: the
    /// processes that DISPLAY it. **Reveal step, part 1.**
    ///
    /// A pane that a headless server owns (herdr) is on no terminal the user can
    /// see; a client process shows it, and that client usually sits in a pane of
    /// ANOTHER multiplexer (tmux). Once this backend has focused the session, the
    /// seam asks it which terminals show it, so the other multiplexer can bring
    /// that terminal forward ([`MultiplexerBackend::focus_tty`]).
    ///
    /// `None` means the backend has no such notion (the default; tmux), so the
    /// seam does no `/proc` walk for it. `Some(vec![])` means it has one and no
    /// client is attached. The list is sorted and holds no duplicates.
    fn client_ttys(&self, _scan: &ProcScan<'_>) -> Option<Vec<String>> {
        None
    }

    /// Bring forward the pane that shows one of `ttys`. **Reveal step, part 2**,
    /// run on a backend OTHER than the one that focused the session. The
    /// default is [`MuxError::Unsupported`]; tmux overrides it.
    fn focus_tty(&self, _env: &EnvSnapshot, _ttys: &[String]) -> Result<FocusOutcome, MuxError> {
        Err(MuxError::Unsupported(format!(
            "{} cannot reveal a client terminal",
            self.name()
        )))
    }

    /// Is `comm` (a `/proc/<pid>/stat` command name) this backend's server
    /// process? Used only to order nested multiplexers ([`order_innermost`]).
    /// Exact equality, never a prefix or substring test. Defaults to `false`:
    /// a backend that cannot be recognised by its process name never wins the
    /// ancestry rule and keeps its static rank.
    fn is_server_comm(&self, _comm: &str) -> bool {
        false
    }
}

/// Every backend, in the static tie-break order: tmux first (D-07, SC2).
pub static REGISTRY: &[&dyn MultiplexerBackend] = &[&tmux::TMUX, &herdr::HERDR];

/// The backends that detected themselves as "inside", innermost first, each at
/// most once. **Pure**: it reads only its arguments.
///
/// Detection returns an ordered list, innermost first, because focus and launch
/// use the order differently (D-08): focus tries each in turn, launch takes the
/// first. The detection layer is now shared, superseding the old note that only
/// `in_tmux()` was shared between focusing and launching (D-04). Two detected
/// backends are never merged and none is duplicated.
///
/// # The rule (MUX-02, D-07)
///
/// 1. The candidates are the registry entries whose own `detect` says "inside",
///    in `registry` order (the static tie-break, tmux first).
/// 2. `ancestry` is the TUI's own process chain, nearest first (see
///    `proc_chain`). The FIRST entry whose comm is some candidate's
///    [`MultiplexerBackend::is_server_comm`] names the innermost multiplexer,
///    and that candidate moves to the front. Nothing else reorders.
/// 3. An ancestor naming a backend whose env gate failed is ignored: the env
///    marker decides candidacy, the ancestry only decides order.
///
/// # Why both signals
///
/// * Env alone cannot order a nesting, because tmux-in-herdr carries both
///   marker sets: herdr leaves its `HERDR_*` variables in the environment of
///   the tmux server started from a herdr pane (research E5).
/// * The walk is nearest-first and first-hit-wins, not "is X anywhere in the
///   chain", because a freshly started herdr server is a child of its client, so
///   one chain can hold both multiplexers' servers (E5).
/// * The static order, tmux first, applies whenever the chain is inconclusive:
///   empty, unreadable, or naming no candidate. Off Linux the chain is always
///   empty, so the static order is the only rule there (SC2, D-07's fallback).
/// * `TERM_PROGRAM` also names the innermost multiplexer in both nestings, but
///   it is deliberately unused, so as not to invent a second heuristic (D-07).
pub fn order_innermost(
    registry: &[&'static dyn MultiplexerBackend],
    env: &EnvSnapshot,
    ancestry: &[(u32, String)],
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

    let innermost = ancestry
        .iter()
        .find_map(|(_, comm)| found.iter().position(|d| d.backend.is_server_comm(comm)));
    if let Some(index) = innermost {
        let winner = found.remove(index);
        found.insert(0, winner);
    }
    found
}

/// The comm of a `/proc/<pid>/stat` line: from the first `(` to the LAST `)`,
/// the kernel's own rule, because the comm may itself contain spaces and `)`.
/// Sibling of `session_detector::parse_stat_starttime`.
pub(crate) fn parse_stat_comm(stat: &str) -> Option<&str> {
    let open = stat.find('(')?;
    let close = stat.rfind(')')?;
    stat.get(open + 1..close)
}

/// The parent pid of a `/proc/<pid>/stat` line: the second whitespace field
/// after the comm (the first is the state). Truncated, non-numeric or
/// out-of-range input is `None`.
pub(crate) fn parse_stat_ppid(stat: &str) -> Option<u32> {
    let after_comm = stat.rfind(')')?.checked_add(1)?;
    let rest = stat.get(after_comm..)?;
    rest.split_whitespace().nth(1)?.parse::<u32>().ok()
}

/// The longest chain `ProcFs::chain` will walk. Bounds a cyclic or absurd chain.
const PROC_CHAIN_MAX: usize = 64;

/// The most bytes of one `environ` or `cmdline` that are read. Another
/// process's data is bounded before it is parsed.
const PROC_READ_MAX: u64 = 1 << 20;

/// The most `/proc` entries one `ProcFs::scan` looks at.
const PROC_SCAN_MAX: usize = 262_144;

/// The terminal name of a `/proc/<pid>/fd/0` link target: `pts/N` for a
/// pseudo-terminal, `ttyN` / `ttyS0` for a console. **Pure.** `/dev/null`, a
/// pipe, a socket or a path outside `/dev` is not a terminal and gives `None`,
/// which is what keeps a headless server (stdin `/dev/null`) from looking like
/// a client.
pub(crate) fn terminal_name(link: &Path) -> Option<String> {
    let link = link.to_str()?;
    let rest = link.strip_prefix("/dev/")?;
    let is_pts = rest
        .strip_prefix("pts/")
        .is_some_and(|n| !n.is_empty() && n.bytes().all(|b| b.is_ascii_digit()));
    let is_console =
        rest.len() > 3 && rest.starts_with("tty") && !rest.contains('/') && !rest.contains(' ');
    (is_pts || is_console).then(|| rest.to_string())
}

/// Where `/proc` is read from, and the ONLY reader of it in this module.
///
/// A value, not a hardcoded path, so a test can describe a process table nobody
/// is running (a herdr server and its client, a session, a tmux server) by
/// laying out a directory. **Facts only**: every method answers "could not say"
/// (empty, `None`) rather than an error, and none of them spawns anything. Off
/// Linux [`ProcFs::system`] has no root, so there is no `/proc` read at all and
/// no `ps` call (that would add a spawn site); D-07's static order is then the
/// only rule, and focus falls back to the TUI-derived backends.
#[derive(Debug, Clone)]
pub(crate) struct ProcFs {
    root: Option<std::path::PathBuf>,
}

impl ProcFs {
    /// The real `/proc` (Linux), or nothing.
    pub(crate) fn system() -> Self {
        Self {
            root: cfg!(target_os = "linux").then(|| std::path::PathBuf::from("/proc")),
        }
    }

    /// A procfs laid out under `root` (only the unix-only fixtures use it).
    #[cfg(all(test, unix))]
    pub(crate) fn at(root: &Path) -> Self {
        Self {
            root: Some(root.to_path_buf()),
        }
    }

    /// No procfs at all.
    #[cfg(test)]
    pub(crate) fn none() -> Self {
        Self { root: None }
    }

    fn path(&self, pid: u32, file: &str) -> Option<std::path::PathBuf> {
        self.root
            .as_ref()
            .map(|root| root.join(pid.to_string()).join(file))
    }

    /// At most [`PROC_READ_MAX`] bytes of `/proc/<pid>/<file>`.
    fn read_bounded(&self, pid: u32, file: &str) -> Option<Vec<u8>> {
        use std::io::Read;
        let mut bytes = Vec::new();
        std::fs::File::open(self.path(pid, file)?)
            .ok()?
            .take(PROC_READ_MAX)
            .read_to_end(&mut bytes)
            .ok()?;
        Some(bytes)
    }

    /// `start` and then each of its ancestors as `(pid, comm)`, nearest first,
    /// read from `/proc/<pid>/stat`. **Facts only**: an empty or short chain
    /// means "could not say", never "no multiplexer".
    ///
    /// The walk stops at ppid 0, at the first unreadable or unparsable entry, on
    /// a repeated pid, or after 64 entries, and never guesses past a gap.
    pub(crate) fn chain(&self, start: u32) -> Vec<(u32, String)> {
        let mut chain: Vec<(u32, String)> = Vec::new();
        let mut pid = start;
        while chain.len() < PROC_CHAIN_MAX {
            let Some(path) = self.path(pid, "stat") else {
                break;
            };
            let Ok(stat) = std::fs::read_to_string(path) else {
                break;
            };
            let (Some(comm), Some(ppid)) = (parse_stat_comm(&stat), parse_stat_ppid(&stat)) else {
                break;
            };
            chain.push((pid, comm.to_string()));
            if ppid == 0 || chain.iter().any(|(seen, _)| *seen == ppid) {
                break;
            }
            pid = ppid;
        }
        chain
    }

    /// The processes whose comm EQUALS `comm`, each with its argv and stdin
    /// terminal. Unreadable entries are skipped. Bounded by [`PROC_SCAN_MAX`]
    /// entries, and only the matching processes have their cmdline and `fd/0`
    /// read.
    pub(crate) fn scan(&self, comm: &str) -> Vec<ProcEntry> {
        let Some(root) = &self.root else {
            return Vec::new();
        };
        let Ok(entries) = std::fs::read_dir(root) else {
            return Vec::new();
        };
        let mut found: Vec<ProcEntry> = entries
            .filter_map(Result::ok)
            .take(PROC_SCAN_MAX)
            .filter_map(|entry| entry.file_name().to_str()?.parse::<u32>().ok())
            .filter_map(|pid| {
                let stat = std::fs::read_to_string(self.path(pid, "stat")?).ok()?;
                (parse_stat_comm(&stat)? == comm).then(|| ProcEntry {
                    pid,
                    comm: comm.to_string(),
                    argv: self
                        .read_bounded(pid, "cmdline")
                        .map(|raw| {
                            raw.split(|b| *b == 0)
                                .filter(|part| !part.is_empty())
                                .map(|part| String::from_utf8_lossy(part).into_owned())
                                .collect()
                        })
                        .unwrap_or_default(),
                    tty: self
                        .path(pid, "fd/0")
                        .and_then(|link| std::fs::read_link(link).ok())
                        .and_then(|target| terminal_name(&target)),
                })
            })
            .collect();
        found.sort_by_key(|entry| entry.pid);
        found
    }

    /// The [`SessionContext`] of `pid`: its `environ` markers and its ancestry.
    /// Each half degrades to "unknown" on its own (another uid's environ is
    /// unreadable, a process that exited has no chain).
    pub(crate) fn session_context(&self, pid: u32) -> SessionContext {
        SessionContext {
            env: self
                .read_bounded(pid, "environ")
                .map(|block| EnvSnapshot::from_environ_block(&block))
                .unwrap_or_default(),
            ancestry: self.chain(pid),
        }
    }
}

/// `start` and each of its ancestors as `(pid, comm)`, nearest first, from the
/// real `/proc` (`ProcFs::chain`).
pub(crate) fn proc_chain(start: u32) -> Vec<(u32, String)> {
    ProcFs::system().chain(start)
}

/// Snapshot the process environment and order [`REGISTRY`] over it and the
/// TUI's own ancestry. **The impure edge**; only the production seam functions
/// call it.
pub fn detect_current() -> (EnvSnapshot, Vec<Detected>) {
    let env = EnvSnapshot::from_process();
    let ancestry = proc_chain(std::process::id());
    let backends = order_innermost(REGISTRY, &env, &ancestry);
    (env, backends)
}

/// What the SESSION's own process says about where it lives: the
/// [`ENV_KEYS`] of its `/proc/<pid>/environ` and its own ancestry (nearest
/// first, as `ProcFs::chain` returns it).
///
/// Focus has to follow the session, not the TUI: the TUI can run in tmux while
/// the session runs under a herdr server that is a sibling daemon, not an
/// ancestor of the TUI, so the TUI's own markers say nothing about it.
/// `Default` is "unknown" (no markers, no chain): everything then falls back to
/// the TUI-derived list, which is exactly the behaviour before this type existed.
#[derive(Debug, Clone, Default)]
pub struct SessionContext {
    /// The session's own [`ENV_KEYS`].
    pub env: EnvSnapshot,
    /// The session's ancestry, nearest first.
    pub ancestry: Vec<(u32, String)>,
}

/// Which backends to try to focus a session, innermost first, and the
/// environment they act on (like [`LaunchPlan`], decided once and purely).
pub struct FocusPlan {
    /// The environment the backends act on: the TUI's, overlaid with the
    /// session's own markers (the session wins).
    pub env: EnvSnapshot,
    /// The backends to try, in order.
    pub backends: Vec<Detected>,
}

/// Decide which backends to try to focus a session. **Pure.**
///
/// # Focus follows the SESSION's multiplexer, wherever the TUI runs
///
/// The user's rule is "jump to either herdr or tmux depending on context": the
/// TUI's location decides only where a NEW session launches. Focus is decided by
/// where the target session lives, read from the session's own facts:
///
/// 1. **The session's backends first**: [`order_innermost`] over the session's
///    own `environ` markers and ancestry (`session`). The same pure rule as for
///    the TUI, so a session nested in two multiplexers is ordered innermost-first
///    and an ancestor whose env gate fails is ignored.
/// 2. **Then the TUI's backends**, de-duplicated by name: D-09's "not found is
///    not final", and the whole answer when nothing is known about the session
///    (non-Linux, another uid, an exited process). With an empty `session` the
///    plan is exactly the TUI-derived one, over the TUI's own environment.
/// 3. **The environment they act on** is the TUI's overlaid with the session's
///    markers, the session winning. That is what lets herdr reach the SESSION's
///    server socket (named herdr sessions have different sockets) and tmux
///    address the SESSION's tmux server (`tmux -S <socket>`, from the session's
///    `$TMUX`), neither of which the TUI's environment has to mention.
///
/// A `Detected`'s reachability is judged against the environment it was
/// detected from (the session's for rule 1, the TUI's for rule 2).
pub fn plan_focus(
    registry: &[&'static dyn MultiplexerBackend],
    tui_env: &EnvSnapshot,
    tui_ancestry: &[(u32, String)],
    session: &SessionContext,
) -> FocusPlan {
    let mut backends = order_innermost(registry, &session.env, &session.ancestry);
    for tui in order_innermost(registry, tui_env, tui_ancestry) {
        if !backends
            .iter()
            .any(|known| known.backend.name() == tui.backend.name())
        {
            backends.push(tui);
        }
    }
    FocusPlan {
        env: tui_env.overlaid_with(&session.env),
        backends,
    }
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
///    `"<backend>: <phrase> — opened in <term>"`; the user asked for a session
///    where they are and got one somewhere else.
/// 3. With no GUI, `"<backend>: <phrase>; no terminal emulator found (set
///    $TERMINAL)"`, or the bare `"No terminal emulator found (set $TERMINAL)"`
///    when no backend was involved.
///
/// `<phrase>` is a short human phrase for the failure class
/// ([`MuxError::launch_phrase`], or `"not reachable"` for an unreachable
/// detection), never the raw reason: that can carry socket paths and herdr's
/// error JSON, so it is logged at `warn` instead (UI-01) \[inferred, for audit\].
pub fn launch_with(
    plan: &LaunchPlan,
    cwd: &Path,
    argv: Result<ExecArgv, MuxError>,
    gui_argv: &dyn Fn(&str) -> Vec<String>,
    spawn_gui: &mut GuiSpawner<'_>,
    success: &str,
) -> String {
    // The backend's name and the short phrase the status line shows. The full
    // reason goes to the log (UI-01).
    let mut fallthrough: Option<(&'static str, &'static str)> = None;
    if let Some(innermost) = plan.backends.first() {
        let (phrase, reason) = match &innermost.detection.reachable {
            Err(why) => ("not reachable", why.clone()),
            Ok(()) => match &argv {
                Err(refused) => (refused.launch_phrase(), refused.to_string()),
                Ok(argv) => match innermost.backend.open(&plan.env, cwd, argv) {
                    Ok(()) => return success.to_string(),
                    Err(e) => (e.launch_phrase(), e.to_string()),
                },
            },
        };
        tracing::warn!(
            backend = innermost.backend.name(),
            reason = %reason,
            "the multiplexer did not open the session; falling back to a GUI terminal"
        );
        fallthrough = Some((innermost.backend.name(), phrase));
    }

    match &plan.gui {
        Some(term) => match spawn_gui(term, &gui_argv(term), cwd) {
            Ok(()) => match fallthrough {
                Some((name, phrase)) => format!("{name}: {phrase} — opened in {term}"),
                None => success.to_string(),
            },
            Err(e) => format!("Failed to launch: {e}"),
        },
        None => match fallthrough {
            Some((name, phrase)) => {
                format!("{name}: {phrase}; no terminal emulator found (set $TERMINAL)")
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
/// **Pure over `backends`**: tests pass fakes. Reads `/proc` only for the reveal
/// step (`reveal_client`).
///
/// * With no backend detected: `"Tab-switching requires running inside <names>
///   (none detected)"`, where `<names>` is the [`REGISTRY`] names joined with
///   `" or "`, so a future backend appears automatically.
/// * Otherwise each backend is tried in order (the SESSION's own first, see
///   [`plan_focus`]) and **the first success wins**. Not found is not final: the
///   session may live in an outer multiplexer. A detection that is `inside` but
///   not `reachable` counts as that backend's failure with its reason, and its
///   `focus` is not called.
/// * On success the winner's client, if it has one, is revealed in the other
///   backends (`reveal_client`). The reveal is best effort and can only add a
///   note, never turn the success into an error.
/// * On success: `"Switched to <label>"`, or `"Switched to <label> (<note>)"`
///   when the focus landed short of the exact pane. The matched identity goes to
///   the log only, which keeps the success text byte-identical to the tmux-only
///   days.
/// * When all fail: a single tried backend gives its reason verbatim (so tmux
///   failures are byte-identical); several give `"<name>: <reason>"` joined with
///   `"; "`, in the order tried.
pub fn focus_with(
    backends: &[Detected],
    env: &EnvSnapshot,
    session: &ClaudeSession,
    label: &str,
) -> String {
    focus_with_procs(backends, env, session, label, &ProcFs::system())
}

/// [`focus_with`] over an explicit `/proc`.
pub(crate) fn focus_with_procs(
    backends: &[Detected],
    env: &EnvSnapshot,
    session: &ClaudeSession,
    label: &str,
    procfs: &ProcFs,
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
                let outcome = reveal_client(detected, backends, env, procfs, outcome);
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

/// After `winner` focused the session, bring forward the terminal that shows
/// it. **Best effort: it can add a note to `outcome`, never an error.**
///
/// A pane that a headless server owns (herdr) is on no terminal the user can
/// see. The server's CLIENT process shows it, and that client usually sits in a
/// pane of another multiplexer in the plan (tmux), which is what the user is
/// looking at. So: ask the winner for its client terminals
/// ([`MultiplexerBackend::client_ttys`]) and have each other reachable backend
/// try to select the pane that shows one ([`MultiplexerBackend::focus_tty`]).
///
/// * Nothing to reveal into (the winner is the only backend), or a winner with
///   no client concept (tmux): nothing happens, no `/proc` walk, no note.
/// * The winner has clients and one is shown in a pane of another backend: that
///   pane is selected, no note. With several clients the other backend picks
///   deterministically (the first match in its own listing order); it can only
///   choose which already-correct view to show, never route input elsewhere,
///   because the winner's focus was exact.
/// * No client attached, a client in a terminal no other backend manages, or a
///   reveal that failed: success with a note saying so. The note never repeats a
///   backend's own "not found" error, because the session WAS focused.
fn reveal_client(
    winner: &Detected,
    backends: &[Detected],
    env: &EnvSnapshot,
    procfs: &ProcFs,
    mut outcome: FocusOutcome,
) -> FocusOutcome {
    let winner_name = winner.backend.name();
    let others: Vec<&Detected> = backends
        .iter()
        .filter(|d| d.backend.name() != winner_name && d.detection.reachable.is_ok())
        .collect();
    if others.is_empty() {
        return outcome;
    }
    let Some(ttys) = winner.backend.client_ttys(&|comm| procfs.scan(comm)) else {
        return outcome;
    };

    let names = others
        .iter()
        .map(|d| d.backend.name())
        .collect::<Vec<_>>()
        .join(" or ");
    let note = if ttys.is_empty() {
        Some(format!(
            "focused in {winner_name}; no {winner_name} client is attached"
        ))
    } else {
        let mut failed = false;
        let mut revealed = false;
        for other in &others {
            match other.backend.focus_tty(env, &ttys) {
                Ok(_) => {
                    revealed = true;
                    break;
                }
                Err(MuxError::Unsupported(_) | MuxError::NotFound(_)) => {}
                Err(e) => {
                    tracing::warn!(
                        backend = other.backend.name(),
                        reason = %e,
                        "could not reveal the terminal of the focused session's client"
                    );
                    failed = true;
                }
            }
        }
        match (revealed, failed) {
            (true, _) => None,
            (false, true) => Some(format!(
                "focused in {winner_name}; could not reveal its {winner_name} client in {names}"
            )),
            (false, false) => Some(format!(
                "focused in {winner_name}; its {winner_name} client is not in a {names} pane"
            )),
        }
    };
    if let Some(note) = note {
        outcome.partial = Some(match outcome.partial.take() {
            Some(earlier) => format!("{earlier}; {note}"),
            None => note,
        });
    }
    outcome
}

/// [`focus_with`] over the detected process environment. **The
/// real-environment entry, called only from `normal.rs` and `detail.rs`.**
///
/// There is ONE entry for both focus paths, the Sessions panel and the
/// tab-switch, so they cannot route differently (`tests/focus_entry_guard.rs`
/// pins that no UI code reaches a backend another way).
pub fn focus_message(session: &ClaudeSession, label: &str) -> String {
    focus_message_in(
        &ProcFs::system(),
        REGISTRY,
        &EnvSnapshot::from_process(),
        std::process::id(),
        session,
        label,
    )
}

/// [`focus_message`] with its inputs explicit: the `/proc` to read, the
/// registry, the TUI's own environment and pid. The whole decision, from the
/// session's `/proc` facts to the status text, runs in a test against a process
/// table laid out in a directory and backends pointed at fixture scripts.
pub(crate) fn focus_message_in(
    procfs: &ProcFs,
    registry: &[&'static dyn MultiplexerBackend],
    tui_env: &EnvSnapshot,
    tui_pid: u32,
    session: &ClaudeSession,
    label: &str,
) -> String {
    let tui_ancestry = procfs.chain(tui_pid);
    let context = procfs.session_context(session.pid);
    let plan = plan_focus(registry, tui_env, &tui_ancestry, &context);
    focus_with_procs(&plan.backends, &plan.env, session, label, procfs)
}

/// Accept one connection on `listener`, giving up after `limit`. For the test
/// listeners only: a blocking `accept` hangs the whole suite when the client
/// under test fails before it connects, so a regression would look like a CI
/// timeout (review IN-05). The accepted stream gets a read timeout for the same
/// reason.
#[cfg(all(test, unix))]
pub(crate) fn accept_within(
    listener: &std::os::unix::net::UnixListener,
    limit: std::time::Duration,
) -> std::io::Result<std::os::unix::net::UnixStream> {
    let deadline = std::time::Instant::now() + limit;
    listener.set_nonblocking(true)?;
    loop {
        match listener.accept() {
            Ok((stream, _)) => {
                stream.set_nonblocking(false)?;
                stream.set_read_timeout(Some(limit))?;
                return Ok(stream);
            }
            Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                if std::time::Instant::now() >= deadline {
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::TimedOut,
                        "no client connected to the test listener",
                    ));
                }
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
            Err(e) => return Err(e),
        }
    }
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
            "the refusal must name the multiplexer's own option parser as the reason; got {err:?}"
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
        let backends = order_innermost(REGISTRY, &env, &[]);
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
            order_innermost(REGISTRY, &EnvSnapshot::from_pairs(pairs), &[])
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
        let found = order_innermost(&doubled, &env, &[]);
        assert_eq!(found.len(), 1);
        assert_eq!(found[0].backend.name(), "tmux");
    }

    /// IN-01: `inside` is a deliberate part of [`Detection`] (D-02: a backend may
    /// have something to say about the environment without the TUI being inside
    /// it), but no in-tree backend returns `inside: false`. This fixture is the
    /// one that exercises the branch, so it cannot rot unnoticed.
    #[test]
    fn order_innermost_skips_a_backend_that_is_not_inside() {
        struct Elsewhere;
        impl MultiplexerBackend for Elsewhere {
            fn name(&self) -> &'static str {
                "elsewhere"
            }
            fn detect(&self, _env: &EnvSnapshot) -> Option<Detection> {
                Some(Detection {
                    inside: false,
                    reachable: Ok(()),
                })
            }
        }
        static ELSEWHERE: Elsewhere = Elsewhere;

        let env = EnvSnapshot::from_pairs(&[("TMUX", "x")]);
        let registry: [&'static dyn MultiplexerBackend; 2] = [&ELSEWHERE, &tmux::TMUX];
        let found = order_innermost(&registry, &env, &[]);
        let names: Vec<&str> = found.iter().map(|d| d.backend.name()).collect();
        assert_eq!(names, vec!["tmux"]);
    }

    // ---- nesting precedence (MUX-02, D-07) ---------------------------------

    const TMUX_ENV: (&str, &str) = ("TMUX", "/tmp/tmux-1000/default,1,0");
    const HERDR_ENV_ON: (&str, &str) = ("HERDR_ENV", "1");
    const HERDR_SOCK: (&str, &str) = ("HERDR_SOCKET_PATH", "/tmp/h.sock");

    /// A nearest-first chain with synthetic pids.
    fn chain(comms: &[&str]) -> Vec<(u32, String)> {
        comms
            .iter()
            .enumerate()
            .map(|(i, c)| (100 + i as u32, (*c).to_string()))
            .collect()
    }

    fn ordered(pairs: &[(&str, &str)], comms: &[&str]) -> Vec<&'static str> {
        order_innermost(REGISTRY, &EnvSnapshot::from_pairs(pairs), &chain(comms))
            .iter()
            .map(|d| d.backend.name())
            .collect()
    }

    #[test]
    fn parse_stat_ppid_reads_the_field_after_the_state() {
        assert_eq!(parse_stat_ppid("123 (bash) S 77 77 77 0"), Some(77));
        assert_eq!(parse_stat_ppid("5 (tmux: server) S 1 5 5 0"), Some(1));
    }

    #[test]
    fn parse_stat_ppid_closes_the_comm_at_the_last_paren() {
        assert_eq!(parse_stat_ppid("9 (a) b) S 4 9 9 0"), Some(4));
    }

    #[test]
    fn parse_stat_ppid_rejects_truncated_and_non_numeric_input() {
        assert_eq!(parse_stat_ppid(""), None);
        assert_eq!(parse_stat_ppid("9 (a) S"), None);
        assert_eq!(parse_stat_ppid("9 (a"), None);
        assert_eq!(parse_stat_ppid("9 (a) S x 1"), None);
        assert_eq!(parse_stat_ppid("9 (a) S -3 1"), None);
        assert_eq!(parse_stat_ppid("9 (a) S 99999999999 1"), None);
    }

    #[test]
    fn parse_stat_comm_keeps_spaces_and_colons() {
        assert_eq!(
            parse_stat_comm("5 (tmux: server) S 1 5 5 0"),
            Some("tmux: server")
        );
        assert_eq!(parse_stat_comm("9 (a) b) S 4"), Some("a) b"));
        assert_eq!(parse_stat_comm("no parens"), None);
    }

    #[test]
    fn order_innermost_herdr_alone() {
        assert_eq!(
            ordered(&[HERDR_ENV_ON, HERDR_SOCK], &["fish", "herdr", "systemd"]),
            vec!["herdr"]
        );
    }

    #[test]
    fn order_innermost_tmux_in_herdr_picks_tmux() {
        assert_eq!(
            ordered(
                &[TMUX_ENV, HERDR_ENV_ON, HERDR_SOCK],
                &["sleep", "tmux: server", "systemd"]
            ),
            vec!["tmux", "herdr"]
        );
    }

    #[test]
    fn order_innermost_herdr_in_tmux_picks_herdr() {
        assert_eq!(
            ordered(
                &[HERDR_ENV_ON, HERDR_SOCK],
                &["bash", "herdr", "herdr", "tmux: server"]
            ),
            vec!["herdr"],
            "herdr strips $TMUX from its panes, so only herdr is a candidate"
        );
    }

    #[test]
    fn order_innermost_nearer_herdr_wins_over_tmux() {
        assert_eq!(
            ordered(
                &[TMUX_ENV, HERDR_ENV_ON, HERDR_SOCK],
                &["bash", "herdr", "tmux: server"]
            ),
            vec!["herdr", "tmux"]
        );
    }

    #[test]
    fn order_innermost_empty_chain_keeps_the_static_order() {
        assert_eq!(
            ordered(&[TMUX_ENV, HERDR_ENV_ON, HERDR_SOCK], &[]),
            vec!["tmux", "herdr"]
        );
    }

    #[test]
    fn order_innermost_ignores_an_ancestor_whose_env_gate_fails() {
        assert_eq!(
            ordered(&[TMUX_ENV], &["bash", "herdr"]),
            vec!["tmux"],
            "the env gate wins over the ancestor"
        );
        assert!(
            ordered(&[], &["bash", "herdr", "tmux: server"]).is_empty(),
            "ancestry alone never makes a backend a candidate"
        );
    }

    #[test]
    fn order_innermost_never_takes_a_tmux_client_for_a_server() {
        assert_eq!(
            ordered(
                &[TMUX_ENV, HERDR_ENV_ON, HERDR_SOCK],
                &["bash", "tmux: client"]
            ),
            vec!["tmux", "herdr"]
        );
        assert_eq!(
            ordered(
                &[TMUX_ENV, HERDR_ENV_ON, HERDR_SOCK],
                &["bash", "tmux: client", "herdr"]
            ),
            vec!["herdr", "tmux"],
            "the client is skipped, the next real server decides"
        );
    }

    #[test]
    fn order_innermost_a_chain_naming_herdr_twice_yields_herdr_once() {
        assert_eq!(
            ordered(&[HERDR_ENV_ON, HERDR_SOCK], &["herdr", "herdr"]),
            vec!["herdr"]
        );
        assert_eq!(
            ordered(
                &[TMUX_ENV, HERDR_ENV_ON, HERDR_SOCK],
                &["herdr", "herdr", "tmux: server"]
            ),
            vec!["herdr", "tmux"]
        );
    }

    /// Env markers and the server comm of every backend. A backend added to
    /// [`REGISTRY`] without a row here fails the test below: this is what makes
    /// "add a backend" carry its own detection and nesting fixtures.
    type BackendFixture = (
        &'static str,
        &'static [(&'static str, &'static str)],
        &'static str,
    );
    const BACKEND_FIXTURES: &[BackendFixture] = &[
        ("tmux", &[TMUX_ENV], "tmux: server"),
        ("herdr", &[HERDR_ENV_ON, HERDR_SOCK], "herdr"),
    ];

    #[test]
    fn every_registered_backend_is_selected_by_its_own_fixtures() {
        let registry_names: Vec<&str> = REGISTRY.iter().map(|b| b.name()).collect();
        let fixture_names: Vec<&str> = BACKEND_FIXTURES.iter().map(|f| f.0).collect();
        for name in &registry_names {
            assert!(
                fixture_names.contains(name),
                "backend {name:?} is registered but has no fixture row"
            );
        }
        for name in &fixture_names {
            assert!(
                registry_names.contains(name),
                "fixture row {name:?} names no registered backend"
            );
        }

        let every_marker: Vec<(&str, &str)> = BACKEND_FIXTURES
            .iter()
            .flat_map(|f| f.1.iter().copied())
            .collect();
        for (name, env, comm) in BACKEND_FIXTURES {
            let alone = order_innermost(REGISTRY, &EnvSnapshot::from_pairs(env), &[]);
            assert_eq!(
                alone.iter().map(|d| d.backend.name()).collect::<Vec<_>>(),
                vec![*name],
                "{name}: its own env markers alone must select it"
            );

            let nested = order_innermost(
                REGISTRY,
                &EnvSnapshot::from_pairs(&every_marker),
                &[(2, (*comm).to_string())],
            );
            assert_eq!(
                nested.first().map(|d| d.backend.name()),
                Some(*name),
                "{name}: with every marker set, its server comm must put it first"
            );
        }
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
        assert_eq!(msg, "tmux: could not open a session — opened in kitty");
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
            "tmux: could not open a session; no terminal emulator found (set $TERMINAL)"
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
        assert_eq!(msg, "tmux: refused for safety — opened in kitty");
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
            "herdr: could not open a session; no terminal emulator found (set $TERMINAL)"
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
        assert_eq!(msg, "herdr: not reachable — opened in kitty");
        assert_eq!(fake.open_calls(), 0);
    }

    #[test]
    fn every_error_class_has_a_short_launch_phrase_free_of_the_raw_reason() {
        let raw =
            "herdr socket connect: /run/user/1000/herdr.sock: Permission denied (os error 13)";
        for error in [
            MuxError::NotFound(raw.into()),
            MuxError::Unsupported(raw.into()),
            MuxError::RefusedForSafety(raw.into()),
            MuxError::Failed(raw.into()),
        ] {
            let phrase = error.launch_phrase();
            assert!(!phrase.is_empty() && phrase.len() <= 32, "{phrase}");
            assert!(
                !phrase.contains("socket") && !phrase.contains('/'),
                "{phrase}"
            );
        }
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

    // ---- focus routing: where the SESSION lives, not where the TUI runs -----
    //
    // Debug session `herdr-session-focus-from-tmux`. Measured 2026-11-10: the
    // TUI ran in a tmux pane (`$TMUX` set, no `HERDR_*`), while the sessions ran
    // under a herdr SERVER that is a sibling daemon under systemd, not an
    // ancestor of the TUI: claude(672012) -> fish -> herdr(`herdr server`,
    // 671927) -> systemd, environ `HERDR_ENV=1`, `HERDR_SOCKET_PATH=...`, no
    // `$TMUX`, stdin `/dev/pts/3`. Focus offered the session only to tmux, which
    // answered "TTY pts/3 not found in any tmux pane (session may have exited)"
    // because pts/3 is owned by the herdr server and tmux has no such pane.

    /// herdr's default control socket, as the measured session's environ had it.
    const SESSION_HERDR_SOCKET: &str = "/home/blk/.config/herdr/herdr.sock";

    /// The tmux symptom, verbatim from the user's status line.
    const TMUX_NOT_FOUND: &str = "TTY pts/3 not found in any tmux pane (session may have exited)";

    /// The TUI in a plain tmux pane: `$TMUX` set, no `HERDR_*`, and a chain that
    /// ends in the tmux server. herdr is nowhere in it.
    fn tui_in_tmux() -> (EnvSnapshot, Vec<(u32, String)>) {
        (
            EnvSnapshot::from_pairs(&[TMUX_ENV]),
            chain(&["gsd-meta-manager", "fish", "tmux: server", "systemd"]),
        )
    }

    /// The TUI inside a herdr pane (the Phase 33 SC1 topology): `HERDR_*` set,
    /// no `$TMUX`, a chain through the herdr server.
    fn tui_in_herdr() -> (EnvSnapshot, Vec<(u32, String)>) {
        (
            EnvSnapshot::from_pairs(&[HERDR_ENV_ON, HERDR_SOCK]),
            chain(&["gsd-meta-manager", "fish", "herdr", "systemd"]),
        )
    }

    /// A session hosted by a herdr server: the markers and chain measured for
    /// pid 672012, with no tmux anywhere.
    fn session_in_herdr() -> SessionContext {
        SessionContext {
            env: EnvSnapshot::from_pairs(&[
                HERDR_ENV_ON,
                ("HERDR_SOCKET_PATH", SESSION_HERDR_SOCKET),
                ("HERDR_WORKSPACE_ID", "w2"),
            ]),
            ancestry: chain(&["claude", "fish", "herdr", "systemd", "systemd"]),
        }
    }

    /// A session in a tmux pane: `$TMUX` set and the tmux server in its chain.
    fn session_in_tmux() -> SessionContext {
        SessionContext {
            env: EnvSnapshot::from_pairs(&[TMUX_ENV]),
            ancestry: chain(&["claude", "fish", "tmux: server", "systemd"]),
        }
    }

    fn plan_names(plan: &FocusPlan) -> Vec<&'static str> {
        plan.backends.iter().map(|d| d.backend.name()).collect()
    }

    /// A backend that, like the real ones, is a candidate only when ITS OWN
    /// marker is in the snapshot it is shown, names its server's comm for the
    /// nesting rule, and records every `focus` call with the herdr socket the
    /// snapshot it acted on carried. No process is spawned and no socket is
    /// opened, so it runs where no multiplexer exists.
    struct Gated {
        name: &'static str,
        marker: fn(&EnvSnapshot) -> bool,
        server_comm: &'static str,
        focus_result: Result<FocusOutcome, MuxError>,
        log: Arc<Mutex<Vec<String>>>,
    }

    impl Gated {
        fn leak(
            name: &'static str,
            marker: fn(&EnvSnapshot) -> bool,
            server_comm: &'static str,
            focus_result: Result<FocusOutcome, MuxError>,
            log: &Arc<Mutex<Vec<String>>>,
        ) -> &'static Gated {
            Box::leak(Box::new(Gated {
                name,
                marker,
                server_comm,
                focus_result,
                log: Arc::clone(log),
            }))
        }
    }

    impl MultiplexerBackend for Gated {
        fn name(&self) -> &'static str {
            self.name
        }
        fn detect(&self, env: &EnvSnapshot) -> Option<Detection> {
            (self.marker)(env).then_some(Detection {
                inside: true,
                reachable: Ok(()),
            })
        }
        fn focus(
            &self,
            env: &EnvSnapshot,
            _session: &ClaudeSession,
        ) -> Result<FocusOutcome, MuxError> {
            self.log.lock().unwrap().push(format!(
                "focus:{} sock={}",
                self.name,
                env.get("HERDR_SOCKET_PATH").unwrap_or("-")
            ));
            self.focus_result.clone()
        }
        fn is_server_comm(&self, comm: &str) -> bool {
            comm == self.server_comm
        }
    }

    /// **The reported bug, routing half, through the real registry.** A TUI in
    /// tmux and a session under a sibling herdr server: the plan must offer
    /// herdr, first, and hand it the SESSION's socket (the herdr backend reads
    /// the socket only from the snapshot it acts on, and the TUI's has none).
    #[test]
    fn plan_focus_routes_a_herdr_session_to_herdr_when_the_tui_runs_in_tmux() {
        let (tui_env, tui_ancestry) = tui_in_tmux();
        let plan = plan_focus(REGISTRY, &tui_env, &tui_ancestry, &session_in_herdr());

        assert_eq!(
            plan_names(&plan).first().copied(),
            Some("herdr"),
            "the session lives under herdr (HERDR_ENV=1, a `herdr` server in its own chain), \
             so herdr must be tried FIRST even though the TUI itself runs in tmux; got {:?}, \
             the TUI-derived list, which can only ever end in the tmux tty error",
            plan_names(&plan)
        );
        assert_eq!(
            plan.env.get("HERDR_SOCKET_PATH"),
            Some(SESSION_HERDR_SOCKET),
            "herdr must act on the SESSION's own server socket: the TUI's snapshot has none, \
             and the herdr CLI edge refuses to spawn without one"
        );
    }

    /// The same defect mirrored: a TUI inside herdr and a session in a tmux
    /// pane. The TUI-derived list is `[herdr]` alone, so tmux (which owns the
    /// session's tty) is never asked.
    #[test]
    fn plan_focus_routes_a_tmux_session_to_tmux_when_the_tui_runs_in_herdr() {
        let (tui_env, tui_ancestry) = tui_in_herdr();
        let plan = plan_focus(REGISTRY, &tui_env, &tui_ancestry, &session_in_tmux());

        assert_eq!(
            plan_names(&plan).first().copied(),
            Some("tmux"),
            "the session lives in tmux ($TMUX set, `tmux: server` in its own chain), so tmux \
             must be tried first even though the TUI runs in herdr; got {:?}",
            plan_names(&plan)
        );
    }

    /// **The reported bug, end to end** through `plan_focus` and `focus_with`
    /// with fake backends (no multiplexer): the user must get the herdr focus,
    /// not tmux's "not found" text, and herdr must have acted on the session's
    /// socket.
    #[test]
    fn a_herdr_session_is_focused_in_herdr_instead_of_failing_in_tmux() {
        let log = new_log();
        let tmux = Gated::leak(
            "tmux",
            |env| env.get("TMUX").is_some(),
            "tmux: server",
            Err(MuxError::NotFound(TMUX_NOT_FOUND.to_string())),
            &log,
        );
        let herdr = Gated::leak(
            "herdr",
            |env| env.get("HERDR_ENV") == Some("1"),
            "herdr",
            Ok(FocusOutcome {
                by: MatchedBy::SessionId,
                partial: None,
            }),
            &log,
        );
        let registry: [&'static dyn MultiplexerBackend; 2] = [tmux, herdr];

        let (tui_env, tui_ancestry) = tui_in_tmux();
        let plan = plan_focus(&registry, &tui_env, &tui_ancestry, &session_in_herdr());
        let message = focus_with(&plan.backends, &plan.env, &session(), "sentriq");

        assert!(
            !message.contains("not found in any tmux pane"),
            "a session that runs under herdr was reported missing from tmux: {message:?}"
        );
        assert!(
            message.starts_with("Switched to sentriq"),
            "focusing a herdr-hosted session must succeed through herdr; got {message:?}"
        );
        assert_eq!(
            log.lock().unwrap().first().map(String::as_str),
            Some(format!("focus:herdr sock={SESSION_HERDR_SOCKET}").as_str()),
            "herdr must be the first backend asked, acting on the session's socket; calls: {:?}",
            log.lock().unwrap()
        );
    }

    /// **Guard, passes before and after the fix (SC2).** A tmux-only user (TUI
    /// and session both in tmux) keeps exactly the old list: tmux alone.
    #[test]
    fn plan_focus_keeps_tmux_only_routing_for_a_tmux_session_in_a_tmux_tui() {
        let (tui_env, tui_ancestry) = tui_in_tmux();
        let plan = plan_focus(REGISTRY, &tui_env, &tui_ancestry, &session_in_tmux());
        assert_eq!(plan_names(&plan), vec!["tmux"]);
    }

    /// **Guard, passes before and after the fix.** When nothing can be read
    /// about the session (non-Linux, another uid, an exited process) the plan is
    /// the TUI-derived one, over the TUI's own environment, as before.
    #[test]
    fn plan_focus_without_session_facts_falls_back_to_the_tuis_own_backends() {
        let (tui_env, tui_ancestry) = tui_in_tmux();
        let plan = plan_focus(
            REGISTRY,
            &tui_env,
            &tui_ancestry,
            &SessionContext::default(),
        );
        assert_eq!(plan_names(&plan), vec!["tmux"]);
        assert_eq!(plan.env.get("TMUX"), tui_env.get("TMUX"));
        assert_eq!(plan.env.get("HERDR_SOCKET_PATH"), None);
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
            let stream = accept_within(&listener, std::time::Duration::from_secs(10))
                .expect("a client connects within 10 s");
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
        let backends = order_innermost(REGISTRY, &env, &[]);
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
            msg, "herdr: could not open a session — opened in kitty",
            "the raw herdr error stays out of the status line"
        );
        assert_eq!(spawned.len(), 1);
    }

    // ---- the session's own facts: environ block, overlay, plan ---------------

    /// A `/proc/<pid>/environ` block from `KEY=VALUE` entries.
    fn environ_block(entries: &[&str]) -> Vec<u8> {
        entries
            .iter()
            .flat_map(|e| e.bytes().chain(std::iter::once(0)))
            .collect()
    }

    #[test]
    fn an_environ_block_keeps_only_the_detection_keys_and_the_first_entry_of_each() {
        let block = environ_block(&[
            "PATH=/usr/bin",
            "HERDR_ENV=1",
            "HERDR_PANE_ID=w2:p1",
            "HERDR_SOCKET_PATH=/home/blk/.config/herdr/herdr.sock",
            "HERDR_WORKSPACE_ID=w2",
            "HERDR_ENV=0",
            "SECRET_TOKEN=hunter2",
        ]);
        let env = EnvSnapshot::from_environ_block(&block);
        assert_eq!(
            env.get("HERDR_ENV"),
            Some("1"),
            "getenv: the first entry wins"
        );
        assert_eq!(env.get("HERDR_WORKSPACE_ID"), Some("w2"));
        assert_eq!(env.get("HERDR_SOCKET_PATH"), Some(SESSION_HERDR_SOCKET));
        assert_eq!(env.get("TMUX"), None);
        assert_eq!(
            env.get("HERDR_PANE_ID"),
            None,
            "a pane id is not a detection key: it can be stale after `pane move`"
        );
        assert_eq!(env.get("SECRET_TOKEN"), None, "other keys are never copied");
    }

    #[test]
    fn an_environ_block_tolerates_malformed_entries() {
        assert!(EnvSnapshot::from_environ_block(&[]).get("TMUX").is_none());
        let mut block = environ_block(&["no-equals-sign", "=novalue-key"]);
        block.extend_from_slice(b"\xff\xfe=bad-utf8-key\0");
        block.extend(environ_block(&["HERDR_ENV=1", "HERDR_WORKSPACE_ID=a=b"]));
        let env = EnvSnapshot::from_environ_block(&block);
        assert_eq!(env.get("HERDR_ENV"), Some("1"));
        assert_eq!(
            env.get("HERDR_WORKSPACE_ID"),
            Some("a=b"),
            "only the first `=` splits"
        );
    }

    /// The control-socket values come from ANOTHER process's environment.
    #[test]
    fn an_environ_block_drops_a_relative_control_socket() {
        let env = EnvSnapshot::from_environ_block(&environ_block(&[
            "HERDR_ENV=1",
            "HERDR_SOCKET_PATH=relative/herdr.sock",
            "TMUX=relative/tmux,1,0",
        ]));
        assert_eq!(
            env.get("HERDR_SOCKET_PATH"),
            None,
            "a relative path would resolve against THIS process's working directory"
        );
        assert_eq!(
            env.get("TMUX"),
            None,
            "a tmux with no absolute socket is no tmux"
        );
        assert_eq!(env.get("HERDR_ENV"), Some("1"), "the marker itself stays");
        for tmux in ["", ",1,0", "tmux,1,0"] {
            let block = environ_block(&[&format!("TMUX={tmux}")]);
            assert_eq!(
                EnvSnapshot::from_environ_block(&block).get("TMUX"),
                None,
                "TMUX={tmux:?} names no absolute socket"
            );
        }
        let ok = EnvSnapshot::from_environ_block(&environ_block(&[
            "TMUX=/tmp/tmux-1000/default,3628116,0",
        ]));
        assert_eq!(ok.get("TMUX"), Some("/tmp/tmux-1000/default,3628116,0"));
    }

    #[test]
    fn an_overlay_prefers_the_over_snapshot_and_keeps_the_rest() {
        let tui = EnvSnapshot::from_pairs(&[TMUX_ENV, ("HERDR_WORKSPACE_ID", "w2")]);
        let session = EnvSnapshot::from_pairs(&[HERDR_ENV_ON, ("HERDR_WORKSPACE_ID", "w6")]);
        let merged = tui.overlaid_with(&session);
        assert_eq!(
            merged.get("TMUX"),
            Some(TMUX_ENV.1),
            "the TUI's key survives"
        );
        assert_eq!(merged.get("HERDR_ENV"), Some("1"));
        assert_eq!(
            merged.get("HERDR_WORKSPACE_ID"),
            Some("w6"),
            "the session wins"
        );
        assert_eq!(tui.get("HERDR_ENV"), None, "the base is not modified");
    }

    #[test]
    fn terminal_names_are_terminals_only() {
        let rows: [(&str, Option<&str>); 9] = [
            ("/dev/pts/13", Some("pts/13")),
            ("/dev/pts/0", Some("pts/0")),
            ("/dev/tty1", Some("tty1")),
            ("/dev/ttyS0", Some("ttyS0")),
            ("/dev/null", None),
            ("/dev/pts/", None),
            ("/dev/pts/x", None),
            ("pipe:[1234]", None),
            ("socket:[99]", None),
        ];
        for (link, expected) in rows {
            assert_eq!(
                terminal_name(Path::new(link)).as_deref(),
                expected,
                "{link}: a headless server's stdin (/dev/null) must not look like a client's"
            );
        }
    }

    /// A session nested in BOTH multiplexers is ordered by its own chain, like
    /// the TUI's was.
    #[test]
    fn plan_focus_orders_a_nested_session_by_its_own_ancestry() {
        let (tui_env, tui_ancestry) = tui_in_tmux();
        let nested = SessionContext {
            env: EnvSnapshot::from_pairs(&[TMUX_ENV, HERDR_ENV_ON, HERDR_SOCK]),
            ancestry: chain(&["claude", "fish", "herdr", "tmux: server", "systemd"]),
        };
        let plan = plan_focus(REGISTRY, &tui_env, &tui_ancestry, &nested);
        assert_eq!(
            plan_names(&plan),
            vec!["herdr", "tmux"],
            "the nearer server (herdr) is tried first and tmux is not listed twice"
        );
    }

    #[test]
    fn plan_focus_appends_the_tuis_backends_after_the_sessions() {
        let (tui_env, tui_ancestry) = tui_in_tmux();
        let plan = plan_focus(REGISTRY, &tui_env, &tui_ancestry, &session_in_herdr());
        assert_eq!(
            plan_names(&plan),
            vec!["herdr", "tmux"],
            "the session's herdr first, the TUI's tmux as the fallback (not found is not final)"
        );
    }

    #[test]
    fn plan_focus_acts_on_the_session_markers_over_the_tuis() {
        let (tui_env, tui_ancestry) = tui_in_tmux();
        let plan = plan_focus(REGISTRY, &tui_env, &tui_ancestry, &session_in_herdr());
        assert_eq!(
            plan.env.get("HERDR_SOCKET_PATH"),
            Some(SESSION_HERDR_SOCKET)
        );
        assert_eq!(
            plan.env.get("TMUX"),
            tui_env.get("TMUX"),
            "the TUI's tmux stays for the reveal step"
        );

        let (herdr_tui_env, herdr_tui_ancestry) = tui_in_herdr();
        let session = SessionContext {
            env: EnvSnapshot::from_pairs(&[("TMUX", "/tmp/tmux-1000/work,2,0")]),
            ancestry: chain(&["claude", "fish", "tmux: server", "systemd"]),
        };
        let plan = plan_focus(REGISTRY, &herdr_tui_env, &herdr_tui_ancestry, &session);
        assert_eq!(plan_names(&plan), vec!["tmux", "herdr"]);
        assert_eq!(plan.env.get("TMUX"), Some("/tmp/tmux-1000/work,2,0"));
    }

    #[test]
    fn plan_focus_judges_reachability_by_the_environment_each_backend_was_detected_in() {
        let (tui_env, tui_ancestry) = tui_in_tmux();
        let no_socket = SessionContext {
            env: EnvSnapshot::from_pairs(&[HERDR_ENV_ON]),
            ancestry: chain(&["claude", "fish", "herdr", "systemd"]),
        };
        let plan = plan_focus(REGISTRY, &tui_env, &tui_ancestry, &no_socket);
        assert_eq!(plan_names(&plan), vec!["herdr", "tmux"]);
        assert_eq!(
            plan.backends[0].detection.reachable,
            Err("HERDR_SOCKET_PATH is unset".to_string()),
            "the session's herdr names no socket: reported, not guessed"
        );
        assert_eq!(plan.backends[1].detection.reachable, Ok(()));
    }

    // ---- ProcFs on a fake /proc ----------------------------------------------

    #[cfg(unix)]
    mod procfs {
        use super::*;
        use crate::multiplexer::fixtures::FakeProc;

        #[test]
        fn a_chain_is_read_nearest_first_and_stops_at_a_gap() {
            let proc = FakeProc::new();
            proc.process(1, "systemd", 0, &[], &[], None)
                .process(10, "fish", 1, &[], &[], None)
                .process(20, "claude", 10, &[], &[], None)
                .process(30, "orphan", 99, &[], &[], None);
            let fs = proc.procfs();
            assert_eq!(
                fs.chain(20),
                vec![
                    (20, "claude".to_string()),
                    (10, "fish".to_string()),
                    (1, "systemd".to_string())
                ]
            );
            assert_eq!(
                fs.chain(30),
                vec![(30, "orphan".to_string())],
                "pid 99 does not exist: the chain stops, it does not guess past the gap"
            );
            assert!(fs.chain(555).is_empty());
            assert!(ProcFs::none().chain(20).is_empty());
        }

        #[test]
        fn a_session_context_reads_the_markers_and_the_chain_of_the_session() {
            let proc = FakeProc::new();
            proc.process(1, "systemd", 0, &[], &[], None)
                .process(
                    671927,
                    "herdr",
                    1,
                    &["herdr", "server"],
                    &[],
                    Some("/dev/null"),
                )
                .process(671995, "fish", 671927, &["fish"], &[], Some("/dev/pts/3"))
                .process(
                    672012,
                    "claude",
                    671995,
                    &["claude"],
                    &[
                        "HERDR_ENV=1",
                        "HERDR_SOCKET_PATH=/home/blk/.config/herdr/herdr.sock",
                        "HERDR_PANE_ID=w2:p1",
                    ],
                    Some("/dev/pts/3"),
                );
            let context = proc.procfs().session_context(672012);
            assert_eq!(context.env.get("HERDR_ENV"), Some("1"));
            assert_eq!(
                context.env.get("HERDR_SOCKET_PATH"),
                Some(SESSION_HERDR_SOCKET)
            );
            let comms: Vec<&str> = context.ancestry.iter().map(|(_, c)| c.as_str()).collect();
            assert_eq!(comms, vec!["claude", "fish", "herdr", "systemd"]);
        }

        /// Another uid's process, or one that exited: nothing can be read, and
        /// the answer is "unknown", which plan_focus turns into the TUI-derived
        /// plan.
        #[test]
        fn an_unreadable_session_is_unknown_not_an_error() {
            let proc = FakeProc::new();
            let context = proc.procfs().session_context(424242);
            assert!(context.env.get("HERDR_ENV").is_none());
            assert!(context.ancestry.is_empty());
            let none = ProcFs::none().session_context(1);
            assert!(none.ancestry.is_empty());
        }

        #[test]
        fn a_scan_returns_only_the_matching_comm_with_argv_and_terminal() {
            let proc = FakeProc::new();
            proc.process(1, "systemd", 0, &[], &[], None)
                .process(5, "herdr", 1, &["herdr", "server"], &[], Some("/dev/null"))
                .process(
                    9,
                    "herdr",
                    1,
                    &["herdr", "session", "attach", "default"],
                    &[],
                    Some("/dev/pts/13"),
                )
                .process(11, "fish", 1, &["fish"], &[], Some("/dev/pts/13"))
                .process(12, "herdr", 1, &["herdr"], &[], None);
            let found = proc.procfs().scan("herdr");
            let shape: Vec<(u32, Vec<&str>, Option<&str>)> = found
                .iter()
                .map(|e| {
                    (
                        e.pid,
                        e.argv.iter().map(String::as_str).collect(),
                        e.tty.as_deref(),
                    )
                })
                .collect();
            assert_eq!(
                shape,
                vec![
                    (5, vec!["herdr", "server"], None),
                    (
                        9,
                        vec!["herdr", "session", "attach", "default"],
                        Some("pts/13")
                    ),
                    (12, vec!["herdr"], None),
                ]
            );
            assert!(ProcFs::none().scan("herdr").is_empty());
        }
    }

    // ---- the reveal step, with fake backends ---------------------------------

    /// A backend that can focus, report its clients and reveal a client's
    /// terminal, recording each call. No process, no socket.
    struct Revealing {
        name: &'static str,
        focus_result: Result<FocusOutcome, MuxError>,
        clients: Option<Vec<String>>,
        reveal: Result<FocusOutcome, MuxError>,
        log: Arc<Mutex<Vec<String>>>,
    }

    impl Revealing {
        fn leak(
            name: &'static str,
            clients: Option<&[&str]>,
            reveal: Result<FocusOutcome, MuxError>,
            log: &Arc<Mutex<Vec<String>>>,
        ) -> &'static Revealing {
            Box::leak(Box::new(Revealing {
                name,
                focus_result: Ok(FocusOutcome {
                    by: MatchedBy::SessionId,
                    partial: None,
                }),
                clients: clients.map(|c| c.iter().map(|s| (*s).to_string()).collect()),
                reveal,
                log: Arc::clone(log),
            }))
        }
    }

    impl MultiplexerBackend for Revealing {
        fn name(&self) -> &'static str {
            self.name
        }
        fn detect(&self, _env: &EnvSnapshot) -> Option<Detection> {
            Some(Detection {
                inside: true,
                reachable: Ok(()),
            })
        }
        fn focus(&self, _: &EnvSnapshot, _: &ClaudeSession) -> Result<FocusOutcome, MuxError> {
            self.log
                .lock()
                .unwrap()
                .push(format!("focus:{}", self.name));
            self.focus_result.clone()
        }
        fn client_ttys(&self, _scan: &ProcScan<'_>) -> Option<Vec<String>> {
            self.log
                .lock()
                .unwrap()
                .push(format!("client_ttys:{}", self.name));
            self.clients.clone()
        }
        fn focus_tty(&self, _: &EnvSnapshot, ttys: &[String]) -> Result<FocusOutcome, MuxError> {
            self.log
                .lock()
                .unwrap()
                .push(format!("focus_tty:{}:{}", self.name, ttys.join(",")));
            self.reveal.clone()
        }
    }

    fn revealed() -> Result<FocusOutcome, MuxError> {
        Ok(FocusOutcome {
            by: MatchedBy::Tty,
            partial: None,
        })
    }

    fn run_reveal(backends: &[Detected]) -> String {
        focus_with_procs(
            backends,
            &EnvSnapshot::default(),
            &session(),
            "alpha",
            &ProcFs::none(),
        )
    }

    /// The reveal tests must never read the real `/proc`: the fakes ignore the
    /// scan, and `ProcFs::none()` has no root.
    #[test]
    fn a_revealed_client_adds_no_note() {
        let log = new_log();
        let herdr = Revealing::leak("herdr", Some(&["pts/13"]), revealed(), &log);
        let tmux = Revealing::leak("tmux", None, revealed(), &log);
        assert_eq!(
            run_reveal(&[detected(herdr), detected(tmux)]),
            "Switched to alpha"
        );
        assert_eq!(
            *log.lock().unwrap(),
            vec![
                "focus:herdr".to_string(),
                "client_ttys:herdr".to_string(),
                "focus_tty:tmux:pts/13".to_string()
            ],
            "focus first, then the winner's clients, then the other backend reveals them"
        );
    }

    #[test]
    fn several_clients_are_all_offered_to_the_revealing_backend() {
        let log = new_log();
        let herdr = Revealing::leak("herdr", Some(&["pts/13", "pts/20"]), revealed(), &log);
        let tmux = Revealing::leak("tmux", None, revealed(), &log);
        run_reveal(&[detected(herdr), detected(tmux)]);
        assert!(
            log.lock()
                .unwrap()
                .contains(&"focus_tty:tmux:pts/13,pts/20".to_string()),
            "{:?}",
            log.lock().unwrap()
        );
    }

    #[test]
    fn a_client_in_a_terminal_no_other_backend_manages_is_a_note_not_an_error() {
        let log = new_log();
        let herdr = Revealing::leak("herdr", Some(&["pts/20"]), revealed(), &log);
        let tmux = Revealing::leak(
            "tmux",
            None,
            Err(MuxError::NotFound(
                "no tmux pane shows that terminal".into(),
            )),
            &log,
        );
        let message = run_reveal(&[detected(herdr), detected(tmux)]);
        assert_eq!(
            message,
            "Switched to alpha (focused in herdr; its herdr client is not in a tmux pane)"
        );
        assert!(
            !message.contains("not found"),
            "the session WAS focused: no backend's not-found text may reach the user: {message}"
        );
    }

    #[test]
    fn no_attached_client_is_a_note_and_nothing_is_revealed() {
        let log = new_log();
        let herdr = Revealing::leak("herdr", Some(&[]), revealed(), &log);
        let tmux = Revealing::leak("tmux", None, revealed(), &log);
        assert_eq!(
            run_reveal(&[detected(herdr), detected(tmux)]),
            "Switched to alpha (focused in herdr; no herdr client is attached)"
        );
        assert!(
            !log.lock()
                .unwrap()
                .iter()
                .any(|c| c.starts_with("focus_tty")),
            "with no client there is no terminal to reveal: {:?}",
            log.lock().unwrap()
        );
    }

    #[test]
    fn a_failing_reveal_is_a_note_that_keeps_the_reason_out_of_the_status_line() {
        let log = new_log();
        let herdr = Revealing::leak("herdr", Some(&["pts/13"]), revealed(), &log);
        let tmux = Revealing::leak(
            "tmux",
            None,
            Err(MuxError::Failed(
                "tmux list-panes exited exit status: 1: no server running on /tmp/x".into(),
            )),
            &log,
        );
        assert_eq!(
            run_reveal(&[detected(herdr), detected(tmux)]),
            "Switched to alpha (focused in herdr; could not reveal its herdr client in tmux)"
        );
    }

    #[test]
    fn a_winner_without_a_client_concept_reveals_nothing() {
        let log = new_log();
        let tmux = Revealing::leak("tmux", None, revealed(), &log);
        let herdr = Revealing::leak("herdr", Some(&["pts/13"]), revealed(), &log);
        assert_eq!(
            run_reveal(&[detected(tmux), detected(herdr)]),
            "Switched to alpha"
        );
        assert!(
            !log.lock()
                .unwrap()
                .iter()
                .any(|c| c.starts_with("focus_tty")),
            "the winner (tmux) has no client terminal to reveal: {:?}",
            log.lock().unwrap()
        );
    }

    /// A TUI inside herdr and a session in herdr: there is no other multiplexer
    /// to reveal into, so no `/proc` walk and no note.
    #[test]
    fn a_lone_winner_has_nothing_to_reveal_into() {
        let log = new_log();
        let herdr = Revealing::leak("herdr", Some(&["pts/13"]), revealed(), &log);
        assert_eq!(run_reveal(&[detected(herdr)]), "Switched to alpha");
        assert_eq!(*log.lock().unwrap(), vec!["focus:herdr".to_string()]);
    }

    #[test]
    fn the_reveal_note_follows_the_winners_own_note() {
        let log = new_log();
        let herdr: &'static Revealing = Box::leak(Box::new(Revealing {
            name: "herdr",
            focus_result: Ok(FocusOutcome {
                by: MatchedBy::Cwd,
                partial: Some("matched by working directory only".to_string()),
            }),
            clients: Some(vec![]),
            reveal: revealed(),
            log: Arc::clone(&log),
        }));
        let tmux = Revealing::leak("tmux", None, revealed(), &log);
        assert_eq!(
            run_reveal(&[detected(herdr), detected(tmux)]),
            "Switched to alpha (matched by working directory only; \
             focused in herdr; no herdr client is attached)"
        );
    }

    #[test]
    fn an_unreachable_backend_is_not_asked_to_reveal() {
        let log = new_log();
        let herdr = Revealing::leak("herdr", Some(&["pts/13"]), revealed(), &log);
        let tmux = Revealing::leak("tmux", None, revealed(), &log);
        let unreachable_tmux = Detected {
            backend: tmux,
            detection: Detection {
                inside: true,
                reachable: Err("no server".to_string()),
            },
        };
        assert_eq!(
            run_reveal(&[detected(herdr), unreachable_tmux]),
            "Switched to alpha"
        );
        assert!(
            !log.lock()
                .unwrap()
                .iter()
                .any(|c| c.starts_with("focus_tty")),
            "{:?}",
            log.lock().unwrap()
        );
    }

    #[test]
    fn a_backend_that_does_not_override_the_reveal_is_unsupported_and_never_scans() {
        struct Bare;
        impl MultiplexerBackend for Bare {
            fn name(&self) -> &'static str {
                "bare"
            }
            fn detect(&self, _env: &EnvSnapshot) -> Option<Detection> {
                None
            }
        }
        assert_eq!(
            Bare.focus_tty(&EnvSnapshot::default(), &["pts/1".to_string()])
                .unwrap_err(),
            MuxError::Unsupported("bare cannot reveal a client terminal".into())
        );
        assert_eq!(
            Bare.client_ttys(&|_| panic!("a backend with no client concept must not scan")),
            None
        );
        assert_eq!(
            tmux::TMUX.client_ttys(&|_| panic!("tmux has no client concept: no /proc walk")),
            None
        );
    }

    // ---- focus_message_in: fake /proc + fixture programs, end to end ---------
    //
    // Both UI paths (the Sessions panel in `normal.rs` and the project tab-switch
    // in `detail.rs`) call the one `focus_message`, which is `focus_message_in`
    // over the real `/proc`; `tests/focus_entry_guard.rs` pins that nothing else
    // reaches a backend. These tests drive that entry with a process table laid
    // out in a directory and `herdr` / `tmux` fixture scripts, so they cover the
    // SAME code for both.

    #[cfg(unix)]
    mod entry {
        use super::*;
        use crate::multiplexer::fixtures::{
            claude, pane_list_json, tmux_call, FakeProc, World, HERDR_SOCKET, LIST_PANES_ARGS,
            TMUX_SOCKET,
        };

        // The fake processes' cmdline carries no `--resume <id>`: the session id
        // reaches the focus path through `ClaudeSession::session_id` (the
        // detector's job), and spelling the option here would make this file a
        // `claude` argv site the census in `session_detector` must adjudicate.
        const SID_P1: &str = "6e02a046-aa43-4f2c-9b1e-0123456789ab";
        const SID_P2: &str = "f1644e8e-63f3-4d51-8a7c-0123456789ab";
        const SID_KNIT: &str = "c0ffee00-1111-4222-8333-444455556666";
        const TMUX_VALUE: &str = "/tmp/tmux-1000/default,3628116,0";

        const HERDR_SESSION_P1: [&str; 6] = [
            "HERDR_ENV=1",
            "HERDR_SOCKET_PATH=/home/blk/.config/herdr/herdr.sock",
            "HERDR_WORKSPACE_ID=w2",
            "HERDR_TAB_ID=w2:t1",
            "HERDR_PANE_ID=w2:p1",
            "TERM_PROGRAM=herdr",
        ];
        const HERDR_SESSION_P2: [&str; 6] = [
            "HERDR_ENV=1",
            "HERDR_SOCKET_PATH=/home/blk/.config/herdr/herdr.sock",
            "HERDR_WORKSPACE_ID=w2",
            "HERDR_TAB_ID=w2:t2",
            "HERDR_PANE_ID=w2:p2",
            "TERM_PROGRAM=herdr",
        ];

        /// The measured topology (debug session `herdr-session-focus-from-tmux`).
        /// The herdr server 671927 is headless under systemd; two Claude
        /// sessions hang off it. The herdr CLIENT 3630647 is on pts/13, which
        /// is tmux pane 0:0.0, a child of the tmux server 3628116 through fish.
        /// The TUI 3631000 sits in tmux pane 0:0.1 (pts/8) of the same window.
        fn measured_topology() -> FakeProc {
            let p = FakeProc::new();
            p.process(1, "systemd", 0, &["/sbin/init"], &[], None)
                .process(2384, "systemd", 1, &["systemd", "--user"], &[], None)
                .process(
                    671927,
                    "herdr",
                    2384,
                    &["/home/blk/.local/bin/herdr", "server"],
                    &[],
                    Some("/dev/null"),
                )
                .process(671995, "fish", 671927, &["fish"], &[], Some("/dev/pts/3"))
                .process(
                    672012,
                    "claude",
                    671995,
                    &["claude"],
                    &HERDR_SESSION_P1,
                    Some("/dev/pts/3"),
                )
                .process(672020, "fish", 671927, &["fish"], &[], Some("/dev/pts/4"))
                .process(
                    672048,
                    "claude",
                    672020,
                    &["claude"],
                    &HERDR_SESSION_P2,
                    Some("/dev/pts/4"),
                )
                .process(3628116, "tmux: server", 1, &["tmux"], &[], None)
                .process(
                    3628117,
                    "fish",
                    3628116,
                    &["fish"],
                    &[],
                    Some("/dev/pts/13"),
                )
                .process(
                    3630647,
                    "herdr",
                    3628117,
                    &["herdr", "session", "attach", "default"],
                    &[],
                    Some("/dev/pts/13"),
                )
                .process(3629000, "fish", 3628116, &["fish"], &[], Some("/dev/pts/8"))
                .process(
                    3631000,
                    "gsd-meta-manager",
                    3629000,
                    &["gsd-meta-manager"],
                    &[],
                    Some("/dev/pts/8"),
                );
            p
        }

        const TUI_PID_IN_TMUX: u32 = 3631000;

        fn tmux_tui_env() -> EnvSnapshot {
            EnvSnapshot::from_pairs(&[("TMUX", TMUX_VALUE)])
        }

        fn herdr_panes() -> String {
            pane_list_json(&[("w2:p1", "w2:t1", SID_P1), ("w2:p2", "w2:t2", SID_P2)])
        }

        fn herdr_call(rest: &str) -> String {
            format!("herdr {rest} sock={HERDR_SOCKET}")
        }

        /// The TUI in a tmux pane, the session under a sibling herdr server,
        /// the herdr client in the neighbouring tmux pane: herdr focuses
        /// w2:p1, THEN tmux selects 0:0.0. This is the reported bug, end to
        /// end, for the Sessions panel and the tab-switch alike.
        #[test]
        fn a_herdr_session_is_focused_in_herdr_then_its_client_pane_is_selected_in_tmux() {
            let proc = measured_topology();
            let world = World::new(&herdr_panes(), "/dev/pts/13 0:0.0\n/dev/pts/8 0:0.1\n");
            let (message, calls) = world.run(|| {
                focus_message_in(
                    &proc.procfs(),
                    &world.registry(),
                    &tmux_tui_env(),
                    TUI_PID_IN_TMUX,
                    &claude(672012, Some(SID_P1), Some("pts/3")),
                    "sentriq",
                )
            });
            assert_eq!(message, "Switched to sentriq");
            assert!(!message.contains("not found in any tmux pane"), "{message}");
            assert_eq!(
                calls,
                vec![
                    herdr_call("pane list"),
                    herdr_call("agent focus w2:p1"),
                    tmux_call(Some(TMUX_SOCKET), LIST_PANES_ARGS),
                    tmux_call(Some(TMUX_SOCKET), "select-window -t 0:0.0"),
                    tmux_call(Some(TMUX_SOCKET), "select-pane -t 0:0.0"),
                ],
                "the session id joins the pane (no process-info, no cwd guess); herdr acts on \
                 the SESSION's socket; tmux then selects the pane showing the herdr client"
            );
        }

        #[test]
        fn the_second_session_is_focused_in_its_own_pane() {
            let proc = measured_topology();
            let world = World::new(&herdr_panes(), "/dev/pts/13 0:0.0\n/dev/pts/8 0:0.1\n");
            let (message, calls) = world.run(|| {
                focus_message_in(
                    &proc.procfs(),
                    &world.registry(),
                    &tmux_tui_env(),
                    TUI_PID_IN_TMUX,
                    &claude(672048, Some(SID_P2), Some("pts/4")),
                    "sentriq",
                )
            });
            assert_eq!(message, "Switched to sentriq");
            assert!(
                calls.contains(&herdr_call("agent focus w2:p2")),
                "{calls:?}"
            );
            assert!(
                !calls.iter().any(|c| c.contains("process-info")),
                "the session-id tier is enough: {calls:?}"
            );
        }

        /// Several clients attached (two herdr sessions, or two terminals): the
        /// pick is the first match in tmux's own listing order, so it is the
        /// same every time, and it is a view of an already-correct focus.
        #[test]
        fn several_clients_give_a_deterministic_tmux_pick() {
            let proc = measured_topology();
            proc.process(
                3640001,
                "herdr",
                3628117,
                &["herdr", "--session", "work"],
                &[],
                Some("/dev/pts/9"),
            )
            .process(
                3640002,
                "herdr",
                1,
                &["herdr", "session", "attach", "other"],
                &[],
                Some("/dev/pts/20"),
            );
            // pts/9 is listed before pts/13; pts/20 is in no tmux pane.
            let listing = "/dev/pts/8 0:0.1\n/dev/pts/9 0:1.0\n/dev/pts/13 0:0.0\n";
            let world = World::new(&herdr_panes(), listing);
            let run = || {
                world.run(|| {
                    focus_message_in(
                        &proc.procfs(),
                        &world.registry(),
                        &tmux_tui_env(),
                        TUI_PID_IN_TMUX,
                        &claude(672012, Some(SID_P1), Some("pts/3")),
                        "sentriq",
                    )
                })
            };
            let (first, calls) = run();
            let (second, _) = run();
            assert_eq!(first, "Switched to sentriq");
            assert_eq!(first, second);
            assert!(
                calls.contains(&tmux_call(Some(TMUX_SOCKET), "select-pane -t 0:1.0")),
                "the first matching pane in tmux's listing order: {calls:?}"
            );
        }

        // ---- locate: the Sessions tab's location column, read-only ------------
        //
        // `locate_sessions_in` is the location twin of `focus_message_in`: the
        // same process table, the same fixture programs, the same routing. The
        // call log of every test here may hold only the read verbs.

        /// The label of `session`'s location (`-` when none), through the real
        /// `locate_sessions_in`, with the fixture programs' call log.
        fn located(
            proc: &FakeProc,
            world: &World,
            tui_env: &EnvSnapshot,
            tui_pid: u32,
            session: &ClaudeSession,
        ) -> (String, Vec<String>) {
            world.run(|| {
                let found = location::locate_sessions_in(
                    &proc.procfs(),
                    &world.registry(),
                    tui_env,
                    tui_pid,
                    std::slice::from_ref(session),
                );
                found
                    .get(&session.pid)
                    .and_then(|entry| entry.location.as_ref())
                    .map_or_else(|| "-".to_string(), |location| location.label())
            })
        }

        fn assert_reads_only(calls: &[String]) {
            for call in calls {
                assert!(
                    !call.contains("agent focus")
                        && !call.contains("tab focus")
                        && !call.contains("select-"),
                    "locating must not act: {calls:?}"
                );
            }
        }

        /// A tmux session: claude under a fish under the tmux server, on its
        /// own pts, with `$TMUX` in its environment.
        fn tmux_session_topology() -> FakeProc {
            let p = measured_topology();
            let tmux_env = format!("TMUX={TMUX_VALUE}");
            p.process(
                3700000,
                "fish",
                3628116,
                &["fish"],
                &[],
                Some("/dev/pts/21"),
            )
            .process(
                3700001,
                "claude",
                3700000,
                &["claude"],
                &[tmux_env.as_str()],
                Some("/dev/pts/21"),
            );
            p
        }

        #[test]
        fn locate_a_tmux_session_reads_list_panes_and_selects_nothing() {
            let proc = tmux_session_topology();
            let world = World::new(&herdr_panes(), "/dev/pts/8 0:0.1\n/dev/pts/21 0:1.2\n");
            let (label, calls) = located(
                &proc,
                &world,
                &tmux_tui_env(),
                TUI_PID_IN_TMUX,
                &claude(3700001, None, Some("pts/21")),
            );
            assert_eq!(label, "tmux 0:1.2");
            assert_eq!(
                calls,
                vec![tmux_call(Some(TMUX_SOCKET), LIST_PANES_ARGS)],
                "one read-only list-panes on the session's own server"
            );
            assert_reads_only(&calls);
        }

        #[test]
        fn locate_a_session_outside_any_multiplexer_is_absent_and_spawns_nothing() {
            let plain = FakeProc::new();
            plain
                .process(1, "systemd", 0, &["/sbin/init"], &[], None)
                .process(5000, "fish", 1, &["fish"], &[], Some("/dev/pts/30"))
                .process(5001, "claude", 5000, &["claude"], &[], Some("/dev/pts/30"))
                .process(5100, "gsd-meta-manager", 1, &[], &[], Some("/dev/pts/31"));
            let world = World::new(&herdr_panes(), "/dev/pts/30 0:0.0\n");
            let (label, calls) = located(
                &plain,
                &world,
                &EnvSnapshot::default(),
                5100,
                &claude(5001, None, Some("pts/30")),
            );
            assert_eq!(label, "-", "no multiplexer finds it");
            assert!(calls.is_empty(), "nothing was spawned: {calls:?}");
        }

        /// The herdr client runs in a terminal that is not a tmux pane (a plain
        /// terminal emulator): the herdr focus succeeded, so no error, only a
        /// note, and tmux selects nothing.
        #[test]
        fn a_client_in_a_non_tmux_terminal_is_a_note_with_no_tmux_step() {
            let proc = measured_topology();
            let world = World::new(&herdr_panes(), "/dev/pts/8 0:0.1\n");
            let (message, calls) = world.run(|| {
                focus_message_in(
                    &proc.procfs(),
                    &world.registry(),
                    &tmux_tui_env(),
                    TUI_PID_IN_TMUX,
                    &claude(672012, Some(SID_P1), Some("pts/3")),
                    "sentriq",
                )
            });
            assert_eq!(
                message,
                "Switched to sentriq (focused in herdr; its herdr client is not in a tmux pane)"
            );
            assert!(
                !calls.iter().any(|c| c.contains("select-")),
                "no tmux pane shows the client, so no tmux pane is selected: {calls:?}"
            );
        }

        #[test]
        fn no_attached_client_is_a_clear_partial_outcome_and_tmux_is_never_asked() {
            // The measured topology with the client gone: only the server, the
            // session and the TUI remain.
            let gone = FakeProc::new();
            gone.process(1, "systemd", 0, &[], &[], None)
                .process(2384, "systemd", 1, &[], &[], None)
                .process(
                    671927,
                    "herdr",
                    2384,
                    &["herdr", "server"],
                    &[],
                    Some("/dev/null"),
                )
                .process(671995, "fish", 671927, &["fish"], &[], Some("/dev/pts/3"))
                .process(
                    672012,
                    "claude",
                    671995,
                    &["claude"],
                    &HERDR_SESSION_P1,
                    Some("/dev/pts/3"),
                )
                .process(3628116, "tmux: server", 1, &["tmux"], &[], None)
                .process(3629000, "fish", 3628116, &["fish"], &[], Some("/dev/pts/8"))
                .process(
                    3631000,
                    "gsd-meta-manager",
                    3629000,
                    &[],
                    &[],
                    Some("/dev/pts/8"),
                );
            let world = World::new(&herdr_panes(), "/dev/pts/8 0:0.1\n");
            let (message, calls) = world.run(|| {
                focus_message_in(
                    &gone.procfs(),
                    &world.registry(),
                    &tmux_tui_env(),
                    TUI_PID_IN_TMUX,
                    &claude(672012, Some(SID_P1), Some("pts/3")),
                    "sentriq",
                )
            });
            assert_eq!(
                message,
                "Switched to sentriq (focused in herdr; no herdr client is attached)"
            );
            assert!(
                !calls.iter().any(|c| c.starts_with("tmux")),
                "nothing to reveal, so tmux is not asked: {calls:?}"
            );
        }

        /// Live case B1. The TUI is in herdr pane w2:p5; the `knit` project's
        /// session is herdr pane w6:p1 in ANOTHER workspace. herdr focuses
        /// workspace, tab and pane in one `agent focus`. v2.2.0 answered
        /// "Tab-switching requires running inside tmux (no $TMUX set)" here;
        /// that text is gone, and this pins that it does not come back.
        #[test]
        fn a_herdr_tui_focuses_a_herdr_session_in_another_workspace() {
            let proc = FakeProc::new();
            proc.process(1, "systemd", 0, &[], &[], None)
                .process(2384, "systemd", 1, &[], &[], None)
                .process(
                    671927,
                    "herdr",
                    2384,
                    &["herdr", "server"],
                    &[],
                    Some("/dev/null"),
                )
                .process(700000, "fish", 671927, &["fish"], &[], Some("/dev/pts/20"))
                .process(
                    700100,
                    "gsd-meta-manager",
                    700000,
                    &["gsd-meta-manager"],
                    &[],
                    Some("/dev/pts/20"),
                )
                .process(700400, "fish", 671927, &["fish"], &[], Some("/dev/pts/21"))
                .process(
                    700500,
                    "claude",
                    700400,
                    &["claude"],
                    &[
                        "HERDR_ENV=1",
                        "HERDR_SOCKET_PATH=/home/blk/.config/herdr/herdr.sock",
                        "HERDR_WORKSPACE_ID=w6",
                        "HERDR_PANE_ID=w6:p1",
                    ],
                    Some("/dev/pts/21"),
                );
            let panes = pane_list_json(&[
                ("w2:p5", "w2:t5", "00000000-0000-4000-8000-000000000000"),
                ("w6:p1", "w6:t1", SID_KNIT),
            ]);
            let world = World::new(&panes, "");
            let tui_env = EnvSnapshot::from_pairs(&[
                ("HERDR_ENV", "1"),
                ("HERDR_SOCKET_PATH", HERDR_SOCKET),
                ("HERDR_WORKSPACE_ID", "w2"),
            ]);
            let (message, calls) = world.run(|| {
                focus_message_in(
                    &proc.procfs(),
                    &world.registry(),
                    &tui_env,
                    700100,
                    &claude(700500, Some(SID_KNIT), Some("pts/21")),
                    "knit",
                )
            });
            assert_eq!(message, "Switched to knit");
            assert!(!message.contains("Tab-switching requires"), "{message}");
            assert!(!message.contains("no $TMUX set"), "{message}");
            assert_eq!(
                calls,
                vec![herdr_call("pane list"), herdr_call("agent focus w6:p1")],
                "the session's own pane in the other workspace, and no tmux step: the TUI \
                 and the session are both in herdr"
            );
        }

        /// Live case B2. The TUI runs in herdr (no `$TMUX` in its environment);
        /// the session lives in tmux. The tmux server is addressed from the
        /// SESSION's `$TMUX`, with `-S`, and herdr is never asked.
        #[test]
        fn a_herdr_tui_focuses_a_tmux_session_through_the_sessions_own_tmux_socket() {
            let proc = FakeProc::new();
            proc.process(1, "systemd", 0, &[], &[], None)
                .process(2384, "systemd", 1, &[], &[], None)
                .process(
                    671927,
                    "herdr",
                    2384,
                    &["herdr", "server"],
                    &[],
                    Some("/dev/null"),
                )
                .process(700000, "fish", 671927, &["fish"], &[], Some("/dev/pts/20"))
                .process(
                    700100,
                    "gsd-meta-manager",
                    700000,
                    &[],
                    &[],
                    Some("/dev/pts/20"),
                )
                .process(3628116, "tmux: server", 1, &["tmux"], &[], None)
                .process(800400, "fish", 3628116, &["fish"], &[], Some("/dev/pts/5"))
                .process(
                    800500,
                    "claude",
                    800400,
                    &["claude"],
                    &["TMUX=/tmp/tmux-1000/default,3628116,0", "TMUX_PANE=%4"],
                    Some("/dev/pts/5"),
                );
            let world = World::new(&herdr_panes(), "/dev/pts/5 0:1.2\n/dev/pts/6 0:2.0\n");
            let tui_env = EnvSnapshot::from_pairs(&[
                ("HERDR_ENV", "1"),
                ("HERDR_SOCKET_PATH", HERDR_SOCKET),
                ("HERDR_WORKSPACE_ID", "w2"),
            ]);
            let (message, calls) = world.run(|| {
                focus_message_in(
                    &proc.procfs(),
                    &world.registry(),
                    &tui_env,
                    700100,
                    &claude(800500, None, Some("pts/5")),
                    "alpha",
                )
            });
            assert_eq!(message, "Switched to alpha");
            assert_eq!(
                calls,
                vec![
                    tmux_call(Some(TMUX_SOCKET), LIST_PANES_ARGS),
                    tmux_call(Some(TMUX_SOCKET), "select-window -t 0:1.2"),
                    tmux_call(Some(TMUX_SOCKET), "select-pane -t 0:1.2"),
                ],
                "tmux first, on the session's socket; herdr (the TUI's) is never needed"
            );
        }

        /// The TUI and the session are on DIFFERENT tmux servers: the session's
        /// socket wins, not the TUI's.
        #[test]
        fn a_session_on_another_tmux_server_is_addressed_by_its_own_socket() {
            let proc = FakeProc::new();
            proc.process(1, "systemd", 0, &[], &[], None)
                .process(3628116, "tmux: server", 1, &["tmux"], &[], None)
                .process(
                    3631000,
                    "gsd-meta-manager",
                    3628116,
                    &[],
                    &[],
                    Some("/dev/pts/8"),
                )
                .process(
                    4000001,
                    "tmux: server",
                    1,
                    &["tmux", "-L", "work"],
                    &[],
                    None,
                )
                .process(
                    4000100,
                    "claude",
                    4000001,
                    &["claude"],
                    &["TMUX=/tmp/tmux-1000/work,4000001,0"],
                    Some("/dev/pts/5"),
                );
            let world = World::new("", "/dev/pts/5 work:0.0\n");
            let (message, calls) = world.run(|| {
                focus_message_in(
                    &proc.procfs(),
                    &world.registry(),
                    &tmux_tui_env(),
                    3631000,
                    &claude(4000100, None, Some("pts/5")),
                    "alpha",
                )
            });
            assert_eq!(message, "Switched to alpha");
            assert_eq!(
                calls,
                vec![
                    tmux_call(Some("/tmp/tmux-1000/work"), LIST_PANES_ARGS),
                    tmux_call(Some("/tmp/tmux-1000/work"), "select-window -t work:0.0"),
                    tmux_call(Some("/tmp/tmux-1000/work"), "select-pane -t work:0.0"),
                ]
            );
        }

        /// A `$TMUX` in the session's environment that names no absolute socket
        /// is dropped, so it can neither route to tmux nor reach `tmux -S`.
        #[test]
        fn a_relative_tmux_socket_in_a_sessions_environment_is_never_used() {
            let proc = FakeProc::new();
            proc.process(1, "systemd", 0, &[], &[], None)
                .process(
                    671927,
                    "herdr",
                    1,
                    &["herdr", "server"],
                    &[],
                    Some("/dev/null"),
                )
                .process(
                    700100,
                    "gsd-meta-manager",
                    671927,
                    &[],
                    &[],
                    Some("/dev/pts/20"),
                )
                .process(
                    800500,
                    "claude",
                    1,
                    &["claude"],
                    &["TMUX=relative/sock,1,0"],
                    Some("/dev/pts/5"),
                );
            let world = World::new(&herdr_panes(), "/dev/pts/5 0:1.2\n");
            let tui_env =
                EnvSnapshot::from_pairs(&[("HERDR_ENV", "1"), ("HERDR_SOCKET_PATH", HERDR_SOCKET)]);
            let (message, calls) = world.run(|| {
                focus_message_in(
                    &proc.procfs(),
                    &world.registry(),
                    &tui_env,
                    700100,
                    &claude(800500, None, Some("pts/5")),
                    "alpha",
                )
            });
            assert!(
                !calls.is_empty() && calls.iter().all(|c| c.starts_with("herdr")),
                "the garbage value must not reach tmux; only the TUI's own herdr was asked: \
                 {calls:?} / {message}"
            );
            assert!(!message.starts_with("Switched"), "{message}");
        }

        /// SC2 guard, through the whole entry: a tmux-only user (TUI and
        /// session on the same tmux server) gets the tmux-only behaviour.
        #[test]
        fn a_tmux_session_in_a_tmux_tui_is_focused_by_tmux_alone() {
            let proc = FakeProc::new();
            proc.process(1, "systemd", 0, &[], &[], None)
                .process(3628116, "tmux: server", 1, &["tmux"], &[], None)
                .process(
                    3631000,
                    "gsd-meta-manager",
                    3628116,
                    &[],
                    &[],
                    Some("/dev/pts/8"),
                )
                .process(
                    3632000,
                    "claude",
                    3628116,
                    &["claude"],
                    &[&format!("TMUX={TMUX_VALUE}")],
                    Some("/dev/pts/5"),
                );
            let world = World::new("", "/dev/pts/5 0:3.0\n");
            let (message, calls) = world.run(|| {
                focus_message_in(
                    &proc.procfs(),
                    &world.registry(),
                    &tmux_tui_env(),
                    3631000,
                    &claude(3632000, None, Some("pts/5")),
                    "alpha",
                )
            });
            assert_eq!(message, "Switched to alpha");
            assert_eq!(calls.len(), 3, "{calls:?}");
            assert!(calls.iter().all(|c| c.starts_with("tmux")), "{calls:?}");
        }

        /// When the session's process cannot be read (it exited, or belongs to
        /// another uid), the TUI-derived plan applies and tmux's wording is
        /// byte-identical to the tmux-only days.
        #[test]
        fn an_unreadable_session_falls_back_to_the_tuis_own_backends_verbatim() {
            let proc = FakeProc::new();
            proc.process(1, "systemd", 0, &[], &[], None)
                .process(3628116, "tmux: server", 1, &["tmux"], &[], None)
                .process(
                    3631000,
                    "gsd-meta-manager",
                    3628116,
                    &[],
                    &[],
                    Some("/dev/pts/8"),
                );
            let world = World::new("", "/dev/pts/8 0:0.1\n");
            let (message, calls) = world.run(|| {
                focus_message_in(
                    &proc.procfs(),
                    &world.registry(),
                    &tmux_tui_env(),
                    3631000,
                    &claude(999_999, None, Some("pts/3")),
                    "alpha",
                )
            });
            assert_eq!(
                message,
                "TTY pts/3 not found in any tmux pane (session may have exited)"
            );
            assert_eq!(
                calls,
                vec![tmux_call(Some(TMUX_SOCKET), LIST_PANES_ARGS)],
                "no select step when nothing matched"
            );
        }

        /// With no multiplexer anywhere (neither the TUI nor the session), the
        /// message is the unchanged "none detected" text.
        #[test]
        fn no_multiplexer_anywhere_is_the_none_detected_text() {
            let proc = FakeProc::new();
            proc.process(1, "systemd", 0, &[], &[], None)
                .process(500, "gsd-meta-manager", 1, &[], &[], Some("/dev/pts/1"))
                .process(
                    600,
                    "claude",
                    1,
                    &["claude"],
                    &["PATH=/usr/bin"],
                    Some("/dev/pts/2"),
                );
            let world = World::new("", "");
            let (message, calls) = world.run(|| {
                focus_message_in(
                    &proc.procfs(),
                    &world.registry(),
                    &EnvSnapshot::default(),
                    500,
                    &claude(600, None, Some("pts/2")),
                    "alpha",
                )
            });
            assert_eq!(
                message,
                "Tab-switching requires running inside tmux or herdr (none detected)"
            );
            assert!(calls.is_empty());
        }
    }
}
