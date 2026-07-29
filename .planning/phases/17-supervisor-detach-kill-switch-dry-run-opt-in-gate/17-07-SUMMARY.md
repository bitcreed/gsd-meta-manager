---
phase: 17-supervisor-detach-kill-switch-dry-run-opt-in-gate
plan: 07
subsystem: ui
tags: [driver, ui, keybinding, confirmation, opt-in, retention, sequence-gap, phase-gate, ctrl-01, ctrl-03, ctrl-04]

requires:
  - phase: 17-supervisor-detach-kill-switch-dry-run-opt-in-gate
    plan: 03
    provides: "registry::record_opt_in / clear_opt_in / is_opted_in, RegisteredProject.driver_opt_in, schema v2, driver_max_concurrent"
  - phase: 17-supervisor-detach-kill-switch-dry-run-opt-in-gate
    plan: 05
    provides: "App::start_driver_run, AppContext.observed_runs, reconcile_all on the existing 20-tick block"
  - phase: 17-supervisor-detach-kill-switch-dry-run-opt-in-gate
    plan: 06
    provides: "App::stop_driver_run, Action::DriverStopRequested, AppContext.session_spawned_runs"
  - phase: 16-run-journal-state-substrate
    provides: "reader::seq_gaps, TailCursor, ReadDiagnostics.last_seq, RETAIN_RUNS, new_run_id's lexicographic-equals-chronological property"
provides:
  - "`src/ui/screens/driver_confirm.rs`: DriverAction {Start, Stop, ToggleOptIn}, DriverConfirmScreen, DEFAULT_DRIVE_COMMAND, prompt_text/prompt_color, and the three do_* free functions"
  - "Three dashboard key bindings — `r` start, `x` stop, `o` toggle opt-in — each pushing the same confirmation"
  - "`Action::DriverStartRequested { alias, command }` and its `App::update` arm"
  - "`src/journal/reader.rs`: `JournalCursor {cursor, last_seq}` (Copy) and `seq_gaps_from`, with `seq_gaps` collapsed onto it"
  - "`App::prune_driver_maps` — unregistered aliases dropped and RETAIN_RUNS runs per alias retained, on the existing 20-tick block"
  - "Driver-map cleanup on the interactive removal path, and `remove_project`'s doc naming both cleanup sites"
  - "The phase gate: clippy delta, per-target suite breakdown, five-criterion traceability, requirement coverage, anti-pattern sweep, leak check, dependency check"
affects: [18 driver tab and command picker, 20 decision router]

tech-stack:
  added: []
  patterns:
    - "One confirmation screen in three modes rather than three screens, following the delete_confirm template file for file"
    - "A UI gate documented as an affordance at the point where somebody would later mistake it for the enforcement"
    - "A prompt whose wording is bounded by what the code actually does — the withdraw direction says NEW because disabling stops no live run"
    - "A cursor widened with a second scalar keeps its Copy derive, and a test exists whose only job is to fail if that changes"
    - "Retention pruning rides the existing tick counter and sorts run-id strings rather than reading run.json timestamps"

key-files:
  created:
    - src/ui/screens/driver_confirm.rs
  modified:
    - src/ui/screens/normal.rs
    - src/ui/screens/mod.rs
    - src/ui/screens/help.rs
    - src/ui/screens/delete_confirm.rs
    - src/action.rs
    - src/app.rs
    - src/journal/reader.rs
    - src/registry.rs
    - .planning/REQUIREMENTS.md

key-decisions:
  - "The start path routes through Action::DriverStartRequested rather than reaching App directly: the seam lives on App, a Screen only ever receives &mut AppContext, and plan 17-06 already built exactly this route for the stop — adding the sibling is one mechanism, not two"
  - "A failed save_config on the opt-in toggle is ROLLED BACK in memory, which delete_confirm does not do and this must: the gate reads config.json from disk in a different process, so an unpersisted opt-in would make the TUI report a project drivable while every run against it is refused"
  - "DEFAULT_DRIVE_COMMAND is /gsd-progress and hardcoded as a scope fence rather than a stub — the command picker is Phase 18's and the router is Phase 20's, and the const's doc says so"
  - "The empty-batch case retains the previous last_seq rather than resetting it: an empty tail is the COMMON case (the watcher fires before the first append, a torn trailing line yields no record), and resetting would silently disarm the boundary check for the next batch that does carry records"
  - "prune_driver_maps runs inline rather than on spawn_blocking — it is pure in-memory map work with no syscall, unlike the reconciliation probe it shares a counter with"
  - "REQUIREMENTS.md is updated here but ROADMAP.md is not: the orchestrator owns ROADMAP.md and STATE.md for this worktree wave, and the plan's instruction to mark the ROADMAP checklist is recorded as a handoff rather than executed"

patterns-established:
  - "A grep criterion whose pattern appears in the criterion's own echo defeats itself — the bracket trick only works if the literal appears nowhere else on the command line"
  - "A process-leak criterion written as `pgrep -x claude` returns nothing is unsatisfiable on the machine this project is developed on; the checkable form is age-bounded against the suite's own start"

requirements-completed: [CTRL-01, CTRL-02, CTRL-03, CTRL-04, CTRL-05]

coverage:
  - id: U1
    description: "Three dashboard keys reach the phase's whole capability through one confirmation screen, with no new tab, widget or second screen, and no field on ProjectState"
    requirement: "CTRL-01"
    verification:
      - kind: unit
        ref: "src/ui/screens/driver_confirm.rs#every_prompt_names_the_alias_and_offers_the_same_two_keys"
        status: pass
      - kind: manual_procedural
        ref: "grep -c for the three KeyCode::Char arms in normal.rs is 3; `git status src/ui/screens/` shows exactly one file added; `git diff --stat src/state_reader/` is empty"
        status: pass
    human_judgment: false
  - id: U2
    description: "Starting a run against a project that has not opted in refuses visibly AND dispatches nothing"
    requirement: "CTRL-03"
    verification:
      - kind: unit
        ref: "src/ui/screens/driver_confirm.rs#starting_a_run_on_a_project_that_has_not_opted_in_sets_an_error_and_spawns_nothing"
        status: pass
      - kind: manual_procedural
        ref: "planted the removal of the early return; the test failed on the dispatch assertion (not the message assertion), then the plant was reverted"
        status: pass
    human_judgment: false
  - id: U3
    description: "The opt-in toggle records and clears a real record and persists it, because the gate reads config.json from a different process"
    requirement: "CTRL-03"
    verification:
      - kind: unit
        ref: "src/ui/screens/driver_confirm.rs#the_toggle_records_and_then_clears_an_opt_in_record"
        status: pass
    human_judgment: false
  - id: U4
    description: "A sequence gap that straddles two tail reads is detected, proved by two real reads with a skipped seq between them"
    verification:
      - kind: unit
        ref: "src/journal/reader.rs#a_gap_that_straddles_two_tail_reads_is_detected"
        status: pass
      - kind: unit
        ref: "src/journal/reader.rs#seq_gaps_from_zero_matches_the_unseeded_gap_check"
        status: pass
      - kind: unit
        ref: "src/journal/reader.rs#the_journal_cursor_is_copy"
        status: pass
    human_judgment: false
  - id: U5
    description: "The inline windows(2) duplicate in app.rs is gone, collapsed onto the shared reader function"
    verification:
      - kind: manual_procedural
        ref: "comment-filtered grep -c 'windows(2)' src/app.rs is 0; grep -c '^pub fn seq_gaps' src/journal/reader.rs is 2; one windows(2) in the reader's non-test region"
        status: pass
    human_judgment: false
  - id: U6
    description: "journal_cursors, run_states and observed_runs are pruned of unregistered aliases and bounded to RETAIN_RUNS per alias, on the existing 20-tick block with no new timer"
    verification:
      - kind: unit
        ref: "src/app.rs#pruning_drops_cursors_for_an_unregistered_alias"
        status: pass
      - kind: unit
        ref: "src/app.rs#pruning_retains_at_most_the_newest_runs_per_alias"
        status: pass
      - kind: unit
        ref: "src/app.rs#removing_a_project_interactively_clears_its_driver_maps"
        status: pass
      - kind: manual_procedural
        ref: "reversing the retention sort kept the right COUNT of the wrong runs and failed on the surviving ids; plant reverted. Timer grep in app.rs is 5, unchanged, all five session_poll_counter"
        status: pass
    human_judgment: false
  - id: U7
    description: "All five ROADMAP success criteria name a test that was re-run in this session and passed"
    requirement: "CTRL-01, CTRL-02, CTRL-03, CTRL-04, CTRL-05"
    verification:
      - kind: integration
        ref: "driver_kill, driver_reattach, driver_dry_run, driver_tracer, driver_optin, driver_lock — all re-run by name; see the traceability table below"
        status: pass
    human_judgment: false
  - id: U8
    description: "The --all-targets clippy count is still exactly 5, in the same three pre-existing files, and the phase's whole dependency footprint is the one rustix line with zero new lockfile package entries"
    verification:
      - kind: manual_procedural
        ref: "verbatim clippy output recorded below; `git diff fe2a8a6 HEAD -- Cargo.toml Cargo.lock` is 9 insertions, and `+name = ` in the lockfile diff is 0"
        status: pass
    human_judgment: false

duration: 42 min
completed: 2026-07-29
status: complete
---

# Phase 17 Plan 07: Three Keys, Two Carry-Forwards, and the Phase Gate Summary

**Three dashboard keys and one confirmation now make the whole phase reachable by a human — and pressing start on a project that has not opted in refuses visibly while dispatching nothing, which was proved by removing the guard and watching the test fail on the dispatch rather than on the message. The two Phase 16 carry-forwards are closed: a sequence gap straddling two tail reads is now detected by a genuinely two-read test, and `journal_cursors` no longer grows for the process lifetime.**

## Performance

- **Duration:** 42 min
- **Started:** 2026-07-29T19:01:36Z
- **Completed:** 2026-07-29T19:43:00Z
- **Tasks:** 3
- **Files modified:** 10 (1 created, 9 modified)

## Accomplishments

- **The phase is reachable from the dashboard, and the surface is exactly what D-25 fences it to.** `r`, `x` and `o` push one confirmation screen built from the `delete_confirm.rs` template — same `[y/n]` footer, same two-arm `handle_key`, same free functions reporting through `ctx.error_message` / `ctx.status_message`. Exactly one file was added under `src/ui/screens/`, `ProjectState` gained nothing, and the module doc names **Phase 18** twice so a reader arriving later does not mistake the minimalism for an oversight.
- **CTRL-03's affordance layer is proved by the assertion that discriminates, not the one that is easy.** A screen that set `error_message` *and dispatched anyway* would satisfy a test that only checked the message — and the run would start regardless of what the user was told. The test asserts the absence of a sent `Action`, and removing the early return failed it on exactly that line. The comment at the call site states both halves: the UI check is an **affordance**, the driver holds the real gate, and neither should be "simplified" into the other.
- **The opt-in toggle rolls back a failed save, which the screen it is modelled on does not need to do.** `delete_confirm.rs` removes then saves and leaves the in-memory removal standing on failure. Here that would be a real defect: the gate reads `config.json` **from disk, in a different process**, so an opt-in that never reached the file would make the TUI report a project as drivable while every run against it is refused by the driver. Reverting keeps memory and disk in agreement so the error message is the only thing to act on.
- **The cross-batch gap bug is closed by a test that a single-batch test cannot substitute for.** `a_gap_that_straddles_two_tail_reads_is_detected` performs two real `tail_lines` reads with seq 3 never written, and asserts — explicitly, in the test body — that the *within-batch* check sees nothing across `[4, 5]`. That assertion is what makes the test discriminating: the old implementation passes any gap that happens to sit inside a batch.
- **One filter, not two, in two senses.** `seq_gaps` is now `seq_gaps_from(0, ...)`, so the comparison exists once in the reader; and the inline `windows(2)` copy in `app.rs` — which could not be seeded and therefore could only ever see a within-batch gap — is deleted rather than commented out.
- **The retention prune was red-checked in the direction a count-only assertion cannot see.** Reversing the sort kept exactly `RETAIN_RUNS` entries — the right *count* — of the **oldest** runs, discarding precisely the cursors a live run needs. The test failed naming the surviving ids.
- **The gate was run, not quoted.** Every row below comes from a command executed in this session. Three plan criteria turned out to be unsatisfiable as literally written and one leak check silently defeated itself; all four are recorded with their corrected form rather than waved through.

## Task Commits

1. **Task 1: Three keys, one confirmation, and a visible refusal** — `64fb523` (feat)
2. **Task 2: The two Phase 16 carry-forwards — bounded cursors and cross-batch gap detection** — `15c477b` (fix)
3. **Task 3: Phase gate — traceability, requirement coverage, and the clippy delta** — this commit (docs)

## Files Created/Modified

**Created**

- `src/ui/screens/driver_confirm.rs` — `DEFAULT_DRIVE_COMMAND`, `DriverAction`, `DriverConfirmScreen`, `prompt_text`, `prompt_color`, `do_start_run`, `do_stop_run`, `do_toggle_opt_in`, `dispatch`, and five unit tests. The module doc opens with the Phase 18 fence and enumerates every driver surface that is *not* here.

**Modified**

- `src/ui/screens/normal.rs` — three key arms in the `KeyCode::Char('d')` shape, with the collision check against every binding this screen already claims recorded in a comment.
- `src/ui/screens/mod.rs` — module registration; the `journal_cursors` doc rewritten for the widened value and the prune; a note on `observed_runs` recording that Phase 17 added no field to `ProjectState`.
- `src/ui/screens/help.rs` — the three keys, scoped `(dashboard)` alongside the pre-existing detail-view `r`.
- `src/ui/screens/delete_confirm.rs` — the three driver maps cleaned in the same block as `project_states` / `last_refresh`.
- `src/action.rs` — `DriverStartRequested`; the `DriverJournalAppended` cursor retyped with its sizing note updated (80 → 88 bytes, still nothing to box).
- `src/app.rs` — the `DriverStartRequested` arm; `schedule_journal_tail` carrying and rebuilding the widened cursor; the `windows(2)` duplicate replaced by a seeded `seq_gaps_from`; `prune_driver_maps` and its call inside the existing 20-tick block; three new tests and one retyped.
- `src/journal/reader.rs` — `JournalCursor`, `seq_gaps_from`, `seq_gaps` collapsed onto it, and three tests.
- `src/registry.rs` — a paragraph on `remove_project` naming both cleanup sites.
- `.planning/REQUIREMENTS.md` — CTRL-01…CTRL-05 checked and their traceability rows set to Complete.

## The Phase Gate

### 1. Clippy delta audit

`rtk proxy sh -c "cargo clippy --all-targets 2>&1 | grep '^warning: ' | grep -v generated"`, verbatim:

```
warning: used `assert_eq!` with a literal bool
   --> src/browser.rs:131:9
warning: used `assert_eq!` with a literal bool
   --> src/browser.rs:132:9
warning: used `assert_eq!` with a literal bool
   --> src/browser.rs:133:9
warning: this creates an owned instance just for comparison
   --> src/project_creator.rs:146:27
warning: items after a test module
   --> src/state_reader/mod.rs:258:1
warning: `gsd-meta-manager` (lib test) generated 5 warnings
```

**Count: exactly 5.** Three files, all pre-existing, none of them this phase's: `browser.rs` ×3, `project_creator.rs` ×1, `state_reader/mod.rs` ×1. Neither grown nor shrunk — a shrink would have meant a file this phase should not have opened was opened.

**Path correction.** The plan and `17-CONTEXT.md` name the first file `src/ui/screens/browser.rs`. That path **does not exist**; the file is `src/browser.rs`, and `src/ui/screens/mod.rs` declares no `browser` module. Same three modules, same five lints, same line numbers — the transcription error was carried forward through the phase's context document and is corrected here rather than left for Phase 18 to trip over.

### 2. The lib gate

| Check | Result |
|---|---|
| `cargo build` | clean |
| `cargo clippy -- -D warnings` | exits 0 |
| `rtk proxy cargo test` | **519 passed, 0 failed** |

Per-target breakdown, against the **427** baseline measured at phase start:

| Target | Baseline | Now | Δ |
|---|---|---|---|
| lib (unit) | 366 | 430 | +64 |
| `main.rs` (unit) | 0 | 0 | — |
| `tests/driver_tracer.rs` | — | 4 | new (17-01) |
| `tests/spawn_seam_guard.rs` | — | 5 | new (17-01, 17-05, 17-06) |
| `tests/driver_lock.rs` | — | 4 | new (17-02) |
| `tests/driver_optin.rs` | — | 3 | new (17-03) |
| `tests/driver_dry_run.rs` | — | 4 | new (17-04) |
| `tests/driver_reattach.rs` | — | 3 | new (17-05) |
| `tests/driver_kill.rs` | — | 3 | new (17-06) |
| `tests/executor_lifecycle.rs` | 11 | 11 | — |
| `tests/executor_transport.rs` | 7 | 8 | +1 |
| `tests/journal_crash.rs` | 3 | 3 | — |
| `tests/journal_gitignore.rs` | 3 | 3 | — |
| `tests/registry_test.rs` | 12 | 13 | +1 |
| `tests/state_reader_test.rs` | 25 | 25 | — |
| doc-tests | 0 | 0 | — |
| **Total** | **427** | **519** | **+92** |

Every pre-existing target is still present and none lost a test. All **seven** new integration targets appear by name.

### 3. Criteria traceability

Every command below was **re-run in this session**; no number is copied from a SUMMARY.

| # | ROADMAP criterion (verbatim) | Evidencing test | Command | Observed |
|---|---|---|---|---|
| 1 | "Pressing stop during an active run leaves no `claude` process, no grandchild build or server process, and no zombie behind — verified 15s later" | `driver_kill::stopping_a_run_leaves_no_claude_no_grandchild_and_no_zombie` | `cargo test --test driver_kill stopping_a_run_leaves_no_claude_no_grandchild_and_no_zombie` | **1 passed** in 15.16s |
| 2 | "Closing the TUI mid-run leaves the run going; reopening the TUI shows that run still live with its current step" | `driver_reattach::a_run_outlives_the_process_that_spawned_it` and `::a_fresh_scan_finds_the_orphaned_run_live_with_its_last_journal_step` | `cargo test --test driver_reattach` | **3 passed** in 6.10s, both named tests present |
| 3 | "A dry-run reports the GSD commands it would issue plus the diffstat and push refspecs those commands would produce, and performs zero git writes" | `driver_dry_run::the_dry_run_output_names_the_command_the_diffstat_and_the_refspecs` and `::a_dry_run_leaves_the_git_directory_byte_identical` | `cargo test --test driver_dry_run` | **4 passed**, both named tests present |
| 4 | "A run cannot be started against a project that has not opted in, and a live run against one project provably performs no write, spawn, or git operation against any other registered project" | `driver_tracer::a_drive_against_a_project_with_no_opt_in_record_is_refused_and_writes_nothing` and `driver_optin::a_run_against_one_project_leaves_every_other_project_byte_identical` | `cargo test --test driver_tracer`; `cargo test --test driver_optin` | **4 passed** and **3 passed**, both named tests present |
| 5 | "A second attempt to drive the same project reports which run holds the lock instead of starting a second one" | `driver_lock::a_second_drive_reports_which_run_holds_the_lock` | `cargo test --test driver_lock` | **4 passed** in 6.07s, the named test present |

**No gaps.** Every row is filled by a test that exists and passed.

Criterion 4's TUI half is additionally covered at the affordance layer by `driver_confirm::starting_a_run_on_a_project_that_has_not_opted_in_sets_an_error_and_spawns_nothing`, which is this plan's contribution — but the criterion's *enforcement* half is `driver_tracer`'s, in the child process, and that is the one that makes "never" true.

### 4. Requirement coverage

| Requirement | Delivered by | Evidenced by criterion row |
|---|---|---|
| CTRL-01 — stop any run from the TUI, whole process tree | 17-06 (teardown, both reap arms) + 17-07 (the `x` key that reaches it) | 1 |
| CTRL-02 — dry-run reporting commands, diffstat and refspecs | 17-04 | 3 |
| CTRL-03 — opt-in enforced at the spawn seam | 17-01 (gate, `DrivableProject::from_registry`) + 17-03 (record, migration) + 17-07 (the `o` toggle and the visible refusal) | 4 |
| CTRL-04 — a run survives the TUI closing; reconciliation on restart | 17-05 | 2 |
| CTRL-05 — one driver per project, OS-level lock | 17-02 | 5 |

`.planning/REQUIREMENTS.md` updated: all five checkboxes checked and all five Traceability rows set to **Complete**.

**`.planning/ROADMAP.md` was deliberately not touched.** This executor runs in a worktree and the orchestrator owns `ROADMAP.md` and `STATE.md` for the wave — that constraint is in the executor's own objective and overrides the plan's step 4 instruction to mark the Phase 17 plan checklist. Handoff below.

### 5. Anti-pattern sweep

`grep -nE 'TODO|FIXME|XXX|HACK|todo!|unimplemented!|placeholder'` over every file this plan touched:

| File | Findings |
|---|---|
| `src/ui/screens/driver_confirm.rs` | none |
| `src/ui/screens/normal.rs` | none |
| `src/ui/screens/mod.rs` | none |
| `src/ui/screens/help.rs` | none |
| `src/ui/screens/delete_confirm.rs` | none |
| `src/app.rs` | none |
| `src/action.rs` | none |
| `src/journal/reader.rs` | none |
| `src/registry.rs` | none |

**Zero markers across all nine files** (grep exit 1). Nothing to fix and nothing to scope-defer.

### 6. Leak check

| Command | Output |
|---|---|
| `pgrep -af 'fak[e]-claude' \| wc -l` | `0` |
| `pgrep -af 'slee[p] 600' \| wc -l` | `0` |
| `ps -o etimes=,pid=,args= -C claude \| awk '$1 < 3600'  \| wc -l` | `0` |

No fixture stand-in leaked, and no `sleep 600` grandchild from `driver_kill`'s three-level process tree.

**Two corrections, both recorded rather than waved through.** First, the plan's `pgrep -f 'fake-claude'` self-matches (wave 6 recorded this) — and the bracket workaround *also* self-matched on the first attempt, because the same literal appeared in the criterion's own `echo "fake-claude EXIT=$?"`. The pattern is only self-proof if the literal appears **nowhere else on the command line**. Second, `pgrep -x claude` returns two live processes on this machine — pids 20520 and 711810, elapsed **1d 03h** and **21h 37m**. Both long predate every command in this session, one of them *is* the Claude Code session running this executor, and the suite never spawns a real `claude` at all (it runs against checked-in shell fixtures with no subscription). The criterion as literally written is unsatisfiable on the machine this project is developed on; the property it reaches for — "the suite leaked no agent" — is checked age-bounded above and holds.

### 7. Dependency check

`git diff --stat fe2a8a6 HEAD -- Cargo.toml Cargo.lock` (base = `fe2a8a6 docs(16): mark Phase 16 complete`, the last commit before Phase 17):

```
 Cargo.lock | 1 +
 Cargo.toml | 8 ++++++++
 2 files changed, 9 insertions(+)
```

The `Cargo.toml` change is the single `rustix = { version = "1.1", features = ["process", "fs"] }` line plus its seven-line comment recording D-08's reasoning. The `Cargo.lock` change is one line — `+ "rustix 1.1.4",` added to this crate's own dependency array. `git diff … Cargo.lock | grep -c '^+name = '` outputs **0**: **no new package entry**, because 1.1.4 was already in the graph transitively via ratatui/crossterm. The phase's entire dependency footprint is one direct dependency and zero new compilation units, exactly as D-08 predicted.

## Decisions Made

- **The start routes through an `Action`, not a direct call.** The plan left the choice open ("choose whichever of the two routes matches what plans 17-05 and 17-06 actually built"). 17-06 built `Action::DriverStopRequested` → `App::stop_driver_run`; `App::start_driver_run` had no message at all because it had no caller. Adding `DriverStartRequested` beside its sibling keeps one mechanism. The alternative — reaching `App` from a screen — is not available anyway: `Screen::handle_key` receives `&mut AppContext`, and `App` is a level above it.
- **`goal` is `None` on the dispatched start, not `Some("")`.** `drive_argv` omits the flag entirely for `None` and would record an empty goal verbatim into `RunRecord.goal` for `Some("")` — a distinction wave 5 already added a test for. There is no screen to type a goal into; that is Phase 18's.
- **A failed `save_config` on the toggle reverts the in-memory change.** Recorded in Accomplishments above; the short version is that the enforcement reads the file, not the process, so memory and disk disagreeing is a live defect rather than a cosmetic one.
- **The empty-batch case retains `last_seq`.** An empty tail read is the common case, not the exception. Resetting to zero on one would disarm the boundary check for the next batch that *does* carry records — turning the fix into a fix that works only when it is not needed.
- **`prune_driver_maps` runs inline, not on `spawn_blocking`.** It shares a counter with the reconciliation probe but not the probe's reason for being off-thread: the probe reads `/proc` and `run.json`, this touches three in-memory maps and makes no syscall.
- **The help screen keeps its existing flat format** and scopes the new keys `(dashboard)`, mirroring the existing `(detail view)` annotations. `r` now appears twice, once per screen, which is honest — the two bindings genuinely exist on two different screens with two different `handle_key` matches.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Task 1 cannot be done within its declared file list**

- **Found during:** Task 1
- **Issue:** The plan's `<files>` for Task 1 lists only the four `src/ui/screens/` files, but its own action text directs adding `DriverStartRequested` "in `src/action.rs`, if plan 17-05 did not already leave one" and handling it "in `App::update`". 17-05 left no such variant — it built `start_driver_run` with no message and no caller. So `src/action.rs` and `src/app.rs` are unavoidably in Task 1.
- **Fix:** Both files committed with Task 1. The plan frontmatter's `files_modified` already lists `src/app.rs`; `src/action.rs` is the one file the frontmatter omits entirely, and it gains one variant and one doc block.
- **Files modified:** `src/action.rs`, `src/app.rs`
- **Verification:** `cargo build` clean after the Task 1 commit; the full suite green at 513.
- **Committed in:** `64fb523`

**2. [Rule 1 - Bug] The help-screen acceptance grep cannot match the file's format**

- **Found during:** Task 1 verification
- **Issue:** The criterion is `grep -c "'r'\|'x'\|'o'" src/ui/screens/help.rs`, which requires **literal single quotes** around the key characters. `help.rs` renders a plain text key list — `Line::from("  d             Delete project")` — with no quoting anywhere in the file. The criterion returns `0` regardless of whether the bindings are documented, and the only way to satisfy it literally would be to abandon the file's existing format, which the same task's action text explicitly requires preserving ("in the file's existing format and grouping"). The two instructions are in direct conflict.
- **Fix:** No code change; the format instruction wins, being the one that describes an observable property of the result. Evaluated in a corrected form that matches the file's actual shape: `grep -cE '^ *Line::from\("  (r|x|o) ' src/ui/screens/help.rs` outputs `4` — the three new bindings plus the pre-existing detail-view `r`, all four verified by eye at lines 54, 55, 56 and 58.
- **Files modified:** none
- **Verification:** the three new lines printed with their line numbers and confirmed present.
- **Committed in:** n/a (verification-only)

**3. [Rule 1 - Bug] Three Task 2 greps count comments and the plan's own test name**

- **Found during:** Task 2 verification
- **Issue:** Three criteria are unsatisfiable as literally written, each for a different reason. (a) `grep -c 'windows(2)' src/app.rs == 0` — the raw count is `1`, on a **comment** at line 716 explaining that the arm calls the shared function "rather than re-implementing its `windows(2)` filter". Deleting the sentence to satisfy the grep would remove the note that stops the duplicate coming back. (b) `grep -c 'fn seq_gaps' src/journal/reader.rs == 2` — the raw count is `3`, because the plan's **own named test** is `fn seq_gaps_from_zero_matches_the_unseeded_gap_check`. (c) `grep -c 'windows(2)' src/journal/reader.rs == 1` — the raw count is `2`, the second being inside `a_gap_that_straddles_two_tail_reads_is_detected`, where the test asserts the within-batch check sees **nothing** across `[4, 5]`. That assertion is what makes the test discriminating; removing it to satisfy a grep would remove the reason the test is not vacuous. This is wave 5's recorded lesson in a new costume: a negative grep whose scope includes the test module constrains the test module too.
- **Fix:** No code change; all three properties hold. Evaluated in corrected forms: (a) comment-filtered, `grep -v '^[[:space:]]*//' src/app.rs | grep -c 'windows(2)'` → `0`; (b) `grep -c '^pub fn seq_gaps' src/journal/reader.rs` → `2`; (c) restricted to the non-test region, → `1`.
- **Files modified:** none
- **Verification:** all three corrected forms run and recorded above.
- **Committed in:** n/a (verification-only)

**4. [Rule 1 - Bug] The leak-check bracket guard defeated itself**

- **Found during:** Task 3
- **Issue:** Wave 6 recorded that `pgrep -f 'fake-claude'` matches the shell running it, and prescribed the bracket form `fake-claud[e]`. The first run of the corrected form **still** reported three hits — because the same command line also contained `echo "fake-claude EXIT=$?"`, and that unbracketed literal is exactly what the bracketed regex matches. The guard only works if the literal appears nowhere else on the line, including in the criterion's own diagnostics.
- **Fix:** Re-run as `pgrep -af 'fak[e]-claude' | wc -l` with no literal anywhere else. Output `0`. Worth carrying forward: a bracket guard is not a property of the pattern alone but of the whole command line.
- **Files modified:** none
- **Verification:** recorded in section 6 above.
- **Committed in:** n/a (verification-only)

**5. [Rule 1 - Bug] `pgrep -x claude` is unsatisfiable on this machine**

- **Found during:** Task 3
- **Issue:** The criterion requires `pgrep -x claude` to return nothing after the suite. It returns two pids. Neither is a leak: their elapsed times are **1d 03h 08m** and **21h 37m**, both long predating this session, and one of them is the Claude Code process running this executor. The suite cannot leak a `claude` in any case — it runs entirely against checked-in shell fixtures with no subscription and never invokes the real binary.
- **Fix:** No code change. Evaluated in an age-bounded form that is meaningful on a developer machine: `ps -o etimes=,pid=,args= -C claude | awk '$1 < 3600'` → **0 processes younger than an hour**. The two long-lived sessions were identified by `ps -o lstart,etime` and recorded verbatim rather than dismissed.
- **Files modified:** none
- **Verification:** recorded in section 6 above.
- **Committed in:** n/a (verification-only)

**6. [Rule 2 - Missing Critical] Three tests beyond the plan's named list**

- **Found during:** Tasks 1 and 2
- **Issue:** Three gaps where a plausible wrong implementation passes every named test. Nothing exercised the **decline** arms — an implementation whose `n`/`Esc` fell through to the action would pass all three named `driver_confirm` tests while performing the very thing the confirmation exists to prevent. Nothing asserted the stop path dispatches the right message with the right payload, or that a stop is **not** gated on the opt-in record (withdrawing an opt-in stops no live run, so a run started before a withdrawal must remain stoppable — T-17-48). And the retention test's under-the-bound case was unexercised, so a prune that evicted unconditionally would pass the over-the-bound assertion.
- **Fix:** `declining_dispatches_nothing_and_pops` (both keys plus the consume-everything-else arm), `a_stop_dispatches_the_alias_and_nothing_else`, and an under-the-bound arm folded into `pruning_retains_at_most_the_newest_runs_per_alias`.
- **Files modified:** `src/ui/screens/driver_confirm.rs`, `src/app.rs`
- **Verification:** all pass; suite total 519.
- **Committed in:** `64fb523`, `15c477b`

**7. [Rule 3 - Blocking] `.planning/ROADMAP.md` is not this executor's to write**

- **Found during:** Task 3
- **Issue:** Task 3 step 4 instructs marking the Phase 17 plan checklist in `.planning/ROADMAP.md`. This executor runs in a worktree, and its objective states plainly that the orchestrator owns `ROADMAP.md` and `STATE.md` and performs all post-wave shared-file writes. Writing it here would either be discarded on merge or conflict with the orchestrator's own edit.
- **Fix:** `REQUIREMENTS.md` updated (it is explicitly in this executor's commit set); `ROADMAP.md` left untouched and recorded as a handoff in Next Phase Readiness below, with the exact edit named so the orchestrator does not have to derive it.
- **Files modified:** `.planning/REQUIREMENTS.md` only
- **Verification:** `git status` shows no `ROADMAP.md` modification.
- **Committed in:** this commit

**8. [Rule 1 - Bug] The clippy audit's first file path does not exist**

- **Found during:** Task 3
- **Issue:** The plan, the phase context's "Project gates" block, and this executor's own success criteria all name `src/ui/screens/browser.rs` as the file carrying three of the five pre-existing lints. There is no such file — `src/ui/screens/mod.rs` declares no `browser` module — and the lints are in `src/browser.rs`. The count, the module, and the three line numbers all match; only the path is wrong, and it has been carried through every wave of the phase.
- **Fix:** No code change; the five lints are not this phase's to touch. The correct path is recorded in section 1 above so Phase 18 does not re-derive it from a failing check.
- **Files modified:** none
- **Verification:** verbatim clippy output with file paths recorded in section 1.
- **Committed in:** n/a (verification-only)

---

**Total deviations:** 8 (1 unbuildable task file list, 5 criteria unsatisfiable as literally written, 1 shared-file ownership conflict, 1 coverage addition). **Impact:** deviation 1 is the only one that changed what was committed; deviations 2–5 and 8 are all verification-form corrections where the underlying property holds and was checked in a form that can actually observe it. **No scope creep:** the public surface is the plan's symbol list plus `Action::DriverStartRequested`, which the plan's own action text directs. **No dependency added.**

## Issues Encountered

**Five of this plan's grep-shaped criteria could not hold as written, and the pattern across them is worth naming.** Each fails for a different reason — a comment containing the forbidden literal, the plan's own test name matching a production-symbol count, a test assertion that must contain the thing the criterion forbids, a self-matching process pattern, and a check that assumes the developer machine is not running the tool it tests. What they share is that a naive "make the grep pass" response would have **damaged something real** in four of the five: deleting the comment that stops the duplicate returning, renaming the plan's own test, deleting the assertion that makes the gap test discriminating, or weakening the leak check. This is the fifth consecutive wave to record corrections of this shape, which suggests the fix belongs at planning time: a criterion that greps a whole file for a literal is constrained by that file's comments and tests too, and should be written comment-filtered and test-region-scoped from the start.

**`rtk proxy sh -c` for every pipeline, as all six previous waves recorded.** Every criterion that pipes one filter into another was run as `rtk proxy sh -c '<whole pipeline>'`, and every `cargo` invocation needing raw `warning:` / `test result:` lines as `rtk proxy cargo …`. The clippy audit specifically was run with `rtk proxy` wrapping the whole pipeline so the raw lint lines survived the summarising wrapper. No criterion in this plan passed vacuously.

**The environment's Bash guard refuses compound commands from a worktree agent**, so the branch and base assertions and several verification steps were issued as separate plain commands rather than as `&&` chains. No workaround was needed for anything the tests do.

**"Live" is not "recorded" did not bite here**, because this plan's tests are all in-process unit tests over constructed `AppContext`s and a `tempfile` config — no process lifecycle, no `/proc`. Recorded because wave 6 flagged it as a forward hazard and the absence is a property of this plan's shape rather than luck.

## Verification Results

| Check | Result |
|---|---|
| `cargo build` | clean |
| `rtk proxy cargo test` | **519 passed, 0 failed** (baseline 508 after wave 5; +11 this plan) |
| `cargo clippy -- -D warnings` | exits 0 |
| `cargo clippy --all-targets` warning count | **exactly 5**, all pre-existing (`src/browser.rs` ×3, `src/project_creator.rs` ×1, `src/state_reader/mod.rs` ×1); none in any file this plan touched |
| `rtk proxy cargo test --lib ui::screens::driver_confirm` | 5 passed — all three named tests present |
| `rtk proxy cargo test --lib ui::` | 46 passed |
| `rtk proxy cargo test --lib journal::reader` | 16 passed — all three named tests present |
| `rtk proxy cargo test --lib journal::` | 49 passed |
| `rtk proxy cargo test --lib app::` | 23 passed — all 20 pre-existing plus 3 new |
| `rtk proxy cargo test --test journal_crash` | 3 passed, unchanged |
| `grep -c` the three `KeyCode::Char` arms in `normal.rs` | `3`, none appearing twice |
| files added under `src/ui/screens/` | **exactly 1** — `driver_confirm.rs` |
| `git diff --stat src/state_reader/` for this plan | empty — `ProjectState` gained no driver field |
| `grep -c 'Phase 18' src/ui/screens/driver_confirm.rs` | `2` |
| new bindings in `help.rs` (corrected form) | `4` — the three new plus the pre-existing detail-view `r` |
| `grep -c 'windows(2)' src/app.rs`, comments filtered | `0` |
| `grep -c '^pub fn seq_gaps' src/journal/reader.rs` | `2` |
| `grep -c 'windows(2)' src/journal/reader.rs`, non-test region | `1` |
| `grep -c '_poll_counter\|_prune_counter\|interval(' src/app.rs` | `5`, **unchanged** from wave 5, all five `session_poll_counter` |
| `grep -c 'observed_runs\|run_states\|journal_cursors' src/ui/screens/delete_confirm.rs` | `4` |
| `grep -c 'prune_driver_maps' src/registry.rs` | `1` |
| anti-pattern sweep over all 9 touched files | **0 findings** |
| `pgrep -af 'fak[e]-claude' \| wc -l` | `0` |
| `ps -C claude` younger than 1h | `0` |
| `git diff --stat fe2a8a6 HEAD -- Cargo.toml Cargo.lock` | 9 insertions — one `rustix` line plus its comment |
| new `name =` entries in `Cargo.lock` across the phase | `0` |

### The plants, verbatim

**Plant 1** — the opt-in early return removed from `do_start_run`, modelling a screen that shows the message and starts the run anyway:

```
---- starting_a_run_on_a_project_that_has_not_opted_in_sets_an_error_and_spawns_nothing stdout ----
panicked at src/ui/screens/driver_confirm.rs:389:9:
a refused start must dispatch NOTHING — a message plus a dispatch is a run that
starts anyway while telling the user it did not
```

It failed on the **dispatch** assertion, not the message assertion — which is the point of asserting both.

**Plant 2** — the retention sort reversed, modelling a prune that keeps the right count of the wrong runs:

```
---- pruning_retains_at_most_the_newest_runs_per_alias stdout ----
assertion `left == right` failed: the NEWEST ids must survive
  left:  [...T12-00-00Z..., T12-01-00Z, ..., T12-09-00Z]   <-- the ten OLDEST
 right:  [...T12-05-00Z..., T12-06-00Z, ..., T12-14-00Z]   <-- the ten newest
```

The count was correct in both. Only the identity assertion could see the failure.

Both plants were reverted, `grep -c '_planted'` over both files is `0`, and the full suite was re-run green afterwards.

## Known Stubs

None.

Two things that could be mistaken for stubs, both deliberate and both documented at the code:

- **`DEFAULT_DRIVE_COMMAND` is a single hardcoded `/gsd-progress`.** This is a scope fence, not a placeholder: the command picker is Phase 18's and the decision router is Phase 20's, and D-22 already records that the honest command "sequence" for Phase 17 is the single `--command` argument. The const's doc says exactly this.
- **The opt-in toggle is a bare confirmation, not a disclosure flow.** D-26 is explicit that the rich flow — listing every file that will enter prompts, requiring the user to type the project name — is Phase 18's, and that Phase 17 must not block on a flow that has no screen to live in. The module doc names Phase 18 as its owner.

## User Setup Required

None — no external service configuration required. Every test in this plan runs against constructed in-memory state and temporary directories, with no subscription, network, process or quota dependency.

## Next Phase Readiness

**Handoff to the orchestrator (required):** `.planning/ROADMAP.md`'s Phase 17 block still reads `**Plans**: 6/7 plans executed` with `- [ ] 17-07-PLAN.md` unchecked, and the top-level line 91 still reads `- [ ] **Phase 17: Supervisor**`. Both need marking; this executor deliberately did not write that file (deviation 7). `.planning/REQUIREMENTS.md` **is** updated here and needs no further edit.

- **Phase 18 (Driver tab, live watch, injection)** inherits every seam it needs and one explicit debt. The seams: `Action::DriverStartRequested` already carries a `command` field, so the command picker fills a value rather than adding a field; `AppContext.observed_runs` and `journal_cursors` are populated, bounded and pruned; and `driver_confirm.rs`'s module doc enumerates exactly what Phase 18 owns, so the fence is a checklist rather than a memory. The debt: `DEFAULT_DRIVE_COMMAND` is the one line a picker replaces, and `DriverConfirmScreen` is the confirmation a richer opt-in disclosure flow (D-26, PITFALLS:511) should grow out of rather than sit beside.
- **Phase 18 (orphan sweep)** — unchanged from wave 6's handoff: `StopOutcome::ExitedAfterKill` is the signal, the journaled `claude_pgid` is the handle, and `tests/driver_kill.rs::group_members` is a working `/proc` group scan to reuse.
- **Phase 20 (router)** — unaffected by this plan. The `command` field on `DriverStartRequested` is a single value today; a multi-command sequence changes the driver's loop, not this message.
- **A carry-forward for whoever plans next.** Five of this plan's acceptance criteria were unsatisfiable as literally written, and every previous wave in this phase recorded at least one of the same class. The concrete fix is at planning time: write file-scoped negative greps comment-filtered and test-region-scoped, and never write a `pgrep` criterion whose pattern can appear on its own command line.

## Requirements Traceability

`CTRL-01` through `CTRL-05` are all marked complete in `.planning/REQUIREMENTS.md` by this plan, with both the checkbox list and the Traceability table's Status column updated. Section 4 above names, for each requirement, the plan that delivered it and the criterion row that evidences it, and every one of those five criterion rows was re-run in this session.

`CTRL-04` is declared by both 17-05 and this plan; 17-05 deliberately left it unmarked because a sibling that also claimed it was unwritten. That sibling is this plan, it is now written, and its `x`/`r` keys are what make the reconciled run reachable by a user — so marking it here closes the gap 17-05 identified rather than papering over it.

## Self-Check: PASSED

- Created file verified present on disk: `src/ui/screens/driver_confirm.rs`.
- Both task commits verified present in `git log`: `64fb523`, `15c477b`.
- All Task 1 and Task 2 acceptance criteria re-run after the Task 2 commit; all pass, with the five unsatisfiable criteria evaluated in their corrected forms and every correction documented above.
- Plan-level verification re-run: build clean, **519 tests passing** (baseline 508), zero failures, `cargo clippy -- -D warnings` exits 0, `--all-targets` warning count still exactly 5 and all pre-existing, `Cargo.toml` and `Cargo.lock` carrying only the phase's single `rustix` line with zero new lockfile package entries.
- Both plants verified reverted: `grep -c '_planted'` is `0` for `src/ui/screens/driver_confirm.rs` and `src/app.rs`, and the full suite was re-run green afterwards.
- Fixture-leak check re-run after a clean suite: no stand-in, no orphaned grandchild, no agent process younger than the suite.
- No modification to `STATE.md` or `ROADMAP.md` — the orchestrator owns those writes.

---
*Phase: 17-supervisor-detach-kill-switch-dry-run-opt-in-gate*
*Completed: 2026-07-29*
