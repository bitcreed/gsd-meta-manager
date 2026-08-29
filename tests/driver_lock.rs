// ============================================================================
// The single-execution lock: one driver per project, and a loser that can say
// which run holds it (CTRL-05, D-19, D-20).
//
// Every test here needs two real, concurrently-live processes, which is the
// whole reason none of them is an in-source unit test. `flock` is a property of
// the kernel's view of open file descriptions; a test that never has two of them
// open at once proves nothing about contention, and a lock that is only ever
// taken by one caller passes every assertion an implementation could make about
// itself.
//
// One of them goes further and needs a driver in a **separate process image**:
// `the_lock_is_released_when_the_holding_process_dies` spawns the real binary
// through `CARGO_BIN_EXE_gsd-meta-manager` so it can SIGKILL it. That property —
// the lock dies with its holder — is the entire reason `flock` was chosen over a
// PID file, and it cannot be observed from inside the process that holds it.
//
// Unix-only by construction: driving is a Unix capability (D-05) and
// `src/driver/lock.rs` is `#[cfg(unix)]`.
// ============================================================================

#![cfg(unix)]

use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::time::Duration;

use gsd_meta_manager::config::{save_config, Config, DriverOptIn, RegisteredProject};
use gsd_meta_manager::driver::lock::{self, LockHolder};
use gsd_meta_manager::driver::{drive, DriveArgs};
use gsd_meta_manager::error::{DriveError, LockError};
use tempfile::TempDir;

/// A visible argv payload for the fixtures below.
///
/// `DriveArgs`'s argv-derived fields are `payload::NonBlank`, whose field is
/// private: there is exactly one route in and it refuses a value carrying
/// nothing a reader could see. It `expect`s rather than returning the
/// constructor's `Option` directly, so a fixture whose own literal turned out to
/// be invisible fails loudly here instead of silently becoming an ABSENT flag —
/// which would quietly convert a test of "blank is refused" into a test of
/// "nothing was supplied".
fn nonblank(raw: &str) -> gsd_meta_manager::driver::payload::NonBlank {
    gsd_meta_manager::driver::payload::NonBlank::new(raw)
        .expect("a visible test literal is a payload")
}


/// The paced stand-in, whose three leading arguments control how long it stays
/// alive. It is what holds the first run open while the second attempts.
const FAKE_SLOW: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/fake-claude-slow.sh"
);

const ALIAS: &str = "locked";

/// The alias for the early-exit reporting control below.
///
/// Deliberately NOT registered in that test's config: the refusal it wants is
/// `OptInError::UnknownAlias`, raised at the registry lookup before any envelope
/// or lock work happens (src/driver/mod.rs:837), which is exactly the shape of
/// child failure that used to masquerade as a lock timeout.
const ALIAS_EARLY_EXIT_REPORT: &str = "lock-early-exit-report";

/// Run A's id: the one a losing attempt must name.
const RUN_A: &str = "2026-07-29T12-00-00Z-aaaa";
/// Run B's id: deliberately different, so an assertion on run A's id cannot pass
/// by accident.
const RUN_B: &str = "2026-07-29T12-00-01Z-bbbb";

/// How long run A stays alive: 40 heartbeats at 150ms is about six seconds,
/// comfortably inside the executor's 15-minute idle cap and comfortably longer
/// than a non-blocking refusal takes.
const A_HEARTBEATS: &str = "40";
const A_INTERVAL: &str = "0.15";

/// A project root with a `.planning/` directory.
fn project_root() -> TempDir {
    let root = TempDir::new().expect("temp dir");
    std::fs::create_dir_all(root.path().join(".planning")).expect("scratch .planning");
    root
}

/// Point the envelope at a temp root for this test binary.
///
/// **Since plan 19-07 a driven run establishes its envelope before the executor
/// is constructed**, and the envelope lives under the application data
/// directory. Without this redirect these fixtures would write hook stubs, a
/// generated git config and a settings file into the developer's real
/// `~/.local/share` under a fixture's alias — the same "a test may not write
/// into the developer's real data directory" rule `envelope::hooks::guard_in`
/// records for its own explicit-root sibling.
///
/// The `set_var` happens exactly **once** per test binary, inside the
/// `OnceLock` initialiser, and the `TempDir` is held by the `static` for the
/// process lifetime so the root outlives every test that drives a run.
fn isolate_envelope_root() {
    static ROOT: std::sync::OnceLock<TempDir> = std::sync::OnceLock::new();
    ROOT.get_or_init(|| {
        let dir = TempDir::new().expect("an envelope temp root");
        std::env::set_var(gsd_meta_manager::envelope::ENVELOPE_ROOT_ENV, dir.path());
        dir
    });
}

/// A one-entry registry pointing at `root`, opted in.
///
/// Built fresh per caller rather than shared: `Config` is not `Clone`, and each
/// concurrent drive in these tests needs its own owned value.
fn config_for(root: &Path) -> Config {
    isolate_envelope_root();
    let mut config = Config::new();
    config.projects.insert(
        ALIAS.to_string(),
        RegisteredProject {
            path: root.to_path_buf(),
            added: "2026-07-29T12:00:00Z".to_string(),
            driver_opt_in: Some(DriverOptIn {
                opted_in_at: "2026-07-29T11:59:00Z".to_string(),
                claude_md_digest: None,
                prompt_inputs: gsd_meta_manager::registry::current_prompt_inputs(root),
                extra: Default::default(),
                branch_namespace: None,
                credential: None,
                pr_cap_per_24h: None,
                pr_cap_per_run: None,
            }),
            extra: Default::default(),
        },
    );
    config
}

/// Drive arguments pointed at the paced stand-in.
fn args(run_id: &str, heartbeats: &str, interval: &str) -> DriveArgs {
    DriveArgs {
        alias: nonblank(ALIAS),
        command: Some(nonblank("/gsd-progress")),
        target_phase: None,
        max_steps: None,
        wall_clock_cap_secs: None,
        max_escalations: None,
        approved_plan: None,
        run_id: Some(nonblank(run_id)),
        dry_run: false,
        goal: None,
        claude_program: Some(FAKE_SLOW.into()),
        claude_args: vec![
            OsString::from(heartbeats),
            OsString::from(interval),
            OsString::from("result"),
        ],
    }
}

fn planning_of(root: &Path) -> PathBuf {
    root.join(".planning")
}

/// Start run A in the background and return once it actually holds the lock.
///
/// Polls for the holder record rather than sleeping a fixed duration and hoping:
/// a fixed sleep either flakes on a loaded machine or wastes the difference on
/// every run, and neither is a property of the thing under test.
async fn start_holder(root: &Path) -> (tokio::task::JoinHandle<Result<(), DriveError>>, LockHolder) {
    let root_owned = root.to_path_buf();
    let handle = tokio::spawn(async move {
        let config = config_for(&root_owned);
        drive(args(RUN_A, A_HEARTBEATS, A_INTERVAL), &config).await
    });

    let planning = planning_of(root);
    for _ in 0..600 {
        if let Some(holder) = lock::read_holder(&planning) {
            return (handle, holder);
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    panic!("run A never wrote a lock holder record within 30s");
}

/// Attempt a second drive against the same project, in this process.
async fn second_drive(root: &Path) -> DriveError {
    let config = config_for(root);
    drive(args(RUN_B, "1", "0"), &config)
        .await
        .expect_err("a second drive against a live run must be refused")
}

/// Directory entries under the runs root that are run directories.
fn run_directories(root: &Path) -> Vec<String> {
    let runs_root = gsd_meta_manager::journal::runs_root(&planning_of(root));
    let mut names: Vec<String> = std::fs::read_dir(&runs_root)
        .expect("the runs root exists once a run has started")
        .filter_map(|entry| {
            let entry = entry.ok()?;
            entry
                .file_type()
                .ok()?
                .is_dir()
                .then(|| entry.file_name().to_string_lossy().into_owned())
        })
        .collect();
    names.sort();
    names
}

/// Where a spawned child's stdout and stderr go, and how they are rendered when
/// something needs explaining.
///
/// **Files, never pipes.** Nothing drains a pipe during the poll below, so a
/// child that outgrew the kernel's buffer would block forever on its next write
/// and this test would hang instead of failing — strictly worse than the rare
/// misleading failure being fixed here. A file has no such ceiling. The `TempDir`
/// is owned by this struct so the captures outlive the child and are readable
/// after it dies.
struct ChildCapture {
    /// Held for its `Drop`: the captures live exactly as long as this value.
    _dir: TempDir,
    stdout: PathBuf,
    stderr: PathBuf,
}

impl ChildCapture {
    fn new() -> Self {
        let dir = TempDir::new().expect("a temp dir for the child's captured output");
        let stdout = dir.path().join("stdout.log");
        let stderr = dir.path().join("stderr.log");
        Self {
            _dir: dir,
            stdout,
            stderr,
        }
    }

    fn redirect_stdout(&self) -> std::process::Stdio {
        Self::redirect(&self.stdout)
    }

    fn redirect_stderr(&self) -> std::process::Stdio {
        Self::redirect(&self.stderr)
    }

    fn redirect(path: &Path) -> std::process::Stdio {
        std::process::Stdio::from(
            std::fs::File::create(path).expect("a capture file for the child's output"),
        )
    }

    /// Both captures, rendered for a failure message.
    ///
    /// Called on failure paths ONLY. A success path that reads these files is a
    /// success path that can fail for a new reason.
    fn report(&self) -> String {
        let read = |path: &Path| std::fs::read_to_string(path).unwrap_or_default();
        format!(
            "\n--- child stdout ---\n{}\n--- child stderr ---\n{}\n--- end ---",
            read(&self.stdout),
            read(&self.stderr)
        )
    }
}

/// Wait for the child to take the lock, or say what actually happened instead.
///
/// A `Result` rather than a `panic!` inside the helper, deliberately: it is what
/// makes the reporting behaviour itself testable, which is what
/// `a_child_that_dies_before_taking_the_lock_is_reported_with_its_status_and_stderr`
/// exercises.
///
/// The holder record is checked BEFORE the child's exit status on every
/// iteration, so a child that takes the lock and then dies is still a success —
/// the record is on disk and outlives its writer, which is the property
/// `the_lock_is_released_when_the_holding_process_dies` exists to prove.
async fn wait_for_lock_or_report(
    child: &mut std::process::Child,
    planning: &Path,
    capture: &ChildCapture,
) -> Result<(), String> {
    for _ in 0..600 {
        if lock::read_holder(planning).is_some() {
            return Ok(());
        }

        // The arm that would have turned a two-hour diagnosis into a one-line
        // answer (G-19-4, 2026-08-18): a child that dies in envelope
        // establishment never reaches `lock::acquire`, so polling only for the
        // holder record reports the 30s deadline and says nothing about the
        // exit that made the deadline inevitable.
        if let Some(status) = child
            .try_wait()
            .expect("the child's exit status is readable")
        {
            return Err(format!(
                "the child driver exited with {status} BEFORE taking the lock, so the \
                 deadline below was never reachable{}",
                capture.report()
            ));
        }

        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    // Kept verbatim from before this helper existed, so anyone searching the
    // history for the original symptom still lands here.
    Err(format!(
        "the child driver never took the lock within 30s{}",
        capture.report()
    ))
}

#[tokio::test]
async fn a_second_drive_reports_which_run_holds_the_lock() {
    let root = project_root();
    let (a, holder) = start_holder(root.path()).await;
    assert_eq!(holder.run_id, RUN_A, "run A recorded itself as the holder");

    let err = second_drive(root.path()).await;

    let DriveError::Lock(LockError::HeldBy {
        run_id,
        pgid,
        started_at,
    }) = &err
    else {
        panic!("the refusal must be the who-holds-it variant, got: {err:?}");
    };
    assert_eq!(
        run_id, RUN_A,
        "the refusal names the HOLDING run, not the attempting one"
    );
    assert_ne!(*pgid, 0, "a zero pgid names no process group");
    assert!(!started_at.is_empty(), "the holder's start time is recorded");

    // Criterion #5 is about what the USER IS TOLD, so the assertion is on the
    // rendered string and not merely on the variant. A `HeldBy` whose `Display`
    // said only "locked" would satisfy every other assertion in this file.
    let rendered = err.to_string();
    assert!(
        rendered.contains(RUN_A),
        "the rendered refusal must name the holding run id, got: {rendered}"
    );
    assert!(
        rendered.contains(&pgid.to_string()),
        "the rendered refusal must name the holding process group, got: {rendered}"
    );

    // "A second run was not started" is checked ON DISK, not inferred from the
    // `Err`: the losing attempt must have created no run directory at all,
    // because it refused before `JournalRun::start` (D-12).
    assert_eq!(
        run_directories(root.path()),
        vec![RUN_A.to_string()],
        "exactly one run directory may exist; the loser started no run"
    );

    let completed = a.await.expect("run A's task did not panic");
    completed.expect("run A holds the lock and runs to completion");
}

// The multi-threaded flavour and the `tokio::spawn` below are both load-bearing,
// and neither is stylistic.
//
// `lock::acquire` is a **synchronous** syscall inside an async fn, so a blocking
// `flock` parks the OS thread rather than yielding. `tokio::time::timeout` can
// only fire when the task awaiting it is polled, and `Timeout::poll` polls the
// inner future inline — so wrapping the call directly, on a current-thread
// runtime, produces a bound that cannot fire: the timer, run A, and the timeout
// itself all share the one parked thread, and the suite deadlocks instead of
// failing. That was observed, not theorised.
//
// Putting the second drive on its own task and awaiting the *handle* keeps the
// parked thread away from the timer: the stuck poll occupies one worker, and the
// task holding the timeout is woken on another. Four workers so run A keeps
// making progress alongside both.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn a_duplicate_start_refuses_promptly_rather_than_blocking() {
    let root = project_root();
    let (a, _holder) = start_holder(root.path()).await;

    // The one test that distinguishes a non-blocking acquire from a blocking one
    // (D-20.4). A blocking `flock` would wait here until run A finished and then
    // SUCCEED, passing every other assertion in this file while turning a
    // duplicate start into the failure mode a user cannot diagnose. One second is
    // generous by orders of magnitude against a `LOCK_NB` syscall, and run A
    // stays alive for about six.
    let root_owned = root.path().to_path_buf();
    let attempt = tokio::spawn(async move { second_drive(&root_owned).await });

    let refused = tokio::time::timeout(Duration::from_secs(1), attempt).await;

    let err = refused
        .expect("the second drive must refuse within one second, never block")
        .expect("the second drive's task did not panic");
    assert!(
        matches!(err, DriveError::Lock(_)),
        "the prompt refusal must be the lock refusal, got: {err:?}"
    );

    let completed = a.await.expect("run A's task did not panic");
    completed.expect("run A runs to completion");
}

/// The named WR-10 / D-28 regression: the run body's blocking work runs on the
/// blocking pool, so the async runtime is never parked.
///
/// **Current-thread on purpose, and that is the whole test.** The comment above
/// records the deadlock this repository *observed*: a blocking `flock` inside an
/// `async fn`, on a runtime with one thread, defeats `tokio::time::timeout`
/// outright — the timer, the attempting task and the timeout itself all share
/// that thread, and `Timeout::poll` polls its inner future inline, so a parked
/// thread polls nothing at all. Its sibling above buys its way out of that with
/// four workers; this one deliberately does not, because the property under test
/// is *where the blocking call runs*, not how many threads can absorb it.
///
/// **Against a tree where the wrap is missing and the acquire blocks, this test
/// HANGS rather than fails**, which is strictly worse than a red test and is why
/// that is said here out loud rather than left for the next person to discover
/// at 3am. The assertion is therefore on the timeout being *able to fire* — a
/// bound that cannot fire is not a bound — and the timeout is generous by orders
/// of magnitude against a `LOCK_NB` syscall on the blocking pool.
///
/// The attempt runs on its own task and the test awaits the **join handle**,
/// never the call itself: this file's own header records why, and awaiting the
/// call directly would put the stuck poll and the timer on the same task as well
/// as the same thread.
///
/// The lock is held by a plain guard rather than by a live run, because this
/// test is about where the acquisition executes and not about who wins — and a
/// foreign holder keeps the assertion down to a single `drive`.
#[tokio::test(flavor = "current_thread")]
async fn acquiring_the_run_lock_does_not_block_the_async_runtime() {
    let root = project_root();
    let planning = planning_of(root.path());

    let held = lock::acquire(&planning, RUN_A, std::process::id())
        .expect("this test takes the lock before anything contends for it");

    let root_owned = root.path().to_path_buf();
    let attempt = tokio::spawn(async move { second_drive(&root_owned).await });

    let err = tokio::time::timeout(Duration::from_secs(5), attempt)
        .await
        .expect(
            "the bound must be ENFORCEABLE. A timeout that expires here means the \
             attempt was slow; a timeout that never resolves at all means the one \
             runtime thread was parked inside a synchronous syscall, which is the \
             deadlock recorded at the top of this file (D-28, WR-10)",
        )
        .expect("the attempting task did not panic");

    let DriveError::Lock(LockError::HeldBy { run_id, .. }) = &err else {
        panic!("the refusal must still be the who-holds-it variant, got: {err:?}");
    };
    assert_eq!(
        run_id, RUN_A,
        "moving the acquire onto the blocking pool must not change WHAT it \
         answers — only which thread finds out"
    );

    // Explicit, and not merely `let _`: the descriptor is the lock, so the
    // release has to happen after the assertions rather than wherever the
    // compiler happens to end the binding's scope.
    drop(held);
}

#[tokio::test]
async fn the_losing_reader_does_not_truncate_the_lock_file() {
    let root = project_root();
    let (a, _holder) = start_holder(root.path()).await;

    let lock_file = lock::lock_path(&planning_of(root.path()));
    let before = std::fs::read(&lock_file).expect("the holder's record is on disk");
    assert!(!before.is_empty(), "the holder wrote a non-empty record");

    let _ = second_drive(root.path()).await;

    let after = std::fs::read(&lock_file).expect("the lock file survives a losing attempt");
    assert_eq!(
        before.len(),
        after.len(),
        "a losing attempt must not change the lock file's length"
    );
    assert_eq!(
        before, after,
        "the lock file must be BYTE-IDENTICAL after a losing attempt. A loser that \
         opened it with O_TRUNC would destroy the answer to \"which run holds the \
         lock\" — and every other assertion in this file would still pass (D-20.3)"
    );

    let completed = a.await.expect("run A's task did not panic");
    completed.expect("run A runs to completion");
}

#[tokio::test]
async fn the_lock_is_released_when_the_holding_process_dies() {
    let root = project_root();
    let planning = planning_of(root.path());

    // The holder must be a process this test can kill, so it is the REAL BINARY
    // in its own process image rather than a task in this one.
    let config_dir = TempDir::new().expect("temp dir for the config");
    let config_path = config_dir.path().join("config.json");
    save_config(&config_for(root.path()), &config_path).expect("write the driver's config");

    // Files, not pipes, and not `Stdio::null()`. Null'd output is what made a
    // child that died in envelope establishment indistinguishable from a child
    // that was merely slow (G-19-4); a pipe nothing drains would deadlock the
    // child once the kernel buffer filled, which is worse than either.
    let capture = ChildCapture::new();
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_gsd-meta-manager"))
        .arg("--config")
        .arg(&config_path)
        .arg("drive")
        .arg(ALIAS)
        .arg("--command")
        .arg("/gsd-progress")
        .arg("--run-id")
        .arg(RUN_A)
        .arg("--claude-program")
        .arg(FAKE_SLOW)
        .args(["--claude-args", A_HEARTBEATS])
        .args(["--claude-args", A_INTERVAL])
        .args(["--claude-args", "result"])
        .stdin(std::process::Stdio::null())
        .stdout(capture.redirect_stdout())
        .stderr(capture.redirect_stderr())
        .spawn()
        .expect("the driver binary is built and spawnable");

    wait_for_lock_or_report(&mut child, &planning, &capture)
        .await
        .unwrap_or_else(|why| panic!("{why}"));

    // SIGKILL: the one signal the holder cannot handle, so no cooperative
    // release can possibly have run. Whatever releases the lock here is the
    // kernel closing the descriptor as the process dies, and nothing else.
    //
    // This is the exact scenario a PID file cannot survive (D-19): the file
    // would still name a dead pid, and every later start would either refuse
    // forever or have to guess whether that pid had been recycled. The child's
    // own `claude` stand-in is orphaned by this and exits on its own within a
    // few seconds; it holds no descriptor on the lock file, because Rust opens
    // files O_CLOEXEC and the lock fd is therefore never inherited.
    child.kill().expect("SIGKILL the holding driver");
    child.wait().expect("reap the killed driver");

    let recovered = lock::acquire(&planning, "recovery-after-sigkill", std::process::id())
        .expect(
            "a SIGKILLed holder releases the lock by dying; if this fails the lock is \
             not process-death-safe and flock was not what was implemented (D-19)",
        );
    assert_eq!(
        recovered.holder().run_id,
        "recovery-after-sigkill",
        "the recovering acquire records itself as the new holder"
    );
}

/// The reporting itself, under test (G-19-4, T-19-10-01).
///
/// On 2026-08-18 the sigkill test above failed with the single sentence "the
/// child driver never took the lock within 30s". The child had in fact died
/// early, in envelope establishment, and its output was null'd — so the only
/// visible symptom named the lock, which was never reached. This control proves
/// the harness now says what happened: a child that exits before writing a
/// holder record is reported with its exit status and its stderr, in seconds.
///
/// The refusal is manufactured at the cheapest possible seam — an alias that is
/// not in the saved config, refused at the registry lookup
/// (src/driver/mod.rs:837) and printed by `main` before any envelope, journal or
/// lock work exists. That is deliberately the same *shape* as the real failure
/// (die early, say nothing, look like a timeout) without depending on the real
/// failure being reproducible, which it is not: it was a microsecond window.
#[tokio::test]
async fn a_child_that_dies_before_taking_the_lock_is_reported_with_its_status_and_stderr() {
    // `config_for` is not used here — this alias must NOT be registered — so the
    // envelope redirect that it would otherwise perform is done explicitly.
    isolate_envelope_root();

    let root = project_root();
    let planning = planning_of(root.path());

    let config_dir = TempDir::new().expect("temp dir for the config");
    let config_path = config_dir.path().join("config.json");
    save_config(&Config::new(), &config_path).expect("write a config with no projects");

    let capture = ChildCapture::new();
    let mut child = std::process::Command::new(env!("CARGO_BIN_EXE_gsd-meta-manager"))
        .arg("--config")
        .arg(&config_path)
        .arg("drive")
        .arg(ALIAS_EARLY_EXIT_REPORT)
        .arg("--command")
        .arg("/gsd-progress")
        .stdin(std::process::Stdio::null())
        .stdout(capture.redirect_stdout())
        .stderr(capture.redirect_stderr())
        .spawn()
        .expect("the driver binary is built and spawnable");

    let started = std::time::Instant::now();
    let reported = wait_for_lock_or_report(&mut child, &planning, &capture).await;
    let elapsed = started.elapsed();

    let why = reported.expect_err(
        "a child refused at the registry lookup never takes the lock, so the wait must \
         report rather than succeed",
    );

    // `try_wait` caches the status, so asking again after the helper observed it
    // yields the same value — which is what lets this assert on the ACTUAL
    // status rather than on a guess at how one renders.
    let status = child
        .try_wait()
        .expect("the child's exit status is readable")
        .expect("the child has already exited");
    assert!(
        !status.success(),
        "the fixture must produce a FAILING child, got: {status}"
    );
    assert!(
        why.contains(&status.to_string()),
        "the report must name the child's exit status, got: {why}"
    );

    // The stderr half. Without it the report would say "it died" and still leave
    // the reader to go and find out why.
    assert!(
        why.contains("no project is registered under the alias"),
        "the report must carry the child's stderr, got: {why}"
    );
    assert!(
        why.contains(ALIAS_EARLY_EXIT_REPORT),
        "the report must carry the alias the child refused, got: {why}"
    );

    // The point of polling `try_wait` alongside the holder record is that a dead
    // child is reported when it dies, not when the deadline expires. Ten seconds
    // is generous by orders of magnitude against an immediate refusal and still
    // a third of the 30s deadline, so a regression to "wait it out" is caught
    // rather than merely being slower.
    assert!(
        elapsed < Duration::from_secs(10),
        "an early exit must be reported in seconds, not at the 30s deadline; took {elapsed:?}"
    );
}
