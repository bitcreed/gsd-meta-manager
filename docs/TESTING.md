<!-- generated-by: gsd-doc-writer -->
# Testing

## Test Framework and Setup

`gsd-meta-manager` uses Rust's built-in test harness (`#[test]` and `#[tokio::test]`) — no third-party
test runner is required. Test-only crates (all in `[dev-dependencies]` of `Cargo.toml`):

| Dependency | Version | Purpose |
|------------|---------|---------|
| `assert_fs` | 1 | Temporary directory and filesystem fixture helpers for integration tests |
| `rustix` (`net` feature) | 1.1 | Raw socket calls in the driverctl integration tests (already locked, no new crate) |
| `unicode-properties` (`general-category`) | 0.1.4 | Independent oracle for the invisible-character class sweep in `src/text.rs` |
| `unicode-width` (`cjk`) | 0.2 | Second, independent width oracle for the glyph registry tests in `src/ui/glyphs.rs` |

`tempfile` (3) is a regular dependency that the tests also use for `TempDir` scratch directories.

The declared minimum Rust version is `1.89` (`rust-version` in `Cargo.toml`). Cloning the repo and
running `cargo test` is sufficient for the default suite: there are no test databases or external
services, and no environment variables need to be set (see [Test environment variables](#test-environment-variables)
for the optional ones). Most integration tests are `#![cfg(unix)]` and the launcher/driver ones
that need Linux are `#![cfg(target_os = "linux")]`.

One test needs an external tool: `tests/driver_router_conformance.rs` compares the Rust decision
router against GSD's own `gsd-tools.cjs` and **fails by design when that oracle is not installed**
(see [Router conformance oracle](#router-conformance-oracle)).

## Test Organization

The suite is split across three places:

- **Unit tests (in-source `#[cfg(test)] mod tests`)** — colocated with the module they exercise
  (about 120 source files carry one). These cover parsing, state mutation, UI rendering and pure
  logic.
- **Integration tests (`tests/*.rs`)** — each file compiles as its own crate and drives the public
  library API (`gsd_meta_manager::…`) or the compiled binary (via `env!("CARGO_BIN_EXE_gsd-meta-manager")`).
- **Shared support** — `src/test_support.rs` (exported from the library so integration crates can use
  it), `tests/common/mod.rs` (a `git(dir, args)` helper for tests that build throwaway repositories)
  and `tests/launcher_support/mod.rs` (fake `systemd-run`, `bwrap`, `systemctl` and `claude` shims).

Integration test files, grouped by area (counts are deliberately not listed; they go stale):

| Area | Files | Focus |
|------|-------|-------|
| State reading and registry | `state_reader_test.rs`, `state_reader_gsd_home.rs`, `registry_test.rs`, `registry_worktree_guard.rs` | Frontmatter, `ROADMAP.md` / `STATE.md` / `QUEUE.md` parsing, `GSD_HOME` rooting of the global defaults, `add_project` / `remove_project` / `list_projects`, worktree guard, CLI end to end |
| Agents | `agents_claude.rs`, `agents_fixers.rs`, `agents_process.rs`, `agents_scan.rs`, `agents_unmerged.rs`, `agents_waves.rs`, `agents_workstreams.rs`, `agents_worktree_plans.rs` | Agent/worktree scanning against throwaway git repositories built with `tests/common` |
| Driver | `driver_new.rs`, `driver_prompt.rs`, `driver_router_table.rs`, `driver_router_conformance.rs`, `driver_tab_launch_e2e.rs` | `driver new`/`check`/`launch`, the master prompt and its goldens, the decision router and its conformance oracle |
| driverctl | `driverctl_client.rs`, `driverctl_frontmatter.rs`, `driverctl_liveness.rs`, `driverctl_lock_race.rs`, `driverctl_race.rs`, `driverctl_reconcile.rs`, `driverctl_replay.rs`, `driverctl_serve.rs`, `driverctl_stop_guard.rs`, `driverctl_store.rs` | The run store, serve loop, client, lock races, liveness, reconcile and replay |
| Launcher and sandbox | `launcher_acceptance.rs`, `launcher_auth.rs`, `launcher_bridge.rs`, `launcher_caps.rs`, `launcher_config_dir.rs`, `launcher_degraded.rs`, `launcher_e2e.rs`, `launcher_egress.rs`, `launcher_push.rs`, `launcher_reconcile.rs`, `launcher_sandbox.rs`, `launcher_sandbox_data_mask.rs`, `launcher_stop.rs`, `launch_target_cli.rs` | Launcher behaviour on fake `systemd-run` / `bwrap` / `systemctl` / `claude` shims; no real sandbox or model |
| Live launcher (opt-in) | `launcher_live_egress.rs`, `launcher_live_isolation.rs`, `launcher_live_managed.rs`, `launcher_live_scope.rs`, `launcher_live_token_reach.rs` | `#[ignore]`d; need `bwrap`, a user systemd, network and sometimes a token |
| One-off tasks | `oneoff_e2e.rs`, `oneoff_taskctl.rs`, `oneoff_runtime_seam_guard.rs` | `task run|status|stop|land` end to end on the fake harness, `taskctl report`, runtime-neutrality scan |
| Safety envelope | `envelope_pr_cap.rs` | The pull-request cap enforced through the ledger, with injected timestamps |
| Acceptance harness | `prompt_acceptance.rs`, `launcher_acceptance.rs` | Offline proofs about `scripts/prompt-acceptance.sh` and `scripts/launcher-acceptance.sh` (selftest, pre-spend refusals, evidence-tree invariants) |
| Mechanical guards | `spawn_seam_guard.rs`, `async_blocking_guard.rs`, `oneoff_runtime_seam_guard.rs` | Source-scanning tests: the process-spawn allowlist, blocking calls inside `async fn`, agent naming outside `src/oneoff/runtime.rs` |
| Misc | `bounded_output.rs`, `editor_command.rs`, `next_cli.rs` | `stdout_within` timeouts, editor command resolution, the `next` CLI |

The guard tests read the source tree and fail if a listed file is missing, so a rename cannot
silently retire a guard. If you move or rename a file named in one of them, update the guard's list.

Most test inputs are inline string literals written into per-test temporary directories. The exceptions
are the recorded fixtures under `tests/fixtures/` — see [Fixtures](#fixtures) below.

## Running Tests

All commands run from the repository root.

```bash
# Run the full suite (unit + integration)
cargo test

# Run with optimizations — slower to build, faster to execute; matches release behavior
cargo test --release

# Run a single integration test file (name = file stem under tests/)
cargo test --test state_reader_test
cargo test --test registry_test
cargo test --test oneoff_e2e

# Run only library unit tests
cargo test --lib

# Filter by test name substring (matches any test whose name contains the string)
cargo test parse_queue_md
cargo test add_project_with

# Show stdout/stderr from passing tests (default: only failing tests print)
cargo test -- --nocapture

# Run tests single-threaded — useful when debugging tests that share global state
cargo test -- --test-threads=1

# Keep going after a failing test binary (what the release workflow runs)
cargo test --no-fail-fast
```

### Ignored tests

Tests that need a live service, a sandbox tool or real model spend are marked `#[ignore]` with
the reason in the attribute, so the default `cargo test` loop stays fast and offline. Run them
explicitly:

```bash
# Every ignored test in the suite
cargo test -- --ignored

# One opt-in file, with output
cargo test --test launcher_live_egress -- --ignored --nocapture
```

The ignored tests fall into these groups:

- **Live launcher tests** (`tests/launcher_live_*.rs`, one in `tests/launcher_egress.rs`) — need a working
  `bwrap`, a user systemd and/or network; some need a stored token and spend a few cents on Haiku
  (the reason string states the amount).
- **Paid prompt-acceptance cases** (`tests/prompt_acceptance.rs`) — only call
  `scripts/prompt-acceptance.sh`, which stays behind its own consent, pre-registration and ledger gates.
- **On-demand reads of real files** — `tests/state_reader_test.rs` reads the file named by
  `GSD_META_MANAGER_REAL_STATE_MD`; one test in `tests/agents_worktree_plans.rs` reads a local project checkout outside the repository.
- **Golden regeneration** — see `REGENERATE_PROMPT_VARIANT` below.

Do not run `cargo test -- --ignored` casually: some of these tests spend real model budget.

### Test environment variables

None are required for the default suite. These are read by the tests themselves:

| Variable | Used by | Effect |
|----------|---------|--------|
| `GSD_META_MANAGER_ALLOW_MISSING_ORACLE` | `tests/driver_router_conformance.rs` | Any non-blank value lets the conformance test pass when no GSD oracle can be found. Unset (the default), a missing oracle fails the test. |
| `CODEX_HOME` | `tests/driver_router_conformance.rs` | Where to look for a Codex-installed `gsd-core` (falls back to `~/.codex` when unset or empty). |
| `REGENERATE_PROMPT_VARIANT=1` | `tests/driver_prompt.rs` | Rewrites the committed `tests/fixtures/prompt/MASTER-PROMPT.no-gap-rules.md` variant; it is never edited by hand. |
| `GSD_RUNTIME` | `tests/driver_router_conformance.rs` (set, not read) | Not something you set. The test sets `GSD_RUNTIME=claude` on each `gsd-tools query` child it starts. gsd-core decides how to spell commands from this variable (`resolveRuntime`), and it overrides a Codex install's `.gsd-runtime` marker. Pinned to `claude`, the oracle answers with `/gsd-…` commands, which is the spelling stored in the rule table. Without the pin a Codex install would answer `$gsd-…`. A `GSD_RUNTIME` already set in your shell does not affect the test. The production `smart-entry` call in `src/state_reader/queue_md.rs` uses the same pin. |
| `GSD_META_MANAGER_REAL_STATE_MD` | `tests/state_reader_test.rs` (ignored test) | Path to a real `STATE.md` to parse. |
| `FAKE_SHIM_LOG`, `FAKE_CLAUDE_DIR`, `FAKE_CLAUDE_SCENARIO`, `FAKE_GH_LOG` | fake tools from `tests/launcher_support` (`FAKE_GH_LOG` from the fake `gh` in `tests/launcher_push.rs`) | Set by the harness itself for the fake scripts; not meant to be set by hand. |

The launcher harness also gives every child process a private `HOME` and `XDG_RUNTIME_DIR`,
`XDG_CONFIG_HOME`, `XDG_DATA_HOME` and `XDG_STATE_HOME` under a short scratch directory, so tests
never touch the real user's files.

`GSD_MM_ENVELOPE_ROOT` (documented in [CONFIGURATION.md](CONFIGURATION.md)) overrides where the
per-alias safety-envelope ledger lives, so a test that must run the real binary can point the ledger at a temporary
directory instead of `~/.local/share`. The envelope integration tests (`tests/envelope_pr_cap.rs`)
do not need it: they call the `…_in(root, alias, …)` library functions with an explicit temp root.

### Router conformance oracle

`tests/driver_router_conformance.rs` runs every fixture through GSD's own router (`gsd-tools.cjs`)
and the Rust table and requires them to agree. The oracle is looked up at
`~/.claude/gsd-core/bin/gsd-tools.cjs`, then `${CODEX_HOME:-~/.codex}/gsd-core/bin/gsd-tools.cjs`,
and run through `node`. On a machine without GSD installed, the test fails instead of silently
skipping. To provision it the way the release runner does:

```bash
./scripts/install-conformance-oracle.sh
```

### Optional: cargo-nextest

`cargo-nextest` is listed as a recommended dev tool in `CLAUDE.md`. If you have installed it
(`cargo install cargo-nextest --locked`), you can substitute:

```bash
cargo nextest run                       # full suite, parallel, prettier output
cargo nextest run --test registry_test  # one integration test file
cargo nextest run parse_queue_md        # filter by name
```

The project does not require nextest — CI-equivalent commands work with plain `cargo test`.

## Writing New Tests

### Naming conventions

Both styles appear in the codebase; either is acceptable, but prefer matching the file you are
editing:

- **Descriptive snake_case** (used in `tests/registry_test.rs`):
  `add_project_with_duplicate_alias_returns_error`, `list_projects_returns_sorted_by_alias`.
- **`test_`-prefixed** (used in `tests/state_reader_test.rs` and most in-source unit tests):
  `test_parse_real_state_md`, `test_parse_queue_md_basic`.

### Unit tests (in-source)

Add tests in a `#[cfg(test)] mod tests` block at the bottom of the module:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_minimal_frontmatter() {
        let content = "---\nstatus: active\n---\n";
        let fm = parse_state_md(content).unwrap();
        assert_eq!(fm.status, "active");
    }
}
```

For async code (the watcher, the event loop), use `#[tokio::test]` — `tokio` is already a runtime
dependency with the `full` feature enabled, so no extra setup is required:

```rust
#[tokio::test]
async fn watcher_emits_event_on_file_change() {
    // ...
}
```

### Integration tests (`tests/`)

Each file in `tests/` compiles as its own crate and tests the public surface of
`gsd_meta_manager::…`. Use `assert_fs::TempDir` for filesystem fixtures:

```rust
use assert_fs::prelude::*;
use assert_fs::TempDir;
use gsd_meta_manager::config::Config;
use gsd_meta_manager::registry::add_project;

#[test]
fn add_project_with_valid_alias_and_planning_dir_succeeds() {
    let temp = TempDir::new().unwrap();
    temp.child(".planning").create_dir_all().unwrap();

    let mut config = Config::new();
    let result = add_project(&mut config, "myapp", temp.path());
    assert!(result.is_ok());
}
```

For tests that must exercise the compiled binary (CLI end-to-end), shell out via
`std::process::Command::new("cargo").args(["run", "--quiet", "--", ...])` — see
`end_to_end_add_then_list_via_cli` in `tests/registry_test.rs` for the canonical pattern. Keep
these sparingly; they're slow because they rebuild the binary on first run.

### Faking external tools

Tests never call the real `systemd-run`, `bwrap`, `systemctl`, `claude` or `gh`. Launcher and
one-off tests `mod launcher_support;` and put fake `/bin/sh` executables first on `PATH`; the fakes log
their argv, and the fake `claude` runs a scenario script the test writes. Use short scratch paths
(unix socket paths are limited to 107 bytes). Tests that build throwaway git repositories use
`mod common;` and `common::git`.

### Fixture strategy

Parser inputs are mostly **inline string literals**, which keeps each test self-contained. When a
test needs an on-disk layout, build it with `assert_fs::TempDir` or `tempfile::TempDir` and
`.child("...").write_str("...")`. Larger recorded inputs live under `tests/fixtures/`.

## Fixtures

All of the following are committed under `tests/fixtures/`:

| Directory | Contents |
|-----------|----------|
| `transcripts/` | `01`–`08` `*.ndjson` are redacted captures of real `claude` 2.1.220 runs; `09-rate-limit-rejected.ndjson` is the one synthesised file (labelled in the directory README). Loaded with `include_str!`; they drive `src/executor/stream_json.rs` and `src/driver/rate_limit.rs`. Do not edit them to make a test pass; capture a new one instead. |
| `roadmaps/` | Sanitised excerpts of real `ROADMAP.md` / `STATE.md` / `HANDOFF.json` files used by the state reader and Roadmap tab tests. The README in the directory lists each file's source and the shape it pins. |
| `roadmap-shapes/` | Further roadmap layouts for the parser. |
| `prompt/` | The master prompt and its generated `no-gap-rules` variant (goldens for `driver_prompt.rs`). |
| `driverctl/` | Run-store scenarios (`direction/`, `golden/`, `replay/`, `spike003/`). |

## Coverage Requirements

No coverage threshold is configured. There is no `tarpaulin.toml`, `.codecov.yml`, or
`cargo-llvm-cov` setup, and no `coverageThreshold` enforcement anywhere in the repo. Coverage is
maintained by review — new logic in `state_reader/`, `registry`, and parsers is expected to land
with tests in the same change.

If you want a local coverage report, install one of the standard Rust coverage tools manually:

```bash
cargo install cargo-llvm-cov
cargo llvm-cov --html
```

## Quality Gates

The release process documented in `CLAUDE.md` requires the following to pass cleanly before
tagging a milestone. Treat these as the de-facto pre-merge checks even outside release time:

```bash
cargo build                    # must succeed without warnings
cargo test                     # full test suite must pass
cargo clippy -- -D warnings    # lints promoted to errors; any clippy warning fails the gate
```

A typical local pre-commit loop:

```bash
cargo build && cargo test && cargo clippy -- -D warnings
```

To rehearse the whole release pipeline before tagging, use the local dry run:

```bash
./scripts/pre-tag-check.sh --container vX.Y.Z   # runs the gates in an ubuntu-24.04 lookalike
```

Container mode provisions the conformance oracle exactly as the publish job does. A bare
`./scripts/pre-tag-check.sh vX.Y.Z` runs on your machine, where GSD is installed, so a green result
there cannot certify the publish job.

## CI Integration

The only workflow is `.github/workflows/release.yml`, triggered by pushing a `vX.Y.Z` tag. Its jobs
pin `ubuntu-24.04`:

1. `msrv` — installs the declared `rust-version` toolchain and runs `cargo check --all-targets --locked`.
2. `cross-target-check` — `cargo check` and `cargo clippy --all-targets -- -D warnings` for non-Linux
   targets (compile only; tests are not run on them).
3. `publish` — provisions the conformance oracle (`./scripts/install-conformance-oracle.sh`), checks
   the tag against the `Cargo.toml` version, then runs `cargo build --release`, `cargo test --no-fail-fast`,
   `cargo clippy -- -D warnings` and `cargo publish --locked`.

There is no workflow that runs tests on pull requests or ordinary pushes; run the gates above
locally before opening a PR.

## Next Steps

- See [DEVELOPMENT.md](DEVELOPMENT.md) (if generated) for the broader local development workflow.
- See [ARCHITECTURE.md](ARCHITECTURE.md) for the module layout that test files mirror.
- See [CONFIGURATION.md](CONFIGURATION.md) for environment variables that affect runtime
  behavior — none are required for the default test suite.
