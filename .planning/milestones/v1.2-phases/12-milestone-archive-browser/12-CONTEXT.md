# Phase 12: Milestone Archive Browser - Context

**Gathered:** 2026-03-31
**Status:** Ready for planning

<domain>
## Phase Boundary

Add a Milestone Archive Browser tab to the detail view. Users can browse completed milestones, drill into phases and artifacts, and view files with styled markdown rendering — all without leaving the TUI. Archive data loads asynchronously and is cached in memory.

</domain>

<decisions>
## Implementation Decisions

### Archive Tab Navigation & Drill-Down
- Sequential list drill-down model (back with Esc) — matches existing Backlog/Queue patterns using `ListState`
- Breadcrumb header bar showing current depth: `Archive > v1.0 > Phase 01 > PLAN.md`
- `Esc` returns to parent level (consistent with existing detail view escape behavior)
- Archive tab is the 8th tab (after Sessions) — index 7 in `DetailSubView` enum

### Markdown Rendering & Styling
- Custom parser using line-by-line regex (no pulldown-cmark dependency) — `#` headers get bold, triple-backtick code blocks get DarkGray background, `**bold**` gets bold, `- ` lists get bullet prefix
- Code blocks: DarkGray background with no syntax highlighting — distinguishable from prose
- Full file content in a scrollable Paragraph widget (up/down to scroll)
- Header tiers: Bold + Cyan for `#`, Bold for `##`, Bold + dim underline for `###`

### Async Loading & Caching Strategy
- Discover archived milestones by scanning `.planning/milestones/` for `v*-ROADMAP.md` files — extract version from filename
- In-memory `HashMap<String, MilestoneArchive>` in `AppContext` — populated on first access per milestone, never invalidated (archived milestones are immutable)
- `tokio::spawn_blocking` for file I/O, send results via existing `mpsc` channel — same pattern as session detection
- "Loading..." text in the archive pane while data loads

### Claude's Discretion
- Internal data structures for `MilestoneArchive`, `PhaseArchive`, etc.
- How to extract phase list from archived ROADMAP.md (parse or filename scan)
- Scroll position tracking per drill-down level
- Whether to show file sizes or modification dates in the artifact list
- Tab bar overflow handling at 80 columns (noted as concern in STATE.md — resolve or defer)

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Existing archive structure
- `.planning/milestones/` — Contains archived ROADMAP.md, REQUIREMENTS.md, MILESTONE-AUDIT.md, and phase subdirectories per milestone
- `.planning/milestones/v1.0-phases/` — Example of archived phase directory with artifacts
- `.planning/milestones/v1.1-phases/` — Another example of archived phase artifacts

### Existing tab/view patterns
- `src/ui/screens/detail.rs` — DetailScreen with 7 existing tabs, `tab_index()`, `sub_view_from_index()` functions
- `src/app.rs:DetailSubView` — Enum of existing tab views to extend
- `src/ui/screens/detail.rs` (Backlog tab) — Reference for ListState-based drill-down pattern
- `src/ui/screens/detail.rs` (GitHistory tab) — Reference for scrollable list with content preview

### Async loading patterns
- `src/session_detector.rs` — `detect_sessions()` called via `tokio::spawn_blocking` (reference pattern)
- `src/app.rs` — `mpsc` channel setup for async event delivery

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `ListState` used throughout detail.rs for scrollable, selectable lists (backlog, git history, queue, sessions)
- `DetailSubView` enum in `app.rs` defines tabs — needs `Archive` variant added
- `tab_index()` and `sub_view_from_index()` map between enum and tab indices
- `detail_sub_view_per_project: HashMap<String, DetailSubView>` tracks per-project tab state

### Established Patterns
- Each tab renders in a match arm inside `DetailScreen::render()`
- Backlog tab uses `ListState` with `cache.backlog_selected` for selection tracking
- GitHistory tab has scrollable content with `cache.git_selected`
- Queue tab has multi-level list (queue items → details)
- Tab navigation via left/right arrows, wrapping at boundaries

### Integration Points
- `src/app.rs:DetailSubView` — add `Archive` variant
- `src/ui/screens/detail.rs:tab_index()` — add mapping for index 7
- `src/ui/screens/detail.rs:sub_view_from_index()` — add mapping for index 7
- `src/ui/screens/detail.rs` tab bar rendering — add "Archive" label
- `src/ui/screens/mod.rs:AppContext` — add archive cache field
- `src/app.rs:App` — add archive state initialization

</code_context>

<specifics>
## Specific Ideas

- Tab bar overflow at 80 columns is a known concern (from STATE.md blockers). Adding 8th tab may need abbreviated tab labels or scrolling tabs. Resolve during implementation or note as deferred.
- Archived milestone directory structure: `v{version}-phases/{NN}-{slug}/` contains artifacts like `{NN}-{padded}-PLAN.md`, `{NN}-{padded}-SUMMARY.md`, etc.

</specifics>

<deferred>
## Deferred Ideas

- Styled markdown in existing tabs (Backlog, etc.) — deferred per REQUIREMENTS.md (MKDN-01 is future)
- Search within archive files
- File diffing between milestones

</deferred>

---

*Phase: 12-milestone-archive-browser*
*Context gathered: 2026-03-31*
