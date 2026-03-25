# Requirements: GSD Manager

**Defined:** 2026-03-24
**Core Value:** See the state of every GSD project at a glance and act on any of them without leaving the TUI.

## v1 Requirements

Requirements for initial release. Each maps to roadmap phases.

### Registration

- [x] **REG-01**: User can add a GSD project by path (validates `.planning/` exists before registering)
- [x] **REG-02**: User can remove a tracked project from the manager
- [x] **REG-03**: Project registry persists across restarts (config file on disk)

### Dashboard

- [ ] **DASH-01**: User sees a scrollable project list with name, current phase, and status per row
- [ ] **DASH-02**: Project rows are color-coded by workflow state (idle, active, blocked, complete)
- [ ] **DASH-03**: A persistent status bar shows aggregate counts across all projects (e.g., "5 projects: 2 active, 1 blocked, 2 idle")
- [ ] **DASH-04**: User sees an ASCII roadmap visualization showing phase structure and progress for a selected project
- [ ] **DASH-05**: User sees a change summary showing what changed in a project since last visit (e.g., "Phase 3 completed 2h ago")

### Navigation & UX

- [ ] **NAV-01**: User navigates with vim-style keys (j/k or arrows to move, Enter to select, q/Esc to go back)
- [ ] **NAV-02**: User presses `?` to see a help overlay with all keybindings for the current view
- [ ] **NAV-03**: User presses `/` to filter the project list by name or status
- [ ] **NAV-04**: TUI layout adapts to terminal size without crashing or corrupting output
- [ ] **NAV-05**: TUI exits cleanly on quit (q, Ctrl+C) — terminal state fully restored, including on panic

### State Reading

- [ ] **STATE-01**: Manager reads project state from `.planning/` files (STATE.md, ROADMAP.md, config.json) without running GSD commands
- [ ] **STATE-02**: Manager shows phase progress indicators (completed vs total tasks from PLAN.md files)
- [ ] **STATE-03**: Manager shows backlog item count per project (999.x directories in `.planning/`)
- [ ] **STATE-04**: Manager auto-refreshes when `.planning/` files change via file system watcher (inotify/kqueue)
- [ ] **STATE-05**: Research whether GSD hooks can push state updates to the manager instead of polling

### Detail View

- [ ] **DET-01**: User can drill into a project to see: project path, all roadmap phases, current phase, and task completion counts
- [ ] **DET-02**: Detail view shows phase-level breakdown with status per phase (pending, in-progress, complete)

### Project Creation

- [ ] **CREATE-01**: User can create a new GSD project from the TUI (specify name, directory path)
- [ ] **CREATE-02**: New project creation initializes directory, git repo, and imports global GSD settings
- [ ] **CREATE-03**: Newly created project is automatically registered in the manager

### Work Enqueue

- [ ] **ENQ-01**: User can enqueue a next action for a project (next phase, task, or verification command)
- [ ] **ENQ-02**: Enqueued actions are visible in the project detail view
- [ ] **ENQ-03**: V1 enqueue mechanism copies the GSD command to clipboard (native queue integration deferred)

## v2 Requirements

Deferred to future release. Tracked but not in current roadmap.

### Claude Session Integration

- **SESSION-01**: User can hook into or launch a Claude terminal session for a project from the TUI
- **SESSION-02**: Manager shows live output from running GSD sessions

### Plugin System

- **PLUG-01**: User can extend the manager with custom plugins
- **PLUG-02**: Plugin API provides access to project state and TUI rendering

### Remote Projects

- **REMOTE-01**: User can register and monitor projects on remote machines via SSH

### Native Queue Integration

- **ENQ-04**: Enqueued actions are written to a GSD-native queue file that GSD agents pick up automatically

## Out of Scope

Explicitly excluded. Documented to prevent scope creep.

| Feature | Reason |
|---------|--------|
| GUI / web companion | Target audience lives in the terminal; TUI is the product |
| Animated transitions / spinners | TUI anti-pattern — adds rendering complexity for zero productivity gain |
| Global keybinding daemon / system tray | Platform-specific, out of TUI scope |
| Real-time streaming log view | GSD sessions run in separate processes; no stdout to tail — file watching is the right model |
| Mouse-first interaction | Target users are keyboard-driven developers |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| REG-01 | Phase 1 | Complete |
| REG-02 | Phase 1 | Complete |
| REG-03 | Phase 1 | Complete |
| STATE-01 | Phase 1 | Pending |
| STATE-02 | Phase 1 | Pending |
| STATE-03 | Phase 1 | Pending |
| DASH-01 | Phase 2 | Pending |
| DASH-02 | Phase 2 | Pending |
| DASH-03 | Phase 2 | Pending |
| NAV-01 | Phase 2 | Pending |
| NAV-02 | Phase 2 | Pending |
| NAV-03 | Phase 2 | Pending |
| NAV-04 | Phase 2 | Pending |
| NAV-05 | Phase 2 | Pending |
| STATE-04 | Phase 3 | Pending |
| STATE-05 | Phase 3 | Pending |
| DET-01 | Phase 3 | Pending |
| DET-02 | Phase 3 | Pending |
| DASH-05 | Phase 3 | Pending |
| DASH-04 | Phase 4 | Pending |
| CREATE-01 | Phase 4 | Pending |
| CREATE-02 | Phase 4 | Pending |
| CREATE-03 | Phase 4 | Pending |
| ENQ-01 | Phase 4 | Pending |
| ENQ-02 | Phase 4 | Pending |
| ENQ-03 | Phase 4 | Pending |

**Coverage:**
- v1 requirements: 26 total
- Mapped to phases: 26
- Unmapped: 0

---
*Requirements defined: 2026-03-24*
*Last updated: 2026-03-24 after roadmap creation*
