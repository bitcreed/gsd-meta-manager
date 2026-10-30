---
phase: 30-driver-tab-on-direction-md
plan: 09
subsystem: ui
tags: [rust, ratatui, tui, launcher, launch-form, gap-closure]
status: complete
gap_closure: true

requires:
  - phase: 30-driver-tab-on-direction-md
    provides: 30-06 confirm popup API (body_room, fit_list, wrapped_rows, clip guard), 30-07 INJECTION_RETIRED copy, 30-08 app.rs editor changes
provides:
  - "launch form body in priority order: spend and sandbox rows first, findings fitted to the rows left, key row only when free"
  - "Check DIRECTION.md popup that fits its problems and always keeps its question"
  - "slug input whose second row wraps; shortened left-unlaunched copy"
  - "request_new_run refusals close the slug input through finish_new_run"
  - "launch preflight and new-run check numbering lint lines from the first body line"
  - "launch form reading control with the launcher's control_line and judging a typed model with model_name_ok"
affects: []

actuals:
  tokens: 14300
  tasks: 3
  commits: 6
plan_head_before: bd078a7e35e14048914987be5c085c37cc63a3e5
plan_head_after: 9ad47bd9adedbb2d33af081c53f987aaf1e5a20e
commits: 6

tech-stack:
  added: []
  patterns:
    - "essential rows are built first and counted with wrapped_rows; findings go through fit_list into the rows left (the form sizes content to the popup, the popup guard disarms y when the essentials cannot fit)"
    - "the form predicts the launcher's verdict with the launcher's own readers (control_line) and the editor's own line numbering (lint the body)"

key-files:
  created: []
  modified:
    - src/ui/screens/driver_launch.rs
    - src/app.rs
    - .planning/phases/30-driver-tab-on-direction-md/30-UI-SPEC.md

key-decisions:
  - "[inferred — audit] Launch form row order: title, spend paragraph, fresh-per-item line, sandbox rows, control / lint headline / refusal, six fields, minutes hint, then findings, then the key row. Deviates from the UI-SPEC sketch (spend line last); the UI-SPEC is updated."
  - "[inferred — audit] FORM_FINDINGS_MAX_ROWS = 7 (the old 3 + 3 + a count row); the per-class `… k more` rows are replaced by the one count row fit_list adds."
  - "[inferred — audit] When the essentials fill the room exactly, no findings and no count row are drawn (a count row would clip a confirmation whose essentials are all visible and disarm y)."
  - "[inferred — audit] Left-unlaunched copy: `{run id} created, not launched: [R] launches it.` (58 cells for an 18-character id)."
  - "[inferred — audit] Unregistered-alias refusal copy: `this project is no longer registered. Nothing was created.`; the no-event-channel copy is the existing one. Both go through finish_new_run."
  - "[inferred — audit] Typed model bound is 64 characters (the recorded-model rule), replacing the old 100."
  - "[inferred — audit] Info findings IN-02 and IN-03 are included, as the plan states."
  - "[inferred — audit] control_blocking compares the control word exactly (`!= \"run\"`), as launcher preflight_direction does; `Run` now blocks where it used to be treated as run."

requirements-completed: [DTAB-03, DTAB-04]

coverage:
  - id: D1
    description: "Launch form shows the spend paragraph and sandbox row before findings; at 80x18 with six warnings the warnings are cut to what fits plus a count row and y still launches; at 80x24 everything is visible"
    requirement: DTAB-04
    verification:
      - kind: unit
        ref: "src/ui/screens/driver_launch.rs#launcher_launch_the_spend_and_sandbox_rows_outlive_the_findings"
        status: pass
    human_judgment: false
  - id: D2
    description: "A launch form cut off at 80x12 shows the spend paragraph, the sandbox row and the cut-off hint, and y / Enter on Yes launch nothing; n still leaves"
    requirement: DTAB-04
    verification:
      - kind: unit
        ref: "src/ui/screens/driver_launch.rs#launcher_launch_a_form_cut_off_at_80x12_launches_nothing"
        status: pass
    human_judgment: false
  - id: D3
    description: "A degraded host at 80x24 keeps the two Red sandbox rows and the [U] row visible and y refuses"
    requirement: DTAB-04
    verification:
      - kind: unit
        ref: "src/ui/screens/driver_launch.rs#launcher_launch_a_degraded_host_at_80x24_still_shows_the_two_red_rows_and_u"
        status: pass
    human_judgment: false
  - id: D4
    description: "Check DIRECTION.md at 80x12 with six problems fits what it can, counts the rest, keeps Edit it again now? and y re-opens the editor"
    requirement: DTAB-04
    verification:
      - kind: unit
        ref: "src/ui/screens/driver_launch.rs#launcher_launch_the_lint_popup_keeps_its_question_at_80x12"
        status: pass
    human_judgment: false
  - id: D5
    description: "The slug not-usable copy is readable to its last word at 80 columns and stays on one row at 120; the unlaunched copy fits 62 cells"
    requirement: DTAB-04
    verification:
      - kind: unit
        ref: "src/ui/screens/driver_launch.rs#launcher_new_run_the_not_usable_copy_wraps_to_its_last_word"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver_launch.rs#launcher_new_run_the_unlaunched_copy_fits_the_status_bar"
        status: pass
    human_judgment: false
  - id: D6
    description: "A new-run request refused before it starts (unregistered alias, no event channel) closes the slug input and says Nothing was created."
    requirement: DTAB-04
    verification:
      - kind: unit
        ref: "src/app.rs#launcher_new_run_a_refused_request_closes_the_slug_input"
        status: pass
    human_judgment: false
  - id: D7
    description: "Lint line numbers in the launch preflight and the new-run check count from the first body line, equal to what apply_edit reports"
    requirement: DTAB-03
    verification:
      - kind: unit
        ref: "src/ui/screens/driver_launch.rs#launcher_launch_preflight_numbers_lint_lines_like_the_editor"
        status: pass
      - kind: unit
        ref: "src/app.rs#launcher_new_run_check_numbers_lint_lines_like_the_editor"
        status: pass
    human_judgment: false
  - id: D8
    description: "The form reads control with the launcher's control_line and blocks exactly when the launcher would; a typed model that looks like a flag is refused with must be a model name"
    requirement: DTAB-04
    verification:
      - kind: unit
        ref: "src/ui/screens/driver_launch.rs#launcher_launch_preflight_reads_control_like_the_launcher"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver_launch.rs#launcher_launch_a_typed_model_that_looks_like_a_flag_is_refused"
        status: pass
    human_judgment: false
  - id: D9
    description: "UI-SPEC rows for the three shortened status copies and the Launch form section match the shipped strings and row order"
    requirement: DTAB-04
    verification: []
    human_judgment: true
    rationale: "A documentation edit; no test compares the spec to the code. Greps confirm the three shipped strings are present once each."

duration: 13 min
completed: 2026-10-21
---

# Phase 30 Plan 09: Launch form priority order, new-run fit and launcher agreement Summary

**The launch form now states the spend ceiling, the detach and the sandbox posture before anything else and fits its lint findings to the rows left (a cut-off form cannot launch), the slug input wraps at 80 columns, a refused new-run request closes the input, and the preflight numbers lines like the editor and reads `control` and the model the way the launcher does.**

## Performance

- **Duration:** 13 min
- **Started:** 2026-10-21T04:47:52Z
- **Completed:** 2026-10-21T05:01Z
- **Tasks:** 3 (1 tracer, 2 auto), each with a RED and a GREEN commit
- **Files modified:** 3 (`src/ui/screens/driver_launch.rs`, `src/app.rs`, `30-UI-SPEC.md`)

## Accomplishments

- **WR-05 / UI D5 closed.** `body_lines` is split into `essential_lines` and `finding_lines`; `fitted_body_lines(ctx, area)` counts the essentials with `wrapped_rows`, gives the findings `min(rows left, FORM_FINDINGS_MAX_ROWS)` through `fit_list`, and appends the key row only when a row is free. The spend paragraph and the sandbox row are always before the findings; at 80x12 the essentials do not fit, the popup is clipped and the 30-06 guard keeps `y` inert. `Check DIRECTION.md` fits its problems to the rows left and keeps `Edit it again now?`.
- **UI D8, UI D4, IN-02 closed.** The slug input's second row is a wrapping `Paragraph` (footer height `1 + line_count`, capped at half the screen); the unlaunched copy is 58 cells for an 18-character id; the two Linux early returns in `request_new_run` go through `finish_new_run`, so the input pops and the reason shows.
- **WR-04, IN-03, WR-01 closed.** Preflight and new-run check lint the body (editor numbering); `control` comes from `control_line`; `recorded_model_ok` is now `model_name_ok` and also judges a typed model.
- **UI-SPEC** updated: the three shortened copies, the spend line, the Launch form row order and bullets, a cut-off confirmation line.

## Task Commits

| Task | RED | GREEN |
|------|-----|-------|
| 1 (tracer) launch form priority order, lint popup | `879dbda` test | `65d04da` feat |
| 2 new-run fit, refused request, UI-SPEC | `77076c7` test | `7d8fd67` feat |
| 3 body numbering, control reader, model rule | `b2a9d29` test | `9ad47bd` fix |

Tracer verified end-to-end before expansion: the Task 1 `<verify>` (lib tests for `screens::driver_launch::` and `render_escape_guard`, `driver_tab_launch_e2e`, clippy) passed.

## TDD Gate Compliance

Each task has a `test(30-09)` commit followed by a `feat`/`fix(30-09)` commit. RED evidence (Rust `cargo test` text output; the `tdd-red-evidence` classifier has no adapter for it, so the semantic assessment is recorded here):

- Task 1: three tests failed on the planned assertions: the 80x18 spend paragraph was cut off behind the warnings, the 80x12 form lost its spend paragraph, and the 80x12 lint popup was clipped without its question. The 80x24 half and the essential-row count (11) passed by design.
- Task 2: five tests failed: old long copy (`the_copy_is_the_spec_text`, `app` n-leaves test, the new unlaunched test), unwrapped slug row, and `No registered project named 'ghost'` leaving the input open.
- Task 3: four tests failed on the planned assertions (`-x` accepted, whole-file line numbers `DIRECTION.md:26:` vs `:1:`, `pause` read from a stray line).

## Row budget (the inferred essential count)

At 80x18: `body_room` = inner width 70, 13 body rows. Essentials at the default form: title 1 + spend paragraph 3 (146 cells at `$10.00`) + `SANDBOX_OK` 1 + six fields 6 = **11** (asserted in the test), leaving 2 finding rows. The planned height 18 is kept; the mid-height test shows one warning plus `… 5 more`. No height change was needed.

## Deviations from Plan

**1. [Rule 3 - Blocking] `body_lines` became `#[cfg(test)]`**
- **Found during:** Task 1
- **Issue:** with `render` drawing `fitted_body_lines`, the full unfitted `body_lines` is used only by tests and `-D warnings` rejects dead code.
- **Fix:** gated it with `#[cfg(test)]`.
- **Files modified:** `src/ui/screens/driver_launch.rs` | **Commit:** `65d04da`

**2. [Rule 1 - Bug in plan] Acceptance grep for the unlaunched copy counts tests**
- **Found during:** Task 2
- **Issue:** `grep -c 'created, not launched: \[R\] launches it.' src/ui/screens/driver_launch.rs` is 1 in the plan, but the required tests in the same file assert the string, so the count is 4 (doc comment, format string, two tests). The copy itself exists exactly once in non-test code.
- **Fix:** none needed; noted.

**3. [Scope note] `cargo fmt` touched Task 1 formatting**
- Task 2's commit includes rustfmt-only changes to a few lines of Task 1's code in `driver_launch.rs`.

**4. [Scope note] The lint also flags a stray `control:` outside `## Direction`.** In the IN-03 scenario the run is still refused for the problem `driver check` reports; what changed is that the form's `control is {word}` row no longer fires on a line the launcher's reader ignores.

**Total deviations:** 1 auto-fixed (blocking), 3 notes. **Impact:** none on behaviour.

## Inferred Decisions `[inferred — audit]`

See `key-decisions`: row order, `FORM_FINDINGS_MAX_ROWS = 7`, no findings or count row when the essentials fill the room, the unlaunched copy, the unregistered-alias refusal copy, the 64-character model bound, info findings IN-02 and IN-03 included, exact `control` comparison.

## Verification

- `rtk proxy cargo test --no-fail-fast`: **4068 passed, 1 failed, 24 ignored**. The one failure is the expected environment-only `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`. (Baseline 4057 + 11 new tests.) The flaky `launcher_new_run_the_footer_names_launch_or_relaunch_for_the_selected_row` did not fail.
- `rtk proxy cargo test --no-fail-fast --test driver_tab_launch_e2e --test launcher_e2e`: 2 + 29 passed.
- `rtk proxy cargo clippy --all-targets -- -D warnings`: clean. `cargo fmt --check`: clean.
- `git diff --stat bd078a7..HEAD -- src/launcher src/cli.rs Cargo.toml Cargo.lock`: empty.

## Threat Surface

T-30-29 (launch body clipping), T-30-30 (typed model into argv) and T-30-31 (control verdict) are mitigated as planned. No new network, auth, file or schema surface; every value still passes through `render_for_terminal` (`shown`). No threat flags.

## Known Stubs

None. No entries appended to `.planning/WINDOWS.md`.

## Next

Last plan of phase 30: ready for phase verification.

## Self-Check: PASSED

- Files: `src/ui/screens/driver_launch.rs` (`fn fitted_body_lines`, `fn model_name_ok`), `src/app.rs` (`launcher_new_run_a_refused_request_closes_the_slug_input`), `30-UI-SPEC.md` (three shipped strings, one each) exist and carry the changes above.
- Commits `879dbda`, `65d04da`, `77076c7`, `7d8fd67`, `b2a9d29`, `9ad47bd` are ancestors of HEAD; `git rev-list --count bd078a7..HEAD` = 6.
- All task `<acceptance_criteria>` re-run green except the literal grep count noted in deviation 2.
