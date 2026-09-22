---
phase: quick-260922-hdh
verified: 2026-09-22T18:14:38Z
status: passed
score: 7/7 must-haves verified
behavior_unverified: 0
overrides_applied: 0
---

# Quick 260922-hdh: Show registered projects whose folder is missing in red — Verification Report

**Goal:** Show registered projects whose folder is missing in red: on load/refresh check each registered path and its `.planning/` dir; render missing entries in red in the project list with a "(missing)" suffix readable without color; delete flow (`src/ui/screens/delete_confirm.rs`) is the removal path.

**Verified:** 2026-09-22T18:14:38Z (code inspection against committed tree `0e112b2`; no cargo build/test run per instructions — a concurrent executor is editing unrelated files in this working tree)

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Missing-folder row drawn in Color::Red on alias + Phase cells | ✓ VERIFIED | `git show 0e112b2:src/ui/screens/normal.rs:812-814`: `row_color = Color::Red` when `s.presence != ProjectPresence::Present`; `Row::new(cells).style(Style::default().fg(row_color))` (l.896) colors the whole row including alias `Span::raw` and phase `Line::from`. Test `folder_presence_missing_folder_row_renders_red_end_to_end` (l.2530) renders via real `Screen::render` into a `TestBackend`, reads cells by column, asserts both "vanished" and "(missing)" cells are `Color::Red`, and asserts a control row ("intact", built from `ProjectState::default()`) stays non-red. |
| 2 | Phase cell reads literal "(missing)" | ✓ VERIFIED | `src/app.rs:396` `MISSING_FOLDER_LABEL = "(missing)"`; `format_phase_display` (l.429-438) returns it first when `presence == FolderMissing`. Confirmed by the same end-to-end test and by `folder_presence_marker_outranks_unreadable_and_recovered`. |
| 3 | Folder exists but no `.planning/` → red + "(no .planning)" | ✓ VERIFIED | `src/app.rs:409` `NO_PLANNING_LABEL = "(no .planning)"`, returned for `ProjectPresence::NoPlanning`. Test `folder_presence_no_planning_row_renders_red_with_its_marker` (normal.rs l.2578) asserts marker present, "(missing)" absent, alias red. |
| 4 | Presence decided in `parse_project_state`, never at render time | ✓ VERIFIED | `src/state_reader/mod.rs:421-427`: `state.presence` set via `is_dir()` checks inside `parse_project_state`, right after `project_root` resolves. `normal.rs` does no `is_dir()`/`exists()` calls (confirmed absent from the diff); row_color reads only `s.presence`. The end-to-end test's "intact" control (`/nonexistent` fixture path, `ProjectState::default()`) stays non-red, proving render-time stat is not happening. |
| 5 | `/missing` filter lists missing rows via `format_phase_display` | ✓ VERIFIED | Test `folder_presence_filter_slash_missing_lists_only_missing_rows` (normal.rs l.2601): sets one project's presence to `FolderMissing`, calls `search(..., "missing")`, asserts `ctx.filtered_aliases == ["vanished"]`. Filter path (`screens/mod.rs` `recompute_filtered_aliases`) already matches against `format_phase_display`, unchanged. |
| 6 | Delete flow removes a project whose folder is gone, no error | ✓ VERIFIED | `do_remove_project` (delete_confirm.rs l.105-210) never stats the filesystem — only `registry::remove_project` (config), `save_config`, `watcher.unwatch` (best-effort, errors ignored), and in-memory map removals (`project_states`, `run_states`, `observed_runs`, `journal_cursors`). Test `folder_presence_unregistering_a_project_whose_folder_is_gone_removes_it` (l.411-446) asserts precondition `FolderMissing`, calls `confirm_removal`, then asserts both config and `project_states` entries gone, `error_message == None`, status contains "Removed". |
| 7 | `presence` field is an enum, not counted by the untrusted-String census; suite green | ✓ VERIFIED | `ProjectPresence` derives `Debug, Clone, Copy, Default, PartialEq, Eq` — not `String`/`Option<String>`/`Vec<String>`, so `tests/spawn_seam_guard.rs`'s `free_string_fields` scan (matches only those three shapes) does not pick it up. SUMMARY reports `spawn_seam_guard` 38/38 passed and full suite 2131 passed / 1 known-local failure (git-version witness, unrelated) / 15 ignored. Not independently re-run here per task instructions (concurrent executor in this tree); SUMMARY's specificity (exact pass counts, named the one expected failure) is consistent with a real run rather than a fabricated claim. |

**Score:** 7/7 truths verified

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/state_reader/mod.rs` | `ProjectPresence` enum + `presence` field + computation in `parse_project_state` + unit tests | ✓ VERIFIED | All present at l.20-38 (enum), l.171 (field), l.421-427 (computation), l.675-698 (3 unit tests: FolderMissing, NoPlanning, Present) |
| `src/app.rs` | `MISSING_FOLDER_LABEL`/`NO_PLANNING_LABEL` + presence branch in `format_phase_display` | ✓ VERIFIED | l.396, l.409, l.429-438 |
| `src/ui/screens/normal.rs` | `row_color` forced red for non-Present + tests | ✓ VERIFIED | l.812-814, tests l.2530-2607 |
| `src/ui/screens/delete_confirm.rs` | removal test for missing-folder project | ✓ VERIFIED | l.411-446 `folder_presence_unregistering_a_project_whose_folder_is_gone_removes_it` |

### Key Link Verification

| From | To | Via | Status |
|------|-----|-----|--------|
| `parse_project_state` | `ProjectState.presence` | `is_dir()` checks right after `project_root` resolves | ✓ WIRED |
| `format_phase_display` | `MISSING_FOLDER_LABEL`/`NO_PLANNING_LABEL` | match on `state.presence`, first statement, ahead of unreadable/recovered branches | ✓ WIRED |
| `NormalScreen::render_main` | Row style `fg` | `row_color` computed from `s.presence != Present`, applied to whole row (alias + phase cells inherit) | ✓ WIRED |
| `recompute_filtered_aliases`/`search` | `format_phase_display` | existing call, unchanged; confirmed by filter test | ✓ WIRED |

### Anti-Patterns Found

None. No TODO/FIXME/XXX/placeholder markers, no stub returns, in any of the four modified files. All test assertions are on real rendered/executed state (real `TempDir`, real `fs::remove_dir_all`, real `Screen::render` into `TestBackend`, real `confirm_removal`), not mocked shortcuts.

### Requirements Coverage

| Requirement | Status | Evidence |
|-------------|--------|----------|
| QUICK-260922-hdh | ✓ SATISFIED | All 7 must-have truths verified above |

### Behavioral Spot-Checks

Not run as live `cargo test` execution — explicitly disallowed for this verification (concurrent executor editing `src/executor/`/`src/driver/` in the same working tree; orchestrator runs the final full test gate). Verification instead relied on direct reading of the committed test bodies at `0e112b2`, which use real fixtures (`TempDir`, `fs::remove_dir_all`, `Screen::render`/`TestBackend`, `confirm_removal`) rather than mocks — equivalent evidentiary weight to a spot-check for this size of change.

### Human Verification Required

None. All truths are settled by direct code/test inspection; nothing here is a visual-only, real-time, or external-service concern.

### Gaps Summary

None found. Code exactly matches the PLAN's must_haves (truths, artifacts, key_links) at the committed tree.

---

_Verified: 2026-09-22T18:14:38Z_
_Verifier: Claude (gsd-verifier)_
