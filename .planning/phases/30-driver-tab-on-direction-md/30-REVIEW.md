---
phase: 30-driver-tab-on-direction-md
reviewed: 2026-10-23T00:00:00Z
depth: standard
files_reviewed: 7
files_reviewed_list:
  - src/app.rs
  - src/driver/direction.rs
  - src/driver/steer.rs
  - src/ui/screens/detail.rs
  - src/ui/screens/driver_launch.rs
  - src/ui/screens/driver_run.rs
  - src/ui/screens/driver_steer.rs
findings:
  critical: 0
  warning: 2
  info: 3
  total: 5
status: issues_found
---

# Phase 30: Code Review Report (re-review of `git diff e05ad64 HEAD`)

**Reviewed:** 2026-10-23
**Depth:** standard
**Files Reviewed:** 7
**Status:** issues_found

## Summary

Scope: `git diff e05ad64 HEAD` for the seven files that changed after the last review. `src/driver/direction.rs` was read from `git show HEAD:src/driver/direction.rs` only. The working-tree copy carries another agent's uncommitted edits and was not used. The diff breaks down as follows.

- **Quick 261021-101:** the fixes for WR-02, WR-07 and IN-04.
- **Quick 261021-1id:** the glyph and DIM label routing through `crate::ui::glyphs`.
- **Quick 261020-vxg and 261021-0jg:** macOS and Windows build gating.
- **Commit 39dad19:** test environment assumptions.
- **The Phase 28 and workstream series:** most of `detail.rs` and `app.rs`.

I traced the following and found them sound.

- **`run_loop` guarded reads.** Its three reads (`DIRECTION.md`, the displaced temp, the exchange-back temp) go through `open_run_file` (`O_NOFOLLOW | O_NONBLOCK`, type check on the handle). A refusal is an `Io` error, a missing file is still `NoDirection`, and an unreadable displaced file is still a mismatch that is given back.
- **macOS exchange.** `rustix` 1.1.5 defines `RenameFlags::EXCHANGE` for Apple targets as `RENAME_SWAP`, and `renameat_with` is `cfg(any(apple, linux_kernel, ..))`. The `any(linux, macos)` gating in `direction.rs` therefore compiles. `direction_lock` is `cfg(unix)`, which covers macOS.
- **`model_name_ok` and the `--model` argv.** The model reaches the detached launch as a separate argv element (`launcher/run.rs:1147`, `sandbox.rs:374`), never through a shell. The brackets cannot inject anything, and the leading-alphanumeric rule still blocks flag-shaped values. The relaunch prefill shares the predicate, so a recorded `claude-opus-5[1m]` (the real identity in the fixtures) is now kept.
- **Glyph registry.** All ten Driver glyph constants are `&'static str` aliases of registry constants, so the "no glyph is derived from file content" guarantee is intact. Every new glyph is 1 cell, so no column arithmetic (`RUN_WORD_CELLS`, `ITEM_WORD_CELLS`, `DRIVER_LIVE_MARKER` cell reservation) shifts. Each status still has a distinct glyph.
- **`scroll_offset` and `generic_viewport` removal from `DetailScreen`.** No other file reads them. The Driver tab keeps its own `driver_viewport` and its `move_driver_selection` and `scroll_driver_output` arms, and the removed `_ =>` fallback only served the retired Roadmap box view. `switch_to_sub_view` and `switch_to_tab` lost one parameter, and every call site (keys, mouse, `opened_on`) was updated.
- **Workstream additions.** The code in `detail.rs` (`scoped_phases`, `scoped_phase_count`, `focus_agent_of_plan`) and `app.rs` (`project_status_category`, `format_workstream_display`, the `agents.workstreams.is_empty()` scan condition) touches the Detail screen's shared `pipeline_selected` and `roadmap_cursor` only through clamped indices. I found no panic path and nothing that reaches the Driver tab. This was a read-through, not an audit of Phase 28.
- **Test changes.** The log-replacement test now renames a staged file and asserts the inode differs. The `cfg(unix)` helpers in `app.rs` match their callers' gating.

I did not run any tests. Another agent is running timing-sensitive stress tests on this machine, and building the lib test binary would compete with it.

No critical findings. Two new warnings sit next to the WR-02 fix, which closed the symptom (a blocking `open`) but not the class (an unguarded open in the same run directory). Three info items remain.

## Prior findings status

- **WR-02** (run-directory files opened without a file-type guard): **Resolved.** `read_run_file` (`src/driver/direction.rs`, in `mod imp`) wraps `open_run_file`. `run_loop` uses it for the target read, the displaced-temp compare and the exchange-back read. The new test `steer_run_refuses_a_fifo_direction_without_blocking` (`src/driver/steer.rs`) covers the path the old review named (the `c`/`a` keys reach `write_body` with no guarded read before it) and asserts `DIRECTION.lock` is released afterwards. Two sibling gaps in the same class remain: see WR-08 and WR-09.
- **WR-07** (model rule narrower than `claude --model`): **Resolved.** `model_name_ok` admits `[` and `]` (`driver_launch.rs:232-241`). The tests accept `opus[1m]`, `sonnet[1m]`, `claude-sonnet-4-5[1m]` and `anthropic.claude-3-5-sonnet-20240620-v1:0`, and refuse `-opus[1m]`. `relaunch_prefill` shares the predicate, so a recorded `opus[1m]` is no longer dropped for the default.
- **IN-04** (lint popup whose findings get zero rows is forced clipped): **Resolved.** Both `LintPopupScreen` (`driver_launch.rs:1512-1527`) and `EditLintConfirmScreen` (`driver_steer.rs:877-899`) skip `fit_list` at a zero row budget. New tests pin the exact heights (80x8 and 80x7) and assert `y` confirms. See IN-07 for a disclosure side effect of this fix.
- **IN-05** (body-relative `DIRECTION.md:<line>` differs from `driver check` and the launcher's refusal): **Still stands.** `launch_preflight` (`driver_launch.rs:389`) and `request_new_run_check` (`app.rs:1692`) lint `split_frontmatter(&text).1`. `launcher/run.rs:240` and `driver_cli.rs:459` lint the whole file, and `launch_error_copy` (`driver_launch.rs:451`) prints the launcher's whole-file numbers. Nothing in the diff touched this. The disposition correctly records it as deferred.

## Warnings

### WR-08: `DIRECTION.lock` is opened by path with symlink-following and truncating-write semantics, the one run-directory open the hardening still skips

**File:** `src/driver/direction.rs` (`locked()` in `mod imp`, via `direction_lock`) reaching `src/driver/lock.rs:196-260` (`acquire_at`, `write_holder`)
**Issue:** `locked()` takes `<run>/DIRECTION.lock` before every `write_body` and splice. `acquire_at` opens that path with `OpenOptions::new().create(true).read(true).write(true).truncate(false)`. This follows symlinks and has no `O_NOFOLLOW` and no file-type check. After the `flock`, `write_holder` calls `file.set_len(0)`, writes the holder JSON through the handle, and then `set_permissions(0o644)`.

If `DIRECTION.lock` is a symlink to a file the user can write (for example `~/.ssh/authorized_keys` or a dotfile), pressing `c` or `a` on that run, or confirming an edit, makes the TUI:

- truncate that file;
- fill it with the holder record;
- chmod it to 0644.

This is the T-30-04/T-30-24 threat model: 30-SECURITY lists the run directory as writable by an `--unsandboxed` run. The same class that WR-02 closed for reads (a planted entry in the run directory) therefore still applies to the lock open, with a destructive write instead of a hang. A FIFO at that name does not block on Linux (`O_RDWR`). It fails at `set_len` with `Unavailable`, which `locked()` turns into a steer error.

Severity is Warning, not Critical. The code is pre-existing (Phase 17/27) and `lock.rs` is outside this review's file set. Reaching it needs a writer in the run directory, and an `--unsandboxed` run can already act as the user. But the phase claims "one guarded open for every run-dir read", and the lock open is the one run-directory open that writes.
**Fix:** Open the lock through the same guard discipline. In `acquire_at`, add `custom_flags((OFlags::NOFOLLOW | OFlags::NONBLOCK).bits() as i32)` on unix, and after the open refuse anything that is not a regular file before `flock` or `set_len`:
```rust
let meta = file.metadata().map_err(|e| LockError::Unavailable { detail: e.to_string() })?;
if !meta.file_type().is_file() {
    return Err(LockError::Unavailable {
        detail: format!("{} is not a regular file", path.display()),
    });
}
```
`launcher.lock` goes through the same function and gets the same protection. Add a test with a symlinked `DIRECTION.lock` pointing at a sentinel file, and assert the sentinel is unchanged and `steer_run` returns `Failed`.

**DTAB impact:** none of DTAB-01..04 is defeated. DTAB-03's "takes the shared flock" still holds for a regular lock file. This is a hardening gap, not a functional one.

### WR-09: `run_loop`'s guarded read has no size cap, although every other run-directory reader enforces 1 MiB

**File:** `src/driver/direction.rs` (`read_run_file` and its three callers in `run_loop`), reached from `src/driver/steer.rs` `steer_run` and `apply_edit`
**Issue:** `read_run_file` ends in `file.read_to_end(&mut bytes)`. The reader that feeds the tab caps at `DIRECTION_READ_MAX_BYTES` (1 MiB, `driver_frontmatter.rs:770`, `take(..+1)` at `:870`). The editor temp caps at `EDIT_TEMP_MAX_BYTES` (`steer.rs:595`, `:632`). The write path has no cap. `request_steer` goes straight to `steer_run` and then `write_body` with no capped read first. A regular file of arbitrary size at `DIRECTION.md` (a sparse multi-GB file is enough) is read fully into memory under `DIRECTION.lock`, once for the target and again for the displaced inode after the exchange. 30-SECURITY T-30-04 names `DIRECTION_READ_MAX_BYTES` + `take` as the mitigation for this same planted-file threat, so the cap exists for the display path and was not carried into the write path when WR-02 moved it onto `open_run_file`. The previous `std::fs::read` was equally unbounded, so this is not a regression. But the new `read_run_file` doc comment ("a planted entry cannot park the caller") overstates what it guarantees.
**Fix:** Cap the read and treat an over-cap file as a refusal.
```rust
fn read_run_file(path: &Path) -> std::io::Result<Vec<u8>> {
    const CAP: u64 = crate::state_reader::driver_frontmatter::DIRECTION_READ_MAX_BYTES;
    let file = crate::state_reader::driver_frontmatter::open_run_file(path)?;
    let mut bytes = Vec::new();
    file.take(CAP + 1).read_to_end(&mut bytes)?;
    if bytes.len() as u64 > CAP {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            "DIRECTION.md is larger than 1 MiB",
        ));
    }
    Ok(bytes)
}
```
Add a test that `steer_run` over a file just past the cap returns `Failed` without allocating it.

**DTAB impact:** none defeated. DTAB-03's edits still work for any legitimately sized `DIRECTION.md`.

## Info

### IN-05: Body-relative `DIRECTION.md:<line>` in the TUI differs from `driver check` and the launcher's own refusal (carried, unchanged)

**File:** `src/ui/screens/driver_launch.rs:389`, `src/app.rs:1692`, against `src/launcher/run.rs:240` and `src/driver_cli.rs:459`
**Issue:** See "Prior findings status". The TUI numbers findings from the first body line (the editor contract). The launcher and `driver check` number them against the whole file. `launch_error_copy` can therefore print "DIRECTION.md:57" for a problem the form showed as "DIRECTION.md:25". The window is narrow, because the preflight normally catches the problem first. It is deferred in the disposition.
**Fix:** Label the TUI numbers with their base (`body line 25`), or re-base `DirectionInvalid` problems by the frontmatter line count in `launch_error_copy`.
**DTAB impact:** none.

### IN-06: macOS exchange failure on a volume without `RENAME_SWAP` surfaces under the Linux syscall name

**File:** `src/driver/direction.rs` (the `io_err("renameat2(RENAME_EXCHANGE)", errno)` arm of `run_loop`; the module doc and `unsupported()` were updated for macOS, this string was not)
**Issue:** 261020-vxg enabled the exchange path on macOS, where `renameatx_np(RENAME_SWAP)` fails with `ENOTSUP` on volumes that do not implement it (non-APFS/HFS+: SMB, NFS, exFAT). The arm still reports `renameat2(RENAME_EXCHANGE): <errno>`. A macOS user steering a project on such a volume gets a Linux syscall name and a bare errno, with no hint that the volume is the cause. The design is deliberately fail-closed with no plain-rename fallback, so the only gap is the message. Linux filesystems without `RENAME_EXCHANGE` (EINVAL) have always had the same bare message.
**Fix:** Name the operation portably and map the unsupported errnos:
```rust
Err(errno) => {
    let _ = std::fs::remove_file(&temp_path);
    let why = if matches!(errno, rustix::io::Errno::NOTSUP | rustix::io::Errno::INVAL) {
        format!("{errno} (this filesystem does not support an atomic rename exchange)")
    } else {
        errno.to_string()
    };
    return Err(io_err("atomic rename exchange", why));
}
```
**DTAB impact:** none on Linux. On macOS with an unsupporting volume DTAB-03 steering fails closed (by design), so this only affects diagnosability.

### IN-07: At a zero row budget both lint popups drop every finding and leave a header that ends in a colon

**File:** `src/ui/screens/driver_launch.rs:1512-1527` (`LintPopupScreen`), `src/ui/screens/driver_steer.rs:877-899` (`EditLintConfirmScreen`)
**Issue:** The IN-04 fix (leave the list out at a zero budget) is what the prior review proposed, and it does make `y` live. A side effect is that at those one or two terminal heights the popup reads "driver check found 10 problem(s) in your edit:" directly above "Apply it anyway?". The user confirms an edit that lint rejected without seeing any finding. The same applies when `rows == 1` and the first finding does not fit: `fit_list` returns only `… N more`. The count is disclosed, so this is not a safety hole. It is a thin-disclosure choice on the lint-override confirmation specifically.
**Fix:** When the list is dropped, end the header without the colon, or add a `(enlarge the terminal to list them)` hint on the muted row. Alternatively keep `y` inert at a zero budget for `EditLintConfirmScreen` (the override), and live only for `LintPopupScreen` (the "edit again" popup, where disclosure is not what `y` accepts).
**DTAB impact:** none.

---

_Reviewed: 2026-10-23_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
