# Requirements: GSD Meta Manager

**Defined:** 2026-03-31
**Core Value:** See the state of every GSD project at a glance and act on any of them without leaving the TUI.

## v1.2 Requirements

Requirements for v1.2 Housekeeping & Archive Browser. Each maps to roadmap phases.

### Tech Debt

- [ ] **DEBT-01**: Build compiles with zero warnings (resolve all compiler warnings and `#[allow(dead_code)]` annotations)
- [ ] **DEBT-02**: All integration tests pass with correct CLI argument order and current API assumptions

### Paused Project Detection

- [x] **PAUSE-01**: User sees a pause badge on dashboard rows for projects with a `.planning/HANDOFF.md` or `HANDOFF.json` file

### Milestone Archive Browser

- [x] **ARCH-01**: User can see a list of completed milestones in an Archive tab within the detail view
- [x] **ARCH-02**: User can drill into a milestone to see its phases, then into a phase to see its artifact files
- [x] **ARCH-03**: User can view a selected artifact file with styled markdown rendering (headers, bold, lists, code blocks)
- [x] **ARCH-04**: Archive data is loaded asynchronously and cached (completed milestones are immutable)

### Queue Execution Research

- [x] **QRES-01**: Research document covers GSD autonomous mode lifecycle, hook points, and CLI capabilities (`claude -p`, `--continue`, `--resume`)
- [x] **QRES-02**: Research document includes a design for auto-continue from QUEUE.md with identified integration points and trade-offs

## Future Requirements

Deferred to future release. Tracked but not in current roadmap.

### Visual UAT

- **VUAT-01**: Deferred visual UAT items from v1.1 addressed

### Queue Execution

- **QEXE-01**: User can execute queued GSD commands from the TUI (depends on QRES research)

### Markdown Consistency

- **MKDN-01**: Backlog tab uses same styled markdown rendering as Archive tab

## Out of Scope

| Feature | Reason |
|---------|--------|
| Tree widget for archive navigation | Sequential drill-down matches existing UX patterns |
| New Cargo dependencies (except pulldown-cmark) | Existing stack sufficient for all features |
| Queue execution implementation | Research only this milestone; implementation in v1.3+ |
| Styled markdown in existing tabs | Consistency upgrade deferred; Archive tab is proving ground |

## Traceability

Which phases cover which requirements. Updated during roadmap creation.

| Requirement | Phase | Status |
|-------------|-------|--------|
| DEBT-01 | Phase 10 | Pending |
| DEBT-02 | Phase 10 | Pending |
| PAUSE-01 | Phase 11 | Complete |
| ARCH-01 | Phase 12 | Complete |
| ARCH-02 | Phase 12 | Complete |
| ARCH-03 | Phase 12 | Complete |
| ARCH-04 | Phase 12 | Complete |
| QRES-01 | Phase 13 | Complete |
| QRES-02 | Phase 13 | Complete |

**Coverage:**
- v1.2 requirements: 9 total
- Mapped to phases: 9
- Unmapped: 0

---
*Requirements defined: 2026-03-31*
*Last updated: 2026-03-31 after roadmap creation*
