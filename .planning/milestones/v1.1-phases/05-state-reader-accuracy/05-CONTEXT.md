# Phase 05: State Reader Accuracy - Context

**Gathered:** 2026-03-26
**Status:** Ready for planning

<domain>
## Phase Boundary

Fix plan counting bugs, completed-milestone display, add disk-based phase inference (GSD's disk_status algorithm), make CLI alias optional with auto-derive from folder name, refactor InputMode to screen/component architecture, and migrate all file I/O to async spawn_blocking.

</domain>

<decisions>
## Implementation Decisions

### Disk Inference
- **D-01:** Claude's discretion on disk inference depth and relationship to ROADMAP.md. Recommended: disk is ground truth for phase status, ROADMAP.md provides structure/names. Use GSD's 7 statuses extended with plan/summary counts where useful for the detail view.

### Status Display
- **D-02:** Dashboard project list shows compact pipeline (`D-R-P-E-V` with color per stage) in the status column. When the row is selected/focused, switch to showing the full text label (e.g., "Executing 2/3").
- **D-03:** When a milestone is fully complete, show milestone name (e.g., "v1.0 Complete") if the milestone version is not already visible elsewhere on the same screen row. Claude's discretion on exact format.

### CLI Ergonomics
- **D-04:** Change `Add` subcommand signature to `add <path> [alias]` — alias becomes optional positional arg, defaults to last path component (folder name).

### Architecture Refactor
- **D-05:** Refactor InputMode enum to screen/component architecture in this phase (prerequisite for Phase 06+). Research says 11 variants will explode past 20 with new views — do it now.

### Async Migration
- **D-06:** Full async migration in this phase. Move all `parse_project_state()` file reads to `spawn_blocking`. Pay the cost once so Phase 06+ features build on an async foundation.

### Claude's Discretion
- Disk inference depth: Claude picks GSD's algorithm + extended counts as appropriate
- Disk vs ROADMAP.md relationship: Claude decides merge strategy
- Completed milestone display format: Claude decides based on screen layout

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### State Reader
- `.planning/research/FEATURES.md` — Feature details including disk_status algorithm (lines 59-71), GSD's roadmap.cjs disk inference logic
- `.planning/research/ARCHITECTURE.md` — Integration architecture for state reader, async patterns
- `.planning/research/PITFALLS.md` — State reader pitfalls, sync I/O debt, InputMode explosion risk

### GSD Internals
- `.planning/research/SUMMARY.md` — Research synthesis with suggested phase ordering and key findings

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `state_reader/roadmap_md.rs`: `parse_roadmap_phases()` — plan counting regex needs fix (line 19: `^\s*- \[([ xX])\] \d+-\d+-PLAN\.md` misses standalone `PLAN.md`)
- `state_reader/mod.rs`: `parse_project_state()` — sync function that reads STATE.md, ROADMAP.md, QUEUE.md; needs async wrapper
- `state_reader/mod.rs`: `count_backlog_items()` — scans `.planning/phases/` for 999* dirs; pattern reusable for disk inference scanning
- `ui/roadmap_widget.rs`: Custom Widget impl pattern — reusable for compact pipeline display

### Established Patterns
- TEA architecture: single App struct, Action enum, mpsc EventBus, stateless render
- `InputMode` enum (11 variants): Normal, AddProjectName, AddProjectPath, RemoveConfirm, Search, DetailView, HelpPopup, DetailRoadmap, QueueInput, CreateProjectName, CreateProjectPath
- `RoadmapPhase` struct: number, name, description, completed bool, total_plans, completed_plans
- `ProjectState` struct: status, current_phase, total_phases, completed_phases, total_plans, completed_plans, milestone, backlog_count, phases vec, queued_actions vec

### Integration Points
- `state_reader/mod.rs:50` — P5:Unknown bug: `format!("Phase {}", completed_phases + 1)` when all phases complete
- `cli.rs:18-23` — `Add { alias: String, path: PathBuf }` needs alias made optional
- `app.rs` — App struct and InputMode enum need screen/component refactor
- `ui/project_list.rs` — Status column rendering needs compact pipeline + focused text label

</code_context>

<specifics>
## Specific Ideas

- Compact pipeline format: `D-R-P-E-V` with per-stage colors (green=done, yellow=current, dim=pending)
- On row focus, expand to full text like "Executing 2/3" or "Planned (3 plans)"
- Completed milestone: show "v1.0 Complete" when milestone name not visible elsewhere on row

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 05-state-reader-accuracy*
*Context gathered: 2026-03-26*
