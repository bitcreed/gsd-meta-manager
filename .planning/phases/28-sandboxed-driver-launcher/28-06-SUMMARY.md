---
phase: 28-sandboxed-driver-launcher
plan: 06
subsystem: launcher
tags: [launcher, auth, setup-token, stop-guard, hooks, driverctl, rust]

requires:
  - phase: 28-sandboxed-driver-launcher
    provides: "28-01 auth read path (token_path, read_stored_token, host_access_token, resolve_launch_token) and SeedSpec.run_dir; 28-03 settings_transform and its goldens; 28-05 stored-vs-host revocation handling"
  - phase: 27-driverctl-driver-frontmatter
    provides: "counters::outstanding_dispatch (the shared definition), log::parse_log, the driverctl subcommand group"
provides:
  - "launcher::auth::{validate_pasted_token, store_token, auth_status, auth_status_in, AuthStatus, has_expected_prefix, AuthError::Invalid}"
  - "launcher::process::run_interactive (inherited stdio, for claude setup-token)"
  - "gsd-meta-manager driver auth setup|status (DriverAction::Auth, AuthAction::{Setup, Status})"
  - "driverctl::stop_guard::{GUARD_LIMIT, guard, GuardOutcome, append_log, read_count} and the hidden driverctl stop-guard --run-dir"
  - "config_dir::settings_transform(host, hook_command, stop_guard_command): the guard is the last Stop entry"
affects: [28-07, 28-08, 28-09, phase-29, phase-30]

plan_head_before: 2b59e5bf8b6c61b24d488719bafc2b0129f65071
plan_head_after: a6eb87aeb70c66efec765825eef36bce0ccdfe52
commits: 3
actuals:
  tokens: 21000
  tasks: 2
  commits: 3

tech-stack:
  added: []
  patterns:
    - "A secret is validated, stored and reported by three different functions, and no error or status type holds its value"
    - "A hook that must never wedge the driver exits 0 on every path, catches panics, and fails open when its own state cannot be persisted"
    - "A counter in a model-writable directory is read as untrusted: any malformed shape is the zero state, so forgery can only shorten a budget"

key-files:
  created:
    - src/driverctl/stop_guard.rs
    - tests/launcher_auth.rs
    - tests/driverctl_stop_guard.rs
    - tests/fixtures/driverctl/spike003/r261-l8/stop-guard.log
  modified:
    - src/launcher/auth.rs
    - src/launcher/process.rs
    - src/launcher/config_dir.rs
    - src/cli.rs
    - src/driver_cli.rs
    - src/driverctl/mod.rs
    - src/driverctl_cli.rs
    - tests/launcher_config_dir.rs
    - tests/launcher_support/mod.rs

key-decisions:
  - "The token shape check is advisory: a value not starting with sk-ant-oat01- is stored with a one-line stderr warning [inferred - audit]"
  - "The guard budget is GUARD_LIMIT blocks per outstanding dispatch (spike WR-02); CONTEXT D-17's 'at most 3 blocks per run' is read as describing that guard [inferred - audit]"
  - "A pasted token must be printable ASCII (stricter than 'no whitespace or control character'), 20..=512 characters [inferred - audit]"
  - "An existing token directory is accepted when this user owns it; group/other write bits are removed from it instead of refusing, because the default directory also holds the registry and is group-writable under a 0002 umask [inferred - audit]"
  - "store_token creates the temp file with mode 0600 at open time and pins 0600 with fchmod, instead of wrapping the write in a process-wide umask(0o177) [inferred - audit]"
  - "A multi-line paste is refused by a non-blocking peek at stdin after the first line, so half of a paste is never stored [inferred - audit]"
  - "A stop-guard counter that cannot be written fails open as allow reason=counter-unwritable [inferred - audit]"
  - "driver auth status exits 0 whenever it can report, including when no token is usable [inferred - audit]"

requirements-completed: [LNCH-05, LNCH-03]

coverage:
  - id: D1
    description: "driver auth setup explains why a long-lived token, runs claude setup-token on the terminal, reads one pasted line, and stores it 0600 (0700 dir it created) without ever echoing it; a failing or absent claude, or a refused paste, stores nothing"
    requirement: LNCH-05
    verification:
      - kind: e2e
        ref: "tests/launcher_auth.rs#setup_stores_a_0600_token_and_never_echoes_it"
        status: pass
      - kind: e2e
        ref: "tests/launcher_auth.rs#setup_failing_setup_token_writes_nothing"
        status: pass
      - kind: e2e
        ref: "tests/launcher_auth.rs#setup_with_no_claude_on_the_path_writes_nothing"
        status: pass
      - kind: e2e
        ref: "tests/launcher_auth.rs#setup_refuses_an_unusable_paste_and_stores_nothing"
        status: pass
      - kind: unit
        ref: "src/launcher/auth.rs#a_pasted_token_that_is_not_one_clean_line_is_refused_without_echoing_it"
        status: pass
    human_judgment: false
  - id: D2
    description: "driver auth status reports presence, path, mode and the host token's expiry, flags a stored file a launch would refuse, and never prints a token value or the refresh token"
    requirement: LNCH-05
    verification:
      - kind: e2e
        ref: "tests/launcher_auth.rs#status_reports_presence_without_the_value"
        status: pass
      - kind: e2e
        ref: "tests/launcher_auth.rs#status_with_nothing_stored_says_so_and_names_the_setup_command"
        status: pass
      - kind: unit
        ref: "src/launcher/auth.rs#status_reports_presence_path_mode_and_host_expiry_and_no_value"
        status: pass
    human_judgment: false
  - id: D3
    description: "Token precedence is fixed: a stored token wins over the host token and the two are never combined; an unsafe or empty stored file is a typed refusal at launch with no host fallback; ANTHROPIC_API_KEY / ANTHROPIC_AUTH_TOKEN / apiKeyHelper are never consulted or passed"
    requirement: LNCH-05
    verification:
      - kind: e2e
        ref: "tests/launcher_auth.rs#stored_token_wins_over_the_host_token"
        status: pass
      - kind: e2e
        ref: "tests/launcher_auth.rs#stored_token_with_group_bits_is_refused_at_launch"
        status: pass
      - kind: e2e
        ref: "tests/launcher_auth.rs#an_empty_stored_token_is_refused_at_launch"
        status: pass
      - kind: e2e
        ref: "tests/launcher_auth.rs#without_a_stored_token_the_host_access_token_is_passed_and_never_the_refresh_token"
        status: pass
    human_judgment: false
  - id: D4
    description: "driverctl stop-guard blocks a premature end of the top-level driver's turn while LOG.md shows an unreported dispatch, at most 3 times per outstanding dispatch, resets per dispatch, clears when nothing is outstanding, never lets a forged counter extend the budget, and always exits 0"
    requirement: LNCH-03
    verification:
      - kind: e2e
        ref: "tests/driverctl_stop_guard.rs#budget_is_three_blocks_then_the_limit_for_one_dispatch"
        status: pass
      - kind: e2e
        ref: "tests/driverctl_stop_guard.rs#a_new_dispatch_resets_the_budget_and_logs_the_reset"
        status: pass
      - kind: e2e
        ref: "tests/driverctl_stop_guard.rs#a_forged_counter_can_never_extend_the_budget"
        status: pass
      - kind: e2e
        ref: "tests/driverctl_stop_guard.rs#the_hook_exits_zero_on_every_path_and_never_writes_under_the_run_dir"
        status: pass
      - kind: e2e
        ref: "tests/driverctl_stop_guard.rs#the_r261_l8_run_replays_to_the_decisions_the_spike_guard_recorded"
        status: pass
      - kind: unit
        ref: "src/driverctl/stop_guard.rs (12 unit tests)"
        status: pass
    human_judgment: false
  - id: D5
    description: "The seeded settings.json carries exactly two driverctl Stop entries after the user's, the heartbeat then driverctl stop-guard --run-dir '<quoted run dir>', the guard not fail-closed; seeding twice does not stack them"
    requirement: LNCH-03
    verification:
      - kind: unit
        ref: "tests/launcher_config_dir.rs#seed_adds_the_stop_guard_as_the_last_stop_entry_naming_the_quoted_run_dir"
        status: pass
      - kind: unit
        ref: "tests/launcher_config_dir.rs#settings_transform_keeps_user_hooks_in_order_and_drops_the_auth_keys"
        status: pass
    human_judgment: false
  - id: D6
    description: "driver auth setup against the real claude setup-token (browser login, the real printed token's shape) and the guard under a real headless claude driver"
    requirement: LNCH-05
    verification: []
    human_judgment: true
    rationale: "Every test here uses a fake claude on PATH; the real setup-token needs an interactive login and the real Stop hook needs a live driver. Neither was run (no spend, no human); the live L8 stop-guard case is the 28-09 harness."

duration: 18min
completed: 2026-10-19
status: complete
---

# Phase 28 Plan 06: Auth Onboarding and the Stop Guard Summary

**`driver auth setup|status` walks the operator through `claude setup-token` and stores the pasted long-lived token 0600 (refusing empty, multi-line, whitespace, control-character, short and long pastes, and never printing it), and the spike-003 Stop guard is now `driverctl stop-guard`, seeded as the last `Stop` hook entry: it keeps a driver alive up to three times per unreported dispatch using the same `outstanding_dispatch` the exit classifier uses.**

## Performance

- **Duration:** 18 min (about 20:30Z to 20:48Z)
- **Tasks:** 2 (a tracer and a `tdd="true"` task), 3 commits (tracer `feat`, RED `test`, GREEN `feat`)
- **Files modified:** 13 (4 created, 9 modified); +1,152 / -22 lines

## Accomplishments

- `auth.rs` write path: `validate_pasted_token` (trim, one line, printable ASCII, no inner whitespace or control character, 20..=512), `store_token` (sibling temp file created 0600, fsync, rename; 0700 dir when created; the value is never logged), `auth_status` (stored path and mode, why a launch would refuse it, host expiry from a type that deserializes `expiresAt` alone). The 28-01 read-path signatures are untouched.
- `driver auth setup`: explains why (a copied host token is revoked by the host's own refresh, spike 002 A2.4), runs `claude setup-token` with inherited stdio (`process::run_interactive`), reads one bounded line, refuses a multi-line paste by a non-blocking peek at stdin, validates, stores, prints `stored at <path> (0600)`. A failing or missing `claude`, or a refused paste, writes nothing and exits 1. A token not shaped like `sk-ant-oat01-...` is stored with a stderr warning.
- `driver auth status`: presence, path, mode, host expiry in RFC 3339 and minutes, and which source the next launch would use. No value is ever formatted (the status type has none to format).
- `driverctl stop-guard --run-dir <rd>` (hidden): a line-for-line port of `stop_guard.py::guard` over `parse_log` and the shared `outstanding_dispatch`. Per-dispatch budget (`GUARD_LIMIT = 3`), `reset from=<old|-> to=<new|->` lines, the spike's block reason verbatim, `stop-guard.count` written atomically and read as untrusted (IN-01), `stop-guard.log` appended. Exits 0 on every path, panics included.
- `config_dir::settings_transform` takes `stop_guard_command` and appends it as the last `Stop` entry (after the user's and the heartbeat), not fail-closed; `seed` builds it from `SeedSpec.run_dir`, single-quoted.

## Task Commits

1. **Task 1 (tracer): `driver auth setup|status` and the stored-token precedence tests** - `627dfae` (feat)
2. **Task 2 (tdd): the Stop guard**
   - RED `1a1b0eb` (test)
   - GREEN `a6eb87a` (feat)

**Plan metadata:** the docs commit carrying this file.

## TDD Gate Compliance

RED (`test(28-06)` `1a1b0eb`) precedes GREEN (`feat(28-06)` `a6eb87a`) for Task 2.

- **RED:** the commit carries the scaffold that lets the tests compile (the `StopGuard` clap variant and its `driverctl_cli` arm, `pub mod stop_guard`, the `settings_transform` parameter, which RED ignores) and a `decide` stub that always returns `allow reason=not-implemented`. 8 of 9 tests in `driverctl_stop_guard`, 11 of 12 unit tests in `stop_guard.rs` and 3 of 9 in `launcher_config_dir` failed on the planned assertions: no block on the first Stop (`budget_...` failed at `assert_eq!(kinds, [true, true, true, false, false])`), the r261-l8 replay returned `allow reason=not-implemented` where the spike recorded `block item=I-01 attempt=1`, the seeded `Stop` list had 2 entries where 3 were planned, and `settings_transform` produced no guard entry. Two tests (`nothing_outstanding_...`, `a_new_dispatch_resets_...`) panicked at their first read of the counter file, which exists only once a block is recorded: that is the planned behavior missing, not a fixture or compile fault. The ones that passed at RED characterise behavior the scaffold already had (`the_hook_exits_zero_on_every_path...`, `it_never_writes_under_the_run_dir`). semanticAssessment: each failing test executed and failed on the planned assertion or on the absence of the planned artifact; none failed on a compile, load or fixture fault.
- `gsd_run check tdd-red-evidence` accepts TAP / JUnit / swift-testing / unittest / go output only and `cargo test` output is none of those, so the machine classifier was not run (as in 28-03..28-05); the assessment above is by hand. `workflow.tdd_mode` is not set. No REFACTOR commit.
- Task 1 is a tracer, not `tdd="true"`: one `feat` commit with its tests. Its `<verify>` was re-run after Task 2 (`launcher_auth` 11, `--lib launcher::auth` 22), so the tracer gate (auto/end-of-phase) passed before the phase moved on.

## Verification

- Full `cargo test --no-fail-fast`: **3454 passed, 1 failed, 22 ignored** (3453 + the one lib test fixed after the full run, re-run with `--lib`: 2375 passed). The one failure is the expected local `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`.
- Plan verify set, all green: `launcher_auth` 11, `driverctl_stop_guard` 9, `--lib launcher::auth` 22, `--lib driverctl::stop_guard` 12, `launcher_config_dir` 9, `launcher_e2e` 17, `spawn_seam_guard` 41, `async_blocking_guard` 9, `driverctl_client` 19.
- `cargo clippy --all-targets -- -D warnings`: clean. `cargo fmt --check`: clean.
- Acceptance greps: `pub fn store_token` 1 and `pub fn validate_pasted_token` 1 in `auth.rs`; `setup-token` 6 in `driver_cli.rs`; `pub mod stop_guard` 1 in `driverctl/mod.rs` (baseline 0); `StopGuard` 3 in `cli.rs` (baseline 0); `stop-guard` 4 in `config_dir.rs`.
- **Mutation checks (not committed).** Removing the multi-line peek made `setup_refuses_an_unusable_paste_and_stores_nothing` fail (the first line of a two-line paste was stored). `GUARD_LIMIT = 4` failed 2 of 9 `driverctl_stop_guard` tests; dropping the per-dispatch reset failed 3 of 9.
- The tests also pass under both a 002 and a 077 umask (a temp-dir mode assumption in one unit test was removed).

## Files Created/Modified

- `src/driverctl/stop_guard.rs` - the guard, `read_count` / `write_count` / `append_log`, 12 unit tests, numbered facts header citing D-17, T-28-16
- `src/launcher/auth.rs` - write path, `AuthStatus`, 13 new unit tests
- `src/launcher/process.rs` - `run_interactive`
- `src/launcher/config_dir.rs` - the `stop_guard_command` parameter and the last-`Stop`-entry rule
- `src/cli.rs` - `DriverAction::Auth`, `AuthAction`, hidden `DriverctlAction::StopGuard`, parse tests
- `src/driver_cli.rs` - `auth_setup`, `auth_status_main`, `more_input_pending`
- `src/driverctl_cli.rs` - `stop_guard_main` (bounded stdin, panic-catching, exits 0)
- `src/driverctl/mod.rs` - `pub mod stop_guard` and fact 10
- `tests/launcher_auth.rs`, `tests/driverctl_stop_guard.rs` - through the real binary
- `tests/launcher_config_dir.rs` - goldens for the guard entry; `tests/launcher_support/mod.rs` - `hermetic_command` made `pub`
- `tests/fixtures/driverctl/spike003/r261-l8/stop-guard.log` - the spike guard's recorded decisions, replayed

## Decisions Made

All `[inferred - audit]` unless the plan stated them; see `key-decisions`. In addition:

- The block output is compact JSON (`{"decision":"block","reason":"..."}`), where the spike's `json.dumps` put a space after each separator; Claude parses both.
- `append_log` writes the reset lines and the decision line in one `write`, so concurrent Stop hooks cannot interleave inside one decision.
- The stop-guard stdin is read up to 1 MiB and the counter file up to 4 KiB; `LOG.md` up to 16 MiB.

## Deviations from Plan

**1. [Rule 2 - Missing critical] A multi-line paste is refused at the CLI, not only in the validator**
- **Found during:** Task 1
- **Issue:** `read_line` takes only the first line, so a pasted two-line value would have stored its first line and reported success; `validate_pasted_token` alone cannot see the second line.
- **Fix:** after the first line, `more_input_pending` flips stdin non-blocking for one `fill_buf`, treats any non-whitespace as a second line, and restores the flags on every path.
- **Files modified:** `src/driver_cli.rs`; **Verification:** `setup_refuses_an_unusable_paste_and_stores_nothing` (the two-line case), mutation-checked. **Committed in:** `627dfae`.

**2. [Rule 1 - Bug avoided] No process-wide `umask(0o177)` in `store_token`**
- The plan asked for the write under `rustix::process::umask(0o177)`. `umask` is process-global and the unit tests run on many threads, so a concurrent file creation would have been affected. The temp file is instead created with mode 0600 at `open(2)` time and pinned with `fchmod`, which gives the same property (never more permissive, even for an instant). Test: `a_stored_token_is_0600_...`. **Committed in:** `627dfae`.

**3. [Rule 1 - Bug avoided] An existing token directory is tightened, not refused**
- The plan said to check owner and mode "like serve.rs", which refuses any group or other access. The default directory (`~/.config/gsd-meta-manager`) already holds `config.json` and is 0775 under a 0002 umask (Debian, Ubuntu), so refusing would have broken `driver auth setup` for those users. An owned directory loses its group/other write bits and nothing else; one this user does not own is `AuthError::Unsafe`. A directory setup creates is 0700. Tests: `a_directory_others_can_write_to_loses_those_bits_and_nothing_else`, `an_existing_0755_directory_is_accepted_...`. **Committed in:** `627dfae`.

**4. [Rule 2 - Missing critical] A stop-guard counter that cannot be written fails open**
- The spike printed nothing when the counter write raised. Blocking without a persisted count would have no budget, so the Rust guard allows with `reason=counter-unwritable` and says so in `stop-guard.log`. Test: `an_unwritable_counter_fails_open_rather_than_blocking_without_a_budget`. **Committed in:** `a6eb87a`.

**5. [Plan clarification] "nothing outstanding (run1 LOG)"**
- `run1/LOG.md` ends with `RUN | end`, so the guard's own decision order (run-end before outstanding) answers `allow reason=run-end` for it whole. The test asserts that, and asserts `no-outstanding` on the same LOG cut before the `RUN | end` line. No behavior change; the plan line described the LOG's dispatches, not the whole file.

**6. [Rule 3 - Blocking] The in-crate unit test `launcher::config_dir::tests::seed_is_idempotent_and_does_not_stack_hooks` asserted one `Stop` entry**
- Found by the full-suite run after GREEN; updated to the two entries the plan specifies and to assert the guard's command. **Committed in:** `a6eb87a`.

**7. Files outside the plan's list:** `tests/launcher_support/mod.rs` (one word: `hermetic_command` is now `pub`) and `tests/fixtures/driverctl/spike003/r261-l8/stop-guard.log` (a copy of the spike's recorded decisions, so the replay does not read `.planning/spikes`).

**Total deviations:** 3 auto-fixed (2 Rule 2, 1 Rule 3), 2 plan-instruction changes made to avoid a Rule 1 bug (2 and 3), 1 clarification. **Impact:** none on planned behavior; deviations 1, 3 and 4 are user-facing hardening or usability the plan did not spell out.

## Issues Encountered

None beyond deviation 6.

## Authentication Gates

None at execution time. `driver auth setup` is itself the auth gate for users (it runs the real `claude setup-token`); it was not run for real here.

## Pending live

- **`driver auth setup` against the real `claude setup-token`** needs an interactive browser login and a human to paste: not run (no spend, no human). Command: `gsd-meta-manager driver auth setup`, then `gsd-meta-manager driver auth status`. What is unproven is only the real token's printed shape (the `sk-ant-oat01-` prefix check is advisory, so a different shape still stores).
- **The Stop guard under a real headless driver** is the spike L8 case, owned by the 28-09 harness. Here it is proven against the committed r261-l8 LOG prefixes and the real binary.

## Known Stubs

None.

## Threat Flags

None. The surfaces are the plan's T-28-07 (value never in stdout/stderr/status/LOG/state.json/settings.json, tested; 0600 file, atomic write) and T-28-16 (IN-01 clamp, tested with 12 forged shapes). The hook adds no network or auth path.

## Handoffs

- 28-07/28-08: nothing new to call. `AuthStatus` is available if the TUI wants a one-line token badge; it carries no secret.
- 28-09: the guard's decision grammar for pass rules is `stop-guard.log` lines `block item=<id> attempt=<n>`, `allow reason=<background|not-top-level|bad-input|run-end|no-outstanding|no-config-dir|limit|counter-unwritable>`, `reset from=<key|-> to=<key|->`, in `<CLAUDE_CONFIG_DIR>/stop-guard.log` (the per-run config dir).
- Phase 29: the prompt may rely on a premature Stop being blocked at most 3 times per dispatch; it must still tell the worker-wait protocol (`SendMessage` to the same agent), which the block reason repeats.

## Self-Check: PASSED

Created files exist (`src/driverctl/stop_guard.rs`, `tests/launcher_auth.rs`, `tests/driverctl_stop_guard.rs`, `tests/fixtures/driverctl/spike003/r261-l8/stop-guard.log`); commits `627dfae`, `1a1b0eb`, `a6eb87a` are ancestors of HEAD; the plan verify commands exit 0.
