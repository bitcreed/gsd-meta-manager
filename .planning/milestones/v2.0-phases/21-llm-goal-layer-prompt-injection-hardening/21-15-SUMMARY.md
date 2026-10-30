---
phase: 21-llm-goal-layer-prompt-injection-hardening
plan: 15
subsystem: driver
tags: [security, type-safety, argv, parse-boundary, gap-closure]
requires: []
provides:
  - "text::carries_visible_content — the single production visibility predicate"
  - "test_support::DEGENERATE — the shared blank-shape payload set"
  - "DriveArgs with all six argv string fields typed payload::NonBlank"
  - "RawDriveArgs + DriveArgs::from_argv — the parse boundary with an exhaustive destructure"
  - "run::iteration_source — the narrowing from CommandSource to IterationSource"
affects:
  - src/driver/mod.rs
  - src/driver/run.rs
  - src/journal/mod.rs
  - src/text.rs
  - src/test_support.rs
  - src/lib.rs
  - src/main.rs
  - src/cli.rs
  - src/app.rs
  - src/ui/screens/driver.rs
  - tests/spawn_seam_guard.rs
  - tests/driver_dry_run.rs
  - tests/driver_goal_seam.rs
  - "9 further integration-test fixture files"
tech-stack:
  added: []
  patterns:
    - "newtype with a private field in a nested module as the single validating constructor"
    - "exhaustive destructure with no `..` as a compile-time completeness mechanism"
    - "one production predicate, deliberately re-spelled in the test-side oracle"
key-files:
  created:
    - src/text.rs
    - src/test_support.rs
  modified:
    - src/driver/mod.rs
    - src/driver/run.rs
    - src/journal/mod.rs
    - src/main.rs
    - src/cli.rs
    - src/app.rs
    - src/ui/screens/driver.rs
    - src/lib.rs
    - tests/spawn_seam_guard.rs
    - tests/driver_dry_run.rs
    - tests/driver_goal_seam.rs
key-decisions:
  - "D-15-1 through D-15-6 executed as written; no decision reversed."
  - "The tracer's fixture goes through DrivableProject::from_registry, NOT for_testing_bypassing_opt_in — the plan's suggested route would break spawn_seam_guard's ESCAPE_HATCH assertion (deviation 1)."
  - "test_support is #[cfg(test)] as specified, so integration tests cannot reach DEGENERATE; the exhaustive six-shape sweep therefore lives in-crate and the two retargeted driver_dry_run tests keep their own pre-existing payload lists (residual 1)."
requirements-completed: []
duration: "~2h"
completed: 2026-08-22
coverage:
  - deliverable: "execute_run resolves its source above every disk write; both String::new() arms deleted"
    verification:
      - kind: test
        ref: "src/driver/run.rs#a_run_with_no_command_source_writes_nothing_before_refusing"
        status: pass
      - kind: command
        ref: "rtk proxy grep -n '=> String::new()' src/driver/run.rs src/driver/mod.rs -> 0 hits (control: 5 String::new() in run.rs)"
        status: pass
    human_judgment: false
  - deliverable: "One production spelling of the invisibility judgment"
    verification:
      - kind: test
        ref: "src/text.rs#only_a_value_with_no_visible_character_is_blank"
        status: pass
      - kind: test
        ref: "src/text.rs#the_zero_width_classes_are_exactly_what_trim_cannot_see"
        status: pass
      - kind: command
        ref: "rtk proxy grep -rn '2060' src/ --include=*.rs -> src/text.rs (production) + src/driver/mod.rs:2083 (the test-side oracle, inside #[cfg(test)] mod tests)"
        status: pass
    human_judgment: false
  - deliverable: "All six DriveArgs argv string fields typed NonBlank; refusal at one parse boundary"
    verification:
      - kind: test
        ref: "src/driver/mod.rs#every_argv_position_refuses_every_degenerate_payload_at_the_parse_boundary"
        status: pass
      - kind: test
        ref: "src/driver/mod.rs#one_visible_character_is_accepted_in_every_argv_position"
        status: pass
      - kind: command
        ref: "sed -n '/pub struct DriveArgs {/,/^}/p' | grep -c ': String|Option<String>' -> 0 (control: 9 payload::NonBlank)"
        status: pass
      - kind: command
        ref: "seven end-to-end refusals against target/debug/gsd-meta-manager, .planning/meta-manager absent, with a visible-payload non-vacuity control"
        status: pass
    human_judgment: false
  - deliverable: "A seventh argv field cannot arrive unclassified"
    verification:
      - kind: command
        ref: "rtk proxy grep -A 20 'pub fn from_argv' src/driver/mod.rs -> full destructure of all 12 RawDriveArgs fields, no `..`"
        status: pass
    human_judgment: true
    rationale: "The destructure forces CLASSIFICATION at compile time but nothing mechanical forces a matrix ROW for a new field; that residual is disclosed at positions() and needs a human to weigh."
---

# Phase 21 Plan 15: DriveArgs Becomes the Domain Summary

Pass-5's three reproduced Criticals closed by closing the **domain** rather
than the instances: all six argv-derived string fields of `DriveArgs` are now
`payload::NonBlank`, judged once at a new parse boundary whose exhaustive
destructure makes a seventh field a compile error until somebody classifies it;
`execute_run` resolves its command source above every disk write and both
`(None, None) => String::new()` arms are deleted as unrepresentable; and
`text::carries_visible_content` becomes the only production spelling of "is
there anything here a reader could see?".

- **Duration:** ~2h · **Tasks:** 3 · **Commits:** 4 · **Files touched:** 18

## Commits

| SHA | Type | What |
|---|---|---|
| `92ad11f` | test | **RED ARM** — the tracer, failing at HEAD |
| `015aea5` | feat | Task 1 GREEN — source resolved above every write; both blank-manufacturing arms deleted |
| `1f0a609` | feat | Task 2 — one visibility predicate; shared `DEGENERATE` |
| `db75f92` | feat | Task 3 — `DriveArgs` retyped, `from_argv`, the 7-position matrix, fixture sweep |

## Red-arm evidence (required, quoted verbatim)

`a_run_with_no_command_source_writes_nothing_before_refusing` was written
first, run against **unmodified** code at `199d334`, and committed failing as
`92ad11f` before any fix was written. Observed output:

```
running 1 test

thread 'driver::run::tests::a_run_with_no_command_source_writes_nothing_before_refusing' (33820) panicked at src/driver/run.rs:3688:9:
a refused run must have created NOTHING — no run directory, no run.json, no journal.jsonl. A refusal that fires AFTER `JournalRun::start` has already committed `"gsd_command": ""` into the record that says what ran, and `""` is the field-absent sentinel (D-30), so the record stops being evidence. Found:
  /tmp/.tmphhj0xK/.planning/meta-manager/runs/
  /tmp/.tmphhj0xK/.planning/meta-manager/runs/run.lock: {
  "run_id": "2026-07-29T12-00-00Z-aaaa",
  "pgid": 33819,
  "started_at": "2026-08-22T17:00:44Z"
}

  /tmp/.tmphhj0xK/.planning/meta-manager/runs/2026-07-29T12-00-00Z-aaaa/
  /tmp/.tmphhj0xK/.planning/meta-manager/runs/2026-07-29T12-00-00Z-aaaa/run.json: {
  "run_id": "2026-07-29T12-00-00Z-aaaa",
  "goal": "",
  "gsd_command": "",
  "target_phase": null,
  "bounds": {
    "max_steps": 20,
    "wall_clock_cap_secs": 14400
  },
  "approved_plan": null,
  "escalation_cap": 3,
  "escalations_used": 0,
  "target": "Host",
  "opt_in": "2026-07-29T11:59:00Z",
  "started_at": "2026-08-22T17:00:44Z",
  "session_id": "c23ce900-63a2-4c6e-8810-6ee7f5720911",
  "pid": 33819,
  "pgid": 33819,
  "claude_code_version": "",
  "argv_digest": "fnv1a64:4ff83dead5f0d238",
  "ended_at": "2026-08-22T17:00:44Z",
  "outcome": "no_command_source"
}
  ...journal.jsonl with three records...

test result: FAILED. 0 passed; 1 failed; 0 ignored; 0 measured; 1023 filtered out
```

The typed error was correct (`Err(NoCommandSource)`) and the disk was corrupt:
a committed `run.json` carrying `"gsd_command": ""` — the D-30 field-absent
sentinel — in the field that says what ran, plus `run.lock`, a run directory
and a journal. Exactly the artifact set pass 5 reproduced by hand.

After `015aea5` the same test passes with the nothing-created assertion intact.

## The three pass-5 reproductions, re-run against the built binary

Fixture: a temp project with `.planning/`, an opted-in registry entry, and
`target/debug/gsd-meta-manager` built from this tree.

```
=== --goal '   ' beside --command ===
Error: a run needs something to do: pass `--command <c>` ... A value made only of
whitespace or invisible characters counts as absent — it names nothing to run, and
recorded it would read as a missing field
exit=1

=== --run-id zero-width U+200B ===
Error: the run id "\u{200b}" is not a single directory name, so it is refused
before anything is created. ...
exit=1

=== --run-id '   ' ===
Error: the run id "   " is not a single directory name ...
exit=1

=== --command '   ' ===
Error: a run needs something to do ...
exit=1

=== --target-phase '   ' ===
Error: a run needs something to do ...
exit=1

=== --goal U+FEFF (sole source) ===
Error: a run needs something to do ...
exit=1

=== alias '   ' ===
Error: no project is registered under the alias `   `
exit=1

=== --approved-plan '   ' ===
Error: the `--approved-plan` value carries no `+`, so it is not an approval token. ...
exit=1

=== nothing created ===
ABSENT

=== NON-VACUITY CONTROL: identical shape, VISIBLE payloads ===
Error: the driver opt-in for `demo` needs re-confirming: this opt-in predates the
prompt-input disclosure ...
```

The control is the important line: the identical invocation with **visible**
payloads gets *past* `from_argv` and reaches a later, different refusal
(`drive`'s opt-in gate). The boundary is refusing blankness, not everything.
`.planning/meta-manager` never existed at any point.

## Prohibition audit

Every prohibition's mechanical check, run against **this plan's own final
tree**. Round 4's prohibitions were written and never checked against the
diff; this table is that check.

| # | Prohibition | Check | Result |
|---|---|---|---|
| 1 | MUST NOT manufacture a blank/sentinel value as ANY match arm's result | `rtk proxy grep -n "=> String::new()" src/driver/run.rs src/driver/mod.rs` | **0 hits** (exit 1) — PASS |
| 1 | …with a positive control so the zero is not vacuous | `rtk proxy grep -c "String::new()" src/driver/run.rs` | **5** — the scanner works; `claude_code_version: String::new()` and four others legitimately construct one outside any match arm |
| 1 | …tree-wide disclosure (out of this plan's scope) | `rtk proxy grep -rn "=> String::new()" src/` | **4 pre-existing hits**, none in this plan's diff: `src/executor/outcome.rs:485`, `src/archive.rs:154`, `src/ui/screens/normal.rs:680`, `src/ui/screens/detail.rs:3524`. Disclosed rather than silently scoped away; none is on an argv or record-writing path. |
| 2 | MUST NOT hand-copy a subset of the degenerate payload set into any pin | `rtk proxy sh -c 'grep -rn "\"\\n  \\n\"" src/ \| grep -v ":[[:space:]]*//"'` | **exactly 1 code hit**: `src/test_support.rs:23`, the shared const — PASS. (The raw, comment-inclusive grep returns 4; the other 3 are prose in `src/test_support.rs`, `src/journal/mod.rs:2732` and `src/driver/mod.rs:1742` *quoting the historical three-of-six hand copy* in the docs that explain why the const exists. Reported rather than filtered away silently.) |
| 3 | MUST NOT introduce a second production spelling of the invisible-character classes | `rtk proxy grep -rn "2060" src/ --include=*.rs` | **2 files**: `src/text.rs` (production, `:45`; `:30/:66/:89` are its doc and its own tests) and `src/driver/mod.rs:2083` — which is inside `#[cfg(test)] mod tests`, the `visibly_empty_numbered_entry` oracle that MUST re-spell the classes independently. PASS |
| 4 | MUST NOT let a preview refuse less than the real run it previews | Discharged **by construction**: `DriveArgs::from_argv` runs before a `DriveArgs` exists, so it cannot read `dry_run`. Asserted anyway over both raw records by the retargeted `a_blank_command_is_refused_in_preview_and_in_a_real_run` and `a_blank_target_phase_is_refused_in_preview_and_in_a_real_run` (`tests/driver_dry_run.rs`). No task reintroduced a mode-dependent parse. | PASS |
| 5 | MUST NOT flip any `.planning/REQUIREMENTS.md` requirement | `rtk proxy git log --oneline 199d334..HEAD -- .planning/REQUIREMENTS.md` | **no commits**; `git log --name-only \| grep -c REQUIREMENTS` → **0**. PASS |

## Deviations from Plan

**[Rule 1 — plan instruction wrong against live code] The tracer fixture cannot
use `for_testing_bypassing_opt_in`.**
Found during: Task 1. The plan's `<action>` says to build the fixture project
"(`DrivableProject::for_testing_bypassing_opt_in` per the verifier's
reproduction)". The verifier's reproduction was a throwaway *integration* test;
inside `src/` that identifier must appear on exactly one executable line — its
own definition at `src/executor/mod.rs:207` — and `tests/spawn_seam_guard.rs`'s
`ESCAPE_HATCH` assertion (`:297-320`) does not exempt `#[cfg(test)]` modules.
`src/driver/mod.rs:1851` records this trap in as many words. Fix: the fixture
goes through the production `DrivableProject::from_registry` with a genuine
opt-in record, the same route `src/driver/mod.rs`'s own `previewable` helper
takes. Verified: `rtk proxy cargo test --test spawn_seam_guard` → 26 passed, 0
failed. Commit `92ad11f`.

**[Rule 1 — plan instruction would introduce a lint] `positions()`'s return type
needed a type alias.**
Found during: Task 3. The 7-position table's natural type
(`Vec<(&str, &str, PositionBuilder, fn(&DriveError) -> bool)>`) trips
`clippy::type_complexity`, adding a fifth `--all-targets` lint where the
measured baseline carries four. Fix: two named aliases, `PositionRefusal` and
`Position`. Verified: `--all-targets` clippy back to the 4 pre-existing lints
in `src/browser.rs`/`src/project_creator.rs`. Commit `db75f92`.

**[Rule 1 — style, deliberately diverged] Fixture payloads are built with an
explicit `.expect`, not the constructor's bare `Option`.**
Found during: Task 3(h). The plan prescribes
`command: payload::NonBlank::new(<s>)` — "the constructor's `Option` return IS
the field type". That is terser but SILENT: a fixture literal that turned out
to be invisible would become `None`, quietly converting a test of "a blank is
refused" into a test of "nothing was supplied" — the exact class of silent
narrowing this round exists to close. Fix: every swept fixture reads
`Some(nonblank(<s>))` / `Some(visible(<s>))`, where the helper `expect`s with a
message. Behaviourally identical for every literal in the tree; loud instead of
silent if one ever stops being. Commit `db75f92`.

**[Rule 1 — a plan row became inexpressible in a file the plan did not list]
`tests/driver_goal_seam.rs`'s blank-goal rows retargeted.**
Found during: Task 3(g). `the_decomposition_capability_exists_only_for_an_invocation_that_supplies_a_goal_alone`
built `goal_args(.., "   \t ")` and `goal_args(.., "")` — both unrepresentable
once `DriveArgs::goal` is `Option<NonBlank>`. Fix: a `raw_goal_args` builder was
added and those two rows became boundary assertions over `DriveArgs::from_argv`
across four blank shapes (including both zero-width ones, which the original
row did not cover); `goal_args` itself now goes *through* the production
boundary rather than being a struct literal. The file was in the plan's
`files_modified`; only the retarget was unplanned. Commit `db75f92`.

**Total deviations:** 4 (4 auto-fixed under Rule 1, 0 escalated).
**Impact:** none reverses a plan decision or narrows a plan property. Two are
corrections of instructions that were wrong against live code, one avoids
introducing a new lint, one trades terseness for loudness in fixtures.

## Residuals and things I am not fully confident about

Flagged deliberately — the verifier will find these anyway.

1. **`test_support` is `#[cfg(test)]`, so integration tests cannot consume
   `DEGENERATE`.** The plan specifies `#[cfg(test)] pub mod test_support` and I
   followed it. The consequence is that the exhaustive six-shape sweep exists
   only *in-crate* (the 7×6 matrix, the journal pin, the `goal_or_none` pin, the
   `goal_lines` pin, the `text.rs` pins). The two retargeted
   `tests/driver_dry_run.rs` tests keep the payload lists they already had
   (`["", "   "]` and `["   ", "\t", "\n  \n", ""]`) rather than gaining a
   hand-copied six — copying would itself be the prohibited pattern. Prohibition
   2's mechanical check is scoped to `src/` and passes; the prohibition's
   *words* say "any pin", and an integration-test pin is arguably one. The clean
   fix is making `test_support` unconditionally `pub` (a single `const`, no
   runtime cost); I did not do it because the plan's acceptance criterion names
   the `#[cfg(test)]` form explicitly. **Adjudication belongs to the verifier.**

2. **The matrix row table is hand-maintained.** Disclosed in the plan's truth 6
   and now in `positions()`'s own doc: `from_argv`'s destructure and 21-16's
   guard nine bound the *type* of a seventh argv field, but nothing forces a
   *row*, so a seventh field correctly typed `NonBlank` with no row leaves the
   matrix at 7×6 silently. That is a coverage residual, not a blank-payload
   route.

3. **The blank-command-beside-a-target-phase refusal changed variant.** It was
   `AmbiguousCommandSource` from `command_source`; it is now `NoCommandSource`
   from `from_argv`, because the boundary judges each position before ambiguity
   is considered. The invocation is still refused, refused *earlier*, and still
   creates nothing — T-21-11-05's actual property (blankness must not DEMOTE an
   ambiguous invocation into a legal one) holds. The old assertions were
   replaced by `a_blank_payload_beside_a_visible_one_is_still_refused_at_the_boundary`
   with the reasoning written at the site. A reviewer may reasonably want the
   ambiguity check hoisted above the per-position checks instead; I judged
   "refused with the flag you mistyped named" to be the better message.

4. **A window of one commit is red.** `92ad11f` is the tracer's red arm and
   fails by design; `015aea5` greens it. That is the plan's explicit
   requirement, but anyone bisecting through `92ad11f` will see a failing
   `cargo test --lib`. That commit's test also writes an envelope into the real
   user data directory unless `GSD_MM_ENVELOPE_ROOT` is set (I set it to a temp
   dir for the observation above); from `015aea5` onward the test refuses before
   envelope establishment and touches nothing.

5. **`main.rs` calls `load_config` before `DriveArgs::from_argv`.** So an
   unparseable `--config` masks a blank-payload refusal. That ordering is
   pre-existing and out of this plan's scope; noted because it surfaced while
   building the end-to-end reproduction.

## Test delta (auditable by name — no numeric floor, per the plan)

**Deleted (2):**
- `driver::tests::a_blank_run_id_is_refused_without_touching_disk` — superseded
  by the matrix's `--run-id` row, which sweeps all six shapes rather than three.
  Its legitimate-id control arm survives as
  `driver::tests::a_legitimate_run_id_still_passes_the_seams_predicate` and in
  `journal`'s own both-directions pin.
- `driver::tests::every_command_source_refuses_or_previews_cleanly_for_every_degenerate_payload`
  — replaced by the 7-position matrix over `from_argv` (the old 3-tuple
  `PositionBuilder` axis was tied to `command_source`'s arity and structurally
  could not hold a `--run-id`, `--approved-plan`, alias or multi-flag column).

**Added (8):**
- `driver::run::tests::a_run_with_no_command_source_writes_nothing_before_refusing` (the tracer)
- `text::tests::only_a_value_with_no_visible_character_is_blank`
- `text::tests::the_zero_width_classes_are_exactly_what_trim_cannot_see`
- `driver::tests::every_argv_position_refuses_every_degenerate_payload_at_the_parse_boundary`
- `driver::tests::one_visible_character_is_accepted_in_every_argv_position`
- `driver::tests::a_blank_payload_beside_a_visible_one_is_still_refused_at_the_boundary`
- `driver::tests::a_blank_approval_token_is_refused_by_the_real_parser`
- `driver::tests::a_legitimate_run_id_still_passes_the_seams_predicate`

**Modified in place (not counted above):** the two `tests/driver_dry_run.rs`
blank-payload tests (retargeted to the boundary), `app`'s
`an_empty_goal_buffer_reaches_start_driver_run_as_none` and
`ui::screens::driver`'s `an_absent_goal_renders_the_pinned_copy_and_nothing_else`
(both now sweep the shared `DEGENERATE`), `journal`'s
`only_a_single_plain_component_is_accepted_as_a_run_id` (blank half consumes the
shared const; structural hostiles stay literal),
`driver::tests::a_run_with_no_command_source_at_all_is_refused_before_anything_is_created`
and `…_a_run_naming_both_command_sources_is_refused_rather_than_resolved` (blank
rows moved to the boundary).

Net **+6**: 1320 total before (1318 passed + 2 known flakes) → **1326 passed,
0 failed**.

## Verification

| Gate | Result |
|---|---|
| `rtk proxy cargo build --all-targets` | clean, 0 warnings |
| `rtk proxy cargo clippy --lib -- -D warnings` | clean |
| `rtk proxy cargo clippy --all-targets` | 4 lints, all pre-existing (`src/browser.rs` ×3, `src/project_creator.rs` ×1) — unchanged from the measured baseline |
| `rtk proxy cargo test --workspace --no-fail-fast -- --test-threads=2` | **1326 passed, 0 failed** (the two documented `driver_reattach` flakes did not fire this run) |
| `rtk proxy cargo test --test spawn_seam_guard` | 26 passed, 0 failed — guard eight green with the new `("src/driver/run.rs", "iteration_source")` allowlist entry |

Every count-bearing and presence-bearing check in this SUMMARY was run through
`rtk proxy`, per the phase's three false-`0` incidents.

## Issues Encountered

None blocking. The five residuals above are disclosed rather than resolved.

## Next Phase Readiness

Ready for `21-16` (wave 2, `depends_on: [21-15]`). The properties 21-16's guard
nine reads are in place: `DriveArgs`'s declaration text now contains six
`payload::NonBlank` fields and no raw `String`/`Option<String>` argv field, and
`RawDriveArgs` is a separate struct declaring the raw side.

## Self-Check: PASSED
