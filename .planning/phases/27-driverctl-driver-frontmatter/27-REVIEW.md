---
phase: 27-driverctl-driver-frontmatter
reviewed: 2026-10-19T00:00:00Z
depth: standard
files_reviewed: 28
files_reviewed_list:
  - src/cli.rs
  - src/driverctl_cli.rs
  - src/driverctl/counters.rs
  - src/driverctl/log.rs
  - src/driverctl/mod.rs
  - src/driverctl/protocol.rs
  - src/driverctl/scan.rs
  - src/driverctl/serve.rs
  - src/driverctl/store.rs
  - src/driver/direction.rs
  - src/driver/lock.rs
  - src/driver/mod.rs
  - src/lib.rs
  - src/main.rs
  - src/state_reader/driver_frontmatter.rs
  - src/state_reader/mod.rs
  - tests/driverctl_client.rs
  - tests/driverctl_frontmatter.rs
  - tests/driverctl_liveness.rs
  - tests/driverctl_lock_race.rs
  - tests/driverctl_race.rs
  - tests/driverctl_replay.rs
  - tests/driverctl_serve.rs
  - tests/driverctl_store.rs
  - tests/fixtures/driverctl/MASTER-PROMPT.md
  - tests/fixtures/driverctl/replay/s002-R2/LOG.md
  - tests/fixtures/driverctl/spike003/r261a/state.json
  - tests/fixtures/driverctl/spike003/run1/LOG.md
findings:
  critical: 1
  warning: 9
  info: 3
  total: 13
status: issues_found
---

# Phase 27: Code Review Report

**Reviewed:** 2026-10-19
**Depth:** standard
**Files Reviewed:** 28
**Status:** issues_found

## Summary

Phase 27 adds the `driverctl` client, the `serve` socket server, the `RunStore` single writer, the LOG/counter/protocol modules, the DIRECTION.md exchange-splice and the frontmatter schema. The core is carefully built: validation precedes mutation, the run lock is in the type, the frontmatter emitter escapes every control character, and the client gate fails closed. Pre-existing files (`cli.rs`, `main.rs`, `lib.rs`, `driver/lock.rs`, `driver/mod.rs`, `state_reader/mod.rs`) were reviewed only for the phase-27 diff and look sound. `main`'s early synchronous dispatch and the fallible `init_tracing` are correct.

The code has one real defect against its own hostile-driver threat model. The gap-round scan reads project files the sandboxed driver can plant, with no file-type or size guard, while holding the one store mutex. Beyond that there are several robustness gaps where the code or docs claim more than the implementation delivers. These are the per-read-only connection timeout, the stale-socket "never steals" claim, header-value hardening and the fresh-exit terminal case. Fixtures were treated as data and not reviewed for defects. No structural (fallow) findings were supplied.

## Critical Issues

### CR-01: Gap scan reads driver-plantable files with no type or size guard, under the store mutex

**File:** `src/driverctl/scan.rs:76-79` (called from `src/driverctl/serve.rs:547-548` and `src/driverctl/store.rs:975-983`)
**Issue:** `scan_verifications` runs `fs::metadata(&path)` then `fs::read(&path)` on every `*-VERIFICATION.md` under `<project>/.planning/phases/*/`. That tree lives in the project root, which is writable by the sandboxed driver (it writes these very files). There is no `is_file()` check, no `symlink_metadata`, no `O_NONBLOCK`/`O_NOFOLLOW` and no size cap. A hostile or prompt-injected driver can:
- `mkfifo .planning/phases/01-x/01-VERIFICATION.md`. The launcher's `fs::read` then blocks forever in `open(2)` with no writer.
- or `ln -s /dev/zero .planning/phases/01-x/01-VERIFICATION.md`. `metadata` succeeds and `fs::read` grows without bound (OOM-kill of the launcher/serve).

The ticker takes the store mutex before calling `scan_tick` (`serve.rs:547`), so a blocked read wedges every `handle_json` call. No log, show or hook request is ever answered, and every gate hook times out client-side and is denied. This defeats T-27-24 ("a stalled connection never blocks a gate hook") and the single-writer liveness the whole phase relies on. The thread also cannot be interrupted by SIGTERM: shutdown does `ticker_handle.join()` before anything else (`serve.rs:641`), so a ticker stuck in `open(2)` blocks graceful shutdown for good.
**Fix:**
```rust
// scan.rs: only regular, non-symlink files, bounded read, never block on open.
use std::io::Read;
use std::os::unix::fs::OpenOptionsExt;

const MAX_VERIFICATION_BYTES: u64 = 1 << 20;

let Ok(meta) = fs::symlink_metadata(&path) else { continue };
if !meta.file_type().is_file() || meta.len() > MAX_VERIFICATION_BYTES { continue; }
let Ok(mut f) = fs::OpenOptions::new()
    .read(true)
    .custom_flags(rustix::fs::OFlags::NONBLOCK.bits() as i32 | rustix::fs::OFlags::NOFOLLOW.bits() as i32)
    .open(&path) else { continue };
let mut raw = Vec::new();
if f.take(MAX_VERIFICATION_BYTES).read_to_end(&mut raw).is_err() { continue; }
```
Also do the filesystem scan outside the mutex. Have the ticker call `scan_verifications(&project)` first, then take the lock only to feed `observe_verification`. A slow scan must never hold the writer. Apply the same `symlink_metadata`/`is_dir` discipline to `visible_children` for symlinked phase directories. Add a test that plants a FIFO and asserts that `scan_verifications` returns promptly.

## Warnings

### WR-01: Connection timeout is per-read, so trickling clients hold all 32 slots indefinitely

**File:** `src/driverctl/serve.rs:423-462, 471-482, 509`
**Issue:** `CONN_TIMEOUT` is a per-`read` socket timeout. A client that sends one byte every ~9 s is never timed out. It can keep a handler thread for up to `MAX_REQUEST_BYTES * 9 s`. `MAX_CONNECTIONS = 32` such connections make every other request, including every gate hook, answer `busy`, which the client turns into a deny. The test `a_flood_of_idle_connections_is_answered_busy_and_the_gate_denies` shows 32 idle connections already lock everyone out for the timeout, and trickling extends that without bound. The client side was given a total deadline for exactly this reason (`driverctl_cli.rs:50-55`, T-27-14) and the server side was not. The sandboxed driver owns a connectable 0600 socket, so this is reachable from the stated adversary.
**Fix:** Track a total deadline in `read_request` (for example `Instant::now() + CONN_TIMEOUT`) and, before each `read`, call `set_read_timeout(Some(deadline.saturating_duration_since(now)))`. Return `TimedOut` once the deadline passes.

### WR-02: `end --reason fresh-exit` followed by shutdown leaves status `running`, so the reader derives a false `crashed`

**File:** `src/driverctl/store.rs:1002-1004` (with `1174-1179` and `src/driverctl/serve.rs:646`)
**Issue:** `fresh-exit` records `RunStatus::Running` and sets `ended_by_driver = true`. `set_terminal` returns `KeptDriverEnd` whenever `ended_by_driver` is set, even when the kept status is not terminal. For `driverctl serve` there is no relaunch after a fresh-exit, so SIGTERM leaves `status: running` with the lock released, and `derive_status` reports `Crashed` for a run that was stopped cleanly. No test covers set_terminal after `fresh-exit`; the only keep-driver-end test uses `all-done`.
**Fix:**
```rust
if self.state.ended_by_driver && self.state.status.is_terminal() {
    return Ok(TerminalOutcome::KeptDriverEnd);
}
```
Add a store test for `fresh-exit` followed by `set_terminal(Stopped, ..)`.

### WR-03: LOG.md line is appended before `state.json` is durable, so a failed save leaves them diverged

**File:** `src/driverctl/store.rs:507-523`
**Issue:** `commit` appends to LOG.md (508-513), then calls `write_state` (522). If `write_state` fails (ENOSPC, EIO) the LOG already carries the event but `self.state` is unchanged and the caller gets `internal: ...`. The module doc promises that an I/O failure "leaves the in-memory state as it was and writes nothing further". A driver that retries a rejected `dispatched` writes a second `dispatched` line while `dispatches` counts it once. `outstanding_dispatch` (counters.rs) and the crash/stall classification read LOG.md as the audit source, so the two records then disagree.
**Fix:** Write `state.json` first and append the LOG only on success, or document and test the divergence. At minimum, correct the doc claim and make the LOG append failure the only abort point before the state write.

### WR-04: Early returns after `begin_launch` leave status `running` and a stale socket file

**File:** `src/driverctl/serve.rs:604-632`
**Issue:** `begin_launch` writes `running` (605). The bind-failure path correctly records `serve:bind-failed`. The ticker spawn failure path (`.map_err(...)?` at 631) returns without `set_terminal`, without unlinking the freshly bound socket (path remains after `listener` drops), and without stopping accept. The lock is released as the store drops, so readers derive `crashed` for a run that never served. The same applies if a later `?` is added.
**Fix:** Wrap the post-bind setup in a closure or guard, so any error calls `set_terminal(Stopped, "serve:startup-failed")` and `fs::remove_file(&sock)` before returning.

### WR-05: Stale-socket replacement never checks that the socket is actually stale

**File:** `src/driverctl/serve.rs:314-336, 602, 615`
**Issue:** `existing_socket` returns `true` for any socket file. `replace_and_bind` then unlinks it. Holding this run's lock does not make the socket this run's. An explicit `--sock` pointing at another live run's socket (or a default-path 32-bit hash collision) unlinks and rebinds a live server's socket, orphaning that server. `driverctl/mod.rs` fact 9 and serve.rs fact 3 claim it "never steals a live run's socket"; the tests only cover the same-run case, which the lock already blocks.
**Fix:** Before unlinking, probe with `UnixStream::connect(sock)`. Treat success as `SocketPath { "socket is live" }` and only `ECONNREFUSED` as stale. Add a test with two runs and a shared `--sock`.

### WR-06: Driver-supplied header values may carry control characters other than CR/LF

**File:** `src/driverctl/log.rs:154-165` (used by `src/driverctl/protocol.rs:379-386, 479-557`)
**Issue:** `one_line` rejects `\n`, `\r` and `|` only. `step`, `phase`, `evidence`, `outcome`, `question`, `reason`, `session` and `loop-mode` come from the sandboxed LLM and are written verbatim to LOG.md. They can carry ESC (terminal escape injection when a human `cat`s or `less`es the log, or when a Phase 30 view renders the line), NUL, VT/FF/FS-RS and U+0085/U+2028/U+2029 (line breaks to Python's `splitlines`, which the spike tooling uses). The module's own tightening list adds CR and rejects control characters in report bodies, and the launcher-side `header_safe` strips all of them, so the driver side is the inconsistent one.
**Fix:** In `one_line`, reject `c.is_control() || c == '\u{2028}' || c == '\u{2029}'` (tab optional) in addition to `|`. Add a table test for ESC, NUL, VT, U+0085 and U+2028.

### WR-07: Gap-round `seen` is keyed by digest alone, so identical files in different phases collapse

**File:** `src/driverctl/counters.rs:80-83`
**Issue:** `observe_gap` returns `Duplicate` if `state.seen` contains the digest, regardless of phase. Two phases whose `gaps_found` VERIFICATION.md bytes are identical (a stub, a templated minimal file) hash identically. The second phase's round is never counted, and the cap the launcher enforces under-counts. The same happens when a file is reverted to earlier bytes (v1, v2, v1). The `scan.rs` test only avoids this by putting the same bytes in dirs that are never observed together.
**Fix:** Key `seen` by `format!("{phase}:{digest}")` (or keep the digest key and compare `seen[digest].phase` to the current phase), keeping the `state.json` shape tolerant of the spike layout.

### WR-08: Client `connect` has no timeout, contradicting the documented total deadline

**File:** `src/driverctl_cli.rs:50-55, 166-175`
**Issue:** `CLIENT_TIMEOUT` is documented as the total for "a connect, a write or the wait for a reply", but `UnixStream::connect(&path)` is a blocking call with no timeout. When a listener exists but its owner is stopped or wedged (SIGSTOP, the CR-01 hang), the backlog fills and `connect(2)` blocks indefinitely. A gate hook then hangs until Claude Code kills it, which is only safe if the hook seed carries `onFailure: block` (the Phase 28 hand-off).
**Fix:** Connect on a helper thread with `mpsc::recv_timeout(CLIENT_TIMEOUT)`, or use a non-blocking `connect` via `rustix` and poll for writability. At minimum, correct the doc to say connect is unbounded.

### WR-09: Wall-clock assertions in default-run tests are flake-prone

**File:** `tests/driverctl_lock_race.rs:181-188`, `tests/driverctl_race.rs:349-352`, `tests/driverctl_serve.rs:681-685`
**Issue:** These tests assert absolute timings on a shared machine. `real_holder_is_still_refused_within_budget` requires `elapsed < 500 ms + 100 ms`; any scheduler pause after the final retry sleep fails it. `editor_race_smoke` bounds in-place losses by a Poisson 99th percentile with an expectation near 0.07 (so exactly one loss is tolerated, two fail) on a 1 s run under CPU contention. `stalled_connection_does_not_block_the_gate` asserts a 2 s ceiling. All run in the default suite, and the working tree is shared with other agents.
**Fix:** Widen slack (for example `+ 500 ms`), assert only the lower bound plus "did not hang", or mark the in-place statistical bound `#[ignore]` like `editor_race_full` and keep only the rename-save `lost == 0` check in the smoke test.

## Info

### IN-01: Orphaned `state.json` temp files are never swept

**File:** `src/driverctl/store.rs:480` (and `src/driver/direction.rs:354-372`)
**Issue:** `write_state` creates `NamedTempFile` with a random `.tmpXXXX` name in the run dir. A SIGKILL between create and persist leaves it behind. `sweep_stale_temps` only matches the `.DIRECTION.md.fm.` prefix. Orphans accumulate, one per crash.
**Fix:** Give the state temp a distinct prefix via `tempfile::Builder::new().prefix(".state.json.tmp.")` and sweep that prefix at startup too.

### IN-02: A panic in the ticker thread is silent and permanent

**File:** `src/driverctl/serve.rs:538-553, 641`
**Issue:** If `scan_tick` or `flush_heartbeat` panics, the ticker thread dies, no gap rounds are counted and no heartbeat flush occurs for the rest of the run. `ticker_handle.join()` discards the result, so nothing is logged. The connection path wraps panics in `catch_unwind`; the ticker does not.
**Fix:** Wrap the tick body in `catch_unwind` and log the failure to stderr, or check the `join()` result and report it.

### IN-03: Replay test silently accepts a changed dispatch budget

**File:** `tests/driverctl_replay.rs:289-311, 338-341`
**Issue:** When the recorded `dispatch=N/B` budget differs from the fixture body, the test only prints `REPLAY-NOTE` and compares the regenerated budget to the body. SC1 asserts the same events with the same values, so a budget regression that moves the value together with the body would pass. The counter `N` is still compared exactly, which keeps the main property.
**Fix:** Keep an explicit allow-list of known budget edits (run id, event index, expected values) and fail on any difference outside it.

---

_Reviewed: 2026-10-19_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
