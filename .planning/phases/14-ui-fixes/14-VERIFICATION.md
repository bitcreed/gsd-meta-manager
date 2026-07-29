---
phase: 14-ui-fixes
verified: 2026-07-28T00:00:00Z
status: gaps_found
score: 3/5 must-haves verified
behavior_unverified: 0
overrides_applied: 0
gaps:
  - truth: "The D-R-P-E-V status display renders with no leading blank at any terminal width (ROADMAP SC2 / UIFIX-02)"
    status: failed
    reason: >
      Independently reproduced with a ratatui TestBackend render harness using the exact
      production Constraint vectors from src/ui/screens/normal.rs (Percentage(15) at the
      >=80 tier, Percentage(20) at the >=60 tier). At terminal widths 60, 61, 62, 63, 66,
      80, 81, 82, 83, and 85 — including the default 80-column terminal — the Status
      column resolves to roughly 11-12 cells, one short of the 13-cell contract
      `compact_pipeline` produces, so the trailing "V" (verified) stage is clipped to
      "D  R  P  E ". A fully-verified project and a project still mid-pipeline (e.g.
      "executing") render identically in the Status column at these widths — this is
      exactly the "misreporting project state" defect class the phase exists to
      eliminate, not a cosmetic nit. This independently confirms code review finding
      CR-01. The phase's own tests (`test_compact_pipeline_*`) only assert the isolated
      `Line` object carries 13 cells of span content; none of them render that `Line`
      inside the real `Table`/`Constraint::Percentage` column, so this class of bug was
      structurally invisible to the phase's test suite. The literal "no leading blank"
      wording is technically true (no test width shows a leading space before "D"), but
      the plan's own must-have and the UI-SPEC's UIFIX-02 contract both go further,
      requiring the cell to render "identical at >=80, >=60, and <60" — which is false.
    artifacts:
      - path: "src/ui/screens/normal.rs"
        issue: "Status column constraints (Constraint::Percentage(15) at the >=80 tier, ~line 319; Constraint::Percentage(20) at the >=60 tier, ~line 330) do not guarantee the 13-cell floor compact_pipeline (lines 80-109) requires."
    missing:
      - "A Constraint::Min(13) (or equivalent hard floor) on the Status column at both the >=80 and >=60 tiers so the column never allocates fewer than 13 cells, as the code reviewer's CR-01 fix proposes."
      - "A render-level regression test (ratatui TestBackend) asserting the literal substring \"D  R  P  E  V\" is present in the rendered buffer row at widths 60, 80, and 120 — a unit test on compact_pipeline in isolation cannot catch a column-width clipping defect."
  - truth: "The stored scroll offset never exceeds total_lines minus visible_height (14-02-PLAN.md must_haves truth, part of UIFIX-04)"
    status: failed
    reason: >
      Confirmed by direct source read: the four *up*-direction handler sites —
      src/ui/screens/detail.rs:691-693 (Archive FileView, `k`/Up), :716-718 (Browse View,
      `k`/Up), :912-914 (Archive FileView, PageUp), :932-936 (Browse View, PageUp) — all
      perform a bare `saturating_sub` on the cached offset with no call to `clamp_scroll`
      and no read of the recorded `ViewportMetrics`, unlike their four sibling
      down-direction sites (:583-587, :627-631, :816-820, :848-852), which are correctly
      patched. Whenever the recorded viewport metrics shrink between key presses — the
      ordinary case of maximizing/resizing a terminal while scrolled near the end of a
      long document — the stored offset can remain above the new max_scroll after a
      PageUp press, because subtracting PAGE_SCROLL_LINES from an already-too-high stored
      value does not necessarily bring it back into range in one step. This reproduces
      the exact "dead PageUp press" symptom the UI-SPEC's own UIFIX-04 contract table
      requires to be eliminated ("PageUp after that: the first PageUp press visibly
      scrolls. No dead presses."). This independently confirms code review finding
      WR-02. The only test that names this contract row,
      `test_clamp_scroll_first_page_up_moves_viewport` (detail.rs:4852-4864), is a pure
      arithmetic simulation that calls `clamp_scroll` directly and manually computes
      `offset.saturating_sub(PAGE_SCROLL_LINES)` as a stand-in for PageUp — it never
      calls into the real `KeyCode::PageUp` handler, so it passes without exercising the
      actual defect. The narrower ROADMAP wording ("PageDown at the end of a document
      leaves the last line on screen") does hold — all four PageDown/Down sites are
      genuinely and correctly clamped, confirmed by source and by the passing
      `test_clamp_scroll_page_down_stops_at_content_end` / idempotence tests — but the
      broader invariant this plan itself authored as a must-have does not.
    artifacts:
      - path: "src/ui/screens/detail.rs"
        issue: "Up/PageUp handlers for Archive::FileView and Browse::View (lines 691-693, 716-718, 912-914, 932-936) never clamp the stored offset against ViewportMetrics."
    missing:
      - "Apply clamp_scroll at all four up-direction sites exactly as it is already applied at the four down-direction sites (per code review WR-02's fix suggestion)."
      - "A test that exercises the actual KeyCode::Up/PageUp handler (not just the clamp_scroll function) with a stale/over-large stored offset and asserts the viewport moves on the first PageUp press."
---

# Phase 14: UI Fixes Verification Report

**Phase Goal:** Four long-standing display defects stop misreporting project state
**Verified:** 2026-07-28
**Status:** gaps_found
**Re-verification:** No — initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | SC1: A project with a non-empty `HANDOFF.md` shows the pause badge on its dashboard row (UIFIX-01) | ✓ VERIFIED | `detect_handoff` (src/state_reader/mod.rs:57-99) and `alias_badge` (src/ui/screens/normal.rs:61-77) read and confirmed to match the described three-tier priority (pause > async-job > session). 9 tests in `normal.rs` + 5 in `state_reader/mod.rs` re-ran and pass (`cargo test --lib`, 204 passed). Wiring from `ProjectState.paused` (normal.rs:440-444) to `alias_badge` (normal.rs:452-459) confirmed by direct source read. |
| 2 | SC2: The D-R-P-E-V status display renders with no leading blank at any terminal width (UIFIX-02) | ✗ FAILED | `compact_pipeline` (normal.rs:80-109) correctly produces a leading-blank-free 13-cell `Line` in isolation — but independently reproduced with a TestBackend render of the real `Table`/`Constraint::Percentage` column that the cell is placed inside: the trailing "V" is clipped at widths 60-63, 66, and 80-85 (including the default 80-column terminal). See gaps. |
| 3 | SC3: Pressing the markdown edit key enters edit mode on the first press (UIFIX-03) | ✓ VERIFIED | `KeyCode::Char('e')` arm (detail.rs:1644) confirmed by direct source read: the `current_view == DetailSubView::Browse` block (detail.rs:1691-1701) resolves `browse_edit_target` and returns `ScreenAction::SuspendAndEdit(path)` synchronously on the first press, ahead of the generic Enqueue fall-through (detail.rs:1703+). 8 `test_browse_edit_target_*`/footer tests re-ran and pass. |
| 4 | SC4: PageDown at the end of a document leaves the last line on screen instead of scrolling past the content (UIFIX-04) | ✓ VERIFIED (narrow) | All four PageDown/Down handler sites (detail.rs:583-587, 627-631, 816-820, 848-852) confirmed by source read to assign through `clamp_scroll` against recorded `ViewportMetrics`. 5 `test_clamp_scroll_*` tests re-ran and pass, including idempotence on repeated PageDown. |
| 5 | The stored scroll offset never exceeds `total_lines - visible_height` (14-02-PLAN.md must-have, the general form of UIFIX-04) | ✗ FAILED | The four *up*-direction sibling sites (detail.rs:691-693, 716-718, 912-914, 932-936) do not call `clamp_scroll` at all — confirmed by direct source read. See gaps. The one test naming this property tests `clamp_scroll` in isolation, not the real `PageUp` handler. |

**Score:** 3/5 truths verified (2 failed)

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/ui/screens/normal.rs` — `alias_badge` | Pure 3-tier badge priority fn | ✓ VERIFIED | Present, matches documented glyph/color/priority, wired at 2 call sites (definition + row builder), 10 tests in module, all pass |
| `src/ui/screens/normal.rs` — `compact_pipeline` (rewritten) | Whitespace-free 13-cell Line | ⚠️ HOLLOW (partial) | The `Line` construction itself is correct (verified by test), but the artifact does not achieve its stated purpose end-to-end once placed in the real Status column — see SC2 gap |
| `src/state_reader/mod.rs` — HANDOFF regression tests | 5 new tests inside existing `mod tests` | ✓ VERIFIED | All 5 present and passing; wiring to `parse_project_state` confirmed |
| `src/ui/screens/detail.rs` — `browse_edit_target` | Pure path resolver, 2 guards | ✓ VERIFIED | Present, wired at the `e`-arm Browse branch, 7 tests pass. Root-fence guard (`if let Some(root) = ...`) fails open when `browser_root` is `None` (code review WR-04) — confirmed by source read, but current wiring always sets `browser_root` together with `browser_entries` (detail.rs:296-309), and `BrowserDepth::View` is only reachable by selecting a populated entry, so this fail-open pattern is not reachable through any live code path today. Flagged as a latent defense-in-depth weakness, not a live gap — see Human Verification. |
| `src/ui/screens/detail.rs` — `clamp_scroll` / `ViewportMetrics` | Renderer's own bound, reused by handler | ⚠️ HOLLOW (partial) | Present and correctly wired at the 4 down-direction sites; absent at the 4 up-direction sibling sites — see gap #2 |

### Key Link Verification

| From | To | Via | Status | Details |
|------|-----|-----|--------|---------|
| `src/state_reader/mod.rs` (parse_project_state) | `src/ui/screens/normal.rs` (alias_cell) | `state.paused` read at normal.rs:440-444 | ✓ WIRED | Confirmed |
| `normal.rs` (alias_cell construction) | `normal.rs` (alias_badge) | `alias_badge(...)` call at normal.rs:453 | ✓ WIRED | Confirmed |
| `detail.rs` (`KeyCode::Char('e')` Browse arm) | `detail.rs` (browse_edit_target) | `browse_edit_target(cache)` call at detail.rs:1694 | ✓ WIRED | Confirmed |
| `detail.rs` (render_archive_tab / render_browser_tab) | `detail.rs` (PageDown/Down arms) | `clamp_scroll(...)` at 4 down-direction sites | ✓ WIRED | Confirmed |
| `detail.rs` (render_archive_tab / render_browser_tab) | `detail.rs` (PageUp/Up arms) | expected `clamp_scroll(...)`, NOT PRESENT | ✗ NOT_WIRED | The up-direction sibling sites never read `ViewportMetrics` or call `clamp_scroll` — see gap #2 |
| `normal.rs` (compact_pipeline) | Status column `Table` cell | expected: column width >= 13 cells at all declared tiers | ✗ NOT_WIRED (defect) | `Constraint::Percentage` at the >=80 and >=60 tiers does not guarantee a 13-cell floor; the value the `Line` carries is silently truncated by the surrounding layout — see gap #1 |

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| D-R-P-E-V column renders all 5 stages at common terminal widths | Standalone `ratatui::backend::TestBackend` render of the exact production `Table`/`Constraint` setup at widths 59-100 (temporary scratch test, removed after use; not committed) | "D  R  P  E  V" present at 59, 70, 79, 86, 100; **absent** (V clipped) at 60, 63, 66, 80, 85 | ✗ FAIL — confirms gap #1 |
| `cargo test --lib` | `cargo test --lib` | 204 passed, 0 failed | ✓ PASS |
| `cargo build` / `cargo clippy -- -D warnings` | per CLAUDE.md release gate | exit 0 both | ✓ PASS |
| All 27 phase-added tests exist | `cargo test --lib -- --list \| grep -E "handoff\|compact_pipeline\|browse_edit_target\|clamp_scroll\|badge"` | all 27 named tests present | ✓ PASS |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|------------|--------------|--------|----------|
| UIFIX-01 | 14-01 | Paused projects show a pause badge derived from HANDOFF.md | ✓ SATISFIED | Verified end-to-end; matches shipped Phase 11 behavior plus extracted `alias_badge` |
| UIFIX-02 | 14-01 | The DRPEV status display has no leading blank | ✗ BLOCKED | REQUIREMENTS.md marks this `Complete`, but the underlying defect class (column clipping causing misreported status) is not fixed — see gap #1. The "no leading blank" clause narrowly holds; the phase's own broader contract ("identical at all three tiers") does not. |
| UIFIX-03 | 14-02 | Markdown edit mode activates on key press | ✓ SATISFIED | Verified: `e` returns `SuspendAndEdit` on the first press for the Browse tab |
| UIFIX-04 | 14-02 | PageDown scroll offset is clamped to the end of content | ⚠️ PARTIAL | The literal PageDown behavior is fixed and verified. The general "stored offset never exceeds max" invariant this same plan authored as a must-have is violated on the up-direction paths — see gap #2. REQUIREMENTS.md marks this `Complete`; that should be revisited pending the fix. |

No orphaned requirements found — REQUIREMENTS.md's Phase 14 row set (UIFIX-01..04) matches exactly what the three plans declare.

### Anti-Patterns Found

None. No `TBD`/`FIXME`/`XXX`/`TODO`/`HACK`/`PLACEHOLDER` markers in any of the three touched files (`normal.rs`, `detail.rs`, `state_reader/mod.rs`). Confirmed by direct grep.

### Additional Code-Review Findings (not scored against the four success criteria, but worth carrying forward)

These were raised by the prior code review (`14-REVIEW.md`) and independently spot-checked here. They do not gate this phase's four named success criteria but are real, reproducible issues:

- **WR-01 (badge glyph unequal display width):** `⏳` (U+23F3, East_Asian_Width=Wide) occupies 3 terminal cells while `⏸`/`▶` occupy 2, so the alias column is not actually aligned across badge states as the `alias_badge` docstring claims. `test_badge_is_never_two_glyphs` asserts char count, not display width, so it does not catch this. Terminal-rendering dependent — routed to human verification below.
- **WR-03 (stale content after `$EDITOR` exits):** `cache.browser_file_content` / archive equivalent is never invalidated after `SuspendAndEdit` returns, so a user who edits and saves a file continues to see the pre-edit text in the viewer. This is outside the four stated success criteria (which only require entering edit mode on the first press, not post-edit refresh) but is a real, user-visible "misreporting" bug in spirit. Recommend filing as a follow-up.
- **WR-04 (root fence fails open when `browser_root` is `None`):** confirmed by source read (detail.rs:3688-3692) — the `if let Some(root) = ...` guard skips the check entirely when unset. Traced the only code path that populates `browser_entries` (detail.rs:296-309) and confirmed it always sets `browser_root` in the same block, and that `BrowserDepth::View` is only reachable by selecting a populated entry — so this fail-open pattern is not reachable today, but it is a fragile invariant a future change could silently disarm with a green test suite. Recommend the reviewer's fix (`.ok_or(NO_FILE)?` instead of `if let Some`) as defense-in-depth.
- **IN-07 (unbounded offset on non-file tabs):** deliberately deferred by this phase's own CD-03 decision, recorded honestly in both SUMMARYs. Not a gap — a documented, in-scope-limiting decision.

## Human Verification Required

### 1. D-R-P-E-V column alignment and badge glyph width, live terminal

**Test:** Open the dashboard at 80 and 60 columns in a real terminal (not just the automated TestBackend check already run here) and visually compare the Status column against `executing`/`v1.0 Complete` rows; also compare the alias-column start position across a paused row, an async-job row, and a session row.
**Expected:** `D` starts at the same column as sibling Status text, and the alias text starts at the same column regardless of which badge (if any) is shown.
**Why human:** Actual glyph advance width varies by terminal emulator and font beyond what `unicode-width` reports (code review WR-01); this is confirmed to fail structurally in the >=60/>=80 tiers by TestBackend (see gap #1) but the exact visual severity across terminals needs an eyeball.

### 2. `$EDITOR` suspend/resume round-trip and post-edit refresh

**Test:** On the Docs (Browse) tab, press `e` on a markdown file, edit and save it in `$EDITOR`, then quit the editor and observe the TUI.
**Expected:** The TUI resumes, and the viewer shows the newly-saved content.
**Why human:** No test in this repo drives a real terminal or the suspend/resume process boundary (inherited from 14-01 D6 / 14-02 D11). Also surfaces WR-03: the current shipped behavior is expected to show *stale* pre-edit content, which the human check should confirm and file as a follow-up.

## Gaps Summary

Two of the five must-have truths verified here are FAILED, both independently reproduced against the actual codebase rather than inferred from the SUMMARYs:

1. **UIFIX-02 (SC2) is not actually fixed at the column-width level.** The `compact_pipeline` construction-site fix is correct in isolation (13 leading-blank-free cells), but the surrounding dashboard `Table`'s `Constraint::Percentage` columns do not guarantee 13 cells at the `>=60` and `>=80` tiers, so the trailing `V` stage is silently clipped at widths 60-63, 66, and 80-85 — including the default 80-column terminal. This means a fully-verified project and a mid-pipeline project can render identically in the Status column, which is precisely the "misreporting project state" defect class the phase set out to eliminate. This is not a hypothetical — it was reproduced with a TestBackend render using the actual production `Constraint` vectors.

2. **UIFIX-04's general "stored offset never exceeds max" invariant does not hold.** All four PageDown/Down sites are correctly clamped and this narrowly satisfies the literal ROADMAP wording ("PageDown at the end... leaves the last line on screen"). But the four sibling up-direction sites (Up/`k`/PageUp on Archive FileView and Browse View) were never patched — confirmed directly in source — so the stored offset can remain out of range after a terminal resize, reproducing the exact "dead PageUp press" symptom UIFIX-04 was meant to eliminate. The one test that names this contract row tests the `clamp_scroll` function in isolation and never exercises the real key handler, so it passes without covering the actual defect.

Both gaps are narrowly scoped, single-file fixes (one `Constraint::Min` change plus a render-level test; four `clamp_scroll` call sites plus one handler-level test) consistent with the size of fixes already landed in this phase. REQUIREMENTS.md currently marks UIFIX-02 and UIFIX-04 `Complete`; that should be revisited once these are closed.

UIFIX-01 (SC1) and UIFIX-03 (SC3) are both genuinely and fully verified — the extracted `alias_badge`/`browse_edit_target` functions are correctly wired, tested, and match the documented contract with no gap found.

---

_Verified: 2026-07-28_
_Verifier: Claude (gsd-verifier)_
