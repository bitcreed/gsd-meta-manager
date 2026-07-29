<!-- generated-by: gsd-doc-writer -->
# Testing

## Test Framework and Setup

`gsd-meta-manager` uses Rust's built-in test harness (`#[test]` and `#[tokio::test]`) — no third-party
test runner is required. The only test-time dependency is:

| Dependency | Version | Purpose |
|------------|---------|---------|
| `assert_fs` | 1 (dev-dependency in `Cargo.toml`) | Ergonomic temporary directory and filesystem fixture helpers for integration tests |
| `tempfile` | 3 (runtime dependency, also used in tests) | `TempDir` for unit-test scratch directories |

No setup is required beyond a working Rust toolchain (`1.87+ stable`, per the project's stack
decisions). Cloning the repo and running `cargo test` is sufficient — there are no test databases,
external services, or environment variables to configure.

## Test Organization

The suite is split across two locations following standard Rust conventions:

- **Unit tests (in-source `#[cfg(test)] mod tests`)** — colocated with the module they exercise.
  These cover pure parsing and state-mutation logic.
- **Integration tests (`tests/` directory)** — black-box tests against the public library API
  (`gsd_meta_manager::…`) and one end-to-end test that invokes the compiled binary via `cargo run`.

Current breakdown (counted directly from the source tree):

| Location | Tests | Focus |
|----------|------:|-------|
| `tests/state_reader_test.rs` | 25 | Frontmatter parsing, roadmap/STATE.md/QUEUE.md parsing, integration across the `state_reader` module |
| `src/state_reader/disk_status.rs` | 17 | Git status, dirty-file detection, lock-file inspection |
| `tests/registry_test.rs` | 12 | `add_project` / `remove_project` / `list_projects` plus one end-to-end CLI test |
| `src/state_reader/state_md.rs` | 8 | `STATE.md` frontmatter extraction |
| `src/state_reader/queue_md.rs` | 8 | `QUEUE.md` action parsing, write roundtrips, idle-project suggestions |
| `src/state_reader/roadmap_md.rs` | 7 | `ROADMAP.md` phase parsing |
| `src/browser.rs` | 7 | `.planning/` document tree traversal |
| `src/registry.rs` | 6 | Auto-registration of GSD projects discovered on disk |
| `src/project_creator.rs` | 6 | New-project scaffolding |
| `src/change_tracker.rs` | 5 | Change-detection bookkeeping |
| `src/state_reader/config_json.rs` | 4 | GSD `config.json` parsing |
| `src/state_reader/backlog.rs` | 4 | Backlog item counting |
| `src/watcher.rs` | 3 | Filesystem watcher lifecycle (uses `#[tokio::test]`) |
| `src/session_detector.rs` | 3 | Active-session detection |
| `src/app.rs` | 3 | Top-level event handling |

Most test inputs are constructed inline as string literals (see the frontmatter examples in
`tests/state_reader_test.rs`) and written into per-test `TempDir`s when filesystem layout matters.
The one exception is the executor, which has a `tests/fixtures/` directory — see
[The executor's process fixtures](#the-executors-process-fixtures) below.

## Running Tests

All commands run from the repository root.

```bash
# Run the full suite (unit + integration)
cargo test

# Run with optimizations — slower to build, faster to execute; matches release behavior
cargo test --release

# Run only integration tests in tests/
cargo test --test state_reader_test
cargo test --test registry_test

# Run only library unit tests (skips the binary's own #[cfg(test)] blocks if any)
cargo test --lib

# Filter by test name substring (matches any test whose name contains the string)
cargo test parse_queue_md
cargo test add_project_with

# Show stdout/stderr from passing tests (default: only failing tests print)
cargo test -- --nocapture

# Run tests single-threaded — useful when debugging tests that share global state
cargo test -- --test-threads=1
```

### Ignored tests: the executor teardown grace

A small number of tests are marked `#[ignore]` because they wait out a **real ten-second
period** — the grace the executor grants a `claude` process group between the terminate signal
and the uncatchable one. `cargo test` skips them so the default loop stays fast. Run them
explicitly:

```bash
# Every ignored test in the suite
cargo test -- --ignored

# Just the process-lifecycle ones
cargo test --test executor_lifecycle -- --ignored
```

**Run these before tagging a release.** The ignored
`a_child_that_ignores_the_terminate_signal_is_still_killed_and_reaped` is currently the *only*
test that exercises the escalation half of the teardown — a child that ignores the terminate
signal, the grace expiring, the uncatchable signal, and the reap. The fast tests cover the
path where the terminate signal is honoured, so a regression that breaks escalation alone
passes `cargo test` and fails only here.

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

### Fixture strategy

Outside the executor, the codebase deliberately uses **inline string literals** for parser
inputs rather than separate fixture files. This keeps each test self-contained and makes the
expected shape obvious at the test site. When a test needs an on-disk layout, build it with
`assert_fs::TempDir` or `tempfile::TempDir` and `.child("...").write_str("...")`.

## The executor's process fixtures

`src/executor/` drives a real `claude` CLI over a duplex NDJSON protocol, and two of its
concerns cannot be tested with inline strings: the wire protocol, and the process lifecycle.
Both live under `tests/fixtures/`.

### Transcripts

`tests/fixtures/transcripts/*.ndjson` are **redacted captures of real `claude` 2.1.220 runs**,
not synthesised examples. They are loaded with `include_str!` at compile time — no I/O, no
`TempDir` — and drive the parser, the capability gate and the outcome-derivation matrix. Their
value is precisely that nobody wrote them by hand: a model that only parses invented input
proves nothing about a CLI whose stream is 41% undocumented message subtypes. Do not edit them
to make a test pass; capture a new one instead.

### Process stand-ins

`tests/fixtures/fake-claude*.sh` are small shell scripts the executor is pointed at **instead
of the real binary**, which is how the whole transport — argv construction, process-group
spawn, the pipe tasks, the gate, framing, teardown — is exercised with zero subscription,
network or quota dependency. Each expresses one behaviour a transcript cannot:

| Stand-in | Behaviour | What it makes provable |
|----------|-----------|------------------------|
| `fake-claude.sh` | Replays a transcript line by line, exits with a given code | The end-to-end read path |
| `fake-claude-echo.sh` | Reads stdin and answers it; logs every line written to it | `send`/`interrupt` correlation, and that a refused run writes **zero bytes** |
| `fake-claude-slow.sh` | Emits N heartbeats then either goes silent or finishes | Both deadlines: a silent run is stalled, a chattering one is not |
| `fake-claude-spawner.sh` | Backgrounds a long-lived grandchild and announces its pid | That teardown reaches the whole process **tree**, not just the direct child |
| `fake-claude-deaf.sh` | Ignores the terminate signal entirely | That the escalation to the uncatchable signal actually fires |

All five must keep the executable bit in git (`git ls-files -s` must report mode `100755`); a
stand-in committed as `100644` fails at spawn with a permission error rather than a useful one.

### Why the lifecycle tests are Unix-only

`tests/executor_lifecycle.rs` opens with `#![cfg(unix)]`, and so does `src/executor/claude.rs`'s
module gate. Process-group spawn and group-wide signalling are `#[cfg(unix)]` in `process-wrap`,
and taking the whole tree down is the entire reason that dependency exists — there is no
portable subset of the behaviour left to test. The wire model, the capability gate and outcome
derivation stay portable and are tested everywhere.

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

`cargo clippy -- -D warnings` is the load-bearing static analysis step — it catches issues the
test suite cannot (unused code paths, suboptimal patterns, unsafe casts). Run it before opening a
PR.

A typical local pre-commit loop:

```bash
cargo build && cargo test && cargo clippy -- -D warnings
```

## CI Integration

No CI workflow is currently checked into the repository — there is no `.github/workflows/`
directory, no `.gitlab-ci.yml`, and no other CI configuration. The quality gates above are
enforced locally and at release time by the maintainer per the process in `CLAUDE.md`.

<!-- VERIFY: whether a hosted CI service runs these checks outside the repo -->

## Next Steps

- See [DEVELOPMENT.md](DEVELOPMENT.md) (if generated) for the broader local development workflow.
- See [ARCHITECTURE.md](ARCHITECTURE.md) for the module layout that test files mirror.
- See [CONFIGURATION.md](CONFIGURATION.md) for environment variables that affect runtime
  behavior — none are required for tests.
