---
phase: quick-260926-gtl
plan: 01
type: execute
wave: 2
depends_on: ["260926-gtk"]
files_modified:
  - src/state_reader/roadmap_md.rs
  - src/state_reader/mod.rs
  - src/agents/worktrees.rs
autonomous: true
requirements: [QUICK-260926-gtl]

must_haves:
  truths:
    - "A letter-suffixed phase id (`12A`, `12A.1`, `23A.1.2`) is recognised by every roadmap phase-line recogniser, keeps its full id, has its `23A.1.2-01-PLAN.md` checklist items counted, and keys its goal by the full id; every decimal, padded, project-code-prefixed and v1-era shape (quick 260926-fi9) parses exactly as before"
    - "`**Depends on**:` prose is read with gsd-core 1.15.0's PHASE_DEP_REF grammar (#4764): case-insensitive `phase`/`phases`, lists joined by `,` / `, and` / `and` / `&`, ranges by `-` / `to` / `through` yielding ENDPOINTS only, the row's own phase dropped, pad- and letter-case-insensitive dedupe keeping the first-written spelling — and it reproduces the 1.15.0 oracle's `init manager` dep_phases for every non-parenthetical line in this plan's oracle table"
    - "Parenthetical qualifiers in a dependency line are still stripped before extraction (Plan 20-03's T-20-16 mitigation); this is the one recorded divergence from 1.15.0 and is pinned by a test, not left implicit"
    - "A hard-wrapped `**Goal**` field is read past the line break and stops exactly where 1.15.0's extractPhaseFieldMultiline stops (blank line, `-`/`*`/`+` list item, `**Label**` line of either case, heading, table row, code fence; a fence on the label line means no goal), matching the oracle-verified table in this plan; the two recorded divergences (empty label line, level-5/6 heading) are pinned by tests"
    - "A Codex-style agent branch `agent-p12A-01-<ts>` and a ledger id `12A-01` pass the plan-id rule; lowercase `13x`, double-letter, leading-letter and dot-traversal shapes are still rejected"
    - "`rtk proxy cargo test --no-fail-fast` shows no failure other than `the_config_section_constants_record_the_git_version_they_were_derived_against` (src/envelope/policy.rs), the router conformance test passes against the installed oracle (not a SKIP), and `rtk proxy cargo clippy --all-targets -- -D warnings` exits 0"
  artifacts:
    - path: "src/state_reader/roadmap_md.rs"
      provides: "widened PHASE_ID + plan-checklist regexes, PHASE_DEP_REF port in parse_depends_on (own-number self-skip, normalised dedupe), extractPhaseFieldMultiline port in parse_phase_goals, parity tests"
      contains: "fn parse_depends_on(text: &str, own_number: &str)"
    - path: "src/agents/worktrees.rs"
      provides: "plan_id_re accepting `^[0-9]+[A-Z]?(\\.[0-9]+)*(-[0-9]+)?$`, letter-suffixed branch tests"
    - path: "src/state_reader/mod.rs"
      provides: "end-to-end tracer test through parse_project_state; phase_goals doc updated"
      contains: "fn letter_suffixed_phase_reads_end_to_end"
  key_links:
    - from: "roadmap_md::parse_phases_in_region"
      to: "roadmap_md::parse_depends_on"
      via: "the entry's own recognised number is passed so a self-reference is dropped"
      pattern: "parse_depends_on\\(&dep_caps\\[1\\], &phase.number\\)"
    - from: "driver/router.rs unsatisfied_dependency / reaches"
      to: "RoadmapPhase::depends_on"
      via: "unchanged consumer — the router gates execute/discuss on exactly the list this parser now produces"
      pattern: "depends_on"
    - from: "state_reader/mod.rs parse_project_state"
      to: "roadmap_md::parse_phase_goals"
      via: "state.phase_goals = roadmap_md::parse_phase_goals(&content)"
      pattern: "phase_goals = roadmap_md::parse_phase_goals"
    - from: "agents/worktrees.rs parse_agent_branch + read_ledger_plan"
      to: "valid_plan_id"
      via: "the widened plan-id rule"
      pattern: "valid_plan_id"
---

<objective>
Bring the ROADMAP.md reader to parity with gsd-core 1.15.0 (release-1.15.0, ec81d0d) on three
upstream fixes, and widen the agent-worktree plan-id rule to the same phase-token grammar:

1. **#4764 PHASE_DEP_REF** — `parse_depends_on` reads phase references the way GSD's own router
   (`init manager` dep_phases) now does: plural lists, `and`/`&` joins, ranges as endpoints,
   case-insensitive, self-reference dropped.
2. **#4731 / #4837 extractPhaseFieldMultiline** — `parse_phase_goals` reads hard-wrapped goals
   up to upstream's structural boundaries (this repo's own ROADMAP `999.4` goal wraps over three
   lines and is truncated to its first line today).
3. **#2128 / #4830 letter-suffixed ids** — `PHASE_ID` accepts `\d+[A-Z]?(?:\.\d+)*` shapes
   (`12A`, `23A.1.2`), and `agents/worktrees.rs`'s `plan_id_re` accepts `12A-01`.

Purpose: the driver's router transcribes GSD's routing table (tests/driver_router_conformance.rs),
so the dependency list it gates on must be read the way GSD reads it; the Roadmap tab should show
the goal GSD shows. Output: the three edited source files, parity tests pinned to values measured
from the built 1.15.0 oracle, and a SUMMARY that lists every INFERRED decision for audit.

The human is unavailable. Decisions below that the operator spec did not state are marked
**INFERRED** — copy each into the SUMMARY's "Inferred decisions" list verbatim.
</objective>

<assumption_delta_decision>
Detector (plan:pre hook) fired on `plural`/`another`/`also`/`optional`. Most hits are lexical (a
regex's "plural lists", "optional prefix"). The real question it points at: phase identity is the
numeric `PhaseNum` (with `phase_key` falling back to raw text), and letter-suffixed ids do not fit
that numeric identity.

- Primary noun: the phase id as `phase_key` renders it (numeric canonical, raw text otherwise). Unchanged.
- Decision: **add-alongside** (INFERRED). Letter-suffixed ids use `phase_key`'s existing raw-text
  fallback next to the numeric `PhaseNum` identity. `PhaseNum` is not promoted in this item. The
  only letter-aware normalisation added is the private dependency dedupe/self-skip key, which is
  local to `parse_depends_on`.
- Accepted debt: `012A` and `12A` do not merge as one roadmap row. A `Phase 012A` dependency on a
  roadmap `12A` row reads as undeclared, so the router fails closed. Letter-suffixed agent rows are
  not attributed to a plan. The promote is to give `PhaseNum` an optional per-segment letter,
  ordered the way upstream's `comparePhaseNum` orders it. It becomes necessary when a registered
  project uses letter-suffixed phases with padded headings, or runs letter-suffixed executor
  worktrees. List this under follow-ups in the SUMMARY.
</assumption_delta_decision>

<execution_context>
@~/.claude/gsd-core/workflows/execute-plan.md
@~/.claude/gsd-core/templates/summary.md
</execution_context>

<context>
@CLAUDE.md
@src/state_reader/roadmap_md.rs
@src/state_reader/phase_num.rs
@src/agents/worktrees.rs

Upstream is a READ-ONLY sibling checkout. Read it only with
`git -C ~/projects/node/gsd-core show upstream/release-1.15.0:<path>`; never check out a branch
there. The built 1.15.0 oracle (used below to measure every expected value) lives at
`/tmp/claude-1000/-home-blk-projects-rust-gsd-meta-manager/9e776af9-6a4a-4326-9fd5-76d0bedeccfe/scratchpad/gsd115`
with a fake home at `.../scratchpad/fakehome` — it may have been cleaned up by the time this runs;
nothing in this plan REQUIRES it (every expected value is already written down below).

Sequencing: this item runs after 260926-gtk on the primary checkout (no worktrees, no branches,
never push). 260926-gtk touches config_json.rs / detail.rs / docs/GSD-CORE-SYNC.md — no overlap
with this plan's files. Do NOT edit docs/GSD-CORE-SYNC.md (it is the config-key sync record owned
by 260926-gtk).
</context>

<upstream_reference>
Verbatim from `upstream/release-1.15.0` (ec81d0d). Port the behaviour, not the TypeScript.

**src/phase-id.cts:65 — the canonical phase-number token (#2128):**

```ts
const PHASE_NUMBER_TOKEN_SOURCE = '\\d+[A-Z]?(?:\\.\\d+)*';
```

**src/phase-id.cts:80-81 — the dependency-reference grammar (#4764):**

```ts
const PHASE_DEP_REF_SOURCE =
  `\\bphases?\\s+(${PHASE_NUMBER_TOKEN_SOURCE}(?:(?:\\s*,\\s*(?:and\\s+)?|\\s+and\\s+|\\s*&\\s*|\\s+(?:to|through)\\s+|\\s*-\\s*)${PHASE_NUMBER_TOKEN_SOURCE})*)`;
```

Its own comment: "The `-` separator deliberately extracts range ENDPOINTS only ("Phase 1-3" → 1, 3)
— the pre-#4764 behavior; interior enumeration stays out".

**src/init.cts:3086-3095 and 3148-3176 — the router's consumer (flags `gi` on both regexes):**

```ts
function normalizePhaseNumber(value: string): string {
  return value
    .split('.')
    .map((part) => {
      const match = /^(\d+)([A-Z]?)$/i.exec(part);
      if (!match) return part;
      return `${Number(match[1])}${match[2].toUpperCase()}`;
    })
    .join('.');
}
// ...
const depPhaseRefRe = new RegExp(`${PHASE_DEP_REF_SOURCE}`, 'gi');
const depTokenRe = new RegExp(`${PHASE_NUMBER_TOKEN_SOURCE}`, 'gi');
// per phase row, when depends_on is present and not /^none$/i:
const ownNumber = normalizePhaseNumber(phase['number'] as string);
const depNums: string[] = [];
const seen = new Set<string>();
while ((refMatch = depPhaseRefRe.exec(prose)) !== null) {
  depTokenRe.lastIndex = 0;
  while ((tok = depTokenRe.exec(refMatch[1])) !== null) {
    const normalized = normalizePhaseNumber(tok[0]);
    if (normalized === ownNumber) continue; // #4764: never the row's own phase
    if (seen.has(normalized)) continue;
    seen.add(normalized);
    depNums.push(tok[0]);
  }
}
```

Upstream's comment on direction: "silently dropping a REAL dependency would clear deps_satisfied
prematurely, the dangerous direction. Negation prose ("dropped the dependency on Phase 654") is
NOT detected: the issue's own minimum keeps such tokens."

**src/roadmap-parser.cts:2294-2340 — extractPhaseFieldMultiline (#4731, #4837):**

```ts
function extractPhaseFieldMultiline(section: string, label: string): string | null {
  const labelRe = new RegExp(
    '^[ \\t]*\\*\\*' + label + '(?::\\*\\*|\\*\\*\\s*:?)\\s*([^\\n]+)',
    'im',
  );
  const match = section.match(labelRe);
  if (!match) return null;
  const startIdx = match.index ?? 0;
  const after = section.slice(startIdx + match[0].length);
  const firstLine = match[1].trim();
  if (/^(?:`{3,}|~{3,})/.test(firstLine)) return null;
  const contLines = [];
  const lines = after.split('\n');
  for (let li = 0; li < lines.length; li++) {
    const raw = lines[li];
    if (li === 0 && !raw.trim()) continue;
    if (!raw.trim()) break;
    if (/^\s*[-*+]\s/.test(raw)) break;
    if (/^\s*\*\*[A-Za-z][A-Za-z ]*:?(\*\*)?:?\s/.test(raw)) break;
    if (/^\s*#{1,4}\s/.test(raw)) break;
    if (/^\s*\|/.test(raw)) break;
    if (/^\s*(?:`{3,}|~{3,})/.test(raw)) break;
    contLines.push(raw.trim());
  }
  return [firstLine, ...contLines].join(' ').trim() || null;
}
```

Callers: `roadmap.cts:239` and `:515` (`roadmap get-phase` / `roadmap analyze` goal),
`init.cts:2940` (`init manager` goal). The Rust app reads only the Goal field (nothing in `src/`
consumes a roadmap Requirements field), so only Goal is ported.
</upstream_reference>

<oracle_tables>
Every "1.15.0" value below was MEASURED on 2026-09-26 by running the built 1.15.0 oracle
(`node .../gsd115/gsd-core/bin/gsd-tools.cjs init manager --raw` for dep_phases,
`roadmap get-phase N` for goals) over fixture roadmaps holding exactly these lines. Pin the
"Rust (this plan)" column in tests.

**Dependency lines** — row = the entry's own `### Phase <row>:` heading:

| row | `**Depends on**:` text | 1.15.0 dep_phases | Rust (this plan) |
|---|---|---|---|
| 2 | `phase 1` | 1 | 1 |
| 3 | `Phases 1, 2, and 12A; see 2026-09-14 and sha 8bf403100d` | 1, 2, 12A | 1, 2, 12A |
| 12A | `Phase 1-3 and Phase 12A` | 1, 3 | 1, 3 |
| 21 | `Phase 15 only — parallel-eligible with Phases 17-21` | 15, 17 | 15, 17 |
| 22 | `Phase 1 & 2, phases 3 to 12A through 20` | 1, 2, 3, 12A, 20 | 1, 2, 3, 12A, 20 |
| 22 | `Phase 15 only — parallel-eligible with Phases 17-21, but must land before Phase 20 closes` (this repo's real phase 22 line) | 15, 17, 21, 20 | 15, 17, 21, 20 |
| 07 | `Phase 7, Phase 8, phase 08, PHASES 9 AND 10` | 8, 9, 10 | 8, 9, 10 |
| 12A | `phase 12a, Phase 012A, Phase 11` | 11 | 11 |
| 30 | `Phase 16 and 17; Phases 18-20; Phase 19. then Phase 21,` | 16, 17, 18, 20, 19, 21 | 16, 17, 18, 20, 19, 21 |
| 31 | `Subphase 4, Build phases 8-13, Phase M-2, phase 5 through 6` | 8, 13, 5, 6 | 8, 13, M-2, 5, 6 (divergence B) |
| 14 | `Nothing (no v2.0 dependencies — parallel-safe, can ship any time)` | (none) | (none) |
| 32 | `None` | (none) | (none) |
| 20 | `Phase 16, Phase 17, Phase 19 (and Phase 22 must land before this phase closes)` (this repo's real phase 20 line) | 16, 17, 19, 22 | 16, 17, 19 (divergence A) |
| 12 | `Nothing in this milestone — opportunistic, and blocks on nothing in Phases 9-11` (sentriq fixture's phase 12) | 9, 11 (negation prose kept by design) | 9, 11 |

Divergence A (INFERRED, keep): parenthetical qualifiers stay stripped. This is Plan 20-03's
threat-register mitigation T-20-16 ("parenthetical qualifier text is never promoted into an
identifier, so a dependency condition cannot be made unsatisfiable — or trivially satisfiable —
by prose"). Measured cost of dropping it on THIS repo's ROADMAP: 1.15.0 gives phase 20 →
16, 17, 19, 22 while phase 22 → 15, 17, 21, 20, a 20↔22 cycle that makes both unsatisfiable,
and phases 24/25 gain 15, 23, 14 (and 24) out of an "independent of … phases 15-23 … like Phase
14" aside. Reversing a threat-register mitigation is scope-changing, so it stays; the divergence
is pinned and reported instead.

Divergence B (pre-existing, keep): the Rust token keeps its optional project-code prefix
(`M-2`, `AB-29`), which the pinned test `Phase 0.3, Phase M-2, Phase AB-29` requires; upstream's
token grammar has no prefix and skips them.

**Goal fields** — each body sits alone under `### Phase 1: T` (use one roadmap string per case so
an unbalanced fence in one case cannot leak into another):

| body (`\n` = line break) | 1.15.0 goal | Rust (this plan) |
|---|---|---|
| `**Goal:** Decide whether the `COVERAGE.md` sub-stage should be read as a *state* parsed\nfrom frontmatter rather than as a *presence bit*, and change the Pipeline drill-down\nrendering if so.\n**Requirements:** TBD` (this repo's ROADMAP 999.4, verbatim) | `Decide whether the `COVERAGE.md` sub-stage should be read as a *state* parsed from frontmatter rather than as a *presence bit*, and change the Pipeline drill-down rendering if so.` | same |
| `**Goal**: Ship it\n- a bullet` (also `* a bullet`, `+ a bullet`) | `Ship it` | same |
| `**Goal**: Ship it\n   indented wrap   \n\| a \| table \|` | `Ship it indented wrap` | same |
| `**Goal**: Do the thing\n```\nnot this\n```` (also with `~~~`) | `Do the thing` | same |
| `**Goal:** ```js\nfoo()\n```` | null | no goal for the key |
| `**Goal** without colon\nwrapped too` | `without colon wrapped too` | same |
| `**Goal**: one\n**requirements**: lower` | `one` | same |
| `**Goal**: eight\n\ntail after blank` | `eight` | same |
| `**Goal**: nine\n1. numbered item\nPlans:` | `nine 1. numbered item Plans:` | same (numbered items and bare `Plans:` are not boundaries upstream) |
| `**Goal**: letter-suffixed` under `### Phase 23A.1.2: K` | `letter-suffixed` | same, keyed `23A.1.2` |
| `**Goal:**\nnext line goal\nwrapped` | `next line goal wrapped` | same |
| `**Goal**: ten\n##### deep heading\nmore` | `ten ##### deep heading more` | `ten` (divergence C) |
| `**Goal**:\n**Depends on**: Phase 1` | `**Depends on**: Phase 1` | no goal (divergence D) |
| `**Goal**:   \n\nafter blank` | `after blank` | no goal (divergence D) |

Divergence C (INFERRED): a `#####`/`######` line ends the goal because in this reader every
heading of any level already ends the entry (`any_heading_re`, one boundary for goals, phase list
and sections); upstream's `#{1,4}` stop folds the heading text into the goal, a display artifact.

Divergence D (INFERRED): upstream's `\s*` in the label regex crosses the newline when the label
line is empty, so it borrows the next non-blank line even across a blank line or when that line
is a different field (row above: the goal becomes `**Depends on**: Phase 1`). The Rust port is
line-based: continuation always starts on the line after the label line and the SAME boundary
rules apply to it, which reproduces upstream whenever that line is plain prose (the
`next line goal wrapped` row) and refuses the two artifact cases. The existing test asserting
`**Goal**:   ` (nothing after) yields no goal keeps passing.
</oracle_tables>

<tasks>

<task type="tracer" tdd="true">
  <name>Task 1: Letter-suffixed phase ids end-to-end — PHASE_ID, roadmap plan checklist, worktree plan-id rule</name>
  <files>src/state_reader/roadmap_md.rs, src/agents/worktrees.rs, src/state_reader/mod.rs</files>
  <read_first>
    - src/state_reader/roadmap_md.rs lines 40-112 (PHASE_ID, extract_phase_id, is_sentinel_phase), 194-240 (recognisers), 350-360 and 836-840 (the two plan-checklist regexes), 722-740 (phase_heading_re)
    - src/agents/worktrees.rs lines 228-250 (plan_branch_re, plan_id_re, valid_plan_id), 308-324 (parse_agent_branch), 367-403 (read_ledger_plan), 680-710 (agent_branch_grammar test)
    - src/state_reader/mod.rs lines 240-253 (roadmap_phase), 1430-1447 (make_planning), 1189-1205 (a parse_project_state test to copy the shape of)
    - src/state_reader/phase_num.rs module docs (why PhaseNum stays numeric-only)
  </read_first>
  <behavior>
    - `### Phase 23A.1.2: Letter` and `- [ ] **Phase 12A: Twelve** - d` and `- [x] Phase 12A.1: Sub (1/1 plans)` each yield a RoadmapPhase whose number is the full id (`23A.1.2`, `12A`, `12A.1`)
    - `- [x] 23A.1.2-01-PLAN.md` under the `23A.1.2` detail heading counts as a completed plan (total 1, completed 1)
    - `extract_phase_id("23A.1.2-some-slug")` is `Some("23A.1.2")`; `extract_phase_id("Phase 12A")` is `Some("12A")`; `phase_section(content, "23A.1.2")` finds the entry
    - parse_project_state over a temp `.planning/` whose ROADMAP carries the `23A.1.2` checklist line + detail entry (single-line `**Goal**: letter-suffixed goal`, `**Depends on**: Phase 12A`, one ticked plan item) returns `roadmap_phase("23A.1.2")` with name `Letter`, total_plans 1, completed_plans 1, depends_on `["12A"]`, and `phase_goals["23A.1.2"]` raw text `letter-suffixed goal`
    - `parse_agent_branch("worktree-agent-p12A-01-1790386422")` → plan `12A-01`; `agent-p23A.1.2-03-1790386422` → `23A.1.2-03`; `agent-p07.1.2-1790386422` → `07.1.2`
    - still `None`: `agent-p13x-1790386422` (existing assertion, unchanged), `agent-p12a-01-1790386422`, `agent-p12AB-01-1790386422`, `agent-pA12-01-1790386422`, `agent-p12A..1-01-1790386422`, `agent-p../x-1790386422`
    - `valid_plan_id`: true for `12A`, `12A-01`, `23A.1.2-03`, `07.1-02`, `13`; false for `12a-01`, `A12-01`, `12A.`, `12A..1`, `12A-01-slug`
  </behavior>
  <action>
    Write the tests in the behavior block FIRST and watch them fail (RED), then implement (GREEN). Test names must contain the substring `letter_suffixed` so the verify filter selects them: `letter_suffixed_phase_ids_parse_in_every_recognizer` (roadmap_md.rs test module), `letter_suffixed_plan_ids_parse_from_agent_branches` (worktrees.rs test module), `letter_suffixed_phase_reads_end_to_end` (state_reader/mod.rs test module, built with the existing `make_planning` helper and `parse_project_state`; add a minimal `STATE.md` like the neighbouring tests if the reader needs one).

    roadmap_md.rs `PHASE_ID` (line 49): append an optional dotted-sub-phase tail after the optional trailing letter, giving the token `(?:[A-Za-z]{1,4}-)?[0-9][0-9.]*[A-Za-z]?(?:\.[0-9]+)*`. This is a strict superset of upstream PHASE_NUMBER_TOKEN_SOURCE (`\d+[A-Z]?(?:\.\d+)*`, phase-id.cts:65, #2128/#4830): for letter-free input the new tail can add nothing the greedy `[0-9.]*` body has not already consumed, so every existing decimal/padded/prefixed/sentinel (`999.x`) match is unchanged; only a letter followed by `.N` segments newly matches. Keep the prefix and the `[A-Za-z]` case-flexibility. Update the doc comment's bullet list with the letter-suffixed form (`12A`, `23A.1.2`) citing the upstream constant. Every recogniser, `extract_phase_id`, `build_heading_re`, `parse_build_depends_on` and `roadmap_milestones` pick the widened token up through the shared constant — do not re-derive the grammar anywhere.

    roadmap_md.rs plan-checklist regexes — the `PLAN` static in parse_phases_in_region (~line 356) and the `plan_re` local in parse_planned_build_phases (~line 838): widen the phase part of the optional `NN-NN-` prefix from `\d+(?:\.\d+)*` to `\d+[A-Za-z]?(?:\.\d+)*`, so a newly recognised letter phase's `12A-01-PLAN.md` items count. INFERRED (the spec names PHASE_ID and plan_id_re only; without this a letter phase the reader now recognises would always report zero plans).

    worktrees.rs `plan_id_re` (line 241): change `^[0-9]+(\.[0-9]+)?(-[0-9]+)?$` to `^[0-9]+[A-Z]?(\.[0-9]+)*(-[0-9]+)?$` — one optional UPPERCASE letter, then any number of `.N` segments (upstream allows `23A.1.2`). INFERRED: uppercase only, because GSD writes the canonical uppercase suffix (init.cts normalizePhaseNumber uppercases), the existing pinned rejection of `agent-p13x-…` stays true, and the narrower rule is the conservative one for an id that feeds attribution. The id still cannot contain `/`, and `..` is impossible because every dot must be followed by digits — say so in the doc comment. Update the doc comments of `plan_id_re` (examples `13`, `13-02`, `07.1`, `07.1-02`, `12A-01`, `23A.1.2-03`) and `valid_plan_id` (quote the new pattern). `plan_branch_re` already captures letters and dots; leave it.

    Do NOT touch src/agents/waves.rs (`plan_id_re`, `PlanRef`) or src/state_reader/phase_num.rs (`PhaseNum`, `phase_key`). `PlanRef::from_id` resolves through `disk_status::plan_index` → `PhaseNum::parse`, which is numeric-only BY DESIGN (phase_num.rs module docs: a letter id "has no defined place in a numeric order"), so widening waves.rs's regex would change nothing observable, and giving letter ids an order is a separate change. Record in the SUMMARY as a follow-up: a letter-suffixed agent row now keeps its `branch_plan`/`ledger_plan` but stays unattributed until PhaseNum learns letter ordering.
  </action>
  <verify>
    <automated>rtk proxy cargo test --lib letter_suffixed</automated>
    <automated>rtk proxy cargo test --lib state_reader::roadmap_md</automated>
    <automated>rtk proxy cargo test --lib agents::</automated>
    <automated>git diff --quiet 228a063b9b0a74f1a46b365ff71f73240a671c11 -- src/agents/waves.rs src/state_reader/phase_num.rs</automated>
  </verify>
  <acceptance_criteria>
    - The `letter_suffixed` filter runs exactly 3 tests and all pass (read the raw `test result:` line — do not pipe it into grep).
    - Every pre-existing test in `state_reader::roadmap_md` and `agents::` passes with its body unedited.
    - waves.rs and phase_num.rs are byte-identical to the batch base revision 228a063 (BATCH.json `base_revision`; 260926-gtk touches neither), committed or not — the pinned `git diff --quiet` exits 0.
  </acceptance_criteria>
  <done>A letter-suffixed phase reads end-to-end through parse_project_state (phase row, plan counts, dependency, goal key), Codex branches and ledger ids with letter-suffixed plans pass the plan-id rule, and nothing numeric changed.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Port PHASE_DEP_REF (#4764) into parse_depends_on — lists, ranges as endpoints, case-insensitive, self-reference dropped</name>
  <files>src/state_reader/roadmap_md.rs</files>
  <read_first>
    - src/state_reader/roadmap_md.rs lines 114-145 (parse_depends_on and its doc), 350-412 (parse_phases_in_region, the only call site), 761-823 (parse_build_depends_on doc that references the plural-range test), 1651-1758 (the existing depends_on tests)
    - this plan's upstream_reference (PHASE_DEP_REF_SOURCE, init.cts consumer, normalizePhaseNumber) and oracle_tables (dependency rows)
    - src/driver/router.rs lines 746-800 (unsatisfied_dependency, reaches — the consumer; do not edit)
  </read_first>
  <behavior>
    - Every dependency row of the oracle table yields exactly its "Rust (this plan)" column, in that order
    - Existing tests keep passing unedited: several phases in order written, the parenthetical-qualifier test (16, 17, 19 and the `Nothing (no v2.0 …)` empty case), absent line → empty, `Phase 0.3, Phase M-2, Phase AB-29` → 0.3, M-2, AB-29, and the read-from-this-repository test (20 → 16, 17, 19; 16 → 15; 17 → 15, 16; 15 → empty)
    - The one existing test whose assertion the spec flips — `test_depends_on_accepts_every_phase_id_form_and_rejects_a_plural_range` — is renamed to `test_depends_on_accepts_every_phase_id_form_and_reads_a_plural_range_as_its_endpoints` and now asserts `Phase 15 only — parallel-eligible with Phases 17-21` (row 20) → 15, 17, 21; its message cites #4764 endpoints-only
  </behavior>
  <action>
    RED first: add tests whose names start with `depends_on_` (so the `depends_on` filter selects them together with the existing `test_depends_on_*` tests): `depends_on_matches_gsd_1_15_dep_phases` (a table test over every oracle dependency row except the two divergence rows; build each entry with a small helper that writes `### Phase {row}: X` followed by a blank line and `**Depends on**: {text}`, parses it with parse_roadmap_phases, and returns the depends_on of the phase whose number equals `row`), `depends_on_parenthetical_qualifier_stays_stripped_unlike_gsd_1_15` (divergence A, row 20 real line → 16, 17, 19; the assertion message names T-20-16 and the measured 20↔22 cycle), `depends_on_keeps_project_code_prefixed_refs_unlike_gsd_1_15` (divergence B, row 31), and `depends_on_reads_negation_prose_like_gsd_1_15` (sentriq row 12 → 9, 11; the message quotes upstream's "Negation prose … is NOT detected" stance). Rename and flip the plural-range test as the behavior block says. Watch them fail, then implement.

    Change the signature to `fn parse_depends_on(text: &str, own_number: &str) -> Vec<String>` and pass the recognised entry's number from parse_phases_in_region (`parse_depends_on(&dep_caps[1], &phase.number)`). Move its regexes into `OnceLock` statics (today it compiles two regexes per call). Keep step 1 exactly: replace every `\([^)]*\)` group with a space before extraction (T-20-16, divergence A). Step 2 is the upstream grammar with the Rust token: build the reference regex as case-insensitive `\bphases?\s+(ID(?:SEP ID)*)` where ID is `PHASE_ID` and SEP is upstream's alternation verbatim — `\s*,\s*(?:and\s+)?` | `\s+and\s+` | `\s*&\s*` | `\s+(?:to|through)\s+` | `\s*-\s*` (hyphen only; en/em dashes are NOT separators upstream). For each match, scan capture group 1 left to right with an unanchored `PHASE_ID` regex (upstream's depTokenRe); for each token trim trailing `.` and `,` (the existing trim — `PHASE_ID`'s `[0-9.]*` body can swallow a sentence-final dot, e.g. `Phase 19.`), skip it if empty, skip it if its normalised key equals the normalised key of `own_number` (#4764 self-reference), skip it if the key was already seen, otherwise push the token as written. A range therefore contributes its two endpoints only, as upstream does.

    Normalised key: add a private helper (e.g. `dep_ref_key`) that ports init.cts `normalizePhaseNumber` exactly — split on `.`; a segment matching `^(\d+)([A-Za-z]?)$` becomes its digits with leading zeros stripped (an all-zero run becomes `0`; strip, do not parse, so an absurdly long digit run cannot overflow) followed by the letter uppercased; any other segment (e.g. `M-2`) is kept verbatim; rejoin with `.`. Use it for both the self-skip and the dedupe, so `Phase 8, Phase 08` yields `8` once and `phase 12a` is recognised as row `12A` itself. Do NOT route this through `phase_key` (it keeps letter ids raw and case-sensitive, and changing it would move every other caller).

    Rewrite parse_depends_on's doc comment: rule 1 (parenthetical strip, T-20-16, with divergence A and the measured 1.15.0 numbers for this repo's phases 20/22/24/25 recorded as the reason it is kept); rule 2 (the #4764 grammar: plural/singular keyword, list joins, range endpoints only, case-insensitive, self-reference dropped, normalised dedupe keeping first-written spelling; the Rust token additionally reads project-code prefixes — divergence B; negation prose is read, as upstream reads it). Delete the sentence claiming a plural range does not match. Update parse_build_depends_on's doc (lines ~762-763) so it no longer says the GSD grammar ignores plural ranges: the difference is now that the GSD grammar reads range ENDPOINTS (upstream parity) while the build-phase grammar EXPANDS a range against the known ids. Leave RoadmapPhase::depends_on's doc accurate (add that the entry's own phase is never listed). Do not edit router.rs.

    Known pre-existing scan boundary, not changed here: an entry's dependency scan runs to the next phase ENTRY line, not the next heading, so a GSD phase entry with no own `**Depends on**:` line directly followed by `#### Build phase` entries would take a build entry's line; with the case-insensitive keyword such a line (`Build phases 8-13`) now yields endpoints where it yielded nothing. No fixture or real roadmap has that layout (ttbook's last GSD entry, Phase 13, declares its own line) — mention it in the SUMMARY, do not fix it here.
  </action>
  <verify>
    <automated>rtk proxy cargo test --lib depends_on</automated>
    <automated>rtk proxy cargo test --lib state_reader::roadmap_md</automated>
    <automated>rtk proxy cargo test --lib driver::router</automated>
  </verify>
  <acceptance_criteria>
    - `grep -c 'fn parse_depends_on(text: &str, own_number: &str)' src/state_reader/roadmap_md.rs` prints 1.
    - The `depends_on` filter runs the 4 new tests plus the existing `test_depends_on_*` tests (one of them renamed) and all pass.
    - `grep -c 'reads_a_plural_range_as_its_endpoints' src/state_reader/roadmap_md.rs` prints 1.
    - Router unit tests pass unchanged.
  </acceptance_criteria>
  <done>parse_depends_on reproduces the 1.15.0 oracle's dep_phases for every non-parenthetical oracle row, the two divergences are pinned by named tests, and the doc comments say what the parser now does.</done>
</task>

<task type="auto" tdd="true">
  <name>Task 3: Port extractPhaseFieldMultiline (#4731/#4837) into parse_phase_goals — hard-wrapped goals to upstream's boundaries</name>
  <files>src/state_reader/roadmap_md.rs, src/state_reader/mod.rs</files>
  <read_first>
    - src/state_reader/roadmap_md.rs lines 715-760 (any_heading_re, phase_heading_re, is_fence_line), 904-948 (parse_phase_goals), 2455-2515 (existing goal tests and the goal_of helper)
    - src/state_reader/mod.rs lines 84-95 (the phase_goals field doc)
    - this plan's upstream_reference (extractPhaseFieldMultiline) and oracle_tables (goal rows, divergences C and D)
  </read_first>
  <behavior>
    - Every goal row of the oracle table yields exactly its "Rust (this plan)" column (a "no goal" row means the entry's key is absent from the map)
    - Existing goal tests keep passing unedited: both bold forms and lowercase `**goal**:`, first Goal in an entry wins and `### Phase 07:` keys as `7`, `**Goal**:   ` alone yields no goal, a Goal after a non-phase heading is not attributed upward, a build-phase goal is read, every fixture's detailed phases have a goal starting `(sanitised)`
  </behavior>
  <action>
    RED first: add `a_hard_wrapped_goal_reads_past_the_line_break` (the 999.4 row, verbatim), `a_goal_stops_at_the_gsd_1_15_boundaries` (one assertion per remaining parity row, including the `*`/`+` list variants, the `~~~` fence variant, the same-line fence → no goal, the colon-less label, the lowercase bold label, the blank line, the numbered-item/`Plans:` fold, the empty-label-line → next-line row and the `23A.1.2` key), and `goal_divergences_from_gsd_1_15_are_pinned` (divergences C and D; each assertion message states the upstream value from the table). Use a helper that builds `### Phase 1: T`, a blank line and the body, so each case is its own roadmap string. Watch them fail, then implement.

    Label regex (the `GOAL` static): port upstream's labelRe for label `Goal` as the single-line, case-insensitive `^[ \t]*\*\*Goal(?::\*\*|\*\*[ \t]*:?)[ \t]*(.*)$` — the colon after a closing `**` becomes optional (upstream accepts `**Goal** text`); `**Goal:**`, `**Goal**:` and `**Goal** :` keep working.

    Continuation: add a private predicate (e.g. `ends_field_continuation(line)`) that is true for upstream's boundary set, checked on the raw line: blank after trim; `^\s*[-*+]\s` (list item — numbered items are deliberately NOT boundaries, as upstream); `^\s*\*\*[A-Za-z][A-Za-z ]*:?(?:\*\*)?:?\s` (any `**Label**`/`**Label:**` line, either case); `any_heading_re()` (divergence C — any level ends it, same as the entry boundary); `^\s*\|` (table row); `^\s*(?:`{3,}|~{3,})` (fence opener). Put the regexes in `OnceLock` statics like the rest of the file.

    parse_phase_goals loop: keep the existing entry model (a phase or build-phase heading opens a key, any heading closes it, the first Goal line in an entry wins and closes the entry for further Goal lines). When the label matches: take the captured text trimmed as the first part; if it starts with a fence (``` or ~~~, upstream's `^(?:`{3,}|~{3,})` on the trimmed first line) record NO goal for the entry (upstream returns null) and stop; otherwise hold a pending goal (key + parts). On each following line, while a goal is pending: if the line ends the continuation, finalise the pending goal and then let the line go through the normal loop (so a heading still opens/closes an entry); otherwise push the line trimmed and move to the next line. Finalise also at end of input. Finalising joins the parts with single spaces and trims; an empty result is skipped (existing rule), a non-empty one is inserted with `entry(key).or_insert_with(Untrusted::from_untrusted_source(..))` as today. Continuation always starts on the line after the label line, even when the label line's own text is empty (divergence D). CRLF input is already handled by `str::lines()`.

    Doc comments: rewrite parse_phase_goals's doc — remove the sentence that restricts it to one-line goals and claims every roadmap seen writes the goal on one line (false: this repo's own 999.4 wraps), describe the boundary set, cite upstream #4731/#4837 and the two divergences, and keep the Untrusted paragraph. Update state_reader/mod.rs's `phase_goals` field doc (lines ~85-86) from "`**Goal**:` line" to the goal field including hard-wrapped continuation lines. The joined text stays `Untrusted` and reaches a cell only through `shown()` — joining with spaces means no raw newline from the roadmap enters a cell.

    Docs: grep README.md and docs/*.md for any statement of the goal or depends-on grammar. None is expected (README only says the Roadmap detail pane shows each phase's goal, needs and unblocks, which stays true); if none, record "no doc change needed — no doc states either grammar" in the SUMMARY rather than inventing a doc section.
  </action>
  <verify>
    <automated>rtk proxy cargo test --lib goal</automated>
    <automated>rtk proxy cargo test --lib state_reader::</automated>
  </verify>
  <acceptance_criteria>
    - The `goal` filter includes the 3 new tests and every pre-existing `*goal*` test in roadmap_md.rs; all pass.
    - `grep -c 'Single-line goals only' src/state_reader/roadmap_md.rs` prints 0 after the doc rewrite (the phrase exists only in the current doc comment, and nothing in this plan's instructions asks for it to be written).
    - All `state_reader::` lib tests pass.
  </acceptance_criteria>
  <done>A hard-wrapped goal is read in full exactly where GSD 1.15.0 reads it, the two deliberate divergences are pinned, and the docs describe the multi-line behaviour.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| project ROADMAP.md → roadmap parser | Project-authored prose (untrusted) becomes phase ids, dependency lists and goal text |
| dependency list → driver router | `RoadmapPhase::depends_on` gates the router's execute/discuss choice (fail-closed on unknown ids) |
| goal text → terminal | Joined goal text is rendered in Roadmap tab cells |
| agent branch names / git admin-dir ledger names → plan-id rule | Agent-written strings become plan ids used for attribution |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-gtl-01 | Elevation / Tampering | parse_depends_on | medium | mitigate | Parenthetical qualifiers stay stripped before extraction (T-20-16 retained, pinned by `depends_on_parenthetical_qualifier_stays_stripped_unlike_gsd_1_15`), only `phase`/`phases`-anchored tokens are read, and the router's fail-closed treatment of undeclared ids is untouched — prose cannot drop a real `Phase N` reference, and cannot invent one from a date, sha or version string (`2026-09-14`, `8bf403100d`, `v2.0` pinned by the oracle-table test). |
| T-gtl-02 | Denial of Service | new regexes (dep-ref, goal label, boundary set) | low | accept | The `regex` crate guarantees linear-time matching (no backtracking), all inputs are bounded by the file already read; no new unbounded loop — continuation ends at the entry's next heading or end of input. |
| T-gtl-03 | Tampering | worktrees.rs plan_id_re | low | mitigate | Anchored `^[0-9]+[A-Z]?(\.[0-9]+)*(-[0-9]+)?$`: no `/`, no `..` (each dot needs digits), no lowercase; `letter_suffixed_plan_ids_parse_from_agent_branches` asserts the traversal (`../x`, `12A..1`) and shape-confusion cases stay rejected, and the id still never becomes a path. |
| T-gtl-04 | Spoofing / Information disclosure (terminal injection) | parse_phase_goals joined text | medium | mitigate | Goals remain `crate::text::Untrusted` via `from_untrusted_source` and reach cells only through `shown()` (render_escape_guard unchanged); continuation lines are trimmed and joined with single spaces, so no roadmap newline reaches a cell. |
| T-gtl-05 | Tampering | self-reference skip / dedupe key | low | mitigate | The normalised key is a pure string transform (leading zeros stripped, not parsed — no overflow), used only to drop the row's own id and duplicates; it never widens what counts as a reference. |
| T-gtl-SC | Tampering | dependencies | low | accept | No crate is added or bumped by this plan; no package-manager install occurs, so the package-legitimacy gate does not apply. |
</threat_model>

<verification>
Run in this order on the primary checkout after all three tasks. Read the RAW result lines; do not
pipe cargo output into grep/awk (rtk filters downstream of `rtk proxy`, and a piped count can pass
vacuously).

1. Router conformance against the installed oracle (~/.claude/gsd-core, currently 1.14.0) — must
   report `the_rust_rule_table_agrees_with_gsd_s_own_router_over_a_fixture_per_state ... ok`, not a
   SKIP line:
   `rtk proxy cargo test --test driver_router_conformance`
2. Router conformance against the 1.15.0 oracle, ONLY if
   `/tmp/claude-1000/-home-blk-projects-rust-gsd-meta-manager/9e776af9-6a4a-4326-9fd5-76d0bedeccfe/scratchpad/fakehome/.claude/gsd-core/bin/gsd-tools.cjs`
   still exists (grounded 2026-09-26 on the pre-change tree: 3 passed):
   `HOME=/tmp/claude-1000/-home-blk-projects-rust-gsd-meta-manager/9e776af9-6a4a-4326-9fd5-76d0bedeccfe/scratchpad/fakehome CARGO_HOME=/home/blk/.cargo RUSTUP_HOME=/home/blk/.rustup rtk proxy cargo test --test driver_router_conformance`
   If the path is gone, say so in the SUMMARY; step 1 is the required gate.
3. Full suite, every binary: `rtk proxy cargo test --no-fail-fast` — the ONLY permitted failure is
   `the_config_section_constants_record_the_git_version_they_were_derived_against`
   (src/envelope/policy.rs, the git-version witness; expected on this machine). Any other failure
   blocks. Record the per-suite totals in the SUMMARY.
4. `rtk proxy cargo clippy --all-targets -- -D warnings` exits 0.
</verification>

<success_criteria>
- Every row of both oracle tables is pinned by a test and passes with the "Rust (this plan)" value.
- Divergences A-D are each pinned by a named test and each listed in the SUMMARY as INFERRED with its measured upstream value.
- No existing test body edited except the one renamed plural-range test.
- `git diff --quiet 228a063b9b0a74f1a46b365ff71f73240a671c11 -- src/agents/waves.rs src/state_reader/phase_num.rs src/driver/router.rs` exits 0, and this plan's commits do not touch docs/GSD-CORE-SYNC.md (owned by 260926-gtk).
- Gates 1, 3, 4 green as defined above; gate 2 green or reported as unavailable.
</success_criteria>

<output>
Create `.planning/quick/260926-gtl-roadmap-parser-parity-with-gsd-core-1-15-0-port-phase-dep-re/260926-gtl-SUMMARY.md` when done. Include: the three upstream issues ported with their source lines; an "Inferred decisions" list (uppercase-only plan-id letter; plan-checklist regex widening; divergences A, B, C, D with measured upstream values); follow-ups (PhaseNum/PlanRef letter ordering so letter-suffixed agent rows attribute; the pre-existing depends-scan boundary); the doc-change finding; and the raw gate results.
</output>
