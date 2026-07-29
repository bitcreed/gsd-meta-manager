---
phase: 14-ui-fixes
plan: 01
subsystem: ui
tags: [rust, ratatui, tui, regression-test, display-defect, handoff, pause-badge]

# Dependency graph
requires:
  - phase: 11-paused-project-detection
    provides: detect_handoff + ProjectState.paused + the cyan pause badge this plan verifies
  - phase: quick-260722-emn
    provides: the external-job hourglass badge that sits at priority rank 2
provides:
  - "alias_badge(is_paused, external_job_waiting, has_session) -> Option<(&'static str, Color)> — the shipped three-tier badge priority as a pure, testable function"
  - "compact_pipeline emitting the exact 13-cell `D  R  P  E  V` contract with no leading or trailing pad cell"
  - "The first #[cfg(test)] mod tests under src/ui/ (10 tests)"
  - "Five HANDOFF pause-detection regression tests inside the state reader's existing test block"
affects: [14-02, 14-03, dashboard-row-rendering, status-column-alignment]

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "ratatui Line assertion via span-content concatenation (new to this repo)"
    - "Locating stage spans by content rather than index so separator spans cannot shift assertions"

key-files:
  created: []
  modified:
    - src/ui/screens/normal.rs
    - src/state_reader/mod.rs

key-decisions:
  - "UIFIX-01 confirmed ALREADY SATISFIED by the shipped Phase 11 implementation — the 2026-03-27 todo is stale. Zero production changes to detection or badge vocabulary."
  - "alias_badge extracted as a behavior-preserving refactor so badge priority is testable without constructing an 18-field AppContext"
  - "UIFIX-02 fixed at the construction site (separator emitted only between stages) rather than by trimming at the call site"
  - "Requirements UIFIX-01/UIFIX-02 deliberately NOT marked complete — plan 14-03 also declares them and has no SUMMARY yet, so the shared-ID gate correctly defers"

patterns-established:
  - "src/ui/ modules can carry inline #[cfg(test)] mod tests at the END of the file (avoids the items_after_test_module lint)"
  - "Badge/glyph decisions are pure functions returning &'static str — never file-derived text (threat T-14-03 enforced structurally)"

requirements-completed: [UIFIX-01, UIFIX-02]

coverage:
  - id: D1
    description: "A project whose .planning/ holds a non-empty HANDOFF.md is reported as paused and renders the cyan pause badge on its dashboard row"
    requirement: "UIFIX-01"
    verification:
      - kind: integration
        ref: "src/ui/screens/normal.rs#test_paused_project_shows_pause_badge_end_to_end"
        status: pass
    human_judgment: false
  - id: D2
    description: "All eight UI-SPEC UIFIX-01 HANDOFF states (md non-empty, json next_action, whitespace-only, absent, invalid json, pause>session, pause>async, async>session) behave as contracted"
    requirement: "UIFIX-01"
    verification:
      - kind: unit
        ref: "src/state_reader/mod.rs#test_handoff_md_non_empty_sets_paused"
        status: pass
      - kind: unit
        ref: "src/state_reader/mod.rs#test_handoff_json_non_empty_sets_paused_with_context"
        status: pass
      - kind: unit
        ref: "src/state_reader/mod.rs#test_handoff_md_whitespace_only_is_not_paused"
        status: pass
      - kind: unit
        ref: "src/state_reader/mod.rs#test_no_handoff_file_is_not_paused"
        status: pass
      - kind: unit
        ref: "src/state_reader/mod.rs#test_handoff_json_invalid_is_paused_without_context"
        status: pass
      - kind: unit
        ref: "src/ui/screens/normal.rs#test_pause_badge_wins_over_session"
        status: pass
      - kind: unit
        ref: "src/ui/screens/normal.rs#test_pause_badge_wins_over_async_job"
        status: pass
      - kind: unit
        ref: "src/ui/screens/normal.rs#test_async_job_badge_when_not_paused"
        status: pass
      - kind: unit
        ref: "src/ui/screens/normal.rs#test_no_badge_when_nothing_active"
        status: pass
    human_judgment: false
  - id: D3
    description: "At most one badge renders per dashboard row — two glyphs never appear in the alias cell"
    requirement: "UIFIX-01"
    verification:
      - kind: unit
        ref: "src/ui/screens/normal.rs#test_badge_is_never_two_glyphs"
        status: pass
    human_judgment: false
  - id: D4
    description: "The D-R-P-E-V status cell renders exactly `D  R  P  E  V` (13 terminal cells) and never begins with whitespace, for every DiskStatus variant"
    requirement: "UIFIX-02"
    verification:
      - kind: unit
        ref: "src/ui/screens/normal.rs#test_compact_pipeline_has_no_leading_blank"
        status: pass
      - kind: unit
        ref: "src/ui/screens/normal.rs#test_compact_pipeline_exact_cells_and_width"
        status: pass
      - kind: unit
        ref: "src/ui/screens/normal.rs#test_compact_pipeline_no_workflow_data_still_renders_five_stages"
        status: pass
    human_judgment: false
  - id: D5
    description: "Per-stage D-R-P-E-V color rule (Green at/past threshold, Yellow one step below, DarkGray otherwise) survives the pad-cell removal"
    requirement: "UIFIX-02"
    verification:
      - kind: unit
        ref: "src/ui/screens/normal.rs#test_compact_pipeline_stage_colors_preserved"
        status: pass
    human_judgment: false
  - id: D6
    description: "In a live terminal, `D` visually aligns with the first character of sibling Status values (`executing`, `verifying`, `v1.0 Complete`) at widths >=80, >=60, and <60"
    requirement: "UIFIX-02"
    verification: []
    human_judgment: true
    rationale: "The construction-site fix provably feeds all three width tiers (one status_cell, three cell vectors), and the 13-cell contract is asserted. But column alignment as *perceived* in a real terminal depends on ratatui's percentage-width layout and the user's font — no test in this repo renders a frame (TestBackend is used nowhere, and introducing it was explicitly out of scope). A human should eyeball one dashboard row."

# Metrics
duration: 6 min
completed: 2026-07-29
status: complete
---

# Phase 14 Plan 01: Dashboard Row Display Defects Summary

**UIFIX-01 proven already-shipped by an end-to-end HANDOFF tracer (todo was stale), and UIFIX-02's leading pad cell removed at the `compact_pipeline` construction site — 16 cells to the contracted 13.**

## Performance

- **Duration:** 6 min
- **Started:** 2026-07-29T06:08:11Z
- **Completed:** 2026-07-29T06:14:34Z
- **Tasks:** 3
- **Files modified:** 2

## Accomplishments

- **UIFIX-01 verified, not reimplemented.** A non-empty `HANDOFF.md` written to a temp `.planning/` flows through the real `parse_project_state` into a cyan pause badge decision — and the test passed on the **first run against completely unmodified detection code**. The 2026-03-27 todo is confirmed stale; Phase 11 already shipped this.
- **All eight UI-SPEC UIFIX-01 rows plus the no-badge row now have committed regression tests**, split between the state reader (file → flag) and the dashboard module (flag → badge). All nine passed against unmodified production code.
- **UIFIX-02 fixed at the source.** `compact_pipeline` padded every stage label on both sides (`format!(" {} ", label)`), producing `" D  R  P  E  V "` — 16 cells with a leading blank that misaligned the Status column. Now the two-cell separator is emitted only *between* stages.
- **`src/ui/` gained its first `#[cfg(test)] mod tests`** — 10 tests, placed at the end of the file so no sixth `items_after_test_module` lint appears.
- Test count went 175 → 190 lib tests (227 total across 5 suites).

## Task Commits

1. **Task 1 (tracer): end-to-end pause badge** — `2bb5aad` (test)
2. **Task 2: full eight-row HANDOFF contract** — `b4ab7ce` (test)
3. **Task 3 RED: failing D-R-P-E-V tests** — `aba2639` (test)
4. **Task 3 GREEN: remove pad cells** — `2000a00` (fix)

## Files Created/Modified

- `src/ui/screens/normal.rs` — extracted `alias_badge` (three-tier badge priority as a pure fn); rewrote `alias_cell` to match on it; rewrote `compact_pipeline` to drop the pad cells; added the module's first `#[cfg(test)] mod tests` (10 tests).
- `src/state_reader/mod.rs` — five HANDOFF regression tests appended inside the **existing** `mod tests` block (no second module, `make_planning` reused verbatim). No production code touched.

## Decisions Made

- **UIFIX-01 required zero production changes.** Per CONTEXT.md D-01 this was verify-first. The tracer and all eight expansion tests passed against unmodified `detect_handoff` / `parse_project_state` / badge priority. Nothing in the shipped detection or badge vocabulary was altered.
- **`alias_badge` extraction was the only production edit UIFIX-01 authorized.** It is behavior-preserving: same glyphs (`U+23F8`/`U+23F3`/`U+25B6` each + one space), same colors (Cyan/Yellow/Green), same priority order, same one-badge-per-row rule. The escape forms were copied byte-for-byte from the original call site.
- **UIFIX-02 fixed by restructuring span assembly, not by trimming.** An explicit loop pushes a bare stage letter per span and an unstyled `"  "` separator only when `i > 0`. Each letter remains its own span, so per-letter color survives; the return type stays `Line<'static>`.
- **Requirements UIFIX-01/UIFIX-02 were NOT marked complete.** `requirements.ready-ids` returned `0/2 ready` because plan **14-03 also declares both IDs** and has no SUMMARY yet. The shared-ID gate is working as designed — 14-03 will flip them when it closes. `REQUIREMENTS.md` is intentionally unmodified.

## Deviations from Plan

No code deviations — no bug, missing-critical, blocker, or architectural change arose. Both items below are **defects in the plan's acceptance-criteria arithmetic**, not in the code, and neither required a Rule 1-4 fix.

### 1. [Plan defect — acceptance criterion miscount] `grep -c 'fn test_handoff'` expects 3, actual is 4

- **Found during:** Task 2
- **Issue:** The criterion states the grep outputs `3`. But the plan's own "Artifacts this phase produces" table prescribes four test names beginning with `test_handoff`: `test_handoff_md_non_empty_sets_paused`, `test_handoff_json_non_empty_sets_paused_with_context`, `test_handoff_md_whitespace_only_is_not_paused`, `test_handoff_json_invalid_is_paused_without_context`. (The fifth, `test_no_handoff_file_is_not_paused`, has its own separate criterion.) The expected count is simply off by one.
- **Resolution:** Kept the prescribed names — the artifacts table is authoritative and the names encode the UI-SPEC rows. Recorded the arithmetic error rather than renaming a test to satisfy a miscount.
- **Substantive intent:** satisfied — all five HANDOFF tests exist and pass.

### 2. [Plan defect — grep proxy false positive] `grep -c 'assert_eq!(.*, true)'` expects 0, reports 4 in `normal.rs`

- **Found during:** Task 2
- **Issue:** The criterion is a proxy for clippy's `bool_assert_comparison` lint ("never compare a boolean to a literal"). The regex matches `, true)` **inside the argument list** of calls like `assert_eq!(alias_badge(true, true, true), Some(PAUSE_BADGE))` — the arguments to the function under test, not a boolean compared against a literal.
- **Resolution:** Verified the real constraint authoritatively with clippy instead of the regex. `cargo clippy --all-targets -- -D warnings` still reports exactly **3** `bool_assert_comparison` errors, all pre-existing in `browser.rs`, and **5** lints total — unchanged from baseline. No new bool-literal comparison was introduced. Reordering the boolean arguments purely to dodge the regex would have distorted the tests' meaning.
- **Substantive intent:** satisfied — no `assert!(x)`/`assert!(!x)` convention was violated.

---

**Total deviations:** 0 code deviations; 2 documented plan-artifact defects.
**Impact on plan:** None on behavior or scope. All substantive intents satisfied; no scope creep.

## Verification Results

| # | Check | Result |
|---|-------|--------|
| 1 | `cargo build` | exit 0 |
| 2 | `cargo test` | exit 0 — **227 passed**, 5 suites |
| 3 | `cargo clippy -- -D warnings` (lib gate) | exit 0 — no issues |
| 4 | `cargo test --lib ui::screens::normal::tests::` | **10 passed** (>= 9 required) |
| 5 | `cargo test --lib state_reader::tests::` | **16 passed** (11 pre-existing + 5 new HANDOFF) |
| 6 | `cargo clippy --all-targets -- -D warnings` | still **exactly 5** pre-existing lints — 3x `assert_eq!` literal bool (`browser.rs`), 1x owned-instance-for-comparison (`project_creator.rs`), 1x items-after-test-module (`state_reader/mod.rs`). Count did not grow. |

**RED gate evidence (Task 3):** before the fix, all four `test_compact_pipeline_*` tests failed with `left: " D  R  P  E  V "` / `right: "D  R  P  E  V"` — the defect reproduced exactly as the UI-SPEC described. After the fix, all four pass.

## Scope Fence Compliance

- Touched only `src/ui/screens/normal.rs` and `src/state_reader/mod.rs`. `src/ui/screens/detail.rs` (plan 14-02's file) untouched.
- No changes to `STATE.md` or `ROADMAP.md` (orchestrator-owned).
- Zero new Cargo dependencies. `tempfile` was already a runtime dependency.
- Pause glyph, its color, badge priority order, and the two-cell inter-stage rhythm are all byte-identical to what shipped.
- Pipeline tab's expanded rendering in `detail.rs` (deliberate 2-cell panel indent) untouched, per UI-SPEC Non-Goals.
- The 5 pre-existing `--all-targets` clippy lints were left alone, as CONTEXT.md deferred.

## Threat Model Compliance

- **T-14-03 (Information Disclosure, `mitigate`):** now enforced *structurally* rather than by convention. `alias_badge` returns `Option<(&'static str, Color)>` — the glyph is a compile-time literal, so no HANDOFF file body can reach the dashboard row by construction. `must_haves.prohibitions` (no HANDOFF body text in the alias or status cells) holds. Pause context remains confined to the detail view.
- **T-14-05 (DoS, `accept`):** `detect_handoff`'s `read_to_string` is unchanged shipped Phase 11 behavior.
- **T-14-SC (Tampering, `accept`):** zero dependencies added; no package-manager install ran. The Package Legitimacy Gate did not apply.

No new threat surface. No `## Threat Flags` — no new network endpoints, auth paths, file-access patterns, or trust-boundary schema changes were introduced.

## Known Stubs

None. No `TODO`, `FIXME`, `todo!()`, `unimplemented!()`, placeholder value, or skipped test exists in either touched file. Every `<verify>` in the plan was executed; no verification was deferred.

## Flagged Assumption A-01 — status

The plan surfaced A-01: the deterministic edge probe could not classify UIFIX-01's shape, and the assumed shape was *boolean-flag-from-file-presence*. That assumption **held** — `detect_handoff` is exactly a file-presence-plus-non-empty predicate returning `(bool, Option<String>)`, verified by reading `src/state_reader/mod.rs:57-99`. No ordering or concurrency dimension exists: `HANDOFF.json` is checked before `HANDOFF.md` with a single early return, and only one `.planning/` directory is ever consulted per project. **The one dimension not covered:** both `HANDOFF.json` and `HANDOFF.md` present simultaneously — the json wins by construction, which is deterministic but untested here. Low risk; noted for manual review.

## Issues Encountered

None. The only friction was the two acceptance-criteria arithmetic defects documented above, resolved by verifying substantive intent with the authoritative tool (clippy) rather than the regex proxy.

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- **Ready for 14-02** (UIFIX-03 markdown edit mode, UIFIX-04 PageDown clamp). Zero file overlap: 14-02 owns `src/ui/screens/detail.rs` and `src/ui/screens/mod.rs`, neither of which this plan touched.
- **Ready for 14-03** (todo closeout). 14-03 must mark UIFIX-01 and UIFIX-02 complete in `REQUIREMENTS.md` — this plan deliberately left them `Pending` because the shared-ID gate blocks a plan from flipping an ID a sibling also declares. 14-03 should also record in the `.planning/todos/pending/` UIFIX-01 todo that it was **stale, not fixed**.
- **No blockers.**

---
*Phase: 14-ui-fixes*
*Completed: 2026-07-29*

## Self-Check: PASSED

- `src/ui/screens/normal.rs` — FOUND on disk
- `src/state_reader/mod.rs` — FOUND on disk
- Commit `2bb5aad` — FOUND in git log
- Commit `b4ab7ce` — FOUND in git log
- Commit `aba2639` — FOUND in git log
- Commit `2000a00` — FOUND in git log
- All plan `<verification>` items 1-6 re-run and passing (see Verification Results)
- All task `<acceptance_criteria>` re-run; the only two non-matching are the documented plan-arithmetic defects, both with substantive intent satisfied
