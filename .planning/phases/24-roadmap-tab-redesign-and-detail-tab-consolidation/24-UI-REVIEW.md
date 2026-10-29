# Phase 24 — UI Review

**Audited:** 2026-10-28 (re-audit at HEAD, after the workstream-band, lane-corner and gsd-core 1.15 parsing quick tasks)
**Baseline:** abstract 6-pillar standards, with MOCKUPS.md and 24-CONTEXT.md (D-A01..D-A15, D-B01..D-B12) as the design reference. No UI-SPEC.md exists.
**Screenshots:** not applicable. This is a ratatui TUI with no dev server or browser. Evidence is terminal renders: the real debug binary driven in detached tmux sessions against the sanitised fixtures in `tests/fixtures/roadmaps` (daily-vow at 120x36; sentriq, ttbook at 80x30; ttbook at 60x22), captured with `tmux capture-pane`. Sessions were killed afterwards and no source or tests were touched.
**Interaction captures:** off (`workflow.ui_interaction_capture` is false). Keys `h`, `Space`, `G`, `Enter`, `9`, `v` and `?` were driven through tmux and observed directly.

---

## Pillar Scores

| Pillar | Score | Key Finding |
|--------|-------|-------------|
| 1. Copywriting | 3/4 | The explanatory empty copy ("nothing declared", "no deps; can run any time", "none in wave N") matches D-A14. A roadmap-done phase with no phase directory is badged `[Not started]`. |
| 2. Visuals | 3/4 | The master/detail layout, lane graph, cursor and `⇡`/`⇣`/`∙` markers closely match Mockups A-C at 120, 80 and 60 columns. The stacked detail pane clips Goal to one row. |
| 3. Color | 3/4 | Status colour semantics follow D-A04/D-A05 (dim done, yellow active, green ready, cyan deps, magenta unblocks). Glyph shapes deliberately differ from the mockups. |
| 4. Typography | 3/4 | Hierarchy comes from reverse video, bold and dim: selected row, band headers, "Goal" label, dim done rows. It is consistent. Truncation with `…` is applied to names but also to Goal and the head line. |
| 5. Spacing | 3/4 | Columns align and the 80-column budget holds (every row measured exactly 80 cells). The `▸ shipped` row and the lane header do not share a lane-column ruler. |
| 6. Experience Design | 3/4 | Keys, folding, Enter-to-Phases, `v` toggle and dead digits 9/0 all behave per D-A10/D-B10. The footer hint line is truncated at 80 columns and loses `[?]help`. |

**Overall: 18/24**

---

## Top 3 Priority Fixes

1. **Footer key hint is cut off at 80 columns and below, hiding `[?]help`** — a user on a standard 80-column terminal cannot discover help, and the hint line already omits `Enter`, `g/G`, `[ ]` and `v` — Prioritise `[?]help` (and `Enter`) first in the Roadmap footer, drop `[e]nqueue` to the tail, or switch to a compact variant below roughly 100 columns. WARNING.
2. **Stacked detail pane clips Goal to a single `…`-truncated row at 80x30 and 60x22** — below 100 columns the Goal paragraph, which is the main reason to open the pane, is mostly unreadable (ttbook phase 8: "...enforces every hard limit from…"). `DETAIL_STACKED_ROWS` is 9, but it shrinks to `DETAIL_STACKED_MIN_ROWS` (6) when the list claims rows. Cap the list at a smaller share, or let Goal wrap and the Needs/Unblocks/Parallel lines drop first. WARNING.
3. **Stage badge contradicts the status for roadmap-done phases without a phase directory** — daily-vow phase 21 is `◍ done · 5/5 plans  [Not started]`. `disk_stage_label` maps `NoDirectory` to "Not started" (`src/ui/screens/detail.rs:2721`) even when ROADMAP and STATE say the phase is complete. This is the same class of bug as folded todo D-B12. Suppress the badge when the status is done and no directory exists, or word it "no phase dir". WARNING.

Further minor items (not top 3):
- Glyphs differ from D-A04 and the mockups (`◍` for ●, `►` for ▶, `⇡`/`⇣` for ↑/↓, `∙` for ·). They come from the central `src/ui/glyphs.rs`, chosen for width safety, so this is deliberate. MOCKUPS.md and D-A04/D-A05 should be annotated so that the contract and the build agree.
- The 24-VERIFICATION.md claims predate the band, lane-corner and parser changes. Re-verify rather than trust it.
- The Help overlay truncates its rows at 60 columns (pre-existing, outside this phase's contract).

---

## Detailed Findings

### Pillar 1: Copywriting (3/4)
- Detail pane copy at HEAD: "Needs  nothing declared   Unblocks  nothing", "Parallel  none in wave 1   (no deps; can run any time)", "Unblocks  nothing (last in M5)", "(implied via 21)", "planned (not a GSD phase)". These satisfy D-A14's "explain absence" rule (sentriq phase 12 and ttbook placeholder phases).
- Start-now line, band headers (`▾ v1.5 Closing the Loop  5/6 ━━━`) and the shipped row (`▸ v1.0 … v1.4   5 milestones · 17 phases shipped`) match D-A06/D-A07 and Mockup B.
- Tab labels match D-B01/D-B11: `1:Roadmap | 2:Phases | 3:Backlog | 4:Git | 5:Queue | 6:Sess | 7:Cfg | 8:Docs`, compact `1:Rd … 8:Dc` at 60 columns.
- Defect: `[Not started]` on a done phase (see fix 3). `src/ui/screens/detail.rs:2721`.
- Defect: the singular is handled ("1 milestone · 8 phases shipped"), but the active phase head "plans TBD  [Not started]" stacks two status-like phrases on one line.

### Pillar 2: Visuals (3/4)
- The focal point is clear: the reverse-video row with `►`, mirrored by the titled detail pane ("Phase N"). Dependencies and unblocked phases are marked inline with `⇡`/`⇣` (daily-vow after `h`: 20 `⇡`, 22/23 `⇣`, 21 `►`).
- The lane graph is clean: `├─╮`, `├─╯`, `├─┼─╮`, and the ttbook workstream bands (Milestone 3/4/5) nest under the lane column with no duplicated nodes (D-A14 satisfied for daily-vow 20 and sentriq 9).
- Folding a band with `Space` collapses its rows, re-titles the detail pane to the milestone ("5/6 phases done · Space fold/unfold") and flips `▾` to `▸`.
- Defect: the stacked layout at 80x30 gives the list roughly 15 rows and the pane 6, so Goal is a single truncated row. At 60x22 the list shows only 5 rows. It is usable but cramped.
- Wide layout (120x36): the detail pane leaves about 12 blank rows below its content. That is acceptable, but Goal could use the space.

### Pillar 3: Color (3/4)
- Roadmap view uses only named ANSI colours (`Cyan`, `Magenta`, `Green`, `Yellow`, `Red`, `DarkGray`, `src/ui/roadmap_view.rs:950-3174`); no hardcoded RGB, so it follows the user's terminal theme.
- Semantics match D-A04/D-A05. Lane colours come from the shared `LANE_PALETTE`. The accent (yellow/bold) is confined to the active phase.
- Deviation: the unmerged-worktree glyph is cyan and shares cyan with the dep markers. Distinguishable by shape, but colour alone does not separate them.
- Colour-only information: ready (`○` green) versus blocked (`◌`) also differ in shape, so monochrome terminals still work.

### Pillar 4: Typography (3/4)
- Terminal "type" is limited to reverse, bold and dim. Hierarchy: band headers, the "Goal" label, the selected row reversed, done rows dim. Applied consistently across 120/80/60 columns.
- Truncation is by chars and ends in `…` (`Notification Scheduling Extraction & Dete…`, `Autonomous responder, staged hold and remote con…`). It is correct and cannot panic on multibyte text. It also truncates Goal and the pane head line at narrow widths (see fix 2).

### Pillar 5: Spacing (3/4)
- Every row measured exactly 80 cells at 80 columns, including band rows and wide glyph rows. No overflow past the border.
- The column header `lanes  #  Phase  plans  wave` aligns with the data rows. Gap after the lane column is stable between lane-count changes.
- Defect: the shipped row (`▸ v1   1 milestone · 8 phases shipped`) starts text at column 3 while the header's `#` column sits at column 14, so it reads as a different grid. The mockups have the same quirk, so this is minor.
- Defect: at 60 columns the Needs/Unblocks line packs two fields onto one row (`Needs ◌ 9  ◉ 8 (implied via 9)   Unblocks  ◌ 12`) with a three-space gap, and the head line clips mid-phrase (`◌ blocked …`).

### Pillar 6: Experience Design (3/4)
- Verified live: `h` jumps to a dep and the pane follows; `Space` folds; `G` goes to the bottom and the list scrolls to keep the cursor visible; `Enter` opens the Phases tab; `9` is a no-op (D-B10); `v` toggles to the box view; `?` opens help with Roadmap key rows.
- Tab bar is `[1:Roadmap]|` and `[1-8]jump` matches the eight-tab consolidation. The Driver tab is absent without the experimental flag (D-B10).
- Defect: the footer hint is clipped at 80 columns (`[e]nque`) and 60 columns (`[`), losing `[?]help`. It also never mentions `Enter`, `g/G`, `[`/`]` or `v` (fix 1).
- Defect: a done phase with no directory reads "Not started" (fix 3).
- Empty states: the registered-project fixtures all carried roadmaps. The "no roadmap data" path was not driven here. Whether it is covered by the existing render tests is unverified.
- Mouse click-to-select is out of scope per D-A11, so it is not scored.

---

## Files Audited
- `src/ui/roadmap_view.rs`, `src/ui/roadmap_graph.rs` (constants, facts, colours)
- `src/ui/screens/detail.rs` (`disk_stage_label`, tab bar, `render_roadmap`)
- `src/ui/glyphs.rs`
- `.planning/phases/24-roadmap-tab-redesign-and-detail-tab-consolidation/` MOCKUPS.md, 24-CONTEXT.md, 24-0{1..7}-SUMMARY.md and PLAN.md (design reference)
- `tests/fixtures/roadmaps/*` (renders via the real binary)
