# Phase 30 — UI Review (re-audit at HEAD, master, after 28.1, Phase 31 and quick 261028-n0u)

**Audited:** 2026-10-28
**Baseline:** 30-UI-SPEC.md (approved, terminal/ratatui medium)
**Screenshots:** not captured (TUI, no dev server). Evidence is the ratatui TestBackend render tests. `rtk proxy cargo test --no-fail-fast --lib ui::screens` ran 853 passed, 0 failed. That includes the orphan-rule sweep (`driver_run::tests::launcher_orphan_sweep_no_rule_or_header_without_content_at_any_height`) and the popup, steer and launch width-matrix tests. The tmux-driven binary was not used. No temporary files were written.
**Interaction captures:** off (workflow.ui_interaction_capture is false). Interaction findings come from key-driven unit tests and code.

---

## Prior findings status (re-checked at HEAD)

| ID | Prior finding | Status at HEAD | Evidence |
|----|---------------|----------------|----------|
| D1 | Polite-stop popup clips "Nothing is killed." | Still fixed | Width-matrix test in `driver_steer` passes. |
| D2 | False Yellow "(no control line)" while the body loads | Still fixed | `driver_run` Loading/no-control-line tests pass. |
| D3 | D-16 answer warning clipped | Still fixed | The wrap test passes. The copy is now shorter: `"{item} answered; {rule} parks it: [e] budget, [R] caps."` (`driver_steer.rs:125-129`). It was shortened to fit the 62-cell status room. |
| D4 | Contract copies lose key hints | Partly fixed, unchanged | See Pillar 1. |
| D5 | Clipped popups keep a live `y` | Still fixed | The `confirm_unless_clipped` guard tests pass. |
| D6 | Kill confirm repaints the dashboard | Still fixed | `driver_confirm.rs` routes the kill confirm to `render_main_only`. The test passes. |
| D7 | Popup background re-lays out to a different height tier | Still open, cosmetic | Not changed. |
| D8 | Slug not-usable row clipped | Still fixed | The wrap test passes. |
| Legacy section | Collapsed legacy runs, `L` key, legacy detail (WR-05 height rule) | OBSOLETE, code deleted by Phase 31 (D-03) | `driver_run.rs:3033` doc comment; the negative tests `detail.rs:20978` ("no legacy section") and `help.rs:759-777` pass. |

---

## Pillar Scores

| Pillar | Score | Key Finding |
|--------|-------|-------------|
| 1. Copywriting | 3/4 | Four success messages of roughly 90 characters are cut by `fit_cells` in the tab-bar title row. |
| 2. Visuals | 4/4 | Tiers, glyph triples, popup layering and the orphan sweep pass. |
| 3. Color | 4/4 | Cyan is selection-only. There is no false warning. |
| 4. Typography | 4/4 | Two weights only. There is no `Wrap` in scroll panes. |
| 5. Spacing | 3/4 | LOG lines are cut without `…`, and the popup background tier mismatch remains. Both are minor. |
| 6. Experience Design | 4/4 | A cut-off confirmation cannot be confirmed. Loading is distinct from "no control line". |

**Overall: 22/24** (unchanged from the prior review). Two pillars sit at 3, so there is no BLOCKER, and no pillar is below 3.

---

## Top 3 Priority Fixes

1. **Shorten or wrap the long success messages** (WARNING).
   - `driver_steer.rs:132` ("Answer written to {} and hold set to false. The driver re-reads it before its next dispatch.") and `driver_steer.rs:147` run about 90 characters.
   - `driver_steer.rs:615` ("DIRECTION.md updated. The driver re-reads it before its next dispatch.") runs about 70.
   - The control-written strings at `driver_steer.rs:105,108` run 80 to 95.
   - They are drawn in `detail.rs::tab_bar_block` (about line 1301-1318). That function applies `fit_cells(.., room)` and truncates to `width - project - 3`, so the tail is lost. The tail is "before its next dispatch" and "It applies at the next launch".
   - The head carries the key fact, so this is not a blocker.
   - Fix: apply the treatment used on the D-16 copy and add a fit test per string.
   - The spec fixes some of these strings, so the change is `[inferred — cleanup 2026-10-28]`.
2. **Mark the clipped end of long LOG lines with `…`.** The spec says long values are ellipsised. LOG lines in the no-Wrap output pane are hard-cut. Minor. Either ellipsise, or record the deviation in the UI-SPEC.
3. **Update 30-UI-SPEC.md to match Phase 31.** The spec still documents the deleted legacy section (`L` key, `▸ legacy runs` rows, the legacy height rule, the `L` help row, the `[L]egacy` footer token). The code and tests now assert the opposite. This is contract drift, not a UI defect. Mark the legacy items superseded by Phase 31 D-03.

(Cosmetic, deferred: D7, rendering the popup background at the screen's own height tier.)

---

## Detailed Findings

### Pillar 1: Copywriting (3/4)
- The prior blocker and warning copy defects stay closed, each with a width-matrix test.
- WARNING: the success messages in fix 1 exceed the status room at 80 columns. The shortened D-16 copy shows the approach that works.
- Minor: the footer CTAs stay verb-only (`[s]tart`), as in the spec's own contract. The spec tokens drop by priority, and that behaviour is unchanged.
- No generic labels ("Submit", "Click here", "Something went wrong") were found in the Driver screens. The refusal copy is specific.

### Pillar 2: Visuals (4/4)
- The orphan sweep passes at heights 1 to 40.
- Status glyph/word/colour triples are intact.
- The legacy-section visual surface is gone, so nothing in the tab now diverges from the new run-list model.
- Residual: D7, cosmetic.

### Pillar 3: Color (4/4)
- Cyan appears only on selection rows, as in the spec's reserved list.
- Yellow, Red and Magenta follow the semantic table.
- The Loading state is DarkGray, not Yellow.
- No colour is chosen from LOG content.

### Pillar 4: Typography (4/4)
- Regular and BOLD only. REVERSED stays on the tab label and the popup button.
- No `Wrap` in scrolling panes.
- Popup and footer-input wrapping is spec-permitted.

### Pillar 5: Spacing (3/4)
- The 10-cell label column, 2-cell indents and the 39-cell floor are covered by the passing tests.
- Remaining: the un-ellipsised LOG clip (fix 2) and D7.

### Pillar 6: Experience Design (4/4)
- The clipped-popup guard keeps `y` and Enter-on-Yes inert while cut off, and `n` and Esc still cancel.
- Held-run and unknown-liveness refusals, the default focus No, and the Loading states are covered by passing tests.
- The retired `i` key shows a message and opens nothing.
- Phase 31 removed the legacy read-only dead branch, and the key-handling tests still pass.

---

## Needs human (subjective only)
- Density of the FULL tier at 100x30.
- DarkGray and DIM label contrast on the user's terminal theme.
- Glyph rendering of ‖ ◈ ⚑ ◇ ⧫ ⬖ in the user's font.

## Registry Safety
Not applicable (Rust/ratatui, no `components.json`).

## Files Audited
- src/ui/screens/driver_steer.rs (success copy, 100-150 and 615)
- src/ui/screens/detail.rs (`tab_bar_block`, status message at 1295-1320; legacy-absence test at 20978)
- src/ui/screens/driver_run.rs (the Phase 31 legacy deletion note at 3033, the orphan sweep)
- src/ui/screens/driver_confirm.rs and confirm_popup.rs (guards, background)
- src/ui/screens/help.rs (legacy-absence tests, 759-777)
- 30-UI-SPEC.md and the prior 30-UI-REVIEW.md
