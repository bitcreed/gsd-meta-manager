---
created: 2026-09-24T06:14:50.145Z
title: "Backlog tab: Enter-to-focus scrollable content pane + edit opens ROADMAP.md section"
area: ui
severity: major
completed: 2026-09-24
resolved_by: quick 260924-drx (67e318a..a723b3a)
files:
  - src/ui/screens/detail.rs:2743-2745
  - src/ui/screens/detail.rs:3796-3808
  - src/ui/screens/detail.rs:4206-4310
  - src/ui/screens/mod.rs:337
  - src/app.rs:2236
  - src/state_reader/backlog.rs:259
---

<!-- severity: INFERRED (major, "wrong behavior": long items are cut off with no way to read them; edit key reports "No file found" although content exists). Human was unavailable to confirm; audit later. -->

## Problem

Follow-up from debug session `.planning/debug/resolved/backlog-content-empty.md`
(commits 3a8d77e, d44c160). The 3:Backlog tab's content pane now shows the
item's ROADMAP.md `### Phase 999.N` section plus any `.md` files in the
`999.*` backlog directory (`backlog::load_backlog_content`). Two gaps remain:

1. **Content pane does not scroll.** Long items are truncated at the pane
   bottom. Enter currently toggles `cache.backlog_expanded` (detail.rs ~2743),
   and the pane is always stacked under the list (render_backlog_tab ~4268).
   j/k and PgUp/PgDn always move the list selection.
2. **Edit key says "No file found".** When expanded, the edit key opens
   `item.path` in `$EDITOR`; if the backlog dir has no `.md` file,
   `item.path` is `None` and it returns
   `SetStatusMessage("No file found for this backlog item")` (detail.rs
   ~3807), even though the displayed content comes from ROADMAP.md.

User's words: "When ENTER is pressed, the item appears (right hand side if
space is available, otherwise underneath where it's now) - after ENTER is
pressed, j/k PGUP/PGDN scroll the buffer instead of entries until ENTER is
pressed again or Esc is pressed. Yes, when the edit key is pressed we have to
open the file where it's located in."

## Solution

Wanted behaviour:

- **Enter** opens/focuses the content pane: side-by-side on the right when the
  terminal is wide enough, otherwise stacked below the list (current layout).
  Width threshold is TBD (INFERRED: reuse whatever breakpoint other
  split-pane views in detail.rs use, if any).
- **While focused**, j/k and PgUp/PgDn scroll the content buffer instead of
  moving the selection. Scroll offset must be clamped to content end (see
  completed todo `2026-04-05-fix-pagedown-scroll-offset-not-clamped-to-content-end.md`
  for the prior pattern). INFERRED: reset scroll offset when the selected item
  changes or the pane is closed.
- **Enter again or Esc** returns focus to list navigation (INFERRED: this also
  collapses the pane, matching today's Enter toggle; revisit if the user wants
  the preview to stay visible while navigating).
- **Edit key** opens the file the content actually lives in: the backlog dir's
  `.md` file when present, otherwise `.planning/ROADMAP.md`, ideally at the
  `### Phase 999.N` heading line. Note: `ScreenAction::SuspendAndEdit` currently
  carries only a `PathBuf` (screens/mod.rs:337, handled at app.rs:2236), so a
  line-jump needs a new optional line field and an editor-specific `+N`
  argument (`+N file` works for vim/nvim/nano/emacs; VS Code needs `-g file:N`; fall back to
  path only when the editor is unknown). INFERRED: when both a dir `.md` and a
  ROADMAP section exist, keep opening the dir file (current behaviour).
- Update the footer/help keybinding hints for the focused state.
