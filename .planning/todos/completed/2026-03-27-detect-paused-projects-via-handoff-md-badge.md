---
created: 2026-03-27T20:23:29.151Z
title: Detect paused projects via HANDOFF.md badge
area: ui
resolves_phase: 14
files:
  - src/state_reader/mod.rs
  - src/ui/screens/normal.rs
---

## Problem

Projects with a `.planning/HANDOFF.md` file have paused work (created by `/gsd:pause-work`, resumed with `/gsd:resume-work`). The dashboard should detect this and show a `||` (pause) badge next to the project name, similar to how `▶` shows active Claude sessions.

## Solution

1. In `parse_project_state()` (or a new helper), check if `.planning/HANDOFF.md` exists in the project directory
2. Add a `has_handoff: bool` field to `ProjectState`
3. In `NormalScreen` dashboard row rendering, check `state.has_handoff` and prepend a `||` badge (dim yellow or similar) to the alias cell
4. Badge should be shown alongside (not replacing) the `▶` session indicator if both are true
