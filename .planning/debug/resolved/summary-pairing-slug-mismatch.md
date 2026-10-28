---
status: resolved
trigger: "Phase summaries with slugless names never pair with slugged plan files, collapsing completed phases to Planned and rendering Execute-skipped next to Verified"
created: 2026-09-10
updated: 2026-09-10
---

# Debug: slugless summaries never pair with slugged plans

## Symptoms

- **Expected:** For `/home/blk/projects/flutter/wordoclock`, phases 13/14/15 hold a complete
  set of plans and summaries plus a passing VERIFICATION, so they must read `Complete` (`+`);
  phase 16 is executed with gaps, so `Executed` (`+`, `[Executed]`); the disk frontier is 17
  and phase 17 must draw `*`.
- **Actual:** Phases 13-16 render `o` (future) badged `[Planned (N plans)]`. In the detail
  view the D-R-P-E-V pipeline shows Execute `Skipped` (`[--]`, magenta) beside Verify
  `Complete` (green) - a self-contradicting row.
- **Errors:** None. A silently wrong render.
- **Timeline:** Present since the matched-summary rule (#1988) was introduced.
- **Reproduction:** Point the dashboard at any GSD project whose plans carry a descriptive
  slug (GSD's actual emitted shape) - `13-01-verse-pipeline-PLAN.md` beside `13-01-SUMMARY.md`.

## Evidence

- timestamp: 2026-09-10 - real filenames on disk, `wordoclock/.planning/phases/13-.../`:
  `13-01-verse-pipeline-fourteen-translations-PLAN.md` and `13-01-SUMMARY.md`. Every phase
  13-17 has this shape: plans slugged, summaries not.
- timestamp: 2026-09-10 - `src/state_reader/disk_status.rs:602-606` derives a plan's id as
  the full stem (`13-01-verse-pipeline-fourteen-translations`); `:649-658` derives a
  summary's id as the full stem (`13-01`) and requires `plan_ids.contains(&id)`.
  The two ids can never be equal for a slugged plan, so `summary_count` is always 0.
- timestamp: 2026-09-10 - `:669` `implementation_complete = summary_count >= plan_count &&
  plan_count > 0` is therefore false, and `:676` collapses the status to `Planned`.
- timestamp: 2026-09-10 - `src/ui/screens/detail.rs:4758-4759`: E reads `summary_count > 0`
  (count-derived), V reads `has_verification` (file presence). `:4777-4779` then marks the
  absent-but-later-present E stage `Skipped`.
- timestamp: 2026-09-10 - baseline `cargo test --no-fail-fast`: 48 suites,
  1973 passed / 2 failed / 14 ignored. Failures are pre-existing: the envelope git-version
  pin and the flaky `a_fresh_scan_finds_the_orphaned_run_live_...`.

## Eliminated

- hypothesis: "two separate bugs" - REJECTED. Both symptoms flow from `summary_count == 0`.
- hypothesis: "the ROADMAP `- [x]` fallback should rescue it" - REJECTED.
  `PhaseMarker::decide` (`src/state_reader/mod.rs:276-282`) only consults the roadmap when
  the phase directory is absent; wordoclock's directories exist.
- hypothesis: "`roadmap_md.rs:405-411`'s `eq_ignore_ascii_case(\"complete\")` is the cause" -
  OUT OF SCOPE by operator ruling. It cannot match wordoclock's
  `Complete - verification passed`, but that file is not to be touched.

## Root Cause

The plan/summary pairing key is derived from two different strings. GSD emits plans with a
descriptive slug and summaries without one, so the full filename stem is not a shared
identity between the pair - the leading `NN-MM` plan index is. Every test fixture in
`disk_status.rs` writes slugless plans (`05-01-PLAN.md`), and this repository's own
`.planning/phases/` does too, so neither the unit tests nor the dogfood could reproduce it.

## Fix (prescribed)

1. Pair on the leading plan index, derived identically on both sides, with the exact-stem
   match kept as the first tier so the bare `PLAN.md` <-> `SUMMARY.md` path is unchanged.
   An index that names more than one surviving plan pairs with nothing.
2. `derive_all_stage_statuses`: drive V from `verification_status` rather than bare
   `has_verification`, and never mark Execute `Skipped`.
3. Add fixtures in GSD's real filename shape.

## Current Focus

hypothesis: CONFIRMED - the pairing key divergence was the sole cause of both symptoms.
test: 13 new tests; the 10 pairing ones and all 3 pipeline ones fail on the reverted bodies.
expecting: 13/14/15 Complete, 16 Executed, frontier 17, phase 17 draws `*`. OBSERVED.
next_action: none - resolved.

## Resolution

root_cause: the plan/summary pairing key was the full filename stem on both sides, but GSD
  slugs plans and not summaries, so the two stems are never equal for a real GSD phase.
  Every surviving summary was discarded, `summary_count` was 0, `implementation_complete`
  was false, and the status collapsed to `Planned`.

fix:
  - `src/state_reader/disk_status.rs:304-332` - new `plan_index()`: the leading `NN-MM` of a
    stem, parsed as two `u32`s so `5-1` and `05-01` are one index.
  - `src/state_reader/disk_status.rs:513-514,636-639,675-702` - a second, shared key recorded
    for surviving plans only, and a two-tier pairing: exact stem first (the original rule,
    verbatim), then the plan index, and only when that index names exactly one surviving
    plan. The count is of PLANS matched, not summary files.
  - `src/ui/screens/detail.rs:4756-4774` - V reads `verification_status != Missing` rather
    than the bare `has_verification` presence flag, matching E's evidence-derived nature.
  - `src/ui/screens/detail.rs:4788-4800` - Execute is never rendered `Skipped`; nothing
    downstream of it can exist without it.
  - `src/ui/screens/detail.rs:947-953` - the `[verified]` badge reads the same evidence.

verification:
  - reproduction: the fix bodies REVERTED in place with the new tests kept - 8 of the 9
    non-pre-existing failures are the reported symptom verbatim (`summary_count` 0 not 4,
    `Planned` not `Complete`, `StageStatus::Skipped` for Execute beside a green Verify).
    Fix restored; all green.
  - tests: baseline `cargo test --no-fail-fast` 1973 passed / 2 failed / 14 ignored over 48
    suites; after, 1986 passed / 2 failed / 14 ignored. +13, all new. Both post-fix failures
    are pre-existing/environmental and pass on a re-run of their own suite, except the
    envelope git-version pin (2.53.0 installed vs 2.43.0 recorded), which failed at baseline
    too.
  - clippy: `cargo clippy -- -D warnings` clean.
  - fmt: the repo is broadly fmt-dirty; every line added here is rustfmt-clean.
  - real data: wordoclock 13/14/15 `Complete`, 16 `Executed`/`GapsFound`, frontier 17,
    `active_phase_number` 17, markers `13:+ 14:+ 15:+ 16:+ 17:* 18:o`.
    picsync `1:+ 2:+ 3:+ 4:+ 5:+ 6:* 7:o 8:o`, IDENTICAL before and after this change.
    This repo `…19:+ 20:+ 21:+ 22:* 23:o`, active 22, IDENTICAL before and after.

files_changed:
  - src/state_reader/disk_status.rs - `plan_index`, the two-tier pairing, 10 new tests
  - src/ui/screens/detail.rs - E/V evidence parity, Execute never skipped, 3 new tests

## Out of scope, reported not fixed

`src/state_reader/roadmap_md.rs:407` matches a phase status with
`eq_ignore_ascii_case("complete")`, which cannot match wordoclock's
`Complete - verification passed`. Left alone by operator ruling. Its visible effect on
wordoclock is `completed_phases/total_phases = 2/6` while the disk says 13/14/15/16 are at
or above `Executed`.
