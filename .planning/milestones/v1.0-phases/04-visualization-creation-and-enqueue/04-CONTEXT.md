# Phase 4: Visualization, Creation, and Enqueue - Context

**Gathered:** 2026-03-25
**Status:** Ready for planning

<domain>
## Phase Boundary

Add ASCII roadmap visualization in the detail view, project creation from the TUI with hook support, and work enqueue with GSD command suggestions stored in each project's `.planning/QUEUE.md`.

</domain>

<decisions>
## Implementation Decisions

### ASCII Roadmap Visualization
- **D-01:** Roadmap visualization appears inside the detail view — press `r` to toggle between phase list and roadmap view
- **D-02:** Vertical pipeline style: phases as boxes connected by `│ ▼` arrows, with progress fill and status icon per box
- **D-03:** Each phase box shows: phase number, name (truncated), status icon, plan count — 3-line box format
- **D-04:** Current phase highlighted with bold/bright border + `▶` marker

### Project Creation Flow
- **D-05:** Press `c` from dashboard — modal input: name → path → confirm. Creates directory, `git init`, auto-registers in manager. Does NOT initialize GSD — user runs `/gsd:new-project` themselves in the new directory
- **D-06:** Path input supports `~` expansion (to `$HOME`), relative-to-pwd resolution, and tab autocompletion for directory paths
- **D-07:** Hook script support — configurable pre/post-create hooks in `~/.config/gsd-manager/config.json` with params: `name`, `path`, `alias`. Hooks are shell commands executed with environment variables `GSD_PROJECT_NAME`, `GSD_PROJECT_PATH`, `GSD_PROJECT_ALIAS`
- **D-08:** Newly created project appears in dashboard immediately after creation (auto-registered + file watcher starts)

### Work Enqueue
- **D-09:** Press `e` in detail view — free-form text input for commands. Suggestions/autocomplete for GSD commands based on the project's current state and next logical step (e.g., if project is at "Ready to plan" → suggest `/gsd:discuss-phase N`, `/gsd:plan-phase N`)
- **D-10:** Enqueued actions stored in each project's `.planning/QUEUE.md` — one action per line. GSD can check this file when it thinks it's done to pick up the next queued action
- **D-11:** Enqueued actions visible in the detail view as a "Queued" section below the phase list
- **D-12:** Clipboard copy feature dropped — not needed

### Claude's Discretion
- Exact box dimensions and ASCII art style for roadmap visualization
- Tab completion implementation (filesystem walk vs shell integration)
- QUEUE.md format (simple markdown list vs structured format)
- How to determine "next logical step" for command suggestions
- Whether to show confirmation dialog before `git init`

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project context
- `.planning/PROJECT.md` — Core value, constraints, key decisions
- `.planning/REQUIREMENTS.md` — DASH-04, CREATE-01, CREATE-02, CREATE-03, ENQ-01, ENQ-02, ENQ-03
- `.planning/ROADMAP.md` — Phase 4 success criteria and dependency chain

### Prior phase artifacts
- `.planning/phases/01-core-infrastructure/01-CONTEXT.md` — Registry/config design (D-01 through D-04)
- `.planning/phases/02-dashboard-and-navigation/02-CONTEXT.md` — Dashboard layout, key bindings, search/filter
- `.planning/phases/03-live-state-and-detail-view/03-CONTEXT.md` — Detail view design (D-01 through D-08), file watcher

### Existing implementation
- `src/app.rs` — App struct, InputMode enum, key handling (extend for `c`/`e`/`r` keys)
- `src/ui/detail_view.rs` — Detail view render (extend with roadmap viz tab and queue section)
- `src/config.rs` — Config struct and persistence (extend for hooks config)
- `src/registry.rs` — add_project/remove_project (reuse for auto-registration after create)
- `src/watcher.rs` — FileWatcher (start watching new project after creation)
- `src/state_reader/mod.rs` — ProjectState and parse_project_state (used for command suggestions)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `registry::add_project()` — validates `.planning/` exists, adds to config — reuse after project creation (skip validation since we just created the dir)
- `FileWatcher` — can add new paths dynamically via notify's `Watcher::watch()` — start watching newly created project
- `detail_view.rs` — already renders phase list with scroll — extend with roadmap viz toggle and queue section
- `InputMode::DetailView { alias }` — extend with sub-mode (PhaseList vs RoadmapViz)
- `classify_status()` and `status_color()` — reuse for roadmap viz phase box coloring
- `ProjectState.phases` — Vec<RoadmapPhase> with name, completed, plan counts — all data needed for roadmap viz

### Established Patterns
- TEA pattern: Action → App::update() → render
- Modal input (AddAlias, AddPath, DeleteConfirm) — same pattern for project creation flow
- Footer-based input with error messages — reuse for path input with autocomplete
- Detail view key handling (handle_detail_key) — extend for `r` toggle and `e` enqueue

### Integration Points
- Dashboard normal mode: add `c` keybinding for create
- Detail view: add `r` for roadmap toggle, `e` for enqueue
- Config struct: add `hooks` field for pre/post-create hooks
- help_overlay.rs: update keybinding reference with new keys

</code_context>

<specifics>
## Specific Ideas

- QUEUE.md should be simple enough for users to edit manually — one command per line, `#` for comments
- Hook scripts receive params via environment variables, not positional args — more robust
- Path autocomplete can start simple (just `~` expansion + `..` navigation) — full filesystem walk is a stretch goal
- Roadmap viz toggle should remember which view was active per project within the session

</specifics>

<deferred>
## Deferred Ideas

None — discussion stayed within phase scope

</deferred>

---

*Phase: 04-visualization-creation-and-enqueue*
*Context gathered: 2026-03-25*
