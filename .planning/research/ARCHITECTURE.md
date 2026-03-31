# Architecture Patterns

**Domain:** v1.2 Feature Integration (Archive Browser, Paused Detection, Queue Execution Research)
**Researched:** 2026-03-31
**Confidence:** HIGH (based on direct codebase analysis of v1.1 shipped code and GSD workflow file inspection)

## Existing Architecture Summary (Post-v1.1)

The app follows TEA (The Elm Architecture) with a screen stack:

- **App** struct holds `AppContext` and `screen_stack: Vec<Box<dyn Screen>>`
- **Action** enum (7 variants): Tick, RawKey, Resize, FileChanged, ProjectStateLoaded, GitLogLoaded, GitDiffStatLoaded, SessionsDetected, CreateProjectResult
- **Screen** trait: `handle_key()` returns `ScreenAction` (None/Push/Pop/Quit/SetStatusMessage/DispatchAction)
- **DetailScreen** has 7 tabs via `DetailSubView` enum: PhaseList, RoadmapViz, Backlog, GitHistory, Pipeline, Queue, Sessions
- **TAB_TITLES** constant: `["1:Phases", "2:Roadmap", "3:Backlog", "4:Git", "5:Pipeline", "6:Queue", "7:Sessions"]`
- **ProjectViewCache** holds async-loaded per-project tab data (backlog items, git entries, diff stats, selection indices)
- **StateReader** (`parse_project_state()`) reads `.planning/` into `ProjectState` struct -- called on startup and on every `FileChanged`
- **FileWatcher** via notify-debouncer-full triggers `Action::FileChanged` on `.planning/` directory changes
- **Session detection** via `pgrep`/`/proc` polls every 20 ticks (~5s), stores in `App.active_sessions`
- All state mutation flows through `App::update()`; rendering is pure

**Key data flow:**
```
FileWatcher -> Action::FileChanged -> spawn_blocking(parse_project_state)
  -> Action::ProjectStateLoaded -> App.ctx.project_states.insert() -> redraw
```

## New Features and Integration Points

### 1. Paused-Project Detection (HANDOFF.json)

**What it is:** GSD's `/gsd:pause-work` writes `.planning/HANDOFF.json` with structured pause state. Projects with this file are paused and should show a badge on the dashboard.

**Data source:** `.planning/HANDOFF.json` -- JSON file with fields:
- `status`: always `"paused"`
- `timestamp`: ISO 8601 when paused
- `phase`: current phase number or `"milestone-complete"`
- `phase_name`: human-readable phase name
- `next_action`: what to do when resuming
- `blockers`: array of `{description, type, workaround}` objects
- `human_actions_pending`: array of `{action, context, blocking}` objects
- `context_notes`: mental state summary

Note: HANDOFF.json is deleted on resume (`/gsd:resume-work`). Its presence means the project is paused.

**Integration approach -- extend existing StateReader:**

#### New: `state_reader/handoff.rs` (~50 lines)

```rust
use std::path::Path;

#[derive(Debug, Clone, Default)]
pub struct PauseInfo {
    pub timestamp: String,
    pub phase: String,
    pub phase_name: String,
    pub next_action: String,
    pub blocker_count: usize,
    pub human_actions_count: usize,
    pub context_notes: String,
}

pub fn parse_handoff(planning_dir: &Path) -> Option<PauseInfo> {
    let path = planning_dir.join("HANDOFF.json");
    let content = std::fs::read_to_string(&path).ok()?;
    let json: serde_json::Value = serde_json::from_str(&content).ok()?;
    // Extract fields from JSON, return PauseInfo
    Some(PauseInfo {
        timestamp: json["timestamp"].as_str()?.to_string(),
        phase: json["phase"].as_str().unwrap_or("").to_string(),
        phase_name: json["phase_name"].as_str().unwrap_or("").to_string(),
        next_action: json["next_action"].as_str().unwrap_or("").to_string(),
        blocker_count: json["blockers"].as_array().map(|a| a.len()).unwrap_or(0),
        human_actions_count: json["human_actions_pending"].as_array().map(|a| a.len()).unwrap_or(0),
        context_notes: json["context_notes"].as_str().unwrap_or("").to_string(),
    })
}
```

#### Modified: `ProjectState` (state_reader/mod.rs)

Add two fields:

```rust
pub struct ProjectState {
    // ... existing fields ...
    pub is_paused: bool,
    pub pause_info: Option<PauseInfo>,
}
```

#### Modified: `parse_project_state()` (state_reader/mod.rs)

Add after queue loading (line ~102):

```rust
// Check for HANDOFF.json (paused project)
if let Some(info) = handoff::parse_handoff(planning_dir) {
    state.is_paused = true;
    state.pause_info = Some(info);
}
```

#### Modified: Dashboard alias cell rendering (ui/screens/normal.rs, ~line 344)

The existing session badge pattern is the template. Insert paused check with higher priority:

```rust
let is_paused = ctx.project_states.get(alias)
    .map(|s| s.is_paused)
    .unwrap_or(false);

let alias_cell: Line = if is_paused {
    Line::from(vec![
        Span::styled("\u{23f8} ", Style::default().fg(Color::Cyan)),
        Span::raw(alias.clone()),
    ])
} else if has_session {
    // existing session badge (green triangle)
    Line::from(vec![
        Span::styled("\u{25b6} ", Style::default().fg(Color::Green)),
        Span::raw(alias.clone()),
    ])
} else {
    Line::from(alias.clone())
};
```

**Decision: Do NOT add a new `StatusCategory::Paused` variant.** Paused maps to Idle (yellow). The visual distinction comes from the badge icon. Adding a variant forces changes to every `match` on StatusCategory.

#### Modified: Detail view PhaseList tab

When `is_paused` is true, render a pause info panel above the phase list:

```
 PAUSED  since 2026-03-27
 Phase: v1.0 archived
 Next: Set up Railway credentials
 Blockers: 2 | Human actions: 4
```

This is a conditional `Paragraph` widget rendered in the existing PhaseList layout -- no new screen or tab needed.

#### File watcher integration

Already handled automatically. HANDOFF.json lives in `.planning/`, which the watcher monitors. When it appears/disappears, `Action::FileChanged` fires, `parse_project_state` re-runs, and `is_paused` updates.

---

### 2. Milestone Archive Browser Tab

**What it is:** A new 8th tab in DetailScreen for browsing completed milestones and drilling into archived phase artifacts.

**Data source:** `.planning/milestones/` directory structure (verified on disk):
```
milestones/
  v1.0-ROADMAP.md          # Full phase details, milestone summary
  v1.0-REQUIREMENTS.md     # Archived requirements with final status
  v1.0-MILESTONE-AUDIT.md  # YAML frontmatter with scores + gaps
  v1.0-phases/             # Phase directories with all artifacts
    01-core-infrastructure/
      01-CONTEXT.md, 01-01-PLAN.md, 01-01-SUMMARY.md,
      01-VERIFICATION.md, 01-UI-SPEC.md, etc.
    02-dashboard-and-navigation/
      ...
  v1.1-ROADMAP.md
  v1.1-MILESTONE-AUDIT.md
  v1.1-REQUIREMENTS.md
  v1.1-phases/
    05-state-reader-accuracy/
      ...
```

#### Modified: `DetailSubView` enum (app.rs)

```rust
pub enum DetailSubView {
    PhaseList,
    RoadmapViz,
    Backlog,
    GitHistory,
    Pipeline,
    Queue,
    Sessions,
    Archives,    // NEW -- 8th tab
}
```

Update TAB_TITLES to 8 elements:
```rust
const TAB_TITLES: [&str; 8] = [
    "1:Phases", "2:Roadmap", "3:Backlog", "4:Git",
    "5:Pipeline", "6:Queue", "7:Sessions", "8:Archives"
];
```

Update `tab_index()` and `sub_view_from_index()` with the new variant.

#### New: `state_reader/milestones.rs` (~120 lines)

Data structures and scanning logic:

```rust
#[derive(Debug, Clone)]
pub struct ArchivedMilestone {
    pub version: String,           // "v1.0"
    pub name: String,              // extracted from ROADMAP.md header
    pub audit_status: String,      // from AUDIT.md YAML: "passed", "tech_debt", "gaps_found"
    pub shipped_date: String,      // from ROADMAP.md
    pub phase_count: usize,
    pub phases: Vec<ArchivedPhase>,
    pub has_audit: bool,
    pub has_requirements: bool,
    pub top_level_files: Vec<String>, // ["v1.0-ROADMAP.md", "v1.0-MILESTONE-AUDIT.md", ...]
}

#[derive(Debug, Clone)]
pub struct ArchivedPhase {
    pub dir_name: String,          // "01-core-infrastructure"
    pub number: String,            // "01"
    pub name: String,              // "core-infrastructure" -> "Core Infrastructure"
    pub artifact_count: usize,
    pub artifacts: Vec<String>,    // file names in the phase dir
}

/// Scan .planning/milestones/ for archived milestones.
/// Lightweight: reads directory listings and a few header lines, not full file contents.
pub fn scan_milestones(planning_dir: &Path) -> Vec<ArchivedMilestone> {
    // 1. List milestones/ dir entries
    // 2. Group by version prefix (v1.0-*, v1.1-*, etc.)
    // 3. For each version:
    //    a. Check for -ROADMAP.md, -REQUIREMENTS.md, -MILESTONE-AUDIT.md
    //    b. Parse milestone name from ROADMAP.md first heading
    //    c. Parse audit status from AUDIT.md YAML frontmatter
    //    d. List -phases/ subdirectories, for each list artifact files
    // 4. Sort by version descending (newest first)
}
```

#### Modified: `ProjectViewCache` (ui/screens/mod.rs)

Add archive cache fields:

```rust
pub struct ProjectViewCache {
    // ... existing fields ...
    pub archived_milestones: Vec<ArchivedMilestone>,
    pub archive_milestone_selected: usize,
    pub archive_phase_selected: usize,
    pub archive_artifact_selected: usize,
    pub archive_depth: ArchiveBrowseDepth,
    pub archive_content: Option<String>,  // loaded artifact text
    pub loading_archives: bool,
}

#[derive(Debug, Clone, Default, PartialEq)]
pub enum ArchiveBrowseDepth {
    #[default]
    MilestoneList,      // List of milestones (v1.0, v1.1, ...)
    PhaseList,          // Phases within selected milestone
    ArtifactList,       // Files within selected phase
    ArtifactContent,    // Viewing file content (scrollable)
}
```

#### Navigation pattern

Drill-down with Enter/Escape, matching existing Backlog expand/collapse UX:

```
Milestones              Phases                  Artifacts               Content
v1.0 MVP (4 phases)  > 01: Core Infra        > 01-CONTEXT.md        > (scrollable text)
v1.1 Polish (5 ph)     02: Dashboard           01-01-PLAN.md
                        03: Live State          01-01-SUMMARY.md
                        04: Viz & Create        01-VERIFICATION.md
```

- **Enter**: drill deeper
- **Escape**: back one level (at MilestoneList level, Escape exits to previous screen like other tabs)
- **j/k or Up/Down**: navigate within current level
- **Scrolling in ArtifactContent**: same mechanism as existing backlog content preview

At MilestoneList level, also show top-level files (ROADMAP.md, AUDIT.md, REQUIREMENTS.md) as browsable items alongside the phases directory.

#### New Action variants

```rust
pub enum Action {
    // ... existing ...
    ArchivesLoaded {
        alias: String,
        milestones: Vec<ArchivedMilestone>,
    },
    ArchiveContentLoaded {
        alias: String,
        content: String,
    },
}
```

Loading follows the established async pattern:
1. Tab activation checks `ProjectViewCache.archived_milestones`
2. If empty, sets `loading_archives = true`, spawns `tokio::task::spawn_blocking`
3. Blocking task sends `Action::ArchivesLoaded` through event bus
4. `App::update()` stores in cache and triggers redraw
5. Artifact content similarly loaded via `ArchiveContentLoaded`

#### Cache invalidation

`Action::FileChanged` for a project clears `archived_milestones` from its cache. Since milestone archives change only at milestone boundaries (rare), this is fine as a simple clear-on-any-change strategy. The next tab visit triggers a re-scan.

---

### 3. Queue Execution Research (Data Model Implications)

**What it is:** Research-only phase. Documents how GSD could auto-execute queue items. No code changes -- informs future data model design.

#### GSD Autonomous Workflow Analysis

From reading `autonomous.md`, the key lifecycle:

1. **Initialize**: `init milestone-op` bootstraps context, reads STATE.md/ROADMAP.md
2. **Discover phases**: `roadmap analyze` finds incomplete phases, sorts by number
3. **Per-phase execution loop**:
   - Smart discuss (or skip if infrastructure) -> writes CONTEXT.md
   - Plan (`gsd:plan-phase N`) -> writes PLAN.md files
   - Execute (`gsd:execute-phase N --no-transition`) -> writes SUMMARY.md files
   - Post-execution routing on VERIFICATION.md status (passed/human_needed/gaps_found)
   - Gap closure limited to 1 retry
4. **Iterate**: Re-reads ROADMAP.md (catches decimal phase insertions), checks STATE.md for blockers
5. **Lifecycle**: audit -> complete-milestone -> cleanup

**Key insight: Autonomous mode has NO awareness of QUEUE.md.** There is no hook point where queue items are consumed. Queue items are free-form text strings (GSD commands, notes, reminders) -- not structured phase definitions.

#### Hook points for future queue execution

Three possible integration strategies:

**(a) Launch-time injection (recommended for v1.3+):**
The TUI launches a Claude session with queue items as the initial prompt. Example: `claude -p "/gsd:quick Fix the login button"` in the project directory. This is the simplest approach and already possible with v1.1's session launch feature. The missing piece is a "run next queue item" action that:
1. Takes the first pending queue item
2. Launches `claude -p "<item>"` in a new terminal
3. Marks the item as "running" in the UI (not persisted -- session state only)

**(b) Autonomous pre-phase hook (requires GSD changes):**
Add a `--queue` flag to `/gsd:autonomous` that reads QUEUE.md before each phase and offers to execute pending items. This requires changes to the GSD workflow file, not the TUI.

**(c) Phase insertion (requires GSD changes):**
Convert queue items to decimal phases in ROADMAP.md. Example: queue item "Fix login button" becomes "Phase 5.1: Quick fix -- login button". Autonomous mode would then pick it up in the iterate step. Complex and fragile.

#### Data model recommendation

The current `QueuedAction { command: String }` is sufficient for v1.2. Do NOT add status/timestamp fields yet because:
- QUEUE.md is plain markdown (`- command text`) that GSD reads directly
- Adding structured fields requires a format change (YAML frontmatter, JSON, or custom markers)
- Status tracking is session-ephemeral (a running item in one TUI session is stale the next)

**Future-proofing note:** When queue execution is implemented (v1.3+), the model should expand to:

```rust
pub struct QueuedAction {
    pub command: String,
    pub status: QueueStatus,        // Pending | Running | Done | Failed
    pub created_at: Option<String>,
    pub executed_at: Option<String>,
}
```

But this requires a QUEUE.md format migration (from plain list to structured format). That decision should be made when execution is actually built.

---

## Component Boundaries

| Component | Status | Location | Responsibility |
|-----------|--------|----------|----------------|
| `state_reader/handoff.rs` | **NEW** | src/state_reader/ | Parse HANDOFF.json into PauseInfo |
| `state_reader/milestones.rs` | **NEW** | src/state_reader/ | Scan milestones/ directory for ArchivedMilestone structs |
| `ProjectState` | **MODIFIED** | src/state_reader/mod.rs | Add `is_paused: bool`, `pause_info: Option<PauseInfo>` |
| `parse_project_state()` | **MODIFIED** | src/state_reader/mod.rs | Call `handoff::parse_handoff()` after queue loading |
| `DetailSubView` | **MODIFIED** | src/app.rs | Add `Archives` variant (8th tab) |
| `ProjectViewCache` | **MODIFIED** | src/ui/screens/mod.rs | Add archive browse state fields |
| `Action` enum | **MODIFIED** | src/action.rs | Add `ArchivesLoaded`, `ArchiveContentLoaded` |
| `NormalScreen::render` | **MODIFIED** | src/ui/screens/normal.rs | Add pause badge to alias cell |
| `DetailScreen` | **MODIFIED** | src/ui/screens/detail.rs | Archives tab rendering, pause info panel |
| `App::update()` | **MODIFIED** | src/app.rs | Handle new Action variants |

**Files unchanged:** cli.rs, config.rs, error.rs, event.rs, lib.rs, main.rs, tui.rs, registry.rs, change_tracker.rs, project_creator.rs, session_detector.rs, watcher.rs, all existing state_reader submodules, ui/project_list.rs, ui/roadmap_widget.rs, all existing screen modules except normal.rs and detail.rs

## Data Flow

### Paused Detection Flow

```
FileWatcher detects HANDOFF.json appear/disappear in .planning/
  -> Action::FileChanged { project_path }
  -> spawn_blocking(parse_project_state)
    -> handoff::parse_handoff() reads HANDOFF.json
    -> ProjectState { is_paused: true, pause_info: Some(...) }
  -> Action::ProjectStateLoaded { alias, state }
  -> App::update() stores in project_states HashMap
  -> NormalScreen renders pause badge on alias cell
  -> DetailScreen PhaseList shows pause info panel
```

### Archive Browser Flow

```
User presses '8' or Tab-navigates to Archives tab
  -> DetailScreen sets sub_view to Archives
  -> If cache.archived_milestones empty:
       spawn_blocking(milestones::scan_milestones(planning_dir))
       -> Action::ArchivesLoaded { alias, milestones }
       -> App::update() stores in ProjectViewCache
  -> Render milestone list (from cache)

User presses Enter on milestone
  -> archive_depth = PhaseList
  -> Show phases from cached ArchivedMilestone.phases (no I/O)

User presses Enter on phase
  -> archive_depth = ArtifactList
  -> Show artifact file names from cached ArchivedPhase.artifacts (no I/O)

User presses Enter on artifact
  -> spawn_blocking(fs::read_to_string(artifact_path))
  -> Action::ArchiveContentLoaded { alias, content }
  -> archive_depth = ArtifactContent
  -> Render scrollable Paragraph with raw markdown content

User presses Escape
  -> Go back one depth level (Content -> Artifacts -> Phases -> Milestones -> exit tab)
```

## Patterns to Follow

### Pattern 1: Badge Priority on Dashboard Alias Cell

At most one badge icon per project. Priority order:

1. **Paused** (`\u{23f8}` cyan) -- highest priority, project is inactive
2. **Active session** (`\u{25b6}` green) -- project has running Claude

A paused project with an active session is contradictory (session would consume HANDOFF.json). If both somehow true, pause wins.

### Pattern 2: Async Tab Data Loading (Established)

All tab content follows this pattern (already used by Git, Backlog):

1. Tab activation checks `ProjectViewCache` for existing data
2. If empty: set `loading_X = true`, `spawn_blocking` with I/O work
3. Blocking task sends Action variant through event bus
4. `App::update()` stores in cache, sets `loading_X = false`, triggers redraw
5. Subsequent visits use cached data (invalidated on FileChanged)

Archives follow this identically.

### Pattern 3: Drill-Down Navigation State

`ArchiveBrowseDepth` enum is per-project state in `ProjectViewCache`. Switching between projects preserves each project's browse position (same as how `detail_sub_view_per_project` preserves tab selection).

Key bindings within Archives tab:
- **Enter**: drill deeper
- **Escape**: back one level (at MilestoneList, Escape pops DetailScreen)
- **j/k or Up/Down**: navigate within current level

## Anti-Patterns to Avoid

### Anti-Pattern 1: Loading Full Archive Content Eagerly

Do NOT parse all archived markdown files during `scan_milestones()`. Scan collects directory structure and metadata only (file names, counts, first-line headers). Content is loaded on-demand when user drills into a specific artifact.

**Why:** A project with 5 milestones and 40 archived phases could have 200+ files. Reading them all blocks the render loop and wastes memory.

### Anti-Pattern 2: Putting Archive Data in ProjectState

Do NOT add archived milestone data to `ProjectState`. Archives are view-level data belonging in `ProjectViewCache` (lazily loaded when user opens detail view).

**Why:** `ProjectState` is parsed on every `FileChanged` event for every project. Loading archive data there makes file-change handling slow for all projects, even when the user is not viewing archives.

### Anti-Pattern 3: Creating a New Screen for Archive Browser

Do NOT create a new `Screen` impl for the archive browser. It is a tab within DetailScreen, following the established 7-tab pattern (now 8).

**Why:** The screen stack is for distinct navigation contexts (dashboard -> detail -> confirm dialog). Tabs within DetailScreen are for different data views of the same project.

### Anti-Pattern 4: Adding StatusCategory::Paused

Do NOT add a new variant to the `StatusCategory` enum for paused projects. Paused maps to `Idle` (yellow). The distinction is communicated via the badge icon, not the status color.

**Why:** Every `match` on StatusCategory across the codebase would need updating. The badge provides a clearer visual signal than a color change anyway.

## Build Order (Dependency-Aware)

| Order | Feature | Depends On | New Files | Modified Files |
|-------|---------|------------|-----------|----------------|
| 1 | **Tech debt cleanup** | Nothing | 0 | Multiple (warnings, stale test) |
| 2 | **Paused detection** | Nothing | 1 (handoff.rs) | 3 (mod.rs, normal.rs, detail.rs) |
| 3 | **Archive browser tab** | Nothing | 1 (milestones.rs) | 4 (app.rs, action.rs, mod.rs, detail.rs) |
| 4 | **Queue execution research** | Nothing | 0 | 0 (documentation only) |

**Parallelism:** All four are independent. Paused detection and archive browser touch different code areas:
- Paused: state_reader/mod.rs + dashboard rendering (normal.rs) + detail PhaseList
- Archive: app.rs (enum) + action.rs (enum) + screens/mod.rs (cache) + detail.rs (new tab)

The only shared file is `detail.rs`, but they modify different sections (PhaseList rendering vs new Archives tab case).

**Recommended phase ordering for roadmap:**
1. Tech debt first -- clean foundation
2. Paused detection second -- smallest scope, immediate user value
3. Archive browser third -- largest scope, most new code
4. Queue execution research last or parallel -- no code changes, can run any time

## Sources

- **Codebase analysis:** Direct reading of all 32 source files in `src/` (HIGH confidence)
- **GSD workflow files:** `autonomous.md`, `pause-work.md`, `resume-project.md`, `complete-milestone.md` in `~/.claude/get-shit-done/workflows/` (HIGH confidence)
- **Milestone archive structure:** Direct inspection of `.planning/milestones/` showing v1.0 and v1.1 archives with -ROADMAP.md, -REQUIREMENTS.md, -MILESTONE-AUDIT.md, and -phases/ directories (HIGH confidence)
- **HANDOFF.json format:** Live example from `/home/blk/projects/web/shopify-error-tracker/.planning/HANDOFF.json` showing all fields (HIGH confidence)
- **HANDOFF.json lifecycle:** From `pause-work.md` (writes it) and `resume-project.md` (reads and deletes it) (HIGH confidence)
