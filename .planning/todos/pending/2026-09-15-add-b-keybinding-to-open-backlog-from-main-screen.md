---
created: 2026-09-15T16:37:10.776Z
title: Add b keybinding to open backlog from main screen
area: ui
severity: minor
files:
  - src/ui/screens/normal.rs:400-470
  - src/ui/screens/normal.rs:520-524
  - src/ui/screens/detail.rs:534-565
---

## Problem

On the main/overview screen (`NormalScreen`), reaching a project's backlog
requires pressing `Enter` to open the detail view, then pressing `3` (or
navigating tabs) to land on the `Backlog` sub-view (`DetailSubView::Backlog`,
tab index 2 / label `"3:Backlog"`). The overview already surfaces a non-zero
backlog count per project in its "Backlog" column (`normal.rs:325`,
`normal.rs:744` `backlog_cell`), so the count is visible right where the user
would want one-key access to the underlying items.

The key `'b'` is currently unbound in `NormalScreen::handle_key`
(`normal.rs:400`). The collision-check comment above the driver keys block
(`normal.rs` ~446-452) enumerates claimed keys as `q, j, k, a, c, d, r, x, o,
/, ?, Tab, Enter, Up, Down` — `b` is not among them, so it's free to bind.

## Solution

TBD — approach hint: bind `KeyCode::Char('b')` in `NormalScreen::handle_key`
similarly to the existing `Enter` handler (`normal.rs:520-524`, which does
`ScreenAction::Push(Box::new(DetailScreen::new(alias)))`), but land directly
on the Backlog sub-view instead of whatever tab was last active for that
project. `DetailScreen::new` / `ctx.detail_sub_view_per_project` would need a
way to set the initial sub-view to `DetailSubView::Backlog` (tab index 2, see
`detail.rs:534-565` `tab_index`/`sub_view_from_index`) before pushing, and the
Backlog data-loading logic in `switch_to_tab` (`detail.rs:1006-1029`) should
run for this entry path too so the tab isn't blank on first paint.
