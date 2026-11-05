---
phase: quick-261105-e7u
plan: 01
subsystem: gsd-core-sync
tags: [sync, baseline, gsd-core, v1.20.1]
requires: []
provides:
  - "gsd-core sync baseline at v1.20.1 (constants, README note, sync record)"
affects: [src/state_reader/config_json.rs, docs/GSD-CORE-SYNC.md, README.md]
key-files:
  modified:
    - src/state_reader/config_json.rs
    - docs/GSD-CORE-SYNC.md
    - README.md
decisions:
  - "I-e7u-1..7 (see below)"
metrics:
  completed: 2026-11-05
status: complete
commits: 1
plan_head_before: 631e4fc527e3b2503325632ea2c674afcb4bc705
plan_head_after: f7e6fefc402a3c6e40d8c9e84b947ea750b9b201
actuals:
  tokens: 14000
  tasks: 1
  commits: 1
---

# Phase quick-261105-e7u Plan 01: Sync baseline to gsd-core 1.20.1 Summary

Moved the gsd-core sync baseline from v1.20.0 to v1.20.1 (constants 1.20.1 / `v1.20.1` / 1.20.1, README note, sync record with a new v1.20.1 delta section and the REF=v1.21.0 / PREV=v1.20.1 recipe) in one commit, `f7e6fef`.

## Re-measured values (all matched the planner's numbers; no deviations)

- Tag `v1.20.1` = `9385f4618b52ef68ed8a5e8bdd93cfb25bd87765`, dated 2026-11-05T14:28:43Z; `package.json` 1.20.1; `npm view @opengsd/gsd-core version` = 1.20.1; installed `~/.claude/gsd-core/VERSION` = 1.20.1.
- 20 commits v1.20.0..v1.20.1.
- CONFIGURATION.md key cells 263 -> 263 (+0/-0); no diff in schema/defaults manifests, config template, config.cts, secrets.cts, config-loader.cts, gsd-run-resolver.md, agents/, smart-entry.cts, tests/fixtures.
- 45 capability.json files changed, 0 non-`version` lines.
- commands/gsd: only new-project.md and new-milestone.md; src/init.cts has a 13-line diff (convention threading).
- `.gsd` claims confirmed: both hooks/lib/isolation-sentinel.js and src/worktree-base-ref.cts name `.gsd` at v1.20.0; `src/gsd-tool-dir.cts` exists at v1.20.1.
- App side: `dispatch-isolation|harness-fork|fork-probe` hits only src/launcher/assets/MASTER-PROMPT.md; `gsd-file-manifest` has no hit in src.
- Upstream checkout HEAD: `next` at `v1.20.1-24-g9d6b6c728` (24 commits past the tag), package.json 1.20.1.

## Verification

- Three tie tests pass (3 passed, 0 failed).
- CONSTANTS-OK, README-OK, RECORD-OK all printed.
- `rtk proxy cargo test --no-fail-fast`: exit 0; summed raw `test result:` lines across 59 suites = **3623 passed, 0 failed, 17 ignored**. `tests/driver_router_conformance.rs` ran (6 passed, not skipped) against installed gsd-core 1.20.1.
- `cargo clippy --all-targets -- -D warnings`: exit 0. `cargo fmt --check`: clean.

## Deviations from Plan

None - plan executed exactly as written.

## Inferred decisions (for audit)

- I-e7u-1 One task, one commit; the tie tests require the three files to move together.
- I-e7u-2 No dependency on 261105-e7v; the record names it owner of the milestone-prefixed port and never claims this build recognises those ids.
- I-e7u-3 `.gsd` files already lived under `<project>/.gsd/` at v1.20.0; the record states the real change (#5682 keyed records, #5757 single owner) rather than the item text.
- I-e7u-4 CLI/routing/codex nuance recorded as measured; none needs code.
- I-e7u-5 Recipe REF stays v1.21.0, PREV moves to v1.20.1; stale "at the time of writing" describe refreshed.
- I-e7u-6 `./scripts/pre-tag-check.sh --container` NOT run (needs a release tag). **Release-time obligation:** the next release must run `./scripts/pre-tag-check.sh --container vX.Y.Z` to certify the 1.20.1 oracle pin.
- I-e7u-7 "Next sync" item 8 (custom/bracket phase identity) unchanged.

## Known Stubs

None.

## Threat Flags

None.

## Self-Check: PASSED

- config_json.rs, GSD-CORE-SYNC.md, README.md modified and present; commit `f7e6fef` is an ancestor of HEAD and touches exactly those three files.
