---
phase: 24-roadmap-tab-redesign-and-detail-tab-consolidation
plan: 01
subsystem: state_reader
tags: [roadmap, parser, fixtures, milestones, build-phases, goals]
status: complete
requires: []
provides:
  - roadmap_md::parse_planned_build_phases (+ private parse_build_depends_on)
  - roadmap_md::parse_phase_goals
  - roadmap_md::shipped_milestones
  - roadmap_md::declared_phase_count
  - ProjectState.planned_phases / phase_goals / milestone_name
  - tests/fixtures/roadmaps (six sanitised excerpts + provenance README)
affects: [24-02, 24-05, 24-06]
tech-stack:
  added: []
  patterns:
    - "Display-only third-party text on ProjectState as Untrusted (Option<Untrusted>, HashMap<String, Untrusted>) so the free-string census is unchanged"
    - "Real-roadmap regression fixtures vendored as sanitised structural excerpts, read with include_str! only, guarded by a unit test"
key-files:
  created:
    - tests/fixtures/roadmaps/README.md
    - tests/fixtures/roadmaps/daily-vow-ROADMAP.md
    - tests/fixtures/roadmaps/daily-vow-STATE.md
    - tests/fixtures/roadmaps/sentriq-ROADMAP.md
    - tests/fixtures/roadmaps/sentriq-STATE.md
    - tests/fixtures/roadmaps/ttbook-ROADMAP.md
    - tests/fixtures/roadmaps/ttbook-STATE.md
  modified:
    - src/state_reader/roadmap_md.rs
    - src/state_reader/mod.rs
key-decisions:
  - "Build phases live in ProjectState.planned_phases, never in phases: GSD's heading grammar does not count `Build phase` headings, so the router/frontier stay GSD-conformant (pinned by a characterisation test)"
  - "Build-phase dependency ranges expand only to known phase ids (GSD phases plus build phases), in numeric order, so a huge span allocates nothing"
  - "shipped_milestones: only milestones with a range or scoped phases qualify; before the active milestone, else lower by numeric version order, else none"
  - "The milestone detector's phase-heading recogniser accepts an optional case-insensitive `Build ` prefix and lowercase `phase`; its range regex accepts `Phases (4-7)`"
requirements-completed: [D-A03, D-A07, D-A12, D-A14]
duration: 12 min
completed: 2026-09-24
plan_head_before: 063ace12f4c12375f7b8413224d900b9c18a42b2
actuals:
  tokens: 15100
  tasks: 3
  commits: 5
coverage:
  - deliverable: "ttbook build-phase headings parse as planned phases with dependencies; GSD phase list unchanged"
    human_judgment: false
    verification:
      - kind: unit-test
        ref: "src/state_reader/roadmap_md.rs#ttbook_fixture_gsd_phase_list_is_unchanged_by_build_phase_support"
        status: pass
      - kind: unit-test
        ref: "src/state_reader/roadmap_md.rs#ttbook_build_phase_headings_parse_as_planned_phases"
        status: pass
      - kind: unit-test
        ref: "src/state_reader/roadmap_md.rs#a_build_dependency_range_expands_only_to_known_ids"
        status: pass
      - kind: unit-test
        ref: "src/state_reader/roadmap_md.rs#a_build_heading_with_no_parenthetical_parses"
        status: pass
      - kind: unit-test
        ref: "src/state_reader/mod.rs#parse_project_state_reads_planned_build_phases"
        status: pass
      - kind: command
        ref: "rtk proxy cargo test --test driver_router_conformance --no-fail-fast"
        status: pass
  - deliverable: "Build-phase headings no longer create spurious milestones; exactly v1, v2, M3, M4, M5 on ttbook"
    human_judgment: false
    verification:
      - kind: unit-test
        ref: "src/state_reader/roadmap_md.rs#build_phase_headings_are_not_milestones"
        status: pass
  - deliverable: "Phase goals (both bold forms) and STATE milestone_name on ProjectState, census unchanged"
    human_judgment: false
    verification:
      - kind: unit-test
        ref: "src/state_reader/roadmap_md.rs#a_goal_is_read_in_both_bold_forms"
        status: pass
      - kind: unit-test
        ref: "src/state_reader/roadmap_md.rs#a_goal_after_the_next_heading_is_not_attributed_upward"
        status: pass
      - kind: unit-test
        ref: "src/state_reader/roadmap_md.rs#fixture_goals_cover_every_detailed_phase"
        status: pass
      - kind: unit-test
        ref: "src/state_reader/mod.rs#parse_project_state_reads_milestone_name_after_progress"
        status: pass
      - kind: unit-test
        ref: "src/state_reader/mod.rs#parse_project_state_reads_phase_goals"
        status: pass
      - kind: command
        ref: "rtk proxy cargo test --test spawn_seam_guard --no-fail-fast"
        status: pass
  - deliverable: "Shipped-milestone flags and declared phase counts on all three real shapes"
    human_judgment: false
    verification:
      - kind: unit-test
        ref: "src/state_reader/roadmap_md.rs#daily_vow_shipped_milestones_are_the_five_before_v1_5"
        status: pass
      - kind: unit-test
        ref: "src/state_reader/roadmap_md.rs#sentriq_v0_11_is_shipped_by_version_order"
        status: pass
      - kind: unit-test
        ref: "src/state_reader/roadmap_md.rs#ttbook_v1_is_shipped"
        status: pass
      - kind: unit-test
        ref: "src/state_reader/roadmap_md.rs#a_milestone_without_range_or_members_is_never_shipped"
        status: pass
      - kind: unit-test
        ref: "src/state_reader/roadmap_md.rs#declared_phase_count_counts_integer_majors"
        status: pass
  - deliverable: "Six sanitised fixtures plus provenance README, guarded"
    human_judgment: true
    rationale: "The guard proves Goal markers and no home paths; whether the residual structure and kept daily-vow/sentriq titles are acceptable to publish on crates.io is the operator's risk acceptance (T-24-03), flagged for the pre-release audit"
    verification:
      - kind: unit-test
        ref: "src/state_reader/roadmap_md.rs#vendored_roadmap_fixtures_are_sanitised"
        status: pass
---

# Phase 24 Plan 01: Roadmap reader facts for the new Roadmap tab Summary

**The ROADMAP reader now returns ttbook's `#### Build phase N (Milestone M)` placeholders as `ProjectState.planned_phases`, with their dependencies (ranges like `Build phases 8-13` expand against known ids). It also reads every phase's Goal in both bold forms, carries STATE.md's `milestone_name`, and marks shipped milestones with their declared phase counts. None of this touches the GSD-facing `phases` list, which is checked against six sanitised real-roadmap fixtures.**

## Performance

- **Duration:** 12 min
- **Started:** 2026-09-24T02:55:50Z
- **Completed:** 2026-09-24T03:07:57Z
- **Tasks:** 3 (1 tracer, 2 TDD)
- **Files modified:** 9 (7 created, 2 modified)

## Accomplishments

- `parse_planned_build_phases` returns one `RoadmapPhase` per build heading (number, title, plan counts, dependencies). On ttbook, 14 depends on 8-13, 15 on 14, 16 on 12 and 13, 17 on 12, 13 and 16, and 18 on 12. `parse_roadmap_phases` gives byte-identical output on the ttbook fixture (numbers 8-13, same deps and plan counts), and `driver_router_conformance` stays green.
- `roadmap_milestones` on ttbook now returns exactly `v1, v2, M3, M4, M5`. Before, it returned five spurious `Build phase …` milestones. Build ids 14/15 belong to M3, 16/17 to M4 and 18 to M5, by bullet range and by heading scope.
- `parse_phase_goals` reads `**Goal**:` and `**Goal:**` (case-insensitive). The first Goal line in an entry wins, keys are pad-insensitive (`07` equals `7`), and an entry ends at the next heading. Goals are stored as `Untrusted` in `ProjectState.phase_goals`.
- `ProjectState.milestone_name: Option<Untrusted>` is read even when the key comes after `progress:` (the sentriq shape). An empty value gives `None`.
- `shipped_milestones`: daily-vow v1.0 through v1.4 (declared counts sum to 17), sentriq `v0.11` (by numeric version order; the range 4-7 is now read from `Phases (4-7)`), and ttbook v1 (count 7).
- The six fixtures and the provenance README are guarded by `vendored_roadmap_fixtures_are_sanitised`.

## Task Commits

1. **Task 1: ttbook build phases end to end (tracer)**: `9e17dea` (feat)
2. **Task 2: Goals and milestone_name**: `e5b52d1` (test, RED) → `ef86625` (feat, GREEN)
3. **Task 3: Shipped-milestone facts and the sanitisation guard**: `6805fd9` (test, RED) → `2466e1d` (feat, GREEN)

No REFACTOR commits were needed.

## Files Created/Modified

- `src/state_reader/roadmap_md.rs`: adds `build_heading_re`, `any_heading_re`, `parse_build_depends_on`, `parse_planned_build_phases`, `parse_phase_goals`, `version_segments`, `version_lower`, `shipped_milestones` and `declared_phase_count`. Also widens the milestone detector's phase-heading and range regexes, and adds 18 tests.
- `src/state_reader/mod.rs`: adds the `planned_phases`, `phase_goals` and `milestone_name` fields with docs, assigns them in `parse_project_state`, and adds 3 tests.
- `tests/fixtures/roadmaps/*`: six sanitised excerpts plus a provenance README (sources, commits, sanitisation rules, the defect each pins, rules for adding a fixture).

## TDD Gate Compliance

- Task 2: RED `e5b52d1` (6 target tests failed on assertions; `check tdd-red-evidence` returned `RED_EVIDENCE_OK`), then GREEN `ef86625`.
- Task 3: RED `6805fd9` (5 target tests failed on assertions; `RED_EVIDENCE_OK`), then GREEN `2466e1d`.
- To keep RED from failing on compile errors (INVALID_RED), each RED commit included empty stub signatures (`parse_phase_goals` returning an empty map, the new fields left unassigned, `shipped_milestones` returning all `false`, `declared_phase_count` returning 0).
- The RED evidence was cargo output turned into TAP (one line per `test … ok/FAILED`) so the Node checker could read it.
- `vendored_roadmap_fixtures_are_sanitised` is an invariant guard, not a behaviour test. It went into the GREEN commit because it passes against the already-sanitised fixtures by design.

## Inferred decisions (for audit)

1. **Build phases stay out of `state.phases`** (plan-level [INFERRED]). The Roadmap adapter (24-05) merges `planned_phases` in after `phases`.
2. **Fixture sanitisation** (T-24-03). ttbook titles and milestone names were replaced with neutral titles. daily-vow and sentriq titles were kept per the plan (the v1.5 and v0.12 names appear in public `MOCKUPS.md`; daily-vow's v1.0-v1.4 milestone names do not, but the plan keeps them and they are generic). Goal bodies became `(sanitised) ` plus neutral filler, with ttbook phase 8's goal longer than 120 characters for later wrap tests. Parenthetical contents in Depends-on lines were neutralised, keeping any `Phase N` token. Daily-vow's shipped-phase history table keeps only ids, milestone, plan counts and dates. The fixtures are committed, but they become permanent once published to crates.io, so they are flagged for the operator's pre-release audit.
3. **ttbook source drift.** The ROADMAP was excerpted at `9b7b305`, which is newer than RESEARCH's `d54fa75`. Its shape matches what RESEARCH describes.
4. **STATE fixtures keep only the keys the plan names** (plus `percent` inside `progress:`). `gsd_state_version`, `stopped_at`, `last_*` and `state_head` were dropped, and source key order is kept.
5. **Build-dependency grammar details:**
   - The plain `Phase` keyword is case-sensitive (as in `parse_depends_on`); `Build phase(s)` is case-insensitive.
   - Comma lists are accepted after any keyword.
   - Range expansion is sorted numerically.
   - Dedupe is pad-insensitive (`phase_key`), keeping the first spelling.
   - Non-numeric range endpoints (`M-2-M-5`) expand to nothing.
6. **`parse_planned_build_phases` also applies the sentinel (`0`/`999`) and strikethrough exclusions** that `parse_roadmap_phases` uses, and merges duplicate headings with `merge_duplicate_phases`.
7. **Goals:** an empty first Goal line closes the entry, so a later Goal line in the same entry is ignored ("first wins, empty or not"). `**Milestone Goal:**` lines are not phase goals.
8. **`declared_phase_count`** skips inserted decimals; this is a display approximation, documented on the function.
9. **Known difference from Mockup C:** its "earlier" row also names `pre-GSD 1–3` and `TASK-111 (quick)`. Those are index-only table sections, not milestones under any detector the reader has, so only `v0.11` is reported shipped. This is recorded in the README's sentriq row.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Corrected a wrong expectation in a Task 3 test**
- **Found during:** Task 3 GREEN
- **Issue:** `a_milestone_without_range_or_members_is_never_shipped` asserted `shipped_milestones(&ms, "v0.10")` would use version order. But `v0.10` names a roadmap milestone (`v0.10 Loose`), so the plan's rule 1 (the roadmap order before the active milestone) applies instead.
- **Fix:** The numeric-order case now uses `v0.10.1`, which is absent from the roadmap (lexicographically `v0.11` < `v0.10.1`; numerically it is not). A separate assertion covers the rule-1 case with `v0.20`.
- **Files modified:** `src/state_reader/roadmap_md.rs` (test only)
- **Commit:** `2466e1d`

**Total deviations:** 1 auto-fixed (a test-expectation error). **Impact:** none on scope; the implementation follows the plan's algorithm exactly.

## Issues Encountered

- **Clippy baseline mismatch (pre-existing, out of scope).** `rtk proxy cargo clippy --all-targets -- -D warnings` reports 11 errors, not the 7 given as the baseline. The extra 4 are in `tests/envelope_wrapper_class.rs` (lines 6127, 6213, 10795, 11232), which this plan did not touch (`git diff 063ace1 -- tests/envelope_wrapper_class.rs` is empty; last changed in 19-33). The 7-error baseline was most likely taken with compilation cut short before that target was linted. This plan adds no new clippy errors: `cargo clippy -- -D warnings` (lib) is clean, and every all-targets error sits in a file outside this plan.

## Verification

- `rtk proxy cargo test --lib state_reader`: 294 passed, 0 failed.
- `rtk proxy cargo test --test driver_router_conformance --no-fail-fast`: 3 passed.
- `rtk proxy cargo test --test spawn_seam_guard --no-fail-fast`: 41 passed. `git diff --stat 063ace1 -- src/driver/untrusted.rs tests/spawn_seam_guard.rs` is empty.
- `rtk proxy cargo test --no-fail-fast`: 49 `test result:` lines; 2269 passed (baseline 2251 plus 18 new), 1 failed, 15 ignored. The only failure is `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against` (the known local git-version witness).
- `rtk proxy cargo clippy -- -D warnings`: clean.
- The `parse_depends_on` and `parse_roadmap_phases` function bodies are byte-identical to `063ace1` (md5 of each function body matched).
- `tests/fixtures/roadmaps/ttbook-ROADMAP.md` has 5 `#### Build phase ` lines, each with `(Milestone `. No fixture contains `/home/`.
- The sentriq STATE fixture has `milestone_name:` on line 11, after `progress:` on line 6.

## Threat Flags

None. No new surface beyond the plan's threat model: T-24-01/04 mitigated (text stored as `Untrusted`, census unchanged), T-24-02 mitigated (linear-time `regex`, range expansion bounded by the known ids), T-24-03 mitigated by the guard test with the residual flagged above, T-24-05 mitigated (separate `planned_phases`, characterisation test, router conformance green).

## Next Phase Readiness

Plans 24-02 (layout engine) and 24-05 (Roadmap adapter) can now read `state.planned_phases`, `state.phase_goals`, `state.milestone_name`, `shipped_milestones` and `declared_phase_count`, and pin behaviour against the fixtures in `tests/fixtures/roadmaps/`. Ready for 24-02.

## Self-Check: PASSED

- All 7 created files and 2 modified files exist on disk.
- All 5 commits (`9e17dea`, `e5b52d1`, `ef86625`, `6805fd9`, `2466e1d`) exist in `git log`.
- `commits: 5` measured with `git rev-list --count 063ace1..HEAD` before the docs commit.
