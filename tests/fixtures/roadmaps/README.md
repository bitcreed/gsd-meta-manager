# Real-roadmap fixtures (sanitised excerpts)

Structural excerpts of three real GSD projects' `.planning/ROADMAP.md` and
`.planning/STATE.md`, vendored on 2026-09-24 for Phase 24 (Roadmap tab
redesign). Each one reproduces a roadmap shape, or a defect of the old Roadmap
graph, that the state reader and the Roadmap list must handle. Quoted in
`.planning/phases/24-roadmap-tab-redesign-and-detail-tab-consolidation/24-RESEARCH.md`,
section "Real Fixtures".

| File | Source project | Source commit | Excerpted | Shape / defect it pins |
|---|---|---|---|---|
| `ttbook-ROADMAP.md` | ttbook | `9b7b305` | 2026-09-24 | `#### Build phase N (Milestone M): Title` placeholder phases 14-18 under `### 📋 Milestone M "…" (planned)` headings; build-phase dependency lines (`Build phases 8-13`, `Build phases 12, 13`, `Build phase 14`); the old milestone detector read each build heading as a spurious milestone; shipped v1 phases written as non-bold lines inside `<details>` (not parsed as phases) |
| `ttbook-STATE.md` | ttbook | `c71fcd0` | 2026-09-24 | `milestone: v2` names a roadmap milestone by its first token |

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
  titles of varied length).
- **Dropped:** requirement, gate and success-criteria lines and other prose
  sections the reader does not parse.
- STATE.md fixtures are the frontmatter keys the reader uses, in source key
  order, followed by `# Project State`.

Tests read these files only through `include_str!`, never from the source
projects' paths, so they run anywhere.
