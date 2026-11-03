//! `gsd-meta-manager taskctl report` (quick 261103-f5y, C-12): the one-off agent's
//! strict report CLI.
//!
//! Exit codes are the contract the prompt states: `0` recorded, `2` a field is out of
//! bounds (one `taskctl report: <problem>` line per problem on stderr, nothing
//! written, an earlier report left untouched), `1` the report directory is missing or
//! not absolute, or the write failed.

use std::path::PathBuf;

use gsd_meta_manager::cli::{TaskctlAction, TaskctlReport};
use gsd_meta_manager::oneoff::report::{write_atomic, Report};
use gsd_meta_manager::oneoff::REPORT_DIR_ENV;

/// Run one `taskctl` verb; returns the exit code.
pub fn run(action: TaskctlAction) -> i32 {
    match action {
        TaskctlAction::Report(args) => report(&args),
    }
}

fn report(args: &TaskctlReport) -> i32 {
    let dir = match std::env::var_os(REPORT_DIR_ENV).map(PathBuf::from) {
        Some(dir) if dir.is_absolute() && dir.is_dir() => dir,
        Some(dir) => {
            eprintln!(
                "taskctl report: {REPORT_DIR_ENV} ({}) is not an absolute path to an existing directory",
                dir.display()
            );
            return 1;
        }
        None => {
            eprintln!(
                "taskctl report: {REPORT_DIR_ENV} is not set; this command runs inside a one-off sandbox only"
            );
            return 1;
        }
    };
    let report = match Report::validate(
        args.status,
        &args.headline,
        &args.changed,
        &args.verified,
        &args.followup,
    ) {
        Ok(report) => report,
        Err(problems) => {
            for problem in problems {
                eprintln!("taskctl report: {problem}");
            }
            eprintln!("taskctl report: nothing was recorded; fix the arguments and run it again");
            return 2;
        }
    };
    match write_atomic(&dir, &report) {
        Ok(_) => {
            println!(
                "taskctl report: recorded (status={})",
                report.status.as_str()
            );
            0
        }
        Err(e) => {
            eprintln!("taskctl report: could not write the report: {e}");
            1
        }
    }
}
