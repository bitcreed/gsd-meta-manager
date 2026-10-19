# Phase 27: driverctl & Driver Frontmatter - Research

**Researched:** 2026-10-18
**Domain:** Rust port of a unix-socket single-writer daemon. Covers flock liveness, `renameat2(RENAME_EXCHANGE)` splicing, a typed YAML frontmatter reader, and replay-testable counters.
**Confidence:** HIGH for the port surface, flock semantics and existing-code reuse, all read or measured this session. MEDIUM for the statistical pass criterion of the race test and for the schema additions recommended for Phase 30.
**Mode:** operator unavailable. Calls inferred from the artifacts are tagged `[inferred — audit]`.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions

#### Protocol & CLI compatibility (DCTL-01, SC1)
- **D-01:** The CLI surface and wire protocol are those of spike 002 `driverctl.py`, kept unchanged:
  - verbs `log <event> …`, `show`, `hook` and `serve`, with the event flags listed in the driverctl.py docstring;
  - one JSON request per connection over a unix socket, the path taken from `DRIVERCTL_SOCK`, created mode 0600;
  - the server unlinks a stale socket at start.

  The spike-002 MASTER-PROMPT must run against the Rust binary with no prompt edits. The acceptance check is a
  scripted replay of a spike-002/003 LOG through the Rust client. — **Reversibility:** costly — the MASTER-PROMPT
  (Phase 29) and the hook settings seeded by the Phase 28 launcher both encode this CLI.
- **D-02:** `driverctl` is a regular clap subcommand of the existing binary, not a second binary (same rule as
  Phase 17 D-01: one build, one version). `[inferred — audit]` Hide it from `--help` (`#[command(hide = true)]`),
  because users never invoke it directly.
- **D-03:** `serve` runs outside the sandbox, on the launcher side. In Phase 27 it is a standalone entry point
  (`serve --run-dir … --project … --sock …`) so it can be tested without the launcher. Phase 28 embeds or spawns it.
  `[inferred — audit]`: Phase 28 is free to call the server as a library (`RunStore`-equivalent) from the launcher
  process, as `launcher_s003.py` did with `dc.RunStore`. Phase 27 therefore exposes the store as a library type
  as well as the subcommand.
- **D-04:** Malformed requests, unknown events, unknown items and enum violations are rejected with **rc=2**, and
  nothing is written for them. The client propagates the server's rc. — from DCTL-04 / SC4.

#### Status enum & 26.1 follow-up (DCTL-03, SC3)
- **D-05:** The run status enum is spike 002's `RUN_STATUS_ENUM` plus **`stalled`**:
  `starting | running | paused | parked | stopped | done | crashed | stalled`.
  - `stalled` means the headless driver exited cleanly (rc 0, stream ended in `result success`) while a dispatched
    worker was still unreported and no cap had tripped. It is a terminal status distinct from `crashed`.
  - Source: the 26.1 follow-up (ROADMAP Phase 27 note; spike MANIFEST; `exitclass.py` forward note). The status
    `exitclass.py` wrote was unvalidated in the spike (26.1-RESEARCH A2); in Rust it is a typed enum variant,
    accepted by the server and the reader.

  — **Reversibility:** costly — the frontmatter v1 schema is read by Phase 30 and written into run dirs on disk.
- **D-06:** `[inferred — audit]` Phase 27 ports `exitclass.py::outstanding_dispatch` as a pure function over
  driverctl's own event model, with the same semantics:
  - a dispatch stays open until a later `report`, `recovered` or terminal `status` (done|parked|failed) for its item;
  - a `RUN | dispatch-denied` cancels only the immediately preceding open dispatch, and only when no item-level event
    lies between them (WR-01). RUN-level launcher lines never count as "between".

  Phase 27 also exposes a launcher-only (server-side, not socket-reachable from the sandbox) terminal-status setter
  for `crashed` / `stalled`, which records the reason. The `classify_exit(rc, result_ok, events)` decision itself
  is called by Phase 28's launcher. Rationale: `outstanding_dispatch` reads driverctl-owned events, and one
  definition of "a dispatch with no report" must be shared by the launcher and the Stop guard (26.1 D-03 lesson).
- **D-07:** Reader liveness (DCTL-03):
  - `running`, `starting` or `paused` with the run flock free reads as **crashed**. This is derived at read time and
    never written by the reader.
  - Terminal statuses (`stopped | done | crashed | stalled | parked`) are shown as written; a free lock is expected
    for them.
  - The probe is a non-blocking flock attempt that is released immediately.
  - `[inferred — audit]` The probe targets the lock file that Phase 28's launcher holds. LNCH-02 says that lock
    reuses Phase 17 CTRL-05 (`driver::lock::lock_path`). Phase 27 adds a `probe_held(path) -> bool` beside
    `acquire` in `src/driver/lock.rs` and takes the lock path as a parameter, so Phase 28 can still choose a per-run
    path.
  - The research step must confirm that a momentary probe acquisition cannot make a concurrently starting launcher
    see a false "already running" refusal. If it can, the probe uses `LOCK_SH|LOCK_NB` or a retry.

#### Gap-round counting (DCTL-04, SC4; 26.1 lesson)
- **D-08:** Gap rounds come only from `gaps_found` VERIFICATION.md writes on disk, never from the driver's account.
  Phase 27 adopts the **26.1 coalescing rule verbatim** from `tools/gapcount.py`, not spike 002's raw hash-distinct
  count. Phase 26 Run 1 shows why: one verifier `sed -i` re-write 6.5 s later inflated the count to 3. The rule:
  - a digest already seen is a `duplicate` and moves neither the count nor the window anchor;
  - otherwise, a write less than `COALESCE_S = 45` s (strict float seconds, file mtime) after the previous
    gaps_found write of the same phase is `coalesced`, and the anchor slides to it;
  - otherwise it is `new`, and the phase count goes up by 1.

  Status is read by one definition: the leading-frontmatter `status:` key, with the value optionally quoted (WR-03).
  State persists as `caps.gap_rounds {seen, per_phase, last_write, coalesced}` in state.json, so a relaunch keeps
  the window.
  — **Reversibility:** costly — Phase 28's deny-at-L+1 / kill-at-L+2 gate reads `per_phase`, and the replay
  fixtures pin the numbers.
- **D-09:** `[inferred — audit]` Ownership of the counter is shifted from Phase 28 to Phase 27. ROADMAP/MANIFEST
  say "Phase 28 ports this rule", but DCTL-04 makes driverctl the owner of the counters and the frontmatter
  displays `gap_rounds`. Phase 27 therefore owns the counting function and its state, and Phase 28 owns only the
  gate/trip/kill that consume `per_phase`.
  - The selftest fixtures from `gapcount.py` (`selftest`, the Phase 26 Run 1 replay that must show the 6.5 s re-write
    as coalesced, and the `r261a` A9 shape) are ported as Rust unit tests.
  - The *observation* trigger (how a new VERIFICATION.md write reaches the store) is a store method,
    `observe_verification(path, mtime, digest)`. `serve` may drive it with a scan or notify. The launcher monitor
    loop can call it directly.
- **D-10:** Dispatches are counted from hook payloads: a `PreToolUse Agent|Task` payload with no `agent_id` is a
  driver-level dispatch and increments `hook_dispatches` (= `caps.dispatches.used`). A nested payload (with
  `agent_id`) does not count. The dispatch audit `logged == observed` is kept in the frontmatter as
  `dispatch_audit {logged, observed, denied}`. — from DCTL-04 / spike 002.
- **D-11:** Heartbeat-only rewrites of DIRECTION.md (hook `PostToolUse *` / `Stop` events with no state change)
  are coalesced to at most one every **120 s** (`HEARTBEAT_REGEN_MIN_S`). state.json still updates on every
  heartbeat. Any non-heartbeat event regenerates immediately. — from DCTL-04 / spike 002 finding 7.

#### Frontmatter write path (DCTL-02, SC2)
- **D-12:** Exchange-verify splice via `rustix::fs::renameat_with(…, RenameFlags::EXCHANGE)`. The steps:
  1. write the new file (new frontmatter plus the current body, byte-for-byte) to a temp file in the same dir;
  2. exchange it with DIRECTION.md;
  3. compare the displaced inode with the one read;
  4. on a mismatch, swap back and retry from a fresh read.

  The human body is preserved byte-for-byte. A body lacking either `## Direction` or `## Items` is never written
  back: the server logs `skipped-insane-body` and leaves the file. Linux-only, behind `#[cfg(target_os = "linux")]`.
  On other platforms the subcommand returns a typed unsupported-platform error, mirroring the `Drive` posture
  (Phase 17 D-05). `[inferred — audit]`: no macOS `renamex_np` path in this phase.
- **D-13:** gsd-mm's own DIRECTION writes take the same flock as driverctl's splice, so they never race. No gsd-mm
  code writes DIRECTION.md today, because the steering UI comes in Phase 30. Phase 27 therefore ships the shared
  primitive:
  - a `direction_lock(run_dir)` and a body-only write helper that never touches the `---` block;
  - Phase 30 must use it.
  - `[inferred — audit]` The lock file is `<run-dir>/DIRECTION.lock`, separate from the run flock, so steering
    edits never contend with liveness.
- **D-14:** Editor-race test (SC2), ported from spike 002 `experiment/race_test.py`:
  - a human writer saves continuously, by rename-save and by in-place save, while the store regenerates in a tight
    loop;
  - a save is LOST when the file still shows an older body 50 ms later;
  - pass criteria: zero lost rename-saves, and an in-place loss rate at or below the spike's measured rate
    (1 / 13466 saves, about 7.4e-5).
  - `[inferred — audit]` The full 15 s run is `#[ignore]` (opt-in, like the conformance oracle test), and a short
    smoke variant runs in the default `cargo test`.

#### Schema & reader (DCTL-03, SC3)
- **D-15:** Frontmatter schema v1 is a typed `DriverFrontmatter` (serde, `serde_yml` 0.0.13, already a dependency)
  in `state_reader`. Its fields follow spike 002's generator:
  - `schema`, `run_id`, `status`, `control`, `heartbeat`;
  - `connected[]` and `items{state, outcome, …}`;
  - `caps{dispatches, wall_clock, gap_rounds{used, limit, phase}}`;
  - `dispatch_audit`;
  - `last_event`.

  Parsing goes through the existing `state_md::extract_frontmatter`. Unknown keys are tolerated, so the reader stays
  forward-compatible. Field-level detail is the researcher's job, reading `driverctl.py:270-310` and the spike's
  `templates/`.
- **D-16:** Golden fixtures are kept for every generated state: template, starting, running, stopped, crashed,
  **stalled**, and multi-connection. Each one must round-trip: the Rust generator emits it and the Rust reader
  parses it into `DriverFrontmatter`. The spike-002 Python outputs are cross-checked as additional reader fixtures.

#### Run-state lessons from 26.1 (carried as constraints)
- **D-17:** Launch-end classification must never collapse "ended with a worker outstanding" into `crashed`, which
  was the spike 002 `launcher.py:362-363` bug. Phase 27's store must accept both terminal outcomes and record the
  evidence string, for example `status=running and no RUN end; lock was free` for crash-detected. `RUN end`
  logged by the driver, or a tripped cap, takes precedence over classification. — from 26.1 D-03 / exitclass.py.
- **D-18:** Counters stay replay-testable offline. Every counting rule (dispatch audit, gap coalescing, outstanding
  dispatch) is a pure function with fixture tests built from recorded spike LOGs and timestamps. This is the
  26.1 D-02 lesson: prove counting offline before any paid run.

#### Musk pass (discuss:pre)
- No scope cuts. DCTL-01..04 all have owners: the spike-002 user requirement for a tool-written frontmatter, and
  the v2.0 re-plan. Lean lane not taken: more than 3 tasks across socket, splice, schema, counters and the race test.

### Claude's Discretion
- Module layout: a new `src/driverctl/` module versus a submodule under `src/driver/`.
- Sync std unix sockets versus tokio for `serve`. The request rate is one per event, so either works.
- Plan split and waves. The schema/reader, counters and splice are largely independent. The socket server and the
  replay test depend on all three.

### Deferred Ideas (OUT OF SCOPE)
- macOS `renamex_np(RENAME_SWAP)` splice: deferred until the sandbox story is settled. Not in this phase.
- The launcher's `classify_exit` call site, cap gate/trip/kill and Stop guard (`stop_guard.py`, at most 3 blocks):
  Phase 28. The GSD stall-watch path (`planner.stall_detection_enabled`): Phase 28/29 carry.
- 26.1 review warnings WR-10 and WR-11 (spike harness pre-registration and budget gate): not relevant to
  driverctl; they stay open in `26.1-REVIEW-DISPOSITION.md`.
- Reviewed Todos (not folded): "Driver tab renders blank pipeline row at terminal heights 8-13" (Phase 30) and
  "The verify-work gate policy must be configurable" (Phase 20/29).
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| DCTL-01 | `gsd-meta-manager driverctl serve\|log\|hook\|show` is the single writer of `state.json`, `LOG.md` and the DIRECTION.md frontmatter. It speaks the spike-002 socket protocol and runs outside the sandbox. | §Wire protocol (byte-level), §Answer A (single-writer lock for `serve`), Pitfall 1 (in-sandbox panic), §Code Examples (clap layout) |
| DCTL-02 | Exchange-verify splice; body preserved byte-for-byte; insane body never written; gsd-mm DIRECTION writes share a flock | §Pattern 3 (splice port: the comparison must be on content as well as inode), §Race test port, §Answer A (DIRECTION.lock is a separate file plus a sandbox mask) |
| DCTL-03 | Typed `DriverFrontmatter` v1 in `state_reader`; liveness from a non-blocking probe of the run flock | §Schema v1 field map (actual generator keys), §Answer A (probe = `LOCK_SH\|LOCK_NB`; corrected crash-derivation table) |
| DCTL-04 | Counters only from tool-observed state; rc=2 rejection; heartbeat coalescing | §Counters (hook dispatch, gap coalescing port, outstanding_dispatch), §Answer B, Pitfall 6 (trailing heartbeat flush) |
</phase_requirements>

## Project Constraints (from CLAUDE.md)

- Rust stable with an MSRV floor: `rust-version = "1.89"` [VERIFIED: Cargo.toml:15]. No new crate is needed for this phase (see Standard Stack), so the floor does not move.
- Read state from files; never require running Claude or GSD. The reader is a file plus flock probe only.
- Non-intrusive: must not interfere with running GSD instances. driverctl writes only inside the run dir. Its git read is `rev-parse` through the no-optional-locks wrapper.
- Portability: the clap subcommand stays ungated. Linux-only internals are `#[cfg]`-gated and return a typed `UnsupportedPlatform` error elsewhere (D-12 and the house style in `src/driver/mod.rs` fact 2).
- Logs go to a file, never stdout. ratatui owns the terminal. **But see Pitfall 1:** the file appender panics where the data dir is not writable, which is the situation inside the sandbox.
- Avoid `std::sync::Mutex` held across `.await`. The repository additionally enforces "no named blocking call inside an `async fn`" mechanically (`tests/async_blocking_guard.rs`, WR-10). That is why serve should be sync std threads.
- Run `cargo test --no-fail-fast`, using `rtk proxy` for raw output (project memory). The release gates are `./scripts/pre-tag-check.sh --container vX.Y.Z`. Any new test that needs a tool absent on `ubuntu-24.04` must self-skip or be `#[ignore]`.
- GSD workflow enforcement: edits only through GSD commands. This document is research only.

## Summary

Phase 27 ports a 575-line Python prototype (`tools/driverctl.py`) whose behavior is fully specified by code that was read this session. Almost every piece needed already exists in the crate:
- rustix 1.1 provides `flock` and `renameat_with(RenameFlags::EXCHANGE)`;
- `serde_yml`, `sha2`, `regex`, `serde_json`, `chrono` and `tempfile` are already dependencies;
- `state_md::extract_frontmatter` parses the generated block;
- `driver::lock` is the house flock idiom, with a guard type and a holder record;
- `git_ops::git_read_raw_within` is the budgeted git read.

**No new crates. MSRV is unchanged.**

The port is mostly mechanical, with six non-obvious traps. Each was found by reading code or by measuring:

1. **The binary panics inside the sandbox before clap runs** (rc=101). This was reproduced under `bwrap --ro-bind / /` and with an unwritable `XDG_DATA_HOME`. Claude Code treats a non-2 hook failure as non-blocking, so a panicking dispatch gate **fails open**.
2. **The lock-probe race is real.** Both `LOCK_EX` and `LOCK_SH` probes make a concurrently starting `LOCK_EX|LOCK_NB` acquirer refuse. In a tight probe loop, 19–40 % of starts were falsely refused with no retry, and 0 % with a 250 ms bounded retry.
3. Read literally, D-12's inode-only comparison would regress in-place saves. Python compares the displaced **body bytes**.
4. D-07's rule makes two cases read as crashed when they are not: a cleanly `paused` run, and a never-launched `starting` run.
5. D-15's field names do not match the generator. The actual keys are listed below.
6. Spike-002 `serve` unconditionally unlinks the socket path. A second `serve` therefore silently steals a live run's socket.

**Primary recommendation:** build `src/driverctl/` as a sync std-thread library (`RunStore`, splice, counters, protocol) plus a thin hidden clap subcommand that is dispatched **before** tracing init.
- **Locks:** gate the store behind the run flock, held for the store's lifetime. Probe with `LOCK_SH|LOCK_NB`. Acquire with `LOCK_EX|LOCK_NB` and a bounded retry. Keep `DIRECTION.lock` as a separate short-hold lock.
- **Counters:** port gap coalescing, `outstanding_dispatch` and `classify_exit` as pure functions with spike fixtures.
- **Splice:** verify on body content as well as inode.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Socket server, state.json/LOG.md/frontmatter writes | Host process outside the sandbox (driverctl `serve` / Phase 28 launcher, library `RunStore`) | — | Single writer; the run dir is read-only inside the sandbox |
| `log`/`hook`/`show` client | Sandboxed CLI (same binary, run by the driver and by Claude hooks) | — | Validates arguments, sends one JSON request, never touches files |
| Run liveness (flock held) | Host OS kernel (flock on `<run-dir>/launcher.lock`) | Reader (`state_reader`) | Kernel-released on death; no pids (spike L5) |
| Frontmatter parse and derived status | `state_reader` (TUI-side reader, Phase 30 consumer) | — | Read-only; never writes the derived `crashed` |
| Gap-round observation | `RunStore` scan tick (`serve`) / launcher monitor | Pure `GapCounter` | Counted from disk, not from the driver |
| Dispatch gate (caps) | Phase 28 launcher, via a `Gate` hook point on `RunStore` | Hook client (fail-closed) | Phase 27 ships the hook point with a default allow |
| DIRECTION body edits (steering) | Phase 30 TUI, via the Phase 27 `write_body` primitive | `DIRECTION.lock` | Same exchange-verify path |

## Standard Stack

### Core (all already in Cargo.toml; nothing to install)
| Library | Version (locked) | Purpose | Why |
|---------|---------|---------|-----|
| rustix | 1.1.5 (`features = ["process", "fs"]`) [VERIFIED: Cargo.toml + Cargo.lock] | `flock`, `renameat_with(…, RenameFlags::EXCHANGE)` | Already the repo's only flock idiom. `FlockOperation::NonBlockingLockShared` exists [VERIFIED: rustix-1.1.5/src/backend/linux_raw/fs/types.rs:586 `NonBlockingLockShared = linux_raw_sys::general::LOCK_SH \| linux_raw_sys::general::LOCK_NB,`] and `const EXCHANGE = linux_raw_sys::general::RENAME_EXCHANGE;` [VERIFIED: same file :310-311]. `renameat_with` is cfg `any(apple, linux_kernel, target_os = "redox")` [VERIFIED: rustix-1.1.5/src/fs/at.rs:296-302] |
| serde_yml | 0.0.13 [VERIFIED: Cargo.toml `serde_yml = "0.0.13"`] | Parse frontmatter into `DriverFrontmatter` | Locked by D-15. 0.0.13 is a shim forwarding to pure-Rust `noyalib` [CITED: github.com/sebastienrousseau/serde_yml]. RUSTSEC-2025-0068 still flags it [CITED: rustsec.org/advisories/RUSTSEC-2025-0068.html]. Use it **only to parse**. Emit with the hand-written emitter (spike `y()`/`flow()` port), which never touches the advisory's emitter path |
| serde / serde_json | 1.x | state.json, wire protocol | Existing |
| sha2 | 0.11.x [VERIFIED: Cargo.toml `sha2 = "0.11.0"`] | Gap-write digest (`sha256(raw)[:16]` hex) | Existing |
| regex | 1.x | DIRECTION parse (`ITEM_RE`, control, budget), LOG `EV_RE`, verification `STATUS_RE` | Existing |
| chrono | 0.4 | `%Y-%m-%dT%H:%M:%SZ` timestamps | Existing |
| std `os::unix::net::UnixListener/UnixStream` | std | Socket server and client | No crate needed. Use sync threads (see below) |

### Supporting
| Library | Purpose | When |
|---------|---------|------|
| tempfile 3 | Test scratch dirs | Tests only (already a normal dependency) |
| assert_fs 1 | Integration test fixtures | dev-dependency, existing |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| std threads + `UnixListener` | tokio `UnixListener` + `select!` | Rejected. Every store op is blocking file I/O + `git` + flock. `tests/async_blocking_guard.rs` forbids those inside `async fn` (WR-10, deadlock reproduced in `tests/driver_lock.rs`). Phase 28's tokio launcher can host the sync store on a dedicated `std::thread` |
| 2 s poll scan of `*-VERIFICATION.md` | notify watcher | Poll is what both spikes ran and what WR-07's stat-read-stat was proven against. notify adds event-coalescing semantics nobody measured |
| hand-written YAML emitter | `serde_yml::to_string` | Rejected: no control over flow style or quoting, and the advisory path is in the emitter |
| `/proc/locks` lock-free probe | flock probe | Linux-only, pid-namespace translation issues, inode matching. Rejected in favor of an SH probe + acquirer retry |

**Installation:** none. **Version verification:** `Cargo.lock` was read this session (rustix 1.1.5, serde_yml 0.0.13, sha2 0.11.1, tracing-appender 0.2.5).

## Package Legitimacy Audit

No external packages are installed by this phase. Every crate used is already in `Cargo.lock`.

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| (none new) | — | — | — | — | — | — |

**Packages removed due to [SLOP] verdict:** none. **Packages flagged [SUS]:** none.

## Answer A: Lock-probe liveness race (mandatory)

### Verdict: YES, the race exists, and D-07's proposed `LOCK_SH|LOCK_NB` mitigation does not close it. A bounded retry on the acquirer closes it.

**Measured on this box** (Linux 7.0.0-38, zfs + tmpfs; throwaway script `scratchpad/flock_probe.py`, using Python `fcntl.flock`, which is the same `flock(2)` syscall rustix issues) [VERIFIED: empirical, output pasted]:

```
E1/E2 probe=LOCK_EX held while launcher does EX|NB -> launcher sees EWOULDBLOCK
E1/E2 probe=LOCK_SH held while launcher does EX|NB -> launcher sees EWOULDBLOCK
E3 probe LOCK_EX vs concurrent probe LOCK_EX -> second probe sees EWOULDBLOCK (false LIVE)
E3 probe LOCK_SH vs concurrent probe LOCK_SH -> second probe sees acquired (ok)
E4 LOCK_EX on an O_RDONLY descriptor -> acquired
E5 same process, second open() + SH|NB while first OFD holds EX -> EWOULDBLOCK
E5b after closing the probe fd, the first OFD still excludes others -> EWOULDBLOCK
E6 2 probers (EX|NB+UN tight loop), 20000 launcher starts, retry budget 0 ms: false refusals=3873 (19.36%) worst acquire wait=0.02 ms
E6 2 probers (EX|NB+UN tight loop), 20000 launcher starts, retry budget 250 ms: false refusals=0 (0.00%) worst acquire wait=63.39 ms
E6 2 probers (SH|NB+UN tight loop), 20000 launcher starts, retry budget 0 ms: false refusals=7992 (39.96%) worst acquire wait=0.03 ms
E6 2 probers (SH|NB+UN tight loop), 20000 launcher starts, retry budget 250 ms: false refusals=0 (0.00%) worst acquire wait=63.45 ms
```

### The exact losing interleaving (false "already running")

```
Reader R (TUI refresh)                     Starting launcher / 2nd serve L
open(launcher.lock, O_RDONLY)
flock(SH|NB)  -> OK  (run is dead/new)
   ── R holds the lock: µs normally; ms if R is preempted here ──
                                           open(launcher.lock, O_RDWR|O_CREAT)
                                           flock(EX|NB)  -> EWOULDBLOCK
                                           read_holder() -> the PREVIOUS launch's record
                                           refuse: "held by run X pgid 1234"   <- false, and it
                                                                                  names a stale pid
close(fd)  (lock released)
```

- Today's `driver::lock::acquire` has exactly this shape. It makes one `NonBlockingLockExclusive` attempt, and on `WOULDBLOCK` it returns `HeldBy`/`HeldByUnknownRun` [VERIFIED: src/driver/lock.rs:171-186]. The refusal names whatever record the *crashed* previous holder left. That invites the user to act on a recycled pgid.
- The probability per real launch is about probe-hold / probe-period: around 1e-5 for a TUI probing every second. It is not zero. It grows with the number of readers (TUI, `driver status` CLI, Phase 30 per-run rows), and it is 19–40 % in a tight loop.

### "Wrongly conclude dead" interleavings (reader side)

The launcher never concludes "dead" from a probe. Its `crash-detected` branch runs only **after** it won the lock, and winning is the proof. The **reader**, however, derives `crashed` wrongly in these cases under D-07 as written:

| # | State on disk | Lock | D-07 says | Truth | Fix |
|---|---|---|---|---|---|
| R1 | `status: starting`, `launches: 0` (a fresh run dir; `RunStore` init default is `starting` [VERIFIED: driverctl.py:223-226 `"status": "starting"`, `"launches": 0`]) | absent/free | crashed | never launched | `starting` + free reads crashed **only if `launches > 0`**. Otherwise show `starting`/not-launched |
| R2 | `status: paused` after `log end --reason pause` (`END_REASONS` maps `"pause": "paused"` [VERIFIED: driverctl.py:47-48 `END_REASONS = {"all-done": "done", "all-parked": "parked", "pause": "paused", "stop": "stopped", "run-budget": "stopped", "fresh-exit": "running"}`]); the MASTER-PROMPT says "`control: pause` -> dispatch nothing new, `end --reason pause`, exit" [VERIFIED: spike-002 MASTER-PROMPT.md:50] | free (the launcher has exited) | crashed | cleanly paused | **Remove `paused` from the crash rule** `[inferred — audit; corrects D-07]`. FRONTMATTER-SCHEMA.md rule 2 named only `running` |
| R3 | `status: running` from `end --reason fresh-exit` | free | crashed | between fresh-per-item launches | Phase 28 obligation: hold the run lock **across** fresh-exit relaunches. Otherwise this is a false crash. Record it in the Phase 28 hand-off |
| R4 | Phase 28 writes `running` *before* acquiring, or releases the lock *before* writing the terminal status | free (transient) | crashed | starting / ending cleanly | Ordering invariant (below) |
| R5 | Probe errno other than EWOULDBLOCK (EACCES, ENOLCK on NFS) | unknown | — | unknown | Tri-state: `LivenessUnknown`, never crashed (the house CR-05 posture, `Liveness::Unknown` [VERIFIED: src/driver/liveness.rs:73-83]) |
| R6 | Two readers probing with `LOCK_EX` at once | "held" (the other probe) | running | crashed | Probe with `LOCK_SH` (E3) |

### Second `driverctl serve` and stale-socket cleanup

Spike `serve()` does `if os.path.exists(sock_path): os.unlink(sock_path)` and then binds [VERIFIED: driverctl.py:464-469]. With no lock, a second `serve` for the same run:
- unlinks the live server's socket;
- binds its own;
- becomes a **second writer** of state.json. The first server keeps running on an orphaned inode, and its in-memory state diverges.

Spike-003 additionally names sockets `<run-id>.sock` in a shared `$XDG_RUNTIME_DIR/gsdmm-driverctl` [VERIFIED: launcher_s003.py:293-295]. Two *projects* with the same `yyyymmdd-slug` run id would therefore collide even with a per-run lock.

### Recommended design (closes all of the above)

1. **Two lock files, two meanings, never shared:**
   - **run lock** `<run-dir>/launcher.lock` [VERIFIED: the spike path `lockf = open(os.path.join(self.rd, "launcher.lock"), "a+")` at launcher.py:199]. It means "the single writer of this run is alive". It is held for the whole life of the `RunStore` by whoever owns the store: Phase 27 standalone `serve`, or the Phase 28 launcher embedding the library.
   - **`<run-dir>/DIRECTION.lock`** (D-13). Short critical sections only, around one splice or one body write.
2. **Reuse the mechanism, not the per-project path.** Generalise `lock.rs` with:
   - `acquire_at(path, holder) -> RunLock`, with the existing `acquire(planning_dir, …)` becoming a wrapper;
   - `probe_at(path) -> LockProbe {Held, Free, Absent, Unknown(errno)}`.
   - Keep `lock_path()` (the Phase 17 per-project `run.lock` [VERIFIED: src/driver/lock.rs:52 `const LOCK_FILE_NAME: &str = "run.lock";`]) untouched for `drive`.
   - `[inferred — audit]`: LNCH-02's "reuses Phase 17 CTRL-05" is read as reusing the mechanism. A per-project lock would forbid two runs per project and would not match the spike's L5 evidence.
3. **Probe = `open(O_RDONLY, no O_CREAT)` → `flock(LOCK_SH|LOCK_NB)` → drop the File.**
   - `NotFound` → `Absent`; `EWOULDBLOCK` → `Held`; any other errno → `Unknown`.
   - SH removes the probe-vs-probe false-live (E3). The reader never creates or truncates the file (house fact 4).
4. **Acquirer = `LOCK_EX|LOCK_NB` with a bounded retry.**
   - Exponential backoff from 1 ms, capped at 50 ms per sleep, total budget **500 ms** `[inferred — audit]`. E6 showed a worst real wait of 63 ms under adversarial probing, so 500 ms gives about 8x headroom.
   - Only after the budget is exhausted does it report `HeldBy`. This keeps D-20.4's "a duplicate start never hangs": it is bounded and still non-blocking per attempt.
   - Use it in `serve` and hand it to Phase 28. Phase 17's `drive` keeps its single attempt unless the planner chooses to adopt it; it is not probed by readers today.
5. **Acquire-before-publish / publish-before-release ordering** (copy the spike-003 order, which already does this [VERIFIED: launcher_s003.py:253-261 lock → `RunStore`; :425-436 verdict → `st.save()` → `stop.set()` → `srv.join` → `flock(LOCK_UN)`]):
   ```
   start: acquire run lock (retry) → load state.json → crash-detected check (status running|starting,
          launches>0, !ended_by_driver) → launches+=1, status=running → save+regen →
          ensure DIRECTION.lock exists → unlink stale socket (safe: we hold the run lock, so no live server
          for this run exists) → bind → chmod 0600 → listen → [Phase 28: spawn driver]
   end:   terminal status written (driver end / cap / classify_exit) → save+regen → stop accept loop →
          unlink socket → drop RunLock LAST
   ```
6. **Socket path:** `$XDG_RUNTIME_DIR/gsdmm-driverctl/<run-id>-<sha256(canonical run dir)[:8]>.sock` `[inferred — audit]`.
   - The dir is created 0700. Verify that it is owned by our uid and not group/world-accessible.
   - If `XDG_RUNTIME_DIR` is unset, refuse instead of using the spike's `/tmp` fallback, which a shared, world-writable `/tmp` makes pre-creatable by another user.
   - Check `sun_path` ≤ 107 bytes and return a typed error otherwise.
   - `serve --sock` stays as an override (D-03).
7. **The sandbox can DoS a visible lock file. Verified:**
   ```
   --- case 1: run dir ro-bound, lock file visible
   sandbox: LOCK_EX taken on read-only bind
   host EX|NB on DIRECTION.lock: EWOULDBLOCK (sandbox holds it)
   --- case 2: lock file masked with --ro-bind /dev/null
   ... PermissionError ...
   host EX|NB on DIRECTION.lock: acquired
   ```
   flock works on an `O_RDONLY` descriptor of a read-only bind (E4 and case 1). Therefore:
   - (a) splice and body-write acquire `DIRECTION.lock` with a bounded retry of about 2 s. On timeout the **splice proceeds without it** and logs `direction-lock-timeout`. This is safe because exchange-verify alone already prevents rename-save loss; the flock only closes the residual in-place window between the two *tool* writers.
   - (b) **Phase 28 hand-off:** bind `/dev/null` over `<run-dir>/DIRECTION.lock` and `<run-dir>/launcher.lock` inside bwrap. `serve` creates both files before the launcher builds the bwrap argv.
   - The run lock itself cannot be stolen while the launcher holds EX.
8. **Lock order:** run lock (long) before `DIRECTION.lock` (short). Nobody ever waits on the run lock while holding `DIRECTION.lock`. Every acquisition is NB with a bounded retry, so deadlock is impossible by construction.

### Test that demonstrates the race is closed (put in `tests/driverctl_lock_race.rs`)

flock is per open file description, so threads with separate `File::open` calls behave exactly like processes (E5). Tests can stay in-process.

1. **Negative control (proves the race exists):** a thread holds `probe_at`-style `LOCK_SH` for 30 ms. The single-attempt `acquire_at` (no retry) must return `HeldBy…`.
2. **Closed:** the same setup, and `acquire_at_with_retry(budget = 500 ms)` must succeed.
3. **Stress:** two threads tight-loop `probe_at()`. Run 2 000 start/stop cycles of `acquire_at_with_retry`; assert 0 false refusals. Run the no-retry variant in the same test and assert it produced more than 0, so the test is not vacuous.
4. **Mutual exclusion kept:** a real holder keeps EX longer than the budget. The second acquire returns `HeldBy` within `budget + 100 ms`, so it never hangs.
5. **Probe vs probe:** two concurrent probes against an unheld file both return `Free`.
6. **Second serve:** start `serve` on a run dir, then start a second one. The second must exit non-zero, and the first must still answer `show` over the original socket path (socket not stolen).
7. **SIGKILL liveness (spike L5 equivalent):** spawn `serve` as a child process, `kill -9` it, and the reader must derive `crashed`. Restart `serve`: `LOG.md` gains `RUN | crash-detected | previous_launch=1 | evidence=…`.
8. **Reader table:** one unit test per row R1–R6.

## Answer B: Gap-round counter move (mandatory)

### Verdict: **ENDORSE-WITH-CHANGES.**

**Why the move is sound:**
- Phase 27 SC4 already reads "Gap rounds come from `gaps_found` VERIFICATION.md writes on disk, never from the driver's own account" [VERIFIED: ROADMAP.md:1021-1024]. DCTL-04 lists gap rounds among the counters driverctl owns [VERIFIED: REQUIREMENTS.md:157-162]. Phase 27 therefore has to count gap rounds *somehow* to meet its own success criterion.
- The frontmatter field `caps.gap_rounds {used, limit, phase}` is generated by driverctl [VERIFIED: driverctl.py:279-299].
- If Phase 27 used spike-002's raw hash-distinct rule [VERIFIED: launcher.py:119-140], Phase 27 would ship the exact Phase 26 Run 1 inflation: the run1 LOG shows counts 1, 2 and 3 with the 2→3 step 8 s apart [VERIFIED: spike-003 results/run1/LOG.md:56,86,87]. That is the bug 26.1 was created to fix.
- The MANIFEST line "Phase 28 ports this rule" predates Phase 27's detailed scoping. It was about *where the counter lives in the spike* (the launcher), not a deliberate design choice.
- The split is clean: counting is data (Phase 27). The gate predicates `n >= gl + 1` / `worst > gl + 1`, `mentions_phase` and the scope kill are policy (Phase 28) [VERIFIED: gapcount.py:31-33 docstring; launcher.py:170-174, 328-329].

**Changes required (the "WITH-CHANGES"):**
1. **Phase 27 must also own the observation trigger,** not only the pure function. Otherwise SC4 cannot be demonstrated in Phase 27, because without the launcher nothing would feed `observe_verification`.
   - The standalone `serve` runs a 2 s scan tick: glob `<project>/.planning/phases/*/*-VERIFICATION.md`.
   - It uses WR-07 stat-read-stat and skips a file whose `mtime_ns` moved [VERIFIED: launcher_s003.py:150-158].
   - It uses the `fm_status` port and passes the `st_mtime` float to `observe` [VERIFIED: :159-163].
   - Phase 28's launcher uses the same scan function on the embedded store.
2. **The `Gate` hook point lives on `RunStore`** in Phase 27 (the spike's `self.gate = None  # callable(payload) -> (allow, reason)` [VERIFIED: driverctl.py:218]). The default is allow, and Phase 28 installs the cap gate. Without it, Phase 28 would have to reopen the store.
3. **Planning-artifact edits** (orchestrator/planner to apply, no requirement-ID changes):
   - `.planning/spikes/MANIFEST.md`: the two lines saying "Phase 28 ports this rule" / "Phase 28 ports the coalescing gap count (`COALESCE_S = 45`)" → "Phase 27 ports the counter (DCTL-04); Phase 28 consumes `per_phase` in its gate/trip".
   - `REQUIREMENTS.md` DCTL-04: append "coalesced per the 26.1 rule (`COALESCE_S = 45`, strict, mtime-anchored)". LNCH-03's gap-round bullet: append "reading driverctl's `caps.gap_rounds.per_phase`". The traceability rows are unchanged (DCTL-04 → Phase 27 already).
   - `ROADMAP.md` Phase 27 SC4: append the coalescing sentence. Phase 28 SC2: add "(gap count source: the Phase 27 store)". Do **not** edit STATE.md's historical decision lines; add a new decision entry instead.
4. **Port the status definition from `gapcount.fm_status`, not from the dashboard's reader.**
   - `STATUS_RE = re.compile(r"""^status:\s*["']?([A-Za-z_]+)""", re.M)` on the text before the first `\n---`, only when the text starts with `---` [VERIFIED: gapcount.py:63-73].
   - The dashboard's `VerificationStatus::from_raw` does **not** strip quotes [VERIFIED: src/state_reader/disk_status.rs:101-111 `match trimmed.to_ascii_lowercase().as_str() { "passed" => …, "gaps_found" => VerificationStatus::GapsFound, …, _ => VerificationStatus::Unknown(trimmed.to_string()) }`]. A `status: "gaps_found"` therefore counts as a gap round while the dashboard shows `Unknown("\"gaps_found\"")`.
   - That is the WR-03 divergence again, across crates instead of across scripts. Record it as a todo (out of scope here). Do not "unify" by using the dashboard reader: the 26.1 fixtures pin `fm_status`.

**What it leaves for Phase 28:** deny at L+1 for dispatches naming the phase, kill at L+2, the trip LOG line `cap-tripped`, scope kill, the L7 acceptance cases, and the real-Opus deny/kill carried risk.

**Risks:**
- (a) `COALESCE_S = 45` is itself `[inferred - audit]` in the spike [VERIFIED: gapcount.py:54-56]. It is persisted only implicitly (via `last_write`), so changing it later re-scores no history. Keep it a named constant with that provenance comment.
- (b) Phase 28 might run a second scanner beside `serve`. The digest `seen` set makes re-observation idempotent, but only under the same store mutex. Embedding the library (one store, one scanner) avoids it.
- (c) Phase directory names that do not start with a digit made the Python `re.match(...).group(1)` raise [VERIFIED: launcher_s003.py:162]. Rust must skip them, not abort the scan tick.

## Architecture Patterns

### System Architecture Diagram

```
                 INSIDE bwrap (run dir read-only)                     OUTSIDE (host)
 ┌───────────────────────────────────────────┐        ┌─────────────────────────────────────────────┐
 │ driver (claude -p)                         │        │ RunStore owner: `driverctl serve` (P27)     │
 │   `driverctl log …`  ──┐                   │        │   or Phase 28 launcher (library)            │
 │ Claude hooks:          │  wrapper script   │        │                                             │
 │   PreToolUse Agent|Task├─► gsd-meta-manager│ JSON\n │  accept loop ──► parse+validate ──rc2──┐    │
 │   PostToolUse * / Stop │   driverctl …     ├──────► │     (unix socket 0600, per-conn timeout)│    │
 │   SessionStart/End     │  (no tracing, no  │ ◄──────┤  op: log | hook | show     reject: ok:false│
 └────────────────────────┴── config, no fs)  │ JSON\n │      │                                  │    │
                                              │        │      ▼                                  │    │
                                              │        │  RunStore (Mutex) ── Gate (P28 caps) ──►deny│
                                              │        │      │ append LOG.md (one O_APPEND write)   │
                                              │        │      │ state.json (atomic tmp+fsync+rename) │
                                              │        │      ▼                                      │
                                              │        │  regen? (heartbeat-only → ≤1/120 s + trailing│
                                              │        │      flush)  ── DIRECTION.lock (bounded) ──► │
                                              │        │  exchange-verify splice of DIRECTION.md      │
                                              │        │  scan tick 2 s: *-VERIFICATION.md → GapCounter│
                                              │        │  holds <run-dir>/launcher.lock (EX) for life │
                                              │        └──────────────────────┬──────────────────────┘
                                              │                               │ files
                                              │                               ▼
 ┌────────────────────────────────────────────┴───────┐   TUI / Phase 30 reader (state_reader):
 │ human editor (vim rename-save / VS Code in-place)  │   read DIRECTION.md → extract_frontmatter →
 │ edits DIRECTION.md body ─────────────────────────► │   DriverFrontmatter; probe launcher.lock
 └────────────────────────────────────────────────────┘   (SH|NB) → derived status (crashed/unknown)
```

### Recommended Project Structure
```
src/driverctl/                 # new top-level module (Claude's discretion → recommend top-level, not under
│                              #   src/driver/: driver/ is the Phase 17-21 subprocess driver; this is a store)
├── mod.rs                     # header doc (house style), pub surface, UnsupportedPlatform for non-Linux
├── protocol.rs                # Request/Response serde types, op/event enums, arg validation (one_line, enums)
├── client.rs                  # log/show/hook client: env DRIVERCTL_SOCK, 15 s timeout, rc mapping
├── store.rs                   # RunStore: state.json model, handle(), begin_launch/end_launch, set_terminal
├── frontmatter.rs             # emitter (y()/flow() port) — reader type lives in state_reader
├── direction.rs               # split_frontmatter, parse_direction, body_sane
├── splice.rs   #[cfg(linux)]  # exchange-verify splice + write_body + DIRECTION.lock
├── counters.rs                # GapCounter, fm_status, outstanding_dispatch, classify_exit (pure)
├── log.rs                     # LOG line writer + EV_RE parser (pure)
├── scan.rs                    # VERIFICATION.md scan (stat-read-stat)
└── serve.rs    #[cfg(linux)]  # listener, per-conn threads, scan/flush ticker, shutdown
src/state_reader/driver_frontmatter.rs   # DriverFrontmatter, RunStatus, LockProbe→DerivedStatus
src/driver/lock.rs             # + acquire_at, acquire_at_with_retry, probe_at (generalised, same idiom)
tests/driverctl_*.rs           # protocol replay, race, lock race, reader fixtures, prompt-parse
tests/fixtures/driverctl/      # COPIES of spike LOG/DIRECTION/state/launcher-result files (.planning/ is excluded
                               #   from the crate package — Cargo.toml `exclude = [".planning/", …]`)
```

### Pattern 1: Dispatch `driverctl` before tracing and color-eyre init
**What:** `main.rs` currently initialises a `tracing_appender::rolling::daily` file appender *before* `Cli::parse_checked()` [VERIFIED: src/main.rs:61-76]. `RollingFileAppender::new` ends in `.expect("initializing rolling file appender failed")` [VERIFIED: tracing-appender-0.2.5/src/rolling.rs:152-156].
**Fix:**
- Parse first. On `Commands::Driverctl`, run the sync driverctl entry point (via `tokio::task::spawn_blocking` or a sync `fn main` split) and `std::process::exit(rc)`, without tracing, color-eyre or config.
- Independently, make tracing init fallible (`RollingFileAppender::builder()…build()` returns `Result`; on error, fall back to no file logging). That hardens every other in-sandbox invocation as well.
**Regression test (CI-safe, no bwrap):** run the binary with `XDG_DATA_HOME=/proc/nonexistent-gsdmm` and `DRIVERCTL_SOCK` pointing at a missing path. `driverctl hook` with a gate payload must print the deny JSON and exit 0. `driverctl log start …` must exit with the transport rc, **not 101**.

### Pattern 2: Store gated by the run lock (single writer by type)
`RunStore::open(run_dir, project, lock: RunLock)` takes ownership of the guard, so "a store without the lock" is unrepresentable. Standalone `serve` acquires the lock itself; Phase 28 acquires and passes it in. D-03 allows "spawns" for Phase 28, but a spawned `serve` child would be refused by its parent's lock. Recommend that Phase 28 **embed** the library `[inferred — audit]`.

### Pattern 3: Exchange-verify splice (port of `splice_frontmatter`, strategy=exchange)
The source is driverctl.py:137-187 [VERIFIED]. The loop:
1. Read the file. `split_frontmatter` requires `text.startswith("---\n")` and a `"\n---\n"` end with `generated_by: driverctl` inside [VERIFIED: :79-85].
2. Bail out with `skipped-insane-body` unless the body contains both `## Direction` and `## Items` (a substring test) [VERIFIED: :131-134].
3. Write `fm + body` to a same-dir temp file and chmod it to the original mode.
4. `renameat_with(dir, tmp, dir, "DIRECTION.md", EXCHANGE)`.
5. Read the displaced file (now at tmp). **If its body == the body we read**, unlink tmp → `ok`.
6. Otherwise exchange back. If tmp then holds exactly our bytes, unlink and retry. If not, a newer rename-save landed: `rename(tmp, path)`, so the newest human bytes win, then retry.
7. Up to 5 tries with a sleep of `5 ms * attempt` → `gave-up`.

**Correction to D-12** `[inferred — audit]`: compare **body bytes**, which is what Python does. An inode comparison alone misses an in-place save (same inode, new content) that completes between the read and the exchange, so it would raise in-place loss. Comparing inode *and* body is allowed. Body-only is the measured design.

**Add, beyond the spike** `[inferred — audit]`:
- fsync the temp file before the exchange; otherwise a power loss can surface an empty DIRECTION.md;
- tolerate a CRLF leading block (`---\r\n`), so a CRLF-converting editor does not cause a second block to be prepended on every regen;
- use a fixed temp prefix (`.DIRECTION.md.fm.<pid>.<n>`) and sweep stale ones at `serve` start.

### Pattern 4: Shared body-write primitive for Phase 30 (D-13)
`write_body(run_dir, |old_body| -> new_body)`:
1. Take `DIRECTION.lock` (bounded).
2. Read the file and split it.
3. Compute the new body. Refuse if the new body is not sane.
4. Run the same exchange-verify loop with the **current** tool block kept verbatim.

Splice and body-write therefore share one loop and differ only in which half they replace.

### Pattern 5: Hook client (fail-closed gate)
Port of `hook_main` [VERIFIED: driverctl.py:518-539]:
- Read stdin. Invalid JSON → exit 0 silently.
- Forward only `hook_event_name, session_id, agent_id, agent_type, tool_name, source, reason` plus `prompt = tool_input.prompt[:4000]`. That is 4000 **chars**: use `chars().take(4000)`.
- `is_gate` = PreToolUse ∧ tool ∈ {Agent, Task} ∧ `agent_id` is null.
- Transport error: on the gate, print the deny JSON with reason `driverctl unreachable (…); launcher-side caps cannot be checked`; on anything else, exit 0.
- `{"deny": r}` → print `{"hookSpecificOutput": {"hookEventName": "PreToolUse", "permissionDecision": "deny", "permissionDecisionReason": "LAUNCHER CAP: " + r}}` and exit 0. That shape matches current docs [CITED: code.claude.com/docs/en/hooks].
- **Deviation recommended** `[inferred — audit]`: on the gate, `{"ok": false}` (a server-side internal error) must also deny. Python lets it through.

**Phase 28 hand-off:**
- add `"onFailure": "block"` to the seeded PreToolUse Agent|Task hook. Claude Code v2.1.295+ supports it, and any non-2 failure or timeout otherwise lets the tool proceed [CITED: code.claude.com/docs/en/hooks];
- the hook timeout is 20 s in the spike seeding [VERIFIED: launcher.py:90];
- SessionEnd hooks share a 1.5 s budget [CITED: same], so the client must stay fast.

### Anti-Patterns to Avoid
- **`serve` that unlinks the socket before taking the run lock.** This is how a second serve steals a live run (Answer A).
- **Probe with `LOCK_EX`.** Two readers produce a false-live (E3).
- **Treating `paused` / never-launched `starting` as crash candidates** (R1, R2).
- **Running the gap scan or store I/O inside an `async fn`.** This trips `tests/async_blocking_guard.rs` and repeats the WR-10 deadlock.
- **`serde_yml::to_string` for emission.** No quoting control, and it is the advisory path.
- **Inode-only displaced check** (Pattern 3).
- **Logging unvalidated values into LOG.md.** Python validates `step/outcome/evidence/question/reason` with `one_line`, but writes `phase`, `loop_mode`, `session`, `mode` and the hook's `session_id/source/reason` raw [VERIFIED: driverctl.py:353-354, 369-372, 440, 446]. A `|` or newline there corrupts the grammar `EV_RE` parses. Validate every value written into a header line `[inferred — audit]`.

## Wire Protocol: byte-level compatibility (DCTL-01, SC1)

All of the following are [VERIFIED: driverctl.py], with line refs.

| Aspect | Spike behavior | Rust requirement |
|---|---|---|
| Framing | Client sends `json.dumps(req) + "\n"` (:508). Server reads until the buffer ends with `\n` **or EOF** (:479-484), then parses the whole buffer | Accept trailing whitespace and a missing final `\n` at EOF. Cap the request at 1 MiB → `{"ok":false}`. Ignore extra connections' ordering (serial at the store) |
| Response | `json.dumps(resp) + "\n"` (:491); the client reads to `\n` (:510-514) | Compact `serde_json::to_string` + `\n` is fine, since both sides `json.loads`. Non-ASCII may be raw UTF-8 (Python emits `\uXXXX`; both are valid JSON) |
| Ops | `op ∈ {"log","hook","show"}` (:330-337); anything else → `Invalid` → `{"ok": False, "error": …}` | Same. **No** socket op for the launcher-only terminal setter (D-06) |
| Log request | `{"op":"log","event":<str>,"args":{…}}`. The args keys are argparse dests: `item, step, round, phase, status, done_when, evidence, state, rule, outcome, question, action, reason, found, mode, loop_mode, session`, plus `body` for report (:547-563). **Underscores, not hyphens**, and all values are **strings** | Deserialize with those exact keys. `round`/`found` arrive as strings: parse to int, and a non-int → rc 2 (Python raised ValueError → `internal: …` → client rc 2) |
| Success replies | `{"ok": True}`, `{"ok": True, "deny": reason}`, `{"ok": True, "frontmatter": "<text>"}` | Same field names |
| Client rc | `log`: ok → print `ok`, rc 0. `ok:false` → stderr `driverctl: REJECTED: <error>`, rc **2** (:566-571). `report` with a tty stdin → rc 2 (:560-562). Unset `DRIVERCTL_SOCK` → SystemExit message (rc 1). Connect error → traceback (rc 1). `show` prints the frontmatter with no extra newline (:556) | Same strings. Transport errors → rc 1 `[inferred — audit]`; the MASTER-PROMPT only distinguishes `REJECTED` [VERIFIED: MASTER-PROMPT.md:39-40]. clap usage errors exit 2, the same as argparse [VERIFIED: clap_builder-4.6.7/src/util/mod.rs:32 `pub(crate) const USAGE_CODE: i32 = 2;`] |
| CLI flags | `log <event>` with 17 optional `--flag value` (:547-549); `start` defaults `session` from `CLAUDE_CODE_SESSION_ID` (:564-565). argparse allows unambiguous long-flag abbreviations by default | All flags `Option<String>`. Optionally enable clap `infer_long_args(true)` for abbreviation parity (exists [VERIFIED: clap_builder command.rs:1786]). The MASTER-PROMPT uses only full flags [VERIFIED: MASTER-PROMPT.md:25-36] |
| Timeouts | Server per-conn `settimeout(10)` (:478); client 15 s (:506) | Keep the client at 15 s. **Server: a thread per connection** (bounded, e.g. 32) with a 10 s read timeout, so a stalled sandbox connection cannot starve the gate hooks (a hook timeout fails *open*) `[inferred — audit]` |

**Validation rules to port verbatim** [VERIFIED: driverctl.py]:
- `one_line`: required, no `\n`, no `|`, ≤ 200 chars (question ≤ 300), then strip (:68-74).
- start `mode ∈ {interactive, headless}` (:350-352).
- dispatched: `known_item` + `one_line(step)` + int `round` (:357-372).
- report `status ∈ {done, partial, blocked, failed}`, body non-empty, truncated to 15 lines with `truncated=yes`, and `FINDINGS: N` parsed (:373-386).
- verified `done_when ∈ {met, not-met}` + `one_line(evidence)` (:387-392).
- status `state ∈ STATE_ENUM`; `rule ∈ RULE_ENUM` required when parked; `outcome` required unless running (:393-404).
- needs-human `question` (≤ 300).
- recovered `action ∈ {done, resume, reset, park}` + `reason` (:408-413).
- end `reason ∈ END_REASONS` (:414-420).
- `known_item` accepts `RUN` or an item present in DIRECTION.md (:339-344).

**Tightenings** `[inferred — audit]` (none of them affects a MASTER-PROMPT invocation; checked against every recorded spike LOG, where no item-level event names `RUN`):
- reject `--item RUN` on item-level events;
- validate `rule` whenever it is given, not only for parked;
- validate `phase`/`loop_mode`/`session` with `one_line`;
- reject control characters other than tab in report body lines.

**Enums** [VERIFIED: driverctl.py:43-48, verbatim]:
```
STATE_ENUM = ("pending", "running", "done", "parked", "failed")
RUN_STATUS_ENUM = ("starting", "running", "paused", "parked", "stopped", "done", "crashed")
RULE_ENUM = ("budget", "gap-rounds", "divergence", "no-progress", "needs-human", "scope",
             "run-budget", "launcher-cap")
END_REASONS = {"all-done": "done", "all-parked": "parked", "pause": "paused", "stop": "stopped",
               "run-budget": "stopped", "fresh-exit": "running"}
HEARTBEAT_STALE_AFTER_S = 300      # readers: running + heartbeat older than this = stale
HEARTBEAT_REGEN_MIN_S = 120        # heartbeat-only frontmatter rewrites are coalesced to this
MAX_REPORT_LINES = 15
ITEM_RE = re.compile(r"^### (I-\d+)\b", re.M)
FM_MARKER = "generated_by: driverctl"
```
Rust `RunStatus` = these 7 + `Stalled` (D-05).

**SC1 acceptance tests:**
1. **Prompt-parse test:** extract every `driverctl …` line from `MASTER-PROMPT.md:25-36`, substitute the placeholders, and assert the clap parser accepts each one and builds the expected request JSON. Copy the prompt into the fixtures.
2. **LOG replay:**
   - Map each driver-originated LOG event (`start, reconcile, dispatched, report (+4-space detail lines as stdin), verified, status, needs-human, recovered, end`) of the r261a/run1/R2 LOGs back to `driverctl log …` argv. Drop the tool-computed `base`/`dispatch` keys.
   - Run them through the Rust client against a Rust `serve` with that run's DIRECTION.md.
   - Assert the regenerated LOG matches line-for-line modulo timestamps and `base=`, and that `dispatch_audit.logged` and the item counts match the recorded frontmatter.
3. **Recorded-response fixtures** (optional, no Python in CI): commit request/response pairs captured once from the Python server. Do not run python3 in `cargo test`: the conformance-oracle precedent shows how an absent tool on the runner blocks a publish.

## Schema v1 field map (actual generator, driverctl.py:268-318) — reconciles D-15

The D-15 shorthand (`schema`, `heartbeat`, `wall_clock`, `items{state,outcome}`) does **not** match the generator. The actual keys, in emission order, are [VERIFIED: driverctl.py:284-317]:

| Key | YAML form | Rust type (reader) |
|---|---|---|
| `generated_by` | `driverctl <VERSION>   # TOOL-WRITTEN…` (a plain scalar + comment) | `String` (marker check = the block contains `generated_by: driverctl`) |
| `driver_frontmatter_version` | `1` | `u32`; an unknown major → `UnsupportedVersion` (the schema doc says "Readers reject unknown majors" [VERIFIED: FRONTMATTER-SCHEMA.md table]) |
| `run_id` | JSON-quoted | `String` |
| `status` | bare enum word | `RunStatus` (8 variants) + a tolerant `Unknown(String)` per the house wire-enum posture |
| `status_reason` | quoted/null | `Option<String>` |
| `control` | quoted/null (an echo of the body at the last regen) | `Option<String>`. Phase 30 must read live control from the body, not here |
| `current_item` / `current_phase` / `current_step` | quoted/null | `Option<String>` |
| `items` | `{total, pending, running, done, parked, failed}` counts | `ItemCounts` (u32s) |
| `caps.dispatches` | `{used, limit}` | `Cap<u32>` with `limit: Option<u32>` |
| `caps.wall_clock_min` | `{used, limit}`; Python prints `12.0` | `used: f64` |
| `caps.gap_rounds` | `{used, limit, phase}` (worst phase) | `GapCap { used: u32, limit: Option<u32>, phase: Option<String> }` |
| `dispatch_audit` | `{logged, observed, denied}` | u32s |
| `launches` | int | u32 |
| `connected` | `[]` or block list of flow maps `{role, state, session_id, model, mode, pid, scope_unit, host, started_at, last_heartbeat}` | `Vec<Connection>`, all `Option` except role/state |
| `heartbeat_stale_after_s` | `300` | u64 |
| `last_event` | `{at, item, event, summary}` | all `Option<String>` |
| `updated_at` | quoted | `String` |

**Recommended additive v1 key** `[inferred — audit; costly to reverse]`: Phase 30 SC1 requires "per-item state" and "All of it is read from frontmatter" [VERIFIED: ROADMAP.md:1141-1149]. The spike frontmatter carries only counts, so add
`item_states:` followed by one line per DIRECTION item, `  "I-01": {state: "running", dispatches: 1, outcome: null}`. It is missing from the Python fixtures, so `#[serde(default)]` is required.
Other emitter notes:
- **Tie-break for `gap_rounds.phase`:** Python uses dict insertion order with `>=`. Rust maps are sorted, so define it as "highest count, ties → most recent `last_write`" `[inferred — audit]`.
- Emitter rules (spike): strings via JSON quoting; nested maps single-line flow; no newline in any value; the closing `---` on its own line [VERIFIED: FRONTMATTER-SCHEMA.md "Emitter rules"].

**Fixtures available this session** (17 Python-generated DIRECTION.md files) [VERIFIED: grep `^status:` over spike results]:
- status `stopped` ×4 (L1-final, l7, r261-l7, r261-l8);
- `parked` ×11;
- `done` ×1 (r261-run2);
- `crashed` ×1 (run1b);
- the template `templates/DIRECTION.md` (status `running` with `{run-id}` placeholders, two connections).

**No recorded `starting` or `stalled` frontmatter exists.** Those (and `running`, multi-connection) must come from the Rust generator (D-16).

## Counters (DCTL-04) — pure functions to port

| Function | Source [VERIFIED] | Notes |
|---|---|---|
| `GapCounter::observe(phase, ts, digest, file) -> (Kind, count)` | gapcount.py:81-105 | `if prev is not None and count >= 1 and abs(ts - prev) < self.window` → coalesced, `last_write = max(prev, ts)`. Duplicate digests are untouched. State keys `seen, per_phase, last_write, coalesced`. Accept the S2-shaped state lacking `last_write`/`coalesced` (selftest :322-324) |
| `fm_status(raw)` | gapcount.py:63-73 | Byte input decoded lossily; `\xff\xfe---…` → None (:338) |
| phase key | `re.match(r"0*(\d+(?:\.\d+)?)", basename(dirname))` (launcher_s003.py:162) | `01-s` → `"1"`, `26.1-x` → `"26.1"`. Skip on no match |
| digest | `hashlib.sha256(raw).hexdigest()[:16]` (:161) | sha2 |
| `outstanding_dispatch(events)` | exitclass.py:59-83, `DONE_STATES = ("done", "parked", "failed")` (:52) | WR-01 cases (a)–(f) from the selftest :297-308 |
| `classify_exit(rc, tripped, ended_by_driver, result_ok, outstanding)` | exitclass.py:92-99 | The 27-CONTEXT specifics require the Run 1b shape → `stalled`. Port it as a pure fn; Phase 28 calls it |
| LOG parser | `EV_RE = r"^- (\d{4}-\d\d-\d\dT\d\d:\d\d:\d\d(?:\.\d+)?Z) \| (\S+) \| (\S+)((?: \| .*)?)$"` (dup_check.py:40-62) | kv split on `" \| "`, partition on `=` |
| hook dispatch count | driverctl.py:449-459 | Increment only when the gate allows; `denied_dispatches` otherwise; `caps.dispatches.used = hook_dispatches` |
| heartbeat coalescing | driverctl.py:236-242, 460 | state.json always; regen if `now - last_regen >= 120` |

**Fixture tests to port** (D-09, D-18):
- the whole `gapcount.selftest` value table (534 s rounds; 44.9/45.0/44.999999 boundaries; sliding anchor 20 s ×5; duplicate anchor; phase independence; JSON round-trip; S2 shape);
- the Run 1 replay: `OLD [1,2,3]` → `NEW [1,2]`, coalesced = 1;
- the r261a state.json shape `per_phase {"1": 3}`, `coalesced {"1": 2}`, 5 seen digests [VERIFIED: results/r261a/state.json];
- the exitclass matrix, plus the run1b → `stalled` and run1 → `parked` replays over copies of `LOG.md` + `launcher-result-1.json`.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| flock / renameat2 FFI | ctypes-style `libc` decls | rustix `flock`, `renameat_with` | Repo posture: no `libc`/`nix` [VERIFIED: Cargo.toml rustix comment] |
| Frontmatter split | New splitter for the reader | `state_md::extract_frontmatter` [VERIFIED: src/state_reader/state_md.rs:325-340] | Proven against generated files in spike 002 |
| Git HEAD for `base=` | Raw `Command::new("git")` | `git_ops::git_read_raw_within(root, &["rev-parse","--short","HEAD"], budget)` [VERIFIED: src/state_reader/git_ops.rs:244-257] | No-optional-locks plus a budget. The store mutex is held during the call, and a hung git must not freeze the gate |
| Lock guard / holder record | New lock type | Generalised `driver::lock` (`RunLock`, `LockHolder`, never truncate on open, write the record after winning) | House facts 1–5 already encode the pitfalls |
| Atomic state.json | Ad-hoc write | tmp + fsync + rename (`atomic_write` port) | Readers never see a torn state |
| YAML emitter | serde_yml serializer | Hand port of `y()`/`flow()` with `serde_json::to_string` for strings | Format-correct by construction |

## Race test port (SC2, D-14)

**Source:** race_test.py [VERIFIED: experiment/race_test.py:22-90]:
- the human writer keeps whatever header it loaded and writes `fm + BODY(k)`, either rename-save (tmp + `os.replace`) or in-place (`open("w")`);
- it sleeps `uniform(0, 2 ms)` and reads back; if the read is not `k`, it waits 50 ms → LOST if still not `k`, otherwise transient;
- the tool splices `---\n{FM_MARKER} race\nn: {n}\n---\n` in a tight loop.

**Spike numbers** [VERIFIED: results/race-test.txt]:
- `exchange rename saves= 12943 regens= 42364 LOST= 0 transient= 2 … tool={'ok': 42363, 'gave-up': 1}`
- `exchange inplace saves= 13466 regens= 50699 LOST= 1 … 'skipped-insane-body': 861`

**Rust port:**
- threads, with separate File handles (equivalent at the syscall level);
- the human and the tool call the library splice;
- keep a test-only `naive` strategy as a **positive control**: naive rename must show LOST > 0, which proves the detector works, in the house non-vacuity style.

**Pass criterion: needs a decision** `[inferred — audit]`. "In-place loss ≤ 7.4e-5" is ill-posed for an implementation whose true rate *equals* the spike's. With about 13k saves, a Poisson mean of 1 gives P(≥2 losses) ≈ 26 %, so the literal assertion is flaky by construction. Recommendation:
- assert `lost_rename == 0` (exact);
- assert `lost_inplace ≤ poisson_q99(7.4e-5 × saves_inplace)`, a one-sided "not worse than the spike" test at 1 % false-fail;
- print the observed rate for the record.
- Full run: 15 s per style, `#[ignore]`. Smoke: about 1 s, rename + in-place, asserting `lost_rename == 0` and a sane final body, in the default `cargo test`.
- Also add a deterministic test: two writers (splice and `write_body`) **both under `DIRECTION.lock`** → zero lost body writes ("gsd-mm's own DIRECTION writes take the shared flock").

## Common Pitfalls

### Pitfall 1: In-sandbox panic makes the dispatch gate fail open
**What goes wrong:** `gsd-meta-manager <anything>` panics with rc 101 when the data dir is unwritable. That is the case inside bwrap with `--ro-bind / /` (the spike's profile [VERIFIED: launcher.py:282]).
**Evidence** [VERIFIED: empirical]:
```
$ bwrap --ro-bind / / --dev /dev --proc /proc --tmpfs /tmp target/release/gsd-meta-manager --version
thread 'main' … panicked at …/tracing-appender-0.2.5/src/rolling.rs:156:14:
initializing rolling file appender failed: InitError { context: "failed to create log file", source: Os { code: 30, kind: ReadOnlyFilesystem, … } }
rc=101
$ XDG_DATA_HOME=/proc/nonexistent-gsdmm target/release/gsd-meta-manager --version
… InitError { context: "failed to create log directory", … NotFound … }   rc=101
```
**Why it matters:** "Any other non-zero exit = non-blocking error … The action goes ahead", and a `PreToolUse` gate that crashes lets the Agent call through [CITED: code.claude.com/docs/en/hooks].
**Avoid:** Pattern 1, plus `onFailure: "block"` in the Phase 28 seeding.

### Pitfall 2: Probe/acquire race and reader false-crash
See Answer A (R1–R6).

### Pitfall 3: Socket theft by a second serve or a cross-project run-id collision
See Answer A, points 5 and 6.

### Pitfall 4: Inode-only displaced check
See Pattern 3.

### Pitfall 5: The race-test threshold is statistically flaky
See the race-test section.

### Pitfall 6: Coalescing hides the last heartbeat
**What goes wrong:** a heartbeat regen happens at t=0; heartbeats at t=1..119 are coalesced away; then silence until t=400. At t=301 the frontmatter shows heartbeat age 301 > `heartbeat_stale_after_s: 300` and flags the run stale, although the real last heartbeat (t=119) is only 182 s old.
**Avoid:** a trailing-edge flush on the 2 s ticker: if a heartbeat was suppressed and `now - last_regen >= 120`, regen once. That still respects "at most one per 120 s" `[inferred — audit]`.

### Pitfall 7: State.json forward compatibility
Phase 28 adds `auth`, `cost`, `active_seconds` and `ended_by_driver` [VERIFIED: results/r261a/state.json keys]. Load with `#[serde(default)]` and a `#[serde(flatten)] extra: Map<String, Value>`, so a Phase 27 store never drops launcher fields on rewrite.

### Pitfall 8: Run-dir path escape (STATE.md WR-02 carry-forward)
`--run-dir` / run id must be validated: one path component, `^[A-Za-z0-9][A-Za-z0-9._-]{0,79}$`, canonicalised under `<project>/.planning/meta-manager/drivers/` `[inferred — audit]`. The run id also feeds the socket name and the scope unit name.

### Pitfall 9: Watcher churn
Every heartbeat rewrites state.json, and every event writes temp files in the run dir. The TUI's watcher (Phase 30) should debounce or ignore `state.json` and `.DIRECTION.md.fm.*`. Note it for Phase 30.

## Code Examples

### clap surface (hidden, ungated; mirrors argparse)
```rust
// Source: pattern of src/cli.rs `Drive`/`Envelope` (hide = true, ungated) + driverctl.py:542-551
#[command(hide = true)]
Driverctl { #[command(subcommand)] action: DriverctlAction },

#[derive(Subcommand)]
pub enum DriverctlAction {
    Log {
        event: String,
        #[arg(long)] item: Option<String>,   #[arg(long)] step: Option<String>,
        #[arg(long)] round: Option<String>,  #[arg(long)] phase: Option<String>,
        #[arg(long)] status: Option<String>, #[arg(long = "done-when")] done_when: Option<String>,
        #[arg(long)] evidence: Option<String>, #[arg(long)] state: Option<String>,
        #[arg(long)] rule: Option<String>,   #[arg(long)] outcome: Option<String>,
        #[arg(long)] question: Option<String>, #[arg(long)] action: Option<String>,
        #[arg(long)] reason: Option<String>, #[arg(long)] found: Option<String>,
        #[arg(long)] mode: Option<String>,   #[arg(long = "loop-mode")] loop_mode: Option<String>,
        #[arg(long)] session: Option<String>,
    },
    Show,
    Hook,
    Serve { #[arg(long)] run_dir: PathBuf, #[arg(long)] project: PathBuf, #[arg(long)] sock: Option<PathBuf> },
}
// args map keys sent on the wire = the field names above (underscored), values as strings.
```

### Probe and bounded acquire (extends src/driver/lock.rs idiom)
```rust
// Source: rustix FlockOperation (types.rs:580-586); house facts in src/driver/lock.rs:13-39
pub enum LockProbe { Held, Free, Absent, Unknown(rustix::io::Errno) }

pub fn probe_at(path: &Path) -> LockProbe {
    let file = match File::open(path) {                       // O_RDONLY, never creates
        Ok(f) => f,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return LockProbe::Absent,
        Err(e) => return LockProbe::Unknown(rustix::io::Errno::from_io_error(&e)
                                             .unwrap_or(rustix::io::Errno::IO)),
    };
    match flock(&file, FlockOperation::NonBlockingLockShared) {
        Ok(()) => LockProbe::Free,                           // dropping `file` releases it
        Err(e) if e == rustix::io::Errno::WOULDBLOCK => LockProbe::Held,
        Err(e) => LockProbe::Unknown(e),
    }
}
// acquire_at_with_retry: loop { NonBlockingLockExclusive; on WOULDBLOCK sleep(min(d*2, 50ms)) until
// 500 ms elapsed; then read holder → HeldBy/HeldByUnknownRun }. Never truncate on open.
```

### Exchange step
```rust
// Source: rustix-1.1.5/src/fs/at.rs:302 renameat_with; driverctl.py:174-186
use rustix::fs::{renameat_with, RenameFlags};
renameat_with(&dir_fd, tmp_name, &dir_fd, "DIRECTION.md", RenameFlags::EXCHANGE)?;
// tmp_name now names the displaced inode: read it, compare BODY bytes with the body we built on.
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Hash-distinct gap count (spike 002) | 45 s mtime-coalesced count (gapcount.py) | 26.1, 2026-10-17 | Phase 27 adopts it (Answer B) |
| `crashed` for every non-`RUN end` exit | `classify_exit` → `stalled` vs `crashed` | 26.1 | A new enum variant |
| serde_yml (libyml) | serde_yml 0.0.13 = noyalib shim, still flagged | 2025-09 | Parse only. Emit by hand |
| PreToolUse hook failures fail open | `onFailure: "block"` available (CC ≥ 2.1.295) | 2026 | The Phase 28 seeding should set it |

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | 500 ms acquirer retry budget is enough (63 ms worst measured under a tight loop) | Answer A | Rare false refusal on a heavily loaded host. Tunable constant |
| A2 | Run lock path `<run-dir>/launcher.lock` (per-run, not per-project) | Answer A | Phase 28 could pick per-project. The reader takes a path parameter, so only a constant moves |
| A3 | `paused` removed from the crash rule (corrects D-07) | Answer A R2 | If wrong, a cleanly paused run shows as crashed in Phase 30 |
| A4 | Additive `item_states` frontmatter key for Phase 30 | Schema | Costly to reverse once run dirs exist. Needs operator or Phase 30 confirmation |
| A5 | Race pass criterion `poisson_q99` | Race test | Interprets SC2's wording |
| A6 | Transport-error rc = 1, REJECTED = 2 | Protocol | The MASTER-PROMPT reacts only to REJECTED |
| A7 | Validation tightenings (`RUN` item, rule, phase/loop_mode/session, control chars) | Protocol | Could reject an exotic but valid driver call. None found in recorded LOGs |
| A8 | Gate `ok:false` → deny (deviation from Python) | Pattern 5 | Fails closed on server bugs, so a server bug blocks dispatching |
| A9 | Socket name gets a run-dir hash suffix; no `/tmp` fallback | Answer A | The launcher must pass the same path in `DRIVERCTL_SOCK`. Phase 28 uses the helper |
| A10 | fsync tmp before exchange; CRLF tolerance; trailing heartbeat flush | Patterns/Pitfalls | Minor perf cost |
| A11 | `dirs::data_local_dir` honors `XDG_DATA_HOME` (the reproduction relied on it and did panic, so it holds on this box) | Pitfall 1 | None for the fix |

## Open Questions (RESOLVED)

1. **RESOLVED (D-19, operator 2026-10-18; 27-03).** **Per-item state in the frontmatter (A4).** Phase 30 SC1 needs it from the frontmatter; the spike has counts only. Recommendation: add `item_states` now (additive, `serde(default)`).
2. **RESOLVED (D-20; 27-01).** **Run lock scope (A2).** Per-run vs per-project. Recommendation: per-run `launcher.lock`, which is spike-proven (L5). Phase 28 confirms.
3. **RESOLVED (27-06 Task 1).** **Standalone `serve` launch bookkeeping.** Recommendation: `serve` calls the same `begin_launch` the launcher will (lock → crash-detected → `running`, `launches+1`, a `launcher` connection with `mode: "standalone"`). On SIGTERM/SIGINT it records `stopped` with reason `serve:signal` unless the driver logged `RUN end`. Phase 28 replaces this with `classify_exit` `[inferred — audit]`.
4. **RESOLVED (D-25 Phase 28 hand-off; 27-06 Task 3).** **`fresh-exit` → `running` (R3).** Phase 28 must hold the lock across relaunches, or the reader false-crashes. Record it in the Phase 28 hand-off.
5. **RESOLVED (D-27; todo filed).** **Dashboard quote divergence on `status: "gaps_found"`** (Answer B, item 4). Out of scope. File as a todo.

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| rustc / cargo | build | ✓ | 1.99.1 (MSRV floor 1.89) | — |
| Linux `renameat2(RENAME_EXCHANGE)` | splice | ✓ | kernel 7.0.0-38. Verified on zfs (project), tmpfs (scratch, `$XDG_RUNTIME_DIR`) | typed error on unsupported fs (EINVAL) — never fall back to naive |
| `XDG_RUNTIME_DIR` | socket dir | ✓ | `/run/user/1000` (tmpfs) | refuse with a typed error |
| git | `base=` sha | ✓ | 2.53.0 | `base` = null |
| python3 | spike tools only (not needed by tests) | ✓ | 3.14.4 (note: default mp start is forkserver; spike scripts may need `fork`) | — |
| bwrap | optional manual sandbox check | ✓ | 0.11.1 | CI-safe `XDG_DATA_HOME` reproduction instead |
| systemd-run | Phase 28 only | ✓ | — | — |

**Missing dependencies with no fallback:** none.

## Validation Architecture

> `workflow.nyquist_validation` is `false` in `.planning/config.json`. This section is included at the orchestrator's explicit request.

### Test Framework
| Property | Value |
|----------|-------|
| Framework | built-in `cargo test` (integration tests in `tests/`, unit tests in-module) |
| Config file | none |
| Quick run command | `rtk proxy cargo test --no-fail-fast driverctl` |
| Full suite command | `rtk proxy cargo test --no-fail-fast` (+ `-- --ignored driverctl_race_full` for the 15 s race) |

### Phase Requirements → Test Map
| Req ID | Behavior | Test Type | Automated Command | File Exists? |
|--------|----------|-----------|-------------------|-------------|
| DCTL-01 | Every MASTER-PROMPT `driverctl` line parses and maps to the spike request JSON | unit | `cargo test --test driverctl_prompt_parse` | ❌ Wave 0 |
| DCTL-01 | LOG replay (r261a, run1, R2) through client → serve reproduces LOG modulo ts/base | integration | `cargo test --test driverctl_replay` | ❌ |
| DCTL-01 | In-sandbox-shaped invocation never panics; gate denies on transport error | integration | `cargo test --test driverctl_client_failclosed` | ❌ |
| DCTL-01 | Second serve refused; socket not stolen; SIGKILL → crashed; restart → crash-detected | integration | `cargo test --test driverctl_lock_race` | ❌ |
| DCTL-02 | Splice preserves body bytes; insane body skipped; CRLF tolerated; mode kept | unit | `cargo test driverctl::splice` | ❌ |
| DCTL-02 | Race smoke (0 lost rename) / full (`#[ignore]`) with naive positive control | integration | `cargo test --test driverctl_race` | ❌ |
| DCTL-02 | Two writers under DIRECTION.lock → 0 lost body writes | integration | same file | ❌ |
| DCTL-03 | Golden round-trip: template, starting, running, stopped, crashed, stalled, multi-connection | unit | `cargo test state_reader::driver_frontmatter` | ❌ |
| DCTL-03 | 17 Python-generated DIRECTION.md fixtures parse | unit | same | ❌ |
| DCTL-03 | Probe/acquire race closed (tests 1–5 of Answer A); reader table R1–R6 | integration/unit | `cargo test --test driverctl_lock_race` | ❌ |
| DCTL-04 | gapcount selftest table + Run 1 replay + r261a shape | unit | `cargo test driverctl::counters` | ❌ |
| DCTL-04 | outstanding_dispatch WR-01 (a)–(f); classify_exit matrix; run1b → stalled | unit | same | ❌ |
| DCTL-04 | rc=2 for every malformed/unknown/enum-violating request; nothing written | integration | `cargo test --test driverctl_protocol` | ❌ |
| DCTL-04 | Hook dispatch counting (no agent_id counts; nested does not); heartbeat ≤1/120 s + trailing flush (injected clock) | unit | `cargo test driverctl::store` | ❌ |

### Sampling Rate
- **Per task commit:** `rtk proxy cargo test --no-fail-fast driverctl`
- **Per wave merge:** `rtk proxy cargo test --no-fail-fast`
- **Phase gate:** full suite green, plus one `--ignored` full race run recorded in VERIFICATION

### Wave 0 Gaps
- [ ] `tests/fixtures/driverctl/` — copies of the spike-002 R1/R2/L1 and spike-003 run1/run1b/r261a/l7… DIRECTION.md, LOG.md, state.json, launcher-result-1.json, plus MASTER-PROMPT.md
- [ ] An injectable clock on `RunStore` (heartbeat/coalesce tests must not sleep 120 s)
- [ ] `poisson_q99` helper (test-only)

## Security Domain

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no (same-uid unix socket) | 0600 socket in a 0700, uid-owned dir |
| V3 Session Management | no | — |
| V4 Access Control | yes | Run dir read-only in the sandbox (Phase 28). The terminal setter is not socket-reachable. Lock files are masked in the sandbox |
| V5 Input Validation | yes | serde typed requests, `one_line` on every LOG header value, enum checks, request-size cap, chars-based prompt truncation |
| V6 Cryptography | no (sha256 is a dedup digest, not security) | sha2 |

### Known Threat Patterns
| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| LOG grammar injection via `\|`/newline in unvalidated fields | Tampering | Validate all header values (Anti-Patterns) |
| Panicking/timeouting gate hook fails open | Elevation | Dispatch before tracing init; deny on `ok:false`; `onFailure: block` (Phase 28) |
| Sandbox holds `DIRECTION.lock` / stalls a connection | DoS | Bounded lock retry + exchange-verify fallback; `/dev/null` mask; thread per connection with a timeout |
| Second serve / socket theft / cross-project socket collision | Spoofing/Tampering | Run lock before unlink; run-dir hash in the socket name |
| Run id path escape (WR-02) | Tampering | Component regex + canonical-under-drivers check |
| Host-side git in a sandbox-writable repo | Elevation | `rev-parse` only (reads no index, runs no hooks [VERIFIED: git_ops.rs:266-267 doc]), budgeted. Never run index-touching git from the store |
| Pre-created `/tmp/gsdmm-driverctl` by another user | Spoofing | No `/tmp` fallback; verify dir owner and mode |

## Sources

### Primary (HIGH confidence)
- Read this session: `.planning/spikes/002-…/tools/driverctl.py` (all), `tools/launcher.py` (all), `experiment/race_test.py`, `experiment/results/race-test.txt`, `FRONTMATTER-SCHEMA.md`, `README.md` §A1/B/Results, `templates/DIRECTION.md`, `templates/LOG.md`, `MASTER-PROMPT.md` (driverctl lines)
- `.planning/spikes/003-…/tools/gapcount.py` (all), `exitclass.py` (all), `launcher_s003.py` (:135-215, :395-440), `dup_check.py:40-62`, `results/r261a/state.json`, `results/run1/LOG.md`, MANIFEST.md, CONVENTIONS.md
- `src/driver/lock.rs` (all), `src/driver/liveness.rs:1-98`, `src/driver/mod.rs:1-80`, `src/cli.rs`, `src/main.rs:1-82, 280-420`, `src/state_reader/state_md.rs:300-380`, `src/state_reader/disk_status.rs:40-130, 560-640, 745-854`, `src/state_reader/git_ops.rs:100-270`, `tests/async_blocking_guard.rs` header, `Cargo.toml`, `Cargo.lock`
- Crate sources: rustix-1.1.5 (`fs/at.rs`, `backend/linux_raw/fs/types.rs`), tracing-appender-0.2.5 (`rolling.rs`), clap_builder-4.6.7
- Empirical: `scratchpad/flock_probe.py` (E1–E6), the bwrap lock-mask test, the RENAME_EXCHANGE probe on zfs/tmpfs, the binary panic reproductions

### Secondary (MEDIUM confidence)
- [Claude Code hooks reference](https://code.claude.com/docs/en/hooks): exit-code semantics, deny JSON, `onFailure: "block"`, timeouts
- [RUSTSEC-2025-0068](https://rustsec.org/advisories/RUSTSEC-2025-0068.html), [serde_yml repo (0.0.13 shim)](https://github.com/sebastienrousseau/serde_yml)

### Tertiary (LOW confidence)
- None relied upon.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH. No new deps; APIs read from the locked crate sources.
- Architecture / Answer A: HIGH. Measured on this kernel; ordering read from the spike code.
- Answer B: HIGH. ROADMAP, REQUIREMENTS, MANIFEST and the spike code read verbatim.
- Pitfalls: HIGH for 1–4 and 6 (reproduced or read). MEDIUM for 5 (statistical interpretation).

**Research date:** 2026-10-18
**Valid until:** 2026-11-17 (stable internals; recheck the Claude Code hook semantics if the CC version changes)
