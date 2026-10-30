---
phase: 30-driver-tab-on-direction-md
plan: 06
subsystem: ui
tags: [rust, ratatui, tui, confirm-popup, gap-closure]
status: complete
gap_closure: true

requires:
  - phase: 30-driver-tab-on-direction-md
    provides: 30-05 Driver tab, steering screens and the shared confirm popup
provides:
  - "confirm popup sized by the rows word wrap really draws (wrapped_rows == Paragraph::line_count)"
  - "PopupLayout { buttons, clipped } and a clip guard: a cut-off confirmation cannot be confirmed"
  - "body_room / fit_list helpers and the edit-lint confirmation that always keeps its question"
  - "answer input footer that wraps its Red refusal and Yellow OQ-R1 rows"
  - "D-16 answer status copy short enough for an 80-column status bar"
  - "kill confirmation painted over the Driver tab"
affects: [30-07, 30-08, 30-09]

actuals:
  tokens: 6877
  tasks: 3
  commits: 6
plan_head_before: 80354092f9b8d2b80ad7d4fb2e1fbd3e4e5c6d4e
plan_head_after: 9ddadd5dd018084087f022d62ef0de425eae7770
commits: 6

tech-stack:
  added: []
  patterns:
    - "popup height comes from the same Paragraph::line_count the body is drawn with"
    - "render records a PopupLayout; key and mouse handlers read it (render-time guard, same Cell pattern as the button rects)"

key-files:
  created: []
  modified:
    - src/ui/screens/confirm_popup.rs
    - src/ui/screens/driver_steer.rs
    - src/ui/screens/driver_confirm.rs
    - src/ui/screens/create_project.rs
    - src/app.rs

key-decisions:
  - "[inferred — audit] A clipped ConfirmPopup disarms y, Enter-on-Yes and a Yes click; n, Esc and a No click still cancel; the hint row reads CLIPPED_HINT in Yellow."
  - "[inferred — audit] The D-16 answer status copy is shortened to `{id} answered; {rule} parks it: [e] budget, [R] caps.` (59 cells for launcher-cap); the long OQ-R1 sentence stays in the answer input's second row."
  - "[inferred — audit] Only DriverAction::StopLauncher paints the Driver tab behind its popup; Start/Stop/ToggleOptIn keep the dashboard background."
  - "fit_list keeps at most LINT_FINDINGS_SHOWN + 1 rows for findings, so nine findings on a tall terminal list all nine rather than 8 plus `… 1 more`."

requirements-completed: [DTAB-03]

coverage:
  - id: D1
    description: "Polite-stop prompt keeps 'Nothing is killed.' for an 18-char run id at 50x24, 60x16, 80x12, 80x24, 100x30, 120x40, 160x40"
    requirement: DTAB-03
    verification:
      - kind: unit
        ref: "src/ui/screens/driver_steer.rs#launcher_steer_the_polite_stop_sentence_survives_every_width"
        status: pass
      - kind: unit
        ref: "src/ui/screens/confirm_popup.rs#wrapped_rows_counts_the_rows_word_wrap_draws"
        status: pass
    human_judgment: false
  - id: D2
    description: "A clipped confirmation cannot be confirmed (primitive guard) and says so in its hint row"
    requirement: DTAB-03
    verification:
      - kind: unit
        ref: "src/ui/screens/confirm_popup.rs#a_clipped_body_disarms_yes_and_says_so"
        status: pass
      - kind: unit
        ref: "src/ui/screens/confirm_popup.rs#a_body_that_fits_keeps_yes_armed"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver_steer.rs#launcher_steer_a_clipped_polite_stop_writes_nothing_on_y"
        status: pass
    human_judgment: false
  - id: D3
    description: "Edit-lint confirmation lists what fits, counts the rest, and keeps 'Apply it anyway?'"
    requirement: DTAB-03
    verification:
      - kind: unit
        ref: "src/ui/screens/confirm_popup.rs#fit_list_keeps_the_rows_that_fit_and_counts_the_rest"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver_steer.rs#launcher_edit_the_lint_confirmation_keeps_its_question_on_a_short_terminal"
        status: pass
    human_judgment: false
  - id: D4
    description: "Answer input's Red and Yellow rows wrap and end visibly with [R]."
    requirement: DTAB-03
    verification:
      - kind: unit
        ref: "src/ui/screens/driver_steer.rs#launcher_steer_the_answer_warning_wraps_to_its_last_word"
        status: pass
    human_judgment: false
  - id: D5
    description: "D-16 answer status copy is at most 62 cells for every park rule"
    requirement: DTAB-03
    verification:
      - kind: unit
        ref: "src/ui/screens/driver_steer.rs#launcher_steer_the_park_answer_copy_fits_the_status_bar"
        status: pass
    human_judgment: false
  - id: D6
    description: "Kill confirmation paints the Driver tab, not the dashboard"
    requirement: DTAB-03
    verification:
      - kind: unit
        ref: "src/ui/screens/driver_confirm.rs#launcher_kill_confirm_keeps_the_driver_tab_behind_it"
        status: pass
    human_judgment: false

duration: ~20 min
completed: 2026-10-20
---

# Phase 30 Plan 06: Confirm-popup clipping and answer-footer gaps Summary

**Confirm popups are sized by `Paragraph::line_count` (the wrap they draw with) and refuse `y` while their body is cut off, which restores the polite-stop prompt's "Nothing is killed." at every width.**

## Accomplishments

- **VERIFICATION gap 1 (BLOCKER) closed.** `wrapped_rows` divided a character count by the width, so word wrap drew more rows than the popup had; at 80x12 the polite-stop prompt for `20261019-0042-abcd` lost "Nothing is killed.". It now asks ratatui (`line_count`, `Wrap { trim: false }`), the same wrap the body is drawn with. A sweep test covers seven terminal sizes.
- **Clip guard (UI D5).** `render_confirm_popup` returns `PopupLayout { buttons, clipped }`. A clipped popup shows `CLIPPED_HINT` in place of the key hint, and `ConfirmPopup` ignores `y`, Enter on Yes and a click on Yes until a render says the body fits. `n`, Esc and a click on No still cancel. A never-rendered popup is not clipped, so key-only tests are unchanged. All ten shipped callers pick the guard up through `record(..)`.
- **Edit-lint confirmation.** `body_room` and `fit_list` fit the findings into the rows left after the header and the question; `… {k} more` counts what was left out, and `Apply it anyway?` stays on screen whenever the header, one row and the question fit.
- **Answer footer (UI D3).** Row 1 is unchanged; the Red refusal and Yellow OQ-R1 rows are one wrapping `Paragraph`, footer height `1 + line_count`, capped at `max(2, height / 2)`.
- **D-16 answer copy (UI D4).** `{id} answered; {rule} parks it: [e] budget, [R] caps.`, 59 cells for `launcher-cap`.
- **Kill confirm (UI D6).** `StopLauncher` paints `DetailScreen::render_main_only` behind the popup.

## Task Commits

| Task | RED | GREEN |
|------|-----|-------|
| 1 (tracer) polite-stop sizing | `8de1ebc` test | `4177b47` fix |
| 2 clip guard, capped lint list | `beb1b43` test | `fed0569` feat |
| 3 wrapped footer, D-16 copy, kill background | `c5ab7a4` test | `9ddadd5` fix |

Tracer verified end-to-end before expansion: the Task 1 `<verify>` (lib tests for `screens::confirm_popup::` and `screens::driver_steer::`, plus clippy) passed.

## TDD Gate Compliance

Each task has a `test(30-06)` commit followed by a `fix`/`feat(30-06)` commit.
RED evidence (Rust `cargo test` text output; the `tdd-red-evidence` classifier has no adapter for it, so the semantic assessment is recorded here):

- Task 1: `wrapped_rows_counts_the_rows_word_wrap_draws` failed `left: 2, right: 3` (the planned 3-row case); `launcher_steer_the_polite_stop_sentence_survives_every_width` failed at 80x12 with the prompt cut after "Nothing is". Both are the planned assertions failing for the planned reason.
- Task 2: a compile-only skeleton (`PopupLayout`, `CLIPPED_HINT`, `body_room`, a pass-through `fit_list`) was committed with the tests so they failed on assertions, not on the build. Four tests failed (`a_clipped_body_disarms_yes_and_says_so`, `fit_list_keeps_the_rows_that_fit_and_counts_the_rest`, `launcher_steer_a_clipped_polite_stop_writes_nothing_on_y`, `launcher_edit_the_lint_confirmation_keeps_its_question_on_a_short_terminal`); `a_body_that_fits_keeps_yes_armed` passed by design as the regression guard.
- Task 3: four tests failed on the planned assertions (old long copy, unwrapped footer, dashboard background).

## Deviations from Plan

**1. [Rule 3 - Blocking] `src/app.rs` pinned the old D-16 copy**
- **Found during:** Task 3
- **Issue:** `app::tests` asserted `status.starts_with("Answer written to I-01, but it is parked by gap-rounds")`; the shortened copy the plan requires would fail it. `src/app.rs` is not in the plan's `files_modified`.
- **Fix:** one-line assertion update to `I-01 answered; gap-rounds parks it: [e] budget, [R] caps.`
- **Files modified:** `src/app.rs` | **Commit:** `c5ab7a4`

**2. [Rule 1 - Bug in plan test geometry] Clipped-hint text cannot fit a 40-wide popup**
- **Found during:** Task 2
- **Issue:** the plan asks for the hint text to be asserted in a 40x6 area, but its inner width is 34 cells and `CLIPPED_HINT` is 53.
- **Fix:** the disarm behaviour is asserted at 40x6 as planned; the hint text is asserted at 60x6. At narrower widths the hint is truncated by the terminal, while the guard is unaffected.
- **Files modified:** `src/ui/screens/confirm_popup.rs` | **Commit:** `beb1b43`

**3. [Scope note] `fit_list` test width.** The planned example gave no wrap-cost case; I added one at width 16 (a two-row item) to pin that a wrapped item costs its wrapped rows.

**Total deviations:** 2 auto-fixed (1 blocking, 1 plan-test geometry). **Impact:** none on behaviour.

## Inferred Decisions `[inferred — audit]`

1. Clip guard and hint copy (`Cut off: enlarge the terminal to confirm · Esc cancel`), per the plan.
2. Shortened D-16 answer copy. This deviates from the UI-SPEC row "Answer written, non-needs-human park"; plan 30-09 updates the UI-SPEC rows for all three shortened copies.
3. `StopLauncher`-only background change.
4. `render_confirm_popup` reports `clipped: true` when the popup has no room for its buttons (including `inner.height == 0`).
5. `fit_list` lets a tall terminal list up to nine findings with no `… k more` row (the row budget is `LINT_FINDINGS_SHOWN + 1`), instead of strictly eight plus a count.

## Verification

- `rtk proxy cargo test --no-fail-fast`: 4033 passed, 1 failed, 24 ignored. The one failure is the expected environment-only `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`.
- A first full run showed `app::tests::launcher_new_run_the_footer_names_launch_or_relaunch_for_the_selected_row` failing with "the launcher listing never reported" (a load-dependent async timeout). It passed alone three times and in the second full run; unrelated to this plan, not changed.
- `rtk proxy cargo clippy --all-targets -- -D warnings`: clean.
- `git diff --stat -- Cargo.toml Cargo.lock`: empty.

## Threat Surface

T-30-21 (popup body to decision) and T-30-22 (footer and status copy) are mitigated as planned; ids and rules still pass through `render_for_terminal` before becoming spans. No new network, auth, file or schema surface. No threat flags.

## Known Stubs

None. No entries appended to `.planning/WINDOWS.md`.

## Next

Ready for 30-07.

## Self-Check: PASSED

- Files: `src/ui/screens/confirm_popup.rs`, `driver_steer.rs`, `driver_confirm.rs`, `create_project.rs`, `src/app.rs` exist and carry the changes above.
- Commits `8de1ebc`, `4177b47`, `beb1b43`, `fed0569`, `c5ab7a4`, `9ddadd5` are ancestors of HEAD.
- All task `<acceptance_criteria>` greps and both `<verify>` commands re-run green.
