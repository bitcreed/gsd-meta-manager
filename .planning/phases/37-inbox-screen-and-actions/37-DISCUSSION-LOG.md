# Phase 37: Inbox screen and actions - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-11-14
**Phase:** 37-inbox-screen-and-actions
**Areas discussed:** Inbox layout and ordering, Run command, Acknowledge writer, Clipboard, Decide-lane noise, Owner block keys

Run unattended (`--auto`); every choice below was made by the orchestrator from the report and
prior-phase artifacts. No AskUserQuestion was used.

---

## Inbox layout and ordering

| Option | Description | Selected |
|--------|-------------|----------|
| Default = Test+Decide+Fix, `a` = all incl. Blocked | Matches the §3.3 mockup (Blocked only in header) | ✓ |
| Default = all four lanes | Blocked rows the owner cannot act on dilute the list | |

| Option | Description | Selected |
|--------|-------------|----------|
| Progress = dashboard Progress ratio, age oldest first | Same number the owner sees on the dashboard | ✓ |
| Progress = fewest open phases; age newest first | | |

**Choice:** [auto] first options; ordering is a human UAT item.

## Run command

| Option | Description | Selected |
|--------|-------------|----------|
| Typed {verb, token, ws}, re-validated at launch | SC1: argv only from validated token and ws name | ✓ |
| Reuse the reader's `command` string | Simpler, but the string is not the validated source | |

Live session: focus it and show the command in the status line (report §3.4) ✓; injecting into
the session deferred.

## Acknowledge writer

| Option | Description | Selected |
|--------|-------------|----------|
| `gsd-tools audit-open acknowledge …` as a bounded direct child | Exit status and refusals reach the owner | ✓ |
| Same command in a multiplexer window (report §3.4 wording) | Exit status lost in another window | |
| Native frontmatter write | Violates D7 / non-intrusive | |

Spelling verified in `audit-command-router.cjs` / `audit.cjs` and on a throwaway project; `--phase`
needs the exact directory token; `--ws` works.

## Clipboard

| Option | Description | Selected |
|--------|-------------|----------|
| Native tool (wl-copy/xclip/xsel/pbcopy/clip.exe), else OSC 52 | Works locally and over ssh | ✓ |
| OSC 52 only | Fails silently where the terminal blocks it | |
| New crate (arboard) | New dependency for one key | |

## Decide-lane noise

Live fleet: mailbot 19 and sentriq 11 of 17 are CONTEXT open-question **lines** in phases that
are already planned/executed. Verdict: noise.

| Option | Description | Selected |
|--------|-------------|----------|
| Gate on "phase has no PLAN/SUMMARY yet" + count questions not lines | Keeps the one moment an answer matters | ✓ |
| Gate on ROADMAP phase status (not complete) | mailbot 04 / sentriq 16 are in progress and would stay | |
| Drop context questions from the inbox | Loses real pre-planning questions | |

## Owner block keys

| Option | Description | Selected |
|--------|-------------|----------|
| Bind `v`/`o`/`A`/`y` with no layout change | 36 deferred it here; 36 UAT layout frozen | ✓ |
| Leave the Owner block without actions | | |

## Claude's Discretion

- Column widths/breakpoints (37 UI-SPEC), `o` deep link vs pushed viewer, module split, plan count.

## Deferred Ideas

- Injecting the command into a live session; aggregated debug rows; upstream issue on
  `deriveOpenQuestions` line counting.
