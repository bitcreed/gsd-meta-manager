---
created: 2026-10-25T00:00:00.000Z
title: Round-trip certificate for session ids omits the --session-id spellings the executor emits
area: session_detector
severity: minor
files:
  - src/ui/screens/detail.rs:22286
  - src/session_detector.rs:365-374
---

## Problem

`session_id_in_cmdline` recognises six spellings (`--resume=<id>`, `--resume <id>`, `-r<id>`, `-r <id>`,
`--session-id=<id>`, `--session-id <id>`), but `wire_forms()` in `src/ui/screens/detail.rs` registers four rows
(`fused`, `split`, `short-split`, `short-attached`). `the_round_trip_property_holds_for_every_generated_byte_string`
and the non-vacuity floor both iterate `wire_forms()`, so no whitespace-padded, ill-formed-UTF-8, option-shaped or empty
value is ever driven through either `--session-id` spelling. That is the form `executor::claude::build_argv` emits on
every driver-launched run. Nothing couples the parser's spelling set to `wire_forms()`'s row count, so a spelling taught
to the parser without a row silently narrows the certificate while every gate stays green.

Re-checked at HEAD on 2026-10-25: `wire_forms()` still has exactly four rows. This was Phase 21's CR-02
(21-VERIFICATION, `status: failed`). Phase 31 did not close it as superseded: the parser and the test survive the
goal-layer retirement. It is a backlog item, not a resumption of Phase 21's gap loop.

## Solution direction

Add two rows to `wire_forms()` for `--session-id=<id>` and `--session-id <id>` (the generated property and the floor
then cover them with no further change), and add a control that fails when the parser's recognised-spelling set and
`wire_forms()` disagree in count. Decide the oracle for option-shaped values per option, since `--session-id` binds a
required value and `--resume`/`-r` an optional one (see the CR-01 todo
`2026-10-24-session-detector-resume-optional-arity.md`); the two should land together.
