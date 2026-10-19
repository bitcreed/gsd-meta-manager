//! The Stop-hook keep-alive guard: `driverctl stop-guard` (Phase 28, 28-06; D-17,
//! LNCH-03). A port of the spike-003 `stop_guard.py`.
//!
//! Phase 26 Run 1b: the driver dispatched one worker, the worker started its own
//! background agent and ended its turn, and the driver ("the worker is running,
//! I'll wait") ended ITS turn too. `claude -p` then exited 0 with the worker's
//! dispatch still unreported. This is the second `Stop` hook entry the launcher
//! seeds (after the driverctl heartbeat). It blocks a premature end of the
//! top-level driver's turn only.
//!
//! # Numbered facts
//!
//! 1. **Decision order, first match wins** (D-17). Unparseable stdin or no
//!    `--run-dir`: `allow reason=bad-input`. A non-`Stop` event, or an `agent_id`
//!    present (a subagent): `not-top-level`. `background_tasks` non-empty:
//!    `background`. A `RUN | end` in `<run dir>/LOG.md`: `run-end`. No outstanding
//!    dispatch: `no-outstanding`. `CLAUDE_CONFIG_DIR` unset: `no-config-dir`.
//!    Otherwise bump the per-dispatch counter: attempt <= [`GUARD_LIMIT`] blocks,
//!    else `allow reason=limit`.
//! 2. **"Outstanding" is the exit classifier's definition**, not a copy:
//!    [`outstanding_dispatch`] is the function `classify_exit` uses, so the guard
//!    and the verdict can never disagree about whether a worker is unreported.
//! 3. **The budget is [`GUARD_LIMIT`] blocks per outstanding dispatch, not per
//!    run** (spike WR-02). `stop-guard.count` holds `{"key":"<item>@<event
//!    index>","n":<blocks so far>}`; a different outstanding dispatch resets the
//!    count to 1 and logs `reset from=<old key or -> to=<new key>`; when nothing is
//!    outstanding any more the counter is cleared and `reset from=<old> to=-` is
//!    logged before the allow line. CONTEXT D-17's parenthetical "at most 3 blocks
//!    per run" is read as describing this guard `[inferred — audit]`. Claude Code
//!    itself overrides a block after eight consecutive hook continuations; the
//!    limit stays well below it.
//! 4. **The counter is model-writable and is not trusted** (IN-01, T-28-16). It
//!    lives in `CLAUDE_CONFIG_DIR`, which the driver can write. A file that is
//!    unreadable, not JSON, a bare integer, or has a missing or non-string `key`
//!    or a negative, fractional, boolean or string `n` reads as `("", 0)`. A
//!    forged counter can therefore only restart the count or shorten it, never
//!    extend the budget; the wall-clock cap (28-05) is the backstop. A file larger
//!    than [`MAX_COUNT_BYTES`] is not read at all.
//! 5. **It only reads the run directory** (read-only inside the sandbox) and
//!    writes only under `CLAUDE_CONFIG_DIR`: the counter, atomically, and the
//!    append-only `stop-guard.log`.
//! 6. **A broken guard must never wedge the driver.** The process exits 0 on every
//!    path (`driverctl_cli`). A counter that cannot be written fails open as
//!    `allow reason=counter-unwritable` (the spike silently printed nothing):
//!    blocking without a persisted count would have no budget `[inferred — audit]`.
//!
//! This module is ungated: it needs no `flock` and no store.

use std::fs;
use std::io::{self, Read, Write};
use std::path::Path;

use chrono::{DateTime, Utc};
use serde_json::{json, Value};

use super::counters::outstanding_dispatch;
use super::log::{parse_log, utc_string};

/// Blocks allowed per outstanding dispatch (the spike's value).
pub const GUARD_LIMIT: u32 = 3;

/// The counter file under `CLAUDE_CONFIG_DIR`.
pub const COUNT_FILE: &str = "stop-guard.count";

/// The append-only decision log under `CLAUDE_CONFIG_DIR`.
pub const LOG_FILE: &str = "stop-guard.log";

/// The most of `stop-guard.count` that is read (a real one is under 100 bytes).
pub const MAX_COUNT_BYTES: u64 = 4096;

/// The most of `LOG.md` that is read.
const MAX_LOG_BYTES: u64 = 16 * 1024 * 1024;

/// What one Stop hook call decided.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct GuardOutcome {
    /// What the hook prints: empty to allow, `{"decision":"block",...}` to block.
    pub stdout: String,
    /// The lines for `stop-guard.log`, each timestamped, `reset` lines first.
    pub lines: Vec<String>,
}

/// The reason Claude is shown when a premature end is blocked (spike text, verbatim).
fn block_reason(item: &str) -> String {
    format!(
        "The dispatch for {item} has no report in LOG.md and no agent is running, so the worker \
         ended early. Do NOT dispatch a new worker: that would be a duplicate. Use SendMessage to \
         the SAME worker agentId from its 'Async agent launched' result and ask it to finish the \
         step, commit, and reply with its STATUS report. If it cannot, follow Startup step 3 \
         (recovered)."
    )
}

/// Python truthiness of a JSON value (`if p.get("agent_id")`).
fn truthy(value: Option<&Value>) -> bool {
    match value {
        None | Some(Value::Null) | Some(Value::Bool(false)) => false,
        Some(Value::String(s)) => !s.is_empty(),
        Some(Value::Number(n)) => n.as_f64().is_some_and(|f| f != 0.0),
        Some(Value::Array(a)) => !a.is_empty(),
        Some(Value::Object(o)) => !o.is_empty(),
        Some(Value::Bool(true)) => true,
    }
}

/// `(key, n)` from the counter file: anything that is not exactly
/// `{"key": <string>, "n": <non-negative integer>}` reads as `("", 0)` (IN-01).
pub fn read_count(path: &Path) -> (String, u64) {
    let forged = (String::new(), 0);
    let Ok(file) = fs::File::open(path) else {
        return forged;
    };
    let mut bytes = Vec::new();
    if file
        .take(MAX_COUNT_BYTES + 1)
        .read_to_end(&mut bytes)
        .is_err()
        || bytes.len() as u64 > MAX_COUNT_BYTES
    {
        return forged;
    }
    let Ok(Value::Object(map)) = serde_json::from_slice::<Value>(&bytes) else {
        return forged;
    };
    match (map.get("key"), map.get("n")) {
        (Some(Value::String(key)), Some(Value::Number(n))) => match n.as_u64() {
            Some(n) => (key.clone(), n),
            None => forged,
        },
        _ => forged,
    }
}

/// Write the counter through a temp file and a rename.
fn write_count(path: &Path, key: &str, n: u64) -> io::Result<()> {
    let dir = path.parent().unwrap_or_else(|| Path::new("."));
    let tmp = dir.join(format!(".{COUNT_FILE}.{}.tmp", std::process::id()));
    let _ = fs::remove_file(&tmp);
    let written = (|| {
        let mut file = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)?;
        file.write_all(json!({"key": key, "n": n}).to_string().as_bytes())?;
        file.write_all(b"\n")?;
        fs::rename(&tmp, path)
    })();
    if written.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    written
}

/// `<run dir>/LOG.md` as text; absent or unreadable is empty.
fn read_log(run_dir: &Path) -> String {
    let Ok(file) = fs::File::open(run_dir.join("LOG.md")) else {
        return String::new();
    };
    let mut bytes = Vec::new();
    let _ = file.take(MAX_LOG_BYTES).read_to_end(&mut bytes);
    String::from_utf8_lossy(&bytes).into_owned()
}

/// Decide one Stop hook call. See the module docs for the order.
///
/// Reads `<run_dir>/LOG.md` and, when `cfg_dir` is given, reads and writes the
/// counter there. The caller prints [`GuardOutcome::stdout`] and appends
/// [`GuardOutcome::lines`] to `stop-guard.log` ([`append_log`]).
pub fn guard(
    stdin: &str,
    run_dir: Option<&Path>,
    cfg_dir: Option<&Path>,
    now: DateTime<Utc>,
) -> GuardOutcome {
    let mut pre: Vec<String> = Vec::new();
    let mut stdout = String::new();
    let decision = decide(stdin, run_dir, cfg_dir, &mut pre, &mut stdout);
    let stamp = utc_string(now);
    let mut lines: Vec<String> = pre.into_iter().map(|r| format!("{stamp} {r}")).collect();
    lines.push(format!("{stamp} {decision}"));
    GuardOutcome { stdout, lines }
}

fn decide(
    stdin: &str,
    run_dir: Option<&Path>,
    cfg_dir: Option<&Path>,
    pre: &mut Vec<String>,
    stdout: &mut String,
) -> String {
    let payload = match serde_json::from_str::<Value>(stdin) {
        Ok(Value::Object(map)) => map,
        _ => return "allow reason=bad-input".to_owned(),
    };
    let Some(run_dir) = run_dir.filter(|d| !d.as_os_str().is_empty()) else {
        return "allow reason=bad-input".to_owned();
    };
    if payload.get("hook_event_name").and_then(Value::as_str) != Some("Stop")
        || truthy(payload.get("agent_id"))
    {
        return "allow reason=not-top-level".to_owned();
    }
    if truthy(payload.get("background_tasks")) {
        return "allow reason=background".to_owned();
    }
    let events = parse_log(&read_log(run_dir));
    if events.iter().any(|e| e.item == "RUN" && e.event == "end") {
        return "allow reason=run-end".to_owned();
    }
    let count_path = cfg_dir.map(|d| d.join(COUNT_FILE));
    match outstanding_dispatch(&events) {
        None => {
            if let Some(path) = &count_path {
                let (old, n0) = read_count(path);
                if n0 > 0 {
                    let from = if old.is_empty() { "-" } else { old.as_str() };
                    pre.push(format!("reset from={from} to=-"));
                    let _ = fs::remove_file(path);
                }
            }
            "allow reason=no-outstanding".to_owned()
        }
        Some(_) if count_path.is_none() => "allow reason=no-config-dir".to_owned(),
        Some((item, index)) => {
            let Some(path) = count_path else {
                return "allow reason=no-config-dir".to_owned();
            };
            let key = format!("{item}@{index}");
            let (old, mut n) = read_count(&path);
            if old != key {
                let from = if old.is_empty() { "-" } else { old.as_str() };
                pre.push(format!("reset from={from} to={key}"));
                n = 0;
            }
            // Past the limit the value no longer matters.
            n = n.saturating_add(1).min(u64::from(GUARD_LIMIT) + 1);
            if write_count(&path, &key, n).is_err() {
                return "allow reason=counter-unwritable".to_owned();
            }
            if n <= u64::from(GUARD_LIMIT) {
                *stdout = json!({"decision": "block", "reason": block_reason(&item)}).to_string();
                format!("block item={item} attempt={n}")
            } else {
                "allow reason=limit".to_owned()
            }
        }
    }
}

/// Append `lines` to `<cfg_dir>/stop-guard.log` in one write.
pub fn append_log(cfg_dir: &Path, lines: &[String]) -> io::Result<()> {
    let mut text = lines.join("\n");
    text.push('\n');
    let mut file = fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open(cfg_dir.join(LOG_FILE))?;
    file.write_all(text.as_bytes())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    const STOP: &str = r#"{"hook_event_name":"Stop","session_id":"s","stop_hook_active":false,"background_tasks":[],"last_assistant_message":"PLANTED-SECRET-c0ffee"}"#;

    const DISPATCH: &str = "- 2026-10-17T02:59:05Z | I-01 | dispatched | step=/gsd-plan-phase 1 | base=e1003bc | round=0 | dispatch=1/8 | phase=1\n";
    const REPORT: &str = "- 2026-10-17T03:09:00Z | I-01 | report | status=done\n";
    const REDISPATCH: &str = "- 2026-10-17T03:10:00Z | I-01 | dispatched | step=/gsd-plan-phase 1 | base=e1003bc | round=0 | dispatch=2/8 | phase=1\n";
    const RUN_END: &str = "- 2026-10-17T03:08:00Z | RUN | end | reason=stop\n";

    fn now() -> DateTime<Utc> {
        DateTime::from_timestamp(1_800_000_000, 0).unwrap()
    }

    struct Fx {
        _root: TempDir,
        rd: std::path::PathBuf,
        cfg: std::path::PathBuf,
    }

    impl Fx {
        fn new(log: &str) -> Fx {
            let root = TempDir::new().unwrap();
            let rd = root.path().join("rd");
            let cfg = root.path().join("cfg");
            fs::create_dir_all(&rd).unwrap();
            fs::create_dir_all(&cfg).unwrap();
            fs::write(rd.join("LOG.md"), log).unwrap();
            Fx {
                _root: root,
                rd,
                cfg,
            }
        }

        fn put(&self, log: &str) {
            fs::write(self.rd.join("LOG.md"), log).unwrap();
        }

        /// One hook call: the decision line without its timestamp, and whether it blocked.
        fn call(&self, stdin: &str) -> (bool, String) {
            let out = guard(stdin, Some(&self.rd), Some(&self.cfg), now());
            append_log(&self.cfg, &out.lines).unwrap();
            let last = out.lines.last().unwrap().split_once(' ').unwrap().1;
            (!out.stdout.is_empty(), last.to_owned())
        }

        fn log_tail(&self, k: usize) -> Vec<String> {
            let text = fs::read_to_string(self.cfg.join(LOG_FILE)).unwrap();
            let lines: Vec<_> = text.lines().map(str::to_owned).collect();
            lines[lines.len().saturating_sub(k)..].to_vec()
        }

        fn counter(&self) -> Value {
            serde_json::from_str(&fs::read_to_string(self.cfg.join(COUNT_FILE)).unwrap()).unwrap()
        }
    }

    #[test]
    fn budget_is_three_blocks_then_the_limit_for_one_dispatch() {
        let fx = Fx::new(DISPATCH);
        let seen: Vec<_> = (0..5).map(|_| fx.call(STOP)).collect();
        let kinds: Vec<bool> = seen.iter().map(|(b, _)| *b).collect();
        assert_eq!(kinds, [true, true, true, false, false]);
        for (i, (_, line)) in seen.iter().take(3).enumerate() {
            assert_eq!(line, &format!("block item=I-01 attempt={}", i + 1));
        }
        assert_eq!(seen[3].1, "allow reason=limit");
        assert_eq!(seen[4].1, "allow reason=limit");
        assert_eq!(
            fx.counter()["n"],
            4,
            "the count stops at the limit plus one"
        );
    }

    #[test]
    fn the_block_output_is_one_json_object_with_the_spike_reason() {
        let fx = Fx::new(DISPATCH);
        let out = guard(STOP, Some(&fx.rd), Some(&fx.cfg), now());
        let parsed: Value = serde_json::from_str(&out.stdout).unwrap();
        assert_eq!(parsed.as_object().unwrap().len(), 2);
        assert_eq!(parsed["decision"], "block");
        let reason = parsed["reason"].as_str().unwrap();
        assert!(reason.contains("I-01") && reason.contains("SendMessage"));
        assert!(reason.contains("SAME") && reason.contains("Startup step 3"));
        assert!(
            !out.stdout.contains("PLANTED-SECRET"),
            "the payload is not echoed"
        );
    }

    #[test]
    fn a_new_outstanding_dispatch_resets_the_budget_and_logs_the_reset() {
        let fx = Fx::new(DISPATCH);
        for _ in 0..4 {
            fx.call(STOP);
        }
        let key0 = fx.counter()["key"].as_str().unwrap().to_owned();
        fx.put(&format!("{DISPATCH}{REPORT}{REDISPATCH}"));
        let (blocked, line) = fx.call(STOP);
        assert!(blocked && line == "block item=I-01 attempt=1", "{line}");
        let key1 = fx.counter()["key"].as_str().unwrap().to_owned();
        assert_ne!(key0, key1);
        assert_eq!(fx.counter()["n"], 1);
        let tail = fx.log_tail(2);
        assert!(
            tail[0].ends_with(&format!("reset from={key0} to={key1}")),
            "{tail:?}"
        );
        assert!(tail[1].ends_with("block item=I-01 attempt=1"), "{tail:?}");
    }

    #[test]
    fn nothing_outstanding_clears_the_counter_with_a_reset_line() {
        let fx = Fx::new(DISPATCH);
        fx.call(STOP);
        let key = fx.counter()["key"].as_str().unwrap().to_owned();
        fx.put(&format!("{DISPATCH}{REPORT}"));
        let (blocked, line) = fx.call(STOP);
        assert!(!blocked && line == "allow reason=no-outstanding", "{line}");
        assert!(fx.log_tail(2)[0].ends_with(&format!("reset from={key} to=-")));
        assert!(!fx.cfg.join(COUNT_FILE).exists());
        let before = fs::read_to_string(fx.cfg.join(LOG_FILE))
            .unwrap()
            .lines()
            .count();
        fx.call(STOP);
        let after = fs::read_to_string(fx.cfg.join(LOG_FILE))
            .unwrap()
            .lines()
            .count();
        assert_eq!(
            after,
            before + 1,
            "an already clear counter logs no second reset"
        );
    }

    #[test]
    fn the_allow_paths_and_none_of_them_touches_the_counter() {
        let fx = Fx::new(DISPATCH);
        let with = |key: &str, value: Value| {
            let mut payload: Value = serde_json::from_str(STOP).unwrap();
            payload[key] = value;
            payload.to_string()
        };
        assert_eq!(
            fx.call(&with("background_tasks", json!([{"task_id": "a1"}]))),
            (false, "allow reason=background".to_owned())
        );
        assert_eq!(
            fx.call(&with("agent_id", json!("a1"))),
            (false, "allow reason=not-top-level".to_owned())
        );
        assert_eq!(
            fx.call(&with("hook_event_name", json!("SubagentStop"))),
            (false, "allow reason=not-top-level".to_owned())
        );
        for text in ["{not json", "", "[1, 2]", "\"Stop\""] {
            assert_eq!(
                fx.call(text),
                (false, "allow reason=bad-input".to_owned()),
                "{text:?}"
            );
        }
        let out = guard(STOP, None, Some(&fx.cfg), now());
        assert!(out.stdout.is_empty() && out.lines[0].ends_with("allow reason=bad-input"));
        assert!(!fx.cfg.join(COUNT_FILE).exists());
    }

    #[test]
    fn a_falsy_agent_id_does_not_make_a_stop_a_subagent_stop() {
        for falsy in [json!(""), json!(null), json!(0), json!(false), json!([])] {
            let fx = Fx::new(DISPATCH);
            let mut payload: Value = serde_json::from_str(STOP).unwrap();
            payload["agent_id"] = falsy.clone();
            let (blocked, line) = fx.call(&payload.to_string());
            assert!(blocked, "{falsy}: {line}");
        }
    }

    #[test]
    fn run_end_wins_over_an_unreported_dispatch() {
        let fx = Fx::new(&format!("{DISPATCH}{RUN_END}"));
        assert_eq!(fx.call(STOP), (false, "allow reason=run-end".to_owned()));
        assert!(!fx.cfg.join(COUNT_FILE).exists());
    }

    #[test]
    fn without_a_config_dir_it_allows_and_writes_nothing() {
        let fx = Fx::new(DISPATCH);
        let out = guard(STOP, Some(&fx.rd), None, now());
        assert!(out.stdout.is_empty());
        assert!(out.lines[0].ends_with("allow reason=no-config-dir"));
        assert_eq!(fs::read_dir(&fx.cfg).unwrap().count(), 0);
    }

    #[test]
    fn a_missing_log_has_no_outstanding_dispatch() {
        let fx = Fx::new("");
        fs::remove_file(fx.rd.join("LOG.md")).unwrap();
        assert_eq!(
            fx.call(STOP),
            (false, "allow reason=no-outstanding".to_owned())
        );
    }

    #[test]
    fn a_forged_counter_can_never_extend_the_budget() {
        let live_index = {
            let events = parse_log(DISPATCH);
            outstanding_dispatch(&events).unwrap().1
        };
        let forged = [
            ("negative bare integer", "-999\n".to_owned()),
            ("garbage", "garbage\n".to_owned()),
            ("legacy bare integer", "3\n".to_owned()),
            ("negative n", r#"{"key": "I-01@0", "n": -999}"#.to_owned()),
            ("string n", r#"{"key": "I-01@0", "n": "1"}"#.to_owned()),
            ("boolean n", r#"{"key": "I-01@0", "n": true}"#.to_owned()),
            ("fractional n", r#"{"key": "I-01@0", "n": 1.5}"#.to_owned()),
            ("missing key", r#"{"n": 2}"#.to_owned()),
            ("numeric key", r#"{"key": 7, "n": 2}"#.to_owned()),
            ("a list", "[1, 2]".to_owned()),
            ("empty", String::new()),
            (
                "oversized",
                format!(r#"{{"key": "{}", "n": 0}}"#, "k".repeat(5000)),
            ),
        ];
        for (label, content) in forged {
            let fx = Fx::new(DISPATCH);
            fs::write(fx.cfg.join(COUNT_FILE), content).unwrap();
            let results: Vec<_> = (0..6).map(|_| fx.call(STOP)).collect();
            let blocks = results.iter().filter(|(b, _)| *b).count();
            assert_eq!(blocks, 3, "{label}: {results:?}");
            assert_eq!(results[0].1, "block item=I-01 attempt=1", "{label}");
        }
        // A forged count naming the live key is honoured only as far as the clamp.
        let fx = Fx::new(DISPATCH);
        fs::write(
            fx.cfg.join(COUNT_FILE),
            json!({"key": format!("I-01@{live_index}"), "n": 2}).to_string(),
        )
        .unwrap();
        assert_eq!(
            fx.call(STOP),
            (true, "block item=I-01 attempt=3".to_owned())
        );
        assert_eq!(fx.call(STOP), (false, "allow reason=limit".to_owned()));
        // An absurd count cannot overflow into a block.
        let fx = Fx::new(DISPATCH);
        fs::write(
            fx.cfg.join(COUNT_FILE),
            format!(r#"{{"key": "I-01@{live_index}", "n": 18446744073709551615}}"#),
        )
        .unwrap();
        assert_eq!(fx.call(STOP), (false, "allow reason=limit".to_owned()));
    }

    #[test]
    fn an_unwritable_counter_fails_open_rather_than_blocking_without_a_budget() {
        let fx = Fx::new(DISPATCH);
        // A directory where the counter must go: the rename onto it fails.
        fs::create_dir(fx.cfg.join(COUNT_FILE)).unwrap();
        let (blocked, line) = fx.call(STOP);
        assert!(!blocked, "{line}");
        assert_eq!(line, "allow reason=counter-unwritable");
    }

    #[test]
    fn it_never_writes_under_the_run_dir() {
        let fx = Fx::new(DISPATCH);
        let before: Vec<_> = fs::read_dir(&fx.rd)
            .unwrap()
            .filter_map(Result::ok)
            .map(|e| e.file_name())
            .collect();
        for _ in 0..5 {
            fx.call(STOP);
        }
        let after: Vec<_> = fs::read_dir(&fx.rd)
            .unwrap()
            .filter_map(Result::ok)
            .map(|e| e.file_name())
            .collect();
        assert_eq!(before, after);
        assert_eq!(fs::read_to_string(fx.rd.join("LOG.md")).unwrap(), DISPATCH);
    }
}
