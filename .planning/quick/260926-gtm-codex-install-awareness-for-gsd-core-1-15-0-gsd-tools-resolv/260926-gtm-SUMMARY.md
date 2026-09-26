---
phase: quick-260926-gtm
plan: 01
status: complete
subsystem: state_reader, executor, conformance harness
tags: [gsd-core-1.15.0, codex, gsd-tools-resolver, env-scrub, conformance]
requires: [260926-gtl]
provides:
  - "gsd-tools resolver that finds Codex-only installs (queue_md.rs)"
  - "conformance oracle that resolves Codex installs, pinned to canonical /gsd- spelling"
  - "codex exec child with an inherited GSD_RUNTIME scrubbed"
affects: [next-command suggestions, driver codex transport, router conformance test]
tech-stack:
  added: []
  patterns: ["pure candidate list + injected home/CODEX_HOME, one thin env-reading wrapper"]
key-files:
  created: []
  modified:
    - src/state_reader/queue_md.rs
    - tests/driver_router_conformance.rs
    - src/executor/codex.rs
    - tests/driver_codex_runtime.rs
    - tests/fixtures/fake-codex.sh
    - README.md
decisions:
  - "INFERRED: scrub (never set) an inherited GSD_RUNTIME from the codex exec child"
  - "INFERRED: honour a non-empty CODEX_HOME for the Codex home candidate in both resolvers"
  - "INFERRED: add the project-local <root>/.codex/gsd-core candidate to the production resolver only"
  - "Pin GSD_RUNTIME=claude on the conformance oracle child (spelling-only for init.manager)"
metrics:
  started: 2026-09-26T17:55:00Z
  completed: 2026-09-26T18:17:05Z
  tasks: 3
  files: 6
plan_head_before: f98696a15cbac89edae04ff5e0080ac5d1c8b452
actuals:
  tokens: 6019
  tasks: 3
  commits: 6
---

# Quick 260926-gtm: Codex install awareness for gsd-core 1.15.0

The TUI's gsd-tools resolver and the conformance oracle now both find a Codex-only gsd-core install at `${CODEX_HOME:-~/.codex}/gsd-core`, and the Claude install still wins when both exist. The oracle is pinned to the canonical `/gsd-` spelling. A driven `codex exec` child no longer inherits `GSD_RUNTIME`.

## What changed

- **src/state_reader/queue_md.rs**
  - New pure `gsd_tools_candidates(root, home, codex_home)`. Its order mirrors upstream 1.15.0 `gsd-run-resolver.md` (#4834): `<root>/gsd-core`, `<root>/.claude/gsd-core`, `<root>/.codex/gsd-core`, `~/.claude/gsd-core`, `${CODEX_HOME:-~/.codex}/gsd-core`, then `gsd-tools` on PATH.
  - New `resolve_gsd_tools_from` takes home and CODEX_HOME by injection. `resolve_gsd_tools` is now a thin wrapper and the only place that reads them from the process environment.
  - The doc comment lists the deliberate differences from upstream. It also says why no GSD_RUNTIME pin is applied to `smart-entry`: that command hardcodes `/gsd:`.
- **tests/driver_router_conformance.rs**
  - New `oracle_candidates`: `~/.claude` first, then `${CODEX_HOME:-~/.codex}`.
  - New `Oracle::query_command` sets `GSD_RUNTIME=claude`, with a comment giving the measured reason.
  - The SKIP message and the missing-oracle message both name the two install locations.
- **src/executor/codex.rs**: `scrubbed_from_codex_child` drops exactly `GSD_RUNTIME`. Its doc comment carries the scrub-not-set rationale. The scrub-then-envelope ordering in `start_run` is untouched.
- **tests/driver_codex_runtime.rs** and **tests/fixtures/fake-codex.sh**
  - `GSD_RUNTIME=claude` is now planted in the test process.
  - The fake codex also logs `GSD_*` names (names only, never values).
  - The env test asserts that `GSD_RUNTIME` is absent in the child. As a positive control it asserts that `ENVELOPE_ROOT_ENV` is present.
- **README.md**: the Codex bullet now mentions Codex-only install discovery and the `GSD_RUNTIME` scrub.

## Commits

| Task | Commit | Message |
|------|--------|---------|
| 1 RED | a7948ca | test: failing gsd-tools resolver order tests for Codex installs |
| 1 GREEN | 459ed05 | feat: resolve Codex-only gsd-core installs in the TUI resolver |
| 2 RED | 3bc0137 | test: failing conformance-oracle tests for Codex installs |
| 2 GREEN | d5bd8d7 | feat: conformance oracle finds Codex installs, asks for /gsd- spelling |
| 3 RED | fb29df0 | test: failing GSD_RUNTIME scrub tests for the codex child |
| 3 GREEN | ea92175 | feat: scrub inherited GSD_RUNTIME from the codex exec child (+ README) |

## Task 2 RED/GREEN observations (end to end, no ALLOW_MISSING_ORACLE)

1. **Before the change, with a Codex-only temp HOME** (`.codex/gsd-core` symlinked to the installed 1.14.0 Codex install, marker `codex`): FAILED at "the conformance oracle could not be resolved".
2. **Before the change, with the real HOME and `GSD_RUNTIME=codex`:** FAILED at the safe-alphabet assert. `upstream now emits $gsd-discuss-phase 01`.
3. **Candidate list added, no pin yet, Codex-only temp HOME:** the oracle resolved, then FAILED at the safe-alphabet assert with `$gsd-discuss-phase 01`.
4. **After the pin:** GREEN in all three runs (real HOME, Codex-only temp HOME, `GSD_RUNTIME=codex`). Each run reported `conformance: 5 command comparisons, 2 declared divergences, 2 upstream-silent states, over 9 fixtures`.

## Router conformance against both oracles

- **Installed oracle** (`~/.claude/gsd-core`, 1.14.0): ok, 6/6, 5 comparisons.
- **1.15.0 oracle, as a Claude install** (scratchpad `fakehome`): ok, 6/6, 5 comparisons.
- **1.15.0 oracle, as a Codex-only install:** ok, 6/6, 5 comparisons.
  - Setup: a mirror of the gsd115 checkout, with the real `gsd-core` dir copied and a `.gsd-runtime` of `codex`, linked as `<tmp>/.codex/gsd-core`. The shared scratch oracle was not modified.
  - A bare copy of `gsd-core` fails with `Cannot find module '../../../scripts/fix-slash-commands.cjs'`, because it is a source checkout; hence the mirror.
  - A direct probe of this Codex-marked 1.15.0 install spells `$gsd-new-milestone` with GSD_RUNTIME unset and `/gsd-new-milestone` with `GSD_RUNTIME=claude`. That confirms both the marker and the pin take effect.

## Gates

- `rtk proxy cargo test --no-fail-fast`: 56 suites, **2705 passed, 1 failed, 15 ignored**. The baseline after gtl was 2694, plus 11 new tests (7 queue_md, 3 conformance, 1 codex unit).
  - Lib: 1845 passed, 1 failed, 1 ignored.
  - The only failure is `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`, the known local git-version witness.
  - No "Text file busy" flake this run; envelope_carrier_reach passed 39/39.
- `rtk proxy cargo clippy --all-targets -- -D warnings`: clean.

## Decisions (INFERRED; the human was unavailable, so these are for later audit)

- **INFERRED: SCRUB `GSD_RUNTIME` from the codex child; never set it to `codex`.**
  - An inherited value describes the launching session. That is the same class as the ID-8 `CODEX_THREAD_ID` scrub.
  - 1.15.0 (#4717) ranks it above `config.runtime` and the install marker.
  - Once it is removed, the child's own GSD resolves its runtime from `config.runtime`, then the marker of the install it loads (the Codex one, post-#4667), then host detection.
  - Setting it would override an explicit project `runtime`, which #4717 deliberately never does. It would also couple the manager into GSD's runtime ladder, the inverse of ID-2.
  - Envelope entries still apply after the scrub, so the decision is reversible.
- **INFERRED: honour a non-empty `CODEX_HOME`** for the Codex home candidate in both resolvers. An empty value falls back to `~/.codex`, mirroring upstream `${CODEX_HOME:-$HOME/.codex}`.
- **INFERRED: add the project-local `<root>/.codex/gsd-core` candidate** to the production resolver only. It is upstream's third runtime-root candidate. The conformance fixtures never carry a project-local install.
- **Oracle pin `GSD_RUNTIME=claude`.** This is a comparison harness asking for the canonical spelling the rule table stores. For `init.manager` the runtime feeds only `formatGsdSlash`, so the pin changes spelling, not routing.

## Deviations from Plan

- None in code or behaviour.
- The plan's Task 3 text says "Do not commit (the batch coordinator owns commits)". That conflicts with the operator constraint to commit each task atomically, and the constraint won: code, test and README changes are committed per task. The PLAN and SUMMARY are left uncommitted for the orchestrator.

## Out of scope: observed, NOT fixed (follow-ups)

- `CLAUDE_CONFIG_DIR` is not honoured for the Claude home candidate (pre-existing).
- Upstream's other 14 runtime homes (Gemini, OpenCode, Cursor, ...) are not probed.
- The PATH arm is not gated on `gsd-tools runtime-identity` (#4834).
- `src/executor/claude.rs` scrubs only `CLAUDE*`, so an inherited `GSD_RUNTIME=codex` still reaches Claude children. This is the mirror of this item; the catalog scoped item D to the codex child.

## Threat model

- T-gtm-01 (GSD_RUNTIME spoofing): mitigated by the scrub, with a unit test and an end-to-end test.
- T-gtm-04 (env log disclosure): mitigated. The sed logs names only.
- T-gtm-05 (oracle tampering): mitigated by the pin.
- T-gtm-02 and T-gtm-03: accepted as planned.
- No new surface beyond the threat register.

## Self-Check: PASSED

All six modified files exist. Commits a7948ca, 459ed05, 3bc0137, d5bd8d7, fb29df0 and ea92175 are on master, and `git rev-list --count f98696a..HEAD` = 6.
