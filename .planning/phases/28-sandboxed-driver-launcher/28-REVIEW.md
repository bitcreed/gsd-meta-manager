---
phase: 28-sandboxed-driver-launcher
reviewed: 2026-10-19T00:00:00Z
depth: standard
files_reviewed: 17
files_reviewed_list:
  - scripts/launcher-acceptance.sh
  - src/cli.rs
  - src/driver_cli.rs
  - src/driverctl/serve.rs
  - src/launcher/auth.rs
  - src/launcher/config_dir.rs
  - src/launcher/mod.rs
  - src/launcher/process.rs
  - src/launcher/run.rs
  - src/launcher/sandbox.rs
  - src/lib.rs
  - src/main.rs
  - tests/launcher_acceptance.rs
  - tests/launcher_e2e.rs
  - tests/launcher_live_isolation.rs
  - tests/launcher_support/mod.rs
  - tests/spawn_seam_guard.rs
findings:
  critical: 1
  warning: 7
  info: 4
  total: 12
status: issues_found
---

# Phase 28: Code Review Report

**Reviewed:** 2026-10-19
**Depth:** standard
**Files Reviewed:** 17
**Status:** issues_found

## Summary

The launcher's control flow (lock-first ordering, relaunch under one lock, cap monitor, containment stop) is
carefully built and the pure argv builders are well covered. The defects found are concentrated in three
places: a process-wide `umask` race in `bind_private` that is the confirmed cause of the flaky
`PermissionDenied` bursts, sandbox hardening that rests on incomplete denylists (env, home files, controlling
terminal), and the acceptance harness / its CI tests, which delete committed evidence before their gates and
depend on `.planning/` content that is excluded from the published crate.

## Critical Issues

### CR-01: `bind_private` changes the PROCESS-WIDE umask; concurrent callers can leave it stuck at 0o177

**File:** `src/driverctl/serve.rs:402-413` (callers: `replace_and_bind` 399, `bind_run_socket` 423-427,
tests 1117, 1153)

**Issue:** The orchestrator lead is CONFIRMED.

`umask(2)` is per process, not per thread. `bind_private` does
`old = umask(0o177); bind(); umask(old)`. Two defects follow:

1. **Window.** For the duration of `bind`, every other thread that creates a file or directory gets its mode
   masked by 0o177. `tempfile::TempDir` asks for 0o700; under 0o177 that becomes 0o600, a directory with no
   search bit, so the next `create_dir_all` / `fs::write` inside it fails with `EACCES`
   (`PermissionDenied`). That is exactly the symptom reported.
2. **Permanent corruption (the worse one, and the only thing that explains a burst of 435 failures rather
   than a handful).** Interleave two binders A and B:
   `A: old_A = umask(0o177)` returns 0o022; `B: old_B = umask(0o177)` returns **0o177** (A's value);
   `A: umask(old_A)` restores 0o022; `B: umask(old_B)` sets **0o177 for the rest of the process**.
   From then on every `TempDir`/`create_dir` in the process produces an unusable 0o600 directory.
   In the lib test binary, `wr04_an_aborted_start_records_the_stop_and_unlinks_the_socket` (calls
   `bind_private` directly, line 1117) and `wr05_a_dead_socket_file_is_replaced` (via `replace_and_bind`,
   line 1153) run on parallel test threads and are the two binders. All other `tests/launcher_*.rs`
   binaries exec the real binary as a child, so they are unaffected.

**Production exposure:** latent, not currently triggered. In `serve()` and `launcher::run::Session::start`
the only other live threads at bind time are the idle tokio signal thread
(`install_signal_handlers`, runs before the bind) and possibly a finished/blocked `driverctl-probe` thread;
none creates files. The ticker and accept threads are started after the bind. But `bind_run_socket` is the
documented embed entry (`pub(crate)`, "Phase 28 embed entry"), and any future caller in a multithreaded
process (the TUI tokio runtime, a second concurrent run) inherits both defects, plus a secondary one: the
module comment in `auth.rs:289-295` explicitly rejects `umask(0o177)` because it touches process-wide state,
so the codebase already knows this is wrong.

**Fix:** do not touch the umask. Bind inside a private 0700 staging directory next to the target, chmod,
then rename (a bound unix socket keeps working after `rename`, the listener is bound to the inode):

```rust
fn bind_private(path: &Path) -> Result<UnixListener, DriverctlError> {
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    // 0700 staging dir: nobody else can traverse it, so the socket's creation mode
    // (whatever the process umask is) is unreachable until the chmod below.
    let staging = tempfile::Builder::new()
        .prefix(".b")
        .permissions(fs::Permissions::from_mode(0o700))
        .tempdir_in(parent)
        .map_err(|e| io_error(&format!("stage a bind in {}", parent.display()), e))?;
    let staged = staging.path().join("s");
    check_sun_len(&staged)?;                       // staging path must fit sun_path too
    let listener = UnixListener::bind(&staged)
        .map_err(|e| io_error(&format!("bind {}", staged.display()), e))?;
    fs::set_permissions(&staged, fs::Permissions::from_mode(0o600))
        .map_err(|e| io_error("chmod the staged socket", e))?;
    fs::rename(&staged, path)
        .map_err(|e| io_error(&format!("move the socket to {}", path.display()), e))?;
    listener
        .set_nonblocking(true)
        .map_err(|e| io_error("set the listener non-blocking", e))?;
    Ok(listener)
}
```

`tempfile` is already a regular dependency (`launcher/process.rs`). The default socket directory is already
vetted 0700/owner-only by `socket_path_in`, so a simpler variant (bind, then `chmod 0600`) is also race-free
for the default path; the staging directory is needed to keep an explicit `--sock` in a shared directory
equally safe. Either way, also add a regression test that spawns N threads calling `bind_private` while
another thread asserts `umask` is unchanged.

## Warnings

### WR-01: bwrap is started without `--new-session` (controlling-terminal injection)

**File:** `src/launcher/sandbox.rs:196-200`

**Issue:** `bwrap_argv` passes `--unshare-pid --die-with-parent` but not `--new-session`. In `--foreground`
mode (and in every acceptance-harness `run_launch ... --foreground`) the sandboxed model process inherits the
operator's controlling terminal, and bwrap's `--dev` exposes `/dev/tty`. Without a new session the model
can `ioctl(TIOCSTI)` keystrokes into the operator's shell, which runs them after the launcher exits: a
sandbox escape with `bypassPermissions` on. The bwrap manual calls this out and tells callers to use
`--new-session` (or a seccomp filter) whenever the terminal is shared. On this host
`/proc/sys/dev/tty/legacy_tiocsti` is 0, so it is not exploitable here, but that is a per-distro kernel
setting, not something the launcher may assume. The default detached mode is safe (daemon is a `setsid`
leader), which is why this only shows up in foreground and harness use. `tests/launcher_live_isolation.rs`
never probes `/dev/tty`.

**Fix:** add `argv.push(os("--new-session"));` before `--die-with-parent`, update the golden order in
`bwrap_argv_has_the_fixed_order`, and add a `tty_ioctl=` line to the live probe.

### WR-02: Credential hiding is a denylist and is incomplete (env and home files), while the network is open

**File:** `src/launcher/sandbox.rs:44-78` (`STRIP_ENV`, `TMPFS_MASK_DIRS`, `NULL_MASK_FILES`);
`src/launcher/run.rs:1205-1214` (`child_env(std::env::vars_os(), ...)`)

**Issue:** The child gets the whole host environment minus 15 named variables, on top of `--ro-bind / /`
with only 5 directories and 5 files masked. The sandbox shares the network (the driver needs the API), so
anything readable is exfiltratable. Not hidden today: `AWS_ACCESS_KEY_ID`/`AWS_SECRET_ACCESS_KEY`/
`AWS_SESSION_TOKEN`, `NPM_TOKEN`, `GITLAB_TOKEN`, `GH_ENTERPRISE_TOKEN`, `OPENAI_API_KEY`, `GIT_ASKPASS`,
`SSH_ASKPASS`, `GIT_SSH_COMMAND`, `GIT_CONFIG_*`; files `~/.cargo/credentials.toml` (this project publishes
to crates.io), `~/.npmrc`, `~/.pypirc`, `~/.kube/config`, `~/.config/gcloud`, `~/.config/glab-cli`,
`~/.local/share/keyrings`, and `~/.config/gsd-meta-manager/` (the registry and the stored
`driver-oauth-token`, which is the run token's long-lived form). `apiKeyHelper` is dropped from settings but
`awsAuthRefresh`/`awsCredentialExport`/`otelHeadersHelper` are not. D-27's "a push from inside must find no
credential" only holds for the credential stores someone thought of.

**Fix:** invert the env policy to an allowlist (`PATH`, `HOME`, `LANG`, `LC_*`, `TERM`, `TZ`, `USER`,
`LOGNAME`, `SHELL`, `XDG_*` minus `XDG_RUNTIME_DIR`, `TMPDIR`, plus the launcher's own `sets`); extend the
mask lists with the paths above (keep "only when they exist"); consider masking all of
`~/.config/gsd-meta-manager`. Add the extra paths to the live isolation probe so the claim is measured.

### WR-03: The master prompt is passed as a positional that can be parsed as an option, and as a single argv string

**File:** `src/launcher/sandbox.rs:234-254` (`claude_argv`), `render_prompt` 259-269

**Issue:** `claude -p <prompt> --setting-sources ...`: the prompt comes from a user file. A template whose
first line starts with `-` (a Markdown front-matter fence `---`, a bullet `- ...`) is read by the CLI as an
option and the launch fails after `begin_launch` has already recorded `RUN | launch`, so it surfaces as a
`crashed` run. Separately, Linux caps a single argv string at 128 KiB (`MAX_ARG_STRLEN`); a larger template
makes `execve` fail with `E2BIG` for the same effect, and nothing validates either condition before the
lock is taken.

**Fix:** put the prompt last after a `--` (verify against the pinned claude minimum, 2.1.295), and reject an
oversized template (and an empty one) in `launch`/`launch_detached` before anything is written, next to the
existing `prompt_file` checks. Add a test with a `---`-leading template through the fake `claude`.

### WR-04: `RERUN=1` deletes the committed evidence BEFORE any gate that can refuse the rerun

**File:** `scripts/launcher-acceptance.sh:815-829` versus gates at 836-849 and 849

**Issue:** Step "(1) evidence guard" runs `rm -f "$EVD"/{verdict.txt,LOG.md,state.json,...}` for every
non-d33 case as soon as `RERUN=1` is set. The campaign-ceiling check (exit 8, line 836-840), scratch-root
check (exit 2, 843-844), `cargo build` (847), and `host_preconditions` (exit 5, 849) all come afterwards. A
rerun refused by any of them has already destroyed the committed `verdict.txt` and evidence with nothing
written in its place. The comment on the evidence guard says a re-verification can never spend twice; it does
not say it can destroy the only record.

**Fix:** move the `rm -f` down to just before `mkdir -p "$SCRATCH_ROOT" "$EVD"` (line 856), i.e. after
`host_preconditions` has passed, or better, write the new evidence into a fresh directory and swap it in
only after `evidence_gates` passes.

### WR-05: Tests (and the harness selftest) require `.planning/` content that the crate excludes

**File:** `tests/launcher_acceptance.rs:248-250`, `223`; `scripts/launcher-acceptance.sh:461-464`

**Issue:** `Cargo.toml` has `exclude = [".planning/", ...]`. `the_committed_campaign_ledger_stays_within_the_phase_ceiling`
does `fs::read_to_string(&ledger).expect("acceptance/campaign.txt exists")` and panics without it, and
`the_harness_selftest_passes_offline` fails because `run_selftest` asserts that every case's prompt file under
`.planning/spikes/002-...` and `003-...` exists. Both therefore fail in a packaged crate (`cargo test` from the
tarball, which is what a downstream verifier or packager runs) and on any branch produced by `gsd-pr-branch`,
which filters `.planning/` out. The sibling test `the_committed_acceptance_tree_holds_no_stream_config_or_token`
silently passes in the same situation, so the file is also inconsistent.

**Fix:** skip with a printed reason when `repo().join(PHASE_DIR)` (or the spike directory) is absent, as
`skip_reason()` already does for missing tools, or gate the tests behind an env var the planning checkout
sets. Make the ledger test follow the same rule as the secret-scan test.

### WR-06: Float and budget flags are unvalidated; `inf`, `NaN` and negative values reach systemd and `claude`

**File:** `src/cli.rs:479-486` (`--max-budget-usd`, `--cap-minutes`), `src/launcher/run.rs:831-834`
(`runtime_max_sec`), `src/driver_cli.rs:285-289`

**Issue:** clap's `f64` parser accepts `inf`, `NaN` and (via `=`) negatives. `--cap-minutes inf` is not
exhausted, never trips, and yields `RuntimeMaxSec=18446744073709551615`, which systemd-run rejects, so the
failure appears as a `crashed` driver after `RUN | launch` is logged. `--cap-minutes nan` is refused, but as
"used 0 of its NaN active wall-clock minutes; raise --cap-minutes", which tells the operator nothing.
`--max-budget-usd nan|-1|inf` is handed straight to `claude --max-budget-usd`; the acceptance harness relies
on that flag as its spend bound.

**Fix:** add a clap `value_parser` that requires `is_finite() && > 0` for both flags (and a sane upper bound
for `runtime_max_sec`, since systemd stores microseconds in a `u64`); clamp or refuse in `launch` as defence in
depth.

### WR-07: `install_signal_handlers` is one-shot; a second SIGTERM/SIGINT is swallowed for the rest of the process

**File:** `src/driverctl/serve.rs:431-477` (used by `serve` 747 and `launcher::run::launch` 474)

**Issue:** The signal thread's runtime ends after the first signal sets `stop`, but tokio never uninstalls a
`sigaction` it has installed, so later SIGTERM/SIGINT go to a handler that notifies nobody. After the first
Ctrl-C (or `driver stop`) the launcher can wait up to `USER_STOP_BUDGET` (60 s) plus `CHILD_EXIT_WAIT` inside
`Containment::stop`, and a second Ctrl-C does nothing. The only way out is SIGKILL, which is the path that
leaves a `crashed` run behind.

**Fix:** keep the signal thread alive (loop on `recv()`), count signals, and on the second one escalate
(`child.kill()` / `Containment::stop` fast path, or `std::process::exit(130)` after recording the stop).

## Info

### IN-01: `DRIVER_LOCK_HELD=1` is exported to the driver but nothing in `src/` reads it

**File:** `src/launcher/run.rs:1210`

**Issue:** The variable is set (and asserted in `launcher_e2e.rs:114`) but has no consumer in this crate. If
it is a contract with the spike's master prompt or scripts, say so in a comment; if not, it is dead surface
that a sandboxed process could mistake for a "skip the lock" switch in a future change.

**Fix:** document the consumer next to the `child_env` call or drop it.

### IN-02: Module documentation drift in `process.rs` and `auth.rs`

**File:** `src/launcher/process.rs:11-22`, `src/launcher/auth.rs:289-295`

**Issue:** `process.rs` says "Three shapes" and then lists five; the `store_token` doc comment has a sentence
spliced mid-paragraph (`... [inferred - audit]. The value goes to ...`) and cites `umask(0o177)` as the thing
being avoided while `serve.rs` does exactly that (see CR-01).

**Fix:** correct the count; re-flow the paragraph.

### IN-03: Harness-tests pass vacuously when their tools are missing

**File:** `tests/launcher_acceptance.rs:37-44, 109-112`

**Issue:** Missing `bash`/`python3`/`git` prints `SKIP:` and returns `Ok`, so a CI image without `python3` reports
all of this file green while proving nothing.

**Fix:** mark the tests `#[ignore]` with a reason, or fail when `CI` is set and the tool is missing.

### IN-04: The campaign ledger is read-modify-append without a lock

**File:** `scripts/launcher-acceptance.sh:105-125, 743-755`

**Issue:** `ceiling_ok` sums the ledger and `record_costs` appends later, with no `flock`, so two concurrent
`run` invocations can each pass the $20 gate. Single-operator use makes this unlikely.

**Fix:** wrap check-and-reserve in `flock "$LEDGER.lock"` and append a reservation line before the launch.

---

_Reviewed: 2026-10-19_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
