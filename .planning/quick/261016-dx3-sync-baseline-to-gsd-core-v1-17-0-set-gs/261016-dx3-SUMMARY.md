---
quick_id: 261016-dx3
phase: quick-261016-dx3
plan: 01
subsystem: gsd-core-sync
tags: [sync, baseline, gsd-core, v1.17.0]
status: complete
commits: 1
plan_head_before: e1ab9adf78a67c884750dca2f4b89e8693cfc62c
plan_head_after: a254c32ce31e3ad59239f1e15c4c2ba036978f39
key-files:
  modified:
    - src/state_reader/config_json.rs
    - docs/GSD-CORE-SYNC.md
    - README.md
actuals:
  tasks: 1
  commits: 1
---

# Quick 261016-dx3: Sync baseline to gsd-core v1.17.0

The gsd-core sync baseline moved from v1.16.0 to v1.17.0 across the three constants, README's compatibility note and the sync record, in one commit. The delta changes no config key, default or loader.

**Commit:** a254c32 `chore(sync): baseline gsd-core v1.17.0 (quick 261016-dx3)`. It touches exactly the three files above.

## Re-measured values (all matched the planner's numbers; no deviation)

- `git describe --tags v1.17.0` = `v1.17.0`.
- Full sha 962c1192e062b5278281cd3a7bfd671a67a4db44 (lightweight tag). Tagged 2026-10-14.
- `package.json` is 1.17.0 and `npm view @opengsd/gsd-core version` = 1.17.0.
- CONFIGURATION.md key cells went 254 -> 255. The only addition is `review.effort.opencode`; nothing was removed.
- The following are unchanged: the schema and defaults manifests, `src/config-loader.cts`, `src/config.cts`, `src/secrets.cts` and `gsd-run-resolver.md`.
- 45 capability.json files changed. All 45 bump `version`, and 8 change more than that.
- Seven runtimes gained `unsupportedFeatures: ["context-warnings"]`: antigravity, augment, codebuddy, hermes, kimi-code, kimi and qwen.
- opencode changed `dispatch.nested` and `dispatch.maxDepth` from "undocumented" to `false` / `1`, and gained a `nativeCliPlugin` block.
- The secret-heuristic key cells are the same four budget keys.
- The local checkout is on `next` at `v1.17.0-75-g46a119f06`, `package.json` 1.17.0.

## Verification

- The three tie tests pass (3 passed). The CONSTANTS-OK, README-OK and RECORD-OK checks print.
- `cargo clippy --all-targets -- -D warnings` exits 0.
- The full `cargo test --no-fail-fast` run has 2099 passed, 1 failed, 1 ignored in the lib test target, and every other test target is ok.
- The single failure is the expected `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`.
- `tests/driver_router_conformance.rs` passed against the installed gsd-core 1.17.0.

## Inferred decisions (audit)

- I-dx3-1: One task and one commit, because the constants, README and record must move together.
- I-dx3-2: `review.effort.opencode` is recorded as measured. The family has been capability-registered since v1.13.0 (5ad9a36f3, #4255). v1.17.0 only adds its first leading CONFIGURATION.md key cell. It is listed under Pass-through, and Modelled and DEFAULTS_OPTION_COUNT stay 135.
- I-dx3-3: The recipe's "local checkout at time of writing" parenthetical was refreshed to `next` / `v1.17.0-75-g46a119f06` / 1.17.0.
- I-dx3-4: The `review.effort.<slug>` family is also named in the ReviewConfig doc comment and in Next sync step 3.
- I-dx3-5: `./scripts/pre-tag-check.sh --container` was not run. The next release must run `./scripts/pre-tag-check.sh --container vX.Y.Z` to certify the 1.17.0 oracle pin.

## Deviations from Plan

None - plan executed exactly as written.

## Known Stubs

None.

## Self-Check: PASSED

The three files are in commit a254c32, and `git rev-list --count e1ab9ad..HEAD` = 1.
