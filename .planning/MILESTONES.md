# Milestones

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
