---
phase: 19-gitsafe-git-blast-radius-envelope
plan: 10
subsystem: testing
tags: [rust, tokio, flock, envelope, test-harness, race-condition]

requires:
  - phase: 19-gitsafe-git-blast-radius-envelope
    provides: "`envelope::hooks::write_settings_in`'s persist -> read-back -> refuse sequence (D-07), `envelope::envelope_dir_in`, and the `driver::lock` flock seam these tests drive"
provides:
  - "`tests/driver_lock.rs` with one alias per driving test, so no two tests write the same `<envelope_root>/<alias>/settings.json`"
  - "`wait_for_lock_or_report`, which polls `Child::try_wait` alongside `lock::read_holder` and reports an early-dying child with its exit status and captured stderr"
  - "`ChildCapture`, redirecting a spawned child's stdout/stderr to files in a `TempDir` (never pipes), read only on failure paths"
  - "a control proving the early-exit reporting works, in seconds rather than at the 30s deadline"
  - "`no_two_driving_tests_share_an_envelope_settings_path` — a source-scanning regression gate, observed RED against the pre-fix file"
  - "`two_writers_sharing_one_alias_break_each_others_settings_verification` — the mechanism reproduced by construction at the real `write_settings_in`/`verify_settings` seam"
affects: [driver_lock, envelope, driver_reattach]

actuals:
  tokens: 6865
  tasks: 3
  commits: 3

tech-stack:
  added: []
  patterns:
    - "Per-test aliases under a shared process-wide envelope root, enforced by a source-scanning gate rather than by a comment"
    - "Child output captured to files in a TempDir, never pipes, read only on failure paths"
    - "Source-scanning gates assemble every needle at runtime so the scanner cannot match itself"

key-files:
  created: []
  modified:
    - tests/driver_lock.rs

key-decisions:
  - "The fix stays entirely in the harness; `write_settings_in`'s read-back refusal was NOT weakened, because it is the only moment this process can detect a settings file the agent CLI would silently ignore (D-07, SAFE-06)"
  - "The envelope ROOT stays shared and process-wide; only the alias, and therefore the directory beneath the root, becomes per-test"
  - "Within a single test the holder and the second drive keep the SAME alias — they contend for one project, and changing that would delete the contention the file exists to test"
  - "Control B (alias uniqueness) is the regression gate; Control A (two writers) is evidence of the mechanism and is green in both directions"
  - "The child's output goes to files, never pipes: an un-drained pipe deadlocks a child that outgrows the kernel buffer, converting a rare misleading failure into a reliable hang"

patterns-established:
  - "Anti-vacuity first: a source-scanning gate asserts its extraction is non-empty before asserting anything about the extraction"
  - "A poll loop over a spawned child checks the success condition BEFORE the exit status, so a child that succeeds and then dies is still a success"

requirements-completed: [SAFE-05, SAFE-06]

coverage:
  - id: D1
    description: "A child that exits before taking the lock is reported with its exit status and its captured stderr, in seconds rather than after the 30s deadline"
    requirement: "SAFE-06"
    verification:
      - kind: integration
        ref: "tests/driver_lock.rs#a_child_that_dies_before_taking_the_lock_is_reported_with_its_status_and_stderr"
        status: pass
    human_judgment: false
  - id: D2
    description: "The child's stdout/stderr are captured to FILES in a TempDir, never to pipes, and are read only on failure paths"
    verification: []
    human_judgment: true
    rationale: "No automated assertion forbids a future edit from reintroducing `Stdio::piped()`. The property rests on the redirect in `ChildCapture` and the comment beside it; a gate for it was not in scope and would have to scan for the absence of a construct rather than the presence of one."
  - id: D3
    description: "No two driving tests in tests/driver_lock.rs resolve to the same envelope settings directory — 6 aliases, 6 driving tests, 6 distinct directories"
    requirement: "SAFE-06"
    verification:
      - kind: unit
        ref: "tests/driver_lock.rs#no_two_driving_tests_share_an_envelope_settings_path"
        status: pass
    human_judgment: false
  - id: D4
    description: "The shared-path mechanism is demonstrated deterministically at the real write_settings_in/verify_settings seam, with the refusal carrying REASON_ENVELOPE_ASSERTION_FAILED"
    verification:
      - kind: unit
        ref: "tests/driver_lock.rs#two_writers_sharing_one_alias_break_each_others_settings_verification"
        status: pass
    human_judgment: false
  - id: D5
    description: "No file under src/ was modified and the pre-lock ordering of establish_envelope() and lock::acquire is untouched"
    verification:
      - kind: other
        ref: "git diff --stat fbc5670..04fb2bf -- src/ (empty output)"
        status: pass
    human_judgment: false
  - id: D6
    description: "The race that produced the 2026-08-18 one-off no longer has a window to open on"
    verification:
      - kind: other
        ref: "20 sequential runs of the driver_lock binary under a concurrent cargo test loop: 20/20 pass, worst wall time 6.349s"
        status: pass
    human_judgment: true
    rationale: "Sampling cannot disprove a microsecond window. The repetition is corroboration only; the load-bearing evidence is the source-scanning gate D3, which makes a shared settings path a compile-time-shaped red test rather than a timing question."

duration: 20 min
completed: 2026-08-28
status: complete
---

# Phase 19 Plan 10: Per-test envelope aliases in `tests/driver_lock.rs` Summary

**Six per-test aliases replace one shared `ALIAS`, so five tests no longer write one `settings.json` from two different `current_exe()` values — plus a `try_wait`-polling reporter that names a dying child's exit status and stderr instead of blaming a 30s lock timeout.**

## Performance

- **Duration:** ~20 min
- **Started:** 2026-08-29T02:47Z
- **Completed:** 2026-08-29T03:08Z
- **Tasks:** 3
- **Files modified:** 1 (`tests/driver_lock.rs`)

## Accomplishments

- **The race has no window left.** Each of the six driving tests in `tests/driver_lock.rs` now names its own alias and therefore its own directory beneath the shared envelope root. `config_for`, `args`, `start_holder` and `second_drive` take the alias as a parameter rather than reading a global.
- **The harness now says what actually happened.** `wait_for_lock_or_report` polls `lock::read_holder` and `Child::try_wait` together, checking the holder record FIRST so a child that takes the lock and then dies is still a success, and folding the child's exit status and captured stderr into the failure when it dies early.
- **The invariant is held by a control that was observed RED**, not by an intention, and the verbatim RED output is quoted below.
- **The mechanism is demonstrated by construction**, not by waiting for a race, at the real `write_settings_in`/`verify_settings` seam.

## Task Commits

1. **Task 1: Make an early-dying child report itself** — `bfa3fee` (test)
2. **Task 2: Two controls — mechanism demonstrated, harness invariant observed RED** — `0447272` (test)
3. **Task 3: One alias per test** — `04fb2bf` (fix)

## The verbatim RED output of the alias-uniqueness gate

Captured at commit `0447272`, against the file as it stood before Task 3 changed anything —
`rtk proxy cargo test --test driver_lock`, raw (unfiltered) output:

```
running 8 tests
test no_two_driving_tests_share_an_envelope_settings_path ... FAILED
test two_writers_sharing_one_alias_break_each_others_settings_verification ... ok
test acquiring_the_run_lock_does_not_block_the_async_runtime ... ok
test a_child_that_dies_before_taking_the_lock_is_reported_with_its_status_and_stderr ... ok
test the_lock_is_released_when_the_holding_process_dies ... ok
test a_second_drive_reports_which_run_holds_the_lock ... ok
test the_losing_reader_does_not_truncate_the_lock_file ... ok
test a_duplicate_start_refuses_promptly_rather_than_blocking ... ok

failures:

---- no_two_driving_tests_share_an_envelope_settings_path stdout ----

thread 'no_two_driving_tests_share_an_envelope_settings_path' (866155) panicked at tests/driver_lock.rs:758:5:
the shared alias declaration is still present: every test in this file would write one settings.json. a shared envelope settings path plus differing current_exe() values fails the child's read-back verification (hooks.rs:1340-1341) before it ever reaches lock::acquire, and the child's death then reads as a lock timeout (G-19-4)

note: run with `RUST_BACKTRACE=1` environment variable to display a backtrace


failures:
    no_two_driving_tests_share_an_envelope_settings_path

test result: FAILED. 7 passed; 1 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.09s
```

Assertion 2 (the shared declaration is gone) fired first and short-circuited assertion 3, which
would also have failed at that commit: one alias declaration existed against six driving tests.
Assertion 1, the anti-vacuity guard, passed — the scan found the one `ALIAS_EARLY_EXIT_REPORT`
declaration Task 1 added, so the gate was reading real source rather than nothing.

After Task 3 (`04fb2bf`), same command, raw:

```
test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.09s
```

## The six aliases and the six directories they resolve to

`no_two_driving_tests_share_an_envelope_settings_path` maps each alias through
`envelope::envelope_dir_in` against the fixed root `/gsd-mm-alias-uniqueness-gate`
(a pure validate-and-join; the filesystem is never touched):

| Const | Value | Owning test | Resolved directory |
|---|---|---|---|
| `ALIAS_HOLDER_NAMED` | `lock-holder-named` | `a_second_drive_reports_which_run_holds_the_lock` | `/gsd-mm-alias-uniqueness-gate/lock-holder-named` |
| `ALIAS_PROMPT_REFUSAL` | `lock-prompt-refusal` | `a_duplicate_start_refuses_promptly_rather_than_blocking` | `/gsd-mm-alias-uniqueness-gate/lock-prompt-refusal` |
| `ALIAS_BLOCKING_POOL` | `lock-blocking-pool` | `acquiring_the_run_lock_does_not_block_the_async_runtime` | `/gsd-mm-alias-uniqueness-gate/lock-blocking-pool` |
| `ALIAS_NO_TRUNCATION` | `lock-no-truncation` | `the_losing_reader_does_not_truncate_the_lock_file` | `/gsd-mm-alias-uniqueness-gate/lock-no-truncation` |
| `ALIAS_HOLDER_DIES` | `lock-holder-dies` | `the_lock_is_released_when_the_holding_process_dies` | `/gsd-mm-alias-uniqueness-gate/lock-holder-dies` |
| `ALIAS_EARLY_EXIT_REPORT` | `lock-early-exit-report` | `a_child_that_dies_before_taking_the_lock_is_reported_with_its_status_and_stderr` | `/gsd-mm-alias-uniqueness-gate/lock-early-exit-report` |

Six aliases, six driving tests, six distinct directories. Independently confirmed outside the
test: `grep -c 'const ALIAS_'` = 6 and `grep -c '#\[tokio::test'` = 6.

## Which control is load-bearing, and which is not

**`no_two_driving_tests_share_an_envelope_settings_path` (Control B) is the regression gate.** It
was red before Task 3 and green after, and a seventh test reusing a sibling's alias makes it red
again. It is a source scan, so it does not depend on a race being reproducible.

**`two_writers_sharing_one_alias_break_each_others_settings_verification` (Control A) is evidence,
not a gate.** It is green before the fix and green after it. It exists to show that the diagnosis
rests on a measured cause: writing one alias's settings from two different binary paths and then
verifying against the first writer's expected value produces an `Err` carrying
`policy::REASON_ENVELOPE_ASSERTION_FAILED`, deterministically and with no timing involved.
Presenting it as the fix's gate would be a category error.

**The read-back refusal it triggers is NOT a product bug.** `write_settings_in` persisting and then
re-reading is exactly what D-07 asks of it — the agent CLI ignores an invalid settings file
silently, so the moment after the write is the only moment this process can tell the difference.
The defect was a harness that pointed two writers at one path.

## The repetition run — sampling, not proof

20 sequential runs of the built `driver_lock` binary (`--test-threads=4`), with a background loop
running `cargo test --no-fail-fast` concurrently for load:

- **Pass count: 20/20.** 0 failures.
- **Worst wall time: 6.349s.** Median ~6.14s.

The ~6.1s floor is the paced `fake-claude-slow.sh` stand-in's own lifetime (40 heartbeats at
150ms), not lock latency.

**This is sampling and it is labelled as such.** A microsecond window cannot be disproved by
sampling — 100 attempts under load average 55 during diagnosis produced 0 failures too, which is
precisely why the original defect took a full investigation. The load-bearing evidence is the
source-scanning gate, which makes a shared settings path a red test rather than a timing question.
The repetition is corroboration only.

## The refuted hypothesis was NOT revived

`establish_envelope()` at `src/driver/run.rs:2493` does precede `lock::acquire` at
`src/driver/run.rs:2609`. **That ordering was not touched, and slowness is not the cause.** It was
measured and refuted: 100 reproduction attempts under deliberate load to load average 55 produced
0 failures, worst wall time 1.09s against the 600 x 50ms = 30s deadline — roughly 28x margin. It is
recorded here only so the next reader is not sent to a path where nothing is wrong (D-28).

## Files Created/Modified

- `tests/driver_lock.rs` — six per-test alias consts with a G-19-4 block comment recording why they
  may not be consolidated; `config_for`/`args`/`start_holder`/`second_drive` take the alias as a
  parameter; `ChildCapture` (file redirects, `TempDir`-owned); `wait_for_lock_or_report`
  (`try_wait` + `read_holder`, `Result` not `panic!`); three new tests.

**Nothing under `src/` was modified.** Proved by `git diff --stat fbc5670..04fb2bf -- src/`, which
produces empty output. The whole-plan diff is `tests/driver_lock.rs | 436 insertions(+), 32
deletions(-)`, one file.

## Decisions Made

- **Kept the envelope root shared, changed only the alias.** `isolate_envelope_root`'s `OnceLock`
  and its `TempDir` are untouched: the root must outlive every test, and the child inherits it
  through `GSD_MM_ENVELOPE_ROOT`, which is what carries the isolation across the process boundary.
- **Holder and second drive within one test share an alias.** They contend for one project and the
  lock lives in that project's `.planning`; splitting them would delete the contention. Both are
  in-process, so both derive the same `current_exe()` and write byte-identical settings — the
  mismatch only ever existed between the in-process siblings and the real-binary child.
- **`wait_for_lock_or_report` returns `Result`, not `panic!`.** That is what makes the reporting
  behaviour itself testable, which is the point of the new control.
- **Every needle in the source-scanning gate is assembled at runtime** (`format!("const {}", "ALIAS_")`
  and friends). A scanner containing its own search strings matches itself; here it would also make
  the shared-declaration assertion permanently unsatisfiable.
- **The gate `expect`s `Some` from `envelope_dir_in`** rather than filtering: an alias that is not a
  plain path component sanctions no settings file at all, which would be a different defect wearing
  this one's clothes.

## Deviations from Plan

None — plan executed exactly as written, in the specified order (observability, then RED gate, then
fix). No `src/` file was reached into and no deviation rule fired.

One arithmetic correction to the plan's own verification note, recorded for honesty rather than as
a deviation: `<verification>` item 4 predicts "+2 over the pre-plan count in this binary (two
controls in Tasks 1-2)". The actual delta is **+3** — Task 1 adds one control and Task 2 adds two.
`tests/driver_lock.rs` went from 5 tests to 8. Every one of the three is named in this plan, so the
delta is fully attributed.

## Verification Results

All measured through `rtk proxy` — the global RTK hook strips `warning:` and `test result:` lines,
so a grep for either against filtered output succeeds vacuously.

| Check | Result |
|---|---|
| `rtk proxy cargo test --test driver_lock` | `test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.09s` |
| `rtk proxy git diff --stat fbc5670..HEAD` | `tests/driver_lock.rs \| 468 ++++...`, **1 file changed** |
| `rtk proxy git diff --stat fbc5670..HEAD -- src/` | empty — `src/` untouched |
| `rtk proxy cargo test --no-fail-fast` (full suite) | **1436 passed; 0 failed; 13 ignored** on the green run |
| `rtk proxy cargo clippy -- -D warnings` | **exit 0** (the project's lib gate) |
| `rtk proxy cargo clippy --all-targets` | 4 warnings, **0 of them in `tests/driver_lock.rs`** — all 4 cite `src/browser.rs:155-157` and `src/project_creator.rs:146`, files this plan did not touch |
| Repetition study | 20/20 pass, worst 6.349s |

**A note on the pre-existing `--all-targets` clippy baseline.** `14-CONTEXT.md` documents **5**
pre-existing lints; I measure **4** today — the 3x `assert_eq!`-with-literal-bool in `browser.rs`
and the 1x owned-instance-for-comparison in `project_creator.rs` are all present, but the
items-after-test-module lint in `state_reader/mod.rs` no longer fires. This plan cannot have changed
that (its diff touches no `src/` file), so the count moved at some earlier point and the documented
figure is stale. Recorded, not acted on.

## Issues Encountered

**`tests/driver_reattach.rs` is flaky, it is out of scope, and it looks like the same family of
defect.** This is the known flake named in the execution brief (19-09 saw it too). Confirmed in
isolation at this commit: running the binary's 3 tests together failed 2 of 3 attempts; running the
single named test alone passed. Two different tests in that binary failed on different attempts:

```
---- a_run_killed_without_an_ending_is_reported_crashed_and_nothing_on_disk_is_repaired stdout ----
thread '...' panicked at tests/driver_reattach.rs:542:50:
the run record is on disk: Os { code: 2, kind: NotFound, message: "No such file or directory" }
```

That symptom — a child that died before writing its run record — is the G-19-4 shape. A read-only
look confirms the structural precondition: `tests/driver_reattach.rs:128` declares a single shared
`const ALIAS: &str = "detached"` and `:181` has its own process-wide `isolate_envelope_root`, the
same combination this plan just removed from `driver_lock.rs`.

**This is an observation, not a claim, and nothing was done about it.** The plan's prohibition
explicitly fences the fix to `driver_lock.rs`, and its scope note names
`tests/driver_refusal_record.rs` and `tests/envelope_wiring.rs` as the files that were checked and
cleared — `driver_reattach.rs` is not among them, so the plan's "the only file that mixes a
real-binary drive with in-process drives under one process-wide root and one alias" claim may be
incomplete. Whether `driver_reattach.rs` actually spawns the real binary was not verified here.
**Recommend a follow-up gap item.**

Confirmed as pre-existing and independent of this change: the full suite ran **1436 passed / 0
failed** on one execution and red on another with only `driver_reattach` failing, and this plan's
diff touches one file in a different test binary.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- G-19-4 is closed. `tests/driver_lock.rs` holds its own invariant mechanically.
- The `driver_reattach.rs` flake above is the highest-value carry-forward from this plan.
- Phase 19's deferred human-judgement UAT items are unchanged by this plan.

## Self-Check: PASSED

- `tests/driver_lock.rs` exists on disk and contains `wait_for_lock_or_report` (the artifact
  contract's `contains` clause).
- All three commits exist: `bfa3fee`, `0447272`, `04fb2bf`.
- All 8 tests in `tests/driver_lock.rs` pass; `git diff --stat -- src/` is empty.

---
*Phase: 19-gitsafe-git-blast-radius-envelope*
*Completed: 2026-08-28*
