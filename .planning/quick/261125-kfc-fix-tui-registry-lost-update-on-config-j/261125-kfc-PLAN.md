---
phase: quick-261125-kfc
plan: 01
type: execute
wave: 1
depends_on: []
quick_id: 261125-kfc
mode: quick
files_modified:
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
autonomous: true
requirements: [QUICK-261125-kfc]

estimate:
  tokens: 150000
  raw_tokens: 150000
  tasks: 3
  confidence: low

must_haves:
  truths:
    - "Every registry write the TUI makes (session auto-registration, the add screen, the delete screen, the driver opt-in toggle, a created project, launch-target registration) re-reads config.json and re-runs only its own registry operation on what it read: an entry another process removed stays removed, and an entry another process added survives with its own `added` timestamp"
    - "After each such write the in-memory registry is exactly the config that was read and written: aliases new to the TUI get project state, a `.planning/` watch and an owner digest, vanished aliases lose them, and the table selection stays in bounds"
    - "A write that fails (config.json unreadable, unparsable or unwritable) changes nothing on disk and nothing in memory and shows `Failed to save config: ...`; only session auto-registration still registers in memory for the session, as it did before; a mutation that changes nothing touches no file"
    - "On the existing ~5s session-poll counter a running TUI adopts a config.json another process changed; a file that is missing or does not parse leaves the in-memory registry as it is"
    - "Session auto-registration never registers a resolved root inside gsd-meta-manager's own data dir (canonical `dirs::data_local_dir()/gsd-meta-manager`) or any root whose path has a `gsd-meta-manager` component directly followed by `oneoff`, `driver` or `envelope` (one-off clones `<run>/work` and `<run>/land/<n>/work` under a relocated data dir included), and logs `auto-register: skipping root inside gsd-meta-manager's own data dir` at debug; a project folder named `gsd-meta-manager`, or one under `gsd-meta-manager-x/oneoff/`, still registers; manual add and launch-target registration are unchanged"
    - "README.md, docs/GETTING-STARTED.md and docs/CONFIGURATION.md state the data-dir rule and that registry writes keep other processes' edits; the full suite runs 69 suites with 0 failed and at least 4715 passed; clippy -D warnings (host and x86_64-pc-windows-gnu) and rustfmt are clean"
  artifacts:
    - path: src/config.rs
      provides: "Config derives Clone; update_config (fresh read, re-run the caller's mutation, write only on a change, atomic through save_config)"
    - path: src/ui/screens/mod.rs
      provides: "AppContext::adopt_config, track_project, forget_project"
    - path: src/app.rs
      provides: "auto_register_new_sessions_using (probe, update_config, adopt, in-memory fallback); CreateProjectResult through update_config; config_stamp + reload_config_if_changed on the 20-tick session-poll counter"
    - path: src/ui/screens/add_project.rs
      provides: "do_add_project through update_config + adopt_config"
    - path: src/ui/screens/delete_confirm.rs
      provides: "do_remove_project through update_config + adopt_config; an alias already gone from disk counts as removed"
    - path: src/ui/screens/driver_confirm.rs
      provides: "do_toggle_opt_in through update_config + adopt_config; no in-memory rollback; a grant refused when the on-disk path differs from the confirmed one"
    - path: src/main.rs
      provides: "launch-target registration through update_config"
    - path: src/registry.rs
      provides: "GSDMM_DATA_SUBDIR, GSDMM_RUN_TREES, gsdmm_data_root, is_in_gsdmm_data_dir; the data-dir guard inside auto_register_from_sessions_with_temp_root"
    - path: README.md
      provides: "Quick start step 4 data-dir rule; Configuration section on registry writes and the reload"
    - path: docs/GETTING-STARTED.md
      provides: "first-launch auto-registration bullet states the data-dir rule"
    - path: docs/CONFIGURATION.md
      provides: "Atomic writes bullet describes the re-read and the residual race; update_config in the source list"
  key_links:
    - from: src/app.rs
      to: src/config.rs
      via: "App::auto_register_new_sessions -> auto_register_new_sessions_using(registry::auto_register_from_sessions) -> config::update_config -> AppContext::adopt_config"
    - from: src/ui/screens/delete_confirm.rs
      to: src/config.rs
      via: "do_remove_project -> config::update_config(registry::remove_project) -> AppContext::adopt_config (forget_project does the map cleanup)"
    - from: src/ui/screens/driver_confirm.rs
      to: src/config.rs
      via: "do_toggle_opt_in -> config::update_config(registry::record_opt_in | registry::clear_opt_in) -> AppContext::adopt_config"
    - from: src/app.rs
      to: src/ui/screens/mod.rs
      via: "the 20-tick session-poll block calls App::reload_config_if_changed before prune_driver_maps; on a new config_stamp it loads and calls AppContext::adopt_config"
    - from: src/main.rs
      to: src/config.rs
      via: "TUI launch with an Unregistered target -> config::update_config(launch_target::auto_register) -> app.ctx.config = fresh"
    - from: src/registry.rs
      to: src/registry.rs
      via: "auto_register_from_sessions_with_temp_root computes gsdmm_data_root() once per call and skips a root for which is_in_gsdmm_data_dir holds"
---

<objective>
Fix two registry bugs seen live in the user's `~/.config/gsd-meta-manager/config.json`.

1. Lost update. Every TUI registry write calls `save_config` with the TUI's whole in-memory `Config`, loaded once at startup. The user ran `gsd-meta-manager remove .tmp8d6c3g` while a TUI was running; the TUI's next auto-register save wrote the removed entry back with its original `added` timestamp. A CLI `add` made meanwhile would be lost the same way, and a hand-edited config.json that no longer parses would be overwritten too. Fix: one primitive, `config::update_config`, re-reads config.json, re-runs the caller's own registry operation on what it read, and writes atomically only when something changed. Every TUI write site uses it, the TUI adopts the result in memory (`AppContext::adopt_config`), and the existing ~5s session poll reloads config.json when another process changed it.
2. One-off clones get auto-registered. gsdmm's one-off runs start `claude` in `<data dir>/gsd-meta-manager/oneoff/<hash8>/<run>/work`, a clone with a real `.planning/ROADMAP.md`, so session auto-registration adds it as `work`, `work-2`, ... `tests/oneoff_e2e.rs::a_data_root_outside_home_is_masked_in_the_sandbox` puts its data dir under `target/tmp/hx7-mask/`, outside /tmp, so the jx7 temp-dir guard misses it, and production clones live under `~/.local/share`. Fix: session auto-registration never registers a root inside gsdmm's own data dir, matched by path structure and by the resolved real data dir.

Purpose: the registry on disk is the single source of truth, edited by any number of gsdmm processes, and it only ever holds projects the user would recognize.
Output: `update_config` + `adopt_config` with unit and end-to-end tests, every TUI writer migrated, a reload on the session poll, the data-dir guard with tests, updated docs.
</objective>

<execution_context>
@~/.claude/gsd-core/workflows/execute-plan.md
@~/.claude/gsd-core/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@CLAUDE.md
@.planning/quick/261125-jx7-don-t-auto-register-non-gsd-or-temp-dir-/261125-jx7-SUMMARY.md

Read these regions (line numbers at HEAD 2d0be8ac; read each once):
- src/config.rs 23-29 (`Config`), 643-672 (`load_config`, `save_config`), 674 (test module marker), 815-822 (`write_config` test helper)
- src/ui/screens/mod.rs 1351-1470 (`AppContext` fields), 2105-2125 (`schedule_owner_digest`: needs the `project_states` entry first and returns early without `event_tx`), 2283-2292 (`recompute_filtered_aliases`), 2344 (end of `impl AppContext`), 2380 (test module marker), 2794 (test `AppContext` fixture)
- src/app.rs 936-1037 (`App::new`, `new_for_test`, `from_config`, the only `App` literal), 2683-2735 (`auto_register_new_sessions`), 2770-2781 and 2877-2881 (the 20-tick session-poll block and its `prune_driver_maps` call), 3103-3191 (`Action::CreateProjectResult`), 3843 (`prune_driver_maps`), 4030 (test module marker), 4837 (`obs_app`), 6975-7010 (`removing_a_project_interactively_clears_its_per_alias_maps`)
- src/ui/screens/add_project.rs 181-230 and 273-291 (its one test uses `App::new_for_test`)
- src/ui/screens/delete_confirm.rs 121-215 and 228-260 (`ctx_with_project`)
- src/ui/screens/driver_confirm.rs 274-371 (`do_toggle_opt_in` and its rollback), 382-445 (`ctx_with_project`, shared by detail.rs, driver.rs, driver_run.rs, driver_launch.rs tests), 515-700 (`unwritable_config_path`, `grant`, `toggle`, the four failed-save tests)
- src/main.rs 470-527 (launch-target registration before `tui::init`); 188-290 (CLI add/remove: NOT modified)
- src/registry.rs 1040-1092 (markers, temp guard), 1214-1300 (`auto_register_from_sessions*`), 1432-1476 (`load_config_pruning_worktrees`: NOT modified), the `gsd_root_guard_tests` module (above `mod tests`)
- src/oneoff/mod.rs 69-70, src/oneoff/store.rs 777-781 (`oneoff_root`, linux/macos only), src/launcher/config_dir.rs 66-69 and 142-146 (`driver_root`, linux/macos only), src/envelope/mod.rs 98-126
- README.md 213-226 and 936-947; docs/GETTING-STARTED.md 73-91; docs/CONFIGURATION.md 677 and 719

Survey results the executor can rely on (do not re-survey):
- Registry writers in the TUI process, all writing the whole in-memory `Config`: `src/app.rs:2696` (auto-register, once per added alias), `src/app.rs:3128` (CreateProjectResult), `src/ui/screens/add_project.rs:199`, `src/ui/screens/delete_confirm.rs:157`, `src/ui/screens/driver_confirm.rs:314`, `src/main.rs:505` (launch target, before the TUI starts). The Preferences screen never writes (its tests assert the config path stays absent). No other production `save_config` call exists besides `src/main.rs:236` / `:285` (CLI add/remove) and `src/registry.rs:1467` (startup prune), which load immediately before they save.
- Production code never indexes `config.projects[...]`; every `[...]` index is in test code, so an alias vanishing from the in-memory registry cannot panic a screen.
- `App` has exactly one struct literal (`from_config`); `AppContext` has several literal fixtures and one exhaustive destructure (`src/app.rs:7097`), so this plan adds no `AppContext` field.
- `tests/spawn_seam_guard.rs` (`no_production_item_follows_a_test_module_marker`) fails when a production item follows a file's first test-module marker (the jx7 deviation): put every new production item above it. In-source tests may not spawn processes: std::fs only. `tests/async_blocking_guard.rs` scans `async fn` bodies for named calls; neither `save_config` nor `update_config` is a marker.
- Test leak, root cause [confirmed by reading `tests/launcher_support/mod.rs:381-402` and `src/session_detector.rs:63,89`]: the test process is hermetic (`env_clear`, `--config <scratch>/registry.json`, XDG_CONFIG/DATA/STATE_HOME under the scratch dir). The entries reached the real config because the user's own running TUI polls `pgrep -x claude` system-wide, saw the fake `claude` the test started in its one-off clone, and auto-registered that cwd into the TUI's own config. No test-isolation change can stop another process from observing it; the product guard is the fix.
- CI and the pre-tag container run tests as a non-root user (`runner`), so `/nonexistent/config.json` (`App::new_for_test`) is reliably unwritable.
- `rtk proxy <cmd>` preserves the exit status. Full-suite baseline: `target/jx7-final.log` at a363aeab (no src/tests change since): 69 `test result: ok` lines, 0 FAILED, 4701 passed. `cargo fmt --check` exits 0 at HEAD. Installed targets include x86_64-pc-windows-gnu. `estimate-calibration`: factor 1, 0 samples.

<interfaces>
Names are binding; the gates grep for them.

```rust
// src/config.rs
#[derive(Debug, Clone, Serialize, Deserialize)]   // Clone added
pub struct Config { /* unchanged */ }

/// Read config.json, run `mutate` on what was read, write only if it changed.
/// Outer Err: unreadable/unparsable file or failed write; nothing was written.
/// Inner Err: the mutation refused; nothing was written and the config is returned as loaded.
pub fn update_config<T, E>(
    path: &Path,
    mutate: impl FnOnce(&mut Config) -> Result<T, E>,
) -> anyhow::Result<(Config, Result<T, E>)>;

// src/ui/screens/mod.rs, impl AppContext
pub fn adopt_config(&mut self, fresh: Config);
pub(crate) fn track_project(&mut self, alias: &str, path: &Path);
pub(crate) fn forget_project(&mut self, alias: &str, path: &Path);

// src/app.rs
fn auto_register_new_sessions_using(&mut self, register: fn(&mut Config, &[ClaudeSession]) -> Vec<(String, PathBuf)>);
fn reload_config_if_changed(&mut self);
fn config_stamp(path: &Path) -> Option<(std::time::SystemTime, u64)>;   // free fn
// App field: config_stamp: Option<(std::time::SystemTime, u64)>

// src/registry.rs
const GSDMM_DATA_SUBDIR: &str = "gsd-meta-manager";
const GSDMM_RUN_TREES: [&str; 3] = ["oneoff", "driver", "envelope"];
pub(crate) fn gsdmm_data_root() -> Option<PathBuf>;                 // canon_or_raw(data_local_dir/gsd-meta-manager)
pub(crate) fn is_in_gsdmm_data_dir(root: &Path, data_root: Option<&Path>) -> bool;
```
</interfaces>

Decisions (no CONTEXT.md exists; the brief left the design open, so all are [inferred] unless marked):
- [inferred] Replay, not merge: `update_config` re-runs the caller's own registry function (`add_project`, `add_project_unchecked`, `remove_project`, `record_opt_in`/`clear_opt_in`, `auto_register_from_sessions`, `launch_target::auto_register`) on the file as it is now. The registry's own refusals (duplicate alias, unknown alias, linked worktree) then judge the current registry, and no entry is ever rewritten from a stale copy. A three-way diff of whole entries would still overwrite a concurrent change to the same entry.
- [inferred] Order inside `update_config`: load, clone, mutate, then write only on a change (serde_json Value comparison of before and after). A refusal or a no-op performs no filesystem write and creates no directory, and a refusal is reported even when the location is unwritable. A missing file loads as an empty registry: that is what is on disk, and the first write ever must work. A file that does not parse is an outer error and is never overwritten.
- [inferred] No cross-process lock. The repository's only file lock (`src/driver/lock.rs`) is unix-only flock with a guarded-open discipline. A config lock would need that on all three release targets, and that change is bigger than this fix. The read-to-rename window is milliseconds and every writer is user-paced. The residual race is documented in docs/CONFIGURATION.md and accepted (T-kfc-02).
- [inferred] Auto-registration first probes a clone of the in-memory registry. Only when the probe finds something new does it touch disk, so the ~5s poll stays free of disk I/O as before. On an update error it keeps the old behavior: register in memory for the session.
- [inferred] Screens on a failed write change nothing in memory. The driver opt-in rollback (WR-04 revert) is deleted: memory now changes only by adopting what reached disk, so there is nothing to revert and no revert can mint a record. The WR-04 property holds by construction.
- [inferred] Removing an alias that is already gone from disk counts as success (a no-op write).
- [inferred] The opt-in direction is the one the user confirmed (read from memory before the update). A grant is refused when the on-disk entry's path differs from the path the user confirmed, because consent covers the files that were disclosed (D-14).
- [inferred] Reload: `App::reload_config_if_changed` rides the existing 20-tick session-poll counter, never a timer of its own, matching the block's rule. It runs inline: one `fs::metadata` call, plus a small read only when the (mtime, len) stamp changed, and the registry writes already do synchronous config I/O on this thread. A missing or unparsable file is not adopted. After the TUI's own write, the next reload is a harmless no-op adopt.
- [inferred] CLI add/remove (`src/main.rs:190-285`) and `registry::load_config_pruning_worktrees` are not changed. They load immediately before they save, hold no long-lived copy, and their window is the same as `update_config`'s.
- [inferred] Data-dir guard: structural (a `gsd-meta-manager` path component directly followed by `oneoff`, `driver` or `envelope`) OR under the canonical real `dirs::data_local_dir()/gsd-meta-manager`. The structural half catches a relocated data dir of another gsdmm process (the hx7-mask leak). The real-root half catches a symlinked app dir. `oneoff_root()` and `driver_root()` are linux/macos only and cannot be called from the portable registry, so a unix-only test pins them to the guard's constants. The guard is not a parameter: its structural half reads no environment, and the real data root never contains a test fixture. `GSD_MM_ENVELOPE_ROOT` is not consulted, so a stray override cannot refuse real projects.
- [inferred] The data-dir guard applies to session discovery only. Manual add and `gsd-meta-manager <dir>` stay as they are, by the reasoning jx7 used for the temp guard: the user named the folder.
- The user's real `~/.config/gsd-meta-manager/config.json` is never read, edited or written by the executor or by any test. The leaked `work*` entries in it stay. The SUMMARY tells the user to remove them.
</context>

<tasks>

<task type="tracer" tdd="true">
  <name>Task 1: Session auto-registration writes its additions onto the config on disk and adopts the result, end to end</name>
  <files>src/config.rs, src/ui/screens/mod.rs, src/app.rs</files>
  <behavior>
    - config tests (existing `mod tests` in src/config.rs, tempdir + std::fs): (1) an_update_applies_to_the_file_on_disk_not_to_a_stale_copy: disk {a, b}; another writer then removes a and adds c with `added` "2026-02-02"; update_config inserting d leaves disk {b, c, d}, c keeps "2026-02-02", a stays absent, and the returned config has the same keys; (2) a_refused_mutation_writes_nothing_and_returns_the_config_as_loaded: the closure inserts x and then returns Err; the inner result is Err, the returned config lacks x, and the file bytes are unchanged; (3) a_mutation_that_changes_nothing_leaves_the_file_untouched: the file is seeded as compact JSON with std::fs::write (not save_config's pretty form); a closure returning Ok(()) leaves the bytes identical; (4) an_unreadable_config_fails_before_the_mutation_runs: a directory at the config path gives an outer Err, and a flag the closure would set stays false; (5) a_missing_config_starts_empty_and_is_written_only_on_a_change: a path in a not-yet-existing subdir; a no-op update leaves the subdir absent; an insert creates the file holding the entry
    - mod.rs test adopting_a_registry_tracks_new_aliases_and_forgets_vanished_ones: ctx with {keep, gone}, where gone has project_states, last_refresh, owner_digests and detail_sub_view_per_project entries and is selected last; adopt {keep, new}: project_states gains new and loses gone, the other three maps lose gone, filtered_aliases are the sorted [keep, new], and the selection is in bounds
    - app.rs tracer test auto_registration_keeps_registry_edits_another_process_made: tempdir config seeded {a, b} with save_config, then App::new(path); another writer removes a and adds c ("2026-02-02"); the session cwd is a marked root (`.planning/STATE.md`) named fresh-proj; run auto_register_new_sessions_using with a register fn calling registry::auto_register_from_sessions_with_temp_root(.., None). Disk keys are {b, c, fresh-proj}, c's `added` is unchanged, a is absent; the in-memory keys match; project_states holds fresh-proj and c, not a; the status reads "Auto-registered: fresh-proj". At HEAD, a would come back and c would be lost
    - app.rs test auto_registration_still_registers_in_memory_when_the_config_cannot_be_read: the config path is a directory; the same session registers fresh-proj in memory with project state; the path is still a directory
  </behavior>
  <action>
Step 1 (tests first, from behavior; they must fail or fail to compile before Step 2): add the config tests to the existing `mod tests` in src/config.rs, the adopt test to the existing test module of src/ui/screens/mod.rs (reuse its AppContext fixture near line 2794), and the two App tests to the test module of src/app.rs. Plain #[test] is enough: App::new leaves event_tx and the watcher None, so nothing is spawned. Use #[tokio::test] only if a spawn needs a runtime. Never set an env var. Never use the real config path. The tracer goes through the injected register fn because the public auto-register entry refuses every tempdir root by design (jx7).

Step 2, src/config.rs: add Clone to Config's derive. Below save_config and above the test module, add update_config per the interfaces block. Steps: load_config(path) (missing gives Config::new(); unreadable or unparsable gives an outer Err and nothing is written); clone it; run mutate on the loaded config. On an inner Err, return the pre-mutation clone with that Err, so a partly applied refusal is neither returned nor written. On Ok, compare serde_json::to_value of the clone and the result (a serialization error is an outer Err with context). Equal: return without touching the filesystem. Different: save_config, the existing atomic tempfile + rename, which also creates the parent dir; then return. Doc comment: why it exists (a TUI holds a long-lived copy, and a whole-file write of it undid CLI add/remove, quick 261125-kfc); every registry writer in a long-lived process must use it; a missing file reads as empty; an unparsable file is never overwritten; and the residual race (no cross-process lock; two writes within the same few ms can still lose one) [inferred].

Step 3, src/ui/screens/mod.rs, inside `impl AppContext` and above the test module:
- track_project: watch `path/.planning` when it is a directory (tracing::warn! with alias, path and error on failure, message `registry: watcher.watch failed`); parse_project_state; change_tracker.record_initial; insert into project_states; then schedule_owner_digest. This order matters, because schedule_owner_digest reads the state entry. Lift these steps from the per-alias block of auto_register_new_sessions (src/app.rs 2705-2720).
- forget_project: the cleanup block of delete_confirm::do_remove_project (src/ui/screens/delete_confirm.rs 163-181): unwatch `path/.planning`; remove the alias from project_states, detail_sub_view_per_project, last_refresh and owner_digests; reset the view_cache entry's owner_block. Keep that block's comments about the immediate path and the prune backstop.
- adopt_config(fresh): swap fresh in for self.config and keep the old one. An alias is vanished when it is absent from fresh or its path changed; call forget_project with its old path. An alias is new when it is absent from the old config or its path changed; call track_project. When the alias set changed, call recompute_filtered_aliases, clamp the selection exactly as do_remove_project does today (None when the list is empty, else at most len - 1), and set needs_redraw. Entries whose other fields changed (an opt-in) are simply replaced. Doc comment: the one way the TUI's registry changes after startup is by adopting a config that was read from or written to disk (quick 261125-kfc).

Step 4, src/app.rs:
- auto_register_new_sessions keeps its signature and body-delegates to the new private auto_register_new_sessions_using(registry::auto_register_from_sessions). A type alias for the fn type is fine if clippy flags its complexity.
- auto_register_new_sessions_using: (a) clone self.ctx.config into a probe and run register on it against self.active_sessions; an empty result returns at once, so no disk I/O happens on an idle poll. (b) Call config::update_config on self.ctx.config_path with a closure that runs register on the loaded config and returns it as Ok with std::convert::Infallible as the error (the result unpacks with an irrefutable `let Ok(..) =`, fine at MSRV 1.89). On Ok, call self.ctx.adopt_config(fresh): it tracks this poll's additions and any external changes. (c) On an outer Err, warn (`auto-register: config update failed; registering in memory only`, with the error), put the probe in as self.ctx.config, call track_project for each probe addition, then recompute_filtered_aliases. This keeps the old "in-memory registration still helps this session" behavior. (d) For each alias this call added, keep today's tracing::info! line and status message, then set self.needs_redraw. Remove the old per-alias save loop and its watcher/state block, which now live in track_project. Rewrite the doc comment to say how the additions are persisted and why.
- The CreateProjectResult save site stays for Task 2, so the save_config import stays for now.

Run `rtk proxy cargo fmt`, then the verify command. Commit with `git commit -- src/config.rs src/ui/screens/mod.rs src/app.rs` (never `git add -A`). Message: `fix(quick-261125-kfc): session auto-registration applies its additions to the config on disk`, ending with the session's Co-Authored-By and Claude-Session trailers. Tests and fix go in one commit; never commit a non-compiling RED state.
  </action>
  <verify>
    <automated>rtk proxy cargo test --lib --no-fail-fast -- config::tests ui::screens::tests app::tests && test "$(rtk bypass grep -c 'pub fn update_config' src/config.rs)" -ge 1 && test "$(rtk bypass grep -c 'pub fn adopt_config' src/ui/screens/mod.rs)" -ge 1 && test "$(rtk bypass grep -c 'fn auto_register_new_sessions_using' src/app.rs)" -ge 1 && test "$(rtk bypass grep -c 'update_config(' src/app.rs)" -ge 1 && test "$(rtk bypass grep -c 'fn auto_registration_keeps_registry_edits_another_process_made' src/app.rs)" -ge 1</automated>
    <!-- Baselines measured at HEAD 2d0be8ac -->
    <!-- update_config def in src/config.rs [HEAD=0 EXPECT>=1 @2d0be8ac] -->
    <!-- adopt_config def in src/ui/screens/mod.rs [HEAD=0 EXPECT>=1 @2d0be8ac] -->
    <!-- auto_register_new_sessions_using def [HEAD=0 EXPECT>=1 @2d0be8ac] -->
    <!-- update_config call in src/app.rs [HEAD=0 EXPECT>=1 @2d0be8ac] -->
    <!-- tracer test fn [HEAD=0 EXPECT>=1 @2d0be8ac] -->
  </verify>
  <done>The 8 new tests and every existing config::, ui::screens:: (mod.rs) and app:: test pass; an external remove survives a TUI auto-register write and an external add is kept and adopted; committed on feat/v2.5.0-owner-inbox.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Every other TUI registry write goes through update_config, and the session poll adopts external edits</name>
  <files>src/ui/screens/add_project.rs, src/ui/screens/delete_confirm.rs, src/ui/screens/driver_confirm.rs, src/app.rs, src/main.rs</files>
  <behavior>
    - add_project.rs: the_add_screen_keeps_an_entry_another_process_added: disk {x}; another writer adds ext; do_add_project adds new. Disk and memory both hold {x, ext, new}, and project_states holds new and ext. The existing linked-worktree refusal test still passes unchanged (the refusal is reported before any write)
    - delete_confirm.rs: removing_a_project_keeps_an_entry_another_process_added (disk {proj}; another writer adds ext; delete proj: disk and memory {ext}); removing_a_project_another_process_already_removed_succeeds (disk {proj}; another writer removes it; delete proj: no error, the toast reads Removed, memory is empty, the file bytes are unchanged)
    - driver_confirm.rs: the_opt_in_toggle_keeps_an_entry_another_process_added (another writer adds ext; the grant leaves disk with ext and proj opted in); a_grant_is_refused_when_the_project_was_repointed_on_disk (another writer changes proj's path; the toggle shows an error and records nothing on disk or in memory). The four existing failed-save tests keep their assertions; only the unwritable-path helper changes
    - app.rs: the_session_poll_adopts_a_registry_another_process_changed: App::new with disk {a}; another writer removes a and adds c, changing the file length; reload_config_if_changed leaves memory {c} with project_states holding c, not a. A second call with the file unchanged reloads nothing: a sentinel inserted into memory survives it. Second test, a_registry_that_vanishes_or_stops_parsing_is_not_adopted: invalid JSON, then a deleted file; memory keeps {a} both times
  </behavior>
  <action>
Write the behavior tests first in each file's existing test module (std::fs only, tempdir config paths, no env mutation). Then migrate the sites. Each site follows the same shape: call config::update_config on ctx.config_path with a closure that runs the site's existing registry call. On an outer Err, show `Failed to save config: {e}` and change nothing in memory. On an inner Err, call ctx.adopt_config(fresh) and then show the site's existing refusal message. On success, call ctx.adopt_config(fresh) and keep only what adopt_config does not do (toasts, selecting the new row). Remove each file's top-level save_config import and every production call to it.

- src/ui/screens/add_project.rs do_add_project: the closure runs registry::add_project with the judged alias and canonical path. Drop the site's own parse/insert/schedule_owner_digest lines, because track_project now does them and also adds the `.planning/` watch the screen never had. Keep the status message, the input clear, the recompute and the selection of the new row.
- src/ui/screens/delete_confirm.rs do_remove_project: the live-run refusal still comes first and untouched. The closure returns Ok(()) when the raw alias is no longer a key of the loaded registry [inferred]; otherwise it runs registry::remove_project with the LegacyRegistryKey, keeping that call's comments. Drop the project_path capture and the cleanup block (forget_project via adopt does it) and the selection clamp (adopt does it). Keep the escaped "Removed" toast.
- src/ui/screens/driver_confirm.rs do_toggle_opt_in: before the update, read was_opted_in and the alias's path from memory: that is what the user confirmed. The closure refuses with an anyhow error when the loaded entry exists with a different path [inferred, D-14]. The message is escaped with crate::text::render_for_terminal like the delete toast, and says the project changed on disk and nothing was recorded. Otherwise it runs clear_opt_in when was_opted_in, else record_opt_in. Delete the `previous` capture, the whole failed-save revert block and the note appended when the in-memory registry cannot be put back. Rewrite the function's doc comment: the toggle is applied to a fresh read and memory adopts only what reached disk, so a failed save leaves memory as it was, no revert exists, and record_opt_in stays the sole constructor of a record (WR-04 and D-14 hold by construction). The launcher still reads config.json from disk, which is why the order matters. Tests: make ctx_with_project persist its config with crate::config::save_config at the end, so every fixture user models production (memory mirrors disk); make grant persist too, ignoring the result where the path is unwritable. Change unwritable_config_path to create a directory where config.json must be: reading it fails for every user, root included, before any mutation runs. Rewrite that helper's doc comment to match, and keep the root-CI reason.
- src/app.rs CreateProjectResult: the closure runs registry::add_project_unchecked with the judged alias and path. An inner Err gives `Failed to register: {e}`, after adopting. Keep the `.planning`-not-yet-a-directory poll branch. Drop the is_dir watch (track_project watched it) and the parse/insert/schedule block. Keep the "Created project" toast and the selection. Remove the save_config import.
- src/app.rs reload: add the App field config_stamp, initialized in from_config from the free fn config_stamp of the config path. config_stamp returns std::fs::metadata's modified time and len, or None when either is unavailable. Add App::reload_config_if_changed, which: takes the current stamp; returns when it equals the stored one; stores it; returns when it is None (a missing file is not adopted); calls config::load_config; on Ok, calls self.ctx.adopt_config and sets self.needs_redraw when ctx.needs_redraw is set; on Err, tracing::warn! `registry: config.json changed on disk but could not be loaded; keeping the in-memory registry` with the error. Call it in the 20-tick session-poll block right before self.prune_driver_maps(), so the prune sees the reloaded registry. Add a comment: this rides the same counter as everything in the block, never a timer of its own; it runs inline because it is one stat plus, only on change, the small read every registry write already does on this thread.
- src/main.rs launch-target registration (lines 500-517): replace the in-memory call and its save with config::update_config on app.ctx.config_path, with a closure returning Ok::<_, std::convert::Infallible> of launch_target::auto_register on the loaded config. Propagate an outer Err with `?` as the old save did. Assign the returned config to app.ctx.config directly, not through adopt_config: no state, watcher or digest exists yet, and load_project_states and the watcher setup below build them. Then match the registered alias exactly as before (None keeps the error exit). Leave the CLI add/remove arms and their save_config calls untouched [inferred, see context].

Existing tests elsewhere that build a registry only in memory and then trigger one of these writes now see the disk instead (for example `removing_a_project_interactively_clears_its_per_alias_maps`). Seed such a fixture on disk with crate::config::save_config. Do not weaken an assertion. If that forces an edit in a file not listed here, record it in the SUMMARY as a deviation. A fixture whose config path is relative or shared (for example detail.rs's relative path) must not start writing: switch that test to a tempdir path.

Run `rtk proxy cargo fmt`, then the verify command. Commit with `git commit -- src/ui/screens/add_project.rs src/ui/screens/delete_confirm.rs src/ui/screens/driver_confirm.rs src/app.rs src/main.rs` plus any deviation file. Message: `fix(quick-261125-kfc): every TUI registry write re-reads config.json; the session poll adopts external edits`, ending with the session's trailers.
  </action>
  <verify>
    <automated>rtk proxy cargo test --lib --no-fail-fast -- ui::screens:: app::tests &&
rtk proxy cargo check --bins &&
test "$(rtk bypass grep -c 'update_config(' src/main.rs)" -ge 1 &&
test "$(rtk bypass grep -c 'fn reload_config_if_changed' src/app.rs)" -ge 1 &&
test "$(rtk bypass grep -v '^\s*//' src/main.rs | rtk bypass grep -c 'save_config(')" -le 2 &&
test "$(rtk bypass awk '/^#\[cfg\(test\)\]/{exit} /save_config\(/ && !/^[ \t]*\/\//{n++} END{print n+0}' src/app.rs)" -eq 0 &&
test "$(rtk bypass awk '/^#\[cfg\(test\)\]/{exit} /save_config\(/ && !/^[ \t]*\/\//{n++} END{print n+0}' src/ui/screens/add_project.rs)" -eq 0 &&
test "$(rtk bypass awk '/^#\[cfg\(test\)\]/{exit} /save_config\(/ && !/^[ \t]*\/\//{n++} END{print n+0}' src/ui/screens/delete_confirm.rs)" -eq 0 &&
test "$(rtk bypass awk '/^#\[cfg\(test\)\]/{exit} /save_config\(/ && !/^[ \t]*\/\//{n++} END{print n+0}' src/ui/screens/driver_confirm.rs)" -eq 0 &&
test "$(rtk bypass awk '/^#\[cfg\(test\)\]/{exit} /could not be restored/{n++} END{print n+0}' src/ui/screens/driver_confirm.rs)" -eq 0 &&
test "$(rtk bypass grep -c '^use crate::config::.*save_config' src/app.rs src/ui/screens/add_project.rs src/ui/screens/delete_confirm.rs src/ui/screens/driver_confirm.rs | rtk bypass awk -F: '{s+=$2} END {print s+0}')" -eq 0</automated>
    <!-- Baselines measured at HEAD 2d0be8ac; production region = lines before the file's first column-0 #[cfg(test)]. The src/app.rs count drops by one in Task 1; the two CLI calls in src/main.rs stay. -->
    <!-- save_config call, production region of src/app.rs [HEAD=2 EXPECT==0 @2d0be8ac] -->
    <!-- save_config call, production region of add_project.rs [HEAD=1 EXPECT==0 @2d0be8ac] -->
    <!-- save_config call, production region of delete_confirm.rs [HEAD=1 EXPECT==0 @2d0be8ac] -->
    <!-- save_config call, production region of driver_confirm.rs [HEAD=1 EXPECT==0 @2d0be8ac] -->
    <!-- save_config call lines outside comments in src/main.rs [HEAD=3 EXPECT<=2 @2d0be8ac] -->
    <!-- update_config call in src/main.rs [HEAD=0 EXPECT>=1 @2d0be8ac] -->
    <!-- reload_config_if_changed def [HEAD=0 EXPECT>=1 @2d0be8ac] -->
    <!-- in-memory-restore note, production region of driver_confirm.rs [HEAD=1 EXPECT==0 @2d0be8ac] -->
    <!-- top-level save_config imports across the four files [HEAD=4 EXPECT==0 @2d0be8ac] -->
  </verify>
  <done>All ui::screens:: and app:: tests pass, including the 7 new ones and the four unchanged failed-save assertions; the binary compiles; no TUI write site calls save_config any more; committed.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 3: Never auto-register a root inside gsd-meta-manager's own data dir; document both fixes; full gate</name>
  <files>src/registry.rs, README.md, docs/GETTING-STARTED.md, docs/CONFIGURATION.md</files>
  <behavior>
    - a_session_in_a_one_off_or_driver_tree_is_never_auto_registered: each of `<t>/data/gsd-meta-manager/oneoff/abcd1234/oneoff-261125-x/work`, `.../oneoff-261125-x/land/1/work`, `<t>/data/gsd-meta-manager/driver/abcd1234/run-1/mirror` and `<t>/data/gsd-meta-manager/envelope/proj` holds `.planning/ROADMAP.md`; auto_register_from_sessions_with_temp_root(.., None) registers none of them and the config stays empty
    - a_project_merely_named_like_gsd_meta_manager_still_registers: marked roots `<t>/src/gsd-meta-manager` (alias gsd-meta-manager), `<t>/gsd-meta-manager-x/oneoff/p` and `<t>/oneoff/gsd-meta-manager/p` each register once (separate configs), proving the match is on whole components, in order, with the run-tree name directly after
    - the_real_data_dir_guard_is_component_wise: with data_root Some(`<t>/relocated`), is_in_gsdmm_data_dir is true for `<t>/relocated` and `<t>/relocated/x/work`, false for `<t>/relocated-2/p` and `<t>/other/p`; with None it is false for `<t>/other/p`
    - #[cfg(any(target_os = "linux", target_os = "macos"))] the_guard_names_the_trees_the_run_locators_use: crate::oneoff::store::oneoff_root() and crate::launcher::config_dir::driver_root() (each expect: CI has a data dir) joined with `abcd1234/run-1/work` satisfy is_in_gsdmm_data_dir(.., None), so renaming either tree breaks this test, not production
  </behavior>
  <action>
Step 1 (tests first, from behavior): add the four tests to the existing `gsd_root_guard_tests` module in src/registry.rs, which sits above `mod tests` (std::fs only, no env mutation, reuse its session helper).

Step 2, src/registry.rs, next to temp_guard_root, system_temp_root and is_under_temp_root:
- Add GSDMM_DATA_SUBDIR and GSDMM_RUN_TREES per the interfaces block. Doc them as the app subdir of the platform data dir and the trees gsdmm creates under it (one-off clones, the driver's per-run state and push mirrors, the envelope ledgers). Name the locators they mirror (oneoff::store::oneoff_root, launcher::config_dir::driver_root, envelope::envelope_root) and the pin test.
- Add gsdmm_data_root: dirs::data_local_dir() joined with GSDMM_DATA_SUBDIR, through canon_or_raw.
- Add is_in_gsdmm_data_dir. It is true when root's Path::components holds a Normal component equal to GSDMM_DATA_SUBDIR directly followed by one equal to a GSDMM_RUN_TREES entry, or when data_root is Some and root starts_with it (component-wise). Doc comment: the structural half catches another gsdmm process's relocated data dir (the test harness's `target/tmp/hx7-mask` clone that leaked as `work`); the real-root half catches a symlinked app dir; it is not a parameter of the injectable entry because its structural half reads no environment and the real data root never holds a test fixture [inferred]; GSD_MM_ENVELOPE_ROOT is deliberately not consulted.
- In auto_register_from_sessions_with_temp_root: compute gsdmm_data_root() once, before the session loop. Right after the temp-dir check, skip a root for which is_in_gsdmm_data_dir holds, with tracing::debug! carrying `root` and `data_root` fields and the message `auto-register: skipping root inside gsd-meta-manager's own data dir`. Judge the RESOLVED root, as the other guards do. Update both auto-register doc comments: the root must also lie outside gsd-meta-manager's own data dir (quick 261125-kfc). launch_target and add_project are not changed.
Run `rtk proxy cargo fmt` and `rtk proxy cargo test --lib --no-fail-fast -- registry::`. Commit with `git commit -- src/registry.rs`, message `fix(quick-261125-kfc): never auto-register a root inside gsd-meta-manager's own data dir`, with the session's trailers.

Step 3, docs (US spelling, plain prose, no other edits):
- README.md Quick start step 4: after the temp-dir sentence ending "`add` still registers them.", add one sentence. It says folders inside gsd-meta-manager's own data dir (`<data dir>/gsd-meta-manager/`, typically `~/.local/share/gsd-meta-manager/`) are never auto-registered either, nor any folder under a `gsd-meta-manager/oneoff/`, `gsd-meta-manager/driver/` or `gsd-meta-manager/envelope/` directory elsewhere, so the private clones one-off runs work in never show up as `work`, `work-2`, and so on. The phrase gsd-meta-manager's own data dir must appear verbatim; a gate greps for it.
- README.md Configuration section, after "Override the config location with `--config <PATH>`.": say that each registry change a running TUI makes (auto-registration, adding, removing, a driver opt-in) re-reads `config.json` and writes back only that change, so an `add` or `remove` from the command line or another TUI is never undone, and that the TUI picks such edits up within one session poll (about 5 seconds). Keep the phrases re-reads `config.json` and within one session poll verbatim.
- docs/GETTING-STARTED.md first-launch bullet: the same data-dir sentence in the bullet's voice, with the same verbatim phrase.
- docs/CONFIGURATION.md line 677 Atomic writes bullet: keep the tempfile + rename sentence, naming update_config and save_config. Replace the closing sentence about uncoordinated concurrent invocations with these points: every registry write re-reads `config.json` and applies only its own change, whether from the TUI or the CLI; the TUI also adopts edits other processes make, within one session poll; writers are not locked against each other, so two writes landing within the same few milliseconds can still lose one; a config.json that does not parse is never overwritten. At line 719 add update_config to the `src/config.rs` function list.
Commit with `git commit -- README.md docs/GETTING-STARTED.md docs/CONFIGURATION.md`, message `docs(quick-261125-kfc): registry writes keep external edits; one-off clones are never auto-registered`, with the session's trailers.

Step 4: run the verify command; it is the plan's final gate. It writes the whole suite's raw output to target/kfc-final.log (target/ is ignored, so git status stays clean); never judge test results from rtk-filtered output. While the suite runs, an older gsdmm TUI the user has open may auto-register the one-off fixture clones into the real config. That is expected and the reason for this fix; it is not a test failure. Do not read or edit the real config. If a gate fails, fix it in a further path-scoped commit and rerun.
  </action>
  <verify>
    <automated>rtk proxy cargo fmt --check &&
rtk proxy cargo clippy --all-targets -- -D warnings &&
rtk proxy cargo clippy --locked --target x86_64-pc-windows-gnu --all-targets -- -D warnings &&
rtk proxy cargo test --no-fail-fast > target/kfc-final.log 2>&1 &&
test "$(rtk bypass grep -c '^test result: ok' target/kfc-final.log)" -ge 69 &&
test "$(rtk bypass awk '/^test result:/ {s+=$4} END {print s+0}' target/kfc-final.log)" -ge 4715 &&
test "$(rtk bypass grep -c "auto-register: skipping root inside gsd-meta-manager's own data dir" src/registry.rs)" -ge 1 &&
test "$(rtk bypass grep -c 'fn is_in_gsdmm_data_dir' src/registry.rs)" -ge 1 &&
test "$(rtk bypass grep -c 'fn the_guard_names_the_trees_the_run_locators_use' src/registry.rs)" -ge 1 &&
test "$(rtk bypass grep -c "gsd-meta-manager's own data dir" README.md)" -ge 1 &&
test "$(rtk bypass grep -c "gsd-meta-manager's own data dir" docs/GETTING-STARTED.md)" -ge 1 &&
test "$(rtk bypass grep -cF 're-reads `config.json`' README.md)" -ge 1 &&
test "$(rtk bypass grep -cF 'within one session poll' README.md)" -ge 1 &&
test "$(rtk bypass grep -cF 're-reads `config.json`' docs/CONFIGURATION.md)" -ge 1 &&
test "$(rtk bypass grep -c 'update_config' docs/CONFIGURATION.md)" -ge 1 &&
test "$(rtk bypass grep -c '^test result: FAILED' target/kfc-final.log)" -eq 0 &&
test "$(rtk bypass grep -cF 'the last writer wins' docs/CONFIGURATION.md)" -eq 0 &&
test -z "$(git status --porcelain -- src tests README.md docs Cargo.toml Cargo.lock)"</automated>
    <!-- Suite baselines from target/jx7-final.log at a363aeab (src/tests unchanged through HEAD 2d0be8ac) -->
    <!-- ok suites [HEAD=69 EXPECT>=69 @a363aeab GUARD MUTATION: a test binary that stops running] -->
    <!-- FAILED suites [HEAD=0 EXPECT==0 @a363aeab GUARD MUTATION: a fixture left memory-only after the migration] -->
    <!-- passed sum, baseline plus the 19 new tests of Tasks 1-3 with slack of 5 [HEAD=4701 EXPECT>=4715 @a363aeab] -->
    <!-- data-dir skip log in src/registry.rs [HEAD=0 EXPECT>=1 @2d0be8ac] -->
    <!-- is_in_gsdmm_data_dir def [HEAD=0 EXPECT>=1 @2d0be8ac] -->
    <!-- locator pin test fn [HEAD=0 EXPECT>=1 @2d0be8ac] -->
    <!-- data-dir phrase in README.md [HEAD=0 EXPECT>=1 @2d0be8ac] -->
    <!-- data-dir phrase in docs/GETTING-STARTED.md [HEAD=0 EXPECT>=1 @2d0be8ac] -->
    <!-- re-read phrase in README.md [HEAD=0 EXPECT>=1 @2d0be8ac] -->
    <!-- session-poll phrase in README.md [HEAD=0 EXPECT>=1 @2d0be8ac] -->
    <!-- re-read phrase in docs/CONFIGURATION.md [HEAD=0 EXPECT>=1 @2d0be8ac] -->
    <!-- old race sentence in docs/CONFIGURATION.md [HEAD=1 EXPECT==0 @2d0be8ac] -->
    <!-- update_config in docs/CONFIGURATION.md [HEAD=0 EXPECT>=1 @2d0be8ac] -->
    <!-- cargo fmt --check exits 0 at HEAD 2d0be8ac and must still exit 0 (not a count gate) -->
  </verify>
  <done>One-off, land, driver and envelope trees are never auto-registered while same-named projects still are; the docs state both fixes; fmt, clippy -D warnings (host and windows-gnu) and the full no-fail-fast suite are green with 69 suites, 0 failed, at least 4715 passed; src, tests, docs and Cargo files are fully committed.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| other gsdmm processes → config.json | CLI add/remove, other TUIs and the user's editor rewrite the registry while a TUI holds a copy |
| any local process named `claude` → registry | session detection trusts the process name and cwd system-wide; a gsdmm one-off run (or its test harness) starts one in a clone it owns |
| config.json → driver opt-in | the launcher reads opt-in records from disk in another process; a record must reflect a deliberate user act over the disclosed files (D-14) |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-kfc-01 | Tampering | TUI registry writes (config::update_config callers) | high | mitigate | Every TUI write re-reads config.json and re-runs only its own registry operation; the TUI adopts the result. Pinned by the Task 1 config and tracer tests and the Task 2 per-screen tests (external remove stays removed, external add survives with its `added`) |
| T-kfc-02 | Tampering | concurrent writers within one read-to-rename window | low | accept | No cross-process lock (rationale in context decisions); the window is milliseconds and writers are user-paced; documented in docs/CONFIGURATION.md |
| T-kfc-03 | Elevation of privilege | driver_confirm::do_toggle_opt_in | high | mitigate | The grant is applied to the fresh entry only when its path equals the one the user confirmed, else refused (Task 2 test); memory changes only by adopting what reached disk, so no revert can mint a record (WR-04, D-14); record_opt_in stays the sole constructor |
| T-kfc-04 | Tampering | registry::auto_register_from_sessions_with_temp_root | medium | mitigate | is_in_gsdmm_data_dir refuses one-off clones, land attempts, driver and envelope trees, structurally and under the real data root, so an agent-written `.planning/` in a gsdmm-owned clone never becomes a registered (and later drivable) project; Task 3 tests incl. no-false-positive and locator pin |
| T-kfc-05 | Denial of service | config.json that does not parse (hand edit) | medium | mitigate | update_config returns an outer error and never overwrites it (the old whole-file save did); the reload keeps the in-memory registry; auto-registration falls back to memory only |
| T-kfc-06 | Information disclosure | new debug/warn log lines | low | accept | Same fields (paths, aliases, error text) as the existing registry log lines, written only to the local log file |
| T-kfc-SC | Tampering | npm/pip/cargo installs | high | accept | No package is added or installed; Cargo.toml and Cargo.lock are not in files_modified |
</threat_model>

<verification>
- Task 3's verify is the plan-wide gate: rustfmt clean; clippy -D warnings clean on the host and on x86_64-pc-windows-gnu (the target whose dead-code lints killed v2.2.0/v2.3.0); `cargo test --no-fail-fast` raw log with 69 ok suites, 0 FAILED, at least 4715 passed.
- `git status` is clean after the four commits (target/ logs are ignored).
- Source audit (no ROADMAP phase, REQUIREMENTS, RESEARCH or CONTEXT exist for a quick task; the brief is the source):
  - bug 1 "TUI writes must not discard external changes": Tasks 1 and 2.
  - "re-read and apply only this process's delta, refresh in memory": update_config + adopt_config (Task 1), at every site (Task 2).
  - "and/or reload when config.json changes": reload_config_if_changed (Task 2).
  - "find ALL TUI save sites": survey list, each migrated (Tasks 1-2); CLI/prune excluded with rationale.
  - "keep writes atomic": save_config's tempfile + rename is still the only writer.
  - the required test "external remove + auto-register save keeps it removed; external add survives": the Task 1 tracer.
  - bug 2 "never auto-register gsdmm's oneoff tree, by structure and/or real data dir, reuse oneoff's locator, consider driver/envelope, log at debug": Task 3.
  - "investigate the test leak, fix isolation if broken": root cause confirmed in context (hermetic test, observed by the user's live TUI), no isolation change needed; SUMMARY records it.
  - "update README / GETTING-STARTED": Task 3 (plus CONFIGURATION.md, whose race sentence becomes false).
  - "verification commands, never touch the real config, path-scoped commits, no push": every task.
  - Nothing is unplanned.
</verification>

<success_criteria>
- A CLI `remove` or `add` made while a TUI runs is never undone by any TUI write, and the TUI shows it within one session poll
- A failed or refused TUI write leaves disk and memory as they were and says why; a hand-broken config.json is never overwritten
- gsdmm's own one-off clones (and driver/envelope trees) are never auto-registered, wherever the data dir lives; real projects of any name still are
- Every pre-existing test still passes with its original claim; docs describe both fixes; full suite, clippy (host, windows-gnu) and fmt green
</success_criteria>

<output>
Create `.planning/quick/261125-kfc-fix-tui-registry-lost-update-on-config-j/261125-kfc-SUMMARY.md` when done. Include the four commit shas, the final suite counts from target/kfc-final.log, the test-leak finding (the test is hermetic; the user's running TUI observed its fake `claude` system-wide; the data-dir guard is the fix), every deviation file, and this note for the user: install the new build and restart every running gsdmm TUI first, because an older TUI still rewrites its whole copy and would undo the cleanup. Then run `gsd-meta-manager list` and `gsd-meta-manager remove <alias>` for each leaked one-off entry (aliases `work`, `work-2`, ... whose path runs through `gsd-meta-manager/oneoff/`). The executor did not touch the real config.
</output>
