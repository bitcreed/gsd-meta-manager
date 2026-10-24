---
phase: 28-sandboxed-driver-launcher
reviewed: 2026-10-23T00:00:00Z
depth: standard
files_reviewed: 36
files_reviewed_list:
  - scripts/launcher-acceptance.sh
  - src/action.rs
  - src/app.rs
  - src/cli.rs
  - src/driver_cli.rs
  - src/driverctl/mod.rs
  - src/driverctl/serve.rs
  - src/driverctl/store.rs
  - src/driverctl_cli.rs
  - src/launcher/auth.rs
  - src/launcher/config_dir.rs
  - src/launcher/gate.rs
  - src/launcher/host.rs
  - src/launcher/mod.rs
  - src/launcher/outcome.rs
  - src/launcher/process.rs
  - src/launcher/run.rs
  - src/launcher/sandbox.rs
  - src/lib.rs
  - src/main.rs
  - src/state_reader/driver_frontmatter.rs
  - src/ui/screens/detail.rs
  - src/ui/screens/driver.rs
  - src/ui/screens/driver_confirm.rs
  - src/ui/screens/mod.rs
  - src/ui/screens/render_escape_guard.rs
  - tests/driverctl_frontmatter.rs
  - tests/driverctl_store.rs
  - tests/launcher_acceptance.rs
  - tests/launcher_caps.rs
  - tests/launcher_config_dir.rs
  - tests/launcher_e2e.rs
  - tests/launcher_live_isolation.rs
  - tests/launcher_sandbox.rs
  - tests/launcher_support/mod.rs
  - tests/spawn_seam_guard.rs
findings:
  critical: 0
  warning: 6
  info: 4
  total: 10
status: issues_found
---

# Phase 28: Code Review Report

**Reviewed:** 2026-10-23
**Depth:** standard
**Files Reviewed:** 36

## Summary

Reviewed the changes since `91d1f08` (`git diff 91d1f08 HEAD`) in the 36 listed files. The files now carry
Phase 29 and 30 work as well (embedded prompt, DIRECTION preflight, recovery and control gating, the Driver
tab). I read the launcher, sandbox, driverctl and CLI changes in full and sampled the TUI files. `cargo check
--all-targets` is clean. The full test suite was not run.

**Prior findings (28-REVIEW.md, 12 items) re-checked against the current source:**

| Prior | Status |
|---|---|
| CR-01 process-wide umask in `bind_private` | Fixed (staging dir). The fix introduces WR-01 below. |
| WR-01 `--new-session` | Fixed (`sandbox.rs:318-321`, golden updated). The live probe for it is weak, see IN-02. |
| WR-02 credential denylist | Partially fixed. Name-pattern env stripping and more masks landed. A whole-`$HOME` read is still open, see WR-06. |
| WR-03 prompt as option / oversized | Fixed (`--` before the prompt, `check_prompt`). |
| WR-04 `RERUN=1` evidence removal | Fixed (`CLEAR_OLD_EVIDENCE` is applied after `host_preconditions`). |
| WR-05 tests need `.planning/` | Fixed (`planning_skip_reason`). |
| WR-06 float flag validation | Fixed (clap `value_parser`s plus `check_limits`). |
| WR-07 one-shot signal handler | Fixed (loop plus `STOP_ESCALATED`). A small semantic wrinkle remains, see IN-03. |
| IN-01, IN-02, IN-03 | Fixed. |
| IN-04 ledger read-modify-append race | Still open (deliberately skipped by the fixer), see IN-01. |

No critical issues. The new findings are concentrated in three places. The CR-01 fix weakened the
"never replace a live socket" guarantee. The new sibling-run hiding in `bwrap_argv` can silently turn itself
off. The macOS enablement is only half applied.

## Warnings

### WR-01: `bind_private` now replaces whatever sits at the target path (stage-then-`rename`), so the "refuse a live socket" rule has a race

**File:** `src/driverctl/serve.rs:556-580` (callers `replace_and_bind` 521-547, `bind_run_socket` 590-594)

**Issue:** Before the CR-01 fix, `UnixListener::bind(path)` was an exclusive create: if anything appeared at
`path` after `existing_socket()` / `socket_is_live()` had looked, bind failed with `EADDRINUSE`. The
replacement does `bind(staged)` then `fs::rename(staged, path)`. `rename(2)` silently replaces an existing
socket, regular file or symlink. A second server (an explicit `--sock` shared between runs, or two runs that
collide on the 32-bit `hash8`; the doc on `replace_and_bind` names both cases) that binds between the
vetting step and the rename is unlinked and replaced. Its listener keeps running but is unreachable, so its
driver's hooks fail. The vetting in `bind_run_socket` is a check-then-act sequence, and the old atomic
bind was what made it safe.

A secondary effect: `check_sun_len(&staged)` measures `<parent>/.bXXXXXX/s`, which is 10 bytes longer than a
short explicit `--sock` name. A final path that fits can be refused because its staging path does not.

**Fix:** publish with `hard_link`, which fails with `EEXIST` instead of replacing. The staging directory's
removal then cleans up the staged name.

```rust
fs::set_permissions(&staged, fs::Permissions::from_mode(0o600))
    .map_err(|e| io_error("chmod the staged socket", e))?;
fs::hard_link(&staged, path)
    .map_err(|e| io_error(&format!("publish the socket at {}", path.display()), e))?;
// `staging` drops here and removes `s` and the directory; the bound inode lives on at `path`.
```

Add a test where a file is created at `path` after vetting and the bind must fail. Also drop the
`check_sun_len(&staged)` call, or make it a warning, so it cannot refuse a final path that fits.

### WR-02: The sibling-run hiding tmpfs is silently skipped when the drivers dir is reached through a symlink

**File:** `src/launcher/sandbox.rs:285-295`

**Issue:** `bwrap_argv` emits `--tmpfs <project>/.planning/meta-manager/drivers` only when
`paths.run_dir.parent() == Some(project.join(DRIVER_RUNS_REL))`. In production `run_dir` comes from
`validate_run_dir`, which returns the CANONICAL run dir. It accepts a drivers dir (or `.planning`) that is a
symlink, because it compares against `canonicalize(project/DRIVERS_REL)`. With `.planning -> docs/planning`,
for example, `run_dir.parent()` is `<project>/docs/planning/meta-manager/drivers`. That is not equal to the
literal `<project>/.planning/meta-manager/drivers`, so the tmpfs is omitted without a message. Sibling runs
then stay visible, and writable if the target is inside the project bind. That is exactly what quick
261020-f80 set out to prevent. The doc comment says "`validate_run_dir` guarantees that in production", which
is not true for symlinked layouts, and the no-tmpfs golden test locks in the silent skip.

**Fix:** hide the directory that actually holds the run, whatever its spelling:

```rust
if let Some(drivers) = paths.run_dir.parent() {
    if drivers.ends_with(DRIVER_RUNS_REL) {
        argv.push(os("--tmpfs"));
        argv.push(drivers.as_os_str().to_owned());
    }
}
```

The `no_drivers_tmpfs_unless...` test needs a symlinked-planning case that expects the tmpfs. If the skip is
kept on purpose, fail the launch (or log a visible warning) when the run dir is not under the drivers dir.

### WR-03: Run-store and CLI reads of `DIRECTION.md` are unbounded and blocking, unlike the TUI reader hardened for the same threat

**File:** `src/driverctl/store.rs:644-661` (`direction_read`), `src/launcher/run.rs:229-231`
(`preflight_direction`), `src/driver_cli.rs` (`check_main`)

**Issue:** `state_reader::driver_frontmatter::open_run_file` was added (WR-02 / T-30-04) because "a run
directory is writable by sibling sandboxes and by `--unsandboxed` runs", so `DIRECTION.md` may be a FIFO or
symlink. It opens with `O_NOFOLLOW | O_NONBLOCK`, type-checks the handle and caps the size at 1 MiB. The
launcher side does not use it. `RunStore::direction_read` calls `fs::read`, and `preflight_direction` and
`check_main` call `fs::read_to_string`. These are plain blocking opens with no size limit.

- A FIFO at `DIRECTION.md` blocks `fs::read` forever. `direction_read` runs inside the store critical section
  (every `log` request and every gate hook). The tick thread needs the same mutex, so cap monitoring stops
  too, and the launcher wedges. `driver launch` and `driver check` hang on the same FIFO.
- An unsandboxed driver can grow the file without limit, and the whole file is read into memory on each hook.
- A symlink is followed, so the gate's pause/stop decision can be read from another file.

**Fix:** route all three through `open_run_file` plus a bounded `take(DIRECTION_READ_MAX_BYTES + 1)` read. A
non-regular file or an oversized file becomes `Err(kind)`, which the gate already treats as "control
unknown, deny".

### WR-04: The D-17 recovery block cannot be satisfied when DIRECTION has no parsed items

**File:** `src/driverctl/store.rs:1000-1006` (`blocking_recovery`), `src/driverctl/protocol.rs:397-406`
(`known_item`)

**Issue:** `blocking_recovery` treats an awaiting id as blocking when `direction.items.is_empty() ||
direction.items.iter().any(..)`. The doc comment above it says an id the human removed from DIRECTION "cannot
be recovered ... so it no longer blocks". The empty case does the opposite. With no parsed items (the file is
absent, mid-edit, or its `### I-NN` headers no longer parse), every awaiting id blocks. The gate then denies
every dispatch with "log recovered for it first", while `validate_log("recovered", ...)` rejects any item
with "is not an item in DIRECTION.md". The driver is told to do something the protocol will refuse, and the
run is stuck until a human repairs DIRECTION.md.

**Fix:** drop the `direction.items.is_empty() ||` clause so the behaviour matches the comment. If it was added
for a test fixture, change the fixture.

```rust
.find(|id| direction.items.iter().any(|i| &i.id == *id))
```

### WR-05: macOS enablement is half applied (TUI still gates on Linux and shows a contradictory message)

**File:** `src/app.rs:1796-1803`, `3104-3113` (`target_os = "linux"` arms), `src/ui/screens/driver_steer.rs:39-45`
(`can_steer`), `src/driverctl/store.rs:429-433`, `src/driverctl/serve.rs:1592-1601`

**Issue:** Phase 28 and 29 widened the launcher, the run store, `serve` and `direction.rs` to
`any(linux, macos)` and changed `LauncherError::UnsupportedPlatform` to read "the launcher needs Linux or
macOS". The TUI paths still use `#[cfg(target_os = "linux")]` and reuse that same error. On macOS, pressing
`x`, launching or steering refuses with "unsupported platform (the launcher needs Linux or macOS) Nothing was
started." That is a self-contradicting message on a platform the CLI supports. `can_steer()` is documented
as "the write primitive exists on Linux and nowhere else", which is no longer true now that `direction.rs`
has macOS arms. Two Linux assumptions also leak into code compiled on macOS. `host_name()` reads
`/proc/sys/kernel/hostname` and returns `None` on macOS, so the connection record loses its host. The new
test `cr01_bind_private_never_touches_the_process_umask_and_binds_concurrently` reads `/proc/self/status` and
panics on macOS, so a macOS test run fails.

**Fix:** widen the TUI cfg arms and `can_steer` to `any(linux, macos)`, or use a TUI-specific refusal string
that does not say "or macOS". Use `gethostname`/`rustix::system::uname()` in `host_name`. Gate the umask test
with `#[cfg(target_os = "linux")]`, or read the umask with a portable probe.

### WR-06: Credential hiding is still a denylist; `$HOME` is otherwise readable while the network is open (prior WR-02, reduced)

**File:** `src/launcher/sandbox.rs:69-173` (`is_stripped_env`, `TMPFS_MASK_DIRS`, `NULL_MASK_FILES`)

**Issue:** The pattern-based env scrub and the longer mask lists are a real improvement. The model still runs
with `bypassPermissions` on `--ro-bind / /` with open network. Anything under `$HOME` that nobody thought of
stays readable and exfiltratable: browser profiles and cookies, password-manager stores, `~/.config/pip`,
`~/.config/gsd-meta-manager/config.json` (the registry), `~/.terraform.d` credentials outside the masked dir,
`/etc` secrets, and `HTTPS_PROXY=http://user:pass@...` (not name-matched). The fixer's note records that the
allowlist was not adopted. That is a reasoned call, but the D-27 claim ("no credential reachable") is only as
strong as the list.

**Fix:** state the residual risk in the sandbox module doc and the README, and consider a read allowlist
mount model for `$HOME` (bind only the project, the config dir and the toolchain homes, and tmpfs the rest of
`$HOME`). At minimum, scrub `*_PROXY` values that contain `@`.

## Info

### IN-01: Campaign ledger check-and-append race is still open (prior IN-04)

**File:** `scripts/launcher-acceptance.sh:105-125`, `743-755`

**Issue:** `ceiling_ok` and `record_costs` still run without a lock. The script header now says the harness is
kept as a historical record and a helper library for `prompt-acceptance.sh`, so the exposure is lower.

**Fix:** wrap the check and append in `flock "$LEDGER.lock"` if the harness is run for real again.

### IN-02: The `tty_open` probe cannot fail without a controlling terminal

**File:** `tests/launcher_live_isolation.rs:82` (`tty_open` line) and the `tty_open == "NONE"` assertion

**Issue:** `(: < /dev/tty)` fails with `ENXIO` whenever the process has no controlling terminal. That is the
normal state under CI, a pipe or a harness. The assertion then passes even if `--new-session` is removed. The
test only proves WR-01 when run from an interactive terminal.

**Fix:** have the test run the sandbox under a pseudo-terminal (a `script -qc` or `openpty` wrapper), or skip
with a printed reason when stdin is not a tty.

### IN-03: Signal escalation counts "stop already set", not "second signal"

**File:** `src/driverctl/serve.rs:608-612` (`note_stop_signal`), `STOP_ESCALATED` (601)

**Issue:** `stop.swap(true)` returning `true` marks the escalation. If `stop` is set by anything other than the
first signal, the first SIGTERM escalates immediately. Today only the signal path sets the launcher's
`user_stop`, so it is latent. `STOP_ESCALATED` is a process global that is never reset, which would also
affect a future caller that runs more than one launch in a process.

**Fix:** keep a separate signal counter (`fetch_add`) and escalate on `>= 2`. Add a `reset` or per-launch flag
if the launcher is ever embedded.

### IN-04: Doc drift and small leftovers

**File:** `src/launcher/mod.rs:28-32`, `src/driverctl/store.rs:1018-1024`, `scripts/launcher-acceptance.sh:174`

**Issue:** `mod.rs` still says `sandbox` "is the one submodule that is not gated to Linux and macOS", but
`pub mod prompt;` is now ungated too. The doc comment on `recovery_rejection` repeats the "An awaiting id the
human has since removed ..." paragraph that is already on `blocking_recovery`. The `mode_config` header comment
lists `WRITES WRITE_S` but omits `WRITE_FIRST`, and the writer is called with `WRITE_S` before `WRITE_FIRST`
(correct for `gap-writer.sh <project> <count> <interval> [first]`, but easy to misread).

**Fix:** adjust the comments.

---

_Reviewed: 2026-10-23_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
