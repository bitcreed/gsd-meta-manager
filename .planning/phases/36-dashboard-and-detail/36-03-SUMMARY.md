---
phase: 36-dashboard-and-detail
plan: 03
subsystem: ui
tags: [ratatui, owner-inbox, detail, phases-tab, owner-block]
requires:
  - phase: 36-01
    provides: AppContext.owner_digests
  - phase: 36-02
    provides: hostile_owner_digest, OWNER_SEP
provides:
  - "owner_block.rs: BlockModel, block_model, block_height, render_owner_block, sample_owner_digest (test)"
  - "inbox.rs: closes_after_clauses, ClauseForm, ClosesAfterClauses; plural, blocked_tally, detail now pub(crate)"
  - "detail.rs: Owner block placement above the Phases split; pub(super) shown and fit_spans"
  - "escape-guard state Phases tab with an Owner block"
affects: [36-04]
key-files:
  created:
    - src/ui/screens/owner_block.rs
  modified:
    - src/inbox.rs
    - src/ui/screens/detail.rs
    - src/ui/screens/mod.rs
    - src/ui/screens/render_escape_guard.rs
key-decisions:
  - "render_pipeline_tab became a thin wrapper that places the block, then calls the unchanged body as render_phases_body"
  - "Bookkeeping-only phrase may spill across the age and command columns"
requirements-completed: [OWN-19]
status: complete
commits: 2
plan_head_before: ace859369a7c9fb73c4752d440b87bfc63858c8e
plan_head_after: 894de57f165afb03f1dad42f94f0bd46233dfb60
actuals:
  tokens: 60000
  tasks: 3
  commits: 2
completed: 2026-11-15
---

# Phase 36 Plan 03: Owner block on the Phases tab Summary

The detail Phases tab now draws a borderless Owner block above the phase list: a `closes after` header with a five-rung width ladder, one row per current Test, Decide and Fix group plus one BLOCKED aggregate per phase, and a pinned collapsed Old debt / Acknowledged / Todos fold line, all from `ctx.owner_digests` in memory.

## Commits

| Tasks | Commit | Subject |
|-------|--------|---------|
| 1, 2 | 26ea57d8 | feat(36-03): draw the Owner block above the phase list on the Phases tab |
| 3 | 894de57f | test(36-03): guard the Phases tab with an Owner block against terminal escapes |

## What was built

- `inbox.rs`: `closes_after_clauses(gate, ClauseForm)` returns the lane clauses and the separate `not built ph ...` clause; `push_closes_after` calls it with `Full`, so the CLI lines and both goldens are byte-identical (tests/owner_digest.rs and tests/inbox_cli.rs green, owner_golden fixtures untouched).
- `owner_block.rs`: header ladder (full, phase lists capped at 6, counts only, not-built dropped, `fit_spans`); D-17 body rows (order, columns, phrases, `closes phase` Green, `stale` Yellow, Magenta `ws` prefix, BLOCKED rows DIM and aggregated through `blocked_tally`); column grid dropping the command below 100 and the age below 80; collapsed fold line with four rungs; `block_height` (zero when empty or tab < 12 rows, cap `max(4, 40%)`); `render_owner_block` with the fold line pinned and a DIM ` 1-N of T` hint when clipped.
- `detail.rs`: placement in `render_pipeline_tab` (zero rows under a focused full-tab Waves pane); `shown` and `fit_spans` made `pub(super)`. No digest read or file I/O on the render path.
- `render_escape_guard.rs`: `DETAIL_SUB_STATES` entry and `DETAIL_TAB_ARRIVAL` row for "Phases tab with an Owner block" (hostile digest at 200x60); the DetailScreen adjudication names it.

## Tests added

inbox.rs `closes_after_clauses_forms`. owner_block.rs (14): header rungs, capped list, blank milestone / no clause, long header, height, nothing-to-show, row order and phrases, fold line, fold ladder, archived summary and `Todos ?`, `has_content`, 300-character strings. detail.rs (9): `an_empty_or_missing_digest_leaves_the_phases_tab_unchanged`, `the_owner_block_hides_below_twelve_tab_rows`, `the_owner_block_draws_above_no_phases_found`, `the_full_tab_waves_pane_hides_the_owner_block`, `sc4_the_owner_block_heads_the_phases_tab_at_120x30`, `the_owner_block_row_grid_drops_command_then_age`, `a_fold_only_block_has_no_body`, `a_clipped_block_shows_its_row_range`, `a_click_in_the_owner_block_selects_no_phase`, `a_long_text_digest_stays_inside_the_block`.

## Verification (raw)

- `rtk bypass cargo test --locked --no-fail-fast --all-targets`: 4229 passed, 0 failed, 19 ignored, across 64 result lines (exit 0).
- `rtk bypass cargo clippy --locked --all-targets -- -D warnings`: clean (exit 0).
- `--lib ui::glyphs` (glyph source scan), `--test owner_digest` (25), `--test inbox_cli` (26): green.
- `cargo fmt` touches the unrelated `src/state_reader/disk_status.rs`; it was reverted each time and not committed.

## Divergences from CONTEXT

- D-16: the header's not-built clause reads `not built ph ...` (the CLI's text, no colon) where CONTEXT D-16 wrote `. not built: ph ...` (orchestrator decision 4, RESEARCH Correction 4).
- D-09: an unreadable owner file is Fix work in the live reader and shows as a FIX row with `N unreadable check(s)`, where CONTEXT D-09 called it Test work (orchestrator decision 2, RESEARCH Correction 1).

## Deviations from Plan

- **TDD commit split.** As in 36-01 and 36-02, tests and implementation were committed together (no standalone RED commits), and Tasks 1 and 2 share one commit because `owner_block.rs` is one file whose model, rows and fold line were written together.
- **At 120 columns the header is the counts-only rung** for the sample digest (the full form needs about 131 cells), so SC4 asserts `test 12` rather than the `(ph 21 22 24)` lists; the full form is covered at 140 by `the_header_walks_the_five_rungs`.
- **At 100 columns the command is kept but cut** to the room left (`/gsd-ve...` for the sample); the plan's "100x30 keeps the command" holds as "shows the command column".

## Inferred decisions

- Inferred decision: fold-line rungs 2 and 3 keep the `▸` glyph per the UI-SPEC ladder; only the plan's rung 3 text dropped it.
- Inferred decision: a BLOCKED row's tally spills across the blank columns to the right and does not size the name column.
- Inferred decision: a bookkeeping-only TEST phrase (`all checks passed, run ... to finalize`) spills over the age and command columns (age omitted on that row); when it does not fit even there, the row reads `all checks passed` in the phrase column with its age.
- Inferred decision: Acknowledged `N files` uses `hidden.acknowledged_files`, falling back to the distinct files of acknowledged groups.
- Inferred decision: a TEST row that neither closes a phase nor has Fix or Blocked siblings has a blank phrase.
- Inferred decision: the checks column is filled for UAT and VERIFICATION groups with at least one listed check.
- Inferred decision: `render_pipeline_tab` stays the single entry point (wrapper) and the old body is `render_phases_body`, so the two existing call sites are unchanged.

## Known Stubs

None. The block is draw-only by design; focus, expansion, opening the folds, cursor and scroll are 36-04.

## Threat Flags

None. T-36-09: every digest string passes through `shown()`; the new escape-guard state and `a_long_text_digest_stays_inside_the_block` (with a raw ESC in the strings) pass. T-36-10: height capped, only the visible window drawn, no digest computation in `detail.rs` or `owner_block.rs`.

## Human UAT

None for this plan (covered by SC4 tests; D-23 UAT concerns the dashboard).

## Self-Check: PASSED

owner_block.rs exists; commits 26ea57d8 and 894de57f are reachable from HEAD; STATE.md, state.json and ROADMAP.md untouched.
