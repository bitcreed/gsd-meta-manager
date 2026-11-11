---
created: 2026-11-11T00:00:00.000Z
title: Registry glyphs and typography arrows overflow their cell when the terminal font lacks them
area: ui
severity: minor
files:
  - src/ui/glyphs.rs
  - src/ui/screens/normal.rs
  - src/ui/screens/confirm_popup.rs
---

## Problem

Quick 261111-ixj found that the overview Status cell drew its unmerged glyph (U+25D2) over the digit after it.
The cause was not East Asian Width (U+25D2 is Neutral) but font fallback. The reference terminal is Ptyxis (VTE)
with the system monospace font Ubuntu Sans Mono 11, a cell of 0.56 em. That font lacks U+25D2, so fontconfig
substitutes the proportional DejaVu Sans, whose glyph is 0.873 em (1.56 cells). VTE draws the fallback glyph at its
natural width, over the next cell. U+25D2 was replaced by U+2206 (in both mono fonts) and `glyphs::FLUSH_SAFE` now
guards glyphs drawn flush against text.

Other glyphs fall back the same way and overflow. Measured on 2026-11-11 (`fc-list ':file=<font>:charset=<hex>'`,
`hmtx` advances, cell ratio = advance / 0.56 em):

| Glyph | Registry name | Ratio |
|---|---|---|
| U+25CD | `CIRCLE_FILLED` | 1.56 |
| U+25C9 | `FISHEYE` | 1.56 |
| U+25F7 | `CLOCK` | 1.56 |
| U+2717 | `BALLOT_X` | 1.50 |
| U+2762 | `ATTENTION` | 1.50 |
| U+21E1 | `ARROW_UP_DASHED` | 1.50 |
| U+21E3 | `ARROW_DOWN_DASHED` | 1.50 |
| U+23CE | `RETURN_SYMBOL` | 1.50 |
| U+22EF | `MIDLINE_ELLIPSIS` | 1.79 |
| U+2B16 | `DIAMOND_HALF_BLACK` | 1.37 |
| U+25BA | `POINTER_RIGHT` | 1.37 |
| U+25E6 | `CIRCLE_OPEN` | 1.05 |
| U+2190..U+2193 | TYPOGRAPHY arrows | 1.50 |

That is 12 registry glyphs and the 4 TYPOGRAPHY arrows. Each is drawn before a space or at the end of a cell today
(the overview badge constants all end in a space), so the overflow lands on blank space and nothing is covered.
The one known visible case outside the indicator set is the confirm-popup key hint, which puts the U+2190 arrow
flush against a slash (`src/ui/screens/confirm_popup.rs`).

## Why nothing was replaced

The code points that are not East Asian Ambiguous and are present in both Ubuntu Sans Mono and DejaVu Sans Mono
number about 26, mostly currency symbols. Replacing these glyphs therefore means ASCII, which is a design decision
for the operator.

## Options

1. An ASCII badge and state set (`x`, `*`, `o`, `!`, ...) used everywhere.
2. An opt-in ASCII mode, merged with `2026-10-21-ambiguous-wide-typography-borders-and-darkgray`, which already
   asks for an ASCII border-and-typography set.
3. Leave as is and rely on `FLUSH_SAFE` to keep any new flush draw safe.
