---
phase: quick-261113-fi0
plan: 01
subsystem: gsd-core-sync
tags: [sync, config, secrets, conformance]
requires: []
provides:
  - "hooks.secret_read_exempt typed read-only Defaults row (since v1.21.0)"
  - "review.context_window_tokens unmasked; NON_SECRET_CONFIG_KEYS holds 6 exemptions"
  - "baseline trio 1.21.0 / v1.21.0 / 1.21.0, README note, sync record with v1.21.0 delta"
affects: [src/state_reader/config_json.rs, src/state_reader/config_secrets.rs, src/ui/screens/detail.rs, docs/GSD-CORE-SYNC.md, README.md]
key-files:
  modified:
    - src/state_reader/config_json.rs
    - src/state_reader/config_secrets.rs
    - src/ui/screens/detail.rs
    - docs/GSD-CORE-SYNC.md
    - README.md
key-decisions:
  - "I-fi0-1..10 as recorded in the plan; all applied as written"
status: complete
completed: 2026-11-13
plan_head_before: aa1f29b8ac1eff1561d0050bf37e44cab530bf32
plan_head_after: 1ac98325e87304b4dc475a8a2b7516370f612540
actuals:
  tokens: 60000
  tasks: 3
  commits: 4
---

# Phase quick-261113-fi0 Plan 01: Sync against gsd-core v1.21.0 Summary

Models gsd-core v1.21.0's one new config key (`hooks.secret_read_exempt`) as a read-only unmasked row, fixes the v1.20.0 sync miss that masked `review.context_window_tokens`, aligns the `claude_orchestration.*` help texts, and moves the baseline trio, README and sync record to 1.21.0.

## Commits

| Commit | Subject |
|---|---|
| 3d13fd1 | feat(quick-261113-fi0): model hooks.secret_read_exempt as a read-only row (gsd-core v1.21.0) |
| ed60cc9 | fix(quick-261113-fi0): stop masking review.context_window_tokens as a secret |
| 715801c | fix(quick-261113-fi0): align the claude_orchestration help texts with gsd-core v1.21.0 |
| 1ac9832 | chore(sync): move the gsd-core baseline to v1.21.0 (quick-261113-fi0) |

## Re-measured values (throwaway bare clone borrowing the sibling's objects)

All planner numbers confirmed, no deviation: `v1.21.0` = 357508df233ccca0dcf4332b385d6bee9b113da9 (describe `v1.21.0`, 2026-11-13T04:05:13Z, package.json 1.21.0); npm 1.21.0 with `gitHead` equal to that sha; v1.20.1 is an ancestor; 101 commits, 411 files; key cells 263 -> 264 (+1 / -0 `hooks.secret_read_exempt`, b39076563, describe `v1.20.1-59-gb39076563`, first tag `v1.21.0`); schema manifest +1 `validKeys` (at `gsd-core/bin/shared/config-schema.manifest.json`); secret-heuristic matches 5 -> 6; capabilities 45 files, version bumps plus claude/claude-orchestration/mempalace non-version lines; commands/gsd 72 files at both tags, only `autonomous.md` changes; agents 4 files; CLI-SURFACE-MANIFEST.json absent at v1.20.1, present at v1.21.0 (94 verbs, `smart-entry` and `init.manager` undeclared, `--version`/`-v` rejected). Sibling checkout HEAD: `v1.20.1-97-gd668c307b`, package.json 1.20.1, no `v1.21.0` tag. Scratch clone removed.

## gsd-tools invocation audit

All five invocations audited against v1.21.0; run against installed 1.21.0: `smart-entry --json` exit 0; `query init.manager` exit 0; `runtime-identity --raw` prints `{"packageName":"@opengsd/gsd-core","version":"1.21.0"}`; `gsd-tools --version` rejected exit 1 (predates v1.21.0, spawn-only probe). File:line references in the plan were correct. Result: no code change needed.

## Conformance oracle certification

- `tests/driver_router_conformance.rs` against installed `~/.claude/gsd-core` 1.21.0: 8 passed.
- `scripts/install-conformance-oracle.sh` into a throwaway HOME after the pin bump: `ORACLE READY`, `action : installed @opengsd/gsd-core@1.21.0`, probe PASS; conformance test against it: 8 passed. Throwaway HOME removed.
- Release-time obligation (I-fi0-9): run `./scripts/pre-tag-check.sh --container vX.Y.Z` to certify the 1.21.0 oracle pin.

## Verification

- `cargo test --locked --no-fail-fast`: 3876 passed, 0 failed, 19 ignored (summed raw `test result:` lines; lib 3116 passed).
- `cargo clippy --all-targets --locked -- -D warnings`: clean.
- `cargo fmt --check`: clean.

## Deviations from Plan

None - plan executed exactly as written, with two process notes:

- **TDD ordering:** the Task 1 and Task 2 tests were written in the same working session as the implementation and I did not capture a separate failing-run before implementing; they were verified passing and exercise the new behaviour (the Task 1 test cannot pass without the typed row; the Task 2 masking test cannot pass without the exemption).
- **Commit split:** because Task 2's two commits share `detail.rs`, commit A was produced by temporarily reverting the help-text and test-B hunks, then restoring them for commit B (full file preserved byte for byte).

## Inferred decisions [inferred - audit]

I-fi0-1..10 applied exactly as listed in the plan (read-only row; exemptions for `hooks.secret_read_exempt` and `review.context_window_tokens`; `min_agent_sdk_version` keeps its editable row; help texts follow v1.21.0; no change for strict argv, commit refusal, project-context, config-get --default, $HOME walk, .gsd gitignore, "(INSERTED)"; recipe scratch-clone fallback with REF=v1.22.0/PREV=v1.21.0; conformance against both oracles; Next-sync row count 73 -> 72).

## Known Stubs

None.

## Threat Flags

None.

## Self-Check: PASSED

Commits 3d13fd1, ed60cc9, 715801c, 1ac9832 are on master; working tree clean; nothing pushed.
