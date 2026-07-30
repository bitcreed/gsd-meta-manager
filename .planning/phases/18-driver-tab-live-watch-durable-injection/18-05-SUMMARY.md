---
phase: 18-driver-tab-live-watch-durable-injection
plan: 05
subsystem: app
tags: [rust, controller, spawn-blocking, ring-buffer, tick-gate, wr-15, wr-10, wr-11, path-traversal]

requires:
  - phase: 17-supervisor-detach-kill-switch-dry-run-opt-in-gate
    provides: "`StopOutcome`, `stop_run`, `ObservedRun::is_live()`, `App::start_driver_run`'s `goal: Option<&str>` parameter, `reader::seq_gaps_from`, and WR-10/WR-11/WR-15's reproductions"
  - phase: 18-driver-tab-live-watch-durable-injection
    plan: 01
    provides: "the fallible `journal::run_paths -> Option<RunPaths>` with its `inbox` member, `journal::inbox::{append, tail, cap_chars, InboxMessage}`"
  - phase: 18-driver-tab-live-watch-durable-injection
    plan: 03
    provides: "`journal::RunSummary`, `journal::list_runs` (already newest-first), and the readable `exec_message_text` projection that makes `ExecEvent.text` worth buffering"
  - phase: 18-driver-tab-live-watch-durable-injection
    plan: 04
    provides: "`DriverOutput`/`DriverLineKind`/`push_record`, the sanitiser, `StopDisposition` + `From<&StopOutcome>`, and the four new `Action` variants"
provides:
  - "`Action::DriverStopped` gated on `StopDisposition::RunGone` — WR-15 closed"
  - "`App::schedule_inbox_append` / `schedule_run_list_scan` / `schedule_dry_run_report` — three `spawn_blocking` → `Action` seams (D-28)"
  - "`goal_or_none` — the empty-goal trap shut, so `RunRecord.goal` never records `Some(\"\")`"
  - "`driver_line_for_record` — the pure journal-record → `DriverLineKind` projection"
  - "`journal_gap_line` — the Copywriting Contract's gap diagnostic"
  - "`driver_elapsed_redraw_wanted` — D-21's three-condition redraw gate as a pure predicate"
  - "`DetailSubView::Driver` (index 10) plus the four `detail.rs` arms the compiler forces"
  - "`DetailScreen::NAME` — a compiler-enforced screen-name constant"
  - "`Action::DriverRunsListed.runs` and `ProjectViewCache.driver_runs` — the two-field gap 18-04 deferred"
  - "`driver_output` pruned in `App::prune_driver_maps` — the phase's last carry-forward, discharged"
affects: [18-07, 18-09, 18-10, phase-20]

tech-stack:
  added: []
  patterns:
    - "`spawn_blocking` → `Action` on a cloned sender, with a `let … else` guard on `event_tx` and owned data moved in (S1)"
    - "Refuse visibly, never silently: every early return sets `error_message` and `needs_redraw` (S4)"
    - "Pure predicates extracted from handlers so their truth tables are assertable without a terminal (S6)"
    - "A constant on the type rather than a repeated string literal, so a rename cannot silently disable a comparison"

key-files:
  created: []
  modified:
    - src/app.rs
    - src/ui/screens/detail.rs
    - src/action.rs
    - src/ui/screens/mod.rs

key-decisions:
  - "The four per-outcome WR-15 tests drive the real `StopDisposition::from` rather than a hand-picked disposition, so the send site and the handler cannot drift apart silently"
  - "The run-list scan and the selected run's inbox read share one `spawn_blocking` task, because the second depends on the first — splitting them costs either a round trip through the event loop or a stale index"
  - "`goal_or_none` tests emptiness on the trimmed text but returns the untrimmed original: three spaces is no goal, and a goal that was given is stored verbatim"
  - "`driver_line_for_record` returns `None` for `run_started`, `exec_started`, `cost` and the reserved kinds — they are header or Phase 20 data, and duplicating them into the pane would make its first rows a copy of the header above"
  - "The gap diagnostic is pushed ABOVE the batch that arrived, because a gap is what came before those records and never will"
  - "`DetailScreen::NAME` exists so the tick gate's screen comparison is compiler-enforced; a renamed screen would otherwise leave the gate matching a string nothing answers to and silently disable the redraw"
  - "The `DetailSubView::Driver` render arms paint nothing rather than a placeholder — the variant is unreachable today, and a placeholder would be a surface promising output no producer feeds"
  - "`Action::DriverDryRunLoaded` still stores nothing: the preview's state shape belongs to the screen that renders it, and the WR-10 half (scheduling it off the render thread) is the half with a threat-register row"

requirements-completed: []

duration: 41min
completed: 2026-07-29
status: complete
---

# Phase 18 Plan 05: Wire the Controller Summary

**The seam `app.rs` had been holding open for two phases is closed — journal
records now reach a bounded, pruned ring buffer and set the redraw flag, gaps
reach the user instead of only a log file, and a stop that stopped nothing no
longer erases the run from the dashboard's memory.**

## Performance

- **Duration:** 41 min
- **Tasks:** 3, plus the assigned two-field gap closure
- **Files modified:** 4
- **Tests:** 612 → 632 passing, 0 failing

## Accomplishments

- **WR-15 is closed, and the tests prove the half that matters.**
  `Action::DriverStopped` mutates `observed_runs` and `session_spawned_runs`
  only on `StopDisposition::RunGone`. Four new tests, one per `StopOutcome`
  variant, each driven through the **real** `StopDisposition::from` conversion
  rather than a hand-picked disposition — so the send site and the handler
  cannot drift apart silently. Each asserts the map state *and* the status
  message, following `driver_confirm.rs:367-372`'s discipline: an
  implementation that set the message and mutated anyway would pass a
  message-only test while the dashboard had already forgotten the run. The
  handler's doc names both costs the old behaviour paid — a five-second "no
  run" window, and a **permanently** lost `session_spawned_runs` entry that
  makes a later stop take the `Adopted` reaping arm for a run this session did
  spawn — and names the Phase 18 consequence: the Driver tab renders a "no run"
  pane directly beside a status line reading *"the stop signal could not be
  delivered"*.
- **The D-20 seam is open, and the comment that described its absence is
  gone.** `Action::DriverJournalAppended` projects each record into a
  `DriverLineKind`, pushes it through `DriverOutput::push_record` (which
  sanitises and enforces the cap), and sets `needs_redraw`. The comment block
  that read *"Phase 18 is what adds the surface and the flag together"* was
  rewritten rather than left standing above code that now does add them — while
  keeping, and re-justifying, the three omissions that are still real:
  `ProjectState`, `last_refresh` and the re-parse counter are all still
  untouched, and the new test asserts all three alongside the append.
- **Gaps became visible.** `seq_gaps_from`'s count previously reached a
  `tracing::warn!` and nothing else — a log file the user of a TUI never opens,
  while the pane presented itself as complete. It now also pushes
  `journal gap: {n} record(s) not read` as a `Diagnostic` line, **above** the
  batch that did arrive, because that is where the missing records belong.
  `events_dropped` and `journal_truncated` get the same treatment, and a
  seven-case table test asserts every diagnostic-bearing kind reaches the pane
  and that none of them renders as a `Debug` struct.
- **Three `spawn_blocking` schedulers, including the phase's own new blocking
  paths.** `schedule_inbox_append` (the `sync_data()`-backed durable write),
  `schedule_run_list_scan` (`read_dir` + one `run.json` per run + the selected
  run's inbox tail) and `schedule_dry_run_report` (two synchronous `git`
  shell-outs — one of the WR-10 sites the review named by hand). Wrapping only
  the calls the review listed while leaving this phase's own new ones on the
  render thread is the named half-fix, and it was not taken. Both filesystem
  schedulers resolve their run id through the **fallible** `run_paths` first, so
  a traversing id is refused before any join (T-18-27), logged by kind only —
  never the path, which is the untrusted value itself.
- **The empty-goal trap is shut.** `goal_or_none` maps an empty or
  whitespace-only buffer to `None`, and the test drives the real `drive_argv` to
  show the consequence: `None` omits `--goal` entirely, while `Some("")` would
  push the flag with an empty operand and have the driver record an empty goal
  **as though one had been given**. The distinction is user-visible — no goal
  renders `(none given)`, a recorded empty goal renders as a blank line that
  looks like a goal the reader cannot see.
- **D-21's gate is a pure predicate with a five-case truth table.** The elapsed
  counter rides the existing 250 ms tick — `rg -c 'tokio::time::interval'`
  reports **0**, unchanged — and repaints only when a detail view is on top, its
  sub-view is `Driver`, and the selected project has a live run. The dashboard
  case is asserted explicitly, because an unconditional per-tick redraw would
  repaint an idle fleet dashboard four times a second for the life of the
  process, which is the exact opposite of this tool's pitch. A second test
  drives the gate through a real `Action::Tick` rather than only the predicate.
- **The phase's last carry-forward is discharged.** `driver_output` is retained
  by registered alias in `prune_driver_maps`, with a control arm asserting a
  registered alias keeps its buffer **and its contents** — a prune that emptied
  every buffer would satisfy a `contains_key` assertion while erasing the pane.
- **The two-field gap 18-04 deferred to this plan is closed.**
  `Action::DriverRunsListed.runs` and `ProjectViewCache.driver_runs`, both
  naming `journal::RunSummary`, which exists on this tree now that 18-03 has
  merged. 18-04's loud deferral note was replaced, not merely supplemented.
- **No new dependency, no clippy regression.** `git diff Cargo.toml Cargo.lock`
  is empty; `cargo clippy --all-targets` still reports exactly the 5
  pre-existing lints.

## Task Commits

1. **Task 1: Stop dropping a run that was never stopped (WR-15 / D-29)** — `3fbe2c7` (fix)
2. **Assigned gap closure: the two `RunSummary`-typed members** — `903e6e0` (feat)
3. **Task 2: Three schedulers, the goal, the Driver sub-view** — `d9f720b` (feat)
4. **Task 3: The seam, gap diagnostics, the gated tick, the prune** — `23feb3f` (feat)

## Files Created/Modified

**Modified**

- `src/app.rs` — `DetailSubView::Driver`; `goal_or_none`;
  `driver_line_for_record`; `journal_gap_line`; `driver_elapsed_redraw_wanted`;
  `App::driver_elapsed_redraw_due`; `App::planning_dir_for`;
  `App::schedule_inbox_append` / `schedule_run_list_scan` /
  `schedule_dry_run_report`; the rewritten `DriverJournalAppended`,
  `DriverStopped`, `DriverInjectRequested`, `DriverInjectWritten` and
  `DriverRunsListed` handlers; the gated `Action::Tick` arm; the extended
  `prune_driver_maps`; the WR-11 visible refusal in `stop_driver_run`; and 23
  new inline tests.
- `src/ui/screens/detail.rs` — `DetailScreen::NAME`; `DetailSubView::Driver` in
  `tab_index` and `sub_view_from_index`; the two compile-forced render-dispatch
  arms; `tab_index`/`sub_view_from_index` raised to `pub(crate)` so the index
  round trip is assertable from the module that owns the enum.
- `src/action.rs` — `DriverRunsListed.runs` (the assigned addition, and the only
  edit to this file).
- `src/ui/screens/mod.rs` — `ProjectViewCache.driver_runs` (the assigned
  addition, and the only edit to this file).

## Decisions Made

- **The WR-15 tests go through `StopDisposition::from`, not a literal
  disposition.** Testing the handler against a hand-picked value would assert
  its behaviour for a mapping the send site might not produce, and the two
  halves of the fix could then drift apart with every test still green.
- **The run list and the inbox share one blocking task.** Which run's inbox to
  read is decided by indexing the freshly-listed runs, so splitting them would
  mean either a round trip through the event loop between the two reads or an
  index taken before the list it indexes.
- **The inbox is read whole, from offset zero, every time.** The payload is
  authoritative rather than a delta — the same reason `RunsReconciled` carries
  the whole scan — so a message no longer in the file is expressed by its
  absence and by nothing else.
- **`goal_or_none` trims to *test* and returns *untrimmed*.** A buffer of three
  spaces is no goal; a goal that was given is stored verbatim and never
  paraphrased, so nothing rewrites what the user typed.
- **Header data is deliberately not buffered into the output pane.**
  `run_started`, `exec_started` and `cost` render above the pane as the goal, the
  command, the started time and the cumulative cost; putting them in both would
  make the pane's first rows a duplicate of the header. This is not "losing
  lines" — the seven-case table test is what guarantees every
  diagnostic-bearing kind *does* arrive.
- **`DetailScreen::NAME` rather than a repeated `"detail"` literal.** The tick
  gate has to ask "is a detail view on top?" of a `Box<dyn Screen>`, so the only
  answer available is the name. A renamed screen would otherwise leave the gate
  comparing against a string nothing answers to — and the failure mode is a
  silently dead redraw, which no test would catch if the test used the literal
  too.
- **The `DetailSubView::Driver` render arms paint nothing.** The variant is
  genuinely unreachable today: `TAB_TITLES` still has ten entries so `Right`
  stops at 9, and no key binds to index 10. A placeholder would be a surface
  promising output that no producer feeds, which is the named "looks done but
  isn't" failure; an empty arm with a comment saying why is honest.
- **The three schedulers are `pub`**, for the reason `start_driver_run` and
  `stop_driver_run` are: they are the seams the UI layer reaches, and two of
  them have no in-file caller yet because their callers are the Driver tab's own
  key handling.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] An existing test asserted the WR-15 behaviour being removed**
- **Found during:** Task 1
- **Issue:** `a_stop_returns_its_outcome_as_an_action_rather_than_blocking`
  dispatches a stop against a pid belonging to no driver, so the task takes the
  `AlreadyGone` path — which now maps to `MayStillBeLive`. Its closing
  assertions (`!observed_runs.contains_key`, `!session_spawned_runs.contains`)
  asserted exactly the behaviour the fix removes, and would have failed.
- **Fix:** The test's real subject is the seam, so that half is unchanged and
  now also asserts the received disposition **is** `MayStillBeLive`. The drop
  assertions were kept but re-driven with an explicit `RunGone`, with a comment
  recording that reusing the dispatched disposition there would make the
  assertion silently test nothing.
- **Files modified:** `src/app.rs`
- **Committed in:** `3fbe2c7`

**2. [Rule 2 - Missing Critical] WR-11 taken alongside WR-15, in the same handler**
- **Found during:** Task 1
- **Issue:** `stop_driver_run`'s `let Some(tx) = … else { return; }` returned
  silently. The user presses the stop key against a live run, is told nothing at
  all, and the run keeps driving their repository. The scope fence names WR-11
  as *"welcome but not an acceptance criterion"* if the fix touches those lines
  anyway — it does.
- **Fix:** A visible refusal following S4, plus a test. The condition is
  unreachable in production, which is exactly why the silent arm survived: it
  fires only in a test or after a wiring regression, and both are cases where
  silence costs the most.
- **Files modified:** `src/app.rs`
- **Committed in:** `3fbe2c7`

**3. [Rule 3 - Blocking] The compiler forces four `detail.rs` arms, not the two the plan names**
- **Found during:** Task 2
- **Issue:** The plan names `tab_index` and `sub_view_from_index` as *"the
  minimum to keep the crate building"*. Both render-dispatch matches
  (`detail.rs:1899` and its duplicate at `:3195`, used by `EnqueueScreen`) are
  also exhaustive with no `_ =>`, so a new variant is a hard compile error at
  four sites rather than two. `footer_spans` has a catch-all and needed nothing.
- **Fix:** Two empty arms, each with a comment stating that 18-09 renders the
  tab, that the variant is unreachable today, and that a placeholder would be
  worse than nothing.
- **Files modified:** `src/ui/screens/detail.rs`
- **Committed in:** `d9f720b`

**4. [Rule 3 - Blocking] `tab_index` / `sub_view_from_index` raised to `pub(crate)`**
- **Found during:** Task 2
- **Issue:** The plan's inline tests live in `app.rs`, which owns
  `DetailSubView`, but both mapping functions were private to `detail.rs`. The
  round trip — the property actually worth asserting, since a tab whose index
  does not round-trip lands the user on a different tab — takes both halves.
- **Fix:** Both raised to `pub(crate)` with a doc recording why. No behaviour
  change.
- **Files modified:** `src/ui/screens/detail.rs`
- **Committed in:** `d9f720b`

**5. [Rule 4 - Deferred rather than guessed] `Action::DriverDryRunLoaded` stores nothing**
- **Found during:** Task 2
- **Issue:** The plan says *"the handler just stores the string"*, but there is
  nowhere in scope to store it. The execution brief fences this plan to exactly
  two additions in `src/ui/screens/mod.rs`, and `ProjectViewCache` is the only
  sensible home. The preview is rendered by `DriverStartScreen` at Step B
  (UI-SPEC Surface 5) — a sibling plan's screen, whose owner should choose the
  field's shape.
- **Fix:** `schedule_dry_run_report` **landed in full**, which is the half with
  a threat-register row (T-18-25: `build_report` shells out to `git` twice and
  must never run on the render thread). The handler logs by count and carries a
  doc naming the missing field, its owner and the exact remedy — the same seam
  convention 18-04 used for the gap this plan just closed. D-26 also marks the
  whole preview **CUTTABLE** and *"the first thing to cut"*, the only item on
  that surface with no requirement id, so nothing downstream is blocked.
  Recorded under **Known Stubs**.
- **Alternative rejected:** guessing a field shape. If a sibling defines
  `driver_dry_run_report: Option<String>` while this plan guessed
  `Option<(String, String)>`, the result is a real merge conflict plus two
  copies of one piece of state — strictly worse than a declared seam.
- **Files modified:** `src/app.rs`
- **Committed in:** `d9f720b`

**6. [Rule 3 - Blocking] The three schedulers are `pub`**
- **Found during:** Task 2
- **Issue:** `schedule_run_list_scan` and `schedule_dry_run_report` have no
  in-file caller — their callers are the Driver tab's key handling, a later
  plan's. A private method with no non-test caller is `dead_code`, which
  `cargo clippy -- -D warnings` fails on (test usage does not rescue it, because
  that gate does not build test targets).
- **Fix:** All three `pub`, matching `start_driver_run` and `stop_driver_run` —
  the file's existing convention for driver seams the UI layer reaches. A shared
  comment records the reason once rather than three times.
- **Files modified:** `src/app.rs`
- **Committed in:** `d9f720b`

---

**Total deviations:** 6 auto-fixed (4 blocking, 1 bug, 1 missing critical). One
(#5) defers a storage field with a named owner rather than guessing its shape.
**Impact on plan:** No scope creep, no new file, no new dependency. Every
acceptance criterion in all three tasks is met.

## Issues Encountered

- **`git stash list` in this worktree shows entries from sibling sessions.** The
  stash ref lives in the parent `.git/` and is shared across every linked
  worktree, so the six WIP entries visible here belong to other work. Nothing in
  this plan touched them; recorded because the shared-stash surprise is easy to
  mistake for local state.
- **`tests/driver_reattach.rs` did not flake during this plan**, but it was run
  in isolation to confirm rather than assumed — 3 passed, as expected.
- **Every count in this summary was taken through `rtk proxy`.** Plain `cargo`
  output is filtered by the summarising wrapper, so a criterion grepping for
  `warning:` or `test result:` without it passes **vacuously**. This bit Phase
  15 and Phase 17 and would have made the clippy-count claim below meaningless.

## Verification

| Gate | Result |
|---|---|
| `cargo build` | pass |
| `cargo test` | **632 passed, 0 failed** (baseline 612) |
| `cargo clippy -- -D warnings` | pass |
| `rtk proxy … cargo clippy --all-targets … \| wc -l` | **5** — the pre-existing count, unchanged |
| `git diff --stat Cargo.toml Cargo.lock` | empty — no new dependency |
| `rtk proxy cargo test --test driver_kill --test driver_kill_startup` | pass (3 + 1) |
| `rtk proxy cargo test --test driver_reattach` (isolated) | pass (3) |
| `rg -c 'tokio::time::interval\|Interval::' src/app.rs` | **0** — no second timer was added |
| `rg -c 'spawn_blocking' src/app.rs` | 10 → **16** (≥ 3 more, as required) |
| `rg -n 'StopDisposition' src/app.rs` | matches at the send site **and** the handler |
| `rg -n 'DetailSubView::Driver' src/app.rs src/ui/screens/detail.rs` | matches in both files |
| `rg -n 'driver_output' src/app.rs` | matches inside `prune_driver_maps` |
| `git diff --diff-filter=D` per commit | empty — no file was deleted |

### Threat register

| Threat ID | Disposition | Evidence |
|---|---|---|
| T-18-25 (blocking `sync_data` / `git` / `read_dir` on the render thread) | **mitigated** | All three schedulers run on `spawn_blocking` and return through an `Action`; `an_injection_is_written_off_the_render_thread_and_reports_back` drives the append round trip, and no filesystem call sits outside a closure in any of the three |
| T-18-26 (a stop that did not stop, recorded as if it had) | **mitigated** | Disposition-gated mutation plus four per-outcome tests asserting map state **and** message, driven through the real `From<&StopOutcome>` |
| T-18-27 (a traversing run id reaching a scheduler) | **mitigated** | Both filesystem schedulers resolve through the fallible `run_paths` before any join; `a_traversing_run_id_never_reaches_the_inbox_append` asserts the refusal with a plain-id positive control beside it |
| T-18-28 (unbounded `driver_output` growth) | **mitigated** | `prune_driver_maps` retains by registered alias; `pruning_drops_the_output_buffer_for_an_unregistered_alias` asserts the drop and that a registered alias keeps its buffer *and its contents* |
| T-18-29 (unconditional per-tick redraw) | **mitigated** | `the_elapsed_redraw_gate_is_true_for_exactly_one_combination` pins the five-case truth table; `a_tick_repaints_only_when_the_driver_tab_is_watching_a_live_run` drives it through a real `Action::Tick` |
| T-18-30 (scheduler logs carrying paths or message bodies) | **mitigated** | Every new log line carries an error kind or a count. The inbox append renders `e.kind()` rather than the `io::Error`, whose `Display` can carry an OS path |
| T-18-31 (package-manager installs) | **accepted** | Zero dependencies added; `Cargo.toml`/`Cargo.lock` untouched |

## Known Stubs

Each is a declared seam with a named owner. None asserts a state the mechanism
cannot back.

1. **`Action::DriverDryRunLoaded` stores nothing.** `schedule_dry_run_report`
   builds the report off the render thread — the half with the threat-register
   row — but no field parks the string. **Owner: the plan that adds
   `driver_start.rs`**, which renders the preview at Step B and should choose
   its own state shape. D-26 marks the whole preview CUTTABLE, so nothing is
   blocked. See deviation 5.
2. **`DetailSubView::Driver` is unreachable.** `TAB_TITLES` still has ten
   entries, so `Right` stops at index 9 and no key binds to 10; both render
   arms paint nothing. **Owner: 18-09** — the title vector, the `Shift+D`
   binding, the footer hints and the rendering. This is a staged landing, and
   the code says so at all four sites.
3. **`App::schedule_run_list_scan` and `schedule_dry_run_report` have no
   production caller.** Both are complete, tested and `pub`; their callers are
   the Driver tab's own key handling. **Owners: 18-07 and 18-09.**
4. **`ProjectViewCache.driver_runs` is written but never read.** The reader is
   the run-list pane. **Owner: 18-09.**

## User Setup Required

None — no external service configuration required.

## Next Phase Readiness

- **18-07** has `App::schedule_inbox_append` behind
  `Action::DriverInjectRequested`, so its injection screen dispatches a message
  and touches no file. The status and error copy for the write are already
  wired to the Copywriting Contract's exact strings. It also inherits the
  dry-run storage field from Known Stub 1.
- **18-09** has `driver_output` filling with classified, sanitised, bounded
  lines and a `needs_redraw` that fires on every append; `driver_runs` and
  `driver_inbox` populated by `schedule_run_list_scan`; `DetailSubView::Driver`
  at index 10 with both mappings agreeing; and `DetailScreen::NAME` for any
  further stack-top checks. The five remaining 11th-tab sites — `TAB_TITLES`,
  `switch_to_tab`, the two render dispatches' real bodies, `footer_spans` — are
  its own.
- **The negative carry-forward is fully discharged.** Every per-alias map this
  phase added is pruned in `App::prune_driver_maps`, asserted by test. A plan
  that adds another one inherits the obligation afresh.
- **Requirements are deliberately not marked complete.** TRANS-05, OBS-03,
  OBS-04 and STEER-01 are not reachable by a user until there is a Driver tab to
  see and an injection screen to type into. This plan builds the controller they
  run on; they close later in the phase.

## Self-Check: PASSED

All four modified files present on disk (`src/app.rs`,
`src/ui/screens/detail.rs`, `src/action.rs`, `src/ui/screens/mod.rs`); all four
commits present in `git log` (`3fbe2c7`, `903e6e0`, `d9f720b`, `23feb3f`);
`git diff --diff-filter=D` empty for each of the four — no file was deleted;
working tree clean apart from this summary.

---
*Phase: 18-driver-tab-live-watch-durable-injection*
*Completed: 2026-07-29*
