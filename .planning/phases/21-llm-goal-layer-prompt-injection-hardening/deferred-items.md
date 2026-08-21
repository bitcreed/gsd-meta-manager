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
