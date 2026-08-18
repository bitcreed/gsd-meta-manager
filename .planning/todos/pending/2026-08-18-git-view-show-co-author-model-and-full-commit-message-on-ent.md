---
created: 2026-08-18T21:29:21.620Z
title: Git view — show co-author model and full commit message on Enter
area: ui
severity: minor
files:
  - src/state_reader/git_ops.rs:524-530
  - src/state_reader/git_ops.rs:540-584
  - src/state_reader/git_ops.rs:586-610
  - src/ui/screens/detail.rs:2964-2979
  - src/ui/screens/detail.rs:2993-3030
  - src/ui/screens/detail.rs:1495-1515
  - src/ui/screens/mod.rs:432-435
---

## Problem

Two gaps in the `4: Git` tab of the project detail screen.

**1. The row shows the human author but not the model that did the work.**
`GitLogEntry` (`git_ops.rs:524-530`) carries only `hash`, `date`, `author`, `message`, and
`load_git_log` fetches `--format=%h\x1f%ad\x1f%an\x1f%s` (`git_ops.rs:551`). `%s` is the
subject only, so the `Co-Authored-By:` trailer in the body is never read. The row renders as
`hash -- date -- subject  author` (`detail.rs:2964-2979`).

In this repo essentially every commit is co-authored by a model — the project's own commit
convention appends `Co-Authored-By: Claude Opus 5 <noreply@anthropic.com>` — so "who actually
wrote this" is information the view has access to and drops.

**2. Enter shows the touched files but not what the commit says.**
Enter loads only `load_diff_stat` (`detail.rs:1502` → `git_ops.rs:586`, a
`git diff-tree --stat`), rendered in a 40% bottom pane (`detail.rs:2993-3030`). For a repo
whose commit bodies carry the actual reasoning — deviations, blind spots, why a thing was
declined — the body is the more valuable half and is unreachable from the TUI.

## Solution

**Model attribution.** Extend the log format to pull the trailer, e.g. add a fifth
`\x1f`-separated field using
`%(trailers:key=Co-authored-by,valueonly,separator=%x2c)` and bump the `splitn(4, …)` at
`git_ops.rs:571` to 5 (guard the arity check — it currently drops any line that isn't exactly
4 parts, so a partial rollout would silently show an empty log). Add `model: Option<String>`
to `GitLogEntry`; parse the display name before the `<` and discard the address, which is
irrelevant. Render it after the author in `detail.rs:2964-2979`, dimmed like the author is.

Points to settle while implementing:
- A commit may carry several `Co-authored-by` trailers, or none. Decide whether to show the
  first, all, or only ones matching a known-model pattern — and make an absent trailer render
  as nothing rather than an empty column artifact.
- Trailer parsing is case-insensitive in git (`Co-Authored-By` vs `Co-authored-by`);
  `%(trailers:key=…)` already matches case-insensitively, so don't re-implement it by hand.
- Width: the row is already `hash -- date -- subject  author`. Adding a model risks crowding
  at narrow terminals — consider truncating the subject before the attribution columns, since
  the full message is about to become reachable via Enter anyway.

**Enter detail.** Add a full-message read (`git show -s --format=%B <hash>`, or extend the
existing diff-stat call site at `detail.rs:1495-1515` so one spawn fetches both) and render
the commit message **first**, with the touched-file list below it — the current diff-stat
output becomes the lower half rather than the whole pane. Widen the pane beyond the present
40%, and make it scrollable: this repo's commit bodies routinely run past a screen.

Keep the existing async pattern — the load is a `tokio::spawn` sending an `Action` back
(`GitDiffStatLoaded` at `app.rs:1133`); add a sibling action or extend that payload rather
than doing a blocking git read on the render path.
