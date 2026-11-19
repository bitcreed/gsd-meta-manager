---
quick_id: 261119-ikn
mode: quick
phase: quick-261119-ikn
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/state_reader/config_json.rs
  - src/ui/screens/detail.rs
  - src/state_reader/roadmap_md.rs
  - src/state_reader/disk_status.rs
  - docs/GSD-CORE-SYNC.md
  - README.md
autonomous: true
requirements: [QUICK-261119-ikn]

estimate:
  tokens: 120000
  raw_tokens: 120000
  tasks: 3
  confidence: low

must_haves:
  truths:
    - "A project whose .planning/config.json carries `workflow.cross_runtime_execution: true` sees ONE typed editable Bool row `workflow.cross_runtime_execution` (category Execution, directly after `workflow.agent_hint_routing`, since v1.22.0, built-in default false) instead of a `Not modelled` pass-through row; Enter toggles it, set and x-clear work, and the key survives a save."
    - "A project carrying `execute.models.codex` sees that model id unmasked inside the read-only top-level `execute` pass-through row, preserved byte for byte on save; docs/GSD-CORE-SYNC.md lists `execute.models.<runtime>` under Pass-through."
    - "The `planning.commit_docs` and `planning.search_gitignored` help summaries say gsd-core reads the namespaced spelling only when the flat key is unset, so the flat key wins when both are set (gsd-core #5470, documented at v1.22.0)."
    - "A ROADMAP.md that shows `### Phase 9: Example` inside a fenced code block, or indented four spaces, or a fenced `- [ ] **Phase 4: Example**` checklist line, lists no phase 9 or 4 in the phase list; `##### Phase N:`, `###Phase N:` and `> ### Phase N:` stay non-phases; a fenced example neither shadows a real phase's goal nor opens a milestone (port of gsd-core #5880 / PR #6291)."
    - "A phase directory `XR-03-demo` reads `03-UAT.md` / `03-VERIFICATION.md` and `5-demo` reads `05-UAT.md` / `05-VERIFICATION.md` even when `03-HUMAN-UAT.md` / `03-CORRECTION-VERIFICATION.md` sort first; a literal `XR-03-VERIFICATION.md` outranks `03-VERIFICATION.md`; a bare `VERIFICATION.md` is read only when no dashed report exists; a directory whose token names no listed report keeps the sorted-first rule (port of gsd-core #3492 + #5931 / PR #6263)."
    - "src/state_reader/config_json.rs declares GSD_CORE_SYNCED_VERSION = \"1.22.0\", GSD_CORE_SYNCED_COMMIT = \"v1.22.0\", GSD_CORE_SYNCED_TREE_VERSION = \"1.22.0\"; the install-conformance-oracle.sh extraction yields 1.22.0 from exactly one declaration line; a user on installed gsd-core 1.22.0 no longer gets the 'newer than synced' warning."
    - "README.md's single `**GSD compatibility:**` paragraph names gsd-core 1.22.0 (`v1.22.0`) with oracle pin `1.22.0`; docs/GSD-CORE-SYNC.md carries the v1.22.0 header, constants table, history sentence, a `## The v1.22.0 delta (quick task 261119-ikn)` section placed before the v1.21.0 delta, the REF=v1.23.0 / PREV=v1.22.0 recipe, and the Modelled count 140."
    - "Every gsd-tools invocation this project makes is audited against v1.22.0's docs/CLI-SURFACE-MANIFEST.json and recorded; none needs a code change. The secret-name heuristic re-diff is recorded (6 -> 6, no new exemption)."
    - "Gate: tests/driver_router_conformance.rs passes against the installed ~/.claude/gsd-core 1.22.0 AND against a freshly npm-installed @opengsd/gsd-core@1.22.0 oracle provisioned by scripts/install-conformance-oracle.sh into a throwaway HOME; `cargo test --locked --no-fail-fast`, `cargo clippy --all-targets --locked -- -D warnings` and `cargo fmt --check` are clean."
  artifacts:
    - path: src/state_reader/config_json.rs
      provides: "WorkflowConfig.cross_runtime_execution and the baseline trio at 1.22.0 / v1.22.0 / 1.22.0"
      contains: "pub const GSD_CORE_SYNCED_COMMIT: &str = \"v1.22.0\";"
    - path: src/ui/screens/detail.rs
      provides: "The workflow.cross_runtime_execution Bool row and its set/clear/toggle arms, RESYNCED_KEYS_1_22_0, DEFAULTS_OPTION_COUNT 140, the two namespaced help texts, new tests"
      contains: "const DEFAULTS_OPTION_COUNT: usize = 140;"
    - path: src/state_reader/roadmap_md.rs
      provides: "Phase list, goals and milestone scope read only real ATX phase headings (<= 3 spaces, outside fenced code)"
      contains: "fenced_line_mask"
    - path: src/state_reader/disk_status.rs
      provides: "Phase-token-first VERIFICATION/UAT report selection with the code-stripped zero-padded spelling"
      contains: "upstream_dir_token"
    - path: docs/GSD-CORE-SYNC.md
      provides: "v1.22.0 header, delta section, Execution row, Pass-through row, Modelled 140, recipe REF=v1.23.0 / PREV=v1.22.0, Next sync item"
      contains: "## The v1.22.0 delta (quick task 261119-ikn)"
    - path: README.md
      provides: "Compatibility note naming 1.22.0"
      contains: "gsd-core 1.22.0 (`v1.22.0`), with oracle pin `1.22.0`"
  key_links:
    - from: src/state_reader/config_json.rs
      to: src/ui/screens/detail.rs
      via: "WorkflowConfig.cross_runtime_execution is read by build_defaults_entries through opt_bool; before this task the key lands in WorkflowConfig.extra and renders as a pass-through row"
      pattern: "cross_runtime_execution"
    - from: src/state_reader/roadmap_md.rs
      to: src/state_reader/mod.rs
      via: "parse_roadmap_phases / roadmap_milestones feed state.phases and state.milestones (mod.rs ~999-1003), which the dashboard, the Roadmap tab and the driver router read"
      pattern: "parse_roadmap_phases"
    - from: src/state_reader/disk_status.rs
      to: tests/driver_router_conformance.rs
      via: "infer_disk_status's verification_status / uat_status drive routing; the conformance test compares that routing with gsd-tools `query init.manager`"
      pattern: "read_verification_status"
    - from: src/state_reader/config_json.rs
      to: scripts/install-conformance-oracle.sh
      via: "the script greps the single `pub const GSD_CORE_SYNCED_VERSION: &str =` line and npm-installs @opengsd/gsd-core@<pin> (release.yml publish job, pre-tag-check --container)"
      pattern: "pub const GSD_CORE_SYNCED_VERSION"
    - from: src/ui/screens/detail.rs
      to: docs/GSD-CORE-SYNC.md
      via: "the_sync_record_names_every_modelled_key_and_the_measured_baseline (~17449) and the_readme_compatibility_note_names_the_synced_baseline (~17522)"
      pattern: "GSD-CORE-SYNC.md"
---

<!-- planner-discipline-allow: 1\.21\.0 -->
<!-- planner-discipline-allow: 1.21.0 -->

<objective>
Sync this build against gsd-core v1.22.0 (npm `@opengsd/gsd-core@latest`, published 2026-11-19), following docs/GSD-CORE-SYNC.md "How to re-measure" with REF=v1.22.0, PREV=v1.21.0, exactly as quick task 261113-fi0 synced v1.21.0. Four product commits plus one baseline commit:

1. (tracer) model the new scalar key `workflow.cross_runtime_execution` end to end: serde field -> typed Bool row with set/clear/toggle -> sync record's Modelled list -> tests; record the new map key `execute.models.<runtime>` as pass-through;
2. align the two namespaced `commit_docs` / `search_gitignored` help texts with the flat-wins precedence gsd-core v1.22.0 documents (#5470);
3. port two `.planning` reader fixes gsdmm has the same reader for: real ATX phase headings only (#5880) and the phase-token VERIFICATION/UAT resolver (#5931, with its #3492 prerequisite);
4. move the baseline trio, README note and sync record to v1.22.0 in ONE commit, record the measured delta (gsd-tools audit, secret re-diff, every candidate's verdict), and run the router conformance test against the 1.22.0 oracle.

Purpose: this machine's installed gsd-core is already 1.22.0 (`~/.claude/gsd-core/VERSION`), so every user on 1.22.0 currently sees a false "newer than synced" warning; the new gate key renders as a `Not modelled` row; and a ROADMAP example block or a project-coded phase directory makes gsdmm disagree with what gsd-core v1.22.0 counts and routes.

## Measured by the planner on 2026-11-19 (the executor re-measures; the record's motto is "run these, do not trust this file's numbers")

How measured: the sibling checkout `~/projects/node/gsd-core` HAS the `v1.22.0` tag (its HEAD is a fork docs branch at `v1.21.0-13-g4f9cb0d42`, package.json 1.21.0, with local work). Every number below came from `git -C ~/projects/node/gsd-core show|diff|log|describe|tag --contains <tag>...` against the tags. Nothing was fetched, checked out or written there, and no scratch clone was needed.

Baseline:
- `git rev-parse v1.22.0^{commit}` = 1f800effd5c33c155c57ad89ca7ec620f4b56035 (lightweight tag); `git describe --tags --always v1.22.0` = `v1.22.0`; tag commit 2026-11-19T04:03:58Z "chore: promote CHANGELOG for v1.22.0"; `v1.22.0:package.json` version 1.22.0.
- `npm view @opengsd/gsd-core version gitHead` = 1.22.0 / 1f800effd5c33c155c57ad89ca7ec620f4b56035, published 2026-11-19T04:11:34Z, repository `open-gsd/gsd-core`.
- v1.21.0 is an ancestor of v1.22.0. `git rev-list --count v1.21.0..v1.22.0` = 74 (4 merges); `git diff --shortstat` = 482 files changed, 30159 insertions, 2313 deletions.

Config surface:
- Step 2 key cells: 264 at v1.21.0, 266 at v1.22.0, **+2 / -0**: `workflow.cross_runtime_execution` (boolean, default `false`; "Registered in this release; nothing reads it until the dispatch phase ships") and `execute.models.<runtime>` (string, "(target default)"; model id passed to a foreign executor of `<runtime>`, a `[A-Za-z0-9_-]` segment; "unused until the dispatch phase ships"). Step 5 for both: first commit e99b85854 (2026-11-18, "feat(#6302): executor_runtime plan field, refusals, cross_runtime_execution gate and execute.models keys (#6307)"), `tag --contains` -> `v1.22.0`, describe `v1.21.0-62-ge99b85854`. So `since` = `v1.22.0`, MEASURED.
- `workflow.agent_hint_routing` is NOT new: a key cell at both tags, modelled here since 260916-vqw (since v1.11.0). At v1.22.0 its doc row grows (a routed specialist gets gsd-executor's isolation and model rules; GSD role names other than gsd-executor are refused; asked as the Executor question by /gsd-new-project and /gsd-settings) and `planNewProjectConfig` now writes `agent_hint_routing: true` into new configs (#6211 / PR #6225). Default still `true`; this build's row help ("Sends a plan whose frontmatter names a specialist subagent to that one instead of the generic executor.") and built-in default (`"true"`, detail.rs ~12920) stay accurate.
- `config-schema.manifest.json`: +1 `validKeys` (`workflow.cross_runtime_execution`), +1 dynamic pattern (`topLevel: "execute"`, source `^execute\.models\.[a-zA-Z0-9_-]+$`). `config-defaults.manifest.json`: +`"cross_runtime_execution": false` in `workflow`. `gsd-core/templates/config.json`: no diff.
- `src/config.cts`: SCHEMA_DEFAULTS +`workflow.cross_runtime_execution: false`; planNewProjectConfig +`agent_hint_routing: true`; `cmdConfigGet` resolves `commit_docs` and `search_gitignored` through the loader under either spelling (#5470 / PR #5502).
- `src/config-loader.cts`: new `CONFIG_KEY_ALIASES` table (#5470). The reads are unchanged: flat key first, nested `<section>.<field>` only when the flat key is unset (except `branching_strategy`, `base_branch`, `sub_repos`, where normalizeLegacyKeys hoists and the nested value wins). No key or default semantics change.
- `src/secrets.cts`, `gsd-core/references/gsd-run-resolver.md`: no diff. Existing keys whose default changed: none.
- `capabilities/*/capability.json`: 45 files at both tags, all 45 change only `version`.
- `resolve_model_ids: "omit"` (#3703 / PR #6217) changes resolve-model semantics; it is not a CONFIGURATION.md key cell and this build has no row or help text for it (a project carrying it sees the generic pass-through row). No change.
- Step 3 (`comm -23`) at the current tree: 172 against v1.21.0, 174 against v1.22.0.

Secret-bearing re-diff: heuristic matches 6 at v1.21.0 and the same 6 at v1.22.0 (`hooks.secret_read_exempt`, `review.context_window_tokens`, `review.max_prompt_tokens`, `review.max_prompt_tokens_per_reviewer`, `statusline.show_context_tokens`, `workflow.smart_zone_tokens`) — all already in NON_SECRET_CONFIG_KEYS. The new names `workflow.cross_runtime_execution` and `execute.models.<runtime>` trip none of the markers. CONFIGURATION.md still has 6 "Masked in display" cells. No change to config_secrets.rs.

gsd-tools invocation audit against v1.22.0's docs/CLI-SURFACE-MANIFEST.json: 94 declared verbs at both tags; `global.flags` = cwd, default, json-errors, pick, project-dir, raw, ws (unchanged); `global.rejected` = `--version`, `-v` (unchanged); `undeclared` identical at both tags (138 entries, still holding `smart-entry` and `init.manager`). Verbs whose entry changed: `record-dispatch-isolation` (+`--executor-type`), `resolve-agent` (+`--phase`, `--plan`), `resolve-model` (positionals 2 -> 1, +`--fallback-agent`, free positionals) — none called here. Invocations, run against installed 1.22.0 in this repo:
- src/state_reader/queue_md.rs:186 `smart-entry --json`: exit 0, valid JSON with `actions`.
- tests/driver_router_conformance.rs:177 (`ORACLE_VERB` :56) and scripts/install-conformance-oracle.sh:242 (`ORACLE_VERB` :104) `query init.manager`: exit 0.
- tests/launcher_live_isolation.rs:126 `runtime-identity --raw`: exit 0, `{"packageName":"@opengsd/gsd-core","version":"1.22.0"}`; `flags: {}` at both tags.
- tests/driver_router_conformance.rs:137 PATH probe `gsd-tools --version`: no `gsd-tools` on PATH here; the probe only checks that a binary spawns, and `--version` was already rejected at v1.21.0.
- tests/launcher_config_dir.rs:299 `query init` inside a hook-command string fixture: never executed.
Conclusion: no code change.

Reader candidates, each checked against this tree:
- total_phases counts only real level 2-4 ATX headings (#5880 / PR #6291, abc94ae5a; src/state.cts countRoadmapPhaseHeadings and src/validate.cts buildRoadmapPhaseVariants now read through `tokenizeHeadings`: <= 3 leading spaces, outside fences, levels 2-4). gsdmm reads `progress.total_phases` from STATE.md (src/state_reader/state_md.rs:314), which upstream writes, so that number arrives through upstream's writer. But gsdmm's own ROADMAP phase list has the same heading reader and it diverges: `phase_heading_re` (roadmap_md.rs:957-965) accepts any leading whitespace, and `parse_phases_in_region` (:530-596) has no fence handling, so a fenced or four-space-indented `### Phase 9:` (and a fenced checklist entry) becomes a phase. Upstream's milestone phase-id scan already used tokenizeHeadings and stripFencedCode. `parse_phase_goals` (:1167) and `roadmap_milestones` (:1904, its own phase-heading regex at :1922) have the same gap. The tree already ports upstream's tokenizer pieces as `atx_heading` (:1364, <= 3 spaces) and `fenced_line_mask` (:1393, used by `roadmap_progress`). PORT, Task 2.
- Decimal `02.1` vs `2.1` dedup in the milestone phase-id scan (#5895 / PR #6252, f6408e608; first-seen spelling kept, custom ids keep their identity). gsdmm already keys `merge_duplicate_phases` (roadmap_md.rs:658) and milestone `scoped_phases` by `phase_num::phase_key` (pad-insensitive, first-seen spelling kept; non-numeric ids keyed raw). Already equivalent, no change.
- Requirements traceability Status read by leading word (#6238 / PR #6293): gsdmm reads no REQUIREMENTS.md traceability table (REQUIREMENTS.md appears only in fixtures and archive file lists). No surface.
- Project-coded / unpadded phase dirs resolve `NN-UAT.md` / `NN-VERIFICATION.md` (#5931 / PR #6263, fc7b7cfef; src/verification.cts `resolvePhaseArtifactFile` :1692-1727 tries `<extractPhaseToken(dir)>-<name>`, then `<normalizePhaseName(token)>-<name>`, then the phase-scoped sorted-first fallback (#3511), then the bare name). gsdmm's `read_verification_status` (disk_status.rs:791) and `read_uat_status` (:854) read the plain sorted-first of every `*-VERIFICATION.md` / `*-UAT.md` (bare name included), citing an old upstream line. They lack even the token-first step (#3492), so `03-CORRECTION-VERIFICATION.md` outranks `03-VERIFICATION.md` (the #3357 case), and a bare `VERIFICATION.md` outranks `XR-03-VERIFICATION.md`. The tree already ports `extractPhaseToken` as `upstream_dir_token` (:1525), `stripProjectCodePrefix` as `strip_project_code_prefix` (:1517) and a partial `normalizePhaseName` as `upstream_normalize_phase_name` (:1495). PORT, Task 2 (token-first + both spellings + dashed-before-bare); the #3511 phase scoping of the fallback is not part of v1.22.0's delta and goes to "Next sync" (I-ikn-7).
- Plans gain `executor_runtime` frontmatter (#6302 / PR #6307; phase-plan-index emits it, verify plan-structure refuses it with `autonomous: false`, `checkpoint:*` or `agent_hint`). gsdmm's PlanMeta reads `status`, `estimate.tokens`, `wave` and the title only (disk_status.rs:1166-1199), and upstream dispatches nothing on the field yet. No surface.
- `audit-uat` `summary.files_scanned` (#6231 / PR #6294), STATE.md over 3x its 100-line target as health warning W031 and the 240-char quick-row warning (#5830 / PR #5882): gsdmm runs no audit-uat or health check and writes no STATE.md. No surface. (This repository's own .planning/STATE.md is 609 lines, so upstream's `/gsd-health` will now warn W031 here. That is process, not product, and stays out of this task.)
- Also no surface: `state record-session` keeps `total_plans` after a milestone close (#6279, an upstream writer); verification scaffolds written without `status` (#5398 — gsdmm already reads a status-less report as Missing); lazy module loading (#5865 / PR #6098); secret-read guard, symlinked-install, quick-batch and workstream-scoping fixes (all upstream runtime or installer behavior).
- Routing: `src/smart-entry.cts` no diff; `src/init.cts` changes are planningDir scoping of MILESTONES.md / research_dir, a new `state_path`, and the shared 240-char cap. The planner ran `cargo test --locked --test driver_router_conformance` against installed 1.22.0: 8 passed.
- CLI: `commands/gsd` has 72 files at both tags, 17 modified, none added or removed, no `argument-hint` line changed; the slash flags this app emits (`--gaps`, `--gaps-only`, `--next`, `--advanced`, `--backlog`) are all still present. `agents/`: 64 files at both tags, 20 modified.
</objective>

<execution_context>
@~/.claude/gsd-core/workflows/execute-plan.md
@~/.claude/gsd-core/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md (frontmatter and Current Position only)
@./CLAUDE.md
@docs/GSD-CORE-SYNC.md (1155 lines. Header 1-76, recipe 80-157, the v1.21.0 delta 161-253 is the shape analog for the new section, Secret-bearing keys 720-786, Modelled 790-1043 with Execution 851-881, Docs & Output 883-893 and Misc ~935, Pass-through 1045-1075, Next sync 1093-1155)
@src/state_reader/config_json.rs (constants 1-58; WorkflowConfig ~560-610, `agent_hint_routing` at 596; workflow parse tests ~1510-1625; `the_gsd_core_sync_baseline_is_recorded` ~1774)
@src/ui/screens/detail.rs (26k lines: read only these ranges. Execution rows ~12260-12290 with `workflow.agent_hint_routing` at 12273; Docs & Output `planning.commit_docs` 12351; `planning.search_gitignored` 12659; `builtin_default_for_key` 12903 with the `"false"` group ~12928-12942; `set_config_value` 13505 (agent_hint_routing arm 13737); `clear_config_value` 14279 (arm 14662); `mutate_config_entry` 15135 (arm 15311); pass-through walk 12976-13110; `populated_gsd_config` 15803 (workflow block ~15901); `DEFAULTS_OPTION_COUNT` 16002; summary-budget test 16423; claude_orchestration help test ~16457 (analog); pin tables 17383-17435; length asserts ~17585-17610; tie tests 17449 / 17522; hooks read-only round-trip test 17724 (analog); chained loops 17811 / 17865)
@src/state_reader/roadmap_md.rs (182K: read only `parse_roadmap_phases` 281-303, `phase_entry_open` 347, `parse_phases_in_region` 530-596, `phase_heading_re` 948-965, `parse_phase_goals` 1134-1232, `atx_heading` 1364, `fenced_line_mask` 1380-1424, `roadmap_milestones` 1904-2075, the fence tests ~3937-3975)
@src/state_reader/disk_status.rs (163K: read only `read_verification_status` 772-801, `read_uat_status` 850-862, the name-collection loop 1029-1235, `phase_dir_matches` / `pad2` / `upstream_normalize_phase_name` / `strip_project_code_prefix` / `upstream_dir_token` 1440-1600, tests ~1948-2140 incl. `test_two_verification_artifacts_always_yield_the_sorted_first` 2106)
@README.md (lines 15-19)
@scripts/install-conformance-oracle.sh (read only; VERSION_DECL grep ~138)

Upstream sources to read (read-only, against the tag):
- `git -C ~/projects/node/gsd-core show v1.22.0:src/verification.cts | sed -n 1692,1727p` (resolvePhaseArtifactFile)
- `git -C ~/projects/node/gsd-core show v1.22.0:src/phase-id.cts | sed -n 536,569p` (stripProjectCodePrefix, normalizePhaseName)
- `git -C ~/projects/node/gsd-core show abc94ae5a -- src/state.cts src/validate.cts` (#5880) and `git -C ~/projects/node/gsd-core show v1.22.0:src/markdown-sectionizer.cts | sed -n 432,505p` (tokenizeHeadings)

Analogs:
- 3d13fd1a "feat(quick-261113-fi0): model hooks.secret_read_exempt as a read-only row (gsd-core v1.21.0)": new-key modelling across config_json.rs, detail.rs, GSD-CORE-SYNC.md.
- 715801c2 "fix(quick-261113-fi0): align the claude_orchestration help texts with gsd-core v1.21.0": help-text alignment with a help-text test.
- 1ac98325 "chore(sync): move the gsd-core baseline to v1.21.0 (quick-261113-fi0)": baseline bump of config_json.rs, README.md, GSD-CORE-SYNC.md; .planning/quick/261113-fi0-sync-against-gsd-core-v1-21-0/261113-fi0-PLAN.md is this plan's shape analog.
- For a typed editable workflow Bool row with all arms: `workflow.agent_hint_routing` (every place listed above) and `("audit.enabled", "bool")` in RESYNCED_KEYS_1_16_0.

Package legitimacy audit (for the oracle install in Task 3's verify; no dependency is added to Cargo.toml):

| Package | Version | Source evidence | Verdict |
|---|---|---|---|
| @opengsd/gsd-core | 1.22.0 (exact pin from GSD_CORE_SYNCED_VERSION) | npm repository `open-gsd/gsd-core`; npm `gitHead` 1f800effd5c33c155c57ad89ca7ec620f4b56035 equals the upstream `v1.22.0` tag sha; the project's existing conformance oracle, already installed on this machine at 1.22.0 | [VERIFIED] |

Coverage audit (operator task -> plan):

| Source item | Where |
|---|---|
| Baseline trio, README note, delta section, intro paragraph, header, recipe REF=v1.23.0/PREV=v1.22.0, all in one commit | Task 3 |
| `workflow.cross_runtime_execution` (scalar -> typed row, measured since, count bump, RESYNCED_KEYS_1_22_0, secret check) | Task 1 commit A |
| `execute.models.<runtime>` (map -> pass-through) | Task 1 commit A (record), Task 3 (delta) |
| `workflow.agent_hint_routing` | verdict: already modelled, no change (I-ikn-3); Task 3 records it |
| `commit_docs` / `search_gitignored` flat vs `planning.` | Task 1 commit B (help texts); no resolver in gsdmm (I-ikn-4) |
| `executor_runtime`, `resolve_model_ids: "omit"`, requirements Status note, `files_scanned`, W031 | no gsdmm surface; Task 3 records each |
| total_phases / real ATX headings | Task 2 commit A |
| decimal `02.1` vs `2.1` dedup | already equivalent; Task 3 records it |
| project-coded / unpadded UAT and VERIFICATION resolver | Task 2 commit B |
| gsd-tools audit, secret re-diff | Task 3 Step 0 and delta |
| conformance vs installed and npm-pinned 1.22.0 | Task 3 Step 5 and the verification block |
| gates, commits, no push, no tag | every task, verification block |

Inferred decisions, recorded for audit (the human is unavailable):
- [I-ikn-1, inferred — audit] `workflow.cross_runtime_execution` gets a typed EDITABLE Bool row (`Option<bool>` in WorkflowConfig, Execution category, directly after `workflow.agent_hint_routing` as upstream's table orders them, built-in default `"false"`, since `v1.22.0`), like `workflow.agent_hint_routing`. Upstream's `config-set` accepts it and nothing dispatches on it yet; the help text says so. A typed row with honest help beats a `Not modelled` row for a key `config-set` writes.
- [I-ikn-2, inferred — audit] `execute.models.<runtime>` stays pass-through: it is templated by runtime name, like `model_profile_overrides.<runtime>.<tier>`, so a typed field per runtime is not expressible. The top-level `execute` object lands in GsdConfig's extra map and shows as one read-only `execute` row, unmasked (a model id is not a secret and trips no marker), preserved on save. It joins the existing `model_policy.*` / `model_profile_overrides.<runtime>.<tier>` Pass-through row.
- [I-ikn-3, inferred — audit] `workflow.agent_hint_routing` needs no change: it is typed since 260916-vqw, its help still describes it, and its built-in default `"true"` matches what new-project configs now write.
- [I-ikn-4, inferred — audit] gsdmm models `commit_docs` and `planning.commit_docs` (and the `search_gitignored` pair) as distinct rows on purpose (config_json.rs ~182) and has no reader that resolves an effective value, so there is no resolver to port. The two namespaced rows' help texts are rewritten to say the namespaced spelling is read only when the flat key is unset (the flat key wins), which v1.22.0 documents (#5470). `planning.sub_repos` keeps its text: there the nested value wins and its text says gsd-core reads that path.
- [I-ikn-5, inferred — audit] The #5880 port applies upstream's tokenizer rule (<= 3 leading spaces, outside fenced code, levels 2-4) to `phase_heading_re` (shared by the phase list, goals, sections and entry boundaries), to `parse_phases_in_region` (headings AND checklist entries AND plan items inside fences are ignored, as upstream's scan strips fenced code before its bullet pass), to `parse_phase_goals` and to `roadmap_milestones`, so the four ROADMAP readers agree. `build_heading_re` (a local, display-only extension) and `any_heading_re` stay as they are. `total_phases` itself is read from STATE.md, which upstream writes.
- [I-ikn-6, inferred — audit] Decimal-phase dedup (#5895) needs no port: `phase_key` already makes `02.1` and `2.1` one phase and keeps the first-seen spelling.
- [I-ikn-7, inferred — audit] The VERIFICATION/UAT port brings gsdmm to `resolvePhaseArtifactFile`'s order minus one tier: token-first (#3492, the prerequisite the v1.22.0 fix extends), then the code-stripped zero-padded spelling (#5931), then the sorted-first dashed report, then the bare name. The phase-scoped fallback (#3511, which drops another phase's stray report from the sorted-first tier) predates v1.22.0's delta and is NOT ported; it becomes a "Next sync" item.
- [I-ikn-8, inferred — audit] No surface, recorded only: `executor_runtime`, `resolve_model_ids: "omit"`, requirements Status notes, `audit-uat` `files_scanned`, W031 and the quick-row warning, `total_plans` after milestone close, status-less verification scaffolds, lazy module loading, secret-read guard / installer / quick-batch / workstream fixes.
- [I-ikn-9, inferred — audit] No code change for the gsd-tools invocation audit (all five invocations unchanged and valid at v1.22.0) and no change to NON_SECRET_CONFIG_KEYS (heuristic matches 6 -> 6, already exempt).
- [I-ikn-10, inferred — audit] The conformance test runs against both the installed oracle and an npm-pinned oracle provisioned by the CI script into a throwaway HOME. `./scripts/pre-tag-check.sh --container vX.Y.Z` stays a release-time obligation: it needs a release tag.
- [I-ikn-11, inferred — audit] The recipe moves to `REF=v1.23.0` / `PREV=v1.22.0` (the next minor; a hotfix tag `v1.22.1` would be measured the same way). The scratch-clone fallback paragraph stays, noting that v1.22.0 did not need it because the sibling checkout carried the tag.
</context>

<tasks>

<task type="tracer" tdd="true">
  <name>Task 1: Model workflow.cross_runtime_execution end to end (gsd-core v1.22.0), record execute.models as pass-through, then align the namespaced commit_docs / search_gitignored help texts</name>
  <files>src/state_reader/config_json.rs, src/ui/screens/detail.rs, docs/GSD-CORE-SYNC.md</files>
  <behavior>
    - Test `workflow_cross_runtime_execution_is_one_typed_bool_row_that_round_trips` (detail.rs tests, next to the hooks read-only test ~17724): parse `{"workflow":{"cross_runtime_execution":true}}` with `parse_gsd_config`. `build_defaults_entries` yields exactly ONE row keyed `workflow.cross_runtime_execution`; its kind is `ConfigValueKind::Bool`, its category is `Execution` (not `PASSTHROUGH_CATEGORY`), its value is `true`, its help `since` is `v1.22.0`, and its index is the index of `workflow.agent_hint_routing` plus one. `mutate_config_entry(&mut config, "workflow.cross_runtime_execution", &ConfigValueKind::Bool)` flips it, and after `serialize_gsd_config` the pointer `/workflow/cross_runtime_execution` is `false`. With `{}` the row exists, is not a pass-through row, and `builtin_default_for_key("workflow.cross_runtime_execution")` is `Some("false")`.
    - Test `execute_models_is_an_unmasked_pass_through_row`: parse `{"execute":{"models":{"codex":"gpt-5.5-codex"}}}`. There is exactly one `PASSTHROUGH_CATEGORY` ReadOnly row keyed `execute`; its value contains `gpt-5.5-codex` and does not contain `MASKED_SECRET`; after mutating an unrelated row (`research`, Bool) and serializing, pointer `/execute/models/codex` is still `gpt-5.5-codex`.
    - config_json.rs: the existing workflow parse fixture (~1518) round-trips `cross_runtime_execution`; the all-unset test (~1616-1623) asserts it is `None` and not serialized.
    - Test `namespaced_commit_docs_and_search_gitignored_help_says_the_flat_key_wins` (commit B): in `all_config_entries()`, the `planning.commit_docs` and `planning.search_gitignored` summaries each contain "flat" and "unset" (case-insensitive is fine). Both stay within the 160-char / 4-novel-words summary test.
    - Census tests stay green: `every_resynced_key_has_one_row_of_the_right_kind_with_a_measured_since`, `every_editable_resynced_key_sets_toggles_and_clears`, `every_config_entry_carries_a_non_empty_summary` (count 140), `every_summary_is_within_the_pane_budget_and_is_not_a_restatement_of_the_key`, `no_authored_row_trips_the_secret_heuristic_unmasked`, `an_unmodelled_key_in_every_nested_block_becomes_a_visible_pass_through_row`, `the_sync_record_names_every_modelled_key_and_the_measured_baseline`.
  </behavior>
  <action>
RED first: write the commit-A behavior tests, run them and see them fail, then implement. Make TWO path-scoped commits. Finish and commit A before you start B, so no hunk has to be split out of detail.rs afterwards.

Commit A, the new keys (per I-ikn-1, I-ikn-2):
1. src/state_reader/config_json.rs, `WorkflowConfig` (~560-610). After the last field of the struct's newest marker group, add a marker comment `// --- gsd-core v1.22.0 (#6302, quick task 261119-ikn) ---` and a field `cross_runtime_execution: Option<bool>` with `#[serde(default, skip_serializing_if = "Option::is_none")]` and a one-line doc: the opt-in gate for dispatching a plan that declares `executor_runtime:` to another runtime's CLI, registered upstream with nothing reading it yet. Extend the workflow parse fixture (~1518) with `"cross_runtime_execution": true` and assert it in both the parse and the round-trip assertions (~1557, ~1594); extend the all-unset assertions (~1616-1623). Do NOT touch the three constants in this task.
2. src/ui/screens/detail.rs.
   - Row: directly after the `workflow.agent_hint_routing` push (~12273-12276), push `workflow.cross_runtime_execution` with `opt_bool(pwf.and_then(|w| w.cross_runtime_execution))`, editable flag `false` like its neighbor. Help: one `ConfigHelp::new` summary of at most 160 chars with at least 4 words not in the key, e.g. "Opt-in gate for running a plan that names executor_runtime on another vendor's CLI; gsd-core 1.22.0 registers it, but nothing dispatches on it yet." Chain `.since("v1.22.0")`. Add a one-line comment naming gsd-core v1.22.0, #6302 and quick task 261119-ikn.
   - Arms, each mirroring the `workflow.agent_hint_routing` arm in the same function: `builtin_default_for_key` (add the key to the `"false"` group, ~12928-12942), `set_config_value` (~13737), `clear_config_value` (~14662), `mutate_config_entry` (~15311, toggle with `unwrap_or(false)`).
   - Fixture `populated_gsd_config` (~15803): add `"cross_runtime_execution": true` to its `"workflow"` block next to `"agent_hint_routing"` (~15901).
   - `DEFAULTS_OPTION_COUNT` (16002): 139 -> 140.
   - Pins: after `RESYNCED_KEYS_1_21_0` (17416) add `const RESYNCED_KEYS_1_22_0: &[(&str, &str)] = &[("workflow.cross_runtime_execution", "bool")];` with a doc comment in the style of its neighbor: MEASURED key-cell diff v1.21.0..v1.22.0 (+2 / -0), e99b85854, #6302 / PR #6307; the other cell, `execute.models.<runtime>`, is templated by runtime and stays pass-through. Add its length assert (1, same message style, saying +2 cells of which one is a typed key) after the v1.21.0 one (~17597). Chain it after `RESYNCED_KEYS_1_21_0` in BOTH loops (~17819 and ~17873).
   - Write the two commit-A tests from the behavior block.
3. docs/GSD-CORE-SYNC.md (same commit; the tie tests require it).
   - `### Execution` table (851-881): add a row whose key cell is `workflow.cross_runtime_execution` in backticks and whose since cell is v1.22.0 directly after the `workflow.agent_hint_routing` row, matching the display order. Under the table add one sentence: gsd-core v1.22.0 registers it as an opt-in gate (default false) that nothing reads until its cross-runtime dispatch ships.
   - `## Modelled` opening paragraph (792-809): 139 -> **140 rows**. Add "**1** carries `v1.22.0` (261119-ikn)" after the `v1.21.0` clause, before "and the **72** without one". The DEFAULTS_OPTION_COUNT sentence gains "139 -> 140 at 261119-ikn" and pins 140. The pin list adds `RESYNCED_KEYS_1_22_0`, and "seven key lists" becomes "eight key lists".
   - `## Pass-through` table (1058-1075): add `execute.models.<runtime>` to the `model_policy.*` / `model_profile_overrides.<runtime>.<tier>` family cell, and extend its why cell: gsd-core v1.22.0 (#6302) registers `execute.models.<runtime>` for foreign executors and nothing reads it yet; the whole top-level `execute` object shows as one read-only row.
   - Edit nothing else in the record in this commit.
4. Gate (`rtk bypass cargo test --locked --lib`), then commit: `git commit -m "feat(quick-261119-ikn): model workflow.cross_runtime_execution as a typed row (gsd-core v1.22.0)" -- src/state_reader/config_json.rs src/ui/screens/detail.rs docs/GSD-CORE-SYNC.md`. Use the session's attribution trailer lines. Never `git add -A`.

Commit B, the namespaced help texts (per I-ikn-4):
5. RED: write `namespaced_commit_docs_and_search_gitignored_help_says_the_flat_key_wins` and see it fail.
6. src/ui/screens/detail.rs: replace only the `ConfigHelp::new` summary of the `planning.commit_docs` row (12351-12354) and of the `planning.search_gitignored` row (12659-12662). Kinds, arms, `since` and the comment above the commit_docs pair stay. Suggested wording for both, which you may adjust to fit the summary test: "The namespaced form of the switch above; gsd-core reads it only when the flat key is unset, so the flat key wins when both are set." The flat rows' texts stay.
7. docs/GSD-CORE-SYNC.md `### Docs & Output` (883-893): under its table add one sentence: `commit_docs` / `planning.commit_docs` and `search_gitignored` / `planning.search_gitignored` are modelled as distinct paths; upstream reads the flat key first and the namespaced one only when the flat key is unset, which gsd-core v1.22.0 documents (#5470), and the namespaced rows' help says so (261119-ikn).
8. Gate, then commit: `git commit -m "fix(quick-261119-ikn): say the flat commit_docs and search_gitignored keys win (gsd-core v1.22.0)" -- src/ui/screens/detail.rs docs/GSD-CORE-SYNC.md`. Attribution trailer lines; never `git add -A`.
  </action>
  <verify>
    <automated>rtk bypass cargo test --locked --lib</automated>
    <automated>rtk bypass bash -c 'set -e; test "$(grep -c cross_runtime_execution src/state_reader/config_json.rs)" -ge 3; test "$(grep -cE "const DEFAULTS_OPTION_COUNT: usize =[[:space:]]*140;" src/ui/screens/detail.rs)" -ge 1; test "$(grep -cF RESYNCED_KEYS_1_22_0 src/ui/screens/detail.rs)" -ge 4; for f in workflow_cross_runtime_execution_is_one_typed_bool_row_that_round_trips execute_models_is_an_unmasked_pass_through_row namespaced_commit_docs_and_search_gitignored_help_says_the_flat_key_wins; do grep -q "fn $f" src/ui/screens/detail.rs || { echo "MISSING $f"; exit 1; }; done; test "$(grep -cF "| \`workflow.cross_runtime_execution\` | v1.22.0 |" docs/GSD-CORE-SYNC.md)" -ge 1; awk "/^## Pass-through/{f=1;next} /^## /{f=0} f" docs/GSD-CORE-SYNC.md | grep -qF "execute.models.<runtime>"; test "$(grep -cF "gsd-core reads this path, and a project may carry either one" src/ui/screens/detail.rs)" -eq 0; echo T1-OK'</automated>
  </verify>
  <done>
- `cargo test --locked --lib` exits 0; the three new tests and the census tests in the behavior block pass. Gate baselines at HEAD 682b8af1: config_json.rs `cross_runtime_execution` [HEAD=0 EXPECT>=3]; `DEFAULTS_OPTION_COUNT: usize = 140;` [HEAD=0 EXPECT>=1]; `RESYNCED_KEYS_1_22_0` [HEAD=0 EXPECT>=4]; the three test fns [HEAD=0 EXPECT>=1 each]; doc Execution row [HEAD=0 EXPECT>=1]; `execute.models.<runtime>` inside Pass-through [HEAD=0 EXPECT>=1]; the old shared namespaced summary phrase in detail.rs [HEAD=2 EXPECT=0]. T1-OK printed.
- Two commits: A touches exactly config_json.rs, detail.rs and GSD-CORE-SYNC.md; B touches exactly detail.rs and GSD-CORE-SYNC.md. `git log -2 --stat` confirms.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Port two .planning reader fixes from gsd-core v1.22.0: real ATX phase headings only (#5880), and the phase-token VERIFICATION/UAT resolver (#5931)</name>
  <files>src/state_reader/roadmap_md.rs, src/state_reader/disk_status.rs</files>
  <behavior>
    - roadmap_md.rs test `only_real_atx_phase_headings_open_a_phase_entry`: a ROADMAP with real `## Phase 1: One`, `   ### Phase 2: Three spaces` (3 leading spaces) and `#### Phase 3: Four hashes` headings, plus non-phases: `##### Phase 5: Level five`, `###Phase 6: No space`, `> ### Phase 7: Quoted`, `    ### Phase 8: Indented four` (4 spaces), a ``` fenced block holding `### Phase 9: Example` and `- [ ] **Phase 4: Fenced entry**`, and a `~~~` fenced block holding `## Phase 10: Tilde example`. `parse_roadmap_phases(.., Legacy)` returns exactly the numbers `1`, `2`, `3`. A real phase whose entry contains a fenced `- [x] 01-02-PLAN.md` line next to a real `- [ ] 01-01-PLAN.md` line reports `total_plans == 1`, and the fenced block does not end its entry (a `**Depends on**:` line after the fence is still read).
    - roadmap_md.rs test `a_fenced_example_neither_shadows_a_goal_nor_opens_a_milestone`: a fenced block BEFORE the real entries holds `## v9.9 Example milestone`, `### Phase 1: Example` and `**Goal**: example goal`; the real `### Phase 1: Real` follows with `**Goal**: real goal`. `parse_phase_goals` gives phase `1` the goal `real goal`; `roadmap_milestones` returns no milestone labelled `v9.9 ...`; a real `## v1.0 Real` heading still yields its milestone with phase `1` scoped.
    - disk_status.rs test `a_project_coded_or_unpadded_phase_dir_reads_its_own_reports`: under a tempdir, phase dir `XR-03-demo` (with the plan/summary files the existing `implementation_complete_dir` helper writes, or equivalent) holding `03-CORRECTION-VERIFICATION.md` (`status: gaps_found`), `03-VERIFICATION.md` (`status: passed`), `03-HUMAN-UAT.md` (`status: failed`) and `03-UAT.md` (`status: partial`) yields `VerificationStatus::Passed` and `UatStatus::Partial`; phase dir `5-demo` holding `05-CORRECTION-VERIFICATION.md` (gaps_found) and `05-VERIFICATION.md` (passed) yields `Passed`.
    - disk_status.rs test `the_phase_token_report_outranks_a_sibling_that_sorts_first`: phase dir `03-demo` with `03-CORRECTION-VERIFICATION.md` (gaps_found) and `03-VERIFICATION.md` (passed) yields `Passed` (#3357 / #3492); phase dir `XR-03-demo` with `XR-03-VERIFICATION.md` (passed) and `03-VERIFICATION.md` (gaps_found) yields `Passed` (the literal token wins over the normalized spelling).
    - disk_status.rs test `a_bare_report_is_read_only_when_no_dashed_report_exists`: phase dir `XR-03-demo` with bare `VERIFICATION.md` (gaps_found) and `XR-03-VERIFICATION.md` (passed) yields `Passed`; a dir holding only a bare `VERIFICATION.md` (passed) still yields `Passed`.
    - Existing tests stay green, notably `test_two_verification_artifacts_always_yield_the_sorted_first` (a tempdir name yields no token that names a listed report, so the sorted-first rule still applies) and the fence tests ~3937-3975.
  </behavior>
  <action>
RED first for each half, then implement. Make TWO path-scoped commits, one per file. Re-read the three upstream sources named in the context block before writing code; port behavior, not prose.

Commit A, ROADMAP headings (per I-ikn-5), src/state_reader/roadmap_md.rs:
1. `phase_heading_re` (948-965): its line anchor accepts at most three leading spaces (CommonMark ATX, the rule `atx_heading` at 1364 already ports from upstream's `tokenizeHeadings`), no tab or deeper indent, and the gap after the hashes is spaces or tabs. Keep the rest of the pattern (the id slot, the optional parenthetical, the colon or spaced-dash separator) byte for byte. Extend its doc comment: only real level 2-4 ATX headings open an entry — not a fenced, four-space-indented, blockquoted, `#####` or no-space line — a port of gsd-core #5880 / PR #6291 (v1.22.0). The regex is shared by `parse_roadmap_phases`, `parse_phase_goals`, `phase_section` and `is_entry_line`, so all four change together.
2. `parse_phases_in_region` (530-596): compute `fenced_line_mask(&lines)` once beside `closed`. In the outer loop a fenced line never opens an entry. In the inner scan a fenced line is skipped BEFORE the entry-boundary check, so it neither ends the entry nor counts as a plan item nor supplies the dependency line. This mirrors upstream's milestone phase-id scan, which reads headings through `tokenizeHeadings` and bullets after `stripFencedCode`. Add a bullet to the `parse_roadmap_phases` doc list (281-300) saying lines inside fenced code are ignored (gsd-core #5880).
3. `parse_phase_goals` (1167-1232): iterate with the mask. When no goal continuation is pending, a fenced line is skipped, so a fenced example can neither open an entry nor supply a goal. A pending continuation still ends at a fence opener through `ends_field_continuation`, as today.
4. `roadmap_milestones` (1904-2075): iterate with the mask and skip fenced lines entirely, and give its own phase-heading regex (~1922) the same at-most-three-spaces anchor, so the milestone scope agrees with the phase list.
5. Leave `build_heading_re`, `any_heading_re`, `locate_phase_section`'s own fence tracking and `roadmap_progress` unchanged.
6. Write the two roadmap tests from the behavior block, run `rtk bypass cargo test --locked --lib roadmap_md`, then commit: `git commit -m "fix(quick-261119-ikn): read ROADMAP phase headings from real ATX headings only (gsd-core #5880)" -- src/state_reader/roadmap_md.rs`.

Commit B, report selection (per I-ikn-7), src/state_reader/disk_status.rs:
7. Add one private selection helper used by BOTH `read_verification_status` (791) and `read_uat_status` (854), e.g. `select_phase_report(phase_dir: &Path, names: &[String], bare: &str) -> Option<String>`. It ports `resolvePhaseArtifactFile` (gsd-core v1.22.0 src/verification.cts:1692-1727) without the #3511 scoping tier, in this order:
   (a) dashed candidates: the listed names ending in `-` + `bare`, sorted;
   (b) if any exist and the phase dir's file name is valid UTF-8, the token is `upstream_dir_token(dir_name)` (the port of `extractPhaseToken`, which keeps the project code and the padding as written: `XR-03`, `5`, `03`). Try `<token>-<bare>`, then `<normalized>-<bare>`, where normalized is upstream's `normalizePhaseName(token)` (phase-id.cts:546-569: strip a project-code prefix, pad the leading digit run and each `-N` milestone segment to two digits, keep a letter suffix and the decimal tail, return a custom id unchanged). The first that is a listed candidate wins;
   (c) else the first sorted dashed candidate;
   (d) else `bare` itself, if listed;
   (e) else none.
   Only names from the directory listing are ever opened: never join a path built from the token unless that exact name is in `names`. For the normalizer, reuse `upstream_normalize_phase_name` after `strip_project_code_prefix` when it already matches upstream for every shape the tests and these dirs use. If a shape diverges (a letter suffix on a one-digit major, a deep `M-N-N` id), make that function a faithful port of phase-id.cts:546-569 rather than adding a second normalizer, and confirm the `phase_dir_matches` tests stay green.
8. Rewrite the doc comments of `read_verification_status` and `read_uat_status`: the selection order above, citing #3357 / #3492 (token first) and #5931 / PR #6263 (the scaffolder's code-stripped zero-padded name, gsd-core v1.22.0), naming `resolvePhaseArtifactFile` (src/verification.cts) as the source in place of the stale upstream line citation, and saying the phase-scoped fallback of #3511 is not ported (quick task 261119-ikn, I-ikn-7). The status parsing after selection is unchanged (`verification_status_of`, the byte-zero frontmatter rule, Unparseable).
9. Write the three disk_status tests from the behavior block, run `rtk bypass cargo test --locked --lib disk_status`, then commit: `git commit -m "fix(quick-261119-ikn): resolve a phase's own VERIFICATION and UAT report by its dir token (gsd-core #5931)" -- src/state_reader/disk_status.rs`.

Both commits carry the session's attribution trailer lines. Never `git add -A`. If `tests/driver_router_conformance.rs` turns red after either commit, the port disagrees with upstream: fix the port, never the test.
  </action>
  <verify>
    <automated>rtk bypass cargo test --locked --lib</automated>
    <automated>rtk bypass cargo test --locked --test driver_router_conformance</automated>
    <automated>rtk bypass bash -c 'set -e; test "$(grep -c fenced_line_mask src/state_reader/roadmap_md.rs)" -ge 6; test "$(grep -c "verification.cjs:302-303" src/state_reader/disk_status.rs)" -eq 0; for f in only_real_atx_phase_headings_open_a_phase_entry a_fenced_example_neither_shadows_a_goal_nor_opens_a_milestone; do grep -q "fn $f" src/state_reader/roadmap_md.rs || { echo "MISSING $f"; exit 1; }; done; for f in a_project_coded_or_unpadded_phase_dir_reads_its_own_reports the_phase_token_report_outranks_a_sibling_that_sorts_first a_bare_report_is_read_only_when_no_dashed_report_exists; do grep -q "fn $f" src/state_reader/disk_status.rs || { echo "MISSING $f"; exit 1; }; done; echo T2-OK'</automated>
  </verify>
  <done>
- `cargo test --locked --lib` exits 0 with the five new tests passing; `driver_router_conformance` reports 8 passed against the installed 1.22.0. Gate baselines at HEAD 682b8af1: `fenced_line_mask` occurrences in roadmap_md.rs [HEAD=3 EXPECT>=6]; the stale upstream line citation in disk_status.rs [HEAD=1 EXPECT=0]; the five test fns [HEAD=0 EXPECT>=1 each]. T2-OK printed.
- Two commits: A touches exactly src/state_reader/roadmap_md.rs; B touches exactly src/state_reader/disk_status.rs.
  </done>
</task>

<task type="auto">
  <name>Task 3: Move the gsd-core baseline to v1.22.0 (constants, README, sync record with the measured delta) in one commit, and certify the 1.22.0 oracle</name>
  <files>src/state_reader/config_json.rs, README.md, docs/GSD-CORE-SYNC.md</files>
  <action>
**Step 0: re-measure. Do not copy the planner's numbers blind.** Set CORE=~/projects/node/gsd-core. It carries `v1.22.0`, so no scratch clone is needed; if `git -C "$CORE" rev-parse v1.22.0` fails, use the record's "When the sibling checkout lacks REF" bare-clone recipe instead. Measure ONLY with `git -C "$CORE" show|diff|log|describe|tag --contains|rev-list|ls-tree` against the tags. Never check out, switch, fetch into or write to that checkout: it is on a fork branch with local work. Run the record's recipe steps 1-5 with REF=v1.22.0 and PREV=v1.21.0, plus: `npm view @opengsd/gsd-core version gitHead`; the capability non-version line filter; the secret-heuristic command at both tags; the CLI-SURFACE-MANIFEST.json comparison (verb count, `global`, `undeclared`, changed verbs); `git diff --stat` over commands/gsd, agents, src/smart-entry.cts and src/init.cts; and `git -C "$CORE" describe --tags HEAD` plus that HEAD's package.json version for the recipe parenthetical. Re-run the five gsd-tools invocations from the objective against the installed 1.22.0. Pipe-sensitive counts run inside `rtk bypass bash -c '...'`. Wherever a value differs from the objective's measured list, use yours and say so in the SUMMARY.

**Step 1: src/state_reader/config_json.rs**, following 1ac98325's pattern.
- Set `GSD_CORE_SYNCED_VERSION` = "1.22.0", `GSD_CORE_SYNCED_COMMIT` = "v1.22.0" and `GSD_CORE_SYNCED_TREE_VERSION` = "1.22.0". Each stays one `pub const NAME: &str = "...";` line; the oracle script's grep must keep matching exactly one VERSION line.
- Doc comments: append 261119-ikn to the quick-task list; reword the "At the ... sync (...) the target is a published tag" sentence to name the v1.22.0 sync and 261119-ikn, keeping "so all three constants name the same release"; set the text block's `REF=` to v1.22.0; in the COMMIT comment, name the `v1.22.0` tag and full sha 1f800effd5c33c155c57ad89ca7ec620f4b56035 in place of the previous one.
- From the top of the file through the TREE_VERSION declaration, nothing may name the previous release. No other line changes (Task 1's WorkflowConfig marker stays).

**Step 2: README.md.** In the single `**GSD compatibility:**` paragraph, the last sentence becomes "build is synced to gsd-core 1.22.0 (`v1.22.0`), with oracle pin `1.22.0`." Keep the wrapping, the link and the single marker. Nothing else changes.

**Step 3: docs/GSD-CORE-SYNC.md.**
(a) Header (lines 3-6): synced against gsd-core `v1.22.0` (`1f800effd5c33c155c57ad89ca7ec620f4b56035`, `package.json` 1.22.0, tagged 2026-11-19) on 2026-11-19; pin `1.22.0`, the latest version on npm.
(b) Constants table Value column: `1.22.0` / `v1.22.0` / `1.22.0`, keeping the cell shape. Nothing from the top through the "This file is the human-readable half" line names the previous release.
(c) History paragraph (ends line 55): append one sentence for the v1.22.0 sync (quick task 261119-ikn): it modelled `workflow.cross_runtime_execution` as a typed row, kept `execute.models.<runtime>` pass-through, corrected the namespaced `commit_docs` / `search_gitignored` help, and ported two `.planning` reader fixes (real ATX phase headings, #5880; the phase-token VERIFICATION/UAT resolver, #5931). Link [The v1.22.0 delta](#the-v1220-delta-quick-task-261119-ikn).
(d) Recipe (80-157), per I-ikn-11. In the bash block set `REF=v1.23.0` and `PREV=v1.22.0`, keeping the comment column. Replace the "(at the time of writing: ...)" parenthetical, including its stale describe, with the re-measured sibling HEAD: a fork docs branch at its describe and package.json version, carrying the `v1.22.0` tag (planner saw `v1.21.0-13-g4f9cb0d42`, package.json 1.21.0). After the bare-clone block, the closing sentence says the v1.21.0 sync measured from a clone and the v1.22.0 sync did not need one.
(e) New section `## The v1.22.0 delta (quick task 261119-ikn)`, inserted immediately BEFORE `## The v1.21.0 delta (quick task 261113-fi0)` (line 161) and followed by a `---` line. It must contain no `## Modelled` / `## Pass-through` heading; use bold lead-ins. Content, from Step 0 and the objective's measured list:
   1. Opening: measured on 2026-11-19 between `v1.21.0` and `v1.22.0` (`1f800effd5c33c155c57ad89ca7ec620f4b56035`) against the tags in the sibling checkout, read-only; 74 commits, 482 files; v1.21.0 is an ancestor.
   2. Measurement table (Measurement | v1.21.0 | v1.22.0 | Change): key cells 264 / 266, **+2 / -0** `workflow.cross_runtime_execution` (boolean, default `false`) and `execute.models.<runtime>` (string, target default), both e99b85854 (#6302 / PR #6307, 2026-11-18; first tag `v1.22.0`); schema manifest (+1 `validKeys`, +1 `execute.models` pattern); defaults manifest (+`workflow.cross_runtime_execution: false`) and config template (no diff); `src/config.cts` (the new default, `agent_hint_routing: true` in new-project configs #6211 / PR #6225, config-get loader resolution of `commit_docs` / `search_gitignored` #5470 / PR #5502); `src/config-loader.cts` (`CONFIG_KEY_ALIASES`, flat-then-nested unchanged, no key semantics change); `src/secrets.cts` unchanged; existing defaults changed: **none**; capabilities: 45 version-only.
   3. **How this build models the change**: the typed row (I-ikn-1), Modelled and `DEFAULTS_OPTION_COUNT` 139 -> 140, `RESYNCED_KEYS_1_22_0`; `execute.models.<runtime>` pass-through (I-ikn-2); `workflow.agent_hint_routing` unchanged (I-ikn-3); the namespaced help texts (I-ikn-4). Name the Task 1 commits by short sha.
   4. **Reader ports**: real ATX phase headings (#5880 / PR #6291; gsdmm reads `total_phases` from STATE.md, which upstream writes, and its own phase list, goals and milestone scope now follow the same tokenizer rule, I-ikn-5); the VERIFICATION/UAT resolver (#3492 + #5931 / PR #6263, I-ikn-7, with #3511 not ported); decimal dedup (#5895 / PR #6252) already equivalent (I-ikn-6). Name the Task 2 commits by short sha.
   5. **No gsdmm surface** (I-ikn-8), one bullet each: `executor_runtime`; `resolve_model_ids: "omit"` (#3703 / PR #6217); requirements Status notes (#6238 / PR #6293); `audit-uat` `files_scanned` (#6231 / PR #6294); W031 and the quick-row warning (#5830 / PR #5882); `total_plans` after a milestone close (#6279); status-less verification scaffolds (#5398); lazy module loading (#5865 / PR #6098); secret-read guard, installer, quick-batch and workstream fixes.
   6. **The gsd-tools invocation audit**: manifest facts at v1.22.0 (94 verbs, `global` and `undeclared` unchanged, `smart-entry` and `init.manager` still undeclared, the three changed verbs none of which is called here) and a table of the five invocations with file:line and result, as in the v1.21.0 section. Conclusion: no change (I-ikn-9).
   7. **Other areas**: Routing (smart-entry.cts no diff; init.cts changes; `tests/driver_router_conformance.rs` green against installed 1.22.0 and the npm-pinned oracle, citing this task's SUMMARY); The CLI (72 commands, 17 modified, no `argument-hint` change, emitted slash flags still present); Agents (64 files, 20 modified); Codex and installs (capabilities version-only; the `<config home>/gsd-core/VERSION` root `src/state_reader/gsd_install.rs` reads is unchanged); Secret-bearing keys (secrets.cts and gsd-run-resolver.md no diff; heuristic matches 6 -> 6; the new key names trip no marker).
(f) Next sync. Step 1's parenthetical becomes (`REF=v1.23.0`, `PREV=v1.22.0`), agreeing with (d). Append item 9, **Phase-scoped report fallback (deferred at 261119-ikn)**: port gsd-core #3511's scoping of `resolvePhaseArtifactFile`'s sorted-first tier (`scopeToPhase`), so another phase's stray `NN-VERIFICATION.md` in a directory can no longer become this phase's report. Item 5's row count stays equal to the Modelled paragraph's count of rows without a `since` (72). Leave the other items alone.
(g) Do NOT edit `## Modelled`, Pass-through, Secret-bearing keys (Task 1 owns the first two) or any older delta section.

**Step 4: gate, then ONE commit** with all three files: `git commit -m "chore(sync): move the gsd-core baseline to v1.22.0 (quick-261119-ikn)" -- src/state_reader/config_json.rs README.md docs/GSD-CORE-SYNC.md`. The body summarizes the delta and lists I-ikn-1..11, followed by the session's attribution trailer lines. Do not edit src/ui/screens/detail.rs, the two readers, scripts/, .github/, Cargo.toml or .planning/STATE.md in this task. No version bump, no tag, no push.

**Step 5: certify the oracle** (verify block 4). It provisions the exact CI path into a throwaway HOME and runs the router conformance test against it. If npm or the network is unavailable, record a SKIP with its error in the SUMMARY, rely on the installed-oracle run in the full suite, and do not loosen any gate.
  </action>
  <verify>
    <automated>rtk bypass cargo test --locked --lib -- the_gsd_core_sync_baseline_is_recorded the_sync_record_names_every_modelled_key_and_the_measured_baseline the_readme_compatibility_note_names_the_synced_baseline the_sync_record_names_every_secret_key_and_exemption</automated>
    <automated>rtk bypass bash -c 'set -e; F=src/state_reader/config_json.rs; DECL="^[[:space:]]*pub const[[:space:]]+GSD_CORE_SYNCED_VERSION[[:space:]]*:[[:space:]]*&str[[:space:]]*="; test "$(grep -E "$DECL" $F | head -1 | sed -E "s/.*\"(.*)\".*/\1/")" = "1.22.0"; test -z "$(grep -E "$DECL" $F | sed -n 2p)"; test "$(grep -E "^pub const GSD_CORE_SYNCED_COMMIT: &str =" $F | sed -E "s/.*\"(.*)\".*/\1/")" = "v1.22.0"; test "$(grep -E "^pub const GSD_CORE_SYNCED_TREE_VERSION: &str =" $F | sed -E "s/.*\"(.*)\".*/\1/")" = "1.22.0"; test "$(grep -c 1f800effd5c33c155c57ad89ca7ec620f4b56035 $F)" -ge 1; test "$(grep -c 357508df233ccca0dcf4332b385d6bee9b113da9 $F)" -eq 0; TOP="$(awk "{print} /^pub const GSD_CORE_SYNCED_TREE_VERSION/{exit}" $F)"; test "$(printf "%s\n" "$TOP" | grep -c "1\.21\.0")" -eq 0; test "$(printf "%s\n" "$TOP" | grep -c 261119-ikn)" -ge 1; test "$(printf "%s\n" "$TOP" | grep -c "REF=v1\.22\.0")" -ge 1; test "$(grep -cF "gsd-core 1.22.0 (\`v1.22.0\`), with oracle pin \`1.22.0\`" README.md)" -ge 1; test "$(grep -c "1\.21\.0" README.md)" -eq 0; grep -qF "GSD compatibility:" README.md; test -z "$(grep -F "GSD compatibility:" README.md | sed -n 2p)"; echo CONSTANTS-README-OK'</automated>
    <automated>rtk bypass bash -c 'set -e; D=docs/GSD-CORE-SYNC.md; H="$(awk "/^This file is the human-readable half/{exit} {print}" $D)"; test "$(printf "%s\n" "$H" | grep -c "1\.21\.0")" -eq 0; printf "%s\n" "$H" | grep -qF 1f800effd5c33c155c57ad89ca7ec620f4b56035; printf "%s\n" "$H" | grep -qF 2026-11-19; grep -qE "^\| \`GSD_CORE_SYNCED_VERSION\` \| \`1\.22\.0\` \|" $D; grep -qE "^\| \`GSD_CORE_SYNCED_COMMIT\` \| \`v1\.22\.0\` \|" $D; grep -qE "^\| \`GSD_CORE_SYNCED_TREE_VERSION\` \| \`1\.22\.0\` \|" $D; grep -qE "^REF=v1\.23\.0 " $D; grep -qE "^PREV=v1\.22\.0 " $D; test "$(grep -cE "^PREV=v1\.21\.0 " $D)" -eq 0; grep -qF "(\`REF=v1.23.0\`, \`PREV=v1.22.0\`)" $D; test "$(grep -c "v1\.20\.1-97-gd668c307b" $D)" -eq 0; grep -qF "#the-v1220-delta-quick-task-261119-ikn" $D; N1="$(grep -nxF "## The v1.22.0 delta (quick task 261119-ikn)" $D | cut -d: -f1)"; N0="$(grep -nxF "## The v1.21.0 delta (quick task 261113-fi0)" $D | cut -d: -f1)"; test -n "$N1"; test -n "$N0"; test "$N1" -lt "$N0"; for t in 1f800effd5c33c155c57ad89ca7ec620f4b56035 2026-11-19 264 266 workflow.cross_runtime_execution "execute.models" "#6302" 140 RESYNCED_KEYS_1_22_0 agent_hint_routing "#5470" commit_docs "#5880" total_phases "#5895" "#5931" "#3511" "#6238" executor_runtime files_scanned W031 resolve_model_ids CLI-SURFACE-MANIFEST.json smart-entry init.manager runtime-identity --version gsd_install.rs secrets.cts driver_router_conformance.rs; do awk "/^## The v1\\.22\\.0 delta/{f=1;next} /^## /{f=0} f" $D | grep -qF -- "$t" || { echo "DELTA-MISSING $t"; exit 1; }; done; awk "/^## Next sync/{f=1;next} f" $D | grep -qF "#3511"; grep -q "^## Modelled" $D; test -z "$(grep "^## Modelled" $D | sed -n 2p)"; grep -q "^## Pass-through" $D; test -z "$(grep "^## Pass-through" $D | sed -n 2p)"; echo RECORD-OK'</automated>
    <automated>T=$(mktemp -d) && mkdir -p "$T/home" && env -u CLAUDE_CONFIG_DIR -u CODEX_HOME HOME="$T/home" bash scripts/install-conformance-oracle.sh && CLAUDE_CONFIG_DIR="$T/home/.claude" rtk bypass cargo test --locked --test driver_router_conformance; rc=$?; rm -rf "$T"; exit $rc</automated>
  </verify>
  <done>
- The four tie tests pass. CONSTANTS-README-OK and RECORD-OK are printed. Baselines at HEAD 682b8af1:
  - VERSION/COMMIT/TREE read the previous release [HEAD fails EXPECT 1.22.0 / v1.22.0 / 1.22.0].
  - In config_json.rs: new sha [HEAD=0 EXPECT>=1]; previous sha [HEAD=1 EXPECT=0]; previous-release hits above TREE_VERSION [HEAD=6 EXPECT=0]; `261119-ikn` and `REF=v1.22.0` there [HEAD=0 EXPECT>=1].
  - README: new sentence [HEAD=0 EXPECT>=1]; previous-release count [HEAD=1 EXPECT=0].
  - Doc header: previous-release hits [HEAD=6 EXPECT=0]; new sha [HEAD=0 EXPECT>=1].
  - Doc recipe and Next sync: constants-table rows [HEAD=0 EXPECT>=3]; `REF=v1.23.0` [HEAD=0 EXPECT>=1]; `PREV=v1.22.0` [HEAD=0 EXPECT>=1]; old `PREV` line [HEAD=1 EXPECT=0]; Next-sync parenthetical [HEAD=0 EXPECT>=1]; stale describe [HEAD=1 EXPECT=0]; anchor [HEAD=0 EXPECT>=1]; delta heading [HEAD=absent EXPECT before line N0=161]; `#3511` in Next sync [HEAD=0 EXPECT>=1].
  - Invariants that hold by design: a single VERSION declaration, a single README marker, one `## Modelled` and one `## Pass-through`.
- The oracle certification prints `ORACLE READY` with `action : installed @opengsd/gsd-core@1.22.0` and the conformance test reports `8 passed`. A network SKIP, if any, is recorded per Step 5.
- One commit touching exactly src/state_reader/config_json.rs, README.md and docs/GSD-CORE-SYNC.md.
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| project config.json -> Defaults tab render and save | Hand-edited, untrusted values are drawn in the TUI and written back; secret classification decides what is masked. |
| project ROADMAP.md / phase directories -> state readers | Untrusted text and file names decide the phase list, goals, milestone scope and which report file is read; their result drives routing. |
| npm registry / upstream repo -> oracle pin | `GSD_CORE_SYNCED_VERSION` decides which `@opengsd/gsd-core` CI installs and executes in the job holding the crates.io token. |
| config_json.rs declaration -> CI shell script | install-conformance-oracle.sh parses the Rust source text of one `pub const` line. |
| sync record -> future syncer | docs/GSD-CORE-SYNC.md is trusted as the measured baseline the next sync diffs from. |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-ikn-01 | Tampering | `workflow.cross_runtime_execution` editable row | low | mitigate | It writes only the documented boolean on an explicit keypress, its built-in default is false, and its help says what the gate will do (spend a second vendor's subscription, outside the host's hooks) once upstream dispatch ships. The round-trip test pins that the save path writes exactly that pointer. |
| T-ikn-02 | Information disclosure | `execute` pass-through row | low | mitigate | A model id is not a secret and `execute.models.<runtime>` trips no marker; `display_config_value` still runs `redact_json_for_display`, so a secret-named member nested in the object stays masked. The new test pins the unmasked model id and `MASKED_SECRET` absence. |
| T-ikn-03 | Denial of service | `phase_heading_re`, the milestone phase regex, the fence mask | low | mitigate | The `regex` crate is linear-time (no backtracking); the anchor change narrows the pattern; `fenced_line_mask` is one O(n) pass already used by `roadmap_progress`. |
| T-ikn-04 | Tampering / Information disclosure | `select_phase_report` | medium | mitigate | A report is chosen only from the names `read_dir` listed for that directory and is opened by joining that exact listed name; the token derived from the untrusted directory name is only compared against listed names, never used to build a path. `upstream_dir_token` uses splits and char predicates and never byte-indexes untrusted text. |
| T-ikn-05 | Tampering | install-conformance-oracle.sh declaration grep | medium | mitigate | Task 3's gate replays the script's own extraction (regex, `head -1`, sed), requires `1.22.0`, and requires exactly one matching line. Verify block 4 then runs the real script end to end. |
| T-ikn-06 | Repudiation | docs/GSD-CORE-SYNC.md delta claims | low | mitigate | Every number is re-measured in Step 0 against the REF tag, never the sibling's working tree, and deviations go to the SUMMARY. The audit names file:line for every gsd-tools invocation, and every candidate from the changelog gets a recorded verdict. |
| T-ikn-SC | Tampering | npm install of @opengsd/gsd-core@1.22.0 (oracle) | high | mitigate | Package legitimacy is [VERIFIED] (see context table): exact version pin, npm `gitHead` equals the upstream `v1.22.0` tag sha, and it is the project's existing oracle package. Task 3 installs it only into a throwaway HOME that is deleted afterwards, never into ~/.claude. No Cargo dependency is added. `pre-tag-check --container` re-certifies it at release (I-ikn-10). |
</threat_model>

<verification>
Full regression, after the last commit. Use `rtk bypass` so rtk cannot filter the results.

- `rtk bypass cargo test --locked --no-fail-fast` exits 0. tests/driver_router_conformance.rs runs and does not skip, against the installed ~/.claude/gsd-core (VERSION 1.22.0). Report the summed raw `test result:` counts.
- `rtk bypass cargo clippy --all-targets --locked -- -D warnings` exits 0.
- `rtk bypass cargo fmt --check` is clean.
- `git log --oneline -5` shows the five commits (feat, fix, fix, fix, chore(sync)); `git status --short` shows nothing from this task. Nothing is pushed and no tag is created.
</verification>

<success_criteria>
- `workflow.cross_runtime_execution` is a typed, editable Bool Defaults row with since v1.22.0 and built-in default false; Modelled and DEFAULTS_OPTION_COUNT are 140; `execute.models.<runtime>` is a recorded, unmasked pass-through family.
- The namespaced `commit_docs` / `search_gitignored` help says the flat key wins.
- gsdmm's ROADMAP phase list, goals and milestone scope ignore fenced and over-indented phase headings, as gsd-core v1.22.0 does; a project-coded or unpadded phase directory reads its own UAT and VERIFICATION report.
- The baseline trio reads 1.22.0 / v1.22.0 / 1.22.0. README and the sync record agree, the record carries the measured v1.22.0 delta with the gsd-tools audit and a verdict for every changelog candidate, and the recipe reads REF=v1.23.0 / PREV=v1.22.0.
- The router conformance test is green against installed 1.22.0 and against the npm-pinned 1.22.0 oracle. The full suite, clippy and fmt are clean.
</success_criteria>

<output>
Create `.planning/quick/261119-ikn-sync-against-gsd-core-v1-22-0/261119-ikn-SUMMARY.md` when done. It records the re-measured values and any deviation from the planner's numbers, I-ikn-1..11, the oracle-certification result, and the release-time obligation to run `./scripts/pre-tag-check.sh --container vX.Y.Z` to certify the 1.22.0 oracle pin.
</output>
