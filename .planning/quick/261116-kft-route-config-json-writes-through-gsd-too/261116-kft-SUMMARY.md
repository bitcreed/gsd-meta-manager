---
quick_id: 261116-kft
status: complete
date: 2026-11-16
commits:
  - c9249393 feat(quick-261116-kft): save project config through gsd-tools config-set
  - 5253f8d6 docs(quick-261116-kft): document config saves through gsd-tools
---

# 261116-kft: Route config.json writes through gsd-tools

## What shipped

- `src/state_reader/config_set.rs` (new, pure): `plan_config_set(before, after)` diffs two
  configs into ordered `config-set <key> <value>` / `config-set <key> null` /
  `config-set-model-profile <profile>` calls. `cli_parse` replicates gsd-core 1.21.0's
  `parseConfigArgValue` (ASCII `DECIMAL_LITERAL`, JSON `[`/`{`, `project_code` verbatim).
  Refusals (Coerced, FlagShaped, Nul, KeyPath, Serialize) carry the key, never the value; the
  hand-written `Debug` of a call never prints the value.
- `src/ui/screens/detail.rs`: `persist_active_config` routes the Project scope to
  `save_project_config` (sync, 3 s whole-edit budget, `save_project_config_within` for tests).
  Refusal order: platform, active workstream, fresh read/parse, plan, launcher, `node` on PATH.
  Failure lines: gsd-tools' last stderr line (secret redacted, capped at 120, `shown()`), the
  timeout / spawn / io lines, and `Config partly saved (k of n changes): …`. The cache is
  re-read from disk after every attempt.
- `src/ui/screens/owner_actions.rs`: `run_gsd_tools` shared by the acknowledge and config
  saves; `resolve_launcher` / `on_path` / `active_workstream_is_set` are `pub(crate)`;
  unforced `resolve_launcher` resolves nothing under `cfg(test)`.
- `src/state_reader/config_write.rs`: the `.planning/.lock` replica is gone (`LockFlavor`,
  `PlanningLockBody`, `planning_on_exists`, `planning_verified_live`,
  `is_transient_create_error`, `STALE_COUNTER`, `ConfigWriteTarget`, six planning-lock tests).
  The only entry point is `update_global_defaults` for `$GSD_HOME/.gsd/defaults.json`.
- `tests/config_set_roundtrip.rs`: oracle round trip (commit_docs, response_language, clear,
  workflow.plan_check from an absent block, context_window, model_profile, brave_search and
  tavily_search keys) leaves an empty product diff; `cli_parse` agrees with
  `config-set --dry-run` on all 30 table rows; the coercion refusal leaves the file
  byte-identical. Temp projects only, `GSD_HOME` pointed at a TempDir.
- Docs: README (gsd-tools row, "What it writes"), CONFIGURATION (refusals, argv exposure, how
  a save works), ARCHITECTURE (`config_set.rs`), GSD-CORE-SYNC (re-run the round trip each
  sync; check for a defaults.json writer).

## D-02: defaults.json gap (reported to the user)

gsd-core 1.21.0 has no command that writes `$GSD_HOME/.gsd/defaults.json`. Re-measured: the
only writes are the installer (`acquireInstallMigrationLock`) and a `depth` -> `granularity`
migration rewrite inside `loadConfig` (`config.cjs:386`, `:513-516`); `config-set` has no
global/defaults flag. The Global scope therefore still writes the file directly (installer
lock + atomic replace). The user decides whether to keep or drop the Global edit.

## Rows gsd-core refuses

`config-set <key> null --dry-run` against the installed 1.21.0 for every top-level field and
every dotted key literal in the Config-tab edit functions (117 keys): every dotted leaf is
accepted. Refused: block names (never emitted as leaves by the planner) plus
`context_profile` (an editable row: its save now fails visibly with gsd-core's
`Unknown config key`), `agent_skills` and `sub_repos` (read-only rows, never saved).

## Inferred decisions

- I-kft-1: cwd = project root, no `--project-dir` (Phase 37 precedent).
- I-kft-2: an active workstream refuses the save (gsd-core has no "root" spelling).
- I-kft-3: the save stays synchronous with a 3 s budget (one call ~150 ms).
- I-kft-4: secrets travel as argv (`/proc/<pid>/cmdline` exposure accepted, documented).
- I-kft-5: the edit applies to a fresh disk read (I-guw-4 semantics kept).
- I-kft-6: the cache is refreshed from a re-read whatever the outcome.
- I-kft-8: non-unix refusals of `atomic_write` / `update_global_defaults` kept.
- I-kft-11: unit tests can never resolve this machine's gsd-core (forced fake launchers).
- Deviation: Tasks 1 and 2 landed as one feat commit (the previous executor was rate-limited
  mid-task; splitting the finished tree by hunk added no value).

## Verification

- `cargo test --locked --no-fail-fast`: 4420 passed, 0 failed, 19 ignored (67 binaries);
  oracle tests ran against the installed gsd-core 1.21.0, not skipped.
- `cargo clippy --locked --all-targets -- -D warnings`: clean on host,
  `x86_64-pc-windows-gnu` and `aarch64-apple-darwin`. `cargo fmt --check`: clean.
