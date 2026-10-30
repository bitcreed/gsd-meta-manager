---
phase: 18-driver-tab-live-watch-durable-injection
plan: 04
subsystem: ui
tags: [rust, ratatui, ring-buffer, vecdeque, sanitiser, ansi-escape, sort, filter, action-types]

requires:
  - phase: 16-run-journal-state-substrate
    provides: "`JournalEvent`, `RunRecord.outcome`, the `journal_cursors` sibling-map doc this plan's map copies, and `App::prune_driver_maps` as the pruning site"
  - phase: 17-supervisor-detach-kill-switch-dry-run-opt-in-gate
    provides: "`ObservedRun`/`Liveness`/`is_live()`, `StopOutcome`, `App::start_driver_run`'s `goal: Option<&str>` parameter, and WR-15's reproduction"
  - phase: 18-driver-tab-live-watch-durable-injection
    plan: 01
    provides: "`journal::inbox::InboxMessage`, the type `ProjectViewCache.driver_inbox` and `Action::DriverRunsListed` carry"
provides:
  - "`DriverOutput` — the repo's first bounded in-memory collection, a private `VecDeque` with a drop counter and a record-truncation flag"
  - "`sanitize_render_line` / `sanitize_record_lines` — the single append-time gate every string from disk passes through, with an unconditional `ESC` strip"
  - "`DRIVER_OUTPUT_RING_LINES` / `_LINE_CELLS` / `_RECORD_MAX_LINES`"
  - "`needs_human` — the pure four-source predicate (D-14) — and `attention_rank`"
  - "`SortMode { Alphabetical (default), AttentionFirst }` and a sort-mode-aware `sorted_aliases`"
  - "`AppContext.driver_output` / `.sort_mode`; `ProjectViewCache.driver_selected_run` / `_scroll_offset` / `_follow` / `_inbox`"
  - "`AppContext::needs_human_for` and alias-pinned selection in `recompute_filtered_aliases`"
  - "`FilterColumn::NeedsHuman` and the `/h` suffix in `parse_filter`"
  - "`StopDisposition { RunGone, MayStillBeLive }` plus `From<&StopOutcome>` (WR-15/D-29)"
  - "`Action::DriverInjectRequested` / `DriverInjectWritten` / `DriverRunsListed` / `DriverDryRunLoaded`, and the widened `DriverStopped` / `DriverStartRequested`"
affects: [18-05, 18-07, 18-09, 18-10]

tech-stack:
  added: []
  patterns:
    - "Bounded ring buffer with a private inner collection, so the cap cannot be bypassed by a caller pushing directly"
    - "Cap asymmetry from `journal/writer.rs:97-118`: the drop counter is the mechanism, never a per-drop log line"
    - "One shared append-time sanitiser for every untrusted string, applied before the value enters any buffer"
    - "Selection pinned to the alias rather than the row index, so a re-rank cannot move a row out from under the cursor"

key-files:
  created: []
  modified:
    - src/ui/screens/mod.rs
    - src/action.rs
    - src/app.rs
    - src/ui/screens/driver_confirm.rs
    - src/ui/screens/detail.rs
    - src/ui/screens/delete_confirm.rs

key-decisions:
  - "`record_truncated` is sticky rather than tracking only the last record: a flag an ordinary later record clears loses the signal, and a pane that silently loses lines is the named looks-done-but-isn't failure"
  - "`ESC` is stripped before the C0 replacement runs, so the introducer can never survive as a middle dot and be re-assembled"
  - "Truncation gives the ellipsis a cell of its own, so a truncated line is exactly `DRIVER_OUTPUT_LINE_CELLS` chars rather than one over"
  - "`needs_human` returns `false` on the stale-outcome arm when a run is live, but the paused / external-job / parked arms still fire — those are facts about the project, not about a finished run"
  - "`RunOutcome::Killed` is deliberately not a needs-human source: a run the user stopped is not a run waiting on them"
  - "`AttentionFirst` re-sorts the already-alphabetical vector with `sort_by_key`, whose stability is what makes 'alphabetical within rank' true without a second comparator"
  - "The four new `Action` variants got explicit match arms in `app.rs` rather than a catch-all, so a future variant cannot be silently ignored"
  - "`Action::DriverRunsListed` ships without its `runs` field because `journal::RunSummary` is a wave-2 sibling's deliverable — see deviation 1"

metrics:
  duration: 28min
  tasks: 3
  files-modified: 6
  tests-added: 18
  commits: 3

completed: 2026-07-29
status: complete
---

# Phase 18 Plan 04: Driver State Layer, Sanitiser & Message Types Summary

**Live driver output now has a bounded, drop-counting home that is a sibling map on `AppContext` and never a field on `PartialEq`-deriving `ProjectState`, and no byte from disk reaches it without passing through one shared sanitiser that strips `ESC` unconditionally.**

## Performance

- **Duration:** 28 min
- **Started:** 2026-07-29T23:15:00Z
- **Completed:** 2026-07-29T23:43:00Z
- **Tasks:** 3
- **Files modified:** 6

## Accomplishments

- **The repo's first bounded in-memory collection.** `grep -rn VecDeque src/`
  returned nothing before this plan. `DriverOutput` wraps a **private**
  `VecDeque<DriverOutputLine>` behind `push_record`, so the cap is enforced at
  the only entry point and a caller cannot bypass it — which is precisely the
  "a `Vec` that grows" failure 18-CONTEXT enumerates. Three named `pub const`s
  carry the `main_loop.rs:41-49` register: what the number bounds, and that it is
  a defensible starting value with no tuning data behind it, named so tuning is a
  one-line change. Overflow is counted, never logged per drop — the
  `journal/writer.rs:97-118` cap asymmetry, where the counter *is* the mechanism.
- **The sanitiser, which is the phase's highest-value security control.**
  `sanitize_render_line` strips `ESC` unconditionally before anything else runs,
  replaces every other C0 control and `DEL` with a middle dot, expands tabs, and
  truncates by **`char`** count. The `ESC` rule is the one that matters: without
  it, agent prose lands in `ExecEvent.text` / `Diagnostic.detail` /
  `RunRecord.goal` / `gsd_command` and can emit ANSI/OSC sequences that repaint
  the screen, forge a status line, move the cursor, or set the window title. The
  test feeds it an SGR, a clear-screen and an OSC window-title set and asserts no
  `ESC` survives — while asserting the prose itself is still shown, so the rule
  cannot pass by deleting everything.
- **`needs_human` answers from evidence that exists today (D-14).** All four
  arms — `paused`, `external_job_waiting`, the four terminal outcomes, and the
  forward-compatible `parked` — have a unit test, including the two a running
  system reaches rarely. `RunOutcome::Killed` is deliberately excluded. The doc
  states the fence in bold: **do not wire this to `JournalEvent::Parked` alone**,
  because nothing emits that until Phase 20, the badge would never light, and no
  test would catch it because the test would emit the event by hand.
- **Alphabetical is still the default, and `AttentionFirst` is genuinely
  stable.** `sorted_aliases` sorts alphabetically in *both* modes and then
  re-sorts by rank with `sort_by_key` — whose stability is what makes
  "alphabetical within rank" true with no second comparator and no `sort_by`,
  so clippy 1.97's `unnecessary_sort_by` fix from commit 1984a6c stays fixed.
- **The selection is now pinned to the alias, not the index.** Without this,
  `AttentionFirst` would be actively dangerous: a run finishing re-ranks its
  project, the row moves, and the next keystroke acts on a project the user never
  chose. `recompute_filtered_aliases` re-derives `table_state` from the
  previously selected alias, clamps into range when it is gone, and selects
  `None` on an empty list. All 556 pre-existing tests still pass with it in.
- **`Action` carries a portable stop disposition and four new plain-data
  variants, still `Clone`.** `grep -nE 'JoinHandle|std::fs::File|Sender<'
  src/action.rs` returns nothing.
- **No new dependency, no clippy regression.** `Cargo.toml`/`Cargo.lock`
  untouched; `cargo clippy --all-targets` still reports exactly the 5 pre-existing
  lints. Test count 556 → 574.

## Task Commits

1. **Task 1: The bounded ring buffer and the render-line sanitiser** — `acf3e26` (feat)
2. **Task 2: `AppContext`/`ProjectViewCache` fields, `needs_human`, the sort mode** — `8094dd3` (feat)
3. **Task 3: The message types the controller will dispatch** — `d0bc309` (feat)

## Files Created/Modified

**Modified**
- `src/ui/screens/mod.rs` — `DRIVER_OUTPUT_RING_LINES` / `_LINE_CELLS` /
  `_RECORD_MAX_LINES`, `ESC` / `CONTROL_REPLACEMENT` / `ELLIPSIS` / `TAB_WIDTH`,
  `DriverLineKind`, `DriverOutputLine`, `DriverOutput`, `sanitize_render_line`,
  `sanitize_record_lines`, `SortMode`, `needs_human`, `attention_rank`, the two
  new `AppContext` fields and four new `ProjectViewCache` fields,
  `AppContext::needs_human_for` / `attention_rank_for` / `pin_selection_to_alias`,
  the sort-mode-aware `sorted_aliases`, the `NeedsHuman` filter arm, and a
  16-case inline `mod tests`.
- `src/action.rs` — `StopDisposition` + `From<&StopOutcome>`, the widened
  `DriverStopped` and `DriverStartRequested`, the four new variants, and a
  2-case inline `mod tests`.
- `src/app.rs` — `FilterColumn::NeedsHuman`, the `/h` arm in `parse_filter`, the
  `goal` threading into `start_driver_run`, the disposition derived at the stop
  seam, four new exhaustive match arms, and the two new `AppContext` fields at
  the production construction site.
- `src/ui/screens/driver_confirm.rs` — the full-field fixture, the
  `DriverStartRequested { goal: None }` sender, and the widened destructure in
  its test.
- `src/ui/screens/detail.rs` — the `test_ctx` fixture.
- `src/ui/screens/delete_confirm.rs` — the third full-field fixture (deviation 2).

## Decisions Made

- **`record_truncated` is sticky.** The plan fixes the field on `DriverOutput`
  rather than on the line, so it cannot be positional. Between "the last record
  was cut" and "any record was cut", the second is the only one that does not
  silently lose the signal when an ordinary record follows a giant one.
- **`ESC` is stripped before the C0 replacement, not as part of it.** Order
  matters: replacing `ESC` with a middle dot first and stripping later would
  leave nothing to strip, and a sequence whose introducer became a visible glyph
  is not a sequence — but the rule as written is also simply what the UI-SPEC
  says, and the doc records the ordering so a later refactor cannot reorder it
  by accident.
- **The ellipsis gets a cell of its own.** A truncated line is exactly
  `DRIVER_OUTPUT_LINE_CELLS` chars, not one over, so the cap means what it says
  when a renderer budgets against it.
- **A live run suppresses only the stale-outcome arm of `needs_human`.** This is
  what makes the `run: Option<&ObservedRun>` parameter carry its weight: an
  earlier run's failure is not a summons while something is actively driving,
  but a HANDOFF is a fact about the project and a live run must not hide it.
- **`AppContext::needs_human_for` passes `last_outcome: None` today, and that is
  a seam rather than a stub.** Unlike `JournalEvent::Parked`, that arm's
  **producer exists on disk right now** (`RunRecord.outcome`, written by
  `driver::run::outcome_label`); only the reader is a later plan's. The typed
  label table in `run.rs` is module-private and belongs to a sibling plan's file,
  so duplicating it here to bridge the gap would have created a second
  vocabulary that drifts silently — declined.
- **Four explicit `Action` match arms in `app.rs`, not a catch-all.** A `_ =>`
  would let a future variant be added and silently ignored, which is the exact
  opposite of what exhaustive matching on this enum is for. The arms log by
  **kind and count only** — never `text`, never a message body — per the phase's
  log-hygiene pattern.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `Action::DriverRunsListed.runs` and `ProjectViewCache.driver_runs` could not land in this tree**
- **Found during:** Task 2 (planning the field list), confirmed in Task 3
- **Issue:** The plan specifies `driver_runs: Vec<journal::RunSummary>` and
  `DriverRunsListed { alias, runs: Vec<journal::RunSummary>, inbox }`.
  **`journal::RunSummary` does not exist.** It is plan **18-03**'s named
  deliverable (`18-03-PLAN.md:91`), and 18-03 is a **wave-2 sibling of 18-04**
  (`wave: 2`, `depends_on: ["18-01"]` — the same wave, executing concurrently in
  a separate worktree). Writing a field naming that type would have made this
  worktree fail `cargo build`, and with it every `<verify>` and every acceptance
  criterion in all three tasks — not just the ones about the run list.
- **Fix:** Both `runs`-typed members are omitted. `driver_inbox:
  Vec<journal::inbox::InboxMessage>` **did** land, because `InboxMessage` is
  18-01's and exists here. The omission is recorded as a loud doc comment on
  `Action::DriverRunsListed` naming the exact missing field, why it is missing,
  and who adds it: **plan 18-05**, which runs in wave 3 with both 18-03 and
  18-04 merged, already owns the `DriverRunsListed` handler
  (`18-05-PLAN.md:101,199,227`) and already reads `src/action.rs` as landed by
  18-04. Adding one field to an enum variant and one to a `#[derive(Default)]`
  struct is a two-line change there.
- **Files modified:** `src/action.rs`, `src/ui/screens/mod.rs`
- **Verification:** `cargo build && cargo test && cargo clippy -- -D warnings`
  all pass; the seam is a doc comment a grep for `RunSummary` finds immediately.
- **Committed in:** `d0bc309`

**2. [Rule 3 - Blocking] A third full-field `AppContext` fixture exists, outside this plan's `files_modified`**
- **Found during:** Task 2
- **Issue:** The plan (and the execution brief) name two fixtures that enumerate
  every `AppContext` field by name and therefore break on any addition:
  `driver_confirm.rs::ctx_with_project` and `detail.rs::test_ctx`. There is a
  **third**: `src/ui/screens/delete_confirm.rs:201`. It is not in
  `files_modified`, but it is a hard compile break.
- **Fix:** Added the two new fields there too. Two lines, no behaviour change.
- **Files modified:** `src/ui/screens/delete_confirm.rs`
- **Verification:** `rtk proxy cargo test --lib` — all 479 lib tests pass,
  including `delete_confirm`'s own five.
- **Committed in:** `8094dd3`

**3. [Rule 3 - Blocking] The four new `Action` variants required match arms in `app.rs`**
- **Found during:** Task 3
- **Issue:** `App::update` matches `Action` exhaustively with no catch-all, so
  adding four variants is a compile error. Their real handlers are plan 18-05's
  named deliverable.
- **Fix:** Four arms that log by kind and count and do nothing else, each
  carrying a comment naming 18-05 as the owner and stating why a catch-all was
  rejected. No message body is logged, per the phase's log-hygiene rule. Listed
  under **Known Stubs** below.
- **Files modified:** `src/app.rs`
- **Committed in:** `d0bc309`

**4. [Rule 1 - Bug] Three new `clippy::field_reassign_with_default` lints introduced by this plan's own tests, fixed before commit**
- **Found during:** Task 2
- **Issue:** `let mut s = ProjectState::default(); s.paused = true;` in three
  test sites took `cargo clippy --all-targets` from **5 warnings to 8**. The
  phase's own gate is that the count must not grow, and the ordinary
  `cargo clippy -- -D warnings` gate does **not** catch it because it does not
  build test targets — exactly the kind of vacuous pass the phase context warns
  about.
- **Fix:** Rewritten as struct-update syntax
  (`ProjectState { paused: true, ..Default::default() }`).
- **Verification:**
  `rtk proxy sh -c "cargo clippy --all-targets 2>&1 | grep '^warning: ' | grep -v generated | wc -l"`
  reports **5**, unchanged.
- **Committed in:** `8094dd3`

**5. [Rule 3 - Blocking] A fifth full-field `AppContext` construction, in this module's own test fixture**
- **Found during:** Task 2
- **Issue:** The sort and filter tests need a real `AppContext` with registered
  projects. No existing fixture is reachable from `screens/mod.rs`'s test module
  without importing another screen's private test helper.
- **Fix:** `ctx_with_aliases` in `src/ui/screens/mod.rs`'s `mod tests`. Its doc
  records that it is the fifth such site and why it belongs here rather than
  being borrowed: the behaviour under test is this module's.
- **Files modified:** `src/ui/screens/mod.rs`
- **Committed in:** `8094dd3`

**6. [Rule 2 - Missing Critical] The `//h` form in the spec is `/h` in `filter_text`**
- **Found during:** Task 2
- **Issue:** UI-SPEC Surface 7 and the plan both write the all-rows filter as
  `//h`, asserting it "falls out of the existing grammar with no special case".
  It does not: `"//h".strip_suffix("/h")` yields the term `"/"`, which matches
  almost nothing. The **search prompt supplies its own leading `/`** —
  `normal.rs:284-291` clears `filter_text` on `/` and never stores it — so the
  stored form is `"/h"`, which does yield an empty term and does fall out of the
  grammar with no special case.
- **Fix:** No code change; the grammar was already right. The discrepancy is
  recorded in `parse_filter`'s new doc comment and asserted directly in
  `the_needs_human_filter_parses_with_and_without_a_term`, so a later reader
  hitting the spec's `//h` finds the explanation at the code.
- **Files modified:** `src/app.rs`, `src/ui/screens/mod.rs`
- **Committed in:** `8094dd3`

---

**Total deviations:** 6 auto-fixed (4 blocking, 1 bug, 1 missing critical)
**Impact on plan:** Deviation 1 is the only one that changes what shipped, and
it removes two members rather than adding anything — with the owner, the wave,
and the exact two-line remedy named at the code. No scope creep.

## Issues Encountered

- **`journal::RunSummary` is a same-wave dependency the plan set did not
  account for.** 18-04's frontmatter says `depends_on: ["18-01"]`, but its
  artifact list names a type 18-03 creates in the same wave. This is a planning
  gap worth carrying forward: a plan's `depends_on` should cover every type it
  names, and a wave-2 plan naming a wave-2 sibling's type cannot compile in
  isolation. See deviation 1 for the resolution.
- **`cargo clippy -- -D warnings` passes on lints that
  `cargo clippy --all-targets` catches.** The project gate does not build test
  targets, so three test-only lints slipped past it (deviation 4). Every count in
  this summary was taken through `rtk proxy`, because plain `cargo` output is
  filtered by the `rtk` summarising wrapper and a criterion that greps for
  `warning:` or `test result:` without it passes **vacuously**.

## Verification

| Gate | Result |
|---|---|
| `cargo build` | pass |
| `cargo test` | **574 passed, 0 failed** (baseline 556; 479 lib + 95 across 18 integration binaries) |
| `cargo clippy -- -D warnings` | pass |
| `rtk proxy … cargo clippy --all-targets … \| wc -l` | **5** — the pre-existing count, unchanged |
| `git diff --stat Cargo.toml Cargo.lock` | empty — no new dependency |
| `rg -c 'VecDeque' src/ui/screens/mod.rs` | 3 — the repo's first bounded collection exists |
| `rg -n 'sort_by\(' src/ui/screens/mod.rs` | no match — the 1984a6c clippy fix is preserved |
| `rg -nE 'JoinHandle\|std::fs::File\|Sender<' src/action.rs` | no match — no handle entered a message type |

### Threat register

| Threat ID | Disposition | Evidence |
|---|---|---|
| T-18-19 (ANSI/OSC in agent prose) | **mitigated** | `an_escape_bearing_string_never_reaches_the_buffer_with_its_escape` feeds an SGR, a clear-screen and an OSC window-title set through both the function and the append path |
| T-18-20 (unbounded live output) | **mitigated** | private `VecDeque`; `pushing_past_the_ring_cap_keeps_exactly_the_cap_and_counts_every_drop` pushes 2050 lines and asserts both the cap and the drop count |
| T-18-21 (per-alias map growth) | **partially mitigated — 18-05 completes it** | The map is declared with the pruning obligation in its doc; `App::prune_driver_maps` is 18-05's to wire and assert. See Known Stubs |
| T-18-22 (HANDOFF/agent body on a row) | **mitigated** | `needs_human` returns `bool`; no glyph or label anywhere in this plan is derived from file content |
| T-18-23 (handle smuggled into an `Action`) | **mitigated** | grep above, plus `every_new_variant_is_plain_data_so_action_stays_clone` |
| T-18-24 (package-manager installs) | **accepted** | Zero dependencies added; `VecDeque` is std |

## Known Stubs

Each is a declared seam with a named owner, not an oversight. None asserts a
state the mechanism cannot back.

1. **`AppContext.driver_output` is declared but never written or pruned.**
   Nothing appends to it (the `Action::DriverJournalAppended` handler still drops
   `records` — D-20's seam) and `App::prune_driver_maps` does not yet touch it.
   **Owner: 18-05**, which the plan names for both. Until then the map is always
   empty, so it leaks nothing; the obligation is stated in the field's own doc
   because forgetting it reintroduces the Phase 16 leak under a new name.
2. **`ProjectViewCache.driver_selected_run` / `_scroll_offset` / `_follow` /
   `_inbox` are declared but never read.** **Owner: 18-09** (the Driver tab
   render) and **18-05** (the inbox load).
3. **`Action::DriverInjectRequested` / `DriverInjectWritten` /
   `DriverRunsListed` / `DriverDryRunLoaded` have log-only handlers in
   `app.rs`.** **Owner: 18-05.** Nothing constructs them yet either, so no UI
   shows a state derived from them.
4. **`Action::DriverStopped.disposition` is carried but not acted on.** The
   handler still removes from `observed_runs` and `session_spawned_runs`
   unconditionally — i.e. WR-15's behaviour is unchanged, not regressed. The
   fix is **18-05**'s named deliverable, together with the test that reproduces
   the five-second "no run" window; the value is in the message now so that fix
   is two lines rather than a second round of message-type surgery.
5. **`needs_human`'s `last_outcome` arm has no production caller.** Its producer
   exists on disk today (`RunRecord.outcome`) — unlike `JournalEvent::Parked`,
   whose producer is Phase 20's — but the TUI-side reader is **18-09**'s, once
   the run list lands. The arm is fully unit-tested for all four qualifying
   outcomes and the one excluded one.
6. **`Action::DriverRunsListed` is missing its `runs` field.** See deviation 1.
   **Owner: 18-05.**

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

**Ready for wave 3.**

- **18-05** has everything it needs and inherits four named obligations from the
  Known Stubs list: append into `driver_output` from the
  `Action::DriverJournalAppended` handler and set `needs_redraw` (D-20), prune
  `driver_output` in `App::prune_driver_maps` and **assert** the pruning, gate
  the `DriverStopped` map mutation on `disposition == RunGone` (WR-15/D-29), and
  add `runs: Vec<journal::RunSummary>` to `Action::DriverRunsListed` plus
  `driver_runs` to `ProjectViewCache`.
- **18-07** has `Action::DriverInjectRequested` and the widened
  `DriverStartRequested { goal }` waiting; `driver_confirm.rs` currently sends
  `goal: None` with a comment naming 18-07 as what replaces it.
- **18-09** has `DriverOutput::lines()` / `dropped()` / `record_truncated()`,
  the three caps, `DriverLineKind`'s five classes, and the four
  `ProjectViewCache` view-state fields. The UI-SPEC's overflow affordances map
  one-to-one onto `dropped()` and `record_truncated()`.
- **18-10** has `sanitize_render_line` for every header interpolation. Every
  string rendered from disk must go through it — a single un-sanitised path lets
  agent prose repaint the terminal.

**Carried obligations**

- `requirements-completed` is deliberately empty. OBS-04, OBS-07 and STEER-02
  are not reachable by a user until there is a Driver tab to see and a filter key
  to press; this plan builds the state they are computed from, and the
  requirements close later in the phase.
- The negative carry-forward stands and this plan added exactly one map to it:
  **`driver_output` must be pruned in `App::prune_driver_maps`.**

## Self-Check: PASSED

All six modified files present on disk; all three commits present in `git log`
(`acf3e26`, `8094dd3`, `d0bc309`); `git diff --diff-filter=D` empty for each of
the three commits — no file was deleted; working tree clean apart from this
summary.

---
*Phase: 18-driver-tab-live-watch-durable-injection*
*Completed: 2026-07-29*
