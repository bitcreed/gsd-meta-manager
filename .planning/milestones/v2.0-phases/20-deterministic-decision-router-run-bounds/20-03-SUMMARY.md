---
phase: 20-deterministic-decision-router-run-bounds
plan: "03"
subsystem: state_reader
status: complete
tags: [state-reader, disk-status, verification, gates, roadmap, drive-05, drive-02]
requires:
  - src/state_reader/disk_status/infer_disk_status
  - src/state_reader/parse_project_state
  - src/state_reader/roadmap_md/parse_roadmap_phases
provides:
  - src/state_reader/disk_status::DiskStatus::Executed
  - src/state_reader/disk_status::VerificationStatus
  - src/state_reader/disk_status::UatStatus
  - "DiskInference: verification_status, uat_status, continue_here_blocking"
  - "ProjectState: continue_here_present, deferred_verification_phases"
  - "RoadmapPhase: depends_on"
  - src/state_reader/state_md::is_error_status
  - src/state_reader/state_md::deferred_verification_phases
affects:
  - src/state_reader/disk_status.rs
  - src/state_reader/mod.rs
  - src/state_reader/state_md.rs
  - src/state_reader/roadmap_md.rs
  - src/driver/router.rs
  - src/ui/screens/detail.rs
  - src/ui/screens/normal.rs
  - src/app.rs
  - src/browser.rs
  - src/change_tracker.rs
tech-stack:
  added: []
  patterns:
    - "one byte-zero-anchored frontmatter parse, shared by every artifact reader"
    - "tolerant &str classification with an Unknown/Other arm carrying the observed value"
    - "collect names -> sort -> read the first, as a determinism tie-break"
    - "gate predicates over existing fields rather than new fields"
    - "exhaustive match with no wildcard as the migration mechanism for an enum variant"
key-files:
  created: []
  modified:
    - src/state_reader/disk_status.rs
    - src/state_reader/mod.rs
    - src/state_reader/state_md.rs
    - src/state_reader/roadmap_md.rs
    - src/driver/router.rs
    - src/ui/screens/detail.rs
    - src/ui/screens/normal.rs
    - src/app.rs
    - src/browser.rs
    - src/change_tracker.rs
    - tests/state_reader_test.rs
decisions:
  - "Executed is INSERTED between Partial and Complete; two assertions pin the position"
  - "Complete is now a conjunction: implementation complete AND verification passed"
  - "the D-R-P-E-V stage thresholds did NOT move; only prev_status did"
  - "the phase continue-here gate parses a Severity column by index, never a substring"
  - "parse_depends_on strips parenthetical groups BEFORE extracting identifiers"
  - "is_error_status is a predicate over the existing status field, not a new field"
  - "archived phases keep their Complete short-circuit and a Missing verification status"
metrics:
  duration: ~1h30m
  completed: 2026-08-19
actuals:
  tokens: 21000
  tasks: 3
  commits: 3
---

# Phase 20 Plan 03: The One Reader — GSD's Vocabulary and the Gate Set Summary

The reader the dashboard and the driver share now speaks GSD's eight-state
vocabulary, reads the verification *status* rather than the artifact's presence,
and can express every human-judgement gate that leaves a trace on disk.

## What Was Built

**`DiskStatus::Executed`, inserted between `Partial` and `Complete`.** Not
appended — `Ord` is derived from declaration order, and an appended variant would
sort above `Complete` and silently reorder every comparison in the tree while
still compiling and still passing every equality test.
`test_disk_status_ordering` asserts `Partial < Executed` and
`Executed < Complete` with an assertion message naming that hazard by name.

**`Complete` corrected into a conjunction.** The old first derivation arm was
GSD's `implementation_complete` predicate verbatim (`init.cjs:181`), so this
repository's `Complete` meant GSD's `executed`. It now yields `Executed`, and a
new arm above it yields `Complete` only when that predicate holds **and** the
verification status is `passed` — `init.cjs:194-195`'s rule. Against this
repository's own planning directory, phase 19 moved from `Complete` to
`Executed`, which is the whole point: its `19-VERIFICATION.md` reads
`status: human_needed`.

**`VerificationStatus`** — the six `VERIFICATION_ROUTING_TABLE` values plus an
`Unknown(String)` arm carrying the observed bytes verbatim, matched as a string
with an explicit fallback in the posture `executor::outcome` already uses. It sits
on `DiskInference` beside the kept `has_verification`, whose doc now states that
presence and status answer different questions and that only the status can
express a gate.

**One byte-zero-anchored frontmatter parse, now shared.**
`leading_frontmatter_value` replaced the bespoke scan inside
`plan_frontmatter_superseded`, which now calls it — one anchor implementation
rather than two that can drift. The anchor is the mitigation for T-20-12: GSD's
own verification library records a defect where a broad search false-matched
`status:` inside a fenced code block, and two tests reproduce that document shape
and assert `Missing`.

**Determinism tie-break.** Verification and UAT artifacts are collected into
vectors, sorted, and the first is read — `verification.cjs:302-303`'s rule. A test
writes two verification artifacts with different statuses and reads eight times,
because the hazard is directory-iteration order and it need not differ on any
single read.

**The rest of the disk-observable gate set**, with conditions taken from the
research document's enumerated table rather than restated from prose:

| Gate | Where | Shape |
|---|---|---|
| G7 outstanding UAT | `DiskInference::uat_status` | typed status + `is_outstanding()` |
| G10 root continue-here | `ProjectState::continue_here_present` | **content** check |
| G11 error/failed status | `state_md::is_error_status` | predicate over the existing field |
| G13 phase continue-here | `DiskInference::continue_here_blocking` | blocking-severity **row** |
| G15 deferred verification | `ProjectState::deferred_verification_phases` | named, not counted |

**`RoadmapPhase::depends_on`**, parsed from the entry's `**Depends on**:` line.

## Key Decisions

**The phase continue-here gate parses a `Severity` column by index, and a
substring search was never on the table.** This repository already carries a
stale `.continue-here.md` in the phase-19 directory whose every severity row
reads `advisory` — and whose prose contains the word "blocking" exactly once, as
part of the filename `tests/async_blocking_guard.rs`. Both an existence check and
a substring search would fire on it and park every run against this project
forever. `test_phase_19_stale_continue_here_marker_is_not_blocking` reads that
real file and asserts the gate stays shut; a companion fixture reproduces the
same shape (a "blocking"-containing string inside an advisory row) so the property
survives the day phase 19 is archived.

**`parse_depends_on` strips parenthetical groups before extracting anything.**
Phase 20's own line is `Phase 16, Phase 17, Phase 19 (and Phase 22 must land
before this phase closes)`, and promoting `22` out of that aside would state a
dependency the roadmap does not. The same strip disarms a sharper trap:
`Nothing (no v2.0 dependencies — parallel-safe, can ship any time)`, where a
bare-number scan matches `2.0` inside the version string `v2.0` and invents a
phase that will never exist — an unsatisfiable dependency condition, forever.
Requiring the `Phase` keyword is the second narrowing, and it also leaves
`Phases 17-21` as prose (the plural leaves no whitespace after `Phase`).

**The D-R-P-E-V stage thresholds did not move; only `prev_status` did.** The
tempting change was to repoint the `E` stage from `Partial` to `Executed`. It is
wrong: it would render `E` dark for a `Planned` phase and break the pinned
`test_compact_pipeline_stage_colors_preserved`. Repointing only
`prev_status(Complete)` from `Partial` to `Executed` gives the correct reading
with a one-line change — an `Executed` phase now shows `E` green and `V` yellow
("verification is next up"), pinned by a new test. The one behaviour shift is
that a `Partial` phase's `V` went from yellow to dark grey, which is more honest:
verification is not next up when execution has not finished.

**`is_error_status` is a predicate, not a field.** The plan is explicit that a
gate reading of `error`/`failed` is the router's disposition to make, not the
reader's to encode. It matches exactly rather than by substring, so a
`stopped_at` of "failed to reach the registry" or a status of "recovered from
error" is not a gate.

**Archived phases keep their `Complete` short-circuit and a `Missing`
verification status.** A phase archived into a milestone shipped, and the archive
is the corroboration; `Missing` is the honest value for a file nobody read, not a
claim that it passed. The repo-level invariant test admits this case explicitly
(`is_passed() || !has_verification`).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 - Blocking] Four files outside the declared `files_modified` needed
`depends_on: Vec::new()`**
- **Found during:** Task 3
- **Issue:** `RoadmapPhase` does not derive `Default` and is built by exhaustive
  struct literal in `src/app.rs` (2 sites), `src/browser.rs` (3),
  `src/change_tracker.rs` (1) and `src/driver/router.rs` (1). A new field is a
  compile error at every one.
- **Fix:** Added the field to each literal. All seven are test fixtures or
  synthetic-state constructors; no behaviour changed. Checked against plan 20-02's
  declared file list first — no overlap, so the merge stays clean.
- **Commit:** 3e948d2

**2. [Rule 1 - Now-false assertions] Five in-source tests pinned the old
`Complete` meaning**
- **Found during:** Task 1
- **Issue:** `test_all_summaries_returns_complete`,
  `test_fix_summary_not_counted`, `test_superseded_plan_excluded_from_counts`,
  `test_normal_two_plan_two_summary_still_complete` and
  `test_standalone_summary_requires_standalone_plan` all asserted `Complete` for
  directories with no verification artifact — which is exactly the collapse this
  plan removes.
- **Fix:** Assertions moved to `Executed`; the two whose *names* claimed
  completeness were renamed
  (`..._returns_executed_not_complete`, `..._is_executed`) in the same commit as
  the code, per CONVENTIONS.md:75. The counting behaviour each test exists to
  pin (FIX/GAPCLOSURE exclusion, superseded plans, standalone pairing) is
  untouched.
- **Commit:** f3f0784

**3. [Rule 2 - Missing critical functionality] `UatStatus` was not named in the
plan's artifact list**
- **Issue:** The plan names the field `uat_status` and calls for "a typed UAT
  status", but lists no type for it. Modelling it as a bare `String` plus a
  free-floating blocking-set check would have put the vocabulary in one place and
  the predicate in another.
- **Fix:** `UatStatus` mirrors `VerificationStatus` exactly — closed arms for
  `uat-predicate.cjs:30-32`'s six blocking values, an `Other(String)` arm
  carrying anything else verbatim, and `is_outstanding()` as the single
  definition of the gate.
- **Commit:** 1f6b725

### Judgement Calls Worth Review

**4. The tasks were committed atomically rather than as RED/GREEN pairs.** Every
task in this plan is a *type extension* — a new enum variant, new struct fields
on `DiskInference`, `ProjectState` and `RoadmapPhase`. A test-only RED commit
referencing `DiskStatus::Executed` or `inference.verification_status` before the
type carries them does not compile, so the RED commit would be a broken commit in
history rather than a failing test. Tests and the code they cover landed in one
commit per task; the tests were written from the plan's `<behavior>` block before
the implementation in each case, so the specification order held even though the
commit granularity did not.

**5. The repo-level test asserts the DRIVE-05 *invariant*, not phase 19's current
values.** The plan's `<verification>` block asks that the reader report phase 19
as `Executed` with a `human_needed` status. Pinning those literal values in a test
would turn a future `/gsd-verify-work 19` into a build failure for an unrelated
reason. `test_this_repositorys_own_planning_dir_reads_without_panicking` instead
sweeps every phase and asserts the property itself — no phase reads `Complete`
while carrying a non-passing verification — which catches the same regression and
cannot rot. The literal values were confirmed out-of-band and are recorded under
Verification below.

**6. `Decision::GoalMet` still has no producer, and its doc was rewritten rather
than deleted.** Plan 20-01 recorded the blocker as "the reader records only
presence"; that blocker is now gone. What remains is a routing choice that plan
20-04 owns, and the doc now says so — a reader who arrives there finds a decision
point rather than a stale excuse. `RouterReason::DependencyUnsatisfied` is in the
same position, and `depends_on` is now the input it was waiting for.

## Known Stubs

| Stub | File | Reason / who resolves it |
|---|---|---|
| `Decision::GoalMet` has no producer | `src/driver/router.rs` | The reader blocker is closed. Which of `DiskStatus::Complete` and a passing verification the goal is stated against is a routing decision, and 20-04 owns the rule table. |
| `RouterReason::DependencyUnsatisfied` has no producer | `src/driver/router.rs` | `RoadmapPhase::depends_on` is now readable; the rule that gates on it is 20-04's. |
| `DiskStatus::Executed` routes to `NoRule` | `src/driver/router.rs` | Covered by its own explicit arm with a comment, not by falling into the catch-all. GSD routes `executed` to `verify`; 20-04 writes that row. |
| `UatStatus`, `continue_here_blocking`, `continue_here_present`, `deferred_verification_phases`, `is_error_status`, `depends_on` have no consumer yet | reader | Every one is read by tests and by nothing else. This plan's declared job is to make them *observable*; 20-04 is the consumer. Not stubs in the "returns empty data to a UI" sense — each returns real parsed values. |
| `VerificationStatus::Stale` is never *derived* | `src/state_reader/disk_status.rs` | Only recognised when written literally into frontmatter. GSD derives it from a summary being newer than the verification artifact (`verification.cjs:94-98`); that mtime comparison is not in this plan's scope. Named here because it is the partial detector for T-20-13. |

None prevents this plan's goal.

## Deferred Items

- **G3 staleness derivation** (a `*-SUMMARY.md` newer than the
  `*-VERIFICATION.md`). The status is read when present; the mtime comparison
  that *produces* it is unimplemented. This matters beyond completeness: the
  research document names it the partial detector for T-20-13, the accepted
  residual risk that a driven agent writes a passing status to unpark itself.
- **G6 `staleCheckIndeterminate`**, **G8 per-item UAT results**, **G12
  unresolved `FAIL` items with no override**, and **G14 incomplete prior phase**
  are not modelled. G14 is derivable today from `plan_count`/`summary_count`;
  the other three need per-item parsing this plan did not scope.
- **G16-G20** are `AskUserQuestion` gates inside milestone/cleanup workflows and
  leave no disk trace, so they are outside "disk-observable" by construction.
- **The `Complete` semantic change has no UI-level test for the dashboard badge
  path.** `needs_human_for` and the alias-badge logic live outside this plan's
  file list; a phase that now reads `Executed` may deserve the `⚑` needs-human
  badge and does not get one.

## Threat Flags

None. Every file touched is a read path over `.planning/` artifacts already read
by this module; no network surface, no auth path, no schema at a trust boundary
was introduced. T-20-12 (frontmatter false-match) and T-20-14 (malformed
directory) are mitigated as the register requires; T-20-13 remains accepted and
disclosed above.

## Verification

| Gate | Result |
|---|---|
| `rtk proxy cargo build` | clean, no warnings |
| `rtk proxy cargo test --test state_reader_test` | **37 passed**, 0 failed |
| `rtk proxy cargo test --lib disk_status` | 67 passed |
| `rtk proxy cargo test --lib roadmap_md` | 29 passed |
| `rtk proxy cargo test` | **27 targets, 0 failures** (878 lib + every suite) |
| `cargo clippy -- -D warnings` (documented gate) | clean |
| `rtk proxy cargo clippy --all-targets -- -D warnings` | exactly the **5** pre-existing lints, same locations (`state_reader/mod.rs` line moved 258→279 — same item, same lint, shifted by the added fields) |

**Reader run against this repository's own planning directory**, out-of-band via
a temporary probe (reverted; tree clean):

```
phase19 status=Executed verification=HumanNeeded has_verification=true
        uat=Other("deferred") outstanding=false
        continue_here_blocking=false root_marker=false deferred=["19"]
```

Every line of the plan's `<verification>` block, confirmed: phase 19 reads
`Executed` with a human-needed status, its stale phase-directory marker is not
blocking, its `deferred` UAT does not gate, and the deferred-verification table
names it. No new module reads `.planning/` — every addition is on the existing
scan.

**Flaky-test advisory checked deliberately.** `tests/driver_reattach.rs` went red
once and `tests/driver_lock.rs` once, each on a *different* full-suite run, and
each passed in isolation immediately after at its normal duration (6.12s and
6.10s) versus the failing runs' 0.53s and 30.75s. That is the signature phase
19's own anti-pattern table describes — an order-of-magnitude timing anomaly
means the work never happened, not that it raced. A subsequent full run was green
across all 27 targets. Nothing in this plan touches the driver's lock, spawn or
reattach paths.

Every `rtk proxy` above is deliberate: `rtk` strips cargo's `warning:` and `test
result:` lines, so a grep of those against bare `cargo` succeeds vacuously.

## Self-Check: PASSED

Modified files verified present and non-empty: `src/state_reader/disk_status.rs`,
`src/state_reader/mod.rs`, `src/state_reader/state_md.rs`,
`src/state_reader/roadmap_md.rs`, `src/driver/router.rs`,
`src/ui/screens/detail.rs`, `src/ui/screens/normal.rs`,
`tests/state_reader_test.rs`.
Commits verified in `git log`: `f3f0784`, `1f6b725`, `3e948d2`.
Working tree clean; no file deletions in any commit
(`git diff --diff-filter=D` empty for all three).

## Notes on `actuals`

`tokens: 21000` is chars/4 over the realized diff (83,778 characters across 11
files, measured with `rtk proxy git diff` — the rtk-filtered `git diff` reports
27,468 and would understate it by 3×). The estimate was 42,000 at `confidence:
low`, so the plan came in at roughly half. The gap is almost entirely that the
three tasks turned out to be one shape repeated — collect, sort, parse the
leading frontmatter, thread a field through one struct literal — so Tasks 2 and 3
reused Task 1's helper rather than each adding a parser.
