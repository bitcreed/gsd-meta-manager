# Testing Patterns

**Analysis Date:** 2026-08-18

## Test Framework

**Runner:**
- Rust's built-in `#[test]` / `cargo test` — no `nextest`/custom runner config detected in the repo despite `CLAUDE.md`'s stack recommendations mentioning `cargo-nextest` as a dev tool; actual usage is plain `cargo test`.
- No `[dev-dependencies]` test framework beyond `tempfile` (used pervasively for filesystem fixtures) — verify current `Cargo.toml` `[dev-dependencies]` before assuming more.

**Assertion Library:** Standard `assert!`, `assert_eq!`, `assert_ne!` macros. No `pretty_assertions` or similar detected.

**Run Commands:**
```bash
cargo build && cargo test && cargo clippy -- -D warnings   # documented pre-PR gate (CONTRIBUTING.md), passes clean
cargo test                                                  # all unit + integration tests
cargo test <substring>                                      # filter by test name
```

**IMPORTANT — the `rtk` CLI wrapper filters build/test output.** This project uses `rtk` (Rust Token Killer) as a hook-rewritten proxy for shell commands, including `cargo build`/`cargo test`/`cargo clippy`. `rtk` strips `warning:` lines and `test result:` lines from cargo's output to save tokens. Consequences:
- **A `grep` for `warning:` or `test result:` on `rtk`-filtered output passes vacuously** — the lines were already removed before you searched, not because there were none.
- To see raw, unfiltered cargo output (e.g., to confirm the actual clippy warning count, or to see `test result: ok. N passed`), you must bypass the filter explicitly: `rtk proxy cargo test`, `rtk proxy cargo clippy --all-targets -- -D warnings`, `rtk proxy cargo build`.
- This document's clippy findings below were captured via `rtk proxy cargo clippy --all-targets -- -D warnings`, not via the filtered wrapper.

## Build/Test/Lint Gate

Two different commands, two different results — know which one is being asserted:

| Command | Result (2026-08-18) |
|---|---|
| `cargo build && cargo test && cargo clippy -- -D warnings` | **Passes clean.** This is the gate documented in `CONTRIBUTING.md` and expected of every PR. |
| `cargo clippy --all-targets -- -D warnings` | **5 known pre-existing lint errors**, all confined to test code (see below). Not part of the documented gate; treat as a known baseline, not a regression, unless the count or location changes. |

**The 5 pre-existing `--all-targets` lints (verified via `rtk proxy cargo clippy --all-targets -- -D warnings`):**
1. `src/browser.rs:131` — `clippy::bool_assert_comparison`: `assert_eq!(entries[0].is_dir, true)`
2. `src/browser.rs:132` — same lint: `assert_eq!(entries[1].is_dir, true)`
3. `src/browser.rs:133` — same lint: `assert_eq!(entries[2].is_dir, false)`
4. `src/project_creator.rs:146` — `clippy::cmp_owned`: `PathBuf::from("~")` allocated only to compare
5. `src/state_reader/mod.rs:258` (flagging `count_backlog_items` at line 477) — `clippy::items_after_test_module`: production items declared after the `mod tests` block

If asked to "fix all clippy warnings," confirm with the user whether `--all-targets` fixes are in scope — they touch test-only code paths, not the documented gate.

## Test File Organization

**Two tiers, used for different purposes — this is a deliberate split, not inconsistency:**

1. **In-source `#[cfg(test)] mod tests` blocks** — the default for testing a module's own logic. Present in the majority of `src/` files (36 files have exactly one `#[cfg(test)]` block; `src/executor/mod.rs` and `src/journal/writer.rs` have more than one). Convention: `mod tests { use super::*; ... }` placed at the bottom of the file, e.g. `src/journal/mod.rs:1356-1358`.
2. **Integration tests under `tests/*.rs`** (16 files) — reserved for tests that need to observe the crate from outside (`use gsd_meta_manager::...`), spawn real subprocesses, or — critically — **walk and audit the source tree itself**. `tests/spawn_seam_guard.rs` explains this split explicitly: "It is an integration test rather than an in-source one because it reads the source tree, and a test that walks `src/` has no business living inside it" (line 10-11).

**Naming:** Integration test files are named for the *property* or *subsystem* being guarded (`driver_dry_run.rs`, `driver_lock.rs`, `journal_gitignore.rs`, `spawn_seam_guard.rs`), not for a 1:1 mirror of a `src/` filename. Individual test functions are named as full sentences describing the invariant, e.g. `the_escape_hatch_has_no_call_site_in_src`, `drivable_project_has_exactly_two_constructors_and_private_fields`, `an_override_field_declared_without_the_debug_gate_is_reported` (`tests/spawn_seam_guard.rs`).

**Test counts per integration file** (via `grep -c '#\[test\]' tests/*.rs`): `state_reader_test.rs` (25), `registry_test.rs` (13), `spawn_seam_guard.rs` (7), `journal_gitignore.rs` / `journal_crash.rs` (3 each), `driver_dry_run.rs` / `journal_run_paths.rs` (1 each). Several driver/executor integration files (`driver_kill.rs`, `driver_lock.rs`, `driver_optin.rs`, `driver_reattach.rs`, `driver_tracer.rs`, `executor_lifecycle.rs`, `executor_transport.rs`, `driver_inbox.rs`, `driver_kill_startup.rs`) define `#[test]` via macros/attributes not matched by a literal grep for `#[test]` — verify test count directly with `cargo test` output rather than `grep` for these files.

## House Mechanism: "A comment is not a guard; the test is."

**This is the single most important testing convention in this codebase.** Stated verbatim in `tests/spawn_seam_guard.rs:4-8`:

> "One rule governs every assertion below: a comment is not a guard; the test is. `src/executor/mod.rs` says the opt-in escape hatch has no production call site, and `src/journal/` has claimed since Phase 16 that the strict unknown-field attribute is kept out by a grep. Prose cannot enforce either. This file does."

Whenever a doc comment in `src/` asserts an invariant like "exactly one call site," "no bypass flag reaches release," or "no field opts into strict rejection," there is (or should be) a companion integration test in `tests/` that **mechanically re-derives the invariant by scanning the source tree**, rather than trusting the comment to stay true. `tests/spawn_seam_guard.rs` is the canonical example and covers four such properties in one file:

1. **Single call site for a named escape hatch.** `the_escape_hatch_has_no_call_site_in_src` greps every non-comment line under `src/` for the literal `for_testing_bypassing_opt_in` and asserts it appears exactly once, in `src/executor/mod.rs`, as a `pub fn` definition — not a call.
2. **Capability-type shape.** `drivable_project_has_exactly_two_constructors_and_private_fields` parses the `DrivableProject` struct body out of the source text and asserts every field line lacks `pub`.
3. **Spawn-site allowlist.** `every_process_spawn_site_in_src_is_on_the_allowlist` scans for three literal call shapes (`Command::new(`, `CommandWrap::with_new(`, `process_group(`) with a hand-rolled word-boundary matcher (`calls_marker`, lines 90-104) and diffs the observed set against a declared `SPAWN_ALLOWLIST` constant — failing in **both directions**: an unlisted spawn site is a violation, and a listed site that no longer spawns anything is *also* a violation ("the allowlist is now wider than the truth it describes").
4. **Debug-only gating of dangerous override fields.** `the_agent_program_override_fields_are_debug_only` scans for field declarations (`claude_program:`, `claude_args:`) and asserts a `#[cfg(debug_assertions)]`-shaped attribute (not `any(...)`, not `not(...)`) appears within 3 lines above each one, so a release build can never accept the override.
5. **Forbidden attribute.** `no_executable_line_in_src_opts_into_strict_unknown_field_rejection` scans for `deny_unknown_fields` (assembled at runtime from two string halves specifically so this file's own source doesn't match its own grep — lines 189-197) and asserts zero hits under `src/`.

**Guard-of-the-guard pattern.** Every source-scanning assertion above has (or should have) a paired "control arm" test that proves the matcher itself works — both that it *fires* on a synthetic bad example and that it *doesn't* fire on a synthetic good one. See `a_signal_to_a_process_group_is_not_mistaken_for_a_spawn` (tests the word-boundary logic against both `kill_process_group(` false positives and real `Command::new(`/`process_group(0)` true positives) and `an_override_field_declared_without_the_debug_gate_is_reported` (tests `override_declarations` against synthetic gated/ungated snippets, not tree-scraped ones, specifically so the control arm "keeps proving the matcher works even once — especially once — the tree is correct").

**When adding a new invariant of this shape** (an "exactly N," "never," or "always gated" property about the source tree): write it as a source-scanning integration test following the pattern in `spawn_seam_guard.rs`, including a non-vacuity check (assert the scan found *something* to examine, not just that violations are empty) and a control-arm test proving the scanner itself is correct on synthetic input.

## Pinned-Contract Tests

User-facing string constants that other tests or scripts depend on verbatim get a dedicated test asserting their exact presence and order in rendered output, rather than relying on the constant definition alone. Example: `src/driver/dry_run.rs` defines `SECTION_COMMANDS`, `SECTION_DIFFSTAT`, `SECTION_REFSPECS` as "pinned contract" constants; `tests/driver_dry_run.rs::the_dry_run_output_names_the_command_the_diffstat_and_the_refspecs` asserts all three appear in that order. Follow this pattern for any new user-facing text block another test or downstream tool parses.

## Fixtures

**Filesystem fixtures via `tempfile::TempDir`, not mocks.** Tests that touch the filesystem create a real temp directory and write real files into it, then call the production parsing/reading functions against that directory — see `tests/state_reader_test.rs:155-294` (`TempDir::new().unwrap()` used repeatedly) and elsewhere across `driver_*`/`journal_*` integration tests.

**Old-schema config fixtures are byte/string literals of the raw format, not built via current structs.** When testing that an old `config.json` still loads under a new schema, the fixture is written as a literal JSON string representing the *old* shape, so the test can't silently drift to match whatever the current struct produces. Cross-reference: `RegisteredProject::extra` doc comment in `src/config.rs:48-64` names the test `a_config_written_by_a_newer_binary_keeps_its_unknown_fields_and_its_version` as the place this is asserted rather than assumed.

**No fixture directory convention observed** (no `tests/fixtures/` directory) — fixtures are inlined as string literals in the test file that uses them.

## Mocking

**No mocking framework is used** (`mockall`, etc. not present in `Cargo.toml`). The project's convention is to test against real filesystem state (via `TempDir`) and real subprocess behavior rather than mocking I/O boundaries:

- `src/driver/kill.rs` and `src/driver/liveness.rs` spawn a real `sleep` child process in their in-source tests specifically so "already gone" and "pid reuse" code paths can be pointed at a real, reaped PID rather than an arbitrary number chosen to be implausible — see the rationale comments cited in `SPAWN_ALLOWLIST` (`tests/spawn_seam_guard.rs:43-50`).
- `tests/driver_dry_run.rs` proves zero-git-writes/zero-agent-spawns not via a mock but by capturing `git reflog`, every ref, and the full `.git` directory listing before and after, then diffing — plus a "tripwire program" supplied as the agent that leaves an evidence file on disk if it is ever actually executed (`src/driver/dry_run.rs:31-36`).

**What to mock:** Nothing, by convention — prefer real filesystem/subprocess fixtures scoped to a `TempDir` or a real spawned throwaway process.

**What NOT to mock:** Git operations, process spawning, and filesystem reads/writes are all tested against real instances rather than trait-object mocks.

## Idempotence as a Test Class

`src/journal/redact.rs` establishes idempotence checking as a deliberate test category, not an incidental assertion: "`redact(redact(x)) == redact(x)` over the whole corpus is the cheapest possible detector for 'a replacement literal is itself redactable' and for 'rule A ate rule B's output.' It caught three real defects in one session that review had not" (lines 24-27). Apply this pattern to any transformation function with a fixed-point property (redaction, normalization, sanitization) — round-trip it over a representative corpus and assert stability.

## Coverage

**Requirements:** No coverage tool or enforced threshold detected (no `cargo-tarpaulin`/`grcov` config, no `codecov.yml`). Coverage is driven by convention (every module gets in-source tests; every load-bearing invariant gets an integration test) rather than a measured percentage gate.

## Test Types

**Unit tests:** In-source `#[cfg(test)] mod tests` per module, testing that module's own logic in isolation with real (not mocked) collaborators where I/O is involved.

**Integration tests:** `tests/*.rs`, used for (a) cross-module behavior needing the crate's public API, (b) subprocess/real-OS behavior (kill, liveness, lock, spawn), and (c) source-tree audits enforcing invariants that must hold across the whole `src/` directory (see house mechanism above).

**E2E tests:** Not used — no browser/TUI-driver E2E harness detected.

## Common Patterns

**Non-vacuity assertions.** Several audit tests assert not just "no violations found" but "something was actually examined," to distinguish a real pass from a scanner that silently checks nothing (e.g. after a rename). See `source_files()` in `tests/spawn_seam_guard.rs:210-214` (`assert!(!out.is_empty(), ...)`) and `the_agent_program_override_fields_are_debug_only`'s `found_in_parser >= AGENT_OVERRIDE_FIELDS.len()` check (lines 514-521).

**Word-boundary string matching over naive `contains`.** When scanning source text for a marker that could appear as a substring of an unrelated identifier, do not use a plain `.contains()`. `calls_marker` (`tests/spawn_seam_guard.rs:90-104`) checks the character immediately preceding a match to reject cases where the marker continues a longer identifier (e.g. `process_group(` inside `kill_process_group(`), with an explicit regression story in the doc comment about how a naive `contains` once caused a false positive that would have been "fixed" by widening an allowlist instead of narrowing the matcher.

---

*Testing analysis: 2026-08-18*
