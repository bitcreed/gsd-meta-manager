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
//! This file spawns no process, so it is deliberately not on the spawn-seam
//! allowlist.

use super::{Detection, EnvSnapshot, ExecArgv, MultiplexerBackend, MuxError};
use std::path::Path;

/// The herdr backend.
pub struct HerdrBackend;

/// The registry entry for herdr.
pub static HERDR: HerdrBackend = HerdrBackend;

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
/// wrong. The error key is checked first, so an ambiguous reply fails closed.
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
    if let Some(error) = object.get("error") {
        let field = |name: &str| {
            error
                .get(name)
                .and_then(|v| v.as_str())
                .unwrap_or("unknown")
                .to_string()
        };
        return Err(MuxError::Failed(format!(
            "herdr layout.apply failed ({}): {}",
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
                "herdr layout.apply failed (layout_apply_failed): Unable to spawn env".to_string()
            )
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
            let (stream, _) = listener.accept().unwrap();
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
}
