---
phase: quick-261111-ixj
plan: 01
subsystem: ui
tags: [glyphs, font-fallback, status-cell, regression-guard]
requires: []
provides:
  - "glyphs::INCREMENT (U+2206) replaces CIRCLE_HALF (U+25D2) as the unmerged/delivered glyph"
  - "glyphs::FLUSH_SAFE and a flush-draw source scan"
  - "U+2219 as the Status-cell ladder separator"
affects: [src/ui/glyphs.rs, src/agents/waves.rs, src/agents/unmerged.rs]
key-files:
  created:
    - .planning/todos/pending/2026-11-11-registry-glyphs-overflow-under-font-fallback.md
  modified:
    - src/ui/glyphs.rs
    - src/agents/unmerged.rs
    - src/agents/waves.rs
    - src/ui/screens/help.rs
    - src/ui/screens/normal.rs
    - src/ui/screens/detail.rs
    - src/ui/roadmap_graph.rs
    - src/ui/roadmap_view.rs
    - src/app.rs
    - tests/agents_unmerged.rs
    - tests/agents_waves.rs
    - tests/agents_fixers.rs
    - docs/GETTING-STARTED.md
    - .planning/milestones/v2.0-phases/30-driver-tab-on-direction-md/30-UI-SPEC.md
decisions:
  - "INFERRED-1..7 from the plan, all kept (see Decisions)"
metrics:
  tasks: 3
  tests_passed: 3827
  tests_failed: 0
status: complete
commits: 3
plan_head_before: b39ad129c7818111a564d86fba8f87f26cd92d28
plan_head_after: ac9214e3b3afbef3ab8852c5a6bf6a94f1e188ad
actuals:
  tasks: 3
  commits: 3
---

# Quick 261111-ixj: Replace the overlapping unmerged glyph in the Status cell

The overview Status cell no longer draws its unmerged glyph over the digit after it: U+25D2 became U+2206 INCREMENT (`∆1 ∙ 1 stalled`, `1run ∙ ∆30`), the Status-cell separator U+00B7 became U+2219, and a measured `FLUSH_SAFE` list plus a flush source scan stop a spilling glyph from being drawn against text again.

## Cause

The hypothesis (U+25D2 is East Asian Width Ambiguous) is false. Re-verified in Task 1, outputs verbatim:

```
python3 -I: unicodedata 16.0.0, EAW(U+25D2)=N, EAW(U+2206)=N, EAW(U+2219)=N, EAW(U+00B7)=A
gsettings get org.gnome.Ptyxis use-system-font          -> true
gsettings get org.gnome.desktop.interface monospace-font-name -> 'Ubuntu Sans Mono 11'
fc-list ':file=/usr/share/fonts/truetype/ubuntu/UbuntuSansMono[wght].ttf:charset=25d2' family  -> (nothing)
fc-match -f '%{family[0]}' 'Ubuntu Sans Mono:charset=25d2'     -> DejaVu Sans
fc-match -f '%{family[0]}' 'Ubuntu Sans Mono:charset=2206'     -> Ubuntu Sans Mono
```

U+2206 coverage, re-checked after the fact: `fc-list 'Ubuntu Sans Mono:charset=2206' file family` lists `UbuntuSansMono[wght].ttf`, and the hmtx reader (`adv.py`) reports `U+2206 primary adv_em=0.56 cells=1.0`, `U+2219 primary cells=1.0`, `U+25D2 fallback adv_em=0.873 cells=1.56`. (The file-path form of the `fc-list` query printed nothing for 2206 because the `[wght]` brackets in the file name need escaping; the family form is reliable.)

Mechanism: Ubuntu Sans Mono (the Ptyxis system font) has no U+25D2, so fontconfig substitutes the proportional DejaVu Sans, whose glyph is 0.873 em against a 0.56 em cell (1.56 cells). VTE draws it at natural width, over the next cell, which is the digit in `∆30`. The terminal and ratatui agree on width (1 cell); the glyph itself is just wider than its cell.

## Glyph replacements

| Old | New | Where | Meaning |
|---|---|---|---|
| U+25D2 `CIRCLE_HALF` (now FORMER) | U+2206 `INCREMENT` (`GLYPH_UNMERGED` = same) | dashboard Status cell, Sessions Agents summary, Phases graph and Waves pane, help legend, Driver `delivered` row | unmerged / delivered work |
| U+00B7 (EAW Ambiguous) | U+2219 BULLET OPERATOR (EAW Neutral, already a registry glyph) | all 14 format strings of the Status-cell ladders in `src/agents/waves.rs` | segment separator |

U+00B7 stays as exempt TYPOGRAPHY outside the Status cell (titles, hints, status lines, Waves pane title).

## Guards added

- `glyphs::FLUSH_SAFE` (13 named registry glyphs, with measurement provenance in its doc comment).
- `flush_safe_is_a_measured_subset_of_the_registry`, `the_flush_scanner_flags_a_glyph_touching_text` (negative control), `registry_glyphs_drawn_flush_against_text_are_flush_safe` (at least 3 flush draws asserted), `the_status_cell_source_draws_no_typography`, all in `src/ui/glyphs.rs`.
- `every_status_cell_form_is_width_safe_and_flush_safe` in `src/agents/waves.rs`.
- Mutation check done during execution: removing `INCREMENT` from `FLUSH_SAFE` made the flush scan fail on 8 real draw sites (waves.rs, roadmap_view.rs, help.rs), so the scan is not vacuous. Reverted.
- Existing test renamed: `the_middle_dot_is_one_cell` -> `the_status_cell_separator_is_one_cell`.

## Deviations from Plan

**1. [Rule 1 - Bug] Extra Status-cell pin in detail.rs.** `agents_strip_is_one_line_and_marks_the_current_wave_in_text` (`detail.rs` ~line 30058) searched for `P13 \u{b7} w2/2`; the plan listed only two detail.rs pins. Updated to U+2219. Found when the first full-suite run failed on it.

**2. [Rule 1 - Bug] Clippy `chars_next_cmp` in the new flush scanner.** `g.chars().next() == Some(c)` replaced by `g.starts_with(c)`. Amended into the Task 3 commit before the gate ran green.

**3. Un-commissioned minor choices:** the todo names `src/ui/screens/confirm_popup.rs` (verified: line 288 holds `\u{2190}/\u{2192}` flush against `/`). Also `production_lines` was extracted from `drawn_codepoints` so the flush scanner shares the production-region and comment rules (the plan asked for that behavior; the helper is the means).

## Issues / flake observed

One full-suite run reported `ui::screens::driver_run::tests::a_held_lock_with_an_old_heartbeat_reads_stale_and_a_free_lock_reads_crashed` failing (3826 passed, 1 failed). It passed 3 of 3 times in isolation and the immediate full re-run passed completely. It exercises a `flock` that drops and is re-read, and renders `⬖ stale` where `✗ crashed` was expected after `drop(lock)`: the signature of a lock fd inherited by a concurrently spawned child in a parallel test. It reads no glyph I changed and is not caused by this work. Not fixed (out of scope per the scope boundary); recorded here for an operator decision.

## Gates

- `rtk bypass cargo test --no-fail-fast` (log `target/ixj-full-tests.log`, read by awk, not grepped): exit 0, **3827 passed, 0 failed** (floor was 3822).
- `cargo clippy --all-targets -- -D warnings`: exit 0.
- `cargo clippy --locked --target x86_64-pc-windows-gnu --all-targets -- -D warnings`: exit 0.
- `release.yml` matrix (`aarch64-apple-darwin`, `x86_64-apple-darwin`, `x86_64-pc-windows-gnu`): `cargo check --locked --target T --all-targets` and `cargo clippy --locked --target T --all-targets -- -D warnings` all exit 0.
- `cargo fmt --all -- --check`: clean.
- `git status`: clean apart from the untracked quick-task docs directory (docs commit belongs to the orchestrator).
- Source checks: the only `\u{25D2}` in `src`/`tests` is in `src/ui/glyphs.rs` (FORMER list and its test deviation); the literal `◒` appears nowhere under src, tests, docs or README; the production region of `src/agents/waves.rs` draws no U+00B7 and 14 U+2219 lines.

## Decisions (INFERRED, for audit)

- INFERRED-1: the cause is font fallback, not East Asian Width (measurements above). Confirmed by re-verification.
- INFERRED-2: unmerged/delivered becomes U+2206 INCREMENT; registry constant renamed `INCREMENT`.
- INFERRED-3: U+00B7 replaced by U+2219 in the Status-cell ladders only; U+00B7 elsewhere remains TYPOGRAPHY. This reverses the earlier inference on `the_middle_dot_is_one_cell`. Consequence accepted: the Sessions Agents sub-view summary shows U+2219 while the Waves pane title keeps U+00B7.
- INFERRED-4: `FLUSH_SAFE` is a committed table with provenance (13 glyphs: 9 primary-covered plus 4 whose DejaVu Sans fallback is narrower than a cell), not a font read at test time.
- INFERRED-5: the 12 other overflowing registry glyphs and the 4 TYPOGRAPHY arrows are not replaced; recorded in the pending todo.
- INFERRED-6: README.md audited and unchanged; `docs/GETTING-STARTED.md` updated (`∆ leftover`).
- INFERRED-7: the archived 30-UI-SPEC `## Glyph width safety` table row and a dated amendment note were updated.

## Escalation (INFERRED-5)

Twelve registry glyphs (`CIRCLE_FILLED`, `FISHEYE`, `CLOCK`, `BALLOT_X`, `ATTENTION`, `ARROW_UP_DASHED`, `ARROW_DOWN_DASHED`, `RETURN_SYMBOL`, `MIDLINE_ELLIPSIS`, `DIAMOND_HALF_BLACK`, `POINTER_RIGHT`, `CIRCLE_OPEN`) and the four TYPOGRAPHY arrows still fall back to a proportional font on the user's setup and overflow their cell by 5 to 79 percent. Today each is drawn before a space or at the end of a cell, so nothing is covered. Replacing them means ASCII (a design call) or an opt-in ASCII mode. See `.planning/todos/pending/2026-11-11-registry-glyphs-overflow-under-font-fallback.md` for the inventory with ratios and options. This is the one item that needs the operator.

## Known Stubs

None.

## Threat Flags

None. No new endpoint, auth path or file access; glyphs remain compile-time `\u{..}` constants (T-261111-ixj-02).

## Commits

- 288a32d `fix(quick-261111-ixj): replace the unmerged glyph U+25D2 with U+2206 INCREMENT`
- 73e32be `fix(quick-261111-ixj): use U+2219 as the Status-cell separator instead of U+00B7`
- ac9214e `test(quick-261111-ixj): guard glyphs drawn flush against text and the Status cell forms`

## Self-Check: PASSED

All three commits are ancestors of HEAD, the pending todo exists, and the working tree is clean apart from the untracked quick-task directory.
