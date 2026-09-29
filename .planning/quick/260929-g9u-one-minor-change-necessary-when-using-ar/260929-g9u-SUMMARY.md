---
quick_id: 260929-g9u
phase: quick-260929-g9u
plan: 01
subsystem: ui/detail (Config tab)
tags: [tui, keyboard, focus, config, scope]
status: complete
requires: [quick-260927-t3s scope strip, quick-260926-1t1 tab-bar focus level, quick-260926-2l4 Pane focus level]
provides: [DetailFocus::ScopeStrip, handle_config_scope_key, config_scope_footer_spans, focused config_scope_strip]
affects: [src/ui/screens/detail.rs, src/ui/screens/help.rs]
tech-stack:
  added: []
  patterns: [one-tab focus level modelled on DetailFocus::Pane; Some-consumes/None-falls-through key handler]
key-files:
  created: []
  modified: [src/ui/screens/detail.rs, src/ui/screens/help.rs]
decisions:
  - "Down/j/Enter/Space at the 7:Cfg tab bar land on a new DetailFocus::ScopeStrip level; Left/Right there select Project/Global (clamped) via set_config_scope"
  - "Up/k on the first visible Config row climbs to the strip; Esc from the rows still goes straight to the tab bar"
  - "Config rows footer leads with [↑]scope"
requirements: [QUICK-260929-g9u]
metrics:
  duration_minutes: 8
  completed: 2026-09-29
actuals:
  tokens: 8356
  tasks: 3
  commits: 6
plan_head_before: d95e2c6b0793326944cc3e490f592a2e89148ddb
plan_head_after: ff3a19d093ed7a0b91ed97479aa66e689a0b5289
---

# Quick 260929-g9u: `↓` from the 7:Cfg tab bar lands on the Project │ Global strip, and the arrows switch scope there — Summary

The Config tab's scope strip is now a keyboard focus level (`DetailFocus::ScopeStrip`). You reach it with `↓` from the tab bar or `↑` from the first config row. It shows a cyan bold `▸` cue and a `←/→/g switch` hint. `←`/`→` there select Project/Global through the same `set_config_scope` that `g` and the click use.

## Commits

| # | Sha | Subject |
|---|-----|---------|
| 1 RED | f44c28a | test(quick-260929-g9u): add failing scope-strip focus tracer |
| 1 GREEN | d73d388 | feat(quick-260929-g9u): arrow down at the Config tab bar focuses the scope strip; arrows switch scope |
| 2 RED | ea0f68c | test(quick-260929-g9u): add failing scope-strip level-key tests |
| 2 GREEN | 6e87c2b | feat(quick-260929-g9u): scope strip level keys and up-from-first-row climb |
| 3 RED | c209096 | test(quick-260929-g9u): add failing scope-strip footer/help tests |
| 3 GREEN | ff3a19d | feat(quick-260929-g9u): scope-strip footer and help row |

## RED evidence

- **Task 1:** `cargo test --lib scope_strip` gave 2 passed, 5 failed. The tracer failed at its first assertion after `Down` (`left: Content, right: ScopeStrip`). The focused-strip test failed because the text had no `▸`. The arrows test failed because `Left` went to `TabBar`. The content-key and off-Config tests failed because focus stayed `ScopeStrip`.
- **Task 2:** 8 passed, 4 failed. The climb test failed with `Up` going to `TabBar`. The descend test failed because `Down` moved the cursor 60 to 61. The `Up`/`k`/`Esc` test failed because focus went to `Content`. The `g` test failed because focus went to `Content`. The regression guard `up_below_the_first_config_row_moves_the_cursor_not_to_the_scope_strip` passed, as the plan expected.
- **Task 3:** 13 passed, 2 failed. `the_scope_strip_footer_names_only_its_keys` and `help::tests::the_config_scope_strip_arrows_are_documented` failed (count 0, expected 1). `a_click_on_a_scope_label_from_the_scope_strip_still_switches_it` passed on RED. It is a regression guard for behaviour the plan leaves unchanged (I-10), so it could not fail.

## New tests (13)

detail.rs:
- `down_from_the_config_tab_bar_lands_on_the_scope_strip_and_right_selects_global` (tracer)
- `focused_config_scope_strip_shows_the_cursor_and_arrow_hint_and_keeps_its_click_rects`
- `left_right_on_the_scope_strip_select_a_scope_and_clamp`
- `a_content_key_on_the_scope_strip_runs_in_the_rows`
- `scope_strip_focus_off_the_config_tab_falls_back_to_content`
- `up_from_the_first_config_row_lands_on_the_scope_strip_and_down_returns_without_moving`
- `up_below_the_first_config_row_moves_the_cursor_not_to_the_scope_strip`
- `up_k_and_esc_leave_the_scope_strip_for_the_tab_bar`
- `descend_keys_leave_the_scope_strip_without_opening_an_editor`
- `g_on_the_scope_strip_switches_scope_and_keeps_the_strip_focused`
- `the_scope_strip_footer_names_only_its_keys`
- `a_click_on_a_scope_label_from_the_scope_strip_still_switches_it`

help.rs:
- `the_config_scope_strip_arrows_are_documented`

## Existing-test adjustments

- **E-1 `config_scope_strip_marks_the_active_scope`:** the two calls changed from `config_scope_strip(T)` to `config_scope_strip(T, false)`. The doc comment now says "unfocused". Every assertion is unchanged, including the two no-arrow assertions.
- **E-2 `test_other_footers_unchanged_by_browse_edit_hint`:** the lead of the Defaults expected string changed from `  [↑]tab bar  [←/→]tabs …` to `  [↑]scope  [←/→]tabs …`. The rest of the string and the Backlog assertion are unchanged.
- **E-3 `the_tabs_hint_drops_shift_d_on_every_tab_when_experimental_is_off`:** the prefix used to be `  [↑]tab bar  {arrows}  [1-8]jump  ` for every view. It is now `  {up}  {arrows}  [1-8]jump  `, where `up` is `[↑]scope` for Defaults and `[↑]tab bar` for every other view.
- **E-4 `content_footers_lead_with_the_tab_bar_and_the_arrow_meaning`:** the same per-view `up` token replaces the fixed `[↑]tab bar` in the prefix.

No other existing test failed. These pass unmodified:
- `mouse_click_on_a_config_scope_label_switches_it`
- `g_switches_the_config_scope_and_writes_land_in_that_scopes_file`
- `the_g_config_scope_switch_is_documented`
- `the_detail_navigation_block_is_documented_as_whole_rows`
- `up_on_the_first_row_goes_to_the_tab_bar_and_down_returns_without_moving`

## Inferred decisions (for audit)

I-1..I-12 were applied as the plan states them:
- **I-1:** `ScopeStrip` is a `DetailFocus` variant modelled on `Pane`, not a view-cache flag.
- **I-2:** the arrows are directional and clamped, go through `set_config_scope`, and set the same status message as `g`.
- **I-3:** you reach the strip with the tab bar's whole descend set on Config, or with `↑`/`k` on the first visible row. A digit still lands on the rows.
- **I-4:** `↓`/`j`/`Enter`/`Space` go to the rows with no other change. `↑`/`k`/`Esc` go to the tab bar, and `Esc` does not clear the filter.
- **I-5:** on the rows, `Esc` still goes straight to the tab bar, `←`/`→` still switch tabs, and the wheel guard is unchanged.
- **I-6:** `g`/`d` switch scope and keep the strip focused. `?`, `Tab`, `q` and `M` are unchanged. Every other key drops to the rows and runs there.
- **I-7:** the focus cue is a cyan bold `▸` in the gutter plus the `←/→/g switch` hint. It is width-neutral, and the row highlight stays drawn.
- **I-8:** there is a strip footer, and the Config rows footer leads with `[↑]scope`.
- **I-9:** there is one new help row.
- **I-10:** clicks are unchanged, so focus goes to Content.
- **I-11:** the entry points are unchanged.
- **I-12:** no contributions file was used.

Executor-level decisions:
- **X-1:** in the Task 1 RED commit, `config_scope_strip` and `config_scope_row` already took the `focused` parameter, and `config_scope_strip` ignored it (`_focused`). This was needed so the pure test could compile. E-1's `, false` also landed in that commit rather than the GREEN one. The behaviour was still RED; see the evidence above.
- **X-2:** the Task 3 RED commit added `config_scope_footer_spans()` as a stub that returns an empty `Vec`, for the same compile reason. The GREEN commit replaced it, so no stub remains.
- **X-3:** the tracer reads the strip row starting at `strip.x`, rather than from the start of the `buffer_row`. This keeps it independent of the content area's x offset. The assertion is equivalent when `strip.x == 0`.
- **X-4:** the new tests share small helpers named with `scope_strip`: `on_the_scope_strip`, `scope_strip_status`, `scope_strip_target`, `scope_strip_cursor` and `scope_strip_park_cursor`. With these, `cargo test scope_strip` selects exactly this set.
- **X-5:** the commits landed directly on `master`, as the coordinator instructed. Isolation was forced off, and the project's integration branch is `dev`. The executor's protected-branch assertion would normally refuse `master`, and `gsd-tools` was not on PATH to query the override. The explicit coordinator instruction was followed.

## Gates

- `rtk proxy cargo test --no-fail-fast`: **2824 passed, 1 failed, 15 ignored.** The baseline was 2811 / 1 / 15, plus the 13 new tests. The only failure is the known local witness `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`. The counts were totalled from the per-target `test result:` lines of the saved log.
- `rtk proxy cargo clippy --all-targets -- -D warnings`: exit 0.

## Deviations from Plan

None beyond X-1..X-5 above. No auth gates.

## Known Stubs

None.

## Threat Flags

None. No new file or network surface was added. Scope changes still go only through `set_config_scope` (T-g9u-01). The new tests that switch to Global use `ctx_on_config_row`, or set `defaults_user_path = None` (T-g9u-02).

## Self-Check: PASSED

- src/ui/screens/detail.rs and src/ui/screens/help.rs were modified and committed.
- The commits f44c28a, d73d388, ea0f68c, 6e87c2b, c209096 and ff3a19d exist on master (`git rev-list --count d95e2c6..HEAD` = 6).
