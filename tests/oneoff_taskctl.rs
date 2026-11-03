//! `gsd-meta-manager taskctl report` through the real binary (quick 261103-f5y,
//! Task 1 tracer): CLI -> validator -> atomic file -> host re-validated read.

#![cfg(unix)]

use std::path::Path;
use std::process::{Command, Output};

use gsd_meta_manager::oneoff::report::{read_validated, ReportStatus};
use gsd_meta_manager::oneoff::{REPORT_DIR_ENV, REPORT_FILE};
use tempfile::TempDir;

fn taskctl(dir: Option<&std::ffi::OsStr>, args: &[&str]) -> Output {
    let mut cmd = Command::new(env!("CARGO_BIN_EXE_gsd-meta-manager"));
    cmd.env_clear().arg("taskctl").arg("report").args(args);
    if let Some(dir) = dir {
        cmd.env(REPORT_DIR_ENV, dir);
    }
    cmd.output().expect("run the binary")
}

fn valid_args() -> Vec<&'static str> {
    vec![
        "--status",
        "done",
        "--headline",
        "Fixed the parser",
        "--changed",
        "src/a.rs",
        "--changed",
        "src/b.rs",
        "--verified",
        "cargo test: 812 passed",
        "--followup",
        "tidy the tests",
    ]
}

#[test]
fn a_valid_report_is_written_and_reads_back_through_the_host_validator() {
    let dir = TempDir::new().unwrap();
    let out = taskctl(Some(dir.path().as_os_str()), &valid_args());
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    assert!(String::from_utf8_lossy(&out.stdout).contains("recorded (status=done)"));
    let report = read_validated(&dir.path().join(REPORT_FILE)).unwrap();
    assert_eq!(report.status, ReportStatus::Done);
    assert_eq!(report.headline, "Fixed the parser");
    assert_eq!(report.changed, ["src/a.rs", "src/b.rs"]);
    assert_eq!(report.verified, ["cargo test: 812 passed"]);
    assert_eq!(report.followups, ["tidy the tests"]);
}

#[test]
fn a_bad_report_exits_2_one_line_per_problem_and_leaves_the_earlier_file_alone() {
    let dir = TempDir::new().unwrap();
    assert_eq!(
        taskctl(Some(dir.path().as_os_str()), &valid_args())
            .status
            .code(),
        Some(0)
    );
    let path = dir.path().join(REPORT_FILE);
    let before = std::fs::read(&path).unwrap();
    let long = "x".repeat(121);
    let out = taskctl(
        Some(dir.path().as_os_str()),
        &[
            "--status",
            "failed",
            "--headline",
            &long,
            "--verified",
            "no colon here",
            "--changed",
            "1",
            "--changed",
            "2",
            "--changed",
            "3",
            "--changed",
            "4",
            "--changed",
            "5",
            "--changed",
            "6",
        ],
    );
    assert_eq!(out.status.code(), Some(2), "{out:?}");
    let stderr = String::from_utf8_lossy(&out.stderr);
    let problems: Vec<&str> = stderr
        .lines()
        .filter(|l| l.starts_with("taskctl report: --"))
        .collect();
    assert_eq!(problems.len(), 3, "{stderr}");
    assert!(
        stderr.contains("--headline: must be 1-120 characters"),
        "{stderr}"
    );
    assert!(stderr.contains("--changed: at most 5"), "{stderr}");
    assert!(stderr.contains("--verified #1"), "{stderr}");
    assert_eq!(
        std::fs::read(&path).unwrap(),
        before,
        "the earlier report changed"
    );
}

#[test]
fn a_second_valid_report_replaces_the_first() {
    let dir = TempDir::new().unwrap();
    taskctl(Some(dir.path().as_os_str()), &valid_args());
    let out = taskctl(
        Some(dir.path().as_os_str()),
        &["--status", "partial", "--headline", "Half of it"],
    );
    assert_eq!(out.status.code(), Some(0), "{out:?}");
    let report = read_validated(&dir.path().join(REPORT_FILE)).unwrap();
    assert_eq!(report.status, ReportStatus::Partial);
    assert!(report.changed.is_empty());
}

#[test]
fn a_missing_or_relative_report_dir_exits_1() {
    let missing = taskctl(None, &valid_args());
    assert_eq!(missing.status.code(), Some(1), "{missing:?}");
    let relative = taskctl(Some(std::ffi::OsStr::new("relative/dir")), &valid_args());
    assert_eq!(relative.status.code(), Some(1), "{relative:?}");
    let absent = taskctl(
        Some(Path::new("/nonexistent/gsdmm-oneoff-report").as_os_str()),
        &valid_args(),
    );
    assert_eq!(absent.status.code(), Some(1), "{absent:?}");
    assert!(String::from_utf8_lossy(&absent.stderr).contains(REPORT_DIR_ENV));
}

#[test]
fn taskctl_is_absent_from_help() {
    let out = Command::new(env!("CARGO_BIN_EXE_gsd-meta-manager"))
        .env_clear()
        .arg("--help")
        .output()
        .unwrap();
    let help = String::from_utf8_lossy(&out.stdout);
    assert!(help.contains("list"), "positive control: {help}");
    assert!(!help.contains("taskctl"), "{help}");
}
