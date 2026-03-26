# Phase 05: State Reader Accuracy - Research

**Researched:** 2026-03-26
**Domain:** Rust TUI state parsing, async I/O migration, screen architecture refactor, CLI ergonomics
**Confidence:** HIGH

## Summary

Phase 05 addresses four concrete requirements (STATE-01, STATE-02, STATE-03, CLI-01) plus two architectural prerequisites (D-05: InputMode refactor, D-06: async migration) that are mandatory before Phase 06+ features can be layered on. The codebase has been analyzed line-by-line and every bug, pattern, and integration point is documented from direct source observation.

The six workstreams are: (1) fix plan counting regex in `roadmap_md.rs` to match standalone `PLAN.md` files, (2) fix the P5:Unknown bug in `state_reader/mod.rs` where completed milestones show nonexistent phases, (3) implement GSD's `disk_status` algorithm for phase inference from disk artifacts, (4) make CLI `add` alias optional with auto-derive from folder name, (5) refactor `InputMode` from a flat 11-variant enum to a screen/component architecture, and (6) wrap all synchronous file I/O in `spawn_blocking` with result dispatch through the event bus.

**Primary recommendation:** Execute the InputMode refactor and async migration first (they touch foundational code), then layer the bug fixes and new features on the refactored architecture. The disk inference module is the most complex new code and should be developed with comprehensive test fixtures from both active and archived phase directories.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- **D-01:** Claude's discretion on disk inference depth and relationship to ROADMAP.md. Recommended: disk is ground truth for phase status, ROADMAP.md provides structure/names. Use GSD's 7 statuses extended with plan/summary counts where useful for the detail view.
- **D-02:** Dashboard project list shows compact pipeline (`D-R-P-E-V` with color per stage) in the status column. When the row is selected/focused, switch to showing the full text label (e.g., "Executing 2/3").
- **D-03:** When a milestone is fully complete, show milestone name (e.g., "v1.0 Complete") if the milestone version is not already visible elsewhere on the same screen row. Claude's discretion on exact format.
- **D-04:** Change `Add` subcommand signature to `add <path> [alias]` -- alias becomes optional positional arg, defaults to last path component (folder name).
- **D-05:** Refactor InputMode enum to screen/component architecture in this phase (prerequisite for Phase 06+). Research says 11 variants will explode past 20 with new views -- do it now.
- **D-06:** Full async migration in this phase. Move all `parse_project_state()` file reads to `spawn_blocking`. Pay the cost once so Phase 06+ features build on an async foundation.

### Claude's Discretion
- Disk inference depth: Claude picks GSD's algorithm + extended counts as appropriate
- Disk vs ROADMAP.md relationship: Claude decides merge strategy
- Completed milestone display format: Claude decides based on screen layout

### Deferred Ideas (OUT OF SCOPE)
None -- discussion stayed within phase scope
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| STATE-01 | User sees accurate plan counts per phase on dashboard (fix regex for standalone PLAN.md) | Bug located at `roadmap_md.rs:19` -- regex `^\s*- \[([ xX])\] \d+-\d+-PLAN\.md` misses `PLAN.md` standalone files. Fix pattern documented in Code Examples. |
| STATE-02 | User sees "Complete" instead of "P5: Unknown" when all phases are done | Bug located at `state_reader/mod.rs:50` -- `format!("Phase {}", completed_phases + 1)` when all phases complete. Fix: check `completed_phases >= total_phases` before computing. Also affects `format_phase_display()` at `app.rs:80-89`. |
| STATE-03 | User sees phase status inferred from disk files (discuss/research/plan/execute/verify stages) with confidence indicators | GSD's `disk_status` algorithm verified from `roadmap.cjs:127-166`. Seven statuses: no_directory/empty/discussed/researched/planned/partial/complete. File detection patterns documented. |
| CLI-01 | User can register a project by path only -- name auto-derived from last folder component | CLI struct at `cli.rs:18-23` needs alias made optional. Auto-derive from `path.file_name()`. |
</phase_requirements>

## Standard Stack

### Core (no new dependencies needed)
| Library | Version | Purpose | Why Standard |
|---------|---------|---------|--------------|
| ratatui | 0.30.0 | TUI rendering | Already in Cargo.toml; screen/component refactor uses existing Widget trait |
| crossterm | 0.29.0 | Terminal backend | Already in Cargo.toml; unchanged for this phase |
| tokio | 1.x | Async runtime | Already in Cargo.toml; `spawn_blocking` and mpsc channels used for async migration |
| regex | 1.x | Pattern matching | Already in Cargo.toml; plan counting regex fix |
| clap | 4.x | CLI parsing | Already in Cargo.toml; `Option<String>` for alias parameter |

### No New Dependencies

This phase requires zero new crate additions. All work uses existing dependencies. The async migration uses `tokio::task::spawn_blocking` (already available via `tokio = { features = ["full"] }`). The screen architecture uses ratatui's existing trait system.

## Architecture Patterns

### Recommended Project Structure Changes

```
src/
  app.rs              # MAJOR REFACTOR: InputMode -> Screen trait, async state loading
  action.rs           # MODIFY: add ProjectStateLoaded action variant
  cli.rs              # MODIFY: alias becomes Option<String>
  state_reader/
    mod.rs            # MODIFY: add disk_status module, async wrapper
    roadmap_md.rs     # MODIFY: fix plan counting regex
    disk_status.rs    # NEW: GSD disk_status algorithm implementation
  ui/
    mod.rs            # MODIFY: dispatch via Screen trait instead of InputMode match
    screens/          # NEW: directory for screen trait impls (optional, can keep flat)
    project_list.rs   # MODIFY: compact pipeline display, focused text label
    detail_view.rs    # MODIFY: completed milestone display
```

### Pattern 1: Screen/Component Architecture (D-05)

**What:** Replace the flat `InputMode` enum with a `Screen` trait. Each screen owns its key handling and rendering. The `App` holds a screen stack.

**When to use:** Any TUI with more than 5 distinct input-handling modes.

**Current state (11 variants):**
```rust
pub enum InputMode {
    Normal, AddAlias, AddPath { alias }, DeleteConfirm { alias },
    Search, HelpOverlay, DetailView { alias }, CreateName,
    CreatePath { name }, CreateConfirm { name, path }, EnqueueInput { alias },
}
```

**Target architecture:**
```rust
// Screen trait -- each screen handles its own input and rendering
pub trait Screen {
    fn handle_key(&mut self, code: KeyCode, modifiers: KeyModifiers, ctx: &mut AppContext) -> ScreenAction;
    fn render(&self, frame: &mut Frame, area: Rect, ctx: &AppContext);
    fn name(&self) -> &str; // for debugging / help overlay
}

// ScreenAction -- what to do after handling a key
pub enum ScreenAction {
    None,
    Push(Box<dyn Screen>),      // navigate to a new screen
    Pop,                         // go back (Esc)
    PopTo(usize),               // pop multiple levels
    Quit,
    SetStatusMessage(String),
    DispatchAction(Action),     // send action through event bus
}

// AppContext -- shared state that screens can read/write
pub struct AppContext {
    pub config: Config,
    pub config_path: PathBuf,
    pub project_states: HashMap<String, ProjectState>,
    pub table_state: TableState,
    pub filtered_aliases: Vec<String>,
    pub filter_text: String,
    pub change_tracker: ChangeTracker,
    pub detail_sub_view_per_project: HashMap<String, DetailSubView>,
    pub status_message: Option<(String, std::time::Instant)>,
    pub error_message: Option<String>,
    pub event_tx: Option<UnboundedSender<Action>>,
    pub watcher: Option<FileWatcher>,
    pub last_refresh: HashMap<String, std::time::Instant>,
    // ... other shared state
}

// App becomes thin orchestrator
pub struct App {
    pub should_quit: bool,
    pub needs_redraw: bool,
    pub ctx: AppContext,
    pub screen_stack: Vec<Box<dyn Screen>>,
}
```

**Key insight:** Modal screens (AddAlias -> AddPath, CreateName -> CreatePath -> CreateConfirm) become screen pushes. The `.clone()` on `self.input_mode` at `app.rs:363` disappears because each screen owns its state. Escape always pops. No key conflict checking needed across modes.

**Migration strategy:** The refactor preserves all existing behavior. Each current `handle_*_key` method becomes a Screen impl. The `InputMode` enum is deleted once all screens are migrated. This can be done incrementally: start with `NormalScreen` and `DetailScreen`, verify they work, then migrate the modal screens.

### Pattern 2: Async State Loading (D-06)

**What:** Wrap `parse_project_state()` in `spawn_blocking`, dispatch results through the event bus.

**Current sync pattern (app.rs:248-251):**
```rust
// In FileChanged handler -- BLOCKS the render loop
let new_state = state_reader::parse_project_state(&planning_dir);
self.project_states.insert(alias.clone(), new_state);
```

**Target async pattern:**
```rust
// New Action variant
Action::ProjectStateLoaded {
    alias: String,
    state: ProjectState,
}

// In FileChanged handler -- non-blocking
if let Some(tx) = &self.ctx.event_tx {
    let tx = tx.clone();
    let alias = alias.clone();
    let planning_dir = planning_dir.clone();
    tokio::task::spawn_blocking(move || {
        let state = state_reader::parse_project_state(&planning_dir);
        let _ = tx.send(Action::ProjectStateLoaded { alias, state });
    });
}

// New handler for the result
Action::ProjectStateLoaded { alias, state } => {
    if let Some(old_state) = self.ctx.project_states.get(&alias) {
        self.ctx.change_tracker.detect_changes(&alias, old_state, &state);
    }
    self.ctx.project_states.insert(alias.clone(), state);
    self.ctx.last_refresh.insert(alias, std::time::Instant::now());
    self.ctx.recompute_filtered_aliases();
    self.needs_redraw = true;
}
```

**Also apply to:**
- `load_project_states()` at `app.rs:148-155` (initial load at startup)
- `do_add_project()` at `app.rs:886` (after adding a project)
- `handle_enqueue_key()` at `app.rs:593` (after saving queue)
- `CreateProjectResult` handler at `app.rs:329` (after project creation)

**Note:** `ProjectState` must derive `Send` to cross the spawn_blocking boundary. It already does (all fields are String, u32, Vec of Send types).

### Pattern 3: Disk Status Inference (STATE-03)

**What:** Replicate GSD's `disk_status` algorithm from `roadmap.cjs:127-166`.

**GSD's algorithm (verified from source):**
```rust
pub enum DiskStatus {
    NoDirectory,  // Phase directory doesn't exist
    Empty,        // Directory exists but no workflow artifacts
    Discussed,    // CONTEXT.md or *-CONTEXT.md present
    Researched,   // RESEARCH.md or *-RESEARCH.md present
    Planned,      // PLAN.md or *-PLAN.md present, no SUMMARY files
    Partial,      // Some SUMMARY files but count < PLAN count
    Complete,     // SUMMARY count >= PLAN count (all plans executed)
}

pub struct DiskInference {
    pub status: DiskStatus,
    pub plan_count: u32,
    pub summary_count: u32,
    pub has_context: bool,
    pub has_research: bool,
}

/// Infer phase status from disk artifacts.
/// `phase_dir` is the full path to the phase directory (e.g., `.planning/phases/05-state-reader-accuracy/`)
pub fn infer_disk_status(phase_dir: &Path) -> DiskInference {
    if !phase_dir.is_dir() {
        return DiskInference { status: DiskStatus::NoDirectory, ..Default::default() };
    }

    let files: Vec<String> = std::fs::read_dir(phase_dir)
        .into_iter()
        .flatten()
        .filter_map(|e| e.ok())
        .filter_map(|e| e.file_name().to_str().map(String::from))
        .collect();

    let plan_count = files.iter()
        .filter(|f| f.ends_with("-PLAN.md") || *f == "PLAN.md")
        .count() as u32;
    let summary_count = files.iter()
        .filter(|f| f.ends_with("-SUMMARY.md") || *f == "SUMMARY.md")
        .count() as u32;
    let has_context = files.iter()
        .any(|f| f.ends_with("-CONTEXT.md") || f == "CONTEXT.md");
    let has_research = files.iter()
        .any(|f| f.ends_with("-RESEARCH.md") || f == "RESEARCH.md");

    let status = if summary_count >= plan_count && plan_count > 0 {
        DiskStatus::Complete
    } else if summary_count > 0 {
        DiskStatus::Partial
    } else if plan_count > 0 {
        DiskStatus::Planned
    } else if has_research {
        DiskStatus::Researched
    } else if has_context {
        DiskStatus::Discussed
    } else {
        DiskStatus::Empty
    };

    DiskInference { status, plan_count, summary_count, has_context, has_research }
}
```

**Phase directory resolution:** Phase directories live in `.planning/phases/` and follow the pattern `NN-slug-name` (e.g., `05-state-reader-accuracy`). GSD's algorithm normalizes the phase number to a zero-padded string and finds directories starting with that prefix. The existing `count_backlog_items()` at `state_reader/mod.rs:72-88` already scans this directory -- the disk inference module can reuse the same `read_dir` pattern.

**Merge strategy (Claude's discretion area):** Use GSD's algorithm as the primary disk inference, but also check the ROADMAP.md checkbox status. If ROADMAP.md marks a phase as complete (`[x]`) but disk says otherwise, trust ROADMAP.md (matches GSD's own behavior at `roadmap.cjs:164-166`). The `RoadmapPhase` struct should be extended with a `disk_status: Option<DiskStatus>` field populated during parsing.

### Pattern 4: Compact Pipeline Display (D-02)

**What:** Show `D-R-P-E-V` with per-stage colors in the status column. On row focus, expand to full text.

**Implementation:**
```rust
/// Render compact pipeline for unfocused rows
fn compact_pipeline(disk_status: &DiskStatus) -> Vec<Span> {
    let stages = [
        ("D", DiskStatus::Discussed),
        ("R", DiskStatus::Researched),
        ("P", DiskStatus::Planned),
        ("E", DiskStatus::Partial),  // partial = executing
        ("V", DiskStatus::Complete), // complete = verified (or fully executed)
    ];

    stages.iter().map(|(label, threshold)| {
        let color = if *disk_status >= *threshold {
            Color::Green
        } else if *disk_status == previous_status(*threshold) {
            Color::Yellow  // current stage
        } else {
            Color::DarkGray  // not reached
        };
        Span::styled(*label, Style::default().fg(color))
    }).collect()
}

/// Render expanded text for focused rows
fn expanded_status(disk_status: &DiskStatus, plan_count: u32, summary_count: u32) -> String {
    match disk_status {
        DiskStatus::NoDirectory => "Not started".to_string(),
        DiskStatus::Empty => "Empty".to_string(),
        DiskStatus::Discussed => "Discussed".to_string(),
        DiskStatus::Researched => "Researched".to_string(),
        DiskStatus::Planned => format!("Planned ({} plans)", plan_count),
        DiskStatus::Partial => format!("Executing {}/{}", summary_count, plan_count),
        DiskStatus::Complete => "Complete".to_string(),
    }
}
```

**Note:** D-02 says the compact pipeline appears in the dashboard project list status column. This requires the `DiskStatus` to be computed per-phase AND a summary status per-project (current phase's disk status). The `ProjectState` struct needs a `current_phase_disk_status` field.

### Anti-Patterns to Avoid

- **Cloning InputMode on every key event** (`app.rs:363` -- `match &self.input_mode.clone()`): The screen refactor eliminates this by making each screen own its state independently.
- **Synchronous file I/O in event handlers**: Every call to `parse_project_state()` in `update()` or `handle_key()` blocks the render loop. After this phase, no sync I/O should remain on the main thread path.
- **Hardcoding completed_phases+1 as current phase**: The P5:Unknown bug. Always check bounds before computing phase references.
- **Flat enum with data variants**: `InputMode::AddPath { alias }` requires cloning the alias on every key press. Screen structs own their data without cloning.

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| YAML frontmatter parsing | Custom YAML parser | `serde_yml::from_str` (already used) | Edge cases in YAML quoting, multi-line strings |
| Terminal event handling | Raw terminal control | `crossterm::event::EventStream` (already used) | Cross-platform, handles resize, paste, focus events |
| File change debouncing | Custom timer logic | `notify-debouncer-full` (already used) | Handles rename vs modify vs create correctly |
| Path canonicalization | Manual `../` resolution | `std::fs::canonicalize()` (already used at `app.rs:875`) | Handles symlinks, relative paths, home expansion |

## Common Pitfalls

### Pitfall 1: InputMode Refactor Breaks Existing Behavior
**What goes wrong:** Screen refactor introduces regressions in key handling -- modal flows (AddAlias -> AddPath) lose state, Escape doesn't work correctly, help overlay stops appearing.
**Why it happens:** The refactor touches every input handling path simultaneously.
**How to avoid:** Migrate one screen at a time. Start with `NormalScreen`, verify all keybindings work. Then `DetailScreen`. Then modal flows. Run existing tests after each migration. Keep `InputMode` enum alive during migration -- delete it only after all screens are migrated.
**Warning signs:** Tests fail after removing InputMode. Key sequences that worked before (e.g., `a` -> type alias -> Enter -> type path -> Enter) stop working.

### Pitfall 2: Async Migration Creates Race Conditions
**What goes wrong:** Two `FileChanged` events arrive in quick succession. Both spawn `spawn_blocking` tasks. The second task's result arrives before the first's. The first result overwrites the second, showing stale state.
**Why it happens:** `spawn_blocking` tasks execute on a thread pool with no ordering guarantees.
**How to avoid:** Use the existing 500ms dedup in the `FileChanged` handler (`app.rs:241-245`). Additionally, include a generation counter or timestamp in `ProjectStateLoaded` -- only apply if the generation matches or is newer than what's currently stored.
**Warning signs:** State flickers between old and new values after rapid file changes.

### Pitfall 3: Plan Counting Regex Fix Is Incomplete
**What goes wrong:** Fix matches standalone `PLAN.md` but still misses other formats used in the wild. Different GSD projects format ROADMAP.md plan lists differently -- some use indented bullets, some use numbered lists, some have plan descriptions after the filename.
**Why it happens:** The fix focuses on the known missing pattern without checking what other GSD projects actually produce.
**How to avoid:** Check the GSD source for how ROADMAP.md is written. The canonical plan line format is: `- [x] NN-MM-PLAN.md -- description` or `- [x] PLAN.md`. The regex should match both. Test against the actual v1.0 archived phases in this project AND at least one other GSD project's ROADMAP.md.
**Warning signs:** Plan counts are correct for this project but wrong for others.

### Pitfall 4: Disk Status Scanning Archived Phases
**What goes wrong:** The disk_status module only scans `.planning/phases/` and misses phases that have been archived to `.planning/milestones/v1.0-phases/`. The dashboard shows completed milestone phases as "no_directory" instead of "complete".
**Why it happens:** GSD archives completed milestone phases. The archive path is `.planning/milestones/{milestone_version}-phases/{phase_slug}/`.
**How to avoid:** When computing disk_status for a phase, check both `.planning/phases/` AND `.planning/milestones/*/` for the phase directory. If the phase is found in the milestones archive, its status is "complete" regardless of file contents (archived = done).
**Warning signs:** After running `/gsd:complete-milestone`, dashboard shows all phases as "no_directory".

### Pitfall 5: CLI Alias Auto-Derive Conflicts
**What goes wrong:** Auto-deriving alias from folder name produces conflicts. Two projects at `/home/user/work/my-app` and `/home/user/personal/my-app` both get alias "my-app".
**Why it happens:** `path.file_name()` only returns the last component.
**How to avoid:** Check for alias uniqueness after auto-derivation. If a conflict exists, error with a clear message telling the user to provide an explicit alias. Do NOT auto-disambiguate (e.g., "my-app-2") -- that creates confusing names.
**Warning signs:** Silent overwrite of existing project registration.

## Code Examples

### Fix 1: Plan Counting Regex (STATE-01)

**Current regex (roadmap_md.rs:19):**
```rust
let plan_re = Regex::new(
    r"^\s*- \[([ xX])\] \d+-\d+-PLAN\.md"
).unwrap();
```

**Fixed regex:**
```rust
let plan_re = Regex::new(
    r"^\s*- \[([ xX])\] (?:\d+-\d+-)?PLAN\.md"
).unwrap();
```

This makes the `NN-MM-` prefix optional, matching both `01-01-PLAN.md` and `PLAN.md`.

Verified against GSD source (`roadmap.cjs:142`):
```javascript
planCount = phaseFiles.filter(f => f.endsWith('-PLAN.md') || f === 'PLAN.md').length;
```

### Fix 2: P5:Unknown Bug (STATE-02)

**Current code (state_reader/mod.rs:46-51):**
```rust
if !fm.stopped_at.is_empty() {
    state.current_phase = fm.stopped_at.clone();
} else {
    state.current_phase = format!("Phase {}", fm.progress.completed_phases + 1);
}
```

**Fixed code:**
```rust
if !fm.stopped_at.is_empty() {
    state.current_phase = fm.stopped_at.clone();
} else if fm.progress.completed_phases >= fm.progress.total_phases && fm.progress.total_phases > 0 {
    // All phases complete -- show milestone name
    state.current_phase = if !fm.milestone.is_empty() {
        format!("{} Complete", fm.milestone)
    } else {
        "Complete".to_string()
    };
} else {
    state.current_phase = format!("Phase {}", fm.progress.completed_phases + 1);
}
```

**Also fix format_phase_display() (app.rs:80-89):**
```rust
pub fn format_phase_display(state: &ProjectState) -> String {
    if state.completed_phases >= state.total_phases && state.total_phases > 0 {
        if !state.milestone.is_empty() {
            return format!("{} Complete", state.milestone);
        }
        return "Complete".to_string();
    }
    let phase_num = state.completed_phases + 1;
    let phase_name = state
        .phases
        .iter()
        .find(|p| p.number == phase_num.to_string())
        .map(|p| p.name.as_str())
        .unwrap_or("Unknown");
    format!("P{}: {}", phase_num, phase_name)
}
```

### Fix 3: CLI Alias Optional (CLI-01)

**Current (cli.rs:18-23):**
```rust
Add {
    alias: String,
    path: PathBuf,
}
```

**Fixed:**
```rust
Add {
    /// Path to the project root (must contain .planning/)
    path: PathBuf,
    /// Optional alias (defaults to last folder component of path)
    alias: Option<String>,
}
```

**Usage in main.rs handler:**
```rust
Commands::Add { path, alias } => {
    let alias = alias.unwrap_or_else(|| {
        path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unnamed")
            .to_string()
    });
    // ... existing add logic with this alias
}
```

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Flat InputMode enum | Screen trait + stack | ratatui 0.28+ community pattern | Scalable to 20+ screens without combinatorial explosion |
| Sync file I/O in event loop | spawn_blocking + action dispatch | Standard since tokio 1.0 | Non-blocking render loop, responsive TUI |
| ROADMAP.md checkbox only | Disk artifact inference + ROADMAP.md | GSD's own algorithm | Ground truth from files, not stale checkboxes |

## Open Questions

1. **DiskStatus ordering for pipeline display**
   - What we know: GSD's 7 statuses form a natural ordering (no_directory < empty < discussed < ... < complete)
   - What's unclear: How to map this to the 5-stage `D-R-P-E-V` pipeline display. "Partial" maps to Execute, "Complete" could mean executed-but-not-verified OR fully-done.
   - Recommendation: Treat DiskStatus::Complete as "all plans executed" (the V stage requires VERIFICATION.md which is a separate check). For now, Complete = E stage done. Add V stage detection as a VERIFICATION.md file check.

2. **Screen trait object safety**
   - What we know: `Box<dyn Screen>` requires the trait to be object-safe (no generic methods, no `Self` in return types)
   - What's unclear: Whether `render(&self, frame: &mut Frame, area: Rect, ctx: &AppContext)` with `Frame` (which uses generics internally) is object-safe
   - Recommendation: ratatui's `Frame` is not generic since 0.26. The trait as designed is object-safe. Verify during implementation.

3. **Compact pipeline on dashboard vs current "Status" column**
   - What we know: D-02 says compact pipeline in status column, expanded on focus
   - What's unclear: Whether the compact pipeline replaces the existing Status column entirely or is added as a new column
   - Recommendation: Replace the Status column content. The pipeline IS the status. The existing status text ("planning", "executing", etc.) becomes the expanded form shown on focus.

## Project Constraints (from CLAUDE.md)

- **State reading**: Must not require running Claude/GSD to check status -- read from files or cached state
- **Non-intrusive**: Must not interfere with running GSD instances on active projects
- **Portability**: Should work for any GSD user, not hardcoded to one user's setup
- **Stack**: Rust 1.85+, ratatui 0.30.0, crossterm 0.29.0, tokio 1.x
- **No new dependencies** for this phase (all needed crates already in Cargo.toml)
- **GSD Workflow**: Use `/gsd:execute-phase` for planned work; do not make direct edits outside GSD workflow
- **Architecture patterns**: TEA (The Elm Architecture), `tokio::select!` for event racing, state mutated only by message handlers, render driven by state
- **Avoid**: `std::sync::Mutex` in async code, `env_logger` (use tracing), `tui-rs` (use ratatui)

## Sources

### Primary (HIGH confidence)
- `src/state_reader/mod.rs` -- direct source: P5:Unknown bug at line 50, sync I/O pattern
- `src/state_reader/roadmap_md.rs` -- direct source: plan counting regex at line 19, test fixtures
- `src/app.rs` -- direct source: InputMode enum (11 variants), handle_key dispatch, FileChanged handler, format_phase_display bug
- `src/cli.rs` -- direct source: Add command struct with required alias
- `src/ui/project_list.rs` -- direct source: status column rendering, adaptive layout
- `src/ui/detail_view.rs` -- direct source: phase list rendering, sub-view toggle
- GSD `roadmap.cjs:127-166` -- disk_status algorithm: file detection patterns for 7 statuses
- GSD `roadmap.cjs:142-143` -- plan/summary counting: `f.endsWith('-PLAN.md') || f === 'PLAN.md'`
- GSD `roadmap.cjs:164-166` -- ROADMAP checkbox overrides disk status for completed phases
- `.planning/milestones/v1.0-phases/01-core-infrastructure/` -- archived phase file structure (verified: PLAN.md, SUMMARY.md, CONTEXT.md, RESEARCH.md, VERIFICATION.md present)

### Secondary (MEDIUM confidence)
- [ratatui component architecture docs](https://ratatui.rs/concepts/application-patterns/component-architecture/) -- Screen trait pattern for scaling TUI apps
- `.planning/research/PITFALLS.md` -- InputMode explosion analysis, async I/O blocking analysis
- `.planning/research/ARCHITECTURE.md` -- TEA pattern description, Action enum growth plan

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH -- no new dependencies, all verified in Cargo.toml
- Architecture (screen refactor): HIGH -- pattern well-documented in ratatui community, codebase analyzed line-by-line
- Architecture (async migration): HIGH -- spawn_blocking pattern already exists in codebase (`app.rs:820`)
- Bug fixes (STATE-01, STATE-02): HIGH -- bugs located to exact lines, fixes verified against GSD source
- Disk inference (STATE-03): HIGH -- algorithm verified directly from GSD `roadmap.cjs` source
- CLI change (CLI-01): HIGH -- clap Optional<String> is standard pattern

**Research date:** 2026-03-26
**Valid until:** 2026-04-26 (stable -- no external dependency changes expected)
