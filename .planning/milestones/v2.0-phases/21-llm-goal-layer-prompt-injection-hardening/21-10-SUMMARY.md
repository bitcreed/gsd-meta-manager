---
phase: 21-llm-goal-layer-prompt-injection-hardening
plan: 10
subsystem: driver
tags: [auditability, single-call-site, source-guard, opt-in, disclosure-drift, tdd, rust]

# Dependency graph
requires:
  - phase: 21-llm-goal-layer-prompt-injection-hardening (plan 21-02)
    provides: "escalate::EscalationBudget, its used()/cap() readings, and the escalations_used field this plan makes unconditional"
  - phase: 21-llm-goal-layer-prompt-injection-hardening (plan 21-03)
    provides: "the SHA-256 prompt_inputs disclosure digests whose drift the opt-in revert was laundering"
  - phase: 21-llm-goal-layer-prompt-injection-hardening (plan 21-04)
    provides: "the decomposition wired ABOVE execute_run, which is why a goal-driven run has always spent one consultation before any kill path can fire"
provides:
  - "driver::run::finish_run(&mut JournalRun, &str, u32) — the single stamped terminal write every production path in the module uses"
  - "shutdown_on_terminate(.., escalations_used: u32) and shutdown_during_startup(.., escalations_used: u32)"
  - "tests/spawn_seam_guard.rs guard six: one terminal-write call site in src/driver/run.rs's production region, with its two over-approximations stated"
  - "do_toggle_opt_in's save-failure revert restoring a clone of the record the user granted rather than calling the sole DriverOptIn constructor"
affects: [21-verification, 21-secure-phase, driver-terminal-records, tui-opt-in-toggle]

actuals:
  tokens: 6780
  tasks: 2
  commits: 4

tech-stack:
  added: []
  patterns:
    - "A property every branch must remember is a property a fourth branch will forget: collapse it to one call site so a new path inherits it, and let a source guard prove the site stays single"
    - "A guard states its own over-approximations in its doc rather than implying exactness; a guard read as exact and quietly approximate is a defect this repository has already paid for"
    - "A revert restores a clone of what existed; it never calls a constructor, because a constructor re-derives its inputs from the world as it is NOW rather than as it was when the user consented"
    - "A production/test region boundary found by a column-zero line marker beats a per-function allowlist: the exclusion is by construction and nobody has to maintain it"
    - "The equality assertion IS the proof the sole constructor was not called — it re-stamps its timestamp even when nothing on disk moved, so no re-minted record can compare equal"

key-files:
  created: []
  modified:
    - src/driver/run.rs
    - src/ui/screens/driver_confirm.rs
    - tests/spawn_seam_guard.rs

key-decisions:
  - "finish_run wraps JournalRun::finish rather than narrowing it: finish and set_escalations_used are used from other modules and from tests, so changing them would push this refactor outside the driver"
  - "The count is a PARAMETER on the two shutdown helpers rather than something they read: the budget is owned by execute_run, and threading it is what keeps the helpers free of a second source of truth"
  - "The normal path's set_escalations_used line was moved into the helper, with a comment saying where it went, so a reader diffing this change cannot read the deletion as a removal of the stamp"
  - "The spawn-failure behaviour test exercises the helper with the arm's own label rather than manufacturing a failing spawn; that the arm CALLS the helper is a source property the guard carries, and the test says so instead of implying it proved more"
  - "The guard's region boundary is a column-zero `mod tests {` marker, and both its over-approximations (marker-based boundary, line-comment-only filter — IN-03) are named in its own doc rather than implied away"
  - "The withdrawal revert is a field assignment on the entry, not a `record_opt_in` call: the value assigned is a clone of a record the user already constructed by opting in, so registry.rs's sole-constructor claim (D-14) stays true"
  - "The unrestorable-registry note is kept and re-derived from whether the entry was found, with its currently-unreachable precondition named rather than asserted away"
  - "save_config is made to fail by a config path whose parent COMPONENT is a regular file, not by a read-only directory: a permission bit does not stop a process running as root"
  - "REQUIREMENTS.md is deliberately NOT touched, following 21-09's precedent and commit 828d7cc's premature-Complete revert"

patterns-established:
  - "A behavioural test fixture parameterised so its read-back is honest evidence: started_run_with_cap exists because `escalation_cap: 0` beside `escalations_used: 1` would be an arithmetically odd record to quote as proof"
  - "A guard's control arm asserts BOTH directions over synthetic source — the helper's own write attributed to the helper, a bare write attributed to its own function — so neither a scanner that reports nothing nor one that reports everything satisfies the emptiness assertion"

requirements-advanced: [DRIVE-04, SAFE-07]

coverage:
  - id: D1
    description: "A terminate-signal shutdown's terminal record names the consultations the run spent, rather than writing null on a run that had already been decomposed"
    requirement: DRIVE-04
    verification:
      - kind: unit
        ref: "src/driver/run.rs#a_terminate_signal_shutdown_records_the_consultations_the_run_spent"
        status: pass
      - kind: other
        ref: "RED against the pre-change build: the lib test target did not compile, because the parameter carrying the count did not exist"
        status: pass
    human_judgment: false
  - id: D2
    description: "A startup-kill shutdown's terminal record names the consultations the run spent, and its `killed` label is unchanged by the stamp"
    requirement: DRIVE-04
    verification:
      - kind: unit
        ref: "src/driver/run.rs#a_startup_kill_records_the_consultations_the_run_spent"
        status: pass
      - kind: other
        ref: "Read-back of the real run.json this test writes, quoted verbatim below"
        status: pass
    human_judgment: false
  - id: D3
    description: "A spawn failure's terminal record names the consultations the run spent"
    requirement: DRIVE-04
    verification:
      - kind: unit
        ref: "src/driver/run.rs#a_spawn_failure_records_the_consultations_the_run_spent"
        status: pass
      - kind: other
        ref: "That the arm reaches the helper rather than `finish` is the guard's assertion, not this test's; stated in the test's own doc"
        status: pass
    human_judgment: false
  - id: D4
    description: "Exactly one function in src/driver/run.rs's production region writes a run's terminal label, so a fifth terminal path inherits the stamp rather than having to remember it"
    requirement: DRIVE-04
    verification:
      - kind: unit
        ref: "tests/spawn_seam_guard.rs#every_terminal_write_in_the_driver_run_goes_through_the_stamped_helper"
        status: pass
      - kind: unit
        ref: "tests/spawn_seam_guard.rs#the_terminal_write_scanner_reports_a_bare_call_and_not_the_helpers_own"
        status: pass
      - kind: other
        ref: "RED against the pre-change build: the guard FAILED naming all four sites (1005, 1126, 2826, 3305); message quoted verbatim below"
        status: pass
    human_judgment: false
  - id: D5
    description: "A failed save after an opt-in withdrawal restores the record the user granted — same opted_in_at, same prompt_inputs, same digests — rather than minting a fresh one"
    requirement: SAFE-07
    verification:
      - kind: unit
        ref: "src/ui/screens/driver_confirm.rs#a_failed_save_after_a_withdrawal_restores_the_record_the_user_granted"
        status: pass
      - kind: other
        ref: "RED against the pre-change build: failed on opted_in_at AND on the whole prompt_inputs list; diff quoted verbatim below"
        status: pass
    human_judgment: false
  - id: D6
    description: "Disclosed-file drift that should have required re-confirmation cannot be laundered into an approved state by a failed withdrawal"
    requirement: SAFE-07
    verification:
      - kind: unit
        ref: "src/ui/screens/driver_confirm.rs#a_failed_save_after_a_withdrawal_does_not_rebaseline_a_drifted_disclosure"
        status: pass
      - kind: other
        ref: "RED against the pre-change build: the revert had adopted sha256:b5441fb4… — the digest of the file rewritten after the opt-in"
        status: pass
    human_judgment: false
  - id: D7
    description: "registry::record_opt_in remains the only function outside tests that constructs a DriverOptIn: no revert path calls it"
    requirement: SAFE-07
    verification:
      - kind: unit
        ref: "src/ui/screens/driver_confirm.rs#a_failed_save_after_a_withdrawal_restores_the_record_the_user_granted (equality is the proof: record_opt_in re-stamps opted_in_at even when nothing on disk moved, so no re-minted record can compare equal)"
        status: pass
      - kind: other
        ref: "Source assertion over do_toggle_opt_in's body: the only `registry::` calls are is_opted_in, record_opt_in (apply path only) and clear_opt_in"
        status: pass
    human_judgment: false
  - id: D8
    description: "Either direction, a failed save still tells the user the SAVE is what failed, and the grant-direction revert still leaves no record at all"
    requirement: SAFE-07
    verification:
      - kind: unit
        ref: "src/ui/screens/driver_confirm.rs#a_failed_save_names_the_save_failure_in_both_directions"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver_confirm.rs#a_failed_save_after_a_grant_leaves_no_record_at_all"
        status: pass
    human_judgment: false

duration: 25min
completed: 2026-08-20
status: complete
---

# Phase 21 Plan 10: One Stamped Terminal Write, and a Revert That Restores Summary

**The driver's four production terminal writes now go through one helper that stamps the run's model-consultation count, with a source guard that keeps the site single; and a failed save after an opt-in withdrawal puts back the record the user actually granted instead of minting a fresh approval over whatever the disclosed files contain now.**

## Performance

- **Duration:** ~25 min
- **Started:** 2026-08-21T02:47Z
- **Completed:** 2026-08-21T03:08Z
- **Tasks:** 2/2
- **Files modified:** 3

## Accomplishments

- **WR-02 is closed at the shape, not at the three call sites.** `set_escalations_used` was called once, immediately before the normal path's `finish`. Three other production paths — `shutdown_on_terminate`, `shutdown_during_startup` and the spawn-failure arm — called `finish` without it. A goal-driven run always spends one consultation before any of them can fire, because the decomposition happens above `execute_run`, so those runs wrote `escalation_cap: Some(n)` with `escalations_used: null` — which is byte-for-byte what a build predating the counter writes. The ambiguity the field's own doc says it exists to remove, reintroduced on exactly the killed and failed-to-spawn runs an operator opens an investigation with.
- **The durable half is the guard, and it was written to fail first.** Fixing three call sites fixes three call sites; the fourth terminal path somebody adds next year is the one that matters. `finish_run` is now the only place in `src/driver/run.rs`'s production region that writes a terminal label, and `tests/spawn_seam_guard.rs` proves the site stays single — the same technique `DrivableProject::from_registry` and the goal-decomposition capability already use, and a property a test can check where "every branch remembered" is not.
- **The guard states what it over-approximates.** Its region boundary is a column-zero `mod tests {` marker, not a parse, and `executable_lines`'s comment filter handles line comments only (IN-03). Both fail in the over-detection direction — loud, not silent — and both are named in the guard's own doc. A guard read as exact and quietly approximate is a defect this repository has already recorded once, so it is not repeated in the guard written to prevent a different one.
- **WR-04 was laundering exactly the drift the digests were added to catch.** When a withdrawal succeeded in memory but `save_config` failed, the revert called `registry::record_opt_in` — the *only* constructor of a `DriverOptIn` (D-14), which stamps a fresh `opted_in_at` and takes a fresh `current_prompt_inputs` snapshot. A project whose `CLAUDE.md` was rewritten under a `git pull`, and which the spawn gate would have refused with `PromptInputsDrifted`, came back with the new bytes **already approved** — no disclosure shown, no user act — and the next successful save persisted it. The RED test reproduced that verbatim.
- **The revert is now an assignment of a clone, and that is what preserves the sole-constructor property rather than violating it.** Nothing new comes into existence, so a `Some(record)` in a `config.json` is still proof of a deliberate user action — the claim `registry.rs:96-99` makes and which this call site had become a counterexample to.
- **The whole suite is green.** 1193 tests across 35 binaries, 0 failures. Lib rose 1011 → 1018 (+3 escalation-stamp, +4 opt-in revert); `spawn_seam_guard` rose 22 → 24.

## Task Commits

Both tasks were `tdd="true"`, so each behaviour-adding change has a RED before it:

1. **Task 1 (RED): failing proof that three terminal writes drop the escalation count** — `c74a7f0` (test)
2. **Task 1 (GREEN): one stamped terminal write, so no path can forget the count** — `78ff0e1` (fix)
3. **Task 2 (RED): failing proof that a failed withdrawal launders disclosure drift** — `d208efe` (test)
4. **Task 2 (GREEN): the opt-in revert restores the prior record instead of minting one** — `fbc6a46` (fix)

**Plan metadata:** committed with this SUMMARY (docs).

_TDD gate sequence: `test(...)` → `fix(...)`, in that order, twice. No `refactor(...)` commit — neither GREEN implementation left anything to clean up. `fix` rather than `feat` on both GREENs, because both are corrections of shipped behaviour rather than new surface._

## Files Created/Modified

- `src/driver/run.rs` — `finish_run` with the doc stating the property it exists to hold; `escalations_used: u32` added to `shutdown_on_terminate` and `shutdown_during_startup`; all four production terminal writes routed through the helper (`budget.used()` passed from the three call sites inside `execute_run`); the normal path's `set_escalations_used` line moved into the helper with a comment saying where it went; `started_run_with_cap` beside `started_run`; three in-module behaviour tests and a `finished_record` read-back helper.
- `tests/spawn_seam_guard.rs` — guard six and its control arm: `TERMINAL_WRITE_CALL`, `TERMINAL_WRITE_HELPER`, `TERMINAL_WRITE_HOME`, `TEST_REGION_MARKER`, `test_region_start`, and a 30-line header stating the property, the reason a source guard carries it, and both over-approximations.
- `src/ui/screens/driver_confirm.rs` — `do_toggle_opt_in` captures `previous` before applying and assigns it back on a withdrawal-direction save failure; the sole-constructor rationale lives at the revert itself; the unrestorable-registry note re-derived from whether the entry was found; four in-module tests plus `unwritable_config_path`, `granted_record`, `grant` and `toggle` helpers.
- `.planning/phases/21-.../deferred-items.md` — two pre-existing parallel-execution flakes, with the evidence that they are flakes and that this plan cannot be their cause.

## The read-back the plan required, verbatim

### `escalations_used` on a killed goal-driven run's `run.json`

Written by the production `shutdown_during_startup` path through `finish_run`, read back off disk. Captured with a temporary `eprintln!` that was removed before the GREEN commit:

```json
{
  "run_id": "2026-08-20T00-00-00Z-startup",
  "goal": "",
  "gsd_command": "/gsd-progress",
  "target_phase": null,
  "bounds": {
    "max_steps": 20,
    "wall_clock_cap_secs": 14400
  },
  "approved_plan": null,
  "escalation_cap": 4,
  "escalations_used": 1,
  "target": "Host",
  "opt_in": null,
  "started_at": "2026-08-21T02:59:14Z",
  "session_id": "9ab43d76-2b01-494f-bb70-32dcad1c6b50",
  "pid": 979227,
  "pgid": 979227,
  "claude_code_version": "",
  "argv_digest": "fnv1a64:0000000000000000",
  "ended_at": "2026-08-21T02:59:14Z",
  "outcome": "killed"
}
```

The load-bearing line is `"escalations_used": 1` beside `"outcome": "killed"`. **Against the pre-change build that field reads `null`** — `make_run_record` writes `escalations_used: None` at write one (`src/driver/run.rs:760`) and nothing on this path ever stamped it. The fixture's cap is `4` rather than the shared fixture's `0` deliberately: a record reading `escalation_cap: 0` beside `escalations_used: 1` would be an arithmetically odd thing to quote as evidence, so `started_run_with_cap` exists.

### The guard's failure message, verbatim

From the RED run against the unmodified tree:

```
a terminal write in src/driver/run.rs's production region does not go through `finish_run`. That helper stamps the run's model consultation count before it finishes the journal, and it is the ONLY place a run's ending is written so that a new terminal path inherits the stamp rather than having to remember it. A bare `.finish(` here writes `escalations_used: null` on a run that spent consultations, which reads identically to a record from a build that predates the counter — on exactly the killed and failed-to-spawn runs a reader is auditing (WR-02, DRIVE-04). Offending lines:
  src/driver/run.rs:1005: if let Err(err) = journal.finish(&label) {
  src/driver/run.rs:1126: if let Err(err) = journal.finish("killed") {
  src/driver/run.rs:2826: if let Err(journal_err) = run.journal.finish("spawn_failed") {
  src/driver/run.rs:3305: run.journal.finish(&label).map_err(|err| DriveError::Journal {
```

Those are the three lines `21-VERIFICATION.md` named plus the normal path, found independently by the scan rather than listed by hand.

### The laundered approval, verbatim

`a_failed_save_after_a_withdrawal_does_not_rebaseline_a_drifted_disclosure`, against the pre-change build:

```
  left: [PromptInput { path: "CLAUDE.md", digest: Some("sha256:b5441fb4b0a06b4bef9dab70d333f3a19885caae6748051d9224b2cebb87c935"), extra: {} }, PromptInput { path: ".planning/STATE.md", digest: None, extra: {} }, ...]
 right: [PromptInput { path: "CLAUDE.md", digest: Some("sha256:aaaa"), extra: {} }, PromptInput { path: ".planning/STATE.md", digest: Some("sha256:bbbb"), extra: {} }]
```

`sha256:b5441fb4…` is the digest of the `CLAUDE.md` the test wrote **after** the opt-in, standing in for a `git pull`. The revert had adopted it. `sha256:aaaa` is what the user actually approved, and what the spawn gate compares against.

The sibling test's failure showed the other half:

```
  left:  DriverOptIn { opted_in_at: "2026-08-21T03:01:16Z", ... }
 right:  DriverOptIn { opted_in_at: "2020-01-01T00:00:00Z", ... }
```

A fresh timestamp on a withdrawal the user asked for and that never reached disk.

## The guards can actually fail

Every new guard failed against a real unmodified build rather than being reasoned about — no probe was needed, because each was written as its task's RED.

| Guard | How it was shown to fail | Result |
|---|---|---|
| `every_terminal_write_in_the_driver_run_goes_through_the_stamped_helper` | RED against the unmodified tree | FAILED, naming all four sites |
| `the_terminal_write_scanner_reports_a_bare_call_and_not_the_helpers_own` | control arm over synthetic source, both directions in one assertion | passes on both builds by design — its job is to stop the guard above passing vacuously |
| the three escalation-stamp tests | RED against the unmodified tree | did not compile: the parameter and the helper did not exist |
| `a_failed_save_after_a_withdrawal_restores_the_record_the_user_granted` | RED against the unmodified tree | FAILED on `opted_in_at` and on the whole `prompt_inputs` list |
| `a_failed_save_after_a_withdrawal_does_not_rebaseline_a_drifted_disclosure` | RED against the unmodified tree | FAILED, showing the adopted post-`git pull` digest |
| `a_failed_save_after_a_grant_leaves_no_record_at_all`, `a_failed_save_names_the_save_failure_in_both_directions` | pass on both builds | by design: those arms are unchanged, and their job is to prove the fix does not regress them |

**Honesty about the two that pass on both builds.** The grant-direction and message tests are not skipped gates and are not claimed as REDs. `clear_opt_in` was already the correct revert in that direction and the message shape was already correct; those tests exist so the withdrawal fix cannot quietly change the arms it does not touch.

**Honesty about the spawn-failure test.** It calls `finish_run` with the arm's own `spawn_failed` label rather than manufacturing a failing spawn inside the iteration loop. What it proves is the helper's behaviour; that the *arm* reaches the helper is the guard's assertion, and the test's own doc says so rather than implying it proved more.

## Decisions Made

1. **`finish_run` wraps rather than narrows.** `JournalRun::finish` and `set_escalations_used` are untouched: both are used from other modules and from tests, and restricting them would have pushed this refactor outside the driver, which the plan explicitly forbade.
2. **The count is threaded, not read.** The budget is owned by `execute_run`, so the two shutdown helpers take `escalations_used: u32` and the call sites pass `budget.used()`. A helper that reached for the budget itself would be a second source of truth for one number.
3. **The moved line is documented at the site it left.** A reader diffing this change sees a `set_escalations_used` deletion on the normal path; without the comment that reads as a removal of the stamp, which is the opposite of what happened.
4. **The guard's boundary is a marker, and it says so.** The alternative to the two stated over-approximations is a Rust parser in a test, which the plan's own threat register dispositions as `accept` for exactly that reason (T-21-10-03).
5. **`started_run_with_cap` rather than editing the shared fixture.** Three tests need a non-zero cap for their read-back to be honest evidence; every other caller of `started_run` still gets the zero it was written against, so nothing else moved.
6. **The revert assigns a clone and the reason lives at the assignment.** `registry.rs:96-99`'s claim that a `Some(record)` is proof of a deliberate user act depends on this call site not being a counterexample, so the argument is written where a future editor of this line will read it — not in a plan file.
7. **The unrestorable-registry note is kept, and its unreachability is named rather than asserted away.** Nothing in this process can make the alias vanish between the toggle and the revert — both `registry` calls bail on an unknown alias before the save is attempted — so the message is derived from an outcome rather than from a claim, and the test asserts the note is *absent* on the reachable paths.
8. **`REQUIREMENTS.md` deliberately not touched.** Following 21-09's precedent and commit `828d7cc`'s premature-Complete revert: this plan *advances* DRIVE-04 and SAFE-07, and whether they are Complete is `/gsd-verify-work`'s finding, not an executor's.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing critical] The plan's fixture would have produced a record that undermined its own read-back**

- **Found during:** Task 1, at the first `run.json` read-back.
- **Issue:** The plan's `<output>` requires the SUMMARY to quote `escalations_used` from a killed run's `run.json` verbatim, as the evidence the fix works. The shared `started_run` fixture hard-codes `escalation_cap: 0`, so the record read `"escalation_cap": 0` beside `"escalations_used": 1` — arithmetically impossible for a real run, and quoting it as proof would invite the reader to distrust the fixture rather than believe the field.
- **Fix:** Added `started_run_with_cap(run_id, escalation_cap)`; `started_run` delegates to it with `0`, so no existing caller changed. The three new tests take a cap of `4`.
- **Files modified:** `src/driver/run.rs`
- **Verification:** `rtk proxy cargo test --lib driver::run` — 30 passed; the quoted record now reads `"escalation_cap": 4, "escalations_used": 1`.
- **Committed in:** `78ff0e1` (Task 1 GREEN)

### Line references re-derived

The plan was written before waves 1 and 2 landed, and its line numbers were stale. Every reference was re-derived against the current tree before editing:

| Plan reference | Actual, on this tree |
|---|---|
| `shutdown_on_terminate`'s `finish` at `:1005` | `:1005` — unchanged |
| `shutdown_during_startup`'s `finish("killed")` at `:1126` | `:1126` — unchanged |
| spawn-failure `finish("spawn_failed")` at `:2808` | `:2826` |
| `set_escalations_used` at `:3285` | `:3303`, with its `finish` at `:3305` |
| the `escalations_used` doc at `src/journal/mod.rs:1240-1265` | `:1437-1452` |
| `executable_lines` at `tests/spawn_seam_guard.rs:261-270` | `:262-271` |
| in-module `finish` calls at `:3871`, `:4059`, `:4176` | `:3889`, `:4077`, `:4194` |

Nothing waves 1–2 landed was reverted. `src/driver/run.rs`'s spawn-gate approval re-check — 21-09's work — was not touched.

---

**Total deviations:** 1 auto-fixed (1 missing-critical)
**Impact on plan:** No scope creep. The fix is a test fixture addition of eleven lines that changes no existing caller's behaviour.

## Deferrals

The phase's deferral table is carried forward here unchanged, so no finding from `21-REVIEW.md` or `21-VERIFICATION.md` is silently dropped. **None of these is part of any recorded gap.**

| Finding | Severity | Deferred because |
|---|---|---|
| WR-03 (two escalation refusals name an action the caller cannot take, or state something untrue) | Warning | Message-only defect on the `--max-steps 1` edge; not part of either recorded gap and not on the review-integrity chain this gap-closure set exists to repair. Carry into the next phase's review backlog. |
| WR-06 (the opt-in disclosure is unwrapped and unscrollable, so the residual-exposure block clips on a small terminal) | Warning | A real disclosure defect, but it needs a buffer-level render test against a small `Rect` and belongs with UI work rather than in a gap-closure set scoped to the approval chain. |
| WR-07 (the disclosure the user reads and the list `record_opt_in` writes are two independent disk reads) | Warning | Shares a file with Task 2 but is a different fix (snapshot on screen construction plus a `record_opt_in_with` variant). Bundling it would have widened Task 2 from a two-line restore into a signature change across `registry.rs`, diluting the fix WR-04 needs. **Still true after this plan:** the revert now restores a snapshot, but the *grant* path still re-reads. |
| WR-08 (the hostile `CLAUDE.md` sits at an auto-loadable path in this repository) | Warning | Fixture-layout change plus a materialisation rename; touches `tests/fixtures/injection-corpus/` and `tests/driver_injection_corpus.rs`, neither of which any plan in this set opens. |
| WR-09 (`consult_model_seam` discards a valid payload when a later turn reports none) | Warning | Fails in the safe direction (the run parks rather than proceeding on bad evidence); not on the review-integrity chain. |
| IN-02, IN-03 | Info | Latent-trap notes, no live defect. **IN-03 is now named explicitly** in guard six's doc as one of its two stated over-approximations, rather than left implicit. |
| Two integration-test binaries flaky under parallel execution (`envelope_tracer`, `driver_reattach`) | — | **Newly found here**, not a finding of either review. Pre-existing concurrency flakes in files no `21-*` plan opens; evidence in `deferred-items.md`. |

IN-01 was **not** deferred — plan `21-09` Task 2 removed the throwaway `ApprovedPlan` it named.

## Issues Encountered

- **`cargo fmt --check` is not clean on this tree, and was not made clean.** The same version drift `21-07-SUMMARY.md` and `21-09-SUMMARY.md` recorded: the installed rustfmt disagrees with committed formatting in files this plan never touches, and running `cargo fmt` would reformat unrelated files, which the scope boundary forbids. Lines added here were hand-matched to the surrounding style. The gate this plan is judged by is `cargo clippy -- -D warnings` on the **lib** target, which is clean.
- **`rtk` filters `warning:` and `test result:` lines**, so every `cargo` invocation went through `rtk proxy` and every PASS/FAIL below was read from unfiltered output.
- **Two test binaries are flaky under full parallelism**, and it cost a detour to establish they are not regressions. `cargo test --test driver_reattach` returned FAILED, FAILED, ok on three consecutive runs of **one unchanged binary**, and ok 3/3 with `--test-threads=1`. Neither file is in any `21-*` plan's `<files>`, and both failing assertions are about the run record at write **one**, which this plan does not touch. Logged to `deferred-items.md` rather than fixed.

## Verification Results

All six items from the plan's `<verification>` block:

| # | Gate | Result |
|---|------|--------|
| 1 | `rtk proxy cargo build --all-targets` | clean, exit 0 |
| 2 | `rtk proxy cargo clippy -- -D warnings` (lib gate) | clean, exit 0 |
| 3 | `rtk proxy cargo test` (whole suite) | **1193 tests across 35 binaries, 0 failures, exit 0** at `--test-threads=2`. At full parallelism two unrelated binaries flake — see Issues Encountered and `deferred-items.md`. |
| 4 | `rtk proxy cargo test --test spawn_seam_guard` | **24 passed**, 0 failed (was 22 in `21-VERIFICATION.md` — exactly +2) |
| 5 | `--test driver_kill --test driver_kill_startup --test driver_escalation_cap --test driver_optin` | 3 / 1 / 8 / 3 passed, 0 failed |
| 6 | Manual read-back of the killed run's `escalations_used` | recorded verbatim above |

Additional gates from the tasks' `<acceptance_criteria>`:

| Criterion | Result |
|---|---|
| `grep -c 'fn finish_run' src/driver/run.rs` | **1** |
| `rtk proxy cargo test --lib driver::run` | 30 passed, 0 failed |
| `rtk proxy cargo test --lib ui::screens::driver_confirm` | 18 passed, 0 failed |
| Source assertion: `registry::` calls inside `do_toggle_opt_in` | `is_opted_in`, `clear_opt_in`, `record_opt_in` (apply path only), `clear_opt_in` (grant revert) — the withdrawal revert is a field assignment |
| Behaviour assertion: `escalations_used` is `Some` after terminate, startup-kill and spawn-failure | all three pass |
| Behaviour assertion: the restored record equals the captured prior record | passes; failed against the unfixed build for the right reason |
| `git diff --diff-filter=D --name-only 69bb159 HEAD` | **no deletions** |

## Threat Mitigations Applied

Three of the plan's four `<threat_model>` rows carried `mitigate`; the fourth is an explicit `accept`.

| Threat ID | Category | Status |
|---|---|---|
| T-21-10-01 | Repudiation (a killed run's `escalations_used: null`) | mitigated — one stamped helper makes the absence unrepresentable rather than remembered; guard proves the site stays single |
| T-21-10-02 | Elevation of privilege (a revert re-snapshotting the disclosed files) | mitigated — the withdrawal revert assigns a clone of the granted record; the drift test shows the old digest survives and the post-`git pull` one is not adopted |
| T-21-10-03 | Tampering (the guard itself) | **accepted, as planned** — both over-approximations fail loud and are stated in the guard's own doc rather than implied away |
| T-21-10-04 | Repudiation (the message on a failed revert) | mitigated — `a_failed_save_names_the_save_failure_in_both_directions` asserts the message names the save, that the note is absent when the entry was found, and that no success message is set |

No dependency was added and no `cargo add` was run, so no `T-21-10-SC` row is fabricated.

## Known Stubs

None. No placeholder, hardcoded empty value, `TODO`, `FIXME`, `#[ignore]` or skipped test was introduced. The one temporary probe used to capture the `run.json` bytes — an `eprintln!` inside the `finished_record` test helper — was added, read, and removed **before** the GREEN commit; `git diff --stat` and the post-removal test run confirmed it.

## Threat Flags

None. No new network endpoint, auth path, file access pattern or schema change at a trust boundary. `escalations_used` is now present on records where it was previously absent, which is a widening of what is *recorded* rather than of what is *exposed*, and the field already existed with `#[serde(default)]` so no reader needs a migration.

## Next Phase Readiness

- **`21-VERIFICATION.md`'s two Warning-level anti-patterns are closed.** WR-02 and WR-04 were recorded in that document's anti-patterns table but deliberately not elevated to blocking gaps; both were confirmed live and both are now fixed with a regression test each.
- **Every finding in `21-REVIEW.md` is now either fixed by plans 21-07…21-10 or named in this SUMMARY's `## Deferrals` table with a reason.** That table is carried verbatim from the plan so it survives into the committed record.
- **DRIVE-04 and SAFE-07 are advanced, not asserted Complete.** `REQUIREMENTS.md` was deliberately not touched, following 21-09's precedent and the premature-Complete revert at `828d7cc`. Re-verification is the right gate for that call.
- **A future caller should know:** `shutdown_on_terminate` and `shutdown_during_startup` now take an `escalations_used: u32`, and **any new terminal path in `src/driver/run.rs` must call `finish_run`** — a bare `journal.finish(...)` in that file's production region fails `tests/spawn_seam_guard.rs` with the message quoted above, by design.
- **And:** a revert in `src/ui/screens/driver_confirm.rs` restores a captured clone. Do not "simplify" it back into a `registry::record_opt_in` call; that function is a constructor, and calling it on a revert re-approves whatever the disclosed files contain at that instant.
- **One outstanding non-blocker:** two integration-test binaries flake under full test parallelism. Documented with evidence in `deferred-items.md`; not caused by this phase.

## Self-Check: PASSED

- `src/driver/run.rs` — FOUND; `grep -c 'fn finish_run'` = 1; contains `started_run_with_cap` and the three escalation-stamp tests
- `src/ui/screens/driver_confirm.rs` — FOUND; `do_toggle_opt_in` captures `previous` and assigns it; four new tests present
- `tests/spawn_seam_guard.rs` — FOUND; 24 tests; contains `TERMINAL_WRITE_HELPER` and `test_region_start`
- `.planning/phases/21-.../deferred-items.md` — FOUND
- Commit `c74a7f0` — FOUND
- Commit `78ff0e1` — FOUND
- Commit `d208efe` — FOUND
- Commit `fbc6a46` — FOUND
- Branch `worktree-agent-ac889dcb4982b9776`, base `69bb159`; working tree clean apart from this SUMMARY and `deferred-items.md` before this commit
- `git diff --diff-filter=D --name-only 69bb159 HEAD` — no deletions

---
*Phase: 21-llm-goal-layer-prompt-injection-hardening*
*Completed: 2026-08-20*
