---
phase: 29-master-prompt-run-templates
plan: 01
subsystem: launcher
tags: [driver-prompt, master-prompt, include_str, golden-tests, stop-rules, driverctl]

requires:
  - phase: 27-driverctl
    provides: "driverctl CLI + protocol enums (MAX_REPORT_LINES, RULE_ENUM, END_REASONS, RECOVER_ACTIONS, REPORT_STATUSES, DONE_WHEN, MODES) the prompt is checked against"
  - phase: 28-sandboxed-driver-launcher
    provides: "launcher::sandbox::render_prompt + MAX_PROMPT_BYTES, the render seam the embedded prompt goes through"
provides:
  - "src/launcher/assets/MASTER-PROMPT.md: driver-prompt/1.0.0, the embedded versioned master prompt"
  - "launcher::prompt: DRIVER_PROMPT_VERSION, MASTER_PROMPT, STOP_RULES_BEGIN/END, stop_rules_block, header_version, ResolvedPrompt, embedded, custom, custom_version"
  - "tests/driver_prompt.rs goldens + D-24 variant derivation; tests/fixtures/prompt/MASTER-PROMPT.no-gap-rules.md"
  - "the embedded prompt's driverctl block runs through the real CLI (tests/driverctl_client.rs)"
affects: [29-04 DIRECTION template (copies the stop-rules block), 29-06 launcher wiring (consumes ResolvedPrompt), 29-07/29-08 acceptance (H4 variant fixture)]

actuals:
  tokens: 16000
  tasks: 3
  commits: 6

plan_head_before: c0cb151355ceffcbf74b39318d9f9f7ef9cbf859
plan_head_after: 30930f97a441c0509f8eb5f29c1f5e0103959543
commits: 6

tech-stack:
  added: []
  patterns:
    - "prompt-as-asset: include_str! constant + version const + line-1 header, with a test that the two agree"
    - "golden/lint tests over the embedded constant, with enum membership taken from the protocol constants (never hand-copied)"
    - "derived-fixture drift guard: exactly-once span transform + REGENERATE_PROMPT_VARIANT=1 (mirrors spike-003 make_variant.py)"

key-files:
  created:
    - src/launcher/assets/MASTER-PROMPT.md
    - src/launcher/prompt.rs
    - tests/driver_prompt.rs
    - tests/fixtures/prompt/MASTER-PROMPT.no-gap-rules.md
  modified:
    - src/launcher/mod.rs
    - tests/driverctl_client.rs
    - .planning/phases/29-master-prompt-run-templates/29-CONTEXT.md

key-decisions:
  - "Operator-confirmed D-18: control is honoured before the next dispatch (PRMT-04 reading); CONTEXT D-18 now carries [operator-confirmed 2026-10-19]."
  - "The five SC1 rules are hard rules 1-5; the carried v0.4 rules are renumbered 6-10; the stop rules moved out of the hard-rule list into their own precedence section."
  - "Stop rule 1 (human control, incl. the LAUNCHER CAP: control= deny form) sits above launcher-cap so a control deny ends the run and is never parked as launcher-cap."

patterns-established:
  - "Marker-delimited blocks (stop-rules, gap-semantics) so downstream assets copy or strip a block byte-for-byte"

requirements-completed: [PRMT-01, PRMT-03, PRMT-05]

duration: 13min
completed: 2026-10-19
status: complete
---

# Phase 29 Plan 01: Master Prompt Summary

**The driver's contract text is now embedded in the binary as `driver-prompt/1.0.0`: five numbered SC1 hard rules, an eight-rule stop-rule precedence block with the OQ-R1 lift rule, mechanical-reconcile wording, the control deny form and an advisory-only router hint, each guarded by a golden test, plus a drift-checked derived test variant.**

## Performance

- **Duration:** 13 min (21:48 to 22:01 CDT, from the plan commit ledger mtime to SUMMARY write)
- **Tasks:** 3 (1 tracer, 2 TDD)
- **Files:** 4 created, 3 modified
- **Final asset size:** 16275 bytes (`src/launcher/assets/MASTER-PROMPT.md`)

## Accomplishments

- Tracer: asset + `launcher::prompt` + `pub mod prompt;` (un-gated, like `sandbox`); version-agreement, non-empty and 200-char-RUN_DIR render goldens; the `tests/driverctl_client.rs` lifter now takes its subject text and `the_embedded_master_prompt_runs_through_the_real_cli` runs every `driverctl` line of the embedded asset through the real binary (D-07).
- `driver-prompt/1.0.0` body: SC1 rules as hard rules 1-5, stop rules 1-8 between `<!-- stop-rules:begin/end -->` (no braces, single-line items, D-12/D-13), gap semantics between `<!-- gap-semantics:begin/end -->`, Startup reconcile ("exactly one `recovered`", including a paused run's running item, `reset` then `status --state pending`), Loop with the single advisory `next {PROJECT_ROOT} --json` citation and the `human-verification: defer|skip` rule, worker brief with the 11 fields in D-06 order.
- D-24: `strip_gap_rules` (exactly-once spans, errors name the span), the committed fixture, and a drift test with `REGENERATE_PROMPT_VARIANT=1`.
- D-18 pause-semantics confirmation recorded in CONTEXT (separate docs commit, as directed by the operator).

## Anchors enforced by the tests (`tests/driver_prompt.rs`)

- Header line 1 version == `DRIVER_PROMPT_VERSION` (`driver-prompt/1.0.0`); non-empty; render under `MAX_PROMPT_BYTES` with no `{RUN_DIR}`/`{PROJECT_ROOT}`/`{MODE}` left.
- Hard rules 1-5 contain, respectively: `One fresh subagent per GSD step`; `11-field report` + `at most 15 lines` (from `MAX_REPORT_LINES`) + `verbatim`; `isolation: "worktree"`; `Never re-spawn` + `SendMessage`; ``Every record goes through `driverctl` ``.
- Worker brief: the 11 field tokens at line start in order, `at most 15 lines`.
- Stop-rule block: eight `N. ` lines in order; rule 1 has `control: stop`, `control: pause`, `not a park`; rules 2-8 name `--rule launcher-cap|run-budget|divergence|gap-rounds|budget|no-progress|needs-human`; `Divergence outranks gap rounds`; ``lifts only a `needs-human` park``; `budget edit`; no `{` or `}`. Divergence is listed before gap rounds and says it outranks.
- Every `--rule/--reason/--action/--status/--state/--done-when/--mode` alternative is a member of the protocol enum (`RULE_ENUM`, `END_REASONS`, `RECOVER_ACTIONS`, `REPORT_STATUSES`, `ItemState::ALL`, `DONE_WHEN`, `MODES`); the `status` line offers every state word including `pending`.
- Every line naming AskUserQuestion, tmux, stdin or "wait for input" carries a negation; no `call AskUserQuestion`, `tmux attach`, `read from stdin`.
- `next {PROJECT_ROOT} --json` exactly once, in `## Loop`, on a line containing `advisory`; the text keeps `never binds`, `never replaces checking done-when`, `is information, not a stop rule`.
- `LAUNCHER CAP: control=`, ``exactly one `recovered` ``, `caps.gap_rounds`, `human-verification`, and `Before a gap-closure step, check stop rules 4 and 5 first.` are present.
- D-24: variant equals `strip_gap_rules(MASTER_PROMPT)` byte-for-byte; transform errors name a doubled rule 5, a missing gap-semantics end marker and a missing pre-check sentence; the variant has no `4. `/`5. ` rules, contains `no round limit`, keeps rules 1-3, 6-8 and the OQ-R1 line.

## Task Commits

1. **Task 1 (tracer): embedded prompt, version, render, real-CLI lift** - `4722cd3` (feat)
2. **Task 2 RED: failing goldens** - `d8b40c9` (test); **GREEN: 1.0.0 body** - `853d72c` (feat)
3. **Task 3 RED: failing variant-derivation test** - `9fd919c` (test); **GREEN: derived fixture** - `e125bcf` (feat)
4. **Operator D-18 confirmation** - `30930f9` (docs)

## TDD notes

- Task 2 RED: 6 of 11 `driver_prompt` tests failed on the planned assertions against the v0.4 text (missing stop-rules block, missing SC1 anchors, `call AskUserQuestion` shape in `Never call AskUserQuestion`, no `next ... --json` citation, missing `LAUNCHER CAP: control=`). Semantic assessment: each target test executed and failed for the intended reason. `gsd_run check tdd-red-evidence` has no adapter for `cargo test` console output, so RED is assessed semantically only.
- Task 3 RED: `the_no_gap_rules_variant_is_derived_from_the_embedded_prompt` failed because the fixture did not exist (the transform lives in the test file, so the other variant tests passed immediately).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] Variant test picked the wrong "5. " line**
- **Found during:** Task 3 RED
- **Issue:** `the_variant_transform_names_a_span_that_is_not_unique` duplicated the first `5. ` line of the whole prompt, which is hard rule 5, not stop rule 5, so the transform legitimately succeeded.
- **Fix:** select the `5. ` line from `stop_rules_block(MASTER_PROMPT)`.
- **Files modified:** tests/driver_prompt.rs
- **Commit:** `9fd919c` (fixed before the RED commit)

**2. [Instruction-driven extra commit] D-18 operator confirmation**
- Added `30930f9` editing `29-CONTEXT.md` D-18 per the operator decision relayed in the dispatch; no code impact.

**Total deviations:** 1 auto-fixed (test bug), 1 operator-directed docs commit. **Impact:** none on scope.

## Wording decisions beyond CONTEXT `[inferred — audit]`

- Carried v0.4 rules renumbered 6-10; hard rule 1 names rule 4's `SendMessage` continuation as the one exception to "never reuse a worker".
- Hard rule 6 says "Never use AskUserQuestion" (not "Never call ...") because the plan's instruction-shape lint forbids the substring `call AskUserQuestion`.
- Stop rule 3 parks the in-flight item with `--rule run-budget` and ends with `--reason run-budget`; rule 5 states the default of 2 closure rounds when the budget has no `gap-rounds` key (RESEARCH Open Question 1 wording); an empty `answer:` lifts nothing is stated in the OQ-R1 line rather than in rule 5.
- The D-24 variant keeps the version token on line 1 (suffix appended), so `header_version` still parses it; it is recorded as `custom:<sha8>` at launch (plan 29-06).
- Duration is approximate: start time came from the commit-ledger file mtime because the start timestamp was not captured in a persistent shell.

## Issues Encountered

- Full suite (`cargo test --no-fail-fast`): 3552 passed, 1 failed, 22 ignored across 81 targets. The single failure is the expected local `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against` git-version witness (green on the CI runner). `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` pass.

## Known Stubs

None.

## Threat Flags

None - no new network, auth or file-access surface (pure text, hashing and tests).

## Next Phase Readiness

Plan 29-04 can copy the stop-rules block via `launcher::prompt::{STOP_RULES_BEGIN, STOP_RULES_END, stop_rules_block}`; plan 29-06 can consume `ResolvedPrompt`/`embedded()`/`custom()`; plan 29-07 H4 uses `tests/fixtures/prompt/MASTER-PROMPT.no-gap-rules.md`.

## Self-Check: PASSED

All four created files exist and all six task commits are ancestors of HEAD.
