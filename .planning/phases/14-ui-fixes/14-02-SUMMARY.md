---
phase: 14-ui-fixes
plan: 02
subsystem: ui
tags: [rust, ratatui, tui, key-routing, scroll, regression-test, interior-mutability]

# Dependency graph
requires:
  - phase: 14-01
    provides: the wave-1 base (alias_badge extraction + compact_pipeline pad-cell removal) this plan builds on without touching
  - phase: quick-260401-t7y
    provides: the ScreenAction::SuspendAndEdit -> $EDITOR shell-out machinery reused unchanged
  - phase: quick-260512-fe6
    provides: the Docs (Browse) tab whose `e` key had no branch
  - phase: 12-milestone-archive-browser
    provides: render_markdown_lines + the Archive `/milestones/` read-only guard copied verbatim
provides:
  - "browse_edit_target(&ProjectViewCache) -> Result<PathBuf, &'static str> — the whole UIFIX-03 contract as a pure, AppContext-free function"
  - "footer_spans(&DetailSubView) -> Vec<Span<'static>> — makes the footer hint set assertable (Paragraph has no text accessor)"
  - "clamp_scroll(offset, total_lines, visible_height) -> u16 — the renderer's own max-scroll formula, reusable by key handlers"
  - "ViewportMetrics + two Cell<ViewportMetrics> fields on DetailScreen — render-pass viewport recording under an &self render signature"
  - "The first key-routing and scroll-state regression tests in this repo (14 new tests in detail.rs)"
affects: [14-03, docs-browser-tab, archive-file-view, markdown-viewer-scroll]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "std::cell::Cell interior mutability to let an &self render pass record state for a &mut self key handler"
    - "Pure Result<T, &'static str> resolver returning the user-facing status message on the error path"
    - "Footer assertion via span-content concatenation (inherited from plan 14-01's Line-assertion pattern)"

key-files:
  created: []
  modified:
    - src/ui/screens/detail.rs

key-decisions:
  - "CD-01 taken: viewport metrics live in two Cell<ViewportMetrics> on DetailScreen, not on ProjectViewCache — Screen::render takes &self and &AppContext, so no render path can mutate the cache"
  - "CD-02 taken: no test_ctx() AppContext helper built; all 14 tests are pure-function assertions"
  - "CD-03 taken: the generic `_ =>` scroll fallback is deliberately left UNCLAMPED"
  - "CD-04 taken: the root fence reports the existing `Select a markdown file to edit` string rather than inventing a third guard message"
  - "Requirements UIFIX-03/UIFIX-04 deliberately NOT marked complete — plan 14-03 also declares them, so the shared-ID gate defers (same as 14-01 did for UIFIX-01/02)"
  - "Both RED commits wire production routing against a stub resolver so every commit in the series is clippy-clean and bisectable"

patterns-established:
  - "A key handler that needs render-time geometry reads it from a Cell recorded by the render pass — no Screen trait signature change"
  - "Guard predicates for $EDITOR launches are pure functions returning &'static str messages, unit-testable with zero fixtures"

requirements-completed: []

coverage:
  - id: D1
    description: "Pressing `e` on the Docs (Browse) tab resolves an editable markdown path and returns SuspendAndEdit on the first press — it never pushes the Enqueue overlay"
    requirement: "UIFIX-03"
    verification:
      - kind: unit
        ref: "src/ui/screens/detail.rs#test_browse_edit_target_view_depth_returns_file_path"
        status: pass
      - kind: unit
        ref: "src/ui/screens/detail.rs#test_browse_edit_target_list_depth_md_file"
        status: pass
      - kind: source-assertion
        ref: "src/ui/screens/detail.rs KeyCode::Char('e') arm — the Browse block returns before the generic has_planning fall-through"
        status: pass
    human_judgment: false
  - id: D2
    description: "Pressing `e` on an empty Browse listing shows `Select a markdown file to edit` — no editor launch, no panic"
    requirement: "UIFIX-03"
    verification:
      - kind: unit
        ref: "src/ui/screens/detail.rs#test_browse_edit_target_empty_listing_is_rejected"
        status: pass
      - kind: unit
        ref: "src/ui/screens/detail.rs#test_browse_edit_target_list_depth_directory_is_rejected"
        status: pass
    human_judgment: false
  - id: D3
    description: "A Browse path containing `/milestones/` shows `Archived files are read-only` and never launches the editor"
    requirement: "UIFIX-03"
    verification:
      - kind: unit
        ref: "src/ui/screens/detail.rs#test_browse_edit_target_milestones_path_is_read_only"
        status: pass
    human_judgment: false
  - id: D4
    description: "MUST NOT open $EDITOR on a path outside the project's `.planning/` browse root (prohibition, safety)"
    requirement: "UIFIX-03"
    verification:
      - kind: unit
        ref: "src/ui/screens/detail.rs#test_browse_edit_target_outside_root_is_rejected"
        status: pass
    human_judgment: false
  - id: D5
    description: "While a Browse file's content is still None (the Loading state), `e` is inert — no editor launch, no panic"
    requirement: "UIFIX-03"
    verification:
      - kind: unit
        ref: "src/ui/screens/detail.rs#test_browse_edit_target_loading_content_is_inert"
        status: pass
    human_judgment: false
  - id: D6
    description: "The Browse footer advertises a bold `[e]` followed by `dit`, ordered between [Enter]open and [Esc]up; every other tab's footer is unchanged"
    requirement: "UIFIX-03"
    verification:
      - kind: unit
        ref: "src/ui/screens/detail.rs#test_browse_footer_has_edit_hint"
        status: pass
      - kind: unit
        ref: "src/ui/screens/detail.rs#test_other_footers_unchanged_by_browse_edit_hint"
        status: pass
    human_judgment: false
  - id: D7
    description: "PageDown at the end of a document leaves the last content line on screen; the stored scroll offset never exceeds total_lines minus visible_height"
    requirement: "UIFIX-04"
    verification:
      - kind: unit
        ref: "src/ui/screens/detail.rs#test_clamp_scroll_page_down_stops_at_content_end"
        status: pass
      - kind: source-assertion
        ref: "src/ui/screens/detail.rs:584,628,817,849 — all four file-view sites assign through clamp_scroll"
        status: pass
    human_judgment: false
  - id: D8
    description: "Repeated PageDown at the end of content is idempotent — the stored offset stays at max_scroll instead of growing unbounded"
    requirement: "UIFIX-04"
    verification:
      - kind: unit
        ref: "src/ui/screens/detail.rs#test_clamp_scroll_repeated_page_down_is_idempotent"
        status: pass
    human_judgment: false
  - id: D9
    description: "When total_lines is less than or equal to visible_height the max scroll is 0, PageDown is a no-op, and the first line stays at the top"
    requirement: "UIFIX-04"
    verification:
      - kind: unit
        ref: "src/ui/screens/detail.rs#test_clamp_scroll_short_document_never_scrolls"
        status: pass
      - kind: unit
        ref: "src/ui/screens/detail.rs#test_clamp_scroll_pre_first_render_floor"
        status: pass
    human_judgment: false
  - id: D10
    description: "After repeated PageDown presses past the end of a long document, the first PageUp press moves the viewport — asserted against recorded viewport metrics rather than a live terminal"
    requirement: "UIFIX-04"
    verification:
      - kind: unit
        ref: "src/ui/screens/detail.rs#test_clamp_scroll_first_page_up_moves_viewport"
        status: pass
    human_judgment: false
  - id: D11
    description: "In a live terminal, pressing `e` on the Docs tab actually suspends the TUI and opens $EDITOR on the right file, and PageDown/PageUp feel correct at the end of a long document"
    requirement: "UIFIX-03, UIFIX-04"
    verification: []
    human_judgment: true
    rationale: "The resolver, the guards, the clamp formula and the four call sites are all pinned by tests, and the SuspendAndEdit -> $EDITOR spawn is shipped, unchanged machinery. But no test in this repo renders a frame or drives a real terminal (TestBackend is used nowhere and was explicitly out of scope), so the end-to-end suspend/resume round-trip and the perceived scroll feel are unverified by automation. A human should press `e` on a Docs file and PageDown to the end of a long SUMMARY."

# Metrics
duration: 9 min
completed: 2026-07-29
status: complete
---

# Phase 14 Plan 02: Docs Tab Edit Key + Scroll Clamp Summary

**`e` on the Docs tab now resolves through a pure, guarded `browse_edit_target` into the shipped `SuspendAndEdit` path instead of falling through to the Enqueue overlay, and all four file-view scroll sites clamp the stored offset to the renderer's own `total_lines - visible_height` bound.**

## Performance

- **Duration:** 9 min
- **Started:** 2026-07-29T06:17:07Z
- **Completed:** 2026-07-29T06:26:00Z
- **Tasks:** 2 (each executed as a RED/GREEN pair)
- **Files modified:** 1

## Accomplishments

- **UIFIX-03 fixed as a key-routing gap, not a new feature.** The `KeyCode::Char('e')` arm branched on Archive, Queue and Backlog only, so `DetailSubView::Browse` fell through to the generic `else` and pushed the Enqueue overlay. A `current_view == DetailSubView::Browse` block now sits between the Archive block and the generic fall-through and returns the **existing** `ScreenAction::SuspendAndEdit`. No second editor flow was invented; `ScreenAction` and the `Screen` trait are untouched.
- **All path logic is pure.** `browse_edit_target` performs zero filesystem I/O — depth resolution, root fence, and read-only guard are all in-memory path arithmetic. That is what made seven contract rows testable with `ProjectViewCache::default()` fixtures and no `AppContext` construction at all.
- **The browse-root fence is a real security boundary, and it is tested.** `test_browse_edit_target_outside_root_is_rejected` proves `/etc/passwd.md` cannot reach `$EDITOR` from a session rooted at `/proj/.planning`.
- **UIFIX-04 fixed at the handler, so the *stored* value is bounded.** The render path had always clamped for display; the stored offset kept growing, which is why PageUp was dead for several presses. All four sites named in the UI-SPEC root-cause table now assign through `clamp_scroll`.
- **`clamp_scroll` reuses the renderer's exact formula** (`total_lines.saturating_sub(visible_height)`) rather than a parallel calculation, so display and storage cannot drift.
- **`detail.rs` gained 14 tests** — it previously had only three JSON-parse tests. This is the repo's first key-routing coverage and first scroll-state coverage; quick task 260403-p84 (PageUp/PageDown) had shipped with zero tests.
- Test count went 227 → **241** total (190 → 204 lib tests). `detail.rs` tests: 3 → 17.

## Task Commits

1. **Task 1 RED: failing `[e]dit` routing tests** — `e631dc8` (test)
2. **Task 1 GREEN: `browse_edit_target` + footer hint** — `5b5fe07` (fix)
3. **Task 2 RED: failing scroll-clamp tests** — `b86b140` (test)
4. **Task 2 GREEN: apply the bound** — `a8f37aa` (fix)

## Files Created/Modified

- `src/ui/screens/detail.rs` — added `browse_edit_target`, `footer_spans` (extracted from `build_footer`), `clamp_scroll`, `ViewportMetrics`, and two `Cell<ViewportMetrics>` fields on `DetailScreen`; wired the Browse branch of the `e` arm; added the `[e]dit` Browse footer hint; recorded viewport metrics in both file-view render arms; clamped four scroll sites; appended 14 tests to the existing `mod tests` block at the end of the file.

## Decisions Made

All four plan-mandated decisions were **taken as written**. None were diverged from.

- **CD-01 — taken.** Viewport metrics live in two `Cell<ViewportMetrics>` fields on `DetailScreen`, not on `ProjectViewCache`. Confirmed against the source: `Screen::render(&self, frame, area, ctx: &AppContext)` (`src/ui/screens/mod.rs:33`) takes both receiver and context immutably, so no render path can write into the view cache. Options (a) changing the trait signature and (b) recomputing `visible_height` in a handler with no `Rect` were both rejected exactly as the plan reasoned. Interior mutability is safe here because `App` already holds `Vec<Box<dyn Screen>>` and is therefore already neither `Send` nor `Sync` — a `Cell` changes no auto-trait bound. Both render sites (`render_archive_tab` and `render_browser_tab`) already take `&self`, so no signature changed anywhere.
- **CD-02 — taken.** No `test_ctx()` helper was built. All 14 tests call `browse_edit_target`, `footer_spans`, or `clamp_scroll` directly. Zero `AppContext` constructions, zero new test-harness patterns, zero `TestBackend`.
- **CD-03 — taken. The generic `_ =>` scroll fallback was deliberately left UNCLAMPED.** `self.scroll_offset.saturating_add(1)` (`detail.rs:637`) and `self.scroll_offset.saturating_add(PAGE_SCROLL_LINES)` (`detail.rs:858`) still grow without an upper bound, and are still clamped only at render time. This is the identical defect shape on the seven non-file tabs, but each of those computes its content differently, so a correct `total_lines` would need per-tab plumbing across seven render arms — invasive, and the UI-SPEC explicitly says the optional fallback "must not expand the phase". Recorded here as a **decision, not a miss**; a follow-up can pick it up.
- **CD-04 — taken.** The root fence returns the existing `Select a markdown file to edit` message. No third guard string was invented, honoring the UI-SPEC Copywriting Contract's "no new user-facing strings except two" rule.

**Additional executor decision (not in the plan):** both RED commits include the *production wiring* against a stubbed resolver, rather than tests alone. Committing tests that reference a not-yet-existing function would leave the tree non-compiling, and committing the helper without a call site would fail `cargo clippy -- -D warnings` on `dead_code`. Wiring first against a stub keeps **every commit in the series buildable, clippy-clean and bisectable** while still producing a genuine RED. The Task 2 stub (`clamp_scroll` returning the offset unmodified) is particularly faithful — it *is* the shipped defect, and the RED output showed `left: 80, right: 70`, reproducing UIFIX-04 exactly.

**Requirements deliberately NOT marked complete.** `REQUIREMENTS.md` is intentionally unmodified. Plan **14-03 also declares UIFIX-03 and UIFIX-04** (verified: its frontmatter lists all four IDs), so the shared-ID gate correctly defers — identical to how 14-01 handled UIFIX-01/02. 14-03 closes all four.

## Deviations from Plan

**None.** No Rule 1 (bug), Rule 2 (missing critical), Rule 3 (blocker), or Rule 4 (architectural) deviation arose. Every acceptance criterion in both tasks matched its expected value on the first check — including all seven greps in Task 1 and all seven in Task 2. No plan-arithmetic defects were found this time.

The autonomous-run directive was in force; no checkpoint, decision point, or gate was reached that would have required user input. CD-01 through CD-04 were pre-decided by the plan, not by the executor.

**Total deviations:** 0.
**Impact on plan:** None. No scope creep, no scope reduction.

## Verification Results

| # | Check | Result |
|---|-------|--------|
| 1 | `cargo build` | exit 0 |
| 2 | `cargo test` | exit 0 — **241 passed**, 5 suites |
| 3 | `cargo clippy -- -D warnings` (lib gate) | exit 0 — no issues |
| 4 | `cargo test --lib ui::screens::detail::tests::` | **17 passed** (>= 16 required: 3 pre-existing + 14 new) |
| 5 | `grep -c 'TestBackend' src/ui/screens/detail.rs` | **0** — no rendering-harness pattern introduced |
| 6 | `cargo clippy --all-targets -- -D warnings` | still **exactly 5** pre-existing lints — count did not grow |

### Task 1 acceptance criteria

| Criterion | Expected | Actual |
|-----------|----------|--------|
| `grep -c 'fn browse_edit_target'` | 1 | **1** |
| `grep -c 'browse_edit_target('` | >= 8 | **9** |
| `grep -c 'fn footer_spans'` | 1 | **1** |
| `grep -c 'Select a markdown file to edit'` | >= 1 | **2** |
| `grep -c 'Archived files are read-only'` | >= 2 | **3** |
| `grep -c 'fn test_browse_edit_target'` | 7 | **7** |
| `grep -c 'ScreenAction::SuspendAndEdit'` | 3 | **3** |
| `git status --porcelain src/ui/screens/mod.rs` | no output | **no output** |

### Task 2 acceptance criteria

| Criterion | Expected | Actual |
|-----------|----------|--------|
| `grep -c 'fn clamp_scroll'` | 1 | **1** |
| `grep -c 'clamp_scroll'` | >= 11 | **17** |
| `grep -c 'struct ViewportMetrics'` | 1 | **1** |
| `grep -c 'Cell<ViewportMetrics>'` | 2 | **2** |
| `grep -c 'const PAGE_SCROLL_LINES: u16 = 20;'` | 1 | **1** |
| `grep -c 'fn test_clamp_scroll'` | 5 | **5** |

**Source assertion (Task 2):** `grep -n "scroll_offset" | grep "saturating_add"` returns exactly six lines. Four (`:584`, `:628`, `:817`, `:849`) are the file-view sites and every one is an argument *inside* a `clamp_scroll(...)` call. The remaining two (`:637`, `:858`) are the generic `_ =>` fallback on `self.scroll_offset`, left unclamped per CD-03. No file-view `saturating_add` lands outside a `clamp_scroll` call.

**RED gate evidence:**
- Task 1: 8 of 12 tests failed (`left: Err("unimplemented")` against each expected value; the footer test failed on `text.contains("[e]dit  ")`). `test_other_footers_unchanged_by_browse_edit_hint` passed at RED by design — it is a regression guard, not a driver.
- Task 2: all 5 tests failed, with `left: 80 / right: 70` on the end-of-content case — the shipped UIFIX-04 defect reproduced exactly as the UI-SPEC described.

## Scope Fence Compliance

- Touched **only** `src/ui/screens/detail.rs`. `git diff --stat ceaa9c4..HEAD` reports 1 file changed, 337 insertions, 12 deletions.
- `src/ui/screens/normal.rs` and `src/state_reader/mod.rs` (plan 14-01's files, already merged) untouched.
- `.planning/todos/` (plan 14-03's files) untouched.
- `STATE.md` and `ROADMAP.md` untouched — orchestrator-owned.
- `src/ui/screens/mod.rs` untouched: `ScreenAction`, the `Screen` trait signature, and `ProjectViewCache` are all unchanged.
- `PAGE_SCROLL_LINES` unchanged at 20. No keybinding refactor, no scroll-behavior redesign.
- The already-clamped list-selection sibling arms were not touched.
- Wave 1's work was neither re-derived nor reverted.
- Zero new Cargo dependencies. Zero new test-harness patterns.
- `git diff --diff-filter=D` reports **no deletions**.
- The 5 pre-existing `--all-targets` clippy lints were left alone, as CONTEXT.md deferred. No `assert_eq!(x, true)` / `assert_eq!(x, false)` was written; the new test module sits at the END of the file, so no sixth `items_after_test_module` lint appears.

## Threat Model Compliance

- **T-14-01 (Tampering, `mitigate`) — enforced and tested.** The `$EDITOR` launch path gained two guards on top of the unchanged, argument-vector process spawn from quick task 260401-t7y: a browse-root containment fence and the `/milestones/` read-only guard. **No new process-spawn code was written** — the Browse branch returns the same `ScreenAction::SuspendAndEdit(PathBuf)` the Archive and Backlog branches already return.
- **T-14-06 (Tampering, `mitigate`) — enforced.** Archived-milestone immutability now holds regardless of which tab reached the file. Pinned by `test_browse_edit_target_milestones_path_is_read_only`.
- **T-14-04 (DoS, `mitigate`) — fixed.** This was UIFIX-04 itself. `u16` saturating arithmetic had already prevented an overflow panic, so the residual harm was a dead-key UX defect; the offset is now bounded at the handler.
- **T-14-02 (EoP, `accept`) — unchanged.** `$VISUAL`/`$EDITOR` binary selection is pre-existing and untouched.
- **T-14-SC (Tampering, `accept`) — not applicable.** Zero dependencies added; no package-manager install ran. The Package Legitimacy Gate did not apply.

No `## Threat Flags` — no new network endpoints, auth paths, file-access patterns, or trust-boundary schema changes were introduced. Note that `browse_edit_target` *reduced* the file-access surface: before this plan, the Docs tab had no editor path at all, and the new one is fenced twice.

## Known Stubs

None. No `TODO`, `FIXME`, `todo!()`, `unimplemented!()`, placeholder value, or skipped test exists in the touched file. Both transient RED stubs (`Err("unimplemented")` and the identity `clamp_scroll`) were replaced by their GREEN commits and exist only inside `e631dc8` and `b86b140` respectively — neither survives at `HEAD`. Every `<verify>` in the plan was executed; no verification was deferred.

**One deliberate, documented omission** (not a stub): the generic `_ =>` scroll fallback remains unclamped per CD-03. See Decisions Made.

## Flagged Assumptions A-02 and A-03 — status

The plan surfaced two `unclassified, unresolved` edge-probe rows. Both assumed shapes **held**:

- **A-02 (UIFIX-03) — assumed shape "key-routing dispatch with two guard predicates": confirmed.** The root cause was exactly a missing `match`/`if` branch, and exactly two guards were required. **Dimensions NOT covered, as the plan warned:** concurrent edits to the same file from two TUI instances (no locking exists anywhere in this codebase), and `$EDITOR` exiting non-zero (the shipped suspend/resume path swallows the exit status — pre-existing behavior from quick task 260401-t7y, not introduced here). Both remain for manual review.
- **A-03 (UIFIX-04) — assumed shape "a numeric bound on a monotonically increasing counter": confirmed.** The fix is a single `.min()` against a two-field metric. **Dimension NOT covered, as the plan warned:** a terminal resize between the last render and the next keypress leaves the recorded `visible_height` stale for exactly one frame, so one keypress can clamp against the previous geometry. Self-correcting on the next render; harmless in practice (the render path still clamps for display, so nothing renders out of range). Noted for manual review.

## Issues Encountered

None. Both tasks executed exactly as planned, and every acceptance criterion matched on first check.

One environmental note: the worktree sandbox rejects compound shell commands containing redirects, so verification greps were run as separate plain commands rather than a single loop. No impact on results.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- **Ready for 14-03** (todo closeout + requirement marking). 14-03 must mark **all four** of UIFIX-01, UIFIX-02, UIFIX-03 and UIFIX-04 complete in `REQUIREMENTS.md` — both 14-01 and this plan deliberately left them `Pending` because the shared-ID gate blocks a plan from flipping an ID a sibling also declares.
- **Zero file overlap with 14-03:** this plan touched only `src/ui/screens/detail.rs`; 14-03 owns `.planning/todos/`.
- **One item worth carrying forward:** the generic `_ =>` scroll fallback (CD-03) is a known, deliberate omission with the identical defect shape on non-file tabs. It is not a Phase 14 success criterion, but it is a real dead-key defect on seven tabs and a natural follow-up.
- **No blockers.**

---
*Phase: 14-ui-fixes*
*Completed: 2026-07-29*

## Self-Check: PASSED

- `src/ui/screens/detail.rs` — FOUND on disk
- `.planning/phases/14-ui-fixes/14-02-SUMMARY.md` — FOUND on disk
- Commit `e631dc8` (Task 1 RED) — FOUND in git log
- Commit `5b5fe07` (Task 1 GREEN) — FOUND in git log
- Commit `b86b140` (Task 2 RED) — FOUND in git log
- Commit `a8f37aa` (Task 2 GREEN) — FOUND in git log
- All plan `<verification>` items 1-6 re-run and passing (see Verification Results)
- All 14 task `<acceptance_criteria>` re-run; every one matched its expected value
- TDD gate sequence present per task: `test(...)` RED commit precedes its `fix(...)` GREEN commit for both tasks. No REFACTOR commit was needed — neither GREEN left cleanup behind.
