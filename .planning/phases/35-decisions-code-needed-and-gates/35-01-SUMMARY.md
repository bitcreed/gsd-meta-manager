---
phase: 35-decisions-code-needed-and-gates
plan: 01
subsystem: owner-inbox
tags: [owner-items, debug-sessions, inbox, status, audit-open-oracle]
requires:
  - phase: 34
    provides: owner_items Layer A/B split, parse cache, inbox and status schemas
provides:
  - "owner_items::debug (Layer A port of scanDebugSessions) and OpenScan/scan_open"
  - "OwnerDigest.decide and fix_debug; summary.fix_debug and owner.fix_debug"
  - "Kind::{Debug,Handoff,Checkpoint,ContextQuestions}, generic ParseCache<K,V>, OPEN_CACHE"
  - "tests/owner_open_conformance.rs (audit-open oracle harness)"
affects: [35-02, 35-03, 35-04]
tech-stack:
  added: []
  patterns: ["Layer A port pinned by an oracle test, Layer B rules fixture-tested", "signal_item for non-UAT items"]
key-files:
  created:
    - src/state_reader/owner_items/debug.rs
    - tests/owner_open_conformance.rs
    - tests/fixtures/owner/debug-awaiting/
    - tests/fixtures/owner/debug-sessions/
  modified:
    - src/state_reader/owner_items/mod.rs
    - src/state_reader/owner_items/rules.rs
    - src/state_reader/owner_items/scan.rs
    - src/state_reader/owner_items/frontmatter.rs
    - src/inbox.rs
    - src/status_cli.rs
    - src/agent_skill.md
    - README.md
    - tests/inbox_cli.rs
    - tests/status_cli.rs
    - tests/owner_digest.rs
key-decisions:
  - "Debug groups are root-scoped: phase \"\", phase_name = slug, command /gsd-debug, never counted in fix_phases/closeable_phases"
  - "Layer B recovers a column-0 status: line from a closed but YAML-rejected fence ([INFERRED] F2); Layer A keeps unknown for oracle parity"
  - "Unreadable debug file is a Fix group with counts.unreadable 1 ([INFERRED] A9)"
requirements-completed: [OWN-10]
duration: one session
completed: 2026-11-14
status: complete
actuals:
  tokens: 15800
  tasks: 2
  commits: 2
plan_head_before: a30adb18823749520fdf06ac053f50d8510aabd2
plan_head_after: 948c4f55cba06d48a5c5d0c41669485208a564b3
---

# Phase 35 Plan 01: Debug sessions as Decide and Fix signals Summary

Debug sessions under `.planning/debug/` now travel through a process-free Layer A port of gsd-core 1.21.0's `scanDebugSessions` into the owner rules, `inbox --json --lane decide` (awaiting confirmation) or the Fix lane (open investigation), and the `decide` / `fix_debug` counts of both CLIs, oracle-pinned against `audit-open`.

## Tasks

| Task | Commit | Result |
|------|--------|--------|
| 1 tracer: awaiting session to `inbox --lane decide` and `status` | 53c42593 | `debug.rs`, new `Kind` variants, `OpenScan`/`scan_open`, `decide`/`fix_debug`, DECIDE/FIX debug rows, README and agent-skill contracts, `debug-awaiting` fixture |
| 2 oracle parity and every debug row | 948c4f55 | `tests/owner_open_conformance.rs`, `debug-sessions` fixture, unit tests (scan rules, recovery, `moment`, cache, Layer B lanes, age boundary), `owner_digest` test |

## Verification (raw)

- `cargo test --no-fail-fast` (whole crate): 4099 passed, 0 failed, 19 ignored.
- `cargo clippy --all-targets -- -D warnings`: clean.
- `owner_open_conformance`: 3 passed (2 fixtures compared with the real oracle, equal); `owner_audit_conformance`: 3 passed (new fixtures accepted); `spawn_seam_guard`: 10 passed.
- Read-only check on sentriq: `inbox --json --lane decide` lists 5 debug groups (baseline-purge-reruns-on-reprof, crank-missed-in-standby, faults-disconnected-live-dash, polling-rpm-skips-hung-stop, towing-drive-splits-18min); `crank-missed-in-standby` has Layer A status `unknown` and is recovered by the F2 rule. `summary.decide` 5, `fix_debug` 0.
- SC4: no existing key-path string removed from `INBOX_PATHS` / `SCHEMA_PATHS` / README (diff gate prints 0); `SCHEMA_PATHS.len()` 50 to 51; goldens untouched.

## Deviations from Plan

**1. [Rule 3 - Blocking] Generic cache pulled into Task 1.** `debug.rs` reads through `OPEN_CACHE`, so `ParseCache<K, V>` and `OpenKind` landed in the tracer commit instead of Task 2. `OpenKind::Context` is `#[allow(dead_code)]` until plan 35-02 constructs it.

**2. [Rule 1 - Plan miscount] Oracle lists 11, not 10, open sessions in `debug-sessions`.** The plan's fixture list includes a session whose unparseable frontmatter says `resolved`; upstream reads it as `unknown` (open), so with the kept `resolved ` file the oracle count is 11 (acknowledged 1, as planned). Layer B still lists 5 Decide and 4 Fix and drops both resolved ones, as the plan's truths require.

**3. [Rule 3 - Blocking] `spawn_seam_guard`.** The first FIFO test used `Command::new("mkfifo")`, a spawn site; replaced with the shared `fifo_test_support::mkfifo`. The `moment` tests were moved into the existing `mod tests` of `frontmatter.rs` (a second test module trips the post-marker item guard).

**4.** `scan.rs` `subdirectories` was left private (no caller needed it); `cargo fmt` also reflowed three unrelated test lines in `scan.rs`.

## Inferred decisions

F2 status recovery, A9 unreadable-as-Fix, `/gsd-debug` (not `continue <slug>`), category `unknown` with meaning in `result` (A6), header suffix ` + N debug` only when `fix_debug` > 0, and a normalized `resolved`/`complete` (including `resolved ` and a recovered value) is not listed and not counted as acknowledged.

## Known Stubs

None.

## Threat Flags

None beyond the plan's register: symlinks skipped by lstat, containment plus NOFOLLOW open, FIFO and 1 MiB cap tested, no process API in `src/` (`no_source_file_names_the_open_audit_verb` passes). Plan 35-04 owns the hostile-string test for debug fields; all debug strings already pass `terminal_capped`.

## Self-Check: PASSED

Created files exist (debug.rs, owner_open_conformance.rs, both fixtures); commits 53c42593 and 948c4f55 are ancestors of HEAD.
