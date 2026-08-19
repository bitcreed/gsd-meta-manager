---
phase: 20-deterministic-decision-router-run-bounds
plan: "02"
subsystem: journal
status: checkpoint-blocked
tags: [journal, run-record, dry-run, drive-06, ctrl-06, checkpoint]
requires:
  - src/driver/router::decide
  - src/driver/bounds::evaluate
  - src/journal/RunRecord
provides: []
affects: []
tech-stack:
  added: []
  patterns: []
key-files:
  created: []
  modified: []
decisions: []
metrics:
  duration: ~20m (investigation only)
  completed: n/a
actuals:
  tokens: 0
  tasks: 0
  commits: 1
---

# Phase 20 Plan 02: Make the Durable Record Honest — Summary (CHECKPOINT-BLOCKED)

**No implementation was performed.** The plan's first task is a
`checkpoint:decision` rated `one-way`, and every subsequent task's content
depends on which option is selected. Execution stopped at the gate, as the plan
requires.

## Status

| Task | State |
|---|---|
| Checkpoint: what `RunRecord.gsd_command` means under a sequence | **awaiting human decision** |
| Task 1: classify every singular field; bounds on disk | not started (blocked — writes `gsd_command` per the resolved option) |
| Task 2: repair the pinned dry-run contract and four sibling sites | not started |
| Task 3: make snapshot capture visible to the async-blocking lint | not started |

Working tree is clean; no source file was modified. The only commit this plan
has produced is this summary.

## Investigation Performed Before Stopping

Read-only grounding for the checkpoint recommendation. Three findings materially
change the option trade-offs as the plan states them, and all three are facts
about the tree rather than judgements.

### Finding 1 — option-b is already shipped, in its worst form

`src/driver/run.rs:632-643` (`recorded_command`) writes the string
`"--target-phase {N}"` into `gsd_command` for a routed run, and
`tests/driver_iteration_loop.rs:307` pins that exact value. The field named
*command* therefore already carries an **argv fragment** — not a command, not a
sentinel, not empty. This is the class of quiet lie the plan exists to remove,
and it is on disk for every routed run recorded since 20-01. It renders to the
user as `cmd: --target-phase 3` (`src/ui/screens/driver.rs:806`), which reads as
a pasteable command line and is not one.

The checkpoint is therefore not a greenfield choice. It is a choice about what
to replace an already-wrong value with.

### Finding 2 — the empty string is already taken, which weakens option-c

`src/journal/mod.rs:2140-2143`
(`a_record_from_an_unknown_schema_still_produces_a_row`) pins `gsd_command == ""`
to mean **"the field was absent from the record"** — D-30 tolerance for a record
written by a schema this build has never heard of.

Option-c would overload that same value with "deliberately routed, read the
`Decided` events". A reader — and the existing test — could then not distinguish
a routed run from an old or unparseable record. Option-c's stated con ("a weaker
signal than a named marker") is understated: the signal is not merely weaker, it
is **already assigned a different meaning** in the same field by a passing test.

### Finding 3 — option-a's vocabulary already exists, and its con is smaller than stated

`src/driver/mod.rs:259` already defines
`ROUTED_PREVIEW = "(routed: chosen per iteration by the decision router)"` — a
parenthesised routed-mode marker, shipped by 20-01 for the preview. Option-a
does not invent a sentinel vocabulary; it gives an existing one a single owner.

Option-a's con is "the render layer needs an arm for it". The render layer does
**not** strictly need one: both render sites pass the value through
`sanitize_render_line` as free text (`src/ui/screens/driver.rs:806` and `:1263`),
so a marker renders legibly as `cmd: (routed → phase 3)` with no match arm and
no code change. An arm would *improve* it, not be required by it. This matters
because `src/ui/screens/*` is **owned by plan 20-03 in a parallel worktree** and
is not in this plan's `files_modified` — under option-a this plan can complete
without touching it, and under any option a render improvement is a follow-up.

### Adjacent conflict noted for Task 2 (not part of the checkpoint)

`previewed_command`'s doc at `src/driver/mod.rs:250-256` states that rendering
the router's first command **was declined**, calling it "the same untruth in a
more convincing form". Task 2's action text instructs the opposite: populate the
routed preview with the router's first selection plus a plain statement of its
limit. Task 2's framing appears to be the considered answer to 20-01's
objection — the untruth 20-01 feared comes from presenting one command *as a
sequence*, which the replacement text explicitly refuses. This is resolvable
under the deviation rules when Task 2 runs and needs no separate gate; it is
recorded here so the resolution is not mistaken for an oversight.

## Known Stubs

None — no code was written.

## Deferred Items

The whole of this plan's work, pending the checkpoint answer. Also inherited
from 20-01 and still open, all of which Task 1 is scoped to close:

- `run.json` does not record the bounds in force.
- `dry_run::SECTION_COMMANDS` and its three sibling sites still deny a sequence
  exists (Task 2).
- `RunSnapshot::capture` / `capture_snapshot` visibility to the async-blocking
  guard (Task 3). Note 20-01 already added `"RunSnapshot::capture("` to
  `BLOCKING_HELPERS` with two allowlist entries (20-01 SUMMARY deviation 4),
  which **partially overlaps Task 3's action** — Task 3 must be re-scoped
  against the tree as it now stands rather than as the plan assumed it, since
  the plan expects to add two markers and exactly one allowlist entry.

## Verification

Not applicable — no code changed. No build, test or clippy gate was run, and
none is claimed.

## Self-Check: PASSED

Files created: `.planning/phases/20-deterministic-decision-router-run-bounds/20-02-SUMMARY.md` (this file).
Source files modified: none, confirmed by a clean `git status --short` before commit.
