# Phase 27: driverctl & Driver Frontmatter - Pattern Map

**Mapped:** 2026-10-18
**Files analyzed:** 20 (new + modified)
**Analogs found:** 16 / 20

All analog paths below were checked as git-tracked source (`git ls-files`).

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `src/driverctl/mod.rs` (new) | module root / config | n/a | `src/driver/mod.rs` (header lines 1-40, cfg submodules ~66-104, non-unix refusal 1450-1462) | exact |
| `src/driverctl/protocol.rs` (new) | model | request-response | `src/cli.rs` `EnvelopeAction` (typed exit-code variants, 380-400) + spike `driverctl.py` | partial |
| `src/driverctl/client.rs` (new) | utility (CLI client) | request-response | `src/main.rs` Envelope arm (367-…) exit-code-is-the-control | partial |
| `src/driverctl/store.rs` (new) | service (RunStore) | CRUD / file-I/O | `src/journal/writer.rs::write_run_record` (540-575) | role-match |
| `src/driverctl/frontmatter.rs` (new emitter) | utility | transform | none in crate (hand emitter; spike `y()/flow()`) | no analog |
| `src/driverctl/direction.rs` (new) | utility (parser) | transform | `src/state_reader/state_md.rs::extract_frontmatter` (325-340), `disk_status.rs::leading_frontmatter_value` (578-600) | role-match |
| `src/driverctl/splice.rs` (new, linux) | utility | file-I/O | `src/driver/lock.rs` (rustix idiom) + `journal/writer.rs` temp-file | partial |
| `src/driverctl/counters.rs` (new) | utility (pure fns) | transform/batch | none (spike `gapcount.py`, `exitclass.py`) | no analog |
| `src/driverctl/log.rs` (new) | utility | file-I/O + parse | none (spike `EV_RE`) | no analog |
| `src/driverctl/scan.rs` (new) | utility | file-I/O (poll) | `src/state_reader/disk_status.rs` (frontmatter scalar read) | partial |
| `src/driverctl/serve.rs` (new, linux) | server | request-response / event loop | none (no unix socket in crate) | no analog |
| `src/state_reader/driver_frontmatter.rs` (new) | model + reader | transform | `state_md.rs` frontmatter reader + `src/driver/liveness.rs::Liveness` tri-state (60-95) | role-match |
| `src/state_reader/mod.rs` (mod) | config | n/a | itself lines 1-12 (`pub mod …` list) | exact |
| `src/driver/lock.rs` (mod: `acquire_at`, `acquire_at_with_retry`, `probe_at`) | utility | file-I/O | itself (`acquire` 151-218, `read_holder` 256-264) | exact |
| `src/error.rs` (mod: driverctl/probe errors) | model | n/a | `LockError` (541-…), `DriveError::UnsupportedPlatform` (626-640) | exact |
| `src/cli.rs` (mod: `Commands::Driverctl`) | config/route | request-response | `Commands::Envelope` (354-374) + `Drive` hide comment (100-127) | exact |
| `src/main.rs` (mod: dispatch-before-tracing, fallible tracing) | entrypoint | n/a | itself lines 64-82 | exact |
| `src/lib.rs` (mod: `pub mod driverctl;`) | config | n/a | itself lines 1-30 | exact |
| `tests/driverctl_lock_race.rs`, `tests/driverctl_*.rs` (new) | test | n/a | `tests/driver_lock.rs` (header 1-30, SIGKILL test 549-…) | exact |
| `tests/fixtures/driverctl/` (new) | fixtures | n/a | `tests/fixtures/` (fake-claude-*.sh, transcripts/) | role-match |

## Pattern Assignments

### `src/driver/lock.rs` (extend: `acquire_at`, `acquire_at_with_retry`, `probe_at`)

**Analog:** itself. Keep the five-fact header (lines 1-39) and append facts for SH probe + bounded retry (D-20). Do not touch `LOCK_FILE_NAME`/`lock_path`/`acquire` semantics; refactor `acquire` into a wrapper over `acquire_at`.

Imports (41-49):
```rust
use std::fs::{File, OpenOptions};
use std::io::{Read, Seek, Write};
use std::path::{Path, PathBuf};
use rustix::fs::{flock, FlockOperation};
use serde::{Deserialize, Serialize};
use crate::error::LockError;
```

Core acquire (161-192) — copy open flags and errno mapping verbatim into `acquire_at(path, …)`:
```rust
let mut file = OpenOptions::new()
    .create(true).read(true).write(true).truncate(false)
    .open(&path)
    .map_err(|err| LockError::Unavailable { detail: format!("could not open {}: {err}", path.display()) })?;
match flock(&file, FlockOperation::NonBlockingLockExclusive) {
    Ok(()) => {}
    Err(errno) if errno == rustix::io::Errno::WOULDBLOCK => {
        return Err(match read_holder(planning_dir) {   // -> read_holder_at(path)
            Some(holder) => LockError::HeldBy { run_id: holder.run_id, pgid: holder.pgid, started_at: holder.started_at },
            None => LockError::HeldByUnknownRun,
        });
    }
    Err(errno) => return Err(LockError::Unavailable { detail: format!("flock on {} failed: {errno}", path.display()) }),
}
```
Holder write-in-place (200-242, NOT temp+persist — comment explains why) and `RunLock` guard with no `Drop` (98-129) are reused unchanged.

Probe (new) — follow `read_holder` (256-264) "never opens for writing" posture: `File::open(path)` (O_RDONLY, no create) → `NotFound` ⇒ `Absent`; `flock(&f, FlockOperation::NonBlockingLockShared)` → `Ok` ⇒ `Free` (drop file), `WOULDBLOCK` ⇒ `Held`, other errno ⇒ `Unknown(errno)`.

Retry (new): loop `acquire_at` while `HeldBy|HeldByUnknownRun`, sleep 1 ms doubling capped 50 ms, total ≤ 500 ms (`std::thread::sleep`; this is sync code — never call from `async fn`, see async guard).

Unit-test style (266-375): `tempfile::TempDir`, descriptive snake_case test names, assertion messages explaining the invariant.

---

### `src/driverctl/mod.rs` (module root)

**Analog:** `src/driver/mod.rs`
- Header doc style lines 1-40: numbered "facts", each citing the decision ID (D-xx). Write facts for: single writer by type (D-20), dispatched before tracing (D-25), linux-only splice/serve with typed refusal (D-12), sync std threads not tokio (async guard).
- Submodule gating (around line 102-104):
```rust
#[cfg(unix)]
pub mod kill;
pub mod liveness;
```
→ use `#[cfg(target_os = "linux")] pub mod splice; #[cfg(target_os = "linux")] pub mod serve;`, others ungated.
- Non-platform refusal (1450-1462):
```rust
#[cfg(not(unix))]
async fn dispatch(...) -> Result<(), DriveError> {
    let _ = (project, args, entry, approved_plan, budget);
    Err(DriveError::UnsupportedPlatform {
        detail: "process-group detachment is a Unix facility and has no portable equivalent".to_string(),
    })
}
```
→ `#[cfg(not(target_os = "linux"))] pub fn run(cmd) -> i32` returning a typed `DriverctlError::UnsupportedPlatform { detail: "renameat2(RENAME_EXCHANGE) …" }` (sync fn).

---

### `src/cli.rs` (add `Commands::Driverctl`)

**Analog:** `Commands::Envelope` (354-374) and the `Drive` comment block (100-127).
```rust
    /// Internal: envelope enforcement re-entered by a generated git hook
    #[command(hide = true)]
    Envelope {
        #[command(subcommand)]
        action: EnvelopeAction,
    },
```
- Variant ungated by `#[cfg]` (comment at 100-104 states why). Add a comment block of the same shape explaining hide-not-gate and that the sandbox hook re-enters it.
- Nested `#[derive(Subcommand)] pub enum DriverctlAction { Serve{run_dir, project, sock: Option<PathBuf>}, Log{event: String, + 17 Option<String> flags with underscore dests}, Hook, Show }` modeled on `EnvelopeAction` (380-400), using `#[arg(long)]`.
- Parse entry: `Cli::parse_checked()` (76-78) — exits 2 on usage error (argparse parity, RESEARCH).
- Add a parse test alongside existing `Commands::Drive` parse tests (~535, 604).

---

### `src/main.rs` (dispatch-before-tracing, D-25)

**Analog:** itself, lines 64-82 (current order is the bug):
```rust
#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let log_dir = dirs::data_local_dir()...;
    let file_appender = tracing_appender::rolling::daily(&log_dir, "gsd-meta-manager.log"); // panics if unwritable
    tracing_subscriber::fmt().with_writer(file_appender)...init();
    color_eyre::install()...?;
    let cli = Cli::parse_checked();
```
Change: parse first; `if let Some(Commands::Driverctl { action }) = cli.command { std::process::exit(driverctl::run(action)) }` (sync; if inside tokio main use `spawn_blocking` or split a sync `fn main` that builds the runtime only for non-driverctl). Then replace `rolling::daily` with `RollingFileAppender::builder()…build(&log_dir)` and skip file logging on `Err`. Exit-code style: `std::process::exit(code)` as in `judged_alias_or_exit` (31-62) and the Envelope arm (367+).

---

### `src/driverctl/store.rs` (RunStore; state.json atomic write)

**Analog:** `src/journal/writer.rs::write_run_record` (540-575):
```rust
let mut handle = NamedTempFile::new_in(&paths.dir).with_context(|| ...)?;
let json = serde_json::to_string_pretty(record).context("Failed to serialize the run record")?;
handle.write_all(json.as_bytes()).context(...)?;
#[cfg(unix)] { use std::os::unix::fs::PermissionsExt;
  handle.as_file().set_permissions(std::fs::Permissions::from_mode(0o644)).context(...)?; }
handle.persist(&paths.run_json).with_context(|| ...)?;
```
Add `handle.as_file().sync_all()` before persist (RESEARCH diagram: tmp+fsync+rename). Tolerant JSON read precedent: `has_end_timestamp` (~590) reads `serde_json::Value` so later schemas still load.
`RunStore::open(run_dir, project, lock: RunLock)` takes the guard by value — mirrors `RunLock` doc (lock.rs 98-111: "guard must be stored … not a local").
Git reads (if any, e.g. rev-parse): `crate::state_reader::git_ops::git_read_raw_within(root, &args, budget) -> Option<String>` (git_ops.rs 244-257); it is `pub(crate)`, usable from `src/driverctl`.

---

### `src/driverctl/direction.rs` and `src/state_reader/driver_frontmatter.rs` (frontmatter split + typed reader)

**Analog:** `src/state_reader/state_md.rs` 325-340:
```rust
pub fn extract_frontmatter(content: &str) -> Option<&str> {
    let content = content.trim_start();
    if !content.starts_with("---") { return None; }
    let after_open = &content[3..];
    let after_open = after_open.trim_start_matches(['\r', '\n']);
    if let Some(end) = after_open.find("\n---") { Some(&after_open[..end]) } else { None }
}
```
Reader: `extract_frontmatter(text)` → `serde_yml::from_str::<DriverFrontmatter>` with `#[serde(default)]` on optional fields (incl. `item_states`, D-19), no `deny_unknown_fields`. Note: the splice's own `split_frontmatter` must be byte-exact (`---\n` / `---\r\n` start, `\n---\n` end, `generated_by: driverctl` inside) — do not reuse the trimming `extract_frontmatter` for writes.
`fm_status` port: column-zero scalar scan modeled on `disk_status.rs::leading_frontmatter_value` (578-600), but strip optional quotes per gapcount `STATUS_RE` (D-27: do NOT call `VerificationStatus::from_raw`).

Derived status tri-state: copy `src/driver/liveness.rs` 60-83 doc/enum shape (`Alive/Dead/Unknown`, "Unknown never a synonym for Dead", derives `Debug, Clone, Copy, PartialEq, Eq`) for `LockProbe {Held, Free, Absent, Unknown(errno)}` and `DerivedStatus` incl. `LivenessUnknown` (D-22 table R1-R6).

Export: add `pub mod driver_frontmatter;` to `src/state_reader/mod.rs` alphabetical list (lines 1-12).

---

### `src/error.rs` (new error variants)

**Analog:** `LockError` (541-571) and `DriveError::UnsupportedPlatform` (626-640): `#[derive(Debug, Clone, PartialEq, Eq)]`, one doc paragraph per variant stating why it is distinct, struct-variant `{ detail: String }`, hand `Display` (~947). Add e.g. `DriverctlError { UnsupportedPlatform{detail}, Rejected{error} (rc 2), Transport{detail} (rc 1), SocketPath{detail}, UnsupportedVersion{major} }`.

---

### `tests/driverctl_lock_race.rs` and other `tests/driverctl_*.rs`

**Analog:** `tests/driver_lock.rs`
- Banner comment header (1-21) explaining why integration not unit; `#![cfg(unix)]` (→ `#![cfg(target_os = "linux")]` for splice/serve tests).
- Imports pattern (23-31): `use gsd_meta_manager::driver::lock::{self, LockHolder}; use gsd_meta_manager::error::LockError; use tempfile::TempDir;`
- Real-binary child + SIGKILL (549-600): `std::process::Command::new(env!("CARGO_BIN_EXE_gsd-meta-manager"))`, stdout/stderr to files (not null, not undrained pipe — comment 560-563), wait for lock, `kill -9`, assert release. Reuse for test 6 (second serve refused) and test 7 (SIGKILL → crashed, restart logs `crash-detected`).
- In-process threads with separate `File::open` are valid flock contenders (RESEARCH E5) for tests 1-5.
- Opt-in long runs: `#[ignore]` like the conformance oracle test (`tests/driver_router_conformance.rs`) for the 15 s editor race; short smoke variant un-ignored.
- D-25 regression: spawn binary with `XDG_DATA_HOME=/proc/nonexistent-gsdmm`, missing `DRIVERCTL_SOCK`; assert hook prints deny JSON + exit 0, `log` exits 1 not 101.

### `tests/fixtures/driverctl/`
Copy spike LOG/DIRECTION/state files into the crate tree (`.planning/` is excluded from the package). Reference via `concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/driverctl/…")` as `FAKE_SLOW` does (driver_lock.rs ~51-54).

## Shared Patterns

### Doc-comment headers with decision IDs
**Source:** `src/driver/lock.rs` 1-39, `src/driver/mod.rs` 1-40. **Apply to:** every new `src/driverctl/*.rs` and `driver_frontmatter.rs`. Include schema-evolution rule (D-19: adding value additive; rename/remove = `driver_frontmatter_version` major bump) on `DriverFrontmatter` and the emitter.

### Typed refusal, ungated parser
**Source:** `src/cli.rs` 100-104 + `src/driver/mod.rs` 1450-1462. **Apply to:** cli variant, `driverctl::run`, splice, serve.

### Sync-only blocking I/O
**Source:** `tests/async_blocking_guard.rs` 1-40 (lexical lint over `src/`: no named blocking call inside `async fn`). **Apply to:** all of `src/driverctl/` and `probe_at`/`acquire_at_with_retry` — write them as plain `fn`, use `std::thread`/`std::os::unix::net`. The guard scans `src/` automatically; verify it stays green.

### Atomic write vs lock-file write
**Source:** `journal/writer.rs` 540-575 (temp + persist, for state.json) vs `lock.rs` 200-242 (in-place through held fd, for lock files — never rename a lock file). **Apply to:** store.rs (temp+fsync+persist), lock.rs (in-place).

### Exit codes are the control
**Source:** `src/cli.rs` `EnvelopeAction` doc (374-379), `main.rs` `std::process::exit`. **Apply to:** client rc mapping (0 ok, 2 rejected/usage, 1 transport; hook gate fail-closed exit 0 with deny JSON).

## No Analog Found

| File | Role | Data Flow | Reason |
|---|---|---|---|
| `src/driverctl/serve.rs` | server | request-response | No unix-socket server exists in the crate; follow RESEARCH §Wire Protocol + spike `driverctl.py:464-491` |
| `src/driverctl/counters.rs` | pure fns | transform | Port of spike `gapcount.py` / `exitclass.py`; tests from their selftest fixtures |
| `src/driverctl/frontmatter.rs` (emitter) | utility | transform | No YAML emitter in crate (serde_yml parse-only by D-24); port spike `y()/flow()` |
| `src/driverctl/log.rs` | utility | file-I/O | LOG.md grammar (`EV_RE`) is spike-only; use one `O_APPEND` write per line |

## Metadata

**Analog search scope:** `src/driver/`, `src/state_reader/`, `src/journal/`, `src/cli.rs`, `src/main.rs`, `src/error.rs`, `tests/`
**Files scanned:** ~14
**Pattern extraction date:** 2026-10-18
