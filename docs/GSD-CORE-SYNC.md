# gsd-core config sync record

**Config surface synced against gsd-core `release-1.15.0` at
`v1.14.0-111-gec81d0d10`** (`ec81d0d10f545dd5aea2cc893863a542bc49029d`,
`package.json` 1.15.0, untagged) **on 2026-09-26. Conformance oracle /
`GSD_CORE_SYNCED_VERSION` pinned at `1.14.0`**, the latest version published on
npm.

`src/state_reader/config_json.rs` carries the machine-readable copy of those
values as three constants: `GSD_CORE_SYNCED_VERSION` (the oracle pin, `1.14.0`),
`GSD_CORE_SYNCED_COMMIT` (the synced tree's describe) and
`GSD_CORE_SYNCED_TREE_VERSION` (the synced tree's `package.json` version,
`1.15.0`). This file is
the human-readable half: the full key inventory, per-key status, and the exact
commands that produced it, so the *next* sync starts from a measured point
instead of a blank slate.

Before this record existed the drift was silent in both directions. The sync
before quick task 260916-vqw was at GSD 1.8.0 (quick task 260722-emn), nothing
in the tree said so, and by 1.14.0 gsd-core documented **57 scalar keys this
build modelled nowhere** — including `workflow.compact_content`, the one a human
happened to notice. The 1.14.0 re-sync (260916-vqw, at `v1.14.0-52-g651511d1e`)
closed that gap; the release-1.15.0 re-sync (260926-gtk) is the first delta sync
made FROM this record.

> **Keep this file, `config_json.rs`'s three constants and README.md's
> `**GSD compatibility:**` note in step, in the SAME commit.** A record that
> outlives its subject tells the next reader a surface is covered when it is not.

---

## Why VERSION and COMMIT disagree right now

The two constants normally name the same release. Today they deliberately do
not:

| Constant | Value | What it means |
|---|---|---|
| `GSD_CORE_SYNCED_COMMIT` | `v1.14.0-111-gec81d0d10` | The tree the config surface was diffed against: the tip of gsd-core's `release-1.15.0` branch, 111 commits past `v1.14.0`. Its `package.json` says `1.15.0`, but no `v1.15.0` tag exists, so `git describe` still names `v1.14.0`. |
| `GSD_CORE_SYNCED_TREE_VERSION` | `1.15.0` | The `package.json` version of the tree at `GSD_CORE_SYNCED_COMMIT`. The ceiling of the installed-GSD comparison's in-sync range (quick task 260926-j0a). |
| `GSD_CORE_SYNCED_VERSION` | `1.14.0` | The **conformance-oracle pin**: the latest gsd-core release published on npm at or below that commit. `scripts/install-conformance-oracle.sh` greps this declaration and runs `npm install @opengsd/gsd-core@<version>` — in the publish job (`.github/workflows/release.yml`) and inside `./scripts/pre-tag-check.sh --container`. |

`npm view @opengsd/gsd-core versions --json` ends at `1.14.0` as of 2026-09-26:
1.15.0 is unpublished, so pinning `1.15.0` would make the publish job fail to
install its oracle.

**Exit condition.** Once `npm view @opengsd/gsd-core version` reports `1.15.0`
AND a `v1.15.0` tag exists upstream: bump `GSD_CORE_SYNCED_VERSION` to `1.15.0`,
re-describe `GSD_CORE_SYNCED_COMMIT` (it will then read `v1.15.0…`), replace the
provisional `v1.15.0` `since` values below with the measured tag, and re-run
`./scripts/pre-tag-check.sh --container vX.Y.Z` so the new oracle is certified.

`the_gsd_core_sync_baseline_is_recorded` (in `config_json.rs`) requires COMMIT
to CONTAIN VERSION. That holds today only because 1.15.0 is untagged and the
describe still names `v1.14.0`; it is the test that will flag a half-done bump.

---

## How to re-measure (run these, do not trust this file's numbers)

The sibling checkout is **read-only** for this purpose. Nothing below writes to
it, and nothing below checks out a branch there.

**Measure against a REF, never the working tree.** The local checkout's HEAD is
whatever branch someone last worked on (at the time of writing: `next`, at
`v1.14.0-117-gebe51d68c` with `package.json` 1.14.0) — not the sync target. A
plain `cat "$CORE/docs/CONFIGURATION.md"` or `node -p require(package.json)`
measures the wrong tree.

```bash
CORE=~/projects/node/gsd-core
REF=upstream/release-1.15.0          # the sync target; a tag once one exists
PREV=651511d1e                       # the previous GSD_CORE_SYNCED_COMMIT's sha

# 1. The baseline constants.
git -C "$CORE" describe --tags --always "$REF"   # -> GSD_CORE_SYNCED_COMMIT
git -C "$CORE" show "$REF:package.json" | grep '"version"'   # -> GSD_CORE_SYNCED_TREE_VERSION; the tree's version
npm view @opengsd/gsd-core version               # -> GSD_CORE_SYNCED_VERSION

# 2. The two key sets. Pipe RAW output — rtk filters piped command output and
#    can make a count pass vacuously; use `rtk proxy` when a count matters.
git -C "$CORE" show "$REF:docs/CONFIGURATION.md" \
  | grep -oP '^\| `\K[a-z0-9_.<>-]+(?=`)' \
  | LC_ALL=C sort -u > /tmp/core-keys.txt
grep -oP 'push\(cat, "\K[^"]+' src/ui/screens/detail.rs \
  | LC_ALL=C sort -u > /tmp/gmm-keys.txt

# 3. What gsd-core documents and this build does not push a row for.
LC_ALL=C comm -23 /tmp/core-keys.txt /tmp/gmm-keys.txt

# 4. The delta since the previous baseline — keys, defaults, capability-
#    registered keys, and loader semantics.
git -C "$CORE" diff "$PREV" "$REF" -- docs/CONFIGURATION.md \
  gsd-core/bin/shared/config-schema.manifest.json \
  gsd-core/bin/shared/config-defaults.manifest.json \
  'capabilities/*/capability.json' src/config-loader.cts

# 5. Per-key introduction version (ID-4 — MEASURED, never recalled).
K=workflow.compact_content
C=$(git -C "$CORE" log "$REF" --reverse --format=%H -S"$K" -- docs/CONFIGURATION.md src/ capabilities/ | head -1)
git -C "$CORE" tag --contains "$C" --sort=v:refname | head -1
git -C "$CORE" describe --tags "$C"
```

**Step 5's needle is the DOTTED key, not the leaf name.** A leaf like `enabled`
or `auto_update` matches dozens of unrelated commits across `src/`, and the
first of those is a `since` that is wrong by several releases. Every key below
resolved on the dotted form; the leaf-name fallback was never reached.

**When `tag --contains` prints nothing**, the key is not in any release tag
yet. The `since` is then `v` + the REF's `package.json` version (today
`v1.15.0`), and it is **provisional** until that tag exists — re-run step 5 at
the next sync and replace it with the measured tag.

**gsd-core's early tags are spelled oddly and that is not a typo here.**
`v1.01.0` (2026-05-24) and `v1.03.0` sit between `v1.0.0` and `v1.2.0` in that
repository's own tag list. A `since: v1.01.0` below is the measured tag name,
reproduced verbatim.

**Capability-registered keys are not in the schema manifest.** A key declared in
`capabilities/<name>/capability.json` (like `workflow.ui_interaction_capture`)
shows up in CONFIGURATION.md and in step 4's capability diff, but not in
`config-schema.manifest.json`'s `validKeys`. Diff both.

---

## The release-1.15.0 delta (quick task 260926-gtk)

Measured on 2026-09-26 between the previous baseline `651511d1e`
(`v1.14.0-52-g651511d1e`, an ancestor of the target) and `ec81d0d`:

| Measurement | 651511d1e | ec81d0d | Change |
|---|---:|---:|---|
| CONFIGURATION.md key cells (step 2) | 258 | 260 | **+2 / -0** |
| `config-schema.manifest.json` `validKeys` | 120 | 121 | + `planner.stall_detection_enabled` (121 -> 122 counting the one `runtimeStateKeys` entry) |
| `config-defaults.manifest.json` | — | — | one new top-level block, `"planner": { "stall_detection_enabled": true }` |
| Existing keys whose default changed | | | **none** |

The two added keys, both now modelled as editable Bool rows with the
provisional `since` `v1.15.0` (`git tag --contains` is empty for both
introducing commits; they are reachable only from the release-1.15.0 branch):

| Key | Type / default | Introduced by | Notes |
|---|---|---|---|
| `planner.stall_detection_enabled` | boolean, `true` | 1e3e1f7cd (#4570 / PR #4585, `v1.14.0-98`) | New top-level `planner` block. `resolvePlannerStallDetectionEnabled` in `src/config-loader.cts` lets only a real JSON boolean override the default; a string, number or null fails safe to `true`. CONFIGURATION.md warns that `false` gives up bounded recovery if the runtime loses the completion handoff. |
| `workflow.ui_interaction_capture` | boolean, `false` | 88b5775dc (#4223 / PR #4477, `v1.14.0-80`) | Declared in `capabilities/ui/capability.json:43`, not in the schema manifest. Lets `gsd-ui-auditor` add post-interaction captures (hover, focus ring, open menus, filled forms) through the chrome-devtools CLI run from Bash; needs an installed Chrome (`CHROME_BIN` overrides); read by `/gsd-ui-review`. |

**A deliberate divergence (INFERRED I-5).** gsd-core fails a non-boolean
`planner.stall_detection_enabled` safe to `true`; this build types the key as
`Option<bool>`, so a hand-edited string or number is a whole-file parse failure
— the same as every other typed key here. A failed parse draws no Defaults rows
and therefore cannot save over the file, and gsd-core's own `config-set` coerces
`"false"` to a real boolean, so only a hand edit can produce the case.

**Config-adjacent diffs in the range that are NOT new keys:**

- `src/config-loader.cts` runtime-identity fill (#4717, 9a41a9521): fills the
  existing `runtime` key from `GSD_RUNTIME` / the per-install marker. Loader
  semantics, not a key — handled by the batch's Codex-awareness item.
- Every `capabilities/*/capability.json` `version` bumped `1.14.0` -> `1.15.0`
  (45 files).
- `capabilities/opencode` drops the installer flag `skipCodexSkillsManifest`
  (installer metadata, not a config key).
- CONFIGURATION.md prose edits to the `phase_id_convention`, `worktree.baseRef`
  and `resolve_model_ids` rows, and a new graphify-CLI prose section (no key).

### Step 3's output is not a gap list — read it with these buckets

At `ec81d0d` against this build it reports **169 lines, 113 of them dotted**
(was 171 before the two keys above were modelled):

| Bucket | Count | What it is |
|---|---:|---|
| Undotted value rows, not keys | 48 | The grep matches any leading table cell, so enum *values* (`balanced`, `codex`, `true`, `v1`) and table headings come through as if they were keys. |
| Undotted top-level keys, pass-through | 8 | `runtime`, `context_profile`, `agent_skills`, `kg_backend`, and the four search-provider overrides `tavily_search`, `ref_search`, `perplexity`, `jina`. Real keys with no typed row; each renders as a `Not modelled` row. Not new in this range. |
| Dotted value rows, not keys | 3 | `gpt-5.6-luna`, `gpt-5.6-sol`, `gpt-5.6-terra` — model ids from a value table. |
| Modelled under an unprefixed row label | 30 | `git.base_branch` is this tab's `base_branch` row, `intel.enabled` is `intel_enabled`, and 21 `workflow.*` keys are spelled bare. The JSON path each row reads is correct; only the label is unprefixed. See "Known inconsistency" below. |
| Pass-through families | 80 | The nested and templated keys in the [Pass-through](#pass-through) section, which get no typed row **by design**. |

The 1.14.0 record described this output as containing no missing scalar key.
Re-bucketing it here shows that was true of the dotted half only: the eight
undotted top-level keys above are real, pre-date this range, and are recorded
as pass-through rather than promoted, because this sync's scope is the 1.15.0
delta.

---

## Modelled

Every key `build_defaults_entries` pushes a row for, in display order. **132
rows**: **57** carry a `since` measured at the 1.14.0 re-sync (260916-vqw), **2**
carry the provisional `v1.15.0` added by the release-1.15.0 re-sync
(260926-gtk), and the **73** without one predate this record and were not
retro-measured.

`DEFAULTS_OPTION_COUNT` in `src/ui/screens/detail.rs` pins this count at 132,
`RESYNCED_KEYS` / `RESYNCED_KEYS_1_15_0` pin the two re-syncs' key lists, and
three coverage assertions refuse a row with no `ConfigHelp`.

### Planning

| Key | since |
|---|---|
| `research` | |
| `plan_check` | |
| `pattern_mapper` | |
| `nyquist_validation` | |
| `ui_phase` | |
| `ui_safety_gate` | |
| `ai_integration_phase` | |
| `subagent_timeout` | |
| `workflow.specless_probe_fallback` | |
| `workflow.assumption_delta` | |
| `workflow.plan_drift_precheck` | |
| `workflow.plan_chunked` | |
| `workflow.context_guard_mode` | |
| `workflow.context_coverage_gate` | v1.01.0 |
| `workflow.context_drift_precheck` | v1.13.0 |
| `workflow.context_drift_action` | v1.13.0 |
| `workflow.drift_threshold` | v1.01.0 |
| `workflow.drift_action` | v1.01.0 |
| `workflow.inline_plan_threshold` | v1.01.0 |
| `workflow.max_discuss_passes` | v1.01.0 |
| `workflow.smart_zone_tokens` | v1.9.0 |
| `workflow.post_planning_gaps` | v1.01.0 |
| `workflow.plan_bounce` | v1.01.0 |
| `workflow.plan_bounce_passes` | v1.01.0 |
| `workflow.plan_bounce_script` | v1.01.0 |
| `workflow.plan_review_convergence` | v1.01.0 |
| `planning.chunked_parallel` | v1.13.0 |
| `planning.pr_strict` | v1.12.0 |
| `planner.stall_detection_enabled` | v1.15.0 |
| `plan_review.source_grounding` | v1.2.0 |
| `plan_review.source_grounding_authority` | v1.2.0 |
| `features.global_learnings` | v1.01.0 |
| `features.thinking_partner` | v1.01.0 |

### Execution

| Key | since |
|---|---|
| `verifier` | |
| `tdd_mode` | |
| `code_review` | |
| `code_review_depth` | |
| `ui_review` | |
| `node_repair` | |
| `node_repair_budget` | |
| `workflow.api_coverage_gate` | |
| `workflow.windows_enforce` | |
| `workflow.mvp_mode` | |
| `workflow.test_gate_timeout` | |
| `workflow.code_review_command` | |
| `workflow.security_asvs_level` | |
| `workflow.security_block_on` | |
| `workflow.security_enforcement` | v1.01.0 |
| `workflow.agent_hint_routing` | v1.11.0 |
| `workflow.code_review_point` | v1.13.0 |
| `workflow.code_review_depth_overrides` | v1.12.0 |
| `workflow.human_verify_mode` | v1.01.0 |
| `workflow.build_command` | v1.01.0 |
| `workflow.test_command` | v1.01.0 |
| `workflow.worktree_skip_hooks` | v1.01.0 |
| `workflow.live_dom_uat` | v1.12.0 |
| `workflow.ui_interaction_capture` | v1.15.0 |
| `workflow.cross_ai_execution` | v1.01.0 |
| `workflow.cross_ai_command` | v1.01.0 |
| `workflow.cross_ai_timeout` | v1.01.0 |

### Docs & Output

| Key | since |
|---|---|
| `commit_docs` | |
| `planning.commit_docs` | v1.01.0 |
| `skip_discuss` | |
| `use_worktrees` | |
| `text_mode` | |
| `workflow.compact_content` | v1.14.0 |
| `response_language` | |

### Features

| Key | since |
|---|---|
| `intel_enabled` | |
| `graphify_enabled` | |
| `graphify_build_timeout` | |
| `graphify.graph_path` | |
| `graphify.auto_update` | v1.01.0 |
| `brave_search` | |
| `firecrawl` | |
| `exa_search` | |

### Model & Pipeline

| Key | since |
|---|---|
| `mode` | |
| `granularity` | |
| `model_profile` | |
| `parallelization` | |
| `auto_advance` | |
| `auto_chain_active` | |
| `branching_strategy` | |
| `base_branch` | |
| `phase_branch_template` | |
| `milestone_branch_template` | |
| `quick_branch_template` | |
| `git.create_tag` | v1.01.0 |
| `git.allow_default_branch_commits` | v1.13.0 |
| `git.protected_branches` | v1.12.0 |

### Misc

| Key | since |
|---|---|
| `context_warnings` | |
| `hooks.context_warning_threshold` | v1.14.0 |
| `hooks.context_critical_threshold` | v1.14.0 |
| `hooks.workflow_guard` | v1.01.0 |
| `context_window` | v1.01.0 |
| `research_before_questions` | |
| `discuss_mode` | |
| `search_gitignored` | |
| `planning.search_gitignored` | v1.01.0 |
| `project_code` | |
| `phase_naming` | |
| `phase_id_convention` | |
| `claude_md_path` | |
| `sub_repos` | |
| `planning.sub_repos` | v1.01.0 |
| `workflow.auto_prune_state` | v1.01.0 |

### Orchestration

| Key | since |
|---|---|
| `claude_orchestration.enabled` | |
| `claude_orchestration.execution_backend` | |
| `claude_orchestration.min_agent_sdk_version` | |

### Statusline

| Key | since |
|---|---|
| `statusline.show_context_tokens` | |
| `statusline.state_format` | |
| `statusline.show_git` | |
| `statusline.context_position` | v1.01.0 |
| `statusline.show_last_command` | v1.01.0 |
| `statusline.show_state_freshness` | v1.12.0 |

### Routing

| Key | since |
|---|---|
| `dynamic_routing.provider_escalation` | |
| `dynamic_routing.max_escalations` | |
| `dynamic_routing.enabled` | v1.01.0 |
| `dynamic_routing.escalate_on_failure` | v1.01.0 |

### External Job

| Key | since |
|---|---|
| `external_job.submit_timeout_ms` | |
| `external_job.poll_timeout_ms` | |
| `external_job.artifact_dir` | |

### Capabilities

| Key | since |
|---|---|
| `capabilities.strict_known_registries` | |
| `capabilities.auto_update` | |

### Review

| Key | since |
|---|---|
| `review.reviewer_instances` | |

### Gates

| Key | since |
|---|---|
| `gates.confirm_project` | v1.01.0 |
| `gates.confirm_roadmap` | v1.01.0 |
| `gates.confirm_phases` | v1.01.0 |
| `gates.confirm_breakdown` | v1.01.0 |
| `gates.confirm_plan` | v1.01.0 |
| `gates.execute_next_plan` | v1.01.0 |
| `gates.confirm_transition` | v1.01.0 |
| `gates.issues_review` | v1.01.0 |

---

## Pass-through

These gsd-core keys deliberately get **no typed row**. They are not missing:
each one still appears in the Defaults tab under the **`Not modelled`**
category, is rendered read-only, and round-trips byte-for-byte through a save.
Recording them here is what makes "deliberately deferred" distinguishable from
"nobody noticed".

Every key of a parsed block that this build has no field for lands in that
block's `#[serde(flatten)] extra` map, so the list below is a description of
what is *currently* in there — not a filter. A key gsd-core adds tomorrow, under
a name nobody here has seen, is handled by the same mechanism without an edit.

| Family | Why no typed row |
|---|---|
| `review.models.<cli>`, `review.reviewer_instances.<name>.*` | Templated by CLI / instance name; a typed field per unknown name is not expressible. |
| `review.default_reviewers`, `review.*_host`, `review.max_prompt_tokens*`, `review.parallel_lanes` | Would be typeable; deferred because the whole `review.*` block is one editing surface and splitting it half-and-half reads worse than leaving it whole. |
| `model_policy.*`, `model_profile_overrides.<runtime>.<tier>` | Keyed by runtime and tier; templated. |
| `effort.*`, `fast_mode.*` | Both carry `agent_overrides.<agent-id>` and `routing_tier_defaults.*`; templated. |
| `dynamic_routing.tier_models.<tier>` | Templated by tier name. |
| `models.<phase_type>`, `granularities.<phase_type>` | Templated by phase type. |
| `agent_tools.<selector>`, `agent_skills*`, `agent_skills_security.trusted_global_roots` | Templated by selector; the security roots key is a list. |
| `code_quality.fallow.*` | Four keys of one opt-in subsystem this TUI surfaces nowhere else. |
| `mempalace.*` (11 keys) | An optional integration this build has no other awareness of. |
| `parallelization.*` (6 keys) | gsd-core's namespaced expansion of the top-level `parallelization` boolean this tab already edits; promoting them needs a decision about which wins. |
| `refactor.*`, `safety.*`, `security.injection_blocking` | Small coherent blocks, no typed row yet. |
| `executor.stall_*`, `planner.stall_detect_interval_minutes`, `planner.stall_threshold_minutes`, `manager.flags.*` | Runtime tuning knobs, rarely edited by hand. Since 260926-gtk `planner` is a TYPED block (`planner.stall_detection_enabled` is modelled), so its two remaining knobs render as separate `planner.<key>` rows; before it, the whole `planner` object was a single top-level `planner` row. |
| `runtime`, `tavily_search`, `ref_search`, `perplexity`, `jina` | Undotted top-level keys that pre-date the 1.15.0 range. `runtime` is also filled by gsd-core's loader from `GSD_RUNTIME` / the install marker (#4717), so the file value is not the whole story. The four search-provider keys double as API-key slots that gsd-core masks in its own display; typing them needs a masking decision first. |
| `claude_md_assembly.mode`, `learnings.max_inject`, `kg_backend`, `context_profile` | Singletons that did not fit an existing category. |

### Known inconsistency — unprefixed row labels

Thirty rows read a namespaced JSON path but are LABELLED without its prefix:
`base_branch` for `git.base_branch`, `intel_enabled` for `intel.enabled`,
`research` for `workflow.research`, and so on. The path each row reads is
correct; only the label a human sees is short. The newer rows all use the full
dotted spelling, so the tab currently mixes both conventions.

This is recorded rather than fixed here because renaming a label changes what
`set_config_value`, `clear_config_value` and `mutate_config_entry` match on, and
those three tables are keyed by the label. Doing it properly means moving all
three to the dotted spelling at once — a change with no user-visible benefit
beyond consistency, and one that would have been buried inside a key re-sync.

---

## Next sync

Start from step 1 above. **Every sync, whatever else it does:** when any of
`GSD_CORE_SYNCED_VERSION`, `GSD_CORE_SYNCED_COMMIT` or
`GSD_CORE_SYNCED_TREE_VERSION` moves, update README.md's `**GSD compatibility:**`
paragraph in the same commit — `the_readme_compatibility_note_names_the_synced_baseline`
fails otherwise. The app's installed-GSD comparison uses the oracle pin as the
floor and the tree version as the ceiling of the in-sync range, so bumping
either one moves the boundary at which users see the "newer than synced"
warning.

Then, in priority order:

1. **Bump the oracle pin once 1.15.0 ships.** When `npm view @opengsd/gsd-core
   version` reports `1.15.0` and a `v1.15.0` tag exists: set
   `GSD_CORE_SYNCED_VERSION` to `1.15.0`, re-describe `GSD_CORE_SYNCED_COMMIT`,
   replace the two provisional `v1.15.0` `since` values (in
   `build_defaults_entries` and in the tables above) with step 5's measured tag,
   and re-run `./scripts/pre-tag-check.sh --container vX.Y.Z`. See "Why VERSION
   and COMMIT disagree right now".
2. **Re-measure the baseline pair.** If `GSD_CORE_SYNCED_COMMIT` already
   matches the new REF, there is nothing else to do.
3. **Promote `parallelization.*` (6 keys).** The highest-value pass-through
   family: this tab already edits a top-level `parallelization` boolean, so a
   project carrying the namespaced block sees a row that disagrees with the file.
   Decide which path wins before adding rows.
4. **Promote the flat `review.*` keys** (`default_reviewers`, the three `*_host`
   keys, `max_prompt_tokens`, `max_prompt_tokens_per_reviewer`,
   `parallel_lanes`). All are scalars; only the `models.<cli>` and
   `reviewer_instances.<name>.*` families have to stay pass-through.
5. **Then `refactor.*` and `safety.*`**, which are small and fully scalar.
6. **Retro-measure `since` for the 73 rows that have none**, using step 5's
   command, if a reader ever needs the full history rather than the delta.
7. **Consider the label rename** described under "Known inconsistency" — as its
   own change, never folded into a sync.
