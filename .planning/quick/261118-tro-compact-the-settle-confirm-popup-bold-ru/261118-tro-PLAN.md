---
quick_id: 261118-tro
mode: quick
phase: quick-261118-tro
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/ui/screens/confirm_popup.rs
  - src/ui/screens/settle_popup.rs
  - src/ui/screens/render_escape_guard.rs
  - README.md
autonomous: true
requirements: [QUICK-261118-tro]

estimate:
  tokens: 110000
  raw_tokens: 110000
  tasks: 3
  confidence: low

must_haves:
  truths:
    - "At 80x24 the default (compact) settle popup is unclipped and confirmable for won't do, deferred permanently, move to a known phase, move to an unknown phase, check scope, phase scope and the live-session variant; it draws Project, Target (one row, cut with an ellipsis), Outcome (one row), Reason (only when one was entered), a Runs row, at most 3 dim instruction rows and one short dim footer (D-01..D-05)."
    - "The Runs row draws `claude /gsd-verify-work 01` (plus `--ws NAME` when present) in BOLD and a dim ` · env -u (N session vars)`; every part is sliced from settle_mux_argv(plan).as_slice(), N is the number of `-u` elements in that argv, and no `-u` operand (environment variable name) is drawn in the compact view (D-02)."
    - "The compact instruction rows are the tail of the argv's last element after the slash command, dim, with the target text, `phase N`, `result: skipped`, the reason and (Move) `phase M` in BOLD; with whitespace removed their characters equal the instruction's prefix (bold changes no character), and a cut instruction ends in an ellipsis (D-03)."
    - "`d` toggles the full-command view and back: it draws every element of settle_mux_argv(plan).as_slice() through `shown`, verbatim, in order, the prompt element in double quotes, wrapped, nothing elided; `d` itself launches nothing, sets no status and writes nothing (D-04)."
    - "The settle popup's hint row reads `d full command · y/n · ←/→ move · Enter select · Esc cancel` in the compact view and starts with `d summary` in the full view; every other confirm popup's hint row is unchanged (D-04)."
    - "A view the terminal cuts off shows CLIPPED_HINT and cannot be confirmed by `y`, Enter on Yes or a click on Yes; `d` from a cut-off full view returns to the compact view, which confirms when it fits; Yes from an unclipped full view launches exactly the same prompt as from the compact view (D-07)."
    - "The unknown-phase Move warning still renders in Yellow with its full text in both views (D-06); README describes the summary and the `d` full command (D-09); src/ui/screens/detail.rs and src/ui/screens/help.rs are not modified."
  artifacts:
    - path: src/ui/screens/confirm_popup.rs
      provides: "render_confirm_popup_hinted: the shared popup with an optional leading hint fragment; render_confirm_popup delegates to it unchanged"
      contains: "pub fn render_confirm_popup_hinted"
    - path: src/ui/screens/settle_popup.rs
      provides: "compact settle_body (Runs summary + capped emphasized instruction), settle_full_body (the whole argv), the `d` toggle on SettleConfirmScreen"
      contains: "fn settle_full_body"
    - path: src/ui/screens/render_escape_guard.rs
      provides: "a SettleConfirmScreen fixture state in the full-command view"
      contains: "showing_full_command"
    - path: README.md
      provides: "the settle paragraph describing the summary and `d` full command"
      contains: "full command"
  key_links:
    - from: src/ui/screens/settle_popup.rs
      to: src/ui/screens/detail.rs
      via: "settle_mux_argv(plan) — both settle_body and settle_full_body read the one ExecArgv the launcher executes (import only, detail.rs untouched)"
      pattern: "settle_mux_argv\\(plan\\)"
    - from: src/ui/screens/settle_popup.rs
      to: src/ui/screens/confirm_popup.rs
      via: "SettleConfirmScreen::render calls render_confirm_popup_hinted with `d full command` / `d summary`"
      pattern: "render_confirm_popup_hinted"
    - from: src/ui/screens/render_escape_guard.rs
      to: src/ui/screens/settle_popup.rs
      via: "SettleConfirmScreen::showing_full_command() in the SettleConfirmScreen fixture states"
      pattern: "showing_full_command"
---

<objective>
Compact the settle confirm popup (`x`, Phase 39) so the default view shows what matters and
the exact argv stays one key away. The owner's words: the popup is "probably all correct but
does it really have to be that much text? Find a good balance between security (knowing what's
executed) and actually showing what's important."

This narrows Phase 39 D-10 (popup "shows project, target, outcome, the exact argv command line
and the full instruction, wrapped") for the DEFAULT view only, at the owner's request: the full
39 D-10 content is the `d` view, unchanged in substance.

Decisions of this quick task (all INFERRED by the orchestrator from the owner's request; the
executor records each as INFERRED in the SUMMARY):

- D-01: Compact header rows, label column LABEL_W = 11 as today. Project keeps today's wrapping
  `field` row (it carries the escaped hostile identity the render_escape_guard requires, and the
  cwd is the one fact the argv does not contain). Target and Outcome become ONE row each, cut
  with `fit_cells` (ellipsis). A Reason row (one row, cut) appears only when a reason was
  entered. Every value goes through `shown` BEFORE it is cut.
- D-02: A Runs row derived ONLY from settle_mux_argv(plan).as_slice(): program words (the
  elements between index 0 and the last element that are neither `-u` nor a `-u` operand, i.e.
  `claude`) plus the slash command (the last element's leading `target.command` Display, which
  includes `--ws NAME`) in BOLD, then a dim ` · {elements[0]} -u ({n} session vars)` where n is
  the count of elements equal to `-u`. If n is 0 the dim fragment is omitted and elements[0]
  joins the bold program words. A Runs value wider than its column wraps with
  `wrap_at_separators` under the label; the dim fragment goes on its own indented row when it
  does not fit after the command.
- D-03: Under Runs, indented LABEL_W, the instruction (the last element after the command and
  its separating space, sliced from the argv, never recomputed for display) in dim DarkGray,
  capped at `const INSTRUCTION_ROWS: usize = 3` rows; a cut ends in a dim ellipsis. Key facts
  BOLD by substring ranges over the real text: `target_text(target)`, `phase {n}` (n =
  target.phase()), `result: skipped`, the reason, and for Move `phase {m}`. Bold changes no
  character.
- D-04: `d` (no CONTROL/ALT) toggles a full-command view and back. The full view is today's
  body (Project, Target, Outcome wrapped, blank, the exact argv joined with the last element
  quoted, wrapped at separators with a 2-space continuation, the unknown-phase warning, the
  footer). The hint row gains a leading fragment through a backward-compatible
  `render_confirm_popup_hinted`; `render_confirm_popup` delegates to it with no fragment. No
  copy key (`y` is Yes).
- D-05: One short dim footer shared by both views. New session:
  `Yes starts a new session · GSD records it · this app writes nothing` (67 cells). Live
  session: `Claude runs here · Yes focuses it, copies the command · no new session` (70 cells,
  one row at 80 cols). Both wrapped with `wrap_at_separators` so narrower popups still draw
  every word.
- D-06: The unknown-phase Move warning stays as today in both views (Yellow AND its text).
- D-07: The full view does not scroll. When it does not fit it clips: CLIPPED_HINT, not
  confirmable, `d` returns to the compact view. Yes is allowed from the compact view (a
  deliberate summary) and from an unclipped full view.
- D-08: Security tests: the full view contains every element of settle_mux_argv(plan) verbatim
  (built from the same `as_slice()`, through `shown`), the compact Runs row and count are
  derived from the same argv, and truncation never hides anything from the full view. The
  RENDERS_ATTACKER_INFLUENCED_IDENTITY adjudication text and fixture states cover both views.
- D-09: README's settle paragraph describes the summary and `d`. help.rs line ~543 describes
  the `x` key, not popup content, so help.rs is not touched.

Purpose: the owner sees the command and the key facts at a glance; the exact argv is one key
away and remains the only thing Yes executes.
Output: compact + full settle popup, the shared hint extension, updated tests, guard state,
README paragraph.
</objective>

<execution_context>
@~/.claude/gsd-core/workflows/execute-plan.md
@~/.claude/gsd-core/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@CLAUDE.md

Read before editing (line numbers at HEAD 3266fb5d):
- src/ui/screens/settle_popup.rs: imports 12-30 (bold(), muted(), fit_cells, fit_spans,
  settle_mux_argv, shown, wrap_at_separators already imported); SettleConfirmScreen 515-715
  (settle_body 575-665, adjudicate_screen! 667-675, handle_key/render 677-714); tests from 717,
  especially tracer_plan 848, move_plan 1316, the_move_popup_fits_* 1342-1367, squashed 1456,
  the_popup_fits_80x24_with_the_exact_argv 1470, the_tracer_popups_fit_the_body_room_at_80x24
  1518, session_in 1533, a_live_session_is_focused_not_relaunched 1658,
  the_popup_clips_at_60x20_and_cannot_confirm 1702, an_unknown_phase_is_warned_in_the_popup 1201.
- src/ui/screens/confirm_popup.rs 1-340: module contract, ConfirmPopup::on_key (`y` is Yes;
  Left/Right/Tab/BackTab/h/l move focus), render_confirm_popup 233-300 (key hint literal at
  288), body_room 303, wrapped_rows 194, CLIPPED_HINT 67.
- src/ui/screens/owner_settle.rs: target_text 350, instruction 397, move_instruction 422,
  prompt 454 (`"{command} {instruction}"`), SettleTarget::phase 114, MoveTarget 129.
- src/ui/screens/detail.rs 3134-3205 (settle_mux_argv, owner_scrubbed_program) and fit_cells
  11439 / fit_spans 9948 / shown 83: READ ONLY. Do NOT edit detail.rs (another agent edits it
  concurrently). Do NOT edit help.rs.
- src/ui/screens/render_escape_guard.rs 2552-2610 (SettleConfirmScreen fixture states).
- src/ui/glyphs.rs TYPOGRAPHY list (line ~322): the only non-ASCII chars allowed are already on
  it; this plan uses only U+00B7 middle dot, U+2026 ellipsis and the existing U+2190/U+2192
  arrows, always written as `\u{..}` escapes. Never U+00D7.
- tests/owner_settle_guard.rs: settle_popup.rs production lines must not spell a write or
  spawn API.

Environment: build with `CARGO_TARGET_DIR=target-wt` (already excluded through the shared
`.git/info/exclude`; never commit it). Wrap every check whose output matters in
`rtk bypass`, and never pipe test output through grep (rtk filters downstream of a pipe).
</context>

<tasks>

<task type="tracer" tdd="true">
  <name>Task 1: End-to-end `d` toggle — hinted popup, compact Runs summary, full-command view</name>
  <files>src/ui/screens/confirm_popup.rs, src/ui/screens/settle_popup.rs</files>
  <precondition>`git check-ignore -q target-wt` succeeds in the worktree (target-wt excluded); src/ui/screens/detail.rs and src/ui/screens/help.rs show no diff against HEAD.</precondition>
  <behavior>
    - confirm_popup: render_confirm_popup_hinted with fragment `d full command` and a short body at 80x24 draws the hint row `d full command · y/n · ←/→ move · Enter select · Esc cancel`; at 60x20 the combined row does not fit the 54-cell inner width, so the fragment is dropped and the plain key hint is whole; a body taller than the room with fragment `d summary` at 80x24 draws `d summary · ` followed by CLIPPED_HINT and reports clipped; render_confirm_popup (no fragment) draws exactly what it drew before (existing confirm_popup tests stay green unmodified).
    - settle compact view (tracer plan, Won't do, reason `no device here`, 80x24): contains `Project    alder-api`, `Target     phase 01: tests 2, 3 in 01-UAT.md`, `Outcome    Won't do`, `Reason     no device here`, `Runs       claude /gsd-verify-work 01 · env -u ({n} session vars)` with n computed in the test from `argv.as_slice()` (count of `-u`, asserted > 0), the D-05 new-session footer, `d full command`, `[ Yes ]`, `[ No ]`; contains NO element that follows a `-u` in the argv; not clipped; focus is No.
    - `d` returns ScreenAction::None, sets no status, launches nothing; the next render draws the full view whose squashed text contains the joined argv built from `argv.as_slice()` through `shown` (last element quoted) AND each element individually, plus `d summary`; a second `d` restores the compact view; Ctrl-d and Alt-d do not toggle.
    - Stack flow: to_popup, `d`, `y` launches exactly one plan whose prompt equals TRACER_PROMPT (Yes from an unclipped full view).
    - Cut-off views: the full view at 60x20 is clipped (assert it), shows CLIPPED_HINT, and `y`, then Left + Enter, return ScreenAction::None with no launch; `d` then re-render at 60x20 shows the compact view unclipped; the compact view at 60x12 is clipped and `y` returns ScreenAction::None; `n` and Esc still Pop.
    - Live session (a_live_session_is_focused_not_relaunched): the 100x30 render contains the D-05 live footer words `Claude runs here`, `Yes focuses it, copies the command`, `no new session`; the rest of that test is unchanged.
  </behavior>
  <action>
    Write the behavior tests first (RED), then implement (GREEN).

    confirm_popup.rs (per D-04): extract the key-hint literal at line 288 into a private `const KEY_HINT: &str` (same escapes). Add `pub fn render_confirm_popup_hinted(frame, area, title, body, accent, focus, extra_hint: &str) -> PopupLayout` holding the whole current body of render_confirm_popup; render_confirm_popup becomes a one-line delegate passing an empty fragment, signature unchanged, so every other caller (ack_confirm, driver_launch, driver_steer, driver_confirm, delete_confirm, queue_delete_confirm, create_project) is byte-identical. Hint rule: base = CLIPPED_HINT when clipped, else KEY_HINT, styled as today (Yellow / DarkGray); when extra_hint is non-empty and its cell width + 3 + the base's cell width is at most inner.width, the row text is `{extra_hint} \u{b7} {base}` in the base's style, otherwise the base alone. Measure cells with ratatui `Span::width`, never `str::len`. Add one bullet to the module doc contract saying a caller may prepend one short key fragment, dropped when it would not fit, so CLIPPED_HINT is never truncated. Add the confirm_popup unit test from the behavior list beside the existing render tests.

    settle_popup.rs: add `full: bool` to SettleConfirmScreen (false in new()). In handle_key, before `self.popup.on_key`, a `KeyCode::Char('d')` whose modifiers contain neither CONTROL nor ALT flips `full`, keeps `ctx.needs_redraw = true` and returns ScreenAction::None; any other key goes to the popup exactly as today (`_modifiers` becomes `modifiers`). In render, pick settle_full_body or settle_body by `full` and call render_confirm_popup_hinted with `d summary` (full) or `d full command` (compact); title, accent, dim_background and record unchanged. Mouse handling unchanged.

    Move today's settle_body (lines 575-665) to `pub(crate) fn settle_full_body(plan, inner_w)` (per D-04) with only the consequence rows replaced by a shared private `footer(plan, inner_w) -> Vec<Line<'static>>` producing the D-05 text (dim, wrapped with wrap_at_separators), and factor the unknown-phase warning (D-06, unchanged text and Yellow style) into a shared private helper both bodies call. Then write the new compact `pub(crate) fn settle_body(plan, inner_w)` (signature unchanged, so the existing fit tests keep calling it): Project via the existing wrapping `field` closure; Target, Outcome and (non-empty reason only) Reason as one row each — label span bold, value `fit_cells(&shown(..), inner_w - LABEL_W)` (D-01); blank row; the Runs row per D-02, computed from `settle_mux_argv(plan)` — on Ok, split the last element at `plan.draft.target.command.to_string()` plus one space (if the last element does not start with that, treat the whole last element as the instruction text and draw only the program words bold, so nothing is invented); on Err, the Runs value is `shown(&error.to_string())` as today's fallback. Each drawn string goes through `shown`. Then the warning helper, a blank row and the footer. Task 2 inserts the instruction rows directly under Runs; leave a single blank row between the Runs block and the warning/footer. Every compact Line must fit inner_w by construction (no reliance on Paragraph re-wrap). Update doc comments: module header line 2, the SettleConfirmScreen doc and both body fns to say the default view is a summary cut from the exact argv and `d` shows the whole argv.

    Update the existing tests this changes, in the existing style: the_popup_fits_80x24_with_the_exact_argv (compact assertions, then `d` via screen.handle_key with a `mut ctx`, then the exact-argv assertion built from `argv.as_slice()` through `shown`); a_live_session_is_focused_not_relaunched (new footer words); replace the_popup_clips_at_60x20_and_cannot_confirm with `a_cut_off_view_cannot_confirm_and_d_returns_to_the_summary` covering the cut-off bullets above. Add `d_toggles_the_full_command_and_launches_nothing` covering the toggle, modifier and Stack-flow bullets. Commit: `git commit -m "feat(quick-261118-tro): settle popup summary with a d full-command view" -- src/ui/screens/confirm_popup.rs src/ui/screens/settle_popup.rs` (path-scoped, never `git add -A`; end the message with the session's attribution lines).
  </action>
  <verify>
    <automated>CARGO_TARGET_DIR=target-wt rtk bypass cargo test --lib --no-fail-fast -- ui::screens::confirm_popup ui::screens::settle_popup</automated>
    <automated>rtk bypass grep -c "pub fn render_confirm_popup_hinted" src/ui/screens/confirm_popup.rs</automated> [HEAD=0 EXPECT==1]
    <automated>rtk bypass grep -c "fn settle_full_body" src/ui/screens/settle_popup.rs</automated> [HEAD=0 EXPECT==1]
  </verify>
  <done>detail.rs and help.rs remain untouched (invariant, not a change measure: `rtk bypass git diff --name-only HEAD -- src/ui/screens/detail.rs src/ui/screens/help.rs` prints nothing). The compact view draws the argv-derived Runs row and no environment variable name; `d` shows every argv element verbatim and back; the shared hint row carries the fragment only for the settle popup; a cut-off view of either kind cannot confirm; confirm_popup and settle_popup tests all pass; one path-scoped commit.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Capped, emphasized instruction in the compact view and the 80x24 fit matrix</name>
  <files>src/ui/screens/settle_popup.rs</files>
  <behavior>
    - emphasized_rows on the tracer instruction at width 59 with no cap: concatenated span contents, whitespace removed, equal the instruction with whitespace removed; every BOLD span's content is a substring of one of the needles; no row is wider than 59 cells.
    - The same call capped at INSTRUCTION_ROWS returns exactly 3 rows, the last ending in U+2026, each row at most 59 cells.
    - A needle absent from the text bolds nothing and does not panic; a `phase 1` needle does not bold the head of `phase 10`; text with typographic quotes (a check-scope target_text) splits on char boundaries.
    - Compact body of the tracer plan: the BOLD span contents of the instruction rows, joined and whitespace-removed, contain `tests2,3in01-UAT.md` and `phase01`; the non-bold instruction spans are DarkGray; the Runs line's BOLD spans contain `claude /gsd-verify-work 01` and its dim span contains `env -u (`.
    - Fit matrix at 80x24 (inner 70, room 19): Won't do with reason, Deferred permanently without reason, Move check known (move_plan heuristic-labels `01-UAT.md#4`), Move phase known, Move check unknown (known false, number `7`), Move phase with a reason, and the tracer plan with a live session (draft.live_session = Some(session_in(&root))): for each, compact rows = wrapped_rows(settle_body) are at most room AND equal body.len() (no Paragraph re-wrap), and SettleConfirmScreen renders unclipped; the unknown-phase case draws the warning text in Yellow.
    - Full view fit, as today: tracer and Move check at 80x24 unclipped; Move phase at 100x30 unclipped (eprintln its 80x24 full-view row count for the SUMMARY).
  </behavior>
  <action>
    Write the behavior tests first (RED), then implement (GREEN), per D-03.

    Add `const INSTRUCTION_ROWS: usize = 3;` and a private `fn emphasized_rows(text: &str, needles: &[String], width: usize, max_rows: usize) -> Vec<Vec<Span<'static>>>`: (1) ranges = every `match_indices` hit of every non-empty needle, dropping a hit whose next char is ASCII alphanumeric, sorted by start then longer first, kept greedily non-overlapping; (2) rows = wrap_at_separators(text, width), each located in text by a forward `find` from a cursor (rows are in-order substrings; if a find ever fails, emit that row as one muted span so no text is dropped or changed); (3) split each row at range boundaries into `bold()` spans (normal color) and `muted()` spans; (4) when rows exceed max_rows keep max_rows, append a muted ` \u{2026}` span to the last kept row and pass it through `fit_spans(spans, width)` so it never exceeds width and always ends in the ellipsis.

    In settle_body, directly under the Runs block, emit the instruction rows: text = shown(instruction slice from Task 1's argv split), needles = shown(&target_text(target)), shown(&format!("phase {}", target.phase())), `result: skipped`, shown(&choice.reason) when non-empty, and for Move shown(&format!("phase {}", to.number)); width = inner_w - LABEL_W, max_rows = INSTRUCTION_ROWS; each row becomes a Line led by a LABEL_W-space raw span. Do not touch settle_full_body (its argv stays undimmed and whole).

    Tests: add the emphasized_rows unit tests and `the_compact_popup_fits_80x24_for_every_outcome_and_scope` (the matrix above; build the unknown and reasoned Move plans by editing the plan move_plan returns); extend the_tracer_popups_fit_the_body_room_at_80x24 with the no-re-wrap equality; update the_move_popup_fits_for_a_check_at_80x24 and the_move_popup_fits_for_a_phase_at_100x30 to assert the compact body at 80x24 and the full body at their sizes; keep an_unknown_phase_is_warned_in_the_popup passing (it renders compact at 100x40). Commit: `git commit -m "feat(quick-261118-tro): cap and emphasize the settle instruction in the summary" -- src/ui/screens/settle_popup.rs` with the session's attribution lines.
  </action>
  <verify>
    <automated>CARGO_TARGET_DIR=target-wt rtk bypass cargo test --lib --no-fail-fast -- ui::screens::settle_popup</automated>
    <automated>rtk bypass grep -c "const INSTRUCTION_ROWS" src/ui/screens/settle_popup.rs</automated> [HEAD=0 EXPECT==1]
  </verify>
  <done>Every listed variant fits 80x24 unclipped in the compact view with at most 3 dim instruction rows whose key facts are bold and whose characters are the argv's; the full view still fits where it fit before; settle_popup tests pass; one path-scoped commit.</done>
</task>

<task type="auto">
  <name>Task 3: Adjudication, render-escape guard full-view state, README, full gates</name>
  <files>src/ui/screens/settle_popup.rs, src/ui/screens/render_escape_guard.rs, README.md</files>
  <action>
    Per D-08: in settle_popup.rs add `#[cfg(test)] pub(crate) fn showing_full_command(mut self) -> Self` (sets `full = true`), and rewrite the SettleConfirmScreen adjudicate_screen! text so it stays accurate: the alias, phase token, check file and names, workstream and reason (read out of `.planning/` or typed) are drawn in the summary (header rows, the Runs row and the capped instruction, all cut from the argv after `shown`) and, behind `d`, in the whole argv, each value through `shown`, over the inbox or the detail view; fixture states: the summary and the full-command view with hostile values at the sizes the guard renders. In render_escape_guard.rs add one more `one_state("popup in the full-command view", probe_ctx(identity), Box::new(SettleConfirmScreen::new(AckHost::Inbox, plan()).showing_full_command()))` to the SettleConfirmScreen arm (2552-2610). If a guard assertion fails for the compact states, fix the popup (every value through `shown` before `fit_cells`; Project keeps its wrapping row), never the guard.

    Per D-09: in README.md (lines ~451-453) replace the sentence describing what the default-No popup displays with one saying the popup sums the settle up — project, target, outcome, your reason, the command it runs in bold with how many session variables `env -u` clears, and the start of the instruction with its key facts in bold — and that `d` switches to the full command, every argv element verbatim, and back; keep "only `y` launches `/gsd-verify-work N <instruction>` ..." and add that a view the terminal cuts off cannot be confirmed. Keep the paragraph's ~90-column line wrapping. Leave help.rs alone (its `x` row describes the key, not popup content).

    Final gates in this order, reading raw output: `CARGO_TARGET_DIR=target-wt rtk bypass cargo fmt --check` (if it fails, run `rtk bypass cargo fmt` and confirm `git status --short` lists only this plan's files; restore any other file with `git checkout -- <file>`), then the full test suite and clippy below. Commits, path-scoped with the session's attribution lines: `test(quick-261118-tro): cover the settle full-command view in the render-escape guard` for settle_popup.rs and render_escape_guard.rs, then `docs(quick-261118-tro): describe the settle summary and its d full command` for README.md. Never push. End with a clean `git status` (target-wt ignored).
  </action>
  <verify>
    <automated>CARGO_TARGET_DIR=target-wt rtk bypass cargo test --no-fail-fast</automated>
    <automated>CARGO_TARGET_DIR=target-wt rtk bypass cargo clippy --all-targets -- -D warnings</automated>
    <automated>CARGO_TARGET_DIR=target-wt rtk bypass cargo fmt --check</automated>
    <automated>rtk bypass grep -c "then shows the exact" README.md</automated> [HEAD=1 EXPECT==0]
    <automated>rtk bypass grep -c "full command" README.md</automated> [HEAD=0 EXPECT>=1]
    <automated>rtk bypass grep -c "showing_full_command" src/ui/screens/render_escape_guard.rs</automated> [HEAD=0 EXPECT>=1]
  </verify>
  <done>The render-escape guard renders both views with hostile values and passes; the adjudication text names both views; README describes the summary and `d`; full test suite (no fail-fast), clippy -D warnings and fmt are clean; detail.rs and help.rs untouched; commits path-scoped; git status clean.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| `.planning/` files and typed reason → popup cells | alias, phase token, workstream, check file/names and the reason are attacker-influenced text drawn in the popup |
| popup → launcher | what the owner reads must be what `launch_settle` executes (settle_mux_argv) |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-261118tro-01 | Spoofing | compact Runs row / instruction rows | high | mitigate | Every compact element is sliced from settle_mux_argv(plan).as_slice(), the same ExecArgv the launcher runs; the `-u` count is computed from it; tests build their expectations from the same argv (Task 1, Task 2) |
| T-261118tro-02 | Repudiation | truncation (Target/Outcome/Reason rows, 3-row instruction cap) hiding injected text | medium | mitigate | Every cut is marked with an ellipsis; `d` shows every argv element verbatim with nothing elided (test asserts each element and the ordered join); bold changes no character (squashed-equality test) |
| T-261118tro-03 | Tampering | terminal escape / invisible characters from hostile values | high | mitigate | `shown` applied before `fit_cells`, emphasis and wrapping; render_escape_guard renders the compact view (4 existing states) and the new full-command state with the hostile identity (Task 3) |
| T-261118tro-04 | Elevation of Privilege | confirming a view the owner could not read | medium | mitigate | Both views go through render_confirm_popup_hinted's clipped flag: a cut-off view shows CLIPPED_HINT and `y`/Enter/click do nothing; CLIPPED_HINT is never truncated by the extra fragment (fit rule + test) |
| T-261118tro-05 | Elevation of Privilege | `y` pressed after `d` but before the next draw uses the previous view's clipped flag | low | accept | The flag describes the view last drawn, which the owner saw; the compact view is confirmable by design and a stale clipped flag only blocks |
| T-261118tro-SC | Tampering | npm/pip/cargo installs | low | accept | No dependency is added or updated by this plan; no install task exists |
</threat_model>

<verification>
- `CARGO_TARGET_DIR=target-wt rtk bypass cargo test --no-fail-fast` passes (compare the pass count against the pre-change run; every settle_popup, confirm_popup and render_escape_guard test reports).
- `CARGO_TARGET_DIR=target-wt rtk bypass cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` are clean.
- Rendered at 80x24 the compact tracer popup shows Project / Target / Outcome / Reason / Runs / 3 instruction rows / footer / buttons / `d full command` hint; after `d` the whole argv; after `d` again the summary.
- detail.rs and help.rs have no diff; target-wt is not tracked.
</verification>

<success_criteria>
- The default settle popup is a compact, fully argv-derived summary that fits 80x24 for every outcome, scope and the live-session variant.
- The exact, complete argv is one `d` away and is still the only thing Yes executes.
- "A cut-off popup cannot be confirmed" holds for both views.
- Every other confirm popup is unchanged.
</success_criteria>

<output>
Create `.planning/quick/261118-tro-compact-the-settle-confirm-popup-bold-ru/261118-tro-SUMMARY.md` when done, listing D-01..D-09 as INFERRED, the 80x24 row counts (compact per variant, full for the phase-level Move), and the commit shas.
</output>
