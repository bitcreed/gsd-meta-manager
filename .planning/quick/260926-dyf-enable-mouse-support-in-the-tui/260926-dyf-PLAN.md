---
quick_id: 260926-dyf
mode: quick
phase: quick-260926-dyf
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/config.rs
  - src/ui/mouse.rs
  - src/ui/mod.rs
  - src/action.rs
  - src/event.rs
  - src/tui.rs
  - src/main.rs
  - src/app.rs
  - src/ui/screens/mod.rs
  - src/ui/screens/normal.rs
  - src/ui/screens/detail.rs
  - src/ui/screens/render_escape_guard.rs
  - src/ui/screens/help.rs
  - README.md
  - docs/GETTING-STARTED.md
  - docs/CONFIGURATION.md
autonomous: true
requirements: [QUICK-260926-dyf]

estimate:
  tokens: 330000
  raw_tokens: 330000
  tasks: 3
  confidence: low

must_haves:
  truths:
    - "With the default config, launching the TUI turns on terminal mouse reporting: inside tmux, `#{mouse_any_flag}#{mouse_sgr_flag}` reads `11`. Quitting with q or Ctrl+C leaves it at `00`. During a $VISUAL hand-off it is `00`, and it is `11` again after the editor returns. A panic runs a hook that turns reporting off before ratatui restores the terminal (D-05)"
    - "`preferences.mouse` in config.json defaults to true whether the key is absent or the file is missing. With `\"mouse\": false` the TUI starts with reporting off. `M` toggles capture live on the dashboard and in the detail view, and shows a short status message on both. The dashboard shows it in the footer; the detail view shows it right-aligned in the tab bar's title row. The toggle is never written back to config.json (D-06)"
    - "On the dashboard, clicking a project row selects it, double-clicking opens its detail view (the Enter path), and the wheel moves the selection, stopping at both ends (D-01, D-02, D-03)"
    - "In the detail view, clicking a top tab does exactly what that tab's digit (or Shift+D) key does. Clicking a sub-tab (Sessions / Agents, Files / Milestones) switches to it. Clicking a row in the Phases phase list, Sessions or Agents selects it and focuses content. Clicking a Waves-pane row moves the pane cursor onto it and focuses the pane (D-01)"
    - "A double-click performs Enter on the row the first click selected, through the unchanged keyboard path. That includes resume on Sessions, both Agents <-> Waves-pane cross-jumps, wave fold/unfold, and phase list -> pane focus (D-02)"
    - "The wheel moves the selection in the region under the pointer, even when that region is not focused. Wheel-up at a content's first row never climbs to the tab bar, and the wheel never switches tabs (D-03)"
    - "Hit-test rects are recorded at render time on both render paths and reset every frame. Clicks and wheel events are mapped in the event handler through pure functions over those rects, and no render path reads a file (D-04)"
    - "Mouse motion, drag, button release, non-left buttons and horizontal scroll never enter the biased-first Action FIFO. Only a left press and vertical wheel events do (T-dyf-01)"
    - "The help overlay, README, GETTING-STARTED and CONFIGURATION.md document click, double-click, wheel, the `M` toggle and the Shift+drag text-selection note (D-07)"
    - "Pure tests pin click->target mapping, wheel routing, double-click detection, the reader filter, list-row mapping through the scroll offset, tab-rect layout against the real Tabs widget, and the config default (D-08)"
  artifacts:
    - path: src/ui/mouse.rs
      provides: "MouseInput {Click{column,row,double}, Wheel{column,row,down}}; routed(MouseEvent) reader filter; ClickTracker (500 ms, same row, +/-1 col); ListRegion {rect, offset, len} + row_at; tab_entry_rects (mirrors ratatui Tabs layout); mouse_status_text(on)"
    - path: src/tui.rs
      provides: "set_mouse_capture(on) / mouse_captured(); init() with a once-installed panic hook that disables capture; restore() that disables capture before ratatui::restore"
    - path: src/config.rs
      provides: "Preferences.mouse: bool, #[serde(default = \"default_mouse\")] -> true, Preferences::default() true"
    - path: src/app.rs
      provides: "App.mouse_capture (desired state), App.click_tracker, Action::Mouse -> Screen::handle_mouse, ScreenAction::ToggleMouseCapture handling + status message"
    - path: src/ui/screens/normal.rs
      provides: "dashboard table region + persisted table offset; handle_mouse (click/double/wheel); M arm"
    - path: src/ui/screens/detail.rs
      provides: "DetailRegions.tabs / sub_tabs / list; click_target / wheel_target (pure); persisted phase/sessions/agents list offsets; DetailScreen::handle_mouse; M arm; status title in the tab bar"
  key_links:
    - from: "src/event.rs reader (EventStream)"
      to: "App::update(Action::Mouse)"
      via: "ui::mouse::routed() keeps only Down(Left)/ScrollUp/ScrollDown; everything else is dropped before the FIFO"
    - from: "App::handle_mouse (ClickTracker)"
      to: "Screen::handle_mouse(MouseInput) on the top screen"
      via: "trait default returns ScreenAction::None, so modal/overlay screens ignore the mouse"
    - from: "render (both detail render paths, dashboard render_main)"
      to: "handle_mouse hit tests"
      via: "DetailRegions / the dashboard table region, reset and refilled every frame; tab_entry_rects is pinned against the real Tabs widget buffer"
    - from: "App.mouse_capture (desired)"
      to: "terminal mouse reporting (applied)"
      via: "run_tui_loop syncs through tui::set_mouse_capture each iteration; tui::restore, the panic hook and the editor suspend always disable"
---

# Quick 260926-dyf: mouse support in the TUI

<objective>
Add mouse support to the TUI. Clicking selects tabs, sub-tabs and rows, and focuses the pane it lands in. A double-click performs Enter on the clicked row. The wheel moves the selection in the pane under the pointer. Mouse capture is on by default (`preferences.mouse`), `M` toggles it live, and the terminal is always left with mouse reporting off, including on panic and while $EDITOR has the terminal.

Purpose: 4a (quick 260926-1t1) and 4b (quick 260926-2l4) left the hooks for this: `DetailRegions` / `regions()`, the tab-bar focus level, `focus_block`, and `waves_pane` / `waves_rows` with row identities. This task consumes them.

Output:
- a new pure module, `src/ui/mouse.rs`;
- the capture lifecycle in `src/tui.rs` and `src/main.rs`;
- `Action::Mouse`, `Screen::handle_mouse` and `ScreenAction::ToggleMouseCapture`;
- dashboard and detail-view hit tests;
- help, README, GETTING-STARTED and CONFIGURATION updates.

**Decision IDs.** D-01..D-08 are the locked scope items from the planning context, in order:

- D-01: a click on a top tab switches to it, and a click on a sub-tab switches to it. A click on a list row (dashboard project rows, session rows, agent rows, Phases Waves-pane rows) selects it and focuses that pane.
- D-02: a double-click acts like Enter on the row: the same action as the keyboard path, existing cross-jumps included.
- D-03: the wheel scrolls or moves the selection in the pane under the pointer, which is not necessarily the focused pane.
- D-04: reuse `DetailRegions` / `regions()` and the tab-bar focus model. Rects are recorded at render time and clicks are mapped in the event handler. No file I/O in render.
- D-05: enable crossterm mouse capture on start, and ALWAYS disable it on exit, on the panic/error path and on suspend paths. Suspend means disable before handing off the terminal, and re-enable on return only if it was enabled.
- D-06: the config defaults to mouse on, plus a runtime toggle key (`M` if free) that toggles capture live and shows a short status message.
- D-07: the help overlay mentions the mouse controls, the toggle key, and that Shift+drag selects/copies text while capture is on (terminal-dependent). README / GETTING-STARTED key tables are updated the way 1t1 and 2l4 did.
- D-08: tests for click->target mapping, wheel routing (pure functions over rects), and the config default.

Each task cites the IDs it implements.

**Inferred decisions.** The human is unavailable. Record every one of these in the SUMMARY under "Inferred decisions" for audit, with any execution-time additions numbered E-1, E-2, ...

- **[inferred I-1] The config key.** The user config is `config.json`, not TOML (docs/CONFIGURATION.md already records that the README's "config.toml" is wrong). The key is `preferences.mouse`, a JSON bool. It follows `driver_max_concurrent`'s free-function-default pattern exactly: `#[serde(default = "default_mouse")]`, and `Preferences::default()` sets true. It is serialised on every save, so the next save writes `"mouse": true`.
- **[inferred I-2] The toggle key is `M` (Shift+m).**
  - Measured free: there is no `KeyCode::Char('M')` anywhere in src/. The only uppercase bindings are `D`, `G`, `J` and `K`. `m` (the sub-tab alias) is a different key.
  - It is bound on the dashboard (outside the `/` search sub-mode) and in the detail view. In the detail view it sits AFTER the Config text-edit and filter intercepts and BEFORE the tab-bar and Waves-pane blocks, so it is focus-neutral at all three levels, like `?`.
  - It is not bound on modal, overlay or text-entry screens, where `M` is text.
  - It is runtime only and is never written to config.json.
- **[inferred I-3] The status text.** On: `Mouse on (M toggles) — Shift+drag selects text`. Off: `Mouse off (M toggles)`. One pure `mouse_status_text(on)` owns both strings.
- **[inferred I-4] Status visibility in the detail view.** Measured: `DetailScreen::render` never draws `ctx.status_message`; only the dashboard footer does. Without a change, `M` would say nothing in the detail view.
  - The detail view therefore draws a live status message right-aligned in the tab bar block's top border row, on both render paths through one helper.
  - It is escaped at the render site with `crate::text::render_for_terminal`, the dashboard's WR-03 rule, cut with `fit_cells` to the cells left of ` Project: {alias} ` plus a 1-cell gap, and omitted when fewer than 8 cells remain.
  - The footer hints are unchanged. They are not replaced, because frequent `Updated: {alias}` messages would otherwise hide the navigation hints.
  - Side effect, which is intended: every existing detail-view status message (`No agent is attributed to this plan`, `Editor closed: …`) becomes visible there.
- **[inferred I-5] Capture and the reader filter.** Capture uses crossterm's standard `EnableMouseCapture` / `DisableMouseCapture`; no hand-rolled sequences.
  - EnableMouseCapture also turns on any-motion tracking (1003). So the reader forwards ONLY `Down(Left)`, `ScrollUp` and `ScrollDown`.
  - `Moved`, `Drag`, `Up`, the other buttons and `ScrollLeft`/`ScrollRight` are dropped before the Action FIFO. `pump`'s doc (src/main_loop.rs ~140-150) makes a continuous producer on the biased-first arm a correctness bug, not a style issue.
  - Key modifiers on mouse events are ignored.
- **[inferred I-6] Double-click detection.**
  - A second `Down(Left)` counts as a double when it arrives within 500 ms, on the same row, and within 1 column of the first.
  - A third click starts over.
  - Any key press resets the tracker.
  - Detection lives in `App` (`ClickTracker`) and reaches screens as `MouseInput::Click { double }`.
- **[inferred I-7] A double-click performs Enter on the selection the FIRST click made, and never re-hit-tests.**
  - The screen keeps an armed flag. A single click that selected a row arms it; any other mouse input disarms it.
  - On a double, if armed, the screen runs `handle_key(Enter)`; otherwise the double is treated as a single click.
  - Reason: the first click can re-lay-out the frame. Focusing the Waves pane below 100 cols widens it to full width, and a list can scroll. So the cell under the second click may be a different row.
  - A double on a tab or sub-tab is a single click.
- **[inferred I-8] What each click does (detail view).** Priority is the order below.
  - Tab: exactly its digit or Shift+D arm: focus Content, then `switch_to_tab` (sub-tab memory included). The `‹`/`›` overflow markers are inert.
  - Sub-tab: focus Content, then `switch_to_sub_view` when it differs from the current one.
  - Waves-pane row: focus Pane, and `waves_cursor` = the row's target.
  - List row: focus Content and select:
    - Phases list: `pipeline_selected`, `waves_cursor = None`, `share_pipeline_selection`;
    - Sessions: `sessions_selected`;
    - Agents: `agents_selected`, as a list-line index.
  - Elsewhere inside the Waves pane: focus Pane.
  - Elsewhere in content: focus Content.
  - Tab bar outside a tab, the footer, or outside the frame: nothing.
  - A single click never runs Enter, Space or any Queue/driver action.
- **[inferred I-9] Which rows are clickable.**
  - Row clicks exist for dashboard rows, the Phases phase list, Waves-pane rows, Sessions rows and Agents rows.
  - The Phases phase list goes beyond the literal scope list. It is the other half of the Waves pane, and clicking it is how a mouse user returns focus from the pane.
  - Git, Backlog, Queue, Config, Roadmap and Docs rows get the wheel and click-to-focus only (scope fence).
- **[inferred I-10] Wheel routing (detail view).**
  - One wheel event is one Down/Up step. Terminals already send several events per notch.
  - Over the Waves pane: focus Pane, then the pane's Down/Up.
  - Over other content: focus Content, then that content's Down/Up. A wheel-up where `content_at_first_row` holds is a no-op; it never climbs to the tab bar.
  - Over the tab bar, the sub-tab strip or the footer: ignored. The wheel never switches tabs.
  - The wheel moves focus to the pane it scrolls. The Waves pane draws and follows its cursor only while focused, and the tab-bar level dims content, so scrolling an unfocused region would be invisible.
- **[inferred I-11] Dashboard wheel.** It clamps at both ends; keyboard j/k wrap, but a wheel flick past the end would cycle to the top. It acts anywhere over the table's block and is ignored over the footer. The header row is inert to clicks.
- **[inferred I-12] Persisted list offsets.**
  - The dashboard table, the Phases phase list, the Sessions list and the Agents list keep their scroll offset across frames in a `Cell<usize>`, the `roadmap_list_offset` / `waves_offset` pattern. Each frame seeds the widget state with it and stores the widget's clamped offset back.
  - Today a fresh state per frame means clicking a visible row of a scrolled list moves that row to the bottom.
  - Keyboard behaviour changes only toward standard scrolling: moving up no longer drags the list.
- **[inferred I-13] Mouse input is ignored while text is being typed.**
  - The typing states are: dashboard `/` search, a Config String edit, and Config `/` filter typing. This extends T-1t1-02's guarantee to the mouse.
  - Modal and overlay screens keep the trait default and ignore the mouse, so a click can never confirm a dialog. These are help, delete/driver confirmations, add/create project, enqueue and driver inject/start.
- **[inferred I-14] Capture lifecycle.**
  - The binary's main loop applies App's desired `mouse_capture`, which is initialised from `preferences.mouse`.
  - Capture is first enabled inside `run_tui_loop`, after every fallible startup `?` in main.rs (App::new, FileWatcher::new). An early startup error therefore cannot leave capture on.
  - `tui::restore()` always disables capture when it is on.
  - A once-installed panic hook disables capture, then chains to ratatui's restore hook and color_eyre.
  - The editor suspend records the state, restores (which disables), re-inits, and re-enables only if capture had been on.
  - If enabling fails, the desired state falls back to off and a status message says so.
  - The editor is the only terminal hand-off (measured: `pending_editor`). Project hooks use null stdio, and session resume/switch open other windows or tmux clients.
- **[inferred I-15] No footer hint changes.** Footers are width-budgeted. The mouse is documented in the help overlay, README, GETTING-STARTED and CONFIGURATION.md.

**Scope fences.** Do NOT build:

- drag or drag-select inside the TUI;
- right- or middle-click;
- hover effects;
- horizontal scroll;
- click targets on Git/Backlog/Queue/Config/Roadmap/Docs rows;
- clicking `‹`/`›` to page tabs;
- mouse in modal or overlay screens;
- persisting the `M` toggle;
- a CLI flag;
- any footer hint change;
- any fix to the pre-existing raw-mode leak when `App::new`/`FileWatcher::new` fail after `tui::init()`. Note it under Deferred in the SUMMARY; it predates this task and the mouse lifecycle does not depend on it, per I-14.

**Working rules.**

- **Line numbers drift.** Every line below is a locator to grep near, not a contract.
- **detail.rs size.** detail.rs is about 20.6k lines. Grep, then Read with offset/limit. Never read it whole.
- **Tests.** ALWAYS use `rtk proxy cargo test --no-fail-fast …`. A plain `cargo test` stops at the first failing binary and never reaches the envelope suites. Never pipe cargo output into grep/awk: rtk filters downstream of `rtk proxy` too and truncates the counts.
- **Baseline.** At 39a91fb the baseline is 55 suites, **2621 passed / 1 failed / 15 ignored**. 39a91fb is docs-only on top of f1ba834. Re-measure it before Task 1. The single failure, `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`, is EXPECTED locally; any other failure is yours.
- **Clippy.**
  - `rtk proxy cargo clippy -- -D warnings` must exit 0.
  - `rtk proxy cargo clippy --all-targets` must show exactly the 11 pre-existing findings: browser.rs 3, project_creator.rs 1, tests/envelope_wrapper_class.rs 4, and 1 each in envelope_config_resolution.rs, envelope_control_carrier.rs and envelope_carrier_reach.rs. Nothing new in touched files.
- **Test names.** Every new test name starts with `mouse_` (help.rs and the escape guard included), so `-- mouse_` selects them all.
- **Where you run.** Execute on master, without worktree isolation. Do not push.
</objective>

<execution_context>
@~/.claude/gsd-core/workflows/execute-plan.md
@~/.claude/gsd-core/templates/summary.md
</execution_context>

<context>
@CLAUDE.md
@.planning/STATE.md
@.planning/quick/260926-1t1-4a-detail-view-navigation-model-tab-bar-/260926-1t1-SUMMARY.md
@.planning/quick/260926-2l4-4b-phases-tab-waves-pane/260926-2l4-SUMMARY.md

Interfaces as they are today (grep to confirm):

- `src/event.rs` (binary): `EventBus::spawn_crossterm_reader` reads an `EventStream`. `map_event_to_action` (~47-59) maps Key(Press) → `Action::RawKey` and Resize → `Action::Resize`, and drops everything else.
- `src/tui.rs` (binary, 9 lines): `init()` = `ratatui::init()` (ratatui 0.30.2's init installs its own restore panic hook, init.rs ~566); `restore()` = `ratatui::restore()`.
- `src/main.rs`:
  - `color_eyre::install()` ~78; `tui::init()` ~522;
  - `App::new(config_path)?` ~523; `FileWatcher::new(..)?` ~550;
  - `spawn_crossterm_reader` ~600; `run_tui_loop` call ~609, then `tui::restore()` ~611, then `result?`.
  - `run_tui_loop` ~631-710 draws when `needs_redraw`, pumps, then runs the editor branch ~659-702 (`ratatui::restore()` ~661, `ratatui::init()` ~700).
- `src/action.rs`: `enum Action` ~49 (`Tick`, `RawKey(KeyEvent)`, `Resize`, …). It derives Debug and Clone.
- `src/app.rs`:
  - `App` struct ~494-504 and `from_config` ~576-630.
  - `update` handles `Action::RawKey` ~1260, which calls `handle_key(code, modifiers)` ~2343. Ctrl+C is checked first, then the top screen's `handle_key`.
  - `process_screen_action` ~2356-2395. `App::new_for_test` ~557. Tests: `press` helper ~4953.
- `src/config.rs`:
  - `Preferences` ~313-345 (no derived Default); `impl Default for Preferences` ~364-373.
  - `default_driver_max_concurrent` ~299.
  - The model test is `driver_max_concurrent_defaults_to_one_from_default_and_from_empty_json` ~869; `load_config` / `save_config` ~436-464.
- `src/ui/mod.rs`: `pub mod` list ~1-4. `render(frame, app)` ~9 renders ONLY the top screen.
- `src/ui/screens/mod.rs`: `trait Screen: RenderAdjudicated` ~298-307 (`handle_key`, `render`, `name`); `enum ScreenAction` ~309 (None, Push, Replace, Pop, Quit, SetStatusMessage, SuspendAndEdit, DispatchAction); `AppContext` ~1294.
- `src/ui/screens/normal.rs`:
  - `NormalScreen { pub searching: bool }` ~18-32; `handle_key` ~476-693 (the collision-check comments list claimed keys ~526-536, ~579-586, ~629-633; Enter ~612 pushes `DetailScreen::new(alias)`); `handle_search_key` ~719.
  - `render_main` ~765-993: the table is `dashboard_table(rows, width)` (header row, bottom_margin 0) rendered into `inner` with a COPY of `ctx.table_state` ~986-991.
  - `render_footer` ~995 (status branch ~1002); `move_selection_down/up` ~1196-1214 (wrap around).
  - Tests: `ctx_with_aliases` ~1276, `press` ~1369.
  - The escape guard constructs `NormalScreen { searching: true }` as a literal (render_escape_guard.rs ~3329); change it to struct-update syntax when fields are added.
- `src/ui/screens/detail.rs`:
  - Tab bar:
    - `TAB_OVERFLOW_LEFT`/`RIGHT` ~555-561 and `tab_entry_cells` ~564 (label + 2);
    - `tab_titles(width, active, driver_live, experimental) -> (Vec<Line>, select)` ~595;
    - `tab_bar_widget` ~634 frames each title with `[`/`]` or spaces, padding "" and divider "|".
  - `DetailScreen` fields ~766-827: the `Cell` viewports, `roadmap_list_offset`, `waves_offset`, `focus`, and `regions: RefCell<DetailRegions>`.
  - `DetailRegions` ~839-856 (`tab_bar`, `sub_tab_strip`, `content`, `pane`, `waves_pane`, `waves_rows`); `WavesRowRegion { rect, target: WavesCursor }` ~861; `DetailFocus { TabBar, Content, Pane }` ~882.
  - `new` ~906. `regions()` ~931 carries a `cfg_attr` dead-code allow because only tests read it today.
  - Region helpers: `reset_regions` ~937, `record_sub_tab_strip` / `record_pane` / `record_waves` ~972-990, `sub_tab_row(frame, area, strip)` ~995 (4 callers: sessions, agents, browser, archive).
  - `handle_waves_pane_key` ~1511 (j/k/Down/Up = `waves_move`; Enter/Space = `waves_activate`); `content_at_first_row` ~1569.
  - Navigation helpers: `share_pipeline_selection` ~2345, `switch_to_tab` ~2368, `switch_to_sub_view` ~2388.
  - `handle_key` ~2615:
    - Config text-edit intercept ~2651-2665 and filter-typing intercept ~2670-2676;
    - tab-bar block ~2683-2722; pane block ~2728-2734;
    - main match: `j`/Down arm ~2874; digit arm ~3527; `D` arm ~3542.
  - Render:
    - `render` ~4831-4929: the tab bar ~4864-4874 (a `Block` with BOTTOM border and title ` Project: {alias} `), content dispatch, footer choice ~4914-4928;
    - `render_pipeline_tab` ~5590-5724 (phase `List` with a `Borders::RIGHT` block titled ` Phases ` and a fresh `ListState` ~5647-5669; narrow focused-pane breadcrumb path ~5618-5636); `render_waves_pane` ~5732;
    - `render_sessions_tab` ~5878-5970 and `render_agents_tab` ~5993-6059 each build a fresh `ListState` per frame;
    - `render_main_only` ~6479-6540 duplicates the tab bar.
  - Strip and pane helpers: `two_sub_tab_strip` ~8143 (spans: gutter, left label, ` │ `, right label, hint), `sub_tab_pair` ~8177, `focus_block` ~8201, `fit_cells`, `agent_list_len` ~8409, `agents_list_max` ~8422.
  - Tests: `test_ctx` ~13786, `press` ~13893, `render_detail_sized` ~14074, `render_detail_to_text_at` ~15015, `two_sessions_fixture` ~19117, `arrived_on` ~19424, `render_records_the_regions_a_mouse_hit_test_needs` ~19626, and the `waves_pane_*` / `cross_jump_*` fixtures.
- `src/ui/screens/render_escape_guard.rs`: `STATUS_BRANCH_TOKEN` ~1333; `status_message_like_app_builds_it` ~1344; `DETAIL_TAB_ARRIVAL` ~1506; `DETAIL_SUB_STATES` ~1821 (arrange fns take `(&str, &mut AppContext)`); `the_status_footer_state_reaches_the_status_branch` ~3307.
- `src/ui/screens/help.rs`: `row(key, description)` ~181 (key padded to 13); `help_lines(experimental)` ~200, whose Waves-pane block is ~271-284. Every description must be unique in the body.
- ratatui 0.30.2 facts (verified in ~/.cargo/registry): `Tabs::render_tabs` lays each title at x, then the divider (1 cell) between titles, clipped at the area's right edge, in the first row of the block's inner area. `ListState::with_offset` / `offset()` and `TableState::with_offset` / `offset()` exist, and `TableState` is Copy. `Rect::contains(Position)` exists.
- crossterm 0.29: `MouseEvent { kind, column, row, modifiers }`; `MouseEventKind::{Down(b), Up(b), Drag(b), Moved, ScrollDown, ScrollUp, ScrollLeft, ScrollRight}`; `EnableMouseCapture` / `DisableMouseCapture` commands.
</context>

<tasks>

<task type="tracer" tdd="true">
  <name>Task 1 (tracer): config → terminal capture → reader filter → Action::Mouse → App → dashboard click / double-click / wheel, plus the M toggle and the full capture lifecycle</name>
  <files>src/config.rs, src/ui/mouse.rs, src/ui/mod.rs, src/action.rs, src/event.rs, src/tui.rs, src/main.rs, src/app.rs, src/ui/screens/mod.rs, src/ui/screens/normal.rs, src/ui/screens/render_escape_guard.rs</files>
  <read_first>src/event.rs, src/tui.rs, src/main.rs (~70-90, ~515-710), src/action.rs (~48-60), src/app.rs (~494-630, ~1255-1266, ~2343-2396), src/config.rs (~288-375, ~860-890), src/ui/screens/mod.rs (~292-345), src/ui/screens/normal.rs (~18-32, ~400-450, ~476-763, ~765-1000, ~1196-1214, ~1276-1380), src/main_loop.rs (~130-150), src/ui/screens/render_escape_guard.rs (~3326-3335)</read_first>
  <behavior>
    - mouse_preference_defaults_to_true_from_default_and_from_empty_json (config.rs): `Preferences::default().mouse` is true. A config whose `preferences` is `{}` loads with mouse true. `"preferences": {"mouse": false}` loads false. `save_config` then `load_config` keeps false.
    - mouse_reader_forwards_only_left_press_and_vertical_wheel (ui/mouse.rs): `routed` maps Down(Left) to a click, and ScrollDown/ScrollUp to a wheel carrying down=true/false with the event's column/row. Moved, Drag(Left), Up(Left), Down(Right), Down(Middle), ScrollLeft and ScrollRight give None.
    - mouse_double_click_needs_the_same_cell_within_the_window (ui/mouse.rs), with `ClickTracker::register(now, column, row) -> bool` and Instants built from one base:
      - (5,3) at t0 is single; (5,3) at t0+200ms is double; a third (5,3) at t0+300ms is single again;
      - (6,3) inside the window after a single is double (±1 column); (5,4) is single;
      - a second click after 600 ms is single; `reset()` makes the next click single.
    - mouse_list_region_maps_rows_through_the_offset (ui/mouse.rs): with `ListRegion { rect: (2,4,30,5), offset: 10, len: 12 }`, (3,4) → Some(10) and (3,5) → Some(11). (3,6) → None (beyond len), (40,4) → None (outside columns) and (3,3) → None (outside rows).
    - mouse_dashboard_click_selects_the_row_under_the_pointer (normal.rs): 5 projects rendered at 100x30. A click at the centre of the third recorded body row sets `ctx.table_state.selected()` = Some(2) and returns ScreenAction::None. A click on the header row or the footer changes nothing.
    - mouse_dashboard_double_click_opens_the_first_clicks_row (normal.rs): a click on row 2, then `Click{double: true}` at the same cell, returns `ScreenAction::Push` of a screen whose `name()` is `DetailScreen::NAME`. A double whose first click hit the header behaves as a single click and pushes nothing.
    - mouse_dashboard_wheel_moves_and_clamps (normal.rs): a wheel down over the table moves the selection +1. At the last row it stays there (no wrap); at row 0, wheel up stays at 0. A wheel over the footer changes nothing.
    - mouse_dashboard_keeps_its_scroll_offset (normal.rs): 40 projects at 100x20 with row 30 selected. After a render the recorded offset is > 0. A click on the top visible body row selects that index, and after a re-render the recorded offset is unchanged, so the clicked project is still under the pointer.
    - mouse_dashboard_ignores_the_mouse_while_searching (normal.rs): with `searching` true, a click on a row and a wheel both leave the selection unchanged.
    - mouse_toggle_key_flips_capture_and_says_so (app.rs):
      - `App::new_for_test()` starts with `mouse_capture` true.
      - Pressing `M` gives false and a status message containing `Mouse off`. Pressing it again gives true and a status containing `Shift+drag`.
      - After `/`, typing `M` goes into the filter text and `mouse_capture` is unchanged.
    - mouse_events_are_ignored_while_capture_is_off (app.rs): with `mouse_capture` false, `update(Action::Mouse(<left press on a drawn row>))` leaves the selection unchanged. The App test draws first through a `ratatui::Terminal<TestBackend>` and `crate::ui::render`.
    - mouse_app_turns_a_quick_second_press_into_enter (app.rs): after a TestBackend draw of 3 projects, two immediate `update(Action::Mouse(Down(Left)))` at the third row push the detail view: `screen_stack.len()` becomes 2. A key pressed between the two presses prevents it.
  </behavior>
  <action>
**Config (per D-06, [inferred I-1]), `src/config.rs`.**

1. Add `fn default_mouse() -> bool` returning true, with a one-line doc giving the reason.
2. Add `pub mouse: bool` to `Preferences` with `#[serde(default = "default_mouse")]`, placed before the flattened `extra`. Its doc says it is the startup default for terminal mouse capture, that `M` toggles it at runtime without saving, and that terminal text selection needs Shift+drag while capture is on.
3. Set `mouse: default_mouse()` in `impl Default for Preferences`.
4. Add the config test next to `driver_max_concurrent_defaults_to_one_from_default_and_from_empty_json`.

**Pure mouse module (per D-01..D-04, D-08, [inferred I-5], [inferred I-6]), new `src/ui/mouse.rs`, declared `pub mod mouse;` in `src/ui/mod.rs`.**

1. `pub enum MouseInput { Click { column: u16, row: u16, double: bool }, Wheel { column: u16, row: u16, down: bool } }`, deriving Debug/Clone/Copy/PartialEq/Eq.
2. `pub enum Routed { Click { column, row }, Wheel { column, row, down } }` and `pub fn routed(event: &MouseEvent) -> Option<Routed>`, following [inferred I-5].
3. `pub const DOUBLE_CLICK_WINDOW: Duration` (500 ms), and `pub struct ClickTracker` (Default) with `register(&mut self, now: Instant, column: u16, row: u16) -> bool` and `reset(&mut self)`, following [inferred I-6]. After a double it forgets the click, so a third click is single.
4. `pub struct ListRegion { pub rect: Rect, pub offset: usize, pub len: usize }` (Debug/Clone/Copy/PartialEq/Eq) with `row_at(&self, column, row) -> Option<usize>`: `rect.contains(Position)`, then `offset + (row - rect.y)`, and `None` when that is `>= len`.
5. `pub fn tab_entry_rects(area: Rect, entry_widths: &[u16], divider_cells: u16) -> Vec<Rect>`, used in Task 2. It mirrors ratatui 0.30's `Tabs::render_tabs`: x starts at area.x; entry i is `(x, area.y, w, 1)` clipped at `area.right()`; x advances by w and then by the divider between entries (not after the last). An entry starting at or past the right edge gets width 0. Unit-test it in `mouse_tab_entry_rects_mirror_the_tabs_layout` on plain widths, including clipping.
6. `pub fn mouse_status_text(on: bool) -> &'static str`, per [inferred I-3].
7. A module doc stating that these are pure functions over rects recorded at render time (D-04). No I/O and no terminal access.

**Action and reader (per D-04, [inferred I-5]).**

1. Add `Action::Mouse(crossterm::event::MouseEvent)` in `src/action.rs`, with a doc naming the reader filter.
2. In `src/event.rs` `map_event_to_action`, map `Event::Mouse(m)` to `Action::Mouse(m)` ONLY when `gsd_meta_manager::ui::mouse::routed(&m)` is Some. Add a comment citing `pump`'s biased-first rule: this is why motion never enters the FIFO.

**Screen seam (per D-01, D-02, [inferred I-13]), `src/ui/screens/mod.rs`.**

1. Add to `trait Screen` a DEFAULT method `handle_mouse(&mut self, input: MouseInput, ctx: &mut AppContext) -> ScreenAction` that returns `ScreenAction::None`. Its doc says that modal, overlay and text-entry screens rely on this default, so a click can never confirm a dialog.
2. Add `ScreenAction::ToggleMouseCapture`, with a doc saying App owns the desired state and the binary applies it. Fix any exhaustive `match` over ScreenAction that the compiler names.

**App (per D-02, D-06, [inferred I-2], [inferred I-6]), `src/app.rs`.**

1. Add `pub mouse_capture: bool` to `App`, the DESIRED state, initialised in `from_config` from `config.preferences.mouse` (read before `config` moves into ctx). Add `click_tracker: crate::ui::mouse::ClickTracker`.
2. Add `pub fn handle_mouse(&mut self, event: MouseEvent, now: Instant)`:
   - return early when `!self.mouse_capture`;
   - `routed()`; a Click goes through `click_tracker.register` to fill `double`;
   - pass the resulting `MouseInput` to the top screen's `handle_mouse`, then `process_screen_action`;
   - set `needs_redraw`.
3. In `update`, `Action::Mouse(ev)` calls `self.handle_mouse(ev, Instant::now())`.
4. In `handle_key`, reset the click tracker at the top, so a key between two presses prevents a double.
5. In `process_screen_action`, handle `ToggleMouseCapture`: flip `mouse_capture`, set `ctx.status_message` to `mouse_status_text(self.mouse_capture)` with `Instant::now()`, and set `needs_redraw`. Nothing is written to config.json.

**Dashboard (per D-01, D-02, D-03, [inferred I-7], [inferred I-11], [inferred I-12], [inferred I-13]), `src/ui/screens/normal.rs`.**

1. Add these fields to `NormalScreen`, and initialise them in `new()`:
   - `table_offset: Cell<usize>`;
   - `table_region: Cell<Option<ListRegion>>` (the body rows);
   - `table_area: Cell<Option<Rect>>` (the outer block area, used for the wheel);
   - `mouse_row_armed: bool`.
2. Fix every `NormalScreen { .. }` literal the compiler names. The escape guard's `NormalScreen { searching: true }` becomes struct-update syntax over `NormalScreen::new()`.
3. In `render`, reset both region cells to None at the top, so the too-small branch records nothing.
4. In `render_main`'s table branch:
   - seed the copied state with `ctx.table_state.with_offset(self.table_offset.get())`;
   - after `render_stateful_widget`, store `table_state.offset()` back;
   - record `ListRegion { rect: inner minus its first (header) row, offset: table_state.offset(), len: ctx.filtered_aliases.len() }` and `table_area = area`.
   - The empty state records nothing.
5. Implement `handle_mouse`:
   - ignore everything while `searching`.
   - Click (single): if `table_region.row_at` gives Some(i), select i in `ctx.table_state`, arm, and set `needs_redraw`; otherwise disarm.
   - Click with double: if armed, disarm and return the `Enter` arm's action by calling `self.handle_key(KeyCode::Enter, KeyModifiers::NONE, ctx)`; if not armed, treat it as single.
   - Wheel: when inside `table_area`, step the selection +1/-1 CLAMPED (a small helper beside `move_selection_down/up`; the keyboard helpers keep wrapping), and disarm.
6. Add `KeyCode::Char('M') => ScreenAction::ToggleMouseCapture` in the non-search match. Append `M` to the collision-check comment lists near `s` and `b`.

**Terminal lifecycle (per D-05, [inferred I-14]), `src/tui.rs` and `src/main.rs`.**

1. In `src/tui.rs`, add a private `static MOUSE_CAPTURED: AtomicBool` and these functions:
   - `pub fn set_mouse_capture(on: bool) -> std::io::Result<()>`: `execute!` EnableMouseCapture or DisableMouseCapture on stdout, and record the flag only on success;
   - `pub fn mouse_captured() -> bool`;
   - `init()`: `ratatui::init()`, then install ONCE (`std::sync::Once`) a panic hook that takes the previous hook and, when the flag is set, writes DisableMouseCapture (errors ignored) and clears the flag before calling the previous hook. Installed after ratatui's, it runs first. Doc comment: ratatui::init reinstalls its own restore hook on every call; the Once keeps this one single;
   - `restore()`: when captured, disable (errors ignored) and clear the flag, then `ratatui::restore()`.
2. In `src/main.rs` `run_tui_loop`, at the top of each iteration before drawing: when `app.mouse_capture != tui::mouse_captured()`, call `tui::set_mouse_capture(app.mouse_capture)`. On Err, set `app.mouse_capture = tui::mouse_captured()`, a status message `Mouse capture unavailable: {e}`, and `needs_redraw`. This is also the FIRST enable of the session. It runs after every fallible startup `?` in the TUI arm, and every exit path of `run_tui_loop` is followed by `tui::restore()`.
3. In the editor branch:
   - replace `ratatui::restore()` with `let mouse_was_on = tui::mouse_captured(); tui::restore();`;
   - replace `ratatui::init()` with `tui::init()`;
   - immediately after, when `mouse_was_on`, call `tui::set_mouse_capture(true)`, ignoring the error; the loop's sync covers it;
   - add a comment naming D-05: disable before hand-off, re-enable on return only if it was on.
4. Leave `color_eyre::install()` where it is.

**End-to-end check (the tracer's proof).** Run it through `bash -c` (the login shell is fish), and paste the observed values into the SUMMARY.

1. Setup:
   - `cargo build`, then make a `mktemp -d` scratch dir;
   - create three GSD dirs, each with `.planning/STATE.md` (a one-line file is enough), and register them with `target/debug/gsd-meta-manager --config <scratch>/config.json add <dir> <alias>`;
   - write an executable probe script. It runs `tmux display -p -t dyf '#{mouse_any_flag}#{mouse_sgr_flag}'` into `<scratch>/in-editor`, then exits 0.
2. Launch and check the flags:
   - `tmux new-session -d -s dyf -x 100 -y 30` (a shell), then `tmux send-keys -t dyf "VISUAL=<probe> <binary> --config <scratch>/config.json" Enter`, then sleep 2;
   - `tmux display -p -t dyf '#{mouse_any_flag}#{mouse_sgr_flag}'` must print `11`;
   - send `M`: it prints `00` and the footer shows `Mouse off`; send `M` again: `11`.
3. Inject clicks. Use `tmux send-keys -t dyf -l` with an SGR report built by printf: ESC `[<0;` column `;` row `M`, 1-based.
   - A press at column 5 on the third project's row selects it; `tmux capture-pane -p -t dyf` shows `> ` on that row.
   - Two presses sent back to back open its detail view, and the capture shows ` Project: `.
4. Editor hand-off: reach any `e` path that returns SuspendAndEdit, for example `8` for Docs › Files, `j` onto STATE.md, then `e`. `<scratch>/in-editor` must read `00`, and after the probe exits the flags read `11`.
5. Quit: `q` back to the dashboard, then `q` to quit. At the shell prompt the flags read `00`.
6. Mouse off by default: edit the scratch config to `"mouse": false` under preferences and relaunch. The flags read `00`; `M` gives `11`; quit gives `00`.
7. Clean up: kill the tmux session.

If SGR injection through send-keys does not reach the app, record that as E-n and rely on the unit/App tests for steps 3; steps 2 and 4-6 must still be measured.

Commit: `feat(quick-260926-dyf): mouse capture lifecycle, preferences.mouse, M toggle and dashboard click, double-click and wheel`.
  </action>
  <verify>
    <automated>rtk proxy cargo test --no-fail-fast -- mouse_ render_escape_guard</automated>
  </verify>
  <done>
- All behaviours above pass.
- The tmux end-to-end values (launch 11, M 00/11, in-editor 00, after the editor 11, after quit 00, mouse:false launch 00) are recorded in the SUMMARY.
- The SUMMARY cites the `run_tui_loop` lines where the editor hand-off calls `tui::restore()` and `tui::init()`, and the line that re-enables capture only when `mouse_was_on`.
- `rtk proxy cargo clippy -- -D warnings` exits 0.
- The full `rtk proxy cargo test --no-fail-fast` fails only on the expected git-version witness.
- Committed.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: detail-view clicks: tab and sub-tab rects, clickable Phases, Sessions, Agents and Waves-pane rows, click-to-focus, and double-click as Enter</name>
  <files>src/ui/screens/detail.rs, src/ui/mouse.rs</files>
  <read_first>src/ui/screens/detail.rs (tab bar ~555-760, DetailScreen/DetailRegions/DetailFocus/new/regions/record_* ~766-1000, handle_waves_pane_key ~1504-1550, share_pipeline_selection/switch_to_tab/switch_to_sub_view ~2345-2420, handle_key intercepts ~2615-2735, digit and D arms ~3520-3545, render ~4831-4929, render_pipeline_tab ~5590-5724, render_waves_pane ~5732-5817, render_sessions_tab ~5878-5970, render_agents_tab ~5993-6059, render_main_only ~6479-6540, two_sub_tab_strip/sub_tab_pair ~8122-8187, tests: test_ctx ~13786, press ~13893, two_sessions_fixture ~19117, arrived_on ~19424, render_records_the_regions_a_mouse_hit_test_needs ~19626), src/ui/mouse.rs (from Task 1)</read_first>
  <behavior>
    - mouse_tab_rects_match_the_rendered_tab_bar:
      - Setup: for the full tier (120 cols), the compact tier (the smallest width at `tab_bar_compact_cells`) and the windowed tier (40 cols, active tab in the middle), with the experimental flag on and off, render and read `regions().tabs`.
      - Every TabRegion's rect lies in the tab bar's second row. The rendered buffer text inside the rect equals that tab's framed entry, and contains `TAB_LABELS_FULL[tab]` or `TAB_LABELS_COMPACT[tab]`.
      - Overflow markers are never recorded, and the recorded tab indices are exactly the tabs drawn.
    - mouse_click_on_a_tab_is_its_digit:
      - From Sessions content, a click on the Git tab rect gives GitHistory with focus Content; from TabBar focus, the same.
      - After using Agents, leaving, then clicking the Sessions tab, the view lands on Agents, the digit's memory rule.
      - With the flag on, a click on the Driver tab gives Driver.
      - The returned action equals what `press(<digit>)` returns in the same state; compare the resulting view and focus.
    - mouse_click_on_a_sub_tab_switches_it:
      - On Sessions, a click inside the `Agents` label rect gives Agents; a click on `[Agents]` then leaves it unchanged.
      - On Docs › Files, a click on `Milestones` gives Archive.
      - A click on the `←/→ switch` hint changes nothing but focus (Content).
    - mouse_click_selects_a_list_row_and_focuses_content:
      - `two_sessions_fixture` at TabBar focus: a click on the second session row gives sessions_selected 1 and focus Content.
      - Agents with a child line: a click on the child line gives agents_selected = that line index.
      - Phases with 5 phases: a click on the third phase row gives pipeline_selected 2 and `waves_cursor` None. From Pane focus, the same click gives focus Content.
    - mouse_click_on_a_waves_row_moves_the_pane_cursor: on an executing Phases fixture with the list focused, a click on a recorded plan row gives focus Pane and `waves_cursor` = that row's `target`. A click on a header row gives its `Wave(..)` target. A click on the pane's border gives focus Pane with the cursor unchanged.
    - mouse_double_click_is_enter_on_the_first_clicks_row:
      - A header row double-click flips the fold exactly as `press(Enter)` does in the pane.
      - A plan row with an attributed agent jumps to Sessions › Agents with that agent selected; this is the 2l4 cross-jump.
      - A phase-list row double-click focuses the pane.
      - An Agents row jumps to Phases with the pane focused on its plan.
      - On Sessions, the result equals `handle_key(Enter)` from the same state. Use the fixture that returns a status message, so no terminal is spawned.
      - A double whose first click hit a tab or empty content is a single click and does not run Enter.
    - mouse_click_target_is_a_pure_function_of_the_regions: a hand-built DetailRegions with known rects gives:
      - a tab point → Tab(i); a sub-tab point → SubTab(view);
      - a waves row point → WavesRow(target), which wins over the enclosing pane; a waves border point → WavesPane;
      - a list point → ListRow(offset + dy); a point beyond the list len → Content;
      - other content → Content; a tab-bar gap → Nothing; the footer row → Nothing.
    - mouse_regions_reset_every_frame_on_both_render_paths: Sessions records 2 sub_tabs and a list; after switching to Queue and re-rendering, both are empty/None and tabs is non-empty. `render_main_only` records the same tabs as `render`.
    - mouse_clicks_are_ignored_while_typing_in_config: while a Config String value is being edited, and while the `/` filter is being typed, a click on a tab and a click on a row change neither the view, the focus nor the buffer.
    - mouse_lists_keep_their_scroll_offset: 30 phases at 100x20 with phase 25 selected. After a render the list offset is > 0. A click on the top visible phase row selects that index, and a re-render keeps the same offset. The same holds for the Agents list with enough lines to scroll.
  </behavior>
  <action>
**Regions (per D-04, D-01, [inferred I-12]).**

1. Extend `DetailRegions` with:
   - `tabs: Vec<TabRegion>`, where `TabRegion { rect: Rect, tab: usize }`;
   - `sub_tabs: Vec<SubTabRegion>`, where `SubTabRegion { rect: Rect, view: DetailSubView }`;
   - `list: Option<crate::ui::mouse::ListRegion>`: the content's clickable list, meaning the Phases phase list, the Sessions list or the Agents list.
2. `reset_regions` clears all three. Update the struct doc: the mouse hit test is now the live consumer.
3. Remove the dead-code `cfg_attr` on `regions()`, whose doc says only tests read it. `handle_mouse` reads it now; clippy `-D warnings` proves the consumer exists.
4. Add one helper, `record_tab_bar(&self, tab_area, tab_block: &Block, titles: &[Line], select, active)`, and call it from BOTH `render` and `render_main_only`, keeping the file's "one function each" rule for the tab bar.
   - It computes the Tabs inner area from `tab_block.inner(tab_area)`, and entry widths as `tab_entry_cells(line.width())` for each title (the brackets/spaces framing adds 2).
   - Then `crate::ui::mouse::tab_entry_rects(inner, widths, 1)`.
   - Then it maps title i to a tab. A title whose text equals `TAB_OVERFLOW_LEFT` or `TAB_OVERFLOW_RIGHT` is skipped. Every other title maps to `active_clamped + i - select`, where `active_clamped` = `active.min(visible_tab_count(experimental) - 1)`, the clamp `tab_titles` applies.
   - Build the block before handing it to the widget, and pass it by reference.
5. Change `sub_tab_row` to take the view being rendered, and record `sub_tabs` from `sub_tab_pair(view)`: the left label rect starts after span 0's width and is span 1's width; the right label rect starts after spans 0-2 and is span 3's width. Update its 4 callers.
6. Add `phase_list_offset`, `sessions_offset` and `agents_offset` as `Cell<usize>` fields, initialised in `new`. In `render_pipeline_tab`, `render_sessions_tab` and `render_agents_tab`:
   - seed `ListState::default().with_offset(cell.get())` plus the existing selection;
   - after `render_stateful_widget`, store `list_state.offset()` back;
   - record `ListRegion { rect: <the list's inner area>, offset, len }`.
   - For the phase list, the inner area is `Block::inner` of the ` Phases ` block computed before the block moves into the `List`, and len = `state.phases.len()`.
   - Sessions: rect = `inner`, len = the filtered session count.
   - Agents: rect = `list_area`, len = `agent_list_len(view)`.
   - Record nothing on the empty, no-state and too-small branches, or in the narrow focused-pane path where the list is not drawn.
7. Every recorded value is layout arithmetic over rects already computed. No render path gains any I/O (D-04).

**Pure hit test (per D-08, [inferred I-8]).**

1. Add `enum ClickTarget { Tab(usize), SubTab(DetailSubView), WavesRow(WavesCursor), ListRow(usize), WavesPane, Content, Nothing }`.
2. Add `impl DetailRegions { fn click_target(&self, column: u16, row: u16) -> ClickTarget }`, checking in the priority order of [inferred I-8]: tabs, sub_tabs, waves_rows, list rows (`row_at`), waves_pane, content, then Nothing. The tab bar outside a tab and the footer are Nothing.

**`DetailScreen::handle_mouse` for clicks (per D-01, D-02, [inferred I-7], [inferred I-8], [inferred I-13]).**

1. Add a `mouse_row_armed: bool` field.
2. Guard: a text entry is active when `current_view == Defaults` and either a String value is being edited or the filter is being typed. The condition must be the same one `handle_key`'s two intercepts use; extract a small shared predicate so the two cannot drift. When it holds, return ScreenAction::None and change nothing.
3. Click (single), by target:
   - Tab(i): disarm, set focus Content, and return `switch_to_tab(&self.alias, i, &mut self.scroll_offset, ctx)`. This is literally the digit arm.
   - SubTab(v): disarm, set focus Content, and switch through `switch_to_sub_view` when `v` differs from the current view.
   - WavesRow(t): set focus Pane and `cache.waves_cursor = Some(t)`, then arm.
   - ListRow(i): set focus Content and set the view's selection:
     - Pipeline: `pipeline_selected = i`, `waves_cursor = None`, then `share_pipeline_selection` (the j/k arm's triple);
     - Sessions: `sessions_selected = i`;
     - Agents: `agents_selected = i`;
     - then arm.
   - WavesPane: focus Pane, disarm. Content: focus Content, disarm. Nothing: disarm.
   - Set `ctx.needs_redraw` whenever state changed.
4. Click with double: when armed, disarm and return `self.handle_key(KeyCode::Enter, KeyModifiers::NONE, ctx)`. Focus is already the level the first click chose, so Enter takes the pane path on a Waves row and the content path on a list row. When not armed, run the single-click logic.
5. Leave `Wheel` returning ScreenAction::None for now; Task 3 fills it in.

Commit: `feat(quick-260926-dyf): detail view clicks — tabs, sub-tabs, list and Waves-pane rows, double-click as Enter`.
  </action>
  <verify>
    <automated>rtk proxy cargo test --no-fail-fast -- mouse_ waves_pane cross_jump render_escape_guard</automated>
  </verify>
  <done>
- All behaviours above pass.
- The existing `render_records_the_regions_a_mouse_hit_test_needs`, `waves_pane_*` and `cross_jump_*` tests pass. Any expectation that changed because of the persisted list offsets is named in the SUMMARY with old → new.
- `rtk proxy cargo clippy -- -D warnings` exits 0.
- The full `rtk proxy cargo test --no-fail-fast` fails only on the expected witness.
- Committed.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 3: wheel routing in the detail view, M and a visible status title there, then help, README, GETTING-STARTED and CONFIGURATION</name>
  <files>src/ui/screens/detail.rs, src/ui/screens/render_escape_guard.rs, src/ui/screens/help.rs, README.md, docs/GETTING-STARTED.md, docs/CONFIGURATION.md</files>
  <read_first>src/ui/screens/detail.rs (content_at_first_row ~1569, handle_key intercepts ~2648-2735, the handle_mouse added in Task 2, render ~4831-4929, render_main_only ~6479-6515, fit_cells), src/ui/screens/render_escape_guard.rs (~1323-1346, ~1506-1560, ~1821-1860, ~3284-3340), src/ui/screens/help.rs (~174-290 and its whole-row tests), README.md (~52-100, ~179-216), docs/GETTING-STARTED.md (~129-176), docs/CONFIGURATION.md (~28-75)</read_first>
  <behavior>
    - mouse_wheel_target_is_a_pure_function_of_the_regions: with hand-built regions, a point in the waves pane → WavesPane, including a point on a waves row. Other content → Content. The tab bar, the sub-tab strip and the footer → Nothing.
    - mouse_wheel_moves_the_pane_under_the_pointer:
      - Phases, list focused: wheel down over the Waves pane focuses the pane and moves the cursor exactly as one `press(Down)` in the pane would from the entry cursor.
      - With the pane focused: wheel down over the phase list focuses Content, gives pipeline_selected +1 and waves_cursor None.
      - Sessions: wheel down gives sessions_selected +1, clamped at the last row.
      - Roadmap list and Git: one wheel down equals one `press(Down)`; compare the cache fields.
    - mouse_wheel_up_never_climbs_to_the_tab_bar:
      - On Sessions at row 0 in Content, wheel up leaves focus Content and the selection 0. `press(Up)` in the same state would go to TabBar; assert both, to prove the guard matters.
      - From TabBar focus, wheel down over content focuses Content and moves one row.
    - mouse_wheel_never_switches_tabs: wheel events over the tab bar and over the sub-tab strip leave the view and the focus unchanged.
    - mouse_toggle_key_in_the_detail_view_is_focus_neutral:
      - `M` returns ToggleMouseCapture with focus unchanged at TabBar, at Content and at Pane (Phases).
      - During a Config String edit, `M` is appended to the buffer, and during filter typing it is appended to the filter; ToggleMouseCapture is not returned in either case.
    - mouse_status_message_shows_in_the_detail_tab_bar:
      - With `ctx.status_message` = `Mouse off (M toggles)` at 120x30, row 0 contains ` Project: ` and ends (right side) with the message.
      - At a width where fewer than 8 cells remain, the message is absent and ` Project: ` is intact.
      - With no status message, row 0 is unchanged from today.
      - Both render paths draw it.
    - mouse_status_title_state_reaches_the_title (render_escape_guard.rs): a new `DETAIL_SUB_STATES` entry, `Detail view with a status message`, sets `ctx.status_message` from `status_message_like_app_builds_it(identity)`. The rendered detail buffer contains `STATUS_BRANCH_TOKEN`, so the probe reaches the new title, and the guard's hostile-identity assertions pass for that state. It has a matching `DETAIL_TAB_ARRIVAL` row.
    - mouse_help_block_is_documented_as_whole_rows (help.rs): a `Mouse` heading followed by these whole rows, each description unique in the body: `Click`, `Double-click`, `Wheel`, `M` and `Shift+drag`. The Shift+drag row says it is terminal-dependent.
  </behavior>
  <action>
**Wheel (per D-03, D-08, [inferred I-10]).**

1. Add `enum WheelTarget { WavesPane, Content, Nothing }` and `impl DetailRegions { fn wheel_target(&self, column, row) -> WheelTarget }`. Inside `waves_pane` it is WavesPane; otherwise inside `content` but outside `sub_tab_strip` it is Content; otherwise Nothing.
2. In `handle_mouse`, the `Wheel { down }` arm:
   - applies the same text-entry guard, then disarms;
   - WavesPane: set focus Pane, then return `self.handle_key(if down { Down } else { Up }, NONE, ctx)`. The pane intercept handles it, and ↑ at the first pane row stays in the pane.
   - Content: set focus Content. If `!down && self.content_at_first_row(&current_view, ctx)`, set `needs_redraw` and return None, because the wheel never climbs to the tab bar. Otherwise return `self.handle_key(Down or Up, NONE, ctx)`. Use the arrow KeyCodes, never `j`/`k`: Down/Up are the codes every list arm and the Config dropdown/filter arms already accept.
   - Nothing: return None.

**`M` in the detail view (per D-06, [inferred I-2]).** In `handle_key`, directly after the Config filter-typing intercept and before the tab-bar block, add: when `code == KeyCode::Char('M')`, return `ScreenAction::ToggleMouseCapture` without touching focus. Comment: focus-neutral at every level, and after the intercepts so a typed `M` stays text.

**Status title (per D-06, [inferred I-4]).**

1. Add one helper that builds the tab bar `Block`: the existing BOTTOM border and ` Project: {shown(alias)} ` title, plus, when `ctx.status_message` is Some, a second right-aligned top title.
   - The title is `crate::text::render_for_terminal(msg)` cut with `fit_cells` to `tab_area.width - project_title_cells - 3`, and omitted below 8 cells.
   - Use ratatui 0.30's right-aligned `Line` title on the block; confirm the API in the registry source if unsure.
   - Comment at the render site: the escape lives HERE for the reason the dashboard's WR-03 block records (normal.rs `render_footer`). The producer set is not closed, so the message is escaped where it is drawn.
2. Use the helper at both tab-bar sites (`render`, `render_main_only`), and pass the same block to Task 2's `record_tab_bar`. Adding a title does not change `Block::inner`; the recorded tab rects must be unchanged, and the Task 2 rect test proves it.
3. Update the DetailScreen `adjudicate_screen!` reason text (detail.rs ~2567) to name the new surface: the tab bar's title row draws `ctx.status_message`, escaped at the render site.
4. In `render_escape_guard.rs`:
   - add the `Detail view with a status message` sub-state and its arrival row (`true`, with a reason naming the title row);
   - add the reach test modelled on `the_status_footer_state_reaches_the_status_branch`.

**Help overlay (per D-07).** In `help_lines`, directly after the Waves-pane block, add a `Mouse` heading, a blank line, these rows and a trailing blank line. Keep every description unique in the body.

- `row("Click", "Mouse: switch tab / sub-tab; select a row and focus its pane")`
- `row("Double-click", "Mouse: open the row, the same as Enter")`
- `row("Wheel", "Mouse: move the selection in the pane under the pointer")`
- `row("M", "Toggle mouse capture on / off (dashboard and detail view)")`
- `row("Shift+drag", "Select and copy text while mouse capture is on (terminal-dependent)")`

Add the whole-row test.

**Docs (per D-07), mirroring how 1t1 and 2l4 edited these files.**

1. README.md:
   - add a `M` row to the dashboard key table (`Toggle mouse capture`);
   - add a short `Mouse` paragraph plus a table after the detail-view table: click, double-click, wheel, `M`, Shift+drag;
   - add one sentence naming `"mouse": false` under `preferences` in config.json to start with capture off;
   - add one Features bullet if the Features list has a natural place for it.
2. docs/GETTING-STARTED.md: in "Open the dashboard", add a bullet that you can click tabs and rows, double-click to open, and use the wheel to scroll. `M` turns the mouse off and on; while it is on, use Shift+drag to select text.
3. docs/CONFIGURATION.md: add `"mouse": true` to the sample `preferences` object and a `mouse` row to the `preferences` table (boolean, default `true`, what it does, `M` at runtime, not saved).
4. Wording must match the behaviour actually built; re-read the help rows before writing the docs.

Commits:

- code: `feat(quick-260926-dyf): detail wheel routing, M in the detail view and the status title`;
- docs: `docs(quick-260926-dyf): mouse controls in help, README, GETTING-STARTED and CONFIGURATION`.
  </action>
  <verify>
    <automated>rtk proxy cargo test --no-fail-fast -- mouse_ render_escape_guard help</automated>
  </verify>
  <done>
- All behaviours above pass.
- The full `rtk proxy cargo test --no-fail-fast` shows the baseline passed count plus the new tests, with exactly 1 failure (the expected envelope/policy.rs witness). Name any changed or retired tests in the SUMMARY.
- `rtk proxy cargo clippy -- -D warnings` exits 0. `rtk proxy cargo clippy --all-targets` shows exactly the 11 pre-existing findings.
- A tmux spot check at 100x30 is recorded in the SUMMARY:
  - click the Git tab;
  - click the Agents sub-tab;
  - wheel over the Waves pane on Phases;
  - press `M` in the detail view and see `Mouse off (M toggles)` at the right of the tab bar's title row.
- Two commits are made.
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| terminal → TUI input | Mouse reports arrive on the same stdin byte stream as keys. The terminal generates them, and anything that can write to the pty can forge them. |
| TUI → user's terminal state | Mouse-reporting modes (1000/1002/1003/1006) are global terminal state that outlives the process unless it is turned off. |
| status_message producers → render | Composed strings that can carry registry keys and run ids (third-party text) are drawn at a new site, the detail tab bar title. |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-dyf-01 | Denial of service | src/event.rs reader → biased Action FIFO (main_loop.rs pump) | high | mitigate | EnableMouseCapture turns on any-motion tracking. `ui::mouse::routed` forwards only Down(Left), ScrollUp and ScrollDown; motion, drag, release, other buttons and horizontal scroll are dropped before the FIFO. Pinned by `mouse_reader_forwards_only_left_press_and_vertical_wheel`. |
| T-dyf-02 | Tampering | src/tui.rs, src/main.rs (exit, panic, editor hand-off) | high | mitigate | `tui::restore()` always disables capture. A once-installed panic hook disables capture before ratatui's restore. The editor branch disables before hand-off and re-enables only if capture was on. The first enable happens inside `run_tui_loop`, after every fallible startup step. Proven end to end with the tmux `#{mouse_any_flag}#{mouse_sgr_flag}` readings in Task 1. |
| T-dyf-03 | Elevation of privilege | Screen::handle_mouse (detail, dashboard) | medium | mitigate | A single click only selects and focuses. Only a double-click runs Enter, through the unchanged keyboard path, and only on the row the first click selected. The wheel maps only to Down/Up. Modal and overlay screens keep the default (ignore), so a click can never confirm a dialog. Mouse input is ignored during text entry. Queue, driver and Space actions are unreachable by mouse. Pinned by the Task 2/3 tests and `mouse_clicks_are_ignored_while_typing_in_config`. |
| T-dyf-04 | Spoofing | detail tab bar status title | medium | mitigate | The message is escaped at the render site with `crate::text::render_for_terminal` (the WR-03 rule) and cut with `fit_cells`. The escape-guard sub-state `Detail view with a status message` plus its reach test prove that hostile identity text arrives escaped. |
| T-dyf-05 | Tampering | forged mouse reports on stdin | low | accept | A process that can write to the pty can already forge any key. Mouse input grants nothing beyond the keyboard: double-click equals Enter and the wheel equals Down/Up. |
| T-dyf-SC | Tampering | dependencies | low | accept | No new crates. crossterm 0.29 and ratatui 0.30 are already direct dependencies, and no package install happens. |
</threat_model>

<verification>
- `rtk proxy cargo test --no-fail-fast`: the baseline 2621 passed plus the new `mouse_*` tests, exactly 1 failure (`envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`), and 15 ignored. Read the raw `test result:` lines, never piped.
- `rtk proxy cargo clippy -- -D warnings` exits 0. `rtk proxy cargo clippy --all-targets` shows exactly the 11 pre-existing findings.
- The Task 1 tmux lifecycle readings are 11 / 00 / 11 / 00 (in editor) / 11 / 00 (after quit), and 00 at a `"mouse": false` launch.
- The Task 3 tmux spot check covers tab click, sub-tab click, wheel over the Waves pane, and `M` showing the status title.
- No render function gains `std::fs`, `read_to_string`, `read_dir`, `File::open`, `.exists()`, `is_file` or `is_dir`. List the render functions touched and state that each was checked.
</verification>

<success_criteria>
- D-01..D-08 are each implemented and cited in the SUMMARY with the test or measurement that proves them.
- I-1..I-15 are listed in the SUMMARY as implemented, or with the deviation and its reason. Execution-time decisions are numbered E-n.
- The terminal is never left in mouse-reporting mode on quit, on the editor hand-off, or at a `mouse: false` launch (measured). The panic hook is present and runs before ratatui's restore (by construction, with the code cited).
- 4 commits on master, not pushed.
</success_criteria>

<output>
Create `.planning/quick/260926-dyf-enable-mouse-support-in-the-tui/260926-dyf-SUMMARY.md` when done. Follow the 2l4/1t1 SUMMARY shape:

- commits table;
- measured test and lint gates;
- final mouse map (dashboard and detail);
- inferred decisions table (I-1..I-15, E-n);
- tmux readings;
- changed or retired tests (old → new);
- deferred items, including the pre-existing raw-mode leak on early startup errors;
- threat flags;
- Self-Check.
</output>
