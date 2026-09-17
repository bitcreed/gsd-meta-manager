---
created: 2026-09-15T16:37:10.776Z
title: Backlog tab shows empty despite non-zero count on overview
area: ui
severity: major
files:
  - src/state_reader/mod.rs:598-615
  - src/state_reader/backlog.rs:34-44
  - src/state_reader/backlog.rs:49-98
  - src/ui/screens/detail.rs:1019-1029
---

## Problem

The main overview screen shows a non-zero "Backlog" count (e.g. 4) for a
project, but that same project's detail view, tab `3:Backlog` (`DetailSubView
::Backlog`, tab index 2 — index mapping itself is correct, see `detail.rs
:534-565`), renders empty. The count and the tab content clearly disagree
about how many backlog items exist for the same project.

**Hypothesis (not confirmed — capture only, no fix applied):** the two code
paths that produce these two numbers use different, inconsistent matching
rules against `.planning/phases/`:

- The overview's count comes from `count_backlog_items`
  (`src/state_reader/mod.rs:599-614`), which counts *any directory* whose
  name merely `starts_with("999")` — a loose prefix match.
- The detail tab's content comes from `parse_backlog_items`
  (`src/state_reader/backlog.rs:49-98`), which is stricter on two counts:
  1. `parse_backlog_dir_name` (`backlog.rs:34-44`) requires the name to match
     `"999.<N>-<slug>"` exactly (a literal dot after `999` and a dash after
     the number) — a bare `"999-something"` directory would satisfy the
     overview's `starts_with("999")` but fail this parse and be silently
     dropped.
  2. Even directories that parse correctly are filtered out entirely if they
     have no `.md` file inside (`backlog.rs:70`, "Skip directories with no
     .md files (empty backlog placeholders)").

If this project's `999.*` phase directories are missing a `.md` file, or are
named without the `999.N-` dot/dash pattern the strict parser expects, the
overview would still count them (loose match) while the detail tab would
filter every single one out (strict match + content requirement) — producing
exactly this symptom: overview shows 4, detail view shows 0.

`switch_to_tab`'s Backlog branch (`detail.rs:1019-1029`) then caches
`cache.backlog_items` from that (possibly wrongly-empty) `parse_backlog_items`
result, so once loaded it stays empty for the session.

## Solution

TBD — needs confirmation against this project's actual `.planning/phases/
999.*` directory contents/naming before deciding the fix. Likely candidates:
reconcile `count_backlog_items` and `parse_backlog_items` to use the same
matching rule (either loosen the parser or tighten the counter), and/or
relax/report the "no .md file" skip in `parse_backlog_items` so a
miscounted-vs-empty distinction is visible to the user instead of silently
rendering an empty list.
