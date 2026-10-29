---
phase: 28-sandboxed-driver-launcher
reviewed: 2026-10-28T00:00:00Z
depth: standard
files_reviewed: 42
files_reviewed_list:
  - scripts/launcher-acceptance.sh
  - src/action.rs
  - src/app.rs
  - src/bounded_output.rs
  - src/cli.rs
  - src/config.rs
  - src/driver/direction.rs
  - src/driver/kill.rs
  - src/driver/liveness.rs
  - src/driver_cli.rs
  - src/driverctl/serve.rs
  - src/driverctl/store.rs
  - src/driverctl_cli.rs
  - src/launcher/auth.rs
  - src/launcher/config_dir.rs
  - src/launcher/host.rs
  - src/launcher/mod.rs
  - src/launcher/process.rs
  - src/launcher/push.rs
  - src/launcher/run.rs
  - src/launcher/sandbox.rs
  - src/main.rs
  - src/state_reader/driver_frontmatter.rs
  - src/ui/screens/detail.rs
  - src/ui/screens/driver.rs
  - src/ui/screens/driver_confirm.rs
  - src/ui/screens/driver_launch.rs
  - src/ui/screens/mod.rs
  - src/ui/screens/render_escape_guard.rs
  - tests/async_blocking_guard.rs
  - tests/driverctl_frontmatter.rs
  - tests/driverctl_store.rs
  - tests/launcher_auth.rs
  - tests/launcher_caps.rs
  - tests/launcher_config_dir.rs
  - tests/launcher_degraded.rs
  - tests/launcher_e2e.rs
  - tests/launcher_live_isolation.rs
  - tests/launcher_push.rs
  - tests/launcher_sandbox.rs
  - tests/launcher_support/mod.rs
  - tests/spawn_seam_guard.rs
findings:
  critical: 0
  warning: 4
  info: 2
  total: 6
status: issues_found
---

# Phase 28: Code Review Report

**Reviewed:** 2026-10-28
**Depth:** standard
**Files Reviewed:** 42

## Summary

Re-review at HEAD of the Phase 28 files that changed since the prior review at `9252aa5`
`[inferred — cleanup 2026-10-28]`. The scope is the intersection of Phase 28's files with
`git diff 9252aa5 HEAD`. The workflow's degraded fallback offered 112 files, most of them not Phase 28's.
Three bodies of work landed in that range:

- the 2026-10-28 cleanup batch (quick 261028-n0z, n0u, n11, n12),
- Phase 28.1, which added the network, IPC and UTS namespaces, the in-sandbox bridge, the egress proxy, the
  `$HOME` tmpfs allowlist, managed settings and the planted-file guard,
- Phase 31's deletions, which account for most of the diff volume in `app.rs`, `cli.rs`, `action.rs`,
  `kill.rs`, `driver.rs` and `spawn_seam_guard.rs`.

The new files `launcher/home.rs`, `launcher/egress/*`, `launcher/bridge.rs` and `launcher/push/planted.rs` are
not in the requested file list. I read `bridge.rs` and the parts of `home.rs`, `planted.rs` and `egress/mod.rs`
that the listed files call, and did not audit the rest.

Verification run, in a private target directory so the shared tree was untouched:

- `cargo clippy --all-targets` is clean.
- `cargo test --no-fail-fast` over `async_blocking_guard`, `driverctl_frontmatter`, `driverctl_store`,
  `launcher_auth`, `launcher_caps`, `launcher_config_dir`, `launcher_degraded`, `launcher_e2e`, `launcher_push`,
  `launcher_sandbox` and `spawn_seam_guard` passes (255 tests, 0 failed).
- The live bwrap tests are `#[ignore]` and were not run.

No critical issues. Four warnings, two info items. Two of the warnings are prior deferred findings that are
still present at HEAD. Five of the six prior findings the triage marked fixed (WR-01, WR-03, WR-05, IN-02, IN-04) hold at HEAD (see the table).
The sixth, WR-06 (the credential denylist), was fixed by Phase 28.1 and I did not re-derive that fix. The `$HOME` tmpfs,
env routing and `/etc` re-binds are in place, but `home.rs` is outside the requested list. Two prior info
items were triaged as skipped and are unchanged.

**New IDs start at WR-08 and IN-05, not WR-07.** `28-REVIEW-DISPOSITION.md` already holds a WR-07
("`install_signal_handlers` is one-shot", fixed) and a CR-01 ("`bind_private` changes the process-wide umask",
fixed), both from `28-REVIEW-FIX.md`. Reusing WR-07 would drop that recorded decision. `[inferred — cleanup 2026-10-28]`

### Prior findings re-checked at HEAD

| Prior | Disposition | Status at HEAD |
|---|---|---|
| WR-01 `bind_private` rename race | fixed (n12 e98520c) | **Holds.** `serve.rs` now `hard_link`s the staged socket into place, so `EEXIST` replaces the old silent overwrite. Three new tests pin it: a file at the target, a live socket, and mode 0600. `hard_link` of a socket is fine on Linux and APFS, and the `protected_hardlinks` sysctl does not apply because the caller owns the inode. |
| WR-02 drivers tmpfs skipped for a symlinked `.planning` | deferred | **Still present.** Restated below as WR-02. |
| WR-03 unbounded `DIRECTION.md` reads | fixed (n12 dc812c0) | **Holds.** `RunStore::direction_read`, `preflight_direction` and `driver check` all go through `read_run_file_capped`, which uses `open_run_file` plus the 1 MiB cap. Tests cover FIFO, symlink, exactly-at-cap and over-cap. I found no remaining unguarded `DIRECTION.md` reader outside tests. |
| WR-04 D-17 block with no parsed items | deferred | **Still present.** Restated below as WR-04. |
| WR-05 macOS enablement half applied | fixed (n11) | **Holds.** The TUI launch, stop, steer and `can_steer()` arms are `any(linux, macos)`. `host_name()` uses `rustix::system::uname()` and the umask test is Linux-gated. The only `target_os = "linux"` arms left under `app.rs`, `ui`, `driverctl` or `launcher` are `/proc`-reading tests and `LIVENESS_SUPPORTED`. |
| WR-06 credential denylist | fixed (Phase 28.1) | **Not re-derived.** `$HOME` is a size-capped tmpfs with a derived allowlist, `/etc` is rebuilt read-only, `/run` is a tmpfs, and the sandbox has `--unshare-net`. The only way out is the allowlisted egress server. The allowlist builder `home.rs` is outside the requested files. |
| IN-01 ledger race in the acceptance script | skipped | Unchanged (comment-only edit). The skip reason stands. |
| IN-02 `tty_open` probe | fixed (n12 88a61ec) | **Holds.** The assertion runs only with a controlling terminal and prints a SKIP note otherwise. |
| IN-03 signal escalation counts "already set" | skipped | Unchanged. Still latent: `launch_detached` runs the launcher as a separate process, so nothing embeds it. |
| IN-04 doc drift | fixed (n12 d344401) | **Holds** for `launcher/mod.rs` and the `mode_config` header. The `blocking_recovery` doc comment is still wrong, but that belongs to WR-04. |

## Warnings

### WR-02: The sibling-run hiding tmpfs is silently skipped when the drivers dir is reached through a symlink

**Status:** prior, deferred, **still present at HEAD**
(`todos/pending/2026-10-28-sandbox-drivers-tmpfs-skipped-for-symlinked-planning.md`).

**File:** `src/launcher/sandbox.rs:582-587`

**Issue:** `bwrap_argv` still emits `--tmpfs <project>/.planning/meta-manager/drivers` only when
`paths.run_dir.parent() == Some(project.join(DRIVER_RUNS_REL))`. In `launch()` the run dir comes from
`validate_run_dir`, which returns the canonical path, so a symlinked `.planning` or drivers directory never
matches. The golden test `no_drivers_tmpfs_unless_the_run_dir_sits_directly_under_the_projects_drivers_dir`
(`sandbox.rs:1145`) pins the silent skip. The doc comment above the check still says `validate_run_dir`
"guarantees" the shape, which is false for symlinked layouts.

Phase 28.1 raises the cost. Sibling runs that stay visible are not only readable. When the symlink target lies
inside the project bind they are writable, and the launcher reads some of their files with plain blocking
reads before taking the lock (`run.rs:1219`, `fs::read(run_dir.join(STATE_FILE))`). A sandboxed driver could
therefore plant a FIFO or an oversized `state.json` in a sibling run and hang or bloat that run's next launch.
That is the same class as the prior WR-03, now reachable across runs.

The disposition is right that the review's original `ends_with(DRIVER_RUNS_REL)` fix does not match its own
example (`planning` against `.planning`).

**Fix:** carry the canonical drivers directory in `LaunchPaths` and compare against that. The launcher
already knows it, because `validate_run_dir` canonicalizes `project/DRIVERS_REL`.

```rust
// LaunchPaths gains: pub drivers_dir: PathBuf  // canonical parent of run_dir
if paths.run_dir.parent() == Some(paths.drivers_dir.as_path()) { /* emit the tmpfs */ }
```

If the run dir is not directly under it, refuse the launch rather than skipping the hiding. Update the golden
test to expect a symlinked-planning case with the tmpfs.

### WR-04: The D-17 recovery block cannot be satisfied when DIRECTION has no parsed items

**Status:** prior, deferred, **still present at HEAD**
(`todos/pending/2026-10-28-d17-recovery-block-with-no-direction-items.md`).

**File:** `src/driverctl/store.rs:1070-1075` (`blocking_recovery`)

**Issue:** The line `.find(|id| direction.items.is_empty() || direction.items.iter().any(..))` is unchanged.
The doc comment directly above still says an awaiting id "the human has since removed from `DIRECTION.md`
cannot be recovered ... so it no longer blocks". With zero parsed items, every awaiting id blocks, while
`validate_log("recovered", ..)` rejects any id not listed in DIRECTION. The driver is told to log `recovered`
for an item the protocol then refuses.

The disposition's reasoning for deferring is sound. Dropping the clause would let a crashed run dispatch while
DIRECTION.md is absent or unparseable, which loosens a fail-closed gate. Fail-closed is the right default for
a gate. The defect is the unsatisfiable message, not the block.

**Fix:** keep the block and make it satisfiable or honest. Either reword both refusals when
`direction.items.is_empty()` to say "DIRECTION.md has no parseable `### I-NN` items; repair it, then the run
can continue". Or accept `recovered` for an awaiting id while items are empty. Correct the doc comment to
describe the empty case.

### WR-08: A malformed host managed `hooks` or `permissions` value replaces the launcher's gate (NEW)

**File:** `src/launcher/config_dir.rs:583-616` (`managed_settings`)

**Issue:** The host-merge match has two typed arms, `("hooks", Value::Object(..))` and
`("permissions", Value::Object(..))`. Any other value for those two keys (`"hooks": []`, `"hooks": null`,
`"permissions": "x"`) falls through to the `_ => settings.insert(key.clone(), value.clone())` arm. That arm
overwrites the launcher's own `hooks` object, which holds the dispatch gate and the stop guard, or its
`permissions.deny` list. The code already protects the inner level: a non-array host `deny` never replaces the
launcher's list (IN-03, test `a_non_array_host_deny_list_does_not_replace_the_launchers`). The same protection
is missing one level up.

`read_host_managed` accepts any JSON object, so `{"hooks": []}` passes it. The comment on that function says
Claude Code refuses to start on such a file, but that is a claim about another program's validation. This
is the managed tier, which on bwrap launches carries the gate (D-12). A safety list should not depend on
Claude Code rejecting a bad file. `[inferred — cleanup 2026-10-28]`

**Fix:** make the two keys non-overridable by type, as `LOCK_KEYS` does for the others.

```rust
match (key.as_str(), value) {
    ("hooks", Value::Object(events)) => { /* unchanged */ }
    ("permissions", Value::Object(lists)) => { /* unchanged */ }
    // A non-object `hooks` / `permissions` never replaces the launcher's.
    ("hooks" | "permissions", _) => continue,
    _ => { settings.insert(key.clone(), value.clone()); }
}
```

Add a test next to `a_non_array_host_deny_list_does_not_replace_the_launchers` for `{"hooks": []}` and
`{"permissions": "x"}`.

### WR-09: A launcher binary under `/tmp` or `/run` is unreachable inside the sandbox (NEW)

**File:** `src/launcher/config_dir.rs:291` (`snapshot_exe`), with `src/launcher/sandbox.rs:512,545`
(`--tmpfs /tmp`, `--tmpfs /run`) and `src/launcher/run.rs:1596` (`bridged_command(&ctx.binary, ..)`)

**Issue:** Phase 28.1 makes `<this binary> bridge ...` bwrap's command, and the `bin/driverctl` shim execs
the same path. `snapshot_exe` copies the binary out only when it lies under the driven project. The
sandbox then replaces `/tmp` and `/run` with empty tmpfs mounts, and `$HOME` with a tmpfs holding only the
allowlist. `HomeSetup::for_host(&home, &binary, ..)` takes care of a binary under `$HOME`. Nothing handles a
binary under `/tmp` or `/run`. A user running a build from `CARGO_TARGET_DIR=/tmp/...`, an extracted release
under `/tmp`, or a layout reached through `/run` (NixOS `/run/current-system`) gets a launch where bwrap
cannot exec the bridge. The visible failure is a bwrap or bridge "No such file" (exit 127) after the run was
already recorded, not a clear refusal. It is fail-closed, so this is a robustness defect, not an exposure.
`[inferred — cleanup 2026-10-28]`: I read the argv order and the three call sites, and did not run a live
sandbox with such a binary.

**Fix:** widen `snapshot_exe`'s condition to "exe is somewhere the sandbox hides": under the project, or
under `/tmp`, `/run`, `/var/run`, `/dev` or `/proc`. Copy it to `<cfg>/bin` exactly as for the project case.

```rust
const SANDBOX_HIDDEN: &[&str] = &["/tmp", "/run", "/var/run", "/dev", "/proc"];
let hidden = |p: &Path| p.starts_with(project) || SANDBOX_HIDDEN.iter().any(|d| p.starts_with(d));
if !hidden(&real) && !hidden(exe) { return Ok(exe.to_owned()); }
```

Add a unit test beside `a_binary_under_the_project_is_copied_out_and_any_other_is_left_alone`.

## Info

### IN-05: The UTS namespace does not hide the host's name (NEW)

**File:** `src/launcher/sandbox.rs:51` (module doc), `src/launcher/sandbox.rs:625-626` (`--hostname`)

**Issue:** The module doc says `--unshare-uts` fixes the hostname to `gsdmm-sandbox` so "the host's name is not
leaked". The sandbox then re-binds every host `/etc` entry read-only, which includes `/etc/hostname`, usually
`/etc/hosts` (it normally names the host), and `/etc/machine-id`. The kernel hostname is hidden, but the host
name and a stable host identifier are still readable. The driver can send these out only through the inference
proxy or an allowlisted CONNECT host, so the exposure is small. The claim is stronger than the profile.

**Fix:** either soften the doc to "the kernel hostname is not leaked", or skip `hostname` and `machine-id` in
`etc_entries_in` and bind per-run stand-ins at those paths.

### IN-06: A failed `record_launch_posture` leaves the launch half recorded (NEW)

**File:** `src/launcher/run.rs:937` (compare the `supervise` error arm just below)

**Issue:** `begin_launch(..)?` has already bumped `launches` and written the launcher connection. The next
statement, `record_launch_posture(..)?`, can fail on a store write error. It returns without the
`stop_serving()` plus `set_terminal(Stopped, "launcher:startup-failed")` that the `supervise` error arm
performs for the same state. The caller closes the session and drops the lock, leaving a run whose status says
it started a launch that never ran. The state is recoverable (liveness derives the run as dead), and a store
that fails here would likely fail the terminal write too. This is a consistency gap, not a data-loss path.

**Fix:** route both early `?` returns through the same cleanup as the `Err(e)` arm, e.g. wrap them in a small
closure or helper that records the failed start before returning the error.

---

_Reviewed: 2026-10-28_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
