---
phase: quick-261012-jqb
plan: 01
status: complete
subsystem: config (Defaults tab / gsd-core sync)
tags: [gsd-core, config, sync, v1.16.0]
requires: []
provides:
  - "AuditConfig block + GsdConfig.audit"
  - "Defaults rows gates.confirm_milestone_scope and audit.enabled (since v1.16.0)"
  - "Six legacy gates.* keys retired to pass-through, lossless"
affects: [src/state_reader/config_json.rs, src/ui/screens/detail.rs, docs/GSD-CORE-SYNC.md]
tech-stack:
  added: []
  patterns: ["typed block + #[serde(flatten)] extra", "JSON-pointer mutation assertions"]
key-files:
  created: []
  modified:
    - src/state_reader/config_json.rs
    - src/ui/screens/detail.rs
    - docs/GSD-CORE-SYNC.md
decisions:
  - "I-1..I-10 applied as written in the plan (see Inferred decisions)"
metrics:
  duration: "~25 min"
  completed: 2026-10-12
actuals:
  tasks: 3
  commits: 3
plan_head_before: 7303b41d0a9bcaa12d30370221c806b037ccf7b6
plan_head_after: 467a82dc67c1e8b8f39cf237a1a3ce0784e14216
worktree: /home/blk/projects/rust/gsd-meta-manager/.claude/worktrees/agent-ae7548555a24ed5cb
branch: worktree-agent-ae7548555a24ed5cb
---

# Phase quick-261012-jqb Plan 01: gsd-core v1.16.0 config-key delta Summary

gsd-core v1.16.0's config delta is applied to the Defaults tab. `gates.confirm_milestone_scope` and `audit.enabled` are now editable Bool rows with `since v1.16.0`, and `audit.enabled` sits in a new Observability category. The six `gates.*` keys upstream stopped reading are no longer typed rows. A file that still carries them keeps them: they round-trip without loss and show as read-only `Not modelled` rows. Gates and drift help is refreshed, and docs/GSD-CORE-SYNC.md's key inventory matches the 135 rows.

## Commits (branch worktree-agent-ae7548555a24ed5cb, base 7303b41)

| Task | Commit | Subject |
|---|---|---|
| 1 (tracer) | b82f375 | feat(config): model gates.confirm_milestone_scope and audit.enabled (gsd-core v1.16.0) |
| 2 | 793a904 | feat(config): retire the six gates.* keys gsd-core v1.16.0 removed |
| 3 | 467a82d | docs(config): refresh gates/drift help for gsd-core v1.16.0 and settle the v1.15.0 since |

Commit count measured with `git rev-list --count 7303b41..HEAD` = 3.

## What changed

- **config_json.rs:**
  - `GatesConfig` now has only `confirm_transition`, `confirm_milestone_scope`, `execute_next_plan` and `extra`, plus a doc comment on the retirement.
  - New `AuditConfig { enabled, extra }` with the I-2 divergence note. `GsdConfig.audit` is the last typed field, before `extra`.
  - Tests:
    - New: `the_gsd_core_1_16_0_keys_are_modelled` and `the_gates_keys_gsd_core_1_16_0_removed_round_trip_through_extra` (JSON-pointer comparisons; `safety.*` kept in the top-level extra).
    - Repaired: `the_resynced_non_workflow_keys_parse_and_round_trip`.
    - Extended: `the_resynced_blocks_are_absent_from_a_default_config` (adds `audit`).
  - Lines ~640-700 (`user_defaults_path` and the top of the tests module) are untouched. The hunks are at 224-346 and 1084+.
- **detail.rs:**
  - Rows:
    - The Gates category is now `execute_next_plan` (first), `confirm_transition`, `confirm_milestone_scope`.
    - New Observability category holding `audit.enabled`.
    - The six legacy rows and all their set/clear/toggle arms are removed.
    - New set/clear/toggle arms for the two new keys.
    - The pass-through walk now covers `audit.`.
  - `builtin_default_for_key` returns `true` for the three live gates.
  - Help text: gates summaries open with "Interactive mode only:". The drift_threshold summary now counts modified and deleted files, and drift_action is reworded.
  - The two v1.15.0 `since` comments now state the measured tag.
  - Test tables: added `RESYNCED_KEYS_1_16_0` and `REMOVED_KEYS_1_16_0`. `RESYNCED_KEYS` drops the six keys, and `RESYNCED_KEYS + REMOVED == 57`. `DEFAULTS_OPTION_COUNT` is 135. `NESTED_CONFIG_BLOCKS` gains `audit`.
  - Renamed test: `every_gsd_core_delta_key_is_an_editable_row_writing_its_own_json_path` (it now loops over the 1.15.0 and 1.16.0 tables).
  - New tests:
    - `the_gsd_core_1_16_0_rows_are_placed_and_carry_the_manifest_defaults`
    - `the_gates_keys_gsd_core_1_16_0_removed_stay_visible_read_only_and_survive_a_toggle`
    - `the_gates_category_holds_exactly_the_three_live_gates`
    - `the_gsd_core_1_16_0_help_refresh_is_in_place`
  - Extended: the sync-record test now also fails if a removed key appears under `## Modelled`.
- **docs/GSD-CORE-SYNC.md:**
  - `## Modelled`:
    - Intro now reads 135 rows: 51 / 2 / 7 / 2 / 73.
    - Gates table trimmed to the three live gates.
    - New Observability table with the I-1 note.
  - `## Pass-through`: `safety.*` is gone from the refactor row, and a new row lists the six removed gates keys and the two removed safety keys.
  - The sentence in the release-1.15.0 delta section that called its `since` values not-yet-tagged now states the measured tag.
  - Header, "Why VERSION and COMMIT disagree", "How to re-measure", "Next sync" and the GSD_CORE_SYNCED_* constants are untouched (I-8).
  - Counts checked against the tables: 135 rows, 73 with no `since`, 2 with v1.16.0, 2 with v1.15.0, and 0 "provisional" in the Modelled section.

## Verification (measured)

- `rtk proxy cargo test --lib --no-fail-fast -- state_reader::config_json ui::screens::detail`: 368 passed / 0 failed after Task 2, and green again after Task 3.
- `rtk proxy cargo test --no-fail-fast` (full suite, not piped): 58 result lines, **2954 passed, 1 failed, 15 ignored**. The only failure is the expected local one, `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`. The lib unittests reported 2081 passed / 1 failed / 1 ignored.
- `cargo fmt --check`: exit 0. `cargo build`: exit 0.
- Plan greps: removed-field grep = 0, removed-arm grep = 0, `provisional` in detail.rs = 0, `provisional` in the Modelled section = 0.
- `git diff --quiet -- Cargo.toml Cargo.lock`: no dependency change (T-jqb-SC).
- `rtk proxy cargo clippy --all-targets -- -D warnings`: **fails on one pre-existing lint outside this plan**, so the plan's "clippy exits 0" criterion is not met. See Deferred Issues.

## Deviations from Plan

- **TDD order, Tasks 1-2:** I wrote the implementation and tests in the same pass rather than watching each new test fail first. For Task 1 the sync-record test was observed red until the doc landed. For Task 2 the new reverse sync-record assertion was observed red until the Gates table was trimmed. Task 3 followed RED -> GREEN strictly: `the_gsd_core_1_16_0_help_refresh_is_in_place` failed on the old `gates.execute_next_plan` summary.
- **Precondition check:** this worktree-isolated agent cannot run `git -C ~/projects/node/gsd-core`. I verified the precondition by reading the upstream checkout's `.git/refs/tags/v1.16.0`, which is `87e87d34b2d8…`, so it is met. I did not re-derive any other upstream fact; the plan's measured `<upstream_facts>` were used as given.
- **Summary wording:** the drift_threshold summary says "in mapped directories", not "in directories the codebase map already describes", to stay within the 160-character budget.
- **Commit type, Task 3:** the Task 3 commit is typed `docs(config)` but also changes help-string literals and adds one test.
- **SUMMARY location:** worktree isolation refused a write to the main checkout path. This file was written to the same relative path inside the worktree and left uncommitted, for the coordinator to copy across.

## Deferred Issues

- **Pre-existing clippy failure, not caused by this plan:** `clippy::field_reassign_with_default` at `src/ui/screens/detail.rs:29973-29974`, in a test added by commit 4c5c71b4 (2026-10-06): `let mut cache = ProjectViewCache::default(); cache.defaults_config = Some(project);`. It is the only error `cargo clippy --all-targets -- -D warnings` reports (local clippy is rust-1.98.0). It is outside this plan's regions, so it was left alone under the scope rule. It is a one-line struct-init fix for whoever owns that test. Until then the plan's clippy-exits-0 criterion cannot hold on this machine.

## Inferred decisions — for later audit

All of these are copied from the plan and applied as written. Each is inferred — for later audit.

- **I-1 (inferred — for later audit):** `audit.enabled` gets `since` "v1.16.0". That is the release that made the key effective: af464656e, whose `tag --contains` gives v1.16.0, and which gave the key its Settings row. Step 5's first-mention recipe would give v1.2.0 (2d14eb887), but that commit was prose only, for a key config-set rejected and nothing read. The rationale is recorded beside the row in code and under the Observability table in the doc.
- **I-2 (inferred — for later audit):** `AuditConfig.enabled` is `Option<bool>`. A hand-edited non-boolean (for example the string "true") therefore makes the whole file fail to parse, where gsd-core would treat it as off. This is the same accepted divergence as PlannerConfig I-5, and a test pins it (`parse_gsd_config(r#"{"audit":{"enabled":"true"}}"#)` is `None`).
- **I-3 (inferred — for later audit):** New category "Observability" (upstream's own section name), placed after Gates. `GsdConfig.audit` is the last typed field, directly before `extra`.
- **I-4 (inferred — for later audit):** `builtin_default_for_key` maps the three live gates to "true", following the v1.16.0 defaults manifest. `audit.enabled` gets no built-in default because the manifest has no audit entry; its help text says it is off by default instead.
- **I-5 (inferred — for later audit):** Both new rows follow the existing toggle convention, `Some(!x.unwrap_or(false))`: from unset, the first toggle writes `true`. For `audit.enabled` that is the opt-in direction.
- **I-6 (inferred — for later audit):** `RESYNCED_KEY_COUNT` stays 57, the historical 1.14.0 measurement, and is asserted as `RESYNCED_KEYS.len() + REMOVED_KEYS_1_16_0.len()`. The 27 workflow.* count is unchanged.
- **I-7 (inferred — for later audit):** `workflow.drift_action`'s summary now says "drifted further" instead of "new structure", so it stays consistent with drift_threshold. Upstream's drift_action row did not change.
- **I-8 (inferred — for later audit):** In docs/GSD-CORE-SYNC.md, only `## Modelled`, the `## Pass-through` table and the one release-1.15.0 delta sentence were edited. The header and baseline sections and the GSD_CORE_SYNCED_* constants and README note are left for the later rewrite, even where they still describe 1.15.0 as untagged.
- **I-9 (inferred — for later audit):** `safety.*` was also removed upstream by ded8b6259. It was never modelled here, so only the Pass-through table changes.
- **I-10 (inferred — for later audit):** The six removed keys are not backtick-named anywhere inside `## Modelled`. They are listed only in the Pass-through table, so the new reverse assertion holds.

## Known Stubs

None.

## Threat Flags

None. No new surface beyond the plan's threat model. The new pass-through rows go through the existing `append_passthrough_entries` route, and all new `ConfigHelp` text is `&'static str`.

## Self-Check: PASSED

- FOUND: src/state_reader/config_json.rs, src/ui/screens/detail.rs, docs/GSD-CORE-SYNC.md (modified in the worktree)
- FOUND: b82f375, 793a904, 467a82d (each an ancestor of HEAD 467a82d)

## Coordinator note (inferred — for later audit)

- quick-batch 261012-jqa assigned jqc to wave 1 because both plans list `src/state_reader/config_json.rs` and `src/ui/screens/detail.rs`. The coordinator ran both executors in parallel anyway (explicit operator instruction), with plans confined to disjoint regions. Merge order was jqb then jqc via `worktree.cleanup-wave`; both merged cleanly. Combined tree: 2958 passed, 1 failed (expected envelope::policy git-version test), 15 ignored; fmt and `clippy -D warnings` clean.
