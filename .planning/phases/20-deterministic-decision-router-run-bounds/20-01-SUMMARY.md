---
phase: 20-deterministic-decision-router-run-bounds
plan: "01"
subsystem: driver
status: complete
tags: [driver, router, bounds, iteration-loop, ctrl-06, drive-02, drive-06]
requires:
  - src/state_reader/parse_project_state
  - src/executor/outcome/DiskDelta
  - src/journal/JournalEvent::Parked
  - src/driver/run/terminal_label
provides:
  - src/driver/router::decide
  - src/driver/bounds::evaluate
  - src/driver/run::Terminal
  - "CLI: --target-phase, --max-steps, --wall-clock-cap-secs"
affects:
  - src/driver/run.rs
  - src/driver/mod.rs
  - src/cli.rs
  - src/error.rs
  - src/journal/mod.rs
tech-stack:
  added: []
  patterns:
    - "pure classification fn + REASON_* consts + as_str(), mirroring envelope::policy::ParkReason"
    - "exhaustive match with no wildcard as a compile-time gate"
    - "seam validation in driver::drive rather than a clap value_parser"
    - "source-scanning guard with a non-vacuity floor and a control arm"
key-files:
  created:
    - src/driver/router.rs
    - src/driver/bounds.rs
    - tests/driver_iteration_loop.rs
  modified:
    - src/driver/run.rs
    - src/driver/mod.rs
    - src/driver/spawn.rs
    - src/cli.rs
    - src/main.rs
    - src/error.rs
    - src/journal/mod.rs
    - src/ui/screens/driver_start.rs
    - tests/async_blocking_guard.rs
decisions:
  - "ITERATION_WALL_CLOCK_CAP is 3h, strictly inside the 4h run-level cap, asserted by a named test"
  - "DriveArgs::command became Option<String>; exactly one of it and target_phase is required"
  - "Terminal's third arm is named Completed rather than GoalMet, because a failed outcome is not a met goal"
  - "observed/decided are journalled before the bounds gate, so a halt is legible"
  - "the executor is constructed per iteration, because observing_spawn consumes its sender"
metrics:
  duration: ~2h
  completed: 2026-08-19
actuals:
  tokens: 46000
  tasks: 2
  commits: 3
---

# Phase 20 Plan 01: Deterministic Decision Router & Run Bounds Summary

A driven run now executes more than one GSD command in one process, under one
lock and one journal — each command chosen by a pure rule over observed project
state with no model call, and the run stopping itself with the detector named on
disk.

## What Was Built

**`src/driver/router.rs`** — a pure decision function. `decide(&ProjectState,
target_phase) -> Decision` performs no I/O, opens no file and contacts no model:
the caller reads the project with the one shared reader and hands the value in
(D-11). One forward rule (a target phase whose disk status is `Discussed` or
`Researched` routes to `/gsd-plan-phase <N>`) plus two fail-closed arms — a phase
the roadmap does not corroborate parks as `router_state_unverified`, every other
observed status parks as `router_no_rule` naming what was seen. `Decision` has
four arms with no wildcard anywhere it is matched. `phase_disk_statuses` is
indexed by the target key and never iterated.

**`src/driver/bounds.rs`** — CTRL-06's four detectors in a fixed, documented
evaluation order (wall clock → step cap → no progress → command repeat), first
hit returning immediately. `BoundVerdict` cannot express two reasons, which is
what makes "which detector fired" answerable rather than a list. No-progress
compares through `DiskDelta::between` and computes no digest.

**The iteration loop** — `run.rs:1401`–`:1673` hoisted into an outer loop.
Nothing above or below moved: the SIGTERM handler, the process group,
`establish_envelope`, `lock::acquire` and `JournalRun::start` remain per-run, and
`run.json` is still written exactly twice. Every new `select!` preserves `biased`
with the terminate arm first. `Terminal` has three arms and no unclassified one.

**`tests/driver_iteration_loop.rs`** — five tests. The end-to-end one drives a
real fixture project through a real child process and reads every fact back off
disk with the shipped reader: two `decided` records naming the same command, two
`observed` records whose `drpev` vector is length five, exactly one `parked`
record, exactly one `exec_started` (proving the halt preceded the second spawn),
one `run_started`, one `run_ended`, and `run.json`'s outcome equal to
`parked:bounds_command_repeat`.

## Key Decisions

**The cap collision was the phase's live trap and is now a tested invariant.**
`ExecutionOptions::wall_clock_cap` already defaulted to exactly four hours — the
same value proposed for the run-level cap — which would have made the run-level
wall-clock reason unreportable while every test still passed.
`ITERATION_WALL_CLOCK_CAP` is three hours and
`the_iteration_wall_clock_cap_is_strictly_inside_the_run_level_cap` asserts the
strict inequality; the constant's doc names that test rather than claiming the
arithmetic.

**Caps are refused at the seam, and no value disables a detector.** Zero step
caps, zero wall-clock caps and any wall-clock cap above the compiled-in
`MAX_WALL_CLOCK_CAP_SECS` (24h) are refused before a run directory exists.
CTRL-06's prohibition is not merely "do not add a `--no-bounds` flag" but "do not
accept a value that is a disablement in disguise", and a ceiling is what makes
that a property of the parser rather than of the caller's restraint.

**One fresh session per iteration, never `resume_session`** (CONTEXT.md OQ5), so
a multi-hour run cannot hit a context limit for reasons unrelated to the work and
`run.json`'s single `session_id` stays honest.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Spec conflict] The step-cap boundary: the plan contradicts itself,
and the `must_haves` truth wins**
- **Found during:** Task 2
- **Issue:** `must_haves.truths` says *"At max_steps = N, iteration N runs and
  iteration N+1 is refused"* and *"Step counting is u32 compared with `>=`"*.
  The Task 2 acceptance criterion says completed-step counts of N-1, N and N+1
  give *"no halt, no halt, StepCap"* — which is a `>` boundary, permitting N+1
  iterations under a cap of N. Both cannot hold.
- **Fix:** Implemented `completed_steps >= max_steps`, honouring the truths
  (which are the normative contract, are internally consistent, and name the
  comparison operator explicitly). `the_step_cap_fires_at_the_cap_and_not_one_step_below_it`
  asserts N-1 → Continue, N → Halt, N+1 → Halt, all three in one body, which
  satisfies the criterion's stated *purpose* (boundary and both sides proven
  together) even though it inverts one of its three expected values.
- **Files modified:** `src/driver/bounds.rs`
- **Commit:** 2ef488c

**2. [Rule 3 - Blocking] `DriveArgs::command` had to become `Option<String>`**
- **Found during:** Task 1
- **Issue:** The plan requires *"exactly one of `command` and `target_phase` must
  be present"*, which is unrepresentable while `command` is a required `String`.
- **Fix:** `Option<String>`, with the exclusivity refused at the seam by
  `command_source_refusal`. Ripples updated: `src/cli.rs`, `src/main.rs`,
  `src/driver/spawn.rs`, `src/ui/screens/driver_start.rs` and seven integration
  test fixtures. The pinned test
  `drive_args_carry_the_single_command_the_router_phase_will_replace` asserted
  the Phase 17 shape it was named for; it is replaced by
  `drive_args_carry_exactly_one_command_source_and_never_a_supplied_sequence` in
  the same commit as the code, per CONVENTIONS.md:75.
- **Commit:** 6f868a5

**3. [Rule 2 - Missing critical functionality] A zero wall-clock cap is refused
too**
- **Issue:** The plan names only `max_steps` of zero as refusable. A
  `--wall-clock-cap-secs 0` halts before the first spawn, producing the identical
  zero-iteration run the step-cap refusal exists to prevent.
- **Fix:** `BoundsRefusal::ZeroWallClockCap`, refused at the same seam.
- **Commit:** 6f868a5

**4. [Rule 2 - Missing critical functionality] Closed the async-blocking lint
hole research Pitfall 5 documents**
- **Issue:** `RunSnapshot::capture` does full-tree I/O and shells out to git
  twice, and has been invisible to `tests/async_blocking_guard.rs` since Phase 15
  because it was never named in `BLOCKING_HELPERS`. This plan captures a snapshot
  before **every** command rather than twice per run, and the thread it must not
  park is the one polling the terminate arm.
- **Fix:** Added `"RunSnapshot::capture("` to `BLOCKING_HELPERS` plus two
  allowlist entries with written reasons — this plan's join-failure fallback and
  the pre-existing one in `src/executor/claude.rs`. The guard's own
  `no_allowlist_entry_is_stale` passes, which proves both entries suppress a real
  hit and therefore that the hole was real.
- **Files modified:** `tests/async_blocking_guard.rs` (not in the plan's file
  list; added under Rule 2)
- **Commit:** 6f868a5

**5. [Rule 3 - Blocking] `observed` and `decided` moved to `EMITTED_KINDS`**
- **Issue:** `every_reserved_kind_is_declared_and_none_is_emitted_by_this_phase`
  fails the moment the loop emits a reserved kind.
- **Fix:** Both moved, exactly as `interjected` did in Phase 18 and `parked` in
  Phase 19. `RESERVED_KINDS` is now empty — every kind D-36 reserved has been
  drawn down by the phase it was reserved for, and not one needed a migration.
  The guard is kept rather than deleted, with its now-vacuous loop documented as
  deliberate.
- **Commit:** 6f868a5

### Judgement Calls Worth Review

**6. `Terminal`'s third arm is `Completed`, not `GoalMet`.** The plan asks for
"goal-met, parked with a reason, halted with a reason". A routed run whose agent
*failed* also ends on this arm, and naming that state "goal met" would be a lie
in the type name even with the right label on disk. `Completed` documents that
DRIVE-06's goal-met is this arm and that its reason is always `outcome_label`'s.
Three arms, no unclassified arm, every match exhaustive with no wildcard.

**7. `GOAL_MET_LABEL` is a third terminal-label source, for an unreachable
branch.** The plan prohibits any label *"sourced from anywhere other than
`outcome_label` or the `parked:` prefix"*. One case admits neither: a routed run
whose router reports the target already met before it spawned anything has no
`RunOutcome` for `outcome_label` to map, and `parked:` would claim it stopped
needing a human. The alternatives were fabricating a `RunOutcome` or labelling a
finished run as parked. It is **unreachable today** — `Decision::GoalMet` has no
producer until the reader records verification frontmatter status — and it is
documented in-source as the exception so the plan that adds a producer finds a
decision rather than a guess.

**8. `observed`/`decided` are journalled BEFORE the bounds gate.** The plan's
action text lists the journal write after the bounds in one place, but its
acceptance criterion demands **two** `decided` records for a two-iteration run —
which only holds if the record precedes the gate that stops iteration two. It is
also the better record: a `parked` reading `bounds_command_repeat` beside two
`decided` records naming the same command says exactly what happened. Recording a
decision the bounds refused is not a claim it ran; `exec_started` is what says a
command ran, and none follows a halt (asserted end-to-end).

**9. The end-to-end run halts on `bounds_command_repeat` alone, not on two
detectors at once.** The plan expects the two-iteration fixture to have *"both
the no-progress and command-repeat detectors live at once"*. With
`NO_PROGRESS_THRESHOLD = 2`, a two-iteration run accumulates only **one**
unchanged pair, so command-repeat is the only live detector — the two cannot be
simultaneously true in a run this short. The "several at once, fixed order
decides" property is pinned at unit level instead
(`the_documented_evaluation_order_decides_when_every_condition_holds_at_once`,
which walks the whole order down), and a second end-to-end test with
`max_steps: 1` proves the step cap outranks command-repeat on disk.

**10. The executor is constructed per iteration.** `observing_spawn` takes a
`oneshot::Sender` consumed by the first spawn, so a reused executor would publish
the agent's process group for the first command and for no other — and that
channel is what a stop landing during startup uses to reach a group the driver
otherwise cannot signal (CR-01). A second iteration whose startup stop found an
exhausted channel would orphan the agent group.

**11. The inbox sweep moved from per-iteration to once-per-run.** Swept between
commands, a message queued while iteration one was finishing would be journalled
`missed` even though iteration two was about to open a fresh stdin and could have
delivered it. The inbox cursor is likewise per-run, so no message is re-delivered
to a fresh agent that has no idea it is a replay.

## Known Stubs

| Stub | File | Reason / who resolves it |
|---|---|---|
| `Decision::GoalMet` has no producer | `src/driver/router.rs` | Goal-met is the target phase's verification frontmatter `status == passed` (OQ4), and `DiskInference` records only *presence* of a `*-VERIFICATION.md` (research Pitfall 2). Producing it from presence would step past the `human_needed` gate DRIVE-05 exists to park at. The plan that extends the one reader owns it. |
| `RouterReason::DependencyUnsatisfied` has no producer | `src/driver/router.rs` | The plan mints exactly three reason arms; dependency filtering is `init.cjs`'s collision rule and belongs to the plan that widens the rule table. |
| `GOAL_MET_LABEL` is unreachable | `src/driver/run.rs` | Reachable only from `Decision::GoalMet` above. Documented as a decision point rather than a guess. |

None prevents this plan's goal: a routed run executes, routes, bounds and halts
end to end without any of them.

## Deferred Items

- **`dry_run::SECTION_COMMANDS` still claims the single `--command` is the
  complete honest sequence** (research Pitfall 6), which this plan makes false.
  `src/driver/dry_run.rs` is not in this plan's file list. A routed preview
  reports `(routed: chosen per iteration by the decision router)` — a value, not
  the pinned prose — so nothing lies about a specific command; the *paragraph* is
  still stale. Sibling now-false prose sits at `src/driver/dry_run.rs:14` and
  `:107-111`.
- **`DiskStatus` still has no `Executed` variant and `DiskInference` still
  records verification presence rather than frontmatter status** (Pitfalls 1 and
  2). Six of the twenty DRIVE-05 gates remain unobservable. This plan's router
  never treats `Complete` as "advance", so it cannot step past a gate — it parks
  as `router_no_rule` instead.
- **`run.json` does not record the bounds in force.** CONTEXT.md wants the caps
  readable after the fact rather than inferred from the binary's defaults;
  `RunRecord` has no field for them and widening it is a schema change no plan
  declared.
- **Per-iteration `argv_digest`.** `run.json` carries one digest, computed from
  the recorded command. The per-iteration argv variation is visible on the
  `decided` records instead.
- **CTRL-07 rate-limit parking** is not in this plan.

## Verification

| Gate | Result |
|---|---|
| `rtk proxy cargo build` | clean, no warnings |
| `rtk proxy cargo test` | **27 targets, 0 failures.** 853 lib + 5 `driver_iteration_loop` + every pre-existing suite |
| `cargo clippy -- -D warnings` (documented gate) | clean |
| `rtk proxy cargo clippy --all-targets -- -D warnings` | exactly the **5** pre-existing lints, at the same locations recorded in `TESTING.md` |
| `rtk proxy cargo test --lib bounds` | 13 passed |
| `rtk proxy cargo test --lib router` | 6 passed |
| `rtk proxy cargo test --test spawn_seam_guard` | 12 passed, allowlist unchanged in both directions |
| `rtk proxy cargo test --test async_blocking_guard` | 7 passed, including `no_allowlist_entry_is_stale` |
| `driver_dry_run`, `driver_kill`, `driver_lock`, `driver_reattach`, `driver_inbox` | all pass — single-command mode untouched |

**Flaky-test advisory checked deliberately** (research Pitfall 7):
`tests/driver_reattach.rs` passed **3 of 3** in full parallel runs after this
change, and `tests/driver_lock.rs` likewise. The outer loop does not delay the
first `run.json` write — the journal still starts before the loop — so the race
that file inherits is not widened by this plan.

Every `rtk proxy` above is deliberate: `rtk` strips cargo's `warning:` and `test
result:` lines, so a grep of those against bare `cargo` succeeds vacuously.

## Self-Check: PASSED

Created files verified present: `src/driver/router.rs`, `src/driver/bounds.rs`,
`tests/driver_iteration_loop.rs`.
Commits verified in `git log`: `6f868a5`, `2ef488c`.

## Notes on `actuals`

`tokens: 46000` is chars/4 over the **realized diff** (184,754 characters across
19 files), against an estimate of 48,000 — within 5%. The alternative reading of
the same instruction, chars/4 over every *file* touched, gives ~166,000, and that
number is recorded here rather than omitted: it is 3.5× the estimate, and the
gap is entirely the two large existing files (`run.rs`, `journal/mod.rs`) this
plan edits lightly. The diff figure is the one that reflects work performed.
