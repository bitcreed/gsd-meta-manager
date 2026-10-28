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
