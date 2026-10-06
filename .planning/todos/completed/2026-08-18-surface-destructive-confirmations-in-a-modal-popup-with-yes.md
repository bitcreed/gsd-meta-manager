---
created: 2026-08-18T21:29:21.620Z
title: Surface destructive confirmations in a modal popup with Yes/No buttons
area: ui
severity: minor
files:
  - src/ui/screens/delete_confirm.rs:46-69
  - src/ui/screens/queue_delete_confirm.rs:72-97
  - src/ui/screens/driver_confirm.rs
  - src/ui/screens/mod.rs
---

## Problem

Destructive confirmations are presented as a one-line footer prompt, which is a hardly
visible safety net for an irreversible action.

Deleting a project is the worst case. `DeleteConfirmScreen::render`
(`src/ui/screens/delete_confirm.rs:46-69`) draws only an **empty bordered block** as its
background:

```rust
let block = Block::default().borders(Borders::ALL).title(" GSD Manager ");
frame.render_widget(block, chunks[0]);
```

so the project list the user was just looking at vanishes, and the question
(`Remove "{alias}"? This only unregisters it — project files are not deleted. [y/n]`)
appears alone on the final line. The screen reads as "the list is gone, something happened"
rather than "you are about to remove something, confirm".

The same footer-line pattern is used elsewhere, with varying context preservation:

- `queue_delete_confirm.rs:72-97` — renders the real detail view behind the prompt, so
  context survives, but the question is still a thin red footer line.
- `driver_confirm.rs` — same family of y/n footer gates.

Because the prompt sits where transient status messages also appear, the safety net for an
irreversible action looks identical to routine chatter.

## Solution

Introduce one reusable modal-popup widget for destructive confirmations and route every
y/n safety gate through it, instead of each screen hand-rolling a footer line:

- Centred bordered popup over a **dimmed but still rendered** background (never a blank
  block — losing the list is what makes the current delete screen disorienting).
- Title states the action, body states the consequence and what is *not* affected, and two
  focusable buttons — `[ Yes ]` `[ No ]` — with `No` focused by default so a stray Enter is
  safe.
- Keep `y` / `n` / `Esc` working as accelerators for muscle memory; add Left/Right/Tab to
  move focus and Enter to activate.
- Audit every screen under `src/ui/screens/` for the footer-prompt pattern and migrate them
  together, so the safety-net affordance is consistent rather than per-screen.

Worth checking during implementation whether the existing `ratatui` `Clear` widget plus a
centred `Rect` helper already exists in `src/ui/` before adding one.

Note: the removal guard itself (`do_remove_project`, CR-06 — refuses while a driver run is
`Alive` or `Unknown`) is correct and independent of this change. This is about the
*visibility* of the confirmation, not the rule behind it.
