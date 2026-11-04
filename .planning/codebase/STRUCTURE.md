---
last_mapped_commit: d7381ab615e5181d33f3b6a5e02cf4d2a42594af
last_mapped_at: 2026-11-03
---
<!-- refreshed: 2026-11-03 -->
# Codebase Structure

**Analysis Date:** 2026-11-03

## Directory Layout

```
[project-root]/                         # gsd-meta-manager (Rust crate, single binary)
├── Cargo.toml / Cargo.lock             # Crate manifest (edition 2021, rust-version 1.89)
├── README.md, CLAUDE.md, CONTRIBUTING.md, LICENSE
├── src/                                # All production code (28 top-level .rs files + 11 subtree directories; `ui/screens/` holds 19 files)
│   ├── main.rs                         # Binary entry: subcommand dispatch, tokio runtime, TUI loop
│   ├── lib.rs                          # Re-exports every non-binary module as `pub mod`
│   ├── action.rs, app.rs, event.rs     # Message enum, reducer, event bus
│   ├── cli.rs, tui.rs, main_loop.rs    # clap definitions, terminal init, loop helpers
│   ├── config.rs, registry.rs          # Config/preferences I/O, project registry
│   ├── watcher.rs, session_detector.rs # Filesystem watch, /proc session scan
│   ├── terminal_switch.rs              # tmux pane switching
│   ├── change_tracker.rs, archive.rs, browser.rs, project_creator.rs
│   ├── driver_cli.rs, driverctl_cli.rs, task_cli.rs, taskctl_cli.rs, next_cli.rs  # subcommand handlers
│   ├── launch_target.rs, experimental.rs, bounded_output.rs, text.rs, error.rs
│   ├── agents/                         # Read-only agent-worktree observation
│   ├── driver/                         # Pure driver modules (direction, steer, router, …)
│   ├── driver_templates/               # `driver new` templates and DIRECTION lint
│   ├── driverctl/                      # Run store, Unix socket server, protocol
│   ├── envelope/                       # PR-cap / envelope policy and ledger
│   ├── executor/                       # Claude stream-json parsing
│   ├── journal/                        # Event journal + redaction
│   ├── launcher/                       # Sandboxed driver launcher (bwrap, systemd-run, egress)
│   ├── oneoff/                         # One-off task runner (experimental)
│   ├── state_reader/                   # `.planning/` parsers → ProjectState
│   └── ui/                             # ratatui rendering and Screen implementations
├── tests/                              # Integration tests (about 56 top-level `*.rs` files) + fixtures
│   ├── *.rs                            # One file per area (agents_*, driverctl_*, launcher_*, oneoff_*, state_reader_*, …)
│   ├── common/mod.rs                   # Shared test helpers
│   ├── launcher_support/mod.rs         # Launcher test helpers
│   ├── fixtures/                       # Golden and replay data (driverctl, prompt, roadmaps, transcripts)
│   └── *_guard.rs                      # Architecture guards (async, spawn seam, oneoff seam)
├── docs/                               # Human-facing architecture, config, testing, sandbox docs
├── scripts/                            # Acceptance, conformance and pre-tag check scripts
├── assets/screenshots/                 # README images (not included in the published crate)
└── .github/workflows/release.yml       # Release and MSRV job
```

## Directory Purposes

**`src/` (top level):**
- Purpose: Binary and library root. Each subsystem is either a single file or a directory.
- Contains: `main.rs` (entry), `lib.rs` (module exports), TUI core, subcommand handlers
  (`*_cli.rs`), and standalone modules.
- Key files: `src/main.rs`, `src/app.rs`, `src/action.rs`, `src/event.rs`, `src/config.rs`,
  `src/registry.rs`, `src/watcher.rs`.

**`src/ui/`:**
- Purpose: Rendering and input handling for the TUI.
- Contains: `mod.rs` (`render` entry), `roadmap_view.rs`, `roadmap_graph.rs`, `glyphs.rs`,
  `mouse.rs`, and `screens/`.
- Key files: `src/ui/mod.rs`, `src/ui/screens/mod.rs` (`Screen`, `ScreenAction`, `AppContext`),
  `src/ui/screens/detail.rs` (largest screen, about 35k bytes), `src/ui/screens/normal.rs`.

**`src/ui/screens/`:**
- Purpose: One file per screen or modal.
- Contains: normal, detail, help, add/create/delete project, enqueue, queue delete confirm,
  driver (`driver.rs`, `driver_confirm.rs`, `driver_launch.rs`, `driver_run.rs`,
  `driver_steer.rs`), oneoff (`oneoff.rs`, `oneoff_viewer.rs`), preferences, confirm popup,
  and `render_escape_guard.rs`.

**`src/state_reader/`:**
- Purpose: Read-only parsers for `.planning/` artifacts and git.
- Contains: One module per artifact (STATE.md, ROADMAP.md, QUEUE.md, backlog, config.json,
  secrets presence, driver frontmatter, plan waves, workstreams, install detection, git ops).
- Key files: `src/state_reader/mod.rs` (`ProjectState`, `parse_project_state`),
  `src/state_reader/roadmap_md.rs`, `src/state_reader/disk_status.rs`.

**`src/launcher/`:**
- Purpose: Experimental sandboxed driver launcher.
- Contains: `run.rs` (orchestration), `sandbox.rs` (pure argv builders), `process.rs` (sole spawn
  entry), `egress/` (`http1.rs`, `upstream.rs`, `policy.rs`, `log.rs`), `push.rs` and
  `push/planted.rs`, `auth.rs`, `config_dir.rs`, `home.rs`, `host.rs`, `bridge.rs`, `gate.rs`,
  `outcome.rs`, `prompt.rs`, `stop.rs`, `assets/MASTER-PROMPT.md`.

**`src/oneoff/`:**
- Purpose: Experimental one-off task runner.
- Contains: `runtime.rs` (only agent-specific seam), `launch.rs`, `session.rs`, `stream.rs`,
  `prompt.rs`, `report.rs`, `store.rs`, `inbox.rs`, `merge.rs`, `land.rs`, `resolve.rs`,
  `resume.rs`, `cancel.rs` (stop/kill/pause requests and state machine), `recycle.rs` (slim landed
  runs, recover dead ones, prune orphans), `todos.rs`, `mod.rs` (module docs and the trust split).

**`src/driver/` and `src/driverctl/`:**
- Purpose: `driver/` holds pure readers and classifiers; `driverctl/` holds the stateful run
  store and its socket server.
- Key files: `src/driver/direction.rs`, `src/driver/router.rs`, `src/driverctl/store.rs`,
  `src/driverctl/serve.rs`, `src/driverctl/protocol.rs`.

**`src/driver_templates/`:**
- Purpose: Templates for `driver new`.
- Contains: `mod.rs`, `lint.rs`, `assets/DIRECTION.md`, `assets/LOG.md`.

**`src/agents/`:**
- Purpose: Read-only scan of agent worktrees and wave derivation.
- Key files: `src/agents/mod.rs` (`scan_projects_guarded`), `src/agents/waves.rs`,
  `src/agents/adapters/claude.rs`, `src/agents/worktrees.rs`.

**`src/envelope/`, `src/journal/`, `src/executor/`:**
- Purpose: Respectively, PR-cap policy and ledger; redacted event journal and reader;
  Claude stream-json parser.

**`tests/`:**
- Purpose: Integration tests. Most build `assert_fs` trees mimicking `.planning/`.
- Contains: `*_guard.rs` architecture guards (`async_blocking_guard.rs`,
  `spawn_seam_guard.rs`, `oneoff_runtime_seam_guard.rs`), `*_e2e.rs` end-to-end tests,
  `launcher_live_*.rs` (live-environment tests), `oneoff_e2e.rs` / `oneoff_taskctl.rs` (one-off runs),
  and `fixtures/` (including `fixtures/oneoff/` recorded stream-json runs, see its `README.md`).
- Key files: `tests/common/mod.rs`, `tests/launcher_support/mod.rs`.

**`tests/fixtures/`:**
- Purpose: Static inputs.
- Contains: `driverctl/` (golden run files, replay logs, spike run artifacts),
  `prompt/MASTER-PROMPT.no-gap-rules.md`, `roadmaps/` (sample ROADMAP/STATE pairs),
  `roadmap-shapes/`, `transcripts/` (stream-json NDJSON scenarios with a README).

**`docs/`:**
- Purpose: Long-form documentation: `ARCHITECTURE.md`, `CONFIGURATION.md`, `DEVELOPMENT.md`,
  `DRIVER-SANDBOX.md`, `GETTING-STARTED.md`, `GSD-CORE-SYNC.md`, `TESTING.md`.

**`scripts/`:**
- Purpose: Acceptance and release checks: `launcher-acceptance.sh`, `prompt-acceptance.sh`,
  `install-conformance-oracle.sh`, `pre-tag-check.sh`, `pre-tag-check.Dockerfile`.

**`.planning/` (per-project, written by the app, not by this repo's source):**
- The repo itself carries `.planning/` for its own GSD work. At runtime the app reads
  `.planning/` in each registered project and writes `.planning/config.json` and
  `.planning/meta-manager/QUEUE.md` only on user action.

## Key File Locations

**Entry Points:**
- `src/main.rs`: binary entry and subcommand dispatch.
- `src/lib.rs`: library root for tests.

**Configuration:**
- `Cargo.toml`: crate metadata, dependency set, `exclude` list for publishing.
- `src/config.rs`: `Config`, `RegisteredProject`, `Preferences`.
- `src/state_reader/config_json.rs`, `src/state_reader/config_write.rs`: GSD project config.
- `src/experimental.rs`: `GSDMM_EXPERIMENTAL_FEATURES` gate.

**Core Logic:**
- `src/app.rs`: reducer.
- `src/action.rs`: message enum.
- `src/state_reader/mod.rs`: project state aggregation.
- `src/driver/router.rs`: next-command router (`next --json` contract).
- `src/launcher/run.rs`, `src/launcher/sandbox.rs`: driver launch.
- `src/oneoff/launch.rs`, `src/oneoff/runtime.rs`: one-off launch and agent seam.

**Testing:**
- `tests/*_guard.rs`: architecture invariants.
- `tests/fixtures/`: golden data.

**Documentation:**
- `docs/ARCHITECTURE.md`: system overview.
- `CLAUDE.md`: project instructions for agents.

## Naming Conventions

**Files:**
- `snake_case.rs` for all Rust modules (`state_reader`, `driver_cli.rs`, `queue_delete_confirm.rs`).
- `*_cli.rs` for subcommand handlers (`driver_cli.rs`, `taskctl_cli.rs`).
- `*_guard.rs` for architecture-invariant tests under `tests/`.
- `*_e2e.rs` for end-to-end tests; `launcher_live_*.rs` for environment-dependent tests.
- `ALL-CAPS.md` for assets consumed by `include_str!` (`MASTER-PROMPT.md`, `DIRECTION.md`, `LOG.md`).
- Module doc comments name the phase or quick-task that introduced them (for example, "quick 261103-f5y").

**Directories:**
- `snake_case` for module directories (`state_reader/`, `driver_templates/`).
- Subdirectories that are themselves modules have a `mod.rs` (for example `src/ui/screens/mod.rs`);
  a few use the Rust 2018 sibling form with a same-named `.rs` file (`src/launcher/push.rs`
  alongside `src/launcher/push/`).

**Functions and types:**
- Screen types end with `Screen` (for example `DetailScreen`); the modal forms are named by
  purpose, for example `DeleteConfirmScreen`.
- Action variants are past-tense results (`ProjectStateLoaded`, `GitLogLoaded`) or events
  (`FileChanged`, `Tick`).

## Where to Add New Code

**New TUI screen or modal:**
- Implementation: `src/ui/screens/<name>.rs`, implementing `Screen` and `RenderAdjudicated`.
- Register it in `src/ui/screens/mod.rs`. Open it from `normal.rs` or `detail.rs` via
  `ScreenAction::Push` (or `Replace` when the handed-off screen must not be re-submittable).
- Tests: `tests/` integration test that drives the same code the binary uses through `lib.rs`.

**New detail sub-view:**
- Implementation: add a variant to `DetailSubView` in `src/app.rs` and render it from
  `src/ui/screens/detail.rs`.

**New `.planning/` artifact reader:**
- Implementation: `src/state_reader/<artifact>.rs`, then wire it into `parse_project_state` in
  `src/state_reader/mod.rs` and add the field to `ProjectState`.
- Tests: `tests/state_reader_test.rs` with a fixture under `tests/fixtures/roadmaps/` or a new
  folder.

**New background (async) data load:**
- Add an `Action` variant in `src/action.rs`; spawn a `spawn_blocking` worker from `App::update`
  in `src/app.rs` that sends the result back on `event_tx`.

**New CLI subcommand:**
- Add the clap variant in `src/cli.rs` (`Commands`), handle it in `src/main.rs` before
  `init_tracing` if it must write nothing or run in a sandbox, or after it if it is synchronous
  but may log. Put its handler in a `*_cli.rs` file at `src/`.

**New one-off or launcher capability:**
- Agent-specific facts go in `src/oneoff/runtime.rs` (or `src/launcher/` equivalents), never in
  other `oneoff/` files. Spawns go through `src/launcher/process.rs`.
- Tests: add or extend `tests/oneoff_e2e.rs` or `tests/launcher_*.rs`, and run the guard tests.

**New pure driver logic:**
- Implementation: `src/driver/<module>.rs`; keep it free of process spawns and TUI types so it
  compiles on every platform.

**Utilities:**
- Shared text helpers: `src/text.rs`.
- Bounded external command output: `src/bounded_output.rs`.

**New integration test:**
- `tests/<area>_<topic>.rs`. Use `tests/common/mod.rs` for shared helpers and `tests/fixtures/`
  for static inputs.

## Special Directories

**`tests/fixtures/`:**
- Purpose: Golden files, replay logs, and sample roadmaps and transcripts.
- Generated: No (hand-maintained, with some captured from real runs).
- Committed: Yes.

**`assets/`:**
- Purpose: README screenshots.
- Generated: No.
- Committed: Yes. Excluded from the published crate by `Cargo.toml` (`exclude = ["/assets/"]`).
  The `src/**/assets/` directories are NOT excluded, because they are `include_str!` targets.

**`.planning/`:**
- Purpose: GSD planning state for this repo's own development.
- Generated: Yes (by GSD workflow commands).
- Committed: Yes for the repo's own history; excluded from the published crate.

**Runtime data directories (outside the repo):**
- Driver runs: `.planning/meta-manager/drivers/` in each registered project.
- One-off runs: `gsd-meta-manager/oneoff/` under the user data directory, one run directory per
  task holding a private `work/` clone, `inbox/`, `report/`, `run.json`, `stream.jsonl`,
  `launch.log`, and `injections.jsonl`.
- Not committed; created at run time.

---

*Structure analysis: 2026-11-03*
