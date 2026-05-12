---
quick_id: 260512-fe6
description: Add markdown document browser tab for .planning/ rooted at active phase
status: complete
date: 2026-05-12
commits:
  - 18d3ed9 feat(quick-260512-fe6): add browser module for .planning/ docs traversal
  - 106cbcc feat(quick-260512-fe6): wire Docs browser tab into detail view
---

# Quick Task 260512-fe6 — Summary

## What Shipped

A new `0:Docs` tab in the detail view. Opens on the project's **active phase directory** by default (or `.planning/` root if the milestone is complete), shows directories first then `.md` files, and lets the user drill in with `Enter` and pop back with `Esc`. Quick-jumps: `g` → `.planning/` root, `p` → entry phase dir.

## Files

- **`src/browser.rs`** (new) — `BrowserDepth { List | View }`, `BrowserEntry`, `list_dir`, `resolve_active_phase_dir`, `read_md_file`. 7 unit tests.
- **`src/lib.rs`** — `pub mod browser;`.
- **`src/app.rs`** — `DetailSubView::Browse` variant.
- **`src/ui/screens/mod.rs`** — added `browser_depth`, `browser_current_dir`, `browser_root`, `browser_entry_dir`, `browser_entries`, `browser_selected`, `browser_scroll_offset`, `browser_file_content`, `browser_file_name` to `ProjectViewCache`.
- **`src/ui/screens/detail.rs`** —
  - `TAB_TITLES` extended (`"0:Docs"`); `tab_index` / `sub_view_from_index` updated.
  - `switch_to_tab` lazy-initializes the browser on first activation.
  - `KeyCode::Char('0')` binds to tab 9.
  - Esc/q drops `View → List` or `List → parent` (above root) before pop.
  - `j`/`k` / `PageUp`/`PageDown` move selection in `List` or scroll in `View`.
  - `Enter` descends into directory or opens `.md` into `View`.
  - `g` jumps to `.planning/` root; `p` returns to entry phase dir.
  - `render_browser_tab` shows a breadcrumb header (`Docs > .planning/<rel>`) plus a `List` of entries (`[DIR]` prefix for directories) or a markdown viewer with line-number gutter (`archive::line_number_lines` + `archive::render_markdown_lines`).
  - `build_footer` adds Browse-specific hints (`[Enter]open [Esc]up [g]root [p]hase`).
- **`README.md`** — Features line updated from 9-tab to 10-tab, mentions Docs.

## Design Decisions (from /gsd-quick discussion)

- New tab in detail view (vs modal/overlay) → matches existing Archive idiom.
- Active phase as entry, with `g`/`p` quick-jumps for root/phase.
- Drill-down list → view → back (Archive-style), not split pane.
- `.md` + directories only (hides JSON, code, dotfiles).

## Verification

- `cargo build` — clean
- `cargo test` — **118 passed** (111 prior + 7 new in `browser::tests`)
- `cargo clippy -- -D warnings` — clean

## Out of Scope

- In-place editing (Backlog already uses `$EDITOR`; could later add `[e]` hook).
- Non-`.md` files (JSON/code/etc.) — by decision.
- Fuzzy search across `.planning/` — possible follow-up.
