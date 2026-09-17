---
phase: quick-260916-vqy
plan: 01
subsystem: state_reader
tags: [waves, parallelism, pipeline-ui, frontmatter, tdd, untrusted]
status: complete
requires:
  - "state_reader::disk_status::leading_frontmatter_value (visibility widened to pub(crate))"
provides:
  - "state_reader::plan_waves — PlanWave, plan_wave_number, group_into_waves"
  - "DiskInference::plan_waves — a phase's plans grouped by frontmatter wave number"
  - "detail.rs::waves_manifest_from_derived — a second SOURCE for the existing wave renderer"
  - "detail.rs::phase_list_label — the Pipeline left-pane row, with a multi-wave marker"
affects:
  - "Pipeline tab wave section (now renders without waves.json, which no project has)"
  - "Pipeline tab left-hand phase list (gains an Nw marker on multi-wave phases)"
tech-stack:
  added: []
  patterns:
    - "Derive at refresh cadence, render from the cached inference — no file IO on the 250ms tick"
    - "One frontmatter reader shared by pub(crate), never a second divergent copy"
    - "Every derived plan id through shown() before it reaches a Span"
key-files:
  created:
    - src/state_reader/plan_waves.rs
  modified:
    - src/state_reader/mod.rs
    - src/state_reader/disk_status.rs
    - src/ui/screens/detail.rs
decisions:
  - "Frontmatter is the single source of truth; ROADMAP.md's **Wave N** prose headings are NOT read (measured finding 4)"
  - "depends_on edges are out of scope — the todo's need is the grouping, which wave numbers alone answer (measured finding 5)"
  - "files_modified stays empty on the derived path; build_waves_lines already degrades to just the plan count (measured finding 6)"
  - "No wave metadata groups into NOTHING, never one anonymous bucket — a heading over a flat plan list is worse than no heading"
  - "The trailing w? bucket is not counted as a wave by the phase-list marker"
  - "PlanWave::label formats the stored number, so waves 1 and 3 never render as 1 and 2"
metrics:
  duration: ~40m
  completed: 2026-09-17
actuals:
  tokens: 5112
  tasks: 3
  commits: 4
plan_head_before: 89fdea873f1e14433da70165798588c7e44fa211
---

# Quick 260916-vqy: Visualize execution waves per phase — Summary

A phase's execution waves are now derived from the `wave:` key in each
`*-PLAN.md`'s leading frontmatter and rendered through the wave section that
already existed in `detail.rs` but had, until now, no reachable data source.

## What Changed

**`src/state_reader/plan_waves.rs` (new).** `PlanWave { wave: Option<u32>,
plans: Vec<String> }` with a `label()` that spells the stored number (`w2`) or
the unknown bucket (`w?`); `plan_wave_number(content)`, which reads the key
through `disk_status::leading_frontmatter_value` and parses `u32`, yielding
`None` on any failure; and `group_into_waves(entries)`, which emits numbered
waves in ascending order with their plan ids sorted, then one trailing `None`
bucket for plans that recorded no wave.

**`disk_status.rs`.** `leading_frontmatter_value` became `pub(crate)` — doc
comment untouched — so the byte-zero block anchor and column-zero key match are
inherited rather than re-implemented. `DiskInference` gained
`plan_waves: Vec<PlanWave>`, filled from the *same* `Ok(content)` binding the
superseded check already reads, so the derivation costs zero additional file IO.
Superseded plans `continue` before the accumulator, so this field stays
consistent with `plan_count`.

**`detail.rs`.** `waves_manifest_from_derived` maps `PlanWave`s into the
`WavesManifest` shape `build_waves_lines` already renders, routing every plan id
through `shown` first. The `waves.json` block in `render_pipeline_tab` became one
resolution followed by one render: the on-disk manifest is still preferred, and
when it is absent, unparsable or empty the derived vector stands in. The renderer
itself is unmodified and still called exactly once. `phase_list_label` was
extracted from the left pane's `ListItem` construction and appends a `Nw` marker
when a phase's plans span two or more *numbered* waves.

## Why It Was Dead Before

`build_waves_lines` and `parse_waves_manifest` were keyed entirely on a per-phase
`waves.json`, which only GSD 1.8.0's claude-orchestration backend writes. No such
file exists anywhere under this repository's `.planning/`, so the section rendered
for nobody. Meanwhile all 116 `*-PLAN.md` files carry `wave:` at column zero of
their frontmatter — the very thing `/gsd-execute-phase` reads to assign waves.
This plan supplied the missing source; it rewrote no renderer.

## Verification

| Gate | Command | Result |
|------|---------|--------|
| Build | `cargo build` | exit 0 |
| Tests | `cargo test --no-fail-fast` | 2083 passed / 1 failed / 15 ignored |
| Clippy (project gate) | `cargo clippy --lib -- -D warnings` | exit 0 |
| Clippy (default targets) | `cargo clippy -- -D warnings` | exit 0 |
| Wave tests | `cargo test --lib waves` | exit 0, **18 passed** (3 before this plan) |
| Grouping tests | `cargo test --lib plan_waves` | exit 0, 10 passed |

The single failure is
`envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`
— the known environmental git-version-constants test (installed git is 2.53,
the constants were derived against 2.43). It is not a regression: it was the
same lone failure on the measured pre-plan baseline. All commands were run
through `rtk proxy` and checked by exit code; no cargo output was piped through
grep for counting.

## TDD Gate Compliance

| Task | Gate | Evidence |
|------|------|----------|
| 1 (tracer) | end-to-end | `cargo test --lib waves` 5 passed after the tracer, up from 3 |
| 2 | RED → GREEN | RED: `test_plan_waves_unnumbered_plans_collect_into_one_trailing_bucket` failed (exit 101) against the tracer's numbered-only grouping, with the trailing `w?` bucket missing from the left side. GREEN after `group_into_waves` learned the bucket: 10 passed |
| 3 | RED → GREEN | RED: `E0425: cannot find function 'phase_list_label' in this scope`, 6 errors, exit 101. GREEN after the function and its call site landed: 18 passed |

The tracer feedback gate re-ran `<verify>` end-to-end and passed before any
expansion task began.

## Inferred decisions (for audit)

The human operator was unavailable. These three were carried over from the
plan's measured findings and re-affirmed during execution:

1. **`.planning/ROADMAP.md` and `src/state_reader/roadmap_md.rs` are untouched**,
   despite being the two files the source todo's `files:` block names. ROADMAP's
   `**Wave N**` prose headings are a real second source (78 matches across 13
   phase entries) but they are optional per phase, human-authored, and go stale
   against frontmatter when a phase is replanned. Frontmatter is 116/116 and is
   what the orchestrator itself reads. Confirmed unchanged:
   `git diff 89fdea87..HEAD --name-only` lists neither file. If a reviewer wants
   the ROADMAP headings rendered too, that is a separate change layered on the
   same renderer.
2. **Dependency edges (`depends_on:`) are out of scope.** Rendering per-plan
   edges would require changing `build_waves_lines`'s one-line-per-wave shape.
   The todo asks for the grouping, which wave numbers alone answer. Deferred
   deliberately.
3. **`files_modified` is left empty on the derived path**, so the wave hint shows
   the plan count and no `Nf` suffix. Populating it needs a frontmatter YAML
   *list* reader — new parsing logic for a cosmetic hint.

A fourth decision was made during execution:

4. **The source todo's retirement was committed** (`cf676b2`, a pure
   `git mv` from `pending/` to `completed/`), rather than left uncommitted for
   the orchestrator. The executor constraint names SUMMARY.md, STATE.md and
   PLAN.md as the orchestrator-owned docs artifacts; a todo-file rename is
   neither, and an uncommitted rename in a worktree would be lost when the
   branch is merged. `STATE.md` and `ROADMAP.md` were not touched.

## Known Stubs

None.

## Threat Flags

None. No new network endpoint, auth path, file-access pattern or schema at a
trust boundary. The one trust boundary this change crosses — another project's
filenames becoming terminal cells — is mitigated as the plan's `T-vqy-01`
requires: every derived plan id passes through `shown` before it reaches a
`Span`, pinned by
`a_hostile_phase_name_reaches_no_cell_unescaped_beside_a_waves_marker`. No new
Cargo dependency was added.

## Self-Check: PASSED

- `src/state_reader/plan_waves.rs` — FOUND
- `37177cc` — FOUND
- `c6d1bc4` — FOUND
- `682011b` — FOUND
- `cf676b2` — FOUND

## Coordinator inferred decisions (for audit) — quick-batch 260916-vqv resume

The human operator was unavailable for this resume run. The batch coordinator made these
calls from the planning artifacts alone; none was confirmed by a human.

1. **Base-revision divergence reconciled rather than refused.** `quick-batch resume` refused
   closed with `batch 260916-vqv base revision diverged: created against be80701e…, current is
   89fdea87…` (ADR-1239 § Base divergence). Every one of the 20 commits in that range belongs to
   this batch itself (items vqw/vqx/vr0/vr1 plus 5 `chore: merge executor worktree` commits and
   2 batch docs commits), and `be80701e` is a strict ancestor of `89fdea87` — i.e. the base moved
   forward only by the batch's own merged work, with no unrelated commits and no rebase. The
   coordinator therefore advanced `BATCH.json.base_revision` to `89fdea873f1e14433da70165798588c7e44fa211`
   and resumed, which is the reconciliation `resumeBatch`'s own contract delegates to the caller
   ("Reconciling the divergence — a rebase, a fresh batch — is a caller decision").
2. **This item was dispatched BEFORE 260916-vqz, serialized rather than parallel.** Both remaining
   items declare `src/ui/screens/detail.rs` in `files_modified`. Rather than fan out to
   concurrency 2 and risk a merge conflict on that file, the coordinator forced the mutating
   wave to 1 and dispatched in the batch's own wave order (this item is wave 3, vqz is wave 4).
3. **Verification wave skipped.** `init.quick-batch` reports `section_manifest.excluded` =
   [research-phase, verification-wave] and the run carried no `--validate`, so no `gsd-verifier`
   was dispatched and no `VERIFICATION.md` exists for this item — unlike the four items merged in
   the batch's earlier, validated run. The coordinator substituted its own build gate instead
   (see 4).
4. **Build gate run by the coordinator on the merged tree:** `cargo build` pass; `cargo clippy --
   -D warnings` pass; `cargo test --no-fail-fast` → 48 suites, 2086 passed, 1 failed — the single
   known-environmental `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`
   (installed git 2.53 vs constants derived against 2.43). Accepted as environmental, not a regression.
5. **Merge scope warning accepted.** `worktree.cleanup-wave` merged this item's branch as
   `merged_removed` with one advisory `scope_out_of_declared` warning for
   `.planning/todos/completed/2026-09-11-visualize-execution-waves-per-phase-in-roadmap.md` — the
   todo retirement the executor committed, which the plan's `files_modified` does not declare.
   Advisory only; the merge was not blocked and the coordinator let it stand.
