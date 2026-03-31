# Stack Research: v1.2 New Feature Dependencies

**Domain:** TUI project manager -- archive browser tab, paused-project detection, queue execution research
**Researched:** 2026-03-31
**Confidence:** HIGH (versions verified via crates.io API and Cargo.toml inspection)

## Context

This research covers ONLY new dependencies needed for v1.2 features. The existing stack is validated and not re-evaluated:
- Core: Rust 1.85+, ratatui 0.30, crossterm 0.29, tokio 1.50, notify-debouncer-full 0.5
- Data: serde 1, serde_json 1, serde_yml 0.0.12, toml (not currently in Cargo.toml but in CLAUDE.md), chrono 0.4
- App: anyhow 1, color-eyre 0.6, clap 4, tracing 0.1, regex 1, dirs 6, tempfile 3, futures 0.3

## v1.2 Feature Requirements

1. **Milestone Archive Browser tab** -- browse `.planning/milestones/` tree: milestone versions, archived phases, plan/summary/verification artifacts
2. **Paused-project detection** -- read HANDOFF.md presence, show badge on dashboard
3. **Queue execution research** -- document-only; no new runtime dependencies
4. **Tech debt cleanup** -- no new dependencies

## Recommended Stack Additions

### New Dependencies

| Library | Version | Purpose | Why Recommended |
|---------|---------|---------|-----------------|
| *None* | -- | -- | No new crate dependencies required for v1.2 |

### Why No New Dependencies

The v1.2 features are achievable with the existing stack. Here is the analysis for each capability:

#### Milestone Archive Browser (Tab 8)

**Data structure:** The archive is a simple directory tree:
```
.planning/milestones/
  v1.0-MILESTONE-AUDIT.md
  v1.0-REQUIREMENTS.md
  v1.0-ROADMAP.md
  v1.0-phases/
    01-core-infrastructure/
      01-01-PLAN.md
      01-01-SUMMARY.md
      01-CONTEXT.md
      01-VERIFICATION.md
      ...
    02-dashboard-and-navigation/
      ...
```

**Navigation approach:** Use a flat list with indentation (the pattern already used by the Backlog and Pipeline tabs), not a tree widget. Rationale:

- The hierarchy is exactly 3 levels deep (milestone -> phase -> artifact). This is too shallow for a tree widget to add value over indented list items.
- The existing `ListState`-based navigation pattern is proven across 4 tabs (Backlog, Git, Pipeline, Queue). Adding a 5th tab with the same pattern keeps the codebase consistent.
- `tui-tree-widget` (0.24.0) is a well-maintained crate compatible with ratatui 0.30 (uses ratatui-core 0.1.0 + ratatui-widgets 0.3.0), but introduces a new widget paradigm (TreeState, TreeItem) that differs from the app's ListState pattern. The cognitive overhead is not justified for 3-level nesting.

**Content preview:** Raw markdown in a `Paragraph` widget, matching the existing Backlog tab pattern. No markdown rendering library needed because:

- The backlog tab already displays `.md` content as raw text and users have accepted this UX.
- Archive artifacts (PLAN.md, SUMMARY.md, VERIFICATION.md) are structured with headings and bullet points that read well as plain text in a TUI.
- `tui-markdown` (0.3.7, compatible with ratatui 0.30 via ratatui-core) adds styled headings, bold, code blocks, but the value is marginal for read-only browsing of planning artifacts. The added dependency (pulls in `pulldown-cmark` 0.13.3) and rendering complexity is not justified.
- If markdown rendering is desired later, `tui-markdown` is the correct choice -- it returns a `ratatui::text::Text` value that drops into `Paragraph` with zero architecture changes.

**Implementation approach:**
- New `ArchiveItem` struct in a new `src/state_reader/archive.rs` module (follows `backlog.rs` pattern)
- Parse `milestones/` directory: list `v*-phases/` dirs, then phase subdirs, then `.md` files
- Store as flat `Vec<ArchiveItem>` with depth field for indentation rendering
- New `DetailSubView::Archive` variant, tab "8:Archive"
- Split-pane on Enter: list top, content bottom (same as Backlog)

#### Paused-Project Detection (HANDOFF.md)

**No new dependencies.** Detection is a single `Path::exists()` check for `.planning/HANDOFF.md`. The file's presence means the project is paused/handed off.

**Implementation approach:**
- Add `is_paused: bool` to the project state struct
- Check in the existing disk_status scan: `planning_dir.join("HANDOFF.md").exists()`
- Render a `[PAUSED]` badge in the dashboard row, similar to the existing Claude session indicator

#### Queue Execution Research

**This is a documentation-only deliverable.** No code changes, no new dependencies. The research output will document:
- GSD autonomous mode (`/gsd:autonomous`) session lifecycle
- How QUEUE.md items map to GSD slash commands
- What hooks/injection points exist for triggering GSD from external tools
- Design sketch for future auto-continue from QUEUE.md (v2+)

#### Markdown Rendering (Future Consideration)

If markdown rendering becomes desirable (e.g., for a richer archive browsing experience), the upgrade path is straightforward:

| Library | Version | ratatui Compat | Integration Effort |
|---------|---------|----------------|-------------------|
| tui-markdown | 0.3.7 | 0.30 (via ratatui-core 0.1.0) | LOW -- returns `Text`, drops into `Paragraph` |
| the-other-tui-markdown | 0.1.0 | Unknown | NOT RECOMMENDED -- too new, unproven |
| markdown-tui | recent | Unknown | NOT RECOMMENDED -- different API, less mature |

`tui-markdown` is the only option worth considering. It is maintained by joshka (a ratatui core maintainer), uses pulldown-cmark for CommonMark parsing, and has optional syntax highlighting via the `highlight-code` feature. Adding it later is a one-line change: replace `Paragraph::new(raw_text)` with `Paragraph::new(tui_markdown::from_str(raw_text))`.

#### Tree Widget (Explicitly Not Adding)

| Library | Version | ratatui Compat | Why Not Adding |
|---------|---------|----------------|----------------|
| tui-tree-widget | 0.24.0 | 0.30 (via ratatui-core 0.1.0 + ratatui-widgets 0.3.0) | Archive hierarchy is only 3 levels; indented flat list is simpler, consistent with existing tabs, and avoids introducing TreeState/TreeItem alongside the established ListState pattern |

If the app later adds deeply nested navigation (e.g., filesystem browser, dependency graphs), `tui-tree-widget` is the correct crate to adopt.

## What NOT to Add

| Avoid | Why | Use Instead |
|-------|-----|-------------|
| tui-tree-widget | Overkill for 3-level archive; introduces new widget paradigm | Flat `Vec<ArchiveItem>` with depth-based indentation in existing `List` widget |
| tui-markdown / pulldown-cmark | Marginal UX gain for read-only planning artifacts; raw text works | `Paragraph::new(raw_text)` -- upgrade path exists if needed later |
| sysinfo | Was considered in v1.1 research but pgrep+/proc was chosen; no change in v1.2 | Existing pgrep + /proc session detection |
| petgraph | Was considered in v1.1 for execution flow; flat Vec was sufficient; no change in v1.2 | Direct DiskInference struct |
| walkdir | Tempting for recursive directory traversal, but `std::fs::read_dir` with 3-level nesting is trivial | Nested `std::fs::read_dir` calls (matches `backlog.rs` pattern) |

## Cargo.toml Changes

**None required.** All v1.2 features use existing dependencies.

## Integration Points for Archive Browser

The archive browser follows established patterns. Key reuse points:

| Existing Pattern | Source | Reuse For |
|-----------------|--------|-----------|
| `BacklogItem` struct + `parse_backlog_items()` | `state_reader/backlog.rs` | `ArchiveItem` struct + `parse_archive_tree()` |
| `load_backlog_content()` | `state_reader/backlog.rs` | `load_archive_content()` |
| `render_backlog_tab()` split-pane | `ui/screens/detail.rs` | `render_archive_tab()` split-pane |
| `DetailSubView` enum + tab index | `ui/screens/detail.rs` | Add `Archive` variant at index 7 |
| `ProjectViewCache` fields | `app.rs` | Add `archive_items`, `archive_selected`, `archive_expanded` |
| Lazy loading on tab switch | `switch_tab()` in detail.rs | Load archive tree on first switch to Archive tab |

## GSD CLI Inspection (Queue Execution Research)

No stack additions needed. The research phase will examine:

| GSD Component | Location | What to Document |
|--------------|----------|-----------------|
| Autonomous workflow | `~/.claude/get-shit-done/workflows/autonomous.md` | Session lifecycle, phase loop, pause conditions |
| gsd-tools.cjs | `~/.claude/get-shit-done/bin/gsd-tools.cjs` | CLI subcommands, JSON output format, `init milestone-op` |
| QUEUE.md format | `.planning/QUEUE.md` | Item format, how GSD sessions consume items |
| Transition workflow | `~/.claude/get-shit-done/workflows/transition.md` | Phase completion hooks, auto-advance behavior |

This is a documentation deliverable -- the output is a design document in `.planning/research/`, not code.

## Version Compatibility (v1.2 stack)

No compatibility changes from v1.1. The existing Cargo.toml remains as-is:

| Package | Version in Cargo.toml | Status |
|---------|----------------------|--------|
| ratatui | 0.30 | Current stable |
| crossterm | 0.29 | Current stable, compatible with ratatui 0.30 |
| tokio | 1 | Stable, LTS-like |
| notify | 8 | Stable (9.0 still rc) |
| notify-debouncer-full | 0.5 | Compatible with notify 8 |
| chrono | 0.4 | Stable |
| All others | Current | No updates needed |

## Sources

- crates.io API -- tui-tree-widget 0.24.0 confirmed, deps: ratatui-core ^0.1.0, ratatui-widgets ^0.3.0 (HIGH confidence, verified via API)
- crates.io API -- tui-markdown 0.3.7 confirmed, deps: pulldown-cmark ^0.13.0, ratatui-core (HIGH confidence, verified via API)
- crates.io search -- pulldown-cmark 0.13.3 latest stable (HIGH confidence)
- GitHub Cargo.toml -- tui-markdown workspace uses ratatui 0.30.0 (HIGH confidence, raw file verified)
- GitHub Cargo.toml -- tui-tree-widget uses ratatui-core 0.1.0, ratatui-widgets 0.3.0, ratatui 0.30 as dev-dep (HIGH confidence, raw file verified)
- Existing codebase -- `src/state_reader/backlog.rs`, `src/ui/screens/detail.rs` tab patterns (HIGH confidence, read directly)
- Filesystem -- `.planning/milestones/` directory structure (HIGH confidence, listed directly)
- GSD workflows -- `~/.claude/get-shit-done/workflows/autonomous.md` (HIGH confidence, read directly)

---
*Stack research for: v1.2 Housekeeping & Archive Browser*
*Researched: 2026-03-31*
