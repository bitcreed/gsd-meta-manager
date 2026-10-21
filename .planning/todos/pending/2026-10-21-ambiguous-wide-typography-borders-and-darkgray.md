---
created: 2026-10-21T00:00:00.000Z
title: Borders and typography still draw 2 cells under ambiguous-wide settings; DarkGray remains outside the Driver tab
area: ui
severity: minor
files:
  - src/ui/glyphs.rs
  - src/ui/screens/driver.rs
  - src/ui/screens/driver_steer.rs
  - src/ui/screens/driver_launch.rs
---

## Problem

Quick 261021-1id made every status glyph width-safe (registry in `src/ui/glyphs.rs`) and moved the Driver tab's
Label and Muted roles to `Modifier::DIM`. It left three known limitations, recorded here so they are a decision
rather than an oversight.

1. **The STRUCTURAL and TYPOGRAPHY exemptions are still East Asian Width Ambiguous.** Box drawing and block
   elements (U+2500..=U+259F: borders, rules, graph lanes, the caps bar) and the typography code points
   U+00B7, U+2013, U+2014, U+2026, U+2190..U+2193 (separators, truncation, ranges, key hints, scroll
   indicators) draw 2 cells on a terminal set to ambiguous-wide or CJK widths. ratatui's own `Block` and
   `Gauge` use the same range. Fixing it means an opt-in ASCII border-and-typography set, which is new config,
   so it is the operator's call.
2. **`Color::DarkGray` remains** in the other tabs, in the Driver popups' own `muted()`
   (`driver_steer.rs`, `driver_launch.rs`), and as the inert-state colour on the Driver tab (unlaunched,
   stopped, pending, unseen, `[not live]`, killed). Bright-black equals the background in Solarized-Dark-family
   palettes. Changing a state's colour is an app-wide 18-UI-SPEC call.
3. **The Phase 18/24/25 UI-SPECs keep the pre-swap glyphs** as historical record. `src/ui/glyphs.rs` and
   30-UI-SPEC.md `## Glyph width safety` are authoritative.

## Solution

Operator decision first. If wanted: a single `ascii_fallback` config flag that swaps STRUCTURAL and TYPOGRAPHY for
`+-|`, `-`, `...`, `<-`/`->`/`^`/`v`, plus a theme-aware replacement for the inert-state DarkGray, with the
`ui::glyphs` source scan extended to cover the fallback table.
