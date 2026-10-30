---
phase: 27-driverctl-driver-frontmatter
plan: 02
subsystem: driverctl
tags: [rust, gap-rounds, log-grammar, exit-classification, replay-tests]

requires:
  - phase: 26.1-driver-validation-spike-003-re-run
    provides: gapcount.py / exitclass.py counting rules and the spike-003 result files used as fixtures
provides:
  - "crate::driverctl module (counters, log, scan)"
  - "observe_gap / GapRoundsState: the 26.1 coalesced gap-round counter and its exact state.json shape"
  - "scan_verifications: stat-read-stat VERIFICATION.md scan"
  - "log: the LOG.md grammar owner (format_line, append, parse_log, one_line, epoch_seconds)"
  - "outstanding_dispatch / classify_exit: stalled-versus-crashed decision for Phase 28"
affects: [27-driverctl-store, 28-launcher]

actuals:
  tokens: 24000
  tasks: 3
  commits: 4
plan_head_before: 1866e0761570f50563adf5822ffa566a4275b868
plan_head_after: 98cc7b17e94af9b459cf964aef1fa2d200da7f32

tech-stack:
  added: []
  patterns:
    - "Counting rules are pure functions over recorded evidence, replay-tested against committed byte copies of spike results"
    - "include_str! of tests/fixtures so replay tests are cwd-independent"

key-files:
  created:
    - src/driverctl/mod.rs
    - src/driverctl/counters.rs
    - src/driverctl/log.rs
    - src/driverctl/scan.rs
    - tests/fixtures/driverctl/spike003/ (11 byte-identical files from run1, run1b, r261a, r261-run2, r261-l8)
  modified:
    - src/lib.rs

key-decisions:
  - "COALESCE_S is f64 45.0 with a strict < comparison; 45.0 s is a new round, 44.999999 s coalesces"
  - "fm_status is the single status definition and is not the dashboard's VerificationStatus::from_raw (D-27)"
  - "worst_phase ties go to the latest last_write, then the first sorted phase key"

requirements-completed: [DCTL-04]

coverage:
  - id: D1
    description: "Coalesced gap-round counter (observe_gap) with the exact caps.gap_rounds state shape, S2-shape loading, strict 45 s boundary, sliding anchor"
    requirement: DCTL-04
    verification:
      - kind: unit
        ref: "src/driverctl/counters.rs#window_boundary_is_strict_and_unrounded"
        status: pass
      - kind: unit
        ref: "src/driverctl/counters.rs#sliding_anchor_makes_five_writes_20s_apart_one_round"
        status: pass
      - kind: unit
        ref: "src/driverctl/counters.rs#s2_shaped_state_without_window_fields_loads_and_counts"
        status: pass
    human_judgment: false
  - id: D2
    description: "Phase 26 Run 1 replay (OLD [1,2,3] -> NEW [1,2], coalesced 1) and the r261a state.json shape"
    requirement: DCTL-04
    verification:
      - kind: unit
        ref: "src/driverctl/counters.rs#run1_replay_coalesces_the_sed_rewrite"
        status: pass
      - kind: unit
        ref: "src/driverctl/counters.rs#r261a_state_shape_loads"
        status: pass
    human_judgment: false
  - id: D3
    description: "scan_verifications: gaps_found-only, stat-read-stat, numeric phase key, LOG-header-safe paths, tracer scan->observe flow"
    requirement: DCTL-04
    verification:
      - kind: unit
        ref: "src/driverctl/scan.rs#scan_then_observe_counts_coalesces_and_dedups"
        status: pass
    human_judgment: false
  - id: D4
    description: "LOG.md grammar owner: format_line/parse_log round trip, detail lines, one_line validation"
    requirement: DCTL-04
    verification:
      - kind: unit
        ref: "src/driverctl/log.rs#parse_log_round_trips_format_line_and_captures_detail"
        status: pass
    human_judgment: false
  - id: D5
    description: "outstanding_dispatch (WR-01 cases a-f) and classify_exit with the five exitclass replays (run1b stalled, run1 parked, r261a parked, r261-run2 done, r261-l8 stopped)"
    requirement: DCTL-04
    verification:
      - kind: unit
        ref: "src/driverctl/counters.rs#run1b_replay_classifies_stalled_with_i01_outstanding"
        status: pass
      - kind: unit
        ref: "src/driverctl/counters.rs#wr01_a_denial_cancels_only_its_own_immediately_preceding_dispatch"
        status: pass
    human_judgment: false

duration: 25min
completed: 2026-10-19
status: complete
---

# Phase 27 Plan 02: driverctl counters, LOG grammar and scan Summary

**Rust port of the spike-003 coalesced gap-round counter, the VERIFICATION.md scan, the LOG.md grammar and the stalled-versus-crashed exit classifier, pinned offline by committed copies of the spike's own results.**

## Performance

- **Duration:** about 25 min
- **Started:** 2026-10-18T05:00Z (approx.)
- **Completed:** 2026-10-19T05:06Z
- **Tasks:** 3
- **Files modified:** 5 source files, 11 fixture files

## Accomplishments

- `observe_gap` reproduces the 26.1 rule exactly: the Run 1 `sed -i` re-write 6.5 s after the verifier write now coalesces (recorded counts [1, 2, 3] become [1, 2], coalesced 1), the 45.0 s boundary is strict, and `GapRoundsState` serialises to the exact `caps.gap_rounds` key set while still loading the spike-002 shape.
- `scan_verifications` derives counts only from `gaps_found` bytes on disk (digest, mtime, WR-07 stat-read-stat); a non-numeric phase dir or a path containing `|`, CR, LF or over 200 chars is skipped without aborting the tick.
- `log` is the single owner of the LOG header grammar, including a detail-capturing `parse_log` and the `one_line` header-value check later plans must route values through.
- `outstanding_dispatch` and `classify_exit` pin D-17: the Run 1b shape (rc 0, result success, I-01 dispatched with no report) classifies `stalled`, never `crashed`; the other four replays keep their recorded statuses.

## Task Commits

1. **Task 1: Tracer, scan then observe** - `dc84ecc` (feat)
2. **Task 2: LOG grammar, selftest table, Run 1 replay, r261a shape** - `9eecbd4` (feat)
3. **Task 3 RED: outstanding_dispatch / classify_exit tests and fixtures** - `493819e` (test)
4. **Task 3 GREEN: implementation** - `98cc7b1` (feat)

The tracer verify (`cargo test --lib driverctl::`) was green before Task 2 started, so expansion proceeded.

## Verification

- `rtk proxy cargo test --no-fail-fast --lib driverctl::` : 43 passed, 0 failed.
- `rtk proxy cargo test --no-fail-fast --test async_blocking_guard` : 9 passed.
- `rtk proxy cargo clippy --all-targets -- -D warnings` : clean.
- Full `cargo test --no-fail-fast`: lib 2149 passed, 1 failed (the expected local git-version constants test in `src/envelope/policy.rs`); every integration target green.
- All 11 fixture files verified byte-identical to their spike sources with `cmp`.

## Deviations from Plan

### Auto-fixed Issues

None - plan executed as written, with the following process notes.

- **TDD shape of Tasks 2 and the tracer:** `observe_gap` and `log.rs` were authored together with their tests (characterization tests over already-ported behaviour), so there is no separate failing RED commit for them. Task 3 followed a real RED (stubs with `unimplemented!()`, 8 tests failing) then GREEN.
- **Trailer attribution:** the dispatch prompt asked for `Co-Authored-By: Claude Opus 5.5`; commits carry `Claude Sonnet 5.5` per the session's attribution instruction, which names the model that actually ran. The `Claude-Session` line is as requested.

## Inferred decisions (audit)

- `[inferred - audit]` `worst_phase` ties on count go to the larger `last_write` (Python relied on dict insertion order); a remaining tie goes to the first phase key in sorted order.
- `[inferred - audit]` `log::one_line` also rejects a bare CR (the spike rejected only LF) so a value can never forge a second event for a CR-splitting reader.
- `[inferred - audit]` `scan_verifications` skips dot-prefixed directory and file names (mirrors Python `glob` `*`) and sorts the project-relative path strings, matching the spike's sort of the full glob result.
- `[inferred - audit]` `parse_log` attaches four-space-indented lines only while they directly follow an event (a blank or prose line ends the detail run); the spike parser ignored detail lines entirely.
- `[inferred - audit]` `classify_exit` takes `tripped` as a `bool`; the replay helper treats a JSON `tripped` of null, false or empty string as not tripped.
- The 24000-token `actuals.tokens` is chars/4 over the new source plus fixtures (about 96 KB), counted once.

## Known Stubs

None.

## Threat Flags

None. The scan reads project files and no new network, auth or schema surface was added. T-27-05 through T-27-08 are mitigated by the tests named in the plan's threat register.

## Issues Encountered

None.

## Next Phase Readiness

Plan 27-02 is complete; the `driverctl` module root declares `counters`, `log` and `scan`, ready for the later `protocol`, `store` and `serve` plans. Phase 28 consumes `GapRoundsState`, `worst_phase`, `outstanding_dispatch` and `classify_exit`.

## Self-Check: PASSED

- Files present: `src/driverctl/{mod,counters,log,scan}.rs`, `tests/fixtures/driverctl/spike003/` (11 files).
- Commits present: `dc84ecc`, `9eecbd4`, `493819e`, `98cc7b1`.
