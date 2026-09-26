# gsd-core config sync record

**Synced against gsd-core `1.14.0` at `v1.14.0-52-g651511d1e`, on 2026-09-16.**

`src/state_reader/config_json.rs` carries the machine-readable copy of those two
values as `GSD_CORE_SYNCED_VERSION` and `GSD_CORE_SYNCED_COMMIT`. This file is
the human-readable half: the full key inventory, per-key status, and the exact
commands that produced it, so the *next* sync starts from a measured point
instead of a blank slate.

Before this record existed the drift was silent in both directions. The previous
sync was at GSD 1.8.0 (quick task 260722-emn), nothing in the tree said so, and
by 1.14.0 gsd-core documented **57 scalar keys this build modelled nowhere** —
including `workflow.compact_content`, the one a human happened to notice.

> **Keep this file and `config_json.rs`'s two constants in step, in the SAME
> commit.** A record that outlives its subject tells the next reader a surface is
> covered when it is not.

---

## How to re-measure (run these, do not trust this file's numbers)

The sibling checkout is **read-only** for this purpose. Nothing below writes to
it.

```bash
CORE=~/projects/node/gsd-core

# 1. The baseline pair.
node -p "require('$CORE/package.json').version"     # -> GSD_CORE_SYNCED_VERSION
git -C "$CORE" describe --tags --always             # -> GSD_CORE_SYNCED_COMMIT

# 2. The two key sets. Pipe RAW output — rtk filters piped command output and
#    can make a count pass vacuously; use `rtk proxy` when a count matters.
grep -oP '^\| `\K[a-z0-9_.<>-]+(?=`)' "$CORE/docs/CONFIGURATION.md" \
  | LC_ALL=C sort -u > /tmp/core-keys.txt
grep -oP 'push\(cat, "\K[^"]+' src/ui/screens/detail.rs \
  | LC_ALL=C sort -u > /tmp/gmm-keys.txt

# 3. What gsd-core documents and this build does not push a row for.
LC_ALL=C comm -23 /tmp/core-keys.txt /tmp/gmm-keys.txt

# 4. Per-key introduction version (ID-4 — MEASURED, never recalled).
K=workflow.compact_content
C=$(git -C "$CORE" log --reverse --format=%H -S"$K" -- docs/CONFIGURATION.md src/ | head -1)
git -C "$CORE" tag --contains "$C" --sort=v:refname | head -1
```

**Step 4's needle is the DOTTED key, not the leaf name.** A leaf like `enabled`
or `auto_update` matches dozens of unrelated commits across `src/`, and the
first of those is a `since` that is wrong by several releases. Every one of the
57 keys below resolved on the dotted form; the leaf-name fallback was never
reached.

**gsd-core's early tags are spelled oddly and that is not a typo here.**
`v1.01.0` (2026-05-24) and `v1.03.0` sit between `v1.0.0` and `v1.2.0` in that
repository's own tag list. A `since: v1.01.0` below is the measured tag name,
reproduced verbatim.

### Step 3's output is not a gap list — read it with these three buckets

At the time of writing it reports 171 lines, 113 of them dotted, and **none of
them is a missing scalar key**:

| Bucket | Count | What it is |
|---|---:|---|
| Value rows, not keys | ~58 | The grep matches any leading table cell, so enum *values* (`balanced`, `codex`, `gpt-5.6-sol`, `true`) come through as if they were keys. |
| Modelled under an unprefixed row label | 30 | `git.base_branch` is this tab's `base_branch` row, `intel.enabled` is `intel_enabled`, and 21 `workflow.*` keys are spelled bare. The JSON path each row reads is correct; only the label is unprefixed. See "Known inconsistency" below. |
| Pass-through families | ~80 | The nested and templated keys in the [Pass-through](#pass-through) section, which get no typed row **by design**. |

---

## Modelled

Every key `build_defaults_entries` pushes a row for, in display order. **130
rows**, of which **57** were added by this sync and carry a measured `since`; the
73 without one predate this record and were not retro-measured.

`DEFAULTS_OPTION_COUNT` in `src/ui/screens/detail.rs` pins this count, and three
coverage assertions refuse a row with no `ConfigHelp`.

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
| `executor.stall_*`, `planner.stall_*`, `manager.flags.*` | Runtime tuning knobs, rarely edited by hand. |
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
beyond consistency, and one that would have been buried inside a 57-key re-sync.

---

## Next sync

Start from step 3 above, then, in priority order:

1. **Re-measure the baseline pair first.** If `GSD_CORE_SYNCED_COMMIT` already
   matches, there is nothing to do.
2. **Promote `parallelization.*` (6 keys).** The highest-value pass-through
   family: this tab already edits a top-level `parallelization` boolean, so a
   project carrying the namespaced block sees a row that disagrees with the file.
   Decide which path wins before adding rows.
3. **Promote the flat `review.*` keys** (`default_reviewers`, the three `*_host`
   keys, `max_prompt_tokens`, `max_prompt_tokens_per_reviewer`,
   `parallel_lanes`). All are scalars; only the `models.<cli>` and
   `reviewer_instances.<name>.*` families have to stay pass-through.
4. **Then `refactor.*` and `safety.*`**, which are small and fully scalar.
5. **Retro-measure `since` for the 73 rows that have none**, using step 4's
   command, if a reader ever needs the full history rather than the delta.
6. **Consider the label rename** described under "Known inconsistency" — as its
   own change, never folded into a sync.
