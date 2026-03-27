# Requirements: GSD Meta Manager

**Defined:** 2026-03-26
**Core Value:** See the state of every GSD project at a glance and act on any of them without leaving the TUI.

## v1.1 Requirements

Requirements for the v1.1 milestone. Each maps to roadmap phases.

### State Reader

- [x] **STATE-01**: User sees accurate plan counts per phase on dashboard (fix regex for standalone PLAN.md)
- [x] **STATE-02**: User sees "Complete" instead of "P5: Unknown" when all phases are done
- [x] **STATE-03**: User sees phase status inferred from disk files (discuss/research/plan/execute/verify stages)

### CLI Ergonomics

- [x] **CLI-01**: User can register a project by path only — name auto-derived from last folder component

### GSD Integration

- [x] **GSD-01**: User can opt into enriching state with cached gsd-tools.cjs JSON output
- [x] **GSD-02**: User sees [verified] vs [inferred] badges on status fields

### Backlog

- [x] **BLOG-01**: User can browse backlog items (999.*) in a scrollable list within detail view
- [x] **BLOG-02**: User can view backlog item details (markdown content)
- [x] **BLOG-03**: User can queue a promotion command for a backlog item

### Git History

- [x] **GIT-01**: User can view scrollable git log for a project
- [x] **GIT-02**: User can toggle between full repo and .planning/-scoped history
- [x] **GIT-03**: User can view commit diff stats by selecting a commit

### Execution Flow Graph

- [x] **FLOW-01**: User sees per-phase pipeline visualization (discuss/research/plan/execute/verify)
- [x] **FLOW-02**: User sees color-coded status per stage (not started, current, complete, skipped)
- [x] **FLOW-03**: User sees plan execution progress as fraction in the execute stage

### Queue Execution

- [ ] **QUEUE-01**: User can execute a queued item with confirmation dialog
- [ ] **QUEUE-02**: User sees queue item status (pending/in-progress/done/failed)
- [ ] **QUEUE-03**: User can manage queue items (add/remove/reorder)

### Claude Sessions

- [ ] **SESS-01**: User sees which projects have active Claude sessions on the dashboard
- [ ] **SESS-02**: User can browse session list with last activity and status in detail view
- [ ] **SESS-03**: User can resume or launch a Claude session from the TUI

## Future Requirements

Deferred to v1.2+. Tracked but not in current roadmap.

### Queue Enhancements

- **QUEUE-04**: User can edit queue item text inline
- **QUEUE-05**: User can batch-execute multiple queue items

### Backlog Enhancements

- **BLOG-04**: User can promote backlog item directly to a new phase (creates phase directory)
- **BLOG-05**: User can edit backlog item description from TUI

### Session Enhancements

- **SESS-04**: User sees real-time session activity indicators (streaming status)
- **SESS-05**: User can attach to a running session's output stream

## Out of Scope

| Feature | Reason |
|---------|--------|
| Embedded Claude terminal inside TUI | Terminal-in-terminal UX disaster: keypress conflicts, rendering glitches, impossible scrollback |
| Real-time streaming of Claude output | Massive complexity for marginal value; show metadata instead |
| Full text editor for QUEUE.md | Users have preferred editors; support structured operations instead |
| Auto-executing queued commands | Dangerous: stale/context-dependent/destructive items need confirmation |
| Git commit/push/branch from TUI | Scope creep into gitui/lazygit territory; read-only in v1.1 |
| Plugin system / extensibility | Architecture still stabilizing; defer to v2+ |
| Backlog promotion (direct phase creation) | Needs deeper GSD integration research; defer to v1.2 |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| STATE-01 | Phase 05 | Complete |
| STATE-02 | Phase 05 | Complete |
| STATE-03 | Phase 05 | Complete |
| GSD-01 | Phase 07 | Complete |
| GSD-02 | Phase 07 | Complete |
| BLOG-01 | Phase 06 | Complete |
| BLOG-02 | Phase 06 | Complete |
| BLOG-03 | Phase 06 | Complete |
| GIT-01 | Phase 06 | Complete |
| GIT-02 | Phase 06 | Complete |
| GIT-03 | Phase 06 | Complete |
| FLOW-01 | Phase 07 | Complete |
| FLOW-02 | Phase 07 | Complete |
| FLOW-03 | Phase 07 | Complete |
| QUEUE-01 | Phase 08 | Pending |
| QUEUE-02 | Phase 08 | Pending |
| QUEUE-03 | Phase 08 | Pending |
| SESS-01 | Phase 09 | Pending |
| SESS-02 | Phase 09 | Pending |
| CLI-01 | Phase 05 | Complete |
| SESS-03 | Phase 09 | Pending |

**Coverage:**
- v1.1 requirements: 21 total
- Mapped to phases: 21
- Unmapped: 0

---
*Requirements defined: 2026-03-26*
*Last updated: 2026-03-26 after roadmap creation*
