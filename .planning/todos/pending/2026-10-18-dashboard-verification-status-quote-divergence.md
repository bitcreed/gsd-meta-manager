---
created: 2026-10-18T00:00:00.000Z
title: Dashboard renders quoted VERIFICATION status as Unknown while driverctl counts it as a gap round
area: state_reader
severity: minor
files:
  - src/state_reader/disk_status.rs:101-111
  - .planning/spikes/003-divergent-gap-loop-validation/tools/gapcount.py:63-73
---

## Problem

A VERIFICATION.md whose frontmatter reads `status: "gaps_found"` (quoted) is treated two ways:

- Phase 27's gap-round counter ports `gapcount.fm_status`
  (`^status:\s*["']?([A-Za-z_]+)`), which strips the optional quote, so the write **counts as a gap round**.
- The dashboard's `VerificationStatus::from_raw` (`src/state_reader/disk_status.rs:101-111`) matches the trimmed
  raw value without stripping quotes, so it renders `Unknown("\"gaps_found\"")`.

This is the WR-03 divergence (two definitions of "status") again, now across the crate instead of across spike
scripts. Source: 27-RESEARCH.md Answer B item 4 and Open Question 5; recorded as 27-CONTEXT D-27.

## Solution direction

Make the dashboard reader accept an optionally quoted value (single or double quotes) for VERIFICATION.md
`status:`, matching `fm_status`. Do NOT switch the driverctl counter to the dashboard reader: the 26.1 fixtures pin
`fm_status`. Add a fixture with a quoted `status: "gaps_found"` to both readers' tests so they cannot diverge again.

Out of scope for Phase 27 (driverctl boundary).
