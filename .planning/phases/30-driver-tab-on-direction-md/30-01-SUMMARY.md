---
phase: 30-driver-tab-on-direction-md
plan: 01
subsystem: ui
tags: [rust, ratatui, tui, driver-tab, watcher, launcher, frontmatter, run-lock]

requires:
  - phase: 27-driverctl-driver-frontmatter
    provides: read_driver_run / list_driver_runs / DerivedStatus (frontmatter plus run-lock probe)
  - phase: 28-sandboxed-driver-launcher
    provides: stop_run, LauncherStopRequested/LauncherStopped, the 28-07 section this plan re-hosts
provides:
  - launcher-first Driver tab (driver_run.rs): run list, selected-run header rows 1-2, status vocabulary, empty/loading/unreadable states
  - ChangeKind::DriverRun / ChangeKind::Ignored and the drivers/ allowlist (no re-parse for run-dir events)
  - Action::LauncherRunsRefreshed as the single producer of launcher rows, AppContext::schedule_launcher_refresh
  - one selection across launcher rows then read-only legacy rows (L expands the collapsed section)
  - x = kill the selected launcher run (Kill driver run confirmation), DriverKeys-driven footer, Driver Run States help legend
affects: [30-02, 30-03, 30-04, 30-05, 31]

actuals:
  tokens: 33000
  tasks: 3
  commits: 6
plan_head_before: b24aef76b332899de40e8153a68ab7595487bc02
plan_head_after: 3cbdcb6bddb462f6acfbfb1bfaf6ec89d38976ea

tech-stack:
  added: []
  patterns:
    - "one off-thread producer per cache field (spawn_blocking -> Action), replaced wholesale, selection pinned to an id"
    - "status, glyph and colour only from DerivedStatus; fixed &'static str glyph constants"
    - "untrusted text: shown_capped first, fit_cells (display width) second"
    - "footer = prioritised whole tokens gated by a DriverKeys availability struct"

key-files:
  created:
    - src/ui/screens/driver_run.rs
  modified:
    - src/state_reader/driver_frontmatter.rs
    - src/journal/mod.rs
    - src/watcher.rs
    - src/action.rs
    - src/app.rs
    - src/ui/screens/mod.rs
    - src/ui/screens/driver.rs
    - src/ui/screens/detail.rs
    - src/ui/screens/driver_confirm.rs
    - src/ui/screens/help.rs
    - src/ui/screens/render_escape_guard.rs
    - src/ui/mod.rs

key-decisions:
  - "Selection is pinned to the run id (launcher_selected_id); a refresh that adds a newer run keeps the same run selected, a vanished run clamps the index"
  - "A legacy selection is honoured only while the legacy rows are shown, so collapsing can never leave the detail on a hidden row"
  - "x on an unreadable launcher run opens the kill confirmation (it is treated like liveness-unknown) [inferred - audit]"
  - "The legacy-row footer suffix sits before [?]help so [?]help is always last [inferred - audit]"
  - "The bare drivers/ directory itself classifies as Ignored [inferred - audit]"

patterns-established:
  - "Allowlist routing for a noisy directory: only the paths a consumer reads become events, everything else is Ignored before an Action is built"
  - "Drop-by-priority footer builder that never splits a token and never drops the help key"

requirements-completed: [DTAB-01, DTAB-02]

coverage:
  - id: D1
    description: "The Driver tab lists a project's launcher runs newest first; the selected run's status is read from the frontmatter plus the run-lock probe, never from the written status word (a running file with a free lock reads crashed, a held lock reads running, a failed probe reads liveness unknown)."
    requirement: "DTAB-01"
    verification:
      - kind: e2e
        ref: "src/app.rs#launcher_view_a_free_lock_reads_crashed_and_a_held_lock_reads_running"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver_run.rs#status_cell_follows_the_ui_spec_table_for_every_derived_status"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver_run.rs#the_header_reads_the_lock_not_the_file_for_a_crashed_run"
        status: pass
    human_judgment: false
  - id: D2
    description: "Watcher events under .planning/meta-manager/drivers/ refresh the launcher list for the open Driver tab without a project re-parse; splice temps, state.json, locks and swap files emit nothing."
    requirement: "DTAB-02"
    verification:
      - kind: unit
        ref: "src/journal/mod.rs#classify_change_ignores_every_other_drivers_path"
        status: pass
      - kind: unit
        ref: "src/watcher.rs#an_ignored_drivers_path_emits_no_event"
        status: pass
      - kind: integration
        ref: "src/app.rs#launcher_view_a_driver_run_event_refreshes_without_a_reparse"
        status: pass
    human_judgment: false
  - id: D3
    description: "Every disk touch for the run list runs on spawn_blocking and returns through Action::LauncherRunsRefreshed; the 20-tick rescan re-lists so a SIGKILL-released lock reads crashed within one slow tick."
    requirement: "DTAB-02"
    verification:
      - kind: integration
        ref: "src/app.rs#launcher_view_the_slow_tick_schedules_the_launcher_refresh"
        status: pass
    human_judgment: false
  - id: D4
    description: "Legacy Phase 17/18 runs collapse into one read-only row below the launcher list (L expands it); the 28-07 bottom section and its {, } and X keys are gone; x kills the selected launcher run through the Kill driver run confirmation and refuses on a legacy or not-running row."
    requirement: "DTAB-01"
    verification:
      - kind: unit
        ref: "src/ui/screens/detail.rs#launcher_view_x_kills_a_live_crashed_or_unknown_run_through_the_confirmation"
        status: pass
      - kind: unit
        ref: "src/ui/screens/detail.rs#launcher_view_j_walks_into_legacy_rows_only_when_the_section_is_expanded"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver_confirm.rs#launcher_stop_prompt_names_the_run_the_alias_and_the_whole_scope"
        status: pass
    human_judgment: false
  - id: D5
    description: "The footer names only keys that act on the current selection (no [i]nject, [X] or [{/}] at any width, never wider than its width, [?]help last) and the help screen documents the new list keys and every run state."
    requirement: "DTAB-01"
    verification:
      - kind: unit
        ref: "src/ui/screens/detail.rs#the_driver_footer_fits_its_width_and_always_ends_with_help"
        status: pass
      - kind: unit
        ref: "src/ui/screens/help.rs#the_driver_run_states_legend_names_every_run_status_with_its_glyph"
        status: pass
    human_judgment: false
  - id: D6
    description: "Kill confirmation dialog: red title, UI-SPEC copy, default focus No (the engine could not classify this dialog's other states, e.g. a second x while a kill is in flight)."
    verification: []
    human_judgment: true
    rationale: "Flagged assumption carried from the plan (UI unclassified, kill dialog): copy, colour and default focus are asserted by test, but whether a second kill while stop_run is still waiting should queue or refuse is a product call."

duration: 31min
completed: 2026-10-21
status: complete
---

# Phase 30 Plan 01: Driver Tab, Launcher Runs First Summary

**Launcher runs as the Driver tab's run list, with status derived from the frontmatter plus the run-lock probe (a running file with a free lock reads crashed), watcher events routed to an off-thread refresh instead of a re-parse, legacy runs folded into one read-only row, and `x` as the only kill key.**

## Performance

- **Duration:** 31 min
- **Started:** 2026-10-20T23:59:36Z
- **Completed:** 2026-10-21T00:30:00Z
- **Tasks:** 3 (6 commits: Task 1 in four groups)
- **Files modified:** 13 (1 created)

## Accomplishments

- A fixture run whose frontmatter says `running` with `launches: 1` renders `✗ crashed` (Red, BOLD) in its list row and header while its `launcher.lock` is free, and `◆ running` once a test thread holds the lock (`driver::lock::acquire_at`); `LivenessUnknown` is `◇ liveness unknown`, never crashed or running. The tracer path (watcher event, off-thread listing, cache, render) is proven end to end by one app test.
- `ChangeKind::DriverRun` / `Ignored` with an allowlist in `classify_change`: only the run dir, `DIRECTION.md` and `LOG.md` become events; `state.json`, `.DIRECTION.md.fm.*`, `.DIRECTION.md.edit`, the locks, swap files and deeper paths are dropped before an `Action` exists; a run-dir event never reaches `schedule_reparse` and never touches `last_refresh`.
- `Action::LauncherRunsRefreshed` is the single producer of launcher rows (`DriverRunsListed.launcher_runs` removed); the tab-entry, watcher, 20-tick and post-stop paths all call `schedule_launcher_refresh`; the selection is pinned to the run id.
- `read_driver_run` refuses a `DIRECTION.md` over 1 MiB as an unreadable row (T-30-04).
- The 28-07 bottom section and `{`/`}`/`X` are deleted; legacy journal runs collapse to `▸ legacy runs (n), read-only` (`L` expands); `j`/`k`/mouse walk launcher rows then legacy rows; `x` pushes `Kill driver run` (Red, default No) for a live, crashed, liveness-unknown or unreadable run and says why otherwise.
- Footer is a `DriverKeys`-gated, priority-dropping token builder; the help screen documents `x`, `L` and a `Driver Run States` legend.

## Task Commits

1. **Task 1 group A: reader cap** - `38251d5` (feat)
2. **Task 1 group B: off-thread listing** - `86ec47b` (feat)
3. **Task 1 group C: watcher route** - `e4e1697` (feat)
4. **Task 1 group D: view** - `6e4a124` (feat)
5. **Task 2: legacy collapse, 28-07 removal, x kills** - `efc8248` (feat)
6. **Task 3: footer and help** - `3cbdcb6` (feat)

**Plan metadata:** committed with this file (docs: complete plan)

## Files Created/Modified

- `src/ui/screens/driver_run.rs` - view model and render: `status_cell`, `run_row`, header rows 1-2, `render_launcher_tab`, selection/legacy toggling, `kill_available`, `DriverKeys`, run-state legend; no file I/O
- `src/journal/mod.rs` / `src/watcher.rs` - `ChangeKind::{DriverRun, Ignored}` and the `drivers/` allowlist; `batch_actions` drops Ignored
- `src/action.rs` / `src/app.rs` / `src/ui/screens/mod.rs` - `LauncherRunsRefreshed`, `schedule_launcher_refresh`, `FileChanged` arms, new cache fields (`launcher_listed`, `launcher_selected_id`, `legacy_expanded`, `legacy_row_selected`)
- `src/state_reader/driver_frontmatter.rs` - `DIRECTION_READ_MAX_BYTES`
- `src/ui/screens/detail.rs` - `render_driver`, `j/k`/mouse selection, `x`, `L`, footer builder; `{`/`}`/`X` arms deleted
- `src/ui/screens/driver.rs` - 28-07 section, `render_driver_tab`, `no_runs_lines` and their tests removed; helpers made `pub(super)`
- `src/ui/screens/driver_confirm.rs` - `Kill driver run` title and UI-SPEC kill copy
- `src/ui/screens/help.rs` - Driver rows and `Driver Run States` legend
- `src/ui/screens/render_escape_guard.rs`, `src/ui/mod.rs` - probe states and census pins for the new composition

## Decisions Made

- Selection pinned to `launcher_selected_id`; a legacy selection counts only while the legacy rows are shown (so a stale flag cannot leave the detail on a hidden row).
- Task 1 group D routed the composition by "project has a launcher run" and Task 2 made it final (as planned); the empty state and `Loading...` live in `render_launcher_tab` from the start.
- Kept the cell cut by display width (`fit_cells`) for run ids, per orchestrator decision 2.
- `[inferred - audit]` x on an unreadable run opens the kill confirmation. Nothing establishes that such a run is not running (a DIRECTION.md grown past the read cap is still a live scope), and the old `X` allowed it; `stop_run` reports "not running" when the lock is free.
- `[inferred - audit]` the legacy-row footer suffix (`legacy run: read-only`) is placed before `[?]help`, because the plan requires the footer to always end with `[?]help` and the UI-SPEC table places the suffix after it.
- `[inferred - audit]` the bare `drivers` directory classifies as `Ignored` (it names no run).
- `[inferred - audit]` the opt-in empty-state copy (`"X" is not opted in to driving. Press [o]`) is replaced by the UI-SPEC empty state; `o` still works and is documented in help, but the empty state no longer advertises it.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `pub mod driver_run;` registered in group D, not B**
- **Found during:** Task 1 group B
- **Issue:** the plan lists the module declaration with step 5 (group B), but the file only exists from step 8 (group D); declaring it earlier would not build.
- **Fix:** declared the module in group D with the file.
- **Committed in:** `6e4a124`

**2. [Rule 3 - Blocking] render-escape guard needed a separate legacy state in Task 1**
- **Found during:** Task 1 group D
- **Issue:** with `probe_ctx` seeding `launcher_runs`, the default `Driver tab` state draws the launcher composition, so the legacy sites (run detail, output pane, injection rows, dry-run preview) were unreachable and `the_two_new_driver_states_reach_their_branches` went red.
- **Fix:** added a `Driver tab, legacy runs` probe state (and its `DETAIL_TAB_ARRIVAL` row); in group D it emptied `launcher_runs`, in Task 2 it expands the legacy section (`legacy_expanded`, `legacy_row_selected`) as the plan describes.
- **Files modified:** `src/ui/screens/render_escape_guard.rs`
- **Committed in:** `6e4a124`, `efc8248`

**3. [Rule 1 - Bug] Census and help-test pins moved with the removed code**
- **Found during:** Task 2 and Task 3
- **Issue:** removing `no_runs_lines` shifted the display-identity census pins (`(5,6,7)` to `(4,5,6)`; `MEASURED_REACH` driver.rs 2 to 1, file count now 17); the help test that forbids the word `read` outside the queued gloss tripped on the new `read-only` and `unreadable` copy.
- **Fix:** re-measured the pins and their docs in the same commit; the help test excludes the legacy `read-only` row and the run-state legend from the `read` count (it is about injected messages), still checking `sent`/`received`/`acknowledged` over the whole body.
- **Files modified:** `src/ui/screens/driver.rs`, `src/ui/mod.rs`, `src/ui/screens/help.rs`
- **Committed in:** `efc8248`, `3cbdcb6`

**4. [Rule 3 - Blocking] 80x24 render test asserts glyphs, words at 100 columns**
- **Found during:** Task 1 group D
- **Issue:** the plan asks for "the expected glyph/word per row" at 80x24, but at 80 columns the list's inner width is 31, below `RUN_ROW_WORD_MIN_CELLS` (36) from the UI-SPEC, so the word column legitimately does not appear.
- **Fix:** assert glyph and run id at 80x24 and glyph plus word at 100x24.
- **Committed in:** `6e4a124`

### Legacy tests rewritten rather than ported

- `driver.rs`: the ten 28-07 section tests (`launcher_rows_badge_a_degraded_run...`, `launcher_rows_use_the_derived_status...`, `launcher_an_unreadable_run...`, `launcher_a_hostile_run_id...`, `launcher_a_narrow_pane...`, `launcher_a_project_with_only_launcher_runs...`, `launcher_one_spare_row...`, `launcher_a_pane_with_no_spare_row...`, `launcher_stop_highlight...`, `launcher_a_short_pane...`) and `the_empty_run_list_copy_differs_by_opt_in_state` were deleted with the code they exercised; `driver_run` tests cover posture cell, derived status, unreadable, hostile text, narrow cut, offset window and the empty state. The `UNSANDBOXED` badge text has no home until 30-02's header row 4; the list row shows the `!` posture cell meanwhile. The four legacy tier tests that called `render_driver_tab` now call it through a test wrapper over `render_launcher_tab` with the legacy section expanded.
- `detail.rs`: `launcher_stop_braces_*` (2), `launcher_stop_capital_x_*` (2) and `launcher_stop_keys_exist_only_on_the_driver_tab` became `launcher_view_*` tests; `pressing_x_on_the_driver_tab_pushes_the_stop_confirmation` now targets a launcher run; the two three-form footer tests were replaced by the `DriverKeys` footer tests; the Driver mouse tests and `changing_the_selected_run_resets_...` set the legacy-expanded flags.
- `app.rs`: the `DriverRunsListed` launcher-row tests were ported to `LauncherRunsRefreshed` (replace, clamp) and the stop-relist test now waits for both listings.
- `driver_confirm.rs`: three tests' strings updated to the kill copy.

---

**Total deviations:** 4 auto-fixed (2 blocking-ordering, 1 pin bookkeeping, 1 test-width). **Impact on plan:** none on scope; all are ordering or measurement adjustments.

## Issues Encountered

None.

## Verification

- `mkdir -p -m700 /tmp/claude-1000/t; umask 077; TMPDIR=/tmp/claude-1000/t rtk proxy cargo test --no-fail-fast`: **3749 passed, 1 failed, 24 ignored.** The one failure is the expected git-version witness `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against` (local git 2.53.0, pinned 2.55.0).
- `rtk proxy cargo clippy --all-targets -- -D warnings`: clean.
- Task verify filter `classify_change watcher:: driver_frontmatter screens::driver_run:: launcher_view_ render_escape_guard`: 77 passed, including `launcher_view_a_free_lock_reads_crashed_and_a_held_lock_reads_running`.
- Cfg hygiene (RESEARCH Pitfall 13): reviewed by hand (only the Linux target is installed). New production code adds no call into a `#[cfg(unix)]` / `#[cfg(target_os = "linux")]` item: `driver_run.rs` and the footer are platform-neutral, the empty-state body picks its text with `cfg!(target_os = "linux")`, `read_driver_run` uses `File`/`metadata` only, and `LauncherStopRequested` already had its non-Linux arm. The one lock-dependent test is `#[cfg(unix)]`.
- Group commits: one hash per group A-D above; no group was merged into a neighbour (only the module declaration moved from B to D, deviation 1).

## Next Phase Readiness

- Ready for 30-02: the selected-run detail is two header rows; items, caps, connections and the LOG pane fill into `detail_lines` / `render_launcher_tab`. `DriverKeys` and `FooterToken` are the extension points for the new keys; `status_cell` has no `stale` variant yet (it needs a `now`).
- 30-05 owns `s`; the opt-in empty-state hint is gone from the empty state (see Decisions).

## Self-Check: PASSED

- Created/modified files exist (13 of 13 checked); commit hashes `38251d5 86ec47b e4e1697 6e4a124 efc8248 3cbdcb6` are ancestors of HEAD; `check evaluation-scope --plan 30-01 --commits-only` resolves with the six commits; acceptance greps for all three tasks hold; no deletions outside the intended code.

---
*Phase: 30-driver-tab-on-direction-md*
*Completed: 2026-10-21*
