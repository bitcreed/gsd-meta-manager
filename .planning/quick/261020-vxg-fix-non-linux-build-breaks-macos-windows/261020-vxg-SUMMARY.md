---
phase: quick-261020-vxg
plan: 01
subsystem: build-portability
tags: [macos, windows, cfg-gating, ci, sandbox-fail-closed]
status: complete
requirements: [QUICK-261020-vxg]
commits: 3
plan_head_before: 80354092f9b8d2b80ad7d4fb2e1fbd3e4e5c6d4e
plan_head_after: 946508b79abebfdc37c4dc5e0817ff0a015041cf
actuals:
  tokens: 60000
  tasks: 3
  commits: 3
key-files:
  modified:
    - src/launcher/host.rs
    - src/state_reader/config_write.rs
    - src/driverctl/scan.rs
    - src/launcher/sandbox.rs
    - .github/workflows/release.yml
    - README.md
---

# Quick 261020-vxg: Build on macOS and Windows

The crate now compiles for macOS and Windows, the macOS launcher takes the real `--unsandboxed` degraded path, and a CI job gates `publish` on cross-target checks.

## Commits

| Task | Commit | What |
|------|--------|------|
| 1 | af80271 | macOS: fail-closed host probe, launcher stack widened to Linux+macOS |
| 2 | 15289fe | Windows: unix-only code gated, config save and scan fail closed |
| 3 | 946508b | CI cross-target-check job gating publish, README Windows note |

## Results

- **macOS (x86_64-apple-darwin):** `cargo check --all-targets --locked` exits 0. `cargo clippy --lib --bins --locked -- -D warnings` exits 0. `clippy --all-targets` reports only the two out-of-scope helpers (`src/app.rs:8460`, `src/ui/screens/detail.rs:20043`).
- **Windows (x86_64-pc-windows-gnu):** `check --lib --bins` exits 0. `check --all-targets` exits 0, with 6 test-only dead-code warnings (the two foreign helpers, `app.rs:8209` and `:8237`, `cli.rs:1395` `OsStringArg`, `direction.rs:686` `block`). `clippy --lib --bins -D warnings` exits 0.
- **Linux:** `cargo test --no-fail-fast` gives 4027 passed, 1 failed, 24 ignored. The baseline was 4023 / 1 / 24, so the delta is the 4 new `host.rs` tests. The single failure is the known `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`. `cargo clippy --all-targets -- -D warnings` exits 0.
- `src/launcher/sandbox.rs`: 2 added lines, 0 deleted. `src/app.rs`, `src/ui/screens/detail.rs`, `Cargo.toml` and `Cargo.lock` are untouched.

## Deviations from Plan

None in structure. Details beyond what the plan spelled out:

- **[Rule 3 - Blocking] Windows `--all-targets` needed 7 `#![cfg(unix)]` test files**, as the plan anticipated: `tests/driver_dry_run.rs`, `driver_injection_corpus.rs`, `driver_model_seam.rs`, `envelope_carrier_reach.rs`, `envelope_interior_path.rs`, `envelope_word_set.rs`, `spawn_seam_guard.rs`. They use `std::os::unix` or `executor::claude`, which is cfg(unix).
- **[Rule 3 - Blocking] Windows clippy lib+bins** needed dead-code cleanup, done with the plan's cfg discipline: `#[cfg(unix)]` on `executor/mod.rs` items whose only users are cfg(unix) modules (`WriterCommand::Line`, three `ExecutionHandle` fields, `write_line`, `next_control_id`, `encode_interrupt`, the `ControlRequest` import, one test), `GoalRefusal::empty_plan`, and the `driver_cli.rs` imports. For `driverctl_cli.rs` (`CLIENT_TIMEOUT`, three `ClientError` variants) I used `#[cfg_attr(not(unix), allow(dead_code))]`, following the file's own existing precedent on `MAX_REPLY_BYTES` and `UnsupportedPlatform`. This is a targeted per-item attribute, not a blanket suppression.
- `launcher/sandbox.rs` also got `#[cfg_attr(not(linux/macos), allow(unused_mut))]` on `null_masks` (additive only), since gating the token push leaves the `mut` unused on Windows.
- `driver/mod.rs:1210`: the unused-variable warning was fixed with a `cfg_attr(not(unix), allow(unused_variables))` on the non-unix `decomposed` binding, per the file's own comment at :1011.

## Inferred decisions [inferred — audit]

1. The launcher stack is widened to `any(linux, macos)` rather than `unix`; other unixes stay on the refusal stubs (fail closed).
2. `SANDBOX_SUPPORTED = cfg!(target_os = "linux")` forces bwrap, systemd-run and user-systemd missing off Linux, so `decide(_, false)` always refuses and `BwrapScope` is unreachable on macOS whatever sits on PATH. Pinned by 4 tests that run on Linux.
3. Windows config save returns `ErrorKind::Unsupported` before any I/O and never calls `mutate`. The Windows verification scan skips every file (CR-01 guard cannot be reproduced without `O_NOFOLLOW`/`O_NONBLOCK`).
4. `Cargo.toml` and rustix are unchanged. The reported rustix `lib.rs` errors on Windows did not reproduce with `--lib`, `--bins` or `--all-targets`; rustix compiles there without its fs/process modules, and the only errors were this crate's own `std::os::unix` and `rustix::fs`/`process` uses.
5. CI: ubuntu with rustup targets, gnu for Windows, tag-push only (like the rest of the workflow), gates `publish` via `needs`. Windows is checked with `--all-targets` (it is green) and clippy `--lib --bins`. Clippy stays lib+bins because of the two foreign dead helpers; widen it once the master fix lands. `scripts/pre-tag-check.sh` parity is a follow-up.
6. User-facing "needs Linux" text now says "Linux or macOS" in `driver_cli.rs`, `driverctl_cli.rs`, `direction.rs`, `launcher/mod.rs` and docs. Error variants and exit codes are unchanged.
7. TUI-side driver launch on macOS is still Linux-gated in `app.rs` and `ui/screens/driver_launch.rs`, untouched per constraint. Follow-up.
8. macOS runtime behaviour of `driver launch --unsandboxed` is unverified here (no Mac). The operator's 28-UAT item 2 is the check. Watch for: liveness reports Unknown (`LIVENESS_SUPPORTED` is false), store `host_name` is None, and the rustix `RENAME_EXCHANGE` to `renameatx_np(RENAME_SWAP)` splice needs APFS (ENOTSUP elsewhere surfaces as an error, not a partial write).

## Remaining on Windows

`clippy --all-targets -D warnings` would fail on 6 test-only dead-code warnings; 3 are in `app.rs`/`detail.rs` (out of scope), 2 more are cheap (`cli.rs:1395`, `direction.rs:686`) but not worth fixing while the app.rs/detail.rs ones remain. Windows runtime is entirely unexercised.

## Known Stubs

The non-unix `update_gsd_config`/`atomic_write` and `read_regular_bounded` are intentional fail-closed refusals, not unfinished work. No Windows implementation is planned.

## Threat Flags

None. The new CI job reuses only actions already in the file.

## Self-Check: PASSED

Commits af80271, 15289fe, 946508b exist on `worktree-agent-ab1cb1167abca0b32`. Modified files verified present; `git rev-list --count 8035409..HEAD` is 3.
