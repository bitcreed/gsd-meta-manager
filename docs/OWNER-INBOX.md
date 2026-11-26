# Owner inbox

The owner inbox shows what is waiting on you, the human, across every registered
project: checks only you can run, rulings you have to make, and code that needs an
agent. It is read from each project's `.planning/` files (UAT and VERIFICATION files,
CONTEXT questions, debug sessions, handoffs, `deferred-items.md`, pending todos).

Reading is free: the dashboard, the project detail and the inbox screen only read
files. Four actions go further, each on your key press: `v` launches the group's
command in the project, `A` runs gsd-core's acknowledge writer after a confirmation,
`x` settles a check or a phase through `/gsd-verify-work N` after a confirmation, and
`y` copies a checklist. The app itself writes nothing into any project; every change
comes from a command GSD runs.

Press `?` in the TUI for the keys of each screen; the footer lists the keys that work
on the selected row, and `Enter` on a group opens an action menu with each action, its
key and what it runs. Scripts and agents get the same data from
[`gsd-meta-manager inbox`](SCRIPTING.md#owner-work-inbox).

## Lanes

Owner work falls into lanes, always shown in this order:

- **test**: checks to run. A phase whose only open work is checks *closes* once you
  run them.
- **decide**: rulings that wait on you: debug sessions awaiting your confirmation,
  handoffs, checkpoints, open CONTEXT questions.
- **fix**: phases that need code (failed checks, unreadable files) and open debug
  investigations.
- **blocked**: checks waiting on a device, server, build or third party. Not yours to
  act on, so it is shown only on request.

A UAT and a VERIFICATION file of one phase are one entry. Archived milestones,
acknowledged files, pending todos and work older than `preferences.owner.fold_days`
(default 90) are folded away but still counted.

## Where it shows

**Dashboard (100 columns and wider).** A `You` column between Status and Progress shows
at most two lanes for the current milestone:

| Cell | Meaning |
|------|---------|
| `test 10 → closes 4` | 10 checks to run; testing alone closes 4 phases (only shown when `test` is the only lane) |
| `decide 1 · fix 2 ph` | 1 ruling waits on you; 2 phases need code |
| `fix 2` | open debug investigations count as fixes but not as phases |
| `·` | nothing waits on you (blocked work is not yours to act on) |
| `(+3 old)` | 3 more test, decide or fix items in archived milestones |
| (blank) | no owner data: it has not loaded yet, or the folder or its `.planning/` is gone |

A summary line under the table totals the visible rows. Attention-first sort (`s`)
ranks projects with owner work right after those waiting on a human, and the filter
`//y` (or `//y test`, `//y decide`, `//y fix`) lists projects with owner work.

**Project detail, Phases tab.** An Owner block above the phase list names what closes
the milestone (`closes after: test 12 (ph 21 22 24) · unblock 5 (ph 20 23) · …`) and
lists one row per group (`TEST`, `DECIDE`, `FIX`, `BLOCKED`) with its phase, checks,
age and the command that handles it. A group expands to its checks and source file,
so one check can be settled on its own.

**Inbox screen (`i` on the dashboard).** One list across every project, grouped into
`TEST — closes a phase`, `TEST — phase stays open`, `DECIDE` and `FIX — code needed`,
with Blocked, Old debt, Acknowledged, Triage and Stale folded below. Within Test,
phases that close first come first, then projects closest to closing a milestone, then
the oldest; Decide and Fix list the oldest first. The header totals match
`gsd-meta-manager inbox --json`.

## Actions

**`v` run.** Builds the command from the group's kind, phase and workstream
(`/gsd-verify-work N`, `/gsd-plan-phase N --gaps`, `/gsd-debug`, `/gsd-resume-work`,
`--ws NAME` for workstream groups) and runs it in the project through your multiplexer,
or switches to the project's live session and shows the command to type. A phase token
or workstream name that is not plain is refused.

**`A` or `x`?** `A` hides a file for now: nothing is recorded, its phase stays open,
and its items come back when the file changes. `x` records checks you cannot or will
not test as skipped (won't do, deferred permanently, or moved to phase M), so the phase
can close.

**`A` acknowledge.** After a confirmation, runs in the project directory:

```
node <gsd-tools.cjs> audit-open acknowledge --category <uat_gaps|verification_gaps|context_questions|debug_sessions> --milestone <version> <--phase DIR --file NAME | --slug STEM> [--archived-milestone V] [--ws NAME]
```

`<gsd-tools.cjs>` comes from your own install only (`${CLAUDE_CONFIG_DIR:-~/.claude}/gsd-core`,
then `${CODEX_HOME:-~/.codex}/gsd-core`, then an absolute `PATH` entry named
`gsd-tools`); a `gsd-tools.cjs` the project ships is never run. gsd-core writes
`audit_acknowledged` into that file's frontmatter and does not commit. The marker is
file-level (it hides every item of the file in every lane) and lapses by itself when
the file changes.

**`x` settle.** On a TEST or BLOCKED group it settles the whole phase; on one expanded
check, that check. You pick an outcome (won't do, deferred permanently, or move to
phase M) and an optional reason. A default-No popup sums it up: project, target,
outcome, your reason, and the command it runs (`d` shows every argv element verbatim).
Confirming launches `/gsd-verify-work N <instruction>` in the project (`--ws NAME` for a
workstream) in a **new** session through your multiplexer; it never types into a live
session or touches the clipboard. The instruction asks GSD to resume the UAT session,
record each check as `result: skipped` with a `Deferred follow-up:` reason (which
gsd-core counts as non-blocking), and not to promote it to the 999.x backlog. Your
reason is quoted verbatim as `Owner’s note: “““…”””`, with double quotes turned into
typographic ones so they cannot end the note early. Move also asks for one todo naming
phase M, so the check comes up again when phase M runs.

After confirming, the row shows a `settling…` badge with the time since, `recorded`
once the source file changes on disk, and `✓ test 2 skipped` (or `✓ moved to phase M`)
for about two seconds before the row goes. The badge lives in memory only: it survives
refreshes and is gone after a restart. If the settle never finishes (the session was
closed, or GSD recorded nothing), `x` on the row offers to stop waiting, which clears
the badge in this app only.

**`y` copy.** Copies the checklist `gsd-meta-manager inbox ALIAS` prints for that row,
through `wl-copy`, `xclip`/`xsel`, `pbcopy` or `clip.exe` when present, otherwise an
OSC 52 escape sequence your terminal must allow (inside tmux: `set -g set-clipboard on`).
