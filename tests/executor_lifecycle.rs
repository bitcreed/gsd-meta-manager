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
use std::path::Path;
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
/// The stand-in that never emits `system/init` at all, so the gate can only be
/// answered by a bound expiring.
const FAKE_SILENT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/fake-claude-silent.sh"
);
/// The stand-in that emits `system/init` only after its first stdin line — the
/// measured CLI 2.1.266 shape.
const FAKE_LATE_INIT: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/fake-claude-late-init.sh"
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

/// An executor pointed at the **late-announcing** stand-in, advertising the
/// full required capability set.
///
/// The stand-in prints nothing until it has read a line, so an executor that
/// withholds the prompt until the gate has ruled never gets a single byte out
/// of it. That is the CLI 2.1.266 deadlock, in miniature.
fn late_announcing(stdin_log: &Path) -> ClaudeExecutor {
    ClaudeExecutor::with_program(
        FAKE_LATE_INIT,
        vec![
            OsString::from("interrupt_receipt_v1,interrupt_cancel_queued_v1,msg_lifecycle_v1"),
            OsString::from("2.1.266"),
            OsString::from("none"),
            stdin_log.to_path_buf().into_os_string(),
        ],
    )
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
    let project = DrivableProject::for_testing_bypassing_opt_in("spawner", scratch.path());
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
    let project = DrivableProject::for_testing_bypassing_opt_in("reaped", scratch.path());
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
    let project = DrivableProject::for_testing_bypassing_opt_in("deaf", scratch.path());
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
    let project = DrivableProject::for_testing_bypassing_opt_in("drained", scratch.path());
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
    let project = DrivableProject::for_testing_bypassing_opt_in("capped-control", scratch.path());
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
    let project = DrivableProject::for_testing_bypassing_opt_in("stalled", scratch.path());
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
    let project = DrivableProject::for_testing_bypassing_opt_in("chatty", scratch.path());
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
    let project = DrivableProject::for_testing_bypassing_opt_in("timed-out", scratch.path());
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
    let project = DrivableProject::for_testing_bypassing_opt_in("flood-wall", scratch.path());
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
    let project = DrivableProject::for_testing_bypassing_opt_in("flood-cancel", scratch.path());
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
    let project = DrivableProject::for_testing_bypassing_opt_in("orphan", scratch.path());
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

// ============================================================================
// The spawn observer: a teardown handle for a start that never returns (CR-01)
//
// `ExecutionHandle` carries the agent's pgid, and for every test above that is
// enough — they all get a handle. The driver does not always get one: a stop
// that lands while `start` is still blocked on the capability gate leaves it
// with no handle and therefore no way to reach the agent's process group, which
// is a DIFFERENT group from the driver's own. The observer channel is what
// closes that window, and this is the test that it is fed before the gate rather
// than after it.
// ============================================================================

#[tokio::test]
async fn the_spawn_observer_publishes_the_agent_pgid_even_when_the_gate_never_opens() {
    use gsd_meta_manager::driver::liveness;

    let scratch = TempDir::new().expect("temp dir");
    let pidfile = scratch.path().join("agent-pids");
    let project = DrivableProject::for_testing_bypassing_opt_in("silent", scratch.path());

    let (tx, mut rx) = tokio::sync::oneshot::channel::<u32>();
    let executor =
        ClaudeExecutor::with_program(FAKE_SILENT, vec![OsString::from(pidfile.as_os_str())])
            .observing_spawn(tx);

    // The fixture never emits `system/init`, so `start` blocks on the capability
    // gate indefinitely. Racing it against a short sleep and asserting the SLEEP
    // won is what proves the parked state rather than assuming it: without that
    // assertion this test would pass identically against a fixture that started
    // normally, and would then be proving nothing about the startup window.
    let parked = tokio::select! {
        _ = executor.start(&project, "/gsd-progress".to_string(), capped(30_000, 20_000)) => false,
        _ = tokio::time::sleep(Duration::from_secs(2)) => true,
    };
    assert!(
        parked,
        "the silent stand-in must leave `start` parked on the capability gate. If \
         `start` returned, the fixture emitted a system/init it must never emit, and \
         this test is measuring the ordinary path instead of the startup window"
    );

    let pgid = rx
        .try_recv()
        .expect("the observer must receive the agent pgid BEFORE the gate is awaited");

    // The published number names a real process rather than something the
    // observer invented. The pid file is the only channel the fixture has — the
    // journal is fed from the event drain, and the drain never runs here — and
    // the first value in it is the stand-in's own `$$`. The executor spawns the
    // agent as a group LEADER, so that pid and the pgid are the same number.
    let pidfile_contents = std::fs::read_to_string(&pidfile).unwrap_or_default();
    let announced: Vec<u32> = pidfile_contents
        .split_whitespace()
        .filter_map(|token| token.parse().ok())
        .collect();
    assert_eq!(
        announced.first().copied(),
        Some(pgid),
        "the published pgid must be the stand-in's own pid; a mismatch means the \
         observer published something that is not the agent's group, and a teardown \
         aimed at it would miss (D-06, D-09). Pid file read: {pidfile_contents:?}"
    );

    // Losing the `select!` above dropped the `start` future, which dropped
    // `cancel_tx`, which fires the Coordinator's cancellation arm and tears the
    // agent group down. That this is an ASSERTION rather than a best-effort
    // cleanup is deliberate: it documents the exact behaviour
    // `driver::run::shutdown_during_startup` chooses NOT to rely on, because the
    // Coordinator's teardown is unobservable to the driver and dies with the
    // runtime the instant the driver process exits — which is the driver's very
    // next act after a startup stop.
    //
    // No signal is sent from this test and no `kill` shell-out is added: this
    // file spawns only through `ClaudeExecutor`.
    let deadline = Instant::now() + Duration::from_secs(5);
    let mut gone = false;
    while Instant::now() < deadline {
        if liveness::process_state(pgid).is_none() {
            gone = true;
            break;
        }
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
    assert!(
        gone,
        "the agent {pgid} outlived the dropped `start` future, so the Coordinator's \
         cancellation path did not run"
    );
}

// ============================================================================
// A CLI that announces only after the first user message (260908-uqq)
//
// The startup handshake is not symmetric across CLI versions. 2.1.220 prints
// `system/init` at startup; **2.1.266 prints it only once a user message has
// arrived on stdin** — measured directly, stdout and stderr both at zero bytes
// with stdin held open and empty, and init/assistant/result all arriving within
// a second of the first write. An executor that withholds the prompt until the
// gate has judged an init therefore deadlocks against it: the driver waits for
// the init, the CLI waits for the prompt. The reproduced failure ran 900.068
// seconds — `ExecutionOptions::default().idle_cap` to the millisecond — and was
// reported as `spawn_failed`, having delivered nothing.
//
// `ExecutionOptions::prompt_release_grace` breaks the tie: when the grace
// expires with no init observed, the supervisor writes the prompt anyway.
// ============================================================================

#[tokio::test]
async fn a_cli_that_announces_only_after_the_first_user_message_still_receives_its_command() {
    let scratch = TempDir::new().expect("temp dir");
    let stdin_log = scratch.path().join("stdin.log");
    let project = DrivableProject::for_testing_bypassing_opt_in("late-init", scratch.path());
    let executor = late_announcing(&stdin_log);

    // A short grace so the test does not wait out the production five seconds,
    // and an idle cap an order of magnitude above it so the run can only be
    // ended by the handshake completing, never by a bound expiring.
    let options = ExecutionOptions {
        prompt_release_grace: Duration::from_millis(200),
        ..capped(30_000, 5_000)
    };

    let started = Instant::now();
    let mut handle = tokio::time::timeout(
        Duration::from_secs(20),
        executor.start(&project, "/gsd-progress".to_string(), options),
    )
    .await
    .expect(
        "`start` never returned against a CLI that announces only after the first user \
         message. That is the 2.1.266 deadlock: the gate is waiting for an init the child \
         will not emit until the prompt it is gating arrives",
    )
    .expect("the stand-in advertises every required capability, so the gate must pass");

    // One command means one message; EOF is "no more input", not "stop".
    handle.close_input().await.expect("close stdin");

    let events = drain(&mut handle).await;
    let outcome = handle.wait_outcome().await;
    let elapsed = started.elapsed();

    // The point of the whole change: the command reached the child.
    let written = std::fs::read_to_string(&stdin_log)
        .expect("the stand-in truncates its stdin log at startup, so it must exist");
    assert!(
        written.contains("/gsd-progress"),
        "the command must reach the child's stdin — a run that gets past startup and \
         delivers nothing is the bug this test exists for. Log: {written:?}"
    );

    // Exactly once, whichever arm released it. The grace fires first here, and
    // the init it provokes must not release a second copy.
    assert_eq!(
        written.lines().filter(|line| line.contains("/gsd-progress")).count(),
        1,
        "the prompt is written exactly once whichever arm released it; two copies means \
         the grace arm and the gate arm both fired. Log: {written:?}"
    );

    assert!(
        events
            .iter()
            .any(|event| matches!(event, ExecutionEvent::SessionStarted { .. })),
        "the init the prompt provoked must still reach the caller as a session start: \
         {events:?}"
    );
    assert!(
        !matches!(
            outcome,
            RunOutcome::Stalled { .. } | RunOutcome::TimedOut { .. }
        ),
        "the run must complete on the handshake, not be ended by a bound expiring. \
         Got: {outcome:?}"
    );
    assert!(
        elapsed < Duration::from_secs(10),
        "the grace is 200ms; a run that took {elapsed:?} was released by something else"
    );
}

#[tokio::test]
async fn the_default_prompt_release_grace_sits_strictly_between_zero_and_the_idle_cap() {
    let defaults = ExecutionOptions::default();

    // Zero would delete the eager arm outright: no CLI could ever announce in
    // time, so the pre-prompt capability refusal — and with it the unconditional
    // D-06 zero-token property on every CLI that *does* announce eagerly —
    // would stop existing rather than become conditional.
    assert!(
        defaults.prompt_release_grace > Duration::ZERO,
        "a zero grace releases the prompt at spawn and destroys the eager arm's \
         pre-prompt refusal on every CLI, including the ones that announce in time"
    );
    // At or above the idle cap the grace is unreachable and the 2.1.266 deadlock
    // survives the fix, ended by the idle cap exactly as it was before.
    assert!(
        defaults.prompt_release_grace < defaults.idle_cap,
        "the grace must fire long before the idle cap; a grace at or beyond it leaves the \
         startup deadlock in place and merely renames its symptom. grace={:?} idle_cap={:?}",
        defaults.prompt_release_grace,
        defaults.idle_cap
    );
    // The run-wide idle cap is deliberately NOT shrunk to serve as a startup
    // bound: the two answer different questions, and `iteration_options`
    // inherits this value from `Default` on purpose.
    assert_eq!(
        defaults.idle_cap,
        Duration::from_secs(15 * 60),
        "the startup bound is its own knob; the run-wide idle cap stays where it was"
    );
}
