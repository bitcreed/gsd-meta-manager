# Phase 17: Supervisor — Detach, Kill Switch, Dry-Run, Opt-In Gate - Pattern Map

**Mapped:** 2026-07-29
**Files analyzed:** 18 (5–6 new source, 3–4 new tests, 9 modified)
**Analogs found:** 15 / 18 (three concerns have no analog anywhere in the tree: `flock`,
detached `process_group(0)` spawn, and the mechanical grep-guard test — see
[No Analog Found](#no-analog-found))

No RESEARCH.md for this phase (ROADMAP marks it skip). Every excerpt below was read directly
from the current tree this pass; line numbers are as of this date.

---

## File Classification

| New/Modified File | New? | Role | Data Flow | Closest Analog | Match Quality |
|---|---|---|---|---|---|
| `src/cli.rs` | mod | config / CLI surface | request-response | itself — `Commands::Add` | exact |
| `src/main.rs` | mod | controller (dispatch) | request-response | itself — `Some(Commands::Add {..})` arm `:34-56` | exact |
| `src/driver/mod.rs` (or `supervisor.rs`) | new | module root / domain model | event-driven | `src/executor/mod.rs:1-40` (module-root doc + `#[cfg(unix)]` submodule) | exact |
| `src/driver/spawn.rs` — detached spawn (D-01/D-02) | new | service | process-lifecycle | `src/executor/claude.rs:333-365` (spawn seam, pgid capture) | role-match (shape only — **not** `process-wrap`) |
| `src/driver/kill.rs` — TUI-side stop (D-06/D-07/D-08) | new | service | process-lifecycle | `src/executor/claude.rs:1482-1542` (`terminate_group`/`tear_down_group`/`finish_teardown`) | role-match (sequence to mirror; the `claude` half is **reused verbatim**) |
| `src/driver/liveness.rs` — `/proc` probe (D-10) | new | utility (pure-ish probe) | file-I/O | `src/session_detector.rs:46-69, 83-98` | exact |
| `src/driver/lock.rs` — `flock` (D-19/D-20) | new | service | file-I/O | none in repo | **no analog** |
| `src/driver/dry_run.rs` (D-22/D-24) | new | service (report builder) | transform / request-response | `src/state_reader/git_ops.rs:108-153` + `src/journal/writer.rs:322-359` | role-match |
| `src/state_reader/git_ops.rs` (diffstat, refspecs) | mod | utility (git shell-out) | request-response | itself — `head_sha` / `is_dirty` | exact |
| `src/config.rs` — `DriverOptIn`, `driver_opt_in`, `driver_max_concurrent` (D-14/D-15/D-18) | mod | model + config I/O | CRUD / file-I/O | itself — `RegisteredProject` / `Preferences` / `save_config` | exact |
| `src/executor/mod.rs` — `DrivableProject::from_registry`, rename `for_testing` (D-16/D-17) | mod | model (capability type) | — | itself `:109-151` | exact |
| `src/error.rs` — `OptInError`, `LockError`, `DriveError` | mod | model (typed errors) | — | itself — `SpawnError` `:18-60` (hand-written `Display`, no `thiserror`) | exact |
| `src/journal/mod.rs` — `ExecStarted { claude_pgid }` (D-09) | mod | model (schema) | event-driven | itself `:337-342` + `:773-776` | exact |
| `src/journal/reader.rs` — cursor carries `last_seq` (D-28) | mod | model / service | streaming | itself `:67-72`, `:244-269` | exact |
| `src/ui/screens/mod.rs` — driver sibling maps, cursor type change (D-25/D-27/D-28) | mod | state container | — | itself `:138-170` | exact |
| `src/app.rs` — startup+20-tick reconcile, prune, gap collapse (D-13/D-27/D-28) | mod | controller (reducer) | event-driven | itself `:280-351`, `:438-449`, `:663-687` | exact |
| `src/ui/screens/driver_confirm.rs` (D-25) | new | component (confirm screen) | request-response | `src/ui/screens/delete_confirm.rs` (whole file, 113 lines) | exact |
| `tests/driver_kill.rs`, `tests/driver_lock.rs`, `tests/driver_optin.rs` | new | integration test | process-lifecycle | `tests/executor_lifecycle.rs:1-110` | exact |
| `tests/…` grep-guard for `for_testing_bypassing_opt_in` (D-17) | new | integration test | file-I/O | **none** — the technique is documented in prose only | **no analog** |

---

## Pattern Assignments

### `src/cli.rs` — the `Drive` subcommand (D-01, D-05)

**Analog:** itself. The whole file is 35 lines and every convention is visible in it:

```rust
#[derive(Parser)]
#[command(
    name = "gsd-meta-manager",
    version,
    about = "TUI command center for GSD projects"
)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Option<Commands>,

    /// Path to config file (overrides default location)
    #[arg(long, global = true)]
    pub config: Option<PathBuf>,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Add a GSD project to the registry
    Add {
        /// Path to the project root (must contain .planning/)
        path: PathBuf,
        /// Optional alias (defaults to last folder component of path)
        alias: Option<String>,
    },
    // ...
}
```

Conventions to copy: struct-variant subcommands (never tuple), a `///` doc on **every**
field (clap renders it as help text), positional for the required argument and `Option<T>`
for the optional one. `Drive` is a struct variant with `alias: String` positional plus
`command`/`run_id`/`dry_run`/`goal` as `#[arg(long)]`.

**D-05 constraint:** the variant itself must **not** be `#[cfg(unix)]` — the subcommand
parses everywhere and the *handler* returns a typed platform error. Only the `src/driver/`
implementation modules carry the `cfg`, mirroring `src/executor/mod.rs:22-28`:

```rust
// `claude` is Unix-only by construction: `process-wrap`'s `ProcessGroup` and
// its `signal()` method are both `#[cfg(unix)]`, and process-group teardown is
// the entire reason the dependency is here (D-03, D-14). Driving is a Unix
// capability in v2.0; the wire model, the gate and outcome derivation stay
// portable and testable everywhere.
#[cfg(unix)]
pub mod claude;
pub mod gate;
```

---

### `src/main.rs` — the `Drive` dispatch arm (D-01)

**Analog:** the `Add` arm, `src/main.rs:34-56`. The house shape is: load config, validate,
`eprintln!` + `std::process::exit(1)` on a user error, `println!` on success — **not**
`anyhow::bail!` for user-facing refusals:

```rust
    match cli.command {
        Some(Commands::Add { path, alias }) => {
            let canonical_path = path.canonicalize().unwrap_or(path);
            // ...
            let config = load_config(&config_path)?;
            if config.projects.contains_key(&alias) {
                eprintln!(
                    "Error: alias '{}' already exists. Provide an explicit alias: gsd-manager add {} <alias>",
                    alias,
                    canonical_path.display()
                );
                std::process::exit(1);
            }
            let mut config = config;
            add_project(&mut config, &alias, &canonical_path)?;
            save_config(&config, &config_path)?;
            println!("Added project '{}' at {}", alias, canonical_path.display());
        }
        // ...
        None => {
            // TUI mode
            let mut terminal = tui::init();
```

**Load-bearing placement:** `tui::init()` is at `src/main.rs:83`, *inside* the `None =>` arm.
A `Some(Commands::Drive { .. })` arm added anywhere in this `match` is by construction before
it, satisfying ARCHITECTURE M8/M9 ("the driver never touches ratatui") without a new mechanism.
Note the logging setup at `:17-26` (`tracing_appender::rolling::daily` into
`~/.local/share/gsd-meta-manager/`) runs before the match, so the driver process inherits
file-based tracing for free — and `color_eyre::install()` at `:28` likewise.

**Startup reconciliation site (D-13)** is the existing startup session scan, `src/main.rs:128-135`:

```rust
            // Startup scan: auto-register any active Claude sessions whose
            // working_dir is an unregistered GSD project. The session poll
            // tick handles the same logic ongoing (every ~5s), but this
            // closes the gap between launch and the first poll.
            let initial_sessions = gsd_meta_manager::session_detector::detect_sessions();
            app.active_sessions = initial_sessions.clone();
            app.ctx.active_sessions = initial_sessions;
            app.auto_register_new_sessions();
```

Note this one is a **synchronous** call at startup (not `spawn_blocking`) — the TUI is not
yet in its loop. The reconciliation scan can follow that precedent verbatim; its tick-time
sibling must use `spawn_blocking` (below).

---

### Detached spawn + process group (D-01, D-02, D-04)

**Analog:** `src/executor/claude.rs:333-365`. This is the repo's only spawn seam and the
source of the "child is group leader so pgid == pid" reasoning D-04 reuses. **Copy the
shape and the comments, not the mechanism** — the new code is plain
`tokio::process::Command` + `std::os::unix::process::CommandExt::process_group(0)`, never
`CommandWrap`/`ProcessGroup::leader()`:

```rust
        let mut wrap = CommandWrap::with_new(&program, |cmd| {
            cmd.args(&argv)
                .current_dir(&cwd)
                .stdin(Stdio::piped())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped());

            // The TUI is plausibly launched from inside a Claude Code session,
            // so inherited CLAUDE* variables would leak into the driven child
            // and change `-p` behaviour in ways that look like "works on my
            // machine". Scrub them all, then set the one we mean to set.
            for (key, _) in std::env::vars_os() {
                if key.to_string_lossy().starts_with("CLAUDE") {
                    cmd.env_remove(&key);
                }
            }
            cmd.env("CLAUDE_CODE_PRINT_BG_WAIT_CEILING_MS", &bg_ceiling);
        });
        wrap.wrap(ProcessGroup::leader());
        // Backstop only, never the teardown story: `KillOnDrop` is SIGKILL.
        wrap.wrap(KillOnDrop);

        let mut child = wrap.spawn().map_err(|source| SpawnError::Launch {
            program: program.display().to_string(),
            source,
        })?;

        // With a process-group leader wrapper the child *is* the group leader,
        // so the pgid equals the child pid. The concrete group type is not
        // reachable through the returned trait object, so this is how the pgid
        // is obtained — and it is recorded immediately, because a teardown with
        // no group handle is not a teardown.
        let pgid = child.id().ok_or(SpawnError::PidUnavailable)?;
```

Deltas the new spawn takes:
- All three stdio to `Stdio::null()` (D-01), not `piped()`.
- `.kill_on_drop(false)` written out explicitly, and the `KillOnDrop` wrapper **absent** —
  D-02 makes the default load-bearing and the comment above is the precedent for saying so
  in code. Note the existing comment already calls `KillOnDrop` "backstop only, never the
  teardown story"; the driver spawn's comment must say the *opposite* thing loudly.
- The pre-spawn validity check idiom is at `claude.rs:314-317`
  (`if !root.is_dir() { return Err(SpawnError::ProjectRootUnusable { root }) }`) — the
  opt-in gate refusal belongs at the same position.
- The pgid is `std::process::id()` **inside the driver** (D-04), not read from the parent's
  `child.id()` — the parent knows the pid, the driver writes the record (D-03).

---

### Process-group teardown to REUSE (D-06, D-08)

**Analog and literal reuse target:** `src/executor/claude.rs:1482-1542`. Layer 2 of D-06 is
`Executor::cancel()`, which lands in this code. Quoted in full because the plan must not
re-implement any of it:

```rust
/// Send SIGTERM to the process **group** — step 1 of the teardown.
///
/// `signal(15)` and not `start_kill()`, and emphatically not `kill()`: on a
/// process group the wrapper's start-of-kill method sends the **uncatchable**
/// signal, and its combined convenience method is that plus a wait. Reading
/// `kill()` as "terminate politely" is natural and wrong, and taking it would
/// skip the CLI's entire documented clean shutdown — the turn abort, the
/// Bash-tree teardown through its own handler, the `SessionEnd` hooks, and the
/// conventional signal-terminated exit status (D-14, Pitfall B).
fn terminate_group(child: &dyn ChildWrapper) {
    if let Err(err) = child.signal(SIGTERM) {
        tracing::warn!("failed to SIGTERM the claude process group: {}", err);
    }
}
```

```rust
async fn tear_down_group(child: &mut Box<dyn ChildWrapper>) -> Option<ExitStatus> {
    terminate_group(&**child);
    finish_teardown(child, TEARDOWN_GRACE).await
}

/// Steps 2 to 4 of the teardown: wait out `grace`, escalate, then reap.
///
/// A zero `grace` means it has already elapsed elsewhere and the escalation is
/// due now. The final `wait()` is deliberately unbounded and deliberately not
/// raced against anything — see [`tear_down_group`] step 4.
async fn finish_teardown(child: &mut Box<dyn ChildWrapper>, grace: Duration) -> Option<ExitStatus> {
    if !grace.is_zero() {
        if let Ok(result) = tokio::time::timeout(grace, child.wait()).await {
            return result.ok();
        }
    }
    if let Err(err) = child.start_kill() {
        tracing::warn!("failed to SIGKILL the claude process group: {}", err);
    }
    child.wait().await.ok()
}
```

The four-step doc at `:1497-1521` is the **template for the TUI→driver layer's own doc**:
number the steps, say what each earns, and mark which wait is unbounded on purpose.

**Constants idiom** (`claude.rs:102-107`) — D-06's new grace constant copies this shape, and
the SIGTERM constant plus the no-`libc` rationale is the precedent D-08 cites:

```rust
/// SIGTERM, as a raw signal number. `ChildWrapper::signal` takes an `i32`, so
/// no `nix` or `libc` dependency is needed to reach the graceful path.
const SIGTERM: i32 = 15;

/// Grace between SIGTERM and SIGKILL on the process group (D-14).
const TEARDOWN_GRACE: Duration = Duration::from_secs(10);
```

**Dependency-comment idiom for `rustix` (D-08)** — `Cargo.toml:36-40` is the house form for
recording *why* a dep is here and which alternative was declined:

```toml
# `tokio1` is NOT a default feature and the crate is inert without it. This
# dependency's own manifest declares rust-version = "1.87.0", which is where
# the [package] rust-version floor above comes from.
# No line-framing crate is added: NDJSON framing uses tokio::io::BufReader::lines()
# plus an explicit byte-length bound, so tokio-util is deliberately not a dependency.
process-wrap = { version = "9.1.0", features = ["tokio1"] }
```

---

### `/proc` liveness probe (D-10)

**Analog:** `src/session_detector.rs`. Two excerpts; the second is the exact shape D-10 needs.

The liveness idiom — one required read, `ok()?` on failure, dropped by `filter_map`
(`session_detector.rs:21-27, 46-53`):

```rust
/// Detect active Claude Code sessions by inspecting the Linux /proc filesystem.
///
/// Uses `pgrep -x claude` to find PIDs, then reads /proc entries for each.
/// Silently skips any PID where reads fail (stale/exited processes).
/// Uses std::process::Command (not tokio) — called from spawn_blocking.
pub fn detect_sessions() -> Vec<ClaudeSession> {
    // ...
    pids.into_iter().filter_map(build_session).collect()
}

fn build_session(pid: u32) -> Option<ClaudeSession> {
    let proc_path = PathBuf::from(format!("/proc/{}", pid));

    // Read working directory from /proc/PID/cwd symlink
    let working_dir = std::fs::read_link(proc_path.join("cwd")).ok()?;
```

The cmdline NUL-split `windows(2)` flag scan (`session_detector.rs:83-98`) — copy this
function almost verbatim, replacing `--resume` with `--run-id` and comparing against the
expected run id:

```rust
fn read_session_id(pid: u32) -> Option<String> {
    let cmdline = std::fs::read(format!("/proc/{}/cmdline", pid)).ok()?;
    let args: Vec<&[u8]> = cmdline.split(|&b| b == 0).collect();

    for window in args.windows(2) {
        if window[0] == b"--resume" {
            let val = String::from_utf8_lossy(window[1]);
            let val = val.trim();
            if !val.is_empty() {
                return Some(val.to_string());
            }
        }
    }

    None
}
```

D-10's second half (require `gsd-meta-manager` in argv[0]) has no line to copy — `args[0]`
is the first NUL-separated element from the same `Vec<&[u8]>`.

**Note the `cfg` posture:** this module has **no `#[cfg]` guards** except `#[cfg(test)]`.
Reads simply fail and yield `None` off Linux. D-10 says to match that honest failure mode
rather than `cfg`-gating.

**Consumers of this module are the two sites D-13 attaches to:** `src/main.rs:132`
(startup, synchronous) and `src/app.rs:438-449` (tick).

---

### The 20-tick block (D-13) and `spawn_blocking` result-as-`Action` (D-27)

**Analog:** `src/app.rs:438-449`. D-13 explicitly says *reuse this counter, do not add a
second timer*:

```rust
                // Poll for Claude sessions every 20 ticks (~5s at 250ms interval)
                self.session_poll_counter += 1;
                if self.session_poll_counter >= 20 {
                    self.session_poll_counter = 0;
                    if let Some(ref tx) = self.ctx.event_tx {
                        let tx: UnboundedSender<Action> = tx.clone();
                        tokio::task::spawn_blocking(move || {
                            let sessions = crate::session_detector::detect_sessions();
                            let _ = tx.send(Action::SessionsDetected { sessions });
                        });
                    }
                }
```

The richer instance of the same idiom, and the closest analog for a reconciliation task that
must not log content — `App::schedule_journal_tail`, `src/app.rs:276-307`:

```rust
    /// The read runs on `spawn_blocking` with its result returned as an
    /// `Action` on a cloned sender, following the idiom this file already uses
    /// for the re-parse and for session detection. No file I/O on the render
    /// thread (D-16).
    fn schedule_journal_tail(&mut self, alias: &str, project_path: &Path, run_id: &str) {
        let Some(tx) = &self.ctx.event_tx else {
            return;
        };
        let tx = tx.clone();

        let key = (alias.to_string(), run_id.to_string());
        let cursor = self.ctx.journal_cursors.get(&key).copied().unwrap_or_default();
        let (alias_for_task, run_id_for_task) = key;

        let planning_dir = project_path.join(".planning");
        let journal = crate::journal::run_paths(&planning_dir, run_id).journal;

        tokio::task::spawn_blocking(move || {
            let read = match crate::journal::reader::tail_lines(&journal, cursor) {
                Ok(read) => read,
                Err(e) => {
                    // The error KIND only. Neither the path nor the message
                    // body is logged (D-28).
                    tracing::warn!(
                        alias = %alias_for_task,
                        run_id = %run_id_for_task,
                        kind = ?e.kind(),
                        "journal tail failed",
                    );
                    return;
                }
            };
```

**The D-28 change site** — `src/app.rs:663-687`. The `windows(2)` filter here is the literal
duplicate of `reader::seq_gaps` that D-28 says to collapse, and `journal_cursors.insert` at
`:686` is the only writer of the map D-27 must prune:

```rust
            Action::DriverJournalAppended {
                alias,
                run_id,
                records,
                cursor,
            } => {
                // `seq` is monotonic from 1 and exists so a tailing reader can
                // detect gaps (D-03). A gap is reported as a COUNT and never
                // as a parse failure, and never with a record body (D-28,
                // D-30).
                let gaps = records
                    .windows(2)
                    .filter(|pair| pair[1].seq != pair[0].seq + 1)
                    .count();
                if gaps > 0 {
                    tracing::warn!(/* ... */);
                }

                self.ctx.journal_cursors.insert((alias, run_id), cursor);
            }
```

The shared function it collapses onto, `src/journal/reader.rs:263-269`:

```rust
pub fn seq_gaps(records: &[JournalRecord]) -> Vec<(u64, u64)> {
    records
        .windows(2)
        .filter(|pair| pair[1].seq != pair[0].seq + 1)
        .map(|pair| (pair[0].seq, pair[1].seq))
        .collect()
}
```

And the cursor type D-28 widens, `reader.rs:65-72` — note it is `Copy`, which
`schedule_journal_tail`'s `.copied()` depends on; a `{cursor, last_seq}` struct must stay
`Copy` or that call site changes too:

```rust
/// How far into a journal a reader has already consumed.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq)]
pub struct TailCursor {
    /// Byte offset of the first unconsumed byte.
    pub offset: u64,
}
```

`ReadDiagnostics.last_seq` already exists (`reader.rs:244-251`) and is the value to carry.

---

### `src/ui/screens/mod.rs` — sibling maps (D-25, D-27)

**Analog:** itself, `:138-170`. Two existing fields are simultaneously the precedent, the
doc-comment template, and the D-27 leak:

```rust
    /// Per-alias live driver state (D-19).
    ///
    /// A **sibling map**, shaped exactly like `last_refresh` and
    /// `archive_cache` above. Driver state deliberately does NOT live on
    /// `ProjectState`: that type derives `PartialEq`, and `app.rs` uses the
    /// derived equality to suppress the "Updated: {alias}" status message.
    /// Driver state changes every few seconds — one 68-second spike turn
    /// emitted 22 `thinking_tokens` events — so putting it there would flood
    /// the status bar for an entire multi-hour run.
    pub run_states: HashMap<String, RunState>,
```

```rust
    /// Byte offset into each run journal, keyed by `(alias, run_id)` (D-13).
    ///
    /// * A **sibling map**, shaped exactly like `run_states` above and
    ///   `last_refresh` / `archive_cache` below. Phase 16 extends that
    ///   neighbourhood rather than recreating it (D-19).
    /// * It must not live on `ProjectState`: that type derives `PartialEq` and
    ///   `app.rs` uses the derived equality to suppress the "Updated: {alias}"
    ///   status message. ...
    /// * It holds offsets, run ids and counts — **never a file handle or a
    ///   join handle**. `Action` derives `Clone` and a handle is not `Clone`
    ///   (D-20).
    pub journal_cursors: HashMap<(String, String), crate::journal::reader::TailCursor>,
```

The bullet-per-invariant style (`exec_tx`, `:126-137`; `reparse_dispatches`, `:148-157`) is
the form for any new field: *"Two things about this field are load-bearing, and neither is
stylistic"*. Any new field also needs a line in the exhaustive `AppContext` literal in
`App::from_config` (`src/app.rs:140-163`).

**D-27's leak is confirmed:** `registry::remove_project` (`src/registry.rs:76-81`) touches
neither map —

```rust
/// Remove a project from the registry by alias.
pub fn remove_project(config: &mut Config, alias: &str) -> Result<()> {
    if config.projects.remove(alias).is_none() {
        bail!("Project not found: {}", alias);
    }
    Ok(())
}
```

— but the *UI* removal path already cleans three sibling maps, `delete_confirm.rs:88-90`,
and is the analog for where the driver maps should be cleaned on an interactive removal:

```rust
            ctx.project_states.remove(alias);
            ctx.detail_sub_view_per_project.remove(alias);
            ctx.last_refresh.remove(alias);
```

---

### `src/config.rs` — `DriverOptIn`, migration (D-14, D-15, D-18)

**Analog:** itself, the whole file (88 lines). The target structs, `:8-34`:

```rust
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub version: u32,
    pub projects: HashMap<String, RegisteredProject>,
    #[serde(default)]
    pub preferences: Preferences,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RegisteredProject {
    pub path: PathBuf,
    pub added: String,
}

#[derive(Debug, Serialize, Deserialize, Default, Clone)]
pub struct Preferences {
    #[serde(default)]
    pub hooks: HooksConfig,
    #[serde(default)]
    pub gsd_integration: bool,
}
```

Note: `Preferences` derives `Default` and every field is `#[serde(default)]` — that is the
pattern `driver_max_concurrent: usize` (D-18) follows, except that its default is **1**, not
`usize::default()`, so it needs `#[serde(default = "default_max_concurrent")]` plus a manual
`Default` impl for `Preferences`, or a newtype. `RegisteredProject` has **no** serde
attributes today, so `driver_opt_in` introduces the first one on this struct.

**The atomic write to copy (`:71-87`)** — D-15 names this and `journal/writer.rs`, and names
`queue_md.rs` as the one *not* to copy:

```rust
/// Save config atomically using tempfile + rename.
pub fn save_config(config: &Config, path: &Path) -> anyhow::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .with_context(|| format!("Failed to create config directory: {}", parent.display()))?;
    }

    let dir = path.parent().unwrap_or(Path::new("."));
    let mut tmp = NamedTempFile::new_in(dir).context("Failed to create temp file for config")?;
    let json = serde_json::to_string_pretty(config).context("Failed to serialize config")?;
    tmp.write_all(json.as_bytes())
        .context("Failed to write config to temp file")?;
    tmp.persist(path)
        .with_context(|| format!("Failed to persist config to {}", path.display()))?;

    Ok(())
}
```

`src/journal/writer.rs:479-506` is the same idiom plus the 0644 permission fix and, at
`:466-478`, the doc that explicitly rejects `queue_md.rs`'s form — read that doc before
writing the migration:

```rust
/// Two deliberate departures from the `src/config.rs:71-87` idiom this mirrors:
/// the record is serialised *pretty*, ... and under unix its mode is set to 0644
/// before the persist, because `persist` preserves the temp file's 0600 ...
///
/// `queue_md.rs`'s fixed-name-plus-rename write is deliberately **not** copied:
/// a fixed temporary name collides if two writers ever race ...
pub fn write_run_record(paths: &RunPaths, record: &RunRecord) -> anyhow::Result<()> {
    let mut handle = NamedTempFile::new_in(&paths.dir).with_context(|| { /* ... */ })?;
    let json = serde_json::to_string_pretty(record).context("Failed to serialize the run record")?;
    handle.write_all(json.as_bytes())/* ... */;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        handle.as_file()
            .set_permissions(std::fs::Permissions::from_mode(0o644))/* ... */;
    }
    handle.persist(&paths.run_json)/* ... */;
    Ok(())
}
```

**Migration *structure* analog (read-side fallback + write-side one-shot):**
`src/state_reader/queue_md.rs:177-230`. Copy the **structure and the rationale-on-the-path-
helper habit**, not the `.tmp` write at `:223-225` (marked below):

```rust
/// The queue lives in an app-namespaced subdirectory (`meta-manager/`) rather
/// than at the `.planning/` root to satisfy GSD 1.8.0's `/gsd-health` W019
/// check. ... Placing the queue under a subdir therefore produces zero health findings.
fn queue_paths(planning_dir: &Path) -> (PathBuf, PathBuf) {
    let canonical = planning_dir.join("meta-manager").join("QUEUE.md");
    let legacy = planning_dir.join("QUEUE.md");
    (canonical, legacy)
}

/// Reads the canonical ... first; if it is absent, falls back to the legacy ...
/// so existing installs keep working with no user action.
pub fn load_queue(planning_dir: &Path) -> Vec<QueuedAction> {
    let (canonical, legacy) = queue_paths(planning_dir);
    if let Ok(content) = std::fs::read_to_string(&canonical) {
        return parse_queue_md(&content);
    }
    match std::fs::read_to_string(&legacy) { /* ... */ }
}
```

```rust
    // NOT THIS PART (D-15: a fixed temp name collides under a race):
    // let tmp_path = meta_dir.join("QUEUE.md.tmp");
    // std::fs::write(&tmp_path, &content)?;
    // std::fs::rename(&tmp_path, &canonical)?;
    // One-shot migration: remove the legacy root queue now that the canonical
    // location holds the current queue.
    let _ = std::fs::remove_file(&legacy);
```

`load_config` (`:59-69`) is the read path the version bump touches; note it returns
`Config::new()` for an absent file and `Config::new()` hardcodes `version: 1` at `:50-56`.

**Silent-enrollment hazard site (D-15's test target):** `registry::add_project` at
`src/registry.rs:63-70` constructs `RegisteredProject` with a struct literal, so adding a
field is a compile error there and in `auto_register_from_sessions` — which is exactly the
mechanical proof the new field is never set by the discovery path:

```rust
    let now = chrono::Utc::now().to_rfc3339();
    config.projects.insert(
        alias.to_string(),
        RegisteredProject {
            path: path.to_path_buf(),
            added: now,
        },
    );
```

(`chrono::Utc::now().to_rfc3339()` here is also the timestamp idiom for `DriverOptIn::opted_in_at`;
`JournalRun::finish` uses the seconds-precision variant,
`chrono::Utc::now().to_rfc3339_opts(chrono::SecondsFormat::Secs, true)` at `journal/mod.rs:625`
— pick one and say why.)

---

### `src/executor/mod.rs` — `from_registry` and the renamed escape hatch (D-16, D-17)

**Analog:** itself, `:109-151`. The doc comment already names Phase 17 as the owner of the
change:

```rust
/// Proof that the user designated a project as drivable.
///
/// This is the D-23 capability token, and it is a *type* on purpose: the
/// compiler, not a code review, is what enforces the single spawn seam.
/// [`Executor::start`] takes this and never a bare path and never a `bool`,
/// so a call site cannot spawn an agent against a directory the user never
/// opted in.
///
/// The fields are private, so only this module can construct one. Phase 17
/// supplies the validated `driver_opt_in` config record as the **only**
/// production constructor; [`DrivableProject::for_testing`] is the explicitly
/// named test/dev escape hatch until then. ...
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DrivableProject {
    alias: String,
    root: PathBuf,
}

impl DrivableProject {
    /// Construct a token without a validated opt-in record.
    ///
    /// **Test and development only.** Phase 17 adds the production constructor
    /// that reads the user's `driver_opt_in` record; until it exists this is
    /// the only way to build one, and the name is deliberately loud.
    pub fn for_testing(alias: impl Into<String>, root: impl Into<PathBuf>) -> Self {
```

Both doc comments are stale on the day `from_registry` lands and must be rewritten, not
appended to. D-16 requires the "both entry points are gated by the same code" claim to live
in the type's doc.

**All 14 call sites are in `tests/executor_lifecycle.rs` and `tests/executor_transport.rs`**
— the rename is a mechanical two-file sweep plus the guard test.

---

### `src/error.rs` — `OptInError`, `LockError` (D-16, D-20)

**Analog:** itself, `:1-60`. The module header states the convention the new error types must
follow, including the no-`thiserror` line:

```rust
// Error types for gsd-meta-manager.
//
// The rest of the codebase uses `anyhow::Result` + `.with_context()` at every
// seam existing callers touch, and that stays true (see `config.rs`,
// `registry.rs`, `git_ops.rs`). Only the executor's own surface uses the typed
// errors below: a driver UI needs a *state* to render, not a message, and
// PITFALLS Pitfall 10 assigns "widen the error type before the driver" to this
// phase explicitly (D-12).
//
// `Display` and `std::error::Error` are hand-written rather than derived. Three
// small enums do not justify an error-derive dependency, and keeping the Cargo
// surface to exactly the two crates plan 15-01 added (`process-wrap`, `uuid`)
// is a deliberate line, not an oversight.
```

The variant-doc convention, `SpawnError` `:18-45`:

```rust
/// Why a run could not be started.
///
/// Every variant is reachable before any turn begins, which is the point: a
/// refusal at this stage costs zero tokens and zero quota (D-06).
#[derive(Debug)]
pub enum SpawnError {
    /// The drivable project's root is not an existing directory. Checked before
    /// the process is launched so a stale registry entry cannot spawn an agent
    /// against a path that no longer exists (T-15-06).
    ProjectRootUnusable {
        /// The root that failed the check.
        root: PathBuf,
    },
    // ...
    /// The child's pid was not readable after spawn, so the process group id
    /// could not be recorded. Without it there is no teardown handle (D-14).
    PidUnavailable,
```

D-05's "driving is not supported on this platform" and D-20's "held by run X / held by an
unknown run" are variants in this style — struct variants with per-field docs, hand-written
`Display`.

---

### `src/driver/dry_run.rs` + `src/state_reader/git_ops.rs` (D-22, D-23)

**Analog A — the git shell-out signature and failure posture:** `git_ops.rs:96-153`. The
diffstat and refspec helpers extend **this module** (CONTEXT: "do not start a second git
wrapper") and copy the `-C`-not-`current_dir`, `Option`-not-`Result`, never-panics shape:

```rust
/// A project's current git `HEAD` commit sha, in full.
///
/// Synchronous by design, matching `git_last_commit_time` above: its consumer
/// is the run snapshot in `src/executor/outcome.rs`, which calls it from the
/// same blocking closure as `parse_project_state`. Passes the repository root
/// as an argument (`-C`) rather than setting a working directory, consistently
/// with [`is_dirty`].
///
/// Returns `None` — never an error — when the directory is not a git
/// repository, has no commits yet, or git is unavailable. ... Never panics.
pub fn head_sha(project_root: &Path) -> Option<String> {
    let output = std::process::Command::new("git")
        .arg("-C")
        .arg(project_root)
        .args(["rev-parse", "HEAD"])
        .output()
        .ok()?;

    if !output.status.success() {
        return None;
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let trimmed = stdout.trim();
    if trimmed.is_empty() {
        return None;
    }

    Some(trimmed.to_string())
}
```

```rust
/// Whether a project's working tree carries any uncommitted change.
///
/// **Untracked files count as dirty.** `git status --porcelain` reports them by
/// default, and an agent that creates a new file without committing it has
/// unambiguously changed the project ...
pub fn is_dirty(project_root: &Path) -> Option<bool> {
    let output = std::process::Command::new("git")
        .arg("-C").arg(project_root)
        .args(["status", "--porcelain"])
        .output().ok()?;
    if !output.status.success() { return None; }
    let stdout = String::from_utf8_lossy(&output.stdout);
    Some(!stdout.trim().is_empty())
}
```

`GitDiffStat { files_changed, insertions, deletions, file_stats }` already exists at
`git_ops.rs:163-169` — D-22's diffstat should reuse or extend it rather than define a second
shape.

**Analog B — "inspect the output, not the exit status":** `src/journal/writer.rs:308-359`.
D-22's refspec computation reads `git config` output, where an exit status is likewise not
the answer; this doc is the precedent for saying so:

```rust
/// **The polarity looks backwards on purpose.** `git check-ignore` answers
/// *"did a pattern match?"*, **not** *"is this file ignored?"*, and a negation
/// counts as a match — so a bare quiet-mode invocation exits 0 for a file our
/// own `!*/run.json` rule deliberately re-includes, reporting the precise
/// inverse of the truth. A first pass of RESEARCH §1.2 made exactly that
/// mistake before the ground-truth check corrected it. That is why this
/// function inspects the *reported pattern* rather than the exit status.
pub fn parent_excludes_run_record(project_root: &Path, run_json: &Path) -> bool {
    let output = match std::process::Command::new("git")
        .arg("check-ignore").arg("-v").arg("--").arg(run_json)
        .current_dir(project_root)
        .output()
    {
        Ok(output) => output,
        // git is not installed, or is not executable here. Not our problem.
        Err(_) => return false,
    };
    // Exit 1 means no pattern matched at all; 128 means "not a repository".
    if !output.status.success() { return false; }
    let stdout = String::from_utf8_lossy(&output.stdout);
    // `-v` prints `<source>:<lineno>:<pattern>\t<pathname>`.
    let Some((reported, _pathname)) = stdout.lines().next()?.split_once('\t') else { /* ... */ };
```

Note this one uses `current_dir` rather than `-C`, so the repo has both forms; `git_ops.rs`
documents `-C` as its own convention, and new helpers landing in `git_ops.rs` should follow
the module they live in.

**Blocking discipline:** `src/executor/outcome.rs:60` documents that `RunSnapshot::capture`
*"shells out to git twice"* and must run on a blocking thread. Every new helper inherits
that; the dry-run path is a CLI foreground invocation (D-24) so it can call them directly,
but any TUI-side caller goes through `spawn_blocking`.

---

### `src/journal/` wiring — first production caller (in scope)

**Analog:** `src/journal/mod.rs:566-695`. `JournalRun` is complete; the driver's job is to
call it in the documented order. The lifecycle contract, `mod.rs:529-550`:

```rust
/// ```text
/// JournalRun::start(planning, record)   -> prune, mkdir+ignore, run.json #1, active, journal, run_started
/// JournalRun::record_exec(&event)       -> zero or more, one per observed ExecutionEvent
/// JournalRun::finish("completed")       -> run_ended, stamp, run.json #2, clear active
/// ```
///
/// **`run.json` is written exactly twice across that lifetime and nothing else
/// ever rewrites it** (D-06). ...
///
/// **Phase 16 drives this from a synthetic event sequence.** Nothing in this
/// repository spawns a `ClaudeExecutor` yet; wiring a real one is Phase 17's
/// (D-36).
```

`start` (`:587-611`) shows the exact call the driver makes and the two fields Phase 17 fills:

```rust
    pub fn start(planning_dir: &Path, record: RunRecord) -> anyhow::Result<Self> {
        writer::prune_runs(planning_dir, RETAIN_RUNS, Some(&record.run_id))?;

        let paths = writer::create_run_dir(planning_dir, &record.run_id)?;
        writer::write_run_record(&paths, &record)?;
        writer::write_active_pointer(&runs_root(planning_dir), &record.run_id)?;

        let mut writer = writer::JournalWriter::open(&paths.journal)?;
        writer.append(&JournalEvent::RunStarted {
            goal: record.goal.clone(),
            // Phase 17 owns dry-run; until it exists every run is a real one.
            // The field is written now so that phase adds no schema migration.
            dry_run: false,
            target: record.target.clone(),
        })?;
```

`RunRecord` (`mod.rs:461-497`) is the struct the driver populates; `opt_in: Option<String>`
at `:475` is already reserved for D-14's record and `ended_at`'s doc at `:488-494` states the
contract D-12 depends on:

```rust
    /// **`None` is precisely the signal Phase 17's crash reconciliation reads**
    /// (D-06, D-32): a run directory whose record has no `ended_at` is a run
    /// that never reached a terminal transition, and retention must never prune
    /// it.
    pub ended_at: Option<String>,
```

`finish` (`:624-643`) is what the SIGTERM handler calls in D-06 step 3, including the
assertion D-12 must not break:

```rust
        writer::write_run_record(&self.paths, &self.record)?;
        self.record_writes += 1;
        debug_assert_eq!(
            self.record_writes, 2,
            "run.json is written exactly twice and nothing else ever rewrites it (D-06)"
        );

        writer::clear_active_pointer(self.runs_root())?;
```

**D-09's field addition site:** `JournalEvent::ExecStarted` (`mod.rs:337-342`) and its
projection (`mod.rs:773-776`):

```rust
    ExecStarted {
        /// The session UUID the CLI reported at `system/init`.
        session_id: String,
        /// [`argv_digest`] of the spawned command line.
        argv_digest: String,
    },
```

```rust
        ExecutionEvent::SessionStarted { session_id, .. } => JournalEvent::ExecStarted {
            session_id: session_id.clone(),
            argv_digest: argv_digest.to_string(),
        },
```

The `..` in that pattern is where `pgid` comes from if `SessionStarted` carries it;
otherwise it comes from `ExecutionHandle.pgid` and `record_exec`'s run-scoped-stamp idiom
(`mod.rs:663-674`) is the precedent for a value the per-event projection cannot know:

```rust
        // The two run-scoped fields no single event can know. `from_exec_event`
        // is a pure per-event projection, so it leaves them empty and the run
        // — which owns the clock and the cost total — stamps them here.
```

**Reconciliation reads:** `writer::read_active_run` (`writer.rs:427-443`) is the cheap
pointer read, and its own doc states the directory listing is authoritative:

```rust
pub fn read_active_run(planning_dir: &Path) -> Option<String> {
    let root = runs_root(planning_dir);
    let raw = std::fs::read_to_string(root.join("active")).ok()?;
    let run_id = raw.trim();
    if run_id.is_empty() { return None; }
    if !root.join(run_id).is_dir() {
        tracing::warn!(
            "the active pointer under {} names a run directory that does not exist; \
             the directory listing is authoritative, so the pointer is ignored",
            root.display()
        );
        return None;
    }
    Some(run_id.to_string())
}
```

`has_end_timestamp` (`writer.rs:523-539`) is the schema-agnostic `ended_at` read
reconciliation should reuse rather than deserialising `RunRecord`:

```rust
/// Read as a `Value` rather than through [`RunRecord`] deliberately: a record
/// written by a later schema must still be prunable, and the only field this
/// question needs is `ended_at` (D-30's tolerance, applied to a write path).
/// A missing, unreadable or unparseable record answers `false`, which keeps it.
fn has_end_timestamp(run_dir: &Path) -> bool {
```

(It is currently private — reconciliation needs an equivalent, so either make it `pub` or
write the sibling in the driver module; either way copy the *Value-not-struct* reasoning.)

---

### `src/ui/screens/driver_confirm.rs` (D-25)

**Analog:** `src/ui/screens/delete_confirm.rs` — the whole file, 113 lines, is the smallest
complete `Screen` in the tree and is exactly the "one key binding and one confirmation"
shape D-25 asks for.

```rust
pub struct DeleteConfirmScreen {
    pub alias: String,
}

impl Screen for DeleteConfirmScreen {
    fn handle_key(
        &mut self,
        code: KeyCode,
        _modifiers: KeyModifiers,
        ctx: &mut AppContext,
    ) -> ScreenAction {
        match code {
            KeyCode::Char('y') => {
                do_remove_project(ctx, &self.alias);
                ScreenAction::Pop
            }
            KeyCode::Char('n') | KeyCode::Esc => {
                ctx.needs_redraw = true;
                ScreenAction::Pop
            }
            _ => ScreenAction::None,
        }
    }

    fn render(&self, frame: &mut Frame, area: Rect, _ctx: &AppContext) {
        let chunks = ratatui::layout::Layout::vertical([
            ratatui::layout::Constraint::Min(0),
            ratatui::layout::Constraint::Length(1),
        ])
        .split(area);
        // ... Block background, then a one-line footer prompt:
        let prompt = format!(
            "Remove \"{}\"? This only unregisters it \u{2014} project files are not deleted. [y/n]",
            self.alias
        );
        let line = Line::from(Span::styled(prompt, Style::default().fg(Color::Red)));
        frame.render_widget(Paragraph::new(line), chunks[1]);
    }

    fn name(&self) -> &str { "delete_confirm" }
}
```

Copy: the `[y/n]` one-line footer, the `Color::Red` for a destructive action, the free
function that does the work and reports through `ctx.error_message` / `ctx.status_message`
(`:69-112`), and the `ScreenAction::Pop` on both arms.

**Key registration** is `src/ui/screens/normal.rs:207-226` — one `KeyCode::Char(_)` arm that
pushes the screen:

```rust
            KeyCode::Char('d') => {
                if let Some(alias) = ctx.selected_alias() {
                    ctx.needs_redraw = true;
                    ScreenAction::Push(Box::new(DeleteConfirmScreen::new(alias)))
                } else {
                    ScreenAction::None
                }
            }
```

**Status message idiom:** `ctx.status_message = Some((msg, std::time::Instant::now()))`
(`delete_confirm.rs:92-93`, `app.rs:721`), expiring after 3s at `app.rs:430-436`. D-25's
"refusing visibly when not opted in" is `ctx.error_message = Some(e.to_string())`
(`delete_confirm.rs:109`).

---

### `tests/driver_*.rs` (integration tests)

**Analog:** `tests/executor_lifecycle.rs:1-110`. The header banner + `#![cfg(unix)]` +
`concat!(env!("CARGO_MANIFEST_DIR"), …)` fixture constants are the pattern:

```rust
// ============================================================================
// Process lifecycle: the two deadlines (D-13) and the group teardown (D-14)
//
// Everything here needs a real child process, which is why it is not in-source
// with the parsing tests. Three shell stand-ins supply the behaviours a
// transcript cannot express: going silent, spawning a grandchild, and ignoring
// the terminate signal.
//
// Unix-only by construction. ...
// ============================================================================

#![cfg(unix)]

use gsd_meta_manager::executor::claude::ClaudeExecutor;
use gsd_meta_manager::executor::{
    DrivableProject, ExecutionEvent, ExecutionHandle, ExecutionOptions, Executor, RunOutcome,
};
use tempfile::TempDir;

const FAKE_SLOW: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/fake-claude-slow.sh"
);
const FAKE_SPAWNER: &str = concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/tests/fixtures/fake-claude-spawner.sh"
);
```

**How the fake `claude` is built and located — the exact answer for the kill-switch tests:**

- The fixtures are **checked-in shell scripts**, not compiled artifacts:
  `tests/fixtures/fake-claude{,-slow,-spawner,-deaf,-orphan,-echo}.sh`. There is **no build
  script and no `cargo` step**; they are located by absolute path via
  `concat!(env!("CARGO_MANIFEST_DIR"), "/tests/fixtures/…")`, which is resolved at compile
  time so the test is cwd-independent.
- `fake-claude-spawner.sh` is the one that spawns a **real grandchild** — it is what
  success criterion #1's "no grandchild build or server process" test needs.
- They are injected through `ClaudeExecutor::with_program`, whose doc at
  `src/executor/claude.rs:295-306` states the design rule:

```rust
    /// path entirely — no Cargo feature, no fixture branch in `main`.
    pub fn with_program(program: impl Into<PathBuf>, leading_args: Vec<OsString>) -> Self {
        Self {
            program: program.into(),
            leading_args,
        }
    }
```

  `leading_args` are prepended before the executor's own argv, which is how the shell
  fixtures receive their pacing parameters:

```rust
/// An executor pointed at the paced stand-in.
fn slow(heartbeats: u32, interval: &str, ending: &str) -> ClaudeExecutor {
    ClaudeExecutor::with_program(
        FAKE_SLOW,
        vec![
            OsString::from(heartbeats.to_string()),
            OsString::from(interval),
            OsString::from(ending),
        ],
    )
}
```

  **Phase 17 needs the same seam one level up:** the driver spawns `gsd-meta-manager drive`,
  so the *driver's* executor program must be overridable too, or the kill-switch integration
  test cannot reach a fake `claude`. There is no existing analog for that — flag it.
  `src/executor/claude.rs:1919` shows the in-source tests use the same `CARGO_MANIFEST_DIR`
  constant, so the pattern is already used from both `src/` and `tests/`.
- The observable-constant convention when a private constant matters to a test
  (`executor_lifecycle.rs:43-46`) — D-06's grace value needs the same treatment against
  success criterion #1's 15s:

```rust
/// The real grace between the terminate signal and the uncatchable one. The
/// constant itself is private to the executor; this is the observable value the
/// ignored tests wait out.
const TEARDOWN_GRACE: Duration = Duration::from_secs(10);
```

- The process-liveness helpers (`:88-110`) are directly reusable for the zombie check:

```rust
/// Whether `pid` still exists, via the zero signal.
///
/// The shell builtin rather than `/bin/kill` so this works wherever `sh` does,
/// and `pid` is an integer so the interpolation cannot carry anything else.
fn alive(pid: u32) -> bool {
    std::process::Command::new("sh")
        .arg("-c")
        .arg(format!("kill -0 {pid} 2>/dev/null"))
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

/// Poll until `pid` is gone, giving up after `limit`.
///
/// A signal is delivered asynchronously and a just-signalled process is briefly
/// still a pid, so the assertion has to be "gone soon" rather than "gone now".
async fn gone_within(pid: u32, limit: Duration) -> bool { /* ... */ }
```

Note `alive()` uses `kill -0` and answers "exists", which **includes zombies** — success
criterion #1's "no zombie" half needs a `/proc/<pid>/stat` state-`Z` check, which does not
exist yet; `session_detector::read_start_time` (`:100-109`) shows how this repo parses
`/proc/<pid>/stat` past the parenthesised `comm` field.

---

## Shared Patterns

### Doc comments carry the rationale and the rejected alternative
**Source:** `src/executor/mod.rs:1-26`, `:109-122`; `src/executor/claude.rs:1482-1521`;
`src/journal/writer.rs:466-478`; `src/ui/screens/mod.rs:126-137`; `src/error.rs:1-13`
**Apply to:** every new public item this phase adds.
House style: bold the load-bearing claim, cite the decision id `(D-06)`, name what the code
deliberately does *not* do, and name the alternative that was declined and why. Phase 17 has
an unusual number of "the naive version silently fails" decisions (D-02's `kill_on_drop`,
D-20's four lock properties, D-12's zero writes) — each belongs in a doc comment the
acceptance criteria can point at.

### `anyhow::Result` + `.with_context()` everywhere except a rendered state
**Source:** `src/config.rs:59-87`, and the rule stated at `src/error.rs:1-13`
**Apply to:** the lock, the migration, the journal wiring. Typed errors only where the TUI
must render a *state* (D-05 platform refusal, D-16 opt-in refusal, D-20 lock-loser).

### `tracing::warn!` for non-fatal degradation, structured fields, no content
**Source:** `src/executor/claude.rs:1493, 1539`; `src/journal/writer.rs:435-440`;
`src/app.rs:296-343`
```rust
                    tracing::warn!(
                        alias = %alias_for_task,
                        run_id = %run_id_for_task,
                        kind = ?e.kind(),
                        "journal tail failed",
                    );
```
**Apply to:** every failed signal, failed probe, and lock contention. Never `error!`, never a
panic, never a message body.

### `spawn_blocking` + result-returned-as-`Action` on a cloned sender
**Source:** `src/app.rs:293-350`, `:442-448`, `:301-308`
**Apply to:** the tick-time reconciliation probe and any TUI-side git read. No sync fs or
process I/O on the render thread.

### `#[cfg(test)] mod tests` at the bottom of each module; `tests/*.rs` only when a real
process is required
**Source:** `src/journal/reader.rs:560-610`, `src/watcher.rs:92-117`, and the banner at
`tests/executor_lifecycle.rs:1-12` explaining *why* a test is an integration test
**Apply to:** dry-run output shaping, refspec computation, migration, `/proc` cmdline parse
(all in-source); kill switch, lock contention, cross-project isolation (all `tests/`).

### `#[cfg(unix)]` on the implementation, never on the public surface
**Source:** `src/executor/mod.rs:22-30`, `src/journal/writer.rs:492-499`
**Apply to:** the whole `src/driver/` tree per D-05 — but the `Commands::Drive` variant and
its typed platform error stay portable.

---

## No Analog Found

| File / concern | Role | Data Flow | Reason | What to use instead |
|---|---|---|---|---|
| `flock(2)` advisory lock (D-19, D-20) | service | file-I/O | **No file locking of any kind exists in the repo** — no `nix`, `libc`, `fs4`, `fs2`, or `flock` in `src/` or `Cargo.toml`. `NamedTempFile` + `persist` is the only concurrency primitive on disk | `rustix::fs::flock` with `LOCK_EX \| LOCK_NB` (D-08); the *file layout* analog is `journal/mod.rs:130-148` (`RunPaths` + `runs_root`), and the holder-metadata write follows `write_run_record`'s atomic idiom — but the lock file itself must be `open`ed and **held**, never persisted-over |
| Detached spawn with `process_group(0)` + `Stdio::null()` (D-01, D-02) | service | process-lifecycle | Every spawn in the tree is either `process-wrap` with piped stdio (`claude.rs:333`) or a synchronous `Command::output()` (`git_ops.rs`, `session_detector.rs:31`). Nothing detaches, nothing nulls stdio, and `std::os::unix::process::CommandExt` appears nowhere | `claude.rs:333-365` for the *shape* (env scrub, error mapping, immediate pgid record) with the mechanism replaced; D-02 spells out the `tokio::process::Command` + `as_std_mut()` + reaping-task form |
| Mechanical grep-guard test walking `src/**` (D-17) | test | file-I/O | **The technique is documented in prose only and is not implemented.** `grep -rn deny_unknown_fields src/ tests/` returns exactly one hit — the sentence at `src/journal/mod.rs:34` claiming a guard exists. There is no test, no build script, no CI step. `src/journal/reader.rs:8-13` even explains why the attribute name is *not* spelled in prose there, which only makes sense if a grep exists | Write the first one. It is a `tests/*.rs` that recursively `read_dir`s `concat!(env!("CARGO_MANIFEST_DIR"), "/src")` — the recursive-listing idiom is at `src/journal/writer.rs:571` and `src/archive.rs:116`; the manifest-dir constant idiom is at `tests/executor_lifecycle.rs:26-29`. **Consider covering `deny_unknown_fields` in the same test** and thereby retiring a currently-false claim |
| `Z`-state (zombie) detection | test | file-I/O | `tests/executor_lifecycle.rs:88` `alive()` uses `kill -0`, which reports a zombie as alive | Parse `/proc/<pid>/stat` field 3; the comm-field-skipping parse is at `src/session_detector.rs:100-109` |
| SIGTERM handler in a long-lived process (D-06 step 2) | service | event-driven | Nothing in the tree installs a signal handler. `tokio::signal` is available (`tokio` has `features = ["full"]`) but unused | `tokio::signal::unix::signal(SignalKind::terminate())` raced in the driver's main `select!`; the `biased` select precedent is `src/main_loop.rs:130` |
| Directory-tree hashing for "project B is byte-identical" (criterion #4) | test | batch | No tree-hashing helper exists | `src/journal/writer.rs:571` / `src/archive.rs:116` recursive `read_dir`, or shell out to `git status --porcelain` per the `is_dirty` precedent |

---

## Notes for the Planner

1. **Three excerpts here are explicitly anti-patterns**, included because a pattern-matching
   implementer would reach for them: `queue_md.rs`'s fixed-`.tmp` write (`:223-225`, D-15
   forbids), `process-wrap`'s `KillOnDrop` near the detached spawn (`claude.rs:353` — the
   inverse of what D-02 requires), and `alive()`'s `kill -0` as a zombie check
   (`executor_lifecycle.rs:92`).
2. **`src/journal/mod.rs:34-38` and `reader.rs:8-13` claim a grep guard that does not exist.**
   D-17 asks for the "same technique"; there is no technique to copy. This is the phase's one
   genuinely unprecedented test, and it can retire the false claim at the same time.
3. **The fake-`claude` fixture seam does not reach the driver.** `ClaudeExecutor::with_program`
   injects a fake into the *executor*; Phase 17's driver is a separate process that constructs
   its own executor. The kill-switch and no-cross-project tests need an equivalent override on
   the `drive` path (an env var or a hidden flag), and no analog for it exists. Design it
   deliberately — `with_program`'s doc (`claude.rs:295-306`) states the rule it must satisfy:
   "no Cargo feature, no fixture branch in `main`".
4. **`TailCursor` is `Copy` and `app.rs:287` calls `.copied()`.** D-28's widened
   `{cursor, last_seq}` must stay `Copy` or that call site changes too.
5. **`Preferences` derives `Default`**, so `driver_max_concurrent` defaulting to 1 (not 0)
   needs either `#[serde(default = "…")]` plus a hand-written `Default for Preferences`, or a
   newtype. This is the one place D-18's "almost free" costs more than a field.
6. **Adding a field to `RegisteredProject` breaks two struct literals**
   (`registry.rs:64-70` and the one in `auto_register_from_sessions`), which is exactly the
   compile-time proof D-15's test wants — note it in the plan so the breakage is read as the
   feature it is.
7. **`cargo clippy --all-targets` has exactly 5 pre-existing lints** (`browser.rs` ×3,
   `project_creator.rs` ×1, `state_reader/mod.rs` ×1). None of those files appear in this map;
   none should be opened.
8. **`rtk` filters raw `cargo` output.** Any acceptance criterion that greps for `warning:` or
   `test result:` must use `rtk proxy cargo …` or it passes vacuously.

---

## Metadata

**Analog search scope:** `src/` (all 18 root modules plus `executor/`, `journal/`,
`ui/screens/`, `state_reader/`), `tests/`, `tests/fixtures/`, `Cargo.toml`
**Files read this pass:** 17 — `cli.rs`, `config.rs`, `session_detector.rs`, `lib.rs`,
`ui/screens/delete_confirm.rs` in full; `main.rs`, `app.rs`, `registry.rs`,
`executor/mod.rs`, `executor/claude.rs`, `journal/mod.rs`, `journal/writer.rs`,
`journal/reader.rs`, `state_reader/git_ops.rs`, `state_reader/queue_md.rs`, `error.rs`,
`ui/screens/normal.rs`, `tests/executor_lifecycle.rs` in targeted ranges
**Pattern extraction date:** 2026-07-29
