---
phase: 17-supervisor-detach-kill-switch-dry-run-opt-in-gate
plan: 02
subsystem: infra
tags: [driver, lock, flock, rustix, journal, gitignore, concurrency, ctrl-05]

requires:
  - phase: 16-run-journal-state-substrate
    provides: "runs_root, run_paths, RUNS_GITIGNORE_BODY, write_runs_gitignore, create_run_dir, JournalRun"
  - phase: 17-supervisor-detach-kill-switch-dry-run-opt-in-gate
    plan: 01
    provides: "src/driver/{mod,run}.rs, DriverRun, execute_run, DriveError, rustix 1.1 with features process+fs, tests/fixtures/fake-claude-slow.sh"
provides:
  - "`src/driver/lock.rs`: the repository's first file lock — non-blocking exclusive `flock(2)`, a descriptor-owning `RunLock` guard, and a read-only loser path"
  - "`lock::acquire` / `lock::read_holder` / `lock::lock_path` / `LockHolder` / `RunLock::holder`"
  - "`LockError { HeldBy, HeldByUnknownRun, Unavailable }` and `DriveError::Lock` — the typed refusal the CLI renders verbatim"
  - "`journal::writer::ensure_runs_root` — the single entry point that decides when the runs `.gitignore` lands"
  - "tests/driver_lock.rs — the first contention proof in the repository, and the first test that SIGKILLs a real child driver"
affects: [17-04 dry-run, 17-05 detached spawn and reattach, 17-06 kill switch, 17-07 UI]

tech-stack:
  added: []
  patterns:
    - "Guard type owning a held descriptor, with no `impl Drop` — the close is the only release path, stated in the doc so nobody adds a second"
    - "Truncate-then-write through an already-held descriptor, never temp-file-plus-persist, because a rename replaces the inode and hands the lock away"
    - "An unreadable holder record is reported as held-by-unknown, never as not-held: a partial read degrades the message, never the invariant"
    - "Bounded-timeout tests for synchronous syscalls put the call on its own task and time out on the JoinHandle, because `Timeout::poll` polls inline"

key-files:
  created:
    - src/driver/lock.rs
    - tests/driver_lock.rs
  modified:
    - src/driver/mod.rs
    - src/driver/run.rs
    - src/error.rs
    - src/journal/writer.rs

key-decisions:
  - "The lock file is `.planning/meta-manager/runs/run.lock`, not `.planning/meta-manager/run.lock`: the runs `.gitignore` catch-all already excludes it, so it is kept out of a driven repo by a rule that already exists, and it sits beside `active`"
  - "`ensure_runs_root` became public because the lock file is now the FIRST file to land in that directory — the ignore file has to be writable from outside `create_run_dir` or there is a window with no protection"
  - "The duplicate-start test spawns the second drive as a task and times out on the handle; timing out on the call directly cannot fire, because a blocking `flock` parks the very thread the timer needs (observed, not theorised)"
  - "`RunLock.file` and `DriverRun.lock` carry `#[allow(dead_code)]` plus a sentence saying why, rather than an underscore rename that would read as leftover and invite deletion"

patterns-established:
  - "A negative grep is part of the contract: no `truncate(true)`, no `persist(`, no `impl Drop` in the lock module"
  - "Plant the wrong implementation and observe the discriminating test red before trusting it green"

requirements-completed: []

coverage:
  - id: L1
    description: "A second `drive` against a project whose first run is still live refuses and names which run holds the lock — run id and pgid — in the rendered message, and starts no second run"
    requirement: "CTRL-05"
    verification:
      - kind: integration
        ref: "tests/driver_lock.rs#a_second_drive_reports_which_run_holds_the_lock"
        status: pass
    human_judgment: false
  - id: L2
    description: "The lock is flock(2) on a real descriptor held for the run's duration, so a driver killed with SIGKILL releases it by dying"
    requirement: "CTRL-05"
    verification:
      - kind: integration
        ref: "tests/driver_lock.rs#the_lock_is_released_when_the_holding_process_dies"
        status: pass
    human_judgment: false
  - id: L3
    description: "The acquire is non-blocking, so a duplicate start is a fast typed refusal rather than a hang"
    requirement: "CTRL-05"
    verification:
      - kind: integration
        ref: "tests/driver_lock.rs#a_duplicate_start_refuses_promptly_rather_than_blocking"
        status: pass
      - kind: manual_procedural
        ref: "planted FlockOperation::LockExclusive; the test failed in 1.07s naming the property, then was reverted"
        status: pass
    human_judgment: false
  - id: L4
    description: "The loser opens the lock file read-only and never truncates it; the file is byte-identical after a losing attempt"
    requirement: "CTRL-05"
    verification:
      - kind: integration
        ref: "tests/driver_lock.rs#the_losing_reader_does_not_truncate_the_lock_file"
        status: pass
      - kind: manual_procedural
        ref: "grep -v '^[[:space:]]*//' src/driver/lock.rs | grep -c 'truncate(true)' == 0"
        status: pass
    human_judgment: false
  - id: L5
    description: "A partial or empty holder record is reported as held by an unknown run, never as not held"
    verification:
      - kind: unit
        ref: "src/driver/lock.rs#an_empty_lock_file_reads_as_an_unknown_holder_not_as_unheld"
        status: pass
      - kind: unit
        ref: "src/driver/lock.rs#a_truncated_holder_record_reads_as_an_unknown_holder_not_as_unheld"
        status: pass
    human_judgment: false
  - id: L6
    description: "The lock file is created under a runs root whose .gitignore already exists, so it is never committed and never leaves an unprotected window"
    verification:
      - kind: unit
        ref: "src/driver/lock.rs#the_lock_path_sits_under_the_gitignored_runs_root"
        status: pass
      - kind: unit
        ref: "src/driver/lock.rs#a_holder_record_round_trips_through_the_lock_file"
        status: pass
    human_judgment: false
  - id: L7
    description: "The driver, not the TUI, acquires the lock in its own process after exec, after the opt-in gate and before the journal"
    verification:
      - kind: manual_procedural
        ref: "grep -n 'lock::acquire|JournalRun::start' src/driver/run.rs — acquire at 186, journal start at 188; the gate ran in drive() before dispatch"
        status: pass
      - kind: integration
        ref: "tests/driver_tracer.rs#a_drive_against_a_project_with_no_opt_in_record_is_refused_and_writes_nothing"
        status: pass
    human_judgment: false

duration: 21 min
completed: 2026-07-29
status: complete
---

# Phase 17 Plan 02: The Single-Execution Lock Summary

**A second `drive` against a live project now refuses in well under a second, names the holding run id and process group in the message the user reads, starts no second run directory, leaves the lock file byte-identical — and the lock dies with its holder, which is the one property a PID file cannot provide.**

## Performance

- **Duration:** 21 min
- **Started:** 2026-07-29T17:05:00Z
- **Completed:** 2026-07-29T17:26:00Z
- **Tasks:** 2
- **Files modified:** 6 (2 created, 4 modified)

## Accomplishments

- **The repository has file locking for the first time.** There was no `nix`, no `libc`, no `fs4`, no `fs2` and no `flock` anywhere in `src/` or `Cargo.toml`; `NamedTempFile` + `persist` was the only on-disk concurrency primitive that existed. `src/driver/lock.rs` is the first, and it needed no new dependency — `rustix` arrived in plan 17-01 with the `fs` feature already on for exactly this.
- **Success criterion #5 is mechanically proved, not asserted.** The contention test runs two real concurrent drives and checks the *rendered* refusal string for the holding run id, then checks on disk that exactly one run directory exists. A `HeldBy` variant whose `Display` said only "locked" would satisfy the variant assertion and fail the string one — which is the distinction the criterion is actually about.
- **Each of D-20's four properties has its own test, and the ones that matter were observed failing first.** A blocking `flock` passes every other test in the file; planting `FlockOperation::LockExclusive` made the discriminating test fail in 1.07s naming the property, and the plant was then reverted.
- **The ignore file now provably lands before the file it protects.** `ensure_runs_root` is the single entry point both the run directory and the lock file go through, so there is no ordering a future caller can get wrong — the lock file is the first byte to land in that directory, which is precisely why the function had to become callable from outside `create_run_dir`.
- **The kernel is the enforcement and the record is only a message.** `LockHolder`'s doc says so in as many words (T-17-08): a hand-edited record can change what a loser is *told* and can never grant a second lock.

## Task Commits

1. **Task 1: `flock` on a held descriptor, with holder metadata a loser can read** — `1a56a13` (feat)
2. **Task 2: Hold the lock for the run, and prove a second start names the first** — `6573023` (feat)

## Files Created/Modified

**Created**

- `src/driver/lock.rs` — `lock_path`, `LockHolder`, `RunLock` (+ `holder()`), `acquire`, `read_holder`, and four unit tests. The module doc states D-19 and all four of D-20's properties as numbered facts, each naming the failure it prevents.
- `tests/driver_lock.rs` — the four proofs that need two concurrently-live processes, one of which needs the driver in a separate process image so it can be SIGKILLed.

**Modified**

- `src/driver/mod.rs` — the one additive `#[cfg(unix)] pub mod lock;` line, plus one stale sentence corrected (the module doc listed `lock` among modules "later plans add").
- `src/driver/run.rs` — `execute_run` acquires the lock between the gate and the journal; `DriverRun` gains the guard field.
- `src/error.rs` — `LockError` with three variants and hand-written `Display`; `DriveError::Lock`, its `Display` arm, its `source` arm, and `From<LockError>`.
- `src/journal/writer.rs` — `ensure_runs_root` added and `create_run_dir` refactored onto it; `write_runs_gitignore`'s "private because `create_run_dir` is the only correct moment" doc corrected to name the new single entry point.

## Decisions Made

- **The lock lives at `runs/run.lock`, not `meta-manager/run.lock`.** PITFALLS suggested the latter literally. The runs root won on two counts recorded in `lock_path`'s doc: `RUNS_GITIGNORE_BODY`'s catch-all already excludes everything directly under `runs/` except `.gitignore` and `*/run.json`, so the lock is kept out of a driven repository's index by a rule that already exists rather than one somebody must remember to add; and it sits beside `active`, the other run-scoped pointer file, keeping the run neighbourhood one directory.
- **The winner writes through the descriptor it already holds.** `set_len(0)` + `rewind` + `write_all`, never `NamedTempFile` + `persist`. The temp-file idiom is what `write_run_record` uses two modules away and is the single most likely way a reader of it gets this wrong: a rename replaces the *inode*, so the lock would go on protecting a file nobody can see while the path it names is unlocked. Enforced by a negative grep, and explained in a comment beside the code that looks like it.
- **No `impl Drop`.** Closing the descriptor is what releases the lock and the kernel does that on drop. An explicit unlock would add a second release path for one resource, which is how a release comes to happen twice or not at all. Also enforced by a negative grep.
- **`O_CLOEXEC` is load-bearing and is now written down.** Rust opens files `O_CLOEXEC`, so the lock descriptor is *not* inherited by the spawned agent. If it were, a SIGKILLed driver's lock would survive for as long as its grandchildren lived and the whole "process-death-safe" argument would quietly stop being true. This is not something the code does — it is something the code depends on — so it is stated in the module doc rather than left to be rediscovered.
- **`.truncate(false)` is written out rather than omitted.** It is the explicit form of "never `truncate(true)` here", and clippy's `suspicious_open_options` asks for a truncate decision beside `create` anyway, so the one place the answer matters states it.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] The specified duplicate-start timeout could not fire, and deadlocked the suite instead**

- **Found during:** Task 2, while red-checking the test against a planted blocking acquire
- **Issue:** The plan specifies wrapping the second `drive` in `tokio::time::timeout` and asserting the timeout did not fire. `lock::acquire` is a **synchronous** syscall inside an async fn, so a blocking `flock` parks the OS thread rather than yielding — and `Timeout::poll` polls the inner future *inline*, so the timer, run A, and the timeout itself all shared the one parked current-thread runtime. Planting `FlockOperation::LockExclusive` did not produce a timeout failure: it deadlocked **three** of the four tests indefinitely, and `cargo test` had to be killed by hand. A blocking implementation would therefore have been detected only as "CI hangs forever", which is the diagnosis-free failure D-20.4 exists to prevent, one layer up.
- **Fix:** the test is `#[tokio::test(flavor = "multi_thread", worker_threads = 4)]`, the second drive runs as its own `tokio::spawn`ed task, and the timeout is applied to the `JoinHandle`. The stuck poll then occupies one worker while the task holding the timer is woken on another. The plan's criterion is satisfied in both letter and substance — the call is wrapped in a bounded timeout and the test asserts it did not fire — and the bound is now real. A comment on the test records why the obvious shape is wrong, so nobody simplifies it back.
- **Files modified:** `tests/driver_lock.rs`
- **Verification:** with the blocking form planted, the test failed in **1.07s** with `the second drive must refuse within one second, never block: Elapsed(())`; reverted, and `git diff --stat src/driver/lock.rs` confirmed the module byte-identical to its commit before proceeding.
- **Committed in:** `6573023`

**2. [Rule 1 - Bug] `src/driver/mod.rs`'s module doc claimed `lock` did not exist yet**

- **Found during:** Task 2
- **Issue:** The doc read "Later plans add `lock` (17-02), `dry_run` (17-04), …". Adding the module without touching the sentence would have left a false statement two lines above the declaration that falsifies it.
- **Fix:** one sentence rewritten to `[lock] landed in plan 17-02. Later plans add …`. The plan budgets "exactly one additive line" for this file; this is a documentation correction rather than a second mechanism, and leaving it stale was not an option.
- **Files modified:** `src/driver/mod.rs`
- **Verification:** `cargo doc` intra-doc link resolves; clippy lib target clean.
- **Committed in:** `1a56a13`

**3. [Rule 2 - Missing Critical] `DriveError`'s own doc still described the lock variant as future work**

- **Found during:** Task 1
- **Issue:** `src/error.rs` said "plan 17-02 adds a lock-contention variant" while that variant was being added in the same edit.
- **Fix:** rewritten to name `DriveError::Lock` in the past tense; the 17-05 concurrency-cap clause is untouched, since that one is still true.
- **Files modified:** `src/error.rs`
- **Committed in:** `1a56a13`

---

**Total deviations:** 3 auto-fixed (1 bug in a test's ability to detect the thing it exists to detect, 2 stale-doc corrections). **Impact:** deviation 1 is the substantive one — without it the phase would have shipped a test that cannot distinguish a blocking acquire from a non-blocking one except by hanging. No scope creep: the public surface is exactly the plan's symbol list plus nothing.

## Issues Encountered

**`Timeout` cannot bound a synchronous syscall on the runtime it shares.** Recorded above as deviation 1, and worth carrying forward as a general note for plans 17-05 and 17-06, both of which test process-lifecycle properties around blocking calls: if the thing being bounded is a blocking syscall rather than an `await` point, the timeout has to be applied across a task or thread boundary, or it is decoration. The failure signature is a deadlocked suite, not a failed assertion.

**`rtk`'s output filtering, third instance.** Plan 17-01 recorded that `rtk` strips raw `warning:`/`test result:` lines and that its `grep` wrapper drops piped stdin. Both were worked around here from the start by running every pipeline as `rtk proxy sh -c '<whole pipeline>'` and every cargo invocation needing raw output as `rtk proxy cargo …`. No criterion in this plan passed vacuously; the counts below were all obtained through `rtk proxy`.

**One orphaned fixture per SIGKILL test.** `the_lock_is_released_when_the_holding_process_dies` kills the driver, which orphans its `fake-claude-slow.sh` child (the driver's own two-layer teardown is plan 17-06's, and SIGKILL skips it by definition). The stand-in exits on its own within about six seconds and holds no descriptor on the lock file, so it affects nothing — noted in a comment in the test rather than worked around, because working around it would mean implementing 17-06's teardown early.

## Verification Results

| Check | Result |
|---|---|
| `cargo build` | clean |
| `rtk proxy cargo test` | **447 passed, 0 failed** (baseline 439: +4 unit, +4 integration) |
| `cargo clippy -- -D warnings` | exits 0 |
| `cargo clippy --all-targets` warning count | **exactly 5**, all pre-existing (`browser.rs` ×3, `project_creator.rs` ×1, `state_reader/mod.rs` ×1); none in `src/driver/`, `src/error.rs` or `src/journal/` |
| `git diff --stat` on `Cargo.toml` / `Cargo.lock` since wave 1 | empty — no dependency added (D-08) |
| `grep -c 'truncate(true)'` in `src/driver/lock.rs`, comments filtered | `0` |
| `grep -c 'NamedTempFile\|persist('` in `src/driver/lock.rs`, comments filtered | `0` |
| `grep -c 'impl Drop'` in `src/driver/lock.rs`, comments filtered | `0` |
| `grep -ci 'nonblock'` in `src/driver/lock.rs`, comments filtered | `1` |
| `grep 'LockExclusive' src/driver/lock.rs` | 2 hits, both `NonBlockingLockExclusive` (one doc, one call); no blocking form exists |
| `grep -c 'pub fn ensure_runs_root' src/journal/writer.rs` | `1` |
| `grep -c 'ensure_runs_root' src/journal/writer.rs` | `4` |
| `grep -c 'lock::acquire' src/driver/run.rs`, comments filtered | `1`, at line 186 — before `JournalRun::start` at line 188 |
| `rtk proxy cargo test --test driver_tracer` | 4 passed — the single-run path plan 17-01 proved is unbroken |
| `rtk proxy cargo test --test journal_gitignore` | 3 passed — the ignore posture is unchanged by the refactor |

## Known Stubs

None. Every symbol this plan added is implemented and exercised; `LockError::Unavailable` is the only variant with no dedicated test, and it is an I/O-fault passthrough with no behaviour of its own beyond carrying a rendered message.

## User Setup Required

None — no external service configuration required. The integration tests run against checked-in shell fixtures with no subscription, network or quota dependency.

## Next Phase Readiness

- **17-04 (dry-run)** — unaffected. Its refusal arm sits in `drive()` *before* dispatch, so a preview never reaches `lock::acquire` and never creates a lock file, which is the correct behaviour for a mode that promises zero writes (D-23).
- **17-05 (detached spawn, reattach)** — the TUI's optional pre-check should call `lock::read_holder`, which is `pub` and read-only for exactly that purpose. **It is TOCTOU by nature and the driver's refusal remains the real gate (D-20.4)** — a pre-check that becomes the gate would reintroduce the race the lock exists to close. `LockHolder.pgid` is the same value as `RunRecord.pgid`, so reconciliation can cross-check them.
- **17-06 (kill switch)** — the lock is released when `DriverRun` drops, which happens after `journal.finish`. When 17-06 adds its SIGTERM arm to the `select!` loop, the ordering it needs is already correct: finish the journal, then fall out of scope. It must not add an explicit unlock; the module doc says why.
- **17-07 (UI)** — `DriveError::Lock`'s `Display` is already the user-facing string. `main.rs` needs no change; it prints `Error: {err}` and the `HeldBy` variant renders run id, pgid and start time itself.

One forward note: `LockHolder`'s JSON shape and the lock path are read by any concurrently-running build of this binary, so changing either later means a version in which two builds do not see each other's lock. The plan rated this reversibility "costly" and that assessment stands.

## Requirements Traceability

`CTRL-05` is declared by this plan alone in its frontmatter, but the phase's REQUIREMENTS entry is satisfied jointly — the TUI-side surfacing of the refusal belongs to 17-07. `REQUIREMENTS.md` is therefore **deliberately untouched by this plan**, matching plan 17-01's reasoning: marking a requirement complete while a sibling plan that also declares it is still unwritten is exactly the gap phase verification exists to catch. The orchestrator owns the post-wave shared-file writes.

## Self-Check: PASSED

- Both created files verified present on disk: `src/driver/lock.rs`, `tests/driver_lock.rs`.
- Both commits verified present in `git log`: `1a56a13`, `6573023`.
- All Task 1 and Task 2 acceptance criteria re-run after the final task commit; all pass.
- Plan-level verification re-run: build clean, **447 tests passing** (baseline 439), zero failures, `cargo clippy -- -D warnings` exits 0, `--all-targets` warning count still exactly 5 and all pre-existing, `Cargo.toml`/`Cargo.lock` unchanged since wave 1.
- The planted blocking acquire was verified reverted by `git diff --stat src/driver/lock.rs` returning empty against the committed file.
- No modification to `STATE.md` or `ROADMAP.md` — the orchestrator owns those writes.

---
*Phase: 17-supervisor-detach-kill-switch-dry-run-opt-in-gate*
*Completed: 2026-07-29*
