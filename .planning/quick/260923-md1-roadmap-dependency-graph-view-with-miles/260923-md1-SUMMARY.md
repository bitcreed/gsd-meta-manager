---
phase: quick-260923-md1
plan: 01
quick_id: 260923-md1
status: complete
subsystem: ui/roadmap
tags: [roadmap, dependency-graph, milestones, tui, escaping]
requires: [state_reader::roadmap_md, state_reader::phase_num, text::render_for_terminal]
provides:
  - ui::roadmap_graph (pure layout + RoadmapGraphWidget)
  - roadmap_md::{RoadmapMilestone, roadmap_milestones, milestone_index_of, active_milestone_index, split_milestone_label}
  - ProjectState.milestones
  - ProjectViewCache.roadmap_box_view + `v` toggle
affects: [Detail screen Roadmap tab, help screen, render_escape_guard probe, ui census]
tech-stack:
  added: []
  patterns:
    - pure layout -> typed segments -> both render_text (tests) and widget (styling)
    - Paragraph::scroll((y, x)) instead of Buffer::set_string index math
    - iterative DFS cycle breaking + Kahn longest-path layering (no recursion on input depth)
key-files:
  created:
    - src/ui/roadmap_graph.rs
  modified:
    - src/ui/mod.rs
    - src/ui/screens/mod.rs
    - src/ui/screens/detail.rs
    - src/ui/screens/help.rs
    - src/ui/screens/render_escape_guard.rs
    - src/state_reader/roadmap_md.rs
    - src/state_reader/mod.rs
decisions:
  - "Graph is the Roadmap tab default; `v` (Roadmap tab only) toggles the untouched RoadmapWidget box list, per project in ProjectViewCache"
  - "Milestone labels typed Untrusted inside RoadmapMilestone; ProjectState gains no free-string field"
  - "Heading-scope milestone parsing kept (covers the GSD template shape)"
  - "Decimal range membership: first <= p && (p <= last || p.major() == last.major())"
metrics:
  duration: ~19min
  started: 2026-09-23T21:48:21Z
  completed: 2026-09-23T22:07:12Z
  tasks: 3
  files: 8
estimate:
  tokens: 150000
  tasks: 3
actuals:
  tokens: 21170
  tasks: 3
  commits: 3
plan_head_before: 7cf69101b198e95214f9f6a9d5374f86506dbf3d
---

# Quick 260923-md1: Roadmap dependency graph view with milestone boundaries Summary

The Detail screen's Roadmap tab now draws a left-to-right phase dependency graph by default. It shows junction fan-out and fan-in, reference rows for edges it cannot draw directly, a `Milestones:` header band and row-end milestone labels. It also draws notes for external deps and cycles, plus a `▶ P<id>: <name>` detail line for the current phase. All of it comes from declared `**Depends on**` edges and ROADMAP.md milestone ranges. `v` toggles back to the unchanged box list.

## Commits

| Task | Commit | Title |
|------|--------|-------|
| 1 | 8137449 | feat(quick-260923-md1): roadmap tab dependency graph view with v toggle |
| 2 | 2c51845 | feat(quick-260923-md1): junction branches and fan-in for the roadmap graph |
| 3 | 76c1fd3 | feat(quick-260923-md1): milestone boundaries in the roadmap graph |

Commit count was measured: `git rev-list --count 7cf6910..HEAD` = 3. The plan ledger file could not be written into `.git/worktrees/...`, because the isolation guard blocks writes outside the worktree. The base sha was therefore recorded by hand as `plan_head_before`.

## Rendered example graph (oracle O10, asserted verbatim)

Test: `ui::roadmap_graph::tests::roadmap_graph_o10_example_with_milestones_end_to_end`. It goes end to end from a ROADMAP.md string through `parse_roadmap_phases`, `roadmap_milestones`, `active_milestone_index(&ms, "")`, `milestone_tags`, `milestone_index_of`, `layout_graph` and `render_text`:

```
Milestones: ◆ M3 live booking  ◇ M4 support chat  ◇ M5 web
8 ─► 9 ─┬─► 10 ─┬─► 12 ─┬─► 13 ─┬─► 14 ─► 15  (M3: live booking)
        └─► 11 ─┘       │       └─► 16 ─► 17  (M3 → M4: support chat)
                        └─► 18                (M5: web)
▶ P12: Booking core
```

(The third graph line is `" ".repeat(24) + "└─► 18" + " ".repeat(16) + "(M5: web)"`.)

Oracles O1-O10 (plus O3b and O4b) all pass as exact `Vec<String>` equality. **No whitespace adjustments were needed**: every hand-computed oracle matched the D2 rules as written.

## Verification

- Full suite, `rtk proxy cargo test --no-fail-fast`: 49 result lines, **2227 passed, 1 failed, 15 ignored** (raw `test result:` lines summed). The only failure is the known local git-version witness `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`. No test binary failed to compile.
- Targeted runs, all green:
  - `roadmap_graph`: 25
  - `roadmap_md`: 40
  - `state_reader`: 276
  - `render_escape_guard`: 14
  - `help`: 21
  - `ui::tests`: 8
  - `--test spawn_seam_guard`: 41 (unchanged; no new free-string field)
- `rtk proxy cargo clippy --all-targets -- -D warnings`: **zero findings in any file this task touched.** It still exits non-zero on 11 PRE-EXISTING findings in files this task never modified (`git diff --stat 7cf6910 -- src/browser.rs src/project_creator.rs tests/` is empty):
  - `src/browser.rs:156-158`: bool literal in assert_eq
  - `src/project_creator.rs:146`: cmp_owned
  - `tests/envelope_carrier_reach.rs:1711`
  - `tests/envelope_control_carrier.rs:978`
  - `tests/envelope_config_resolution.rs:2206`
  - `tests/envelope_wrapper_class.rs:6127, 6213, 10795, 11232`

  These are presumably new lints in the local toolchain (rustc 1.98.1). They are out of scope and listed under Deferred.
- `RoadmapWidget` is byte-identical. Its three existing tests pass untouched.
- The census `MEASURED_REACH` pin is unchanged. `src/ui/roadmap_graph.rs` contains zero `display_identity` occurrences.
- The collision map was respected. The detail.rs hunks are at about 16-19 (imports), about 1433 (adjudication prose), 3370 (`v` arm), 3852-3900 (`render_roadmap`), 6195 (footer arm) and 14320+ (tests at the end of the module). The help.rs hunks are the row after `r` (about 230) and a test at the end of the file. The render_escape_guard.rs hunks are at about 929 (fixture), 1378-1391 and 1456 (arrival rows) and 1643 (sub-state). Nothing touches normal.rs.

## Deviations from Plan

1. **[Rule 1 - test precision] Detail test (a) box-view assertion.** The plan asked that the box-view render "shows the box border `┌`". The graph view's header `Block` already draws a `┌` corner, so that check would pass vacuously. The test instead asserts that the box view draws MORE `┌` than the graph view (one per phase box), and that it does not contain `1 ─► 2`.
2. **[Cosmetic] Re-indented the preserved box-view statements** in `render_roadmap` by one level, so they sit correctly inside the new `if` arm. The statement text is unchanged.
3. **[Rule 2 - PI-9 interpretation] Pass-through through a cell the same group already owns.** Taken literally, "intermediate rows need no straight edge" would block a third fan-out branch from passing the second branch's `├` row, and would likewise block a fan-in passing another source's stub. The rule is applied as: a cell already owned by the same group is always claimable, and an unowned intermediate cell needs no straight edge and no stub. A reserved-but-unclaimed cell does NOT block a pass-through, per the rule as written.
4. **[Inferred] `<summary>` labels** also have leading non-alphanumeric characters (emoji) trimmed, the same treatment headings get, so `<summary>✅ v1.0 …` dedupes with its bullet twin.
5. Nothing else. No architectural changes and no package installs.

## Inferred decisions (for audit)

### CONTEXT `[INFERRED]` items, all implemented
- External deps are ignored for layout and surfaced in ONE note line: `external deps: 14 ◄ 7`.
- The pure layout module `src/ui/roadmap_graph.rs` implements typed segments, longest-path layering, iterative cycle breaking with a note, primary-parent chains, a downward branch search with pass-through, fan-in, reference rows, 4- and 6-cell gutters, glyphs derived from junction bits, and dash padding.
- The example-topology oracle (O7) is exact, with 18 drawn as a fan-out sibling of 13.
- Milestone rendering has a header band (◆ marks the active milestone), row-end labels with `→` at boundaries, and no in-row marker.
- UX: the graph is the default, `v` toggles, help documents it, the horizontal offset is automatic, vertical scrolling reuses `scroll_offset` and `generic_viewport`, and nothing panics at 1x1, 5x3, 10x3 or 20x5 (an offset-Rect no-bleed check is included).
- Discretion: the toggle state is per project, on `ProjectViewCache` (see PI-11).

### Planner-inferred PI-1 .. PI-12
- **PI-1**: Tasks ran in tracer-first order: an end-to-end path first, then junctions, then milestones.
- **PI-2**: A continuing row pads with one space, then dashes (`2 ───┬─►`). A node with no outgoing edge pads with spaces.
- **PI-3**: Row-end labels are aligned at the widest row + 2. Reference rows carry no label.
- **PI-4**: The header lists the milestones that have at least one member node, plus the active one, in roadmap order.
- **PI-5**: The current node is highlighted Yellow+BOLD+REVERSED, not with a `▶` prefix, so no column moves. The detail line is omitted when no phase is Current.
- **PI-6**: A singular `Phase N` counts only on `## Milestones` bullets. A milestone heading needs a version token (`\bv\d+(\.\d+)*\b`) or `Milestone <n>`. `### Milestone mapping (decision D10)` is ignored.
- **PI-7**: Dedupe treats two entries as the same milestone when their full labels are equal ignoring ASCII case, or when their short ids are version-like (`v`+digit or `M<n>`) and equal. The merged entry keeps the first entry's label and position, ORs `in_progress`, fills a missing range from the later entry, and appends its scoped keys.
- **PI-8**: The note formats are `external deps: a ◄ b; c ◄ d, e` and `dependency cycle: <DFS stack path> (edges ignored)`. The self-dependency form is `dependency cycle: 1 (edges ignored)`.
- **PI-9**: Junction-cell ownership works through FanOut(P) and FanIn(T) claims. A blocked pass-through falls back to a fresh row plus a reference row, and a blocked fan-in becomes a reference row (O9). See Deviation 3 for the same-group refinement.
- **PI-10**: Styling:
  - The graph is indented 2 columns (`Margin::new(2, 0)`).
  - Nodes: Done is DarkGray, Current is Yellow+BOLD+REVERSED, Future is default.
  - Edges and spaces: default. References: DarkGray+DIM. Row-end labels: Cyan.
  - Header: `Milestones: ` is BOLD, the active item Yellow+BOLD, other items default.
  - Footer: notes DarkGray, detail line Yellow.
- **PI-11**: The `v` toggle state is per project, in `ProjectViewCache.roadmap_box_view`. `false` (the default) means the graph. Toggling resets `scroll_offset` to 0.
- **PI-12**: The heading-scope fallback was KEPT, because it covers the GSD template shape (`### 🚧 v8.1 Core (In Progress)` with `#### Phase 5:` beneath it).

### Executor-added
- **EX-1**: Pending branches are taken deepest child layer first, with ties to the lower roadmap index. A branch search that finds no row falls back to a fresh row plus a reference row, and that reference row sorts with the others by target index, then by the source's declaration position.
- **EX-2**: Reserved trailing gaps count as "used" for branch placement, condition (a), but do not block a vertical pass-through.
- **EX-3**: A fan-in is refused when source and target share a row. It becomes a reference row.
- **EX-4**: `active_milestone_index` matches the full label or the first token, both ignoring ASCII case.

## Deferred (recorded, not implemented)

1. **`#### Build phase N (Milestone M)` headings are not parsed as phases** (research open question 1). `parse_roadmap_phases`'s `heading_re` requires `#{2,4} Phase <id>:`. Because of that, the user's sketch project (ttbook) shows **only the phases today's parser recognizes** (about phases 1-7.1), not the sketch's build phases 8-18. The sketch's topology cannot appear there until the phase parser is extended.
2. Milestone-prefixed ids such as `Phase 1-01` fail `PHASE_ID` today. On a milestone line, the range regex would also read `Phase 1-01` as the range 1..1.
3. The Progress-table `Milestone` column as a third membership source.
4. The stale help row `r  Toggle roadmap visualization (detail view)`, which is pinned at help.rs (the `flag-off` test). It was left untouched.
5. Display-width (unicode-width) measurement instead of `chars().count()`. This is the same deferral as the box widget's IN-02 and IN-03.
6. Eleven pre-existing clippy findings in untouched files (listed under Verification). They are out of scope for this task.

## Threat model

All mitigations are in place:
- **T-md1-01**: Every id, dep id, name and milestone label goes through `render_for_terminal` or `Untrusted::shown()` before it is measured. Unit tests cover ESC and U+202E in names, deps and milestone labels. A buffer-level spot-check was added to `ui::tests`. The probe covers the graph (with a hostile milestone) and the box view (new sub-state).
- **T-md1-02**: Cycle detection is an iterative DFS and layering uses Kahn's order. O3 and O3b prove termination.
- **T-md1-03**: Widths are converted with saturating `u16::try_from`, drawing uses `Paragraph::scroll`, and zero-size Rects return early. Tiny-size and no-bleed tests cover this.

No new threat surface beyond the plan.

## Known Stubs

None.

## Self-Check: PASSED

- FOUND: src/ui/roadmap_graph.rs
- FOUND commits: 8137449, 2c51845, 76c1fd3 (`git log --oneline -4`)

## Post-execution review fixes (orchestrator, unattended)

Code review (`260923-md1-REVIEW.md`): 0 critical / 3 warning / 7 info. Fixed:
- WR-03 `3b0830c` — exact milestone range match wins over the decimal-belongs-to-integer rule.
- WR-01 `10a0bf3` — graph footer bounded (cycle notes collapse to one line; graph keeps >= half the body).
Deferred [INFERRED]: WR-02 (no manual horizontal scroll; auto-offset keeps the current phase in view per CONTEXT decision) and all INFO items.
Verification (`260923-md1-VERIFICATION.md`): passed, 9/9 must-haves.
Clippy: 11 pre-existing findings in untouched files (src/browser.rs, src/project_creator.rs, tests/envelope_*.rs — new lints under the installed rustc); zero in changed files.
