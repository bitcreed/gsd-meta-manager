---
phase: 21-llm-goal-layer-prompt-injection-hardening
plan: 11
subsystem: driver
status: complete
tags: [gap-closure, invocation-validation, preview-symmetry, regression-guard]

requires:
  - "src/driver/mod.rs::command_source (21-07's CommandSource promotion)"
  - "src/journal/mod.rs::parse_approval_token (21-09's two-half token)"
provides:
  - "command_source refuses a blank or whitespace-only --command with DriveError::NoCommandSource"
  - "drive parses --approved-plan in the pure-refusal group, above the dry-run branch and above the decomposition seam"
  - "approve_plan takes already-parsed halves and reads nothing off argv"
  - "a degenerate-payload x argv-position matrix driven through the production resolver"
  - "a no-wildcard CommandSource classifier that makes a fourth variant a compile error inside the test"
affects:
  - "21-12 (its single-construction-site guard is what makes this plan's seam-only reduction sound)"

tech-stack:
  added: []
  patterns:
    - "Invocation-shape refusals are pure and sit above the dry-run branch, so a preview refuses exactly what the real run refuses (WR-09)"
    - "Enumeration guards key on payloads through the production resolver, never on a curated array of hand-constructed variants"
    - "Total classification via a no-wildcard match, so a new enum variant is a compile error in the test that guards it"
    - "Spawn counts are read off a stand-in's on-disk ledger, never from an in-process counter"

key-files:
  created: []
  modified:
    - src/driver/mod.rs
    - src/error.rs
    - tests/driver_dry_run.rs
    - tests/driver_goal_seam.rs

key-decisions:
  - "The blank --command refusal reuses DriveError::NoCommandSource rather than adding a fourth variant: a command made of nothing IS a run with nothing to do, which is exactly what that variant already names"
  - "The ambiguity arm keeps matching first, so a blank --command beside --target-phase stays AmbiguousCommandSource — blankness must not demote an ambiguous invocation into a legal one"
  - "A blank --command is refused rather than falling through to --goal, so a goal the documented precedence says loses to a supplied command is never silently promoted"
  - "The approval parse refuses malformation only, never absence: PlanApprovalRequired still lives in approve_plan where a plan exists to name and a token exists to print"
  - "recorded_approval is annotated #[cfg_attr(not(unix), allow(unused_variables))] rather than #[cfg(unix)]-gated, so the refusal is answered identically on every platform"
  - "Degenerate payloads are enumerated at the seam rather than defended in the renderer: two places answering one question are two places that can disagree"

requirements-completed: [DRIVE-01, DRIVE-03]

coverage:
  - deliverable: "A blank or whitespace-only --command is refused at command_source, with the boundary pinned on both sides"
    human_judgment: false
    verification:
      - kind: test
        ref: "src/driver/mod.rs#a_run_with_no_command_source_at_all_is_refused_before_anything_is_created"
        status: pass
      - kind: test
        ref: "src/driver/mod.rs#a_run_naming_both_command_sources_is_refused_rather_than_resolved"
        status: pass
      - kind: test
        ref: "tests/driver_dry_run.rs#a_blank_command_is_refused_in_preview_and_in_a_real_run"
        status: pass
  - deliverable: "A malformed --approved-plan is refused for zero seam spawns and identically in preview and real mode; an absent one is not refused there"
    human_judgment: false
    verification:
      - kind: test
        ref: "src/driver/mod.rs#a_malformed_approval_token_is_refused_in_a_preview_exactly_as_a_real_run_would_be"
        status: pass
      - kind: test
        ref: "src/driver/mod.rs#an_absent_approval_is_not_refused_above_the_dry_run_branch"
        status: pass
      - kind: test
        ref: "tests/driver_goal_seam.rs#a_malformed_approval_token_is_refused_before_the_seam_is_spawned_and_identically_in_preview"
        status: pass
      - kind: test
        ref: "tests/driver_goal_seam.rs#a_half_supplied_approval_token_is_refused_by_name_and_never_treated_as_an_approval"
        status: pass
  - deliverable: "The command-source honesty guard enumerates degenerate payloads through the production resolver, and a fourth variant is a compile error inside it"
    human_judgment: false
    verification:
      - kind: test
        ref: "src/driver/mod.rs#every_command_source_refuses_or_previews_cleanly_for_every_degenerate_payload"
        status: pass
      - kind: command
        ref: "temporary fourth CommandSource variant -> cargo build --all-targets fails E0004 at variant_name"
        status: pass
  - deliverable: "No doc in src/driver/mod.rs or src/error.rs still describes the old ordering"
    human_judgment: true
    rationale: "Doc-prose accuracy is a reading judgement. The mechanical half is covered — parse_approval_token appears on exactly 1 executable line and approve_plan's signature carries no `args` — but whether the corrected paragraphs read as candid rather than as after-the-fact tidying is not something a test asserts."

metrics:
  duration: 41 min
  completed: 2026-08-21

actuals:
  tokens: 13298
  tasks: 3
  commits: 5

estimate_delta:
  estimated_tokens: 62000
  actual_tokens: 13298
  note: "Overestimated ~4.7x. The plan's own read_first list was large (nine file regions across four files), but the realized change was three localized edits plus test code; the doc corrections it called for were paragraph rewrites rather than new prose."
---

# Phase 21 Plan 11: Blank-Command Refusal and Free Approval-Token Refusal Summary

A blank `--command` is now refused at `command_source` with `NoCommandSource`, the `--approved-plan` token is parsed in `drive`'s pure-refusal group above both the dry-run branch and the decomposition seam, and the guard that should have caught both enumerates degenerate payloads through the production resolver instead of a hand-picked array.

- **Duration:** 41 min
- **Tasks:** 3 of 3
- **Files modified:** 4
- **Commits:** 5 (TDD: 3 `test` gates, 2 `feat` gates)

## Accomplishments

### Task 1 — the degenerate-payload matrix and the blank-`--command` refusal (tracer)

`command_source`'s `(Some(command), None)` arm gained the match guard `if !command.trim().is_empty()`, in the same register the `Goal` arm has used since the goal arm existed, with a following `(Some(_), None)` arm returning `DriveError::NoCommandSource`. The `(Some(_), Some(_))` ambiguity arm was left untouched and still matches first.

The guard that failed to catch this was replaced rather than extended:

- **`variant_name`** — a `match` over `CommandSource` with three arms and **no wildcard**, living in the test module. A fourth variant is a compile error *in the test file*.
- **`every_command_source_refuses_or_previews_cleanly_for_every_degenerate_payload`** — a `{"", "   ", "\t", "\n  \n", realistic}` × `{--command, --target-phase, --goal}` matrix run through `command_source`, asserting each cell is either `Err(NoCommandSource)` or an `Ok` whose preview carries `SECTION_COMMANDS` and renders no empty numbered entry.
- **`every_command_source_renders_a_preview_with_no_empty_numbered_command`** — the 21-07 sweep, reshaped from index-keyed (`expected == 0/1/2`, a per-position check wearing a per-variant check's name) to a per-variant count built from `variant_name`.
- **`empty_numbered_entry`** — the detector, factored into one helper both tests share.

### Task 2 — the approval token moves into the pure-refusal group

A `recorded_approval: Option<(String, String)>` binding was added as the **fourth and last** invocation-shape refusal, immediately after `escalate::resolve` and immediately before `if args.dry_run`. `approve_plan`'s signature changed from `(project, args, plan)` to `(project, recorded: Option<&(String, String)>, plan)` and its duplicate parse was deleted; it now reads nothing off argv at all.

Three docs that described the old ordering were corrected: `drive`'s step 3 and the invocation-shape block comment now name **four** pure refusals; `approve_plan`'s four-outcome list now says a non-token is refused by the caller; and `DriveError::PlanApprovalMalformed` names where the parse happens and what the position buys.

### Task 3 — the verifier's two hand reproductions became tests

| Binary | HEAD | Now |
|---|---|---|
| `tests/driver_dry_run.rs` | 12 | **13** |
| `tests/driver_goal_seam.rs` | 19 | **20** |
| `cargo test --lib driver::tests::` | 59 | **64** |

## Red-Arm Evidence

Every new assertion is on record as having failed against the tree it was written for. This section is the plan's `<output>` contract.

### Task 1 — the matrix against the unfixed `command_source`

```
---- driver::tests::every_command_source_refuses_or_previews_cleanly_for_every_degenerate_payload stdout ----

thread '...' panicked at src/driver/mod.rs:1733:25:
a numbered entry with nothing after the number reads as a command the run would issue, beneath a header
that promises the COMPLETE and honest sequence — it invites a user to authorise a run on a claim the tool
never checked (review-CR-02). --command with payload "" produced Some("    1. ") in:
...
== GSD commands this run would issue ==
...For a supplied command, the line below is
the complete and honest sequence....
  1 command in the sequence:
    1. 

---- driver::tests::a_run_with_no_command_source_at_all_is_refused_before_anything_is_created stdout ----

thread '...' panicked at src/driver/mod.rs:1167:9:
an empty --command is not a command source

test result: FAILED. 59 passed; 2 failed; 0 ignored; 0 measured; 958 filtered out
```

The rendered `1 command in the sequence:` total and the `1. ` entry with nothing after the number are review-CR-02 verbatim, printed beneath the header that promises the complete and honest sequence.

### Task 2 — the preview half against the unfixed ordering

```
thread 'driver::tests::a_malformed_approval_token_is_refused_in_a_preview_exactly_as_a_real_run_would_be'
panicked at src/driver/mod.rs:1509:46:
a value that is not a token cannot approve anything, and a preview must answer that identically to the
run it previews: ()

test result: FAILED. 63 passed; 1 failed; 0 ignored; 0 measured; 958 filtered out
```

The trailing `: ()` is the whole finding — `expect_err` received `Ok(())`. The preview exited clean, having printed the full report, on an invocation the real run refused.

### Task 3 — both integration binaries against the tree with Tasks 1 and 2 reverted

```
$ cargo test --test driver_dry_run
thread 'a_blank_command_is_refused_in_preview_and_in_a_real_run' panicked at tests/driver_dry_run.rs:529:60:
a command made of nothing is a run with nothing to instruct it, on both paths: ()
thread 'a_preview_refuses_exactly_what_the_real_run_would_refuse' panicked at tests/driver_dry_run.rs:486:10:

failures:
    a_blank_command_is_refused_in_preview_and_in_a_real_run
    a_preview_refuses_exactly_what_the_real_run_would_refuse
test result: FAILED. 11 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out
```

```
$ cargo test --test driver_goal_seam
thread 'a_malformed_approval_token_is_refused_before_the_seam_is_spawned_and_identically_in_preview'
panicked at tests/driver_goal_seam.rs:839:9:
assertion `left == right` failed: the refusal is a pure string check and must cost NO process spawn and
NO model consultation out of the run's budget — this count was 1 against the build that shipped (T-21-11-03)
  left: 1
 right: 0

thread 'a_half_supplied_approval_token_is_refused_by_name_and_never_treated_as_an_approval'
panicked at tests/driver_goal_seam.rs:787:5:
assertion `left == right` failed: a value that is not a token must be refused by a PURE string check,
above the seam: zero spawns, read off the stand-in's own on-disk ledger
  left: 1
 right: 0

failures:
    a_half_supplied_approval_token_is_refused_by_name_and_never_treated_as_an_approval
    a_malformed_approval_token_is_refused_before_the_seam_is_spawned_and_identically_in_preview
test result: FAILED. 18 passed; 2 failed; 0 ignored; 0 measured; 0 filtered out
```

`left: 1, right: 0` is `21-VERIFICATION.md`'s finding reproduced mechanically: the malformed token was refused **after exactly one seam spawn was recorded on disk**. The revert was performed by moving the `recorded_approval` binding back below the dry-run branch and below `decompose`, and by deleting the `command_source` match guard; both were restored afterwards and `git diff --stat` against the committed state was empty before Task 3 was committed.

## Seam-Spawn Counts Observed

| Arm | Invocation | `seam_spawns(workdir)` |
|---|---|---|
| real | `--goal` + garbage `--approved-plan`, `dry_run: false` | **0** |
| preview | the same, `dry_run: true` | **0** |
| control | the same with a **well-formed** token | **1** |
| half-token (existing test, extended) | `--goal` + approval-half only | **0** |

The control arm is what makes the zeroes load-bearing: a payload is planted in every arm, so the stand-in would answer if it were reached, and it demonstrably does answer when the token parses.

## Compile-Forcing Check

A fourth variant `ProbeFourthVariant(String)` was temporarily added to `CommandSource` and `cargo build --all-targets` was run. It failed at **two** sites — the production `preview_text` and, as required, the test module's own `variant_name`:

```
error[E0004]: non-exhaustive patterns: `&driver::CommandSource::ProbeFourthVariant(_)` not covered
    --> src/driver/mod.rs:1535:15
     |
1535 |         match source {
     |               ^^^^^^ pattern `&driver::CommandSource::ProbeFourthVariant(_)` not covered
     |
note: `driver::CommandSource` defined here
    --> src/driver/mod.rs:314:17
     |
 314 | pub(crate) enum CommandSource {
     |                 ^^^^^^^^^^^^^
...
 326 |     ProbeFourthVariant(String),
     |     ------------------ not covered
     = note: the matched value is of type `&driver::CommandSource`
help: ensure that all possible cases are being handled by adding a match arm with a wildcard pattern
      or an explicit pattern as shown
```

Line 1535 is `fn variant_name`. The probe variant was reverted immediately and the suite re-run green.

## Verification

| # | Check | Result |
|---|---|---|
| 1 | `cargo build --all-targets` | clean, exit 0 |
| 2 | `cargo clippy -- -D warnings` (lib gate) | clean, exit 0 |
| 3 | `cargo test --workspace -- --test-threads=4` | **1315 passed, 0 failed**, 35 binaries |
| 4 | `cargo test --test driver_dry_run` | **13** passed (plan required ≥ 13) |
| 5 | `cargo test --test driver_goal_seam` | **20** passed (plan required ≥ 20) |
| 6 | `cargo test --lib driver::tests::` | **64** passed, 0 failed |
| 7 | compile-forcing probe | E0004 at `variant_name`, recorded above |
| 8 | red-arm evidence | recorded verbatim above for all three tasks |
| 9 | `cargo test --test spawn_seam_guard` | 24 passed — the opt-in hatch gained no call site under `src/` |

**Source assertions (Task 2's acceptance criteria):**

- `grep -v '^[[:space:]]*//' src/driver/mod.rs | grep -c 'parse_approval_token('` → **1**, at line 644, inside `drive`, above `if args.dry_run` at line 649.
- `grep -n -A 5 'fn approve_plan' src/driver/mod.rs` → signature is `(project, recorded: Option<&(String, String)>, plan)`, **no `args` parameter**.

**On the documented flakes:** neither `envelope_tracer::a_relocated_copy_of_the_stub_refuses_instead_of_acting` nor either `driver_reattach` test failed in this run — the whole-workspace pass was clean at `--test-threads=4`. No re-run-alone confirmation was needed.

## Deviations from Plan

### Placement of three boundary assertions (no behavioural difference)

The plan's Task 1 `<behavior>` requires assertions for the one-character boundary, the blank-command-beside-`--target-phase` ambiguity case, and the blank-command-beside-`--goal` precedence case, without specifying which test holds them. They were added to the two **existing** tests the plan's own `<read_first>` points at — `a_run_with_no_command_source_at_all_is_refused_before_anything_is_created` (which already pinned the blank-`--goal` sibling) and `a_run_naming_both_command_sources_is_refused_rather_than_resolved` — rather than to the new matrix test, so each assertion sits beside the assertion it is the sibling of. The `artifacts_this_phase_produces` table does not list these two tests as extended; every assertion the plan named nonetheless exists and passes, and the first of them is in the recorded red arm.

### Task 2's red arm proved in-module rather than in an integration binary

Task 2's `<files>` are `src/driver/mod.rs` and `src/error.rs` only, but it carries `tdd="true"` and requires red-arm evidence. Three `#[tokio::test]`s were added to the driver's own test module — within Task 2's declared files — to demonstrate the preview half, the absence control and the target-phase ordering. The real-run half (zero seam spawns) needs the seam fixture and was proved in Task 3, where the plan places it.

**Total deviations:** 2, both organisational. **Impact:** none on behaviour, coverage or the plan's success criteria.

## Threat Flags

None. The `<threat_model>`'s seven rows are all `mitigate` and all are closed by the tests listed under Coverage; the changes introduce no new network endpoint, auth path, file access pattern or schema movement. No `Cargo.toml` change and no dependency movement, so no package-legitimacy question arose.

## Known Stubs

None. No `TODO`, `FIXME`, `todo!`, `unimplemented!`, `#[ignore]` or skipped test was introduced — verified by grepping the added lines of `git diff fad82ec..HEAD`. Every `<verify>` block in the plan was run.

## Issues Encountered

**`rtk` filtered a pipeline the acceptance criteria depend on.** `grep -v '^\s*//' src/driver/mod.rs | grep -c 'parse_approval_token('` returned **0** through the hook-rewritten path and **1** through `rtk proxy sh -c`. The plan warns about exactly this class, and a `0` here would have been read as "the call was deleted" rather than "the output was filtered". All count-bearing checks in this plan were re-run under `rtk proxy`.

## Next

Ready for **21-12**, which is wave 2 for a reason this plan depends on: its single-construction-site guard — asserting tree-wide that the three `CommandSource` variant spellings appear only in `command_source` (constructing) and `preview_text` (matching) — is what makes this plan's decision to enumerate degenerate payloads *only* at the seam a checked property rather than a grep result. The two plans are one argument.

**Carried forward, named rather than glossed:** `src/driver/run.rs:614` still declares a second, unrelated `enum CommandSource { Fixed, Routed }` in the same crate. Renaming it was out of this round's scope; it constrains 21-12's guard, which must key on variant spellings the two types do not share.

## Self-Check: PASSED

- `src/driver/mod.rs`, `src/error.rs`, `tests/driver_dry_run.rs`, `tests/driver_goal_seam.rs` — all present on disk, all modified in this plan's commits.
- Commits `bb4c812`, `585cb40`, `94492f8`, `e2b1222`, `496bd8b` — all present in `git log`.
- All `<acceptance_criteria>` across the three tasks re-run and passing; all nine plan-level `<verification>` items recorded above.
- Working tree clean at `git status --short` before this SUMMARY was written.
