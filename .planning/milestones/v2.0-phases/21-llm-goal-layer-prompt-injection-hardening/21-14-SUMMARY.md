---
phase: 21-llm-goal-layer-prompt-injection-hardening
plan: 14
subsystem: driver
tags: [guards, honesty, refactor, gap-closure]

requires:
  - "21-13 (the payload newtype; guard eight's needles must still match the rewritten command_source arms)"
provides:
  - "tests/spawn_seam_guard.rs::no_production_item_follows_a_test_module_marker (the boundary self-check)"
  - "tests/spawn_seam_guard.rs::ITEM_OPENERS"
  - "src/driver/run.rs::IterationSource (renamed from the colliding CommandSource)"
  - "guard six header limits 4 and 5; guard eight limits block (5 numbered limits)"
  - "guard eight's single-declaration assertion and variant-import assertion"
affects:
  - "src/state_reader/mod.rs (count_backlog_items relocated above the mod tests marker)"
  - ".planning/phases/21-.../deferred-items.md (round-4 adjudications)"

tech-stack:
  added: []
  patterns:
    - "convert a textual guard's SILENT under-detection into a LOUD over-detection by asserting its blind region is empty"
    - "resolve a name collision by renaming, never by allowlisting the colliding file"
    - "an unreachable match arm refuses with a typed error rather than manufacturing a sentinel"

key-files:
  created: []
  modified:
    - tests/spawn_seam_guard.rs
    - src/driver/run.rs
    - src/state_reader/mod.rs
    - .planning/phases/21-llm-goal-layer-prompt-injection-hardening/deferred-items.md

key-decisions:
  - "D-14-1 rename run.rs's enum CommandSource -> IterationSource"
  - "D-14-2 the (None, None) arm returns a typed Err instead of Fixed(String::new())"
  - "D-14-3 items-after-marker is a build failure tree-wide"
  - "D-14-4 three round-2 carryovers adjudicated OUT with recorded reasons"
  - "DEVIATION: the (None,None) refusal stamps the journal via finish_run before returning, matching the spawn-failure arm, because the journal is already open at that point"

requirements-completed: [DRIVE-04, SAFE-07, SAFE-08]

coverage:
  - deliverable: "Guard six's marker-to-EOF blind region is provably empty and self-checking"
    verification:
      - kind: test
        ref: "tests/spawn_seam_guard.rs#no_production_item_follows_a_test_module_marker"
        status: pass
      - kind: command
        ref: "cargo clippy --all-targets: items_after_test_module cleared, 5 -> 4 warnings"
        status: pass
    human_judgment: false
  - deliverable: "Guard six's header names limits 4 and 5 with failure directions"
    verification:
      - kind: command
        ref: "rtk proxy grep -c 'END OF FILE|end of file' tests/spawn_seam_guard.rs -> 3"
        status: pass
    human_judgment: true
    rationale: "Whether a prose limits block is honest and complete is a reading judgment, not a machine-checkable property; the verifier should read limits 4 and 5 directly."
  - deliverable: "Guard eight carries a limits block and closes its two cheapest gaps"
    verification:
      - kind: test
        ref: "tests/spawn_seam_guard.rs#every_command_source_variant_is_named_only_where_it_is_built_or_matched"
        status: pass
      - kind: command
        ref: "COMMAND_SOURCE_VARIANTS contains 6 needles; scan attributes 6 hits, 3 to command_source and 3 to preview_text"
        status: pass
    human_judgment: false
  - deliverable: "Exactly one enum CommandSource is declared under src/"
    verification:
      - kind: command
        ref: "rtk proxy grep -rn 'enum CommandSource' src/ | wc -l -> 1; grep -c in run.rs -> 0; enum IterationSource -> 1"
        status: pass
      - kind: test
        ref: "tests/spawn_seam_guard.rs#every_command_source_variant_is_named_only_where_it_is_built_or_matched (single-declaration assertion)"
        status: pass
    human_judgment: false
  - deliverable: "execute_run's source re-derivation can no longer manufacture a blank command"
    verification:
      - kind: command
        ref: "rtk proxy grep -c 'Fixed(String::new())' src/driver/run.rs -> 0; no panic!/unreachable! in the arm"
        status: pass
    human_judgment: true
    rationale: "The arm is unreachable (drive refuses both-or-neither first), so no test drives it; verified structurally. The added finish_run stamp is likewise unexercised — see Issues."
  - deliverable: "DRIVE-04's escalation cap and terminal-write properties still hold"
    verification:
      - kind: test
        ref: "tests/driver_escalation_cap.rs (8 tests, unchanged and passing)"
        status: pass
      - kind: test
        ref: "tests/spawn_seam_guard.rs#every_terminal_write_in_the_driver_run_goes_through_the_stamped_helper"
        status: pass
    human_judgment: false
  - deliverable: "SAFE-07's untrusted boundary and arrival-evidence field are unregressed"
    verification:
      - kind: test
        ref: "tests/driver_injection_corpus.rs (12 passed, 10 ignored, unchanged)"
        status: pass
      - kind: test
        ref: "tests/spawn_seam_guard.rs#the_arrival_evidence_field_is_named_only_where_the_schema_declares_it"
        status: pass
    human_judgment: false
  - deliverable: "SAFE-08's enum-refusal property is unregressed by the rename"
    verification:
      - kind: test
        ref: "tests/driver_refusal_record.rs (9 tests, unchanged and passing)"
        status: pass
    human_judgment: true
    rationale: "Flagged assumption per the plan's edge_probe_audit: SAFE-08's unclassified probe row stays unresolved. The suite passing unchanged is non-regression evidence, not proof of the property."

duration: ~45 min
completed: 2026-08-21
---

# Phase 21 Plan 14: Honest Guards and One Name Per Type Summary

Made the guard layer honest about what its textual scans cannot see, emptied the
blind region both guards shared, and removed the two structural smells
`21-PREMISES.md` adjudicated against: a name collision that was shaping a
guard's needles, and an "unreachable" match arm that manufactured the exact
blank value this phase exists to refuse.

- **Duration:** ~45 min
- **Tasks:** 3 (1 tracer, 2 execute)
- **Commits:** 4 (tracer split RED/GREEN, per this phase's precedent)
- **Files modified:** 4

## Red-arm evidence (REQUIRED — observed, not reconstructed)

The boundary self-check was written first and run against the tree with
`count_backlog_items` still in place. It named the predicted violation exactly:

```
running 1 test
test no_production_item_follows_a_test_module_marker ... FAILED

---- no_production_item_follows_a_test_module_marker stdout ----

thread 'no_production_item_follows_a_test_module_marker' (802812) panicked at tests/spawn_seam_guard.rs:2042:5:
a column-zero item declaration follows a file's `mod tests {` marker. Every scan in this file skips from that marker to END OF FILE, so an item down there is invisible to guard six's terminal-write audit and to guard eight's construction-site audit — both would report green about a region they never read. Move the item ABOVE the marker (which also clears clippy's `items_after_test_module`), or, if a post-marker item is genuinely wanted, delete this assertion in the same commit and say what the guards are giving up. Offending lines:
  src/state_reader/mod.rs:530: pub fn count_backlog_items(planning_dir: &Path) -> u32 {

failures:
    no_production_item_follows_a_test_module_marker

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 25 filtered out; finished in 0.06s
```

`src/state_reader/mod.rs:530`, 219 lines past its marker at :311 — as WR-01
predicted. **Exactly one offender tree-wide**, which is the useful second
finding: the blind region was inhabited, but only once.

## Accomplishments

### Task 1 (tracer) — the blind region made loud, then emptied

`no_production_item_follows_a_test_module_marker` scans every file with a
`mod tests {` marker for column-zero item declarations after it, against an
explicit `ITEM_OPENERS` token list. `count_backlog_items` moved byte-for-byte
above the marker (now :312, marker at :331), next to its caller's region — no
logic change. `cargo clippy --all-targets` dropped **5 → 4** warnings, exactly as
predicted; the remaining 4 are three `assert_eq!`-with-literal-bool in
`src/browser.rs` and one owned-instance comparison in `src/project_creator.rs`,
neither file opened this round.

Guard six's header gained limits **4** (the marker-to-EOF skip: under-detection,
silent, now *bounded* by the new assertion) and **5** (`enclosing_fn` has no
brace tracking: under-detection, silent, named rather than fixed).

I added a non-vacuity control the plan did not ask for: the assertion requires
≥10 files to carry a marker. Without it, re-spelling `TEST_REGION_MARKER` would
empty the scan and the emptiness assertion would hold forever — the exact
failure mode the test exists to close.

### Task 2 — the rename, the typed arm, guard eight

`run.rs`'s `enum CommandSource` → `IterationSource`; all 7 sites plus the doc.
The tree now declares exactly one `CommandSource`.

The `(None, None)` arm no longer constructs `Fixed` around a fresh empty string
— it refuses with `DriveError::NoCommandSource`.

Guard eight: 6 needles (the `Self::`-qualified forms join), the collision
watchdog replaced by a **positive** single-declaration assertion plus a specific
pin that `run.rs` declares none, a new assertion making a variant-path `use`
import loud, and a 5-limit block each with a failure direction.

### Task 3 — the adjudications recorded

`deferred-items.md` gains a "Round-4 adjudications" section: three OUT items
with reasons, and the one IN item with its disposition.

## Verification

| Gate | Result |
|---|---|
| `cargo build --all-targets` | clean |
| `cargo clippy --lib -- -D warnings` | clean |
| `cargo clippy --all-targets` | **4** warnings (was 5; `items_after_test_module` cleared) |
| `cargo test --workspace -- --test-threads=2` | **1320 passed, 0 failed**, 35 binaries |
| `rtk proxy grep -rn "enum CommandSource" src/ \| wc -l` | 1 |
| `rtk proxy grep -c "enum CommandSource" src/driver/run.rs` | 0 |
| `rtk proxy grep -c "enum IterationSource" src/driver/run.rs` | 1 |
| `rtk proxy grep -c 'Fixed(String::new())' src/driver/run.rs` | 0 |
| `COMMAND_SOURCE_VARIANTS` needles | 6 |
| `spawn_seam_guard` | 26 passed (25 + the boundary self-check) |
| `driver_refusal_record` / `driver_injection_corpus` / `driver_escalation_cap` | 9 / 12 / 8, all passing, **no edits** to those files in this plan's diff |

Every count-bearing check was run through `rtk proxy`.

## Deviations from Plan

**[Rule 1 — the plan's instruction would have created a different defect] The
`(None, None)` arm stamps the journal before returning.**
Found during: Task 2. The plan says replace the arm with
`(None, None) => return Err(DriveError::NoCommandSource)`. A **bare** early
return is wrong here: `run` (the `DriverRun`, with its journal and lock) is
constructed ~75 lines above this match, and there is no other `return Err`
between the two. A bare return would leave a run directory with **no terminal
record**, which is the signal D-12 reserves for a genuine crash. Manufacturing a
false crash signal in order to avoid manufacturing a blank command is the same
mistake pointing the other way. The arm therefore follows the shape the
spawn-failure arm below it already establishes — `finish_run(&mut run.journal,
"no_command_source", budget.used())` with a `tracing::warn!` on journal error,
then `return Err(...)` — which is also what T-17-06 requires ("a run that
started always has a terminal record, even when the thing it was started for
never launched"). No panic. **Commit:** `0dcac4e`. **See Issues** for what I am
not fully confident about here.

**[Rule 1 — added control] A non-vacuity assertion on the new boundary check.**
The plan specified the offender scan but no control. An assertion over an empty
file set is satisfied by finding nothing, which is precisely the failure this
test exists to close, so it also requires ≥10 files to carry a marker.

**[Rule 1 — added assertion] A specific pin that `run.rs` declares no
`CommandSource`.**
The plan asks for a single-declaration assertion. A bare count of 1 would also
pass if `mod.rs`'s declaration vanished and `run.rs`'s returned — the one way to
satisfy the count while reintroducing the collision. Both assertions are present;
the plan named the second one too, and this records that it is load-bearing
rather than belt-and-braces.

**[process] Two commits for the tracer task**, matching this phase's history and
21-13.

**Total deviations:** 4 (1 plan-instruction deviation, 2 added controls, 1
process). **Impact:** no `must_haves` truth weakened; the one plan-instruction
deviation makes the refused path *more* correct than specified.

## Issues Encountered

**Flagged — the new `"no_command_source"` terminal label is unexercised and
unknown to the TUI.** The arm that writes it is unreachable today (`drive`
refuses both-or-neither before `execute_run` is called), so no test drives it and
I could not observe the stamp. Terminal labels are free-form `&str` with no
validating enumeration, and `src/ui/screens/driver.rs` maps known labels to a
glyph and to a `DriverRunState`; both matches have a safe `_` fallback, so the
new label renders as `?` / `Unrecorded` rather than panicking. I checked that
fallback explicitly. **What I am not confident about:** whether a new terminal
label ought to come with TUI table rows the way `spawn_failed` and `killed` did,
and whether stamping at all is preferable to a bare return for a state that
cannot occur. I chose stamping because the alternative fabricates a crash signal,
but this is a judgment the reviewer should re-make rather than inherit. It is
also the only place in either plan where I did something the plan did not
literally specify **in production code**.

**Flagged — guard eight's three new `Self::` needles are type-unqualified.**
`Self::Routed(` would also match an unrelated enum with a `Routed` tuple variant
constructing it inside its own `impl`. That is over-detection (loud), the
acceptable direction, and no such enum exists in the tree today — but it is a new
false-positive surface that did not exist before, and I have named it as limit 5
in guard eight's header rather than leaving it to be discovered.

**`tests/driver_reattach.rs` flakes far more than `deferred-items.md` records,
and I proved it is not a regression.** Six runs of the **unmodified** pre-round-4
tree (`6eb1d49`, separate worktree): `ok, FAILED, FAILED, FAILED, FAILED,
FAILED`. The untouched baseline flakes *worse* than this one. Its documented
mitigation (`--test-threads=1`) no longer works; `--test-threads=2` is reliably
green for the whole workspace. Recorded in `deferred-items.md`, not fixed (the
file is in no `21-*` plan's `<files>`). Note for whoever reads the parent's
stated baseline of "1316 passed, 0 failed": that was a lucky `--test-threads=4`
run, not a stable property.

## Self-Check: PASSED

All `<acceptance_criteria>` re-run; all plan `<verification>` commands re-run.
`.planning/REQUIREMENTS.md` is absent from every commit in this plan
(`4ba4d4a`, `66a6393`, `0dcac4e`, `8f26f0a`) — confirmed per-commit with
`git diff --stat`. `.planning/STATE.md` and `.planning/ROADMAP.md` likewise
untouched.

Phase 21's round-4 plan set is complete and ready for independent code review and
verification.
