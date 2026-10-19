---
phase: 28-sandboxed-driver-launcher
plan: 07
subsystem: ui
tags: [tui, driver-tab, launcher, ratatui, stop, frontmatter, rust]

requires:
  - phase: 28-sandboxed-driver-launcher
    provides: "28-02: launcher::stop::stop_run / StopReport; 28-04: DriverFrontmatter.sandbox / sandbox_missing"
  - phase: 27-driverctl-driver-frontmatter
    provides: "read_driver_run / DerivedStatus (status derived from the run lock)"
provides:
  - "state_reader::driver_frontmatter::{list_driver_runs, DriverRunEntry, DRIVER_RUNS_REL}"
  - "ProjectViewCache.launcher_runs / launcher_selected, filled by the existing off-thread Driver listing"
  - "Launcher runs section on the Driver tab with derived status and the UNSANDBOXED badge"
  - "{ / } / X keys, DriverAction::StopLauncher, DriverConfirmScreen::new_launcher_stop"
  - "Action::LauncherStopRequested / LauncherStopped and the app.rs spawn_blocking stop dispatch"
affects: [30]

plan_head_before: 0e82eb6e6974b386fea29d878934c8e6421d24c0
plan_head_after: 1d51ec2a5b53fb14f5956ccf0a81a7b5929c2231
commits: 3
actuals:
  tokens: 18500
  tasks: 2
  commits: 3

tech-stack:
  added: []
  patterns:
    - "Launcher rows ride the existing Driver listing's spawn_blocking and replace (never merge) in the cache"
    - "Section carved off the bottom of the pane only from rows the legacy tiers can spare, windowed around the highlight, collapsing to a one-line count"
    - "Run id lives on the confirmation screen so DriverAction stays Copy and one String feeds both prompt and dispatch"

key-files:
  created: []
  modified:
    - src/state_reader/driver_frontmatter.rs
    - src/ui/screens/mod.rs
    - src/ui/screens/driver.rs
    - src/ui/screens/detail.rs
    - src/ui/screens/driver_confirm.rs
    - src/ui/screens/render_escape_guard.rs
    - src/action.rs
    - src/app.rs

key-decisions:
  - "The rows and keys exist only where the Driver tab exists (GSDMM_EXPERIMENTAL_FEATURES gate); the stop handler returns silently when ctx.experimental is off, like stop_driver_run [inferred - audit]"
  - "Keys: } and { move the launcher highlight, capital X stops; the Driver footer's three measured width tiers are untouched and the keys are named on the section header instead [inferred - audit]"
  - "The section is carved from the bottom of the pane: it never takes the legacy tab's first 15 rows (4 when the legacy tab only has its 'no runs' block); a short pane shows a window that keeps the highlight in view, one spare row collapses to 'launcher runs: <n>' [inferred - audit]"
  - "The collapsed line adds '(<k> UNSANDBOXED)' when any run is degraded, so the posture is not hidden by the collapse [inferred - audit]"
  - "A run id is ellipsised before the status word or the badge is, so a narrow pane cannot clip the posture off [inferred - audit]"
  - "A finished stop re-lists the Driver runs unconditionally (one blocking read) rather than only when the tab is on top [inferred - audit]"
  - "The stop prompt quotes the alias like the other driver prompts: Stop launcher run <run-id> on \"<alias>\"? This stops its whole sandbox scope. [y/n]"

requirements-completed: [LNCH-01, LNCH-07]

coverage:
  - id: D1
    description: "The Driver tab lists the project's launcher run dirs newest first (max 20) with the status derived from the run lock, an unreadable DIRECTION.md as an 'unreadable' row beside the others"
    requirement: LNCH-07
    verification:
      - kind: unit
        ref: "src/state_reader/driver_frontmatter.rs#list_driver_runs_orders_newest_first_and_honours_the_limit"
        status: pass
      - kind: unit
        ref: "src/state_reader/driver_frontmatter.rs#list_driver_runs_derives_the_status_from_the_lock_not_the_written_word"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver.rs#launcher_rows_use_the_derived_status_not_the_written_one"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver.rs#launcher_an_unreadable_run_reads_unreadable_and_hides_no_other_row"
        status: pass
    human_judgment: false
  - id: D2
    description: "A run whose frontmatter says sandbox: degraded carries an UNSANDBOXED badge in the warning style; a bwrap-scope run carries none; a narrow pane keeps the badge"
    requirement: LNCH-07
    verification:
      - kind: unit
        ref: "src/ui/screens/driver.rs#launcher_rows_badge_a_degraded_run_and_not_a_sandboxed_one"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver.rs#launcher_a_narrow_pane_shortens_the_run_id_before_it_clips_the_badge"
        status: pass
    human_judgment: false
  - id: D3
    description: "Rows are loaded inside the existing off-thread listing and replaced wholesale; the legacy tiers render byte-identically when no row is spare"
    verification:
      - kind: unit
        ref: "src/app.rs#a_run_list_result_replaces_the_launcher_runs_never_merges"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver.rs#launcher_a_pane_with_no_spare_row_renders_exactly_the_legacy_tab"
        status: pass
    human_judgment: false
  - id: D4
    description: "{ and } move and clamp the launcher highlight, X opens the driver confirmation naming the highlighted run id, confirming sends LauncherStopRequested with that id and cancelling sends nothing; the keys are inert off the Driver tab"
    requirement: LNCH-01
    verification:
      - kind: unit
        ref: "src/ui/screens/detail.rs#launcher_stop_braces_move_the_highlight_and_clamp_at_both_ends"
        status: pass
      - kind: unit
        ref: "src/ui/screens/detail.rs#launcher_stop_capital_x_confirms_the_highlighted_run_and_the_request_names_it"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver_confirm.rs#launcher_stop_confirming_sends_the_confirmed_run_id_and_cancelling_sends_nothing"
        status: pass
      - kind: unit
        ref: "src/ui/screens/detail.rs#launcher_stop_keys_exist_only_on_the_driver_tab"
        status: pass
    human_judgment: false
  - id: D5
    description: "The confirmed stop runs launcher::stop::stop_run on spawn_blocking (the function driver stop calls), the outcome reaches the status line and the Driver runs are re-listed"
    requirement: LNCH-01
    verification:
      - kind: unit
        ref: "src/app.rs#launcher_stop_requested_runs_the_shared_stop_and_reports_through_launcher_stopped"
        status: pass
      - kind: unit
        ref: "src/app.rs#launcher_stop_stopped_sets_the_status_line_and_relists_the_driver_runs"
        status: pass
      - kind: integration
        ref: "tests/async_blocking_guard.rs"
        status: pass
    human_judgment: false
  - id: D6
    description: "Every run id, status word, badge and the stop prompt reaches the terminal escaped; the stop prompt and a hostile launcher row are probed by the render-escape guard"
    verification:
      - kind: unit
        ref: "src/ui/screens/render_escape_guard.rs#the_screen_renders_identity_escaped"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver.rs#launcher_a_hostile_run_id_and_status_word_reach_the_terminal_sanitized"
        status: pass
    human_judgment: false
  - id: D7
    description: "Whether the section, its header key hint and the chevron read well on a real terminal at common sizes"
    verification: []
    human_judgment: true
    rationale: "Layout adequacy at real terminal sizes is a subjective call; the tests prove placement, windowing and the collapse at fixed sizes only"

duration: 16min
completed: 2026-10-19
status: complete
---

# Phase 28 Plan 07: Launcher Runs on the Driver Tab Summary

**The existing Driver tab now lists a project's launcher runs with the status derived from the run lock and an `UNSANDBOXED` warning badge, and `{` / `}` / `X` stop the highlighted run through `launcher::stop::stop_run` on a blocking task.**

## Performance

- **Duration:** 16 min
- **Started:** 2026-10-19T20:49:55Z
- **Completed:** 2026-10-19T21:05:59Z
- **Tasks:** 2 (a tracer and a `tdd="true"` task), 3 commits (tracer `feat`, RED `test`, GREEN `feat`)
- **Files modified:** 8 (0 created); +1365 / -12 lines

## Accomplishments

- `list_driver_runs(project_root, limit)`: read-only, cross-platform, newest first, plain run ids only (a symlink named like a run id is skipped), a missing drivers dir is an empty list. It runs inside the Driver listing's existing `spawn_blocking` and returns through `Action::DriverRunsListed.launcher_runs`; the handler replaces the cache rows.
- `Launcher runs (n)` section on the Driver tab: `<run id>  <status>` with the status word from `DerivedStatus` (never the raw `status:`), an `UNSANDBOXED` bold-yellow span for `sandbox: degraded`, and `<run id>  unreadable` for an `Err` row. The section takes only rows the legacy tab can spare, shows a window that keeps the highlight in view, and collapses to `launcher runs: <n>` when one row is spare.
- `{` / `}` move the highlight (clamped, inert with no rows), `X` opens the driver confirmation `Stop launcher run <id> on "<alias>"? This stops its whole sandbox scope.`; confirming sends `LauncherStopRequested { alias, run_id }`, the app runs `stop_run(path, run_id, 60 s)` on `spawn_blocking`, and `LauncherStopped` sets `<alias>: <report>` on the status line and re-lists.
- The render-escape guard now probes the launcher stop prompt (a hostile alias and run id) and a hostile launcher row (run id, status word, degraded badge).

### TestBackend render excerpt (80x12, a degraded row highlighted, then sandboxed and unmarked)

```
Launcher runs (3)  [{/}]select  [X]stop
> 20261019-degraded  done  UNSANDBOXED
  20261018-sandboxed  done
  20261017-unknown  done
```

`20261018-sandboxed` carries `sandbox: bwrap-scope`, `20261017-unknown` has no `sandbox` key (written before the launcher existed); neither shows a badge.

## Task Commits

1. **Task 1 (tracer): launcher runs with derived status and the UNSANDBOXED badge** - `7a28611` (feat)
2. **Task 2 (tdd): select a launcher run and stop it**
   - RED `40d0d34` (test)
   - GREEN `1d51ec2` (feat)

**Plan metadata:** the docs commit carrying this file.

## TDD Gate Compliance

RED (`test(28-07)` `40d0d34`) precedes GREEN (`feat(28-07)` `1d51ec2`). RED added the Action variants, `DriverAction::StopLauncher`, `new_launcher_stop` and no-op handler arms so the tests compile; 14 of the 18 `launcher_stop` tests then failed on the planned assertions (the RED commit message says 13: the escaped-prompt test was given its positive assertion just before the commit): the highlight stayed at 0 after `}` (detail.rs), `X` pushed no screen, the stop prompt was an empty string, a confirmed `y` dispatched nothing, the unregistered-alias and no-channel refusals left `error_message` unset, `LauncherStopped` left `status_message` unset and sent no re-list, the highlight clamp test saw 5 instead of 0, the chevron row did not start with `> `. Four tests passed at RED by design: they characterise behaviour the plan had to keep (`X` and the braces inert with no rows, the keys inert off the Driver tab, the silent non-experimental no-op). The escaped-prompt test first passed vacuously against the empty stub prompt, so it was given a positive `starts_with("Stop launcher run ")` assertion and went red. `gsd_run check tdd-red-evidence` accepts TAP/JUnit/swift/unittest/go output only and `cargo test` output is none of those, so the machine classifier was not run; the semantic assessment is by hand as above (same limitation as 28-02 and 28-04). `workflow.tdd_mode` is not set. No REFACTOR commit. Task 1 is a tracer: one `feat` commit with its tests; its automated verify re-ran green before Task 2 started.

## Files Created/Modified

- `src/state_reader/driver_frontmatter.rs` - `list_driver_runs`, `DriverRunEntry`, `DRIVER_RUNS_REL`, 6 tests
- `src/ui/screens/mod.rs` - `launcher_runs` / `launcher_selected` cache fields, `LAUNCHER_RUNS_LISTED`, the listing calls `list_driver_runs`
- `src/ui/screens/driver.rs` - `render_driver_tab` split into the legacy tab plus the launcher section, row/badge/window/collapse rendering, 11 tests
- `src/ui/screens/detail.rs` - `{` `}` `X` arms (Driver tab only), 5 tests
- `src/ui/screens/driver_confirm.rs` - `StopLauncher`, `new_launcher_stop`, `do_stop_launcher`, prompt/title/colour, 4 tests, variant tables extended
- `src/ui/screens/render_escape_guard.rs` - launcher stop prompt state and a hostile launcher row in `probe_ctx`
- `src/action.rs` - `DriverRunsListed.launcher_runs`, `LauncherStopRequested`, `LauncherStopped`
- `src/app.rs` - `stop_launcher_run` (Linux), `project_path_for`, `LauncherStopped` handler, clamp, 8 tests

## Decisions Made

All `[inferred - audit]` unless the plan stated them (see `key-decisions` above). Beyond the plan's text:

- The prompt wording keeps the plan's sentence but quotes the alias, like the other driver prompts.
- The launcher section sits below the legacy tab rather than inside the run list pane, because the legacy tab returns early on an empty `driver_runs` and a launcher-only project (the normal case) must still list its runs. When the legacy tab has nothing but its `no runs` block it keeps 4 rows, otherwise 15 (the run detail's full height tier of 14 plus its title row).
- `X` is not gated on the run being live: stopping a finished run reports `the run is not running` through the same path as `driver stop`.
- The `LauncherStopped` report and the "Stopping launcher run" status are passed through `render_for_terminal`, since the report embeds a status word read back from the run dir.

## Deviations from Plan

**1. [Rule 2 - Missing critical] Windowing and the posture-preserving collapse**
- **Found during:** Task 1
- **Issue:** The plan said to collapse the section to one line when the rows do not fit. With the legacy tab keeping its tiers, a typical 24-row terminal leaves 5-9 spare rows, so a plain collapse would make the rows (and therefore the `X` selection) unreachable for any project with more than a handful of runs, and the collapsed line would hide an `UNSANDBOXED` run.
- **Fix:** show a window of rows that keeps the highlight on screen when at least a header and one row are spare; collapse to `launcher runs: <n>` only when exactly one row is spare, with a `(<k> UNSANDBOXED)` suffix when any run is degraded.
- **Files modified:** `src/ui/screens/driver.rs`
- **Verification:** `launcher_one_spare_row_collapses_the_section_to_a_count`, `launcher_a_short_pane_shows_a_window_that_keeps_the_highlight_in_view`
- **Committed in:** `7a28611`

**2. [Rule 3 - Blocking] Launcher-only projects hit the legacy early return**
- **Found during:** Task 1
- **Issue:** `render_driver_tab` returned the `no runs` block as soon as `driver_runs` was empty, which would have hidden every launcher run of a project that never used the Phase 17 driver.
- **Fix:** `render_driver_tab` is now a wrapper that carves the section off the bottom and delegates the rest to the unchanged body (`render_legacy_driver_tab`); the regions it reports come from the part the legacy tab was given.
- **Files modified:** `src/ui/screens/driver.rs`
- **Verification:** `launcher_a_project_with_only_launcher_runs_still_lists_them`, `launcher_a_pane_with_no_spare_row_renders_exactly_the_legacy_tab`, the 102 existing `ui::screens::driver*` tests
- **Committed in:** `7a28611`

**3. [Rule 2 - Missing critical] A named-key hint on the section header**
- **Found during:** Task 2
- **Issue:** `{`, `}` and `X` would have had no discoverable name: the Driver footer has three measured width forms pinned by test and no room for three more hints.
- **Fix:** the header reads `Launcher runs (n)  [{/}]select  [X]stop` at widths of 44 cells or more; the footer is untouched.
- **Files modified:** `src/ui/screens/driver.rs`
- **Committed in:** `1d51ec2`

**4. [Rule 3 - Blocking] `prompt_text` gained a `run_id` parameter**
- **Found during:** Task 2
- **Issue:** the prompt needs the run id and `DriverAction` must stay `Copy`.
- **Fix:** a fifth `&str` argument (ignored by every other action); the existing call sites in the tests pass `""`.
- **Files modified:** `src/ui/screens/driver_confirm.rs`
- **Committed in:** `40d0d34`

**Total deviations:** 4 (2 Rule 2, 2 Rule 3). **Impact:** none on the plan's contract; the section, keys and stop path are as specified.

## Issues Encountered

- `driverctl::serve::tests::wr05_a_live_socket_is_never_replaced` failed once in the first full `cargo test --no-fail-fast` run and passed in 3 isolated runs and 2 further `--lib` runs. The file (`src/driverctl/serve.rs`) is untouched by this plan; it reads as a contention flake under the full parallel run. Not investigated further (out of scope).

## Verification

- Full `cargo test --no-fail-fast` (first run): 3487 passed, 2 failed, 22 ignored. The two failures are the expected local `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against` and the one-off `wr05` flake above. `cargo test --lib` re-run twice: 2409 passed, 1 failed (the git-version constants), 1 ignored.
- Plan verify set, all green: `--lib ui::screens::driver` (102), `--lib list_driver_runs` (5), `--lib action::` (2), `--lib launcher_stop` (18), `--lib ui::screens::driver_confirm` (27), `--lib ui::screens::render_escape_guard` (17), `--lib ui::screens` (630 before Task 2), `--test async_blocking_guard` (9), `--test spawn_seam_guard` (41).
- `cargo clippy --all-targets -- -D warnings`: clean. `cargo fmt --check`: clean.
- Acceptance greps: `pub fn list_driver_runs` 1; `UNSANDBOXED` in driver.rs 11; `launcher_runs` in screens/mod.rs 4; `StopLauncher` in driver_confirm.rs 14; `LauncherStopRequested` in action.rs 1; `spawn_blocking` in app.rs 15 -> 17.
- **Mutation check (not committed).** Replacing `render_for_terminal(run_id)` in the stop prompt and `shown_capped(&entry.0)` in the row with the raw values made `render_escape_guard::the_screen_renders_identity_escaped` fail; restored.

## User Setup Required

None - no external service configuration required.

## Pending live

None. Nothing in this plan needs bwrap, systemd or the network: the stop path is exercised against a real run dir with a free lock (`the run is not running`); the live scope kill it reaches is 28-02's `tests/launcher_live_scope.rs` (`cargo test --test launcher_live_scope -- --ignored --nocapture --test-threads=1`). The non-Linux arm of the `LauncherStopRequested` handler (sets the unsupported-platform error) cannot be compiled on this host; verify with `cargo check --target aarch64-apple-darwin` on a macOS runner.

## Known Stubs

None.

## Threat Flags

None. The surfaces added are the plan's T-28-18 (the confirmation names the id it sends, one `String` feeds prompt and dispatch, `stop_run` re-validates the run dir and acts only through the run's own lock) and T-28-19 (`shown_capped` on every row value, `render_for_terminal` on the prompt and status line, both probed by the render-escape guard, mutation-checked).

## Next Phase Readiness

- 28-08/28-09 are unaffected. Phase 30 re-hosts the section, the badge and the keys in the new Driver tab; the data side (`list_driver_runs`, `DriverRunEntry`, `LauncherStopRequested`) is tab-independent.

## Self-Check: PASSED

Modified files exist; commits `7a28611`, `40d0d34`, `1d51ec2` are ancestors of HEAD; the plan verify commands exit 0.
