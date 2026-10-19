//! Golden round-trip tests for the driver frontmatter schema v1 (D-16).
//!
//! D-16 asks for fixtures covering "template, starting, running, stopped, crashed,
//! stalled, multi-connection". The concrete value behind each name is an
//! `[inferred — audit]` interpretation, pinned here:
//!
//! - template: `starting`, `launches: 0`, no connections, a null `last_event`, and
//!   two pending items (the not-yet-launched block Phase 29's template ships)
//! - starting: `starting`, `launches: 1`, one launcher connection
//! - running: `running` with a current item, phase and step, one item running with
//!   two dispatches, limits set, one gap round used in phase "1"
//! - stopped: `stopped`, reason `driver:stop`
//! - crashed: `crashed`, reason `driver exited rc=1 without RUN end`
//! - stalled: `stalled`, the exit-classifier's reason text naming I-01 (no spike
//!   ever recorded one; this is the first)
//! - multi-connection: `running` with a lost previous driver, a connected launcher
//!   and a connected driver, in that order
//!
//! Every golden is a complete DIRECTION.md emitted by the Rust emitter. Each test
//! asserts that `emit()` equals the golden's leading block byte for byte and that
//! parsing the golden returns the value it was built from. Fixed `generated_by` and
//! timestamps keep the goldens independent of the crate version and the clock.

use gsd_meta_manager::state_reader::driver_frontmatter::{
    parse_driver_frontmatter, Cap, Caps, Connection, DispatchAudit, DriverFrontmatter,
    FrontmatterError, GapCap, ItemCounts, ItemState, ItemStateEntry, ItemStates, LastEvent,
    RunStatus, WallClockCap, DRIVER_FRONTMATTER_VERSION, HEARTBEAT_STALE_AFTER_S,
};
use std::path::PathBuf;

fn fixture(rel: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/driverctl")
        .join(rel)
}

/// The human-owned body every golden carries below its block.
const BODY: &str = "# Driver direction: golden

## Direction

- control: run
- goal: Ship phase 1 of the golden project, unattended.

## Items

### I-01 Phase 1 first slice

- intent: Phase 1 is planned, executed and verified.
- hold: false

### I-02 Phase 2 second slice

- intent: Phase 2 is planned, executed and verified.
- after: I-01
- hold: false
";

/// The golden text up to and including its second `---` line and that line's
/// newline. Test-local on purpose: the production splitter is built in the same
/// wave by another plan.
fn leading_block(text: &str) -> &str {
    let mut seen = 0;
    let mut end = 0;
    for line in text.split_inclusive('\n') {
        end += line.len();
        if line == "---\n" {
            seen += 1;
            if seen == 2 {
                return &text[..end];
            }
        }
    }
    panic!("golden has no closing --- line");
}

fn base(status: RunStatus) -> DriverFrontmatter {
    DriverFrontmatter {
        generated_by: "driverctl 0.0.0-golden".to_string(),
        driver_frontmatter_version: DRIVER_FRONTMATTER_VERSION,
        run_id: "golden".to_string(),
        status,
        status_reason: None,
        control: Some("run".to_string()),
        current_item: None,
        current_phase: None,
        current_step: None,
        items: ItemCounts {
            total: 2,
            pending: 2,
            ..ItemCounts::default()
        },
        item_states: ItemStates(vec![pending("I-01"), pending("I-02")]),
        caps: Caps {
            dispatches: Cap {
                used: 0,
                limit: Some(10),
            },
            wall_clock_min: WallClockCap {
                used: 0.0,
                limit: Some(60.0),
            },
            gap_rounds: GapCap {
                used: 0,
                limit: Some(2),
                phase: None,
            },
        },
        dispatch_audit: DispatchAudit::default(),
        launches: 0,
        connected: Vec::new(),
        heartbeat_stale_after_s: HEARTBEAT_STALE_AFTER_S,
        last_event: LastEvent::default(),
        updated_at: "2026-01-01T00:00:00Z".to_string(),
    }
}

fn pending(id: &str) -> (String, ItemStateEntry) {
    entry(id, ItemState::Pending, 0, None)
}

fn entry(
    id: &str,
    state: ItemState,
    dispatches: u32,
    outcome: Option<&str>,
) -> (String, ItemStateEntry) {
    (
        id.to_string(),
        ItemStateEntry {
            state,
            dispatches,
            outcome: outcome.map(str::to_string),
        },
    )
}

fn conn(role: &str, state: &str, pid: u32, session: Option<&str>) -> Connection {
    Connection {
        role: role.to_string(),
        state: state.to_string(),
        session_id: session.map(str::to_string),
        model: session.map(|_| "opus".to_string()),
        mode: Some(
            if role == "launcher" {
                "bwrap"
            } else {
                "headless"
            }
            .to_string(),
        ),
        pid: Some(pid),
        scope_unit: Some("gsdmm-golden-1.scope".to_string()),
        host: Some("goldenhost".to_string()),
        started_at: Some("2026-01-01T00:00:00Z".to_string()),
        last_heartbeat: Some("2026-01-01T00:10:00Z".to_string()),
    }
}

fn event(at: &str, item: &str, ev: &str, summary: &str) -> LastEvent {
    LastEvent {
        at: Some(at.to_string()),
        item: Some(item.to_string()),
        event: Some(ev.to_string()),
        summary: Some(summary.to_string()),
    }
}

fn template() -> DriverFrontmatter {
    base(RunStatus::Starting)
}

fn starting() -> DriverFrontmatter {
    let mut fm = base(RunStatus::Starting);
    fm.launches = 1;
    fm.connected = vec![conn("launcher", "connected", 41234, None)];
    fm.last_event = event("2026-01-01T00:00:01Z", "RUN", "launch-start", "launch=1");
    fm
}

fn running() -> DriverFrontmatter {
    let mut fm = base(RunStatus::Running);
    fm.current_item = Some("I-01".to_string());
    fm.current_phase = Some("1".to_string());
    fm.current_step = Some("/gsd-execute-phase 1".to_string());
    fm.items = ItemCounts {
        total: 2,
        pending: 1,
        running: 1,
        ..ItemCounts::default()
    };
    fm.item_states = ItemStates(vec![
        entry("I-01", ItemState::Running, 2, None),
        pending("I-02"),
    ]);
    fm.caps.dispatches.used = 2;
    fm.caps.wall_clock_min.used = 14.2;
    fm.caps.gap_rounds = GapCap {
        used: 1,
        limit: Some(2),
        phase: Some("1".to_string()),
    };
    fm.dispatch_audit = DispatchAudit {
        logged: 2,
        observed: 2,
        denied: 0,
    };
    fm.launches = 1;
    fm.connected = vec![
        conn("launcher", "connected", 41234, None),
        conn(
            "driver",
            "connected",
            41240,
            Some("7c0e1d2a-0000-4000-8000-000000000001"),
        ),
    ];
    fm.last_event = event(
        "2026-01-01T00:13:58Z",
        "I-01",
        "dispatched",
        "step=/gsd-execute-phase 1 base=3b5167b round=0 dispatch=2/10 phase=1",
    );
    fm.updated_at = "2026-01-01T00:14:12Z".to_string();
    fm
}

fn ended(status: RunStatus, reason: &str, summary: &str) -> DriverFrontmatter {
    let mut fm = running();
    fm.status = status;
    fm.status_reason = Some(reason.to_string());
    fm.connected = vec![
        conn("launcher", "disconnected", 41234, None),
        conn(
            "driver",
            "disconnected",
            41240,
            Some("7c0e1d2a-0000-4000-8000-000000000001"),
        ),
    ];
    fm.last_event = event("2026-01-01T00:20:00Z", "RUN", "launch-end", summary);
    fm.updated_at = "2026-01-01T00:20:00Z".to_string();
    fm
}

fn stopped() -> DriverFrontmatter {
    ended(RunStatus::Stopped, "driver:stop", "rc=-15 status=stopped")
}

fn crashed() -> DriverFrontmatter {
    ended(
        RunStatus::Crashed,
        "driver exited rc=1 without RUN end",
        "rc=1 status=crashed",
    )
}

fn stalled() -> DriverFrontmatter {
    ended(
        RunStatus::Stalled,
        "driver ended its turn with a worker outstanding (dispatch I-01 has no report in LOG.md); \
         the process exited cleanly, so this is not a crash",
        "rc=0 status=stalled",
    )
}

fn multi_connection() -> DriverFrontmatter {
    let mut fm = running();
    fm.launches = 2;
    fm.connected = vec![
        conn(
            "driver",
            "lost",
            41240,
            Some("7c0e1d2a-0000-4000-8000-000000000001"),
        ),
        conn("launcher", "connected", 41300, None),
        conn(
            "driver",
            "connected",
            41306,
            Some("7c0e1d2a-0000-4000-8000-000000000002"),
        ),
    ];
    fm
}

/// Assert `emit()` of `fm` is the golden's leading block and that the golden parses
/// back to `fm`.
fn assert_golden(name: &str, fm: &DriverFrontmatter) {
    let path = fixture(&format!("golden/{name}.md"));
    let golden = std::fs::read_to_string(&path)
        .unwrap_or_else(|e| panic!("golden {} unreadable: {e}", path.display()));
    assert_eq!(
        leading_block(&golden),
        fm.emit(),
        "emit() differs from the leading block of {}",
        path.display()
    );
    assert_eq!(
        &parse_driver_frontmatter(&golden).expect("golden parses"),
        fm,
        "golden {name} does not parse back to the value it was emitted from"
    );
}

#[test]
fn golden_template_round_trips() {
    assert_golden("template", &template());
}

#[test]
fn golden_starting_round_trips() {
    assert_golden("starting", &starting());
}

#[test]
fn golden_running_round_trips() {
    assert_golden("running", &running());
}

#[test]
fn golden_stopped_round_trips() {
    assert_golden("stopped", &stopped());
}

#[test]
fn golden_crashed_round_trips() {
    assert_golden("crashed", &crashed());
}

#[test]
fn golden_stalled_round_trips() {
    assert_golden("stalled", &stalled());
}

#[test]
fn golden_multi_connection_round_trips() {
    let fm = multi_connection();
    assert_golden("multi-connection", &fm);
    let roles: Vec<(&str, &str)> = fm
        .connected
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
fn stalled_is_a_terminal_status() {
    assert!(RunStatus::Stalled.is_terminal());
    assert_eq!(RunStatus::Stalled.as_str(), "stalled");
    assert_eq!(RunStatus::from_word("stalled"), RunStatus::Stalled);
    assert!(!RunStatus::Running.is_terminal());
    assert!(!RunStatus::Starting.is_terminal());
    // Emitted bare, like the other known words.
    assert!(stalled().emit().contains("\nstatus: stalled\n"));
}

#[test]
fn item_states_keep_direction_order() {
    // Deliberately not sorted: DIRECTION order is I-10 before I-02.
    let mut fm = running();
    fm.item_states = ItemStates(vec![
        entry("I-10", ItemState::Done, 3, Some("passed")),
        entry("I-02", ItemState::Pending, 0, None),
        entry("I-05", ItemState::Failed, 1, Some("verify failed")),
    ]);
    let back = parse_driver_frontmatter(&fm.emit()).expect("round trip");
    let ids: Vec<&str> = back
        .item_states
        .0
        .iter()
        .map(|(id, _)| id.as_str())
        .collect();
    assert_eq!(ids, ["I-10", "I-02", "I-05"]);
    assert_eq!(back, fm);
}

// ---------------------------------------------------------------- spike fixtures

/// Every spike-generated DIRECTION.md, as (file name, text), sorted by name.
fn spike_directions() -> Vec<(String, String)> {
    let dir = fixture("direction");
    let mut out: Vec<(String, String)> = std::fs::read_dir(&dir)
        .expect("direction fixture dir")
        .map(|e| e.expect("dir entry").path())
        .filter(|p| p.extension().is_some_and(|x| x == "md"))
        .map(|p| {
            let name = p.file_name().unwrap().to_string_lossy().into_owned();
            let text = std::fs::read_to_string(&p).expect("fixture readable");
            (name, text)
        })
        .collect();
    out.sort();
    out
}

/// D-16's "spike-002 Python outputs cross-checked as additional reader fixtures"
/// and D-19's `serde(default)` guarantee: none of these has `item_states`.
#[test]
fn every_spike_direction_parses() {
    let files = spike_directions();
    assert_eq!(files.len(), 18, "a spike copy is missing: {files:?}");

    let mut tally: std::collections::BTreeMap<String, u32> = Default::default();
    for (name, text) in &files {
        let fm =
            parse_driver_frontmatter(text).unwrap_or_else(|e| panic!("{name} does not parse: {e}"));
        assert_eq!(fm.generated_by, "driverctl 0.2-spike", "{name}");
        assert!(fm.item_states.0.is_empty(), "{name} has item_states");
        if name == "s002-template.md" {
            assert_eq!(fm.status, RunStatus::Running);
            assert_eq!(fm.connected.len(), 2, "template carries two connections");
        } else {
            *tally.entry(fm.status.as_str().to_string()).or_default() += 1;
        }
    }
    let tally: Vec<(&str, u32)> = tally.iter().map(|(k, v)| (k.as_str(), *v)).collect();
    assert_eq!(
        tally,
        [("crashed", 1), ("done", 1), ("parked", 11), ("stopped", 4)],
        "status tally over the 17 run files"
    );
}

#[test]
fn version_marker_and_block_errors() {
    let (_, text) = spike_directions()
        .into_iter()
        .find(|(n, _)| n == "s003-run1b.md")
        .expect("run1b fixture");

    let bumped = text.replace(
        "driver_frontmatter_version: 1",
        "driver_frontmatter_version: 2",
    );
    assert_eq!(
        parse_driver_frontmatter(&bumped),
        Err(FrontmatterError::UnsupportedVersion { found: 2 })
    );

    let unmarked = text.replace("generated_by: driverctl", "generated_by: somethingelse");
    assert_eq!(
        parse_driver_frontmatter(&unmarked),
        Err(FrontmatterError::NotDriverctl)
    );

    let body_only = text.split_once("\n---\n").expect("block end").1.to_string();
    assert_eq!(
        parse_driver_frontmatter(&body_only),
        Err(FrontmatterError::NoFrontmatter)
    );
    assert_eq!(
        parse_driver_frontmatter(""),
        Err(FrontmatterError::NoFrontmatter)
    );
}

/// Writes the golden files from `emit()`. Run once with `DRIVERCTL_GOLDEN_WRITE=1`.
#[test]
#[ignore = "regenerates the goldens; run explicitly"]
fn write_goldens() {
    let all: [(&str, DriverFrontmatter); 7] = [
        ("template", template()),
        ("starting", starting()),
        ("running", running()),
        ("stopped", stopped()),
        ("crashed", crashed()),
        ("stalled", stalled()),
        ("multi-connection", multi_connection()),
    ];
    for (name, fm) in all {
        std::fs::write(
            fixture(&format!("golden/{name}.md")),
            format!("{}{BODY}", fm.emit()),
        )
        .expect("write golden");
    }
}
