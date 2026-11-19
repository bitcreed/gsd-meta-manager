---
phase: quick-261119-ikn
plan: 01
subsystem: gsd-core-sync
tags: [sync, config, readers, conformance]
requires: []
provides:
  - "workflow.cross_runtime_execution typed editable Bool Defaults row (since v1.22.0)"
  - "execute.models.<runtime> recorded as unmasked pass-through"
  - "namespaced commit_docs / search_gitignored help says the flat key wins"
  - "ROADMAP real-ATX-heading reader (#5880) and phase-token VERIFICATION/UAT resolver (#5931)"
  - "baseline trio 1.22.0 / v1.22.0 / 1.22.0, README note, sync record with v1.22.0 delta"
affects: [src/state_reader/config_json.rs, src/ui/screens/detail.rs, src/state_reader/roadmap_md.rs, src/state_reader/disk_status.rs, docs/GSD-CORE-SYNC.md, README.md]
key-files:
  modified:
    - src/state_reader/config_json.rs
    - src/ui/screens/detail.rs
    - src/state_reader/roadmap_md.rs
    - src/state_reader/disk_status.rs
    - docs/GSD-CORE-SYNC.md
    - README.md
key-decisions:
  - "I-ikn-1..11 as recorded in the plan; all applied as written"
status: complete
completed: 2026-11-19
plan_head_before: 8fa367e3
plan_head_after: 157d6b5f
actuals:
  tokens: 120000
  tasks: 3
  commits: 5
---

# Quick 261119-ikn: Sync against gsd-core v1.22.0

## Commits

- 51d0520c feat: model workflow.cross_runtime_execution as a typed row
- 6b3f8849 fix: flat commit_docs / search_gitignored keys win (help text)
- 6de749e0 fix: ROADMAP phase headings from real ATX headings only (#5880)
- a8a4a2ca fix: phase's own VERIFICATION / UAT report by dir token (#5931)
- 157d6b5f chore(sync): baseline moved to v1.22.0

## Re-measured values (all matched the plan except one)

74 commits, 482 files, key cells 264 -> 266 (+2/-0), npm 1.22.0 gitHead
1f800eff..., secret heuristic 6 -> 6, CLI manifest 94 verbs with global and
undeclared unchanged, commands 72 files / 17 modified / no argument-hint
change, agents 64 / 20 modified, capabilities 45 version-only. Step 3 count
is 173 against v1.22.0 (the plan said 174; measured after
workflow.cross_runtime_execution became a row). All five gsd-tools
invocations re-run against installed 1.22.0: exit 0.

## Deviations

- `upstream_normalize_phase_name` was made a faithful port of
  `normalizePhaseName` (letter suffix, uppercase project-code strip, custom
  ids unchanged), as the plan allowed; existing tests stay green.
- The roadmap_milestones local `any_heading` regex was also tightened to at
  most three leading spaces, so an indented heading cannot open a milestone.
- The Docs & Output sentence about flat-vs-namespaced keys was moved from
  commit A to commit B (its intended place) before committing.
- `ui::screens::driver_run` status-token tests are wall-clock sensitive and
  failed once under load; they pass alone and in every full run.

## Gates

- `cargo test --locked --no-fail-fast`: 3884 passed, 0 failed, 19 ignored
  (summed raw `test result:` lines).
- `cargo clippy --all-targets --locked -- -D warnings`: clean.
- `cargo fmt --check`: clean.
- Conformance: 8 passed against installed 1.22.0; 8 passed against a
  throwaway-HOME npm-installed `@opengsd/gsd-core@1.22.0` (`ORACLE READY`,
  `action : installed @opengsd/gsd-core@1.22.0`).

## Release-time obligation

Run `./scripts/pre-tag-check.sh --container vX.Y.Z` to certify the 1.22.0
oracle pin; not run here (needs a release tag). Nothing pushed, no tag.
