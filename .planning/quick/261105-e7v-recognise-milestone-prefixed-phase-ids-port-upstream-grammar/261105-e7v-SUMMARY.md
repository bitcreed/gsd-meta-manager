---
phase: quick-261105-e7v
plan: 01
subsystem: state_reader
tags: [phase-ids, milestone-prefixed, gsd-core-1.20.1, roadmap, disk-status, conformance]
status: complete
requires: [261105-e7u]
provides:
  - "PhaseNum accepts gsd-core M-NN ids (3-01 == 03-01 == 3-1)"
  - "PhaseIdConvention { Legacy, MilestonePrefixed } gating the roadmap scanners and the directory rule"
  - "state_reader::phase_id_convention(planning_dir): one raw config.json read per snapshot, workstream -> root federation"
  - "two oracle-backed conformance fixtures (3-01, 3)"
key-files:
  modified:
    - src/state_reader/phase_num.rs
    - src/state_reader/mod.rs
    - src/state_reader/roadmap_md.rs
    - src/state_reader/disk_status.rs
    - tests/driver_router_conformance.rs
    - tests/state_reader_test.rs
    - tests/agents_waves.rs
decisions:
  - "Identity is convention-free (PhaseNum), only scanners are gated; exact config value `milestone-prefixed` opts in"
  - "depends_on keeps upstream range endpoints (Phase 3-01 -> [3, 01]); pinned by a test"
  - "No bare-integer directory fallback under milestone-prefixed (documented divergence from findPhaseInternal #2528)"
commits: 1
plan_head_before: 0e4a81bd49ffcfd628d863c7a5a5143535968b68
plan_head_after: 31f8166ad88853b1a34db39208bb76b7bdd6d541
actuals:
  tokens: 60000
  tasks: 2
  commits: 1
tests:
  command: "rtk proxy cargo test --no-fail-fast (raw stdout, `test result:` lines summed over 59 suites)"
  passed: 3638
  failed: 0
  ignored: 17
  clippy: "cargo clippy --all-targets -- -D warnings: clean"
  fmt: "cargo fmt --check: clean"
  oracle: "gsd-core 1.20.1 at ~/.claude/gsd-core; GSD_META_MANAGER_ALLOW_MISSING_ORACLE unset"
---

# Phase quick-261105-e7v Plan 01: Milestone-prefixed phase ids Summary

A project with `phase_id_convention: "milestone-prefixed"` in `.planning/config.json` now shows its `3-01` phases with their directories, goals, milestone bands and Progress counts, and the driver routes them as gsd-core 1.20.1's own `init.manager` does. Every other project reads exactly as before.

## What changed

- **phase_num.rs**: `PhaseNum` is now `{ major, sub: Option<u32>, decimals }`; one optional dash segment parses (`3-01`, `03-01`, `3-1` are one value displaying `3-01`, padding to `03-01`). Order `3 < 3.1 < 3-01 < 3-01.1 < 3-02 < 4`. `3-`, `-01`, `3--01`, `3-01-02`, `3.1-01`, `3-01a`, u32 overflow abstain. New `PhaseIdConvention` (`from_config_value` is true only for the exact string `milestone-prefixed`).
- **mod.rs**: `phase_id_convention(planning_dir)` reads one key from a raw `serde_json::Value` (logs nothing), with the `workstreams/<ws>` -> root federation; `parse_project_state_with` resolves it once and threads it. `config_json.rs` untouched.
- **roadmap_md.rs**: `PHASE_ID_MILESTONE_PREFIXED` + `phase_id_source(convention)`; R1/R2/R3/heading/R5 getters cache one regex per convention (Legacy compiles the byte-identical pattern); `parse_roadmap_phases`, `parse_shipped_phases`, `parse_phase_goals`, `parse_planned_build_phases`, `roadmap_milestones` take the convention; `locate_phase_section` reads M-NN headings under every convention; `0-NN` / `999-NN` are sentinels (also for Progress rows).
- **disk_status.rs**: `phase_dir_matches(dir, phase, convention)` reads the directory's leading M-NN token under the convention; `find_phase_dir` self-resolves the convention so its five callers are unedited; `infer_phase_status` takes the convention. Test renamed `test_phase_dir_matches_letter_prefixed_id` (assertion unchanged).
- **tests/driver_router_conformance.rs**: per-fixture `target`, `milestone_prefixed_skeleton`, fixtures `milestone_prefixed_m_nn_phase` (3-01) and `milestone_prefixed_bare_phase` (3). Lines above `ORACLE_VERB` kept in place.

## Verification

- Conformance fixtures **ran against the real oracle** (gsd-core 1.20.1; the missing-oracle opt-out variable is unset, and a missing oracle fails by default). Both new fixtures are `SameCommand`: `/gsd-execute-phase 3-01` and `/gsd-discuss-phase 3`, no divergence.
- Full suite 3638 passed / 0 failed / 17 ignored; clippy `-D warnings` and `fmt --check` clean.
- `git diff 0e4a81b -- Cargo.toml Cargo.lock` empty; no commit touches `config_json.rs`.

## Deviations from Plan

1. **[Process] One commit instead of two.** The task-2 pieces (milestone bands, sentinel rule, federation) are interleaved in the same hunks as task 1 and the tree does not compile with either half alone (`infer_phase_status` needs `phase_id_convention`), so the implementation is one atomic `feat` commit (31f8166). The `roadmap_milestones` parameter and the workstream federation landed with it.
2. **[Process] Tests written alongside the implementation, not strictly red-first.** The new tests were not observed failing before the implementation, so they are characterization plus regression pins. The Progress-row 999-01 case is the one the plan said fails today; it passes because `PhaseNum` now parses `999-01`, so the existing sentinel rule fires.
3. No Rule 1-4 auto-fixes were needed.

## Inferred decisions (for audit)

- Added `PhaseNum::has_milestone_segment()` (public) so `is_sentinel_phase` can apply the `0-NN` rule without exposing fields. Not in the plan's interface list.
- `phase_id_convention` treats a non-string or empty `phase_id_convention` value like an absent key (falls through to the root for a workstream); only a non-empty string stops federation. Matches upstream's "non-empty value wins".
- The conformance helper `every_fixture_reaches_the_state_it_is_named_for` maps the two new fixtures to `planned` (3-01) and `no_directory` (3) explicitly, since their names do not follow the state-name convention.
- No musk ledger entries were appended (M-0043 is referenced by the plan as already logged).

## Known Stubs

None.

## Threat Flags

None. T-e7v-04 (lone `03-02-beta` with a bare `3`) remains an accepted, documented divergence in the `phase_dir_matches` doc.

## Self-Check: PASSED

- Commit 31f8166 present on master; all seven modified files committed; working tree clean apart from this SUMMARY.
