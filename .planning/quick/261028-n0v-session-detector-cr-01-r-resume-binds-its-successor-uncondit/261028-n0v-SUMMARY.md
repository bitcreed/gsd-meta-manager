---
quick_id: 261028-n0v
phase: quick-261028-n0v
plan: 01
subsystem: session_detector
tags: [session-detector, claude-cli-arity, round-trip-certificate, R13-CR-01]
status: complete
commits: 2
plan_head_before: 5a5175c09df7b73e27a692c203b6ea2d9949f3d3
plan_head_after: 30dcf32843000de338ad423cbd9947ece2043e3b
key-files:
  modified:
    - src/session_detector.rs
    - src/ui/screens/detail.rs
    - .planning/todos/completed/2026-10-24-session-detector-resume-optional-arity.md
  deleted:
    - .planning/todos/pending/2026-10-24-session-detector-resume-optional-arity.md
completed: 2026-10-28
---

# Quick 261028-n0v: Resume-rank successor follows the option's optional arity

`session_id_in_cmdline` now refuses an option-shaped successor (first byte `-`, longer than one byte) of a bare `--resume`/`-r` and re-examines it as an element in its own right; `--session-id` still binds its successor unconditionally.

## Commits

- `3103f5e` fix: parser change, `WireForm.optional_successor`, option-shaped oracle branch, reach floor, partitioned SPLIT arm, renamed pinning test, census re-measure.
- `30dcf32` docs: CORRECTED blocks, todo moved (git rename) to `completed/` with a Resolution section.

## Step 0: probes re-measured at `claude` 2.1.312 (stdin `/dev/null`, from /tmp)

```
claude --version              -> 2.1.312 (Claude Code)                              exit 0
claude --resume --version     -> 2.1.312 (Claude Code)                              exit 0
claude -r --version           -> 2.1.312 (Claude Code)                              exit 0
claude --resume -h            -> Usage: claude [options] [command] [prompt]         exit 0
claude --session-id --version -> Error: Invalid session ID. Must be a valid UUID.   exit 1
claude -r -5 --version        -> 2.1.312 (Claude Code)                              exit 0 (non-discriminating)
claude --bogus-opt --version  -> 2.1.312 (Claude Code)                              exit 0
  -r, --resume [value]   /   --session-id <uuid>
```
Precondition met; all probes agreed with the plan.

## RED then GREEN

RED (tests only, before the parser change): 5 of 26 failed, for the intended reasons:
- `an_option_shaped_successor_is_bound_by_session_id_and_refused_by_resume`: left `Some("-h")`, right `None`.
- `a_session_id_survives_the_round_trip_in_both_wire_forms`: SPLIT arm, option-shaped id `"-h"` bound as the value.
- `the_round_trip_property_holds_for_every_generated_byte_string`: `[bound-an-option-shaped-successor-as-the-value] 1196 occurrence(s)` (split and short-split).
- `the_generator_reaches_...`: `wire form "split" read back 0 option-shaped, valid, non-empty inputs as something other than the input itself`.
- `every_claude_argv_option_site_under_src_is_adjudicated`: 31 sites vs 23.

GREEN (after parser fix and census update): `test result: ok. 26 passed; 0 failed`. Full `cargo test --no-fail-fast`: every `test result:` line `0 failed` (lib: 2528 passed, 1 ignored). Clippy `--all-targets -D warnings` clean. `cargo fmt --check`: only the pre-existing `detail.rs:14890` hunk (sibling n10).

## Census re-measure (`CLAUDE_ARGV_SITES`)

`src/session_detector.rs`: 23 -> 31 (measured from the test failure message). `src/ui/screens/detail.rs` stayed 6; `git_ops.rs` stayed 1.

## Inferred decisions `[inferred — cleanup 2026-10-28]`

- I1 refused successor not stepped over; I2 single `-` stays a value; I3 negative-number-shaped successor counts as option-shaped (residual: under-detection, silent, if `claude` binds it); I4 split forms get their own oracle branch; I5 pinning test renamed; I6 todo closed into `completed/`; I7 `optional_successor` is the per-option arity seam. All implemented as planned.

## Deviations from Plan

- Minor, no rule triggered: the plan's `<action>` item (c) described "one exemplar per (class x wire form)", but the existing collector keys by class only. I corrected the doc to "one exemplar per violation class" with the new bound of six classes rather than changing the collector.
- Tasks 1 and 2 each committed once; the optional two-commit test/fix split inside Task 1 was not used (the RED transcript is captured above).
- Committed on `master` per the orchestrator's isolation-none instruction.
- A stray `scratch_full.txt` test-output file written into the repo root was removed before any commit.

## Known Stubs

None.

## Threat Flags

None.

## Self-Check: PASSED

- `3103f5e` and `30dcf32` are in HEAD history; the pending todo path is gone and the completed copy has one `## Resolution`.
- Old pinning-test name appears 0 times under `src/`; `fn an_option_shaped_successor_is_bound_by_session_id_and_refused_by_resume(` appears once; `Invalid session ID` and `optional_successor` present; `CORRECTED 2026-10-28` appears in both source files.
