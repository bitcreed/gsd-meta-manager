# Real-roadmap fixtures (sanitised excerpts)

Structural excerpts of three real GSD projects' `.planning/ROADMAP.md` and
`.planning/STATE.md` (and one `HANDOFF.json`), vendored on 2026-09-24 for Phase 24 (Roadmap tab
redesign). Each one reproduces a roadmap shape, or a defect of the old Roadmap
graph, that the state reader and the Roadmap list must handle. Quoted in
`.planning/phases/24-roadmap-tab-redesign-and-detail-tab-consolidation/24-RESEARCH.md`,
section "Real Fixtures".

| File | Source project | Source commit | Excerpted | Shape / defect it pins |
|---|---|---|---|---|
| `ttbook-ROADMAP.md` | ttbook | `9b7b305` | 2026-09-24 | `#### Build phase N (Milestone M): Title` placeholder phases 14-18 under `### 📋 Milestone M "…" (planned)` headings; build-phase dependency lines (`Build phases 8-13`, `Build phases 12, 13`, `Build phase 14`); the old milestone detector read each build heading as a spurious milestone; shipped v1 phases written as non-bold lines inside `<details>` (not parsed as phases) |
| `ttbook-STATE.md` | ttbook | `c71fcd0` | 2026-09-24 | `milestone: v2` names a roadmap milestone by its first token |
| `daily-vow-ROADMAP.md` | daily-vow | `abc11af` | 2026-09-24 | `## Milestones` list with the `--` separator (v1.0-v1.4 shipped, v1.5 in progress); shipped phases 1-17 only in a `## Complete Phase History` table; phase 23 declares `Phase 21 (…), Phase 20 (…)`, so 20 is an implied dependency via 21 and the old graph drew `20` twice (a reference row); `## Requirement Coverage (v1.5)` reads as a spurious member-less milestone |
| `daily-vow-STATE.md` | daily-vow | `abc11af` | 2026-09-24 | `milestone: v1.5` + `milestone_name` |
| `sentriq-ROADMAP.md` | sentriq | `49533e9` | 2026-09-24 | no `## Milestones` list: the active milestone is only a bold `**v0.12 — Actuation Routines** (phases 9-12)` line, so no roadmap milestone holds phases 9-12 (a synthetic band is needed); `## v0.11 Phases (4-7)` carries its range in parentheses; phase 11 declares `Phase 10 (…), Phase 9 (…)`, so 9 is implied via 10 and the old graph drew `9` twice; phase 12 declares no dependency; `## Scope Explicitly Excluded from v0.12` reads as a spurious member-less milestone. Known difference from Mockup C: its "earlier" row also names `pre-GSD 1–3` and `TASK-111 (quick)`, but those are index-only table sections, not milestones under any detector the reader has, so only `v0.11` is reported shipped |
| `sentriq-STATE.md` | sentriq | `dfc6d2c` | 2026-09-24 | `milestone_name` written AFTER the `progress:` block |
| `ttbook-phase13-ROADMAP.md` | ttbook | `3f51b43` | 2026-09-26 | a copy of `ttbook-ROADMAP.md` with phases 8-11 ticked `[x]` and the `## Progress` rows 8-11 `Complete`, 12-13 `In Progress` — ttbook's real bookkeeping of 4 of 6 while phase 12 is executed and verified on disk (quick 260926-16t: the dashboard said `4/6 phases`, the Roadmap header `5 of 11 phases done`) |
| `ttbook-phase13-STATE.md` | ttbook | `e4e5db2` | 2026-09-26 | `current_phase: 13`, `status: executing`, `last_updated` later than the handoff |
| `ttbook-phase13-HANDOFF.json` | ttbook | `a5a7733` | 2026-09-26 | a stale handoff: `phase: "12"` behind STATE.md's 13, `status: ready`, `timestamp` with a `-05:00` offset. Only the keys the reader uses, with a neutral `phase_name`; none of the source's task, decision or note content is copied. Written into a temp `.planning/` together with phase directories 08-13 by `state_reader::write_ttbook_phase13_fixture` |

## These are SANITISED excerpts, not copies

The source repositories are **private**; this crate is public and publishes
`tests/`. So nothing here is verbatim prose from them:

- **Kept, byte-for-byte in shape:** every heading and its level, every phase and
  milestone id, the `(Milestone M)` parentheticals, quotes, separators (`-`,
  `--`, `—`), emoji, `<details>` / `<summary>` wrappers, checklist boxes, plan
  file names, the `## Progress` table's columns, and every `**Depends on**:`
  line (a parenthetical's contents become neutral text, but the parentheses and
  any `Phase N` token inside them stay, so the parse is unchanged).
- **Replaced:** every `**Goal**:` body (with `(sanitised) ` followed by neutral
  filler of roughly the original length), plan-line descriptions and every kept
  prose line (with `(sanitised)`), and — for ttbook, which does not appear in
  the public `MOCKUPS.md` — every phase title and milestone name (with neutral
  titles of varied length). daily-vow's and sentriq's phase and milestone
  titles are kept: they already appear in this repository's public
  `.planning/phases/24-*/MOCKUPS.md`. daily-vow's shipped-phase history table
  keeps only its shape (ids, milestone, plan counts, dates).
- **Dropped:** requirement, gate and success-criteria lines and other prose
  sections the reader does not parse.
- STATE.md fixtures are the frontmatter keys the reader uses, in source key
  order, followed by `# Project State`.

Tests read these files only through `include_str!`, never from the source
projects' paths, so they run anywhere.

## Rules for adding a fixture

Two invariants hold for every file in this directory, and the unit test
`state_reader::roadmap_md::tests::vendored_roadmap_fixtures_are_sanitised`
enforces both (add a new file to its list):

1. Every line containing `**Goal` also contains `(sanitised)` — a goal body is
   always replaced, never copied.
2. No file contains `/home/` — no absolute filesystem path of the machine the
   excerpt was taken on.

Beyond what the guard can check: replace every other prose line too, keep the
structure byte-for-byte, and add a row to the table above with the source
commit (`git log -1 --format=%h -- .planning/ROADMAP.md` in the source
repository) and the shape or defect the fixture pins.
