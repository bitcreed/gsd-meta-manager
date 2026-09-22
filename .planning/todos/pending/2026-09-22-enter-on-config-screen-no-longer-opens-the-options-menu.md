---
created: 2026-09-22T16:52:25.454Z
title: Enter on config screen no longer opens the options menu
area: ui
severity: major
files:
  - src/ui/screens/detail.rs:6044
  - src/ui/screens/detail.rs:6343
  - src/ui/screens/detail.rs:6823
---

## Problem

Regression: pressing Enter on a configuration entry no longer opens the chooser menu
listing the available options (e.g. `ConfigValueKind::Enum` values like mode
interactive/yolo, granularity, model_profile). Nothing appears, so enum settings can't be
changed from the TUI. It used to work.

## Solution

TBD — investigate with /gsd-debug. Bisect recent commits touching
`src/ui/screens/detail.rs` key handling (recent quick tasks 260916-vqz `b` binding and
260917-fko experimental gating both touched screen key dispatch and are prime suspects for
swallowing Enter). Add a regression test pinning Enter → options popup on an Enum row.
