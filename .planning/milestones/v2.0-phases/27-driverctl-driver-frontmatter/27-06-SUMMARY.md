---
phase: 27-driverctl-driver-frontmatter
plan: 06
subsystem: driverctl
tags: [rust, unix-socket, flock, serve, replay, sigkill, hand-off]

requires:
  - phase: 27-driverctl-driver-frontmatter
    provides: "27-01 RunLock/acquire_at_with_retry/probe_at, DIRECTION.lock, sweep_stale_temps; 27-04 driverctl client and read_driver_run; 27-05 RunStore (handle_json, begin_launch, scan_tick, flush_heartbeat, set_terminal)"
provides:
  - "crate::driverctl::serve: ServeOptions, serve, socket_path/socket_path_in, validate_run_dir, valid_run_id, DriverctlError (+ exit_code), SOCKET_DIR_NAME, MAX_SUN_PATH, MAX_CONNECTIONS, CONN_TIMEOUT, TICK, STALE_TEMP_AGE, RUN_ID_RE"
  - "gsd-meta-manager driverctl serve --run-dir --project [--sock] (hidden clap variant DriverctlAction::Serve)"
  - "SC1 proof: three recorded spike runs replayed line for line through the Rust client and server"
  - "Phase 28 and Phase 30 hand-off sections in src/driverctl/mod.rs"
affects: [28-launcher, 30-driver-tab]

actuals:
  tokens: 22000
  tasks: 3
  commits: 3
plan_head_before: 168ea362fb57c4bc328c2551b6759359354e7c0c
plan_head_after: 920d565864779504b68d150e255f34833096845c

tech-stack:
  added: []
  patterns:
    - "Lock first, socket second: the run lock is acquired before any socket is created, unlinked or bound, and released last (store drop)"
    - "Sync std threads for connections and ticker; the only tokio use is one current-thread runtime on one thread for the signal wait"
    - "Reply-or-close connection handler: read before the store mutex is taken, panic -> internal: panic reply, poisoned mutex recovered"
    - "Testable core with the env read in a thin wrapper (socket_path -> socket_path_in)"

key-files:
  created:
    - src/driverctl/serve.rs
    - tests/driverctl_serve.rs
    - tests/driverctl_replay.rs
    - tests/fixtures/driverctl/replay/s002-R2/LOG.md
  modified:
    - src/driverctl/mod.rs
    - src/cli.rs
    - src/driverctl_cli.rs

key-decisions:
  - "serve resolves and vets the socket path (and refuses a non-socket at it) BEFORE begin_launch, so a refusal registers no launch; the stale-socket unlink and bind happen after, with the lock held"
  - "A bind failure after begin_launch records stopped/serve:bind-failed before returning, so the run is not left as a false running"
  - "The run id is validated on the raw basename and again on the canonical directory name; the canonical directory must be the direct child of the canonical drivers dir, which also refuses a symlinked run dir that leaves it"
  - "Signal handlers are installed after the lock and before begin_launch, and the install waits for registration, so a signal sent once the server is visible is never lost"

patterns-established:
  - "Mutation-checked guarantees: each single-writer property was shown to fail when its guard was removed"

requirements-completed: [DCTL-01, DCTL-03, DCTL-04]

duration: ~75min
completed: 2026-10-19
status: complete
---

# Phase 27 Plan 06: driverctl serve and the SC1 replay Summary

**`driverctl serve` wraps the 27-05 RunStore in a lock-first unix-socket server (no socket theft, private default path, run-id validation, 2 s gap/heartbeat ticker, graceful SIGTERM stop, SIGKILL -> crashed -> crash-detected), and three recorded spike runs (65 driver events) replay through the real Rust client and server with identical LOG lines and frontmatter counts.**

## Performance

- **Tasks:** 3 (tracer, single-writer guarantees, SC1 replay + hand-offs)
- **Files:** 4 created, 3 modified
- **Tests added:** 11 integration (`driverctl_serve`), 19 unit (`driverctl::serve`), 4 integration (`driverctl_replay`)

## Accomplishments

- `serve` takes `<run-dir>/launcher.lock` before any socket operation, creates `DIRECTION.lock` (both lock files exist for the Phase 28 bwrap mask), sweeps stale splice temps, opens the store, registers a `standalone` launch with a headless driver connection, binds a 0600 socket (umask 0o177 around bind), and serves one thread per connection (32 in flight, then `busy`; 10 s timeouts; 1 MiB cap; panic -> `internal: panic`).
- Shutdown order is the D-20 one: stop accepting, join the ticker, wait for in-flight handlers, `set_terminal(Stopped, "serve:signal")` (a driver `RUN end` is kept), unlink the socket, release the lock last.
- The default socket path is `$XDG_RUNTIME_DIR/gsdmm-driverctl/<run-id>-<sha256(canonical run dir)[:8]>.sock` with a 0700 owned-by-us directory check and a 107-byte limit; no `/tmp` fallback.
- SC1: `s003-r261a` (23 events), `s003-run1` (19) and `s002-R2` (23) replay with exit 0 for every event; item, event, key/value pairs (modulo `base=`), report bodies (including a truncated one), the `dispatch=` counter, `dispatch_audit.logged` and item counts all equal the recordings.

## Task Commits

1. **Task 1 (tracer): serve, DriverctlAction::Serve, tracer test** - `7f0295f` (feat)
2. **Task 2: single-writer guarantees under duplicate serve, SIGKILL, hostile paths, load; unit tests** - `171d131` (test)
3. **Task 3: SC1 replay, R2 fixture, Phase 28/30 hand-offs** - `920d565` (test)

The tracer verify was green before Task 2 started, so expansion proceeded.

## Verification

- `rtk proxy cargo test --no-fail-fast --test driverctl_serve`: 11 passed. `--lib driverctl::serve`: 19 passed. `--test driverctl_replay`: 4 passed (3 replay + a fixture-coverage guard).
- Full `rtk proxy cargo test --no-fail-fast`: 3191 passed, 1 failed - the expected local git-version constants test (`envelope::policy`). With that one test skipped (the plan's own verify command): 3191 passed, 0 failed, every suite ok (includes `async_blocking_guard` 9 and `spawn_seam_guard` 41).
- `rtk proxy cargo clippy --all-targets -- -D warnings`: clean. `cargo fmt`: applied.
- Acceptance: `cmp` of the R2 LOG fixture exits 0; `onFailure`, `fresh-exit` (x2) and `ro-bind` present in `src/driverctl/mod.rs`; `pub mod serve;`, `Serve` and `serve::serve` present; `acquire_at_with_retry` precedes `UnixListener::bind` and `remove_file` in `serve.rs`.
- **Mutation checks (non-vacuity):** (1) removing the group/other mode check, raising `MAX_CONNECTIONS` to 1000, and letting `existing_socket` accept any file make `default_socket_path_*`, `a_flood_of_idle_connections_*` and `stale_socket_*` fail (3 of 3); (2) unlinking `--sock` before taking the lock makes `second_serve_is_refused_and_does_not_steal_the_socket` fail; (3) dropping the `--evidence` flag from the replay makes all three replays fail with `REJECTED: --evidence is required`.

## Replay results per fixture

| Fixture | Run id | Driver events replayed | rc 0 | LOG events equal | `dispatch_audit.logged` | `items` |
|---------|--------|------------------------|------|------------------|-------------------------|---------|
| `spike003/r261a` | `s003-r261a` | 23 | all | all | equal | equal |
| `spike003/run1` | `s003-run1` | 19 | all | all | equal | equal |
| `replay/s002-R2` | `s002-R2` | 23 | all | all | equal | equal |

Total 65 driver events. The recorded `dispatch=N/B` matched on both N and B for every `dispatched` line, so no line needed the budget-edit exception (the test would have printed a `REPLAY-NOTE` line for each; none appeared). `run1` carries the one recorded `truncated=yes` report, reproduced by sending one line past the 15-line cap. No recorded run contains a `recovered` event, so that event is covered by the 27-05 store tests and not by the replay.

## Phase 28 hand-off

Copied from the `src/driverctl/mod.rs` header:

1. Seed the PreToolUse `Agent|Task` hook with `"onFailure": "block"` (D-25).
2. Bind `/dev/null` over `<run-dir>/launcher.lock` and `<run-dir>/DIRECTION.lock` inside bwrap as `--ro-bind /dev/null <path>`; `serve` and the store create both files before the bwrap argv is built (D-25).
3. Hold the run lock across `fresh-exit` relaunches or the reader false-crashes (R3, D-25); acquire before writing `running`, write the terminal status before releasing (R4).
4. Embed `RunStore` on a dedicated `std::thread` rather than spawning `serve`, which the launcher's own lock would refuse (D-03, Pattern 2).
5. Pass `serve::socket_path(run_id, run_dir)` as `DRIVERCTL_SOCK` (D-21).
6. Install the cap gate with `RunStore::set_gate` reading `caps.gap_rounds.per_phase`; deny at limit + 1 and kill at limit + 2 are Phase 28's (D-26).
7. At launch end call `counters::classify_exit` and `RunStore::set_terminal` (D-06, D-17).
8. Put a `driverctl` shim that execs `gsd-meta-manager driverctl "$@"` on the sandbox PATH so the MASTER-PROMPT runs unchanged (SC1).
9. Drive `scan_tick` and `flush_heartbeat` about every 2 s (D-26, D-11), as `serve`'s ticker does.

Phase 30 hand-off (same file): steering edits via `driver::direction::write_body` under `DIRECTION.lock` (D-13); live `control` from the body, not the frontmatter echo; the watcher debounces or ignores `state.json` and `.DIRECTION.md.fm.*` (Pitfall 9); derived status from `read_driver_run` (D-22).

## Deviations from Plan

### Process notes (no behaviour change)

- **TDD shape (Task 2):** Task 2 is `tdd="true"`, but Task 1 already carried the whole of `serve.rs` including `socket_path` and `validate_run_dir` (the module is one coherent unit, and its tracer needed the ticker, shutdown and path resolution to be meaningful). Task 2 therefore has no failing-first RED commit: its tests passed on first run. Non-vacuity is shown by the mutation checks above instead, the same substitute 27-04 and 27-05 used. Likewise Task 3's replay passed on first run (the 27-05 store already reproduced the grammar), and its non-vacuity is the evidence-flag mutation.
- **Trailer attribution:** the dispatch prompt asked for `Co-Authored-By: Claude Opus 5.5`; commits carry `Claude Sonnet 5.5` per the session's attribution instruction, which names the model that actually ran (same call 27-02 and 27-05 made). The `Claude-Session` line is as requested.
- The plan's plan-head ledger file (`gsd-plan-head-before-27-06` under the worktree git dir) could not be written from this sandbox (git-dir writes are refused for worktree agents); `plan_head_before` is the verified base `168ea36` that the worktree check asserted at start.
- `STATE.md`, `ROADMAP.md` and `REQUIREMENTS.md` were not touched, per the dispatch instruction (orchestrator owns them).

**Total deviations:** 0 auto-fixed (no Rule 1-3 triggers). **Impact:** none on the contract.

## Inferred decisions (audit)

- `[inferred — audit]` `serve` vets the socket path and refuses a non-socket at it before `begin_launch`, and a bind failure after it records `stopped` with reason `serve:bind-failed`, so a refused or failed start never leaves a false `running` for the reader to call `crashed`.
- `[inferred — audit]` `--sock` is held to the same 107-byte limit as the default path (a typed `SocketPath` error rather than an opaque `EINVAL` from `bind`).
- `[inferred — audit]` A relative `XDG_RUNTIME_DIR` is refused (it would resolve against the cwd and silently move the socket).
- `[inferred — audit]` Run-id validation runs on the raw basename and on the canonical directory name; the canonical name is the run id handed to the lock and the store, so a symlink alias cannot change which run is addressed.
- `[inferred — audit]` A read that times out mid-request is answered `{"ok":false,"error":"timed out reading the request"}` (best effort); an immediate EOF with no bytes closes silently.
- `[inferred — audit]` The `busy` reply is written without reading the request and with a 1 s write timeout, so refusing cannot itself stall the accept loop.
- `[inferred — audit]` After SIGTERM the server waits at most `CONN_TIMEOUT + 1 s` for in-flight handlers; one that outlives that is abandoned and the kernel releases the lock at process exit (after the socket is unlinked).
- `[inferred — audit]` A second SIGTERM/SIGINT during shutdown is ignored (the handlers stay registered), so a double Ctrl-C cannot interrupt the terminal-status write.
- `[inferred — audit]` The replay's dispatch-budget comparison asserts the regenerated `B` against the fixture body's `dispatches` budget rather than against the recorded `B`, and reports any recorded/regenerated difference; none occurred.
- `actuals.tokens` is chars/4 over the added content (new files plus the changed hunks), counted once.

## Known Stubs

None.

## Threat Flags

None beyond the plan's register. T-27-21 (socket theft) is covered by `second_serve_is_refused_and_does_not_steal_the_socket` and the lock-first mutation check; T-27-22 by `default_socket_path_requires_xdg_runtime_dir` (group-writable dir refused) and the unit tests (symlinked dir, wrong mode); T-27-23 by `run_dir_outside_drivers_is_refused` and the `validate_run_dir` unit tests; T-27-24 by `stalled_connection_does_not_block_the_gate` and the 32-connection flood test; T-27-25 by the hash-stability/difference unit test; T-27-26 by the 0600/0700 mode assertions.

## Self-Check: PASSED

- Files present: `src/driverctl/serve.rs`, `tests/driverctl_serve.rs`, `tests/driverctl_replay.rs`, `tests/fixtures/driverctl/replay/s002-R2/LOG.md`, `src/driverctl/mod.rs`, `src/cli.rs`, `src/driverctl_cli.rs`.
- Commits `7f0295f`, `171d131`, `920d565` are ancestors of HEAD.
