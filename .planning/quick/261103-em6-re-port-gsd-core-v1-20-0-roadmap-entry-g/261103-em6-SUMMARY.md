---
phase: quick-261103-em6
plan: 01
subsystem: state_reader
tags: [roadmap, gsd-core-sync, progress-table, phase-entry-grammar]
requires: []
provides:
  - "roadmap_progress(content, state_milestone) with the #5761 owner rule and the #5165 milestone slice"
  - "phase_entry_open: one #5763 entry prefix shared by R1/R2/R3 and the roadmap_milestones phase scope"
affects: [src/state_reader/roadmap_md.rs, src/state_reader/mod.rs]
tech-stack:
  added: []
  patterns: ["recognizers compose their prefix from one helper", "fence-aware heading scan"]
key-files:
  created: []
  modified:
    - src/state_reader/roadmap_md.rs
    - src/state_reader/mod.rs
key-decisions:
  - "Progress table sliced to the current milestone by a port of normalizeMilestoneToken (#5165)"
  - "Progress section located fence- and level-aware, first table with Phase + Plans Complete, no fall-through (#5761)"
  - "Phase entries are line-anchored with a shared prefix (#5763)"
requirements-completed: [QUICK-261103-em6]
status: complete
commits: 3
plan_head_before: 713d490955ebfcf3b00dd31186cfe5858deb6507
plan_head_after: 785fae96f75eaaac9330c97c814e2b29278540b3
actuals:
  tokens: 9600
  tasks: 3
  commits: 3
duration: ~1h
completed: 2026-11-03
---

# Phase quick-261103-em6 Plan 01: Re-port gsd-core v1.20.0 ROADMAP entry grammar and Progress-table owner Summary

Ported three gsd-core v1.20.0 reader rules into `src/state_reader/roadmap_md.rs`: a milestone-sliced Progress table (#5165), a fence-aware Progress section/table owner (#5761), and one line-anchored phase-entry prefix shared by every checklist recognizer (#5763).

## Commits (branch `quick/261103-em6`, see Deviations)

| Task | Commit | What |
| ---- | ------ | ---- |
| 1 | e3b32a1 | `normalize_milestone_token`, `roadmap_progress(content, state_milestone)` slice, mod.rs passes `&state.milestone`, end-to-end test |
| 2 | b2af8ac | `atx_heading`, `fenced_line_mask`, `is_progress_heading_title`; section + table rewrite; 8 `progress_5761_*` tests |
| 3 | 785fae9 | `phase_entry_open` + `PHASE_ENTRY_BOLD_OPEN`; R1/R2/R3 and the milestone `phase_checklist` rebuilt; 6 `entry_grammar_5763_*` tests |

## Gates

- `rtk proxy cargo test --no-fail-fast`: rc 0, 55 suites, 3237 passed, 0 failed, 17 ignored. Lib suite `test result: ok. 2599 passed; 0 failed; 1 ignored`. `tests/driver_router_conformance.rs` ran: 6 tests, all ok.
- `rtk proxy cargo clippy --all-targets -- -D warnings`: clean (Finished, no warnings).
- `cargo fmt --check`: clean.
- docs/GSD-CORE-SYNC.md, README.md and `GSD_CORE_SYNCED_*` constants unchanged since 22563cf. Corpus harness deleted, never staged.

## Corpus invariant (24 roadmaps)

Corpus: 15 `~/projects/*/*/.planning/ROADMAP.md` (found at depth 4, includes this checkout's), this checkout's `.planning/milestones/*-ROADMAP.md`, and every `tests/fixtures` file with `ROADMAP` in its name. 24 files total.

- `phases`, `shipped` and `milestones` Debug output: **byte-identical** before and after (diff of the non-`progress:` lines is empty). No real roadmap relied on the narrower HEAD grammar, so no never-real-entry exceptions had to be recorded.
- `progress:` lines changed for exactly two files (harness passes an empty milestone, so the slice uses the roadmap's in-progress milestone):
  1. `~/projects/flutter/wordoclock`: `None` -> `5 phases / 0 complete / 24 plans / 21 done`. Rule: #5761 first-matching-table (milestone summary table skipped, per-phase `v2.2` table read).
  2. `~/projects/python/ttbook`: `15/8/285/279` -> `8/1/87/81`. Rule: #5165 slice (v1, v2 and `v3 (M3)` rows in one table; only the in-progress milestone's rows count).
- picsync stays `None`, as planned.

## Inferred decisions [inferred — audit]

- **I-1** A document with no Progress heading returns `None` (not upstream's whole-document scan); mod.rs then keeps STATE.md counts. Avoids reopening the #2012 decoy risk.
- **I-2** `roadmap_progress` takes `state_milestone: &str`; mod.rs passes `state.milestone` (STATE.md, already backfilled from `active_milestone`). A blank argument falls back to `active_milestone(content)` inside the function.
- **I-3** No resolvable milestone, or no `Milestone` column: every row counts.
- **I-4** A resolved milestone with a `Milestone` column but no matching row returns `None` (STATE.md counts stand). Implementation detail: "matched" is recorded when a row's Milestone cell matches, before the sentinel/empty-phase checks, so a milestone whose only rows are backlog sentinels yields `Some(0,0,0,0)` rather than `None`.
- **I-5** `Phase`, `Plans Complete`, `Milestone` column names are case-insensitive (superset of upstream's exact match).
- **I-6** The table probe reads the section's lines including fenced ones (as upstream); only headings are fence-aware.
- **I-7** The `~~` strike slot between box and bold stays in every recognizer (local extension).
- **I-8** Indent narrowed from `\s*` to spaces/tabs plus optional BOM; corpus proves no real roadmap relied on it.
- **I-9** R1 keeps the `-` marker only (`*` bullets stay with R2, whose `split_bold_tail` strips `--` differently).
- **I-10** The milestone-scope `phase_checklist` keeps requiring the bold wrapper; it now also accepts `*` bullets and the shared gaps.
- **I-11** `plan_checklist_re`, the `## Milestones` bullet checks, and `phase_heading_re` are untouched; `is_fence_line` users untouched.
- R1 opens with `PHASE_ENTRY_BOLD_OPEN` (accepts `**` or `__`) but closes on `**` only, as the plan specified; a mismatched `__...**` line is parsed by R1 as before-but-now-anchored (no corpus line exercises it).

## Deviations from Plan

**1. [Process - Protected-branch guard] Work committed on branch `quick/261103-em6`, not `master`.**
- **Found during:** Task 1 commit.
- **Issue:** HEAD was on `master`. `gsd_run query git.base-branch --is-protected master` returns `true` and `git.allow_default_branch_commits` is not set, so the pre-commit assertion forbids committing there. I did not edit `.planning/config.json` or move any ref.
- **Fix:** created `quick/261103-em6` from master HEAD 713d490 and committed all three tasks there. The tree is left on that branch. The orchestrator can integrate with `git switch master && git merge --ff-only quick/261103-em6` (linear, no rebase). The orchestrator's docs commit must land after that, or on whichever branch it prefers.
- **Commits:** e3b32a1, b2af8ac, 785fae9.

**2. [TDD ordering] Task 2 implementation was written before its tests.**
- RED was re-established afterwards: the new `progress_5761_*` tests were run against the Task 1 version of the code (4 of 7 behavioral tests failed: fenced heading, fenced boundary, suffixed/indented titles, first-matching-table), then against the new code (all pass). Tasks 1 and 3 followed RED -> GREEN directly.

Otherwise the plan executed as written. The plan's `<automated>` verify blocks were XML-escaped (`&gt;`); I ran the equivalent checks by hand (named tests green, `fn` / issue-number greps, harness untracked/absent, sync files unchanged).

## Known Stubs

None.

## Threat Flags

None. Parsing stays on the linear-time `regex` crate; fence-aware heading scan closes the decoy-table path (T-em6-02); no new files, network or auth surface.

## Self-Check: PASSED

- src/state_reader/roadmap_md.rs and src/state_reader/mod.rs modified and present.
- Commits e3b32a1, b2af8ac, 785fae9 are ancestors of HEAD on `quick/261103-em6`.
- tests/zz_corpus_dump_tmp.rs absent; `git status --short` clean.
