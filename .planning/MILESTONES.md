# Milestones

## v1.6.0 GSD 1.8.0 Catch-up (Shipped: 2026-07-22)

**Phases completed:** 0 phases, 1 quick task (10 plans, 5 waves) + 2 fast tasks

**Key accomplishments:**

- README rewritten around the value proposition, with the distinction from the new claude-orchestration backend and an explicit GSD 1.8.0 compatibility statement
- `disk_status`: matched-summary plan counting (FIX-/GAPCLOSURE-/PLAN-REVIEW summaries excluded), superseded-plan handling, flexible phase-directory tokens, and COVERAGE / WINDOWS / deferred-items / SKELETON artifact detection
- `roadmap_md`: `###` and project-code-prefixed headings, `<details>`-wrapped phases, strikethrough-retired and Phase 0 / 999.x sentinels excluded, and the `## Progress` table adopted as the authoritative count source
- `config.json` schema extended with the `claude_orchestration`, `statusline`, `dynamic_routing`, `review`, `external_job`, and `capabilities` blocks, 12 workflow gates, and `graphify.graph_path`
- Staleness derived from git commit times with an mtime fallback
- ADR-2207 status vocabulary, `current_phase` / `current_plan` frontmatter, and `external_job_waiting` for async jobs
- Smart-entry next-command integration; queue relocated to `.planning/meta-manager/QUEUE.md` with legacy migration (gsd-health W019 clean)
- Workstreams data layer reading `.planning/workstreams/`
- UI: `waves.json` in the Pipe tab, new Cfg categories, and workstream / external-job / artifact badges
- Release hygiene: version 1.6.0, Cargo.lock refresh, `Action` enum boxed for clippy `large_enum_variant`; direct dep bumps (notify-debouncer-full 0.7, serde_yml 0.0.13); GitHub Actions release workflow publishing to crates.io on version tags

---

## v1.5.0 Sub-phase Artifact Detection (Shipped: 2026-05-15)

**Phases completed:** 0 phases, 2 quick tasks

**Key accomplishments:**

- Pipeline drill-down surfaces UAT, SPEC, and EVAL-REVIEW sub-phase artifacts alongside PLAN / EXECUTE / VERIFY
- crates.io publish metadata in Cargo.toml; README gained a `cargo install gsd-meta-manager` path
- Adopted the semver `vX.Y.Z` tag convention from this release onward (v1.0–v1.4 used the older `vX.Y` form)
- `RUST_LOG` honored via `EnvFilter` on the tracing subscriber
- Canonical project documentation regenerated; Release Process section added to CLAUDE.md

---

## v1.4 Live Sessions & Document Browsing (Shipped: 2026-05-12)

**Phases completed:** 0 phases, 4 quick tasks

**Key accomplishments:**

- Auto-register GSD projects discovered from running `claude` sessions (startup scan + 5s poll)
- 10th detail tab (`0:Docs`) — drill-down markdown browser of `.planning/` rooted at the active phase, with `g`/`p` quick-jumps
- SECURITY.md sub-phase artifact detected and rendered as a Security row in Plan sub-stages
- Suppressed the "Updated" status when project state is unchanged, ending no-op refresh notification spam
- Release hygiene: Cargo.toml version 0.1.0 → 1.4.0, ending the drift where v1.0–v1.3 all reported 0.1.0; `--version` wired through clap

---

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
