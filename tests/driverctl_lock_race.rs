// ============================================================================
// The probe/acquire race, demonstrated and closed (D-20 race tests 1-5).
//
// A reader that probes `launcher.lock` takes `LOCK_SH`. That hold is momentary
// but real, and a launcher whose single `LOCK_EX | LOCK_NB` attempt lands inside
// it is refused: the run it would have started is reported as "already running",
// naming a stale pgid. `acquire_at_with_retry` closes that by re-attempting for
// a bounded budget.
//
// `flock` is per OPEN FILE DESCRIPTION, so two threads that each make their own
// `File::open` contend exactly like two processes do (27-RESEARCH E5). That is
// why these tests stay in-process: a thread with its own descriptor is a faithful
// stand-in for a reader in another process, and nothing here needs a separate
// image the way `tests/driver_lock.rs`'s SIGKILL test does.
//
// Items 6-8 of D-20's race-test list land in 27-04 and 27-06.
//
// Unix-only by construction: `src/driver/lock.rs` is `#[cfg(unix)]`.
// ============================================================================

#![cfg(unix)]

use std::fs::File;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread;
use std::time::{Duration, Instant};

use gsd_meta_manager::driver::lock::{
    acquire_at, acquire_at_with_retry, probe_at, LockProbe, ACQUIRE_RETRY_BUDGET,
};
use gsd_meta_manager::error::LockError;
use rustix::fs::{flock, FlockOperation};
use tempfile::TempDir;

/// Hold `LOCK_SH` on `path` from a read-only descriptor for `hold`, signalling
/// once it is held. This is the probe's shape, held deliberately longer than a
/// real probe so the window is wide enough to hit on every run.
fn hold_shared(path: &Path, hold: Duration) -> thread::JoinHandle<()> {
    let path = path.to_path_buf();
    let (tx, rx) = mpsc::channel();
    let handle = thread::spawn(move || {
        let file = File::open(&path).expect("open the lock file read-only");
        flock(&file, FlockOperation::NonBlockingLockShared).expect("take LOCK_SH");
        tx.send(()).expect("signal that the hold has begun");
        thread::sleep(hold);
        drop(file);
    });
    rx.recv().expect("the shared hold began");
    handle
}

fn lock_file(dir: &TempDir) -> std::path::PathBuf {
    let path = dir.path().join("launcher.lock");
    std::fs::write(&path, b"").expect("seed the lock file");
    path
}

fn is_refusal(err: &LockError) -> bool {
    matches!(err, LockError::HeldBy { .. } | LockError::HeldByUnknownRun)
}

#[test]
fn single_attempt_is_refused_under_a_probe_hold() {
    let dir = TempDir::new().expect("temp dir");
    let path = lock_file(&dir);
    let holder = hold_shared(&path, Duration::from_millis(30));

    let result = acquire_at(&path, "launcher", 1);

    holder.join().expect("holder thread");
    match result {
        Err(err) => assert!(
            is_refusal(&err),
            "a single attempt inside a shared hold is a refusal, not a fault: {err:?}"
        ),
        Ok(_) => panic!(
            "NEGATIVE CONTROL FAILED: a single attempt succeeded while a LOCK_SH was held, so \
             the retry tests below would prove nothing"
        ),
    }
}

#[test]
fn retrying_acquire_wins_under_a_probe_hold() {
    let dir = TempDir::new().expect("temp dir");
    let path = lock_file(&dir);
    let holder = hold_shared(&path, Duration::from_millis(30));

    let result = acquire_at_with_retry(&path, "launcher", 1, ACQUIRE_RETRY_BUDGET);

    holder.join().expect("holder thread");
    assert!(
        result.is_ok(),
        "the retrying acquire must outlast a 30 ms probe hold (budget {ACQUIRE_RETRY_BUDGET:?}): {:?}",
        result.err()
    );
}

#[test]
fn stress_no_false_refusal_with_retry_and_some_without() {
    const CYCLES: usize = 2_000;
    let dir = TempDir::new().expect("temp dir");
    let path = lock_file(&dir);

    let stop = Arc::new(AtomicBool::new(false));
    let probers: Vec<_> = (0..2)
        .map(|_| {
            let stop = Arc::clone(&stop);
            let path = path.clone();
            thread::spawn(move || {
                while !stop.load(Ordering::Relaxed) {
                    let _ = probe_at(&path);
                }
            })
        })
        .collect();

    let mut refused_with_retry = 0usize;
    for _ in 0..CYCLES {
        match acquire_at_with_retry(&path, "launcher", 1, ACQUIRE_RETRY_BUDGET) {
            Ok(lock) => drop(lock),
            Err(_) => refused_with_retry += 1,
        }
    }

    let mut refused_without_retry = 0usize;
    for _ in 0..CYCLES {
        match acquire_at(&path, "launcher", 1) {
            Ok(lock) => drop(lock),
            Err(_) => refused_without_retry += 1,
        }
    }

    stop.store(true, Ordering::Relaxed);
    for prober in probers {
        prober.join().expect("prober thread");
    }

    assert_eq!(
        refused_with_retry, 0,
        "the retrying acquire produced {refused_with_retry} false refusals in {CYCLES} cycles \
         against two tight-loop probers (no-retry arm: {refused_without_retry})"
    );
    assert!(
        refused_without_retry > 0,
        "the single-attempt arm saw {refused_without_retry} refusals in {CYCLES} cycles; a zero \
         would mean the probers never contend and the retry arm's zero above is vacuous"
    );
}

#[test]
fn real_holder_is_still_refused_within_budget() {
    let dir = TempDir::new().expect("temp dir");
    let path = dir.path().join("launcher.lock");

    let (tx, rx) = mpsc::channel();
    let holder_path = path.clone();
    let holder = thread::spawn(move || {
        let lock = acquire_at(&holder_path, "holder-run", 4242).expect("the holder wins");
        tx.send(()).expect("signal that the holder is in");
        thread::sleep(Duration::from_millis(800));
        drop(lock);
    });
    rx.recv().expect("the holder acquired and wrote its record");

    let started = Instant::now();
    let result = acquire_at_with_retry(&path, "second", 1, ACQUIRE_RETRY_BUDGET);
    let elapsed = started.elapsed();

    holder.join().expect("holder thread");
    match result {
        Err(LockError::HeldBy { run_id, pgid, .. }) => {
            assert_eq!(run_id, "holder-run", "the refusal names the real holder");
            assert_eq!(pgid, 4242);
        }
        Err(other) => panic!("a real exclusive holder must be refused with HeldBy, got {other:?}"),
        Ok(_) => panic!("a real exclusive holder must be refused, but the acquire succeeded"),
    }
    // WR-09: only "it did not hang". A tight upper bound over the retry budget fails
    // on any scheduler pause after the last retry sleep; the lower bound below is
    // the deterministic half of the property.
    assert!(
        elapsed < ACQUIRE_RETRY_BUDGET + Duration::from_secs(10),
        "the refusal must arrive after the budget, not hang; it took {elapsed:?}"
    );
    assert!(
        elapsed >= ACQUIRE_RETRY_BUDGET,
        "a real holder is refused only after the budget has been spent; it took {elapsed:?}"
    );
}

#[test]
fn concurrent_probes_never_see_each_other() {
    let dir = TempDir::new().expect("temp dir");
    let path = lock_file(&dir);

    // One reader holds LOCK_SH on its own descriptor: a second reader must still
    // read Free, because two readers are not holders of each other (E3).
    let holder = hold_shared(&path, Duration::from_millis(200));
    assert_eq!(
        probe_at(&path),
        LockProbe::Free,
        "a probe must not see another reader's LOCK_SH as an exclusive holder"
    );

    let probes: Vec<_> = (0..2)
        .map(|_| {
            let path = path.clone();
            thread::spawn(move || probe_at(&path))
        })
        .collect();
    for probe in probes {
        assert_eq!(
            probe.join().expect("probe thread"),
            LockProbe::Free,
            "two concurrent probes of an unheld file both read Free"
        );
    }
    holder.join().expect("holder thread");

    assert_eq!(
        probe_at(&dir.path().join("missing.lock")),
        LockProbe::Absent,
        "a missing path reads Absent"
    );

    let guard = acquire_at(&path, "launcher", 1).expect("uncontended acquire");
    assert_eq!(
        probe_at(&path),
        LockProbe::Held,
        "an exclusive holder reads Held while its guard is alive"
    );
    drop(guard);
}
