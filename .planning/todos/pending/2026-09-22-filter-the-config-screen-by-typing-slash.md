---
created: 2026-09-22T16:52:25.454Z
title: Filter the config screen by typing slash
area: ui
severity: minor
files:
  - src/ui/screens/detail.rs:5065
  - src/ui/screens/detail.rs:6823
---

## Problem

The Config Settings screen lists many keys across categories (mode, granularity,
model_profile, workflow toggles, …). Finding one means scrolling the whole list; there is
no search.

## Solution

Press `/` on the config screen to open a filter input (vim/less-style search); typing
narrows the visible rows to keys (and possibly help text) matching the query, Esc clears
the filter, Enter/arrows act on the filtered selection. Reuse any existing filter/search
input pattern in the codebase if one exists.
