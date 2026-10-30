---
phase: 31-retire-superseded-envelope-goal-layer-code
plan: 02
subsystem: retirement
tags: [rust, ratatui, tui, retirement, injection, experimental-gate, confirm-screen]

requires:
  - phase: 31-retire-superseded-envelope-goal-layer-code
    provides: 31-01 rewired the survivors (push redaction, PR ledger, router refusal), so the UI half can go first
provides:
  - a TUI with no stream-json injection surface (driver_inject.rs, the two inject Actions, schedule_inbox_append, the injection model in ui/screens/driver.rs)
  - a dashboard with no legacy start/stop/opt-in keys (driver_start.rs, App::start_driver_run/stop_driver_run, three Actions, StopDisposition)
  - DriverConfirmScreen trimmed to DriverAction { ToggleOptIn, StopLauncher } with opt-in prompts that state only what launcher::push::policy_for reads
  - re-measured census pins (MEASURED_REACH, driver CENSUS totals, glyph-scan file floor)
affects: [31-03, 31-04, 31-05, 31-06]

actuals:
  tokens: 5000
  tasks: 2
  commits: 2

plan_head_before: c20b871887f862df9b430219b35fbf6bf5bf8a13
plan_head_after: eafd784cf920fb6184f9500f81d93502bc9d7c59
commits: 2

tech-stack:
  added: []
  patterns:
    - "delete the surface, let cargo check list what broke, delete what only served it (pub items need a manual grep: cargo does not flag them)"
    - "regression test for a retired key selects a project and asserts ScreenAction::None with the flag on and off"

key-files:
  created: []
  modified:
    - src/action.rs
    - src/app.rs
    - src/error.rs
    - src/text.rs
    - src/ui/glyphs.rs
    - src/ui/mod.rs
    - src/ui/screens/detail.rs
    - src/ui/screens/driver.rs
    - src/ui/screens/driver_confirm.rs
    - src/ui/screens/help.rs
    - src/ui/screens/mod.rs
    - src/ui/screens/normal.rs
    - src/ui/screens/render_escape_guard.rs
    - README.md
    - docs/CONFIGURATION.md
  deleted:
    - src/ui/screens/driver_inject.rs
    - src/ui/screens/driver_start.rs

key-decisions:
  - "The dashboard o is deleted with r and x (D-03 names it); the Driver-tab o keeps the opt-in toggle reachable [inferred - audit]"
  - "C12 copy: withdraw says the PR caps and branch namespace stop applying and the push step falls back to the defaults; allow says the push step reads them from the record, else uses the defaults [inferred - audit]"
  - "The opt-in disclosure (render_disclosure, SECTION_*) stays: it is read by the grant prompt, not the Start prompt. Only its two gate sentences (re-confirmed at the spawn gate / before any run starts) were reworded [inferred - audit]"
  - "DriverRunJournal.injections was deleted with INJECTION_KINDS (nothing left to read it); driver_inbox and DriverRunsListed.inbox stay for 31-03/31-04"
  - "session_spawned_runs stays for 31-03: ui/screens/driver.rs render_output_section still reads it; its last writer (start_driver_run) is gone"

patterns-established:
  - "A retired-key regression test must select a project and run with the flag both on and off, or an absent arm and a bound-but-gated arm look the same"

requirements-completed: []  # RETIRE-01 and RETIRE-02 are shared with 31-01 and 31-03..31-07; the shared-ID gate (#2388) keeps them open until every declaring plan has a SUMMARY, and PATTERNS says flip RETIRE-01..03 only at verification

coverage:
  - id: D1
    description: "No key, action, handler or screen in the TUI can queue a stream-json injection; the Phase 30 retired-i pointer survives and DIRECTION.md steering tests pass unchanged"
    requirement: RETIRE-01
    verification:
      - kind: unit
        ref: "src/ui/screens/help.rs#launcher_steer_the_help_has_no_injection_row_and_no_injected_message_legend"
        status: pass
      - kind: unit
        ref: "driver::steer:: + screens::driver_steer:: + launcher_steer_ (lib, 277 pass in the tracer verify)"
        status: pass
    human_judgment: false
  - id: D2
    description: "The dashboard r, x and o keys are gone with their experimental guards; they push nothing with the flag on or off, and App has no legacy start/stop path"
    requirement: RETIRE-02
    verification:
      - kind: unit
        ref: "src/ui/screens/normal.rs#the_retired_dashboard_driver_keys_push_nothing_with_the_flag_on_or_off"
        status: pass
    human_judgment: false
  - id: D3
    description: "Both opt-in prompts state exactly what the record feeds and claim no gate; a hostile alias is escaped in both directions; the launcher kill prompt is unchanged"
    requirement: RETIRE-02
    verification:
      - kind: unit
        ref: "src/ui/screens/driver_confirm.rs#the_opt_in_prompts_say_what_the_record_feeds_and_claim_no_gate"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver_confirm.rs#the_opt_in_prompts_escape_a_hostile_alias"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver_confirm.rs#launcher_stop_prompt_names_the_run_the_alias_and_the_whole_scope"
        status: pass
    human_judgment: false
  - id: D4
    description: "Census pins travel with the deletion and every surviving screen keeps its identity fixture"
    requirement: RETIRE-02
    verification:
      - kind: unit
        ref: "src/ui/mod.rs#no_display_identity_call_under_ui_stands_outside_a_composition"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver.rs#no_executable_control_class_call_in_these_two_files_stands_outside_a_composition"
        status: pass
      - kind: unit
        ref: "src/ui/screens/render_escape_guard.rs#the_screen_renders_identity_escaped"
        status: pass
    human_judgment: false

duration: 20min
completed: 2026-10-25
status: complete
---

# Phase 31 Plan 02: Retire the Injection UI and the Legacy Dashboard Start/Stop Summary

**The stream-json injection screen and the dashboard's legacy `r` / `x` / `o` start, stop and opt-in keys are gone with their handlers and census entries; `DriverConfirmScreen` keeps only the launcher's two actions and its opt-in prompts now say what `launcher::push::policy_for` reads, not that a spawn gate exists.**

## Performance

- **Duration:** about 20 min (commit clock: 23:50 to 00:08)
- **Tasks:** 2 (1 tracer, 1 auto)
- **Files:** 15 modified, 2 deleted. Net -4,377 lines over the two commits.

## Phase baseline and post-plan suite

Before (from the 31-01 SUMMARY, HEAD `c20b871`):

```
results=94 passed=4418 failed=1 ignored=30
failing: envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against
```

After (FULLSUITE, HEAD `eafd784`):

```
results=94 passed=4359 failed=1 ignored=30
failing: envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against
expected results=94
```

The lone failure is the known git-version witness (local git 2.53.0 against the constant's 2.55.0; 31-05 deletes it). `editor_race_smoke` passed. -59 passed matches the net change in `#[test]`/`#[tokio::test]` attributes under `src/` (3142 to 3083): the injection model, `driver_inject` and `driver_start` screens, the legacy start/stop handler tests and `goal_or_none` tests went; three tests were added (the retired-dashboard-keys regression and the two opt-in prompt tests). `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` both exit 0.

## Census pins: old and new

| Pin | Old | New | Why |
|---|---|---|---|
| `ui::MEASURED_REACH` `driver_confirm.rs` | 4 | 2 | start prompt's command call and `goal_row`'s call left with `DriverAction::Start` and `goal_row`; array length stays 3 |
| `ui::MEASURED_REACH` `driver.rs` / `render_escape_guard.rs` | 1 / 3 | 1 / 3 | unchanged |
| `ui::tests` doc table | 8 total, 5 non-exempt, 17 files | 6 total, 3 non-exempt, 22 files | file count re-measured (the old 17 was already stale: 24 before this plan) |
| `UI_SOURCE_FLOOR` | 16 | 16 | stays; 22 files remain |
| `driver::CENSUS_FILES` totals (units, occurrences, whole-file) | (4, 5, 6) | (2, 3, 4) | the same two driver_confirm.rs sites; two sites removed, none moved |
| `ui::glyphs` scan file floor (not named in the plan) | 25 | 24 | 26 files before, 24 after the two screen files went; a floor against a vacuous walk |
| `render_escape_guard` `SCREEN_IDENTITY_DISPOSITIONS` | 23 rows | 21 rows | `DriverInjectScreen` and `DriverStartScreen` rows, fixtures and doc mentions removed |
| `render_escape_guard` `DriverConfirmScreen` fixture | 5 prompts | 3 prompts | start and stop prompts removed; launcher kill, grant and withdraw remain |

## Surviving `ctx.experimental` sites (recorded per the plan's verification)

`src/app.rs`: `stop_launcher_run` (1379), `launch_target` (1431), `request_new_run` (1559), `steering_target` (1802), the watcher-event read (2246), and the schedule-on-event guard (3222). `src/ui/screens/normal.rs`: the driven-badge read (355). No surviving guard was removed or loosened. The removed sites were the two inside `start_driver_run` / `stop_driver_run` and the three dashboard match guards.

## `session_spawned_runs`

**Left for 31-03.** `ui/screens/driver.rs` `render_output_section` still reads it (`adopted`), inside the legacy run-detail renderer 31-03 deletes. Its only writer, `start_driver_run`, is gone, so the set is always empty until then.

## Accomplishments

- **D-08 UI half.** `driver_inject.rs`, `Action::DriverInject{Requested,Written}`, `schedule_inbox_append` and their `update` arms are gone, plus the injection model in `ui/screens/driver.rs` (`InjectionState`, `MissedReason`, `derive_injection_states`, `injection_rows`, `injection_block`, `INJECTION_KINDS`, glyph/label/gloss constants). `output_body_lines` now renders injection-kind journal records as ordinary lines (nothing is spliced or swallowed). The Phase 30 `i` pointer (`INJECTION_RETIRED`) is intact.
- **D-03 UI half.** `driver_start.rs`, the dashboard `r`/`x`/`o` arms, `App::{start_driver_run, stop_driver_run}`, `Action::{DriverStartRequested, DriverStopRequested, DriverStopped}`, `StopDisposition` and `goal_or_none` are gone. A new regression test selects a project and presses the three keys with the flag on and off: nothing is pushed.
- **C1 / C12.** `DriverAction` is `{ ToggleOptIn, StopLauncher }`. Withdraw: *Withdraw the driver opt-in for "{alias}"? Its PR caps and branch namespace stop applying; the push step falls back to the defaults.* Allow: *Record a driver opt-in for "{alias}"? The push step reads its PR caps and branch namespace from it; without one it uses the defaults.* The alias still goes through `render_for_terminal`.
- **Honest text.** Help rows for the dashboard `r`/`x`/`o` and the README sentence are gone; the README now states that `Shift+D` opens the experimental Driver tab.

## Task Commits

1. **Task 1 (tracer): retire the injection UI** - `a805eb5` (refactor)
2. **Task 2: retire the dashboard start/stop/opt-in keys, trim the confirm screen** - `eafd784` (refactor)

Tracer gate: `HUMAN_VERIFY_MODE=end-of-phase` and the tracer `<verify>` is automated-only, so it was re-run before expansion (277 lib tests in the steer/driver_run/escape-guard/help filter pass, `launcher_steer_the_help_has_no_injection_row_and_no_injected_message_legend` present; clippy clean). Tracer verified end-to-end, expanding.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing critical] User-facing text still named deleted keys and gates**
- **Found during:** Task 2 (the plan's prohibition: no text may claim a gate, key or capability this plan deleted)
- **Issue:** beyond the files the plan listed, four places still said something false: `OptInError::NotOptedIn` in `src/error.rs` ("Press `o` on the dashboard to opt it in"); the opt-in disclosure's `SECTION_PROMPT_INPUTS` ("re-confirmed at the spawn gate before a run starts") and its empty-list line ("will be re-confirmed before any run starts"); the `do_toggle_opt_in` status messages ("no NEW run can start"); and `docs/CONFIGURATION.md` ("no `r`/`x`/`o` driver keys").
- **Fix:** error text now says the project's Driver tab; the disclosure keeps its shape but drops the two gate claims (the empty-list test no longer asserts "re-confirmed"); status messages read `"{alias}": driver opt-in withdrawn` / `recorded`; the docs line says "no driver keys". Comments that named the spawn gate or the dashboard `o` in `detail.rs` and `driver_confirm.rs` were reworded.
- **Files modified:** src/error.rs, src/ui/screens/driver_confirm.rs, src/ui/screens/detail.rs, docs/CONFIGURATION.md
- **Commit:** `eafd784`

**2. [Rule 3 - Blocking/census] A census pin the plan did not name**
- **Found during:** Task 2 FULLSUITE-adjacent run (`ui::glyphs::tests::production_ui_code_draws_only_registry_or_exempt_glyphs`: "only 24 files read")
- **Issue:** the glyph scan asserts `files.len() >= 25`; deleting two screen files leaves 24.
- **Fix:** floor lowered to 24 with a comment. It is a non-vacuity floor, not a coverage target; the two deleted files drew no glyph the scan could have missed.
- **Files modified:** src/ui/glyphs.rs
- **Commit:** `eafd784`

**3. [Rule 3 - Dead code cargo cannot see] Orphans of the deleted handlers**
- **Found during:** both tasks (`pub` items do not trigger `dead_code`; found by grep)
- **Removed:** `goal_or_none` and its test (its only production caller was the start handler), `App::planning_dir_for` (only caller `schedule_inbox_append`), `DriverRunJournal.injections` (only reader `injection_rows`), `StopDisposition`, `DEFAULT_DRIVE_COMMAND`, the `now` parameter of `output_body_lines` / `render_output_section`. `text.rs` doc-link to `goal_or_none` was dropped.
- **Commits:** `a805eb5`, `eafd784`

**4. Process note.** The first `git add` of Task 1 failed on the already-staged deletion, so the first commit held only the deletion; it was amended (unpublished, before any other commit) to include the rest. Result: `a805eb5` is the single Task 1 commit.

**Total deviations:** 3 auto-fixed (1 Rule 2, 2 Rule 3), 1 process note. **Impact:** no scope growth; every item is a leftover of the deleted surfaces.

## Decisions Made (inferred, for audit)

- Dashboard `o` deleted, not kept (D-03 names it; research Open Question 1 suggested keeping it). The Driver tab's `o` keeps the toggle reachable. `[inferred - audit]`
- C12 prompt wording as above. `[inferred - audit]`
- The opt-in disclosure stays because the grant prompt reads it (the plan's condition for deleting it, "only the Start prompt read them", does not hold). Its seam/executor paragraphs are untouched and will need a pass when 31-04 deletes the seams and 31-07 decides the executor. `[inferred - audit]`
- The opt-in toggle's background stays the dashboard (`NormalScreen`), as before; only its comment changed. Switching it to the Driver tab (where it is now opened from) is a UI change outside this plan.

## Issues Encountered

None beyond the deviations above.

## Known Stubs

None.

## Threat Flags

None. The plan added no network endpoint, auth path or file-access surface; T-31-04 (prompt claims), T-31-05 (gate sites) and T-31-06 (escape coverage) are mitigated as described above.

## Left for later plans (recorded, not done)

- **31-03:** `schedule_dry_run_report` has no caller now (its only caller was `DriverStartScreen`); `session_spawned_runs`; `driver_inbox` / `DriverRunsListed.inbox` and the inbox read in `schedule_run_list_scan`; `DriverLineKind::Injection` and `MARKER_INJECTION` (journal records of injection kinds still project to them).
- **31-04:** `docs/CONFIGURATION.md` still says `driver_max_concurrent` "still caps concurrent runs" and the `drive` subcommand lines (its rewrite is in that plan); the opt-in disclosure's model-seam paragraphs; `registry::current_prompt_inputs`.
- **31-07 (held):** executor stdin write path and Codex transport untouched, as instructed.

## Next Phase Readiness

Ready for 31-03 (legacy runs section) and 31-04 (core modules): nothing under `src/ui/` calls `driver::spawn`, `journal::inbox::append`, `driver::kill::stop_run` or the dry-run builder any more, so those can be deleted without orphaning a caller. The git-version witness stays red locally until 31-05.

## Self-Check: PASSED

- Files: both deleted files absent (`test ! -e` on `driver_inject.rs` and `driver_start.rs`); all 15 modified files present
- Commits `a805eb5` and `eafd784` are ancestors of HEAD; `git rev-list --count` from the recorded `plan_head_before` is 2
- Acceptance greps: `DriverInjectRequested|DriverInjectWritten` in action.rs 0; `schedule_inbox_append` in src 0; `injection_rows|injection_block|derive_injection_states` in driver.rs 0; `"DriverInjectScreen"` in the escape guard 0; `INJECTION_RETIRED` in detail.rs 3; `start_driver_run|stop_driver_run` in app.rs 0; `DriverStartRequested|DriverStopRequested|DriverStopped` in action.rs 0; `No NEW run can start` 0; `spawn gate` in driver_confirm.rs 0; `DriverAction::(ToggleOptIn|StopLauncher)` 37; the README `r / x / o` sentence 0; the three dashboard help-row strings 0; `Toggle driver opt-in for this project` 3
- Plan verification: targeted lib filters (892 tests) pass; clippy and fmt clean; FULLSUITE `results=94` equals expected, only the known witness fails

---
*Phase: 31-retire-superseded-envelope-goal-layer-code*
*Completed: 2026-10-25*
