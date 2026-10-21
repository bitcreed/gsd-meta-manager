---
phase: 28-sandboxed-driver-launcher
plan: 04
subsystem: launcher
tags: [launcher, host-detection, degraded-mode, process-group, frontmatter, rust]

requires:
  - phase: 28-sandboxed-driver-launcher
    provides: "28-02: run loop, Containment::Scope, user-stop path, launch_argv; 28-03: bwrap_argv / child_env / hardened profile"
  - phase: 27-driverctl-driver-frontmatter
    provides: "RunStore, DriverFrontmatter emit/parse, byte-exact goldens"
provides:
  - "launcher::host::{HostCaps, Posture, detect, decide, parse_user_manager, parse_claude_version, MIN_CLAUDE_VERSION}"
  - "LauncherError::{SandboxUnavailable, ClaudeVersion} (rc 1)"
  - "driver launch --unsandboxed (LaunchArgs.unsandboxed, LaunchOptions.unsandboxed, launch_argv)"
  - "Containment::Group { pgid } and stop_group (sync twin of the Phase 17 kill escalation)"
  - "LaunchSpec.sandbox / sandbox_missing -> state.json extra -> DriverFrontmatter.sandbox / sandbox_missing"
  - "tests/launcher_degraded.rs (16 tests)"
affects: [28-05, 28-07, 28-08, 28-09, 30]

plan_head_before: eaa546fbf4e71722dda5b5b8ed8d1f1d083fd5dc
plan_head_after: 86405f82736258c09cbff6d00b97aa9d70a63fcb
commits: 3
actuals:
  tokens: 11600
  tasks: 2
  commits: 3

tech-stack:
  added: []
  patterns:
    - "Capability probes execute the tool (bwrap runs a trivial command) and parse stdout, not exit codes, for the user manager"
    - "Refusal is a typed error decided by a pure function (decide) over a probe result, so the policy is unit-testable without any host tool"
    - "Additive frontmatter keys emitted only when set, so byte-exact goldens never move"

key-files:
  created:
    - src/launcher/host.rs
    - tests/launcher_degraded.rs
  modified:
    - src/launcher/run.rs
    - src/launcher/mod.rs
    - src/cli.rs
    - src/driver_cli.rs
    - src/driverctl/store.rs
    - src/state_reader/driver_frontmatter.rs
    - tests/driverctl_frontmatter.rs
    - tests/driverctl_store.rs
    - tests/launcher_e2e.rs

key-decisions:
  - "A claude that is absent, unparseable or older than 2.1.295 is refused in BOTH modes [inferred - audit]"
  - "--unsandboxed degrades a host that could sandbox too (the flag is the operator's choice); sandbox_missing is then empty and omitted [inferred - audit]"
  - "A degraded launch records no scope_unit (there is none) and the launcher connection mode is 'degraded' [inferred - audit]"
  - "Posture keys belong to the launch: begin_launch removes a stale sandbox / sandbox_missing when the next launch carries none (a later driverctl serve) [inferred - audit]"
  - "After a normal exit and after a stop the degraded group gets a final SIGKILL, accepting the millisecond pid-reuse window after the leader is reaped [inferred - audit]"
  - "launch_detached runs the probes too, so the refusal reaches the caller's terminal; the daemon decides again for itself [inferred - audit]"

requirements-completed: [LNCH-07]

coverage:
  - id: D1
    description: "A host whose bwrap does not run, whose systemd-run is absent or whose user manager is not running/degraded is refused (rc 1) naming each missing piece and --unsandboxed, writing nothing and starting no driver, in both foreground and detached launches; a degraded manager (exit 1) is usable"
    requirement: LNCH-07
    verification:
      - kind: e2e
        ref: "tests/launcher_degraded.rs#launch_refuses_without_a_working_bwrap"
        status: pass
      - kind: e2e
        ref: "tests/launcher_degraded.rs#launch_refuses_when_the_user_manager_is_offline"
        status: pass
      - kind: e2e
        ref: "tests/launcher_degraded.rs#a_degraded_user_manager_is_usable"
        status: pass
      - kind: e2e
        ref: "tests/launcher_degraded.rs#a_detached_launch_is_refused_in_the_callers_terminal"
        status: pass
      - kind: e2e
        ref: "tests/launcher_degraded.rs#the_flag_is_never_implied_by_a_failed_probe"
        status: pass
    human_judgment: false
  - id: D2
    description: "claude --version below 2.1.295 or unparseable is refused in both modes"
    requirement: LNCH-07
    verification:
      - kind: e2e
        ref: "tests/launcher_degraded.rs#launch_refuses_an_old_claude"
        status: pass
      - kind: unit
        ref: "src/launcher/host.rs#an_old_or_unreadable_claude_is_refused_in_both_modes"
        status: pass
    human_judgment: false
  - id: D3
    description: "--unsandboxed runs claude with no bwrap and no scope as a supervised process group; stop kills the whole group, escalates to SIGKILL for a SIGTERM-proof driver after the 12 s grace, and a normal exit leaves no group descendant"
    requirement: LNCH-07
    verification:
      - kind: e2e
        ref: "tests/launcher_degraded.rs#unsandboxed_launch_runs_without_bwrap_and_records_degraded"
        status: pass
      - kind: e2e
        ref: "tests/launcher_degraded.rs#unsandboxed_stop_kills_the_group"
        status: pass
      - kind: e2e
        ref: "tests/launcher_degraded.rs#unsandboxed_stop_escalates_to_kill_for_a_driver_that_ignores_term"
        status: pass
      - kind: e2e
        ref: "tests/launcher_degraded.rs#unsandboxed_normal_exit_leaves_no_group_descendant"
        status: pass
    human_judgment: false
  - id: D4
    description: "The posture (sandbox: bwrap-scope|degraded, sandbox_missing) is recorded in state.json and the DIRECTION frontmatter, only when set; every existing golden is byte-identical"
    requirement: LNCH-07
    verification:
      - kind: e2e
        ref: "tests/launcher_degraded.rs#unsandboxed_launch_records_degraded_posture"
        status: pass
      - kind: e2e
        ref: "tests/launcher_degraded.rs#sandboxed_launch_records_bwrap_scope_posture"
        status: pass
      - kind: integration
        ref: "tests/driverctl_store.rs#begin_launch_records_the_posture_in_state_and_frontmatter"
        status: pass
      - kind: unit
        ref: "src/state_reader/driver_frontmatter.rs#the_sandbox_keys_round_trip_and_sit_right_after_launches"
        status: pass
      - kind: integration
        ref: "tests/driverctl_frontmatter.rs (all goldens)"
        status: pass
    human_judgment: false
  - id: D5
    description: "On non-Linux targets driver launch without --unsandboxed is refused naming bwrap and systemd-run, and with it returns the unsupported-platform error; this narrows SC6's macOS example"
    requirement: LNCH-07
    verification: []
    human_judgment: true
    rationale: "The non-Linux arm of driver_cli.rs cannot be compiled or run on this Linux host (only the x86_64-unknown-linux-gnu target is installed); verify with cargo check --target aarch64-apple-darwin on a macOS runner"

duration: ~40min (start not captured; estimated from the first compile)
completed: 2026-10-19
status: complete
---

# Phase 28 Plan 04: Degraded Mode and Posture Summary

**A host that cannot run bwrap, systemd-run or a user manager (or has a Claude Code older than 2.1.295) is now refused with a message naming each missing piece and `--unsandboxed`; that flag, per launch, runs the driver as a supervised process group (TERM, 12 s, KILL), and every run records `sandbox: bwrap-scope|degraded` (+ `sandbox_missing`) without moving a single existing frontmatter byte.**

## Performance

- **Tasks:** 2 (a tracer and a `tdd="true"` task), 3 commits (1 tracer `feat`, RED `test`, GREEN `feat`)
- **Files modified:** 11 (2 created, 9 modified); +1187 / -18 lines

## Accomplishments

- `launcher::host::detect` runs, through the one launcher spawn file, `bwrap --ro-bind / / --dev /dev -- true` (must exit 0), `systemd-run --version`, `systemctl --user is-system-running` (stdout `running`/`degraded`; exit code ignored) and `claude --version`, each within 5 s. `decide` (pure) turns that plus the flag into a `Posture` or a typed refusal. Missing names are `bwrap`, `systemd-run`, `user-systemd`.
- The probes run in `launch` after every refusal that needs no process (run id, token, prompt) and before the lock, and in `launch_detached` before any state dir or log exists. Run dir byte-identical, no `launcher.lock`, no state dir, no driver, asserted for each refusal.
- `Containment::Group`: `signal_group(pgid, TERM)`, poll up to `DRIVER_TEARDOWN_GRACE`, then `signal_group(KILL)`; a final group KILL runs after the grace and after a normal exit. Mutation-checked (see below).
- `LaunchSpec.sandbox` / `sandbox_missing` land in `state.json`'s `extra`, `RunStore::frontmatter` maps them, `emit` writes them right after `launches:` only when set. `tests/fixtures/driverctl/golden` is untouched since Phase 27 (`git diff ca2473b` exits 0) and all 11 golden tests pass.

## Task Commits

1. **Task 1 (tracer): host detection, refusal, `--unsandboxed` process-group launch** - `163ad66` (feat)
2. **Task 2 (tdd): posture in state and frontmatter**
   - RED `6eb7d34` (test)
   - GREEN `86405f8` (feat)

**Plan metadata:** the docs commit carrying this file.

## TDD Gate Compliance

RED (`test(28-04)` `6eb7d34`) precedes GREEN (`feat(28-04)` `86405f8`) for Task 2. RED added the new struct fields as no-ops (`LaunchSpec.sandbox*`, `DriverFrontmatter.sandbox*`, emit and mapping absent) so the tests compile; they then failed on the planned assertions: `state["sandbox"]` was `Null` instead of `"degraded"` / `"bwrap-scope"` (store tests), `frontmatter.sandbox` was `None` instead of `Some("degraded")` (3 launcher tests), and the two frontmatter round-trip unit tests failed on the missing `sandbox:` line. `a_launch_without_a_posture_writes_neither_key_and_clears_a_stale_one` passed at RED: it characterises the existing "no key" behaviour the plan had to keep. `gsd_run check tdd-red-evidence` accepts TAP / JUnit / swift-testing / unittest / go output only and `cargo test` output is none of those, so the machine classifier was not run; the semantic assessment is by hand as above. Task 1 is a tracer (not `tdd="true"`): one `feat` commit with its tests, as in 28-03. `workflow.tdd_mode` is not set; no REFACTOR commit.

## D-31 narrowing

SC6's example names macOS ("a macOS host without bwrap/systemd runs degraded"). In this phase that clause is satisfied **only as a refusal on macOS**:

- On non-Linux targets `driver launch` without `--unsandboxed` is refused with the `SandboxUnavailable` text naming `bwrap` and `systemd-run`, and with `--unsandboxed` it prints `unsupported platform: the run store needs Linux (no renamex_np splice yet)`; rc 1 both (`src/driver_cli.rs`, the `not(target_os = "linux")` arm).
- Degraded mode is **functional on Linux hosts without bwrap or systemd** (proven here with a failing `bwrap` shim). The run store (`flock`, `renameat2` splice) is Linux-only in Phase 27, so a working degraded macOS launch needs the `renamex_np` splice, which is not in this phase.

The phase verifier should read SC6's "(macOS)" example with this narrowing. The non-Linux arm could not be compiled here (only `x86_64-unknown-linux-gnu` is installed): it is a three-line arm using only types that exist on every platform, but it is unverified until a macOS runner does `cargo check`.

**D-31 re-assessment (2026-10-21, quick 261021-0jg)** `[inferred — audit]`: macOS `driver launch --unsandboxed` is now supported at the code level. This supersedes the "no renamex_np splice yet" narrowing above. Evidence:

- The DIRECTION.md splice is portable: `src/driver/direction.rs:408` and `:476` gate on linux+macos, and rustix 1.1.5 maps `RenameFlags::EXCHANGE` to `RENAME_SWAP` via `renameatx_np` (`rustix-1.1.5/src/backend/libc/fs/types.rs:544-545`, `syscalls.rs:594-616`).
- The stop path is flock-based (`src/launcher/stop.rs`) and `signal_group` is POSIX (`src/driver/kill.rs:183`).
- The socket base falls back to an absolute `$TMPDIR` on macOS when `XDG_RUNTIME_DIR` is unset (`src/driverctl/serve.rs:291` `socket_base_dir`, called from `socket_path` at `:274`). `src/launcher/run.rs:622` and `driverctl serve` (`src/driverctl/serve.rs:862`) both reach it through `socket_path`; the `gsdmm-driverctl` subdir keeps the 0700 / real-dir / owner / no-group-other checks. Linux never consults `TMPDIR`.
- `MAX_SUN_PATH` is 103 on macOS (`src/driverctl/serve.rs:90`; 107 elsewhere at `:94`).

Remaining caveats:

- Runtime is unverified without a Mac (`28-UAT.md:23`, item 2).
- Liveness reports Unknown off Linux (`src/driver/liveness.rs:56`, `LIVENESS_SUPPORTED`).
- Store `host_name` is None off Linux (`src/driverctl/store.rs:430` reads `/proc/sys/kernel/hostname`).
- `RENAME_SWAP` needs APFS; `ENOTSUP` surfaces as an error and is not a partial write.
- TUI-side launch is still Linux-gated (the `cfg(target_os = "linux")` launch arms in `src/app.rs`, e.g. `:589`, and `src/ui/screens/driver_launch.rs:38`).
- Under a stock `/var/folders` TMPDIR (about 48 bytes) the 103-byte limit leaves roughly 24 bytes for the run id. A longer id is refused with the typed length error, and `driver launch` has no `--sock` override `[inferred — audit]`.

## Verification

- Full `cargo test --no-fail-fast`: **3343 passed, 1 failed, 22 ignored.** The one failure is the expected local `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`.
- Plan verify set, all green: `launcher_degraded` 16, `driverctl_frontmatter` 11, `driverctl_store` 39, `launcher_e2e` 17, `launcher_stop` 5, `--lib launcher::host` 9, `--lib state_reader::driver_frontmatter` 8, plus `spawn_seam_guard` 41 and `async_blocking_guard` 9 (`host.rs` adds no `Command::new`).
- `cargo clippy --all-targets -- -D warnings`: clean. `cargo fmt --check`: clean.
- Acceptance greps: `pub fn detect` in host.rs 1; `signal_group` in run.rs 4; `unsandboxed` in cli.rs 1 (was 0); `sandbox` in driver_frontmatter.rs 21 (was 0); goldens untouched.
- **Mutation check (not committed).** Replacing the group TERM with a leader-only `child.kill()` and dropping the final group KILL made both `unsandboxed_stop_kills_the_group` (`leader alive=false background alive=true`) and the escalation test fail: the tests distinguish a group stop from a leader stop.
- **Real-host probe results** (the commands `detect` runs, run by hand on this host): `bwrap --ro-bind / / --dev /dev -- true` rc 0; `systemd-run --version` systemd 259; `systemctl --user is-system-running` prints `degraded` and exits 1 (so a real default launch here classifies the manager as usable only because the parser ignores the exit code); `claude --version` 2.1.303.

## Files Created/Modified

- `src/launcher/host.rs` - probes, parsers, `Posture`, `decide`, with 9 unit tests
- `src/launcher/run.rs` - `LaunchOptions.unsandboxed`, `LaunchContext.posture`, `Containment::Group`, `stop_group`, degraded argv/spawn, probes before the lock, `--unsandboxed` in `launch_argv`
- `src/launcher/mod.rs` - `SandboxUnavailable`, `ClaudeVersion`, module facts
- `src/cli.rs`, `src/driver_cli.rs` - `--unsandboxed`, non-Linux arm (D-31)
- `src/driverctl/store.rs` - `LaunchSpec.sandbox*`, `begin_launch` writes/clears the keys, `frontmatter` maps them
- `src/state_reader/driver_frontmatter.rs` - the two fields, additive emit
- `tests/launcher_degraded.rs` (new), `tests/driverctl_store.rs`, `tests/driverctl_frontmatter.rs` (literal), `tests/launcher_e2e.rs`

## Decisions Made

All `[inferred - audit]` unless the plan stated them:

- Version gate applies in both modes (plan-stated); an unparseable `claude --version` is also refused.
- `--unsandboxed` degrades even a complete host (D-29 reads "runs claude with no bwrap and no scope"); `sandbox_missing` is then absent rather than `[]`.
- A degraded launch has `scope_unit: None`, so `driver stop` on a free lock (which stops only a recorded `gsdmm-driver-*.scope`) finds nothing to stop for a degraded run whose launcher died.
- `begin_launch` clears stale posture keys, so a standalone launch after a degraded one does not inherit it.
- The final group `SIGKILL` follows a reaped leader; the pid-reuse window is a millisecond and needs a full pid wrap-around.
- `launch_detached` also probes so the refusal prints in the caller's terminal instead of only reaching `launcher.log`.

## Deviations from Plan

**1. [Rule 3 - Blocking] Existing e2e assertions assumed an empty shim log or that the first `systemd-run` line is the launch**
- **Found during:** Task 1
- **Issue:** the capability probes run `systemd-run --version` and `bwrap ... -- true` through the same logging shims, so `launch_runs_the_driver_in_the_scope_and_records_its_end` (`find(starts_with("systemd-run "))`) read the probe line and `a_held_run_lock_is_refused_with_exit_3_before_anything_is_written` (`shim_log().is_empty()`) saw the probes (the plan puts the probes before the lock, so a launch refused for a held lock has run them).
- **Fix:** the first looks for `systemd-run --user`; the second asserts no `systemd-run --user` and no `claude -p` line. The token / run-id / registry refusals still assert an empty shim log (they refuse before the probes).
- **Files modified:** `tests/launcher_e2e.rs`
- **Committed in:** `163ad66`

**2. [Rule 2 - Missing critical] Tests the plan did not list**
- Added: `a_detached_launch_is_refused_in_the_callers_terminal`, `a_degraded_user_manager_is_usable`, `a_missing_systemd_run_is_named`, `an_unreadable_claude_version_is_refused`, `the_flag_is_never_implied_by_a_failed_probe` (the prohibition), `unsandboxed_stop_escalates_to_kill_for_a_driver_that_ignores_term` (T-28-08's KILL leg, ~13 s), `unsandboxed_normal_exit_leaves_no_group_descendant`, `unsandboxed_run_holds_the_lock_like_a_sandboxed_one`, `unsandboxed_on_a_complete_host_is_degraded_with_nothing_missing`, and the store tests including stale-key clearing.

**Total deviations:** 1 auto-fixed (Rule 3), 1 test addition (Rule 2). **Impact:** none on behaviour.

## Issues Encountered

None beyond the Rule 3 item. The mutation run left one `sleep 600` from this fixture; it was identified by its `FAKE_SHIM_LOG` environment and killed.

## Pending live

None required. The degraded path is fully testable through fakes; no live bwrap / systemd run is needed for this plan (28-03's live probes cover the sandboxed arm). Non-Linux compile check, on a macOS runner: `cargo check --target aarch64-apple-darwin`.

## Limits of the degraded mode (for the TUI and the verifier)

- A degraded driver has none of 28-03's isolation: it runs as the user with the user's `HOME`, ssh agent socket path (the env var is still stripped), network and files. Only the env scrub, the per-run config dir and the hooks apply. The posture says so (`sandbox: degraded`); 28-07 shows it.
- A process that leaves the group (`setsid`) escapes the group stop. A SIGKILLed launcher leaves a degraded driver running (no `--die-with-parent`); the run then reads `crashed`, and a later launch logs `crash-detected`.

## Handoffs

- 28-05: `Containment::Group` exists beside `Scope`; a cap kill goes through `Containment::stop` in both modes. `LaunchContext.posture` is available to the cap path.
- 28-07 / 28-08: read `DriverFrontmatter.sandbox` / `sandbox_missing` for the badge; `degraded` is never `bwrap-scope`.
- 28-09: the live harness may pass `--unsandboxed` only with intent; the launch lines for a sandboxed run are `systemd-run --user ...` and a `bwrap` line running `claude`.

## Known Stubs

None.

## Threat Flags

None. The surfaces added are the plan's T-28-06 (no silent unsandboxed run: refusal default, per-launch flag, posture recorded, all tested) and T-28-08 (group TERM -> 12 s -> KILL plus a final group KILL, tested).

## Self-Check: PASSED

Created files exist (`src/launcher/host.rs`, `tests/launcher_degraded.rs`); commits `163ad66`, `6eb7d34`, `86405f8` are ancestors of HEAD; the plan verify commands exit 0.
