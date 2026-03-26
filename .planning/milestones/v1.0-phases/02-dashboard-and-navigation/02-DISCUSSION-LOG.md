# Phase 2: Dashboard and Navigation - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-03-25
**Phase:** 02-dashboard-and-navigation
**Areas discussed:** Dashboard layout & density, Color-coding scheme, Status bar design, Search/filter behavior

---

## Dashboard Layout & Density

### Q1a: Columns

| Option | Description | Selected |
|--------|-------------|----------|
| A: Rich columns | Alias, Current Phase Name, Status, Progress (2/4), Backlog | ✓ |
| B: Visual bar | Alias, Phase X of Y, Status, Mini bar [███░░], Path | |
| C: Compact | Alias, Phase Name, Status, Plans (3/5) — no path | |

**User's choice:** A — Rich columns with progress fraction and backlog count
**Notes:** Path moves to detail view (Phase 3)

### Q1b: Progress indicator

| Option | Description | Selected |
|--------|-------------|----------|
| A: Mini bar | Inline progress bar like [███░░] | |
| B: Fraction | Text like "2/4 phases" | ✓ |
| C: Both | Fraction + bar | |

**User's choice:** B — Fraction only

### Q1c: Phase display format

| Option | Description | Selected |
|--------|-------------|----------|
| A: Name only | "Dashboard and Navigation" | |
| B: Number + name | "P2: Dashboard and Nav" | ✓ |
| C: Number only | "Phase 2" | |

**User's choice:** B — Number + truncated name

---

## Color-Coding Scheme

### Q2a: State-to-color mapping

| Option | Description | Selected |
|--------|-------------|----------|
| A: Green=active | Green active, Yellow idle, Red blocked, Gray complete, Magenta unknown | ✓ |
| B: Green=complete | Green complete, Blue active, Yellow idle, Red blocked | |
| C: Custom | User-defined mapping | |

**User's choice:** A — Green=active, Yellow=idle, Red=blocked, Gray=complete, Magenta=unknown

### Q2b: Selected row styling

| Option | Description | Selected |
|--------|-------------|----------|
| A: Reverse video | Current behavior, color lost | |
| B: Bold + underline | Preserves status color | ✓ |
| C: Hybrid | Reverse + colored status cell | |

**User's choice:** B — Bold + underline, preserves status colors while navigating

---

## Status Bar Design

### Q3a: Aggregate bar content

| Option | Description | Selected |
|--------|-------------|----------|
| A: Full text | "5 projects: 2 active, 1 blocked, 1 idle, 1 complete" | |
| B: Minimal | "5 projects \| 2 need attention" | |
| C: Icons | "5 projects: 2 ▶ 1 ⚠ 1 ● 1 ✓" | ✓ |

**User's choice:** C — Icon shorthand for density

### Q3b: Footer layout

| Option | Description | Selected |
|--------|-------------|----------|
| A: Two-line | Top=counts, Bottom=keybinds | |
| B: Single line | Aggregate left, keybinds right | ✓ |
| C: Header+footer | Header for aggregates, footer for keybinds | |

**User's choice:** B — Single line, same layout as Phase 1 but richer

---

## Search/Filter Behavior

### Q4a: Filter UX

| Option | Description | Selected |
|--------|-------------|----------|
| A: Inline footer | Replaces keybind hints, live-filter, Esc to clear | ✓ |
| B: Separate bar | Above footer, always visible when active | |
| C: Overlay | Popup at top, vim-style | |

**User's choice:** A — Inline in footer

### Q4b: Filter scope

| Option | Description | Selected |
|--------|-------------|----------|
| A: Name only | Match alias | |
| B: Name + status | Match alias or status text | |
| C: All columns + selectors | Fuzzy match across all columns with /term/key syntax | ✓ |

**User's choice:** C — Full column match with selector syntax: `/term/p` (phase), `/term/n` (name), `/term/s` (status), bare `/term` for global fuzzy

---

## Claude's Discretion

- Help overlay layout and content
- Fuzzy matching algorithm choice
- Terminal resize handling strategy
- Unicode icon fallback strategy
- Phase name truncation logic

## Deferred Ideas

None — discussion stayed within phase scope
