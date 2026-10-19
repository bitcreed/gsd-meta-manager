#![cfg(unix)]
// ============================================================================
// The sandbox-side `driverctl` client, through the REAL binary (Phase 27, plan
// 27-04; DCTL-01 client half, D-01, D-02, D-04, D-25).
//
// Every test spawns `gsd-meta-manager driverctl ...` as a child process against a
// test-local recording server, and every spawn carries
// `XDG_DATA_HOME=/proc/nonexistent-gsdmm`. That variable is the regression guard
// for T-27-11: the data dir is unwritable, so a client that initialised tracing
// first would panic (rc 101) and, as a PreToolUse hook, fail OPEN. The binary is
// the unit under test on purpose; an in-process call to the client would not
// exercise `main`'s dispatch order, which is the property that matters.
//
// The recording server answers each connection with the next scripted reply line
// (the last one repeats) and records the request line it received, so a test can
// compare the exact wire request the spike client would have sent.
// ============================================================================

use std::io::{BufRead, BufReader, Write};
use std::os::unix::net::UnixListener;
use std::path::{Path, PathBuf};
use std::process::{Command, Output, Stdio};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use serde_json::{json, Value};
use tempfile::TempDir;

/// The unwritable data dir every spawn runs under (D-25, T-27-11).
const UNWRITABLE_DATA_HOME: &str = "/proc/nonexistent-gsdmm";

/// A unix-socket server that records requests and plays scripted replies.
struct Recorder {
    _dir: TempDir,
    sock: PathBuf,
    requests: Arc<Mutex<Vec<String>>>,
    stop: Arc<AtomicBool>,
    handle: Option<JoinHandle<()>>,
}

impl Recorder {
    /// Serve `replies` in order (the last repeats). Each is one JSON line.
    fn start(replies: &[&str]) -> Recorder {
        let dir = TempDir::new().expect("temp dir for the socket");
        let sock = dir.path().join("driverctl.sock");
        let listener = UnixListener::bind(&sock).expect("bind the recording socket");
        listener.set_nonblocking(true).expect("non-blocking accept");
        let requests = Arc::new(Mutex::new(Vec::<String>::new()));
        let stop = Arc::new(AtomicBool::new(false));
        let replies: Vec<String> = replies.iter().map(|r| r.to_string()).collect();

        let handle = {
            let requests = Arc::clone(&requests);
            let stop = Arc::clone(&stop);
            std::thread::spawn(move || {
                let mut served = 0usize;
                while !stop.load(Ordering::SeqCst) {
                    match listener.accept() {
                        Ok((stream, _)) => {
                            stream.set_nonblocking(false).expect("blocking connection");
                            stream
                                .set_read_timeout(Some(Duration::from_secs(5)))
                                .expect("read timeout");
                            let mut reader = BufReader::new(stream);
                            let mut line = String::new();
                            if reader.read_line(&mut line).is_ok() && !line.is_empty() {
                                requests.lock().expect("requests").push(line);
                                let reply = replies
                                    .get(served)
                                    .or_else(|| replies.last())
                                    .cloned()
                                    .unwrap_or_else(|| r#"{"ok": true}"#.to_string());
                                served += 1;
                                let _ = reader.get_mut().write_all(format!("{reply}\n").as_bytes());
                            }
                        }
                        Err(e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                            std::thread::sleep(Duration::from_millis(5));
                        }
                        Err(_) => break,
                    }
                }
            })
        };
        Recorder {
            _dir: dir,
            sock,
            requests,
            stop,
            handle: Some(handle),
        }
    }

    /// Every request line received so far, parsed.
    fn requests(&self) -> Vec<Value> {
        self.requests
            .lock()
            .expect("requests")
            .iter()
            .map(|line| serde_json::from_str(line).expect("the client sent one JSON object"))
            .collect()
    }

    /// The raw request lines, for newline-framing assertions.
    fn raw_requests(&self) -> Vec<String> {
        self.requests.lock().expect("requests").clone()
    }
}

impl Drop for Recorder {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::SeqCst);
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }
}

/// `gsd-meta-manager driverctl ...` with the hostile environment of the sandbox.
fn driverctl(sock: Option<&Path>) -> Command {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_gsd-meta-manager"));
    cmd.env("XDG_DATA_HOME", UNWRITABLE_DATA_HOME)
        // The executor's own Claude session sets this; a test that reads it would
        // assert against the wrong session.
        .env_remove("CLAUDE_CODE_SESSION_ID")
        .env_remove("DRIVERCTL_SOCK");
    if let Some(sock) = sock {
        cmd.env("DRIVERCTL_SOCK", sock);
    }
    cmd.arg("driverctl");
    cmd
}

/// Run `cmd` to completion with `stdin` (empty when `None`) and capture it.
fn run(cmd: &mut Command, stdin: Option<&str>) -> Output {
    let mut child = cmd
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .expect("the binary is built and spawnable");
    {
        let mut pipe = child.stdin.take().expect("piped stdin");
        if let Some(text) = stdin {
            pipe.write_all(text.as_bytes()).expect("write stdin");
        }
    }
    child.wait_with_output().expect("wait for the client")
}

fn stdout(out: &Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

/// A socket path that nothing listens on.
fn dead_socket() -> (TempDir, PathBuf) {
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join("nobody-home.sock");
    (dir, path)
}

#[test]
fn log_start_sends_the_spike_wire_request_and_prints_ok() {
    let server = Recorder::start(&[r#"{"ok": true}"#]);
    let out = run(
        driverctl(Some(&server.sock)).args([
            "log",
            "start",
            "--mode",
            "headless",
            "--loop-mode",
            "loop",
        ]),
        None,
    );

    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    assert_eq!(stdout(&out), "ok\n");
    assert!(
        !stderr(&out).contains("panicked"),
        "the client must not panic under an unwritable data dir: {}",
        stderr(&out)
    );

    let requests = server.requests();
    assert_eq!(requests.len(), 1, "exactly one request per connection");
    assert_eq!(
        requests[0],
        json!({"op": "log", "event": "start",
               "args": {"mode": "headless", "loop_mode": "loop", "session": null}})
    );
    let raw = server.raw_requests();
    assert!(raw[0].ends_with('\n'), "the request is framed by a newline");
    assert_eq!(
        raw[0].matches('\n').count(),
        1,
        "one line, no embedded newline"
    );
}

#[test]
fn rejected_reply_exits_2_with_the_spike_message() {
    let server = Recorder::start(&[r#"{"ok": false, "error": "bad rule: nope"}"#]);
    let out = run(
        driverctl(Some(&server.sock))
            .args(["log", "status", "--item", "I-01", "--state", "parked"]),
        None,
    );

    assert_eq!(out.status.code(), Some(2));
    assert_eq!(stdout(&out), "", "a rejection prints nothing on stdout");
    assert_eq!(stderr(&out), "driverctl: REJECTED: bad rule: nope\n");
}

#[test]
fn transport_error_exits_1_not_101() {
    let (_dir, dead) = dead_socket();
    let out = run(
        driverctl(Some(&dead)).args(["log", "end", "--reason", "stop"]),
        None,
    );

    assert_eq!(out.status.code(), Some(1), "stderr: {}", stderr(&out));
    assert!(stderr(&out).starts_with("driverctl: "), "{}", stderr(&out));
    assert!(!stderr(&out).contains("panicked"), "{}", stderr(&out));
}

#[test]
fn unset_socket_env_exits_1() {
    let out = run(
        driverctl(None).args(["log", "end", "--reason", "stop"]),
        None,
    );

    assert_eq!(out.status.code(), Some(1));
    assert!(
        stderr(&out).contains("DRIVERCTL_SOCK is not set"),
        "{}",
        stderr(&out)
    );
}

#[test]
fn usage_errors_exit_2() {
    let server = Recorder::start(&[r#"{"ok": true}"#]);
    let out = run(
        driverctl(Some(&server.sock)).args(["log", "start", "--bogus", "x"]),
        None,
    );

    assert_eq!(
        out.status.code(),
        Some(2),
        "argparse parity for usage errors"
    );
    assert!(server.requests().is_empty(), "a usage error sends nothing");
}

#[test]
fn report_body_comes_from_stdin() {
    let server = Recorder::start(&[r#"{"ok": true}"#]);
    let body = "STATUS: done\nITEM: I-01\n";
    let out = run(
        driverctl(Some(&server.sock)).args(["log", "report", "--item", "I-01", "--status", "done"]),
        Some(body),
    );

    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    assert_eq!(stdout(&out), "ok\n");
    assert_eq!(
        server.requests()[0],
        json!({"op": "log", "event": "report",
               "args": {"item": "I-01", "status": "done", "body": body}})
    );
}

#[test]
fn start_session_defaults_from_the_claude_env() {
    let server = Recorder::start(&[r#"{"ok": true}"#]);
    let out = run(
        driverctl(Some(&server.sock))
            .env("CLAUDE_CODE_SESSION_ID", "sess-from-env")
            .args(["log", "start", "--mode", "headless"]),
        None,
    );
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    assert_eq!(
        server.requests()[0]["args"]["session"],
        json!("sess-from-env")
    );

    // An explicit flag wins over the environment.
    let out = run(
        driverctl(Some(&server.sock))
            .env("CLAUDE_CODE_SESSION_ID", "sess-from-env")
            .args(["log", "start", "--session", "sess-flag"]),
        None,
    );
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    assert_eq!(server.requests()[1]["args"]["session"], json!("sess-flag"));

    // The default applies to `start` only.
    let out = run(
        driverctl(Some(&server.sock))
            .env("CLAUDE_CODE_SESSION_ID", "sess-from-env")
            .args(["log", "end", "--reason", "stop"]),
        None,
    );
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(&out));
    assert!(
        server.requests()[2]["args"].get("session").is_none(),
        "only `start` defaults the session"
    );
}

// ---------------------------------------------------------------------------
// The dispatch gate (`driverctl hook`), D-25 / T-27-11 / T-27-12.
//
// Each test states which failure it forces and which answer it demands. The
// property under test is that NO failure of the transport or of the server opens
// the gate: a PreToolUse Agent|Task call with no agent_id is answered `deny`
// unless the server affirmatively said `ok`.
// ---------------------------------------------------------------------------

/// A PreToolUse Agent payload from the top-level driver (no `agent_id`).
fn gate_payload() -> String {
    json!({
        "hook_event_name": "PreToolUse",
        "session_id": "sess-1",
        "tool_name": "Agent",
        "tool_input": {"prompt": "run the worker", "subagent_type": "general-purpose"},
    })
    .to_string()
}

/// Parse a hook's stdout as exactly one deny decision and return its reason.
fn deny_reason(out: &Output) -> String {
    assert_eq!(
        out.status.code(),
        Some(0),
        "a hook always exits 0: {}",
        stderr(out)
    );
    let text = stdout(out);
    assert_eq!(
        text.lines().count(),
        1,
        "one JSON line expected, got {text:?}"
    );
    let decision: Value = serde_json::from_str(text.trim()).expect("stdout is JSON");
    let hso = &decision["hookSpecificOutput"];
    assert_eq!(hso["hookEventName"], json!("PreToolUse"));
    assert_eq!(hso["permissionDecision"], json!("deny"), "{text}");
    hso["permissionDecisionReason"]
        .as_str()
        .expect("a reason string")
        .to_string()
}

fn assert_silent(out: &Output) {
    assert_eq!(out.status.code(), Some(0), "stderr: {}", stderr(out));
    assert_eq!(stdout(out), "", "a silent hook prints nothing");
}

#[test]
fn gate_fails_closed_when_unreachable() {
    let (_dir, dead) = dead_socket();
    let out = run(driverctl(Some(&dead)).arg("hook"), Some(&gate_payload()));

    let reason = deny_reason(&out);
    assert!(
        reason.starts_with("LAUNCHER CAP: driverctl unreachable"),
        "{reason}"
    );
    assert!(!stderr(&out).contains("panicked"), "{}", stderr(&out));
}

#[test]
fn gate_fails_closed_without_socket_env() {
    let out = run(driverctl(None).arg("hook"), Some(&gate_payload()));

    let reason = deny_reason(&out);
    assert!(
        reason.starts_with("LAUNCHER CAP: driverctl unreachable"),
        "{reason}"
    );
    assert!(reason.contains("DRIVERCTL_SOCK"), "{reason}");
}

#[test]
fn gate_fails_closed_on_an_unreadable_reply() {
    // Not a JSON object: the server is broken, which is not an affirmative answer.
    let server = Recorder::start(&["this is not json"]);
    let out = run(
        driverctl(Some(&server.sock)).arg("hook"),
        Some(&gate_payload()),
    );

    let reason = deny_reason(&out);
    assert!(
        reason.starts_with("LAUNCHER CAP: driverctl unreachable"),
        "{reason}"
    );
}

#[test]
fn server_deny_is_relayed() {
    let server = Recorder::start(&[r#"{"ok": true, "deny": "dispatch cap 10 reached"}"#]);
    let out = run(
        driverctl(Some(&server.sock)).arg("hook"),
        Some(&gate_payload()),
    );

    assert_eq!(deny_reason(&out), "LAUNCHER CAP: dispatch cap 10 reached");
}

#[test]
fn server_error_on_gate_denies() {
    let server = Recorder::start(&[r#"{"ok": false, "error": "internal"}"#]);
    let out = run(
        driverctl(Some(&server.sock)).arg("hook"),
        Some(&gate_payload()),
    );

    let reason = deny_reason(&out);
    assert!(
        reason.starts_with("LAUNCHER CAP: driverctl refused the gate request (internal)"),
        "{reason}"
    );
}

#[test]
fn server_error_on_non_gate_is_silent() {
    let server = Recorder::start(&[r#"{"ok": false, "error": "internal"}"#]);
    let post = json!({"hook_event_name": "PostToolUse", "tool_name": "Agent"}).to_string();
    let out = run(driverctl(Some(&server.sock)).arg("hook"), Some(&post));

    assert_silent(&out);
}

#[test]
fn server_ok_without_deny_lets_the_gate_through() {
    let server = Recorder::start(&[r#"{"ok": true}"#]);
    let out = run(
        driverctl(Some(&server.sock)).arg("hook"),
        Some(&gate_payload()),
    );

    assert_silent(&out);
}

#[test]
fn nested_and_non_gate_hooks_are_silent_when_unreachable() {
    let (_dir, dead) = dead_socket();
    let payloads = [
        // A nested agent's dispatch: the gate covers the top-level driver only.
        json!({"hook_event_name": "PreToolUse", "tool_name": "Agent", "agent_id": "a-1"}),
        json!({"hook_event_name": "PreToolUse", "tool_name": "Task", "agent_id": "a-1"}),
        json!({"hook_event_name": "PostToolUse", "tool_name": "Agent"}),
        json!({"hook_event_name": "PreToolUse", "tool_name": "Bash"}),
        json!({"hook_event_name": "SessionStart", "source": "startup"}),
    ];
    for payload in payloads {
        let out = run(
            driverctl(Some(&dead)).arg("hook"),
            Some(&payload.to_string()),
        );
        assert_silent(&out);
    }

    // `agent_id: null` is still the top-level driver, so it IS the gate.
    let null_id = json!({"hook_event_name": "PreToolUse", "tool_name": "Task", "agent_id": null});
    let out = run(
        driverctl(Some(&dead)).arg("hook"),
        Some(&null_id.to_string()),
    );
    assert!(deny_reason(&out).starts_with("LAUNCHER CAP: "));
}

#[test]
fn invalid_hook_stdin_is_silent() {
    let server = Recorder::start(&[r#"{"ok": true}"#]);
    for stdin in ["not json at all", "", "[1, 2, 3]", "\"a string\"", "42"] {
        let out = run(driverctl(Some(&server.sock)).arg("hook"), Some(stdin));
        assert_silent(&out);
    }
    assert!(
        server.requests().is_empty(),
        "unparsable input sends nothing"
    );
}

#[test]
fn hook_forwards_only_the_seven_fields_and_4000_chars() {
    let server = Recorder::start(&[r#"{"ok": true}"#]);
    // 5000 CHARACTERS of a 3-byte codepoint: a byte cut at 4000 would split one.
    let prompt: String = "日".repeat(5000);
    let payload = json!({
        "hook_event_name": "PreToolUse",
        "session_id": "sess-1",
        "agent_type": "general-purpose",
        "tool_name": "Agent",
        "source": "startup",
        "reason": "because",
        "cwd": "/should/not/be/forwarded",
        "transcript_path": "/neither/this",
        "tool_input": {"prompt": prompt, "description": "nor this"},
    });
    let out = run(
        driverctl(Some(&server.sock)).arg("hook"),
        Some(&payload.to_string()),
    );
    assert_silent(&out);

    let requests = server.requests();
    assert_eq!(requests.len(), 1);
    assert_eq!(requests[0]["op"], json!("hook"));
    let forwarded = requests[0]["payload"]
        .as_object()
        .expect("a payload object");
    let mut keys: Vec<&str> = forwarded.keys().map(String::as_str).collect();
    keys.sort_unstable();
    assert_eq!(
        keys,
        [
            "agent_id",
            "agent_type",
            "hook_event_name",
            "prompt",
            "reason",
            "session_id",
            "source",
            "tool_name"
        ]
    );
    assert_eq!(
        forwarded["agent_id"],
        Value::Null,
        "an absent key is sent as null"
    );
    assert_eq!(forwarded["session_id"], json!("sess-1"));
    let sent = forwarded["prompt"].as_str().expect("a prompt string");
    assert_eq!(
        sent.chars().count(),
        4000,
        "cut to 4000 characters, not bytes"
    );
    assert_eq!(sent, "日".repeat(4000));
}

#[test]
fn fallible_tracing_survives_unwritable_data_dir() {
    let dir = TempDir::new().expect("temp dir");
    let config = dir.path().join("config.json");
    gsd_meta_manager::config::save_config(&gsd_meta_manager::config::Config::default(), &config)
        .expect("write an empty registry");

    let out = run(
        Command::new(env!("CARGO_BIN_EXE_gsd-meta-manager"))
            .env("XDG_DATA_HOME", UNWRITABLE_DATA_HOME)
            .arg("--config")
            .arg(&config)
            .arg("list"),
        None,
    );

    assert_eq!(
        out.status.code(),
        Some(0),
        "an unwritable data dir must cost the log file, not the command. stderr: {}",
        stderr(&out)
    );
    assert!(!stderr(&out).contains("panicked"), "{}", stderr(&out));
    assert!(
        stdout(&out).contains("No projects registered."),
        "{}",
        stdout(&out)
    );
}

// ---------------------------------------------------------------------------
// SC1: the spike-002 MASTER-PROMPT, unchanged, against the Rust CLI.
//
// The fixture is a byte copy of the prompt the driver will actually be given. The
// test lifts every `driverctl` line out of its "Recording events" block, fills the
// placeholders, and runs each one through the real binary. A flag the Rust CLI
// does not know, a value shape it rejects, or a wire request that differs from
// what the spike client would send fails here, before Phase 29 puts the prompt in
// front of a model.
// ---------------------------------------------------------------------------

const MASTER_PROMPT: &str = include_str!("fixtures/driverctl/MASTER-PROMPT.md");

/// The fenced block under `## Recording events`.
fn recording_block() -> Vec<&'static str> {
    let mut lines = MASTER_PROMPT.lines();
    assert!(
        lines
            .by_ref()
            .any(|l| l == "## Recording events: `driverctl` (the only way)"),
        "the Recording events heading moved; update this lifter"
    );
    let mut in_fence = false;
    let mut block = Vec::new();
    for line in lines {
        if line.starts_with("```") {
            if in_fence {
                break;
            }
            in_fence = true;
        } else if in_fence {
            block.push(line);
        }
    }
    assert!(!block.is_empty(), "no fenced block under Recording events");
    block
}

/// Drop a trailing `# comment` that sits outside double quotes.
fn strip_comment(line: &str) -> &str {
    let mut in_quote = false;
    let mut prev_space = true;
    for (i, c) in line.char_indices() {
        match c {
            '"' => in_quote = !in_quote,
            '#' if !in_quote && prev_space => return line[..i].trim_end(),
            _ => {}
        }
        prev_space = c.is_whitespace();
    }
    line.trim_end()
}

/// Replace every `<a|b|...>` with its first alternative.
fn first_alternatives(line: &str) -> String {
    let mut out = String::new();
    let mut rest = line;
    while let Some(open) = rest.find('<') {
        let Some(close) = rest[open..].find('>') else {
            break;
        };
        let inner = &rest[open + 1..open + close];
        out.push_str(&rest[..open]);
        if inner.contains('|') {
            out.push_str(inner.split('|').next().unwrap_or(""));
        } else {
            out.push_str(&rest[open..=open + close]);
        }
        rest = &rest[open + close + 1..];
    }
    out.push_str(rest);
    out
}

/// A double-quote-aware word splitter (no crate): quotes group, and are removed.
fn split_words(line: &str) -> Vec<String> {
    let mut words = Vec::new();
    let mut cur = String::new();
    let mut in_word = false;
    let mut in_quote = false;
    for c in line.chars() {
        match c {
            '"' => {
                in_quote = !in_quote;
                in_word = true;
            }
            c if c.is_whitespace() && !in_quote => {
                if in_word {
                    words.push(std::mem::take(&mut cur));
                    in_word = false;
                }
            }
            c => {
                cur.push(c);
                in_word = true;
            }
        }
    }
    if in_word {
        words.push(cur);
    }
    words
}

/// Every runnable form of one prompt line, placeholders filled.
///
/// `[--rule <rule>]` is optional in the prompt, so such a line yields two forms
/// (without and with `--rule budget`): both are things the driver may type.
fn instantiate(line: &str) -> Vec<String> {
    let line = strip_comment(line);
    let line = line.strip_suffix("<<'EOF'").unwrap_or(line).trim_end();
    let prose = line
        .replace("\"<one GSD command>\"", "\"/gsd-plan-phase 1\"")
        .replace("\"<short, one line>\"", "\"tests green\"")
        .replace("\"<one line>\"", "\"one line of text\"");
    let forms = if prose.contains("[--rule <rule>]") {
        vec![
            prose.replace("[--rule <rule>]", ""),
            prose.replace("[--rule <rule>]", "--rule budget"),
        ]
    } else {
        vec![prose]
    };
    forms
        .into_iter()
        .map(|form| {
            first_alternatives(&form)
                .replace("<n>", "1")
                .replace("<N>", "1")
                .replace("{MODE}", "loop")
        })
        .collect()
}

#[test]
fn master_prompt_lines_parse_and_build_spike_requests() {
    let report_stdin = "STATUS: done\nITEM: I-01\n";
    let server = Recorder::start(&[r#"{"ok": true, "frontmatter": "---\nrun: demo\n---\n"}"#]);
    let mut exercised = 0usize;

    for line in recording_block() {
        if !line.starts_with("driverctl ") {
            continue;
        }
        for form in instantiate(line) {
            assert!(
                !form.contains(['<', '>', '{', '}', '[', ']']),
                "unfilled placeholder in {form:?}"
            );
            let words = split_words(&form);
            assert_eq!(words[0], "driverctl", "{form:?}");
            let argv = &words[1..];

            let is_report = argv.get(1).map(String::as_str) == Some("report");
            let before = server.requests().len();
            let out = run(
                driverctl(Some(&server.sock)).args(argv),
                is_report.then_some(report_stdin),
            );
            assert_eq!(
                out.status.code(),
                Some(0),
                "the Rust CLI rejected the unchanged prompt line {form:?}: {}",
                stderr(&out)
            );

            let requests = server.requests();
            assert_eq!(requests.len(), before + 1, "{form:?} sends one request");
            let got = &requests[before];

            let expected = if argv[0] == "show" {
                assert_eq!(
                    stdout(&out),
                    "---\nrun: demo\n---\n",
                    "show prints the block"
                );
                json!({"op": "show"})
            } else {
                assert_eq!(stdout(&out), "ok\n", "{form:?}");
                assert_eq!(argv[0], "log", "{form:?}");
                let mut args = serde_json::Map::new();
                for pair in argv[2..].chunks(2) {
                    let key = pair[0]
                        .strip_prefix("--")
                        .expect("a --flag")
                        .replace('-', "_");
                    args.insert(key, json!(pair[1]));
                }
                if argv[1] == "start" {
                    args.entry("session").or_insert(Value::Null);
                }
                if is_report {
                    args.insert("body".to_string(), json!(report_stdin));
                }
                json!({"op": "log", "event": argv[1], "args": Value::Object(args)})
            };
            assert_eq!(got, &expected, "wire request for {form:?}");
            exercised += 1;
        }
    }

    assert!(
        exercised >= 11,
        "only {exercised} prompt lines were exercised; the lifter is not finding the block"
    );
}
