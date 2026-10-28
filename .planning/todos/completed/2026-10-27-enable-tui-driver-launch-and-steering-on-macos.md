---
created: 2026-10-27T14:51:14.884Z
title: Enable TUI driver launch and steering on macOS (close 28-REVIEW WR-05)
area: ui
severity: minor
files:
  - src/ui/screens/driver_steer.rs:43
  - src/app.rs:2488
  - src/ui/screens/detail.rs:5846
  - src/ui/screens/detail.rs:5874
  - src/ui/screens/driver_launch.rs
  - src/ui/screens/driver_run.rs
  - src/driver/direction.rs
decision: enable
status: completed
completed: 2026-10-28
resolution: fixed
---

## Problem

Priority: low / nice-to-have. Run as a `/gsd-quick` AFTER Phase 31 closes (it touches `src/app.rs` heavily).

Severity `minor` [inferred — audit]: human unavailable at capture; mapped from "low / nice-to-have" (feature works via CLI, TUI path is the annoyance).

2026-10-27 macOS UAT re-run on HEAD 2ea7d77 (`28-UAT.md`, commit d1ff8e6): the CLI `driver launch --unsandboxed` works on Darwin, but the TUI refuses `R`/`s` with "read-only: steering needs Linux". This is 28-REVIEW WR-05. The write primitive already exists on macOS: `src/driver/direction.rs` uses rustix `RENAME_EXCHANGE`, which maps to `renameatx_np` on Darwin.

## Solution

- Widen `can_steer()` (`src/ui/screens/driver_steer.rs:43`) to `any(target_os = "linux", target_os = "macos")` and fix its stale doc comment.
- Widen TUI `#[cfg(target_os = "linux")]` arms to `any(linux, macos)` where they gate launch/steer/stop: `src/app.rs` (~70 sites incl. `DriverLaunchRequested` at :2488), `src/ui/screens/detail.rs` (:5846 `s`, :5874 `R`), `driver_launch.rs`, `driver_run.rs`. Keep genuinely Linux-only ones (bwrap/systemd, mknodat FIFO tests).
- `host_name()` reads `/proc/sys/kernel/hostname` -> use `rustix::system::uname()`/gethostname so the macOS connection record has a host.
- Gate test `cr01_bind_private_never_touches_the_process_umask_and_binds_concurrently` (reads `/proc/self/status`) to Linux, or make it portable.

Verify: cross-target clippy for both apple targets; then on the docker-osx VM (`systemctl --user start osx`, `ssh osx`, procedure in `28-UAT.md` test 2) drive TUI `R` -> `[U]` unsandboxed launch and `s` steering end to end.

Risk: the `renameatx_np` swap needs APFS (HFS+ refuses cleanly).

## Decision (2026-10-28)

The user decided to ENABLE TUI launch and steering on macOS: Phase 28 review WR-05 is resolved as
"enable on macOS". The gate to widen is still `can_steer()` at `src/ui/screens/driver_steer.rs:43-45`
(`cfg!(target_os = "linux")`). Phase 31 has closed, so the "after Phase 31" precondition above is met.

## Resolution — fixed (2026-10-28, 261028-n11)

Commits: 76a9ecb (TUI launch, steering and stop enabled on macOS: `can_steer()` true on Linux and macOS, launch/preflight/stop
arms and tests widened, refusal copy now "Linux or macOS"), de20ec3 (portable `host_name()` via `rustix::system::uname()`,
umask test Linux-gated, test gates widened), 9b68124 (Darwin lib-suite fixes found by the VM run).

Evidence: host `cargo test --locked --no-fail-fast` 3172 passed, 0 failed; `cargo clippy --locked --all-targets -- -D warnings`
exit 0 on the host, x86_64-apple-darwin, aarch64-apple-darwin and x86_64-pc-windows-gnu; `cargo fmt --check` clean. Darwin
(docker-osx VM, Intel): lib suite 2522 passed, 0 failed; real TUI drive of `s`, the launch form, `y` refusal with the `[U]`
offer, the `[U]` second confirmation, a degraded launch with a non-null launcher host, `c` pause, and `x` stop (recorded in
28-UAT.md test 2).

Deliberately kept Linux-only: `SANDBOX_SUPPORTED` (bwrap/systemd), `LIVENESS_SUPPORTED` (/proc), the FIFO tests (need
`mknodat`), the CR-01 umask test and four `driver::liveness` tests (read /proc).
