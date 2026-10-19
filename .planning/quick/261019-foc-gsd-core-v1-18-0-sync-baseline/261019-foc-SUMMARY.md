---
phase: quick-261019-foc
plan: 01
subsystem: gsd-core-sync
tags: [sync, baseline, gsd-core, v1.18.0]
requires: [261016-dx3]
provides: [gsd-core v1.18.0 sync baseline, GSD_HOME resolveGsdHome citation]
affects: [src/state_reader/config_json.rs, docs/GSD-CORE-SYNC.md, README.md]
key-files:
  modified:
    - src/state_reader/config_json.rs
    - docs/GSD-CORE-SYNC.md
    - README.md
decisions:
  - "I-foc-1..7 (see Inferred decisions) plus I-foc-8 (worktree isolation forced to none)"
metrics:
  tasks: 1
  files: 3
status: complete
commits: 1
plan_head_before: 084f1fa65df63346acb7bb4b60827a336522e892
plan_head_after: 8c3dea2bb380030de1fc3e160d6482442cc08fa6
actuals:
  tasks: 1
  commits: 1
---

# Quick 261019-foc: gsd-core v1.18.0 sync baseline Summary

The sync baseline moved from v1.17.0 to v1.18.0 (`GSD_CORE_SYNCED_VERSION` 1.18.0, `COMMIT` v1.18.0, `TREE_VERSION` 1.18.0) across config_json.rs, README.md and docs/GSD-CORE-SYNC.md in one commit (`8c3dea2`), with the measured v1.17.0..v1.18.0 delta recorded and the `GSD_HOME` docstring re-cited to `resolveGsdHome` (#5017).

## Re-measured values (all matched the planner's numbers; no deviations)

- `v1.18.0` sha 35f5a452239b897b8d6352c973e1e4efd683370f, tagged 2026-10-19, `package.json` 1.18.0, `npm view` 1.18.0.
- CONFIGURATION.md key cells 255 -> 255, sorted sets identical (+0 / -0).
- No diff in config-schema/defaults manifests, `src/config.cts`, `src/secrets.cts`, `gsd-run-resolver.md`.
- `src/config-loader.cts` +3/-3 and `src/runtime-homes.cts` +18: `resolveGsdHome(fallbackHome = os.homedir())` returns `process.env['GSD_HOME'] || fallbackHome`.
- 45 capability.json files changed; the only non-`version` lines are `defaultHomeIncludeRoot` (antigravity `.gemini/antigravity`, copilot `.copilot`) and kimi `retargetsGsdCoreProsePath: true`.
- Secret heuristic over v1.18.0 key cells matches only the four exempt budget keys.
- Local gsd-core checkout is on `next` at `v1.18.0-6-g8d3d29c46` (recipe parenthetical refreshed accordingly).

## Verification

- Three tie tests: 3 passed.
- `cargo test --no-fail-fast`: 3209 passed, 1 failed, 18 ignored across all binaries. The sole failure is the expected local-only `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`; not fixed.
- `cargo clippy --all-targets -- -D warnings`: exit 0.
- Plan CONSTANTS-OK / README-OK / RECORD-OK gates all printed.

## Deviations from Plan

None - plan executed exactly as written.

## Inferred decisions (for audit)

- I-foc-1: One task, one commit (tracer slice is the whole item).
- I-foc-2: Only the three listed files change; the grep for other 1.17.0 pins found none tied to the constants (oracle script derives from the constant, release.yml has no literal, fixtures read the constants; remaining src/ 1.17.0 strings are "introduced in 1.17.0" citations).
- I-foc-3: kimi `retargetsGsdCoreProsePath: true` recorded as a runtime-only field beside `defaultHomeIncludeRoot`.
- I-foc-4: One Pass-through row for `ponytail.*` / `musk.*`; `## Modelled` untouched.
- I-foc-5: Recipe moved to `REF=v1.19.0` / `PREV=v1.18.0`, parenthetical refreshed.
- I-foc-6: `./scripts/pre-tag-check.sh --container` not run (needs a release tag). The next release MUST run `./scripts/pre-tag-check.sh --container vX.Y.Z` to certify the 1.18.0 oracle. Meanwhile `tests/driver_router_conformance.rs` ran green against the locally installed gsd-core 1.18.0.
- I-foc-7: GSD_HOME docstring keeps "non-empty" (upstream code is `||`, not a trim).
- I-foc-8: Worktree isolation was forced to none because the base-check returned null values (origin/HEAD -> origin/dev; local master 154 commits ahead of origin/master); executed sequentially on master.

## Known Stubs

None.

## Self-Check: PASSED

Commit `8c3dea2` exists on master and touches exactly the three files; `.planning/state.json` left untracked and untouched.
