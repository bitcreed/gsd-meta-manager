---
phase: 30-driver-tab-on-direction-md
reviewed: 2026-10-24T00:00:00Z
depth: standard
files_reviewed: 4
files_reviewed_list:
  - src/driver/direction.rs
  - src/driverctl_cli.rs
  - tests/driver_lock.rs
  - tests/driverctl_race.rs
findings:
  critical: 0
  warning: 3
  info: 3
  total: 6
status: issues_found
---

# Phase 30: Code Review Report (incremental re-review of `git diff 7f1f044 HEAD`)

**Reviewed:** 2026-10-24
**Depth:** standard
**Files Reviewed:** 4
**Status:** issues_found

## Summary

Scope: `git diff 7f1f044 HEAD` on four files, produced by quick task 261023-v1s (commits 717d724, 801c656, d960678). The working tree had no edits to these files (`git status` showed only `.planning/` changes), so I read them directly. I did not run the tests or clippy.

I traced the new code against its tests and found it sound on the paths it targets.

- **`give_back_newest` (`direction.rs`).** I walked the three racing unit tests step by step (inode and bytes at each exchange). In `a_save_landing_right_after_the_exchange_back_is_not_reverted`, round 1 places save 2 and brings save 3 back, which is not the expected displaced file. Round 2 places save 3 and brings save 2 back, which equals the previous placing, so it is dropped. Save 3 stands. The `expect`/`placing` hand-off (`expect = replace(placing, back)`) is correct. Inode reuse cannot produce a false "unchanged": a reused inode would have to carry byte-identical content, and dropping an identical copy loses nothing.
- **`send_and_read` / `read_reply` (`driverctl_cli.rs`).** On an AF_UNIX stream, a peer that closes with unread data sets `ECONNRESET` on the other end, but queued data is still delivered first. `read_reply` stops at the first `\n` and never issues the read that would surface the reset, so a queued `busy` line is returned. Only `BrokenPipe` and `ConnectionReset` trigger the relay, and a failed relay falls back to the original send error. The new tests are `cfg(all(test, unix))`-gated like the rest of the module.
- **`wait_for_exec_started_or_report` (`driver_lock.rs`).** `exec_started` is journaled from `ExecutionEvent::SessionStarted` (`journal/mod.rs:2197`), the agent's own init event. It is therefore emitted after the exec, which is what the comment claims. The wait is bounded (600 x 50 ms) and reports an early child exit.
- **Handshake (`driverctl_race.rs`).** The yield cannot deadlock. `yield_one_splice` is bounded by `wait_until` (4 s) and by `tool_exited`. `tool` runs past its deadline only while `ok == 0 && !human_finished`, and `human_finished`/`tool_exited` are set from drop guards, so a panic on either side releases the other. `ok` is incremented before `done`, so a human that sees `done > begun` also sees `ok >= 1`.

No critical findings. The two carried warnings (WR-08, WR-09) and the carried info item (IN-06) apply unchanged at HEAD. WR-09 is slightly wider now. One new warning sits in the new give-back path.

## Prior findings status

- **WR-08** (`DIRECTION.lock` opened by path with symlink-following, truncating-write semantics): **Still applies.** `src/driver/lock.rs` is untouched by `7f1f044..HEAD`. `acquire_at` still opens with `create(true).read(true).write(true).truncate(false)` and no `O_NOFOLLOW` (`lock.rs:201-209`), then `set_len(0)` (`:265`) and `set_permissions(0o644)` (`:278`). `direction.rs` still reaches it unchanged through `direction_lock` in `locked()` (`:641`).
- **WR-09** (`run_loop`'s guarded read has no 1 MiB size cap): **Still applies, and the surface grew.** `read_run_file` (`direction.rs:588-593`) still ends in an unbounded `read_to_end`. The new `seen()` (`:558-565`) calls it for every displaced and given-back file, and each `Seen` keeps the whole byte vector. One splice now reads a planted large file up to three times, plus once per give-back round (up to 8), all under `DIRECTION.lock`.
- **IN-06** (macOS exchange failure reported under Linux syscall name): **Still applies.** `direction.rs:770` still reports `renameat2(RENAME_EXCHANGE)`. The new give-back messages (`:689`, `:704`) do not name the syscall, but `:770` and the module doc are unchanged.
- IN-05 and IN-07 are outside these files and are not re-reported.

## Warnings

### WR-08: `DIRECTION.lock` is opened by path with symlink-following and truncating-write semantics, the one run-directory open the hardening still skips

**File:** `src/driver/direction.rs` (`locked()` in `mod imp`, via `direction_lock`) reaching `src/driver/lock.rs:196-260` (`acquire_at`, `write_holder`)
**Issue:** `locked()` takes `<run>/DIRECTION.lock` before every `write_body` and splice. `acquire_at` opens that path with `OpenOptions::new().create(true).read(true).write(true).truncate(false)`. This follows symlinks and has no `O_NOFOLLOW` and no file-type check. After the `flock`, `write_holder` calls `file.set_len(0)`, writes the holder JSON through the handle, and then `set_permissions(0o644)`.

If `DIRECTION.lock` is a symlink to a file the user can write (for example `~/.ssh/authorized_keys` or a dotfile), pressing `c` or `a` on that run, or confirming an edit, makes the TUI:

- truncate that file;
- fill it with the holder record;
- chmod it to 0644.

This is the T-30-04/T-30-24 threat model: 30-SECURITY lists the run directory as writable by an `--unsandboxed` run. The class that WR-02 closed for reads (a planted entry in the run directory) therefore still applies to the lock open, with a destructive write instead of a hang. A FIFO at that name does not block on Linux (`O_RDWR`). It fails at `set_len` with `Unavailable`, which `locked()` turns into a steer error.

Severity is Warning, not Critical. The code is pre-existing (Phase 17/27) and `lock.rs` is outside this file set. Reaching it needs a writer in the run directory, and an `--unsandboxed` run can already act as the user. But the phase claims "one guarded open for every run-dir read", and the lock open is the one run-directory open that writes.
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

### WR-09: `run_loop`'s guarded read has no size cap, although every other run-directory reader enforces 1 MiB

**File:** `src/driver/direction.rs:588-593` (`read_run_file`; callers in `run_loop` at `:723`, and via `seen()` at `:563`, `:779`, `:804`, `:695`), reached from `src/driver/steer.rs` `steer_run` and `apply_edit`
**Issue:** `read_run_file` ends in `file.read_to_end(&mut bytes)`. The reader that feeds the tab caps at `DIRECTION_READ_MAX_BYTES` (1 MiB, `driver_frontmatter.rs:770`, `take(..+1)` at `:870`). The editor temp caps at `EDIT_TEMP_MAX_BYTES` (`steer.rs:595`, `:632`). The write path has no cap. A regular file of arbitrary size at `DIRECTION.md` (a sparse multi-GB file is enough) is read fully into memory under `DIRECTION.lock`. Since 261023-v1s it is read again for the displaced inode (`seen`), again for the exchange-back, and once per give-back round, with each `Seen` holding its own copy of the bytes. 30-SECURITY T-30-04 names `DIRECTION_READ_MAX_BYTES` + `take` as the mitigation for this planted-file threat, so the cap exists for the display path and was not carried into the write path. The doc comment on `read_run_file` ("a planted entry cannot park the caller") overstates what it guarantees.
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
`seen()` already maps a failed read to `bytes: None`, so an over-cap displaced file is compared by inode alone and still gives back correctly. Add a test that `steer_run` over a file just past the cap returns `Failed` without allocating it.

### WR-10: A give-back `ENOENT` strands the human's newest save at a hidden temp name, where the next launch sweeps it

**File:** `src/driver/direction.rs:684-694` (`give_back_newest`, the `exchange` error arm), swept by `src/launcher/run.rs:613` and `src/driverctl/serve.rs:930` (`STALE_TEMP_AGE` = 60 s)
**Issue:** Before 261023-v1s the newest save was restored with `std::fs::rename(&temp_path, &target)`. That works whether or not `DIRECTION.md` exists at that instant. The replacement is `renameat2(RENAME_EXCHANGE)`, which fails with `ENOENT` when `DIRECTION.md` is absent. The window is the same one the fix is about: between the exchange back (`:792`) and the give-back exchange, DIRECTION.md holds the older displaced file. An editor or tool that saves by unlink-then-create, or a `git checkout` that removes the file, hits it. In that case:

- `give_back_newest` returns `Err("could not put a newer save back; a displaced file was left at .DIRECTION.md.fm.<pid>.<n>")`.
- The human's newest bytes exist only at the dot-named temp.
- `DIRECTION.md` is absent.
- Any later gsd-mm launch or `driverctl serve` for that run calls `sweep_stale_temps` with a 60 s threshold, which deletes the temp. The save is then gone.

The first exchange in `run_loop` already treats `ENOENT` specially (`:764`, returned as `NoDirection`). The new loop has no equivalent, and here the temp holds a human's file rather than the tool's own, so the right outcome is to restore it, not to report an error.
**Fix:** On `ENOENT`, restore with a no-replace rename, which cannot overwrite anything a newer writer creates in the meantime:
```rust
Err(errno) if errno == rustix::io::Errno::NOENT => {
    match renameat_with(dir, temp_name, dir, DIRECTION_FILE, RenameFlags::NOREPLACE) {
        Ok(()) => return Ok(()),
        // Something re-created DIRECTION.md in between: exchange on the next round.
        Err(rustix::io::Errno::EXIST) => continue,
        Err(e) => return Err(io_err("could not restore a newer save", e)),
    }
}
```
`RenameFlags::NOREPLACE` is available on Linux and Apple targets in rustix 1.x. Add a race-hook point between the exchange back and the give-back (a new `Point`) so a unit test can unlink `DIRECTION.md` there and assert the newest save ends up at `DIRECTION.md`.

## Info

### IN-06: macOS exchange failure on a volume without `RENAME_SWAP` surfaces under the Linux syscall name

**File:** `src/driver/direction.rs:770` (the `io_err("renameat2(RENAME_EXCHANGE)", errno)` arm of `run_loop`)
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

### IN-08: `contended > 0` is satisfied by any busy tool thread, so it does not show the read-to-exchange window was raced

**File:** `tests/driverctl_race.rs:253-256` (the sample), `:467-474` (the assertion)
**Issue:** The header and the assertion message claim `contended > 0` means "a save started while a splice was in flight, so the race was exercised". The sample is `begun > done` read before the save is written. The tool thread loops splice after splice with no pause, so `begun > done` holds for nearly all of the tool's duty cycle: the lock wait, temp write and fsync all count as "in flight". The save then lands anywhere in that span, usually not in the narrow read-to-exchange window. A harness in which no save ever raced a read-to-exchange window would still pass the assertion. The non-vacuity proof the file really has is `editor_race_naive_positive_control` (LOST > 0), not this assertion. It is not a correctness hazard, but the failure message ("the race was never exercised") promises more than the check delivers.
**Fix:** Either reword the header and message to "a save landed while the tool was busy", or measure the real window: have the tool publish a counter bumped right after its read and cleared after the exchange (this needs a hook in the library, which the unit-level `race_hook` already provides under `cfg(test)` only), or assert on an outcome that needs the race, such as `tool` containing `gave-up` or a non-zero regen count in the naive control.

### IN-09: Exhausting the give-back leaves the newest save at the temp name and an older one at `DIRECTION.md`

**File:** `src/driver/direction.rs:684-709` (`give_back_newest`, the loop exit)
**Issue:** After round N the file at `DIRECTION.md` is the previous `placing` and the file that just came back, which is the newest, sits at the temp name. When all `GIVE_BACK_ROUNDS` (8) rounds see a fresh save, the function returns `Err("newer saves kept arriving; a displaced file was left at ...")` in that state. The message is accurate, but the temp is exactly what `sweep_stale_temps` deletes after 60 s (see WR-10). Eight consecutive racing saves within microseconds is not a realistic input, so this is documentation-level.
**Fix:** On exhaustion, do one more exchange so the newest file ends at `DIRECTION.md` and the older one is what is left at the temp name. Failing that, say in the error that the file will be removed by the next sweep.

---

_Reviewed: 2026-10-24_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
