---
phase: 31-retire-superseded-envelope-goal-layer-code
reviewed: 2026-10-28T00:00:00Z
depth: standard
files_reviewed: 31
files_reviewed_list:
  - Cargo.toml
  - scripts/launcher-acceptance.sh
  - src/app.rs
  - src/config.rs
  - src/driver/direction.rs
  - src/driver/kill.rs
  - src/driver/liveness.rs
  - src/driver/lock.rs
  - src/driver/rate_limit.rs
  - src/driver/steer.rs
  - src/driver_cli.rs
  - src/driverctl/serve.rs
  - src/driverctl/store.rs
  - src/envelope/scan.rs
  - src/executor/stream_json.rs
  - src/journal/redact.rs
  - src/launcher/mod.rs
  - src/launcher/process.rs
  - src/launcher/run.rs
  - src/session_detector.rs
  - src/state_reader/driver_frontmatter.rs
  - src/state_reader/git_ops.rs
  - src/ui/screens/detail.rs
  - src/ui/screens/driver_launch.rs
  - src/ui/screens/driver_run.rs
  - src/ui/screens/driver_steer.rs
  - src/ui/screens/mod.rs
  - tests/driverctl_race.rs
  - tests/driverctl_store.rs
  - tests/launcher_live_isolation.rs
  - tests/spawn_seam_guard.rs
findings:
  critical: 0
  warning: 3
  info: 5
  total: 8
status: issues_found
---

# Phase 31: Code Review Report

**Reviewed:** 2026-10-28
**Depth:** standard
**Files Reviewed:** 31
**Status:** issues_found

## Summary

Scope was the diff `1cb1cd3..HEAD` over the listed files, read in context. Most of the change is
documentation retirement (executor references), the Linux-to-Linux-or-macOS `cfg` widening, and a
set of hardening fixes (lock-file symlink/FIFO/hard-link refusal, capped `DIRECTION.md` reads,
no-replace socket publish, no-replace give-back restore, session-id option-shaped successor rule).
`cargo test --lib --no-fail-fast` passes (2551 passed, 0 failed), so the findings below are logic
gaps and incomplete hardening, not test failures. No security vulnerability or data-loss defect was
proven; three warnings concern hardening that is narrower than its own documentation claims.

## Warnings

### WR-01: Lock hardening covers `acquire_at` only; `probe_at` and `read_holder_at` still follow symlinks and block on a FIFO

**File:** `src/driver/lock.rs:304-307` and `src/driver/lock.rs:370-379`
**Issue:** The `acquire_at` doc (lines 149-154) says a symlink, FIFO or hard link at the name is refused
and that "`launcher.lock` goes through this same function and gets the same protection". The read-side
functions for the same file were not touched. `probe_at` uses a bare `File::open(path)`, which
follows a symlink and blocks indefinitely on a FIFO with no writer. `read_holder_at` does the same.
`probe_at` is on the display path (`state_reader/driver_frontmatter.rs:959`, `read_driver_run`), and
also in `launcher/stop.rs:102,149,161`, `launcher/run.rs:1539` and `launcher/push.rs:939`. A FIFO at
`launcher.lock` therefore parks a `spawn_blocking` worker on every refresh. A symlink at the name
(which a cloned repository can supply) makes `probe_at` take `LOCK_SH` on the symlink target and
report `Held` when an unrelated process holds a lock on it. The sibling `DIRECTION.md` readers were
moved onto `open_run_file` in this same change, so the lock file is the one run-dir entry left
behind.
**Fix:** Open both read paths with `O_NOFOLLOW | O_NONBLOCK` and check `is_file()` on the handle, as
`open_run_file` does:
```rust
fn open_lock_file_readonly(path: &Path) -> std::io::Result<File> {
    use std::os::unix::fs::OpenOptionsExt;
    let flags = (rustix::fs::OFlags::NOFOLLOW | rustix::fs::OFlags::NONBLOCK).bits();
    let file = OpenOptions::new()
        .read(true)
        .custom_flags(i32::try_from(flags).unwrap_or(0))
        .open(path)?;
    if !file.metadata()?.file_type().is_file() {
        return Err(std::io::Error::new(std::io::ErrorKind::InvalidData, "not a regular file"));
    }
    Ok(file)
}
```
Map `InvalidData` to `LockProbe::Unknown` in `probe_at` and to `None` in `read_holder_at`. Otherwise
narrow the `acquire_at` doc so it does not claim the whole lock file is protected.

### WR-02: Socket publish via `hard_link` has no fallback on filesystems without hard-link support

**File:** `src/driverctl/serve.rs:584-585`
**Issue:** `bind_private` replaced `rename` with `fs::hard_link(&staged, path)` to get no-replace
semantics. `rename` works on every filesystem that can hold a socket. `link(2)` fails with
`EPERM`, `ENOTSUP` or `EMLINK` on filesystems that lack hard links (some FUSE/network mounts,
vfat/exFAT, certain bind-mounted volumes on macOS and WSL). On those, the launcher can no longer
bind its control socket and every launch fails with only "publish the socket at ...". There is no
distinct message and no retry on a different strategy. The change also adds a portability regression
to the same commit series that widens the launcher to macOS.
**Fix:** Keep `link` as the primary no-replace path. On `EPERM`/`ENOTSUP`/`EMLINK`/`EXDEV`, either
fall back to a no-replace `renameat_with(..., RenameFlags::NOREPLACE)` (the primitive
`driver/direction.rs` already uses), or return an error that names the cause: "the filesystem under
{parent} does not support hard links; set --sock to a path on another filesystem".

### WR-03: The first exchange-back in `run_loop` still fails on an absent `DIRECTION.md` (WR-10 fix is incomplete)

**File:** `src/driver/direction.rs:904-914`
**Issue:** WR-10 teaches `give_back_newest` to handle `ENOENT` from the exchange with a
`RENAME_NOREPLACE` restore. The earlier exchange-back in `run_loop` ("A human save raced us: give
their inode back") has the same exposure. A human's unlink-then-create save, or a checkout, can
remove `DIRECTION.md` between the first exchange and this call. The `exchange` then returns `ENOENT`
and the function returns `Err("could not exchange back; a displaced file was left at ...")`. The
splice is lost and surfaced as an error, and the displaced file is left at the temp name for
`sweep_stale_temps` to delete about 60 seconds later. The new test
`a_give_back_that_finds_direction_md_unlinked_restores_the_newest_save` exercises only the second
site, so this one is unpinned.
**Fix:** Factor the `NOENT` branch of `give_back_newest` into a helper (exchange, and on `ENOENT` a
no-replace rename of the temp into `DIRECTION.md`, continuing on `EXIST`) and call it from both
sites. Add a race-hook test with `Act::Unlink` at `Point::AfterExchange`.

## Info

### IN-01: Two copies of the capped, guarded reader with divergent messages

**File:** `src/driver/direction.rs:604-619` and `src/state_reader/driver_frontmatter.rs:882-905`
**Issue:** `read_run_file` (direction.rs) and `read_run_file_capped` (driver_frontmatter.rs) are the
same algorithm: open, check size on the handle, `take(cap + 1)`, check length. They differ only in
the error text. direction.rs hard-codes "larger than 1 MiB", while the shared helper formats
`DIRECTION_READ_MAX_BYTES`. Two tests assert the literal string, so a change to the constant leaves
a wrong message.
**Fix:** Have `direction.rs::read_run_file` call `read_run_file_capped`, and assert on the shared
error text.

### IN-02: Stale "Linux-only" comments after the macOS widening

**File:** `src/app.rs:2251`, `src/app.rs:2412`, `src/app.rs:6086`, `src/ui/screens/detail.rs:20554`, `src/ui/screens/driver_run.rs:2841-2842`, `src/driver/steer.rs:18`
**Issue:** The `cfg` gates on these items were widened to `any(linux, macos)`, but the adjacent
comments still say "Linux-only" or "Linux only". `driver_run.rs:2841` says creating a run and
editing its body "are Linux-only writes", which now contradicts `can_steer()`.
**Fix:** Reword to "Linux and macOS" or reference `can_steer()`.

### IN-03: A planted FIFO, symlink or oversized `DIRECTION.md` is reported to the operator only as "invalid data"

**File:** `src/driverctl/store.rs:715` (surfaced at `store.rs:1121`, `1241`)
**Issue:** `direction_read` maps every non-`NotFound` error with `e.kind().to_string()`. The new
guards all produce `ErrorKind::InvalidData`, so the dispatch denial reads "DIRECTION.md is
unreadable (invalid data)" for a FIFO, a symlink and a file over 1 MiB alike. An operator whose
`DIRECTION.md` simply grew past the cap gets a stalled run and no hint why.
**Fix:** Pass the message through: `Err(e) => return Err(format!("{}: {e}", e.kind()))`, which keeps
the kind and adds "not a regular file (...)" or "larger than the N-byte limit".

### IN-04: Brittle hard-coded census counts in session-detector tests

**File:** `src/session_detector.rs:2220-2250` (`CLAUDE_ARGV_SITES` counts 31 and 7), `src/ui/screens/detail.rs` (`split_round_tripped == 20`, `split_option_shaped == 8`)
**Issue:** The tests pin exact literal-site counts and corpus partition sizes. Any unrelated edit
that adds one option literal or one fixture fails them, and the comment trail already records two
manual re-measurements (23 to 31, 6 to 7). They guard a real property but make the failure noise
proportional to churn.
**Fix:** Derive the partition counts from the corpus (`corpus.len() == a + b`, both non-zero). Keep
the per-file site counts if the adjudication is intended, but document the bump procedure in the
test failure message.

### IN-05: Production code left with no production caller, and garbled doc prose in `config.rs`

**File:** `src/state_reader/git_ops.rs:108` (`head_sha`), `src/registry.rs:897` (`check_prompt_input_drift`), `src/config.rs:281-285`, `src/config.rs:322-326`
**Issue:** `head_sha` is now called only from tests (the doc says so and points at a todo).
`check_prompt_input_drift` and `OptInDrift` have no production caller, so the prompt-input drift
re-confirmation recorded at opt-in is enforced nowhere on the launcher path (documented, but it is
a retired safeguard, not a mere cleanup). The edited `config.rs` doc comments splice retirement
caveats into the middle of sentences ("which re-confirmed (the retired executor gate only), which
was correct: a record...") and leave very long lines.
**Fix:** Track removal of the two functions in the same todo. Rewrite the `DriverOptIn` doc as one
paragraph stating that drift detection currently has no enforcement point.

---

_Reviewed: 2026-10-28_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
