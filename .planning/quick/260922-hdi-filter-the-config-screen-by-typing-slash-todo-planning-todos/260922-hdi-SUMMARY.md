---
phase: quick-260922-hdi
plan: 01
status: complete
subsystem: ui/detail (Config Settings tab)
tags: [tui, filter, config, keyboard]
requires: []
provides:
  - "`/` filter on the Config Settings (Defaults) tab: narrows rows by key/category, echoes `/query_ (n/total)` in the list title"
affects: [src/ui/screens/detail.rs, src/ui/screens/mod.rs]
tech-stack:
  added: []
  patterns:
    - "filtered list over UNDERLYING indices: the cursor stays an index into entries_for_cache; only nav, render and two guards read the filter"
    - "typing-mode intercept before `match code` (mirrors NormalScreen::handle_search_key)"
key-files:
  created: []
  modified:
    - src/ui/screens/mod.rs
    - src/ui/screens/detail.rs
decisions:
  - "[INFERRED A1] match key + category only, case-insensitive substring; help text and value excluded"
  - "[INFERRED A2] Enter while typing only confirms; the next Enter edits"
  - "[INFERRED A3] q clears a confirmed filter like Esc (shared arm)"
  - "[INFERRED A4] `/` reopens the input seeded with the current filter"
  - "[INFERRED A5] filter echo + count live in the list block title; footer only gains a static [/]filter"
  - "[INFERRED A6] each filter-text change jumps to the first match; /, d, r only snap a hidden cursor"
  - "[INFERRED A7] arrows/PgUp/PgDn navigate while typing; j/k are text until confirmed"
  - "[INFERRED A8] d and r keep the filter; tab re-arrival clears it"
metrics:
  duration: "~70 min"
  completed: 2026-09-22
plan_head_before: 0e112b225267907a30b41836867be01f06194e20
actuals:
  tokens: 10442
  tasks: 3
  commits: 5
---

# Quick 260922-hdi Plan 01: `/` filter on the Config Settings tab Summary

A vim/less-style `/` filter on the Config Settings tab. It narrows the ~130 rows by key or category, case-insensitively, and echoes `/query_ (visible/total)` in the list title, escaped via `shown()`. Esc/q clear it, and Enter, x and the arrows act on the filtered row by its underlying index. The ConfigValueKind::Unset Enter-opens-chooser fix is re-proved through the filter.

## Commits

| Task | Commit | Subject |
|------|--------|---------|
| 1 RED | 8a0c732 | test(quick-260922-hdi): pin / filter on the Config tab end to end, RED |
| 1 GREEN | 93ca479 | feat(quick-260922-hdi): / filters the Config tab by key and category |
| 2 RED | f7044f7 | test(quick-260922-hdi): pin Esc/q clearing, empty-result inertness, arrival reset, RED |
| 2 GREEN | f41a6bd | feat(quick-260922-hdi): Esc/q clear the Config filter; empty result is inert |
| 3 | b90ef32 | feat(quick-260922-hdi): [/]filter hint in the Config tab footer |

## What was built

- **State** (`mod.rs`, `ProjectViewCache`): `defaults_filter: String` and `defaults_filter_typing: bool`. The fields are additive and the struct still derives `Default`.
- **Helpers** (`detail.rs`, beside `entries_for_cache`): `config_row_matches`, `visible_defaults_indices`, `move_defaults_selection` (identical to the old arithmetic when the filter is empty), `snap_defaults_selection`, `select_first_visible` and `defaults_selection_visible`.
- **Keys**:
  - A typing intercept sits after the String-edit intercept and before `match code`. It hands every key to `handle_config_filter_key`, which always returns `ScreenAction::None`.
  - A new `/` arm opens the input.
  - The nav arms (Down/j, Up/k, PageDown, PageUp) call `move_defaults_selection` when no editor is open.
  - Esc/q clear a confirmed filter after the popup-close check and before Pop.
  - Enter and x skip a cursor that the filter hides.
  - `d` and `r` snap a hidden cursor onto a visible row.
  - Arriving on the tab clears the filter and the input focus.
- **Render**:
  - Items are built from `visible`. The category column restarts on each visible run.
  - The highlight compares underlying indices. `ListState` selects the visible position.
  - The title gets ` /<shown(filter)>[_] (v/total) `.
  - A filter that matches nothing draws a dim "No config keys match" and no help pane.
  - With no filter and no input focus, the render and title are unchanged.
- **Footer**: `[/]filter` added after `[r]eload`, and the pinned string was updated.

## Tests added (9, all `config_filter_*`, appended after the Unset block)

1. `config_filter_enter_on_a_filtered_unset_row_opens_that_rows_chooser` (tracer: query contains x/d/r; the pick lands on that key only)
2. `config_filter_arrows_move_only_among_matching_rows` ("drift" and "DRIFT")
3. `config_filter_render_shows_only_matches_and_count`
4. `config_filter_typing_swallows_shortcut_keys`
5. `config_filter_esc_clears_typing_then_confirmed_filter_then_pops`
6. `config_filter_empty_result_is_inert`
7. `config_filter_x_clears_the_filtered_rows_key_only`
8. `config_filter_arrival_and_target_toggle_keep_cursor_consistent`
9. `config_filter_echo_is_escaped`

Tests 4, 7 and 9 passed at their Task 2 RED commit, because they pin behaviour Task 1 had already built (the plan expected this). Tests 5, 6 and 8 failed there for the intended reasons: Esc popped, Enter stepped a hidden Integer row, and arrival kept the filter.

## Verification

- `rtk proxy cargo test --no-fail-fast`: 2140 passed, 1 failed across 48 suites. The only FAILED test is `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`, which fails locally by design.
- One intermediate full run also failed `tests/envelope_carrier_reach.rs::the_t_19_116_replacement_takes_layer_2_as_well_and_that_is_measured_separately` with `ETXTBSY` ("Text file busy" spawning a freshly built binary), a spawn race. That suite passed three times in isolation, and the next full run was clean. It is unrelated to this change.
- `cargo clippy -- -D warnings` exits 0. `cargo clippy --all-targets` exits 0, with nothing reported in detail.rs or mod.rs. Its remaining warnings are pre-existing ones in src/browser.rs, src/project_creator.rs and tests/envelope_*.rs.
- The Unset regression block (145 lines from `debug enter-on-unset-config-row`) is byte-identical to 9ef0f7f. Both of its tests pass.
- Footer pin `test_other_footers_unchanged_by_browse_edit_hint` passes.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Removed `entries_count_for_cache`**
- **Found during:** Task 1
- **Issue:** Its only callers were the nav arms that now call `move_defaults_selection`, so the function would have been dead code and a clippy warning.
- **Fix:** Deleted the function.
- **Commit:** 93ca479

**2. [Convention] rustfmt applied only to lines this plan added**
- **Found during:** Task 3
- **Issue:** detail.rs already had 218 rustfmt hunks before this plan, and a whole-file format would have touched unrelated code.
- **Fix:** Applied only the hunks that fall entirely within this plan's changed lines. 216 pre-existing hunks remain.
- **Commit:** b90ef32, folded into the footer commit.

**3. [Minor] Added a `defaults_selection_visible` helper**
- **Found during:** Task 2
- **Issue:** The plan's inline "`visible` does not contain `selected`" guard was needed in both the Enter and x arms.
- **Fix:** Factored it into one helper that both arms call. It always returns true with an empty filter, so the unfiltered paths are unchanged.
- **Commit:** f41a6bd

## Inferred decisions (for audit)

A1-A8 are listed in the frontmatter `decisions`, as the plan specified. Help screen (`help.rs`) left unchanged.

## Known Stubs

None.

## Threat Flags

None. No new surface beyond the plan's threat model (T-HDI-01..07). All mitigations are pinned by tests.

## Self-Check: PASSED

- FOUND: src/ui/screens/mod.rs, src/ui/screens/detail.rs
- FOUND commits: 8a0c732, 93ca479, f7044f7, f41a6bd, b90ef32
