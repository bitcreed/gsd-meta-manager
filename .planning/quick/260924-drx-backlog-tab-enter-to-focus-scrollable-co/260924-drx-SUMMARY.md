---
quick_id: 260924-drx
status: complete
date: 2026-09-24
commits: [67e318a, ca80ada, a723b3a]
---

# Quick 260924-drx — Summary

Backlog tab (3): Enter opens AND focuses the item's content pane; while focused
j/k and PgUp/PgDn scroll the content (clamped against the wrapped row count),
Enter again or Esc closes it and returns focus to the list. The pane sits to the
right of the list (40/60) when the tab's inner width >= 100 columns
(`roadmap_view::ROADMAP_SIDE_BY_SIDE_MIN_COLS`), otherwise stacked 50/50 below as
before. With the pane open, `e` opens the file the item lives in: ROADMAP.md at
its `### Phase 999.N` heading line (`$EDITOR +N`), else the dir's first `.md`.

## Commits
- 67e318a feat(backlog): resolve a backlog item's edit target to its ROADMAP.md heading line
- ca80ada feat(edit): SuspendAndEdit carries an optional line; editor_args positions known editors
- a723b3a feat(backlog): Enter focuses a scrollable content pane, side-by-side when wide; e edits ROADMAP.md at the item

## INFERRED decisions (audit later)
- Open == focused: one flag (`backlog_expanded`); Enter/Esc close the pane rather than leaving a
  non-focused preview visible (as the todo proposed). `q` shares the Esc arm, so it closes too.
- Width breakpoint reuses the Roadmap tab's list+detail breakpoint (100 cols).
- ROADMAP.md wins over a dir `.md` when both exist (overrides the todo's own INFERRED note):
  the user said "open the file where it's located"; GSD writes the item into ROADMAP.md.
- Editor line syntax: `+N` for vi/vim/nvim/view/gvim/vis/nano/pico/emacs/emacsclient/kak/micro/
  joe/jed/ne/mg; `-g path:N` for code/codium/cursor; `path:N` for hx/helix/subl/zed; unknown
  editors get the path only.
- Wrapped row count via ratatui's `unstable-rendered-line-info` feature (`Paragraph::line_count`):
  no new crate, exact for the widget's own wrap.
- Scroll resets to 0 on open/close; selection cannot change while focused.

## Verification
- 9 new tests (editor_args, phase_section_line, 2x backlog_edit_target, 5 detail key/render tests).
- `cargo test --no-fail-fast`: 2365 passed, 15 ignored; failures = the known git-version test
  (src/envelope/policy.rs) plus one BrokenPipe race in tests/envelope_carrier_reach.rs
  (`the_t_19_116_replacement_takes_layer_2...`) that passes 3/3 in isolation — unrelated.
- `cargo clippy --all-targets --keep-going -- -D warnings`: 11 errors, all pre-existing, none in
  touched files; `--lib --bins` clean.

## Deviation
- Planned and executed inline by the quick-task agent (no separate gsd-planner/gsd-executor
  dispatch) — the todo was already a full spec; ISOLATION=none.
