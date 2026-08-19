---
phase: 20-deterministic-decision-router-run-bounds
plan: "02"
subsystem: journal
status: complete
tags: [journal, run-record, dry-run, drive-06, ctrl-06, serde-migration, guard]
requires:
  - src/driver/router::decide
  - src/driver/bounds::resolve
  - src/state_reader::parse_project_state
  - src/journal/RunRecord
provides:
  - src/journal::RecordedBounds
  - "RunRecord::target_phase, RunRecord::bounds, RunRecord::extra"
  - src/driver::ROUTED_RECORD_MARKER
  - src/driver/dry_run::build_routed_report
  - src/driver/dry_run::PreviewScope
  - src/driver/dry_run::RoutedPreview
affects:
  - src/journal/mod.rs
  - src/journal/writer.rs
  - src/driver/run.rs
  - src/driver/mod.rs
  - src/driver/dry_run.rs
  - src/driver/reconcile.rs
  - src/envelope/mod.rs
  - tests/async_blocking_guard.rs
  - tests/driver_dry_run.rs
  - tests/driver_iteration_loop.rs
tech-stack:
  added: []
  patterns:
    - "per-field scope classification enforced by a struct-body-parsing test with a non-vacuity floor"
    - "old-schema fixture as a raw byte literal, never built from the current struct"
    - "#[serde(flatten)] overflow map for cross-version round trips (mirrors RegisteredProject::extra)"
    - "scope carried on a wrapper type so the common path keeps its signature"
    - "control-arm test for the guard's own non-vacuity floor (guard-of-the-guard-of-the-guard)"
key-files:
  created: []
  modified:
    - src/journal/mod.rs
    - src/journal/writer.rs
    - src/driver/run.rs
    - src/driver/mod.rs
    - src/driver/dry_run.rs
    - src/driver/reconcile.rs
    - src/envelope/mod.rs
    - tests/async_blocking_guard.rs
    - tests/driver_dry_run.rs
    - tests/driver_iteration_loop.rs
    - tests/envelope_wiring.rs
    - tests/journal_gitignore.rs
    - tests/journal_run_paths.rs
decisions:
  - "gsd_command carries a named routed marker (option-a); target_phase is a typed sibling"
  - "argv_digest is split off recorded_command so a routed digest still discriminates by target"
  - "PreviewScope rides RoutedPreview, not DryRunReport, keeping build_report and render signatures intact"
  - "ROUTED_PREVIEW deleted; exactly one routed literal now exists in the tree"
  - "capture_snapshot( deliberately NOT added to BLOCKING_HELPERS — it already hands off"
metrics:
  duration: ~3h
  completed: 2026-08-19
actuals:
  tokens: 6800
  tasks: 3
  commits: 4
---

# Phase 20 Plan 02: Make the Durable Record Honest Summary

`run.json` now states which scope every one of its fields belongs to, the caps
the run actually ran under, and — for a routed run — a marker that cannot be
mistaken for a command, with the phase it drove toward in a field whose name
says so. No user-facing statement left in the tree survives that 20-01 made
false.

## Checkpoint Decision (one-way) — Rationale of Record

The plan's first task was a `checkpoint:decision` rated one-way, because
`run.json` carries **no version discriminator**: changing what a field means
after records exist reinterprets every record already on disk rather than
migrating it. Execution stopped, the blocked state was committed (`12a377f`),
and the coordinator resolved it as **option-a** — a named routed-mode sentinel
in `gsd_command` plus a typed `target_phase: Option<String>`.

The rationale, recorded here because a one-way decision must stay auditable:

- **Option-b (record the goal text) was rejected because it was not
  hypothetical.** `recorded_command` (`src/driver/run.rs:632-643`) already wrote
  the argv fragment `"--target-phase {N}"` into `gsd_command`, pinned by
  `tests/driver_iteration_loop.rs:307`, and rendered it to the user as
  `cmd: --target-phase 3` (`src/ui/screens/driver.rs:806`) — a pasteable command
  line that is not one. Choosing b would have ratified the exact quiet lie this
  plan exists to remove.
- **Option-c (empty string) was rejected because `""` is already taken.**
  `src/journal/mod.rs:2140-2143`
  (`a_record_from_an_unknown_schema_still_produces_a_row`) pins `""` to mean
  *"the field was absent from the record"* under D-30, with the passing message
  `"an absent field is empty, not fatal"`. Overloading it would make a routed
  run indistinguishable from an old or unparseable one. The plan listed this con
  as "a weaker signal"; it is stronger than that, and the coordinator recorded
  this as the decisive finding.
- **Option-a's stated cost — a render-layer arm — is optional, not required.**
  Both render sites pass the value through `sanitize_render_line` as free text,
  so a marker renders legibly with **no change to `src/ui/screens/*`**. That
  mattered operationally: those files belong to plan 20-03, running in a
  parallel worktree, and this plan did not touch them (verified below).

## What Was Built

**Per-field scope classification on `RunRecord`.** All thirteen pre-existing
fields plus the three new ones state, in their own doc, whether they are
**run-scoped** or **iteration-scoped** now that a run issues a sequence. Two are
iteration-scoped and say which iteration they name: `session_id` and
`claude_code_version` are both the **first** iteration's, because they are
written at write one and nothing overwrites them.

`run_record_fields_all_declare_their_scope` parses the struct body out of the
module's own source (the `tests/spawn_seam_guard.rs` precedent — a doc comment is
invisible at runtime, so the only way to assert a field *documents* something is
to read the text) and fails on any declaration whose doc carries neither word. It
carries a `>= 10` non-vacuity floor. **Verified to fire**: temporarily replacing
`target`'s "Run-scoped." with an unclassified word produced
`These declare neither ... : ["target"]`, then reverted.

**The caps in force, on disk.** `RunRecord::bounds: Option<RecordedBounds>`
records the resolved `max_steps` and `wall_clock_cap_secs`, sourced from
`bounds::resolve` rather than from the constants. `bounds::resolve` moved to
*before* the record is built and the loop now uses that same value — previously
it was called twice on the reasoning that a pure function cannot disagree with
itself. That was true but is no longer sufficient: "the caps on disk are the caps
the loop enforced" is now a property held by construction rather than by
re-deriving and trusting purity.

`RecordedBounds` is a record-side mirror rather than `bounds::RunBounds` itself,
because that type carries a `Duration`, which serde renders as
`{"secs":…,"nanos":…}` — a shape nothing reading this file wants.

**`extra`, the flattened overflow.** T-20-08's mitigation: two binary versions
share this committed file, and an older build loading and re-saving a newer
record would otherwise delete every field it cannot model. It complements rather
than replaces the `Value`-based read paths, which exist for a different reason
(one *added* field must not make older runs invisible).

**The park-reason taxonomy doc.** `JournalEvent::Parked.reason` now names all
three sanctioned taxonomies that ride the one field — envelope `ParkReason`
(unprefixed, staying at seven arms), `RouterReason` (`router_`), `BoundsReason`
(`bounds_`) — in a table with their on-disk prefixes. The doc previously named
one while a sibling's strings rode the same field.

**An honest dry-run preview.** `SECTION_COMMANDS` no longer claims the single
`--command` is the complete sequence. A routed preview renders the router's
**own first selection** and states plainly that the run continues past it, with
`PreviewScope` (three arms, no unclassified one) carrying what the command list
is a complete answer *to*. A routed run whose router would park says so and names
the reason from the router's own taxonomy.

## Key Decisions

**The argv digest was split off `recorded_command`.** The record's routed marker
is a constant, so digesting it would collapse every routed run against every
target to one digest — destroying the only thing `argv_digest` promises, which is
telling two runs with different command lines apart. `digested_command_fragment`
digests the driver's real argv (`--target-phase 3`); `recorded_command` answers
the different question of what command the run issued (for a routed run: none).

**`PreviewScope` rides `RoutedPreview`, not `DryRunReport`.** Command mode is
`Complete` by construction, so a field on the common path would be constant
there and would oblige every existing constructor to name it. The wrapper also
keeps `build_report`'s and `render`'s signatures byte-identical — which is what
let this land **without touching `src/ui/screens/*`**, the files plan 20-03 owns
in a parallel worktree. The first design did add the field, and it broke
`src/ui/screens/mod.rs:1136` and `src/ui/screens/driver.rs:3284`; the wrapper was
adopted specifically to honour the isolation constraint rather than to negotiate
it.

**`ROUTED_PREVIEW` was deleted rather than kept as a second owner.** The
coordinator's constraint was one owner per routed literal and no third literal.
Because the preview now has a real command to show, its marker had no consumer
left — so the tree is left with **exactly one** routed literal,
`ROUTED_RECORD_MARKER`, which is the strongest form of that constraint.

**The marker is asserted un-misreadable, not merely equal to its literal.**
`the_routed_record_marker_cannot_be_misread_as_absent_or_as_an_argv_fragment`
pins that it is non-empty (`""` means absent), contains no `--` (not an argv
fragment), starts with neither `/` nor `-` (not a command, not a flag), and names
the `decided` records so it points somewhere rather than only saying what is
missing.

## Deviations from Plan

### Corrected plan premises (documented in-source rather than restated)

**1. [Rule 1 - Bug] `Decided` carries no `session_id`.** The plan's `must_haves`
and Task 1 action both state that *"per-iteration session ids ride the `Decided`
and `ExecStarted` journal events, which already carry them"*. `JournalEvent::Decided`
carries `by`, `command` and `rationale` — no session id, because the decision
precedes the spawn that creates one. Writing the plan's sentence into
`session_id`'s doc would have been a fresh false statement in the very field the
task exists to make honest. The doc names `exec_started` alone and says why
`decided` does not qualify. **Commit:** `5c0c202`.

**2. [Rule 1 - Bug] The two-iteration fixture would have proved the bounds
assertion by coincidence.** Task 1's acceptance asks for a test that *"asserts a
routed run's `run.json` names a step cap that differs from the compiled-in
default when one was supplied on argv"*. The existing end-to-end fixture passed
`max_steps: None`, so the recorded cap would have equalled `DEFAULT_MAX_STEPS`
and the assertion could not have distinguished a resolved value from a constant.
It now supplies `EXPLICIT_STEP_CAP = 5` (high enough that command-repeat still
fires first, so the run's terminal reason is unchanged) plus an `assert_ne`
against the default. **Commit:** `5c0c202`.

**3. [Rule 3 - Blocking] Three of Task 2's "four sibling sites" were already
discharged by 20-01.** The plan names the driver module-doc numbered point, the
`DriveArgs::command` field doc, the CLI command-flag comment and the test named
for the single-command shape. All four were checked against the tree: the first
three already state what is true, and the test was already renamed to
`drive_args_carry_exactly_one_command_source_and_never_a_supplied_sequence`
(20-01 SUMMARY deviation 2). Only `src/driver/dry_run.rs` was genuinely stale, in
three places — module-doc point 1, `SECTION_COMMANDS`, and
`DryRunReport::commands`. **No change was manufactured to match the plan text.**
**Commit:** `52ca9b5`.

**4. [Rule 3 - Blocking] Task 3's marker/allowlist delta is EMPTY.** Flagged
before execution and confirmed by the coordinator. The plan asks for two new
snapshot-capture markers and exactly one new allowlist entry; 20-01 already added
`RunSnapshot::capture(` and both join-failure allowlist entries (its deviation
4). The second marker the plan implies, `capture_snapshot(`, must **not** be
added: both spellings (`src/executor/claude.rs:747`, `src/driver/run.rs:1401`)
are `async fn`s that hand the work to `spawn_blocking` internally, so naming it
would report three already-correct call sites and the only route back to green
would be three allowlist entries suppressing non-problems — the silent widening
CTRL-06 forbids. This is asserted as a **negative** in
`the_snapshot_capture_marker_covers_both_of_the_trees_inline_captures`, so the
finding is enforced rather than only written down. **Commit:** `0a2f9a2`.

Note the plan's related prohibition holds and was verified: no allowlist entry
exists for the driver's *per-iteration* capture. Both entries are for inline
join-failure fallbacks, which is a different thing.

### Auto-fixed issues

**5. [Rule 2 - Missing critical functionality] The Task 2 refactor blinded the
async guard, and the guard caught it.** Moving the preview behind `preview_text`
made the entry `("src/driver/mod.rs", "build_report(")` stop suppressing
anything, failing `no_allowlist_entry_is_stale`. The blocking work had not gone
anywhere — only its name had — so deleting the entry would have left the inline
join-failure fallback unreported. `preview_text(` and `build_routed_report(` were
added to `BLOCKING_HELPERS` and the allowlist marker moved, in the same commit as
the code that moved it. **Commit:** `52ca9b5`.

**6. [Rule 2 - Missing critical functionality] Routed previews needed their own
zero-write and zero-spawn proofs.** A routed preview reads `.planning/` and calls
the router, neither of which the command-mode path does, so the D-23 proofs
cannot be inherited. `a_routed_dry_run_also_leaves_the_git_directory_byte_identical`
re-runs the reflog + refs + full `.git` listing fingerprint, and
`a_routed_dry_run_spawns_no_agent` re-runs the tripwire program that leaves
evidence if it is ever executed (T-20-11). **Commit:** `52ca9b5`.

**7. [Rule 1 - Bug] Two tautological assertions in the floor control arm.**
Clippy flagged `MIN_ASYNC_BODY_LINES > 0` as constant-valued; `real < real + 1`
proved arithmetic rather than the scanner. Both replaced with the measured
property they were meant to express: the broken-walk measurement falls below the
floor and the real tree clears it, so the floor sits strictly between them.
**Commit:** `0a2f9a2`.

## Known Stubs

None introduced by this plan. Two pre-existing ones are now *covered* rather than
resolved:

| Item | File | Status |
|---|---|---|
| `router::Decision::GoalMet` has no producer | `src/driver/router.rs` | Inherited from 20-01. `build_routed_report` spells the arm out rather than wildcarding it, so adding a producer is a decision at that site rather than a silent fall-through to "would park". |
| `RouterReason::DependencyUnsatisfied` has no producer | `src/driver/router.rs` | Inherited from 20-01, untouched here. |

## Deferred Items

- **Per-iteration `argv_digest`** (inherited from 20-01). `run.json` still carries
  one digest. It is now explicitly the *driver's own* argv and its doc says so;
  per-iteration variation remains on the `decided` records.
- **`DiskStatus` has no `Executed` variant** (research Pitfalls 1-2), inherited
  from 20-01 and owned by the plan that extends the reader.
- **`tests/driver_reattach.rs` flakiness** remains phase 19's deferred item; see
  Verification below for the measurement taken here.

## Verification

| Gate | Result |
|---|---|
| `rtk proxy cargo build` | clean |
| `rtk proxy cargo test` | **27 suites, 0 failures** on the final run. 864 lib tests. |
| `cargo clippy -- -D warnings` (documented gate) | clean |
| `rtk proxy cargo clippy --all-targets -- -D warnings` | exactly the **5** pre-existing lints recorded in `TESTING.md`, at the same locations (`browser.rs:131-133`, `project_creator.rs:146`, `state_reader/mod.rs:258`) |
| `rtk proxy cargo test --lib journal` | 94 passed |
| `rtk proxy cargo test --lib dry_run` | 11 passed |
| `rtk proxy cargo test --lib driver` | 197 passed |
| `rtk proxy cargo test --test driver_dry_run` | 7 passed (4 pre-existing + 3 new) |
| `rtk proxy cargo test --test async_blocking_guard` | 9 passed (7 pre-existing + 2 new), including `no_allowlist_entry_is_stale` |
| `rtk proxy cargo test --test driver_iteration_loop` | 5 passed |

Every `rtk proxy` is deliberate: `rtk` strips cargo's `warning:` and
`test result:` lines, so a grep of those against bare `cargo` succeeds vacuously.

**`driver_reattach` investigated rather than assumed.** Its two tests failed
intermittently during this work. The base commit `9b2ab7b` was extracted clean
with `git archive` into a scratch directory, with **none** of this plan's changes
present, and `cargo test --test driver_reattach` there failed **4 runs out of 4**
— twice with both tests failing. This tree passes all three on clean runs and
passed the full suite green on the final run. The two failing names are exactly
the pair recorded in phase 19's `deferred-items.md`
(`a_fresh_scan_finds_the_orphaned_run_live_with_its_last_journal_step`,
`a_run_killed_without_an_ending_is_reported_crashed_and_nothing_on_disk_is_repaired`),
with the same `left: 0` assertion. Pre-existing, and not worsened here — the
`bounds::resolve` call this plan moved earlier is a pure function and does not
delay the first `run.json` write.

**Isolation from plan 20-03 verified.** `git diff --stat` over all four commits
touches 13 files; **none** is under `src/ui/screens/`, `src/state_reader/`,
`src/driver/router.rs` or `tests/state_reader_test.rs`. `STATE.md`, `ROADMAP.md`
and `WINDOWS.md` are untouched, per the worktree contract.

## Self-Check: PASSED

Files verified present and modified: `src/journal/mod.rs`, `src/driver/run.rs`,
`src/driver/dry_run.rs`, `src/driver/mod.rs`, `tests/async_blocking_guard.rs`,
`tests/driver_dry_run.rs`, `tests/driver_iteration_loop.rs`.
Commits verified in `git log`: `12a377f`, `5c0c202`, `52ca9b5`, `0a2f9a2`.

## Notes on `actuals`

`tokens: 6800` is chars/4 over the **realized diff** — 27,034 characters across
13 files — against an estimate of 40,000. The plan over-estimated by ~6x, and the
reason is legible rather than mysterious: three of Task 2's four sibling sites
and the whole of Task 3's marker/allowlist delta had already been done by 20-01,
so roughly half the planned edits were verification that the tree was already
correct. That verification was not free — it is most of the reading cost — but it
produced no diff, and recording the smaller honest number is what keeps the next
estimate calibrated. The alternative reading (chars/4 over every file touched)
gives ~145,000, inflated by the two large files (`journal/mod.rs`, `run.rs`) this
plan edits heavily but which were already large.
