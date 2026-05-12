<!-- generated-by: gsd-doc-writer -->
# Development

This guide covers local development of `gsd-meta-manager`: setting up the toolchain,
the build/check/run cycle, debugging a ratatui TUI (where stdout is owned by the
renderer), code organization, commit conventions, and the milestone release flow.

## Prerequisites

- **Rust 1.85+** (stable) -- required by the `Cargo.toml` `edition = "2021"` setup and
  the upstream `notify` MSRV. Install via [rustup](https://rustup.rs) and confirm with
  `rustc --version`.
- **Git** -- for branch and tag operations.
- A POSIX-ish terminal that supports the crossterm event stream (Linux, macOS, or
  Windows Terminal). The TUI uses `crossterm`'s alternate screen + raw mode, so
  the terminal must allow those switches.
- **No additional runtime services** -- the app reads from local files only and ships
  as a single static binary.

Optional but recommended (matches the stack listed in `CLAUDE.md`):

- [`cargo-watch`](https://crates.io/crates/cargo-watch) -- live rebuild on save.
- [`bacon`](https://crates.io/crates/bacon) -- background `cargo check` runner.
- [`cargo-nextest`](https://crates.io/crates/cargo-nextest) -- faster parallel test
  runner with better output than `cargo test`.

Install the optional tools once:

```bash
cargo install cargo-watch bacon cargo-nextest
```

## Local Setup

```bash
git clone <repo-url> gsd-meta-manager
cd gsd-meta-manager
cargo build
```

The first build pulls and compiles the dependency tree (ratatui, crossterm, tokio,
notify, serde, clap, color-eyre, tracing, tui-textarea, ...). Subsequent
incremental builds are fast.

There is no `.env` file and no project-local `.cargo/config.toml` -- the build
works with default Cargo settings. User configuration (`~/.config/gsd-meta-manager/config.json`)
is created on first run; see `docs/CONFIGURATION.md` for details.

## Build Commands

| Command                                                  | Description                                                                 |
|----------------------------------------------------------|-----------------------------------------------------------------------------|
| `cargo check`                                            | Type-check without producing a binary. Fastest signal during development.   |
| `cargo build`                                            | Debug build, output at `target/debug/gsd-meta-manager`.                     |
| `cargo build --release`                                  | Optimized build, output at `target/release/gsd-meta-manager`.               |
| `cargo run`                                              | Debug build + launch the TUI.                                               |
| `cargo run -- list`                                      | Run the `list` subcommand (or `add`, `remove`) without entering TUI mode.   |
| `cargo run -- --config /tmp/test.json`                   | Run against an isolated config file (handy for testing registry changes).   |
| `cargo install --path .`                                 | Install the binary into `~/.cargo/bin/` as `gsd-meta-manager`.              |
| `cargo test`                                             | Run the full test suite (unit + integration tests under `tests/`).          |
| `cargo nextest run`                                      | Same suite with parallel execution -- prefer this if `cargo-nextest` is installed. |
| `cargo clippy -- -D warnings`                            | Lint with all warnings treated as errors (matches the release verify step). |
| `cargo fmt`                                              | Format the codebase with rustfmt defaults.                                  |
| `cargo watch -x check -x test`                           | Re-run `check` then `test` on every file change.                            |
| `bacon`                                                  | Background `cargo check` loop, surfaces errors without rebuilding.          |

See `docs/TESTING.md` for testing specifics (once generated).

## Running the App

Launch the TUI directly from a debug build:

```bash
cargo run
```

Subcommands skip the TUI:

```bash
cargo run -- add /path/to/some/gsd-project
cargo run -- list
cargo run -- remove some-alias
```

Pass an alternate config file (useful when iterating without disturbing your real
registry):

```bash
cargo run -- --config /tmp/dev-config.json
```

## Logging and Debugging

A ratatui TUI owns stdout and the alternate screen -- anything written to
stdout/stderr will scribble over the UI. All diagnostics therefore go to a file.

### Log destination

`main.rs` installs a `tracing-appender` daily-rolling file appender at
`$XDG_DATA_HOME/gsd-meta-manager/gsd-meta-manager.log` (resolved via the `dirs`
crate). On Linux this is typically:

```text
~/.local/share/gsd-meta-manager/gsd-meta-manager.log
```

If `dirs::data_local_dir()` returns `None`, logs fall back to `/tmp`. ANSI
colors are disabled in the file output (`with_ansi(false)`).

### Filtering log output

`main.rs` installs `EnvFilter::from_default_env()` on the subscriber, so the
file appender honors `RUST_LOG`. With no `RUST_LOG` set, the default filter is
empty and only `ERROR`-level events are recorded. Examples:

```bash
RUST_LOG=debug cargo run                       # everything at debug+
RUST_LOG=gsd_meta_manager=debug cargo run      # this crate only
RUST_LOG=gsd_meta_manager=trace,notify=warn cargo run
```

Any filter directive accepted by `tracing_subscriber::EnvFilter` works.

Tail the log in a separate terminal while the TUI runs:

```bash
tail -F ~/.local/share/gsd-meta-manager/gsd-meta-manager.log
```

### Panics and error reports

`color-eyre` is installed as the panic/report handler before the TUI starts. The
terminal restore step lives in `tui::restore()` (called from `main`), but if a
panic happens before that runs, the terminal may be left in raw mode. Recover
with `reset` or `stty sane`.

### Common debugging workflow

1. Run the TUI in one terminal: `RUST_LOG=debug cargo run`.
2. Tail the log file in a second terminal.
3. Add `tracing::debug!(...)` calls in the code path you are investigating.
4. Reproduce the interaction in the TUI; observe traces in the tail.

`println!` and `eprintln!` should not be used inside the TUI render/update path.
They are fine inside CLI subcommands (`add`, `remove`, `list`) because those
exit before the TUI ever starts.

## Code Organization

High-level module layout (see `docs/ARCHITECTURE.md` for the full component diagram
and data flow description):

```text
src/
  main.rs              -- tokio runtime, CLI dispatch, TUI event loop
  lib.rs               -- library entry, re-exports modules used by main + tests
  cli.rs               -- clap definitions (Cli, Commands)
  app.rs               -- App struct, update() message handler
  event.rs             -- EventBus (crossterm + tick + mpsc Action channel)
  action.rs            -- Action enum
  watcher.rs           -- notify FileWatcher wrapper
  session_detector.rs  -- detects active `claude` processes via /proc
  registry.rs          -- add/remove/list registered projects
  config.rs            -- load/save user config.json
  archive.rs           -- milestone archive browser logic
  browser.rs           -- .planning/ docs traversal
  change_tracker.rs    -- per-project change detection between polls
  project_creator.rs   -- "new project" flow
  terminal_switch.rs   -- tmux session switching glue
  error.rs             -- shared error type
  tui.rs               -- ratatui init/restore helpers
  state_reader/        -- .planning/ file parsers (STATE.md, ROADMAP.md, queue, ...)
  ui/                  -- ratatui widgets and screen stack
    screens/           -- one Screen impl per detail tab
    project_list.rs
    roadmap_widget.rs
tests/
  registry_test.rs       -- registry CRUD integration tests
  state_reader_test.rs   -- .planning/ parser integration tests
```

Refer to `docs/ARCHITECTURE.md` before adding a new module so it lands in the
right layer of the message-driven loop.

## Commit Conventions

The repository follows Conventional Commits. The observed type distribution in
`git log` is:

- `feat(<scope>):` -- new functionality. Scope is typically the GSD task id
  (e.g. `feat(quick-260512-fe6): wire Docs browser tab into detail view`).
- `fix(<scope>):` -- bug fix.
- `docs(<scope>):` -- documentation, including `.planning/` artifacts and
  user-facing README/`docs/` updates.
- `chore(<scope>):` -- release commits, dependency bumps, non-functional cleanup
  (e.g. `chore(release): v1.4 -- Live Sessions & Document Browsing`).
- `test(<scope>):` -- test-only changes.

Scope conventions seen in history:

- `quick-<YYMMDD>-<id>` for quick-task work driven by `/gsd:quick`.
- `<phase-number>` for planned phase work.
- `release` for milestone bumps.
- A bare module name (e.g. `defaults`, `state`, `CLAUDE.md`) when the change is
  scoped to a specific module or file rather than a tracked task.

Subject line is lowercase after the colon (matching the existing log). Body is
optional; use it for the "why" when the diff alone is not self-explanatory.

### GSD workflow gate

Per the `CLAUDE.md` "GSD Workflow Enforcement" section, file-changing operations
should start through a GSD command (`/gsd:quick`, `/gsd:debug`, or
`/gsd:execute-phase`) so the planning artifacts in `.planning/` stay in sync with
the commit history. The commit scope (`quick-...`, phase number, ...) ties each
commit back to the planning artifact that authorized it.

## Branch Conventions

The default branch is `master`. No additional branch naming convention is
documented in the repo (no `.github/PULL_REQUEST_TEMPLATE.md`; see
`CONTRIBUTING.md` for the external-contributor flow). Use short descriptive
branch names when working outside `master`.

## PR Process

This is a single-maintainer open-source project (see the LICENSE and README).
There is no `.github/` directory with PR templates, issue templates, or CI
workflows checked in. Practical guidelines:

- Open an issue first to discuss non-trivial changes (per the README).
- Keep PRs scoped to one quick task or one phase where possible -- this keeps
  the `.planning/` audit trail clean.
- Run the release-grade verification locally before opening a PR:
  ```bash
  cargo build && cargo test && cargo clippy -- -D warnings
  ```
- Update relevant docs in `docs/` when changing user-visible behavior or the
  module layout.

## Release Process

Milestone releases follow the procedure documented in `CLAUDE.md` under
"Release Process". In short, when tagging `vX.Y`:

1. Bump `Cargo.toml` `version` to `X.Y.0` so `gsd-meta-manager --version` matches
   the tag (historically the crate version drifted -- do not re-introduce that).
2. `cargo update` to refresh `Cargo.lock` within current semver bounds; note any
   deferred semver-breaking deps in the release commit body.
3. Verify clean: `cargo build && cargo test && cargo clippy -- -D warnings`.
4. Update `.planning/STATE.md` (milestone, status, position, continuity).
5. Commit as `chore(release): vX.Y -- <milestone name>` with an itemized body.
6. Create an annotated tag whose annotation body is the full changelog (this is
   the canonical changelog -- `git tag -l --format='%(contents)' vX.Y` must
   render what shipped without consulting the commit log).

The full template (including the `git tag -a vX.Y -F - <<'EOF'` form) lives in
the `CLAUDE.md` Release Process section -- treat that file as the source of
truth.

## Next Steps

- `docs/ARCHITECTURE.md` -- component diagram, data flow, key abstractions.
- `docs/CONFIGURATION.md` -- user config schema and environment variables.
- `README.md` -- end-user installation and feature overview.
- `CLAUDE.md` -- GSD workflow conventions and the canonical release procedure.
