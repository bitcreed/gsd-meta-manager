---
phase: 36-dashboard-and-detail
plan: 02
subsystem: ui
tags: [ratatui, owner-inbox, dashboard, summary, filter, help, readme]
requires:
  - phase: 36-01
    provides: owner_numbers, has_owner_work, you_cell_line, AppContext.owner_digests
provides:
  - "OwnerTotals, owner_totals, summary_left_line, summary_right_spans, OwnerLane, lane_count (owner.rs)"
  - "the You: summary row with [i] inbox and the interim i key (normal.rs)"
  - "4-rank attention_rank, owner_work_matches, FilterColumn::OwnerWork and the /y suffix grammar"
  - "help overlay Owner Work section, README ### Owner inbox (dashboard part), dashboard escape-guard state"
affects: [36-03, 36-04]
key-files:
  modified:
    - src/ui/screens/owner.rs
    - src/ui/screens/normal.rs
    - src/ui/screens/mod.rs
    - src/app.rs
    - src/ui/screens/help.rs
    - src/ui/screens/render_escape_guard.rs
    - README.md
key-decisions:
  - "Owner rank is independent of ctx.experimental; the driven rank keeps its gate"
  - "Totals come from the same owner_numbers as the cell"
requirements-completed: [OWN-17, OWN-18, OWN-23]
status: complete
commits: 3
plan_head_before: afe91b1dd0523c7a662f579c0530ab2cf5aa1474
plan_head_after: 484f3e976ad80ed83c3a8c05258d3dfb46fcf105
actuals:
  tokens: 45000
  tasks: 3
  commits: 3
completed: 2026-11-14
---

# Phase 36 Plan 02: Dashboard summary, owner rank and /y filter Summary

A `You:` fleet-totals row with `[i] inbox` under the dashboard table (from 100 columns and 12 rows), an owner-work rank between needs-a-human and driven under attention-first, the `/y [lane]` filter, and the help, README and escape-guard coverage for those surfaces.

## Commits

| Task | Commit | Subject |
|------|--------|---------|
| 1 | 02c81a41 | feat(36-02): show fleet owner totals in a You: summary row and add the interim i key |
| 2 | 21f4cbbe | feat(36-02): rank owner work in attention-first order and filter it with /y [lane] |
| 3 | 484f3e97 | docs(36-02): document the owner work column, sort and filter and guard the dashboard state |

## What was built

- `owner.rs`: `owner_totals` sums `owner_numbers` over the visible rows (pending rows counted, not summed); `summary_left_line` takes the first whole form of the D-10 ladder (full, no `K of N projects`, no `(C close)`, no ` in P phases`, then `fit_cells` with `…`), loading clause kept through the four whole forms; `summary_right_spans`, `INBOX_HINT_CELLS = 9`; `OwnerLane` (Test, Decide, Fix, so `/y blocked` is unrepresentable) and `lane_count`.
- `normal.rs`: vertical split `[Min(0), Length(1), Length(1)]` at width >= 100 and height >= 12 (`SUMMARY_MIN_WIDTH`, `SUMMARY_MIN_HEIGHT`), otherwise the old two-part split; the `i` arm sets the status message and pushes nothing.
- `mod.rs` and `app.rs`: `attention_rank(needs_human, has_owner_work, driven_and_live)`, `owner_work_matches`, `FilterColumn::OwnerWork(Option<OwnerLane>)`, `parse_filter` suffixes `/y test`, `/y decide`, `/y fix`, `/yt`, `/yd`, `/yf`, `/y` (any other word after `/y ` stays a plain term).
- help overlay: changed `s` row (text lives in the `SORT_ROW` constant, used by the row and its test), `i` row, `/term/y`, `//y`, `//y test`, and an `Owner Work` section between Filter Syntax and Dashboard Badges. README: `### Owner inbox` before `### Examples` (intro, dashboard paragraph, cell table, summary/sort/filter paragraph) and the `s`, `i`, `/` key rows.
- `render_escape_guard.rs`: `hostile_owner_digest(identity)` (36-03 and 36-04 reuse it) and the state `dashboard with owner work`; the NormalScreen adjudication reason lists it.

## Tests added

owner.rs: `owner_totals_sum_the_visible_rows`, `summary_forms`, `summary_ladder`, `lanes_and_hint`. normal.rs: `the_summary_row_sits_above_the_footer_at_100_120_and_140`, `summary_totals_equal_the_sum_of_the_visible_cells` (unfiltered, name filters, `//y`, `//y test`, with the two D-11 texts), `the_summary_reads_loading_until_a_digest_arrives`, `a_fleet_without_owner_work_reads_a_dim_dot`, `the_i_key_points_at_the_inbox_command`, `the_summary_line_takes_no_color`, `sort_and_filter_by_owner_work_at_120`. mod.rs: `the_attention_rank_table_covers_every_combination`, `attention_first_puts_owner_work_between_humans_and_driven_runs`, `blocked_and_archived_debt_alone_is_not_owner_work`, `owner_work_ordering_is_stable_inside_a_rank`, `an_empty_registry_sorts_and_filters_to_an_empty_list`, `the_owner_work_filter_parses_every_form`, `owner_work_filters_select_by_lane`. app.rs: `a_digest_only_change_re_sorts_under_attention_first`. help.rs: `help_documents_the_owner_work_column_sort_and_filter`.

## Verification (raw)

- `rtk bypass cargo test --locked --no-fail-fast --all-targets`: 4206 passed, 0 failed, 19 ignored, across 64 result lines.
- `rtk bypass cargo clippy --all-targets -- -D warnings`: clean (exit 0).
- needs_human function-name count 15 (unchanged); diff of `fn ...needs_human` lines against d36f6733: 0; the `sorted_aliases` assertion line of `attention_first_is_stable_across_projects_of_equal_rank` occurs once, unchanged.
- `cargo fmt --check` differs only in the pre-existing `src/state_reader/disk_status.rs`, left alone (not committed).

## SC3 (snapshots)

No test below 100 columns changed; `below_100_columns_the_dashboard_ignores_owner_digests` (36-01) still holds. No existing test at 100 columns or wider needed an edit: all existing 12+ row dashboard tests pass with the summary row, so the Pitfall 8 height gate stayed at 12. The only existing test edited is `attention_first_is_stable_across_projects_of_equal_rank`, and only its three `attention_rank` assertion lines (orchestrator decision 6).

## Divergences from CONTEXT

- D-09: unreadable owner files are Fix work in the live reader (orchestrator decision 2, RESEARCH Correction 1). The help `fix N ph` row names unreadable checks as fix work and the README never calls them test work, where CONTEXT D-09 and the UI-SPEC called them Test work.
- D-16: the Owner block header (36-03) uses the CLI's `not built ph ...` without a colon (orchestrator decision 4) where CONTEXT D-16 wrote `· not built: ph ...`. No help row or README text written here uses the colon form.

## Deviations from Plan

- **TDD commit split.** As in 36-01, tests and implementation were committed together per task; no standalone RED commits (the tracer task spans files that must compile together, and `OwnerLane` and `lane_count` had to wait for Task 2 or clippy fails on dead code).
- **[Rule 1 - Bug] help `i` row wording.** The plan said to use the UI-SPEC `i` description verbatim (`... opens the inbox screen (for now it shows the inbox command)`), which contradicts the prohibition on documenting the inbox screen as available. The row reads `Owner inbox (for now: shows how to run gsd-meta-manager inbox)`, matching the README `i` row, and the help test asserts the text `opens the inbox screen` is absent.
- **Plan wording.** README intro drops "and the project detail view" (that surface arrives in 36-03/36-04); 36-04 adds the detail paragraph.

## Inferred decisions

- Inferred decision: an all-digests-present fleet with no clause but some rows pending reads `You: · · M loading` (dim dot, then the dim loading clause), so the fail-safe signal is not lost.
- Inferred decision: rung 5 of the summary ladder truncates the body of rung 4 (loading clause included) with `fit_cells`, after the bold `You: ` prefix.
- Inferred decision: `SORT_ROW` constant holds the `s` row description so the literal appears once in help.rs (the plan's grep expects 1).
- Inferred decision: `attention_rank` argument order is `(needs_human, has_owner_work, driven_and_live)` (orchestrator decision 5).
- Inferred decision: the `i` help row description follows the README row instead of the UI-SPEC wording (see deviation).

## Known Stubs

None. The interim `i` handler is intentional (D-12) and is replaced in Phase 37.

## Threat Flags

None. T-36-06 (hostile text): the summary and cell hold only numbers and fixed words; the new escape-guard state renders a hostile alias plus `hostile_owner_digest` at 200x60. T-36-07: `the_i_key_points_at_the_inbox_command` asserts `ScreenAction::None`. T-36-08: `summary_totals_equal_the_sum_of_the_visible_cells` over five filter subsets; pending rows report `M loading`.

## Notes

- `ui::screens::driver_run::tests::a_held_lock_with_an_old_heartbeat_reads_stale_and_a_free_lock_reads_crashed` failed once in a parallel `--lib ui::screens` run and passed alone and in the full run (it takes a real process lock); unrelated to this plan.

## Human UAT (run in /gsd-verify-work)

The summary line fits and `[i] inbox` is right-aligned at 100 and 140 columns on the real terminal font (part of UAT item 2).

## Self-Check: PASSED

Files and the three commits verified reachable from HEAD.
