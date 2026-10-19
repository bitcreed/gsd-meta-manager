---
phase: 28-sandboxed-driver-launcher
plan: 03
subsystem: launcher
tags: [launcher, bwrap, sandbox, isolation, claude-config, settings, rust]

requires:
  - phase: 28-sandboxed-driver-launcher
    provides: "28-01: sandbox.rs / config_dir.rs / auth.rs builders and the launch path; 28-02: live-scope test template"
provides:
  - "hardened bwrap profile: $XDG_RUNTIME_DIR tmpfs with only the run's driverctl socket re-bound; settings.json and bin/ read-only inside the writable config dir; git credential masks"
  - "sandbox::{TMPFS_MASK_DIRS, NULL_MASK_FILES, EXTRA_STRIP_ENV}, SandboxSpec.runtime_dir, --setting-sources user, GIT_TERMINAL_PROMPT=0, ANTHROPIC_AUTH_TOKEN stripped"
  - "config_dir::{settings_transform, DROPPED_SETTINGS_KEYS}: pure per-run settings transform"
  - "tests/launcher_sandbox.rs, tests/launcher_config_dir.rs (CI goldens) and tests/launcher_live_isolation.rs (#[ignore] live SC3 / D-27 probe)"
affects: [28-04, 28-05, 28-06, 28-09]

plan_head_before: 7a7cbea787bb35efba8ed0c54a43abecb03eef79
plan_head_after: d0f3b8060ba3ae8d879ae3cbc5b4e23c18d73879
commits: 4
actuals:
  tokens: 13900
  tasks: 3
  commits: 4

tech-stack:
  added: []
  patterns:
    - "Pure argv/env/settings builders pinned by goldens that run without bwrap or systemd"
    - "Live isolation probe prints KEY=VALUE lines and pairs every negative with a host control"

key-files:
  created:
    - tests/launcher_sandbox.rs
    - tests/launcher_config_dir.rs
    - tests/launcher_live_isolation.rs
  modified:
    - src/launcher/sandbox.rs
    - src/launcher/config_dir.rs

key-decisions:
  - "Socket is re-bound only when it lies under the runtime dir; a socket elsewhere stays reachable through the read-only root [inferred - audit]"
  - "settings.json and bin/ read-only inside the writable config dir; seed rewrites them on the host before each launch, outside the sandbox [inferred - audit, plan-stated]"
  - ".config/git/credentials and .netrc masked in addition to .git-credentials [inferred - audit, plan-stated]"
  - "settings env block scrubbed of STRIP_ENV and ANTHROPIC_AUTH_TOKEN keys [inferred - audit, plan-stated]"
  - "Live probe uses GIT_SSH_COMMAND with accept-new / BatchMode / UserKnownHostsFile=/dev/null so an inside ssh failure means no credential, not an unknown host [inferred - audit]"

requirements-completed: [LNCH-04, LNCH-05, LNCH-06]

coverage:
  - id: D1
    description: "The bwrap profile is pinned by a golden: runtime-dir tmpfs with the socket re-bound, conditional masks including git credential files, read-only settings.json and bin/, fixed mount order; masks omitted when absent"
    requirement: LNCH-04
    verification:
      - kind: unit
        ref: "tests/launcher_sandbox.rs#golden_bwrap_argv"
        status: pass
      - kind: unit
        ref: "tests/launcher_sandbox.rs#masks_are_omitted_when_absent"
        status: pass
    human_judgment: false
  - id: D2
    description: "Env scrub strips the spike list plus ANTHROPIC_AUTH_TOKEN and sets GIT_TERMINAL_PROMPT=0; claude runs with --setting-sources user"
    requirement: LNCH-05
    verification:
      - kind: unit
        ref: "tests/launcher_sandbox.rs#golden_child_env"
        status: pass
      - kind: unit
        ref: "tests/launcher_sandbox.rs#claude_argv_restricts_setting_sources"
        status: pass
    human_judgment: false
  - id: D3
    description: "settings_transform keeps GSD and rtk hooks in order with the fail-closed gate first, drops apiKeyHelper and the UI keys, scrubs the settings env block, yields only the five driverctl entries for an empty host"
    requirement: LNCH-04
    verification:
      - kind: unit
        ref: "tests/launcher_config_dir.rs#settings_transform_keeps_user_hooks_in_order_and_drops_the_auth_keys"
        status: pass
      - kind: unit
        ref: "tests/launcher_config_dir.rs#settings_transform_of_nothing_is_exactly_the_five_driverctl_entries"
        status: pass
    human_judgment: false
  - id: D4
    description: "seed links only existing host entries, never the global CLAUDE.md, never merges or overwrites an existing entry, and leaves the host ~/.claude byte-identical; two projects with one run id get different state dirs"
    requirement: LNCH-04
    verification:
      - kind: unit
        ref: "tests/launcher_config_dir.rs#seed_links_only_existing_host_entries_and_never_claude_md"
        status: pass
      - kind: unit
        ref: "tests/launcher_config_dir.rs#seed_leaves_the_host_claude_dir_byte_identical"
        status: pass
      - kind: unit
        ref: "tests/launcher_config_dir.rs#run_state_dirs_of_two_projects_with_one_run_id_differ"
        status: pass
    human_judgment: false
  - id: D5
    description: "No refresh-token material reaches the sandbox: the access token appears only as CLAUDE_CODE_OAUTH_TOKEN, no env value or per-run config file holds either token's sentinel"
    requirement: LNCH-05
    verification:
      - kind: unit
        ref: "tests/launcher_config_dir.rs#no_refresh_token_material_reaches_the_sandbox"
        status: pass
    human_judgment: false
  - id: D6
    description: "Live on this host: ~/.claude read-only, credential files unreadable, ~/.ssh empty, secret env vars unset, run dir read-only with a 0-byte lock, config dir writable except settings.json and bin/, runtime dir holds only gsdmm-driverctl, no bus, no user manager, driverctl socket connects"
    requirement: LNCH-04
    verification:
      - kind: integration
        ref: "tests/launcher_live_isolation.rs#sandbox_isolates_host_credentials_and_services"
        status: pass
      - kind: integration
        ref: "tests/launcher_sandbox.rs#runtime_dir_holds_only_the_driverctl_socket"
        status: pass
    human_judgment: false
  - id: D7
    description: "A git push --dry-run from inside the sandbox over ssh and over https fails for lack of credentials, while the host control succeeds over ssh"
    requirement: LNCH-06
    verification:
      - kind: integration
        ref: "tests/launcher_live_isolation.rs#sandbox_isolates_host_credentials_and_services"
        status: pass
    human_judgment: false

duration: 11min
completed: 2026-10-19
status: complete
---

# Phase 28 Plan 03: Hardened Sandbox Profile and Per-Run Config Summary

**The driver's bwrap profile now hides the whole `$XDG_RUNTIME_DIR` (SSH agent, D-Bus, user systemd) except the run's own driverctl socket, keeps the gate's `settings.json` and shim read-only, masks every git credential store, and a live probe on this host proves it, including a push from inside failing while the same push from the host succeeds.**

## Performance

- **Duration:** 11 min
- **Started:** 2026-10-19T19:35:30Z
- **Completed:** 2026-10-19T19:46:14Z
- **Tasks:** 3 (a tracer, a `tdd="true"` task, an auto task), 4 commits
- **Files modified:** 5 (3 created, 2 modified)

## Accomplishments

- `bwrap_argv` closes RESEARCH §1: `--tmpfs $XDG_RUNTIME_DIR` then `--bind <socket> <socket>`, after the config dir, so the host agent / bus / user manager are unreachable while `DRIVERCTL_SOCK` still connects.
- `settings.json` and `bin/` are `--ro-bind` over the writable config dir: the driver can no longer rewrite its own gate hooks or the shim they call.
- `claude` runs with `--setting-sources user`; `child_env` strips `ANTHROPIC_AUTH_TOKEN` and always sets `GIT_TERMINAL_PROMPT=0`.
- `settings_transform` (pure `Value -> Value`) drops `apiKeyHelper` as well as the UI keys, scrubs the settings `env` block, puts the fail-closed gate first and the lifecycle hooks last, and keeps the user's GSD and rtk hooks in order. `seed` delegates to it.
- Goldens (CI) pin the argv, the env and the transform; a test proves no refresh-token material reaches the sandbox.

## Task Commits

1. **Task 1 (tracer): hardened profile, golden argv/env, live runtime-dir smoke** - `3d1fa18` (feat)
2. **Task 2 (tdd): per-run config dir**
   - RED `2fa4632` (test)
   - GREEN `b679047` (feat)
3. **Task 3: live SC3 / D-27 isolation probe** - `d0f3b80` (test)

**Plan metadata:** the docs commit carrying this file.

## TDD Gate Compliance

RED (`test(28-03)` `2fa4632`) precedes GREEN (`feat(28-03)` `b679047`). RED committed a `settings_transform` stub returning its input, so the file compiled and 4 of 8 tests failed on the planned assertions: `statusLine must be dropped`, `a hooks object` missing (twice), and `GH_TOKEN` still present in the seeded settings `env`. The other 4 (`seed_links_only_existing_host_entries_and_never_claude_md`, `seed_without_a_host_settings_file_holds_only_the_driverctl_hooks`, `run_state_dirs_of_two_projects_with_one_run_id_differ`, `no_refresh_token_material_reaches_the_sandbox`) passed at RED: they characterise 28-01 behaviour (links, empty host, state-dir hash, access-token-only auth) that this plan was required to pin, not new behaviour. `gsd_run check tdd-red-evidence` accepts TAP / JUnit / swift-testing / unittest / go output only, and `cargo test` output is none of those, so the machine classifier was not run; the semantic assessment is by hand as above (the 4 failures were assertion failures on the planned behaviour, not compile or fixture faults). `workflow.tdd_mode` is not set. No REFACTOR commit. Task 1 is a tracer (not `tdd="true"`): it was committed as one `feat` with its goldens.

## Live evidence (run on this host outside a tool sandbox, no `SKIP:` line)

Command: `cargo test --test launcher_live_isolation -- --ignored --nocapture`, real `bwrap`, `systemd-run --user --scope`, real `HOME`, real `XDG_RUNTIME_DIR`. The four secret env vars were injected into the host env with sentinel values before `child_env`.

```
LIVE host user manager: degraded
LIVE host XDG_RUNTIME_DIR entries: 35
LIVE host ~/.ssh entries: 13
claude_touch=EROFS
creds_read=1
claude_json_read=1
git_credentials_read=absent
netrc_read=absent
ssh_entries=0
gh_entries=0
env_ssh_auth_sock=
env_gh_token=
env_anthropic_api_key=
env_anthropic_auth_token=
env_git_terminal_prompt=0
run_dir_touch=EROFS
lock_bytes=0
lock_read=1
direction_lock_bytes=0
cfg_touch=WRITABLE
settings_touch=EROFS
bin_touch=EROFS
runtime_entries=gsdmm-driverctl
bus=absent
user_manager=Failed to connect to user scope bus via local transport: No such file or directory
sock=present
LIVE host control push --dry-run over ssh:   success=true
LIVE host control push --dry-run over https: success=false (fatal: could not read Username for 'https://github.com': terminal prompts disabled)
LIVE inside push --dry-run over ssh:   success=false (... Please make sure you have the correct access rights / and the repository exists.)
LIVE inside push --dry-run over https: success=false (fatal: could not read Username for 'https://github.com': terminal prompts disabled)
test result: ok. 1 passed
```

Reading the evidence:

- The masked credential files fail to read (`creds_read=1`, `claude_json_read=1`, `lock_read=1`) because `--ro-bind /dev/null` is a `nodev` mount: a device node opened there gives EACCES. The lock files report 0 bytes through `stat` while the host's file holds data (the fixture wrote a non-empty record). `~/.config/gh` exists on the host and lists empty inside. `git_credentials_read` and `netrc_read` are `absent`: those two files do not exist on this host, so those two masks are proven only by the golden argv, not live.
- **Non-vacuity.** The host control succeeds over ssh, so the inside ssh failure is real. **The https control fails on the host too** (no https credential configured here), so the inside https failure is not independently informative on this host; the plan requires at least one scheme, which the ssh control gives.
- **Mutation check (not committed).** With the runtime-dir block removed from `bwrap_argv`, the same live test fails at `runtime_entries` and shows `bus=present` with the host's 35 runtime entries (agent socket `openssh_agent`, `bus`, `systemd` ...). The probe catches the hole it was written for. In that mutated run the inside ssh push still failed (the env `SSH_AUTH_SOCK` strip plus the empty `~/.ssh` independently block it), so the push probe alone would not have caught the runtime-dir hole: `runtime_entries`, `bus` and `user_manager` are the lines that do.
- `~/.claude/sandbox-probe` was not created on the host (checked after the run).
- Also re-run after the profile change: `launcher_live_scope -- --ignored --test-threads=1` (2 passed) and the plan-1/2 CI e2e suites below, so the new mounts do not break the launch path.

## Files Created/Modified

- `src/launcher/sandbox.rs` - `runtime_dir`, `EXTRA_STRIP_ENV`, public mask lists, read-only settings/bin binds, runtime tmpfs + socket bind, `--setting-sources user`, `GIT_TERMINAL_PROMPT=0`
- `src/launcher/config_dir.rs` - `settings_transform`, `DROPPED_SETTINGS_KEYS` (now with `apiKeyHelper`), `seed` delegating, module docs (global `CLAUDE.md` deliberately not linked)
- `tests/launcher_sandbox.rs` - goldens, empty-home and outside-socket edges, opt-in runtime-dir live smoke
- `tests/launcher_config_dir.rs` - transform, links, host byte-identity, empty-host, state-dir and no-refresh-token tests
- `tests/launcher_live_isolation.rs` - the opt-in live SC3 / D-27 probe

## Decisions Made

All `[inferred - audit]`; the plan itself marked the first three:

- Git credential masks `.config/git/credentials` and `.netrc` added; `settings.json` and `bin/` read-only inside; settings `env` block scrubbed (all stated in the plan).
- The socket is re-bound only when it lies under the runtime dir. A socket outside it (not the case for `serve::socket_path`, which always builds it there) is left reachable through the read-only root, because a `--bind` into a read-only root would abort bwrap.
- The live probe pins `GIT_SSH_COMMAND` (`accept-new`, `BatchMode`, `UserKnownHostsFile=/dev/null`) on both the host control and the sandboxed side, so a failure means "no credential" rather than "unknown host" or a prompt. It is a test-only addition; the launcher does not set it.
- The probe reports `lock_bytes` through `stat` (0 for the `/dev/null` mask) and `lock_read` through `cat` (non-zero), rather than `wc -c <`, because the mask is a nodev bind and `cat` fails; the plan's `lock_bytes=0` holds as a size, not as a successful empty read.
- `user_manager` is judged by the `systemctl --user is-system-running` output, not its exit code: on this host the real manager is `degraded` and exits 1, so the exit code alone is vacuous.

## Deviations from Plan

None in behaviour. Two small additions inside the plan's file list: a `a_socket_outside_the_runtime_dir_is_not_re_bound` test, and a `GIT_SSH_COMMAND` pin in the live test (above). One RED-phase stub (`settings_transform` returning its input) so the tests compile and fail on assertions instead of failing to build.

**Total deviations:** 0 auto-fixed. **Impact:** none.

## Issues Encountered

None. The tracer, the transform and the live probe all passed on first run; the one compile error in the RED test file (a `Cow::into_bytes`) was fixed before the RED commit.

## Verification

- `launcher_sandbox`: 5 passed, 1 ignored (the live smoke, run separately: ok, `gsdmm-driverctl` / `SOCK_OK`). `launcher_config_dir`: 8 passed. `--lib launcher::config_dir`: 6 passed; `--lib launcher::sandbox`: 8 passed. `launcher_e2e` 17 and `launcher_stop` 5: pass.
- Full `cargo test --no-fail-fast`: 3312 passed, 1 failed, 22 ignored. The one failure is the expected local `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`.
- `cargo clippy --all-targets -- -D warnings`: clean. `cargo fmt --check`: clean.
- Live: `launcher_live_isolation -- --ignored --nocapture` 1 passed, `launcher_sandbox -- --ignored runtime_dir_holds_only_the_driverctl_socket` 1 passed, `launcher_live_scope -- --ignored --test-threads=1` 2 passed; no `SKIP:` line in any.
- Acceptance greps: `ANTHROPIC_AUTH_TOKEN` 3 and `setting-sources` 3 and `GIT_TERMINAL_PROMPT` 3 in `sandbox.rs`; `pub fn settings_transform` 1 and `apiKeyHelper` 3 in `config_dir.rs`; `fn sandbox_isolates_host_credentials_and_services` 1.

## Pending live

None. To re-run the committed live proofs (outside any tool sandbox): `cargo test --test launcher_live_isolation -- --ignored --nocapture` and `cargo test --test launcher_sandbox -- --ignored --nocapture runtime_dir_holds_only_the_driverctl_socket`.

## Handoffs

- 28-04: `bwrap_argv` is the sandboxed arm; the degraded (unsandboxed) arm has none of these protections by definition and must say so in the posture it records.
- 28-05/09: the profile now includes read-only `settings.json` / `bin/`; anything the launcher needs to change in the config dir must happen on the host before the launch (as `seed` does), not from inside.
- 28-09: the live harness can reuse `launcher_live_isolation.rs`'s fixture (project repo, seeded config dir, listener at `socket_path`, `systemd_run_argv` around `bwrap_argv`).

## Known Stubs

None.

## Threat Flags

None. The surfaces touched are the ones in the plan's threat model (T-28-09 to T-28-13); each mitigation is now tested (goldens, the no-refresh-token test, the live probe).

## Self-Check: PASSED
