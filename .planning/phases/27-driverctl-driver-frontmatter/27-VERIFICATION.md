---
phase: 27-driverctl-driver-frontmatter
verified: 2026-10-19T06:10:00Z
status: passed
score: 4/4 must-haves verified
covered_files:
  - ".planning/phases/27-driverctl-driver-frontmatter/27-01-PLAN.md"
  - ".planning/phases/27-driverctl-driver-frontmatter/27-01-SUMMARY.md"
  - ".planning/phases/27-driverctl-driver-frontmatter/27-02-PLAN.md"
  - ".planning/phases/27-driverctl-driver-frontmatter/27-02-SUMMARY.md"
  - ".planning/phases/27-driverctl-driver-frontmatter/27-03-PLAN.md"
  - ".planning/phases/27-driverctl-driver-frontmatter/27-03-SUMMARY.md"
  - ".planning/phases/27-driverctl-driver-frontmatter/27-04-PLAN.md"
  - ".planning/phases/27-driverctl-driver-frontmatter/27-04-SUMMARY.md"
  - ".planning/phases/27-driverctl-driver-frontmatter/27-05-PLAN.md"
  - ".planning/phases/27-driverctl-driver-frontmatter/27-05-SUMMARY.md"
  - ".planning/phases/27-driverctl-driver-frontmatter/27-06-PLAN.md"
  - ".planning/phases/27-driverctl-driver-frontmatter/27-06-SUMMARY.md"
  - "src/cli.rs"
  - "src/driver/direction.rs"
  - "src/driver/lock.rs"
  - "src/driverctl/counters.rs"
  - "src/driverctl/log.rs"
  - "src/driverctl/mod.rs"
  - "src/driverctl/protocol.rs"
  - "src/driverctl/scan.rs"
  - "src/driverctl/serve.rs"
  - "src/driverctl/store.rs"
  - "src/driverctl_cli.rs"
  - "src/main.rs"
  - "src/state_reader/driver_frontmatter.rs"
covered_digest: "v3:sha256:729bdcb6daf6072883f5824dbb8fd9a3bd03a318b7cad45fffe7b714dc97a3cf"
behavior_unverified: 0
overrides_applied: 0
---

# Phase 27: driverctl & Driver Frontmatter Verification Report

**Phase Goal:** The single writer of a run's state is a Rust subcommand of gsd-mm. Its frontmatter on DIRECTION.md is format-correct by construction and never costs a human edit.
**Verified:** 2026-10-19
**Status:** passed
**Re-verification:** No, initial verification

## Goal Achievement

### Observable Truths (ROADMAP success criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | `driverctl serve\|log\|hook\|show` replaces `tools/driverctl.py` with the same socket protocol; the spike-002 MASTER-PROMPT runs unchanged | VERIFIED | Commands::Driverctl dispatched from a sync `main` before tracing (`src/main.rs:106`). I started `driverctl serve` and drove it with `log start` (rc 0, "ok"), `hook` and a rejected `log` through the real binary. `tests/driverctl_client.rs` parses every MASTER-PROMPT line, and `tests/driverctl_replay.rs` replays three recorded spike LOGs through the Rust client and server. All pass. |
| 2 | Editor-race port: zero lost rename-saves, in-place loss at or below the spike rate; gsd-mm's own DIRECTION writes take the shared flock | VERIFIED | I ran `editor_race_full` (15 s per style): rename LOST=0 of 13722 saves, in-place LOST=0 of 13926 saves (allowed 4). The naive positive control shows LOST>0. `write_body` and `splice_frontmatter` share `direction_lock` (`src/driver/direction.rs`). Lock race tests 1-5 pass. |
| 3 | Every generated frontmatter parses into a typed `DriverFrontmatter` in `state_reader`; a body without `## Direction`/`## Items` is never written back | VERIFIED | `src/state_reader/driver_frontmatter.rs` holds the schema, hand emitter and `parse_driver_frontmatter`. `tests/driverctl_frontmatter.rs` round-trips 7 goldens (template, starting, running, stopped, crashed, stalled, multi-connection) and parses 18 spike files. `SkippedInsaneBody` and `NoDirection` leave bytes unchanged (`direction.rs:506`). In my live run the body after the frontmatter was md5-identical to the input. |
| 4 | Malformed events and unknown items give rc=2; dispatches counted from hook payloads (no `agent_id` means driver-level); gap rounds from `gaps_found` VERIFICATION.md on disk, coalesced per 26.1 (45 s strict, mtime-anchored), `caps.gap_rounds.per_phase` and a cap hook point exposed; heartbeat rewrites coalesced to 120 s | VERIFIED | Live: an unknown event and an unknown item (`I-99`) were rejected with `driverctl: REJECTED` and rc=2. An agent-less hook gave `dispatches.used: 1` and `dispatch_audit.observed: 1`, and a nested `agent_id` hook was not counted. A `gaps_found` file dropped on disk produced `gap_rounds {used: 1, phase: "3"}` and a `RUN \| gap-observed ... source=driverctl-scan` LOG line within the 2 s tick. Code: `COALESCE_S = 45.0` with strict `<` (`counters.rs:22,93`), `HEARTBEAT_REGEN_MIN_S = 120` (`store.rs:72`), `set_gate` (`store.rs:440`). The protocol has no op that sets `crashed`/`stalled` or counters (`protocol.rs:28`, test at line ~1002). |

**Score:** 4/4 truths verified (0 behavior-unverified). Behavior-dependent invariants have passing named tests: exchange-back on a mismatch, lock-before-socket, no socket theft (second serve rc=3, confirmed live), and SIGKILL then `Crashed` plus `crash-detected` (`tests/driverctl_serve.rs::sigkill_reads_crashed_and_restart_logs_crash_detected`).

### Required Artifacts

All artifacts from the six plans exist and are substantive and wired: `src/driver/direction.rs` (896 lines), `src/driver/lock.rs` (`acquire_at`, `acquire_at_with_retry`, `probe_at`, `run_lock_path`), `src/driverctl/{mod,counters,log,scan,protocol,store,serve}.rs`, `src/driverctl_cli.rs`, `src/state_reader/driver_frontmatter.rs`, and 8 `tests/driverctl_*.rs` files with fixtures under `tests/fixtures/driverctl/`.

### Key Link Verification

| From | To | Status |
|------|----|--------|
| `direction_lock` | `acquire_at_with_retry` | WIRED |
| `store.rs` regen | `splice_frontmatter(emit())` (`store.rs:666`) | WIRED |
| `store.rs` | `observe_gap` and `worst_phase` | WIRED |
| `store.rs` | `validate_log` and `validate_hook` before mutation | WIRED |
| `main.rs` | `driverctl_cli::run` before tracing | WIRED |
| `read_driver_run` | `probe_at(run_lock_path)` | WIRED |
| `serve.rs` | `acquire_at_with_retry` before bind, then `RunStore` and `set_terminal` | WIRED |

### Data-Flow Trace (Level 4)

The frontmatter values observed live came from real inputs: hook payloads (dispatch counts), the VERIFICATION.md on disk (gap rounds), DIRECTION.md items (`item_states`), and the lock file (`launches`). No static or hollow values found.

### Behavioral Spot-Checks

| Behavior | Result | Status |
|----------|--------|--------|
| `rtk proxy cargo test --no-fail-fast` | 3191 passed, 1 failed (the expected `envelope::policy` git-version constants test); all `driverctl_*` targets green | PASS |
| `editor_race_full --ignored` | LOST=0 rename, LOST=0 in-place | PASS |
| Live `serve` + `log`/`hook` via the binary | rc 0 / rc 2 / rc 3 as specified | PASS |
| Hook gate with an unreachable socket | Deny JSON with `LAUNCHER CAP:` prefix, rc 0 (fail closed; observed incidentally with an over-long socket path) | PASS |

### Probe Execution

No probe scripts are declared by this phase (SKIPPED).

### Requirements Coverage

| Requirement | Source Plans | Description | Status | Evidence |
|-------------|--------------|-------------|--------|----------|
| DCTL-01 | 27-04, 27-05, 27-06 | driverctl serve/log/hook/show is the single writer of state.json, LOG.md and the frontmatter | SATISFIED | See truth 1 and the live run |
| DCTL-02 | 27-01 | Exchange-verify splice, body byte-preserved, insane body never written, shared flock | SATISFIED | See truths 2 and 3 |
| DCTL-03 | 27-01, 27-03, 27-04, 27-06 | Typed `DriverFrontmatter`; liveness from a non-blocking run-lock probe | SATISFIED | Golden and spike parse tests; `tests/driverctl_liveness.rs` R1-R6; SIGKILL test |
| DCTL-04 | 27-02, 27-04, 27-05, 27-06 | Tool-observed counters; rc=2 rejection; 120 s heartbeat coalescing | SATISFIED | See truth 4 |

All four IDs are declared in plan frontmatter and mapped to Phase 27 in REQUIREMENTS.md. There are no orphaned requirements. The REQUIREMENTS.md checkboxes and traceability table still read `Pending`/`[ ]` and should be flipped by the orchestrator when the phase is closed (a bookkeeping item, not a gap).

### Anti-Patterns Found

None. A scan of all phase-modified source and test files for TBD, FIXME, XXX, TODO, HACK, `todo!` and `unimplemented!` found nothing.

### Human Verification Required

None. Everything checkable was checked by execution.

### Gaps Summary

No gaps. The single failing test in the workspace is the known machine-local git-version witness (`envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`), which is expected to pass on the CI runner.

---

_Verified: 2026-10-19_
_Verifier: Claude (gsd-verifier)_
