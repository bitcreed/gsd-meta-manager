---
created: 2026-07-29T00:00:00.000Z
title: Driver tab renders blank pipeline row at terminal heights 8-13
area: ui
severity: minor
resolves_phase: null
files:
  - src/ui/screens/driver.rs
---

## Problem

At terminal heights of roughly 8-13 rows, the Driver tab's medium layout tier renders a
blank pipeline row and a bare `── steps ──` rule with nothing under it. The section
headers survive the height budget but their content does not, so the tab looks broken
rather than compressed.

Logged during Phase 18 as code-review finding WR-05 and deliberately deferred: closing it
needs the UI-SPEC height-tier tables plus observation at a real terminal, not a grep or a
unit test.

## Solution

TBD. Likely either an additional height tier that drops the pipeline row and its rule
together, or a minimum-height floor below which the tab renders a single-line summary
instead of the sectioned layout.

Verify against `18-UI-SPEC.md`'s height-tier tables, and check the neighbouring tabs for
the same class of defect while in there.

## Resolution

Closed 2026-10-28 at HEAD `d5b8699`: the defect no longer exists. Phase 31 (77908eb)
removed the legacy pipeline/steps renderer; the current launcher-run tab
(`src/ui/screens/driver_run.rs`, `height_tier` / `allocate_rows` / `compose_selected`)
has no `pipeline` or `steps` section at all.

Evidence: a throwaway TestBackend test (deleted afterwards; source tree unchanged)
rendered both the bare tab area (`render_launcher_tab`) and the whole `DetailScreen`
on the Driver tab (tab bar 3 rows + content + footer 1) at heights 8-17, widths 80
and 120, for a populated fixture (2 runs, first selected, 3 items with a cursor item)
and for the empty state — 80 frames. A scan for any section rule (`── items`,
`── log`, steps, pipeline) followed by a blank row found 0.

- Whole screen, terminal heights 8-15: content area is 4-11 rows, so the SUMMARY tier
  applies — one row per run, no section headers, footer intact. Nothing can be bare.
- Whole screen, 16-17 (and tab area 12-17): MEDIUM tier — header row 1 + token row,
  three self-labelled cap rows, `── items` with the cursor item (plus `… N more items`
  once a spare row exists), `── log` with its body beneath. Every rule has content.
- Empty state, all heights: bordered `Driver — EXPERIMENTAL` block with the
  "No driver runs yet" copy and the `[s]` hint; no rules.

Neighbouring-tab check was not in scope of this re-verification.
