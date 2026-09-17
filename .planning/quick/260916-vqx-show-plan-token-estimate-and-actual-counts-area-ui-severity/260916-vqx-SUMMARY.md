---
phase: quick-260916-vqx
plan: 01
subsystem: ui
tags: [state-reader, frontmatter, pipeline-tab, escape-guard, tokens]
status: complete

requires:
  - "src/state_reader/disk_status.rs::infer_disk_status (the two-pass scan and its pairing loop)"
  - "src/ui/screens/detail.rs::shown (the SAFE-07 escape boundary)"
provides:
  - "DiskInference::plan_tokens — per-plan estimate/actual token counts, cached with the inference"
  - "disk_status::PlanTokens + PlanTokens::label()"
  - "detail::build_plan_token_lines / fmt_tokens — the Pipeline tab's `Plan tokens (est/act)` section"
  - "render_escape_guard::hostile_project_state now populates phase_disk_statuses (Pipeline tab probes its REAL branch)"
affects:
  - "Pipeline tab right pane (one new section between the external-job block and the waves block)"
  - "DETAIL_TAB_ARRIVAL's `Pipeline tab` reason string"

tech-stack:
  added: []
  patterns:
    - "Nested frontmatter reads use a hand-rolled single forward line scan, anchored at byte zero — no YAML crate, matching leading_frontmatter_value"
    - "Third-party filename text reaches a ratatui Span only through shown(); the crate's own authored strings and arithmetic deliberately do not, and the asymmetry is commented at the site"
    - "A fixture edge that closes a probe hole is proved fail-first by reverting exactly one escape, capturing the verbatim red, and restoring it"

key-files:
  created: []
  modified:
    - src/state_reader/disk_status.rs
    - src/ui/screens/detail.rs
    - src/ui/screens/render_escape_guard.rs

decisions:
  - "A nested key is a different key, both directions: leading_frontmatter_nested_value refuses a column-zero twin, a wrong-parent key and a two-levels-in key — WR-05's doctrine inverted rather than an exception beside it"
  - "Blank lines and `#` comments end nothing, because the shape GSD writes into a *-SUMMARY.md has a column-zero heading comment above the parent and indented comments inside the block"
  - "plan_tokens is sorted by NUMERIC plan index so DiskInference stays comparable across refreshes (260512-eyv change suppression)"
  - "fmt_tokens rounds half-up with integer arithmetic, not `{:.1}` on a float — Rust rounds a float tie to even, which would render 1.25M as 1.2M"
  - "The actual is attributed ONLY through the existing two-tier pairing; no second, looser matching rule was introduced"

metrics:
  duration: one session
  completed: 2026-09-17
  tasks: 3

actuals:
  tokens: 10758        # chars/4 over the realized diff (43 030 added chars / 4)
  tasks: 3
  commits: 3
plan_head_before: 2251792f2a3cc98c28f9f55c6d002734055cf3fe
---

# Quick Task 260916-vqx: Show plan token estimate and actual counts — Summary

Per-plan `estimate.tokens` and `actuals.tokens` are now parsed during the existing
`.planning/` disk scan, cached on `DiskInference`, and drawn as a compact
`Plan tokens (est/act)` section in the Pipeline tab — with the tab's escape probe
finally rendering its real branch instead of `No disk data`.

## Tasks

| # | Task | Commit | Files |
|---|------|--------|-------|
| 1 | Read `estimate.tokens` and `actuals.tokens` into `DiskInference` (tracer, TDD) | `ac0b203` | `src/state_reader/disk_status.rs` |
| 2 | Render the per-plan estimate/actual section in the Pipeline tab (TDD) | `853d639` | `src/ui/screens/detail.rs` |
| 3 | Close the Pipeline-tab fixture hole so the new sink is probed | `f330b9a` | `src/ui/screens/render_escape_guard.rs`, `src/state_reader/disk_status.rs` |

## What was built

**`leading_frontmatter_nested_value(content, parent, key)`** — a byte-zero-anchored
forward line scan reading a key ONE LEVEL inside a column-zero block. Blank lines and
`#` comments are skipped and end nothing; an unindented line opens the block (key equals
`parent`, empty value) or closes it; inside the block the indentation of the first
non-blank non-comment line is the block's level and only lines at exactly that width are
candidate children. `frontmatter_token_count` layers on the `tokens` key, strips a
whitespace-preceded inline `#` comment and surrounding quotes, and parses `u64`.

**`PlanTokens { id, estimate, actual }` + `DiskInference::plan_tokens`** — one row per
surviving plan with at least one number, ordered by numeric plan index. The estimate is
read out of the plan-file read the superseded check already performs (no extra I/O); the
actual is read from a summary only when that summary resolves to a surviving plan through
the existing two-tier pairing.

**`fmt_tokens` + `build_plan_token_lines` + `MAX_PLAN_TOKEN_ROWS`** — a header carrying the
phase's summed estimate, summed actual and `measured/plan_count`, then up to 10 rows of
`est N  act N  ±P%` (green at or under estimate, yellow over), then one honest `+N more`
line when the cap drops any. Empty `plan_tokens` produces zero lines.

**The escape-guard fixture** — `hostile_project_state` now populates
`phase_disk_statuses`, keyed off its own phase entry, with a `plan_tokens` row whose `id`
is the hostile identity.

## Measured evidence

**Real data (ephemeral smoke over this repository's own `.planning/`, run then deleted):**

```
.planning/phases/22-container-execution-target
  plan_count=4 summary_count=0 rows=4
    22-01    est Some(95000) act None
    22-02    est Some(80000) act None
.planning/phases/19-gitsafe-git-blast-radius-envelope
  plan_count=33 summary_count=33 rows=33
    19-01    est Some(72000) act Some(16114)
    19-02    est Some(68000) act Some(15800)
```

Phase 22 renders estimates with `-` for every actual; phase 19 renders 10 rows plus
`... +23 more` with both totals non-zero — exactly what verification item 6 predicted.

**Task 3's fail-first proof.** With `shown()` removed from `build_plan_token_lines` and
nothing else changed:

```
thread 'ui::screens::render_escape_guard::tests::the_screen_renders_identity_escaped' (294725) panicked at src/ui/screens/render_escape_guard.rs:2894:17:
DetailScreen (src/ui/screens/detail.rs) [Pipeline tab] rendered ['\u{e0041}'] into the terminal buffer. Those characters render as nothing, so what the operator reads is not what the value is.
```

`shown()` restored → 14 passed / 0 failed. No other assertion turned red under the
populated fixture.

## Verification

| Gate | Baseline | Result |
|------|----------|--------|
| `cargo test --lib state_reader::disk_status` | 79 | **98 passed / 0 failed** (+19) |
| `cargo test --lib ui::screens::detail` | 82 (plan said 74; vqw added tests) | **90 passed / 0 failed** (+8) |
| `cargo test --lib ui::screens::render_escape_guard` | 14 | **14 passed / 0 failed**, Pipeline fixture populated |
| `cargo clippy -- -D warnings` | exit 0 | **exit 0** |
| `cargo test --no-fail-fast` | 2005 passed / 1 failed | **2052 passed / 1 failed** |

The single failure is
`envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`
— the recorded environmental git-version-constants test (installed git 2.53 against the
constants' expectation), a strict subset of the baseline's failing set and not a
regression from this work.

TDD gates observed, both tasks against stub implementations:

- Task 1 RED: 86 passed / 12 failed → GREEN 98 / 0.
- Task 2 RED: 85 passed / 5 failed → GREEN 90 / 0.

## Deviations from Plan

### Auto-fixed issues

**1. [Rule 1 — Bug] `"\u{202e}"` in a junk-value fixture tripped `spawn_seam_guard`**

- **Found during:** Task 3's full `--no-fail-fast` run.
- **Issue:** `tests/spawn_seam_guard.rs::the_degenerate_payload_set_is_spelled_in_exactly_one_place`
  runs a per-file census of `test_support::DEGENERATE` witnesses and reports any hand copy
  outside `src/test_support.rs`. Task 1's junk-`tokens`-value list contained a bare
  `"\u{202e}",` literal, which the census read as a blank-shape payload hand copy.
- **Fix:** Replaced with Arabic-Indic `"\u{0661}\u{0662}"` — a DIGIT that `u64::from_str`
  refuses, which is the parse property that list is actually testing. The guard's
  allow-table was deliberately NOT widened: doing so would exempt a file for a reason that
  does not hold. The escaping property is pinned where it belongs, in
  `detail::tests::a_hostile_plan_label_reaches_no_cell_unescaped`.
- **Files modified:** `src/state_reader/disk_status.rs` (declared).
- **Commit:** `f330b9a`.

### Deliberate departures from the plan's letter

**2. `plan_estimates` / `plan_actuals` are `HashMap<String, u64>`, not `HashMap<String, Option<u64>>`.**
The plan specified the `Option` form. Storing only the `Some` case is behaviourally
identical (a missing key and a stored `None` are read the same way by `.get().copied()`)
and removes a state that can never mean anything. No test distinguishes them.

**3. `matched_plans` became `HashSet<String>` from `HashSet<&str>`.** The pairing loop now
needs the matched plan id to key `plan_actuals` while `plan_ids` is still borrowed;
cloning the id is the smallest change that keeps the borrow checker satisfied.
`summary_count` is still `matched_plans.len()` and no test moved.

**4. `DETAIL_TAB_ARRIVAL`'s `Pipeline tab` reason string was updated.** The plan named only
`hostile_project_state`. Leaving the reason reading "Draws the current phase name, status
and pause context" would have been stale the moment the right pane started drawing —
that row is a reason a later reader inherits, and this module's own doctrine is that a
`true` whose reason has drifted is worse than no row.

**5. Rustfmt.** Only lines this task authored were formatted to `cargo fmt` output. The
repository is NOT rustfmt-clean (pre-existing drift in `app.rs`, `detail.rs:503/1255/1421`,
`disk_status.rs:1240`, `git_ops.rs` and others), and reformatting those would have been an
undeclared-scope edit.

### Baseline corrections

The plan's stated baselines for `ui::screens::detail` (74) and the full suite (2005/1) were
measured before sibling batch item **260916-vqw** merged. Measured here: detail **82**, full
suite unchanged at **2005/1** as the comparison point. Both `done` criteria ("strictly
greater than") hold against the plan's number and against the re-measured one.

## Inferred decisions (for audit)

The human operator was unavailable throughout. The plan's five decisions are carried
forward unchanged and were all honoured as written:

- **D-INF-01 — Numbers live on `DiskInference`, read during the disk scan; the renderer
  reads no files.** Honoured. `build_plan_token_lines` performs no I/O; every number comes
  off `inf`. Phase 19 would otherwise have meant 66 `read_to_string` calls per frame.
- **D-INF-02 — A plan row with NEITHER number is omitted, and an empty `plan_tokens` omits
  the whole section.** Honoured, pinned by
  `test_plans_without_the_keys_leave_plan_tokens_empty` and
  `an_empty_plan_tokens_renders_no_section_at_all`. The header therefore states its own
  denominator (`2/5 measured`).
- **D-INF-03 — The list is CAPPED at 10 rows plus an explicit `... +N more`.** Honoured.
  The real-data smoke confirms phase 19 needs it (33 rows into an unscrolled `Paragraph`).
  Follow-up if it annoys: pane scrolling, not a bigger constant.
- **D-INF-04 — The escape-guard fixture hole is closed here rather than deferred.**
  Honoured and proved fail-first (verbatim red above). Without it the guard would have
  passed while certifying nothing about the new sink.
- **D-INF-05 — `depends_on: []` with an honest `files_modified`.** Held. Nothing this item
  needs comes from a sibling; the overlap on `detail.rs` is a merge-DAG fact only.

Two further judgements taken without the operator, both recorded above as deviations 1 and
4: the `spawn_seam_guard` fix chose to change this task's own fixture rather than widen an
existing guard's allow-table, and the `DETAIL_TAB_ARRIVAL` reason string was updated so the
recorded reason stays true.

## Known Stubs

None. Every function this task added is fully implemented and exercised by a committed
test; no placeholder value, empty collection or TODO reaches the UI. The transient stub
bodies used for the TDD RED steps were replaced in the same tasks and are not present in
any commit.

## Threat Flags

None. The task introduces no network endpoint, auth path or schema change. Its one new
surface — a plan identifier read out of a foreign `.planning/` filename — is
`T-VQX-01` in the plan's own register, disposition `mitigate`, and is mitigated by
`shown()` with the mitigation now measured rather than asserted (Task 3's fail-first
proof). `T-VQX-03`'s bounded summary read is once per refresh, never per frame, and
fail-safe on an unreadable file.

**Ledger note:** `.planning/WINDOWS.md` was NOT appended to. This item runs inside a quick
batch that merges back by declared file set, and the orchestrator owns all `.planning/`
writes other than this summary; appending would have been an undeclared-file scope
violation. There is nothing to record — no stub, no skipped test, no unrun `<verify>`, and
the single deviation is documented above and already fixed.

## Self-Check: PASSED

- `src/state_reader/disk_status.rs` — FOUND (modified)
- `src/ui/screens/detail.rs` — FOUND (modified)
- `src/ui/screens/render_escape_guard.rs` — FOUND (modified)
- `ac0b203` — FOUND
- `853d639` — FOUND
- `f330b9a` — FOUND
- `git rev-list --count 2251792f..HEAD` — 3, matching `commits: 3`
- Working tree clean; no undeclared file created or left behind
