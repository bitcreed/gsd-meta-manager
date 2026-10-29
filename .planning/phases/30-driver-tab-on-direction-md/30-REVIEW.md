---
phase: 30-driver-tab-on-direction-md
reviewed: 2026-10-28T00:00:00Z
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
  warning: 1
  info: 3
  total: 4
status: issues_found
---

# Phase 30: Code Review Report

**Reviewed:** 2026-10-28
**Depth:** standard
**Files Reviewed:** 12
**Status:** issues_found

## Summary

Reviewed the diff since `5cd877b` for the twelve files, with the surrounding code read for context.
No cargo build or test was run, as instructed.

**WR-11 (lock.rs probe/holder opens): fix verified, no defect.**
- `probe_at` and `read_holder_at` now go through the same `open_lock_file` as `acquire_at`: `O_NOFOLLOW | O_NONBLOCK`, a type check and link-count check on the open handle, and a 4096-byte cap on the holder read.
- A symlink, hard link, FIFO or directory is `Unknown(EINVAL)` or `None`, and none of them blocks.
- The type check runs before `flock`, so a FIFO never reaches it.
- The callers I traced (`stop.rs`, `push.rs`, the detach loop in `run.rs`, `read_driver_run_text`) all treat `Unknown` and `None` fail-closed.
- `O_NONBLOCK` stays on the descriptor held in `RunLock`. That is harmless on a regular file.

**WR-12 (direction.rs exchange-back): fix verified, no defect.** I traced these cases through `exchange_or_restore`:
- `DIRECTION.md` absent at the exchange-back: `Restored`, then `continue` re-reads the restored save.
- `EEXIST` on the no-replace restore: the helper exchanges again, then `give_back_newest` fixes the ordering. The older save is back at `DIRECTION.md`, the newer one is at the temp name, and the next round swaps them.
- Exhaustion and non-`NOENT` errors keep the displaced temp.
- `Restored` consumes one of `SPLICE_TRIES`, which is bounded.

No file is overwritten or unlinked on any path.

The other changes are correct as written:
- The guarded `state.json` reads in `store.rs`, `stop.rs` and `run.rs`. The wall-clock pre-check treating a refused file as zero time is covered by the store's later refusal. `connections` and `items` are bounded, so the 1 MiB cap on `state.json` is not a realistic regression.
- `ensure_lock_file` in `serve.rs` and `run.rs`.
- The `("hooks" | "permissions", _) => {}` arm. The arm order is right and nothing non-object reaches the catch-all.

One warning and three infos follow. None of them re-raises IN-10 to IN-14.

## Warnings

### WR-13: `managed_settings` still copies non-array nested host values verbatim, one level below the WR-08 fix

**File:** `src/launcher/config_dir.rs:593-616`
**Issue:** The WR-08 / 29-REVIEW WR-02 fix stops a non-object host `hooks` or `permissions` from replacing the gate. Its own rationale is that "the safety tier must not depend on Claude Code refusing such a file". One level down, the same hole remains.
- For `("hooks", Object)`, an event whose value is not an array goes through `ours.entry(event).or_insert_with(|| other.clone())`. That is also how a non-array `permissions.<list>` is handled.
- For any event or list name the launcher does not already hold, that inserts the malformed value into the managed file. Examples are `{"hooks": {"Notification": "x"}}` and `{"permissions": {"allow": "x"}}`.
- The managed file then carries a schema-invalid `hooks` or `permissions` block next to the gate hooks and the deny list.
- If Claude Code rejects or drops an invalid managed file or block, the gate and the deny list go with it. That is the failure mode WR-08 closed at the top level.
- IN-03's guard only protects names the launcher already holds (`deny`, `PreToolUse`, `Stop`).
- The new tests (`non_object_shapes`, `a_managed_seed_keeps_the_gate_...`) only vary the top-level `hooks` / `permissions` value, so this is not pinned.
- Whether Claude Code drops the whole file or only the bad block is unverified. The code-level hole is certain, and the project's own rule says to close it.

**Fix:** Drop non-array entries instead of inserting them:
```rust
for (event, entries) in events {
    if let Value::Array(entries) = entries {
        append_unique(ours, event, entries);
    }
    // a non-array event value is ignored, like a non-object `hooks`
}
```
Apply the same change to the `permissions` lists. Extend `non_object_shapes` coverage to `{"hooks": {"Notification": <non-array>}, "permissions": {"allow": <non-array>}}`, and assert the output keeps only array-valued, launcher-known entries.

## Info

### IN-15: `ensure_lock_file` documents a directory at the name as `InvalidData`, but it is `EISDIR`

**File:** `src/driver/lock.rs:216-226`
**Issue:** The doc says a symlink, hard link, FIFO "or a directory at the name is `InvalidData`". `ensure_lock_file` opens read+write, so `open(2)` on a directory fails with `EISDIR` (`ErrorKind::IsADirectory`) before the type check runs.
- The read-only callers (`probe_at`, `read_holder_at`) do reach the type check for a directory. That is probably where the claim came from.
- The serve / launch message therefore reads "Is a directory (os error 21)" rather than "is not a regular file".
- No test covers the directory case, so the doc and the behavior can drift unnoticed.

**Fix:** Either reword the doc to "a directory is refused (`EISDIR`)" or map `IsADirectory` to the same `InvalidData` "is not a regular file" message inside `open_lock_file`. Add a directory test beside the FIFO one.

### IN-16: `probe_at` flattens every guard refusal to `Unknown(EINVAL)`, so `driver stop` and `push` print "Invalid argument"

**File:** `src/driver/lock.rs:438-446` (callers `src/launcher/stop.rs:105`, `src/launcher/push.rs:942`)
**Issue:** `open_lock_file` words a precise reason: "is a symbolic link", "has 2 hard links, not exactly one" or "is not a regular file". `probe_at` discards that text and returns `Unknown(Errno::INVAL)`.
- `stop_run` and the push path print only the errno: "could not probe the run lock: Invalid argument (os error 22)".
- A user with a hard-linked or symlinked `launcher.lock` is told nothing actionable. That compounds IN-13's "no way out".
- `acquire_at` does surface the full text through `LockError::Unavailable`, so the read side is the inconsistent one.

**Fix:** Keep `LockProbe::Unknown(Errno)` for compatibility and add a refusal variant that carries the text, for example `LockProbe::Refused(String)`. Have `stop.rs` and `push.rs` report that text. Alternatively, log the `InvalidData` message at the call site before collapsing it.

### IN-17: `direction.rs` `read_run_file` duplicates `read_run_file_capped`, with a different error text

**File:** `src/driver/direction.rs:605-621`
**Issue:** `imp::read_run_file` re-implements `driver_frontmatter::read_run_file_capped`: open through `open_run_file`, check the handle's length, then `take(CAP + 1)`, then check the length again. The only difference is the message ("file is larger than 1 MiB" versus "<path>: larger than the N-byte limit"). Two copies of a security-relevant bounded read can drift, and the next cap or guard change has to land twice.

**Fix:** Delete the local copy and call `crate::state_reader::driver_frontmatter::read_run_file_capped`, which already carries the path in its message. If the exact text matters to a test, adjust that test instead.

---

_Reviewed: 2026-10-28_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
