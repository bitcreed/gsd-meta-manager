---
phase: 37-inbox-screen-and-actions
plan: 01
subsystem: owner-items reader, inbox --json
tags: [decide-lane, context-questions, schema-v1]
requires: []
provides:
  - ContextFile.phase_planned, ContextFile.entries, ContextEntry
  - Hidden.planned_questions (inbox --json hidden and summary, schema v1 additive)
affects: [37-02, 37-03, 37-04, 37-05, 37-06, 37-07]
tech-stack:
  added: []
  patterns: [Layer B inputs cached beside Layer A in ParsedContext]
key-files:
  created:
    - tests/fixtures/owner/context-planned/
    - tests/fixtures/owner/context-summary-only/
    - tests/fixtures/owner/context-entries/
  modified:
    - src/state_reader/owner_items/context.rs
    - src/state_reader/owner_items/rules.rs
    - src/state_reader/owner_items/mod.rs
    - src/inbox.rs
    - src/ui/screens/owner_block.rs
    - src/agent_skill.md
    - README.md
    - tests/owner_digest.rs
    - tests/inbox_cli.rs
key-decisions:
  - "D-17 gate reads the CONTEXT scan's directory listing (regular files only), not DiskStatus"
  - "Acknowledged wins over planned; unreadable stays listed (fail-safe)"
  - "Item names keep the list marker, as today"
requirements-completed: [OWN-20, OWN-23]
duration: 18 min
completed: 2026-11-14
status: complete
commits: 2
plan_head_before: c91214793f2610c1f9941243913897fa1e3c6661
plan_head_after: fcf05302
actuals:
  tokens: 21000
  tasks: 2
  commits: 2
---

# Phase 37 Plan 01: Decide-lane noise Summary

Questions of a phase that already has a PLAN or SUMMARY file leave the lanes and are counted in `hidden.planned_questions`; remaining CONTEXT groups count one item per top-level question entry, with an emphasis-tolerant answered matcher.

## Accomplishments

- D-17: `is_plan_artifact` plus the scan's single directory listing set `ContextFile.phase_planned`; `context_groups` counts and drops planned groups (current and archived), so `decide`, `closes_after` and `archived.decide` shrink.
- `planned_questions` in both `hidden` objects and `summary` of `inbox --json` (error entries 0), README fence and table rows, INBOX_PATHS pins. `render_text` and the goldens are untouched.
- D-18: `top_level_entries` (Layer B, cached in `ParsedContext`) folds wrapped, nested and lazy continuation lines; rules, headings and post-blank prose close an entry; frontmatter list wins; no-list sections fall back to `?` lines. `QUESTION_LABEL` accepts an emphasis closer.
- Fixtures `context-planned`, `context-summary-only`, `context-entries` (all `same` against the oracle); `status --skill` sentence updated.

## Divergences from CONTEXT

- D-17 gate source: the directory listing of the CONTEXT scan (regular files named `PLAN.md`, `*-PLAN.md`, `SUMMARY.md`, `*-SUMMARY.md`), not `DiskStatus.plan_count + summary_count` (RESEARCH Pattern 1: SUMMARY-only, archived and non-roadmap phases have no usable DiskStatus). Inferred, per coordinator directive 1.
- D-18 numbers for `context-questions` 02-alpha: 5 entries, 2 open (`#q4`, `#q5`); `decide` 6 to 5; text line `2 open questions in 02-CONTEXT.md`. Exactly as RESEARCH predicted.
- Fixture `context-planned`: `closes_after.decide` is 0, not the surviving group count, because `closes_after` counts only built phases and phase 03 has no plans (its group still counts in `decide`, 2).

## Deviations from Plan

None in behavior. Process notes:
- [Inferred] Each task is one commit (tests and code together) rather than separate RED/GREEN commits; the plan defines the commit granularity per task.
- The directory listing keeps CONTEXT names without a file-type filter (Layer A parity, a symlink inside the phase dir is still read); only the plan-artifact check requires a regular file.
- `.planning/STATE.md` and `.planning/state.json` were already modified before this plan started (UI-SPEC session bookkeeping); they are committed with the state update below.

**Total deviations:** 0 auto-fixed.

## Verification

- `cargo test --locked --no-fail-fast --all-targets`: 4284 passed, 0 failed (both oracle conformance suites ran, no SKIP).
- `cargo clippy --all-targets -- -D warnings`: clean.
- Owner text goldens unchanged since 35a5e74e. Fleet effect (mailbot decide 19 to 0, sentriq 17 to 6) not measured: real projects are read-only and no registry run was made; fixtures cover the shapes.

## Known Stubs

None.

## Self-Check: PASSED

Created fixtures exist; commits bfe855c1 and fcf05302 are ancestors of HEAD.
