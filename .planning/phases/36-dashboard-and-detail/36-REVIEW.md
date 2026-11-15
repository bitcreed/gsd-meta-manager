---
phase: 36-dashboard-and-detail
reviewed: 2026-11-15T01:29:13Z
depth: standard
files_reviewed: 15
files_reviewed_list:
  - src/ui/screens/owner.rs
  - src/ui/screens/owner_block.rs
  - src/ui/screens/normal.rs
  - src/ui/screens/detail.rs
  - src/ui/screens/mod.rs
  - src/ui/screens/help.rs
  - src/ui/screens/render_escape_guard.rs
  - src/app.rs
  - src/inbox.rs
  - src/action.rs
  - src/main.rs
  - src/ui/screens/add_project.rs
  - src/ui/screens/delete_confirm.rs
  - src/ui/screens/driver_confirm.rs
  - README.md
findings:
  critical: 0
  warning: 7
  info: 5
  total: 12
  necessity: 0
status: issues_found
---

# Phase 36: Code Review Report

**Reviewed:** 2026-11-15T01:29:13Z
**Depth:** standard
**Files Reviewed:** 15
**Status:** issues_found

## Narrative Findings (AI reviewer)

## Summary

The review covered `git diff 7bfe5d1a..HEAD` for phase 36: the dashboard `You` column, the summary row, the owner attention rank and the `/y` filter (`owner.rs`, `normal.rs`, `mod.rs`, `app.rs`), the digest plumbing (`action.rs`, `main.rs`, `add_project.rs`, `schedule_reparse`), the Owner block on the Phases tab with focus, expansion and cursor (`owner_block.rs`, `detail.rs`), the shared `closes after` wording (`inbox.rs`), and the help and README text.

The pure presentation core is tidy and well tested. The defects sit at the edges, in three places:
- **Data lifecycle.** The digest can describe the wrong project or a moment that has passed (WR-02, WR-06, WR-07).
- **Absent data that reads as clean.** WR-01.
- **The Owner focus level.** It can hand keys to the phase list while the footer still names Owner keys (WR-05).

In `inbox.rs`, `ClauseForm::CountsOnly` breaks its own documented contract (WR-03). Running the digest reader inside the reparse task couples the two: a panic in the reader now also loses the project-state refresh (WR-04).

The necessity rubric (`~/.claude/gsd-core/references/necessity-review.md`) was read. The only candidates were infallible `try_from(u16)` fallbacks. They copy an existing idiom (`detail.rs:1629`), so they were not reported.

## Warnings

### WR-01: A missing or `.planning`-less project shows `·` ("nothing waits on you") instead of an absent value

**File:** `src/app.rs:1036-1048`, `src/ui/screens/mod.rs:1892-1914`, `src/ui/screens/owner.rs:130-148`
**Issue:** `read_owner_digest` has no failure channel. For a project whose folder is gone (`ProjectPresence::FolderMissing`) or that has no `.planning/` (`NoPlanning`), the scanners find nothing and return `OwnerDigest::default()`. That value is stored in `owner_digests` like any other. `you_cell_line` then renders the dim `·`, which the README and help define as "nothing waits on you here". The summary row also counts the project as a visible, loaded row with no work.

This contradicts D-09, which the code cites in `you_cell_line`'s doc: "absent data must not read as clean". The CLI handles the same case correctly: `cli_targets` produces a `Slot::Failed` entry and `inbox` reports an error for it.
**Fix:** Do not store a digest for a project that is not `Present`. Alternatively, have the cell check presence first:
```rust
// normal.rs, You cell
let present = ctx.project_states.get(alias)
    .is_some_and(|s| s.presence == ProjectPresence::Present);
let numbers = present
    .then(|| ctx.owner_digests.get(alias).map(owner::owner_numbers))
    .flatten();
let you_cell = owner::you_cell_line(numbers.as_ref(), YOU_COLUMN_CELLS);
```
Apply the same gate in `owner_totals` (count the row as pending, or exclude it), in `attention_rank_for` and in `owner_work_matches`.

### WR-02: Deleting and re-adding an alias keeps the old project's digest indefinitely

**File:** `src/ui/screens/delete_confirm.rs:170-172`, `src/app.rs:3601-3609`
**Issue:** `do_remove_project` removes `project_states`, `detail_sub_view_per_project` and `last_refresh`, but not `owner_digests` (or the `owner_block` view state). Only `prune_driver_maps` removes the digest, and it runs every 20 ticks (5 s).

Suppose the user removes alias `x` and registers `x` again (at a different path, or after the project changed) before the next prune. The prune then sees a registered alias and keeps the stale entry. The first-fill `OwnerDigestLoaded` that `do_add_project` scheduled is rejected by `!self.ctx.owner_digests.contains_key(&alias)`, because the handler assumes any existing digest is newer. The dashboard, the attention rank, the `/y` filter and the Owner block keep showing the old project's numbers until a file changes under the new path. `do_add_project` schedules no reparse.
**Fix:** In `do_remove_project`, remove the alias from `owner_digests` (and reset `view_cache[alias].owner_block`) at the same time as `project_states`. Do the same in any other unregister path. Also update the "always newer" comment on the `OwnerDigestLoaded` arm, which this case disproves.

### WR-03: `ClauseForm::CountsOnly` still lists every not-built phase

**File:** `src/inbox.rs:514-546`
**Issue:** The `phases` closure maps `ClauseForm::Full | ClauseForm::CountsOnly => phase_list(list)`. `with_phases` skips the lists for lanes, but the `not_built` clause calls `phases(&gate.not_built)` directly. Under `CountsOnly` the clause is therefore `not built ph 26 27 28 …` with every phase, uncapped. This contradicts the enum's doc ("Counts only, no phase lists").

The header ladder in `owner_block::header_line` suffers for it. Rung 3 `(CountsOnly, true)` can be wider than rung 2 `(PhasesCapped(6), true)`, so the ladder is not monotonic, and the not-built clause is dropped at widths where a counted form would have fit. The unit test `closes_after_clauses_forms` asserts only `counts.lanes`, so this went unnoticed.
**Fix:**
```rust
let not_built = (!gate.not_built.is_empty()).then(|| match form {
    ClauseForm::CountsOnly => format!("not built {}", gate.not_built.len()),
    _ => format!("not built ph {}", phases(&gate.not_built)),
});
```
Add an assertion for `counts.not_built` to the test.

### WR-04: A panic in the owner reader now also drops the project-state refresh

**File:** `src/app.rs:1036-1049`
**Issue:** Before this phase, the reparse task ran only `parse_project_state`. It now also runs `read_owner_digest` and sends a single `ProjectStateLoaded` that carries both results. The owner reader parses project-authored files with byte slicing (for example `deferred.rs:66,121,165,544` and `debug.rs:46`). If it panics, the `spawn_blocking` task dies before the send, so the freshly parsed `ProjectState` is lost too: the phase, status and progress columns stop updating for that project.

The codebase already guards scans of untrusted files this way. The agents scan uses `catch_unwind` (`app.rs:2735`, `agents/mod.rs:647`), and so does the task at `app.rs:2562`.
**Fix:** Isolate the digest:
```rust
let digest = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
    state_reader::owner_items::read_owner_digest(&planning_dir, &state, chrono::Utc::now(), stale_days)
}));
// Send the state regardless; on Err either keep the previous digest (make the
// field Option<Box<OwnerDigest>> and skip the insert) or log and send a marker.
```
Wrap `schedule_owner_digest` the same way so that a panic logs instead of leaving the row "loading" forever.

### WR-05: Owner focus hands keys to the phase list while the footer still advertises the Owner keys

**File:** `src/ui/screens/detail.rs:2149-2165`, `src/ui/screens/detail.rs:6463-6467`
**Issue:** When focus is `Owner` and a digest refresh leaves the block without stops, `handle_owner_block_key` sets `focus = Content` and returns `None`. A block with only a non-selectable `Todos` fold, or with every row resolved, has no stops. The same happens when the digest is missing. In each case the key falls through to the main match and runs as a phase-list key.

The previous frame was drawn with `focus == Owner && owner_block_rows > 0`, so the footer showed `[j/k]move [Enter]expand … [Esc]phases`. The user presses `Enter` expecting an expansion and opens the phase-list item instead. Pressing `e` (which the doc says the block swallows "so none of them reaches Content (`e` would enqueue)") reaches the enqueue arm at `detail.rs:6164`.
**Fix:** In the "nothing to focus" branches, reset the focus and consume the key:
```rust
if self.owner_block_rows.get() == 0 || sk.stops.is_empty() {
    self.focus = DetailFocus::Content;
    ctx.needs_redraw = true;
    return Some(ScreenAction::None);
}
```
Do the same for the missing-digest branch. Keep the fall-through only for the explicit digit, `D`, `[`, `]`, `?` and `Tab` arms.

### WR-06: The 500 ms reparse dedup can leave the digest (and so the You cell, rank and filter) stale indefinitely

**File:** `src/app.rs:2847-2856`
**Issue:** `FileChanged` returns early when the previous reparse started less than 500 ms ago, and nothing sends a trailing refresh. The neighboring Archive comment says so itself: "the dedup drops the last write of a burst".

Before this phase that cost only `ProjectState` freshness. The owner digest now rides the same task, and owner files change in bursts. A verify step typically writes `NN-UAT.md`, `STATE.md` and `ROADMAP.md` back to back. If the blocking task reads the UAT file before the later write lands, the `You` count, the summary totals, the attention rank and the `/y` match stay wrong until some unrelated `.planning/` change. The new test `a_watcher_refresh_delivers_a_fresh_owner_digest` steps around this by calling `last_refresh.remove(OBS_ALIAS)` before the second refresh.
**Fix:** Make the dedup trailing-edge. When an event is dropped, mark the alias dirty, and have the tick schedule one reparse for dirty aliases once the 500 ms window has passed:
```rust
if now.duration_since(*last) < Duration::from_millis(500) {
    self.ctx.reparse_pending.insert(alias.clone());
    return;
}
// in the Tick handler: for each pending alias whose window has elapsed -> schedule_reparse
```

### WR-07: Ages and the `stale` flag are frozen at read time

**File:** `src/app.rs:1040-1045`, `src/ui/screens/mod.rs:1904-1909`, `src/ui/screens/owner_block.rs:347-349`, `src/ui/screens/owner_block.rs:811-818`
**Issue:** `read_owner_digest` receives `chrono::Utc::now()` at dispatch and stores `age_days`, `stale` and `hidden.todos_oldest_days` as fixed values. The digest is recomputed only on a `.planning/` file change for that project; there is no periodic reparse. In a dashboard left open for days, a quiet project's Owner block keeps showing `3 d`, keeps `stale` off after `stale_days` has passed, and keeps the Todos fold at `(oldest N d)`. Being "the state of every GSD project at a glance" is the tool's core value, so silently wrong ages matter.
**Fix:** Store absolute timestamps in the digest (or alongside the ages) and derive the days and `stale` at render time from one `now` per frame, as the dashboard already does for workstream claims (`recompute_filtered_aliases` takes one `Utc::now()`). Alternatively, schedule a digest-only refresh for every alias when the UTC day changes.

## Info

### IN-01: The `LauncherMarkerScanned` comment now sits above the `OwnerDigestLoaded` arm

**File:** `src/app.rs:3594-3600`
**Issue:** The new arm was inserted between the "A one-project marker scan landed (30-03, D-13)…" comment and the `LauncherMarkerScanned` arm it describes. As a result, two unrelated comments now head `OwnerDigestLoaded`.
**Fix:** Move lines 3594-3596 below the new arm, directly above `Action::LauncherMarkerScanned`.

### IN-02: The fold-line ladder ignores the 2-cell `> ` cursor prefix

**File:** `src/ui/screens/owner_block.rs:851-858`, `src/ui/screens/owner_block.rs:1187-1206`
**Issue:** `fold_line` picks the widest rung that fits `width`. When the fold line has the cursor, `fold_line_with_cursor` adds `"> "` in front and cuts again to `width`. A rung within 2 cells of the width therefore loses its tail to `…`, which can include the fold under the sub-cursor (for example `Acknowledged (2 fil…`).
**Fix:** Pass `width - 2` to `fold_line` (or pick the rung again inside `fold_line_with_cursor`) whenever the line is drawn with the cursor.

### IN-03: The dashboard and the Owner block disagree on what "old" counts

**File:** `src/ui/screens/owner.rs:67`, `src/ui/screens/owner_block.rs:490-492`
**Issue:** The dashboard `(+N old)` sums archived test, fix and decide, excluding blocked (D-08). The Owner block's `Old debt (N)` and the fold's selectability also add `archived.blocked`. The same project can therefore read `(+2 old)` on the dashboard and `Old debt (6)` in detail. The module doc of `owner.rs` promises that the owner surfaces "cannot disagree".
**Fix:** Either label the detail fold differently, or document the difference in the help's Owner Work section so that the two numbers are not read as one.

### IN-04: The mouse wheel over the Owner block scrolls the phase list and drops Owner focus

**File:** `src/ui/screens/detail.rs:1142-1144`, `src/ui/screens/detail.rs:6583-6600`
**Issue:** `regions.content` is the whole tab content area, which now includes the Owner block. A wheel step over the block resolves to `WheelTarget::Content`, so it sets `focus = Content` and moves the phase-list cursor below. The README now says that the block is keyboard-only, but mentions only clicks.
**Fix:** Record the block's rect in the regions, and return `WheelTarget::Nothing` for it (or route the step to `move_cursor`). Alternatively, extend the README sentence to cover the wheel.

### IN-05: `block_height` can overflow `u16`

**File:** `src/ui/screens/owner_block.rs:1040`
**Issue:** `tab_height * 40` overflows for `tab_height > 1638`. That panics in debug builds and wraps to a wrong cap in release. Such heights are unlikely, but the arithmetic costs nothing to make safe.
**Fix:** `let cap = (usize::from(tab_height) * 40 / 100).max(4);`

---

_Reviewed: 2026-11-15T01:29:13Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
