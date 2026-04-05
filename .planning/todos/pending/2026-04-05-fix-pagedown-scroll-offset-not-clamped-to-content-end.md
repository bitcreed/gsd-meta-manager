---
created: 2026-04-05T08:50:00.000Z
title: Fix PageDown scroll offset not clamped to content end
area: ui
files: []
---

## Problem

When viewing a markdown file, pressing PageDown past the end of content continues incrementing the scroll offset beyond the actual content length. The screen visually stops scrolling (nothing more to show), but the internal offset keeps growing. When the user then presses PageUp, nothing happens for several presses because the offset is decreasing from a value well past the content end back toward visible range.

Example: if content is 50 lines and viewport is 20 lines, max scroll should be ~30. But pressing PageDown 4x past the end pushes offset to e.g. 110. User must then press PageUp 4x just to get back to line 30 before visible scrolling resumes.

## Solution

Clamp the scroll offset on PageDown so it never exceeds `total_lines - visible_height` (same max_scroll calculation already used in the rendering code). The offset should be clamped at the point where the last line of content is at the bottom of the viewport.
