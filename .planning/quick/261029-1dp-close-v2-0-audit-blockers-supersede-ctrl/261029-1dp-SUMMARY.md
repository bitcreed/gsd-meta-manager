---
phase: quick-261029-1dp
plan: 01
subsystem: launcher, requirements records
tags: [safe-04, redaction, supersession, audit-closure]
status: complete
requirements-completed: [SAFE-04, CTRL-02, CTRL-03, TRANS-01, TRANS-04, TRANS-05, STEER-02, CTRL-05]
key-files:
  modified:
    - src/launcher/process.rs
    - src/launcher/run.rs
    - src/config.rs
    - src/registry.rs
    - src/executor/mod.rs
    - .planning/REQUIREMENTS.md
    - .planning/PROJECT.md
    - .planning/v2.0-MILESTONE-AUDIT.md
  deleted:
    - src/executor/runtime.rs
  created:
    - .planning/todos/pending/2026-10-29-driverctl-log-values-unredacted.md
metrics:
  tasks: 3
  commits: 3
  completed: 2026-10-29
actuals:
  tasks: 3
  commits: 3
plan_head_before: 002b2f03581684c7ba515cba8e77adf11fe9cff4
plan_head_after: 0d18fe9
---

# Phase quick-261029-1dp Plan 01: Close v2.0 audit blockers Summary

Master stdout/stderr now passes through a line-buffered redacting pump before it reaches `launch-N.jsonl`, the dead `AgentRuntime` key is gone, and the requirement, decision and tracking records match HEAD.

## Commits

- f4cfa01 fix: redact master stdout before it reaches launch-N.jsonl (SAFE-04)
- ce86206 refactor: drop dead AgentRuntime key; correct opt-in gate claims
- 0d18fe9 docs: supersede CTRL-02/03, TRANS-01/04/05, STEER-02; SAFE-04 satisfied; 28.1 probe closed

## Items A-F

- **A** done (user decision): CTRL-02/CTRL-03 superseded; verbatim dated entry in REQUIREMENTS.md, PROJECT.md (Constraints and Key Decisions), 17-CONTEXT.md note and 31-CONTEXT.md subsection. Three stale code comments fixed (`registry.rs` `is_opted_in`, `ui/screens/mod.rs` `experimental`, `ui/screens/detail.rs` test doc and assert; fn name kept). The sweep of `src/` and `docs/` found no other sentence claiming the record gates a launch.
- **B** done: TRANS-01 -> LNCH-01, TRANS-05 -> DTAB-04, STEER-02 -> DTAB-03/PRMT-04 (list and traceability).
- **C** done: `spawn_launch_child` now returns `(Child, CaptureHandle)`; a `std::io::pipe` feeds a pump thread that redacts whole lines only (`redact_value` for JSON, `redact` for text), replaces over-cap lines (`MAX_STREAM_LINE`, moved to process.rs) with a fixed note, and keeps draining after a write error. `run.rs` waits for the pump (`CAPTURE_FINISH_BUDGET` 5 s) before `summarize_stream_file`. SAFE-04 marked Complete.
- **D** done, `[inferred — cleanup 2026-10-28]`: CTRL-05 accepted-partial, carried to the next milestone (lock is per run dir, `driver/lock.rs:85`, `launcher/run.rs:667`). TRANS-04 superseded by LNCH-07 from evidence (`host.rs:40`, `:286-307`).
- **E** done `[inferred — 2026-10-29]`: removed `src/executor/runtime.rs`, `RegisteredProject::runtime()`, `Preferences::default_runtime()`; registry census re-measured 5 -> 4 subjects. `runtime` / `default_runtime` remain in the flattened `extra` maps and round-trip (the preferences.rs round-trip test passes). Bail-out did not fire. Also reworded stale docs in `config_json.rs`, `agents/adapters/mod.rs`, `docs/CONFIGURATION.md`, `docs/GSD-CORE-SYNC.md`; marked the item done in the existing executor-remnants todo.
- **F** done: 28.1 D-11 probe recorded as passed 2026-10-28 (7b90d25) in the audit (frontmatter, bookkeeping, Human items), 28.1-VERIFICATION.md, 28.1-SECURITY.md and 31-SECURITY.md (conditional discharged). STATE.md was already correct.

The audit report gained a dated "Gap closure 2026-10-29" section with the expected tally (18 superseded, 71 active, 70 satisfied, 1 accepted-partial; not re-measured). OBS-03 also marked Complete via replacement.

## Tests added (src/launcher/process.rs)

- a_json_line_split_inside_a_token_is_redacted_whole_and_stays_json
- a_text_line_with_a_token_is_text_redacted
- the_oauth_revoked_marker_and_type_survive_redaction_so_the_outcome_is_unchanged
- a_line_over_the_cap_becomes_the_note_and_none_of_its_bytes_are_kept
- a_final_line_with_no_newline_is_redacted_whole_at_eof
- a_write_error_does_not_stop_the_pump_from_draining_its_input
- the_child_stdout_and_stderr_reach_the_file_redacted_even_when_a_token_is_split (process-level, token split across two writes on stdout and on stderr)

RED evidence: with `redact_line` neutralised, 5 of the 7 failed (the other two assert the cap and drain behaviour, not redaction). The tests are written against the new `(Child, CaptureHandle)` API, so a literal fail-before-change run against the old direct-file stdio was a compile failure rather than an assertion failure.

## Gate (run at ce86206; the later commit is docs only)

- `rtk proxy cargo test --no-fail-fast`: 55 test binaries, 3216 passed, 0 failed, 17 ignored (summed from the log in the scratchpad).
- `cargo clippy --all-targets -- -D warnings`: clean. `cargo fmt --check`: clean.

## Deviations from Plan

None requiring a rule. Minor: the `finish()` call sits after `EgressGuard::finish` (still after `stop_leftovers`, before `summarize_stream_file`).

## Known Stubs

None.

## Threat Flags

None. T-261029-06 (driverctl LOG.md values) is accepted and tracked in `.planning/todos/pending/2026-10-29-driverctl-log-values-unredacted.md`.

## Self-Check: PASSED

f4cfa01, ce86206 and 0d18fe9 are ancestors of HEAD; `src/executor/runtime.rs` is deleted; the new todo exists.
