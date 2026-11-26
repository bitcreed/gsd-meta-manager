---
phase: 36-dashboard-and-detail
plan: 04
subsystem: ui
tags: [ratatui, owner-inbox, detail, phases-tab, owner-block, focus, help, readme]
requires:
  - phase: 36-03
    provides: owner_block.rs (BlockModel, block_model, render_owner_block), render_pipeline_tab wrapper
provides:
  - "owner_block.rs: GroupKey, Fold, Stop, OwnerBlockState, Cursor, Skeleton, skeleton, move_cursor, toggle, fold_step, clamp, resolve_cursor, window_rows, follow_offset; block_model(alias, milestone, digest, view, width); render_owner_block(frame, area, model, cursor, offset)"
  - "ProjectViewCache.owner_block"
  - "detail.rs: DetailFocus::Owner, handle_owner_block_key, enter_owner_block, owner_block_rows / owner_offset / owner_viewport_rows cells, owner_block_footer_spans, footer_spans(.., owner_block_drawn)"
  - "help rows, README Phases-tab paragraph and key rows, escape-guard state Phases tab with an expanded Owner block"
affects: []
key-files:
  created: []
  modified:
    - src/ui/screens/owner_block.rs
    - src/ui/screens/detail.rs
    - src/ui/screens/mod.rs
    - src/ui/screens/help.rs
    - src/ui/screens/render_escape_guard.rs
    - README.md
key-decisions:
  - "Cursor stops and prunable identities come from a width-free Skeleton (digest + view), so key handlers need no drawn model"
  - "Cursor index is remembered beside the identity so a vanished stop clamps to its neighbor"
requirements-completed: [OWN-19, OWN-23]
status: complete
commits: 2
plan_head_before: a15e5d968e3ea6c87cfcf8d8200c71e9a7b67ae4
plan_head_after: 11c50f51 (plus the SUMMARY commit)
actuals:
  tokens: 75000
  tasks: 3
  commits: 2
completed: 2026-11-15
---

# Phase 36 Plan 04: Walk and open the Owner block Summary

The Owner block on the detail Phases tab is now walkable and openable: `DetailFocus::Owner` entered with `↑`/`k` from the first phase, expandable groups (items, detail, gaps, file), openable Old debt and Acknowledged folds with a fold sub-cursor, a scroll window that follows the cursor, a refresh-safe identity-based view state, footers, help rows and README. Nothing in the block launches, opens, acknowledges, copies or writes anything.

## Commits

| Tasks | Commit | Subject |
|-------|--------|---------|
| 1, 2 | 6527f3e1 | feat(36-04): walk and open the Owner block with a refresh-safe cursor |
| 3 | 11c50f51 | docs(36-04): document the Owner block keys and guard its expanded state |

## What was built

- `owner_block.rs`: identities (`GroupKey` built from `Lane::as_str`/`Kind::as_str`; BLOCKED aggregate uses lane and kind `blocked` and an empty file), `Fold`, `Stop`, `OwnerBlockState { expanded, open_folds, cursor, fold_cursor, cursor_index }`. `block_model` takes the view state and adds the D-18 expansion lines (items 4 columns in, at most 5 then DIM `… N more`; DIM detail line at column 7 with absent parts omitted; DIM gap lines, at most 2 then `… N more gaps`; DIM `file {path}` per distinct file; BLOCKED rows expand to all merged items and files), the D-19 open-fold rows at the end of the body (Old debt under DIM milestone sub-headers by rank, Acknowledged rows DIM), `▾` for open folds, and `stops`, `stop_lines`, `folds`, `fold_marks`. One grid is computed over all drawn rows. Helpers `move_cursor`, `toggle`, `fold_step`, `clamp`, `resolve_cursor`, `enter_last`, `window_rows`, `follow_offset`.
- `render_owner_block` draws the cursor row (`> ` gutter, BOLD plus Cyan, only while focus is Owner); on the fold line the fold under the sub-cursor is BOLD Cyan.
- `detail.rs`: `DetailFocus::Owner` (Phases tab only, reset to Content elsewhere), dispatch right after the Pane block, entry in the `↑`/`k` arm before the tab-bar fallback when the last frame drew a block, D-20 key table (`j k ↓ ↑ g G PgUp PgDn Enter Space ← → Esc q`, swallowed `e o v A y`, fall-through `? Tab`, digits/`D`/`[`/`]` reset focus and fall through), footers (`[↑]owner` lead; the Owner footer text exact), render-written cells.
- `help.rs`: six `(detail view)` rows under `Phases Owner Block (detail view)`; `README.md`: the Phases-tab paragraph, the `↑ / k` row, `Phases` and `Owner block` Tab-specific rows, keyboard-only note in the Mouse paragraph. The docs describe no Phase 37 action.
- `render_escape_guard.rs`: state `Phases tab with an expanded Owner block` (hostile digest, every current group expanded, Old debt open).

## Tests added

detail.rs (14): `owner_block_focus_path`, `without_a_block_up_still_reaches_the_tab_bar`, `owner_block_swallows_action_keys`, `owner_block_digits_reset_focus`, `enter_expands_and_collapses_a_group`, `sc4_an_expanded_group_shows_its_checks_beside_the_collapsed_rows`, `the_cursor_row_is_marked_only_while_focused`, `the_footer_names_the_owner_block`, `focus_pane_below_100_columns_still_takes_the_whole_tab`, `the_fold_line_walks_and_opens_old_debt_and_acknowledged`, `owner_block_state_survives_a_digest_refresh`, `owner_block_scrolls_with_the_cursor`, `an_expanded_long_text_block_stays_inside_the_block`. owner_block.rs (8): expanded lines and caps, BLOCKED expansion, stops and open folds, `fold_step`, `toggle`, `move_cursor`, `clamp`, scroll window. help.rs: `help_documents_the_owner_block_keys`.

## Verification (raw)

- `rtk bypass cargo test --locked --no-fail-fast --all-targets`: 4251 passed, 0 failed, 19 ignored across 64 result lines.
- `rtk bypass cargo clippy --locked --all-targets -- -D warnings`: clean (host).
- Cross-target clippy (`--locked --all-targets -- -D warnings`): `x86_64-pc-windows-gnu` clean; `aarch64-apple-darwin` clean.
- README acceptance greps: `| Owner block |` 1, `Project detail, Phases tab` 1, the Phase 37 action grep over `### Owner inbox` 0.
- `cargo fmt` touched the unrelated `src/state_reader/disk_status.rs`; reverted each time, never committed.

## Divergences from CONTEXT

Repeated from 36-03 so the verifier finds them in one place. The docs written here describe neither form.

- D-16: the header's not-built clause reads `not built ph ...` without a colon (orchestrator decision 4, RESEARCH Correction 4), where CONTEXT D-16 wrote `. not built: ph ...`.
- D-09: an unreadable owner file is Fix work in the live reader and shows as a FIX row with `N unreadable check(s)` (orchestrator decision 2, RESEARCH Correction 1), where CONTEXT D-09 called it Test work.

## Deviations from Plan

- **Tasks 1 and 2 share one commit, and TDD RED commits are not separate** (as in 36-01..36-03): the focus, expansion, fold, scroll and clamp code is one tightly coupled model in `owner_block.rs`, so a Task-1-only state that compiles without the fold and scroll parts was not worth faking.
- **Signature deltas (Rule 3, internal):** `render_owner_block` takes `cursor: Option<Cursor>` (stop index plus the fold sub-cursor) instead of `Option<usize>`, because the fold line needs the sub-cursor; `fold_step` takes `&[Fold]` and `clamp` takes the `OwnerDigest`, not a `BlockModel`, so key handlers need no width or drawn model (a new width-free `skeleton(digest, view)` provides stops, selectable folds and valid keys).
- **SC4 expanded at 120x30:** the block's 40% cap leaves a window shorter than one expanded group plus its siblings, so the test shows the group scrolled into view (rows, then the tail with `… 1 more` and `file …` after moving to the next stop) rather than every line at once. The model test asserts every line.

## Inferred decisions

- Inferred decision: the fold line has no gutter in the collapsed state (36-03's tests assert its text starts at `▸`); with the cursor on it, a `> ` gutter is prefixed and the line is cut to the width.
- Inferred decision: the cursor index is stored beside the cursor identity (`OwnerBlockState.cursor_index`) so a vanished stop clamps to the nearest remaining index.
- Inferred decision: `PgUp`/`PgDn` move by the viewport row count measured in stops.
- Inferred decision: the window keeps the cursor row and as much of its expansion as fits in view.
- Inferred decision: `… N more gaps` is literal for every N (the plan's behavior bullet writes `… 1 more gaps`).
- Inferred decision: Old debt rows are the archived, non-acknowledged groups `Filters { archived: true }` lists, including BLOCKED ones as their own rows (tally, falling back to the phase name when the group has no items); Acknowledged rows are all groups flagged `acknowledged`.
- Inferred decision: Old debt is selectable when its count is non-zero; Acknowledged when the digest carries acknowledged groups; Todos never.
- Inferred decision: scroll offset is a render-written `Cell` (as in the plan's assumptions); help keys use the overlay's ASCII vocabulary.

## Known Stubs

None.

## Threat Flags

None. T-36-12: `Enter`/`Space` only toggle view state; `e o v A y` are swallowed (`owner_block_swallows_action_keys`). T-36-13: identities, `clamp` before use, render resolves the cursor against current stops (`owner_block_state_survives_a_digest_refresh`, including an emptied digest). T-36-14: every string through `shown()`; the expanded escape-guard state and `an_expanded_long_text_block_stays_inside_the_block` pass. T-36-15: five items, two gaps, windowed body.

## Human UAT

None for this plan (D-23 UAT concerns the dashboard).

## Self-Check: PASSED

owner_block.rs and the other modified files exist; commits 6527f3e1 and 11c50f51 are reachable from HEAD; STATE.md, state.json and ROADMAP.md untouched.
