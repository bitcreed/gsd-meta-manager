---
phase: quick-261021-1id
plan: 01
subsystem: ui
tags: [tui, glyphs, unicode-width, east-asian-width, dim]
requires: []
provides:
  - src/ui/glyphs.rs width-safe glyph registry with tests and a production source scan
  - Driver tab Label/Muted roles as Modifier::DIM
affects: [dashboard badges, Driver tab, roadmap, Waves pane, stage ladder, git lanes]
tech-stack:
  added: [unicode-width 0.2 (dev-dependency, already locked via ratatui)]
  patterns: [single glyph registry aliased by screen constants, source-scan guard]
key-files:
  created:
    - src/ui/glyphs.rs
    - .planning/todos/pending/2026-10-21-ambiguous-wide-typography-borders-and-darkgray.md
  modified:
    - Cargo.toml
    - Cargo.lock
    - src/ui/mod.rs
    - src/ui/screens/normal.rs
    - src/ui/screens/driver_run.rs
    - src/ui/screens/driver.rs
    - src/ui/screens/detail.rs
    - src/ui/roadmap_graph.rs
    - src/ui/roadmap_widget.rs
    - src/agents/unmerged.rs
    - src/state_reader/git_ops.rs
    - .planning/phases/30-driver-tab-on-direction-md/30-UI-SPEC.md
    - .planning/phases/30-driver-tab-on-direction-md/30-UAT.md
    - docs/GETTING-STARTED.md
decisions:
  - "Driver tab Label and Muted roles are DIM on the default fg; inert-state colour stays DarkGray [inferred — audit]"
  - "Glyph picks per the plan glyph_map, U+2762 for parked [inferred — audit]"
  - "Compact Waves strip queued suffix (w12 followed by the mark) follows the queued glyph U+2219 [inferred — audit]"
metrics:
  tasks: 3
  commits: 6
  completed: 2026-10-21
status: complete
plan_head_before: fe23d99c809c70519bba276e63b4c1ab861817e7
plan_head_after: 7bd6e15c523f5a7d465ee8570bdf9459fec1640e
commits: 6
actuals:
  tasks: 3
  commits: 6
---

# Phase quick-261021-1id Plan 01: Width-safe TUI status glyphs Summary

Every status glyph the TUI draws now comes from, or is pinned against, `src/ui/glyphs.rs`: EAW Neutral/Narrow/Halfwidth, never emoji-capable, 1 cell under both `unicode-width` widths, with a production source scan to keep strays out. The Driver tab's Label and Muted roles moved from DarkGray to DIM on the default foreground.

## Commits

| Step | Sha |
|---|---|
| Task 1 RED (registry + width test) | 8421547 |
| Task 1 GREEN (badges, Driver tab runs/items) | 58ff7c0 |
| Task 2 RED (source scan) | 8ec6976 |
| Task 2 GREEN (remaining glyphs through the registry) | d7049ae |
| Task 3 DIM labels | 07ae883 |
| Task 3 docs (UI-SPEC, UAT, Waves legend, todo) | 7bd6e15 |

## Verification

- `cargo test --no-fail-fast`: 4089 passed / 1 failed / 24 ignored. 4082 baseline plus 7 net new `#[test]` fns. The only failure is the expected `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`.
- `cargo clippy --all-targets -- -D warnings`: exit 0. `cargo fmt --check`: clean.
- `git diff --stat fe23d99..HEAD -- src/driverctl src/launcher`: empty.
- Cargo.lock gained only the root package's `unicode-width 0.2.0` edge, no new package stanza.

## Glyph swaps as shipped (old to new)

| Meaning | Old | New |
|---|---|---|
| driven / live / running | U+25C6 | U+29EB BLACK LOZENGE |
| stale | U+25C8 | U+2B16 DIAMOND WITH LEFT HALF BLACK |
| liveness unknown | U+25C7 | U+25CA LOZENGE |
| parked / needs human | U+2691 | U+2762 HEAVY EXCLAMATION MARK ORNAMENT |
| paused | U+2016 | U+01C1 LATIN LETTER LATERAL CLICK |
| stopped / killed | U+25A0 | U+25AE BLACK VERTICAL RECTANGLE |
| done / commit / selected option | U+25CF | U+25CD CIRCLE WITH VERTICAL FILL |
| unlaunched / pending / queued / ready / planned / merge | U+25CB | U+25E6 WHITE BULLET |
| unmerged / delivered | U+25D0 | U+25D2 CIRCLE WITH LOWER HALF BLACK |
| session / selected / Waves running | U+25B6 | U+25BA BLACK RIGHT-POINTING POINTER |
| roadmap widget open | U+25BC | U+25BE |
| roadmap dependency / unblocks | U+2191 / U+2193 | U+21E1 / U+21E3 |
| unseen / implied / stderr / Waves queued | U+00B7 | U+2219 BULLET OPERATOR |
| stage in progress | U+2026 | U+22EF |
| stage skipped / not started | U+2013 | ASCII `-` |

Exempt, audited and kept: STRUCTURAL U+2500..=U+259F (borders, rules, graph lanes, the caps bar fill; ratatui's own `Block`/`Gauge` use the range and no Narrow/Neutral equivalent exists), and TYPOGRAPHY U+00B7, U+2013, U+2014, U+2026, U+2190..U+2193 inside composed text (an ASCII fallback mode would be new config).

## DarkGray decision [inferred — audit]

`label_style()` and `muted_style()` are `Modifier::DIM` on the terminal's default foreground with no fg colour. DIM adapts to light and dark themes; bright-black matches the background in Solarized-Dark-family palettes. The inert-state colour (unlaunched, stopped, pending, unseen, `[not live]`, killed) is unchanged DarkGray; the Driver popups' `muted()` and other tabs are untouched and recorded in the follow-up todo.

## Deviations from Plan

**1. [Rule 1 - Bug] The U+2691 control premise does not hold against icu_properties 2.3.0**
- **Found during:** Task 1 RED
- **Issue:** The plan says the width predicate rejects U+2691 for Extended_Pictographic alone. `icu_properties` 2.3.0 reports ExtendedPictographic and Emoji false for U+2691 (the Unicode 13 emoji-data.txt in the cargo registry lists it as Extended_Pictographic), and both unicode-width widths are 1, so the predicate accepts it. The control `the_width_predicate_rejects_every_former_glyph` failed.
- **Fix:** U+2691 is skipped in the rejection loop with a comment; it stays in FORMER (never allowed in ALL) and is replaced regardless. A new test, `the_property_check_carries_weight_the_widths_cannot`, witnesses the property check with U+23F8 (width 1 under both measures, rejected only by Emoji/Extended_Pictographic). Net new tests are 7, not 6.
- **Files modified:** src/ui/glyphs.rs. **Commits:** 8421547, 58ff7c0. Recorded in 30-UI-SPEC.md `## Glyph width safety`.

**2. [Rule 1 - Bug] Scanner control tripped spawn_seam_guard**
- **Found during:** Task 2 GREEN full run
- **Issue:** the synthetic source in `the_scanner_flags_a_stray_glyph_and_ignores_tests_and_comments` had column-zero `mod tests {` lines, which `tests/spawn_seam_guard.rs::no_production_item_follows_a_test_module_marker` reads as a production item after a test marker.
- **Fix:** the synthetic source is built line by line from an indented array. **Commit:** d7049ae.

**3. [Rule 2 - consistency] Compact Waves strip queued suffix**
- The plan mapped Waves queued to U+2219 but listed only `PaneState::Queued`. The compact strip's `w12` + queued mark (detail.rs wave strip) also used U+00B7 as a queued status mark, so it follows the same glyph; its tests were updated. [inferred — audit]

**4. [Rule 3 - housekeeping]** `cargo fmt` applied to files touched by Task 1 and 2 (app.rs, normal.rs, glyphs.rs, detail.rs, roadmap_widget.rs); folded into the Task 2 GREEN commit.

## Other [inferred — audit] choices

- All glyph picks per the plan `glyph_map`, including U+2762 for parked and U+25AE for stopped.
- Scan scope: `src/ui/**` (minus the whole-file test module `render_escape_guard.rs`, whose `#[cfg(test)]` declaration the scan asserts), `src/agents/{unmerged,waves}.rs`, `src/state_reader/git_ops.rs`.
- 30-UAT.md item 1 pass (operator, 2026-10-21, relayed) cites 58ff7c0, d7049ae, 07ae883 (RED shas 8421547, 8ec6976); item 2 stays pending with the note that it also covers legibility of the replacement glyphs.

## Known Stubs

None.

## Threat Flags

None. All glyphs remain compile-time `&'static str` constants; `unicode-width` is a dev-only edge already locked via ratatui.

## Follow-up

`.planning/todos/pending/2026-10-21-ambiguous-wide-typography-borders-and-darkgray.md`

## Self-Check: PASSED

Commits 8421547, 58ff7c0, 8ec6976, d7049ae, 07ae883, 7bd6e15 are in `git log`; `src/ui/glyphs.rs`, the todo, and this summary exist.
