---
phase: 28-sandboxed-driver-launcher
fixed_at: 2026-10-19T00:00:00Z
review_path: .planning/phases/28-sandboxed-driver-launcher/28-REVIEW.md
iteration: 1
findings_in_scope: 12
fixed: 11
skipped: 1
status: partial
---

# Phase 28: Code Review Fix Report

**Fixed at:** 2026-10-19
**Source review:** .planning/phases/28-sandboxed-driver-launcher/28-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 12 (fix_scope: all)
- Fixed: 11
- Skipped: 1

**Verification ran in the main checkout** (no worktree, per operator guidance), branch `master`:
`TMPDIR=/tmp/claude-1000/t cargo test --no-fail-fast` gave 3528 passed, 1 failed (the expected
`envelope::policy` git-version constants test, pinned to CI's git); `cargo clippy --all-targets -- -D warnings`
is clean. The `#[ignore]` live bwrap isolation test was NOT run, so the new `tty_open`, `npmrc_read`,
`env_aws_access_key_id` and related probe lines are unmeasured on this host.

## Fixed Issues

### CR-01: `bind_private` changes the PROCESS-WIDE umask; concurrent callers can leave it stuck at 0o177

**Files modified:** `src/driverctl/serve.rs`
**Commit:** 0951aea
**Applied fix:** The umask call is gone. The socket is bound inside a private 0700 staging directory
(`tempfile::Builder` in the target's parent), chmod-ed 0600, then renamed into place. Added
`cr01_bind_private_never_touches_the_process_umask_and_binds_concurrently` (8 threads x 10 binds, asserts
umask unchanged, mode 0600, no leftover staging dirs). Logic change: requires human verification.

### WR-01: bwrap is started without `--new-session` (controlling-terminal injection)

**Files modified:** `src/launcher/sandbox.rs`, `tests/launcher_live_isolation.rs`
**Commit:** a9fa45a (the `tests/launcher_sandbox.rs` golden was updated in 60f634e)
**Applied fix:** `--new-session` added before `--die-with-parent` (`--die-with-parent` was already
present); golden order updated; live probe gained a `tty_open` line asserted `NONE`.

### WR-02: Credential hiding is a denylist and is incomplete (env and home files), while the network is open

**Files modified:** `src/launcher/sandbox.rs`, `src/launcher/config_dir.rs`, `tests/launcher_live_isolation.rs`
**Commit:** 862847a
**Applied fix:** `[inferred - audit]` kept a denylist (an allowlist would have to guess the driver's long tail
of needed variables) but made it pattern-based: new `is_stripped_env` strips the exact `STRIP_ENV` lists, a new
`MORE_STRIP_ENV` (GIT_ASKPASS, SSH_ASKPASS, GIT_SSH(_COMMAND), KUBECONFIG, DOCKER_CONFIG, ...), the prefixes
`AWS_ AZURE_ ARM_ GOOGLE_ GCP_ GCLOUD_ CLOUDSDK_ GH_ GITHUB_ GITLAB_ GLAB_ CI_JOB_`, and the suffixes
`_TOKEN _SECRET _PASSWORD _API_KEY _ACCESS_KEY _CREDENTIALS ...`. The launcher's own values are applied after
the filter, so `CLAUDE_CODE_OAUTH_TOKEN` still reaches the driver. The settings `env` block uses the same
predicate. Mask lists extended: dirs `.kube .azure .config/gcloud .config/glab-cli .local/share/keyrings
.terraform.d`; files `.npmrc .pypirc .cargo/credentials(.toml) .vault-token` and the stored run token
(`~/.config/gsd-meta-manager/driver-oauth-token`, plus `auth::token_path()` when `XDG_CONFIG_HOME` moves it).
Not done: `GIT_CONFIG_*` (the envelope phase relies on it), `awsAuthRefresh`-style settings keys (covered only
as far as their `env` entries), and a blanket mask of `~/.config/gsd-meta-manager` (the registry may be needed
by the in-sandbox shim; only the token file is masked). Live probe extended with these paths (unrun).

### WR-03: The master prompt is passed as a positional that can be parsed as an option, and as a single argv string

**Files modified:** `src/launcher/sandbox.rs`, `src/launcher/run.rs`, `tests/launcher_support/mod.rs`, `tests/launcher_e2e.rs`, `tests/launcher_sandbox.rs`
**Commit:** 60f634e
**Applied fix:** `claude_argv` puts the prompt last after `--`. `[inferred - audit]` that the pinned claude
(commander) honours `--`; not tested against the real binary. `check_prompt` refuses an empty template and a
rendered prompt over 120 KiB in both `launch` and `launch_detached`, before the lock. The fake claude shim now
reads the argument after `--`. New e2e tests cover a `---`-leading template and the empty/oversized refusals.

**Real-CLI audit (2026-10-19) — VERIFIED, the inferred item is closed.** Host `claude --version` =
`2.1.304 (Claude Code)`. Run from a neutral scratch dir with the launcher's exact `claude_argv` shape, only
model and budget swapped:
`claude -p --setting-sources user --model haiku --output-format stream-json --verbose --permission-mode bypassPermissions --max-budget-usd 0.03 -- "$P" </dev/null`

| Prompt `$P` | exit | `result` event |
|---|---|---|
| `reply with the word OK` | 0 | `{"subtype":"success","is_error":false,"result":"OK","total_cost_usd":0.0047}` |
| `---\ntitle: x\n---\nreply with the word DASHOK` | 0 | `{"subtype":"success","is_error":false,"result":"DASHOK","total_cost_usd":0.0047}` |

Control (same dash-leading prompt as a positional right after `-p`, no `--`): exit 1,
`error: unknown option '---\ntitle: x\n---\nreply with the word DASHOK'`, no API call. So `--` is honoured,
the argument after it is the prompt, and it is required for front-matter templates. Spend ≈ $0.0094. No code
change needed. (The `[inferred — audit]` note on `claude_argv` in `src/launcher/sandbox.rs` is now settled by
this entry.)

### WR-04: `RERUN=1` deletes the committed evidence BEFORE any gate that can refuse the rerun

**Files modified:** `scripts/launcher-acceptance.sh`
**Commit:** 112f55a
**Applied fix:** The `rm -f` now runs after `host_preconditions` (step 6), driven by a `CLEAR_OLD_EVIDENCE`
flag set in the evidence guard. The fresh-directory-and-swap variant was not done (larger change).

### WR-05: Tests (and the harness selftest) require `.planning/` content that the crate excludes

**Files modified:** `tests/launcher_acceptance.rs`
**Commit:** f9adaa3
**Applied fix:** `planning_skip_reason()` (phase dir and both spike dirs) feeds `skip_reason()` and the ledger
test, which now print `SKIP:` instead of panicking in a packaged crate or `gsd-pr-branch` checkout.

### WR-06: Float and budget flags are unvalidated; `inf`, `NaN` and negative values reach systemd and `claude`

**Files modified:** `src/cli.rs`, `src/launcher/run.rs`
**Commit:** 8b3eb4c
**Applied fix:** clap `value_parser`s require finite, > 0 and bounded (`MAX_BUDGET_USD` 100000, `MAX_CAP_MINUTES`
525600) for `--max-budget-usd` and `--cap-minutes`; `check_limits` repeats the check in `launch` and
`launch_detached`. Test added.

### WR-07: `install_signal_handlers` is one-shot; a second SIGTERM/SIGINT is swallowed for the rest of the process

**Files modified:** `src/driverctl/serve.rs`, `src/launcher/run.rs`
**Commit:** 53c59f5
**Applied fix:** The signal task loops for the life of the process. The first signal sets `stop`; a second sets
a process-wide `STOP_ESCALATED`, which makes `Containment::stop` kill the child at once (no wait for the
`systemctl stop` helper) and `stop_group` skip its grace period. The recorded stop is still written. Unit test
covers the counting; no real signals are sent in tests. Logic change: requires human verification.

### IN-01: `DRIVER_LOCK_HELD=1` is exported to the driver but nothing in `src/` reads it

**Files modified:** `src/launcher/run.rs`
**Commit:** 44f1c39
**Applied fix:** Comment records that the spike master prompts consume it.

### IN-02: Module documentation drift in `process.rs` and `auth.rs`

**Files modified:** `src/launcher/process.rs`, `src/launcher/auth.rs`, `src/driverctl/serve.rs`
**Commit:** 3279b91
**Applied fix:** "Three shapes" -> "Five shapes"; the `store_token` paragraph re-flowed; the stale
`replace_and_bind` doc that described the umask swap corrected.

### IN-03: Harness-tests pass vacuously when their tools are missing

**Files modified:** `tests/launcher_acceptance.rs`
**Commit:** 313f054
**Applied fix:** A missing `bash`/`python3`/`git` now panics when `CI` is set; elsewhere it still skips.

## Skipped Issues

### IN-04: The campaign ledger is read-modify-append without a lock

**File:** `scripts/launcher-acceptance.sh:105-125, 743-755`
**Reason:** Not trivial: a correct fix needs a reservation-line protocol around the paid launch plus a
`flock`, which changes the ledger format that the selftest and the committed-ledger test parse. Single-operator
use makes the race unlikely. Left for a deliberate change.
**Original issue:** `ceiling_ok` sums the ledger and `record_costs` appends later without a lock, so two
concurrent `run` invocations can each pass the $20 gate.

---

_Fixed: 2026-10-19_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
