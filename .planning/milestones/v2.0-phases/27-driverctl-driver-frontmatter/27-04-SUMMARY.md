---
phase: 27-driverctl-driver-frontmatter
plan: 04
subsystem: driverctl
tags: [driverctl, clap, unix-socket, hook-gate, fail-closed, liveness, flock]
requires: [27-01, 27-03]
provides:
  - "gsd-meta-manager driverctl log|show|hook: hidden, ungated clap subcommand speaking spike-002 wire protocol over DRIVERCTL_SOCK"
  - "driverctl_cli (binary-crate module): run, request, log/show/hook, fail-closed dispatch gate"
  - "state_reader::driver_frontmatter: DerivedStatus, DriverRunView, derive_status, read_driver_run, FrontmatterError::Io"
affects: [27-05, 27-06, phase-28, phase-29, phase-30]
tech-stack:
  added: []
  patterns:
    - "sync main dispatches the sandbox client before tracing/color-eyre/config/runtime (D-25)"
    - "fallible tracing init: RollingFileAppender::builder, no subscriber on Err"
    - "derived-not-persisted status: the lock probe corrects the written status at read time only"
key-files:
  created:
    - src/driverctl_cli.rs
    - tests/driverctl_client.rs
    - tests/driverctl_liveness.rs
    - tests/fixtures/driverctl/MASTER-PROMPT.md
  modified:
    - src/cli.rs
    - src/main.rs
    - src/state_reader/driver_frontmatter.rs
key-decisions:
  - "Client lives in the binary crate (mod driverctl_cli), not the library"
  - "Gate denies on transport error, unset socket, unreadable reply AND server ok:false (D-25 deviation from the spike)"
  - "derive_status implements the D-22 table; LivenessUnknown is its own answer"
requirements-completed: [DCTL-01, DCTL-03, DCTL-04]
status: complete
duration: ~35min
completed: 2026-10-19
commits: 4
plan_head_before: 8c4961f8d2e2dd2cdcd0bdef89fe3a8e8227be3e
plan_head_after: 7f55926aa9dd84868b21fb472ba93cabfbd2035b
actuals:
  tokens: 21000
  tasks: 3
  commits: 4
---

# Phase 27 Plan 04: driverctl client, fail-closed dispatch gate and reader liveness Summary

The hidden `driverctl log|show|hook` client re-enters the existing binary before any tracing init, sends one spike-wire JSON request per connection, and denies every top-level Agent/Task dispatch it cannot affirmatively clear; `read_driver_run` turns a run dir into a frontmatter plus a lock-derived status that never calls a paused, never-launched or unknown-liveness run crashed.

## What was built

- **`src/cli.rs`**: `Commands::Driverctl { action }` (`hide = true`, no `#[cfg]`), `DriverctlAction { Log(Box<DriverctlLog>), Show, Hook }`, `DriverctlLog` with positional `event` and the 17 long flags (`--done-when`, `--loop-mode` derived). Four parse tests (MASTER-PROMPT shapes, show/hook, unknown flag = usage error rc 2, absent from rendered help).
- **`src/main.rs`**: sync `fn main` parses first, hands `Commands::Driverctl` to `driverctl_cli::run` and `process::exit`s with its code (no tracing, color-eyre, config or runtime). Everything else runs through `init_tracing()` (now fallible via `RollingFileAppender::builder`, installs nothing on `Err`), `color_eyre::install`, an explicit `new_multi_thread().enable_all()` runtime and `async_main` (the old body; its `Driverctl` arm returns an error instead of panicking).
- **`src/driverctl_cli.rs`**: `request` (unix: connect, 15 s TOTAL deadline for the reply, 1 MiB cap, JSON-object check), `log_main` (underscored keys, `report` body from stdin with a tty refusal rc 2, `start` session defaulting from `CLAUDE_CODE_SESSION_ID`), `show_main`, `hook_main` (fail-closed gate, `HOOK_FIELDS` + 4000-char prompt), `deny_json`. Output goes through ignore-error writers so a closed pipe cannot panic.
- **`src/state_reader/driver_frontmatter.rs`**: `DerivedStatus`, `DriverRunView`, `derive_status` (unix) implementing the D-22 table over a private `Seen` reduction shared with the non-unix path, `read_driver_run`, `FrontmatterError::Io`.
- **Tests**: `tests/driverctl_client.rs` (19 tests through the real binary against a recording unix-socket server, every spawn under `XDG_DATA_HOME=/proc/nonexistent-gsdmm`), `tests/driverctl_liveness.rs` (11 tests, R1-R6 on real `flock` files), `tests/fixtures/driverctl/MASTER-PROMPT.md` (`cmp`-identical to the spike).

## Task commits

1. Task 1 (tracer) `787923b`: CLI surface, sync main, `log` client, 7 tests. Tracer re-verified end-to-end before Task 2.
2. Task 2 `5663eba`: `hook` gate, `show`, MASTER-PROMPT fixture and parse test, 12 more tests.
3. Clippy fix `30568d8`: `Log(Box<DriverctlLog>)` for `large_enum_variant`.
4. Task 3 `7f55926`: `derive_status` / `read_driver_run` and the R1-R6 suite.

## Verification

- `cargo test --no-fail-fast --test driverctl_client`: 19 passed. `--test driverctl_liveness`: 11 passed. `--lib cli::`: 13 passed. `--test async_blocking_guard`: 9 passed.
- `cargo clippy --all-targets -- -D warnings`: clean. `cargo fmt --check`: clean.
- Full `cargo test --no-fail-fast`: lib 2177 passed, 1 failed = the expected git-version constants test (`envelope::policy`); every other binary green.
- Mutation check: disabling the gate's deny branch makes `gate_fails_closed_*` fail (3 of 3), so the tests discriminate.
- Acceptance greps: `hide = true` x9, `driverctl_cli::run` x2, `new_multi_thread` x1, `REJECTED` x2, `onFailure` x1, `RollingFileAppender::builder` x1, `pub fn derive_status` / `read_driver_run` / `fresh-exit` present, `cmp` of the fixture exits 0.

## Phase 28 hand-off

- **`"onFailure": "block"`** on the seeded PreToolUse `Agent|Task` hook (D-25): the client closes every failure it can observe, but a missing or killed binary exits non-zero and Claude Code treats that as a non-blocking error. The setting makes even that deny. Recorded in the `driverctl_cli` header (fact 5).
- **Hold the run lock across `fresh-exit` relaunches** (R3): `end --reason fresh-exit` records `running`, so releasing `launcher.lock` between fresh-per-item launches makes the reader show `crashed` in the gap. Also (R4): acquire the lock before writing `running`; write the terminal status before releasing it. Both are documented on `derive_status`.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] clippy `large_enum_variant` on `Commands`/`DriverctlAction`**
- **Found during:** Task 3 verify (`clippy --all-targets -D warnings`)
- **Issue:** the 17-field `DriverctlLog` made `DriverctlAction` (and through it `Commands`) 432 bytes against Drive's 225.
- **Fix:** `Log(Box<DriverctlLog>)`; parse tests unbox. Wire behaviour unchanged.
- **Files modified:** `src/cli.rs`. **Commit:** `30568d8`

### Plan-ordering notes (no behaviour change)

- The fallible tracing init (`RollingFileAppender::builder`) landed in Task 1, not Task 2, because rewriting `main` for the driverctl dispatch is where `init_tracing` had to be extracted. Task 2 added its regression test (`fallible_tracing_survives_unwritable_data_dir`).
- Task 2 is `tdd="true"` but tests and implementation were committed together (no separate RED commit); discriminating power was shown by the mutation check above instead.

**Total deviations:** 1 auto-fixed (Rule 3). **Impact:** none on the contract.

## Inferred decisions (audit)

- `[inferred — audit]` The MASTER-PROMPT recording block holds 10 `driverctl` lines, but the plan asks for at least 11 exercised. `[--rule <rule>]` is optional in the prompt, so that line is run in both forms (without, and with `--rule budget`), giving 11 runs; this exercises a real driver variant rather than padding.
- `[inferred — audit]` `ClientError::BadResponse` (a non-JSON or non-object reply) denies the gate, same as a transport error: the server not giving an affirmative answer is not a pass. The reason text reads "driverctl unreachable (...)".
- `[inferred — audit]` Reply deadline is TOTAL (15 s from connect), not per-`recv` as in the spike, so a trickling server cannot hold a hook open past the cap (T-27-14).
- `[inferred — audit]` `report` stdin is read as bytes and decoded lossily; the spike would raise on invalid UTF-8. A worker report is text, and a replacement character is better than losing the whole report.
- `[inferred — audit]` A reply `ok:true` with `deny` set denies even for a non-gate payload (the spike printed the deny whenever the server sent one); only the unreachable/`ok:false` fallbacks are gate-only.
- `[inferred — audit]` On non-unix, `starting` with `launches == 0` still reads `NotLaunched` (needs no lock); only `running` and a launched `starting` become `LivenessUnknown`.
- `[inferred — audit]` `Starting` with `launches == 0` and an Unknown probe reads `NotLaunched` (the plan's "otherwise NotLaunched").
- `[inferred — audit]` `DriverctlLog` derives `Default`, `Debug`, `Clone`, `PartialEq`, `Eq` for test ergonomics.
- `[inferred — audit]` REQUIREMENTS.md was not touched: DCTL-01/03/04 are shared with sibling plans 27-05/27-06 in this wave, so the shared-ID gate (#2388) would block the checkoff anyway and the orchestrator owns it.

## Threat Flags

None. The new surface (a unix-socket client and a stdin-reading hook) is the one the plan's threat model covers (T-27-11 .. T-27-14); no endpoint, auth path or file access was added beyond it.

## Known Stubs

None.

## Self-Check: PASSED

- Files: `src/driverctl_cli.rs`, `tests/driverctl_client.rs`, `tests/driverctl_liveness.rs`, `tests/fixtures/driverctl/MASTER-PROMPT.md` exist.
- Commits `787923b`, `5663eba`, `30568d8`, `7f55926` are ancestors of HEAD.
