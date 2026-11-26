# Phase 39: Check actions through GSD - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-11-17
**Phase:** 39-check-actions-through-gsd
**Mode:** `--auto` (unattended); locked owner decisions supplied by the coordinator
**Areas discussed:** Outcomes and targets, Key and flow, Command and instruction, Live session, Read-only proof, Todo fold

---

## Outcomes and targets

| Option | Description | Selected |
|--------|-------------|----------|
| Three outcomes: won't do / deferred permanently / move to phase M | Owner's locked decision | ✓ |
| Two outcomes as in the roadmap (won't do, defer/move) | Merges permanent defer with move | |

**User's choice:** [user] three outcomes. **[auto]** Targets: item lines and group rows in TEST and BLOCKED lanes; FIX/DECIDE/archived/acknowledged refused (recommended; FIX is code work, DECIDE rows are not checks, archived phases are not addressable).
**[auto]** Reason: required for won't do, optional otherwise (SC1 wording).
**[auto]** Item selection: add item-line cursor stops under expanded groups (scout: stops are group rows only today).

## Key and flow

| Option | Description | Selected |
|--------|-------------|----------|
| One key `x` + chooser (`w`/`p`/`m`) | One key in both hosts, no collision with Owner block or detail keys | ✓ |
| Three letters (`W`, `D`, `M`) | Fewer keystrokes; `D` is the detail tab key, `M`/`m` used in detail | |

**[auto]** Selected: `x` chooser (recommended). Move target: picker of open phases, default next phase, free entry validated by `is_command_token`.

## Command and instruction

| Option | Description | Selected |
|--------|-------------|----------|
| `/gsd-verify-work N [--ws]` + appended instruction, one argv element | Owner's locked decision; verify-work already handles skip / deferred follow-up | ✓ |
| Separate `gsd-tools todo` call for move | App would run a second writer | |

**User's choice:** [user] GSD command with a plain-language instruction; for move, the same instruction asks GSD to add the todo for phase M. No upstream requests.
**[auto]** Instruction templates per outcome × check/phase (wording is a human UAT item).

## Live session

| Option | Description | Selected |
|--------|-------------|----------|
| Focus it + copy command line to clipboard | Matches 37 D-12; no second session racing on STATE.md | ✓ |
| Launch a second session anyway | Risk of concurrent GSD writes | |
| Refuse | Blocks the owner | |

**[auto]** Selected: focus + clipboard (recommended).

## Read-only proof

| Option | Description | Selected |
|--------|-------------|----------|
| Source scan + fake-launcher tree snapshot test; real round trip as human UAT | The instruction needs a real Claude session to interpret | ✓ |
| Automated real round trip in CI | Requires Claude in CI; not available | |

**[auto]** Selected (recommended).

## Todo fold

17 keyword matches from `todo.match-phase 39` (score 0.3-0.9), none related to check outcomes. Auto-fold threshold (>= 0.4) overridden by the scope guardrail: none folded.

## Claude's Discretion

- Popup layout and widths, module split, reason input reuse, chooser/reason in one or two popups.

## Deferred Ideas

- Outcomes on FIX and DECIDE rows; CLI outcome actions; batch settling.
