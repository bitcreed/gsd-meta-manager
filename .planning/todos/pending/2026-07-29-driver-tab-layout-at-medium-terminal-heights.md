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
