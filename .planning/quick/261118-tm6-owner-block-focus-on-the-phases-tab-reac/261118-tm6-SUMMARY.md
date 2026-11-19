---
quick_id: 261118-tm6
status: complete
commits:
  - c8dd976e fix: reach the Owner block with Down; no focus on an empty phase list
  - c86bcc02 docs: README + help overlay Esc row
---

# Quick 261118-tm6 Summary: Owner block focus on the Phases tab

## Root cause
- Tab bar `↓`/`j`/`Enter`/`Space` always set `DetailFocus::Content` (phase list),
  skipping the Owner block drawn between them; only `↑` from phase row 0 entered it.
- With "No phases found", `Content` focus had nothing to draw; `↑` then jumped onto the
  block's last stop.
- With the block focused, the phase list still drew its `> ` marker (two cursors).

## Model (inferred I-1)
Reading order tab bar -> Owner block -> phase list in both directions. Landing on the tab
stays on the phase list when it has rows (visible cursor; footer already names `[↑]owner`;
inbox `g` unchanged). With an empty list the block is the only focusable level: the render
draws its first stop as the cursor (`owner_landing`) and the next key/mouse event makes it
real before acting, so a key always lands on the drawn cursor.

## Inferred decisions
- I-1: landing stays on phases when there are phases (rejected: always landing on the block,
  which needs render-side arrival tracking and moves every phase-list key one press away).
- I-2: empty list: `↓` past the last stop stays; `Esc` goes to the tab bar; footer shows
  `[Esc]tab bar`.
- I-3: the phase list keeps its selection (right pane) but draws no `> ` while the block
  has focus; focus is shown by the `> ` gutter marker, not color alone.

## Tests (written first, failed before the fix)
`tab_bar_down_on_the_phases_tab_enters_the_owner_block_first`,
`with_no_phases_the_owner_block_holds_visible_focus`,
`with_no_phases_up_leaves_the_block_it_showed_focused`,
`only_the_focused_level_draws_a_cursor_marker`.

## Verification
`cargo test --no-fail-fast`: lib 3707 passed / 0 failed / 3 ignored; every integration
target green except `owner_audit_conformance::ours_equals_the_oracle_on_every_owner_fixture`
(the external gsd-core oracle errors "No phases directory found" on fixture debug-sessions;
untouched by this task). `cargo clippy --all-targets -- -D warnings`: clean.
