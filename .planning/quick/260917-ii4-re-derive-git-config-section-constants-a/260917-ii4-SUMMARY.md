---
quick_id: 260917-ii4
phase: quick-260917-ii4
plan: 01
subsystem: envelope/policy
status: complete
tags: [git-config, guard-doc, re-derivation, release-ci, provenance]

requires: []
provides:
  - "CONFIG_SECTION_CONSTANTS_DERIVED_AGAINST_GIT_VERSION == \"git version 2.55.0\" — the release-CI runner's git, so the version pin passes on the runner"
  - "A re-derivation record on both config-section constants covering RelNotes 2.44.0.adoc..2.55.0.adoc"
affects:
  - "The v1.7.1 release run: the publish job's `cargo test` no longer fails on the version pin"

tech-stack:
  added: []
  patterns:
    - "Documentary re-derivation recorded AS documentary — the measured 2.43.0 probe tables are left byte-identical rather than restamped"

key-files:
  created: []
  modified:
    - src/envelope/policy.rs

decisions:
  - "NEITHER array changes: `INDIRECTION_SECTIONS` stays `&[\"include\", \"includeIf\"]`, `REPARSED_COMMAND_SECTIONS` stays `&[\"alias\"]`. No-change is the re-derivation's genuine result."
  - "git 2.54's `alias.<name>.command` is a new three-level spelling that needs NO array entry, because its SECTION is still `alias` — a vindication of not reading the subsection."
  - "`hook.<friendly-name>.command` is recorded as a NEW K2 family, examined and excluded, with inheritance explicitly NOT claimed."
  - "The falsified subsection justification in `config_key_names_a_reparsed_command_section` was repaired in place rather than left (T-19-107 failure mode)."

metrics:
  duration: "~35 min"
  completed: 2026-09-17

actuals:
  tokens: 3300
  tasks: 3
  commits: 3
plan_head_before: 51658244d37fb04174369814e881dc3023463fec
---

# Quick Task 260917-ii4: Re-derive the git config-section constants against git 2.55.0 — Summary

Moved `CONFIG_SECTION_CONSTANTS_DERIVED_AGAINST_GIT_VERSION` to `"git version 2.55.0"` and wrote
the twelve-release (`RelNotes/2.44.0.adoc`..`2.55.0.adoc`) re-derivation evidence into both
constants' own doc comments — verdict NOTHING NEW for both, with the near-misses named and
excluded, the honesty clause (documentary, not measured) and the schedule-versus-control clause
stated unsoftened — and repaired the one justification git 2.54 made false. One file, one string
literal, everything else `///`.

## What Changed

| Task | What | Commit |
|------|------|--------|
| 1 (tracer) | Version constant → `2.55.0`; CLASS (a) re-derivation record on `INDIRECTION_SECTIONS` | `ef4a2a8` |
| 2 | CLASS (b) re-derivation record on `REPARSED_COMMAND_SECTIONS`, incl. the 2.54 three-level alias finding and the new `hook.*` K2 family; K2 enumeration extended | `86cc7dd` |
| 3 | Repaired the `**The SUBSECTION is not read**` justification that 2.54 falsified | `9b7d42a` |

**Neither array changed.** `INDIRECTION_SECTIONS` and `REPARSED_COMMAND_SECTIONS` are
byte-identical to their pre-task text; the grep gates confirm each still matches its exact
original literal exactly once.

## The Re-derivation's Findings, As Recorded

- **Class (a), indirection**: no third section at 2.55.0. `include` and `includeIf` remain
  complete; `config.c`'s `git_config_include` splices on exactly two keys. Excluded with reasons:
  2.46's `includeif.onbranch` fix and `hasconfig:remote.*.url` straightening, 2.47's
  `includeIf.onbranch` crash fix (all the existing section), 2.52's `:(optional)` pathname prefix
  (no config read, no section), 2.47's default object hash / ref backend (git's own storage).
  Flagged NOT IN RANGE: 2.56's worktree-location condition — a new condition inside the existing
  `includeIf` section, covered by construction.
- **Class (b), re-parsed command**: no new section; `alias` remains the only K1 member. The
  load-bearing finding is written out in full: git **2.54** added `alias.<name>.command`, a
  genuinely new three-level spelling whose value git re-parses as a git command line, and it needs
  **no** array entry because `config_key_section` reads only the text before the first `.` — so
  `-c alias.co.command='<body>'` is already refused. The counterfactual is recorded: a guard that
  had enumerated the two-level `alias.<name>` shape **would have failed open** on it.
- **New K2 family (D-07)**: `hook.<friendly-name>.command`/`.event`/`.enabled` (2.54), extended by
  `.parallel`/`hook.<event>.jobs`/`hook.jobs` (2.55). Recorded as K2, not K1, so no entry — and
  the claim is limited in the same terms the doc already uses for `core.pager`/`core.editor`: **not
  exercised in the child-environment-dump harness, inheritance NOT claimed.**

## Verification

| Gate | Required | Measured |
|------|----------|----------|
| `rtk proxy cargo build` | exit 0 | exit 0 |
| `rtk proxy cargo clippy -- -D warnings` | exit 0 | exit 0 |
| Constant at `2.55.0` / `Measured against 2.43.0` tables / both arrays | 1, 4, 1, 1 | 1, 4, 1, 1 |
| Pin live and firing | `PIN_LIVE_AND_FIRING` | printed — `derived against : "git version 2.55.0"` vs `installed : "git version 2.53.0"` |
| Structural gate (diff vs base) | 1 file, 0 non-doc removals, 0 non-doc additions, 0 probe rows removed | 1, 0, 0, 0 |
| rustdoc warning locations for policy.rs | ≤ 58 | 58 (unchanged; the 2 `invalid_html_tags` warnings in the run are pre-existing and live in `src/ui/screens/driver_start.rs`, identical count before and after) |
| `rtk proxy cargo test --no-fail-fast` | failing set identical to measured baseline | 48 suites / 2119 passed / 2 failed / 15 ignored, before **and** after — see below |

**The version pin fails locally and that is the correct, required outcome.** The local machine runs
`git version 2.53.0`; the pin is now aimed at the release runner's `2.55.0`. It was not reverted,
softened, skipped, gated or ignored — the structural gate (0 non-doc changes outside the string
literal) enforces that mechanically rather than by care.

### The failing set, measured both sides

Both runs: **48 suites / 2119 passed / 2 failed / 15 ignored**. The deterministic failure is the
same single test in both — the version pin. The second slot is one nondeterministic
process/timing test, and it is a *different* one each run:

- pre-edit: `driver::run::tests::the_current_group_agrees_with_the_proc_parse`
- post-edit: `a_run_killed_without_an_ending_is_reported_crashed_and_nothing_on_disk_is_repaired`
  (`tests/driver_reattach.rs:542` — the `driver_reattach` failure the plan itself names as a
  documented pre-existing pair)

Rather than assert flakiness, it was measured: three re-runs of each on the finished tree gave
`proc_parse` ok / ok / ok and the reattach kill test FAILED / ok / ok. So the stable failing set is
exactly one test — the pin — on both sides, with one flaky process test occupying a rotating second
slot. Nothing in the diff is executable outside the one string literal, so no count could have
moved for a real reason.

## Deviations from Plan

**None to the implementation.** Two verification *expectations* in the plan were measured to be
wrong about the pre-existing tree; both are recorded below rather than silently absorbed.

## Coordinator inferred decisions (for audit)

1. **The 1345 baseline was stale, and the source of the number is now known.** The originating
   brief asked for "no regression from 1345 passed". `.planning/STATE.md:428` records the v1.7.0
   measured figures as 48 suites / 2120 passed / 1 failed / 15 ignored. Neither is the whole story:
   **1345 is the LIB suite's pass count**, not the whole-run total (`test result: FAILED. 1345
   passed; 1 failed; 1 ignored` for `src/lib.rs` post-edit). The measured whole-run baseline taken
   on this base revision immediately before the first edit was **48 suites / 2119 passed / 2 failed
   / 15 ignored** — one pass short of STATE.md's 2120/1 because the flaky `proc_parse` test failed
   in that particular run. The verification above therefore pins measured-before against
   measured-after, which is strictly stronger than either quoted number. **Treat 1345 as stale and
   as a per-suite figure; treat STATE.md's 2120/1 as a run where the flaky slot happened to pass.**
2. **The verdict that NEITHER array changes is the re-derivation's genuine result, not skipped
   work.** Twelve release-note files' worth of candidates were examined; every one is named in the
   doc comments with the reason it fails its class test. The most interesting candidate —
   2.54's `alias.<name>.command` — is a real new spelling that genuinely requires no entry, which
   is a substantive finding rather than an absence of one.
3. **Task 3's done-criterion count was wrong about the pre-existing file; the invariant it encodes
   was verified instead.** The plan required `grep -c "The SUBSECTION is not read"` to be `1`. That
   phrase occurs **3** times in the base file (`ef4a2a8~1`), not 1 — three separate docs use the
   same bullet heading. Post-edit it is still **3**, and `grep -c "no condition between them"` went
   `1` → `0` as required. The criterion's *purpose* — "the bullet is repaired in place, not
   deleted" — is therefore satisfied exactly (3 → 3, falsified clause gone). No text was added or
   removed to make a count match; the count was re-measured against the base revision instead.
4. **Where the honesty/control clauses required a judgement call, so a later reader can re-check
   rather than inherit.** Two places:
   - *Which direction the drift pin's two-sided confirmation actually reaches.* The claim written
     is deliberately narrow: the pin re-measures on 2.53.0 locally and on 2.55.0 in release CI, so
     it confirms the REVERSE direction on a git past 2.43.0. It does **not** claim confirmation on
     2.55.0 *from this machine* — nobody here ran it on 2.55.0; that leg is CI's. A reader who
     wants the 2.55.0 leg should look at a release-CI run, not at this record.
   - *How far the `hook.*` K2 claim goes.* The text places the family in K2 on the strength of
     `Documentation/config/hook.adoc`'s "executable path or shell oneliner" wording alone, and then
     explicitly refuses the inheritance claim, because the child-environment-dump harness was never
     pointed at it. The class placement is documentary; the inheritance property is unmeasured and
     stated as unmeasured. A K2 member that does not inherit would be a finding.
5. **Scope held.** `Cargo.toml` untouched, no tag created, no release run — the structural gate's
   `files changed: 1` proves it. The v1.7.1 release is a separate step.

## Deferred (out of scope, not caused by this task)

`.planning/WINDOWS.md` is internally inconsistent: `gsd-tools windows append` refuses with
"Ledger table … disagrees with the fenced JSON entries (the sole source of truth) for row id(s):
19". The one entry this task would have filed (deviation 3 above) therefore was **not** recorded in
the ledger. Pre-existing and unrelated to this diff; repairing another round's ledger row is
outside this task's scope. Fix by editing the fenced JSON block for row 19 and re-running so the
table regenerates.

## Self-Check: PASSED

- `src/envelope/policy.rs` — FOUND (modified, 1 file in diff)
- `.planning/quick/260917-ii4-.../260917-ii4-SUMMARY.md` — FOUND (this file)
- Commits `ef4a2a8`, `86cc7dd`, `9b7d42a` — all FOUND in `git log`; `git rev-list --count
  51658244..HEAD` = 3, matching `commits: 3`
</content>
</invoke>
