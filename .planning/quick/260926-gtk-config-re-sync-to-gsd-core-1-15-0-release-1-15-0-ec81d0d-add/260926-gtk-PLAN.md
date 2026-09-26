---
phase: quick-260926-gtk
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/state_reader/config_json.rs
  - src/ui/screens/detail.rs
  - docs/GSD-CORE-SYNC.md
autonomous: true
requirements: [QUICK-260926-gtk]

must_haves:
  truths:
    - "The Defaults tab shows `planner.stall_detection_enabled` (category Planning, directly after `planning.pr_strict`) and `workflow.ui_interaction_capture` (category Execution, directly after `workflow.live_dom_uat`) as editable Bool rows whose help pane draws `since gsd-core v1.15.0`"
    - "Set, toggle and clear on each new row write exactly the JSON path `planner.stall_detection_enabled` / `workflow.ui_interaction_capture` in the saved config.json (asserted through serde_json::to_value + JSON pointer, not through the struct)"
    - "A project's `planner.stall_detect_interval_minutes` and `planner.stall_threshold_minutes` stay visible as read-only `Not modelled` rows spelled with the `planner.` prefix, and survive a toggle-then-save byte-for-byte"
    - "`DEFAULTS_OPTION_COUNT` is 132 (130 + 2) and the pass-through census `NESTED_CONFIG_BLOCKS` includes `planner`"
    - "`GSD_CORE_SYNCED_COMMIT` is `v1.14.0-111-gec81d0d10` (git describe of release-1.15.0 tip ec81d0d) while `GSD_CORE_SYNCED_VERSION` stays `1.14.0`, and both config_json.rs doc comments and docs/GSD-CORE-SYNC.md say why (oracle is npm-installed; 1.15.0 is not on npm)"
    - "docs/GSD-CORE-SYNC.md records the measured 1.15.0 delta (+2 keys, 0 removed, 0 default changes), ref-based re-measure commands, and a `## Modelled` section listing all 132 rows"
    - "`rtk proxy cargo test --no-fail-fast` has no failure other than the src/envelope/policy.rs git-version witness, and `rtk proxy cargo clippy --all-targets -- -D warnings` exits 0"
  artifacts:
    - path: "src/state_reader/config_json.rs"
      provides: "PlannerConfig block (typed stall_detection_enabled + flatten extra), GsdConfig.planner, WorkflowConfig.ui_interaction_capture, moved GSD_CORE_SYNCED_COMMIT, parse/round-trip tests"
      contains: "pub struct PlannerConfig"
    - path: "src/ui/screens/detail.rs"
      provides: "Two Defaults rows with ConfigHelp .since(\"v1.15.0\"), set/clear/toggle arms, `take(\"planner.\", …)` in the pass-through walk, count pin 132, RESYNCED_KEYS_1_15_0 table and tests"
      contains: "RESYNCED_KEYS_1_15_0"
    - path: "docs/GSD-CORE-SYNC.md"
      provides: "Rewritten sync record for release-1.15.0 / oracle pin 1.14.0"
      contains: "v1.14.0-111-gec81d0d10"
  key_links:
    - from: "src/ui/screens/detail.rs build_defaults_entries"
      to: "src/state_reader/config_json.rs GsdConfig.planner / WorkflowConfig.ui_interaction_capture"
      via: "bool_l(...) layered accessor + push(cat, \"<dotted key>\", …)"
      pattern: "push\\(cat, \"planner.stall_detection_enabled\""
    - from: "src/ui/screens/detail.rs append_passthrough_entries::collect"
      to: "PlannerConfig.extra"
      via: "take(\"planner.\", &block.extra)"
      pattern: "take\\(\"planner\\.\""
    - from: "src/ui/screens/detail.rs the_sync_record_names_every_modelled_key_and_the_measured_baseline"
      to: "docs/GSD-CORE-SYNC.md"
      via: "include-by-path read through CARGO_MANIFEST_DIR; doc must contain both constants and every row key"
      pattern: "GSD_CORE_SYNCED_COMMIT"
    - from: "scripts/install-conformance-oracle.sh"
      to: "src/state_reader/config_json.rs GSD_CORE_SYNCED_VERSION"
      via: "grep of the `pub const GSD_CORE_SYNCED_VERSION` declaration -> npm install @opengsd/gsd-core@<version>"
      pattern: "pub const GSD_CORE_SYNCED_VERSION: &str = \"1.14.0\";"
---

<objective>
Re-sync this build's gsd-core config surface to gsd-core `release-1.15.0` (tip ec81d0d, package.json 1.15.0, untagged, not on npm): model the two keys that release added as typed Defaults-tab rows, move the machine-readable sync baseline to that commit while keeping the npm-installable oracle version pinned at 1.14.0, and rewrite the human-readable sync record.

Purpose: every gsd-core key a 1.15.0 project can carry must be visible and editable (or deliberately pass-through) in the Defaults tab, and the next syncer must start from a measured baseline rather than a stale one.
Output: modified `src/state_reader/config_json.rs`, `src/ui/screens/detail.rs`, rewritten `docs/GSD-CORE-SYNC.md`, and `260926-gtk-SUMMARY.md`.
</objective>

<execution_context>
@~/.claude/gsd-core/workflows/execute-plan.md
@~/.claude/gsd-core/templates/summary.md
</execution_context>

<context>
@CLAUDE.md
@docs/GSD-CORE-SYNC.md
@src/state_reader/config_json.rs

Run context: quick batch 260926-gtj, item A. MANDATED sequential on the primary checkout, no worktrees, never push, do not commit outside the executor's normal atomic-commit flow. The human is unavailable: decide from the facts below and record every inferred decision with an `INFERRED:` marker in the SUMMARY.

<upstream_facts>
MEASURED by the planner on 2026-09-26 (re-verify cheaply, never re-derive from memory). The upstream checkout ~/projects/node/gsd-core is READ-ONLY: use `git -C … show/diff/describe/log <ref>` only, never check out or switch branches there. Its working tree HEAD is `next` at `v1.14.0-117-gebe51d68c` with package.json 1.14.0 — it is NOT the sync target, so any command reading the working tree (plain `cat $CORE/docs/CONFIGURATION.md`, `node -p require(package.json)`) measures the wrong ref.

- Target ref: `upstream/release-1.15.0` = ec81d0d10f545dd5aea2cc893863a542bc49029d ("chore: bump version to 1.15.0 for release"); `git -C ~/projects/node/gsd-core describe --tags --always ec81d0d` prints `v1.14.0-111-gec81d0d10`; `git show ec81d0d:package.json` version is `1.15.0`; no v1.15.0 tag exists.
- Previous baseline `v1.14.0-52-g651511d1e` (651511d1e) is an ancestor of ec81d0d.
- npm: `npm view @opengsd/gsd-core versions --json` ends at `1.14.0` — 1.15.0 is unpublished.
- Key-set delta, docs/CONFIGURATION.md key-cell grep (the recipe in GSD-CORE-SYNC.md step 2) at 651511d1e vs ec81d0d: 258 -> 260 lines; added exactly `planner.stall_detection_enabled` and `workflow.ui_interaction_capture`; removed none.
- gsd-core/bin/shared/config-schema.manifest.json: 121 -> 122 keys, only `planner.stall_detection_enabled` added. config-defaults.manifest.json: only a new top-level `"planner": { "stall_detection_enabled": true }` block. No existing key's default changed.
- `planner.stall_detection_enabled`: boolean, default true. Introduced by 1e3e1f7cd (#4570 / PR #4585, describe v1.14.0-98). src/config-loader.cts adds resolvePlannerStallDetectionEnabled: only a real JSON boolean overrides; strings/numbers/null fail safe to true. `config-set` (src/config.cts:854) coerces the string "false" to boolean false, so non-boolean values only arise from hand edits. CONFIGURATION.md warns that `false` gives up bounded recovery if the runtime loses the completion handoff.
- `workflow.ui_interaction_capture`: boolean, default false, declared in capabilities/ui/capability.json:43 (not in the schema manifest — capability-registered). Introduced by 88b5775dc (#4223 / PR #4477, describe v1.14.0-80). Lets gsd-ui-auditor add post-interaction captures (hover, focus ring, open menus, filled forms) via the chrome-devtools CLI run from Bash; needs an installed Chrome (CHROME_BIN overrides); read by /gsd-ui-review.
- `git tag --contains` is EMPTY for both introducing commits (1.15.0 untagged); both commits are reachable only from release-1.15.0 branches.
- Other config-adjacent diffs in the range, NOT new keys: config-loader.cts runtime-identity fill (#4717, fills the existing `runtime` key from GSD_RUNTIME env / install marker — item D's concern, not this item's); every capabilities/*/capability.json version bump 1.14.0 -> 1.15.0; capabilities/opencode drops the installer flag `skipCodexSkillsManifest` (installer metadata, not a config key); CONFIGURATION.md prose edits to `phase_id_convention`, `worktree.baseRef`, `resolve_model_ids` rows and a new graphify-CLI prose section (no key).
- Step-3 gap output (core key cells at ec81d0d minus this build's `push(cat, "…")` keys): 171 lines today, expected 169 lines / 113 dotted after this item.
- This repo today: 130 `push(cat, "…")` call sites; `DEFAULTS_OPTION_COUNT = 130` (detail.rs:11662); `cargo clippy --all-targets -- -D warnings` exits 0 on the untouched tree.
</upstream_facts>

<code_anchors>
Line numbers are pre-edit; locate by the quoted identifier if they drift.
- config_json.rs:3-28 — `GSD_CORE_SYNCED_VERSION` / `GSD_CORE_SYNCED_COMMIT` and their doc comments ("synced 52 commits past `v1.14.0`").
- config_json.rs:45-120 — `GsdConfig`; the 1.14.0-sync blocks `features`, `gates`, `planning`, `plan_review` sit at 100-116, each `#[serde(default, skip_serializing_if = "Option::is_none")]`.
- config_json.rs:175-184 — `PlanReviewConfig`, the shape to copy (Option fields + `#[serde(flatten)] pub extra: ExtraKeys`).
- config_json.rs:295-431 — `WorkflowConfig`; the 1.14.0 block ends at `worktree_skip_hooks` (426-427).
- config_json.rs:840-850 — `compact_content_is_a_modelled_workflow_key`, the per-key test shape to copy.
- config_json.rs:1073-1088 — `the_resynced_blocks_are_absent_from_a_default_config`.
- config_json.rs:1092-1112 — `the_gsd_core_sync_baseline_is_recorded` (asserts COMMIT starts with `v` and CONTAINS VERSION — `v1.14.0-111-gec81d0d10` satisfies it with VERSION `1.14.0`).
- detail.rs:9409-9420 — `ConfigHelp::since`; 9465 `SINCE_PREFIX`.
- detail.rs:9655 — `build_defaults_entries`; Planning `planning.pr_strict` push at 9830-9833 (then `let prv = config.plan_review…`); Execution `workflow.live_dom_uat` push at 9993-9996.
- detail.rs:10519-10582 — `append_passthrough_entries::collect`, one `take("<block>.", &block.extra)` per `GsdConfig` block.
- detail.rs:10829 `set_config_value` (1.14.0 bool arms ~10886-10904), 11025 `clear_config_value` (~11116-11139), 11178 `mutate_config_entry` Bool arms (~11236-11254). All three `use crate::state_reader::config_json::*;`.
- detail.rs:11482-11643 — `populated_gsd_config` fixture (MUST stay free of unmodelled keys — see the doc comment at 11657-11661).
- detail.rs:11651-11662 — `DEFAULTS_OPTION_COUNT` and its doc comment.
- detail.rs:11811 `render_defaults_to_text(width, height, selected)`, 11868 `squeeze_ws`.
- detail.rs:12051-12067 — `NESTED_CONFIG_BLOCKS`; 12094 the census test that makes a missing `take` a failure.
- detail.rs:12164-12211 — `compact_content_is_an_editable_row_naming_its_gsd_core_version`, the dedicated-row test shape.
- detail.rs:12220-12286 — `RESYNCED_KEYS` (57) / `RESYNCED_KEY_COUNT`; 12300 `the_sync_record_names_every_modelled_key_and_the_measured_baseline` (splits the doc at `\n## Modelled` and `\n## Pass-through`); 12379 and 12425 the two per-key loops over `RESYNCED_KEYS`.
- scripts/install-conformance-oracle.sh:55-56 and .github/workflows/release.yml:70 consume `GSD_CORE_SYNCED_VERSION` — do not edit either.
</code_anchors>
</context>

<tasks>

<task type="tracer" tdd="true">
  <name>Task 1 (tracer): `planner.stall_detection_enabled` end to end — typed block, Defaults row, all mutation arms, pass-through walk, sync-record row</name>
  <files>src/state_reader/config_json.rs, src/ui/screens/detail.rs, docs/GSD-CORE-SYNC.md</files>
  <behavior>
    - config_json.rs `planner_stall_detection_enabled_is_a_modelled_planner_key`: parsing `{"planner":{"stall_detection_enabled":false,"stall_detect_interval_minutes":5,"stall_threshold_minutes":10}}` gives `config.planner.stall_detection_enabled == Some(false)`; `planner.extra` holds exactly the keys `stall_detect_interval_minutes` and `stall_threshold_minutes` (not `stall_detection_enabled`); `config.extra` has no `planner` key; serialize -> reparse -> serialize is byte-identical.
    - config_json.rs `the_resynced_blocks_are_absent_from_a_default_config` additionally covers `"planner"` (absent from a default serialisation, `parse_gsd_config("{}").planner.is_none()`).
    - detail.rs `every_gsd_core_1_15_0_key_is_an_editable_row_writing_its_own_json_path`: for each `(key, "bool")` in `RESYNCED_KEYS_1_15_0` — exactly one row in `all_config_entries()`, kind Bool, category not `PASSTHROUGH_CATEGORY`, `help.since == "v1.15.0"`; on a config parsed from `{}`: `set_config_value(key, "false")` returns true and `serde_json::to_value(&config).pointer("/" + key with '.' replaced by '/')` is `false`; `mutate_config_entry(key, &ConfigValueKind::Bool)` makes it `true`; `clear_config_value(key)` makes the pointer resolve to nothing; and the help pane rendered by `render_defaults_to_text(120, 40, idx)` contains `squeeze_ws(SINCE_PREFIX + "v1.15.0")`.
    - detail.rs `planner_stall_tuning_keys_stay_visible_and_survive_a_toggle`: building entries from the three-key planner config above yields pass-through rows `planner.stall_detect_interval_minutes` and `planner.stall_threshold_minutes` (ReadOnly), no row keyed `planner`, the typed row `planner.stall_detection_enabled` with value `"false"`; after `mutate_config_entry` on the typed key, the serialised config still carries both tuning keys with values 5 and 10 and the typed key is `true`.
    - Existing census `an_unmodelled_key_in_every_nested_block_becomes_a_visible_pass_through_row` goes red when `"planner"` is added to `NESTED_CONFIG_BLOCKS` before `collect()` gains its `take` (fail-first proof), green after.
    - `every_config_entry_carries_a_non_empty_summary` passes with `DEFAULTS_OPTION_COUNT = 131` at the end of this task.
  </behavior>
  <action>
    RED first: add the tests named in `<behavior>` plus the `"planner"` entry in `NESTED_CONFIG_BLOCKS` (keep it sorted: between `plan_review` and `planning`), and a new `const RESYNCED_KEYS_1_15_0: &[(&str, &str)]` holding only `("planner.stall_detection_enabled", "bool")` for now, placed right after `RESYNCED_KEY_COUNT` with a doc comment saying it is the gsd-core release-1.15.0 re-sync (quick 260926-gtk), MEASURED from the key-cell diff 651511d1e..ec81d0d, and kept separate from `RESYNCED_KEYS` because that table's 57 / 27 counts pin the 1.14.0 measurement (INFERRED I-7). Also make the two existing per-key loops (`every_resynced_key_has_one_row_of_the_right_kind_with_a_measured_since`, `every_editable_resynced_key_sets_toggles_and_clears`) iterate `RESYNCED_KEYS.iter().chain(RESYNCED_KEYS_1_15_0)`. Run the verify command and confirm the new detail.rs tests and the census fail (config_json.rs tests fail to compile until the field exists — acceptable RED for a new struct field; record which).

    GREEN, config_json.rs: add `pub struct PlannerConfig` next to `PlanReviewConfig` with the same derives, one field `stall_detection_enabled: Option<bool>` under `#[serde(default, skip_serializing_if = "Option::is_none")]`, and `#[serde(flatten)] pub extra: ExtraKeys` with the `/// See [`ExtraKeys`].` doc. Its doc comment states: gsd-core 1.15.0 added this block (#4570, default true); only `stall_detection_enabled` is typed — `stall_detect_interval_minutes` and `stall_threshold_minutes` stay in `extra` as pass-through per docs/GSD-CORE-SYNC.md's runtime-tuning-knobs decision (INFERRED I-3); gsd-core fails a non-boolean value safe to `true`, whereas a typed `Option<bool>` makes such a hand edit a whole-file parse failure exactly like every other typed key here (INFERRED I-5, accepted — `config-set` always writes a real boolean). Add `pub planner: Option<PlannerConfig>` to `GsdConfig` after `plan_review`, under a `// --- gsd-core re-sync at 1.15.0 (quick task 260926-gtk) ---` comment, with `#[serde(default, skip_serializing_if = "Option::is_none")]`.

    GREEN, detail.rs: (a) in `collect()` add `take("planner.", &block.extra)` for `config.planner` after the `plan_review` take, extending that comment block to say the 1.15.0 re-sync added `planner`. (b) In `build_defaults_entries`, directly after the `planning.pr_strict` push and before `let prv = …`, add a `// gsd-core 1.15.0 re-sync (quick task 260926-gtk)` comment, bind `let ppn = config.planner.as_ref(); let dpn = defaults.and_then(|d: &GsdConfig| d.planner.as_ref());`, and push `"planner.stall_detection_enabled"` via `bool_l` with `ConfigHelp::with_choices("Watches the planner, plan-checker and revision agents for stalls and offers accept/retry/stop when one goes quiet.", &[("true", "bounded stall checks with a recovery prompt, the default"), ("false", "no watchdog; a lost completion handoff needs a manual interrupt")]).since("v1.15.0")`. The since string is INFERRED I-1: no v1.15.0 tag exists (`tag --contains` empty); introducing commit 1e3e1f7cd is reachable only from release-1.15.0 whose package.json is 1.15.0 — say so in the comment. (c) Add the three mutation arms beside the 1.14.0 nested-block arms, using `config.planner.get_or_insert_with(PlannerConfig::default)`: set -> `Some(b)`; clear -> `None`; toggle -> `Some(!p.stall_detection_enabled.unwrap_or(false))` (the file-wide convention; INFERRED I-4 — from unset the first Enter writes `true`, gsd-core's own default, same as every `gates.*` row). (d) Add `"planner": { "stall_detection_enabled": true }` to `populated_gsd_config` (no other planner key — the fixture must stay free of unmodelled keys). (e) `DEFAULTS_OPTION_COUNT` 130 -> 131 for now; Task 2 lands the final 132.

    docs/GSD-CORE-SYNC.md: insert a `| \`planner.stall_detection_enabled\` | v1.15.0 |` row into the `### Planning` table directly after the `planning.pr_strict` row (the table is in display order), so `the_sync_record_names_every_modelled_key_and_the_measured_baseline` stays green. The full rewrite is Task 3.
  </action>
  <verify>
    <automated>rtk proxy cargo test --lib -- planner_stall gsd_core_1_15_0 every_config_entry_carries nested_block the_sync_record resynced the_resync_covers the_resynced_blocks_are_absent</automated>
  </verify>
  <done>All filtered tests pass; the census test was observed red before the `take("planner.", …)` line and green after; a config carrying the three planner keys shows one editable Planning row plus two `planner.`-prefixed Not-modelled rows and round-trips losslessly.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: `workflow.ui_interaction_capture` row, arms, fixture — count pinned at 132</name>
  <files>src/state_reader/config_json.rs, src/ui/screens/detail.rs, docs/GSD-CORE-SYNC.md</files>
  <behavior>
    - config_json.rs `ui_interaction_capture_is_a_modelled_workflow_key`: `{"workflow":{"ui_interaction_capture":true}}` parses to `Some(true)`, the key is absent from `wf.extra`, it survives serialize -> reparse, and a workflow block without it serialises with no `ui_interaction_capture` substring.
    - `RESYNCED_KEYS_1_15_0` gains `("workflow.ui_interaction_capture", "bool")`, so Task 1's `every_gsd_core_1_15_0_key_is_an_editable_row_writing_its_own_json_path` and the two chained loops now cover it (JSON pointer `/workflow/ui_interaction_capture`).
    - `every_config_entry_carries_a_non_empty_summary` passes with `DEFAULTS_OPTION_COUNT = 132`; `every_choice_bearing_entry_documents_exactly_its_dropdown_options` still counts 12 Enum rows (both new rows are Bool).
  </behavior>
  <action>
    RED: add the config_json.rs test and the second `RESYNCED_KEYS_1_15_0` entry, bump `DEFAULTS_OPTION_COUNT` to 132, run the verify command and confirm failure (the count pin and the detail.rs 1.15.0 test fail; the config_json.rs test fails to compile until the field exists).

    GREEN, config_json.rs: in `WorkflowConfig`, after `worktree_skip_hooks`, add a `// --- gsd-core re-sync at 1.15.0 (quick task 260926-gtk) ---` comment and `pub ui_interaction_capture: Option<bool>` with `#[serde(default, skip_serializing_if = "Option::is_none")]`.

    GREEN, detail.rs: directly after the `workflow.live_dom_uat` push in the Execution category, push `"workflow.ui_interaction_capture"` via `bool_l(pwf.and_then(|w| w.ui_interaction_capture), dwf.and_then(|w| w.ui_interaction_capture))` with `ConfigHelp::new("Lets the UI auditor add hover, focus, open-menu and filled-form screenshots via the chrome-devtools CLI; needs Chrome.").since("v1.15.0")` and a comment citing introducing commit 88b5775dc (#4223) and INFERRED I-1/I-2 (placement beside the other browser-driven gate). Add set / clear / toggle arms beside the `workflow.live_dom_uat` arms, same shape as those. Add `"ui_interaction_capture": false` to the `workflow` object of `populated_gsd_config`. Rewrite the `DEFAULTS_OPTION_COUNT` doc comment to record both measurements: 73 -> 130 at the 1.14.0 re-sync (260916-vqw) and 130 -> 132 at the release-1.15.0 re-sync (260926-gtk).

    docs/GSD-CORE-SYNC.md: insert `| \`workflow.ui_interaction_capture\` | v1.15.0 |` into the `### Execution` table directly after the `workflow.live_dom_uat` row.
  </action>
  <verify>
    <automated>rtk proxy cargo test --lib -- ui_interaction_capture gsd_core_1_15_0 every_config_entry_carries every_choice_bearing the_sync_record resynced planner_stall</automated>
  </verify>
  <done>Both 1.15.0 keys are editable Bool rows with `since` v1.15.0; `DEFAULTS_OPTION_COUNT` is 132 and passes; the Enum count is still 12.</done>
</task>

<task type="auto">
  <name>Task 3: Move the sync baseline to ec81d0d (VERSION stays 1.14.0), rewrite docs/GSD-CORE-SYNC.md, run the full gates</name>
  <files>src/state_reader/config_json.rs, docs/GSD-CORE-SYNC.md</files>
  <action>
    Re-measure before editing (read-only on the upstream repo): `git -C ~/projects/node/gsd-core describe --tags --always upstream/release-1.15.0` must print `v1.14.0-111-gec81d0d10`; `npm view @opengsd/gsd-core version` must print `1.14.0` (if it now prints 1.15.0, STOP and escalate — the VERSION decision below flips). Re-run the step-3 comm against `git show ec81d0d:docs/CONFIGURATION.md` using `rtk proxy` for any count and record the measured line / dotted counts (expected 169 / 113).

    config_json.rs: set `GSD_CORE_SYNCED_COMMIT` to `"v1.14.0-111-gec81d0d10"` (INFERRED I-6: the describe spelling, which contains the requested abbreviated sha ec81d0d and satisfies `the_gsd_core_sync_baseline_is_recorded`). Leave `GSD_CORE_SYNCED_VERSION` at `"1.14.0"`. Rewrite both doc comments: VERSION is now defined as the latest gsd-core release PUBLISHED ON NPM at or below the synced commit — the conformance-oracle pin that scripts/install-conformance-oracle.sh (and through it the publish job, .github/workflows/release.yml, and `./scripts/pre-tag-check.sh --container`) npm-installs — and it deliberately differs from the synced tree's package.json (1.15.0) because 1.15.0 is unpublished; bump it to 1.15.0 together with COMMIT once `npm view @opengsd/gsd-core version` reports 1.15.0 and a v1.15.0 tag exists. COMMIT's comment: the tip of gsd-core's release-1.15.0 branch, 111 commits past v1.14.0, full sha ec81d0d10f545dd5aea2cc893863a542bc49029d. Replace the re-measure recipe in the comment with the ref-based form (`git -C … describe --tags --always <ref>`, `git -C … show <ref>:package.json`, `npm view @opengsd/gsd-core version`), noting the local checkout's HEAD is not the sync target. Do not edit scripts/ or .github/.

    docs/GSD-CORE-SYNC.md — full rewrite, keeping the exact headings `## Modelled` and `## Pass-through` (the sync-record test splits on them) and the existing per-category tables (with Task 1/2's rows). Required content:
    1. Header: config surface synced against gsd-core `release-1.15.0` at `v1.14.0-111-gec81d0d10` (ec81d0d10f545dd5aea2cc893863a542bc49029d, package.json 1.15.0, untagged) on 2026-09-26; oracle / `GSD_CORE_SYNCED_VERSION` pinned at `1.14.0`, the latest version published on npm. Keep the "keep this file and the two constants in step, in the SAME commit" rule.
    2. A section explaining the VERSION/COMMIT split and its exit condition (bump VERSION and re-describe COMMIT once 1.15.0 is on npm and tagged; then re-run `./scripts/pre-tag-check.sh --container vX.Y.Z`), and that `the_gsd_core_sync_baseline_is_recorded` requires COMMIT to contain VERSION, which holds only while 1.15.0 is untagged.
    3. "How to re-measure" rewritten ref-based: set `CORE` and `REF`; baseline pair via `git -C "$CORE" describe --tags --always "$REF"` and `npm view @opengsd/gsd-core version`; key cells via `git -C "$CORE" show "$REF:docs/CONFIGURATION.md"` piped to the existing grep; the delta between the previous and new baseline via `git -C "$CORE" diff <prev> "$REF" -- docs/CONFIGURATION.md gsd-core/bin/shared/config-schema.manifest.json gsd-core/bin/shared/config-defaults.manifest.json 'capabilities/*/capability.json' src/config-loader.cts`; the per-key `since` recipe with `git log "$REF" --reverse -S"<dotted key>"`, plus the rule that when `tag --contains` is empty the `since` is `v` + the ref's package.json version, marked as provisional until that tag exists. Warn that the checkout's working tree (currently `next`) is not the ref, and keep the existing warnings (dotted needle, odd early tag names, rtk-filtered pipes).
    4. A "1.15.0 delta" section recording the measured facts from `<upstream_facts>`: 258 -> 260 key cells, +2 / -0, schema manifest 121 -> 122, defaults manifest's new `planner` block, no default changes, the two introducing commits with issue numbers, the non-key diffs (runtime-identity fill #4717, capability version bumps, opencode installer flag), and the gsd-core fail-safe for a non-boolean `planner.stall_detection_enabled` versus this build's typed parse (I-5).
    5. Step-3 bucket table re-counted from the measurement above (replace the stale 171 / 113 figures with what was measured).
    6. `## Modelled` intro: 132 rows; 57 carry a measured 1.14.0-sync `since`, 2 carry the provisional `v1.15.0`, 73 predate the record; `DEFAULTS_OPTION_COUNT` pins 132.
    7. `## Pass-through`: replace the `executor.stall_*`, `planner.stall_*` family entry with `executor.stall_*` plus `planner.stall_detect_interval_minutes` / `planner.stall_threshold_minutes`, noting `planner` is now a typed block whose remaining keys render as `planner.<key>` rows (previously a single `planner` row holding the whole object). Keep the unprefixed-label "Known inconsistency" section.
    8. "Next sync": first item is the VERSION/COMMIT bump once 1.15.0 publishes and is tagged (and replacing the provisional `v1.15.0` since values with the measured tag); then the existing priority list.

    Then run the full gates from the repository root with raw output (never pipe these into grep; rtk filters piped output): `rtk proxy cargo test --no-fail-fast` and `rtk proxy cargo clippy --all-targets -- -D warnings`. The only permitted test failure is the src/envelope/policy.rs git-version witness (local git differs from the pinned constant). Record passed / failed / ignored counts and clippy's exit code in the SUMMARY, plus the INFERRED I-1..I-7 list.
  </action>
  <verify>
    <automated>rtk proxy cargo test --no-fail-fast</automated>
    <automated>rtk proxy cargo clippy --all-targets -- -D warnings</automated>
    <automated>test "$(grep -c '^pub const GSD_CORE_SYNCED_COMMIT: &str = "v1.14.0-111-gec81d0d10";$' src/state_reader/config_json.rs)" = 1 && test "$(grep -c '^pub const GSD_CORE_SYNCED_VERSION: &str = "1.14.0";$' src/state_reader/config_json.rs)" = 1 && grep -q 'v1.14.0-111-gec81d0d10' docs/GSD-CORE-SYNC.md && grep -q 'ec81d0d10f545dd5aea2cc893863a542bc49029d' docs/GSD-CORE-SYNC.md</automated>
  </verify>
  <done>Constants read `v1.14.0-111-gec81d0d10` / `1.14.0` with the split documented in code and in the record; docs/GSD-CORE-SYNC.md is rewritten with ref-based commands, the 1.15.0 delta and 132 modelled rows; the full test run shows no failure beyond the policy.rs git-version witness; clippy --all-targets -D warnings exits 0.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| project `.planning/config.json` / `~/.gsd/defaults.json` -> Defaults tab | Hand-editable, attacker-influenceable JSON parsed into `GsdConfig` and drawn in the terminal |
| Defaults tab save -> config.json on disk | The TUI rewrites the whole file from `GsdConfig`; anything not captured is deleted |
| src/state_reader/config_json.rs constant -> publish job | `GSD_CORE_SYNCED_VERSION` is grepped by scripts/install-conformance-oracle.sh and npm-installed in the job holding the crates.io token |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-gtk-01 | Tampering | `append_passthrough_entries` / new `planner.` rows | medium | mitigate | Project-supplied `planner.<key>` names reach the screen only as `Cow::Owned` pass-through keys, which already flow through `shown()`; the two authored rows use `&'static str` keys and help (T-VQW-01 preserved). `planner_stall_tuning_keys_stay_visible_and_survive_a_toggle` asserts they are ReadOnly and the census asserts no mutation arm claims them. |
| T-gtk-02 | Tampering (data loss) | `PlannerConfig` replacing the top-level `planner` pass-through | high | mitigate | `#[serde(flatten)] extra` on `PlannerConfig` keeps `stall_detect_interval_minutes` / `stall_threshold_minutes`; Task 1 tests assert the serialize->reparse->serialize fixed point and that a toggle-then-save keeps both values. |
| T-gtk-03 | Denial of service | typed `Option<bool>` for `planner.stall_detection_enabled` | low | accept | A hand-edited non-boolean now fails the whole-file parse (no Defaults rows, therefore no save and no loss) — identical to every other typed key; gsd-core's `config-set` always writes a real boolean. Recorded as INFERRED I-5 in code and in the sync record. |
| T-gtk-04 | Tampering / availability | `GSD_CORE_SYNCED_VERSION` -> oracle install in the publish job | high | mitigate | VERSION stays `1.14.0` (latest on npm); Task 3's exact-line grep gate enforces it and Task 3 halts if npm already has 1.15.0. scripts/ and .github/ are not edited. |
| T-gtk-SC | Tampering | npm/cargo installs | low | accept | No dependency is added or bumped; `npm view` is a read-only metadata query. |
</threat_model>

<verification>
- `rtk proxy cargo test --no-fail-fast` (raw, unpiped): every suite runs; the only failure allowed is the src/envelope/policy.rs git-version witness.
- `rtk proxy cargo clippy --all-targets -- -D warnings` exits 0 (baseline measured clean before this item).
- `grep -oP 'push\(cat, "\K[^"]+' src/ui/screens/detail.rs | sort -u | wc -l` reports 132 (run via `rtk proxy` or with raw output).
- Constants: COMMIT `v1.14.0-111-gec81d0d10`, VERSION `1.14.0` (Task 3 grep gate).
- docs/GSD-CORE-SYNC.md names both constants and lists all 132 keys between `## Modelled` and `## Pass-through` (enforced by `the_sync_record_names_every_modelled_key_and_the_measured_baseline`).
</verification>

<success_criteria>
- The two gsd-core 1.15.0 keys are first-class, editable, correctly-pathed Defaults rows with `since` v1.15.0; no other upstream key was added, removed or changed in the range (re-confirmed in Task 3).
- `planner` tuning keys remain visible and lossless.
- The sync baseline points at ec81d0d while the oracle pin stays installable, and the reason is written down where the next syncer will read it.
- Full test and clippy gates as stated.
</success_criteria>

<output>
Create `.planning/quick/260926-gtk-config-re-sync-to-gsd-core-1-15-0-release-1-15-0-ec81d0d-add/260926-gtk-SUMMARY.md` when done, including the measured test counts, clippy exit code, step-3 recount, and the INFERRED I-1..I-7 decisions for audit.
</output>
