---
phase: 05-state-reader-accuracy
verified: 2026-03-26T23:45:00Z
status: human_needed
score: 11/11 must-haves verified
re_verification:
  previous_status: human_needed
  previous_score: 11/11
  gaps_closed:
    - "Dashboard selected row no longer shows inaccurate expanded status text (expanded_status() removed)"
    - "All dashboard rows uniformly show compact D-R-P-E-V pipeline regardless of selection state"
    - "Detail view phase list now includes a legend line explaining bracket notation and phase icons"
  gaps_remaining: []
  regressions: []
human_verification:
  - test: "Launch TUI with a registered GSD project and verify dashboard pipeline"
    expected: "All rows (selected and unselected) show D-R-P-E-V pipeline with colored letters (green=reached, yellow=current, dark-gray=not reached); no expanded text on selection"
    why_human: "Cannot verify visual pipeline rendering or color mapping without running the TUI"
  - test: "Navigate to a project detail view and check phase list legend"
    expected: "Below 'Phases:' header, a DarkGray legend line reads: '  Legend: + done  * current  o future  [stage] = disk-inferred  (N plans) = plan count'"
    why_human: "Visual rendering check requires running the TUI"
  - test: "Navigate to a project detail view and check phase entries"
    expected: "Each phase line shows disk-inferred status in brackets, e.g. '* P05: State Reader  2/3 plans [Executing 2/3]'"
    why_human: "Phase list rendering with suffix formatting requires visual inspection of a running TUI"
  - test: "Verify completed milestone display on dashboard"
    expected: "Projects where all phases are complete show 'v1.0 Complete' in dimmed gray instead of the pipeline"
    why_human: "Requires running the TUI with a completed milestone project"
---

# Phase 05: State Reader Accuracy Verification Report

**Phase Goal:** Users see correct, trustworthy project state on the dashboard without manual verification
**Verified:** 2026-03-26T23:45:00Z
**Status:** human_needed
**Re-verification:** Yes — third pass, after 05-05 gap closure (UAT feedback)

## Summary

This is the final re-verification after plan 05-05 closed two UAT-reported issues: the `expanded_status()` function produced inaccurate labels (one GSD stage ahead of actual progress) and the detail view lacked any explanation for its bracket notation. Plan 05-05 removed `expanded_status()` entirely and added a DarkGray legend line below the "Phases:" header in both `render()` and `render_main_only()`. The previous score of 11/11 is maintained; the content of Truth 10 changed to reflect the UAT-driven design decision.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|---------|
| 1 | Plan counts include standalone PLAN.md files (not just NN-MM-PLAN.md) | VERIFIED | `roadmap_md.rs:19` regex matches both formats. Tests `test_parse_roadmap_standalone_plan_md` and `test_parse_roadmap_mixed_standalone_and_numbered` pass. |
| 2 | Completed milestones show "v1.0 Complete" instead of "P5: Unknown" | VERIFIED | `state_reader/mod.rs:54-59` guard; `normal.rs:321-332` renders milestone name + " Complete" in DarkGray for `is_milestone_complete`. Three unit tests in `app.rs::tests` cover all cases. |
| 3 | User can run `gsd-manager add /path/to/project` without alias | VERIFIED | `cli.rs:22` has `alias: Option<String>`. `main.rs:43-49` uses `alias.unwrap_or_else(...)`. |
| 4 | InputMode enum deleted and replaced by Screen trait + screen stack | VERIFIED | `grep -rn "InputMode" src/ --include="*.rs"` returns 0 matches. Screen stack confirmed in `app.rs`. |
| 5 | All file I/O in parse_project_state path uses spawn_blocking | VERIFIED | `app.rs:188` calls `tokio::task::spawn_blocking(...)` inside `Action::FileChanged` handler. |
| 6 | Escape pops the screen stack | VERIFIED | `detail.rs:80` — `KeyCode::Esc \| KeyCode::Char('q') => ScreenAction::Pop`. |
| 7 | Modal flows (add project, create project) work via screen pushes | VERIFIED | `normal.rs:98` and `normal.rs:104` push modal screens. All 7 Screen impls confirmed. |
| 8 | User sees phase status inferred from disk artifacts (detail view brackets) | VERIFIED | `disk_suffix()` at `detail.rs:41-75`. Both rendering loops (lines 294, 526) call `disk_suffix(&phase.number, &state.phase_disk_statuses)`. Legend line at `detail.rs:269-272` and `detail.rs:501-504` explains bracket notation. |
| 9 | Dashboard status column shows compact D-R-P-E-V pipeline for all rows | VERIFIED | `compact_pipeline()` at `normal.rs:49`. All rows use single branch at `normal.rs:333-342`: `Some(inference) => compact_pipeline(&inference.status)`. No `is_selected` branching. No `expanded_status` function exists. |
| 10 | All dashboard rows show uniform pipeline (no expanded text on selection) | VERIFIED | `grep -c "expanded_status" src/ui/screens/normal.rs` returns 0. `grep -c "is_selected" src/ui/screens/normal.rs` returns 0. Only `compact_pipeline` used for status cell construction. |
| 11 | Archived milestone phases show as Complete, not NoDirectory | VERIFIED | `disk_status.rs:182-230` — `infer_phase_status()` checks `milestones/` first and returns `DiskStatus::Complete`. Test `test_archived_phase_returns_complete` passes. |

**Score:** 11/11 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/ui/screens/normal.rs` | Compact pipeline for all rows, no expanded_status | VERIFIED | Contains `compact_pipeline()` at line 49; single status-cell branch at lines 333-342 applies to all rows; `expanded_status` function absent (0 occurrences); `DiskInference` import absent; `is_selected` variable absent |
| `src/ui/screens/detail.rs` | Legend line + disk status brackets in phase list | VERIFIED | Legend at lines 269-272 (`render`) and 501-504 (`render_main_only`); `disk_suffix()` at lines 41-75; both rendering loops call `disk_suffix` at lines 294 and 526 |
| `src/ui/project_list.rs` | Deleted (orphaned) | VERIFIED | File does not exist on disk |
| `src/ui/detail_view.rs` | Deleted (orphaned) | VERIFIED | File does not exist on disk |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `src/ui/screens/normal.rs` | `src/state_reader/disk_status.rs` | `use crate::state_reader::disk_status::DiskStatus` | WIRED | Import at line 8; `DiskStatus` used by `compact_pipeline` and `prev_status`. `DiskInference` no longer imported (removed with `expanded_status`). |
| `src/ui/screens/detail.rs` | `src/state_reader/disk_status.rs` | `use crate::state_reader::disk_status::DiskStatus` | WIRED | Import at line 5; `DiskStatus` used in `disk_suffix()` match arms |
| `src/ui/screens/normal.rs` | `ProjectState.current_phase_status` | `state.and_then(\|s\| s.current_phase_status.as_ref())` | WIRED | Single `match` branch at `normal.rs:335` passes inference to `compact_pipeline` |
| `src/ui/screens/detail.rs` | `ProjectState.phase_disk_statuses` | `disk_suffix(&phase.number, &state.phase_disk_statuses)` | WIRED | Called in both rendering loops at lines 294 and 526 |

### Data-Flow Trace (Level 4)

| Artifact | Data Variable | Source | Produces Real Data | Status |
|----------|---------------|--------|--------------------|--------|
| `normal.rs` status_cell | `current_phase_status` (DiskInference) | `ProjectState` populated by `state_reader/mod.rs` via `infer_phase_status()` disk scan | Yes — scans `.planning/phases/` directories for real plan/summary/context/research files | FLOWING |
| `detail.rs` disk suffix | `phase_disk_statuses` HashMap | Same `ProjectState` disk scan, keyed by phase number string | Yes — maps phase numbers to `DiskInference` structs computed from real directory contents | FLOWING |
| `detail.rs` legend line | Static string | Hardcoded `"  Legend: + done  * current  o future  [stage] = disk-inferred  (N plans) = plan count"` | N/A — informational text, not data-driven | STATIC (intentional) |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Build succeeds | `cargo build` | Finished dev profile — 11 warnings, 0 errors | PASS |
| All unit tests pass | `cargo test --lib` | 55 passed; 0 failed; 0 ignored | PASS |
| expanded_status removed from normal.rs | `grep -c "expanded_status" src/ui/screens/normal.rs` | 0 occurrences | PASS |
| is_selected removed from normal.rs | `grep -c "is_selected" src/ui/screens/normal.rs` | 0 occurrences | PASS |
| compact_pipeline present with call site | `grep -c "compact_pipeline" src/ui/screens/normal.rs` | 2 occurrences (definition + 1 call site) | PASS |
| Legend line present in render() | `grep -c "Legend:" src/ui/screens/detail.rs` | 2 occurrences (one per render path) | PASS |
| disk-inferred text present in both legend lines | `grep -c "disk-inferred" src/ui/screens/detail.rs` | 2 occurrences | PASS |
| disk_suffix wired in both render loops | `grep -c "disk_suffix" src/ui/screens/detail.rs` | 3 occurrences (definition + 2 call sites) | PASS |
| Orphaned project_list.rs deleted | `test ! -f src/ui/project_list.rs` | File not found | PASS |
| Orphaned detail_view.rs deleted | `test ! -f src/ui/detail_view.rs` | File not found | PASS |
| No InputMode in compiled files | `grep -rn "InputMode" src/ --include="*.rs"` | 0 matches | PASS |

### Requirements Coverage

| Requirement | Source Plans | Description | Status | Evidence |
|-------------|-------------|-------------|--------|---------|
| STATE-01 | 05-01, 05-04 | User sees accurate plan counts per phase on dashboard (fix regex for standalone PLAN.md) | SATISFIED | `roadmap_md.rs:19` regex matches both `NN-MM-PLAN.md` and `PLAN.md` formats; 2 dedicated tests pass |
| STATE-02 | 05-01, 05-04 | User sees "Complete" instead of "P5: Unknown" when all phases are done | SATISFIED | `state_reader/mod.rs:54-59` guard; `normal.rs:321-332` renders milestone name + " Complete" in DarkGray |
| STATE-03 | 05-03, 05-04, 05-05 | User sees phase status inferred from disk files (discuss/research/plan/execute/verify stages) | SATISFIED | `disk_status.rs` implements full GSD disk inference algorithm; wired into NormalScreen (pipeline) and DetailScreen (brackets + legend) |
| CLI-01 | 05-02, 05-04 | User can register a project by path only — name auto-derived from last folder component | SATISFIED | `cli.rs:22` `alias: Option<String>`; `main.rs:43-49` derives alias from `canonical_path.file_name()` |

No orphaned requirements. All four IDs from REQUIREMENTS.md assigned to Phase 05 are claimed by plans and verified. No other REQUIREMENTS.md IDs are mapped to Phase 05 in the Traceability table.

### Anti-Patterns Found

None. No TODO/FIXME/PLACEHOLDER comments in modified files. No `InputMode` references in any compiled source file. No stub implementations detected. No hardcoded empty state passed to rendering paths.

### Human Verification Required

#### 1. Dashboard pipeline visual rendering (uniform across all rows)

**Test:** Launch `gsd-manager` with at least one registered GSD project that has disk artifacts (phases with `.planning/phases/` directories). Navigate up and down to change selection. Observe the status column for both selected and unselected rows.
**Expected:** Status column on ALL rows (selected or not) shows 5 colored letter-blocks "D R P E V" — green letters for stages reached, yellow letter for the current stage, dark-gray letters for stages not yet reached. Selected row shows the same pipeline format as unselected rows (no expanded text).
**Why human:** Color mapping and row-selection behavior require visual confirmation in a running terminal.

#### 2. Detail view legend line

**Test:** Press Enter on a project to open the detail view. Observe the text immediately below the bold "Phases:" header.
**Expected:** A DarkGray line reads: `  Legend: + done  * current  o future  [stage] = disk-inferred  (N plans) = plan count`
**Why human:** Color rendering (DarkGray) and correct placement relative to the header require visual inspection.

#### 3. Detail view disk status brackets

**Test:** With the detail view open, observe the individual phase entries.
**Expected:** Each phase line reads like `* P05: State Reader  2/3 plans [Executing 2/3]` with a disk-inferred status bracket appended after the plan count.
**Why human:** Phase list rendering with suffix formatting requires visual inspection of a running TUI.

#### 4. Completed milestone display

**Test:** If a project with all phases complete is available, observe its dashboard row.
**Expected:** Status column shows "v1.0 Complete" (or the actual milestone name + " Complete") rendered in dimmed gray, not a pipeline.
**Why human:** Requires a project in the completed state to trigger the milestone-complete branch.

### Gaps Summary

All gaps from previous verification rounds are closed. Plan 05-05 resolved two UAT-reported issues:

**UAT Issue 1 — Inaccurate expanded status text:** The `expanded_status()` function was producing labels that were one GSD stage ahead of actual progress (e.g., reporting "Executing" when the phase was only Planned). Resolution: `expanded_status()` deleted entirely, `is_selected` branching removed, all dashboard rows now use `compact_pipeline()` uniformly. The compact pipeline (D-R-P-E-V letter blocks with color coding) conveys the same information more accurately without the off-by-one label problem.

**UAT Issue 2 — No explanation for bracket notation:** The detail view showed disk-inferred status brackets like `[Executing 2/3]` without any in-UI explanation. Resolution: DarkGray legend line added immediately after the "Phases:" header in both `render()` and `render_main_only()` code paths. Legend text: `+ done  * current  o future  [stage] = disk-inferred  (N plans) = plan count`.

Build: 0 errors, 11 warnings (pre-existing, unrelated to phase 05 changes). Tests: 55/55 passing.

---

_Verified: 2026-03-26T23:45:00Z_
_Verifier: Claude (gsd-verifier)_
_Re-verification after: 05-05-SUMMARY.md gap closure (UAT feedback round)_
