#![cfg(target_os = "linux")]
// ============================================================================
// The relaunch reconcile rule, through the REAL binary (Phase 29, plan 29-02;
// PRMT-04, D-17).
//
// A launch that begins while an item is still `running` (the earlier driver
// died mid-dispatch) must not dispatch anything until each such item has its one
// recorded `recovered` decision. The rule is the STORE's, so these tests drive
// `driverctl serve` + `driverctl log` as separate processes and kill the first
// server with SIGKILL to leave an item running.
//
// Every spawn runs under `XDG_DATA_HOME=/proc/nonexistent-gsdmm` and the clients
// without `CLAUDE_CODE_SESSION_ID`, like `tests/driverctl_serve.rs`.
// ============================================================================

use std::fs;
use std::path::PathBuf;
use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};

use tempfile::TempDir;

const UNWRITABLE_DATA_HOME: &str = "/proc/nonexistent-gsdmm";
const RUN_ID: &str = "r29";

const BODY: &str = "# Driver direction: reconcile test

## Direction

- control: run
- goal: Exercise the relaunch reconcile rule.

## Items

### I-01 First slice

- intent: Phase 1 is planned, executed and verified.

### I-02 Second slice

- intent: Phase 2 is planned, executed and verified.
";

const BODY_ONLY_I02: &str = "# Driver direction: reconcile test

## Direction

- control: run
- goal: Exercise the relaunch reconcile rule.

## Items

### I-02 Second slice

- intent: Phase 2 is planned, executed and verified.
";

struct Fixture {
    project_dir: TempDir,
    scratch: TempDir,
    run_dir: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let project_dir = TempDir::new().expect("project dir");
        let scratch = TempDir::new().expect("scratch dir");
        let run_dir = project_dir
            .path()
            .join(".planning/meta-manager/drivers")
            .join(RUN_ID);
        fs::create_dir_all(&run_dir).expect("run dir");
        fs::write(run_dir.join("DIRECTION.md"), BODY).expect("DIRECTION.md");
        Fixture {
            project_dir,
            scratch,
            run_dir,
        }
    }

    fn sock(&self) -> PathBuf {
        self.scratch.path().join("r.sock")
    }

    fn log(&self) -> String {
        fs::read_to_string(self.run_dir.join("LOG.md")).unwrap_or_default()
    }
}

/// A running `driverctl serve`, killed on drop.
struct Served {
    child: Child,
    stderr: PathBuf,
}

impl Served {
    fn stderr_text(&self) -> String {
        fs::read_to_string(&self.stderr).unwrap_or_default()
    }

    /// SIGKILL: the run is left `running` with no `RUN end`, like a crash.
    fn kill9(&mut self) {
        let pid = rustix::process::Pid::from_child(&self.child);
        rustix::process::kill_process(pid, rustix::process::Signal::KILL).expect("SIGKILL");
        let _ = self.child.wait();
    }
}

impl Drop for Served {
    fn drop(&mut self) {
        if matches!(self.child.try_wait(), Ok(None)) {
            let _ = self.child.kill();
            let _ = self.child.wait();
        }
    }
}

fn spawn_serve(fx: &Fixture, tag: &str) -> Served {
    let sock = fx.sock();
    let stdout = fx.scratch.path().join(format!("{tag}.out"));
    let stderr = fx.scratch.path().join(format!("{tag}.err"));
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_gsd-meta-manager"));
    cmd.args(["driverctl", "serve", "--run-dir"])
        .arg(&fx.run_dir)
        .arg("--project")
        .arg(fx.project_dir.path())
        .arg("--sock")
        .arg(&sock)
        .env("XDG_DATA_HOME", UNWRITABLE_DATA_HOME)
        .env_remove("XDG_RUNTIME_DIR")
        .env_remove("DRIVERCTL_SOCK")
        .env_remove("CLAUDE_CODE_SESSION_ID")
        .stdin(Stdio::null())
        .stdout(fs::File::create(&stdout).expect("stdout file"))
        .stderr(fs::File::create(&stderr).expect("stderr file"));
    let child = cmd.spawn().expect("the binary is built and spawnable");
    let mut served = Served { child, stderr };
    let started = Instant::now();
    loop {
        if served.stderr_text().contains("listening on") {
            return served;
        }
        if let Some(status) = served.child.try_wait().expect("try_wait") {
            panic!(
                "serve exited early ({status})\nstderr: {}",
                served.stderr_text()
            );
        }
        assert!(
            started.elapsed() < Duration::from_secs(10),
            "serve never reported listening\nstderr: {}",
            served.stderr_text()
        );
        std::thread::sleep(Duration::from_millis(20));
    }
}

fn client(fx: &Fixture, args: &[&str]) -> Output {
    Command::new(env!("CARGO_BIN_EXE_gsd-meta-manager"))
        .arg("driverctl")
        .args(args)
        .env("DRIVERCTL_SOCK", fx.sock())
        .env("XDG_DATA_HOME", UNWRITABLE_DATA_HOME)
        .env_remove("CLAUDE_CODE_SESSION_ID")
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("spawn the client")
}

fn stderr(out: &Output) -> String {
    String::from_utf8_lossy(&out.stderr).into_owned()
}

fn rc(out: &Output) -> Option<i32> {
    out.status.code()
}

fn start(fx: &Fixture) {
    let out = client(fx, &["log", "start", "--mode", "headless"]);
    assert_eq!(rc(&out), Some(0), "log start: {}", stderr(&out));
}

fn dispatch(fx: &Fixture, item: &str) -> Output {
    client(
        fx,
        &[
            "log",
            "dispatched",
            "--item",
            item,
            "--step",
            "/gsd-execute-phase 1",
            "--round",
            "0",
        ],
    )
}

fn recover(fx: &Fixture, item: &str) -> Output {
    client(
        fx,
        &[
            "log",
            "recovered",
            "--item",
            item,
            "--action",
            "resume",
            "--reason",
            "worker died mid-dispatch",
        ],
    )
}

fn assert_ok(out: &Output, what: &str) {
    assert_eq!(rc(out), Some(0), "{what}: {}", stderr(out));
}

fn assert_rejected(out: &Output, needle: &str, what: &str) {
    assert_eq!(rc(out), Some(2), "{what}: {}", stderr(out));
    assert!(
        stderr(out).contains(needle),
        "{what}: stderr lacks {needle:?}: {}",
        stderr(out)
    );
}

/// Launch 1 dispatches `items` and is SIGKILLed, leaving them running.
fn crash_with_running(fx: &Fixture, items: &[&str]) {
    let mut first = spawn_serve(fx, "first");
    start(fx);
    for item in items {
        assert_ok(&dispatch(fx, item), "launch 1 dispatch");
    }
    first.kill9();
}

#[test]
fn a_relaunch_refuses_dispatch_until_each_running_item_is_recovered_once() {
    let fx = Fixture::new();
    crash_with_running(&fx, &["I-01"]);

    let _second = spawn_serve(&fx, "second");
    start(&fx);
    assert_rejected(
        &dispatch(&fx, "I-02"),
        "reconcile first: I-01 was running at launch start",
        "dispatch before recovery",
    );
    assert_ok(&recover(&fx, "I-01"), "first recovered");
    assert_rejected(
        &recover(&fx, "I-01"),
        "already recovered in this launch",
        "second recovered",
    );
    assert_ok(&dispatch(&fx, "I-01"), "dispatch after recovery");

    let log = fx.log();
    assert_eq!(
        log.matches("| I-01 | recovered |").count(),
        1,
        "exactly one recovered line: {log}"
    );
    let launch2 = log
        .find("RUN | launch | n=2")
        .expect("launch 2 is in the LOG");
    let recovered = log.find("| I-01 | recovered |").expect("recovered line");
    assert!(
        recovered > launch2,
        "the recovered line follows serve #2's launch line: {log}"
    );
}

#[test]
fn the_rejection_names_the_exact_recovery_command() {
    let fx = Fixture::new();
    crash_with_running(&fx, &["I-01"]);
    let _second = spawn_serve(&fx, "second");
    start(&fx);
    assert_rejected(
        &dispatch(&fx, "I-02"),
        "driverctl: REJECTED: reconcile first: I-01 was running at launch start; record \
         driverctl log recovered --item I-01 --action done|resume|reset|park",
        "dispatch before recovery",
    );
}

#[test]
fn a_launch_with_no_running_item_puts_no_constraint_on_dispatch() {
    let fx = Fixture::new();
    let _only = spawn_serve(&fx, "only");
    start(&fx);
    assert_ok(&dispatch(&fx, "I-01"), "dispatch on a fresh run");
    assert_ok(&dispatch(&fx, "I-02"), "second dispatch on a fresh run");
}

#[test]
fn a_relaunch_after_a_finished_item_dispatches_at_once() {
    let fx = Fixture::new();
    let mut first = spawn_serve(&fx, "first");
    start(&fx);
    assert_ok(&dispatch(&fx, "I-01"), "launch 1 dispatch");
    let done = client(
        &fx,
        &[
            "log",
            "status",
            "--item",
            "I-01",
            "--state",
            "done",
            "--outcome",
            "merged",
        ],
    );
    assert_ok(&done, "status done");
    first.kill9();

    let _second = spawn_serve(&fx, "second");
    start(&fx);
    assert_ok(&dispatch(&fx, "I-02"), "nothing is running, nothing blocks");
}

#[test]
fn recoveries_are_accepted_in_any_order_and_the_lowest_awaiting_id_is_named() {
    let fx = Fixture::new();
    crash_with_running(&fx, &["I-01", "I-02"]);

    let _second = spawn_serve(&fx, "second");
    start(&fx);
    assert_rejected(
        &dispatch(&fx, "I-02"),
        "reconcile first: I-01 was running",
        "both awaiting",
    );
    assert_ok(&recover(&fx, "I-02"), "recovered in reverse order");
    assert_rejected(
        &dispatch(&fx, "I-02"),
        "reconcile first: I-01 was running",
        "I-01 still awaiting",
    );
    assert_ok(&recover(&fx, "I-01"), "recovered I-01");
    assert_ok(&dispatch(&fx, "I-02"), "all recovered");
}

#[test]
fn an_awaiting_item_removed_from_direction_does_not_wedge_the_run() {
    let fx = Fixture::new();
    crash_with_running(&fx, &["I-01"]);
    // The human drops I-01 from DIRECTION.md before the relaunch.
    fs::write(fx.run_dir.join("DIRECTION.md"), BODY_ONLY_I02).expect("rewrite DIRECTION.md");

    let _second = spawn_serve(&fx, "second");
    start(&fx);
    assert_ok(
        &dispatch(&fx, "I-02"),
        "an item that is no longer listed cannot block",
    );
}

#[test]
fn each_relaunch_rebuilds_the_awaiting_and_recovered_sets() {
    let fx = Fixture::new();
    crash_with_running(&fx, &["I-01"]);

    // Launch 2 recovers I-01 and re-dispatches it, then crashes too.
    let mut second = spawn_serve(&fx, "second");
    start(&fx);
    assert_ok(&recover(&fx, "I-01"), "launch 2 recovered");
    assert_ok(&dispatch(&fx, "I-01"), "launch 2 dispatch");
    second.kill9();

    // Launch 3: I-01 is running again, so it awaits again and one `recovered`
    // is accepted even though launch 2 already recorded one.
    let _third = spawn_serve(&fx, "third");
    start(&fx);
    assert_rejected(
        &dispatch(&fx, "I-02"),
        "reconcile first: I-01 was running at launch start",
        "launch 3 dispatch before recovery",
    );
    assert_ok(&recover(&fx, "I-01"), "launch 3 recovered");
    assert_ok(&dispatch(&fx, "I-01"), "launch 3 dispatch");
    assert_eq!(
        fx.log().matches("| I-01 | recovered |").count(),
        2,
        "one recovered line per launch that needed one"
    );
}
