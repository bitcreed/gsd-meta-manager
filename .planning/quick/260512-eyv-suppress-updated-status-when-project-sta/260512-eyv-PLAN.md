---
quick_id: 260512-eyv
description: Suppress "Updated" status when project state is unchanged
date: 2026-05-12
mode: quick
---

# Quick Task 260512-eyv — Suppress No-op "Updated" Status

## Problem

The TUI flashes `Updated: <project>` every time the file watcher fires for a registered project, even when nothing the meta-manager actually displays has changed (e.g. unrelated `.planning/` writes touching files the state reader doesn't surface). This creates noise for projects with live activity.

## Fix

Compare the freshly-parsed `ProjectState` to the previously-cached one. Only set the `status_message` and `recompute_filtered_aliases()` if they differ. The `change_tracker` call stays unconditional — it's already a no-op when nothing meaningful changed.

## Tasks

### Task 1 — Derive `PartialEq` on `ProjectState` and its sub-types

**Files:**
- `src/state_reader/mod.rs` — `ProjectState`
- `src/state_reader/roadmap_md.rs` — `RoadmapPhase`
- `src/state_reader/queue_md.rs` — `QueuedAction`
- `src/state_reader/disk_status.rs` — `DiskInference`

All fields are already `PartialEq` (String, primitives, Vec, HashMap, Option, enums). No exotic types. Just add `PartialEq` to each `#[derive(...)]`.

**Verify:** `cargo check` passes.

### Task 2 — Skip "Updated" status on no-op state changes

**File:** `src/app.rs` — `Action::ProjectStateLoaded` handler.

Change the handler so:
1. The change-tracker call still runs unconditionally.
2. The state insert always happens (so any cosmetic-only fields the UI still might read pick up the latest snapshot).
3. `status_message` + `recompute_filtered_aliases` + `needs_redraw` only fire when `old_state != new_state`.

If there was no prior state (first load), treat as "changed" so the message fires once.

**Verify:**
- `cargo build` clean.
- `cargo test` passes (existing 111 tests).
- Manual: hammering `.planning/STATE.md` with `touch` should not produce an `Updated:` flash if the parsed content is identical.

## Out of Scope

- Suppressing the file-watcher event itself (it's already debounced and useful for waking up downstream caches).
- Reworking `ChangeTracker` semantics — the in-memory event log keeps its current scope.
