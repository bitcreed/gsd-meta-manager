---
phase: 18-driver-tab-live-watch-durable-injection
plan: 06
subsystem: ui
tags: [rust, ratatui, dashboard, badges, sort, filter, obs-02, obs-07]

requires:
  - phase: 18-driver-tab-live-watch-durable-injection
    plan: 04
    provides: "`needs_human` / `attention_rank`, `AppContext::needs_human_for`, `SortMode`, the sort-mode-aware `sorted_aliases`, `FilterColumn::NeedsHuman` + the `/h` suffix, and the alias-pinned selection in `recompute_filtered_aliases`"
  - phase: 17-supervisor-detach-kill-switch-dry-run-opt-in-gate
    provides: "`ObservedRun` / `Liveness` / `is_live()` — the tri-state the driven badge is wired to"
provides:
  - "`AliasBadge { glyph: &'static str, color: Color, modifier: Modifier }` — the widened badge return type"
  - "`BADGE_DRIVEN` / `BADGE_NEEDS_HUMAN` / `BADGE_PAUSED` / `BADGE_EXTERNAL_JOB` / `BADGE_SESSION` — the five glyphs as `\\u{…}` `&'static str` constants"
  - "`BadgeInputs` — the five named facts that select a badge"
  - "`alias_badge(BadgeInputs) -> Option<AliasBadge>` — five ranks, still one `Option`"
  - "`row_badge(&AppContext, &str)` — the production wiring, lifted out of the render closure so tests reach it"
  - "The `s` sort toggle on the dashboard and its `Sort: attention first` / `Sort: alphabetical` confirmation"
  - "`summary_spans` + `SORT_INDICATOR` — the `sort: attention` summary-row indicator, rendered only in the non-default mode"
affects: [18-09, 18-11]

tech-stack:
  added: []
  patterns:
    - "A named `Default`-able input struct instead of a five-`bool` parameter list, so each fact is self-labelling at the call site and in every test"
    - "The production wiring lifted out of a render closure into a named function, so tests exercise *which state feeds which input* without rendering a frame"
    - "A type used as a mechanism: `&'static str` is what enforces 'never derived from file content', and the single `Option` return is what enforces 'at most one badge'"
    - "An indicator with no `else` branch — the default state renders nothing, and the doc says so at the code"

key-files:
  created: []
  modified:
    - src/ui/screens/normal.rs

key-decisions:
  - "`BadgeInputs` rather than five positional bools — the plan permitted either, and five bools is a five-way ordering hazard at every call site"
  - "Rank 3 (the cyan pause badge) is production-superseded by rank 2 and the arm is kept anyway, because `alias_badge` is pure over its inputs and a caller passing a narrower predicate still needs the shipped v1.4 badge"
  - "`row_badge` exists so 'an empty registry and a never-driven project yield no badge' is a test of the real wiring rather than of a hand-assembled input struct"
  - "`summary_spans` split out of `render_normal_footer` so the indicator is assertable on spans — a missing indicator and a clipped one are indistinguishable in a rendered buffer"
  - "No `[s]ort` footer hint was added: help.rs is 18-11's file and the footer's right half is already at its width budget at 80 columns"

metrics:
  duration: 17min
  tasks: 2
  files-modified: 1
  tests-added: 15
  commits: 2

completed: 2026-07-29
status: complete
---

# Phase 18 Plan 06: Dashboard Badges, Sort & Filter Summary

**A glance at the dashboard now answers "is anything driving my repos, and which
of them need me?" — with at most one badge per row, no colour-only meaning, and
alphabetical still the default order.**

## Performance

- **Duration:** 17 min
- **Started:** 2026-07-29T17:02:13Z (worktree base)
- **Completed:** 2026-07-29T17:19:08Z
- **Tasks:** 2
- **Files modified:** 1

## Accomplishments

- **Five badge ranks, and the two mechanisms that make "at most one" true are
  intact and now *documented as mechanisms*.** `alias_badge` still returns a
  single `Option`, and the glyph field is still `&'static str`. The doc no longer
  leaves either as a stylistic reading: the `Option` is what keeps the Alias
  column aligned, and the `&'static str` is what makes it structurally impossible
  for a HANDOFF body or an agent's prose summary to reach a dashboard row
  (T-18-32). Widening the glyph to `String` would silently delete that guarantee,
  so the type says why it is the type it is.
- **The driven badge is wired to liveness, not to the presence of a run.**
  `row_badge` calls `ObservedRun::is_live()` and never re-derives it, so the
  tri-state `Liveness` keeps owning the distinction CR-05 was filed for.
  `the_driven_badge_is_wired_to_liveness_not_to_the_mere_presence_of_a_run`
  asserts that `Dead` **and** `Unknown` both fail to light it — the second half
  matters because "I do not know" being rendered as "an agent is driving your
  repo" is exactly the spoofing failure T-18-33 names.
- **The needs-human badge consumes 18-04's predicate rather than growing a second
  one.** `grep -c Parked src/ui/screens/normal.rs` returns **0**: there is no
  branch anywhere in this file that lights a badge from a `Parked` record, which
  is what keeps the badge honest before Phase 20 ships the producer (D-14).
  One predicate feeds the badge, the filter and the sort, so the user cannot get
  three different answers to the same question.
- **No meaning rides on colour alone.** Five distinct glyph shapes, five distinct
  colours, asserted as two separate dedup checks so a future swap that collapses
  either dimension fails a test. Only the two new badges carry BOLD, which lifts
  them above the three shipped ones without needing a second cell.
- **The alignment claim is checked where it is observable.** A render-level test
  paints an unbadged row, a driven row and a needs-human row through the real
  `dashboard_table` and asserts every later column starts at the same buffer
  column — using **char** offsets, because a badged row is no longer pure ASCII
  and `\u{25C6}` is three bytes wide but one cell wide.
- **`STATUS_COLUMN_MIN_CELLS` is still 13.** The badge lives in the Alias cell,
  and nothing here touched the Status floor (UIFIX-02 / CR-01).
- **One key, one indicator, no new screen.** `s` toggles the sort mode,
  recomputes (which both applies the order and re-pins the selection to the
  alias), and announces the new mode. `sort: attention` renders in Cyan only in
  the non-default mode; the default renders nothing, and the doc records the
  reason so nobody adds the `else` branch back.
- **The dangerous half of `AttentionFirst` is asserted at the dashboard.**
  `an_attention_sort_keeps_the_cursor_on_the_same_alias_when_a_run_finishes`
  drives the real key handler, lets a run finish, and asserts the cursor lands on
  index 2 — the index the *old* index-pinned behaviour would have left pointing
  at `alpha`, a project the user never chose.
- **No new dependency, no clippy regression.** `Cargo.toml`/`Cargo.lock`
  untouched; `cargo clippy --all-targets` still reports exactly the 5 pre-existing
  lints. Test count 612 → 627.

## Task Commits

1. **Task 1: Two new dashboard badges, at most one per row (D-24 / OBS-02)** — `58731c6` (feat)
2. **Task 2: The sort toggle, its indicator, and the needs-human filter (D-25 / OBS-07)** — `74876dd` (feat)

## Files Created/Modified

**Modified**
- `src/ui/screens/normal.rs` — the five `BADGE_*` glyph constants, `AliasBadge`,
  `BadgeInputs`, the widened `alias_badge`, `row_badge`, the simplified alias-cell
  construction in `render_main`, the `s` key arm, `SORT_INDICATOR`,
  `summary_spans` (split out of `render_normal_footer`), and 15 new inline tests
  plus the six pre-existing badge tests rewritten for the new signature.

## Decisions Made

- **`BadgeInputs` rather than five positional `bool`s.** The plan and PATTERNS §8
  both permitted either. Five same-typed positional parameters is a five-way
  ordering hazard that the compiler cannot catch, at every call site and in every
  one of the fifteen tests; `Default` + struct-update syntax lets a test state one
  condition and mean exactly that one. It also sidesteps the
  `field_reassign_with_default` class of lint that cost 18-04 a fix.
- **Rank 3 is production-superseded, the arm stays, and the supersession is
  asserted rather than discovered.** `needs_human` counts `state.paused` among its
  four sources (D-14), so on the live dashboard a paused project renders the
  **red flag** (rank 2), not the cyan pause bars (rank 3). That is what the
  UI-SPEC's priority table asks for and it is the stronger signal OBS-02 wants,
  but it does change what a paused row has looked like since v1.4 — so
  `test_paused_project_shows_pause_badge_end_to_end` now asserts *both* the
  isolated rank-3 selection and the rank-2 outcome the real wiring produces. The
  rank-3 arm is kept because `alias_badge` is pure over its inputs: a caller
  passing a narrower predicate still gets the shipped badge, and deleting the arm
  would make that unavailable without anyone noticing.
- **`row_badge` exists so the zero-projects and never-driven cases test the real
  wiring.** With the badge selection inline in the render closure, the only way to
  reach it is to paint a frame, and the must-have truth is about *which state
  feeds which input* — precisely the layer a frame test cannot isolate. This is
  the same lesson `detail.rs:4931-4938` records for the scroll handlers.
- **`summary_spans` split out of `render_normal_footer`.** The indicator is
  asserted on the built spans, per the plan. In a rendered buffer a missing
  indicator and one clipped off the end of the row are the same pixels, so a
  buffer-level test would pass for the wrong reason at the widths where it
  matters most.
- **The recompute happens before the status message in the `s` arm.** Ordering is
  not incidental: the recompute is what re-pins the selection, and a status
  message written first would be truthful about a state the rows had not reached
  yet.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] One new `--all-targets` clippy lint introduced by this plan's own test, fixed before commit**
- **Found during:** Task 1
- **Issue:** `assert!(ctx.observed_runs.get("alpha").is_none())` took
  `cargo clippy --all-targets` from **5 warnings to 6**
  (`unnecessary use of get(..).is_none()`). The phase gate is that the count must
  not grow, and the ordinary `cargo clippy -- -D warnings` gate does **not** catch
  it because it does not build test targets — the same vacuous-pass trap 18-04
  hit from the other direction.
- **Fix:** Rewritten as `!ctx.observed_runs.contains_key("alpha")`.
- **Verification:**
  `rtk proxy sh -c "cargo clippy --all-targets 2>&1 | grep '^warning: ' | grep -v generated | wc -l"`
  reports **5**, unchanged.
- **Committed in:** `58731c6`

**2. [Rule 3 - Blocking] A sixth full-field `AppContext` fixture, in this file's own test module**
- **Found during:** Task 1
- **Issue:** `row_badge`, the `s` toggle and the filter tests all need a real
  `AppContext` with registered projects. 18-04 added `ctx_with_aliases` to
  `screens/mod.rs`'s `mod tests`, but that module is a **sibling** of
  `screens::normal`, not an ancestor, so its private fixture is unreachable from
  here — and `src/ui/screens/mod.rs` is explicitly outside this plan's file scope.
- **Fix:** `ctx_with_aliases` in `normal.rs`'s `mod tests`, with a doc recording
  that it is the sixth such site and why it could not be borrowed.
- **Files modified:** `src/ui/screens/normal.rs`
- **Committed in:** `58731c6`

**3. [Rule 1 - Bug] The alignment test's offsets had to be char-based, not byte-based**
- **Found during:** Task 1
- **Issue:** `render_dashboard_interior`'s doc states that its returned strings
  are pure ASCII "so byte offsets equal column offsets". A badged row breaks that
  premise: `\u{25C6}` is three bytes and one cell, so `str::find` would have
  reported a column two greater than the truth and the alignment assertion would
  have failed for a reason that has nothing to do with alignment.
- **Fix:** A local `column_of` helper that converts the byte offset to a char
  count, with a comment naming the premise it is restoring and why every badge
  glyph being East-Asian-Width narrow is what makes one char equal one cell.
- **Committed in:** `58731c6`

### Judgement Calls (recorded per the plan's autonomy clause)

**4. No `[s]ort` footer hint was added.** The plan's artifact list names the key
arm, the indicator and the status copy, and does not name a footer hint. The
footer's right half already renders 7 hints and is laid out with
`Constraint::Length(right_len + 1)`, so an eighth would compete with the summary
row at 80 columns — the default terminal. Keybinding discoverability lives in
`help.rs`, which **18-11 owns and this plan must not edit**. Flagged here so 18-11
can decide deliberately rather than inherit the omission silently.

**5. The `//h` form is stored as `"/h"`, and the test types it rather than
asserting the spec's literal.** 18-04 deviation 6 established this: the search
prompt supplies its own leading `/` and never stores it. The dashboard tests drive
the real key handler through a `search` helper whose doc records the discrepancy,
so a reader arriving from the UI-SPEC's `//h` finds the explanation at the code
rather than concluding the test is wrong.

---

**Total deviations:** 3 auto-fixed (1 blocking, 2 bugs), 2 recorded judgement calls
**Impact on plan:** None on what shipped. Every artifact the plan names exists.

## Issues Encountered

- **The rank-2 / rank-3 overlap is a real behaviour change that the plan's task
  list does not call out.** Because `needs_human` includes `state.paused`, the
  cyan pause badge shipped in v1.4 no longer appears on the live dashboard. This
  is what the UI-SPEC's priority table specifies and it is an improvement, but it
  is the kind of change a user notices and a plan reads past. It is documented in
  `alias_badge`'s doc comment, asserted in the end-to-end tracer, and recorded
  under Decisions Made — worth a line in the phase's UAT.

## Verification

| Gate | Result |
|---|---|
| `cargo build` | pass |
| `cargo test` | **627 passed, 0 failed** (baseline 612; 526 lib + 101 across 18 integration binaries) |
| `cargo clippy -- -D warnings` | pass |
| `rtk proxy … cargo clippy --all-targets … \| wc -l` | **5** — the pre-existing count, unchanged |
| `git diff --stat Cargo.toml Cargo.lock` | empty — no new dependency |
| `rtk proxy cargo test --lib ui::screens::` | 81 passed, 0 failed |

### Acceptance criteria

| Criterion | Evidence |
|---|---|
| `BADGE_DRIVEN` / `BADGE_NEEDS_HUMAN` are `&str` constants with `\u{` escapes | `normal.rs:77`, `normal.rs:83` |
| `fn alias_badge` returns a single `Option<…>` | `normal.rs:169` — `-> Option<AliasBadge>` |
| `STATUS_COLUMN_MIN_CELLS` still `13` | `normal.rs:297` |
| `needs_human` consumed, not re-derived | 19 matches in the file; `row_badge` calls `ctx.needs_human_for` |
| No branch lights the badge from a `Parked` record | `grep -c Parked` → **0** |
| `KeyCode::Char('s')` once in the dashboard handler | `normal.rs:472`; every other match is a test key press |
| `sort: attention` present, no default-mode label | `SORT_INDICATOR` at `normal.rs:792`; `the_sort_indicator_renders_only_in_the_non_default_mode` asserts the absence |
| `an_attention_sort_keeps_the_cursor_on_the_same_alias_when_a_run_finishes` | passes |

### Threat register

| Threat ID | Disposition | Evidence |
|---|---|---|
| T-18-32 (badge glyph derived from file content) | **mitigated** | `AliasBadge.glyph` is `&'static str`; all five values are `\u{…}` constants; `test_badge_is_never_two_glyphs` sweeps the full 2^5 input space and asserts every result comes from the documented table |
| T-18-33 (a badge misrepresenting whether an agent is driving) | **mitigated** | rank 1 is driven-**and**-live via `ObservedRun::is_live()`; `Dead` and `Unknown` are both asserted not to light it |
| T-18-34 (per-frame badge/sort cost) | **accepted** | Both are pure functions over already-loaded state with no I/O; recorded, not measured, per the register |
| T-18-35 (filter text echoed to a log or a path) | **mitigated** | The filter is compared in memory only; this plan added no log line and derives no path from it |
| T-18-36 (package-manager installs) | **accepted** | Zero dependencies added; `git diff` on `Cargo.toml`/`Cargo.lock` is empty |

## Known Stubs

None. Every badge rank, the sort toggle and the filter are wired to producers
that exist today. The one rank whose producer is a later phase's —
`JournalEvent::Parked` — is deliberately **not** referenced anywhere in this file
(D-14): a badge that can never light is worse than no badge.

## User Setup Required

None.

## Next Phase Readiness

- **18-09** (the Driver tab) can reuse `AliasBadge`, the five glyph constants and
  the `\u{…}`/`&'static str` discipline for the run-list glyphs; UI-SPEC Surface 8
  names `◆ ● ○ ✗ ■` for that list and rank 1's `◆` is already defined here.
- **18-11** (help/keybindings) must document `s` — `Sort: attention first` /
  `Sort: alphabetical` — and the `/h` filter suffix. See judgement call 4: no
  footer hint was added, deliberately.
- **Phase 20**, when it ships the `Parked` producer, extends `needs_human` in
  `screens/mod.rs` by one argument. Nothing in `normal.rs` changes: the badge
  already asks the predicate rather than the record.

## Self-Check: PASSED

`src/ui/screens/normal.rs` present on disk; both commits present in `git log`
(`58731c6`, `74876dd`); `git diff --diff-filter=D` empty for both — no file was
deleted; working tree clean apart from this summary.

---
*Phase: 18-driver-tab-live-watch-durable-injection*
*Completed: 2026-07-29*
