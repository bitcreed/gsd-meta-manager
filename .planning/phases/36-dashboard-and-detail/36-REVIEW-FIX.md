---
phase: 36-dashboard-and-detail
fixed_at: 2026-11-15T02:01:23Z
review_path: .planning/phases/36-dashboard-and-detail/36-REVIEW.md
iteration: 1
findings_in_scope: 12
fixed: 12
skipped: 0
status: all_fixed
---

# Phase 36: Code Review Fix Report

**Fixed at:** 2026-11-15T02:01:23Z
**Source review:** .planning/phases/36-dashboard-and-detail/36-REVIEW.md
**Iteration:** 1

**Summary:**
- Findings in scope: 12 (7 warnings, 5 info)
- Fixed: 12
- Skipped: 0

I checked each claim against the source before fixing it, and all 12 held. Each fixed
warning has a regression test with synthetic fixtures only. WR-01, WR-03, WR-05 and WR-06
were also shown to fail without the fix: either the test was run against the old code, or
it asserts the exact value the old code produced. WR-01, WR-04, WR-06 and WR-07 change
logic, so they are marked **fixed: requires human verification**.

## Verification

Run in the existing checkout `/home/blk/projects/rust/gsd-meta-manager-owner-inbox` (branch
`feat/v2.5.0-owner-inbox`), not in a separate review-fix worktree, after the last commit:

- `rtk bypass cargo test --locked --all-targets`: 64 test binaries, 4261 passed, 0 failed,
  19 ignored.
- `rtk bypass cargo clippy --locked --all-targets -- -D warnings`: 0 warnings, 0 errors.
- `cargo fmt --check`: clean.

## Fixed Issues

### WR-01: A missing or `.planning`-less project shows `·` ("nothing waits on you") instead of an absent value

**Status:** fixed: requires human verification
**Files modified:** `src/app.rs`, `src/ui/screens/mod.rs`, `src/ui/screens/normal.rs`,
`src/ui/screens/detail.rs`, `src/ui/screens/help.rs`, `README.md`
**Commits:** 983cd80c, 6fbf8ddd (docs)
**Applied fix:**
- The store now refuses a digest for such a project. Both `ProjectStateLoaded` and
  `OwnerDigestLoaded` go through `AppContext::owner_digest_storable`, and a reparse that
  finds the folder or `.planning/` gone removes the old digest. `schedule_owner_digest`
  skips such projects.
- Reads are gated too. Every surface (the cell, the rank, `/y` and the Owner block) reads
  through `AppContext::owner_digest`, which returns `None` for a project whose folder or
  `.planning/` is gone.
- The `You:` summary leaves such rows out through `summary_owner_rows`. Counting them as
  pending would have read `1 loading` forever. This is an inferred choice: D-11 defines
  N as the visible rows, and N now excludes rows with no owner state.
- The fixture fleet's `cedar-web` is no longer `FolderMissing`, because such a row can no
  longer show lane text. The red row is still covered by `dune-app`, which has a blocked
  status.
- The help and README now define the blank cell as "no owner data: not loaded yet, or the
  folder or its `.planning/` is gone".
- Tests: `a_project_without_planning_holds_no_owner_digest` (app) and
  `a_missing_project_shows_no_owner_value` (normal).

### WR-02: Deleting and re-adding an alias keeps the old project's digest indefinitely

**Files modified:** `src/ui/screens/delete_confirm.rs`, `src/app.rs`, `src/ui/screens/mod.rs`
**Commit:** eb5d773f
**Applied fix:**
- `do_remove_project` removes `owner_digests[alias]` and resets
  `view_cache[alias].owner_block`.
- The first-fill comment and the field docs now state the invariant the handler relies
  on.
- There is no other unregister path in the TUI. `main.rs:282` is the CLI.
- Test: `unregistering_a_project_drops_its_owner_digest_and_block_state`.

### WR-03: `ClauseForm::CountsOnly` still lists every not-built phase

**Files modified:** `src/inbox.rs`, `src/ui/screens/owner_block.rs`
**Commit:** 87b35a3f
**Applied fix:**
- Under `CountsOnly` the not-built clause now reads `not built N`.
- `closes_after_clauses_forms` now asserts `counts.not_built`, and checks that the rung is
  never wider than the capped one.
- The new test `the_counts_rung_counts_the_not_built_phases_too` checks the header ladder
  with 14 unbuilt phases.
- The existing `the_header_walks_the_five_rungs` encoded the old wording (`not built ph
  27` on the counts rung). It now expects `not built 1`.

### WR-04: A panic in the owner reader now also drops the project-state refresh

**Status:** fixed: requires human verification
**Files modified:** `src/app.rs`, `src/action.rs`, `src/ui/screens/mod.rs`
**Commit:** c0ccb9a3
**Applied fix:**
- `AppContext::guarded_owner_digest` runs the reader behind `catch_unwind`, the same
  pattern as the agents scan, and logs a panic. Both dispatch sites use it.
- `ProjectStateLoaded.digest` is now `Option<Box<OwnerDigest>>`. On `None` the state
  still lands and the old digest is removed (the numbers are unknown, so it is not kept
  as a stale moment).
- A first-fill panic logs and sends nothing, so the row reads as not loaded, never as
  clean. Leaving the row out of the "loading" count would need a new failure map on
  `AppContext`, which was not added.
- Test: `an_owner_reader_panic_still_delivers_the_project_state`.

### WR-05: Owner focus hands keys to the phase list while the footer still advertises the Owner keys

**Files modified:** `src/ui/screens/detail.rs`
**Commit:** 04f8c18f
**Applied fix:**
- When there is nothing to focus (missing digest, no stops, or a zero-row block), focus
  resets to Content and the key is consumed (`Some(ScreenAction::None)`).
- The digit, `D`, `[`, `]`, `?` and `Tab` arms still fall through.
- Test: `a_block_left_without_stops_consumes_the_next_key` covers a Todos-only digest and
  a missing digest, with `Enter`, `e` and `Esc`. Before the fix, `Enter` moved focus to
  the Waves pane.

### WR-06: The 500 ms reparse dedup can leave the digest (and so the You cell, rank and filter) stale indefinitely

**Status:** fixed: requires human verification
**Files modified:** `src/app.rs`
**Commit:** 193a5935
**Applied fix:**
- A `Planning` change dropped by the dedup now marks the alias in `App::reparse_pending`.
- `flush_pending_reparses` runs on every tick. Once the alias's 500 ms window has passed,
  it re-parses the alias through `schedule_reparse`, which stays the only dispatch site.
- The result is one trailing re-parse per burst, with no new timer. OBS-06 is unaffected,
  because journal and driver-run events never mark an alias.
- Test: `a_write_inside_the_reparse_window_is_reparsed_on_a_later_tick`. It flips a UAT
  row inside the window and asserts that the next tick past the window delivers the
  lower count, and that only one re-parse is dispatched.

### WR-07: Ages and the `stale` flag are frozen at read time

**Status:** fixed: requires human verification
**Files modified:** `src/app.rs`
**Commit:** 8b2a8021
**Applied fix:**
- `App::sweep_owner_ages` runs on the existing 20-tick counter. Once per UTC hour it
  re-parses, through `schedule_reparse`, every project whose digest has groups or todos.
  The first call only records the hour.
- **Design note [inferred]:** this relaxes 36 D-03 ("no timer-driven recompute") for ages
  only. It was chosen over render-time derivation for three reasons:
  - Exact render-time ages would need the reader (Phase 34) to store per-group source
    timestamps.
  - Deriving from the stored whole days plus the elapsed time is off by up to a day,
    which is no better than a daily refresh.
  - Going through `schedule_reparse` keeps sweep results ordered like watcher re-parses.
    A digest-only refresh could overwrite a newer re-parse.
- Cost: a warm re-parse costs a `stat` per owner file plus `parse_project_state`, at most
  once an hour per project that shows an age.
- The planning docs (CONTEXT D-03) were not edited. Please confirm or revise D-03.
- Test: `the_owner_age_sweep_re_parses_aged_digests_once_an_hour`.

### IN-01: The `LauncherMarkerScanned` comment now sits above the `OwnerDigestLoaded` arm

**Files modified:** `src/app.rs`
**Commit:** 5c8a5b97
**Applied fix:** Moved the comment back above `Action::LauncherMarkerScanned`.

### IN-02: The fold-line ladder ignores the 2-cell `> ` cursor prefix

**Files modified:** `src/ui/screens/owner_block.rs`
**Commit:** 5588f2f1
**Applied fix:**
- `BlockModel` gains `cursor_fold_line`, the fold line and its marks picked for
  `width - 2`. `fold_line_with_cursor` draws that version, so a rung that fits exactly is
  no longer cut by the gutter.
- Test: `the_cursor_fold_line_picks_its_rung_for_the_gutter`.

### IN-03: The dashboard and the Owner block disagree on what "old" counts

**Files modified:** `src/ui/screens/help.rs`, `README.md`, `src/ui/screens/owner.rs`
**Commit:** ce6d8feb
**Applied fix:**
- Fixed by documenting the difference. The difference itself is agreed design (UI-SPEC:
  "differs from the dashboard's `(+N old)`, which excludes blocked, by design").
- The help row, the README table and the `owner.rs` module doc now say that the detail
  Old debt fold also counts archived blocked items.

### IN-04: The mouse wheel over the Owner block scrolls the phase list and drops Owner focus

**Files modified:** `src/ui/screens/detail.rs`, `README.md`
**Commit:** 57a8a5a6
**Applied fix:**
- `DetailRegions` records the block's rect each frame (`owner_block`, reset in
  `reset_regions`), and `wheel_target` returns `Nothing` inside it.
- The README's keyboard-only sentence now covers the wheel.
- Clicks are unchanged.
- Test: `the_wheel_over_the_owner_block_scrolls_nothing`.

### IN-05: `block_height` can overflow `u16`

**Files modified:** `src/ui/screens/owner_block.rs`
**Commit:** 75aff7ef
**Applied fix:**
- The height cap is now computed in `usize`.
- `the_height_follows_content_and_tab` now asserts the result at heights 1639 and
  `u16::MAX`. The old code panics at those heights in debug builds.

---

_Fixed: 2026-11-15T02:01:23Z_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
