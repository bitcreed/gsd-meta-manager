---
phase: 30-driver-tab-on-direction-md
plan: 04
subsystem: ui
tags: [rust, ratatui, tui, steering, direction-md, editor, write-body, flock]

requires:
  - phase: 30-driver-tab-on-direction-md
    provides: 30-01 launcher-first tab and DriverKeys footer; 30-02 steer::read_items, merged_items, the item cursor and launcher_log.park_rules; 30-03 the tiered composition the steering keys sit on
  - phase: 27-driverctl-driver-frontmatter
    provides: driver::direction::write_body (DIRECTION.lock plus the RENAME_EXCHANGE verify loop), split_frontmatter, body_sane, parse_direction
provides:
  - "driver::steer::{set_control, answer, unhold, validate_answer, apply_op, steer_run}: pure body transforms and the one blocking write, a write_body closure"
  - "driver::steer::{prepare_edit, apply_edit, live_body, discard_edit}: the body-only edit temp (O_EXCL, 0600) and a no-clobber apply that compares the live body with the base twice"
  - "Driver-tab keys c (control chooser), a (answer), u (un-hold), e (edit in $VISUAL/$EDITOR); i retired; polite stop behind a confirmation"
  - "App::after_editor(path, EditorExit), App::edit_sessions, EditSession, and the Action::DriverSteer*/DriverEdit* family"
affects: [30-05, 31]

actuals:
  tokens: 58000
  tasks: 3
  commits: 4
plan_head_before: e72c94d969012caed5d99c3725e49925d194a52f
plan_head_after: cda180ab0ade60b4f378b27d6a9e4f95cfb977e0

tech-stack:
  added: []
  patterns:
    - "every DIRECTION.md write the tab makes is a write_body closure on spawn_blocking; the key handlers and screens contain no file write (a test pins it)"
    - "transforms are idempotent and stateless across closure re-runs; each run recomputes refusal and changed from the live body"
    - "offsets are read from the comment-blanked body and applied to the original through splice_ranges, which refuses an out-of-order or off-boundary range instead of panicking a blocking task"
    - "the edit temp is created with create_new, read back only as a regular file within 1 MiB, and the path removed on discard is built from the run directory, never from an action"
    - "a not-live-yet write is worded as written, never as applied: nothing says paused, stopped or sent for an un-echoed control"

key-files:
  created:
    - src/ui/screens/driver_steer.rs
  modified:
    - src/driver/steer.rs
    - src/action.rs
    - src/app.rs
    - src/main.rs
    - src/ui/screens/mod.rs
    - src/ui/screens/detail.rs
    - src/ui/screens/driver_run.rs
    - src/ui/screens/help.rs
    - src/ui/screens/render_escape_guard.rs

key-decisions:
  - "SC3 is met through the flock-taking primitive: every edit is a write_body closure under DIRECTION.lock. When the 2 s lock budget elapses write_body proceeds without the lock (27 D-20); the tab surfaces that as ' Written without DIRECTION.lock (another writer held it); the exchange check kept every save.' on every written message. VERIFICATION should record this nuance: the lock is best-effort, the exchange-verify is what prevents loss."
  - "The chooser's s replaces itself with the polite-stop confirmation (ScreenAction::Replace), so n and Esc return to the tab rather than to a chooser the user already left; a body that already says stop skips the confirmation."
  - "steer_run returns the live body unchanged on a refusal or a no-op, so write_body still performs one byte-identical exchange; the cost is one extra watcher event, the benefit is that the transform sees the body under the lock."
  - "The editor apply and every steering write share one gate, App::steering_target (experimental flag, Linux, registered project, plain run id, event channel); start_apply keeps its own non-Linux refusal."

patterns-established:
  - "EditSession { alias, run_id, run_dir, base } keyed by the temp path in App::edit_sessions: plan 30-05's new-run flow chains by sending Action::DriverEditReady (which records the session and sets pending_editor) and finishing in App::finish_edit on Action::DriverEditDone { alias, run_id, temp, outcome: EditOutcome }"
  - "steered_message / edit_done_message / editor_failed_message / park_warning are the pure copy functions, so every string is asserted without an App"

requirements-completed: [DTAB-03, DTAB-04]

coverage:
  - id: D1
    description: "c opens the control chooser; r and p write control at once, choosing the word the body already has writes nothing and says so, and the write rewrites every uncommented control line (or inserts one under ## Direction) leaving the tool block byte-identical; the header then reads '{word} (pending)' until the frontmatter echoes it."
    requirement: "DTAB-03"
    verification:
      - kind: e2e
        ref: "src/app.rs#launcher_steer_c_then_p_shows_pending"
        status: pass
      - kind: unit
        ref: "src/driver/steer.rs#launcher_steer_two_control_lines_both_follow_the_word"
        status: pass
      - kind: unit
        ref: "src/driver/steer.rs#launcher_steer_a_control_write_changes_only_the_body"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver_steer.rs#launcher_steer_choosing_the_current_word_writes_nothing_and_says_so"
        status: pass
    human_judgment: false
  - id: D2
    description: "a answers the cursor item (parked or hold: true, present in the body) and writes '- answer:' plus '- hold: false' into that item's segment; u writes hold: false at once; a missing line is appended to the segment; every item id survives; the OQ-R1 Yellow row and the written message name a park rule an answer does not lift."
    requirement: "DTAB-03"
    verification:
      - kind: unit
        ref: "src/driver/steer.rs#launcher_steer_answer_lifts_hold_and_keeps_every_item"
        status: pass
      - kind: e2e
        ref: "src/app.rs#launcher_steer_a_then_enter_answers_the_parked_item_and_warns_about_the_rule"
        status: pass
      - kind: e2e
        ref: "src/app.rs#launcher_steer_u_takes_the_held_item_off_hold"
        status: pass
      - kind: unit
        ref: "src/driver/steer.rs#launcher_steer_a_missing_hold_or_answer_line_is_appended_to_the_segment"
        status: pass
    human_judgment: false
  - id: D3
    description: "An answer is trimmed and refused when empty, over 500 chars, or holding a control character, CR, LF, U+2028, U+2029, '<!--' or '-->'; '|' is allowed; the refusal keeps the buffer and the screen and writes nothing."
    requirement: "DTAB-03"
    verification:
      - kind: unit
        ref: "src/driver/steer.rs#launcher_steer_validate_answer_table"
        status: pass
      - kind: unit
        ref: "src/driver/steer.rs#launcher_steer_the_answer_cap_is_five_hundred_chars_not_bytes"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver_steer.rs#launcher_steer_a_refused_answer_keeps_the_buffer_and_the_screen_and_writes_nothing"
        status: pass
    human_judgment: false
  - id: D4
    description: "The polite stop is a confirmation with default No whose only y writes control: stop; i opens nothing and names [c] [a] [e]; the help screen has no i row and no Injected Message States legend, with the flag on or off."
    requirement: "DTAB-04"
    verification:
      - kind: unit
        ref: "src/ui/screens/driver_steer.rs#launcher_steer_only_y_on_the_polite_stop_writes_control_stop"
        status: pass
      - kind: unit
        ref: "src/ui/screens/detail.rs#launcher_steer_i_is_retired_opens_nothing_and_names_the_steering_keys"
        status: pass
      - kind: unit
        ref: "src/ui/screens/help.rs#launcher_steer_the_help_has_no_injection_row_and_no_injected_message_legend"
        status: pass
    human_judgment: false
  - id: D5
    description: "e hands a body-only O_EXCL 0600 temp (.DIRECTION.md.edit) to the shipped editor hand-off; on a clean exit the edit is applied only if the live body still equals the base (checked before and inside the write); a conflict, an insane body or lint problems write nothing and keep the text with re-open, discard and keep keys; a failed editor keeps the temp and says where; a stale or symlinked temp is never overwritten or followed; the watcher ignores the temp and the splice sweep never deletes it."
    requirement: "DTAB-03"
    verification:
      - kind: e2e
        ref: "src/app.rs#launcher_edit_e_hands_the_body_only_temp_to_the_editor_and_applies_a_clean_save"
        status: pass
      - kind: e2e
        ref: "src/app.rs#launcher_edit_a_conflict_opens_the_chooser_and_reopen_rebases_on_the_live_body"
        status: pass
      - kind: unit
        ref: "src/driver/steer.rs#launcher_edit_a_save_landing_while_the_write_waits_for_the_lock_is_kept"
        status: pass
      - kind: unit
        ref: "src/driver/steer.rs#launcher_edit_a_planted_symlink_is_a_stale_temp_and_never_followed"
        status: pass
      - kind: unit
        ref: "src/driver/steer.rs#launcher_edit_the_temp_name_is_not_a_splice_temp_and_the_watcher_ignores_it"
        status: pass
      - kind: e2e
        ref: "src/app.rs#launcher_edit_a_failed_editor_keeps_the_temp_and_writes_nothing"
        status: pass
    human_judgment: false
  - id: D6
    description: "When write_body reports lock_timed_out with SpliceOutcome::Ok the write lands exactly once, the tool block is byte-identical, and the message names the missing lock."
    requirement: "DTAB-03"
    verification:
      - kind: unit
        ref: "src/driver/steer.rs#launcher_steer_a_held_lock_past_the_budget_still_writes_once_and_says_so"
        status: pass
    human_judgment: false
  - id: D7
    description: "The control chooser, answer echo, polite-stop prompt, lint findings and edit chooser render hostile identities escaped (render-escape-guard states for all five new screens)."
    requirement: "DTAB-03"
    verification:
      - kind: unit
        ref: "src/ui/screens/render_escape_guard.rs#every_adjudicated_screen_has_a_probe_fixture"
        status: pass
      - kind: unit
        ref: "src/ui/screens/driver_steer.rs#launcher_steer_a_hostile_buffer_and_item_render_escaped"
        status: pass
    human_judgment: false
  - id: D8
    description: "On a non-Linux build c, a, u and e are absent from the footer and inert, the footer carries 'read-only: steering needs Linux', and every App arm that reaches a write refuses with that copy."
    requirement: "DTAB-03"
    verification: []
    human_judgment: true
    rationale: "Only the Linux target is installed. The refusal paths compile on every target (cfg! rather than cfg attributes) and the cfg(not(target_os = \"linux\")) tests exist, but none can run here; the Linux-side footer tests assert the same fields through cfg!."
  - id: D9
    description: "The chooser and popup compositions (one-row versus two-row chooser, the extra note row, the lint list cut at eight) read well at real terminal sizes."
    verification: []
    human_judgment: true
    rationale: "Every structural property is asserted (copy, keys, widths, no clipped keys at 80 columns); whether the layouts are pleasant is a taste call."

duration: 41min
completed: 2026-10-21
status: complete
---

# Phase 30 Plan 04: Steering a Run From the Tab Summary

**The Driver tab now steers a launcher run through its DIRECTION.md body: `c` sets control, `a` and `u` answer or un-hold the cursor item, and `e` edits the body in `$VISUAL`/`$EDITOR` through an O_EXCL temp that is applied only if nobody else changed the body meanwhile; every write is a `write_body` closure under `DIRECTION.lock` that leaves the tool block byte-identical, and the Phase 18 injection key is retired.**

## Performance

- **Duration:** 41 min
- **Started:** 2026-10-21T01:41:00Z
- **Completed:** 2026-10-21T02:23:00Z
- **Tasks:** 3 (4 commits: one follow-up refactor)
- **Files modified:** 10 (1 created)

## Accomplishments

- **Tracer (Task 1):** `set_control` rewrites every uncommented `- control:` line at its word's byte offsets (or inserts one under `## Direction`), with the store's own `parse_direction` as the post-condition; `steer_run` is a `write_body` closure on `spawn_blocking`. `launcher_steer_c_then_p_shows_pending` drives the real key handler and App against a lock-held run: the status says the word was **written**, the header reads `pause (pending)`, the tool block is byte-identical, and nothing says `paused`. The tracer was re-verified end to end before expanding.
- **Answer, un-hold, polite stop, injection retired (Task 2):** `validate_answer` (trim; refuse empty, control chars, U+2028/2029, `<!--`, `-->`, over 500 chars; `|` allowed), `answer`/`unhold` operating on the item's own segment with an id-set post-condition, `AnswerScreen` (with the Yellow OQ-R1 row for a park rule an answer does not lift), `PoliteStopScreen` (default No), `i` retired, help gains `c`/`a`/`u` and loses the `i` row and the `Injected Message States` legend.
- **Editor flow (Task 3):** `prepare_edit` / `apply_edit` / `live_body` / `discard_edit`, `App::after_editor(path, EditorExit)` with `App::edit_sessions`, the chooser and lint-confirmation screens, and the exit status plumbed from `main.rs` with the `$VISUAL`/`$EDITOR`/`vi` resolution untouched. The apply compares the live body with the base before the write and again inside the `write_body` closure; a save that lands while the write waits for the lock is kept.
- **Cfg hygiene (stated for the plan's verification):** every App arm that reaches `steer_run`, `prepare_edit` or `apply_edit` has a non-Linux refusal: `request_steer`, `request_edit`, `request_reopen` and `request_discard` go through `App::steering_target`, and `start_apply` refuses on its own. Reviewed by hand and by the Linux footer tests (`cfg!`); only the Linux target is installed.

## Task Commits

1. **Task 1 (tracer): c then p writes control: pause through write_body and the header shows it pending** - `c93f4da` (feat)
2. **Task 2: answer and un-hold the cursor item, confirm the polite stop, retire the injection key** - `1dbb532` (feat)
3. **Task 3: edit DIRECTION.md in the user's editor through a body-only temp that never clobbers** - `66c4b8e` (feat)
4. **Follow-up: the edit apply keeps its own non-Linux refusal** - `cda180a` (refactor)

**Plan metadata:** committed with this file (docs: complete plan)

## Files Created/Modified

- `src/driver/steer.rs` - `SteerOp`/`SteerError`/`AnswerRule`, `set_control`, `answer`, `unhold`, `validate_answer`, `apply_op`, `SteerOutcome`, `steer_run`, `prepare_edit`, `apply_edit`, `live_body`, `discard_edit`, `EditOutcome`
- `src/ui/screens/driver_steer.rs` (new) - `ControlChooserScreen`, `PoliteStopScreen`, `AnswerScreen`, `EditChooserScreen`, `EditLintConfirmScreen`, and the copy functions
- `src/action.rs` - `DriverSteerRequested`/`DriverSteered`, `DriverEditRequested`/`Ready`/`Stale`/`Failed`/`Reopen`/`Apply`/`Done`/`Discard`/`Discarded`
- `src/app.rs` - `EditorExit`, `EditSession`, `App::{edit_sessions, after_editor}`, the request/finish handlers and their tests
- `src/main.rs` - builds `EditorExit` from the existing status match and calls `app.after_editor`
- `src/ui/screens/detail.rs` - `c`, `a`, `u`, `e` arms (all `current_view == Driver`), `i` retired, footer tokens `[c]ontrol`, `[a]nswer`, `[u]nhold`, `[e]dit` and the non-Linux suffix
- `src/ui/screens/driver_run.rs` - `SteerTarget`, `steer_refusal`, `cursor_item_row`, `park_rule_for`, `ItemRow::{is_parked, in_body, answerable}`, `DriverKeys::{control, answer, unhold, edit, read_only_platform}`, `INJECTION_RETIRED`
- `src/ui/screens/help.rs` - `c`/`a`/`u`/`e` rows (with the blocking-editor note), injection row and legend removed
- `src/ui/screens/render_escape_guard.rs`, `src/ui/screens/mod.rs` - five new rows and fixtures; `pub mod driver_steer`

## Decisions Made

- `[inferred — audit]` **Refused-transform copy.** UI-SPEC has no row for a transform refusal (an item that vanished between the key and the write): it reads `Nothing was written: {reason}.` and is shown as an error, not a status.
- `[inferred — audit]` **Key refusals the spec does not word.** `a` with no items: `This run has no items, so there is nothing to answer.`; `a` on a parked item only the frontmatter knows: `{id} is not in DIRECTION.md, so it cannot be answered. Add it with [e].` (it has no segment to write into); a steering key on an unreadable run: `{id} could not be read, so it cannot be steered.`
- `[inferred — audit]` **Answer text.** Trimmed before validation, so a pasted trailing newline is accepted rather than refused; the 500-char cap counts `char`s; a control character typed or pasted into the buffer is accepted into it and refused on Enter, so the refusal copy is reachable.
- `[inferred — audit]` **`unhold`/`answer` target the line `read_items` reads** (the first line with a value, else the first empty one), so the post-condition and the display agree; a duplicate `- hold:` further down is left alone.
- `[inferred — audit]` **Chooser rendering.** The UI-SPEC's one-row chooser is used when it fits the width; below that the message and the keys take a row each (so the keys are never clipped), and a muted note row says that `r` re-opens against the live body and saving replaces what is there. A stale temp's path is cut from the left to fit.
- `[inferred — audit]` **Lint findings are body-relative.** `driver check` lints the edit body alone, so `DIRECTION.md:{n}` is the line in the temp the user was editing, not a line in the file with the frontmatter above it.
- `[inferred — audit]` **Discard feedback.** `Discarded your edit. DIRECTION.md was not changed.` arrives through a new `Action::DriverEditDiscarded` so the message is only shown after the removal happened; `Esc` in the chooser says `Your text is kept at {path}.`
- `[inferred — audit]` **`s` on a body that already says `stop`** takes the already-that-word path (no confirmation, nothing written).
- `[inferred — audit]` **Re-open and discard derive the temp path from the run directory,** not from the action's `temp`, so only `<run>/.DIRECTION.md.edit` can ever be read or removed.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Existing tests constructed `DriverKeys` and asserted the retired `i` flow**
- **Found during:** Tasks 1-3
- **Issue:** five `DriverKeys { .. }` literals and three bit-mask loops in `detail.rs` tests no longer compiled once the struct gained fields; three `pressing_i_*` tests asserted the Phase 18 injection screen; two `help.rs` tests asserted the injection legend and the single `read` in its gloss.
- **Fix:** the literals gained the new fields and the loops widened (`0..2048`); the `i` tests became one test asserting the retirement message, no screen and no dispatch for a live observed run, a launcher run and a legacy row; the legend tests became absence tests with the flag on and off; the `read`-count assertion went with its queued gloss.
- **Files modified:** `src/ui/screens/detail.rs`, `src/ui/screens/help.rs`
- **Committed in:** `c93f4da`, `1dbb532`, `66c4b8e`

**2. [Rule 2 - Missing Critical] `Action::DriverEditDiscarded` added**
- **Found during:** Task 3
- **Issue:** the plan's variant list has no way to report a discard back, so the success message would have been shown before the removal happened and a failed removal would have been silent.
- **Fix:** one extra plain-data variant answered by the blocking removal.
- **Files modified:** `src/action.rs`, `src/app.rs`
- **Committed in:** `66c4b8e`

**3. [Rule 2 - Missing Critical] Stale-temp path clipped the chooser keys**
- **Found during:** Task 3 (App test at 120 columns)
- **Issue:** a long project path pushed `[r]e-open it ...` off the row.
- **Fix:** the path is cut from the left (`tail_cells`) to the room the row has.
- **Files modified:** `src/ui/screens/driver_steer.rs`
- **Committed in:** `66c4b8e`

**4. [Rule 2 - Missing Critical] `start_apply` refusal on non-Linux**
- **Found during:** final cfg review
- **Issue:** the apply path relied on the arms that open a session for its non-Linux refusal.
- **Fix:** `start_apply` refuses itself.
- **Files modified:** `src/app.rs`
- **Committed in:** `cda180a`

### Plan-vs-implementation notes (not defects)

- **Acceptance grep `grep -c '"ControlChooserScreen"' render_escape_guard.rs` is 1:** it is 2, because every adjudicated screen has a row in `SCREEN_IDENTITY_DISPOSITIONS` and an arm in `fixture_for` (`DriverStartScreen` is 2 as well). Both are present.
- **Acceptance grep `grep -c 'Message injection is retired' detail.rs` is at least 1:** the string lives in `driver_run::INJECTION_RETIRED` (production) and is asserted by literal in the `detail.rs` test, which is the only `detail.rs` occurrence.
- `steer::tests` is `pub(crate)` (test-only) so the App tests reuse `write_run` and `clean_body`.

---

**Total deviations:** 4 auto-fixed (1 blocking, 3 missing critical). **Impact on plan:** no scope change; test updates the retired key forced, one report variant, one rendering guard and one refusal.

## Issues Encountered

- A full run additionally failed `the_next_shim_refuses_any_other_verb_without_running_the_binary` (`tests/launcher_config_dir.rs`, `Text file busy` writing and executing a shim under parallel load). The file is untouched by this plan, it passed 3 of 3 in isolation and in the preceding full run: a third load-sensitive flake alongside the two 30-03 recorded (`driver::lock` `probe_at`, `envelope_pr_cap` boundary). Noted for the phase verifier.
- No way to run the non-Linux arms here (only the Linux target is installed).

## Verification

- `mkdir -p -m700 /tmp/claude-1000/t; umask 077; TMPDIR=/tmp/claude-1000/t rtk proxy cargo test --no-fail-fast`: **3958 passed, 1 failed, 24 ignored** on the code of `66c4b8e` (baseline after 30-03: 3870 passed, 1 failed, 24 ignored; +88 tests); the one failure is the expected git-version witness `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`. The final run on `cda180a` read 3957 passed, 2 failed, 24 ignored: the same witness plus the `Text file busy` flake above, which passes in isolation.
- `rtk proxy cargo clippy --all-targets -- -D warnings`: **clean**. `cargo fmt --check`: clean.
- Per-task filters: Task 1 `driver::steer:: launcher_steer_ screens::driver_steer:: render_escape_guard` 56 passed (including `launcher_steer_c_then_p_shows_pending`); Task 2 `... screens::detail:: screens::help::` 500 passed; Task 3 `... launcher_edit_ classify_change` 536 passed and `--test async_blocking_guard --test spawn_seam_guard` 9 + 41 passed.
- Acceptance greps: `pub fn set_control` 1, `write_body(` in steer.rs 1, `pub fn validate_answer` 1, `DIRECTION_ANSWER_MAX_CHARS` in steer.rs 6, `heading("Injected Message States")` 0 (baseline 1), `DriverInjectScreen::new` in detail.rs 0 (baseline 1), file-write calls in driver_steer.rs 0, `fn apply_edit` 1, `create_new(true)` 1, `app.after_editor(` in main.rs 1, `refresh_after_editor` in main.rs 0 (baseline 1), `std::env::var("VISUAL")` in main.rs 1, `pub enum EditorExit` 1.

## Known Stubs

None.

## Threat Flags

| Flag | File | Description |
|------|------|-------------|
| threat_flag: file-access (residual TOCTOU) | src/driver/steer.rs | `apply_edit` checks the temp with `symlink_metadata` (regular file, at most 1 MiB) and then opens it by path; a process able to write the run directory could swap a symlink in between. The worst outcome is that the swapped-in file's UTF-8 text is offered to the lint and, with a base-matching live body, written as the body (never the tool block); the user sees the lint confirmation or an unchanged-body result first. Within T-30-12; noted for the 28-SECURITY follow-up with the 30-02 flag of the same shape. |

## User Setup Required

None - no external service configuration required.

## Next Phase Readiness

- Ready for 30-05: `s`, `R` and the `s` help row are untouched. The new-run flow chains onto the editor by sending `Action::DriverEditReady { alias, run_id, temp, base }` (it records the `EditSession` and sets `pending_editor`) and by finishing in `App::finish_edit` on `Action::DriverEditDone { alias, run_id, temp, outcome: EditOutcome }`; `EditSession { alias, run_id, run_dir, base }` is keyed by the temp path in `App::edit_sessions`. `prepare_edit(run_dir)` is the library call that makes the temp.
- For the phase verifier: SC3's flock requirement is met by routing every edit through `write_body`; the 2 s lock-timeout fallback is surfaced in the message, not hidden (D6). The three load-sensitive flakes above. D8 and D9 are the two human-judgement deliverables.

## Self-Check: PASSED

- Created/modified files exist (10 of 10 checked); commit hashes `c93f4da 1dbb532 66c4b8e cda180a` are ancestors of HEAD; `check evaluation-scope --plan 30-04 --commits-only` resolves with no missing, outside-union or unreachable entries; acceptance greps for all three tasks hold (the two count notes above); no deletions in any of the four commits (post-commit deletion check empty).

---
*Phase: 30-driver-tab-on-direction-md*
*Completed: 2026-10-21*
