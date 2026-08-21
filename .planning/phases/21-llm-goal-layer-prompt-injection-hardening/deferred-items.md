# Deferred items — phase 21

Out-of-scope discoveries logged during execution, per the executor's scope
boundary: only issues **directly caused by** a task's own changes are auto-fixed.
Neither item below is caused by any plan in this phase, and neither is in a file
any plan in this phase opened.

## Two integration-test binaries are flaky under parallel execution

Found during plan `21-10`'s whole-suite verification. Both are **pre-existing
concurrency flakes**, not regressions:

| Binary | Test | Symptom |
|---|---|---|
| `tests/envelope_tracer.rs` | `a_relocated_copy_of_the_stub_refuses_instead_of_acting` | `the generated stub is executable: Os { code: 26, kind: ExecutableFileBusy, message: "Text file busy" }` — the classic write-then-exec race |
| `tests/driver_reattach.rs` | `a_run_killed_without_an_ending_is_reported_crashed_and_nothing_on_disk_is_repaired`, `a_fresh_scan_finds_the_orphaned_run_live_with_its_last_journal_step` | `the run record is on disk: Os { code: 2, kind: NotFound }` and `exactly one project has a run to observe — left: 0, right: 1` |

**Evidence that these are flakes rather than a regression.** Against **one
unchanged binary**, `cargo test --test driver_reattach` returned FAILED, FAILED,
then ok on three consecutive runs; the same binary with `-- --test-threads=1`
returned ok three times out of three. The failing runs finish in ~0.5s against
~6.1s for a passing one, which is the shape of a probe bailing before the thing
it waits for exists. `envelope_tracer` failed once inside a full-suite run and
passed immediately when run alone.

**Evidence that plan 21-10 cannot be the cause.** `21-10` changed only the
*second* `run.json` write (the terminal one) and the TUI's opt-in revert. Both
failing `driver_reattach` assertions are about the run record's existence and
observability at write **one** (`JournalRun::start`), which is untouched, and
they run before any terminal write by construction — the crash test asserts
`ended_at` is still null at that point.

`rtk proxy cargo test -- --test-threads=2` over the whole suite exits **0** with
**1193 passing tests across 35 binaries and 0 failures**.

**Not fixed here.** Neither `tests/envelope_tracer.rs` nor
`tests/driver_reattach.rs` is in any `21-*` plan's `<files>`, and a fix is a
synchronisation change to a live-process probe rather than a one-line
correction. Carry into the next phase's backlog.

**Update (2026-08-21, round 4): `driver_reattach` is now flakier than recorded
above, and its documented mitigation no longer works.** Under
`--test-threads=4` it fails intermittently in whole-suite runs; under
`-- --test-threads=1` — which this file records as passing three times out of
three — it now also fails intermittently. Confirmed **not** a round-4
regression by building the pre-round-4 tree (`6eb1d49`) in a separate worktree
and running the binary six times: `ok, FAILED, FAILED, FAILED, FAILED, FAILED`.
The untouched baseline flakes *worse* than the round-4 tree. The run ids it uses
(`2026-07-29T12-00-00Z-aaaa`) are exactly the shape 21-13's tightened
`is_plain_path_component` pins as accepted, so that change cannot be the cause.
`--test-threads=2` remains reliably green for the whole workspace. Raising the
priority of the carried item rather than adding a new one.

---

# Round-4 adjudications (2026-08-21)

Items carried forward from the round-2 review that round 4 deliberately declines,
each with the reason from `21-PREMISES.md` Premise 6. Recorded here rather than
dropped, so nothing leaves the phase silently — the prohibition
`21-14-PLAN.md` carries as `MUST NOT drop an adjudicated-out finding silently`.

## OUT — deferred

| Item | Location | Reason declined |
|---|---|---|
| `registry::current_prompt_inputs` absent from `BLOCKING_HELPERS` | `tests/async_blocking_guard.rs:124-144` | Async-hygiene (synchronous disk reads under an `async fn`), not the failing criterion's class. The fix forces production `spawn_blocking` rewiring in `approve_plan` and `execute_run` — real scope, and zero bearing on ROADMAP criterion 1. |
| The spawn-gate plan-half argument lives in a comment rather than in a checked property | `src/driver/run.rs:2300-2328` | The comment now states plainly that the plan half is a no-op there and why that is sound (decompose-once). Converting a sound, honestly-documented argument into a checked property is hardening, not gap closure. |
| Dead `PlanStep::rationale` | `src/driver/goal.rs` | No production reader. A cosmetic dead field with no security or honesty bearing. |

Any of the three can be promoted into a future phase; none is closed by
round 4, and none should be read as fixed.

## IN — closed by 21-13

| Item | Location | Disposition |
|---|---|---|
| `plan_target_phase(plan).unwrap_or_default()` writing a blank `target_phase` | `src/driver/mod.rs` (`approve_plan`) | **Adjudicated IN and closed.** Same defect class as the criterion-1 failures — a blank value reaching a persisted record, where `""` already means field-absent (D-30) — merely arriving through the model seam instead of argv. Excluding it would have repeated the exact scoping bet that lost three times. `approve_plan` now refuses with the same typed error `goal::legality` raises for a stepless plan. |
