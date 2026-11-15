---
phase: 36-dashboard-and-detail
verified: 2026-11-14T00:00:00Z
status: human_needed
score: 4/4 roadmap success criteria verified (plan must-haves: all machine-checkable truths verified; 0 failed)
covered_files:
  - .planning/phases/36-dashboard-and-detail/36-01-PLAN.md
  - .planning/phases/36-dashboard-and-detail/36-01-SUMMARY.md
  - .planning/phases/36-dashboard-and-detail/36-02-PLAN.md
  - .planning/phases/36-dashboard-and-detail/36-02-SUMMARY.md
  - .planning/phases/36-dashboard-and-detail/36-03-PLAN.md
  - .planning/phases/36-dashboard-and-detail/36-03-SUMMARY.md
  - .planning/phases/36-dashboard-and-detail/36-04-PLAN.md
  - .planning/phases/36-dashboard-and-detail/36-04-SUMMARY.md
  - README.md
  - src/action.rs
  - src/app.rs
  - src/inbox.rs
  - src/main.rs
  - src/ui/screens/add_project.rs
  - src/ui/screens/delete_confirm.rs
  - src/ui/screens/detail.rs
  - src/ui/screens/driver_confirm.rs
  - src/ui/screens/help.rs
  - src/ui/screens/mod.rs
  - src/ui/screens/normal.rs
  - src/ui/screens/owner.rs
  - src/ui/screens/owner_block.rs
  - src/ui/screens/render_escape_guard.rs
covered_digest: "v4:sha256:8c755959a086807ddc5f28a9963bbff80735c5583597c856472e75cfd02ba10b"
behavior_unverified: 0
overrides_applied: 0
re_verification: false
human_verification:
  - test: "Read the `You` cell wording on a real fleet at 100, 120 and 140 columns (see the real-fleet renders at the uncommitted real-fleet ASCII renders (phase36-screens.txt, orchestrator scratchpad), then a live terminal)"
    expected: "`decide N`, `test N -> closes M`, `fix N ph`, dim `.` and `(+N old)` are legible and tell the owner which projects to test first; the Progress and Backlog columns stay aligned on a terminal that draws U+00B7 / U+2192 double-wide (OWNER_SEP is the one-line swap)"
    why_human: "Subjective wording and glyph-width behavior (East-Asian-Ambiguous) depend on the owner's terminal and font; ROADMAP names this as human UAT"
  - test: "Open a project's detail Phases tab with real owner work; press Up on the first phase, walk the Owner block, Enter on a TEST group, open the Old debt fold, Esc back"
    expected: "Header reads `Owner -- alias . milestone closes after: ...`; rows, expansion, folds and the footers read well and nothing is launched or written"
    why_human: "Visual layout and feel of the block (UI audit scored 19/24; five real bugs fixed, remaining polish is subjective)"
  - test: "Decide whether the summary line totals should count lanes the two-lane cap hides"
    expected: "OWN-17 says totals equal the sum of the visible cells. The implementation (CONTEXT D-11, inferred) sums the same per-project numbers over the visible rows, including a `fix` count that a `decide + test` cell hides, so the `You:` line can show `fix 11` while the visible cell text sums to fewer. Accept D-11, or require the totals to sum the displayed lane text only"
    why_human: "Interpretation of the requirement wording; a documented, tested choice rather than a defect"
  - test: "Accept or reject the recorded deviations: WR-07 (once per UTC hour re-read of projects that show ages, relaxing D-03 `no timer-driven recompute`), WR-01 (a project whose folder or `.planning/` is gone shows a blank `You` cell and is left out of the K-of-N count), D-09 divergence (unreadable owner files count as fix, not test), D-16 (no colon in the Owner block header)"
    expected: "Each deviation is acceptable for v2.5.0"
    why_human: "Inferred decisions that change a CONTEXT decision or a UI-SPEC detail; all are documented in code comments, 36-REVIEW-DISPOSITION.md and the plans"
---

# Phase 36: Dashboard and detail Verification Report

**Phase Goal:** From the dashboard the owner sees at a glance which projects need him and why, without the hard-stop badge changing meaning, and the detail Phases tab shows the full owner work for one project.
**Verified:** 2026-11-14
**Status:** human_needed
**Re-verification:** No, initial verification (HEAD 9110584b)

## Goal Achievement

### Observable Truths (ROADMAP success criteria, the contract)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | At 120 columns the owner reads off which three projects to test first; `You` shows at most two lanes in fixed order for the current milestone, dim `.` when clean, hidden below 100 columns | VERIFIED | `dashboard_columns` (normal.rs:443) adds `You` only at >= 100 with `Length(21)`; `lane_parts` (owner.rs:94) emits decide, test, fix in order with `truncate(2)`; `you_cell_line` returns a dim `·` when `!has_owner_work`; the cell reads only `ctx.owner_digest()` (no parse on the render path; `grep` finds no `read_owner_digest` or `parse_project_state` under the draw code). Tests pass: `sc1_the_three_closeable_test_projects_read_legibly_at_120`, `the_you_column_shows_every_fleet_cell_at_100_120_and_140`, `below_100_columns_the_dashboard_ignores_owner_digests` |
| 2 | A project with only blocked or archived items shows no lane text; summary totals (with `[i] inbox`) equal the sum of the visible cells | VERIFIED (interpretation flagged) | `has_owner_work` excludes blocked and archived (owner.rs:79); `sc2_blocked_and_archived_only_projects_show_no_lane_text` passes; summary row `render_owner_summary` (normal.rs:711) uses `owner_totals` over the same `owner_numbers`; `summary_totals_equal_the_sum_of_the_visible_cells` passes. Caveat: totals include a lane the two-lane cap hides (D-11), see human item 3 |
| 3 | AttentionFirst ranks `has owner work` and `/y [lane]` filters by it, while `needs_human` and `❢` are unchanged; no existing snapshot changes except the added column | VERIFIED | `attention_rank(needs_human, has_owner_work, driven_and_live)` returns 0/1/2/3 (mod.rs:1512); `the_attention_rank_table_covers_every_combination` and `attention_first_is_stable_across_projects_of_equal_rank` pass; `owner_work_matches` + `FilterColumn::OwnerWork` implement `/y`, `/y test|decide|fix`; `git diff 5f08a40d..HEAD` shows no removed or added line touching `needs_human` logic or the badge (the only `attention_rank` edit is the signature and its three assertion lines); sub-100-column output covered by `below_100_columns_the_dashboard_ignores_owner_digests` |
| 4 | The detail Phases tab shows the Owner block with expandable groups, the `closes after` header and collapsed Old debt / Acknowledged / Todos folds | VERIFIED | `render_pipeline_tab` (detail.rs:7646) builds `owner_block::block_model` from `ctx.owner_digests` and draws it above the split with a zero-row gate; header wording from `inbox::closes_after_clauses` (shared with the CLI, goldens untouched); `DetailFocus::Owner` + `handle_owner_block_key` (detail.rs:2148) give Enter/Space expansion, fold stepping, paging; the SC4 buffer tests in owner_block.rs and detail.rs pass |

**Score:** 4/4 roadmap truths verified; 0 present-but-behavior-unverified.

### Plan must-haves (spot-checked against code, not SUMMARY)

| Truth | Status | Evidence |
|-------|--------|----------|
| D-01 digest in the existing reparse task, one dispatch site | VERIFIED | app.rs:1056 `read_owner_digest` inside `schedule_reparse`'s `spawn_blocking`, sent in `ProjectStateLoaded.digest` |
| D-02 digests on `AppContext.owner_digests`, never on `ProjectState`; digest-only change redraws and re-sorts but sets no `Updated:` | VERIFIED | app.rs:2975-3010; `a_digest_only_change_re_sorts_under_attention_first` passes |
| D-02 prune | VERIFIED | `prune_driver_maps` retains `owner_digests` (app.rs ~3781); `pruning_drops_the_view_cache_for_an_unregistered_alias` passes |
| D-04 watcher refresh delivers a fresh digest | VERIFIED | `a_watcher_refresh_delivers_a_fresh_owner_digest` passes |
| Startup/registration first fill never replaces a reparse digest | VERIFIED | `OwnerDigestLoaded` arm (app.rs:3706) only fills when no digest is present; `schedule_owner_digest` wired from main.rs:551, app.rs:2696 and 3117, add_project.rs:207; it clones the parsed state and never calls `parse_project_state` |
| D-07 quiet cell | VERIFIED | every span sets `Color::Reset` (owner.rs `quiet`/`quiet_dim`); `the_cell_is_quiet_on_every_span` passes |
| D-09 absent digest is blank, never `·` | VERIFIED | `you_cell_line(None)` returns an empty line |
| D-10/D-12 summary row, interim `i` pushes nothing | VERIFIED | normal.rs:808 sets a status message and returns `ScreenAction::None`; README and help describe `i` as "currently shows how to run `gsd-meta-manager inbox`" |
| D-14 `/y` grammar | VERIFIED | `sort_and_filter_by_owner_work_at_120` and parse tests pass |
| D-15 zero rows when nothing to show | VERIFIED | render_pipeline_tab falls through to `render_phases_body` unchanged |
| D-18..D-20 expansion, folds, focus, no actions (`v o A y e` do nothing) | VERIFIED | handle_owner_block_key `_ => {}` arm; only Enter/Space toggles |
| Refresh-safe cursor and clamp | VERIFIED | `clamp_drops_vanished_groups_and_folds_and_pulls_the_cursor_in`, `owner_block_state_survives_a_digest_refresh` pass |
| OWN-11 escape coverage | VERIFIED | escape-guard states for the dashboard, collapsed and expanded Owner block exist and pass |

### Prohibitions

| Prohibition | Status | Evidence |
|-------------|--------|----------|
| No red/yellow/green `You` cell, no badge glyph | VERIFIED (test-tier) | `the_cell_is_quiet_on_every_span` |
| Absent digest never reads as clean | VERIFIED (test-tier) | blank cell test; WR-01 read-side guard `owner_digest()` |
| Do not widen `needs_human`, `❢`, `/h`, rank 0 | VERIFIED (test-tier) | diff shows these untouched |
| `i` pushes no screen, launches nothing | VERIFIED (test-tier) | normal.rs:2984 test |
| Owner block launches/writes nothing | VERIFIED (test-tier) | key handler above |
| Docs do not claim unbuilt actions (judgment-tier) | VERIFIED by reading, flagged non-authoritative | README (lines 259, 368-400) and help.rs describe only `i` (interim), `Enter`, folds, `Esc`; no `v`/`o`/`A`/`y` documented as available |

### Required Artifacts

| Artifact | Status | Details |
|----------|--------|---------|
| src/ui/screens/owner.rs (752 lines) | VERIFIED | real pure core: numbers, cell ladder, totals, summary ladder; wired from normal.rs and mod.rs |
| src/ui/screens/owner_block.rs (2240 lines) | VERIFIED | model, stops, folds, clamp, render; wired from detail.rs |
| src/inbox.rs shared wording | VERIFIED | `closes_after_clauses` used by both CLI and block; CLI goldens unchanged |
| src/action.rs `OwnerDigestLoaded`, `ProjectStateLoaded.digest` | VERIFIED | consumed in app.rs |
| README `### Owner inbox`, help overlay `Owner Work` + block keys | VERIFIED | present, consistent, no claims about Phase 37 actions |

### Key Link / Data-Flow

Digest flows from files to the screen: `read_owner_digest` (reparse task, startup task) -> `Action` -> `ctx.owner_digests` -> `owner_digest()` -> cell, summary, rank, filter, block. Every link is wired and FLOWING; no static or hollow data. Render path performs no I/O.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Owner/summary/attention/digest tests | `cargo test --lib -- owner summary attention you_cell digest` | 369 passed, 0 failed | PASS |
| Named phase tests (watcher digest, rank table, totals, re-sort, <100 cols, prune, stability) | single-name runs | each 1 passed | PASS |

Orchestrator gate at HEAD (not re-run in full by this verifier): 4266 passed / 0 failed / 19 ignored; clippy and fmt clean.

### Probe Execution

SKIPPED: the phase declares no probe scripts.

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|-------------|--------|----------|
| OWN-16 | 36-01 | `You` column per report 3.1 | SATISFIED | SC1/SC2 above; buffer tests at 100/120/140 and below-100 identity |
| OWN-17 | 36-02 | Summary line with totals and `[i] inbox` | SATISFIED (see human item 3 on the hidden-lane reading) | `render_owner_summary`, `summary_totals_equal_the_sum_of_the_visible_cells` |
| OWN-18 | 36-02 | `has owner work` rank and `/y [lane]`, `needs_human`/`❢` unchanged | SATISFIED | rank table test, diff inspection |
| OWN-19 | 36-03, 36-04 | Owner block on the Phases tab with expandable groups, `closes after` header, folds | SATISFIED | SC4 |
| OWN-23 (help overlay and README, TUI half only) | 36-02, 36-04 | Help and README for the dashboard and detail surfaces | SATISFIED for this phase's part; the inbox-screen/actions/`inbox` schema half belongs to Phase 37 and stays pending |

No orphaned requirement IDs: REQUIREMENTS.md maps OWN-16..19 to Phase 36 and OWN-23 to Phase 37 (partially delivered here). Note: the REQUIREMENTS.md checkboxes and traceability rows for OWN-16..19 still read `[ ]` / `Pending`; the orchestrator should flip them when it closes the phase.

### Anti-Patterns Found

None. No TBD/FIXME/XXX/TODO/HACK or `todo!`/`unimplemented!` added in the phase's diff; no stub or hardcoded-empty render paths (empty cells are the specified D-09 behavior). The review (12 findings) and the UI audit bugs are fixed per 36-REVIEW-DISPOSITION.md (0 open) and the commits through 9110584b.

### Evaluation of the flagged deviations

- WR-07 hourly re-read: bounded (once per UTC hour, skips digests without ages, goes through the single dispatch site, rides the existing tick counter). Defensible; test `the_owner_age_sweep_re_parses_aged_digests_once_an_hour` passes. Acceptance left to the owner (human item 4).
- WR-01: consistent with the D-09 "absent is not clean" principle; enforced on both write (`owner_digest_storable`) and read side (`owner_digest`); documented in README (blank cell row).
- D-09 divergence (unreadable owner files count as fix): matches the live reader; the plan/UI-SPEC wording was corrected in the plans.
- D-16 no colon: matches the CLI wording; cosmetic.

### Human Verification Required

See the `human_verification` frontmatter: cell wording at 100/120/140 columns and glyph alignment (ROADMAP human UAT), Owner block feel, the D-11 totals interpretation, and acceptance of the recorded deviations.

### Gaps Summary

No gaps. Every roadmap success criterion and every machine-checkable plan truth traces to real, wired, tested code, and no behavior-dependent truth is left without a passing test. The status is `human_needed` solely because the phase's own UAT (subjective wording, terminal glyph alignment) and four inferred-decision acceptances require the owner.

---

_Verified: 2026-11-14_
_Verifier: Claude (gsd-verifier)_
