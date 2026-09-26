//! `bounded_output::stdout_within` (Phase 25 review WR-02): a command that
//! hangs is killed at its budget, and a command whose stdout outgrows a pipe
//! buffer is drained rather than mistaken for a hang.
#![cfg(unix)]

use std::process::Command;
use std::time::{Duration, Instant};

use gsd_meta_manager::bounded_output::stdout_within;

fn sh(script: &str) -> Command {
    let mut command = Command::new("sh");
    command.args(["-c", script]);
    command
}

#[test]
fn a_command_that_outlives_its_budget_is_stopped_and_reads_as_none() {
    let started = Instant::now();
    let answer = stdout_within(&mut sh("exec sleep 30"), Duration::from_millis(300));
    assert!(answer.is_none(), "a timed-out command has no answer");
    assert!(
        started.elapsed() < Duration::from_secs(5),
        "the caller got control back at the budget, not when the command ended ({:?})",
        started.elapsed()
    );
}

#[test]
fn stdout_larger_than_a_pipe_buffer_is_drained_while_the_command_runs() {
    // 1 MiB, far past the ~64 KiB a pipe holds: an undrained pipe would block
    // the writer until the budget killed it.
    let answer = stdout_within(
        &mut sh("head -c 1048576 /dev/zero"),
        Duration::from_secs(10),
    )
    .expect("the command finishes well inside its budget");
    assert!(answer.status.success());
    assert_eq!(answer.stdout.len(), 1_048_576);
}

#[test]
fn a_quick_command_reports_its_status_and_stdout() {
    let answer = stdout_within(
        &mut sh("printf 'a\\nb\\n'; exit 3"),
        Duration::from_secs(10),
    )
    .expect("the command finishes well inside its budget");
    assert_eq!(answer.status.code(), Some(3));
    assert_eq!(answer.stdout, b"a\nb\n");
}

#[test]
fn a_command_that_cannot_be_spawned_is_none() {
    let answer = stdout_within(
        &mut Command::new("/nonexistent/gsdmm-no-such-program"),
        Duration::from_secs(1),
    );
    assert!(answer.is_none());
}

#[test]
fn a_leftover_process_holding_stdout_cannot_stall_the_caller() {
    // The shell exits at once, but a background `sleep` inherits stdout and
    // keeps the pipe open: end-of-file never comes within the budget.
    let started = Instant::now();
    let answer = stdout_within(
        &mut sh("sleep 5 & echo started"),
        Duration::from_millis(300),
    );
    assert!(answer.is_none(), "stdout never reached end-of-file in time");
    assert!(
        started.elapsed() < Duration::from_secs(3),
        "the reader was abandoned, not joined ({:?})",
        started.elapsed()
    );
}
