---
phase: 25-running-agents-live-wave-view
plan: 03
subsystem: agents
tags: [wave-model, plan-attribution, frontmatter-waves, summary-pairing, finished-unmerged, status-ladder]

requires:
  - phase: 25-01
    provides: "AgentRow, ProjectAgents, AgentLiveness, classify_liveness(summary_in_worktree), scan_project_with, BranchPlan, ledger_plan, git_read_raw"
provides:
  - "src/agents/waves.rs: PlanRef, PlanState, WaveRow, AgentView, attribute, commit_scope_plan, derive, AgentView::{summary_forms, is_active}"
  - "AgentRow.{plan, summary_in_worktree}, filled in scan_project_with before liveness classification"
  - "DiskInference.summarized_plans (sorted, from the Pass 2 pairing)"
  - "disk_status::{plan_index, leading_frontmatter_nested_value} pub(crate)"
  - "git_ops::log_subjects (HEAD or <full hex>..HEAD only) and worktrees::commit_subjects"
affects: [25-04, 25-05, 25-06]

actuals:
  tokens: 15900
  tasks: 3
  commits: 5
plan_head_before: f4720adc520e97a8ccfef31de8ac4a6e1ed04168

tech-stack:
  added: []
  patterns:
    - "Attribution tiers resolve to one validated, pad-insensitive PlanRef; a failing capture falls through to the next tier"
    - "All I/O the wave model needs (commit scopes, worktree SUMMARY names) happens in the scan; derive is pure"
    - "Phase directories are resolved from main's listing only, cached once per phase per scan"

key-files:
  created:
    - src/agents/waves.rs
    - tests/agents_waves.rs
  modified:
    - src/agents/mod.rs
    - src/agents/worktrees.rs
    - src/state_reader/disk_status.rs
    - src/state_reader/git_ops.rs

key-decisions:
  - "The commit-scope git call goes through a worktrees::commit_subjects wrapper rather than git_ops directly from mod.rs, keeping 25-01's rule that every git call in src/agents lives in worktrees.rs [inferred]"
  - "The worktree SUMMARY check excludes -FIX- and -GAPCLOSURE- summaries, matching the main-worktree Pass 2 pairing [inferred]"
  - "Display order puts an attributed row before an unattributed one within a liveness rank (None-last), then path [inferred]"
  - "A phase without wave metadata counts only attributed plus summarized plans, so queued is not plan_total minus the rest; the ladder reads only d/t, so nothing rendered changes [inferred]"
  - "The disk inference for the active phase is found via disk_status_for(padded), then a sorted same_phase key scan, so a map keyed '5' still serves phase 05 deterministically [inferred]"
  - "P13 is dropped first in the ladder (RESEARCH A3), deviating from D-C14's drop-from-the-right [inferred, carried from plan]"
  - "A plan whose only attributed agents read Unknown is queued (D-C10 literal) [inferred, carried from plan]"

patterns-established:
  - "Pure in-source derive tests build ProjectAgents / DiskInference / ProjectState literals; real-worktree proofs live in tests/agents_waves.rs with a scripted adapter"

requirements-completed: [AGENT-02]

coverage:
  - id: D1
    description: "Plans group by their own frontmatter wave: through DiskInference.plan_waves; done comes from SUMMARY pairing in main (summarized_plans); the current wave and w? bucket rules hold"
    requirement: AGENT-02
    verification:
      - kind: unit
        ref: "src/state_reader/disk_status.rs#test_summarized_plans_collected_and_sorted_by_pass_2"
        status: pass
      - kind: unit
        ref: "src/agents/waves.rs#the_unknown_wave_bucket_is_never_current_nor_the_denominator"
        status: pass
      - kind: unit
        ref: "src/agents/waves.rs#a_phase_without_wave_metadata_has_no_wave_segment"
        status: pass
      - kind: integration
        ref: "tests/agents_waves.rs#a_branch_attributed_executor_drives_the_wave_summary_end_to_end"
        status: pass
    human_judgment: false
  - id: D2
    description: "Attribution tiers in priority order (description plan-of-phase, plan(s) N-M, branch, commit scope, ledger), pad-insensitive joins, hostile ids rejected"
    requirement: AGENT-02
    verification:
      - kind: unit
        ref: "src/agents/waves.rs#attribution_tiers_in_priority_order"
        status: pass
      - kind: unit
        ref: "src/agents/waves.rs#a_bare_plan_number_joins_its_phase"
        status: pass
      - kind: unit
        ref: "src/agents/waves.rs#hostile_or_malformed_ids_never_attribute"
        status: pass
      - kind: unit
        ref: "src/agents/waves.rs#commit_scopes_pick_the_first_plan_scope"
        status: pass
      - kind: unit
        ref: "src/agents/waves.rs#plan_ids_join_pad_insensitively"
        status: pass
      - kind: unit
        ref: "src/state_reader/git_ops.rs#log_subjects_refuses_a_range_that_is_not_head_or_a_hex_base"
        status: pass
      - kind: unit
        ref: "src/state_reader/git_ops.rs#log_subjects_reads_newest_first"
        status: pass
      - kind: integration
        ref: "tests/agents_waves.rs#commit_scopes_attribute_an_executor_whose_description_and_branch_do_not"
        status: pass
      - kind: integration
        ref: "tests/agents_waves.rs#a_description_outranks_the_branch"
        status: pass
    human_judgment: false
  - id: D3
    description: "Plan states and the summary ladder for the three observed shapes, finished/stalled/unknown cases, narrowing widths, stable display order"
    requirement: AGENT-02
    verification:
      - kind: unit
        ref: "src/agents/waves.rs#a_single_executor_in_a_single_plan_wave"
        status: pass
      - kind: unit
        ref: "src/agents/waves.rs#three_code_fixers_without_plans_use_the_generic_form"
        status: pass
      - kind: unit
        ref: "src/agents/waves.rs#thirteen_executors_in_wave_two_of_eleven"
        status: pass
      - kind: unit
        ref: "src/agents/waves.rs#finished_agents_count_as_done_plus_unmerged"
        status: pass
      - kind: unit
        ref: "src/agents/waves.rs#stalled_and_unknown_only_states"
        status: pass
      - kind: unit
        ref: "src/agents/waves.rs#every_summary_form_is_narrower_than_the_one_before"
        status: pass
      - kind: unit
        ref: "src/agents/waves.rs#display_order_is_total_and_stable"
        status: pass
    human_judgment: false
  - id: D4
    description: "A SUMMARY committed in the agent's worktree marks its plan finished (unmerged, counted apart from done) before the merge; other plans' summaries and phases missing from main never mark it"
    requirement: AGENT-02
    verification:
      - kind: integration
        ref: "tests/agents_waves.rs#a_summary_committed_in_the_worktree_reads_finished_before_the_merge"
        status: pass
      - kind: integration
        ref: "tests/agents_waves.rs#a_held_lock_with_a_worktree_summary_is_live_but_its_plan_is_finished"
        status: pass
      - kind: integration
        ref: "tests/agents_waves.rs#another_plans_summary_does_not_mark_this_plan"
        status: pass
      - kind: integration
        ref: "tests/agents_waves.rs#a_phase_missing_from_main_skips_the_summary_check"
        status: pass
    human_judgment: false
  - id: D5
    description: "src/agents stays write-, /proc- and spawn-free with waves.rs present"
    verification:
      - kind: integration
        ref: "tests/agents_scan.rs#no_file_under_src_agents_writes_reads_proc_or_spawns"
        status: pass
    human_judgment: false

duration: 11min
completed: 2026-09-26
status: complete
---

# Phase 25 Plan 03: Wave Model Summary

**Each agent row is attributed to a validated `PlanRef` at scan time, via description, branch, commit scope, then ledger. A worktree SUMMARY marks its plan `finished` (unmerged) before the merge. `waves::derive` joins the scan with the phase's frontmatter `wave:` groups and main's paired summaries. The result is an `AgentView` with per-wave counts and the 6-step ladder `P13 · w2/11 · 13 run · 8/35 done` … `13run`.**

## Performance

- **Duration:** 11 min
- **Started:** 2026-09-26T03:01:28Z
- **Completed:** 2026-09-26T03:13:11Z
- **Tasks:** 3 (1 tracer, 2 TDD)
- **Files modified:** 6

## Accomplishments

- **`DiskInference.summarized_plans`:** filled from the existing Pass 2 pairing, with no new I/O. It stores the plan stems (slug included), in the same numeric order as `plan_tokens`; both now use one `plan_id_order` helper. `plan_index` and `leading_frontmatter_nested_value` are now `pub(crate)`, with their docs unchanged.
- **`src/agents/waves.rs`:**
  - Types and functions: `PlanRef`, `PlanState`, `WaveRow`, `AgentView`, `attribute`, `commit_scope_plan`, `derive`, `summary_forms` and `is_active`, exactly as in `<interfaces>`.
  - Every captured id must pass `^[0-9]+(\.[0-9]+)?-[0-9]+$`, then `plan_index`, before it becomes a join key.
  - `derive` is pure.
  - The ladder follows RESEARCH Pattern 6 (drop `P13` first).
- **Scan wiring:** `scan_project_with` does the following for each row:
  - Tries attribution tiers 1-3 first. They cost no I/O.
  - Runs one bounded `git log` (via `worktrees::commit_subjects` → `git_ops::log_subjects`) only when those tiers fail, then falls back to the ledger.
  - For an attributed row, resolves the phase directory once per phase from main's own listing and reads the entry names in the worktree's copy. `summary_in_worktree` then feeds both `classify_liveness` and the `finished` plan state.
- **The three observed shapes are pinned:**
  - One executor in a single-plan wave gives `P05 · w2/2 · 1 run · 1/2 done`.
  - Three unattributed fixers give `3 agents`.
  - Thirteen executors in wave 2 of 11 give the full six-string ladder.

## Task Commits

1. **Task 1 (tracer): a branch-attributed executor drives the wave summary end to end:** `f69363e` (feat). Tracer gate (interactive, end-of-phase, automated-only verify): `agents_waves`, `agents_scan`, `agents_claude` and `state_reader::disk_status` re-ran green, then the plan expanded.
2. **Task 2: attribution tiers, plan states, the three shapes and the ladder:** `faf4c83` (test, RED) → `e18d01c` (feat, GREEN)
3. **Task 3: finished before merge, the worktree SUMMARY check:** `70dba5d` (test, RED) → `2a6a5d1` (feat, GREEN)

No REFACTOR commits were needed.

## Files Created/Modified

- `src/agents/waves.rs`: the wave model, plus 14 pure tests
- `src/agents/mod.rs`: `pub mod waves`, the two `AgentRow` fields, attribution, the phase-dir cache, `worktree_holds_summary`
- `src/agents/worktrees.rs`: `commit_subjects` wrapper (skips prunable worktrees; at most 50 subjects)
- `src/state_reader/disk_status.rs`: `summarized_plans`, `plan_id_order`, two `pub(crate)` exposures, 1 test
- `src/state_reader/git_ops.rs`: `log_subjects`, 2 tests
- `tests/agents_waves.rs`: 7 integration tests (tracer, 2 attribution, 4 finished-before-merge)

## Test Results

- `cargo test --lib agents::waves`: 14 passed.
- `cargo test --lib state_reader::disk_status`: 101 passed, including `test_summarized_plans_collected_and_sorted_by_pass_2`.
- `cargo test --lib state_reader::git_ops`: 23 passed, including both `log_subjects_*` tests.
- `cargo test --test agents_waves --no-fail-fast`: 7 passed. `agents_scan`: 13 passed, including the zero-write/spawn/proc guard with `waves.rs` present. `agents_claude`: 10 passed.
- `cargo test --no-fail-fast`: 52 `test result:` lines, 2439 passed, 1 failed, 15 ignored.
  - The one failure is the known local git-version witness (`envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`).
  - 2439 = 2415 + 24 new.
- `cargo clippy -- -D warnings`: clean.
- `git grep` for the two `pub(crate)` functions prints two lines.

## TDD Gate Compliance

- Both TDD tasks have `test(25-03)` followed by `feat(25-03)`.
- **Task 2 RED targets:**
  - `log_subjects_reads_newest_first`, run against a stub that returned an empty Vec. It failed with left `[]` against the two subjects.
  - `commit_scopes_attribute_an_executor_whose_description_and_branch_do_not`, which failed with left `None` against `Some("13-03")`.
- **Task 3 RED targets:** `a_summary_committed_in_the_worktree_reads_finished_before_the_merge` and `a_held_lock_with_a_worktree_summary_is_live_but_its_plan_is_finished`. Both failed on the `summary_in_worktree` assertion.
- **Green at RED time:**
  - The 14 waves tests, because the Task 1 tracer implemented `derive`, `attribute` and the ladder, as the plan's `<interfaces>` required.
  - `a_description_outranks_the_branch`.
  - `another_plans_summary_does_not_mark_this_plan` and `a_phase_missing_from_main_skips_the_summary_check`. These are negative controls, and they passed vacuously while the check did not exist.

  The same pattern appeared in 25-01 and 25-02.
- **`gsd-tools check tdd-red-evidence`:** not run. It parses only node TAP output (see 25-01). `workflow.tdd_mode` is not enabled, so the gate is advisory. [inferred: proceeded on cargo's output]

## Decisions Made

See `key-decisions` in the frontmatter. Every entry is marked [inferred] for audit.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Consistency] The worktree SUMMARY check excludes FIX and GAPCLOSURE summaries**
- **Found during:** Task 3
- **Issue:** without the exclusion, `13-02-FIX-SUMMARY.md` would have marked plan 13-02 finished. The main-worktree Pass 2 never pairs such files with a plan (#1988).
- **Fix:** `worktree_holds_summary` applies the same exclusion.
- **Files modified:** src/agents/mod.rs
- **Committed in:** 2a6a5d1

**2. [Convention] The commit-scope git call goes through `worktrees::commit_subjects`**
- The plan named `git_ops::log_subjects` as called from `scan_project_with`. A thin wrapper in `worktrees.rs` keeps 25-01's recorded rule that every git call in `src/agents` lives there, and it skips prunable worktrees. The behavior and cost are identical.
- **Committed in:** e18d01c

**3. [Refactor, no behavior change] `plan_id_order` helper**
- `plan_tokens`' inline sort key was extracted so that `summarized_plans` uses the same key, as the plan requires. The existing `plan_tokens` tests still pass.
- **Committed in:** f69363e

---

**Total deviations:** 1 auto-fixed (Rule 2), 1 convention, 1 behavior-neutral refactor. **Impact:** none on the public contract. All `<interfaces>` names and fields are as written.

## Issues Encountered

None.

## Known Stubs

None. The Task 2 RED stub of `log_subjects` was replaced in the GREEN commit that followed (`e18d01c`).

## Threat Flags

None. T-25-15 is mitigated: ids are validated by `PlanRef::from_id`, and the directory comes from main's own `find_phase_dir` listing; only entry names are read. T-25-16 is mitigated: `finished` is kept separate from `done`, and waves come from frontmatter only. T-25-17 is mitigated: the `log_subjects` range allowlist and the `-n` bound are covered by `log_subjects_refuses_a_range_that_is_not_head_or_a_hex_base`. No new surface was added.

## User Setup Required

None.

## Next Phase Readiness

- 25-04 can call `waves::derive(&scan, &project_state)` in the `AgentsScanned` handler and render `summary_forms()`. 25-05 renders `AgentView.{waves, agents, worktreeless}`.
- For audit:
  - The flagged assumptions carried from the plan: Unknown-only plans read as queued; `P13` is dropped first.
  - The [inferred] key-decisions above.

## Self-Check: PASSED

- `src/agents/waves.rs` and `tests/agents_waves.rs` exist on disk.
- Commits f69363e, faf4c83, e18d01c, 70dba5d and 2a6a5d1 are all present.
- `git rev-list --count f4720ad..HEAD` = 5.

---
*Phase: 25-running-agents-live-wave-view*
*Completed: 2026-09-26*
