---
phase: 27-driverctl-driver-frontmatter
plan: 03
subsystem: state_reader
tags: [frontmatter, schema, serde_yml, golden-fixtures, driverctl]
requires: []
provides:
  - "crate::state_reader::driver_frontmatter: DriverFrontmatter v1, RunStatus, ItemState, parse_driver_frontmatter, DriverFrontmatter::emit"
  - "golden + spike DIRECTION.md fixtures under tests/fixtures/driverctl/"
affects: [27-04, 27-05, phase-30]
tech-stack:
  added: []
  patterns:
    - "one typed schema for both reading (serde_yml, parse only) and hand-written emission"
    - "order-preserving map newtype with manual MapAccess Deserialize"
key-files:
  created:
    - src/state_reader/driver_frontmatter.rs
    - tests/driverctl_frontmatter.rs
    - tests/fixtures/driverctl/direction/ (18 spike byte copies)
    - tests/fixtures/driverctl/golden/ (7 Rust-emitted goldens)
  modified:
    - src/state_reader/mod.rs
key-decisions:
  - "Emitter lives in state_reader::driver_frontmatter as DriverFrontmatter::emit (pure &self -> String)"
  - "RunStatus::is_terminal covers stopped|done|crashed|stalled|parked (27-CONTEXT)"
  - "U+2028/U+2029 escaped in addition to char::is_control"
requirements-completed: [DCTL-03]
status: complete
duration: ~25min
completed: 2026-10-19
commits: 3
plan_head_before: 1866e0761570f50563adf5822ffa566a4275b868
plan_head_after: e527419df3b4cdff1a83f0ff373e90c4e773f1a1
actuals:
  tokens: 32000
  tasks: 3
  commits: 3
---

# Phase 27 Plan 03: DriverFrontmatter v1 schema, parser and emitter Summary

One typed `DriverFrontmatter` module reads (serde_yml, parse only) and writes (hand-written, injection-safe) the tool-written block of a driver run's DIRECTION.md, proven by 7 Rust-emitted goldens and 18 byte-identical spike files.

## What was built

- `src/state_reader/driver_frontmatter.rs`: `RunStatus` (8 words + tolerant `Unknown`, `stalled` terminal), closed `ItemState`, order-preserving `ItemStates`, `Cap`/`WallClockCap`/`GapCap`/`Caps`, `DispatchAudit`, `LastEvent`, `Connection`, `DriverFrontmatter`, `FrontmatterError`, `parse_driver_frontmatter` (via `state_md::extract_frontmatter`, marker check, two-field version probe, then full parse) and `DriverFrontmatter::emit`. Module doc carries the D-19 evolution rule (additive vs. major bump) and the parse-only note for serde_yml (RUSTSEC-2025-0068). Every type derives Serialize and Deserialize for the 27-05 store.
- `tests/driverctl_frontmatter.rs`: 7 `golden_*` round-trips, `stalled_is_a_terminal_status`, `item_states_keep_direction_order`, `every_spike_direction_parses` (18 files, tally stopped 4 / parked 11 / done 1 / crashed 1, template running with two connections), `version_marker_and_block_errors`, plus an `#[ignore]`d `write_goldens` regenerator.
- 7 golden files and 18 spike copies (all `cmp`-identical to their sources).

## Task commits

1. Task 1 (tracer) `3ca76e0`: schema module, `pub mod`, run1b fixture; 6 unit tests incl. `spike_run1b_parses_and_reemits_equal`.
2. Task 2 `c1afba6`: goldens + golden tests.
3. Task 3 `e527419`: 17 remaining spike copies, tally and error tests, rustfmt.

## Verification

- `cargo test --lib state_reader::driver_frontmatter`: 6 passed.
- `cargo test --test driverctl_frontmatter`: 11 passed, 1 ignored (`write_goldens`).
- `cargo clippy --all-targets -- -D warnings`: clean.
- Full `cargo test --no-fail-fast`: 2112 lib passed, 1 failed = the expected git-version constants test in `src/envelope/policy.rs`; every integration binary green.

## Deviations from Plan

None to code behavior. Minor, non-rule notes:

- The plan's source path for the L1 copy (`.../cap-tests/L1-DIRECTION-final.md`) resolves to `.planning/spikes/002-launcher-caps-and-tool-frontmatter/experiment/results/cap-tests/L1-DIRECTION-final.md`; copied from there.
- Task 2 is tdd-flagged but goldens are by prescription produced from `emit()` output and then inspected, so there is no separate RED commit (the golden tests cannot exist before the files they read).
- Added `write_goldens` as an `#[ignore]`d test so the goldens are reproducible (not requested by the plan).

## Inferred decisions (audit)

- `[inferred — audit]` `is_terminal()` includes `parked` (27-CONTEXT line 74 lists `stopped | done | crashed | stalled | parked` as terminal); `Unknown` is non-terminal.
- `[inferred — audit]` `quote()` also escapes U+2028 and U+2029 (not `is_control`, but libyaml treats them as line breaks); a strict superset of the plan's rule.
- `[inferred — audit]` `emit` drops control characters from `generated_by` (written plain, so it cannot be quoted) to keep it on one line.
- `[inferred — audit]` `updated_at` is a `String` (default empty), `heartbeat_stale_after_s` defaults to 300 when absent; the plan left both types open.
- `[inferred — audit]` The seven golden state definitions are the interpretation recorded in the banner of `tests/driverctl_frontmatter.rs`.

## Known Stubs

None.

## Threat Flags

None. T-27-09 (emitter injection) mitigated and tested by `quote_escapes_control_and_quotes`; T-27-10 mitigated by typed `FrontmatterError`, version gate and unknown-key tolerance.

## Self-Check: PASSED

- Files exist: `src/state_reader/driver_frontmatter.rs`, `tests/driverctl_frontmatter.rs`, 18 files in `tests/fixtures/driverctl/direction/`, 7 in `tests/fixtures/driverctl/golden/`.
- Commits `3ca76e0`, `c1afba6`, `e527419` are ancestors of HEAD.
