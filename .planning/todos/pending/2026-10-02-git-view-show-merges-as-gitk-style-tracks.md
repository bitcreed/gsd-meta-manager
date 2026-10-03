---
created: 2026-10-02T00:00:00.000Z
title: Git view should draw merges as gitk-style tracks
area: ui
severity: cosmetic
files:
  - src/state_reader/git_ops.rs:809-850
  - src/ui/screens/detail.rs
---

## Problem

The 4:Git overview lists commits as a flat, linear log, so merges are invisible:
you cannot see which commits belong to a side branch or where it joined back.
`load_git_log` (`src/state_reader/git_ops.rs`) runs a plain `git log` with no
parent/topology information.

## Solution

TBD. Render a gitk-style graph column: separate tracks (lanes) for parallel
lines of history that stay apart until the merge commit joins them. Likely
approach: add `%P` (parents) to the `--format` (keep the subject LAST per the
existing field-order invariant) and optionally `--topo-order`, then compute lane
assignment in Rust and draw the lane glyphs in a leading column of the log table.
Mind display-width alignment and the third-party-subject sanitising rules
(`.shown()`); the `planning_only` path filter changes parent rewriting, so check
that lanes still make sense there.
