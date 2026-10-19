#![cfg(unix)]
// ============================================================================
// Reader liveness: `derive_status` and `read_driver_run` against REAL lock files
// (Phase 27, plan 27-04; DCTL-03, D-07 as corrected by D-20/D-22).
//
// 27-RESEARCH Answer A lists six situations (R1-R6) in which a reader that derives
// "crashed" from the run lock gets it wrong unless the rule is exactly the D-22
// table. Each test below names the row it certifies. Every one copies a golden
// DIRECTION.md (27-03) into a fresh run dir and arranges `launcher.lock` with the
// same primitives the Phase 28 launcher will use (`acquire_at`), so the probe is
// exercised against the kernel's flock, not against a mock.
//
// The prohibition being enforced: a run is never shown as crashed, or as alive,
// when its liveness could not be established. `LivenessUnknown` is its own answer.
// ============================================================================

use std::fs::File;
use std::path::Path;

use gsd_meta_manager::driver::lock::{acquire_at, run_lock_path, LockProbe};
use gsd_meta_manager::state_reader::driver_frontmatter::{
    derive_status, parse_driver_frontmatter, read_driver_run, DerivedStatus, FrontmatterError,
    RunStatus,
};
use rustix::fs::{flock, FlockOperation};
use rustix::io::Errno;
use tempfile::TempDir;

fn golden(name: &str) -> String {
    let path = format!(
        "{}/tests/fixtures/driverctl/golden/{name}.md",
        env!("CARGO_MANIFEST_DIR")
    );
    std::fs::read_to_string(&path).unwrap_or_else(|e| panic!("read golden {path}: {e}"))
}

/// A fresh run dir whose `DIRECTION.md` is `text`.
fn run_dir_with(text: &str) -> TempDir {
    let dir = TempDir::new().expect("temp run dir");
    std::fs::write(dir.path().join("DIRECTION.md"), text).expect("write DIRECTION.md");
    dir
}

fn with_status(text: &str, from: &str, to: &str) -> String {
    let needle = format!("\nstatus: {from}\n");
    assert!(
        text.contains(&needle),
        "golden has no `status: {from}` line"
    );
    text.replacen(&needle, &format!("\nstatus: {to}\n"), 1)
}

/// Create the lock file and leave it unheld.
fn free_lock(run_dir: &Path) {
    drop(acquire_at(&run_lock_path(run_dir), "test", std::process::id()).expect("take the lock"));
    assert_eq!(
        gsd_meta_manager::driver::lock::probe_at(&run_lock_path(run_dir)),
        LockProbe::Free,
        "dropping the guard must release the lock"
    );
}

fn derived(run_dir: &Path) -> DerivedStatus {
    read_driver_run(run_dir).expect("read_driver_run").derived
}

#[test]
fn r1_a_fresh_run_dir_is_not_launched_not_crashed() {
    // The template golden is `starting` with `launches: 0`.
    let text = golden("template");
    let fm = parse_driver_frontmatter(&text).expect("template parses");
    assert_eq!(fm.status, RunStatus::Starting);
    assert_eq!(fm.launches, 0);

    // No lock file at all.
    let dir = run_dir_with(&text);
    assert_eq!(derived(dir.path()), DerivedStatus::NotLaunched);

    // A lock file that exists but is free.
    free_lock(dir.path());
    assert_eq!(derived(dir.path()), DerivedStatus::NotLaunched);

    // A launcher already holding the lock before it recorded a launch.
    let _held = acquire_at(&run_lock_path(dir.path()), "test", std::process::id()).expect("hold");
    assert_eq!(
        derived(dir.path()),
        DerivedStatus::AsWritten(RunStatus::Starting)
    );
}

#[test]
fn r2_paused_reads_as_written_with_a_free_lock() {
    let text = with_status(&golden("running"), "running", "paused");
    let dir = run_dir_with(&text);
    free_lock(dir.path());

    assert_eq!(
        derived(dir.path()),
        DerivedStatus::AsWritten(RunStatus::Paused),
        "a cleanly paused run has exited and its lock is free by design (D-22 corrects D-07)"
    );
}

#[test]
fn r3_running_with_a_free_lock_is_crashed_and_with_a_held_lock_is_alive() {
    let dir = run_dir_with(&golden("running"));

    // No lock file: the launcher never took it, or the dir was copied elsewhere.
    assert_eq!(
        derived(dir.path()),
        DerivedStatus::Crashed {
            written: RunStatus::Running
        }
    );

    // A free lock file. This is also what a `fresh-exit` gap looks like if Phase 28
    // releases the lock between launches: the false crash its hand-off forbids.
    free_lock(dir.path());
    assert_eq!(
        derived(dir.path()),
        DerivedStatus::Crashed {
            written: RunStatus::Running
        }
    );

    let _held = acquire_at(&run_lock_path(dir.path()), "test", std::process::id()).expect("hold");
    assert_eq!(
        derived(dir.path()),
        DerivedStatus::AsWritten(RunStatus::Running)
    );
}

#[test]
fn r4_starting_after_a_launch_follows_the_lock() {
    let text = golden("starting");
    let fm = parse_driver_frontmatter(&text).expect("starting golden parses");
    assert_eq!(fm.status, RunStatus::Starting);
    assert!(fm.launches > 0, "this row needs a launched run");

    let dir = run_dir_with(&text);
    free_lock(dir.path());
    assert_eq!(
        derived(dir.path()),
        DerivedStatus::Crashed {
            written: RunStatus::Starting
        }
    );

    let _held = acquire_at(&run_lock_path(dir.path()), "test", std::process::id()).expect("hold");
    assert_eq!(
        derived(dir.path()),
        DerivedStatus::AsWritten(RunStatus::Starting)
    );
}

#[test]
fn r5_an_unknown_probe_is_liveness_unknown_never_crashed() {
    let fm = parse_driver_frontmatter(&golden("running")).expect("running golden parses");
    assert_eq!(
        derive_status(&fm, LockProbe::Unknown(Errno::ACCESS)),
        DerivedStatus::LivenessUnknown {
            written: RunStatus::Running
        }
    );

    let launched = parse_driver_frontmatter(&golden("starting")).expect("starting golden parses");
    assert_eq!(
        derive_status(&launched, LockProbe::Unknown(Errno::NOLCK)),
        DerivedStatus::LivenessUnknown {
            written: RunStatus::Starting
        }
    );
}

#[test]
fn r5_an_unreadable_lock_file_gives_liveness_unknown() {
    if rustix::process::geteuid().is_root() {
        eprintln!("skipped: root reads a mode-000 file, so the probe cannot fail");
        return;
    }
    use std::os::unix::fs::PermissionsExt;

    let dir = run_dir_with(&golden("running"));
    free_lock(dir.path());
    let lock = run_lock_path(dir.path());
    std::fs::set_permissions(&lock, std::fs::Permissions::from_mode(0o000)).expect("chmod 000");

    assert_eq!(
        derived(dir.path()),
        DerivedStatus::LivenessUnknown {
            written: RunStatus::Running
        },
        "an EACCES probe is not evidence that the launcher is gone"
    );
}

#[test]
fn r6_another_readers_shared_lock_does_not_fake_liveness() {
    let dir = run_dir_with(&golden("running"));
    free_lock(dir.path());

    // A second reader mid-probe: a shared lock on the same file.
    let other_reader = File::open(run_lock_path(dir.path())).expect("open the lock read-only");
    flock(&other_reader, FlockOperation::NonBlockingLockShared).expect("shared lock");

    assert_eq!(
        derived(dir.path()),
        DerivedStatus::Crashed {
            written: RunStatus::Running
        },
        "a LOCK_SH probe is compatible with another reader's LOCK_SH: it must read Free"
    );
}

#[test]
fn terminal_statuses_read_as_written_with_a_free_lock() {
    let mut cases: Vec<(String, RunStatus)> = vec![
        (golden("stopped"), RunStatus::Stopped),
        (golden("crashed"), RunStatus::Crashed),
        (golden("stalled"), RunStatus::Stalled),
        (
            with_status(&golden("stopped"), "stopped", "parked"),
            RunStatus::Parked,
        ),
        (
            with_status(&golden("stopped"), "stopped", "done"),
            RunStatus::Done,
        ),
    ];
    cases.push((
        with_status(&golden("stopped"), "stopped", "levitating"),
        RunStatus::Unknown("levitating".to_string()),
    ));

    for (text, status) in cases {
        let dir = run_dir_with(&text);
        free_lock(dir.path());
        assert_eq!(
            derived(dir.path()),
            DerivedStatus::AsWritten(status.clone()),
            "{status:?} must read as written"
        );
        // And with no lock file at all.
        std::fs::remove_file(run_lock_path(dir.path())).expect("remove the lock");
        assert_eq!(derived(dir.path()), DerivedStatus::AsWritten(status));
    }
}

#[test]
fn read_driver_run_returns_the_parsed_frontmatter_beside_the_derived_status() {
    let dir = run_dir_with(&golden("running"));
    let view = read_driver_run(dir.path()).expect("read");

    assert_eq!(view.frontmatter.run_id, "golden");
    assert_eq!(view.frontmatter.status, RunStatus::Running);
    assert_eq!(
        view.derived,
        DerivedStatus::Crashed {
            written: RunStatus::Running
        },
        "the written status is untouched: crashed is a reading, not a record"
    );
}

#[test]
fn read_driver_run_leaves_the_run_dir_byte_identical_and_never_creates_the_lock() {
    let dir = run_dir_with(&golden("running"));
    let direction = dir.path().join("DIRECTION.md");
    let before = std::fs::read(&direction).expect("read before");
    let listing = |p: &Path| {
        let mut names: Vec<String> = std::fs::read_dir(p)
            .expect("list")
            .map(|e| e.expect("entry").file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    };
    let names_before = listing(dir.path());

    // Several reads, each deriving `Crashed` from an absent lock.
    for _ in 0..3 {
        read_driver_run(dir.path()).expect("read");
    }

    assert_eq!(std::fs::read(&direction).expect("read after"), before);
    assert_eq!(listing(dir.path()), names_before);
    assert!(
        !run_lock_path(dir.path()).exists(),
        "the reader must never create launcher.lock"
    );
}

#[test]
fn read_driver_run_reports_typed_errors() {
    let missing = TempDir::new().expect("temp dir");
    assert!(matches!(
        read_driver_run(missing.path()),
        Err(FrontmatterError::Io { .. })
    ));

    let bare = run_dir_with("# no frontmatter here\n");
    assert_eq!(
        read_driver_run(bare.path()),
        Err(FrontmatterError::NoFrontmatter)
    );
}
