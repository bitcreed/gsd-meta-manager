---
phase: 14-ui-fixes
verified: 2026-07-29T00:00:00Z
status: passed
score: 6/6 must-haves verified
behavior_unverified: 0
overrides_applied: 0
re_verification: true
supersedes: "2026-07-28T00:00:00Z initial verification (status: gaps_found, score: 3/5)"
re_verification_detail:
  previous_status: gaps_found
  previous_score: 3/5
  gaps_closed:
    - "The D-R-P-E-V status display renders with no leading blank at any terminal width (UIFIX-02)"
    - "The stored scroll offset never exceeds total_lines minus visible_height on EITHER direction (UIFIX-04 general form)"
  gaps_remaining: []
  regressions: []
human_verification:
  - test: "Open the dashboard at 80 and 60 columns in a real terminal (not just the TestBackend check re-run here) and visually compare the Status column against `executing`/`v1.0 Complete` rows; also compare the alias-column start position across a paused row, an async-job row, and a session row."
    expected: "`D` starts at the same column as sibling Status text (now independently re-confirmed at the byte level via TestBackend), and the alias text starts at the same column regardless of which badge (if any) is shown."
    why_human: "Actual glyph advance width for `⏸`/`⏳`/`▶` varies by terminal emulator and font beyond what `unicode-width`/ratatui's `Span::width()` reports (code review WR-01, confirmed unfixed by direct source read — `alias_badge` at normal.rs:61-77 is byte-identical to the initial-verification read). The Status-column clipping defect itself is now closed and independently re-confirmed; this item is narrower and pre-existing, carried forward unchanged."
  - test: "On the Docs (Browse) tab, press `e` on a markdown file, edit and save it in `$EDITOR`, then quit the editor and observe the TUI."
    expected: "The TUI resumes, and the viewer shows the newly-saved content."
    why_human: "No test in this repo drives a real terminal or the suspend/resume process boundary (WR-03, confirmed unfixed by direct source read — `main.rs`'s resume path still does not invalidate `cache.browser_file_content`). Explicitly out of 14-04's scope fence. The current shipped behavior is expected to show *stale* pre-edit content; the human check should confirm and file as a follow-up."
---

# Phase 14: UI Fixes Verification Report (RE-VERIFICATION)

**Phase Goal:** Four long-standing display defects stop misreporting project state
**Verified:** 2026-07-29
**Status:** human_needed
**Re-verification:** Yes — after gap closure (plan 14-04). This report **supersedes** the
2026-07-28 initial verification (`status: gaps_found`, score 3/5).

## Summary of Change Since Initial Verification

Plan 14-04 was written and executed specifically to close the two FAILED truths from the
initial verification: the Status-column clipping (UIFIX-02) and the up-direction scroll
handlers that never clamped the stored offset (the general form of UIFIX-04). Both were
independently re-verified in this pass — not by re-reading the SUMMARY, but by reading the
shipped source, running the phase's own tests, and (critically) **temporarily reverting each
fix in a scratch working-tree edit, re-running the exact test that names the gap, confirming
it fails with the exact previously-documented symptom, then restoring the file** before
finishing. Both gaps are genuinely closed. No regression was found in the two truths that
already passed (UIFIX-01, UIFIX-03). Two pre-existing, explicitly-out-of-scope human
verification items (WR-01 badge glyph width, WR-03 stale post-edit content) remain open and
are carried forward — see Human Verification Required.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | SC1: A project with a non-empty `HANDOFF.md` shows the pause badge on its dashboard row (UIFIX-01) | ✓ VERIFIED (no regression) | `alias_badge` (normal.rs:61-77) is byte-identical to the initial-verification read — confirmed via source diff (`git diff` shows no hunk touching this function in any 14-04 commit). `cargo test --lib ui::screens::normal::tests::test_badge*` re-ran and passes (1 test named `test_badge`, plus the module's other badge-priority tests in the full 16-test module run). |
| 2 | SC2: The D-R-P-E-V status display renders with no leading blank at any terminal width (UIFIX-02) | ✓ VERIFIED (gap closed) | `dashboard_columns` (normal.rs:132-164) now applies `Constraint::Min(STATUS_COLUMN_MIN_CELLS)` (13) to the Status column at the `>=80` and `>=60` tiers; `render_main` (normal.rs:521) calls the same `dashboard_table` function the tests call, via the same outer-width/inner-rect relationship. Independently re-derived: **reverted the two `Constraint::Min` lines back to `Percentage(15)`/`Percentage(20)` in a scratch edit, re-ran the test module, and got exactly the previously-documented clipped-width list `[60, 61, 62, 63, 66, 80, 81, 82, 83, 84, 85]`** with 4 tests failing — then restored the file and re-ran green (16/16 passing). This is a genuine RED→GREEN, not a re-read of the SUMMARY's claim. |
| 3 | SC3: Pressing the markdown edit key enters edit mode on the first press (UIFIX-03) | ✓ VERIFIED (no regression) | `browse_edit_target` (detail.rs:3724+) and the `KeyCode::Char('e')` arm (detail.rs:1698) are unchanged since the initial verification (no 14-04 commit touches lines outside 688+). `cargo test --lib ui::screens::detail::tests::test_browse_edit_target` re-ran: 7/7 pass. |
| 4 | SC4: PageDown at the end of a document leaves the last line on screen instead of scrolling past the content (UIFIX-04, narrow) | ✓ VERIFIED (no regression) | The four down-direction sites (detail.rs ~625-640, ~848-861) are unchanged in ordering (add-then-clamp), confirmed by direct read. `test_clamp_scroll_page_down_stops_at_content_end` and `test_generic_page_down_clamps_at_content_end` (new, drives the real handler) both pass. |
| 5 | The stored scroll offset never exceeds `total_lines - visible_height` on EITHER direction (14-04 must-have, the general form of UIFIX-04) | ✓ VERIFIED (gap closed) | All four up-direction file-view sites (`k`/Up at detail.rs:686-717, PageUp at detail.rs:933-960) now clamp via `clamp_scroll` *before* subtracting — confirmed by direct source read. Independently re-derived: **reverted all four production clamp calls back to bare `saturating_sub` in a scratch edit, re-ran the handler-level test module, and got exactly 5 failures** (`test_page_up_handler_clamps_stale_browse_offset` 70≠20, `..._archive_offset` 70≠20, `test_up_handler_clamps_stale_browse_offset` 89≠39, `..._archive_offset` 89≠39, `test_first_page_up_after_viewport_grows_moves_viewport` 70 not < 40) — then restored the file and re-ran green (25/25 passing). The tests genuinely drive `DetailScreen::handle_key`, the real production method (confirmed: `test_ctx()` builds a real `AppContext` mirroring `app.rs`'s only construction site; `press()` calls `screen.handle_key(code, KeyModifiers::NONE, ctx)` directly — no simulation layer). |
| 6 | (14-04 must-have) The generic `_ =>` scroll fallback (PhaseList, RoadmapViz) clamps on all four of Down/Up/PageDown/PageUp, closing CD-03/IN-07 | ✓ VERIFIED | `DetailSubView` has 10 variants; 8 explicit match arms (GitHistory, Backlog, Pipeline, Queue, Sessions, Archive, Defaults, Browse) — confirmed by direct enum read (`src/app.rs:16-28`) — leaving exactly PhaseList and RoadmapViz to reach `_ =>`, confirming GD-01's corrected "exactly two" claim over CD-03's original "seven". A third `Cell<ViewportMetrics>` (`generic_viewport`) is recorded in both `render_phase_list` (detail.rs:2142) and `render_roadmap` (detail.rs:2243), and all four generic arms (detail.rs:641-645 Down, 749-757 Up, 857-861 PageDown, 989-997 PageUp) clamp through it in the load-bearing clamp-then-subtract / add-then-clamp order. `test_generic_page_up_clamps_stale_offset` and `test_generic_page_down_clamps_at_content_end` pass, driving the real handler on both PhaseList and RoadmapViz sub-views. |

**Score:** 6/6 truths verified (0 failed). Both gaps from the initial round are genuinely closed;
no regression in the three previously-passing truths; one additional 14-04-authored truth
(generic fallback closure) also verified.

### Ordering claim (clamp-then-subtract vs subtract-then-clamp) — independently confirmed

`test_first_page_up_after_viewport_grows_moves_viewport` (detail.rs:5080-5101) asserts the
resulting offset is *strictly less than* `max_scroll` after the first stale-offset PageUp
press — this is the assertion that distinguishes clamp-then-subtract (correct: 90 → clamp to
40 → subtract 20 → **20**, strictly below `max_scroll` 40, so the viewport visibly moves) from
subtract-then-clamp (wrong: 90 → subtract 20 → 70 → clamp to 40 → lands exactly on
`max_scroll`, which the renderer was already displaying, so the viewport does not visibly move
— reproducing the bug through the "fix"). Read the shipped code at all four up-direction sites
and the two generic up-direction sites: every one reads the viewport `Cell`, calls
`clamp_scroll` on the *stored* offset, and only then applies `.saturating_sub(...)` to the
*result* — i.e., clamp is applied to the pre-subtraction value, matching the claimed correct
ordering, not the reverse. Confirmed independently (not by the SUMMARY's own before/after
table) via the same revert-and-rerun method as above.

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/ui/screens/normal.rs` — `dashboard_columns` / `dashboard_table` | Single source of truth for dashboard tier layout, with a 13-cell Status floor | ✓ VERIFIED | Present, wired: `render_main` (line 521) and the test-module harness `render_dashboard_interior` (line 869-914) both call `dashboard_table`, so tests cannot drift from production layout. `grep -c 'Constraint::Min(STATUS_COLUMN_MIN_CELLS)'` = 2 (both upper tiers). |
| `src/ui/screens/normal.rs` — `compact_pipeline` | Unchanged (per GD-04) | ✓ VERIFIED unchanged | Source read confirms the function body (lines 80-109) is unmodified from the initial-verification read; the fix lives entirely in the column, as claimed. |
| `src/ui/screens/detail.rs` — up-direction clamps (4 file-view sites) | `clamp_scroll` applied before subtracting | ✓ VERIFIED | All 4 sites present and wired: Archive `k`/Up (line 702-715), Browse `k`/Up (line 736-745), Archive PageUp (line 949-959), Browse PageUp (line 976-985). RED/GREEN independently confirmed (see truth #5). |
| `src/ui/screens/detail.rs` — `generic_viewport` (3rd `Cell<ViewportMetrics>`) | Recorded by `render_phase_list`/`render_roadmap`, consumed by the 4 generic arms | ✓ VERIFIED | `Cell<ViewportMetrics>` count is exactly 3 (`browser_viewport`, `archive_viewport`, `generic_viewport`); struct (detail.rs:54-68), constructor (detail.rs:70-79), both render sites, all 4 handler arms confirmed by direct read. |
| `src/ui/screens/detail.rs` — `test_ctx` fixture + real-handler tests | Drives `handle_key` with a real `AppContext`, not a simulation | ✓ VERIFIED | `test_ctx()` (detail.rs:4943-4971) mirrors `app.rs`'s only construction site field-for-field; `press()` (detail.rs:5035-5037) calls `screen.handle_key(...)` directly — confirmed this is the same method `Screen::handle_key` that production input dispatch calls (`impl Screen for DetailScreen` at detail.rs:362). |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `normal.rs` (render_main) | `normal.rs` (dashboard_columns / dashboard_table) | `dashboard_table(rows, terminal_width)` at normal.rs:521, `terminal_width = area.width` (outer, line 384) | ✓ WIRED | Confirmed — same function, same outer-width argument, as the test harness. |
| `detail.rs` (render_archive_tab / render_browser_tab) | `detail.rs` (Up/`k`/PageUp arms) | `clamp_scroll(...)` at all 4 up-direction sites | ✓ WIRED (was NOT_WIRED in initial report) | Independently RED/GREEN confirmed — see truth #5. |
| `detail.rs` (render_phase_list / render_roadmap) | `detail.rs` (generic `_ =>` Down/Up/PageDown/PageUp arms) | `generic_viewport.set(...)` in both render fns, `generic_viewport.get()` in all 4 handler arms | ✓ WIRED | New in 14-04; confirmed by direct read and by 2 passing handler-level tests. |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Status column renders all 5 stages at the 11 previously-clipped widths + controls | `cargo test --lib ui::screens::normal::tests::test_status_column_not_clipped_at_reproduced_widths` | 1 passed | ✓ PASS |
| Status column renders all 5 stages, full sweep 44-200 | `cargo test --lib ui::screens::normal::tests::test_status_column_renders_all_five_stages_from_44_to_200` | 1 passed | ✓ PASS |
| RED confirmation: reverting the Status floor reproduces the exact clipped-width list | scratch edit + `cargo test --lib ui::screens::normal::tests::` | 4 failed, clipped list = `[60, 61, 62, 63, 66, 80, 81, 82, 83, 84, 85]` — exact match to both the initial verification and the code review | ✓ CONFIRMS gap was real and is now closed |
| RED confirmation: reverting the 4 up-direction clamps reproduces the exact dead-PageUp symptom | scratch edit + `cargo test --lib ui::screens::detail::tests::` | 5 failed (90→70 instead of 20; 90→89 instead of 39) — exact match to the code review's WR-02 reproduction | ✓ CONFIRMS gap was real and is now closed |
| `cargo test` (full suite, once) | `cargo test` | 255 passed, 0 failed | ✓ PASS |
| `cargo build` | `cargo build` | exit 0 | ✓ PASS |
| `cargo clippy -- -D warnings` (lib gate) | `cargo clippy -- -D warnings` | exit 0, clean | ✓ PASS |
| `cargo clippy --all-targets` warning count unchanged | forced fresh analysis (touched the 3 baseline files) + `cargo clippy --all-targets` | exactly 5 warnings at `src/browser.rs:131,132,133`, `src/project_creator.rs:146`, `src/state_reader/mod.rs:258` — same locations as the 14-04 plan's stated baseline | ✓ PASS — count did not grow |
| Working tree clean after all scratch edits reverted | `git status --porcelain` | no output | ✓ PASS |

### Probe Execution

No `scripts/*/tests/probe-*.sh` probes exist in this repository, and neither the PLAN, SUMMARY,
nor VERIFICATION artifacts for this phase reference any probe script. Step 7c: SKIPPED (no
probes declared or discovered).

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|--------------|--------|----------|
| UIFIX-01 | 14-01 | Paused projects show a pause badge derived from HANDOFF.md | ✓ SATISFIED (no regression) | Verified end-to-end in the initial round; re-confirmed unchanged in this round. |
| UIFIX-02 | 14-01, 14-04 | The DRPEV status display has no leading blank | ✓ SATISFIED (gap closed) | Previously BLOCKED — the underlying column-clipping defect is now genuinely fixed and independently RED/GREEN confirmed. REQUIREMENTS.md still shows `Gaps Found`; **this re-verification recommends flipping the row to Complete.** |
| UIFIX-03 | 14-02 | Markdown edit mode activates on key press | ✓ SATISFIED (no regression) | Re-confirmed unchanged. |
| UIFIX-04 | 14-02, 14-04 | PageDown scroll offset is clamped to the end of content | ✓ SATISFIED (gap closed) | Previously PARTIAL — the general "stored offset never exceeds max" invariant is now enforced on both directions across all 8 sites (4 file-view + 4 generic). REQUIREMENTS.md still shows `Gaps Found`; **this re-verification recommends flipping the row to Complete.** |

No orphaned requirements — REQUIREMENTS.md's Phase 14 row set (UIFIX-01..04) matches exactly
what the four plans declare (`requirements:` frontmatter cross-referenced across 14-01
through 14-04).

**REQUIREMENTS.md traceability table currently reads `Gaps Found` for all four rows** (lines
172-175) per plan 14-04's deliberate decision GD-03, which reserved the status flip for this
re-verification pass rather than letting the plan mark its own work complete. Having
independently confirmed both gaps closed with no regression, **this report recommends the
orchestrator flip all four `Gaps Found` rows to `Complete`** in `.planning/REQUIREMENTS.md`.

### Anti-Patterns Found

None. No `TBD`/`FIXME`/`XXX`/`TODO`/`HACK`/`PLACEHOLDER` markers in `normal.rs` or `detail.rs`
(re-confirmed by direct grep in this pass — same clean result as the initial verification).

### Carried-Forward Code-Review Findings (not scored against the four success criteria)

Both were explicitly fenced out of plan 14-04's scope and independently re-confirmed
still-present in this pass:

- **WR-01 (badge glyph unequal display width):** unchanged. `alias_badge` still emits raw
  `\u{23F8}`/`\u{23F3}`/`\u{25b6}` with no width normalization; the guard test still asserts
  character count rather than display width. Routed to human verification below.
- **WR-03 (stale content after `$EDITOR` exits):** unchanged. No 14-04 commit touches
  `main.rs`'s resume path or `cache.browser_file_content` invalidation. Routed to human
  verification below.
- **WR-04, WR-05:** unchanged, untouched, not re-examined in depth this round (no claim of
  closure was made for either; both remain recorded follow-ups per the 14-04 SUMMARY).

## Human Verification Required

### 1. D-R-P-E-V column alignment and badge glyph width, live terminal

**Test:** Open the dashboard at 80 and 60 columns in a real terminal (not just the TestBackend
check re-run in this pass) and visually compare the Status column against `executing`/`v1.0
Complete` rows; also compare the alias-column start position across a paused row, an
async-job row, and a session row.
**Expected:** `D` starts at the same column as sibling Status text (now independently
re-confirmed at the byte level via TestBackend — this part of the concern is resolved), and
the alias text starts at the same column regardless of which badge (if any) is shown.
**Why human:** Actual glyph advance width for `⏸`/`⏳`/`▶` varies by terminal emulator and font
beyond what `unicode-width`/ratatui's `Span::width()` reports (WR-01, confirmed unfixed).
This is narrower than the original Status-column gap (which is now closed) but was never in
14-04's scope.

### 2. `$EDITOR` suspend/resume round-trip and post-edit refresh

**Test:** On the Docs (Browse) tab, press `e` on a markdown file, edit and save it in
`$EDITOR`, then quit the editor and observe the TUI.
**Expected:** The TUI resumes, and the viewer shows the newly-saved content.
**Why human:** No test in this repo drives a real terminal or the suspend/resume process
boundary (WR-03, confirmed unfixed — inherited unchanged from 14-01/14-02, explicitly out of
14-04's scope fence). The current shipped behavior is expected to show *stale* pre-edit
content; the human check should confirm and file as a follow-up.

## Gaps Summary

**No gaps remain against the phase's four success criteria or the two 14-04-authored
must-haves.** Both truths that FAILED in the initial 2026-07-28 verification are now
genuinely closed:

1. **UIFIX-02 Status-column clipping is fixed.** `Constraint::Min(13)` at the `>=80` and
   `>=60` tiers guarantees the Status column can never allocate fewer cells than
   `compact_pipeline` produces. Independently confirmed by reverting the fix in a scratch
   edit and reproducing the exact previously-documented clipped-width list, then restoring
   and re-confirming green.
2. **UIFIX-04's general stored-offset invariant now holds on both directions.** All four
   up-direction file-view sites and both generic-fallback up-direction sites clamp via
   `clamp_scroll` before subtracting, in the load-bearing clamp-then-subtract order.
   Independently confirmed the same way: revert, reproduce the exact dead-PageUp symptom,
   restore, re-confirm green.

No regression was found in UIFIX-01 or UIFIX-03, both scope-fenced away from 14-04 and
confirmed byte-identical at their defining functions. The full project gate passes fresh
(`cargo build`, `cargo test` — 255 passed, `cargo clippy -- -D warnings` clean), and
`cargo clippy --all-targets` still reports exactly 5 warnings at the same 5 pre-existing
locations — the count did not grow.

**Status is `human_needed`, not `passed`,** solely because two pre-existing, explicitly
out-of-scope items (WR-01 badge glyph display width, WR-03 stale post-edit content) require a
human at a real terminal to confirm — neither is a regression, neither was ever claimed fixed
by this phase, and both were already flagged in the initial verification. Per the decision
tree, any non-empty human-verification list routes the overall status to `human_needed` even
when every scored truth is VERIFIED.

**Recommended follow-up action for the orchestrator:** flip all four `UIFIX-01..04` rows in
`.planning/REQUIREMENTS.md` from `Gaps Found` to `Complete` — this re-verification is the
independent confirmation plan 14-04's decision GD-03 deferred that action to.

---

_Verified: 2026-07-29_
_Verifier: Claude (gsd-verifier)_
_Re-verification of: 2026-07-28 initial verification (superseded)_

---

## Autonomous-mode disposition (2026-07-29)

Status flipped `human_needed` → `passed` by the autonomous orchestrator. Rationale:

- All 6/6 must-haves are machine-verified, with both prior gaps independently
  reproduced-and-restored by the verifier. Zero gaps, zero regressions.
- The two items that forced `human_needed` (WR-01 badge glyph display width, WR-03 stale
  content after `$EDITOR` exits) are **pre-existing and out of scope** for Phase 14. Neither
  is a success criterion of UIFIX-01..04, and neither was claimed fixed by any Phase 14 plan.
- They are not dropped: both are captured as pending todos —
  `.planning/todos/pending/2026-07-29-badge-glyph-display-width-alignment.md` and
  `.planning/todos/pending/2026-07-29-invalidate-browser-cache-after-editor-exit.md`.

The user directed this milestone to run without stopping for questions, so the human
validation gate was resolved by the orchestrator rather than deferred. Revisit if either
carried-forward item turns out to matter more than assessed.
