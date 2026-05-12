---
quick_id: 260512-epe
description: Auto-detect and register GSD projects from active claude sessions
date: 2026-05-12
mode: quick
must_haves:
  truths:
    - "Detection source is the existing `session_detector::detect_sessions()` (active /proc-based claude sessions only)."
    - "A path qualifies as a GSD project iff `<path>/.planning/` is a directory."
    - "Aliases must be unique. When path basename collides with an existing alias for a DIFFERENT canonical path, append a numeric suffix (e.g. `myproj`, `myproj-2`)."
    - "Duplicate detection compares canonicalized paths (resolve symlinks), not raw strings, so `/home/blk/projects/foo` and `/home/blk/projects/foo/` (or a symlinked variant) don't double-register."
    - "Auto-registration runs (a) once at TUI startup before the event loop, AND (b) on every `Action::SessionsDetected` handled by `App::update` (the existing 5-second session poll)."
    - "On successful auto-registration: persist config, start the file watcher on the new `.planning/` dir, load initial project state, emit `tracing::info!`, surface a `status_message` like `Auto-registered: <alias>`."
  artifacts:
    - ".planning/quick/260512-epe-auto-detect-and-register-gsd-projects-fr/260512-epe-PLAN.md"
    - ".planning/quick/260512-epe-auto-detect-and-register-gsd-projects-fr/260512-epe-SUMMARY.md"
  key_links:
    - "src/registry.rs (existing add_project helper — will be reused)"
    - "src/session_detector.rs (ClaudeSession struct + detect_sessions fn — input)"
    - "src/app.rs (Action::SessionsDetected handler — wire-in point)"
    - "src/main.rs (TUI startup path — startup-scan call site)"
    - "src/config.rs (Config struct + save_config — persistence)"
---

# Quick Task 260512-epe: Auto-Detect and Register GSD Projects

## Goal

When a Claude Code session is detected with a `working_dir` that contains a `.planning/` directory but is not yet in the registry, automatically register it as a project. Runs on TUI startup and on every session-poll tick (~5s).

## User Decisions (from /gsd-quick discussion)

- **Source:** Active claude processes via `/proc` (reuse `session_detector::detect_sessions`).
- **Trigger:** Startup scan + existing 5-second session poll.
- **Behavior:** Auto-add silently, plus emit `tracing::info!` log and a transient `status_message` row in the TUI.

## Tasks

### Task 1 — `auto_register` helper in `registry.rs`

**Files:** `src/registry.rs`

**Action:**

Add a pure helper that takes `&mut Config` and `&[ClaudeSession]`, and returns the list of newly-added aliases (`Vec<(String, PathBuf)>`). The function:

1. Builds a `HashSet<PathBuf>` of currently-registered canonical paths (skip canonicalization failures — fall back to the raw path).
2. For each unique session `working_dir`:
   - Skip if `working_dir.join(".planning")` is not a directory.
   - Canonicalize the path; skip if already registered.
   - Derive alias from `path.file_name()` → lowercase string; fall back to `"project"` if missing/non-utf8.
   - If alias collides with an existing key, append `-2`, `-3`, … until unique.
   - Call existing `add_project` (which validates `.planning/` again and inserts).
   - On success, push `(alias, canonical_path)` to the result vec.
3. Returns the new entries. Caller is responsible for saving config and starting watchers.

Function signature:

```rust
pub fn auto_register_from_sessions(
    config: &mut Config,
    sessions: &[crate::session_detector::ClaudeSession],
) -> Vec<(String, PathBuf)>
```

**Verify:**
- `cargo check` passes.
- Unit tests cover: (a) new path added, (b) already-registered path skipped, (c) path without `.planning/` skipped, (d) alias collision uses `-2` suffix, (e) duplicate sessions for same path register once.

**Done:**
- New helper compiles, all new unit tests pass with `cargo test registry::tests::auto_register`.

---

### Task 2 — Wire helper into `app.rs` and startup path

**Files:** `src/app.rs`, `src/main.rs`

**Action:**

**In `app.rs`:**
- Add a new method `App::auto_register_new_sessions(&mut self)` that:
  1. Reads `self.active_sessions` (already updated by `SessionsDetected` handler).
  2. Calls `registry::auto_register_from_sessions(&mut self.ctx.config, &self.active_sessions)`.
  3. For each `(alias, path)` returned:
     - Calls `save_config(&self.ctx.config, &self.ctx.config_path)` — on error, log via `tracing::warn!` and continue.
     - Starts the file watcher: `self.ctx.watcher.as_mut().map(|w| w.watch(&path.join(".planning")))`.
     - Loads initial state: `let state = state_reader::parse_project_state(&path.join(".planning")); self.ctx.project_states.insert(alias.clone(), state);`.
     - Records initial change-tracker snapshot.
     - Emits `tracing::info!(alias=%alias, path=%path.display(), "Auto-registered GSD project from active session");`.
     - Sets `self.ctx.status_message = Some((format!("Auto-registered: {}", alias), Instant::now()));`.
  4. After the loop, if any aliases were added: `self.ctx.recompute_filtered_aliases(); self.needs_redraw = true;`.

- In `Action::SessionsDetected` handler (currently lines 266-270 of `app.rs`): after the existing assignment lines, call `self.auto_register_new_sessions();`.

**In `main.rs`:**
- After `app.load_project_states();` / `app.init_change_tracker();` and after the `app.ctx.watcher = Some(watcher);` line, run a blocking startup scan:
  ```rust
  let initial_sessions = gsd_meta_manager::session_detector::detect_sessions();
  app.active_sessions = initial_sessions.clone();
  app.ctx.active_sessions = initial_sessions;
  app.auto_register_new_sessions();
  ```
  (Synchronous is fine here — `/proc` reads are fast and this runs before the TUI loop starts.)

**Verify:**
- `cargo build` succeeds with no new warnings.
- `cargo test` passes (existing tests + new ones).
- Manual smoke: with no claude running outside this TUI, startup behaviour is unchanged. With a running claude session whose cwd has `.planning/` but isn't in the config, the project appears in the list after launch.

**Done:**
- Build + tests green. Wire-in lines present at both call sites. Watcher and state-load correctly engaged for auto-registered entries.

---

## Out of Scope

- Removing projects when their session ends (registry persistence is intentional — once added, projects stay until manually removed).
- Monitoring `~/.claude/projects/` directory (decision: active sessions only).
- UI for accepting/rejecting auto-registrations (decision: auto-add silently with notification).
