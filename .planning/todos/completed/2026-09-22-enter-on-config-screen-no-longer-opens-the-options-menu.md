---
created: 2026-09-22T16:52:25.454Z
title: Enter on config screen no longer opens the options menu
area: ui
severity: major
completed: 2026-09-22
resolved_by: debug session enter-unset-config-row (72b5e03 RED, 0581c70 fix)
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

Resolved — see `.planning/debug/resolved/enter-unset-config-row.md`. Enter was dead only on
rows showing `(unset)`: the layered row builders collapsed an unset key's kind to
`ConfigValueKind::Null`, so the Enter arm had no chooser to open. Set rows (`mode`,
`granularity`, `model_profile`) always worked; the 260916-vqz / 260917-fko suspects were
disproved. Fixed with `ConfigValueKind::Unset(Box<kind>)` + `kind.editable()` on every
edit path; pinned by `every_unset_choice_row_opens_a_chooser_whose_options_all_apply`.
