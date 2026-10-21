---
phase: quick-261021-1if
plan: 01
subsystem: driverctl
tags: [unix-socket, macos, sun_path, sha2]
requires: [phase-27-D-21, phase-28-D-31]
provides: [fixed-length default socket name]
affects: [driver launch, driverctl serve]
key-files:
  modified:
    - src/driverctl/serve.rs
    - tests/driverctl_serve.rs
    - src/cli.rs
    - .planning/phases/28-sandboxed-driver-launcher/28-04-SUMMARY.md
    - .planning/phases/28-sandboxed-driver-launcher/28-CONTEXT.md
decisions:
  - "D-21 name kept when it fits; otherwise fixed-length <16 hex>.sock in the same private dir [inferred — audit]"
  - "Explicit --sock is never rewritten; stays a hard refusal [inferred — audit]"
status: complete
commits: 4
plan_head_before: 2c76287e464203f5873afb1b67783bc49a3953f8
plan_head_after: dffb6c906db753c5ce45285d24067dc1b03b01af
actuals:
  tokens: 14000
  tasks: 3
  commits: 4
---

# Phase quick-261021-1if Plan 01: Remove the macOS socket-path run-id length limit Summary

D-21 socket names are kept where they fit; over-limit ones fall back to a fixed-length `<16 hex of sha256(run id, NUL, run dir)>.sock` in the same private 0700 `gsdmm-driverctl` directory, so a stock macOS TMPDIR takes any valid 80-character run id.

## Commits

- 7fcdb7f test: RED (unit + real-binary tests)
- 7070462 feat: `default_socket_file`, `socket_name_hash`, named limits, wiring through `socket_path_in`
- 59fc4c9 docs: module fact 4, rustdoc, `--sock` help
- dffb6c9 docs: Phase 28 D-31 record (append-only)

## Numbers

- Stock macOS base `/var/folders/zz/.../T/` is 49 bytes; an 80-character run id gives an 86-byte path (limit 103).
- Fixed overhead past the base is 38 bytes, so the largest base that fits is 65 bytes (66 is a typed refusal).
- Linux: D-21 name fits up to a 62-character id under `/run/user/1000`; 63 to 80 characters now resolve to the fixed-length name instead of being refused.

## Gates

- `cargo test --no-fail-fast`: 88 suites, 4090 passed, 1 failed, 24 ignored. The one failure is the expected `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`.
- clippy `-D warnings`: host 0, x86_64-apple-darwin 0, x86_64-pc-windows-gnu 0. `cargo fmt --check` clean.
- Cargo.toml, Cargo.lock and src/ui unchanged versus 2c76287. No `DefaultHasher` / `std::hash` in serve.rs.
- Phase 28 files: 0 deletions (9 and 1 added lines).

## Deviations from Plan

None - plan executed as written. `cargo fmt` reformatted two long lines in the new code before the GREEN commit.

## Known Stubs

None.

## Threat Flags

None. The fallback changes only the file name inside the already-vetted private directory.

## Self-Check: PASSED

Commits 7fcdb7f, 7070462, 59fc4c9, dffb6c9 are on the branch; modified files exist.
