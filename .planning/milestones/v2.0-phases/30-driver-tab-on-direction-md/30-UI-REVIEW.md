# Phase 30 — UI Review (refresh at HEAD e9de71b, master)

**Audited:** 2026-10-29 (HEAD e9de71b)
**Baseline:** 30-UI-SPEC.md (approved, terminal/ratatui medium)
**Screenshots:** not captured (TUI, no dev server). Evidence is the ratatui TestBackend render tests carried from the prior audit (`rtk proxy cargo test --no-fail-fast --lib ui::screens`: 853 passed, 0 failed). Not re-run in this refresh because no rendering code changed.
**Interaction captures:** off (workflow.ui_interaction_capture is false). Interaction findings come from key-driven unit tests and code.

---

## Refresh at e9de71b (2026-10-29)

**Nothing UI-facing changed versus the previous audit (0791cae).**

- `git diff --stat 0791cae HEAD -- src tests` shows only `src/launcher/config_dir.rs` (+281/-19 lines) and `tests/launcher_config_dir.rs` (+50). `git diff --stat 0791cae HEAD -- src/ui` is empty.
- The diff comes from quick task 261028-v2z (launcher managed-settings merge). The added string literals are a `PERMISSION_LISTS` constant (`allow`, `ask`, `deny`, `additionalDirectories`), JSON settings keys, and test-assert messages (`"host doc {doc} changed the managed settings"`). None is a rendered TUI string, widget, or status text.
- `[inferred — cleanup 2026-10-28]`: a full test re-run was skipped. No `src/ui/` file changed, so the 853-pass baseline carries over.
- Scores, findings and the line references below are unchanged: **22/24**, no BLOCKER, no new HIGH. The `driver_steer.rs` and `detail.rs` evidence was not touched by any commit since 0791cae.

---

## Prior findings status (re-checked at HEAD)

| ID | Prior finding | Status at HEAD | Evidence |
|----|---------------|----------------|----------|
| D1 | Polite-stop popup clips "Nothing is killed." | Still fixed | `driver_steer` width-matrix test; file unchanged. |
| D2 | False Yellow "(no control line)" while the body loads | Still fixed | `driver_run` Loading tests; file unchanged. |
| D3 | D-16 answer warning clipped | Still fixed | `driver_steer.rs:125-129`, shortened copy. |
| D4 | Contract copies lose key hints | Partly fixed, unchanged | See Pillar 1. |
| D5 | Clipped popups keep a live `y` | Still fixed | `confirm_unless_clipped` guard tests. |
| D6 | Kill confirm repaints the dashboard | Still fixed | `driver_confirm.rs` uses `render_main_only`. |
| D7 | Popup background re-lays out to a different height tier | Still open, cosmetic | Not changed. |
| D8 | Slug not-usable row clipped | Still fixed | Wrap test. |
| Legacy section | Collapsed legacy runs, `L` key, legacy detail | OBSOLETE, deleted by Phase 31 (D-03) | `driver_run.rs:3033`; negative tests `detail.rs:20978`, `help.rs:759-777`. |

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

**Overall: 22/24** (unchanged). Two pillars sit at 3, so there is no BLOCKER, and no pillar is below 3.

---

## Top 3 Priority Fixes

1. **Shorten or wrap the long success messages** (WARNING).
   - `driver_steer.rs:132` ("Answer written to {} and hold set to false. The driver re-reads it before its next dispatch.") and `driver_steer.rs:147` run about 90 characters.
   - `driver_steer.rs:615` ("DIRECTION.md updated. The driver re-reads it before its next dispatch.") runs about 70.
   - The control-written strings at `driver_steer.rs:105,108` run 80 to 95.
   - They are drawn in `detail.rs::tab_bar_block` (about line 1301-1318), which applies `fit_cells(.., room)` and drops the tail ("before its next dispatch", "It applies at the next launch").
   - The head carries the key fact, so this is not a blocker.
   - Fix: apply the D-16 treatment (shorter copy) and add a fit test per string. The spec fixes some of these strings, so the change is `[inferred — cleanup 2026-10-28]`.
2. **Mark the clipped end of long LOG lines with `…`.** The spec says long values are ellipsised. LOG lines in the no-Wrap output pane are hard-cut. Either ellipsise, or record the deviation in the UI-SPEC.
3. **Update 30-UI-SPEC.md to match Phase 31.** The spec still documents the deleted legacy section (`L` key, `▸ legacy runs` rows, legacy height rule, `L` help row, `[L]egacy` footer token). The code and tests assert the opposite. This is contract drift, not a UI defect. Mark the legacy items superseded by Phase 31 D-03.

(Cosmetic, deferred: D7, rendering the popup background at the screen's own height tier.)

---

## Detailed Findings

### Pillar 1: Copywriting (3/4)
- The prior blocker and warning copy defects stay closed, each with a width-matrix test.
- WARNING: the success messages in fix 1 exceed the status room at 80 columns.
- Minor: footer CTAs stay verb-only (`[s]tart`), as in the spec's own contract.
- No generic labels ("Submit", "Click here", "Something went wrong") were found in the Driver screens.

### Pillar 2: Visuals (4/4)
- The orphan sweep passes at heights 1 to 40. Status glyph/word/colour triples are intact. The legacy visual surface is gone. Residual: D7, cosmetic.

### Pillar 3: Color (4/4)
- Cyan appears only on selection rows. Yellow, Red and Magenta follow the semantic table. Loading is DarkGray, not Yellow. No colour is chosen from LOG content.

### Pillar 4: Typography (4/4)
- Regular and BOLD only. REVERSED stays on the tab label and the popup button. No `Wrap` in scrolling panes.

### Pillar 5: Spacing (3/4)
- The 10-cell label column, 2-cell indents and the 39-cell floor are covered by passing tests.
- Remaining: the un-ellipsised LOG clip (fix 2) and D7.

### Pillar 6: Experience Design (4/4)
- The clipped-popup guard keeps `y` and Enter-on-Yes inert while cut off; `n` and Esc still cancel.
- Held-run and unknown-liveness refusals, the default focus No, and the Loading states are covered by tests.
- The retired `i` key shows a message and opens nothing.

---

## Needs human (subjective only)
- Density of the FULL tier at 100x30.
- DarkGray and DIM label contrast on the user's terminal theme.
- Glyph rendering of ‖ ◈ ⚑ ◇ ⧫ ⬖ in the user's font.

## Registry Safety
Not applicable (Rust/ratatui, no `components.json`).

## Files Audited
- src/ui/screens/driver_steer.rs (success copy, 100-150 and 615)
- src/ui/screens/detail.rs (`tab_bar_block`, 1295-1320; legacy-absence test at 20978)
- src/ui/screens/driver_run.rs (Phase 31 deletion note at 3033, orphan sweep)
- src/ui/screens/driver_confirm.rs and confirm_popup.rs
- src/ui/screens/help.rs (759-777)
- src/launcher/config_dir.rs and tests/launcher_config_dir.rs (diff only, confirmed non-UI)
- 30-UI-SPEC.md and the prior 30-UI-REVIEW.md

## Delta vs prior refresh (e9de71b) — no re-audit

`[inferred — cleanup 2026-10-28]` Existing review kept (the workflow's unattended path reuses it; the auditor was not re-spawned) because quick 261029-1dp (f4cfa01, ce86206) has no user-visible TUI surface:
- `git diff --stat e9de71b HEAD -- src/ui src/app.rs`: only `src/ui/screens/detail.rs` (test doc comment + assert message in `mod tests`, ~21257-21277) and `src/ui/screens/mod.rs:1389-1393` (doc comment on `AppContext::experimental`). No rendered string, layout, style or key binding changed.
- The corrected wording ("opt-in gates no launch; it only shapes the push policy") already matches the rendered copy: `driver_confirm.rs:118,132` speak of PR caps / the push step, and `help.rs:370,627,980` say "Toggle driver opt-in for this project". No copy drift.
- f4cfa01 (SAFE-04 stdout/stderr redaction pump) touches only `src/launcher/`; ce86206 drops the dead `AgentRuntime` key, and `grep -rn AgentRuntime src/ui` is empty.
- `cargo test --no-fail-fast --lib ui::screens`: 853 passed, 0 failed.

Scores unchanged: 22/24, 0 blockers; top fixes as listed above. Method: git diff + grep + render tests.
