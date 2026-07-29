---
phase: 17-supervisor-detach-kill-switch-dry-run-opt-in-gate
plan: 03
subsystem: infra
tags: [config, registry, migration, opt-in, schema-version, forward-compat, isolation]

requires:
  - phase: 17-supervisor-detach-kill-switch-dry-run-opt-in-gate
    provides: "DriverOptIn, RegisteredProject.driver_opt_in, DrivableProject::from_registry, driver::drive, tests/fixtures/fake-claude-cwd.sh (plan 17-01)"
  - phase: 16-run-journal-state-substrate
    provides: "journal::argv_digest — the non-cryptographic identity digest reused for the CLAUDE.md fingerprint"
provides:
  - "`config::CONFIG_SCHEMA_VERSION` — schema version 2, meaning 'a registry entry may carry a DriverOptIn record'"
  - "`config::migrate` — the read-side migration whose entire content is 'change nothing', plus the never-downgrade rule for a newer binary's config"
  - "`RegisteredProject.extra` / `Preferences.extra` — flattened unknown-field carriers, so a downgrade preserves rather than deletes"
  - "`Preferences.driver_max_concurrent` (default 1) — the counting seam Phase 20 inherits as a working cap"
  - "`registry::record_opt_in` — the ONLY non-test construction site of a DriverOptIn"
  - "`registry::clear_opt_in`, `registry::is_opted_in` — withdrawal and a cheap UI read"
  - "tests/driver_optin.rs — ROADMAP success criterion #4's cross-project isolation proof"
affects: [17-05 spawn-seam concurrency enforcement, 17-07 UI opt-in toggle, 20 global cap policy, 21 CLAUDE.md drift re-confirmation]

tech-stack:
  added: []
  patterns:
    - "Read-side migration + write-side one-shot: migrate() sets the version in memory and the next ordinary save stamps the file, with no destructive rewrite of a file an older binary might still be reading"
    - "Flattened unknown-field carriers on a user-owned config file shared by two binary versions (the JournalRecord.rest technique applied to config.json)"
    - "Hand-written Default where a derived one would produce a semantically catastrophic zero, with a two-armed test that fails on each half of the trap independently"
    - "Whole-tree fingerprint (path + length + content digest, .git included, mtime deliberately excluded) as a before/after safety assertion"

key-files:
  created:
    - tests/driver_optin.rs
  modified:
    - src/config.rs
    - src/registry.rs
    - src/app.rs
    - src/executor/mod.rs
    - tests/registry_test.rs
    - tests/driver_tracer.rs

key-decisions:
  - "The acceptance criterion's `grep 'DriverOptIn {' src/ | wc -l == 1` is unsatisfiable as literally written — the pattern also matches the struct definition and one construction inside a `#[cfg(test)] mod tests`. The intended property (exactly one non-test construction site) was verified with a test-module-truncating pass instead"
  - "Two pre-existing `registry_test.rs` assertions pinned `version == 1`; both now assert against `CONFIG_SCHEMA_VERSION` rather than a literal, so a future bump needs no test edit"
  - "The `extra` field broke three more struct literals than the plan anticipated (two test helpers under src/, one under tests/); each was written out explicitly rather than absorbed by a struct-update shorthand"
  - "The D-21 `worktrees` assertion is anchored at the test's temp root rather than run over the absolute path, so it is about the driver's choice and not about the host's TMPDIR"
  - "The isolation test's fingerprint comparison is pinned against vacuity: an empty walk would compare equal to itself, so the test asserts the walk saw both `.planning/STATE.md` and at least one `.git/` entry"

patterns-established:
  - "A migration whose content is 'change nothing' still needs a byte-for-byte literal fixture — a round-trip through the current struct tests serialisation, not migration"
  - "A safety assertion that compares two collected states must first prove the collection is non-empty"

requirements-completed: []

coverage:
  - id: M1
    description: "A config.json written by a pre-Phase-17 binary loads byte-for-byte unchanged and every project in it comes back not-opted-in"
    requirement: "CTRL-03"
    verification:
      - kind: unit
        ref: "src/config.rs#a_pre_phase_17_config_loads_with_every_project_not_opted_in"
        status: pass
    human_judgment: false
  - id: M2
    description: "The migration is idempotent — running it on an already-migrated config changes nothing"
    verification:
      - kind: unit
        ref: "src/config.rs#the_migration_is_idempotent"
        status: pass
    human_judgment: false
  - id: M3
    description: "A config written by a newer binary keeps its unknown fields and its recorded version through a load-and-save by this build"
    verification:
      - kind: unit
        ref: "src/config.rs#a_config_written_by_a_newer_binary_keeps_its_unknown_fields_and_its_version"
        status: pass
    human_judgment: false
  - id: M4
    description: "Preferences.driver_max_concurrent defaults to 1 from both Default and an empty JSON object — the derived-Default zero trap does not fire"
    verification:
      - kind: unit
        ref: "src/config.rs#driver_max_concurrent_defaults_to_one_from_default_and_from_empty_json"
        status: pass
    human_judgment: false
  - id: M5
    description: "The discovery path that auto-registers projects from active Claude sessions can never set an opt-in record"
    requirement: "CTRL-03"
    verification:
      - kind: unit
        ref: "src/registry.rs#auto_registration_leaves_every_discovered_project_not_opted_in"
        status: pass
      - kind: manual_procedural
        ref: "exactly one non-test, non-definition `DriverOptIn {` construction site under src/, verified by a test-module-truncating grep"
        status: pass
    human_judgment: false
  - id: M6
    description: "The opt-in record is constructed in exactly one place, stamps a second-precision RFC3339 timestamp, records a CLAUDE.md digest, and round-trips through save and load"
    requirement: "CTRL-03"
    verification:
      - kind: unit
        ref: "src/registry.rs#record_opt_in_stamps_a_record_with_a_second_precision_timestamp"
        status: pass
      - kind: unit
        ref: "src/registry.rs#clear_opt_in_removes_the_record_entirely"
        status: pass
      - kind: integration
        ref: "tests/registry_test.rs#a_round_trip_through_save_and_load_preserves_an_opt_in_record"
        status: pass
    human_judgment: false
  - id: M7
    description: "A live run against one project performs no write, no spawn and no git operation against any other registered project"
    requirement: "CTRL-03"
    verification:
      - kind: integration
        ref: "tests/driver_optin.rs#a_run_against_one_project_leaves_every_other_project_byte_identical"
        status: pass
      - kind: integration
        ref: "tests/driver_optin.rs#the_driven_agents_working_directory_is_the_driven_projects_root"
        status: pass
      - kind: integration
        ref: "tests/driver_optin.rs#a_refused_drive_writes_nothing_under_either_project"
        status: pass
    human_judgment: false

duration: 13 min
completed: 2026-07-29
status: complete
---

# Phase 17 Plan 03: Migration, the Single Construction Site, and Criterion #4 Summary

**An upgrade cannot silently enrol a project into being driven — proved against a byte-for-byte pre-Phase-17 `config.json`, not against a round-trip — and a live run against one project leaves every other registered project byte-identical down to its `.git`.**

## Performance

- **Duration:** 13 min
- **Started:** 2026-07-29T17:05:00Z
- **Completed:** 2026-07-29T17:18:09Z
- **Tasks:** 2
- **Files modified:** 7 (1 created, 6 modified)

## Accomplishments

- **The migration is now a proof rather than an assumption.** `#[serde(default)]` makes an old `config.json` *load*; it does not demonstrate that every project comes back not-opted-in. `a_pre_phase_17_config_loads_with_every_project_not_opted_in` builds its input from a literal `const &str` containing `"version": 1` — deliberately not by serialising a `Config`, which would test the round-trip through today's struct and say nothing about the upgrade path. FEATURES:236 names silent enrollment as the one failure that is *"invisible until the agent has already committed"*, and this is the assertion that would catch it.
- **A downgrade now preserves instead of deleting.** `config.json` is a file two binary versions may share. Without the flattened `extra` carriers, a v1.6 binary loading a v2.1 config and saving it would silently drop every field it had never heard of — a destructive rewrite by omission. The same tolerance technique `JournalRecord.rest` already uses, applied one layer up. `migrate` also refuses to downgrade a higher recorded version, so a newer binary is never told its own migration had already run.
- **`DriverOptIn` has exactly one non-test construction site.** A record that can be built anywhere is a record that can be built by accident; because `registry::record_opt_in` is the only place one comes into existence, a `Some(record)` in a config is proof of a deliberate user action. `auto_register_from_sessions` — the silent-enrollment hazard site FEATURES:236 names — reaches an explicit `driver_opt_in: None` that the compiler demanded, and a test asserts every discovered project comes back not opted in.
- **The `driver_max_concurrent` zero trap is closed from both directions.** A derived `Default` yields `usize::default()`, which is `0`, which means "no run may ever start" — a total denial of service dressed as a default. `Default` is hand-written and the serde attribute names a function, and the test has two arms because there are two independent ways to reach the zero.
- **ROADMAP success criterion #4 is met on disk in both directions.** A real drive through the production `driver::drive` entry leaves project B byte-identical including its `.git`, its working tree still clean by an independent `git status --porcelain` check, and the driven agent's working directory is A's root — not under B, and not inside any worktree.

## Task Commits

1. **Task 1: Schema version 2, a read path that cannot enrol, and the single construction site** — `57ccf17` (feat)
2. **Task 2: Prove a live run touches exactly one project** — `470cfe3` (test)

## Files Created/Modified

**Created**

- `tests/driver_optin.rs` — Three integration proofs plus `fingerprint_tree`, `describe_difference` and `make_project`. Its banner states what the file is **not**: a test that a run stays inside its project, never a sandbox mechanism.

**Modified**

- `src/config.rs` — `CONFIG_SCHEMA_VERSION`, `migrate`, the two `extra` carriers, `driver_max_concurrent` + `default_driver_max_concurrent`, a hand-written `impl Default for Preferences`, and the file's first `#[cfg(test)] mod tests` (four tests).
- `src/registry.rs` — `record_opt_in`, `clear_opt_in`, `is_opted_in`, `claude_md_digest`; `extra: Default::default()` written out at both struct literals; three new tests.
- `src/app.rs`, `src/executor/mod.rs`, `tests/driver_tracer.rs` — one `extra` field each, at test-helper struct literals the new field broke.
- `tests/registry_test.rs` — two stale `version == 1` assertions now compare against `CONFIG_SCHEMA_VERSION`; the new opt-in round-trip test.

## Decisions Made

- **The `grep 'DriverOptIn {' src/ | wc -l == 1` criterion cannot be satisfied literally.** The pattern also matches `pub struct DriverOptIn {` (the definition) and one construction inside `src/executor/mod.rs`'s `#[cfg(test)] mod tests` — the `entry()` helper wave 1 added at line 753, well after the `#[cfg(test)]` at line 722. Raw count is 3 and always will be. The *intended* property is the one `record_opt_in`'s own doc states: the only construction site **outside tests**. Verified by truncating every `src/**/*.rs` at its `#[cfg(test)]` line and excluding the definition — exactly one hit, `src/registry.rs:115`.
- **Version assertions now name the constant, not the number.** Pinning `2` in `registry_test.rs` would need a test edit on every future schema bump for no gain; pinning `CONFIG_SCHEMA_VERSION` asserts the real property (a config this build writes carries this build's version).
- **`clear_opt_in` removes the record rather than storing a "revoked" marker.** The gate asks one question — is there a record? — and a second representation of "no" is a second thing that can be got wrong.
- **The D-21 `worktrees` assertion is anchored at the temp root.** Run over the absolute path it would be an assertion about the host's `TMPDIR` as much as about the driver; anchored, it is purely about the path the driver chose.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Two pre-existing tests pinned the old schema version**

- **Found during:** Task 1
- **Issue:** `tests/registry_test.rs::save_config_then_load_config_roundtrips` and `::load_config_on_nonexistent_file_returns_default` both assert `version == 1`. Bumping `CONFIG_SCHEMA_VERSION` to 2 — which `Config::new()` now uses — makes both fail. The plan's acceptance criterion requires all 12 pre-existing `registry_test` cases to remain present, so deleting them was never an option.
- **Fix:** Both now assert against `CONFIG_SCHEMA_VERSION`, with a comment recording why the constant beats a literal here.
- **Files modified:** `tests/registry_test.rs`
- **Verification:** `cargo test --test registry_test` — 13 passing (12 pre-existing + the new round-trip).
- **Committed in:** `57ccf17`

**2. [Rule 1 - Bug] An acceptance criterion's grep is unsatisfiable as written**

- **Found during:** Task 1 verification
- **Issue:** `grep -rn --include=*.rs 'DriverOptIn {' src/ | wc -l` outputs `1` cannot hold: the pattern matches the struct definition in `src/config.rs` and a test-module construction in `src/executor/mod.rs` that wave 1 added. Actual output is 3.
- **Fix:** No code change — the property the criterion is reaching for already holds. Verified with a pass that truncates each source file at its `#[cfg(test)]` marker and excludes the definition line, yielding exactly one hit at `src/registry.rs:115` inside `record_opt_in`.
- **Files modified:** none
- **Verification:** the truncating grep, plus `record_opt_in`'s doc stating the property in the form that is actually true ("the only function outside tests").
- **Committed in:** n/a (verification-only)

**3. [Rule 3 - Blocking] The `extra` field broke three more struct literals than the plan named**

- **Found during:** Task 1
- **Issue:** The plan anticipated the two `RegisteredProject` literals in `src/registry.rs`. Adding `extra` also broke `src/app.rs:915` (the `obs_app` test helper), `src/executor/mod.rs:753` (the `entry` test helper) and `tests/driver_tracer.rs:48` (wave 1's `config_for`). None of the three is in this plan's declared `files_modified`.
- **Fix:** One explicit `extra: Default::default()` line at each site — never a struct-update shorthand, which would absorb the next field silently and destroy the compile-time proof the breakage exists to provide.
- **Files modified:** `src/app.rs`, `src/executor/mod.rs`, `tests/driver_tracer.rs`
- **Verification:** `cargo build` clean; the full suite green.
- **Committed in:** `57ccf17`

**4. [Rule 2 - Missing Critical] The fingerprint comparison could pass vacuously**

- **Found during:** Task 2
- **Issue:** `assert_eq!(before, after)` on two empty vectors passes. A `fingerprint_tree` that silently found nothing — a swallowed `read_dir` error, a wrong root, a filter that skipped everything — would make the phase's headline safety assertion succeed while proving nothing. In a safety test that is the failure mode that matters most.
- **Fix:** Before the comparison, assert the walk saw `.planning/STATE.md` and at least one `.git/` entry. The second is doubly load-bearing: it is also the mechanical check that the walk descends into `.git`, which the plan requires and which a dot-directory filter would break.
- **Files modified:** `tests/driver_optin.rs`
- **Verification:** all three isolation tests pass with the pins in place.
- **Committed in:** `470cfe3`

**5. [Rule 1 - Bug] The `worktrees` substring check would have been host-fragile**

- **Found during:** Task 2
- **Issue:** The plan asks the cwd assertion to reject the substring `worktrees` in the recorded path. Run over the absolute path, that is partly an assertion about the machine's `TMPDIR` — and this very execution ran inside `…/.claude/worktrees/agent-…`, so the failure mode is not hypothetical.
- **Fix:** Strip the test's own temp root first, then check the remainder. Same property, anchored at the boundary where the driver's choice begins.
- **Files modified:** `tests/driver_optin.rs`
- **Verification:** `the_driven_agents_working_directory_is_the_driven_projects_root` passes and is meaningfully non-vacuous — it also asserts the recorded cwd equals A's canonical root, which would fail if the stand-in had merely inherited the test runner's `PWD`.
- **Committed in:** `470cfe3`

---

**Total deviations:** 5 (2 bugs in the plan's own criteria, 1 blocking compile breakage, 1 missing-critical anti-vacuity pin, 1 host-fragility fix). **Impact:** no scope change. No public symbol beyond the plan's list; no dependency added.

## Issues Encountered

**`rtk proxy sh -c` was needed for every pipeline, as wave 1 recorded.** Both plan-level greps in this plan pipe one filter into another, and under the hook the second filter receives no stdin and returns `0` vacuously. Wrapping the *whole* pipeline — not just the `cargo` invocation — in `rtk proxy sh -c` is the working form, exactly as 17-01's summary warned.

**The `$PWD` propagation in the stand-in is real, not assumed.** `std::process::Command::current_dir` does not update the `PWD` environment variable, so a naive `sh` could have reported the *parent's* directory. It does not — POSIX shells re-derive `PWD` at startup when it does not name the actual working directory — and the equality assertion against A's canonical root is what proves it rather than a comment claiming it.

## Verification Results

| Check | Result |
|---|---|
| `cargo build` | clean |
| `rtk proxy cargo test` | **450 passed, 0 failed** (baseline 439; +11) |
| `rtk proxy cargo test --lib config::` | 4 passed — all four named tests present |
| `rtk proxy cargo test --lib registry::` | 9 passed — all three named tests present |
| `rtk proxy cargo test --test registry_test` | 13 passed — 12 pre-existing + the new round-trip |
| `rtk proxy cargo test --test driver_optin` | 3 passed — all three named tests present |
| `cargo clippy -- -D warnings` | exits 0 |
| `cargo clippy --all-targets` warning count | **exactly 5**, all pre-existing (`browser.rs` ×3, `project_creator.rs` ×1, `state_reader/mod.rs` ×1); none in `src/config.rs`, `src/registry.rs` or `tests/driver_optin.rs` |
| `git diff --stat Cargo.toml Cargo.lock` | no change — no dependency added |
| `Preferences` derive list | `Debug, Serialize, Deserialize, Clone` — no `Default`, with a hand-written `impl` |
| non-test `DriverOptIn {` construction sites under `src/` | **exactly 1**, `src/registry.rs:115` inside `record_opt_in` |
| `grep -c 'driver_opt_in: None' src/registry.rs` | `2` — both literals explicit |

## Known Stubs

None. `driver_max_concurrent` is not a stub: it is a preference with a correct default whose *enforcement* is assigned to plan 17-05 at the spawn seam, and the field's doc says so at the field so nobody looks for the policy in `config.rs`.

## User Setup Required

None. The isolation tests initialise their own git repositories with identity and signing forced on the command line, so they neither depend on nor disturb a developer's global git configuration, and they run against checked-in shell fixtures with no subscription, network or quota dependency.

## Next Phase Readiness

- **17-05 (detached spawn, reattach)** inherits `Preferences.driver_max_concurrent` as a working count and owns its enforcement at the spawn seam. The reconciliation scan it builds is the enumeration the cap compares against.
- **17-07 (UI)** has `registry::record_opt_in`, `clear_opt_in` and `is_opted_in` ready; `is_opted_in` is the cheap read for a toggle's rendered state, and it is explicitly **not** the gate.
- **Phase 21 (CLAUDE.md drift)** has the digest on disk from the first opt-in, so it adds no second migration of a user-owned file. Phase 17 records it and acts on nothing.
- **One forward note:** the schema version is stamped into a user's `config.json` the first time this build saves — the plan rated this one-way and the `extra` carriers are what make it safe to live with. A rollback to a pre-Phase-17 binary preserves the opt-in records rather than deleting them, because that binary's `RegisteredProject` has no carrier; **it will drop them**. The carriers protect *forward* rollbacks (v1.6-with-carriers reading v2.1), not the one downgrade that predates the carriers themselves. That is unavoidable and is recorded here rather than discovered later.

## Requirements Traceability

`CTRL-03` is declared by this plan **and** by 17-07, which is not yet written. Marking it complete now would flip it to `Complete` while a sibling that also claims it is still unwritten — exactly the gap phase verification exists to catch. `REQUIREMENTS.md` is therefore deliberately untouched, consistently with 17-01's decision.

## Self-Check: PASSED

- `tests/driver_optin.rs` verified present on disk.
- Both commits verified present in `git log`: `57ccf17`, `470cfe3`.
- All Task 1 and Task 2 acceptance criteria re-run after the final commit; all pass, with the two grep criteria evaluated in their corrected form and the correction documented above.
- Plan-level verification re-run: build clean, 450 tests passing, zero failures, `cargo clippy -- -D warnings` exits 0, `--all-targets` warning count still exactly 5 and all pre-existing, `Cargo.toml` and `Cargo.lock` unchanged.

---
*Phase: 17-supervisor-detach-kill-switch-dry-run-opt-in-gate*
*Completed: 2026-07-29*
