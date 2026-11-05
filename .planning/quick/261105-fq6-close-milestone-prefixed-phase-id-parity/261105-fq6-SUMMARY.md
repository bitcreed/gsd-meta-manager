---
phase: quick-261105-fq6
plan: 01
subsystem: state_reader, driver/router
tags: [milestone-prefixed, phase-id, gsd-core-1.20.1, parity, musk-M-0043]
status: complete
requires: [quick-261105-e7v]
provides:
  - upstream matchPhaseDirs port (incl. #2528 unique bare-integer fallback) under milestone-prefixed
  - M-NN summary pairing and PLAN.md roadmap-row counting
  - convention-aware extract_phase_id (G15, HANDOFF)
  - 5 oracle-backed conformance fixtures
key-files:
  modified:
    - src/state_reader/disk_status.rs
    - src/state_reader/roadmap_md.rs
    - src/state_reader/mod.rs
    - src/state_reader/workstreams.rs
    - src/driver/router.rs
    - tests/driver_router_conformance.rs
    - .planning/musk/ledger.jsonl
decisions:
  - "M-NN directory lookup is the legacy matchPhaseDirs: primary token pass plus a set-rule bare-integer fallback; Ambiguous refuses and stops before archives"
  - "No M-NN plan_index grammar; tier-2 summary pairing narrows to two-segment stems under milestone-prefixed"
  - "ProjectState carries phase_id_convention (Copy enum)"
  - "M-0043 closed with musk reinstate; item 3 recorded not-applicable"
metrics:
  duration: ~35 min
  completed: 2026-11-05
actuals:
  tokens: 9200
  tasks: 3
  commits: 3
plan_head_before: 0ba622d57248efdae937a1017f631c633944cfd5
plan_head_after: a6b9d5140371b15918276e09eb1466322990b199
---

# Quick 261105-fq6: Close milestone-prefixed phase-id parity gaps (M-0043)

On a `milestone-prefixed` project the app now reads directories, plan ids, G15 identities and HANDOFF phases the way gsd-core 1.20.1 does. All five measured divergence cases agree with the installed oracle. Default (Legacy) behaviour and `parse_depends_on` are unchanged.

## Commits

| Commit | Content |
|---|---|
| 828068f | Task 1 (tracer): matchPhaseDirs port, `infer_disk_status_with` and tier-2 narrowing, 3 fixtures |
| cf967c9 | Task 2: plan rows, whole-id `extract_phase_id`, `ProjectState.phase_id_convention`, HANDOFF/G15, 2 fixtures |
| a6b9d51 | Task 3: `docs(musk): reinstate M-0043` (via `gsd-tools musk commit`) |

Commit 828068f also holds the tier-2 pairing rule and its test, which the plan lists under Task 2. They live in the same file as the Task 1 resolver and were committed together.

## Per-item outcome (M-0043)

1. **Bare-integer fallback / matchPhaseDirs: done.**
   - Added `upstream_normalize_phase_name`, `strip_project_code_prefix`, `upstream_dir_token`, `unpad`, `DirSelection` and `select_phase_dir`.
   - `phase_dir_matches` under M-NN is now phaseTokenMatches.
   - `find_phase_dir_in` and the archive pre-scan use the resolver. An `Ambiguous` result in `phases/` returns `None` without consulting archives.
2. **Plan ids: done.**
   - Tier-2 summary pairing under M-NN requires a two-segment stem. `03-01-01-setup-PLAN.md` beside `03-01-01-SUMMARY.md` is Planned with summary_count 0. `03-01-SUMMARY.md` still pairs.
   - `plan_checklist_re(convention)` counts `03-01-01-PLAN.md` rows under M-NN. The Legacy pattern string is byte-identical.
3. **Milestone-list ranges: not applicable, no code.** Evidence is recorded in the `roadmap_milestones` doc and the M-0043 reinstate reason. gsd-core 1.20.1 has no milestone-list range reader, its migrator never rewrites milestone bullets, and its template shows only legacy `Phases 1-4`.
4. **`extract_phase_id`: done.**
   - It takes a convention. G15 `phase_identity` uses `state.phase_id_convention`, and HANDOFF.json reads the whole M-NN id.
   - A sibling's `3-02` Deferred Verification row no longer parks `3-01`. The target's own row still parks as `3-02`, `03-01` or `Phase 3-01`.

## Verification (raw, oracle present, `GSD_META_MANAGER_ALLOW_MISSING_ORACLE` unset)

- `rtk proxy cargo test --no-fail-fast`: **3645 passed, 0 failed, 17 ignored** (sum of every `test result` line; baseline 3638 plus 7 new tests).
- Conformance: **12 command comparisons, 4 declared divergences, 2 upstream-silent states over 18 fixtures**, no divergence from oracle 1.20.1. The 5 new `milestone_prefixed_*` fixtures are all `Upstream::SameCommand`.
- `rtk proxy cargo clippy --all-targets -- -D warnings`: clean.
- `rtk proxy cargo fmt --check`: clean.
- `parse_depends_on` body is byte-identical to BASE (diff exit 0), and `Cargo.toml` / `Cargo.lock` are unchanged. `milestone_prefixed_depends_on_keeps_upstream_range_endpoints` is not in the diff and passes.
- `gsd-tools musk list --json` reports M-0043 `reinstated: true`.

## Deviations from Plan

None in behaviour. One addition outside the plan's file list is recorded below.

**[Rule 3 - blocking] `workstreams.rs` call site.** `read_milestone_claim` called `handoff_phase`, whose signature gained a convention. It now passes the workstream's own `phase_id_convention(planning_dir)`, so an M-NN `milestone.lock` claim `"3-02"` names phase 3-02 and no longer phase 3. Legacy is byte-identical.

Musk reinstate step 2 (reverse the cut's edits) had nothing to reverse. M-0043 is a class-impl, unit-task cut, so no REQUIREMENTS or ROADMAP line was ever moved.

## Inferred decisions (for audit)

1. Wholesale replacement of e7v's M-NN directory rule (`dir_leading_phase`) by the matchPhaseDirs port. e7v's `14-2026-foo` assertion is flipped, because the oracle measures that directory as phase 14.
2. No M-NN `plan_index` grammar, because upstream has none (`extractCanonicalPlanId` reads `03-01-01-x` as `03-01` everywhere). The narrowing is in tier-2 pairing only. The uniqueness abstention is kept.
3. Plan-row widening `(?:-\d+)?` is [inferred]: upstream has no roadmap plan-row counter. This local counter reads the filename upstream writes.
4. Item 3 closed as not applicable on source evidence from gsd-core v1.20.1.
5. `ProjectState` gains `phase_id_convention` (a Copy enum, census-safe).
6. `milestone.lock` claim phases use the workstream's convention (see Deviations).
7. `musk reinstate` is the verb used to close M-0043.

Pre-existing, convention-independent divergences, left unchanged and outside M-0043:
- Legacy first-match where upstream refuses ambiguity.
- Archive-first lookup order in `infer_phase_status`.
- Legacy-numbered `Phases 1-4` bullets left in a migrated M-NN roadmap still band by numeric order. This is a local display rule with no upstream reader.

## Known Stubs

None.

## Threat Flags

None. All new code parses untrusted directory names with split, char predicates and linear-time regex captures. Digit runs are compared as strings, so a long run cannot overflow.

## Self-Check: PASSED

- Commits 828068f, cf967c9 and a6b9d51 are ancestors of HEAD.
- `commits: 3` is measured with `git rev-list --count ${PLAN_HEAD_BEFORE}..HEAD`.
- `git status` is clean apart from the docs artifacts the orchestrator commits.
