---
phase: 27-driverctl-driver-frontmatter
reviewed: 2026-10-29T00:00:00Z
depth: standard
files_reviewed: 28
files_reviewed_list:
  - src/cli.rs
  - src/driverctl_cli.rs
  - src/driverctl/counters.rs
  - src/driverctl/log.rs
  - src/driverctl/mod.rs
  - src/driverctl/protocol.rs
  - src/driverctl/scan.rs
  - src/driverctl/serve.rs
  - src/driverctl/store.rs
  - src/driver/direction.rs
  - src/driver/lock.rs
  - src/driver/mod.rs
  - src/lib.rs
  - src/main.rs
  - src/state_reader/driver_frontmatter.rs
  - src/state_reader/mod.rs
  - tests/driverctl_client.rs
  - tests/driverctl_frontmatter.rs
  - tests/driverctl_liveness.rs
  - tests/driverctl_lock_race.rs
  - tests/driverctl_race.rs
  - tests/driverctl_replay.rs
  - tests/driverctl_serve.rs
  - tests/driverctl_store.rs
  - tests/fixtures/driverctl/MASTER-PROMPT.md
  - tests/fixtures/driverctl/replay/s002-R2/LOG.md
  - tests/fixtures/driverctl/spike003/r261a/state.json
  - tests/fixtures/driverctl/spike003/run1/LOG.md
findings:
  critical: 0
  warning: 6
  info: 10
  total: 16
status: issues_found
---

# Phase 27: Code Review Report

**Reviewed:** 2026-10-29
**Depth:** standard
**Files Reviewed:** 28
**Status:** issues_found

This is a re-review at current HEAD, after quick batch 261028-t3d (commits 65233b8 and 4977318). It replaces the previous report. Finding ids carry over from the prior report. A finding that is unchanged keeps its id and title, and the two new findings continue the numbering (WR-10 and WR-11). Gaps in the numbering are intentional.

## Summary

**Resolved since the prior review (not re-listed):**

- **WR-03 is fixed.** `serve` and `driver launch` now create `DIRECTION.lock` through `lock::ensure_lock_file`. That function uses `O_NOFOLLOW | O_NONBLOCK`, checks on the open handle that the file is a regular file, and checks that the link count is exactly one. `RunStore::open_with_clock` reads `state.json` through `read_run_file_capped`. A symlink (live or dangling), a FIFO or an over-cap file is `CorruptState`, and the file is left alone. `check_wall_clock_budget` and `stop_orphaned_scope` use the same guarded read. Tests pin FIFO, dangling-symlink and over-cap cases in `lock.rs`, `store.rs` and `tests/driverctl_serve.rs`. I traced the `Err` arm order in `open_with_clock`: `NotFound` takes the fresh-run branch, `InvalidData` takes `CorruptState`, and a dangling symlink gives ELOOP, which maps to `InvalidData`, so it is not mistaken for a missing file.
- **WR-04 is fixed.** `probe_at` and `read_holder_at` now go through the same private `open_lock_file` as `acquire_at`. A symlink, hard link, FIFO or directory is `Unknown(EINVAL)` in the probe and `None` in the holder read, and the open does not block. The holder read is capped at 4096 bytes. I found no remaining unguarded opens of `launcher.lock` or `DIRECTION.lock` in `src/`.
- **The direction.rs change (`exchange_or_restore`) holds up.** It shares one put-back helper between `give_back_newest` and `run_loop`. Its `EEXIST` loop is bounded by `GIVE_BACK_ROUNDS`. In `run_loop`, `PutBack::Restored => continue` stays inside `SPLICE_TRIES`. The `EEXIST` then exchange path still reaches `give_back_newest` with a consistent `expect`.

**Still open:** WR-01, WR-02, WR-05, WR-06 and IN-01..IN-10 are unchanged at HEAD. I re-checked their line numbers, and WR-05 and IN-09 moved slightly.

**New:**

- **WR-10:** the same class as WR-03, on a different file. `LOG.md` is appended with a plain `OpenOptions` and no `O_NOFOLLOW` or `O_NONBLOCK`.
- **WR-11:** the new 1 MiB read cap on `state.json` has no matching write bound.

No Critical issues.

## Warnings

### WR-01: `driverctl log` rejects any value that starts with `-`, where the argparse client accepted it

**File:** `src/cli.rs:426-459`
**Issue:** The 17 flags of `DriverctlLog` are plain `#[arg(long)] Option<String>`. clap treats a value that begins with `-` as another flag. The spike's argparse client accepted any such value that contains a space, so a driver prompt that worked under it now fails with rc 2 before the server sees it.

Reproduced against `target/debug/gsd-meta-manager`:

- `driverctl log status --item I-01 --state done --outcome "- note"` gives `error: unexpected argument '- ' found`, rc 2.
- `--outcome "-5 failing"` fails the same way.
- `dispatched ... --round -1` fails the same way.

`outcome`, `evidence`, `question`, `reason` and `step` are free text a driver LLM writes, and a leading `- ` or `-N` is plausible. The server's `one_line` accepts these values, so only the client is stricter. The comment at `cli.rs:415` says the client must be a pass-through ("adding a `value_parser` would be a second judge"), and this is the same kind of second judge. The failure is reported as a clap usage error, not a `REJECTED:` line.
**Fix:**
```rust
#[arg(long, allow_hyphen_values = true)]
pub outcome: Option<String>,
// ...repeat for every one of the 17 value flags (or set it on the Command via
// #[command(allow_hyphen_values = true)] on DriverctlLog's Args derive).
```
Add a client test that sends `--outcome "- note"` and expects the recorded request to carry it.

### WR-02: A failed `FileWatcher::new` leaves the terminal in raw mode on the alternate screen

**File:** `src/main.rs:457-472`
**Issue:** `tui::init()` (line 457) enters raw mode and the alternate screen. `FileWatcher::new(event_bus.tx.clone())?` at line 472 comes after it and returns early on error. `FileWatcher::new` returns `anyhow::Result`, and it fails when inotify instances are exhausted. The `?` skips `tui::restore()`, so the shell is left raw and the error text is drawn on the alternate screen.

The comments at `main.rs:401-405` and `main.rs:546-548` say all fallible startup `?` calls precede `tui::init()`. That is not true of this one.
**Fix:** Construct the watcher before `tui::init()`:
```rust
let mut watcher = FileWatcher::new(event_bus.tx.clone())?;   // move above tui::init()
let mut terminal = tui::init();
```
Alternatively wrap the startup in a block that calls `tui::restore()` on `Err`.

### WR-05: `serve` shutdown overwrites a driver's `paused` end with `stopped`

**File:** `src/driverctl/store.rs:1386`, reached from `src/driverctl/serve.rs:1008`
**Issue:** `set_terminal` keeps a driver `RUN end` only when `ended_by_driver && status.is_terminal()`. `RunStatus::is_terminal()` is false for `Paused` (`driver_frontmatter.rs:152-161`). After `end --reason pause`, a SIGTERM to `driverctl serve` therefore rewrites the status to `stopped` with reason `serve:signal` and discards the driver's `paused`.

The module docs (`serve.rs` fact 8) say the stop is written "unless the driver logged `RUN end`". The WR-02 comment in `set_terminal` justifies the exception only for `fresh-exit` (`Running`). `derive` treats `paused` as a clean end that should read as written, so serve's own shutdown contradicts it.
**Fix:** Keep every driver end except the `fresh-exit` one:
```rust
let driver_end_is_final = self.state.ended_by_driver
    && (self.state.status.is_terminal() || self.state.status == RunStatus::Paused);
if driver_end_is_final { return Ok(TerminalOutcome::KeptDriverEnd); }
```
Add a store test: `end --reason pause`, then `set_terminal(Stopped, ...)`, and expect `KeptDriverEnd` with status `paused`.

### WR-06: Log-directory fallback to `/tmp` is a predictable path in a shared directory

**File:** `src/main.rs:28-30`
**Issue:** `init_tracing` falls back to `PathBuf::from("/tmp")` when `dirs::data_local_dir()` is `None`. `RollingFileAppender` then opens `/tmp/gsd-meta-manager.log.<date>` with create and append. The path is fixed, and the open does not refuse a pre-created symlink or a file owned by another user. Another local user can plant a symlink there, and the process then appends log text to the link target.

`serve.rs` refuses a `/tmp` fallback for its socket for exactly this reason ("a shared directory is exactly where a squatter pre-creates the path"). The comment at `main.rs:15-24` is careful about failing without a panic, but not about this path.
**Fix:** Drop the fallback and return, installing no subscriber, as the builder-error branch already does:
```rust
let Some(base) = dirs::data_local_dir() else { return };
let log_dir = base.join("gsd-meta-manager");
```

### WR-10: `LOG.md` is appended with an unhardened open (follows symlinks, blocks on a FIFO) under the store mutex

**File:** `src/driverctl/log.rs:91-101`, called from `src/driverctl/store.rs:796` and `src/driverctl/store.rs:904`; the read side is `src/launcher/run.rs:1074` (outside this phase's list)
**Issue:** Quick 261028-t3d hardened `DIRECTION.lock`, `state.json` and the lock probes, but `LOG.md` is the other run-directory name the run opens, and it is still opened as `OpenOptions::new().append(true).create(true)` with no `O_NOFOLLOW` and no `O_NONBLOCK`, and no type check on the handle.

- **Symlink:** a symlink at `LOG.md` (live, or dangling with `create`) is followed. `serve` runs outside the sandbox with the user's full rights, so every `commit` appends the event text, including driver-written fields such as `outcome`, to the link target. The LOG line format is only partly controlled, so this is an append-only write into a file the run does not own.
- **FIFO:** a write-only open of a FIFO that has no reader blocks in `open(2)`. `commit` calls `log::append` while holding the store mutex, so the first `begin_launch` (or the first commit after the plant) parks the whole server: every client connection then waits on the mutex, and the dispatch gate cannot answer.
- **Read side:** `finish_launch` does `fs::read_to_string(run_dir/LOG.md)` while holding the store lock. It follows links, blocks on a FIFO and is unbounded.

A sandboxed driver cannot plant here, because the run directory is read-only bound inside bwrap. The reachable cases are an `--unsandboxed` run and a leftover plant from an earlier unsandboxed run, which is the same scenario `driver_frontmatter.rs:810-814` states for the other run-directory names. For that reason I rate this the same as WR-03 and WR-04 (Warning) [inferred — cleanup 2026-10-28].
**Fix:** Open `LOG.md` through a shared guarded open, the same way `lock::open_lock_file` does:
```rust
opts.append(true).create(true).mode(0o644)
    .custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32 | rustix::fs::OFlags::NONBLOCK.bits() as i32);
let f = opts.open(path)?;
if !f.metadata()?.file_type().is_file() { /* InvalidData "not a regular file" */ }
```
Map ELOOP to `InvalidData`, as `open_run_file` does. The write can then fail without blocking, and `commit` already rolls `state.json` back on a failed append. Read `LOG.md` in `finish_launch` through `read_run_file_capped`. Add a FIFO test with `within_5s` and a symlink test with a sentinel target, as the new lock tests do.

### WR-11: The new 1 MiB read cap on `state.json` has no matching write bound, so a large run can write a `state.json` it cannot reopen

**File:** `src/driverctl/store.rs:650` (read, capped) against `src/driverctl/store.rs:742-763` (write, unbounded); also `src/launcher/stop.rs:203`
**Issue:** The WR-03 fix reads `state.json` through `read_run_file_capped`, whose limit is `DIRECTION_READ_MAX_BYTES` (1 MiB). Before this change the read had no limit. `write_state` still has none. State growth that is not bounded by the file itself includes `ItemRecord.findings`, which gets one `i64` appended for every report that carries `FINDINGS: N` (`store.rs:1752`) with no cap, and `RunState.items`, which has one entry per DIRECTION item.

Once the serialised state passes 1 MiB, the running server goes on writing it without complaint. The next `RunStore::open` (a relaunch, a resume) returns `CorruptState`, and the file is "never overwritten", so the run cannot be reopened until someone trims the file by hand. `stop_orphaned_scope` reads the same file through the capped read, so `driver stop` can no longer find the scope to stop for such a run. The pre-check `check_wall_clock_budget` treats an over-cap file as zero recorded time, so it hides the wall-clock use too.

The practical likelihood is low: it takes tens of thousands of reports or thousands of items. The failure is also silent until the next open, and the asymmetry is new in this batch, so I count it as a regression risk of the fix rather than a pre-existing issue. Severity is an inferred judgement [inferred — cleanup 2026-10-28].
**Fix:** Make the two sides agree. Either refuse the write that would exceed the cap, in `write_state`:
```rust
if json.len() as u64 > DIRECTION_READ_MAX_BYTES {
    return Err(StoreError::Io { detail: "state.json would exceed the 1 MiB read cap".into() });
}
```
(`commit` already restores the previous state on failure, so the event is refused cleanly), or use a separate, larger `STATE_READ_MAX_BYTES` for `state.json` and bound `findings` (keep the last N). Add a store test that a state written at the cap reopens.

## Info

### IN-01: Wrong binary name in the duplicate-alias hint

**File:** `src/main.rs:161`
**Issue:** The message says `Provide an explicit alias: gsd-manager add ...`. The binary is `gsd-meta-manager`, and line 136 uses the right name. A user who pastes the hint gets "command not found".
**Fix:** Change `gsd-manager` to `gsd-meta-manager`.

### IN-02: A 447-byte DIRECTION.md frontmatter costs about 0.8 s of CPU per parse

**File:** `src/state_reader/driver_frontmatter.rs:488-498`
**Issue:** `parse_driver_frontmatter` parses the block twice, once for `VersionProbe` and once for the full schema. The YAML backend is now `noyalib`, behind the `serde_yml 0.0.13` shim. A nested-alias "lol" document of 447 bytes ended in `alias expansion limit exceeded` after about 0.8 s in a release build and 2 s in a debug build, measured in a scratch crate. The error is typed, so T-27-10 holds. But a hostile `DIRECTION.md` in an `--unsandboxed` run costs a `spawn_blocking` worker about 1.6 s per refresh per run row.
**Fix:** Cheap guard before parsing. The emitter never writes anchors or aliases, so refuse a block containing an unquoted `&`/`*` token. Or parse once by folding `driver_frontmatter_version` into the main struct and checking it after.

### IN-03: Module docs cite a RUSTSEC advisory in a parser the crate no longer uses

**File:** `src/state_reader/driver_frontmatter.rs:54-58`, `Cargo.toml:43`
**Issue:** The docs justify hand-written emission with "RUSTSEC-2025-0068 sits in its emitter path". `serde_yml 0.0.13` is a deprecated compatibility shim over `noyalib` (its `lib.rs` carries `#![deprecated]`), and the libyml chain is gone. The rationale is stale. Every call site also compiles against a deprecated crate.
**Fix:** Update the paragraph. Plan a move to `noyalib` or `serde-saphyr` directly.

### IN-04: `fm_status` regex lets `\s*` cross a newline

**File:** `src/driverctl/counters.rs:138-139`
**Issue:** `(?m)^status:\s*["']?([A-Za-z_]+)` allows `\s*` to span lines. A frontmatter line `status:` with an empty value followed by `gaps_found: x` captures `gaps_found`, and `status:\nfoo: 1` captures `foo`. Real files are unlikely to hit this, but the counter is the cap the launcher enforces and the intent is "the value on this line".
**Fix:** Use `[ \t]*` instead of `\s*`. If the spike's regex was identical, record the deviation next to D-27.

### IN-05: The lock module's "worst case is a misleading string" claim is no longer true

**File:** `src/driver/lock.rs:87-96`, with `src/launcher/stop.rs:134-147`
**Issue:** The `LockHolder` docs say a forged record "can never grant a second lock, so the worst outcome of tampering is a misleading string (T-17-08)". `driver stop` now takes `holder.pgid` from that record and sends `SIGTERM` to it. In a bwrap run the file is bound to `/dev/null`, so this only matters for `--unsandboxed` runs, where the driver already runs as the same user. The stop path does verify `probe_at == Held` first, but that does not tie the pid to the lock holder. The `read_holder_at` docs added by t3d (`lock.rs:358-360`) already say the record names the pgid `driver stop` signals, so the two doc comments in the same file now contradict each other. The t3d change does keep a symlinked or hard-linked file from supplying the record.
**Fix:** Amend the `LockHolder` doc to say the record is a signal target for `driver stop`. Optionally verify before signalling that `/proc/<pid>` still holds the lock file open.

### IN-06: Stale references to the deleted `drive` command and goal layer

**File:** `src/cli.rs:530`, `src/state_reader/mod.rs:87` and `src/state_reader/mod.rs:977`
**Issue:**

- A test message mentions "the TUI's `drive` respawn".
- Two comments mention a "legal driver goal target" and "the DRIVE-05 gate set". Phase 31 removed both concepts, and `disk_status.rs` still cites DRIVE-05.

The test message and comments now describe code that does not exist.
**Fix:** Reword to current behaviour, or delete the clauses.

### IN-07: Report-body handling: Unicode line separators allowed, stdin unbounded, over-1-MiB reports rejected whole

**File:** `src/driverctl/protocol.rs:433-440`, `src/driverctl_cli.rs:416`, `src/driverctl/serve.rs:729-737`
**Issue:**

- **Line separators:** `report_body` rejects `char::is_control` characters but allows U+2028 and U+2029. `one_line` and the launcher-side `header_safe` treat U+2028/U+2029 as line breaks (U+2028/U+2029 are not `is_control`). `parse_log` splits on `\n` only, so nothing forges an event today. It is an inconsistent rule, and the doc comment on `one_line` says some readers treat them as breaks.
- **Unbounded stdin:** the client reads stdin with an unbounded `read_to_end`, while the server caps a request at 1 MiB. A worker report whose JSON exceeds 1 MiB is therefore refused with `request too large`, so the 15-line truncation never applies and the event is lost.

**Fix:**

- Apply the same `unsafe_char` predicate in `report_body`.
- Cap the client's stdin read, for example at 512 KiB. Keep the first lines and flag truncation, or fail with a clear message.

### IN-08: `read_driver_run_text` does not re-check size after reading and fails on non-UTF-8

**File:** `src/state_reader/driver_frontmatter.rs:939-953`
**Issue:** The size is checked on the handle before reading. After `take(CAP+1).read_to_string` there is no post-read length check, unlike `read_run_file_capped` (lines 901-903), so a file that grew past the cap during the read returns a 1-byte-over buffer as success. `read_to_string` also errors on non-UTF-8, so a `DIRECTION.md` with one stray byte reads as an unreadable row. The store reads the same file with `from_utf8_lossy` and carries on.
**Fix:** Reuse `read_run_file_capped` and decode with `String::from_utf8_lossy`.

### IN-09: Test reliability notes

**File:** `tests/driverctl_client.rs:32`, `tests/driverctl_replay.rs:45`, `tests/driverctl_lock_race.rs:163-190`, `tests/driverctl_serve.rs:872-880`
**Issue:**

- **Unwritable data dir:** the T-27-11 guard (`XDG_DATA_HOME=/proc/nonexistent-gsdmm`) only has an effect where `dirs` honours `XDG_DATA_HOME`, which is Linux. On macOS the variable is ignored, so these tests pass without exercising an unwritable data dir.
- **Lock budget race:** `real_holder_is_still_refused_within_budget` has the holder release after 800 ms while the budget is 500 ms. A 300 ms scheduler stall makes the second acquire succeed.
- **Flood test:** `a_flood_of_idle_connections...` waits 500 ms for 32 accepts to land, a fixed sleep that is load-sensitive.

**Fix:**

- Set `HOME` to the unwritable path as well, or assert with a Linux gate.
- Hold the lock until the second attempt finishes (join on a channel) instead of a fixed 800 ms.
- Poll `/proc/net/unix` or retry the client until the `busy` answer appears.

### IN-10: Committed fixtures carry a real username, hostname and session id

**File:** `tests/fixtures/driverctl/spike003/r261a/state.json:4,68-90` (and the run1 and s002-R2 fixtures)
**Issue:** The fixtures contain `/home/blk/.local/state/...` project paths, the hostname `mediabox` and a real Claude `session_id`. The repository is public. This is low impact, but it is identifying data in a published test fixture.
**Fix:** Replace the user and host with neutral values (`/home/user`, `testhost`) and regenerate the affected replay expectations if any depend on them.

---

_Reviewed: 2026-10-29_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
