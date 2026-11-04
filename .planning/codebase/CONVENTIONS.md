---
last_mapped_commit: d7381ab615e5181d33f3b6a5e02cf4d2a42594af
last_mapped_at: 2026-11-03
---
# Coding Conventions

**Analysis Date:** 2026-11-03

## Language and Toolchain

The crate is Rust, edition 2021, declared `rust-version = "1.89"` in `Cargo.toml` (the MSRV floor).
`docs/TESTING.md` still says `1.88+`; the `Cargo.toml` value is authoritative.

- Formatting: no `rustfmt.toml` or `.rustfmt.toml` exists. Code follows default `rustfmt` style. No CI job runs `cargo fmt --check`, so run `cargo fmt` before committing.
- Linting: CI runs `cargo clippy -- -D warnings` (`.github/workflows/release.yml` line 158) and `cargo clippy --locked --all-targets -- -D warnings` in the release build matrix (line 98). Treat clippy warnings as errors.
- Lint suppression is rare and local. Examples: `#[allow(clippy::type_complexity)]` on `src/launcher/push.rs:347`, `#![allow(dead_code)]` in `tests/common/mod.rs` (justified in its header comment because each integration target compiles the module separately).
- No crate-level `#![deny(...)]` or `[lints]` table exists in `Cargo.toml`.

## Naming Patterns

**Files and modules:**
- `snake_case.rs` everywhere: `src/oneoff/cancel.rs`, `src/state_reader/queue_md.rs`, `src/ui/screens/driver_confirm.rs`.
- Parser or file-format modules are named after the file they read, with an `_md` or `_json` suffix: `src/state_reader/roadmap_md.rs`, `src/state_reader/state_md.rs`, `src/state_reader/config_json.rs`, `src/state_reader/queue_md.rs`.
- Module directories hold a `mod.rs` entry point: `src/oneoff/mod.rs`, `src/launcher/mod.rs`, `src/driverctl/mod.rs`, `src/agents/adapters/mod.rs`. Sibling-file modules are also used (`src/launcher/push.rs` with `src/launcher/push/planted.rs`).
- CLI wrappers are suffixed `_cli.rs`: `src/driver_cli.rs`, `src/driverctl_cli.rs`, `src/taskctl_cli.rs`, `src/next_cli.rs`.
- Integration test files under `tests/` are prefixed by the subsystem: `tests/driverctl_*.rs`, `tests/launcher_*.rs`, `tests/agents_*.rs`, `tests/state_reader_test.rs`, `tests/registry_test.rs`.

**Functions:**
- `snake_case`. Predicates read as questions or states: `valid_oneoff_run_id`, `runtime_max_sec_for`.
- Parsers use `parse_` and return `Option` for "not this shape" or `Result` for "invalid with reason":
  - `Option`: `parse_backlog_dir_name` (`src/state_reader/backlog.rs:35`), `parse_waves_manifest` (`src/state_reader/plan_waves.rs:206`), `parse_git_log_line` (`src/state_reader/git_ops.rs:1050`).
  - `Result` with a typed error: `parse_driver_frontmatter` (`src/state_reader/driver_frontmatter.rs:480`) returns `Result<DriverFrontmatter, FrontmatterError>`.
- Private helpers that wrap an error into a variant are named for their context: `io_error(context, e)` in `src/oneoff/launch.rs:139`.
- Pure state-machine methods take injected time rather than calling the clock. Use `now: DateTime<Utc>` or `Instant` parameters (`src/launcher/outcome.rs:130`, `src/launcher/auth.rs:181`).

**Variables:** `snake_case`. Short names are normal in tight loops (`req`, `ctx`, `dir`). Use `ctx` for shared per-session context structs (`fn run_session(ctx: &Ctx<'_>, ...)` in `src/oneoff/launch.rs:352`).

**Types:**
- `PascalCase` structs and enums. Error enums end in `Error` (`LauncherError` in `src/launcher/mod.rs:132`, `LockError` in `src/error.rs`) or are named after their domain (`RecordStatus`, `ReportSource`, `LandingKind`, `MergeHow` in `src/oneoff/store.rs`).
- Enum variants that carry data use struct-style fields: `LauncherError::Io { detail: String }`, `LockError::HeldBy { run_id, pgid, started_at }`.
- Wire-format enums are annotated with serde renames: `#[serde(rename_all = "lowercase")]` (`RecordStatus`, `MergeHow`) or `#[serde(tag = "kind", rename_all = "snake_case")]` for tagged unions (`ReportSource`).

**Constants:** `SCREAMING_SNAKE_CASE` with a doc comment. Limits and environment-variable names are exported constants:
- `MAX_CHANGED_FILES`, `RECORD_SCHEMA` in `src/oneoff/store.rs`
- `MAX_REQUEST_BYTES`, `STOP_GRACE`, `STOP_GRACE_ENV` in `src/oneoff/cancel.rs`
- `DEGENERATE` in `src/test_support.rs`

## Code Style

**Derives:** Data types derive `Debug, Clone` plus `PartialEq, Eq` where comparison is meaningful. Serialized types add `Serialize, Deserialize`. Copyable enums add `Copy`. Example from `src/oneoff/cancel.rs`:

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum StopKind {
    Pause,
    Hard,
    Kill,
}
```

**Display and Error impls are hand-written.** `src/error.rs` states the choice explicitly: one small enum does not justify an error-derive dependency. Do not add `thiserror` for a single enum. Follow the `impl fmt::Display` plus `impl std::error::Error {}` pattern in `src/error.rs`.

**Long string literals** are split with a trailing backslash continuation inside `write!` (see `LockError`'s `Display` in `src/error.rs`).

**Imports:** Group as `std`, then third-party crates, then `crate::` and `super::`, separated by blank lines. This is the dominant pattern but it is not enforced. `src/state_reader/queue_md.rs` mixes `super::` imports between `std` imports, so match the file you are editing.

**Sections inside large files:** Use banner comments to separate logical blocks:

```rust
// ============================================================================
// The shared `git` helper for the integration tests ...
// ============================================================================
```

Banners appear at roughly 60 places across `src/` and `tests/`, mostly in guard tests and seam modules.

## Documentation and Comments

**Module headers** are `//!` blocks that state what the module owns, what it does not own, and which requirement or decision IDs drove it. Read the header first when working in an unfamiliar module. Examples: `src/oneoff/cancel.rs`, `src/oneoff/store.rs`, `src/error.rs`.

**Decision and requirement IDs are part of the convention.** The codebase tags design choices with IDs in comments, roughly 1,600 occurrences of patterns like `D-19`, `C-06`, `CTRL-05`, `T-28-04`, `I-09`, and quick-task IDs such as `quick 261103-klf`. Keep this convention when you change a guarded behavior: cite the ID that governs it, and do not invent new IDs.

**Doc comments on items** are `///` and usually state the invariant, not the mechanism. Example: `/// The request file inside a run dir ...` in `src/oneoff/cancel.rs`.

**Tests must be sandbox-neutral.** The suite runs inside the one-off sandbox (hostname `gsdmm-sandbox`, no network, no ambient git identity). Assert keys and fields (`"\nsandbox_missing:"` in `tests/launcher_degraded.rs`), not substrings a hostname or path can contain; pin git identity and default branch explicitly in test helpers (`tests/oneoff_e2e.rs::git`). See `.planning/codebase/TESTING.md`.

**Comments that assert a guarantee should be backed by a test.** `tests/async_blocking_guard.rs` and `tests/spawn_seam_guard.rs` exist because prose comments were not enough. A comment that says "never call X here" should have a guard test or the rule is not enforced.

**Explanatory comments are long on purpose.** When a line looks odd (a `#[ignore]`, an `unsafe`-adjacent syscall, a seam), the comment states the failure it prevents. Keep that style for non-obvious code.

## Error Handling

**Two layers:**

1. Application code uses `anyhow::Result` with `.with_context(...)` at seams that callers touch (`src/config.rs`, `src/registry.rs`, `src/state_reader/git_ops.rs`). `src/main.rs` returns `anyhow::Result<()>`. `color-eyre` is the top-level report handler.
2. Domain code that callers must match on uses a typed error enum:
   - `src/error.rs`: `LockError` (variants `HeldBy`, `HeldByUnknownRun`, `Unavailable`). The header says only the lock refusal is typed.
   - `src/launcher/mod.rs:132`: `LauncherError` with struct variants (`InvalidRun`, `Locked`, `Auth`, `Spawn`, `Io`, `Store`, `UnsupportedPlatform`) and an `exit_code()` method.

**Rule:** If a caller will branch on the failure, make it a typed enum variant. If it only gets reported, use `anyhow` with context.

**Validation returns early with a typed error before any side effect.** `validate(opts)` in `src/oneoff/launch.rs:146` runs before anything is probed or written. Follow that order.

**Error details are strings.** Enum variants carry `detail: String` built with `format!("{context}: {e}")`. Prefer the helper pattern at `src/oneoff/launch.rs:139`.

**`unwrap()` and `expect()`:** About 230 calls appear in non-test code, counted by a rough scan that stops at the first `#[cfg(test)]` in each file, so the count is approximate. Do not add new ones on fallible paths that take external input (files, git output, process output, sockets). Use `?` or an explicit error. The existing uses are mostly on infallible or internally constructed values.

**Security-relevant inputs are validated and bounded.** Examples: `MAX_REQUEST_BYTES` read limit (`src/oneoff/cancel.rs`), `MAX_CHANGED_FILES`, and report headline length limits in `src/oneoff/report.rs`. Untrusted strings from agents are wrapped or checked for invisible and bidi-control characters; see `src/text.rs` and `src/test_support.rs` (`DEGENERATE`).

## Logging

**Framework:** `tracing` with `tracing-subscriber` (env-filter) and `tracing-appender`. Output goes to a log file, never stdout, because ratatui owns the terminal (`src/main.rs` calls `init_tracing` for the TUI path).

**Subcommands that run inside the sandbox must not create a log file or write stdout.** `src/main.rs` dispatches `driverctl`, `taskctl`, and `bridge` before `init_tracing` for that reason. Preserve this ordering when adding a sandbox-facing command.

**Usage:** `tracing::warn!`, `tracing::info!`, `tracing::error!`, `tracing::debug!` with structured fields. About 40 call sites, concentrated in `src/app.rs`, `src/watcher.rs`, `src/bounded_output.rs`, `src/state_reader/config_write.rs`.

**`println!` / `eprintln!`:** Reserved for CLI subcommand output (`driverctl_cli`, `taskctl_cli`, `task_cli`, `next_cli`). Do not use them in TUI code paths.

## Async and Blocking

- Tokio runtime with `full` features. Blocking filesystem or syscall work inside an `async fn` must go through `tokio::task::spawn_blocking`. `tests/async_blocking_guard.rs` enforces a lexical version of this rule. Keep new blocking calls behind `spawn_blocking` or a hand-off, or the guard test fails.
- Process spawning is restricted to an allowlist. `tests/spawn_seam_guard.rs` walks `src/` and fails on unlisted spawn sites. Add a new spawn site to that allowlist deliberately, not casually.
- `std::sync::Mutex` must not be held across `.await` (see `CLAUDE.md` stack notes).

## Comments and Dead Code

- Commented-out code is not used. Remove it.
- `#[allow(dead_code)]` is used only on shared test helpers (`tests/common/mod.rs`).
- `TODO` markers are rare. Track follow-up work in `.planning/` todos rather than inline.

## Function Design

- Functions are often long in `src/app.rs`, `src/launcher/`, and `src/oneoff/`. Prefer extracting a pure helper with injected inputs (time, paths) when a unit test needs it.
- Parameters: pass `&Path` and `&str` for inputs. Use `impl Into<...>` or `impl Display` for error-context helpers (`io_error(context: &str, e: impl Display)`).
- Return values: `Option` for "not applicable", `Result` for "failed with reason", plain values otherwise. Do not return sentinel strings or `-1`.
- Exit codes are returned from CLI runners: `std::process::exit(driverctl_cli::run(action))`. `LauncherError::exit_code()` maps errors to codes.

## Module Design

- Public surface is explicit. Items shared across modules inside the crate use `pub(crate)` (for example `claim_new_run` in `src/oneoff/launch.rs:199`).
- Library root is `src/lib.rs`; the binary entry point is `src/main.rs`. Integration tests import the library as `gsd_meta_manager::...`.
- Barrel-style `pub use` re-exports are used in `mod.rs` files to flatten a subsystem's API. Follow the existing re-export list when adding a new public type.
- Shared test fixtures that must be reachable from integration tests ship in the library: `src/test_support.rs` is not `#[cfg(test)]` (decision D-17-5). Do not re-add `#[cfg(test)]` to it.

## Serialization and Persisted Records

- Persisted records carry a schema version constant: `RECORD_SCHEMA` in `src/oneoff/store.rs`, `SCHEMA` in `src/oneoff/report.rs`. Bump it on incompatible changes.
- Host-written records must be derived by the host from git, not trusted from agent output. See the header of `src/oneoff/store.rs`.
- Serde defaults: use `#[serde(rename_all = ...)]` on the type, not per field, unless one field differs.

## Git Commit Conventions

Commits follow Conventional Commits with a scope:

- `feat(quick-261103-klf): ...`
- `docs(261103-klf): ...`
- `chore(release): vX.Y.Z — <milestone name>`

Quick-task IDs go in the scope. Release commits itemize the shipped work in the body (see `CLAUDE.md` Release Process).

---

*Convention analysis: 2026-11-03*
