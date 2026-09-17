---
quick_id: 260916-vqw
status: complete
subsystem: config
tags: [config, gsd-core-sync, tui-defaults, data-loss-repair, untrusted-text]
requires:
  - src/state_reader/config_json.rs (GsdConfig and its nested blocks)
  - src/ui/screens/detail.rs (build_defaults_entries and the three mutation tables)
  - ~/projects/node/gsd-core @ v1.14.0-52-g651511d1e (READ-ONLY source of truth)
provides:
  - GsdConfig::extra — unknown-key preservation across a save/reload cycle
  - 57 new first-class Defaults-tab rows with measured per-key `since`
  - Pass-through read-only rows for every unmodelled key found in a project's config
  - docs/GSD-CORE-SYNC.md — the baseline the next sync diffs from
  - GSD_CORE_SYNCED_VERSION / GSD_CORE_SYNCED_COMMIT
affects:
  - The Defaults tab's save path (persist_active_config) — now lossless
  - ConfigEntry::key — &'static str -> Cow<'static, str>, Owned half is untrusted
tech-stack:
  added: []
  patterns:
    - "serde flatten remainder capture for lossless round-trips over an unknown schema"
    - "measured `since` per config key, resolved from the upstream repo's git history"
key-files:
  created:
    - docs/GSD-CORE-SYNC.md
  modified:
    - src/state_reader/config_json.rs
    - src/ui/screens/detail.rs
    - src/ui/screens/render_escape_guard.rs
decisions:
  - "Unknown-key preservation is in scope: it is the only mechanism that makes gsd-core's nested and templated keys visible without a typed field each, and it repairs a live data-loss bug on the save path."
  - "The 57 scalar keys get typed rows; everything else is pass-through read-only AND listed in docs/GSD-CORE-SYNC.md with a reason, so deferred is distinguishable from unnoticed."
  - "`since` is resolved by measuring gsd-core's history with the DOTTED key as the pickaxe needle, never the leaf name and never from memory."
metrics:
  duration: ~2h
  completed: 2026-09-17
actuals:
  tokens: 48000
  tasks: 3
  commits: 3
  plan_head_before: 3fa669e880dfd26df5254a9e829a557f45d2dfd6
---

# Quick Task 260916-vqw: gsd-core config re-sync Summary

Re-synced this build's config schema against gsd-core 1.14.0, repaired the
Defaults tab's silently-destructive save path, exposed all 57 previously
unmodelled scalar keys with measured introduction versions, and recorded the
baseline so the next sync starts from a known point instead of a blank slate.

## WORKTREE_*_OBSERVED

```
WORKTREE_BASE_OBSERVED=3fa669e880dfd26df5254a9e829a557f45d2dfd6
WORKTREE_BRANCH_OBSERVED=worktree-agent-afd87da7811dc2feb
WORKTREE_PATH_OBSERVED=/home/blk/projects/rust/gsd-meta-manager/.claude/worktrees/agent-afd87da7811dc2feb
```

## What shipped

| Task | Commit | Status |
|---|---|---|
| 1 — Tracer: unknown-key survival end to end, plus `workflow.compact_content` | `181101c` | done |
| 2 — The 26 remaining `workflow.*` keys with measured `since` | `a4c9535` | done |
| 3 — The 30 non-`workflow` keys, count pinned at 130, sync record written | `29d34bd` | done |

### Task 1 — the data-loss repair, and the new untrusted-text boundary

`persist_active_config` writes `serialize_gsd_config(&GsdConfig)` straight over
the operator's `.planning/config.json`, and serde drops what it did not parse.
Before this task, toggling one checkbox in the Defaults tab **deleted every
gsd-core key this build did not model** — `review.models.*`, `model_policy.*`,
`effort.*`, `gates.*`, all of it. `GsdConfig` and all twelve nested blocks now
carry a `#[serde(flatten)] extra` map, so the writer is a fixed point over keys
it has never heard of.

Those captured keys are then SHOWN: `append_passthrough_entries` emits one
sorted read-only row per unmodelled key under a `Not modelled` category, with
the same project-then-defaults layering the `opt_*_layered` helpers use.

That made `ConfigEntry::key` project-supplied text for the first time, so it
became `Cow<'static, str>` and every render site routes it through `shown()`
(T-VQW-01). The `'x'`-clear status message escapes it at the `format!`, which is
the arm a pass-through key reaches by construction.

### Tasks 2 and 3 — 57 keys, 73 -> 130 rows

11 booleans, 4 strings, 4 enums, 6 integers and 1 read-only list for
`workflow.*`; four brand-new blocks (`features`, `gates`, `planning`,
`plan_review`), `context_window`, and extensions to `git`, `hooks`, `graphify`,
`statusline` and `dynamic_routing` for the rest. Each wired into all four
mutation paths and each carrying a `since` measured from gsd-core's history.

## Measured, not asserted

**The baseline.** `node -p` on gsd-core's `package.json` gave `1.14.0`;
`git describe --tags --always` gave `v1.14.0-52-g651511d1e`. Both are in
`config_json.rs` as constants and in `docs/GSD-CORE-SYNC.md` as prose, pinned
together by `the_sync_record_names_every_modelled_key_and_the_measured_baseline`.

**Per-key `since`.** All 57 resolved on the first attempt using the DOTTED key
as the `git log -S` needle; the leaf-name fallback was never reached. The
distribution: 36 at `v1.01.0`, 6 at `v1.13.0`, 5 at `v1.12.0`, 2 each at
`v1.14.0` and `v1.2.0`, 1 each at `v1.11.0` and `v1.9.0`. `v1.01.0` is a real
gsd-core tag dated 2026-05-24, not a typo — verified against that repo's
`for-each-ref --sort=creatordate` listing.

**The re-generated drift.** Re-running the plan's own extraction after the work
reports **none of the 57 still unmodelled**. The residual `comm` output (171
lines) breaks into three buckets, all documented in the sync record: ~58 enum
VALUES the grep mistakes for keys, 30 keys modelled under an unprefixed row
label, and ~80 pass-through families. The row count landed on exactly 130
without adjustment, matching the plan's 73 + 57.

## Fail-first evidence

`a_hostile_pass_through_key_cannot_reach_a_cell_unescaped` was observed RED by
reverting exactly one `shown()` on the key span, then restored:

```
thread '...a_hostile_pass_through_key_cannot_reach_a_cell_unescaped' panicked at
src/ui/screens/detail.rs:7416:9:
the pass-through row put ['\u{e0041}'] into the terminal buffer — those
characters render as nothing, so what the operator reads is not what the key is
```

Two more reds were produced by the work itself rather than by planting, and both
are recorded because each is a fixture that would have asserted nothing:

- The first version of that test built its hostile JSON with `format!` and
  `escape_default()`, which emits Rust's `\u{e0041}` rather than JSON's
  surrogate pair. It failed at `the hostile fixture parses`. The fixture now
  goes through `serde_json`.
- Adding `compact_content` to `populated_gsd_config` twice (once per task) made
  the whole fixture fail to parse — serde's flatten rejects the duplicate rather
  than taking the last. Nine assertions went red at once on
  `the populated Defaults fixture parses`, which is the fixture's non-vacuity
  floor working.

The unknown-key preservation tests have a different fail-first character worth
naming rather than glossing: `wf.extra` does not COMPILE without the field, so
the test could not be written-then-failed in the usual order. The compile error
is the red.

## Threat model dispositions

| Threat | Disposition | What landed |
|---|---|---|
| T-VQW-01 (project-supplied KEY on screen) | mitigated | `shown()` at the list span, the string-edit popup title, the dropdown title, and the clear status message. `build_config_help_pane` keeps its structural "no config.json string reaches here" property — the shared pass-through help is a static literal and the key is never interpolated into it. Observed red. |
| T-VQW-02 (destructive writer) | mitigated | `extra` flatten maps; the pin is a byte-equality of two successive serialisations, not a key spot-check. |
| T-VQW-03 (many unknown keys → long list) | accepted | Unchanged; bounded by the operator's own file. |
| T-VQW-04 (sibling repo) | mitigated | Every gsd-core access was a read. `git -C ~/projects/node/gsd-core status --porcelain` is empty after the work. |
| T-VQW-SC (dependency installs) | n/a | No dependency added or changed. |

The escape guard was strengthened, not weakened: `hostile_gsd_config` now
carries an unmodelled key whose NAME is the hostile identity, and `probe_ctx`
moves the Defaults cursor onto the first pass-through row via a derived index —
without that, a 130-row list at the probe's 60-row terminal would leave the
fixture populated, unrendered, and counted as coverage.

## Verification

| Gate | Result |
|---|---|
| `cargo test --lib --no-fail-fast` | 1251 passed, 1 failed (pre-existing, environmental — see below) |
| `cargo clippy -- -D warnings` | exit 0 |
| `cargo build` | exit 0 |
| Drift regenerated | 0 of the 57 still unmodelled |
| Sibling repo untouched | `git status --porcelain` empty |
| No file outside the worktree modified | confirmed |

The one failure is
`envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`,
which compares a pinned git-version constant against the installed git (2.43.0
here). It is environmental, pre-existing, unrelated to this task, and recorded
in project memory as such.

## Inferred decisions (for audit)

The human was unavailable throughout. Each of these was decided from the plan
and the codebase.

1. **`workflow.compact_content` was pulled into Task 1, not Task 2.** Task 1's
   `<behavior>` block requires it to NOT appear among the pass-through rows,
   which is only assertable once it is modelled. Making it the tracer's modelled
   half gives the slice both directions of the property. `DEFAULTS_OPTION_COUNT`
   therefore went 73 → 74 → 100 → 130 rather than 73 → 73 → 100 → 130.

2. **`since` was measured with the DOTTED key as the pickaxe needle**, with the
   plan's leaf-name command as the fallback. The plan's literal command uses the
   leaf name, and for keys like `enabled`, `auto_update` or `create_tag` that
   matches dozens of unrelated commits across `src/`, giving a first-touch commit
   several releases too early. All 57 resolved on the dotted form, so the
   fallback never fired and no result is leaf-derived.

3. **Integer rows got `mutate_config_entry` arms**, though the plan only
   specified arm-table wiring "for every bool and enum". Without one, pressing
   Enter on `workflow.smart_zone_tokens` is a dead key — the row is visibly
   selectable and silently inert. Each cycle's step and wrap point comes from
   the key's own documented range; the two `hooks.context_*_threshold` keys cycle
   inside their own half of 0..100 because gsd-core enforces
   `critical < warning` and a single 0..100 cycle would step straight past it.

4. **`git.protected_branches`, `planning.sub_repos` and
   `workflow.code_review_depth_overrides` are `ReadOnly`** per the plan; their
   clear arms were deliberately omitted so a read-only row cannot be half-
   editable, and the coverage test asserts that in the negative direction.

5. **The todo file was NOT moved to `.planning/todos/completed/`.** The plan's
   Task 3 step 5 defers to the workflow when the workflow owns that lifecycle
   step, and this executor's constraints forbid touching files outside
   `files_modified`. The orchestrator owns it.

6. **`render_escape_guard.rs`'s census `MEASURED_REACH` was left untouched.** The
   first version of the escape test called `crate::text::display_identity`
   directly, which added a needle occurrence under `src/ui/` and turned the
   census red. Rather than re-pin the census, the test now names the same
   composition the render applies (`render_for_terminal`) — which is more
   correct as an oracle anyway, since a half-composition would disagree with the
   render the day the composition changes.

7. **Category placement.** `gates.*` got its own category (the plan said so);
   `features.*`, `plan_review.*` and the planning-shaped `planning.*` keys sit in
   Planning; `workflow.auto_prune_state` and `planning.sub_repos` sit in Misc
   beside the STATE.md and `sub_repos` rows they relate to. The plan's guidance
   was "follow where the neighbouring gates already sit", which is a judgement
   call each time.

## Known Stubs

None. Every row added is wired to a real config field, and every pass-through
row is wired to a real captured key.

## Deferred

Recorded in `docs/GSD-CORE-SYNC.md`'s "Next sync" section rather than left
implicit:

- **~80 pass-through families** get no typed row by design (ID-3); the record
  names each family and why. `parallelization.*` is flagged as the highest-value
  promotion because this tab already edits a top-level `parallelization` boolean
  and a project carrying the namespaced block sees a row that disagrees with its
  file.
- **The unprefixed row labels.** Thirty rows read a namespaced JSON path but are
  labelled without the prefix (`base_branch` for `git.base_branch`). The path is
  correct; only the label is short, so the tab now mixes two conventions. Fixing
  it means moving `set_config_value`, `clear_config_value` and
  `mutate_config_entry` to the dotted spelling at once, which has no user-visible
  benefit beyond consistency and would have been buried inside a 57-key re-sync.
- **The 73 pre-existing rows carry no `since`** and were not retro-measured.

## Self-Check: PASSED

- `docs/GSD-CORE-SYNC.md` — FOUND
- `src/state_reader/config_json.rs` — FOUND (modified)
- `src/ui/screens/detail.rs` — FOUND (modified)
- `src/ui/screens/render_escape_guard.rs` — FOUND (modified)
- commit `181101c` — FOUND
- commit `a4c9535` — FOUND
- commit `29d34bd` — FOUND
- `git rev-list --count 3fa669e..HEAD` = 3, matching the `commits:` frontmatter
- working tree clean, no deletions in the range, sibling repo untouched

## Gap fix (post-verification)

`260916-vqw-VERIFICATION.md` scored this item 5/6 and found one blocker against
must-have truth #2 ("every key present in a project's `.planning/config.json` is
visible somewhere in the Defaults tab"). Closed by commit `26cbef7`, one file:
`src/ui/screens/detail.rs`.

**The gap.** `append_passthrough_entries`'s `collect()` walked eleven nested
blocks' `extra` maps plus the top level, but not the four blocks Task 3 of this
same item introduced — `features`, `gates`, `planning`, `plan_review`. Those
blocks each carry their own `#[serde(flatten)] extra`, so an unmodelled key under
them was preserved on save (T-VQW-02 genuinely held), but no row was ever
emitted, so the operator could not see the key existed. The "silently missing"
failure mode this sync fixed on the WRITE path had simply relocated to four spots
on the DISPLAY path — the sync never extended its own pass-through walk to cover
its own new blocks.

**What changed.**

1. Four more `take("features.", …)` / `take("gates.", …)` /
   `take("planning.", …)` / `take("plan_review.", …)` calls in `collect()`,
   mirroring the existing eleven, with a comment at the site naming why a new
   `GsdConfig` block must get one.
2. `an_unmodelled_key_in_every_nested_block_becomes_a_visible_pass_through_row` —
   a CENSUS over all fifteen nested blocks rather than a spot-check of the four,
   so the next block added fails here instead of becoming invisible identically.
   It asserts each probe key becomes a row, that the row is `ReadOnly`, that no
   set/clear/toggle arm claims it, and that it REACHES A TERMINAL CELL, not just
   the entry vector. A count assertion is the non-vacuity floor.
3. `render_defaults_config_to_text` — the existing render probe, generalised over
   a caller-supplied config. `populated_gsd_config` is deliberately free of
   unmodelled keys (Task 1 step 7, so `DEFAULTS_OPTION_COUNT` counts static rows
   only), so a pass-through row is unrenderable through it.

**RED → GREEN.** The test was written first and observed failing on the merged
code, naming exactly the four omitted blocks:

```
assertion `left == right` failed: expected one pass-through row per nested block
plus the top level; got ["capabilities.zz_block_probe",
"claude_orchestration.zz_block_probe", "dynamic_routing.zz_block_probe",
"external_job.zz_block_probe", "git.zz_block_probe", "graphify.zz_block_probe",
"hooks.zz_block_probe", "intel.zz_block_probe", "review.zz_block_probe",
"statusline.zz_block_probe", "workflow.zz_block_probe", "zz_top_level_probe"]
  left: 12
 right: 16
```

After the four `take` calls: `ok`, alongside the pre-existing
`an_unmodelled_key_becomes_a_read_only_row_under_its_own_category`.

**Untrusted-text discipline unchanged.** The pass-through key still crosses to a
`Span` through `shown()` at the list site (`key_span` in `render_defaults_tab`);
`render_escape_guard.rs` was not touched, not weakened and not re-pinned; the
shared pass-through `ConfigHelp` stays a static literal with the key never
interpolated into it, so `build_config_help_pane` keeps its structural property
(T-VQW-01). The four new keys are four more rows through the boundary the
original item already built, not a new boundary.

**Gates.** `cargo build` exit 0. `cargo clippy -- -D warnings` exit 0.
`cargo test --no-fail-fast`: lib 1279 passed / 1 failed / 1 ignored — the one
failure is
`envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`,
the pre-existing environmental git-version mismatch already recorded in project
memory; every other target green.

**Judgement calls (human unavailable).**

1. **The test is a census over all fifteen blocks, not just the four the gap
   named.** A four-block test would go green and leave the sixteenth block, when
   it is added, free to repeat the bug verbatim — the defect's real shape is "the
   walk is a hand-maintained list that drifts from the struct", not "four names
   are missing". The census costs one const and fails naming the missing block.
2. **The test asserts a RENDERED cell, not only a `ConfigEntry`.** The must-have
   says "visible in the Defaults tab"; an entry-vector assertion would certify the
   data structure while a layout or scroll bug kept the row off screen, which is
   the same invisibility from the operator's seat.
3. **`render_defaults_to_text` was refactored to delegate rather than
   duplicated.** One extra test-only helper; the existing signature and all its
   callers are unchanged.
4. **Scope held to `src/ui/screens/detail.rs`.** No second file was needed — both
   the walk and its test module live there. `src/state_reader/` was not touched.
