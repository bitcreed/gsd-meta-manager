---
phase: 37-inbox-screen-and-actions
plan: 04
subsystem: Owner inbox layout, loading/unreadable states, live refresh
tags: [inbox-screen, layout-ladders, lane-filters, owner-digest-failed, sc3]
requires: ["37-03"]
provides:
  - Action::OwnerDigestFailed and AppContext.owner_digest_failed
  - DigestState (Loaded / Loading / Unreadable) and digest_state
  - reanchor (cursor identity fall-through after a digest update)
  - LaneFilter (1-4, a), InboxViewState.filter
  - grid_for_inbox with the FIX Command column
  - header, fold-line and footer ladders, empty and loading copy
affects: [37-05, 37-06, 37-07]
key-files:
  modified:
    - src/action.rs
    - src/app.rs
    - src/ui/screens/mod.rs
    - src/ui/screens/inbox.rs
    - src/ui/screens/owner_block.rs
    - src/ui/screens/delete_confirm.rs
    - src/ui/screens/detail.rs
    - src/ui/screens/driver_confirm.rs
    - src/ui/screens/normal.rs
key-decisions:
  - "Directive 2: absent data is loading (still expected) or unreadable (will not come); no timer. A reader panic in the first fill sends OwnerDigestFailed; a reparse with digest None marks the alias too; any later digest clears it; prune_driver_maps drops unregistered aliases"
  - "D-10: App::update calls inbox::reanchor after every digest change, so a vanished group's cursor falls to the row that took its index (the next), else the previous, else none"
  - "D-03/D-04: lane filters and the FIX Command column are layout state only; column widths are computed over every current row so no filter reflows them"
requirements-completed: []
requirements-advanced: [OWN-20]
duration: 95 min
completed: 2026-11-14
status: complete
commits: 2
plan_head_before: 913b6f9381c18649725ab20a799295d8f1e0158c
plan_head_after: 658e3e57766bbb7445a379b9afe5bf795f64e66b
actuals:
  tokens: 60000
  tasks: 2
  commits: 2
---

# Phase 37 Plan 04: Inbox layout contract, loading/unreadable, live refresh Summary

The inbox now tells a project that is still loading from one that cannot be read, follows the files (a passed or acknowledged row leaves on the next watcher refresh and the cursor moves to its neighbor), and holds the UI-SPEC layout at every width with `1`-`4`/`a` lane filters.

## Accomplishments

- Task 1 (4f3cfd6b): `Action::OwnerDigestFailed`, `AppContext.owner_digest_failed`, the App handlers and the prune; `DigestState` and the DIM ` · {M} loading` / ` · {U} unreadable` header clauses; body copy for no projects, loading, none readable, no owner work; `reanchor`; SC3 tests (`sc3_a_passed_uat_leaves_the_inbox_and_the_cursor_moves_on`, the previous-row variant, `sc3_an_acknowledged_file_leaves_the_inbox`) over real watcher refreshes of temp fixture copies.
- Task 2 (658e3e57): `grid_for_inbox` with the FIX Command column (only when it fits without squeezing, 25 cells on the fixture from W 126), Age from W 80, then phrase, phase cell, alias, name; `LaneFilter` with the REVERSED header token; the pinned collapsed fold line (three rungs, empty folds omitted, `items` vs `checks`, Triage `?`); the footer ladder over the keys that work (`FOOTER_KEYS`); lane-specific empty copy; tier, SC2, degrade, filter, empty, quiet-color and fold-ladder tests.
- Owner block pin untouched; `owner_block::acknowledged_files` only gained `pub(crate)`.

## Deviations from Plan

- [Inferred] The six-rung header ladder (with the unreadable-then-loading clause drops) was built in Task 1's commit, not Task 2's, because the new clauses live in the header and the two could not be tested apart. Task 2 only wires the active lane token into it.
- [Inferred] The plan says "W 79: Age dropped, Phrase 12". The arithmetic over the fixture gives phrase 25 at W 79 (Age's 13 cells return to the phrase); 12 is reached at W 66 and below. The UI-SPEC table says "shrinks as needed". The test asserts 25 at 79 and 12 at 60.
- [Inferred] Fold line row height 0 below 6 rows (the spacer drops below 7), so the UI-SPEC "spacer, then fold line" order holds.
- [Inferred] Header at a project count of 0 reads `Owner inbox ── 0 projects`; with no digest and nothing loading it reads `{N} projects` plus the unreadable clause.
- [Inferred] `OwnerDigestFailed` is ignored for an alias that already holds a digest (a reparse digest is never older).
- [Inferred] UI-SPEC fixture fleet gained the numbers its folds need (8 acknowledged files, 3 archived checks, 11 pending todos in 2 projects, handoff human action, 1 open question) so the fold and phrase copy match the spec literally.
- Rule 3: four test-only `AppContext` literals (`delete_confirm`, `detail`, `driver_confirm`, `normal`) gained the new field.

## Follow-ups

- The dashboard `You` cell and `You:` summary are frozen (36 UAT pending): a project whose digest failed still counts as loading there. Align them after 36 UAT if the owner wants the same word.
- Fold line is drawn collapsed and is not a cursor stop yet (37-05); footer rungs for `Enter g o A y` are asserted in 37-07.

## Verification

- `cargo test --locked --no-fail-fast --all-targets`: 4330 passed, 0 failed, 19 ignored.
- `cargo clippy --all-targets -- -D warnings`: clean.
- Human UAT (D-23 item 1, in /gsd-verify-work): ordering and legibility at 120 columns on the real fleet; `·` and `—` alignment on the owner's font (ambiguous-width backstop).

## Known Stubs

None.

## Self-Check: PASSED

Both commits (4f3cfd6b, 658e3e57) are ancestors of HEAD; the files listed above exist.
