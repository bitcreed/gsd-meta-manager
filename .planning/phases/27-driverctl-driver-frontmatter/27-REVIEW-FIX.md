---
phase: 27-driverctl-driver-frontmatter
fixed_at: 2026-10-19T00:00:00Z
review_path: .planning/phases/27-driverctl-driver-frontmatter/27-REVIEW.md
iteration: 1
findings_in_scope: 11
fixed: 11
skipped: 0
status: all_fixed
---

# Phase 27: Code Review Fix Report

**Fixed at:** 2026-10-19
**Source review:** .planning/phases/27-driverctl-driver-frontmatter/27-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 11 (CR-01, WR-01 to WR-09, plus SEC-01 from the orchestrator)
- Fixed: 11
- Skipped: 0
- Out of scope (fix_scope `critical_warning`): IN-01, IN-02, IN-03

**Verification environment:** worked directly in the main checkout on `master` (no worktree, per the orchestrator's instruction; `workflow.use_worktrees` is true in config but was overridden by the prompt). `cargo test --no-fail-fast` and `cargo clippy --all-targets -- -D warnings` ran in the main checkout. Results: 3209 passed, 1 failed, 18 ignored. The one failure is the expected local-only `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`. Clippy is clean. Output was captured to a scratch file, not read through rtk-filtered pipes.

## Fixed Issues

### CR-01: Gap scan reads driver-plantable files with no type or size guard, under the store mutex

**Files modified:** `src/driverctl/scan.rs`, `src/driverctl/serve.rs`, `src/driverctl/store.rs`, `tests/driverctl_store.rs`
**Commit:** ca2473b
**Applied fix:** `scan_verifications` now goes through `read_regular_bounded`. It runs `lstat`, requires a regular non-symlink file of at most 1 MiB, and opens with `O_NONBLOCK | O_NOFOLLOW`. It re-checks `fstat` on the open handle and reads through `take(MAX+1)`, skipping anything that outgrew the cap. A symlinked phase directory is no longer followed (`symlink_metadata`). The serve ticker scans before taking the store mutex and calls the new `RunStore::observe_verifications` under the lock; `scan_tick` is kept and composes the two. Tests: FIFO, symlinked file, symlinked phase dir, over/at-cap sizes (unit), and a FIFO through `scan_tick` (integration). The two FIFO tests run on a thread with a 30 s join bound so a regression is a failure, not a hung suite.
**Notes:** the "scan outside the mutex" half has no failing-without-the-fix test, because it needs a deterministically slow read. It is covered by construction (the ticker calls `scan_verifications` before `lock_store`).

### WR-01: Connection timeout is per-read

**Files modified:** `src/driverctl/serve.rs`
**Commit:** 630b149
**Applied fix:** `read_request(stream, total)` computes a deadline once and sets the read timeout to the time remaining before each read. Test: a socketpair client trickles a byte every 20 ms with a 200 ms deadline. The per-read behaviour would return `Ok` after the writer closes; only a total deadline yields `TimedOut`. The assertion is on the variant, not on elapsed time.

### WR-02: `end --reason fresh-exit` then shutdown leaves status `running`

**Files modified:** `src/driverctl/store.rs`, `tests/driverctl_store.rs`
**Commit:** a76e1ae
**Applied fix:** `set_terminal` returns `KeptDriverEnd` only when `ended_by_driver && status.is_terminal()`. Test: `fresh-exit` followed by `set_terminal(Stopped)` records `stopped`.
**Status:** fixed: requires human verification (logic change; see the Phase 28 note below).

### WR-03: LOG line appended before `state.json` is durable

**Files modified:** `src/driverctl/store.rs`, `tests/driverctl_store.rs`
**Commit:** 6e0af6d
**Applied fix:** `commit` writes `state.json` first, then appends the LOG. If the append fails, the previous `state.json` is written back best-effort, so the module-doc promise holds in both directions. Tests: a directory in place of `state.json` leaves no new LOG line, and a retry logs exactly one (this one fails without the fix). A directory in place of `LOG.md` leaves `state.json` unchanged (guard for the rollback path; it passes on the old code too, since the old order never wrote state first).
**Status:** fixed: requires human verification (ordering choice [inferred]: a missing LOG line was judged less harmful than a duplicate `dispatched` line, since the LOG is the audit source).

### WR-04: Early return after `begin_launch` leaves `running` and a stale socket

**Files modified:** `src/driverctl/serve.rs`
**Commit:** 0ab97c8
**Applied fix:** New `abort_startup(store, sock, reason)` records `Stopped` with reason `serve:startup-failed` and unlinks the socket. It is called when the ticker thread cannot spawn, after the listener is dropped. Test: `abort_startup` on a real `RunStore` sets `stopped` and removes the socket. The spawn failure itself cannot be injected, so the test exercises the extracted helper.

### WR-05: Stale-socket replacement never checks the socket is stale

**Files modified:** `src/driverctl/serve.rs`, `tests/driverctl_serve.rs`
**Commit:** 14bdca5
**Applied fix:** `replace_and_bind` probes with `connect` before unlinking (`socket_is_live`). Only `ECONNREFUSED` or `ENOENT` proves the file stale. A successful connect, a probe that does not finish in 2 s, and any other error all count as live and produce a typed `SocketPath` "live socket" refusal. The probe runs on a helper thread so a full backlog cannot block startup. Tests: unit (live is refused and still connectable, dead is replaced) and an end-to-end test in which two different runs share one `--sock` (the second exits 1, the inode is unchanged and the first still answers).
**Status:** fixed: requires human verification (logic change).

### WR-06: Header values may carry control characters other than CR/LF

**Files modified:** `src/driverctl/log.rs`
**Commit:** 7ef6820
**Applied fix:** `one_line` rejects `|`, U+2028, U+2029 and every control character except tab. Tab stays allowed [inferred]. Table test: ESC, NUL, VT, FF, FS, U+0085, U+2028, U+2029 and DEL are rejected; tab and non-ASCII letters are accepted.

### WR-07: Gap-round `seen` is keyed by digest alone

**Files modified:** `src/driverctl/counters.rs`
**Commit:** ce65379
**Applied fix:** `seen` is keyed `"{phase}:{digest}"`. A legacy bare-digest key still counts as seen, but only for the phase it records, so spike-layout `state.json` files load and are not double counted. A first attempt to keep the bare key and compare the phase was rejected: it would ping-pong between two phases and count a new round on every tick. Tests: identical bytes in two phases are two rounds, and re-scanning both in either order is a duplicate; the legacy key is handled.
**Status:** fixed: requires human verification. The reviewer's "revert to earlier bytes (v1, v2, v1)" case is deliberately unchanged: within one phase an identical digest remains a duplicate, as in the spike and as `scan_then_observe_counts_coalesces_and_dedups` asserts.

### WR-08: Client `connect` has no timeout

**Files modified:** `src/driverctl_cli.rs`
**Commit:** b3b7fcd
**Applied fix:** `connect_within(path, limit)` connects on a helper thread and waits with `recv_timeout`. The write timeout now takes the time left in the total deadline. Tests (new `#[cfg(test)]` module in the bin): connecting to a listener whose backlog is full times out (filled by looping `connect_within(300 ms)`, no wall-clock upper-bound assert); live and missing paths behave as before.

### WR-09: Wall-clock assertions in default-run tests

**Files modified:** `tests/driverctl_lock_race.rs`, `tests/driverctl_race.rs`, `tests/driverctl_serve.rs`
**Commit:** d3e7362
**Applied fix:**
- `real_holder_is_still_refused_within_budget`: kept the lower bound, replaced the +100 ms ceiling with +10 s ("did not hang").
- `editor_race_smoke`: `assert_exchange_holds(secs, check_inplace_bound)` applies the Poisson bound only in the `#[ignore]`d `editor_race_full`. The smoke keeps `rename.lost == 0` and the sanity, temp and tool checks.
- `stalled_connection_does_not_block_the_gate`: replaced the 2 s ceiling with a logical check that the stalled peer is still open (`WouldBlock`) when the gate answers.

This is a test-only change, so no new regression test.

### SEC-01 (extra, not in REVIEW.md): `driverctl hook` fails open on non-JSON stdin

**Files modified:** `src/driverctl_cli.rs`, `tests/driverctl_client.rs`
**Commit:** 1791f2f
**Source:** 27-SECURITY.md line 87, which records the exit-0 silent path as an `[inferred — audit]` fail-open note that was not counted.
**Applied fix:** `hook_main` prints the existing deny JSON (`LAUNCHER CAP: hook payload ...`) and exits 0 when stdin is unreadable or not a JSON object. The contract is taken from D-25 and the module doc: "decision is the JSON, the exit code is 0 in every case". `[inferred]`: I chose deny JSON with exit 0 over a non-zero exit, because Claude Code ignores stdout JSON on a non-zero exit, and because the docs say the exit code is always 0. The module doc and function doc were updated. The test `invalid_hook_stdin_is_silent` was renamed `invalid_hook_stdin_fails_closed` and now asserts the deny for `not json at all`, empty, array, string, number and `null`.
**Status:** fixed: requires human verification. The deny is emitted even for non-gate events when stdin is garbage, because without a parsed payload the client cannot tell them apart. Claude Code always sends an object, so this should never trigger in a healthy run.

## Skipped Issues

None. IN-01, IN-02 and IN-03 are outside `fix_scope: critical_warning`.

## Inferred decisions for audit

- WR-03: state-first ordering with rollback (above).
- WR-05: a probe timeout or any non-refused error counts as live (never steal on doubt); 2 s probe timeout.
- WR-06: tab remains allowed.
- WR-07: phase-prefixed key with legacy-key tolerance.
- SEC-01: deny JSON with exit 0, for every unparsable stdin.
- WR-02: a `fresh-exit` followed by `set_terminal(Crashed|Stalled)` now overwrites the non-terminal driver end. Phase 28's `classify_exit` already returns `None` for a driver end, so the launcher should not call it in that case, but Phase 28 should confirm.

---

_Fixed: 2026-10-19_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
