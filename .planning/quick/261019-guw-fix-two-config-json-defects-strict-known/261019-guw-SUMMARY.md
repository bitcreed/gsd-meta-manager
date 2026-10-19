---
phase: quick-261019-guw
plan: 01
subsystem: state_reader / Defaults tab
tags: [config-json, locking, atomic-write, gsd-core-sync]
status: complete
requirements: [QUICK-261019-guw]
key-files:
  created:
    - src/state_reader/config_write.rs
  modified:
    - src/state_reader/mod.rs
    - src/state_reader/config_json.rs
    - src/ui/screens/detail.rs
    - docs/GSD-CORE-SYNC.md
commits: 3
plan_head_before: 79a6cc1cc0fa7533693188a6876525e9f087da86
plan_head_after: 76bbac1
actuals:
  tasks: 3
  commits: 3
---

# Quick 261019-guw: config.json lock/atomic saves and strict_known_registries

Every Defaults-tab save now goes through a gsd-core-compatible lock plus fresh read-modify-write plus atomic permission-preserving write (`state_reader::config_write`), and `capabilities.strict_known_registries` is modelled as upstream `array | null` as a read-only row.

## Commits

| Task | Commit | Message |
|------|--------|---------|
| 1 (tracer) | d88ef04 | fix(quick-261019-guw): lock-aware atomic config.json saves (tracer) |
| 2 | da001af | test(quick-261019-guw): pin gsd-core lock semantics and atomic write |
| 3 | 76bbac1 | fix(quick-261019-guw): model capabilities.strict_known_registries as array\|null |

`plan_head_before` is the HEAD when the ledger was first written (79a6cc1, the HEAD at start); the three commits above are the only ones this plan made, so `commits: 3` is the count of those shas (measured from `git log`, no concurrent docs commit landed in this range).

## Lock decision: (a), replicate the protocol in Rust

Reason (I-guw-1): `gsd config-set` writes one key per spawn, would put Secret values on argv (/proc/<pid>/cmdline, breaks T-jnf-01), rejects retired `gates.*` keys the tab must preserve, has no defaults.json target, and would trip the `Command::new` audit. The protocol is small and stable.

Protocol implemented (citations: gsd-core v1.18.0-6-g8d3d29c46):
- `.planning/.lock` = `withPlanningLock` (planning-workspace.cjs:313-472): `O_EXCL` create, body `{"pid","cwd","acquired"}` compact no newline; transient errnos retried; verified-live holder (pid integer > 0, `kill(pid,0)` ok or EPERM) is waited on; not-live holder (dead / garbage / pid-less / empty) is stolen via (dev, ino, body) re-confirm + rename-aside; unreadable body (symlink/FIFO/oversized) never stolen; no deadman steal.
- `$GSD_HOME/.gsd/gsd-install-migration.lock` = `acquireInstallMigrationLock` (installer-migrations.cjs:499-597): body `{"pid","acquiredAt"}` + newline; dead / own / non-positive pid reclaimed by unlink; unparseable body never reclaimed; poll 50 ms.
- Budget 2 s (poll 100 ms planning / 50 ms installer). Write is read-inside-lock, mutate closure on the fresh copy, same-dir `O_EXCL` temp, mode carried over, fsync, rename; release unlinks only if the body is still ours.

## Files changed

- `src/state_reader/config_write.rs` (new): `LockFlavor`, `LockPolicy`, `ConfigWriteTarget`, `ConfigWriteError::status_text`, RAII lock guard, `atomic_write`, `update_gsd_config`, 15 unit tests (18 in the `config_write` filter incl. 3 detail tests).
- `src/state_reader/mod.rs`: `pub mod config_write;`.
- `src/ui/screens/detail.rs`: `persist_active_config` rewritten (closure edit, `PersistOutcome`), all five call sites converted; `x` arm keeps the error status on `Failed`; `opt_registry_allowlist` helper, read-only row with help + `since("v1.6.0")`, the three `strict_known_registries` mutation arms deleted; fixture changed to an array; 4 new tests.
- `src/state_reader/config_json.rs`: `RegistryAllowlist { Hosts, Malformed }`, field now `Option<Option<RegistryAllowlist>>`, 5 new tests, existing fixture updated.
- `docs/GSD-CORE-SYNC.md`: Capabilities row since `v1.6.0`, read-only note, Modelled paragraph re-tallied (1 carries v1.6.0, 72 without; total 135).

## Verification

- `rtk proxy cargo test --no-fail-fast`: 68 suites, 3235 passed, 1 failed, 18 ignored. The one failure is the expected `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against` (lib: 2250 passed, 1 failed, 1 ignored). All envelope integration suites ran.
- `rtk proxy cargo clippy --all-targets -- -D warnings`: exit 0. `cargo fmt --check`: clean.
- Cargo.toml / Cargo.lock unchanged across the three commits (`git diff --stat HEAD~3 -- Cargo.toml Cargo.lock` empty).
- Task 1 greps: `std::fs::write(&path, &json)` count 0 (baseline 1). Task 3 greps: `strict_known_registries: Option<bool>` 0 (baseline 1), `strict_known_registries = Some(` 0 (baseline 2).
- DEFAULTS_OPTION_COUNT still 135.

### Atomicity negative control

`atomic_write_never_exposes_a_partial_file` was pointed temporarily at a plain `std::fs::write` and FAILED immediately ("a reader saw a partial file (106496 bytes)"); reverted, then passes against `atomic_write`.

## Tests updated because of the closure-first reordering

None. No existing test relied on the old in-memory-first ordering; the full suite passed unchanged apart from the strict_known_registries fixture/assertion updates the plan called for.

## Deviations from Plan

**1. [Rule 1 - Bug] `probe_pid` panicked on a negative pid.**
- Found during: Task 2 (`planning_lock_garbage_empty_or_pidless_body_is_stolen`, body `{"pid":-5}`).
- Issue: `rustix::process::Pid::from_raw` asserts `raw >= 0`; a lock body with a negative pid crashed the save (a plantable DoS of the TUI).
- Fix: reject non-positive values before calling `from_raw`.
- Files: `src/state_reader/config_write.rs`. Commit: da001af (with the tests).

**2. Extra tests beyond the plan list** (not deviations in behaviour): `atomic_write_never_touches_a_foreign_file_at_a_temp_name` (T-guw-06) and `status_text_carries_no_path_and_names_only_the_pid` (T-guw-08).

**3. Commit on `master`.** The Step 0 protected-branch guard would refuse master; the orchestrator explicitly dispatched this as a SEQUENTIAL executor on master (`gsd-tools` is not on PATH, so the guard's query was unavailable). Committed on master as directed; nothing pushed.

## Inferred decisions (human unavailable; all marked for audit)

- [INFERRED I-guw-1] Option (a), replicate the protocol in Rust, for the reasons above.
- [INFERRED I-guw-2] The TUI lock budget is **2 s**, against upstream's 10 s, with polls at upstream's intervals (100 ms planning, 50 ms installer). The save runs synchronously on the key-handler thread. gsd-core holds the lock only for a sub-second read-modify-write, so 2 s waits out any real writer without freezing the UI for 10 s.
- [INFERRED I-guw-3] **No deadman steal.** The requester's rule "never steal a lock held by a live holder" overrides upstream's 60 s pid-reuse recovery. A verified-live holder always ends in a timeout error that names the pid. gsd-core's own next write still applies its deadman.
- [INFERRED I-guw-4] **Read-modify-write inside the lock.** The edit is passed in as a closure and applied to a FRESH read of the on-disk file, as upstream setConfigValue does. If the on-disk file exists but does not parse, the save is refused with no write. If it is absent, the cached config is the base (create-on-save, including the Global seed). The cache is replaced only after a successful write. With no resolvable path, the edit is applied in memory only.
- [INFERRED I-guw-5] `defaults.json` gets the same atomic write plus the installer's `gsd-install-migration.lock` with installer semantics.
- [INFERRED I-guw-6] Lock bodies are read with O_NOFOLLOW|O_NONBLOCK, a regular-file fstat check and a 4 KiB cap. The config file read inside the lock follows symlinks, as gsd-core does, but uses O_NONBLOCK, a regular-file check and a 4 MiB cap.
- [INFERRED I-guw-7] A symlinked config.json or defaults.json is replaced by a regular file on save (rename replaces the link), as gsd-core's own writers do.
- [INFERRED I-guw-8] strict_known_registries is tri-state (absent / null / value) with a `Malformed(serde_json::Value)` catch-all so a pre-fix bool round-trips. The row is read-only with no clear; help text points to `gsd config-set`. `since` is `v1.6.0` (measured via the GSD-CORE-SYNC recipe: first commit 9219af336, `tag --contains` gives v1.6.0).
- [INFERRED I-guw-9] On release, the lock is unlinked only if its body still equals the body we wrote; a release failure is a `tracing::warn`, never a failed save.

Executor-level inferences made while implementing (also for audit):
- [INFERRED E-1] A Planning-flavour wait that ends with no parsed holder but a transient create errno (e.g. persistent ENOENT) reports `Io{kind}` rather than a misleading "held by another process".
- [INFERRED E-2] An explicit JSON `null` for the key shows as `null` (ReadOnly); only an absent key shows `(unset)` (Null).
- [INFERRED E-3] A lock pid that does not fit `pid_t` is "not verified-live" for the Planning flavour (stealable, per the plan's "fits i32" definition) and, for the installer flavour, is treated as alive (never reclaimed, conservative).
- [INFERRED E-4] The `x` arm only reports "cannot be cleared" when an active config exists (as before); a `Failed` outcome keeps the lock/IO error status.
- [INFERRED E-5] `status_text` takes the `LockFlavor` as its scope argument (state_reader must not depend on the UI's `DefaultsEditTarget`).

## Known Stubs

None.

## Threat Flags

None. The new surface (lock file creation, temp file creation in `.planning/` and `$GSD_HOME/.gsd/`) is exactly what the plan's threat model T-guw-01..09 covers.

## Self-Check: PASSED

- Files exist: config_write.rs, SUMMARY target path, all modified files present.
- Commits d88ef04, da001af, 76bbac1 are ancestors of HEAD.
