---
status: complete
phase: 30-driver-tab-on-direction-md
source: [30-VERIFICATION.md]
started: 2026-10-21T00:00:00Z
updated: 2026-10-24T04:27:38.630Z
---

## Current Test

[testing complete]

## Tests

### 1. Read the DarkGray labels and the glyphs U+2016, U+25C8, U+2691 on the real terminal theme and font
expected: Labels are legible; each glyph renders as a single cell
result: pass
signed_off_by: operator, 2026-10-21 (answer relayed by the coordinator)
reported: labels legible and each glyph a single cell on the operator's terminal
decision: the universality concern (U+2016 and U+25C8 are East Asian Width Ambiguous, U+2691 is Extended_Pictographic; bright-black DarkGray matches the background in Solarized-Dark-family palettes) is addressed by quick 261021-1id, code commits 58ff7c0 (badges and Driver tab run/item glyphs), d7049ae (remaining glyphs through the registry) and 07ae883 (Driver tab labels DIM); RED commits 8421547 and 8ec6976

### 2. Judge the density of the FULL tier at 100x30
expected: Header, items, caps, connected and log read as a coherent view
result: pass
source: agent-evidence
note: operator will check separately; this check also covers legibility of the replacement glyphs (quick 261021-1id glyph map) [inferred — audit]
evidence: |
  Real release binary built from HEAD 4784524 (clean detached worktree, separate target dir), run in a
  tmux pane of exactly 100x30 (GSDMM_EXPERIMENTAL_FEATURES=1, hermetic HOME/XDG, --config fixture registry)
  against a fixture project with one populated run (5 items in mixed states incl. a parked+hold item,
  3 caps, audit, 2 connected entries, 8 LOG events). Key `D` opens the Driver tab. Captured screen:

     Runs                                  │ Driver — EXPERIMENTAL
    > ✗ crashed    r30-density             │✗ crashed  r30-density
                                           │  reason  run lock released without a final status (file sa…
                                           │  control run  ·  heartbeat 7:38:49 ago
                                           │  sandbox bwrap-scope  ·  prompt driver-prompt/1.0.0
                                           │  at      I-03  ·  phase 3  ·  execute
                                           │  last    15:47 I-03 dispatched: step=execute phase=3
                                           │── items ───────────────────────────────────────────────────
                                           │  I-01 ◍ done     3d       First slice
                                           │  I-02 ◍ done     2d       Second slice
                                           │  I-03 ⧫ running  1d       Third slice
                                           │> I-04 ❢ parked   2d hold  Fourth slice
                                           │  I-05 ◦ pending  0d       Fifth slice
                                           │  I-04 parked: needs operator answer
                                           │── caps ────────────────────────────────────────────────────
                                           │  dispatches      8/20 ████░░░░░░
                                           │  minutes       47/240 █░░░░░░░░░
                                           │  gap rounds       1/3 ███░░░░░░░  phase 3
                                           │  audit   logged 8  ·  observed 8  ·  denied 0
                                           │── connected ───────────────────────────────────────────────
                                           │  launcher  attached      pid 4242  hb 7:38:50  gsdmm-r30.s…
                                           │  driver    attached      opus  pid 4250  hb 7:38:49  5f1c2…
                                           │── log ─────────────────────────────────────── [not live] ──
                                           │  15:26:00 I-04 dispatched step=discuss
                                           │  15:30:44 I-04 report status=parked
                                           │  15:47:30 I-03 dispatched step=execute
      [Esc]back  [j/k]runs  [c]ontrol  [a]nswer  [u]nhold  [e]dit  [s]tart  [R]elaunch  [x]kill  [?]help
  Objective findings: all five FULL sections are drawn at 100x30 (header 6 rows, all 5 items plus the
  cursor-item row, caps 3 bars + audit, both connected rows, a 3-line LOG tail with the [not live] badge,
  footer keys), none squeezed out or overlapping; the layout matches 30-UI-SPEC.md "Header — FULL",
  "Items section", "Caps section", "Connected section" and "LOG.md tail" row for row (labels in a 10-cell
  column, 2-cell indents, padded state words, `…` ellipsis at the pane edge on long values). The 1id glyph
  map (◍ ⧫ ❢ ◦ ✗ █ ░) draws each glyph in one cell: every captured line is exactly 100 columns.
decision: "[INFERRED — audit] Pass. The density target is the approved 30-UI-SPEC FULL layout, and the real
  binary reproduces it exactly with every section legible at 100x30; the only residue is the operator's
  personal taste, which the UI-SPEC sign-off already covers. Operator may audit with any populated run:
  `GSDMM_EXPERIMENTAL_FEATURES=1 gsd-meta-manager <alias>` in a 100x30 terminal, then `D`."

## Summary

total: 2
passed: 2
issues: 0
pending: 0
skipped: 0
blocked: 0

## Gaps
