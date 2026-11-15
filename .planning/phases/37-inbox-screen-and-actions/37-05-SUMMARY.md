---
phase: 37-inbox-screen-and-actions
plan: 05
subsystem: Owner inbox drill-in (viewer, expansion, folds, scroll, go to project)
tags: [inbox-screen, owner-file-view, folds, scroll, owner-actions, d-21]
requires: ["37-04"]
provides:
  - OwnerFileViewScreen (read-only, scrollable, escaped file view)
  - owner_actions::{MAX_VIEW_BYTES, read_owner_file, open_target}
  - owner_block::expansion_lines(src, width, shift), files_for, pub(crate) fold sources
  - InboxViewState.{expanded, open_folds, fold_cursor}; stop_rows, cursor_pos, fleet_folds, prune_view
  - DetailScreen::opened_on_owner and D-21 `o` on the Owner block
affects: [37-06, 37-07]
key-files:
  created:
    - src/ui/screens/owner_file_view.rs
  modified:
    - src/ui/screens/inbox.rs
    - src/ui/screens/owner_actions.rs
    - src/ui/screens/owner_block.rs
    - src/ui/screens/detail.rs
    - src/ui/screens/mod.rs
    - src/ui/screens/render_escape_guard.rs
key-decisions:
  - "D-08: `o` pushes an in-app viewer (inferred, RESEARCH open question 3); the file is read on the key press only when its canonical path is inside the canonical project .planning/, a regular file, at most 1 MiB; nothing is spawned"
  - "D-06/D-02: the stops are group rows, then rows of open folds, then the fold line (Triage is a stop that is refused); cursor identity is an InboxStop, the fold line index is rows.len()"
  - "D-07: `g` re-selects the dashboard table row when listed (Pitfall 6) and pushes DetailScreen::opened_on_owner itself"
  - "Coordinator directive 4: expansion_lines gained `shift` (0 from the Owner block); owner_block_output_is_pinned and the 36 Owner block tests pass with the goldens untouched"
requirements-completed: []
requirements-advanced: [OWN-20]
duration: 120 min
completed: 2026-11-15
status: complete
commits: 1
plan_head_before: f38fa85669991f06393a424afe75923209ede42e
plan_head_after: 1ee31533
actuals:
  tokens: 70000
  tasks: 3
  commits: 1
---

# Phase 37 Plan 05: Inbox drill-in Summary

The owner can now work inside the inbox: `o` shows a group's source file read-only, `Enter`/`Space` expand a group to its checks, the fold line opens Old debt and Acknowledged, a long inbox scrolls with its cursor, and `g` jumps to the project's Phases tab on the same group.

## Accomplishments

- Task 1: `owner_actions::read_owner_file` (canonicalize both `.planning/` and the target, require containment, regular file, 1 MiB cap, lossy decode) and `open_target` (one file opens, several give `{N} files behind this row: expand it with Enter, or press g`, failures give `Cannot open {file}: not readable`). `OwnerFileViewScreen` with help.rs-order scrolling, `Esc`/`q` pop, markdown through `archive::render_markdown_lines`, other files escaped line by line; no editor path exists. Census row and fixture added.
- Task 2: `expansion_lines(src, width, shift)`; inbox expansion under the row with the `run {command}` line (from `run_plan`, so exactly what `v` launches), fold stops with a `←`/`→` sub-cursor, Old debt under DIM milestone sub-headers, Acknowledged rows DIM, Triage refused, whole-row scroll with ` {first}-{last} of {total}` on the rule row, `prune_view` on every refresh (a vanished expanded group leaves the set; folds without content close), fold-line and footer ladders.
- Task 3: `g` and `DetailScreen::opened_on_owner` (focus Owner only when the identity is a block stop, else tab top, focus Content), D-21 `o` in the Owner block (`owner_block_swallows_action_keys` narrowed to `e A y x`), expanded/folds/status hostile inbox census states.

## Deviations from Plan

- [Process] The three tasks went into one commit (1ee31533), not three: they share hunks in inbox.rs, detail.rs and the census file, and the census test is red unless the viewer row, its fixture and the inbox states land together. Path-scoped commits cannot split hunks.
- [Inferred] `Home`/`End` and `PgUp`/`PgDn` move over all stops, so `End` lands on the fold line (the existing `j_k_home_end_and_paging_move_the_cursor` test was updated for that).
- [Inferred] Open-fold rows ignore the lane filter (the filter narrows the four sections only); a fold with content but no listable rows (the UI-SPEC fixture's counts-only fleet) opens to nothing.
- [Inferred] Old debt rows across projects are ordered by (milestone rank, label), a sub-header per label change; Acknowledged rows by alias.
- [Inferred] Footer: `[Enter]`, `[g]`, `[o]` inserted at their UI-SPEC positions; with no stops only `[Esc]` and `[?]` remain; the fold-line footer has two rungs (`[←/→]fold  [Enter]expand  [j/k]move  [Esc]back  [?]help`, then without `[Esc]back`). The footer ladder test widths moved (92, 70, 59, 17 cells).
- [Inferred] `v` on the fold line now says `Select a group row to run its command` (UI-SPEC table), since the fold line became a stop.
- [Plan arithmetic] Acceptance greps `grep -c "fn read_owner_file"` would count the mandated test name `read_owner_file_stays_inside_planning` too (2, not 1); the function exists once.
- Added tests beyond the list: `a_symlink_out_of_planning_is_not_followed` (`#[cfg(unix)]`), `open_target_pushes_the_viewer_or_says_why_not`, `o_on_a_blocked_row_over_two_files_points_at_expand`, `a_group_without_a_command_has_no_run_line_and_a_blocked_row_lists_its_files`, `a_hostile_drilled_in_inbox_has_no_raw_control_cell_and_nothing_past_the_width`.

## Follow-ups

- `[A]cknowledge` and `[y]copy` footer keys and the fold-line `[y]copy` hint arrive with 37-06 and 37-07.
- No mouse handling in the inbox (UI-SPEC allows deferral; the README in 37-07 says keyboard-only).

## Verification

- `cargo test --locked --no-fail-fast --all-targets`: 4349 passed, 0 failed, 19 ignored.
- `cargo clippy --all-targets -- -D warnings`: clean on the host and with `--target x86_64-pc-windows-gnu`.
- Owner block pin goldens untouched and green.

## Known Stubs

None.

## Self-Check: PASSED
