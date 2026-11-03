---
last_mapped_commit: 5146cf11780714b72ed3b2e6224e849a93e10a08
last_mapped_at: 2026-11-03
---
# Testing Patterns

**Analysis Date:** 2026-11-03

## Test Framework

**Runner:**
- Rust built-in test harness (`#[test]`, and `#[tokio::test]` for async, about 119 uses). No third-party runner is required.
- Config: none. Test behavior comes from `Cargo.toml` `[dev-dependencies]` and the MSRV floor `rust-version = "1.89"`.
- `docs/TESTING.md` says `1.88+`. The `Cargo.toml` value is authoritative.

**Dev dependencies (`Cargo.toml`):**
- `assert_fs = "1"`: temporary directory and filesystem fixtures. Used in very few places; most tests use `tempfile` directly.
- `rustix = { version = "1.1", features = ["net"] }`: dev-only, for one `listen(2)` test in `driverctl_cli`.
- `unicode-properties` (with the `general-category` feature): an independent Unicode oracle for the invisible-character class (see `Cargo.toml` comments and `src/text.rs` tests). Keep this oracle independent of the production derivation.

**Runtime-but-used-by-tests:** `tempfile = "3"` is a normal dependency, used by both source tests and integration tests.

**Assertion library:** Standard `assert!`, `assert_eq!`, `assert_ne!`. No `pretty_assertions` or `rstest` is used.

**Run commands:**

```bash
cargo test --no-fail-fast          # Full suite (unit + integration), matches CI gate 4
cargo test --lib                   # Unit tests only (in-module #[cfg(test)] blocks)
cargo test --test driverctl_store  # One integration test file
cargo test parse_queue_md          # Filter by name substring
cargo test -- --ignored            # Ignored tests (live services, real model spend, tmux)
cargo test -- --nocapture          # Show stdout from passing tests
cargo test -- --test-threads=1     # Serialize when tests share global state
cargo clippy -- -D warnings        # Lint gate, matches CI gate 5
cargo clippy --all-targets -- -D warnings   # Also lints test code
cargo nextest run                  # Optional, if cargo-nextest is installed
```

**Full pre-tag verification:** `./scripts/pre-tag-check.sh vX.Y.Z` runs six gates locally. `./scripts/pre-tag-check.sh --container vX.Y.Z` runs them in `scripts/pre-tag-check.Dockerfile` with the GSD conformance oracle installed (`scripts/install-conformance-oracle.sh`). Use container mode before tagging. See `CLAUDE.md` Release Process.

**Scale:** About 3,400 `#[test]` attributes across `src/` and `tests/`. About 166 `#[cfg(test)]` modules in `src/`. About 56 integration test files under `tests/`.

## Test File Organization

**Unit tests (in-module):**
- Location: bottom of the source file, in a `#[cfg(test)] mod tests` block.
- Standard header inside the module:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| (*s).to_owned()).collect()
    }
    // ...
}
```

Source: `src/oneoff/report.rs` (line 234 onward).

- Heavily-tested modules include `src/state_reader/state_md.rs`, `src/state_reader/queue_md.rs`, `src/state_reader/roadmap_md.rs`, `src/state_reader/disk_status.rs`, `src/browser.rs`, `src/registry.rs`, `src/ui/roadmap_view.rs`, and the `src/oneoff/` modules.

**Integration tests (`tests/`):**
- Black-box tests against the library crate (`gsd_meta_manager::...`) and the compiled binary.
- File naming: `<subsystem>_<topic>.rs`. Prefixes include `agents_`, `driverctl_`, `driver_`, `launcher_`, `envelope_`, `oneoff_`, `registry_`, `state_reader_`, `prompt_`, `next_`, `bounded_output`, `editor_command`, `launch_target_cli`.
- Shared helpers live in `tests/common/mod.rs` (for example `git(dir, args) -> bool`) and `tests/launcher_support/mod.rs`. Include them with `mod common;` in each test crate. The module carries `#![allow(dead_code)]` because each integration target compiles it separately.

**Layout:**

```
src/<module>.rs              # implementation + #[cfg(test)] mod tests { ... }
tests/<subsystem>_<topic>.rs # integration test crate
tests/common/mod.rs          # shared git helper
tests/launcher_support/mod.rs
tests/fixtures/              # recorded transcripts, LOG.md, roadmaps, driverctl goldens
src/test_support.rs          # shared constants (compiled into the library, see below)
```

## Test Structure

**Naming:** Two styles coexist. Match the file you are editing.

- Descriptive `snake_case` sentences, dominant in newer code: `a_valid_report_validates`, `bad_headlines_name_the_flag` (`src/oneoff/report.rs`), `add_project_with_duplicate_alias_returns_error` (`tests/registry_test.rs`).
- `test_`-prefixed, common in `state_reader`: `test_parse_real_state_md`, `test_parse_queue_md_basic`.
- Some suites prefix by requirement or round ID: `r6_another_readers_shared_lock_does_not_fake_liveness` in `tests/driverctl_liveness.rs`. Keep the ID when extending the suite.

**Suite organization (in-module):**

```rust
#[test]
fn a_valid_report_validates() {
    let report = valid();
    assert_eq!(report.status, ReportStatus::Done);
    assert_eq!(report.changed.len(), 2);
    assert_eq!(report.schema, SCHEMA);
}
```

**Fixture builders:** A helper function named for the valid case builds the baseline; each negative test mutates one field. `fn valid() -> Report` in `src/oneoff/report.rs` is the example. Use this shape for validators.

**Test body rules:**
- Use `.unwrap()` in tests. Source-level `unwrap` is tolerated in `#[cfg(test)]` code.
- Assert the specific field or variant, not just `is_ok()`. For error variants, assert the message or the variant name.
- Keep inputs inline as string literals when the parser is text-based. Write files into a `TempDir` only when the on-disk layout matters.

**Setup and teardown:**
- Setup: `TempDir::new().unwrap()` (or `assert_fs::TempDir`) per test. The directory is removed on drop. No explicit teardown.
- Git fixtures: create throwaway repositories with the `git(dir, &[...])` helper in `tests/common/mod.rs`, which runs `git -C <dir>`. Agent and worktree tests (`tests/agents_*.rs`, `tests/registry_worktree_guard.rs`) use this.
- Environment overrides for timing tests are exported constants with `_ENV` suffixes, for example `GSDMM_ONEOFF_STOP_GRACE_SECS` and `GSDMM_ONEOFF_PAUSE_GRACE_SECS` in `src/oneoff/cancel.rs`. These shorten grace periods to tens of milliseconds. Set them in tests, not in production.

**Async tests:** `#[tokio::test]` (default current-thread runtime). Used in `src/watcher.rs` (3 tests) and across `tests/`. Watch out for blocking calls on the current-thread runtime (see the async guard below).

## Mocking

**Framework:** None. The project does not depend on `mockall`, `mockito`, `wiremock`, or similar crates. Substitution is done by injection, not by mock objects.

**Patterns:**
- **Injected clock.** Pure state machines take `now: DateTime<Utc>` or `Instant` as a parameter. The state machine in `src/oneoff/cancel.rs` is driven "with injected instants" so tests advance time without sleeping.
- **Injected paths and roots.** Functions take a base path (`claim_run_dir_at(project_dir, base)` in `src/oneoff/launch.rs:214`) so tests run inside a `TempDir`.
- **Real kernel primitives.** Lock tests use real `flock(2)` on real files and check contention across real file descriptors. `tests/driverctl_liveness.rs` explicitly says it tests against the kernel's `flock`, not a mock.
- **Recorded fixtures instead of live calls.** The stream-JSON parser is tested against recorded transcripts in `tests/fixtures/transcripts/` (see `tests/fixtures/transcripts/README.md`). Replay tests use `tests/fixtures/driverctl/replay/`.

**What to Mock (in this codebase's style):**
- The clock: pass `now` as a parameter.
- Filesystem roots: pass a path under `TempDir`.
- External agent output: use a recorded transcript or a golden file under `tests/fixtures/`.

**What NOT to Mock:**
- File locks: use real `flock` and real files.
- Git: use real throwaway repositories created with `tests/common/mod.rs::git`.
- The sandbox: sandbox tests need `bwrap` and are `#[ignore]`d where unavailable, rather than faked.
- The filesystem parser inputs: use real sample files (`tests/fixtures/roadmaps/`, `tests/fixtures/roadmap-shapes/`).

## Fixtures and Factories

**Test data locations:**
- `tests/fixtures/transcripts/*.ndjson`: recorded agent stream-JSON sessions covering success, budget exhaustion, interrupts, rate limits, and tool use. Each file is named by scenario.
- `tests/fixtures/roadmaps/` and `tests/fixtures/roadmap-shapes/`: real and synthetic `ROADMAP.md` and `STATE.md` for parser tests (`daily-vow`, `sentriq`, `ttbook`, `ttbook-phase13`, `v1-era`).
- `tests/fixtures/driverctl/`: direction files, golden `.md` outputs, replay logs, and spike-003 run artifacts. Golden files are regenerated only by the `#[ignore]` test noted below.
- `tests/fixtures/prompt/MASTER-PROMPT.no-gap-rules.md`: prompt variant for prompt-acceptance tests.

**Shared constants (`src/test_support.rs`):**
- `DEGENERATE: [&str; 10]`: payloads that carry no instruction (empty, whitespace, zero-width space, BOM, bidi override, soft hyphen, tag characters, variation selector). Every seam that accepts a free-text payload must reject all of them. Add new blank shapes here so they propagate to all seams.
- Its header is explicit that these literal payloads are necessary but not sufficient. The property is falsified by a sweep over all code points in `src/text.rs` tests that takes its oracle from an independent crate.
- The module ships in the library (not `#[cfg(test)]`) so integration tests can use it without copies.

**Golden-file tests:** Some tests compare generated output to a checked-in golden under `tests/fixtures/driverctl/golden/`. To regenerate, run the `#[ignore]`d regeneration test explicitly and review the diff.

## Coverage

**Requirements:** None enforced. There is no coverage tool, threshold, or CI coverage job. Do not add a coverage gate without an explicit decision.

**View coverage (optional, not configured in the repo):**

```bash
cargo install cargo-llvm-cov --locked
cargo llvm-cov --all-targets
```

## Guard Tests (Source-Tree Audits)

These integration tests read `src/` as text and fail on violations. They are the enforcement for conventions that comments alone cannot hold.

- `tests/async_blocking_guard.rs`: no named blocking call (`flock`, blocking `std::fs`, `std::process` wait) inside an `async fn` body without a hand-off to `spawn_blocking`. It is a lexical lint, not a proof.
- `tests/spawn_seam_guard.rs` (Unix only, `#![cfg(unix)]`): every process-spawn site in `src/` appears in an allowlist. Walks `src/` only, never `tests/`.
- `tests/oneoff_runtime_seam_guard.rs`: the one-off runtime seam, checked the same way.
- `tests/driver_router_conformance.rs`: fails by design when the GSD conformance oracle is absent. It is why bare `cargo test` can pass on a developer machine while CI fails. Install the oracle with `scripts/install-conformance-oracle.sh` before trusting a green run.

When you add a guarded rule, add a guard test in this style rather than a comment.

## Ignored Tests

Tests that need a live service, a sandbox tool, or real model spend are marked `#[ignore = "<reason>"]`. About 13 are ignored. Examples:

- `#[ignore = "paid live run: needs plan 29-08's consent.txt; spends real model budget"]`
- `#[ignore = "requires a live tmux server; CI has none"]` (`src/terminal_switch.rs:596`)
- `#[ignore = "needs bwrap; run with -- --ignored --nocapture"]`
- `#[ignore = "reads ~/projects/python/mailbot; run on demand"]` (`tests/agents_worktree_plans.rs`)
- `#[ignore = "reads a real file outside the repository; opt in with GSDMM_META_MANAGER_REAL_STATE_MD"]`
- `#[ignore = "regenerates the goldens; run explicitly"]`

Live egress and isolation tests are in `tests/launcher_live_*.rs` (`launcher_live_egress.rs`, `launcher_live_isolation.rs`, `launcher_live_managed.rs`, `launcher_live_scope.rs`, `launcher_live_token_reach.rs`). Run them only in a prepared environment.

Always give an `#[ignore]` a reason string. Do not add a bare `#[ignore]`.

## Test Types

**Unit tests:**
- Scope: parsers, validators, pure state machines, and record serialization round-trips.
- Approach: in-module `#[cfg(test)]`, inline inputs, `TempDir` only when the filesystem matters.
- Example: `src/state_reader/queue_md.rs` includes write round-trip tests.

**Integration tests:**
- Scope: subsystem behavior across module boundaries (launcher, driverctl, agents, registry, state reader).
- Approach: separate crates under `tests/`, real files, real git, real flock. Shared helpers in `tests/common/` and `tests/launcher_support/`.
- Examples: `tests/driverctl_store.rs`, `tests/launcher_sandbox.rs`, `tests/oneoff_e2e.rs`, `tests/driver_tab_launch_e2e.rs`.

**Guard tests:** See the section above. These run in the normal `cargo test` and are not ignored.

**End-to-end tests:**
- `tests/registry_test.rs` includes one test that invokes the compiled binary via `cargo run`.
- `tests/oneoff_e2e.rs`, `tests/driver_tab_launch_e2e.rs`, `tests/launcher_e2e.rs` run whole flows. Some need `bwrap` or tmux and are `#[ignore]`d.
- TUI rendering is tested with ratatui's `TestBackend` in `src/ui/screens/` (for example `src/ui/screens/normal.rs`, `src/ui/screens/driver_run.rs`) and in `src/ui/roadmap_view.rs` (for example `mockup_a_side_by_side_at_120_columns`). No screenshot or PTY harness is used in the default run.

## Common Patterns

**Async testing:**

```rust
#[tokio::test]
async fn watcher_reports_change() {
    // Arrange a TempDir, start the watcher, write a file, await the event.
}
```

Use `tokio::time::timeout` around awaited events so a missed event fails instead of hanging. On a current-thread runtime, do not block inside the async body. Use `spawn_blocking` for filesystem or lock calls, as the async guard requires.

**Error testing:**

```rust
#[test]
fn bad_headlines_name_the_flag() {
    for headline in ["", long.as_str(), "a\nb", "evil \u{202E}txt"] {
        let err = Report::validate(/* ... */).unwrap_err();
        // assert on the specific variant or message
    }
}
```

Loop over a set of bad inputs in one test when they share a failure mode. Assert the variant or the detail string. Do not assert only that an error occurred.

**Blank and adversarial input:** Pin any free-text seam with `crate::test_support::DEGENERATE`. Iterate the whole array so a new entry is checked everywhere:

```rust
for payload in DEGENERATE {
    assert!(seam(payload).is_err(), "accepted blank payload {payload:?}");
}
```

**Table-driven tests:** `tests/driver_router_table.rs` is the table-driven router suite. Follow its structure when adding cases.

**Test-only hooks in production code:** Use `#[cfg(test)]` for hooks that must not ship. Env overrides (`*_ENV` constants) are shipped and documented as tests-only; they clamp to a safe range (for example `1..=600` seconds).

## Adding a New Test

1. Pick the file. If the code has an in-module `tests` block, add there. For cross-module behavior, add or extend a `tests/<subsystem>_*.rs` file.
2. Name it as a sentence in snake_case that states the expectation, matching nearby tests.
3. Build the baseline with a helper (`valid()`-style), then mutate one field per negative test.
4. Use `TempDir` for any filesystem path. Use real git and real flock where relevant.
5. If the test needs a service, a tool, or paid spend, add `#[ignore = "reason"]`.
6. If you are guarding a rule with a comment, add a source-tree guard test instead.
7. Run `cargo test --no-fail-fast` and `cargo clippy --all-targets -- -D warnings` before committing.

---

*Testing analysis: 2026-11-03*
