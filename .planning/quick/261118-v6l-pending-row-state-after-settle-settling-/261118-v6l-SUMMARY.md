---
phase: quick-261118-v6l
plan: 01
subsystem: ui
tags: [ratatui, owner-inbox, owner-block, settle, pending-state]
requires:
  - phase: 39
    provides: settle popup (`x`), launch_settle, SettlePlan
  - phase: 37
    provides: read_owner_file containment, owner digest reparse via ProjectStateLoaded
provides:
  - in-memory PendingSettles on AppContext, registered at Yes by owner_settle::launch_and_track
  - dim `settling… <outcome> · launched|recorded m:ss` badge on the settled row in the inbox and the Owner block
  - launched -> recorded on a source-file fingerprint change at the next reparse; done line `✓ test 2 skipped` / `✓ phase 01 skipped` / `✓ moved to phase M` for 2 s as a draw-only row
affects: [owner inbox, owner block, app tick and refresh, README]
status: complete
commits: 3
plan_head_before: 90173959edef9f1356de094e64f5cc53cc0ee298
plan_head_after: dd086ffbf25f6a1151cf4f48be25c13d57424a5e
actuals:
  tokens: 23400
  tasks: 3
  commits: 3
key-files:
  created:
    - src/ui/screens/settle_pending.rs
  modified:
    - src/ui/screens/mod.rs
    - src/ui/screens/owner_settle.rs
    - src/ui/screens/settle_popup.rs
    - src/ui/screens/inbox.rs
    - src/ui/screens/owner_block.rs
    - src/ui/screens/detail.rs
    - src/app.rs
    - src/ui/screens/normal.rs
    - src/ui/screens/driver_confirm.rs
    - src/ui/screens/delete_confirm.rs
    - tests/owner_settle_guard.rs
    - README.md
decisions:
  - "Pending state lives on AppContext.settle_pending (alias-keyed, memory only), not in inbox_view or ProjectState"
  - "Rows match an entry by facts (alias, lane, scope, workstream, phase, shared file), never by index or whole GroupKey"
  - "Lingering rows are draw-only in both hosts: no stop, no expansion, no key reaches them"
duration: ~1h
completed: 2026-11-19
---

# Quick 261118-v6l: pending row state after settle Summary

After Yes in the settle popup the settled row now stays in place with a dim `| settling… won't do · launched 0:12` badge (ASCII spinner, timer on the 250 ms tick). A reparse that sees the source file change steps it to `recorded`, and when the settled checks leave the list the row shows `✓ phase 01 skipped` for 2 s as a draw-only row and then goes. Inbox and Owner block, phase rows and single checks, new-session and live-session launches. The app still writes nothing: the only new filesystem call is a `stat` (`fingerprint`).

## Commits

| # | Hash | What |
|---|------|------|
| 1 (tracer) | f85d5349 | PendingSettles on AppContext, `launch_and_track`, badge ladder, inbox badge, prune, tick redraw, guard covers the new module |
| 2 | 0eabb7fb | `on_refresh`/`refresh`/`expire`, Done linger, inbox ghost rows (`row_order` extracted), App wiring |
| 3 | dd086ffb | `block_model_with`, Owner block badge and ghost row, detail.rs call site, README |

settle_popup.rs diff against base: `1 file changed, 1 insertion(+), 1 deletion(-)` (the `launch_and_track` line). help.rs and docs/GETTING-STARTED.md untouched.

## Verification (full gates, `rtk bypass`)

- `cargo test --no-fail-fast`: 69 suites, every `test result:` line `ok`; summed 4636 passed, 0 failed, 19 ignored. Lib line: `test result: ok. 3769 passed; 0 failed; 3 ignored; 0 measured; 0 filtered out` (u4e baseline 3739, +30). `owner_audit_conformance`: `test result: ok. 3 passed; 0 failed` (the oracle failure is gone). `owner_settle_guard`: `ok. 3 passed` over 3 files.
- `cargo clippy --all-targets -- -D warnings`: exit 0. `cargo fmt --check`: clean.
- The tracer's first red was a compile error (`settle_pending` module and `ctx.settle_pending` absent); then green.

80x24 lines recorded from the tests:

- inbox pending: `>▸ alder-api  ph 01  Alpha      2 checks  | settling… won't do · launched 0:00`
- detail pending: `> TEST    ph 01  Alpha          2 checks  | settling… won't do · launched 0:00`
- inbox done: ` ▸ alder-api  ph 01  Alpha      2 checks  ✓ phase 01 skipped`
- detail done: `  TEST    ph 01  Alpha          2 checks  ✓ phase 01 skipped`

Each pending line occurs once (one line, never wrapped) and is at most 80 cells.

## Inferred decisions (INFERRED, as built)

- **I-1 INFERRED, stages and detection:** built as planned. `Launched` at Yes; `Recorded` when `fingerprint` (modified time + length via `std::fs::metadata`, canonicalized and contained in `.planning/`) of any source file differs from the one taken before the launch; checked in `Action::ProjectStateLoaded` through `settle_pending::on_refresh`. A file that was readable and now is not counts as changed (test: removed file). `Done { since }` only when no group of the fresh digest with the entry's lane, scope, workstream and phase still lists a stored `(id, name)` pair AND the stage is `Recorded` after this refresh's stat (one write goes Launched -> Done in one refresh, tested). Gone without a change: dropped silently. No digest, no step.
- **I-2 INFERRED, linger and redraw:** `LINGER = 2 s`; `Tick` calls `expire` (drops at exactly `since + 2 s`, keeps at 1.9 s) and sets `needs_redraw` while any entry exists and the top screen is `inbox` or `DetailScreen::NAME` (`settle_pending_redraw_due`). No new timer.
- **I-3 INFERRED, where it lives:** `AppContext.settle_pending` (Default), alias-keyed; `prune_driver_maps` calls `retain_registered`; `every_per_alias_map_is_pruned` populates, destructures and asserts it.
- **I-4 INFERRED, keying:** `PendingSettle::covers` matches by alias, row key lane, scope label, workstream, phase and at least one shared file. Latest registered wins in `for_row`. `insert` replaces an older entry of the same alias whose `(id, name)` set overlaps. Verified across hosts (registered from inbox, shown in Owner block, and the reverse).
- **I-5 INFERRED, badge text:** rungs R1/R2/R3 then `fit_cells`, exactly as specified (`| settling… won't do · launched 0:12`, `| settling… launched 0:12`, `| settling… 0:12`); `Done` is `✓ {short} skipped` for Won't do and Deferred permanently, `✓ moved to phase M` for Move. Words: `won't do`, `defer permanently`, `move to phase M`, with `test 2 ` in front for one check. No glyph added to `glyphs::ALL`; `✓` is `glyphs::CHECK`.
- **I-6 INFERRED, placement:** the badge starts at the host grid's phrase column (inbox `checks_x + checks_w + 2`, Owner block `2 + TAG_W + 1 + phase_w + 2 + name_w + 2 + checks_w + 2`), phrase/extras/age/command cleared; `badged_row` cuts the row two cells before the badge and moves the start left until the shortest rung fits.
- **I-7 INFERRED, styles:** settling badge DIM; done badge not dimmed with `✓` Green; lingering row DIM (asserted on the buffer); the inbox cursor style is patched onto the badge; the Owner block's `mark_cursor_row` covers the whole line (test: the badged cursor row starts with `>`).
- **I-8 INFERRED, lingering rows:** draw-only in both hosts. Inbox: `BodyItem::Ghost`, merged into the plan in `row_order` order (comparator extracted from `sort_groups`), with its section title when it is alone in its section (test shows the `TEST` title), outside the grid, lane filter applied, never in `stop_rows`. Owner block: merged by the new `current_order_key` (extracted from `current_sources`), pushes no `stop_lines`/`item_lines`, never expands, block keeps its height while it lingers and is identical after `expire`. Only unfolded rows are snapshotted, so an open-fold row gets the live badge but no lingering copy.
- **I-9 INFERRED, single checks:** the badge goes on the group row (`test 2 won't do`); once test 2 is recorded and test 3 is open the row stays and shows `✓ test 2 skipped`, then returns to normal after the linger (tested).
- **I-10 INFERRED, launch paths:** `launch_started` (extracted so it is testable): live session always registers; a new session only when the status equals `launched_status` or contains `opened in`; `Failed to launch...` and `no terminal emulator found` do not (tested).
- **I-11 INFERRED, edits outside the core:** settle_popup.rs one line; help.rs and GETTING-STARTED.md untouched; README settle paragraph gained the badge description.
- **I-12 INFERRED, fingerprint timing:** taken in `pending_for` before `launch_settle`.
- **I-13 INFERRED, guard:** `src/ui/screens/settle_pending.rs` added to `SETTLE_FILES` (array of 3) and the doc wording widened; no method named `output`/`status` and no write or spawn API in the module.

## Deviations from Plan

**1. [Rule 3 - Blocking] `block_model` made `#[cfg(test)]`.** The plan kept `block_model` as a one-line delegate, but after the detail.rs call moved to `block_model_with` the only callers are tests, and `clippy -D warnings` rejects the dead non-test function. It stays as the test-side delegate (every existing test unchanged). Commit dd086ffb.

**2. [Rule 3 - Blocking] `#[derive(Clone)]` on `owner_block::RowCells`.** Needed to size the grid from a lone lingering row. Commit dd086ffb.

**3. Ghost rows use `BodyItem::Ghost(usize)` (an index into the ghost list), not `Ghost(Line)`.** The plan wrote `Ghost(Line<'static>)`, but the key handlers also build plans through `full_plan` (scroll math) without a grid, so the plan carries the identity and `render` builds the line with its grid. `full_plan` computes the same ghost list, so scroll offsets agree. Commit 0eabb7fb.

**4. A lingering row sizes the grid only when it is the only row (inbox and Owner block).** The plan said the grid ignores lingering rows; with no other row the grid would collapse to defaults and cut the line. Otherwise they are ignored, as planned. Commits 0eabb7fb, dd086ffb.

**5. Inbox empty-state also keys on ghosts.** The plan said so; noting the realistic consequence: when the settled row was the only row, the inbox now shows the title and the `✓` line for 2 s instead of the empty copy.

**6. Extra tests beyond the plan's list:** two App-level tests (a real watcher-style reparse steps the entry and the tick expires it and redraws; the tick redraw gate only on inbox/detail), a fingerprint containment test, a second-Yes replacement test. `PendingSettle` and `PendingSettles` derive no `Debug` (`RowSrc` has none). `PendingSettles::entries`/`entries_mut`/`len` are `#[cfg(test)]`.

No settle_popup test needed a fix (no test-only hunk in that file).

## Observations (not fixed, out of scope)

- A lingering inbox row is drawn with the collapsed `▸` glyph (it reuses `row_line`), although nothing can expand it. Harmless for 2 s; hiding it would need a flag on `row_line`.
- The badge is built from `Instant::now()` on every render, so the model is not byte-stable between frames; the render path reads no file and the redraw is gated.

## Known Stubs

None.

## Threat Flags

None beyond the plan's register: `fingerprint` canonicalizes `.planning/` and the target and requires containment and a regular file (T-v6l-01); the guard scans the new module (T-v6l-02); `✓` only after a file change and the checks leaving the digest (T-v6l-03); every drawn string goes through `shown` (T-v6l-04); lingering rows are in no stop list and no key handler sees them (T-v6l-05).

## Self-Check: PASSED

Files found: settle_pending.rs, the three commits (f85d5349, 0eabb7fb, dd086ffb) are ancestors of HEAD, full test/clippy/fmt gates green. STATE.md and ROADMAP.md were not updated or committed, per the task constraints.
