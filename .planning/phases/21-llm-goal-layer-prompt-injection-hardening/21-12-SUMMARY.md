---
phase: 21-llm-goal-layer-prompt-injection-hardening
plan: 12
subsystem: driver-guards
status: complete
tags: [gap-closure, guards, docs-accuracy, review-WR-01, review-WR-03]

requires:
  - "21-11: command_source's blankness guard, whose validate-at-the-seam decision this plan's Task 2 enforces"
  - "src/driver/run.rs::finish_run — the stamped terminal-write helper guard six audits"
  - "src/journal/parse_approval_token — the refusal the reworded doc names"
provides:
  - "Guard six scanning every file under src/ with a declared (file, fn) allowlist and two call spellings"
  - "A guard-six header naming each remaining limit with the direction it fails in"
  - "every_command_source_variant_is_named_only_where_it_is_built_or_matched — the single-construction-site guard"
  - "plan_digest's doc naming the refusal production actually produces for a legacy value"
affects:
  - tests/spawn_seam_guard.rs
  - src/driver/goal.rs
  - src/journal/mod.rs
  - tests/driver_goal_seam.rs

tech-stack:
  added: []
  patterns:
    - "(file, fn) allowlist over source_files() with a per-file test_region_start boundary — the register SPAWN_ALLOWLIST and the seam-site guard establish, now applied to two more properties"
    - "Both-directions guard: an unexpected site is a violation, and an allowlisted site that stopped contributing is equally one (T-20-17)"
    - "A shared scanner function driven by both the real guard and its synthetic control arm, so the control arm proves what the guard runs"

key-files:
  created: []
  modified:
    - tests/spawn_seam_guard.rs
    - src/driver/goal.rs
    - src/journal/mod.rs
    - tests/driver_goal_seam.rs

key-decisions:
  - "Widened guard six's scan tree-wide rather than only correcting its header. 21-VERIFICATION.md offered either; widening was checkable — all eight other `.finish(` call sites already sit past their file's column-zero `mod tests {`, so the property held tree-wide and was merely unchecked."
  - "Corrected the header as well as widening, because widening removes one under-approximation and not the others. The three that remain are named individually, each with its own failure direction; no blanket claim about the guard's failure direction survives."
  - "Keyed the CommandSource guard on the three fully-qualified variant spellings with an opening parenthesis, not the bare type name, because src/driver/run.rs:614 declares a second unrelated enum CommandSource. Re-verified by reading both declarations before trusting the needles."
  - "Reworded the legacy-digest doc rather than deleting it: the reader's question ('is there a migration?') is real and the answer is still no. Only the reason changed, from a record path production cannot take to two greppable facts."
  - "Corrected the sibling claim in src/journal/mod.rs as well — 21-REVIEW.md WR-03 named it as the second doc asserting the same wrong mechanism, and reading confirmed it did."

requirements-completed: [DRIVE-04, SAFE-07, SAFE-08]

coverage:
  - deliverable: "Guard six scans every file under src/ and reports exactly one terminal-write hit from one file"
    verification:
      - kind: test
        ref: "tests/spawn_seam_guard.rs#every_terminal_write_in_the_driver_run_goes_through_the_stamped_helper"
        status: pass
      - kind: command
        ref: "red arm: journal.finish(..) injected at src/driver/kill.rs:228 — reported"
        status: pass
    human_judgment: false
  - deliverable: "The UFCS call spelling is matched, closing the second unstated gap 21-REVIEW.md named"
    verification:
      - kind: test
        ref: "tests/spawn_seam_guard.rs#the_terminal_write_scanner_reports_a_bare_call_and_not_the_helpers_own"
        status: pass
      - kind: command
        ref: "red arm: JournalRun::finish(journal, ..) injected at src/driver/run.rs:998 — reported"
        status: pass
    human_judgment: false
  - deliverable: "Guard six's header names each remaining limit with the direction it fails in, and makes no claim about the guard as a whole"
    verification:
      - kind: source
        ref: "tests/spawn_seam_guard.rs guard-six header block"
        status: pass
    human_judgment: true
    rationale: "Prose accuracy. A test can pin that the header exists; whether each limit is labelled with the direction it truly fails in is a reading judgment, and it is the exact judgment review-WR-01 made against the old text."
  - deliverable: "command_source is enforced as the single production constructor of CommandSource, both directions"
    verification:
      - kind: test
        ref: "tests/spawn_seam_guard.rs#every_command_source_variant_is_named_only_where_it_is_built_or_matched"
        status: pass
      - kind: command
        ref: "red arms: third construction site reported; emptied preview_text reported as allowlist-too-wide"
        status: pass
    human_judgment: false
  - deliverable: "plan_digest's doc describes what production actually does with a legacy value, and names the test that exercises that route"
    verification:
      - kind: command
        ref: "rtk proxy grep -rn 'journal::recheck_approval(' src/ — exactly two production call sites, both as the doc describes"
        status: pass
      - kind: test
        ref: "tests/driver_goal_seam.rs#a_half_supplied_approval_token_is_refused_by_name_and_never_treated_as_an_approval"
        status: pass
    human_judgment: true
    rationale: "The greppable facts are checked, but whether the reworded paragraph reads as accurate to a future author — the failure mode WR-03 named — is a reading judgment."
  - deliverable: "No reference to the renamed test's old name survives anywhere in the tree"
    verification:
      - kind: command
        ref: "rtk proxy grep -rn 'a_recorded_approval_carrying_a_legacy' src/ tests/ — no matches (exit 1)"
        status: pass
    human_judgment: false

metrics:
  duration: "13 min"
  started: "2026-08-21T18:17:18Z"
  completed: "2026-08-21T18:30:00Z"
  tasks: 3
  files: 4

actuals:
  tokens: 5687
  tasks: 3
  commits: 3
---

# Phase 21 Plan 12: Guard-Six Widening and Legacy-Digest Doc Correction Summary

Guard six now scans every file under `src/` with a declared `(file, fn)` allowlist and two call spellings, its header names each remaining limit with the direction it actually fails in, `command_source`'s single-constructor property is a checked guard rather than a grep result, and `plan_digest`'s doc names the refusal production really produces for a legacy value.

**Duration:** 13 min (18:17:18Z → 18:30:00Z UTC) | **Tasks:** 3 | **Files:** 4 | **Commits:** 3

## Accomplishments

### Task 1 — Guard six sees the whole tree and says what it cannot see (`8351c16`)

`TERMINAL_WRITE_HOME` (a single file constant) was replaced by `TERMINAL_WRITE_ALLOWLIST`, a declared `(file, fn)` allowlist whose sole entry is `("src/driver/run.rs", TERMINAL_WRITE_HELPER)`. The scan now iterates `source_files()`, computing each file's own `test_region_start` boundary; a file with no column-zero marker is scanned in full, which is correct for a file that declares no in-module tests. The panic for a missing boundary is preserved **for the allowlisted file specifically**, since losing it there would silently scan `run.rs`'s own tests.

A second needle, `TERMINAL_WRITE_UFCS_CALL = "JournalRun::finish("`, was added beside the method-call form. Confirmed by reading `src/journal/mod.rs:1771` that the declaration (`pub fn finish(&mut self, outcome_label: &str)`) matches neither needle, so the definition does not report itself.

Non-vacuity was re-tuned to the widened scope:

| Assertion | Before | After |
|---|---|---|
| `helper_writes == 1` | within `run.rs` only | tree-wide |
| `fn finish_run(` declarations `== 1` | within `run.rs` only | tree-wide |
| distinct contributing files `== 1` | *did not exist* | **new** — the direct statement of the property WR-01 says was unchecked |
| stale-allowlist check | *did not exist* | **new** — second direction, T-20-17's register |
| `uses >= 4` | implicitly file-scoped | explicitly file-scoped, with the reason stated |

The control arm now drives the same `terminal_write_hits` scanner the guard runs, over **two** synthetic files — the second with no test-region marker and carrying the UFCS spelling, exercising both halves of the widening over source the test constructs.

### Task 2 — The `CommandSource` single-construction-site guard (`1f3fa6d`)

New guard `every_command_source_variant_is_named_only_where_it_is_built_or_matched`, keyed on the three fully-qualified variant spellings with an opening parenthesis and allowlisting exactly `("src/driver/mod.rs", "command_source")` and `("src/driver/mod.rs", "preview_text")`. This is the missing half of `21-11`'s design decision: validating a blank `--command` at the seam alone is sound only while `command_source` is the single production constructor.

### Task 3 — `plan_digest`'s doc names the refusal that actually fires (`78b886d`)

The legacy-record claim was replaced in **both** docs that carried it, and the predicate test was renamed to say what it pins.

## Verbatim Evidence

### The name collision, re-verified before trusting the needles

`src/driver/run.rs:614` declares a second, unrelated enum:

```rust
enum CommandSource {
    /// One supplied command, one iteration.
    Fixed(String),
    /// A routed sequence driving toward a phase.
    Routed {
        target_phase: String,
    },
}
```

`Command` and `Goal` do not exist on it, and its `Routed` is a **struct variant** spelled with a brace. The three needles therefore produce zero hits in `run.rs` — asserted explicitly by the guard rather than assumed.

### Guard-six hit table (widened scan)

| File | Function | Hits |
|---|---|---|
| `src/driver/run.rs` | `finish_run` | 1 |
| *(every other file under `src/`)* | — | 0 |

Distinct contributing files: **1**. Tree-wide `fn finish_run(` declarations: **1**. Production `finish_run(` call sites in `src/driver/run.rs`: **4**.

Corroborating grep — all eight other `.finish(` call sites sit past their file's column-zero `mod tests {` (`src/journal/mod.rs:2101`, `src/driver/run.rs:3518`) and are excluded by the boundary, **not** by an allowlist entry:

```
src/driver/run.rs:993:    journal.finish(label)             <- production, inside finish_run
src/driver/run.rs:3966:        run.finish(&label)            <- past 3518
src/driver/run.rs:4154:        journal.finish("succeeded_no_changes")
src/driver/run.rs:4271:        journal.finish("succeeded_no_changes")
src/journal/mod.rs:2814:        run.finish("completed")       <- past 2101
src/journal/mod.rs:3057:        run.finish("parked:bounds_command_repeat")
src/journal/mod.rs:3135:        run.finish(&format!("parked:{reason}"))
src/journal/mod.rs:3166:        run.finish("succeeded_with_changes")
src/journal/mod.rs:3371:        run.finish("succeeded_with_changes")
```

The widened scan was green on the first run with **no new allowlist entry**, exactly as the plan's `<design_decision>` predicted.

### Red arm 1 of 4 — a bare terminal write in a file the old guard could not see

Injected `journal.finish("succeeded_no_changes")` into a new production function in `src/driver/kill.rs`:

```
running 1 test
test every_terminal_write_in_the_driver_run_goes_through_the_stamped_helper ... FAILED

thread 'every_terminal_write_in_the_driver_run_goes_through_the_stamped_helper' (258775) panicked at tests/spawn_seam_guard.rs:1850:5:
a terminal write in a production region under src/ does not go through `finish_run`. That helper stamps the run's model consultation count before it finishes the journal, and it is the ONLY place a run's ending is written so that a new terminal path inherits the stamp rather than having to remember it. A bare `.finish(` — or the fully-qualified `JournalRun::finish(` — writes `escalations_used: null` on a run that spent consultations, which reads identically to a record from a build that predates the counter, on exactly the killed and failed-to-spawn runs a reader is auditing (WR-02, DRIVE-04). `JournalRun::finish` is `pub` on a `pub struct`, so this is reachable from any module in the tree and not only from the driver. Offending lines:
  src/driver/kill.rs:228: journal.finish("succeeded_no_changes")

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 23 filtered out; finished in 0.06s
```

This is the direction the old, file-scoped guard was **silent** in.

### Red arm 2 of 4 — the UFCS spelling

Injected into `src/driver/run.rs`'s production region, outside the helper, so only the *spelling* dimension is under test (the old guard scanned this file, and still would have missed it):

```
running 1 test
test every_terminal_write_in_the_driver_run_goes_through_the_stamped_helper ... FAILED

thread 'every_terminal_write_in_the_driver_run_goes_through_the_stamped_helper' (259886) panicked at tests/spawn_seam_guard.rs:1850:5:
a terminal write in a production region under src/ does not go through `finish_run`. [...] Offending lines:
  src/driver/run.rs:998: JournalRun::finish(journal, "succeeded_no_changes")

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 23 filtered out; finished in 0.07s
```

### Red arm 3 of 4 — a third `CommandSource` construction site

```
running 1 test
test every_command_source_variant_is_named_only_where_it_is_built_or_matched ... FAILED

thread 'every_command_source_variant_is_named_only_where_it_is_built_or_matched' (269725) panicked at tests/spawn_seam_guard.rs:2239:5:
a production line names a `CommandSource` variant outside the two sanctioned functions. What a third site costs: `command_source` validates a blank `--command` and refuses it, and the renderer deliberately does NOT defend again, because two places answering one question are two places that can disagree. That reduction is sound ONLY while `command_source` is the single production constructor — a second construction site is review-CR-02 returning through a new spelling, with every behavioural test still green. If the new site is a legitimate consumer that matches rather than builds, add it to COMMAND_SOURCE_ALLOWLIST in the same commit and say which it is. Offending lines:
  src/driver/mod.rs:448: CommandSource::Goal(goal.to_string())

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 24 filtered out; finished in 0.09s
```

### Red arm 4 of 4 — the other direction, an allowlisted site emptied

`preview_text` temporarily rewritten to name no variant:

```
running 1 test
test every_command_source_variant_is_named_only_where_it_is_built_or_matched ... FAILED

thread 'every_command_source_variant_is_named_only_where_it_is_built_or_matched' (271286) panicked at tests/spawn_seam_guard.rs:2270:9:
src/driver/mod.rs::preview_text is allowlisted as a site that names all 3 `CommandSource` variants, but the scan attributes only 0 lines to it. Either the allowlist is now wider than the truth it describes — the site stopped naming them, and the entry should go in the same commit — or a needle stopped matching and this guard is auditing less than it claims. Needles: ["CommandSource::Command(", "CommandSource::Routed(", "CommandSource::Goal("]

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 24 filtered out; finished in 0.07s
```

All four injections were reverted with `git checkout -- <file>`; `git status --short` showed only the intended test file modified, and `rtk proxy cargo build --all-targets` was clean afterwards.

### The reworded `plan_digest` paragraph, verbatim (`src/driver/goal.rs:750-775`)

```rust
/// The tree still has exactly **two** digest functions and gains no third; this
/// is simply the one with an adversary, so it is the one that reaches for
/// `sha256_digest`.
///
/// # A legacy `fnv1a64:` value needs no migration
///
/// Two facts, and both are greppable at the commit that ships this paragraph.
///
/// **First, there is no record path to migrate.** No recorded
/// [`crate::journal::ApprovedPlan`] is ever deserialised and re-checked:
/// `RunRecord::approved_plan` is written and never read back into
/// [`crate::journal::recheck_approval`]. That predicate has exactly two
/// production call sites, and neither reads a record off disk — `approve_plan`
/// in `src/driver/mod.rs` compares the halves of a token parsed off argv **in
/// the same invocation**, and the spawn gate in `src/driver/run.rs` passes the
/// in-memory `ApprovedPlan` produced by the same run.
///
/// **Second, the legacy value a user could actually still be holding is a token
/// on argv** — the single-digest value an earlier dev build printed — and it
/// never reaches a digest comparison at all.
/// [`crate::journal::parse_approval_token`] refuses it as
/// `ApprovalTokenError::SeparatorAbsent` before any half is compared, because a
/// value carrying one half is not half an approval. That route fails closed by
/// name, and
/// `tests/driver_goal_seam.rs::a_half_supplied_approval_token_is_refused_by_name_and_never_treated_as_an_approval`
/// is the test that exercises it.
```

### `recheck_approval`'s production call sites, so the doc can be checked against the tree

```
$ rtk proxy grep -rn 'journal::recheck_approval(' src/
src/driver/run.rs:2322:        journal::recheck_approval(
src/driver/mod.rs:965:    journal::recheck_approval(
```

Exactly two, and both hold values from the run in progress:

- `src/driver/mod.rs:965` (`approve_plan`) — `Some((recorded_plan_digest, recorded_approval_digest))`, the halves of a token parsed off argv in the same invocation.
- `src/driver/run.rs:2322` (the spawn gate) — `Some((&approved.plan_digest, &approved.approval_digest))`, the in-memory `ApprovedPlan` from the same run.

Neither deserialises a record. `RunRecord::approved_plan` (`src/journal/mod.rs:1422`) is written and never read back into the predicate — confirmed by grep across `src/`, whose only reads are in test modules.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 3 — Blocker] `flat_map` closure moved a captured `String`**

- **Found during:** Task 1
- **Issue:** The tree-wide declaration scan captured `declaration: String` in a `move` closure inside `flat_map`, which `rustc` rejected with `E0507: cannot move out of 'declaration', a captured variable in an 'FnMut' closure`.
- **Fix:** Replaced the iterator chain with a plain nested loop pushing into a `Vec`. Same result, no clone, and it reads closer to the surrounding code.
- **Files modified:** `tests/spawn_seam_guard.rs`
- **Verification:** `rtk proxy cargo test --test spawn_seam_guard` — 24 passed.
- **Commit:** `8351c16`

### Deliberate, minor

**2. [Transparency] One inline comment in the renamed test's body was adjusted**

- **Found during:** Task 3
- **Issue:** The plan says to keep every **assertion** in the body unchanged. Every assertion is unchanged. But an inline comment above the fixture read *"A record written by a build that hashed the plan half with FNV-1a-64"*, which re-asserts the same record framing the task exists to remove — one line below a doc comment now saying production never deserialises such a record.
- **Fix:** Reworded to *"The plan half in the format a build that hashed it with FNV-1a-64 would have produced. Hand-built, because no production path deserialises one"*. No assertion, value or behaviour touched.
- **Files modified:** `tests/driver_goal_seam.rs`
- **Commit:** `78b886d`

**3. [Scope] The `every_terminal_write_in_the_driver_run_goes_through_the_stamped_helper` name was kept**

The test now scans the whole tree, so its name under-describes it. It was kept because `<artifacts_this_phase_produces>` lists it explicitly as *rewritten* under that name, and an under-claiming name is not the defect class this plan closes (over-claiming is). Flagged here rather than silently changed, since a future round may want to rename it.

**Total deviations:** 1 auto-fixed (Rule 3 — blocker), 2 documented minor. **Impact:** none on behaviour; no production logic changed anywhere in this plan.

## Threat Flags

None. No new network endpoint, auth path, file access pattern or schema change. The only production files touched are doc comments in `src/driver/goal.rs` and `src/journal/mod.rs`; no signature, format or behaviour moved.

## Known Stubs

None.

## Verification

| # | Check | Result |
|---|---|---|
| 1 | `rtk proxy cargo build --all-targets` | clean |
| 2 | `rtk proxy cargo clippy -- -D warnings` | clean |
| 3 | `rtk proxy cargo test --test spawn_seam_guard` | **25 passed**, 0 failed (HEAD was 24; +1 from guard eight) |
| 4 | `rtk proxy cargo test --test driver_goal_seam` | **20 passed**, 0 failed — unchanged from `21-11` (a rename, not an addition) |
| 5 | `rtk proxy cargo test --workspace -- --test-threads=4` | **1316 passed, 0 failed** across 35 binaries (baseline after wave 1: 1315; +1) |
| 6 | `rtk proxy grep -rn 'a_recorded_approval_carrying_a_legacy' src/ tests/` | no matches (exit 1) |
| 6b | `rtk proxy grep -rn 'fnv1a64_plan_digest_re_checks_as_stale' src/ tests/` | no matches (exit 1) — checked separately because both docs wrapped the old name across two lines, so a single-line grep alone would have missed them |
| 7 | Four red-arm demonstrations | all four fired; recorded verbatim above |

The documented pre-existing flake `envelope_tracer::a_relocated_copy_of_the_stub_refuses_instead_of_acting` (`deferred-items.md`) did **not** fire in the workspace run.

`rtk proxy cargo doc --no-deps --document-private-items` produces no warning referencing any item touched by this plan (`plan_digest`, `approved_plan`, `parse_approval_token`, `recheck_approval`, `ApprovalTokenError`). The repository's other rustdoc warnings are pre-existing and out of scope.

**Every count-bearing check in this plan was run through `rtk proxy`**, per the project's global CLAUDE.md warning that the hook-rewritten path filters build and test output — the failure mode wave 1 hit concretely, where a filtered `grep -c` returned `0` for a symbol that existed.

## Requirements Completed

`DRIVE-04`, `SAFE-07`, `SAFE-08` — all three were already ✓ SATISFIED in `21-VERIFICATION.md`. **No new behaviour is claimed for any of them.** They are attached because this plan touches their artifacts: guard six is DRIVE-04's `escalations_used` stamp guard; the reworded doc is SAFE-07's approval binding; and `src/driver/goal.rs` is the home of SAFE-08's re-parse.

## Issues Encountered

None.

## Next Phase Readiness

Both Warning-level gaps `21-VERIFICATION.md` recorded against `92a4b4f` are closed:

- **review-WR-01** — guard six's scan covers every file under `src/`, and its header names each remaining limit with the direction it fails in.
- **review-WR-03** — `plan_digest`'s doc (and its sibling in `src/journal/mod.rs`) names the refusal production actually produces for a legacy value, and the test it names is the one that exercises that route.

Plus the wave-2 addition: `21-11`'s validate-at-the-seam decision now rests on a checked property rather than a grep result.

Phase 21 is ready for re-verification.

## Self-Check: PASSED

- `tests/spawn_seam_guard.rs` — FOUND
- `src/driver/goal.rs` — FOUND
- `src/journal/mod.rs` — FOUND
- `tests/driver_goal_seam.rs` — FOUND
- `8351c16` — FOUND in `git log --oneline --all`
- `1f3fa6d` — FOUND in `git log --oneline --all`
- `78b886d` — FOUND in `git log --oneline --all`
- All task `<acceptance_criteria>` re-run and passing; plan-level `<verification>` items 1-7 all recorded above.
