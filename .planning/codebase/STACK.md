# Technology Stack

**Analysis Date:** 2026-08-18

## Languages

**Primary:**
- Rust (edition 2021, `rust-version = "1.87"` per `Cargo.toml`) — entire codebase, `src/`, `tests/`

**Secondary:**
- Shell (`sh`, `bash`) — test fixtures (`tests/fixtures/fake-claude*.sh`), preference hook commands (`hooks.pre_create` / `hooks.post_create` in `src/config.rs`, executed via `sh -c`)
- GSD's own file formats consumed as data, not code: Markdown with YAML frontmatter (`STATE.md`, `ROADMAP.md`, `QUEUE.md`), JSON (`config.json`, `HANDOFF.json`) — parsed in `src/state_reader/`

## Runtime

**Environment:**
- Compiled single Rust binary; no separate runtime needed at execution time
- Async runtime: Tokio 1.53.1 (Cargo.lock), declared as `tokio = { version = "1", features = ["full"] }` in `Cargo.toml`

**Package Manager:**
- Cargo
- Lockfile: present (`Cargo.lock`, committed)

## Frameworks

**Core:**
- ratatui 0.30.2 (Cargo.lock; `Cargo.toml` pins `"0.30"` with `features = ["crossterm"]`) — TUI rendering, `src/ui/`
- crossterm 0.29.0 (also 0.28.1 present transitively via another dep) — terminal backend/input, event-stream feature enabled
- clap 4.6.4, `features = ["derive"]` — CLI argument parsing, `src/cli.rs`

**Testing:**
- Built-in `cargo test` / Rust's `#[test]` harness — no separate test framework
- `assert_fs` 1.1.4 (dev-dependency) — filesystem test fixtures
- `tempfile` 3.27.0 — used both in production code (atomic config writes) and tests

**Build/Dev:**
- No custom build.rs observed
- `.github/workflows/` present for CI (see below)

## Key Dependencies

**Critical:**
- `process-wrap` 9.1.0 (`features = ["tokio1"]`) — wraps `tokio::process::Command` for process-group spawn/signal/kill semantics; the transport for driving the `claude` CLI subprocess (`src/executor/claude.rs`)
- `rustix` 1.1.4 (`features = ["process", "fs"]`) — process-group signalling (`kill_process_group`) and advisory file locking (`flock`) for the driver's single-execution lock (`src/driver/lock.rs`); deliberately chosen over `nix`/`libc`/`fs4` (see Cargo.toml comment, D-08)
- `serde` 1.0.229 / `serde_json` 1.0.151 / `serde_yml` 0.0.13 — structured (de)serialization for `config.json`, GSD's `stream-json` NDJSON protocol, and YAML frontmatter in GSD Markdown files
- `notify` 8.2.0 + `notify-debouncer-full` 0.7.0 — filesystem watching of registered projects' `.planning/` directories (`src/watcher.rs`)
- `uuid` 1.24.0 (`features = ["v4", "serde"]`) — execution/session ids
- `tui-textarea` 0.7.0 (`features = ["ratatui"]`) — multi-line text input widgets in TUI screens

**Infrastructure:**
- `anyhow` 1.0.104 — ergonomic error propagation
- `color-eyre` 0.6.5 — panic/error reporting installed at startup
- `tracing` 0.1.44 / `tracing-subscriber` 0.3.23 (`features = ["env-filter"]`) / `tracing-appender` 0.2.5 — structured logging routed to a file (never stdout, which ratatui owns)
- `dirs` 6.0.0 — platform config-dir resolution (`Config::default_path`)
- `regex` 1.13.1 — pattern matching over GSD Markdown/text state files
- `chrono` 0.4.45 (`features = ["serde"]`) — timestamps throughout journal, config, and state-reader code
- `futures` 0.3.33 — async combinators used alongside Tokio

## Configuration

**Environment:**
- No `.env` file support and no dotenv loading anywhere in the codebase — configuration is either the JSON config file the binary itself manages, or environment variables the user's shell already exports
- Runtime-consulted env vars: `$VISUAL` / `$EDITOR` (fallback `vi`) for the config-edit key (`src/main.rs:264-266`), `$TMUX` for tab-switching capability detection (`src/terminal_switch.rs`), `CLAUDE*`-prefixed vars are explicitly **scrubbed** (never inherited) before spawning the `claude` child (`src/executor/claude.rs`)

**Build:**
- `Cargo.toml` — dependency and metadata manifest, `exclude = [".planning/", "CLAUDE.md"]` from the published crate
- No `.cargo/config.toml`, no workspace — single-crate project

## Platform Requirements

**Development:**
- Rust toolchain ≥ 1.87 (stable)
- `claude` CLI on `PATH` (or overridden via hidden debug-only `--claude-program`/`--claude-args` flags) for driver functionality
- `git` on `PATH` for state-reading and journal/session-detector features
- `tmux` on `PATH` for terminal tab-switching (optional; no-ops with a clear error outside tmux)

**Production:**
- Unix-like target confirmed by process-group code (`std::os::unix::process::CommandExt`, `process_group(0)` in `src/driver/spawn.rs`); the project's own `CLAUDE.md` aspires to cross-platform crossterm/notify but the driver/spawn path as implemented is Unix-specific
- Distributed as a single compiled binary (no runtime dependency beyond the external `claude`, `git`, and optionally `tmux` CLIs)
- CI: `.github/workflows/` present (build/test/lint pipeline)

---

*Stack analysis: 2026-08-18*
