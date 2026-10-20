# Phase 30: Driver Tab on DIRECTION.md - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-10-20
**Phase:** 30-driver-tab-on-direction-md
**Areas discussed:** Tab structure & legacy surface, Run view contents, Crash/stale & dashboard markers, Steering writes & $EDITOR, Lifecycle (new/relaunch/stop/tail), Spend & verification
**Mode:** unattended. The operator was unavailable, and each area was resolved from the planning artifacts
(ROADMAP, REQUIREMENTS, 27/28/29 CONTEXT, SUMMARY, VERIFICATION, REVIEW and SECURITY, backlog 999.7) with
the recommended option. Inferred choices are tagged `[inferred — audit]` in CONTEXT.md.

---

## Tab structure & legacy surface

| Option | Description | Selected |
|--------|-------------|----------|
| Launcher-run first, legacy collapsed read-only | New view on run dirs; Phase 17/18 runs read-only; injection/legacy start unbound | ✓ |
| Two tabs (legacy + new) | Keep the old tab and add a second | |
| Drop legacy display now | Hide journal runs entirely ahead of Phase 31 | |

**Choice:** launcher-run first (D-01..D-03). The code is deleted in Phase 31.

## Run view contents

| Option | Description | Selected |
|--------|-------------|----------|
| Derived status + body `control` + item merge (body x item_states) | Follows the 27 hand-offs 2 and 4 | ✓ |
| Raw frontmatter fields | Display `status:`/`control:` as written | |

**Choice:** D-04..D-10. Stale heartbeat is a flag, not crashed. The lock is the liveness authority.

## Crash/stale & dashboard markers

| Option | Description | Selected |
|--------|-------------|----------|
| crashed > parked > driven > stale; stale counts as attention | One marker per project from non-terminal runs + latest | ✓ |
| Only crashed/parked count as attention | Stale is informational | |

**Choice:** D-11..D-13.

## Steering writes & $EDITOR

| Option | Description | Selected |
|--------|-------------|----------|
| Edit a body-only temp, apply via write_body with base check | Satisfies "every edit under flock, never frontmatter" | ✓ |
| Open live DIRECTION.md in $EDITOR | Editor save bypasses the flock and can touch the frontmatter | |
| Hold DIRECTION.lock for the editor session | Starves the driverctl splice (2 s budget) | |

**Choice:** D-14..D-17. OQ-R1 is surfaced as a hint, with the semantics unchanged.

## Lifecycle

| Option | Description | Selected |
|--------|-------------|----------|
| TUI calls `launch_detached` (daemon owns lock + socket) | The run outlives the TUI, and the TUI never binds the socket | ✓ |
| Embed launcher/RunStore in the TUI process | Run dies with TUI; socket bind in the multithreaded runtime (CR-01 area) | |

**Choice:** D-18..D-23. `--unsandboxed` needs a separate explicit confirm. Relaunch is offered for crashed and
for terminal stopped/parked/stalled runs.

## Spend & verification

| Option | Description | Selected |
|--------|-------------|----------|
| Zero-spend deterministic acceptance (stub claude, fixtures, TestBackend) | No paid run | ✓ |
| Live Haiku TUI launch smoke | Cents; needs operator sign-off | (optional only) |

**Choice:** D-26..D-28. The UI-SPEC gate is applied.

## Claude's Discretion

Module layout, exact keybindings (subject to the UI-SPEC), dashboard scan bound, and the plan split.

## Deferred Ideas

- Phase 31: delete the injection and legacy code.
- Backlog 999.7: prompt 1.1.
- Launcher follow-up: mask sibling run dirs in the sandbox.
- Future: steering through a macOS splice.
- Caps on a live run change only through relaunch.
