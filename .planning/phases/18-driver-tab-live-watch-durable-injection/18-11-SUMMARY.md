---
phase: 18-driver-tab-live-watch-durable-injection
plan: 11
subsystem: ui
tags: [ratatui, tui, help-screen, scroll-clamp, dry-run, blast-radius, spawn-blocking, leak-pruning]

# Dependency graph
requires:
  - phase: 18-06
    provides: "The durable inbox at `RunPaths::inbox` and `inbox::append`, which is what makes `tests/journal_gitignore.rs`'s inbox assertions about a file that actually exists"
  - phase: 18-10
    provides: "The Driver tab's key set, the four injection glyphs and labels, and the run-detail pane the preview replaces"
  - phase: 18-09
    provides: "`clamp_scroll`, `ViewportMetrics` and `PAGE_SCROLL_LINES` widened to `pub(super)` — the one clamp formula the help popup now shares"
  - phase: 18-05
    provides: "`schedule_dry_run_report` and the `Action::DriverDryRunLoaded` seam, with the store deliberately left to this plan"
  - phase: 17-supervisor-detach-kill-switch-dry-run-opt-in-gate
    provides: "`dry_run::build_report` / `render` and the three pinned section headers; D-24 left the TUI surfacing to Phase 18"
provides:
  - "A scrollable, grown help popup — the only place keys are documented, now able to display what it documents"
  - "`help_lines()` — the whole help body as a pure, assertable function"
  - "A five-glyph dashboard badge legend and a four-state injection legend, both built from the shipped constants rather than retyped literals"
  - "`render_dry_run_preview` — blast radius shown in the run-detail pane before a start is confirmed (D-26)"
  - "`ProjectViewCache.driver_dry_run` and `DryRunPreview`, the store the 18-05 seam was waiting for"
  - "`AppContext::schedule_dry_run_report` — the `spawn_blocking` build reachable from a `Screen`"
  - "`prune_driver_maps` extended to `last_refresh` and `archive_cache`: every alias-keyed map on `AppContext` is now pruned"
  - "`every_per_alias_driver_map_is_pruned` — an exhaustive `AppContext` destructure that makes a new unpruned map a compile error"
affects: [phase-19-git-blast-radius, phase-20-decision-router, phase-21-goal-decomposition]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "A scrolling popup shares `clamp_scroll` / `ViewportMetrics` with every other scrolling pane — one clamp formula, imported, never re-derived"
    - "A more-content indicator is gated by a comparison (`more_below`), deliberately not by a second max-scroll subtraction"
    - "Legends interpolate the shipped glyph and label constants, so documentation cannot drift from what renders"
    - "A one-`Option` field where presence *is* the mode, rather than a value plus an `active` flag that can disagree with it"
    - "A cross-cutting invariant proved by exhaustive struct destructuring, so a new field is a compile error until it has been classified"

key-files:
  created: []
  modified:
    - src/ui/screens/help.rs
    - src/ui/screens/driver.rs
    - src/ui/screens/driver_start.rs
    - src/ui/screens/mod.rs
    - src/app.rs
    - src/ui/screens/normal.rs
    - src/ui/screens/detail.rs
    - tests/journal_gitignore.rs

key-decisions:
  - "The help popup's more-content indicator is gated by `more_below(offset, total, visible)` — a comparison — rather than by re-deriving `total - visible`. A third copy of the max-scroll expression is exactly how UIFIX-04 came back the first time, so the file contains no arithmetic of that shape at all"
  - "Key documentation is asserted as WHOLE ROWS built through the same `row()` helper the body uses. `text.contains(\"x\")` is satisfied by the word \"next\" and `contains(\"s\")` by almost anything — a substring check on a one-letter key is the same class of vacuous gate this plan's own criteria were written to avoid"
  - "The injection gloss is split across two rendered rows with a `INJECTION_LEGEND_CONT` companion array, and the test asserts on the whitespace-collapsed body — so the sentence stays honest at 80 columns without a soft wrap that would desynchronise the scroll offset from the rendered line count"
  - "`ProjectViewCache.driver_dry_run` is a single `Option`: presence is the mode. A value plus a separate `active` flag is two fields that must agree, which is two fields that can disagree"
  - "The preview is opened in LOADING and the build is scheduled second. `schedule_dry_run_report` returns immediately, so a pane that only learned about the preview when the report arrived would never render the loading idiom it is specified to render"
  - "`Action::DriverDryRunLoaded` stores under two guards — a preview must be open, and its command must match. A stale report would show one command's blast radius under another command's name, which is the display disagreeing with the disk in the direction that flatters the run"
  - "`schedule_dry_run_report` moved from `App` to `AppContext` with `App` delegating, matching the `schedule_run_list_scan` shape: a `Screen` is handed an `&mut AppContext` and never an `&mut App`, so the alternative was two copies of a `spawn_blocking` closure that would drift"
  - "The preview survives the empty-run-list branch. A project with no runs at all is the *most* likely place to be starting one, and showing \"no runs yet\" over a start the user is one keystroke from confirming would hide the blast radius exactly when it is newest"
  - "The preview says `▾ more` rather than clipping silently — a pane that drops the refspec section is the PITFALLS:69 failure the preview exists to prevent"
  - "[Rule 2] `prune_driver_maps` also prunes `last_refresh` and `archive_cache`, so \"every per-alias map is pruned\" is literally true rather than true of the driver maps and quietly false of two neighbours"

patterns-established:
  - "Whole-row key assertions: documentation of a one-character key is proved by matching the rendered row, never by `contains(key)`"
  - "Presence-is-the-mode state: one `Option` field replaces a value plus a boolean that could contradict it"
  - "Exhaustive destructure as a completeness gate: `let Struct { a, b, c: _, .. } = &value;` with no `..`, so a new field forces a decision at the invariant's test site"
  - "Non-vacuity by construction in ignore tests: the file must be written through the production writer at the library's own path, and asserted to exist, before its absence from the git index is evidence"

requirements-completed: [TRANS-05, OBS-04, OBS-07]

coverage:
  - id: D1
    description: "The help screen documents every key this phase binds — the Driver section, the sort toggle and both needs-a-human filter forms — and can display all of it: the popup grew to 80x90, scrolls with the UIFIX-04 clamp ordering, and shows a more-content indicator"
    requirement: OBS-04
    verification:
      - kind: unit
        ref: "src/ui/screens/help.rs#help_lines_documents_every_key_this_phase_binds, #the_body_is_long_enough_that_the_scroll_work_was_required"
        status: pass
      - kind: unit
        ref: "src/ui/screens/help.rs#page_down_then_page_up_returns_to_the_top, #page_up_at_the_top_stays_at_the_top, #scrolling_past_the_end_clamps_and_the_first_page_up_still_moves, #the_more_indicator_appears_only_while_content_extends_below_the_viewport"
        status: pass
    human_judgment: false
  - id: D2
    description: "The help screen carries a five-glyph badge legend and a four-state injection legend whose gloss says honestly that a delivered message may take about a minute to be picked up and that acted-on means the agent dequeued it"
    requirement: OBS-07
    verification:
      - kind: unit
        ref: "src/ui/screens/help.rs#the_badge_legend_names_all_five_dashboard_glyphs_by_their_constants, #the_injection_legend_names_all_four_states_and_carries_the_honest_gloss, #the_help_screen_never_says_sent_received_read_or_acknowledged"
        status: pass
    human_judgment: false
  - id: D3
    description: "The dry-run report is visible in the run-detail pane before a start is confirmed, built off the render thread on spawn_blocking, sanitised before rendering, with the shipped loading idiom until it resolves and inert keys throughout"
    requirement: TRANS-05
    verification:
      - kind: unit
        ref: "src/ui/screens/driver.rs#the_preview_pane_renders_the_loading_idiom_before_the_report_resolves, #a_loaded_report_renders_its_three_pinned_section_headers, #a_report_carrying_an_escape_sequence_is_sanitised_before_rendering, #a_report_taller_than_the_pane_says_so_rather_than_clipping_silently, #the_preview_replaces_the_run_detail_and_restores_it_when_it_is_cleared"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver_start.rs#reaching_step_b_opens_a_loading_preview_for_the_committed_command, #keys_pressed_while_the_report_is_loading_are_inert_and_nothing_panics, #leaving_step_b_in_any_direction_restores_the_normal_run_detail, #a_project_that_never_opted_in_is_refused_visibly_and_shows_no_preview"
        status: pass
      - kind: unit
        ref: "src/app.rs#a_dry_run_report_is_stored_only_under_the_preview_it_was_built_for"
        status: pass
      - kind: integration
        ref: "cargo test --test driver_dry_run (4 passed — the report builder itself is unchanged)"
        status: pass
    human_judgment: false
  - id: D4
    description: "Every per-alias and per-run map Phase 18 added is pruned, discharging the phase's only remaining Phase 16 carry-forward — and a new unpruned map is now a compile error rather than a thing to remember"
    verification:
      - kind: unit
        ref: "src/app.rs#every_per_alias_driver_map_is_pruned"
        status: pass
      - kind: unit
        ref: "src/app.rs#pruning_drops_the_output_buffer_for_an_unregistered_alias, #pruning_drops_the_view_cache_for_an_unregistered_alias"
        status: pass
    human_judgment: false
  - id: D5
    description: "The phase's mechanical close-out: the project gate passes, the --all-targets clippy count is still exactly 5, the suite is 749 passing with zero failures, no Cargo dependency was added, and the four MUST-FIX carry-ins each have a passing named regression test"
    verification:
      - kind: other
        ref: "rtk proxy sh -c \"cargo build && cargo test && cargo clippy -- -D warnings\" — green"
        status: pass
      - kind: other
        ref: "rtk proxy sh -c \"cargo clippy --all-targets 2>&1 | grep '^warning: ' | grep -v generated | wc -l\" → 5 (browser.rs:131/132/133, project_creator.rs:146, state_reader/mod.rs:258)"
        status: pass
      - kind: other
        ref: "git diff --stat 2e33871 -- Cargo.toml Cargo.lock → empty; grep tui_textarea/TextArea across the repo → 0, unchanged from the phase base"
        status: pass
      - kind: integration
        ref: "cargo test --test journal_run_paths --test driver_lock --test spawn_seam_guard (2 + 5 + 7 passed); cargo test --lib app:: names all four stop-disposition cases"
        status: pass
      - kind: other
        ref: "rtk proxy cargo build --release — succeeds"
        status: pass
    human_judgment: false
  - id: D6
    description: "The grown help popup is actually readable at an 80x24 terminal — the two legends, the scroll indicator and the injection gloss land as a coherent page rather than as a wall of rows"
    verification: []
    human_judgment: true
    rationale: "Whether ~60 rows of key documentation across five headed sections reads as scannable or as a wall is a perceptual judgement about a real terminal at a real size; the tests can prove every row is present and that the popup scrolls, but not that a reader finds what they came for."

# Metrics
duration: 45min
completed: 2026-07-29
status: complete
---

# Phase 18 Plan 11: A Readable Help Screen, the Dry-Run Preview and the Phase's Mechanical Close-Out Summary

**A help popup that grew, scrolls through the shared UIFIX-04 clamp and now carries two constant-derived legends; the dry-run report shown in the run-detail pane before a start is confirmed, built off the render thread; and the phase's promises about lint count, dependencies and unpruned maps each closed by a check whose output is real.**

## Performance

- **Duration:** ~45 min
- **Started:** 2026-07-29T20:44Z (local; first task commit at 20:51:41-07:00)
- **Completed:** 2026-07-29T21:06:40-07:00 (final task commit)
- **Tasks:** 3
- **Files modified:** 8

## Accomplishments

- **`help.rs` can hold what the phase added.** It was a hardcoded `Vec<Line>` in a `centered_rect(area, 60, 70)` overlay with **no scrolling**, already ~30 rows deep — at 80×24 the popup is 48×16, so roughly half was invisible before anything was added. It is now 80×90, scrolls on `j`/`k`/`Up`/`Down`/`PageUp`/`PageDown` through the **shared** `clamp_scroll`, and shows `▾ more` in DarkGray on the last visible row while content extends below. `help_lines()` is extracted as a pure function so the content is assertable on spans rather than through a rendered buffer, where a missing row and a clipped one look identical.
- **The content, sourced from the shipped constants.** A Driver key section (`Shift+D`, `j`/`k`, `PgUp`/`PgDn`, `f`, `G`, `i`, `s`, `x`), the dashboard sort toggle, the `/term/h` and `//h` filter rows, a five-glyph badge legend and a four-state injection legend. Both legends interpolate `BADGE_*` / `GLYPH_*` / `LABEL_*` rather than retyped literals, so a renamed glyph fails a test instead of silently making the documentation wrong (T-18-63).
- **The injection gloss is honest and pinned.** `delivered` says the message was written to the agent's stdin and *it may be a minute before the agent picks it up*; `acted-on` says the agent **dequeued** it and is running it as its own turn. `sent`, `received` and `acknowledged` are forbidden by test, and `read` is allowed exactly once — in the queued gloss, which says nothing has read it. No spinner, no animation.
- **Blast radius before the start (D-26, TRANS-05).** While `DriverStartScreen` is at Step B, the body's run-detail pane renders the dry-run report for the committed command **in place of** the run detail: the GSD command sequence, the working tree a commit would capture, and the push refspecs the current state would produce. Zero new keys, zero new modes.
- **Built off the render thread, and provably so.** `schedule_dry_run_report` moved to `AppContext` (with `App` delegating) so a `Screen` can reach it; the build still runs on `spawn_blocking` and returns through `Action::DriverDryRunLoaded`. `grep build_report src/ui/screens/driver_start.rs src/ui/screens/driver.rs` returns nothing — no screen reaches the blocking builder at all.
- **Sanitised, and honest about its own limits.** Every report row goes through `sanitize_render_line` before rendering, because the report interpolates paths and branch names read from the project (T-18-61). A report taller than the pane says `▾ more` rather than dropping the refspec section, which is the PITFALLS:69 failure the preview exists to prevent.
- **The Phase 16 carry-forward, discharged mechanically.** `prune_driver_maps` now covers `last_refresh` and `archive_cache` too, and `every_per_alias_driver_map_is_pruned` destructures `AppContext` exhaustively — so adding a field is a compile error at that test until someone has classified it. The obligation stops depending on memory.
- **The gates ran through `rtk proxy`.** Every counting criterion was measured through the proxy, because plain `cargo` output is filtered by a summarising wrapper and a bare grep passes vacuously — the failure that occurred in Phase 15 and Phase 17 (T-18-64).

## Task Commits

1. **Task 1: A help screen that can hold what this phase adds** — `d1124fb` (feat)
2. **Task 2: Surface the dry-run report before a start is confirmed** — `392902d` (feat)
3. **Task 3: Mechanical close-out** — `26b8563` (test)

## Files Created/Modified

- `src/ui/screens/help.rs` — rewritten: grown popup, scroll state, `help_lines()`, `more_below()`, the Driver section, the two filter rows, both legends, nine inline tests
- `src/ui/screens/driver.rs` — `render_dry_run_preview` and its two call sites in `render_driver_tab`; `DRY_RUN_LOADING`; eight glyph/label constants widened to `pub(super)`; five inline tests
- `src/ui/screens/driver_start.rs` — `open_dry_run_preview` / `close_dry_run_preview`, wired into all four Step transitions; the Step B module doc; four inline tests
- `src/ui/screens/mod.rs` — `DryRunPreview`, `ProjectViewCache.driver_dry_run`, `AppContext::schedule_dry_run_report`
- `src/app.rs` — `App::schedule_dry_run_report` reduced to a delegation; `Action::DriverDryRunLoaded` stores under two guards; `prune_driver_maps` extended; two inline tests
- `src/ui/screens/normal.rs` — the five `BADGE_*` constants widened to `pub(super)`; `HelpScreen::new()`
- `src/ui/screens/detail.rs` — `PAGE_SCROLL_LINES` widened to `pub(super)`; `HelpScreen::new()`
- `tests/journal_gitignore.rs` — the inbox is written through `inbox::append` at `RunPaths::inbox` and asserted to exist and be non-empty before its absence from the index is claimed as evidence

## Mechanical close-out results (Task 3, each measured through `rtk proxy`)

| # | Check | Result |
|---|---|---|
| 1 | Project gate — `cargo build && cargo test && cargo clippy -- -D warnings` | **pass** |
| 2 | `cargo clippy --all-targets` warnings, `generated` lines excluded | **exactly 5** — `browser.rs:131/132/133`, `project_creator.rs:146`, `state_reader/mod.rs:258`. Unchanged, and not this phase's to shrink |
| 3 | `cargo test` totals | **749 passed, 0 failed** (646 lib + 103 across the integration binaries). Baseline entering the plan was 626 lib; the plan added 20 |
| 4 | `git diff --stat 2e33871 -- Cargo.toml Cargo.lock` | **empty** — zero dependencies added across the whole phase. `tui_textarea` / `TextArea` call sites in the repo: **0**, identical to the phase base. MSRV `1.87` and edition `2021` unchanged |
| 5 | Every per-alias / per-run map pruned | **`every_per_alias_driver_map_is_pruned` passes.** Seven maps asserted with a control arm each: `journal_cursors`, `run_states`, `observed_runs`, `driver_output`, `view_cache`, `last_refresh`, `archive_cache` |
| 6 | `tests/journal_gitignore.rs` inbox assertions non-vacuous | **pass** — the inbox is written through the production appender and asserted to exist and be non-empty before its absence from the index is claimed |
| 7 | The four MUST-FIX carry-ins | **all present and passing** — see below |
| 8 | `cargo build --release` | **succeeds** |

### The four MUST-FIX carry-ins

| Carry-in | Named regression test | Binary | Result |
|---|---|---|---|
| Path traversal, both directions | `a_traversing_run_id_creates_nothing_outside_the_runs_root_and_exits_non_zero`, `an_active_pointer_naming_a_traversing_id_causes_no_read_outside_the_runs_root` | `tests/journal_run_paths.rs` | 2 passed |
| Non-blocking lock acquisition | `acquiring_the_run_lock_does_not_block_the_async_runtime`, `a_duplicate_start_refuses_promptly_rather_than_blocking` | `tests/driver_lock.rs` | 5 passed |
| The four stop dispositions | `a_run_that_exited_on_terminate_is_dropped_from_both_maps`, `a_run_that_exited_after_the_uncatchable_signal_is_dropped_from_both_maps`, `a_stop_that_signalled_nothing_keeps_both_map_entries`, `a_stop_whose_signal_was_never_delivered_keeps_both_map_entries` | `src/app.rs` (lib) | 4 passed |
| The debug-only override guard | `the_agent_program_override_fields_are_debug_only`, `an_override_field_declared_without_the_debug_gate_is_reported` | `tests/spawn_seam_guard.rs` | 7 passed |

## Decisions Made

See `key-decisions` in the frontmatter. The three most load-bearing:

1. **One clamp formula, and no arithmetic shaped like a second one.** The help popup imports `clamp_scroll` / `ViewportMetrics` / `PAGE_SCROLL_LINES` from `detail.rs` and gates its more-content indicator on a **comparison** (`more_below`) rather than on a re-derived `total - visible`. `saturating_sub` appears in the file only as the UIFIX-04 up-direction subtraction — clamp first, subtract second — exactly as `detail.rs` does it.
2. **Whole-row key assertions.** The plan's own acceptance criteria warned about vacuous gates; the first draft of `help_lines_documents_every_key_this_phase_binds` was one, because `contains("x")` is satisfied by the word "next". The test now builds each expected row through the same `row()` helper the body uses and asserts a line equals it, so it proves *this key, with this meaning, on its own row*.
3. **Presence is the mode.** `ProjectViewCache.driver_dry_run: Option<DryRunPreview>` carries both the "a preview is open" fact and the loading state (`report: None`). A value plus a separate `active` flag would be two fields that must agree, which is two fields that can disagree.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] `prune_driver_maps` extended to `last_refresh` and `archive_cache`**

- **Found during:** Task 3, while writing `every_per_alias_driver_map_is_pruned`
- **Issue:** The plan required proving that "every per-alias map this phase added is pruned". Enumerating `AppContext`'s map fields to write that proof surfaced two alias-keyed maps that predate the phase and have **no removal site at all**: `last_refresh` (one `Instant` per alias ever watched) and `archive_cache` (a whole parsed milestone archive per alias ever browsed). Both grow for the process lifetime. Writing a test titled "every per-alias driver map is pruned" that quietly excluded two neighbouring alias-keyed maps would have been the same class of half-true gate the plan's `rtk` warning is about.
- **Fix:** Two `retain` lines in `prune_driver_maps`, with the reasoning recorded inline — both are caches, so dropping an entry costs at most one re-derivation for a project that no longer exists.
- **Files modified:** `src/app.rs`
- **Verification:** `every_per_alias_driver_map_is_pruned` asserts both, each with a registered-alias control arm
- **Committed in:** `392902d` (Task 2 commit — the prune change landed alongside the `driver_dry_run` field it also had to cover)

**2. [Rule 2 - Missing Critical] A more-content indicator on the dry-run preview**

- **Found during:** Task 2
- **Issue:** The preview renders a multi-section report into a pane that is frequently shorter than it. Silently clipping would drop the push-refspec section — which `driver/dry_run.rs` names, citing PITFALLS:69, as *the* warning sign that a dry-run has stopped being a dry-run.
- **Fix:** Reused `help::more_below` and `help::MORE_INDICATOR` rather than adding a second predicate or a second glyph.
- **Files modified:** `src/ui/screens/driver.rs`
- **Verification:** `a_report_taller_than_the_pane_says_so_rather_than_clipping_silently`
- **Committed in:** `392902d`

**3. [Rule 2 - Missing Critical] The preview survives the empty-run-list branch**

- **Found during:** Task 2
- **Issue:** `render_driver_tab` returns early with "No runs yet for …" when the run list is empty. A project with **no runs at all** is the most likely place to be starting one, so the preview would have been invisible in precisely the case it matters most.
- **Fix:** The empty branch checks for a preview first and renders it inside the same bordered block.
- **Files modified:** `src/ui/screens/driver.rs`
- **Verification:** covered by the loading-idiom and pinned-header tests, which render through the empty-list path
- **Committed in:** `392902d`

**4. [Rule 2 - Missing Critical] `Action::DriverDryRunLoaded` guards on the command, not only on the alias**

- **Found during:** Task 2
- **Issue:** The plan specified a store. A store keyed on alias alone would park a report built for command A under a preview about command B, if the user pressed `Esc` and retyped while the first build was in flight — showing one command's blast radius under another command's name.
- **Fix:** Two guards — a preview must be open, and `preview.command == command`.
- **Files modified:** `src/app.rs`
- **Verification:** `a_dry_run_report_is_stored_only_under_the_preview_it_was_built_for` exercises all three arms (no preview, stale command, matching command)
- **Committed in:** `392902d`

---

**Total deviations:** 4 auto-fixed (4 missing-critical). **Impact on plan:** none negative. Three harden the surface the plan specified; one closes a leak the plan's own proof obligation exposed. No scope creep — nothing outside `<domain>` was touched, and the deliberately-deferred items (the dead `src/ui/project_list.rs`, the three duplicate `status_color` implementations, the 5 pre-existing clippy lints) were left alone.

**The designated cut was NOT taken.** D-26 marked the dry-run preview as the first thing to drop if it competed for space. It did not: it landed in one `match` arm plus one `Option` field, with the cut rule recorded in `render_dry_run_preview`'s doc adjacent to the signature, so dropping it later is still a one-arm change.

## Issues Encountered

- **`tests/driver_reattach.rs::a_fresh_scan_finds_the_orphaned_run_live_with_its_last_journal_step` failed once**, during the first whole-suite run, at the `observed.len() == 1` assertion (`left: 0`). It then passed 3/3 in isolation and 3/3 as a full binary, and the whole suite has been green on every run since. The test spawns a real driver binary and races a `live_within(pid, …, 30s)` probe against a fresh reconciliation scan; under the load of ~18 test binaries running in parallel that race can lose. **Pre-existing and unrelated to this plan** — nothing in the three commits touches the driver, the reconciler or the journal reader. Logged rather than fixed: it is a test-isolation flake in Phase 17 territory, and the scope fence puts orphan sweeping and run bounds in Phase 20.
- **`cargo fmt --check` reports ~286 diffs across the repository**, essentially all pre-existing. `cargo fmt` is not part of the project gate (`cargo build && cargo test && cargo clippy -- -D warnings`), so running it would have produced a repo-wide reformat masquerading as this plan's diff. The three spots rustfmt flagged in *new* code were fixed by hand; the pre-existing drift was left alone.
- **`tui-textarea` is declared in `Cargo.toml` with zero call sites anywhere in the repository** — before this phase and after it. The plan's criterion ("gains no new call site") holds at 0 → 0. Noting it because "an unused direct dependency" is a real, separate finding, and it is not this plan's to remove.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- **Phase 18 is complete.** All eleven plans have landed; TRANS-05, OBS-04 and OBS-07 are closed by this plan, and the phase's nine requirements are done.
- **Ready for Phase 19 (git blast radius).** The dry-run report is now *visible* in the TUI at the moment before a start is confirmed, which is exactly the surface Phase 19's enforcement attaches to: push allowlists, `--disallowedTools`, pre-push hooks, secret scanning and worktree isolation. Phase 18 surfaced it and deliberately did not police it, and that fence is recorded in `render_dry_run_preview`'s doc so the next reader cannot undo it by accident.
- **Ready for Phase 20 (decision router).** `dry_run::build_report` still returns exactly one command and *says so* in its output; when the router makes the sequence longer, the preview renders it with no change — the pane shows `render()`'s text verbatim.
- **The Phase 16 carry-forward is discharged and made durable.** A future phase that adds an alias-keyed field to `AppContext` will fail to compile `every_per_alias_driver_map_is_pruned` until it has decided whether that field needs pruning.
- **One item for `/gsd:verify-work`:** D6 — whether the grown help popup reads as a scannable page at 80×24 rather than as a wall of rows. Everything else in this plan auto-passes.

## Self-Check: PASSED

All eight modified files exist on disk; all three task commits (`d1124fb`, `392902d`, `26b8563`) are present in `git log`.

---
*Phase: 18-driver-tab-live-watch-durable-injection*
*Completed: 2026-07-29*
