---
phase: 27-driverctl-driver-frontmatter
reviewed: 2026-10-28T00:00:00Z
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

**Reviewed:** 2026-10-28
**Depth:** standard
**Files Reviewed:** 28
**Status:** issues_found

This is a re-review at current HEAD. It replaces the earlier round's report. Phase 31 deleted `drive` and the goal layer, and Phases 28, 28.1 and 30 plus the cleanup batches have since changed these files.

## Summary

The core is sound. The single-writer store, the validate-then-apply commit order, the run-lock-by-type design, the exchange-verify splice and the fail-closed hook client all hold up under tracing. I found no correctness defect that loses data or opens the dispatch gate.

Two findings were reproduced against the built binary or a scratch crate rather than inferred:

- **WR-01:** clap rejects hyphen-leading values on `driverctl log`.
- **IN-02:** the YAML alias-expansion cost.

Most of the warnings share one cause. The WR-08 and WR-02 hardening (`O_NOFOLLOW | O_NONBLOCK`, size cap) was applied to `acquire_at`, the scan and the DIRECTION reader, but not to the remaining opens of run-directory files. Those opens are the lock probe, the holder read, the `state.json` read at store open, and the `DIRECTION.lock` create. The run directory is documented as writable by sibling sandboxes and `--unsandboxed` runs.

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

### WR-03: Run-start opens of untrusted run-dir names are unhardened (`DIRECTION.lock`, `state.json`)

**File:** `src/driverctl/serve.rs:934-939`, `src/driverctl/store.rs:644`, `src/launcher/run.rs:696-701` (same pattern, outside this phase's list)
**Issue:** `serve` takes the run lock and then does the following:

- It opens `DIRECTION.lock` with `OpenOptions::new().create(true).write(true).truncate(false)`. That open has no `O_NOFOLLOW` or `O_NONBLOCK`. A FIFO pre-planted at that name blocks `open(2)` for good while the run lock is held, so the run can never start and can only be killed. A symlink there makes `create(true)` create an empty file at the link target.
- `RunStore::open_with_clock` then calls `fs::read(&state_path)`, which follows symlinks, blocks on a FIFO and is uncapped.

`acquire_at` was hardened for exactly these cases (WR-08), and `read_run_file_capped` and `open_run_file` exist for the reads. These two opens do not use them. `driver_frontmatter.rs:810-814` states that a run directory is writable from inside an `--unsandboxed` run, which is the leftover-plant scenario.
**Fix:**

- Create `DIRECTION.lock` through the same open flags as `acquire_at`: `custom_flags(O_NOFOLLOW | O_NONBLOCK)`, then check on the handle that it is a regular file. Put this in one helper that both `serve` and `launcher::run` call.
- Read `state.json` through `read_run_file_capped`. Map `InvalidData` to `StoreError::CorruptState` and `NotFound` to the fresh-run branch.

### WR-04: `probe_at` and `read_holder_at` can block, follow links and read without a bound

**File:** `src/driver/lock.rs:304-312` and `src/driver/lock.rs:370-385`, reached from `src/state_reader/driver_frontmatter.rs:959`
**Issue:** Both functions use `File::open(path)`. That open has no `O_NONBLOCK` or `O_NOFOLLOW`, and no type or size check. `probe_at` is called by `read_driver_run_text`, which the TUI runs in `spawn_blocking` for every listed run (`list_driver_runs`). `launcher/stop.rs` and `launcher/push.rs` call it too.

- A FIFO renamed over `launcher.lock` parks that worker forever.
- A symlink at `launcher.lock` makes the status derive from a different file's flock. A dead run can read `Held` and so as written, or the reverse.
- `read_holder_at` does an unbounded `read_to_string`.

The same module's `acquire_at` and the DIRECTION reader already refuse all of this, so the two read paths are the inconsistent ones.
**Fix:** Open both through the `acquire_at` flag pair. Check `file.metadata()?.file_type().is_file()`. Treat anything else as `LockProbe::Unknown(Errno::INVAL)` in `probe_at` and as `None` in `read_holder_at`. Bound the holder read with `take(4096)`.

### WR-05: `serve` shutdown overwrites a driver's `paused` end with `stopped`

**File:** `src/driverctl/store.rs:1375`, reached from `src/driverctl/serve.rs:1008`
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
**Issue:** The `LockHolder` docs say a forged record "can never grant a second lock, so the worst outcome of tampering is a misleading string (T-17-08)". `driver stop` now takes `holder.pgid` from that record and sends `SIGTERM` to it. In a bwrap run the file is bound to `/dev/null`, so this only matters for `--unsandboxed` runs, where the driver already runs as the same user. The stop path does verify `probe_at == Held` first, but that does not tie the pid to the lock holder.
**Fix:** Amend the doc to say the record is a signal target for `driver stop`. Optionally verify before signalling that `/proc/<pid>` still holds the lock file open.

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

**File:** `tests/driverctl_client.rs:32`, `tests/driverctl_replay.rs:45`, `tests/driverctl_lock_race.rs:163-190`, `tests/driverctl_serve.rs:799-802`
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

_Reviewed: 2026-10-28_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
