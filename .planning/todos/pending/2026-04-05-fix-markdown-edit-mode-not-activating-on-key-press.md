---
created: 2026-04-05T08:31:19.586Z
title: Fix markdown edit mode not activating on key press
area: ui
files: []
---

## Problem

Pressing "e" on the markdown detail view does nothing — the expected behavior is to enter edit mode for the markdown content, but there is no response to the key press. The only working key is `esc` to return to the previous screen.

This suggests either:
- The "e" key binding is not registered in the detail/markdown view's input handler
- The edit mode transition logic is broken or guarded by a condition that's never met
- The edit mode feature was planned but not yet implemented

## Solution

1. Check the key event handler for the markdown/detail view screen
2. Verify "e" is mapped and the handler dispatches correctly
3. If edit mode exists, debug why the transition fails
4. If edit mode is not yet implemented, mark as a feature gap and implement the textarea-based editing flow
