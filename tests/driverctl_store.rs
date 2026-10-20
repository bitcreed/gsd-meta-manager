// ============================================================================
// Library-level tests for the driverctl `RunStore` (DCTL-01, DCTL-04).
//
// The store is exercised through its public surface only: a request goes in as
// JSON bytes (`handle_json`, the shape the 27-06 socket server will hand it) and
// the three files a run owns - LOG.md, state.json and DIRECTION.md - are read
// back from disk. Nothing here spawns a process other than `git`, and no test
// reads the wall clock for a decision: the clock is injected wherever time
// matters.
//
// The `RunLock` is acquired for real (`acquire_at_with_retry`), because the type
// system is the point: `RunStore::open` takes the guard by value.
// ============================================================================

#![cfg(target_os = "linux")]

use std::fs;
use std::path::PathBuf;

use chrono::{DateTime, Utc};
use gsd_meta_manager::driver::direction::direction_lock;
use gsd_meta_manager::driver::direction::parse_direction;
use gsd_meta_manager::driver::lock::{acquire_at_with_retry, run_lock_path, ACQUIRE_RETRY_BUDGET};
use gsd_meta_manager::driverctl::protocol::Response;
use gsd_meta_manager::driverctl::store::{
    frontmatter_for, CapLimits, DriverSpec, GateDecision, LaunchSpec, RunState, RunStore,
    StoreError, TerminalOutcome, TerminalStatus,
};
use gsd_meta_manager::launcher::gate::{new_tripped, CapGate};
use gsd_meta_manager::state_reader::driver_frontmatter::{
    parse_driver_frontmatter, ItemState, RunStatus,
};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, UNIX_EPOCH};
use tempfile::TempDir;

/// The human-owned DIRECTION body: two items, I-01 with a dispatch budget of 3.
const BODY: &str = "# Driver direction: store test

## Direction

- control: run
- goal: Exercise the store.

## Items

### I-01 First slice

- intent: Phase 1 is planned, executed and verified.
- budget: dispatches=3

### I-02 Second slice

- intent: Phase 2 is planned, executed and verified.
";

struct Fixture {
    _td: TempDir,
    project: PathBuf,
    run_dir: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        Self::with_body(BODY)
    }

    fn with_body(body: &str) -> Self {
        let td = TempDir::new().unwrap();
        let project = td.path().to_path_buf();
        let run_dir = project.join(".planning/meta-manager/drivers/r27");
        fs::create_dir_all(&run_dir).unwrap();
        fs::write(run_dir.join("DIRECTION.md"), body).unwrap();
        Fixture {
            _td: td,
            project,
            run_dir,
        }
    }

    fn open(&self) -> RunStore {
        let lock = acquire_at_with_retry(
            &run_lock_path(&self.run_dir),
            "r27",
            std::process::id(),
            ACQUIRE_RETRY_BUDGET,
        )
        .expect("run lock");
        RunStore::open(&self.run_dir, &self.project, lock).expect("open store")
    }

    fn read(&self, name: &str) -> String {
        fs::read_to_string(self.run_dir.join(name)).unwrap_or_default()
    }

    fn log(&self) -> String {
        self.read("LOG.md")
    }

    fn state_json(&self) -> serde_json::Value {
        serde_json::from_str(&self.read("state.json")).expect("state.json parses")
    }
}

fn standalone() -> LaunchSpec {
    LaunchSpec {
        launcher_mode: "standalone".to_owned(),
        ..LaunchSpec::default()
    }
}

fn send(store: &mut RunStore, request: &str) -> Response {
    store.handle_json(request.as_bytes())
}

fn expect_ok(r: &Response) {
    assert!(r.ok, "expected ok, got {r:?}");
}

/// The event headers of a LOG.md with the timestamp stripped.
fn log_events(log: &str) -> Vec<String> {
    log.lines()
        .filter(|l| l.starts_with("- "))
        .map(|l| l.split_once(" | ").map_or("", |x| x.1).to_owned())
        .collect()
}

// ---------------------------------------------------------------- tracer

#[test]
fn start_event_writes_log_state_and_frontmatter() {
    let fx = Fixture::new();
    let mut store = fx.open();
    store.begin_launch(standalone()).expect("begin_launch");
    let r = send(
        &mut store,
        r#"{"op":"log","event":"start","args":{"mode":"headless","loop_mode":"loop","session":null}}"#,
    );
    expect_ok(&r);

    assert_eq!(
        log_events(&fx.log()),
        vec![
            "RUN | launch | n=1 | mode=standalone".to_owned(),
            "RUN | start | mode=headless | loop_mode=loop".to_owned(),
        ]
    );

    let state = fx.state_json();
    assert_eq!(state["status"], "running");
    assert_eq!(state["launches"], 1);

    let direction = fx.read("DIRECTION.md");
    let fm = parse_driver_frontmatter(&direction).expect("frontmatter parses");
    assert_eq!(fm.status, RunStatus::Running);
    assert_eq!(fm.launches, 1);
    let ids: Vec<(&str, ItemState)> = fm
        .item_states
        .0
        .iter()
        .map(|(id, e)| (id.as_str(), e.state))
        .collect();
    assert_eq!(
        ids,
        vec![("I-01", ItemState::Pending), ("I-02", ItemState::Pending)]
    );
    // The human body below the tool block is untouched.
    assert!(direction.ends_with(BODY), "body bytes changed");
    assert!(direction.starts_with("---\ngenerated_by: driverctl"));
}

// ---------------------------------------------------------------- request helpers

/// A `log` request as the JSON the wire carries; every value is a string, as the
/// spike client sent it.
fn log_req(event: &str, args: &[(&str, &str)]) -> String {
    let map: serde_json::Map<String, serde_json::Value> = args
        .iter()
        .map(|(k, v)| ((*k).to_owned(), serde_json::json!(v)))
        .collect();
    serde_json::json!({"op": "log", "event": event, "args": map}).to_string()
}

fn log_ok(store: &mut RunStore, event: &str, args: &[(&str, &str)]) {
    let r = send(store, &log_req(event, args));
    expect_ok(&r);
}

/// The three files a rejection must leave byte-identical.
fn snapshot(fx: &Fixture) -> [Vec<u8>; 3] {
    ["LOG.md", "state.json", "DIRECTION.md"]
        .map(|n| fs::read(fx.run_dir.join(n)).unwrap_or_default())
}

// ---------------------------------------------------------------- the rc=2 table

#[test]
fn every_malformed_request_is_rejected_and_nothing_is_written() {
    let fx = Fixture::new();
    let mut store = fx.open();
    store.begin_launch(standalone()).unwrap();
    log_ok(&mut store, "start", &[("mode", "headless")]);

    let long_question = "q".repeat(301);
    let cases: Vec<(&str, String)> = vec![
        ("malformed JSON", r#"{"op":"#.to_owned()),
        ("empty request", String::new()),
        (
            "args not an object",
            r#"{"op":"log","event":"start","args":5}"#.to_owned(),
        ),
        (
            "args an array",
            r#"{"op":"log","event":"start","args":[]}"#.to_owned(),
        ),
        (
            "unknown op",
            r#"{"op":"set_terminal","status":"crashed"}"#.to_owned(),
        ),
        ("unknown event", log_req("explode", &[])),
        ("start --mode bogus", log_req("start", &[("mode", "bogus")])),
        (
            "start --loop_mode with a pipe",
            log_req("start", &[("mode", "headless"), ("loop_mode", "a|b")]),
        ),
        (
            "reconcile --found x",
            log_req("reconcile", &[("found", "x")]),
        ),
        (
            "dispatched --item RUN",
            log_req("dispatched", &[("item", "RUN"), ("step", "go")]),
        ),
        (
            "dispatched unknown item",
            log_req("dispatched", &[("item", "I-99"), ("step", "go")]),
        ),
        (
            "dispatched without --step",
            log_req("dispatched", &[("item", "I-01")]),
        ),
        (
            "dispatched --step with a pipe",
            log_req("dispatched", &[("item", "I-01"), ("step", "a|b")]),
        ),
        (
            "dispatched --round x",
            log_req(
                "dispatched",
                &[("item", "I-01"), ("step", "go"), ("round", "x")],
            ),
        ),
        (
            "dispatched --phase with a newline",
            log_req(
                "dispatched",
                &[("item", "I-01"), ("step", "go"), ("phase", "1\nRUN")],
            ),
        ),
        (
            "report --status maybe",
            log_req(
                "report",
                &[("item", "I-01"), ("status", "maybe"), ("body", "x")],
            ),
        ),
        (
            "report with no body",
            log_req("report", &[("item", "I-01"), ("status", "done")]),
        ),
        (
            "report with an empty body",
            log_req(
                "report",
                &[("item", "I-01"), ("status", "done"), ("body", "")],
            ),
        ),
        (
            "report with only newlines",
            log_req(
                "report",
                &[("item", "I-01"), ("status", "done"), ("body", "\n\n")],
            ),
        ),
        (
            "report with an ESC in a line",
            log_req(
                "report",
                &[
                    ("item", "I-01"),
                    ("status", "done"),
                    ("body", "fine\n\u{1b}[31mred"),
                ],
            ),
        ),
        (
            "verified --done-when maybe",
            log_req(
                "verified",
                &[("item", "I-01"), ("done_when", "maybe"), ("evidence", "x")],
            ),
        ),
        (
            "status --state bogus",
            log_req(
                "status",
                &[("item", "I-01"), ("state", "bogus"), ("outcome", "x")],
            ),
        ),
        (
            "status parked without --rule",
            log_req(
                "status",
                &[("item", "I-01"), ("state", "parked"), ("outcome", "x")],
            ),
        ),
        (
            "status parked --rule bogus",
            log_req(
                "status",
                &[
                    ("item", "I-01"),
                    ("state", "parked"),
                    ("rule", "bogus"),
                    ("outcome", "x"),
                ],
            ),
        ),
        (
            "status running --rule bogus",
            log_req(
                "status",
                &[("item", "I-01"), ("state", "running"), ("rule", "bogus")],
            ),
        ),
        (
            "status done without --outcome",
            log_req("status", &[("item", "I-01"), ("state", "done")]),
        ),
        (
            "status with a multi-line --outcome",
            log_req(
                "status",
                &[("item", "I-01"), ("state", "done"), ("outcome", "a\nb")],
            ),
        ),
        (
            "needs-human question of 301 chars",
            log_req(
                "needs-human",
                &[("item", "I-01"), ("question", &long_question)],
            ),
        ),
        (
            "recovered --action bogus",
            log_req(
                "recovered",
                &[("item", "I-01"), ("action", "bogus"), ("reason", "x")],
            ),
        ),
        (
            "end --reason crashed",
            log_req("end", &[("reason", "crashed")]),
        ),
        (
            "end --reason stalled",
            log_req("end", &[("reason", "stalled")]),
        ),
        (
            "an arg with a JSON number value",
            r#"{"op":"log","event":"dispatched","args":{"item":"I-01","step":"go","round":1}}"#
                .to_owned(),
        ),
        (
            "an unknown arg key",
            r#"{"op":"log","event":"start","args":{"mode":"headless","bogus":"x"}}"#.to_owned(),
        ),
        (
            "an unknown payload key",
            r#"{"op":"hook","payload":{"hook_event_name":"PreToolUse","tool_input":{}}}"#
                .to_owned(),
        ),
        (
            "a hook with a pipe in the session",
            r#"{"op":"hook","payload":{"hook_event_name":"SessionStart","session_id":"a|b"}}"#
                .to_owned(),
        ),
    ];

    for (name, request) in &cases {
        let before = snapshot(&fx);
        let state_before = store.state().clone();
        let r = send(&mut store, request);
        assert!(!r.ok, "case `{name}` must be rejected, got {r:?}");
        assert!(r.error.is_some(), "case `{name}` must carry an error");
        assert_eq!(before, snapshot(&fx), "case `{name}` changed a file");
        assert_eq!(
            &state_before,
            store.state(),
            "case `{name}` changed the in-memory state"
        );
    }
}

#[test]
fn a_direction_with_no_items_rejects_every_item_level_event() {
    let fx = Fixture::with_body("## Direction\n\n- control: run\n\n## Items\n\n");
    let mut store = fx.open();
    store.begin_launch(standalone()).unwrap();
    let before = snapshot(&fx);
    for event in [
        "dispatched",
        "report",
        "verified",
        "status",
        "needs-human",
        "recovered",
    ] {
        let r = send(
            &mut store,
            &log_req(
                event,
                &[
                    ("item", "I-01"),
                    ("step", "go"),
                    ("status", "done"),
                    ("body", "x"),
                    ("done_when", "met"),
                    ("evidence", "x"),
                    ("state", "done"),
                    ("outcome", "x"),
                    ("question", "x"),
                    ("action", "done"),
                    ("reason", "x"),
                ],
            ),
        );
        assert!(!r.ok, "{event} must be rejected without DIRECTION items");
    }
    assert_eq!(before, snapshot(&fx));
}

// ---------------------------------------------------------------- the events

#[test]
fn dispatched_logs_the_spike_key_order_and_updates_state() {
    let fx = Fixture::new();
    let mut store = fx.open();
    store.begin_launch(standalone()).unwrap();
    log_ok(
        &mut store,
        "dispatched",
        &[
            ("item", "I-01"),
            ("step", "/gsd-plan-phase 1"),
            ("round", "0"),
            ("phase", "1"),
        ],
    );
    log_ok(
        &mut store,
        "dispatched",
        &[("item", "I-01"), ("step", "/gsd-execute-phase 1")],
    );
    log_ok(
        &mut store,
        "dispatched",
        &[("item", "I-02"), ("step", "go")],
    );

    let events = log_events(&fx.log());
    assert_eq!(
        events[1],
        "I-01 | dispatched | step=/gsd-plan-phase 1 | round=0 | dispatch=1/3 | phase=1"
    );
    assert_eq!(
        events[2],
        "I-01 | dispatched | step=/gsd-execute-phase 1 | round=0 | dispatch=2/3"
    );
    // I-02 declares no dispatch budget.
    assert_eq!(
        events[3],
        "I-02 | dispatched | step=go | round=0 | dispatch=1/None"
    );

    let state = fx.state_json();
    assert_eq!(state["items"]["I-01"]["state"], "running");
    assert_eq!(state["items"]["I-01"]["dispatches"], 2);
    assert_eq!(state["current"]["item"], "I-02");
    assert_eq!(state["current"]["step"], "go");
    // Driver-logged dispatches never move the hook-observed cap counter.
    assert_eq!(state["caps"]["dispatches"]["used"], 0);
    assert_eq!(state["hook_dispatches"], 0);

    let fm = parse_driver_frontmatter(&fx.read("DIRECTION.md")).unwrap();
    assert_eq!(fm.dispatch_audit.logged, 3);
    assert_eq!(fm.dispatch_audit.observed, 0);
    assert_eq!(fm.items.running, 2);
    assert_eq!(fm.current_item.as_deref(), Some("I-02"));
}

/// Run `git` in `dir` with a throwaway identity and no signing.
fn run_git(dir: &std::path::Path, args: &[&str]) -> String {
    let out = std::process::Command::new("git")
        .arg("-C")
        .arg(dir)
        .args([
            "-c",
            "user.name=t",
            "-c",
            "user.email=t@example.invalid",
            "-c",
            "commit.gpgsign=false",
        ])
        .args(args)
        .output()
        .expect("git runs");
    assert!(
        out.status.success(),
        "git {args:?}: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).trim().to_owned()
}

#[test]
fn dispatched_records_the_base_only_inside_a_repository() {
    let fx = Fixture::new();
    let mut store = fx.open();
    store.begin_launch(standalone()).unwrap();
    log_ok(
        &mut store,
        "dispatched",
        &[("item", "I-01"), ("step", "plain")],
    );
    assert!(
        !fx.log().contains("base="),
        "a non-repository project has no base"
    );

    run_git(&fx.project, &["init", "-q"]);
    run_git(
        &fx.project,
        &["commit", "-q", "--allow-empty", "-m", "base"],
    );
    let head = run_git(&fx.project, &["rev-parse", "--short", "HEAD"]);
    log_ok(
        &mut store,
        "dispatched",
        &[("item", "I-01"), ("step", "in-repo")],
    );
    let events = log_events(&fx.log());
    assert_eq!(
        events.last().unwrap(),
        &format!("I-01 | dispatched | step=in-repo | base={head} | round=0 | dispatch=2/3")
    );
}

#[test]
fn report_keeps_fifteen_lines_and_records_findings() {
    let fx = Fixture::new();
    let mut store = fx.open();
    store.begin_launch(standalone()).unwrap();

    log_ok(
        &mut store,
        "report",
        &[
            ("item", "I-01"),
            ("status", "done"),
            ("body", "FINDINGS: 2\nsecond line\n"),
        ],
    );
    let log = fx.log();
    assert!(
        log.contains("| I-01 | report | status=done\n    FINDINGS: 2\n    second line\n"),
        "{log}"
    );
    assert!(!log.contains("truncated"));

    let long = (1..=20)
        .map(|n| format!("row {n}"))
        .collect::<Vec<_>>()
        .join("\n");
    log_ok(
        &mut store,
        "report",
        &[("item", "I-01"), ("status", "partial"), ("body", &long)],
    );
    let log = fx.log();
    assert!(
        log.contains("| I-01 | report | status=partial | truncated=yes\n"),
        "{log}"
    );
    assert!(log.contains("    row 15\n"));
    assert!(!log.contains("row 16"));
    assert_eq!(
        fx.state_json()["items"]["I-01"]["findings"],
        serde_json::json!([2])
    );
}

#[test]
fn verified_needs_human_recovered_and_reconcile_use_the_spike_keys() {
    let fx = Fixture::new();
    let mut store = fx.open();
    store.begin_launch(standalone()).unwrap();
    log_ok(&mut store, "reconcile", &[]);
    log_ok(&mut store, "reconcile", &[("found", "4")]);
    log_ok(
        &mut store,
        "verified",
        &[
            ("item", "I-01"),
            ("done_when", "met"),
            ("evidence", "tests pass"),
        ],
    );
    log_ok(
        &mut store,
        "needs-human",
        &[("item", "I-02"), ("question", "which one?")],
    );
    log_ok(
        &mut store,
        "recovered",
        &[
            ("item", "I-01"),
            ("action", "resume"),
            ("reason", "worker died"),
        ],
    );
    let events = log_events(&fx.log());
    assert_eq!(
        &events[1..],
        [
            "RUN | reconcile | found=0",
            "RUN | reconcile | found=4",
            "I-01 | verified | done-when=met | evidence=tests pass",
            "I-02 | needs-human | question=which one?",
            "I-01 | recovered | action=resume | reason=worker died",
        ]
    );
}

#[test]
fn status_sets_item_state_and_outcome_in_the_frontmatter() {
    let fx = Fixture::new();
    let mut store = fx.open();
    store.begin_launch(standalone()).unwrap();
    log_ok(
        &mut store,
        "status",
        &[
            ("item", "I-01"),
            ("state", "parked"),
            ("rule", "budget"),
            ("outcome", "out of budget"),
        ],
    );
    log_ok(
        &mut store,
        "status",
        &[("item", "I-02"), ("state", "running")],
    );
    let events = log_events(&fx.log());
    assert_eq!(
        events[1],
        "I-01 | status | state=parked | rule=budget | outcome=out of budget"
    );
    assert_eq!(events[2], "I-02 | status | state=running");

    let fm = parse_driver_frontmatter(&fx.read("DIRECTION.md")).unwrap();
    assert_eq!(fm.items.parked, 1);
    assert_eq!(fm.items.running, 1);
    assert_eq!(fm.items.total, 2);
    let i01 = &fm.item_states.0[0];
    assert_eq!((i01.0.as_str(), i01.1.state), ("I-01", ItemState::Parked));
    assert_eq!(i01.1.outcome.as_deref(), Some("out of budget"));
}

#[test]
fn end_maps_every_reason_to_its_status() {
    for (reason, status) in [
        ("all-done", "done"),
        ("all-parked", "parked"),
        ("pause", "paused"),
        ("stop", "stopped"),
        ("run-budget", "stopped"),
        ("fresh-exit", "running"),
    ] {
        let fx = Fixture::new();
        let mut store = fx.open();
        store.begin_launch(standalone()).unwrap();
        log_ok(&mut store, "end", &[("reason", reason)]);
        let state = fx.state_json();
        assert_eq!(state["status"], status, "end --reason {reason}");
        assert_eq!(state["status_reason"], format!("driver:{reason}"));
        assert_eq!(state["ended_by_driver"], true);
        assert_eq!(
            log_events(&fx.log()).last().unwrap(),
            &format!("RUN | end | reason={reason}")
        );
        let fm = parse_driver_frontmatter(&fx.read("DIRECTION.md")).unwrap();
        assert_eq!(fm.status.as_str(), status);
    }
}

// ---------------------------------------------------------------- launches and forward compatibility

fn with_driver() -> LaunchSpec {
    LaunchSpec {
        launcher_mode: "standalone".to_owned(),
        driver: Some(DriverSpec {
            model: Some("opus".to_owned()),
            mode: "headless".to_owned(),
        }),
        scope_unit: Some("unit.scope".to_owned()),
        ..LaunchSpec::default()
    }
}

#[test]
fn a_relaunch_after_an_unended_launch_logs_crash_detected() {
    let fx = Fixture::new();
    {
        let mut store = fx.open();
        store.begin_launch(with_driver()).unwrap();
        log_ok(
            &mut store,
            "dispatched",
            &[("item", "I-01"), ("step", "go")],
        );
        // The store (and its lock) goes away without a driver `RUN end`.
    }
    let first = log_events(&fx.log());
    assert!(
        first.iter().all(|e| !e.contains("crash-detected")),
        "a first launch is not a crash: {first:?}"
    );

    let mut store = fx.open();
    store.begin_launch(with_driver()).unwrap();
    let events = log_events(&fx.log());
    let tail: Vec<&str> = events.iter().rev().take(2).map(String::as_str).collect();
    assert_eq!(tail[0], "RUN | launch | n=2 | mode=standalone | model=opus");
    assert_eq!(
        tail[1],
        "RUN | crash-detected | previous_launch=1 | evidence=status=running and no RUN end; lock was free"
    );

    let state = store.state();
    assert_eq!(state.launches, 2);
    assert_eq!(state.status, RunStatus::Running);
    let roles: Vec<(&str, &str)> = state
        .connections
        .iter()
        .map(|c| (c.role.as_str(), c.state.as_str()))
        .collect();
    assert_eq!(
        roles,
        [
            ("driver", "lost"),
            ("launcher", "connected"),
            ("driver", "connected")
        ]
    );
}

#[test]
fn a_relaunch_after_a_driver_end_is_not_a_crash() {
    let fx = Fixture::new();
    {
        let mut store = fx.open();
        store.begin_launch(with_driver()).unwrap();
        log_ok(&mut store, "end", &[("reason", "fresh-exit")]);
    }
    let mut store = fx.open();
    store.begin_launch(with_driver()).unwrap();
    assert!(
        log_events(&fx.log())
            .iter()
            .all(|e| !e.contains("crash-detected")),
        "{}",
        fx.log()
    );
    assert!(!store.state().ended_by_driver, "a new launch clears it");
}

/// `value` with every `null` object member removed, recursively.
fn without_nulls(value: &serde_json::Value) -> serde_json::Value {
    use serde_json::Value;
    match value {
        Value::Object(map) => Value::Object(
            map.iter()
                .filter(|(_, v)| !v.is_null())
                .map(|(k, v)| (k.clone(), without_nulls(v)))
                .collect(),
        ),
        Value::Array(items) => Value::Array(items.iter().map(without_nulls).collect()),
        other => other.clone(),
    }
}

#[test]
fn launcher_written_state_keys_survive_a_store_rewrite() {
    let fx = Fixture::new();
    let original = fs::read_to_string(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/driverctl/spike003/r261a/state.json"),
    )
    .unwrap();
    fs::write(fx.run_dir.join("state.json"), &original).unwrap();
    let before: serde_json::Value = serde_json::from_str(&original).unwrap();

    let mut store = fx.open();
    log_ok(&mut store, "reconcile", &[("found", "2")]);
    let after = fx.state_json();

    for key in ["auth", "cost", "active_seconds", "hook_dispatches"] {
        assert_eq!(after[key], before[key], "launcher key `{key}` changed");
    }
    assert_eq!(
        after["items"]["I-01"]["findings"],
        before["items"]["I-01"]["findings"]
    );
    assert_eq!(after["caps"]["gap_rounds"], before["caps"]["gap_rounds"]);
    // The typed `Connection` writes its absent fields as explicit nulls where the
    // spike omitted them; every value that was present is unchanged.
    assert_eq!(
        without_nulls(&after["connections"]),
        without_nulls(&before["connections"])
    );
    assert_eq!(after["run_id"], before["run_id"]);
    assert_eq!(after["last_event"]["event"], "reconcile");
}

#[test]
fn unknown_keys_inside_items_and_caps_survive_too() {
    let fx = Fixture::new();
    fs::write(
        fx.run_dir.join("state.json"),
        r#"{"run_id":"r27","future_top":{"a":[1,2]},"items":{"I-01":{"state":"running","future_item":7}},"caps":{"future_cap":true}}"#,
    )
    .unwrap();
    let mut store = fx.open();
    log_ok(&mut store, "reconcile", &[]);
    let after = fx.state_json();
    assert_eq!(after["future_top"], serde_json::json!({"a": [1, 2]}));
    assert_eq!(after["items"]["I-01"]["future_item"], 7);
    assert_eq!(after["caps"]["future_cap"], true);
}

#[test]
fn a_corrupt_state_json_refuses_to_open_and_is_left_alone() {
    let fx = Fixture::new();
    fs::write(fx.run_dir.join("state.json"), "{ not json").unwrap();
    let lock = acquire_at_with_retry(
        &run_lock_path(&fx.run_dir),
        "r27",
        std::process::id(),
        ACQUIRE_RETRY_BUDGET,
    )
    .unwrap();
    let err = match RunStore::open(&fx.run_dir, &fx.project, lock) {
        Ok(_) => panic!("a corrupt state.json must not open"),
        Err(e) => e,
    };
    assert!(matches!(err, StoreError::CorruptState { .. }), "{err:?}");
    assert_eq!(fx.read("state.json"), "{ not json");
}

#[test]
fn the_run_lock_is_held_until_the_store_drops() {
    let fx = Fixture::new();
    let store = fx.open();
    let second = acquire_at_with_retry(
        &run_lock_path(&fx.run_dir),
        "other",
        std::process::id(),
        std::time::Duration::from_millis(5),
    );
    assert!(second.is_err(), "the store must hold the run lock");
    assert_eq!(store.holder().run_id, "r27");
    drop(store);
    let again = acquire_at_with_retry(
        &run_lock_path(&fx.run_dir),
        "other",
        std::process::id(),
        ACQUIRE_RETRY_BUDGET,
    );
    assert!(again.is_ok(), "dropping the store releases the lock");
}

#[test]
fn show_returns_a_frontmatter_the_reader_parses() {
    let fx = Fixture::new();
    let mut store = fx.open();
    store.begin_launch(standalone()).unwrap();
    let r = send(&mut store, r#"{"op":"show"}"#);
    expect_ok(&r);
    let text = r.frontmatter.expect("frontmatter present");
    let fm = parse_driver_frontmatter(&text).expect("show output parses");
    assert_eq!(fm.run_id, "r27");
    assert_eq!(fm.launches, 1);
}

// ---------------------------------------------------------------- hooks, gate, heartbeat, gaps, terminal

/// 2026-09-21T06:13:20Z; any fixed instant would do.
const T0: i64 = 1_790_000_000;

const GATE_REQ: &str = r#"{"op":"hook","payload":{"hook_event_name":"PreToolUse","tool_name":"Agent","session_id":"s1","prompt":"do the thing"}}"#;
const TASK_REQ: &str = r#"{"op":"hook","payload":{"hook_event_name":"PreToolUse","tool_name":"Task","session_id":"s1"}}"#;
const NESTED_REQ: &str = r#"{"op":"hook","payload":{"hook_event_name":"PreToolUse","tool_name":"Task","agent_id":"sub-1","agent_type":"worker"}}"#;
const HEARTBEAT_REQ: &str =
    r#"{"op":"hook","payload":{"hook_event_name":"PostToolUse","tool_name":"Bash"}}"#;

fn stamp(secs: i64) -> String {
    DateTime::<Utc>::from_timestamp(secs, 0)
        .unwrap()
        .format("%Y-%m-%dT%H:%M:%SZ")
        .to_string()
}

fn open_clocked(fx: &Fixture, clock: &Arc<AtomicI64>) -> RunStore {
    let lock = acquire_at_with_retry(
        &run_lock_path(&fx.run_dir),
        "r27",
        std::process::id(),
        ACQUIRE_RETRY_BUDGET,
    )
    .expect("run lock");
    let c = Arc::clone(clock);
    RunStore::open_with_clock(
        &fx.run_dir,
        &fx.project,
        lock,
        Box::new(move || DateTime::<Utc>::from_timestamp(c.load(Ordering::SeqCst), 0).unwrap()),
    )
    .expect("open store")
}

fn advance(clock: &Arc<AtomicI64>, to: i64) {
    clock.store(to, Ordering::SeqCst);
}

#[test]
fn only_driver_level_agent_dispatches_the_gate_allows_are_counted() {
    let fx = Fixture::new();
    let mut store = fx.open();
    store.begin_launch(with_driver()).unwrap();
    // Three driver-logged dispatches must not move the cap counter (integrity).
    for n in 0..3 {
        log_ok(
            &mut store,
            "dispatched",
            &[("item", "I-02"), ("step", &format!("step {n}"))],
        );
    }
    assert_eq!(fx.state_json()["caps"]["dispatches"]["used"], 0);

    let r = send(&mut store, GATE_REQ);
    assert_eq!(serde_json::to_string(&r).unwrap(), r#"{"ok":true}"#);
    assert_eq!(fx.state_json()["hook_dispatches"], 1);
    assert_eq!(fx.state_json()["caps"]["dispatches"]["used"], 1);

    // A nested (subagent) payload never counts.
    expect_ok(&send(&mut store, NESTED_REQ));
    assert_eq!(fx.state_json()["hook_dispatches"], 1);

    // `Task` counts like `Agent`.
    expect_ok(&send(&mut store, TASK_REQ));
    assert_eq!(fx.state_json()["hook_dispatches"], 2);
    assert_eq!(fx.state_json()["caps"]["dispatches"]["used"], 2);

    let fm = parse_driver_frontmatter(&fx.read("DIRECTION.md")).unwrap();
    assert_eq!(fm.dispatch_audit.logged, 3, "driver-logged dispatches");
    assert_eq!(fm.dispatch_audit.observed, 2, "hook-observed dispatches");
    assert_eq!(fm.dispatch_audit.denied, 0);
    assert_eq!(fm.caps.dispatches.used, 2);
}

#[test]
fn a_denying_gate_counts_the_denial_logs_it_and_replies_deny() {
    let fx = Fixture::new();
    fs::write(
        fx.run_dir.join("state.json"),
        r#"{"run_id":"r27","caps":{"gap_rounds":{"limit":2,"per_phase":{"1":3}}}}"#,
    )
    .unwrap();
    let mut store = fx.open();
    type Seen = Vec<(Option<String>, u32)>;
    let seen: Arc<Mutex<Seen>> = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&seen);
    store.set_gate(Box::new(move |payload, state, _control| {
        let rounds = state
            .caps
            .gap_rounds
            .per_phase
            .get("1")
            .copied()
            .unwrap_or(0);
        sink.lock()
            .unwrap()
            .push((payload.tool_name.clone(), rounds));
        if rounds >= 3 {
            GateDecision::Deny("cap".to_owned())
        } else {
            GateDecision::Allow
        }
    }));

    let r = send(&mut store, GATE_REQ);
    assert_eq!(
        serde_json::to_string(&r).unwrap(),
        r#"{"ok":true,"deny":"cap"}"#
    );
    // The gate was shown the run state, including the gap-round counts.
    assert_eq!(*seen.lock().unwrap(), vec![(Some("Agent".to_owned()), 3)]);

    let state = fx.state_json();
    assert_eq!(state["denied_dispatches"], 1);
    assert_eq!(state["hook_dispatches"], 0);
    assert_eq!(state["caps"]["dispatches"]["used"], 0);
    assert_eq!(
        log_events(&fx.log()).last().unwrap(),
        "RUN | dispatch-denied | reason=cap"
    );
    let fm = parse_driver_frontmatter(&fx.read("DIRECTION.md")).unwrap();
    assert_eq!(fm.dispatch_audit.denied, 1);
    assert_eq!(fm.dispatch_audit.observed, 0);

    // A nested payload never reaches the gate.
    expect_ok(&send(&mut store, NESTED_REQ));
    assert_eq!(seen.lock().unwrap().len(), 1);
}

#[test]
fn session_start_and_end_maintain_the_driver_connection() {
    let fx = Fixture::new();
    let mut store = fx.open();
    store.begin_launch(with_driver()).unwrap();
    expect_ok(&send(
        &mut store,
        r#"{"op":"hook","payload":{"hook_event_name":"SessionStart","session_id":"sess-1","source":"startup"}}"#,
    ));
    let driver = |s: &RunStore| {
        s.state()
            .connections
            .iter()
            .rev()
            .find(|c| c.role == "driver")
            .cloned()
            .unwrap()
    };
    assert_eq!(driver(&store).session_id.as_deref(), Some("sess-1"));
    assert!(driver(&store).last_heartbeat.is_some());
    assert_eq!(
        log_events(&fx.log()).last().unwrap(),
        "RUN | connect | session=sess-1 | source=startup"
    );

    expect_ok(&send(
        &mut store,
        r#"{"op":"hook","payload":{"hook_event_name":"SessionEnd","session_id":"sess-1","reason":"other"}}"#,
    ));
    assert_eq!(driver(&store).state, "disconnected");
    assert_eq!(
        log_events(&fx.log()).last().unwrap(),
        "RUN | disconnect | session=sess-1 | reason=other"
    );
}

#[test]
fn a_heartbeat_refreshes_the_driver_connection() {
    let fx = Fixture::new();
    let clock = Arc::new(AtomicI64::new(T0));
    let mut store = open_clocked(&fx, &clock);
    store.begin_launch(with_driver()).unwrap();
    advance(&clock, T0 + 7);
    expect_ok(&send(&mut store, HEARTBEAT_REQ));
    let hb = store
        .state()
        .connections
        .iter()
        .find(|c| c.role == "driver")
        .and_then(|c| c.last_heartbeat.clone());
    assert_eq!(hb, Some(stamp(T0 + 7)));
}

#[test]
fn heartbeats_rewrite_state_every_time_but_regenerate_direction_every_120_s() {
    let fx = Fixture::new();
    let clock = Arc::new(AtomicI64::new(T0));
    let mut store = open_clocked(&fx, &clock);

    // The first heartbeat has nothing to coalesce into: it regenerates.
    expect_ok(&send(&mut store, HEARTBEAT_REQ));
    let first = fx.read("DIRECTION.md");
    assert!(first.starts_with("---\ngenerated_by: driverctl"), "{first}");

    let mut previous_state = fx.read("state.json");
    for offset in [1, 2, 30, 60, 119] {
        advance(&clock, T0 + offset);
        expect_ok(&send(&mut store, HEARTBEAT_REQ));
        assert_eq!(
            fx.read("DIRECTION.md"),
            first,
            "DIRECTION.md must not be regenerated at +{offset}s"
        );
        let state = fx.read("state.json");
        assert_ne!(
            state, previous_state,
            "state.json must be rewritten at +{offset}s"
        );
        previous_state = state;
    }

    advance(&clock, T0 + 120);
    expect_ok(&send(&mut store, HEARTBEAT_REQ));
    let second = fx.read("DIRECTION.md");
    assert_ne!(
        second, first,
        "a heartbeat 120 s after the last regen regenerates"
    );

    // Any other event regenerates immediately, however recently the last one was.
    advance(&clock, T0 + 125);
    log_ok(
        &mut store,
        "verified",
        &[("item", "I-01"), ("done_when", "met"), ("evidence", "ok")],
    );
    let third = fx.read("DIRECTION.md");
    assert_ne!(third, second);
    assert!(third.contains("event: \"verified\""), "{third}");
}

#[test]
fn a_heartbeat_at_119_s_is_coalesced_and_one_at_exactly_120_s_is_not() {
    for (offset, regenerates) in [(119, false), (120, true)] {
        let fx = Fixture::new();
        let clock = Arc::new(AtomicI64::new(T0));
        let mut store = open_clocked(&fx, &clock);
        expect_ok(&send(&mut store, HEARTBEAT_REQ));
        let first = fx.read("DIRECTION.md");
        advance(&clock, T0 + offset);
        expect_ok(&send(&mut store, HEARTBEAT_REQ));
        assert_eq!(
            fx.read("DIRECTION.md") != first,
            regenerates,
            "heartbeat at +{offset}s"
        );
    }
}

#[test]
fn a_suppressed_heartbeat_is_flushed_once_the_window_has_passed() {
    let fx = Fixture::new();
    let clock = Arc::new(AtomicI64::new(T0));
    let mut store = open_clocked(&fx, &clock);
    assert!(!store.flush_heartbeat(), "nothing pending yet");

    expect_ok(&send(&mut store, HEARTBEAT_REQ));
    let first = fx.read("DIRECTION.md");
    advance(&clock, T0 + 30);
    expect_ok(&send(&mut store, HEARTBEAT_REQ));
    assert_eq!(
        fx.read("DIRECTION.md"),
        first,
        "the +30s heartbeat is coalesced"
    );

    advance(&clock, T0 + 100);
    assert!(
        !store.flush_heartbeat(),
        "the window has not passed at +100s"
    );
    assert_eq!(fx.read("DIRECTION.md"), first);

    advance(&clock, T0 + 150);
    assert!(
        store.flush_heartbeat(),
        "the trailing flush regenerates once"
    );
    let flushed = fx.read("DIRECTION.md");
    assert_ne!(flushed, first);
    assert!(
        flushed.contains(&format!("updated_at: \"{}\"", stamp(T0 + 30))),
        "the flush carries the suppressed heartbeat's state:\n{flushed}"
    );
    assert!(!store.flush_heartbeat(), "a second flush has nothing to do");
}

fn put_verification(project: &std::path::Path, content: &str, mtime_secs: f64) {
    let dir = project.join(".planning/phases/01-slugify");
    fs::create_dir_all(&dir).unwrap();
    let path = dir.join("01-VERIFICATION.md");
    fs::write(&path, content).unwrap();
    fs::OpenOptions::new()
        .write(true)
        .open(&path)
        .unwrap()
        .set_modified(UNIX_EPOCH + Duration::from_secs_f64(mtime_secs))
        .unwrap();
}

#[test]
fn scan_tick_observes_gap_rounds_from_disk_and_coalesces_rewrites() {
    let fx = Fixture::new();
    let mut store = fx.open();
    store
        .begin_launch(LaunchSpec {
            launcher_mode: "standalone".to_owned(),
            limits: CapLimits {
                gap_rounds: Some(2),
                ..CapLimits::default()
            },
            ..LaunchSpec::default()
        })
        .unwrap();
    let file = ".planning/phases/01-slugify/01-VERIFICATION.md";
    let base = 1_000_000.0;

    put_verification(&fx.project, "---\nstatus: gaps_found\n---\nbody v1\n", base);
    assert_eq!(store.scan_tick().unwrap(), 1);
    assert_eq!(
        log_events(&fx.log()).last().unwrap(),
        &format!("RUN | gap-observed | phase=1 | count=1 | file={file} | source=driverctl-scan")
    );
    let fm = parse_driver_frontmatter(&fx.read("DIRECTION.md")).unwrap();
    assert_eq!(fm.caps.gap_rounds.used, 1);
    assert_eq!(fm.caps.gap_rounds.limit, Some(2));
    assert_eq!(fm.caps.gap_rounds.phase.as_deref(), Some("1"));

    // The verifier patches the file 6.5 s later: still one round.
    put_verification(
        &fx.project,
        "---\nstatus: gaps_found\n---\nbody v2\n",
        base + 6.5,
    );
    assert_eq!(store.scan_tick().unwrap(), 1);
    assert_eq!(
        log_events(&fx.log()).last().unwrap(),
        &format!("RUN | gap-coalesced | phase=1 | count=1 | file={file}")
    );
    assert_eq!(fx.state_json()["caps"]["gap_rounds"]["per_phase"]["1"], 1);

    // The same bytes again: nothing is written at all.
    put_verification(
        &fx.project,
        "---\nstatus: gaps_found\n---\nbody v2\n",
        base + 30.0,
    );
    let before = snapshot(&fx);
    assert_eq!(store.scan_tick().unwrap(), 0);
    assert_eq!(
        before,
        snapshot(&fx),
        "a duplicate write must write nothing"
    );

    // A genuine second round, far from the anchor.
    put_verification(
        &fx.project,
        "---\nstatus: gaps_found\n---\nbody v3\n",
        base + 600.0,
    );
    assert_eq!(store.scan_tick().unwrap(), 1);
    assert_eq!(
        log_events(&fx.log()).last().unwrap(),
        &format!("RUN | gap-observed | phase=1 | count=2 | file={file} | source=driverctl-scan")
    );
    let fm = parse_driver_frontmatter(&fx.read("DIRECTION.md")).unwrap();
    assert_eq!(fm.caps.gap_rounds.used, 2);
}

#[test]
fn cr01_scan_tick_survives_a_fifo_planted_as_a_verification() {
    let fx = Fixture::new();
    let mut store = fx.open();
    store.begin_launch(standalone()).unwrap();
    let dir = fx.project.join(".planning/phases/02-planted");
    fs::create_dir_all(&dir).unwrap();
    rustix::fs::mknodat(
        rustix::fs::CWD,
        dir.join("02-VERIFICATION.md"),
        rustix::fs::FileType::Fifo,
        rustix::fs::Mode::from_raw_mode(0o600),
        0,
    )
    .unwrap();
    put_verification(
        &fx.project,
        "---\nstatus: gaps_found\n---\nbody\n",
        1_000_000.0,
    );

    // A blocked read would hang here forever; run it on a thread so the failure
    // is a timeout, not a wedged suite.
    let (tx, rx) = std::sync::mpsc::channel();
    std::thread::spawn(move || {
        let _ = tx.send(store.scan_tick());
    });
    let changed = rx
        .recv_timeout(Duration::from_secs(30))
        .expect("scan_tick blocked on a planted FIFO")
        .unwrap();
    assert_eq!(changed, 1, "only the regular file is observed");
}

#[test]
fn set_terminal_records_the_launcher_verdict() {
    let fx = Fixture::new();
    let mut store = fx.open();
    store.begin_launch(with_driver()).unwrap();
    let reason =
        "driver ended its turn with a worker outstanding (dispatch I-01 has no report in LOG.md)";
    let outcome = store.set_terminal(TerminalStatus::Stalled, reason).unwrap();
    assert_eq!(outcome, TerminalOutcome::Applied);

    let state = fx.state_json();
    assert_eq!(state["status"], "stalled");
    assert_eq!(state["status_reason"], reason);
    assert!(store
        .state()
        .connections
        .iter()
        .all(|c| c.state == "disconnected"));
    assert_eq!(
        log_events(&fx.log()).last().unwrap(),
        &format!("RUN | launch-end | status=stalled | reason={reason}")
    );
    let fm = parse_driver_frontmatter(&fx.read("DIRECTION.md")).unwrap();
    assert_eq!(fm.status, RunStatus::Stalled);

    // A reason that is not header-safe is cleaned, never able to forge an event.
    let other = Fixture::new();
    let mut store = other.open();
    store.begin_launch(standalone()).unwrap();
    store
        .set_terminal(
            TerminalStatus::Crashed,
            "rc=1\n- 2026-01-01T00:00:00Z | RUN | end",
        )
        .unwrap();
    let log = other.log();
    assert_eq!(
        log.lines().filter(|l| l.starts_with("- ")).count(),
        2,
        "launch + launch-end only:\n{log}"
    );
    assert_eq!(other.state_json()["status"], "crashed");
}

#[test]
fn set_terminal_keeps_a_driver_run_end() {
    let fx = Fixture::new();
    let mut store = fx.open();
    store.begin_launch(with_driver()).unwrap();
    log_ok(&mut store, "end", &[("reason", "all-done")]);
    let before = snapshot(&fx);
    let outcome = store
        .set_terminal(
            TerminalStatus::Crashed,
            "driver exited rc=1 without RUN end",
        )
        .unwrap();
    assert_eq!(outcome, TerminalOutcome::KeptDriverEnd);
    assert_eq!(before, snapshot(&fx), "nothing may change");
    assert_eq!(fx.state_json()["status"], "done");
}

#[test]
fn end_launch_records_the_launch_end_without_changing_the_status() {
    let fx = Fixture::new();
    let mut store = fx.open();
    store.begin_launch(with_driver()).unwrap();
    log_ok(&mut store, "end", &[("reason", "stop")]);
    assert_eq!(fx.state_json()["status"], "stopped");

    store.end_launch(Some(0)).unwrap();

    let state = fx.state_json();
    assert_eq!(state["status"], "stopped", "the driver's status is kept");
    assert_eq!(state["status_reason"], "driver:stop");
    assert_eq!(state["ended_by_driver"], true);
    assert!(store
        .state()
        .connections
        .iter()
        .all(|c| c.state == "disconnected"));
    assert_eq!(
        log_events(&fx.log()).last().unwrap(),
        "RUN | launch-end | rc=0 | status=stopped"
    );
}

#[test]
fn end_launch_after_a_fresh_exit_lets_the_next_launch_follow_without_a_crash() {
    let fx = Fixture::new();
    let mut store = fx.open();
    store.begin_launch(with_driver()).unwrap();
    log_ok(&mut store, "end", &[("reason", "fresh-exit")]);
    store.end_launch(Some(0)).unwrap();
    assert_eq!(fx.state_json()["status"], "running");

    store.begin_launch(with_driver()).unwrap();

    let events = log_events(&fx.log());
    assert!(
        !events.iter().any(|e| e.contains("crash-detected")),
        "a fresh-exit end is not a crash: {events:?}"
    );
    assert!(events.last().unwrap().starts_with("RUN | launch | n=2"));
    assert_eq!(fx.state_json()["launches"], 2);
}

#[test]
fn end_launch_without_a_code_omits_the_rc_and_never_forges_a_header() {
    let fx = Fixture::new();
    let mut store = fx.open();
    store.begin_launch(standalone()).unwrap();
    store.end_launch(None).unwrap();
    assert_eq!(
        log_events(&fx.log()).last().unwrap(),
        "RUN | launch-end | status=running"
    );
}

#[test]
fn wr02_set_terminal_after_a_fresh_exit_records_the_stop() {
    let fx = Fixture::new();
    let mut store = fx.open();
    store.begin_launch(with_driver()).unwrap();
    log_ok(&mut store, "end", &[("reason", "fresh-exit")]);
    assert_eq!(fx.state_json()["status"], "running");
    let outcome = store
        .set_terminal(TerminalStatus::Stopped, "serve:signal")
        .unwrap();
    assert_eq!(outcome, TerminalOutcome::Applied);
    assert_eq!(
        fx.state_json()["status"],
        "stopped",
        "a clean stop after fresh-exit must not stay `running` (reader derives crashed)"
    );
}

#[test]
fn wr03_a_failed_state_save_appends_nothing_to_the_log() {
    let fx = Fixture::new();
    let mut store = fx.open();
    store.begin_launch(with_driver()).unwrap();
    log_ok(&mut store, "start", &[("mode", "headless")]);

    // A directory where state.json belongs: the rename-into-place cannot succeed.
    let state = fx.run_dir.join("state.json");
    let state_before = fs::read(&state).unwrap();
    fs::remove_file(&state).unwrap();
    fs::create_dir(&state).unwrap();
    let log_before = fx.log();

    let r = send(
        &mut store,
        &log_req(
            "needs-human",
            &[("item", "I-01"), ("question", "which one?")],
        ),
    );
    assert!(!r.ok, "the save fails, so the request fails: {r:?}");
    assert_eq!(
        fx.log(),
        log_before,
        "no LOG line may exist for a state change that was not saved"
    );

    // Heal the path: the retry succeeds and logs exactly one line.
    fs::remove_dir(&state).unwrap();
    fs::write(&state, &state_before).unwrap();
    let r = send(
        &mut store,
        &log_req(
            "needs-human",
            &[("item", "I-01"), ("question", "which one?")],
        ),
    );
    assert!(r.ok, "{r:?}");
    assert_eq!(fx.log().lines().count(), log_before.lines().count() + 1);
}

#[test]
fn wr03_a_failed_log_append_restores_state_json() {
    let fx = Fixture::new();
    let mut store = fx.open();
    store.begin_launch(with_driver()).unwrap();
    log_ok(&mut store, "start", &[("mode", "headless")]);

    let state_before = fs::read(fx.run_dir.join("state.json")).unwrap();
    // A directory where LOG.md belongs: the append cannot succeed.
    fs::remove_file(fx.run_dir.join("LOG.md")).unwrap();
    fs::create_dir(fx.run_dir.join("LOG.md")).unwrap();

    let r = send(
        &mut store,
        &log_req(
            "needs-human",
            &[("item", "I-01"), ("question", "which one?")],
        ),
    );
    assert!(!r.ok, "{r:?}");
    assert_eq!(
        fs::read(fx.run_dir.join("state.json")).unwrap(),
        state_before,
        "state.json must not advance past a LOG line that was never written"
    );
}

#[test]
fn no_request_can_set_crashed_or_stalled_or_rewrite_a_counter() {
    let fx = Fixture::new();
    let mut store = fx.open();
    store.begin_launch(with_driver()).unwrap();
    let attempts: Vec<String> = vec![
        r#"{"op":"set_terminal","status":"crashed","reason":"x"}"#.to_owned(),
        r#"{"op":"terminal","status":"stalled"}"#.to_owned(),
        r#"{"op":"state","status":"crashed"}"#.to_owned(),
        log_req("end", &[("reason", "stalled")]),
        log_req("end", &[("reason", "crashed")]),
        log_req(
            "status",
            &[("item", "I-01"), ("state", "crashed"), ("outcome", "x")],
        ),
        log_req(
            "status",
            &[("item", "I-01"), ("state", "stalled"), ("outcome", "x")],
        ),
        log_req("start", &[("mode", "crashed")]),
        log_req(
            "recovered",
            &[("item", "I-01"), ("action", "stalled"), ("reason", "x")],
        ),
        r#"{"op":"log","event":"start","args":{"mode":"headless","status":"crashed"}}"#.to_owned(),
        r#"{"op":"log","event":"start","args":{"mode":"headless","hook_dispatches":"99"}}"#
            .to_owned(),
        r#"{"op":"log","event":"start","args":{"mode":"headless","caps":"x"}}"#.to_owned(),
        r#"{"op":"hook","payload":{"hook_event_name":"crashed","reason":"stalled"}}"#.to_owned(),
    ];
    for request in &attempts {
        // Some are refused and a few (a hook with a made-up event name) are plain
        // heartbeats; none may change the status or a counter.
        let _ = send(&mut store, request);
        let state = store.state();
        assert_eq!(state.status, RunStatus::Running, "after {request}");
        assert!(!state.ended_by_driver, "after {request}");
        assert_eq!(state.hook_dispatches, 0, "after {request}");
        assert_eq!(state.denied_dispatches, 0, "after {request}");
        assert_eq!(state.caps.dispatches.used, 0, "after {request}");
    }
}

#[test]
fn an_insane_body_is_logged_once_not_once_per_event() {
    let fx = Fixture::new();
    let mut store = fx.open();
    store.begin_launch(standalone()).unwrap();
    fs::write(fx.run_dir.join("DIRECTION.md"), "# half saved\n").unwrap();
    for found in ["1", "2", "3"] {
        log_ok(&mut store, "reconcile", &[("found", found)]);
    }
    let count = |fx: &Fixture| {
        log_events(&fx.log())
            .iter()
            .filter(|e| *e == "RUN | skipped-insane-body")
            .count()
    };
    assert_eq!(count(&fx), 1, "{}", fx.log());
    // The tool-health line did not become the run's last event.
    assert_eq!(fx.state_json()["last_event"]["event"], "reconcile");

    // The body is saved properly, then breaks again: a new transition, a new line.
    fs::write(fx.run_dir.join("DIRECTION.md"), BODY).unwrap();
    log_ok(&mut store, "reconcile", &[("found", "4")]);
    assert_eq!(count(&fx), 1);
    fs::write(fx.run_dir.join("DIRECTION.md"), "# half saved\n").unwrap();
    log_ok(&mut store, "reconcile", &[("found", "5")]);
    assert_eq!(count(&fx), 2);
}

#[test]
fn a_held_direction_lock_is_logged_and_the_splice_proceeds() {
    let fx = Fixture::new();
    let mut store = fx.open();
    let _held = direction_lock(&fx.run_dir).expect("take DIRECTION.lock");
    log_ok(&mut store, "reconcile", &[]);
    assert!(
        log_events(&fx.log()).contains(&"RUN | direction-lock-timeout".to_owned()),
        "{}",
        fx.log()
    );
    // The lock only serialises gsd-mm's own writers; the exchange-verify protects
    // the human's bytes, so the frontmatter was still written.
    assert!(fx
        .read("DIRECTION.md")
        .starts_with("---\ngenerated_by: driverctl"));
}

// ---------------------------------------------------------------- the launch posture (28-04, D-30)

#[test]
fn begin_launch_records_the_posture_in_state_and_frontmatter() {
    let fx = Fixture::new();
    let mut store = fx.open();
    store
        .begin_launch(LaunchSpec {
            sandbox: Some("degraded".to_owned()),
            sandbox_missing: vec!["bwrap".to_owned(), "user-systemd".to_owned()],
            ..standalone()
        })
        .unwrap();

    let state = fx.state_json();
    assert_eq!(state["sandbox"], "degraded");
    assert_eq!(
        state["sandbox_missing"],
        serde_json::json!(["bwrap", "user-systemd"])
    );
    let fm = parse_driver_frontmatter(&fx.read("DIRECTION.md")).unwrap();
    assert_eq!(fm.sandbox.as_deref(), Some("degraded"));
    assert_eq!(
        fm.sandbox_missing,
        Some(vec!["bwrap".to_owned(), "user-systemd".to_owned()])
    );
}

#[test]
fn a_launch_without_a_posture_writes_neither_key_and_clears_a_stale_one() {
    let fx = Fixture::new();
    {
        let mut store = fx.open();
        store
            .begin_launch(LaunchSpec {
                sandbox: Some("degraded".to_owned()),
                sandbox_missing: vec!["bwrap".to_owned()],
                ..standalone()
            })
            .unwrap();
        store.end_launch(Some(0)).unwrap();
    }
    // A later launch that is not the sandbox launcher's (`driverctl serve`) must not
    // inherit the previous posture.
    let mut store = fx.open();
    store.begin_launch(standalone()).unwrap();
    let state = fx.state_json();
    assert!(state.get("sandbox").is_none(), "{state}");
    assert!(state.get("sandbox_missing").is_none(), "{state}");
    let text = fx.read("DIRECTION.md");
    assert!(!text.contains("sandbox"), "{text}");
    let fm = parse_driver_frontmatter(&text).unwrap();
    assert_eq!((fm.sandbox, fm.sandbox_missing), (None, None));
}

#[test]
fn a_sandboxed_launch_has_no_missing_list() {
    let fx = Fixture::new();
    let mut store = fx.open();
    store
        .begin_launch(LaunchSpec {
            sandbox: Some("bwrap-scope".to_owned()),
            ..standalone()
        })
        .unwrap();
    let state = fx.state_json();
    assert_eq!(state["sandbox"], "bwrap-scope");
    assert!(state.get("sandbox_missing").is_none(), "{state}");
    let text = fx.read("DIRECTION.md");
    assert!(text.contains("\nsandbox: \"bwrap-scope\"\n"), "{text}");
    assert!(!text.contains("sandbox_missing"), "{text}");
}

// ---------------------------------------------------------------- control (D-18, D-19)

/// A `control: <word>` edit of the human's DIRECTION.md, bytes below the tool
/// block otherwise untouched.
fn set_control(fx: &Fixture, word: &str) {
    let path = fx.run_dir.join("DIRECTION.md");
    let text = fs::read_to_string(&path).unwrap();
    assert!(text.contains("- control: run"), "the fixture body says run");
    fs::write(
        &path,
        text.replacen("- control: run", &format!("- control: {word}"), 1),
    )
    .unwrap();
}

#[test]
fn the_cap_gate_denies_a_driver_dispatch_while_control_is_pause() {
    for word in ["pause", "stop"] {
        let fx = Fixture::new();
        let mut store = fx.open();
        store.set_gate(CapGate::new(CapLimits::default(), new_tripped()).into_gate());
        store.begin_launch(standalone()).unwrap();
        set_control(&fx, word);

        let r = send(&mut store, GATE_REQ);
        let deny = r.deny.expect("a paused run denies the dispatch");
        assert!(deny.starts_with(&format!("control={word};")), "{deny}");

        let state = fx.state_json();
        assert_eq!(state["denied_dispatches"], 1);
        assert_eq!(state["hook_dispatches"], 0);
        let log = fx.log();
        let denied: Vec<&str> = log
            .lines()
            .filter(|l| l.contains("RUN | dispatch-denied"))
            .collect();
        assert_eq!(denied.len(), 1, "{log}");
        assert!(denied[0].contains(&format!("control={word}")), "{log}");
    }
}

#[test]
fn control_run_lets_the_cap_gate_allow_the_same_dispatch() {
    let fx = Fixture::new();
    let mut store = fx.open();
    store.set_gate(CapGate::new(CapLimits::default(), new_tripped()).into_gate());
    store.begin_launch(standalone()).unwrap();
    let r = send(&mut store, GATE_REQ);
    assert!(r.deny.is_none(), "{r:?}");
    assert_eq!(fx.state_json()["hook_dispatches"], 1);
}

#[test]
fn dispatched_is_rejected_while_control_is_pause_or_stop_but_the_rest_is_accepted() {
    for word in ["pause", "stop"] {
        let fx = Fixture::new();
        let mut store = fx.open();
        store.begin_launch(standalone()).unwrap();
        log_ok(&mut store, "start", &[("mode", "headless")]);
        log_ok(
            &mut store,
            "dispatched",
            &[("item", "I-01"), ("step", "go"), ("round", "0")],
        );
        set_control(&fx, word);

        let before = snapshot(&fx);
        let r = send(
            &mut store,
            &log_req(
                "dispatched",
                &[("item", "I-02"), ("step", "go"), ("round", "0")],
            ),
        );
        assert!(!r.ok, "{r:?}");
        assert_eq!(
            r.error.as_deref(),
            Some(
                format!(
                    "control={word}: dispatch nothing new; record the in-flight report, then \
                     end --reason {word}"
                )
                .as_str()
            )
        );
        assert_eq!(snapshot(&fx), before, "a rejection writes nothing");

        log_ok(
            &mut store,
            "report",
            &[("item", "I-01"), ("status", "done"), ("body", "ok")],
        );
        log_ok(
            &mut store,
            "verified",
            &[
                ("item", "I-01"),
                ("done_when", "met"),
                ("evidence", "tests pass"),
            ],
        );
        log_ok(&mut store, "end", &[("reason", "pause")]);
    }
}

/// Replace the `- control: run` line of the fixture's DIRECTION.md with `line`.
fn set_control_line(fx: &Fixture, line: &str) {
    let path = fx.run_dir.join("DIRECTION.md");
    let text = fs::read_to_string(&path).unwrap();
    assert!(text.contains("- control: run"), "the fixture body says run");
    fs::write(&path, text.replacen("- control: run", line, 1)).unwrap();
}

#[test]
fn a_pause_or_stop_is_honoured_in_any_case_and_with_stray_whitespace() {
    // WR-02: the lint only checks at launch, so a mid-run edit of `Pause` must still hold.
    for (line, word) in [
        ("- control: Pause", "pause"),
        ("- control: PAUSE", "pause"),
        ("- control:   Stop   ", "stop"),
        ("-   control:\tSTOP", "stop"),
        ("  - Control: pAuSe", "pause"),
    ] {
        let fx = Fixture::new();
        let mut store = fx.open();
        store.set_gate(CapGate::new(CapLimits::default(), new_tripped()).into_gate());
        store.begin_launch(standalone()).unwrap();
        log_ok(&mut store, "start", &[("mode", "headless")]);
        set_control_line(&fx, line);

        let r = send(&mut store, GATE_REQ);
        let deny = r
            .deny
            .unwrap_or_else(|| panic!("{line:?}: the gate must deny"));
        assert!(
            deny.starts_with(&format!("control={word};")),
            "{line:?}: {deny}"
        );

        let r = send(
            &mut store,
            &log_req(
                "dispatched",
                &[("item", "I-02"), ("step", "go"), ("round", "0")],
            ),
        );
        assert!(!r.ok, "{line:?}: dispatched must be refused: {r:?}");
        assert!(
            r.error
                .as_deref()
                .unwrap_or("")
                .starts_with(&format!("control={word}:")),
            "{line:?}: {r:?}"
        );
    }
}

#[test]
fn a_pause_on_any_duplicate_control_line_holds_the_run() {
    let fx = Fixture::new();
    let mut store = fx.open();
    store.set_gate(CapGate::new(CapLimits::default(), new_tripped()).into_gate());
    store.begin_launch(standalone()).unwrap();
    set_control_line(&fx, "- control: run\n- control: pause");
    let r = send(&mut store, GATE_REQ);
    assert!(
        r.deny
            .as_deref()
            .unwrap_or("")
            .starts_with("control=pause;"),
        "{r:?}"
    );
}

#[test]
fn an_unreadable_direction_fails_closed_for_the_gate_and_for_dispatched() {
    // WR-02: a read error other than NotFound hides the human's pause, so it denies.
    let fx = Fixture::new();
    let mut store = fx.open();
    store.set_gate(CapGate::new(CapLimits::default(), new_tripped()).into_gate());
    store.begin_launch(standalone()).unwrap();
    log_ok(&mut store, "start", &[("mode", "headless")]);
    let path = fx.run_dir.join("DIRECTION.md");
    fs::remove_file(&path).unwrap();
    fs::create_dir(&path).unwrap(); // reading a directory is EISDIR, not NotFound

    let r = send(&mut store, GATE_REQ);
    let deny = r
        .deny
        .expect("an unreadable DIRECTION.md denies the dispatch");
    assert!(deny.contains("DIRECTION.md is unreadable"), "{deny}");
    assert!(deny.contains("dispatch nothing new"), "{deny}");

    let r = send(
        &mut store,
        &log_req(
            "dispatched",
            &[("item", "I-02"), ("step", "go"), ("round", "0")],
        ),
    );
    assert!(!r.ok, "{r:?}");
    assert!(
        r.error
            .as_deref()
            .unwrap_or("")
            .contains("DIRECTION.md is unreadable"),
        "{r:?}"
    );
    assert_eq!(fx.state_json()["hook_dispatches"], 0);
}

#[test]
fn an_absent_direction_is_the_no_direction_case_and_the_gate_allows() {
    // NotFound is the documented "no direction" case (a bare `driverctl serve`).
    let fx = Fixture::new();
    let mut store = fx.open();
    store.set_gate(CapGate::new(CapLimits::default(), new_tripped()).into_gate());
    store.begin_launch(standalone()).unwrap();
    fs::remove_file(fx.run_dir.join("DIRECTION.md")).unwrap();
    let r = send(&mut store, GATE_REQ);
    assert!(r.deny.is_none(), "{r:?}");
}

#[test]
fn dispatched_is_accepted_when_control_is_run() {
    let fx = Fixture::new();
    let mut store = fx.open();
    store.begin_launch(standalone()).unwrap();
    log_ok(&mut store, "start", &[("mode", "headless")]);
    log_ok(
        &mut store,
        "dispatched",
        &[("item", "I-02"), ("step", "go"), ("round", "0")],
    );
}

#[test]
fn the_dispatch_gate_itself_refuses_until_each_running_item_is_recovered() {
    // WR-06: D-17 is a refusal of the Agent dispatch, not only of the `dispatched`
    // record, so a driver that skips the log still cannot start un-reconciled work.
    let fx = Fixture::new();
    {
        let mut store = fx.open();
        store.begin_launch(standalone()).unwrap();
        log_ok(&mut store, "start", &[("mode", "headless")]);
        log_ok(
            &mut store,
            "dispatched",
            &[("item", "I-01"), ("step", "go"), ("round", "0")],
        );
    }
    let mut store = fx.open();
    store.set_gate(CapGate::new(CapLimits::default(), new_tripped()).into_gate());
    store.begin_launch(standalone()).unwrap();

    let r = send(&mut store, GATE_REQ);
    let deny = r
        .deny
        .expect("an un-reconciled relaunch denies the dispatch");
    assert!(
        deny.starts_with("reconcile first: I-01 was running at launch start"),
        "{deny}"
    );
    let state = fx.state_json();
    assert_eq!(
        state["hook_dispatches"], 0,
        "a denied dispatch is not counted"
    );
    assert_eq!(state["denied_dispatches"], 1);
    let log = fx.log();
    assert!(
        log.lines()
            .any(|l| l.contains("RUN | dispatch-denied") && l.contains("reconcile first: I-01")),
        "{log}"
    );

    log_ok(
        &mut store,
        "recovered",
        &[
            ("item", "I-01"),
            ("action", "resume"),
            ("reason", "worker died"),
        ],
    );
    let r = send(&mut store, GATE_REQ);
    assert!(
        r.deny.is_none(),
        "after the recovery the dispatch passes: {r:?}"
    );
    assert_eq!(fx.state_json()["hook_dispatches"], 1);
}

#[test]
fn a_control_hold_is_named_before_the_reconcile_rule_at_the_dispatch_gate() {
    let fx = Fixture::new();
    {
        let mut store = fx.open();
        store.begin_launch(standalone()).unwrap();
        log_ok(&mut store, "start", &[("mode", "headless")]);
        log_ok(
            &mut store,
            "dispatched",
            &[("item", "I-01"), ("step", "go"), ("round", "0")],
        );
    }
    set_control(&fx, "pause");
    let mut store = fx.open();
    store.set_gate(CapGate::new(CapLimits::default(), new_tripped()).into_gate());
    store.begin_launch(standalone()).unwrap();
    let deny = send(&mut store, GATE_REQ).deny.expect("denied");
    assert!(deny.starts_with("control=pause;"), "{deny}");
}

#[test]
fn a_paused_relaunch_names_the_control_word_before_the_reconcile_rule() {
    let fx = Fixture::new();
    {
        let mut store = fx.open();
        store.begin_launch(standalone()).unwrap();
        log_ok(&mut store, "start", &[("mode", "headless")]);
        log_ok(
            &mut store,
            "dispatched",
            &[("item", "I-01"), ("step", "go"), ("round", "0")],
        );
    }
    set_control(&fx, "pause");
    let mut store = fx.open();
    store.begin_launch(standalone()).unwrap();
    let r = send(
        &mut store,
        &log_req(
            "dispatched",
            &[("item", "I-02"), ("step", "go"), ("round", "0")],
        ),
    );
    assert!(
        r.error
            .as_deref()
            .unwrap_or("")
            .starts_with("control=pause:"),
        "{r:?}"
    );
    // The running item can still be reconciled while paused.
    log_ok(
        &mut store,
        "recovered",
        &[("item", "I-01"), ("action", "park"), ("reason", "paused")],
    );
}

// ---------------------------------------------------------------- the prompt version (29-02, D-03)

fn with_prompt(version: Option<&str>) -> LaunchSpec {
    LaunchSpec {
        prompt_version: version.map(str::to_owned),
        ..standalone()
    }
}

fn launch_lines(log: &str) -> Vec<String> {
    log_events(log)
        .into_iter()
        .filter(|l| l.starts_with("RUN | launch |"))
        .collect()
}

#[test]
fn the_prompt_version_is_recorded_per_launch_and_never_inherited() {
    let fx = Fixture::new();
    {
        let mut store = fx.open();
        store
            .begin_launch(with_prompt(Some("driver-prompt/1.0.0")))
            .unwrap();
    }
    assert_eq!(fx.state_json()["prompt_version"], "driver-prompt/1.0.0");
    let fm = parse_driver_frontmatter(&fx.read("DIRECTION.md")).unwrap();
    assert_eq!(fm.prompt_version.as_deref(), Some("driver-prompt/1.0.0"));

    {
        let mut store = fx.open();
        store
            .begin_launch(with_prompt(Some("custom:0a1b2c3d")))
            .unwrap();
    }
    assert_eq!(fx.state_json()["prompt_version"], "custom:0a1b2c3d");
    let fm = parse_driver_frontmatter(&fx.read("DIRECTION.md")).unwrap();
    assert_eq!(fm.prompt_version.as_deref(), Some("custom:0a1b2c3d"));

    {
        let mut store = fx.open();
        store.begin_launch(with_prompt(None)).unwrap();
    }
    assert!(
        fx.state_json().get("prompt_version").is_none(),
        "a launch without a version removes the key"
    );
    let direction = fx.read("DIRECTION.md");
    assert!(!direction.contains("prompt_version"), "{direction}");

    let lines = launch_lines(&fx.log());
    assert_eq!(
        lines,
        vec![
            "RUN | launch | n=1 | mode=standalone | prompt=driver-prompt/1.0.0".to_owned(),
            "RUN | launch | n=2 | mode=standalone | prompt=custom:0a1b2c3d".to_owned(),
            "RUN | launch | n=3 | mode=standalone".to_owned(),
        ],
        "LOG.md keeps every launch's own value"
    );
}

#[test]
fn a_hostile_prompt_version_stays_one_log_field() {
    let fx = Fixture::new();
    let mut store = fx.open();
    store
        .begin_launch(with_prompt(Some("a|b\nRUN | end | reason=x")))
        .unwrap();
    let lines = launch_lines(&fx.log());
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert_eq!(
        lines[0],
        "RUN | launch | n=1 | mode=standalone | prompt=a b RUN   end   reason=x"
    );
    assert_eq!(
        fx.log().lines().filter(|l| l.starts_with("- ")).count(),
        1,
        "still one event line"
    );
}

#[test]
fn frontmatter_for_derives_the_never_launched_block_and_matches_the_store() {
    let body = "# Driver direction: pure

## Direction

- control: run
- goal: Derive the block.

## Items

### I-01 Only slice

- intent: Phase 1 is planned, executed and verified.
";
    let direction = parse_direction(body);
    let state = RunState::new("r1", "/p", "2026-10-19T00:00:00Z");
    let fm = frontmatter_for(&state, &direction);
    assert_eq!(fm.run_id, "r1");
    assert_eq!(fm.status, RunStatus::Starting);
    assert_eq!(fm.launches, 0);
    assert_eq!(fm.items.total, 1);
    assert_eq!(fm.items.pending, 1);
    assert_eq!(fm.control.as_deref(), Some("run"));
    assert_eq!(fm.prompt_version, None);
    assert_eq!(fm.updated_at, "2026-10-19T00:00:00Z");

    // The store delegates to it: same state, same DIRECTION, same block.
    let fx = Fixture::with_body(body);
    let store = fx.open();
    assert_eq!(
        store.frontmatter(),
        frontmatter_for(store.state(), &parse_direction(body))
    );

    // A launch carrying a version shows it in the derived block.
    let mut launched = state.clone();
    launched.extra.insert(
        "prompt_version".to_owned(),
        serde_json::json!("driver-prompt/1.0.0"),
    );
    assert_eq!(
        frontmatter_for(&launched, &direction)
            .prompt_version
            .as_deref(),
        Some("driver-prompt/1.0.0")
    );
}
