// ============================================================================
// Process lifecycle: the two deadlines (D-13) and the group teardown (D-14)
//
// Everything here needs a real child process, which is why it is not in-source
// with the parsing tests. Three shell stand-ins supply the behaviours a
// transcript cannot express: going silent, spawning a grandchild, and ignoring
// the terminate signal.
//
// Unix-only by construction. `process-wrap`'s `ProcessGroup` and its `signal()`
// are both `#[cfg(unix)]`, and process-group teardown is the entire reason the
// dependency is here — there is no portable behaviour left to test.
// ============================================================================

#![cfg(unix)]

use std::ffi::OsString;
use std::time::{Duration, Instant};

use gsd_meta_manager::executor::claude::ClaudeExecutor;
use gsd_meta_manager::executor::{
    DrivableProject, ExecutionEvent, ExecutionHandle, ExecutionOptions, Executor, RunOutcome,
};
use tempfile::TempDir;

const FAKE_SLOW: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/fake-claude-slow.sh"
);
const FAKE_SPAWNER: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/fake-claude-spawner.sh"
);
const FAKE_DEAF: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/fake-claude-deaf.sh"
);

/// The real grace between the terminate signal and the uncatchable one. The
/// constant itself is private to the executor; this is the observable value the
/// ignored tests wait out.
const TEARDOWN_GRACE: Duration = Duration::from_secs(10);

/// An executor pointed at the paced stand-in.
fn slow(heartbeats: u32, interval: &str, ending: &str) -> ClaudeExecutor {
    ClaudeExecutor::with_program(
        FAKE_SLOW,
        vec![
            OsString::from(heartbeats.to_string()),
            OsString::from(interval),
            OsString::from(ending),
        ],
    )
}

/// Options with test-sized caps.
///
/// The production defaults encode an ordering constraint — the idle cap must
/// stay strictly greater than the background-subagent wait ceiling, or the idle
/// cap kills healthy runs before that ceiling ever fires. Scaling the caps down
/// for a test scales the ceiling down with them, so the tests exercise the same
/// shape the defaults describe rather than an inverted one.
fn capped(wall_ms: u64, idle_ms: u64) -> ExecutionOptions {
    ExecutionOptions {
        wall_clock_cap: Duration::from_millis(wall_ms),
        idle_cap: Duration::from_millis(idle_ms),
        bg_wait_ceiling_ms: idle_ms / 2,
        ..Default::default()
    }
}

/// Whether `pid` still exists, via the zero signal.
///
/// The shell builtin rather than `/bin/kill` so this works wherever `sh` does,
/// and `pid` is an integer so the interpolation cannot carry anything else.
fn alive(pid: u32) -> bool {
    std::process::Command::new("sh")
        .arg("-c")
        .arg(format!("kill -0 {pid} 2>/dev/null"))
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

/// Poll until `pid` is gone, giving up after `limit`.
///
/// A signal is delivered asynchronously and a just-signalled process is briefly
/// still a pid, so the assertion has to be "gone soon" rather than "gone now".
async fn gone_within(pid: u32, limit: Duration) -> bool {
    let deadline = Instant::now() + limit;
    while Instant::now() < deadline {
        if !alive(pid) {
            return true;
        }
        tokio::time::sleep(Duration::from_millis(25)).await;
    }
    !alive(pid)
}

/// Every event still queued once the run has finished.
async fn drain(handle: &mut ExecutionHandle) -> Vec<ExecutionEvent> {
    let mut events = Vec::new();
    while let Some(event) = handle.events.recv().await {
        events.push(event);
    }
    events
}

/// Whether the run reported a process exit status — which is only possible once
/// the group wait has run to completion, and is therefore the observable proof
/// that nothing was left unreaped.
fn reported_an_exit(events: &[ExecutionEvent]) -> bool {
    events
        .iter()
        .any(|event| matches!(event, ExecutionEvent::Exited(_)))
}

/// Read the grandchild's pid off the stream.
///
/// The spawner announces it on a line whose message `type` no CLI version
/// emits, so the tolerant parser carries it as a forward-compatible unknown with
/// its raw text intact — a *known* type would be parsed into a model with
/// nowhere to put a pid.
async fn grandchild_pid(handle: &mut ExecutionHandle) -> u32 {
    while let Some(event) = handle.events.recv().await {
        let ExecutionEvent::Unknown { raw } = event else {
            continue;
        };
        let Some(rest) = raw.split("\"pid\":").nth(1) else {
            continue;
        };
        let digits: String = rest.chars().take_while(char::is_ascii_digit).collect();
        if let Ok(pid) = digits.parse::<u32>() {
            return pid;
        }
    }
    panic!("the spawner stand-in must announce its grandchild's pid on the stream");
}

// ============================================================================
// Teardown reaches the whole process tree (D-14, T-15-30)
// ============================================================================

#[tokio::test]
async fn a_grandchild_spawned_by_the_child_is_gone_after_teardown() {
    let scratch = TempDir::new().expect("temp dir");
    let project = DrivableProject::for_testing("spawner", scratch.path());
    let executor = ClaudeExecutor::with_program(FAKE_SPAWNER, Vec::new());

    let mut handle = executor
        .start(
            &project,
            "/gsd-progress".to_string(),
            capped(30_000, 20_000),
        )
        .await
        .expect("the spawner stand-in advertises every required capability");

    let pid = grandchild_pid(&mut handle).await;
    assert!(
        alive(pid),
        "precondition: the grandchild {pid} must be running before the teardown, or the \
         test proves nothing"
    );

    let outcome = executor.cancel(&mut handle).await;
    assert!(
        matches!(outcome, RunOutcome::Killed { .. }),
        "a torn-down run is a kill, got: {outcome:?}"
    );

    assert!(
        gone_within(pid, Duration::from_secs(5)).await,
        "the grandchild {pid} outlived the teardown — a signal that reaches only the direct \
         child orphans the tree, leaving a background agent holding files, ports and quota \
         the user cannot see (D-14, T-15-30)"
    );

    let events = drain(&mut handle).await;
    assert!(
        reported_an_exit(&events),
        "the group wait must run to completion, or the tree is torn down but not reaped"
    );
}

#[tokio::test]
async fn a_torn_down_run_is_reaped_and_reports_an_exit_status() {
    let scratch = TempDir::new().expect("temp dir");
    let project = DrivableProject::for_testing("reaped", scratch.path());
    let executor = slow(0, "0", "silent");

    let mut handle = executor
        .start(
            &project,
            "/gsd-progress".to_string(),
            capped(30_000, 20_000),
        )
        .await
        .expect("start");

    let child = handle.pgid;
    let outcome = executor.cancel(&mut handle).await;
    assert!(
        matches!(outcome, RunOutcome::Killed { .. }),
        "expected a kill, got: {outcome:?}"
    );

    let events = drain(&mut handle).await;
    assert!(
        reported_an_exit(&events),
        "after teardown the wait has completed and the child reports an exit status; \
         without that there is a zombie (T-15-34). Observed events: {events:?}"
    );
    assert!(
        gone_within(child, Duration::from_secs(5)).await,
        "the child {child} outlived its own teardown"
    );
    assert!(
        !executor.is_running(&handle),
        "a torn-down run must not still report itself as running"
    );
}

#[tokio::test]
#[ignore = "waits out the real ten-second teardown grace"]
async fn a_child_that_ignores_the_terminate_signal_is_still_killed_and_reaped() {
    let scratch = TempDir::new().expect("temp dir");
    let project = DrivableProject::for_testing("deaf", scratch.path());
    let executor = ClaudeExecutor::with_program(FAKE_DEAF, Vec::new());

    let mut handle = executor
        .start(
            &project,
            "/gsd-progress".to_string(),
            capped(600_000, 300_000),
        )
        .await
        .expect("the deaf stand-in advertises every required capability");

    let child = handle.pgid;
    let started = Instant::now();
    let outcome = executor.cancel(&mut handle).await;
    let elapsed = started.elapsed();

    assert!(
        matches!(outcome, RunOutcome::Killed { .. }),
        "a child that ignored the terminate signal is still a kill, got: {outcome:?}"
    );
    assert!(
        elapsed >= TEARDOWN_GRACE - Duration::from_millis(500),
        "the escalation must come AFTER the grace period, not instead of it — a teardown \
         that skips straight to the uncatchable signal skips the CLI's whole clean \
         shutdown (D-14, Pitfall B). Observed: {elapsed:?}"
    );
    assert!(
        elapsed < TEARDOWN_GRACE * 3,
        "the escalation must actually fire once the grace expires. Observed: {elapsed:?}"
    );

    let events = drain(&mut handle).await;
    assert!(
        reported_an_exit(&events),
        "a signal-ignoring child must still be reaped, not merely killed (T-15-34). \
         Observed events: {events:?}"
    );
    assert!(
        gone_within(child, Duration::from_secs(5)).await,
        "the deaf child {child} survived the uncatchable signal"
    );
}

// ============================================================================
// The two deadlines are independent, and they classify differently (D-13)
// ============================================================================

#[tokio::test]
async fn a_child_that_goes_silent_trips_the_idle_cap_and_is_reported_as_stalled() {
    let scratch = TempDir::new().expect("temp dir");
    let project = DrivableProject::for_testing("stalled", scratch.path());
    // Announces itself, then says nothing at all — the reproduced hang's shape.
    let executor = slow(0, "0", "silent");

    let mut handle = executor
        .start(&project, "/gsd-progress".to_string(), capped(30_000, 400))
        .await
        .expect("start");

    let started = Instant::now();
    let outcome = handle.wait_outcome().await;
    let elapsed = started.elapsed();

    match outcome {
        RunOutcome::Stalled { idle_for } => assert_eq!(
            idle_for,
            Duration::from_millis(400),
            "the stalled outcome carries the cap that was actually breached"
        ),
        other => panic!(
            "a child that went silent must be reported as STALLED, not as {other:?}. The \
             wall-clock cap here is 30 seconds and was nowhere near breached — only a cap \
             measured from the last observed line can end this run (D-13)"
        ),
    }
    assert!(
        elapsed < Duration::from_secs(20),
        "the idle cap must end the run promptly rather than waiting out the wall-clock \
         cap. Observed: {elapsed:?}"
    );
}

#[tokio::test]
async fn a_child_that_keeps_emitting_is_never_killed_by_the_idle_cap() {
    let scratch = TempDir::new().expect("temp dir");
    let project = DrivableProject::for_testing("chatty", scratch.path());
    // Twenty heartbeats 50ms apart: a full second of continuous output against
    // a 400ms idle cap, so the run outlives its own idle cap two and a half
    // times over while never once being silent for it.
    let executor = slow(20, "0.05", "result");

    let mut handle = executor
        .start(&project, "/gsd-progress".to_string(), capped(30_000, 400))
        .await
        .expect("start");

    let started = Instant::now();
    let mut heartbeats = 0usize;
    while let Some(event) = handle.events.recv().await {
        if matches!(event, ExecutionEvent::Message(_)) {
            heartbeats += 1;
        }
    }
    let elapsed = started.elapsed();
    let outcome = handle.wait_outcome().await;

    assert_eq!(
        heartbeats, 20,
        "every heartbeat must reach the caller; a run cut short by the idle cap loses its \
         tail. Observed: {heartbeats}"
    );
    assert!(
        elapsed > Duration::from_millis(400),
        "precondition: the run must outlive its own idle cap, or it never tested it. \
         Observed: {elapsed:?}"
    );
    assert!(
        !matches!(outcome, RunOutcome::Stalled { .. } | RunOutcome::TimedOut { .. }),
        "a run that emits continuously is not stuck, however long it takes — this is the \
         property that makes an idle cap the correct stuck detector and time-since-spawn \
         the wrong one (D-13). Got: {outcome:?}"
    );
}

#[tokio::test]
async fn a_run_that_outlives_the_wall_clock_cap_is_reported_as_timed_out() {
    let scratch = TempDir::new().expect("temp dir");
    let project = DrivableProject::for_testing("timed-out", scratch.path());
    // Chattering for ten seconds: it never goes silent, so the idle cap can
    // never fire and only the wall-clock cap can end it. That is what makes the
    // two breaches distinguishable rather than two names for one timer.
    let executor = slow(200, "0.05", "result");

    let mut handle = executor
        .start(&project, "/gsd-progress".to_string(), capped(500, 30_000))
        .await
        .expect("start");

    let outcome = handle.wait_outcome().await;

    match outcome {
        RunOutcome::TimedOut { after } => assert_eq!(
            after,
            Duration::from_millis(500),
            "the timed-out outcome carries the cap that was actually breached"
        ),
        other => panic!(
            "a run that outlived its wall-clock cap while still emitting must be reported \
             as TIMED OUT — a wall-clock breach and an idle breach mean different things \
             to a user and are never collapsed (D-13). Got: {other:?}"
        ),
    }
}
