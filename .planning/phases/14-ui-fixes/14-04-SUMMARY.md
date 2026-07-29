---
phase: 14-ui-fixes
plan: 04
subsystem: tui-dashboard
tags: [rust, ratatui, tui, layout-constraints, scroll, regression-test, testbackend, gap-closure]
status: complete
requires:
  - "14-01, 14-02, 14-03 (the phase implementation this plan closes gaps in)"
  - "src/ui/screens/detail.rs clamp_scroll + ViewportMetrics (14-02)"
  - "src/ui/screens/normal.rs compact_pipeline (14-01)"
provides:
  - "dashboard_columns / dashboard_table — single source of truth for the dashboard tier layout"
  - "STATUS_COLUMN_MIN_CELLS — the 13-cell floor coupling compact_pipeline to its column"
  - "render_dashboard_interior / render_dashboard_rows — the repo's first rendered-frame test harness"
  - "test_ctx — the repo's first AppContext test fixture"
  - "generic_viewport — recorded viewport metrics for the Phases and Roadmap tabs"
affects:
  - "src/ui/screens/normal.rs"
  - "src/ui/screens/detail.rs"
tech-stack:
  added: []
  patterns:
    - "ratatui::backend::TestBackend render-level regression testing (no dependency change required)"
    - "clamp-then-subtract ordering for all up-direction scroll handlers"
    - "Cell<ViewportMetrics> interior mutability to pass render-pass metrics to &mut self key handlers"
key-files:
  created: []
  modified:
    - "src/ui/screens/normal.rs"
    - "src/ui/screens/detail.rs"
decisions:
  - "GD-01 taken: CD-03/IN-07 closed in this plan rather than deferred a second time"
  - "GD-02 taken: the <60 width tier deliberately left without a floor"
  - "GD-03 taken: REQUIREMENTS.md deliberately left unmodified"
  - "GD-04 taken: the fix is a layout floor, not a cell rewrite — compact_pipeline untouched"
metrics:
  duration: "~1 session"
  completed: 2026-07-29
  tasks: 3
  commits: 3
  tests_added: 14
---

# Phase 14 Plan 04: Status-Column Floor and Up-Direction Scroll Clamp Summary

Closed both `14-VERIFICATION.md` FAILED gaps — the D-R-P-E-V Status column clipped at eleven
common terminal widths including the default 80 (UIFIX-02 / CR-01), and the four up-direction
scroll handlers that never clamped the stored offset (UIFIX-04 / WR-02) — plus CD-03 / IN-07,
by giving the Status column a hard 13-cell floor and applying `clamp_scroll` at all eight
previously-unclamped sites, pinned by the repository's first rendered-frame and first
real-key-handler tests.

## What Was Built

| Task | Commit | What |
|------|--------|------|
| 1 (tracer) | `e27ab10` | `STATUS_COLUMN_MIN_CELLS`, `dashboard_columns`, `dashboard_table`, `TestBackend` harness, 6 render-level tests |
| 2 | `e1ac518` | Four up-direction file-view clamps, `test_ctx` fixture, 6 real-handler tests |
| 3 | `0e4bcdf` | `generic_viewport` + four generic-arm clamps (CD-03 closure), 2 tests, full project gate |

### Gap 1 — UIFIX-02 Status-column clipping

The defect was never in `compact_pipeline` (which always produced a correct 13-cell `Line`) but
in the `Constraint::Percentage` column holding it. `render_main`'s three inline tier branches were
extracted into `dashboard_columns` / `dashboard_table` so the tests exercise the exact production
layout, and the Status constraint at the `>=80` and `>=60` tiers became
`Constraint::Min(STATUS_COLUMN_MIN_CELLS)` where `STATUS_COLUMN_MIN_CELLS = 13`.

The two-cell outer/inner width gap is preserved deliberately: the tier is selected from the
**outer** area width while the constraints are resolved against the block's **inner** rect. That
gap is exactly why width 80 clipped, and it is documented on `dashboard_columns`.

### Gap 2 — UIFIX-04 up-direction scroll clamp

The four down-direction sites assigned through `clamp_scroll` against the recorded
`ViewportMetrics`; their four up-direction siblings performed a bare `saturating_sub` on the
stored offset. All four now read the viewport `Cell` and clamp **before** subtracting.

### CD-03 / IN-07 closure

A third `Cell<ViewportMetrics>` (`generic_viewport`) was added for the two sub-views that reach
the `_ =>` fallback, recorded in `render_phase_list` and `render_roadmap` from the content and
viewport heights those functions already computed, and consumed by all four generic arms.

## Measured Before-and-After (UIFIX-02)

Measured in this worktree by reverting the floor and re-running the sweep test, so these are
reproduced results, not carried-over plan figures.

| | Result |
|---|---|
| **Before** (`Percentage(15)` / `Percentage(20)`) | `D  R  P  E  V` **absent** — trailing `V` clipped — at widths **60, 61, 62, 63, 66, 80, 81, 82, 83, 84, 85** (11 widths, including the default 80) |
| **After** (`Min(13)` at the `>=80` and `>=60` tiers) | `D  R  P  E  V` **present at every width from 44 through 200 inclusive — 157/157** |

The reproduced before-list matches the plan's empirical grounding **exactly**, including width 84
(which the code review listed but the verification report omitted). No sibling column was starved:
the alias, phase, progress and backlog cells all still render, and at width 80 the pipeline cell
begins at the same buffer column as a sibling plain-text `executing` status.

## Ordering Is Load-Bearing — Verified, Not Assumed

The plan flagged clamp-then-subtract vs subtract-then-clamp as a real distinction. It was
confirmed empirically by implementing both:

| Implementation | Stored offset 90, metrics 100/60 (max_scroll 40) | Verdict |
|---|---|---|
| Bare `saturating_sub` (as shipped) | PageUp → **70** — still above max_scroll, viewport does not move | the reported dead-key defect |
| Subtract-then-clamp | PageUp → **40** — lands exactly on max_scroll, the value the renderer was already displaying, so the viewport still does not visibly move | reproduces the gap *through* the fix |
| Clamp-then-subtract (shipped) | PageUp → **20**, strictly below max_scroll | correct |

`test_first_page_up_after_viewport_grows_moves_viewport` fails under both wrong orderings, so the
distinction is pinned by a test rather than by a comment.

## Verification

RED was demonstrated for both gaps before accepting GREEN:

- Reverting the Status floor failed 3 tests with the clipped-width list `[60, 61, 62, 63, 66, 80,
  81, 82, 83, 84, 85]`.
- Reverting the four up-direction clamps failed 5 tests (offset 90 → 70 instead of 20; 90 → 89
  instead of 39).

Final project gate (CLAUDE.md release-process sequence), all fresh, all exit 0:

| Check | Result |
|---|---|
| `cargo build` | exit 0 |
| `cargo test` | **255 passed**, 0 failed (218 lib + 12 + 25; baseline 241) |
| `cargo test --lib` | **218 passed** (baseline 204) |
| `cargo clippy -- -D warnings` | exit 0, clean |
| `cargo test --lib ui::screens::normal::tests::` | **16 passed** (baseline 10) |
| `cargo test --lib ui::screens::detail::tests::` | **25 passed** (baseline 17) |
| `git diff --diff-filter=D --name-only` | empty — nothing deleted |

### `cargo clippy --all-targets` — final count

**Exactly 5 warnings, unchanged from baseline.** Locations:

| Count | File | Lint |
|---|---|---|
| 3 | `src/browser.rs:131,132,133` | `used assert_eq! with a literal bool` |
| 1 | `src/project_creator.rs:146` | `this creates an owned instance just for comparison` |
| 1 | `src/state_reader/mod.rs:258` | `items after a test module` |

The count did **not** grow, and **no reported location lies in either file this plan edits**. All
five are pre-existing and were deliberately left unfixed per the scope fence. New test code used
`assert!` / `!assert!` rather than comparisons against boolean literals specifically to avoid
adding a fourth lint of the first shape, and new tests were placed inside the existing test
modules to avoid an `items after a test module` lint.

Note: the executing prompt's scope fence cited these paths as `src/ui/screens/browser.rs`,
`src/ui/project_creator.rs`; the actual (and plan-stated) paths are `src/browser.rs` and
`src/project_creator.rs`. The plan's paths were correct.

## Decisions

All four plan decisions were taken as written; none were diverged from.

- **GD-01 — taken.** CD-03 / IN-07 closed here rather than deferred again. Independently
  re-confirmed in this worktree: `DetailSubView` has 10 variants, the scroll handlers carry 8
  explicit arms (`GitHistory`, `Backlog`, `Pipeline`, `Queue`, `Sessions`, `Archive`, `Defaults`,
  `Browse`), so the `_ =>` arm is reached by exactly **two** sub-views — `PhaseList` and
  `RoadmapViz`. CD-03's "seven non-file tabs / per-tab plumbing across seven render arms" cost
  estimate was factually wrong; the real cost was one `Cell`, two recording lines and four handler
  clamps.
- **GD-02 — taken.** The `<60` tier keeps `Percentage(30)` and gets no floor. Verified it holds all
  five stages from width 44 up. The sub-44 clipping residue remains a recorded follow-up.
- **GD-03 — taken.** See the explicit statement below.
- **GD-04 — taken.** `compact_pipeline` is byte-identical to its pre-task form; the only diff
  hunks in `normal.rs` are the extraction, the constraint values and the new tests.

## REQUIREMENTS.md Was Deliberately Left Unmodified

**`.planning/REQUIREMENTS.md` was not touched by this plan.** UIFIX-02 and UIFIX-04 remain marked
`Gaps Found` in the traceability table. Flipping them belongs to the re-verification pass that runs
after this plan and independently confirms the gaps closed — a plan marking its own requirements
complete is precisely how the previous round produced a false green (decision GD-03). This
deliberately overrides the executor's normal REQUIREMENTS.md handling.

`git status --porcelain Cargo.toml Cargo.lock src/ui/screens/mod.rs .planning/REQUIREMENTS.md`
produces no output.

## Deviations from Plan

**1. [Documentation] Task 1 acceptance criterion `grep -c 'dashboard_table(' >= 4` reads 2.**

- **Found during:** Task 1 acceptance check.
- **Issue:** The criterion assumed the definition line would match the literal `dashboard_table(`
  and that the empty-row test would call the function directly. The shipped signature is
  `fn dashboard_table<'a>(rows: Vec<Row<'a>>, terminal_width: u16) -> Table<'a>` — the lifetime
  parameter means the definition line does not contain the substring — and the empty-row test goes
  through the shared `render_dashboard_interior` harness rather than duplicating it.
- **Resolution:** No code change. The criterion's *intent* — `dashboard_table` is the single
  construction path used by both production and tests — is fully met: `grep -c 'dashboard_table'`
  returns 5 (definition, `render_main` call site, test-harness call site, plus two references).
  Contriving extra call sites purely to satisfy a substring count would have made the tests worse.
- **Files modified:** none.

**2. [Rule 3 - Blocking] Two rustfmt artifacts introduced by this plan's own code, fixed.**

- **Found during:** Task 3 gate.
- **Issue:** The repository carries large pre-existing `cargo fmt` drift (IN-09, deferred), which
  masks new drift. Measured precisely: `detail.rs` had **96** fmt diffs at HEAD and momentarily
  **99** after the Task 3 edits; `normal.rs` had 1 pre-existing (line 481) and momentarily 2.
- **Fix:** Reformatted the two generic up-direction arms to rustfmt's preferred compact form, and
  inserted a blank line before a new comment that abutted a trailing `// borders` comment (rustfmt
  was treating it as a trailing-comment continuation and re-indenting it to column 62).
- **Result:** `detail.rs` back to **96** and `normal.rs` back to **1** — exactly the pre-existing
  baselines. This plan added **zero** new fmt drift and fixed none of the pre-existing drift.
- **Files modified:** `src/ui/screens/detail.rs`, `src/ui/screens/normal.rs`.

No other deviations. No architectural (Rule 4) decisions arose, no authentication gates were hit,
and no checkpoint was reached — the plan was `autonomous: true` with no checkpoint tasks.

## Scope Fence Compliance

- UIFIX-01 (pause badge) and UIFIX-03 (markdown edit key) — untouched.
- WR-01, WR-03, WR-04, WR-05 — untouched.
- `compact_pipeline`, `alias_badge`, `browse_edit_target`, `footer_spans`, `clamp_scroll`,
  `ViewportMetrics`, `PAGE_SCROLL_LINES`, `ScreenAction`, the `Screen` trait, `ProjectViewCache`
  and `src/ui/screens/mod.rs` — all unchanged. The `detail.rs` diff's first hunk starts at line
  688, well below the lines 20-40 helper block.
- Zero new Cargo dependencies, zero feature-flag changes: `git status --porcelain Cargo.toml
  Cargo.lock` is empty. `TestBackend` was reachable under the existing
  `ratatui = { version = "0.30", features = ["crossterm"] }` manifest entry, as the plan predicted;
  it had simply never been used.
- The five pre-existing all-targets clippy lints — left unfixed, count unchanged.
- STATE.md and ROADMAP.md — not modified (worktree mode; the orchestrator owns those writes).

## Known Stubs

None. No stub, placeholder, TODO, skipped test or unrun `<verify>` was introduced by this plan.
Every `<verify>` block was executed and passed.

## Follow-ups (recorded, not addressed here)

Carried forward from the plan, unchanged:

- **WR-03** — Docs and Archive viewers keep showing pre-edit content after `$EDITOR` exits.
- **WR-04** — the browse-root fence skips its check when the root is unset (fails open).
- **WR-05** — the archived read-only guard matches a forward-slash path segment, so it does not
  fire on Windows.
- **WR-01** — the three badge glyphs have unequal terminal display width; the guard test asserts
  character count, not display width.
- **GD-02 residue** — the `<60` tier clips the pipeline below width 44. Decided as acceptable.
- **IN-08** — the total-line count is cast to `u16` without a checked conversion, so a document
  over 65,535 rendered lines wraps and now also caps the scroll clamp.
- **IN-09** — repository-wide `cargo fmt` drift (96 diffs in `detail.rs`, 1 in `normal.rs`, plus
  others), and the five pre-existing all-targets clippy lints.

## Self-Check: PASSED

- `14-04-SUMMARY.md` present at `.planning/phases/14-ui-fixes/14-04-SUMMARY.md`.
- All four commits present on `worktree-agent-a8ec79863ef24a953`, each touching only its intended
  files: `e27ab10` (`normal.rs`), `e1ac518` (`detail.rs`), `0e4bcdf` (`detail.rs`), `c7aae1b`
  (SUMMARY only).
- Working tree clean; no deletions in any commit.
- STATE.md and ROADMAP.md untouched, as required in worktree mode.
</content>
