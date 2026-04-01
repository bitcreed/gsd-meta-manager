# Phase 11: Paused Project Detection - Context

**Gathered:** 2026-03-31
**Status:** Ready for planning

<domain>
## Phase Boundary

Add pause detection and badge display so users can tell at a glance which GSD projects are paused (have HANDOFF.md or HANDOFF.json in .planning/) and see pause context without opening files.

</domain>

<decisions>
## Implementation Decisions

### Badge Visual Design & Dashboard Placement
- Use `⏸` emoji as the pause badge icon — compact, universally understood
- Badge color: Cyan — distinct from existing Green (active), Yellow (idle), Red (blocked), Magenta (unknown)
- Badge placement: After the status column, same position as session indicator
- Detail view: Show "Paused: {next_action}" from HANDOFF.json in the status area

### Detection Logic & Priority Rules
- Detect both `HANDOFF.md` and `HANDOFF.json` — check existence AND non-empty content (per existing HANDOFF.json stale badge decision from v1.2 planning)
- Priority: Pause badge wins over session-active indicator when both conditions are true (per success criteria #2)
- Code location: Add `paused: bool` + `pause_context: Option<String>` fields to `ProjectState` in `state_reader/mod.rs`, detect in `parse_project_state()`
- File watcher: Reuse existing `notify` watcher on `.planning/` — HANDOFF files get picked up during normal state re-parse cycle, no special watch filter needed

### Claude's Discretion
- How to extract `next_action` from HANDOFF.json (parse JSON field) vs HANDOFF.md (regex or first heading)
- Whether to truncate long pause context strings for dashboard display
- Exact integration with the compact pipeline display in unfocused rows

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/session_detector.rs` — ClaudeSession detection pattern (similar file-based detection)
- `src/state_reader/mod.rs:ProjectState` — struct to extend with `paused` and `pause_context` fields
- `src/ui/screens/normal.rs:compact_pipeline()` — dashboard row rendering to integrate badge into
- `src/ui/screens/detail.rs:disk_suffix_spans()` — badge rendering pattern (verified/inferred badges)

### Established Patterns
- Dashboard status rendering uses `classify_status()` → `status_color()` for color mapping
- Detail view uses `show_badges` flag gated by `config.preferences.gsd_integration`
- State reader parses `.planning/` files in `parse_project_state()` and returns a flat struct
- File watcher triggers full state re-parse via existing notify-debouncer-full setup

### Integration Points
- `src/state_reader/mod.rs:parse_project_state()` — add HANDOFF file detection here
- `src/ui/screens/normal.rs` — dashboard row rendering, add pause badge display
- `src/ui/screens/detail.rs` — detail view status area, show pause context
- `src/app.rs` — may need to pass pause state through AppContext

</code_context>

<specifics>
## Specific Ideas

No specific requirements — open to standard approaches following existing badge/detection patterns.

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 11-paused-project-detection*
*Context gathered: 2026-03-31*
