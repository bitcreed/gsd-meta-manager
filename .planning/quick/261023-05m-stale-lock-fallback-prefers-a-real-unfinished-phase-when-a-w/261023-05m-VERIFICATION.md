---
phase: quick-261023-05m
verified: 2026-10-23T00:00:00Z
status: passed
score: 7/7 must-haves verified
covered_files:
  - ".planning/quick/261023-05m-stale-lock-fallback-prefers-a-real-unfinished-phase-when-a-w/261023-05m-PLAN.md"
  - ".planning/quick/261023-05m-stale-lock-fallback-prefers-a-real-unfinished-phase-when-a-w/261023-05m-SUMMARY.md"
  - "src/app.rs"
  - "src/state_reader/workstreams.rs"
  - "src/ui/screens/detail.rs"
covered_digest: "v3:sha256:026d3cce61f3aedc255f47ce25f67ea54052745a8cbfc0bcde8e6092e7ab7ee7"
behavior_unverified: 0
overrides_applied: 0
---

# Quick 261023-05m: Stale-lock fallback prefers a real unfinished phase - Verification

**Goal:** When a workstream's milestone.lock claim is lapsed (older than the 4 h TTL) or absent, `WorkstreamState::focus_phase_number` must prefer a real (has a directory), unfinished phase, and use the old rule only when none exists. Regression tests use the sentriq-shaped fixture.
**Status:** passed. **Re-verification:** No.

## Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Lapsed claim on the sentriq fixture gives 28, not 27 | VERIFIED | `focus_phase_number` (workstreams.rs:~171-178) is `live_claim_phase(now).or_else(real_unfinished_phase).unwrap_or_else(active_phase_number)`. Test `a_lapsed_claim_falls_back_to_the_real_unfinished_phase_28_not_the_directoryless_27` passed. It asserts focus 28, the phase name, `Partial` inference, and that `active_phase_number()` is still 27 with `NoDirectory`. |
| 2 | Absent claim gives 28, and an unlisted claim falls through to 28 | VERIFIED | `an_absent_claim_falls_back_to_the_real_unfinished_phase_28` and `a_claim_for_a_phase_the_roadmap_does_not_list_is_ignored` passed. |
| 3 | A live claim still leads | VERIFIED | `the_v014_focus_is_the_live_claim_26_not_declared_24_nor_frontier_27` passed. The step order puts the claim first. |
| 4 | The established rule answers only when no real unfinished phase exists | VERIFIED | `real_unfinished_phase` keeps `NoDirectory < status < Executed` (derived `Ord`; the enum order was confirmed in disk_status.rs). `a_workstream_without_a_claim_keeps_the_established_active_phase` (v0.13 gives 22) and `without_a_real_unfinished_phase_the_established_rule_still_answers` passed. |
| 5 | Ranking is stage first, then lowest number | VERIFIED | The code uses `max()` over `(status, Reverse(PhaseNum))`. `an_in_progress_phase_outranks_a_lower_numbered_discussed_or_planned_one` and `the_lowest_numbered_of_equally_advanced_in_progress_phases_wins` passed. |
| 6 | The root project's rule is untouched | VERIFIED | `git diff e6d0ca9..HEAD -- src/state_reader/mod.rs` is empty. The only caller of `real_unfinished_phase()` is `focus_phase_number` (workstreams.rs:176). |
| 7 | Consumers follow through the single change point | VERIFIED | `sentriq_roadmap_model_has_a_scoped_band_per_workstream_and_marks_the_focus_phase` passed (28 Active, 27 not Active). `a_lapsed_workstream_line_names_its_real_unfinished_phase` passed: the lapsed line reads "ws v0.14 P28: Mechanic Toolbox, ...", the live line reads P26, and the v0.13 lapsed line reads P22. The filter tests at lapsed instants still pass. |

The behaviours are all exercised by passing tests, so none is merely present or wired. The gate was not rerun in full.

## Behavioral Spot-Checks

| Command | Result | Status |
|---------|--------|--------|
| `cargo test --lib workstreams` | 26 passed, 0 failed | PASS |
| `cargo test --lib -- a_lapsed_workstream_line sentriq_roadmap_model filter` | 31 passed, 0 failed | PASS |

I did not run the full suite, clippy or fmt. The coordinator is running the suite. The SUMMARY's full-suite gate (4200 passed, 1 expected env-only failure) is taken as reported.

## Anti-Patterns

No TODO, FIXME or stub markers were introduced. The change in `src/` is the helper, the call-site chain, tests and doc comments. The helper works on in-memory state only, with no filesystem access in the render path.

## Real-tree note (informational)

The SUMMARY states that on today's real sentriq tree all of v0.14 is `Executed`, so the change makes no difference there. It only matters while a phase is mid-build, as in the fixture. This was not independently re-probed, because the reference project is read-only and the fixture tests cover the scenario.

## Human Verification Required

None.

## Gaps Summary

None. The goal is achieved: the fallback prefers a real, unfinished phase, and the old rule remains only as the last resort. Regression tests on the sentriq-shaped fixture cover lapsed, absent and unlisted claims, ranking, and the no-unfinished-phase case.

---
_Verifier: Claude (gsd-verifier)_
