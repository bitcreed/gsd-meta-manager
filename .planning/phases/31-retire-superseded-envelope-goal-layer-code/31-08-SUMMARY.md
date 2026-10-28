---
phase: 31-retire-superseded-envelope-goal-layer-code
plan: 08
subsystem: tui-safety-text
tags: [rust, gap-closure, code-review, tui, opt-in, unregister-guard]

requires:
  - phase: 31-retire-superseded-envelope-goal-layer-code
    provides: 31-01..06 deleted the goal layer and the legacy drive path, which left WR-01..03 (31-REVIEW.md) describing or depending on controls that no longer exist
provides:
  - do_remove_project refuses to unregister a project whose launcher run holds its run lock (WR-03)
  - the Driver tab's opt-in grant popup shows only the consent prompt; the retired-seam disclosure renderer is gone (WR-01)
  - every statement about the opt-in record names the PR caps only; a configured namespace is documented as validated but unused (WR-02)
  - PromptInputsDrifted's Display and doc promise no review (WR-01 message half)
affects: [31-07, 31-09]

actuals:
  tokens: 12500
  tasks: 3
  commits: 3

plan_head_before: f1f7b230d7d9d81e69fe1123473bcf5857004bca
plan_head_after: 89f4b3b6f89e3c60ac4652e8eb66303a9ddc1150
commits: 3

tech-stack:
  added: []
  patterns:
    - "a guard on an AppContext mutation reads the cached launcher marker, gated on ctx.experimental, exactly as needs_attention_for does"
    - "census pins are re-measured in the same commit as the deletion that moves them, never relaxed"

key-files:
  created: []
  modified:
    - src/ui/screens/delete_confirm.rs
    - src/ui/screens/driver_confirm.rs
    - src/ui/screens/render_escape_guard.rs
    - src/ui/screens/mod.rs
    - src/ui/screens/driver.rs
    - src/ui/mod.rs
    - src/text.rs
    - src/error.rs
    - src/registry.rs
    - src/config.rs
    - docs/CONFIGURATION.md

key-decisions:
  - "WR-01 takes the review's first fix option: the disclosure block is dropped from the grant popup, not reworded [inferred - audit]"
  - "WR-02 takes the text fix: claims corrected, DriverOptIn::branch_namespace stays parsed and validated [inferred - audit]"
  - "WR-03 guard is gated on ctx.experimental and reads the cached launcher marker [inferred - audit]"
  - "driver_confirm.rs stays in the driver.rs CENSUS_FILES although it now holds no needle call [inferred - audit]"

requirements-completed: [RETIRE-01]

duration: 40min
completed: 2026-10-27
status: complete
---

# Phase 31 Plan 08: Review warnings WR-01..03 Summary

**Unregister is refused while a launcher run is live, and the opt-in popup, prompts, config doc and drift error stop describing deleted controls (retired seam disclosure, branch-namespace confinement, a re-confirmation nobody performs).**

## Performance

- **Duration:** about 40 min
- **Tasks:** 3 of 3
- **Files modified:** 11 (245 insertions, 421 deletions)

## Fixing commit per finding (for 31-09 Task 3 to copy into 31-REVIEW-DISPOSITION.md)

| Finding | Status | Fixing commit |
|---------|--------|---------------|
| WR-03 unregister guard | fixed | `36e0805` |
| WR-01 grant popup disclosure (UI side) | fixed | `12b8a1d` |
| WR-01 drift message and registry digest docs (message side) | fixed | `89f4b3b` |
| WR-02 branch-namespace claims | fixed | `89f4b3b` |

## Accomplishments

- **Task 1 (tracer, WR-03, `36e0805`).** `do_remove_project` returns early with `'<alias>' has a live driver run. Stop it from its Driver tab (x) first.` when `ctx.experimental` is on and the cached `launcher_marker` reports `any_live`. The alias goes through `render_for_terminal`. Nothing is removed, saved or touched. Tests: `unregistering_a_project_with_a_live_launcher_run_is_refused` (stays registered, error names the alias, config file never written) and `a_project_whose_launcher_runs_are_not_live_is_still_removed` (Crashed, `any_live: false`, removed with the toast). The no-marker tests are unchanged and green.
- **Task 2 (WR-01 UI, `12b8a1d`).** Deleted `render_disclosure`, `SECTION_PROMPT_INPUTS`, `SECTION_MODEL_SEAM`, `SECTION_RESIDUAL_EXPOSURE` and the six disclosure tests. The grant branch of `render` no longer computes prompt inputs. The record path (`do_toggle_opt_in` / `record_opt_in`) is unchanged. `adjudicate_screen!` reason re-worded; probe fixture relabelled `opt-in grant prompt`; held-by table row removed; doc mentions in `ui/screens/mod.rs` and `text.rs` corrected.
- **Task 3 (WR-02 and WR-01 message, `89f4b3b`).** Both opt-in prompts now say PR caps only. `DriverOptIn::branch_namespace`, the `driver_confirm.rs` module doc, `prompt_text` doc and the CONFIGURATION.md bullet say the namespace is accepted and validated but not used (every run pushes to `refs/heads/gsd-auto/<alias>/<run-id>`). `PromptInputsDrifted` Display now reads "was recorded against different file contents: <drift>. Withdraw and record the opt-in again to refresh it" and its doc names `DrivableProject::from_registry` (no production caller) as the only producer. registry.rs digest docs say nothing on the launcher path consults them.

## RED assertions (TDD)

- **Task 1:** `unregistering_a_project_with_a_live_launcher_run_is_refused` failed at `src/ui/screens/delete_confirm.rs:311`: `a project with a live launcher run must stay registered`. The control test passed before the guard, as expected.
- **Task 2:** `the_grant_popup_shows_no_disclosure_of_retired_seams` failed with `the grant popup still discloses a retired control ("seam")`.
- **Task 3:** `the_opt_in_prompts_say_what_the_record_feeds_and_claim_no_gate` failed:
  - left: `Withdraw the driver opt-in for "proj"? Its PR caps and branch namespace stop applying; the push step falls back to the defaults. [y/n]`
  - right: `Withdraw the driver opt-in for "proj"? Its PR caps stop applying; the push step falls back to the defaults. [y/n]`

## Census re-measures (same commit as the deletion, `12b8a1d`; nothing relaxed)

| Pin | Before | After | Why |
|-----|--------|-------|-----|
| `src/ui/mod.rs` `MEASURED_REACH` | 3 files: driver.rs 1, driver_confirm.rs 2, render_escape_guard.rs 3 (total 6, 3 non-exempt, 2 of 22 files) | 2 files: driver.rs 1, render_escape_guard.rs 3 (total 4, 1 non-exempt, 1 of 22 files) | the renderer's two `display_identity` calls were the last in driver_confirm.rs |
| `driver.rs` non-vacuity (units, occurrences, whole-file lines) | (2, 3, 4) | (1, 1, 2) | the renderer's one path-and-digest unit carried 2 occurrences on 2 lines |

The reach doc table, its prose and the file-count line ("other 20 files") were updated in the same edit. The measured values passed first time, so the re-measure matches the tree.

## Accepted residuals (WR-03)

- The guard reads the cached marker, which is at most one slow tick (20 ticks) stale. A run launched inside that window is not seen.
- Before a project's first marker scan lands the marker is `None` and nothing is guessed, so removal is allowed.
- With the experimental flag off the guard is inert, consistent with D-13: the scan never populates the marker either.

## Inferred decisions `[inferred - audit]`

1. WR-01: drop the disclosure block entirely (the review's first option), not reword it. The digest machinery 31-07 owns is re-documented, not retired.
2. WR-02: correct the claims and leave `branch_namespace` parsed and validated. The failure direction is safe because the default namespace is the stricter one.
3. WR-03: follow the review's code shape, gated on `ctx.experimental`.
4. `driver_confirm.rs` stays in `CENSUS_FILES` of the `driver.rs` census. The plan said to follow "the census's own documented rule for a file that leaves"; the census documents no such rule. Keeping the file is the stricter direction (it still guards against an unconverted call being re-introduced, and its line-count and one-test-module assertions still hold), so nothing was relaxed.

## Deviations from Plan

**1. [Rule 3 - Blocking] Kept the `two_inputs` test helper.**
- **Found during:** Task 2
- **Issue:** the plan says to delete `two_inputs` with the disclosure tests, but `granted_record()` (used by the four WR-04 failed-save tests, which are not disclosure tests) builds its record from it. Deleting it breaks compilation.
- **Fix:** kept `two_inputs`, re-documented as the recorded-list fixture for `granted_record`.
- **Files modified:** src/ui/screens/driver_confirm.rs
- **Commit:** `12b8a1d`

**2. [Rule 3 - Blocking] render_escape_guard.rs prose after the row removal.**
- **Found during:** Task 2
- **Issue:** removing the held-by row left a sentence ("The last two carriers ...", "three values") that counted rows. Rewrote the paragraph; also kept the acceptance grep for `opt-in disclosure` at 0 by wording the replacement as "the grant popup's file list".
- **Commit:** `12b8a1d`

**3. [Rule 3 - Blocking] `ui/mod.rs` assertion message.**
- **Found during:** Task 2
- **Issue:** the `the_census_does_not_report_a_composed_call` failure message named `driver_confirm.rs` as holding a deliberate composition. Updated to name only `screens/driver.rs`; the assertion is unchanged.
- **Commit:** `12b8a1d`

**Total deviations:** 3 auto-fixed (all Rule 3, all text or fixture bookkeeping). **Impact:** none on behaviour.

## Clippy and dead code

`rtk proxy cargo clippy --all-targets -- -D warnings` exits 0. `PromptProfile` and its variants were not reported as unused (they are `pub` items in a lib-visible module, and the `DISCLOSED_PROMPT_INPUTS` table still constructs them), so no `#[allow(dead_code)]` was added.

## Verification (raw counts)

- `rtk proxy cargo test --no-fail-fast --lib -- screens::delete_confirm:: screens::driver_confirm:: screens::driver:: render_escape_guard registry:: config:: error:: text::`: running 129 tests, 129 passed, 0 failed.
- `rtk proxy cargo test --no-fail-fast --lib -- ui:: text::`: running 981 tests, 981 passed (includes the re-measured censuses).
- Full lib: 2625 passed, 0 failed, 1 ignored.
- `rtk proxy cargo clippy --all-targets -- -D warnings`: exit 0.
- Plan-level checks: `fn from_registry` in `src/executor/mod.rs` count 7; `run_states` in `src/ui/screens/mod.rs` count 3; `git diff f1f7b23 -- src/launcher/push.rs src/envelope/policy.rs` is empty; the `src/registry.rs` diff contains only `//` and `///` lines.
- The FULLSUITE (integration tests) and container gate run in plan 31-09 Task 3, as planned.

## D-04 isolation

Plan 31-07 stays HELD. `DrivableProject`, `from_registry`, `check_prompt_input_drift`, `current_prompt_inputs`, `record_opt_in`'s digest recording, `OptInError::PromptInputsDrifted` (variant, fields, `drift.describe()`) and `AppContext::{run_states, exec_tx}` are all present. Only doc comments and the variant's Display text changed.

## Known Stubs

None.

## Threat Flags

None. No new endpoint, auth path or schema change. T-31-25..28 are mitigated as planned (refusal and control tests; popup absence pinned on the rendered buffer; claims corrected with the push step untouched; alias escaped through `render_for_terminal`).

## Self-Check: PASSED

- Commits `36e0805`, `12b8a1d`, `89f4b3b` are ancestors of HEAD.
- Acceptance greps: all Task 1, 2 and 3 criteria hold (render_disclosure 0, the three SECTION_ constants 0, `model seam` 0, `goal-decomposition` 0, `opt-in disclosure` 0 and `with the disclosure` 0 in render_escape_guard.rs, `driver_confirm.rs", 2` 0 in ui/mod.rs, `branch namespace` 0 in driver_confirm.rs, `namespace the push step reads` 0, `PR caps` 1 in CONFIGURATION.md, `review what reaches a prompt` 0, `seen the new disclosure` 0, the config.rs "confined to" sentence 0, `pub branch_namespace` 1, `Called at the spawn gate` 0).
