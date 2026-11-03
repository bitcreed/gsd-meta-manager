---
last_mapped_commit: 5146cf11780714b72ed3b2e6224e849a93e10a08
last_mapped_at: 2026-11-03
---
# Technology Stack

**Analysis Date:** 2026-11-03

## Languages

**Primary:**
- Rust (edition 2021) - the entire application: TUI, CLI, state reader, driver launcher, one-off runner. Sources under `src/` (e.g. `src/main.rs`, `src/lib.rs`, `src/app.rs`, `src/launcher/`, `src/oneoff/`, `src/state_reader/`).

**Secondary:**
- Node.js 24 - build-time only, used by the CI test suite's conformance oracle (`.github/workflows/release.yml`, `scripts/install-conformance-oracle.sh`). Not a runtime dependency of the crate.
- POSIX shell - helper scripts in `scripts/` (`scripts/pre-tag-check.sh`, `scripts/launcher-acceptance.sh`, `scripts/prompt-acceptance.sh`, `scripts/install-conformance-oracle.sh`).
- Markdown and YAML - run/driver templates embedded into the binary (`src/launcher/assets/MASTER-PROMPT.md`, `src/driver_templates/assets/DIRECTION.md`, `src/driver_templates/assets/LOG.md`) and parsed frontmatter.

## Runtime

**Environment:**
- Native compiled binary; no managed runtime. `main` dispatches the synchronous `driver` subcommands before a tokio runtime is created (`src/launcher/mod.rs` module docs).
- Minimum supported Rust: 1.89 (`rust-version = "1.89"` in `Cargo.toml`). The `msrv` job in `.github/workflows/release.yml` pins `dtolnay/rust-toolchain@1.89.0` and fails if the manifest floor drifts from it.
- Async: tokio multi-thread runtime for the TUI event loop; the driver launcher is std-threads only (`tests/async_blocking_guard.rs` enforces this).

**Package Manager:**
- Cargo.
- Lockfile: present - `Cargo.lock` (committed; CI builds use `--locked`).

## Frameworks

**Core:**
- ratatui 0.30 (features `crossterm`, `unstable-rendered-line-info`) - terminal UI rendering. Screens in `src/ui/screens/`.
- crossterm 0.29 (feature `event-stream`) - terminal backend and async input events.
- tokio 1 (feature `full`) - async runtime for the event loop.
- clap 4 (feature `derive`) - CLI parsing (`src/cli.rs`, `src/driver_cli.rs`, `src/driverctl_cli.rs`, `src/taskctl_cli.rs`, `src/next_cli.rs`).

**Testing:**
- Built-in `cargo test` - unit tests in-module and integration tests in `tests/` (about 56 tracked files, including `tests/fixtures/driverctl/direction`).
- assert_fs 1 - dev-dependency for filesystem fixtures.
- unicode-properties 0.1.4 (feature `general-category`) and unicode-width 0.2 (feature `cjk`) - dev-only independent oracles for the text and glyph width tests.

**Build/Dev:**
- cargo, with `cargo clippy -- -D warnings` and `cargo fmt --check` used as gates (`.github/workflows/release.yml`, `scripts/pre-tag-check.sh`).
- `scripts/pre-tag-check.Dockerfile` - Ubuntu 24.04 image mirroring the release runner.
- GitHub Actions - `.github/workflows/release.yml` (tag-triggered: `msrv`, `cross-target-check`, `publish` jobs).

## Key Dependencies

**Critical:**
- `serde` 1 (derive), `serde_json` 1, `serde_yml` 0.0.13 - state and config serialization; YAML frontmatter in GSD planning files (`src/state_reader/`, `src/config.rs`).
- `notify` 8 and `notify-debouncer-full` 0.7 - filesystem watcher that drives live UI refresh of `.planning/` changes (`src/watcher.rs`).
- `chrono` 0.4 (feature `serde`) and `regex` 1 - timestamps and pattern parsing across readers.
- `tui-textarea` 0.7 (feature `ratatui`) - text input widget in the TUI.
- `dirs` 7 - platform config and data directories (`dirs::config_dir()` in `src/config.rs`, `dirs::data_local_dir()` in `src/envelope/mod.rs` and `src/oneoff/store.rs`).
- `rustix` 1.1 (features `process`, `fs`, `system`) - process-group signalling, `flock`, and `uname()` without `libc`/`nix` (`src/driver/kill.rs`, `src/driver/lock.rs`, `src/driverctl/store.rs`).
- `sha2` 0.11.0 - SHA-256 for the driver opt-in re-confirmation digest (chosen over a hand-rolled hash).
- `icu_properties` 2.2.0 - Unicode invisible-character classification in `src/text.rs`.

**Linux-only (target table in `Cargo.toml`):**
- `rustls` 0.23 (features `ring`, `std`, `tls12`, `logging`, default features off), `tokio-rustls` 0.26 (`ring`, `tls12`, `logging`), `webpki-roots` 1 - TLS client for the launcher's egress proxy to `api.anthropic.com` (`src/launcher/egress/upstream.rs`).
- `httparse` 1 (cross-platform) - HTTP/1 head parsing in the egress server (`src/launcher/egress/http1.rs`).

**Infrastructure / supporting:**
- `clap`, `anyhow` 1, `color-eyre` 0.6, `tracing` 0.1, `tracing-subscriber` 0.3 (feature `env-filter`), `tracing-appender` 0.2 - error reporting and file logging (`src/main.rs`, `src/error.rs`).
- `futures` 0.3, `tempfile` 3 (runtime use as well as tests).

**Native build requirement:**
- `ring` compiles C/asm via `cc`, so a C compiler is required to build on Linux. macOS and Windows builds do not pull the TLS crates (see `Cargo.toml` comments).

## Configuration

**Application config:**
- User config file: `~/.config/gsd-meta-manager/config.json` on Linux (platform config dir via `dirs::config_dir()`); overridable with `--config <PATH>` (`src/config.rs`, `Config::default_path`). Schema versioned via `CONFIG_SCHEMA_VERSION`.
- Project registry lives in the same config file (`projects` map).

**Data directories (outside driven repositories):**
- `dirs::data_local_dir()/gsd-meta-manager/` - log, envelope ledger (`src/envelope/mod.rs`), one-off run store (`src/oneoff/store.rs`, layout `<data_local_dir>/gsd-meta-manager/oneoff/<hash8>/`).

**Environment variables read by the application** (names only, from `std::env::var` call sites):
- `HOME`, `PATH`, `TMPDIR`, `XDG_RUNTIME_DIR`, `TMUX`, `TERMINAL`, `VISUAL`, `EDITOR`, `NO_COLOR`
- `CLAUDE_CONFIG_DIR`, `CLAUDE_CODE_SESSION_ID`, `CODEX_HOME` - locate Claude/Codex config and sessions (`src/agents/adapters/claude.rs`, `src/session_detector.rs`, `src/state_reader/queue_md.rs`)
- `DRIVERCTL_SOCK` - unix socket path for the driver control channel (`src/driverctl/`)

**Secrets handling:**
- The driver OAuth token is produced by `claude setup-token` and stored in a 0600 file named `driver-oauth-token` under the app config directory, with a fallback to the host Claude credentials file (`src/launcher/auth.rs`). Only the access token and expiry are read. No secret values are recorded in this document.
- No `.env` files are tracked in the repository. `.gitignore` excludes `/target/` and `/.claude/worktrees/`.

**Build and release config:**
- `Cargo.toml` (crate metadata, `exclude` list keeping `.planning/`, `CLAUDE.md`, `assets/` out of the published package).
- `Cargo.lock` (locked builds).
- `.github/workflows/release.yml` - release pipeline; `CARGO_REGISTRY_TOKEN` is supplied from repository secrets at publish time only.

## External Tools Invoked at Runtime

These are spawned as child processes (`Command::new` call sites):
- `git` - repository state, worktrees, merges (`src/state_reader/git_ops.rs`, `src/agents/worktrees.rs`, `src/oneoff/`)
- `claude` - headless driver runs (`claude -p --output-format stream-json`) and `claude setup-token` (`src/launcher/sandbox.rs`, `src/driver_cli.rs`, `src/cli.rs`)
- `bwrap` and `systemd-run --user --scope` - sandbox for driver runs (Linux only; `src/launcher/sandbox.rs`, `src/launcher/host.rs`)
- `tmux` - session focus and switching (`src/terminal_switch.rs`, `src/agents/processes.rs`)
- `gitleaks` - optional secret scanning of external output (`src/envelope/scan.rs`, `GITLEAKS_BIN`)
- `xdg-terminal-exec`, `which`, `pgrep`, `sh`, `/bin/sh`, `sleep` - terminal launch, binary probing, process detection, and shell helpers

## Platform Requirements

**Development:**
- Rust 1.89+ with a C compiler (for `ring` on Linux).
- Optional for full test coverage: `gitleaks`, `bwrap`, `systemd-run`, Node.js 24 (CI conformance oracle).

**Production / Distribution:**
- Published to crates.io as `gsd-meta-manager` (`cargo install gsd-meta-manager`); single native binary, no runtime dependencies.
- Platform support: Linux (full, including live session detection via `/proc`, the sandboxed driver, and the egress server); macOS (TUI and `.planning/` reading; no live session detection; no driver launcher); Windows (compiles in CI, not exercised at runtime; config save refused and driver launcher unavailable).

---

*Stack analysis: 2026-11-03*
