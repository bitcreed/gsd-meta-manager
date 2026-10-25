# Phase 22: Container Execution Target — Superseded

> **Superseded.** On Linux, bwrap plus `systemd-run --user --scope` (Phase 28) replaces the container execution target, verified with auth in spikes 001/002. Phase 28.1 adds the egress and `$HOME` isolation. **0 of 4 plans were executed, no code landed, and `bollard` was never added.** The plans, research and spike notes in this directory are kept unchanged for provenance. See `.planning/ROADMAP.md` (Phase 22 banner) and `.planning/REQUIREMENTS.md` (CTNR-01..05 traceability rows).

## What replaced it

Phase 22 (CTNR-01..05) planned to run a driven project inside a docker or podman container. On 2026-10-14 it was superseded by the Phase 28 launcher sandbox:

- `bwrap` under `systemd-run --user --scope` for the process and cgroup boundary;
- no push credentials inside the sandbox, with a read-only run directory (LNCH-04);
- commits leaving the machine only through gsd-mm's fixed-refspec push step outside the sandbox (LNCH-06);
- the Phase 28.1 egress proxies and the `$HOME` allowlist on top (LNCH-09, LNCH-10, LNCH-12).

macOS is covered by Phase 28's degraded mode, not by containers `[inferred]`.

## State of the work

| Item | State |
|---|---|
| 22-01..04-PLAN.md | never executed (0/4); each now carries `status: superseded` and `superseded_by: phase-28` in its frontmatter |
| code under `src/` for a container target | none landed |
| `bollard` dependency | never added; `grep -ci bollard Cargo.toml Cargo.lock` prints 0 and 0 on the Phase 31 tree |
| CTNR-01..05 | superseded in REQUIREMENTS.md, deferred |

## Why the directory stays here

The directory is not moved or renamed. ROADMAP links point into it and `gsd-tools` resolves a phase by its directory name, so moving it would break both. Physical archival belongs to `/gsd-complete-milestone`.

Written by Phase 31 plan 06 (RETIRE-02) on 2026-10-25.
