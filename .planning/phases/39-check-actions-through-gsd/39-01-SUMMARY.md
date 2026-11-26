---
phase: 39-check-actions-through-gsd
plan: 01
subsystem: owner-inbox-tui
tags: [rust, ratatui, inbox, settle, verify-work, confirm-popup]
requires:
  - phase: 37
    provides: RunCommand, launch seam (multiplexer::launch), ack popup, owner_actions helpers
  - phase: 38
    provides: triage entries (merged same-phase rows), inbox rows and cursor
provides:
  - "`x` on an inbox TEST or BLOCKED phase row: chooser, reason field, default-No popup, launch of /gsd-verify-work N <instruction> (OWN-29, OWN-30)"
  - owner_settle (typed target, sanitizer, instruction templates, recheck, launch seam) and settle_popup (three screens)
  - settle_mux_argv / settle_terminal_argv (CWE-88 pin test)
  - command_phase helper shared by TEST and BLOCKED rows
  - tracer flow test proving the fixture copy stays byte-identical (OWN-31, first half)
affects: [39-02, 39-03]
status: complete
commits: 2
plan_head_before: 136a3e22a8b514e7115c45efb433733357c73fa0
plan_head_after: 1fc1938148e0de77d93471d58c8c23fc74d355b4
actuals:
  tokens: 26000
  tasks: 2
  commits: 2
tech-stack:
  added: []
  patterns: [pure core module plus screens module, cfg(test) thread-local launch seam, tracer flow test over a TempDir fixture copy]
key-files:
  created:
    - src/ui/screens/owner_settle.rs
    - src/ui/screens/settle_popup.rs
  modified:
    - src/ui/screens/detail.rs
    - src/ui/screens/inbox.rs
    - src/ui/screens/mod.rs
    - src/ui/screens/ack_confirm.rs
    - src/ui/screens/render_escape_guard.rs
    - src/state_reader/owner_items/rules.rs
    - src/state_reader/owner_items/mod.rs
    - src/ui/glyphs.rs
    - tests/focus_entry_guard.rs
key-decisions:
  - "Module split: owner_settle.rs (pure core) and settle_popup.rs (screens)"
  - "Chooser is a one-row footer, the reason field a second screen (AnswerScreen style), the popup a third"
  - "The popup shows the whole argv once with the instruction inside the quoted prompt element; no duplicate instruction block"
  - "One cfg(test) thread-local launch seam in owner_settle.rs instead of a per-screen fn field"
  - "Every outcome asks GSD for result: skipped with a Deferred follow-up: reason (gsd-core's only non-blocking skip)"
requirements-completed: [OWN-29, OWN-30, OWN-31]
---

# Phase 39 Plan 01: Settle a phase row through GSD Summary

In the TUI inbox, `x` on a TEST or BLOCKED phase row opens a one-row chooser (`w` won't do, `p` defer
permanently), a reason field, and a default-No popup that draws the exact argv. Yes rechecks the row against the
current state and launches `/gsd-verify-work N <instruction>` (`--ws NAME` for a workstream) as ONE argv element
in the project through `multiplexer::launch`; a live session is focused instead. GSD records the outcome, the app
writes nothing.

## Tasks

1. Tracer: `owner_settle` core, chooser, reason field, confirm popup, settle argv, `x` arm, escape-guard
   fixtures, Won't do flow on TEST rows (commit dc162379)
2. Deferred permanently, BLOCKED rows, the D-03 refusal table, check targets, hostile text, `command_phase`,
   fold-row refusals (commit 1fc19381)

## Verification

- `rtk bypass cargo test --no-fail-fast`: 4517 passed, 0 failed, 19 ignored over every suite (lib 3660
  tests; one earlier run showed the time-dependent `driver_run` test
  `the_status_tokens_are_dropped_whole_benign_first_and_never_split` failing on a wall-clock heartbeat
  string, unrelated to this plan; it passed alone three times and in the rerun).
- `rtk bypass cargo clippy --all-targets -- -D warnings` clean; `cargo fmt` applied.
- `A` unchanged: all `ack_confirm` (10) and `owner_actions` tests pass untouched; no file under
  `tests/fixtures/` changed.
- Tracer (D-13b): `the_inbox_wont_do_flow_launches_the_exact_prompt_and_writes_nothing` runs
  `x w "no device here" Enter y` on a TempDir copy of `uat-pending`: the seam holds one plan with the expected
  prompt, cwd is the copy, the status is `Launched /gsd-verify-work 01 in alder-api: won't do for phase 01
  (2 checks)`, the row is still listed and the tree map is byte-identical.

## Measured popup size (80x24, body room 19 rows, inner width 70)

| Plan | Body rows |
|------|-----------|
| Won't do, reason `no device here`, `uat-pending` phase 01 | 17 |
| Deferred permanently, empty reason, same row | 17 |

Both are unclipped at 80x24; at 60x20 the popup clips and `y` launches nothing.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Glyph scan rejected the typographic quotes**
- **Found during:** full suite after Task 2
- **Issue:** `ui::glyphs::tests::production_ui_code_draws_only_registry_or_exempt_glyphs` flags U+2019, U+201C
  and U+201D spelled as `\u{..}` escapes in `owner_settle.rs`; the plan mandates those quotes (D-08, the
  ASCII-quote mapping of `clean`) but they were not in the scan's TYPOGRAPHY set.
- **Fix:** added `0x2019`, `0x201C`, `0x201D` to TYPOGRAPHY (8 to 11 code points) and updated the module doc.
- **Files modified:** src/ui/glyphs.rs
- **Commit:** 1fc19381

**2. [Rule 3 - Blocking] focus_entry_guard lists the files that call `focus_message`**
- **Found during:** full suite after Task 2
- **Issue:** `owner_settle::launch_settle_real` is a fourth caller of `crate::multiplexer::focus_message`
  (as the plan specifies); `tests/focus_entry_guard.rs` pins the caller list.
- **Fix:** added `src/ui/screens/owner_settle.rs` to `FOCUS_CALLERS` (the guard asks for exactly this edit).
- **Files modified:** tests/focus_entry_guard.rs
- **Commit:** 1fc19381

**3. [Rule 3 - Scope placement] `settle_recheck` and `resolve_draft` landed in Task 1**
- **Issue:** the plan adds them in Task 2, but `settle_open` (Task 1) needs the shared resolver and a
  `pub(crate)` `settle_recheck` left unused would fail `clippy -D warnings` at the Task 1 commit.
- **Fix:** both are in Task 1 and `SettleConfirmScreen::confirm` calls the recheck from the start; Task 2 added
  its tests (`yes_rechecks_*`) and the D-03 table.
- **Commit:** dc162379

**4. [Test placement] fixture helpers live in `owner_settle::test_support`**
- `temp_fixture`, `tree` and `fixture_ctx` are shared by the `owner_settle` and `settle_popup` tests, so they sit
  in a `#[cfg(test)] pub(crate) mod test_support` instead of the plan's single `settle_popup` tests module.
  `resolve_draft` is `pub(crate)` so the popup tests can build a plan.

## Inferred decisions

- [inferred, D-10] The popup shows the whole argv once (every `env -u` pair, `claude`, the double-quoted prompt);
  the instruction is read inside it. 17 of 19 rows at 80x24 for both tracer plans.
- [inferred, RESEARCH Pitfall 1] Every outcome records `result: skipped` with a `Deferred follow-up:` reason;
  whether the model running verify-work follows it is human UAT (RESEARCH A2).
- [inferred, D-08] Phase targets list numbers, not names; more than 20 checks in one file read
  `all N open tests in FILE`.
- [inferred] `clean` maps whitespace first (so a tab or newline separates words), drops other control and
  invisible characters, then collapses space and `-` runs; a cut keeps exactly `cap` characters plus `\u{2026}`.
- [inferred] The reason field accepts every `Char` key (as `AnswerScreen` does); `clean` runs at Enter.
- [inferred] A phase target with no listed checks answers `Row is no longer listed; nothing started`.
- [inferred] Yes-time refusals reuse the open-time text with `nothing launched` in place of `nothing started`.
- Edge probe OWN-29 (unclassified) stays flagged for human review as the plan notes; 39-02 and 39-03 cover keys
  and the remaining rows.

## Known Stubs

None.

## Threat Flags

None beyond the plan's threat model (T-39-01 to T-39-08 mitigated and tested; the source-scan guard for T-39-06
follows in 39-03).

## Self-Check: PASSED

`owner_settle.rs`, `settle_popup.rs` and this SUMMARY exist; commits dc162379 and 1fc19381 are ancestors of HEAD.
