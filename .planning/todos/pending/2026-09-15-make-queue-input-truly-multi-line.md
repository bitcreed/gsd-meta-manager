---
created: 2026-09-15T16:33:45.075Z
title: Make queue input truly multi-line
area: ui
severity: minor
files:
  - src/ui/screens/enqueue.rs:38-45
  - src/ui/screens/enqueue.rs:90-95
  - src/ui/screens/enqueue.rs:126-128
---

## Problem

The queue-entry input in `EnqueueScreen` (`src/ui/screens/enqueue.rs`) reads
as single-line to the user even though nothing in the data model actually
caps it at one line — `ctx.input_buffer` is a plain `String` with no length
limit. The UX just behaves like a single-line field:

- `KeyCode::Enter` (`enqueue.rs:38`) always submits the buffer and pops the
  screen; there's no way to insert a literal newline, so multi-line intent
  has no key binding.
- `KeyCode::Char(c)` (`enqueue.rs:90-95`) pushes onto the flat `String` with
  no newline-aware cursor/wrap handling.
- The footer render (`enqueue.rs:126-128`) draws the buffer as a single
  `ratatui::text::Line`/`Span`, so even if a `\n` char ended up in the
  buffer, it wouldn't visually wrap to a second line in the current layout.

User's own framing: "it's not that it's actually limited but it feels that
way" — this is a UX/design gap, not a data-model bug.

## Solution

TBD — this todo's deliverable is a **UX proposal**, not a fix. Whoever picks
this up should propose (not necessarily implement) a way to make multi-line
editing feel natural in a ratatui TUI footer/input, e.g.:

- A modifier-chord for "insert newline" (e.g. Alt+Enter or Shift+Enter,
  terminal-support permitting) vs. plain Enter to submit, with a visible
  hint in the footer.
- Swapping the single `Line`/`Span` footer render for a wrapping
  `Paragraph` (ratatui already supports `Wrap`) so long/multi-line input
  visibly grows beyond one row, with the enqueue screen's layout
  constraints adjusted to give it room.
- Cursor/line navigation (up/down between wrapped or explicit lines) if
  newlines are allowed in queued commands at all — worth checking whether
  `queue_md::QueuedAction { command }` and its consumers tolerate embedded
  `\n` before committing to this.

Candidate reference point: `src/ui/screens/driver_inject.rs`, which may
share similar input-buffer patterns and is worth checking for prior art or
the same limitation.
