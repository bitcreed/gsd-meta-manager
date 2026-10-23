---
phase: quick-261023-05l
verified: 2026-10-23T00:00:00Z
status: passed
score: 7/7 must-haves verified
covered_files:
  - .planning/quick/261023-05l-workstream-with-no-roadmap-phases-gets-a-minimal-band-on-the/261023-05l-PLAN.md
  - .planning/quick/261023-05l-workstream-with-no-roadmap-phases-gets-a-minimal-band-on-the/261023-05l-SUMMARY.md
  - src/state_reader/workstreams.rs
  - src/ui/roadmap_graph.rs
  - src/ui/roadmap_view.rs
  - src/ui/screens/detail.rs
covered_digest: "v3:sha256:5daeee9a55348b324a289f4d19af9c076557a7055bc0a691e1491cc17cc00476"
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: passed
  previous_score: 7/7
  gaps_closed: []
  gaps_remaining: []
  regressions: []
---

# Quick 261023-05l: Workstream with no roadmap phases gets a minimal band - Verification

**Goal:** a workstream whose roadmap lists no phases draws a minimal band (header plus a dim placeholder with its status) on the Roadmap tab and the Phases tab, foldable, without breaking cursor or selection indexing, with regression tests.
**Status:** passed. **Re-verification:** Yes. The earlier report went stale only because quick 261023-05m touched `src/ui/screens/detail.rs` and `src/state_reader/workstreams.rs` afterwards. HEAD is 981da0e.

## What 05m changed

`git show 7d97d56` touched two files. In `detail.rs` it changed doc comments and one existing test (`sentriq_roadmap_model_has_a_scoped_band_per_workstream_and_marks_the_focus_phase`). In `workstreams.rs` it changed the `focus_phase_number` fallback. None of the empty-workstream code paths (`is_empty_workstream`, `Slot::NoPhases`, `ListRow::NoPhases`, `target_of`, `empty_workstream_line`, `render_pipeline_tab` placeholder lines) were edited. The shared fixture writer was not changed.

## Observable Truths (re-checked on HEAD 981da0e)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Roadmap draws `ws vX.Y — Name` header (no `0/0`) plus one dim `no phases on roadmap yet · <status>` row at 80/100/120/200 cols | VERIFIED | `is_empty_workstream` (roadmap_graph.rs:536) drives `row_sequence` to push `Slot::Band` + `Slot::NoPhases` (:1204-1206). Passing tests: `sentriq_empty_workstream_roadmap_draws_its_band_and_placeholder`, `an_empty_workstream_band_draws_its_header_and_one_placeholder_row`, and the roadmap_view tests `an_empty_workstream_band_row_has_no_count_and_its_placeholder_is_dim`, `..._without_a_status_has_no_separator`, `an_empty_workstream_band_pane_says_no_phases_on_roadmap_yet`. |
| 2 | Empty band folds like any band; only the placeholder hides; one toggle stored | VERIFIED | `Slot::NoPhases(b) => !phases_hidden(b)` (roadmap_graph.rs:1012). Passing tests: `folding_an_empty_workstream_band_hides_only_its_placeholder`, `sentriq_empty_workstream_roadmap_j_steps_over_the_placeholder_and_space_folds_it`. |
| 3 | Placeholder is never a cursor target; phase keys unchanged | VERIFIED | `ListRow::NoPhases { .. } => None` in `target_of` (:1304). Passing tests: `an_empty_workstream_placeholder_is_never_a_cursor_target`, `sentriq_empty_workstream_roadmap_model_keeps_every_phase_key`. |
| 4 | The project's OWN empty non-shipped band still draws nothing | VERIFIED | `is_empty_workstream` requires a scope. Passing test: `an_empty_workstream_rule_never_draws_the_projects_own_empty_band`. |
| 5 | Phases tab draws a placeholder line per empty workstream outside List and ListRegion; selection and clicks unchanged | VERIFIED | `render_pipeline_tab` builds `empty_workstreams` (detail.rs:7533-7542) and draws them outside the list rect (:7635-7658). Passing tests: `sentriq_empty_workstream_phases_tab_draws_a_placeholder_line`, `sentriq_empty_workstream_phases_selection_and_clicks_are_unchanged`. |
| 6 | Workstream status and strings escaped on both tabs | VERIFIED | `empty_workstream_line` (detail.rs:9219) routes text through `shown()`. Passing test: `empty_workstream_line_escapes_third_party_text`. Roadmap-side escaping is covered by the graph tests. |
| 7 | Gate: tests pass, clippy 0 warnings, fmt clean | VERIFIED | Targeted: `cargo test --lib empty_workstream` gave 14 passed, 0 failed. Coordinator's final full run on 981da0e: 89 targets, 4200 passed, 1 failed (the expected `envelope::policy` git-version test), 24 ignored; clippy 0 warnings; fmt clean. The full suite was not re-run here. |

**Score:** 7/7. Each behavior-dependent truth (fold, cursor-skip, selection invariants) is backed by a passing named test. 0 behavior-unverified.

## Wiring

| From | To | Status |
|------|----|--------|
| `roadmap_model_at` Scope.status | `roadmap_graph::Scope` / `BandInput` | WIRED |
| `row_sequence` | `ListRow::NoPhases` via `LaneKind::NoPhases` / `assign_lanes` (:831-834, :1121) | WIRED |
| `target_of` | cursor and mouse | WIRED (None for NoPhases) |
| `render_pipeline_tab` | `empty_workstream_line` | WIRED |

Data flow: the status comes from the workstream's STATE.md (`ws.state.status`) and emptiness from the roadmap entries, so nothing is hard-coded.

## Anti-Patterns

No TBD, FIXME or XXX markers in `roadmap_graph.rs`. The only match in `roadmap_view.rs` is a pre-existing user-visible "plans TBD" string at :1163, which is not a debt marker. No stubs found.

## Human Verification

None. Everything is checkable by tests and code reading.

---

_Verified: 2026-10-23_
_Verifier: Claude (gsd-verifier)_
