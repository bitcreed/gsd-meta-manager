---
phase: 25-running-agents-live-wave-view
plan: 06
subsystem: agents
tags: [rust, ratatui, git, agents, code-review, estimate]

requires:
  - phase: 25-running-agents-live-wave-view (25-01, 25-03)
    provides: scan_project_with, AgentRow.plan attribution, git_ops::log_subjects, leading_frontmatter_nested_value, AgentView::summary_forms
  - phase: 25-running-agents-live-wave-view (25-04)
    provides: dashboard Status-cell ladder (agent_summary_line, status_column_cells) and render harness
provides:
  - "src/agents/fixers.rs: FixerEstimate, estimate, finding_ids, fixer_phase, FIXER_AGENT_TYPE, REVIEW_READ_CAP"
  - "ProjectAgents.fixer_estimate, computed at scan time only while an active unattributed gsd-code-fixer row exists"
  - "AgentView.fixers and the fixer summary ladder (N fixers · ~F/T fixed … Nfix)"
affects: [phase-25-verification, agents-dashboard, agents-subview]

actuals:
  tokens: 8637
  tasks: 2
  commits: 2
plan_head_before: e5b6e1cfbd22f8fa03e5cd9bf2fc006c1b777adf

tech-stack:
  added: []
  patterns:
    - "Estimate module: capped single-file frontmatter read + log_subjects only, early return before any I/O when nothing qualifies"
    - "Mutation-checked tests when a tracer task already implemented the behaviour a later tdd task names"

key-files:
  created:
    - src/agents/fixers.rs
    - tests/agents_fixers.rs
  modified:
    - src/agents/mod.rs
    - src/agents/waves.rs
    - src/ui/screens/normal.rs

key-decisions:
  - "Fixer phase = description `phase NN`, else the fixer's own first fix(NN…) scope; no active-phase fallback (the blocking scan has no ProjectState) [inferred — per PLAN flagged assumption]"
  - "Denominator is findings.total verbatim (D-C12, Assumption A7), documented on estimate() for audit; critical+warning not substituted"
  - "Singular `1 fixer` also in the no-count form (`1 fixer`, `1fix`) [inferred — PLAN specified singular only for the two counted forms]"
  - "When a phase dir holds several *-REVIEW.md, the lexicographically first is read (deterministic) [inferred]"
  - "findings.total value: a trailing ` # comment` and surrounding quotes are stripped before the u32 parse, mirroring the tokens-reader convention [inferred]"
  - "Finding ids kept verbatim (IN-2 and IN-02 are distinct) — PLAN's examples use verbatim ids [inferred]"

patterns-established:
  - "Fixer mode sits between executor mode and the generic `N agents` form in summary_forms(); every counted form carries `~`"

requirements-completed: [AGENT-07]

coverage:
  - id: D1
    description: "A live code-fixer run shows `~fixed/total fixed` from its REVIEW.md findings.total and fix(NN) subjects on its worktree plus main"
    requirement: AGENT-07
    verification:
      - kind: integration
        ref: "tests/agents_fixers.rs#a_code_fixer_run_shows_an_estimated_fixed_over_total"
        status: pass
    human_judgment: false
  - id: D2
    description: "Finding ids dedupe across worktrees and main; other phases and non-fix types never count; phase compare is pad-insensitive"
    requirement: AGENT-07
    verification:
      - kind: integration
        ref: "tests/agents_fixers.rs#finding_ids_dedupe_across_worktrees_and_main"
        status: pass
      - kind: unit
        ref: "src/agents/fixers.rs#finding_ids_accepts_only_fix_subjects_of_the_phase"
        status: pass
      - kind: unit
        ref: "src/agents/fixers.rs#finding_ids_match_the_phase_pad_insensitively"
        status: pass
    human_judgment: false
  - id: D3
    description: "Counts hidden once NN-REVIEW-FIX.md exists, or when REVIEW.md / findings.total is missing or unparseable"
    requirement: AGENT-07
    verification:
      - kind: integration
        ref: "tests/agents_fixers.rs#a_review_fix_report_hides_the_estimate"
        status: pass
      - kind: integration
        ref: "tests/agents_fixers.rs#a_missing_or_unreadable_review_total_gives_no_count"
        status: pass
    human_judgment: false
  - id: D4
    description: "Only active (Live/Idle/Finished) unattributed gsd-code-fixer rows count; none → no estimate and no read"
    requirement: AGENT-07
    verification:
      - kind: integration
        ref: "tests/agents_fixers.rs#attributed_or_inactive_fixers_are_not_counted"
        status: pass
    human_judgment: false
  - id: D5
    description: "Fixer phase from the description or, failing that, the fixer's own fix(NN) commits"
    requirement: AGENT-07
    verification:
      - kind: unit
        ref: "src/agents/fixers.rs#fixer_phase_from_description_or_own_commits"
        status: pass
      - kind: integration
        ref: "tests/agents_fixers.rs#the_phase_comes_from_the_fixers_own_commits_when_the_description_has_none"
        status: pass
    human_judgment: false
  - id: D6
    description: "Dashboard Status cell reads `3fix ~5/48` at 80 and 120 columns and `3 fixers · ~5/48 fixed` at 200"
    requirement: AGENT-07
    verification:
      - kind: automated_ui
        ref: "src/ui/screens/normal.rs#the_fixer_estimate_is_a_whole_ladder_form_at_80_120_and_200"
        status: pass
    human_judgment: false

duration: 8min
completed: 2026-09-26
status: complete
---

# Phase 25 Plan 06: Code-Fixer Run Estimate Summary

**Parallel `gsd-code-fixer` runs now show an estimated `N fixers · ~F/T fixed` (compact `Nfix ~F/T`) on the dashboard. `T` is the phase REVIEW.md's `findings.total`; `F` counts the distinct CR/WR/IN ids named in `fix(NN…)` subjects across the fixer worktrees and main. The counts are hidden once REVIEW-FIX.md exists or the total can't be read.**

## Performance

- **Duration:** ~8 min
- **Started:** 2026-09-26T03:40:16Z
- **Completed:** 2026-09-26T03:48:00Z
- **Tasks:** 2
- **Files modified:** 5 (2 created, 3 modified)

## Accomplishments
- `src/agents/fixers.rs`: an estimate module that writes nothing. It reads a single REVIEW.md through the existing nested frontmatter reader, with a 256 KiB `Read::take` cap, and gets commit subjects only from `git_ops::log_subjects`. It returns before any I/O when there is no active unattributed fixer.
- `scan_project_with` stores the result on `ProjectAgents.fixer_estimate` after classification, and `derive` copies it to `AgentView.fixers`.
- `summary_forms()` fixer mode sits between executor mode and the generic form. Every counted form carries `~`, and the widest one keeps `fixed`.
- 3 unit tests, 6 real-worktree integration tests and 1 dashboard render test were added. Each Task 2 test was mutation-checked.

## Task Commits

1. **Task 1: fixer estimate end to end (tracer)**: `0f28869` (feat)
2. **Task 2: edge cases, dashboard fixer forms, phase gates**: `d5ae958` (test)

## Files Created/Modified
- `src/agents/fixers.rs`: FixerEstimate, estimate (the Assumption A7 doc lives here), finding_ids, fixer_phase, and their unit tests
- `src/agents/mod.rs`: `pub mod fixers`, `ProjectAgents.fixer_estimate`, estimate call in the scan
- `src/agents/waves.rs`: `AgentView.fixers`, `fixer_forms()`, fixer mode in `summary_forms()`
- `src/ui/screens/normal.rs`: `the_fixer_estimate_is_a_whole_ladder_form_at_80_120_and_200`
- `tests/agents_fixers.rs`: the tracer plus 5 edge-case integration tests

## Decisions Made
See `key-decisions` in the frontmatter. Items marked [inferred] are for later audit. Assumption A7 (whether the denominator should include `info` findings) is documented on `estimate()` and was not changed.

## Deviations from Plan

**1. [TDD - Unexpected GREEN] Task 2's tests passed before any Task 2 implementation**
- **Found during:** Task 2 RED step
- **Cause:** Task 1's tracer action specified the complete `estimate` (steps 1–7) and both fixer form ladders, so every behaviour Task 2 lists was already implemented.
- **Handling:** Per Fail-Fast rule 1 I stopped to check whether the tests were wrong. Mutation evidence says they are not. With the REVIEW-FIX check removed, the `plan.is_none()` filter removed, the phase comparison made pad-sensitive, and the compact form spelled `3 fix ~…`, these failed: `a_review_fix_report_hides_the_estimate`, `attributed_or_inactive_fixers_are_not_counted`, `finding_ids_match_the_phase_pad_insensitively`, `finding_ids_dedupe_across_worktrees_and_main` and `the_fixer_estimate_is_a_whole_ladder_form_at_80_120_and_200`. The originals were restored. No GREEN commit was needed, so Task 2 is a single `test(25-06)` commit.
- **Also in that commit:** rustfmt of the new `fixers.rs`, which re-wrapped two lines of Task 1 code. There is no behaviour change.

**Total deviations:** 1 (TDD sequencing; no code change). **Impact:** none on behaviour.

## TDD Gate Compliance
- The RED gate did not produce a failing run. The target behaviour was already in `0f28869` (the `feat(25-06)` tracer), which comes before the `test(25-06)` commit `d5ae958`. Mutation checks stand in for RED evidence.
- GREEN: `0f28869` (feat). REFACTOR: none.

## Verification
- `cargo test --lib agents::fixers`: 3 passed
- `cargo test --test agents_fixers --no-fail-fast`: 6 passed
- `cargo test --lib ui::screens::normal`: 53 passed, including the fixer ladder test
- `cargo test --test agents_scan --no-fail-fast`: 13 passed, including `no_file_under_src_agents_writes_reads_proc_or_spawns` with `fixers.rs` present
- `cargo test --lib agents`: 57 passed. The 25-03 waves tests are unchanged, including `three_code_fixers_without_plans_use_the_generic_form`.
- `cargo clippy -- -D warnings`: clean. `--all-targets` shows no warnings in any file this plan touched.
- `cargo test --no-fail-fast`: 53 `test result:` lines, 2479 passed, 1 failed, 15 ignored. The single failure is the expected local git-version witness, `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`.

## Issues Encountered
None.

## User Setup Required
None. No external service configuration is required.

## Next Phase Readiness
- This is the last plan of Phase 25; all 6 plans have summaries. The phase is ready for verification.
- For audit: Assumption A7 (whether `findings.total` should be the denominator) and the [inferred] decisions above.

---
*Phase: 25-running-agents-live-wave-view*
*Completed: 2026-09-26*

## Self-Check: PASSED
