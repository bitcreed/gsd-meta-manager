---
phase: 21-llm-goal-layer-prompt-injection-hardening
plan: 09
subsystem: driver
tags: [approval, cli-contract, tautology, refusal-taxonomy, tdd, rust]

# Dependency graph
requires:
  - phase: 21-llm-goal-layer-prompt-injection-hardening (plan 21-03)
    provides: "journal::approval_digest, ApprovedPlan, ApprovalRefusal and recheck_approval — the approval seam whose plan half was unreachable"
  - phase: 21-llm-goal-layer-prompt-injection-hardening (plan 21-07)
    provides: "CommandSource and the resolved goal command source that reaches approve_plan"
  - phase: 21-llm-goal-layer-prompt-injection-hardening (plan 21-08)
    provides: "sha256:-prefixed plan_digest (so both token halves are prefix:hexdigits and `+` occurs in neither), and PlanApprovalRequired's sanitize_render_line rendering"
provides:
  - "journal::APPROVAL_TOKEN_SEPARATOR — the `+` joining an approval token's two halves, documented as a user-visible CLI contract"
  - "journal::render_approval_token(&str, &str) -> String — the sole joiner of the two halves"
  - "journal::parse_approval_token(&str) -> Result<(String, String), ApprovalTokenError>"
  - "journal::ApprovalTokenError { SeparatorAbsent, SeparatorRepeated, HalfEmpty } with a Display per arm and no wildcard"
  - "journal::recheck_approval(Option<(&str, &str)>, &str, &[PromptInput]) — takes the two RECORDED digests, not an ApprovedPlan"
  - "DriveError::PlanApprovalMalformed(ApprovalTokenError)"
  - "DriveError::PlanApprovalRequired { token, steps } — `digest` became the combined token"
  - "ApprovalRefusal::PlanChanged reachable from the production approval path"
affects: [21-verification, 21-secure-phase, driver-approval-flow, cli-approved-plan]

actuals:
  tokens: 7425
  tasks: 3
  commits: 5

tech-stack:
  added: []
  patterns:
    - "A comparison that can only ever compare equal is deleted, not reworded: the recorded side of a re-check must arrive from where the decision was recorded, never be re-derived from the thing under judgment"
    - "One flag carrying two halves rather than two flags: with one value, 'half supplied' is a parse failure instead of a state the code has to classify"
    - "A predicate takes the fields it compares, not the struct they live on — a comparison-only function that demands a five-field record invites a throwaway record with three empty fields"
    - "A refusal enum arm never echoes the offending argv value back to a terminal when the value adds nothing the caller does not already have"

key-files:
  created: []
  modified:
    - src/journal/mod.rs
    - src/driver/mod.rs
    - src/driver/run.rs
    - src/error.rs
    - src/cli.rs
    - tests/driver_goal_seam.rs
    - tests/driver_refusal_record.rs

key-decisions:
  - "One combined token rather than a second --approved-plan-id flag: DriveArgs gains no field, a reviewer copies one value, and a value that does not carry both halves is simply malformed — the parse IS the check"
  - "recheck_approval takes (recorded_plan_digest, recorded_approval_digest) rather than &ApprovedPlan, because the struct it demanded is what produced the throwaway record whose empty fields hid the tautology"
  - "PlanApprovalMalformed is its own DriveError variant, separate from PlanApprovalStale and from ApprovalRefusal::Absent: 'not a token', 'nobody approved this' and 'what was approved changed' send the caller to three different fixes"
  - "ApprovalTokenError arms carry no payload — the offending value is the caller's own argv, and printing it back would put an unbounded third-party string on the terminal of the person reading a refusal (the capability WR-05 closed elsewhere in this phase)"
  - "The spawn-gate re-check's BEHAVIOUR is unchanged and only its doc is corrected: the plan half genuinely cannot move in that window because a goal is decomposed exactly once, so there is no second observation to compare against"
  - "parse_approval_token trims nothing: silently normalising a padded half would mean two spellings of one approval whose digests then disagree about which was recorded"

patterns-established:
  - "Token extracted from the rendered refusal rather than recomputed in the test, so what is proved is 'the value the user is shown is the value the run checks' rather than 'the test agrees with the helper'"
  - "Paired arm proofs: a plan-changed test and a files-changed test over the same seam, so the suite shows the two arms are DISTINGUISHABLE rather than that one arm swallowed both"

requirements-completed: [DRIVE-01, DRIVE-03, SAFE-07]

coverage:
  - id: D1
    description: "ApprovalRefusal::PlanChanged is produced by the production approval path: the plan digest recheck_approval compares against is the one the approval covered, carried in from the CLI token, never the digest the current run just observed"
    requirement: DRIVE-03
    verification:
      - kind: e2e
        ref: "tests/driver_goal_seam.rs#a_re_decomposition_that_changed_the_plan_is_refused_as_a_changed_plan_not_as_changed_files"
        status: pass
      - kind: other
        ref: "RED against the pre-change build produced PlanApprovalStale(DisclosedFilesChanged { .. }) — the defect reproduced verbatim, recorded below"
        status: pass
    human_judgment: false
  - id: D2
    description: "A run whose disclosed files changed is refused with the files-changed message and NOT the plan-changed one, so the two halves are distinguishable rather than collapsed"
    requirement: DRIVE-03
    verification:
      - kind: e2e
        ref: "tests/driver_goal_seam.rs#a_disclosed_file_rewritten_under_an_approval_is_refused_as_changed_files_not_a_changed_plan"
        status: pass
      - kind: other
        ref: "Neutralization probe: swapping the two halves handed to recheck_approval fails this test; probe reverted"
        status: pass
    human_judgment: false
  - id: D3
    description: "A malformed, truncated or half-supplied approval token is refused by name, is not treated as an absent approval or as a partial one, and creates nothing on disk"
    requirement: SAFE-07
    verification:
      - kind: e2e
        ref: "tests/driver_goal_seam.rs#a_half_supplied_approval_token_is_refused_by_name_and_never_treated_as_an_approval"
        status: pass
      - kind: unit
        ref: "src/journal/mod.rs#a_token_that_does_not_carry_both_halves_is_refused_rather_than_read_as_one"
        status: pass
      - kind: other
        ref: "Neutralization probe: a parse that tolerates a missing separator fails both tests; probe reverted"
        status: pass
    human_judgment: false
  - id: D4
    description: "The approval a user is asked for is a single copy-pasteable token carrying both digest halves, rendered by the one function that joins them, and the round trip is pinned"
    requirement: DRIVE-01
    verification:
      - kind: unit
        ref: "src/journal/mod.rs#an_approval_token_round_trips_its_two_halves_in_plan_then_approval_order"
        status: pass
      - kind: unit
        ref: "src/journal/mod.rs#every_approval_token_refusal_names_the_flag_and_says_how_to_obtain_a_token"
        status: pass
      - kind: e2e
        ref: "tests/driver_goal_seam.rs#the_run_record_carries_the_approval_the_cap_and_the_count (the record's two halves round-trip back to the token the user gave)"
        status: pass
    human_judgment: false
  - id: D5
    description: "The spawn-gate re-check's doc states what that gate can and cannot establish: no second decomposition happens in that window, so the plan half cannot have moved there and the check is about the disclosed files"
    requirement: DRIVE-03
    verification: []
    human_judgment: true
    rationale: "Whether a corrected doc is now TRUE and legible to the next reader is a judgment about prose, not a property a test can assert. The corrected text is quoted verbatim below for exactly this read-back."
  - id: D6
    description: "Approval remains an explicit recorded act: no default yes, no timeout-to-yes, and no path where a malformed or half-supplied token starts a run"
    requirement: DRIVE-01
    verification:
      - kind: e2e
        ref: "tests/driver_goal_seam.rs#a_goal_run_with_no_recorded_approval_refuses_and_says_the_approval_is_absent"
        status: pass
      - kind: e2e
        ref: "tests/driver_goal_seam.rs#a_half_supplied_approval_token_is_refused_by_name_and_never_treated_as_an_approval"
        status: pass
    human_judgment: false

duration: 29min
completed: 2026-08-20
status: complete
---

# Phase 21 Plan 09: The Approval Compares the Half It Covered Summary

**`ApprovalRefusal::PlanChanged` stopped being an unreachable message: `--approved-plan` now takes one token carrying both digest halves, `approve_plan` compares the plan half the approval actually covered instead of comparing the freshly observed digest against itself, and a half-supplied token is a named refusal rather than an approval.**

## Performance

- **Duration:** ~29 min
- **Started:** 2026-08-21T02:18Z
- **Completed:** 2026-08-21T02:47Z
- **Tasks:** 3/3
- **Files modified:** 7

## Accomplishments

- **WR-01 is closed at the comparison, not at the message.** `approve_plan` built a throwaway `journal::ApprovedPlan` whose `plan_digest` was the digest of the plan it had *just decomposed*, then asked `recheck_approval` to compare that against the same value. The plan half compared equal by construction, so `PlanChanged` was unreachable from production and every real mismatch fell through to `DisclosedFilesChanged` — whose message opens *"the plan is unchanged but the disclosed files … are not"*. That throwaway record, with its three deliberately-empty fields (IN-01), no longer exists.
- **The lie was on the common path, not an exotic one.** The review flow is: run without a token to obtain one, then re-run with it — and the second run re-decomposes the goal through a non-deterministic model. A different answer is the *expected* case, and the user was told their `CLAUDE.md` had moved and sent looking for a `git pull` that never happened. The RED commit records that exact wrong outcome verbatim.
- **One flag, two halves.** `--approved-plan` takes `<plan_digest>+<approval_digest>`. `DriveArgs` gains no field (eleven literal construction sites untouched), a reviewer copies one value, and "one half supplied, the other not" is a parse failure rather than a state the code must classify. Both halves are `sha256:`-prefixed after plan 21-08, so `+` occurs in neither and the split is unambiguous.
- **A malformed token is now its own refusal.** `DriveError::PlanApprovalMalformed` is raised *before* any other approval work, and it is deliberately neither `ApprovalRefusal::Absent` ("nobody approved this") nor `PlanApprovalStale` ("what was approved changed") — three different statements sending the caller to three different fixes. A legacy single-half value falls into it and fails closed (T-21-09-03).
- **`recheck_approval` takes the two digests it compares, not a five-field struct.** The struct it used to demand is *why* a caller invented a record with three empty fields; a comparison-only predicate that requires one is an invitation to fill the rest with anything.
- **The spawn gate's doc now says what that gate can establish.** Its behaviour is unchanged and correct — the plan half genuinely cannot move in that window because a goal is decomposed exactly once, above the run — but the comment implied both halves were live there, which would let a future reader build a control on a check nobody performs.
- **`tests/driver_goal_seam.rs` rose from 16 to 19 tests**, all green, with the whole suite at 1011 lib tests plus every integration binary and 0 failures.

## Task Commits

Every task was `tdd="true"`, so each behaviour-adding change has a RED before it:

1. **Task 1 (RED): failing tests for the two-half approval token** — `3abb310` (test)
2. **Task 1 (GREEN): the journal owns the token's separator, render and parse** — `a60f18e` (feat)
3. **Task 2 (RED): failing end-to-end proof that a changed plan reports a changed plan** — `1114f67` (test)
4. **Task 2 (GREEN): `approve_plan` compares the half the approval covered** — `af08f20` (fix)
5. **Task 3: the files-changed and malformed-token refusal proofs** — `cdaf6ed` (test)

**Plan metadata:** committed with this SUMMARY (docs).

_TDD gate sequence: `test(...)` → `feat(...)` and `test(...)` → `fix(...)`, in that order, twice. No `refactor(...)` commit — neither GREEN implementation left anything to clean up._

## Files Created/Modified

- `src/journal/mod.rs` — `APPROVAL_TOKEN_SEPARATOR`, `render_approval_token`, `parse_approval_token`, `ApprovalTokenError` and its `Display`; `recheck_approval`'s signature and doc; `approval_digest`'s "stay distinguishable" promise given a mechanism; `ApprovalRefusal`'s doc recording that `DisclosedFilesChanged`'s opening claim is now backed by a comparison; three new in-module tests plus a `halves` helper.
- `src/driver/mod.rs` — `approve_plan` parses the token first, refuses malformation before anything else, and hands the **recorded** plan digest to `recheck_approval`; the throwaway `ApprovedPlan` is gone; the three-outcome doc becomes a four-outcome doc naming what made the third reachable.
- `src/error.rs` — `PlanApprovalRequired { digest → token }`; `PlanApprovalMalformed(ApprovalTokenError)` with a delegating `Display` and an arm in the deliberately wildcard-free `source` match.
- `src/driver/run.rs` — spawn-gate re-check: call adjusted to the new signature, doc corrected.
- `src/cli.rs` — `--approved-plan`'s doc comment and block comment describe the token's two halves and why one flag; the flag is not renamed.
- `tests/driver_goal_seam.rs` — `approval_for` returns the token; `token_from_refusal` helper; three new tests (16 → 19); two existing assertions strengthened to round-trip **both** recorded halves.
- `tests/driver_refusal_record.rs` — the hand-composed approval at the self-goal test now goes through `render_approval_token`.

## The rendered messages, verbatim

The plan's `<output>` requires these recorded as bytes so the honesty claim can be checked rather than described. Captured from a real `drive()` against the seam fixture.

### `PlanApprovalRequired` — the review surface, with its combined token

```
this run states a goal but records no approval for the plan it was decomposed into, and approval is an explicit act rather than something inferred from silence. The plan is:
  1. command=/gsd-plan-phase phase=21 terminal=verification_passed

If that is what you want run, re-run with `--approved-plan sha256:b763c07159a6538ed33c76f8e35740a8df2b355856e802a6eef9a2177626742e+sha256:96edf4143f1e1c87e0a419706ad0599192a90b1d9c4fbb63c5b504f9e35b6821`, which binds the approval to this plan AND to the disclosed files whose bytes reach a prompt. Both are re-checked at spawn
```

The plan half is `sha256:b763c07…`, which is exactly the value 21-08's SUMMARY recorded for the one-step plan `command=/gsd-plan-phase phase=21 terminal=verification_passed` — the token really does carry the plan digest, not a second copy of the approval digest.

### `PlanChanged` — the message that was unreachable

```
the plan this run decomposed (sha256:a2c688082f28f8a499a6c3b72bf6c9a7d77d4c9151df675820278e91ef9f69db) is not the plan that was approved (sha256:b763c07159a6538ed33c76f8e35740a8df2b355856e802a6eef9a2177626742e). A model asked the same question twice may answer differently, and an approval covers one answer. Review the new plan with `--dry-run` and approve it explicitly
```

**Against the pre-change build the same invocation rendered the `DisclosedFilesChanged` message instead**, opening *"the plan is unchanged but the disclosed files whose bytes reach a prompt are not"* — a sentence asserting a fact the code had not established, about the half that had not moved.

### The three `ApprovalTokenError` arms

```
the `--approved-plan` value carries no `+`, so it is not an approval token. A token names BOTH halves — the plan that was approved and the disclosed files that approval covered — joined by `+`. A value carrying one half is not half an approval; re-run this invocation WITHOUT `--approved-plan` to see the plan and the token that authorises it
```

```
the `--approved-plan` value carries more than one `+`, so it is not two halves and is refused rather than read as the first two. Re-run this invocation WITHOUT `--approved-plan` to see the plan and the single token that authorises it
```

```
the `--approved-plan` value has an empty half, which is what a token truncated at a copy or a shell boundary looks like. An approval that cannot be parsed is an absent approval, so nothing here is treated as partially approved; re-run this invocation WITHOUT `--approved-plan` to see the plan and the whole token that authorises it
```

No arm echoes the offending value. It arrives on argv from whoever launched the run, the caller already has it, and printing it back would put an unbounded third-party string on the terminal of the person reading a refusal — the capability WR-05 closed one plan earlier.

## The spawn-gate doc, corrected

**Added at `src/driver/run.rs`, after the existing "one predicate, two positions" paragraph:**

```
// **What this gate can establish, and what it cannot.** It passes the
// record's own `plan_digest` on both sides, so the plan half compares equal
// here by construction — and unlike the check in `driver::drive`, that is
// sound rather than a tautology dressed as a check, because there is nothing
// for it to compare against. A goal is decomposed EXACTLY ONCE, above the
// run; no second model consultation happens between that check and this one,
// so the plan half provably cannot have moved in this window and there is no
// newly-observed value to weigh the approved one against.
//
// What this gate is therefore actually re-checking is the **disclosed
// files**, and that half is genuinely live: between the decomposition and
// the first agent there is a lock acquisition, a journal start and four
// envelope writes, and a `git pull` landing in that window really does
// rewrite the bytes the approval covered. The plan half's re-comparison here
// is a no-op the shared predicate carries along, not a second opinion — and
// saying otherwise would let a future reader build a control on a check
// nobody performs, which is the defect WR-01 named one call site up.
```

The gate's behaviour was deliberately not changed. The tautology *there* is harmless — it is a comparison with nothing to compare against, which is a different thing from a comparison presented as a distinction — and the only defect was the comment.

## The guards can actually fail

A guard that cannot fail is the Phase-20 defect this repository has already paid for, so each new guard was verified against a neutralized build rather than reasoned about. Both probes were applied by hand and reverted; `git diff --stat` after reverting showed only `tests/driver_goal_seam.rs` modified, and the full suite returned green.

| Guard | Probe | Result |
|---|---|---|
| `a_re_decomposition_..._not_as_changed_files` | none needed — it was written as the RED for Task 2 and failed against the unmodified tree with `PlanApprovalStale(DisclosedFilesChanged { .. })` | FAILED as required |
| `a_disclosed_file_rewritten_..._not_a_changed_plan` | swap the two halves handed to `recheck_approval` in `approve_plan` | FAILED |
| `a_half_supplied_approval_token_...` | make `parse_approval_token` return `Ok((raw, raw))` when no separator is present | FAILED |
| `a_token_that_does_not_carry_both_halves_...` | same probe | FAILED |
| `an_approval_token_round_trips_...` | Task 1's RED — the vocabulary did not exist, so the lib test target did not compile | FAILED as required |

**A note on the files-changed proof's RED, for honesty about the gates.** It does *not* fail against the pre-change build, and that is by design rather than a skipped gate: on that build the same scenario also reaches `DisclosedFilesChanged`, by accident rather than by decision. Its job is to fail against a *future* fix that collapses the two arms — which the swap probe above demonstrates it does. The behaviour-adding tests in both tasks did fail first, so both RED gates are genuine.

## Decisions Made

1. **One combined token, not a second flag.** `21-REVIEW.md` WR-01 suggested `--approved-plan-id` beside `--approved-plan`; `21-VERIFICATION.md` recorded the requirement as "accept both digest halves on the CLI (**e.g.** …)". The combined token satisfies it and avoids touching eleven literal `DriveArgs` construction sites for a one-line addition each.
2. **`recheck_approval` takes two `&str`, not `&ApprovedPlan`.** This is the structural half of the fix. The struct requirement is what made a caller build a record it did not have, and a record built to be thrown away is a record whose fields nobody checks.
3. **Parse before anything else.** A value that is not a token cannot approve anything, so nothing is computed on the strength of it. This also makes `PlanApprovalMalformed` unambiguously distinct from `PlanApprovalRequired`.
4. **`ApprovalTokenError` arms carry no payload.** Considered and rejected carrying the offending `raw`: it is the caller's own argv, they already have it, and echoing an unbounded third-party string into a refusal is the exact capability this phase spent plan 21-08 closing.
5. **No trimming in the parse.** A padded half stays padded and fails the digest comparison by name. Normalising it would create two spellings of one approval.
6. **The spawn gate's behaviour is unchanged.** The plan called for the doc correction only, and execution confirmed the reasoning: there is no second observation available at that point to compare the approved plan digest against, so passing `&approved.plan_digest` on both sides is the honest thing to do — provided the comment says so.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Task 2's signature and field changes broke four test call sites Task 3 nominally owns**

- **Found during:** Task 2 (GREEN), at the first `--all-targets` build and the first suite run.
- **Issue:** `recheck_approval`'s new signature broke `tests/driver_goal_seam.rs:1031`; `PlanApprovalRequired`'s `digest → token` broke `tests/driver_goal_seam.rs:1210`; and once `--approved-plan` demanded a two-half token, four passing tests began failing with `PlanApprovalMalformed(SeparatorAbsent)` because `approval_for` still returned the approval digest alone. `tests/driver_refusal_record.rs`'s hand-composed approval had the same problem. Those are Task 3's `<files>`, but a signature change that does not compile — and a suite that does not pass — cannot be committed atomically at Task 2.
- **Fix:** Migrated `approval_for` to return the combined token via `render_approval_token` (Task 3's first `<action>` step, pulled forward), updated the two mechanical call sites, and migrated the `driver_refusal_record.rs` composition. Two assertions that had compared a single recorded field against the whole approval value were *strengthened* rather than relaxed: they now render the record's two halves back through `render_approval_token` and compare that to the token the user supplied — a bare comparison against one field would pass while the other half was empty, which is precisely the shape WR-01's throwaway record had.
- **Files modified:** `tests/driver_goal_seam.rs`, `tests/driver_refusal_record.rs`
- **Verification:** `rtk proxy cargo test --test driver_goal_seam` (17 passed at that point), `--test driver_refusal_record` (9 passed).
- **Committed in:** `af08f20` (Task 2 GREEN)

**2. [Rule 1 - Bug] A new in-module assertion was case-blind to its own message**

- **Found during:** Task 1 (GREEN), immediately after the implementation landed.
- **Issue:** `every_approval_token_refusal_names_the_flag_and_says_how_to_obtain_a_token` asserted `rendered.contains("without")`, while every `ApprovalTokenError` message spells it `WITHOUT` for emphasis — copying `ApprovalRefusal::Absent`'s register, which the plan explicitly asked for. The assertion was wrong, not the message.
- **Fix:** `rendered.to_lowercase().contains("without")`, so a later reword in either case still satisfies it.
- **Files modified:** `src/journal/mod.rs`
- **Verification:** `rtk proxy cargo test --lib journal::` — 93 passed.
- **Committed in:** `a60f18e` (Task 1 GREEN)

---

**Total deviations:** 2 auto-fixed (1 blocking, 1 bug)
**Impact on plan:** No scope creep. The first was forced by a change the plan mandated and pulls forward one step the plan had assigned to the next task; the second was a one-word fix to a test written minutes earlier.

## Issues Encountered

- **`cargo fmt --check` is not clean on this tree, and was not made clean.** Same version drift `21-07-SUMMARY.md` recorded: the installed rustfmt disagrees with committed formatting in files this plan never touches. Running `cargo fmt` would reformat unrelated files, which the scope boundary forbids. Lines added by this plan were hand-matched to the surrounding style. The gate this plan is judged by is `cargo clippy -- -D warnings` on the **lib** target, which is clean.
- **`rtk` filters `warning:` and `test result:` lines**, so every `cargo` invocation here went through `rtk proxy` and every PASS/FAIL below was read from unfiltered output.

## Verification Results

All five items from the plan's `<verification>` block:

| # | Gate | Result |
|---|------|--------|
| 1 | `rtk proxy cargo build --all-targets` | clean, exit 0 |
| 2 | `rtk proxy cargo clippy -- -D warnings` (lib gate) | clean, exit 0 |
| 3 | `rtk proxy cargo test` (whole suite) | **1011 lib tests + every integration binary, 0 failures**, no `test result: FAILED` line anywhere |
| 4 | `rtk proxy cargo test --test driver_goal_seam` | **19 passed**, 0 failed (was 16 after 21-08 — exactly +3) |
| 5 | Manual read-back of the rendered messages | recorded verbatim above |

Additional gates from the tasks' `<acceptance_criteria>`:

| Criterion | Result |
|---|---|
| `grep -n 'APPROVAL_TOKEN_SEPARATOR' src/journal/mod.rs` | 3 matches (≥1 required) |
| `grep -n 'render_approval_token\|parse_approval_token' src/journal/mod.rs` | 14 matches (≥2 required) |
| `grep -n 'parse_approval_token' src/driver/mod.rs` | 1 match, inside `approve_plan` |
| `grep -c 'PlanApprovalMalformed' src/error.rs` | 3 (variant, `Display` arm, `source` arm) |
| `grep -c 'journal::ApprovedPlan {' src/driver/mod.rs` | **1** — the success path's record is the only one the function builds, so IN-01's three empty fields are gone by construction |
| `grep -n 'render_approval_token' tests/driver_goal_seam.rs` | 4 matches, one inside `approval_for` |
| `rtk proxy cargo test --test driver_refusal_record` | 9 passed, 0 failed |
| `git diff --diff-filter=D --name-only 0849448 HEAD` | **no deletions** |

## Threat Mitigations Applied

All five rows of the plan's `<threat_model>` carried `mitigate` and all five are implemented:

| Threat ID | Category | Status |
|---|---|---|
| T-21-09-01 | Repudiation (a tautology reported as a check) | mitigated — the recorded plan digest arrives from the token; proved end to end |
| T-21-09-02 | Tampering (a partially-supplied token read as an approval) | mitigated — three named parse refusals, raised before any other approval work |
| T-21-09-03 | EoP (a legacy single-half token) | mitigated — fails the parse as `SeparatorAbsent`; pinned by the empty-half and no-separator tests |
| T-21-09-04 | Information disclosure (the refusal's rendered plan) | mitigated — 21-08's `sanitize_render_line` rendering is preserved unchanged, and the new `ApprovalTokenError` arms echo nothing at all |
| T-21-09-05 | Repudiation (the spawn-gate doc) | mitigated — the doc now names the half that gate establishes; quoted verbatim above |

No dependency was added and no `cargo add` was run, so no `T-21-09-SC` row is fabricated.

## Known Stubs

None. No placeholder, hardcoded empty value, `TODO`, `FIXME`, `#[ignore]` or skipped test was introduced. Both neutralization probes were applied and reverted, and the reverted state was confirmed by `git diff --stat` before the Task 3 commit.

## Threat Flags

None. No new network endpoint, auth path, file access pattern or schema change at a trust boundary was introduced. `--approved-plan`'s accepted *value shape* changed, which is a user-visible CLI contract change rather than a new surface; it is documented as such on `APPROVAL_TOKEN_SEPARATOR` and it fails closed.

## Next Phase Readiness

- **`21-VERIFICATION.md` gap 2's plan-half half is closed.** The gap was `partial` because "the approval's plan-half re-check message is not honest"; `src/journal/mod.rs`'s claim that the two halves "stay distinguishable to the person reading the refusal" is now true at both call sites, and a test proves it end to end rather than a doc asserting it.
- **`DRIVE-01`, `DRIVE-03` and `SAFE-07` are advanced by this plan and should not be marked Complete on this plan alone.** The prior premature-Complete revert (`828d7cc`) is the reason to be conservative, and `21-10` is still outstanding in this phase. `REQUIREMENTS.md` was deliberately **not** touched here.
- **`--approved-plan`'s value shape changed.** No released build ever printed a token — `src/driver/goal.rs` does not exist at tag `v1.6.0`, verified in 21-08 — so no user holds one. Any dev-tree value from an earlier build is refused by name with a message telling the caller to re-run without the flag.
- **A future caller should know:** `journal::recheck_approval` no longer takes an `ApprovedPlan`. The two digests it compares are its arguments, and the *recorded* one must come from where the approval was recorded — never be re-derived from the thing being judged. That sentence is the whole of WR-01.
- **No blockers.** The whole suite is green and nothing is left half-applied.

## Self-Check: PASSED

- `src/journal/mod.rs` — FOUND, contains `APPROVAL_TOKEN_SEPARATOR`, `render_approval_token`, `parse_approval_token`, `ApprovalTokenError`
- `src/driver/mod.rs` — FOUND, contains `parse_approval_token` inside `approve_plan`; exactly one `journal::ApprovedPlan {`
- `src/driver/run.rs` — FOUND, spawn-gate call and doc updated
- `src/error.rs` — FOUND, contains `PlanApprovalMalformed` three times and `token` in `PlanApprovalRequired`
- `src/cli.rs` — FOUND, `--approved-plan` doc updated
- `tests/driver_goal_seam.rs` — FOUND, 19 tests, contains `render_approval_token` and `token_from_refusal`
- `tests/driver_refusal_record.rs` — FOUND, 9 tests
- Commit `3abb310` — FOUND
- Commit `a60f18e` — FOUND
- Commit `1114f67` — FOUND
- Commit `af08f20` — FOUND
- Commit `cdaf6ed` — FOUND
- Branch `worktree-agent-a3f951129bb0a6397`, base `0849448`; working tree clean before this SUMMARY

---
*Phase: 21-llm-goal-layer-prompt-injection-hardening*
*Completed: 2026-08-20*
