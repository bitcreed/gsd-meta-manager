---
created: 2026-04-05T08:32:00.000Z
title: Fix folder appears empty after returning from markdown view
area: ui
files: []
---

## Problem

When navigating into a folder in the archive/file browser (e.g. `Archive > v1.0`), the folder contents display correctly with the list of markdown files. However, after selecting and viewing a markdown file, pressing `ESC` to return shows the folder as empty — no files listed, just the folder header.

Pressing `ESC` a second time navigates up one level and re-entering the folder shows the files again. This suggests the state restoration on pop from markdown view is broken — either the file list isn't being repopulated, or the view is returning to a stale/empty state instead of the previously populated folder view.

**Steps to reproduce:**
1. Navigate into a folder (e.g. `Archive > v1.0`)
2. See files listed correctly (v1.0-MILESTONE-AUDIT.md, v1.0-REQUIREMENTS.md, v1.0-ROADMAP.md)
3. Select and view a markdown file
4. Press ESC to go back
5. Folder appears empty — only header shown
6. Press ESC again to go up, then re-enter folder — files appear again

## Solution

Debug the navigation stack / state restoration when popping from markdown detail view back to folder view. Likely the folder's file listing state needs to be preserved or refreshed when returning from a child view.
