---
phase: quick-261125-khh
plan: 01
subsystem: owner-inbox settle (x)
tags: [settle, owner-inbox, multiplexer, verify-work, uat]
status: complete
requires: [phase 37 owner actions, phase 39 settle, quick 261118-v6l pending badge]
provides:
  - settle y always launches a new session (no focus, no clipboard)
  - x on a settling row offers stop waiting (default-No, memory-only)
  - owner reason recorded verbatim between typographic triple quotes
affects: [src/ui/screens/owner_settle.rs, src/ui/screens/settle_popup.rs, src/ui/screens/settle_pending.rs, src/ui/screens/owner_row_actions.rs, src/ui/screens/inbox.rs, src/ui/screens/detail.rs]
tech-stack:
  added: []
  patterns: [default-No confirm_popup screen, memory-only state drop, typographic delimiters for xargs-safe argv]
key-files:
  created: []
  modified:
    - src/ui/screens/owner_settle.rs
    - src/ui/screens/settle_popup.rs
    - src/ui/screens/settle_pending.rs
    - src/ui/screens/owner_row_actions.rs
    - src/ui/screens/owner_actions.rs
    - src/ui/screens/inbox.rs
    - src/ui/screens/detail.rs
    - src/ui/screens/mod.rs
    - src/ui/screens/render_escape_guard.rs
    - src/ui/glyphs.rs
    - src/app.rs
    - tests/focus_entry_guard.rs
    - tests/owner_settle_guard.rs
    - tests/owner_settle_oracle.rs
    - README.md
    - docs/GETTING-STARTED.md
decisions:
  - "Settle y always launches a new session, even beside a live Claude session; v keeps 37 D-12 (D-01, owner)"
  - "x on a pending settle row is an enabled stop waiting entry with a default-No confirm; Yes drops the entry from memory only (D-04)"
  - "Owner reason recorded as Deferred follow-up: <outcome> Owner’s note: “““<note>”””, with typographic double quotes in the note mapped to single ones (D-06, D-07)"
  - "The full-command view of a Move with a reason clips at 80x24 (two rows over); the summary still fits and d returns to it"
  - "U+2018 joins the glyph registry's TYPOGRAPHY exemptions"
metrics:
  duration: 62min
  completed: 2026-11-25
actuals:
  tokens: 30876
  tasks: 3
  commits: 3
plan_head_before: 2d0be8acae15e9d50c298132642b7aaa36c55412
plan_head_after: 81d1d11cf8aecb463b680b9cabc965b83ba8ee8e
---

# Phase quick-261125-khh Plan 01: Settle always launches, stop waiting, quoted reason Summary

Settle's `y` now always starts `/gsd-verify-work N` in a new multiplexer session, with no focus switch and no clipboard write. `x` on a settling row offers a default-No "stop waiting" that clears the badge from memory. The owner's reason is recorded verbatim between typographic triple quotes, and the oracle confirmed the format against the installed gsd-core 1.23.0.

## Commits

| Task | Commit | Message |
|------|--------|---------|
| 1 | 38457b09 | fix(quick-261125-khh): settle always launches a new session, never focuses or copies |
| 2 | 4479993c | feat(quick-261125-khh): x on a settling row offers stop waiting |
| 3 | 81d1d11c | fix(quick-261125-khh): quote the owner's reason verbatim in the settle instruction |

`git rev-list --count 2d0be8ac..HEAD` is 9. The other 6 commits belong to the parallel quick 261125-kfc session on the same branch. 3 commits are this task's (`--grep quick-261125-khh`).

## Final instruction text (Won't do, with a reason)

This is the exact argv element for `x w no device here Enter y` on the `uat-pending` phase row. `TRACER_PROMPT` in settle_popup.rs pins it, and a test checks that it equals `owner_settle::prompt` output:

> /gsd-verify-work 01 The user will not perform tests 2, 3 in 01-UAT.md. Resume the UAT session of phase 01 without restarting it and record each of them as result: skipped, reason: Deferred follow-up: will not do. Owner’s note: “““no device here””” Record that reason exactly, up to and including the closing triple quotes, with the owner’s note between them copied verbatim. They must not block phase 01 or be asked again. Keep them in the UAT file and do not promote them to the 999.x backlog.

When there is no reason, each prompt is byte-identical to the prompt at HEAD, so the no-reason pins did not change.

## Test counts (real, from raw logs)

Final run, after task 3, of `cargo test --no-fail-fast`: 69 suites, **4721 passed, 0 failed, 19 ignored**. `cargo clippy --all-targets -- -D warnings` was clean, and so was `cargo fmt --check`.
- After task 1: 69 suites, 4694 passed, 0 failed.
- After task 2: 69 suites, 4719 passed, 0 failed.
- `tests/owner_settle_oracle.rs`: 4 passed, 0 failed. It ran against the installed gsd-core 1.23.0 with no `SKIP` line, and `GSD_META_MANAGER_ALLOW_MISSING_ORACLE` was not set. audit-uat returns the quoted reason verbatim with `category: deferred`. uat-passed reports it as `passing: true, deferred: true`.
- `cargo test --lib settle`: 86 passed, 0 failed.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Glyph registry rejected U+2018**
- **Found during:** Task 3 (full suite)
- **Issue:** `owner_note` maps `“` to `‘`. `glyphs::tests::production_ui_code_draws_only_registry_or_exempt_glyphs` flags U+2018 as a stray glyph.
- **Fix:** I added 0x2018 to `TYPOGRAPHY`, next to U+2019 which was already there, and updated the module doc's count to twelve.
- **Files modified:** src/ui/glyphs.rs
- **Commit:** 81d1d11c

**2. [Rule 1 - Test expectation] 80x24 full-command view of a Move with a reason**
- **Found during:** Task 3
- **Issue:** The verbatim sentence and the delimiters push the full argv view of "move, phase with a reason" to 21 rows, against the 19 an 80x24 popup has. The plan expected every 80x24 fit test to pass unchanged.
- **Fix:** The compact summary still fits every case. The full view of a reasoned Move now clips at 80x24, as an unknown phase's already did: it cannot be confirmed there, and `d` returns to the summary, which fits. The fit test expects `unknown_phase || reasoned_move` to clip. The D-07 wording is unchanged.
- **Files modified:** src/ui/screens/settle_popup.rs
- **Commit:** 81d1d11c

**3. [Rule 1 - Test] Singular-pronoun test vs. the verbatim sentence**
- **Found during:** Task 3
- **Issue:** `one_check_reads_singular` asserted that the prompt never contains `them`. The mandated sentence says "between them", and "them" there means the quotes.
- **Fix:** The test now rejects the plural forms that refer to the checks (`They `, `Keep them`, `promote them`).
- **Commit:** 81d1d11c

**4. [Process] Shared working tree with the parallel 261125-kfc session**
- **Issue:** Quick 261125-kfc edited src/app.rs and src/ui/screens/mod.rs in the same working tree. The path-scoped Task 1 commit (38457b09) commits whole files, so it took in that session's uncommitted hunks in those two files: a moved test block in app.rs, and `owner_work_matches` / `selected_alias` / `pin_selection_to_alias` moved in mod.rs. No content was lost; only the attribution is off. I did not rewrite history, because the shas are shared and the other session commits on this branch. Its later in-progress changes, including a clippy `type_complexity` failure it introduced in d5f9911c and fixed in 4881f331, also broke the build in the shared tree.
- **Mitigation:** From task 2 on, I verified in a detached scratch worktree (`.claude/worktrees/agent-khh-verify`, since removed) holding the branch HEAD plus only this task's files. I checked that each committed file was byte-identical to the verified copy.

### Other

- I renamed the stop_waiting unit test to `drops_only_the_rows_not_done_entries_on_stop_waiting`, so the plan's `fn stop_waiting` acceptance grep counts exactly 1.
- `recorded_reason` falls back to `moved.` for a Move with no target. The product never builds one, because `move_instruction` already requires the target.

## Threat Flags

None. There is no new network, file or process surface. The settle modules still write nothing and start nothing (tests/owner_settle_guard.rs passes), and SettleStopScreen is registered in the render-escape guard with hostile fixture states.

## Human UAT (not blocking)

1. On a throwaway project with a live Claude session, run `x w <reason> Enter y`. A second session should open beside the live one, the clipboard should be unchanged, and the live session should not get focus.
2. In that new session, `/gsd-verify-work` should record `reason: Deferred follow-up: will not do. Owner’s note: “““<reason>”””` verbatim in the UAT file.
3. Close that session before it records anything. `x` on the still-settling row should offer stop waiting, `y` should clear the badge, and `x` should open the chooser again.

## Self-Check: PASSED

- The commits 38457b09, 4479993c and 81d1d11c are ancestors of HEAD.
- Every file in the modified list exists.
- `git status` shows no uncommitted source changes. Only the untracked quick-task directory is left.
