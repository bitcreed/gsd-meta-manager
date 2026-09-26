---
phase: quick-260926-gtk
plan: 01
subsystem: config (state_reader/config_json, Defaults tab)
tags: [gsd-core-sync, config, defaults-tab, 1.15.0]
status: complete
requires: []
provides:
  - PlannerConfig typed block (planner.stall_detection_enabled + flatten extra)
  - WorkflowConfig.ui_interaction_capture
  - Two Defaults-tab Bool rows with since v1.15.0 (provisional)
  - GSD_CORE_SYNCED_COMMIT = v1.14.0-111-gec81d0d10 (VERSION stays 1.14.0)
  - Rewritten docs/GSD-CORE-SYNC.md
affects: [260926-gtl, 260926-gtm, 260926-gtn]
tech-stack:
  added: []
  patterns: [typed nested block with serde(flatten) extra, RESYNCED_KEYS_<version> per-sync key table]
key-files:
  created: []
  modified:
    - src/state_reader/config_json.rs
    - src/ui/screens/detail.rs
    - docs/GSD-CORE-SYNC.md
decisions:
  - "VERSION/COMMIT split: COMMIT tracks the synced tree (release-1.15.0 ec81d0d); VERSION is the npm-published oracle pin (1.14.0)"
  - "planner is a typed block with only stall_detection_enabled typed; stall_* tuning knobs stay pass-through"
metrics:
  duration: ~35m
  completed: 2026-09-26
  tasks: 3
  files: 3
actuals:
  tokens: 12700
  tasks: 3
  commits: 3
plan_head_before: 228a063b9b0a74f1a46b365ff71f73240a671c11
---

# Quick 260926-gtk: Config re-sync to gsd-core release-1.15.0 Summary

The two keys gsd-core release-1.15.0 added, `planner.stall_detection_enabled` and `workflow.ui_interaction_capture`, are now typed, editable Bool rows in the Defaults tab. Each is marked `since v1.15.0`. The planner stall-tuning knobs stay visible as read-only `planner.<key>` rows and are not lost on save. The sync baseline now points at ec81d0d. The npm oracle pin stays at 1.14.0.

## Commits

| Task | Commit | Message |
|---|---|---|
| 1 (tracer) | 85afd7c | feat(quick-260926-gtk): model gsd-core 1.15.0 planner.stall_detection_enabled |
| 2 | 52cd8ef | feat(quick-260926-gtk): model gsd-core 1.15.0 workflow.ui_interaction_capture |
| 3 | 6da18c3 | docs(quick-260926-gtk): move gsd-core sync baseline to release-1.15.0 ec81d0d |

## What changed

- **config_json.rs**:
  - New `PlannerConfig` with `stall_detection_enabled: Option<bool>` and a `#[serde(flatten)] extra`. It hangs off `GsdConfig.planner`.
  - New field `WorkflowConfig.ui_interaction_capture`.
  - `GSD_CORE_SYNCED_COMMIT` is now `v1.14.0-111-gec81d0d10`. `GSD_CORE_SYNCED_VERSION` stays `1.14.0`, and both doc comments now explain the split, the ref-based re-measure commands, and the exit condition.
  - New tests: `planner_stall_detection_enabled_is_a_modelled_planner_key` and `ui_interaction_capture_is_a_modelled_workflow_key`. The blocks-absent test now also covers `planner`.
- **detail.rs**:
  - `planner.stall_detection_enabled` sits in Planning, right after `planning.pr_strict`. It uses `with_choices` for true/false.
  - `workflow.ui_interaction_capture` sits in Execution, right after `workflow.live_dom_uat`.
  - Both keys have set, clear and toggle arms.
  - `take("planner.", …)` was added to the pass-through walk.
  - `"planner"` was added to `NESTED_CONFIG_BLOCKS`.
  - The fixture carries both keys.
  - `DEFAULTS_OPTION_COUNT` goes from 130 to 132, and its doc comment records both measurements.
  - New table `RESYNCED_KEYS_1_15_0`, pinned at 2 in `the_resync_covers_…`. The two per-key loops now chain it.
  - New tests: `every_gsd_core_1_15_0_key_is_an_editable_row_writing_its_own_json_path` (checks the written path via JSON pointer) and `planner_stall_tuning_keys_stay_visible_and_survive_a_toggle`.
- **docs/GSD-CORE-SYNC.md**: fully rewritten. It keeps the `## Modelled` and `## Pass-through` headings and now has:
  - a header section on the VERSION/COMMIT split;
  - a re-measure recipe that works against a git ref;
  - the measured 1.15.0 delta;
  - a step-3 recount (169/113);
  - the 132-row intro;
  - the `planner.stall_*` pass-through entry;
  - a next-sync list whose first item is the VERSION bump.

## TDD evidence

- Task 1 RED:
  - config_json.rs tests did not compile (E0609, no `planner` field on `GsdConfig`). This was expected.
  - After the struct landed, and before the `take("planner.")` line and the row were added, three tests failed: the census `an_unmodelled_key_in_every_nested_block_becomes_a_visible_pass_through_row` (len assertion, detail.rs:12104), `planner_stall_tuning_keys_…` and `every_gsd_core_1_15_0_key_…`.
  - GREEN: 16/16 filtered tests passed.
- Task 2 RED: compile failure (E0609, no `ui_interaction_capture`). GREEN: 17/17 filtered tests passed, and the Enum count is still 12.

## Upstream re-measure (Task 3, 2026-09-26)

- `describe release-1.15.0` gives `v1.14.0-111-gec81d0d10` (sha ec81d0d10f545dd5aea2cc893863a542bc49029d). package.json says 1.15.0. `npm view @opengsd/gsd-core version` gives `1.14.0`. `tag --contains` is empty for both introducing commits (1e3e1f7cd = v1.14.0-98, 88b5775dc = v1.14.0-80).
- CONFIGURATION.md key cells went from 258 to 260: +2 (`planner.stall_detection_enabled`, `workflow.ui_interaction_capture`), none removed.
- The schema manifest's `validKeys` went from 120 to 121. The plan said 121 -> 122; that figure also counts the one `runtimeStateKeys` entry. The docs record both.
- The defaults manifest gained only the `planner` block. No existing default changed.
- The step-3 gap is now **169 lines, 113 dotted**, as expected. The `push(cat, …)` count is 132.

## Gates

- `rtk proxy cargo test --no-fail-fast`: 56 suites, **2684 passed, 1 failed, 15 ignored**. The one failure is the permitted `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against` (local git version witness).
  - The first full run also failed `tests/envelope_carrier_reach.rs::the_t_19_116_replacement_takes_layer_2_as_well_and_that_is_measured_separately` with `ExecutableFileBusy` (ETXTBSY, "Text file busy"). That is a race when the test spawns a binary, not something the config change caused.
  - That suite then passed 39/39 three times in isolation, and a clean second full run gave the numbers above.
  - Recorded as a flaky test (see Deferred).
- `rtk proxy cargo clippy --all-targets -- -D warnings`: **exit 0**.
- Constant grep gate (Task 3 verify): OK.

## INFERRED decisions (for audit)

- **INFERRED I-1:** `since` is `v1.15.0` for both keys, but it is provisional. No v1.15.0 tag exists, and the introducing commits are reachable only from release-1.15.0, whose package.json says 1.15.0. The code comments and the sync record say so.
- **INFERRED I-2:** `workflow.ui_interaction_capture` is placed in Execution next to `workflow.live_dom_uat`, the other browser-driven gate.
- **INFERRED I-3:** only `planner.stall_detection_enabled` is typed. `planner.stall_detect_interval_minutes` and `planner.stall_threshold_minutes` stay pass-through, following the record's existing runtime-tuning-knobs decision.
- **INFERRED I-4:** the toggle uses the file-wide convention `Some(!v.unwrap_or(false))`. From unset, the first Enter writes `true`, which is gsd-core's default, the same as the `gates.*` rows.
- **INFERRED I-5 (accepted divergence):** gsd-core treats a non-boolean `planner.stall_detection_enabled` as `true`. Here the key is a typed `Option<bool>`, so a hand-edited non-boolean makes the whole file fail to parse, like every other typed key. That failure draws no rows, so nothing can be saved over the file and nothing is lost.
- **INFERRED I-6:** `GSD_CORE_SYNCED_COMMIT` uses the `git describe` spelling, which contains the abbreviated sha ec81d0d and satisfies `the_gsd_core_sync_baseline_is_recorded`.
- **INFERRED I-7:** `RESYNCED_KEYS_1_15_0` is kept separate from `RESYNCED_KEYS`, so the 1.14.0 counts (57 / 27) stay a pinned historical measurement.
- **INFERRED I-8 (executor):**
  - When I re-bucketed the step-3 output, eight undotted top-level gsd-core keys turned out to have no typed row: `runtime`, `context_profile`, `agent_skills`, `kg_backend`, `tavily_search`, `ref_search`, `perplexity`, `jina`. So the 1.14.0 record's claim that the gap list contains no missing scalar key was wrong.
  - None of these keys is new in the 1.15.0 range, so they were not promoted. They are documented as pass-through, and the record's wording was corrected.
- **INFERRED I-9 (executor):** I left out a `git fetch` step in the re-measure recipe, because the record says nothing there writes to the upstream checkout.

## Deviations from Plan

- **[Rule 1 - Accuracy] Schema-manifest count.** The plan said 121 -> 122. The measured `validKeys` count is 120 -> 121; 122 is the total including `runtimeStateKeys`. The docs record both. This changes no code.
- **[Scope note] Step-3 buckets.** The plan asked for a recount only. I also split the old "~58 value rows" bucket into 48 value rows plus 8 real top-level pass-through keys, and corrected the inaccurate "none is a missing scalar key" sentence (I-8).
- **[Rule 2 - small addition]** `the_resync_covers_every_key_the_drift_measurement_found` now also checks `RESYNCED_KEYS_1_15_0.len() == 2`, so a row quietly dropped from the new table is caught.
- **[Cosmetic]** The comment inside the `push` closure now says "133rd option" instead of "131st".

No README or user-doc change was needed. No user doc lists Defaults-tab keys or the gsd-core version; `docs/GSD-CORE-SYNC.md` is the doc for this, and it was rewritten.

## Deferred

- The flaky ETXTBSY spawn race in `tests/envelope_carrier_reach.rs:909` (seen once, not reproduced in 3 isolated runs).
- Promoting the eight undotted top-level keys (I-8). `runtime` touches item D's #4717 semantics. The four search-provider keys double as API-key slots, and pass-through rows currently show them unmasked. That behaviour predates this work; worth a masking decision.
- Bumping VERSION to 1.15.0, and turning the provisional `since` values into measured tags, once 1.15.0 is on npm and tagged. This is the first item in the record's "Next sync" list.

## Threat model

- T-gtk-01: the pass-through `planner.*` rows are ReadOnly (tested).
- T-gtk-02: a toggle-then-save keeps the tuning knobs, and the round-trip is a fixed point (tested).
- T-gtk-04: VERSION is pinned at 1.14.0 (grep gate), and `scripts/` and `.github/` were not edited.
- No new threat surface.

## Self-Check: PASSED

- FOUND: src/state_reader/config_json.rs, src/ui/screens/detail.rs, docs/GSD-CORE-SYNC.md
- FOUND commits: 85afd7c, 52cd8ef, 6da18c3 (`git rev-list --count 228a063..HEAD` = 3)
