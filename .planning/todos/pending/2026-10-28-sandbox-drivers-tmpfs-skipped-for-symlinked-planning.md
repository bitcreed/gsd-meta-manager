---
created: 2026-10-28T00:00:00.000Z
title: Sibling-run tmpfs is skipped when .planning or the drivers dir is reached through a symlink (28-REVIEW WR-02)
area: driver
severity: major
source: .planning/phases/28-sandboxed-driver-launcher/28-REVIEW.md (WR-02), deferred by quick 261028-n12 [inferred — cleanup 2026-10-28]
files:
  - src/launcher/sandbox.rs (bwrap_argv drivers tmpfs)
  - src/driverctl/serve.rs (validate_run_dir)
  - tests/launcher_live_isolation.rs
---

## Problem

`bwrap_argv` hides the sibling runs by mounting a tmpfs over the project's drivers
directory, but only when `run_dir.parent() == project.join(DRIVER_RUNS_REL)`.
`validate_run_dir` returns the **canonical** run dir, so when `.planning` (or the
drivers dir) is a symlink, for example `.planning -> docs/planning`, the parent is
`<project>/docs/planning/drivers` and never equals the un-resolved
`<project>/.planning/drivers`. The tmpfs is skipped with no message, and a sandboxed
driver can read and write its sibling runs' `DIRECTION.md`, `LOG.md` and `state.json`.

The golden test `no_drivers_tmpfs_unless_the_run_dir_sits_directly_under_the_projects_drivers_dir`
deliberately pins the silent skip, including the out-of-project `/other/.planning/...`
case, so a fix has to change that test on purpose.

## Solution

The review's `ends_with(DRIVER_RUNS_REL)` is not enough: with the symlink above the
components compare `planning` with `.planning` and still differ. A design choice is
needed [inferred — cleanup 2026-10-28]:

- carry the canonical drivers dir, which `validate_run_dir` already computes, into
  `SandboxPaths`, emit the tmpfs at it whenever it is set, and refuse the launch in
  production when it is unset; or
- refuse the launch outright when the canonical run dir is not directly under the
  project's literal `.planning/drivers`.

Update the golden test, and add a symlinked-`.planning` case to the live isolation
test showing a sibling run is not visible from inside the sandbox.
