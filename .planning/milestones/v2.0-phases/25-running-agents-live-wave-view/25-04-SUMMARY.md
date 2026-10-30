---
phase: 25-running-agents-live-wave-view
plan: 04
subsystem: app-dashboard
tags: [agents-scan, tick-wiring, in-flight-gate, status-cell, ladder, dashboard]

requires:
  - phase: 25-01
    provides: "scan_projects_guarded, ProjectAgents, AgentRow, AgentLiveness"
  - phase: 25-03
    provides: "waves::derive, AgentView, AgentView::summary_forms, PlanRef"
provides:
  - "Action::AgentsScanned { per_project }"
  - "AppContext.agent_views (wholesale-replaced sibling map) and AppContext.agents_scan_in_flight"
  - "Agents scan on the 20-tick session-poll counter inside spawn_blocking, gated by the in-flight flag"
  - "AgentsScanned handler: registered-alias filter, no-rows skip, derive per alias, equality-guarded redraw, counts-only log"
  - "normal.rs: status_column_cells, agent_summary_line, DASHBOARD_HIGHLIGHT_SYMBOL, DASHBOARD_STATUS_COLUMN; the Status cell shows the widest fitting form"
affects: [25-05, 25-06]

actuals:
  tokens: 9800
  tasks: 3
  commits: 3
plan_head_before: 8a960a7044fe202966b022559ef317ff76daaa8b

tech-stack:
  added: []
  patterns:
    - "Column-fit checks re-run the Table's own Layout over dashboard_columns, never a literal width"
    - "Agent state is a wholesale-replaced sibling map on AppContext; the replacement is its prune"

key-files:
  created: []
  modified:
    - src/action.rs
    - src/app.rs
    - src/ui/screens/mod.rs
    - src/ui/screens/normal.rs
    - src/ui/screens/detail.rs
    - src/ui/screens/delete_confirm.rs
    - src/ui/screens/driver_confirm.rs

key-decisions:
  - "agent_views is classified in the exhaustive every_per_alias_driver_map_is_pruned destructure as alias-keyed but not pruned, because the per-scan wholesale replacement plus the registered-alias filter is its prune (as the plan's field doc states) [inferred]"
  - "The \"> \" highlight symbol became the DASHBOARD_HIGHLIGHT_SYMBOL constant, shared by dashboard_table and status_column_cells so the two cannot drift [inferred]"
  - "The in-flight flag is set only when event_tx is Some, so a test App without a sender never sticks the flag [inferred]"
  - "U+00B7 is one cell under unicode-width but East-Asian-Ambiguous; the 13-cell common-case forms contain no middle dot [inferred — audit, carried from plan]"

patterns-established:
  - "Status-cell render tests measure the column from the rendered Status/Progress header positions, independent of status_column_cells"

requirements-completed: [AGENT-05]

coverage:
  - id: D1
    description: "The agents scan rides the existing 20-tick counter inside spawn_blocking, with an in-flight gate that never overlaps and never sticks"
    requirement: AGENT-05
    verification:
      - kind: unit
        ref: "src/app.rs#the_agents_scan_rides_the_session_poll_counter"
        status: pass
      - kind: unit
        ref: "src/app.rs#no_second_agents_scan_spawns_while_one_is_in_flight"
        status: pass
      - kind: unit
        ref: "src/app.rs#the_agents_scan_handler_clears_the_in_flight_flag_for_an_empty_result"
        status: pass
      - kind: integration
        ref: "tests/async_blocking_guard.rs"
        status: pass
    human_judgment: false
  - id: D2
    description: "The AgentsScanned handler derives views for registered aliases with agents only, replaces the map wholesale and redraws only on change"
    requirement: AGENT-05
    verification:
      - kind: unit
        ref: "src/app.rs#an_unchanged_agents_scan_does_not_request_a_redraw"
        status: pass
      - kind: unit
        ref: "src/app.rs#an_agents_scan_for_an_unregistered_alias_is_dropped"
        status: pass
      - kind: unit
        ref: "src/app.rs#a_project_with_no_agent_rows_gets_no_view"
        status: pass
    human_judgment: false
  - id: D3
    description: "A live wave's summary appears in its dashboard Status cell as the widest whole form that fits the measured column, at every width; rows without agents are unchanged"
    requirement: AGENT-05
    verification:
      - kind: unit
        ref: "src/app.rs#an_agents_scan_reaches_the_dashboard_status_cell"
        status: pass
      - kind: unit
        ref: "src/ui/screens/normal.rs#status_column_cells_matches_the_rendered_status_column"
        status: pass
      - kind: unit
        ref: "src/ui/screens/normal.rs#the_agent_summary_is_a_whole_ladder_form_at_every_width"
        status: pass
      - kind: unit
        ref: "src/ui/screens/normal.rs#a_form_exactly_as_wide_as_the_column_is_chosen"
        status: pass
      - kind: unit
        ref: "src/ui/screens/normal.rs#a_row_without_active_agents_is_byte_identical"
        status: pass
      - kind: unit
        ref: "src/ui/screens/normal.rs#the_stalled_form_replaces_the_status_cell"
        status: pass
      - kind: unit
        ref: "src/ui/screens/normal.rs#the_agent_summary_replaces_a_milestone_complete_cell"
        status: pass
      - kind: unit
        ref: "src/ui/screens/normal.rs#the_middle_dot_is_one_cell"
        status: pass
    human_judgment: false

duration: 8min
completed: 2026-09-26
status: complete
---

# Phase 25 Plan 04: App and Dashboard Wiring Summary

**The agents scan now rides the existing 5 s session-poll tick inside `spawn_blocking`, behind an in-flight flag. Its whole result arrives as `Action::AgentsScanned`, and the handler derives one `AgentView` per registered alias that has agents, replacing `AppContext.agent_views` wholesale. The dashboard Status cell then shows the widest `summary_forms()` entry that fits the measured Status column. That is `w2/11 13run` at 80 and 120 columns for the 13-executor wave.**

## Performance

- **Duration:** 8 min
- **Started:** 2026-09-26T03:15:16Z
- **Completed:** 2026-09-26T03:23:23Z
- **Tasks:** 3 (1 tracer, 2 TDD)
- **Files modified:** 7

## Accomplishments

- **`Action::AgentsScanned { per_project }`.** Its doc follows the `RunsReconciled` structure: whole scan not delta, plain data so `Action` stays `Clone`, and no `Box` needed.
- **`AppContext.agent_views` and `agents_scan_in_flight`.** Both carry sibling-map docs and are added to all six `AppContext { .. }` literals. `agent_views` is also classified in the exhaustive prune-coverage destructure test.
- **Tick wiring (D-C01, D-B04).** The scan runs inside the `session_poll_counter >= 20` block after the reconciliation probe, with no timer or `notify` watcher of its own. The flag is set before the spawn. The closure wraps `scan_projects_guarded` in `catch_unwind`, maps a panic to an empty Vec, and always sends.
- **Handler.** It does the following, in order:
  - Clears the flag first, unconditionally.
  - Skips unregistered aliases and projects with no rows and no worktree-less agents.
  - Runs `waves::derive` against the parsed `ProjectState` (or the default).
  - Replaces the map and redraws only when it changed.
  - Logs `tracing::debug!` with counts only (D-C13).
- **Status cell (D-C14).**
  - `status_column_cells(w)` re-runs `Layout::horizontal(dashboard_columns(w).1).flex(Flex::Start).spacing(1)` over the inner width minus the `"> "` highlight.
  - `agent_summary_line` picks the first form whose `Line::width()` fits. Cyan is the default; a `…stalled` form is yellow.
  - The existing milestone/pipeline/status expression is wrapped unchanged, and the badges are untouched.

## Task Commits

1. **Task 1 (tracer): tick → scan → AgentsScanned → derive → Status cell:** `35d9aac` (feat). Tracer gate (interactive, end-of-phase, automated-only verify): the tracer test, `ui::screens` (381 passed) and `async_blocking_guard` (9 passed) re-ran green, then the plan expanded.
2. **Task 2: the Status-cell ladder at every width:** `8d784bd` (test)
3. **Task 3: scan cadence and cache robustness:** `c22b953` (test)

## Files Created/Modified

- `src/action.rs`: `Action::AgentsScanned`
- `src/app.rs`: tick block, handler, prune-destructure classification, 7 tests plus the `wave_state`, `live_executor_scan` and `next_agents_scan` helpers
- `src/ui/screens/mod.rs`: the two `AppContext` fields with docs; the test literal
- `src/ui/screens/normal.rs`: `status_column_cells`, `agent_summary_line`, two constants, the Status-cell wrap, the `render_dashboard_interior_selecting` harness variant and 7 tests
- `src/ui/screens/{detail,delete_confirm,driver_confirm}.rs`: the two fields in their test literals

## Test Results

- `cargo test --lib app::tests`: 76 passed, including all 7 new tests.
- `cargo test --lib ui::screens::normal`: 52 passed, including all 7 named tests.
- `cargo test --lib ui::screens`: green.
- `cargo test --test async_blocking_guard --no-fail-fast`: 9 passed.
- `cargo clippy -- -D warnings`: clean.
- `cargo test --no-fail-fast`: 52 `test result:` lines, 2453 passed, 1 failed, 15 ignored.
  - The one failure is the known local git-version witness (`envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`).
  - 2453 = 2439 + 14 new.
- Acceptance greps:
  - `git grep -c 'agents_scan_in_flight: false' -- src` finds 6 sites.
  - `scan_projects_guarded` appears in `src/app.rs` exactly once, a non-test call inside the 20-tick block.
  - `fn status_column_cells` appears once, and its body has no literal 13.

## TDD Gate Compliance

- **Order:** Tasks 2 and 3 are `tdd="true"`, but their behaviour was implemented by the Task 1 tracer, as the plan's tracer shape requires. Each committed as `test(25-04)` after the `feat(25-04)` tracer commit, and no `feat` commit follows either one. So the git-log gate check sees GREEN (`35d9aac`) before RED (`8d784bd`, `c22b953`). This is the same tracer-first pattern 25-01, 25-02 and 25-03 recorded.
- **Unexpected-GREEN investigation (Fail-Fast Rule 1).** Every new test passed at RED time. To show the tests are not vacuous, each suite was run against a temporary mutation, then restored:
  - **Task 2 mutation:** no highlight subtraction, and `<` instead of `<=`. It failed 3 tests: `status_column_cells_matches_the_rendered_status_column`, `the_agent_summary_is_a_whole_ladder_form_at_every_width` and `a_form_exactly_as_wide_as_the_column_is_chosen`.
  - **Task 3 mutation:** no in-flight gate, no alias filter, no equality guard, and no flag clear. It failed 5 of the 6 Task 3 tests.
- **`gsd-tools check tdd-red-evidence`:** not run. It parses node TAP output only, and `workflow.tdd_mode` is not enabled, so the gate is advisory. [inferred: proceeded on cargo's output]

## Decisions Made

See `key-decisions` in the frontmatter. Every entry is marked [inferred] for audit.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Classified `agent_views` and `agents_scan_in_flight` in the exhaustive `AppContext` destructure**
- **Found during:** Task 1
- **Issue:** `every_per_alias_driver_map_is_pruned` destructures `AppContext` exhaustively, so it stops compiling when a field is added.
- **Fix:** both fields went into the "alias-keyed but deliberately NOT pruned" / scalar groups, with the reason stated (wholesale replacement per scan, plus the registered-alias filter).
- **Files modified:** src/app.rs
- **Commit:** 35d9aac

**2. [Test harness] Unique alias in the new normal.rs render tests**
- `"proj"` matched more than one rendered row (footer text), so `row_with` refused it. The new tests use `"orbit"` instead. Production code was not affected.
- **Commit:** 8d784bd

**3. [Convention] `DASHBOARD_HIGHLIGHT_SYMBOL` constant**
- `"> "` is now one constant, read by both `dashboard_table` and `status_column_cells`. Behavior is unchanged.
- **Commit:** 35d9aac

---

**Total deviations:** 1 blocking fix, 1 test-harness adjustment, 1 behavior-neutral constant. **Impact:** none on the `<interfaces>` contract.

## Issues Encountered

None.

## Known Stubs

None.

## Threat Flags

None. The threat mitigations are covered as follows:
- **T-25-18:** the in-flight gate and the always-send `catch_unwind` closure, pinned by `no_second_agents_scan_spawns_while_one_is_in_flight` and `the_agents_scan_handler_clears_the_in_flight_flag_for_an_empty_result`.
- **T-25-19:** the scan runs only inside `spawn_blocking`, and `async_blocking_guard` is green.
- **T-25-20:** the handler logs counts only.
- **T-25-21:** wholesale replacement plus the registered-alias filter, pinned by `an_agents_scan_for_an_unregistered_alias_is_dropped`.

No new surface.

## User Setup Required

None.

## Next Phase Readiness

- **25-05:** can read `ctx.agent_views.get(alias)` in the detail view's Agents sub-tab. `agent_summary_line` and `status_column_cells` are module-private to `normal.rs`; widen them to `pub(crate)` if 25-05 needs them.
- **25-06:** can add `AgentView.fixers`. The normal.rs tests build views as `..Default::default()` literals, so a new field does not break them.

## Self-Check: PASSED

- All seven modified files exist.
- Commits 35d9aac, 8d784bd and c22b953 are present.
- `git rev-list --count 8a960a7..HEAD` = 3.

---
*Phase: 25-running-agents-live-wave-view*
*Completed: 2026-09-26*
