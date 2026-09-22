---
created: 2026-09-22T16:52:25.454Z
title: Show registered projects whose folder is missing in red
area: ui
severity: minor
files:
  - src/registry.rs:403
  - src/ui/screens/normal.rs
---

## Problem

When a registered project's directory has been moved or deleted, the dashboard gives no
clear signal. `add_project` checks `path.exists()` only at registration time
(src/registry.rs:403); afterwards a stale entry just sits in the list, and the user has
no obvious cue that it needs removing or re-pointing.

## Solution

On load/refresh, check each registered path (and ideally its `.planning/` dir); render
entries whose folder no longer exists in a red font in the project list so the user can
spot and remove them. Consider a short "(missing)" suffix too, so the state is readable
without color. Existing delete flow (`src/ui/screens/delete_confirm.rs`) is the removal path.
