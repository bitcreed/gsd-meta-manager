# Requirements: GSD Meta Manager

**Defined:** 2026-03-26
**Core Value:** See the state of every GSD project at a glance and act on any of them without leaving the TUI.

## v1.1 Requirements

Requirements for the v1.1 milestone. Each maps to roadmap phases.

### State Reader

- [ ] **STATE-01**: User sees accurate plan counts per phase on dashboard (fix regex for standalone PLAN.md)
- [ ] **STATE-02**: User sees "Complete" instead of "P5: Unknown" when all phases are done
- [ ] **STATE-03**: User sees phase status inferred from disk files (discuss/research/plan/execute/verify stages)

### GSD Integration

- [ ] **GSD-01**: User can opt into enriching state with cached gsd-tools.cjs JSON output
- [ ] **GSD-02**: User sees [verified] vs [inferred] badges on status fields

### Backlog

- [ ] **BLOG-01**: User can browse backlog items (999.*) in a scrollable list within detail view
- [ ] **BLOG-02**: User can view backlog item details (markdown content)
- [ ] **BLOG-03**: User can queue a promotion command for a backlog item

### Git History

- [ ] **GIT-01**: User can view scrollable git log for a project
- [ ] **GIT-02**: User can toggle between full repo and .planning/-scoped history
- [ ] **GIT-03**: User can view commit diff stats by selecting a commit

### Execution Flow Graph

- [ ] **FLOW-01**: User sees per-phase pipeline visualization (discuss/research/plan/execute/verify)
- [ ] **FLOW-02**: User sees color-coded status per stage (not started, current, complete, skipped)
- [ ] **FLOW-03**: User sees plan execution progress as fraction in the execute stage

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
| STATE-01 | — | Pending |
| STATE-02 | — | Pending |
| STATE-03 | — | Pending |
| GSD-01 | — | Pending |
| GSD-02 | — | Pending |
| BLOG-01 | — | Pending |
| BLOG-02 | — | Pending |
| BLOG-03 | — | Pending |
| GIT-01 | — | Pending |
| GIT-02 | — | Pending |
| GIT-03 | — | Pending |
| FLOW-01 | — | Pending |
| FLOW-02 | — | Pending |
| FLOW-03 | — | Pending |
| QUEUE-01 | — | Pending |
| QUEUE-02 | — | Pending |
| QUEUE-03 | — | Pending |
| SESS-01 | — | Pending |
| SESS-02 | — | Pending |
| SESS-03 | — | Pending |

**Coverage:**
- v1.1 requirements: 20 total
- Mapped to phases: 0
- Unmapped: 20 ⚠️

---
*Requirements defined: 2026-03-26*
*Last updated: 2026-03-26 after initial definition*
