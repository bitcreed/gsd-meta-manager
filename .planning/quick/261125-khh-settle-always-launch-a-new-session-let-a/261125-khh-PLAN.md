---
phase: quick-261125-khh
plan: 01
type: execute
wave: 1
depends_on: []
quick_id: 261125-khh
mode: quick
files_modified:
  - src/ui/screens/owner_settle.rs
  - src/ui/screens/settle_popup.rs
  - src/ui/screens/settle_pending.rs
  - src/ui/screens/owner_actions.rs
  - src/ui/screens/mod.rs
  - src/app.rs
  - src/ui/screens/owner_row_actions.rs
  - src/ui/screens/inbox.rs
  - src/ui/screens/detail.rs
  - src/ui/screens/render_escape_guard.rs
  - tests/focus_entry_guard.rs
  - tests/owner_settle_guard.rs
  - tests/owner_settle_oracle.rs
  - README.md
  - docs/GETTING-STARTED.md
autonomous: true
requirements: [QUICK-261125-khh]

estimate:
  tokens: 180000
  raw_tokens: 180000
  tasks: 3
  confidence: low

must_haves:
  truths:
    - "With a live Claude session in the project, y in the settle popup launches /gsd-verify-work N plus the instruction in a NEW multiplexer session; it never switches focus and never writes the clipboard (no OSC 52 sequence, no native copy tool) (D-01)"
    - "When a live session exists the settle popup says a new session starts beside it; without one it keeps the existing new-session footer (D-02)"
    - "A session that starts or ends between opening the popup and pressing y no longer refuses the launch (D-03)"
    - "v keeps its 37 D-12 behavior: it focuses a live session and shows the command to type"
    - "x on a row whose settle is pending is an ENABLED stop waiting entry in the footer and the action menu; it opens a default-No confirm; Yes drops the pending entry from memory and x then opens the settle chooser again; No, Enter on the default and Esc change nothing (D-04)"
    - "A pending settle is never dropped by itself: no timeout, no stall state, no stall wording on the badge (D-05)"
    - "With a reason, the instruction asks GSD to record the reason Deferred follow-up: <outcome words> Owner’s note: “““<note>””” exactly, and to copy the note between the triple quotes verbatim; the note can never contain a typographic double quote, so typed triple quotes cannot close it early (D-06, D-07)"
    - "With no reason, every prompt is byte-identical to the prompt at HEAD (D-08)"
    - "The installed gsd-core 1.23.0 verify-work argument parse takes every new prompt with no xargs warning and no foreign workstream, and audit-uat plus uat-passed count the quoted reason as a non-blocking deferral (tests/owner_settle_oracle.rs green with the oracle present)"
    - "The settle modules still write no file and start no process, and no prompt contains an ASCII apostrophe, ASCII double quote or backslash"
  artifacts:
    - path: src/ui/screens/owner_settle.rs
      provides: "always-new-session launch; recorded_reason, owner_note, delimited_note and the NOTE_OPEN / NOTE_CLOSE delimiters"
      contains: "pub fn recorded_reason"
    - path: src/ui/screens/settle_popup.rs
      provides: "SettleStopScreen (stop waiting confirm) and the beside-the-live-one footer"
      contains: "SettleStopScreen"
    - path: src/ui/screens/settle_pending.rs
      provides: "PendingSettles::stop_waiting, memory only"
      contains: "fn stop_waiting"
    - path: src/ui/screens/owner_row_actions.rs
      provides: "x entry labelled stop waiting on a pending row"
      contains: "stop waiting"
    - path: tests/owner_settle_oracle.rs
      provides: "oracle round trip of the quoted reason built by the product"
      contains: "recorded_reason"
  key_links:
    - from: src/ui/screens/settle_popup.rs
      to: src/ui/screens/owner_settle.rs
      via: "SettleConfirmScreen::confirm calls owner_settle::launch_and_track with no live-session branch"
    - from: src/ui/screens/owner_settle.rs
      to: src/multiplexer
      via: "launch_settle_real always calls crate::multiplexer::launch, never focus_message"
    - from: src/ui/screens/inbox.rs
      to: src/ui/screens/settle_popup.rs
      via: "InboxScreen::settle_key pushes SettleStopScreen on a pending row"
    - from: src/ui/screens/detail.rs
      to: src/ui/screens/settle_popup.rs
      via: "DetailScreen::owner_settle pushes SettleStopScreen on a pending row"
    - from: src/ui/screens/owner_row_actions.rs
      to: src/ui/screens/settle_pending.rs
      via: "row_actions reads PendingSettles::for_row to label x as stop waiting"
    - from: tests/owner_settle_oracle.rs
      to: src/ui/screens/owner_settle.rs
      via: "prompt and recorded_reason run verbatim through verify-work's parse and gsd-tools audit-uat"
---

<objective>
Fix the three problems the owner hit with Owner-row settle (`x`, then `w`/`p`/`m`, then `y`) in
real use on project mailbot phase 5:

1. `y` always launches a NEW session, never focuses a live one, never touches the clipboard (D-01..D-03, D-10).
2. A pending settle can be dropped: `x` on a settling row becomes "stop waiting" (D-04, D-05).
3. The owner's reason is delimited in typographic triple quotes and copied verbatim (D-06..D-09).

Purpose: the owner was sent into a busy worker with an overwritten clipboard; a pending badge
could only be cleared by restarting the app; and the free-text reason ran into the prompt so
the agent could not tell the owner's words from the instruction.

Output: three atomic commits, updated README.md and docs/GETTING-STARTED.md, green clippy and
full test suite, oracle test green against the installed gsd-core 1.23.0.
</objective>

## Decisions

Recorded here only; the phase 37 and 39 CONTEXT files are NOT edited.

- **D-01 [owner]:** Settle's `y` ALWAYS launches `/gsd-verify-work N <instruction>` in a new
  session through the multiplexer, even when the project has a live Claude session. It never
  switches focus and never writes the clipboard. This reverses 39 D-11 ("Live session present
  (37 D-12 path): focus it and copy the full command line to the clipboard ... no second session
  is started"), which inherited 37 D-12 ("nothing is injected into the session"). Owner's
  rationale: "we don't want to mess with the context of an existing session"; in real use the
  owner was sent to a worker that was already busy and the clipboard was overwritten without a
  clear warning. 39 D-11's stated risk (two GSD sessions in one project could race on STATE.md)
  is accepted by the owner; the popup discloses that a second session starts (D-02). `v` keeps
  37 D-12 (focus-or-launch) unchanged: no shared code forces a change.
- **D-02 [inferred wording]:** With a live session the popup's footer reads
  `Yes starts a new session beside the live one · this app writes nothing` (70 cells, the same
  width as the footer it replaces, so the 80x24 fit tests keep their row counts). Without one the
  existing new-session footer stays.
- **D-03 [inferred]:** The Yes recheck no longer refuses when a session started or ended since
  the popup opened: Yes does the same thing either way, so that refusal only adds friction.
- **D-04 [inferred]:** `x` on a row with a not-done pending settle is an ENABLED action labelled
  `stop waiting` (menu label and footer hint), detail `settle may still run; clears the badge here, asks first`.
  It opens a default-No confirm (the `confirm_popup` pattern, Yellow): title
  `Stop waiting for this settle?`, body Project and Settle (the badge words), then
  `It may still be running in its session. Yes only clears the badge here: nothing is stopped or written, and x settles the row again.`
  Yes drops every not-done pending entry covering the row (memory only) and sets the status
  `Stopped waiting for the settle in <alias>; x settles the row again` (or
  `That settle is no longer pending; nothing changed` when none was left). No and Esc pop and
  change nothing. A menu entry runs exactly what the key does.
- **D-05 [inferred, honoring the owner's 261118-v6l decision "Stall handling was NOT chosen:
  build no timeout, no stall state, no warning"]:** no stall hint on the badge and no automatic
  drop. The badge keeps counting; stop waiting (D-04) is the escape.
- **D-06 [owner suggested triple quotes; typographic form inferred]:** The note is delimited by
  `“““` (U+201C x3) and `”””` (U+201D x3). No ASCII quote anywhere, because verify-work parses its
  arguments with `sed | xargs` (which is why `clean()` maps ASCII apostrophe, double quote and
  backslash to `’`, `”` and `/`). After `clean(reason, REASON_MAX_CHARS)`, every `“` in the note becomes
  `‘` and every `”` becomes `’` (this covers ASCII double quotes, which `clean` already turned into
  `”`), so the note can never contain the delimiter.
- **D-07 [inferred]:** The recorded reason keeps the delimiters, so the UAT file keeps the
  distinction. Its value is exactly `Deferred follow-up: <outcome words> Owner’s note: “““<note>”””`
  (outcome words: `will not do.`, `deferred permanently.`, `moved to phase M.`). It still starts with
  `Deferred follow-up:` (gsd-core's DEFERRED_PREFIX non-blocking rule). No period is appended
  inside the note: verbatim means verbatim. Right after it the instruction says:
  `Record that reason exactly, up to and including the closing triple quotes, with the owner’s note between them copied verbatim.`
  Checked by the planner against the installed gsd-core 1.23.0: `audit-uat` returns such a reason
  verbatim with `category: deferred`, and `phase uat-passed 1 --uat-only` reports it as
  `passing: true, deferred: true`.
- **D-08 [inferred]:** With no reason, the note and the verbatim sentence are both omitted, so the
  output is byte-identical to today's. Move gets the same note. The Move todo title does NOT carry
  the reason (checked: it is built from phase M's number and name, the check ref and phase N), so it
  stays unchanged.
- **D-09 [inferred]:** The popup's Reason row shows the note as the instruction quotes it
  (`“““…”””`, after the quote mapping), and the instruction preview puts that quoted note in bold.
- **D-10 [inferred]:** The live-session branch leaves dead code that only settle used (added
  for 39-03 in commit 5180ffb0): `owner_actions::copy_led`, both cfg variants of
  `start_native_copy_led`, `AppContext::start_copy_led`, the `lead` parameter of
  `start_copy_led_with` (folded back into `start_copy_with`), `ScreenAction::PopWriteOsc52` and its
  App arm, `live_settle_lead`, the old live footer constant, and their tests. All of it is deleted.
  `owner_settle.rs` leaves the focus-caller allowlist of `tests/focus_entry_guard.rs`.

<execution_context>
@~/.claude/gsd-core/workflows/execute-plan.md
@~/.claude/gsd-core/templates/summary.md
</execution_context>

<context>
@CLAUDE.md
@.planning/STATE.md

Read only the ranges each task names; the source files are large (settle_popup.rs ~2660 lines,
owner_settle.rs ~1850, owner_row_actions.rs ~1930, settle_pending.rs ~1130, inbox.rs and
detail.rs are far larger). Use grep to find a named function, then Read with offset/limit.

Environment rules for every task:
- Work in this worktree on branch feat/v2.5.0-owner-inbox. Do not push.
- Commit each task atomically with a conventional, path-scoped commit
  (`git commit -m ... -- <paths>`), never `git add -A`. End each message with the session's
  attribution lines.
- US spelling. The app never writes GSD files; nothing here writes into a project.
- RTK filters command output. Run cargo through `rtk bypass` and never pipe cargo output into
  grep or tail (a downstream filter can fake pass counts). To inspect results, redirect to a log
  in the scratchpad and read the `test result:` lines from it with the Read tool. Every
  `test result:` line must say `0 failed`, and `--no-fail-fast` must be used so every suite runs.
</context>

<tasks>

<task type="tracer">
  <name>Task 1: End-to-end "y with a live session launches a new session", plus dead-code removal and docs</name>
  <files>src/ui/screens/owner_settle.rs, src/ui/screens/settle_popup.rs, src/ui/screens/settle_pending.rs, src/ui/screens/owner_actions.rs, src/ui/screens/mod.rs, src/app.rs, tests/focus_entry_guard.rs, tests/owner_settle_guard.rs, README.md, docs/GETTING-STARTED.md</files>
  <read_first>
    - src/ui/screens/owner_settle.rs lines 1-40 (module doc), 150-172 (SettleDraft, SettlePlan), 551-581 (resolve_draft), 693-740 (settle_recheck), 764-784 (launch_settle), 868-920 (launch_started, launch_and_track, launch_settle_real, test thread-local)
    - src/ui/screens/settle_popup.rs lines 521-600 (SettleConfirmScreen::confirm, the lead helper, the footer constants), 670-682 (footer), 1026-1100 (test Stack helper), 2097-2170 and 2367-2570 (live-session tests)
    - src/ui/screens/owner_actions.rs lines 786-915 (copy, the led copy helper and its seams, force_clipboard, seam_ran) and the tests near 2139-2290
    - src/ui/screens/mod.rs lines 375-390 (ScreenAction) and 1980-2080 (start_copy, start_copy_with and the led variants)
    - `git show 5180ffb0 -- src/ui/screens/mod.rs src/app.rs src/ui/screens/owner_actions.rs` (the commit that added the plumbing being removed: restore the pre-5180ffb0 shape of start_copy_with)
    - src/app.rs near 4005-4020 and 6885-6910 (the pop-and-write arm and its test)
    - src/ui/screens/settle_pending.rs lines 580-595 (test Stack) and 914-963 (live-session registration test)
    - tests/focus_entry_guard.rs lines 1-40 and 147-163; tests/owner_settle_guard.rs lines 1-15 and 135-150
    - README.md lines 479-501; docs/GETTING-STARTED.md lines 207-220
  </read_first>
  <behavior>
    - With a session in `ctx.active_sessions` whose working_dir equals the project path, `x w <reason> Enter y` in the inbox records exactly one launch and the status is `launched_status` of that plan (starts with `Launched /gsd-verify-work 01 in alder-api`); no OSC 52 sequence reaches the stack and the native-copy seam records nothing even with a wl-copy probe forced; the fixture tree stays byte-identical
    - The same through the Owner block host (Stack::detail, one-check scope)
    - The popup drawn with a live session contains `beside the live one` and not `copies the command`
    - A session pushed or cleared after the popup opened does not refuse: y still launches once
    - A prefix-sharing sibling directory still does not count as a live session (T-37-06)
    - settle_pending: an entry registers exactly when the launch started (the launched line or a GUI-terminal line), with or without a live session; a failed launch registers nothing
  </behavior>
  <action>
Implements D-01, D-02, D-03 and D-10.

owner_settle.rs (D-01, D-03):
- Rewrite the module-doc paragraph that says nothing is typed into a live session (37 D-12): settle ALWAYS starts a new session through the multiplexer, even when the project already runs Claude (owner decision, quick 261125-khh, reversing 39 D-11 for settle only; `v` keeps 37 D-12). It never focuses a session and never touches the clipboard.
- Keep `SettleDraft::live_session` and the exact-path lookup in `resolve_draft` (T-37-06). Re-document the field: used only for the popup's note that the new session starts beside the live one.
- `settle_recheck`: delete the started-or-ended comparison and the doc sentence about it (D-03). Keep the prompt, cwd and Move-target rechecks.
- `launch_settle`: the test arm records the plan and always returns `launched_status(plan)`. Doc: launch the plan in a new session.
- `launch_started`: drop the live-session disjunct; a launch counts only when the status is the launched line or reports a GUI terminal. Drop the doc sentences about pasting.
- `launch_and_track`: drop "(or focus the live session)" and the paste sentences from its doc.
- `launch_settle_real`: delete the focus branch; it always calls `crate::multiplexer::launch` with the existing arguments. After this the file must not reference the multiplexer's focus entry or its success prefix.

settle_popup.rs (D-01, D-02):
- `SettleConfirmScreen::confirm`: after the recheck, call `owner_settle::launch_and_track`, set `ctx.status_message` to the returned status, return `ScreenAction::Pop`. Rewrite its doc (no D-11, no clipboard). Delete the live-session status-lead helper. Drop the `owner_actions` import if it becomes unused.
- Replace the live footer constant with `FOOTER_BESIDE_LIVE` = `Yes starts a new session beside the live one \u{b7} this app writes nothing` (D-02); `footer()` picks it when `plan.draft.live_session.is_some()`, else the new-session footer. Update the constants' docs.
- Test helper `Stack::key_with`: replace the pop-and-write arm with an arm that holds any `ScreenAction::WriteOsc52(sequence)` in the second field and returns true; keep `take_osc52` (doc: any OSC 52 sequence a screen handed over), so tests can assert settle never emits one.
- Tests: delete `yes_rechecks_a_new_live_session`, `a_failed_focus_never_says_paste_there`, `a_live_session_gets_the_prompt_by_osc52`, `a_live_session_gets_the_prompt_by_a_native_tool`, `an_owner_block_live_session_copies_the_check_prompt`, and the `FOCUSED` constant. Replace `a_live_session_is_focused_not_relaunched` with `a_live_session_still_gets_a_new_session` covering the first three behavior bullets plus the sibling-directory check (force a wl-copy probe with `owner_actions::force_clipboard` before y, reset it to None after). Add `a_session_that_starts_or_ends_before_yes_still_launches`. Keep `no_live_session_copies_nothing` and the live case of `the_compact_popup_fits_80x24_for_every_outcome_and_scope`; both must pass unchanged in intent. Keep `live_flow` only if a remaining test uses it.

Dead code (D-10):
- owner_actions.rs: delete the led copy function, both cfg variants of the led native-copy seam, and test `copy_led_osc52_leads_the_status`. Keep `copy`, `force_clipboard`, `seam_ran` and the plain seam.
- mod.rs: delete `ScreenAction::PopWriteOsc52` and `AppContext::start_copy_led`. Fold `start_copy_led_with` back into `start_copy_with` without the `lead` parameter (the body as it was before 5180ffb0, messages unchanged); delete the owner_actions test `start_copy_led_with_leads_every_outcome` and keep the `start_copy_with` tests.
- app.rs: delete the PopWriteOsc52 arm in `process_screen_action` and test `pop_write_osc52_pops_and_holds_the_sequence`.
- settle_pending.rs tests: drop the PopWriteOsc52 pattern from the test Stack; rewrite `a_live_session_registers_and_a_failed_launch_does_not` per the last behavior bullet and rename it `a_started_launch_registers_and_a_failed_one_does_not`.
- Grep src/ for leftover doc comments naming the removed items or the 39-03 D-11 copy path, and fix them.

Guards:
- tests/focus_entry_guard.rs: remove `src/ui/screens/owner_settle.rs` from `FOCUS_CALLERS` (settle no longer focuses; the list must equal the real callers).
- tests/owner_settle_guard.rs: the module doc and the assertion message say launching goes through `crate::multiplexer` and the clipboard through `owner_actions`; settle no longer uses the clipboard, so say launching goes through `crate::multiplexer` only.

Docs:
- README.md: replace the sentence group starting "If the project already has a live Claude session" (around lines 492-494) with: `y` always starts a new session, even when the project already has a live Claude session: it never switches to that session or types into it, and it never touches the clipboard; the popup notes that the new session starts beside the live one.
- docs/GETTING-STARTED.md (after line 220's settle sentence): `x` always launches GSD in a new session, next to any session already running in the project.

Commit: `fix(quick-261125-khh): settle always launches a new session, never focuses or copies`.
  </action>
  <verify>
    <automated>rtk bypass cargo test --lib settle --no-fail-fast && rtk bypass cargo test --test focus_entry_guard --test owner_settle_guard && rtk bypass cargo clippy --all-targets -- -D warnings && rtk bypass cargo test --no-fail-fast</automated>
  </verify>
  <acceptance_criteria>
    - `rtk bypass grep -rE 'copy_led|PopWriteOsc52|live_settle_lead|FOOTER_LIVE' src | rtk bypass grep -vE '^[^:]*: *//' | wc -l` [HEAD=32 EXPECT==0]
    - `rtk bypass grep -v '^ *//' src/ui/screens/owner_settle.rs | rtk bypass grep -Ec 'focus_message|FOCUS_OK_PREFIX'` [HEAD=2 EXPECT==0]
    - `rtk bypass grep -v '^ *//' tests/focus_entry_guard.rs | rtk bypass grep -c 'src/ui/screens/owner_settle.rs'` [HEAD=1 EXPECT==0]
    - `rtk bypass grep -Fc 'beside the live one' src/ui/screens/settle_popup.rs` [HEAD=0 EXPECT>=1]
    - `rtk bypass grep -v '^#' README.md | rtk bypass grep -Fc 'switches to it'` [HEAD=1 EXPECT==0]
    - `rtk bypass grep -Fc 'new session' docs/GETTING-STARTED.md` [HEAD=0 EXPECT>=1]
    - clippy with `-D warnings` is clean (no dead_code left by the removals) and every `test result:` line of the full `--no-fail-fast` run says `0 failed`
  </acceptance_criteria>
  <done>With or without a live session, y launches one new session and reports the launched status; no focus switch, no OSC 52 and no native copy happen on the settle path; the popup notes the live session; the settle-only clipboard plumbing is gone; v is untouched; README and GETTING-STARTED describe the new behavior; clippy and the full suite are green; one path-scoped commit.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: x on a settling row offers "stop waiting" (default-No confirm, memory-only drop)</name>
  <files>src/ui/screens/settle_pending.rs, src/ui/screens/owner_row_actions.rs, src/ui/screens/settle_popup.rs, src/ui/screens/inbox.rs, src/ui/screens/detail.rs, src/ui/screens/render_escape_guard.rs, README.md, docs/GETTING-STARTED.md</files>
  <read_first>
    - src/ui/screens/settle_pending.rs lines 1-30 (module doc), 55-185 (PendingSettle, covers, is_done, PendingSettles::insert, for_row), 680-700 (keeps-counting test)
    - src/ui/screens/owner_row_actions.rs lines 1-30 (module doc), 70-80 (refusal constants), 139-184 (pending refusal helper, action, availability_of), 240-276 (the x block), 1270-1310 (pending test)
    - src/ui/screens/settle_popup.rs lines 20-60 (imports, bold, muted, render_host), 590-625 (LABEL_W, field, one_row), 946-1010 (SettleConfirmScreen adjudication and Screen impl, the pattern to copy)
    - src/ui/screens/confirm_popup.rs lines 1-40 and the signatures of render_confirm_popup, body_room, click_as_key
    - src/ui/screens/inbox.rs lines 2055-2105 (settle_key) and 6588-6630 (x_on_a_settling_row_refuses_once)
    - src/ui/screens/detail.rs lines 2445-2500 (owner_settle) and 39268-39305 (owner_x_on_a_settling_row_refuses_once)
    - src/ui/screens/render_escape_guard.rs lines 370-400 (screen table) and 2434-2465 (the SettleChooserScreen fixture arm, the pattern to copy)
    - README.md lines 444-466 and 495-501; docs/GETTING-STARTED.md lines 207-222
  </read_first>
  <behavior>
    - PendingSettles::stop_waiting(alias, row) removes every not-done entry that covers the row and returns true; it keeps done entries (the lingering check-mark line) and entries of other rows and aliases; it returns false when nothing not-done covers the row
    - row_actions on a row with a not-done pending entry lists x as Ready with label and footer hint `stop waiting`; with only a done entry or none, the usual `settle` entry (Ready or Blocked as today)
    - Inbox: after y in the settle popup the row is pending; x pushes the screen named `settle_stop`; Enter (default No) pops and the entry is still pending; x then Esc pops and it is still pending; x then y drops it, the status reads `Stopped waiting for the settle in alder-api; x settles the row again`, and x now opens `settle_chooser`
    - Same flow through the Owner block host (Stack::detail)
    - Yes when the entry already left (done or dropped meanwhile) sets `That settle is no longer pending; nothing changed`
    - The stop popup at 80x24 is not clipped, and the fixture tree stays byte-identical through the whole flow
  </behavior>
  <action>
Implements D-04 and D-05. Write the tests from the behavior list first (RED), then implement.

settle_pending.rs:
- Add `PendingSettles::stop_waiting(&mut self, alias: &str, row: &RowSrc) -> bool` with the behavior above. Memory only: no file, no process.
- Rewrite the module doc's closing paragraph: no timeout, no stall state, never dropped by itself (owner decision in 261118-v6l, kept by D-05). The escape is `x` on the row: stop waiting asks first (default No) and drops the entry from memory only, so the row is normal and `x` settles it again; a launched session, if any, keeps running. Add the stop_waiting unit tests here.

owner_row_actions.rs:
- Delete the pending-refusal constant and replace the refusal helper with `pub(crate) fn pending_settle<'a>(ctx: &'a AppContext, alias: &str, src: &RowSrc) -> Option<&'a PendingSettle>`, which returns the not-done entry covering the row via `ctx.settle_pending.for_row`. Update the module doc's mention of the old helper.
- In `row_actions`, check `pending_settle` first in the x block. When Some, push `action(ActionId::Settle, "x", "stop waiting", "stop waiting", "settle may still run; clears the badge here, asks first", Availability::Ready)`, whatever the settle rule would say. When None, keep today's logic unchanged.
- Rewrite the pending test as `x_offers_stop_waiting_while_the_settle_runs`.

settle_popup.rs: add `pub struct SettleStopScreen { host: AckHost, alias: String, row: RowSrc, words: String, popup: ConfirmPopup }` in a new section after the confirm popup. Put it in this module so tests/owner_settle_guard.rs scans it.
- `pub(crate) fn new(host, alias, row, words)` defaults to No.
- `render`: `render_host`, `confirm_popup::dim_background`, then `confirm_popup::render_confirm_popup` with title `Stop waiting for this settle?`, Color::Yellow and `self.popup.focus`, recorded into `self.popup`. Body: `field("Project", &shown(&alias), inner_w)`, `one_row("Settle", &words, inner_w)` (one_row runs `shown`), a blank line, then the D-04 explanation sentence wrapped with `wrap_at_separators` at inner_w in `muted()`.
- `handle_key`: set `ctx.needs_redraw`. Confirm calls `ctx.settle_pending.stop_waiting(&alias, &row)` and sets the D-04 status for true or false, then Pop. Cancel is Pop with no status. Pending returns None.
- `handle_mouse` goes through `confirm_popup::click_as_key`. `name()` returns `settle_stop`.
- `adjudicate_screen!(SettleStopScreen, RENDERS_ATTACKER_INFLUENCED_IDENTITY, ...)`, with a reason saying it draws the registered alias and the badge words (phase token, check number, Move target, all read from `.planning/`) through `shown`, over the inbox or the detail view.
- Add the flow tests from the behavior list here.

inbox.rs `settle_key`: replace the pending-refusal block with: if `owner_row_actions::pending_settle(ctx, &row.alias, &row.src)` is Some, set `ctx.needs_redraw` and return `ScreenAction::Push` of `SettleStopScreen::new(AckHost::Inbox, row.alias.clone(), row.src.clone(), entry.words.clone())`. Clone what you need before the push to satisfy the borrow checker. Rewrite `x_on_a_settling_row_refuses_once` as `x_on_a_settling_row_offers_stop_waiting`: the footer at 250 columns contains `[x]stop waiting`, x returns Push, a done entry opens the chooser.

detail.rs `owner_settle`: the guarded `Stop::Row` arm returns `Ok(ScreenAction::Push(...))` of `SettleStopScreen::new(AckHost::Detail(self.alias.clone()), self.alias.clone(), src, entry.words.clone())`, taking src from `owner_block::row_src_for` and the entry from `pending_settle`. Rewrite `owner_x_on_a_settling_row_refuses_once` the same way as the inbox test.

render_escape_guard.rs: add `("SettleStopScreen", "src/ui/screens/settle_popup.rs")` to the screen table between SettleReasonScreen and SlugInputScreen. Add a fixture arm with two states, "stop popup over the inbox" (probe_ctx plus `hostile_owner_digest`) and "stop popup over the detail view", modeled on the SettleChooserScreen arm. Use alias = identity and words = identity, and take the RowSrc from the hostile digest (`owner_block::row_src_for` on its first group key, or the inbox rows of that digest).

Docs:
- README.md: in the `x` row of the key table, append `; on a row that is still settling it offers to stop waiting`. Change the dimmed-reason example in the action-menu paragraph to `Cannot settle: the source file is unreadable`. After "the badge just keeps counting", add: if it never finishes (the session was closed, or GSD recorded nothing), `x` on the row offers stop waiting; after a default-No confirmation it clears the badge in this app only (a running session keeps running) and `x` settles the row again.
- docs/GETTING-STARTED.md: one sentence after the settle text: a row still settling offers `x` stop waiting, which clears its badge after a confirmation.

Commit: `feat(quick-261125-khh): x on a settling row offers stop waiting`.
  </action>
  <verify>
    <automated>rtk bypass cargo test --lib settle --no-fail-fast && rtk bypass cargo test --lib render_escape --no-fail-fast && rtk bypass cargo test --test owner_settle_guard && rtk bypass cargo clippy --all-targets -- -D warnings && rtk bypass cargo test --no-fail-fast</automated>
  </verify>
  <acceptance_criteria>
    - `rtk bypass grep -rF 'Settle already pending for this row' src README.md docs | rtk bypass grep -vE '^[^:]*: *//' | wc -l` [HEAD=4 EXPECT==0]
    - `rtk bypass grep -v '^ *//' src/ui/screens/owner_row_actions.rs | rtk bypass grep -Fc '"stop waiting"'` [HEAD=0 EXPECT>=1]
    - `rtk bypass grep -Fc 'SettleStopScreen' src/ui/screens/render_escape_guard.rs` [HEAD=0 EXPECT>=2]
    - `rtk bypass grep -Fc 'SettleStopScreen' src/ui/screens/settle_popup.rs` [HEAD=0 EXPECT>=3]
    - `rtk bypass grep -v '^ *//' src/ui/screens/settle_pending.rs | rtk bypass grep -Fc 'fn stop_waiting'` [HEAD=0 EXPECT==1]
    - `rtk bypass grep -v '^#' README.md | rtk bypass grep -Fc 'stop waiting'` [HEAD=0 EXPECT>=1]
    - `rtk bypass grep -Fc 'stop waiting' docs/GETTING-STARTED.md` [HEAD=0 EXPECT>=1]
    - the render-escape guard covers SettleStopScreen (its census test passes with the new table row), and clippy plus the full `--no-fail-fast` run are green
  </acceptance_criteria>
  <done>x on a pending row opens a default-No stop-waiting confirm in both hosts; Yes drops the entry from memory and x settles again; No and Esc leave it; the footer and the menu show the enabled stop waiting entry; nothing auto-drops and there is no stall wording; nothing is written; docs updated; clippy and the full suite are green; one path-scoped commit.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 3: Quote the owner's reason in typographic triple quotes and record it verbatim</name>
  <files>src/ui/screens/owner_settle.rs, src/ui/screens/settle_popup.rs, tests/owner_settle_oracle.rs, README.md</files>
  <read_first>
    - src/ui/screens/owner_settle.rs lines 37-46 (constants), 174-214 (clean), 417-491 (the trailing-sentence helper, instruction, move_instruction, prompt), 1005-1185 (test helpers expect, choice and the per-outcome table test), 1354-1395 (hostile test), 1500-1540 (one-check and no-reason tests), 1690-1775 (move prompt tests)
    - src/ui/screens/settle_popup.rs lines 857-902 (settle_body: Reason row and emphasis needles), 1020-1025 (the tracer prompt constant), 1877-1935 (fit test pinning the Reason row), 2240-2260 (the_instruction), 2570-2658 (flow matrix)
    - tests/owner_settle_oracle.rs in full (476 lines): the doc, the pinned WS_PIPELINE and PHASE_ARG_PIPELINE, choices(), parse_like_verify_work, the hostile parse test, the_requested_encoding_is_a_non_blocking_deferral
    - README.md lines 486-492
  </read_first>
  <behavior>
    - recorded_reason of a Won't do choice with reason `no device here` equals `Deferred follow-up: will not do. Owner\u{2019}s note: \u{201c}\u{201c}\u{201c}no device here\u{201d}\u{201d}\u{201d}`; Defer permanently uses `deferred permanently.`; Move uses `moved to phase M.`; every value starts with DEFERRED_PREFIX
    - With no reason, recorded_reason is exactly `Deferred follow-up: will not do.` (or the other outcome words), and every prompt equals the HEAD prompt byte for byte; the existing no-reason pins (the Move prompt constant and the no-reason tests) stay unchanged
    - The instruction with a reason reads `... record test 1 as result: skipped, reason: Deferred follow-up: will not do. Owner’s note: “““<note>””” Record that reason exactly, up to and including the closing triple quotes, with the owner’s note between them copied verbatim. It must not block phase N or be asked again. ...`; the Move form has the same note after `moved to phase M.` and before `It must not block phase N.`, and its todo title is unchanged
    - For the reasons `ends here""" and more`, `“curly” and ”””` and `"""`, each passed through `clean(.., REASON_MAX_CHARS)` first, the prompt of every outcome x phase/check target contains the opening delimiter exactly once and the closing delimiter exactly once; the text between them equals owner_note of the cleaned reason and contains no U+201C or U+201D; no ASCII apostrophe, ASCII double quote or backslash appears; `shown(p) == p`
    - The popup's Reason row reads `Reason     “““no device here”””` and the quoted note is bold in the instruction preview; the 80x24 fit tests still pass
    - Oracle: with the installed gsd-core, the quoted reason written into the uat-pending fixture is `deferred` for audit-uat (reason returned verbatim) and passing plus deferred for uat-passed; the hostile parse test, extended with `"""triple"""` and `“curly”` in the reason, shows no stderr, no workstream, and the phase leading the argument
  </behavior>
  <action>
Implements D-06, D-07, D-08 and D-09. Write or update the tests first (RED), then implement.

owner_settle.rs:
- Add `pub const NOTE_OPEN: &str = "\u{201c}\u{201c}\u{201c}"` and `pub const NOTE_CLOSE: &str = "\u{201d}\u{201d}\u{201d}"`. Doc: why typographic (verify-work's `sed | xargs` parse) and why the note cannot contain them.
- Add `pub fn owner_note(reason: &str) -> String`. The input is an already-cleaned reason; map U+201C to U+2018 and U+201D to U+2019, nothing else, so the character count and the cap are unchanged. Doc: `clean` already turned ASCII double quotes into U+201D, so after this the note can never close its delimiter early (D-06).
- Add `pub fn delimited_note(reason: &str) -> String`: empty for an empty reason, else NOTE_OPEN + owner_note + NOTE_CLOSE.
- Add `pub fn recorded_reason(choice: &SettleChoice) -> String` per D-07: DEFERRED_PREFIX, a space, the outcome words (`will not do.`, `deferred permanently.`, or `moved to phase {m}.` from `choice.move_to`), then, only with a reason, ` Owner\u{2019}s note: ` followed by `delimited_note`. Append no period to the note. Doc: this is the exact value the instruction asks GSD to record, and why it starts with DEFERRED_PREFIX.
- Add a private const for the verbatim sentence: ` Record that reason exactly, up to and including the closing triple quotes, with the owner\u{2019}s note between them copied verbatim.` Emit it only when the reason is non-empty.
- `instruction()` and `move_instruction()`: replace the `{DEFERRED_PREFIX} {outcome}{reason}` part of each template with the recorded reason followed by the optional verbatim sentence, keeping every other word, including `record {each} as result: skipped, reason: ` before it. Delete the old trailing-sentence helper and the now-unused outcome-word bindings. Leave the Move todo title as it is (D-08: the reason is not in it). Keep every template literal free of ASCII apostrophes, ASCII double quotes and backslashes; write the apostrophe as `\u{2019}`.
- Tests: update the `expect()` helper and the clause values in `settle_prompts_for_every_outcome_scope_and_workstream` to the new form. Update the reasoned Move pin in `move_prompts_for_phase_and_check_targets` (the `after the redesign` case). Extend `deferred_without_a_reason_has_no_empty_clause` to assert no `Owner\u{2019}s note`, no NOTE_OPEN, and the exact no-reason recorded_reason. Add `a_note_cannot_close_its_own_quotes` and `recorded_reason_keeps_the_prefix_and_the_quotes` per the behavior list. The existing hostile test must still pass.

settle_popup.rs (D-09):
- `settle_body`: the Reason row shows `owner_settle::delimited_note(&plan.choice.reason)`; the emphasis needle for the reason becomes `shown(&delimited_note(..))` instead of the raw reason. `settle_full_body` is unchanged (the argv already carries the note).
- Tests: change the tracer prompt constant to the new reasoned form (build it as a literal and assert it equals `owner_settle::prompt` output, as today). Update the Reason row pin, the needles in `the_instruction()`, and every expected prompt in the flow matrix that carries a reason. The Move prompt constant has no reason and stays as it is.

tests/owner_settle_oracle.rs:
- Import `recorded_reason`. In `the_requested_encoding_is_a_non_blocking_deferral`, build the reason with `recorded_reason(&SettleChoice { outcome: Outcome::WontDo, reason: clean("no device here", REASON_MAX_CHARS), move_to: None })`. Assert it equals the literal from the first behavior bullet, write it into the fixture row as now, and keep the audit-uat (category deferred, reason verbatim) and uat-passed (passing, deferred) assertions.
- In the hostile parse test, extend the hostile string with ` """triple""" \u{201c}curly\u{201d}`. The assertions stay.
- Update the module doc's point 3 to name the quoted owner's note.
- Do not set GSD_META_MANAGER_ALLOW_MISSING_ORACLE: the oracle is installed (`$CLAUDE_CONFIG_DIR` or `~/.claude`, gsd-core 1.23.0) and the test must really run.

README.md (around lines 489-491): after the sentence about the `Deferred follow-up:` reason, add: your reason goes into it as `Owner’s note: “““…”””`, verbatim, and any double quotes in it become single typographic quotes so they cannot end the note early.

If audit-uat or uat-passed ever reject or alter the quoted reason, stop and report it. Do not weaken the assertion: it would mean the delimiter does not survive recording.

Commit: `fix(quick-261125-khh): quote the owner's reason verbatim in the settle instruction`.
  </action>
  <verify>
    <automated>rtk bypass cargo test --lib settle --no-fail-fast && rtk bypass cargo test --test owner_settle_oracle --no-fail-fast && rtk bypass cargo clippy --all-targets -- -D warnings && rtk bypass cargo test --no-fail-fast</automated>
  </verify>
  <acceptance_criteria>
    - `rtk bypass grep -v '^ *//' src/ui/screens/owner_settle.rs | rtk bypass grep -Fc 'pub fn recorded_reason'` [HEAD=0 EXPECT==1]
    - `rtk bypass grep -v '^ *//' tests/owner_settle_oracle.rs | rtk bypass grep -Fc 'recorded_reason'` [HEAD=0 EXPECT>=2]
    - `rtk bypass grep -v '^#' README.md | rtk bypass grep -Fc 'Owner’s note'` [HEAD=0 EXPECT>=1]
    - `rtk bypass cargo test --test owner_settle_oracle` runs its four tests with the oracle present (no SKIP line, no opt-out env var) and reports `0 failed`
    - clippy plus the full `--no-fail-fast` run are green
  </acceptance_criteria>
  <done>A reasoned settle records `Deferred follow-up: <outcome> Owner’s note: “““<note>”””` and tells the agent to copy it verbatim; typed triple quotes cannot close the note; a reasonless prompt is byte-identical to HEAD; the popup shows and bolds the quoted note; the oracle confirms verify-work's parse and gsd-core's deferral reading of the new encoding; README updated; one path-scoped commit.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| owner keyboard -> prompt | the free-text reason typed in the reason field enters the argv of `claude` |
| project `.planning/` -> prompt and popup | check names, file names, phase tokens and Move target names come from untrusted project files |
| app -> multiplexer | the launch argv crosses into tmux/herdr/GUI terminal |
| app -> live Claude session | after D-01 nothing crosses this boundary from settle (no focus, no clipboard) |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-khh-01 | Tampering | second GSD session in a project with a live one (STATE.md race, 39 D-11's risk) | medium | accept | owner decision D-01; the popup footer discloses that a new session starts beside the live one (D-02) |
| T-khh-02 | Tampering | owner note closing its delimiter early and injecting instruction text | medium | mitigate | `owner_note` maps U+201C/U+201D to U+2018/U+2019 after `clean`; unit test `a_note_cannot_close_its_own_quotes` asserts exactly one opening and one closing delimiter (Task 3) |
| T-khh-03 | Tampering | verify-work `sed \| xargs` argument parse broken by quotes or a smuggled `--ws` | high | mitigate | delimiters and templates use typographic characters only; the hostile test asserts no ASCII apostrophe, double quote or backslash; the oracle parse test, extended with typed triple quotes, asserts no stderr and no foreign workstream (Task 3) |
| T-khh-04 | Repudiation | stop waiting hides a settle that may still run | low | mitigate | default-No confirm whose body says it may still be running and that nothing is stopped or written; memory-only drop; never automatic (D-04, D-05) |
| T-khh-05 | Information disclosure | clipboard overwritten with the prompt without a clear warning | low | mitigate | the settle path no longer touches the clipboard; regression test asserts no OSC 52 and no native-copy seam call with a live session (Task 1) |
| T-khh-06 | Spoofing | terminal escapes in the alias or badge words drawn by the new stop popup | medium | mitigate | every dynamic value goes through `shown`; SettleStopScreen is registered in render_escape_guard with hostile fixture states (Task 2) |
| T-khh-07 | Tampering | a check or phase name containing a triple-quote run before the note | low | accept | names sit before the opening delimiter and cannot close it; the instruction names the note explicitly (`Owner’s note: “““`); `clean` still strips controls and collapses dashes |
| T-khh-08 | Tampering | settle modules writing project files | high | mitigate | tests/owner_settle_guard.rs still scans owner_settle.rs, settle_popup.rs (incl. SettleStopScreen) and settle_pending.rs; flow tests assert the fixture tree is byte-identical |
| T-khh-SC | Tampering | npm/pip/cargo installs | high | mitigate | no new dependency in this plan; any install would need the package-legitimacy gate and a blocking human checkpoint |
</threat_model>

<verification>
- After each task: `rtk bypass cargo clippy --all-targets -- -D warnings` and `rtk bypass cargo test --no-fail-fast`, with the run redirected to a scratchpad log and its `test result:` lines read back: every one says `0 failed` and every suite ran (unit, integration, oracle, envelope).
- `git status` is clean after the third commit; `git log --oneline -3` shows the three `quick-261125-khh` commits; nothing pushed.
- Human UAT (after execution, not blocking; 39 D-13 keeps instruction wording as human UAT):
  1. On a throwaway project with a live Claude session, `x w <reason> Enter y` opens a second session beside the live one, the clipboard is unchanged, and the live session is not focused.
  2. In that new session, `/gsd-verify-work` records `reason: Deferred follow-up: will not do. Owner’s note: “““<reason>”””` verbatim in the UAT file.
  3. Close that session before it records; `x` on the still-settling row offers stop waiting; `y` clears the badge and `x` opens the chooser again.
</verification>

<success_criteria>
- Settle's `y` always starts a new session: no focus switch, no clipboard, a popup note when a live session exists; `v` unchanged (D-01..D-03, D-10).
- A pending settle can be dropped by `x` and then a default-No confirm, in both hosts, from memory only, never automatically (D-04, D-05).
- The owner's reason is recorded as `Owner’s note: “““…”””` after `Deferred follow-up: <outcome>`, copied verbatim, impossible to close early, and absent (byte-identical prompt) when empty (D-06..D-09).
- tests/owner_settle_oracle.rs is green against the installed gsd-core 1.23.0; clippy with `-D warnings` and the full `--no-fail-fast` suite are green.
- README.md and docs/GETTING-STARTED.md match the new behavior; the phase 37 and 39 CONTEXT files are untouched.
</success_criteria>

<output>
Create `.planning/quick/261125-khh-settle-always-launch-a-new-session-let-a/261125-khh-SUMMARY.md` when done
</output>
