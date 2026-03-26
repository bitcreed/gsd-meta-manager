---
phase: quick
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/main.rs
  - src/action.rs
  - src/app.rs
  - src/state_reader/mod.rs
  - src/state_reader/config_json.rs
  - Cargo.toml
autonomous: true
requirements: []
must_haves:
  truths:
    - "tracing::warn!() calls produce output to a log file"
    - "No compiler warnings for dead Action variants"
    - "Removing a project calls unwatch on its .planning/ directory"
    - "Removing a project cleans up detail_sub_view_per_project entry"
    - "Newly created projects get watched after .planning/ appears"
  artifacts:
    - path: "src/main.rs"
      provides: "tracing_subscriber init with file appender"
      contains: "tracing_subscriber"
    - path: "src/action.rs"
      provides: "Clean Action enum without dead variants"
    - path: "src/app.rs"
      provides: "unwatch + cleanup on removal, deferred watcher for new projects"
  key_links:
    - from: "src/main.rs"
      to: "tracing-appender"
      via: "file appender to ~/.local/share/gsd-manager/gsd-manager.log"
      pattern: "tracing_appender"
    - from: "src/app.rs do_remove_project"
      to: "src/watcher.rs unwatch"
      via: "self.watcher.unwatch(planning_dir)"
      pattern: "unwatch"
---

<objective>
Fix 6 tech debt items identified in the v1.0 milestone audit.

Purpose: Eliminate silent log drops, inotify watch leaks, dead code warnings, and stale state accumulation.
Output: Clean codebase with all audit items resolved.
</objective>

<execution_context>
@$HOME/.claude/get-shit-done/workflows/execute-plan.md
@$HOME/.claude/get-shit-done/templates/summary.md
</execution_context>

<context>
@.planning/v1.0-MILESTONE-AUDIT.md
@src/main.rs
@src/action.rs
@src/app.rs
@src/state_reader/mod.rs
@src/state_reader/config_json.rs
@src/watcher.rs
@Cargo.toml
</context>

<tasks>

<task type="auto">
  <name>Task 1: Init tracing subscriber, remove dead Action variants, remove unused gsd_mode field</name>
  <files>src/main.rs, src/action.rs, src/app.rs, src/state_reader/mod.rs, src/state_reader/config_json.rs</files>
  <action>
1. **Initialize tracing subscriber in main.rs** (before any other code in `main()`):
   - Create a file appender using `tracing_appender::rolling::daily` targeting `~/.local/share/gsd-manager/` with filename `gsd-manager.log`. Use `dirs::data_local_dir()` to get the platform-appropriate path, falling back to `/tmp` if unavailable.
   - Initialize `tracing_subscriber` with the file appender as the writer: `tracing_subscriber::fmt().with_writer(file_appender).with_ansi(false).init();`
   - Place this BEFORE `color_eyre::install()` so tracing is available immediately.
   - The `_guard` from the appender (if using non-blocking) must be held for the lifetime of main. Use the blocking daily rolling appender which does not require a guard.

2. **Remove dead Action variants from action.rs**:
   - Delete `Action::Quit` — quit is handled via `should_quit = true` in key handlers, never sent as an action.
   - Delete `Action::AddProjectConfirm { alias, path }` — add is handled inline in key handlers via `do_add_project()`.
   - Delete `Action::RemoveProjectConfirm { alias }` — remove is handled inline via `do_remove_project()`.
   - Delete `Action::ProjectLoaded { alias, state }` — project loading is synchronous, never sent as an action.

3. **Remove match arms in app.rs `update()`** for the deleted variants (lines 223-225 for Quit, 233-235 for AddProjectConfirm, 236-238 for RemoveProjectConfirm, 332-337 for ProjectLoaded).

4. **Remove gsd_mode field from ProjectState** in `src/state_reader/mod.rs`:
   - Delete `pub gsd_mode: String` from the struct (line 19).
   - Delete `state.gsd_mode = config.mode;` (line 65).
   - Remove the entire config.json parsing block (lines 62-67) since `gsd_mode` was the only consumer. Keep the `config_json` module and its tests intact for future use.
  </action>
  <verify>
    <automated>cd /home/blk/projects/rust/gsd-manager && cargo build 2>&1 | grep -E "warning|error" | head -20; cargo test 2>&1 | tail -5</automated>
  </verify>
  <done>No compiler warnings for removed variants. Tracing subscriber initialized at startup. gsd_mode field removed. All existing tests pass.</done>
</task>

<task type="auto">
  <name>Task 2: Fix watcher lifecycle — unwatch on removal, deferred watch for new projects, clean detail_sub_view</name>
  <files>src/app.rs</files>
  <action>
1. **Call unwatch on project removal** in `do_remove_project()` (around line 896):
   - Before `self.project_states.remove(alias)`, look up the project path from `self.config.projects.get(alias)` (must do this BEFORE `registry::remove_project` removes it from config).
   - Restructure: capture `project_path = self.config.projects.get(alias).map(|p| p.path.clone())` BEFORE calling `registry::remove_project`.
   - After successful removal, if `project_path` is Some, call:
     ```rust
     if let Some(ref mut watcher) = self.watcher {
         let planning_dir = project_path.join(".planning");
         let _ = watcher.unwatch(&planning_dir);
     }
     ```
   - Use `let _ =` because the unwatch may fail if .planning/ didn't exist (not an error).

2. **Clean detail_sub_view_per_project on removal** in `do_remove_project()`:
   - After `self.project_states.remove(alias)`, add: `self.detail_sub_view_per_project.remove(alias);`
   - Also clean `self.last_refresh.remove(alias);` while we're at it (same leak pattern).

3. **Fix new project watcher guard** in `CreateProjectResult` handler (around line 298-304):
   - The current code checks `if planning_dir.is_dir()` but .planning/ doesn't exist yet at project creation time.
   - Replace the immediate watch attempt with a spawned async task that polls for .planning/ to appear:
     ```rust
     // .planning/ may not exist yet (GSD creates it later).
     // Spawn a background task that watches for it to appear, then starts watching.
     if let Some(ref watcher_tx) = self.event_tx {
         let planning_dir = path.join(".planning");
         let tx = watcher_tx.clone();
         tokio::spawn(async move {
             // Poll every 2 seconds for up to 60 seconds
             for _ in 0..30 {
                 if planning_dir.is_dir() {
                     // Send a FileChanged to trigger a refresh (watcher will be set up on next add)
                     let _ = tx.send(Action::FileChanged {
                         project_path: planning_dir.parent().unwrap().to_path_buf(),
                     });
                     break;
                 }
                 tokio::time::sleep(std::time::Duration::from_secs(2)).await;
             }
         });
     }
     ```
   - Also: in the `FileChanged` handler, after re-parsing state, check if the project is not yet being watched and start watching:
     ```rust
     // Auto-start watcher if not yet watching (handles new project case)
     let planning_dir_check = project_path.join(".planning");
     if planning_dir_check.is_dir() {
         if let Some(ref mut watcher) = self.watcher {
             // watch() is idempotent for notify — re-watching an already-watched path is a no-op
             let _ = watcher.watch(&planning_dir_check);
         }
     }
     ```
     Add this inside the `if let Some(alias) = alias {` block, after `self.needs_redraw = true;` (end of the FileChanged handler).
  </action>
  <verify>
    <automated>cd /home/blk/projects/rust/gsd-manager && cargo build 2>&1 | grep -E "warning|error" | head -20; cargo test 2>&1 | tail -5</automated>
  </verify>
  <done>Removing a project calls unwatch and cleans detail_sub_view_per_project + last_refresh. New projects get watched when .planning/ appears via deferred polling + auto-watch on FileChanged. No compiler warnings. All tests pass.</done>
</task>

</tasks>

<verification>
- `cargo build 2>&1 | grep warning` produces zero warnings related to dead code or unused fields
- `cargo test` — all existing tests pass
- `cargo clippy 2>&1 | grep warning` — no new clippy warnings
- Manual: run `cargo run`, check `~/.local/share/gsd-manager/gsd-manager.log` exists and receives log entries
</verification>

<success_criteria>
All 6 tech debt items from v1.0 audit resolved:
1. tracing_subscriber initialized — warn!() calls write to log file
2. Dead Action variants removed — no compiler warnings
3. FileWatcher::unwatch called on project removal — no inotify leaks
4. New project watcher deferred — .planning/ watched when it appears
5. gsd_mode field removed — no unused parsed data
6. detail_sub_view_per_project cleaned on removal — no stale entries
</success_criteria>

<output>
After completion, create `.planning/quick/260325-reh-fix-6-tech-debt-items-from-v1-0-audit-in/260325-reh-SUMMARY.md`
</output>
