# Phase 28: Sandboxed Driver Launcher - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-10-19
**Phase:** 28-sandboxed-driver-launcher
**Areas discussed:** Process model & stop, Caps & real-driver acceptance, Sandbox & per-run config, Auth, Push step, Degraded mode
**Mode:** unattended — operator unavailable; Claude selected every option from planning artifacts. All are `[inferred — audit]` unless CONTEXT.md cites a source.

---

## Process model & stop

| Option | Description | Selected |
|--------|-------------|----------|
| Self-daemonizing launcher holding the lock, embedded RunStore thread | Matches spike + 27 hand-offs 3/4 | ✓ |
| Spawn `driverctl serve` as a child | Refused by the launcher's own lock (hand-off 4) | |
| systemd service unit for the launcher itself | More moving parts; scope is enough | |

**Choice:** daemonizing launcher; stop = SIGTERM launcher → stop scope; direct scope stop if launcher dead; TUI stop is a thin call from the existing Driver screen.

## Caps & real-driver acceptance (carried risk)

| Option | Description | Selected |
|--------|-------------|----------|
| Re-run only scripted L1-L7 on Haiku | Does not discharge the carried risk | |
| Opus run2 variant at L=2 | r261-run2b showed self-park at count 2 → deny unreachable | |
| Opus run2 variant at L=0, organic deny required, kill organic-or-injected | Reaches the deny on the round-1 dispatch Opus does attempt | ✓ |

**Choice:** L=0 with seed v2; pre-registered rubric, 1 run + 1 retry, $20 ceiling; ADJUST and carry to Phase 29 if unreached.

## Sandbox & per-run config

| Option | Description | Selected |
|--------|-------------|----------|
| Config dir under data_local_dir (outside repo) | Same rule as envelope D-02 | ✓ |
| Config dir inside run dir | Run dir is read-only in the sandbox | |

**Choice:** pure argv/settings builders with golden tests in CI; live isolation probes opt-in.

## Auth

| Option | Description | Selected |
|--------|-------------|----------|
| Stored setup-token file 0600 under config dir + host access-token fallback | No new deps; spike-validated | ✓ |
| OS keyring | New dependency, platform variance | |

**Choice:** revoked → re-read host token and relaunch (≤2) on host path; park needs-human on stored-token path.

## Push step

| Option | Description | Selected |
|--------|-------------|----------|
| Auto at terminal status + manual `driver push`, `refs/heads/gsd-auto/<alias>/<run-id>`, PR off by default | Reuses SAFE-01/03/06 and existing namespace | ✓ |
| Push per item | More pushes, more scan runs, no requirement for it | |

## Degraded mode

| Option | Description | Selected |
|--------|-------------|----------|
| Per-launch `--unsandboxed` flag, posture as additive frontmatter key + TUI badge | Explicit every time; additive schema | ✓ |
| Config-level opt-in | Silent persistent downgrade | |

**Note:** macOS degraded mode is blocked by the Linux-only splice (27 D-12); flagged for audit.

## Claude's Discretion

Module layout, std threads vs tokio for the monitor, exact scope-stop mechanism, plan split.

## Deferred Ideas

tmux/interactive attach; macOS functional sandbox and `renamex_np`; stall-watch template setting (Phase 29); Codex under the launcher.
