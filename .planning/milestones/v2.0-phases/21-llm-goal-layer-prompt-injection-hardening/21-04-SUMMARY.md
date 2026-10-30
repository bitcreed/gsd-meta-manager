---
phase: 21-llm-goal-layer-prompt-injection-hardening
plan: 04
subsystem: driver
tags: [model-seam, goal-decomposition, escalation, approval-binding, sha256, capability-type, run-record, drive-01, drive-03, drive-04, safe-08]

requires:
  - phase: 21-llm-goal-layer-prompt-injection-hardening
    plan: "01"
    provides: "SpawnProfile::ModelSeam, goal::escalation_schema/parse_action/legality/plan_digest, untrusted::untrusted_block + bounded, ResultMessage::structured_output"
  - phase: 21-llm-goal-layer-prompt-injection-hardening
    plan: "02"
    provides: "escalate::resolve, EscalationBudget::permit_consultation, the fifth sibling taxonomy and its three reasons, NamedAction"
  - phase: 21-llm-goal-layer-prompt-injection-hardening
    plan: "03"
    provides: "journal::sha256_digest, config::PromptInput, registry::current_prompt_inputs, the spawn-gate drift check"
  - phase: 20-deterministic-decision-router-and-run-bounds
    provides: "router::decide's four arms, REASON_NO_RULE, RouterAction + verb + command_for, bounds::resolve, the Terminal type and the park machinery"
provides:
  - "driver::run::GoalDecomposition — the capability type whose MOVE makes a second decomposition a compile error"
  - "driver::run::consult_model_seam — the only SpawnProfile::ModelSeam construction in the tree, with exactly two call sites"
  - "The ambiguity seam at router::Decision::NoRule: budget -> spawn -> re-parse -> verb, three park reasons, no retry"
  - "driver::run::ParkTaxonomy — two taxonomies through the existing five-arm Terminal::Parked, no sixth arm"
  - "journal::approval_digest / ApprovedPlan / ApprovalRefusal / recheck_approval — approval bound to the plan AND the disclosed files"
  - "Three Run-scoped RunRecord fields: approved_plan, escalation_cap, escalations_used"
  - "--approved-plan on argv; DriveError::GoalRefused / GoalSeamUnusable / PlanApprovalRequired / PlanApprovalStale"
  - "Guard five (seam count, two entries, both directions) and guard six (no construction inside the loop, no Clone/Copy)"
  - "tests/fixtures/fake-claude-seam.sh — a stand-in serving BOTH spawn profiles, leaving seam evidence on disk"
affects: [21-05, 21-06, 22-container-execution-target, 23-gate-policy]

actuals:
  tokens: 47804
  tasks: 3
  commits: 3

tech-stack:
  added: []
  patterns:
    - "Capability-by-move: a consuming method whose `self` receiver makes a second use a compile error"
    - "Allowlist keyed by `path::enclosing_fn` where two sanctioned sites share one file"
    - "Source-scanned ORDER of operations, pinned where a behavioural test can only see the outcome"
    - "A refusal that is itself the review surface, carrying what it is asking about"

key-files:
  created:
    - tests/driver_goal_seam.rs
    - tests/fixtures/fake-claude-seam.sh
  modified:
    - src/driver/run.rs
    - src/driver/mod.rs
    - src/driver/router.rs
    - src/driver/goal.rs
    - src/journal/mod.rs
    - src/journal/writer.rs
    - src/error.rs
    - src/cli.rs
    - src/main.rs
    - src/envelope/mod.rs
    - src/driver/reconcile.rs
    - tests/spawn_seam_guard.rs

key-decisions:
  - "`--goal` alone became a THIRD command source rather than a fourth execution model: it resolves INTO the routed one above the run, and the plan's terminal step's phase becomes the target. A goal beside --command or --target-phase stays recorded prose and opens no seam — which is also what keeps every Phase 20 test green untouched."
  - "The production constructor takes `DriveArgs` rather than a bare string. That IS the never-self-goal prohibition: a DriveArgs can only be built from this process's own argv, and an artifact the run wrote cannot become one."
  - "ParkTaxonomy is a two-arm carrier on the EXISTING Terminal::Parked arm. Phase 21 added a second producer of the park ending, and a producer is not an ending — so `the absence of a sixth is the requirement` still greps 1."
  - "RouterAction::command_for widened to pub(crate) rather than writing a second `format!(\"{verb} {phase}\")` at the seam's call site. A second composition site is how 'one path from an action to a string' stops being true; goal.rs's stale sentence was rewritten in the same commit."
  - "The refusal is the review surface. --dry-run decomposes nothing (a preview spawns no process, D-23), so the plan is printed by DriveError::PlanApprovalRequired beside the digest that identifies it — a refusal naming only an opaque digest would be consent in form and not in substance."
  - "make_run_record's inputs were grouped into EstablishedRun rather than suppressing the too_many_arguments lint the new fields introduced. Every field there is a run-scoped fact resolved once above the loop, which is the same reason bounds::RunBounds is a value."
  - "The escalation arm is unit-tested over its own helpers rather than driven end to end, because `router::decide` cannot return NoRule for any state this tree's reader produces from disk. Recorded as a finding rather than manufactured with a fixture that violates a reader invariant."

requirements-completed: [DRIVE-01, DRIVE-03, DRIVE-04]

coverage:
  - id: G1
    description: "A user states a goal once in plain language, an approved plan is recorded on the run record, and the run pursues its terminal phase across GSD commands with no further input"
    requirement: "DRIVE-01"
    verification:
      - kind: integration
        ref: "tests/driver_goal_seam.rs#a_stated_goal_becomes_a_recorded_plan_and_the_run_drives_its_terminal_phase"
        status: pass
      - kind: integration
        ref: "tests/driver_goal_seam.rs#the_run_record_carries_the_approval_the_cap_and_the_count"
        status: pass
    human_judgment: false
  - id: G2
    description: "A goal that cannot be reduced to a machine-checkable stopping condition is refused before the run exists, naming the unreducible part, and creates no run directory or journal"
    requirement: "DRIVE-03"
    verification:
      - kind: integration
        ref: "tests/driver_goal_seam.rs#a_goal_that_cannot_be_reduced_refuses_the_run_and_creates_nothing"
        status: pass
    human_judgment: false
  - id: G3
    description: "The decomposition happens exactly once per run, above the loop, and a second one inside the loop does not compile — a type-level property rather than a control-flow habit"
    requirement: "DRIVE-01"
    verification:
      - kind: integration
        ref: "tests/spawn_seam_guard.rs#the_decomposition_is_consumed_by_move_and_carries_no_clone_or_copy"
        status: pass
      - kind: integration
        ref: "tests/spawn_seam_guard.rs#the_decomposition_constructor_has_one_call_site_and_it_is_above_the_loop"
        status: pass
      - kind: integration
        ref: "tests/spawn_seam_guard.rs#the_loop_scope_scanner_reports_a_construction_inside_a_label_and_not_one_above_it"
        status: pass
    human_judgment: false
  - id: G4
    description: "Approval is an explicit recorded act bound to the plan AND the disclosed files, re-checked at spawn; absence is a refusal distinguishable from a mismatch"
    requirement: "DRIVE-01"
    verification:
      - kind: unit
        ref: "src/journal/mod.rs#an_approval_whose_disclosed_files_moved_is_refused_even_though_the_plan_is_identical"
        status: pass
      - kind: unit
        ref: "src/journal/mod.rs#an_absent_approval_reports_itself_absent_rather_than_mismatched"
        status: pass
      - kind: unit
        ref: "src/journal/mod.rs#an_approval_digest_changes_when_the_disclosed_file_set_changes_under_an_identical_plan"
        status: pass
      - kind: integration
        ref: "tests/driver_goal_seam.rs#a_goal_run_with_no_recorded_approval_refuses_and_says_the_approval_is_absent"
        status: pass
      - kind: integration
        ref: "tests/driver_goal_seam.rs#an_approval_bound_to_a_different_plan_refuses_the_run"
        status: pass
    human_judgment: true
    rationale: "The human-facing review flow is a terminal refusal that prints the plan and the digest, and the user re-runs with `--approved-plan`. That is explicit and recorded, and it is not a TUI screen. CONTEXT.md makes the surface discretionary, but a human should confirm that a two-invocation CLI flow is the approval experience they want before 21-06 builds on it."
  - id: G5
    description: "The model is consulted at exactly two seams and no third; an error the rules cannot classify is a park, not a prompt"
    requirement: "DRIVE-04"
    verification:
      - kind: integration
        ref: "tests/spawn_seam_guard.rs#every_model_seam_spawn_site_in_src_is_one_of_exactly_two"
        status: pass
      - kind: integration
        ref: "tests/spawn_seam_guard.rs#the_ambiguity_seam_asks_the_budget_then_spawns_then_reparses_then_builds_a_command"
        status: pass
    human_judgment: false
  - id: G6
    description: "The escalation's input is the router's observed state only; no byte read from a project file reaches a seam"
    requirement: "SAFE-07"
    verification:
      - kind: integration
        ref: "tests/driver_goal_seam.rs#the_decomposition_seam_is_shown_no_bytes_read_from_a_project_file"
        status: pass
      - kind: unit
        ref: "src/driver/run.rs#the_escalation_prompt_carries_the_observed_token_and_nothing_read_from_a_file"
        status: pass
    human_judgment: false
  - id: G7
    description: "A model-named action outside the alphabet parks with the action-refused reason and the named string recorded verbatim, bounded, and never as a command line"
    requirement: "SAFE-08"
    verification:
      - kind: unit
        ref: "src/driver/run.rs#a_seam_naming_an_action_outside_the_alphabet_parks_and_records_it_verbatim"
        status: pass
      - kind: unit
        ref: "src/driver/run.rs#a_refused_action_is_bounded_and_cannot_become_two_record_lines"
        status: pass
    human_judgment: false
  - id: G8
    description: "Exceeding the escalation cap PARKS with escalation_cap_reached rather than degrading to rules-only, read back from the on-disk journal"
    requirement: "DRIVE-04"
    verification:
      - kind: unit
        ref: "src/driver/run.rs#a_halt_labels_itself_from_memory_and_a_journal_it_never_reads (the escalation arm of the terminal-label assertion)"
        status: partial
    human_judgment: true
    rationale: "The cap-reached park is wired at the no-rule arm and its label is asserted through the SAME `parked:` prefix and the fifth taxonomy's own `as_str()`. But `router::decide` cannot return NoRule for any state this tree's reader produces from disk (see Known Stubs), so NO run has yet parked under escalation_cap_reached end to end — 21-02 deferred that to 21-06 and it is still deferred. A human should confirm the deferral rather than reading G8 as proven."

duration: 145min
completed: 2026-08-19
status: complete
---

# Phase 21 Plan 04: The Two Seams, Wired Summary

**A goal a human states in plain language now becomes a validated plan through the
model seam, gets an approval bound to that plan *and* to the bytes that will enter
the prompts, and drives the loop toward the plan's terminal phase — with the
never-self-goal prohibition enforced by the compiler, the seam count held at two
by a both-directions allowlist, and the falsified purity comment rewritten in the
commit that falsified it.**

## Performance

- **Duration:** ~145 min
- **Tasks:** 3 of 3
- **Files created:** 2
- **Files modified:** 12 source + 10 test files
- **Net:** +3,479 / −64 lines
- **Lib tests:** 991 → **1,004** (13 new); `spawn_seam_guard` 14 → **21**

## Task Commits

1. **Task 1: the decomposition capability, moved once, outside the loop** — `5e64673` (feat)
2. **Task 2: the ambiguity seam at the router's one uncovered state** — `79e5eef` (feat)
3. **Task 3: the approval bound to what will actually run, and guard five** — `42725ec` (feat)

## Guards and behavioural tests, and what each did against the UNFIXED behaviour

Every guard was run red before being trusted. Each injected defect was reverted
from an explicit per-file backup copy in the scratchpad and verified byte-identical
with `diff` — never `git clean`, `git stash` or a blanket working-tree reset, per
the worktree prohibition.

| Guard / test | Injected defect | Observed |
|---|---|---|
| `the_decomposition_is_consumed_by_move_and_carries_no_clone_or_copy` | `decompose(self, …)` → `decompose(&self, …)` | **FAILED** — `Found: ["&self,", …]`; a reference receiver leaves the capability alive and a second decomposition inside the loop would compile |
| `the_decomposition_constructor_has_one_call_site_and_it_is_above_the_loop` | a second `GoalDecomposition::from_argv_goal(args)` inside `'iterations` | **FAILED**, naming `src/driver/run.rs:2336` — `left: 2, right: 1` |
| `the_loop_scope_scanner_reports_a_construction_inside_a_label_and_not_one_above_it` | — (control arm, three directions in one test) | reports a synthetic construction *inside* the loop label, stays silent on one *above* it **and** on one *below the closed loop* — the third direction is what a line-number comparison would get wrong |
| `the_plans_target_phase_is_its_last_step_and_never_its_first` | `steps.last()` → `steps.first()` | **FAILED** — `left: Some("21"), right: Some("22")`; and the integration arm failed with `left: String("20")`, the prerequisite the plan merely passes through |
| `a_run_whose_step_cap_leaves_no_room_to_escalate_refuses_the_decomposition` | budget checked *after* the consultation | **FAILED** — seam spawn count `left: 1, right: 0`: the model was spawned and the "cap" was a report of a call already made |
| `the_ambiguity_seam_asks_the_budget_then_spawns_then_reparses_then_builds_a_command` | budget check moved below the spawn | **FAILED**, naming `consult_model_seam` at line 2570 as appearing before a step that must precede it |
| same guard, retry direction | a re-consultation of the seam after an `Unusable` answer | **FAILED** — `left: 2, right: 1`: "the ambiguity seam consults the model EXACTLY once per no-rule state" |
| `every_model_seam_spawn_site_in_src_is_one_of_exactly_two` | a third seam, `recover_from_error`, for error recovery | **FAILED** — `Unexpected: ["src/driver/run.rs::recover_from_error"]` |
| same guard, stale direction | the decomposition stubbed so it no longer spawns | **FAILED** — `Stale: ["src/driver/run.rs::decompose"]`: an allowlist wider than the truth it describes |
| `run_record_fields_all_declare_their_scope` | the scope word removed from `escalations_used`'s doc | **FAILED** — `These declare neither run-scoped nor iteration-scoped: ["escalations_used"]` |
| `an_approval_whose_disclosed_files_moved_is_refused_even_though_the_plan_is_identical` | (written to fail against a plan-only binding, and says so in its message) | passes only because the digest covers both halves; with a plan-only binding the two digests are equal and the run starts against bytes the approval never covered |
| `an_absent_approval_reports_itself_absent_rather_than_mismatched` | — (the distinction is the assertion) | `Absent` is its own arm; a single "mismatch" for both would report an unapproved run as a stale approval |
| `a_seam_naming_an_action_outside_the_alphabet_parks_and_records_it_verbatim` | — (leak arm) | the park detail carries `/gsd-ship 21 --force` verbatim and **none** of the three safe-alphabet verbs, so nothing assembled a pasteable command from a refused action |
| `a_refused_action_is_bounded_and_cannot_become_two_record_lines` | — (hostile 400-char value with an embedded newline and a forged `{"reason":"forged"}`) | no newline survives and the truncation marker is present |
| `the_decomposition_seam_is_shown_no_bytes_read_from_a_project_file` | — (planted canary + arrival assertion) | the goal text is asserted to have **arrived** before the canary's absence is claimed — a corpus test with no arrival proof is not evidence (21-01's pattern) |

## Decisions Made

1. **`--goal` alone is a third command SOURCE, not a fourth execution model.**
   `command_source_refusal` gained a third parameter and the decomposed plan's
   terminal step's phase is written back onto `args.target_phase`, so the run then
   proceeds through exactly the Phase 20 routed path. A goal supplied *beside*
   `--command` or `--target-phase` opens no seam at all, because both of those are
   already machine-checkable — which is also why every Phase 20 and Phase 17 test
   stayed green untouched despite `tests/driver_iteration_loop.rs` driving with a
   goal on every invocation.

2. **The production constructor takes `DriveArgs`.** That is the whole of the
   never-self-goal prohibition and it is mechanical rather than documented: a
   `DriveArgs` is built from this process's own argv, and there is no path from a
   file the run wrote to one. The escape hatch
   (`for_testing_bypassing_the_human_goal`) exists only because integration tests
   are separate crates, and a guard proves it has zero non-comment occurrences
   under `src/` outside its own definition.

3. **`ParkTaxonomy`, not a sixth `Terminal` arm.** Phase 21 added a second
   *producer* of the park ending, and a producer is not an ending. Both taxonomies
   reach disk through the existing `Terminal::Parked` arm and each one's own
   `as_str()`, so `grep escalation_cap_reached` finds the budget and the record it
   produced together. `grep -c 'the absence of a sixth is the requirement'` returns
   **1** and that paragraph is unedited.

4. **`RouterAction::command_for` widened to `pub(crate)`.** The alternative was a
   second `format!("{verb} {phase}")` at the seam's call site, and a second
   composition site is precisely how "there is one path from an action to a string"
   stops being true. Neither input can carry a model byte — the action survived
   `goal::parse_action` and the phase was validated on argv — so this widens *where
   the one builder may be called from*, not *what may be built*. `goal.rs`'s
   sentence claiming it is private was rewritten in the same commit.

5. **The refusal is the review surface.** `--dry-run` decomposes nothing, because a
   preview spawns no process (D-23) — so it cannot be where the plan is shown, and
   an earlier draft of the refusal message that told the user to look there was
   corrected before it shipped. `DriveError::PlanApprovalRequired` now carries the
   plan's typed tokens and prints them beside the digest. A refusal naming only an
   opaque digest would be consent in form and not in substance.

6. **`record_iteration_decision` gained `by`, and the journal has its first `llm`
   record.** The field's three-value vocabulary was declared in Phase 16 with `llm`
   reserved for this phase; the doc claiming `by` was always `"policy"` was
   rewritten in the commit that produced the first one. No fourth value was minted
   and `human` still has no producer.

7. **`EstablishedRun` rather than an `#[allow]`.** The three new record fields
   pushed `make_run_record` to nine parameters and clippy's `too_many_arguments`
   fired — a *sixth* lint over the recorded baseline of five. Grouping the
   run-scoped facts into one value is the same move `bounds::RunBounds` already is,
   and it says something a flat parameter list did not.

## Deviations from Plan

### 1. [Rule 3 — Blocking] The purity comment was rewritten in Task 2's commit, not Task 1's

- **The plan's Task 1 action** says to correct `src/driver/run.rs:1915-1924` and
  lists `grep -c 'no I/O, no model call'` returning 0 among **Task 1's** acceptance
  criteria — while the same action states the rewrite must "ride the commit that
  falsified it, per the `src/driver/dry_run.rs:78-83` precedent".
- **Those two instructions conflict**: nothing in Task 1 falsifies the comment. The
  arm was still pure at the end of Task 1; Task 2 is what added the model call.
- **Resolved in favour of the precedent**, which is the codebase's own written rule
  and the one 21-01 followed when it hit the mirror-image case (it moved a rewrite
  *earlier*, into Task 1, for the same reason). The grep returns **0** at the end of
  Task 2 and at the end of the plan.

### 2. [Planned scope] Four files beyond the plan's `files_modified`

`src/error.rs` (four new `DriveError` variants and their exhaustive `source()`
arms), `src/cli.rs` and `src/main.rs` (`--approved-plan`), `src/driver/router.rs`
and `src/driver/goal.rs` (the `command_for` widening and its paired doc
correction). Each is the minimum surface the plan's own text requires: a refusal
"naming the part that could not be reduced" needs a typed variant, and an approval
that is "an explicit recorded act" needs somewhere for the act to arrive.

### 3. [Planned scope] Eleven fixtures threaded the two new required fields

`DriveArgs` gained `approved_plan` and `RunRecord` gained three fields, so every
struct-literal site broke — **which is the mechanism**, exactly as 21-03 recorded
when `DriverOptIn` did the same. All are `None` on fixtures whose assertions
predate the fields, which is precisely the shape the tolerant read path has to keep
loading; `the_three_new_run_record_fields_survive_a_round_trip_and_default_when_absent`
proves a pre-Phase-21 record still loads.

### 4. [Rule 1 — Bug] The approval refusal pointed at a surface that does not render the plan

- **Found during:** Task 3, while checking `cargo run -- drive --help` against the
  message text.
- **Issue:** the refusal said "Review the plan with `--dry-run`", and `--dry-run`
  sits *above* the decomposition in `drive` by design, so it renders no plan at all.
  A refusal that tells the user to look somewhere the information is not is worse
  than one that says nothing.
- **Fix:** `PlanApprovalRequired` carries the plan's typed tokens and prints them.
  The `cli.rs` comment was corrected in the same commit.
- **Recorded rather than quietly fixed**, because a user-facing instruction that
  describes a mode the code does not have is exactly the class this codebase's
  pinned-honesty tests exist to catch, and it nearly shipped.

---

**Total deviations:** 1 blocking sequencing decision, 1 auto-fixed honesty defect,
2 in-scope mechanical passes.
**Impact:** no scope creep, no test weakened, no shipped behaviour removed.

## Issues Encountered

- **`tests/driver_reattach.rs` — the recorded pre-existing intermittent.** It failed
  twice under a fully parallel `cargo test` and **passed 3/3 with
  `--test-threads=1`**, which is the spawn-race signature 21-03 recorded (0.53s
  against a 30s budget). This plan does not modify that file, and its `drive_args`
  supplies `--command` *and* `--goal`, so `from_argv_goal` returns `None` and no
  seam fires on that path — verified by reading the fixture rather than assumed.
- **`tests/envelope_tracer.rs` flaked once** under the same parallel load, at
  `.expect("the generated stub is executable")` — a spawn failure, not an
  assertion. Green in isolation and on re-run. Same class, different file; this plan
  touches no envelope code.
- **`cargo fmt --check` is not clean at baseline** (pre-existing, recorded by 21-02
  and 21-03). No global `cargo fmt` was run.
- **`rtk` filtering** was bypassed with `rtk proxy` for every measurement that
  depends on raw output, per the phase's own instruction.

## Known Stubs

None. Every symbol this plan created is implemented, reachable and exercised.

**Five honest limits, recorded because they are limits rather than stubs:**

1. **`router::decide` cannot return `NoRule` for any state this tree's reader
   produces from disk, so the ambiguity seam has no end-to-end test.** The rule
   table covers `no_directory`, `empty`, `discussed`, `researched` and `planned`;
   `gate_for` intercepts `partial` and every `executed` verification status; and
   `complete` requires `passed`, which `is_goal_met` answers first. The one
   uncovered state — `complete` with a non-passing verification — **violates the
   reader's own invariant** (`Complete` *means* verification passed, at
   `disk_status.rs:670`) and is reachable only by constructing the struct, which
   `router::tests::a_state_the_table_does_not_cover_parks_as_no_rule_naming_it`
   already does. A grep confirms no existing integration test reaches
   `router_no_rule` either.

   The arm is written, wired, order-pinned by a source scan and unit-tested over its
   own helpers (`escalated_action`, `escalation_prompt`) for all four outcomes. What
   is **not** demonstrated is a run parking under `escalation_cap_reached`,
   `escalation_action_refused` or `escalation_output_unusable` on disk. This is the
   same deferral 21-02 recorded for E7 and it is now **larger than 21-02 assumed**:
   21-06 cannot demonstrate it either without either widening the reader, or adding
   a rule-table row that removes a covered state, or accepting a fixture that
   writes an inference the reader would never produce. **Whoever plans 21-06 should
   decide which of those three, deliberately.**

2. **The approval's human surface is a two-invocation CLI flow**, not a TUI screen.
   Run without `--approved-plan`, read the plan and the digest off the refusal,
   re-run with the digest. That satisfies "explicit and recorded" and CONTEXT.md
   makes the surface discretionary, but it is a flow a human should look at before
   21-06 builds on it (coverage item G4).

3. **The seam spawns carry no envelope.** A seam has an empty tool set, so there is
   nothing for the `PreToolUse` hook to guard and nothing for the credential helper
   to answer, and the decomposition runs *above* the run where no envelope has been
   established. The `CLAUDE*` environment scrub in the spawn closure is
   unconditional and still applies. Stated in `consult_model_seam`'s own doc. **The
   hazard is entirely in the future**: the day somebody gives a seam a read tool,
   this becomes a hole, and the existing
   `no_executable_line_in_src_passes_the_hook_disabling_flag` guard does not cover
   it.

4. **`CLAUDE.md` suppression still has no behavioural proof**, inherited from 21-01
   and 21-03 and untouched here. 21-05's corpus is where it lands.

5. **`goal::plan_digest` is still FNV-1a.** It is now one of two inputs to a
   SHA-256 `approval_digest`, so an approval is no longer backed by a non-security
   hash alone — which is what 21-03's Known Stub 3 asked for. The inner value
   remains drift detection and its doc still says so.

## Threat Flags

None. No new network endpoint and no new auth path. The one new file-access pattern
— reading the five disclosed prompt inputs a second time at spawn to re-check the
approval — is read-only, bounded to `registry::DISCLOSED_PROMPT_INPUTS`, and is
itself the mitigation for T-21-24. The one new argv surface (`--approved-plan`) is a
digest compared for equality and never interpolated anywhere.

The threat register's dispositions, all `mitigate`, are implemented:
T-21-23 (self-goal) by the move plus guard six; T-21-24 (approval replay) by the
two-half digest re-checked at spawn; T-21-25 (inferred consent) by
`PlanApprovalRequired` being its own variant with its own wording; T-21-26 (a third
seam) by guard five; T-21-27 (raw file content at the seam) by the `observed`-only
input and the planted-canary test; T-21-28 (a model-named action interpolated into a
command) by `command_for` staying the one path and the park-detail leak assertion;
T-21-29 (an undeclared record field) by `run_record_fields_all_declare_their_scope`,
observed red; T-21-30 (an unfinishable plan reaching approval) by `goal::legality`'s
resolved-step-cap refusal, which 21-01 already red-checked.

## Next Phase Readiness

- **For 21-05:** the seam-aware stand-in `tests/fixtures/fake-claude-seam.sh` serves
  both spawn profiles from one program, captures every seam's stdin, and counts
  spawns on disk. An injection corpus can plant hostile bytes in `.planning/` and
  assert both that the goal *arrived* and that the selected command is unchanged,
  using `seam_stdin` / `seam_spawns` from `tests/driver_goal_seam.rs`.
- **For 21-06:** read Known Stub 1 first. The `escalation_*` park reasons have a
  producer in the code and no reachable state on disk; the end-to-end park needs a
  deliberate decision about which of three routes to take, not another deferral.
- **If a third seam is ever wanted:** `SEAM_SITES` in `tests/spawn_seam_guard.rs` is
  a two-entry allowlist that fails in both directions, and the phase's own prose
  says an error the rules cannot classify is a park rather than a prompt. Adding an
  entry is possible and is meant to be a visible, argued act.
- **If a sixth prompt-input file is ever read:** it must be added to
  `DISCLOSED_PROMPT_INPUTS` in the same commit as the code that reads it, or both
  the disclosure *and* every approval digest become under-broad while every test
  stays green.

## Self-Check: PASSED

- Files claimed created, verified present: `tests/driver_goal_seam.rs`,
  `tests/fixtures/fake-claude-seam.sh`, `21-04-SUMMARY.md`.
- Commits claimed, verified in `git log`: `5e64673`, `79e5eef`, `42725ec`.
- `cargo build` and `cargo build --all-targets`: clean, no warnings.
- `cargo test`: all suites `ok`, 0 failures (with the recorded `driver_reattach`
  intermittent green on a serial run).
- `cargo test --lib`: **1,004 passed**, 0 failed — 13 new.
- `cargo test --test spawn_seam_guard`: **21 passed**, up from 14.
- `cargo test --test driver_goal_seam`: **10 passed**.
- `cargo test --test driver_model_seam -- --ignored`: 3 passed — both OQ1 live arms
  and the retry pin still hold against the real binary after the schema's consumers
  changed.
- `cargo clippy -- -D warnings`: clean.
- `cargo clippy --all-targets -- -D warnings`: exactly the 5 known pre-existing
  lints, in their recorded locations (`browser.rs:131/132/133`,
  `project_creator.rs:146`, `state_reader/mod.rs:311`).
- `grep -c 'no I/O, no model call' src/driver/run.rs`: **0** — the falsified comment
  was rewritten, not left.
- `grep -c 'the absence of a sixth is the requirement' src/driver/run.rs`: **1** —
  the `Terminal` doc still stands and no sixth arm was added.
- `cargo run -- drive --help`: lists `--approved-plan` with what it binds and when
  it is required.
- Every injected-defect restoration verified byte-identical to its backup via
  `diff`; no `git clean`, `git stash` or blanket reset was used at any point.
- STATE.md, ROADMAP.md and WINDOWS.md: **untouched**, per the orchestrator's
  instruction.
- `actuals.tokens` = 47,804 = 191,218 raw diff chars / 4, measured under
  `rtk proxy` against the plan's base commit. Roughly half the plan's 92,000
  estimate; recorded as measured rather than adjusted toward it.

---
*Phase: 21-llm-goal-layer-prompt-injection-hardening*
*Completed: 2026-08-19*
