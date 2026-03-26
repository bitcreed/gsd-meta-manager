# Phase 2: Dashboard and Navigation - Context

**Gathered:** 2026-03-25
**Status:** Ready for planning

<domain>
## Phase Boundary

Deliver a polished, color-coded project dashboard with aggregate status bar, vim-style keyboard navigation, live search/filter with column selectors, a help overlay, and terminal resize handling. Builds on the Phase 1 stub table and TEA event loop.

</domain>

<decisions>
## Implementation Decisions

### Dashboard Layout & Density
- **D-01:** Columns: `Alias | Current Phase Name | Status | Progress (e.g., 2/4) | Backlog` — rich info per row, path moved to detail view
- **D-02:** Progress shown as fraction text only (`2/4 phases`) — no inline progress bars
- **D-03:** Current phase displayed as number + truncated name: `"P2: Dashboard and Nav"` — compact but informative

### Color-Coding Scheme
- **D-04:** Workflow state color mapping:
  | State | Color |
  |-------|-------|
  | Active / In Progress | Green |
  | Idle / Ready to plan | Yellow |
  | Blocked | Red |
  | Complete | Dim/Gray |
  | Unknown / Error | Magenta |
- **D-05:** Selected row uses bold + underline (not reverse video) — preserves status color so users can see state while navigating

### Status Bar Design
- **D-06:** Aggregate counts use icon shorthand: `5 projects: 2 ▶ 1 ⚠ 1 ● 1 ✓` — dense, scannable
- **D-07:** Single-line footer: aggregate counts on left, keybind hints on right — same layout as Phase 1 but richer content

### Search/Filter Behavior
- **D-08:** Filter triggered by `/`, inline in footer — replaces keybind hints with filter input, live-filters as you type, Esc to clear and restore hints
- **D-09:** Filter matches across all visible columns (name, status, phase name) with fuzzy matching
- **D-10:** Column selector syntax: `/term/column_key` where column keys are:
  - `p` = phase (e.g., `/4/p` highlights rows where phase contains "4")
  - `n` = name/alias (e.g., `/Foo/n` filters to alias matching "Foo")
  - `s` = status (e.g., `/Done/s` filters to status matching "Done")
  - No suffix = match across all columns (default fuzzy)

### Claude's Discretion
- Help overlay (`?`) layout and content — standard keybind reference panel
- Exact fuzzy matching algorithm (simple substring vs scored fuzzy)
- Terminal resize handling strategy (reflow vs redraw)
- Status icon choice (Unicode symbols above are suggestions, can adjust for terminal compatibility)
- How to truncate phase names when terminal is narrow

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project context
- `.planning/PROJECT.md` — Core value, constraints, key decisions
- `.planning/REQUIREMENTS.md` — DASH-01 through DASH-03, NAV-01 through NAV-05
- `.planning/ROADMAP.md` — Phase 2 success criteria and dependency chain

### Prior phase artifacts
- `.planning/phases/01-core-infrastructure/01-CONTEXT.md` — Phase 1 decisions (TEA pattern, named aliases, graceful degradation)
- `.planning/research/ARCHITECTURE.md` — TEA pattern, component structure, EventBus design
- `.planning/research/PITFALLS.md` — Terminal cleanup, render loop pitfalls

### Existing implementation
- `src/app.rs` — App struct, InputMode enum, key handling, update loop
- `src/ui/project_list.rs` — Current table rendering, footer, empty state
- `src/state_reader/mod.rs` — ProjectState struct with all fields available for dashboard columns
- `src/action.rs` — Action enum for TEA message passing

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `App` struct with `TableState`, `InputMode` enum, `project_states: HashMap<String, ProjectState>` — direct foundation for dashboard
- `ProjectState` already exposes: `status`, `current_phase`, `total_phases`, `completed_phases`, `total_plans`, `completed_plans`, `milestone`, `backlog_count`, `phases`, `gsd_mode` — all columns can be derived without new parsing
- `ui/project_list.rs` already handles: table rendering, empty state, footer with keybind hints, adaptive column hiding at narrow widths
- `sorted_aliases()` and `selected_alias()` helpers for consistent ordering and selection

### Established Patterns
- TEA: `Action` enum → `App::update()` → `ui::render()` — all new features follow this
- `InputMode` enum for modal key handling — extend with `Search` variant for filter mode
- `RawKey(KeyEvent)` action — `handle_key` dispatches by mode, so adding filter mode follows the existing pattern
- Footer rendering already switches on `InputMode` — filter input footer slots in naturally
- Adaptive layout: already hides Path column when terminal < 60 chars — same pattern for other responsive adjustments

### Integration Points
- `render_main()` in `project_list.rs` — where table columns, colors, and highlight style change
- `render_footer()` — where status bar and filter input render
- `handle_normal_key()` — where `/` and `?` keybindings get added
- `App::update()` — where new Actions (filter, help toggle) get processed

</code_context>

<specifics>
## Specific Ideas

- Filter selector syntax `/term/key` is a distinctive UX choice — implement as: parse last `/x` suffix, if `x` is a known column key use it, otherwise treat the whole string as a global filter
- Icon shorthand in status bar (`▶ ⚠ ● ✓`) — consider fallback to ASCII (`> ! . +`) if terminal doesn't support Unicode, or just require Unicode support (modern terminals)
- Bold + underline for selection (not reverse video) is deliberate — the user wants to see status colors while navigating

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 02-dashboard-and-navigation*
*Context gathered: 2026-03-25*
