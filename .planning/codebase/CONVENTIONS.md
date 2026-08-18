# Coding Conventions

**Analysis Date:** 2026-08-18

## Naming Patterns

**Files:**
- Snake_case modules named for the concept they own: `src/config.rs`, `src/registry.rs`, `src/session_detector.rs`, `src/terminal_switch.rs`.
- Feature areas are directories with a `mod.rs` plus siblings: `src/driver/{mod,dry_run,kill,liveness,lock,reconcile,run,spawn}.rs`, `src/journal/{mod,inbox,reader,redact,writer}.rs`, `src/executor/`, `src/state_reader/`, `src/ui/screens/`.
- Test files under `tests/` are named for the property they guard, not the module under test: `tests/spawn_seam_guard.rs` (not `executor_tests.rs`), `tests/driver_dry_run.rs`, `tests/registry_test.rs`.

**Functions:**
- Free functions, snake_case, verb-first: `add_project`, `record_opt_in`, `clear_opt_in`, `git_last_commit_time`, `project_last_activity` (`src/registry.rs`, `src/state_reader/git_ops.rs`).
- Private helper functions are un-prefixed and colocated above/below their public callers in the same file rather than pulled into a separate `internal` module — see `git_read_raw` / `git_read` / `config_get` in `src/state_reader/git_ops.rs:189-229`.
- A dedicated escape-hatch constructor is named to be self-incriminating rather than terse: `for_testing_bypassing_opt_in` in `src/executor/mod.rs`, not `test_new` or `unchecked_new` — the name itself is part of the audit surface (see `tests/spawn_seam_guard.rs:25-27`).

**Variables:**
- Short, scope-local names (`tmp`, `fm`, `cmd`) in tests; descriptive names in production code, especially around security-sensitive values (`digest`, `redacted`, `allowlist`).

**Types:**
- PascalCase structs/enums, one clear owner per concept: `Config`, `RegisteredProject`, `DriverOptIn`, `Preferences` (`src/config.rs`).
- Domain records over booleans when the boolean would be ambiguous. `DriverOptIn` is a struct with `opted_in_at` + `claude_md_digest`, not a `bool`, specifically because "may be driven" must not be conflatable with "was registered" (`src/config.rs:66-77`, decision D-14).
- Newtypes are used to make an invariant a *type-level* property rather than a discipline. `RedactedLine` in `src/journal/redact.rs` has a private field, one crate-private accessor, and implements **no traits at all** (no `From`, `Deref`, `Display`, or test-only constructor) so the only way to produce one is through the redactor (`src/journal/redact.rs:7-12`, decision D-22).
- Capability types gate dangerous operations: `DrivableProject` (`src/executor/mod.rs`) has private fields and exactly two constructors — one production (`from_registry`), one explicit test escape hatch — enforced by `tests/spawn_seam_guard.rs::drivable_project_has_exactly_two_constructors_and_private_fields`.

## Code Style

**Formatting:**
- Standard `rustfmt` defaults (no `rustfmt.toml` found in repo root — verify before assuming custom width/style rules).

**Linting:**
- Clippy is treated as a hard gate. `CONTRIBUTING.md` states: "Clippy warnings are treated as errors in this project."
- **Two different clippy invocations, two different results — know which one you're running:**
  - `cargo clippy -- -D warnings` (lib target only) is the documented pre-PR gate in `CONTRIBUTING.md` and passes clean.
  - `cargo clippy --all-targets -- -D warnings` (includes `tests/`, in-source `#[cfg(test)]` modules, benches) currently has **5 known pre-existing lint errors** as of 2026-08-18, all in test code:
    1. `src/browser.rs:131-133` — `clippy::bool_assert_comparison` (`assert_eq!(x, true)` instead of `assert!(x)`), x3 occurrences.
    2. `src/project_creator.rs:146` — `clippy::cmp_owned` (`PathBuf::from("~")` allocated just for comparison).
    3. `src/state_reader/mod.rs:258` — `clippy::items_after_test_module` (items declared after the `mod tests` block, including `count_backlog_items` at line 477).
  - Do not treat a clean `cargo clippy -- -D warnings` as proof `--all-targets` is also clean — they check different target sets.

## Import Organization

**Order:** No enforced blank-line grouping observed; imports are effectively alphabetized within a flat `use` block, `crate::` paths and external crates interleaved alphabetically rather than segregated into std/external/internal groups — see `src/app.rs:1-13` (`crate::action`, `crate::change_tracker`, `crate::config`, ... `crossterm::event`, `ratatui::widgets`, `std::collections`, `std::path`, `tokio::sync::mpsc`, all in one alphabetized list).

**Path Aliases:** None (no `#[path]` remapping or workspace aliasing found). Modules are referenced by their full `crate::module::item` path.

## Error Handling

**Pattern:** Hand-written error enums with manual `Display` and `std::error::Error` impls — **not** `thiserror` (`thiserror` is not a dependency; verified against `Cargo.toml`). See `src/error.rs`: `SpawnError`, `SendError`, `CapabilityError`, `OptInError`, `LockError`, `DriveError` each get their own `impl std::error::Error`.

**Exhaustive `match`, no wildcard, by design.** `DriveError::source()` in `src/error.rs:530-547` spells out every variant instead of using `_ => None`, with the rationale written directly above the match:
> "Exhaustive rather than `_ => None`, since plan 17-08. The wildcard meant a variant added later that DID wrap an error would silently lose its source... Spelling every arm out makes the next author decide, which is the only moment the decision is cheap."
This is a deliberate house pattern: wherever a `match` decides something security- or correctness-relevant, prefer listing every arm (even ones with identical bodies, joined with `|`) over `_ =>`, so an upstream variant addition becomes a **compile error** instead of a silent fallthrough. Applied again in `src/driver/reconcile.rs:284-286` (`RunVerdict` matched exhaustively).

**Error messages are written for the user acting on them, not just the maintainer.** `DriveError::RunIdRequired`'s `Display` impl explains what to do about it ("Pass one, or use `--dry-run`, which creates no run to identify") with the in-code comment: "a refusal a caller cannot act on is a bug report rather than an error message" (`src/error.rs:507-513`).

**Fallible-by-signature validation.** Functions that might not have an answer return `Option`/`Result` rather than a sentinel value, specifically to conscript the compiler into enumerating every call site when the fallible case changes. Example: `src/state_reader/git_ops.rs` — `git_last_commit_time`, `head_sha`, `is_dirty`, `config_get` all return `Option<T>`; `src/registry.rs` — `add_project`, `record_opt_in`, `clear_opt_in`, `remove_project` all return `Result<()>`, and `claude_md_digest` returns `Option<String>` so a project without a `CLAUDE.md` is a normal, compiler-checked case rather than a panic path.

**`anyhow::Context` for propagation, not for domain errors.** `src/config.rs` uses `anyhow::Context` for I/O-adjacent failures (file read/write, JSON parse) while domain-significant failures (drive refusals, lock conflicts, spawn failures) get their own typed enums in `src/error.rs`. Use `anyhow` at the outer I/O boundary; use a typed enum when a caller needs to distinguish cases (e.g., match on `DriveError::RunIdRequired` vs `DriveError::Lock(..)`).

## Comments — Rationale-in-Code

**This is the codebase's most distinctive convention: non-obvious decisions carry a comment naming the decision id, the failure it prevents, and the alternative that was declined.** Decision ids (`D-14`, `D-15`, `D-16`, `D-17`, `D-18`, `D-21`..`D-28`, `D-30`, `SAFE-04`, `WR-16`, `CTRL-03`) and pitfall references (`PITFALLS:329`, `PITFALLS:521`, `PITFALLS:63`, `PITFALLS:69`) appear directly above the code they justify, not only in `.planning/` docs. Examples:

- `src/config.rs:9-21` — `CONFIG_SCHEMA_VERSION` doc comment explains what bumping to `2` means, why `Config::version` deliberately has **no** `#[serde(default)]` (a config lacking it should still fail to parse — D-15), and why relaxing that now would be "an unrelated behaviour change smuggled in under a migration."
- `src/config.rs:66-77` — `DriverOptIn` is a record and not a `bool`, with three enumerated reasons, one of which names a *future* need (Phase 21 re-confirmation) as the reason it's a record now rather than later.
- `src/config.rs:109-117` — `Preferences` explicitly does **not** derive `Default`, because a derived `Default` would yield `driver_max_concurrent: 0`, silently denying all runs. A named two-armed test (`driver_max_concurrent_defaults_to_one_from_default_and_from_empty_json`) exists specifically to fail if someone re-derives it.
- `src/journal/redact.rs:1-56` — module-level doc explains *where* redaction must happen (capture path, never UI), *why* the newtype has no trait impls, and states an explicit non-goal section titled "What this module will not grow (D-26)": no raw sidecar file, no unredact path, no verbose bypass mode.
- `src/driver/dry_run.rs:1-40` — cites `PITFALLS:63` and `PITFALLS:69` by exact quoted failure mode ("a dry-run that prints 'would run: /gsd:execute-phase' tells the user nothing about blast radius") as the reason the module exists, then names three required outputs (D-22) each with its own honesty constraint.

**Convention for new code:** when a design choice is not the obvious/simplest option (skipping `#[serde(default)]`, choosing a record over a bool, omitting a trait impl, using `--no-optional-locks` instead of a straightforward git call), write the *why* and the *declined alternative* directly above it, tagged with a decision id if one exists in `.planning/`. A future reader (including an LLM) should not have to open `.planning/` to understand why the code looks the way it does.

**"A comment is not a guard; the test is."** This project treats prose as advisory only — every invariant stated in a doc comment that matters for correctness or safety has a matching integration test that enforces it mechanically. See TESTING.md for the mechanism (`tests/spawn_seam_guard.rs`).

**Pinned string constants + an ordering/contract test.** User-facing text that other code or tests depend on verbatim is defined as a named `const` with a doc comment marking it a "pinned contract," and a test asserts the constants appear, in order, in rendered output. See `src/driver/dry_run.rs:45-66` (`SECTION_COMMANDS`, `SECTION_DIFFSTAT`, `SECTION_REFSPECS`) verified by `tests/driver_dry_run.rs::the_dry_run_output_names_the_command_the_diffstat_and_the_refspecs`. Treat changing pinned-contract text as a breaking, user-visible change — update the paired test in the same commit.

**JSDoc/TSDoc equivalent:** Rust doc comments (`///` for items, `//!` for module-level) are used extensively and are expected to carry design rationale, not just a one-line description. Module-level `//!` blocks in `src/journal/redact.rs`, `src/driver/dry_run.rs`, and `src/config.rs` read as short design documents.

## Function Design

**Size:** No hard line-count rule observed, but files that own a cohesive concern grow large rather than being split prematurely — `src/app.rs` is 187KB, `src/driver/run.rs` is 97.8KB, `src/journal/mod.rs` is 96.6KB. Splitting happens along concern boundaries (`src/driver/{dry_run,kill,liveness,lock,reconcile,run,spawn}.rs`), not by arbitrary size.

**Parameters:** Capability/context types are passed by reference where the function needs authority to act (`project: &DrivableProject`) rather than a bare path — this is itself an audited invariant (`tests/spawn_seam_guard.rs:418-426` asserts the agent spawn seam's signature still takes `&DrivableProject`, never a raw path).

**Return Values:** Prefer `Option<T>` / `Result<T, E>` over sentinel values or panics for anything that can legitimately fail or be absent (see Error Handling above).

## Module Design

**Exports:** Public API surface is deliberately narrow around dangerous operations — `DrivableProject`'s fields are all private, verified by an integration test that scans the struct body for `pub` fields and fails if any exist (`tests/spawn_seam_guard.rs:366-383`).

**Serde migration posture:** Config/registry structs that persist to disk and may be read/written across binary versions use two techniques together, applied consistently:
1. `#[serde(default)]` on any field added after the initial schema, so an older on-disk file still loads. Example: `RegisteredProject::driver_opt_in` (`src/config.rs:46-47`).
2. `#[serde(flatten)] pub extra: Map<String, Value>` to preserve fields a *newer* binary wrote that *this* binary doesn't model, so an older binary loading and re-saving a newer file doesn't silently delete unknown fields. Example: `RegisteredProject::extra` (`src/config.rs:48-64`) and `Preferences::extra` (`src/config.rs:141-146`), both explicitly cross-referencing each other and `JournalRecord.rest` (`src/journal/reader.rs:196-211`) as "the same technique, same reason, same shared-file hazard."
3. The inverse attribute, strict unknown-field rejection (`#[serde(deny_unknown_fields)]`), is **forbidden everywhere under `src/`** — mechanically enforced by `tests/spawn_seam_guard.rs::no_executable_line_in_src_opts_into_strict_unknown_field_rejection`, because every parsed record may have come from an untrusted agent's output or a future GSD phase's new record kind, and rejecting unknown fields would turn forward-compatibility into a hard parse failure (D-30).
4. Old-config-format fixtures in tests are byte literals (raw JSON strings), not constructed via the current struct, so a fixture can't silently drift to match a schema change it's supposed to be testing against. Follow this pattern for any new test asserting cross-version load behavior.

**Barrel Files:** `mod.rs` files re-export the public surface of their directory (`src/driver/mod.rs`, `src/journal/mod.rs`, `src/executor/mod.rs`, `src/state_reader/mod.rs`) rather than requiring callers to reach into submodules directly.

---

*Convention analysis: 2026-08-18*
