---
phase: quick-261119-f5b
verified: 2026-11-19T00:00:00Z
status: passed
score: 8/8 must-haves verified
covered_files:
  - .planning/quick/261119-f5b-owner-row-action-menu-and-context-aware-/261119-f5b-PLAN.md
  - .planning/quick/261119-f5b-owner-row-action-menu-and-context-aware-/261119-f5b-SUMMARY.md
  - README.md
  - docs/GETTING-STARTED.md
  - src/ui/screens/detail.rs
  - src/ui/screens/help.rs
  - src/ui/screens/inbox.rs
  - src/ui/screens/owner_actions.rs
  - src/ui/screens/owner_row_actions.rs
  - src/ui/screens/owner_settle.rs
covered_digest: "v4:sha256:6150d9ab91952081cdf3caeade9bb08300ceb69dac244ad9171d39db396d8ee9"
behavior_unverified: 0
overrides_applied: 0
---

# Quick 261119-f5b: Owner-row action menu and context-aware footer Verification Report

**Goal:** One shared implementation for the Owner block and the inbox; consistent UX with `g` the only documented screen difference; truthful footer degrading at 80/100 columns and keeping `[?]help`; an Enter menu listing applicable actions (blocked ones dim with a textual reason) that executes through the hotkey path (`x` settle confirm, `A` asks first); fits 80x24; docs and help updated.
**Verified:** 2026-11-19
**Status:** passed
**Re-verification:** No, initial verification

## Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Footer on an Owner row lists exactly the working keys, `[v]run CMD` when there is room, drops at 80/100 columns, `[?]help` last | VERIFIED | `footer_spans` in `owner_row_actions.rs` builds tokens from `row_actions` (ready actions only). `fit_footer` order is p3, p2, shorten, p1, and `[?]help` is appended after fitting. Both hosts call it (the old `FOOTER_KEYS` and `owner_block_footer_spans` are gone: grep count 0 each). Tests `the_footer_names_exactly_what_works_on_every_row` and `the_footer_fits_100_and_80_columns_on_every_row` (inbox and Owner block) pass. Inbox goldens: 100 cols `[Enter]actions  [v]run /gsd-verify-work 24  [x]settle  [o]pen  [A]ck  [y]copy  [g]oto  [?]help`; 80 cols drops the command and keeps every key |
| 2 | Enter on a group or debug row opens the menu in both screens with key, label and command or target; blocked entries dim with a reason in text; never-applicable actions absent; fits 80x24 | VERIFIED | `ActionMenu::open` and `render_menu`. `menu_lines` marks blocked entries `Modifier::DIM` and wraps `(reason)` as text. `row_actions` omits never-applicable kinds. Tests `the_menu_fits_80x24_on_every_row` (inbox), `owner_menu_fits_80x24_on_every_row`, `the_menu_fits_80x24_with_every_entry_and_the_selection_marker` and `a_blocked_entry_is_dim_with_its_whole_reason_and_the_menu_flips_above` pass. Hosts: inbox `enter_key` opens the menu; detail `handle_owner_block_key` sets `menu_row` |
| 3 | A menu entry runs the exact hotkey path; Esc/q close; all other keys and mouse swallowed while open | VERIFIED | `MenuKey::Run(code)` closes the menu and re-dispatches `self.handle_key(code, NONE, ctx)` in both hosts (inbox `handle_key` top, detail `handle_key` top). Detail `handle_mouse` returns `None` while the menu resolves. Tests `a_menu_entry_is_the_hotkey_path`, `owner_menu_entries_are_the_hotkey_path`, `an_open_menu_swallows_filters_help_and_navigation`, `owner_open_menu_swallows_keys_and_mouse`, `the_menu_closes_before_a_popup_draws_its_backdrop` pass. The `x` chooser and `A` default-No popup are reached because the real key arms run. `take_run_launches` seam records `v`, so nothing launches for real |
| 4 | Enter on a check or file line opens the viewer; Enter on the fold line toggles the fold; Space toggles; Right/l expand, Left/h collapse; fold-line Left/Right (and h/l) step folds | VERIFIED | Inbox `enter_key` and detail `handle_owner_block_key` arms (`Enter if sub_stop => open_sub`, fold line `toggle`, `Space`, `Left/Right/h/l` with `on_fold_line` guard and `set_expanded`). Tests `right_and_left_expand_and_collapse_rows_and_step_folds`, `owner_right_and_left_expand_collapse_and_step_folds`, `enter_on_the_fold_line_toggles_the_fold_not_a_menu` and the existing u4e viewer tests pass |
| 5 | Same key and label for the same action in both screens; only `g` differs, in a documented exception list asserted by a test | VERIFIED | `INBOX_ONLY = [GoToProject]`, `OWNER_BLOCK_ONLY = []`, each with a doc comment giving the reason. `offered()` applies them. `both_screens_offer_the_same_action_under_the_same_key_and_label` passes and `assert_eq!(INBOX_ONLY, [ActionId::GoToProject])` pins the set |
| 6 | `x` on a settling row refuses with one shared message (hotkey and menu), shown dim in the menu | VERIFIED | `SETTLE_PENDING_REFUSAL` and `settle_pending_refusal` are called by the inbox `settle_key` (`inbox.rs` 2096), the Owner block `owner_settle` (`detail.rs` 2473) and the model (`Availability::Blocked`). Tests `x_is_blocked_while_the_settle_runs`, `x_on_a_settling_row_refuses_once` and `owner_x_on_a_settling_row_refuses_once` pass |
| 7 | Owner block `y` on Old debt copies that project's checklist; Acknowledged refuses with the inbox wording | VERIFIED | `owner_copy` calls `fold_copy(ctx, fold, Some(&self.alias))`. `old_debt_text(ctx, only)` narrows to one alias. The old `Select a group row...` refusal remains only as the `None` fold fallback. Detail test asserts the OSC 52 payload equals `old_debt_text(&ctx, Some(TEST_ALIAS))` and the Acknowledged refusal text. `old_debt_text_is_the_fleet_or_one_project` passes |
| 8 | Help overlay, README and GETTING-STARTED document Enter (menu), Space, Left/Right, h/l and the menu keys; the key-highlight tests stay green | VERIFIED | Diff shows help rows for both sections, README key rows, a footer and action-menu paragraph, a mouse note and settle wording, and a new `### Acting on owner work` section. The help, glyph and render-escape tests (42 selected) pass, including `production_ui_code_draws_only_registry_or_exempt_glyphs` and the key-token tests |

**Score:** 8/8 truths verified, 0 behavior-unverified.

## Required Artifacts

| Artifact | Status | Details |
|----------|--------|---------|
| `src/ui/screens/owner_row_actions.rs` (1630 lines) | VERIFIED | Contains Host, ActionId, Availability, RowAction, the exception lists, `row_actions`, `fold_line_actions`, `footer_spans`, `settle_pending_refusal`, `row_group`, `ActionMenu`, `MenuKey`, `render_menu`, and a purity-lint test (`the_model_source_reads_no_file_and_starts_nothing`). Registered in `mod.rs` |
| `inbox.rs`, `detail.rs` | VERIFIED | Both hold a `menu` field and an anchor cell, intercept keys at the top of `handle_key`, draw the overlay last, and use the shared footer |
| `owner_actions.rs`, `owner_settle.rs` | VERIFIED | `cfg(test)` recording seam in `launch_real` plus `take_run_launches`. Shared refusal helpers `open_files_refusal` and `merged_ack_refusal`. `fold_copy` scoped by `only` |
| `help.rs`, `README.md`, `docs/GETTING-STARTED.md` | VERIFIED | Updated as described under truth 8 |

## Key Links

| From | To | Status |
|------|----|--------|
| inbox `footer_line`, Enter, `MenuKey::Run` to the model | WIRED |
| detail Owner footer, menu intercept and render to the model | WIRED |
| model to `owner_actions::run_plan` and the shared refusal helpers | WIRED |
| both hosts' `x` arms to `settle_pending_refusal` | WIRED |
| detail `owner_copy` to `fold_copy(.., Some(alias))` | WIRED |

## Gates re-run by the verifier

| Check | Result |
|-------|--------|
| `cargo test --lib -- owner_row_actions` | 16 passed, 0 failed |
| `cargo test --no-fail-fast` (whole workspace, summed from the log) | 4683 passed, 0 failed, 19 ignored. Matches the executor's claim |
| `cargo clippy --all-targets -- -D warnings` | clean |
| `cargo fmt --check` | clean |
| Debt markers (TBD/FIXME/XXX/TODO/todo!) in added source lines | none |
| Plan gate greps (`FOOTER_KEYS`=0, `owner_block_footer_spans`=0, `take_run_launches` present, `INBOX_ONLY` at least 2) | satisfied |

## Anti-Patterns

None found. The menu never launches anything itself, because it re-dispatches the host key. The model does no I/O, and a test lints its source for `std::fs`, `Command::new` and similar.

## Notes (non-blocking)

- CONTEXT said "mouse click on a menu entry if the screen already supports mouse". The documented INFERRED decision I-8 overrides this: the menu is keyboard-only, and mouse is swallowed while the menu is open. The README says so. This is a recorded owner-reviewable choice, not a gap.
- The 80x24 test checks line width and that every key and label is drawn. The vertical placement and flip-above logic are covered by the module-level render tests and not by the per-row host tests. That is adequate evidence.

## Human Verification

None required. The goal is observable through the automated tests, which render real frames. A visual look in a terminal would be optional polish only.

## Gaps Summary

No gaps. The phase goal is achieved in the codebase.

---

_Verified: 2026-11-19_
_Verifier: Claude (gsd-verifier)_
