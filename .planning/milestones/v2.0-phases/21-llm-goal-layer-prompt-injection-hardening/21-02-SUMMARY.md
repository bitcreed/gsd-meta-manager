---
phase: 21-llm-goal-layer-prompt-injection-hardening
plan: 02
subsystem: driver
tags: [escalation-cap, taxonomy, park-reason, drive-04, bounds, structured-output, cli-flags]

requires:
  - phase: 21-llm-goal-layer-prompt-injection-hardening
    plan: "01"
    provides: "driver::goal (the plan type, the refusal taxonomy, the schema), driver::untrusted (MAX_UNTRUSTED_FIELD_CHARS, bounded), the SpawnProfile seam"
  - phase: 20-deterministic-decision-router-and-run-bounds
    provides: "bounds::resolve and RunBounds, the four sibling reason taxonomies, the Parked park machinery, router::SAFE_COMMAND_ALPHABET"
  - phase: 15-transport-foundation-duplex-stream-json-executor
    provides: "the (subtype, is_error, terminal_reason) derivation matrix and its explicit fallback arm"
provides:
  - "driver::escalate — the FIFTH sibling taxonomy, its three escalation_-prefixed reasons, and the module doc naming all four existing siblings and the axis"
  - "escalate::resolve — the cap resolved and refused against the RESOLVED step cap, never DEFAULT_MAX_STEPS"
  - "escalate::EscalationBudget — check-before-consult permission with a saturating counter"
  - "escalate::NamedAction — the QuotaWindow::Unknown distinction for a named-but-refused action"
  - "--max-escalations on argv, refused above the run; DriveError::EscalationRefused"
  - "executor::outcome's named arm for structured-output retry exhaustion, with the tolerant fallback intact"
  - "The fifth row of the JournalEvent::Parked taxonomy table, in the same commit as the fifth taxonomy"
affects: [21-03, 21-04, 21-05, 21-06]

actuals:
  tokens: 40000
  tasks: 2
  commits: 2

tech-stack:
  added: []
  patterns:
    - "Sibling reason taxonomy as a new module rather than a new arm on an existing one, with the axis stated in the module doc"
    - "Cap resolution against the value a sibling resolver RETURNED, never against its compiled-in default"
    - "Check-before-consult permission returning a taxonomy member rather than a bool"
    - "Named wire-pair arm that classifies identically to the fallback, kept honest by a half-match control arm in both directions"

key-files:
  created:
    - src/driver/escalate.rs
  modified:
    - src/journal/mod.rs
    - src/driver/mod.rs
    - src/cli.rs
    - src/error.rs
    - src/executor/outcome.rs
    - src/main.rs
    - src/driver/run.rs

key-decisions:
  - "The escalation cap is a FIFTH sibling taxonomy in its own module, not a fifth BoundsReason arm — rate_limit.rs:26-33 records the identical question asked and answered for the quota park, and the axis is the same kind of axis."
  - "The journal taxonomy table's fifth row landed in the SAME commit as the fifth taxonomy, because the table's own text makes an omission a documented lie rather than an oversight."
  - "resolve() refuses a SUPPLIED cap of zero or at/above the resolved step cap, but REDUCES an unsupplied default to what the step cap leaves room for rather than refusing it. Refusing the default would have made `--max-steps 1` unrunnable with no remedy a caller could act on, breaking a shipped CTRL-06 test. Deviation 1 has the full reasoning."
  - "The permission check increments on the ASK, not on the answer — the safe direction, because the alternative under-reports how often the model was consulted and a number that reads as a safety property while under-reporting is worse than no number."
  - "Permission is a two-arm enum rather than a bool, so a refusal hands the caller the taxonomy member it parks under instead of a bare false each call site must translate."
  - "DriveError::EscalationRefused carries EscalationRefusal rather than restating it, exactly as BoundsRefused carries BoundsRefusal — one list answers 'which caps are refusable'."
  - "The escalation cap is NOT recorded on run.json in this plan. journal::RecordedBounds is untouched; recording it belongs with the loop wiring that spends the budget."

requirements-completed: [DRIVE-04]

coverage:
  - id: E1
    description: "An escalation cap greater than or equal to the run's RESOLVED step cap is refused at the seam, before the run exists, with the refusal naming both numbers"
    requirement: "DRIVE-04"
    verification:
      - kind: unit
        ref: "src/driver/escalate.rs#a_cap_that_cannot_bind_is_refused_against_the_resolved_step_cap"
        status: pass
      - kind: unit
        ref: "src/driver/mod.rs#an_escalation_cap_that_can_never_bind_is_refused_before_the_run_exists"
        status: pass
      - kind: unit
        ref: "src/driver/mod.rs#the_escalation_refusal_names_both_numbers_and_an_action"
        status: pass
    human_judgment: false
  - id: E2
    description: "A supplied cap of zero is refused for the same reason a zero step cap is, and the message names zero"
    requirement: "DRIVE-04"
    verification:
      - kind: unit
        ref: "src/driver/escalate.rs#a_supplied_cap_of_zero_is_refused_and_the_message_names_zero"
        status: pass
    human_judgment: false
  - id: E3
    description: "The counter is checked BEFORE a consultation: a run with cap N performs at most N consultations and the consultation that would be number N+1 does not happen. The counter saturates rather than wrapping."
    requirement: "DRIVE-04"
    verification:
      - kind: unit
        ref: "src/driver/escalate.rs#a_budget_of_one_permits_exactly_one_consultation"
        status: pass
      - kind: unit
        ref: "src/driver/escalate.rs#the_counter_saturates_rather_than_wrapping"
        status: pass
    human_judgment: false
  - id: E4
    description: "The reason vocabulary is closed, greppable and prefixed; a new condition is a compile error rather than a park borrowing another taxonomy's string"
    requirement: "DRIVE-04"
    verification:
      - kind: unit
        ref: "src/driver/escalate.rs#every_arm_is_prefixed_and_the_constants_and_arms_match_both_ways"
        status: pass
      - kind: unit
        ref: "src/driver/escalate.rs#each_arm_renders_its_own_constant"
        status: pass
    human_judgment: false
  - id: E5
    description: "A named-but-refused action is carried whole by the type and rendered bounded and control-character-stripped into a record; 'nothing to name' and 'something unrecognised' stay distinct"
    requirement: "DRIVE-04"
    verification:
      - kind: unit
        ref: "src/driver/escalate.rs#a_named_but_refused_action_is_carried_whole_and_rendered_bounded"
        status: pass
      - kind: unit
        ref: "src/driver/escalate.rs#a_refused_action_cannot_become_two_record_lines"
        status: pass
      - kind: unit
        ref: "src/driver/escalate.rs#nothing_named_and_something_unrecognised_are_different_states"
        status: pass
    human_judgment: false
  - id: E6
    description: "The seam's unusable-output case is a named arm carrying both wire strings verbatim, and naming it did not narrow the tolerant fallback path"
    verification:
      - kind: unit
        ref: "src/executor/outcome.rs#structured_output_retry_exhaustion_is_a_named_failure_carrying_both_strings"
        status: pass
      - kind: unit
        ref: "src/executor/outcome.rs#a_half_matched_retry_exhaustion_pair_still_reaches_the_fallback_arm"
        status: pass
    human_judgment: false
  - id: E7
    description: "Exceeding the cap PARKS with escalation_cap_reached readable from the on-disk journal by a separate process, rather than silently degrading to rules-only"
    verification:
      - kind: unit
        ref: "src/driver/escalate.rs#a_budget_of_one_permits_exactly_one_consultation (the reason is produced; the on-disk park is 21-06's)"
        status: partial
    human_judgment: true
    rationale: "The reason exists, is prefixed, and is handed to the caller by Permission::reason(). Nothing in THIS plan spends the budget, so no run has yet parked under escalation_cap_reached on disk. The end-to-end park is plan 21-06's tests/driver_escalation_cap.rs, exactly as the plan's own threat register (T-21-12) says. A human should confirm that deferral rather than reading E7 as already proven."

duration: 20min
completed: 2026-08-19
status: complete
---

# Phase 21 Plan 02: The Escalation Cap and the Fifth Taxonomy Summary

**The model seam now has a budget that can actually bind: `escalate::resolve` refuses every cap that could never fire — judged against the step cap `bounds::resolve` returned, never against `DEFAULT_MAX_STEPS` — and the journal's park-reason table names five sibling taxonomies in the same commit as the fifth one exists.**

## Performance

- **Duration:** ~20 min
- **Tasks:** 2 of 2
- **Files created:** 1
- **Files modified:** 7 source + 9 test files
- **Net:** +1,044 / −7 lines

## Accomplishments

- **The Phase 20 Critical was reproduced deliberately and caught, twice.** Both the pure boundary test and the above-the-run seam test were observed failing against a resolution that compares the supplied cap with `bounds::DEFAULT_MAX_STEPS`. The unit test accepted a cap of 2 under a step cap of 2; the seam test let the run *start*, creating the run directory it asserts does not exist.
- **The fifth taxonomy and the table row that names it landed in one commit.** `src/journal/mod.rs`'s Parked doc says naming only some sanctioned taxonomies would be "the same quiet lie this record exists to prevent" — so shipping the taxonomy without the row would have been a documented lie rather than an oversight. Word "Four" → "Five", new row, "All four reach a terminal record" → "All five".
- **The counter is checked before, not after, and the direction is tested.** The tie-breaking contract is written in the doc as a sentence the test was written from, and against a check-after-the-fact counter the test fails with a recorded count of 2 under a cap of 1.
- **Two doc lines that this plan falsified were corrected in the commit that falsified them**, per the `dry_run.rs:78-83` precedent. `grep -c 'never interpreted'` returns 0 in both `src/driver/mod.rs` and `src/cli.rs`.
- **The retry-exhaustion pair stopped being an assumption.** Research read `MAX_STRUCTURED_OUTPUT_RETRIES` out of the binary's string table and never saw the pair on a transcript (assumption A4). It is now a named arm — and a half-match control arm in *both* directions proves naming it did not narrow the tolerant fallback.

## Task Commits

1. **Task 1: The fifth sibling taxonomy, the cap, and the table row that names it** — `e26bd7d` (feat)
2. **Task 2: The cap on argv, refused at the seam; and a named arm for retry exhaustion** — `b15d82e` (feat)

## Files Created/Modified

- `src/driver/escalate.rs` (NEW, 687 lines) — the fifth sibling taxonomy, three `escalation_`-prefixed reasons, `NamedAction`, `EscalationRefusal`, `resolve`, `EscalationBudget` + `Permission`, and 10 in-source tests.
- `src/journal/mod.rs` — the `JournalEvent::Parked` reason doc: five taxonomies, five rows, "All five reach a terminal record". A doc-only change; `cargo test journal::` passes 82/82 with no serialisation change.
- `src/driver/mod.rs` — `pub mod escalate`; `DriveArgs::max_escalations` and its doc; the corrected `DriveArgs::goal` doc; `escalate::resolve` on the line adjacent to `bounds::resolve` (whose result is now *bound* rather than discarded, which is the whole point); three new seam tests.
- `src/cli.rs` — `--max-escalations`, the corrected `--goal` comment and help text.
- `src/error.rs` — `DriveError::EscalationRefused`, its `Display`, its arm in the exhaustive `source()`, and the `From` impl.
- `src/executor/outcome.rs` — two wire constants, the named arm, the `describe_failure` message, two tests.
- `src/main.rs`, `src/driver/run.rs`, 9 test files — the new `DriveArgs` field threaded through every construction site.

## Guards, and what each did against the UNFIXED behaviour

Every guard was run red before being trusted. Each defect was injected, observed, and reverted from an explicit per-file backup copy — never through a working-tree reset or `git clean`.

| Guard | Injected defect | Observed |
|---|---|---|
| `a_cap_that_cannot_bind_is_refused_against_the_resolved_step_cap` | compared the supplied cap against `bounds::DEFAULT_MAX_STEPS` | FAILED — `EscalationBudget { cap: 2, used: 0 }` returned where a refusal was required. **The Phase 20 Critical, reproduced exactly.** |
| `an_escalation_cap_that_can_never_bind_is_refused_before_the_run_exists` (seam) | same injection | FAILED — `drive` returned `Ok(())`: the run *started*, under `--max-steps 2 --max-escalations 2` |
| `the_escalation_refusal_names_both_numbers_and_an_action` | same injection | FAILED — no refusal to render |
| `the_default_does_not_survive_a_step_cap_it_could_not_bind_under` | returned `DEFAULT_MAX_ESCALATIONS` without consulting the step cap | FAILED — `left: 3, right: 1`; a budget of 3 under a run that can take 2 steps |
| `a_budget_of_one_permits_exactly_one_consultation` | consulted first, checked after (`record()` then `used > cap`) | FAILED — `left: 2, right: 1`; the cap described a consultation that had already happened |
| `the_counter_saturates_rather_than_wrapping` | `wrapping_add` instead of `saturating_add` | FAILED — `left: 0, right: 4294967295`; an exhausted seam reset to an open one |
| `a_named_but_refused_action_is_carried_whole_and_rendered_bounded` | rendered `value.clone()` instead of `untrusted::bounded` | FAILED — 405 characters reached the record |
| `a_refused_action_cannot_become_two_record_lines` | same injection | FAILED — 3 lines out of one value, with a forged-looking `{"reason":"forged"}` on its own line |
| `every_arm_is_prefixed_and_the_constants_and_arms_match_both_ways` | renamed one constant to `bounds_cap_reached` | FAILED, naming the offending identifier — a park borrowing another taxonomy's prefix is caught |
| `a_half_matched_retry_exhaustion_pair_still_reaches_the_fallback_arm` | — (control arm, both directions in one test) | the named subtype with an unknown reason, and an unknown subtype with the named reason, both still carry their observed strings |
| `a_cap_that_can_bind_passes_this_seam` | — (control arm) | a cap one below the step cap renders a preview rather than refusing, so the refusals above are about the cap and not about the opt-in or `--target-phase` |

## Decisions Made

1. **A fifth sibling taxonomy, not a fifth `BoundsReason` arm.** `rate_limit.rs:26-33` records this identical question being asked and answered for the quota park, and the axis here is the same kind of axis: CTRL-06's four bounds are facts about *whether the run is making progress*, an escalation count is a fact about *how much the model was consulted*. A run can burn its whole escalation budget while making excellent progress, and a stalled run can burn none.

2. **The table row rides the taxonomy's commit.** The Parked doc's own text is what forces this. Splitting them would have produced a build in which the record documents four sanctioned taxonomies while five ride the field — precisely the quiet lie the record exists to prevent.

3. **`Permission` is an enum, not a `bool`.** A refused permission hands the caller `EscalationReason::CapReached` through `Permission::reason()`. A bare `false` would have made every call site invent its own translation to a reason string, which is the "fresh literal minted at a call site" the whole taxonomy convention exists to stop.

4. **The count increments on the ask.** A permitted consultation that then fails to happen still counts. That is the safe direction: the alternative under-reports how often the model was consulted, and the plan's own prohibition says a count that under-reports "reads as a safety property and is not one".

5. **The escalation cap is not on `run.json` yet.** `journal::RecordedBounds` is untouched. Recording a budget that nothing spends would be a number with no producer; it belongs with the loop wiring in a later plan. (A stray insertion into `RecordedBounds` was caught during the mechanical field-threading pass and removed.)

## Deviations from Plan

### 1. [Rule 3 — Blocking] The unsupplied default is REDUCED rather than refused

- **Found during:** Task 1, while working out `resolve`'s shape against the existing test suite.
- **The plan's acceptance criterion:** *"assert `escalate::resolve(None, bounds::resolve(Some(2), None).unwrap().max_steps)` is a refusal, so the default cap does not quietly become legal by being a default."*
- **What that would have cost:** `tests/driver_iteration_loop.rs::the_step_cap_halts_a_routed_run_and_reports_itself_rather_than_the_agents_timeout` drives with `max_steps: Some(1)` and no `--max-escalations`. Under a refusing default, that invocation is rejected before the run exists — and *no legal cap exists at all* for a one-step run, since 0 is refused by the zero rule and 1 is refused by the step-cap rule. The test could not be repaired by passing a flag; it could only be repaired by raising `max_steps`, which destroys the CTRL-06 property it exists to prove ("with max_steps of 1 the first iteration runs and the second is refused"). More broadly, `--max-steps 1` — the tightest bound an operator can ask for — would become unrunnable.
- **Why that is wrong rather than merely inconvenient:** `BoundsRefusal`'s own `Display` doc states the rule this violates — *"a refusal a caller cannot act on is a bug report rather than an error message."* For `max_steps == 1` the refusal can name no remedy. And refusing the tightest available bound is contrary to CTRL-06, whose prohibition is against caps that *disable* a detector, not against caps that bind hard.
- **What was implemented instead:** a **supplied** cap is refused exactly as specified (zero; at or above the resolved step cap; naming both numbers). An **unsupplied** cap takes `DEFAULT_MAX_ESCALATIONS` *put through the same comparison*, reduced to `min(3, max_steps - 1)`. The default is therefore not exempt from the check — which is the property the criterion was protecting — but it is reduced rather than refused. A one-step run gets a budget of zero and reports `can_escalate() == false`: a true statement about that run, not a cap pretending to bind.
- **What guards it:** `the_default_does_not_survive_a_step_cap_it_could_not_bind_under` asserts the budget carries **1, not 3**, under a resolved step cap of 2 — and was observed failing against a resolution that hands back the constant unexamined. It also pins the roomy case (default stays 3) so the reduction is about the step cap rather than an always-reduction.
- **What is NOT weakened:** every `must_haves.truths` entry in the plan speaks about a cap that was supplied, and all of them hold. T-21-10 (the operator passing `--max-escalations 999`) is refused. `tests/driver_iteration_loop.rs` stays green untouched.
- **Recorded rather than quietly done**, because deviating from an explicit acceptance criterion on the phase's most load-bearing plan is exactly the kind of thing that should be visible to a reviewer.

### 2. [Rule 1 — Bug] A stray field inserted into `journal::RecordedBounds`

- **Found during:** Task 2, immediately after the mechanical pass that threaded `max_escalations: None` into every `DriveArgs` literal.
- **Issue:** the pass keyed on the line following `wall_clock_cap_secs:`, which also matches `journal::RecordedBounds`'s construction at `src/driver/run.rs:688` — a struct with no such field.
- **Fix:** removed. Caught by the compiler, but recorded because the alternative fix (adding the field to `RecordedBounds`) would have shipped a run-record field that nothing populates, which is worse than the compile error.

---

**Total deviations:** 1 blocking design deviation (documented above at length) + 1 auto-fixed mechanical slip.
**Impact:** no scope creep, no test weakened, no shipped behaviour removed.

## Issues Encountered

- **`cargo fmt --check` is not clean at baseline** on this tree (pre-existing diffs across `src/app.rs` and elsewhere, likely a rustfmt version difference). No global `cargo fmt` was run — that would have produced a diff far larger than this plan. The one over-long line in the new file was wrapped by hand.
- **`tests/driver_reattach.rs`** — the recorded intermittent. It passed on every run here. Not touched, not investigated, per the plan.
- **Red-check hygiene** — every injected defect was reverted from an explicit per-file backup copy in the scratchpad, never through `git clean`, `git stash` or a blanket working-tree reset, per the worktree prohibition.

## Known Stubs

None. Every symbol this plan created is implemented, exercised, and reachable.

Three honest limits, recorded because they are limits rather than stubs:

1. **Nothing spends the budget yet.** `escalate::resolve` runs at the seam and its refusals are load-bearing today, but the returned `EscalationBudget` is discarded there — exactly as `bounds::resolve`'s value was before this plan. `permit_consultation` has no production caller until the loop wiring lands. The consequence for coverage item **E7**: `escalation_cap_reached` is produced by the type and greppable in the source, but **no run has yet parked under it on disk**. The end-to-end park is plan 21-06's, as the plan's own T-21-12 says. Nothing here claims DRIVE-04's park is already proven.
2. **The escalation cap is not recorded on `run.json`.** Deliberate — see decision 5.
3. **The retry-exhaustion pair is still unverified against a real transcript** (research assumption A4). Naming the arm removes the *classification* assumption — the pair now has a case a reader can find — but no capture in `tests/fixtures/transcripts/` carries it, so the strings themselves remain as research read them out of the binary's string table. Plan 21-01's live arm proved the retry *count* is pinned at 1; it did not exercise exhaustion.

Two inherited limits this plan did **not** touch and does **not** claim: `plan_digest` is still FNV-1a (the `sha256:` upgrade is 21-03's), and `CLAUDE.md` suppression still has no behavioural proof (that is 21-05's corpus).

## Threat Flags

None. No new network endpoint, auth path, file access pattern or schema change at a trust boundary. The one new argv surface (`--max-escalations`) is a `u32` refused at the seam and is itself the mitigation for T-21-10.

## Next Phase Readiness

What the next plans inherit:

- **For 21-04 (the loop):** `escalate::resolve` returns the budget the loop should hold across iterations. Spend it with `permit_consultation()` **before** each model call and park on `Permission::CapReached` through the existing Phase 20 park machinery — `EscalationReason::CapReached.as_str()` is the reason string, and `PARKED_LABEL_PREFIX` is how it reaches the terminal label. Do not add a second string source.
- **For 21-04 / 21-06:** `EscalationReason::ActionRefused` and `OutputUnusable` are declared and tested but have no producer yet. `NamedAction` is the carrier for the first; `executor::outcome`'s new named arm is the signal for the second half of the third.
- **For 21-06:** the on-disk park under `escalation_cap_reached`, read back by a separate process, is the missing half of E7. `tests/driver_escalation_cap.rs` should copy `tests/envelope_wiring.rs::parked_events`' shape, per PATTERNS.md.
- **If a sixth taxonomy ever arrives:** the journal table's text and `escalate.rs`'s module doc both say "five" in prose. Both must move together, in the sixth taxonomy's own commit.

## Self-Check: PASSED

- File claimed created, verified present: `src/driver/escalate.rs`.
- Commits claimed, verified in `git log`: `e26bd7d`, `b15d82e`.
- `cargo build`: clean. `cargo test`: 31 test binaries, all `ok`, 0 failures.
- `cargo clippy -- -D warnings`: clean.
- `cargo clippy --all-targets -- -D warnings`: exactly the 5 known pre-existing lints, unchanged.
- `cargo test --lib driver::escalate`: 10 passed.
- `cargo test journal::`: 82 passed — the taxonomy-table edit altered no serialisation.
- `grep -c 'escalation_' src/journal/mod.rs`: 1. `grep -c 'Five sanctioned taxonomies'`: 1. `grep -c 'All four reach a terminal record'`: 0.
- `grep -c 'never interpreted' src/driver/mod.rs src/cli.rs`: 0 and 0.
- `cargo run -- drive --help`: lists `--max-escalations` with the default and the refusal condition.

---
*Phase: 21-llm-goal-layer-prompt-injection-hardening*
*Completed: 2026-08-19*
