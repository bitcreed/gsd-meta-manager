---
status: testing
phase: 30-driver-tab-on-direction-md
source: [30-VERIFICATION.md]
started: 2026-10-21T00:00:00Z
updated: 2026-10-21T12:00:00Z
---

## Current Test

number: 2
name: Judge the density of the FULL tier at 100x30
expected: |
  Header, items, caps, connected and log read as a coherent view; this check also covers legibility of the
  replacement glyphs (quick 261021-1id glyph map)
awaiting: user response

## Tests

### 1. Read the DarkGray labels and the glyphs U+2016, U+25C8, U+2691 on the real terminal theme and font
expected: Labels are legible; each glyph renders as a single cell
result: pass
signed_off_by: operator, 2026-10-21 (answer relayed by the coordinator)
reported: labels legible and each glyph a single cell on the operator's terminal
decision: the universality concern (U+2016 and U+25C8 are East Asian Width Ambiguous, U+2691 is Extended_Pictographic; bright-black DarkGray matches the background in Solarized-Dark-family palettes) is addressed by quick 261021-1id, code commits 58ff7c0 (badges and Driver tab run/item glyphs), d7049ae (remaining glyphs through the registry) and 07ae883 (Driver tab labels DIM); RED commits 8421547 and 8ec6976

### 2. Judge the density of the FULL tier at 100x30
expected: Header, items, caps, connected and log read as a coherent view
result: [pending]
note: operator will check separately; this check also covers legibility of the replacement glyphs (quick 261021-1id glyph map) [inferred — audit]

## Summary

total: 2
passed: 1
issues: 0
pending: 1
skipped: 0
blocked: 0

## Gaps
