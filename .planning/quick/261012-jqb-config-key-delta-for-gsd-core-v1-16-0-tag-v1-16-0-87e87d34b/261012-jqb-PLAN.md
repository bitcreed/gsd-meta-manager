---
phase: quick-261012-jqb
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/state_reader/config_json.rs
  - src/ui/screens/detail.rs
  - docs/GSD-CORE-SYNC.md
autonomous: true
requirements: [QUICK-261012-jqb]

must_haves:
  truths:
    - "The Defaults tab shows `gates.confirm_milestone_scope` (category Gates, directly after `gates.confirm_transition`) and `audit.enabled` (new category Observability, after Gates) as editable Bool rows whose help pane draws `since gsd-core v1.16.0`"
    - "Set, toggle and clear on each new row write exactly the JSON path `/gates/confirm_milestone_scope` / `/audit/enabled` (asserted through serde_json::to_value + JSON pointer, not through the struct)"
    - "The six keys gsd-core v1.16.0 removed (gates.confirm_project, gates.confirm_phases, gates.confirm_roadmap, gates.confirm_breakdown, gates.confirm_plan, gates.issues_review) have no typed field, no Defaults row and no set/clear/toggle arm; a pre-1.16.0 config.json carrying them parses, shows each once as a read-only `Not modelled` row, and keeps every one with its original value through a toggle-then-save (test-asserted)"
    - "An unset `gates.execute_next_plan` / `gates.confirm_transition` / `gates.confirm_milestone_scope` row shows gsd-core's built-in default `true`; every `gates.*` help summary says the gate applies only in interactive mode; `workflow.drift_threshold` help says modified and deleted files count too"
    - "`DEFAULTS_OPTION_COUNT` is 135; `planner.stall_detection_enabled` and `workflow.ui_interaction_capture` keep `since` v1.15.0 with no not-yet-tagged caveat left in detail.rs"
    - "docs/GSD-CORE-SYNC.md's `## Modelled` section lists all 135 rows (Gates table without the six removed keys, new Observability table) with true counts, and its Pass-through table records the removed gates.* and safety.* keys; the header/baseline sections and the three GSD_CORE_SYNCED_* constants are untouched"
    - "`cargo build` and `rtk proxy cargo clippy --all-targets -- -D warnings` exit 0, `cargo fmt --check` exits 0, and `rtk proxy cargo test --no-fail-fast` has no failure other than envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against"
  artifacts:
    - path: "src/state_reader/config_json.rs"
      provides: "AuditConfig block (typed enabled + flatten extra), GsdConfig.audit, GatesConfig reduced to execute_next_plan/confirm_transition/confirm_milestone_scope + extra, parse/round-trip tests"
      contains: "pub struct AuditConfig"
    - path: "src/ui/screens/detail.rs"
      provides: "Two new Defaults rows with .since(\"v1.16.0\"), Observability category, set/clear/toggle arms, take(\"audit.\", …), gates builtin defaults, six rows/arms removed, RESYNCED_KEYS_1_16_0 + REMOVED_KEYS_1_16_0 tables, count pin 135, refreshed help"
      contains: "RESYNCED_KEYS_1_16_0"
    - path: "docs/GSD-CORE-SYNC.md"
      provides: "Updated key-inventory tables (Modelled + Pass-through) for gsd-core v1.16.0"
      contains: "audit.enabled"
  key_links:
    - from: "src/ui/screens/detail.rs build_defaults_entries"
      to: "src/state_reader/config_json.rs GsdConfig.audit / GatesConfig.confirm_milestone_scope"
      via: "opt_bool(...) + push(cat, \"<dotted key>\", …)"
      pattern: "\"audit\\.enabled\""
    - from: "src/ui/screens/detail.rs append_passthrough_entries::collect"
      to: "AuditConfig.extra and GatesConfig.extra (where the six removed keys now land)"
      via: "take(\"audit.\", &block.extra) / take(\"gates.\", &block.extra)"
      pattern: "take\\(\"audit\\.\""
    - from: "src/ui/screens/detail.rs the_sync_record_names_every_modelled_key_and_the_measured_baseline"
      to: "docs/GSD-CORE-SYNC.md ## Modelled"
      via: "CARGO_MANIFEST_DIR read; every row key present, every REMOVED_KEYS_1_16_0 key absent"
      pattern: "REMOVED_KEYS_1_16_0"
---

<objective>
Apply the gsd-core v1.16.0 config-key delta (tag v1.16.0 = 87e87d34b; previous config baseline ec81d0d10) to this build's config surface: retire the six `gates.*` keys upstream stopped reading, model `gates.confirm_milestone_scope` and `audit.enabled` as first-class Defaults rows with help and every mutation path, keep pre-1.16.0 files lossless, refresh the drift/gates help, settle the two v1.15.0 `since` values as measured, and true up the key-inventory tables in docs/GSD-CORE-SYNC.md.

Purpose: a v1.16.0 project's real keys must be visible and editable, the keys gsd-core now rejects must not be offered as live toggles, and nothing an older file carries may be lost on save.
Output: modified `src/state_reader/config_json.rs`, `src/ui/screens/detail.rs`, `docs/GSD-CORE-SYNC.md`, and `261012-jqb-SUMMARY.md` in this directory.
</objective>

<execution_context>
@~/.claude/gsd-core/workflows/execute-plan.md
@~/.claude/gsd-core/templates/summary.md
</execution_context>

<context>
@CLAUDE.md
@docs/GSD-CORE-SYNC.md
@src/state_reader/config_json.rs

src/ui/screens/detail.rs is ~29.9k lines — do NOT read it whole. Anchors at base commit 7303b41 (re-locate with `rtk proxy grep -n`):
- `struct ConfigHelp` ~10685 (fields summary/choices/since; `ConfigHelp::new`, `ConfigHelp::with_choices`, `.since(...)` builder)
- `fn build_defaults_entries` ~11005; drift rows ~11111-11125; planner.stall_detection_enabled row + its comment ~11167-11179; ui_interaction_capture row + its comment ~11344-11351; Gates category ~11825-11898; `append_passthrough_entries(&mut entries, config)` call ~11900
- `fn builtin_default_for_key` ~11917
- `fn append_passthrough_entries` / inner `collect` ~12007-12087 (the `take("<block>.", &block.extra)` walk)
- `fn set_config_value` ~12461 (gates bool arms ~12791-12846)
- `fn clear_config_value` ~13262 (gates arms ~13841-13896)
- `fn mutate_config_entry` ~14131 (gates toggle arms ~14377-14416; the INFERRED I-4 "first toggle writes true" comment ~14444)
- tests: `populated_gsd_config` ~14802 (gates block ~14831-14840), `DEFAULTS_OPTION_COUNT` ~14980-14994, summary budget test ~15414 (<=160 chars, >=4 novel words), `NESTED_CONFIG_BLOCKS` ~15532, `every_gsd_core_1_15_0_key_is_an_editable_row_writing_its_own_json_path` ~15707, `planner_stall_tuning_keys_stay_visible_and_survive_a_toggle` ~15771 (the model for Task 2's detail.rs test), `RESYNCED_KEYS` ~15833 / `RESYNCED_KEY_COUNT` ~15899 / `RESYNCED_KEYS_1_15_0` ~15908 / `PROMOTED_TOP_LEVEL_KEYS` ~15922, `the_sync_record_names_every_modelled_key_and_the_measured_baseline` ~15944, `the_resync_covers_every_key_the_drift_measurement_found` ~16044, `every_resynced_key_has_one_row_of_the_right_kind_with_a_measured_since` ~16214, `every_editable_resynced_key_sets_toggles_and_clears` ~16264.

src/state_reader/config_json.rs anchors: `GsdConfig` ~119-230 (typed fields, then `#[serde(flatten)] extra` last), `GatesConfig` ~243-264, `PlannerConfig` ~296-320 (the model for AuditConfig, including its INFERRED I-5 divergence note), tests module ~676: `planner_stall_detection_enabled_is_a_modelled_planner_key` ~1030, `ui_interaction_capture_is_a_modelled_workflow_key` ~1064, `the_resynced_non_workflow_keys_parse_and_round_trip` ~1210, `the_resynced_blocks_are_absent_from_a_default_config` ~1324.

<run_context>
Quick batch 261012-jqa, item 261012-jqb. Sibling item 261012-jqc runs IN PARALLEL in a separate worktree and edits ONLY `user_defaults_path` (~650-659) of src/state_reader/config_json.rs plus its tests near the top of the tests module (~676-690). Do not touch, reorder or reformat config_json.rs lines ~640-700; put every new config_json.rs test after `ui_interaction_capture_is_a_modelled_workflow_key`. Never push. The human is unavailable: decide from the facts below and copy every decision in <inferred_decisions> into the SUMMARY marked "inferred — for later audit". The upstream checkout ~/projects/node/gsd-core is READ-ONLY: `git -C … show/log/diff/tag` only, never checkout.

Tooling: rtk filters command output. Run cargo through `rtk proxy` whenever a count or a pass/fail line matters, and never pipe a test run through grep (memory: a piped grep downstream of rtk can fake counts). Use `--no-fail-fast` for any full test run.
</run_context>

<upstream_facts>
MEASURED by the planner on 2026-10-12 against ~/projects/node/gsd-core (re-verify cheaply; never re-derive from memory):

- `git rev-parse --short=9 v1.16.0^{commit}` = 87e87d34b ("chore: promote CHANGELOG for v1.16.0"). Tags v1.15.0 and v1.16.0 now exist; v1.15.0 is an ancestor of v1.16.0. The old baseline ec81d0d10 (release-1.15.0 branch tip) is NOT an ancestor of either tag, so compare trees with `git diff ec81d0d10 v1.16.0 -- <paths>`.
- docs/CONFIGURATION.md key cells (GSD-CORE-SYNC.md step 2 recipe) 260 -> 254: added `audit.enabled`, `gates.confirm_milestone_scope`; removed `gates.confirm_breakdown`, `gates.confirm_phases`, `gates.confirm_plan`, `gates.confirm_project`, `gates.confirm_roadmap`, `gates.issues_review`, `safety.always_confirm_destructive`, `safety.always_confirm_external_services`. capability.json changes in the range are installer metadata and version bumps only — no config keys.
- config-schema.manifest.json validKeys gained `gates.execute_next_plan`, `gates.confirm_transition`, `gates.confirm_milestone_scope`, `audit.enabled`. config-defaults.manifest.json gained `"gates": { "execute_next_plan": true, "confirm_transition": true, "confirm_milestone_scope": true }`; it has NO `audit` entry. No other default changed.
- ded8b6259 (#4974 / PR #5076), first tag v1.16.0: the six removed gates keys (and the two safety keys) were never read by any workflow, agent or src module; `config-set` keeps rejecting them (asserted upstream for gates.confirm_project). The three live gates now gate on `mode="interactive" AND="gates.<key> != false"`: execute-plan.md:80 (present plan identification, wait for confirmation), transition.md:145/504/639 (phase-complete transition prompt), complete-milestone.md:165 ("Ready to mark this milestone as shipped? (yes / wait / adjust scope)"). Before ded8b6259 the conditions used a non-existent `mode="custom"`, so `confirm_milestone_scope` had no effect anywhere. CONFIGURATION.md "Gate Settings": each gate only applies under mode interactive; yolo always auto-approves regardless; there is no third "custom" mode.
- af464656e (#4975 / PR #5023), first tag v1.16.0: made `audit.enabled` effective — the live dispatch seams read it, it is registered in the schema, `config-set` accepts booleans only, strict: only a real JSON `true` enables it (the string "true" leaves it off). `GSD_AUDIT=1` enables the trail independently and neither turns the other off. The trail is `.planning/.gsd-trace.jsonl`. Its Settings row (default `false`) sits under CONFIGURATION.md "## Observability"; that row first appeared in v1.16.0.
- Step-5 recipe (`log --reverse -S"audit.enabled" -- docs/CONFIGURATION.md src/`) returns 2d14eb887 (2026-05-24, "feat(observability): add DispatchLogger seam"), first tag v1.2.0 — but that commit only added prose ("Via config (`config.audit.enabled`)"); the key was absent from validKeys and unread at runtime until af464656e ("the documented key was inert").
- Step-5 recipe for `gates.confirm_milestone_scope` returns ded8b6259 -> v1.16.0.
- `git tag --contains 1e3e1f7cd --sort=v:refname | head -1` = v1.15.0 (planner.stall_detection_enabled); same for 88b5775dc = v1.15.0 (workflow.ui_interaction_capture). The `since` values already say v1.15.0; only the not-yet-tagged caveats are stale.
- `workflow.drift_threshold` row at v1.16.0 (changed by a2c43db27, #5134, first tag v1.16.0): "Minimum number of drift elements before the codebase-drift gate takes action: new structural elements (new directories, barrel exports, migrations, route modules) plus modified and deleted files inside directories the codebase map already describes." The `workflow.drift_action` row is unchanged upstream.
</upstream_facts>

<inferred_decisions>
- I-1: `audit.enabled` gets `since` "v1.16.0" — the release that made the key EFFECTIVE (af464656e, `git tag --contains` -> v1.16.0) and gave it a Settings row — not step 5's first-mention result v1.2.0 (2d14eb887 was prose for a key config-set rejected and nothing read). The help pane's `since` tells an operator from which release setting the key does something; v1.2.0 would advertise a key that was inert for fourteen releases. Deviation recorded beside the row in code and under the Observability table in the doc.
- I-2: `AuditConfig.enabled` is `Option<bool>`, so a hand-edited non-boolean (e.g. the string "true") makes the whole config.json a parse failure, where gsd-core treats it as off. Same accepted divergence as `PlannerConfig` INFERRED I-5: gsd-core's config-set writes booleans only, and a failed parse draws no rows and therefore cannot save over the file. Pinned by a test so a future lenient change is deliberate.
- I-3: New category "Observability" (upstream's section name) placed after Gates. `GsdConfig.audit` is the LAST typed field, directly before `extra`, so a file that already carried an `audit` block (until now emitted at the end via the top-level extra map) keeps it near the end on save.
- I-4: `builtin_default_for_key` gains `gates.execute_next_plan`, `gates.confirm_transition`, `gates.confirm_milestone_scope` => "true" (the v1.16.0 defaults manifest added that block). `audit.enabled` gets none — the manifest has no audit entry and this function mirrors the manifest; its off-by-default is stated in the help summary instead.
- I-5: Both new rows follow the existing toggle convention (`Some(!x.unwrap_or(false))`, the gates INFERRED I-4): from unset the first toggle writes `true`. For `audit.enabled` that is the opt-in direction.
- I-6: `RESYNCED_KEY_COUNT` stays 57 — the historical 1.14.0 measurement — and is asserted as `RESYNCED_KEYS.len() + REMOVED_KEYS_1_16_0.len()`; the 27 workflow.* count is unchanged.
- I-7: `workflow.drift_action`'s summary is reworded from "new structure" to "drift" together with drift_threshold, so the pair does not contradict each other (upstream's drift_action row did not change).
- I-8: In docs/GSD-CORE-SYNC.md only the `## Modelled` section (intro counts, tables, two short notes), the `## Pass-through` table, and the one sentence in the release-1.15.0 delta section that labels its two keys' `since` as not-yet-tagged are edited. The title/header, "Why VERSION and COMMIT disagree right now", "How to re-measure", "Next sync", and the three GSD_CORE_SYNCED_* constants / README compatibility note are left for the later rewrite task, even where they still describe 1.15.0 as untagged.
- I-9: `safety.*` was also removed upstream by ded8b6259. It was never modelled here (pass-through), so only the Pass-through table changes to say so.
- I-10: The six removed keys are not named in backticks anywhere inside `## Modelled` (they live in the Pass-through table), so the new reverse assertion in the sync-record test holds.
</inferred_decisions>
</context>

<tasks>

<task type="tracer" tdd="true">
  <name>Task 1 (tracer): model gates.confirm_milestone_scope and audit.enabled end-to-end — struct, row, help, set/clear/toggle, pass-through walk, sync record</name>
  <files>src/state_reader/config_json.rs, src/ui/screens/detail.rs, docs/GSD-CORE-SYNC.md</files>
  <precondition>`git -C ~/projects/node/gsd-core rev-parse --short=9 'v1.16.0^{commit}'` prints 87e87d34b (used only to re-verify upstream_facts; read-only).</precondition>
  <behavior>
    - config_json: parsing `{"gates":{"confirm_milestone_scope":false},"audit":{"enabled":true,"zz_probe":1}}` gives `gates.confirm_milestone_scope == Some(false)`, `audit.enabled == Some(true)`, `audit.extra` holding exactly `zz_probe`, and no `audit`/`gates` key in the top-level `extra`; serialize -> reparse -> serialize is a fixed point and keeps both values.
    - config_json: `GsdConfig::default()` serialises no `audit` key and `parse_gsd_config("{}")` has `audit == None`.
    - config_json: `parse_gsd_config(r#"{"audit":{"enabled":"true"}}"#)` returns `None` (I-2, pinned).
    - detail: each of `gates.confirm_milestone_scope` and `audit.enabled` has exactly one Bool row, not pass-through, `help.since == "v1.16.0"`, and the rendered help pane contains `SINCE_PREFIX` + `v1.16.0`; `audit.enabled` sits in category "Observability" with show_category true; `gates.confirm_milestone_scope` directly follows `gates.confirm_transition`.
    - detail: on a `{}` config, set "false" -> JSON pointer `/gates/confirm_milestone_scope` (resp. `/audit/enabled`) is `false`; toggle -> `true`; clear -> pointer absent.
    - detail: the per-block pass-through census shows `audit.zz_block_probe` as a read-only row.
    - detail: an unset `gates.confirm_milestone_scope` row carries builtin default "true"; an unset `audit.enabled` row carries none.
  </behavior>
  <action>
Write the config_json.rs and detail.rs tests listed in <behavior> first and watch them fail (or fail to compile), then implement.

src/state_reader/config_json.rs:
1. In `GatesConfig`, add `confirm_milestone_scope: Option<bool>` with the same `#[serde(default, skip_serializing_if = "Option::is_none")]` attributes as its siblings, with a doc comment: added to gsd-core's schema by ded8b6259 (#4974, v1.16.0), read by complete-milestone.md only under mode interactive. Do not remove any field in this task (Task 2 does).
2. After `PlannerConfig`, add `pub struct AuditConfig` deriving `Debug, Deserialize, Serialize, Clone, Default`, with `enabled: Option<bool>` (same serde attrs) and `#[serde(flatten)] pub extra: ExtraKeys`. Never add deny_unknown_fields (incompatible with flatten — see the `ExtraKeys` doc). Doc comment: af464656e (#4975, first tag v1.16.0) made the key effective; strict boolean upstream (only a real true enables; `GSD_AUDIT=1` enables independently); then the I-2 divergence paragraph modelled on `PlannerConfig`'s INFERRED I-5 note.
3. In `GsdConfig`, add `pub audit: Option<AuditConfig>` with `#[serde(default, skip_serializing_if = "Option::is_none")]` as the LAST typed field directly before the flatten `extra`, under a `// --- gsd-core v1.16.0 delta (quick task 261012-jqb) ---` comment (I-3).
4. Tests (placed after `ui_interaction_capture_is_a_modelled_workflow_key`, never near the top of the module): one new test `the_gsd_core_1_16_0_keys_are_modelled` covering the three config_json bullets of <behavior>; extend `the_resynced_blocks_are_absent_from_a_default_config` so its block list and its `is_none` checks include `audit`.

src/ui/screens/detail.rs:
5. Gates category: push `gates.confirm_milestone_scope` right after the `gates.confirm_transition` push, `first` false, `ConfigHelp::new(<summary>).since("v1.16.0")`. Summary (string literal, <=160 chars, >=4 words not in the key) must say it applies in interactive mode only, that it asks before /gsd-complete-milestone marks the milestone shipped (yes / wait / adjust scope), and that yolo never asks.
6. After the Gates category and before the `append_passthrough_entries` call, add an `// ── Observability ──` section: `let cat = "Observability";`, push `audit.enabled` from `config.audit` via `opt_bool`, `first` true, `ConfigHelp::new(<summary>).since("v1.16.0")`. Summary must name `.planning/.gsd-trace.jsonl`, say it is off by default, that only a real boolean true enables it, and that GSD_AUDIT=1 enables it independently (<=160 chars). Put the I-1 rationale in a comment beside the row, including both shas (af464656e effective, 2d14eb887 first mention) and both tags.
7. `builtin_default_for_key`: add the three keys `gates.execute_next_plan`, `gates.confirm_transition`, `gates.confirm_milestone_scope` to the `"true"` arm with a comment citing the v1.16.0 config-defaults manifest gates block (I-4). Do not add audit.enabled.
8. Mutation tables, each mirroring the neighbouring planner/gates arms: `set_config_value` bool arms for `gates.confirm_milestone_scope` (via `config.gates.get_or_insert_with(GatesConfig::default)`) and `audit.enabled` (via `config.audit.get_or_insert_with(AuditConfig::default)`); `clear_config_value` arms setting each to `None`; `mutate_config_entry` toggle arms using `Some(!x.unwrap_or(false))` (I-5). `AuditConfig` must be in scope where these functions import from config_json (they use a glob or explicit `use`; follow what is there).
9. `append_passthrough_entries::collect`: add `if let Some(block) = config.audit.as_ref() { take("audit.", &block.extra); }` under a short comment naming 261012-jqb, after the planner walk.
10. Tests: add "audit" to `NESTED_CONFIG_BLOCKS` (keep it sorted — it goes first); in `populated_gsd_config` add `"confirm_milestone_scope": true` to the gates block and an `"audit": { "enabled": false }` block (the fixture must stay free of unmodelled keys); add `const RESYNCED_KEYS_1_16_0: &[(&str, &str)] = &[("gates.confirm_milestone_scope", "bool"), ("audit.enabled", "bool")];` with a doc comment citing the measured key-cell diff ec81d0d10..v1.16.0 (+2 / -8); chain it into `every_resynced_key_has_one_row_of_the_right_kind_with_a_measured_since` and `every_editable_resynced_key_sets_toggles_and_clears`; generalise `every_gsd_core_1_15_0_key_is_an_editable_row_writing_its_own_json_path` to loop over `[(RESYNCED_KEYS_1_15_0, "v1.15.0"), (RESYNCED_KEYS_1_16_0, "v1.16.0")]`, asserting that table's own since and rendered marker (rename it to `every_gsd_core_delta_key_is_an_editable_row_writing_its_own_json_path`); in `the_resync_covers_every_key_the_drift_measurement_found` assert `RESYNCED_KEYS_1_16_0.len() == 2`; add a small test asserting the builtin-default bullet of <behavior> (build entries from `parse_gsd_config("{}")`). Set `DEFAULTS_OPTION_COUNT` to 141 for now (139 + 2; Task 2 lands the final 135 and writes the history line).

docs/GSD-CORE-SYNC.md (`## Modelled` section only):
11. Gates table: add a `gates.confirm_milestone_scope` row with since `v1.16.0` after `gates.confirm_transition`. After the Gates table add `### Observability` with a one-row table `audit.enabled` / `v1.16.0`, followed by a two-to-three sentence note stating the I-1 choice (effective-since, af464656e; step 5's first-mention answer would be v1.2.0 from 2d14eb887, a prose-only promise). The sync-record test reads this section from disk.
  </action>
  <verify>
    <automated>rtk proxy cargo test --lib --no-fail-fast -- state_reader::config_json ui::screens::detail</automated>
  </verify>
  <done>Both new keys parse, render as Bool rows with since v1.16.0, write their own JSON paths on set/toggle/clear, `audit.*` unmodelled keys are visible pass-through rows, the sync record lists both, and the filtered lib tests pass with DEFAULTS_OPTION_COUNT 141.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: retire the six gates keys gsd-core v1.16.0 removed — no rows, no arms, lossless round-trip</name>
  <files>src/state_reader/config_json.rs, src/ui/screens/detail.rs, docs/GSD-CORE-SYNC.md</files>
  <behavior>
    - config_json: a pre-1.16.0 file with all eight legacy gates keys (mixed true/false), `confirm_milestone_scope`, and a top-level `"safety": {"always_confirm_destructive": true, "always_confirm_external_services": false}` parses; `execute_next_plan`, `confirm_transition`, `confirm_milestone_scope` read typed; the six removed keys are in `gates.extra` with their exact values; `safety` is in the top-level `extra`; after serialize -> reparse every `/gates/<removed>` and `/safety/<key>` pointer equals the original value, and serialize is a fixed point.
    - detail: built from that file, no removed key has a typed row; each appears exactly once as a ReadOnly `Not modelled` row spelled `gates.<key>`; `set_config_value(…, "false")`, `clear_config_value` and `mutate_config_entry(…, &ConfigValueKind::ReadOnly)` all return false for every removed dotted key; toggling `gates.confirm_milestone_scope` then `serialize_gsd_config` keeps all six removed keys with original values.
    - detail: the first Gates row is `gates.execute_next_plan` with show_category true; the Gates category has exactly three rows.
    - detail: `the_sync_record_names_every_modelled_key_and_the_measured_baseline` also fails when any removed key appears backticked inside `## Modelled`.
    - `DEFAULTS_OPTION_COUNT == 135`; `RESYNCED_KEYS.len() + REMOVED_KEYS_1_16_0.len() == RESYNCED_KEY_COUNT (57)`.
  </behavior>
  <action>
Tests first (they will not compile or will fail until the removal lands), then the removal.

src/state_reader/config_json.rs:
1. Delete the six legacy fields from `GatesConfig` (confirm_breakdown, confirm_phases, confirm_plan, confirm_project, confirm_roadmap, issues_review). Give `GatesConfig` a doc comment: gsd-core >= 1.16.0 reads exactly three gates, each only under mode interactive (yolo auto-approves); the six it removed in ded8b6259 (#4974) are no longer typed and survive a save through `extra` like any other unmodelled key.
2. Repair `the_resynced_non_workflow_keys_parse_and_round_trip`: its gates fixture keeps only `confirm_transition` and `execute_next_plan`; switch its assertions on removed fields to those two (including the reparsed one); update the doc comment so the "30 non-workflow keys" wording says six gates.* keys were since retired at v1.16.0.
3. New test `the_gates_keys_gsd_core_1_16_0_removed_round_trip_through_extra`, placed right after Task 1's test, covering the config_json bullet of <behavior> (compare values via `serde_json::to_value(...).pointer(...)`, not via struct fields).

src/ui/screens/detail.rs:
4. Delete the six legacy `push` rows from the Gates category and their arms in `set_config_value`, `clear_config_value` and `mutate_config_entry`. Make the `gates.execute_next_plan` push carry `first` true. Rewrite the Gates category comment: three stop-and-ask points, each honoured only under mode interactive (yolo auto-approves all of them); the six keys gsd-core removed at v1.16.0 now show as read-only pass-through rows when a file still carries them.
5. Remove the six legacy entries from `RESYNCED_KEYS`. Add `const REMOVED_KEYS_1_16_0: &[&str]` holding the six dotted `gates.<key>` names, doc-commented with ded8b6259 / #4974 and why they stay out of every typed table. Keep `RESYNCED_KEY_COUNT` at 57 and change `the_resync_covers_every_key_the_drift_measurement_found` to assert `RESYNCED_KEYS.len() + REMOVED_KEYS_1_16_0.len() == RESYNCED_KEY_COUNT` and `REMOVED_KEYS_1_16_0.len() == 6` (I-6); update `RESYNCED_KEY_COUNT`'s doc comment accordingly.
6. `populated_gsd_config`: drop the six legacy keys from its gates block (leaving execute_next_plan, confirm_transition, confirm_milestone_scope) — otherwise they become pass-through rows and break the static count.
7. New test `the_gates_keys_gsd_core_1_16_0_removed_stay_visible_read_only_and_survive_a_toggle`, modelled on `planner_stall_tuning_keys_stay_visible_and_survive_a_toggle`, iterating `REMOVED_KEYS_1_16_0`, covering the detail bullets of <behavior>; add the Gates first-row / three-row assertion to it or to a sibling test.
8. Extend `the_sync_record_names_every_modelled_key_and_the_measured_baseline`: for each key in `REMOVED_KEYS_1_16_0`, assert the Modelled slice does not contain the backticked key, with a message saying the record still lists a key gsd-core removed.
9. `DEFAULTS_OPTION_COUNT` = 135. Its doc comment gains "139 -> 135 at quick task 261012-jqb (gsd-core v1.16.0: +gates.confirm_milestone_scope, +audit.enabled, -6 gates.* keys)" and its "140th option" wording becomes "136th"; update the matching "140th option" remark in `build_defaults_entries`' opening comment to "136th".

docs/GSD-CORE-SYNC.md:
10. `## Modelled` Gates table: delete the six legacy rows, leaving execute_next_plan, confirm_transition, confirm_milestone_scope. Under it add one sentence saying gsd-core v1.16.0 (ded8b6259, #4974) retired six never-read gates and that they are listed under Pass-through — do NOT backtick-name them inside `## Modelled` (I-10).
11. `## Pass-through` table: remove `safety.*` from the `refactor.*`, `safety.*`, `security.injection_blocking` row, and add a row whose Family cell lists the six removed `gates.*` keys and `safety.always_confirm_destructive`, `safety.always_confirm_external_services`, and whose reason says: removed upstream at v1.16.0 (ded8b6259, #4974) — no longer read and rejected by gsd-core's config-set; never get a typed row; a pre-1.16.0 file keeps them (GatesConfig.extra / the top-level extra map) and shows them read-only (I-9).
  </action>
  <verify>
    <automated>rtk proxy cargo test --lib --no-fail-fast -- state_reader::config_json ui::screens::detail</automated>
    <automated>rtk proxy grep -cE 'pub (confirm_project|confirm_phases|confirm_roadmap|confirm_breakdown|confirm_plan|issues_review):' src/state_reader/config_json.rs  # prints 0</automated>
    <automated>rtk proxy grep -cE '"gates\.(confirm_project|confirm_phases|confirm_roadmap|confirm_breakdown|confirm_plan|issues_review)" =>' src/ui/screens/detail.rs  # prints 0</automated>
  </verify>
  <done>The six legacy gates keys have no field, row or mutation arm; a legacy file round-trips them losslessly and shows them read-only; Gates has three rows led by execute_next_plan; DEFAULTS_OPTION_COUNT is 135; the sync record no longer lists them under Modelled and records them (with safety.*) under Pass-through; filtered lib tests pass and both greps print 0.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 3: refresh drift and gates help, settle the two v1.15.0 since comments, true up the sync record counts, full gate</name>
  <files>src/ui/screens/detail.rs, docs/GSD-CORE-SYNC.md</files>
  <behavior>
    - Every Defaults row whose key starts with `gates.` has a summary containing "nteractive" (the interactive-mode-only note).
    - The `workflow.drift_threshold` summary contains "modified" and "deleted"; the `workflow.drift_action` summary no longer says "new structure".
    - All summaries still pass `every_summary_is_within_the_pane_budget_and_is_not_a_restatement_of_the_key` (<=160 chars, >=4 novel words).
  </behavior>
  <action>
src/ui/screens/detail.rs:
1. Add a test `the_gsd_core_1_16_0_help_refresh_is_in_place` asserting the <behavior> bullets over `all_config_entries()`; see it fail first.
2. `gates.execute_next_plan` and `gates.confirm_transition` summaries: rewrite to open with "Interactive mode only:" and end noting yolo never stops/asks, keeping what each does (stops between two plans of a phase / stops at a phase boundary). Make `gates.confirm_milestone_scope` (Task 1) match the same opening if it does not already.
3. `workflow.drift_threshold` summary: per a2c43db27 — how many drift elements trip the codebase-drift gate: new directories, migrations or route modules plus modified or deleted files in directories the codebase map already describes; default 3. Keep it within 160 characters. `workflow.drift_action` summary: same meaning, but "has drifted further than that threshold allows" in place of the "new structure" wording (I-7). Leave both `since` values as they are.
4. Replace the two stale caveat comments — above the `planner.stall_detection_enabled` row (it currently says no v1.15.0 tag exists and that `tag --contains` on 1e3e1f7cd is empty) and above the `workflow.ui_interaction_capture` row (it says the since is uncertain for the same reason) — with measured statements: `git tag --contains 1e3e1f7cd` / `88b5775dc` resolves to v1.15.0, re-measured at quick task 261012-jqb. Keep the #4570 / #4223 references and the INFERRED I-2 placement note. Do not change either `.since("v1.15.0")` value.

docs/GSD-CORE-SYNC.md:
5. `## Modelled` intro paragraph: 135 rows — 51 carry a since measured at the 1.14.0 re-sync (260916-vqw measured 57; six retired at 261012-jqb), 2 carry `v1.15.0` (added by 260926-gtk, measured against the v1.15.0 tag at 261012-jqb), 7 measured by 260926-jnf, 2 carry `v1.16.0` (261012-jqb), 73 carry none. Next paragraph: DEFAULTS_OPTION_COUNT pins 135 (132 -> 139 at 260926-jnf, 139 -> 135 at 261012-jqb); `RESYNCED_KEYS` / `RESYNCED_KEYS_1_15_0` / `RESYNCED_KEYS_1_16_0` / `PROMOTED_TOP_LEVEL_KEYS` pin the four key lists and `REMOVED_KEYS_1_16_0` the six retired ones. Remove the "not yet tagged" qualifier from this section entirely.
6. In "The release-1.15.0 delta" section, rewrite only the sentence introducing its two-key table (the one saying `git tag --contains` is empty for both introducing commits) to say both resolved to v1.15.0 once the tag existed, re-measured at 261012-jqb. Leave the title/header, "Why VERSION and COMMIT disagree right now", "How to re-measure", "Next sync" and the GSD_CORE_SYNCED_* constants untouched (I-8).

Then run the full gate: `cargo fmt` (then `cargo fmt --check` must exit 0), `cargo build`, `rtk proxy cargo clippy --all-targets -- -D warnings`, and `rtk proxy cargo test --no-fail-fast`. The one expected local failure is `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against` (local git differs from the pinned 2.55.0); any other failure is a regression to fix. Confirm `git diff --quiet -- Cargo.toml Cargo.lock` (no dependency change). Record the measured pass/fail/ignored totals in the SUMMARY.
  </action>
  <verify>
    <automated>rtk proxy cargo test --lib --no-fail-fast -- ui::screens::detail state_reader::config_json</automated>
    <automated>rtk proxy grep -ci 'provisional' src/ui/screens/detail.rs  # prints 0</automated>
    <automated>awk '/^## Modelled/,/^## Pass-through/' docs/GSD-CORE-SYNC.md | rtk proxy grep -ci 'provisional'  # prints 0</automated>
    <automated>cargo fmt --check && cargo build && rtk proxy cargo clippy --all-targets -- -D warnings</automated>
    <automated>rtk proxy cargo test --no-fail-fast  # exactly one failure: envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against</automated>
  </verify>
  <done>Gates help notes interactive-only, drift help counts modified and deleted files, no not-yet-tagged caveat remains in detail.rs or in the Modelled section, the sync record's counts read 135 / 51 / 2 / 7 / 2 / 73, fmt/build/clippy are clean, and the full suite's only failure is the known git-version witness.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| project `.planning/config.json` -> TUI | Attacker-influenced file content is parsed, displayed and written back by the Defaults tab |
| TUI save path -> project `.planning/config.json` | `serialize_gsd_config` overwrites the operator's file; anything not captured is deleted |

## STRIDE Threat Register

ASVS level 1, block_on high.

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-jqb-01 | Tampering (integrity / data loss) | `GatesConfig` field removal + Defaults save path | high | mitigate | The six removed keys fall into `GatesConfig`'s `#[serde(flatten)] extra` and `safety` into the top-level `extra`; Task 2's config_json test asserts pointer-equal values after serialize/reparse, and its detail.rs test asserts they survive a toggle-then-save |
| T-jqb-02 | Tampering | set/clear/toggle arms for `audit.enabled` and `gates.confirm_milestone_scope` | medium | mitigate | Arms write only a real JSON boolean (`parse_bool` accepts exactly "true"/"false"; toggles write `Some(bool)`); Task 1's JSON-pointer test asserts the exact path and value written |
| T-jqb-03 | Denial of service (Defaults tab availability) | `AuditConfig.enabled: Option<bool>` on a hand-edited non-boolean | low | accept | I-2: whole-file parse failure draws no rows and cannot save over the file; gsd-core's config-set writes booleans only; same accepted divergence as PlannerConfig I-5; pinned by a test |
| T-jqb-04 | Information disclosure / terminal injection | new pass-through rows (`gates.<removed>`, `audit.<unknown>`) render project-supplied keys and values | low | mitigate | Rows go through the existing `append_passthrough_entries` route (`display_config_value` masking, `shown()` escaping); all new `ConfigHelp` text is `&'static str` literals so the help pane stays outside the escaping rule by construction (T-S0N-01) |
| T-jqb-05 | Repudiation (record integrity) | docs/GSD-CORE-SYNC.md vs the Defaults tab | medium | mitigate | The sync-record test checks every row key is listed under Modelled and (new) every removed key is not |
| T-jqb-SC | Tampering | npm/pip/cargo installs | high | mitigate | No dependency is added: Cargo.toml and Cargo.lock are not in files_modified and Task 3 asserts `git diff --quiet -- Cargo.toml Cargo.lock` |
</threat_model>

<verification>
- `rtk proxy cargo test --lib --no-fail-fast -- state_reader::config_json ui::screens::detail` passes after each task.
- `cargo fmt --check`, `cargo build`, `rtk proxy cargo clippy --all-targets -- -D warnings` exit 0.
- `rtk proxy cargo test --no-fail-fast` (never piped) reports exactly one failure, the git-version witness in src/envelope/policy.rs.
- `git diff --stat` shows only src/state_reader/config_json.rs, src/ui/screens/detail.rs, docs/GSD-CORE-SYNC.md (plus the SUMMARY); config_json.rs lines ~640-700 (user_defaults_path and the top of the tests module) are unchanged.
</verification>

<success_criteria>
- Defaults tab: 135 static rows; `gates.confirm_milestone_scope` and `audit.enabled` editable with since v1.16.0; the six legacy gates keys gone from rows and mutation tables but lossless and visible read-only when present.
- Help: gates rows say interactive-only; drift_threshold counts modified and deleted files; the two v1.15.0 `since` comments state the measured tag.
- docs/GSD-CORE-SYNC.md key-inventory tables match the tab; header/baseline sections untouched.
- SUMMARY lists I-1..I-10 marked "inferred — for later audit" and the measured test totals.
</success_criteria>

<output>
Create `.planning/quick/261012-jqb-config-key-delta-for-gsd-core-v1-16-0-tag-v1-16-0-87e87d34b/261012-jqb-SUMMARY.md` when done.
</output>
