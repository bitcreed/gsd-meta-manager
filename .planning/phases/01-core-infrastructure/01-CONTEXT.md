# Phase 1: Core Infrastructure - Context

**Gathered:** 2026-03-24
**Status:** Ready for planning

<domain>
## Phase Boundary

Build the async TUI foundation (event loop, terminal lifecycle, panic recovery), a state reader that parses `.planning/` files into typed structs, and a project registry (add/remove/persist). This phase delivers a functional stub TUI and CLI subcommands — Phase 2 builds the real dashboard on top.

</domain>

<decisions>
## Implementation Decisions

### Registry & Config
- **D-01:** Config format is JSON — consistent with GSD's own config.json, easier cross-tool parsing
- **D-02:** Config location is `~/.config/gsd-manager/` — XDG-compliant, standard for Linux CLI tools
- **D-03:** Projects are stored as named entries — user assigns an alias when registering (e.g., "myapp" → /home/user/projects/myapp). Alias is displayed in TUI
- **D-04:** Single config file: `~/.config/gsd-manager/config.json` containing both registry and preferences

### State Parser
- **D-05:** Parse essential fields only in Phase 1 — STATE.md (current phase, status), ROADMAP.md (phase list), config.json (mode). Enough for dashboard. Deeper extraction (PLAN.md task counts, REQUIREMENTS.md completion) added in later phases
- **D-06:** Graceful degradation on missing/malformed files — show project with "unknown" status, log warning. Never crash, never auto-remove from registry

### TUI Skeleton
- **D-07:** Phase 1 delivers a functional stub — basic project list with add/remove via keyboard. Usable but unstyled. Phase 2 polishes it
- **D-08:** Both CLI and TUI interfaces — `gsd-manager add <alias> /path/to/project` works headless (scriptable). Same operations available inside the TUI

### GSD Hook Integration
- **D-09:** Hybrid state update design — the state reader uses an event channel (tokio mpsc) that can accept both file-change events and push events. Phase 1 only implements file-based reads, but the channel architecture is ready for hooks
- **D-10:** Design the state update channel now, but no actual hook code in Phase 1. Hook research and implementation happens in Phase 3 alongside the file watcher

### Claude's Discretion
- Exact ProjectState struct field names and types
- Event loop tick rate and render strategy
- CLI argument parser choice (clap vs manual)
- Internal error types and logging setup
- Test strategy (unit tests for parser, integration tests for registry persistence)

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Project context
- `.planning/PROJECT.md` — Core value, constraints, key decisions
- `.planning/REQUIREMENTS.md` — REG-01 through REG-03, STATE-01 through STATE-03
- `.planning/ROADMAP.md` — Phase 1 success criteria and dependency chain

### Research findings
- `.planning/research/STACK.md` — Rust + ratatui 0.30 + crossterm 0.29 + tokio 1.50 stack with rationale
- `.planning/research/ARCHITECTURE.md` — TEA pattern, component structure, EventBus design, build order
- `.planning/research/PITFALLS.md` — Terminal cleanup, blocking I/O, render loop, partial-write races
- `.planning/research/SUMMARY.md` — Synthesized recommendations

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- None — greenfield project, no existing code

### Established Patterns
- None yet — Phase 1 establishes the patterns all later phases follow

### Integration Points
- GSD `.planning/` file format: STATE.md (markdown), ROADMAP.md (markdown), config.json (JSON)
- GSD hook system: shell commands triggered by Claude Code events — potential push mechanism for state updates

</code_context>

<specifics>
## Specific Ideas

- CLI subcommands should feel like standard Rust CLI tools (cargo-like UX)
- The functional stub should be enough to demo the full add → view → remove loop, even if ugly
- Named aliases for projects (not just paths) — this is a user-facing identifier throughout the TUI

</specifics>

<deferred>
## Deferred Ideas

- Hook research and implementation — Phase 3
- PLAN.md task count parsing — later phases (STATE-02 deferred deeper extraction)
- Color-coding and styling — Phase 2
- Search/filter — Phase 2

</deferred>

---

*Phase: 01-core-infrastructure*
*Context gathered: 2026-03-24*
