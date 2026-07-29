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

use gsd_meta_manager::error::SendError;
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
const FAKE_ORPHAN: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/fake-claude-orphan.sh"
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

/// An executor pointed at the paced stand-in with its pacing removed.
///
/// A zero interval means the stand-in emits as fast as the shell can print —
/// `printf` and the arithmetic are builtins and no `sleep` is forked — which is
/// what fills the executor's 8192-slot event channel and, with a consumer that
/// never drains it, keeps it full. That is the CR-01 condition: a supervisor
/// parked handing one event to a stalled consumer is a supervisor with every cap
/// and the cancel signal disabled.
fn flooding(lines: u32) -> ClaudeExecutor {
    slow(lines, "0", "result")
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

/// Deliberately NOT ignored, though it waits out the real ten-second teardown
/// grace and that cost is the point rather than an oversight.
///
/// It carried an ignore attribute for exactly that ~10 second cost, and it is
/// the ONLY proof the SIGKILL escalation path works at all. Three things
/// changed that trade:
///
/// 1. The escalation now has a **second entry point**. The post-exit branch
///    tears the group down whenever the group was not proven reaped, so the
///    path is reachable from an ordinary run's tail and not only from an
///    explicit cancel. A path with two entry points and zero CI coverage is
///    precisely the shape that produced CR-02.
/// 2. The ~10 seconds is wall-clock, not additive: the harness runs the tests
///    in this integration binary concurrently on threads, so the cost is
///    absorbed by the slower tests beside it rather than added to them.
/// 3. This plan forbids new ignored tests. Keeping a stale one while adding
///    bounded ones would be inconsistent.
#[tokio::test]
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
// An unanswered control request can never park its caller (D-13, D-31, CR-03)
//
// Two independent release paths, one test each. Both wrap the call in a hard
// `tokio::time::timeout`, because the defect being guarded against is an
// unbounded await: without the inner bound a regression would HANG the suite
// instead of failing it, and a hung CI job reports nothing at all.
// ============================================================================

/// Release path A: the run ended, so the registered waiter is dropped.
#[tokio::test]
async fn an_unanswered_interrupt_is_released_by_the_run_end_drain() {
    let scratch = TempDir::new().expect("temp dir");
    let project = DrivableProject::for_testing("drained", scratch.path());
    // Announces itself, drains stdin in the background, and answers nothing.
    let executor = slow(0, "0", "silent");

    // The idle cap ends the run at ~600ms. The control cap is left at its
    // 30-second default precisely so it CANNOT be what releases the caller —
    // that is what makes this test about the drain and not about the timeout.
    let mut handle = executor
        .start(&project, "/gsd-progress".to_string(), capped(30_000, 600))
        .await
        .expect("start");

    let started = Instant::now();
    let result = tokio::time::timeout(Duration::from_secs(15), executor.interrupt(&mut handle))
        .await
        .expect(
            "interrupt() never returned. A control_request the child never answers must not \
             park its caller for the process lifetime — that is the unbounded await D-13 \
             forbids (CR-03)",
        );
    let elapsed = started.elapsed();

    assert!(
        matches!(result, Err(SendError::ControlResponseLost { .. })),
        "a run that has ended can never answer a control request, so the waiter must be \
         released with an honest ControlResponseLost rather than a confident wrong answer. \
         Got: {result:?}"
    );
    assert!(
        elapsed < Duration::from_secs(10),
        "the run-end drain must release the caller the INSTANT the run ends, not when the \
         30-second control cap expires — that difference is the whole point of having a \
         drain as well as a cap. Observed: {elapsed:?}"
    );
    assert!(
        handle.pending_control.lock().await.is_empty(),
        "a released waiter must leave no entry behind in the correlation map"
    );
}

/// Release path B: the child is alive and emitting, so only the cap can fire.
#[tokio::test]
async fn an_unanswered_interrupt_on_a_live_child_is_released_by_the_control_response_cap() {
    let scratch = TempDir::new().expect("temp dir");
    let project = DrivableProject::for_testing("capped-control", scratch.path());
    // Twenty seconds of continuous heartbeats: neither deadline can fire and
    // the child is demonstrably alive, so the drain cannot be what releases the
    // caller either. Only the control cap is left.
    let executor = slow(400, "0.05", "result");

    let options = ExecutionOptions {
        control_response_cap: Duration::from_millis(400),
        ..capped(60_000, 30_000)
    };
    let mut handle = executor
        .start(&project, "/gsd-progress".to_string(), options)
        .await
        .expect("start");

    let started = Instant::now();
    let result = tokio::time::timeout(Duration::from_secs(15), executor.interrupt(&mut handle))
        .await
        .expect(
            "interrupt() never returned against a live child. An acknowledgement that never \
             arrives must be bounded, not awaited forever (D-13, CR-03)",
        );
    let elapsed = started.elapsed();

    assert!(
        matches!(result, Err(SendError::ControlResponseLost { .. })),
        "an unanswered request is lost, and saying so is preferable to claiming a \
         cancellation that never happened (D-31). Got: {result:?}"
    );
    assert!(
        elapsed >= Duration::from_millis(400),
        "the cap must actually be waited out rather than short-circuited by something \
         else ending the run. Observed: {elapsed:?}"
    );
    assert!(
        elapsed < Duration::from_secs(10),
        "the cap must fire promptly once it expires. Observed: {elapsed:?}"
    );
    assert!(
        handle.pending_control.lock().await.is_empty(),
        "a released waiter must leave no entry behind in the correlation map"
    );

    // Tear the chatty child down rather than leaving it running out its full
    // twenty seconds of heartbeats.
    executor.cancel(&mut handle).await;
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

// ============================================================================
// A stalled event consumer cannot disable the caps or the cancel signal
// (D-13, CR-01)
//
// Both tests here deliberately NEVER read `handle.events`. The receiver stays
// alive inside the handle, so the bounded channel fills and stays full — which
// is exactly the state the TUI is in during its documented blocking `$EDITOR`
// shell-out. Every existing lifecycle test drains immediately and therefore
// reaches neither condition.
//
// Both wrap the awaited call in a hard `tokio::time::timeout`, because the
// defect being guarded against is an unbounded park: without the inner bound a
// regression would HANG the suite instead of failing it, and a hung CI job
// reports nothing at all.
// ============================================================================

#[tokio::test]
async fn a_wall_clock_cap_still_fires_while_the_event_consumer_is_blocked() {
    let scratch = TempDir::new().expect("temp dir");
    let project = DrivableProject::for_testing("flood-wall", scratch.path());
    let executor = flooding(50_000);

    // A three-second wall cap and an idle cap far out of reach: only the wall
    // cap can end this run, so what the assertion observes is unambiguous.
    let mut handle = executor
        .start(&project, "/gsd-progress".to_string(), capped(3_000, 30_000))
        .await
        .expect("start");

    let started = Instant::now();
    let outcome = tokio::time::timeout(Duration::from_secs(60), handle.wait_outcome())
        .await
        .expect(
            "the run never reported an outcome. A consumer that stops draining must never be \
             able to disable the wall-clock cap — that cap is the only bound on a runaway \
             run's subscription quota spend, and a supervisor parked on a send has it, the \
             idle cap and the cancel signal all switched off at once (D-13, CR-01)",
        );
    let elapsed = started.elapsed();

    match outcome {
        RunOutcome::TimedOut { after } => assert_eq!(
            after,
            Duration::from_millis(3_000),
            "the timed-out outcome carries the cap that was actually breached"
        ),
        other => panic!(
            "a flooding run whose consumer never drains must still be reported as TIMED OUT. \
             Got: {other:?}"
        ),
    }
    assert!(
        elapsed < Duration::from_secs(30),
        "the cap must fire on its own schedule rather than whenever the flood happens to \
         relent. Observed: {elapsed:?}"
    );
}

#[tokio::test]
async fn a_cancel_is_still_honoured_while_the_event_consumer_is_blocked() {
    let scratch = TempDir::new().expect("temp dir");
    let project = DrivableProject::for_testing("flood-cancel", scratch.path());
    let executor = flooding(50_000);

    // Both caps far out of reach: ONLY the cancel can end this run.
    let mut handle = executor
        .start(&project, "/gsd-progress".to_string(), capped(60_000, 30_000))
        .await
        .expect("start");

    // Long enough that the channel is demonstrably full and the supervisor is
    // demonstrably parked handing an event to a consumer that will never take
    // it.
    tokio::time::sleep(Duration::from_secs(1)).await;

    let started = Instant::now();
    let outcome = tokio::time::timeout(Duration::from_secs(60), executor.cancel(&mut handle))
        .await
        .expect(
            "cancel() never returned. A cancel the user explicitly asked for cannot be \
             conditional on the TUI keeping up with the stream — a kill switch that only \
             works while nothing is wrong is not a kill switch (D-13, CR-01)",
        );
    let elapsed = started.elapsed();

    assert!(
        matches!(outcome, RunOutcome::Killed { .. }),
        "a cancelled run is a kill however fast the child was emitting, got: {outcome:?}"
    );
    assert!(
        elapsed < Duration::from_secs(30),
        "the cancel must take effect within the forward bound plus the teardown, not \
         whenever the flood relents. Observed: {elapsed:?}"
    );
}

// ============================================================================
// An observed exit is not a reaped group (D-13, D-14, CR-02)
// ============================================================================

#[tokio::test]
async fn a_descendant_holding_stdout_after_the_leader_exits_cannot_hang_the_run() {
    let scratch = TempDir::new().expect("temp dir");
    let project = DrivableProject::for_testing("orphan", scratch.path());
    let executor = ClaudeExecutor::with_program(FAKE_ORPHAN, Vec::new());

    // BOTH caps far out of reach: neither deadline can be what ends this run,
    // so only the post-exit drain bound can be.
    let mut handle = executor
        .start(&project, "/gsd-progress".to_string(), capped(60_000, 30_000))
        .await
        .expect("the orphan stand-in advertises every required capability");

    let descendant = grandchild_pid(&mut handle).await;
    assert!(
        alive(descendant),
        "precondition: the descendant {descendant} must be running, or the test proves \
         nothing"
    );

    let started = Instant::now();
    let outcome = tokio::time::timeout(Duration::from_secs(60), handle.wait_outcome())
        .await
        .expect(
            "the run never reported an outcome. `reader_rx` reaches EOF only once every \
             process holding the stdout write end is gone, so a descendant that outlives \
             the leader keeps that pipe open forever — and with the exited flag set, every \
             deadline guarded on it is switched off too. A run must ALWAYS end with an \
             outcome (D-13, CR-02)",
        );
    let elapsed = started.elapsed();

    assert!(
        matches!(
            outcome,
            RunOutcome::SucceededNoChanges { .. } | RunOutcome::SucceededWithChanges { .. }
        ),
        "the leader's own terminal envelope reported success and its exit status was \
         observed, so the outcome still comes from the four sources — this is neither a \
         timeout nor a kill. Got: {outcome:?}"
    );
    assert!(
        elapsed < Duration::from_secs(30),
        "the post-exit drain bound must end the run on its own schedule. Observed: \
         {elapsed:?}"
    );

    assert!(
        gone_within(descendant, Duration::from_secs(5)).await,
        "the descendant {descendant} outlived the run. An observed exit is not a reaped \
         group: the wrapper caches the leader's status, so `exited` can be true while group \
         members are still alive, and a survivor holds the user's files, ports and \
         subscription quota invisibly and without consent (D-14, T-15-30, T-15-52)"
    );

    let events = drain(&mut handle).await;
    assert!(
        reported_an_exit(&events),
        "the run must still report the leader's own exit status. Observed events: {events:?}"
    );
}
