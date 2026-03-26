# Phase 1: Core Infrastructure - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-03-24
**Phase:** 01-core-infrastructure
**Areas discussed:** Registry & config, State parser scope, TUI skeleton, GSD hook integration

---

## Registry & Config

### Config format

| Option | Description | Selected |
|--------|-------------|----------|
| TOML | Idiomatic Rust, human-readable, good for nested config. What Cargo uses. | |
| JSON | Consistent with GSD's own config.json. Easier cross-tool parsing. | ✓ |
| You decide | Claude picks based on what works best | |

**User's choice:** JSON
**Notes:** Consistency with GSD ecosystem was the deciding factor.

### Config location

| Option | Description | Selected |
|--------|-------------|----------|
| ~/.config/gsd-manager/ | XDG-compliant, standard for Linux CLI tools | ✓ |
| ~/.gsd/manager/ | Co-located with GSD's own ~/.gsd/ directory | |
| You decide | Claude picks the most sensible location | |

**User's choice:** ~/.config/gsd-manager/
**Notes:** None

### Registry format

| Option | Description | Selected |
|--------|-------------|----------|
| Path-keyed array | Simple array of {path, name, added_at}. Name derived from directory or PROJECT.md. | |
| Named entries | User assigns an alias when registering. Alias used in TUI. | ✓ |
| You decide | Claude picks | |

**User's choice:** Named entries
**Notes:** None

---

## State Parser Scope

### Parse depth

| Option | Description | Selected |
|--------|-------------|----------|
| Essential only | STATE.md, ROADMAP.md, config.json. Enough for dashboard. | ✓ |
| Full extraction | Also PLAN.md task counts, REQUIREMENTS.md completion %, phase dirs for backlog. | |
| You decide | Claude picks the right depth for Phase 1 vs later phases | |

**User's choice:** Essential only
**Notes:** None

### Error handling

| Option | Description | Selected |
|--------|-------------|----------|
| Graceful degrade | Show project with 'unknown' status, log warning. Never crash, never remove. | ✓ |
| Strict validation | Mark as 'error' state, offer to re-validate or remove. | |
| You decide | Claude picks | |

**User's choice:** Graceful degrade
**Notes:** None

---

## TUI Skeleton

### Phase 1 TUI scope

| Option | Description | Selected |
|--------|-------------|----------|
| Minimal proof | Just project list with names and raw status text. No styling. | |
| Functional stub | Basic project list + add/remove via keyboard. Usable but ugly. | ✓ |
| You decide | Claude picks the minimum that proves the foundation | |

**User's choice:** Functional stub
**Notes:** None

### CLI vs TUI

| Option | Description | Selected |
|--------|-------------|----------|
| Both CLI + TUI | gsd-manager add /path — works headless. Also in TUI. | ✓ |
| TUI only | All interaction inside TUI. Simpler but can't script. | |

**User's choice:** Both CLI + TUI
**Notes:** None

---

## GSD Hook Integration

### Hook model

| Option | Description | Selected |
|--------|-------------|----------|
| File watcher primary | inotify watches .planning/ dirs. Hooks are nice-to-have. | |
| Hybrid design | Event channel accepts both file-change and push events. | ✓ |
| You decide | Claude picks the most flexible architecture | |

**User's choice:** Hybrid design
**Notes:** None

### Hook scope in Phase 1

| Option | Description | Selected |
|--------|-------------|----------|
| None in Phase 1 | Pure file-based. Hook research in Phase 3. | |
| Design for it | Design state update channel now, no actual hook code. | ✓ |

**User's choice:** Design for it
**Notes:** Channel architecture ready for hooks, but no hook code ships in Phase 1.

---

## Claude's Discretion

- ProjectState struct field names and types
- Event loop tick rate and render strategy
- CLI argument parser choice
- Internal error types and logging setup
- Test strategy

## Deferred Ideas

- Hook research and implementation — Phase 3
- PLAN.md task count parsing — later phases
- Color-coding and styling — Phase 2
- Search/filter — Phase 2
