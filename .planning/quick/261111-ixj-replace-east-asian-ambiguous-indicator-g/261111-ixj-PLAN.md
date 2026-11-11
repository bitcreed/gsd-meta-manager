---
phase: quick-261111-ixj
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/ui/glyphs.rs
  - src/agents/unmerged.rs
  - src/agents/waves.rs
  - src/ui/screens/help.rs
  - src/ui/roadmap_graph.rs
  - src/ui/roadmap_view.rs
  - src/app.rs
  - src/ui/screens/normal.rs
  - src/ui/screens/detail.rs
  - tests/agents_unmerged.rs
  - tests/agents_waves.rs
  - tests/agents_fixers.rs
  - docs/GETTING-STARTED.md
  - .planning/milestones/v2.0-phases/30-driver-tab-on-direction-md/30-UI-SPEC.md
  - .planning/todos/pending/2026-11-11-registry-glyphs-overflow-under-font-fallback.md
autonomous: true
requirements: [QUICK-261111-ixj]

estimate:
  tokens: 95000
  raw_tokens: 95000
  tasks: 3
  confidence: low

must_haves:
  truths:
    - "The overview Status cell draws the unmerged count as U+2206 INCREMENT followed by the count: `∆1 ∙ 1 stalled`, `1run ∙ ∆30`. U+2206 is East Asian Width Neutral, 1 cell under unicode-width `width` and `width_cjk`, not Emoji, and present in both Ubuntu Sans Mono and DejaVu Sans Mono. No fallback font draws it, so it no longer covers the digit after it (INFERRED-1, INFERRED-2)."
    - "Every surface that draws unmerged or delivered work reads the one registry constant `glyphs::INCREMENT` through `agents::unmerged::GLYPH_UNMERGED`: the dashboard Status cell, the Sessions Agents summary, the Phases roadmap graph and Waves pane, the help legend and the Driver `delivered` row. The U+25D2 escape survives only in the `FORMER` list in `src/ui/glyphs.rs`, and the literal character no longer appears anywhere under src/, tests/ or docs/ (INFERRED-2)."
    - "Every Status-cell ladder (executor, fixer, `N agents`, idle unmerged/stalled, the worktree-phase tag) separates its segments with U+2219 BULLET OPERATOR, which is EAW Neutral and in both fonts. The production code in `src/agents/waves.rs` draws no TYPOGRAPHY code point. U+00B7 stays exempt TYPOGRAPHY in free text elsewhere (INFERRED-3)."
    - "The measured cause is recorded in the SUMMARY. U+25D2 is EAW Neutral, which disproves the Ambiguous hypothesis. The real cause is font fallback: Ubuntu Sans Mono (the user's Ptyxis font via `use-system-font`) has no U+25D2, so fontconfig substitutes the proportional DejaVu Sans, whose glyph is 0.873 em against a 0.56 em cell (1.56 cells). VTE draws it at its natural width, over the next cell (INFERRED-1)."
    - "A regression guard fails if a status glyph that would spill out of its cell is drawn flush against following text. `glyphs::FLUSH_SAFE` is the measured list. A source scan rejects any registry glyph drawn flush that is not in it, whether as an escape, a literal or a `{NAME}` interpolation. A Status-cell form test rejects any char that is not ASCII and not in the registry, and any non-flush-safe glyph that touches text. A negative-control test proves the scanner flags a synthetic flush draw (INFERRED-4)."
    - "Gates: `rtk bypass cargo test --no-fail-fast` exits 0 with at least 3822 passed and 0 failed, read from a log file and never piped through grep. `cargo clippy --all-targets -- -D warnings` exits 0. For every release.yml matrix target (this includes x86_64-pc-windows-gnu), `cargo check --locked --target T --all-targets` and `cargo clippy --locked --target T --all-targets -- -D warnings` exit 0. `cargo fmt --all -- --check` exits 0, and `git status` is clean."
  artifacts:
    - path: src/ui/glyphs.rs
      provides: "INCREMENT (U+2206) in place of the former half-circle entry; FLUSH_SAFE measured list; flush source scan; Status-cell no-typography scan; negative control"
      contains: "pub const FLUSH_SAFE"
    - path: src/agents/unmerged.rs
      provides: "GLYPH_UNMERGED = U+2206"
      contains: "\\u{2206}"
    - path: src/agents/waves.rs
      provides: "Status-cell ladders separated by U+2219; every_status_cell_form_is_width_safe_and_flush_safe"
      contains: "every_status_cell_form_is_width_safe_and_flush_safe"
    - path: .planning/todos/pending/2026-11-11-registry-glyphs-overflow-under-font-fallback.md
      provides: "Measured inventory of the registry glyphs that still fall back and overflow, for an operator decision"
  key_links:
    - from: src/agents/unmerged.rs
      to: src/ui/glyphs.rs
      via: "glyphs_defined_outside_ui_are_registry_glyphs asserts GLYPH_UNMERGED == INCREMENT"
      pattern: "GLYPH_UNMERGED, INCREMENT"
    - from: src/agents/waves.rs
      to: src/ui/screens/normal.rs
      via: "AgentView::summary_forms -> dashboard Status cell (and detail.rs Agents sub-view summary)"
      pattern: "summary_forms"
    - from: src/ui/glyphs.rs
      to: src/agents/waves.rs
      via: "source scan reads src/agents/waves.rs for flush draws and TYPOGRAPHY"
      pattern: "src/agents/waves.rs"
---

<objective>
Fix the overlap the user sees in the overview Status column, where cells like `∆1 ∙ 1 stalled` and `1run ∙ ∆30` used to draw the old half-circle glyph over the digit after it. The fix replaces the offending glyph with one that is both unambiguous and covered by the font. It decides the fate of U+00B7 in the Status cell. It adds regression guards so a spilling glyph cannot be drawn flush against text again.

Purpose: the dashboard is the product's core value ("see the state of every GSD project at a glance"). A count you cannot read defeats that.

Output: a registry swap (U+25D2 to U+2206 INCREMENT, with the constant renamed to `INCREMENT`), the Status-cell separator swap (U+00B7 to U+2219), updated test pins and docs, four new guard tests in `src/ui/glyphs.rs`, one new Status-cell form test in `src/agents/waves.rs`, an amended archived UI-SPEC, and one new pending todo for the operator.
</objective>

<execution_context>
@~/.claude/gsd-core/workflows/execute-plan.md
@~/.claude/gsd-core/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@CLAUDE.md
@src/ui/glyphs.rs
@src/agents/unmerged.rs
@.planning/todos/pending/2026-10-21-ambiguous-wide-typography-borders-and-darkgray.md

Execution is sequential on the main tree (master). No worktree. The user is unavailable. Every decision below marked INFERRED was made by the planner from code and measurements, and is listed for audit.

## Verified cause (planner measurement, 2026-11-11; executor re-verifies in Task 1)

The task hypothesis was that U+25D2 is East Asian Width Ambiguous. **That is false.**
- `python3 -I -c "import unicodedata as u; print(u.east_asian_width(chr(0x25D2)))"` prints `N` (Unicode 16.0.0). Today's `every_registry_glyph_is_width_safe` passes with U+25D2 in the registry, so icu_properties agrees.
- The user's terminal is Ptyxis (VTE) running under herdr (`TERM_PROGRAM=herdr`). `gsettings get org.gnome.Ptyxis use-system-font` is `true`, and `gsettings get org.gnome.desktop.interface monospace-font-name` is `'Ubuntu Sans Mono 11'`.
- `fc-list ':file=/usr/share/fonts/truetype/ubuntu/UbuntuSansMono[wght].ttf:charset=25d2' family` prints nothing: the font lacks the glyph. `fc-match -f '%{family[0]}' 'Ubuntu Sans Mono:charset=25d2'` prints `DejaVu Sans`, a PROPORTIONAL fallback.
- hmtx advances (scratch cmap/hmtx reader at /tmp/claude-1000/-home-blk-projects-rust-gsd-meta-manager/0cd7998f-ef33-4529-8495-22bdf67b5fde/scratchpad/adv.py; run `python3 -I adv.py <mono.ttf> <fallback.ttf> <chars>`): the Ubuntu Sans Mono cell is 0.56 em, and DejaVu Sans draws U+25D2 at 0.873 em, which is 1.56 cells. VTE draws the fallback glyph at its natural width, so it covers the next cell. In `∆30` that cell is a digit.
- The overview badges (`src/ui/screens/normal.rs` `BADGE_*`) also fall back and overflow (for example U+2717 at 1.5 cells). Each badge constant ends in a space, though, so its overflow lands on blank space. GLYPH_UNMERGED is the one registry glyph drawn flush against a count.

## Glyph inventory (measured against Ubuntu Sans Mono primary, DejaVu Sans fallback)

- Primary-covered, 1.00 cell: U+25CA, U+01C1, U+25CC, U+2219, U+2713, U+2039, U+203A, U+00BB, plus the new U+2206.
- Fallback but narrower than a cell (no overlap): U+29EB 0.88, U+25AE 0.98, U+25B8 0.90, U+25BE 0.90.
- Fallback and wider than a cell (overflow): U+25D2 1.56 (replaced here), U+25CD 1.56, U+25C9 1.56, U+25F7 1.56, U+2717 1.50, U+2762 1.50, U+21E1 1.50, U+21E3 1.50, U+23CE 1.50, U+22EF 1.79, U+2B16 1.37, U+25BA 1.37, U+25E6 1.05. TYPOGRAPHY arrows U+2190..U+2193 are 1.50.
- The TYPOGRAPHY exemption: U+00B7, U+2013, U+2014, U+2026 and U+2190..U+2193 are all EAW Ambiguous. U+00B7 IS covered by Ubuntu Sans Mono (1.00 cell), so it is not what causes this overlap.
- Few code points are non-Ambiguous AND present in both Ubuntu Sans Mono and DejaVu Sans Mono: the planner counted 26 symbols, mostly currency. U+2206 is also present in Ubuntu Mono, Liberation Mono, Noto Mono, FreeMono and Nimbus Mono PS. U+25D2 is in only 2 of the 7 installed monospace families.

## Decisions (all INFERRED, for audit)

- INFERRED-1: the cause is font fallback, not East Asian Width (measurements above).
- INFERRED-2: unmerged/delivered becomes U+2206 INCREMENT. Reasons: a delta reads as "changes not yet merged"; it is EAW Neutral; it is in every installed monospace family except Courier 10 Pitch; it is in WGL4 and Mac OS Roman. The registry constant is renamed to `INCREMENT`, because registry names follow the Unicode character name. Rejected alternatives: U+25CA and U+25CC are already taken (unknown, blocked); ASCII `~` already means "estimate" in the fixer ladder; ASCII `+` reads as arithmetic.
- INFERRED-3: U+00B7 is replaced in the Status-cell ladders only, by U+2219, which is already a registry glyph. The Status cell is a fixed-width table cell, and an Ambiguous char there draws 2 cells under ambiguous-wide settings and shifts every later column. That is exactly the failure class the registry exists to prevent. Elsewhere U+00B7 stays exempt TYPOGRAPHY (titles, hints, status lines) under the 2026-10-21 todo, because an opt-in ASCII mode is new config and the operator's call. This reverses the earlier inference recorded on `the_middle_dot_is_one_cell` in normal.rs. Consequence, accepted: the Sessions Agents sub-view summary renders the same `summary_forms` and shows U+2219, while the Waves pane title keeps U+00B7.
- INFERRED-4: FLUSH_SAFE means "draws within one cell on the reference setup" (Ubuntu Sans Mono primary, fontconfig fallback to DejaVu Sans). It holds the 9 primary-covered registry glyphs plus the 4 fallback glyphs that are narrower than a cell. It is a committed table with provenance. Reading font files at test time was rejected: CI runners do not carry Ubuntu Sans Mono, and a test that skips without its fonts proves nothing.
- INFERRED-5: the 12 other overflowing registry glyphs and the 4 arrows are not replaced here. Each is drawn before a space or at the end of a cell. A font-safe replacement pool barely exists, so replacing them means ASCII, which is a design decision. They are recorded in a new pending todo for the operator. This is the one item to escalate.
- INFERRED-6: README.md is audited and unchanged, because its Status-cell examples (`w2/11 13run`, `~5/12 fixed`) carry neither glyph. Help text updates itself through the constant. docs/GETTING-STARTED.md carries the old glyph and is updated.
- INFERRED-7: the archived 30-UI-SPEC `## Glyph width safety` table is amended, because the 2026-10-21 todo names it authoritative beside `src/ui/glyphs.rs`.
</context>

<tasks>

<task type="tracer">
  <name>Task 1: End-to-end unmerged glyph swap (U+25D2 to U+2206) from the registry to the dashboard Status cell</name>
  <files>src/ui/glyphs.rs, src/agents/unmerged.rs, src/agents/waves.rs, src/ui/screens/help.rs, src/ui/roadmap_graph.rs, src/ui/roadmap_view.rs, src/app.rs, tests/agents_unmerged.rs, tests/agents_waves.rs, docs/GETTING-STARTED.md</files>
  <read_first>src/ui/glyphs.rs, src/agents/unmerged.rs (lines 25-40), src/agents/waves.rs (lines 547-566, 700-721, 1418-1512), src/app.rs (fn an_unmerged_only_scan_reaches_the_dashboard_status_cell near line 6305), src/ui/screens/help.rs (lines 155-185, 900-925), docs/GETTING-STARTED.md (lines 180-190)</read_first>
  <action>
Re-verify the cause before changing anything (INFERRED-1). Run the python `unicodedata.east_asian_width` check for U+25D2, the two `gsettings` reads, and the `fc-list`/`fc-match` charset probes listed in the context block. Record their outputs verbatim for the SUMMARY's "Cause" section. If any result differs from the plan (for example U+25D2 reads as Ambiguous, or Ubuntu Sans Mono turns out to carry it), stop and report instead of proceeding.

Start red. In `src/app.rs` `an_unmerged_only_scan_reaches_the_dashboard_status_cell`, change the expected Status-cell content from the U+25D2 escape followed by `1` to the U+2206 escape followed by `1`. Run that one test and confirm it FAILS.

Then make the swap, per INFERRED-2:
- `src/ui/glyphs.rs`: replace the registry entry for U+25D2 with `pub const INCREMENT: &str` holding the U+2206 escape. Give it the doc line "Unmerged, delivered. `U+2206` INCREMENT. Was `U+25D2`, before that `U+25D0`". Rename the matching `ALL` row so the name string says `INCREMENT`, and keep `ALL.len()` at 25. Add U+25D2 to the test-only `FORMER` list. In `the_width_predicate_rejects_every_former_glyph`, add a measured deviation for U+25D2 shaped like the existing U+2691 one. Its comment says U+25D2 is EAW Neutral and one cell under both widths, so the width predicate accepts it, and that it is FORMER because Ubuntu Sans Mono lacks it and the DejaVu Sans fallback draws it 1.56 cells wide. In `glyphs_defined_outside_ui_are_registry_glyphs`, assert `GLYPH_UNMERGED == INCREMENT`.
- `src/agents/unmerged.rs`: `GLYPH_UNMERGED` becomes the U+2206 escape. Fix its doc line, which today names the wrong code point, U+25D0.
- `src/agents/waves.rs` production: in `base_summary_forms`, delete the local alias binding of `GLYPH_UNMERGED` and interpolate `{GLYPH_UNMERGED}` directly, so the Task 3 source scan can see those flush draws. Update every doc comment that shows the old glyph (the `with_unmerged` doc, `pending_unmerged`, the `summary_forms` doc, and the comment in `active_forms`) so it shows `∆N`. In the test module, rename the `HALF` const to `DELTA`, give it the U+2206 escape, and update its uses.
- `src/ui/screens/help.rs`: switch the test-module import and the `delivered` legend pair to `INCREMENT`. The production rows already read `GLYPH_UNMERGED`.
- `src/ui/roadmap_graph.rs` (the status-glyph doc near line 411), `src/ui/roadmap_view.rs` (the comment near line 1166) and the `src/app.rs` comments near lines 3064 and 6303: replace the old glyph in the comment text with `∆`.
- `tests/agents_unmerged.rs` (3 pins) and `tests/agents_waves.rs` (the unmerged pins near lines 292 and 303): swap the U+25D2 escape for the U+2206 escape. Leave the U+00B7 separators in those strings alone; Task 2 owns them.
- `docs/GETTING-STARTED.md` line 185: the Waves-pane legend entry for leftover work shows `∆ leftover`.

The compiler enforces the rename: every use of the old constant name must resolve to `INCREMENT`.
  </action>
  <verify>
    <automated>rtk bypass bash -c 'set -u; fail() { echo "FAIL-$1"; exit 1; }; L=target/ixj-t1.log; ran() { cargo test --lib "$1" > "$L" 2>&1; sed -nE "s/^test result: ok\. ([0-9]+) passed; 0 failed.*/\1/p" "$L"; }; n=$(ran an_unmerged_only_scan_reaches_the_dashboard_status_cell); [ "$n" = 1 ] || fail e2e-dashboard; n=$(ran ui::glyphs::); [ -n "$n" ] && [ "$n" -ge 6 ] || fail glyphs; n=$(ran agents::waves::); [ -n "$n" ] && [ "$n" -ge 20 ] || fail waves-lib; n=$(ran ui::roadmap_view::); [ -n "$n" ] && [ "$n" -ge 1 ] || fail roadmap-view; n=$(ran ui::screens::help::); [ -n "$n" ] && [ "$n" -ge 1 ] || fail help; cargo test --test agents_unmerged --test agents_waves > "$L" 2>&1 || fail agents-integration; grep -q "pub const INCREMENT: &str = \"\\\\u{2206}\";" src/ui/glyphs.rs || fail increment-const; grep -q "GLYPH_UNMERGED: &str = \"\\\\u{2206}\";" src/agents/unmerged.rs || fail glyph-unmerged; [ "$(grep -rl "u{25D2}" src tests --include=*.rs | sort | tr "\n" " ")" = "src/ui/glyphs.rs " ] || fail old-escape-survives; [ -z "$(grep -rlF "◒" src tests docs README.md)" ] || fail old-literal-survives; grep -qF "∆ leftover" docs/GETTING-STARTED.md || fail getting-started; echo verify-ok'</automated>
  </verify>
  <done>Task 1 is done when all of the following hold:
- The updated dashboard render test failed first, then passed after the swap.
- The dashboard Status cell for an unmerged-only scan shows U+2206 followed by the count.
- `GLYPH_UNMERGED == INCREMENT` is asserted.
- The U+25D2 escape appears only in `src/ui/glyphs.rs`, and the literal character appears nowhere under src/, tests/, docs/ or README.md.
- The glyphs, waves, roadmap_view and help unit tests and the agents_unmerged and agents_waves integration tests pass.
- The block prints verify-ok, the cause re-verification outputs are captured for the SUMMARY, and the change is committed as `fix(quick-261111-ixj): ...`.</done>
</task>

<task type="auto">
  <name>Task 2: Status-cell separator U+00B7 to U+2219 across every ladder</name>
  <files>src/agents/waves.rs, tests/agents_waves.rs, tests/agents_fixers.rs, src/app.rs, src/ui/screens/normal.rs, src/ui/screens/detail.rs</files>
  <read_first>src/agents/waves.rs (lines 547-566, 658-801, 812-816, 1402-1416), src/ui/screens/normal.rs (lines 505-516, 3795-3846), src/app.rs (fn an_agents_scan_reaches_the_dashboard_status_cell near line 6275), src/ui/screens/detail.rs (the assertions near lines 19291 and 19717)</read_first>
  <action>
Per INFERRED-3, replace the U+00B7 MIDDLE DOT escape with the U+2219 BULLET OPERATOR escape, in the same backslash-u-brace escape form every registry glyph uses, in every production format string in `src/agents/waves.rs` that builds a Status-cell form. That covers `with_unmerged`, the worktree-phase tag in `summary_forms`, both branches of `base_summary_forms`, both match arms of `active_forms`, and `fixer_forms`: 14 lines today. Keep the surrounding single spaces exactly, so every form keeps its current cell width and `every_summary_form_is_narrower_than_the_one_before` still holds. Do not touch U+00B7 anywhere outside the Status-cell ladders; detail.rs pane titles, hints and status lines keep it.

Update the doc comments in waves.rs that describe the separator. The `summary_forms` doc currently says the separator is U+00B7 and one cell wide. It must now say U+2219 BULLET OPERATOR, EAW Neutral, one cell wide. In the waves.rs test module, rename the `DOT` const to `SEP`, give it the U+2219 escape, and update its uses. The width assertion at the end of `every_summary_form_is_narrower_than_the_one_before` now asserts that `SEP` is one cell.

Update every pin of a Status-cell form that carries the old separator: `tests/agents_waves.rs` (2 strings), `tests/agents_fixers.rs` (4 strings), `src/app.rs` `an_agents_scan_reaches_the_dashboard_status_cell` (2 strings), `src/ui/screens/normal.rs` `the_fixer_estimate_is_a_whole_ladder_form_at_80_120_and_200` (1 string), and `src/ui/screens/detail.rs` `the_agents_sub_view_draws_the_summary_waves_and_agent_rows` and `the_three_observed_shapes_render_at_80_by_24` (1 string each). In `src/ui/screens/normal.rs`, update the `Line::width` doc near line 512 to name the U+2219 separator ("three bytes and one cell"). Rename `the_middle_dot_is_one_cell` to `the_status_cell_separator_is_one_cell`: it asserts U+2219 is one cell under `Line::width` and three bytes long, and its doc comment records that the earlier keep-U+00B7 inference was reversed by quick 261111-ixj because the char is EAW Ambiguous.

If any other Status-cell pin turns up red, update it the same way and list it in the SUMMARY.
  </action>
  <verify>
    <automated>rtk bypass bash -c 'set -u; fail() { echo "FAIL-$1"; exit 1; }; L=target/ixj-t2.log; ran() { cargo test --lib "$1" > "$L" 2>&1; sed -nE "s/^test result: ok\. ([0-9]+) passed; 0 failed.*/\1/p" "$L"; }; [ "$(awk "/^mod tests \\{/{exit} {print}" src/agents/waves.rs | grep -v "^[[:space:]]*//" | grep -cE "u\\{b7\\}|·")" -eq 0 ] || fail waves-prod-still-has-old-separator; [ "$(awk "/^mod tests \\{/{exit} {print}" src/agents/waves.rs | grep -v "^[[:space:]]*//" | grep -c "u{2219}")" -ge 14 ] || fail waves-new-separator-count; n=$(ran agents::waves::); [ -n "$n" ] && [ "$n" -ge 20 ] || fail waves-lib; n=$(ran an_agents_scan_reaches_the_dashboard_status_cell); [ "$n" = 1 ] || fail app-agents-cell; n=$(ran the_fixer_estimate_is_a_whole_ladder_form_at_80_120_and_200); [ "$n" = 1 ] || fail normal-fixer; n=$(ran the_status_cell_separator_is_one_cell); [ "$n" = 1 ] || fail normal-separator; n=$(ran the_agents_sub_view_draws_the_summary_waves_and_agent_rows); [ "$n" = 1 ] || fail detail-agents; n=$(ran the_three_observed_shapes_render_at_80_by_24); [ "$n" = 1 ] || fail detail-shapes; cargo test --test agents_waves --test agents_fixers --test agents_unmerged > "$L" 2>&1 || fail agents-integration; echo verify-ok'</automated>
  </verify>
  <done>Task 2 is done when all of the following hold:
- The production region of `src/agents/waves.rs` (before its column-zero `mod tests {`) draws no U+00B7 and at least 14 U+2219 separators.
- Every Status-cell pin in the 6 files expects U+2219.
- `the_status_cell_separator_is_one_cell` replaces the old middle-dot test and records the reversal.
- The waves unit tests, the 5 named render/app tests and the 3 agents integration suites pass.
- The block prints verify-ok, and the change is committed as `fix(quick-261111-ixj): ...`.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 3: Flush-safety guards, cause and limitation records, full gates</name>
  <files>src/ui/glyphs.rs, src/agents/waves.rs, .planning/milestones/v2.0-phases/30-driver-tab-on-direction-md/30-UI-SPEC.md, .planning/todos/pending/2026-11-11-registry-glyphs-overflow-under-font-fallback.md</files>
  <read_first>src/ui/glyphs.rs (the whole test module, especially drawn_codepoints, collect_rs and production_ui_code_draws_only_registry_or_exempt_glyphs), src/agents/waves.rs (test helpers row, agents, derive callers, plan_item, unmerged_item and the views built in every_summary_form_is_narrower_than_the_one_before and the unmerged tests), .planning/milestones/v2.0-phases/30-driver-tab-on-direction-md/30-UI-SPEC.md (section `## Glyph width safety`, from line 882), tests/spawn_seam_guard.rs (lines 380-420: why the column-zero `mod tests {` marker must stay)</read_first>
  <behavior>
    - flush_safe_is_a_measured_subset_of_the_registry: every FLUSH_SAFE entry is a member of ALL, there are no duplicates, there are exactly 13 entries, INCREMENT is in it, and none of the 13 measured overflow glyphs (U+25CD, U+25C9, U+25F7, U+2717, U+2762, U+21E1, U+21E3, U+23CE, U+22EF, U+2B16, U+25BA, U+25E6, and the former U+25D2) is in it.
    - the_flush_scanner_flags_a_glyph_touching_text (negative control): on synthetic source lines, the scanner reports a U+2717 escape followed by a digit, a `{BALLOT_X}` interpolation followed by a digit, and a `{GLYPH_UNMERGED}` interpolation followed by `{n}`. It reports nothing for the same glyph followed by a space, a comment line, a trailing comment, a glyph that ends a string literal, or a line inside a `#[cfg(test)]` test module.
    - registry_glyphs_drawn_flush_against_text_are_flush_safe: across the same file set as the existing production scan, every flush draw is FLUSH_SAFE. At least 3 flush draws are seen, so the scan cannot pass vacuously: the waves.rs and roadmap_view.rs `{GLYPH_UNMERGED}` sites and the detail.rs U+25B8 and U+2713 sites guarantee this.
    - the_status_cell_source_draws_no_typography: the production region of src/agents/waves.rs draws no TYPOGRAPHY or STRUCTURAL code point, only registry glyphs, and at least one registry glyph is seen there.
    - every_status_cell_form_is_width_safe_and_flush_safe (in src/agents/waves.rs tests): for a matrix of views covering every ladder branch, every char of every summary form is ASCII or a member of glyphs::ALL; no form contains U+00B7; every non-ASCII char followed by a non-space char is in FLUSH_SAFE; at least one form contains INCREMENT followed by a digit; and at least one form contains the U+2219 separator between spaces. The matrix covers active executor with waves plus unmerged, active without waves, the 999 extreme view, fixer with counts plus unmerged, `N agents`, idle unmerged only, idle unmerged plus stalled, stalled only, and a worktree-phase tag.
  </behavior>
  <action>
Per INFERRED-4, add a production `pub const FLUSH_SAFE: &[&str]` to `src/ui/glyphs.rs`, directly after `ALL`, listing registry constants BY NAME, with no new escapes. Group 1 is covered by both Ubuntu Sans Mono and DejaVu Sans Mono: LOZENGE_WHITE, DOUBLE_BAR, CIRCLE_DOTTED, BULLET_OPERATOR, CHECK, ANGLE_LEFT, ANGLE_RIGHT, GUILLEMET_RIGHT, INCREMENT. Group 2 is missing from Ubuntu Sans Mono, but its DejaVu Sans fallback is narrower than a cell: LOZENGE_BLACK at 0.88, BLACK_RECT at 0.98, TRIANGLE_RIGHT_SMALL at 0.90, TRIANGLE_DOWN_SMALL at 0.90. Its doc comment gives the rule ("a registry glyph drawn directly before a non-space character must be in this list"), the reason (a fallback glyph wider than a cell covers the next character; measured: U+25D2 at 1.56 cells covered the count in the Status cell), and the provenance: the reference setup is Ubuntu Sans Mono 11 with fontconfig fallback, it was measured 2026-11-11 with `fc-list ':file=<font>:charset=<hex>'` and hmtx advances, and it is to be re-measured when a glyph is added. Also amend the module doc. The rule section gains a fifth bullet that points to FLUSH_SAFE. The fourth bullet's DejaVu Sans Mono coverage claim gets the qualifier that coverage in one font does not prevent fallback in another. The Exemptions section gains a known-limitations bullet that points to the new todo.

Write the four glyphs.rs tests from the behavior block inside the existing test module. Do NOT change its column-zero `#[cfg(test)]` / `mod tests {` declaration; the spawn-seam guard and this file's own cut logic both read it. Extract the existing file-list construction from `production_ui_code_draws_only_registry_or_exempt_glyphs` (the src/ui walk, the 3 extra files, the render_escape_guard exclusion and its proof) into a shared helper, and have both scans call it. The flush scanner takes the same production-region, comment and trailing-comment rules as `drawn_codepoints`. It returns the glyph and line of every occurrence of (a) a `\u{HEX}` escape that resolves to a registry glyph, (b) a literal registry char, or (c) a `{NAME}` interpolation where NAME is an `ALL` name or `GLYPH_UNMERGED`, whenever the next char on that line is ASCII alphanumeric or an opening brace. Hex is case-insensitive, because detail.rs writes `25b8` in lower case. Build the negative-control synthetic source line by line, as the existing scanner test does, so no synthetic line sits at column zero as a test-module marker.

Write `every_status_cell_form_is_width_safe_and_flush_safe` in the `src/agents/waves.rs` test module, reusing that module's existing helpers and view literals. It reads `crate::ui::glyphs::ALL`, `FLUSH_SAFE` and `INCREMENT` (all `pub`).

Per INFERRED-7, amend the archived 30-UI-SPEC `## Glyph width safety` section. The registry table row for the former half-circle entry becomes `INCREMENT`, `∆` U+2206, previously U+25D2 and before that U+25D0, unmerged / delivered. Add a short dated note under the rule paragraph: "Amended by quick 261111-ixj: coverage in DejaVu Sans Mono did not prevent fallback in Ubuntu Sans Mono; glyphs drawn flush against text must be in `glyphs::FLUSH_SAFE`."

Per INFERRED-5, create `.planning/todos/pending/2026-11-11-registry-glyphs-overflow-under-font-fallback.md` with the same frontmatter shape as the 2026-10-21 todo (created, title, area: ui, severity: minor, files). The body lists the measured overflow inventory from the context block: 12 registry glyphs and the 4 TYPOGRAPHY arrows with their cell ratios. It notes that each is drawn before a space or at the end of a cell today, so the overflow lands on blank space. It names the one known visible case outside the indicator set: the confirm-popup key hint puts the U+2190 arrow flush against a slash. It explains why nothing was replaced (the non-Ambiguous pool common to both fonts is about 26 symbols, mostly currency, so replacing them means ASCII, which is the operator's design call), and it proposes options (ASCII badge/state set, or an opt-in ASCII mode merged with the 2026-10-21 todo).

Order at the end of this task: commit the code, todo and UI-SPEC changes, then run the verify block. Its clean-tree check runs before the SUMMARY exists. After verify-ok, write the SUMMARY; the quick workflow commits it together with STATE.md. The SUMMARY has these sections: Cause (the Task 1 re-verification outputs plus the measurements), Glyph inventory, Decisions (INFERRED-1..7, verbatim IDs), Escalation (INFERRED-5), and the final test count from the verify block.
  </action>
  <verify>
    <automated>rtk bypass bash -c 'set -u; fail() { echo "FAIL-$1"; exit 1; }; L=target/ixj-t3.log; ran() { cargo test --lib "$1" > "$L" 2>&1; sed -nE "s/^test result: ok\. ([0-9]+) passed; 0 failed.*/\1/p" "$L"; }; for t in flush_safe_is_a_measured_subset_of_the_registry the_flush_scanner_flags_a_glyph_touching_text registry_glyphs_drawn_flush_against_text_are_flush_safe the_status_cell_source_draws_no_typography every_status_cell_form_is_width_safe_and_flush_safe; do n=$(ran "$t"); [ "$n" = 1 ] || fail "test-$t"; done; n=$(ran ui::glyphs::); [ -n "$n" ] && [ "$n" -ge 10 ] || fail glyphs-suite; grep -q "^pub const FLUSH_SAFE: &\[&str\]" src/ui/glyphs.rs || fail flush-safe-const; grep -q "^mod tests {" src/ui/glyphs.rs || fail test-marker-moved; grep -q "INCREMENT" .planning/milestones/v2.0-phases/30-driver-tab-on-direction-md/30-UI-SPEC.md || fail ui-spec; grep -q "FLUSH_SAFE" .planning/milestones/v2.0-phases/30-driver-tab-on-direction-md/30-UI-SPEC.md || fail ui-spec-note; T=.planning/todos/pending/2026-11-11-registry-glyphs-overflow-under-font-fallback.md; [ -f "$T" ] || fail todo; for cp in 25CD 2717 22EF 2190; do grep -q "$cp" "$T" || fail "todo-$cp"; done; cargo fmt --all -- --check > "$L" 2>&1 || fail fmt; cargo clippy --all-targets -- -D warnings > "$L" 2>&1 || fail clippy-linux; cargo clippy --locked --target x86_64-pc-windows-gnu --all-targets -- -D warnings > "$L" 2>&1 || fail clippy-windows; TS=$(grep -E "^ +- target: " .github/workflows/release.yml | awk "{print \$3}"); [ -n "$TS" ] || fail no-matrix-targets; for t in $TS; do cargo check --locked --target "$t" --all-targets > "$L" 2>&1 || fail "check-$t"; cargo clippy --locked --target "$t" --all-targets -- -D warnings > "$L" 2>&1 || fail "clippy-$t"; done; F=target/ixj-full-tests.log; cargo test --no-fail-fast > "$F" 2>&1; rc=$?; P=$(awk "/^test result:/ {p+=\$4} END {print p+0}" "$F"); X=$(awk "/^test result:/ {f+=\$6} END {print f+0}" "$F"); echo "full-suite exit=$rc passed=$P failed=$X"; [ "$rc" -eq 0 ] || fail full-suite-exit; [ "$X" -eq 0 ] || fail full-suite-failed; [ "$P" -ge 3822 ] || fail full-suite-count; [ -z "$(git status --porcelain)" ] || fail dirty-tree; echo verify-ok'</automated>
  </verify>
  <done>Task 3 is done when all of the following hold:
- FLUSH_SAFE exists with 13 named registry glyphs and its measured provenance.
- The 4 glyphs.rs guard tests and the waves.rs Status-cell form test each run and pass, and the glyphs suite has at least 10 tests.
- The negative control proves the flush scanner flags a synthetic glyph touching text.
- The column-zero `mod tests {` marker in glyphs.rs is untouched.
- The archived 30-UI-SPEC row and note are amended, and the new pending todo lists the measured overflow inventory.
- fmt is clean, Linux and Windows clippy are clean, and check plus clippy pass for every release.yml matrix target.
- `cargo test --no-fail-fast` exits 0 with at least 3822 passed and 0 failed, read from target/ixj-full-tests.log.
- The SUMMARY records the cause, inventory, decisions and escalation; everything is committed; and `git status` is clean.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| app -> terminal | Every glyph the app emits is drawn by the user's terminal and font stack, which the app does not control. |
| `.planning/` files -> render | Untrusted file content reaches the screen. This plan adds no new path, and glyphs stay compile-time constants. |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-261111-ixj-01 | Tampering (display integrity) | overview Status cell / any registry glyph drawn flush against text | low | mitigate | U+2206 replaces the fallback glyph that covered the count. `FLUSH_SAFE`, the flush source scan and `every_status_cell_form_is_width_safe_and_flush_safe` fail on any spilling glyph drawn flush against text, and the negative control proves the scanner is not vacuous. |
| T-261111-ixj-02 | Tampering | glyph source (terminal escape via derived glyphs) | low | mitigate | Both replacements are `\u{..}` registry constants (D-13, D-24, T-18-32). No file content becomes a glyph. The existing production source scan and render_escape_guard are unchanged and still run. |
| T-261111-ixj-SC | Tampering | npm/pip/cargo installs | low | accept | No dependency is added. icu_properties and unicode-width are already in Cargo.lock, and every cross-target gate runs `--locked`. |
</threat_model>

<verification>
- Task 1 starts red: the dashboard render test expecting U+2206 fails before the swap and passes after.
- No U+25D2 escape outside the glyphs.rs FORMER list. No literal old glyph under src/, tests/, docs/ or README.md.
- No U+00B7 in the production region of src/agents/waves.rs.
- Five new guard tests run and pass. The negative control shows the flush scanner flags a synthetic flush draw.
- Full suite: `cargo test --no-fail-fast` exits 0 with at least 3822 passed and 0 failed, read from a log file inside `rtk bypass` and never piped through grep.
- `cargo clippy --all-targets -- -D warnings` exits 0. Check and clippy are clean for every release.yml matrix target (including x86_64-pc-windows-gnu), and fmt is clean.
</verification>

<success_criteria>
- The user's two example cells render as `∆1 ∙ 1 stalled` and `1run ∙ ∆30`. Every char in them is EAW Neutral or Narrow, and every non-ASCII char is present in Ubuntu Sans Mono, so nothing overlaps on the user's Ptyxis/herdr setup.
- The disproved Ambiguous hypothesis and the measured font-fallback cause are recorded in the SUMMARY.
- Bringing back a spilling glyph flush against text, or an Ambiguous separator in the Status cell, makes a named test fail.
- The remaining fallback-overflow glyphs are recorded in a pending todo for an operator decision (INFERRED-5), with nothing silently dropped.
</success_criteria>

<output>
Create `.planning/quick/261111-ixj-replace-east-asian-ambiguous-indicator-g/261111-ixj-SUMMARY.md` when done
</output>
