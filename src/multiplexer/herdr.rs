//! The herdr backend (MUX-05): detection on `HERDR_ENV`, and a new tab through
//! ONE socket `layout.apply` request.
//!
//! # Why the socket, and never the CLI, to launch (D-12, D-13)
//!
//! * `herdr pane run` is **VIA-SHELL**: it types its text into the pane's shell.
//!   Command injection through it was reproduced (RESEARCH E3). `pane send-text`,
//!   `pane send-keys` and `agent start` also type into a shell. D-12 and MUX-05
//!   forbid all of them as launch paths, so none of them is used here, and none
//!   is a fallback when the socket request fails.
//! * `layout.apply` is **EXEC-DIRECT**: the program is a direct child of the
//!   herdr server, with no shell in between, and each `command` element arrives
//!   as one argv element.
//!
//! `layout.apply` creates the tab and execs the program atomically, and
//! [`ExecArgv`] is validated before any connect, so D-12's order of checks holds
//! by construction and no empty tab can be left behind. A head that cannot be
//! spawned returns a synchronous `layout_apply_failed` error and creates no tab.
//!
//! The exec context is the herdr SERVER's `PATH`, the same property tmux has
//! today. A leading `env` hides a missing `claude` (the tab opens and closes),
//! accepted for parity with tmux.
//!
//! # The decision this resolves [inferred]
//!
//! D-12 names the herdr socket API as the alternative when the CLI types into a
//! shell, while D-13 says "no socket client this phase". The orchestrator
//! resolved it as one narrow, std-only request: one NDJSON line out, one line
//! back, with timeouts and no new crate. It is not a socket client: no
//! subscriptions, no reconnect, no second method. Reversible: it is one private
//! function ([`open_over_socket`]).
//!
//! # Focus (MUX-04, D-11 as corrected by research) \[inferred\]
//!
//! Focus runs the herdr CLI, never the socket, and only the four read and
//! focus verbs in `CLI_VERBS`. Two corrections to D-11, both adopted from the
//! measurements in RESEARCH E2 and E6:
//!
//! * herdr exposes **no tty** anywhere (`pane list` has no such field and
//!   `pane process-info` reports none), so D-11's tty step is replaced by a
//!   **pid** step: the session pid among a pane's foreground processes, or the
//!   pane's `shell_pid` among the session's `/proc` ancestors.
//! * `herdr agent focus <pane_id>` focuses workspace, tab and pane in **one**
//!   call, so there is no workspace-then-tab-then-pane sequence to build. When
//!   herdr tracks no agent in that pane it answers `agent_not_found`, and
//!   `herdr tab focus <tab_id>` is the fallback, reported as partial when the
//!   tab holds more than one pane.
//!
//! The join tiers run in order (session id, pid, working directory), and a tier
//! with two or more matches is **refused**, never guessed (D-11, T-33-09). The
//! working-directory tier is a last resort that cannot guess (review WR-01): it
//! considers only panes herdr reports as running this session's kind of agent
//! whose process-info could not be read, so a plain shell in the same directory
//! is never focused and a pane known to run other processes never pre-empts an
//! outer backend's exact match. A success by directory carries a note saying so. The
//! order of panes in herdr's output never decides anything. The matching is
//! exact for the reason `tmux::pane_tty_matches` is: a prefix or substring test
//! would let one session's focus key send the operator's keystrokes into an
//! unrelated pane.
//!
//! The server is the one the SESSION lives under: the focus environment carries
//! the session's own `HERDR_SOCKET_PATH`, so a TUI that runs outside herdr (in
//! tmux) focuses a herdr session through the same session-id join. After a
//! focus, `client_ttys_from` names the terminals of the herdr CLIENTS that
//! display the server, so the multiplexer around the client (tmux) can bring
//! that pane forward (the reveal step in `super`).
//!
//! This file spawns the herdr CLI for those reads and focuses, so it is on the
//! spawn-seam allowlist. Launching a session never goes through the CLI.

use super::{
    proc_chain, Detection, EnvSnapshot, ExecArgv, FocusOutcome, MatchedBy, MultiplexerBackend,
    MuxError, ProcEntry, ProcScan,
};
use crate::session_detector::{ClaudeSession, SessionKind};
use serde::Deserialize;
use std::path::{Path, PathBuf};
use std::process::Command;

/// The herdr backend.
pub struct HerdrBackend {
    /// The CLI program focus runs. The registry entry uses `"herdr"`; a test
    /// points it at a fixture script. Fixed per instance, never read from the
    /// environment.
    program: &'static str,
}

/// The registry entry for herdr.
pub static HERDR: HerdrBackend = HerdrBackend { program: "herdr" };

impl HerdrBackend {
    /// A backend that runs `program` instead of `herdr`: a fixture script.
    #[cfg(test)]
    pub(crate) fn with_program(program: &'static str) -> Self {
        Self { program }
    }
}

/// The request id; herdr echoes it in the reply.
const REQUEST_ID: &str = "gsdmm-open";

/// Build the `layout.apply` request: a fresh tab holding one pane that runs
/// `argv` in `cwd`. **Pure**, and built with `serde_json`, never by string
/// formatting, so quotes and unicode in an element or the cwd cannot break the
/// envelope (T-33-04). Each argv element is exactly one JSON array element.
///
/// `workspace` is `None` when the snapshot has no workspace; it serializes as
/// JSON `null`, which makes herdr use the focused workspace.
pub fn layout_apply_request(
    workspace: Option<&str>,
    cwd: &Path,
    argv: &ExecArgv,
    focus: bool,
) -> serde_json::Value {
    serde_json::json!({
        "id": REQUEST_ID,
        "method": "layout.apply",
        "params": {
            "workspace_id": workspace,
            "focus": focus,
            "root": {
                "type": "pane",
                "cwd": cwd.to_string_lossy(),
                "command": argv.as_slice(),
            },
        },
    })
}

/// Interpret herdr's reply line. **Pure.**
///
/// A `result` object is success. An `error` object is [`MuxError::Failed`]
/// naming its code and message. Empty input, non-JSON, a non-object, or an
/// object with neither key is also `Failed`, with a reason naming what was
/// wrong. A non-null error key is checked first, so an ambiguous reply fails
/// closed; `"error": null` counts as absent. The echoed `id` is not compared:
/// nothing shows herdr always echoes it, and a mismatch check would turn a
/// working open into a failure and a second GUI window \[inferred, for audit\].
pub fn parse_layout_apply_reply(reply: &[u8]) -> Result<(), MuxError> {
    let text = String::from_utf8_lossy(reply);
    let text = text.trim();
    if text.is_empty() {
        return Err(MuxError::Failed("herdr sent an empty reply".to_string()));
    }
    let value: serde_json::Value = serde_json::from_str(text)
        .map_err(|e| MuxError::Failed(format!("herdr reply is not JSON: {e}")))?;
    let Some(object) = value.as_object() else {
        return Err(MuxError::Failed(
            "herdr reply is not a JSON object".to_string(),
        ));
    };
    // `"error": null` is "no error", not a failure (review IN-03).
    if let Some(error) = object.get("error").filter(|e| !e.is_null()) {
        let field = |name: &str| {
            error
                .get(name)
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string()
        };
        return Err(MuxError::Failed(format!(
            "layout.apply failed ({}): {}",
            field("code"),
            field("message")
        )));
    }
    if object.contains_key("result") {
        return Ok(());
    }
    Err(MuxError::Failed(
        "herdr reply has neither a result nor an error".to_string(),
    ))
}

impl MultiplexerBackend for HerdrBackend {
    fn name(&self) -> &'static str {
        "herdr"
    }

    /// Inside when `HERDR_ENV` equals exactly `"1"` (no normalization). The
    /// control socket is reachable only with a non-empty `HERDR_SOCKET_PATH`.
    fn detect(&self, env: &EnvSnapshot) -> Option<Detection> {
        if env.get("HERDR_ENV") != Some("1") {
            return None;
        }
        let reachable = match env.get("HERDR_SOCKET_PATH") {
            Some(path) if !path.is_empty() => Ok(()),
            _ => Err("HERDR_SOCKET_PATH is unset".to_string()),
        };
        Some(Detection {
            inside: true,
            reachable,
        })
    }

    /// herdr's client and server share the comm `herdr`, and only the cmdline
    /// tells them apart. The nearest-first ancestry walk makes that safe: the
    /// first `herdr` ancestor is what the TUI runs under (research E5).
    fn is_server_comm(&self, comm: &str) -> bool {
        comm == COMM
    }

    /// The terminals of the herdr CLIENT processes that display the server
    /// (reveal step, see [`MultiplexerBackend::client_ttys`]). The pane focus
    /// above is a server-side fact; the user sees it only through a client, and
    /// that client commonly sits in a tmux pane.
    fn client_ttys(&self, scan: &ProcScan<'_>) -> Option<Vec<String>> {
        Some(client_ttys_from(&scan(COMM)))
    }

    /// Focus the herdr pane running `session`: list the panes, join on session
    /// id, then pid, then working directory, then `agent focus` the match with
    /// `tab focus` as the fallback. See the module docs for why each step.
    fn focus(&self, env: &EnvSnapshot, session: &ClaudeSession) -> Result<FocusOutcome, MuxError> {
        let listing = self.herdr_cli(env, &["pane", "list"])?;
        let panes = parse_pane_list(&listing)?;

        let session_id = session
            .session_id
            .as_ref()
            .map(|id| id.as_raw_for_logic_only());
        let (pane, by) = match join_by_session_id(&panes, session_id) {
            Tier::One(pane) => (pane, MatchedBy::SessionId),
            Tier::Ambiguous(n) => return Err(ambiguous(n, "session id")),
            Tier::None => {
                let candidates = cwd_candidates(&panes, &session.working_dir);

                // The session pid itself counts as a chain pid, so the join
                // still works where `/proc` cannot be read.
                let mut chain_pids = vec![session.pid];
                chain_pids.extend(proc_chain(session.pid).into_iter().map(|(pid, _)| pid));
                let mut probed: Vec<(&HerdrPane, ProcessInfo)> = Vec::new();
                // Candidates whose process-info could not be read. Only these
                // may still be matched by directory (see `cwd_fallback`).
                let mut unprobed: Vec<&HerdrPane> = Vec::new();
                for candidate in &candidates {
                    let Ok(id) = validate_id(&candidate.pane_id) else {
                        unprobed.push(candidate);
                        continue;
                    };
                    match self.process_info(env, id) {
                        Ok(info) => probed.push((*candidate, info)),
                        // One dead pane must not block focusing the others.
                        Err(e) => {
                            tracing::debug!(pane = id, error = %e, "herdr process-info failed");
                            unprobed.push(candidate);
                        }
                    }
                }

                match join_by_pid(&probed, session.pid, &chain_pids) {
                    Tier::One(pane) => (pane, MatchedBy::Pid),
                    Tier::Ambiguous(n) => return Err(ambiguous(n, "pid")),
                    Tier::None => match join_by_cwd(&cwd_fallback(&unprobed, session.kind)) {
                        Tier::One(pane) => (pane, MatchedBy::Cwd),
                        Tier::Ambiguous(n) => return Err(ambiguous(n, "working directory")),
                        Tier::None => {
                            return Err(MuxError::NotFound(
                                "no herdr pane runs this session (session may have exited)"
                                    .to_string(),
                            ))
                        }
                    },
                }
            }
        };

        let pane_id = validate_id(&pane.pane_id)?;
        // A directory-only match is a guess the user should hear about.
        let cwd_note = (by == MatchedBy::Cwd).then(|| CWD_ONLY_NOTE.to_string());
        match self.herdr_cli(env, &["agent", "focus", pane_id]) {
            Ok(_) => Ok(FocusOutcome {
                by,
                partial: cwd_note,
            }),
            Err(MuxError::Failed(message))
                if error_code(&message).as_deref() == Some("agent_not_found") =>
            {
                // herdr tracks no agent in this pane (research P7): focus its
                // tab instead, and say so when that is not the exact pane.
                let tab_id = pane.tab_id.as_deref().ok_or_else(|| {
                    MuxError::Failed(
                        "herdr tracks no agent in the matched pane and reports no tab for it"
                            .to_string(),
                    )
                })?;
                let tab_id = validate_id(tab_id)?;
                self.herdr_cli(env, &["tab", "focus", tab_id])?;
                let in_tab = panes
                    .iter()
                    .filter(|p| p.tab_id.as_deref() == Some(tab_id))
                    .count();
                let tab_note = (in_tab != 1).then(|| {
                    format!(
                        "tab focused, not the pane: herdr tracks no agent in that pane and its tab has {in_tab} panes"
                    )
                });
                let partial = match (tab_note, cwd_note) {
                    (Some(tab), Some(cwd)) => Some(format!("{tab}; {cwd}")),
                    (tab, cwd) => tab.or(cwd),
                };
                Ok(FocusOutcome { by, partial })
            }
            Err(e) => Err(e),
        }
    }

    /// Open `argv` in a new herdr tab in `cwd`, through one socket request. The
    /// socket path and workspace come from the snapshot (D-13), never from the
    /// process environment.
    #[cfg(unix)]
    fn open(&self, env: &EnvSnapshot, cwd: &Path, argv: &ExecArgv) -> Result<(), MuxError> {
        open_over_socket(env, cwd, argv)
    }

    #[cfg(not(unix))]
    fn open(&self, _env: &EnvSnapshot, _cwd: &Path, _argv: &ExecArgv) -> Result<(), MuxError> {
        Err(MuxError::Unsupported(
            "herdr's control socket is a Unix domain socket".to_string(),
        ))
    }
}

#[cfg(unix)]
fn open_over_socket(env: &EnvSnapshot, cwd: &Path, argv: &ExecArgv) -> Result<(), MuxError> {
    let socket = env
        .get("HERDR_SOCKET_PATH")
        .filter(|path| !path.is_empty())
        .ok_or_else(|| MuxError::Failed("HERDR_SOCKET_PATH is unset".to_string()))?;
    // A lossy cwd would open the tab somewhere else; refuse instead.
    if cwd.to_str().is_none() {
        return Err(MuxError::Failed(
            "the working directory is not valid UTF-8, which herdr's JSON socket cannot carry"
                .to_string(),
        ));
    }
    let workspace = env.get("HERDR_WORKSPACE_ID").filter(|id| !id.is_empty());
    let request = layout_apply_request(workspace, cwd, argv, true);
    let mut line = serde_json::to_vec(&request)
        .map_err(|e| MuxError::Failed(format!("could not encode the herdr request: {e}")))?;
    line.push(b'\n');
    let reply = exchange(Path::new(socket), &line)?;
    parse_layout_apply_reply(&reply)
}

/// Longest wait for the connect, and for the reply to arrive after the request
/// is sent (T-33-05: this runs on the UI thread).
///
/// These are three separate bounds, not one: a launch can wait up to
/// `CONNECT_TIMEOUT` to connect, `CONNECT_TIMEOUT` to write and `REPLY_TIMEOUT`
/// for the reply, about 9 s in the worst case (review IN-04). The herdr CLI
/// calls used by focus have no deadline at all, the same exposure the tmux CLI
/// path has (T-33-13, accepted). A connect that times out leaves its helper
/// thread blocked in `connect(2)` until the kernel gives up.
#[cfg(unix)]
const CONNECT_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(2);
#[cfg(unix)]
const REPLY_TIMEOUT: std::time::Duration = std::time::Duration::from_secs(5);
/// The reply is one small line; anything larger is cut off.
#[cfg(unix)]
const MAX_REPLY_BYTES: usize = 1024 * 1024;

/// `connect(2)` bounded by `limit`. `UnixStream::connect` has no timeout and
/// blocks for good when the listener's backlog is full, so the connect runs on
/// a helper thread and the caller waits with `recv_timeout`; on a timeout the
/// helper is abandoned.
#[cfg(unix)]
fn connect_within(
    path: &Path,
    limit: std::time::Duration,
) -> Result<std::os::unix::net::UnixStream, MuxError> {
    use std::os::unix::net::UnixStream;

    let failed =
        |what: &str, err: std::io::Error| MuxError::Failed(format!("herdr socket {what}: {err}"));
    let (tx, rx) = std::sync::mpsc::channel();
    let target = path.to_path_buf();
    std::thread::Builder::new()
        .name("herdr-connect".to_owned())
        .spawn(move || {
            let _ = tx.send(UnixStream::connect(target));
        })
        .map_err(|e| failed("connect", e))?;
    match rx.recv_timeout(limit) {
        Ok(result) => result.map_err(|e| failed("connect", e)),
        Err(_) => Err(MuxError::Failed(format!(
            "herdr socket connect: no connection within {} s",
            limit.as_secs().max(1)
        ))),
    }
}

/// Send one request line and read one reply line, all bounded.
#[cfg(unix)]
fn exchange(socket: &Path, line: &[u8]) -> Result<Vec<u8>, MuxError> {
    use std::io::{Read, Write};
    use std::time::Instant;

    let failed =
        |what: &str, err: std::io::Error| MuxError::Failed(format!("herdr socket {what}: {err}"));
    let mut stream = connect_within(socket, CONNECT_TIMEOUT)?;
    let deadline = Instant::now() + REPLY_TIMEOUT;

    stream
        .set_write_timeout(Some(CONNECT_TIMEOUT))
        .map_err(|e| failed("set write timeout", e))?;
    stream
        .write_all(line)
        .map_err(|e| failed("send request", e))?;

    let no_reply = || {
        MuxError::Failed(format!(
            "herdr sent no reply within {} s",
            REPLY_TIMEOUT.as_secs()
        ))
    };
    let mut buf: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 8192];
    while !buf.ends_with(b"\n") && buf.len() < MAX_REPLY_BYTES {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(no_reply());
        }
        stream
            .set_read_timeout(Some(remaining))
            .map_err(|e| failed("set read timeout", e))?;
        match stream.read(&mut chunk) {
            Ok(0) => break,
            Ok(n) => buf.extend_from_slice(&chunk[..n]),
            Err(e) if e.kind() == std::io::ErrorKind::Interrupted => continue,
            Err(e)
                if matches!(
                    e.kind(),
                    std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                ) =>
            {
                return Err(no_reply());
            }
            Err(e) => return Err(failed("read reply", e)),
        }
    }
    if buf.is_empty() {
        return Err(MuxError::Failed(
            "herdr closed the connection without a reply".to_string(),
        ));
    }
    Ok(buf)
}

// ---- focus: the herdr CLI edge and the pane join ----------------------------

/// herdr's process name, for the server and for every client and CLI call.
const COMM: &str = "herdr";

/// Is `entry` a herdr CLIENT: the long-lived process that displays a server in a
/// terminal? **Pure** over the process facts.
///
/// herdr's client, its server and its short-lived CLI calls all have the comm
/// `herdr`, so the argv and the stdin decide:
///
/// * stdin must be a terminal. The server (stdin `/dev/null`) and every
///   `Command::output()` child (stdin null) are not on one.
/// * The arguments after `argv[0]` must be exactly one of `herdr` (attach to the
///   default session), `herdr session attach <name>` or `herdr --session <name>`.
///   `herdr server`, `herdr pane list`, `herdr agent focus ...` (this TUI's own
///   calls included) and `herdr api ...` are not clients.
/// * `--remote` and `--machine` clients never count: they display a server on
///   another machine, so nothing they show can be a pane focused here.
///
/// An argv shape this does not recognise is NOT a client. The cost of a miss is
/// a note ("no herdr client is attached"); the cost of a false hit would be to
/// select the wrong tmux pane, so the rule errs on the side of the note.
pub(crate) fn is_client(entry: &ProcEntry) -> bool {
    if entry.comm != COMM || entry.tty.is_none() {
        return false;
    }
    let args: Vec<&str> = entry.argv.iter().skip(1).map(String::as_str).collect();
    match args.as_slice() {
        [] => true,
        ["session", "attach", name] | ["--session", name] => {
            !name.is_empty() && !name.starts_with('-')
        }
        _ => false,
    }
}

/// The terminals of the clients among `procs`: sorted, without duplicates.
/// Several clients are all returned (several people or several terminals can
/// attach); the caller picks deterministically from them.
pub(crate) fn client_ttys_from(procs: &[ProcEntry]) -> Vec<String> {
    let mut ttys: Vec<String> = procs
        .iter()
        .filter(|entry| is_client(entry))
        .filter_map(|entry| entry.tty.clone())
        .collect();
    ttys.sort();
    ttys.dedup();
    ttys
}

/// The only `(group, verb)` pairs the CLI edge will run: two reads and two
/// focuses. The verbs that type into a pane's shell (`pane run`,
/// `pane send-text`, `pane send-keys`, `agent start`) are absent on purpose, so
/// they are unreachable from this module whatever a caller passes (D-12, D-13,
/// MUX-05, T-33-11).
const CLI_VERBS: [(&str, &str); 4] = [
    ("pane", "list"),
    ("pane", "process-info"),
    ("agent", "focus"),
    ("tab", "focus"),
];

impl HerdrBackend {
    /// Run one allowed herdr CLI verb and return its stdout. **Herdr-private**,
    /// deliberately not a runner shared across backends (D-04).
    ///
    /// * A `(args[0], args[1])` pair outside [`CLI_VERBS`] is refused BEFORE any
    ///   process is spawned.
    /// * `HERDR_SOCKET_PATH` comes from the snapshot and is passed explicitly
    ///   (D-13, research E7), so the call reaches the server the snapshot names
    ///   (for focus, the SESSION's own server, see `plan_focus`), and the edge
    ///   is testable without a live server.
    /// * A spawn error is `Failed("herdr <group> <verb> failed: <e>")`. A
    ///   non-zero exit is `Failed("herdr <group> <verb> exited <status>:
    ///   <stderr>")`, which keeps herdr's JSON error reachable by
    ///   [`error_code`].
    ///
    /// Blocking; each call measured under 10 ms (research E2). A wedged server
    /// could stall it, the same exposure the tmux CLI path has (T-33-13).
    fn herdr_cli(&self, env: &EnvSnapshot, args: &[&str]) -> Result<String, MuxError> {
        let (Some(group), Some(verb)) = (args.first(), args.get(1)) else {
            return Err(MuxError::RefusedForSafety(
                "refusing to run a herdr CLI call without a group and a verb".to_string(),
            ));
        };
        if !CLI_VERBS.contains(&(*group, *verb)) {
            return Err(MuxError::RefusedForSafety(format!(
                "refusing to run `herdr {group} {verb}`: only pane list, pane process-info, \
                 agent focus and tab focus are allowed, and the verbs that type into a pane's \
                 shell never are"
            )));
        }
        let socket = env
            .get("HERDR_SOCKET_PATH")
            .filter(|path| !path.is_empty())
            .ok_or_else(|| MuxError::Failed("HERDR_SOCKET_PATH is unset".to_string()))?;

        let output = Command::new(self.program)
            .args(args)
            .env("HERDR_SOCKET_PATH", socket)
            .output()
            .map_err(|e| MuxError::Failed(format!("herdr {group} {verb} failed: {e}")))?;
        if !output.status.success() {
            return Err(MuxError::Failed(format!(
                "herdr {group} {verb} exited {}: {}",
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            )));
        }
        Ok(String::from_utf8_lossy(&output.stdout).into_owned())
    }

    /// `herdr pane process-info --pane <id>`, parsed. `id` must already have
    /// passed [`validate_id`].
    fn process_info(&self, env: &EnvSnapshot, id: &str) -> Result<ProcessInfo, MuxError> {
        let out = self.herdr_cli(env, &["pane", "process-info", "--pane", id])?;
        parse_process_info(&out)
    }
}

/// One entry of `herdr pane list`. Every field is optional by construction and
/// unknown keys are ignored, so a newer herdr adding fields cannot break the
/// parse (research E2).
#[derive(Debug, Clone, Default, Deserialize)]
struct HerdrPane {
    #[serde(default)]
    pane_id: String,
    #[serde(default)]
    tab_id: Option<String>,
    #[serde(default)]
    cwd: Option<PathBuf>,
    #[serde(default)]
    foreground_cwd: Option<PathBuf>,
    /// The agent herdr detected in the pane (`"claude"`, ...). Absent on a
    /// pane herdr tracks no agent in, such as a plain shell.
    #[serde(default)]
    agent: Option<String>,
    #[serde(default)]
    agent_session: Option<AgentSession>,
}

/// herdr's record of which agent session a pane hosts.
#[derive(Debug, Clone, Default, Deserialize)]
struct AgentSession {
    #[serde(default)]
    kind: String,
    #[serde(default)]
    value: String,
}

/// What `herdr pane process-info` says about a pane, reduced to the pids the
/// join compares. Herdr reports no tty (research E2).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
struct ProcessInfo {
    foreground_pids: Vec<u32>,
    shell_pid: Option<u32>,
}

#[derive(Deserialize)]
struct PaneListEnvelope {
    #[serde(default)]
    result: Option<PaneListResult>,
}

#[derive(Deserialize)]
struct PaneListResult {
    #[serde(default)]
    panes: Vec<HerdrPane>,
}

#[derive(Deserialize)]
struct ProcessInfoEnvelope {
    #[serde(default)]
    result: Option<ProcessInfoResult>,
}

#[derive(Deserialize)]
struct ProcessInfoResult {
    #[serde(default)]
    process_info: Option<RawProcessInfo>,
}

#[derive(Deserialize)]
struct RawProcessInfo {
    #[serde(default)]
    foreground_processes: Vec<RawForeground>,
    #[serde(default)]
    shell_pid: Option<u32>,
}

#[derive(Deserialize)]
struct RawForeground {
    #[serde(default)]
    pid: Option<u32>,
}

/// Parse `herdr pane list` stdout. Anything that is not the expected envelope
/// is [`MuxError::Failed`].
fn parse_pane_list(json: &str) -> Result<Vec<HerdrPane>, MuxError> {
    let envelope: PaneListEnvelope = serde_json::from_str(json)
        .map_err(|e| MuxError::Failed(format!("herdr pane list is not the expected JSON: {e}")))?;
    envelope
        .result
        .map(|result| result.panes)
        .ok_or_else(|| MuxError::Failed("herdr pane list reply has no result".to_string()))
}

/// Parse `herdr pane process-info` stdout.
fn parse_process_info(json: &str) -> Result<ProcessInfo, MuxError> {
    let envelope: ProcessInfoEnvelope = serde_json::from_str(json).map_err(|e| {
        MuxError::Failed(format!(
            "herdr pane process-info is not the expected JSON: {e}"
        ))
    })?;
    let raw = envelope
        .result
        .and_then(|result| result.process_info)
        .ok_or_else(|| {
            MuxError::Failed("herdr pane process-info reply has no process_info".to_string())
        })?;
    Ok(ProcessInfo {
        foreground_pids: raw
            .foreground_processes
            .iter()
            .filter_map(|p| p.pid)
            .collect(),
        shell_pid: raw.shell_pid,
    })
}

/// The `error.code` of a herdr error reply, found anywhere in `text` (the CLI
/// prints `{"error":{"code":...}}`, and [`HerdrBackend::herdr_cli`] prefixes
/// the stderr with the verb and exit status).
fn error_code(text: &str) -> Option<String> {
    text.match_indices('{').find_map(|(at, _)| {
        let value = serde_json::Deserializer::from_str(&text[at..])
            .into_iter::<serde_json::Value>()
            .next()?
            .ok()?;
        value
            .get("error")?
            .get("code")?
            .as_str()
            .map(str::to_string)
    })
}

/// Refuse a pane or tab id that is empty or starts with `-`. The id comes from
/// herdr's output and becomes an argv element of herdr's own option parser, so
/// a leading `-` would be read as an option (CWE-88, T-33-10).
fn validate_id(id: &str) -> Result<&str, MuxError> {
    if id.is_empty() || id.starts_with('-') {
        return Err(MuxError::RefusedForSafety(format!(
            "refusing to pass the herdr id {id:?} to the herdr CLI: it is empty or would be \
             read as an option"
        )));
    }
    Ok(id)
}

/// The result of one join tier.
#[derive(Debug)]
enum Tier<'a> {
    /// Exactly one pane matched.
    One(&'a HerdrPane),
    /// Nothing matched; try the next tier.
    None,
    /// This many panes (two or more) matched. Never resolved by guessing.
    Ambiguous(usize),
}

impl<'a> Tier<'a> {
    fn from_matches(matches: Vec<&'a HerdrPane>) -> Self {
        match matches.as_slice() {
            [] => Tier::None,
            [one] => Tier::One(one),
            many => Tier::Ambiguous(many.len()),
        }
    }
}

/// The refusal for a tier with two or more matches (T-33-09).
fn ambiguous(count: usize, tier: &str) -> MuxError {
    MuxError::RefusedForSafety(format!(
        "refusing to guess: {count} herdr panes match this session by {tier}; focusing the \
         wrong one would send your keystrokes to another session"
    ))
}

/// Tier 1: panes whose `agent_session` is of kind `"id"` and whose value EQUALS
/// the session id, as raw strings. `None` or an empty id skips the tier. A
/// prefix or substring never matches, for the exact-match reason in the module
/// docs. The order of `panes` cannot change the outcome.
fn join_by_session_id<'a>(panes: &'a [HerdrPane], session_id: Option<&str>) -> Tier<'a> {
    let Some(session_id) = session_id.filter(|id| !id.is_empty()) else {
        return Tier::None;
    };
    Tier::from_matches(
        panes
            .iter()
            .filter(|pane| {
                pane.agent_session
                    .as_ref()
                    .is_some_and(|s| s.kind == "id" && s.value == session_id)
            })
            .collect(),
    )
}

/// The panes whose `cwd` or `foreground_cwd` is `working_dir`. Not restricted by
/// agent kind, so a Codex session is focusable too [inferred]; the narrowing to
/// an agent happens only in the last-resort tier ([`cwd_fallback`]).
///
/// The session's `working_dir` is the resolved `/proc/<pid>/cwd`, while herdr
/// may report the path as it was entered (through a symlink), so two paths are
/// the same directory when they are equal as written or resolve to the same
/// place ([`same_dir`], review WR-02).
fn cwd_candidates<'a>(panes: &'a [HerdrPane], working_dir: &Path) -> Vec<&'a HerdrPane> {
    panes
        .iter()
        .filter(|pane| {
            [pane.cwd.as_deref(), pane.foreground_cwd.as_deref()]
                .into_iter()
                .flatten()
                .any(|dir| same_dir(dir, working_dir))
        })
        .collect()
}

/// Equal as written, or both resolvable and resolving to the same path. A path
/// that cannot be resolved (it no longer exists, or lives on another machine's
/// view of the filesystem) only ever matches by exact equality.
fn same_dir(a: &Path, b: &Path) -> bool {
    a == b
        || matches!(
            (std::fs::canonicalize(a), std::fs::canonicalize(b)),
            (Ok(a), Ok(b)) if a == b
        )
}

/// Tier 2: a candidate whose foreground processes include `session_pid`, or
/// whose shell pid is one of `chain_pids` (the session pid and its `/proc`
/// ancestors). Integer equality only: pid 12 never matches 123.
fn join_by_pid<'a>(
    candidates: &[(&'a HerdrPane, ProcessInfo)],
    session_pid: u32,
    chain_pids: &[u32],
) -> Tier<'a> {
    Tier::from_matches(
        candidates
            .iter()
            .filter(|(_, info)| {
                info.foreground_pids.contains(&session_pid)
                    || info
                        .shell_pid
                        .is_some_and(|shell| chain_pids.contains(&shell))
            })
            .map(|(pane, _)| *pane)
            .collect(),
    )
}

/// The note on a focus that was matched by directory alone.
const CWD_ONLY_NOTE: &str = "matched by working directory only";

/// The name herdr gives the agent of a `kind` session in `pane list`'s `agent`
/// field. `"claude"` is measured (research E2); `"codex"` is [inferred] and
/// only ever narrows the last-resort tier, so a wrong name costs a missed
/// directory match, never a wrong focus.
fn agent_name(kind: SessionKind) -> &'static str {
    match kind {
        SessionKind::Claude => "claude",
        SessionKind::Codex => "codex",
    }
}

/// The panes the last-resort directory tier may choose among: `unverified`
/// (candidates whose process-info could not be read, so nothing contradicts
/// them) that herdr reports as running this session's kind of agent. A plain
/// shell pane in the same directory is never one of them (WR-01), and neither
/// is a pane whose process-info was read and named other processes: that pane
/// is positively not running this session.
fn cwd_fallback<'a>(unverified: &[&'a HerdrPane], kind: SessionKind) -> Vec<&'a HerdrPane> {
    unverified
        .iter()
        .copied()
        .filter(|pane| pane.agent.as_deref() == Some(agent_name(kind)))
        .collect()
}

/// Tier 3: the one candidate left when the working directory is the only fact.
fn join_by_cwd<'a>(candidates: &[&'a HerdrPane]) -> Tier<'a> {
    Tier::from_matches(candidates.to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn argv(parts: &[&str]) -> ExecArgv {
        ExecArgv::new(parts.iter().map(|s| (*s).to_string()).collect()).unwrap()
    }

    #[test]
    fn layout_apply_request_carries_a_hostile_element_as_one_array_element() {
        let hostile = "lit a b;touch X $HOME *";
        let req = layout_apply_request(
            Some("wT"),
            Path::new("/work/proj"),
            &argv(&["env", "claude", hostile, "last"]),
            true,
        );
        assert_eq!(req["id"], "gsdmm-open");
        assert_eq!(req["method"], "layout.apply");
        assert_eq!(req["params"]["workspace_id"], "wT");
        assert_eq!(req["params"]["focus"], true);
        assert_eq!(req["params"]["root"]["type"], "pane");
        assert_eq!(req["params"]["root"]["cwd"], "/work/proj");
        assert_eq!(
            req["params"]["root"]["command"],
            serde_json::json!(["env", "claude", hostile, "last"]),
            "elements are kept whole and in order"
        );
        // The serialized line is one JSON line: no raw newline from an element.
        let line = serde_json::to_string(&req).unwrap();
        assert!(!line.contains('\n'));
    }

    #[test]
    fn a_missing_workspace_serializes_as_json_null() {
        let req = layout_apply_request(None, Path::new("/x"), &argv(&["env", "claude"]), false);
        assert!(req["params"]["workspace_id"].is_null());
        assert_eq!(req["params"]["focus"], false);
    }

    #[test]
    fn replies_are_classified() {
        assert!(parse_layout_apply_reply(
            br#"{"id":"gsdmm-open","result":{"type":"layout_apply"}}"#
        )
        .is_ok());
        let err = parse_layout_apply_reply(
            br#"{"id":"x","error":{"code":"layout_apply_failed","message":"Unable to spawn env"}}"#,
        )
        .unwrap_err();
        assert_eq!(
            err,
            MuxError::Failed(
                "layout.apply failed (layout_apply_failed): Unable to spawn env".to_string()
            )
        );
        assert!(parse_layout_apply_reply(
            br#"{"id":"gsdmm-open","result":{"type":"layout_apply"},"error":null}"#
        )
        .is_ok());
        assert!(
            parse_layout_apply_reply(br#"{"id":"x","error":null}"#).is_err(),
            "a null error with no result is still not a success"
        );
        for bad in [
            &b""[..],
            b"   \n",
            b"[1,2]",
            b"\"str\"",
            b"{}",
            b"{\"id\":\"x\"}",
            b"not json",
        ] {
            let err = parse_layout_apply_reply(bad).unwrap_err();
            assert!(matches!(err, MuxError::Failed(_)), "{bad:?} -> {err:?}");
        }
    }

    // ---- which processes are herdr CLIENTS (the reveal step) ---------------

    fn proc_entry(pid: u32, comm: &str, argv: &[&str], tty: Option<&str>) -> ProcEntry {
        ProcEntry {
            pid,
            comm: comm.to_string(),
            argv: argv.iter().map(|a| (*a).to_string()).collect(),
            tty: tty.map(str::to_string),
        }
    }

    /// Every process of the live machine's shape, and the near misses a looser
    /// rule would count. A false hit selects the wrong tmux pane; a miss only
    /// costs a note, so unknown shapes are NOT clients.
    #[test]
    fn only_a_long_lived_local_client_on_a_terminal_is_a_herdr_client() {
        let rows: Vec<(&str, ProcEntry, bool)> = vec![
            (
                "the measured client",
                proc_entry(
                    3630647,
                    "herdr",
                    &["herdr", "session", "attach", "default"],
                    Some("pts/13"),
                ),
                true,
            ),
            (
                "a bare herdr (the default session)",
                proc_entry(1, "herdr", &["herdr"], Some("pts/1")),
                true,
            ),
            (
                "a path in argv[0]",
                proc_entry(1, "herdr", &["/home/blk/.local/bin/herdr"], Some("pts/1")),
                true,
            ),
            (
                "--session <name>",
                proc_entry(1, "herdr", &["herdr", "--session", "work"], Some("pts/1")),
                true,
            ),
            (
                "the headless server (stdin /dev/null)",
                proc_entry(671927, "herdr", &["herdr", "server"], None),
                false,
            ),
            (
                "the server even on a terminal",
                proc_entry(1, "herdr", &["herdr", "server"], Some("pts/1")),
                false,
            ),
            (
                "a client with no terminal on stdin",
                proc_entry(1, "herdr", &["herdr", "session", "attach", "default"], None),
                false,
            ),
            (
                "this TUI's own pane list call",
                proc_entry(1, "herdr", &["herdr", "pane", "list"], Some("pts/1")),
                false,
            ),
            (
                "agent focus",
                proc_entry(
                    1,
                    "herdr",
                    &["herdr", "agent", "focus", "w2:p1"],
                    Some("pts/1"),
                ),
                false,
            ),
            (
                "a --session CLI call with a subcommand",
                proc_entry(
                    1,
                    "herdr",
                    &["herdr", "--session", "work", "pane", "list"],
                    Some("pts/1"),
                ),
                false,
            ),
            (
                "a remote client displays another machine",
                proc_entry(1, "herdr", &["herdr", "--remote", "host"], Some("pts/1")),
                false,
            ),
            (
                "a remote client of a named session",
                proc_entry(
                    1,
                    "herdr",
                    &["herdr", "--remote", "host", "--session", "x"],
                    Some("pts/1"),
                ),
                false,
            ),
            (
                "a machine call",
                proc_entry(
                    1,
                    "herdr",
                    &["herdr", "--machine", "m", "pane", "list"],
                    Some("pts/1"),
                ),
                false,
            ),
            (
                "session attach with no name",
                proc_entry(1, "herdr", &["herdr", "session", "attach"], Some("pts/1")),
                false,
            ),
            (
                "session attach with an option as the name",
                proc_entry(
                    1,
                    "herdr",
                    &["herdr", "session", "attach", "--x"],
                    Some("pts/1"),
                ),
                false,
            ),
            (
                "another program that mentions herdr",
                proc_entry(1, "herdr-notes", &["herdr-notes"], Some("pts/1")),
                false,
            ),
        ];
        for (label, entry, expected) in rows {
            assert_eq!(is_client(&entry), expected, "{label}: {entry:?}");
        }
    }

    #[test]
    fn client_terminals_are_sorted_unique_and_clients_only() {
        let procs = vec![
            proc_entry(5, "herdr", &["herdr", "server"], None),
            proc_entry(
                9,
                "herdr",
                &["herdr", "session", "attach", "default"],
                Some("pts/9"),
            ),
            proc_entry(8, "herdr", &["herdr"], Some("pts/13")),
            proc_entry(7, "herdr", &["herdr", "--session", "work"], Some("pts/13")),
            proc_entry(6, "herdr", &["herdr", "pane", "list"], Some("pts/2")),
        ];
        assert_eq!(client_ttys_from(&procs), vec!["pts/13", "pts/9"]);
        assert!(client_ttys_from(&[]).is_empty());
    }

    #[test]
    fn the_backend_reports_clients_through_the_scan_it_is_handed() {
        let asked = std::cell::RefCell::new(Vec::new());
        let ttys = HERDR.client_ttys(&|comm| {
            asked.borrow_mut().push(comm.to_string());
            vec![proc_entry(9, "herdr", &["herdr"], Some("pts/13"))]
        });
        assert_eq!(ttys, Some(vec!["pts/13".to_string()]));
        assert_eq!(*asked.borrow(), vec!["herdr".to_string()]);
        assert_eq!(
            HERDR.client_ttys(&|_| Vec::new()),
            Some(Vec::new()),
            "herdr has the concept; no client is attached"
        );
    }

    #[test]
    fn detection_reads_exact_markers() {
        let d = |pairs: &[(&str, &str)]| HERDR.detect(&EnvSnapshot::from_pairs(pairs));
        assert!(d(&[]).is_none());
        assert!(d(&[("HERDR_ENV", "0")]).is_none());
        assert!(d(&[("HERDR_ENV", "true")]).is_none());
        assert!(d(&[("HERDR_ENV", "1"), ("HERDR_SOCKET_PATH", "")])
            .unwrap()
            .reachable
            .is_err());
        assert_eq!(
            d(&[("HERDR_ENV", "1"), ("HERDR_SOCKET_PATH", "/s")]).unwrap(),
            Detection {
                inside: true,
                reachable: Ok(())
            }
        );
    }

    /// A listener that reads the request and closes without replying gives a
    /// failure that says no reply arrived (T-33-05).
    #[cfg(unix)]
    #[test]
    fn a_listener_that_closes_without_a_reply_is_a_failure() {
        use std::io::{BufRead, BufReader};
        use std::os::unix::net::UnixListener;

        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("h.sock");
        let listener = UnixListener::bind(&path).unwrap();
        let handle = std::thread::spawn(move || {
            let stream =
                crate::multiplexer::accept_within(&listener, std::time::Duration::from_secs(10))
                    .expect("a client connects within 10 s");
            let mut line = String::new();
            BufReader::new(&stream).read_line(&mut line).unwrap();
        });
        let env = EnvSnapshot::from_pairs(&[
            ("HERDR_ENV", "1"),
            ("HERDR_SOCKET_PATH", path.to_str().unwrap()),
        ]);
        let err = HERDR
            .open(&env, Path::new("/work"), &argv(&["env", "claude"]))
            .unwrap_err();
        handle.join().unwrap();
        assert!(
            matches!(&err, MuxError::Failed(m) if m.contains("without a reply")),
            "{err:?}"
        );
    }

    #[cfg(unix)]
    #[test]
    fn a_socket_path_that_does_not_exist_is_a_failure_not_a_panic() {
        let env = EnvSnapshot::from_pairs(&[
            ("HERDR_ENV", "1"),
            ("HERDR_SOCKET_PATH", "/nonexistent/gsdmm/h.sock"),
        ]);
        let err = HERDR
            .open(&env, Path::new("/work"), &argv(&["env", "claude"]))
            .unwrap_err();
        assert!(matches!(err, MuxError::Failed(_)), "{err:?}");
    }

    // ---- live herdr (ignored): pins the measurements as re-runnable facts ---

    /// The live environment, or `None` (with a printed skip message) when this
    /// test is not running inside a herdr pane with a reachable socket.
    fn live_env() -> Option<EnvSnapshot> {
        let env = EnvSnapshot::from_process();
        let socket = env.get("HERDR_SOCKET_PATH").filter(|p| !p.is_empty());
        if env.get("HERDR_ENV") != Some("1") || socket.is_none() {
            println!("skipped: needs HERDR_ENV=1 and HERDR_SOCKET_PATH (run inside herdr)");
            return None;
        }
        Some(env)
    }

    /// **The EXEC-DIRECT measurement of RESEARCH E3 Probe B, re-runnable.**
    ///
    /// Sends one real `layout.apply` whose command is
    /// `env touch -- <dir>/<payload>`, where the payload is a hostile string
    /// with spaces, `;`, `$HOME` and `*`. Exec-direct means a file with exactly
    /// that name appears and no file named `INJECTED` does. The pane closes when
    /// `touch` exits, so no tab is left behind, and `focus: false` leaves the
    /// user's focused tab alone.
    #[cfg(unix)]
    #[test]
    #[ignore = "requires a live herdr server; run inside herdr"]
    fn the_herdr_layout_apply_exec_direct_boundary_holds_against_a_live_server() {
        let Some(env) = live_env() else {
            return;
        };
        let dir = tempfile::tempdir().expect("tempdir");
        let payload = "lit a b;touch INJECTED $HOME *";
        let argv = argv(&[
            "env",
            "touch",
            "--",
            &dir.path().join(payload).to_string_lossy(),
        ]);
        let workspace = env.get("HERDR_WORKSPACE_ID").filter(|id| !id.is_empty());
        let request = layout_apply_request(workspace, dir.path(), &argv, false);
        let mut line = serde_json::to_vec(&request).unwrap();
        line.push(b'\n');
        let socket = env.get("HERDR_SOCKET_PATH").unwrap();
        let reply = exchange(Path::new(socket), &line).expect("herdr answers layout.apply");
        parse_layout_apply_reply(&reply).expect("herdr accepts layout.apply");

        let literal = dir.path().join(payload);
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(2);
        while !literal.exists() && std::time::Instant::now() < deadline {
            std::thread::sleep(std::time::Duration::from_millis(25));
        }
        assert!(
            literal.exists(),
            "the payload must arrive as ONE argv element and become one file name"
        );
        assert!(
            !dir.path().join("INJECTED").exists(),
            "a file named INJECTED means a shell interpreted the payload"
        );
    }

    /// Read-only: the JSON field names the focus join depends on, against the
    /// real server. The pane list must contain this very pane, and its
    /// process-info must carry a foreground pid and a shell pid.
    #[test]
    #[ignore = "requires a live herdr server; run inside herdr"]
    fn the_live_herdr_pane_list_and_process_info_parse() {
        let Some(env) = live_env() else {
            return;
        };
        let Ok(own_pane) = std::env::var("HERDR_PANE_ID") else {
            println!("skipped: needs HERDR_PANE_ID (run inside a herdr pane)");
            return;
        };
        let listing = HERDR
            .herdr_cli(&env, &["pane", "list"])
            .expect("herdr pane list");
        let panes = parse_pane_list(&listing).expect("pane list parses");
        let own = panes
            .iter()
            .find(|p| p.pane_id == own_pane)
            .unwrap_or_else(|| panic!("pane list must contain {own_pane}"));
        assert!(own.tab_id.is_some(), "a pane carries its tab id");

        let info = HERDR
            .process_info(&env, validate_id(&own_pane).unwrap())
            .expect("herdr pane process-info");
        assert!(
            !info.foreground_pids.is_empty(),
            "at least one foreground pid: {info:?}"
        );
        assert!(info.shell_pid.is_some(), "a shell pid: {info:?}");
    }

    // ---- focus: parsing ----------------------------------------------------

    /// `herdr pane list`, abridged from RESEARCH E2: a Claude pane with
    /// `agent_session`, and a shell pane with neither `agent` nor
    /// `agent_session`. Unknown keys (`scroll`, `revision`) must be ignored.
    const PANE_LIST: &str = r#"{"id":"cli:pane:list","result":{"panes":[
        {"agent":"claude",
         "agent_session":{"agent":"claude","kind":"id","source":"herdr:claude",
                          "value":"0e4f6238-39fc-4d98-9a18-b7156e96bd4b"},
         "agent_status":"idle","cwd":"/home/blk/projects/flutter/bitcreed-apps",
         "focused":false,
         "foreground_cwd":"/home/blk/projects/flutter/bitcreed-apps",
         "pane_id":"w1:p1","revision":3,"scroll":{"offset":0},"tab_id":"w1:t1",
         "terminal_id":"term_65fdd7c960fcd1","terminal_title":"t",
         "terminal_title_stripped":"t","workspace_id":"w1"},
        {"cwd":"/tmp","pane_id":"w1:p2","tab_id":"w1:t1","workspace_id":"w1"}
    ]}}"#;

    /// `herdr pane process-info --current`, verbatim from RESEARCH E2.
    const PROCESS_INFO: &str = r#"{"id":"cli:pane:process_info","result":{"process_info":{"foreground_process_group_id":3620262,"foreground_processes":[{"argv":["claude"],"cmdline":"claude","cwd":"/home/blk/projects/rust/gsd-meta-manager","name":"claude","pid":3620262}],"pane_id":"wA:p1","shell_pid":3620011},"type":"pane_process_info"}}"#;

    #[test]
    fn parse_pane_list_reads_an_agent_pane_and_a_shell_pane() {
        let panes = parse_pane_list(PANE_LIST).unwrap();
        assert_eq!(panes.len(), 2);
        assert_eq!(panes[0].pane_id, "w1:p1");
        assert_eq!(panes[0].tab_id.as_deref(), Some("w1:t1"));
        let session = panes[0].agent_session.as_ref().unwrap();
        assert_eq!(session.kind, "id");
        assert_eq!(session.value, "0e4f6238-39fc-4d98-9a18-b7156e96bd4b");
        assert_eq!(panes[1].pane_id, "w1:p2");
        assert!(panes[1].agent_session.is_none());
    }

    #[test]
    fn parse_pane_list_rejects_garbage_and_a_reply_without_a_result() {
        for bad in ["", "not json", "[1]", "{}", r#"{"error":{"code":"x"}}"#] {
            assert!(
                matches!(parse_pane_list(bad), Err(MuxError::Failed(_))),
                "{bad:?}"
            );
        }
    }

    #[test]
    fn parse_process_info_reads_the_verbatim_reply() {
        let info = parse_process_info(PROCESS_INFO).unwrap();
        assert_eq!(info.foreground_pids, vec![3620262]);
        assert_eq!(info.shell_pid, Some(3620011));
        assert!(matches!(parse_process_info("{}"), Err(MuxError::Failed(_))));
        assert!(matches!(
            parse_process_info("nope"),
            Err(MuxError::Failed(_))
        ));
    }

    #[test]
    fn error_code_is_found_inside_a_cli_failure_message() {
        let message = r#"herdr agent focus exited exit status: 1: {"error":{"code":"agent_not_found","message":"agent target w2:p2 not found"},"id":"cli"}"#;
        assert_eq!(error_code(message).as_deref(), Some("agent_not_found"));
        assert_eq!(error_code("exited 1: boom"), None);
        assert_eq!(error_code(r#"{"result":{}}"#), None);
        assert_eq!(error_code(""), None);
    }

    #[test]
    fn validate_id_refuses_empty_and_dash_leading_ids() {
        assert!(matches!(
            validate_id(""),
            Err(MuxError::RefusedForSafety(_))
        ));
        assert!(matches!(
            validate_id("-x"),
            Err(MuxError::RefusedForSafety(_))
        ));
        assert!(matches!(
            validate_id("--help"),
            Err(MuxError::RefusedForSafety(_))
        ));
        assert_eq!(validate_id("w1:p1").unwrap(), "w1:p1");
    }

    // ---- focus: the join tiers (pure) -------------------------------------

    fn pane(id: &str, tab: &str, cwd: &str, session: Option<(&str, &str)>) -> HerdrPane {
        HerdrPane {
            pane_id: id.to_string(),
            tab_id: Some(tab.to_string()),
            cwd: Some(PathBuf::from(cwd)),
            foreground_cwd: None,
            agent: None,
            agent_session: session.map(|(kind, value)| AgentSession {
                kind: kind.to_string(),
                value: value.to_string(),
            }),
        }
    }

    fn describe(tier: &Tier<'_>) -> String {
        match tier {
            Tier::One(pane) => pane.pane_id.clone(),
            Tier::None => "none".to_string(),
            Tier::Ambiguous(n) => format!("ambiguous:{n}"),
        }
    }

    fn info(foreground: &[u32], shell: Option<u32>) -> ProcessInfo {
        ProcessInfo {
            foreground_pids: foreground.to_vec(),
            shell_pid: shell,
        }
    }

    #[test]
    fn session_id_tier_matches_only_an_exact_value_of_kind_id() {
        let panes = vec![
            pane("a", "t", "/x", Some(("id", "sid-1"))),
            pane("b", "t", "/x", Some(("id", "sid-2"))),
            pane("c", "t", "/x", None),
        ];
        assert_eq!(describe(&join_by_session_id(&panes, Some("sid-2"))), "b");
        assert_eq!(describe(&join_by_session_id(&panes, Some("sid-3"))), "none");
        assert_eq!(
            describe(&join_by_session_id(&panes, Some("sid"))),
            "none",
            "a prefix of the id does not match"
        );
        assert_eq!(
            describe(&join_by_session_id(&panes, Some("sid-22"))),
            "none",
            "a longer id does not match either"
        );
        assert_eq!(
            describe(&join_by_session_id(&panes, Some(""))),
            "none",
            "an empty id matches nothing"
        );
    }

    #[test]
    fn session_id_tier_is_skipped_without_a_session_id() {
        let panes = vec![pane("a", "t", "/x", Some(("id", "sid-1")))];
        assert_eq!(describe(&join_by_session_id(&panes, None)), "none");
    }

    #[test]
    fn session_id_tier_ignores_a_value_of_another_kind() {
        let panes = vec![pane("a", "t", "/x", Some(("name", "sid-1")))];
        assert_eq!(describe(&join_by_session_id(&panes, Some("sid-1"))), "none");
    }

    #[test]
    fn session_id_tier_refuses_two_panes_with_the_same_value() {
        let panes = vec![
            pane("a", "t", "/x", Some(("id", "sid-1"))),
            pane("b", "t", "/x", Some(("id", "sid-1"))),
        ];
        assert_eq!(
            describe(&join_by_session_id(&panes, Some("sid-1"))),
            "ambiguous:2"
        );
    }

    #[test]
    fn pid_tier_matches_the_foreground_pid_or_a_shell_pid_in_the_chain() {
        let a = pane("a", "t", "/x", None);
        let b = pane("b", "t", "/x", None);

        let by_foreground = [(&a, info(&[123], Some(7))), (&b, info(&[999], Some(8)))];
        assert_eq!(describe(&join_by_pid(&by_foreground, 123, &[123])), "a");

        let by_shell = [(&a, info(&[1], Some(7))), (&b, info(&[999], Some(8)))];
        assert_eq!(describe(&join_by_pid(&by_shell, 123, &[123, 8, 1])), "b");

        assert_eq!(describe(&join_by_pid(&by_shell, 555, &[555])), "none");
    }

    #[test]
    fn pid_tier_never_matches_a_pid_that_is_only_a_prefix() {
        let a = pane("a", "t", "/x", None);
        let candidates = [(&a, info(&[123], Some(1230)))];
        assert_eq!(describe(&join_by_pid(&candidates, 12, &[12])), "none");
        assert_eq!(describe(&join_by_pid(&candidates, 1, &[1])), "none");
        assert_eq!(describe(&join_by_pid(&candidates, 12, &[12, 123])), "none");
        assert_eq!(describe(&join_by_pid(&candidates, 123, &[123])), "a");
    }

    #[test]
    fn pid_tier_refuses_two_matching_panes() {
        let a = pane("a", "t", "/x", None);
        let b = pane("b", "t", "/x", None);
        let candidates = [(&a, info(&[42], None)), (&b, info(&[42], None))];
        assert_eq!(
            describe(&join_by_pid(&candidates, 42, &[42])),
            "ambiguous:2"
        );
    }

    #[test]
    fn cwd_tier_needs_exactly_one_candidate() {
        let a = pane("a", "t", "/work/proj", None);
        let b = pane("b", "t", "/work/proj", None);
        let c = pane("c", "t", "/work/other", None);
        let panes = vec![a, b, c];

        let none = cwd_candidates(&panes, Path::new("/work/nowhere"));
        assert_eq!(describe(&join_by_cwd(&none)), "none");

        let one = cwd_candidates(&panes, Path::new("/work/other"));
        assert_eq!(describe(&join_by_cwd(&one)), "c");

        let two = cwd_candidates(&panes, Path::new("/work/proj"));
        assert_eq!(describe(&join_by_cwd(&two)), "ambiguous:2");

        let prefix = cwd_candidates(&panes, Path::new("/work"));
        assert!(prefix.is_empty(), "a parent directory is not a match");
    }

    #[test]
    fn the_cwd_fallback_only_considers_panes_running_the_sessions_agent() {
        let shell = pane("shell", "t", "/work/proj", None);
        let mut claude = pane("claude", "t", "/work/proj", None);
        claude.agent = Some("claude".to_string());
        let mut codex = pane("codex", "t", "/work/proj", None);
        codex.agent = Some("codex".to_string());
        let all = [&shell, &claude, &codex];

        let ids = |kind| -> Vec<String> {
            cwd_fallback(&all, kind)
                .iter()
                .map(|p| p.pane_id.clone())
                .collect()
        };
        assert_eq!(ids(SessionKind::Claude), vec!["claude"]);
        assert_eq!(ids(SessionKind::Codex), vec!["codex"]);
        assert!(cwd_fallback(&[&shell], SessionKind::Claude).is_empty());
    }

    #[cfg(unix)]
    #[test]
    fn a_pane_reporting_a_symlinked_path_is_a_candidate_for_the_resolved_directory() {
        let dir = tempfile::tempdir().unwrap();
        let real = dir.path().join("real");
        std::fs::create_dir(&real).unwrap();
        let link = dir.path().join("link");
        std::os::unix::fs::symlink(&real, &link).unwrap();
        let other = dir.path().join("other");
        std::fs::create_dir(&other).unwrap();

        let panes = vec![
            pane("via-link", "t", link.to_str().unwrap(), None),
            pane("elsewhere", "t", other.to_str().unwrap(), None),
        ];
        let ids: Vec<&str> = cwd_candidates(&panes, &std::fs::canonicalize(&real).unwrap())
            .iter()
            .map(|p| p.pane_id.as_str())
            .collect();
        assert_eq!(ids, vec!["via-link"]);
        assert!(cwd_candidates(&panes, &dir.path().join("missing")).is_empty());
    }

    #[test]
    fn cwd_candidates_also_read_the_foreground_cwd() {
        let mut p = pane("a", "t", "/shell/dir", None);
        p.foreground_cwd = Some(PathBuf::from("/work/proj"));
        let panes = vec![p];
        assert_eq!(cwd_candidates(&panes, Path::new("/work/proj")).len(), 1);
    }

    #[test]
    fn the_order_of_panes_never_decides_the_match() {
        let forward = vec![
            pane("a", "t", "/work/proj", Some(("id", "sid-1"))),
            pane("b", "t", "/work/other", Some(("id", "sid-2"))),
        ];
        let reversed: Vec<HerdrPane> = forward.iter().rev().cloned().collect();
        for panes in [&forward, &reversed] {
            assert_eq!(describe(&join_by_session_id(panes, Some("sid-2"))), "b");
            let cwd = cwd_candidates(panes, Path::new("/work/proj"));
            assert_eq!(describe(&join_by_cwd(&cwd)), "a");
        }

        let (a, b) = (&forward[0], &forward[1]);
        let one_way = [(a, info(&[5], None)), (b, info(&[6], None))];
        let other_way = [(b, info(&[6], None)), (a, info(&[5], None))];
        assert_eq!(describe(&join_by_pid(&one_way, 5, &[5])), "a");
        assert_eq!(describe(&join_by_pid(&other_way, 5, &[5])), "a");
    }

    // ---- focus: the CLI edge against a fixture program --------------------

    #[cfg(unix)]
    mod cli {
        use super::*;
        use crate::session_detector::SessionKind;
        use crate::text::Untrusted;
        use std::os::unix::fs::PermissionsExt;

        // The one lock every fixture-script test in this crate takes (see
        // `fixtures::EXEC_LOCK`): ETXTBSY comes from a fork on ANY other test
        // thread, so a lock of its own would not cover the tests of the other
        // modules.
        use crate::multiplexer::fixtures::EXEC_LOCK;

        const SOCKET: &str = "/run/fake/herdr.sock";

        const AGENT_FOCUS_OK: &str = r#"echo '{"id":"cli:agent:focus","result":{"type":"ok"}}'"#;
        const AGENT_FOCUS_NOT_FOUND: &str = r#"echo '{"error":{"code":"agent_not_found","message":"agent target not found"},"id":"cli"}' >&2; exit 1"#;

        fn env() -> EnvSnapshot {
            EnvSnapshot::from_pairs(&[("HERDR_ENV", "1"), ("HERDR_SOCKET_PATH", SOCKET)])
        }

        fn session(pid: u32, session_id: Option<&str>, working_dir: &str) -> ClaudeSession {
            ClaudeSession {
                pid,
                kind: SessionKind::Claude,
                session_id: session_id.map(|s| Untrusted::from_untrusted_source(s.to_string())),
                working_dir: PathBuf::from(working_dir),
                start_time: None,
                tty: None,
            }
        }

        fn process_info_json(foreground: u32, shell: u32) -> String {
            format!(
                r#"{{"id":"cli:pane:process_info","result":{{"process_info":{{"foreground_processes":[{{"pid":{foreground}}}],"shell_pid":{shell}}},"type":"pane_process_info"}}}}"#
            )
        }

        fn pane_list_json(panes: &[(&str, &str, &str, Option<&str>)]) -> String {
            let rows: Vec<String> = panes
                .iter()
                .map(|(id, tab, cwd, sid)| {
                    let agent = match sid {
                        Some(v) => format!(
                            r#","agent":"claude","agent_session":{{"kind":"id","value":"{v}"}}"#
                        ),
                        None => String::new(),
                    };
                    format!(r#"{{"pane_id":"{id}","tab_id":"{tab}","cwd":"{cwd}"{agent}}}"#)
                })
                .collect();
            format!(
                r#"{{"id":"cli:pane:list","result":{{"panes":[{}]}}}}"#,
                rows.join(",")
            )
        }

        /// Panes herdr reports as running a Claude agent but with no
        /// `agent_session` record: `(pane id, tab id, cwd)`.
        fn claude_pane_list_json(panes: &[(&str, &str, &str)]) -> String {
            let rows: Vec<String> = panes
                .iter()
                .map(|(id, tab, cwd)| {
                    format!(
                        r#"{{"pane_id":"{id}","tab_id":"{tab}","cwd":"{cwd}","agent":"claude"}}"#
                    )
                })
                .collect();
            format!(
                r#"{{"id":"cli:pane:list","result":{{"panes":[{}]}}}}"#,
                rows.join(",")
            )
        }

        /// Write a fixture `herdr`, run `f` against it under the lock, retrying
        /// a residual ETXTBSY, and return the result with the invocation log.
        fn with_fixture<T>(
            pane_list: &str,
            process_info: &str,
            agent_focus: &str,
            mut f: impl FnMut(&'static HerdrBackend, &EnvSnapshot) -> Result<T, MuxError>,
        ) -> (Result<T, MuxError>, Vec<String>) {
            let _guard = EXEC_LOCK.lock().unwrap_or_else(|e| e.into_inner());
            let dir = tempfile::tempdir().expect("tempdir");
            let log = dir.path().join("log");
            std::fs::write(dir.path().join("pane_list.json"), pane_list).unwrap();
            std::fs::write(dir.path().join("process_info.json"), process_info).unwrap();
            let script = format!(
                "#!/bin/sh\n\
                 echo \"$* sock=$HERDR_SOCKET_PATH\" >> \"{log}\"\n\
                 case \"$1 $2\" in\n\
                 \"pane list\") cat \"{dir}/pane_list.json\" ;;\n\
                 \"pane process-info\") cat \"{dir}/process_info.json\" ;;\n\
                 \"agent focus\") {agent_focus} ;;\n\
                 \"tab focus\") echo '{{\"id\":\"cli:tab:focus\",\"result\":{{\"type\":\"ok\"}}}}' ;;\n\
                 *) echo \"unexpected: $*\" >&2; exit 2 ;;\n\
                 esac\n",
                log = log.display(),
                dir = dir.path().display(),
            );
            let program = dir.path().join("herdr");
            std::fs::write(&program, script).unwrap();
            std::fs::set_permissions(&program, std::fs::Permissions::from_mode(0o755)).unwrap();
            let backend: &'static HerdrBackend = Box::leak(Box::new(HerdrBackend {
                program: Box::leak(program.to_string_lossy().into_owned().into_boxed_str()),
            }));

            let mut result = f(backend, &env());
            for _ in 1..5 {
                match &result {
                    Err(MuxError::Failed(m)) if m.contains("Text file busy") => {
                        std::thread::sleep(std::time::Duration::from_millis(20));
                        result = f(backend, &env());
                    }
                    _ => break,
                }
            }
            let lines = std::fs::read_to_string(&log)
                .map(|t| t.lines().map(str::to_string).collect())
                .unwrap_or_default();
            (result, lines)
        }

        fn focus_with_fixture(
            panes: &str,
            process_info: &str,
            agent_focus: &str,
            session: &ClaudeSession,
        ) -> (Result<FocusOutcome, MuxError>, Vec<String>) {
            with_fixture(panes, process_info, agent_focus, |backend, env| {
                backend.focus(env, session)
            })
        }

        fn sock(line: &str) -> String {
            format!("{line} sock={SOCKET}")
        }

        #[test]
        fn focus_in_herdr_lands_in_the_matching_pane() {
            let panes = pane_list_json(&[
                ("w1:p1", "w1:t1", "/work/other", Some("sid-AAA")),
                ("w1:p2", "w1:t2", "/work/proj", Some("sid-BBB")),
            ]);
            let (result, log) = focus_with_fixture(
                &panes,
                &process_info_json(1, 2),
                AGENT_FOCUS_OK,
                &session(4242, Some("sid-BBB"), "/work/proj"),
            );
            assert_eq!(
                result.unwrap(),
                FocusOutcome {
                    by: MatchedBy::SessionId,
                    partial: None
                }
            );
            assert_eq!(
                log,
                vec![sock("pane list"), sock("agent focus w1:p2")],
                "pane list, then agent focus on the matched pane, and every call saw the \
                 snapshot's socket"
            );
        }

        #[test]
        fn focus_falls_back_to_tab_focus_when_herdr_tracks_no_agent() {
            let panes = pane_list_json(&[
                ("w1:p2", "w1:t2", "/work/proj", None),
                ("w1:p3", "w1:t2", "/work/elsewhere", None),
            ]);
            let (result, log) = focus_with_fixture(
                &panes,
                &process_info_json(4242, 4_100_000_000),
                AGENT_FOCUS_NOT_FOUND,
                &session(4242, None, "/work/proj"),
            );
            let outcome = result.unwrap();
            assert_eq!(outcome.by, MatchedBy::Pid);
            assert!(
                outcome
                    .partial
                    .as_deref()
                    .is_some_and(|note| note.contains("2 panes")),
                "the tab has two panes, so the result is partial: {outcome:?}"
            );
            assert_eq!(
                log,
                vec![
                    sock("pane list"),
                    sock("pane process-info --pane w1:p2"),
                    sock("agent focus w1:p2"),
                    sock("tab focus w1:t2"),
                ]
            );
        }

        #[test]
        fn tab_focus_is_exact_when_the_tab_holds_only_that_pane() {
            let panes = pane_list_json(&[("w1:p2", "w1:t2", "/work/proj", None)]);
            let (result, log) = focus_with_fixture(
                &panes,
                &process_info_json(4242, 2),
                AGENT_FOCUS_NOT_FOUND,
                &session(4242, None, "/work/proj"),
            );
            let outcome = result.unwrap();
            assert_eq!(outcome.by, MatchedBy::Pid);
            assert_eq!(outcome.partial, None);
            assert_eq!(
                log.last().map(String::as_str),
                Some(sock("tab focus w1:t2").as_str())
            );
        }

        #[test]
        fn two_panes_matching_by_pid_are_refused_through_focus() {
            let panes = pane_list_json(&[
                ("w1:p2", "w1:t2", "/work/proj", None),
                ("w1:p3", "w1:t3", "/work/proj", None),
            ]);
            // The fixture answers every process-info call alike, so both panes
            // match on the session pid and the tier is ambiguous.
            let (result, log) = focus_with_fixture(
                &panes,
                &process_info_json(4242, 2),
                AGENT_FOCUS_OK,
                &session(4242, None, "/work/proj"),
            );
            let err = result.unwrap_err();
            assert!(
                matches!(&err, MuxError::RefusedForSafety(m) if m.contains("refusing to guess")),
                "{err:?}"
            );
            assert!(
                !log.iter().any(|l| l.contains("agent focus")),
                "nothing was focused: {log:?}"
            );
        }

        #[test]
        fn an_ambiguous_cwd_is_refused_and_nothing_is_focused() {
            let panes = claude_pane_list_json(&[
                ("w1:p2", "w1:t2", "/work/proj"),
                ("w1:p3", "w1:t3", "/work/proj"),
            ]);
            // process-info is unreadable, so only the directory is left.
            let (result, log) = focus_with_fixture(
                &panes,
                "not json",
                AGENT_FOCUS_OK,
                &session(4242, None, "/work/proj"),
            );
            let err = result.unwrap_err();
            assert!(
                matches!(&err, MuxError::RefusedForSafety(m) if m.contains("refusing to guess")),
                "{err:?}"
            );
            assert!(!log.iter().any(|l| l.starts_with("agent focus")), "{log:?}");
            assert!(!log.iter().any(|l| l.starts_with("tab focus")), "{log:?}");
        }

        #[test]
        fn two_panes_with_the_same_session_id_are_refused() {
            let panes = pane_list_json(&[
                ("w1:p1", "w1:t1", "/a", Some("sid-X")),
                ("w1:p2", "w1:t2", "/b", Some("sid-X")),
            ]);
            let (result, log) = focus_with_fixture(
                &panes,
                &process_info_json(1, 2),
                AGENT_FOCUS_OK,
                &session(4242, Some("sid-X"), "/a"),
            );
            assert!(matches!(result, Err(MuxError::RefusedForSafety(_))));
            assert_eq!(log, vec![sock("pane list")]);
        }

        #[test]
        fn no_matching_pane_is_not_found_and_names_herdr() {
            for panes in [
                pane_list_json(&[]),
                pane_list_json(&[("w1:p1", "w1:t1", "/elsewhere", None)]),
            ] {
                let (result, _) = focus_with_fixture(
                    &panes,
                    &process_info_json(1, 2),
                    AGENT_FOCUS_OK,
                    &session(4242, None, "/work/proj"),
                );
                let err = result.unwrap_err();
                assert_eq!(
                    err,
                    MuxError::NotFound(
                        "no herdr pane runs this session (session may have exited)".to_string()
                    )
                );
            }
        }

        #[test]
        fn a_claude_pane_in_the_same_directory_is_focused_when_process_info_is_unreadable() {
            // `process_info.json` is not JSON, so the pid tier sees no
            // candidate, and the one Claude pane in the directory is focused,
            // with a note that the match was by directory.
            let panes = claude_pane_list_json(&[("w1:p2", "w1:t2", "/work/proj")]);
            let (result, log) = focus_with_fixture(
                &panes,
                "not json",
                AGENT_FOCUS_OK,
                &session(4242, None, "/work/proj"),
            );
            let outcome = result.unwrap();
            assert_eq!(outcome.by, MatchedBy::Cwd);
            assert_eq!(outcome.partial.as_deref(), Some(CWD_ONLY_NOTE));
            assert_eq!(
                log.last().map(String::as_str),
                Some(sock("agent focus w1:p2").as_str())
            );
        }

        #[test]
        fn a_plain_shell_pane_in_the_same_directory_is_never_focused() {
            // WR-01: no agent in the pane, process-info unreadable.
            let panes = pane_list_json(&[("w1:p2", "w1:t2", "/work/proj", None)]);
            let (result, log) = focus_with_fixture(
                &panes,
                "not json",
                AGENT_FOCUS_OK,
                &session(4242, None, "/work/proj"),
            );
            assert!(matches!(result, Err(MuxError::NotFound(_))), "{result:?}");
            assert!(!log.iter().any(|l| l.starts_with("agent focus")), "{log:?}");
            assert!(!log.iter().any(|l| l.starts_with("tab focus")), "{log:?}");
        }

        #[test]
        fn a_pane_whose_process_info_names_other_processes_is_not_the_session() {
            // WR-01: another Claude runs in this directory (herdr reports
            // foreground pid 777, not the session's 4242). It must not be
            // focused on the strength of the directory alone.
            let panes = claude_pane_list_json(&[("w1:p2", "w1:t2", "/work/proj")]);
            let (result, log) = focus_with_fixture(
                &panes,
                &process_info_json(777, 2),
                AGENT_FOCUS_OK,
                &session(4242, None, "/work/proj"),
            );
            assert!(matches!(result, Err(MuxError::NotFound(_))), "{result:?}");
            assert!(!log.iter().any(|l| l.starts_with("agent focus")), "{log:?}");
        }

        #[test]
        fn a_dash_leading_pane_id_is_refused_before_it_reaches_argv() {
            let panes = pane_list_json(&[("-x", "w1:t2", "/work/proj", Some("sid-1"))]);
            let (result, log) = focus_with_fixture(
                &panes,
                &process_info_json(1, 2),
                AGENT_FOCUS_OK,
                &session(4242, Some("sid-1"), "/work/proj"),
            );
            assert!(matches!(result, Err(MuxError::RefusedForSafety(_))));
            assert_eq!(log, vec![sock("pane list")]);
        }

        #[test]
        fn herdr_cli_refuses_every_verb_outside_the_allowlist() {
            let (result, log) = with_fixture("{}", "{}", AGENT_FOCUS_OK, |backend, env| {
                for args in [
                    &["pane", "run", "w1:p1", "ls"][..],
                    &["pane", "send-text", "w1:p1", "ls"],
                    &["pane", "send-keys", "w1:p1", "Enter"],
                    &["agent", "start", "claude"],
                    &["pane"],
                    &[],
                ] {
                    let err = backend.herdr_cli(env, args).unwrap_err();
                    assert!(
                        matches!(err, MuxError::RefusedForSafety(_)),
                        "{args:?} -> {err:?}"
                    );
                }
                Ok(())
            });
            result.unwrap();
            assert!(log.is_empty(), "no process was spawned: {log:?}");
        }

        #[test]
        fn focus_without_a_socket_path_fails_before_spawning() {
            let (result, log) = with_fixture("{}", "{}", AGENT_FOCUS_OK, |backend, _| {
                backend.focus(
                    &EnvSnapshot::from_pairs(&[("HERDR_ENV", "1")]),
                    &session(1, None, "/w"),
                )
            });
            assert!(matches!(result, Err(MuxError::Failed(m)) if m.contains("HERDR_SOCKET_PATH")));
            assert!(log.is_empty());
        }

        #[test]
        fn a_program_that_does_not_exist_is_a_failure_not_a_panic() {
            let backend = HerdrBackend {
                program: "/nonexistent/gsdmm/herdr",
            };
            let err = backend.focus(&env(), &session(1, None, "/w")).unwrap_err();
            assert!(
                matches!(&err, MuxError::Failed(m) if m.contains("pane list failed")),
                "{err:?}"
            );
        }
    }
}
