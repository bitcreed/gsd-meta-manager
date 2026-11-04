---
quick_id: 261103-p5n
status: complete
commit: 7545a63
---

# Summary: 261103-p5n

Appended one sentence to docs/TESTING.md "### Faking external tools":
"The one-off session-lifecycle and meter tests use no fakes; they replay the recorded streams in
`tests/fixtures/oneoff/` (lifecycle via `include_str!`, meter by path)."

## Claim verification
- src/oneoff/session.rs:1021-1022 defines `BG_RUN`/`RESUMED_RUN` with `include_str!` of
  tests/fixtures/oneoff/real-bg-subagents.stream.jsonl and real-resumed.stream.jsonl. The
  lifecycle replay tests (`replay_run`, `the_recorded_*_run_closes_*`) use them.
- The meter test `the_recorded_background_run_meters_to_its_reported_total` lives in
  src/oneoff/stream.rs:1056 and reads the fixture **by path** (`CARGO_MANIFEST_DIR` +
  `meter_of_file`), not via `include_str!`. The requested wording was therefore adjusted.
  [inferred — audit]
- Neither test module uses `launcher_support`; both are in-crate unit tests.

## Decisions
- Kept one sentence (semicolon plus a parenthetical) so that both loading mechanisms are stated
  accurately. [inferred — audit]
- The second wrapped line is 102 columns; the paragraph already has a 103-column line. [inferred — audit]
- Ran inline without planner/executor subagents because the task is trivial. [inferred — audit]
