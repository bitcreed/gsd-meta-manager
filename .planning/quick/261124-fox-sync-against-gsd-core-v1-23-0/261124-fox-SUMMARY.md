---
phase: quick-261124-fox
plan: 01
subsystem: gsd-core-sync
tags: [sync, config, readers, router, conformance]
requires: []
provides:
  - "workflow.deep_verify and workflow.deep_verify_all_phases typed editable Bool Defaults rows (since v1.23.0)"
  - "cross-runtime and live_dom_uat help corrected for v1.23.0 (dispatch-isolation reads the keys)"
  - "a Config-tab save never invents a null or empty top-level key and keeps a real barrier null (gsd-core #5096)"
  - "Project view shows (global: <value>) for keys gsd-core v1.23.0 merges per key (gsd-core #4071 / #6030)"
  - "ROADMAP milestone headings need a leading version (#6149); recorded-pause router gate gate_paused (#6431)"
  - "baseline trio 1.23.0 / v1.23.0 / 1.23.0, README note, sync record with v1.23.0 delta"
affects: [src/state_reader/config_json.rs, src/state_reader/config_write.rs, src/ui/screens/detail.rs, src/ui/screens/preferences.rs, src/state_reader/roadmap_md.rs, src/state_reader/mod.rs, src/state_reader/state_md.rs, src/state_reader/disk_status.rs, src/driver/router.rs, docs/GSD-CORE-SYNC.md, README.md]
key-files:
  modified:
    - src/state_reader/config_json.rs
    - src/state_reader/config_write.rs
    - src/ui/screens/detail.rs
    - src/ui/screens/preferences.rs
    - src/state_reader/roadmap_md.rs
    - src/state_reader/mod.rs
    - src/state_reader/state_md.rs
    - src/state_reader/disk_status.rs
    - src/driver/router.rs
    - docs/GSD-CORE-SYNC.md
    - README.md
key-decisions:
  - "I-fox-1..14 as recorded in the plan; all applied as written (deviations below)"
status: complete
completed: 2026-11-24
plan_head_before: 7b7de91fdcbf2dfbd9782377e4c059c091d734f2
plan_head_after: bfcd3fc7ceeb501ac88158353a47c817a4078f0f
actuals:
  tokens: 25800
  tasks: 3
  commits: 8
---

# Quick 261124-fox: Sync against gsd-core v1.23.0

`actuals.tokens` is chars/4 over the realized diff (103159 chars, 11 files, +1177 / -116), the scale the estimate uses.

## Commits

- 2ba4a15f feat: model workflow.deep_verify and workflow.deep_verify_all_phases as typed rows (A)
- 31059eea fix: say dispatch-isolation now reads the cross-runtime keys (B)
- 9897d8d2 fix: a config save never invents a null section gsd-core reads as a barrier, #5096 (C)
- b578fae6 feat: show the global value gsd-core merges per key under a project, #4071 (D)
- dbbfb3c0 fix: a ROADMAP heading opens a milestone only when its version leads, #6149 (E)
- 6e7346be feat: park the driver on a recorded pause, #6431 (F)
- 0f527e37 docs: correct the gaps_found-over-stale claims, #6078 (G)
- bfcd3fc7 chore(sync): move the gsd-core baseline to v1.23.0

## Re-measured values (all matched the plan except the sibling HEAD)

v1.23.0 = e3dcca27e73c3b50d9063192f67fc159769c9f9a, package.json 1.23.0, tagged 2026-11-23T02:29:32Z; npm `latest` 1.23.0, gitHead equal, published 2026-11-23T02:38:10Z. v1.22.0 is an ancestor: 114 commits, 645 files (+49496 / -9071). Key cells 266 -> 268, +2 / -0 (`workflow.deep_verify`, `workflow.deep_verify_all_phases`), both first committed in e43a9e4c6 (#3858 / PR #6378), `tag --contains` -> `v1.23.0`. The `workflow.cross_runtime_execution` and `execute.models.<runtime>` cells changed text only. Schema and defaults manifests, config template, `src/secrets.cts` and `gsd-run-resolver.md`: no diff. Capabilities 45 -> 46 dirs; 46 modified files (45 version-only plus mempalace's `execute:wave:post` -> `execute:post`), 2 added. Secret heuristic 6 -> 6 (the same six, all already exempt), 6 "Masked in display" cells at both tags. CLI manifest: 94 verbs, `global` and `undeclared` (138, holding `smart-entry` and `init.manager`) unchanged; one changed verb, `dispatch-isolation` (not called). `commands/gsd` 72 files, 4 modified, two additive `argument-hint` changes; agents 64 -> 65, 3 modified. Step 3 `comm -23`: 173 against v1.22.0 and 173 against v1.23.0 (plan expected 173 for the latter). All five gsd-tools invocations re-run against installed 1.23.0: `smart-entry --json`, `query init.manager`, `runtime-identity --raw` exit 0; `--version` still rejected (exit 1); the hook-string fixture is never executed.

## Deviations

- **Sibling checkout HEAD.** The planner saw a fork docs branch at `v1.22.0-117-g049d1ca3e`; it is now the `next` branch at `v1.23.0-25-g455cd03c1` (package.json 1.23.0). The recipe parenthetical in docs/GSD-CORE-SYNC.md names the re-measured one, as the plan requires. Every measurement used the tags, so nothing depended on it.
- **[Rule 1 - design] `parse_gsd_config` parses twice, not once.** The plan built the `GsdConfig` from a `serde_json::Value`. `serde_json::from_value` loses the line and column that `parse_error_summary` reports, so the typed `from_str` is kept and a second `Value` parse runs only on text that already parsed, to record `null_barrier_sections`. Behavior and value-free error logging are unchanged.
- **Commit A was not rustfmt-clean.** The new commit-A test in detail.rs had two lines `cargo fmt` rewraps; the rewrap landed in commit D (which already edits that file). The tree is `cargo fmt --check` clean at HEAD.
- **RED-first was not shown commit by commit.** Tests were written alongside each implementation and run green; I did not capture a red run before each fix.
- **`driver::lock::tests::probe_at_reports_absent_free_and_held`** failed once, under load, in the first lib run; it passed in every later lib run and in the full suite. Not touched by this task.
- **Known limit of the global display (I-fox-4).** A global file that spells a short `workflow.*` key flat (`plan_checker`) does not feed `(global: ...)`, because the global entries are built from the typed nested fields. Recorded in the sync record under "Not ported".

## Pinned tests whose expectation moved

- `daily_vow_shipped_milestones_are_the_five_before_v1_5` (no `Requirement Coverage` milestone; shipped vector loses the trailing `false`) and `sentriq_v0_11_is_shipped_by_version_order` (labels `["v0.11 Phases"]`, shipped `[true]`): #6149. No other roadmap pin moved.
- Config scope copy: `config_scope_strip` pins (global hint `new projects + unset keys`) and the four pins of the Global status text (now `Editing global defaults ($GSD_HOME/.gsd/defaults.json): new projects copy them; projects inherit model and gate keys they leave unset`). The Global title and its pin are unchanged.
- `project_view_shows_builtin_default_not_global_value` replaced by `project_view_shows_the_global_value_gsd_core_merges_per_key`; `filter_matches_values_as_well_as_keys` gained an inherited-value case; `DEFAULTS_OPTION_COUNT` 140 -> 142 with `RESYNCED_KEYS_1_23_0` (2) chained into both loops; the workflow parse fixtures and the `populated_gsd_config` fixture carry the two keys.

## Inferred decisions

All of I-fox-1..14 from the plan were applied as written: [I-fox-1] typed editable rows between `live_dom_uat` and `ui_interaction_capture`; [I-fox-2] cross-runtime wording corrected, historical v1.22.0 section left; [I-fox-3] save path never invents a blank key, keeps a recorded barrier null, key order sorted only on that path; [I-fox-4] display-only per-key global layer over 20 rows; [I-fox-5] scope copy corrected; [I-fox-6] pass-through-family global layer and the gitignore rung deferred (Next sync item 10); [I-fox-7] leading-version rule with bare-major `v2` kept; [I-fox-8] Gate 0 as a park with the fixed detail `recorded_pause`; [I-fox-9] comments only for #6078; [I-fox-10] no surface for the listed items, recorded in the sync record; [I-fox-11] no gsd-tools or `NON_SECRET_CONFIG_KEYS` change; [I-fox-12] oracle certified in a throwaway HOME, `pre-tag-check --container` left to release; [I-fox-13] recipe at `REF=v1.24.0` / `PREV=v1.23.0`; [I-fox-14] `~/.gsd/defaults.json` not edited.

## Gates

- `cargo test --locked --no-fail-fast`: 3897 passed, 0 failed, 19 ignored over 61 suites (summed raw `test result:` lines; previous sync 3884). `tests/driver_router_conformance.rs` ran against installed 1.23.0: 8 passed, no skip.
- `cargo clippy --all-targets --locked -- -D warnings`: clean (rc 0).
- `cargo fmt --check`: clean.
- Oracle: `scripts/install-conformance-oracle.sh` in a throwaway HOME printed `ORACLE READY`, `action : installed @opengsd/gsd-core@1.23.0`, probe PASS; conformance against it: 8 passed. The throwaway HOME was removed.
- The plan's three verify blocks (T1-OK, T2-OK, CONSTANTS-README-OK, RECORD-OK) printed.

## Notice for the user (I-fox-14)

`~/.gsd/defaults.json` carries a `"git": null` line (written by an earlier gsdmm Global-view save), and `"mode": ""` / `"granularity": ""` plus null search slots. gsd-core v1.23.0 reads a null `git` section as a barrier that removes branching strategy, base branch and branch templates for every directory without a `.planning/`; this build now preserves that line on a save because it cannot tell it from a deliberate barrier. Remove the line by hand if it was not intended. The blank scalars and null search slots are dropped by the next Global-view save. This task did not edit the file.

## Release-time obligation

Run `./scripts/pre-tag-check.sh --container vX.Y.Z` to certify the 1.23.0 oracle pin in the publish job's own environment; not run here (it needs a release tag). Nothing pushed, no tag, `Cargo.toml` version unchanged.
