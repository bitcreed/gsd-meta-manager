---
phase: 21-llm-goal-layer-prompt-injection-hardening
plan: 13
subsystem: driver
tags: [security, type-safety, argv-validation, gap-closure]

requires:
  - "src/driver/mod.rs::command_source (the single argv resolution site)"
  - "src/journal/mod.rs::is_plain_path_component (the path-component seam)"
provides:
  - "src/driver/mod.rs::payload::NonBlank (blank-proof argv payload, private field, single constructor)"
  - "CommandSource::{Command,Routed,Goal}(payload::NonBlank)"
  - "src/driver/mod.rs::ALL_VARIANT_NAMES (single spelling of the variant list)"
  - "src/driver/mod.rs::visibly_empty_numbered_entry (independent visible-content detector)"
  - "src/driver/goal.rs::GoalRefusal::empty_plan (pub(super) named constructor)"
affects:
  - "src/driver/goal.rs (legality's control-char comment; a falsified premise in tests/driver_goal_seam.rs)"
  - "src/error.rs (NoCommandSource doc + Display)"
  - "src/cli.rs (--approved-plan help)"

tech-stack:
  added: []
  patterns:
    - "newtype with a private field in a NESTED module, so the defining module's own ancestors cannot bypass the constructor"
    - "test oracle deliberately expressed differently from the production predicate, so the test can falsify the guard"
    - "uniform enumeration with no exemption cell and no disjunction"

key-files:
  created: []
  modified:
    - src/driver/mod.rs
    - src/journal/mod.rs
    - src/driver/goal.rs
    - src/error.rs
    - src/cli.rs
    - tests/driver_dry_run.rs
    - tests/driver_goal_seam.rs

key-decisions:
  - "D-13-1 blank payloads unrepresentable via payload::NonBlank rather than a fourth per-arm trim"
  - "D-13-2 'blank' means no visible instruction — whitespace, control, or zero-width/format characters"
  - "D-13-3 is_plain_path_component additionally refuses whitespace-only and control-carrying values"
  - "D-13-4 a supplied-but-blank source in ANY position refuses rather than falling through"
  - "approve_plan refuses a stepless plan with goal::legality's own EmptyPlan error, via a new pub(super) constructor, rather than widening the private GoalRefusal::new"

requirements-completed: [DRIVE-01, DRIVE-03]

coverage:
  - deliverable: "A blank --target-phase is refused identically in preview and real mode, creating nothing"
    verification:
      - kind: test
        ref: "tests/driver_dry_run.rs#a_blank_target_phase_is_refused_in_preview_and_in_a_real_run"
        status: pass
      - kind: command
        ref: "target/debug/gsd-meta-manager drive demo --config cfg.json --target-phase '   ' --dry-run -> exit 1"
        status: pass
    human_judgment: false
  - deliverable: "CommandSource payloads are blank-proof by type (private field, nested module)"
    verification:
      - kind: command
        ref: "cargo build --all-targets: every in-module test construction failed to compile until routed through NonBlank::new"
        status: pass
      - kind: test
        ref: "src/driver/mod.rs#every_command_source_refuses_or_previews_cleanly_for_every_degenerate_payload"
        status: pass
    human_judgment: false
  - deliverable: "The degenerate matrix is uniform, exemption-free, and coverage-tied to ALL_VARIANT_NAMES"
    verification:
      - kind: test
        ref: "src/driver/mod.rs#every_command_source_refuses_or_previews_cleanly_for_every_degenerate_payload"
        status: pass
      - kind: command
        ref: "rtk proxy grep -c 'deliberately absent' src/driver/mod.rs -> 0"
        status: pass
    human_judgment: false
  - deliverable: "The matrix oracle is independent of the production predicate"
    verification:
      - kind: command
        ref: "visibly_empty_numbered_entry's body contains no NonBlank reference and no tail.trim()"
        status: pass
    human_judgment: false
  - deliverable: "is_plain_path_component refuses whitespace-only and control-carrying values, with all prior acceptances intact"
    verification:
      - kind: test
        ref: "src/journal/mod.rs#only_a_single_plain_component_is_accepted_as_a_run_id"
        status: pass
      - kind: test
        ref: "src/driver/mod.rs#a_blank_run_id_is_refused_without_touching_disk"
        status: pass
    human_judgment: false
  - deliverable: "A blank target_phase cannot enter ApprovedPlan from the model seam"
    verification:
      - kind: command
        ref: "rtk proxy grep -c 'unwrap_or_default' src/driver/mod.rs -> 0; no panic!/expect in approve_plan"
        status: pass
    human_judgment: true
    rationale: "The refusal path is unreachable today (legality refuses a stepless plan first), so no test drives it; the change is verified structurally rather than behaviourally."
  - deliverable: "No run.json written by this build carries target_phase or gsd_command blank"
    verification:
      - kind: command
        ref: "backstop — the seam refuses before any file is created; reproduced on the built binary with .planning/meta-manager absent afterward"
        status: pass
    human_judgment: true
    rationale: "A backstop truth about every possible record, not an enumerable property; evidenced by seam refusal plus the artifact-absence assertions."

duration: ~85 min
completed: 2026-08-21
---

# Phase 21 Plan 13: Blank Argv Payloads Made Unrepresentable Summary

Closed round-4 CR-01 at the **type** level rather than the arm level: a `NonBlank`
newtype with a private field in a nested `mod payload` makes a blank
`CommandSource` payload unrepresentable, so the invariant three consecutive
cycles fixed one arm at a time is now written once and enforced by the compiler.

- **Duration:** ~85 min
- **Tasks:** 3 (1 tracer, 2 execute)
- **Commits:** 4 (the tracer split RED/GREEN per this phase's own precedent)
- **Files modified:** 7

## Red-arm evidence (REQUIRED — observed, not reconstructed)

The tracer test was written first and run against unmodified code at
`6eb1d49`. Two failures were observed, in this order.

**First run**, with the payload list ordered `["", "   ", "\t", "\n  \n"]` — the
empty string is refused by the *older*, weaker `TargetPhaseInvalid` seam, so
this run named the wrong defect:

```
running 1 test
test a_blank_target_phase_is_refused_in_preview_and_in_a_real_run ... FAILED

---- a_blank_target_phase_is_refused_in_preview_and_in_a_real_run stdout ----

thread 'a_blank_target_phase_is_refused_in_preview_and_in_a_real_run' (607846) panicked at tests/driver_dry_run.rs:618:13:
the refusal must be the SAME typed variant on both paths, and it must be the command-source one — a preview and a real run must answer an invocation-shape question identically (WR-09). blank="" dry_run=true gave: TargetPhaseInvalid { target_phase: "" }
```

**Second run**, after reordering so `"   "` leads (the payload the round-4
verification actually reproduced). This is the headline red — `expect_err`
failed because `drive()` returned `Ok(())`, and the test's stdout carried an
entire clean dry-run preview of a run with no phase to drive toward:

```
thread 'a_blank_target_phase_is_refused_in_preview_and_in_a_real_run' (608785) panicked at tests/driver_dry_run.rs:617:60:
a target phase made of nothing names no phase to drive toward, on both paths: ()

failures:
    a_blank_target_phase_is_refused_in_preview_and_in_a_real_run

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 13 filtered out; finished in 0.06s
```

The trailing `: ()` is `drive`'s own `Ok(())` value, printed by `expect_err`.

**A second, unplanned red arm — the compiler's.** After changing the three
`CommandSource` variants to carry `payload::NonBlank`, `cargo build
--all-targets` failed at **10 sites**, every one of them an in-module test
construction. This is the mechanism proving itself: `driver/mod.rs`'s own
`#[cfg(test)] mod tests` could not fabricate a blank payload, and rustc even
spelled out why the obvious workaround is unavailable —

```
error[E0308]: mismatched types
    --> src/driver/mod.rs:1887:35
1887 |             CommandSource::Routed("20".to_string()),
     |             --------------------- ^^^^^^^^^^^^^^^^ expected `NonBlank`, found `String`
help: try wrapping the expression in `driver::payload::NonBlank` (its field is private, but it's local to this crate and its privacy can be changed)
```

Had the module been flattened to `driver/mod.rs`'s top level, that suggestion
would have compiled and the guarantee would have been worthless.

## Accomplishments

### Task 1 (tracer) — `NonBlank`, and the `Routed` arm's refusal as a consequence

`mod payload` is nested inside `driver/mod.rs`, and the nesting is the entire
load-bearing element: Rust field privacy is scoped to the defining module and
its **descendants**, never its ancestors. `NonBlank` has a private `String`, two
methods (`new`, `as_str`), and `new` is the only route in. "Blank" means **no
visible instruction** — whitespace, control characters, or the zero-width/format
ranges `U+200B..U+200F`, `U+2060..U+2064`, `U+FEFF` — deliberately wider than
`str::trim`, which `U+200B` survives untouched.

All three `command_source` `Ok` arms now build through `NonBlank::new`; the
`Routed` arm gains its refusal as a consequence of the type, not as a fourth
remembered check. Arm order and precedence are unchanged, including the
blankness-must-not-demote-ambiguity rule.

### Task 2 — the matrix made uniform, the oracle made independent, the predicate tightened

- **The exemption is gone.** Every `DEGENERATE` payload in every position is
  `Err(NoCommandSource)`, in one loop with no `Ok` branch. The old shape offered
  each cell a *disjunction* ("refused OR previewed cleanly") and then re-asserted
  the strict rule by name for only two of three columns, exempting
  `--target-phase` in a comment. The Critical walked between the two.
- **`DEGENERATE` grew to 6** (`U+200B`, `U+FEFF` added) and its doc stopped
  praising the shared-`trim` coupling that made it a tautology (WR-03).
- **`empty_numbered_entry` → `visibly_empty_numbered_entry`**, with its own
  character classes rather than `tail.trim()`, so it can disagree with production.
- **`ALL_VARIANT_NAMES`** is the single spelling of the variant list, read by
  both consumers (IN-02).
- **`PositionBuilder`'s 3-tuple** is documented as the column axis's structural
  exhaustiveness — a fourth argv parameter breaks every builder at compile time.
- **The boundary is pinned on both sides**: one visible character resolves in
  every position, including `"\u{200b}x"` and `"x\u{feff}"`, so the rule is
  visibility rather than length.
- **`is_plain_path_component`** refuses blank and control-carrying values, with
  a both-directions pin block (5 acceptances first, then 15 refusals).

### Task 3 — the same-class edges

`NoCommandSource`'s Display names all three sources and states that an
all-invisible value counts as absent; `approve_plan` refuses a stepless plan
instead of defaulting `""` into `ApprovedPlan.target_phase`; the
`recorded_approval` block and the `--approved-plan` help both name the
validated-on-every-invocation widening; `the_git_fixture_is_available` makes the
file's `repo()` skip guards non-vacuous.

## Verification

The round-4 reproduction, re-run against the built binary with a valid opt-in
record:

```
=== A: --target-phase '   ' --dry-run ===
Error: a run needs something to do: pass `--command <c>` ... A value made only
of whitespace or invisible characters counts as absent — it names nothing to
run, and recorded it would read as a missing field
EXIT=1

=== B: --target-phase '   ' REAL run ===
(same message)
EXIT=1

=== C: control — --target-phase 20 --dry-run ===
DRY RUN — nothing below was executed. ...
EXIT=0

=== artifacts === no .planning/meta-manager (GOOD)
```

Also refused on the binary: `--target-phase` carrying a lone U+200B (the payload
that defeats `trim`), `--command` carrying a lone U+FEFF, and `--run-id '   '`
(refused as `RunIdInvalid`). Written here as codepoint names rather than as the
literal characters, so this file stays greppable and the characters cannot be
mistaken for whitespace by a later reader.

| Gate | Result |
|---|---|
| `cargo build --all-targets` | clean |
| `cargo clippy --lib -- -D warnings` | clean |
| `cargo clippy --all-targets` | 5 warnings — the same 5 pre-existing, none new |
| `cargo test --workspace -- --test-threads=2` | **1319 passed, 0 failed** (baseline 1316, +3) |
| `cargo test --workspace -- --test-threads=4` | 1317 passed, 2 failed — the pre-existing `driver_reattach` flake (see below) |
| `rtk proxy grep -c "deliberately absent" src/driver/mod.rs` | 0 |
| `rtk proxy grep -c "unwrap_or_default" src/driver/mod.rs` | 0 |
| `spawn_seam_guard` | 25 passed (guard eight green, needles still matching) |

Every count-bearing check above was run through `rtk proxy`.

## Deviations from Plan

**[Rule 1 — falsified pin] The predicate tightening broke an existing pinned
premise, and it was corrected rather than silently rewritten.**
Found during: Task 2. `tests/driver_goal_seam.rs:1373` asserted that
`is_plain_path_component` **ACCEPTS** control-carrying phase tokens — true when
written, and the express reason `goal::legality` needed a control-character
bound of its own. D-13-3 makes the predicate refuse them, so the premise became
false and the test failed. The plan instructed me to stop and record rather than
rewrite, so: the assertion is **inverted** (the token must now be refused), and
the cost is stated in place — for those four fixtures the goal layer's own bound
is now defence in depth, so the test can no longer distinguish which layer
refused. What it still pins uniquely is refusal-not-repair and the sanitized
`offending()` value, neither of which the predicate answers. The same commit also
corrects `src/driver/goal.rs:659-663`, a comment that asserted the same
now-false fact. **Files:** `tests/driver_goal_seam.rs`, `src/driver/goal.rs`
(neither in the plan's `files_modified`). **Commit:** `8184e8b`.

**[Rule 2 — missing prerequisite] `GoalRefusal::new` is private, so Task 3(b)
could not "reuse the same error" as written.**
Found during: Task 3. The plan says to reuse the error `goal::legality` raises
for an empty plan, but `GoalRefusal::new` is `fn new` (private to the `goal`
module) and `driver/mod.rs` cannot call it. Widening it to `pub(crate)` would let
any caller bypass the bounding it performs on the offending value. Instead
`goal.rs` gained a narrow `pub(super) fn empty_plan()` producing exactly the
refusal `legality` raises. `new` stays private; the two seams cannot drift.
**Commit:** `7ba4e53`.

**[Rule 1 — my own bug, caught before commit] The blank-run-id test's control arm
would have started a real run.**
Found during: Task 2. I added a control arm calling `drive()` with a *valid* run
id, not noticing that the module's `args()` helper defaults to `dry_run: false`.
It hung the suite spawning a real agent. There is no way to write that control
through `drive`: the run-id check sits *below* the dry-run branch, so
`dry_run: true` never reaches it, and `dry_run: false` with a passing id is a
real run. The control was moved one level down, to the predicate itself, with the
reason stated in the test. Flagging it because the plan did not ask for a control
arm at all — this was my addition, and it was wrong on the first attempt.

**[process] The tracer task produced two commits, not one.**
This phase's own history splits tracer work into `test(21-NN): add failing …`
then `feat(21-NN): …` (see `bb4c812`/`585cb40`, `94492f8`/`e2b1222`). Splitting
makes the red arm a reviewable artifact rather than a quotation. Tasks 2 and 3
are one commit each.

**Total deviations:** 4 (2 plan-instruction deviations, 1 self-caught bug, 1
process). **Impact:** no change to the plan's objective or to any `must_haves`
truth; one plan instruction (3b) was implemented by a different mechanism that
achieves the stated property more narrowly.

## Issues Encountered

**`tests/driver_reattach.rs` is far flakier on this machine than
`deferred-items.md` records, and I proved it is not a regression.** Two of its
three tests fail intermittently under `--test-threads=4`. `deferred-items.md`
documents this flake but says `--test-threads=1` returned ok 3/3; here it fails
under `-- --test-threads=1` too. Because the parent's stated baseline was a clean
1316/0, I did not take the documentation on trust:

- I built the **unmodified baseline** (`6eb1d49`) in a throwaway worktree and ran
  the binary six times: `ok, FAILED, FAILED, FAILED, FAILED, FAILED`. The
  untouched tree flakes *worse* than mine.
- The run ids it uses (`2026-07-29T12-00-00Z-aaaa`) are exactly the shape my
  `is_plain_path_component` change pins as **accepted**, so the tightening cannot
  be the cause.
- On my tree the same binary alone gave `FAILED, ok, ok`; the whole suite ran
  fully green twice at `--test-threads=4` earlier in this plan, and green at
  `--test-threads=2`.

Not a regression, and not fixed here (the file is in no `21-*` plan's `<files>`).
Worth noting for the next phase that its documented mitigation
(`--test-threads=1`) no longer works.

## Self-Check: PASSED

All `<acceptance_criteria>` re-run; all plan `<verification>` commands re-run.
`.planning/REQUIREMENTS.md` is absent from every commit in this plan
(`8762a64`, `b019da7`, `8184e8b`, `7ba4e53`) — confirmed per-commit with
`git diff --stat`.

Ready for `21-14`.
