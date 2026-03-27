# Phase 06: Read-Only Views - Context

**Gathered:** 2026-03-26
**Status:** Ready for planning

<domain>
## Phase Boundary

Add two read-only browsing views to the detail screen: a backlog browser for 999.* items and a scrollable git history viewer. Both integrate as tabs in the existing detail view using the Screen trait architecture from Phase 05. No write/edit operations — read-only browsing and queue integration only.

</domain>

<decisions>
## Implementation Decisions

### Backlog Browser
- **D-01:** Backlog browser lives as a tab/section in the detail view, consistent with how phases are already shown
- **D-02:** Backlog items displayed as a scrollable list showing `999.N-slug` with one-line description parsed from first markdown heading
- **D-03:** Backlog item content shown in an inline expandable panel below the list (split pane style, no screen switch)
- **D-04:** "Queue promotion" pre-fills the enqueue screen with `/gsd:review-backlog` command for the selected item

### Git History Viewer
- **D-05:** Git history lives as a tab/section in the detail view, same pattern as backlog
- **D-06:** One-line commit format: `hash (7) — date — message` with author dimmed
- **D-07:** Keybind toggle (e.g., `p`) switches between full-repo and .planning/-only history, with indicator in header
- **D-08:** Selecting a commit shows inline diff stat summary (files changed, +/- lines) below the log

### Navigation & Integration
- **D-09:** Tab switching via number keys `1`/`2`/`3` or left/right arrows, with a tab bar header showing current view
- **D-10:** Git data accessed via shell-out to `git log`/`git diff` using `tokio::process::Command` — no git2 crate dependency
- **D-11:** Backlog item content read from `.planning/phases/999.N-*/` directories, parsing first `.md` file found
- **D-12:** Async loading shows dim "Loading..." text in the view area while data fetches

### Claude's Discretion
- Exact tab bar styling and layout proportions
- Split pane sizing for backlog content preview
- Git log page size and scrolling behavior
- Diff stat formatting details

</decisions>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/ui/screens/detail.rs` — DetailScreen with phase list rendering, Screen trait impl
- `src/ui/screens/mod.rs` — Screen trait, ScreenAction, AppContext
- `src/state_reader/mod.rs` — `count_backlog_items()`, `ProjectState.backlog_count`
- `src/ui/screens/enqueue.rs` — EnqueueScreen for queue integration

### Established Patterns
- Screen trait architecture: `handle_event()`, `render()`, lifecycle methods
- Async state loading via `spawn_blocking` + `ProjectStateLoaded` action
- ratatui widgets: Block, Paragraph, List, Table for rendering
- Vim-style navigation (j/k, arrows) consistent across screens

### Integration Points
- DetailScreen needs tab support (currently single-view)
- EnqueueScreen for pre-filling promotion commands
- `ProjectState` struct for backlog data expansion
- `tokio::process::Command` for git subprocess calls

</code_context>

<specifics>
## Specific Ideas

No specific requirements beyond the decisions above — standard ratatui patterns apply.

</specifics>

<deferred>
## Deferred Ideas

- Backlog item editing (BLOG-05) — write operations are v2+
- Direct backlog promotion to phase (BLOG-04) — needs more UX design
- Full diff view screen — inline stats sufficient for read-only

</deferred>

---

*Phase: 06-read-only-views*
*Context gathered: 2026-03-26 via Smart Discuss (autonomous mode)*
