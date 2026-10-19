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

use gsd_meta_manager::driver::lock::{acquire_at_with_retry, run_lock_path, ACQUIRE_RETRY_BUDGET};
use gsd_meta_manager::driverctl::protocol::Response;
use gsd_meta_manager::driverctl::store::{DriverSpec, LaunchSpec, RunStore, StoreError};
use gsd_meta_manager::state_reader::driver_frontmatter::{
    parse_driver_frontmatter, ItemState, RunStatus,
};
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
        .map(|l| l.splitn(2, " | ").nth(1).unwrap_or("").to_owned())
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
    run_git(&fx.project, &["commit", "-q", "--allow-empty", "-m", "base"]);
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
    assert_eq!(after["items"]["I-01"]["findings"], before["items"]["I-01"]["findings"]);
    assert_eq!(after["caps"]["gap_rounds"], before["caps"]["gap_rounds"]);
    assert_eq!(after["connections"], before["connections"]);
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
