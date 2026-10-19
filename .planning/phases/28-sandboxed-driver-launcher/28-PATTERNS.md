# Phase 28: Sandboxed Driver Launcher - Pattern Map

**Mapped:** 2026-10-19
**Files analyzed:** 16 (new + modified)
**Analogs found:** 15 / 16

Module placement (CONTEXT "Claude's Discretion"): recommend `src/launcher/` as a new top-level module
(peer of `src/envelope/`, `src/driverctl/`), because it is called from CLI and TUI, same reasoning as the
envelope placement. Linux-only submodules `#[cfg(target_os = "linux")]`, as `src/driverctl/mod.rs:107-112`.
File names below assume that layout; `[inferred — audit]`.

## File Classification

| New/Modified File | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|
| `src/launcher/mod.rs` (new) | module root + numbered facts header | n/a | `src/driverctl/mod.rs` | exact |
| `src/launcher/sandbox.rs` (new): pure bwrap / systemd-run argv, env scrub list, host detection parsing | utility (pure builders) | transform | `src/driver/spawn.rs::drive_argv` (L58-80), `src/executor/claude.rs` build_argv (~L218-260) | role-match |
| `src/launcher/config_dir.rs` (new): per-run `CLAUDE_CONFIG_DIR` path + seeding, settings.json transform | utility | file-I/O + transform | `src/envelope/mod.rs::envelope_root/envelope_dir_in` (L209-261) | role-match |
| `src/launcher/process.rs` (new): THE single spawn file (bwrap, systemd-run, systemctl, git mirror/push, `claude setup-token`, gh) | service | process / request-response | `src/driver/spawn.rs::spawn_detached` (L130-166), `src/executor/claude.rs` env scrub (L507-575) | role-match |
| `src/launcher/run.rs` (new): launch loop, lock, embedded store thread, monitor, caps trip, classify | service | event-driven (supervision loop) | `src/driverctl/serve.rs::serve` (L639-730) | exact |
| `src/launcher/gate.rs` (new): dispatch + gap cap gate (`mentions_phase`) | utility (pure) | transform | `src/driverctl/store.rs` `Gate` type (L229-239) + `counters::classify_exit` (L240-265) | role-match |
| `src/launcher/auth.rs` (new): token store 0600, host fallback, refresh-token invariant | service | file-I/O | `src/driverctl/serve.rs::socket_path_in` owner/mode checks (L278-300) | partial |
| `src/launcher/push.rs` (new): launcher-owned mirror, fixed refspec, scan + ledger | service | request-response (git) | `src/envelope/ledger.rs::record_and_check` (L260-272), `src/envelope/scan.rs::scan_with_external` (L405) | role-match |
| `src/launcher/stop.rs` (or in run.rs): `driver stop` (SIGTERM launcher, scope stop fallback) | service | request-response | `src/driver/kill.rs::signal_group` (L183-205), `stop_run` (L315) | role-match |
| `src/launcher/degraded.rs` (new): unsandboxed supervision | service | process supervision | `src/driver/kill.rs` (`DRIVER_TEARDOWN_GRACE`, `stop_run`) | role-match |
| `src/driverctl/store.rs` (modify): `TerminalStatus::Parked`, cap-tripped writer, launch-end, wall-clock used, launcher keys, `sandbox` posture | model/store | CRUD | itself: `set_terminal` (L1033), `begin_launch` (L728) | exact |
| `src/driverctl/serve.rs` (modify): pub(crate) accept loop + ticker entry | service | event-driven | itself L568-624 | exact |
| `src/state_reader/driver_frontmatter.rs` (modify): additive `sandbox`, `sandbox_missing` | model | transform | itself `DriverFrontmatter` (L348-384), `emit` (L503+) | exact |
| `src/cli.rs` + `src/main.rs` + new `src/driver_cli.rs` (modify/new): `Commands::Driver { Launch, Stop, Push, Auth }` | controller (CLI) | request-response | `Commands::Driverctl` (cli.rs L389-424), `main.rs` L104-108, `src/driverctl_cli.rs::run/serve_main` (L115-160) | exact |
| `src/ui/screens/driver.rs` (modify): run selection, stop action, `UNSANDBOXED` badge | component | request-response | itself (Yellow style L1828) + `read_driver_run` (driver_frontmatter.rs L742) | role-match |
| `tests/spawn_seam_guard.rs` (modify): one allowlist entry | test/guard | n/a | `SPAWN_ALLOWLIST` L45-110 | exact |
| `tests/launcher_*.rs` (new): golden argv, fake-shim integration, `#[ignore]` live | test | n/a | `tests/driverctl_serve.rs`, `tests/driverctl_frontmatter.rs:469` | exact |
| `scripts/launcher-acceptance.sh` + rubric file (new): L1-L5/L7/L8 + D-33 harness | script | batch | none in tree (spike `experiment/run.sh`) | none |

## Pattern Assignments

### `src/launcher/mod.rs` (module root)

**Analog:** `src/driverctl/mod.rs` (whole file, 112 lines). Copy the shape: `//!` header of numbered facts each
citing its decision ID (D-xx), a "# Phase 29/30 hand-off" section, then cfg-gated submodules:

```rust
pub mod counters;
pub mod log;
pub mod protocol;
pub mod scan;
#[cfg(target_os = "linux")]
pub mod serve;
#[cfg(target_os = "linux")]
pub mod store;
```
Fact 5 of that header ("All code here is synchronous: never call it from an `async fn`", enforced by
`tests/async_blocking_guard.rs`) applies to the launcher too.

---

### `src/launcher/run.rs` (launch loop, event-driven)

**Analog:** `src/driverctl/serve.rs::serve` L639-730. This is the exact order the launcher must keep
(lock first, store, begin_launch, threads, terminal status, release lock last).

Lock acquisition with typed `HeldBy` → rc 3 mapping (L644-658):
```rust
let lock = acquire_at_with_retry(
    &run_lock_path(&run_dir), &run_id, std::process::id(), ACQUIRE_RETRY_BUDGET,
).map_err(|e| match e {
    LockError::Unavailable { detail } => DriverctlError::Io { detail: format!("run lock: {detail}") },
    held => DriverctlError::Locked { detail: held.to_string() },
})?;
```
Exit-code mapping (L128-137): `Locked => 3, _ => 1` — copy into a `LauncherError::exit_code`.

Store + begin_launch per launch (L683-691); the launcher fills `launcher_mode` (`bwrap-scope`/`degraded`),
`limits: CapLimits`, `scope_unit: Some(unit)` (store.rs L201-225 already has these fields):
```rust
let mut store = RunStore::open(&run_dir, &project, lock)?;
store.begin_launch(LaunchSpec {
    launcher_mode: "standalone".to_owned(),
    driver: Some(DriverSpec { model: None, mode: "headless".to_owned() }),
    ..LaunchSpec::default()
})?;
```
Background thread pattern (L702-721) — named `std::thread::Builder`, `Arc<Mutex<RunStore>>`, shared
`Arc<AtomicBool>` stop flag, and on spawn failure undo via `abort_startup` (L631-636).

Ticker (L606-624): scan BEFORE taking the mutex (CR-01), then `observe_verifications` + `flush_heartbeat`.
The launcher's monitor tick should reuse this verbatim (via the new pub(crate) entry) and add wall-clock
accounting + gap-cap kill check after `observe_verifications`.

Signal handling (L408-454): tiny current-thread tokio runtime on a dedicated thread, readiness handshake via
`mpsc::channel` + `recv_timeout(5s)`. Reuse for the launcher's SIGTERM (D-09 stop).

Shutdown (L723-742): stop accepting, join ticker, drain in-flight, `set_terminal`, remove socket, drop store
(releases lock LAST):
```rust
let terminal = lock_store(&store).set_terminal(TerminalStatus::Stopped, "serve:signal");
...
let _ = fs::remove_file(&sock);
match Arc::try_unwrap(store) {
    Ok(mutex) => drop(mutex.into_inner().unwrap_or_else(PoisonError::into_inner)),
    Err(_still_shared) => {}
}
```
Poison-tolerant mutex (L460-462): `store.lock().unwrap_or_else(PoisonError::into_inner)`.

Run-dir validation: call `serve::validate_run_dir(project, run_dir)` (L186-238) and
`serve::socket_path(&run_id, &canonical_run_dir)` (L247) — never rebuild paths (hand-off 5). The sha8 for
the scope unit name (D-08) must use the same digest as L272-273:
```rust
let digest = Sha256::digest(canonical_run_dir.as_os_str().as_bytes());
let hash8: String = digest.iter().take(4).map(|b| format!("{b:02x}")).collect();
```
Extract this into a shared `pub(crate) fn hash8(&Path) -> String` in serve.rs rather than duplicating.

Exit classification at launch end: `counters::classify_exit(rc, tripped, ended_by_driver, result_ok,
outstanding)` (counters.rs L240-265) → map `ExitClass::{Stalled,Crashed}` to `TerminalStatus` → `set_terminal`.
`outstanding` from `counters::outstanding_dispatch` (L192).

Rate-limit park: `driver::rate_limit::classify(event, now) -> QuotaVerdict` (L337) and
`park_detail(window, resets_at)` (L377) for the parked reason text.

---

### `src/driverctl/serve.rs` (modify: pub(crate) embed entry)

Today `accept_loop` (L568) and `ticker` (L606) are private `fn`. Make them `pub(crate)` (or wrap in a
`pub(crate) fn run_embedded(listener, store, stop)`), plus expose `bind`-side helpers
(`existing_socket` L318, `replace_and_bind` L369, `check_sun_len` L303). Keep `serve()` calling the same
functions so `tests/driverctl_serve.rs` stays the regression net.

---

### `src/driverctl/store.rs` (modify)

Extend `TerminalStatus` (L246-260) in the same exhaustive-match style:
```rust
pub enum TerminalStatus { Crashed, Stalled, Stopped }
impl TerminalStatus {
    fn run_status(self) -> RunStatus {
        match self {
            TerminalStatus::Crashed => RunStatus::Crashed, ...
```
Add `Parked` (requires `RunStatus::Parked` in driver_frontmatter if absent). New writer methods
(cap-tripped, launch-end-without-status, wall-clock used, launcher keys) copy `begin_launch`'s
clone-validate-commit shape (L728-760): `let mut next = self.state.clone(); let mut entries = Vec::new();`
push `LogEntry::run("cap-tripped", vec![("cap", Some(..)), ("used", ..), ("limit", ..), ("action", Some("kill-scope".into()))])`,
then commit. Launcher-only (no socket op reaches them), documented like `set_terminal`'s doc (L1021-1032).
Header values go through `header_safe` (L329).

---

### `src/state_reader/driver_frontmatter.rs` (modify: additive `sandbox` key)

Struct field pattern (L355-383): every optional key is `#[serde(default)] pub x: Option<..>`.
```rust
#[serde(default)]
pub status_reason: Option<String>,
```
Emit (L503+) appends `lines.push(format!(...))` in fixed order. RESEARCH §6: goldens are byte-exact, so emit
`sandbox:` / `sandbox_missing:` **only when `Some`** (unlike `status_reason`, which always emits via
`quote_or_null`). Golden regen test lives at `tests/driverctl_frontmatter.rs:469` (`#[ignore = "regenerates the goldens; run explicitly"]`).

---

### `src/launcher/sandbox.rs` (pure argv builders)

**Analog:** `src/driver/spawn.rs::drive_argv` L58-80 — pure `fn -> Vec<OsString>`, conditional pushes:
```rust
let mut argv = vec![OsString::from("--config"), config_path.as_os_str().to_owned(), ...];
if let Some(goal) = goal {
    argv.push(OsString::from("--goal"));
    argv.push(OsString::from(goal));
}
argv
```
Builders take all host facts as parameters (paths that exist, `XDG_RUNTIME_DIR`, home, socket path) so
conditional masks (RESEARCH §1: masks on missing paths abort bwrap) and the `$XDG_RUNTIME_DIR` tmpfs +
socket re-bind are golden-testable without a filesystem. Also `--setting-sources user` on the claude argv
(RESEARCH §3), mirroring how `executor/claude.rs` L252-254 pushes `--output-format stream-json`.
Host detection: parse `systemctl --user is-system-running` stdout (`running`|`degraded` usable) in a pure
`fn parse_user_manager(stdout: &str) -> bool`; the spawn itself lives in process.rs.

---

### `src/launcher/process.rs` (THE one spawn file)

**Analog A:** `src/driver/spawn.rs::spawn_detached` L130-166 (doc comment explains every load-bearing call):
```rust
let mut cmd = tokio::process::Command::new(program);
cmd.args(argv);
cmd.current_dir(project_root);
cmd.stdin(Stdio::null()); cmd.stdout(Stdio::null()); cmd.stderr(Stdio::null());
cmd.kill_on_drop(false);
cmd.as_std_mut().process_group(0);
```
(launcher is sync per the driverctl rule, so use `std::process::Command` + `CommandExt::process_group(0)`.)

**Analog B:** env scrub in `src/executor/claude.rs` L507-512 then explicit set:
```rust
for (key, _) in std::env::vars_os() {
    if key.to_string_lossy().starts_with("CLAUDE") {
        cmd.env_remove(&key);
    }
}
cmd.env("CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS", &bg_ceiling);
```
plus the `Option`-valued apply loop (L561-575): `Some(v) => cmd.env(k, v)`, `None => cmd.env_remove(k)`.
Port spike `STRIP_ENV` + `ANTHROPIC_AUTH_TOKEN` (RESEARCH §3) as a const list in sandbox.rs, applied here;
token goes in env only (D-19), plus `GIT_TERMINAL_PROMPT=0` (D-27). Prefer `env_clear()`-free approach
consistent with the analog, or document deviation.

Guard requirement: add exactly one entry to `tests/spawn_seam_guard.rs` `SPAWN_ALLOWLIST` (L45-110) with a
comment in the existing style, e.g.:
```rust
    // The Phase 28 launcher: bwrap under systemd-run, systemctl, git for the
    // push mirror, `claude setup-token`, `gh`. Every launcher spawn lives here
    // so this is the one entry. The agent is reached only through bwrap.
    "src/launcher/process.rs",
```
Check `COMMAND_SOURCE_ALLOWLIST` (L2561) and `OSSTRING_ALLOWED` (L3051) in the same guard for whether a
new argv source also needs declaring.

---

### `src/launcher/config_dir.rs` (data-dir path + seeding)

**Analog:** `src/envelope/mod.rs` L209-261:
```rust
pub fn envelope_root() -> Option<PathBuf> {
    if let Some(override_root) = std::env::var_os(ENVELOPE_ROOT_ENV) {
        if !override_root.is_empty() { return Some(PathBuf::from(override_root)); }
    }
    dirs::data_local_dir().map(|d| d.join(APP_SUBDIR).join(ENVELOPE_SUBDIR))
}
pub fn envelope_dir_in(root: &Path, alias: &str) -> Option<PathBuf> {
    if !crate::journal::is_plain_path_component(alias) { return None; }
    Some(root.join(alias))
}
```
Copy: `Option` return (no `/tmp` fallback), a `*_in(root, ..)` testable variant, an env override const for
tests (e.g. `GSD_MM_DRIVER_ROOT`), validate components before `join`. Settings.json seeding = pure
`fn seed_settings(user: serde_json::Value, driverctl_hooks) -> Value` (drop `statusLine`, plugins,
`apiKeyHelper`; PreToolUse `Agent|Task` hook with `"onFailure": "block"`), golden-tested.

---

### `src/launcher/auth.rs`

**Analog (permission/owner checks):** `src/driverctl/serve.rs::socket_path_in` L278-300:
```rust
match fs::DirBuilder::new().mode(0o700).create(&dir) { ... AlreadyExists => {} ... }
let meta = fs::symlink_metadata(&dir)?;
if meta.uid() != rustix::process::getuid().as_raw() { return Err(..."not owned by the current user") }
if meta.mode() & 0o077 != 0 { return Err(..."must not be accessible to group or others") }
```
Apply the same to the token file (mode 0600, write under umask as `bind_private` L393-404 does with
`rustix::process::umask`). Path base: `dirs::config_dir()` with the `Option`-no-fallback rule above.
Secret reading precedent: `src/envelope/cred.rs` (reads secret to stdout, never a file) — consult for
no-logging discipline.

---

### `src/launcher/push.rs`

**Analogs:** `envelope::scan::scan_with_external(root, ScanLimits) -> ScanReport` (scan.rs L405; `is_clean()`
L188, `render()` L198) on the mirror; `envelope::ledger::record_and_check(alias, &LedgerEntry, &policy)`
(ledger.rs L260-272) for the PR cap; `envelope::policy::DEFAULT_NAMESPACE_ROOT` for the refspec; namespace
refusal via `envelope::hooks`. Park on a scan hit with the existing SAFE-03 `policy::ParkReason`
(`envelope::park_at`, mod.rs L360). Mirror lives under the data-dir root (config_dir.rs pattern).
Git spawns go through process.rs.

---

### `src/launcher/stop.rs` / `degraded.rs`

**Analog:** `src/driver/kill.rs` — `signal_group(pgid, sig)` L183-205 refuses pgid 0 (copy that refusal for
the launcher pid read from the lock holder, `driver::lock::read_holder_at` L301); escalation constants
`DRIVER_TEARDOWN_GRACE` L57 and `pub async fn stop_run(pid, pgid, run_id, arm) -> StopOutcome` L315
(degraded supervision reuses it or a sync twin). `stop_decision` L226 shows the pure-mapping-for-testability
idiom. Lock probe for "launcher dead?" = `driver::lock::probe_at` (L367).

---

### `src/cli.rs` / `src/main.rs` / `src/driver_cli.rs`

**Analog:** `Commands::Driverctl` cli.rs L389-424 — not `#[cfg]`-gated, typed refusal in the handler:
```rust
Driverctl {
    #[command(subcommand)]
    action: DriverctlAction,
},
...
#[derive(Subcommand)]
pub enum DriverctlAction {
    Serve {
        #[arg(long)] run_dir: PathBuf,
        #[arg(long)] project: PathBuf,
        #[arg(long)] sock: Option<PathBuf>,
    },
}
```
Add `Driver { #[command(subcommand)] action: DriverAction }` with `Launch { project, run_id, #[arg(long)]
prompt_file: PathBuf, #[arg(long)] foreground: bool, #[arg(long)] unsandboxed: bool, ... }`, `Stop`, `Push`,
`Auth { #[command(subcommand)] Setup | Status }`. Not hidden (users type it). Parse tests follow the
`fn driverctl(args) -> Result<DriverctlAction, clap::Error>` helper at cli.rs L768-772.

main.rs dispatch (L104-108): synchronous, before tokio runtime, exit code is the control:
```rust
match cli.command.take() {
    Some(Commands::Driverctl { action }) => std::process::exit(driverctl_cli::run(action)),
    other => cli.command = other,
}
```
Driver needs tracing (log to file), so dispatch it after `init_tracing()` but before the multi-thread
runtime, or add an unreachable arm in `async_main` as at L414-418. Handler module mirrors
`src/driverctl_cli.rs::run` + the cfg pair L130-160:
```rust
#[cfg(target_os = "linux")]
fn serve_main(...) -> i32 { match serve(opts) { Ok(()) => 0, Err(e) => { err_line(..); e.exit_code() } } }
#[cfg(not(target_os = "linux"))]
fn serve_main(...) -> i32 { err_line("...: unsupported platform (the run store needs Linux)"); 1 }
```
Daemonize (D-04): `rustix::process::setsid` (RESEARCH §8) after fork/re-exec; re-exec form follows
`spawn_detached` (current_exe + `--foreground`).

---

### `src/ui/screens/driver.rs` (minimal)

Use `state_reader::driver_frontmatter::read_driver_run(run_dir)` (L742) for derived status (Phase 30
hand-off 4: never display raw `status:`). Badge style: existing `Style::default().fg(Color::Yellow)` at
L1828. Stop calls the same library stop fn as the CLI; no new tab.

---

### Tests

**Analog:** `tests/driverctl_serve.rs`: `#![cfg(target_os = "linux")]`, `Fixture { project_dir: TempDir,
scratch: TempDir, run_dir }` building `.planning/meta-manager/drivers/<RUN_ID>/DIRECTION.md` (L34-95),
short socket paths in scratch (107-byte limit), binary via `Command::new(env!("CARGO_BIN_EXE_gsd-meta-manager"))`
(L150), `UNWRITABLE_DATA_HOME` trick (L16). Fake bwrap/systemd-run/systemctl shims: write executable
scripts into `scratch` and prepend to `PATH` on the child Command; they log argv to a file for assertions.
Live tests: `#[ignore = "<reason>; run with `-- --ignored <name> --nocapture`"]` as in
`tests/driverctl_race.rs:383`, skip-with-reason when bwrap/systemd absent. Verify with
`rtk proxy cargo test --no-fail-fast` and `cargo clippy --all-targets -- -D warnings`.

## Shared Patterns

### Typed errors with exit codes
**Source:** `src/driverctl/serve.rs` L113-165 (`DriverctlError` enum, `exit_code`, `Display`, `From<StoreError>`,
`io_error(context, err)` helper). **Apply to:** every launcher module's error type (`LauncherError`,
`AuthError`, `PushError`, `SandboxUnavailable { missing: Vec<..> }`).

### Artifacts outside the repo, no /tmp
**Source:** `src/envelope/mod.rs` L209-261. **Apply to:** config dir, launch log, push mirror, token file.

### Single spawn site
**Source:** `tests/spawn_seam_guard.rs` L45-110. **Apply to:** all launcher Command spawns → `process.rs`.

### Launcher-only store writes
**Source:** `store.rs::set_terminal` doc L1021-1032 ("no op, event or argument reaches this from the socket").
**Apply to:** every new store method.

### Sync code, threads not async
**Source:** `src/driverctl/mod.rs` fact 5 + `serve.rs` signal-thread pattern L432-454.

## No Analog Found

| File | Role | Data Flow | Reason |
|---|---|---|---|
| `scripts/launcher-acceptance.sh` + pre-registered rubric | script | batch | No in-tree harness; port from spike-002/003 `experiment/run.sh`, `cap-tests/gap-writer.sh` (planning/spikes, not src). Use RESEARCH §7 timing (gap writes > 45 s apart, `COALESCE_S`). |
| systemd scope stop (`systemctl --user stop`) | service | process | No systemd usage in tree; follow RESEARCH §2 (`--collect`, RuntimeMaxSec) inside process.rs. |

## Metadata

**Analog search scope:** src/driverctl, src/driver, src/envelope, src/executor, src/state_reader, src/ui/screens, src/cli.rs, src/main.rs, src/driverctl_cli.rs, tests/
**Files scanned:** ~25
**Pattern extraction date:** 2026-10-19
