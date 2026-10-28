---
created: 2026-10-28T00:00:00.000Z
title: "Human check: does the help popup read as a scannable page at 80x24? (18-11 D6)"
area: ui
severity: minor
needs_human: true
source: .planning/phases/18-driver-tab-live-watch-durable-injection/18-VERIFICATION.md
files:
  - src/ui/screens/help.rs
---

## Problem

The one Phase 18 human-judgement item that survives Phase 31. 18-11 grew the help popup (~60 rows across five
headed sections) and made it scroll. Whether it reads as a scannable page at 80x24, rather than as a wall of
rows, is a perceptual judgement (18-11-SUMMARY.md, human_judgment: true). The other Phase 18 human items (SC1
live watch of `drive` runs, the D11 header, the D6 injection glyph) are obsolete: Phase 31 deleted that code.
Note: the injection gloss named in the original check went with the injection UI (31-02); judge the page as it
is now.

## Solution

Open gsd-meta-manager at exactly 80x24, press `?`, scroll the popup end to end. Pass if the legends and the
scroll indicator land as a coherent page. If not, file the specific fix (section collapse, two-column layout).
