<!-- generated-by: gsd-doc-writer -->
# Development

This guide covers local development of `gsd-meta-manager`: setting up the toolchain,
the build/check/run cycle, debugging a ratatui TUI (where stdout is owned by the
renderer), code organization, commit conventions, and the milestone release flow.

## Prerequisites

- **Rust 1.89+** (stable) -- `rust-version = "1.89"` in `Cargo.toml`. The floor is not
  hand-chosen: it is the highest `rust-version` declared across the resolved dependency
  graph, currently set by `uuid` 1.28 (locked transitively through
  `wezterm-blob-leases`), with the `ratatui` 0.30.x family, `time`, `darling` and the
  `icu_*` crates one step below at 1.88.0. It can move under an ordinary `cargo update`
  with no manifest edit, which is why the `msrv` job in
  `.github/workflows/release.yml` re-verifies it before any publish. Install via
  [rustup](https://rustup.rs) and confirm with `rustc --version`.
- **A C compiler on Linux** -- the Linux-only TLS egress dependencies pull in `ring`,
  which compiles a C/asm component. macOS and Windows builds need no C compiler.
- **Git** -- for branch and tag operations; the test suite shells out to a real `git`.
- A POSIX-ish terminal that supports the crossterm event stream (Linux, macOS, or
  Windows Terminal). The TUI uses `crossterm`'s alternate screen + raw mode, so
  the terminal must allow those switches.
- **No additional runtime services** for the TUI and registry commands -- the app reads
  from local files only and ships as a single binary. Running the sandboxed `driver` /
  `task` features for real additionally needs the host sandbox tooling (`bwrap`,
  `systemd-run`, `claude`); the `scripts/*-acceptance.sh` harnesses and the
  `launcher_live_*` tests exercise those and are not needed for ordinary changes.
- **Node.js and a container runtime (`docker` or `podman`)** -- only for the release
  rehearsal (`scripts/pre-tag-check.sh`, see Release Process) and the router
  conformance oracle.

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
notify, serde, clap, color-eyre, tracing, tui-textarea, rustix, and on Linux rustls
for the egress server, ...). Subsequent incremental builds are fast.

There is no `.env` file and no project-local `.cargo/config.toml` -- the build
works with default Cargo settings, and `Cargo.toml` declares no `[features]`, so there
are no feature flags to choose. User configuration (`~/.config/gsd-meta-manager/config.json`)
is created on first run; see `docs/CONFIGURATION.md` for details.

## Build Commands

| Command                                                  | Description                                                                 |
|----------------------------------------------------------|-----------------------------------------------------------------------------|
| `cargo check`                                            | Type-check without producing a binary. Fastest signal during development.   |
| `cargo build`                                            | Debug build, output at `target/debug/gsd-meta-manager`.                     |
| `cargo build --release`                                  | Optimized build, output at `target/release/gsd-meta-manager`.               |
| `cargo run`                                              | Debug build + launch the TUI.                                               |
| `cargo run -- list`                                      | Run the `list` subcommand (or `add`, `remove`) without entering TUI mode.   |
| `cargo run -- <alias-or-path>`                           | Open the TUI directly on a registered project (the positional `target`).    |
| `cargo run -- --config /tmp/test.json`                   | Run against an isolated config file (handy for testing registry changes).   |
| `cargo install --path .`                                 | Install the binary into `~/.cargo/bin/` as `gsd-meta-manager`.              |
| `cargo test`                                             | Run the full test suite (unit + integration tests under `tests/`).          |
| `cargo test --no-fail-fast`                              | What the release workflow runs: report every failing test binary, not just the first. |
| `cargo check --all-targets --locked`                     | Compile every target against `Cargo.lock` (the CI `msrv` job's check).      |
| `cargo nextest run`                                      | Same suite with parallel execution -- prefer this if `cargo-nextest` is installed. |
| `cargo clippy -- -D warnings`                            | Lint with all warnings treated as errors (matches the release verify step). |
| `cargo clippy --all-targets -- -D warnings`              | Same, including tests; CI runs this per cross-target (macOS, Windows).      |
| `cargo fmt`                                              | Format the codebase with rustfmt defaults.                                  |
| `cargo watch -x check -x test`                           | Re-run `check` then `test` on every file change.                            |
| `bacon`                                                  | Background `cargo check` loop, surfaces errors without rebuilding.          |
| `./scripts/pre-tag-check.sh [--container] [vX.Y.Z]`      | Local dry run of the release workflow's gates (see Release Process).        |
| `./scripts/install-conformance-oracle.sh`                | Provision GSD's own router as the oracle for `tests/driver_router_conformance.rs`. |

See `docs/TESTING.md` for testing specifics.

### Helper scripts

`scripts/` holds release and acceptance tooling, not build steps:

- `pre-tag-check.sh` / `pre-tag-check.Dockerfile` -- rehearse the release gates before
  tagging; `--container` runs them in an `ubuntu-24.04` lookalike.
- `install-conformance-oracle.sh` -- installs the pinned GSD version used by the router
  conformance test. It never touches an existing `~/.claude/gsd-core`. The test fails
  by design when the oracle is absent, so run this script once on a machine without GSD
  before `cargo test`.
- `launcher-acceptance.sh`, `prompt-acceptance.sh` -- live acceptance harnesses that
  drive the real `driver` with real `bwrap`, `systemd-run` and `claude`. Their `run`
  mode spends real money; `selftest` and `check` do not.

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
cargo run -- next <alias-or-path>      # advisory next-GSD-command hint (read-only)
cargo run -- driver --help             # sandboxed driver runs: new, check, launch, stop, push, auth
cargo run -- task --help               # one-off sandboxed /gsd-quick or /gsd-fast sessions
```

`envelope`, `driverctl`, `taskctl` and `bridge` are hidden internal subcommands (the
sandboxed driver and one-off agent re-enter the binary through them); they are not
meant to be typed by hand.

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
colors are disabled in the file output (`with_ansi(false)`). If the log directory
cannot be created, `init_tracing` installs no subscriber at all and logging is silently
lost rather than crashing the binary.

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
  main.rs              -- sync entry: hidden-subcommand dispatch, tracing init, tokio runtime, CLI dispatch
  main_loop.rs         -- TUI event-loop helpers
  lib.rs               -- library entry, re-exports modules used by main + tests
  cli.rs               -- clap definitions (Cli, Commands)
  app.rs               -- App struct, update() message handler
  action.rs, event.rs  -- Action enum; EventBus (crossterm + tick + mpsc channel)
  watcher.rs           -- notify FileWatcher wrapper
  session_detector.rs  -- detects active `claude` processes
  registry.rs, config.rs -- registered projects; user config.json
  archive.rs, browser.rs, change_tracker.rs, project_creator.rs
  multiplexer/         -- tmux / herdr focus and launch behind one backend trait, plus the GUI terminal fallback
  driver_cli.rs, driverctl_cli.rs, task_cli.rs, taskctl_cli.rs, next_cli.rs
                       -- handlers for the driver / driverctl / task / taskctl / next subcommands
  agents/              -- agent, worktree and workstream discovery for the Agents view
  driver/              -- driver router, direction, steering, liveness, locking, rate limits
  driverctl/           -- driver control channel (store, serve, protocol, stop guard)
  driver_templates/    -- DIRECTION.md / LOG.md templates and lint
  envelope/            -- credential-shape scan and evidence ledger
  executor/, journal/  -- stream-json executor; journal reader and redaction
  launcher/            -- sandboxed launch: bwrap/systemd scope, egress server, auth, push
  oneoff/              -- one-off task runs: launch, runtime, merge/land, resolver, cancel, resume
  state_reader/        -- .planning/ file parsers (STATE.md, ROADMAP.md, queue, backlog, ...)
  ui/                  -- ratatui widgets and screen stack (screens/, roadmap_view.rs, glyphs.rs, ...)
  text.rs, error.rs, tui.rs, bounded_output.rs, test_support.rs -- shared helpers
tests/                 -- 56 integration test files (agents_*, driver*, launcher_*, oneoff_*,
                          registry_test.rs, state_reader_test.rs, ...) plus common/,
                          fixtures/ and launcher_support/
scripts/               -- release rehearsal and live acceptance harnesses
.github/workflows/     -- release.yml (the only workflow; runs on version tags)
docs/                  -- ARCHITECTURE, CONFIGURATION, DEVELOPMENT, TESTING, GETTING-STARTED,
                          DRIVER-SANDBOX, GSD-CORE-SYNC
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

- `quick-<YYMMDD>-<id>` for quick-task work driven by `/gsd:quick` (pre-dispatch
  artifact commits use the bare `<YYMMDD>-<id>`, e.g. `docs(261103-klf): ...`).
- `<phase-number>` for planned phase work.
- `release` for milestone bumps, `sync` for gsd-core baseline syncs.
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

The default branch is `master`. No branch naming convention is documented in the repo
(no `.github/PULL_REQUEST_TEMPLATE.md`; see `CONTRIBUTING.md` for the external-contributor
flow, which says to create a topic branch off `master`). Release work happens on
`release/vX.Y.Z` branches that are merged into `master` (see the history). Use short
descriptive branch names otherwise.

## PR Process

This is a single-maintainer open-source project (see the LICENSE and README).
`.github/` contains a single workflow, `release.yml`, which runs only on a
`vX.Y.Z` tag push (MSRV check, macOS/Windows cross-target check and clippy, then
build, test, clippy and `cargo publish`). There are no PR templates, issue templates, or
per-PR CI jobs, so verification before a PR is on you. Practical guidelines:

- Open an issue first to discuss non-trivial changes (per the README).
- Keep PRs scoped to one quick task or one phase where possible -- this keeps
  the `.planning/` audit trail clean.
- Run the release-grade verification locally before opening a PR:
  ```bash
  cargo build && cargo test && cargo clippy -- -D warnings
  ```
  `cargo test` includes `tests/driver_router_conformance.rs`, which fails by design
  unless GSD's router is available as an oracle; run
  `./scripts/install-conformance-oracle.sh` first on a machine without GSD.
- Code that is Linux-only must be cfg-gated: the release workflow checks macOS and
  Windows targets with `cargo clippy --all-targets -- -D warnings`.
- Update relevant docs in `docs/` when changing user-visible behavior or the
  module layout.

## Release Process

Milestone releases follow the procedure documented in `CLAUDE.md` under
"Release Process". In short, when tagging `vX.Y.Z`:

1. Bump `Cargo.toml` `version` to `X.Y.Z` so `gsd-meta-manager --version` matches
   the tag (historically the crate version drifted -- do not re-introduce that).
2. `cargo update` to refresh `Cargo.lock` within current semver bounds; note any
   deferred semver-breaking deps in the release commit body.
3. Rehearse the release gates: `./scripts/pre-tag-check.sh --container vX.Y.Z` (needs
   `docker` or `podman`; builds an `ubuntu-24.04` lookalike, provisions the conformance
   oracle, and runs tag/version match, MSRV, release build, tests, clippy and
   `cargo package --locked`). A bare `./scripts/pre-tag-check.sh vX.Y.Z` runs the same
   gates on your machine but cannot certify the publish job. It never publishes.
4. Update `.planning/STATE.md` (milestone, status, position, continuity).
5. Commit as `chore(release): vX.Y.Z -- <milestone name>` with an itemized body.
6. Create an annotated tag `vX.Y.Z` whose annotation body is the full changelog (this is
   the canonical changelog -- `git tag -l --format='%(contents)' vX.Y.Z` must
   render what shipped without consulting the commit log). Pushing the tag triggers
   `.github/workflows/release.yml`, which publishes to crates.io.

The full template (including the `git tag -a vX.Y -F - <<'EOF'` form) lives in
the `CLAUDE.md` Release Process section -- treat that file as the source of
truth.

## Next Steps

- `docs/ARCHITECTURE.md` -- component diagram, data flow, key abstractions.
- `docs/CONFIGURATION.md` -- user config schema and environment variables.
- `docs/GETTING-STARTED.md` -- first-run walkthrough.
- `docs/TESTING.md` -- test suite layout and how to run it.
- `docs/DRIVER-SANDBOX.md` -- the sandboxed driver launcher.
- `docs/GSD-CORE-SYNC.md` -- how the tracked gsd-core baseline is synced.
- `CONTRIBUTING.md` -- external-contributor flow.
- `README.md` -- end-user installation and feature overview.
- `CLAUDE.md` -- GSD workflow conventions and the canonical release procedure.
