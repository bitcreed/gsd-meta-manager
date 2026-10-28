---
phase: 30-driver-tab-on-direction-md
reviewed: 2026-10-28T00:00:00Z
depth: standard
files_reviewed: 27
files_reviewed_list:
  - src/action.rs
  - src/app.rs
  - src/cli.rs
  - src/driver/direction.rs
  - src/driver/lock.rs
  - src/driver/mod.rs
  - src/driver/steer.rs
  - src/driverctl_cli.rs
  - src/journal/mod.rs
  - src/journal/reader.rs
  - src/main.rs
  - src/state_reader/driver_frontmatter.rs
  - src/ui/mod.rs
  - src/ui/screens/confirm_popup.rs
  - src/ui/screens/create_project.rs
  - src/ui/screens/detail.rs
  - src/ui/screens/driver.rs
  - src/ui/screens/driver_confirm.rs
  - src/ui/screens/driver_launch.rs
  - src/ui/screens/driver_run.rs
  - src/ui/screens/driver_steer.rs
  - src/ui/screens/help.rs
  - src/ui/screens/mod.rs
  - src/ui/screens/normal.rs
  - src/ui/screens/render_escape_guard.rs
  - src/watcher.rs
  - tests/driver_tab_launch_e2e.rs
  - tests/driverctl_race.rs
findings:
  critical: 0
  warning: 2
  info: 5
  total: 7
status: issues_found
---

# Phase 30: Code Review Report (re-verification at HEAD)

**Reviewed:** 2026-10-28
**Depth:** standard
**Files Reviewed:** 27 (the 27 files in scope; `confirm_popup.rs` and `create_project.rs` are unchanged since `20bb9de` and were not re-read)
**Status:** issues_found

## Summary

Scope: the Phase 30 Driver-tab and DIRECTION.md surface at HEAD, with attention to `git diff 20bb9de HEAD` (Phase 28.1, Phase 31 deletions, quick 261028-n0z and quick 261028-n12). Large files (`app.rs`, `detail.rs`, `driver_run.rs`, `driver_launch.rs`, `render_escape_guard.rs`) were reviewed on the diff and on the Driver-tab and DIRECTION.md paths. Unrelated legacy code was not read.

Checks run:
- `rtk proxy cargo clippy --all-targets` is clean.
- `rtk proxy cargo test --no-fail-fast --lib -- driver:: state_reader::driver_frontmatter` passes (191 tests, including the new n0z/n12 tests).
- `--test driverctl_race` (4 passed, 1 ignored by design) and `--test driver_tab_launch_e2e` (2 passed) pass.

All six prior findings still marked open (WR-08, WR-09, WR-10, IN-06, IN-08, IN-09) are resolved at HEAD. I traced each fix and found no regression in the fix itself (details below). The Phase 31 deletions left no dangling code reference: every `Action` variant is still produced and consumed, and `legacy_*` fields, the `Selection::Legacy` arm and the `L` key are gone with no stray users.

Two new warnings sit in the same attack surface the n0z fixes hardened, and both are gaps the fixes did not reach:

- **WR-11.** The run lock is still probed with an unguarded `File::open`. A FIFO planted at `launcher.lock` parks a `spawn_blocking` worker forever. The fleet-wide marker scan that reaches it runs every ~5 s with no in-flight guard, so the leak is unbounded.
- **WR-12.** The first exchange-back in `run_loop` has the exact `ENOENT` shape WR-10 fixed in `give_back_newest`, and it was left unfixed.

No critical findings.

## Prior findings status

| ID | Status | Evidence at HEAD |
|----|--------|------------------|
| WR-08 | **Resolved** | `src/driver/lock.rs:174-195` opens with `O_NOFOLLOW \| O_NONBLOCK` (`custom_flags`), maps `ELOOP` to `Unavailable`. `:197-218` checks `is_file()` and `nlink() == 1` on the open handle before `flock`, `set_len(0)` (`:274`) or `set_permissions` (`:287`). Reached from `direction.rs:666` (`locked()` via `direction_lock`), so `launcher.lock` gets the same protection. Tests: `lock.rs:515-570` (symlink, dangling symlink, hard link, FIFO) and `steer.rs:1878` (symlinked `DIRECTION.lock` leaves the sentinel's bytes and mode untouched). The `nlink` check goes beyond what the review asked for and is correct. See IN-13 for its one downside. |
| WR-09 | **Resolved** | `src/driver/direction.rs:604-618`: `read_run_file` checks `metadata().len()` on the guarded handle, then reads through `take(CAP + 1)` and re-checks the length. Over-cap is `InvalidData`. `seen()` (`:570-577`) maps that to `bytes: None`. `:843-848` refuses a `Plan::Write` over the cap before any temp exists, which closes the hole where an over-cap tool file would not match `new_content` on the exchange-back. Tests: `direction.rs:1302` and `:1321`, `steer.rs:1911`. See IN-11 and IN-12 for two consequences. |
| WR-10 | **Partially resolved** | `direction.rs:754-776`: `give_back_newest` now handles `ENOENT` with `RENAME_NOREPLACE`, `EEXIST` falls through to another exchange round, and any other errno keeps the temp and says where it is. I traced the `EEXIST` path: the newer file lands in the temp slot, `expect` becomes the previous `placing`, and the next round converges with the newest file in place. `rustix` 1.1.5 maps `NOREPLACE` to `RENAME_EXCL` on Apple (`types.rs:548`). Tests: `direction.rs:1339`, `:1360`. **The same defect remains at the preceding exchange-back (`direction.rs:903-912`); see WR-12.** |
| IN-06 | **Resolved** | `direction.rs:688-694` (`exchange_failure_detail`) and `:878-881` report the portable label "atomic rename exchange" and add a filesystem hint for `NOTSUP` and `INVAL`. Unit tests at `:696-717`. |
| IN-08 | **Resolved** | `tests/driverctl_race.rs:27-32`, `:255-256` and `:470-475`: the header, the sample comment and the assertion message now say "a save started while the tool thread was busy" and name `editor_race_naive_positive_control` as the non-vacuity proof. The assertion itself is unchanged, which is correct. |
| IN-09 | **Resolved (message only)** | `direction.rs:795-803`: the exhaustion error now says the newest save is the one left at the temp, that the next launch's sweep deletes it once it is a minute old, and to copy it out. I confirmed the claim: `sweep_stale_temps` still runs at `launcher/run.rs:702` and `driverctl/serve.rs:940` with `STALE_TEMP_AGE = 60 s` (`serve.rs:117`). No extra exchange was adopted, which is a defensible decision for an unrealistic input. |
| IN-05 | **Deferred, still applies** | Unchanged. The tab numbers findings from the first body line (`app.rs:1236-1239`, `driver_launch.rs`) while `driver check` and the launcher's own refusal use file-relative lines. Nothing in the diff touched it. |
| IN-07 | **Skipped, still applies** | Unchanged. The zero-row-budget popups behave as before (disposition: skipped, unconfirmable popup would return). |

## Warnings

### WR-11: `probe_at` and `read_holder_at` open the run lock without `O_NOFOLLOW` or `O_NONBLOCK`; a FIFO at `launcher.lock` parks a blocking worker forever, and the fleet-wide scan leaks one per tick

**File:** `src/driver/lock.rs:304-312` (`read_holder_at`), `:370-385` (`probe_at`); reached from `src/state_reader/driver_frontmatter.rs:956-960` (`read_driver_run_text`), `src/ui/screens/mod.rs:1676`, `:1790`, `src/app.rs:153-170` (`launcher_markers_for`) and `:1789-1806` (the 20-tick scan)
**Issue:** WR-08's fix hardened the one *write* open of the run lock (`acquire_at`) and left the *read* opens beside it on a bare `File::open(path)` (`O_RDONLY`, blocking, symlink-following). Phase 30 already treats every run-directory name as untrusted: `open_run_file` (`driver_frontmatter.rs:816-869`) exists so that "a FIFO with no writer opens at once instead of parking a `spawn_blocking` worker forever", and `DIRECTION.md`, `LOG.md` and the edit temp all go through it. `read_driver_run_text` opens `DIRECTION.md` through that guard and then calls `probe_at(&run_lock_path(run_dir))` on the sibling `launcher.lock`, which is not guarded.

I verified that `open(O_RDONLY)` on a FIFO with no writer blocks (a 2 s `timeout` around `open("fifo","rb")` exits 124). So a FIFO named `launcher.lock` in any run directory, beside a valid `DIRECTION.md`, hangs the caller. What the caller is matters here:

- `App::update` on `Action::Tick` spawns `launcher_markers_for(&projects, ..)` every 20 ticks, over every registered project and up to `LAUNCHER_RUNS_LISTED` runs each (`app.rs:1768-1806`). The agents scan right below it has an `agents_scan_in_flight` flag; this one has none. Each tick therefore leaks another parked `spawn_blocking` thread (tokio's default pool is 512), and the single parked task also stops that scan from ever refreshing the markers for the other projects in its loop.
- Once the pool is exhausted, every other `spawn_blocking` the app uses (steering, edit, launch, preflight, refresh) queues and never runs. `main.rs:108-111` builds the runtime as a temporary and drops it at the end of the statement. `Runtime::drop` waits for blocking tasks, so quitting the TUI can hang too.
- `launcher::stop::stop_run` (`launcher/stop.rs:102`, `:149`, `:161`), `launcher/push.rs:939` and `launcher/run.rs:1539` call the same two functions, so `driver stop` blocks the same way.

A symlink at the name is followed as well. A cloned repository can commit `.planning/meta-manager/drivers/<id>/launcher.lock` as a symlink (git stores them), and `flock(LOCK_SH)` is then taken on whatever it points at. The result feeds `derive_status`, so a run can be made to read as live or free from a file the project does not own.

Severity is Warning, not Critical, for the same reason WR-08 was: creating a FIFO in the run directory needs a writer there, which is an `--unsandboxed` run (same user), and the symlink case only misleads a status cell. But the phase claims one guarded open for every run-directory read, and this is the one the claim does not cover.
**Fix:** Give both readers the `open_run_file` discipline, and treat anything that is not a regular file as an unknown probe, never as free.
```rust
pub fn probe_at(path: &Path) -> LockProbe {
    let file = match crate::state_reader::driver_frontmatter::open_run_file(path) {
        Ok(file) => file,
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => return LockProbe::Absent,
        // A symlink, FIFO or device at the name: not a lock file we can trust.
        Err(err) if err.kind() == std::io::ErrorKind::InvalidData => {
            return LockProbe::Unknown(rustix::io::Errno::INVAL)
        }
        Err(err) => {
            return LockProbe::Unknown(
                rustix::io::Errno::from_io_error(&err).unwrap_or(rustix::io::Errno::IO),
            )
        }
    };
    /* flock as before */
}
```
`open_run_file` opens read-only, which is what `LOCK_SH` needs, and a regular file ignores `O_NONBLOCK`. Do the same in `read_holder_at`. Add the FIFO and symlink tests beside the existing `lock.rs` ones (`fifo_test_support::{mkfifo, within_5s}`), plus a `read_driver_run_text` test that a FIFO `launcher.lock` yields a view with `LivenessUnknown` instead of blocking. Separately, add an in-flight flag to the marker scan the way `agents_scan_in_flight` does, so one slow filesystem cannot stack tasks.

### WR-12: The first exchange-back in `run_loop` still strands the human's save at a hidden temp on `ENOENT`, the shape WR-10 fixed one step later

**File:** `src/driver/direction.rs:901-912` (the `exchange(&dir, &temp_name, DIRECTION_FILE)` back-exchange in `run_loop`), swept by `src/launcher/run.rs:702` and `src/driverctl/serve.rs:940` (`STALE_TEMP_AGE` = 60 s)
**Issue:** After the tool's file is exchanged in and the displaced file `H` (a human save that landed in the window) is found at the temp name, `run_loop` exchanges back to restore `H`. If `DIRECTION.md` is absent at that instant (an unlink-then-create save, a `git checkout` or `git stash` that removes it), `exchange` fails with `ENOENT` and the code returns:

```rust
return Err(io_err(
    format!("could not exchange back; a displaced file was left at {}", temp_path.display()),
    errno,
));
```

`H` then exists only at `.DIRECTION.md.fm.<pid>.<n>`, `DIRECTION.md` is absent, and the next `sweep_stale_temps` deletes the temp once it is a minute old. This is WR-10's defect in an earlier step of the same loop. Quick 261028-n0z (44eef32) patched `give_back_newest` (`:754-776`) and added a hook point and tests for that step only. The step before it still treats `ENOENT` as a hard error, so the hook set (`BeforeExchange`, `AfterExchange`, `AfterExchangeBack`, `BeforeGiveBack`, `BeforeRestore`) has no test that unlinks at `AfterExchange`. The window is the same size as the one WR-10 closed: it spans the read, hash and decision between the first exchange and this one.

Whether this loses data depends on what follows. If the remover recreates the file, the recreated file supersedes `H`, and nothing worth keeping is lost. If it does not (a checkout that deletes the file), `H` is the user's last save and is gone within a minute. `regen` in `driverctl/store.rs:864-866` swallows the `Err` as `GaveUp`, so no one is told.
**Fix:** Handle `ENOENT` here the way `give_back_newest` does: restore with a no-replace rename, which cannot overwrite a file created in between, and fall back to a retry when something was created.
```rust
if let Err(errno) = exchange(&dir, &temp_name, DIRECTION_FILE) {
    if errno == rustix::io::Errno::NOENT {
        match renameat_with(&dir, temp_name.as_str(), &dir, DIRECTION_FILE, RenameFlags::NOREPLACE) {
            Ok(()) => return Ok(SpliceOutcome::GaveUp), // H restored; the tool's file was unlinked with the old one
            Err(e) if e == rustix::io::Errno::EXIST => { /* something was created: exchange again */ }
            Err(e) => return Err(io_err(/* keep the existing message */ ..., e)),
        }
    } else {
        return Err(io_err(/* existing message */ .., errno));
    }
}
```
Factor the `ENOENT` restore into a small helper shared with `give_back_newest` so the two cannot drift again. Add a `Point::AfterExchange` + `Act::Unlink` unit test that asserts the human's save ends up at `DIRECTION.md` and no temp is left.

## Info

### IN-10: `read_driver_run_text` bounds its read with `take(CAP + 1)` but never checks the result against the cap

**File:** `src/state_reader/driver_frontmatter.rs:939-953`
**Issue:** The size is checked on the open handle (`len > CAP`), then `read_to_string(take(file, CAP + 1))` reads. If the file grows past the cap between the two, the read returns `CAP + 1` bytes of valid text and the function parses and returns it, a silently truncated `DIRECTION.md` shown as if whole. The sibling `read_run_file_capped` (`:882-905`) and `direction.rs::read_run_file` (`:612-616`) both re-check `bytes.len() > CAP` after the read; this reader does not. A cut that lands inside a multi-byte character fails with `InvalidData`, which is the only reason it is not worse.
**Fix:** After `read_to_string`, return the same `FrontmatterError::Io` ("larger than the N-byte limit") when `text.len() as u64 > DIRECTION_READ_MAX_BYTES`. Better, delegate to `read_run_file_capped` and decode the bytes, so there is one capped reader.

### IN-11: `Seen`'s "equal means unchanged" claim no longer holds for over-cap or unreadable files

**File:** `src/driver/direction.rs:558-577` and `:789-793`
**Issue:** The doc on `Seen` says an in-place write into a displaced inode "changes its bytes, so it never compares equal". Since WR-09, `bytes` is `None` for any file that is unreadable or over the cap, so two `Seen`s of the same inode both compare equal on `(dev, ino, None)` however their contents changed. In `give_back_newest` an equal result means "the older displaced file, unchanged: drop it" (`remove_file(temp_path)`). A file that is, and stays, over 1 MiB and is written in place between the two reads would therefore be deleted as unchanged. The input is far outside what the tab can display and the tool refuses to build on it, so the practical risk is small, but the stated invariant is false and the fix is one field.
**Fix:** Keep `len` and `mtime` in `Seen` (both come from the `symlink_metadata` already taken in `seen()`), or treat `bytes: None` as "never equal" (`expect` must be `Some` with `Some(bytes)` to match). Update the doc comment either way.

### IN-12: A write refused for being over the cap reaches the store as an unexplained `splice-gave-up`

**File:** `src/driver/direction.rs:843-848`; consumed at `src/driverctl/store.rs:864-885`
**Issue:** The new over-cap refusal returns `DirectionError::Io`. `Store::regen` maps every `Err` to `SpliceOutcome::GaveUp` and writes a `splice-gave-up` health line, which is documented as "a lost race". For a human-edited `DIRECTION.md` above 1 MiB (or a body so close to the cap that the regenerated tool block pushes it over) every heartbeat regeneration fails, the only trace in `LOG.md` says "gave up", and the Driver tab's frontmatter never updates. The user sees neither the size problem nor its cause. The editor path is separate: `apply_edit` accepts a body up to `EDIT_TEMP_MAX_BYTES` (1 MiB) with no allowance for the tool block, so a body of 1,048,500 bytes passes `read_capped` and is then refused by this check.
**Fix:** Log a distinct health line (for example `direction-too-large`) when the splice error is the cap refusal, and have `apply_edit` compare the edit's body length plus the block length against the cap so it can report "larger than 1 MiB with the tool block" before it asks `write_body`. Define the 1 MiB limit once; `EDIT_TEMP_MAX_BYTES` (`steer.rs:595`) and `DIRECTION_READ_MAX_BYTES` (`driver_frontmatter.rs:814`) are now two constants for one rule.

### IN-13: The hard-link refusal gives a user no way out, and fails on snapshot trees

**File:** `src/driver/lock.rs:210-218`
**Issue:** `acquire_at` now fails with "has N hard links, not exactly one" whenever `nlink != 1`. That is the right answer to a planted link, but a project tree copied with `cp -al` or `rsync --link-dest` (rsnapshot, many backup tools) hard-links every file in it, `DIRECTION.lock` and `launcher.lock` included. From then on every steering write and every launch on those runs fails with that message and no hint. The recovery (delete the lock file and let it be recreated) is safe and simple, but the message does not say so.
**Fix:** Add the remedy to the `Unavailable` detail: "(a copy of this tree made with hard links? remove the file; it is recreated on the next launch)". No behaviour change.

### IN-14: Stale and dangling documentation left by the Linux-to-macOS widening and the Phase 31 deletions

**File:** `src/ui/screens/render_escape_guard.rs:1215` (names `driver_run::push_posture_28_1`, which does not exist; the functions are `push_security_groups` and `push_auth`, `driver_run.rs:150`, `:176`); `src/driver/steer.rs:17-19` ("`write_body`, which is Linux-only and reports `UnsupportedPlatform` elsewhere"); `src/app.rs:1356-1358` ("a build that can write (Linux)") and `:1396-1397` ("not Linux"); `src/ui/screens/detail.rs:11209-11212` (the footer string `"read-only: steering needs Linux or macOS  "` duplicates `STEERING_UNSUPPORTED_PLATFORM` with different trailing spaces instead of deriving from it); `src/journal/reader.rs:97-111` (`tail_lines`, a public function with no caller outside its own tests, still opens by path with a blocking, symlink-following `File::open`, the pattern WR-02 closed; the Driver tab calls `tail_file` on a vetted handle).
**Issue:** None of these changes behaviour, but they are the documentation of the safety rules this phase depends on, and `tail_lines` is a ready-made unguarded reader for the next person who needs one.
**Fix:** Correct the three doc comments, build the footer span from the constant, and delete `tail_lines` (or make it call `open_run_file`).

---

_Reviewed: 2026-10-28_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
