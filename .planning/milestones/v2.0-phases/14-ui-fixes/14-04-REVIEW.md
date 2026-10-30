---
phase: 14-ui-fixes
plan: 04
reviewed: 2026-07-29T00:00:00Z
depth: deep
files_reviewed: 2
files_reviewed_list:
  - src/ui/screens/normal.rs
  - src/ui/screens/detail.rs
findings:
  critical: 0
  warning: 1
  info: 3
  total: 4
status: issues_found
advisory: true
---

# Phase 14 Plan 04: Gap-Closure Review Report (Advisory)

**Reviewed:** 2026-07-29
**Depth:** deep (cross-file, real-handler and real-render path tracing; empirical re-measurement)
**Files Reviewed:** 2 (`src/ui/screens/normal.rs`, `src/ui/screens/detail.rs`), diff `6592bbf..HEAD`,
commits `e27ab10`, `e1ac518`, `0e4bcdf`
**Status:** issues_found (advisory only — does not block the phase)

## Summary

This plan closes the two gaps `14-VERIFICATION.md` recorded as FAILED (UIFIX-02 status-column
clipping, UIFIX-04 up-direction scroll clamp) plus CD-03/IN-07 (the generic `_ =>` fallback). I
re-derived every load-bearing claim against the actual source rather than trusting the SUMMARY's
prose, specifically hunting for the same failure class that produced the prior round's false
green (tests that assert an isolated value instead of exercising the real render/handler path).

**I did not find that failure class recurring.** Concretely, and independently confirmed:

- `dashboard_table(rows, terminal_width)` is called from exactly two places in the file — the
  production `render_main` call site and the test harness `render_dashboard_interior` — so the
  render-level tests genuinely exercise the shipped `Table`/`Constraint` construction, not a
  parallel reimplementation. Verified by direct grep (`dashboard_table(` → 2 hits) and by reading
  both call sites side by side.
- The scroll tests call `screen.handle_key(code, KeyModifiers::NONE, ctx)` — the real `Screen`
  trait method — against a real `AppContext` built by a new `test_ctx()` fixture whose fields
  match the single production `AppContext` construction site in `app.rs` field-for-field. This is
  not a simulation of the key press; it is the key press.
- All four up-direction sites (Archive FileView and Browse View, both `k`/`Up` and `PageUp`) and
  all four generic-fallback sites (`Down`/`Up`/`PageDown`/`PageUp` on `PhaseList`/`RoadmapViz`)
  clamp through the unchanged `clamp_scroll` helper. Enumerated and confirmed by direct source
  read at `src/ui/screens/detail.rs:587-593, 631-637, 707-714, 738-744, 846-853, 878-884,
  949-957, 977-984` (down-direction, pre-existing/task-3) and `:643-649, 690-693 [archive up],
  715-718 [browse up], 891-897, 951-957, 979-984, 992-996` (up-direction and generic, this plan).
- The clamp-then-subtract vs subtract-then-clamp claim is correct, not just asserted: I hand-
  verified the arithmetic (`clamp_scroll(90,100,60)=40`, `40-20=20` vs `90-20=70` then
  `clamp_scroll(70,100,60)=40` — the latter lands exactly on the value the renderer already
  displays) and confirmed `cargo test --lib ui::screens::detail::tests::` passes clean (25/25) on
  a fresh build.
- `cargo build`, `cargo test` (255 passed), `cargo clippy -- -D warnings` (clean), and
  `cargo clippy --all-targets` (exactly 5 warnings, at the same 3 pre-existing locations in
  `src/browser.rs`, 1 in `src/project_creator.rs`, 1 in `src/state_reader/mod.rs`, none in either
  file this plan touches) all reproduce independently.
- `git diff` confirms `compact_pipeline`, `clamp_scroll`, `ViewportMetrics`, `PAGE_SCROLL_LINES`,
  `alias_badge`, `browse_edit_target`, and `footer_spans` are untouched by this diff.
- `cargo fmt --check` reproduces the SUMMARY's claimed baseline exactly: 96 pre-existing diff
  hunks in `detail.rs`, 1 in `normal.rs` — no new drift added by this plan.
- I independently re-measured the `Constraint::Min(13)` floor's effect on sibling columns with a
  standalone `TestBackend` harness (same constraint vectors, filler cells sized to overflow each
  column) at widths 60, 61, 66, 79, 80, 81, 85, 100. The floor costs the Phase/Alias columns at
  most 1-2 cells versus the pre-fix percentages — not the "starved to 2-3 characters" failure mode
  the plan worried about for the `<60` tier (which is correctly left untouched). No sibling column
  collapses.
- The accepted deviation (`grep -c 'dashboard_table(' >= 4` reading 2) — **I agree with the
  executor's judgment.** The acceptance criterion's literal intent (a single construction path
  shared by production and tests, so they cannot drift) is met: there are exactly two call sites
  and no parallel table-building code exists anywhere in the file.

One real gap survived this pass — see WR-01 below — plus three lower-severity completeness notes.

Note on method: my first `cargo test --lib ui::screens::detail::tests::` run in this session
returned 5 failures reproducing the *pre-fix* arithmetic exactly (e.g. offset 90 → 70 instead of
20). This was a stale incremental-build artifact in my own environment (an untouched `target/`
directory from before the merge commit), not a defect in the reviewed code: touching the file and
rebuilding made it pass consistently on repeated runs, and a full clean `cargo build && cargo
test` afterward was green throughout. Recorded here for transparency, not filed as a finding.

## Critical Issues

None found.

## Warnings

### WR-01: `render_roadmap`'s "no state" branch never records `generic_viewport` — an asymmetric half-wired clamp on one of the two sub-views this plan closes

**File:** `src/ui/screens/detail.rs:2157-2261` (compare with the correctly-unconditional sibling
at `:1985-2154`)

**Issue:** Task 3's own action text says the roadmap renderer should record `generic_viewport`
"the same" way the phase-list renderer does. It does not, in one respect. In
`render_phase_list`, the `self.generic_viewport.set(...)` call sits **after** the `if let
Some(state) = state { ... } else { ... }` block (line 2141), so it fires unconditionally — even
when `state` is `None` and the function falls into the "No state data available for this
project." branch, `content_height` is still computed from the (short) `lines` vector actually
built and recorded.

In `render_roadmap`, the equivalent `self.generic_viewport.set(...)` call sits **inside** the
`if let Some(state) = state { ... }` arm only (lines 2242-2246). The `else` arm (lines 2256-2260,
"No state data available for roadmap.") does not touch `generic_viewport` at all. If a project's
state was previously `Some` while the user was scrolled on the Roadmap tab (recording real,
possibly large, `total_lines`/`visible_height`), and the state then transitions to `None` (e.g.
the filesystem watcher reloads a project whose `STATE.md` briefly fails to parse) without the
user switching tabs, the next `PageUp`/`PageDown`/`Up`/`Down` press clamps `self.scroll_offset`
against the **stale** pre-transition metrics rather than against the empty content actually being
shown.

**Why this is real and not just theoretical:** it is exactly the "records but is never
refreshed on this path" pattern the plan itself was written to eliminate, and it is asymmetric
with the sibling function that got this right. It also has **zero test coverage** — both new
`test_generic_*` tests set `screen.generic_viewport` directly and never call `render_roadmap` or
`render_phase_list`, and `test_ctx()` starts `project_states` as an empty map, which is the exact
condition (`state: None`) that would exercise this branch, yet no test drives `render_roadmap`
with `ctx.project_states` empty to check what gets recorded.

**Why it is a WARNING and not a BLOCKER:** the practical blast radius is small. `switch_to_tab`
resets `self.scroll_offset = 0` on every tab entry, and `clamp_scroll(0, anything, anything)` is
always `0`, so the stale-metrics window only matters for a state transition that happens *without*
a tab switch while already mid-scroll — a narrow, transient condition. The `None` branch also
never applies `self.scroll_offset` to anything it renders (no `Paragraph::scroll` call in that
arm), so there is no visible corruption in that frame; the risk is confined to the stored offset
value itself until the next render self-heals it once state returns.

**Fix:** move the `self.generic_viewport.set(...)` call in `render_roadmap` to fire
unconditionally, mirroring `render_phase_list` exactly — e.g. compute `total_content = 0` and
`visible_height = area.height` in the `else` arm and record them there too, or hoist the
recording above the `if let Some(state) = state` branch using `area.height` for the "no state"
case. Add a test that constructs a `DetailScreen`/`AppContext` with `DetailSubView::RoadmapViz`
and an empty `project_states` map, calls `render_roadmap` against a `TestBackend`, and asserts
`generic_viewport.get()` is `(0, area.height)` rather than untouched.

## Info

### IN-01: No regression test protects the sibling dashboard columns from the new `Constraint::Min(13)` floor

**File:** `src/ui/screens/normal.rs:480-511` (`dashboard_columns`), test module ~846-1054

**Issue:** The plan's empirical grounding and the SUMMARY both assert "no sibling column
starved" as a fact about the shipped fix, but no committed test checks the Alias/Phase/Progress/
Backlog cell widths at all — every new test's row data (`pipeline_row()`) uses short fixed
strings ("proj", "14 ui-fixes") that fit comfortably regardless of column width, so a future
regression that meaningfully starves a sibling column would not be caught. I independently
verified with a standalone `TestBackend` harness (same constraint vectors, oversized filler
cells) that the current floor costs at most 1-2 cells versus the pre-fix percentages at the
tested widths (60, 61, 66, 79, 80, 81, 85, 100) — so the claim holds today — but this is
unverified by the test suite going forward.

**Fix:** add one test per upper tier that renders a row with a long alias/phase string and
asserts the rendered cell is not truncated below some minimum (e.g. `alias.len().min(N)`
characters survive), or at minimum assert the sum of the resolved column widths at a
representative width matches the pre-fix total to catch future overflow.

### IN-02: The "Loading window" sub-case of `test_page_up_at_zero_offset_is_inert` doesn't exercise any content-dependent code path

**File:** `src/ui/screens/detail.rs:5124-5133`

**Issue:** The final block of this test sets `cache.browser_file_content = None` after
`browse_fixture(100, 60, 0)` already set `browser_viewport` to `(100, 60)` and the stored offset
to `0`, then asserts `PageUp` leaves the offset at `0`. But `BrowserDepth::View`'s scroll handler
(`detail.rs:736-745`) never reads `cache.browser_file_content` — it operates purely on the stored
offset and the recorded `ViewportMetrics`, both already at their zero-equivalent state before the
key press. The assertion is true, but it would be equally true with the `browser_file_content =
None` line deleted; the test's docstring ("While the file content is still None (the Loading
window), Up and PageUp are inert") implies content-awareness that the production code doesn't
have and this test doesn't add.

**Fix:** either remove the misleading framing (this is just another instance of the
already-covered zero-offset floor), or, if the intent was genuinely to guard against a future
content-dependent branch being added incorrectly, seed a **non-zero** stored offset alongside
`browser_file_content = None` and assert the clamp still behaves — that would at least prove
something the zero-floor tests don't already cover.

### IN-03: `test_status_column_aligns_with_plain_status_text` only checks alignment at width 80, not width 60

**File:** `src/ui/screens/normal.rs:1002-1020`

**Issue:** Both the `>=80` and `>=60` tiers received the same `Constraint::Min(13)` treatment,
but the alignment assertion (pipeline cell and plain-text status cell starting at the same buffer
column) is only exercised at width 80. This is a minor completeness gap, not a demonstrated
defect — I confirmed by direct calculation that the `>=60` tier's column layout (`Percentage(30),
Percentage(35), Min(13), Percentage(15)`) places the Status column at the same offset regardless
of cell content, same as the `>=80` tier, so the property likely holds — but nothing pins it.

**Fix:** parametrize the existing test over `[60, 80]` rather than hardcoding `80`.

---

_Reviewed: 2026-07-29_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: deep_
_Scope: advisory review of plan 14-04 only (gap closure for CR-01/UIFIX-02 and WR-02/UIFIX-04,
plus CD-03/IN-07). Prior-round findings WR-01, WR-03, WR-04, and the 5 pre-existing
`cargo clippy --all-targets` warnings are explicitly out of scope and were not re-filed._
