# Milestones

## v1.3 Configuration & Pipeline Visibility (Shipped: 2026-05-09)

**Phases completed:** 0 phases, 10 quick tasks

**Key accomplishments:**

- Defaults tab — 9th detail-view tab that reads, displays, and edits `.planning/config.json` with categorized rendering, dropdown picker for booleans and enums, inline text-input for string fields (e.g. `base_branch`), `x`-to-clear shortcut for unsetting Optional fields, and `r`-to-reload
- Defaults layering — `~/.gsd/defaults.json` is parsed and surfaced as a fallback for any project row that's unset; inherited values render with a magenta `*` marker. `d` toggles the tab between editing the project config and the global defaults file (auto-creates `~/.gsd/` on first save)
- Six-section Defaults layout matching GSD's `/gsd-settings` (Planning / Execution / Docs & Output / Features / Model & Pipeline / Misc); `GsdConfig` extended with `pattern_mapper`, `ai_integration_phase`, `tdd_mode`, `code_review`, `code_review_depth`, `ui_review`, `intel.enabled`, `graphify.enabled`, `graphify.build_timeout`
- Pipeline sub-stage drill-down — `DiskInference` learned eight new artifact flags (`PATTERNS.md`, `PLAN-CHECK.md`, `VALIDATION.md`, `UI-SPEC.md`, `UI-CHECK.md`, `AI-SPEC.md`, `REVIEW.md`, `UI-REVIEW.md`); the Pipeline tab renders indented "Plan sub-stages" / "Execute sub-stages" blocks with ✓/○ markers
- Tmux Tab-to-switch — `ClaudeSession` gained a `tty` field; new `terminal_switch` module mirrors claudectl's tmux flow; Tab on the project list (overview) and on the Sessions tab jumps focus to the matching tmux pane
- Polish — `tui-textarea` + `$EDITOR` shell-out for archive/backlog markdown editing, PageUp/PageDown scrolling on the detail screen, fixed folder-empty regression after returning from markdown view, GitHub-ready README

---

## v1.2 Housekeeping & Archive Browser (Shipped: 2026-04-01)

**Phases completed:** 5 phases, 6 plans, 10 tasks

**Key accomplishments:**

- Zero-warning build with all dead_code suppressions resolved, clippy clean, and main.rs refactored to use lib crate
- HANDOFF file detection with cyan pause badge on dashboard and context display in detail view
- Archive data types, filesystem discovery, milestone loading, and markdown renderer in src/archive.rs with action variants for async delivery
- 8-tab detail view with Archive drill-down browser: milestone list, phase list, file list, and styled markdown viewer with breadcrumb navigation
- Queue execution design document covering GSD autonomous lifecycle, two integration strategies (per-item isolation recommended for v1.3), LLM-agnostic Executor interface, and safety requirements with timeouts/retries/escalation

---

## v1.1 Polish & Power Features (Shipped: 2026-03-27)

**Phases completed:** 5 phases, 16 plans, 25 tasks

**Key accomplishments:**

- Fixed plan counting regex for standalone PLAN.md, milestone completion display, and path-first CLI registration with auto-derived alias
- Replaced 11-variant InputMode enum with Screen trait + screen stack, migrated file I/O to async spawn_blocking
- GSD disk_status algorithm scanning phase directories for ground-truth status, with compact D-R-P-E-V pipeline in dashboard and expanded status on focus
- Wired D-R-P-E-V pipeline and disk-status brackets from orphaned files into active Screen trait architecture, deleted dead code
- Removed inaccurate expanded status text from dashboard and added bracket notation legend to detail view
- 4-tab detail view with BacklogItem/GitLogEntry data models, ProjectViewCache, and async loading infrastructure
- Backlog browser tab with ratatui List widget navigation, split-pane markdown content preview, and /gsd:review-backlog queue promotion
- Scrollable git log tab with ratatui List widget, planning-only toggle via 'p', and inline 60/40 diff stat pane on Enter
- Backlog tab split-pane content preview and /gsd:review-backlog queue promotion pre-fill
- Extended DiskInference with per-stage booleans, added gsd_integration config toggle, Pipeline detail tab variant, and pipeline_selected cache field
- Pipeline tab with split-pane phase list and horizontal [D]---[R]---[P]---[E x/y]---[V] stage visualization using DiskInference-derived color coding
- Verified/inferred badge spans on phase status lines gated by gsd_integration config toggle
- Full CRUD queue operations via keyboard: add, delete with confirmation, mark done, reorder with Shift+J/K, and edit with pre-fill
- Detect active Claude Code sessions via Linux /proc filesystem and display green play indicator on dashboard rows
- Sessions tab with per-project list view, terminal-based resume and launch actions using $TERMINAL fallback chain

---

## v1.0 MVP (Shipped: 2026-03-26)

**Phases completed:** 15 phases, 10 plans, 21 tasks

**Key accomplishments:**

- Rust project scaffold with clap CLI (add/remove/list), atomic JSON config persistence via tempfile, and .planning/-validated project registry with 12 passing integration tests
- Async TUI dashboard with TEA state machine, crossterm event bus, project table with state reader integration, and keyboard-driven add/remove flows
- Color-coded 5-column project table with aggregate status footer, adaptive layout, search infrastructure, and terminal resize handling
- Help overlay popup with keybinding reference and filter syntax, completing all Phase 2 navigation UX requirements
- Live auto-refresh via notify-debouncer-full 0.5 watching .planning/ directories with 200ms debounce and 500ms dedup
- Full-screen project detail view with phase breakdown, per-phase plan counts, status icons, and in-memory change tracking banner
- ASCII roadmap pipeline widget with box-drawing chars, scroll support, current-phase highlighting, and r-key toggle in detail view
- TUI project creation flow with modal input, git init via spawn_blocking, pre/post-create hooks, tilde expansion, and tab completion
- QUEUE.md parser/writer with atomic persistence, Tab-cyclable GSD command suggestions, and queued actions display in detail view

---
