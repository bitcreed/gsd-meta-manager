---
phase: 20-deterministic-decision-router-run-bounds
source_review: 20-REVIEW.md
fixed_at: 2026-08-19
scope: critical + warning, plus IN-01 and IN-07
findings_in_scope: 12
fixed: 12
rejected: 0
deferred: 6
status: all_fixed
mode: sequential on master, no worktree
---

# Phase 20: Review Fix Report

Every finding in scope was fixed. Nothing was rejected as a false positive: each of the ten
critical/warning findings was reproduced before being fixed, and eight of them are now pinned by a
test **proven to fail against the pre-fix code** (recorded per finding below).

One finding — WR-04 — was resolved in the *opposite* direction from the review's suggested comment.
That is recorded as a deliberate divergence rather than as a rejection; the reasoning is below.

**Gate:** `cargo build` clean, `cargo test` 937 + 217 tests green (all 30 targets), `cargo clippy --
-D warnings` clean, `cargo clippy --all-targets` reports **exactly the 5 pre-existing lints**
(`src/browser.rs:131-133`, `src/project_creator.rs:146`, `src/state_reader/mod.rs:311`). All gates
ran in the **main working tree** on `master` — no worktree was used, so these numbers are
reproducible from the tree as committed. `tests/driver_reattach.rs` passed on this run; it was not
touched.

---

## Fixed

### CR-01 — the run-level wall-clock cap did not bound the run

`9e70962` · `src/driver/bounds.rs`, `src/driver/run.rs`, `tests/driver_iteration_loop.rs`

`bounds::evaluate` runs at exactly one site, immediately before each spawn, so nothing evaluates the
wall clock *during* an iteration — and `iteration_options` handed the executor the fixed three-hour
`ITERATION_WALL_CLOCK_CAP`. The run-level cap was therefore a bound on the *gaps between*
iterations, not on the run.

`bounds::iteration_wall_clock_cap(bounds, elapsed)` now returns
`wall_clock_cap.saturating_sub(elapsed).min(ITERATION_WALL_CLOCK_CAP)`, and the spawn site passes
`run_started_at.elapsed()`. `bounds::resolve` moved a few lines earlier in `execute_run` so the
resolved caps precede the first construction of the options; the value on disk in `RunRecord.bounds`
and the value every iteration is bounded by are now the same one by construction.

**Both conditions the priority note asked for hold:**

1. The resolved cap bounds the run *inside* a long-running iteration. The per-iteration cap is the
   smaller of the ceiling and the run's remaining time, so the last iteration cannot outlive the
   run's cap.
2. The guard test was rewritten to assert on the **resolved value**. The old
   `the_iteration_wall_clock_cap_is_strictly_inside_the_run_level_cap` compared two constants and
   passed against the defect it was named for; the constant comparison survives only as a labelled
   control ("this proves the run-level *reason* stays reportable, and nothing about enforcement").

**Proven to fail pre-fix.** The new end-to-end test
`a_run_cap_smaller_than_one_iteration_bounds_the_iteration_and_not_the_gap_after_it` drives a real
run with `--wall-clock-cap-secs 1` against `fake-claude-slow.sh 20 1 result` (an agent that chatters
for twenty seconds). Reverting only the `min` and re-running:

```
left: String("parked:bounds_wall_clock")   right: "timed_out"
test result: FAILED ... finished in 20.13s
```

That is the defect exactly: the run overran its stated one-second cap by 20× and then reported the
overrun as though it had stopped at the cap. Post-fix the run ends `timed_out` in ~1s. The test
asserts both the outcome and that the whole `drive()` call finishes well inside the stand-in's
twenty seconds, so a fix that reported the right label late would still fail.

### WR-01 — a failed journal write turned a halt into a reported success

`5b2b74c` · `src/driver/run.rs`

The label was derived by re-reading the journal, and `terminal_label` falls back to `outcome_label`
when that read fails — while `record_terminal` swallows a failed `Parked` write with a `warn!`. A
run that halted on `bounds_step_cap` after a successful iteration could therefore write
`outcome: "succeeded_with_changes"`.

`own_terminal_label(&terminal)` is now asked first. It **takes no `&Path`**, so the property is
structural rather than a comment: no journal read, failed or otherwise, can reach the answer for a
run that holds its own reason. The journal read stays reachable for the case it was written for — a
run with no reason of its own, where a park recorded by another process (the hook or guard
re-entries) may still decide the label.

Test: `a_halt_labels_itself_from_memory_and_a_journal_it_never_reads` constructs the exact pair the
bug needed (a halted terminal beside a plausible successful outcome) plus the negative half. This is
the one fix whose test is structural rather than proven-failing: an I/O failure on an open file
descriptor cannot be simulated end to end (the writer holds the handle for the run, so `chmod` does
not reach it), and the pre-fix call site is inline in `execute_run` with no seam to test. The
signature is the guarantee.

### WR-02 — a `rejected` quota event discarded by a later `allowed`

`6b3f3e5` · `src/driver/run.rs`, `src/driver/rate_limit.rs`, `tests/driver_rate_limit.rs`

The drain loop retained the *last* event and `classify` ran once at the end of the iteration. Two
slots now: the first rejection is latched and never overwritten, and it is classified first. The
predicate is `rate_limit::is_rejection`, split out of `classify`, so the drain loop still asks the
payload nothing itself and still reads no clock — the "retained uninspected" property is preserved.

**Proven to fail pre-fix.** `a_rejection_followed_by_an_allowed_event_still_parks_the_run` emits
`rejected{five_hour}` then `allowed{seven_day}`. With the latch removed the run carries on and ends
`bounds_command_repeat` instead of `quota_rejected`.

### WR-03 — `resetsAt` in the past accepted and reported as fact

`111795f` · `src/driver/rate_limit.rs`, `tests/driver_rate_limit.rs`,
`tests/fixtures/transcripts/README.md`

The bound is now asymmetric: the full thirty-day window ahead, and `RESET_MAX_SKEW_BEHIND_SECS`
(five minutes, clock skew only) behind. The both-sides test asserts the asymmetry, with the same
offset accepted ahead and refused behind, so the property comes from observed answers rather than
from comparing the two constants.

The fixture was **not** changed, and the reasoning is recorded in the README: any hardcoded
`resetsAt` recedes into the past, so refreshing it would only postpone the same staleness. The
assertion was added instead — `the_committed_rejection_fixture_parks_a_real_run` now asserts
`resets_at=unknown`, which is the honest report for a replayed capture.

**Proven to fail pre-fix.** With the symmetric bound restored:
`Got: window=seven_day resets_at=2026-08-04T16:00:00Z` — the park detail naming a reset fifteen days
in the past, which is exactly what the review said was demonstrable on disk today.

### WR-04 — the redefinition of `Complete` silently moved the dashboard's current phase

`1033816` · `src/state_reader/mod.rs`, `tests/state_reader_test.rs`

**Decided in the opposite direction from the review's suggested comment, deliberately.** The review
sketched a comment endorsing the new reading ("the dashboard's current phase is the phase that still
needs something"). I chose the other side: the threshold is now `< DiskStatus::Executed` — the first
phase whose *implementation* is unfinished — which restores the pre-20-03 behaviour as an explicit
choice.

Three reasons, in the comment at the assignment site and in the test:

- **The cell must describe the phase its own row names.** `current_phase_status` feeds the dashboard
  row's compact D-R-P-E-V cell, which sits beside a phase label taken from STATE.md's
  `current_phase` — a label GSD advances on execution. A pipeline describing a different phase from
  the label next to it is a worse defect than a pipeline that omits a gate.
- **The unverified reading gets stuck.** GSD writes a VERIFICATION.md only when a phase is verified,
  so on any project that does not run `/gsd:verify-work`, "first non-Complete" pins the dashboard to
  the first executed phase forever. This repository is diligent (phases 14-18 all `passed`), which
  is why the review saw a one-phase shift rather than a stuck cell.
- **Nothing in the phase declared a dashboard change.** Restoring shipped v1.x behaviour is the
  conservative form of "chosen, not inherited".

The verification gate is not lost by this: it surfaces through the needs-human badge (D-24) and, for
the driver, through the DRIVE-05 gate set — both of which read the per-phase inference directly
rather than this summary.

Written as a comparison against the `Ord` 20-03 added, so a variant inserted below `Executed` is
included automatically and one above it is not.

**Proven to fail pre-fix.** The new test asserts over a two-phase fixture in three verification
states (`human_needed`, `passed`, absent). Against `!= Complete` the first arm fails with
`left: Some(Executed) right: Some(Planned)`.

**Residual, out of scope and pre-existing:** when the target phase is itself fully executed (this
repository, today), the frontier lands on the *next* roadmap phase, which may have no directory —
so the cell can still name a phase later than the row's label. That is v1.x behaviour unchanged, and
fixing it means deriving the cell from STATE.md's `current_phase` rather than from a scan, which is
a display decision this sweep should not make unasked.

### WR-05 — a nested `status:` read as the document's status

`299b3ce` · `src/state_reader/disk_status.rs`

`leading_frontmatter_value` trimmed the key, so an indented mapping key was indistinguishable from a
top-level one and the first match won. It now skips any line starting with whitespace and compares
the key untrimmed. This is GSD's own `DEFECT.FRONTMATTER-SCALAR-BROAD-GREP` one level in, and the
function is the single input to `VerificationStatus` — the goal-met predicate and the whole DRIVE-05
gate set.

**Proven to fail pre-fix.** Both new tests fail against the trimmed comparison:
`test_a_nested_status_key_is_not_the_documents_status` (`left: Passed right: HumanNeeded`) and the
non-vacuity half `test_a_top_level_status_after_a_nested_one_is_still_found`
(`left: Unknown("draft") right: Passed`).

### WR-06 — the G15 gate matched by raw string equality

`1e6660b` · `src/driver/router.rs`, `src/state_reader/roadmap_md.rs`

`roadmap_md::extract_phase_id` is the one normaliser, reusing `PHASE_ID` and the `Phase <id>` keyword
form `parse_depends_on` already accepts rather than maintaining a second set of spellings. Both
sides of the comparison go through it via the router's `phase_identity`. Text that names no
identifier keeps its own bytes, so nothing is silently equated with a guess.

Tests: the gate fires for `19`, `Phase 19`, `phase 19`, `**19**`, `#19` and
`19-gitsafe-git-blast-radius-envelope`; the negative half asserts phase 19's row, however spelled, is
not phase 20's gate. Plus a unit test for the extractor over ten written forms and four
non-identifiers. All five alternate spellings fail against the `==` comparison by construction.

The new extractor uses a `OnceLock`-cached regex, which is the shape IN-08 asks `parse_depends_on`
to adopt; IN-08 itself is deferred (below).

### WR-07 — the conformance oracle passed silently without `gsd-tools`

`60187c3` · `tests/driver_router_conformance.rs`

A missing oracle now **fails by default**, naming what to install and the
`GSD_META_MANAGER_ALLOW_MISSING_ORACLE` opt-out for an environment that genuinely cannot run Node.
An empty or whitespace-only value is not consent (`FOO= cargo test` is how a variable is unset by
accident). The real companion, `a_missing_oracle_is_a_failure_unless_the_environment_says_otherwise`,
pins that default over the pure `skip_permitted` predicate; the misleading comment naming
`every_fixture_reaches_the_state_it_is_named_for` as the guard is corrected in both places, and the
file header no longer claims a guard that did not exist.

**Verified both ways** by running the test binary with `HOME` pointed at an empty directory and
`PATH` stripped: the run FAILS without the variable and passes with it. With the oracle present
(this machine) it still runs its 5 real comparisons.

### WR-08 — goal-met reported two different labels

`c02f4fd` · `src/driver/run.rs`, `tests/driver_iteration_loop.rs`,
`tests/fixtures/fake-claude-planting.sh`

`Terminal::GoalMet` is its own arm (five arms, still exhaustive, still no wildcard) and labels
`goal_met` regardless of `last_outcome`. `Terminal::Completed` keeps its "performed everything asked"
meaning, and its stale doc sentence — IN-01's second site — was replaced here.

The end-to-end proof needs iteration one to leave a fact behind for iteration two's router, which no
existing stand-in could do, so `fake-claude-planting.sh` copies a caller-supplied body to a
caller-supplied path before replaying. The fixture states its own premise in Rust; the script
decides nothing.

**Proven to fail pre-fix.** `a_run_that_drives_its_target_to_verified_reports_goal_met_and_not_the_
agents_outcome` plants a passing `20-VERIFICATION.md` during iteration one. With the arm reverted to
`Terminal::Completed`: `left: String("succeeded_with_changes") right: "goal_met"` — the run that
actually achieved the goal reporting the previous command's outcome.

### WR-09 — `--dry-run` skipped the validations it was previewing

`c97cc24`, `62cc45b` · `src/driver/mod.rs`, `src/driver/router.rs`, `tests/driver_dry_run.rs`

Both refusals moved above the dry-run branch, beside `command_source_refusal`, which is already
positioned there for exactly this reason. Both are pure and neither creates anything on disk. The
run-id and platform refusals stay below it (a preview creates no run to identify and starts no
process to stop). `RouterAction::command_for`'s doc claim — "`phase` arrived on argv and was
validated at the seam" — is now true on every path that reaches it, and says so.

Test: `a_preview_refuses_exactly_what_the_real_run_would_refuse` asserts the typed
`DriveError::TargetPhaseInvalid` for `--dry-run --target-phase '../../../escaped'` and
`DriveError::BoundsRefused` for `--dry-run --max-steps 0`. Both were `Ok(())` before the move.

### IN-01 — two `run.rs` docs describing a closed reader gap

`9eb2664` · `src/driver/run.rs`

`drpev_stages`' doc claimed the reader "records only whether a `*-VERIFICATION.md` exists, never its
frontmatter `status`". It now says the V stage *chooses* presence, and why: the five elements are one
positional vocabulary read by index, and the status — which is what decides the DRIVE-05 gate —
reaches the journal through the `parked` reason and its `Diagnostic` detail, where a reader greps for
why a run stopped rather than what its stages looked like. `Terminal::Completed`'s stale sentence was
replaced under WR-08. No `Pitfall 2`-as-open claim remains under `src/`.

### IN-07 — two self-referential tests skipped rather than failed

`86afdeb` · `src/state_reader/roadmap_md.rs`, `tests/state_reader_test.rs`

`expect`/`assert!` replace the silent `return`s. `disk_status.rs`'s early return is left alone: the
phase-19 directory it guards can legitimately be archived away, which the review itself flags as the
justified case.

---

## Rejected

None. Every finding in scope reproduced.

---

## Deferred, with reasons

- **IN-02** (`prev_status` keeps a `_ =>` wildcard) — no behaviour change today; the trigger is the
  *next* `DiskStatus` insertion, and spelling out `NoDirectory`/`Empty` is a UI-file change that
  belongs with whatever makes that insertion.
- **IN-03** (digest scanner audits only `bounds.rs`) — no digest exists on the no-progress path in
  any of the three files; widening a source scan across `src/driver/` risks false positives on
  unrelated hashing and is test-infrastructure work rather than a defect fix.
- **IN-04** (documentation guard pinned on exact line wrapping) — fails only on a rewrap, and fails
  loudly with the disclosure text in the message, so it misleads nobody.
- **IN-05** (`spawn_failed` is a terminal state outside the three DRIVE-06 classes) — pre-dates this
  phase (T-17-06) and touches the spawn-failure path. WR-08 has just added an arm to `Terminal`,
  which makes routing `spawn_failed` through it the natural next change rather than a rider on this
  sweep.
- **IN-06** (dead `unwrap_or` on an infallible `split().next()`) — cosmetic; the surrounding file was
  edited for WR-07 and left otherwise alone deliberately, to keep that commit about the skip path.
- **IN-08** (`parse_depends_on` recompiles two regexes per call) — a consistency defect with no
  observable cost. The new `extract_phase_id` added under WR-06 uses a `OnceLock`, so the pattern
  IN-08 asks for now exists in the same module for whoever hoists the other two.

---

## Files touched

Source: `src/driver/bounds.rs`, `src/driver/run.rs`, `src/driver/rate_limit.rs`,
`src/driver/router.rs`, `src/driver/mod.rs`, `src/state_reader/mod.rs`,
`src/state_reader/disk_status.rs`, `src/state_reader/roadmap_md.rs`.

Tests and fixtures: `tests/driver_iteration_loop.rs`, `tests/driver_rate_limit.rs`,
`tests/driver_dry_run.rs`, `tests/driver_router_conformance.rs`, `tests/state_reader_test.rs`,
`tests/fixtures/fake-claude-planting.sh` (new), `tests/fixtures/transcripts/README.md`.

Not touched: `.planning/STATE.md`, `.planning/ROADMAP.md`, `.planning/WINDOWS.md`,
`tests/driver_reattach.rs`.

---

_Fixed: 2026-08-19 · 13 commits, one per finding · source review: `20-REVIEW.md`_
