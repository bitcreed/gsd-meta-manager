---
last_mapped_commit: d7381ab615e5181d33f3b6a5e02cf4d2a42594af
last_mapped_at: 2026-11-03
---
<!-- refreshed: 2026-11-03 -->
# Architecture

**Analysis Date:** 2026-11-03

## System Overview

```text
┌──────────────────────────────────────────────────────────────────────────┐
│                     `src/main.rs` (single binary)                         │
│  Subcommand dispatch: driverctl · taskctl · bridge · next (sync, no log)  │
│                       driver · task (sync, after tracing)                 │
│                       default → tokio runtime → TUI loop                  │
└──────────────┬───────────────────────────────────────┬───────────────────┘
               │ TUI path                              │ CLI / sandbox paths
               ▼                                       ▼
┌──────────────────────────────┐        ┌──────────────────────────────────┐
│ Event sources                │        │ Headless subsystems (std threads)│
│ `src/event.rs` EventBus      │        │ `src/launcher/` driver sandbox   │
│ `src/watcher.rs` notify      │        │ `src/oneoff/` one-off tasks      │
│ `src/session_detector.rs`    │        │ `src/driverctl/` run store/socket│
│ `src/agents/` (via tick)     │        │ `src/driver*_cli.rs`, `task_cli`│
└──────────────┬───────────────┘        └──────────────────────────────────┘
               │ `Action` (mpsc::UnboundedSender/Receiver)
               ▼
┌──────────────────────────────────────────────────────────────────────────┐
│ `src/app.rs` App::update  — sole mutator of AppContext                    │
│   owns screen_stack: Vec<Box<dyn Screen>>, AppContext (`ui/screens/mod`) │
└──────────────┬──────────────────────────────────────┬────────────────────┘
               │ render                               │ spawn_blocking
               ▼                                      ▼
┌─────────────────────────────┐        ┌──────────────────────────────────┐
│ `src/ui/` ratatui widgets   │        │ `src/state_reader/` .planning/   │
│ `src/ui/screens/*` Screens  │        │ parsers, `src/archive.rs`,       │
│                             │        │ `src/browser.rs`, git via shell │
└─────────────────────────────┘        └──────────────────────────────────┘
               │ persistent writes (narrow)
               ▼
┌──────────────────────────────────────────────────────────────────────────┐
│ `src/config.rs`, `src/registry.rs`, `.planning/meta-manager/QUEUE.md`,    │
│ run directories under `.planning/meta-manager/drivers/` and the data dir  │
└──────────────────────────────────────────────────────────────────────────┘
```

The binary is `gsd-meta-manager` (`Cargo.toml`, single `src/main.rs`; no `src/bin/`). The
crate root `src/lib.rs` re-exports every non-binary module as `pub mod` so `tests/` drives the
same code the binary uses.

## Pattern Overview

**Overall:** Message-driven (Elm/Redux-style) single-owner TUI, plus a set of synchronous,
process-isolated subsystems (launcher, one-off runner, run store) that share the same crate
but never touch TUI state.

**Key Characteristics:**
- One `Action` enum (`src/action.rs`) is the only way event sources change TUI state.
- All TUI state is mutated on one task by `App::update` (`src/app.rs`). No `RwLock` appears in `src/`; the only shared mutable state is a small set of `Arc<Mutex<…>>` handles (for example `App::oneoff_landings` in `src/app.rs` and `src/driverctl/serve.rs`), which worker threads touch and which never hold `AppContext`.
- Blocking work (file parsing, git, `/proc` scans, agent-worktree scans) runs on
  `tokio::task::spawn_blocking` and returns results as `Action` values.
- Agent-executing code (`src/launcher/`, `src/oneoff/`, `src/driverctl/`) is deliberately
  synchronous (std threads, `flock`, `/proc`), and is never called from an `async fn`.
  `tests/async_blocking_guard.rs` and `tests/spawn_seam_guard.rs` enforce these shapes.
- The model is never trusted with host state: the host writes `run.json`, `stream.jsonl`,
  and `launch.log`; the agent writes only its clone and `report/report.json`
  (`src/oneoff/mod.rs` trust split).
- Reading `.planning/` requires no model call: `state_reader` is pure file parsing.

## Layers

**Entry / orchestration (binary-level):**
- Purpose: Parse argv, route to subsystem, own the tokio runtime and TUI loop.
- Location: `src/main.rs`, `src/cli.rs`, `src/tui.rs`, `src/main_loop.rs`
- Contains: `Cli` / `Commands` clap types, `run_tui_loop`, tracing setup.
- Depends on: every subsystem below.
- Used by: the shell (end users, sandbox-internal invocations).

**Event transport:**
- Purpose: Produce `Action`s from terminal input, timers, filesystem, and background tasks.
- Location: `src/event.rs` (EventBus: crossterm reader, 250ms tick), `src/watcher.rs`
  (notify-debouncer-full, 200ms), `src/session_detector.rs` (`/proc` scan for `claude`).
- Contains: sender clones and spawned tasks only; no state.
- Depends on: `src/action.rs`.
- Used by: `src/main.rs` / `App::update`.

**Application core (state + reducer):**
- Purpose: Single mutation point for all TUI state; debouncing; dispatch of background loads.
- Location: `src/app.rs` (`App`, `App::update`, `DetailSubView`, filter logic).
- Contains: `App { should_quit, needs_redraw, ctx, screen_stack, active_sessions, … }`.
- Depends on: `ui::screens::AppContext`, `state_reader`, `registry`, `change_tracker`, `agents`.
- Used by: `run_tui_loop` in `src/main.rs`.

**Screens and rendering:**
- Purpose: Turn key input into `ScreenAction`s and draw the current view.
- Location: `src/ui/mod.rs` (`ui::render`), `src/ui/screens/*.rs`, `src/ui/roadmap_view.rs`,
  `src/ui/roadmap_graph.rs`, `src/ui/glyphs.rs`, `src/ui/mouse.rs`.
- Contains: `Screen` trait implementations; widgets built on ratatui.
- Depends on: `AppContext` (read/write), `state_reader` types, `driver`/`oneoff` read-side
  helpers, `archive`, `browser`.
- Used by: `App` (screen stack) only.

**Domain readers (read-only `.planning/` parsing):**
- Purpose: Parse GSD artifacts into typed snapshots without running GSD.
- Location: `src/state_reader/` (`mod.rs` defines `ProjectState` and `parse_project_state`;
  `state_md.rs`, `roadmap_md.rs`, `queue_md.rs`, `backlog.rs`, `disk_status.rs`,
  `config_json.rs`, `config_secrets.rs`, `config_write.rs`, `driver_frontmatter.rs`,
  `plan_waves.rs`, `phase_num.rs`, `workstreams.rs`, `gsd_install.rs`, `git_ops.rs`).
- Depends on: `serde`, `serde_yml`, `regex`, git CLI via `bounded_output`.
- Used by: `App` (via `spawn_blocking`), `next_cli`, `driver::router`.

**Project registry and config:**
- Purpose: Persist which projects are registered and user preferences.
- Location: `src/config.rs` (`Config`, `RegisteredProject`, `Preferences`), `src/registry.rs`
  (add/remove validation, auto-register from sessions), `src/project_creator.rs`.
- Depends on: `dirs`, `serde_json`.

**Sandboxed driver launcher (experimental):**
- Purpose: Run one headless `claude -p` driver inside `bwrap` under `systemd-run --user --scope`.
- Location: `src/launcher/` (`run.rs` orchestration, `sandbox.rs` pure argv builders, `egress/`
  proxy, `push.rs`, `auth.rs`, `config_dir.rs`, `bridge.rs`, `gate.rs`, `stop.rs`, `process.rs`
  the sole spawn entry, `assets/MASTER-PROMPT.md`).
- Depends on: `driverctl::store`, `driver::*`, `envelope`, `journal`, `executor::stream_json`.
- Used by: `driver_cli` (`driver launch`, etc.), `ui/screens/driver_*`.

**Driver run store and socket:**
- Purpose: Host-side source of truth for a driver run (`flock`-guarded files and a Unix socket).
- Location: `src/driverctl/` (`store.rs`, `serve.rs`, `protocol.rs`, `scan.rs`, `log.rs`,
  `counters.rs`, `stop_guard.rs`, `mod.rs`), CLI in `src/driverctl_cli.rs`.
- Depends on: `driver::{direction, lock, liveness, kill}`.
- Used by: the launcher and the in-sandbox `driverctl` shim.

**Driver pure modules:**
- Purpose: Frontmatter/`DIRECTION.md` parsing, steering, router, rate-limit classification,
  liveness, kill, lock. Nothing here runs an agent.
- Location: `src/driver/` (`direction.rs`, `steer.rs`, `router.rs`, `rate_limit.rs`,
  `liveness.rs`, `kill.rs`, `lock.rs`), templates in `src/driver_templates/`.

**One-off task runner (experimental):**
- Purpose: One headless agent session running `/gsd-quick` or `/gsd-fast` on a registered
  project, in a private clone, reporting through `taskctl report`.
- Location: `src/oneoff/` (`launch.rs`, `session.rs`, `stream.rs`, `runtime.rs` the single
  agent-specific seam, `prompt.rs`, `report.rs`, `store.rs`, `inbox.rs`, `merge.rs`,
  `land.rs`, `resolve.rs`, `resume.rs`, `cancel.rs`, `recycle.rs`, `todos.rs`); CLI in `src/task_cli.rs`
  and `src/taskctl_cli.rs`.
- Depends on: `launcher` (sandbox argv, token, egress, process seam, stop, push); does NOT call
  `launcher::run::launch`.
- Used by: `task` / `taskctl` subcommands and the One-off screens.

**Agent worktree observation (read-only):**
- Purpose: Discover running agent worktrees and derive wave/phase status for the dashboard.
- Location: `src/agents/` (`worktrees.rs`, `worktree_plans.rs`, `worktree_phases.rs`,
  `waves.rs`, `fixers.rs`, `unmerged.rs`, `processes.rs`, `workstream_scope.rs`,
  `adapters/claude.rs`).
- Used by: `App` via tick-driven `spawn_blocking(agents::scan_projects_guarded)`.

**Supporting modules:**
- `src/executor/` (`stream_json.rs`): Claude stream-json parsing.
- `src/journal/` (`redact.rs`, `reader.rs`): event journal with redaction.
- `src/envelope/` (`policy.rs`, `ledger.rs`, `scan.rs`): PR-cap and envelope policy.
- `src/text.rs`, `src/bounded_output.rs`: text helpers; wall-clock-bounded command output.
- `src/change_tracker.rs`: in-memory status-transition log (not persisted).
- `src/archive.rs`, `src/browser.rs`: milestone archive and `.planning/` docs browsing.
- `src/terminal_switch.rs`: tmux pane switching.
- `src/launch_target.rs`, `src/experimental.rs`, `src/error.rs` (empty placeholder).

## Data Flow

### Primary TUI Request Path (file change → screen)

1. A file under a registered `.planning/` changes; `FileWatcher` (`src/watcher.rs`) debounces
   for 200ms and emits `Action::FileChanged { project_path }` once per project root.
2. The receiver loop in `run_tui_loop` (`src/main.rs`) calls `app.update(action)`.
3. `App::update` applies a per-project 500ms guard via `AppContext.last_refresh`, then
   `spawn_blocking(state_reader::parse_project_state)`.
4. The blocking task sends `Action::ProjectStateLoaded`; `App::update` stores it in
   `AppContext.project_states` and calls `change_tracker::detect_changes`.
5. If `needs_redraw` is set, `terminal.draw(|f| ui::render(f, app))` delegates to the top
   of `app.screen_stack`.

### Key Input Path

1. `EventBus::spawn_crossterm_reader` maps `KeyEvent` → `Action::RawKey`.
2. `App::update` forwards to the top `Screen::handle_key(code, modifiers, &mut AppContext)`.
3. The returned `ScreenAction` (`None`, `Push`, `Replace`, `Pop`, `Quit`,
   `SetStatusMessage`, `SuspendAndEdit`, `DispatchAction`) is applied by `App`, which owns the
   stack. Screens never hold a reference to the stack.

### Background Agent Observation

1. `Action::Tick` every 20 ticks (about 5s) → `spawn_blocking(agents::scan_projects_guarded)`.
2. Result → `Action::AgentsScanned` → `agents::waves::derive` → `AppContext.agent_views`.
3. Dashboard and Sessions › Agents read `agent_views` without computing anything.

### Headless Driver Run (`driver launch`)

1. `main` dispatches `Commands::Driver` before the tokio runtime exists (`driver_cli::run`).
2. The launcher takes `<run-dir>/launcher.lock`, seeds the per-run `CLAUDE_CONFIG_DIR`, and
   starts the `RunStore` socket server on a thread (`src/driverctl/serve.rs`).
3. The sandbox argv is built by pure functions in `src/launcher/sandbox.rs` and spawned only
   through `src/launcher/process.rs`.
4. Inside the sandbox the driver reaches state only through the `driverctl` shim and the
   socket; `driverctl` is dispatched in `main` before tracing so it creates no log file.
5. The lock is released last, after the socket is removed and the terminal status recorded.

### One-off Task Run (`task run`)

1. `task_cli::run` → `oneoff::launch` creates a run directory with a private clone `work/`,
   an `inbox/` (0700, host-only), and a `run.json`.
2. A single agent session runs with the prompt from `oneoff::prompt`; the agent's stdin
   receives stream-json lines (`oneoff::session`, `oneoff::inbox`).
3. The agent calls `taskctl report`, writing `report/report.json`. The host re-reads it capped,
   refuses symlinks, and re-validates it (`oneoff::report::read_validated`). Commit range and
   SUMMARY path are derived from git on the host.
4. Merge and land steps (`oneoff::merge`, `oneoff::land`) are host-side and bounded. A per-run
   `land.lock` admits one landing at a time; conflicts are resolved by a sandboxed agent session
   (`oneoff::resolve`) in a private clone, and the checkout changes only at the final fast-forward.
5. Stop, kill and pause: the CLI (`task stop`) or TUI writes `<run-dir>/stop.request` (or
   `land/stop.request`); only the run's launcher daemon acts on it, driving the pure `Cancel`
   state machine (`src/oneoff/cancel.rs`). Operator messages go through `inbox/` and are logged to
   `injections.jsonl` before acting (`src/oneoff/inbox.rs`, supervised by `Supervisor` in
   `src/oneoff/session.rs`).
6. Recycle: after a merge or push, `land::merge`/`land::push` call `recycle::slim_run`
   (`src/oneoff/recycle.rs`), which deletes the clone and keeps an allowlisted record (`run.json`
   with `pruned`, `summary.md`, stream/launch log tails). `recycle::recycle_project` runs at TUI
   startup (`App::start_oneoff_recycle` in `src/app.rs`, only with the experimental flag, answered
   by `Action::OneoffRecycled`) and at the start of `task run` (`src/task_cli.rs`, parent only).
   It marks dead runs `AgentOutcome::Crashed`, slims landed runs, and removes aged or orphaned runs
   (`RETENTION_DAYS`, `RETENTION_MAX_FINISHED`, `ORPHAN_MIN_AGE`).

## Key Abstractions

**`Action` (`src/action.rs`):**
- Purpose: Every event the TUI reacts to. Variants include `Tick`, `RawKey`, `Mouse`, `Resize`,
  `FileChanged`, `ProjectStateLoaded`, `GitLogLoaded`, `GitCommitDetailLoaded`,
  `CreateProjectResult`, `ArchiveMilestonesDiscovered`, `ArchiveLoaded`, `SessionsDetected`,
  `AgentsScanned`.
- Pattern: Tagged enum; producers hold cloned `UnboundedSender<Action>`.

**`EventBus` (`src/event.rs`):**
- Purpose: Owns the channel and spawns the crossterm reader and tick task.

**`App` (`src/app.rs`):**
- Purpose: Top-level state and the only `update` entry point.
- Pattern: Reducer; `needs_redraw` gates rendering.

**`AppContext` (`src/ui/screens/mod.rs`):**
- Purpose: Shared mutable TUI state passed as `&mut` to screens: `config`, `project_states`,
  selection, filter text, `view_cache`, `change_tracker`, `event_tx`, `watcher`,
  `active_sessions`, `agent_views`, `last_refresh`, `needs_redraw`.

**`Screen` trait (`src/ui/screens/mod.rs`):**
- Methods: `handle_key`, `render`, `name`, plus a mouse hook with a default no-op.
- Supertrait `RenderAdjudicated` is a compile-time guard: every screen must declare how it
  handles render-escape concerns (`src/ui/screens/render_escape_guard.rs`).
- Implementations: `normal.rs` (project list, default), `detail.rs` (per-project sub-views:
  RoadmapViz, Pipeline, Backlog, GitHistory, Queue, Sessions/Agents, Defaults, Browse/Archive,
  Driver), `help.rs`, `add_project.rs`, `create_project.rs`, `delete_confirm.rs`,
  `enqueue.rs`, `queue_delete_confirm.rs`, `driver*.rs`, `oneoff.rs`, `oneoff_viewer.rs`,
  `preferences.rs`, `confirm_popup.rs`.

**`ScreenAction` (`src/ui/screens/mod.rs`):**
- Values: `None`, `Push(Box<dyn Screen>)`, `Replace` (pop-and-push in one step; used where a
  handed-off screen must not be re-submittable), `Pop`, `Quit`, `SetStatusMessage`,
  `SuspendAndEdit`, `DispatchAction`.

**`ProjectState` (`src/state_reader/mod.rs`):**
- Purpose: Parsed snapshot of one `.planning/` tree. `phase_progress()` is the single
  phase-count definition; `paused` and `stale_handoff` are decided in the parser so no
  consumer re-judges a HANDOFF.

**`RunStore` (`src/driverctl/store.rs`):**
- Purpose: Driver run persistence behind `flock`; the launcher embeds it and the socket serves it.

**`OneOff runtime seam` (`src/oneoff/runtime.rs`):**
- Purpose: The only file in `oneoff/` allowed to name a concrete agent (argv, env, config
  seeding, credential, egress upstream, stream parsing, capability flags, models).
  `tests/oneoff_runtime_seam_guard.rs` enforces this.

**`ChangeTracker` (`src/change_tracker.rs`):**
- Purpose: In-memory status-transition and phase-completion log since launch (not persisted).

## Entry Points

**`gsd-meta-manager` (no subcommand) — TUI:**
- Location: `src/main.rs` `async_main` → `run_tui_loop`.
- Triggers: user runs the binary, optionally with a `<alias|path>` launch target
  (`src/launch_target.rs`).
- Responsibilities: tracing, color-eyre, tokio runtime, EventBus, FileWatcher, TUI loop.

**`gsd-meta-manager driverctl …`:**
- Location: `src/driverctl_cli.rs`, dispatched before tracing in `src/main.rs`.
- Triggers: invoked by the in-sandbox driver shim.

**`gsd-meta-manager taskctl …`:**
- Location: `src/taskctl_cli.rs`, dispatched before tracing.
- Triggers: the one-off agent's `report` command inside the sandbox.

**`gsd-meta-manager bridge …`:**
- Location: `src/launcher/bridge.rs`, dispatched before tracing; Linux and macOS only.
- Triggers: in-sandbox bridge process that forwards to the driver command.

**`gsd-meta-manager next …`:**
- Location: `src/next_cli.rs`, dispatched before tracing; read-only router hint, writes nothing.
- Triggers: `next --json` contract (`src/driver/router.rs`).

**`gsd-meta-manager driver …` and `gsd-meta-manager task …`:**
- Location: `src/driver_cli.rs`, `src/task_cli.rs`; synchronous, after tracing, before tokio.
- Triggers: user command line (experimental, gated by `src/experimental.rs`).

## Architectural Constraints

- **Threading:** One multi-thread tokio runtime is built explicitly in `main`. The TUI state
  lives on the loop task only. Launcher, one-off, and driverctl code is std-thread-only.
- **Global state:** `AppContext` and `App` are owned values, not globals. Process-wide state
  is limited to the tracing subscriber and the `GSDMM_EXPERIMENTAL_FEATURES` flag
  (`src/experimental.rs`).
- **Synchronous subsystems:** `driver`, `task`, `driverctl`, and `launcher` must not be called
  from `async fn`; `main` dispatches them before the runtime exists.
- **Spawn allowlist:** Every process the launcher or one-off runner starts goes through
  `src/launcher/process.rs` (`tests/spawn_seam_guard.rs`).
- **Trust boundary:** Agent-written files (`report.json`, `DIRECTION.md`) are re-validated on
  the host and never trusted for commit ranges or paths.
- **Tracing side effects:** `driverctl`, `taskctl`, `bridge`, and `next` must create no log
  file and write nothing to stdout, because they run inside the sandbox. Keep this ordering.
- **Single-binary crate:** No `src/bin/`; the `[package] exclude` list in `Cargo.toml` keeps
  `.planning/`, `CLAUDE.md`, and `assets/` out of the published crate.

## Anti-Patterns

### Calling a launcher, driver, or one-off function from an `async fn`

**What happens:** Adding an async wrapper around `launcher::run`, `driverctl::store`, or
`oneoff::launch`.
**Why it's wrong:** Those paths take `flock`, do blocking file I/O, and spawn processes with
std threads; `tests/async_blocking_guard.rs` fails on this shape.
**Do this instead:** Keep the call synchronous and dispatch it from the synchronous branch of
`src/main.rs`, or wrap it in `tokio::task::spawn_blocking` from the TUI side only, as
`App::update` does for `state_reader` and `agents`.

### Mutating `AppContext` from a worker thread

**What happens:** Passing `Arc<Mutex<AppContext>>` or sending a closure into a worker.
**Why it's wrong:** Breaks the single-owner reducer; the codebase relies on the channel for
synchronization and has no locks around TUI state.
**Do this instead:** Have the worker return a result as an `Action` variant (see
`ProjectStateLoaded`, `AgentsScanned`) and handle it in `App::update` in `src/app.rs`.

### Naming a concrete agent outside the runtime seam

**What happens:** Hard-coding `claude` argv, env, or stream parsing in `src/oneoff/` files
other than `runtime.rs`.
**Why it's wrong:** `tests/oneoff_runtime_seam_guard.rs` fails, and the seam exists so a second
runtime can be added in one place.
**Do this instead:** Add the fact to `src/oneoff/runtime.rs` and read it through the seam.

### Spawning processes directly

**What happens:** Calling `std::process::Command::spawn` in launcher or one-off code.
**Why it's wrong:** Bypasses the spawn allowlist enforced by `tests/spawn_seam_guard.rs`.
**Do this instead:** Route through `src/launcher/process.rs`.

## Error Handling

**Strategy:** `anyhow::Result` at the binary boundary (`main`, `async_main`) with
`color-eyre` installed for reports. Subsystem CLIs return an exit code from `run(...)`
(`driverctl_cli::run`, `driver_cli::run`, `task_cli::run`), which `main` passes to
`std::process::exit`.

**Patterns:**
- Parsers return `Option` or typed `Result` and degrade to a "not detected" state rather than
  panicking (`src/state_reader/*`).
- Background failures surface as `Action` variants with an error payload, then
  `ScreenAction::SetStatusMessage` for the user.
- `src/error.rs` is an empty placeholder; no crate-wide error enum is in use.

## Cross-Cutting Concerns

**Logging:** `tracing` with `tracing-appender` file output, initialized in `init_tracing()`
(`src/main.rs`) after the sandbox-internal subcommands are dispatched. Journal events
(`src/journal/`) are separately redacted (`src/journal/redact.rs`).

**Validation:** Config and state are validated at parse time in `state_reader` and
`registry.rs`. Agent-produced reports are validated again on the host (`oneoff::report`,
`driver::direction`).

**Authentication:** Not in the TUI. Launcher resolves an OAuth token (`src/launcher/auth.rs`)
and keeps it outside the sandbox; egress is mediated by `src/launcher/egress/`.

**Render discipline:** Opt-in redraw via `needs_redraw`; no fixed-rate render loop. The 250ms
tick only expires status messages and polls sessions and agents
(`docs/ARCHITECTURE.md` documents the same design).

---

*Architecture analysis: 2026-11-03*
