---
created: 2026-10-02T00:00:00.000Z
title: Config view must not show global defaults as effective values
area: ui
severity: major
files:
  - src/ui/screens/detail.rs:10713-10760
  - src/ui/screens/detail.rs:3104
  - src/ui/screens/detail.rs:4932
  - src/state_reader/config_json.rs
---

## Problem

GSD does NOT fall back to the global `~/.gsd/defaults.json` when a key is unset in
a project's `.planning/config.json`. It most likely falls back to GSD's built-in
defaults instead. Global defaults only seed the config of *new* projects at
creation time.

The Config/Defaults tab layers project over global (`opt_bool_layered`,
`opt_str_layered`, `opt_secret_layered`, ... with `from_defaults: true`) and shows
the inherited global value as if it were what GSD will use. For an unset project
key that is misleading: the displayed value can differ from GSD's real behaviour.

Related: the global settings editor todo
(2026-09-23-add-a-global-settings-editor-with-unambiguous-scope) — scope labelling
there should say "applies to new projects only".

## Solution

TBD — verify against gsd-core's config loader first (do not trust this note's
"most likely"; confirm the real fallback chain and per-key built-in defaults).
Then: for an unset project key show the GSD built-in default (marked as such,
e.g. "(GSD default)"), not the global value; relabel the global defaults editor
as "template for new projects"; drop or reword the `from_defaults` "inherited"
affordance.
