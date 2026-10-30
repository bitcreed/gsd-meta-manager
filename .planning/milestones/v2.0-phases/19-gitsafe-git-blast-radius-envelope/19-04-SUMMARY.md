---
phase: 19-gitsafe-git-blast-radius-envelope
plan: 04
subsystem: infra
tags: [git, credentials, ssh-agent, git-askpass, GIT_CONFIG_GLOBAL, scoped-token, safety-envelope]

# Dependency graph
requires:
  - phase: 19-gitsafe-git-blast-radius-envelope
    provides: "19-01's `envelope::cred::hooks_path_env`, `envelope_dir_in`, `hooks::install_in` and the `GSD_MM_ENVELOPE_ROOT` override; 19-02's `config::CredentialSource` and `policy::ParkReason::CredentialUnavailable`; 19-03's `journal::redact`"
  - phase: 17-driver-supervisor
    provides: "`executor::claude`'s CLAUDE* scrub — the argument this plan extends from the agent's variable family to git's and ssh's — and `tests/spawn_seam_guard.rs`"
  - phase: 14-driver-preview
    provides: "`state_reader::git_ops::git_read_raw`, the `--no-optional-locks` read shape reused for the identity and remote-URL lookups"
provides:
  - "`envelope::cred::EnvelopeEnv` — the child environment as a value, with REMOVE and SET distinguished in the type"
  - "`envelope::cred::build_env` / `build_env_in` — the whole D-16 scrub-and-rebuild, unit-assertable with no process"
  - "`envelope::cred::write_gitconfig` / `write_gitconfig_in` — an identity-only generated config that names no credential helper"
  - "`envelope::cred::write_askpass_stub_in` — the `<envelope>/<alias>/askpass` re-entry D-17 names"
  - "`envelope::cred::resolve_credential` — env var or argv command, with no ambient branch by construction"
  - "`envelope::cred::askpass` / `askpass_with_config` / `askpass_into` — the responder, scoped to the configured remote's host"
  - "`cli::EnvelopeAction::Askpass` and its `main.rs` arm"
  - "`hooks::hooks_dir_in` — one definition of where an alias's hooks live"
  - "`tests/common/` — the `file://` bare-remote harness, now shared by `envelope_tracer` and `envelope_credential`"
affects: [19-05 PreToolUse guard, 19-06 wiring, 19-07 honesty text, 19-08 gate]

# Actuals (#2632)
actuals:
  tokens: 34600
  tasks: 3
  commits: 4

# Tech tracking
tech-stack:
  added: []
  patterns:
    - "Planted-control assertions: before asserting the envelope makes something unreachable, plant the thing in a fake HOME and prove plain git DOES reach it. An emptied HOME resolves no credential helper either, so without the control the refusal passes against an envelope that does nothing."
    - "Reported skips: a fixture that degrades to a no-op prints `SKIPPED <name>` rather than reporting a green test that asserted nothing — a silent skip in a security suite is the vacuous pass the phase exists to prevent."
    - "Baked-at-run-start scoping: the configured remote host is resolved ONCE and written into the generated responder, never re-derived inside it, because re-derivation reads a value the driven agent can change."

key-files:
  created:
    - tests/envelope_credential.rs
    - tests/common/mod.rs
  modified:
    - src/envelope/cred.rs
    - src/envelope/hooks.rs
    - src/cli.rs
    - src/main.rs
    - tests/envelope_tracer.rs
    - tests/spawn_seam_guard.rs

key-decisions:
  - "`GIT_ASKPASS` names a GENERATED STUB at `<envelope>/<alias>/askpass`, not a multi-word command string. D-17 names that path; the plan's task text said `current_exe()` with the action inline, which would have depended on git choosing to route a space-bearing GIT_ASKPASS through a shell. A stub is the same re-entry technique the hooks use and does not rest on that behaviour."
  - "`EnvelopeAction::Askpass` carries `--host`, which the plan's two-field sketch did not. The host is resolved once at run start from `remote.origin.url` and baked into the stub. A responder that re-derived it would read agent-controlled state — the exact attack D-17 names."
  - "The host check runs BEFORE the credential is resolved, so a prompt naming an unknown remote never triggers a credential lookup at all."
  - "A `Username for …` prompt is answered with the non-secret placeholder `x-access-token` rather than the token, so the secret crosses exactly one prompt."
  - "An absent `user.name`/`user.email` falls back to an RFC 2606 `.invalid` address rather than failing the run: refusals in this module are reserved for what SAFE-05 is about, and a cosmetic config gap is not it."
  - "The identity and remote-URL lookups reuse `git_ops::git_read_raw` rather than growing a second `--no-optional-locks` read; `cred.rs` earns its SPAWN_ALLOWLIST entry only for `resolve_credential`'s configured command."
  - "`hooks::sh_quote` became `pub(super)` rather than being copied — a second copy of the `'\\''` escaping is a second place to get it subtly wrong, in files that run on every push."

patterns-established:
  - "REMOVE vs SET in the type: `Vec<(OsString, Option<OsString>)>` conscripts every caller into matching on the Option and calling `env_remove` or `env`, so D-16's removal cannot silently collapse into an empty assignment."
  - "`GIT_CONFIG_COUNT` derived from the pairs by `config_env`, never written by hand — a count that disagrees with the keys makes git drop the whole injection, silently."
  - "Honest-limit paragraphs at the function, not omitted: `write_askpass_stub_in` states outright that an agent can execute the responder and read the token, and why no envelope can close that."

requirements-completed: [SAFE-05]

coverage:
  - id: D1
    description: "`SSH_AUTH_SOCK` and `SSH_AGENT_PID` are removed rather than overwritten, so no ambient agent and no forwarded keys are reachable"
    requirement: SAFE-05
    verification:
      - kind: unit
        ref: "src/envelope/cred.rs#the_ambient_ssh_agent_is_removed_rather_than_overwritten"
        status: pass
      - kind: integration
        ref: "tests/envelope_credential.rs#the_agent_is_removed_and_every_config_scope_is_redirected_into_the_envelope"
        status: pass
    human_judgment: false
  - id: D2
    description: "`GIT_SSH_COMMAND` reads no `~/.ssh/config`, consults no agent, offers no default identity file and cannot prompt"
    requirement: SAFE-05
    verification:
      - kind: unit
        ref: "src/envelope/cred.rs#the_ssh_command_offers_no_identity_no_agent_and_no_prompt"
        status: pass
    human_judgment: false
  - id: D3
    description: "A credential helper the user really has stops being resolvable under the envelope — `GIT_CONFIG_GLOBAL`/`GIT_CONFIG_SYSTEM` point at a generated config carrying only an identity"
    requirement: SAFE-05
    verification:
      - kind: integration
        ref: "tests/envelope_credential.rs#a_credential_helper_the_user_really_has_stops_being_resolvable_under_the_envelope"
        status: pass
      - kind: unit
        ref: "src/envelope/cred.rs#the_generated_config_carries_an_identity_and_no_credential_helper"
        status: pass
      - kind: other
        ref: "mutation: replacing GIT_CONFIG_GLOBAL/SYSTEM with inert names turns the integration test red with `the envelope still resolves a credential helper (\"store\\n\")` (executed, then reverted)"
        status: pass
    human_judgment: false
  - id: D4
    description: "`GIT_TERMINAL_PROMPT=0` turns a credential failure into an immediate error rather than a block on a detached run's null stdio"
    requirement: SAFE-05
    verification:
      - kind: unit
        ref: "src/envelope/cred.rs#terminal_prompts_are_disabled_because_a_detached_run_has_no_terminal"
        status: pass
      - kind: e2e
        ref: "tests/envelope_credential.rs#an_unconfigured_credential_fails_closed_promptly_instead_of_hanging (asserts `terminal prompts disabled` in stderr and a 30s termination budget)"
        status: pass
    human_judgment: false
  - id: D5
    description: "`GH_CONFIG_DIR` points into the envelope directory, so the external GitHub client cannot read the user's own host configuration"
    requirement: SAFE-05
    verification:
      - kind: unit
        ref: "src/envelope/cred.rs#the_config_and_gh_directories_point_inside_this_aliass_envelope"
        status: pass
    human_judgment: false
  - id: D6
    description: "`GSD_MM_ENVELOPE_PROJECT_ROOT` carries the canonicalized project root, so a hook or guard re-entry can locate the run journal it has to park"
    requirement: SAFE-05
    verification:
      - kind: unit
        ref: "src/envelope/cred.rs#the_run_journal_locator_carries_the_canonicalized_project_root"
        status: pass
    human_judgment: false
  - id: D7
    description: "The token reaches git only through the askpass responder's stdout — never an argv element, never URL userinfo, never a file under the envelope"
    requirement: SAFE-05
    verification:
      - kind: unit
        ref: "src/envelope/cred.rs#no_file_under_the_envelope_directory_holds_the_credential"
        status: pass
      - kind: integration
        ref: "tests/envelope_credential.rs#nothing_the_envelope_wrote_holds_the_credential"
        status: pass
      - kind: other
        ref: "grep -nE 'https?://[^/]*:[^@]*@' src/envelope/cred.rs returns no match — no URL-userinfo construction exists"
        status: pass
      - kind: other
        ref: "grep -nE 'tracing::(info|debug|warn|error)!\\(.*(token|credential|secret)' src/envelope/cred.rs returns no match"
        status: pass
    human_judgment: false
  - id: D8
    description: "The responder emits a credential only for the configured remote's host; an agent that adds a second remote gets an authentication failure rather than the token"
    requirement: SAFE-05
    verification:
      - kind: unit
        ref: "src/envelope/cred.rs#a_second_remote_gets_an_authentication_failure_rather_than_the_token"
        status: pass
      - kind: e2e
        ref: "tests/envelope_credential.rs#the_responder_answers_the_configured_host_and_refuses_every_other_one (drives the real binary via --config)"
        status: pass
      - kind: unit
        ref: "src/envelope/cred.rs#an_unparseable_prompt_and_an_unconfigured_host_are_both_refusals"
        status: pass
    human_judgment: false
  - id: D9
    description: "With no credential configured every push fails closed with a legible reason and there is no ambient fallback; `resolve_credential` has no ambient branch by construction"
    requirement: SAFE-05
    verification:
      - kind: unit
        ref: "src/envelope/cred.rs#an_unconfigured_credential_fails_closed_with_a_named_reason"
        status: pass
      - kind: unit
        ref: "src/envelope/cred.rs#an_unset_variable_and_a_failing_command_are_unavailability_not_an_error"
        status: pass
      - kind: e2e
        ref: "tests/envelope_credential.rs#an_unconfigured_credential_fails_closed_promptly_instead_of_hanging (asserts `credential_unavailable` reaches stderr through the real responder)"
        status: pass
    human_judgment: false
  - id: D10
    description: "A `file://` push inside the reserved namespace succeeds with `HOME` pointed at an empty directory — the envelope scoped the credential without breaking pushing"
    requirement: SAFE-05
    verification:
      - kind: e2e
        ref: "tests/envelope_credential.rs#a_push_inside_the_reserved_namespace_succeeds_with_home_emptied (asserts exit zero plus the ref in `git ls-remote`)"
        status: pass
    human_judgment: false
  - id: D11
    description: "The transparency posture: the module states what this layer cannot do — an agent that unsets the variables escapes it, and an agent that runs the responder itself obtains the token"
    verification:
      - kind: other
        ref: "src/envelope/cred.rs module doc `## The honest limit` and `write_askpass_stub_in`'s limit paragraph"
        status: pass
    human_judgment: true
    rationale: "A grep proves the paragraphs exist. Whether they read as an honest account of the residual exposure — particularly the admission that any credential a driven run can push with is one the run can read — rather than as a hedge, is a judgement only a reader can make. SAFE-01's transparency prohibition uses `judgment` for exactly this."

# Metrics
duration: 42 min
completed: 2026-08-18
status: complete
---

# Phase 19 Plan 04: The Scoped Credential Summary

**A driven run's `git push` inside the reserved namespace succeeds with `HOME` pointed at an empty directory, while a `credential.helper` the user demonstrably has becomes unresolvable to that same child — and the token reaches git through one askpass stdout scoped to the configured remote's host, with an unconfigured run failing closed on `credential_unavailable` in under a second rather than hanging.**

## Performance

- **Duration:** ~42 min
- **Started:** 2026-08-18 (base commit `5e1170b`)
- **Completed:** 2026-08-18
- **Tasks:** 3 (Task 1 executed as RED → GREEN)
- **Files modified:** 8 (2 created, 6 modified)

## Accomplishments

- **SAFE-05's silent failure mode is closed by construction, and the proof is not an environment assertion.** The plan's own warning was that an envelope which merely declines to *use* the user's keychain passes every environment check. So the load-bearing test **plants** a `credential.helper = store` in a fake `HOME`, proves plain git resolves it, and only then asserts the envelope does not. Mutating `build_env` to drop the `GIT_CONFIG_GLOBAL`/`GIT_CONFIG_SYSTEM` redirect turns that test red with `the envelope still resolves a credential helper ("store\n")` — executed and reverted before the commit.
- **Pushing still works, proved against a real remote with `HOME` emptied.** A `file://` push inside `refs/heads/gsd-auto/<alias>/` exits zero and the ref appears in `git ls-remote`. This is D-32's pairing: three of this phase's five requirements are refusals, and an envelope that refuses everything would otherwise pass every criterion.
- **The fail-closed path is asserted as a whole chain, not as a non-zero exit.** Against a loopback listener that answers `401` and nothing else, the real responder emits `credential_unavailable`, git falls back to a terminal prompt, and `GIT_TERMINAL_PROMPT=0` turns that into `fatal: could not read Username … terminal prompts disabled`. Both strings are asserted, inside a 30-second budget — because the hang this prevents does not look like a failure, it looks like a run still going until the idle cap kills it hours later.
- **The token is scoped by a host that the driven agent cannot move.** It is resolved once at run start from `remote.origin.url` and baked into the generated `<envelope>/<alias>/askpass` stub. The host check also runs *before* the credential is resolved, so a prompt naming an unknown remote never triggers a lookup. Proved end to end through the real binary, not only in the library.
- **`resolve_credential` has no ambient branch at all**, and its absence is the requirement rather than an omission. An unset variable, an empty value, an unrunnable command and a non-zero exit are all `Ok(None)` — unavailability is an operational state, not a bug.
- **The `file://` harness is now one fixture, not two.** `tests/common/` is shared by `envelope_tracer` and `envelope_credential`; all six tracer tests pass on it unchanged.

## Task Commits

1. **Task 1 (RED): the child environment as failing assertions against an empty envelope** — `2d0e36e` (test)
2. **Task 1 (GREEN): `build_env` — the ambient git identity made unreachable, as a value** — `83608b0` (feat)
3. **Task 2: the token reaches git through one stdout and no other channel** — `87d5164` (feat)
4. **Task 3: environment assertions AND a real push with `HOME` emptied** — `e1ff952` (test)

_No REFACTOR commit: the GREEN implementation needed no cleanup pass, and an empty `refactor` commit is ceremony rather than a change._

## Files Created/Modified

- `src/envelope/cred.rs` — `EnvelopeEnv`/`EnvelopeVar`, `config_env`, `build_env`/`build_env_in`, `write_gitconfig`/`write_gitconfig_in`, `write_askpass_stub_in`, `resolve_credential`, `askpass`/`askpass_with_config`/`askpass_into`, `askpass_reply`, `url_host`, `prompt_host`, `resolve_identity`, `ENVELOPE_SSH_COMMAND`, `PROJECT_ROOT_ENV`; 20 unit tests
- `src/envelope/hooks.rs` — `hooks_dir_in` as the one definition of the hooks path; `sh_quote` promoted to `pub(super)`
- `src/cli.rs` — `EnvelopeAction::Askpass { alias, host, prompt }` with the `--host`-is-baked rationale at the variant
- `src/main.rs` — the askpass arm, before `tui::init()`, redacting its diagnostic and failing closed
- `tests/envelope_credential.rs` — 6 integration tests: environment shape, the planted-helper control, the real push, fail-closed timing, end-to-end host scoping, and the at-rest secret walk
- `tests/common/mod.rs` — the extracted `file://` bare-remote harness
- `tests/envelope_tracer.rs` — now drives the shared harness; its own copy removed
- `tests/spawn_seam_guard.rs` — `src/envelope/cred.rs` earns an allowlist entry for the configured credential command

## Decisions Made

- **`GIT_ASKPASS` names a generated stub, not a command string.** The plan's Task 2 text said to set it to "the absolute `current_exe()` path with the askpass action and the alias", which is a value containing spaces. Whether git accepts that depends on it routing `GIT_ASKPASS` through a shell — a behaviour no test in this plan would have exercised, because the `file://` push needs no credential at all. D-17 in `19-CONTEXT.md` names `GIT_ASKPASS=<envelope>/askpass` outright, so the locked decision and the robust implementation agree; the stub is the same three-line `exec` shape the hooks already use.
- **The responder is told its host rather than finding it.** This is the security content of the `--host` flag. `alias` alone would have forced the responder to read `remote.origin.url` at prompt time — inside the repository the driven agent controls — so an agent adding a second remote would have moved the host the responder answers for. Resolving once at run start and baking it in makes that attack yield an authentication failure instead.
- **A `Username` prompt gets `x-access-token`, not the token.** Token-authenticated HTTPS ignores the username, so emitting the secret there would spend it twice for nothing.
- **An absent user identity falls back rather than refusing.** `user.name` unset means git will not commit at all, so the generated config supplies `gsd-meta-manager driven run` and an RFC 2606 `.invalid` address. Failing the run would turn a cosmetic configuration gap into a refusal, and refusals here are reserved for what SAFE-05 is actually about.
- **The honest limit is stated at `write_askpass_stub_in`, not omitted.** An agent inside the driven run can execute the responder itself and read the token off its stdout. No envelope can close that: any credential a run can push with is one the run can read. What D-17 buys is that the token is not at rest, not in the process table and not in `.git/config`. The answer to the agent that wants it is the credential's own scope and D-27's server-side protection.
- **The tracer feedback gate was satisfied mechanically, as in 19-01 and 19-02.** `19-CONTEXT.md` records the user direction governing this phase — no questions at any gate, best well-reasoned choice, proceed and record it — and this plan carries no `type="tracer"` task in any case.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 2 - Missing Critical] `EnvelopeAction::Askpass` carries `--host`; the plan specified two fields**

- **Found during:** Task 2
- **Issue:** The plan's variant is `Askpass { alias, prompt }` while `askpass()` takes `configured_remote_host` as a third argument, so the host had to come from somewhere. The only source reachable from `alias` alone is the driven repository's own `remote.origin.url`, read at prompt time — which is a value the driven agent can change. An agent that added a second remote would have moved the host the responder answers for and been handed the token for it, which is precisely the attack D-17 names the scoping to prevent.
- **Fix:** The host is resolved once in `build_env_in` from `remote.origin.url`, baked into the generated stub, and passed back on `--host`. The variant's `//` comment states that this is a security property rather than a convenience.
- **Files modified:** `src/cli.rs`, `src/envelope/cred.rs`, `src/main.rs`
- **Verification:** `a_second_remote_gets_an_authentication_failure_rather_than_the_token` and the end-to-end `the_responder_answers_the_configured_host_and_refuses_every_other_one`
- **Committed in:** `87d5164`

**2. [Rule 3 - Blocking] `GIT_ASKPASS` points at a generated stub rather than a multi-word command string**

- **Found during:** Task 2
- **Issue:** Setting `GIT_ASKPASS` to `<binary> envelope askpass <alias> …` relies on git deciding to route a space-bearing value through `sh -c`. If it does not, git tries to `execvp` the whole string as one program name and every authenticated push fails for a reason unrelated to policy — and no test in this plan would have caught it, because the `file://` push needs no credential.
- **Fix:** `write_askpass_stub_in` generates `<envelope>/<alias>/askpass`, the path D-17 names, using the same `exec` shape and the same `sh_quote` as the hook stubs. `GIT_ASKPASS` is then a plain program path.
- **Files modified:** `src/envelope/cred.rs`, `src/envelope/hooks.rs` (`sh_quote` → `pub(super)`)
- **Verification:** `the_askpass_pointer_is_set_and_no_url_userinfo_is_ever_constructed` asserts the pointer names an existing file inside the alias's envelope; the fail-closed integration test shows git actually executing it (`unable to read askpass response from …/askpass` in git's own stderr).
- **Committed in:** `87d5164`

**3. [Rule 3 - Blocking] `build_env_in` gained a `binary` parameter; `hooks::hooks_dir_in` was added**

- **Found during:** Tasks 1 and 2
- **Issue:** `build_env` needs the hooks directory for the `core.hooksPath` triplet and the binary path for the askpass stub. `HOOKS_SUBDIR` was private to `hooks.rs`, and under `cargo test` `current_exe()` is the test binary — the same trap 19-01 documented, which would have made the fixture generate a stub that execs something with no `envelope` subcommand.
- **Fix:** `hooks::hooks_dir_in(root, alias)` becomes the single definition of the hooks path (`install_in` and `assert_provenance_in` now read it too), and `build_env_in` takes the binary explicitly, matching `install`/`install_in`'s established shape.
- **Files modified:** `src/envelope/cred.rs`, `src/envelope/hooks.rs`
- **Verification:** all six `envelope_tracer` tests still pass unchanged; `envelope_credential` names `CARGO_BIN_EXE_gsd-meta-manager`
- **Committed in:** `83608b0`, `87d5164`

**4. [Rule 2 - Missing Critical] The credential-helper assertion is a planted control, not a bare absence check**

- **Found during:** Task 3
- **Issue:** The plan's criterion is that `git config --get credential.helper` exits non-zero under the built environment with `HOME` emptied. Taken literally that assertion passes against an envelope that does **nothing at all** — an emptied `HOME` resolves no helper either. This is precisely the vacuous pass the plan's own critical-anti-pattern note warns about.
- **Fix:** The test writes `[credential] helper = store` into the fake `HOME`, asserts a control run *does* resolve it, and only then asserts the envelope run does not.
- **Files modified:** `tests/envelope_credential.rs`
- **Verification:** mutation — replacing `GIT_CONFIG_GLOBAL`/`GIT_CONFIG_SYSTEM` with inert names turns the test red naming the resolved `"store\n"`. Executed, then reverted.
- **Committed in:** `e1ff952`

**5. [Rule 2 - Missing Critical] Fixture skips are reported instead of silent**

- **Found during:** Task 3
- **Issue:** The harness returns `None` when a sandbox forbids `git init` or a loopback bind, and the tests then return early — reporting green while asserting nothing. In a suite whose entire subject is "the credential is unreachable", a silent skip is indistinguishable from a pass.
- **Fix:** `skip_unless` prints `SKIPPED <name>: … nothing below was asserted`. Verified with `rtk proxy cargo test -- --nocapture` that no test skipped in this environment.
- **Files modified:** `tests/envelope_credential.rs`
- **Verification:** raw run shows all six tests executing, including git's own askpass and terminal-prompt stderr.
- **Committed in:** `e1ff952`

---

**Total deviations:** 5 (3 missing-critical/security, 2 blocking)
**Impact on plan:** No scope creep. Deviations 1, 2 and 4 each close a way this plan could have shipped a control that passed its own criteria without working — an agent-movable host, an askpass git might never execute, and an absence assertion that proves nothing. Deviation 3 is mechanical. Deviation 5 changed no production code.

## Issues Encountered

None in this plan's own scope. Every verification passed on first execution after the deliberate RED and the deliberate mutation.

## Verification Results

| Check | Result |
|---|---|
| `cargo build` | exit 0 |
| `cargo test` | **880 passing, 0 failed** on a full green run (baseline 856 after 19-03; +24). See "Pre-existing failure" below. |
| `cargo test --lib envelope::cred` | exit 0 — 20 tests |
| `cargo test --test envelope_credential` | exit 0 — 6 tests, none skipped (verified with `rtk proxy … --nocapture`) |
| `cargo test --test envelope_tracer` | exit 0 — 6 tests, unchanged on the shared harness |
| `cargo test --test spawn_seam_guard` | exit 0 — 7 tests |
| `cargo clippy -- -D warnings` | exit 0 |
| `cargo clippy --all-targets -- -D warnings` | **exactly 5** pre-existing lints — `browser.rs` ×3, `project_creator.rs` ×1, `state_reader/mod.rs` ×1. Count unchanged. Measured with `rtk proxy` per D-34. |
| `cargo clippy --test envelope_credential -- -D warnings` | exit 0 (the all-targets run aborts at the pre-existing lib-test lints, so the new targets were linted directly) |
| `cargo clippy --test envelope_tracer -- -D warnings` | exit 0 |
| `grep -nE 'GIT_ASKPASS' src/envelope/cred.rs` | 7 matches |
| `grep -nE 'https?://[^/]*:[^@]*@' src/envelope/cred.rs` | no match — no URL-userinfo construction exists |
| `grep -nE 'tracing::(info\|debug\|warn\|error)!\(.*(token\|credential\|secret)' src/envelope/cred.rs` | no match |
| `grep -n 'BatchMode=yes'` / `'IdentityAgent=none'` / `'GSD_MM_ENVELOPE_PROJECT_ROOT'` | 3 / 3 / 1 matches |
| `grep -c '^#\[test\]' tests/envelope_credential.rs` | 6 (≥ 4 required) |
| `grep -rn 'network\|https://github.com' tests/envelope_credential.rs` | one match, a header comment reading "no network beyond `127.0.0.1`"; no line performs a request |

**Mutation check (executed, then reverted before the Task 3 commit):**

| Mutation | Expected | Observed |
|---|---|---|
| `build_env_in` no longer sets `GIT_CONFIG_GLOBAL`/`GIT_CONFIG_SYSTEM` | the helper test red | `a_credential_helper_the_user_really_has_stops_being_resolvable_under_the_envelope` failed: `the envelope still resolves a credential helper ("store\n")`, plus the env-shape test |

**RED-step evidence (committed, not reverted):** at `2d0e36e` `build_env_in` returns an empty environment and 8 of 10 `envelope::cred` tests fail. The 2 that pass are the pre-existing `hooks_path_env` rows, which must pass against the stub or they would be proving nothing.

## Pre-existing Failure (out of scope)

`tests/driver_reattach.rs` fails **intermittently** on two tests
(`a_fresh_scan_finds_the_orphaned_run_live_with_its_last_journal_step`,
`a_run_killed_without_an_ending_is_reported_crashed_and_nothing_on_disk_is_repaired`).
Proved pre-existing: the wave base `5e1170b` was extracted with `git archive` into
a clean directory and reproduces the same two failures with this plan's changes
absent entirely. Nothing in 19-04 touches `src/driver/`, and the envelope is not
wired into the driver until 19-06. Logged to
`.planning/phases/19-gitsafe-git-blast-radius-envelope/deferred-items.md` rather
than fixed, per the scope boundary.

## Known Stubs

None. Every symbol this plan introduces is exercised by a passing test.
`build_env`, `write_gitconfig`, `write_askpass_stub_in` and `askpass` (the
ambient-root / default-registry variants) have no production caller yet — plan
19-06 owns the driver wiring — but none is a stub: each is the same code path the
tests exercise through its explicit-argument sibling, differing only in how the
root or the config path is resolved.

## Threat Flags

None. No new network endpoint, no schema change, and **no crate was added to
`Cargo.toml`** (T-19-SC holds). The new `Askpass` CLI surface is an auth path, but
it is the one this plan's `<threat_model>` already registers as T-19-24/T-19-25,
and both are mitigated by committed tests named in the coverage block. T-19-23,
T-19-26, T-19-27 and T-19-28 likewise. T-19-29 (an agent rewriting the generated
config) remains **accepted** and stated: the envelope regenerates the file at each
run start, and the honest answer is D-27's server-side recommendation.

One residual exposure is recorded here because it is not in the plan's register:
**an agent inside the driven run can execute `<envelope>/<alias>/askpass` itself
and read the token off its stdout.** This is inherent — any credential a run can
push with is one the run can read — and it is documented at
`write_askpass_stub_in` rather than left to be discovered. It strengthens rather
than weakens D-27's conclusion.

## User Setup Required

None — no external service configuration required. Note for the eventual user
documentation: a project with no `credential` on its driver opt-in can be driven,
can read and can commit, but cannot push. That is D-18's deliberate default.

## Next Phase Readiness

- **Ready for 19-05 (PreToolUse guard).** `cred::PROJECT_ROOT_ENV` is the locator a guard re-entry reads to find the run journal it has to park.
- **Ready for 19-06 (wiring).** `cred::build_env(alias, project_root) -> EnvelopeEnv` is the run-start call; apply `entries()` in the spawn closure at `src/executor/claude.rs:418-435`, matching on the `Option` (`None` → `env_remove`, `Some` → `env`). It pairs with `hooks::install(alias)`, which must run first so `core.hooksPath` points at a directory that has hooks in it. `build_env` resolves the remote host from `remote.origin.url`, so it should be called after the project is known to have its remote configured.
- **For 19-07 (honesty text).** `src/envelope/cred.rs`'s module doc and `write_askpass_stub_in`'s limit paragraph already carry the substance of D-27's "what is NOT guaranteed" half, including the responder-readable-by-the-agent admission. The pinned constant should agree with them rather than restate them differently.
- **Note for 19-08 (gate):** `SAFE-05` is declared by **19-04 only**, so `requirements.ready-ids` should release it and `REQUIREMENTS.md` can be marked by the orchestrator.
- **One open thread for a later plan:** `askpass_with_config` takes a registry path, but the generated stub does not pass `--config`, so a TUI launched with a non-default `--config` yields a responder that reads the default registry, finds no credential and fails closed. Safe direction, but silent. 19-06 can bake `--config` into the stub in the same commit that threads the config path into the envelope.
- **No blockers.**

## Self-Check: PASSED

- Both created files present on disk (`tests/envelope_credential.rs`, `tests/common/mod.rs`); all six modified files present in the diff against the wave base `5e1170b`.
- All four task commits present in `git log` (`2d0e36e`, `83608b0`, `87d5164`, `e1ff952`).
- Every task `<acceptance_criteria>` re-run and passing (see Verification Results); the plan-level `<verification>` re-run and passing.
- `STATE.md` and `ROADMAP.md` deliberately untouched — parallel worktree mode, the orchestrator owns those writes.

---
*Phase: 19-gitsafe-git-blast-radius-envelope*
*Completed: 2026-08-18*
