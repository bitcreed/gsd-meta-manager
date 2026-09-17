---
quick_id: 260916-vqw
type: execute
depends_on: []
files_modified:
  - src/state_reader/config_json.rs
  - src/ui/screens/detail.rs
  - src/ui/screens/render_escape_guard.rs
  - docs/GSD-CORE-SYNC.md
autonomous: true

must_haves:
  truths:
    - "A config key gsd-core writes but gsd-meta-manager does not model survives a Defaults-tab save instead of being deleted from the user's .planning/config.json."
    - "Every key present in a project's .planning/config.json is visible somewhere in the Defaults tab — modelled keys as editable rows, unmodelled keys as read-only rows."
    - "workflow.compact_content is an editable boolean row in the Defaults tab with help text naming the gsd-core version that introduced it."
    - "All 57 scalar config keys gsd-core 1.14.0 documents and this build did not model are editable (or read-only, for array-shaped keys) Defaults-tab rows with help text."
    - "A reader can determine, from a tracked file in this repo, which gsd-core version/commit the config surface was last synced against."
    - "A project-supplied config key rendered as a pass-through row cannot emit a terminal escape sequence."
  artifacts:
    - src/state_reader/config_json.rs
    - src/ui/screens/detail.rs
    - docs/GSD-CORE-SYNC.md
  key_links:
    - "GsdConfig `extra` flatten map -> serialize_gsd_config -> persist_active_config: the write path must round-trip what the read path did not model."
    - "build_defaults_entries pass-through rows -> shown() -> ratatui Span: project-supplied key and value both cross the untrusted-text boundary."
    - "GSD_CORE_SYNCED_VERSION constant -> docs/GSD-CORE-SYNC.md inventory: the baseline a future sync diffs from."
    - "DEFAULTS_OPTION_COUNT (73 -> 130) -> the three coverage assertions that refuse an option with no ConfigHelp."
---

<objective>
Re-sync gsd-meta-manager's understanding of GSD's config schema against gsd-core
1.14.0, record the baseline so the *next* sync starts from a known point, and stop
the drift from being silent.

Purpose: gsd-meta-manager last synced at GSD 1.8.0 (quick task 260722-emn). gsd-core
is now 1.14.0 and documents **57 scalar config keys this build does not model**,
including `workflow.compact_content` named in the todo. Worse, the drift is not
merely cosmetic: `persist_active_config` writes `serialize_gsd_config(&GsdConfig)`
straight over the user's `.planning/config.json`, and serde drops unknown keys — so
toggling one checkbox in the Defaults tab **silently deletes** every gsd-core key
this build does not model (`review.models.*`, `model_policy.*`, `effort.*`,
`gates.*`, …). Exposing new options without fixing that would hand users a wider
surface on top of a destructive writer.

Output: unknown-key preservation + read-only surfacing, 57 new first-class option
rows with per-key `since` annotations, and `docs/GSD-CORE-SYNC.md` recording the
synced gsd-core version, commit, and the full key inventory with per-key status.
</objective>

<execution_context>
@~/.claude/gsd-core/workflows/execute-plan.md
@~/.claude/gsd-core/templates/summary.md
</execution_context>

<context>
@.planning/todos/pending/2026-09-15-sync-gsd-core-config-and-expose-new-options.md
@src/state_reader/config_json.rs
</context>

## Measured drift (do not re-derive by hand — regenerate, see Task 3 verify)

Measured 2026-09-16 against `~/projects/node/gsd-core` at `v1.14.0-52-g651511d1e`
(`package.json` version `1.14.0`), reading `docs/CONFIGURATION.md`'s key table.

- gsd-meta-manager models **73** keys (the `push(cat, "…")` sites in
  `build_defaults_entries`, pinned by `DEFAULTS_OPTION_COUNT`).
- gsd-core documents **57** scalar keys this build models **nowhere**. Every one was
  confirmed present in gsd-core's own key table — none is invented.

| key | type | gsd-core default |
|---|---|---|
| `workflow.agent_hint_routing` | boolean | `true` |
| `workflow.auto_prune_state` | boolean | `false` |
| `workflow.build_command` | string | (none) |
| `workflow.code_review_depth_overrides` | array | `[]` (read-only) |
| `workflow.code_review_point` | enum | `execute:post` \| `execute:wave:post` |
| `workflow.compact_content` | boolean | `false` |
| `workflow.context_coverage_gate` | boolean | `true` |
| `workflow.context_drift_action` | enum | `warn` \| `block` |
| `workflow.context_drift_precheck` | boolean | `true` |
| `workflow.cross_ai_command` | string | (none) |
| `workflow.cross_ai_execution` | boolean | `false` |
| `workflow.cross_ai_timeout` | number | `300` |
| `workflow.drift_action` | enum | `warn` \| `auto-remap` |
| `workflow.drift_threshold` | number | `3` |
| `workflow.human_verify_mode` | enum | `end-of-phase` \| `mid-flight` |
| `workflow.inline_plan_threshold` | number | `3` |
| `workflow.live_dom_uat` | boolean | `false` |
| `workflow.max_discuss_passes` | number | `3` |
| `workflow.plan_bounce` | boolean | `false` |
| `workflow.plan_bounce_passes` | number | `2` |
| `workflow.plan_bounce_script` | string | (none) |
| `workflow.plan_review_convergence` | boolean | `false` |
| `workflow.post_planning_gaps` | boolean | `true` |
| `workflow.security_enforcement` | boolean | `true` |
| `workflow.smart_zone_tokens` | number | `100000` |
| `workflow.test_command` | string | (none) |
| `workflow.worktree_skip_hooks` | boolean | `false` |
| `features.global_learnings` | boolean | `false` |
| `features.thinking_partner` | boolean | `false` |
| `gates.confirm_breakdown` | boolean | `true` |
| `gates.confirm_phases` | boolean | `true` |
| `gates.confirm_plan` | boolean | `true` |
| `gates.confirm_project` | boolean | `true` |
| `gates.confirm_roadmap` | boolean | `true` |
| `gates.confirm_transition` | boolean | `true` |
| `gates.execute_next_plan` | boolean | `true` |
| `gates.issues_review` | boolean | `true` |
| `planning.chunked_parallel` | boolean | `false` |
| `planning.commit_docs` | boolean | `true` |
| `planning.pr_strict` | boolean | `false` |
| `planning.search_gitignored` | boolean | `false` |
| `planning.sub_repos` | array | `[]` (read-only) |
| `plan_review.source_grounding` | boolean | `true` |
| `plan_review.source_grounding_authority` | enum | `grep` \| `intel` \| `treesitter` \| `lsp` \| `scip` |
| `git.create_tag` | boolean | `true` |
| `git.allow_default_branch_commits` | boolean | `false` |
| `git.protected_branches` | array | (none) (read-only) |
| `hooks.workflow_guard` | boolean | `false` |
| `hooks.context_warning_threshold` | number | `35` |
| `hooks.context_critical_threshold` | number | `25` |
| `graphify.auto_update` | boolean | `false` |
| `context_window` | number | `200000` |
| `statusline.context_position` | enum | `end` \| `front` |
| `statusline.show_last_command` | boolean | `false` |
| `statusline.show_state_freshness` | boolean | `false` |
| `dynamic_routing.enabled` | boolean | `true` |
| `dynamic_routing.escalate_on_failure` | boolean | `true` |

73 + 57 = **130**, the value `DEFAULTS_OPTION_COUNT` must carry when this plan is done.

## Inferred decisions (human unavailable — audit these)

- **ID-1 — the durable baseline lives in two places, both tracked.** A
  `GSD_CORE_SYNCED_VERSION` / `GSD_CORE_SYNCED_COMMIT` constant pair in
  `src/state_reader/config_json.rs` (the file a future sync edits first, and the file
  the todo names), plus `docs/GSD-CORE-SYNC.md` carrying the full key inventory.
  The todo offered "a comment/constant in the synced config source file … or a
  tracked marker file"; both are taken because the constant is what a future syncer
  reads at the edit site and the inventory is what they diff against.
- **ID-2 — unknown-key preservation is in scope, not a separate todo.** It is the
  only mechanism that makes "not silently missing" true for gsd-core's *nested and
  templated* keys (`review.models.<cli>`, `model_policy.*`, `effort.*`, `fast_mode.*`,
  `agent_tools.<selector>`, `code_quality.fallow.*`, `models.<phase_type>`,
  `dynamic_routing.tier_models.*`, `agent_skills*`, `claude_md_assembly.mode`,
  `executor.stall_*`, `planner.stall_*`, `review.*` hosts/limits) without modelling
  each as a typed row. It also repairs a live data-loss bug on the save path.
- **ID-3 — the 57 scalar keys become typed rows; everything else is pass-through
  read-only AND listed in the inventory.** Nothing is dropped silently: a key that
  does not get a typed row still renders in the Defaults tab, still round-trips on
  save, and still appears in `docs/GSD-CORE-SYNC.md` with status `passthrough`. The
  inventory's `passthrough` section is the explicit, reviewable record of what a
  future sync should promote next.
- **ID-4 — `since` is resolved by measurement, not memory.** Per-key introduction
  version comes from gsd-core's own history, not from a guess (command in Task 2).
  A key whose first-touch commit is on no tag records `since` as the synced version.

## Read-first (executor: read these regions before editing)

- `src/state_reader/config_json.rs` — whole file (541 lines). `GsdConfig` and its
  nested structs; `serialize_gsd_config`.
- `src/ui/screens/detail.rs`:
  - `5340`–`5420` — `ConfigValueKind`, `ConfigHelp`.
  - `5480`–`5540` — `build_config_help_pane` and its escaping doc comment;
    `ConfigEntry`.
  - `5540`–`5620` — the `opt_*_layered` helpers and `opt_json_readonly`.
  - `5612` onward — `build_defaults_entries`, the `push` closure, the category regions.
  - `6140`–`6180` — `persist_active_config`, `dropdown_options`.
  - `6180`–`6480` — `set_config_value`, the clear arm table, the toggle arm table.
  - `6550`–`6700` — `populated_gsd_config` fixture, `all_config_entries`,
    `DEFAULTS_OPTION_COUNT`.
  - `6700`–`6975` — the coverage assertions, including
    `every_summary_is_within_the_pane_budget_and_is_not_a_restatement_of_the_key`.
- `src/ui/screens/detail.rs:77` — `shown()`, the untrusted-text helper every
  project-supplied string in this file goes through.

<tasks>

<task type="tracer" tdd="true">
  <name>Task 1: End-to-end unknown-key survival — parse, display, save, reload</name>
  <files>src/state_reader/config_json.rs, src/ui/screens/detail.rs, src/ui/screens/render_escape_guard.rs</files>
  <behavior>
    - A config.json containing `{"workflow":{"research":true,"unknown_gsd_key":7},"brand_new_block":{"a":1}}` parses, and both `workflow.unknown_gsd_key` and `brand_new_block` survive `serialize_gsd_config` -> `parse_gsd_config` byte-equivalently.
    - `build_defaults_entries` over that config emits a read-only row for each unmodelled key, under its own category, with kind `ConfigValueKind::ReadOnly`.
    - `set_config_value` / the clear arm / the toggle arm all return `false` for a pass-through key (no arm matches), so a pass-through row cannot be edited.
    - A key or value containing an ANSI escape or a C0 control renders with that sequence neutralised by `shown()`, never emitted raw.
    - `workflow.compact_content` is a modelled, editable boolean row — it does NOT appear among the pass-through rows.
  </behavior>
  <action>
Write the tests above first (RED), then implement.

1. In `src/state_reader/config_json.rs`, add a captured-remainder field to
   `GsdConfig` and to every nested config struct (`GitConfig`, `WorkflowConfig`,
   `HooksConfig`, `IntelConfig`, `GraphifyConfig`, `ClaudeOrchestrationConfig`,
   `StatuslineConfig`, `DynamicRoutingConfig`, `ReviewConfig`, `ExternalJobConfig`,
   `CapabilitiesConfig`):

       #[serde(flatten)]
       pub extra: serde_json::Map<String, serde_json::Value>,

   `Map` defaults to empty, so `Default` still derives and an absent remainder
   serialises to nothing. Serde's flatten removes consumed keys before filling the
   map, so a typed key can never also appear in `extra` (no duplicate-key output).
   Do NOT add `deny_unknown_fields` anywhere — it is incompatible with flatten.

2. Add the sync baseline constants to the same file, above `GsdConfig`, with a doc
   comment stating what they mean and that a future sync updates them (ID-1):

       pub const GSD_CORE_SYNCED_VERSION: &str = "…";   // package.json `version`
       pub const GSD_CORE_SYNCED_COMMIT: &str  = "…";   // `git describe --tags --always`

   Fill both by MEASURING, not from this plan's prose:
   `node -p "require('$HOME/projects/node/gsd-core/package.json').version"` and
   `git -C ~/projects/node/gsd-core describe --tags --always`. The sibling repo is
   READ-ONLY — never write to it.

3. Add a `since: &'static str` field to `ConfigHelp`, defaulting to `""` in the
   existing `new` / `with_choices` constructors so none of the 73 current call sites
   changes, plus a `const fn since(self, v: &'static str) -> Self` builder. Render it
   in `build_config_help_pane` as a dim trailing marker on the summary line when
   non-empty. A static literal is not project-supplied, so it stays outside the
   escaping rule that comment records — keep it that way.

4. Change `ConfigEntry.key` from `&'static str` to `std::borrow::Cow<'static, str>`
   so a pass-through row can carry a project-supplied key. Existing `push` sites keep
   passing `&'static str` via `Cow::Borrowed`; every read site matches on
   `entry.key.as_ref()`. `category` stays `&'static str`.

5. At the END of `build_defaults_entries` (after every static category), walk the
   `extra` maps — the top-level one and each nested struct's — and push one row per
   remaining key under a new final category. Row key is the dotted path
   (`workflow.unknown_gsd_key`); row value is `json_display` of the JSON value; kind
   is `ConfigValueKind::ReadOnly`; `from_defaults` follows the same project-then-
   defaults layering the `opt_*_layered` helpers use. Iterate in sorted key order so
   the row list is stable between frames. All rows share one static `ConfigHelp`
   summary explaining they are present in the project's config but not modelled by
   this build, shown read-only, and preserved on save.

6. **Untrusted-text boundary (T-VQW-01).** Both the dotted key and the rendered value
   come from the project's `.planning/config.json`. Route each through `shown()`
   (detail.rs:77) at the point it becomes a `Span`, exactly as every other
   project-supplied string in this file does. `build_config_help_pane`'s doc comment
   states that nothing parsed out of config.json reaches it — honour that by keeping
   the pass-through help summary a static literal and never interpolating the key
   into it. If `render_escape_guard.rs`'s compile-time census demands a fixture row
   or a registration for the new render site, add it — do not weaken the guard.

7. Leave `populated_gsd_config` free of unmodelled keys so `DEFAULTS_OPTION_COUNT`
   keeps counting static rows only; pass-through rows get their own test fixture.
  </action>
  <verify>
    <automated>cargo test --lib config_json -- --nocapture && cargo test --lib defaults && cargo test --lib render_escape_guard</automated>
  </verify>
  <done>An unmodelled key in a project's config.json is readable in the Defaults tab, is not editable there, cannot emit a terminal escape, and is byte-preserved through a save; `GSD_CORE_SYNCED_VERSION`/`GSD_CORE_SYNCED_COMMIT` carry measured values; `cargo clippy -- -D warnings` exits 0.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Model and expose the 27 new `workflow.*` keys with measured `since`</name>
  <files>src/state_reader/config_json.rs, src/ui/screens/detail.rs</files>
  <behavior>
    - Each of the 27 `workflow.*` keys in the drift table parses from a config.json into its typed `WorkflowConfig` field and survives a serialize/re-parse round trip.
    - Each appears exactly once as a `build_defaults_entries` row with a non-empty summary and a non-empty `since`.
    - Each boolean toggles, each enum sets from its dropdown, each clears back to `(unset)`; `workflow.code_review_depth_overrides` is `ReadOnly` and does none of those.
    - For every enum row, the documented choice list equals `dropdown_options` for that row (the existing paired assertion).
  </behavior>
  <action>
Add the 27 `workflow.*` keys from the drift table to `WorkflowConfig` as
`#[serde(default, skip_serializing_if = "Option::is_none")]` options, typed per the
table: booleans as `Option<bool>`, numbers as `Option<u32>`, free-form strings as
`Option<String>`, enums as `Option<String>` surfaced through `opt_enum_layered`, and
the array-shaped `code_review_depth_overrides` as `Option<serde_json::Value>` shown
through `opt_json_readonly` (the `security_block_on` precedent).

Wire each into `build_defaults_entries` in the category its meaning fits (Planning /
Execution / the existing category regions — follow where the neighbouring
`workflow.*` gates already sit), and into the `set_config_value`, clear and toggle
arm tables for every bool and enum. Extend the `populated_gsd_config` fixture JSON so
all 27 are non-null — the fixture doc comment records why an unpopulated fixture
makes the choice assertion vacuous.

Enum choice sets, each value getting its own one-clause explanation via
`ConfigHelp::with_choices`, taken from gsd-core's documented semantics:
  - `code_review_point`: `execute:post` (once, after every wave lands),
    `execute:wave:post` (once per wave, scoped to that wave's diff).
  - `context_drift_action` / `drift_action`: `warn`, and `block` / `auto-remap`
    respectively.
  - `human_verify_mode`: `end-of-phase`, `mid-flight`.

Resolve `since` per key by MEASURING gsd-core's history — never guess:

    K=compact_content   # the key's leaf name
    C=$(git -C ~/projects/node/gsd-core log --reverse --format=%H -S"$K" \
          -- docs/CONFIGURATION.md src/ | head -1)
    git -C ~/projects/node/gsd-core tag --contains "$C" --sort=v:refname | head -1

Use the first tag as `since`. A key whose first-touch commit is on no tag records
`GSD_CORE_SYNCED_VERSION`'s value (ID-4). The sibling repo is READ-ONLY.

Summaries must satisfy the two existing gates: at most 160 characters, and at least 4
words the key itself does not already spell.
  </action>
  <verify>
    <automated>cargo test --lib defaults && cargo test --lib config_json</automated>
  </verify>
  <done>All 27 `workflow.*` keys round-trip, each has an editable-or-read-only row carrying a summary and a measured `since`, and every enum row's documented choices equal its dropdown.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 3: Model the 30 non-`workflow` keys, pin the count at 130, write the sync record</name>
  <files>src/state_reader/config_json.rs, src/ui/screens/detail.rs, docs/GSD-CORE-SYNC.md</files>
  <behavior>
    - Each of the 30 remaining drift-table keys parses into its typed struct and survives a round trip.
    - `all_config_entries().len()` equals 130 and `DEFAULTS_OPTION_COUNT` equals 130.
    - Every one of the 130 rows carries a non-empty summary (the existing assertion, now over the larger set).
    - `docs/GSD-CORE-SYNC.md` names the synced gsd-core version and commit, and its `modelled` section lists every key `build_defaults_entries` pushes.
  </behavior>
  <action>
1. Add four new structs to `config_json.rs` — `FeaturesConfig`, `GatesConfig`,
   `PlanningConfig`, `PlanReviewConfig` — each `Deserialize, Serialize, Clone,
   Default, Debug` with the `extra` flatten field from Task 1, and hang them off
   `GsdConfig` as `Option<…>` with `skip_serializing_if`. Add `context_window:
   Option<u32>` at top level. Extend the existing `GitConfig` (`create_tag`,
   `allow_default_branch_commits`, `protected_branches` as
   `Option<serde_json::Value>`), `HooksConfig` (`workflow_guard`,
   `context_warning_threshold`, `context_critical_threshold`), `GraphifyConfig`
   (`auto_update`), `StatuslineConfig` (`context_position`, `show_last_command`,
   `show_state_freshness`), and `DynamicRoutingConfig` (`enabled`,
   `escalate_on_failure`) per the drift table.

   Note `planning.commit_docs` / `planning.search_gitignored` / `planning.sub_repos`
   are gsd-core's namespaced spellings of keys this build already models at top
   level. Model BOTH — they are distinct JSON paths and a project may carry either;
   give each namespaced row a summary that says it is the namespaced form.

2. Wire all 30 into `build_defaults_entries` — put `gates.*` in its own category,
   `features.*` and `plan_review.*` alongside the planning options, the rest beside
   their existing sibling rows — and into the set/clear/toggle arm tables for every
   bool and enum. `protected_branches` and `sub_repos` are `ReadOnly`.
   `plan_review.source_grounding_authority` and `statusline.context_position` are
   enums with the choice sets from the drift table. Resolve `since` with the Task 2
   measurement command. Extend `populated_gsd_config` so all 30 are non-null.

3. Update `DEFAULTS_OPTION_COUNT` from 73 to 130 and update its doc comment's "74th
   option" wording to the new boundary.

4. Create `docs/GSD-CORE-SYNC.md`, the tracked baseline a future sync diffs from:
   - the synced gsd-core version and commit (the same measured values the constants
     carry — state that the constants are the machine-readable copy);
   - the exact commands used to produce the diff, so the next sync reproduces it
     rather than starting from scratch;
   - a `## Modelled` table: every key `build_defaults_entries` pushes, with `since`;
   - a `## Pass-through` section: the gsd-core keys deliberately NOT given typed rows
     (the nested/templated families listed in ID-2), each with a one-line reason,
     recording them as tracked rather than missing (ID-3);
   - a `## Next sync` section naming what to promote first.

5. Move `.planning/todos/pending/2026-09-15-sync-gsd-core-config-and-expose-new-options.md`
   to `.planning/todos/completed/` only if the quick-task workflow does not already
   own that lifecycle step; if it does, leave it to the workflow.
  </action>
  <verify>
    <automated>cargo test --lib --no-fail-fast && cargo clippy -- -D warnings</automated>
  </verify>
  <done>`DEFAULTS_OPTION_COUNT` is 130 and the coverage assertions pass over all 130 rows; `docs/GSD-CORE-SYNC.md` exists with the measured version, commit, modelled table and pass-through section; `cargo test --lib --no-fail-fast` and `cargo clippy -- -D warnings` both exit 0.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| project `.planning/config.json` → TUI render | A registered project's config file is attacker-influenced content (a shared repo, a cloned project). Its keys and values are drawn into the terminal. |
| TUI Defaults tab → project `.planning/config.json` | The TUI overwrites the operator's real config file. |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-VQW-01 | Spoofing / Tampering | pass-through rows in `build_defaults_entries` | high | mitigate | Pass-through rows put a project-supplied **key** (new — previously `&'static str`) and value on screen. Both go through `shown()` at the `Span` site; the shared `ConfigHelp` summary stays a static literal so `build_config_help_pane` keeps its structural "no config.json string reaches here" property. `render_escape_guard.rs` must stay green, not be weakened. |
| T-VQW-02 | Denial of Service | `persist_active_config` | high | mitigate | Pre-existing and repaired by this plan: the writer serialises only modelled fields, destroying every unmodelled gsd-core key on save. The `extra` flatten maps make the write path lossless. Task 1's round-trip test is the pin. |
| T-VQW-03 | Denial of Service | `build_defaults_entries` pass-through walk | low | accept | A config with thousands of unknown keys makes a long row list. Bounded by the operator's own file, rendered lazily by ratatui's viewport, and no worse than a long phase list. |
| T-VQW-04 | Tampering | sibling repo `~/projects/node/gsd-core` | medium | mitigate | Every gsd-core access in this plan is a read (`git log`, `git describe`, `grep`, `node -p` on package.json). The task actions state the repo is READ-ONLY; no task writes a path outside `/home/blk/projects/rust/gsd-meta-manager`. |
| T-VQW-SC | Tampering | npm/pip/cargo installs | n/a | accept | No dependency is added or changed. `serde_json::Map` and `std::borrow::Cow` are already in the tree, so no package-legitimacy gate applies. |
</threat_model>

<verification>
- `cargo test --lib --no-fail-fast` — use `--no-fail-fast`: a plain `cargo test` stops
  at the pre-existing failing `driver_reattach` binary and never reaches later
  suites. Scope to `--lib` here because every change is in library code and its inline
  test modules.
- `cargo clippy -- -D warnings` exits 0 (the lib-target project gate;
  `--all-targets` has 5 documented pre-existing lints and is not the gate).
- `cargo build` exits 0.
- Regenerate the drift measurement and confirm it now reports zero unmodelled scalar
  keys, using the same extraction the plan used:
  `grep -oP '^\| \`\K[a-z0-9_.<>-]+(?=\`)' ~/projects/node/gsd-core/docs/CONFIGURATION.md | sort -u`
  compared against `grep -oP 'push\(cat, "\K[^"]+' src/ui/screens/detail.rs | sort -u`.
  Pipe raw output — `rtk` filters piped command output and can make a check pass
  vacuously; use `rtk proxy` if a count is load-bearing.
</verification>

<success_criteria>
- Saving from the Defaults tab preserves every key in the project's config.json,
  including ones this build does not model.
- The Defaults tab lists 130 modelled options plus one read-only row per unmodelled
  key found in the project's config.
- `workflow.compact_content` is editable and its help names the gsd-core version
  that introduced it.
- `GSD_CORE_SYNCED_VERSION` / `GSD_CORE_SYNCED_COMMIT` and `docs/GSD-CORE-SYNC.md`
  give the next sync a measured starting point instead of a blank slate.
- No file outside `/home/blk/projects/rust/gsd-meta-manager` is modified.
</success_criteria>

<output>
Create `.planning/quick/260916-vqw-sync-gsd-core-config-and-expose-new-options-area-config-seve/260916-vqw-SUMMARY.md` when done.
</output>
