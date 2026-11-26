---
phase: quick-261119-f5b
plan: 01
subsystem: ui
tags: [tui, ratatui, owner-inbox, action-menu, footer]
requires: [quick-261118-u4e, quick-261118-v6l, quick-261118-tki]
provides:
  - "src/ui/screens/owner_row_actions.rs: one Owner row action model, footer builder and Enter action menu shared by the inbox and the Owner block"
affects: [src/ui/screens/inbox.rs, src/ui/screens/detail.rs, src/ui/screens/help.rs, README.md, docs/GETTING-STARTED.md]
tech-stack:
  added: []
  patterns: ["pure action model over AppContext (no I/O on the render path)", "menu execution = re-dispatch through the host's own handle_key"]
key-files:
  created: [src/ui/screens/owner_row_actions.rs]
  modified:
    - src/ui/screens/mod.rs
    - src/ui/screens/inbox.rs
    - src/ui/screens/detail.rs
    - src/ui/screens/owner_actions.rs
    - src/ui/screens/owner_settle.rs
    - src/ui/screens/owner_block.rs
    - src/ui/screens/settle_popup.rs
    - src/ui/screens/settle_pending.rs
    - src/ui/screens/render_escape_guard.rs
    - src/ui/screens/help.rs
    - README.md
    - docs/GETTING-STARTED.md
decisions:
  - "Solution B (footer + Enter which-key menu) chosen over A (footer only) and C (inline chip strip)"
  - "Only screen exception is g (INBOX_ONLY); OWNER_BLOCK_ONLY is empty"
  - "x on a settling row is refused with one shared message in both hotkey and menu"
  - "Owner block y on the Old debt fold copies that project's old debt (aligned with the inbox)"
metrics:
  duration: n/a (three executor sessions)
  completed: 2026-11-19
status: complete
commits: 3
plan_head_before: 237ca4d9040e8771aae898ecfb305bba8a292fe7
plan_head_after: 1ad67ee360182a0725bb3bf1ddc765eb23d11c9b
actuals:
  tokens: 49000    # chars/4 over the non-.planning diff 237ca4d9..HEAD (197105 chars)
  tasks: 3
  commits: 3       # git rev-list --count 237ca4d9..HEAD
---

# Quick 261119-f5b: Owner-row action menu and context-aware footer

**One shared action model (`owner_row_actions.rs`) now drives a truthful, width-fitting footer and an Enter
which-key action menu in both the global inbox and the Phases-tab Owner block; menu entries run through the
host's own `handle_key`, so every hotkey guard (settle chooser, default-No confirms) still applies.**

## Commits (path-scoped, on feat/v2.5.0-owner-inbox, nothing pushed)

| Task | Commit | Message |
|------|--------|---------|
| 1 | `81126f3e` | feat(quick-261119-f5b): one Owner row action model drives a truthful footer in both screens |
| 2 | `5c8bc68b` | feat(quick-261119-f5b): Enter opens the Owner row action menu in the inbox and the Owner block |
| 3 | `1ad67ee3` | docs(quick-261119-f5b): document the Owner row action menu, Space and the arrows |

Ledger base (`plan_head_before`): `237ca4d9` (the pre-dispatch artifacts commit). 3 commits since.

## The three solutions and the verdict

- **A. Context-aware footer only.** Smallest change, no muscle-memory break; fixes the three untruthful footers
  (`[Enter]expand` on a check line, `[y]copy`/`[Enter]expand` on folds that only refuse, `[x]settle` hidden below 137
  columns). But at 80 columns it must cut 2-4 hints, refusal reasons stay invisible, nothing can show "applies but
  blocked right now", and it is not the Enter context menu the owner asked for. Rejected as incomplete.
- **B. Footer + Enter opens a which-key action menu. CHOSEN.** Complete and discoverable: every action and every
  reason is one keypress away, fits 80x24, and lets the footer drop low-priority hints at 80/100 columns because the
  menu always holds the full list. Cost: Enter-expands muscle memory changes (mitigated by Space, Right/`l` and an
  `expand` menu entry; about 35 existing tests switched from Enter to Space).
- **C. Inline action strip under the selected row.** Actions visible in place, but it costs a body row in an Owner
  block with about 6 body rows at 80x24, re-flows rows on each cursor move (breaks the identity-based scroll math),
  makes Left/Right mean chip-pick on rows but fold-step on the fold line, and duplicates the footer. Rejected.

**Why B:** it is the only option that satisfies both "truthful and fitting at 80 columns" and "nothing hidden", and
it matches the owner's stated expectation at a small fixed cost (one overlay, one Enter rebind on group rows only).

## Per-row-kind action table (what the shared model returns)

R = ready (footer and menu); B(reason) = applies but blocked now (dim with the reason in the menu, not in the
footer, the hotkey prints the same reason); `-` = never applies (absent). Same in both screens except `g`.

| Row kind | v run | x settle | o open | A acknowledge | y copy | g (inbox only) | Space | Enter |
|---|---|---|---|---|---|---|---|---|
| TEST group (UAT/VERIFICATION) | R `/gsd-verify-work N` | R whole phase | R (1 file) | R | R | R | expand/collapse | menu |
| TEST row with the settling badge | R | B `Settle already pending for this row` | R | R | R | R | R | menu |
| Merged TEST row (inbox only, 2+ files) | R | R | B (n files behind this row: expand it, Enter on a file line) | B (n files: press g, then A in the Owner block) | R | R | R | menu |
| FIX group | R `/gsd-plan-phase N --gaps` | - | R | R | R | R | R | menu |
| DECIDE: CONTEXT questions | - | - | R | R | R | R | R | menu |
| DECIDE: handoff / checkpoint | R `/gsd-resume-work` | - | R | - | R | R | R | menu |
| Debug investigation | R `/gsd-debug` | - | R | R | R | R | R | menu |
| BLOCKED aggregate | - | R whole phase | R if 1 file, else B | - | R | R | R | menu |
| Any lane, unreadable source | B `No command: the source file is unreadable` | B `Cannot settle: the source file is unreadable` | R | R | R | R | R | menu |
| Unsafe phase token / workstream | B `Refused: ...` | key-time | R | key-time | R | R | R | menu |
| Row in Old debt fold (archived) | - | - | R | R (`--archived-milestone`) | R | R | R | menu |
| Row in Acknowledged fold | R if group has a command | - | R | - | R | R | R | menu |
| Check line | parent's | R this one check | parent's | parent's | parent's | R | collapse | read-only viewer at the check |
| `file PATH` line | parent's | R whole phase | parent's | parent's | parent's | R | collapse | read-only viewer |
| Fold line: Old debt | - | - | - | - | R (inbox: fleet; Owner block: this project) | - | open/close | open/close fold |
| Fold line: Acknowledged / Stale | - | - | - | - | - | - | open/close | open/close fold |
| Fold line: Triage / Todos (inbox) | - | - | - | - | - | - | - | refuses (unchanged) |

Environment refusals that need I/O (no launcher, `node` missing, active-workstream pointer, acknowledge busy, file
unreadable at key time) are not predicted by the model (OWN-12: no I/O on render): the action is R and the key prints
the refusal, as before.

**Documented screen exception (the complete list):** `INBOX_ONLY = [ActionId::GoToProject]` (the inbox lists every
project and `g` opens one; in the Owner block you are already in the project and `g` means "first row", navigation not a
row action). `OWNER_BLOCK_ONLY = []`. The cross-screen consistency test asserts the set difference equals these lists.
No further exception was found during implementation. Row-kind differences from data (merged rows, Stale and Triage
folds exist only in the inbox) are the same model applied to rows only one screen lists.

## Behavior delivered

- **Footer** (both screens): lists the R actions of the selected row in the order v x o A y g, with `[v]run CMD` when
  there is room, `[Enter]actions` on rows, `[Enter]view` on check/file lines, `[Enter]open|close` on the fold line,
  `[Space]expand|collapse`. Fit order: drop p3 (`[j/k]`, `[q]`), then p2 (`[Space]`, `[Esc]`), then shorten `[v]run CMD`
  to `[v]run`, then p1; `[?]help` is always last. The old fixed inbox ladder and the Owner block's fixed footer are gone.
- **Menu**: Enter on a group or debug row. Bordered ` Actions ` overlay anchored below the row (flips above, clamped,
  centered without an anchor), `> ` ASCII marker, key padded to 6, label padded to 16, then the detail; blocked entries
  DIM with the reason in parentheses (wrapped, never cut); v preselected when ready. j/k/Up/Down skip blocked entries,
  Enter or a plain key letter (or Space) runs an entry, Esc and q close, every other key and mouse event is swallowed.
  Running = `menu = None` then the host's own `handle_key`, so `x` still opens the chooser and default-No confirm and
  `A` still opens the default-No popup. Popups that draw a fresh host backdrop never show the menu.
- **Enter / Space / arrows**: Enter on a check or file line still opens the viewer; on the fold line it toggles; on
  Triage/Todos it refuses. Space toggles rows, sub-stops (collapse) and folds. Right/`l` expand, Left/`h` collapse
  (from a check or file line too, cursor back on the row); on the fold line they step folds.
- **Behavior changes folded in**: `x` on a settling row refuses with `Settle already pending for this row` (hotkey and
  menu); the fold-line `x` refusal is one shared text; Owner block `y` on the Old debt fold copies that project's old
  debt, on the Acknowledged fold refuses with the inbox's wording.
- **Docs**: help overlay rows (Enter, Space, Left/Right, h/l, `j/k Enter Esc` for both sections), README key tables,
  Phases-tab and inbox paragraphs, a footer-and-action-menu paragraph, the mouse note, and a new
  `### Acting on owner work` section in docs/GETTING-STARTED.md.

## Inferred decisions (I-1..I-19, all INFERRED unless marked orchestrator)

All nineteen decisions in the plan were kept as written:
I-1 footer lists R actions only, blocked ones live in the menu; I-2 (orchestrator) fold-line Enter toggles, check/file
lines open the viewer, group/debug rows open the menu; I-3 `h`/`l` mirror Left/Right; I-4 Left/`h` on a check or file
line collapses the parent; I-5 j/k skip blocked entries, a blocked entry's letter prints its reason and closes the menu;
I-6 preselect v when ready, else the first ready entry; I-7 Esc/q close only the menu; I-8 (orchestrator) no mouse
selection, mouse swallowed while open; I-9 a vanished target closes the menu silently; I-10 footer spelling keeps
`[o]pen [A]ck [g]oto`; I-11 fit order p3, p2, shorten, p1; I-12 static ` Actions ` title, up to 76 cells; I-13
`FooterToken` stays in detail.rs with owned labels, optional short form and one shared fit function; I-14 pending
refusal applies while the covering settle is not Done; I-15 (orchestrator) Owner block Old debt `y` copies that
project; I-16 unified fold-line settle refusal; I-17 (orchestrator) GETTING-STARTED gains a short section;
I-18 live session keeps `[v]run CMD`, menu detail says `focus the live session, type CMD`; I-19 (orchestrator)
cfg(test) recording seam for `v` (`owner_actions::take_run_launches`).

## Deviations from Plan

**1. [Rule 3 - blocking, test] `settle_popup.rs::x_on_an_item_targets_one_check`** settles a check and then the whole
phase on the same row; with the new pending refusal (I-14) the second `x` is refused while the first still shows
`settling...`, so the test clears `ctx.settle_pending` between the two. File not in the plan's list. (Task 1)

**2. [Rule 1 - plan arithmetic] Footer width illustrations.** The plan's 100-column inbox illustration (96 cells) used
`[Enter]actions`; Task 1 used `[Enter]expand` until Task 2 swapped the word. Goldens in the tests are the real outputs.
In the Owner block at 100 columns `[Esc]phases` still fits (99 cells) and is kept. (Task 1)

**3. [Rule 1] `old_debt_text(ctx, Some(alias))`** iterates exactly that alias instead of filtering `config.projects`
(detail test contexts have digests for unregistered aliases); fleet behavior (None) unchanged. (Task 1)

**4. [Rule 1 - plan wording] Blocked menu reasons are wrapped** under the detail column instead of cut with `fit_cells`:
the longest reason (merged-row `A`, 75 chars) would be cut to about 50 cells at 80 columns and lose its instruction.
Ready entries stay one line. Box height is entries + wrapped lines + 2; `the_menu_fits_80x24_on_every_row` covers
every fixture row in both hosts. (Task 2)

**5. [Rule 3 - blocking, test] Files outside the plan's list:** `settle_pending.rs`
(`a_single_check_badges_its_row_and_its_done_line_stays_on_the_live_row` pressed Enter to expand, now Space) and
`settle_popup.rs` flows (settle flows that began with Enter then `x` kept passing because `x` now runs through the
menu; ones needing an expansion switched to Space). `owner_actions.rs` was not touched in Task 2 (the recording seam
came with Task 1). (Task 2)

**6. [Rule 3 - tooling] Render-escape arrival count.** The overlay covers rows that carry the identity, so the
"strictly more than the chrome baseline" arrival count cannot hold for the new state `Phases tab with the Owner action
menu open`; its DETAIL_TAB_ARRIVAL row is `false` with the reason (the state is still probed: containment assertions 2
and 4 apply). New inbox state `inbox with the action menu open over a hostile row`. Both adjudication strings name the
menu. A `#[cfg(test)] DetailScreen::with_owner_menu_open` builder opens the menu over the hostile digest. (Task 2)

**7. [Rule 1] Footer `Enter` enum:** `Expand`/`Collapse` replaced by one `Enter::Actions`; the `[Space]` token is drawn
on rows and check/file lines, not on the fold line (Enter already toggles there). (Task 2)

**8. Small helpers:** `owner_block::set_expanded(view, expand)` for Right/Left/`l`/`h` (the inbox has its own
`expand_key`). The inbox has no mouse handler, so only `DetailScreen::handle_mouse` swallows while the menu is open
(a stale menu is dropped, then the event acts normally). (Task 2)

**9. [Rule 1 - tests] Updated, not deleted:** `footer_ladder_matches_the_ui_spec`, `layout_tiers_140_120_100_80`,
`the_inbox_renders_sections_in_order_at_100x30`, `x_refuses_the_fold_line_and_the_folds_rows`,
`the_inbox_item_highlight_changes_no_text`, `a_selected_file_line_changes_no_text_and_stays_in_view`,
`owner_block_highlights_the_selected_file_line_in_place`, `the_footer_names_the_owner_block`,
`owner_block_y_copies_the_selected_group`, `x_on_an_item_targets_one_check`, about 35 Enter-to-expand tests switched
to Space. Renamed (names lied): detail `enter_expands_and_collapses_a_group` -> `space_expands_and_collapses_a_group`;
inbox `enter_expands_a_group_with_its_run_line` -> `space_expands_a_group_with_its_run_line`. (Tasks 1-2)

**10. [Rule 1 - tests] Help key-highlight match counts.** Adding rows changed the whole-body match counts the tki tests
pin: `j` 1/7 -> 1/9 (two `j/k Enter Esc` rows), `Space` 1/3 -> 1/4 (the old `Enter / Space` Owner row became `Enter` +
`Space`, plus the inbox `Space` row). `Left` stays 1/5. Both tests updated, nothing weakened. (Task 3)

**11. Doc structure.** The footer-and-action-menu text in README.md is a bold-led paragraph inside the Owner inbox
section rather than a heading, so the `v builds the command` and `A` paragraphs stay under the inbox screen. (Task 3)

Auth gates: none. Package installs: none (no dependency added or changed).

## Tests added

owner_row_actions: the_model_matches_the_action_table_row_by_row, an_unsafe_phase_token_blocks_the_run_action,
a_merged_inbox_row_blocks_open_and_acknowledge, both_screens_offer_the_same_action_under_the_same_key_and_label,
the_fold_line_offers_copy_on_old_debt_only, x_is_blocked_while_the_settle_runs, the_footer_names_the_ready_keys_only,
the_footer_drops_in_the_documented_order, the_enter_and_space_tokens_follow_the_cursor,
the_model_source_reads_no_file_and_starts_nothing, the_menu_preselects_v_else_the_first_ready_entry,
the_menu_walks_ready_entries_and_enter_runs_the_selection, the_menu_letters_run_and_everything_else_is_swallowed,
the_menu_fits_80x24_with_every_entry_and_the_selection_marker,
a_blocked_entry_is_dim_with_its_whole_reason_and_the_menu_flips_above,
the_menu_centers_without_an_anchor_and_shrinks_on_a_narrow_frame.

inbox: the_footer_names_exactly_what_works_on_every_row, the_footer_fits_100_and_80_columns_on_every_row,
a_blocked_reason_is_what_the_hotkey_prints, x_on_a_settling_row_refuses_once, v_through_the_key_arm_records_the_plan,
enter_opens_the_menu_without_expanding_and_esc_closes_only_it, a_menu_entry_is_the_hotkey_path,
the_menu_closes_before_a_popup_draws_its_backdrop, a_blocked_menu_entry_prints_the_reason_the_menu_drew,
an_open_menu_swallows_filters_help_and_navigation, a_refresh_that_drops_the_target_closes_the_menu_silently,
right_and_left_expand_and_collapse_rows_and_step_folds, enter_on_the_fold_line_toggles_the_fold_not_a_menu,
the_menu_fits_80x24_on_every_row.

detail: the_owner_footer_names_exactly_what_works_on_every_row, the_owner_footer_fits_100_and_80_columns,
an_owner_blocked_reason_is_what_the_hotkey_prints, owner_x_on_a_settling_row_refuses_once,
owner_x_on_the_fold_line_says_the_shared_text, owner_enter_opens_the_menu_and_esc_or_q_close_only_it,
owner_menu_entries_are_the_hotkey_path, owner_menu_closes_before_a_popup_draws_its_backdrop,
owner_blocked_menu_entry_prints_the_reason_the_menu_drew, owner_open_menu_swallows_keys_and_mouse,
owner_refresh_that_drops_the_target_closes_the_menu_silently, owner_right_and_left_expand_collapse_and_step_folds,
owner_menu_fits_80x24_on_every_row.

owner_actions: old_debt_text_is_the_fleet_or_one_project, the_open_and_merged_ack_refusals_are_pure_and_shared.
owner_settle: the_settle_group_rule_separates_kinds_from_unreadable_files.
settle_popup: x_through_the_menu_still_asks_and_the_default_is_no.

## Raw gate results

All from the repo root with `rtk bypass`.

| Point | `cargo test --no-fail-fast` (69 suites) | lib suite |
|---|---|---|
| Baseline, HEAD 237ca4d9 (`target/f5b-baseline.log`) | 4635 passed, 1 failed, 19 ignored; the failure (`registry_test::end_to_end_add_then_list_via_cli`) was a mid-run edit being compiled by `cargo run`, it passes at HEAD, so clean baseline = 4636 passed / 0 failed / 19 ignored | 3769 passed, 3 ignored |
| After Task 1 (`target/f5b-task1.log`) | 4659 passed, 0 failed, 19 ignored | 3792 passed, 3 ignored |
| After Task 2 (`target/f5b-task2.log`) | 4683 passed, 0 failed, 19 ignored | n/a |
| Final, after Task 3 (`target/f5b-final.log`) | **4683 passed, 0 failed, 19 ignored** (summed over all 69 `test result` lines) | n/a |

- `rtk bypass cargo clippy --all-targets -- -D warnings`: clean (no warnings, finished).
- `rtk bypass cargo fmt --check`: clean (exit 0).
- Task 3 doc gates: `action menu` appears 8x in README.md, 1x in docs/GETTING-STARTED.md, 8x in help.rs (plan floors 1, 1, 2).
- Net over the clean baseline: +47 tests, none removed, ignored count unchanged. Task 3 added no tests; it updated the
  help whole-row tests and the two key-highlight counts.

## Known Stubs

None. No placeholder data, empty-valued UI sources or TODO/FIXME was introduced.

## Threat Flags

None. No new network endpoint, auth path, file access or schema surface. The menu never launches anything itself
(T-f5b-02); the model reads no file or PATH (T-f5b-03, enforced by `the_model_source_reads_no_file_and_starts_nothing`);
project-derived text goes through `shown` (T-f5b-01, render-escape probes cover both hosts); `tests/focus_entry_guard.rs`
and `tests/owner_settle_guard.rs` stay green.

## Self-Check: PASSED

- `src/ui/screens/owner_row_actions.rs`: FOUND
- Commits `81126f3e`, `5c8bc68b`, `1ad67ee3`: all ancestors of HEAD
- Working tree clean apart from this uncommitted SUMMARY.md

## Review fixes

Commits: `ee78c69f` (WR-01, WR-02, WR-03, IN-01, IN-03), `dd15c1a2` (WR-04, WR-02 at render).

- **WR-01** `owner_settle::settle_refusal` and `owner_actions::ack_refusal` are the pure halves of `resolve_draft` and
  `ack_request` (shared `settle_inputs` / `ack_inputs` lookups, then the same `settle_target_for` / `ack_target_for`
  with `active_workstream_set = false`). `row_actions` blocks `x` and `A` with the key's own text, so an unsafe phase
  token or workstream, an unlisted check, a missing milestone or project state no longer list them as ready. Tests
  compare the model text with the key's `Err` for each case.
- **WR-02** `ActionMenu::reconcile` keeps the selection on a shown entry (same id while ready, else the nearest ready
  entry in action order, earlier on a tie); it runs before every key and every frame (`render_open_menu`).
- **WR-03** `render_menu` scrolls the entry window (`scroll_to`) when the frame is shorter than the menu; the hint on
  the bottom border stays. The 80x24 tests are unchanged and green.
- **WR-04** Both hosts keep the menu in a `RefCell`; `render_open_menu` clears it when its row, cursor or focus no
  longer matches, so it cannot reappear and the next key is not swallowed.
- **IN-01** `fold_line_actions` lost its unused `host` parameter. **IN-03** `footer_spans` doc fixed.
- **IN-02** kept: `OWNER_BLOCK_ONLY` documents the exception contract the equality test asserts. **IN-04** left:
  the two expand helpers act on different state types (`InboxViewState` / `OwnerBlockState`) and differ in `follow`;
  both are covered by their own host tests.
- Fixtures: `fixture::known_milestone` gives test projects a milestone (a real project has one; `A` now refuses one
  without), used by the model, the inbox row-action and the two inbox footer tests.

Gates (raw, repo root): `cargo test --no-fail-fast` 69 suites, 4690 passed, 0 failed, 19 ignored (lib: 3823 passed,
3 ignored); `cargo clippy --all-targets -- -D warnings` clean; `cargo fmt --check` clean.
