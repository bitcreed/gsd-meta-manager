---
phase: quick-261125-kfc
plan: 01
subsystem: registry
tags: [config, registry, tui, auto-registration, lost-update]
requires: []
provides:
  - config::update_config (fresh read, replay the caller's registry operation, atomic write only on a change)
  - AppContext::adopt_config / track_project / forget_project
  - App::reload_config_if_changed on the 20-tick session poll
  - registry::is_in_gsdmm_data_dir (data-dir guard for session auto-registration)
affects: [src/config.rs, src/app.rs, src/registry.rs, src/main.rs, src/ui/screens]
key-files:
  created: []
  modified:
    - src/config.rs
    - src/ui/screens/mod.rs
    - src/app.rs
    - src/ui/screens/add_project.rs
    - src/ui/screens/delete_confirm.rs
    - src/ui/screens/driver_confirm.rs
    - src/main.rs
    - src/registry.rs
    - README.md
    - docs/GETTING-STARTED.md
    - docs/CONFIGURATION.md
key-decisions:
  - "Replay, not merge: update_config re-runs the caller's own registry function on the file as it is now"
  - "No cross-process lock; the residual few-millisecond race is documented (T-kfc-02)"
  - "Screens on a failed write change nothing in memory; the driver opt-in revert (WR-04) is deleted, memory only adopts what reached disk"
  - "A driver opt-in grant is refused when the on-disk path differs from the confirmed one (D-14); a withdrawal is not"
  - "Data-dir guard is structural (gsd-meta-manager component followed by oneoff/driver/envelope) plus the canonical real data root"
status: complete
completed: 2026-11-25
commits: 5
plan_head_before: 2d0be8acae15e9d50c298132642b7aaa36c55412
plan_head_after: 4881f331decc981f6d8b70522cf746070fe2a5ba
actuals:
  tokens: 21000
  tasks: 3
  commits: 5
---

# Phase quick-261125-kfc Plan 01: TUI registry lost update and one-off clone auto-registration Summary

A running TUI no longer undoes another process's `config.json` edits (every registry write re-reads the file and replays only its own change), and session auto-registration never registers a root inside gsd-meta-manager's own data dir.

## Commits

| Commit | Subject |
|--------|---------|
| d5f9911c | fix(quick-261125-kfc): session auto-registration applies its additions to the config on disk |
| 6a5c647f | fix(quick-261125-kfc): every TUI registry write re-reads config.json; the session poll adopts external edits |
| 344635fd | fix(quick-261125-kfc): never auto-register a root inside gsd-meta-manager's own data dir |
| 3321a4b9 | docs(quick-261125-kfc): registry writes keep external edits; one-off clones are never auto-registered |
| 4881f331 | fix(quick-261125-kfc): name the session auto-registration function type for clippy |

`plan_head_before..plan_head_after` also contains `38457b09` (quick-261125-khh, another executor working in the same tree concurrently), so `git rev-list --count` reads 6; the 5 above are this plan's. Because khh left uncommitted edits in several of the same files (`src/app.rs`, `src/ui/screens/mod.rs`, README.md, GETTING-STARTED.md), task 1 staged only this plan's hunks (hunk-filtered `git apply --cached`), and the remaining uncommitted files in `git status` belong to khh.

## What was done

**Task 1 (tracer).** `config::update_config(path, mutate)` loads `config.json` (missing = empty, unreadable or unparsable = outer error before `mutate` runs, never overwritten), clones it, runs the caller's registry operation, and writes through `save_config` (atomic tempfile + rename) only when the serialized config changed. An inner refusal returns the config as loaded and writes nothing. `AppContext::adopt_config` swaps the registry in and, for aliases new to the TUI (or repointed), calls `track_project` (`.planning/` watch, parsed state, change tracker, owner digest); for vanished ones it calls `forget_project` (the old delete-screen cleanup). `auto_register_new_sessions_using` probes a clone first (an idle poll does no disk I/O), then updates the file on disk and adopts the result; on an outer error it still registers in memory only.

**Task 2.** The add screen, delete screen, driver opt-in toggle, `CreateProjectResult` and the launch-target registration in `main.rs` all go through `update_config`. A write error shows `Failed to save config: ...` and changes nothing in memory; an alias already gone from disk counts as removed; the opt-in grant is refused when the on-disk path differs from the confirmed one. `App::reload_config_if_changed` (a `(mtime, len)` stamp, one `stat` per call) rides the existing 20-tick poll block before `prune_driver_maps`; a missing or unparsable file is not adopted, and an identical registry is not re-adopted.

**Task 3.** `registry::is_in_gsdmm_data_dir` skips a resolved root that has a `gsd-meta-manager` component directly followed by `oneoff`, `driver` or `envelope`, or that lies under the canonical `dirs::data_local_dir()/gsd-meta-manager`. It logs `auto-register: skipping root inside gsd-meta-manager's own data dir` at debug. A unix test pins the tree names to `oneoff_root()` and `driver_root()`. Manual add and launch-target registration are unchanged. README, GETTING-STARTED and CONFIGURATION document both fixes.

## Test leak finding

The test is hermetic (`env_clear`, scratch `--config`, XDG dirs under the scratch dir). The leaked `work*` entries reached the real config because the user's own running TUI polls `pgrep -x claude` system-wide, saw the fake `claude` the test started in its one-off clone, and auto-registered that cwd into its own config. No test-isolation change can stop another process from observing it; the data-dir guard is the fix.

## Verification

Run in a clean detached worktree of HEAD `4881f331` (the shared tree had another executor's uncommitted, partly non-compiling edits):

- `cargo test --no-fail-fast` (raw log, `target/kfc-final.log` in that worktree): 69 `test result: ok`, 0 FAILED, 4715 passed, 0 failed tests (baseline 4701 at a363aeab).
- `cargo clippy --all-targets -- -D warnings`: clean (host).
- `cargo clippy --locked --target x86_64-pc-windows-gnu --all-targets -- -D warnings`: clean.
- `cargo fmt --check`: clean.
- Doc and source grep gates of the plan all hold (data-dir phrase in README and GETTING-STARTED, `re-reads \`config.json\`` and `within one session poll` in README, `update_config` in CONFIGURATION, old "last writer wins" sentence gone).

New tests: 6 in `config::tests`, 2 in `ui::screens::tests` (adopt), 4 in `app::tests` (tracer, in-memory fallback, reload, no-adopt), 1 add screen, 2 delete screen, 2 driver confirm, 4 in `registry::gsd_root_guard_tests`.

## Deviations from Plan

1. **[Rule 3 - Blocking] Clippy `type_complexity` on the injected register fn type.** Fixed with a `RegisterFromSessions` type alias (the plan allowed it); commit 4881f331.
2. **Additional config tests.** Added `an_unparsable_config_is_never_overwritten` (T-kfc-05) and `adopting_an_empty_registry_clears_the_selection` beyond the planned set.
3. **delete_confirm fixture assertion.** `unregistering_a_project_with_a_live_launcher_run_is_refused` asserted the config file does not exist; the fixture now seeds the registry on disk, so it asserts the seeded bytes are unchanged (same claim: a refused removal writes nothing). No other fixture file outside the plan's list needed edits.
4. **Grant-only path check.** The repointed-path refusal applies to the grant direction only; withdrawing consent is always allowed.
5. **Reload skips an identical registry.** `reload_config_if_changed` compares the serialized configs and does not adopt (or request a redraw) when the file equals memory, so the TUI's own writes cause no redundant redraw.
6. **Same-tick rewrite caveat.** The reload stamp is `(mtime, len)`; a same-length rewrite within one coarse filesystem timestamp tick is picked up on the next change. The reload test therefore uses a longer alias, as the plan's "changing the file length" implied.

## Known Stubs

None.

## Threat Flags

None (no new network, auth or schema surface).

## Notes for the user

Install the new build and restart every running gsdmm TUI first: an older TUI still rewrites its whole copy and would undo the cleanup. Then run `gsd-meta-manager list` and `gsd-meta-manager remove <alias>` for each leaked one-off entry (aliases `work`, `work-2`, ... whose path runs through `gsd-meta-manager/oneoff/`). The executor never read, edited or wrote the real `~/.config/gsd-meta-manager/config.json`.

## Self-Check: PASSED

- All five commits exist and are ancestors of HEAD (`d5f9911c`, `6a5c647f`, `344635fd`, `3321a4b9`, `4881f331`).
- `update_config`, `adopt_config`, `auto_register_new_sessions_using`, `reload_config_if_changed`, `is_in_gsdmm_data_dir` and the locator pin test exist in the tree.
