# Phase 30 — UI Review (re-audit after gap-closure plans 30-06..30-09)

**Audited:** 2026-10-21
**Baseline:** 30-UI-SPEC.md (approved, terminal/ratatui medium)
**Screenshots:** not captured (TUI, no dev server). Evidence is the TestBackend render tests, which assert on drawn buffer text and cell styles at 50x24, 60x16, 80x12, 80x24, 100x30, 120x40 and 160x40. `cargo test --no-fail-fast --lib ui::screens::confirm_popup ui::screens::driver_` ran 259 passed, 0 failed. No throwaway files were written.
**Interaction captures:** off (workflow.ui_interaction_capture is false). Interaction findings come from key-driven unit tests and code.

---

## Prior findings status

| ID | Prior finding | Status | Evidence |
|----|---------------|--------|----------|
| D1 | BLOCKER: polite-stop popup clips "Nothing is killed." | FIXED | `confirm_popup.rs` `wrapped_rows` now uses `Paragraph::line_count` with `Wrap{trim:false}`, the same wrapper the body is drawn with. `driver_steer::launcher_steer_the_polite_stop_sentence_survives_every_width` asserts the full prompt and a blank spacer row at 50, 60, 80x12, 80x24, 100, 120 and 160 columns. It passes. |
| D2 | False Yellow "(no control line)" while the body loads | FIXED | `driver_run::launcher_view_full_header_says_loading_until_the_detail_arrives` asserts `control Loading...` in DarkGray with the heartbeat kept. `..._flags_a_body_without_a_control_line` keeps Yellow for the real case. |
| D3 | D-16 answer warning clipped at 80-120 | FIXED | `launcher_steer_the_answer_warning_wraps_to_its_last_word` asserts the warning ends in `[R].` at 80, 100 and 120 columns, and the Red error row wraps too. |
| D4 | Injection-retired, answer and left-unlaunched copies lose their key hints | PARTIALLY FIXED | The three named copies were shortened, with fit tests (`..._retired_copy_fits_an_80_column_status_bar`, `..._park_answer_copy_fits_the_status_bar`, `..._unlaunched_copy_fits_the_status_bar`). The other contract strings still run about 90 characters (see Pillar 1). |
| D5 | Popups at short heights hide decision content while `y` stays live | FIXED | `confirm_popup::confirm_unless_clipped` makes `y` and Enter-on-Yes inert while clipped. The hint row reads `Cut off: enlarge the terminal to confirm`. The launch form orders its rows spend, sandbox, blockers, fields, then findings. At 80x12 the spend and sandbox rows survive and the form is non-confirmable (`launcher_launch_a_form_cut_off_at_80x12_launches_nothing`). The lint popup keeps `Edit it again now?` and a `… k more` row (`..._lint_popup_keeps_its_question_at_80x12`). |
| D6 | Kill confirm repaints the dashboard behind it | FIXED | `driver_confirm.rs:445-452` routes `StopLauncher` to `DetailScreen::render_main_only`. `launcher_kill_confirm_keeps_the_driver_tab_behind_it` passes. |
| D7 | Popup background re-lays out to a different height tier | STILL OPEN (cosmetic) | Not changed. The background is dimmed. Not scored down. |
| D8 | Slug not-usable row clipped at 80 columns | FIXED | The row now wraps. `launcher_new_run_the_not_usable_copy_wraps_to_its_last_word` passes, and the spec string was updated. |
| Not observed | Stale heartbeat, MISMATCH, UNSANDBOXED, legacy section, edit conflict chooser | Covered by unit tests per the SUMMARYs. Not re-driven visually. |

---

## Pillar Scores

| Pillar | Score | Key Finding |
|--------|-------|-------------|
| 1. Copywriting | 3/4 | All prior clipped-copy defects are closed. The four generic success messages (about 90 chars) still exceed the 62-cell status room at 80 columns. |
| 2. Visuals | 4/4 | Tiers, glyph triples, no-orphan rule and popup layering conform. The kill popup now sits over the Driver tab. |
| 3. Color | 4/4 | Cyan is selection-only. The false warning is gone. Footer keys are BOLD default. |
| 4. Typography | 4/4 | Two weights only, no `Wrap` in scroll panes. Popup bodies wrap, as the spec allows. |
| 5. Spacing | 3/4 | Label column, indents and tiers hold. The LOG-line clip without `…` and the popup-background tier mismatch remain, both minor. |
| 6. Experience Design | 4/4 | A cut-off confirmation cannot be confirmed. The launch form leads with spend and sandbox. Loading is distinct from "no control line". |

**Overall: 22/24**

---

## Top 3 Priority Fixes

No BLOCKER or pillar score of 1 remains. All remaining items are WARNING or minor.

1. **Shorten the generic success copies, or let the status bar wrap or marquee them** (WARNING). "control written", "Answer written", "hold set to false" and "DIRECTION.md updated" run about 90 characters (`driver_steer.rs:102,127,142,610`). At 80 columns they are cut at about 62 cells, so the tail "...before its next dispatch" is lost. The key facts are in the head, so this is not a blocker. Fix: apply the 30-06/07/09 treatment, with a fit test per string. Note the spec fixes these strings, so the change is `[inferred — audit]`.
2. **Mark the clipped end of long LOG lines** (minor, D8 remnant). LOG lines are hard-cut without `…` where the spec says long values are ellipsised. This matches the shipped no-Wrap output pane and was accepted before. If kept, record it as a deliberate deviation in the UI-SPEC.
3. **Render popup backgrounds at the screen's own height tier** (minor, D7). At 80x24 the Driver tab behind the popup is in a different tier than the tab itself. It is dimmed and cosmetic. Defer.

---

## Detailed Findings

### Pillar 1: Copywriting (3/4)
- Every prior blocker and warning is verified closed (see the status table), each with a width-matrix test.
- WARNING: the four generic success strings listed in fix 1 exceed the status room at 80 columns. This is the only remaining copy defect.
- Minor: the footer CTAs stay verb-only (`[s]tart`). This is the UI-checker FLAG, unchanged. At 80 columns `[s]tart` is dropped first when a run is selected, per the spec priority table.
- Conformant: the Cut-off hint, the spend line (a documented deviation), the polite-stop vs kill copy and the sandbox rows.

### Pillar 2: Visuals (4/4)
- Kill popup over the Driver tab (D6). The polite-stop popup is complete at every tested size. The status glyph/word/colour triples, the item cursor idiom and the width and height tiers are unchanged from the prior audit, which found no orphan rules.
- Residual: D7, cosmetic.

### Pillar 3: Color (4/4)
- Cyan appears only on the selected run row, the item cursor row and the launch-form field cursor. The header control row renders DarkGray `Loading...` while the body loads, and Yellow only for a genuinely missing control line. Yellow accents on the polite-stop and launch popups and Red on kill are as contracted.

### Pillar 4: Typography (4/4)
- Regular and BOLD only. The answer footer and the slug row now wrap in a footer input, not in a scrolling pane, so the no-`Wrap` rule is not violated.

### Pillar 5: Spacing (3/4)
- The 10-cell label column, 2-cell indents, the 4-cell LOG continuation and the cap rows at the 39-cell floor hold. The clipped answer warning and slug rows are fixed. Remaining: D7 and the un-ellipsised LOG clip.

### Pillar 6: Experience Design (4/4)
- The 30-06 guard makes the cut-off state structural: `y`, Enter-on-Yes and a Yes click are inert, while `n` and Esc still cancel. Tests cover the polite stop (`launcher_steer_a_clipped_polite_stop_writes_nothing_on_y`) and the launch form.
- The launch form holds the D-19/D-20 essentials in view before findings. Findings are clipped first, with a `… k more` count.
- Retired `i` names the replacement keys. Held-run refusals, `Loading...` states, default focus No and the inert `y` before the host check are intact.
- The 30-08 `$EDITOR`-with-arguments and ETXTBSY fixes remove an editing dead-end. They were verified by their unit tests only.

---

## Needs human (subjective or unobservable only)
- Density of the FULL tier at 100x30.
- DarkGray label contrast on the user's terminal theme.
- Glyph rendering of ‖ ◈ ⚑ ◇ in the user's font.

## Registry Safety
Not applicable (Rust/ratatui, no `components.json`).

## Files Audited
- src/ui/screens/confirm_popup.rs (wrapped_rows, popup_rect, on_key, clip guard)
- src/ui/screens/driver_steer.rs (polite stop, answer warning, success copies at 102-142 and 610)
- src/ui/screens/driver_launch.rs (launch form, lint popup, slug row, 80x12 tests)
- src/ui/screens/driver_run.rs (FULL header Loading/no-control-line tests)
- src/ui/screens/driver_confirm.rs (kill confirm background, 436-452)
- src/ui/screens/detail.rs (retired-key copy test)
- 30-UI-SPEC.md, the prior 30-UI-REVIEW.md, and `git log 8035409..HEAD`
