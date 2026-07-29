---
phase: 16-run-journal-state-substrate
plan: 05
subsystem: watcher / app event loop
tags: [obs-06, watcher, dedup, byte-offset-tail, app-context, performance]
status: complete

requires:
  - "16-01: journal::classify_change, journal::ChangeKind, journal::run_paths"
  - "16-01: journal::reader::{tail_lines, parse_line, TailCursor, JournalRecord, ParsedLine}"
  - "16-02: the ExecutionEvent::EventsDropped arm in apply_exec_event (preserved, untouched)"
provides:
  - "Action::FileChanged carries changed_path — the classification input"
  - "Action::DriverJournalAppended — the tail-result message"
  - "watcher::batch_actions — the per-(root, ChangeKind) debounce fold"
  - "AppContext::reparse_dispatches — the OBS-06 measurement seam"
  - "AppContext::journal_cursors — per-(alias, run_id) byte offsets"
  - "App::schedule_reparse — the ONLY full re-parse dispatch site"
  - "App::schedule_journal_tail — the byte-offset tail dispatch"
affects:
  - "Phase 17: schedule_journal_tail is the read side its driver writes against"
  - "Phase 18: reparse_dispatches and journal_cursors are the state a driver surface renders from"

tech-stack:
  added: []
  patterns:
    - "spawn_blocking-then-Action-on-a-cloned-sender (the file's third instance)"
    - "sibling plain-data maps on AppContext rather than fields on ProjectState"
    - "pure per-batch fold extracted out of a debouncer closure so it is testable"

key-files:
  created: []
  modified:
    - src/action.rs
    - src/watcher.rs
    - src/app.rs
    - src/ui/screens/mod.rs
    - src/ui/screens/detail.rs

decisions:
  - "The watcher dedup key became (project_root, ChangeKind). Per-root dedup silently dropped every path after the first for a root in a 200ms batch — a regression classification introduces, fixed in the same change (D-10)."
  - "Classification happens BEFORE the 500ms dedup check, and the driver route never touches last_refresh, so a journal append cannot suppress a genuine state refresh (D-14)."
  - "reparse_dispatches is a plain u64 on AppContext, not cfg(test)-gated and not a static AtomicUsize — the static would be incremented concurrently by sibling test threads in the same process and fail intermittently (RESEARCH §7.4)."
  - "No wall-clock timing assertion, following main_loop.rs's own recorded precedent. The counter is the load-bearing evidence."
  - "DriverJournalAppended deliberately does not set needs_redraw: this phase ships no surface that renders journal content (D-36)."

metrics:
  duration: ~35 min
  completed: 2026-07-29
  tasks: 3
  tests_added: 8
  tests_total: 399
---

# Phase 16 Plan 05: OBS-06 — Free Watching Gets a Bill Summary

Journal appends now take a byte-offset tail instead of a full `parse_project_state`, proved by
counting the dispatches with a control arm, and the watcher's debounce dedup was widened to
`(root, classification)` so the classification fork cannot swallow a `STATE.md` write.

## What Was Built

**Task 1 — `28717be` — the watcher tells driver writes from GSD writes**

`Action::FileChanged` gained `changed_path` alongside `project_path` (D-09): `project_path` keys
the alias lookup, `changed_path` is the classification input. `Action::DriverJournalAppended`
was added carrying `{ alias, run_id, records: Vec<JournalRecord>, cursor: TailCursor }` — 80
bytes, comfortably inside the measured `large_enum_variant` budget, so nothing was boxed
reflexively (`grep -c 'Box<' src/action.rs` is still `1`, the pre-existing `ProjectStateLoaded`).

The debouncer callback's per-batch fold was extracted into a free function `batch_actions()` and
its dedup key widened from `PathBuf` to `(PathBuf, ChangeKind)`. **This is the correctness fix,
not a tidy-up.** With a per-root key the first path for a project in a batch won and every later
path for that root was silently dropped; once journal appends start arriving, a batch carrying a
journal append followed by a `STATE.md` write loses the state write and the project never
re-parses. The extraction is what makes the regression testable at all — without it the fix
ships silently.

Three regression tests, including
`a_batch_with_the_journal_first_still_emits_the_planning_event`, which orders the journal path at
index 0 (the ordering under which the old shape loses the write) and asserts **two** actions.

**Task 2 — `c1a3f03` — the handler fork**

`AppContext` gained two plain-data sibling maps next to `run_states` / `last_refresh` /
`archive_cache`:

- `reparse_dispatches: u64` — full `parse_project_state` dispatches this process has issued.
  Not `cfg(test)`-gated, deliberately: a gated counter means the test exercises a different
  binary than production, and the count is a legitimate diagnostic Phase 18 may want.
- `journal_cursors: HashMap<(String, String), TailCursor>` — byte offsets keyed by
  `(alias, run_id)`. Offsets, run ids and counts only; no handle, so `Action` stays `Clone` (D-20).

`schedule_reparse()` is now **the only place a full project re-parse is scheduled**, and
increments the counter **before** the `spawn_blocking` so the count is synchronous and cannot
race the blocking task. `schedule_journal_tail()` reads from the stored offset on
`spawn_blocking`, parses the returned lines, and sends `DriverJournalAppended` — calling neither
`parse_project_state`, nor `last_refresh`, nor the counter. Those three negatives are the literal
content of OBS-06.

The `FileChanged` arm classifies **before** the 500 ms dedup check (D-14). The
`DriverJournalAppended` arm advances only its own cursor, logs sequence gaps as a count-only
`tracing::warn!`, and deliberately does not set `needs_redraw` — this phase ships no surface that
renders journal content (D-36), so the flag would schedule a frame that cannot differ.

`ProjectState` is byte-identical to before (`git diff src/state_reader/` is empty), so its derived
equality still suppresses the "Updated: {alias}" status message (D-18).

**Task 3 — `fcf9c62` — the measurement**

Four `#[tokio::test]`s (required: both schedulers call `spawn_blocking`, which panics without a
runtime), all built through one `obs_app()` helper so a handler change cannot satisfy one arm by
breaking the other:

| Test | Asserts |
|---|---|
| `journal_appends_never_trigger_a_full_reparse` | 500 driver-path events, counter unchanged; leaked count in the failure message |
| `a_planning_write_still_triggers_a_reparse` | counter == before **+ exactly 1** — the control arm |
| `the_driver_route_leaves_the_refresh_dedup_map_untouched` | `last_refresh` empty after driver events, populated after a planning event |
| `a_tail_result_advances_only_its_own_cursor` | one run's cursor moves, its sibling's does not |

The `static AtomicUsize` seam is rejected in a comment (cargo runs a crate's tests as threads in
one process, so a sibling test constructing project state would make the failure intermittent),
and the wall-clock assertion is declined citing `main_loop.rs`'s own recorded reasoning — so both
absences read as choices rather than omissions.

## Evidence the Tests Are Not Vacuous

The plan flags the failure mode explicitly: "zero re-parses happened" also passes against a
handler that dropped `FileChanged` entirely. Two independent guards:

1. **The control arm.** `a_planning_write_still_triggers_a_reparse` runs against the same helper,
   so a dropped-events handler fails it. It also proves the alias lookup resolves and `event_tx`
   is live, which is what stops the zero-arm passing for the wrong reason.
2. **An executed mutation check.** Routing the `DriverJournal` branch to `schedule_reparse`
   instead of `schedule_journal_tail` produced:

   ```
   test a_planning_write_still_triggers_a_reparse ... ok
   test the_driver_route_leaves_the_refresh_dedup_map_untouched ... FAILED
   test journal_appends_never_trigger_a_full_reparse ... FAILED
   ```

   The mutation was reverted before the Task 3 commit; the working tree at `fcf9c62` is the
   correct fork.

The same applies to the watcher test: with a per-root key, `fold([journal, state_md])` emits one
action and `assert_eq!(actions.len(), 2)` fails.

## Deviations from Plan

**1. [Rule 3 — blocking] Task 1 needed a placeholder `DriverJournalAppended` match arm**

- **Found during:** Task 1
- **Issue:** The plan puts the new `Action` variant in Task 1 and its handler in Task 2. Adding an
  enum variant without a match arm makes `update()` non-exhaustive, so Task 1 could not satisfy
  its own `cargo build` acceptance criterion.
- **Fix:** Task 1 added `Action::DriverJournalAppended { .. } => {}` with a comment stating the
  real body lands in the next commit and that nothing emits the variant yet. Task 2 replaced it
  with the full handler.
- **Files modified:** `src/app.rs`
- **Commits:** `28717be` (placeholder), `c1a3f03` (replaced)

**2. [Rule 3 — blocking] Task 2's `parse_project_state` grep criterion was unsatisfiable as written**

- **Found during:** Task 2 verification
- **Issue:** The criterion reads *"`grep -c 'parse_project_state' src/app.rs` outputs exactly `2`
  — one inside `schedule_reparse` and one in the pre-existing synchronous startup loader"*. The
  base commit already had **four** call sites, not two: `load_project_states`,
  `auto_register_new_sessions`, the `FileChanged` arm, and `CreateProjectResult`. The criterion
  enumerated two of them.
- **Resolution:** The criterion's *intent* — "no third dispatch site was introduced" — is what was
  verified. After the refactor there are still exactly four call sites: `schedule_reparse`
  (formerly the `FileChanged` arm) plus the three pre-existing ones. The count did not grow, and
  the raw `grep -c` now reads `7` only because the new doc comments name the function in prose.
  The stronger guarantee actually holds: `schedule_reparse` is the only *watcher-driven* dispatch,
  and it is the only writer of `reparse_dispatches`, which is what the OBS-06 assertion rests on.
- **Files modified:** none — documentation-only correction to a plan criterion.

**3. [Rule 2 — missing coverage] One watcher test beyond the three named**

Added `paths_outside_a_planning_directory_emit_nothing`. The extracted `batch_actions` gained a
new `continue` branch for paths with no resolvable project root; leaving that branch uncovered
would have made a future refactor of it invisible.

## Threat Mitigations Applied

| Threat ID | Applied |
|---|---|
| T-16-23 | `classify_change` is consumed unmodified from 16-01, which anchors on the full `.planning/meta-manager/runs/` prefix and is exhaustively tested including the bare-`runs` case |
| T-16-24 | The fork itself, plus the counting test with its control arm |
| T-16-25 | Classification runs on the callback thread precisely because it is pure — `batch_actions` performs no filesystem access; the tail runs on `spawn_blocking` |
| T-16-26 | Dedup key widened to `(root, ChangeKind)`, with the journal-first ordering test |
| T-16-27 | Every `tracing` call added here carries alias, run id, and a flag or a count. The `tail_lines` error path logs `e.kind()` only — never the message body or the path |
| T-16-28 | `DriverJournalAppended` is `String`/`String`/`Vec`/`u64` = 80 bytes; `Action` stays `Clone` and unboxed |

## Verification

```
cargo build                                                    clean
cargo test                                       399 passed, 0 failed  (baseline 391, +8)
cargo clippy -- -D warnings                                  exit 0
rtk proxy cargo clippy --all-targets | grep '^warning: '
  | grep -v generated | wc -l                                       5  (baseline 5, unchanged)
git diff --stat src/state_reader/                            (empty)  D-18 holds
git diff --diff-filter=D --name-only 62b8e32 HEAD            (empty)  no deletions
```

Zero new crate dependencies.

## Success Criteria

- [x] Five hundred journal appends produce zero `parse_project_state` dispatches, and one planning
      write produces exactly one (OBS-06, D-17)
- [x] A debounce batch carrying a journal append first and a state write second emits both actions (D-10)
- [x] The driver route touches neither `last_refresh` nor `reparse_dispatches` (D-14)
- [x] `ProjectState` is unchanged, so the "Updated: {alias}" suppression still works (D-18)

## Known Stubs

None. Every surface this plan touches is wired end to end. The one deliberate omission —
`DriverJournalAppended` not setting `needs_redraw` — is not a stub: this phase ships no UI (D-36),
and the reason is written into the arm so a future reader does not read it as a bug.

## Notes for the Next Plan

- `schedule_journal_tail` is `fn`, not `pub fn`. Phase 17's driver will need a way to trigger a
  tail outside a watcher event; promote it then, with the reason recorded.
- `journal_cursors` is never pruned. A long-lived process that watches many runs accumulates one
  16-byte entry per `(alias, run_id)` forever. At D-32's ten-runs-per-project retention that is
  bounded in practice, but retention pruning (Phase 17) is the natural place to drop the
  corresponding cursor entries.
- Sequence-gap detection is **within one tail batch only** — no last-seen `seq` is stored across
  batches, so a gap straddling two tail reads is invisible. Storing the last `seq` alongside the
  cursor would close that; it was out of this plan's scope.
