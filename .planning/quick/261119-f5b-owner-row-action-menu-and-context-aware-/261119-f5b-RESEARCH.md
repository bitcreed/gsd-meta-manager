# Quick 261119-f5b: Owner-row action menu and context-aware footer - Research

**Researched:** 2026-11-19
**Domain:** ratatui 0.30 TUI key dispatch / footer / popup patterns in this repo (no new dependencies)
**Confidence:** HIGH (every claim below was read from source this session; INFERRED items are marked)

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
- Research must document the current keys per row kind in both screens and write THREE distinct solutions with a short UX trade-off each (A footer only; B footer + Enter action menu; C inline action strip; refine or replace). The plan picks the best UX on the evidence and justifies it. Owner expectation is B, but the decision must be argued.
- One shared implementation (action model + footer builder + menu widget/state) used by both the Owner block and the inbox. No duplicated per-screen action logic. Same interaction model in both screens, same key and same label/wording for the same action on the same item kind. Screen-specific differences only as explicit, documented exceptions in code (named per-screen extras list with a doc comment).
- Required test: for the same item, every action both screens offer uses the same key and the same label; the difference set equals the documented exception set.
- Footer always truthful for the selected row (no listed key that does nothing, no applicable key missing). Degrade at 80 and 100 columns by dropping lower-priority hints; `[?]help` always kept.
- Menu (if chosen): only actions that work on that row; unavailable ones hidden or dim WITH the reason (not color-only). Executing from the menu goes through the exact same code path as the hotkey (x still shows the settle popup; A still asks first). Fits 80x24. Mouse click on an entry only if the screen already supports mouse elsewhere. Respect the glyph rules under src/ui/.
- Expand/collapse stays discoverable: update footers, src/ui/screens/help.rs rows, README.md, docs/GETTING-STARTED.md. Keep the quick-261118-tki help key-highlight tests green.
- Enter on a check line / `file PATH` line keeps opening the read-only viewer directly (quick-261118-u4e).
- Test-first where practical; gates `rtk bypass cargo test --no-fail-fast` and `rtk bypass cargo clippy --all-targets -- -D warnings`, raw counts. Branch feat/v2.5.0-owner-inbox, path-scoped conventional commits, never rebase/force/push, US spelling.

### Claude's Discretion (INFERRED unless evidence says otherwise)
- Direct expand/collapse key once Enter opens a menu (preference: Space and Right/l if no collision with folds).
- Unavailable actions: dim with reason when the action exists for the kind but is blocked right now; hide when it never applies to the kind.
- Menu placement: anchored below/above the selected row, clamped to the frame.

### Deferred Ideas (OUT OF SCOPE)
None listed.
</user_constraints>

## Summary

Both surfaces already funnel every side effect through shared functions (`owner_actions::run_group`, `open_target`, `open_at`, `ack_open`, `copy*`, `owner_settle::settle_open`), but the key arms, refusal-for-fold-line copy and footers are written twice. The Owner block footer advertises no action key at all (`detail.rs:12287-12305`), and the inbox footer is a fixed width ladder that is untruthful in three places today: it shows `[Enter]expand` on a check/file line where Enter opens the viewer, shows `[y]copy`/`[Enter]expand` on every fold (Acknowledged/Stale/Triage only refuse), and hides `[x]settle` below 137 columns (`inbox.rs:1472-1481`).

**Primary recommendation: Solution B (refined).** One pure action model in a new module `src/ui/screens/owner_row_actions.rs` feeds (1) a context-aware footer built on the existing `FooterToken` priority-drop mechanism and (2) a which-key style menu that the HOST screen owns as an `Option<ActionMenu>` field and draws as an overlay. Enter on a group row opens the menu; Enter on check/file lines (viewer) and on the fold line (toggle) is unchanged; Space keeps toggling; Right/`l` expand and Left/`h` collapse on rows only. A menu entry executes by closing the menu and re-dispatching the entry's `KeyCode` into the host's own key handler, so it is literally the hotkey path.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|---|---|---|---|
| Which actions apply to a row, label, reason | Shared pure model (`owner_row_actions.rs`) | state_reader digest | Pure over `AppContext` + row target; no I/O so it can run on the render path (OWN-12) |
| Executing an action | Host key handler (existing arms) | `owner_actions` / `owner_settle` | Re-dispatch keeps one code path; I/O refusals stay key-arm-only |
| Footer tokens / width drop | Shared footer builder | `FooterToken` (moved out of detail.rs) | Both hosts need identical tokens |
| Menu state + overlay | Host screen field | shared `ActionMenu` widget | Popups re-render a fresh host as backdrop, so screen-held state never leaks behind a confirm |

## Current keys per row kind (from code)

Dispatch: inbox `InboxScreen::handle_key` `inbox.rs:2296-2374`; Owner block `DetailScreen::handle_owner_block_key` `detail.rs:2203-2329` (reached only when `focus == DetailFocus::Owner`, `detail.rs:4612-4618`). Letters act only with `is_plain_action_key` (no Ctrl/Alt) `owner_actions.rs:150-153`. [VERIFIED: files read this session]

Commands per kind come from the reader: `run_for` gives `RunVerb::VerifyWork` for Test and `RunVerb::PlanPhaseGaps` for Fix, `None` for `Lane::Decide | Lane::Blocked | Lane::Deferred`, and `None` when `!matches!(file.scope, Scope::Current) || unreadable > 0` (`rules.rs:139-154`); debug groups always carry `debug_run()` = `/gsd-debug`, lane `Lane::Decide` when status starts with `awaiting` else `Lane::Fix` (`rules.rs:636-642, 667-671, 709-710`); CONTEXT questions `run: None` (`rules.rs:890-891`); handoff/checkpoint `RunVerb::ResumeWork` (`rules.rs:914-923`). Display strings verbatim: `"/gsd-verify-work {phase}"`, `"/gsd-plan-phase {phase} --gaps"`, `"/gsd-debug"`, `"/gsd-resume-work"`, plus `" --ws {ws}"` (`owner_items/mod.rs:519-525`).

Legend: Y works; R = refuses with a status line (quoted); - = no-op/swallowed. Same in both screens unless a cell says otherwise.

| Row kind | v | o | A | x | y | g | Enter | Space | Left/Right, h/l |
|---|---|---|---|---|---|---|---|---|---|
| TEST group (UAT/VERIFICATION) | Y `/gsd-verify-work N` | Y (1 file) | Y `uat_gaps`/`verification_gaps` | Y settle phase | Y | inbox: go to project; Owner block: cursor to first stop (`G` last) | toggle | toggle | - |
| Inbox merged TEST row (UAT+VERIFICATION, `parts.len() > 1`) | Y (entry group's run) | R `"{n} files behind this row: expand it and press Enter on a file line"` | R `"{n} files behind this row: press g to pick a file in the Owner block, then A"` | Y (D-03) | Y entry checklist | Y | toggle | toggle | - |
| FIX group | Y `/gsd-plan-phase N --gaps` | Y | Y | R `"Cannot settle a FIX row: it needs code; v plans the fix"` | Y | as above | toggle | toggle | - |
| DECIDE: CONTEXT questions | R `"No command for this row"` | Y | Y `context_questions` | R `"Cannot settle a DECIDE row: it is not a check"` | Y | as above | toggle | toggle | - |
| DECIDE: handoff / checkpoint | Y `/gsd-resume-work` | Y | R `"Cannot acknowledge a handoff: gsd-core has no category for it"` (checkpoint analog) | R (DECIDE) | Y | as above | toggle | toggle | - |
| Debug investigation (DECIDE if awaiting, else FIX) | Y `/gsd-debug` (never `--ws`) | Y | Y `debug_sessions` (`--slug`) | R by lane | Y | as above | toggle | toggle | - |
| BLOCKED aggregate (per ws+phase) | R `"No command for BLOCKED work: unblock it first"` | Y if 1 file, else R multi-file | R `"Cannot acknowledge a BLOCKED row: gsd-core has no category for it"` | Y settle phase | Y every group of (ws, phase) | as above | toggle | toggle | - |
| Any group with `counts.unreadable > 0` | R `"No command: the source file is unreadable"` | depends on read | Y (no unreadable check in `ack_target_for`) | R `"Cannot settle: the source file is unreadable"` | Y | as above | toggle | toggle | - |
| Row inside open Old debt fold (archived) | R `"No command for this row"` (run is None) | Y | Y `--archived-milestone` | R `"Cannot settle old debt: an archived phase is not reachable by /gsd-verify-work"` | Y | as above | toggle | toggle | - |
| Row inside open Acknowledged fold | Y if the group has a run (TEST/FIX keep one) | Y | R `"Already acknowledged: {name}"` | R `"Cannot settle an acknowledged file: it is already handled"` | Y | as above | toggle | toggle | - |
| Row inside open Stale fold (inbox only) | as current row | as current | as current | as current | Y entry | Y | toggle | toggle | - |
| Check line (`SubStop::Item`) | acts on parent row | parent (top of file) | parent | Y settle THIS check | parent | inbox: parent's project; Owner block: first stop | open viewer at check | collapse parent | - |
| `file PATH` line (`SubStop::File`) | parent | parent (multi-file still R) | parent | Y settle whole phase (no item cursor) | parent | as check line | open viewer at group's place | collapse parent | - |
| Detail/gap/`... N more`/inbox `run CMD` lines | not cursor stops (`item_lines` pushes stops only for items, `owner_block.rs:1219-1220`; file lines 1288-1290) | | | | | | | | |
| Fold line, sub-cursor Old debt | R `"Select a group row to run its command"` | R select | R select | R `"Select a check or a phase row to settle it"` (inbox) / `"Select a check or a group row to settle it"` (Owner block) | inbox: copy fleet Old debt checklist; Owner block: R `"Select a group row to copy its checklist"` | inbox R select / Owner block first stop | open/close fold | open/close fold | move between folds |
| Fold line, Acknowledged / Stale | R select | R | R | R | inbox: R `"Nothing to copy: acknowledged items are already handled"` / `"Nothing to copy: open Stale and copy one phase"` | | open/close | open/close | move |
| Fold line, Triage/Todos (inbox only) | R | R | R | R | R `"Nothing to copy: todos are only counted here"` | | R `"Todos are only counted here: see gsd-meta-manager inbox --include todos"` | same | move |
| Pending `settling...` row (quick-261118-v6l) | unchanged (badge is draw-only) | unchanged | unchanged | unchanged: reopens the chooser | unchanged | | unchanged | | |
| Lingering `done` row | not a stop: "Lingering rows are draw-only in both hosts: no stop, no expansion, no key reaches them" (v6l SUMMARY) | | | | | | | | |

Sources: inbox arms `inbox.rs:1980-2010` (Enter), `2015-2044` (toggle, Todos 2029-2033), `2048-2065` (Left/Right: `if at < rows.len() { return; }` swallows on rows), `2069-2088` (o), `2092-2126` (A, merged refusal 2103-2112), `2131-2170` (x, fold refusals 2140-2157, item 2159), `2190-2220` (y), `2228-2252` (g), `2256-2276` (v). Owner block arms `detail.rs:2258-2299`; handlers `owner_settle` 2336-2371, `owner_copy` 2377-2413 (fold-line refusal 2392-2394), `owner_ack` 2419-2460, `owner_open` 2465-2497, `owner_open_sub` 2504-2557, `owner_run` 2563-2595. Shared refusal tables `owner_actions.rs:60-102` (run_plan), `214-235` (open_target), `464-563` (ack_target_for, kinds 478-496), `704-721` (fold_copy); `owner_settle.rs:252-347` (settle_target_for, lane/scope refusals 266-289). Folds: Owner block selects only `Fold::OldDebt` and `Fold::Acknowledged` (`owner_block.rs:764-773`); inbox adds `Fold::Stale` and `Fold::Todos` (`inbox.rs:334-350`). Pending gating: `settle_pending` is referenced only from render code (`inbox.rs:1015, 2466`; `owner_block.rs:1597-1671`), never from a key arm. [VERIFIED: read]

**Differences between screens and verdict:**
1. `g` = go to project (inbox) vs first stop (Owner block, `detail.rs:2269-2271`, with `G` last). Justified: you are already in the project. Must be the documented exception; the Owner block menu never offers `g`.
2. Merged rows exist only in the inbox (`inbox_sources` vs `current_sources`, `owner_block.rs:538-690`), so o/A refusals on them are a row-kind difference, not a screen exception. Justified.
3. `y` on the Old debt fold: inbox copies the fleet checklist, Owner block refuses. Not obviously justified (Owner block could copy that project's old debt). Fold line is footer-only (not in the menu), so for this task: footer must simply not list `[y]` on the Owner block fold line. See Open Questions.
4. x fold-line refusal wording differs ("phase row" vs "group row"). Cosmetic; unify while extracting refusal strings.

## Footer, help, glyph, mouse, popup facts

- **Inbox footer:** `FooterKey`/`FOOTER_KEYS` ladder `inbox.rs:1415-1503`, `fold_footer` 1506-1535, `footer_rung` 1538-1565, `footer_line` 1570-1592; a live `ctx.status_message` REPLACES the footer (1571-1575). Rungs pinned verbatim by `footer_ladder_matches_the_ui_spec` (`inbox.rs:3507-3531`), e.g. rung3 `"  [j/k]move  [Enter]expand  [g]oto  [v]run  [o]pen  [A]ck  [y]copy  [?]help"` (75 cells, the 80-col form).
- **Owner block footer:** `owner_block_footer_spans(no_phases)` `detail.rs:12287-12305`, fixed, no width input; chosen at `detail.rs:6988-6994`. Pinned by `the_footer_names_the_owner_block` (`detail.rs:38419-38443`): `"  [j/k]move  [Enter]expand  [\u{2190}/\u{2192}]folds  [Esc]phases  [q]uit  [?]help"`. Detail status messages go to the tab-bar title, not the footer (`detail.rs:1342-1363`).
- **Reusable width mechanism:** `FooterToken { spans, cells, drop_priority }` + `driver_footer_spans` drop loop (`detail.rs:11947-12058`): drops priority 3, then 2, then 1, rightmost first, never splits a token, `[?]help` appended after the loop so it is never dropped. `FooterToken::hint` takes `&'static str`; a command label needs an owned-`String` constructor.
- **Help:** `row(key, desc)` formats `"  {key:<13} {description}"` (`help.rs:294-296`); Owner block rows `help.rs:447-455`, Inbox rows `611-630`; owner rows wrap at `HELP_ROW_MAX = 76` (`help.rs:674`). tki grammar `key_tokens` (`help.rs:936-1008`): whitespace/`/`-separated pieces; single chars, `Enter Esc Tab Up Down Left Right Home End Space PgUp PgDn Shift+Tab`, `Shift+X`, `Ctrl+X`, `d-d`. Exempt first words `NON_KEY_LABELS = ["Click","Double-click","Wheel","Shift+drag","Stale","Labels"]` (1012-1019). Guarding tests: `every_key_row_names_a_key_or_is_explicitly_exempt` (2704), `key_tokens_covers_every_spelling_the_help_uses` (2664), `help_lines_documents_every_key_this_phase_binds` (1258; asserts `key.chars().count() <= 13` at 1393), `help_documents_the_owner_block_keys` (1436). So a new row key like `Space / Right` (13) or `Right / l` tokenizes; `->` or raw arrows do not.
- **Glyphs:** production code under `src/ui/` may draw only registry glyphs, STRUCTURAL `0x2500..=0x259F` (box borders fine for a menu `Block`) and TYPOGRAPHY `[0x00B7, 0x2013, 0x2014, 0x2019, 0x201C, 0x201D, 0x2026, 0x2190, 0x2191, 0x2192, 0x2193]` (`glyphs.rs:318-324`, scan test 509). Use ASCII `>` as the menu selection marker (the Owner block cursor already does, `fold_line_at` `inbox.rs:1762-1766`).
- **Mouse:** inbox has no `handle_mouse` and README says "The screen is keyboard-only." (README.md:427). Owner block clicks select nothing (`a_click_in_the_owner_block_selects_no_phase`, `detail.rs:37197`). `Screen::handle_mouse` doc: "modal, overlay and text-entry screens rely on that default: a click can never confirm a dialog" (`mod.rs:321-323`). Verdict: the CONTEXT condition for mouse is not met on the row surfaces; the menu should ignore mouse. BUT `DetailScreen::handle_mouse` (`detail.rs:7031-7066`) handles tab/list clicks, so while the menu is open it must swallow (or close-and-swallow) every mouse event.
- **Popup patterns:** popups draw a fresh host as backdrop (`settle_popup.rs:42-49` `render_host`: `InboxScreen::new().render(...)` / `DetailScreen::new(alias).render_main_only(...)`), then `confirm_popup::dim_background` (166-170) and a centered rect (`popup_rect` 177-187). List-with-cursor precedent: `SettlePhaseScreen::render` (`settle_popup.rs:283-343`: `> ` + `bold().fg(Color::Cyan)` on the pick). Letter chooser precedent: `SettleChooserScreen` (`settle_popup.rs:56-134`).
- **How A asks:** `ack_open` pushes `AckConfirmScreen` (default No) after `ack_request` resolved launcher + pointer file (`owner_actions.rs:897-962`); x pushes `SettleChooserScreen` via `settle_open` (`owner_settle.rs:608-619`), later popups use `ScreenAction::Replace` (WR-03).
- **Space budget:** Owner block height capped at 40% of the tab, min 4 (`block_height`, `owner_block.rs:1751-1759`); at 80x24 that is about 8 rows including header and fold line.

## Three solutions

**A. Context-aware footer only.** Footer lists the selected row's available actions from the model (`[v]run /gsd-verify-work 16  [x]settle  [o]pen  [y]copy  [A]ck  [Space]expand ...`), Enter unchanged. Pros: smallest diff, no Enter migration, fixes today's three untruthful footers. Cons: at 80 columns the drop ladder must cut 2-4 actions (the inbox rung that fits 80 today already omits `x`), so secondary actions and every refusal reason stay invisible; no room for the blocked-with-reason requirement; does not deliver the owner's stated "Enter opens a context menu".

**B. Footer (A) + Enter opens a which-key menu (recommended).** Enter on a group row opens an overlay listing every action for that row: key, label, one-line detail (the exact command, or the dim reason). First available entry preselected (v when runnable). `j/k`/arrows move, Enter or the letter executes, Esc/`q` closes. Enter on check/file line: viewer (locked); Enter on fold line: toggles the fold (INFERRED: a fold is navigation, not an item). Space toggles everywhere (already true in both screens: `inbox.rs:2327-2330`, `detail.rs:2284-2286`); Right/`l` expand and Left/`h` collapse on rows and their sub-stops, where both keys are currently swallowed (`inbox.rs:2053-2055`, `detail.rs:2287-2288, 2298`); on the fold line Left/Right keep moving between folds. Pros: complete and discoverable, carries reasons, fits 80x24 (at most 7 entries + 2 border rows), footer can stay short because the menu holds the long tail. Cons: Enter-expands muscle memory changes (mitigated: Space, Right, and an "expand" menu entry); about 20 tests press Enter to expand and need Space; one extra keypress for menu-driven use (hotkeys unchanged).

**C. Inline action strip under the selected row** (chips, Left/Right to pick, Enter to run). Rejected: it costs one body row in an Owner block that has about 6 body rows at 80x24; it re-flows every row below on each cursor move, which breaks the identity-based scroll math (`follow_offset` `owner_block.rs:1770-1800`, inbox `scroll_for`/`scroll_to_item`); Left/Right would mean chip-pick on rows but fold-step on the fold line and could no longer serve expand/collapse; and it duplicates the footer.

**Recommendation: B**, because it is the only option that meets "truthful at 80 columns" AND "nothing hidden": the footer may drop low-priority hints at 80/100 while the menu always lists every action with its reason, at a fixed small cost (one popup, one Enter rebind).

## Shared implementation sketch (prescriptive)

**Module `src/ui/screens/owner_row_actions.rs`** (new; `owner_actions.rs` is already 2213 lines). Pure, no I/O, callable from render:

```rust
// Sketch: names are proposals [ASSUMED], not existing code.
pub(crate) enum Host { Inbox, OwnerBlock }
#[derive(Clone, Copy, PartialEq, Eq, Hash, Debug)]
pub(crate) enum ActionId { Run, Settle, Open, Copy, Acknowledge, GoToProject, Toggle }
pub(crate) enum Availability { Ready, Blocked(String) } // reason = the exact text the hotkey prints
pub(crate) struct RowAction {
    pub id: ActionId,
    pub key: crossterm::event::KeyCode,  // the hotkey the menu re-dispatches
    pub key_label: &'static str,         // "v", "x", "o", "y", "A", "g", "Space"
    pub label: &'static str,             // shared wording, e.g. "run", "settle", "open file"
    pub detail: String,                  // "/gsd-verify-work 16" (through `shown`) or ""
    pub priority: u8,                    // footer drop priority (1 kept longest)
    pub availability: Availability,
}
/// What the cursor rests on, resolved by each host from its own cursor identity.
pub(crate) struct RowTarget<'a> { alias: &'a str, src: &'a RowSrc, fold: Option<Fold>, sub_stop: Option<SubStop> }
/// Actions only one host offers. Every entry needs a reason here.
/// `GoToProject`: the inbox lists every project; in the Owner block you are already in it and `g` means "first row".
pub(crate) const INBOX_ONLY: &[ActionId] = &[ActionId::GoToProject];
pub(crate) const OWNER_BLOCK_ONLY: &[ActionId] = &[];
pub(crate) fn row_actions(ctx: &AppContext, host: Host, t: &RowTarget) -> Vec<RowAction>;
```

Rules for `row_actions`: hide when the lane/kind/scope rule refuses forever (v on BLOCKED or `run: None`; x on FIX/DECIDE/archived/acknowledged; A on BLOCKED/handoff/checkpoint/deferred/acknowledged); `Blocked(reason)` for instance-specific pure refusals (unreadable file, multi-file `o`, merged-row `A`, unsafe token). Reuse the pure halves: `owner_actions::run_plan` is pure and already runs on the inbox render path (`expansion_for`, `inbox.rs:1136-1144`); it also yields `live_session`, so the v label can read "switch to live session, type CMD". Do NOT call `ack_request`, `resolve_draft`, `active_workstream_is_set` or `resolve_launcher` from the model: they read files/PATH (`owner_actions.rs:917-925`, `owner_settle.rs:495`) and OWN-12 forbids I/O on the render path. Those environment refusals stay key-time only (the action is listed; the key prints the reason). Extract every refusal string the model repeats into named consts shared with the key arms, so model and hotkey cannot drift.

**Footer builder** in the same module: `footer_spans(actions, nav: &[NavToken], width) -> Vec<Span>`; move `FooterToken` out of `detail.rs` (keep the driver footer using it) and add an owned-label constructor. Two passes: long labels (with command) first, then short labels, then drop by priority; `[?]help` appended last, never dropped. Context-specific Enter token: `[Enter]actions` on rows, `[Enter]open here` on sub-stops, `[Enter]open/close` on the fold line. The fold-line footer depends on the sub-cursor fold (Todos: `[←/→]fold` only; `[y]copy` only on inbox Old debt).

**Menu:** `pub(crate) struct ActionMenu { target_key: (alias, GroupKey, Option<SubStop>), selected: ActionId }` held as `menu: Option<ActionMenu>` on `InboxScreen` and `DetailScreen`. Entries are recomputed from `row_actions` each frame (never snapshotted); if the cursor identity no longer resolves, close the menu. Overlay: `Clear` + bordered `Block`, width `min(area.width - 2, 64)`, anchored below the cursor row, flipped above when it does not fit, clamped to the frame; labels cut with `fit_cells`; blocked entries DIM plus a parenthesized reason (text, not color only). Keys while open, intercepted FIRST (inbox: top of `handle_key`; Owner block: top of `handle_owner_block_key`, before the `q`/digit arm at `detail.rs:2211-2223`): `j/k/Up/Down` move over Ready entries, `Enter` executes selected, a plain letter (or Space) executes that entry, `Esc`/`q` close, everything else swallowed (so `1`-`4`/`a` filters and tab digits do not fire under the menu).

**Exact hotkey path:** execution = `self.menu = None;` then `return self.handle_key(entry.key, KeyModifiers::NONE, ctx)` (Owner block: `self.handle_owner_block_key(...)`). x therefore still pushes the chooser, A still pushes the default-No popup, and the confirm backdrop (fresh host instance) never shows the menu.

**Required consistency test (sketch):** for each fixture row kind that both screens show identically (single-file TEST, FIX, CONTEXT DECIDE, debug, BLOCKED, Old debt, Acknowledged, plus check and file sub-stops): resolve the same `GroupKey` in both hosts, build `row_actions(Host::Inbox)` and `row_actions(Host::OwnerBlock)`; assert for every `id` in both: equal `key`, `key_label`, `label`, `detail`; assert `ids(inbox) XOR ids(owner) == INBOX_ONLY ∪ OWNER_BLOCK_ONLY` restricted to actions applicable to the row. Exclude merged rows (inbox-only by data, D-08). Companion tests: footer keys at 140 columns == Ready+Blocked-listed keys of the model (truthfulness); footer at 80 and 100 fits and ends with `[?]help`; menu-vs-hotkey equivalence per action (same `ScreenAction` variant/pushed `name()`, same `status_message`, same recorded launch/copy).

## Tests that encode Enter semantics or footer text

- `inbox.rs`: `footer_ladder_matches_the_ui_spec` (3507), `layout_tiers_140_120_100_80` (3382, footer asserts 3475-3502), `the_inbox_renders_sections_in_order_at_100x30` (3030, asserts `[Enter]expand`), `enter_expands_a_group_with_its_run_line` (4546), `a_group_without_a_command_has_no_run_line_and_a_blocked_row_lists_its_files` (4630), `expansion_survives_a_refresh_and_a_vanished_group_is_dropped` (4663), `the_fold_line_walks_and_opens_old_debt_and_acknowledged` (4732), `triage_never_expands` (4843), `the_scroll_keeps_the_cursor_and_its_section_title_in_view` (4370), `item_walk_visits_the_drawn_items_in_order` (5837), `merged_rows_walk_parts_in_order` (5947), `row_keys_on_an_item_act_on_the_parent` (5984), `the_selected_item_stays_in_view` (6101), `enter_on_a_check_line_opens_the_viewer_at_that_check` (6134, first Enter expands), `enter_on_a_file_line_opens_the_file_at_the_groups_first_check` (6195), `enter_on_a_merged_rows_second_file_line_opens_that_file` (6233), `a_selected_file_line_changes_no_text_and_stays_in_view` (6269), `row_keys_on_a_file_line_act_on_the_parent` (6332), `a_vanished_file_line_falls_back_to_its_row` (6377), `a_vanished_item_falls_back_to_its_row` (6399), `a_hostile_drilled_in_inbox_has_no_raw_control_cell_and_nothing_past_the_width` (5230), `a_same_phase_pair_is_one_row` (5355), `y_on_a_merged_row_copies_the_entry_checklist` (5470), `labels_are_dim_and_never_move_a_row` (5751), `o_on_a_blocked_row_over_two_files_points_at_g` (5078).
- `detail.rs`: `the_footer_names_the_owner_block` (38419), `enter_expands_and_collapses_a_group` (38274), `sc4_an_expanded_group_shows_its_checks_beside_the_collapsed_rows` (38338), `owner_block_enter_on_a_check_or_file_line_opens_the_viewer_there` (37954), `owner_block_swallows_action_keys` (37518: asserts Left/Right are swallowed on rows; changes if they expand/collapse), `the_fold_line_walks_and_opens_old_debt_and_acknowledged` (38465), `owner_block_state_survives_a_digest_refresh` (38528), `an_expanded_long_text_block_stays_inside_the_block` (38626), `a_long_text_digest_stays_inside_the_block` (37230).
- `settle_popup.rs`: Owner block/inbox flows that expand with Enter before `j` onto a check: `the_owner_block_check_flow_launches_one_check_and_writes_nothing` (1205), `x_on_an_item_targets_one_check` (1280), `j_past_the_last_item_of_the_last_stop_keeps_the_item` (1324), `the_flow_matrix_launches_every_expected_prompt_and_writes_nothing` (2521). Most of the 40 `KeyCode::Enter` uses in this file are popup Enters and stay.
- `help.rs`: `help_documents_the_owner_block_keys` (1436), `help_lines_documents_every_key_this_phase_binds` (1258).
- The list above is the candidates found by grep plus reads. Confirm with a full `--no-fail-fast` run after the rebind: any test that presses Enter on a group row to expand switches to `KeyCode::Char(' ')`.

## Common Pitfalls

1. **`v` in tests launches for real.** The hotkey arm passes `owner_actions::launch_real` (`inbox.rs:2318`, `detail.rs:2310`), and existing tests avoid pressing `v` through `handle_key`. A menu-vs-hotkey test for `v` needs a cfg(test) recording seam inside `launch_real`, mirroring `launch_settle_real`/`take_settle_launches` (`owner_settle.rs:824-846`).
2. **I/O on the render path.** Footer and menu availability must not call `ack_request`/`resolve_draft` (pointer file, PATH probe). Keep environment refusals key-time only.
3. **Interception order.** In the Owner block, `q` pops the whole screen and digits hand focus to Content before the main match (`detail.rs:2211-2223`); the open menu must intercept above that. In the inbox, `1`-`4`/`a` would re-filter under the menu.
4. **Mouse under the menu.** `DetailScreen::handle_mouse` would switch tabs or select phases under an open menu; swallow while open.
5. **Stale menu after a refresh.** Digests refresh asynchronously; recompute entries each frame by identity and close when the target vanishes. Execution is safe anyway because the hotkey path re-resolves by identity.
6. **Inbox status line hides the footer** for its lifetime (`inbox.rs:1571-1575`); footer-truthfulness tests must clear `status_message`.
7. **Render-escape census.** Menu text includes phase tokens and workstream names: escape through `shown`. Hosts are adjudicated `RENDERS_ATTACKER_INFLUENCED_IDENTITY` (e.g. `inbox.rs:2285-2294`); add a menu-open fixture state to their probe so the hostile-digest render test covers the overlay. A separate pushed `Screen` would additionally need `adjudicate_screen!` (`mod.rs:282-296`) and a census row, which is one more reason to keep the menu embedded.
8. **Help key column.** Keys must tokenize under `key_tokens` and be at most 13 chars; descriptions must stay unique in the body (whole-row assertions).
9. **Chorded letters.** Menu letters must honor `is_plain_action_key` (Ctrl-V paste reflex, WR-01); `A` arrives with SHIFT and is allowed.
10. **Pending settle.** The `settling...` badge does not gate `x` today. Listing `x` there is truthful; do not invent a block without an owner decision.

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|---|---|---|
| A1 | Enter on the fold line keeps toggling the fold (no menu) | Solutions / B | Owner may expect the menu there too; small rework |
| A2 | Right/`l` expand, Left/`h` collapse on rows and sub-stops; fold line unchanged | Solutions / B | Key conflicts are none today (verified swallowed), but the owner may want Space only |
| A3 | Menu ignores mouse (row surfaces are keyboard-only) | Footer facts | CONTEXT's mouse clause reads "if the screen supports mouse elsewhere"; the detail screen does, the Owner block rows do not |
| A4 | Hide = lane/kind/scope rule; dim = instance-specific pure refusal | Shared sketch | Classification boundary is a judgment call |
| A5 | Type and module names in the sketch | Shared sketch | Naming only |

## Open Questions

1. **Owner block `y` on the Old debt fold** refuses while the inbox copies (difference 3). Recommendation: leave behavior; footer omits `[y]` there; record as a follow-up todo.
2. **x on a `settling...` row** reopens the chooser. Recommendation: keep; optionally show "settling..." in the menu detail text (no block).
3. **docs/GETTING-STARTED.md has no Owner section today** (grep for "owner"/"inbox" found nothing). Recommendation: add one short paragraph naming Enter (menu), Space/Right (expand); README rows 310, 340, 411-418, 433 need the rebind.

## Environment Availability

| Dependency | Required By | Available | Version |
|---|---|---|---|
| cargo / rustc | build, tests | yes | 1.99.1 |
| ratatui | `Clear`, `Block`, `Paragraph` | yes (Cargo.toml) | 0.30 |

No new crates. Package Legitimacy Audit: not applicable (nothing installed). `workflow.nyquist_validation` is `false` in `.planning/config.json`, so the Validation Architecture section is omitted; test gates are the CONTEXT ones.

## Security Domain

Applicable: V5 input validation only. The model must keep using the typed `RunCommand` (never the display string) and the existing `is_command_token`/`is_workstream_name` re-checks in `run_plan`; menu/footer text containing project-derived tokens goes through `shown`. No new process spawn or file write paths: execution re-dispatches existing key arms.

## Project Constraints (from CLAUDE.md)

Must not need Claude/GSD to read state; non-intrusive; portable; logs to file only; `tokio::sync::Mutex` across awaits (not relevant here). Release steps apply only at milestone tagging.

## Sources

Primary (HIGH, read this session): `src/ui/screens/inbox.rs`, `detail.rs`, `owner_actions.rs`, `owner_settle.rs`, `owner_block.rs`, `settle_popup.rs`, `confirm_popup.rs`, `ack_confirm.rs`, `help.rs`, `mod.rs`, `src/ui/glyphs.rs`, `src/state_reader/owner_items/{mod,rules,triage}.rs`, `README.md`, `.planning/quick/261118-v6l-*/261118-v6l-SUMMARY.md`, `.planning/config.json`. No web sources needed (in-repo design question, no external library choice).

## Metadata

- Action table / differences: HIGH (every arm read).
- Footer/help/glyph constraints: HIGH.
- Recommendation and sketch: MEDIUM (design judgment; names ASSUMED).
- Valid until: next change to the owner key arms.
