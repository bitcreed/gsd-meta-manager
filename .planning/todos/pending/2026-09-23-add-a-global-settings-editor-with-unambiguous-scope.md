---
created: 2026-09-23T06:11:17.602Z
title: Add a global settings editor with unambiguous scope
area: ui
severity: minor
files:
  - src/ui/screens/detail.rs:1314
  - src/ui/screens/detail.rs:2845-2871
  - src/ui/screens/detail.rs:5173
  - src/ui/screens/detail.rs:6453
  - src/ui/screens/mod.rs:343-348
  - src/ui/screens/normal.rs
  - src/state_reader/config_json.rs:492-506
  - src/config.rs:313
  - src/config.rs:382-387
---

## Problem

User request, verbatim:

> Add an option to change options globally - investigate whether it's better
> to have a config screen from the main menu (project overview) or from a
> specific project's config editor or both - best UX to be targetted. No
> confused users wanted. If on project, it has to be clear to the user when
> he's editing per-project or global settings.

This is an **investigation plus implementation** item: decide where global
settings are edited (project overview, the per-project config editor, or both),
then build it.

**UX requirement (hard):** when a user edits settings from inside a project,
the UI must make it unambiguous whether a change lands in per-project or global
scope. That means scope visible at all times while editing, not only in a
transient status message.

Codebase context today:

- **Per-project config editor**: the Config/Defaults tab in
  `src/ui/screens/detail.rs`. It loads `.planning/config.json` plus
  `~/.gsd/defaults.json` at line 1314. A hidden `d` key (lines 2845-2871)
  toggles `DefaultsEditTarget::{Project, Global}` (enum in
  `src/ui/screens/mod.rs:343`). The scope shows up only in the block title
  (line 5173, " Global Defaults (~/.gsd/defaults.json) ") and in a one-shot
  status message ("Editing ~/.gsd/defaults.json"). So global editing already
  exists, but only from inside a project and behind a key that is easy to miss.
- **Global config sources**:
  1. GSD's own global defaults, `~/.gsd/defaults.json`, loaded by
     `load_user_defaults()` in `src/state_reader/config_json.rs:492-506`.
  2. The manager's own settings, `~/.config/gsd-meta-manager/config.json`
     (JSON, not TOML; `Config::default_path`, `src/config.rs:382`), with
     `Preferences` at `src/config.rs:313` (for example `default_runtime` and
     `driver_max_concurrent`). The TUI has no editor for these yet.
- **Precedence**: for GSD keys, the project's `.planning/config.json` wins and
  unset fields fall back to `~/.gsd/defaults.json`. This is the layered
  `opt_*_layered` accessors around `detail.rs:6555`, and rows record
  `from_defaults` (line 6453) when a value is inherited. For manager keys, the
  registry entry wins over `Preferences`, which wins over the built-in default
  (for example runtime: entry > `default_runtime` > Claude).
- The project overview (`src/ui/screens/normal.rs`) has no settings entry point
  today.

## Solution

TBD. Start with the investigation, then implement the chosen design.

- Compare three entry points: a global settings screen reached from the
  overview; the in-project editor with an explicit scope switch; or both.
  [INFERRED] Offering both is probably right: overview → global only, project →
  project scope by default with a clearly labelled switch to global.
- Scope must be persistent and visible in the project editor, for example a
  labelled `Project | Global` tab or header, colour-coded, with the target file
  path shown. Inherited values should be marked with their source. Consider
  confirming when a global write will affect other projects.
- Decide whether the global screen also exposes manager `Preferences`, kept
  visually separate from GSD's `~/.gsd/defaults.json`. This overlaps with item 8
  (runtime picker) of `2026-09-22-codex-runtime-remainder-after-mvp.md`.

## Audit

- [INFERRED] Severity is `minor`: a feature and UX request with a working
  (hidden) `d` toggle as the workaround. The severity confirmation gate was
  skipped because the human was unavailable.
- [INFERRED] No duplicate: no pending todo covers global settings UX. The
  completed config-screen todos (sync-gsd-core-config, enter-on-config-screen,
  filter-config-screen) are different in scope. The codex-remainder item 8
  (runtime picker) is related, not a duplicate.
