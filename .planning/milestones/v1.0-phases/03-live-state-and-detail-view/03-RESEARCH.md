# Phase 3: Live State and Detail View - Research

**Researched:** 2026-03-25
**Domain:** File-system watching (notify/debouncer), TUI detail views, change tracking
**Confidence:** HIGH

## Summary

Phase 3 adds two major capabilities: (1) live auto-refresh via filesystem watching and (2) a full-screen detail view for individual projects. The existing codebase is well-prepared -- the EventBus (tokio mpsc), TEA pattern, InputMode enum, and ProjectState parsing are all in place. The Enter key handler in `app.rs:257-262` is a literal placeholder saying "Detail view coming in Phase 3".

The primary technical challenge is integrating `notify-debouncer-full` with the existing tokio-based EventBus. The debouncer uses a callback/std::sync::mpsc pattern, which must bridge to tokio's async mpsc channel. The secondary challenge is enriching the ROADMAP.md parser to extract per-phase plan counts (currently only parses phase-level checkboxes, not plan-level `[x]` items).

**Primary recommendation:** Use `notify-debouncer-full 0.5.0` (compatible with notify 8.x stable). Bridge to tokio mpsc via a closure callback that calls `tx.send()` on the unbounded sender. Implement the detail view as a new `InputMode::DetailView { alias: String }` variant with a dedicated `ui/detail_view.rs` render module.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** Enter replaces the main table with a full-screen detail panel -- Esc returns to project list. Consistent with TUI modal pattern established in Phase 2.
- **D-02:** Detail view shows: project path, all roadmap phases with status icons (circle pending, diamond in-progress, checkmark complete), current phase highlighted, plan completion counts per phase (e.g., "checkmark P1: Core Infrastructure  3/3 plans")
- **D-03:** Per-phase status displayed as a vertical list with phase number, name, status icon, and plan count
- **D-04:** Change summary appears at top of detail view as a highlighted banner (e.g., "Phase 3 completed 2h ago") -- visible immediately on drill-in
- **D-05:** File watching via notify 8.x with debouncer (200ms), watching all registered projects' `.planning/` directories. Events flow through the existing tokio mpsc EventBus channel (architecture set up in Phase 1).
- **D-06:** Silent refresh -- table updates without flash/blink. Status bar briefly shows "Updated: projectname" for 2 seconds after a refresh.
- **D-07:** Change tracking is per-app-launch only (in-memory) -- no persistence to config.json. Record initial state snapshot at startup, compare on each file-change event.
- **D-08:** Only phase completions and status transitions are worth summarizing (e.g., "Phase 2 completed", "Status: idle -> active") -- not every `.planning/` file write.

### Claude's Discretion
- notify debouncer crate choice (notify-debouncer-mini vs notify-debouncer-full)
- Detail view scrolling behavior if roadmap has many phases
- How to handle watching projects on paths that become unavailable (unmounted, deleted)
- Change detection algorithm (mtime-based vs content hash)

### Deferred Ideas (OUT OF SCOPE)
None -- discussion stayed within phase scope
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| STATE-04 | Manager auto-refreshes when `.planning/` files change via file system watcher (inotify/kqueue) | notify-debouncer-full 0.5.0 with 200ms debounce; bridge callback to tokio mpsc EventBus; new Action::FileChanged variant |
| STATE-05 | Research whether GSD hooks can push state updates to the manager instead of polling | GSD hooks are shell commands triggered by Claude Code events; a hook could write to a known FIFO or send a signal, but file-watching is simpler and already covers the use case. Recommend: document hook possibility, do not implement in Phase 3 |
| DET-01 | User can drill into a project to see: project path, all roadmap phases, current phase, and task completion counts | New InputMode::DetailView variant; new ui/detail_view.rs; extend roadmap_md parser to count per-phase plan items |
| DET-02 | Detail view shows phase-level breakdown with status per phase (pending, in-progress, complete) | Derive phase status from: completed checkbox = complete, phase number matches current = in-progress, else = pending |
| DASH-05 | User sees a change summary showing what changed since last visit (e.g., "Phase 3 completed 2h ago") | In-memory ChangeTracker storing initial snapshot at startup; compare on each FileChanged event; human-readable relative time using chrono (already in Cargo.toml) |
</phase_requirements>

## Project Constraints (from CLAUDE.md)

- **notify 8.x only**: CLAUDE.md specifies notify 8.0.0 stable, not 9.x rc. Latest stable is actually 8.2.0.
- **notify-debouncer-full 8.x**: CLAUDE.md says "notify-debouncer-full 8.x companion crate". Verified: `notify-debouncer-full 0.5.0` depends on `notify ^8.0.0`.
- **TEA pattern**: All state changes go through Action enum -> App::update(). File watcher events must follow this pattern.
- **tokio::select! for event racing**: crossterm events, notify events, and tick all merge via mpsc.
- **Never block the render loop**: All I/O through tokio channels.
- **Parse .planning/ lazily, cache in memory**: StateReader is the only module that knows the schema.
- **200ms debounce**: Established in CLAUDE.md and CONTEXT.md D-05.
- **No std::sync::Mutex in async code**: Use tokio::sync::Mutex if needed.

## Standard Stack

### Core (new dependencies for Phase 3)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| notify | 8.2.0 | Filesystem watching (inotify/kqueue/ReadDirectoryChangesW) | Latest stable; CLAUDE.md mandates 8.x, not 9.x rc |
| notify-debouncer-full | 0.5.0 | Event debouncing for notify 8.x | Batches rapid FS events into single debounced events; 200ms window per D-05. Version 0.5.0 is the latest compatible with notify ^8.0.0 |

### Already Present (no version changes)
| Library | Version | Purpose |
|---------|---------|---------|
| chrono | 0.4 | Human-readable relative time for change summary ("2h ago") |
| tokio | 1.x | Async runtime, mpsc channels |
| ratatui | 0.30 | TUI rendering |
| crossterm | 0.29 | Terminal backend |

### Debouncer Choice Recommendation (Claude's Discretion)

**Use `notify-debouncer-full 0.5.0`** over `notify-debouncer-mini 0.6.0`.

| Feature | debouncer-full 0.5.0 | debouncer-mini 0.6.0 |
|---------|----------------------|----------------------|
| Tracks renames | Yes (file ID cache) | No |
| Event merging | Full (create+modify+close -> single event) | Minimal (just time dedup) |
| Dependency on notify | ^8.0.0 | ^8.0.0 |
| Complexity | Slightly more | Simpler |

Full is preferred because GSD workflows can rename/move files within `.planning/` (e.g., phase directories), and proper event merging reduces spurious reloads.

**Installation (add to Cargo.toml):**
```toml
notify = "8"
notify-debouncer-full = "0.5"
```

## Architecture Patterns

### Pattern 1: Bridging notify callback to tokio mpsc

The `DebounceEventHandler` trait is implemented for `std::sync::mpsc::Sender` but NOT for `tokio::sync::mpsc::UnboundedSender`. Bridge via a closure:

```rust
// watcher.rs
use notify_debouncer_full::{new_debouncer, DebounceEventResult};
use notify::RecursiveMode;
use std::path::Path;
use std::time::Duration;
use tokio::sync::mpsc::UnboundedSender;
use crate::action::Action;

pub struct FileWatcher {
    debouncer: notify_debouncer_full::Debouncer<
        notify::RecommendedWatcher,
        notify_debouncer_full::RecommendedCache,
    >,
}

impl FileWatcher {
    pub fn new(tx: UnboundedSender<Action>) -> anyhow::Result<Self> {
        let debouncer = new_debouncer(
            Duration::from_millis(200),
            None, // use default file ID cache
            move |result: DebounceEventResult| {
                match result {
                    Ok(events) => {
                        for event in events {
                            // Extract the project path from the changed file's path
                            // e.g., /home/user/project/.planning/STATE.md -> /home/user/project
                            for path in &event.paths {
                                if let Some(project_path) = extract_project_root(path) {
                                    let _ = tx.send(Action::FileChanged {
                                        project_path: project_path.to_path_buf(),
                                    });
                                }
                            }
                        }
                    }
                    Err(errors) => {
                        for error in errors {
                            tracing::warn!("File watcher error: {:?}", error);
                        }
                    }
                }
            },
        )?;

        Ok(Self { debouncer })
    }

    pub fn watch(&mut self, planning_dir: &Path) -> anyhow::Result<()> {
        self.debouncer
            .watch(planning_dir, RecursiveMode::Recursive)?;
        Ok(())
    }

    pub fn unwatch(&mut self, planning_dir: &Path) -> anyhow::Result<()> {
        self.debouncer.unwatch(planning_dir)?;
        Ok(())
    }
}

fn extract_project_root(path: &Path) -> Option<&Path> {
    // Walk up from the changed file to find the parent of .planning/
    let mut current = path;
    while let Some(parent) = current.parent() {
        if parent.file_name().map(|n| n == ".planning").unwrap_or(false) {
            return parent.parent();
        }
        current = parent;
    }
    None
}
```

**Key insight:** The `tokio::sync::mpsc::UnboundedSender::send()` is non-async and can be called from any thread, including the notify callback thread. No `block_on` or async bridging needed.

### Pattern 2: Action Enum Extension

```rust
// action.rs - add these variants
pub enum Action {
    // ... existing variants ...
    FileChanged { project_path: PathBuf },
    RefreshProject { alias: String },
    ProjectStateUpdated { alias: String, old_state: Option<ProjectState>, new_state: ProjectState },
}
```

### Pattern 3: InputMode::DetailView

```rust
// app.rs - extend InputMode
pub enum InputMode {
    Normal,
    AddAlias,
    AddPath { alias: String },
    DeleteConfirm { alias: String },
    Search,
    HelpOverlay,
    DetailView { alias: String },  // NEW
}
```

### Pattern 4: Change Tracking (In-Memory)

```rust
// change_tracker.rs
use crate::state_reader::ProjectState;
use std::collections::HashMap;
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct ChangeEvent {
    pub description: String,  // e.g., "Phase 2 completed", "Status: idle -> active"
    pub timestamp: Instant,
}

pub struct ChangeTracker {
    initial_snapshots: HashMap<String, ProjectSnapshot>,
    changes: HashMap<String, Vec<ChangeEvent>>,
}

#[derive(Debug, Clone)]
struct ProjectSnapshot {
    status: String,
    completed_phases: u32,
    completed_plans: u32,
}

impl ChangeTracker {
    pub fn new() -> Self { /* ... */ }

    pub fn record_initial(&mut self, alias: &str, state: &ProjectState) {
        self.initial_snapshots.insert(alias.to_string(), ProjectSnapshot {
            status: state.status.clone(),
            completed_phases: state.completed_phases,
            completed_plans: state.completed_plans,
        });
    }

    pub fn detect_changes(&mut self, alias: &str, old: &ProjectState, new: &ProjectState) {
        // Compare status transitions
        if old.status != new.status {
            self.add_change(alias, format!("Status: {} -> {}", old.status, new.status));
        }
        // Compare phase completions
        if new.completed_phases > old.completed_phases {
            // Find which phase completed
            for phase in &new.phases {
                if phase.completed {
                    // Check if it was not completed before
                    let was_completed = old.phases.iter()
                        .find(|p| p.number == phase.number)
                        .map(|p| p.completed)
                        .unwrap_or(false);
                    if !was_completed {
                        self.add_change(alias, format!("Phase {} completed", phase.name));
                    }
                }
            }
        }
    }

    pub fn latest_change(&self, alias: &str) -> Option<&ChangeEvent> {
        self.changes.get(alias)?.last()
    }

    fn add_change(&mut self, alias: &str, description: String) {
        self.changes.entry(alias.to_string())
            .or_default()
            .push(ChangeEvent {
                description,
                timestamp: Instant::now(),
            });
    }
}
```

### Pattern 5: Human-Readable Relative Time

```rust
// Using chrono (already in Cargo.toml) is overkill for relative time from Instant.
// Use std::time::Instant with simple arithmetic instead:
fn format_elapsed(instant: std::time::Instant) -> String {
    let secs = instant.elapsed().as_secs();
    if secs < 60 { return "just now".to_string(); }
    if secs < 3600 { return format!("{}m ago", secs / 60); }
    if secs < 86400 { return format!("{}h ago", secs / 3600); }
    format!("{}d ago", secs / 86400)
}
```

### Pattern 6: Detail View Render Layout

```
+--[ Project: my-app ]-------------------------------------------+
|  Path: /home/user/projects/my-app                              |
|  Status: Ready to plan        Milestone: v1.0                  |
|                                                                |
|  [ Phase 3 completed 2h ago ]     <-- change banner (D-04)     |
|                                                                |
|  Phases:                                                       |
|  checkmark P1: Core Infrastructure        3/3 plans            |
|  checkmark P2: Dashboard and Navigation   2/2 plans            |
|  diamond   P3: Live State and Detail View 0/? plans   <-- current, highlighted
|  circle    P4: Visualization              0/? plans            |
|                                                                |
|  Backlog: 2 items                                              |
+----------------------------------------------------------------+
| [Esc]back  [j/k]scroll                                        |
+----------------------------------------------------------------+
```

### Pattern 7: Extending the ROADMAP.md Parser

Current `roadmap_md.rs` only parses phase checklist lines. Need to also parse per-phase plan items:

```rust
// roadmap_md.rs - extend RoadmapPhase
#[derive(Debug, Clone)]
pub struct RoadmapPhase {
    pub number: String,
    pub name: String,
    pub description: String,
    pub completed: bool,
    pub total_plans: u32,      // NEW
    pub completed_plans: u32,  // NEW
}
```

The ROADMAP.md format has plan items like:
```
Plans:
- [x] 01-01-PLAN.md -- description
- [x] 01-02-PLAN.md -- description
- [ ] 01-03-PLAN.md -- description
```

These appear after each phase's metadata block. The parser needs to associate plan checklist items with the preceding phase.

### Recommended Project Structure Changes

```
src/
+-- watcher.rs           # NEW: FileWatcher struct, notify integration
+-- change_tracker.rs    # NEW: In-memory change tracking per D-07/D-08
+-- ui/
|   +-- detail_view.rs   # NEW: Full-screen project detail panel
|   +-- project_list.rs  # MODIFY: Remove Enter placeholder
|   +-- mod.rs           # MODIFY: Dispatch to detail_view when in DetailView mode
+-- app.rs               # MODIFY: Add DetailView to InputMode, handle FileChanged action
+-- action.rs            # MODIFY: Add FileChanged, RefreshProject variants
+-- event.rs             # NO CHANGE: watcher sends directly to EventBus tx
+-- state_reader/
    +-- roadmap_md.rs    # MODIFY: Parse per-phase plan counts
    +-- mod.rs           # NO CHANGE (maybe minor)
```

### Anti-Patterns to Avoid

- **Re-parsing all projects on any file change**: Only re-parse the specific project whose `.planning/` directory changed. The `extract_project_root()` function maps FS events to project aliases.
- **Blocking the render loop with file reads**: The notify callback runs on its own thread. The `App::update()` handler for `FileChanged` should spawn a tokio task or use `tokio::task::spawn_blocking` for the re-parse, then send `ProjectStateUpdated` back via the EventBus.
- **Watching root paths instead of .planning/ dirs**: Only watch `.planning/` directories, not project roots. This avoids noise from source code changes, IDE indexing, etc.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| FS event debouncing | Custom timer + event dedup | notify-debouncer-full 0.5.0 | Handles rename tracking, event merging, cross-platform edge cases |
| Relative time formatting | Custom duration formatter | Simple helper function (8 lines) | chrono is overkill; std::time::Instant arithmetic is sufficient |
| File watcher setup | Raw inotify/kqueue | notify 8.2.0 | Cross-platform abstraction over inotify (Linux), kqueue (macOS), ReadDirectoryChanges (Windows) |

## Common Pitfalls

### Pitfall 1: notify-debouncer version mismatch with notify 8.x
**What goes wrong:** Using `notify-debouncer-full 0.7.0` (latest) pulls in `notify 9.0.0-rc.2`, violating the project constraint of notify 8.x stable.
**Why it happens:** The debouncer crate versions don't align with notify major versions. 0.7.0 -> notify 9.x, 0.5.0 -> notify 8.x.
**How to avoid:** Pin `notify-debouncer-full = "0.5"` and `notify = "8"` explicitly in Cargo.toml.
**Warning signs:** Compile errors about incompatible notify types, or unexpected `notify 9.0.0-rc.2` in `Cargo.lock`.

### Pitfall 2: Blocking file re-parse in the notify callback
**What goes wrong:** The notify callback (running on a background thread) calls `parse_project_state()` synchronously, which does multiple file reads. If a `.planning/` file is mid-write, the parse may fail or return stale data.
**Why it happens:** It's tempting to "handle everything in the callback." But the callback should only signal; the re-parse should happen in the main loop with retry logic.
**How to avoid:** The callback sends `Action::FileChanged { project_path }` only. The `App::update()` handler maps the path to an alias, clones the old state, re-parses, and detects changes.
**Warning signs:** Transient "unknown" status flickers in the dashboard.

### Pitfall 3: Watching paths that become unavailable
**What goes wrong:** A registered project is on a removable drive or network mount that goes offline. The watcher errors out, potentially crashing or flooding logs.
**Why it happens:** notify returns errors when watched paths become unavailable, but the error handling path is often untested.
**How to avoid:** Wrap `watch()` in error handling. If a watch fails, mark the project as "unreachable" in the UI. Periodically retry (on tick, every 30s). Log at `warn` level, not `error`.
**Warning signs:** Application crash when unmounting a drive while running.

### Pitfall 4: Event storms from GSD batch writes
**What goes wrong:** GSD writes STATE.md, ROADMAP.md, and config.json in rapid succession during phase transitions. Even with 200ms debounce, this can cause 2-3 rapid reloads.
**Why it happens:** Each file write triggers an inotify event. Debouncing helps but doesn't eliminate all duplicates when writes span >200ms.
**How to avoid:** Deduplicate at the `App::update()` level. If a `FileChanged` arrives for a project that was already refreshed within the last 500ms, skip the re-parse. Use a `HashMap<String, Instant>` for last-refresh timestamps.
**Warning signs:** Dashboard flickers during active GSD sessions.

### Pitfall 5: Detail view crashes on missing phase data
**What goes wrong:** The detail view assumes `ProjectState.phases` is non-empty or that plan counts are always available. A project with a malformed or minimal ROADMAP.md causes a panic.
**Why it happens:** `parse_roadmap_phases()` can return an empty vec. Plan count parsing is new and may not cover all ROADMAP.md format variants.
**How to avoid:** Always handle empty phases vec in the detail view. Show "No roadmap data" instead of panicking. Use `unwrap_or_default()` patterns.
**Warning signs:** Panic on opening detail view for a newly registered project.

## Code Examples

### FileChanged -> Refresh Flow (App::update)

```rust
// In app.rs update() match:
Action::FileChanged { project_path } => {
    // Find the alias for this project path
    if let Some((alias, _)) = self.config.projects.iter()
        .find(|(_, p)| p.path == project_path)
    {
        let alias = alias.clone();
        let planning_dir = project_path.join(".planning");

        // Deduplicate: skip if refreshed recently
        // (implementation detail: add last_refresh HashMap to App)

        let old_state = self.project_states.get(&alias).cloned();
        let new_state = state_reader::parse_project_state(&planning_dir);

        // Detect and record changes
        if let Some(ref old) = old_state {
            self.change_tracker.detect_changes(&alias, old, &new_state);
        }

        self.project_states.insert(alias.clone(), new_state);
        self.recompute_filtered_aliases();

        // Show "Updated: alias" status message per D-06
        self.status_message = Some((
            format!("Updated: {}", alias),
            std::time::Instant::now(),
        ));
        self.needs_redraw = true;
    }
}
```

### Detail View Key Handling

```rust
// In app.rs handle_normal_key, replace the Enter placeholder:
KeyCode::Enter => {
    if let Some(alias) = self.selected_alias() {
        self.input_mode = InputMode::DetailView { alias };
        self.needs_redraw = true;
    }
}

// New handler for DetailView mode:
fn handle_detail_key(&mut self, code: KeyCode) {
    match code {
        KeyCode::Esc | KeyCode::Char('q') => {
            self.input_mode = InputMode::Normal;
            self.needs_redraw = true;
        }
        KeyCode::Char('j') | KeyCode::Down => {
            // Scroll down if content overflows (Claude's discretion: scrolling)
            self.detail_scroll_offset += 1;
            self.needs_redraw = true;
        }
        KeyCode::Char('k') | KeyCode::Up => {
            self.detail_scroll_offset = self.detail_scroll_offset.saturating_sub(1);
            self.needs_redraw = true;
        }
        _ => {}
    }
}
```

### Watcher Initialization in main.rs

```rust
// In main.rs TUI mode, after app.load_project_states():
let mut watcher = FileWatcher::new(event_bus.tx.clone())?;

// Watch all registered projects
for project in app.config.projects.values() {
    let planning_dir = project.path.join(".planning");
    if planning_dir.is_dir() {
        if let Err(e) = watcher.watch(&planning_dir) {
            tracing::warn!("Failed to watch {}: {}", planning_dir.display(), e);
        }
    }
}

// Initialize change tracker with current state
app.init_change_tracker();
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| notify 7.x with manual debounce | notify 8.x with notify-debouncer-full | notify 8.0.0 released 2025-01-10 | Cleaner API, better event types |
| notify-debouncer-full 0.4 (notify 7.x) | notify-debouncer-full 0.5 (notify 8.x) | 2025 | Version alignment with notify 8.x |
| notify 9.x rc | notify 8.2.0 stable | 9.x still rc as of 2026-03-25 | 8.2.0 is production-ready; 9.x has breaking changes |

## Open Questions

1. **Per-phase plan count parsing scope**
   - What we know: ROADMAP.md has `- [x] XX-YY-PLAN.md` lines under each phase
   - What's unclear: Not all GSD projects may have plan items listed. Some may show `**Plans**: TBD`
   - Recommendation: Parse plan items when present; fall back to 0/0 when absent. Handle `TBD` gracefully.

2. **STATE-05: GSD hook integration**
   - What we know: GSD hooks are shell commands triggered by Claude Code events. The EventBus channel architecture supports receiving external signals.
   - What's unclear: Exact hook mechanism (FIFO? signal? HTTP?), and whether this is worth implementing when file watching already covers the use case.
   - Recommendation: Document the hook integration point (EventBus tx can accept from any source) but do NOT implement hook-based push in Phase 3. File watching is sufficient. Mark STATE-05 as "researched, file watching is the recommended approach."

3. **Async re-parse on FileChanged**
   - What we know: `parse_project_state()` does synchronous file I/O. It's fast for local disks.
   - What's unclear: Whether to spawn a blocking task for the re-parse or do it inline in `App::update()`.
   - Recommendation: Do it inline in `App::update()` for now. The parse is <1ms on local disk. If profiling shows issues with network mounts, add `spawn_blocking` later.

## Sources

### Primary (HIGH confidence)
- [docs.rs/notify-debouncer-full/0.5.0](https://docs.rs/notify-debouncer-full/0.5.0/notify_debouncer_full/) - API docs, DebounceEventHandler trait, version 0.5.0 depends on notify ^8.0.0
- [docs.rs/crate/notify/latest](https://docs.rs/crate/notify/latest) - notify 8.2.0 is latest stable release (2025-08-03)
- [docs.rs/notify-debouncer-full/0.7.0](https://docs.rs/notify-debouncer-full/0.7.0/notify_debouncer_full/trait.DebounceEventHandler.html) - DebounceEventHandler implemented for std::sync::mpsc::Sender and FnMut closures
- Existing codebase: `src/event.rs`, `src/app.rs`, `src/action.rs`, `src/state_reader/`, `src/ui/` -- verified integration points
- cargo search: notify 9.0.0-rc.2 (pre-release), notify-debouncer-full 0.7.0 (depends on 9.x), notify-debouncer-mini 0.7.0

### Secondary (MEDIUM confidence)
- [docs.rs/notify-debouncer-mini/0.6.0](https://docs.rs/crate/notify-debouncer-mini/0.6.0) - debouncer-mini 0.6.0 depends on notify ^8.0.0
- [notify-rs GitHub](https://github.com/notify-rs/notify) - workspace Cargo.toml confirms 0.7.0 debouncers target notify 9.x

### Tertiary (LOW confidence)
- None

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH - versions verified via cargo search and docs.rs dependency chains
- Architecture: HIGH - integration points verified against existing codebase; patterns follow established TEA architecture
- Pitfalls: HIGH - notify version mismatch verified; callback bridging pattern confirmed via trait docs

**Research date:** 2026-03-25
**Valid until:** 2026-04-25 (stable libraries, low churn)
