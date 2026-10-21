# Phase 30 — UI Review

**Audited:** 2026-10-20
**Baseline:** 30-UI-SPEC.md (approved, terminal/ratatui medium)
**Screenshots:** not captured (TUI). Rendered TestBackend text and style snapshots in /tmp/claude-1000/t/ui30/ (80x24, 100x30, 120x40, 80x12, 60x16, 50x24) were used instead.
**Interaction captures:** off (workflow.ui_interaction_capture is false). Interaction findings come from the key-driven harness snapshots and code.

---

## Pillar Scores

| Pillar | Score | Key Finding |
|--------|-------|-------------|
| 1. Copywriting | 2/4 | Polite-stop confirm clips "Nothing is killed." at every width >= 80 (BLOCKER); long status/answer copies lose their next step |
| 2. Visuals | 3/4 | Tiers, no-orphan rule, glyph triples and selection idiom conform; kill popup repaints the dashboard behind it |
| 3. Color | 3/4 | Cyan is selection-only, footer keys BOLD default, no new REVERSED; header row 3 shows a false Yellow warning while loading |
| 4. Typography | 4/4 | Two weights only, roles match the contract; no Wrap in scroll panes |
| 5. Spacing | 3/4 | Label column, indents and width tiers hold; unwrapped warning/slug rows are clipped without an ellipsis |
| 6. Experience Design | 2/4 | Popups at short heights hide the decision content while Yes stays live (D5); header conflates loading with "no control line" |

**Overall: 17/24**

---

## Top 3 Priority Fixes

1. **Polite-stop confirmation clips "Nothing is killed."** (BLOCKER). The sentence that separates polite stop from kill (D-22) is invisible at 80, 100 and 120 columns. Fix `wrapped_rows` in `src/ui/screens/confirm_popup.rs:151-161` to count word-wrapped lines (simulate word wrap, or add a spare row), then re-snapshot c4 at 80x24.
2. **Popups hide their decision content at short heights while `y` stays live** (D5, WARNING). The launch form at 80x12 loses the Sandbox row, spend line and field hint. The lint popup loses findings 4-6 and "Edit it again now?". Either arm `y` only when the full content fits, or show a `[terminal too short: enlarge to confirm]` row and keep `y` inert.
3. **Wrong or truncated guidance in status copy and the header.** Three sub-fixes:
   - `src/ui/screens/driver_run.rs:2472`: distinguish "detail not loaded" from "no control line", so FULL shows `Loading...` or the frontmatter echo, as MEDIUM does.
   - Wrap or shorten the D-16 answer warning row (`driver_steer.rs` ~531-575) so "...or relaunch with higher caps with [R]." survives at 80-120 columns.
   - Shorten the injection-retired, answer-written and left-unlaunched copies so the key hints survive an 80-column status bar.

Also worth fixing: D6 (kill popup paints the dashboard behind it), D7 and D8 (minor).

---

## Detailed Findings

### Pillar 1: Copywriting (2/4)
- **BLOCKER (D1).** `c4_polite_stop_confirm_80x24.txt` rows 9-12 show the body cut after "Nothing is" (row 10), a blank row 11, and "killed." never rendered. Root cause: `confirm_popup.rs:151-161` `wrapped_rows` uses `ceil(chars/width)`, but the body Paragraph word-wraps with `Wrap{trim:false}`, so the real line count is higher and `popup_rect` (`confirm_popup.rs:139-149`) is one row short. The call site is `driver_steer.rs:363-380`. The same clip appears at 80x12, 100x30 and 120x40. The popup fits only at 50 and 60 columns. I verified this by reading the source and the snapshot. This breaks the D-22 contract that polite stop never reads as a kill, in the one place a user decides between them.
- **WARNING (D4).** The injection-retired copy is cut at 80 columns after "Steer this run through DIRECTIO…" (`b5_after_i_retired_80x24` row 0), so "[c] control, [a] answer, [e] edit" is lost. It is still cut at 100. The answer-written and left-unlaunched copies carry the same risk. The status bar truncation is shipped; these long strings are Phase 30's. The contract text (UI-SPEC lines 784, 760, 775) is fixed. Shortening the strings, or wrapping the bar, is `[inferred — audit]`.
- **WARNING (D3).** The D-16 answer warning (row 2) is one unwrapped line, clipped at 80, 100 and 120 columns (b4 rows 23/29/39). Its actionable half ("...relaunch with higher caps with [R].") is never visible, though the spec shows the full sentence.
- Minor (D8): the slug not-usable row is clipped at 80 columns ("...starting with a letter o", `f2_slug_invalid_80x24` row 23).
- Conformant: empty-state heading and body, `Loading...`, `This run has no LOG.md.`, the `(none recorded)` family, `I-01 is not on hold.`, the control chooser row, answer row 1, slug preview, popup titles, degraded-host rows. Documented deviations accepted: the spend line (30-05-PLAN decision 3) and `DIRECTION.md:{n}:` lint prefixes (30-04-SUMMARY:230).
- Minor (D9): `[s]tart` is dropped first at 80 columns when a run is selected. This follows the spec's priority table. The UI-checker's verb-only CTA flag stands.

### Pillar 2: Visuals (3/4)
- Conformant: two-pane at >= 60 columns, single pane with ` {id} ({i}/{n}) ` at 50. FULL, MEDIUM and SUMMARY tiers match the spec. No orphan rules appear in any snapshot. The status glyph/word/colour triples match the table. `D:Drive◆` lights for a live launcher run (c1). Footer drop arithmetic matches at 80/100/120. `[?]help` is never dropped.
- **WARNING (D6).** `x` kill confirm paints the dashboard ("GSD Manager" project table) behind the popup (`c5_kill_confirm_80x24`) because `driver_confirm.rs:445` renders `NormalScreen`. Every other Phase-30 popup keeps the Driver tab behind it via `render_main_only`. The user loses context on the most destructive action. Fix: use `render_main_only`.
- Minor (D7): popup backgrounds re-layout to a different height tier than the screen underneath (80x24: MEDIUM on the tab, FULL behind the popup). The background is dimmed, so this is cosmetic.
- Not observed (fixtures lacked them): stale heartbeat, audit MISMATCH, UNSANDBOXED badge, legacy section, edit conflict chooser, dashboard badges. These are unit-test-covered per the SUMMARYs but not visually verified.

### Pillar 3: Color (3/4)
- Cyan appears only on the selected run row and the item cursor row, both with `"> "` and BOLD (b1_100x30.style rows 4, 11, 29). Footer keys are BOLD on the default fg, never Cyan. There is no new REVERSED. Status colours match the table (‖ Yellow, ● Green, ■ DarkGray, ⚑ Red, ◆ Magenta, ✗ Red).
- **WARNING (D2).** FULL header row 3 renders `control (no control line)` in Yellow while the body is still loading (`b2b_long_id_selected_100x30` row 6). `driver_run.rs:2472` passes `detail.and_then(|d| d.body_control...)`, which conflates "no detail yet" with "no control line". `header_row_3` (`driver_run.rs:981-995`) then styles it as a warning. Result: a false warning colour on a healthy run. MEDIUM falls back to the frontmatter echo (`driver_run.rs:2051-2060`), so the tiers disagree.
- Needs human: DarkGray contrast on the user's theme (see below).

### Pillar 4: Typography (4/4)
- Only the regular and BOLD weights occur in the style dumps. Label, Muted, Emphasis and Selection roles are used as specified. The LOG pane is no-Wrap, with a 4-cell detail indent and DarkGray timestamps. A long run id is ellipsised with `…` in the list (b1). The LOG-line clipping without `…` (D8) is probably accepted: it matches the shipped output pane's no-Wrap behaviour. Not scored down.

### Pillar 5: Spacing (3/4)
- The 10-cell label column, 2-cell indents, the 4-cell LOG continuation and the narrow/wide run-row forms all hold at their tiers. Cap rows fit the 39-cell floor.
- WARNING: unwrapped single-line rows (answer warning D3, slug not-usable D8) overflow the pane at common widths and are cut hard, with no ellipsis. This conflicts with the spec's "long values are ellipsised" rule.

### Pillar 6: Experience Design (2/4)
- Strengths: context-sensitive footer, retired `i`, held-run refusals, `Loading...` states, default focus No on all popups, inert `y` until the host check returns, the `[not live]` LOG indicator.
- **WARNING (D5).** At 80x12 the launch form (`d2_launch_form_ready_80x12`) drops the Sandbox row, spend line and field hint, and the lint popup (`f3_lint_popup_80x12`) drops findings 4-6 and the question. `y` is still live, so a user can confirm a costly launch without seeing the spend ceiling or sandbox posture. Spec D-19/D-20 make those rows the point of the confirmation.
- D1 (stop-vs-kill ambiguity) and D2 (false warning on loading) also land here as decision-quality defects.
- Pillar score reflects D1 and D5 sitting on irreversible or costly actions.

---

## Needs human
- Density of the FULL tier at 100x30.
- DarkGray label contrast on the user's terminal theme.
- Glyph rendering of ‖ ◈ ⚑ in the user's font.

## Registry Safety
Not applicable (Rust/ratatui, no `components.json`).

## Files Audited
- src/ui/screens/confirm_popup.rs (139-161)
- src/ui/screens/driver_steer.rs (363-380, ~531-575)
- src/ui/screens/driver_run.rs (981-995, 2051-2060, 2472)
- src/ui/screens/driver_confirm.rs (445)
- /tmp/claude-1000/t/ui30/*_WxH.txt and *.style.txt
- 30-UI-SPEC.md, 30-CONTEXT.md and the 30-01 to 30-05 plans and summaries (as the contract and intent baseline)
