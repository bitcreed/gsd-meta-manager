# Phase 3: Live State and Detail View - Context

**Gathered:** 2026-03-25
**Status:** Ready for planning

<domain>
## Phase Boundary

Add file-system watching for live dashboard updates and a drill-down detail view for individual projects. The dashboard auto-refreshes when `.planning/` files change, and pressing Enter on a project shows a full-screen detail panel with roadmap phase breakdown and change summary.

</domain>

<decisions>
## Implementation Decisions

### Detail View Layout
- **D-01:** Enter replaces the main table with a full-screen detail panel — Esc returns to project list. Consistent with TUI modal pattern established in Phase 2.
- **D-02:** Detail view shows: project path, all roadmap phases with status icons (○ pending, ◆ in-progress, ✓ complete), current phase highlighted, plan completion counts per phase (e.g., `✓ P1: Core Infrastructure  3/3 plans`)
- **D-03:** Per-phase status displayed as a vertical list with phase number, name, status icon, and plan count
- **D-04:** Change summary appears at top of detail view as a highlighted banner (e.g., "Phase 3 completed 2h ago") — visible immediately on drill-in

### Auto-Refresh Behavior
- **D-05:** File watching via notify 8.x with debouncer (200ms), watching all registered projects' `.planning/` directories. Events flow through the existing tokio mpsc EventBus channel (architecture set up in Phase 1).
- **D-06:** Silent refresh — table updates without flash/blink. Status bar briefly shows "Updated: projectname" for 2 seconds after a refresh.
- **D-07:** Change tracking is per-app-launch only (in-memory) — no persistence to config.json. Record initial state snapshot at startup, compare on each file-change event.
- **D-08:** Only phase completions and status transitions are worth summarizing (e.g., "Phase 2 completed", "Status: idle → active") — not every `.planning/` file write.

### Claude's Discretion
- notify debouncer crate choice (notify-debouncer-mini vs notify-debouncer-full)
- Detail view scrolling behavior if roadmap has many phases
- How to handle watching projects on paths that become unavailable (unmounted, deleted)
- Change detection algorithm (mtime-based vs content hash)

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project context
- `.planning/PROJECT.md` — Core value, constraints, key decisions
- `.planning/REQUIREMENTS.md` — STATE-04, STATE-05, DET-01, DET-02, DASH-05
- `.planning/ROADMAP.md` — Phase 3 success criteria and dependency chain

### Prior phase artifacts
- `.planning/phases/01-core-infrastructure/01-CONTEXT.md` — D-09/D-10: event channel design ready for file watcher + hooks
- `.planning/phases/02-dashboard-and-navigation/02-CONTEXT.md` — Dashboard layout decisions, color scheme, status classification
- `.planning/research/ARCHITECTURE.md` — TEA pattern, EventBus design, state caching with Arc<RwLock<HashMap>>

### Existing implementation
- `src/app.rs` — App struct, InputMode enum (extend with DetailView), event handling, project_states HashMap, classify_status()
- `src/event.rs` — EventBus with tokio mpsc, crossterm event stream — extend with file watcher events
- `src/ui/project_list.rs` — Current table render (Enter placeholder exists at line 257-262)
- `src/state_reader/mod.rs` — parse_project_state() and ProjectState struct with phases, plan counts
- `src/ui/mod.rs` — Render dispatch (extend with detail view conditional)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `ProjectState` already has: `phases: Vec<RoadmapPhase>` with name/completed, `total_plans`/`completed_plans`, `status`, `current_phase` — most detail view data is already parsed
- `classify_status()` in app.rs maps status strings to workflow states — reuse for detail view phase coloring
- `status_color()` in project_list.rs maps states to colors — reuse for detail view
- `InputMode` enum already supports modal switching — add `DetailView { alias: String }` variant
- Event loop in `event.rs` already uses `tokio::select!` — add notify watcher stream as another branch

### Established Patterns
- TEA pattern: Action enum → App::update() → render — new features follow this
- InputMode for modal views — Enter → DetailView, Esc → Normal
- Footer adapts per mode — detail view can show its own keybind hints
- Adaptive layout based on terminal width — detail view should also adapt

### Integration Points
- `event.rs` — Add file watcher initialization and event mapping to Action::FileChanged
- `app.rs` handle_normal_key Enter → switch to InputMode::DetailView
- `ui/mod.rs` — Dispatch to detail_view::render when in DetailView mode
- `app.rs` load_project_states() — currently synchronous, may need to become event-driven for live updates

</code_context>

<specifics>
## Specific Ideas

- Change summary should be human-readable time ("2h ago", "just now") not ISO timestamps
- Detail view should feel like a read-only info panel, not a form — no editing in Phase 3
- The "Updated: projectname" status message on refresh reuses the existing status_message mechanism with 2s timeout

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 03-live-state-and-detail-view*
*Context gathered: 2026-03-25*
