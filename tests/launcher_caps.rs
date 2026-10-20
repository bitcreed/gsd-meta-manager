#![cfg(target_os = "linux")]
// ============================================================================
// The hard caps and the launch-end verdicts, through the REAL binary (Phase 28,
// plan 28-05; LNCH-03, LNCH-05 revocation half; D-12..D-16, D-23).
//
// `systemd-run`, `bwrap`, `systemctl` and `claude` are the fakes of
// `tests/launcher_support`, so this runs on any runner: it proves the launcher's
// decisions (what it denies, when it kills, how it labels a launch end), not the
// kernel's. The live L1-L5 runs against a real scope are the 28-09 harness.
//
// VERIFICATION writes are made with explicit mtimes 50 s apart (`File::set_modified`),
// so the Phase 27 coalescing window (45 s) never needs a real wait (RESEARCH §7).
// ============================================================================

#[path = "launcher_support/mod.rs"]
mod launcher_support;

use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use chrono::{DateTime, SecondsFormat};

use gsd_meta_manager::driver::lock::run_lock_path;
use launcher_support::{wait_until, Fixture, RUN_ID, STARTED_FILE};

fn stderr(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn stdout(out: &std::process::Output) -> String {
    String::from_utf8_lossy(&out.stdout).into_owned()
}

/// A driver that starts, announces itself and then answers gate requests the test
/// drops into the recording directory: `gate_<n>.json` in, `gate_<n>.out` and
/// `gate_<n>.done` out. It never ends the run itself, so only a cap or a stop can.
const RELAY_SCENARIO: &str = r#"
driverctl log start --mode headless --loop-mode loop
touch "$FAKE_CLAUDE_DIR/started"
n=1
while true; do
  if [ -f "$FAKE_CLAUDE_DIR/gate_$n.json" ]; then
    driverctl hook < "$FAKE_CLAUDE_DIR/gate_$n.json" > "$FAKE_CLAUDE_DIR/gate_$n.out"
    touch "$FAKE_CLAUDE_DIR/gate_$n.done"
    n=$((n + 1))
  fi
  sleep 0.1
done
"#;

/// A gate payload as Claude Code sends it: a driver-level `Agent` dispatch.
fn gate_payload(prompt: &str) -> String {
    serde_json::json!({
        "hook_event_name": "PreToolUse",
        "tool_name": "Agent",
        "session_id": "s1",
        "tool_input": {"prompt": prompt},
    })
    .to_string()
}

/// Ask the running driver (the relay scenario) to put `prompt` to the gate and
/// return what the hook printed.
fn ask_gate(fx: &Fixture, n: u32, prompt: &str) -> String {
    let rec = fx.rec_dir();
    fs::write(rec.join(format!("gate_{n}.json")), gate_payload(prompt)).expect("gate request");
    assert!(
        wait_until(Duration::from_secs(10), || rec
            .join(format!("gate_{n}.done"))
            .exists()),
        "the driver never answered gate request {n}"
    );
    fs::read_to_string(rec.join(format!("gate_{n}.out"))).unwrap_or_default()
}

/// Write a `gaps_found` VERIFICATION.md for phase 1 whose mtime is `slot * 50` s
/// after a fixed point in the past, with content that differs per round.
fn write_gaps(project: &Path, round: u32) {
    let dir = project.join(".planning/phases/01-x");
    fs::create_dir_all(&dir).expect("phase dir");
    let path = dir.join("01-VERIFICATION.md");
    fs::write(
        &path,
        format!("---\nstatus: gaps_found\n---\nround {round}\n"),
    )
    .expect("write VERIFICATION.md");
    let base = SystemTime::now() - Duration::from_secs(10_000);
    let mtime = base + Duration::from_secs(50 * u64::from(round));
    fs::File::options()
        .write(true)
        .open(&path)
        .expect("reopen VERIFICATION.md")
        .set_modified(mtime)
        .expect("set the mtime");
}

/// A `state.json` of a run that already used `secs` active seconds, as an earlier
/// launch would have left it (the launcher-owned `active_seconds` key).
fn seed_active_seconds(fx: &Fixture, secs: f64) -> Vec<u8> {
    let bytes = serde_json::to_vec(&serde_json::json!({
        "schema": 1,
        "run_id": RUN_ID,
        "project_root": fx.project(),
        "active_seconds": secs,
    }))
    .expect("state.json");
    fs::write(fx.run_dir.join("state.json"), &bytes).expect("seed state.json");
    bytes
}

fn gap_lines(log: &str) -> Vec<&str> {
    log.lines()
        .filter(|l| l.contains("RUN | gap-observed"))
        .collect()
}

fn cap_tripped_lines(log: &str) -> Vec<&str> {
    log.lines()
        .filter(|l| l.contains("RUN | cap-tripped"))
        .collect()
}

#[test]
fn dispatch_cap_denies_the_fourth_dispatch() {
    let fx = Fixture::new();
    fx.write_scenario(
        r#"
driverctl log start --mode headless --loop-mode loop
for n in 1 2 3 4; do
  printf '%s' '{"hook_event_name":"PreToolUse","tool_name":"Agent","session_id":"s1","tool_input":{"prompt":"run phase 1"}}' | driverctl hook > "$FAKE_CLAUDE_DIR/hook_$n.txt"
done
driverctl log end --reason stop
printf '%s\n' '{"type":"result","subtype":"success"}'
"#,
    );

    let out = fx.launch(&["--cap-dispatches", "3"]);
    assert!(out.status.success(), "{}", stderr(&out));

    for n in 1..=3 {
        let printed = fx.recorded(&format!("hook_{n}.txt"));
        assert!(
            !printed.contains("LAUNCHER CAP"),
            "dispatch {n} is allowed: {printed}"
        );
    }
    let fourth = fx.recorded("hook_4.txt");
    assert!(
        fourth.contains("LAUNCHER CAP: dispatch cap reached (3/3)"),
        "{fourth}"
    );

    let log = fx.log();
    let denied: Vec<&str> = log
        .lines()
        .filter(|l| l.contains("RUN | dispatch-denied"))
        .collect();
    assert_eq!(denied.len(), 1, "{log}");
    assert!(
        denied[0].contains("reason=dispatch cap reached (3/3)."),
        "{}",
        denied[0]
    );
    let state = fx.state_json();
    assert_eq!(state["denied_dispatches"], 1);
    assert_eq!(state["hook_dispatches"], 3);
    assert_eq!(state["caps"]["dispatches"]["used"], 3);
}

#[test]
fn limit_zero_denies_the_first_dispatch() {
    let fx = Fixture::new();
    fx.write_scenario(
        r#"
driverctl log start --mode headless --loop-mode loop
printf '%s' '{"hook_event_name":"PreToolUse","tool_name":"Task","session_id":"s1","tool_input":{"prompt":"x"}}' | driverctl hook > "$FAKE_CLAUDE_DIR/hook_1.txt"
driverctl log end --reason stop
printf '%s\n' '{"type":"result","subtype":"success"}'
"#,
    );
    let out = fx.launch(&["--cap-dispatches", "0"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        fx.recorded("hook_1.txt")
            .contains("LAUNCHER CAP: dispatch cap reached (0/0)"),
        "{}",
        fx.recorded("hook_1.txt")
    );
    assert_eq!(fx.state_json()["hook_dispatches"], 0);
}

#[test]
fn gap_cap_denies_at_limit_plus_one_and_kills_at_limit_plus_two() {
    let fx = Fixture::new();
    fx.write_scenario(RELAY_SCENARIO);
    let launcher = fx.spawn_launch(&["--cap-gap-rounds", "2", "--cap-dispatches", "50"]);
    assert!(
        wait_until(Duration::from_secs(10), || fx.scenario_started()),
        "the launcher never reached its driver"
    );

    // Three gap rounds: the count reaches L + 1 = 3.
    for round in 1..=3 {
        write_gaps(fx.project(), round);
        let want = format!("RUN | gap-observed | phase=1 | count={round} |");
        assert!(
            wait_until(Duration::from_secs(15), || fx.log().contains(&want)),
            "round {round} was never observed:\n{}",
            fx.log()
        );
    }

    // At count 3 = L + 1 a dispatch naming phase 1 is denied; one naming phase 2 is not.
    let denied = ask_gate(&fx, 1, "/gsd-plan-phase 1 --gaps");
    assert!(
        denied.contains("LAUNCHER CAP: phase 1 has failed verification 3 times (gap-round cap 2)"),
        "{denied}"
    );
    let allowed = ask_gate(&fx, 2, "run phase 2 now");
    assert!(!allowed.contains("LAUNCHER CAP"), "{allowed}");
    assert!(
        cap_tripped_lines(&fx.log()).is_empty(),
        "no kill below L + 2:\n{}",
        fx.log()
    );

    // Rescanning the unchanged file adds no round (Phase 27 digest dedupe).
    std::thread::sleep(Duration::from_millis(4500));
    assert_eq!(gap_lines(&fx.log()).len(), 3, "{}", fx.log());

    // The fourth round is L + 2: the whole scope is killed.
    write_gaps(fx.project(), 4);
    let out = launcher
        .wait_with_output()
        .expect("the launcher ends after the kill");
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stdout(&out).contains("ended status=stopped reason=launcher-cap:gap_rounds"),
        "{}",
        stdout(&out)
    );

    let log = fx.log();
    let tripped = cap_tripped_lines(&log);
    assert_eq!(tripped.len(), 1, "{log}");
    assert!(
        tripped[0]
            .ends_with("RUN | cap-tripped | cap=gap_rounds | used=4 | limit=2 | action=kill-scope"),
        "{}",
        tripped[0]
    );
    let denials: Vec<&str> = log
        .lines()
        .filter(|l| l.contains("RUN | dispatch-denied"))
        .collect();
    assert_eq!(denials.len(), 1, "{log}");
    assert!(
        denials[0].contains("reason=phase 1 has failed verification 3 times (gap-round cap 2)"),
        "{}",
        denials[0]
    );
    assert!(
        fx.shim_log()
            .lines()
            .any(|l| l.starts_with("systemctl --user stop gsdmm-driver-r28-")),
        "the scope was stopped:\n{}",
        fx.shim_log()
    );
    assert!(!fx.any_driver_alive(), "no process of the scope survives");
    let state = fx.state_json();
    assert_eq!(state["status"], "stopped");
    assert_eq!(state["status_reason"], "launcher-cap:gap_rounds");
    // The cap-tripped line precedes the launch end (T-28-17).
    let trip_at = log.find("RUN | cap-tripped").expect("cap-tripped");
    let end_at = log.rfind("RUN | launch-end").expect("launch-end");
    assert!(trip_at < end_at, "{log}");
}

#[test]
fn wall_clock_cap_kills_the_scope() {
    let fx = Fixture::new();
    fx.write_scenario(
        "driverctl log start --mode headless --loop-mode loop\n\
         touch \"$FAKE_CLAUDE_DIR/started\"\n\
         sleep 60\n",
    );

    // 0.05 min = 3 s of active wall clock.
    let out = fx.launch(&["--cap-minutes", "0.05"]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert!(
        stdout(&out).contains("ended status=stopped reason=launcher-cap:wall_clock_min"),
        "{}",
        stdout(&out)
    );

    let log = fx.log();
    let tripped = cap_tripped_lines(&log);
    assert_eq!(tripped.len(), 1, "{log}");
    assert!(
        tripped[0].contains("cap=wall_clock_min | used=0.1 | limit=0.05 | action=kill-scope"),
        "{}",
        tripped[0]
    );
    let state = fx.state_json();
    assert_eq!(state["status"], "stopped");
    assert_eq!(state["status_reason"], "launcher-cap:wall_clock_min");
    let active = state["active_seconds"].as_f64().expect("active_seconds");
    assert!(
        (3.0..30.0).contains(&active),
        "the run was killed soon after its 3 s: {active}"
    );
    assert_eq!(state["caps"]["wall_clock_min"]["used"], 0.1);

    // D-13: ceil(3.0) + 300 as the systemd backstop.
    let unit_line = fx
        .shim_log()
        .lines()
        .find(|l| l.starts_with("systemd-run --user"))
        .map(str::to_owned)
        .expect("a systemd-run line");
    assert!(
        unit_line.contains("-p RuntimeMaxSec=303 "),
        "RuntimeMaxSec is ceil(remaining) + 300: {unit_line}"
    );
    assert!(!fx.any_driver_alive());
}

#[test]
fn runtime_max_sec_counts_the_active_seconds_the_run_already_used() {
    let fx = Fixture::new();
    // 10 min cap, 590.5 s already used: 9.5 s left -> ceil = 10, + 300.
    seed_active_seconds(&fx, 590.5);
    fx.write_scenario(launcher_support::WELL_MANNERED_SCENARIO);
    let out = fx.launch(&["--cap-minutes", "10"]);
    assert!(out.status.success(), "{}", stderr(&out));
    let unit_line = fx
        .shim_log()
        .lines()
        .find(|l| l.starts_with("systemd-run --user"))
        .map(str::to_owned)
        .expect("a systemd-run line");
    assert!(unit_line.contains("-p RuntimeMaxSec=310 "), "{unit_line}");
    // The launch's own seconds are added to the earlier ones and persisted.
    let active = fx.state_json()["active_seconds"].as_f64().unwrap();
    assert!(active >= 590.5, "{active}");
}

#[test]
fn exhausted_wall_clock_refuses_the_launch() {
    let fx = Fixture::new();
    let state_bytes = seed_active_seconds(&fx, 700.0);
    fx.write_scenario(launcher_support::WELL_MANNERED_SCENARIO);

    // 10 min = 600 s, 700 s used.
    let out = fx.launch(&["--cap-minutes", "10"]);
    assert_eq!(out.status.code(), Some(1), "{}", stderr(&out));
    assert!(stderr(&out).contains("cap exhausted"), "{}", stderr(&out));
    assert!(stderr(&out).contains("--cap-minutes"), "{}", stderr(&out));

    // Nothing was written: no lock file, no log, state.json as it was, no driver.
    assert!(!run_lock_path(&fx.run_dir).exists());
    assert!(!fx.run_dir.join("LOG.md").exists());
    assert_eq!(
        fs::read(fx.run_dir.join("state.json")).unwrap(),
        state_bytes
    );
    assert!(!fx.state_dir().exists(), "no launcher state directory");
    let shim = fx.shim_log();
    assert!(!shim.contains("systemd-run --user"), "{shim}");
    assert!(!shim.contains("claude -p"), "{shim}");

    // Exactly at the cap is also nothing left (remaining <= 0) ...
    seed_active_seconds(&fx, 600.0);
    let at_cap = fx.launch(&["--cap-minutes", "10"]);
    assert_eq!(at_cap.status.code(), Some(1), "{}", stderr(&at_cap));
    // ... and a raised cap launches again.
    seed_active_seconds(&fx, 700.0);
    let raised = fx.launch(&["--cap-minutes", "20"]);
    assert!(raised.status.success(), "{}", stderr(&raised));
}

#[test]
fn a_tripped_run_denies_every_later_dispatch_before_the_scope_has_exited() {
    let fx = Fixture::new();
    // A driver that ignores SIGTERM: the fake `systemctl stop` waits for it, so the
    // run stays alive (and answering the gate) well after `cap-tripped` is recorded.
    fx.write_scenario(&format!("trap '' TERM\n{RELAY_SCENARIO}"));
    let launcher = fx.spawn_launch(&["--cap-minutes", "0.05", "--cap-dispatches", "50"]);
    assert!(
        wait_until(Duration::from_secs(10), || fx
            .rec_dir()
            .join(STARTED_FILE)
            .exists()),
        "the launcher never reached its driver"
    );
    assert!(
        wait_until(Duration::from_secs(20), || !cap_tripped_lines(&fx.log())
            .is_empty()),
        "the wall clock never tripped:\n{}",
        fx.log()
    );

    let printed = ask_gate(&fx, 1, "any dispatch at all");
    assert!(
        printed.contains("LAUNCHER CAP: run stopped by launcher cap wall_clock_min"),
        "{printed}"
    );

    let out = launcher.wait_with_output().expect("the launcher ends");
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(cap_tripped_lines(&fx.log()).len(), 1, "{}", fx.log());
    assert_eq!(
        fx.state_json()["status_reason"],
        "launcher-cap:wall_clock_min"
    );
}

// ---------------------------------------------------------------- launch-end verdicts

fn epoch_now() -> i64 {
    i64::try_from(
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("after the epoch")
            .as_secs(),
    )
    .expect("fits")
}

fn launch_lines(log: &str) -> usize {
    log.lines().filter(|l| l.contains("RUN | launch |")).count()
}

/// The host `~/.claude/.credentials.json` the host-token path reads, expiring in
/// `expires_in_s` seconds. The refresh token is a decoy no code may ever read.
fn write_host_credentials(fx: &Fixture, access: &str, expires_in_s: i64) {
    let dir = fx.home().join(".claude");
    fs::create_dir_all(&dir).expect("host claude dir");
    fs::write(
        dir.join(".credentials.json"),
        serde_json::json!({"claudeAiOauth": {
            "accessToken": access,
            "refreshToken": "rt-decoy-must-never-appear",
            "expiresAt": (epoch_now() + expires_in_s) * 1000,
        }})
        .to_string(),
    )
    .expect("host credentials");
}

/// A driver that records the token it was given, rewrites the host credentials the
/// way a host refresh would (`at-host-<n+1>`, valid for `next_expiry_s` more
/// seconds) and ends with an authentication-failure result.
fn revoked_scenario(next_expiry_s: i64) -> String {
    format!(
        r#"
n=$(cat "$FAKE_CLAUDE_DIR/count" 2>/dev/null || echo 0)
n=$((n + 1))
echo "$n" > "$FAKE_CLAUDE_DIR/count"
printf '%s\n' "$CLAUDE_CODE_OAUTH_TOKEN" >> "$FAKE_CLAUDE_DIR/tokens.txt"
exp=$((($(date +%s) + {next_expiry_s}) * 1000))
printf '{{"claudeAiOauth":{{"accessToken":"at-host-%s","refreshToken":"rt-decoy-must-never-appear","expiresAt":%s}}}}' "$((n + 1))" "$exp" > "$HOME/.claude/.credentials.json"
printf '%s\n' '{{"type":"result","subtype":"success","is_error":true,"result":"Failed to authenticate: OAuth token revoked"}}'
exit 1
"#
    )
}

#[test]
fn rate_limited_launch_parks_without_retry() {
    let fx = Fixture::new();
    let reset = epoch_now() + 3600;
    fx.write_scenario(&format!(
        r#"
driverctl log start --mode headless --loop-mode loop
printf '%s\n' '{{"type":"rate_limit_event","rate_limit_info":{{"status":"rejected","rateLimitType":"five_hour","resetsAt":{reset}}}}}'
printf '%s\n' '{{"type":"result","subtype":"error_during_execution","is_error":true,"terminal_reason":"api_error","result":"usage limit reached"}}'
exit 1
"#
    ));

    let out = fx.launch(&[]);
    assert!(out.status.success(), "{}", stderr(&out));

    let resets_at = DateTime::from_timestamp(reset, 0)
        .expect("a timestamp")
        .to_rfc3339_opts(SecondsFormat::Secs, true);
    let want = format!("rate-limit: window=five_hour resets_at={resets_at}");
    let state = fx.state_json();
    assert_eq!(state["status"], "parked");
    assert_eq!(state["status_reason"], want.as_str());
    assert!(
        stdout(&out).contains(&format!("ended status=parked reason={want}")),
        "{}",
        stdout(&out)
    );
    let log = fx.log();
    assert_eq!(launch_lines(&log), 1, "a parked run is not retried:\n{log}");
    assert!(
        log.contains(&format!("RUN | launch-end | status=parked | reason={want}")),
        "{log}"
    );
    assert!(!log.contains("auth-relaunch"), "{log}");
}

#[test]
fn a_terminal_reason_naming_a_rate_limit_parks_with_an_unknown_window() {
    let fx = Fixture::new();
    fx.write_scenario(
        r#"
printf '%s\n' '{"type":"result","subtype":"error_during_execution","is_error":true,"terminal_reason":"api_error: rate_limit","result":"x"}'
exit 1
"#,
    );
    let out = fx.launch(&[]);
    assert!(out.status.success(), "{}", stderr(&out));
    let state = fx.state_json();
    assert_eq!(state["status"], "parked");
    assert_eq!(
        state["status_reason"],
        "rate-limit: window=unknown resets_at=unknown"
    );
}

#[test]
fn an_allowed_quota_event_does_not_park_a_crash() {
    let fx = Fixture::new();
    fx.write_scenario(
        r#"
printf '%s\n' '{"type":"rate_limit_event","rate_limit_info":{"status":"allowed_warning","rateLimitType":"seven_day","resetsAt":1900000000}}'
exit 1
"#,
    );
    let out = fx.launch(&[]);
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(fx.state_json()["status"], "crashed");
}

#[test]
fn revoked_host_token_relaunches_twice_then_parks() {
    let fx = Fixture::new();
    fx.write_token(None);
    write_host_credentials(&fx, "at-host-1", 3600);
    fx.write_scenario(&revoked_scenario(3600));

    let out = fx.launch(&[]);
    assert!(out.status.success(), "{}", stderr(&out));

    let log = fx.log();
    assert!(
        log.contains("RUN | auth-relaunch | attempt=1 | source=host"),
        "{log}"
    );
    assert!(
        log.contains("RUN | auth-relaunch | attempt=2 | source=host"),
        "{log}"
    );
    assert!(!log.contains("attempt=3"), "{log}");
    assert_eq!(launch_lines(&log), 3, "{log}");
    let state = fx.state_json();
    assert_eq!(state["status"], "parked", "never crashed:\n{log}");
    assert_eq!(
        state["status_reason"],
        "auth: host token rejected after 2 relaunches"
    );
    assert_eq!(state["auth_relaunches"], 2);
    assert_eq!(state["launches"], 3);

    // Every relaunch re-read the host's CURRENT access token ...
    assert_eq!(
        fx.recorded("tokens.txt"),
        "at-host-1\nat-host-2\nat-host-3\n"
    );
    // ... and neither it nor the refresh token reached any record.
    for secret in ["at-host", "rt-decoy"] {
        assert!(!log.contains(secret), "{log}");
        assert!(!fx.run_file("state.json").contains(secret));
        assert!(!fx.run_file("DIRECTION.md").contains(secret));
    }
    assert!(!fx.any_driver_alive());
}

#[test]
fn a_host_token_about_to_expire_is_not_relaunched_with() {
    let fx = Fixture::new();
    fx.write_token(None);
    write_host_credentials(&fx, "at-host-1", 3600);
    // The "refreshed" host token has 60 s left: under the 5 minute floor.
    fx.write_scenario(&revoked_scenario(60));

    let out = fx.launch(&[]);
    assert!(out.status.success(), "{}", stderr(&out));
    let state = fx.state_json();
    assert_eq!(state["status"], "parked");
    assert_eq!(
        state["status_reason"],
        "auth: host token expires within 5 minutes; refresh the host login"
    );
    assert_eq!(launch_lines(&fx.log()), 1, "{}", fx.log());
}

#[test]
fn revoked_stored_token_parks_without_relaunch() {
    let fx = Fixture::new();
    // The default fixture stores a token; the same scenario rewrites the host file
    // too, which a stored-token run must ignore.
    write_host_credentials(&fx, "at-host-1", 3600);
    fx.write_scenario(&revoked_scenario(3600));

    let out = fx.launch(&[]);
    assert!(out.status.success(), "{}", stderr(&out));

    let log = fx.log();
    assert_eq!(launch_lines(&log), 1, "{log}");
    assert!(!log.contains("auth-relaunch"), "{log}");
    let state = fx.state_json();
    assert_eq!(state["status"], "parked");
    assert_eq!(
        state["status_reason"],
        "auth: stored token rejected \u{2014} rerun driver auth setup"
    );
    assert_eq!(
        fx.recorded("tokens.txt"),
        format!("{}\n", launcher_support::TOKEN)
    );
}

#[test]
fn clean_exit_with_unreported_dispatch_is_stalled() {
    let fx = Fixture::new();
    // run1b-shaped: the driver dispatches a worker, never sees its report, ends its
    // turn cleanly with a successful result.
    fx.write_scenario(
        r#"
driverctl log start --mode headless --loop-mode loop
driverctl log dispatched --item I-01 --step plan
printf '%s\n' '{"type":"result","subtype":"success"}'
exit 0
"#,
    );
    let out = fx.launch(&[]);
    assert!(out.status.success(), "{}", stderr(&out));
    let state = fx.state_json();
    assert_eq!(state["status"], "stalled");
    let reason = state["status_reason"].as_str().expect("a reason");
    assert!(
        reason.contains("I-01") && reason.contains("not a crash"),
        "{reason}"
    );
    assert_eq!(launch_lines(&fx.log()), 1);
}

#[test]
fn a_dying_driver_without_run_end_is_crashed() {
    let fx = Fixture::new();
    fx.write_scenario(
        r#"
driverctl log start --mode headless --loop-mode loop
exit 3
"#,
    );
    let out = fx.launch(&[]);
    assert!(out.status.success(), "{}", stderr(&out));
    let state = fx.state_json();
    assert_eq!(state["status"], "crashed");
    assert_eq!(state["status_reason"], "driver exited rc=3 without RUN end");
}

#[test]
fn a_fresh_exit_relaunches_and_a_user_stop_still_beats_it() {
    let fx = Fixture::new();
    fx.write_scenario(
        r#"
driverctl log start --mode headless --loop-mode fresh-per-item
n=$(cat "$FAKE_CLAUDE_DIR/count" 2>/dev/null || echo 0)
n=$((n + 1))
echo "$n" > "$FAKE_CLAUDE_DIR/count"
if [ "$n" -lt 2 ]; then
  driverctl log end --reason fresh-exit
  printf '%s\n' '{"type":"result","subtype":"success"}'
  exit 0
fi
touch "$FAKE_CLAUDE_DIR/started"
sleep 60
"#,
    );
    let launcher = fx.spawn_launch(&["--loop-mode", "fresh-per-item"]);
    assert!(
        wait_until(Duration::from_secs(15), || fx.scenario_started()),
        "the second launch never started:\n{}",
        fx.log()
    );
    assert_eq!(launch_lines(&fx.log()), 2, "{}", fx.log());
    let stop = fx
        .stop_command(RUN_ID, &[])
        .output()
        .expect("spawn driver stop");
    assert!(stop.status.success(), "{}", stderr(&stop));
    let out = launcher.wait_with_output().expect("the launcher ends");
    assert!(out.status.success(), "{}", stderr(&out));
    assert_eq!(fx.state_json()["status_reason"], "stop:user");
}

/// RESEARCH A1 (the offline gate for plan 29-07's H3/H4): a `status: gaps_found`
/// VERIFICATION.md that exists BEFORE the launch is not baseline. The launcher's
/// per-tick scan counts it on the first tick, so `gap-observed | phase=1 | count=1`
/// is in LOG.md before a driver that waits for it sends its first hook. A harness
/// that wants a gap round counted therefore pre-writes the file; one that wants a
/// clean count must not leave a stale `gaps_found` file in the project.
#[test]
fn a_gaps_found_file_present_at_launch_is_counted_on_the_first_tick() {
    let fx = Fixture::new();
    write_gaps(fx.project(), 1);
    let log_path = fx.run_dir.join("LOG.md");
    fx.write_scenario(&format!(
        r#"
D="$FAKE_CLAUDE_DIR"
driverctl log start --mode headless --loop-mode loop
seen=no
i=0
while [ "$i" -lt 150 ]; do
  if grep -q 'RUN | gap-observed' '{log}'; then seen=yes; break; fi
  sleep 0.1
  i=$((i + 1))
done
echo "$seen" > "$D/gap_seen_before_hook"
printf '%s' '{{"hook_event_name":"PreToolUse","tool_name":"Agent","session_id":"s1","tool_input":{{"prompt":"run phase 1"}}}}' | driverctl hook > "$D/hook_out.txt"
driverctl log end --reason stop
printf '%s\n' '{{"type":"result","subtype":"success"}}'
"#,
        log = log_path.display()
    ));
    let out = fx.launch(&[]);
    assert!(out.status.success(), "{}", stderr(&out));

    assert_eq!(
        fx.recorded("gap_seen_before_hook").trim(),
        "yes",
        "the pre-existing gaps_found file was not counted before the first hook:\n{}",
        fx.log()
    );
    let log = fx.log();
    let want = "RUN | gap-observed | phase=1 | count=1 |";
    assert_eq!(gap_lines(&log).len(), 1, "{log}");
    assert!(
        log.contains(want),
        "no `{want}` for the file present at launch:\n{log}"
    );
}
