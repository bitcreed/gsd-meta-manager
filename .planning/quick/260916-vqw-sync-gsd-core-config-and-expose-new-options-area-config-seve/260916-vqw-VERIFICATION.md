---
item: 260916-vqw
verified: 2026-09-17T01:30:00Z
status: passed
score: 6/6 must-haves verified
covered_files:
  - .planning/quick/260916-vqw-sync-gsd-core-config-and-expose-new-options-area-config-seve/260916-vqw-PLAN.md
  - .planning/quick/260916-vqw-sync-gsd-core-config-and-expose-new-options-area-config-seve/260916-vqw-SUMMARY.md
  - .planning/todos/pending/2026-09-15-sync-gsd-core-config-and-expose-new-options.md
  - docs/GSD-CORE-SYNC.md
  - src/state_reader/config_json.rs
  - src/ui/screens/detail.rs
  - src/ui/screens/render_escape_guard.rs
covered_digest: "v1:sha256:01ca1a3b504265046311bd8576e807d67015d1c3eb60bc69864bb49ffbe7647f"
re_verification:
  previous_status: gaps_found
  previous_score: 5/6
  gaps_closed:
    - "Every key present in a project's .planning/config.json is visible somewhere in the Defaults tab — modelled keys as editable rows, unmodelled keys as read-only rows."
  gaps_remaining: []
  regressions: []
---

# Quick Task 260916-vqw: gsd-core config re-sync — RE-VERIFICATION Report

**Item goal:** Sync gsd-meta-manager's config schema against gsd-core (1.14.0),
record the sync baseline durably, expose new options (incl.
`workflow.compact_content`) with per-key `since` annotations, and stop the
Defaults tab's save path from silently deleting config keys it doesn't model.

**Verified:** 2026-09-17
**Status:** passed
**Re-verification:** Yes — after gap closure (commit `26cbef7`)

Verified against the CURRENT working tree at HEAD `858a307` (main checkout, not
the concurrent worktree under `.claude/worktrees/`). No file under `src/` or
`tests/` was left modified by this verification — probes were reverted
(`git status --porcelain` empty throughout, confirmed below).

## History

The prior verification (`260916-vqw-VERIFICATION.md`, superseded by this file)
scored 5/6 and found exactly one blocker: `append_passthrough_entries()`'s
`collect()` helper walked 11 nested config blocks' `extra` maps but omitted the
four blocks this item's own Task 3 introduced — `gates`, `features`,
`planning`, `plan_review` — so an unmodelled key under those four was
preserved on save (T-VQW-02 held) but never rendered as a row (T-VQW-01's
"visible" half did not).

Commit `26cbef7` claims to close this with four more `take(...)` calls plus a
15-block census test. This re-verification independently confirms that claim
rather than accepting it.

## 1. Is the ORIGINAL gap genuinely closed?

**YES — confirmed by reading the fix and by an independent negative probe.**

- `src/ui/screens/detail.rs:6771-6782` — `collect()` inside
  `append_passthrough_entries` now has four more arms:
  ```rust
  if let Some(block) = config.features.as_ref() { take("features.", &block.extra); }
  if let Some(block) = config.gates.as_ref() { take("gates.", &block.extra); }
  if let Some(block) = config.planning.as_ref() { take("planning.", &block.extra); }
  if let Some(block) = config.plan_review.as_ref() { take("plan_review.", &block.extra); }
  ```
  mirroring the pre-existing 11 (`git`, `workflow`, `hooks`, `intel`,
  `graphify`, `claude_orchestration`, `statusline`, `dynamic_routing`,
  `review`, `external_job`, `capabilities`) at lines 6727-6759.
- Cross-checked against `GsdConfig`'s actual field list
  (`src/state_reader/config_json.rs:46-119`): exactly 15 nested
  `Option<...Config>` fields exist, and all 15 now have a `take` call. No 16th
  block was missed.
- Ran the regression test added by the fix:
  ```
  cargo test --lib an_unmodelled_key_in_every_nested_block_becomes_a_visible_pass_through_row
  → ok
  ```
- Ran a fresh regression check on the earlier (pre-gap-fix) coverage too:
  ```
  cargo test --lib an_unmodelled_key_becomes_a_read_only_row_under_its_own_category
  → ok
  ```

**Verdict:** an unmodelled key under each of `gates`, `features`, `planning`,
`plan_review` now reaches a rendered row. Confirmed by code + test, not by
narration.

## 2. Does the census test really cover all nested blocks, and would it FAIL if a block were dropped?

**YES to both — demonstrated, not just reasoned about.**

Coverage: `NESTED_CONFIG_BLOCKS` (`detail.rs:8156-8171`) lists all 15 blocks
alphabetically (`capabilities`, `claude_orchestration`, `dynamic_routing`,
`external_job`, `features`, `gates`, `git`, `graphify`, `hooks`, `intel`,
`plan_review`, `planning`, `review`, `statusline`, `workflow`) — verified this
is the complete field set by grepping `GsdConfig`'s struct definition (15
`Option<...Config>` fields, matches exactly).

**Demonstration that it fails on regression** (not merely reasoned about): I
temporarily deleted the `gates` `take(...)` arm added by the fix, ran the
census test, and observed it fail:

```
thread '...an_unmodelled_key_in_every_nested_block_becomes_a_visible_pass_through_row' panicked:
assertion `left == right` failed: expected one pass-through row per nested block plus the top level; got
[..."features.zz_block_probe", "git.zz_block_probe", ... "plan_review.zz_block_probe",
"planning.zz_block_probe", ...]  (gates.zz_block_probe absent)
  left: 15
 right: 16
```

Then reverted with `git checkout -- src/ui/screens/detail.rs` and confirmed
`git status --porcelain src/ tests/` returned empty (clean tree, nothing left
behind).

The census also goes beyond entry-vector presence: it asserts (a) exactly
`NESTED_CONFIG_BLOCKS.len() + 1` rows exist (non-vacuity floor), (b) every row
is `ConfigValueKind::ReadOnly`, (c) `set_config_value` / `clear_config_value` /
`mutate_config_entry` all refuse the four previously-omitted keys, and (d) the
dotted key text actually appears in `render_defaults_config_to_text`'s
rendered terminal buffer — a real `ratatui::Terminal<TestBackend>` render, not
just the `ConfigEntry` list. This satisfies the must-have's literal word
"visible" (a rendered cell), not just "modelled" (a data structure).

## 3. Do the item's other must-haves still hold after the fix?

All re-checked against the current tree, not re-taken on faith:

| Must-have | Status | Evidence |
|---|---|---|
| Unmodelled key survives a Defaults-tab save (T-VQW-02) | ✓ VERIFIED | `cargo test --lib unknown_keys_survive_serialize_and_reparse` → ok. Untouched by the gap-fix commit (only `detail.rs` changed — `git show 26cbef7 --stat`). |
| Every config.json key visible somewhere in Defaults tab | ✓ VERIFIED | See §1/§2 above — the gap this re-verification exists to check. |
| `workflow.compact_content` editable, help names introducing version | ✓ VERIFIED | `cargo test --lib compact_content_is_a_modelled_workflow_key` → ok; `cargo test --lib compact_content_is_an_editable_row_naming_its_gsd_core_version` → ok. |
| All 57 newly-modelled scalar keys are editable/read-only rows with help | ✓ VERIFIED | `cargo test --lib every_resynced_key_has_one_row_of_the_right_kind_with_a_measured_since` → ok; `DEFAULTS_OPTION_COUNT == 130` unchanged (`detail.rs:7767`). |
| gsd-core version/commit recorded in tracked source | ✓ VERIFIED | `GSD_CORE_SYNCED_VERSION`/`GSD_CORE_SYNCED_COMMIT` (`config_json.rs:21,28`) unchanged by the fix; `docs/GSD-CORE-SYNC.md` present (13136 bytes) and unchanged by the fix (fix touched only `detail.rs`). |
| Pass-through row cannot emit a terminal escape (T-VQW-01) | ✓ VERIFIED | `cargo test --lib a_hostile_pass_through_key_cannot_reach_a_cell_unescaped` → ok. `render_escape_guard.rs` was NOT touched by commit `26cbef7` (`git show 26cbef7 --stat` shows only `detail.rs`). The render site (`detail.rs:4578-4608`) still routes both `entry.key.as_ref()` and `entry.value` through `shown()` before building the `Span` — read directly, discipline unweakened. |

Full-suite regression check (once, per the "run full suite at most once" rule):

```
cargo test --lib --no-fail-fast
→ 1279 passed, 1 failed, 1 ignored
```

The one failure is
`envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`
— a pinned git-version constant (2.43.0) vs. the installed git (2.53.0),
environmental and pre-existing per project memory
(`git-version-constants-test-env-failure.md`), unrelated to this item and
unrelated to the gap fix.

```
cargo clippy -- -D warnings → exit 0
cargo build → exit 0
```

## Anti-Pattern Scan

`grep -nE "TBD|FIXME|XXX|TODO|HACK|PLACEHOLDER" src/ui/screens/detail.rs` — one
hit, `\uXXXX` inside a doc comment describing JSON surrogate-pair escaping, not
a debt marker. No blockers.

## Self-Check on This Verification

- Probe 1 (deleting the `gates` `take` arm to prove the census fails): reverted
  via `git checkout -- src/ui/screens/detail.rs`.
- `git status --porcelain src/ tests/` returned empty after all probes.
- `git status --porcelain` (full) shows only a pre-existing unrelated modified
  file (`260915-f4n-SUMMARY.md`, from a different quick item), not touched by
  this verification.
- No files were committed by this verification.

## Requirements Coverage (todo's 4 sub-requirements) — unchanged from prior pass, re-confirmed

| # | Requirement | Status | Evidence |
|---|---|---|---|
| 1 | Diff schema against gsd-core sibling repo to find drift | SATISFIED | `docs/GSD-CORE-SYNC.md` documents the diff commands and 130-key inventory. |
| 2 | Record synced gsd-core tag/version durably | SATISFIED | `GSD_CORE_SYNCED_VERSION`/`GSD_CORE_SYNCED_COMMIT` + `docs/GSD-CORE-SYNC.md`. |
| 3 | Newer config options actually reachable/exposed, not silently missing | SATISFIED | Gap closed — see §1/§2. No remaining counter-example found. |
| 4 | In-app help names the gsd-core version that introduced each option | SATISFIED (scoped) | 57/130 rows carry `since`, matching the plan's own must-have scope (57 newly-synced keys); the 73 pre-existing rows are explicitly deferred in `docs/GSD-CORE-SYNC.md`'s "Next sync" section, not hidden. |

## Gaps Summary

None remaining. The single blocker from the prior verification pass is closed
with code-level evidence (a working `take` call per block, cross-checked
against `GsdConfig`'s actual field list) and test-level evidence (a census
test independently demonstrated to fail when a block's `take` call is
removed, and to pass on the current tree).

---

_Verified: 2026-09-17T01:30:00Z_
_Verifier: Claude (gsd-verifier)_
