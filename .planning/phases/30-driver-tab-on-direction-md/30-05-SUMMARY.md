---
phase: 30-driver-tab-on-direction-md
plan: 05
subsystem: ui
tags: [rust, ratatui, tui, launcher, launch-form, relaunch, new-run, unsandboxed-confirmation, e2e]

requires:
  - phase: 30-driver-tab-on-direction-md
    provides: 30-01 launcher-first tab, DriverKeys footer and the kill confirmation; 30-02 the cached RunDetail and item cursor; 30-04 the steering writes, App::edit_sessions / after_editor / finish_edit and ControlChooserScreen
  - phase: 28-driver-launcher
    provides: launcher::run::{launch_detached, launch_argv, LaunchOptions}, launcher::host::{detect, decide}, the fake claude/bwrap/systemd-run shims in tests/launcher_support
  - phase: 29-driver-master-prompt
    provides: driver_templates::{create_run_dir, mint_run_id, SLUG_RE, lint::lint_direction} and the DIRECTION.md lint the launcher preflight applies
provides:
  - "cli::{launch_defaults, parse_launch_field}: the launch form's defaults and per-field verdicts come from clap's own LaunchArgs parser"
  - "ui::screens::driver_launch: launch_kind_for, relaunch_prefill, launch_options_for, launch_preflight, launch_error_copy, slug_run_id and the LaunchFormScreen / UnsandboxedConfirmScreen / SlugInputScreen / LintPopupScreen screens"
  - "app::LaunchSeams {launch, detect} (production = launch_detached + host::detect) as an injectable App field, and the DriverLaunch* / DriverNewRun* actions"
  - "Driver-tab keys R (launch / relaunch) and s (new run: slug, create, edit, check, launch); the legacy command-and-goal start flow is unreachable from the tab"
  - "tests/driver_tab_launch_e2e.rs: the argv the tab builds, run through the real binary with the fake shims (zero spend)"
affects: [31, verification]

actuals:
  tokens: 52000
  tasks: 3
  commits: 4
plan_head_before: 896e21ed012a7d670ff3ecbc28d574a00b885fa1
plan_head_after: 15bcda21d45acd84512ff08622612a73227c3465

tech-stack:
  added: []
  patterns:
    - "the launch form never launches: it dispatches plain-data LaunchArgs and the App builds LaunchOptions and calls the launcher through an injectable fn-pointer seam on spawn_blocking"
    - "a screen that hands off to a confirmation which dispatches uses ScreenAction::Replace, so neither answer returns to a screen that can be submitted again"
    - "the new-run flow rides the 30-04 editor flow through App::new_run_flow plus EditSession::then_launch, and ends on a clean driver check, n, an editor failure, a discard or a launch"
    - "defaults and edited values are judged by re-parsing through the same clap parser the CLI uses, joined as --flag=value so a value that looks like a flag is a value"

key-files:
  created:
    - src/ui/screens/driver_launch.rs
    - tests/driver_tab_launch_e2e.rs
  modified:
    - src/cli.rs
    - src/action.rs
    - src/app.rs
    - src/driver/steer.rs
    - src/ui/screens/mod.rs
    - src/ui/screens/detail.rs
    - src/ui/screens/driver_run.rs
    - src/ui/screens/driver_steer.rs
    - src/ui/screens/help.rs
    - src/ui/screens/render_escape_guard.rs

key-decisions:
  - "The in-process launch_detached call is covered by the LaunchSeams App tests (fake seam records the LaunchOptions) plus the binary e2e with the same argv, not by an in-process call: it re-executes current_exe, so a unit test would start the test binary (RESEARCH Pitfall 5, Assumption A5)."
  - "U and c replace the form instead of stacking on it (the WR-03 rule): after the unsandboxed confirmation or the control chooser the user is back on the tab, never on a form they could submit a second time."
  - "A refusal whose text is already a row of the form (claude refusal, control row, lint headline) is not printed twice; only the sandbox-missing refusal adds a row, because its Red rows say less than the refusal copy."
  - "The new-run flow's flag lives in App::new_run_flow keyed by (alias, run id) and is copied into EditSession::then_launch when the edit temp is ready, so DriverEditRequested / DriverEditReady (30-04) keep their shape."

patterns-established:
  - "launch_error_copy / sandbox_refused_copy / created_copy / exists_copy / left_unlaunched_copy are pure copy functions, so every string is asserted without an App"
  - "App tests drive R / s through App::handle_key and App::update with fn-pointer fakes recording into a static keyed by the project path"

requirements-completed: [DTAB-04]

coverage:
  - id: D1
    description: "R on a run that derives Crashed (or is written stopped, parked, stalled, paused or crashed) opens the launch form prefilled from the run's recorded caps and the driver connection's model, and its confirmation relaunches the SAME run id through the launcher seam sandboxed; R on a never-launched run opens the same form with the CLI defaults; R is refused with the lock-held and liveness-unknown copies, and on a done run."
    requirement: "DTAB-04"
    verification:
      - kind: e2e
        ref: "tests/driver_tab_launch_e2e.rs#a_crashed_run_relaunches_with_the_argv_the_tab_builds"
        status: pass
      - kind: e2e
        ref: "src/app.rs#launcher_launch_r_then_y_relaunches_the_same_run_through_the_seam"
        status: pass
      - kind: e2e
        ref: "src/app.rs#launcher_launch_r_on_a_new_run_opens_the_form_with_the_defaults"
        status: pass
      - kind: e2e
        ref: "src/app.rs#launcher_launch_r_refuses_a_running_run_and_opens_the_defaults_for_a_new_one"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver_launch.rs#launcher_launch_kind_over_every_derived_status"
        status: pass
    human_judgment: false
  - id: D2
    description: "The form shows model, loop mode, spend ceiling, dispatches, minutes and gap rounds; defaults and every edited value come from clap's LaunchArgs parser (no TUI copies); an invalid value shows 'must be a positive number' and editing continues; y and n are text while editing; a recorded model is used only when it matches the conservative charset; a used-up wall clock shows a raise hint."
    requirement: "DTAB-04"
    verification:
      - kind: unit
        ref: "src/cli.rs#launch_field_values_are_judged_by_the_launch_parser"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver_launch.rs#launcher_launch_editing_validates_through_clap_and_y_n_are_text"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver_launch.rs#launcher_launch_prefill_distrusts_a_hostile_model_and_a_forged_cap"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver_launch.rs#launcher_launch_the_default_options_produce_the_golden_argv"
        status: pass
    human_judgment: false
  - id: D3
    description: "A host missing sandbox pieces shows the two Red rows and the Yellow [U] row; y launches nothing and shows the refusal; U opens the separate Launch UNSANDBOXED confirmation (Red, default No) whose y alone launches with unsandboxed = true, once, and nothing is remembered for the next form; a too-old or missing claude is refused in both postures with no [U]."
    requirement: "DTAB-04"
    verification:
      - kind: e2e
        ref: "src/app.rs#launcher_launch_a_degraded_host_launches_unsandboxed_only_through_the_second_confirmation"
        status: pass
      - kind: e2e
        ref: "src/app.rs#launcher_launch_an_old_claude_is_refused_without_an_unsandboxed_offer"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver_launch.rs#launcher_launch_u_then_y_launches_unsandboxed_once_and_remembers_nothing"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver_launch.rs#launcher_launch_u_then_n_dispatches_nothing"
        status: pass
    human_judgment: false
  - id: D4
    description: "A body that says control: pause or stop blocks y and c closes the form and opens the control chooser without writing; driver check problems (template placeholders) block y and warnings do not; the spend line states the per-session --max-budget-usd bound and, in fresh-per-item mode, that a run can spend a multiple of it, and never claims a hard per-run ceiling."
    requirement: "DTAB-04"
    verification:
      - kind: e2e
        ref: "src/app.rs#launcher_launch_a_paused_body_blocks_y_and_c_opens_the_chooser"
        status: pass
      - kind: e2e
        ref: "src/app.rs#launcher_launch_template_placeholders_block_y"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver_launch.rs#launcher_launch_the_spend_line_is_per_session_and_gains_the_multiple"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver_launch.rs#launcher_launch_driver_check_problems_block_y_and_warnings_do_not"
        status: pass
    human_judgment: false
  - id: D5
    description: "s asks for a slug (live UTC run-id preview, the not-usable copy for a slug that fails SLUG_RE or would pass 80 bytes), Enter creates the run through create_run_dir, a repeated slug keeps the input open with 'already exists', success selects the run and opens the 30-04 editor on its body, the edited body is driver-checked (problems open Check DIRECTION.md where y re-opens the editor and n leaves it unlaunched; a clean check opens the launch form), and s never opens the legacy start flow."
    requirement: "DTAB-04"
    verification:
      - kind: e2e
        ref: "src/app.rs#launcher_new_run_s_creates_edits_checks_and_launches"
        status: pass
      - kind: e2e
        ref: "src/app.rs#launcher_new_run_the_same_slug_twice_says_the_run_exists_and_keeps_the_buffer"
        status: pass
      - kind: e2e
        ref: "src/app.rs#launcher_new_run_n_leaves_the_run_unlaunched"
        status: pass
      - kind: e2e
        ref: "tests/driver_tab_launch_e2e.rs#a_new_run_launches_with_the_argv_the_tab_builds"
        status: pass
      - kind: unit
        ref: "src/ui/screens/detail.rs#pressing_s_on_the_driver_tab_pushes_the_new_run_slug_input_not_the_start_flow"
        status: pass
    human_judgment: false
  - id: D6
    description: "Failed launches say nothing was started and are escaped and capped at 3 lines / 300 characters; the run id, alias, model, missing items, findings and the slug buffer render escaped on all four new screens (render-escape-guard states)."
    requirement: "DTAB-04"
    verification:
      - kind: unit
        ref: "src/ui/screens/driver_launch.rs#launcher_launch_error_copy_escapes_every_variant"
        status: pass
      - kind: unit
        ref: "src/ui/screens/render_escape_guard.rs#every_adjudicated_screen_has_a_probe_fixture"
        status: pass
    human_judgment: false
  - id: D7
    description: "The Driver tab's keys shadow no global key: q ? M D 1-8 act on the Driver tab as on GitHistory, Ctrl+C still quits, c a u R L [ ] keep their meaning on Sessions / Queue / Roadmap, and lower-case r never opens the launch form."
    requirement: "DTAB-04"
    verification:
      - kind: e2e
        ref: "src/app.rs#launcher_view_driver_keys_shadow_no_global_key"
        status: pass
      - kind: unit
        ref: "src/ui/screens/detail.rs#launcher_view_driver_only_keys_leave_the_roadmap_alone"
        status: pass
    human_judgment: false
  - id: D8
    description: "The in-process launch_detached call from the TUI is exercised end to end."
    requirement: "DTAB-04"
    verification: []
    human_judgment: true
    rationale: "launch_detached re-executes current_exe, so cargo test cannot call it in-process (it would start the test binary). The App path is proven with a fake LaunchSeams and the binary path with the same argv; a reviewer may still want one real TUI launch against the real claude, which D-26 puts out of scope (zero spend)."
  - id: D9
    description: "On a non-Linux build R and s are inert with the read-only copy and every launch arm refuses with 'Nothing was started'."
    requirement: "DTAB-04"
    verification: []
    human_judgment: true
    rationale: "Only the Linux target is installed here. The non-Linux arms compile by construction (cfg gates mirror the 30-04 pattern) and cfg(not(linux)) tests exist, but none can run; the Linux footer tests assert the same fields through can_steer()."
  - id: D10
    description: "The form and popups read well at real terminal sizes (the form is about 20 rows with a degraded host and findings; the popup cuts findings at 3 and 8)."
    verification: []
    human_judgment: true
    rationale: "Structure, copy, keys and the 80x24 buttons are asserted; whether the density and wording are pleasant is a taste call."

duration: 31min
completed: 2026-10-21
status: complete
---

# Phase 30 Plan 05: Start, launch and relaunch a driver run from the tab Summary

**`R` opens a launch form (prefilled from a crashed run's recorded caps and model, clap-validated, host-checked, with an honest per-session spend line) whose confirmation relaunches the same run id through the launcher's detached path, a degraded host gets a separate Red unsandboxed confirmation, and `s` mints a new run, edits its DIRECTION.md in the editor, driver-checks it and offers the launch.**

## Performance

- **Duration:** 31 min
- **Started:** 2026-10-21T02:32:42Z
- **Completed:** 2026-10-21T03:05:00Z
- **Tasks:** 3 (4 code commits: one follow-up refactor)
- **Files modified:** 12 (2 created), +4953 -18

## Accomplishments

- **Tracer (Task 1):** `R` on a crashed run opens `LaunchFormScreen` (`Launch driver run`, Yellow, default No) prefilled by `relaunch_prefill`; the host check arrives through the injectable `LaunchSeams::detect` and `y` is inert until it does; `y` dispatches plain-data `LaunchArgs`, the App builds the `LaunchOptions` exactly as `src/driver_cli.rs` does and calls `LaunchSeams::launch` (production: `launch_detached`) on `spawn_blocking` with the registry path. The binary e2e kills a launcher and its driver, reads `Crashed`, builds the argv from the prefilled form and runs it through the real binary with the fake shims: `crash-detected` precedes `launch | n=2`. The tracer was re-verified end to end before expanding.
- **Refusals (Task 2):** missing sandbox pieces render the two Red rows and the Yellow `[U]` row and `y` launches nothing; `U` replaces the form with `Launch UNSANDBOXED` (Red, default No, names the missing pieces) whose `y` alone launches with `unsandboxed = true`; a too-old or absent claude is refused in both postures with no `[U]`; `control: pause|stop` blocks `y` and `c` opens the chooser without writing; `driver check` problems block `y` while warnings do not; `launch_error_copy` words every `LauncherError` variant and says nothing was started.
- **New run (Task 3):** `SlugInputScreen` (UTC preview, `SLUG_RE` plus the 80-byte bound, `Exists` keeps the buffer), `create_run_dir` on `spawn_blocking`, the new run selected and its body opened through the 30-04 editor flow (`EditSession::then_launch`), `driver check` over the edited body (`LintPopupScreen` for problems, the launch form for a clean one), `s` no longer opens `DriverStartScreen`, and `launcher_view_driver_keys_shadow_no_global_key` pins the whole keymap.

## Task Commits

1. **Task 1 (tracer): R on a crashed run opens a prefilled launch form that relaunches the same run id** - `8acfbb9` (feat)
2. **Task 2: the launch form refuses what the launcher would refuse; the unsandboxed launch is a second confirmation** - `1336dfd` (feat)
3. **Task 3: s mints a new run, opens its DIRECTION.md, checks it and offers the launch** - `b2b19d8` (feat)
4. **Follow-up: gate the App test helper that only Linux tests call** - `15bcda2` (refactor)

**Plan metadata:** committed with this file (docs: complete plan)

## Files Created/Modified

- `src/ui/screens/driver_launch.rs` (new) - launch kinds and refusals, `LaunchForm`, `relaunch_prefill`, `launch_options_for`, `LaunchPreflight` / `launch_preflight`, `launch_error_copy`, slug helpers, and the four screens
- `tests/driver_tab_launch_e2e.rs` (new) - relaunch of a crashed run and launch of a new run through the real binary, fake shims
- `src/cli.rs` - `launch_defaults`, `parse_launch_field` over a private flattened `LaunchArgs` parser
- `src/app.rs` - `LaunchSeams`, `EditSession::then_launch`, `App::new_run_flow`, the preflight / launch / new-run handlers and their tests
- `src/action.rs` - `DriverLaunchPreflightRequested`, `DriverLaunchPreflighted`, `DriverLaunchRequested`, `DriverLaunched`, `DriverNewRunRequested`, `DriverNewRunCreated`, `DriverNewRunChecked`, `DriverNewRunLeft`, `NewRunFailure`
- `src/ui/screens/detail.rs` - `R` and `s` arms, `[R]launch` / `[R]elaunch` footer tokens
- `src/ui/screens/driver_run.rs` - `DriverKeys::launch`, `start` gated on Linux
- `src/ui/screens/mod.rs` - `pub mod driver_launch`, `ProjectViewCache::{launch_preflight, new_run_status}`
- `src/ui/screens/help.rs` - the `s` and `R` rows
- `src/ui/screens/render_escape_guard.rs` - rows and fixtures for the four screens
- `src/driver/steer.rs`, `src/ui/screens/driver_steer.rs` - `read_direction` and `tail_cells` made visible to the new code (one word each)

## Decisions Made

- The launch itself is never in-process in a test; see `key-decisions` for the Pitfall 5 coverage note, which is also recorded for VERIFICATION below.
- `[inferred — audit]` copy strings and decisions this plan introduced beyond the plan's own `[inferred — audit]` list:
  - **Launching status:** `Launching {run id} on {alias}` shown from `y` until the daemon answers (a silent wait of up to the daemon budget would look like a hang; the wording states a fact and promises nothing).
  - **Unreadable run:** `R` on an unreadable row says `{run id} could not be read, so it cannot be launched.`
  - **Lint headline:** `driver check found {n} problem(s). Close this form, then fix them with [e] before launching.` The plan's string lacked `Close this form, then`, but inside the form `e` edits a field.
  - **Used-up hint:** `  minutes: the old limit is used up. Raise it or the launcher refuses this launch.`
  - **Typed model:** accepted when non-empty, at most 100 characters, no control character and accepted by clap; refusal copy `must be a model name`. The conservative charset applies only to a model prefilled from a recorded connection.
  - **First problem:** a `DirectionInvalid` failure appends ` First problem: {finding}` to the plan's copy.
  - **Slug rows:** an empty slug previews `run id: {yyyymmdd}-`; a slug that fits `SLUG_RE` but would pass 80 bytes shows `not usable: the run id <yyyymmdd>-<slug> may be at most 80 bytes`; Enter on a slug that was just reported as existing does nothing until the slug changes.
  - **Fresh form:** a never-launched run shows no ` (default)` notes (nothing was recorded to default); a relaunch flags only the values the run did not record.
  - **`U` and `c` gating:** `U` is offered only when the host is missing sandbox pieces, claude is fine and nothing else would refuse the launch; `c` only while the body says something other than `run`. Both replace the form.
  - **Non-Linux:** `R` and `s` say `read-only: steering needs Linux`; `[s]tart` leaves the footer there.
  - **Help rows** are single rows with the UI-SPEC wording (the popup clips long rows at 80 columns exactly as the 30-04 rows already do).
  - **Flow flag lifetime:** `App::new_run_flow` is also dropped when the run is launched and when the kept edit text is discarded; an Esc-keep leaves it, so a later successful edit of that same run reaches the check and the form.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `DriverKeys` literals and bit loops no longer compiled**
- **Found during:** Task 1
- **Issue:** five `DriverKeys { .. }` literals and two `0..2048` bit loops in `detail.rs` tests lacked the new `launch` field.
- **Fix:** literals gained `launch: None`; the loops widened to `0..8192` and cover `Launch` / `Relaunch`.
- **Files modified:** `src/ui/screens/detail.rs`
- **Committed in:** `8acfbb9`

**2. [Rule 3 - Blocking] `read_direction` and `tail_cells` were private**
- **Found during:** Tasks 1 and 3
- **Issue:** the launch preflight and the new-run check must read DIRECTION.md under the same 1 MiB cap the edit flow uses, and the slug input echoes its buffer the way the answer input does.
- **Fix:** `driver::steer::read_direction` is `pub`; `driver_steer::tail_cells` is `pub(super)`. No logic changed.
- **Files modified:** `src/driver/steer.rs`, `src/ui/screens/driver_steer.rs`
- **Committed in:** `8acfbb9`, `b2b19d8`

**3. [Rule 2 - Missing Critical] `Action::DriverNewRunLeft` added**
- **Found during:** Task 3
- **Issue:** the plan's variant list gives the `Check DIRECTION.md` popup no way to end the new-run flow from `n`: a screen cannot touch the App's flow flag, and a stale flag would send a later unrelated edit to the launch form.
- **Fix:** one plain-data variant; the App drops the flag and sets the left-unlaunched copy.
- **Files modified:** `src/action.rs`, `src/app.rs`, `src/ui/screens/driver_launch.rs`
- **Committed in:** `b2b19d8`

**4. [Rule 3 - Blocking] 30-04 help test asserted the retired `s` row**
- **Found during:** Task 3 (full run)
- **Issue:** `launcher_steer_the_help_documents_c_a_and_u_only_with_the_flag_on` looked up `Start a run`.
- **Fix:** it looks up the new `s` and `R` rows in UI-SPEC order and asserts the old row is gone and both new rows are absent with the flag off.
- **Files modified:** `src/ui/screens/help.rs`
- **Committed in:** `b2b19d8`

**5. [Rule 2 - Missing Critical] Test helper gated to Linux**
- **Found during:** final cfg review
- **Issue:** `top_screen_name` was only called by Linux tests, so a non-Linux test build would warn under `-D warnings`.
- **Fix:** `#[cfg(target_os = "linux")]`.
- **Files modified:** `src/app.rs`
- **Committed in:** `15bcda2`

### Plan-vs-implementation notes (not defects)

- **`DriverNewRunChecked.warnings` is carried and not read:** the launch form lists the warnings from its own preflight, which re-lints the file the user just saved.
- **The new-run e2e test landed in Task 1's commit** with the shared harness (Task 2's verify still requires both tests to pass).
- **Acceptance grep `grep -c '"SlugInputScreen"' render_escape_guard.rs` is 1 (and `"LintPopupScreen"`):** each is 2, a row in `SCREEN_IDENTITY_DISPOSITIONS` and an arm in `fixture_for`, as 30-04 recorded for its screens.
- **`grep -c 'launch_detached' src/app.rs` is 1:** the production seam value; the plan's "at least 1" holds.
- **`git diff --stat 896e21e..HEAD -- src/launcher` is empty** (and `git diff --stat f8ab657 -- src/launcher` is empty): this plan changed nothing under `src/launcher`.

---

**Total deviations:** 5 auto-fixed (3 blocking, 2 missing critical). **Impact on plan:** no scope change; test updates the new field forced, two one-word visibility changes, one extra plain-data action.

## Issues Encountered

- No way to compile or run the non-Linux arms here (only the Linux target is installed); see D9.
- No flaky test failed in either full run (the three known load-sensitive tests passed).

## Verification

- `mkdir -p -m700 /tmp/claude-1000/t; umask 077; TMPDIR=/tmp/claude-1000/t rtk proxy cargo test --no-fail-fast`: **4023 passed, 1 failed, 24 ignored** on the code of `15bcda2` (baseline after 30-04: 3958 passed, 1 failed, 24 ignored; +65 tests). The one failure is the expected git-version witness `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`; an earlier full run on `b2b19d8` read the same totals.
- `rtk proxy cargo clippy --all-targets -- -D warnings`: **clean**. `cargo fmt --check`: clean.
- Per-task filters: Task 1 `--test driver_tab_launch_e2e` 2 passed and `--lib -- launcher_launch_ screens::driver_launch:: cli::tests render_escape_guard` 68 passed; Task 2 the same filter 60 passed; Task 3 `launcher_new_run_ launcher_launch_ screens::driver_launch:: screens::detail:: screens::help:: render_escape_guard driver_keys_shadow_` 504 passed (including `launcher_view_driver_keys_shadow_no_global_key`).
- Plan verification: `--test driver_tab_launch_e2e --test launcher_e2e --test launcher_reconcile` 2 + 29 + 7 passed; `--test async_blocking_guard --test spawn_seam_guard` 9 + 41 passed (Task 1).
- Acceptance greps: `fn launch_defaults` in `cli.rs` 1, `fn launch_options_for` 1, `RunStore|driverctl::serve|UnixListener` in `src/ui` and `src/app.rs` 0, `Launch UNSANDBOXED` 3, `Each driver session may spend up to` 3, `This run may spend up to` 0, `fn launch_error_copy` 1, `DriverStartScreen::new` in `detail.rs` 0 (baseline 1), `create_run_dir` in `app.rs` 4, `then_launch` in `app.rs` 9, `fn launcher_view_driver_keys_shadow_no_global_key` 1.
- **Pitfall 5 / Assumption A5 coverage note (for VERIFICATION):** the TUI's own `launch_detached` call is not exercised in-process. It is covered by the `LaunchSeams` App tests (a fake `launch` records the `LaunchOptions` built from the form's arguments, with the registry path) plus the binary e2e that runs the argv `launch_argv` derives from the very same options through the real `gsd-meta-manager driver launch`, minus the two daemon-only flags. No live model is called anywhere (D-26).

## Known Stubs

None.

## Threat Flags

| Flag | File | Description |
|------|------|-------------|
| threat_flag: process-spawn (existing path, new caller) | src/app.rs | `request_launch` is a new caller of `launch_detached`, which re-executes the binary and starts a driver that spends money. Within T-30-15/16/19: unsandboxed only through the second confirmation, every value clap-validated, the TUI embeds no run store and binds no socket (grep gate 0). |

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Phase 30 plans are complete: the Driver tab lists, shows, steers, starts, launches and relaunches launcher runs from DIRECTION.md. For the phase verifier: D8, D9 and D10 are the three human-judgement deliverables (this plan), alongside 30-04's D8 and D9; the flagged DTAB-04 probe reviewer questions (relaunch of a `done` run is refused here, `[inferred — audit]`; the in-process launch is covered by seam plus binary) are answered by D1 and D8.
- Phase 31 can drop `DriverStartScreen`, `DriverInjectScreen` and the dashboard `r` start: nothing on the Driver tab reaches them any more.

## Self-Check: PASSED

- Created/modified files exist (12 of 12 checked: `src/ui/screens/driver_launch.rs`, `tests/driver_tab_launch_e2e.rs` and the ten modified); commit hashes `8acfbb9 1336dfd b2b19d8 15bcda2` are ancestors of HEAD; the post-commit deletion check was empty for all four commits; `check evaluation-scope --plan 30-05 --commits-only` is covered by the same hashes; acceptance greps for all three tasks hold (the notes above).

---
*Phase: 30-driver-tab-on-direction-md*
*Completed: 2026-10-21*
