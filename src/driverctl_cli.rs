//! The sandbox side of `driverctl`: `driverctl log|show|hook` (Phase 27, DCTL-01).
//!
//! The driver's unchanged prompt calls `driverctl log ...` and the Claude hooks call
//! `driverctl hook`; both re-enter this binary, which speaks spike 002's wire
//! protocol to the launcher-side server over a unix socket.
//!
//! # Numbered facts
//!
//! 1. **This is the sandbox side, and it writes no file.** It validates nothing a
//!    server would not, sends exactly ONE JSON request per connection and reads ONE
//!    reply line. The run directory is mounted read-only for the driver; the only
//!    writer of the log and of the run state is the launcher-side store (DCTL-01,
//!    single writer). Nothing here touches the filesystem except the socket.
//! 2. **It runs before tracing init** (D-25). `main` dispatches
//!    `Commands::Driverctl` before `init_tracing`, `color_eyre::install`, config
//!    loading and the tokio runtime, because the sandbox's data dir may be
//!    unwritable and a panic here (rc 101) is, for a hook, a non-blocking error
//!    that lets the dispatch through (T-27-11).
//! 3. **Exit codes follow the spike, with one split** (27-RESEARCH A6): `0` ok, `2`
//!    for a server `REJECTED` reply and for a usage error (argparse parity), `1`
//!    for a transport failure or an unset `DRIVERCTL_SOCK`. A caller can tell "fix
//!    your arguments" (2) from "the launcher is not there" (1).
//! 4. **The dispatch gate fails closed** (T-27-12). For a PreToolUse `Agent|Task`
//!    payload with no `agent_id`, `driverctl hook` prints the deny JSON on a
//!    transport error, an unset socket AND a server reply whose `ok` is not true.
//!    The last two are deviations from the spike, which failed closed only on an
//!    `OSError` (`[inferred — audit]` 27-RESEARCH A8): launcher caps must not be
//!    bypassable by breaking the transport or by a server-side internal error.
//!    A server `deny` is relayed. Unreadable or non-object stdin ALSO prints the deny
//!    JSON (SEC-01 `[inferred — audit]`): without a parsed payload the client cannot
//!    tell a gate from any other hook, and "silent exit 0" would let a mangled
//!    dispatch through. Non-gate hooks with a parsed payload exit `0` silently, and
//!    the exit code is `0` in every case: the decision is the JSON.
//! 5. **Phase 28 hand-off.** Phase 28's launcher seeds the PreToolUse `Agent|Task`
//!    hook with `"onFailure": "block"`, so even a non-zero exit of this binary (a
//!    crash this module could not prevent: a missing binary, a killed process)
//!    cannot open the gate. This module closes the failures it can see; that
//!    setting closes the ones it cannot (D-25, T-27-11).
//!
//! # Wire format
//!
//! Request: one JSON object and `\n`. Reply: one JSON object and `\n`.
//! `{"op":"log","event":E,"args":{...}}` carries the spike's underscored argparse
//! keys (`done_when`, `loop_mode`) with string values; `{"op":"show"}` and
//! `{"op":"hook","payload":{...}}` are the other two verbs.

use std::io::{IsTerminal, Read, Write};
use std::time::Duration;

use gsd_meta_manager::cli::{DriverctlAction, DriverctlLog};
use serde_json::{json, Map, Value};

/// How long a connect, a write or the wait for a reply may take, in total (the
/// connect is bounded by [`connect_within`], WR-08).
///
/// Matches the spike client's `settimeout(15)`. It is a TOTAL deadline for the
/// reply here rather than a per-`recv` timeout: a server trickling one byte every
/// 14 s would otherwise hold the client (and a hook) open indefinitely (T-27-14).
#[cfg_attr(not(unix), allow(dead_code))]
pub const CLIENT_TIMEOUT: Duration = Duration::from_secs(15);

/// The largest reply the client reads. A reply is a status block or one error
/// line; a megabyte is far beyond either and bounds a hostile or runaway server.
#[cfg_attr(not(unix), allow(dead_code))]
const MAX_REPLY_BYTES: u64 = 1024 * 1024;

/// Why a request did not produce a reply object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClientError {
    /// `DRIVERCTL_SOCK` is unset or empty.
    #[cfg_attr(not(unix), allow(dead_code))]
    NoSocket,
    /// Connecting, writing or reading failed (including the deadline).
    #[cfg_attr(not(unix), allow(dead_code))]
    Transport { detail: String },
    /// The server answered, but not with one JSON object.
    #[cfg_attr(not(unix), allow(dead_code))]
    BadResponse { detail: String },
    /// No unix domain sockets on this platform.
    #[cfg_attr(unix, allow(dead_code))]
    UnsupportedPlatform,
}

impl std::fmt::Display for ClientError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ClientError::NoSocket => {
                f.write_str("DRIVERCTL_SOCK is not set (are you running under the launcher?)")
            }
            ClientError::Transport { detail } => f.write_str(detail),
            ClientError::BadResponse { detail } => {
                write!(f, "unreadable reply from the launcher: {detail}")
            }
            ClientError::UnsupportedPlatform => f.write_str(
                "driverctl needs unix domain sockets, which this platform does not provide",
            ),
        }
    }
}

impl std::error::Error for ClientError {}

/// Write to stdout, ignoring failure. `print!` PANICS on a closed pipe, and a
/// panic is rc 101 (see fact 2); a client that cannot print has nothing better
/// to do than exit with the code it already computed.
fn out(text: &str) {
    let mut stdout = std::io::stdout().lock();
    let _ = stdout.write_all(text.as_bytes());
    let _ = stdout.flush();
}

/// Write one line to stderr, ignoring failure, for the reason [`out`] gives.
fn err_line(text: &str) {
    let mut stderr = std::io::stderr().lock();
    let _ = writeln!(stderr, "{text}");
}

/// Run one `driverctl` verb and return the process exit code.
pub fn run(action: DriverctlAction) -> i32 {
    match action {
        DriverctlAction::Log(log) => log_main(&log),
        DriverctlAction::Show => show_main(),
        DriverctlAction::Hook => hook_main(),
        DriverctlAction::StopGuard { run_dir } => stop_guard_main(run_dir.as_deref()),
        DriverctlAction::Serve {
            run_dir,
            project,
            sock,
        } => serve_main(run_dir, project, sock),
    }
}

/// The most of a Stop payload that is read (it carries the last assistant message).
const MAX_STOP_PAYLOAD_BYTES: u64 = 1024 * 1024;

/// `driverctl stop-guard --run-dir <rd>` (D-17): the Stop hook that keeps a
/// driver alive while a dispatched worker is unreported.
///
/// Unlike the other verbs this one reads the run directory and writes under
/// `CLAUDE_CONFIG_DIR` (the module docs of `driverctl::stop_guard` say why that
/// is safe). **It returns 0 on every path**, a panic and an unreadable stdin
/// included: a broken guard must never wedge the driver. It runs before tracing
/// init like the rest of this module (fact 2).
fn stop_guard_main(run_dir: Option<&std::path::Path>) -> i32 {
    use gsd_meta_manager::driverctl::stop_guard::{append_log, guard};

    let mut raw = Vec::new();
    let _ = std::io::stdin()
        .lock()
        .take(MAX_STOP_PAYLOAD_BYTES)
        .read_to_end(&mut raw);
    let stdin = String::from_utf8_lossy(&raw).into_owned();
    let cfg_dir = std::env::var_os("CLAUDE_CONFIG_DIR")
        .filter(|v| !v.is_empty())
        .map(std::path::PathBuf::from);

    let decided = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        guard(&stdin, run_dir, cfg_dir.as_deref(), chrono::Utc::now())
    }));
    if let Ok(outcome) = decided {
        if let Some(cfg_dir) = cfg_dir.as_deref() {
            let _ = append_log(cfg_dir, &outcome.lines);
        }
        if !outcome.stdout.is_empty() {
            out(&format!("{}\n", outcome.stdout));
        }
    }
    0
}

/// `driverctl serve`: run the launcher-side server until a signal (D-03).
#[cfg(any(target_os = "linux", target_os = "macos"))]
fn serve_main(
    run_dir: std::path::PathBuf,
    project: std::path::PathBuf,
    sock: Option<std::path::PathBuf>,
) -> i32 {
    let options = gsd_meta_manager::driverctl::serve::ServeOptions {
        run_dir,
        project,
        sock,
    };
    match gsd_meta_manager::driverctl::serve::serve(options) {
        Ok(()) => 0,
        Err(error) => {
            err_line(&format!("driverctl serve: {error}"));
            error.exit_code()
        }
    }
}

/// `driverctl serve` needs an atomic directory-entry exchange and `flock`: Linux and
/// macOS only; this stub is every other platform.
#[cfg(not(any(target_os = "linux", target_os = "macos")))]
fn serve_main(
    _run_dir: std::path::PathBuf,
    _project: std::path::PathBuf,
    _sock: Option<std::path::PathBuf>,
) -> i32 {
    err_line("driverctl serve: unsupported platform (the run store needs Linux or macOS)");
    1
}

/// `connect(2)` bounded by `limit` (WR-08).
///
/// `UnixStream::connect` has no timeout, and it blocks for good when the listener's
/// backlog is full (a stopped or wedged server). The connect therefore runs on a
/// helper thread and the caller waits with `recv_timeout`; on a timeout the helper
/// is abandoned (the process exits right after, which ends it).
#[cfg(unix)]
fn connect_within(
    path: &std::ffi::OsStr,
    limit: Duration,
) -> Result<std::os::unix::net::UnixStream, ClientError> {
    use std::os::unix::net::UnixStream;

    let transport = |err: std::io::Error| ClientError::Transport {
        detail: format!("connect: {err}"),
    };
    let (tx, rx) = std::sync::mpsc::channel();
    let target = std::path::PathBuf::from(path);
    std::thread::Builder::new()
        .name("driverctl-connect".to_owned())
        .spawn(move || {
            let _ = tx.send(UnixStream::connect(target));
        })
        .map_err(transport)?;
    match rx.recv_timeout(limit) {
        Ok(result) => result.map_err(transport),
        Err(_) => Err(ClientError::Transport {
            detail: format!("connect: no connection within {} s", limit.as_secs().max(1)),
        }),
    }
}

/// Send one request and return the reply object.
#[cfg(unix)]
fn request(req: &Value) -> Result<Value, ClientError> {
    use std::time::Instant;

    let transport = |what: &str, err: std::io::Error| ClientError::Transport {
        detail: format!("{what}: {err}"),
    };

    let path = match std::env::var_os("DRIVERCTL_SOCK") {
        Some(path) if !path.is_empty() => path,
        _ => return Err(ClientError::NoSocket),
    };
    let deadline = Instant::now() + CLIENT_TIMEOUT;

    let mut stream = connect_within(&path, CLIENT_TIMEOUT)?;
    stream
        .set_write_timeout(Some(
            deadline
                .saturating_duration_since(Instant::now())
                .max(Duration::from_millis(1)),
        ))
        .map_err(|e| transport("set write timeout", e))?;

    let mut line = serde_json::to_vec(req).map_err(|e| ClientError::BadResponse {
        detail: format!("could not encode the request: {e}"),
    })?;
    line.push(b'\n');
    stream
        .write_all(&line)
        .map_err(|e| transport("send request", e))?;

    // Read until a newline or EOF, under a total deadline and a size cap.
    let mut buf: Vec<u8> = Vec::new();
    let mut chunk = [0u8; 8192];
    while !buf.ends_with(b"\n") && (buf.len() as u64) < MAX_REPLY_BYTES {
        let remaining = deadline.saturating_duration_since(Instant::now());
        if remaining.is_zero() {
            return Err(ClientError::Transport {
                detail: format!("no reply within {} s", CLIENT_TIMEOUT.as_secs()),
            });
        }
        stream
            .set_read_timeout(Some(remaining))
            .map_err(|e| transport("set read timeout", e))?;
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
                return Err(ClientError::Transport {
                    detail: format!("no reply within {} s", CLIENT_TIMEOUT.as_secs()),
                });
            }
            Err(e) => return Err(transport("read reply", e)),
        }
    }

    if buf.is_empty() {
        return Err(ClientError::BadResponse {
            detail: "the launcher closed the connection without a reply".to_string(),
        });
    }
    match serde_json::from_slice::<Value>(&buf) {
        Ok(reply @ Value::Object(_)) => Ok(reply),
        Ok(_) => Err(ClientError::BadResponse {
            detail: "the reply is not a JSON object".to_string(),
        }),
        Err(e) => Err(ClientError::BadResponse {
            detail: e.to_string(),
        }),
    }
}

/// No unix domain sockets: every request is a typed refusal (D-05's posture).
#[cfg(not(unix))]
fn request(_req: &Value) -> Result<Value, ClientError> {
    Err(ClientError::UnsupportedPlatform)
}

/// Whether a reply carries `"ok": true`.
fn reply_ok(reply: &Value) -> bool {
    reply.get("ok") == Some(&Value::Bool(true))
}

/// The `error` of a reply as text (a string as itself, anything else as JSON).
fn reply_error(reply: &Value) -> String {
    match reply.get("error") {
        Some(Value::String(text)) => text.clone(),
        Some(other) => other.to_string(),
        None => "null".to_string(),
    }
}

/// The `args` object of a `log` request: every `Some` flag, keyed by its
/// underscored field name.
fn log_args(log: &DriverctlLog) -> Map<String, Value> {
    let fields: [(&str, &Option<String>); 17] = [
        ("item", &log.item),
        ("step", &log.step),
        ("round", &log.round),
        ("phase", &log.phase),
        ("status", &log.status),
        ("done_when", &log.done_when),
        ("evidence", &log.evidence),
        ("state", &log.state),
        ("rule", &log.rule),
        ("outcome", &log.outcome),
        ("question", &log.question),
        ("action", &log.action),
        ("reason", &log.reason),
        ("found", &log.found),
        ("mode", &log.mode),
        ("loop_mode", &log.loop_mode),
        ("session", &log.session),
    ];
    fields
        .into_iter()
        .filter_map(|(key, value)| {
            value
                .as_ref()
                .map(|text| (key.to_string(), Value::String(text.clone())))
        })
        .collect()
}

/// `driverctl log <event> [flags]`.
fn log_main(log: &DriverctlLog) -> i32 {
    let mut args = log_args(log);

    if log.event == "report" {
        // The worker's report arrives on stdin, verbatim. A terminal stdin means
        // nobody piped it, and waiting on a keyboard in a headless run is a hang.
        if std::io::stdin().is_terminal() {
            err_line("driverctl: pipe the worker report on stdin");
            return 2;
        }
        let mut raw = Vec::new();
        if let Err(e) = std::io::stdin().read_to_end(&mut raw) {
            err_line(&format!(
                "driverctl: could not read the report on stdin: {e}"
            ));
            return 1;
        }
        args.insert(
            "body".to_string(),
            Value::String(String::from_utf8_lossy(&raw).into_owned()),
        );
    }

    if log.event == "start" && !args.contains_key("session") {
        // The spike's `args.setdefault("session", os.environ.get(...))`: the key
        // is present either way, null when the variable is absent.
        let session = std::env::var("CLAUDE_CODE_SESSION_ID")
            .map(Value::String)
            .unwrap_or(Value::Null);
        args.insert("session".to_string(), session);
    }

    let req = json!({"op": "log", "event": log.event, "args": Value::Object(args)});
    match request(&req) {
        Ok(reply) if reply_ok(&reply) => {
            out("ok\n");
            0
        }
        Ok(reply) => {
            err_line(&format!("driverctl: REJECTED: {}", reply_error(&reply)));
            2
        }
        Err(e) => {
            err_line(&format!("driverctl: {e}"));
            1
        }
    }
}

/// `driverctl show`: print the run's current status block, exactly as the server
/// rendered it (no added newline: the block already ends with its own).
fn show_main() -> i32 {
    match request(&json!({"op": "show"})) {
        Ok(reply) if reply_ok(&reply) => match reply.get("frontmatter").and_then(Value::as_str) {
            Some(block) => {
                out(block);
                0
            }
            None => {
                err_line("driverctl: unreadable reply from the launcher: no frontmatter");
                1
            }
        },
        Ok(reply) => {
            err_line(&format!("driverctl: REJECTED: {}", reply_error(&reply)));
            2
        }
        Err(e) => {
            err_line(&format!("driverctl: {e}"));
            1
        }
    }
}

// ---------------------------------------------------------------------- hook

/// The seven payload keys forwarded to the server, copied as-is (null when
/// absent). Everything else in a Claude hook payload stays in the sandbox.
pub const HOOK_FIELDS: [&str; 7] = [
    "hook_event_name",
    "session_id",
    "agent_id",
    "agent_type",
    "tool_name",
    "source",
    "reason",
];

/// The dispatch prompt is cut to this many CHARACTERS (not bytes) before it is
/// forwarded, so a multi-byte prompt is never split mid-codepoint and the cap
/// means the same thing as the spike's Python `[:4000]`.
pub const HOOK_PROMPT_CHARS: usize = 4000;

/// The deny decision Claude Code reads from a PreToolUse hook's stdout: one line.
fn deny_json(reason: &str) -> String {
    json!({"hookSpecificOutput": {
        "hookEventName": "PreToolUse",
        "permissionDecision": "deny",
        "permissionDecisionReason": format!("LAUNCHER CAP: {reason}"),
    }})
    .to_string()
}

/// The payload forwarded to the server: the seven fields plus a cut prompt.
fn hook_payload(p: &Map<String, Value>) -> Value {
    let mut payload = Map::new();
    for key in HOOK_FIELDS {
        payload.insert(key.to_string(), p.get(key).cloned().unwrap_or(Value::Null));
    }
    let prompt: String = p
        .get("tool_input")
        .and_then(|input| input.get("prompt"))
        .and_then(Value::as_str)
        .unwrap_or("")
        .chars()
        .take(HOOK_PROMPT_CHARS)
        .collect();
    payload.insert("prompt".to_string(), Value::String(prompt));
    Value::Object(payload)
}

/// `driverctl hook`: the PreToolUse / lifecycle hook entry point.
///
/// A port of spike 002's `hook_main` with ONE deviation, and it is the point of
/// this function (D-25, `[inferred — audit]` 27-RESEARCH A8): the spike failed
/// closed only on an `OSError`. Here the dispatch gate (a PreToolUse `Agent` or
/// `Task` call with no `agent_id`, i.e. made by the top-level driver) is denied on
/// EVERY failure to get an affirmative answer: an unreachable server, an unset
/// `DRIVERCTL_SOCK` (the spike exited through `SystemExit`, which is not an
/// `OSError`, so it failed open), and a server reply that is not `ok: true`. Any
/// other hook (nested agents, PostToolUse, SessionStart, ...) stays silent when
/// the server cannot be reached: denying those would brick the driver for a
/// transport blip, and they gate nothing.
///
/// SEC-01 `[inferred — audit]`: stdin that cannot be read, or that is not a JSON
/// object, is denied as well. The gate predicate needs the parsed payload, so with
/// none the only fail-closed answer is the deny JSON (exit code still `0`: the
/// channel Claude Code reads is stdout, and the `onFailure: block` hand-off does not
/// cover an exit-0 hook). Claude Code always sends an object, so this costs nothing
/// in a healthy run.
///
/// The exit code is always `0`: the decision travels as JSON on stdout, which is
/// the channel Claude Code reads a `deny` from.
fn hook_main() -> i32 {
    let mut raw = Vec::new();
    if std::io::stdin().read_to_end(&mut raw).is_err() {
        out(&format!(
            "{}\n",
            deny_json("hook payload unreadable on stdin; launcher-side caps cannot be checked")
        ));
        return 0;
    }
    let Ok(Value::Object(p)) = serde_json::from_slice::<Value>(&raw) else {
        out(&format!(
            "{}\n",
            deny_json("hook payload is not a JSON object; launcher-side caps cannot be checked")
        ));
        return 0;
    };

    let is_gate = p.get("hook_event_name").and_then(Value::as_str) == Some("PreToolUse")
        && matches!(
            p.get("tool_name").and_then(Value::as_str),
            Some("Agent" | "Task")
        )
        && p.get("agent_id").is_none_or(Value::is_null);

    let reply = match request(&json!({"op": "hook", "payload": hook_payload(&p)})) {
        Ok(reply) => reply,
        Err(e) => {
            if is_gate {
                out(&format!(
                    "{}\n",
                    deny_json(&format!(
                        "driverctl unreachable ({e}); launcher-side caps cannot be checked"
                    ))
                ));
            }
            return 0;
        }
    };

    match reply.get("deny").and_then(Value::as_str) {
        Some(reason) if !reason.is_empty() => {
            out(&format!("{}\n", deny_json(reason)));
        }
        _ if is_gate && !reply_ok(&reply) => {
            out(&format!(
                "{}\n",
                deny_json(&format!(
                    "driverctl refused the gate request ({}); launcher-side caps cannot be checked",
                    reply_error(&reply)
                ))
            ));
        }
        _ => {}
    }
    0
}

#[cfg(all(test, unix))]
mod tests {
    use super::*;
    use std::os::unix::net::UnixListener;

    #[test]
    fn wr08_connect_to_a_full_backlog_times_out_instead_of_blocking() {
        let td = tempfile::TempDir::new().unwrap();
        let sock = td.path().join("full.sock");
        // Bound, listening, never accepting: the backlog fills and a further
        // blocking connect(2) would wait forever.
        let _listener = UnixListener::bind(&sock).unwrap();
        let mut held = Vec::new();
        let mut timed_out = false;
        for _ in 0..10_000 {
            match connect_within(sock.as_os_str(), Duration::from_millis(300)) {
                Ok(stream) => held.push(stream),
                Err(ClientError::Transport { detail }) => {
                    assert!(detail.contains("no connection within"), "{detail}");
                    timed_out = true;
                    break;
                }
                Err(other) => panic!("unexpected error: {other}"),
            }
        }
        assert!(
            timed_out,
            "the backlog never filled after {} connects",
            held.len()
        );
    }

    #[test]
    fn wr08_connect_to_a_live_listener_still_succeeds_and_a_dead_path_errors() {
        let td = tempfile::TempDir::new().unwrap();
        let sock = td.path().join("ok.sock");
        let _listener = UnixListener::bind(&sock).unwrap();
        assert!(connect_within(sock.as_os_str(), Duration::from_secs(30)).is_ok());
        let missing = td.path().join("none.sock");
        let err = connect_within(missing.as_os_str(), Duration::from_secs(30)).unwrap_err();
        assert!(err.to_string().starts_with("connect: "), "{err}");
    }
}
