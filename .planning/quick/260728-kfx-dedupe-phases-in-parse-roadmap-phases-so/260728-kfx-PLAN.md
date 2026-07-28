---
phase: quick-260728-kfx
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/state_reader/roadmap_md.rs
autonomous: true
requirements: [QUICK-260728-kfx]

must_haves:
  truths:
    - "A roadmap that carries both a summary checklist block and a `## Phase Details` section yields exactly one RoadmapPhase per distinct phase number, not two."
    - "The merged phase keeps the description carried by the checklist form (the heading form has none)."
    - "The merged phase is marked completed if ANY duplicate copy carried a checked box."
    - "The merged phase reports the plan counts from whichever copy actually scanned the plan list (the larger counts win)."
    - "First-seen phase-number order is preserved, so the Phases pane ordering is unchanged."
    - "Roadmaps with only one form per phase (all existing tests) parse byte-identically to before."
  artifacts:
    - "src/state_reader/roadmap_md.rs — dedupe/merge step at the tail of parse_roadmap_phases"
    - "src/state_reader/roadmap_md.rs — regression tests covering checklist-then-details and details-then-checklist layouts"
  key_links:
    - "parse_roadmap_phases return value -> src/state_reader/mod.rs:180 (state.phases) -> TUI Phases pane list"
    - "state.phases -> src/state_reader/mod.rs:191 per-phase disk_status::infer_phase_status loop (duplicates caused redundant inference against the same phase number)"
---

<objective>
Deduplicate the phase list produced by `parse_roadmap_phases` so a standard GSD 1.8.0
ROADMAP.md — which carries a summary checklist near the top AND a `## Phase Details`
section with one `### Phase N: Title` heading per phase — produces one entry per phase
instead of two.

Purpose: the TUI Phases pane currently lists every phase twice for any roadmap using the
standard two-section layout (reproduced against `/home/blk/projects/flutter/sentriq/.planning/ROADMAP.md`,
phases 4-8 each appearing twice). The two copies also disagree on plan counts: the
checklist copy stops scanning immediately (next line is another header) and reports
`total_plans: 0`, while the detail-heading copy captures the real plan items.

Output: a merged, order-preserving phase list from `parse_roadmap_phases`, plus regression
tests locking in the real-world layout.
</objective>

<execution_context>
@$HOME/.claude/gsd-core/workflows/execute-plan.md
@$HOME/.claude/gsd-core/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@CLAUDE.md
@src/state_reader/roadmap_md.rs
@src/state_reader/mod.rs

Interface facts the executor needs:

- `pub struct RoadmapPhase { number: String, name: String, description: String, completed: bool, total_plans: u32, completed_plans: u32 }` — derives `Debug, Clone, PartialEq`. Do NOT change these fields or derives.
- `pub fn parse_roadmap_phases(content: &str) -> Vec<RoadmapPhase>` — public signature MUST stay exactly as-is.
- Inside the function, matched phases accumulate into a local `let mut phases: Vec<RoadmapPhase> = Vec::new();` and the function ends with a bare `phases` expression (roadmap_md.rs:150).
- Sentinel filtering (`is_sentinel_phase`) and strikethrough filtering already run at match time, before a phase is pushed. Leave both untouched.
- The only caller is `src/state_reader/mod.rs:180`, which assigns the result to `state.phases` and then iterates it to build `state.phase_disk_statuses` (a `HashMap` keyed by `phase.number`).
- The module currently imports only `use regex::Regex;`.
</context>

<tasks>

<task type="tracer" tdd="true">
  <name>Task 1: Merge duplicate phase numbers at the tail of parse_roadmap_phases</name>
  <files>src/state_reader/roadmap_md.rs</files>
  <behavior>
    Write this test FIRST and confirm it fails (it currently returns 6 phases, not 3),
    then implement until it passes. Name it `test_parse_roadmap_dedupes_summary_and_details`.

    Fixture reproduces the real sentriq layout — a summary checklist block, then a
    `## Phase Details` section with `### Phase N: Title` headings and plan items beneath:
    - `- [x] **Phase 4: Visualization** - ASCII roadmap rendering`
    - `- [ ] **Phase 5: Queue** - Batch execution`
    - `- [ ] **Phase 6: Sessions** - tmux attach`
    - then `## Phase Details`
    - then `### Phase 4: Visualization` followed by two checked PLAN.md items
    - then `### Phase 5: Queue` followed by one checked and one unchecked PLAN.md item
    - then `### Phase 6: Sessions` followed by no plan items

    Assertions:
    - `phases.len() == 3` (the bug produces 6)
    - numbers in first-seen order are `["4", "5", "6"]`
    - `phases[0].description == "ASCII roadmap rendering"` (survives from the checklist form; the heading form carries an empty description)
    - `phases[0].completed` is true and `phases[1].completed` is false (checkbox survives the merge; the heading form always reports false)
    - `phases[0].total_plans == 2` and `phases[0].completed_plans == 2` (counts come from the detail section, where the plan items live)
    - `phases[1].total_plans == 2` and `phases[1].completed_plans == 1`
    - `phases[2].total_plans == 0`
    - the literal `## Phase Details` heading itself does not become a phase (implied by the count assertion, but assert no phase name equals `Details`)
  </behavior>
  <action>
    Add `use std::collections::HashMap;` alongside the existing `use regex::Regex;` import.

    Add a private, pure helper `fn merge_duplicate_phases(phases: Vec<RoadmapPhase>) -> Vec<RoadmapPhase>`
    directly below `parse_roadmap_phases`. It walks the input in order, keeping a
    `HashMap<String, usize>` from phase `number` to the index of that number's entry in
    an output `Vec<RoadmapPhase>`. On a first sighting of a number, push the phase and
    record its index. On a repeat sighting, merge into the already-pushed entry:
    - `completed` becomes the logical OR of the existing value and the incoming value
    - `name` and `description` are overwritten by the incoming value ONLY when the existing field is empty (first non-empty value wins)
    - `total_plans` and `completed_plans` each take the max of existing and incoming

    Change the final expression of `parse_roadmap_phases` from the bare `phases` to
    `merge_duplicate_phases(phases)`. Make no other change to `parse_roadmap_phases`: the
    public signature, the two regex recognizers, the plan-item scan loop, the sentinel
    filter, and the strikethrough filter all stay exactly as they are.

    Do not modify `RoadmapPhase`, `RoadmapProgress`, `roadmap_progress`, or anything in
    `src/state_reader/mod.rs`. Matching by `number` alone is deliberate — a phase number is
    the identity key the caller already relies on when building `phase_disk_statuses`.

    Give `merge_duplicate_phases` a doc comment stating the merge rule per field and why
    it exists (standard roadmaps describe each phase twice: once in the summary checklist,
    once under `## Phase Details`).
  </action>
  <verify>
    <automated>cargo test --lib roadmap_md 2>&1 | tail -25</automated>
  </verify>
  <done>`test_parse_roadmap_dedupes_summary_and_details` passes, and every pre-existing test in the `roadmap_md` module still passes — notably `test_parse_roadmap_heading_levels_and_plans` (distinct numbers 2 and 3, must remain 2 phases) and `test_parse_roadmap_details_wrapped` (distinct numbers 1 and 2, must remain 2 phases with plan counts 2/2 on phase 1).</done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Cover reversed section order and lock the full build gate</name>
  <files>src/state_reader/roadmap_md.rs</files>
  <behavior>
    Add `test_parse_roadmap_dedupes_details_before_checklist`, covering the mirror layout
    where the `### Phase N:` detail heading (with its plan items) appears BEFORE the
    summary checklist entry for the same phase:
    - `### Phase 2: Dashboard` followed by one checked and one unchecked PLAN.md item
    - then `- [x] **Phase 2: Dashboard** - Main project list`

    Assertions:
    - `phases.len() == 1`
    - `phases[0].number == "2"`
    - `phases[0].completed` is true (the OR rule picks up the later checkbox)
    - `phases[0].description == "Main project list"` (the first-seen copy had an empty description, so the later non-empty one fills it)
    - `phases[0].total_plans == 2` and `phases[0].completed_plans == 1` (the max rule keeps the counts from the earlier copy that actually scanned the plan list)
  </behavior>
  <action>
    Add the test above to the existing `mod tests` block in `src/state_reader/roadmap_md.rs`,
    placed next to the Task 1 regression test. Use the same raw-string fixture style as the
    surrounding tests.

    Then run the full project gate and fix anything it surfaces. Expect clippy to be quiet;
    if it flags the index-map pattern, prefer restructuring the helper over adding an
    `#[allow]` attribute.
  </action>
  <verify>
    <automated>cargo build 2>&1 | tail -5 &amp;&amp; cargo test 2>&1 | tail -25 &amp;&amp; cargo clippy -- -D warnings 2>&1 | tail -15</automated>
  </verify>
  <done>Both new tests pass, the whole suite is green, `cargo clippy -- -D warnings` exits 0, and `cargo build` succeeds.</done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| filesystem -> parser | ROADMAP.md content from an arbitrary registered project directory is untrusted input to `parse_roadmap_phases` |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-kfx-01 | Denial of Service | `merge_duplicate_phases` | low | mitigate | Merge is a single O(n) pass over an already-bounded Vec with a HashMap index; no nested scan over `phases`, so a pathological roadmap with thousands of repeated headings stays linear |
| T-kfx-02 | Tampering | `RoadmapPhase` field merge | low | mitigate | Merge only ever combines values already produced by the existing recognizers; no new parsing, no new field sources, and the sentinel/strikethrough filters still run before any phase enters the merge |
| T-kfx-03 | Information Disclosure | phase `name` / `description` | low | accept | Values are rendered in the same TUI pane as before; the merge changes cardinality, not what is displayed |

No package-manager installs in this task, so no supply-chain (`T-kfx-SC`) entry is required.
</threat_model>

<verification>
- `cargo build` succeeds
- `cargo test` — full suite green, including all pre-existing `roadmap_md` tests
- `cargo clippy -- -D warnings` — exits 0
- Public API unchanged: `parse_roadmap_phases(&str) -> Vec<RoadmapPhase>` and the `RoadmapPhase` field set are identical to before
- `src/state_reader/mod.rs` is untouched
</verification>

<success_criteria>
- A roadmap with a summary checklist plus a `## Phase Details` section returns one entry per distinct phase number
- Descriptions and completion checkboxes survive from the checklist form; plan counts survive from whichever form scanned the plan list
- First-seen phase ordering is preserved
- All previously passing tests still pass with no edits to their assertions
- Build, test, and clippy gates all clean
</success_criteria>

<output>
Create `.planning/quick/260728-kfx-dedupe-phases-in-parse-roadmap-phases-so/260728-kfx-SUMMARY.md` when done
</output>
