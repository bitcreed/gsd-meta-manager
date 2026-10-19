---
phase: 27-driverctl-driver-frontmatter
plan: 05
subsystem: driverctl
tags: [rust, run-store, wire-protocol, hooks, gate, heartbeat, gap-rounds]

requires:
  - phase: 27-driverctl-driver-frontmatter
    provides: "27-01 splice_frontmatter, RunLock, parse_direction; 27-02 GapRoundsState, observe_gap, scan_verifications, log grammar; 27-03 DriverFrontmatter and its emitter"
provides:
  - "crate::driverctl::protocol: Request/Response wire types, parse_request, validate_log, validate_hook, the spike enums"
  - "crate::driverctl::store::RunStore: the single writer of state.json, LOG.md and the DIRECTION frontmatter, gated by the RunLock by type"
  - "RunStore::set_gate (Phase 28 cap gate hook point), observe_verification / scan_tick, flush_heartbeat, set_terminal, begin_launch"
affects: [27-06-serve, 28-launcher, 30-driver-tab]

actuals:
  tokens: 31800
  tasks: 3
  commits: 4
plan_head_before: 8c4961f8d2e2dd2cdcd0bdef89fe3a8e8227be3e
plan_head_after: e677ce0cc9ef63f04e3ef74b160fce20fc159c72

tech-stack:
  added: []
  patterns:
    - "Validate fully, apply on a clone, append LOG, write state.json, commit the clone, then regenerate (a rejection writes nothing)"
    - "Explicit allowed-key list instead of deny_unknown_fields (repo guard D-30)"
    - "Injected Clock (Box<dyn Fn() -> DateTime<Utc> + Send>) so heartbeat windows are tested without sleeping"

key-files:
  created:
    - src/driverctl/protocol.rs
    - src/driverctl/store.rs
    - tests/driverctl_store.rs
  modified:
    - src/driverctl/mod.rs

key-decisions:
  - "RunStore::open takes the RunLock by value; the lock is released only when the store drops"
  - "caps.dispatches.used counts only hook-observed driver-level dispatches the Gate allowed, never the driver's own dispatched lines"
  - "crashed and stalled are set only by RunStore::set_terminal; no op, event or argument reaches it"
  - "Unknown log/hook keys are refused by an explicit key list because tests/spawn_seam_guard.rs forbids deny_unknown_fields under src/"

requirements-completed: [DCTL-01, DCTL-04]

coverage:
  - id: D1
    description: "Request grammar and rc=2 validation: every malformed request is rejected and LOG.md, state.json and DIRECTION.md are byte-identical afterwards"
    requirement: DCTL-01
    verification:
      - kind: integration
        ref: "tests/driverctl_store.rs#every_malformed_request_is_rejected_and_nothing_is_written"
        status: pass
      - kind: unit
        ref: "src/driverctl/protocol.rs#known_item_rejects_run_and_unknown_ids_with_the_spike_message"
        status: pass
    human_judgment: false
  - id: D2
    description: "Every spike log event is accepted with the spike LOG key order; report truncation, FINDINGS, git base, end-reason mapping"
    requirement: DCTL-01
    verification:
      - kind: integration
        ref: "tests/driverctl_store.rs#dispatched_logs_the_spike_key_order_and_updates_state"
        status: pass
      - kind: integration
        ref: "tests/driverctl_store.rs#report_keeps_fifteen_lines_and_records_findings"
        status: pass
      - kind: integration
        ref: "tests/driverctl_store.rs#end_maps_every_reason_to_its_status"
        status: pass
    human_judgment: false
  - id: D3
    description: "Hook dispatch counting only for driver-level Agent/Task payloads the Gate allows; denial logged and replied; driver-logged dispatches never move the cap counter"
    requirement: DCTL-04
    verification:
      - kind: integration
        ref: "tests/driverctl_store.rs#only_driver_level_agent_dispatches_the_gate_allows_are_counted"
        status: pass
      - kind: integration
        ref: "tests/driverctl_store.rs#a_denying_gate_counts_the_denial_logs_it_and_replies_deny"
        status: pass
    human_judgment: false
  - id: D4
    description: "Heartbeat coalescing: >= 120 s regenerates, 119 s does not, other events regenerate immediately, trailing flush regenerates once"
    requirement: DCTL-04
    verification:
      - kind: integration
        ref: "tests/driverctl_store.rs#a_heartbeat_at_119_s_is_coalesced_and_one_at_exactly_120_s_is_not"
        status: pass
      - kind: integration
        ref: "tests/driverctl_store.rs#a_suppressed_heartbeat_is_flushed_once_the_window_has_passed"
        status: pass
    human_judgment: false
  - id: D5
    description: "Gap rounds observed from disk through scan_tick (new, coalesced, duplicate writes nothing)"
    requirement: DCTL-04
    verification:
      - kind: integration
        ref: "tests/driverctl_store.rs#scan_tick_observes_gap_rounds_from_disk_and_coalesces_rewrites"
        status: pass
    human_judgment: false
  - id: D6
    description: "crash-detected launch bookkeeping, launcher-only set_terminal, forward-compatible state.json, no wire path to crashed/stalled"
    requirement: DCTL-01
    verification:
      - kind: integration
        ref: "tests/driverctl_store.rs#a_relaunch_after_an_unended_launch_logs_crash_detected"
        status: pass
      - kind: integration
        ref: "tests/driverctl_store.rs#no_request_can_set_crashed_or_stalled_or_rewrite_a_counter"
        status: pass
      - kind: integration
        ref: "tests/driverctl_store.rs#launcher_written_state_keys_survive_a_store_rewrite"
        status: pass
    human_judgment: false

duration: 70min
completed: 2026-10-19
status: complete
---

# Phase 27 Plan 05: RunStore, the Rust single writer Summary

**Spike-002's RunStore ported as a library type holding the run lock by value: validate-then-apply request handling with the spike's LOG grammar, hook-counted dispatches behind a Phase 28 Gate, 120 s heartbeat coalescing with a trailing flush, disk-observed gap rounds, and a launcher-only terminal setter.**

## Performance

- **Tasks:** 3 (tracer, TDD events/validation, TDD hooks/gate/heartbeat/gaps/terminal)
- **Files:** 3 created (`protocol.rs`, `store.rs`, `tests/driverctl_store.rs`), 1 modified (`mod.rs`)

## Accomplishments

- `protocol.rs` carries the wire types and the pure `validate_log` / `validate_hook`: all nine spike events, the A7 tightenings, and 17 in-module unit tests. A rejection never reaches the store's write path.
- `RunStore::open(run_dir, project, lock: RunLock)` makes a lock-less store unrepresentable. `state.json` loads with `#[serde(default)]` plus a flattened `extra` on `RunState`, `ItemRecord` and `StateCaps`, so the r261a launcher keys (`auth`, `cost`, `active_seconds`) survive a rewrite.
- The frontmatter is a derived view: `frontmatter()` builds a `DriverFrontmatter` (item counts and `item_states` in DIRECTION order, `gap_rounds` from `worst_phase`, dispatch audit) and `regen()` splices `emit()` through `splice_frontmatter`.
- Cap integrity: a test logs three `dispatched` events and shows `caps.dispatches.used` stays 0 until the hook counts a driver-level `Agent`/`Task` dispatch the Gate allowed; a nested payload (with `agent_id`) never counts.
- `no_request_can_set_crashed_or_stalled_or_rewrite_a_counter` drives 13 attempts (a made-up `set_terminal` op, `end --reason stalled`, `status --state crashed`, a `hook_dispatches` arg, ...) and asserts the status and every counter are unchanged.

## Task Commits

1. **Task 1 (tracer): RunStore turns `log start` into a LOG line, state.json and a spliced frontmatter** - `106e611` (feat)
2. **Task 2 RED: tests for every event, the rc=2 table, crash-detected, forward compatibility** - `c1fb207` (test)
3. **Task 2 GREEN: full `validate_log`** - `bd54fae` (feat)
4. **Task 3: hooks, Gate, heartbeat coalescing, gap observation, set_terminal** - `e677ce0` (feat)

The tracer verify was green before Task 2 started, so expansion proceeded.

## Verification

- `rtk proxy cargo test --no-fail-fast --test driverctl_store`: 29 passed (includes the 2 s `a_held_direction_lock_is_logged_and_the_splice_proceeds`).
- `rtk proxy cargo test --no-fail-fast --lib driverctl::protocol`: 17 passed.
- `rtk proxy cargo test --no-fail-fast --test async_blocking_guard`: 9 passed. `--test spawn_seam_guard`: 41 passed.
- `rtk proxy cargo clippy --all-targets -- -D warnings`: clean. `cargo fmt --check`: clean.
- Full `rtk proxy cargo test --no-fail-fast`: 3123 passed, 1 failed (the expected local git-version constants test in `src/envelope/policy.rs`); every integration target green.
- Mutation check: flipping the heartbeat `>=` to `>` fails both 120 s tests, so the boundary assertions are not vacuous.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] `deny_unknown_fields` is forbidden under `src/` by a repository guard**
- **Found during:** Task 3 full-suite run
- **Issue:** the plan's artifact table and Task 2 acceptance call for `#[serde(deny_unknown_fields)]` on `LogArgs` and `HookPayload`, but `tests/spawn_seam_guard.rs::no_executable_line_in_src_opts_into_strict_unknown_field_rejection` (D-30) fails on any executable occurrence of that attribute.
- **Fix:** both structs drop the attribute; `strict_object_or_null` checks the object's keys against `LOG_ARG_KEYS` / `HOOK_PAYLOAD_KEYS` before deserializing, so an unknown key is refused exactly as planned (`malformed request: unknown field ...`). The attribute's name still appears in a doc comment (not executable, so the guard and the plan's `grep -c` both hold).
- **Files modified:** `src/driverctl/protocol.rs`
- **Commit:** `e677ce0`

**2. [Rule 1 - tooling] clippy `large_enum_variant` on `Request`**
- `Request::Log` holds `LogArgs` by value (the plan's shape); an `#[allow]` with a reason is on the enum rather than boxing a public variant field. Commit `e677ce0`.

### Process notes

- **TDD shape:** Task 1 is the tracer and carries the store, so the Task 2 and Task 3 store behaviour was written ahead of, or together with, its tests. Task 2 has a real RED commit (9 protocol unit tests and 10 store tests failing against the Task 1 `validate_log`). Task 3 has no separate RED (the tests do not compile without the new methods); the heartbeat-boundary mutation check stands in for non-vacuity evidence.
- **Trailer attribution:** the dispatch prompt asked for `Co-Authored-By: Claude Opus 5.5`; commits carry `Claude Sonnet 5.5` per the session's attribution instruction, which names the model that actually ran (same call 27-02 made). The `Claude-Session` line is as requested.

## Inferred decisions (audit)

- `[inferred - audit]` `validate_hook` checks only the values that reach a LOG header (`session_id`/`source` on `SessionStart`, `session_id`/`reason` on `SessionEnd`). A gate or heartbeat payload writes nothing derived from the payload, and refusing a gate payload would let the dispatch through uncounted (the hook client treats a reply without `deny` as allow).
- `[inferred - audit]` Optional header values (`phase`, `loop-mode`, `session`) that are blank are treated as absent rather than rejected, so an empty `CLAUDE_CODE_SESSION_ID` cannot fail a `start`. A non-blank value must pass `one_line`.
- `[inferred - audit]` Report control-character rejection applies to the kept (first 15) lines only; a dropped line never reaches the LOG.
- `[inferred - audit]` `status --rule` is checked whenever non-blank, and `status --outcome` for a `running` state is optional but one-line-checked when given.
- `[inferred - audit]` Launcher-supplied text (`launcher_mode`, model, `set_terminal` reason) is tried with `one_line` and otherwise cleaned (`|`, CR, LF and control characters become spaces, 200 chars) rather than rejected: launcher input is trusted and a bad reason must not abort a launch-end record.
- `[inferred - audit]` A `dispatched` event with no budget logs `dispatch=<n>/None`, matching the spike's Python `repr`.
- `[inferred - audit]` The git `base=` value is accepted only if it is hex digits, so a hostile repository cannot inject text into a LOG header through `rev-parse` output (T-27-15, T-27-18).
- `[inferred - audit]` A splice `Err` (I/O fault) is treated like a lost race (`splice-gave-up`, once per transition). A denied dispatch is enforced (`deny` returned) even if recording it fails.
- `[inferred - audit]` `frontmatter()` de-duplicates DIRECTION item ids so a duplicated `### I-01` header cannot become a duplicate YAML key that the typed reader would refuse.
- `[inferred - audit]` `parse_request` also enforces `MAX_REQUEST_BYTES` (the 27-06 server enforces it on the socket as well).
- `actuals.tokens` is chars/4 over the four changed files (127 KB), counted once.

## Known Stubs

None.

## Threat Flags

None. T-27-15 (LOG grammar injection) is mitigated and covered by the rejection table; T-27-16 by `no_request_can_set_crashed_or_stalled_or_rewrite_a_counter`; T-27-17 by the r261a and unknown-key forward-compatibility tests; T-27-18 by `git_read_raw_within` with a 2 s budget and the hex filter; T-27-19 by the byte-identity assertions; T-27-20 by the 15-line report cap and the request-size cap.

## Phase 28 hand-off

- **Embed `RunStore` on a dedicated `std::thread`.** A spawned `serve` child would be refused by the launcher's own run lock (it would need the same `RunLock`); the launcher acquires the lock, builds the store with `RunStore::open`, and moves it onto the thread that owns the socket (27-06 wraps `handle_json` in the server loop). The store is `Send` only if its `Gate` is; keep gate closures `Send`.
- **Install the cap gate with `set_gate`.** The closure receives `(&HookPayload, &RunState)`; read `state.caps.gap_rounds.per_phase` (D-26) and `state.hook_dispatches`/`state.caps.dispatches.limit`, and return `GateDecision::Deny(reason)` with a short one-line reason. The store counts the denial and logs `RUN | dispatch-denied`.
- **Drive the scan and the flush from the launcher ticker.** Call `scan_tick()` (gap rounds) and `flush_heartbeat()` (Pitfall 6) about every 2 s.
- **At launch end:** call `counters::classify_exit(..)`, and when it returns `Some((class, reason))` call `set_terminal(TerminalStatus::Crashed | Stalled, &reason)`. A tripped cap or a driver `RUN end` already yields `None`; `set_terminal` additionally returns `KeptDriverEnd` if a driver end is on record. For a cap trip or operator stop the launcher records `TerminalStatus::Stopped` with its own reason.
- **Call `begin_launch(spec)` after acquiring the lock** and before serving; it logs `crash-detected` for an un-ended previous launch and records the launcher/driver connections. Launcher-owned keys (`auth`, `cost`, `active_seconds`, ...) belong in `RunState::extra`; they survive store rewrites.

## Next Phase Readiness

27-06 can wrap `RunStore::handle_json` in a socket server and a clap client; the exact reply strings are `Response` serialized with `serde_json::to_string` plus `\n`. The `store` module is `#[cfg(target_os = "linux")]` (the splice is); `protocol` is portable.

## Self-Check: PASSED

- Files present: `src/driverctl/protocol.rs`, `src/driverctl/store.rs`, `tests/driverctl_store.rs`, `src/driverctl/mod.rs` (declarations).
- Commits present: `106e611`, `c1fb207`, `bd54fae`, `e677ce0` (ancestors of HEAD).
