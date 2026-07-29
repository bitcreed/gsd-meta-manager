---
phase: 15-transport-foundation
plan: 07
subsystem: executor
tags: [transport, outcome-derivation, control-protocol, gap-closure]
status: complete

requires:
  - 15-04 (the control-request correlation map and the interrupt wire shape)
  - 15-05 (derive_run_outcome_from_envelopes and the D-26 derivation matrix)
provides:
  - "A single, denials-aware run-outcome entry point reachable from production"
  - "A bounded and drained control-response lifecycle (ExecutionOptions::control_response_cap)"
affects:
  - 15-08 (owns the supervisor select! loop proper; this plan fenced it out)
  - Phase 18 (queued -> delivered -> acted-on display reads the interrupt error path)
  - Phase 20 (the router acts on RunOutcome as fact)

tech-stack:
  added: []
  patterns:
    - "Collect the full wire envelope, project late: a projection taken early silently drops the one field a verdict depends on"
    - "Two independent release paths for a cross-task wait — a fast drain at lifecycle end plus a robust outer cap"
    - "Every new async integration test carries its own tokio::time::timeout so a regression FAILS rather than HANGS CI"

key-files:
  created: []
  modified:
    - src/executor/claude.rs
    - src/executor/mod.rs
    - src/executor/outcome.rs
    - tests/fixtures/fake-claude-slow.sh
    - tests/executor_transport.rs
    - tests/executor_lifecycle.rs

decisions:
  - "control_response_cap defaults to 30s — the acknowledgement is a protocol-layer reply that does not wait on a turn, so the bound is generous by orders of magnitude against the observed near-immediate response while still being a bound"
  - "Both CR-03 release paths implemented, not one: the run-end drain makes the common failure fast, the cap makes it robust against a live-but-unresponsive child"
  - "The envelope-discarding derivation entry point was DELETED rather than deprecated — two entry points onto one matrix is the defect's root cause"
  - "The per-turn projection is rebuilt after the supervisor loop so RunOutcome::Killed is byte-identical to before"

metrics:
  duration: ~35 min
  tasks: 2
  files: 6
  completed: 2026-07-29
  tests_before: 355 passed, 1 ignored
  tests_after: 359 passed, 1 ignored
---

# Phase 15 Plan 07: Gap Closure — Permission-Denied Reporting and the Interrupt Lifecycle Summary

Wired the terminal envelope's `permission_denials[]` from the wire through the real `Coordinator` into the outcome derivation, and closed the control-response lifecycle at both ends so an unanswered `control_request` can never park its caller.

## What shipped

Two shipped defects, both **call-site and lifecycle wiring** rather than missing logic — in each case the correct implementation already existed in the codebase and simply was not reached.

### GAP 1 (SC-2 / TRANS-02 / CR-04) — a blocked run reported as success

`Coordinator::run` collected `Vec<TurnOutcome>` projections and called the derivation entry point that hard-coded `Vec::new()` for its denials source. A `--permission-mode dontAsk` run that was blocked from doing anything therefore reported as `SucceededNoChanges`: every verdict field on its terminal envelope reads clean (`subtype: success`, `is_error: false`, `terminal_reason: completed`), and `permission_denials[]` — the only signal that says otherwise — was dropped by the projection before the derivation could see it.

- The coordinator now collects `Vec<ResultMessage>`; `handle_item` pushes the envelope verbatim off the wire before the box moves into `ExecutionEvent::TurnCompleted`.
- The per-turn projection is rebuilt after the supervisor loop, so `RunOutcome::Killed { turns }` is byte-identical to before.
- The deliberate absence of a `break` in the `Result` arm is intact: `result` closes a TURN, never the run (D-29). The multi-turn tests are unchanged and green.

### GAP 2, interrupt half (SC-1 / TRANS-01 / CR-03) — an unanswered interrupt hangs its caller

`interrupt()` registered a oneshot in `pending_control` and awaited it with no timeout, and nothing anywhere cleared that map at run end. `SendError::ControlResponseLost` was unreachable dead code. Both release paths the review specifies are now implemented, because they cover different failures:

| Path | Releases when | Proven by |
|---|---|---|
| Run-end drain | the run ends for any reason | `an_unanswered_interrupt_is_released_by_the_run_end_drain` |
| `control_response_cap` | the child is alive and simply not answering | `an_unanswered_interrupt_on_a_live_child_is_released_by_the_control_response_cap` |

An acknowledgement remains acceptance and never cancellation — nothing added here reports a cancellation on the strength of an ack (D-31).

## Re-verification facts (requested by the plan's `<output>`)

- **The production call site that now reaches the denials-aware derivation:** `src/executor/claude.rs:990` — `None => derive_run_outcome_from_envelopes(&envelopes, status, &before, &after),` (import at `src/executor/claude.rs:68`). The verification report cited the old, wrong call site at claude.rs:962.
- **The envelope-discarding entry point is DELETED, not merely unused.** `grep -rn "derive_run_outcome" src/ | grep -v "_from_envelopes"` returns **zero hits** across the whole `src/` tree — the symbol, its body, and the module-doc reference to it are all gone. `grep -cE "^pub fn derive_run_outcome" src/executor/outcome.rs` returns exactly `1`, and that line is `pub fn derive_run_outcome_from_envelopes(` at outcome.rs:173.
- **Chosen default for `control_response_cap`: `Duration::from_secs(30)`.** The `control_response` is a protocol-layer reply that does not depend on the model finishing a turn — the observed acknowledgement is near-immediate, and in golden transcript 07 it arrived before the target turn had even been dequeued. 30 seconds is therefore generous by orders of magnitude against healthy behaviour while still being a bound, which is the whole requirement: no caller may be parked for the process lifetime (D-13, D-31). It is configurable per run and is asserted bounded and non-zero by `the_default_control_response_cap_is_bounded_and_non_zero`.
- **Measured wall-clock duration of each new test** (run individually, `cargo test --test <suite> <name>`):

  | Test | Duration | Inner hard timeout |
  |---|---|---|
  | `a_permission_blocked_run_is_reported_as_permission_denied_not_success` | 0.01s | 30s |
  | `an_unanswered_interrupt_is_released_by_the_run_end_drain` | 0.61s | 15s |
  | `an_unanswered_interrupt_on_a_live_child_is_released_by_the_control_response_cap` | 0.41s | 15s |

  All three are two orders of magnitude inside their own bounds. Both interrupt tests were observed FAILING on that inner timeout in the RED phase (15.01s for the pair) rather than hanging the suite — the property the hard timeouts exist for was directly exercised, not merely asserted.
- **`src/main_loop.rs` and `src/executor/gate.rs` were NOT touched.** `git status --porcelain src/main_loop.rs src/executor/gate.rs` returns empty output on a clean tree, and neither file appears in either commit's diff. SC-3 and SC-4 were independently verified clean and were fenced out of this plan.

## Task commits

| Task | Name | Commit | Files |
|---|---|---|---|
| 1 (tracer, TDD) | Report permission-denied runs from the full result envelopes | `7c3c62e` | claude.rs, outcome.rs, executor_transport.rs, fake-claude-slow.sh |
| 2 (TDD) | Bound and drain the control-response wait | `02b9416` | claude.rs, mod.rs, executor_lifecycle.rs |

## TDD gate compliance

Both tasks ran a genuine RED before GREEN, and both REDs were the real defect rather than a compile error:

- **Task 1 RED:** the new end-to-end test reported `SucceededNoChanges { turns: [TurnOutcome { subtype: "success", is_error: false, terminal_reason: Some("completed"), .. }] }` — an exact reproduction of CR-04 against a real spawned child, failing in 0.01s.
- **Task 1 revert check** (required by the plan's acceptance criteria): the outcome-match line was temporarily reverted to strip the denials source, reproducing `SucceededNoChanges` in 0.01s, then restored and the full suite re-run green. The regression fails fast; it does not hang.
- **Task 2 RED:** both interrupt tests failed with `Elapsed(())` on their own inner 15-second timeouts — the unbounded await, caught by the bound the tests carry.

Commit subjects use `fix(15-07):` rather than the `test(...)`/`feat(...)` RED/GREEN split, because these are plan-type `execute` tasks with `tdd="true"` (per-task atomic commits), not plan-level `type: tdd` gates. The RED evidence is recorded above.

## Verification

Run from the project root, all green:

| Check | Result |
|---|---|
| `cargo build` | exit 0 |
| `cargo test` | **359 passed, 0 failed, 1 ignored** (baseline was 355 passed, 1 ignored; +4 = 1 transport, 2 lifecycle, 1 mod.rs unit) |
| `cargo clippy -- -D warnings` | exit 0, no issues |
| `cargo clippy --all-targets` | **exactly 5 warnings**, all pre-existing: 3x `src/browser.rs:131-133`, 1x `src/state_reader/mod.rs:258`, 1x `src/project_creator.rs:146`. None in any file this plan modified. |
| `cargo test --test executor_transport a_permission_blocked_run_...` | 1 passed |
| `cargo test --test executor_lifecycle an_unanswered_interrupt` | 2 passed |
| `cargo test --test executor_transport interrupt` | 2 passed — the answered-interrupt correlation tests are unregressed |
| `grep -cE "^[[:space:]]*#\[ignore" tests/executor_lifecycle.rs` | `1` — unchanged; no new ignored test |
| `git status --porcelain src/main_loop.rs src/executor/gate.rs` | empty |

Acceptance-criteria greps, all satisfied: `derive_run_outcome_from_envelopes` in claude.rs = 2; `^pub fn derive_run_outcome` in outcome.rs = 1; `derive_run_outcome\(` in outcome.rs = 0 (was 7) and in claude.rs = 0 (was 1); `envelopes: &mut Vec<ResultMessage>` = 1; `"permission_denials":\[{` in the fixture = 1; `home-blk|/home/` in the fixture = 0; `timeout(handle.control_response_cap` = 1; `pending_control.lock().await.clear()` = 1; `control_response_cap` in mod.rs = 5.

## Deviations from Plan

**1. [Rule 3 — Blocking] The `cap` local in `interrupt` was removed in favour of the direct expression**

- **Found during:** Task 2, verifying acceptance criteria.
- **Issue:** The plan permitted reading the cap into a local *"if the borrow checker requires it"*. I wrote it that way first, which made the acceptance grep `timeout(handle.control_response_cap` return `0`.
- **Fix:** The borrow checker does **not** require the local — `Duration` is `Copy`, so reading the field creates no borrow that outlives the expression and the subsequent `handle.pending_control.lock()` stays legal. Switched to `tokio::time::timeout(handle.control_response_cap, rx)`, which both satisfies the criterion and reads better at the call site. Verified by a clean compile and a full green suite.
- **Files modified:** `src/executor/claude.rs`
- **Commit:** `02b9416`

**2. [Judgment] The permission-denied test does not call `close_input()`**

- **Found during:** Task 1.
- **Issue:** The plan's shape for the test followed `tracer_runs_one_command_end_to_end`, which calls `handle.close_input()`. With the `denied` ending the stand-in exits almost immediately, so `close_input()` can legitimately return `Err(WriterGone)` and an `.expect()` on it would make the test flaky for a reason unrelated to what it proves.
- **Fix:** Omitted the call entirely and awaited `wait_outcome()` directly inside the hard timeout. The run still ends cleanly (stdout EOF on child exit), and the test asserts only the outcome. No behaviour under test is skipped.
- **Files modified:** `tests/executor_transport.rs`
- **Commit:** `7c3c62e`

No architectural changes were needed; no Rule 4 decisions arose. No package-manager installs (`Cargo.toml` and `Cargo.lock` are untouched, as the threat model requires).

## Known Stubs

None. Both gaps are closed in production code with regression tests against real spawned children; nothing was left as a placeholder, and no `TODO`/`FIXME` marker was introduced.

## Threat Flags

None. No new network endpoint, auth path, file-access pattern or trust-boundary schema change was introduced. The four `mitigate` dispositions in the plan's threat register were all applied:

- **T-15-40** (repudiation, wrong outcome): the full `Vec<ResultMessage>` reaches the derivation and the envelope-discarding entry point is deleted, so there is no path back to the wrong answer.
- **T-15-41** (DoS, parked caller): two independent bounded release paths, each regression-tested.
- **T-15-42** (info disclosure, fixture): the `denied` denial record is fully synthetic with the relative filename `README.md`; both the literal and dash-encoded host-path forms grep to zero.
- **T-15-43** (tampering, envelope collection): envelopes are cloned verbatim off the wire; no branch of the derivation reads the prose `result` field, and the derivation body was not modified.
- **T-15-44** (info disclosure, interrupt diagnostics): `ControlResponseLost` carries only the driver-generated request id. No `tracing` call was added, and no raw stream line, denial record body or control-request body is logged at any level.

## Notes for the re-verification pass

- The one behaviour worth re-reading rather than re-grepping is the interaction between the run-end drain and `interrupt`'s `Ok(Err(_))` arm: the drain clears the map (dropping senders), and the waiter observes a dropped sender rather than an elapsed timeout. That is why the drain test asserts `elapsed < 10s` against a 30-second default cap — the timing is what discriminates the two paths, and collapsing them into one assertion would let either regress silently.
- `src/main_loop.rs` and `src/executor/gate.rs` remain the property of SC-3/SC-4 and of plan 15-08; the supervisor `select!` loop (CR-01/CR-02) is untouched here and is still open.

## Self-Check: PASSED

- `src/executor/claude.rs` — FOUND
- `src/executor/mod.rs` — FOUND
- `src/executor/outcome.rs` — FOUND
- `tests/fixtures/fake-claude-slow.sh` — FOUND
- `tests/executor_transport.rs` — FOUND
- `tests/executor_lifecycle.rs` — FOUND
- commit `7c3c62e` — FOUND
- commit `02b9416` — FOUND
