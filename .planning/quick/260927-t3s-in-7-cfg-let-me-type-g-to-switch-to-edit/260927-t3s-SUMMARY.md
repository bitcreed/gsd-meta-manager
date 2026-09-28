---
quick_id: 260927-t3s
phase: quick-260927-t3s
plan: 01
status: complete
subsystem: ui/detail — 7:Cfg tab
tags: [config, keybinding, scope-strip, mouse, tdd]
requires: []
provides:
  - "`g` (alias `d`) switches the Config tab Project <-> Global"
  - "config_scope_strip / config_scope_row: always-visible scope strip"
  - "ProjectViewCache.defaults_user_path + config_write_path: testable global write seam"
  - "config_json::load_user_defaults_from(path)"
  - "ClickTarget::ConfigScope + DetailRegions.config_scope_tabs"
affects: [src/ui/screens/detail.rs, src/ui/screens/mod.rs, src/ui/screens/help.rs, src/state_reader/config_json.rs]
tech-stack:
  added: []
  patterns: ["one scope-setting helper shared by key and click", "resolve-once path stored in view cache"]
key-files:
  created: []
  modified:
    - src/state_reader/config_json.rs
    - src/ui/screens/mod.rs
    - src/ui/screens/detail.rs
    - src/ui/screens/help.rs
decisions:
  - "Scope indicator is a two-label strip ` [Project] │ Global` above the Config content (plus a symmetric title), not a title-only cue (I-3)"
  - "Global path resolved once into ProjectViewCache.defaults_user_path; persist writes to it, never re-reads $HOME (I-7)"
  - "`d` kept as a working alias documented only in help (I-2)"
metrics:
  duration: "~12 min"
  completed: 2026-09-27
estimate:
  tokens: 180000
  tasks: 3
actuals:
  tokens: 13734
  tasks: 3
  commits: 4
plan_head_before: c0ef25731790dfd5dfd6a48b9385b6f384fee7db
plan_head_after: 6c93cae
---

# Quick 260927-t3s: `g` switches the 7:Cfg tab between project and global settings Summary

On the Config tab, `g` (and its old key `d`) now switches editing between `.planning/config.json` and `~/.gsd/defaults.json`. A scope strip ` [Project] │ Global   g switch   * = inherited from global` / ` Project │ [Global]   g switch` is drawn above the list, the empty-config message and every popup. The title names the file, the footer and help advertise `g`, and both labels can be clicked. The global file is resolved once into the view cache, and writes go through a pure `config_write_path`, so tests prove with TempDirs that each scope writes only its own file.

## Commits

| Task | Commit | Subject |
|------|--------|---------|
| 1 RED | 92cd991 | test(quick-260927-t3s): add failing g-scope tracer test |
| 1 GREEN | 46b2c25 | feat(quick-260927-t3s): g switches Config scope with a visible scope strip |
| 2 | e71121f | feat(quick-260927-t3s): advertise g in the Config footer and help |
| 3 | 6c93cae | feat(quick-260927-t3s): click a Config scope label to select it |

## What was built

- **Persistence seam (I-7).**
  - `config_json::load_user_defaults_from(path)` was added, and `load_user_defaults()` now delegates to it.
  - The Config arrival block and the `r` reload both set `cache.defaults_user_path = user_defaults_path()` and then load from that path.
  - `persist_active_config` gained a `user_defaults_path` parameter and resolves its destination through the pure `config_write_path(target, project_path, user_defaults_path)`. It no longer calls `user_defaults_path()`.
  - All five call sites clone `cache.defaults_user_path` next to `target`.
- **Scope switch.** `set_config_scope(cache, target) -> bool` holds the old `d` arm's body: the guard while a chooser or prompt is open, the global bootstrap and the cursor snap. It returns false when nothing changes, which makes clicks idempotent. `config_scope_status` returns the two I-11 messages. A single arm, `KeyCode::Char('g') | KeyCode::Char('d') if current_view == DetailSubView::Defaults`, replaces the old `d` arm.
- **Strip and title.**
  - `two_sub_tab_strip` is now built on `sub_tab_strip_spans` (gutter, label, separator, label). The Docs and Sessions strips are byte- and style-identical, and their existing tests pass unmodified.
  - `config_scope_strip(target)` uses cyan for an active Project label and magenta for an active Global label.
  - `config_scope_row` carves row 0 and records it as `sub_tab_strip` and the two labels as `config_scope_tabs`. `render_defaults_tab` calls it before anything else.
  - The Project title is now ` Project Config (.planning/config.json) `.
- **Footer and help.**
  - The footer's `[d] defaults` became `[g] project/global`.
  - Help has a new row: `g` — "Config tab: switch between project and global (~/.gsd/defaults.json) settings; d is an alias (detail view)".
- **Mouse (I-8).**
  - Added `ConfigScopeRegion`, `DetailRegions.config_scope_tabs` and `ClickTarget::ConfigScope`. The new target is checked after sub-tabs and still sits below the modal chooser.
  - `strip_label_rects(row, &strip)` is shared by `sub_tab_row` and `config_scope_row`.
  - The click arm goes through `set_config_scope`.

## RED evidence

`g_switches_the_config_scope_and_writes_land_in_that_scopes_file` failed on the pre-fix code at the **row-0 scope-strip assertion** (detail.rs:19288). Row 0 was `" Config Settings ───…"`. That is before the `g` press, so no write to the real global file could happen.

## Inferred decisions (for audit)

The planner's I-1..I-13 were applied as written:

- **I-1** Global means `~/.gsd/defaults.json`.
- **I-2** `d` stays as an alias that only the help popup names.
- **I-3 (scope indicator: two-label tab strip plus title, not title only).** The user's suggestion (D-03) was adopted. Rationale:
  - (a) The strip reuses the Docs and Sessions strip language and helper.
  - (b) It shows both scopes, so Global can be discovered from project scope.
  - (c) It gets its own row, laid out before the empty message, the list and both popups, so it cannot be covered while a value is being edited. This is D-02's hardest case, and `config_scope_stays_visible_while_editing` covers it at 80 and 120 columns.
  - (d) The bracketed, reversed label reads in monochrome and can be asserted from a text scrape.
  - Cost: one row of height.
  - The title was kept and made symmetric (I-13), so the scope reads twice.
- **I-4** Global is magenta. Project is cyan.
- **I-5** Arrows and `[`/`]` do not switch scope, so `sub_tab_pair` is unchanged.
- **I-6** The project strip carries the `* = inherited from global` legend. The global strip does not.
- **I-7** The path is resolved once into the cache. `None` writes nothing.
- **I-8** The labels are clickable. A click sets the scope idempotently.
- **I-9** `g` is inert while a chooser is open, and is plain text in the prompt and in the filter.
- **I-10** Scope is per project and in memory only.
- **I-11** The footer and status strings are as specified.
- **I-12** Only literal paths are drawn.
- **I-13** The project title was renamed.

Executor-level inferences:
- **X-1** I updated the `defaults_user_config` doc in mod.rs (it now names `g` with `d` as alias) in the RED commit, together with the new field, instead of in GREEN. It is a doc comment only.
- **X-2** Plan Task 3 asked for `config_scope_tabs: Vec::new()` in both test `DetailRegions` literals. The second literal (the wheel test) already uses `..DetailRegions::default()`, so only the first needed the field.
- **X-3** I added a small test helper, `park_config_cursor(ctx, key)`, to park the cursor on a row in the active scope. It is used by `config_scope_stays_visible_while_editing`.
- **X-4** The `config_filter_typing_swallows_shortcut_keys` doc comment was reworded. It used to say `d` would persist to the real `~/.gsd`, which is no longer true, because an unresolved path writes nothing (I-7).
- **X-5** Commits were made directly on `master`, as the orchestrator instructed (ISOLATION=none). The executor's protected-branch check was deliberately not applied.

**Key collision check for `g` on Config: no collision** (re-verified during execution). The text-edit and filter intercepts run first, so `g` is text there. The tab-bar level does not match `g`. The Waves `g` is Phases-pane only. The other `g` arms are guarded by Browse and by `roadmap_list`.

## Existing tests adjusted (E-n)

- **E-1 `config_tab_shows_the_effective_gsd_install`**
  - Before: `rows[0]` contains ` Config Settings `.
  - After: `rows[1]` contains ` Project Config (.planning/config.json) `. The other asserts read `rows[1]`.
- **E-2 `config_tab_shows_a_missing_gsd_install`**
  - Before: `rows[0]` holds the "GSD not found" label.
  - After: `rows[1]` holds it.
- **E-3 `config_tab_empty_branch_shows_the_gsd_install_on_a_second_line`**
  - Before: `rows[0]` "No config loaded", `rows[1]` install.
  - After: `rows[0]` starts with ` [Project]`, `rows[1]` "No config loaded", `rows[2]` install.
- **E-4 `config_tab_without_a_project_state_draws_no_gsd_install`**
  - Before: `lines().next()` contains ` Config Settings `.
  - After: `lines().nth(1)` contains ` Project Config (.planning/config.json) `.
- **E-5 `config_filter_typing_swallows_shortcut_keys`**
  - Before: keys `…'k','d'` gave filter `"xq3r?jkd"`.
  - After: keys `…'k','g','d'` give filter `"xq3r?jkgd"`, with `d` still last.
- **E-6 `test_other_footers_unchanged_by_browse_edit_hint`**
  - Before: the Defaults footer had `[d] defaults`.
  - After: it has `[g] project/global`.
- **E-7 `render_records_the_regions_a_mouse_hit_test_needs`**
  - Extended: Queue records 0 `config_scope_tabs`; Defaults records `sub_tab_strip == Rect(0,3,120,1)` and 2 `config_scope_tabs`.
- **E-8 `mouse_click_target_is_a_pure_function_of_the_regions`**
  - Extended: the literal gets `config_scope_tabs: Vec::new()`.
  - A `ConfigScopeRegion` at (12,3,8,1) resolves to `ConfigScope(Global)` at (14,3).
  - An open dropdown still wins (`DropdownOutside`).

No other Config test needed a height bump. No assertion was loosened or deleted.

## New tests

- `config_json::load_user_defaults_from_reads_a_given_path`
- `g_switches_the_config_scope_and_writes_land_in_that_scopes_file` (tracer; two TempDirs)
- `config_write_path_resolves_per_scope`
- `arrival_resolves_the_global_defaults_path` (read-only)
- `g_is_inert_while_a_config_chooser_is_open`
- `g_is_text_while_typing_a_config_value`
- `config_scope_stays_visible_while_editing`
- `config_scope_strip_marks_the_active_scope`
- `config_empty_branch_keeps_the_scope_strip`
- `help::the_g_config_scope_switch_is_documented`
- `mouse_click_on_a_config_scope_label_switches_it`

Test safety (T-t3s-02): the only test that mutates in Global scope is the tracer. It asserts that `defaults_user_path` starts with a TempDir path before its first key. Every other Global-scope test either never mutates or has `defaults_user_path == None`.

## Gates

- `rtk proxy cargo test --no-fail-fast`: **2811 passed, 1 failed, 15 ignored.** The baseline was 2800 passed, 1 failed, 15 ignored, and this plan added 11 tests.
  - The only failure is `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`, the known local git-version witness.
- `rtk proxy cargo clippy --all-targets -- -D warnings`: **exit 0.**
  - The first run flagged `clone_on_copy` on `chooser.dropdown.clone()` in the new test. I fixed it before the Task 3 commit and re-ran the 39 `detail::tests::mouse*` tests, all green.

## Deviations from Plan

None beyond X-1..X-5 above. There were no Rule 1-4 auto-fixes to production code outside the plan.

## Known Stubs

None.

## Threat Flags

None. No surface beyond the plan's threat model: the only filesystem writes are the two already-modelled config files.

## Self-Check: PASSED

- Files modified exist: src/state_reader/config_json.rs, src/ui/screens/mod.rs, src/ui/screens/detail.rs, src/ui/screens/help.rs.
- Commits present: 92cd991, 46b2c25, e71121f, 6c93cae (`git log` on master).
