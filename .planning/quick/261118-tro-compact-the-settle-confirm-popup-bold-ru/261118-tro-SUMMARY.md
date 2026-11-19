---
quick_id: 261118-tro
phase: quick-261118-tro
plan: 01
subsystem: ui/settle-popup
tags: [tui, settle, confirm-popup, security-display]
requires: [phase 39 settle popup, confirm_popup]
provides:
  - "settle popup summary view cut from settle_mux_argv, `d` full-command toggle"
  - "render_confirm_popup_hinted (optional leading hint fragment)"
affects: [src/ui/screens/confirm_popup.rs, src/ui/screens/settle_popup.rs, src/ui/screens/render_escape_guard.rs, README.md]
key-files:
  modified:
    - src/ui/screens/confirm_popup.rs
    - src/ui/screens/settle_popup.rs
    - src/ui/screens/render_escape_guard.rs
    - README.md
decisions: [D-01..D-09 all INFERRED, see below]
status: complete
commits: 4
plan_head_before: 3266fb5db6ee3047d440bf7bf66b95a63b60a830
plan_head_after: 376614f9eed9d8a061ac68b387563201720b4610
actuals:
  tokens: 13600   # chars/4 over `git diff 3266fb5d HEAD` (54446 chars)
  tasks: 3
  commits: 4
completed: 2026-11-19
---

# Quick 261118-tro: Compact the settle confirm popup

The settle popup (`x`) now opens on a summary cut from the exact argv Yes executes, and
`d` toggles the whole argv (the old Phase 39 D-10 content) and back.

## Commits

| Task | Sha | Message |
| ---- | --- | ------- |
| 1 | 59f813cd | feat(quick-261118-tro): settle popup summary with a d full-command view |
| 2 | 20069ba6 | feat(quick-261118-tro): cap and emphasize the settle instruction in the summary |
| 3 | 79b15d11 | test(quick-261118-tro): cover the settle full-command view in the render-escape guard |
| 3 | 376614f9 | docs(quick-261118-tro): describe the settle summary and its d full command |

## Before and after (80x24, real TestBackend render of the tracer plan, Won't do, reason `no device here`)

BEFORE (HEAD 3266fb5d, 17 body rows of 19):

```
┌ Settle via /gsd-verify-work ─────────────────────────────────────────┐
│Project    alder-api                                                  │
│Target     phase 01: tests 2, 3 in 01-UAT.md                          │
│Outcome    Won't do                                                   │
│                                                                      │
│env -u GSD_WORKSTREAM -u GSD_PROJECT -u GSD_SESSION_KEY -u            │
│  CODEX_THREAD_ID -u CLAUDE_SESSION_ID -u CLAUDE_CODE_SESSION_ID -u   │
│  CLAUDE_CODE_SSE_PORT -u OPENCODE_SESSION_ID -u GEMINI_SESSION_ID -u │
│  CURSOR_SESSION_ID -u WINDSURF_SESSION_ID -u TERM_SESSION_ID -u      │
│  WT_SESSION -u TMUX_PANE -u ZELLIJ_SESSION_NAME claude "/            │
│  gsd-verify-work 01 The user will not perform tests 2, 3 in          │
│  01-UAT.md. Resume the UAT session of phase 01 without restarting it │
│  and record each of them as result: skipped, reason: Deferred        │
│  follow-up: will not do. no device here. They must not block phase 01│
│  or be asked again. Keep them in the UAT file and do not promote them│
│  to the 999.x backlog."                                              │
│                                                                      │
│Yes starts a new session; GSD records it. This app writes nothing.    │
│                                                                      │
│                           [ Yes ]   [ No ]                           │
│              y/n · ←/→ move · Enter select · Esc cancel              │
└──────────────────────────────────────────────────────────────────────┘
```

AFTER, compact view (default; 11 body rows of 19; the command, `phase 01`, the target
text, `result: skipped` and the reason are BOLD on screen, the rest of the instruction dim):

```
┌ Settle via /gsd-verify-work ─────────────────────────────────────────┐
│Project    alder-api                                                  │
│Target     phase 01: tests 2, 3 in 01-UAT.md                          │
│Outcome    Won't do                                                   │
│Reason     no device here                                             │
│                                                                      │
│Runs       claude /gsd-verify-work 01 · env -u (15 session vars)      │
│           The user will not perform tests 2, 3 in 01-UAT.md. Resume  │
│           the UAT session of phase 01 without restarting it and      │
│           record each of them as result: skipped, reason: Deferred … │
│                                                                      │
│Yes starts a new session · GSD records it · this app writes nothing   │
│                                                                      │
│                           [ Yes ]   [ No ]                           │
│      d full command · y/n · ←/→ move · Enter select · Esc cancel     │
└──────────────────────────────────────────────────────────────────────┘
```

AFTER, `d` full view (17 body rows; the old body, with the shorter shared footer and the
`d summary` hint):

```
┌ Settle via /gsd-verify-work ─────────────────────────────────────────┐
│Project    alder-api                                                  │
│Target     phase 01: tests 2, 3 in 01-UAT.md                          │
│Outcome    Won't do                                                   │
│                                                                      │
│env -u GSD_WORKSTREAM -u GSD_PROJECT -u GSD_SESSION_KEY -u            │
│  CODEX_THREAD_ID -u CLAUDE_SESSION_ID -u CLAUDE_CODE_SESSION_ID -u   │
│  CLAUDE_CODE_SSE_PORT -u OPENCODE_SESSION_ID -u GEMINI_SESSION_ID -u │
│  CURSOR_SESSION_ID -u WINDSURF_SESSION_ID -u TERM_SESSION_ID -u      │
│  WT_SESSION -u TMUX_PANE -u ZELLIJ_SESSION_NAME claude "/            │
│  gsd-verify-work 01 The user will not perform tests 2, 3 in          │
│  01-UAT.md. Resume the UAT session of phase 01 without restarting it │
│  and record each of them as result: skipped, reason: Deferred        │
│  follow-up: will not do. no device here. They must not block phase 01│
│  or be asked again. Keep them in the UAT file and do not promote them│
│  to the 999.x backlog."                                              │
│                                                                      │
│Yes starts a new session · GSD records it · this app writes nothing   │
│                                                                      │
│                           [ Yes ]   [ No ]                           │
│        d summary · y/n · ←/→ move · Enter select · Esc cancel        │
└──────────────────────────────────────────────────────────────────────┘
```

## 80x24 row counts (body rows of 19)

Compact view: won't do with a reason 11, deferred permanently without reason 10, Move known
check 10, Move known phase 10, Move unknown check 12, Move phase with a reason 11, live
session 11. Every compact body draws exactly the rows it has lines (no `Paragraph` re-wrap)
and renders unclipped. Full view: phase-level Move 18 rows at 80x24 (fits 19); tracer and
Move check unclipped at 80x24. The full view of an unknown-phase Move needs 20 rows at 80x24
and clips there (CLIPPED_HINT, not confirmable, `d` goes back to the summary which fits);
that is the same row count the old body had, now covered by an explicit test assertion.

## Decisions (all INFERRED by the orchestrator from the owner's request)

- D-01: compact header: Project keeps the wrapping `field` row; Target, Outcome and (only when
  entered) Reason are one row each, `shown` first, then `fit_cells`.
- D-02: Runs row derived only from `settle_mux_argv(plan).as_slice()`: program words plus the
  slash command bold, dim ` · env -u (N session vars)`, N = count of `-u` elements; no
  variable name drawn in the compact view.
- D-03: instruction sliced from the argv's last element after `{command} `, dim, at most
  `INSTRUCTION_ROWS = 3` rows with a dim ellipsis; key facts bold by substring ranges; a hit
  followed by an ASCII letter/digit is dropped (`phase 1` never bolds the head of `phase 10`).
- D-04: `d` (no CONTROL/ALT) toggles the full-command view; `render_confirm_popup_hinted`
  carries the leading `d full command` / `d summary` fragment; `render_confirm_popup`
  delegates with no fragment (every other popup byte-identical).
- D-05: one short dim footer for both views, wrapped with `wrap_at_separators`.
- D-06: the unknown-phase warning stays Yellow with its full text in both views.
- D-07: the full view does not scroll; cut-off views are not confirmable (either view).
- D-08: tests build their expectations from the same `settle_mux_argv` argv; full view
  asserts every element and the ordered join; guard fixture state covers the full view.
- D-09: README settle paragraph rewritten; help.rs and detail.rs untouched.

## Deviations from Plan

### Auto-fixed Issues

None.

### Notes

- **Fragment dropped on narrow popups.** At 60x20 the fragment does not fit beside the key
  hint, so the plain hint shows (specified behavior); `d` still works there.
- **Unknown-phase full view clips at 80x24.** See row counts. The plan's matrix wording
  implied unclipped compact only for this case; the test asserts clipped-in-full and
  unclipped-in-compact.
- **Test-first order.** The new tests were written alongside the implementation in the same
  working session rather than observed failing first (the confirm_popup function they call
  did not exist beforehand, so the RED state was a compile error). All tests pass.

## Verification

- `CARGO_TARGET_DIR=target-wt cargo test --no-fail-fast`: raw sum of the 69 `test result:`
  lines = 4575 passed, 1 failed, 19 ignored. The one failure is
  `owner_audit_conformance::ours_equals_the_oracle_on_every_owner_fixture`: the locally
  installed GSD oracle errors with `fixture debug-sessions: the oracle failed: Error: No
  phases directory found in planning directory`. It compares the audit digest against
  the machine's gsd-core and touches nothing this plan changed (no UI code); out of scope,
  not fixed. Lib tests: 3709 passed, 0 failed. The settle, confirm_popup,
  render_escape_guard and `owner_settle_guard` suites all pass.
- `cargo clippy --all-targets -- -D warnings`: clean. `cargo fmt --check`: clean.
- `git diff --name-only 3266fb5d HEAD`: confirm_popup.rs, render_escape_guard.rs,
  settle_popup.rs, README.md. detail.rs and help.rs untouched.

## Known Stubs

None.

## Threat Flags

None. No new endpoint, auth path or file access; only what the popup draws changed, and
every drawn value still goes through `shown` before it is cut or emphasized.

## Self-Check: PASSED

- Commits 59f813cd, 20069ba6, 79b15d11, 376614f9 are ancestors of HEAD.
- `render_confirm_popup_hinted`, `settle_full_body`, `INSTRUCTION_ROWS`, `showing_full_command`
  exist; README no longer contains "then shows the exact" and contains "full command".
