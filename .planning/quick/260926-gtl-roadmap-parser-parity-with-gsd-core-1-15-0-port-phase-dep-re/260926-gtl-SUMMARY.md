---
phase: quick-260926-gtl
plan: 01
status: complete
subsystem: state_reader / agents
tags: [roadmap-parser, gsd-core-1.15.0, parity, depends-on, goals, phase-id]
requires: [260926-gtk]
provides:
  - PHASE_DEP_REF (#4764) grammar in parse_depends_on (own-number self-skip, normalised dedupe)
  - extractPhaseFieldMultiline (#4731/#4837) port in parse_phase_goals
  - letter-suffixed phase ids (#2128/#4830) in PHASE_ID, plan-checklist regex and worktrees plan_id_re
affects: [driver router dependency gating (unchanged consumer), Roadmap tab goal/Needs display]
tech-stack:
  added: []
  patterns: [OnceLock-static regexes, oracle-measured parity tables pinned in tests, named divergence tests]
key-files:
  created: []
  modified:
    - src/state_reader/roadmap_md.rs
    - src/state_reader/mod.rs
    - src/agents/worktrees.rs
    - src/ui/screens/detail.rs
decisions:
  - "INFERRED: worktrees plan_id_re accepts an UPPERCASE letter only"
  - "INFERRED: plan-checklist regex widened to count 12A-01-PLAN.md items"
  - "INFERRED: divergence A kept — parenthetical qualifiers stay stripped (T-20-16)"
  - "Divergence B (pre-existing) kept — project-code-prefixed refs (Phase M-2) read"
  - "INFERRED: divergence C — any heading level ends a goal"
  - "INFERRED: divergence D — goal continuation starts on the line after the label line"
  - "INFERRED: sentriq phase-12 detail test rewritten to the Needs GSD 1.15.0 reads"
metrics:
  duration: ~10m
  completed: 2026-09-26
plan_head_before: ba6b087faf465176515f1db41dbb2890dccc1e2a
actuals:
  tokens: 9600
  tasks: 3
  commits: 6
---

# Quick 260926-gtl: Roadmap parser parity with gsd-core 1.15.0 Summary

The ROADMAP.md reader now follows gsd-core 1.15.0 in three places. Dependency lines use the
`PHASE_DEP_REF` grammar: lists, `and`/`&`, range endpoints, case-insensitive, and the phase's own
number is dropped. Hard-wrapped `**Goal**` fields are read up to upstream's boundaries. Phase ids
with a letter suffix (`12A`, `23A.1.2`) parse end to end, including Codex branch and ledger plan ids.
All expected values are the ones measured from the 1.15.0 oracle, and each of the four
divergences has its own named test.

## Upstream issues ported (upstream/release-1.15.0, ec81d0d)

| Issue | Upstream source | Rust site |
|---|---|---|
| #2128 / #4830 letter-suffixed ids | `src/phase-id.cts:65` `PHASE_NUMBER_TOKEN_SOURCE = '\d+[A-Z]?(?:\.\d+)*'` | `roadmap_md.rs` `PHASE_ID` → `(?:[A-Za-z]{1,4}-)?[0-9][0-9.]*[A-Za-z]?(?:\.[0-9]+)*`; `plan_checklist_re`; `worktrees.rs` `plan_id_re` → `^[0-9]+[A-Z]?(\.[0-9]+)*(-[0-9]+)?$` |
| #4764 PHASE_DEP_REF | `src/phase-id.cts:80-81`, consumer `src/init.cts:3086-3095, 3148-3176` (`normalizePhaseNumber`) | `parse_depends_on(text: &str, own_number: &str)` + private `dep_ref_key` |
| #4731 / #4837 extractPhaseFieldMultiline | `src/roadmap-parser.cts:2294-2340` | `parse_phase_goals` + private `ends_field_continuation` |

## Tasks

| Task | RED commit | GREEN commit | Files |
|---|---|---|---|
| 1 Letter-suffixed ids (tracer) | 553e17d | debaf77 | roadmap_md.rs, worktrees.rs, state_reader/mod.rs |
| 2 PHASE_DEP_REF port | 60fba4b | 7feff7f | roadmap_md.rs, ui/screens/detail.rs |
| 3 Multi-line goals | 25eaabc | dd3c835 | roadmap_md.rs, state_reader/mod.rs |

Tracer gate (Task 1): I re-ran `<verify>` after GREEN. `letter_suffixed` ran 3 tests and 3 passed.
`state_reader::` 353/353 and `agents::` 62/62 passed. waves.rs and phase_num.rs are byte-identical
to 228a063.

## Inferred decisions (for audit)

1. **INFERRED: uppercase-only plan-id letter.** `plan_id_re` accepts `[A-Z]` only. GSD writes the
   canonical uppercase suffix (`init.cts` `normalizePhaseNumber` uppercases it), the existing
   pinned rejection of `agent-p13x-…` still holds, and the narrower rule is the conservative one
   for an id that feeds attribution. `12a-01`, `12AB-01`, `A12-01`, `12A..1-01` and `../x` are
   still rejected, and each case is pinned.
2. **INFERRED: plan-checklist regex widening.** The operator spec named PHASE_ID and plan_id_re.
   Without this change, a letter phase the reader now recognises would always report zero plans.
   The regex is now shared by GSD phases and build phases through `plan_checklist_re()`, and
   the per-call compile in `parse_planned_build_phases` is gone.
3. **INFERRED: Divergence A. Parenthetical qualifiers stay stripped** (Plan 20-03 T-20-16).
   Measured 1.15.0 value for this repo's phase 20 line: 16, 17, 19, 22. Rust gives 16, 17, 19.
   Taking the 1.15.0 value would create a 20↔22 cycle, because phase 22 reads 15, 17, 21, 20.
   Phases 24/25 would also gain 15, 23, 14 (and 24). The test that pins this is
   `depends_on_parenthetical_qualifier_stays_stripped_unlike_gsd_1_15`.
4. **Divergence B (pre-existing, kept). Project-code-prefixed refs are read.** For
   `Subphase 4, Build phases 8-13, Phase M-2, phase 5 through 6`, 1.15.0 gives 8, 13, 5, 6 and
   Rust gives 8, 13, M-2, 5, 6. Pinned by `depends_on_keeps_project_code_prefixed_refs_unlike_gsd_1_15`.
5. **INFERRED: Divergence C. A `#####`/`######` heading ends a goal.** For
   `**Goal**: ten\n##### deep heading\nmore`, 1.15.0 gives `ten ##### deep heading more` and
   Rust gives `ten`. Pinned by `goal_divergences_from_gsd_1_15_are_pinned`.
6. **INFERRED: Divergence D. Continuation starts on the line after the label line.** Two cases:
   - `**Goal**:\n**Depends on**: Phase 1`: 1.15.0 gives `**Depends on**: Phase 1`, Rust gives no goal.
   - `**Goal**:   \n\nafter blank`: 1.15.0 gives `after blank`, Rust gives no goal.

   `**Goal:**\nnext line goal\nwrapped` still matches upstream (`next line goal wrapped`).
   Pinned by the same test.
7. **INFERRED: Phase-identity model is add-alongside.** Letter-suffixed ids use `phase_key`'s
   existing raw-text fallback. `PhaseNum` is not promoted. The only letter-aware normalisation
   is the private `dep_ref_key`, local to `parse_depends_on`.
8. **INFERRED (deviation, see below): sentriq phase-12 detail test rewritten.**

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug/test contradiction] A detail test contradicted the plan's pinned oracle row**
- **Found during:** Task 2 (full `--lib` run)
- **Issue:** `ui::screens::detail::tests::sentriq_phase_12_explains_it_has_no_deps` asserted that
  sentriq's phase 12 shows `nothing declared` / `can run any time`. The plan's oracle table
  (sentriq row 12 → 9, 11, "negation prose kept by design") requires that line to read as 9, 11,
  which is also what gsd-core 1.15.0's router gates on. The plan said no existing test body is
  edited except the renamed plural-range test, but it did not know about this test. The two
  requirements cannot both hold.
- **Fix:** Renamed the test to `sentriq_phase_12_needs_what_gsd_1_15_reads_from_its_prose`. It now
  asserts that the Needs row names 11 (9 shows as `implied via 11`), does not say
  `nothing declared`, and does not leak the raw `Nothing in this milestone` prose. The
  `nothing declared` / `can run any time` rendering is still covered by
  `ui::roadmap_view` unit tests (roadmap_view.rs:1495, :1710).
- **Files modified:** src/ui/screens/detail.rs
- **Commit:** 7feff7f

User-visible effect: a Roadmap row whose dependency prose says "blocks on nothing in Phases 9-11"
now shows Needs 11 (and 9 as implied), the same list GSD 1.15.0's own router gates on.

## Follow-ups

- **PhaseNum/PlanRef letter ordering.** A letter-suffixed agent row keeps its
  `branch_plan`/`ledger_plan`, but stays unattributed to a plan until `PhaseNum` gets an
  optional per-segment letter ordered like upstream's `comparePhaseNum`. `PlanRef::from_id` goes
  through `PhaseNum::parse`, which is numeric-only by design. Accepted debt from the same gap:
  `012A` and `12A` do not merge into one roadmap row. A `Phase 012A` dependency on a `12A` row
  reads as undeclared, so the router fails closed. This becomes necessary when a registered
  project uses letter-suffixed phases with padded headings, or runs letter-suffixed executor
  worktrees.
- **Pre-existing depends-scan boundary (not changed).** An entry's dependency scan runs to the
  next phase ENTRY line, not the next heading. So a GSD phase entry with no `**Depends on**:`
  line of its own, followed directly by `#### Build phase` entries, would pick up a build
  entry's line. With the case-insensitive keyword, a line like `Build phases 8-13` now yields
  its endpoints where it used to yield nothing. No fixture or real roadmap has that layout:
  ttbook's last GSD entry, Phase 13, declares its own line.
- `GSD_CORE_SYNCED_VERSION` stays 1.14.0 (owned by 260926-gtk). Once 1.15.0 is on npm, the
  installed oracle can be bumped and gate 2 becomes the default gate.

## Doc-change finding

No doc change was needed, because no README or docs/*.md file states either grammar. README:64-65
only says the Roadmap detail pane shows each phase's goal, needs and unblocks, and that is still
true. docs/GSD-CORE-SYNC.md was not touched (owned by 260926-gtk).

## Verification (raw gate results)

1. **Router conformance, installed oracle** (`~/.claude/gsd-core`, 1.14.0):
   `the_rust_rule_table_agrees_with_gsd_s_own_router_over_a_fixture_per_state ... ok`, with
   `test result: ok. 3 passed; 0 failed; 0 ignored`. Zero SKIP lines under `--nocapture`.
2. **Router conformance, 1.15.0 oracle** (fake home still present; `runtime-identity` reports
   `{"packageName":"@opengsd/gsd-core","version":"1.15.0"}`): `conformance: 5 command comparisons,
   2 declared divergences, 2 upstream-silent states, over 9 fixtures`, with
   `test result: ok. 3 passed; 0 failed; 0 ignored`.
3. **Full suite** `rtk proxy cargo test --no-fail-fast` (56 result lines, all binaries ran):
   2694 passed, 1 failed, 15 ignored. Lib: `1837 passed; 1 failed; 1 ignored`. The only failure
   is `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`,
   the git-version witness, which is expected on this machine. No `Text file busy` flake appeared
   (envelope_carrier_reach: 39 passed).
4. **Clippy** `rtk proxy cargo clippy --all-targets -- -D warnings`: `Finished`, 0 warnings.
- `git diff --quiet 228a063 -- src/agents/waves.rs src/state_reader/phase_num.rs src/driver/router.rs`
  exits 0.
- Acceptance greps: `fn parse_depends_on(text: &str, own_number: &str)` = 1;
  `reads_a_plural_range_as_its_endpoints` = 1; `Single-line goals only` = 0.

## Threat surface

Nothing beyond the plan's threat model. T-gtl-01, 03, 04 and 05 are mitigated as registered, and
each is pinned by the tests named above. No crates were added.

## Self-Check: PASSED

- FOUND: src/state_reader/roadmap_md.rs, src/state_reader/mod.rs, src/agents/worktrees.rs, src/ui/screens/detail.rs
- FOUND commits: 553e17d, debaf77, 60fba4b, 7feff7f, 25eaabc, dd3c835 (6 = `git rev-list --count ba6b087..HEAD`)
