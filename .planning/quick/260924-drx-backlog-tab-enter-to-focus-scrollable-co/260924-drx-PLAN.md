---
quick_id: 260924-drx
mode: quick
source_todo: .planning/todos/pending/2026-09-24-backlog-tab-enter-to-focus-scrollable-content-pane-edit-open.md
---

# Quick 260924-drx: Backlog tab — Enter-to-focus scrollable content pane + edit opens ROADMAP.md section

Planned inline by the quick-task agent (human unavailable; no separate planner
dispatch — the todo is already a full spec). ISOLATION=none (worktree
base-check false negative on this repo).

## Task 1 — Editor line jump (`SuspendAndEdit` carries an optional line)

- files: src/ui/screens/mod.rs, src/app.rs, src/main.rs, src/ui/screens/detail.rs
- action: `ScreenAction::SuspendAndEdit(PathBuf, Option<usize>)`; `App::pending_editor`
  carries the line; `main.rs` builds argv through a pure `editor_args(editor, path, line)`:
  `+N path` for vi/vim/nvim/view/gvim/nano/pico/emacs/emacsclient/kak/micro/joe/jed/ne/mg,
  `-g path:N` for code/codium/code-insiders/cursor/vscodium, `path:N` for hx/helix/subl/zed,
  path only for any other editor. **INFERRED:** unknown editors get no line (never a
  guessed flag that could make the editor open a file literally named `+N`).
- verify: unit tests on `editor_args`.

## Task 2 — Edit target = where the item lives

- files: src/state_reader/roadmap_md.rs, src/state_reader/backlog.rs, src/ui/screens/detail.rs
- action: `roadmap_md::phase_section_line` (1-based heading line, same fence-aware
  matcher as `phase_section`, shared via one locator); `backlog::backlog_edit_target`
  returns `(ROADMAP.md, Some(line))` when the item has a ROADMAP section, else the
  dir's first `.md` (line None), else None.
  **INFERRED (overrides the todo's own INFERRED note):** ROADMAP.md wins over a dir
  `.md` when both exist — the user said "open the file where it's located", GSD's
  backlog capture writes the item into ROADMAP.md, and the pane shows that section
  first; dir `.md`s are secondary CONTEXT/RESEARCH artifacts.
- verify: unit tests for both fns + key-level test (`e` with pane open).

## Task 3 — Enter focuses a scrollable pane; side-by-side when wide

- files: src/ui/screens/detail.rs, src/ui/screens/mod.rs, src/ui/screens/help.rs, Cargo.toml
- action: `backlog_expanded` stays the open/focused flag (INFERRED: open == focused,
  Enter/Esc close it, per the todo). New `backlog_scroll` in the view cache and a
  `backlog_viewport: Cell<ViewportMetrics>` on the screen; while open j/k and
  PgUp/PgDn scroll via `clamp_scroll` (add-then-clamp down, clamp-then-sub up — the
  UIFIX-04 pattern). Scroll resets on open/close. Esc (and q, which shares the arm)
  closes the pane before popping. Side-by-side (list 40% / content 60%) when the
  tab's inner width >= `roadmap_view::ROADMAP_SIDE_BY_SIDE_MIN_COLS` (100, INFERRED:
  the existing list+detail breakpoint), else stacked 50/50 as today. Wrapped row
  count comes from ratatui's `Paragraph::line_count` (feature
  `unstable-rendered-line-info`, INFERRED: no new crate, exact for the widget's own
  wrap) so the clamp matches what is drawn. Per-line escaping via
  `render_markdown_lines` is kept. Footer: focused state advertises
  scroll/close/edit; help gets Backlog rows.
- verify: key-level tests for scroll/clamp/Esc/Enter, layout test at 140 vs 80 cols.
