---
created: 2026-09-15T16:27:52.421Z
title: Sync gsd-core config and expose new options
area: config
severity: major
files:
  - src/config.rs
  - src/state_reader/config_json.rs
  - ~/projects/node/gsd-core (sibling repo, source of truth)
---

## Problem

gsd-meta-manager's understanding of GSD's config schema (what it reads, displays,
and lets users edit) has drifted from the latest gsd-core code at
`~/projects/node/gsd-core`. There's no record of which gsd-core version/tag
gsd-meta-manager was last synced against, so each future sync starts from
scratch with no baseline to diff from. Concretely, newer config options —
e.g. `workflow.compact_content` — may exist in gsd-core but not be reachable
anywhere in gsd-meta-manager's config UI/handling, silently hiding them from
users. Additionally, whatever in-app documentation/help text gsd-meta-manager
shows for each config option doesn't currently note which gsd-core version
introduced support for that option, which matters for users running older
gsd-core versions.

## Solution

TBD — sub-requirements to cover:

1. Diff gsd-meta-manager's config schema/handling against the current
   `~/projects/node/gsd-core` (sibling repo, not this repo) to find drift.
2. Record the gsd-core tag/version this sync was performed against somewhere
   durable — e.g. a comment/constant in the synced config source file
   (`src/config.rs` or `src/state_reader/config_json.rs`) or a tracked marker
   file — not just in this todo, so the *next* sync has a known starting
   point.
3. Ensure newer config options (e.g. `workflow.compact_content`) are actually
   reachable/exposed in gsd-meta-manager's config surface, not silently
   missing.
4. When writing/updating in-app documentation or help text for each config
   option, annotate which gsd-core version introduced support for that
   option.
