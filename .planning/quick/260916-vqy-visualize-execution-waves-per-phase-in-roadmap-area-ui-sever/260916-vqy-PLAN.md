---
phase: quick-260916-vqy
plan: 01
type: execute
wave: 2
depends_on:
  - 260916-vqx
files_modified:
  - src/state_reader/plan_waves.rs
  - src/state_reader/mod.rs
  - src/state_reader/disk_status.rs
  - src/ui/screens/detail.rs
autonomous: true
requirements:
  - TODO-2026-09-11-visualize-execution-waves-per-phase-in-roadmap

estimate:
  tokens: 110000
  raw_tokens: 55000
  tasks: 3
  confidence: low

must_haves:
  truths:
    - "Selecting a phase in the Pipeline tab shows that phase's plans grouped by execution wave, derived from PLAN.md frontmatter, with no waves.json anywhere on disk"
    - "A phase whose plans carry no wave frontmatter renders exactly as it does today: no wave section, no placeholder, no error"
    - "The Pipeline phase list marks phases that ran in more than one wave, so parallelism is visible without selecting each phase in turn"
    - "Wave labels carry the real wave numbers from frontmatter, not list positions, so a phase with waves 1 and 3 never renders as 1 and 2"
    - "Rendering the derived wave section performs no file IO at render time"
  artifacts:
    - src/state_reader/plan_waves.rs
    - "DiskInference::plan_waves field in src/state_reader/disk_status.rs"
    - "waves_manifest_from_derived and phase_list_label in src/ui/screens/detail.rs"
  key_links:
    - "disk_status.rs plan-file scan -> plan_waves::plan_wave_number, reading the content the scan already read for the superseded check (no second read)"
    - "DiskInference.plan_waves -> detail.rs render_pipeline_tab -> the existing, unmodified build_waves_lines renderer"
    - "state.phase_disk_statuses -> phase_list_label -> Pipeline tab left pane"
    - "every derived plan-id string -> shown() before it reaches a rendered Span"
---

<objective>
Make a phase's execution waves visible in the TUI by deriving the grouping from the
place it is actually recorded on disk — `wave:` in each `*-PLAN.md`'s leading
frontmatter — and feeding it to the wave renderer that already exists but is
currently unreachable.

Purpose: a user managing several GSD projects can see which plans in a phase ran
concurrently and which were serialized, without opening that project's `.planning/`
files in an editor.

Output: a new `src/state_reader/plan_waves.rs` parser, a `plan_waves` field on
`DiskInference`, a fallback source for the existing Pipeline-tab wave section, and a
per-phase wave marker in the Pipeline phase list.
</objective>

<execution_context>
@~/.claude/gsd-core/workflows/execute-plan.md
@~/.claude/gsd-core/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@CLAUDE.md
@.planning/todos/pending/2026-09-11-visualize-execution-waves-per-phase-in-roadmap.md

@src/state_reader/disk_status.rs
@src/state_reader/mod.rs
@src/ui/screens/detail.rs
</context>

<measured_findings>
These were measured during planning, not assumed. They decide the design, so an
executor that disagrees with one should re-measure it rather than reinterpret it.

1. **Wave metadata IS derivable — the todo's conditional is satisfied.** All 116
   `*-PLAN.md` files under `.planning/phases/` carry `wave:` and `depends_on:` at
   column zero of their leading frontmatter block (116 of 116, measured by
   `grep -l "^wave:"`). This is what `/gsd-execute-phase` reads to assign waves, so
   it is the authoritative source.

2. **A wave renderer already exists and is dead code in practice.**
   `src/ui/screens/detail.rs` has `WavesManifest` / `parse_waves_manifest` /
   `build_waves_lines` (around lines 4947-5030), rendered from `render_pipeline_tab`
   (the block at ~line 3875) keyed on a per-phase `waves.json`. No `waves.json`
   exists anywhere under `.planning/` in this repository, and it is written only by
   GSD 1.8.0's claude-orchestration backend. The section therefore renders for
   nobody today. Three tests already cover the parser
   (`cargo test --lib waves` lists exactly `test_parse_waves_manifest_two_waves`,
   `test_parse_waves_manifest_lenient_defaults`, `test_parse_waves_manifest_garbage_is_none`).
   This plan supplies a second source for that renderer; it does not rewrite it.

3. **The plan scan already reads every PLAN.md's content.** In
   `infer_disk_status`, the `*-PLAN.md` arm reads the file to run
   `plan_frontmatter_superseded`. Extracting `wave:` from that same in-hand
   `content` costs no additional IO, and it places the derivation in the
   refresh-cadence layer instead of the 250ms render tick.

4. **INFERRED DECISION (human unavailable — audit this): `roadmap_md.rs` and
   `.planning/ROADMAP.md` are NOT modified**, despite being the two files the todo's
   `files:` block names. `ROADMAP.md` does carry `**Wave N**` prose headings inside
   some phases' `Plans:` sections (78 matches across 13 phase entries), so a second
   source genuinely exists. It was rejected on two measured grounds: it is optional
   per phase (frontmatter is 116/116; the headings are not), and it is a
   human-authored mirror that goes stale against frontmatter when a phase is
   replanned. Frontmatter is the single source of truth here. If a later reviewer
   wants the ROADMAP headings rendered too, that is a separate change layered on
   the same renderer.

5. **INFERRED DECISION (human unavailable — audit this): dependency edges are out of
   scope.** `depends_on:` is present in every plan's frontmatter and would answer
   "why was this serialized", but rendering per-plan edges requires changing
   `build_waves_lines`'s one-line-per-wave shape. The todo's stated need is the
   grouping ("which plans ran together vs. which were serialized"), which wave
   numbers alone answer. Deferred deliberately, not overlooked.

6. **INFERRED DECISION (human unavailable — audit this): the `files_modified` count
   in the existing hint is left at zero for the derived path.** `build_waves_lines`
   appends an `Nf` hint only when the count is above zero, so the derived path
   degrades cleanly to just the plan count. Populating it would require a
   frontmatter YAML *list* reader, which is new parsing logic for a cosmetic hint.

7. **Cross-item file overlap.** Batch item 260916-vqx parses PLAN.md/SUMMARY.md
   frontmatter for token estimates and touches the phase/plan list UI, so it
   plausibly touches `src/state_reader/disk_status.rs`, `src/state_reader/mod.rs`
   and `src/ui/screens/detail.rs` — the same three existing files this plan edits.
   `depends_on: [260916-vqx]` is declared to serialize the merge, not because this
   plan consumes vqx's output.
</measured_findings>

<tasks>

<task type="tracer">
  <name>Task 1: End-to-end "a phase's waves render from PLAN.md frontmatter" — one path only</name>
  <files>src/state_reader/plan_waves.rs, src/state_reader/mod.rs, src/state_reader/disk_status.rs, src/ui/screens/detail.rs</files>
  <action>
Wire ONE path from a plan file's frontmatter through to a rendered wave line, touching
every layer, with no other call sites and no batching.

Create `src/state_reader/plan_waves.rs` and declare it in `src/state_reader/mod.rs`
alongside the other `pub mod` lines. It exports:

- `PlanWave`, a struct with `pub wave: Option<u32>` and `pub plans: Vec<String>`,
  deriving Debug, Clone, PartialEq and Eq. Give it a `pub fn label(&self) -> String`
  returning the wave number prefixed with a lowercase w for `Some`, and a single
  lowercase w followed by a question mark for `None`. The `Some` arm must format the
  stored number — never a position — because the caller iterates with an index and
  the existing `WaveEntry::label` would otherwise substitute index-plus-one for a
  phase whose wave numbers are non-contiguous.
- `pub fn plan_wave_number(content: &str) -> Option<u32>`, which reads the `wave` key
  via `disk_status::leading_frontmatter_value` and parses it as `u32`, yielding `None`
  on any parse failure.
- `pub fn group_into_waves(entries: Vec<(String, Option<u32>)>) -> Vec<PlanWave>`,
  taking plan-id/wave-number pairs. For the tracer, handle only the case where every
  entry carries a wave number: group by that number, emit one `PlanWave` per distinct
  number in ascending numeric order, and sort the plan ids inside each wave. Task 2
  adds the remaining cases.

Do NOT copy `leading_frontmatter_value` into the new module. In
`src/state_reader/disk_status.rs`, change its visibility from private to `pub(crate)`
and leave its doc comment byte-for-byte intact — that comment records why the
byte-zero block anchor and the column-zero key match are load-bearing, and a second
divergent copy of that logic is exactly the defect it describes.

Still in `disk_status.rs`: add `pub plan_waves: Vec<PlanWave>` to `DiskInference`
(the derives already cover it once `PlanWave` derives Debug, Clone, PartialEq, and
`Vec` supplies Default). In `infer_disk_status`'s `*-PLAN.md` arm, after the existing
superseded check and inside the same `Ok(content)` binding, capture
`plan_wave_number(&content)` next to the derived plan id and push the pair onto a
local accumulator; a file that fails to read contributes a `None` wave. After the
directory loop, call `group_into_waves` and set the new field in the `DiskInference`
literal at the tail of the function. Sorting inside `group_into_waves` is what keeps
this field stable across refreshes: directory iteration order is unspecified, and
`DiskInference` derives `PartialEq`, which drives the dashboard's unchanged-state
suppression — an unsorted vector would make a project flap as Updated every refresh.

In `src/ui/screens/detail.rs`, add `fn waves_manifest_from_derived(waves: &[PlanWave])
-> WavesManifest` that maps each `PlanWave` to a `WaveEntry` whose `id` holds the
`PlanWave::label` string and whose `plans` hold one `WavePlan` per plan id, with
`files_modified` left empty. Every plan-id string must pass through the module's
existing `shown` helper before it enters the manifest, matching the 49 existing call
sites — these ids come from another project's filenames and are untrusted text.

Then restructure the `waves.json` block in `render_pipeline_tab` into a single
resolution followed by a single render: keep the existing on-disk `waves.json` read as
the preferred source, and when it is absent, unparsable, or carries no waves, fall
back to `state.phase_disk_statuses` for the selected phase's number and build the
manifest from its `plan_waves` when that vector is non-empty. Exactly one call to the
unmodified `build_waves_lines` remains. The fallback path reads no files — the data is
already in `ProjectState` — so it adds nothing to the 250ms render tick.

Prove the path with one end-to-end test in `plan_waves.rs` (or `disk_status.rs`)
whose name contains the word waves: build a temp phase directory holding two plan
files at wave 1 and wave 2 and no manifest file, run `infer_disk_status`, and assert
the resulting `plan_waves` groups them under the two numbered waves in order.
  </action>
  <verify>
    <automated>cargo test --lib waves</automated>
  </verify>
  <done>A phase directory containing two PLAN.md files whose frontmatter declares different wave numbers, and no waves.json, yields a two-entry DiskInference::plan_waves in ascending wave order, and render_pipeline_tab builds its wave section from that vector through the unmodified build_waves_lines. `cargo test --lib waves` exits 0 and reports more tests than the three that exist today.</done>
  <reversibility rating="reversible">A new module plus one struct field and one fallback branch; removing them restores current behaviour exactly.</reversibility>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Grouping edge cases and the graceful-degradation contract</name>
  <files>src/state_reader/plan_waves.rs, src/state_reader/disk_status.rs</files>
  <behavior>
    - No plan in the phase carries a wave key: `group_into_waves` returns an empty vector, so the Pipeline tab renders no wave section at all — the todo's required degradation for projects and GSD versions that do not record waves.
    - Some plans carry a wave number and some do not: the numbered waves come first in ascending order, and the unnumbered plans collect into exactly one trailing `PlanWave` whose `wave` is None and whose label is the question-mark form.
    - Wave numbers 1 and 3 with nothing at 2: two entries labelled from the stored numbers, so the second reads as wave three and not as wave two.
    - A plan whose frontmatter declares the superseded status contributes neither a plan id nor a wave number, keeping this field consistent with the existing plan_count.
    - Non-numeric, negative, empty, and absent wave values each yield None from `plan_wave_number` rather than a panic.
    - A wave key indented under another mapping key is not read as a top-level wave, and a file whose first line is not the block opener yields None.
    - Two plan ids landing in the same wave come out sorted, independent of the order the directory scan supplied them.
  </behavior>
  <action>
Write the tests first, watch them fail, then extend `group_into_waves` and
`plan_wave_number` until they pass. Every test name must contain the word waves so the
verify command's filter reaches it.

Extend `group_into_waves` to cover the two cases the tracer deferred. When no entry
carries a wave number, return an empty vector — the absence of wave metadata is
reported as nothing to draw, never as a single anonymous bucket, because the renderer
would otherwise print a heading over a flat list the user already has. When at least
one entry carries a number, emit the numbered waves in ascending order and append one
trailing `PlanWave` with `wave` set to None holding the remaining plan ids, sorted.

`plan_wave_number` needs no new anchoring logic for the nesting and block-opener
cases: `leading_frontmatter_value` already enforces both, and the tests exist to pin
that this module inherits that behaviour rather than to add it here. The numeric
cases are handled by the `u32` parse discarding its error.

For the superseded case, assert through `infer_disk_status` against a temp phase
directory rather than against `group_into_waves` — the exclusion lives in the scan's
existing early continue, and a unit-level assertion would not touch it.
  </action>
  <verify>
    <automated>cargo test --lib plan_waves</automated>
  </verify>
  <done>Every behaviour listed above is pinned by a named test, all tests contain the word waves, and `cargo test --lib plan_waves` exits 0. A phase whose plans carry no wave key produces an empty plan_waves vector.</done>
  <reversibility rating="reversible">Test-led refinement of one pure function.</reversibility>
</task>

<task type="auto" tdd="true">
  <name>Task 3: At-a-glance wave marker in the Pipeline phase list</name>
  <files>src/ui/screens/detail.rs</files>
  <behavior>
    - A phase whose plans span more than one numbered wave shows a compact wave-count marker in the Pipeline tab's left-hand phase list.
    - A phase with a single wave shows no marker, so the common case stays quiet.
    - A phase with no wave metadata, and a phase with no disk inference at all, show exactly the label they show today.
    - The trailing question-mark bucket is not counted as a wave, since it represents plans with no recorded wave rather than an additional one.
  </behavior>
  <action>
The todo's problem statement is that there is no at-a-glance view across phases; Task
1 only answers it for the selected phase. Close that in the left pane.

Extract the left pane's `ListItem` text construction in `render_pipeline_tab` into
`fn phase_list_label(phase: &RoadmapPhase, inf: Option<&DiskInference>) -> String`, a
free function in the same module, so it is testable without a Frame or a Buffer.
Preserve today's output byte-for-byte when there is no marker to add, including both
existing `shown` calls on the phase number and name.

When `inf` carries two or more `plan_waves` entries with a `Some` wave number, append
a short marker giving that count followed by a lowercase w, separated from the name by
two spaces. Count only numbered waves: the None bucket records plans with no recorded
wave, and counting it would report a phase with one real wave plus some unwaved plans
as parallel. Below two numbered waves, append nothing.

At the call site, look the inference up from `state.phase_disk_statuses` by
`phase.number` and pass it through. The left pane is 40 percent of the tab width, so
ratatui truncates the marker on narrow terminals before the phase name is lost —
acceptable, and no width computation is added here.

Cover the four behaviours with tests over `phase_list_label` directly. Include the
word waves in each test name.
  </action>
  <verify>
    <automated>cargo test --lib waves</automated>
  </verify>
  <done>phase_list_label returns today's exact string for single-wave, unwaved and inference-less phases, and appends a numbered-wave count for phases spanning two or more waves. `cargo test --lib waves` exits 0.</done>
  <reversibility rating="reversible">A pure label function plus its call site.</reversibility>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| another project's `.planning/` -> this TUI's state reader | File names and file contents from a registered third-party project cross into this process. The repository already treats this as untrusted input (`src/driver/untrusted.rs`, `src/ui/screens/render_escape_guard.rs`, the `shown` helper). |
| state reader -> ratatui renderer | Derived strings become terminal cells. |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-vqy-01 | Spoofing/Tampering | plan ids derived from filenames, rendered in `waves_manifest_from_derived` and `phase_list_label` | medium | mitigate | Route every derived plan-id string through detail.rs `shown` before it reaches a `Span`, as the 49 existing call sites do. A filename carrying control or look-alike sequences is neutralised on the same path the rest of the screen uses. |
| T-vqy-02 | Denial of Service | `infer_disk_status` plan scan over a hostile phase directory | low | accept | The scan already reads every `*-PLAN.md` for the superseded check; wave extraction reuses that in-hand content and adds no read. `leading_frontmatter_value` stops at the closing block marker, so an enormous file body is never scanned. The render path adds no IO at all. |
| T-vqy-03 | Tampering | a crafted `wave:` value (non-numeric, negative, or oversized) | low | mitigate | Parsed as `u32` with the error discarded; any failure yields `None` and the plan falls into the unwaved bucket. Pinned by Task 2. |

No package-manager installs are introduced by this plan — no new Cargo dependency,
so the supply-chain row does not apply and no legitimacy checkpoint is required.
</threat_model>

<verification>
- `cargo test --lib waves` exits 0 and reports more tests than the three that exist today.
- `cargo test --lib plan_waves` exits 0.
- `cargo build` exits 0.
- `cargo clippy --lib -- -D warnings` exits 0 — this is the project's clean gate. Do
  not substitute `--all-targets`, which carries five pre-existing lints unrelated to
  this change.
- Do NOT gate on a bare `cargo test --lib`. Its measured baseline on this machine is
  1236 passed / 1 failed / 1 ignored; the single failure is the environmental
  `envelope::policy` git-version-constants test (installed git is newer than the
  constants were derived against) and is not a regression. The filtered commands above
  exclude it by name. Likewise, do not pipe any cargo output through grep to count
  tests — rtk filters build and test output downstream of the pipe, so such a count
  reads low. Use exit codes, or `rtk proxy` if raw output is genuinely needed.
</verification>

<success_criteria>
- Wave grouping for a phase is derived from `*-PLAN.md` frontmatter and reaches the
  Pipeline tab with no `waves.json` present anywhere on disk.
- An existing `waves.json` still takes precedence when it parses and carries waves.
- A project with no wave metadata renders exactly as it does today.
- Wave labels carry the real frontmatter wave numbers, never list positions.
- Phases spanning two or more waves are marked in the Pipeline phase list.
- No new file IO happens on the render tick, and no new Cargo dependency is added.
- `.planning/ROADMAP.md` and `src/state_reader/roadmap_md.rs` are unchanged (see
  measured finding 4 — deliberate, and flagged for audit).
</success_criteria>

<output>
Create `.planning/quick/260916-vqy-visualize-execution-waves-per-phase-in-roadmap-area-ui-sever/260916-vqy-SUMMARY.md` when done.

Retire the source todo by moving
`.planning/todos/pending/2026-09-11-visualize-execution-waves-per-phase-in-roadmap.md`
to the completed todos directory, following whatever convention the previous quick
tasks used. Record the three inferred decisions (measured findings 4, 5 and 6) in the
summary so a later reviewer can audit them.
</output>
