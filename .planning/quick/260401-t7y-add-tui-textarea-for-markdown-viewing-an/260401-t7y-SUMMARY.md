---
phase: quick
plan: 260401-t7y
subsystem: ui/archive, ui/editor
tags: [tui-textarea, editor, archive, line-numbers]
dependency_graph:
  requires: []
  provides: [editor-shellout, archive-line-numbers]
  affects: [detail-screen, main-loop, backlog]
tech_stack:
  added: [tui-textarea-0.7]
  patterns: [tui-suspend-restore, screen-action-pathbuf]
key_files:
  created: []
  modified:
    - Cargo.toml
    - src/archive.rs
    - src/ui/screens/detail.rs
    - src/ui/screens/mod.rs
    - src/app.rs
    - src/main.rs
    - src/state_reader/backlog.rs
decisions:
  - Kept Paragraph + render_markdown_lines for archive FileView (better markdown styling than tui-textarea raw text); added line number gutter via Layout::horizontal split
  - tui-textarea added as dependency for future editor/preview uses but not used for archive rendering
  - Backlog 'e' key behavior: opens $EDITOR when expanded, enqueues when collapsed (preserves existing UX)
  - Editor resolution order: $VISUAL > $EDITOR > vi
metrics:
  duration: 4min
  completed: "2026-04-02T04:11:05Z"
  tasks: 2
  files: 7
---

# Quick Task 260401-t7y: Add tui-textarea for Markdown Viewing and $EDITOR Shell-out Summary

Line number gutter for archive FileView using Layout::horizontal split with synced scrolling, plus $EDITOR shell-out from archive and backlog views with TUI suspend/restore.

## Task Completion

| Task | Name | Commit | Files |
|------|------|--------|-------|
| 1 | Add tui-textarea dependency and line number gutter | 01b5dec | Cargo.toml, src/archive.rs, src/ui/screens/detail.rs |
| 2 | Add $EDITOR shell-out for non-archived planning files | 248a800 | src/ui/screens/mod.rs, src/ui/screens/detail.rs, src/app.rs, src/main.rs, src/state_reader/backlog.rs |

## What Was Done

### Task 1: Line Number Gutter for Archive FileView
- Added `tui-textarea = { version = "0.7", features = ["ratatui"] }` to Cargo.toml
- Added `line_number_lines()` helper in `src/archive.rs` that generates right-aligned, DarkGray-styled line numbers for a visible window
- Split the archive FileView content area into a gutter column (dynamic width based on total line count) and main content column using `Layout::horizontal`
- Gutter and content scroll stay in sync via the same `archive_scroll_offset`
- Added `[e]dit` hint to Archive tab footer

### Task 2: $EDITOR Shell-out
- Added `SuspendAndEdit(PathBuf)` variant to `ScreenAction` enum
- Added `pending_editor: Option<PathBuf>` field to `App` struct
- Main loop checks `pending_editor` after each update: suspends TUI via `ratatui::restore()`, spawns editor process, waits for exit, re-initializes TUI via `ratatui::init()`
- Archive FileView 'e' key: resolves file path from archive cache, checks if under `/milestones/` (shows "Archived files are read-only"), otherwise dispatches `SuspendAndEdit`
- Backlog expanded view 'e' key: opens the backlog item's .md file in $EDITOR
- Added `pub path: Option<PathBuf>` to `BacklogItem` struct, populated during parsing from `find_first_md_file`
- Editor resolution: `$VISUAL` > `$EDITOR` > `vi` fallback
- Status messages confirm editor open/close/error

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] tui-textarea feature flag correction**
- **Found during:** Task 1
- **Issue:** Plan specified `features = ["ratatui-0.30"]` but tui-textarea 0.7 uses `features = ["ratatui"]`
- **Fix:** Changed feature flag to `["ratatui"]` which is the actual available feature
- **Files modified:** Cargo.toml
- **Commit:** 01b5dec

**2. [Rule 2 - Missing functionality] Backlog 'e' preserves enqueue when not expanded**
- **Found during:** Task 2
- **Issue:** Plan suggested wiring 'e' for backlog but the existing 'e' key already had enqueue behavior
- **Fix:** Made 'e' context-sensitive: opens $EDITOR when backlog item is expanded, falls through to enqueue behavior when collapsed
- **Files modified:** src/ui/screens/detail.rs
- **Commit:** 248a800

## Known Stubs

None -- all features are fully wired.

## Self-Check: PASSED
