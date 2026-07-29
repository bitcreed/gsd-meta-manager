---
phase: 17-supervisor-detach-kill-switch-dry-run-opt-in-gate
verified: 2026-07-29T22:30:00Z
status: passed
score: 5/5 must-haves verified (ROADMAP success criteria), 6/6 review blockers closed
behavior_unverified: 0
overrides_applied: 0
re_verification: no — initial verification, executed after the 17-08 gap-closure pass
---

# Phase 17: Supervisor — Detach, Kill Switch, Dry-Run, Opt-In Gate — Verification Report

**Phase Goal:** A run is stoppable, survivable, single-instance, previewable, and impossible to
start against a project that did not opt in
**Verified:** 2026-07-29T22:30:00Z (at HEAD `0e3a5f7`, i.e. with plan 17-08's gap closure merged)
**Status:** passed
**Re-verification:** No — this is the first verification pass, run after `17-REVIEW.md` found six
blockers and plan 17-08 closed them.

## Context

`17-REVIEW.md` (2026-07-29, `status: issues_found`) found six BLOCKER findings (CR-01..CR-06)
that made the phase goal's own word "stoppable" false in three independently reachable paths, plus
17 WARNING findings (WR-01..WR-17). Plan 17-08 is the gap-closure pass; its plan and SUMMARY claim
all six blockers plus WR-01 are closed and the 16 remaining warnings are deliberately deferred.

This verification does not trust that SUMMARY. Every blocker's claimed fix was read from the
current source, and the regression tests that are claimed to prove each fix were re-run in this
session (not merely re-quoted from the SUMMARY).

## Goal Achievement — ROADMAP Success Criteria

| # | Criterion | Status | Evidence |
|---|-----------|--------|----------|
| 1 | Pressing stop during an active run leaves no `claude` process, no grandchild build/server process, no zombie — verified 15s later | ✓ VERIFIED | `cargo test --test driver_kill` — 3 passed (15.16s), including `stopping_a_run_leaves_no_claude_no_grandchild_and_no_zombie`. **Additionally** `cargo test --test driver_kill_startup` — 1 passed (0.23s), proving the CR-01 case (stop during `Executor::start()`) that the original suite could not reach. |
| 2 | Closing the TUI mid-run leaves the run going; reopening shows it live with its current step | ✓ VERIFIED | `cargo test --test driver_reattach` — 3 passed (6.10s): `a_run_outlives_the_process_that_spawned_it`, `a_fresh_scan_finds_the_orphaned_run_live_with_its_last_journal_step`, `a_run_killed_without_an_ending_is_reported_crashed_and_nothing_on_disk_is_repaired`. |
| 3 | A dry-run reports the GSD commands, diffstat, and push refspecs, and performs zero git writes | ✓ VERIFIED | `cargo test --test driver_dry_run` — 4 passed (0.05s), including `a_dry_run_leaves_the_git_directory_byte_identical`. Non-regression of the two new start-side refusals confirmed by `drive_still_previews_when_there_is_no_run_id_because_a_preview_creates_no_run` (in `cargo test --lib driver::`). |
| 4 | A run cannot be started against a non-opted-in project, and a live run performs no write/spawn/git op against any other registered project | ✓ VERIFIED | `cargo test --test driver_tracer` — 4 passed (`a_drive_against_a_project_with_no_opt_in_record_is_refused_and_writes_nothing`), `cargo test --test driver_optin` — 3 passed (`a_run_against_one_project_leaves_every_other_project_byte_identical`), `cargo test --test spawn_seam_guard` — 5 passed (single-gate-call-site audit). |
| 5 | A second attempt to drive the same project reports which run holds the lock | ✓ VERIFIED | `cargo test --test driver_lock` — 4 passed (6.07s), including `a_second_drive_reports_which_run_holds_the_lock`. |

**Score:** 5/5 ROADMAP success criteria verified by tests actually re-run in this session, not by
re-quoting the SUMMARY.

## The Six Review Blockers (CR-01..CR-06) — Verified Against Source, Not Claim

Each row: the defect `17-REVIEW.md` found, the fix read directly from the current tree, and the
regression test re-run in this session.

| Finding | Fix verified in source | Regression test re-run this session | Result |
|---|---|---|---|
| **CR-01** — SIGTERM during `Executor::start()` was swallowed until the drain loop | `src/driver/run.rs:566-578` wraps `executor.start(...)` in a `biased tokio::select!` whose first arm is `term.recv()`, calling `shutdown_during_startup` (lines 337-404), which signals the agent's pgid via `kill::signal_group` (TERM→5s grace→KILL→2s reap), journals `terminate_signal_shutdown`, and finishes with `"killed"`. `src/executor/claude.rs:405-421` publishes the agent's pgid via `observing_spawn`/`spawn_observer` immediately after `child.id()`, before the capability gate is awaited — closing the exact window CR-01 named. | `driver_kill_startup::a_stop_during_agent_startup_tears_down_the_agent_group_and_records_a_killed_run` | **PASS** (0.23s) — 3-process integration test (driver, agent stand-in, grandchild) using `tests/fixtures/fake-claude-silent.sh`, a real fixture that never emits `system/init`, confirmed present and executable. |
| **CR-02** — `stop_run` signalled the agent-writable recorded `pgid` with no validation | `src/driver/kill.rs:253-268` (`resolve_signal_target`) requires `liveness::process_group(pid)` (read from `/proc/<pid>/stat`, `src/driver/liveness.rs:234-241`) to agree with the recorded value; disagreement or an unreadable kernel group returns `Err` naming D-04, and `stop_run` (`kill.rs:315-366`) signals only the resolved (kernel) value. | `driver::kill::a_stop_whose_record_names_another_groups_pgid_signals_nothing_and_the_decoy_survives` | **PASS** — end-to-end negative: a decoy process in an unrelated group, named by a tampered `pgid`, survives the stop untouched. |
| **CR-03** — the detached spawn dropped `--config`, so the child loaded a different registry | `src/driver/spawn.rs:58-80` (`drive_argv`) now takes `config_path: &Path` as its first parameter and emits `--config <path>` before the subcommand; `src/app.rs:917-932` clones `self.ctx.config_path` and passes it into `drive_argv` at the call site. | `driver::spawn::the_drive_argv_parses_back_into_a_drive_command_that_clap_accepts` | **PASS** — proves the flag is in a position clap actually accepts, not merely present in the vector. |
| **CR-04** — a run with `run_id: None` was invisible to liveness; the inline `--run-id=VALUE` spelling was never matched | `src/driver/mod.rs:215-217` refuses a real run with `run_id: None` (`DriveError::RunIdRequired`) before `dispatch`; `src/driver/run.rs:499` re-asserts the same refusal as a second line of defence. `src/driver/liveness.rs:139-144` (`cmdline_names_run`) now matches both `--run-id=VALUE` and the adjacent-pair spelling. | `driver::mod::drive_refuses_a_real_run_that_carries_no_run_id_without_touching_disk`, `driver::liveness::the_inline_run_id_spelling_is_not_invisible_to_the_probe` | **PASS** (both, via `cargo test --lib driver::`) — the refusal test also asserts `.planning/meta-manager` was never created. |
| **CR-05** — an undeterminable liveness (off-Linux) read as "already gone" and "crashed" | `src/driver/liveness.rs:72-83` (`pub enum Liveness { Alive, Dead, Unknown }`) plus `LIVENESS_SUPPORTED` (a `const`, not a `#[cfg]`). `kill::stop_decision` (`kill.rs:226-232`) maps `Unknown` to `Undeterminable`, never `AlreadyGone`. `reconcile::classify` (`reconcile.rs:147-...`) adds `RunVerdict::LivenessUnknown`, never `CrashedWithoutEnding`, for `(None, Unknown)`. `driver::platform_refusal` (`mod.rs:140-150`) refuses to *start* a real run where `LIVENESS_SUPPORTED` is false, exempting `--dry-run`. | `driver::kill::an_undeterminable_liveness_never_answers_already_gone`, `driver::reconcile::an_undeterminable_liveness_is_never_classified_as_a_crash`, `driver::mod::the_platform_gate_refuses_a_real_run_where_liveness_cannot_be_determined` | **PASS** (all three, via `cargo test --lib driver::`) — pure functions, testable on Linux for the otherwise-unreachable non-Linux branch. |
| **CR-06** — unregistering a project abandoned a live agent with no stop path | `src/ui/screens/delete_confirm.rs:76-106` (`do_remove_project`) refuses at the top of the function — before any registry mutation — when the observed run's `liveness != Liveness::Dead` (covering both `Alive` and `Unknown`), naming the alias, the run id, and the `x` stop key. | `unregistering_a_project_with_a_live_run_is_refused_and_leaves_every_map_intact`, `unregistering_a_project_whose_liveness_is_undeterminable_is_refused_too`, plus two control arms (`…_with_no_run_still_removes_it`, `…_whose_run_crashed_still_removes_it`) | **PASS** (all four, via `cargo test --lib ui::screens::delete_confirm`) — confirmed the refusal is not a blanket refusal. |

All six commits (`f858a9c`, `5fdcc37`, `744c137`, `68beab7`, `3321d18`, `8129964`, `2e41287`,
plus `402775e` for a fallout fix) exist in `git log` and are exactly what the SUMMARY claims.

## Required Artifacts

| Artifact | Expected | Status | Details |
|---|---|---|---|
| `src/driver/liveness.rs` | Tri-state `Liveness`, `LIVENESS_SUPPORTED`, `process_group`, inline-spelling match | ✓ VERIFIED | All present, wired into `kill.rs` and `reconcile.rs`; 5 new tests pass including two that prove the `/proc/<pid>/stat` field parse against a real child and its control arm. |
| `src/driver/kill.rs` | `resolve_signal_target`, kernel-confirmed stop | ✓ VERIFIED | Present, wired; `stop_run` uses only the resolved target, never the raw `pgid` parameter, for both TERM and KILL. |
| `src/driver/run.rs` | Terminate signal raced against startup; `shutdown_during_startup` | ✓ VERIFIED | Three `biased select!` blocks with `term.recv()` first (startup, close_input, drain loop); startup budget (5s+2s+2s=9s) asserted `<= DRIVER_TEARDOWN_GRACE` (12s) by a unit test. |
| `src/executor/claude.rs` | `observing_spawn`, pgid published before gate await | ✓ VERIFIED | `spawn_observer` field present; send happens immediately after `child.id()`, one statement before `gate_rx.await`. |
| `tests/driver_kill_startup.rs` | Three-process proof | ✓ VERIFIED | Exists, executable via `cargo test --test driver_kill_startup`, passes. Asserts `ExitedOnTerminate` exactly (not a tolerant `matches!` including `ExitedAfterKill`), asserts `ended_at`/`"killed"` on disk, and asserts `exec_started` absent from the journal as the non-vacuity precondition. |
| `tests/fixtures/fake-claude-silent.sh` | Silent stand-in, backgrounds a grandchild, no `system/init` | ✓ VERIFIED | Present, `chmod +x` (mode 775), content confirmed: backgrounds `sleep 600`, writes both pids to the pidfile, never emits `system/init`/`subtype`. |

## Key Link Verification

| From | To | Via | Status |
|---|---|---|---|
| `src/driver/kill.rs` | `src/driver/liveness.rs` | `stop_run` resolves liveness and signal target from `liveness::probe`/`process_group`, never the raw record | ✓ WIRED |
| `src/driver/run.rs` | `src/executor/claude.rs` | `execute_run` builds a oneshot, calls `.observing_spawn(pgid_tx)` on the executor before `.start()` | ✓ WIRED |
| `src/app.rs` | `src/driver/spawn.rs` | `start_driver_run` clones `self.ctx.config_path` and passes it as `drive_argv`'s first argument | ✓ WIRED |
| `src/ui/screens/delete_confirm.rs` | `src/driver/reconcile.rs` | `do_remove_project` reads `ctx.observed_runs.get(alias)` and checks `.liveness != Liveness::Dead` | ✓ WIRED |

## Requirements Coverage

All 5 requirement IDs are declared across the phase's 8 plans and traced to the ROADMAP row
`CTRL-01 | CTRL-02 | CTRL-03 | CTRL-04 | CTRL-05 | Phase 17: Supervisor | Complete` in
`.planning/REQUIREMENTS.md`. No orphaned requirements found for Phase 17.

| Requirement | Declared in | Status | Evidence |
|---|---|---|---|
| CTRL-01 (stop terminates entire tree) | 17-06, 17-07, 17-08 | ✓ SATISFIED | `driver_kill` + `driver_kill_startup` (CR-01), `resolve_signal_target` (CR-02) |
| CTRL-02 (dry-run preview, zero writes) | 17-04, 17-08 | ✓ SATISFIED | `driver_dry_run` (4 tests); non-regression proven by the two new start-side refusals being positioned after the dry-run branch |
| CTRL-03 (opt-in gate at spawn seam) | 17-01, 17-03, 17-07, 17-08 | ✓ SATISFIED | `driver_tracer`, `spawn_seam_guard` (single gate call site), `drive_argv` config carry-through (CR-03) |
| CTRL-04 (survives TUI close, reconciles on restart) | 17-01, 17-05, 17-07, 17-08 | ✓ SATISFIED | `driver_reattach`, `RunIdRequired` refusal + inline-spelling match (CR-04) |
| CTRL-05 (single instance via OS-level lock) | 17-02, 17-08 | ✓ SATISFIED | `driver_lock` (`a_second_drive_reports_which_run_holds_the_lock`); `read_run_facts` clamp rejects a truncated/zero pid/pgid |

## Anti-Patterns Found

None. `TBD`/`FIXME`/`XXX`/`TODO`/`HACK`/`PLACEHOLDER` grep against every file this plan touched
(`src/driver/{liveness,kill,reconcile,run,mod,spawn}.rs`, `src/error.rs`, `src/cli.rs`, `src/app.rs`,
`src/executor/claude.rs`, `src/ui/screens/delete_confirm.rs`) returned zero matches.

## Behavioral Spot-Checks (Re-Run This Session, Not Copied from SUMMARY)

| Behavior | Command | Result | Status |
|---|---|---|---|
| Startup-stop tears down tree and records `killed` (CR-01) | `rtk proxy cargo test --test driver_kill_startup` | 1 passed, 0.23s | ✓ PASS |
| Kernel-vs-record disagreement refuses to signal (CR-02) | `rtk proxy cargo test --lib driver::kill` | 8 passed | ✓ PASS |
| Config carried into detached spawn (CR-03) | `rtk proxy cargo test --lib driver::spawn` | 5 passed | ✓ PASS |
| Missing/inline run-id handling (CR-04) | `rtk proxy cargo test --lib driver::` | 46 passed | ✓ PASS |
| Undeterminable liveness never "gone"/"crashed" (CR-05) | `rtk proxy cargo test --lib driver::` | 46 passed | ✓ PASS |
| Unregister refuses on live/undeterminable run (CR-06) | `rtk proxy cargo test --lib ui::screens::delete_confirm` | 4 passed | ✓ PASS |
| Original kill-switch proof still holds | `rtk proxy cargo test --test driver_kill` | 3 passed, 15.16s | ✓ PASS |
| Reattach/reconciliation | `rtk proxy cargo test --test driver_reattach` | 3 passed, 6.10s | ✓ PASS |
| Dry-run preview inertness | `rtk proxy cargo test --test driver_dry_run` | 4 passed | ✓ PASS |
| Opt-in isolation, single-gate audit | `driver_tracer` / `driver_optin` / `spawn_seam_guard` | 4 / 3 / 5 passed | ✓ PASS |
| Single-instance lock | `rtk proxy cargo test --test driver_lock` | 4 passed | ✓ PASS |
| Full suite | `cargo test` | **542 passed (17 suites, 58.58s)** | ✓ PASS — matches the SUMMARY's claimed figure exactly |
| Project gate | `cargo clippy -- -D warnings` | exit 0, no issues | ✓ PASS |
| All-targets clippy delta | `rtk proxy sh -c "cargo clippy --all-targets 2>&1 \| grep '^warning: ' \| grep -v generated"` | exactly 5 lines: `src/browser.rs` ×3, `src/project_creator.rs` ×1, `src/state_reader/mod.rs` ×1 | ✓ PASS — matches claim, out of scope per instructions |
| Scope fence | `git diff --name-only 5ad68e7..HEAD` | 16 files, all in `files_modified` plus the SUMMARY itself | ✓ PASS |

## Deferred (WR-02..WR-17)

Per the verification instructions, these are explicitly out of scope and are not gaps. Spot-checked
two of them directly against source to confirm the deferral claim is honest rather than merely
asserted:

- **WR-11** (silent return from `stop_driver_run` with no event channel): confirmed
  `src/app.rs:1005-1020` — the silent-return branch is unchanged; `stop_driver_run` was not edited
  beyond the surrounding `is_live()`/`liveness` sweep this plan required.
- **WR-05** (`ReapArm` has no behavioural effect): confirmed unchanged; `stop_run` still uses
  `arm.reaper()` only in the escalation's `tracing::warn!` field.

All 16 deferred items carry a one-line reason in 17-08-PLAN.md's scope fence and are individually
re-confirmed untouched in 17-08-SUMMARY.md's Deferred-Warning Audit; this verification found no
reason to doubt that audit for the two items spot-checked.

## Minor Observation (Not a Gap)

`ClaudeExecutor.spawn_observer` uses `tokio::sync::Mutex` rather than the `std::sync::Mutex` the
plan's action text specified (`src/executor/claude.rs:73,288`). This is consistent with — not a
violation of — this project's own `CLAUDE.md` guidance ("Never use `std::sync::Mutex` in async
code... Use `tokio::sync::Mutex`"), and the lock is held only briefly across an `.await` inside
`start_run`. Not flagged as a gap; noted only because the SUMMARY's Deviations section did not
mention this substitution.

## Regression-Test Honesty (Explicitly Requested Scrutiny)

17-08-SUMMARY.md distinguishes three fixes "observed RED in this session" (CR-01, CR-02, and the
CR-04 inline-spelling test) from four whose pre-fix failure mode is a compile error because they
reference symbols absent from the base tree (`make_run_record`'s `pgid` param, `platform_refusal`/
`LIVENESS_SUPPORTED`, `observing_spawn`, `Liveness`/`RunVerdict::LivenessUnknown`). This distinction
was checked against the actual source and is drawn honestly:

- `Liveness`, `LIVENESS_SUPPORTED`, `RunVerdict::LivenessUnknown`, `platform_refusal`,
  `observing_spawn`, `drive_argv`'s new `config_path` parameter, and `make_run_record`'s new `pgid`
  parameter are all genuinely new symbols that did not exist before this plan — a test naming any
  of them cannot compile against a tree that lacks them, so "compile-error red" is the correct,
  honest classification rather than an excuse to skip demonstrating a red run.
- The three tests claimed as observed-red (CR-01, CR-02, and the inline-spelling test) exercise
  *behavior* through symbols/call shapes that could exist in some form pre-fix (an existing
  `stop_run`, an existing `cmdline_names_run`, an existing `execute_run`), so a compile-time
  argument alone would not establish the fix — an actual red run was the only way to prove it, and
  the SUMMARY quotes verbatim failing assertions and elapsed times (12.07s for CR-01, consistent
  with the driver outlasting `DRIVER_TEARDOWN_GRACE` and being SIGKILLed) that are specific enough
  to not be fabricated boilerplate.

This verification did not re-revert the fixes to re-observe the red runs (that would require
mutating and restoring the working tree, which is out of scope for a read-only verification pass),
but the claim's internal structure is consistent with what a genuine red/green pair would produce,
and the green side of every one of these tests was independently re-run in this session and passed.

## Gaps Summary

None. All five ROADMAP success criteria are verified by tests re-run in this session. All six
`17-REVIEW.md` blockers (CR-01..CR-06) are closed with fixes read directly from source, not merely
claimed by the SUMMARY, and each is backed by a named regression test that was re-run and passed.
The 16 deferred warnings are legitimately out of scope for this phase per the phase's own scope
fence and the verification instructions. No debt markers, no orphaned requirements, no broken key
links, and the project gate (`cargo build && cargo test && cargo clippy -- -D warnings`) is green
with the pre-existing 5-lint `--all-targets` delta unchanged.

---
_Verified: 2026-07-29T22:30:00Z_
_Verifier: Claude (gsd-verifier)_
