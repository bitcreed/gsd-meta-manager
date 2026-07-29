---
created: 2026-07-29T00:00:00.000Z
title: Badge glyphs may misalign by one cell across terminals
area: ui
severity: cosmetic
files:
  - src/ui/screens/normal.rs
---

## Problem

`alias_badge` (`normal.rs:61-77`) emits raw badge glyphs `⏸` / `⏳` / `▶`. The actual
glyph advance width for these varies by terminal emulator and font beyond what
`unicode-width` and ratatui's `Span::width()` report — `⏳` in particular is
East_Asian_Width=Wide. Badged dashboard rows can therefore misalign by one cell
relative to unbadged rows, depending on the user's terminal.

Raised by the Phase 14 code review as WR-01 and carried forward unchanged: it is
narrower than, and separate from, the Status-column clipping defect that Phase 14
closed. Requires a human check across terminal emulators to characterise.

## Solution

TBD — options include pinning a fixed-width badge cell regardless of glyph, choosing
Narrow-width glyphs, or padding based on measured width. Needs the cross-terminal
observation first.
