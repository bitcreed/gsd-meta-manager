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
use gsd_meta_manager::driverctl::store::{LaunchSpec, RunStore};
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
