---
phase: 30-driver-tab-on-direction-md
plan: 03
subsystem: ui
tags: [rust, ratatui, tui, dashboard, badges, layout-tiers, height-tiers, status-tokens, attention-filter]

requires:
  - phase: 30-driver-tab-on-direction-md
    provides: 30-01 launcher-first Driver tab and LauncherRunsRefreshed; 30-02 heartbeat_stale, the FULL section renderers and the LOG tail
  - phase: 27-driverctl-driver-frontmatter
    provides: list_driver_runs, DerivedStatus (frontmatter plus run-lock probe)
provides:
  - "driver_run::{MarkerClass, LauncherMarker, run_marker_class, launcher_marker}: one dashboard marker per project, crashed > parked > driven > stale > unknown"
  - "app::launcher_markers_for riding the existing 20-tick reconcile spawn_blocking; Action::RunsReconciled.launcher_markers (wholesale) and Action::LauncherMarkerScanned (one-project refresh)"
  - "AppContext::needs_attention_for (h filter, AttentionFirst rank 0), the BADGE_CRASHED / BADGE_STALE / BADGE_UNKNOWN badges and the ten-rank alias_badge table, the tab-bar light on any_live, badge legend rows"
  - "driver_run::{HeightTier, height_tier, allocate_rows, status_tokens, summary_row}: FULL / MEDIUM / SUMMARY tiers with no orphaned rule or header at any height"
  - "legacy run detail drops the pipeline row and steps section together at 8..=13 rows (Phase 18 WR-05)"
affects: [30-04, 30-05, 31]

actuals:
  tokens: 38000
  tasks: 3
  commits: 3
plan_head_before: 49c2edb5ae3a00ed89cde62b719046712b550491
plan_head_after: c5ea9d3ea209ce439e40883283d560b42e8fa2e6

tech-stack:
  added: []
  patterns:
    - "one blocking task per concern on the existing counter: the marker scan shares the reconcile spawn_blocking and its 20-tick counter, so a SIGKILL'd launcher turns into a badge within one slow tick with no second timer"
    - "authoritative-by-absence: RunsReconciled replaces every marker wholesale; an alias with no marker has no badge and no attention contribution (never a guessed class)"
    - "tier allocation as a pure function (floors first, spare rows to items, then connections, then the LOG) with const-asserted sums, and sections rebuilt to their allotted rows so a rule never loses its content"
    - "status tokens as prioritised whole units dropped benign-first, flag-last, never split"

key-files:
  created: []
  modified:
    - src/ui/screens/driver_run.rs
    - src/ui/screens/driver.rs
    - src/ui/screens/normal.rs
    - src/ui/screens/mod.rs
    - src/ui/screens/detail.rs
    - src/ui/screens/help.rs
    - src/ui/screens/render_escape_guard.rs
    - src/ui/mod.rs
    - src/action.rs
    - src/app.rs

key-decisions:
  - "MarkerClass is declared crashed-first so its derived Ord ranks the highest precedence lowest; a project marker is the Ord::min over its runs, and needs_attention is crashed | parked | stale only"
  - "A run's class is chosen only from DerivedStatus (lock probe), the closed ItemState enum and the file's own heartbeat threshold; stale, unknown, not-launched and paused runs are never crashed"
  - "needs_attention_for feeds the h filter and AttentionFirst rank 0, while the badge's rank-4 input stays the legacy needs_human_for alone, so a stale project shows the stale badge and still sorts first"
  - "The 20-tick scan, the event-driven one-project scan and the launcher tab-bar light are all gated on the experimental flag; with it off no launcher file is read"
  - "Status tokens drop benign tokens first and flags last (each group from the end of the UI-SPEC order), so MISMATCH, UNSANDBOXED, cap! and a stale heartbeat outlive audit, prompt custom and even ctl"
  - "Below 12 rows (inner 11) the whole tab is one summary row per run; between 12 and 20 rows the MEDIUM layout, from 21 the FULL layout"

patterns-established:
  - "items_section_fit / connected_section_fit: a section built to at most N lines, ending in a '... k more' line, never header-only"
  - "assert_no_orphans over a TestBackend H x W sweep as the D-29 backstop, with a mutation check that the sweep goes red on the old WR-05 layout"

requirements-completed: [DTAB-02, DTAB-01]

coverage:
  - id: D1
    description: "A launcher run's dashboard class is one of crashed, parked, stale, driven or unknown by the first match of the UI-SPEC table (a live run with a stale heartbeat is stale, not driven; a free-lock running run is crashed; liveness unknown, not launched and paused runs are never crashed); a project's marker is the highest class over its non-terminal runs plus the most recent run."
    requirement: "DTAB-02"
    verification:
      - kind: unit
        ref: "src/ui/screens/driver_run.rs#launcher_marker_precedence_truth_table"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver_run.rs#launcher_marker_a_stale_live_run_is_stale_not_driven"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver_run.rs#launcher_marker_a_non_terminal_older_run_behind_a_newer_done_run_is_still_considered"
        status: pass
    human_judgment: false
  - id: D2
    description: "A launcher run that died without a final status in any registered project lights the crashed badge on its dashboard row and is the only row the h filter lists; with the experimental flag off neither changes; before the first scan nothing is guessed."
    requirement: "DTAB-02"
    verification:
      - kind: e2e
        ref: "src/app.rs#launcher_marker_a_crashed_run_lights_the_badge_and_the_h_filter"
        status: pass
    human_judgment: false
  - id: D3
    description: "The dashboard badge follows the ten-rank table (crashed, parked, driven, legacy needs-human, stale, unknown, then paused / external job / session), at most one two-cell width-stable glyph; a stale project sorts first under AttentionFirst while its badge is the stale glyph, and an unknown one does not move."
    requirement: "DTAB-02"
    verification:
      - kind: unit
        ref: "src/ui/screens/normal.rs#launcher_marker_badge_ranks_follow_the_ui_spec_table"
        status: pass
      - kind: unit
        ref: "src/ui/screens/normal.rs#launcher_marker_attention_first_sorts_a_stale_project_first_and_unknown_not_at_all"
        status: pass
      - kind: unit
        ref: "src/ui/screens/normal.rs#badge_glyphs_are_width_stable"
        status: pass
    human_judgment: false
  - id: D4
    description: "The marker scan runs in the same blocking task as reconcile_all on the existing 20-tick counter and only with the experimental flag on; a DriverRun event for a project whose Driver tab is not on top schedules a one-project marker scan, and with the tab on top the refresh handlers recompute the marker from their rows; no second timer (interval( sites under src/ is still 2)."
    requirement: "DTAB-02"
    verification:
      - kind: integration
        ref: "src/app.rs#launcher_marker_the_slow_tick_scans_only_with_the_flag_on"
        status: pass
      - kind: integration
        ref: "src/app.rs#launcher_marker_a_driver_run_event_off_the_tab_rescans_the_one_project"
        status: pass
      - kind: integration
        ref: "src/app.rs#launcher_marker_the_open_tab_recomputes_the_marker_from_its_rows"
        status: pass
    human_judgment: false
  - id: D5
    description: "The D:Drive tab-bar marker lights when any launcher run of the project holds its lock, and the badge legend gains the crashed, stale and unknown rows, skipped with the experimental flag off."
    requirement: "DTAB-02"
    verification:
      - kind: unit
        ref: "src/ui/screens/detail.rs#launcher_marker_driver_live_for_lights_on_any_live"
        status: pass
      - kind: unit
        ref: "src/ui/screens/help.rs#launcher_marker_the_badge_legend_gains_three_rows_gated_on_the_flag"
        status: pass
    human_judgment: false
  - id: D6
    description: "The Driver tab never renders a section rule or pane title without content directly under it, and never a rule on the bottom row, at any height from 1 to 40 and any width of 40, 60, 80 or 120, for a launcher run with items, caps, connections and LOG lines and for a selected legacy run; FULL / MEDIUM / SUMMARY draw the sections the UI-SPEC names; H == 0 draws nothing."
    requirement: "DTAB-01"
    verification:
      - kind: unit
        ref: "src/ui/screens/driver_run.rs#launcher_orphan_sweep_no_rule_or_header_without_content_at_any_height"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver_run.rs#legacy_orphan_sweep_no_rule_or_header_without_content_at_any_height"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver_run.rs#the_tiers_draw_the_sections_the_ui_spec_names"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver.rs#legacy_wr05_orphan_free_detail_between_eight_and_thirteen_rows"
        status: pass
    human_judgment: false
  - id: D7
    description: "Status tokens follow the UI-SPEC order and styles, are dropped whole (benign first, flags last, never split) and feed MEDIUM header row 2 and every SUMMARY row; MISMATCH and UNSANDBOXED stay whole on the 39-cell detail floor and every cap bar keeps its 10 cells."
    requirement: "DTAB-01"
    verification:
      - kind: unit
        ref: "src/ui/screens/driver_run.rs#the_status_tokens_are_dropped_whole_benign_first_and_never_split"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver_run.rs#cap_bars_and_flag_tokens_are_whole_at_the_detail_floor"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver_run.rs#summary_row_is_highlight_glyph_word_id_and_the_tokens_that_fit"
        status: pass
    human_judgment: false
  - id: D8
    description: "Hostile run ids, status words and control words reach no terminal cell unescaped through a SUMMARY row or the MEDIUM token row."
    requirement: "DTAB-01"
    verification:
      - kind: unit
        ref: "src/ui/screens/render_escape_guard.rs#the_summary_row_and_the_medium_token_row_render_escaped"
        status: pass
    human_judgment: false
  - id: D9
    description: "Whether the three layouts read well at real terminal sizes (where MEDIUM gives way to SUMMARY, the look of the token row, the id floor in a SUMMARY row) is a design judgement."
    verification: []
    human_judgment: true
    rationale: "Every structural property (no orphan, sums, whole tokens) is asserted by test; whether the compositions are pleasant to read at 80x24 and 100x30 is a taste call no test settles."

duration: 31min
completed: 2026-10-21
status: complete
---

# Phase 30 Plan 03: Dashboard Markers and Height Tiers Summary

**Crashed, parked, driven, stale and unknown launcher runs now mark their project on the dashboard (one badge, the h filter and the attention sort, the tab-bar light), scanned on the existing 20-tick reconcile task so a SIGKILL'd run turns red within one slow tick, and the Driver tab lays out as FULL, MEDIUM or one-row-per-run SUMMARY with no rule or header ever left without content.**

## Performance

- **Duration:** 31 min
- **Started:** 2026-10-21T01:07:40Z
- **Completed:** 2026-10-21T01:38:36Z
- **Tasks:** 3 (3 commits)
- **Files modified:** 10 (none created)

## Accomplishments

- **Tracer (Task 1):** `launcher_marker` reduces a project's newest-first run listing (every run whose written status is not terminal, plus the most recent) to one `LauncherMarker`; `launcher_markers_for` runs inside the reconcile `spawn_blocking` on the same 20-tick counter and is skipped with the experimental flag off; `RunsReconciled.launcher_markers` replaces the markers wholesale. `needs_attention_for` feeds the `h` filter and AttentionFirst rank 0. `launcher_marker_a_crashed_run_lights_the_badge_and_the_h_filter` proves two registered projects, one crashed: `✗ ` Red BOLD on the first, nothing on the second, `//h` lists only the first, and the flag off changes neither. The tracer was re-verified end to end before expanding.
- **Markers (Task 2):** `alias_badge` now ranks crashed, parked, driven (marker or legacy), the legacy needs-human flag, stale (`◈ `), unknown (`◇ `), then paused / external job / session; `LauncherMarkerScanned` refreshes one project on a `DriverRun` event when its tab is not on top, and the open-tab handlers recompute the marker from their rows; `driver_live_for` lights on `any_live`; the badge legend gains three rows gated like the driven row.
- **Tiers (Task 3):** `height_tier` (FULL from a 21-row tab area, MEDIUM from 12, SUMMARY below), a pure `allocate_rows` (floors, then items, then connections, then the LOG, const-asserted to `6+3+5+2+4 = 20` and `2+3+2+4 = 11`), sections rebuilt to their allotted rows (`items_section_fit`, `connected_section_fit`, `cap_rows`), `status_tokens`, `summary_row`, and the legacy detail's header-plus-output layout at 8 to 13 rows. The two `TestBackend` sweeps (launcher and legacy, H 1..=40 at W 40/60/80/120) are the D-29 backstop; the legacy sweep was mutation-checked against the old layout and goes red at H = 12.
- **Folded todo:** `.planning/todos/pending/2026-07-29-driver-tab-layout-at-medium-terminal-heights.md` is **resolved by D-29** (the legacy detail no longer paints a blank pipeline row and a bare `steps` rule at 8 to 13 rows). It is left in place; the phase close moves it.
- **Timer count:** `grep -rn 'interval(' src --include=*.rs | wc -l` is still **2** (the UI tick in `src/event.rs` and the legacy inbox poll in `src/driver/run.rs`): no new timer.

## Task Commits

1. **Task 1 (tracer): a crashed launcher run lights the dashboard badge and the h filter** - `42ce31f` (feat)
2. **Task 2: parked, driven, stale and unknown markers, event refresh, tab-bar light, badge legend** - `9378589` (feat)
3. **Task 3: height tiers, status tokens and SUMMARY rows, no orphaned header at any height** - `c5ea9d3` (feat)

**Plan metadata:** committed with this file (docs: complete plan)

## Files Created/Modified

- `src/ui/screens/driver_run.rs` - `MarkerClass`, `LauncherMarker`, `run_marker_class`, `launcher_marker`; `HeightTier`, `height_tier`, `allocate_rows`, `compose_selected`, `status_tokens`, `summary_row`; `*_fit` sections; the SUMMARY branch of `render_launcher_tab`; `detail_lines` removed
- `src/ui/screens/driver.rs` - legacy `render_run_detail` 8..=13 rows draws header + output only (WR-05)
- `src/ui/screens/normal.rs` - `BADGE_CRASHED/STALE/UNKNOWN`, `BadgeInputs::launcher`, the ten-rank `alias_badge`, `row_badge` made `pub(crate)`
- `src/ui/screens/mod.rs` - `ProjectViewCache::launcher_marker`, `needs_attention_for`, `apply_launcher_markers`, `refresh_launcher_marker_from_cache`, `schedule_launcher_marker`
- `src/ui/screens/detail.rs` - `driver_live_for` also lights on the marker's `any_live`
- `src/ui/screens/help.rs` - three badge legend rows and `DRIVER_ONLY_BADGES`
- `src/ui/screens/render_escape_guard.rs` - SUMMARY-row and MEDIUM-token-row escape test
- `src/ui/mod.rs` - display-identity census pin (render_escape_guard.rs 2 to 3) and its disclosed table
- `src/action.rs` - `RunsReconciled.launcher_markers`, `LauncherMarkerScanned`
- `src/app.rs` - `launcher_markers_for`, the tick wiring, the handlers, tests

## Decisions Made

- `[inferred — audit]` **Token drop order.** The UI-SPEC says both "dropped whole from the end" and "a flag token always outlives a benign one / MISMATCH survives ahead of every benign token", while its order puts the benign `ctl` first. Only one reading satisfies both: keep the emission order, but drop benign tokens first (from the end), then flag tokens (from the end). So `ctl` goes before a stale `hb` or `MISMATCH`.
- `[inferred — audit]` **`ctl` token without information.** A run with neither a loaded body nor a frontmatter control echo shows no `ctl` token rather than a claim about a body nobody read (non-selected SUMMARY rows have no cached detail and use the frontmatter echo).
- `[inferred — audit]` **AttentionFirst rank 1 is unchanged.** The UI-SPEC says "driven stays rank 1" and the plan routes only the rank-0 input through `needs_attention_for`; a launcher-driven project gets the driven badge but does not gain rank 1 in the sort.
- `[inferred — audit]` **No throttle on the event-driven scan.** A `DriverRun` event for a closed-tab project schedules one 20-run listing with no in-flight guard; the watcher's 200 ms debounce and the 20-tick wholesale replacement are the bounds (see Threat Flags).
- `[inferred — audit]` **`apply_launcher_markers` creates no cache entry just to hold `None`,** and a late `LauncherMarkerScanned` for an unregistered alias is dropped.
- `[inferred — audit]` **Items / connected budgets.** The "... k more" line counts toward the section's rows; with room for the rule and one line only, the section shows the cursor item (or the first connection) alone, never a bare rule. MEDIUM omits the cursor-item row.
- `[inferred — audit]` **SUMMARY row id floor.** The run id yields to the status tokens down to 12 cells, so a flag is not pushed off a narrow row by a long id; the row is clipped to its width below the fixed 15 cells. Below 12 rows the SUMMARY tier applies to a legacy selection too (the legacy detail's 8..=10 row arms are then unreachable from the tab, but stay correct and tested).
- `[inferred — audit]` **Help copy.** The legend rows are `driver run crashed - its lock is free, never finished`, `driver heartbeat stale - alive but silent` and `driver run liveness unknown - lock probe failed`, kept short to fit the 64-cell help popup at 80 columns; the needs-a-human row now reads `waiting on a human (a parked driver run too)`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] A 30-01 test asserted the premise D-13 replaces**
- **Found during:** Task 2
- **Issue:** `launcher_view_a_driver_run_event_with_the_tab_closed_schedules_nothing` asserted that a `DriverRun` event with the tab closed sends nothing at all; D-13 requires it to schedule the one-project marker scan.
- **Fix:** renamed `..._schedules_only_the_marker_scan`; it now fails on a launcher listing or run read and accepts only `LauncherMarkerScanned`, with no re-parse.
- **Files modified:** `src/app.rs`
- **Committed in:** `9378589`

**2. [Rule 3 - Blocking] A 30-02 test called the removed `detail_lines`**
- **Found during:** Task 3
- **Issue:** the plan replaces the stacked composition; `a_narrow_pane_cuts_the_control_word_before_the_heartbeat_flag` read header row 3 out of `detail_lines`.
- **Fix:** it calls `header_rows` directly (same row, same assertion).
- **Files modified:** `src/ui/screens/driver_run.rs`
- **Committed in:** `c5ea9d3`

**3. [Rule 3 - Blocking] Display-identity census pin**
- **Found during:** Task 3
- **Issue:** the new escape-guard test needs `display_identity(..)` as an oracle, which `ui::tests::no_display_identity_call_under_ui_stands_outside_a_composition` counts (render_escape_guard.rs 2 to 3).
- **Fix:** `MEASURED_REACH` and its disclosed table re-measured in the same commit, as the failure message prescribes; the five non-exempt occurrences are unchanged.
- **Files modified:** `src/ui/mod.rs`
- **Committed in:** `c5ea9d3`

---

**Total deviations:** 3 auto-fixed (1 bug-premise, 2 blocking). **Impact on plan:** none on scope; a test premise, a removed helper and a bookkeeping pin.

## Issues Encountered

- One full-suite run under load (the first after Task 3 changes) additionally failed `driver::lock::tests::probe_at_reports_absent_free_and_held` and `tests/envelope_pr_cap.rs#an_attempt_exactly_at_the_boundary_still_counts_against_the_window`. Neither file is touched by this plan; both pass in isolation and in the next full run, so they read as timing flakes under parallel load, not regressions. Noted for the phase verifier.
- The unresolved flagged assumption (DTAB-02): a run whose written status is `stalled` does **not** raise attention. This plan follows D-12 literally (crashed, parked, stale only) and only adds a most-recent launcher-written `crashed`.

## Verification

- `mkdir -p -m700 /tmp/claude-1000/t; umask 077; TMPDIR=/tmp/claude-1000/t rtk proxy cargo test --no-fail-fast`: **3870 passed, 1 failed, 24 ignored** (baseline after 30-02: 3827 passed, 1 failed, 24 ignored; +43 tests). The one failure is the expected git-version witness `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`.
- `rtk proxy cargo clippy --all-targets -- -D warnings`: **clean**. `cargo fmt --check`: clean.
- Per-task filters: Task 1 `launcher_marker_ screens::driver_run:: screens::normal:: app::tests::` 278 passed (including `launcher_marker_a_crashed_run_lights_the_badge_and_the_h_filter`); Task 2 `launcher_marker_ screens::normal:: screens::help:: screens::detail:: screens::driver_run:: app::tests::` 696 passed; Task 3 `screens::driver_run:: screens::driver:: render_escape_guard` 186 passed, with `launcher_orphan_sweep_...` and `legacy_orphan_sweep_...` named in the output.
- Acceptance greps: `fn launcher_marker` 14 (>= 1), `launcher_markers` in action.rs 1, `BADGE_CRASHED` 2, `fn needs_attention_for` 1, `interval(` sites 2, `BADGE_STALE` 4, `BADGE_UNKNOWN` 4, `LauncherMarkerScanned` in action.rs 3, `fn schedule_launcher_marker` 1, `fn height_tier` 1, `DRIVER_MEDIUM_MIN_ROWS` 8, `fn status_tokens` 1, `orphan` across driver_run.rs and driver.rs 8 and 1.
- Cfg hygiene (RESEARCH Pitfall 13): the new production code is platform-neutral (`list_driver_runs` already splits its unix probe from the non-unix `LivenessUnknown`); the two lock-dependent app tests are `#[cfg(unix)]`. Reviewed by hand (only the Linux target is installed).

## Known Stubs

None.

## Threat Flags

| Flag | File | Description |
|------|------|-------------|
| threat_flag: resource (event-driven marker scan) | src/ui/screens/mod.rs | `schedule_launcher_marker` lists up to 20 runs (20 capped DIRECTION.md reads and lock probes) per `DriverRun` watcher event for a project whose Driver tab is not on top, with no in-flight guard. A sandboxed driver heartbeats at a low rate and the watcher debounces at 200 ms, and the 20-tick scan still replaces every marker, so the cost is bounded by the debounce, not unbounded; skipped with the experimental flag off. Within T-30-09. |

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Ready for 30-04: the cursor item, `launcher_log.park_rules` and the steering keys sit on top of the tiered composition; MEDIUM draws no cursor-item row, so `a` / `u` on a MEDIUM-height pane act on an item whose detail row is not shown (the footer and refusal copy are 30-04's).
- Ready for 30-05: `s`, `R` and the retired `i` row are untouched; the SUMMARY tier shows no footer change.
- For the phase verifier: the two flaky-under-load tests above, and the human-judgement layout deliverable (D9).

## Self-Check: PASSED

- Created/modified files exist (10 of 10 checked); commit hashes `42ce31f 9378589 c5ea9d3` are ancestors of HEAD; acceptance greps for all three tasks hold; no deletions in any of the three commits (post-commit deletion check empty).

---
*Phase: 30-driver-tab-on-direction-md*
*Completed: 2026-10-21*
