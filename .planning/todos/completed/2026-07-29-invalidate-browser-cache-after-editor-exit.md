---
created: 2026-07-29T00:00:00.000Z
title: Invalidate browser file cache after $EDITOR exits
area: ui
severity: minor
files:
  - src/main.rs
---

## Problem

After the `$EDITOR` shell-out returns, `main.rs`'s resume path does not invalidate
`cache.browser_file_content`. The markdown viewer therefore redisplays the *pre-edit*
text — the user's own edit appears not to have taken effect until the cache happens to
be refreshed by some other event.

Found by the Phase 14 code review as WR-01/WR-03's sibling (WR-03) and confirmed unfixed
by direct source read during Phase 14 re-verification. It was explicitly outside plan
14-04's scope fence, so it was carried forward rather than silently absorbed.

## Solution

Invalidate `cache.browser_file_content` (or re-read the file) on the `$EDITOR`
suspend/resume return path in `main.rs`, before the viewer redraws.

Note: no test in this repo drives a real terminal or the suspend/resume process
boundary, so this needs either a seam that can be tested without a PTY, or an explicit
manual UAT step.
