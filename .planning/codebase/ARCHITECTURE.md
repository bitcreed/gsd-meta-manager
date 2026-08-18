<!-- refreshed: 2026-08-18 -->
# Architecture

**Analysis Date:** 2026-08-18

## System Overview

The binary has two independent execution modes selected by `src/main.rs` before anything terminal-related runs: **TUI mode** (no subcommand) and **drive mode** (`gsd-meta-manager drive <alias> ...`). They share the read-side (`state_reader`) and the process-spawn/journal machinery, but the driver never touches ratatui and the TUI never runs an agent turn in-process.

```text
┌──────────────────────────── gsd-meta-manager (single binary) ───────────────────────────┐
│                                                                                            │
│  TUI MODE (src/main.rs None arm)              DRIVE MODE (src/main.rs Drive arm)         │
│  ┌──────────────────────────────┐             ┌────────────────────────────────────┐    │
│  │ event::EventBus               │             │ src/driver/mod.rs::drive()          │    │
│  │  crossterm reader + 250ms tick│             │  gate → DrivableProject::from_reg   │    │
│  └───────────────┬────────────────┘             │  → dispatch → run::execute_run      │    │
│                  │ Action (unbounded mpsc)      └───────────────┬────────────────────┘    │
│                  ▼                                               │ spawns via              │
│  ┌──────────────────────────────┐                                │ driver/spawn.rs         │
│  │ main_loop::pump (src/main_loop.rs) │◄── ExecEvent (bounded 8192)│ (this binary, detached)│
│  │  biased select!: Action > Exec  │                              ▼                        │
│  │  events(batch<=64) > redraw tick│               ┌────────────────────────────────┐      │
│  └───────────────┬────────────────┘                │ executor::claude::ClaudeExecutor│      │
│                  │ app.update(action)               │  spawns `claude` (stream-json)  │      │
│                  ▼                                  └───────────────┬─────────────────┘      │
│  ┌──────────────────────────────┐                                   │ events               │
│  │ App (src/app.rs)              │                                   ▼                       │
│  │  screen_stack: Vec<Box<Screen>>│              ┌────────────────────────────────┐         │
│  │  ctx: AppContext (shared state)│              │ journal::writer (src/journal/)  │         │
│  └───────────────┬────────────────┘              │  append-only NDJSON per run      │         │
│                  │ render(frame, app)             └───────────────┬─────────────────┘         │
│                  ▼                                                 │ tailed by                │
│  ┌──────────────────────────────┐                                  ▼                          │
│  │ ui::render → active Screen    │              ┌────────────────────────────────┐            │
│  │  (src/ui/, src/ui/screens/)   │◄─────────────│ TUI reads journal via FileWatcher│            │
│  └──────────────────────────────┘  FileChanged   │  + journal::reader (byte-offset  │            │
│                                      Action        │  tail) → Action::DriverJournalAppended│    │
└────────────────────────────────────────────────────────────────────────────────────────┘

           ┌────────────────────────────────────────────────────────────┐
           │  READ SIDE (shared by both modes): state_reader/*           │
           │  Parses N projects' `.planning/` trees (STATE.md, ROADMAP.md,│
           │  queue.md, backlog, config.json, git log) into ProjectState  │
           │  Cached, invalidated by notify filesystem events (watcher.rs)│
           └────────────────────────────────────────────────────────────┘
```

## Component Responsibilities

| Component | Responsibility | File |
|-----------|----------------|------|
| Entry/mode select | Parse CLI, branch TUI vs Drive vs Add/Remove/List, init tracing | `src/main.rs` |
| Event bus | Crossterm input stream + 250ms tick → unbounded `Action` channel | `src/event.rs` |
| Event loop core | Testable `select!` body: biased Action > bounded ExecEvent batch > redraw tick | `src/main_loop.rs` |
| App state/reducer | Owns `screen_stack`, `AppContext`; `App::update` applies every `Action` | `src/app.rs` |
| Action/message type | The full message vocabulary crossing the TUI's internal bus | `src/action.rs` |
| Screen trait + registry | `Screen::handle_key` / `Screen::render`; `AppContext` (shared mutable state) | `src/ui/screens/mod.rs` |
| Individual screens | One file per modal screen/tab (normal view, driver tab, dialogs) | `src/ui/screens/*.rs` |
| Render dispatch | Frame → active screen on `screen_stack` top | `src/ui/mod.rs` |
| Filesystem watcher | `notify`-based watch of registered projects' `.planning/`, debounced → `Action::FileChanged` | `src/watcher.rs` |
| Project state parsing | Read-only parse of one project's `.planning/` tree into `ProjectState` | `src/state_reader/mod.rs` + submodules |
| Config/registry | User's `~/.config/gsd-meta-manager/config.toml`, add/remove/list projects | `src/config.rs`, `src/registry.rs` |
| Driver orchestration | Gate, dry-run branch, run-id validation, dispatch to Unix run body | `src/driver/mod.rs` |
| Spawn seam | The single call site that launches a detached child process (this binary, re-invoked as `drive`) | `src/driver/spawn.rs` |
| Run body | Acquire lock, construct executor, drive one command, write journal, release lock | `src/driver/run.rs` |
| Lock / concurrency cap | Per-project run lock + `admit()` global concurrency check | `src/driver/lock.rs` |
| Liveness / reconciliation | `/proc`-based liveness probe; startup + periodic scan reconciling on-disk run state | `src/driver/liveness.rs`, `src/driver/reconcile.rs` |
| Kill switch | Two-layer stop: signal driver's process group, driver's own handler cancels agent's group | `src/driver/kill.rs` |
| Dry-run preview | Build and render a preview report with no process spawned | `src/driver/dry_run.rs` |
| Agent executor trait | `Executor` trait: start/send/interrupt/cancel/is_running/capabilities | `src/executor/mod.rs` |
| Claude transport | Concrete `Executor` impl: spawns `claude`, drives `stream-json` duplex protocol | `src/executor/claude.rs` |
| Capability gate | Validates `system/init` capabilities before releasing the first prompt | `src/executor/gate.rs` |
| Outcome derivation | Maps process exit + stream events to a typed `RunOutcome` | `src/executor/outcome.rs` |
| Wire model | Typed `stream-json` message schema | `src/executor/stream_json.rs` |
| Run journal | Append-only NDJSON writer, tolerant reader (byte-offset tail), capture-path redaction | `src/journal/writer.rs`, `src/journal/reader.rs`, `src/journal/redact.rs` |
| Steering inbox | Durable queue for user messages injected into a live run, read via filesystem only | `src/journal/inbox.rs` |
| Session detection | Detects active `claude` CLI sessions to auto-register projects | `src/session_detector.rs` |
| Errors | Typed error enums shared by driver/executor/journal (`DriveError`, `SpawnError`, `SendError`, `OptInError`) | `src/error.rs` |

## Pattern Overview

**Overall:** Elm/Redux-style unidirectional TUI (crossterm event → `Action` → reducer (`App::update`) → render) layered on top of a **filesystem-mediated actor pair**: the TUI process and a detached driver process communicate through nothing but files on disk (registry config, run journal, inbox, lock files) — never a socket, pipe, or shared memory. This is a deliberate, load-bearing decision (D-03 in `src/action.rs`, `src/journal/mod.rs`) so a driven run survives the TUI restarting or closing.

**Key Characteristics:**
- Single `Action` enum is the entire internal message vocabulary; `App::update` is the one reducer (`src/app.rs:948`).
- A **second, separate, bounded** channel (`ExecEvent`, capacity 8192) carries high-volume executor/journal-tail traffic so it can never starve control-key input in the `Action` FIFO (documented at length in `src/main_loop.rs`).
- `tokio::select!` with `biased;` enforces strict arm priority: Action/input > bounded batch of ExecEvents (≤64 per iteration) > 16ms redraw timer.
- The driver is **not** an in-process tokio task — it's `self.re-exec`'d as a detached child (`gsd-meta-manager drive ...`) with all stdio nulled and its own process group, so it survives the TUI exiting (`src/driver/spawn.rs`).
- There is exactly **one production call site** that constructs a `DrivableProject` (the opt-in capability token) and exactly one that calls `spawn_detached` — both are mechanically checked by `tests/spawn_seam_guard.rs`.
- Read side (`state_reader`) is pure parsing: no mutation, no process spawning, cached and invalidated only by filesystem watch events.
- Screens form a stack (`Vec<Box<dyn Screen>>`) rather than a single "current screen" field, supporting modal dialogs pushed/popped over the base view.

## Layers

**Event/Input Layer:**
- Purpose: turn OS-level input and timers into `Action`s
- Location: `src/event.rs`, `src/main_loop.rs`
- Contains: crossterm event stream reader, tick spawner, the `pump` select loop
- Depends on: `crossterm`, `tokio::sync::mpsc`
- Used by: `src/main.rs`'s `run_tui_loop`

**State/Reducer Layer:**
- Purpose: own all mutable TUI state; apply every `Action`/`ExecEvent` to it
- Location: `src/app.rs`, `src/action.rs`
- Contains: `App`, `AppContext` (via `ui::screens::mod`), `Action` enum, `apply_exec_event`, `start_driver_run`
- Depends on: `state_reader`, `driver`, `journal`, `executor` (for types), `config`, `registry`
- Used by: `main_loop::pump`, `ui::render`

**Presentation Layer:**
- Purpose: render current state to the terminal; translate keys to `ScreenAction`s
- Location: `src/ui/mod.rs`, `src/ui/screens/*.rs`, `src/ui/project_list.rs`, `src/ui/roadmap_widget.rs`
- Contains: `Screen` trait implementors (one per view/dialog), shared `AppContext`
- Depends on: `ratatui`, `App`/`AppContext` state (read-only at render time)
- Used by: `ui::render`, called once per frame from `run_tui_loop`

**Read Side (state_reader):**
- Purpose: parse a registered project's `.planning/` directory tree into `ProjectState`, without ever mutating it
- Location: `src/state_reader/mod.rs` + `backlog.rs`, `config_json.rs`, `disk_status.rs`, `git_ops.rs`, `queue_md.rs`, `roadmap_md.rs`, `state_md.rs`, `workstreams.rs`
- Contains: per-file parsers (`STATE.md`, `ROADMAP.md`, `queue.md`, `config.json`, git log), HANDOFF pause detection, disk-status inference
- Depends on: filesystem only (`std::fs`), `chrono`, `serde_json`
- Used by: `App::load_project_states`, `driver::dry_run`, `driver::run` (reused verbatim so the dashboard and the driver never drift in how they read a project — D-01 in `src/driver/mod.rs`)

**Drive Side (driver / executor / journal):**
- Purpose: run one GSD command against an opted-in project as a supervised, detached, journaled agent process
- Location: `src/driver/*`, `src/executor/*`, `src/journal/*`
- Contains: gate/opt-in checks, spawn seam, lock/concurrency, liveness/reconciliation, kill switch, dry-run, `Executor` trait + Claude transport, append-only journal + redaction + inbox
- Depends on: `state_reader` (project parsing), `config` (opt-in record), `tokio::process`, `process-wrap` (Unix process groups)
- Used by: `src/main.rs`'s `Drive` CLI arm (foreground CLI invocation of the driver process itself) and the TUI's `App::start_driver_run` (which spawns the detached driver and then only ever reads its journal back)

**Config/Registry Layer:**
- Purpose: user-level project registry, persisted TOML config
- Location: `src/config.rs`, `src/registry.rs`, `src/cli.rs`
- Contains: `Config`, `RegisteredProject`, `DriverOptIn`, add/remove/list operations, `clap` CLI definitions
- Depends on: `toml`, `serde`
- Used by: both TUI and Drive modes at startup

## Data Flow

### Primary Request Path (TUI keypress → redraw)

1. Crossterm event captured by `EventBus::spawn_crossterm_reader` → wrapped as `Action::RawKey` (`src/event.rs`)
2. Sent on the unbounded `Action` channel to `main_loop::pump` (`src/main_loop.rs:151`)
3. `pump`'s Arm 1 (biased first) calls `App::update(action)` (`src/app.rs:948`) — screens get `ScreenAction`s via `Screen::handle_key`, which mutate `AppContext` or push follow-up `Action`s
4. `App` sets `needs_redraw` (directly or via `ctx.needs_redraw`, synced each loop iteration in `run_tui_loop`, `src/main.rs:240`)
5. `run_tui_loop` calls `terminal.draw(|frame| ui::render(frame, app))` (`src/main.rs:246`) → `ui::render` dispatches to the top of `App.screen_stack` (`src/ui/mod.rs`)

### Filesystem-Watch Path (external `.planning/` change → TUI update)

1. `notify` fires on a watched path under a project's `.planning/` (`src/watcher.rs`)
2. `watcher.rs::extract_project_root` + `batch_actions` build `Action::FileChanged { project_path, changed_path }`
3. Routed through the same `Action` channel into `App::update`
4. Handler either re-parses the whole project (`state_reader::parse_project_state`) or, if `changed_path` is a driver journal append, does a cheap byte-offset tail read instead (`journal::reader`) — the two-field design of `FileChanged` exists specifically so this distinction is cheap (D-09, `src/action.rs`)

### Drive-Start Path (user requests a driven run from the TUI)

1. Screen sends `Action::DriverStartRequested { alias, command, goal }` (`src/action.rs:195`)
2. `App::start_driver_run` (`src/app.rs:1810`) is the **spawn seam call site**: builds argv via `driver::spawn::drive_argv`, checks the concurrency cap (`driver::lock::admit`), and calls `driver::spawn::spawn_detached` (`src/driver/spawn.rs:115`) — which re-execs `std::env::current_exe()` as `gsd-meta-manager drive <alias> ...` with stdio nulled and a fresh process group
3. The detached child re-enters `src/main.rs`'s `Drive` CLI arm → `driver::drive()` → gate checks → `driver::run::execute_run` → constructs `executor::claude::ClaudeExecutor`, spawns `claude` in `stream-json` duplex mode, writes journal records via `journal::writer` as events arrive
4. The **parent TUI never talks to the child directly** — it only watches the run's `journal.jsonl` file via the same `notify` watcher, tailing new records with `journal::reader` and turning them into `Action::DriverJournalAppended`
5. Steering messages flow the same way in reverse: `Action::DriverInjectRequested` appends a line to `runs/<run-id>/inbox.jsonl` on `spawn_blocking` (never in-process IPC), and the driver tails that file (`src/journal/inbox.rs`, D-03 in `src/action.rs`)

**State Management:**
- All TUI-side mutable state lives in `App` / `AppContext` (`src/app.rs`, `src/ui/screens/mod.rs`); there is no other mutable global.
- Cross-process state (driver run state) is **the filesystem itself** — `run.json`, `journal.jsonl`, `inbox.jsonl`, lock files under each project's `.planning/meta-manager/` — read fresh on each observation rather than cached in the driver's memory being trusted by the TUI.
- `observed_runs` and `last_outcomes` on `AppContext` are periodic (every 20 ticks, i.e. ~5s) snapshots of `driver::reconcile::reconcile_all`, refreshed at startup synchronously to close the gap before the first poll (`src/main.rs:189-199`).

## Key Abstractions

**`Action` (message enum):**
- Purpose: the entire vocabulary of things that can happen to the TUI's state
- Examples: `src/action.rs`
- Pattern: single flat enum, `Clone`, deliberately free of any file/join handles (D-20) so it can travel safely through channels

**`Screen` trait + `AppContext`:**
- Purpose: one implementor per view/dialog; `handle_key` returns `ScreenAction`s, `render` draws given only `&AppContext`
- Examples: `src/ui/screens/mod.rs`, `src/ui/screens/normal.rs`, `src/ui/screens/driver.rs`
- Pattern: stack-based modal navigation (`Vec<Box<dyn Screen>>` on `App`)

**`DrivableProject` (capability token):**
- Purpose: compiler-enforced proof that a project opted into the driver; the *only* thing `Executor::start` and `spawn_detached`'s caller accept
- Examples: `src/executor/mod.rs:110`, constructed only via `DrivableProject::from_registry`
- Pattern: type-as-capability — private fields, one production constructor, checked by `tests/spawn_seam_guard.rs`

**`Executor` trait:**
- Purpose: object-safe interface (`start`/`send`/`interrupt`/`cancel`/`is_running`/`capabilities`) with `claude::ClaudeExecutor` as sole implementor
- Examples: `src/executor/mod.rs:71`
- Pattern: boxed futures instead of `async-trait` (no second implementor exists yet)

**`RedactedLine` (capture-path type gate):**
- Purpose: the journal writer accepts only this type, so redaction cannot be bypassed by construction
- Examples: `src/journal/redact.rs`
- Pattern: smart constructor — no compilable path from a bare `String` to a written journal line

**`ProjectState`:**
- Purpose: the fully parsed snapshot of one project's `.planning/` tree, shared by both the dashboard and the driver's own gate
- Examples: `src/state_reader/mod.rs:14`
- Pattern: plain data struct assembled by `parse_project_state`, boxed inside `Action::ProjectStateLoaded` due to its size (~360 bytes)

## Entry Points

**`src/main.rs::main`:**
- Location: `src/main.rs`
- Triggers: process launch
- Responsibilities: init tracing/color-eyre, parse CLI (`clap`), branch to `Add`/`Remove`/`List`/`Drive`/TUI

**TUI loop (`run_tui_loop`):**
- Location: `src/main.rs:232`
- Triggers: no subcommand given
- Responsibilities: draw, `pump` one iteration, handle pending editor shell-out, check quit

**Drive CLI arm:**
- Location: `src/main.rs:88-120` → `src/driver/mod.rs::drive`
- Triggers: `gsd-meta-manager drive <alias> --command <c> [--run-id ...] [--dry-run] [--goal ...]`
- Responsibilities: gate/opt-in check, dry-run branch, run-id validation, dispatch to `driver::run::execute_run` (Unix) or a typed refusal (non-Unix)

## Architectural Constraints

- **Threading:** Single tokio async runtime (`#[tokio::main]`) for both TUI and drive modes; the driver's blocking git shell-outs (dry-run preview) are explicitly wrapped in `tokio::task::spawn_blocking` because a blocking syscall inside an `async fn` was observed to defeat `tokio::time::timeout` on a current-thread runtime (documented incident referenced in `src/driver/mod.rs`, D-28/WR-10).
- **Global state:** None as a bare static; the closest equivalents are the long-lived `exec_tx` sender clone stashed on `AppContext` (keeps the executor channel open for the process lifetime, `src/main.rs:147-148`) and the file watcher instance stored on `AppContext` (`src/main.rs:166`).
- **Circular imports:** None observed; dependency direction is strictly `ui` → `app`/`action` → `driver`/`executor`/`journal`/`state_reader` → `config`/`error`, with no back-references.
- **Process boundary:** The TUI and any driven run are **separate OS processes** communicating only through the filesystem (config, journal, inbox, lock files, `run.json`) — never a socket or shared memory. This is D-03 and is treated as load-bearing throughout `src/driver/` and `src/journal/`.
- **Single spawn seam:** Exactly one call site constructs a `DrivableProject` in production and exactly one calls `spawn_detached`; `tests/spawn_seam_guard.rs` enforces both mechanically rather than by review discipline.
- **Platform gating:** `driver::kill`, `driver::lock`, `driver::run`, `driver::spawn`, and `executor::claude` are `#[cfg(unix)]`; every non-Unix path returns a typed `DriveError::UnsupportedPlatform` rather than silently degrading (D-05).

## Anti-Patterns

### Routing high-volume data through the `Action` FIFO

**What happens:** A hypothetical future change routes per-token stream output (executor deltas) through the same channel as keypresses.
**Why it's wrong:** The `Action` arm is `biased` first in `main_loop::pump`'s `select!`; anything high-volume on that channel would starve the redraw timer and all subsequent input, exactly the TRANS-03 failure the two-channel split (`Action` vs `ExecEvent`) exists to prevent.
**Do this instead:** All high-volume executor/journal-tail traffic goes through the separate, bounded `ExecEvent` channel (`EXEC_CHANNEL_CAPACITY = 8192`, drained in batches of `EXEC_BATCH = 64`) — see `src/main_loop.rs`.

### Bypassing the spawn seam

**What happens:** Constructing a `DrivableProject` or calling `spawn_detached` from a second call site to "save a hop."
**Why it's wrong:** Defeats the compiler-enforced single-seam guarantee that only opted-in projects can ever have an agent spawned against them; `tests/spawn_seam_guard.rs` will fail the build.
**Do this instead:** Route every drive request through `driver::mod::drive` (CLI) or `App::start_driver_run` (TUI), both of which call the one production constructor/spawn function.

### Blocking syscalls inside `async fn`

**What happens:** A synchronous `std::process::Command` or `flock` call made directly inside an `async fn` body.
**Why it's wrong:** `tests/driver_lock.rs` reproduced a real deadlock where a blocking `flock` inside an `async fn` defeated `tokio::time::timeout` outright on a current-thread runtime, because `Timeout::poll` polls its inner future inline and a parked thread polls nothing at all.
**Do this instead:** Wrap the blocking call in `tokio::task::spawn_blocking` (see `src/driver/mod.rs`'s dry-run branch).

## Error Handling

**Strategy:** Typed error enums per subsystem (`DriveError`, `SpawnError`, `SendError`, `OptInError` in `src/error.rs`) rather than a single catch-all; UI-facing binary code (`src/main.rs`) converts these to `eprintln!` + `std::process::exit(1)` rather than bubbling as an `anyhow` chain, so user-facing refusals are printed directly instead of buried in a generic error wrapper.

**Patterns:**
- Refusals are positioned deliberately early (before any disk write) so a refused operation is provably inert — e.g. `drive()`'s ordering of the opt-in gate, dry-run branch, and run-id validation, each covered by a "writes nothing" test.
- `StopOutcome`/`StopDisposition` model process-signal ambiguity as a value rather than parsing rendered text back into state (`src/action.rs`'s `StopDisposition`).

## Cross-Cutting Concerns

**Logging:** `tracing` + `tracing-subscriber` writing to a daily-rolling file appender under `dirs::data_local_dir()/gsd-meta-manager/` (never stdout/stderr, since ratatui owns the terminal) — set up first thing in `src/main.rs::main`. The journal subsystem explicitly never logs event content (D-28), only counts and already-redacted paths.

**Validation:** Capability gating at multiple layers — CLI opt-in record (`DrivableProject::from_registry`), `system/init` capability check before the first prompt is released (`src/executor/gate.rs`), platform liveness support (`driver::mod::platform_refusal`), and run-id shape validation (`journal::is_plain_run_id`, preventing path traversal via `--run-id`).

**Authentication:** None internal to the tool; the `claude` CLI subprocess handles its own auth. `CLAUDE*` environment variables are scrubbed once, at the real agent spawn (`ClaudeExecutor`), deliberately not duplicated at the outer `spawn_detached` layer to avoid two places drifting.

---

*Architecture analysis: 2026-08-18*
