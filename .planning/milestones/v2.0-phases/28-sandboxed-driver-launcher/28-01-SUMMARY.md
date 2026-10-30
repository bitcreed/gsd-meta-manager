---
phase: 28-sandboxed-driver-launcher
plan: 01
subsystem: launcher
tags: [launcher, bwrap, systemd-run, driverctl, sandbox, rust, oauth]

requires:
  - phase: 27-driverctl-driver-frontmatter
    provides: "RunStore, run lock, serve socket/ticker, classify_exit, read_driver_run, the 9 hand-off items"
provides:
  - "crate::launcher (sandbox, config_dir, auth, process, run) and LauncherError with exit codes"
  - "gsd-meta-manager driver launch <alias|path> <run-id> --prompt-file <P> (foreground, headless)"
  - "serve.rs embed entry points: hash8, bind_run_socket, tick_once, spawn_accept_loop/EmbeddedServer, pub(crate) lock_store"
  - "tests/launcher_support (fake bwrap/systemd-run/systemctl/claude shims) and tests/launcher_e2e.rs"
affects: [28-02, 28-03, 28-04, 28-05, 28-06, 28-07, 28-08, 28-09, phase-29, phase-30]

plan_head_before: a7abfc58bad8c5f67dec6c5074c093abd40702bf
plan_head_after: bda1ad9b02acd256db67915914fbbf49d04f9fd9
commits: 1
actuals:
  tokens: 28200
  tasks: 1
  commits: 1

tech-stack:
  added: []
  patterns:
    - "Pure argv builders (sandbox.rs) golden-tested; one spawn file (process.rs) with one spawn-guard entry"
    - "Embedded RunStore on std threads: lock first, store dropped last (R4)"
    - "Fake-binary shims on PATH for CI-safe end-to-end launcher tests"

key-files:
  created:
    - src/launcher/mod.rs
    - src/launcher/run.rs
    - src/launcher/process.rs
    - src/launcher/sandbox.rs
    - src/launcher/config_dir.rs
    - src/launcher/auth.rs
    - src/driver_cli.rs
    - tests/launcher_support/mod.rs
    - tests/launcher_e2e.rs
  modified:
    - src/lib.rs
    - src/cli.rs
    - src/main.rs
    - src/driverctl/serve.rs
    - tests/spawn_seam_guard.rs

key-decisions:
  - "env_clear + exact map for the driver child (pure child_env decides the scrub) instead of env_remove [inferred - audit]"
  - "Launcher state dir is created after the run lock, not before it, so a refused launch writes nothing at all [inferred - audit]"
  - "A corrupt or non-object host settings.json seeds as an empty object rather than blocking the launch [inferred - audit]"
  - "A launch-end verdict of None (driver ended the run) writes nothing in this plan; RunStore::end_launch is 28-02's"

requirements-completed: [LNCH-01, LNCH-08]

coverage:
  - id: D1
    description: "driver launch holds the run lock, embeds the store and socket, runs the driver as claude in bwrap in a systemd scope, and records its end before releasing the lock"
    requirement: LNCH-01
    verification:
      - kind: e2e
        ref: "tests/launcher_e2e.rs#launch_runs_the_driver_in_the_scope_and_records_its_end"
        status: pass
    human_judgment: false
  - id: D2
    description: "No tmux and no TTY on the launch path; driver stdin is /dev/null"
    requirement: LNCH-08
    verification:
      - kind: e2e
        ref: "tests/launcher_e2e.rs#launch_runs_the_driver_in_the_scope_and_records_its_end"
        status: pass
    human_judgment: false
  - id: D3
    description: "OAuth token only in the child environment: stored 0600 token first, host access token fallback refused under 5 min; never in argv or any launcher-written file"
    verification:
      - kind: unit
        ref: "src/launcher/auth.rs tests (7) plus the e2e token scan"
        status: pass
    human_judgment: false
  - id: D4
    description: "Pure sandbox argv builders (systemd-run, bwrap, claude, prompt, env) and per-run config seeding with the onFailure:block gate"
    verification:
      - kind: unit
        ref: "src/launcher/sandbox.rs and src/launcher/config_dir.rs tests"
        status: pass
    human_judgment: false
  - id: D5
    description: "Real kernel isolation under the real bwrap/systemd-run (EROFS on the run dir, masked locks, stripped env, scope cgroup)"
    verification: []
    human_judgment: true
    rationale: "Proven once by hand in a live smoke on this host (see Live smoke); the committed opt-in live tests with captured evidence are 28-03's"

duration: ~55min (start time not captured; estimated)
completed: 2026-10-19
status: complete
---

# Phase 28 Plan 01: Sandboxed Driver Launcher Tracer Summary

**`gsd-meta-manager driver launch` takes the run lock, embeds the Phase 27 `RunStore` and its socket, seeds a per-run `CLAUDE_CONFIG_DIR`, runs the headless driver as `claude -p` inside `bwrap` under `systemd-run --user --scope`, and records how the launch ended before releasing the lock.**

## Performance

- **Duration:** ~55 min (the start time was not captured; this is an estimate)
- **Completed:** 2026-10-19
- **Tasks:** 1 (a tracer)
- **Files modified:** 14 (9 created, 5 modified)

## Accomplishments

- The launch architecture is proven end to end on one path, both with fake tools in CI and once against the real `systemd-run` + `bwrap` on this host.
- Phase 27's `serve` is now embeddable: `hash8`, `bind_run_socket`, `tick_once`, `spawn_accept_loop` / `EmbeddedServer` and a `pub(crate) lock_store`, with `serve()` rebuilt on the same functions (`tests/driverctl_serve.rs` is unchanged and green).
- One spawn file (`src/launcher/process.rs`, one spawn-guard entry) and pure, golden-testable argv builders (`sandbox.rs`); the OAuth token reaches the child only as `CLAUDE_CODE_OAUTH_TOKEN`.
- The cap limits (10 / 60 / 2, spike defaults) are recorded through `LaunchSpec.limits`; enforcement is 28-05.

## Task Commits

1. **Task 1: Tracer - `driver launch` runs one fake driver in bwrap under a systemd scope and records its end** - `bda1ad9` (feat)

**Plan metadata:** the docs commit that carries this file.

## The argv the tracer captured

The shim log of `launch_runs_the_driver_in_the_scope_and_records_its_end`, one line per tool (`<hash8>` is the first 4 bytes of sha256 of the canonical run dir; masks appear only when the host path exists, and the fixture HOME has none):

```
systemd-run --user --scope --quiet --collect --unit=gsdmm-driver-r28-<hash8>-1 -p TimeoutStopSec=15 -p MemoryMax=6G -p TasksMax=1024 -- bwrap --ro-bind / / --dev /dev --proc /proc --tmpfs /tmp --bind <project> <project> --ro-bind <run_dir> <run_dir> --ro-bind /dev/null <run_dir>/launcher.lock --ro-bind /dev/null <run_dir>/DIRECTION.lock --bind <config_dir> <config_dir> --unshare-pid --die-with-parent --chdir <project> -- claude -p <rendered prompt> --model opus --output-format stream-json --verbose --permission-mode bypassPermissions --max-budget-usd 10
systemctl --user stop gsdmm-driver-r28-<hash8>-1.scope
```

The recorded launch line is `RUN | launch | n=1 | mode=bwrap-scope | model=opus`, followed by the driver's `RUN | start` and `RUN | end | reason=stop`; the reader derives `AsWritten(Stopped)` and the lock probes `Free`.

## Live smoke (not a committed test)

Run once by hand with the real `systemd-run`, `bwrap` and `systemctl` and a fake `claude` (no API spend), script kept at `/tmp/claude-1000/p28/live-smoke.sh` (not committed). Observed inside the sandbox: cgroup `.../app.slice/gsdmm-driver-live1-b7fe3523-1.scope`, `pid=2` (pid namespace), stdin `/dev/null`, `SSH_AUTH_SOCK`/`GH_TOKEN`/`ANTHROPIC_API_KEY` unset, token present in the environment, `write to <run_dir>/LOG.md` fails `Read-only file system`, `cat <run_dir>/launcher.lock` fails `Permission denied`, launch rc 0 with `status=stopped reason=driver:stop`, no leftover scope unit or socket afterwards. This is evidence for this plan only; the opt-in live isolation test with captured evidence remains 28-03's.

## Files Created/Modified

- `src/launcher/mod.rs` - module root, numbered facts, `LauncherError` (`Locked` -> exit 3, else 1)
- `src/launcher/sandbox.rs` - pure `bwrap_argv`, `systemd_run_argv`, `claude_argv`, `render_prompt`, `child_env`, `scope_unit_name`, `STRIP_ENV`
- `src/launcher/config_dir.rs` - `driver_root`, `run_state_dir_in`, `seed`, `hook_command`; the PreToolUse gate carries `onFailure: block`
- `src/launcher/auth.rs` - stored 0600 token, host access-token fallback (refused under 5 min), redacting `LaunchToken`
- `src/launcher/process.rs` - the one spawn file: `spawn_launch_child`, `run_bounded`
- `src/launcher/run.rs` - `launch`, `LaunchOptions`, `LoopMode`, `LaunchSummary`
- `src/driver_cli.rs`, `src/cli.rs`, `src/main.rs` - the `driver launch` subcommand, dispatched synchronously before the tokio runtime
- `src/driverctl/serve.rs` - the embed entry points
- `tests/spawn_seam_guard.rs` - the `src/launcher/process.rs` allowlist entry
- `tests/launcher_support/mod.rs`, `tests/launcher_e2e.rs` - fake-tool fixture and 10 end-to-end tests

## Decisions Made

All `[inferred - audit]` unless noted:

- **`env_clear()` plus the exact `child_env` map** for the driver child (the plan's recorded deviation from the `env_remove` analog), so the scrub is a pure, testable function.
- **The state directory is created after the run lock**, not before it as the plan's listed order had it. The truth says the lock comes before anything is written; the e2e asserts a held-lock refusal leaves no state dir, no LOG, no `state.json`.
- **Teardown order:** stop serving (drain in-flight handlers), classify and `set_terminal`, remove the socket, drop the store last. No request can write after the verdict.
- **One extra `tick_once` after the driver exits**, so a `gaps_found` write made just before exit is still observed.
- **A corrupt or non-object host `settings.json`** seeds as an empty object (the driver then has only the driverctl hooks) instead of blocking every launch.
- **`expiresAt`** is accepted as an integer or a float of epoch milliseconds.
- `--loop-mode` only fills `{MODE}` in this plan; the fresh-per-item relaunch behaviour is 28-02.

## Deviations from Plan

None in behaviour - the plan was executed as written. Two small additions beyond its text, both inside its file list: the extra e2e tests (cap flags recorded, crashed and stalled classification, held lock -> rc 3 with nothing written, missing or world-readable token, bad run ids, unknown project), and `DRIVERS_REL` in `serve.rs` made `pub(crate)` so `run.rs` does not duplicate the constant.

**Total deviations:** 0 auto-fixed. **Impact:** none.

## Verification

- `launcher_e2e`: 10 passed. `--lib launcher::`: all pass. `spawn_seam_guard`, `driverctl_serve`, `async_blocking_guard`: pass.
- Full `cargo test --no-fail-fast` (69 targets): 3274 passed, 1 failed, 18 ignored. The one failure is the expected local `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against` (pinned to the CI git version).
- `cargo clippy --all-targets -- -D warnings`: clean. `cargo fmt --check`: clean.
- Acceptance greps: `src/launcher/process.rs` in the spawn allowlist (1), `pub mod launcher` (1), `mod driver_cli` (1), `pub fn hash8` (1), `pub(crate) fn tick_once` (1), and in `run.rs` the first `acquire_at_with_retry` (line 48) precedes the first `bind_run_socket` (52) and `begin_launch` (first call, line 315).

## Pending live

None required by this plan. The committed live isolation test (28-03) will use `cargo test --test launcher_live_isolation -- --ignored --nocapture`.

## Handoffs

- 28-02: `RunStore::end_launch` is needed - when the verdict is `None` this plan writes no `launch-end` and leaves the launcher/driver connections `connected` in `state.json`.
- 28-03: `LaunchPaths.socket` is carried but unused until the `$XDG_RUNTIME_DIR` tmpfs and socket re-bind exist; the socket is reachable today only because the root is bound read-only.
- 28-05: `systemd_run_argv` already accepts `runtime_max_sec`; `run.rs` passes `None`.

## Known Stubs

None.

## Threat Flags

None. The surfaces added (the stored-token read, the per-run config dir, the existing driverctl socket) are the ones the plan's threat model names (T-28-02, T-28-03, T-28-04).

## Self-Check: PASSED
