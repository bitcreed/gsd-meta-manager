---
created: 2026-10-27T05:58:00.000Z
title: Add TUI badge for per-repo release maps
area: ui
severity: minor
files:
  - src/state_reader/disk_status.rs:1436-1446
---

## Problem

gsd-core v1.19.0 `/gsd-complete-milestone` can now write `.planning/milestones/<ver>-RELEASE-MAP.json` (upstream `~/projects/node/gsd-core` `src/milestone-release-map.cts:183`). Shape: `{version, entries[{repo, release, reference, environment, status, tag}]}`. MILESTONES.md also gets a "Release mapping:" table. The file is only written for multi-repo projects with `git.repo_tagging: per_repo`.

Today the app scans only subdirectories of `milestones/` (`src/state_reader/disk_status.rs:1436-1446`) and never reads this file.

Deferred from the gsd-core 1.19.0 sync, where it was area 8 (optional). Relates to the `git.repo_tagging` Defaults row added in that sync.

## Solution

Add a state_reader parser for `<ver>-RELEASE-MAP.json`, tolerant of a missing file and of unknown fields. Add a per-milestone badge or row in the detail UI showing per-repo tag/status.
