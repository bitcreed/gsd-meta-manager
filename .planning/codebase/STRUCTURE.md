# Codebase Structure

**Analysis Date:** 2026-08-18

## Directory Layout

```
gsd-meta-manager/
├── src/
│   ├── main.rs              # Binary entry point: CLI parse, mode select, TUI loop shim
│   ├── lib.rs                # Library crate root (re-exports modules for tests/binary)
│   ├── action.rs             # Action enum — the entire internal message vocabulary
│   ├── app.rs                # App state/reducer: App::update, App::apply_exec_event, start_driver_run
│   ├── main_loop.rs           # Testable core of the TUI event loop (pump/select!)
│   ├── event.rs               # EventBus: crossterm reader + tick spawner
│   ├── watcher.rs              # notify-based filesystem watcher → Action::FileChanged
│   ├── cli.rs                  # clap CLI definitions (Add/Remove/List/Drive)
│   ├── config.rs               # Config, RegisteredProject, DriverOptIn (TOML)
│   ├── registry.rs             # add/remove/list project registry operations
│   ├── error.rs                 # Shared typed error enums (DriveError, SpawnError, SendError, OptInError)
│   ├── change_tracker.rs        # Detects/tracks project changes for status highlighting
│   ├── session_detector.rs      # Detects active `claude` CLI sessions for auto-registration
│   ├── project_creator.rs       # Scaffolds a new GSD project from the TUI
│   ├── terminal_switch.rs       # Terminal-switching integration (opening a project's terminal)
│   ├── browser.rs               # File/dir browsing helper for screens
│   ├── archive.rs               # Reads archived milestone data
│   ├── state_reader/            # READ SIDE: parses one project's .planning/ into ProjectState
│   │   ├── mod.rs                #  ProjectState struct, HANDOFF pause detection
│   │   ├── state_md.rs            #  STATE.md frontmatter parsing
│   │   ├── roadmap_md.rs           #  ROADMAP.md phase parsing
│   │   ├── queue_md.rs              #  queue.md parsing, suggest_next_commands
│   │   ├── backlog.rs                #  backlog.md parsing
│   │   ├── config_json.rs             #  .planning/config.json parsing
│   │   ├── disk_status.rs              #  per-phase disk-state inference
│   │   ├── git_ops.rs                   #  git log/diff-stat shell-outs
│   │   └── workstreams.rs                #  GSD 1.8.0 parallel workstream state
│   ├── driver/                   # DRIVE SIDE: orchestrates one detached agent run
│   │   ├── mod.rs                 #  drive() entry, gate, dry-run branch, dispatch
│   │   ├── spawn.rs                #  THE spawn seam: spawn_detached, drive_argv, admit()
│   │   ├── run.rs                   #  Unix run body: lock → executor → journal → unlock
│   │   ├── lock.rs                   #  per-project run lock + concurrency cap
│   │   ├── liveness.rs                 #  /proc-based liveness probe
│   │   ├── reconcile.rs                 #  startup + periodic scan of on-disk run state
│   │   ├── kill.rs                       #  two-layer stop (signal driver's process group)
│   │   └── dry_run.rs                      #  preview report builder/renderer
│   ├── executor/                  # Agent process transport
│   │   ├── mod.rs                  #  Executor trait, DrivableProject capability token
│   │   ├── claude.rs                 #  ClaudeExecutor: spawns `claude`, stream-json duplex
│   │   ├── gate.rs                     #  system/init capability validation
│   │   ├── outcome.rs                    #  RunOutcome derivation from exit + stream
│   │   └── stream_json.rs                  #  typed wire model for the stream-json protocol
│   ├── journal/                     # Durable, redacted run record
│   │   ├── mod.rs                    #  JournalRun, RunRecord, run-id validation
│   │   ├── writer.rs                   #  append-only NDJSON writer
│   │   ├── reader.rs                     #  byte-offset tail + tolerant record parse
│   │   ├── redact.rs                       #  RedactedLine capture-path type gate
│   │   └── inbox.rs                          #  durable steering-message inbox (inbox.jsonl)
│   └── ui/                           # Presentation layer
│       ├── mod.rs                     #  render() dispatch to top of screen_stack
│       ├── project_list.rs              #  project list widget
│       ├── roadmap_widget.rs              #  roadmap visualization widget
│       └── screens/                        #  one file per modal screen/tab
│           ├── mod.rs                       #  Screen trait, AppContext, ScreenAction
│           ├── normal.rs                      #  base dashboard/list screen
│           ├── detail.rs                        #  per-project detail view (largest screen file)
│           ├── driver.rs                          #  Driver tab: run list/detail/live output
│           ├── driver_confirm.rs                    #  confirm-before-start dialog
│           ├── driver_start.rs                        #  start-run flow (command/goal picker)
│           ├── driver_inject.rs                         #  steering-message injection dialog
│           ├── add_project.rs, create_project.rs          #  registration/creation dialogs
│           ├── delete_confirm.rs, queue_delete_confirm.rs   #  destructive-action confirms
│           ├── enqueue.rs                                    #  queue-a-command dialog
│           └── help.rs                                         #  help overlay
├── tests/                             # Integration tests (one file per subsystem/property)
│   ├── driver_dry_run.rs, driver_kill.rs, driver_kill_startup.rs,
│   │   driver_lock.rs, driver_optin.rs, driver_reattach.rs, driver_tracer.rs
│   ├── executor_lifecycle.rs, executor_transport.rs
│   ├── journal_crash.rs, journal_gitignore.rs, journal_run_paths.rs
│   ├── registry_test.rs, state_reader_test.rs
│   └── spawn_seam_guard.rs           # Mechanically enforces the single spawn seam invariant
├── docs/                               # Project documentation (not code)
├── .planning/                          # GSD workflow state (phases, plans, STATE.md, codebase docs)
├── Cargo.toml / Cargo.lock
├── CLAUDE.md, CONTRIBUTING.md, README.md, LICENSE
```

## Directory Purposes

**`src/state_reader/`:**
- Purpose: pure, read-only parsing of a registered project's `.planning/` tree into `ProjectState`
- Contains: one file per source-of-truth artifact (`STATE.md`, `ROADMAP.md`, `queue.md`, `backlog.md`, `config.json`, git log, workstreams) plus disk-state inference
- Key files: `src/state_reader/mod.rs` (the `ProjectState` struct and top-level `parse_project_state` entry), `src/state_reader/git_ops.rs` (the only submodule that shells out)

**`src/driver/`:**
- Purpose: orchestrate one detached, journaled agent run against an opted-in project
- Contains: the gate/dispatch entry point, the single spawn seam, lock/concurrency, liveness/reconciliation, the two-layer kill switch, the dry-run preview
- Key files: `src/driver/mod.rs` (entry + module-level design doc), `src/driver/spawn.rs` (the seam), `src/driver/run.rs` (the actual run body)

**`src/executor/`:**
- Purpose: drive an agent process (`claude`) over its `stream-json` duplex protocol
- Contains: the `Executor` trait and its sole implementor, the capability gate, outcome derivation, the wire model
- Key files: `src/executor/mod.rs` (trait + `DrivableProject` capability token), `src/executor/claude.rs` (concrete transport)

**`src/journal/`:**
- Purpose: durable, redacted, append-only record of one driver run, plus the inbox that carries steering messages the other direction
- Contains: writer, tolerant reader (byte-offset tail), capture-path redaction, inbox
- Key files: `src/journal/mod.rs` (module-level design doc, `RunRecord`, run-id validation), `src/journal/redact.rs` (the type gate)

**`src/ui/`:**
- Purpose: everything ratatui-facing; render dispatch plus one `Screen` implementor per view/dialog
- Contains: `Screen` trait + `AppContext` (`screens/mod.rs`), the dashboard (`normal.rs`), the per-project detail view (`detail.rs`, the largest file in the repo), the Driver tab (`driver.rs`) and its supporting dialogs
- Key files: `src/ui/screens/mod.rs` (trait + shared context), `src/ui/screens/detail.rs`, `src/ui/screens/driver.rs`

**`tests/`:**
- Purpose: integration tests, one file per subsystem or cross-cutting property
- Contains: driver lifecycle/lock/kill/dry-run/reattach tests, executor transport/lifecycle tests, journal crash/gitignore/run-path tests, and `spawn_seam_guard.rs`, which mechanically enforces the single-spawn-seam and no-`deny_unknown_fields` invariants across the whole `src/` tree
- Key files: `tests/spawn_seam_guard.rs`

## Key File Locations

**Entry Points:**
- `src/main.rs`: process entry, CLI parse, TUI-vs-Drive mode branch, `run_tui_loop`

**Configuration:**
- `src/config.rs`: `Config`/`RegisteredProject`/`DriverOptIn`, loaded from `~/.config/gsd-meta-manager/config.toml` by default
- `src/cli.rs`: `clap` argument definitions

**Core Logic:**
- `src/app.rs`: the reducer (`App::update`) and the TUI-side spawn seam call site (`start_driver_run`)
- `src/driver/mod.rs`: the drive-mode gate/dispatch entry point
- `src/state_reader/mod.rs`: the read-side entry point (`parse_project_state`)

**Testing:**
- `tests/*.rs`: integration tests, run with `cargo test` or `cargo nextest run`
- `#[cfg(test)] mod tests` blocks embedded in most `src/` files for unit tests co-located with the code they cover (e.g. `src/action.rs`, `src/driver/mod.rs`, `src/main_loop.rs`)

## Naming Conventions

**Files:**
- One file per cohesive submodule/subsystem concern, named after its noun (`spawn.rs`, `lock.rs`, `kill.rs`, `reconcile.rs`) rather than a generic `utils.rs` bucket.
- Screens are named after the view or dialog they implement (`driver_confirm.rs`, `queue_delete_confirm.rs`, `add_project.rs`) and live under `src/ui/screens/`.
- Test files are named after the subsystem plus the property under test (`driver_kill_startup.rs`, `journal_gitignore.rs`, `spawn_seam_guard.rs`), not `test_1.rs`-style.

**Directories:**
- Top-level `src/` subdirectories are named after architectural layers (`driver`, `executor`, `journal`, `state_reader`, `ui`), each with its own `mod.rs` carrying a module-level design doc comment explaining the submodule split and the load-bearing decisions behind it.
- `#[cfg(unix)]` submodules sit inside otherwise cross-platform directories (e.g. `driver/kill.rs`, `driver/lock.rs`, `driver/run.rs`, `driver/spawn.rs`, `executor/claude.rs`) rather than being split into a separate `unix/` tree.

## Where to Add New Code

**New read-side parser (another `.planning/` artifact):**
- Add a new file under `src/state_reader/`, following the existing per-file pattern (e.g. `queue_md.rs`); wire its output into `ProjectState` in `src/state_reader/mod.rs`.
- Tests: co-located `#[cfg(test)]` module in the new file, or extend `tests/state_reader_test.rs`.

**New TUI screen/dialog:**
- Add a file under `src/ui/screens/`, implement the `Screen` trait (`handle_key`, `render`) as defined in `src/ui/screens/mod.rs`, and push it onto `App.screen_stack` from the triggering key handler.
- Any new cross-screen state goes on `AppContext` (`src/ui/screens/mod.rs`), not on `App` directly, unless it's truly loop-level (redraw flags, watcher handle).

**New driver subsystem behavior (e.g. a new refusal, a new reconciliation fact):**
- Extend `src/driver/mod.rs`'s `drive()` gate ordering carefully — refusals are positioned before any disk write by design; add a test asserting the refused path writes nothing, following the pattern in `src/driver/mod.rs`'s `#[cfg(test)]` module.
- Never add a second call site for `DrivableProject::from_registry` or `spawn_detached` — `tests/spawn_seam_guard.rs` will fail the build.

**New Action/message variant:**
- Add the variant to `src/action.rs`, document its sizing (`Action` is intentionally kept small; boxes are used only for genuinely large payloads like `ProjectState`), and add the handling arm in `App::update` (`src/app.rs`).
- If it's high-volume executor/stream traffic, it belongs on the separate bounded `ExecEvent` channel (`src/main_loop.rs`), never on the `Action` FIFO.

**Utilities:**
- Shared helpers live beside their single consumer's module rather than in a generic `utils.rs`; if genuinely cross-cutting, add to `src/error.rs` (errors) or a new top-level `src/*.rs` module named after its concern (as with `src/change_tracker.rs`, `src/session_detector.rs`).

## Special Directories

**`.planning/`:**
- Purpose: this project's own GSD workflow state (phases, plans, STATE.md, codebase docs including this one)
- Generated: partially — phase/plan artifacts are written by GSD commands; `codebase/*.md` docs are written by the codebase mapper
- Committed: yes

**`target/`:**
- Purpose: Cargo build output
- Generated: yes
- Committed: no (gitignored)

**`docs/`:**
- Purpose: project-facing documentation (not GSD planning state)
- Generated: no
- Committed: yes

---

*Structure analysis: 2026-08-18*
