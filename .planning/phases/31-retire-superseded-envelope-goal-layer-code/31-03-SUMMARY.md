---
phase: 31-retire-superseded-envelope-goal-layer-code
plan: 03
subsystem: retirement
tags: [rust, ratatui, tui, retirement, reconcile, watcher, launcher-markers]

requires:
  - phase: 31-retire-superseded-envelope-goal-layer-code
    provides: 31-02 retired the injection UI and the dashboard start/stop/opt-in keys, so the legacy runs half could go next
provides:
  - a 20-tick scan that rescans launcher markers only (Action::LauncherMarkersRescanned), with no startup or tick reconcile
  - a Driver tab that lists launcher runs only (no legacy section, no L toggle, no legacy renderer)
  - a ProjectViewCache / AppContext without legacy run, dry-run, journal and cursor state
  - journal::classify_change that treats everything at or below .planning/meta-manager/runs as inert
affects: [31-04, 31-05, 31-06]

actuals:
  tokens: 11650
  tasks: 3
  commits: 3

plan_head_before: 38ef97789ad605e064febda6ae0c64cfbd7420f8
plan_head_after: 9d22292aa44e3e3aabc86d6d6dbf2b20d3e27cd5
commits: 3

tech-stack:
  added: []
  patterns:
    - "delete the reader first, let cargo check --all-targets list what is now written but never read, then delete that (pub items need a manual grep)"
    - "an item-removal helper that strips string literals before counting braces; a naive brace count silently over-deletes consts that contain \\u{..}"

key-files:
  created: []
  modified:
    - src/action.rs
    - src/app.rs
    - src/main.rs
    - src/journal/mod.rs
    - src/watcher.rs
    - src/ui/mod.rs
    - src/ui/screens/mod.rs
    - src/ui/screens/normal.rs
    - src/ui/screens/delete_confirm.rs
    - src/ui/screens/detail.rs
    - src/ui/screens/driver.rs
    - src/ui/screens/driver_run.rs
    - src/ui/screens/driver_confirm.rs
    - src/ui/screens/help.rs
    - src/ui/screens/render_escape_guard.rs
  deleted: []

key-decisions:
  - "Action renamed RunsReconciled -> LauncherMarkersRescanned { launcher_markers }, not folded into the existing singular LauncherMarkerScanned, which is a different message [inferred - audit]"
  - "needs_human(state, parked) lost both its run and its last_outcome inputs: nothing can observe a legacy run or its outcome any more [inferred - audit]"
  - "attention rank 1 and the driven badge read the launcher marker's Driven class; the legacy observed-run input is gone, so rank 1 would otherwise be dead [inferred - audit]"
  - "delete_confirm's live-legacy-run refusal is deleted with its tests (T-31-09 accepted); launcher runs never had one and adding one is out of scope [inferred - audit]"
  - "classify_change maps runs/** at >= 3 components to Ignored, so runs/.gitignore, runs/active and runs/run.lock are inert too; .planning/runs/x stays Planning [inferred - audit]"
  - "TerminalState (enum + from_label) stays in ui/screens/driver.rs because driver::run's label-vocabulary test still reads it; from_outcome, observed, the Live / LivenessUnknown / CrashedWithoutEnding variants and terminal_state_cell went with the renderer [inferred - audit]"
  - "driver_elapsed_redraw_wanted and its tick gate are deleted whole rather than losing the ObservedRun argument: with no input it could only answer false [inferred - audit]"

patterns-established:
  - "a retired key's regression test presses it and asserts ScreenAction::None, no status line and no dispatch (launcher_view_l_is_retired_and_does_nothing)"

requirements-completed: []  # RETIRE-01 and RETIRE-02 are shared with 31-01, 31-02 and 31-04..31-07; the shared-ID gate (#2388) keeps them open until every declaring plan has a SUMMARY

coverage:
  - id: D1
    description: "The dashboard badge, the h filter and attention rank still follow launcher markers, refreshed by the existing 20-tick scan with no legacy reconcile at startup or on the tick"
    requirement: RETIRE-02
    verification:
      - kind: unit
        ref: "src/app.rs#launcher_marker_a_crashed_run_lights_the_badge_and_the_h_filter"
        status: pass
      - kind: unit
        ref: "src/app.rs#launcher_marker_the_slow_tick_scans_only_with_the_flag_on"
        status: pass
      - kind: unit
        ref: "src/app.rs#a_marker_rescan_replaces_the_marker_rather_than_merging_it"
        status: pass
    human_judgment: false
  - id: D2
    description: "The Driver tab lists launcher runs only: no legacy section, header, toggle key, footer token, help row or probe state remains, and every launcher view, steering key and kill confirm renders as before"
    requirement: RETIRE-01
    verification:
      - kind: unit
        ref: "src/ui/screens/detail.rs#launcher_view_l_is_retired_and_does_nothing"
        status: pass
      - kind: unit
        ref: "src/ui/screens/detail.rs#launcher_view_renders_the_launcher_rows_and_no_legacy_section"
        status: pass
      - kind: unit
        ref: "src/ui/screens/help.rs#the_help_names_no_legacy_runs_section_and_no_toggle_key"
        status: pass
      - kind: unit
        ref: "rtk proxy cargo test --lib -- launcher_marker_ launcher_view_ launcher_stop_ classify_change screens::driver_run:: screens::driver:: screens::detail:: screens::normal:: render_escape_guard (690 pass)"
        status: pass
    human_judgment: false
  - id: D3
    description: "Legacy run dirs left on disk are inert to the watcher at every depth, and a project's own .planning/runs/ and .planning/drivers/ keep re-parsing"
    requirement: RETIRE-01
    verification:
      - kind: unit
        ref: "src/journal/mod.rs#classify_change_ignores_every_legacy_runs_path"
        status: pass
      - kind: unit
        ref: "src/journal/mod.rs#classify_change_keeps_a_project_owned_drivers_dir_as_planning"
        status: pass
      - kind: unit
        ref: "src/watcher.rs#a_legacy_runs_path_emits_no_event"
        status: pass
    human_judgment: false

duration: 32min
completed: 2026-10-25
status: complete
---

# Phase 31 Plan 03: Retire the Legacy Runs Half of the TUI Summary

**The 20-tick scan now carries launcher markers only (`Action::LauncherMarkersRescanned`), the Driver tab lists launcher runs and nothing else, and every feed behind the old "legacy runs" section (run-list scan, dry-run preview, journal tail, cursors) is gone; legacy run dirs on disk are left alone and `classify_change` ignores them at any depth.**

## Performance

- **Duration:** about 32 min (2026-10-25T05:10Z to 05:42Z)
- **Tasks:** 3 (1 tracer, 2 auto)
- **Files:** 20 changed over the three code commits, +777 / -6,270 lines (`git diff --shortstat` from `plan_head_before`)

## Phase baseline and post-plan suite

Before (from the 31-02 SUMMARY, HEAD `38ef977`):

```
results=94 passed=4359 failed=1 ignored=30
```

After (FULLSUITE, HEAD `9d22292`):

```
results=94 passed=4300 failed=1 ignored=30
failing: envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against
expected results=94
```

The lone failure is the known git-version witness (31-05 deletes it). `editor_race_smoke` passed. `results=94` equals `expected results=94`, counted from the raw log (`grep -c '^test result:'` = 94). -59 passed is the net of the legacy tests that went with their subjects (injection-era journal/tally/ring tests in `app.rs`, legacy-row tests in `driver_run.rs` / `driver.rs` / `detail.rs`, the `delete_confirm` refusal tests, the outcome-badge tests, the dry-run preview tests) against the retargeted and new ones. `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` exit 0. The plan-level targeted verification (`launcher_marker_ launcher_view_ launcher_stop_ classify_change screens::driver_run:: screens::driver:: screens::detail:: screens::normal:: render_escape_guard`) passes, 690 tests.

## Census pins: old and new

Re-measured in the Task 2 commit after the legacy renderer left `driver.rs`. Every pin came out **unchanged**, and the reason is recorded next to each:

| Pin | Old | New | Why |
|---|---|---|---|
| `ui::MEASURED_REACH` | driver.rs 1, driver_confirm.rs 2, render_escape_guard.rs 3 | same | the legacy renderer carried no needle call; `shown_capped`'s own is the one left in `driver.rs` |
| `ui::UI_SOURCE_FLOOR` / files under `src/ui/` | 16 / 22 | same | this plan deletes no file |
| `driver::CENSUS_FILES` totals (units, occurrences, whole-file) | (2, 3, 4) | (2, 3, 4) | same reason; a comment now says it was re-measured |
| `render_escape_guard` states | included `Driver tab, legacy runs` and `Driver tab, dry-run preview` | both removed (description rows, arrange fns, the `the_two_new_driver_states_reach_their_branches` test, and the `driver_runs` / `driver_output` / `driver_journal` fixtures in `probe_ctx`) | their render sites are gone; every surviving screen keeps its identity fixture |

## Delete-guard finding (T-31-09)

`delete_confirm::do_remove_project` used to refuse to unregister a project whose `ctx.observed_runs` entry was not `Dead` ("press x to stop it first"). After the C7 rewire nothing populates `observed_runs`, so the refusal had no input; it is deleted with its three tests, and the kept control test is now `unregistering_a_project_removes_it_with_no_refusal`. A launcher run never had this refusal, and adding one would be a new capability, so none was added (T-31-09 accepted as planned). `[inferred - audit]`

## Operator note: a legacy `drive` process still running at upgrade

A legacy `drive` process that is still running when this build is installed is **invisible to the TUI**: nothing reconciles, lists or tails `.planning/meta-manager/runs/` any more, there is no dashboard badge for it, and the TUI cannot stop it. The operator must kill it from a shell. Its run dir stays on disk, untouched and inert (D-03).

## Accomplishments

- **C7 rewire.** The 20-tick block keeps its counter, its single `spawn_blocking`, its experimental short-circuit and the comment forbidding a second timer. It computes `launcher_markers_for(..)` and sends `LauncherMarkersRescanned`; the handler applies it through `apply_launcher_markers`, recomputes the filtered aliases and redraws. The startup `reconcile_all` / `last_ended_outcomes` calls in `main.rs` are gone, and so are `AppContext::{observed_runs, last_outcomes}`, `driver_elapsed_redraw_wanted`, the legacy `driven_and_live` badge input and `needs_human`'s outcome arm.
- **D-03 UI.** `Selection` is `Launcher | Nothing`; `has_legacy`, `toggle_legacy`, the legacy header and rows, `LEGACY_READ_ONLY`, `SteerTarget::Legacy`, the `L` key and `[L]egacy` footer token, and the three help rows are removed. `driver.rs` shrank by about a thousand lines to the shared helpers Phase 30 imports (`driver_panes`, `follow_indicator`, `label_style`, `muted_style`, `pane_header_line`, `ring_overflow_notice`, `shown_capped`, `DriverTabRegions`, `DRIVER_PANE_TITLE`, `DRY_RUN_LOADING`). `render_launcher_tab` no longer takes the context.
- **Feeds.** `schedule_run_list_scan`, `schedule_dry_run_report` (the last non-executor production use of `DrivableProject::from_registry`; the type stays for 31-07), `schedule_journal_tail`, the three `Action`s, `AppContext::{journal_cursors, driver_output, session_spawned_runs}`, the legacy `ProjectViewCache` fields, `DriverRunJournal`, `DryRunPreview`, `DriverRunTally` and the journal-record projection helpers are deleted. Each paired call site kept its `schedule_launcher_refresh`.
- **Inert legacy dirs.** `classify_change` returns `Ignored` for any path at or below `.planning/meta-manager/runs` (`>= 3` components); `ChangeKind::DriverJournal` no longer exists. The `Ignored` doc and the `classify_change` doc say why and that dirs are left on disk.

## Task Commits

1. **Task 1 (tracer): the slow tick rescans launcher markers only** - `f5c06fb` (refactor)
2. **Task 2: the Driver tab lists launcher runs only** - `77908eb` (refactor)
3. **Task 3: delete the feeds; legacy runs/** becomes inert** - `9d22292` (refactor)

Tracer gate: `HUMAN_VERIFY_MODE=end-of-phase` and the tracer `<verify>` is automated-only, so it was re-run before expansion (240 tests in the `launcher_marker_ app::tests:: screens::normal:: screens::delete_confirm::` filter pass; both named `launcher_marker_` tests present; clippy clean). Tracer verified end-to-end, expanding.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `src/watcher.rs` tests used the retired path as their "driver path"**
- **Found during:** Task 3 verify (`watcher::tests::*` failed three times)
- **Issue:** the fold tests built their driver-write example from `meta-manager/runs/<id>/journal.jsonl`, which is now `Ignored`, so the dedup assertions saw no event. The file was not in the plan's list.
- **Fix:** the three tests now use the launcher `meta-manager/drivers/<run>/DIRECTION.md` / `LOG.md` paths (class `DriverRun`), keeping every dedup assertion; `a_legacy_runs_path_emits_no_event` pins the new behaviour at every depth.
- **Files modified:** src/watcher.rs
- **Commit:** `9d22292`

**2. [Rule 3 - Dead code cargo cannot see] Orphans of the deleted feeds**
- **Found during:** all three tasks (`pub` items raise no `dead_code`; found by grep)
- **Removed:** `AppContext::driver_output` and its prune pass (its only writer was the journal-append handler), `DriverRunTally`, `DriverLineKind::Injection` (31-02 left it for this plan), `driver_line_for_record`, `update_driver_tally`, `journal_gap_line`, `driver_elapsed_redraw_due`, the second prune pass that kept the newest `RETAIN_RUNS` cursors, `AppContext::session_spawned_runs` (plan Task 2 step 4), and `TerminalState::{from_outcome, observed}` with the three verdict-only variants.
- **Commits:** `f5c06fb`, `77908eb`, `9d22292`

**3. [Rule 2 - Honest text] Help copy named a deleted key**
- **Found during:** Task 2 (the plan's prohibition: no user-facing text may name the legacy section or its toggle)
- **Fix:** the three `Show or hide legacy runs (read-only)` help rows are removed; `the_help_names_no_legacy_runs_section_and_no_toggle_key` pins it with the flag on and off.
- **Files modified:** src/ui/screens/help.rs
- **Commit:** `77908eb`

**4. [Rule 3 - Process] A scripted edit over-deleted in `app.rs`**
- **Found during:** Task 3, immediately after the edit (the file stopped compiling its handlers)
- **Issue:** a helper that located a comment block with `rfind("\n\n")` cut a large span of unrelated handlers.
- **Fix:** `git checkout -- src/app.rs` (the file was uncommitted since `77908eb`, so nothing else was lost) and the edits were redone with the block bounded by its comment lines. No effect on the committed history.

Small comment-only edits outside the plan's file list (`src/registry.rs`, `src/journal/reader.rs`, `src/journal/writer.rs`, `src/driver/run.rs`, `src/driver/reconcile.rs`) re-word references to deleted symbols so the acceptance greps hold; no code changed there.

**Total deviations:** 3 auto-fixed (1 Rule 2, 2 Rule 3), 1 process note. **Impact:** no scope growth; every item is a leftover of the deleted surfaces.

## Decisions Made (inferred, for audit)

- `LauncherMarkersRescanned` rather than reusing `LauncherMarkerScanned` (a different, per-project message). `[inferred - audit]`
- `needs_human(state, parked)`: the finished-run-outcome source (a legacy `drive` run ending PermissionDenied / Failed / Stalled / TimedOut) can no longer be observed, so the badge, `h` filter and rank 0 now come from paused / external-job evidence and launcher markers only. `[inferred - audit]`
- Attention rank 1 is `experimental && launcher marker class == Driven`. Before this plan a launcher-driven project got the driven badge but did not float to rank 1 (only the legacy observed run did). This is a small behaviour change that keeps rank 1 meaningful. `[inferred - audit]`
- `TerminalState` kept (enum + `from_label`) for `driver::run`'s test `every_label_the_spawn_failure_mapping_emits_is_one_the_render_layer_reads`. 31-04 should delete it together with that test. `[inferred - audit]`
- `L` is unbound rather than rebound: pressing it on the Driver tab does nothing. `[inferred - audit]`

## Issues Encountered

None beyond the deviations above.

## Known Stubs

None.

## Threat Flags

None. The plan added no network endpoint, auth path or file-access surface. T-31-07 (leftover legacy dirs reaching the watcher) is mitigated by `classify_change_ignores_every_legacy_runs_path`, `a_legacy_runs_path_emits_no_event` and the `.planning/runs/x` twin; T-31-08 (marker integrity) by the two retargeted `launcher_marker_` tests, which keep their badge and filter assertions; T-31-09 is accepted as described above.

## Left for later plans (recorded, not done)

- **31-04:** `driver::reconcile`, `driver::dry_run`, `journal::{list_runs, inbox, reader::read_all, ...}` and `crate::registry::current_prompt_inputs` are now callerless `pub` items. `ui::screens::driver::TerminalState` goes with `driver::run`'s label test. `crate::ui::glyphs::GUILLEMET_RIGHT` ("Injection marker") has no production user left.
- **31-07 (held):** `DrivableProject`, the Codex executor and the in-process Claude transport are untouched; `AppContext::run_states` and its `ExecutionEvent` handler stay.

## Next Phase Readiness

Ready for 31-04: nothing under `src/ui/`, `src/app.rs` or `src/main.rs` references `driver::reconcile`, the legacy journal lister or the dry-run builder (`grep 'reconcile::'` outside `reconcile.rs` and comment lines = 0), so those modules can be deleted without orphaning a caller. The git-version witness stays red locally until 31-05.

## Self-Check: PASSED

- Commits `f5c06fb`, `77908eb`, `9d22292` are ancestors of HEAD; `git rev-list --count` from the recorded `plan_head_before` is 3
- Task 1 acceptance: `RunsReconciled` in src 0 (baseline 16); `LauncherMarkersRescanned` in app.rs 8; `reconcile_all|last_ended_outcomes` in main.rs / app.rs 0 / 0; `observed_runs` in ui/screens/mod.rs 0 (baseline 15); `driven_and_live` in normal.rs 0
- Task 2 acceptance: `has_legacy|legacy_row|Selection::Legacy` in driver_run.rs 0 (baseline 42); `RunVerdict` in driver_run.rs 0; legacy renderer fns in driver.rs 0; kept helper count 6; `read-only here` under src/ui 0 (baseline 7)
- Task 3 acceptance: `DriverJournal` in journal/mod.rs 0 (baseline 3); the three schedulers in src 0; the three Actions in action.rs 0; `journal_cursors` in src 0; `reconcile::` outside reconcile.rs and comments 0 (baseline 20); `classify_change_ignores_every_legacy_runs_path` present and passing
- Plan verification: targeted filter 690 pass; clippy and fmt clean; FULLSUITE `results=94` equals expected, only the known witness fails

---
*Phase: 31-retire-superseded-envelope-goal-layer-code*
*Completed: 2026-10-25*
