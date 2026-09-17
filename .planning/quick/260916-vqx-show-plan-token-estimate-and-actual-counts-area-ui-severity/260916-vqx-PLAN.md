---
phase: quick-260916-vqx
plan: 01
type: execute
wave: 1
depends_on: []
files_modified:
  - src/state_reader/disk_status.rs
  - src/ui/screens/detail.rs
  - src/ui/screens/render_escape_guard.rs
autonomous: true
requirements: [QUICK-260916-vqx]
user_setup: []

estimate:
  tokens: 90000
  raw_tokens: 45000
  tasks: 3
  confidence: low

must_haves:
  truths:
    - "For a phase whose plans carry `estimate.tokens`, the Pipeline tab shows a per-plan row with that number; once a plan's paired `*-SUMMARY.md` carries `actuals.tokens`, the same row shows that number beside it. A user never has to open a `*-PLAN.md` to see planned-vs-actual cost."
    - "A nested key is a different key, one level in: `frontmatter_token_count(c, \"estimate\")` reads `tokens` ONLY as a direct child of a column-zero `estimate:` block, and never a `tokens:` at column zero, under a different parent, or two levels deep. This is `leading_frontmatter_value`'s WR-05 doctrine applied to a nested block rather than a second, looser rule beside it."
    - "The block reader survives the shape real GSD writes: blank lines and `# comment` lines inside the block do not end it, a column-zero `# heading` comment between two keys does not end it, an inline ` # comment` after the value is not part of the value, and the closing `---` ends everything."
    - "A project whose plans predate `estimate`/`actuals` renders EXACTLY what it renders today: rows with neither number present are never emitted, so `plan_tokens` is empty and the whole section is omitted. Absence degrades to silence, never to a column of dashes."
    - "`DiskInference::plan_tokens` is ordered DETERMINISTICALLY by plan index (numeric, never lexical), so two scans of an unchanged directory produce equal `DiskInference` values. `plan_ids` is a `HashSet`; an unsorted vector would make every refresh compare unequal and defeat the change-suppression that `ProjectState` equality drives."
    - "Every plan identifier drawn on screen is third-party filename text and reaches its cell through `shown()`; `render_escape_guard`'s `the_screen_renders_identity_escaped` probes the Pipeline tab's REAL render branch rather than its `No disk data` branch, so that claim is measured and not asserted."
    - "No new Cargo dependency is added (CLAUDE.md stack constraint), and the token numbers are parsed with the existing hand-rolled line scan — no YAML crate joins the tree for four integers."
    - "Per-frame file I/O is not added: the numbers are read during the existing `.planning/` disk scan and cached in `ProjectState`, invalidated with it. The render path reads `DiskInference` only."
  artifacts:
    - "src/state_reader/disk_status.rs — `leading_frontmatter_nested_value`, `frontmatter_token_count`, `pub struct PlanTokens`, `DiskInference::plan_tokens`, and the population of both halves inside the existing single directory scan"
    - "src/ui/screens/detail.rs — `fmt_tokens`, `build_plan_token_lines`, `MAX_PLAN_TOKEN_ROWS`, and the call site inside `render_pipeline_tab`"
    - "src/ui/screens/render_escape_guard.rs — a populated `phase_disk_statuses` in `hostile_project_state`, closing the Pipeline-tab fixture hole so the new sink is actually probed"
  key_links:
    - "`infer_disk_status` pass 1 -> `plan_estimates` -> `PlanTokens.estimate`: the plan file is ALREADY read there for the superseded check, so the estimate costs no extra I/O"
    - "`infer_disk_status` pairing loop (`matched_plans`) -> summary read -> `PlanTokens.actual`: the actual is attributed through the SAME plan/summary pairing that drives `summary_count`, so a summary that does not count toward completion does not contribute a number either"
    - "`plan_index()` -> the sort key of `plan_tokens`: the one place `HashSet` iteration order is converted into a stable order, and the reason `DiskInference` stays comparable"
    - "`PlanTokens.label()` -> `shown()` -> ratatui `Span`: the escape boundary for a filename stem that came out of another project's directory"
    - "`hostile_project_state.phase_disk_statuses` -> `render_pipeline_tab`'s `Some(inf)` arm: the fixture edge that makes the guard non-vacuous for this tab"
---

<objective>
Show, per plan, the tokens the plan estimated and the tokens it actually cost.

Purpose: GSD writes `estimate.tokens` into every `*-PLAN.md` frontmatter and `actuals.tokens`
into the matching `*-SUMMARY.md`, but nothing in this TUI reads either. A user comparing planned
against actual cost has to open each file by hand — in a tool whose whole premise is a single
pane over many projects. There is currently NO parser for plan or summary frontmatter in
`src/state_reader/`; this is new capability.

Output: a `plan_tokens` vector on the already-cached `DiskInference`, and a compact
`Plan tokens (est/act)` section in the Pipeline tab's right pane, next to the existing stage,
sub-stage and waves sections.
</objective>

<execution_context>
@~/.claude/gsd-core/workflows/execute-plan.md
@~/.claude/gsd-core/templates/summary.md
</execution_context>

<context>
@.planning/STATE.md
@CLAUDE.md
@.planning/todos/pending/2026-09-11-show-plan-token-estimate-and-actual-counts.md

@src/state_reader/disk_status.rs
</context>

<measured_ground_truth>
Established at plan time against this repository's own `.planning/` tree. The executor should
confirm the shapes still match and then proceed; do not re-derive them.

**1. The two on-disk shapes are NOT the same shape.**

`.planning/phases/22-container-execution-target/22-01-PLAN.md`, lines 23-27 — a clean nested
block, preceded by a blank line:

```text
estimate:
  tokens: 95000
  raw_tokens: 95000
  tasks: 3
  confidence: low
```

`.planning/phases/19-gitsafe-git-blast-radius-envelope/19-12-SUMMARY.md`, lines 21-27 — the same
nesting, but with a column-zero `#` heading comment ABOVE the parent and two indented `#`
comments INSIDE the block before the key:

```text
# Actuals (#2632)
actuals:
  # chars/4 over the realized diff, which is the whole of the one created file
  # (51 383 chars / 4). The plan estimated 80 000 on the same scale.
  tokens: 12846
  tasks: 3
  commits: 3
```

A reader that treats any column-zero line as "the block ended", or any non-key line as "the
block ended", reads the plan correctly and the summary as absent — half the feature, silently.
Both shapes are required fixtures.

**2. `raw_tokens` is a sibling of `tokens`, and a substring of nothing.** Split on the FIRST
`:` and compare the trimmed key for equality. A `contains("tokens")` test matches `raw_tokens`
first in the plan block and reports 95000 as if it were the calibrated figure.

**3. The GSD plan template writes an inline comment after the value** —
`tokens: 60000             # calibrated projection` — even though this repo's own plans do not.
Strip at the first whitespace-preceded `#` before parsing.

**4. The plan file is already read; the summary file is not.** `infer_disk_status` reads every
`*-PLAN.md` at `disk_status.rs:634-639` for the superseded check, so the estimate is free. The
pairing loop at `:704-716` only ever looks at summary FILENAMES. Reading a summary is a genuine
new read, bounded by plan count (33 in the largest phase here, ~116 across the repo), inside a
scan that already performs the plan-side reads. It happens once per state refresh, not per frame.

**5. `DiskInference` has exactly ONE exhaustive struct literal in the tree** — `disk_status.rs:745`.
Every other one of the 41 construction sites closes with `..Default::default()` or `..other`
(measured across `src/` and `tests/`). Adding a field therefore breaks one site, not forty.

**6. `plan_ids` is a `HashSet<String>`.** Its iteration order is not stable across runs, and
`DiskInference` derives `PartialEq` and is compared to suppress spurious "Updated" status
(quick task 260512-eyv). An unsorted `plan_tokens` would make every project look changed on
every refresh. The sort is load-bearing, not cosmetic.

**7. The Pipeline tab is enumerated by the escape guard but its real branch is never drawn.**
`render_escape_guard.rs:1460` maps `DetailSubView::Pipeline` to the probe label `"Pipeline tab"`
and `the_screen_renders_identity_escaped` renders it — but `hostile_project_state`
(`:859-892`) leaves `phase_disk_statuses` at `..Default::default()`, i.e. empty, so
`render_pipeline_tab` returns at its `None => "No disk data"` arm every time. This is LIMIT 1 of
that module's own doc (a fixture hole) sitting on the exact tab this plan adds a sink to.
</measured_ground_truth>

<inferred_decisions>
The human operator was unavailable. These decisions were taken from the measured evidence above
and from the todo file, and are flagged here for later audit.

- **D-INF-01 — The numbers live on `DiskInference`, read during the disk scan; the renderer reads
  no files.** The alternative with local precedent is the `waves.json` block at
  `detail.rs:3867-3887`, which does `read_to_string` inside the render function. That is
  tolerable for ONE small file per frame and intolerable for up to 66 (33 plans + 33 summaries
  in phase 19). The todo also asks for exactly this ("Cache parsed values the same way other
  `.planning/` state is cached"), and `DiskInference` is already cached in
  `ProjectState.phase_disk_statuses` and already invalidated by the notify-driven refresh.

- **D-INF-02 — A plan row with NEITHER number is omitted, and an empty `plan_tokens` omits the
  whole section.** Older GSD versions populate neither key. Emitting a row per plan would give
  such projects a block of `-  -` lines with no information in it. The todo's requirement is
  "fall back gracefully when either field is absent"; falling back to nothing is the graceful
  form. Consequence to accept: the row count is not the plan count, so the section header states
  its own denominator (`n/plan_count measured`) rather than leaving the reader to infer it.

- **D-INF-03 — The list is CAPPED at 10 rows plus an explicit `... +N more` line.** The Pipeline
  pane is a plain `Paragraph` with no scroll (`detail.rs:3889`), so lines past the pane height
  are simply not drawn. Phase 19 has 33 plans. An uncapped list would push the overflow off the
  bottom invisibly; a cap with a stated remainder is honest about what is not shown, and the
  phase totals on the header line cover ALL plans regardless of the cap. This is a rendering
  bound, not a scope reduction: no plan's numbers are dropped from the totals, and adding scroll
  state to this pane is out of proportion to a quick task. Flagged for audit: if this proves
  annoying, the follow-up is pane scrolling, not a bigger constant.

- **D-INF-04 — The escape-guard fixture hole is closed in this plan rather than deferred.** This
  plan adds a third-party-text sink (a plan filename stem) to a tab whose probe currently draws
  the `No disk data` branch. Landing the sink without landing the fixture would produce a guard
  that passes while certifying nothing about the new code — the precise failure `render_escape_guard`'s
  header documents at length (CR-01, the orphaned `project_list.rs` call). Cost: one struct field
  in one fixture function.

- **D-INF-05 — `depends_on: []`, with an honest `files_modified`.** Sibling batch item 260916-vqy
  also renders phase/plan information and will plausibly touch `src/ui/screens/detail.rs`; sibling
  260916-vr0's own plan records that vqz, vqx and vqy all touch it. This item needs NOTHING vqy
  produces — it reads plan/summary frontmatter, which no sibling parses — so the dependency is
  declared as a file overlap for the merge DAG to serialize, not as a semantic dependency that
  would needlessly order the two.
</inferred_decisions>

<tasks>

<task type="tracer" tdd="true">
  <name>Task 1: Read estimate.tokens and actuals.tokens into DiskInference</name>
  <files>src/state_reader/disk_status.rs</files>
  <read_first>
    - `src/state_reader/disk_status.rs:247-302` — `leading_frontmatter_value` and the doc above
      it. Read the WR-05 paragraph in full: it explains, with a worked example, why a key at
      column zero and the same key one level in are DIFFERENT keys, and why widening the scan
      reintroduces a defect GSD's own library has a name for. The new nested reader is that same
      doctrine inverted (accept the child, reject the column-zero twin), not an exception to it.
    - `disk_status.rs:304-332` — `plan_index`, including why it parses two `u32`s instead of
      comparing text. It is the sort key in this task.
    - `disk_status.rs:334-345` — `plan_frontmatter_superseded`, the existing consumer of the
      plan file's content.
    - `disk_status.rs:507-520` and `:628-660` — the two-pass scan's declarations and the two
      `continue` arms this task extends.
    - `disk_status.rs:686-717` — the pairing block, including the two-tier rule and the
      "ambiguity abstains" clause. The actual must be attributed through this and no other rule.
    - `disk_status.rs:745-773` — the one exhaustive `DiskInference` literal.
  </read_first>
  <behavior>
    Unit tests in the existing `#[cfg(test)] mod tests` block, each named for the property:

    - **The clean nested block.** `frontmatter_token_count(PLAN_22_01_SHAPE, "estimate")` is
      `Some(95000)`. Fixture is the verbatim five-line block from `22-01-PLAN.md` inside a
      realistic surrounding frontmatter.
    - **The commented nested block.** `frontmatter_token_count(SUMMARY_19_12_SHAPE, "actuals")`
      is `Some(12846)`. Fixture carries the column-zero `# Actuals (#2632)` heading above the
      parent AND the two indented `#` comment lines before the key, verbatim.
    - **A sibling is not the key.** The same clean fixture yields 95000 and never 95000-from-
      `raw_tokens`; a fixture whose `raw_tokens` differs from `tokens` (`tokens: 1`,
      `raw_tokens: 2`) returns 1.
    - **A column-zero twin is a different key.** A frontmatter with `tokens: 999` at column zero
      and no `estimate:` block returns `None` for parent `estimate`.
    - **A key under the wrong parent is a different key.** `actuals:\n  tokens: 5` returns `None`
      for parent `estimate`, and vice versa.
    - **Two levels in is a different key.** `estimate:\n  breakdown:\n    tokens: 7` returns
      `None` — only a DIRECT child at the block's first indent level counts.
    - **The block ends at the next column-zero key.** `estimate:\n  tasks: 3\nmust_haves:\n  tokens: 9`
      returns `None`.
    - **The block ends at the closing `---`.** A `tokens:` below the closing fence is never read,
      matching the existing `test_status_key_below_the_leading_block_is_never_matched`.
    - **A non-frontmatter file yields None.** No leading `---` on line 1 -> `None`.
    - **Inline comments are not part of the value.** `tokens: 60000   # calibrated projection`
      yields `Some(60000)`.
    - **Junk is None, never a panic.** `tokens: soon`, `tokens:` (empty), `tokens: -5`,
      `tokens: 1e5` all yield `None`.
    - **End-to-end over a TempDir phase directory** (the tracer): a directory holding
      `07-01-slug-PLAN.md` (estimate 95000) + `07-01-SUMMARY.md` (actuals 12846),
      `07-02-slug-PLAN.md` (estimate 40000, no summary), and `07-03-slug-PLAN.md` (no `estimate`
      block at all, no summary) produces `plan_tokens` of LENGTH 2, in index order
      `["07-01", "07-02"]`, with `(Some(95000), Some(12846))` and `(Some(40000), None)`. Plan
      07-03 contributes no row (D-INF-02). `plan_count` is still 3 and `summary_count` still 1 —
      this task changes no count.
    - **Order is deterministic and numeric.** A directory whose plans are `07-2` and `07-10`
      yields rows ordered `07-02`-then-`07-10` by INDEX, and calling `infer_disk_status` twice on
      the identical directory returns two `DiskInference` values that compare EQUAL. Assert the
      equality of the whole inference, not just the vector — that is the property `ProjectState`
      change-suppression depends on.
    - **A superseded plan contributes no row.** Its estimate is not collected and its index is
      not registered, matching `test_superseded_plan_excluded_from_counts`.
    - **An ambiguous index abstains on the actual too.** Two surviving plans sharing index
      `07-01` pair with no summary today; neither gains an `actual`.
  </behavior>
  <action>
    Add, at column zero and BEFORE this file's `#[cfg(test)]` marker (`disk_status.rs:903`):
    `tests/spawn_seam_guard.rs::no_production_item_follows_a_test_module_marker` fails the build
    on any column-zero production item placed after that marker.

    1. `fn leading_frontmatter_nested_value(content: &str, parent: &str, key: &str) -> Option<String>`.
       Same byte-zero anchor as `leading_frontmatter_value`: line 1 must be a bare `---`, and the
       scan stops at the closing `---`. State the rules in the doc comment and implement exactly
       them: a blank line and a line whose trimmed form starts with `#` are SKIPPED and end
       nothing; an unindented line is a column-zero key line, which either opens the block (its
       key equals `parent` and its value is empty) or, once the block is open, closes it; inside
       the block, the indentation width of the first non-blank non-comment line is recorded as
       the block's own level and only lines at exactly that width are candidate children, so a
       deeper nesting is a different key one level further in. Return the trimmed value of the
       first child whose key equals `key`. Cross-reference `leading_frontmatter_value`'s WR-05
       paragraph in the doc comment rather than restating its argument.

    2. `fn frontmatter_token_count(content: &str, parent: &str) -> Option<u64>`. Calls the above
       with key `"tokens"`, truncates the value at the first whitespace-preceded `#`, trims
       surrounding `"` and `'`, and parses `u64`. Anything that does not parse is `None` —
       fail-safe, exactly as the rest of this file behaves on a malformed input.

    3. `pub struct PlanTokens { pub id: String, pub estimate: Option<u64>, pub actual: Option<u64> }`
       deriving `Debug, Clone, Default, PartialEq` so `DiskInference`'s own derives still hold.
       Give it `pub fn label(&self) -> String` returning the zero-padded `NN-MM` form when
       `plan_index(&self.id)` parses, the id itself when it does not, and the authored string
       `"PLAN"` when the id is empty (the standalone `PLAN.md` case).

    4. Add `pub plan_tokens: Vec<PlanTokens>` to `DiskInference` with a doc comment saying what a
       row means, that a row exists only when at least one of the two numbers was found, and that
       the order is by plan index so the value stays comparable.

    5. In the pass-1 arm at `:634`, read the plan content ONCE into a local `Option<String>`,
       keep the existing superseded `continue` driven by it, and collect
       `frontmatter_token_count(content, "estimate")` into a
       `HashMap<String, Option<u64>>` keyed by the plan id. Do not add a second read of the file.

    6. In the pairing loop at `:704-716`, when — and only when — a summary resolves to a
       surviving plan by either tier, read that summary with `std::fs::read_to_string` and record
       `frontmatter_token_count(content, "actuals")` under the matched plan's id. An unreadable
       summary records nothing; it must not disturb `matched_plans` or `summary_count`.

    7. Build `plan_tokens` after the pairing: one row per plan id that has a `Some` estimate or a
       `Some` actual, then `sort_by` on `(plan_index(&id).unwrap_or((u32::MAX, u32::MAX)), id.clone())`
       so the order is total and independent of `HashSet` iteration order. Add it to the
       exhaustive literal at `:745`.

    Do not change `plan_count`, `summary_count`, the status ladder, or any existing pairing
    behaviour. This task adds a field and reads two numbers; every existing test in this module
    must still pass unmodified.

    Read cargo output through `rtk proxy sh -c '...'` with the pipe INSIDE the quoted command.
    The rtk hook rewrites a piped `tail`/`grep` as its own filtered command even when the cargo
    call was proxied, which silently drops the `test result:` line and fakes a count.
  </action>
  <verify>
    <automated>rtk proxy sh -c 'cargo test --lib state_reader::disk_status 2>&amp;1 | tail -25'</automated>
  </verify>
  <done>
    `test result: ok` with a count STRICTLY GREATER than the 79 tests measured on this module
    today, `0 failed`. The end-to-end TempDir test reports two rows in index order for a
    three-plan directory, and the twice-scanned inference compares equal.
  </done>
</task>

<task type="auto" tdd="true">
  <name>Task 2: Render the per-plan estimate/actual section in the Pipeline tab</name>
  <files>src/ui/screens/detail.rs</files>
  <read_first>
    - `src/ui/screens/detail.rs:40-79` — the doc above `shown()` and the function itself. Every
      string in this task that came out of another project's filesystem goes through it.
    - `detail.rs:4947-5050` — `WavesManifest` through `build_waves_lines`. This is the shape to
      mirror: a section header `Span`, then indented rows, fixed-width columns via `{:<n}`,
      `Color::DarkGray` for labels and `Color::Cyan` for the identifier column.
    - `detail.rs:4915-4945` — `build_stage_detail_lines`, for the `  {:<12}` label convention.
    - `detail.rs:3815-3892` — the `Some(inf)` arm of `render_pipeline_tab`, its `lines` vector,
      the external-job block, and the waves block. The insertion point is between them.
    - `detail.rs:6551` — the `mod tests {` marker. New production items go ABOVE it.
  </read_first>
  <behavior>
    Unit tests in `detail.rs`'s existing `mod tests`:

    - **`fmt_tokens` boundary table.** `0 -> "0"`, `999 -> "999"`, `1_000 -> "1.0k"`,
      `12_846 -> "12.8k"`, `95_000 -> "95.0k"`, `100_000 -> "100k"`, `999_999 -> "1000k"`,
      `1_000_000 -> "1.0M"`, `1_250_000 -> "1.3M"`. Assert the exact strings — the point is that
      the column width is predictable, so the boundaries are pinned rather than described.
    - **ASCII only.** Every character `fmt_tokens` can emit, and every character the section's
      authored strings contain, is `is_ascii()`. This project has an open todo about badge glyph
      display width misaligning by one cell across terminals; a numeric column is the last place
      to introduce a non-ASCII glyph.
    - **Empty in, empty out.** `build_plan_token_lines` on a `DiskInference` with empty
      `plan_tokens` returns an empty `Vec`, so the section vanishes for a project whose plans
      carry neither key.
    - **The header states both totals and the denominator.** Three rows, two of them with an
      actual, over a `plan_count` of 5, produce a header containing the summed estimate, the
      summed actual, and `2/5` — the measured count over the PHASE's plan count, not over the
      row count.
    - **Rows render both numbers, and a dash for an absent one.** A row with estimate only shows
      the estimate and `-`; a row with actual only shows `-` and the actual.
    - **The delta is signed and relative to the estimate.** estimate 95000 / actual 12846 renders
      a string containing `-86%` or `-87%` (assert on the computed value, one rounding rule, not
      on a guess); estimate 10000 / actual 15000 renders `+50%`. A row missing either number
      renders no delta. An estimate of `0` renders no delta and does not divide.
    - **The cap is honest.** 14 rows produce `MAX_PLAN_TOKEN_ROWS` row lines plus exactly one
      trailing line containing `+4 more`, and the header's totals still sum ALL 14.
    - **The label is escaped.** `build_plan_token_lines` given a `PlanTokens` whose `id` carries
      a raw `\u{202E}` and a raw ESC emits a line whose rendered text contains neither — assert
      on the characters of the produced `Line`, which is what `shown()` is there to guarantee.
  </behavior>
  <action>
    Add above the `mod tests {` marker at `detail.rs:6551`:

    1. `const MAX_PLAN_TOKEN_ROWS: usize = 10;` with a comment naming D-INF-03 — this pane is a
       `Paragraph` with no scroll, so an uncapped list overflows invisibly.

    2. `fn fmt_tokens(n: u64) -> String` implementing exactly the boundary table in `<behavior>`:
       below 1 000 the bare integer; below 100 000 one decimal place with a `k` suffix; below
       1 000 000 zero decimal places with a `k` suffix; otherwise one decimal place with an `M`
       suffix. ASCII only.

    3. `fn build_plan_token_lines(inf: &DiskInference) -> Vec<Line<'static>>`. Returns empty when
       `inf.plan_tokens` is empty. Otherwise: a `Color::DarkGray` bold header line reading
       `  Plan tokens (est/act):` followed on the same line by the phase totals — the summed
       estimates, the summed actuals, and the measured count over `inf.plan_count` — then up to
       `MAX_PLAN_TOKEN_ROWS` rows of the form
       `    {label:<10} est {:>7}  act {:>7}  {delta}`, then, if any rows were dropped, a
       `Color::DarkGray` line stating how many. The label span is `shown(&row.label())` in
       `Color::Cyan`; the numbers and the authored words are this crate's own text and are not
       run through `shown()` — keep that asymmetry visible in a comment, as
       `unreadable_state_line` does at `:93`. Colour the delta green when the actual is at or
       under the estimate and yellow when it is over.

    4. Call it from `render_pipeline_tab`'s `Some(inf)` arm, inserted BETWEEN the external-job
       block that ends at `:3863` and the waves block that begins at `:3865`: push a blank
       `Line`, then the produced lines, only when the produced vector is non-empty. Do not move,
       reorder or reindent the waves block — sibling batch item 260916-vqy is expected to work in
       that region and a gratuitous reflow there is a merge conflict for no gain.

    Do not add a file read to this function or to any helper it calls; every number comes off
    `inf`. Do not alter `build_waves_lines`, `build_stage_detail_lines` or
    `build_substage_lines`.

    Read cargo output through `rtk proxy sh -c '...'` with the pipe inside the quoted command,
    for the reason given in task 1.
  </action>
  <verify>
    <automated>rtk proxy sh -c 'cargo test --lib ui::screens::detail 2>&amp;1 | tail -25'</automated>
  </verify>
  <done>
    `test result: ok` with a count strictly greater than the 74 tests measured on this module
    today, `0 failed`. `build_plan_token_lines` returns an empty vector for an empty
    `plan_tokens`, and the escaping test passes against a `\u{202E}`-bearing plan id.
  </done>
</task>

<task type="auto">
  <name>Task 3: Close the Pipeline-tab fixture hole so the new sink is actually probed</name>
  <files>src/ui/screens/render_escape_guard.rs</files>
  <read_first>
    - `src/ui/screens/render_escape_guard.rs:1-90` — the module header, in particular the
      CORRECTED 2026-08-27 block and the LIMIT list. LIMIT 1 is the fixture-hole class this task
      closes one instance of.
    - `render_escape_guard.rs:859-892` — `hostile_project_state`, the function this task edits,
      and `:894-935` — the doc above `probe_ctx` describing the `git_entries` hole and how
      closing it changed what the probe measured. This edit is the same move on the Pipeline tab.
    - `render_escape_guard.rs:1445-1465` — `detail_tabs_expected_to_arrive` and `sub_view_label`,
      which already enumerate `"Pipeline tab"`.
    - `src/ui/screens/detail.rs:3815-3820` — the `match inference { None => "No disk data", ... }`
      arm that the empty fixture currently lands on.
  </read_first>
  <action>
    In `hostile_project_state`, populate `phase_disk_statuses` with ONE entry whose key is `"1"`
    — the same string the fixture's single `RoadmapPhase.number` carries at `:876`, because
    `render_pipeline_tab` looks the inference up by exactly that value. The `DiskInference` sets
    `status`, `plan_count: 1`, `summary_count: 1` and a one-element `plan_tokens` whose `id` is
    `identity` and whose `estimate` and `actual` are both `Some`, closing with
    `..Default::default()`. Derive the key from the fixture's own phase entry rather than
    respelling `"1"` a second time if that reads more clearly; the existing `project_path`
    derivation at `:989-994` is the precedent for deriving over respelling.

    Add a comment above it in the register of the surrounding ones: state that without this the
    Pipeline tab probe returned at `detail.rs`'s `No disk data` arm, so the tab was enumerated by
    `detail_tabs_expected_to_arrive` while its real render — which now draws a plan identifier
    read out of a third-party `.planning/` filename — was never drawn by any committed control.

    **Prove the fixture edge is load-bearing before trusting it.** Temporarily make
    `build_plan_token_lines` emit the raw `row.label()` instead of `shown(&row.label())`, run the
    guard, and CONFIRM `the_screen_renders_identity_escaped` fails naming `DetailScreen [Pipeline
    tab]`. Restore the `shown()` call and confirm green. Record the verbatim red output in the
    commit message. A fixture that cannot be shown to fail is a fixture that certifies nothing —
    this module's header makes that argument at length and this task is not exempt from it.

    If populating the fixture turns some OTHER assertion red, that is a pre-existing hole this
    edit has just exposed. Report it in the summary with the verbatim failure; do NOT narrow the
    fixture to make it green again.

    Then run the whole suite. The baseline recorded on 2026-09-15 (quick task 260915-hsh) is
    2005 passed / 1 failed, the single failure being the environmental git-version-constants test
    in `src/envelope/policy.rs` (installed git 2.53 against the constants' expectation) — not a
    regression from this work. `--no-fail-fast` is MANDATORY: a plain `cargo test` stops at the
    first failing test binary and never reaches the `envelope_*` binaries, reporting a number
    that is unrelated to what this plan changed.
  </action>
  <verify>
    <automated>rtk proxy sh -c 'cargo test --lib ui::screens::render_escape_guard 2>&amp;1 | tail -10 &amp;&amp; cargo clippy -- -D warnings 2>&amp;1 | tail -5 &amp;&amp; cargo test --no-fail-fast 2>&amp;1 | grep -E "^test result:"'</automated>
  </verify>
  <done>
    The guard's 14 tests pass with the populated fixture, and the deliberate `shown()` removal was
    observed to turn `the_screen_renders_identity_escaped` red naming the Pipeline tab.
    `cargo clippy -- -D warnings` exits 0. The full `--no-fail-fast` run's failing set is a subset
    of the recorded baseline — at most the environmental git-version-constants test — and the
    total passed count is strictly greater than 2005.
  </done>
</task>

</tasks>

<threat_model>
## Trust Boundaries

| Boundary | Description |
|----------|-------------|
| registered project's `.planning/` -> this process | Every byte read here is authored by a DIFFERENT project's agents and tools. Filenames, frontmatter keys and frontmatter values are all third-party input. |
| parsed values -> ratatui `Buffer` -> the user's terminal | A terminal interprets control bytes it is handed. A string that reaches a cell unescaped is code execution surface, not text. |

## STRIDE Threat Register

| Threat ID | Category | Component | Severity | Disposition | Mitigation Plan |
|-----------|----------|-----------|----------|-------------|-----------------|
| T-VQX-01 | Tampering | `PlanTokens.label()` drawn by `build_plan_token_lines` | high | mitigate | The label is a filename stem from a foreign directory and can carry a raw ESC, a C0/C1 control or `U+202E`. It reaches its `Span` only through `shown()` (`detail.rs:77`). Task 3 makes `render_escape_guard`'s behavioural probe actually draw this tab, and proves the control fails when `shown()` is removed. |
| T-VQX-02 | Denial of Service | `frontmatter_token_count` / `leading_frontmatter_nested_value` | low | mitigate | Both are single forward line scans over content already in memory, bounded by the closing `---`, with no backtracking, no recursion and no regex. A hostile frontmatter cannot make either super-linear. |
| T-VQX-03 | Denial of Service | the new summary read inside `infer_disk_status` | medium | mitigate | One additional `read_to_string` per PAIRED summary, bounded by the plan count of the phase, inside the once-per-refresh disk scan that already reads every `*-PLAN.md`. D-INF-01 keeps it off the render path entirely; an unreadable or absent summary is fail-safe (`None`), never an error or a retry. |
| T-VQX-04 | Information Disclosure | the rendered token counts | low | accept | The numbers are integers the user's own registered projects wrote, displayed to that same user in their own terminal. No path, no prose and no identifier beyond the plan stem is newly surfaced. |
| T-VQX-05 | Spoofing | `actuals.tokens` attributed to the wrong plan | medium | mitigate | The actual is attributed ONLY through the existing two-tier pairing at `disk_status.rs:704-716`, whose "ambiguity abstains" clause already refuses to let one summary satisfy two plans. No second, looser matching rule is introduced — a plan with an unpaired summary shows no actual rather than a neighbour's. |
| T-VQX-SC | Tampering | dependency supply chain | n/a | accept | No package-manager install occurs. This plan adds no crate: the numbers are parsed by the existing hand-rolled line scan, per CLAUDE.md's "no new Cargo dependencies" constraint. No package-legitimacy checkpoint is therefore required. |

**Gate notes.** The API-coverage checkpoint does not fire: this work integrates no external API,
SDK or service — it reads local files under `.planning/` and draws to a local terminal. The
assumption-delta checkpoint is skipped (`phase_unresolved`: a quick task has no ROADMAP phase
section to scan). The schema-push gate is skipped: no ORM, migration or schema file is in scope.
</threat_model>

<verification>
1. `rtk proxy sh -c 'cargo test --lib state_reader::disk_status 2>&1 | tail -25'` — more than 79
   tests, 0 failed.
2. `rtk proxy sh -c 'cargo test --lib ui::screens::detail 2>&1 | tail -25'` — more than 74 tests,
   0 failed.
3. `rtk proxy sh -c 'cargo test --lib ui::screens::render_escape_guard 2>&1 | tail -10'` — 14
   tests, 0 failed, with the populated Pipeline fixture.
4. `rtk proxy cargo clippy -- -D warnings` — exit 0. (`--all-targets` has 5 pre-existing lints
   recorded in `14-CONTEXT.md`; the project gate is the lib-target form above.)
5. `rtk proxy sh -c 'cargo test --no-fail-fast 2>&1 | grep -E "^test result:"'` — passed count
   strictly above the 2005 baseline; the failing set at most the single environmental
   git-version-constants test.
6. Manual smoke, optional but cheap: run the TUI against this repository, open a project's
   Pipeline tab and select phase 22 (its `22-01-PLAN.md` carries `estimate.tokens: 95000` and has
   no summary) and phase 19 (33 plans, many with both keys). Phase 22 shows an estimate and a
   `-`; phase 19 shows a capped list with `+N more` and non-zero totals.
</verification>

<success_criteria>
- A plan's `estimate.tokens` and its paired summary's `actuals.tokens` are both visible in the
  Pipeline tab without opening either file.
- A project whose plans carry neither key renders byte-identically to today.
- `leading_frontmatter_nested_value` reads a direct child and refuses a column-zero twin, a
  wrong-parent key and a two-levels-in key, proved by named tests for each.
- `plan_tokens` is ordered by numeric plan index, and two scans of an unchanged directory produce
  equal `DiskInference` values.
- No render-time file I/O is added and no Cargo dependency is added.
- `the_screen_renders_identity_escaped` draws the Pipeline tab's real branch, and was observed
  red when `shown()` was removed from the new label.
</success_criteria>

<output>
Create `.planning/quick/260916-vqx-show-plan-token-estimate-and-actual-counts-area-ui-severity/260916-vqx-SUMMARY.md` when done.
</output>
