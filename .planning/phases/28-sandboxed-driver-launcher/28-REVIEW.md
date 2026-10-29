---
phase: 28-sandboxed-driver-launcher
reviewed: 2026-10-29T00:00:00Z
depth: standard
files_reviewed: 12
files_reviewed_list:
  - docs/DRIVER-SANDBOX.md
  - src/driver/direction.rs
  - src/driver/lock.rs
  - src/driverctl/serve.rs
  - src/driverctl/store.rs
  - src/launcher/config_dir.rs
  - src/launcher/run.rs
  - src/launcher/stop.rs
  - src/state_reader/driver_frontmatter.rs
  - tests/driverctl_serve.rs
  - tests/driverctl_store.rs
  - tests/launcher_config_dir.rs
findings:
  critical: 0
  warning: 4
  info: 3
  total: 7
status: issues_found
---

# Phase 28: Code Review Report

**Reviewed:** 2026-10-29
**Depth:** standard
**Files Reviewed:** 12
**Status:** issues_found

## Summary

Incremental re-review of Phase 28 at HEAD. The scope is the delta `git diff 80b200e HEAD` over the twelve files
above, which is quick tasks 261028-t3d, t3e, t3f and v2z `[inferred — cleanup 2026-10-29]`:

- t3d: every lock-file open (`acquire_at`, `ensure_lock_file`, `probe_at`, `read_holder_at`) now goes through one
  private `open_lock_file` (`O_NOFOLLOW | O_NONBLOCK`, regular-file and single-link check on the handle).
  `state.json` is read through `read_run_file_capped` in `RunStore::open_with_clock`, `check_wall_clock_budget` and
  `stop_orphaned_scope`. `serve` and `launch` create `DIRECTION.lock` through `ensure_lock_file`.
- t3e: the exchange back after a body mismatch and the give-back loop share one helper, `exchange_or_restore`. An
  absent `DIRECTION.md` is restored by a no-replace rename, bounded by `GIVE_BACK_ROUNDS`.
- t3f and v2z: the host managed `hooks` and `permissions` merge in `config_dir.rs` ignores a non-object value, and
  drops wrong-shape nested values. This is the claimed fix for WR-08.

I read the whole diff, then the surrounding code in `direction.rs` (`run_loop`, `locked`, `sweep_stale_temps`),
`driver_frontmatter.rs` (`open_run_file`, `read_run_file_capped`), `store.rs` (`open_with_clock`, state growth) and
`config_dir.rs` (`managed_settings`, `settings_transform`, `LOCK_KEYS`). I also re-read the sources of the carried
findings in `sandbox.rs`, `store.rs` and `run.rs` at HEAD.

Verification, in a private target directory: `cargo test --no-fail-fast` over `driverctl_serve` (16), `driverctl_store`
(61) and `launcher_config_dir` (28) passes, 105 tests, 0 failed. The lib unit tests for `driver::lock`,
`driver::direction`, `launcher::config_dir` and `driver_frontmatter` were NOT run: the private target directory hit the
disk quota on that second build, so I removed it and did not retry. The live bwrap tests are `#[ignore]` and were not
run.

No critical issues. The hardening work is sound: I found no remaining unguarded read of `state.json` or a lock file
in `src/`, and the `run_loop` restore path terminates (it is bounded by `SPLICE_TRIES`, and the helper by
`GIVE_BACK_ROUNDS`). Two new findings concern the host-policy merge (WR-10, IN-07). Five prior findings are carried
(WR-02, WR-04, WR-09, IN-05, IN-06). WR-08 is fixed and holds.

The new IDs follow the ledger. CR-01, WR-01..WR-09 and IN-01..IN-06 are all taken, so the new ones are WR-10 and IN-07.
There is no CR-02.

### Prior findings re-checked at HEAD

| Prior | Disposition | Status at HEAD |
|---|---|---|
| WR-01 `bind_private` rename race | fixed (n12 e98520c) | Holds. `serve.rs` is unchanged apart from the `ensure_lock_file` call, and the `hard_link` staging is intact. |
| WR-02 drivers tmpfs skipped for a symlinked `.planning` | deferred | **Still present.** `sandbox.rs:582-587` is byte-identical to the prior review. Restated below, with the part t3d closed noted. |
| WR-03 unbounded `DIRECTION.md` reads | fixed (n12 dc812c0) | Holds. t3d extends the same guard (`open_run_file` plus the 1 MiB cap) to `state.json`. No unguarded `DIRECTION.md` or `state.json` reader remains outside tests. |
| WR-04 D-17 block with no parsed items | deferred | **Still present.** `store.rs:1085` keeps `direction.items.is_empty() \|\| ..`. The new test `a_crashed_run_with_direction_md_absent_is_still_refused_dispatch` (`tests/driverctl_store.rs`) pins the fail-closed half the deferral relies on. It does not touch the unsatisfiable message. Restated below. |
| WR-05 macOS enablement half applied | fixed (n11) | Holds. The t3 range adds only `rustix`-based, platform-neutral code. FIFO tests are Linux-gated. |
| WR-06 credential denylist | fixed (Phase 28.1) | Not re-derived. `home.rs` is outside the requested files. |
| WR-08 malformed host managed `hooks`/`permissions` replaces the gate | fixed (t3f 6a55713, v2z) | **Holds.** See the evidence below. No `### WR-08` heading is emitted. |
| IN-01 ledger race in the acceptance script | skipped | Unchanged. |
| IN-02 `tty_open` probe | fixed (n12 88a61ec) | Holds. |
| IN-03 signal escalation counts "already set" | skipped | Unchanged. |
| IN-04 doc drift | fixed (n12 d344401) | Holds. The `blocking_recovery` doc comment is still wrong, which belongs to WR-04. |
| WR-09 launcher binary under `/tmp` or `/run` | open | **Still present.** `snapshot_exe`'s condition is unchanged (`config_dir.rs:303`). Restated below. |
| IN-05 UTS namespace does not hide the host name | open | **Still present.** `sandbox.rs:51,78,625` unchanged; `etc_entries_in` (`sandbox.rs:383-404`) re-binds every `/etc` entry except `claude-code`. Restated below. |
| IN-06 failed `record_launch_posture` leaves the launch half recorded | open | **Still present.** `run.rs:935` still returns through `?` before the `supervise` error arm. Restated below. |

**WR-08 evidence.** `managed_settings` (`config_dir.rs:663-725`) skips `LOCK_KEYS` first. It then handles
`("hooks", Value::Object(..))` and `("permissions", Value::Object(..))`. The new arm `("hooks" | "permissions", _) => {}`
(`config_dir.rs:718`) sits before the `_ => settings.insert(..)` catch-all, so a string, array, null, number or bool can
never replace the launcher's `hooks` object or the `deny` list. Inside an object, the nested values pass through
`well_formed_hooks` and `well_formed_permissions` first. `deny` can only arrive as an array of strings, and `append_unique`
appends after the launcher's list. The degraded posture is covered by `settings_transform` (`config_dir.rs:614-621`).
Tests pin the table of shapes for both keys in both postures (`config_dir.rs` unit tests and
`tests/launcher_config_dir.rs`). The integration tests pass. The fix holds as claimed.

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

When the symlink target lies inside the project bind, the sibling runs stay visible and writable to the sandboxed
driver. The 261028-t3d hardening narrows what that buys. The launcher's pre-lock `state.json` read
(`run.rs:1220`), `RunStore::open` (`store.rs:645`) and `driver stop` (`stop.rs:203`) now refuse a FIFO, a symlink or
an oversized file instead of hanging on it. A driver can still plant one of those in a sibling run, which turns that
run's next launch into a `CorruptState` refusal, and it can rewrite a sibling's `state.json` or lock files. The
hang is closed. The cross-run reach is not.

The disposition is right that the review's original `ends_with(DRIVER_RUNS_REL)` fix does not match its own
example (`planning` against `.planning`).

**Fix:** carry the canonical drivers directory in `LaunchPaths` and compare against that. The launcher already
knows it, because `validate_run_dir` canonicalizes `project/DRIVERS_REL`.

```rust
// LaunchPaths gains: pub drivers_dir: PathBuf  // canonical parent of run_dir
if paths.run_dir.parent() == Some(paths.drivers_dir.as_path()) { /* emit the tmpfs */ }
```

If the run dir is not directly under it, refuse the launch rather than skipping the hiding. Update the golden
test to expect a symlinked-planning case with the tmpfs.

### WR-04: The D-17 recovery block cannot be satisfied when DIRECTION has no parsed items

**Status:** prior, deferred, **still present at HEAD**
(`todos/pending/2026-10-28-d17-recovery-block-with-no-direction-items.md`).

**File:** `src/driverctl/store.rs:1081-1086` (`blocking_recovery`)

**Issue:** The line `.find(|id| direction.items.is_empty() || direction.items.iter().any(..))` is unchanged.
The doc comment directly above (`store.rs:1076-1080`) still says an awaiting id "the human has since removed from
`DIRECTION.md` cannot be recovered ... so it no longer blocks". With zero parsed items, every awaiting id blocks,
while `validate_log("recovered", ..)` rejects any id not listed in DIRECTION. The driver is told to log `recovered`
for an item the protocol then refuses.

The disposition's reasoning for deferring is sound, and the t3e test above now pins it: dropping the clause would
let a crashed run dispatch while DIRECTION.md is absent or unparseable, which loosens a fail-closed gate.
Fail-closed is the right default for a gate. The defect is the unsatisfiable message, not the block.

**Fix:** keep the block and make it satisfiable or honest. Either reword both refusals when
`direction.items.is_empty()` to say "DIRECTION.md has no parseable `### I-NN` items; repair it, then the run
can continue". Or accept `recovered` for an awaiting id while items are empty. Correct the doc comment to
describe the empty case.

### WR-09: A launcher binary under `/tmp` or `/run` is unreachable inside the sandbox (NEW)

**File:** `src/launcher/config_dir.rs:302-304` (`snapshot_exe`), with `src/launcher/sandbox.rs:512,544`
(`--tmpfs /tmp`, `--tmpfs /run`) and `src/launcher/run.rs:1597` (`bridged_command(&ctx.binary, ..)`)

**Issue:** Phase 28.1 makes `<this binary> bridge ...` bwrap's command, and the `bin/driverctl` shim execs
the same path. `snapshot_exe` copies the binary out only when it lies under the driven project
(`!real.starts_with(project) && !exe.starts_with(project)` returns the original path). The sandbox then replaces
`/tmp` and `/run` with empty tmpfs mounts, and `$HOME` with a tmpfs holding only the allowlist.
`HomeSetup::for_host(&home, &binary, ..)` takes care of a binary under `$HOME`. Nothing handles a binary under
`/tmp` or `/run`. A user running a build from `CARGO_TARGET_DIR=/tmp/...`, an extracted release under `/tmp`, or a
layout reached through `/run` (NixOS `/run/current-system`) gets a launch where bwrap cannot exec the bridge. The
visible failure is a bwrap or bridge "No such file" (exit 127) after the run was already recorded, not a clear
refusal. It is fail-closed, so this is a robustness defect, not an exposure.
`[inferred — cleanup 2026-10-28]`: I read the argv order and the three call sites, and did not run a live sandbox
with such a binary.

**Fix:** widen `snapshot_exe`'s condition to "exe is somewhere the sandbox hides": under the project, or under
`/tmp`, `/run`, `/var/run`, `/dev` or `/proc`. Copy it to `<cfg>/bin` exactly as for the project case.

```rust
const SANDBOX_HIDDEN: &[&str] = &["/tmp", "/run", "/var/run", "/dev", "/proc"];
let hidden = |p: &Path| p.starts_with(project) || SANDBOX_HIDDEN.iter().any(|d| p.starts_with(d));
if !hidden(&real) && !hidden(exe) { return Ok(exe.to_owned()); }
```

Add a unit test beside `a_binary_under_the_project_is_copied_out_and_any_other_is_left_alone`.

### WR-10: A wrong-shape host managed policy value is dropped with no diagnostic, though a non-object file is refused (NEW)

**File:** `src/launcher/config_dir.rs:359-424` (`well_formed_hooks`, `well_formed_permissions`),
`src/launcher/config_dir.rs:663-721` (their use in `managed_settings`), against `read_host_managed`
(`config_dir.rs:736-767`)

**Issue:** `read_host_managed` documents the launcher's principle for a bad host managed file: Claude Code refuses
to start on it, "so the launch is refused here ... instead of silently running without the administrator's policy".
The v2z change takes the opposite course one level down. A wrong-shape nested value is discarded without a trace:
`{"permissions": {"deny": "Bash(rm *)"}}` loses the administrator's deny rule, `{"hooks": {"PreToolUse": {...}}}`
loses an audit hook, and one malformed hook item discards its whole matcher entry, valid siblings included. A search
of `config_dir.rs` finds no `eprintln!`, `tracing` call or returned note anywhere on the path. Neither the operator
nor the administrator can learn that the driver ran without the policy they wrote.

WR-08's own fix (ignore a non-object value) is the right fail-closed choice for the launcher's gate. The defect is
the silence. The administrator's policy is then weakened (a lost `deny`, a lost hook) while the launcher reports a
normal launch, and the managed file is the tier that is supposed to be authoritative on bwrap launches.
`[inferred — cleanup 2026-10-29]`: I can prove the absence of any diagnostic, and the contrast with the top-level
refusal. I did not verify how Claude Code treats such a value itself.

**Fix:** make the drop observable, and ideally keep the existing refuse-on-bad-policy rule.

```rust
// in read_host_managed, after the object check, for each doc:
if let Some(Value::Object(h)) = doc.get("hooks") && &well_formed_hooks(h) != h {
    return Err(invalid("a malformed hooks entry"));
}
// likewise for "permissions" with well_formed_permissions.
```

If refusal is too strict for the project, return the dropped key paths from `managed_settings` and have `launch`
print one `driver launch: ignored malformed host managed policy: <file>: <path>` line, as it does for the
host-proxy and gateway warnings. Add a test that asserts the notice (or the refusal).

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

**File:** `src/launcher/run.rs:935` (compare the `supervise` error arm just below)

**Issue:** `begin_launch(..)?` has already bumped `launches` and written the launcher connection. The next
statement, `record_launch_posture(..)?`, can fail on a store write error. It returns without the
`stop_serving()` plus `set_terminal(Stopped, "launcher:startup-failed")` that the `supervise` error arm
performs for the same state. The caller closes the session and drops the lock, leaving a run whose status says
it started a launch that never ran. The state is recoverable (liveness derives the run as dead), and a store
that fails here would likely fail the terminal write too. This is a consistency gap, not a data-loss path.

**Fix:** route both early `?` returns through the same cleanup as the `Err(e)` arm, e.g. wrap them in a small
closure or helper that records the failed start before returning the error.

### IN-07: Host managed `permissions` scalar settings are first-wins while every other host key is last-wins (NEW)

**File:** `src/launcher/config_dir.rs:698-711` (the `other => ours.entry(key).or_insert(other)` arm), against the
catch-all `settings.insert(key.clone(), value.clone())` at `config_dir.rs:719-721`

**Issue:** `managed_settings` merges `host` "in precedence order: the file, then the drop-ins" (sorted by file
name). For a top-level key, the catch-all `insert` makes the last doc win, which is the usual drop-in rule. For the
string and bool `permissions` settings that the v2z change deliberately passes through (`defaultMode`,
`disableBypassPermissionsMode`, `blockReadsOutsideWorkingDirectories`), the arm uses `or_insert`, so the FIRST doc
wins. With `managed-settings.json` setting `defaultMode: "default"` and `managed-settings.d/50-site.json` setting
`"plan"`, the base file's value is merged and the drop-in's is ignored, while the same pair under `model` would
merge as the drop-in's. The comment calls this "(I-6)", a recorded decision, but the function's doc does not say it,
and no test pins which doc wins. `[inferred — cleanup 2026-10-29]`: whether Claude Code's own drop-in precedence is
last-wins is not verified here, so this is flagged as an internal inconsistency, not a confirmed
mismatch.

**Fix:** pick one rule. Either use `ours.insert(key, other)` so these settings follow the last-wins rule of every
other key, or state first-wins in the `managed_settings` doc comment and add a test with the same
`permissions` scalar set in the file and in a drop-in, asserting the winner.

---

_Reviewed: 2026-10-29_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
