---
created: 2026-04-05T08:26:16.213Z
title: Fix leading blank in DRPEV status display
area: ui
files: []
---

## Problem

The DRPEV workflow status indicator in the project list table starts with a leading blank space: `' D R P E V'` instead of `'D R P E V'`. This causes uneven column starting points in the Status column, misaligning it relative to other status values like "executing", "verifying", or "v1.0 Complete".

Example of current output:
```
│  ⏸ cdr-configurator              P3: Unknown                        D  R  P  E  V     2/5 phases          1                 │
```

The space before `D` shifts the entire status string one character to the right compared to where it should start.

## Solution

Find where the DRPEV status string is constructed (likely in the project list rendering or status formatting code) and remove the leading space so the string starts directly with `D`. Ensure all status values in the column share the same starting position.
