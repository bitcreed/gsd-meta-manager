---
phase: quick-260926-fi9
plan: 01
subsystem: state_reader / roadmap tab
status: complete
tags: [roadmap, parser, shipped-milestones, display-only]
requires: []
provides:
  - roadmap_md::parse_shipped_phases (closed-milestone collapse history, display-only)
  - ProjectState::shipped_phases
  - one shared phase-line grammar (recognize_phase_line) for both channels
affects:
  - src/state_reader/roadmap_md.rs
  - src/state_reader/mod.rs
  - src/ui/screens/detail.rs
tech-stack:
  added: []
  patterns: [region-split parsing (current vs closed collapse), GSD closed-marker port]
key-files:
  created:
    - tests/fixtures/roadmap-shapes/v1-era-ROADMAP.md
    - tests/fixtures/roadmap-shapes/README.md
  modified:
    - src/state_reader/roadmap_md.rs
    - src/state_reader/mod.rs
    - src/ui/screens/detail.rs
    - tests/fixtures/roadmaps/README.md
decisions:
  - "Plan's inferred decisions 1-12 executed as written; none overturned"
  - "[INFERRED] Shipped Roadmap nodes are filtered out of start_now and never share `parallel` with live phases (adapter post-process in roadmap_model_for)"
metrics:
  duration: 926s
  completed: 2026-09-26
actuals:
  tokens: 13907
  tasks: 3
  commits: 3
plan_head_before: 864922d614f19f475eb96f8e4cb1e8813231994b
---

# Quick 260926-fi9: Parse v1-era ROADMAP.md phase shapes Summary

Phase lines inside shipped-milestone `<details>` collapses now go to a display-only
`ProjectState::shipped_phases` channel, split off by a port of GSD's own closed-marker rule. The
Roadmap tab draws them in their shipped bands. The GSD-facing parser now shares one grammar with the
new channel, and that grammar covers every surveyed shape: bold with any tail, plain checkbox,
spaced-dash headings, and bare lines, which are only read inside a closed collapse.

## Tasks

| Task | Name | Commit |
|------|------|--------|
| 1 | Tracer: closed-collapse plain lines reach the Roadmap tab's shipped band | d94f980 |
| 2 | Every observed and requested shape parses in both channels; negatives stay prose | e9a4085 |
| 3 | Shipped rows explain themselves on Enter; renumbering, hostile names, docs | c458c7c |

## What was built

- `closed_milestone_lines` ports GSD's `stripClosedMilestoneDetails` / `isClosedMilestoneHeading`,
  tracking nested blocks with a stack. A block with no summary, or with no closing tag, is not closed.
- `recognize_phase_line` tries its recognizers in this order: R1 (the original bold grammar, captures
  unchanged), R2 (bold with any tail), R3 (plain checkbox), colon or spaced-dash headings, then R5
  (bare lines, closed region only).
  - `parse_roadmap_phases` and `parse_shipped_phases` both run `parse_phases_in_region`.
  - Plan counts are the per-field max of the line's tally and the scanned plan items.
  - A heading in the closed region counts as complete only when all its listed plans are checked.
- `phase_heading_re`, which also feeds goals and sections, and `roadmap_milestones`' phase-heading regex
  both accept `### Phase N — Title`. The separator must be spaced.
- `roadmap_model_for` puts shipped nodes first. Their band comes only from the milestone ranges. A GSD or
  planned entry wins a shared key, and nothing is merged across channels.
  - The marker comes from the checkbox only, with no disk lookup.
  - Enter on a shipped node explains that it shipped in an earlier milestone. The id is escaped.

**Shapes supported (dedupe rule):** within a channel there is one row per `phase_key`, with fields
merged: checkbox OR, plan counts max, first non-empty name, description and depends. Across channels
the GSD-facing entry wins and nothing is merged.

## Verification (gate results)

| Gate | Before (864922d) | After (c458c7c) |
|------|------------------|-----------------|
| `rtk proxy cargo test --no-fail-fast` | 56 suites, 2665 passed, 1 failed, 15 ignored | 56 suites, 2680 passed, 1 failed, 15 ignored |
| Only failure | `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against` (expected env witness) | same, and nothing else |
| `rtk proxy cargo clippy --all-targets -- -D warnings` | — | exit 0 |
| `#[test]` counts roadmap_md / mod / detail | 61 / 42 / 258 | 69 / 44 / 263 |
| `grep -rn shipped_phases src/driver src/agents` | — | no match |
| `git diff -- tests/fixtures/roadmaps/` | — | README.md only |

No pre-existing `#[test]` body was edited. I checked the diff: every removed line is non-test code, and
test-region hunks are pure additions.

**Real-roadmap regression dump (throwaway, not committed).** I dumped `parse_roadmap_phases`, goal
keys and milestones for 47 roadmaps (every `~/projects/**/.planning/ROADMAP.md`, their milestone
archives, the vendored fixtures and this repo's roadmap) and compared the output before and after each task.
- Task 1 changed only six **milestone-archive** files (`hitchmatch`/`hm-relverify` v1.2 and v1.3,
  `wordoclock` v1.1 and v1.3). Those had bold phase lines inside SHIPPED collapses that were parsed
  before. The app never parses archive files (`parse_roadmap_phases` has a single caller, which reads the live
  `.planning/ROADMAP.md`), so no live roadmap changed.
- Task 2 changed only mailbot's live roadmap: `03.1` now reads its `- [x] **Phase 03.1: …** (INSERTED) - …`
  checklist line. That makes it completed, gives it its description, and moves it to checklist order. This is
  the survey's intended fix. Goals and milestones are identical for all 47 files.
- Shipped channel on live roadmaps:
  - aiFlowAgent 1-8, predix 1-10+, picsync 1-8, wordoclock 1-11+, hitchmatch/hm-relverify 1-12+
    and shopify-orderly-rescue each list their history.
  - This repo lists 01-13, then its later collapses.
  - daily-vow, sentriq, usbee, nomosquitoz, mailbot and and-bible list nothing.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Shipped nodes polluted "Start now" and "Parallel"**
- **Found during:** Task 1
- **Issue:** `layout_list` puts every Ready or Active node into `start_now` and every same-layer node into
  `parallel`. An unfinished shipped phase, such as a `- [ ] Phase 11: … — rescoped` line (hitchmatch) or the
  fixture's phase 4, would have shown in the Roadmap's `Start now:` line. All of ttbook's shipped phases
  1-7.1 would also have been listed as "Parallel" with the live phase 8 in the detail pane.
- **Fix:** `roadmap_model_for` post-processes the model:
  - it drops shipped nodes from `start_now`;
  - it keeps `parallel` within one channel.
  - `roadmap_graph` is untouched.
- **Files modified:** src/ui/screens/detail.rs
- **Commit:** d94f980

### Notes (not deviations)

- The plan's survey says "No live roadmap has a currently-parsed phase line inside any `<details>` block".
  This is true for live roadmaps but not for milestone archives (see above). There is no runtime impact.
- A live phase that declares a dependency on a shipped id (for example `Depends on: Phase 13` when 13 is
  shipped) now gets a real edge to the shipped node instead of an external reference. The shipped node is
  Done, so the live phase stays Ready. The lane is laid out on the unfolded list, like the edges GSD phases
  already had into shipped bands.

## Inferred decisions (audit)

- All 12 inferred decisions in the plan were executed as written. None was overturned.
- New: `[INFERRED]` Shipped nodes are never "start now" and never parallel with a live phase (Rule 1 above).
- New: `[INFERRED]` R2's leading-parenthetical loop treats a second tally group as a tag. No observed line
  has two tallies.
- `split_summary_rest` reads the first tally anywhere in the text, even after a separator, and removes
  it from the description. This matches the plan's "first tally group removed" wording.

## Known Stubs

None.

## Threat Flags

None. Every new surface is covered by T-fi9-01..05. T-fi9-01 is covered by
`a_hostile_shipped_phase_name_renders_escaped`, T-fi9-02 by the driver/agents grep and T-fi9-03 by the
two renumbering tests.

## Self-Check: PASSED

- FOUND: tests/fixtures/roadmap-shapes/v1-era-ROADMAP.md, tests/fixtures/roadmap-shapes/README.md
- FOUND commits: d94f980, e9a4085, c458c7c
