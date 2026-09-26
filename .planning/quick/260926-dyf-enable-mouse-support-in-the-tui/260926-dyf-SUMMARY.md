---
quick_id: 260926-dyf
phase: quick-260926-dyf
plan: 01
subsystem: tui
status: complete
tags: [mouse, tui, crossterm, ratatui, detail-view, dashboard]
requires: [quick-260926-1t1, quick-260926-2l4]
provides:
  - "ui::mouse pure module (reader filter, ClickTracker, ListRegion, tab_entry_rects, mouse_status_text)"
  - "terminal mouse-capture lifecycle (tui.rs / main.rs), preferences.mouse, M toggle"
  - "dashboard + detail-view click / double-click / wheel"
affects: [src/ui/screens/detail.rs, src/ui/screens/normal.rs, src/app.rs, src/main.rs]
tech-stack:
  added: []
  patterns:
    - "hit-test rects recorded at render time through &self (Cell/RefCell), mapped by pure functions in the event handler"
    - "double-click = Enter on the row the FIRST click armed, through the unchanged keyboard path"
key-files:
  created:
    - src/ui/mouse.rs
  modified:
    - src/config.rs
    - src/action.rs
    - src/event.rs
    - src/tui.rs
    - src/main.rs
    - src/app.rs
    - src/ui/mod.rs
    - src/ui/screens/mod.rs
    - src/ui/screens/normal.rs
    - src/ui/screens/detail.rs
    - src/ui/screens/render_escape_guard.rs
    - src/ui/screens/help.rs
    - README.md
    - docs/GETTING-STARTED.md
    - docs/CONFIGURATION.md
decisions:
  - "preferences.mouse (JSON bool, serde default fn -> true) is the startup capture state; M toggles at runtime, never saved"
  - "Only Down(Left), ScrollUp, ScrollDown enter the Action FIFO; everything else is dropped in the reader"
  - "Double-click runs Enter only on the row the first click selected (armed flag), never re-hit-tests"
  - "Detail view shows status messages right-aligned in the tab bar title row, escaped at the render site"
metrics:
  duration: "~30 min (10:17-10:47 local)"
  completed: 2026-09-26
plan_head_before: 39a91fbcd970dbb6c6c39bc8383e8f975eab5d3e
actuals:
  tokens: 30400
  tasks: 3
  commits: 4
---

# Quick 260926-dyf: mouse support in the TUI — Summary

Terminal mouse capture (on by default via `preferences.mouse`, `M` toggles it live) with click-to-select/switch, double-click-as-Enter and wheel-under-pointer on the dashboard and every detail-view tab bar, sub-tab strip, Phases/Sessions/Agents list and Waves-pane row. Hit tests are pure functions over rects recorded at render time. Capture is always turned off on quit, on panic (a once-installed hook) and across the `$VISUAL` hand-off.

## Commits

| # | Task | Commit | Subject |
|---|------|--------|---------|
| 1 | Task 1 (tracer) | `5a1717d` | feat(quick-260926-dyf): mouse capture lifecycle, preferences.mouse, M toggle and dashboard click, double-click and wheel |
| 2 | Task 2 | `001c78b` | feat(quick-260926-dyf): detail view clicks — tabs, sub-tabs, list and Waves-pane rows, double-click as Enter |
| 3 | Task 3 (code) | `e54ab55` | feat(quick-260926-dyf): detail wheel routing, M in the detail view and the status title |
| 4 | Task 3 (docs) | `d8740b7` | docs(quick-260926-dyf): mouse controls in help, README, GETTING-STARTED and CONFIGURATION |

All four are on master and have not been pushed. The commit count comes from `git rev-list --count 39a91fb..HEAD` and is 4.

## Measured gates

| Gate | Result |
|------|--------|
| Baseline at 39a91fb | 55 suites, **2621 passed / 1 failed / 15 ignored** (see E-9) |
| After Task 1 | 2636 / 1 / 15 |
| After Task 2 | 2646 / 1 / 15 |
| After Task 3 (final) | **55 suites, 2654 passed / 1 failed / 15 ignored**. This is the baseline plus 33 new `mouse_*` tests |
| The single failure | `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`. It is the expected git-version witness on this machine |
| `rtk proxy cargo clippy -- -D warnings` | exit 0 after each task |
| `rtk proxy cargo clippy --all-targets` | exactly the 11 pre-existing findings: browser.rs ×3, project_creator.rs ×1, envelope_wrapper_class.rs ×4, and one each in envelope_carrier_reach.rs, envelope_config_resolution.rs and envelope_control_carrier.rs. There are none in touched files |

The counts were read from the raw `test result:` lines of `rtk proxy cargo test --no-fail-fast`, which was written to a file and summed there, never piped through grep.

New tests (33), all named `mouse_*`:

| Where | Count | Tests |
|-------|-------|-------|
| ui/mouse.rs | 5 | reader filter, double-click window, list-region offset, tab-rect layout, status text |
| config.rs | 1 | `mouse_preference_defaults_to_true_from_default_and_from_empty_json` |
| normal.rs | 5 | click, double, wheel clamp, persisted offset, search ignore |
| app.rs | 4 | toggle key, `mouse:false` start, ignored while off, quick second press is Enter / a key between breaks it |
| detail.rs | 16 | 10 click tests + 6 wheel / `M` / status-title tests |
| render_escape_guard.rs | 1 | `mouse_status_title_state_reaches_the_title` |
| help.rs | 1 | `mouse_help_block_is_documented_as_whole_rows` |

**Changed or retired tests:** none. Persisting the offsets of the phase list, Sessions, Agents and the dashboard table changed no existing expectation, so there is no old → new mapping to report. The existing `render_records_the_regions_a_mouse_hit_test_needs`, `waves_pane_*` and `cross_jump_*` tests all pass unchanged.

## Final mouse map

**Dashboard** (`NormalScreen::handle_mouse`)

| Input | Effect |
|-------|--------|
| Click on a project row | Selects it. The header row, the footer and the empty state do nothing |
| Double-click | `Enter` on the row the first click selected: it opens the detail view |
| Wheel over the table block | Moves the selection ±1, clamped at both ends. It never wraps |
| `M` | Toggles capture and shows `Mouse off (M toggles)` or `Mouse on (M toggles) — Shift+drag selects text` in the footer |
| Anything while `/` search is typed | Ignored |

**Detail view** (`DetailScreen::handle_mouse`). Clicks are resolved in this priority order:

| Input | Effect |
|-------|--------|
| Click on a tab | Same as that tab's digit or Shift+D: focus goes to Content, then `switch_to_tab`, sub-tab memory included. The `‹` and `›` overflow markers do nothing |
| Click on a sub-tab label | Focus goes to Content, then `switch_to_sub_view` when the label is not the current sub-tab |
| Click on a Waves-pane row | Focus goes to Pane and `waves_cursor` moves to that row's target |
| Click on a list row | Focus goes to Content and the row is selected. On Phases this sets `pipeline_selected`, sets `waves_cursor = None` and shares the selection. On Sessions it sets `sessions_selected`, and on Agents it sets `agents_selected` |
| Click elsewhere in the Waves pane | Focuses Pane |
| Click elsewhere in content | Focuses Content |
| Click on the tab-bar gap or the footer | Nothing |
| Double-click | `Enter` on the first click's row, through `handle_key`. That covers session resume, both Agents <-> Waves-pane cross-jumps, wave fold / unfold, and phase list -> pane |
| Double-click on a tab or empty space | Counts as a single click |
| Wheel over the Waves pane | Focuses Pane, then does the pane's `Down` / `Up` |
| Wheel over other content | Focuses Content, then `Down` / `Up`. Wheel-up at the first row is a no-op, so it never climbs to the tab bar |
| Wheel over the tab bar, the sub-tab strip or the footer | Ignored, so the wheel never switches tabs |
| `M` | Focus-neutral at TabBar, Content and Pane. It is placed after the Config text-edit and filter intercepts, so a typed `M` stays text |
| Status message | Drawn right-aligned in the tab bar's title row |
| Any mouse input while a Config String edit or the Config `/` filter is being typed | Ignored |

**Modal, overlay and text-entry screens** keep the trait default, `ScreenAction::None`. A click can never confirm a dialog.

## Key bindings added

| Key / input | Scope |
|-------------|-------|
| `M` (Shift+m) | Dashboard, outside `/` search, and the detail view at every focus level. It is not bound on modal screens |
| Click / Double-click / Wheel | Dashboard and detail view |
| Shift+drag | Terminal text selection while capture is on. This is terminal-dependent and is only documented, not implemented |

## tmux readings (Task 1 tracer, measured end to end)

Setup: a private tmux socket (`tmux -L dyf`) at 100x30 running `bash --norc`, a scratch `config.json` with three GSD dirs (alpha, bravo, charlie), and `VISUAL=<probe>`. The probe writes `#{mouse_any_flag}#{mouse_sgr_flag}` to a file.

| Moment | Reading |
|--------|---------|
| Shell before launch | `00` |
| TUI launched (default config, `"mouse": true` written on first save) | **`11`** |
| After `M` | **`00`**, and the footer showed `Mouse off (M toggles)` |
| After `M` again | **`11`**, and the footer showed `Mouse on (M toggles) — Shift+drag selects text` |
| SGR press `ESC[<0;5;5M` on charlie's row | `> charlie` highlighted, so injection reaches the app |
| Two presses back to back | The detail view opened (` Project: charlie`) |
| `8` (Docs › Files, STATE.md selected), then `e`: flags seen by the probe inside `$VISUAL` | **`00`** |
| After the editor returned | **`11`** |
| `q`, `q` to quit, then read at the shell | **`00`** |
| Relaunch, then Ctrl+C | `11`, then **`00`** |
| `"mouse": false` launch | **`00`** |
| `M` in that session | `11` |
| Quit | **`00`** |

These cover the default launch, the toggle, the editor hand-off, both quit paths and a `"mouse": false` launch.

Where the lifecycle lives in `src/main.rs` `run_tui_loop` (as of HEAD):

- **Line 678-679:** the editor hand-off runs `let mouse_was_on = tui::mouse_captured(); tui::restore();`.
- **Line 718:** `*terminal = tui::init();`.
- **Line 719-722:** `if mouse_was_on { let _ = tui::set_mouse_capture(true); }`. Capture is re-enabled only when it was on.
- **Top of the loop:** the sync `app.mouse_capture != tui::mouse_captured()` calls `tui::set_mouse_capture`. This is the session's first enable, and it runs after every fallible startup `?`. On error it falls back to off and sets the status `Mouse capture unavailable: {e}`.
- **Line 611:** after `run_tui_loop` returns, `tui::restore()` always disables capture.

The panic hook is in `src/tui.rs` `init()`. It is installed once through `std::sync::Once`, after `ratatui::init()` has installed ratatui's own restore hook. Because it is installed last it runs first: when capture is on it writes `DisableMouseCapture`, clears the flag, and then calls the previous hook, which is ratatui's restore and then color_eyre. This ordering holds by construction; it was not exercised with a real panic.

**Task 3 spot check** at 100x30. The scratch config also held the real `gsd-meta-manager` repo, which the startup session scan auto-registered; it was read-only here.

- A double-click opened the gsd-meta-manager detail view.
- Clicking `4:Git` gave `[4:Git]` with the commit list.
- Clicking `6:Sess` and then the `Agents` label gave `Sessions │ [Agents]`.
- Clicking `2:Phases`, then two wheel-downs over the Waves pane, gave the title `▸Waves 4 …`, the cursor `>` on the merged row, and the pane footer.
- Two wheel-downs over the phase list moved `> P14` to `> P16` and returned focus to the list.
- `M` in the detail view made the flags read `00`, and the tab bar title row read ` Project: gsd-meta-manager … Mouse off (M toggles)`.
- `M` again made the flags read `11` and showed `Mouse on (M toggles) — Shift+drag selects text`.
- After quitting, the flags read `00`.

## D-01..D-08 → proof

| ID | What it covers | Proof |
|----|----------------|-------|
| D-01 | Tab, sub-tab and row clicks, and click-to-focus | `mouse_click_on_a_tab_is_its_digit`, `mouse_click_on_a_sub_tab_switches_it`, `mouse_click_selects_a_list_row_and_focuses_content`, `mouse_click_on_a_waves_row_moves_the_pane_cursor`, `mouse_dashboard_click_selects_the_row_under_the_pointer`, and the tmux spot check |
| D-02 | Double-click is Enter | `mouse_double_click_is_enter_on_the_first_clicks_row`, `mouse_dashboard_double_click_opens_the_first_clicks_row`, `mouse_app_turns_a_quick_second_press_into_enter`, and the tmux double press |
| D-03 | The wheel scrolls the pane under the pointer | `mouse_wheel_moves_the_pane_under_the_pointer`, `mouse_wheel_up_never_climbs_to_the_tab_bar`, `mouse_wheel_never_switches_tabs`, `mouse_dashboard_wheel_moves_and_clamps`, and the tmux wheel |
| D-04 | Rects recorded at render time, pure mapping, no render I/O | `mouse_click_target_is_a_pure_function_of_the_regions`, `mouse_wheel_target_is_a_pure_function_of_the_regions`, `mouse_regions_reset_every_frame_on_both_render_paths`, `mouse_tab_rects_match_the_rendered_tab_bar`, and the no-I/O check below |
| D-05 | Capture on, and always off on exit, panic and suspend | tmux readings, the panic hook by construction, and `main.rs` 678-722 |
| D-06 | Config default and the runtime toggle | `mouse_preference_defaults_to_true_from_default_and_from_empty_json`, `mouse_preference_false_starts_with_capture_off`, `mouse_toggle_key_flips_capture_and_says_so`, `mouse_toggle_key_in_the_detail_view_is_focus_neutral`, `mouse_status_message_shows_in_the_detail_tab_bar` |
| D-07 | Help and docs | `mouse_help_block_is_documented_as_whole_rows`, plus README, GETTING-STARTED and CONFIGURATION (commit `d8740b7`) |
| D-08 | Tests | the 33 `mouse_*` tests above |

**No render I/O.** The render functions touched were:

- `NormalScreen::render` / `render_main`
- `DetailScreen::render` / `render_main_only`
- `render_pipeline_tab`, `render_sessions_tab`, `render_agents_tab`
- `sub_tab_row` (called by the Sessions, Agents, Archive and Browse renders)
- `tab_bar_block`, `record_tab_bar`, `record_list`

Each was checked. `git diff 39a91fb..HEAD -- src | grep '^+'` for `std::fs|read_to_string|read_dir|File::open|.exists()|is_file|is_dir` returns nothing. Every added line is layout arithmetic over rects that were already computed.

## Inferred decisions (for audit)

| ID | Decision | Status |
|----|----------|--------|
| I-1 | `preferences.mouse` is a JSON bool, `#[serde(default = "default_mouse")]`, and `Preferences::default()` is true. It is serialised on every save: the scratch config gained `"mouse": true` on its first save | Implemented |
| I-2 | The toggle is `M`. It is bound on the dashboard outside search and in the detail view, after the Config intercepts and before the tab-bar and pane blocks. It is not bound on modal screens and is never persisted | Implemented |
| I-3 | The status strings are `Mouse on (M toggles) — Shift+drag selects text` and `Mouse off (M toggles)`, both owned by one `mouse_status_text` | Implemented |
| I-4 | The detail view draws the status right-aligned in the tab bar's title row, on both render paths through `tab_bar_block`. It is escaped with `render_for_terminal`, cut with `fit_cells` to width − project title − 3, and omitted below 8 cells. Footers are unchanged | Implemented |
| I-5 | Capture uses crossterm `EnableMouseCapture` / `DisableMouseCapture`. `routed()` forwards only `Down(Left)`, `ScrollUp` and `ScrollDown`, and modifiers are ignored | Implemented |
| I-6 | A double is within 500 ms (inclusive), on the same row, within ±1 column. A third click starts over, and any key resets the tracker. The tracker lives in `App` | Implemented |
| I-7 | Enter runs only on the armed row the first click selected, and the second click is never re-hit-tested. A double on a tab, sub-tab or empty space is a single click | Implemented |
| I-8 | Click priority is: tab, sub-tab, Waves row, list row, Waves pane, content, nothing. A single click never runs Enter, Space or any Queue or driver action | Implemented |
| I-9 | Row clicks work on dashboard rows, the Phases list, Waves rows, Sessions and Agents. Other tabs get the wheel and click-to-focus only | Implemented |
| I-10 | One wheel event is one `Down`/`Up` step, sent as arrow codes. The wheel focuses the pane it scrolls, never climbs to the tab bar and is ignored over the tab bar, strip and footer | Implemented |
| I-11 | The dashboard wheel clamps at both ends and acts anywhere over the table block. The header row does nothing | Implemented |
| I-12 | Offsets persist across frames for the dashboard table, the phase list, Sessions and Agents, each in a `Cell<usize>`, seeded from and stored back to the widget | Implemented |
| I-13 | Mouse input is ignored during `/` search and while Config String edit or filter typing is active. The Config condition is one shared predicate (`config_text_edit_index`, `config_filter_typing`, `config_text_entry_active`) that `handle_key`'s intercepts now use too | Implemented |
| I-14 | The first enable happens inside `run_tui_loop` after all startup `?`. `restore` disables, the Once panic hook disables, and the editor suspend records the state and re-enables only when it was on. If enabling fails, capture falls back to off with a status message | Implemented |
| I-15 | No footer hint changes | Implemented |
| E-1 | The escape guard's `NormalScreen { searching: true }` literals became `{ let mut s = NormalScreen::new(); s.searching = true; s }` rather than struct-update syntax, because the new private fields make `..NormalScreen::new()` a compile error (E0451) outside `normal.rs` | Deviation, same behaviour |
| E-2 | `sub_tab_row` now takes the `DetailSubView` and builds its strip internally, instead of taking the strip `Line` and a view. The four callers pass `&DetailSubView::X`. The label rects are read from the strip's own span widths (spans 0-3), so they cannot drift from what is drawn | Minor API deviation |
| E-3 | `WheelTarget` and `wheel_target` were written during Task 2 but moved into the Task 3 commit, so each commit stays clippy `-D warnings` clean with no dead code | Commit-shape choice |
| E-4 | The `Detail view with a status message` escape-guard state is on the Queue tab, and its message is set on the chrome baseline too, so it is not artificially gated. Its `DETAIL_TAB_ARRIVAL` row is `true` because the Queue body arrives. The reason text says so and names the title row. The title's own reach is proven by `mouse_status_title_state_reaches_the_title`, which checks both directions (token present with the message, absent without it) | Honest variant of the plan's "true with a reason naming the title row" |
| E-5 | The status title is yellow. The gap to the project title is at least 3 cells, which is the plan's −3 | Styling choice |
| E-6 | The adjudication reason says "THE ESCAPE FOR THIS SURFACE LIVES AT THE RENDER SITE, `DetailScreen::tab_bar_block`", not "is escaped". The census `every_adjudication_reason_is_non_empty_and_names_values_not_verdicts` forbids the word "escaped" as a verdict | Forced by an existing census |
| E-7 | Two extra tests beyond the plan: `mouse_preference_false_starts_with_capture_off` (app.rs) and `mouse_status_text_names_the_toggle_and_the_selection_modifier` (mouse.rs) | Addition |
| E-8 | The tmux run used a private socket (`-L dyf`) and `bash --norc`, so the user's tmux server was never touched. At launch, the startup session scan auto-registered the real `gsd-meta-manager` and `ttbook` repos into the scratch config only; `~/.config` was not touched. The editor step used `e` directly because STATE.md was already selected, so no `j` was needed | Environment note |
| E-9 | The first baseline run overlapped with in-flight Task 1 edits, and `tests/registry_test.rs` rebuilt the binary mid-edit, so it read 2620/2/15. The plan's stated 2621/1/15 is confirmed by arithmetic: the Task 1 run gave 2636 = 2621 + 15 new tests | Measurement note |
| E-10 | `DetailRegions::click_target` and `wheel_target` are `pub(crate)` methods on `DetailRegions`. `handle_mouse` reads the rects through `regions()`, a clone, so no `RefCell` borrow is held while `handle_key` runs | Implementation detail |

## Deferred items

- **Pre-existing raw-mode leak on early startup errors.** `tui::init()` runs before `App::new(config_path)?` and `FileWatcher::new(..)?` in `main.rs`. If either fails, raw mode and the alternate screen are left on. This predates this task. Mouse capture cannot leak there, because the first enable happens inside `run_tui_loop`, after those `?` (I-14). Fixing it was out of scope.
- The panic hook ordering is proven by construction only. No test injects a real panic into the binary.
- Out of scope per the plan's fences: drag, right and middle click, hover, horizontal scroll, row clicks on Git, Backlog, Queue, Config, Roadmap and Docs, clicking `‹`/`›`, mouse in modals, persisting `M`, and a CLI flag.

## Known Stubs

None.

## Threat Flags

None beyond the plan's threat register:

- **T-dyf-01** (reader filter): mitigated and pinned by `mouse_reader_forwards_only_left_press_and_vertical_wheel`.
- **T-dyf-02** (capture lifecycle): mitigated. See the tmux readings and the panic hook.
- **T-dyf-03** (single-click side effects): mitigated. A single click only selects and focuses, modals keep the default, and text entry is ignored.
- **T-dyf-04** (status title spoofing): mitigated. The title is escaped at the render site, and the escape-guard state and reach test cover it.
- **T-dyf-05** (forged reports) and **T-dyf-SC** (supply chain): accepted. No new crates were added.

## Self-Check: PASSED

- src/ui/mouse.rs: FOUND.
- Commits 5a1717d, 001c78b, e54ab55 and d8740b7: all FOUND in `git log`.
- `git rev-list --count 39a91fb..HEAD` = 4.
- The final full suite gave 2654 / 1 (the expected witness) / 15.
