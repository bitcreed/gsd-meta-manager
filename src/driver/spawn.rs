//! The TUI's detached spawn of a driver, and the admission check in front of it
//! (D-01, D-02, D-18).
//!
//! This is the seam at which a run begins with no further human input, and it is
//! the only place in the tree that detaches a child. Everything it does is
//! chosen so that **closing the TUI does not stop the run**: a new process
//! group, three null stdio handles, no kill-on-drop, and a reaping task whose
//! absence would be the only way a multi-hour run could still leave a zombie.
//!
//! The spawned program is `std::env::current_exe()` — this same binary's `drive`
//! subcommand — which buys three things at once. One build and one version, so a
//! TUI and its drivers can never disagree about how a project is read. The
//! opt-in gate runs **in the child**, where D-16 put it, so a hand-typed `drive`
//! and a TUI-initiated one are refused by the same code. And a spawn failure is
//! synchronous and reportable here, leaving genuinely nothing on disk (D-03).

use std::ffi::OsString;
use std::os::unix::process::CommandExt;
use std::path::Path;
use std::process::Stdio;

/// The argv of a `drive` invocation, as a pure function of what it is driving.
///
/// It is its own function so a test can assert what it does **not** contain.
/// `src/cli.rs` carries two hidden development flags — `--claude-program` and
/// `--claude-args` — that point the driver's executor at a checked-in shell
/// stand-in instead of the real agent. The TUI must never be able to emit
/// either: a spawn seam that could hand a fixture to a production run is a spawn
/// seam whose behaviour depends on which code path built its arguments.
/// `the_drive_argv_carries_no_development_flag` is what keeps that true, and it
/// can only exist because this construction is separable from the spawn.
///
/// `--run-id` is always present and is always supplied by the caller (D-03): the
/// TUI owns the id so it knows what to look for afterwards, and the driver owns
/// the record because `run.json` carries the driver's own pid and pgid.
///
/// **`--config` is always present too, and its absence was CR-03.** The flag is
/// `global = true` and `App` has carried `ctx.config_path` all along, but this
/// argv dropped it — so a driver spawned from a TUI running with `--config
/// /path/to/other.json` loaded `Config::default_path()` instead, and resolved the
/// alias in a registry the user was not looking at. Two consequences, and the
/// second is the one that matters:
///
/// * The benign one: the alias is absent from the default config, the child
///   refuses at its own gate, and **nothing is visible** — the child's stdio is
///   `/dev/null` while the TUI has already shown an optimistic "Driving {alias}"
///   entry, so the run appears to start and then quietly is not there.
/// * The one that matters: the default config happens to carry the **same alias**
///   pointing at a different project, with an opt-in record of its own. The gate
///   passes, and an autonomous agent with git and push rights runs in the wrong
///   repository.
///
/// It is emitted **before** the subcommand, which is the form
/// `tests/driver_kill.rs`, `driver_lock.rs`, `driver_tracer.rs` and
/// `driver_reattach.rs` have been using against the real binary since plan 17-06.
/// Matching it here removes the last axis on which the production argv and the
/// tested argv differed.
pub fn drive_argv(
    config_path: &Path,
    alias: &str,
    command: &str,
    run_id: &str,
    goal: Option<&str>,
) -> Vec<OsString> {
    let mut argv = vec![
        OsString::from("--config"),
        config_path.as_os_str().to_owned(),
        OsString::from("drive"),
        OsString::from(alias),
        OsString::from("--command"),
        OsString::from(command),
        OsString::from("--run-id"),
        OsString::from(run_id),
    ];
    if let Some(goal) = goal {
        argv.push(OsString::from("--goal"));
        argv.push(OsString::from(goal));
    }
    argv
}

/// Spawn a driver detached from this process, returning its pid.
///
/// Every call below is load-bearing and the comment on each says which failure
/// it prevents:
///
/// * **`current_dir(project_root)`** — the project root itself, **never a
///   worktree**. OQ4 was resolved empirically in Phase 17: `claude --worktree`
///   exists and works. Phase 19's D-21 is the final answer, and it is
///   **declined**, for four recorded reasons:
///   1. `claude --worktree` writes **inside the repository working directory**,
///      where `watcher.rs::extract_project_root` resolves the worktree's own
///      `.planning/` as a phantom project on the dashboard.
///   2. The journal, `run.json`, the `runs/active` pointer and the reattachment
///      scan are all keyed to `<project>/.planning/`. A worktree forks every one
///      of them, and the reattach path — a hard requirement — would have to
///      learn which copy is authoritative.
///   3. A driven GSD run whose entire purpose is to advance `.planning/` **in
///      the project the user is watching** is not a thing to isolate from that
///      project. The isolation would defeat the feature.
///   4. The concrete hazard that motivated isolation — the agent's `git add -A`
///      sweeping `.claude/worktrees/` or the runs directory into a commit — is
///      closed **mechanically and differently** (D-22): `envelope::hooks`'
///      `pre-commit` refuses the staged path, its `pre-push` backstop refuses
///      the same path in a commit that skipped the first hook, and
///      `envelope::hooks::write_exclude_block` makes the paths ignored in a file
///      the agent cannot commit. "We declined isolation" would not have been an
///      answer to that hazard; those three are.
/// * **Three null stdio handles** — all three, not two. A detached child holding
///   an inherited terminal handle is a child that writes over the TUI's frame
///   buffer, and stdin is the one people forget.
/// * **`kill_on_drop(false)`, written out** — the default, and the default is
///   *load-bearing here*. A clean TUI shutdown drops the returned `Child`, and
///   an accidental `kill_on_drop(true)` would silently destroy the entire point
///   of this phase: the run would die exactly when the user closed the window.
///   `src/executor/claude.rs:353` calls its `KillOnDrop` wrapper a "backstop
///   only, never the teardown story"; here the wrapper must be **absent**, and
///   this comment says the opposite thing on purpose (D-02).
/// * **`process_group(0)`** — the child becomes its own group leader, which is
///   what makes `RunRecord.pgid == RunRecord.pid` true and what gives the kill
///   switch a group to signal (D-01, D-04).
///
/// The `CLAUDE*` environment scrub `ClaudeExecutor` performs is deliberately
/// **absent**: the child spawned here is this binary, not `claude`, and the
/// scrub already happens at the real agent spawn one process deeper. Doing it
/// twice would put the same rule in two places that could drift.
///
/// **Must be called from inside a Tokio runtime.** It spawns both the child and
/// the reaping task through Tokio.
pub fn spawn_detached(project_root: &Path, argv: &[OsString]) -> std::io::Result<u32> {
    let program = std::env::current_exe()?;

    let mut cmd = tokio::process::Command::new(program);
    cmd.args(argv);
    // The project root, never a worktree (D-21).
    cmd.current_dir(project_root);
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::null());
    cmd.stderr(Stdio::null());
    // Written out because the default is load-bearing, not because it changes
    // anything. See this function's doc (D-02).
    cmd.kill_on_drop(false);
    // `process_group` is `std::os::unix::process::CommandExt`, so it is reached
    // through the std command underneath the Tokio one.
    cmd.as_std_mut().process_group(0);

    let mut child = cmd.spawn()?;
    let pid = child.id().ok_or_else(|| {
        std::io::Error::other(
            "the spawned driver's pid was not readable, so there is no handle to record",
        )
    })?;

    // A detached task whose whole body is a reap. Both arms of the parent
    // question are covered by it (D-02):
    //
    // * While the TUI is still the parent, this is what stops a multi-hour run
    //   accumulating a zombie in the process table, and it gives the TUI an
    //   in-session completion signal without polling for one.
    // * If the TUI exits first, the driver is reparented to init and reaped
    //   there, and this task simply dies with the process that owned it.
    tokio::spawn(async move {
        let _ = child.wait().await;
    });

    Ok(pid)
}

/// Why a spawn was refused by the concurrency cap.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConcurrencyRefusal {
    /// How many runs the scan found live across every registered project.
    pub live: usize,
    /// The configured cap, `Preferences.driver_max_concurrent`.
    pub max: usize,
}

impl std::fmt::Display for ConcurrencyRefusal {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "{} driver run{} already live and the limit is {} — stop one, or raise \
             `preferences.driver_max_concurrent` in config.json",
            self.live,
            if self.live == 1 { " is" } else { "s are" },
            self.max
        )
    }
}

impl std::error::Error for ConcurrencyRefusal {}

/// Whether a new run may start, given how many are already live (D-18).
///
/// A **pure** function, so the policy is unit-testable without a TUI, a process
/// or a disk. The count it compares against comes from the same reconciliation
/// scan the dashboard uses, which is why this cap was almost free to ship: the
/// enumeration already existed.
///
/// Three things D-18 settles and this function encodes:
///
/// * The fleet dimension is a **count**, not a second identity. One driver per
///   project stays the model; REQUIREMENTS defers fleet-level driving past v2.0.
/// * The default is **1** because the 5h/7d Claude quota is shared across every
///   surface the user has. N concurrently driven projects burn one quota N times
///   over, which is the failure PITFALLS Pitfall 4 names.
/// * **Phase 20 owns the policy around this count**; Phase 17 ships the count.
///   A queue, a park, a per-project priority — none of that belongs here.
///
/// A `max` of `0` refuses everything. That is the honest reading of the setting
/// and not a bug: `Preferences::default` yields 1 and the serde default names a
/// function precisely so a derived `usize::default()` can never produce this by
/// accident, so a zero on disk was typed by a user who meant it.
pub fn admit(live_count: usize, max: usize) -> Result<(), ConcurrencyRefusal> {
    if live_count >= max {
        return Err(ConcurrencyRefusal {
            live: live_count,
            max,
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The config path these tests pin, as a path rather than a string so the
    /// `OsString` conversion is the one production performs.
    fn config_path() -> std::path::PathBuf {
        std::path::PathBuf::from("/tmp/gsd-test/config.json")
    }

    #[test]
    fn the_drive_argv_parses_back_into_a_drive_command_that_clap_accepts() {
        use clap::Parser;

        const RUN_ID: &str = "2026-07-29T12-00-00Z-aaaa";

        let argv = drive_argv(&config_path(), "demo", "/gsd-progress", RUN_ID, None);

        // **What this buys over the pinned vector below.** The pinned vector
        // proves the flag is *present*; only parsing it back proves the
        // **position** is one clap accepts. A global flag emitted where the
        // parser rejects it would make every spawned driver exit non-zero — and
        // do so **silently**, because the child's three stdio handles are null,
        // so the error message goes nowhere and the TUI has already drawn an
        // optimistic "Driving {alias}" entry.
        let mut full = vec![std::ffi::OsString::from("gsd-meta-manager")];
        full.extend(argv);
        let cli = crate::cli::Cli::parse_from(&full);

        assert_eq!(
            cli.config.as_deref(),
            Some(config_path().as_path()),
            "the spawned driver must load the registry the TUI is looking at, not \
             Config::default_path() (CR-03)"
        );

        match cli.command {
            Some(crate::cli::Commands::Drive {
                alias,
                command,
                run_id,
                dry_run,
                ..
            }) => {
                assert_eq!(alias, "demo");
                assert_eq!(command, "/gsd-progress");
                assert_eq!(run_id.as_deref(), Some(RUN_ID));
                assert!(!dry_run, "the TUI spawns real runs, never previews");
            }
            other => panic!(
                "the argv must parse back into the Drive command it claims to be, \
                 got: {}",
                if other.is_some() {
                    "a different subcommand"
                } else {
                    "no subcommand at all"
                }
            ),
        }
    }

    #[test]
    fn the_drive_argv_carries_no_development_flag() {
        let argv = drive_argv(
            &config_path(),
            "demo",
            "/gsd-progress",
            "2026-07-29T12-00-00Z-aaaa",
            Some("ship it"),
        );

        // The fixture-seam constraint plan 17-01 established: the TUI can never
        // point a spawned driver at a stand-in. Asserting on the `--claude`
        // prefix rather than on the two flag names means a third development
        // flag added later is caught by this test on the day it lands.
        assert!(
            !argv
                .iter()
                .any(|arg| arg.to_string_lossy().starts_with("--claude")),
            "the TUI's argv must carry no development flag, got: {argv:?}"
        );

        let rendered: Vec<String> = argv
            .iter()
            .map(|arg| arg.to_string_lossy().into_owned())
            .collect();
        assert_eq!(
            rendered,
            vec![
                "--config",
                "/tmp/gsd-test/config.json",
                "drive",
                "demo",
                "--command",
                "/gsd-progress",
                "--run-id",
                "2026-07-29T12-00-00Z-aaaa",
                "--goal",
                "ship it",
            ]
        );
    }

    #[test]
    fn the_drive_argv_omits_the_goal_flag_entirely_when_there_is_no_goal() {
        let argv = drive_argv(
            &config_path(),
            "demo",
            "/gsd-progress",
            "2026-07-29T12-00-00Z-aaaa",
            None,
        );
        assert!(
            !argv.iter().any(|arg| arg == "--goal"),
            "an absent goal must omit the flag, never pass an empty string — an \
             empty `--goal` would be recorded verbatim into RunRecord.goal and \
             read later as a goal the user typed"
        );
        assert_eq!(argv.len(), 8);
    }

    #[test]
    fn admit_refuses_when_the_live_count_already_meets_the_cap() {
        let refusal = admit(1, 1).expect_err("one live run already meets a cap of one");
        assert_eq!(refusal.live, 1);
        assert_eq!(refusal.max, 1);
        // The message is what the user reads, so it must name both numbers and
        // the setting to change.
        let rendered = refusal.to_string();
        assert!(rendered.contains('1'), "got: {rendered}");
        assert!(
            rendered.contains("driver_max_concurrent"),
            "the refusal must name the setting that governs it, got: {rendered}"
        );

        // `>=` and not `>`: a cap of two with two live runs is met, not exceeded.
        assert!(admit(2, 2).is_err());
        assert!(admit(5, 2).is_err());
    }

    #[test]
    fn admit_allows_the_first_run_under_the_default_cap_of_one() {
        assert!(admit(0, 1).is_ok());
        assert!(admit(1, 2).is_ok());
        assert_eq!(
            crate::config::Preferences::default().driver_max_concurrent,
            1,
            "the default this test names must be the configured default, or the \
             test is asserting against a number nobody uses"
        );
    }
}
