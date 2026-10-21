---
phase: quick-261021-0jg
plan: 01
subsystem: driverctl / ci
tags: [macos, d-31, clippy, cross-target, socket]
requires: [quick-261020-vxg]
provides:
  - socket_base_dir (macOS TMPDIR fallback) and platform MAX_SUN_PATH
  - cross-target clippy over --all-targets in release.yml
affects: [src/driverctl/serve.rs, .github/workflows/release.yml]
key-files:
  modified:
    - src/driverctl/serve.rs
    - src/driverctl/mod.rs
    - src/cli.rs
    - tests/driverctl_serve.rs
    - src/app.rs
    - src/driver/direction.rs
    - .github/workflows/release.yml
    - .planning/phases/28-sandboxed-driver-launcher/28-04-SUMMARY.md
    - .planning/phases/28-sandboxed-driver-launcher/28-CONTEXT.md
decisions:
  - "socket_base_dir is the one authority for the socket base; socket_path_in keeps every private-directory check [inferred — audit]"
  - "A set-but-relative XDG_RUNTIME_DIR never falls through to TMPDIR; a relative TMPDIR is refused, not skipped [inferred — audit]"
  - "TMPDIR read via env var, never std::env::temp_dir() (silent /tmp fallback) [inferred — audit]"
status: complete
commits: 4
plan_head_before: db1771fd2d76a3380bb35fd7ec610cf15bef9bdd
plan_head_after: 794af4b2dfe4c71ba96a7c4bd194e47d3a2b4434
actuals:
  tokens: 14000
  tasks: 3
  commits: 4
---

# Quick 261021-0jg: cross-target clippy cleanup and macOS --unsandboxed (D-31)

On macOS the default driverctl socket base now falls back to an absolute `$TMPDIR` when `XDG_RUNTIME_DIR` is unset, with `MAX_SUN_PATH` 103 there. The four Windows-dead test helpers are gated to their callers, and CI clippy now runs `--all-targets -D warnings` on all three foreign targets.

## Commits

| Commit | Type | Content |
| ------ | ---- | ------- |
| 518c2bb | test | 7 RED tests for `socket_base_dir`; over-limit test asserts `MAX_SUN_PATH` |
| bd7aefe | feat | `socket_base_dir`, cfg-split `MAX_SUN_PATH`, docs/help, Linux integration test sets TMPDIR |
| c7e5ac6 | fix | cfg-gate 4 helpers; release.yml clippy `--all-targets` |
| 794af4b | docs | D-31 re-assessment in 28-04-SUMMARY.md and 28-CONTEXT.md (append-only) |

## Verification

- Linux `cargo test --no-fail-fast`: passed=4079 failed=1 ignored=24. The one failure is `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against` (expected locally).
- `cargo clippy --all-targets -- -D warnings`: exit 0 on Linux, x86_64-apple-darwin and x86_64-pc-windows-gnu.
- release.yml YAML assertion: ok. No `allow(dead_code)` added.
- Phase 28 diff: 17 added lines, 0 deleted.

## Deviations from Plan

None. `cargo fmt` reflowed one assertion in the over-limit test before the GREEN commit.

## Known Stubs

None.

## Self-Check: PASSED
