# Phase 39: Check actions through GSD - Context

**Gathered:** 2026-11-17
**Status:** Ready for planning
**Mode:** `--auto` (unattended). Decisions tagged **[user]** are the owner's locked decisions;
**[inferred]** ones were taken from the planning artifacts and the code scout and may be revised.

<domain>
## Phase Boundary

From the TUI inbox (`i`) and the detail view's Owner block, the owner can settle a check (one item)
or a whole phase entry (a group row) with one of three outcomes: **won't do**, **deferred
permanently** (don't ask again), or **moved to phase M** (comes up again in phase M, the source
phase may close). Each outcome launches a GSD command in that project with a prepared
plain-language instruction stating his intent, after a confirm popup that shows the exact command
and instruction. The app writes nothing into the project; GSD records the decision (and, for a
move, the todo/backlog item for phase M). The existing `A` acknowledge is unchanged.

Not in this phase: per-item pass/fail entry (OWN-F06), a native writer of any kind, todo triage
UI (OWN-F03), CLI (`inbox`) actions.

</domain>

<decisions>
## Implementation Decisions

### Carried forward (not re-decided)
- PROJECT.md "GSD files are read-only" (2026-11-16) and report D7: the app never writes
  `.planning/**` or `$GSD_HOME`; every change goes through a launched GSD command or `gsd-tools`.
- 37 D-11/D-12: commands are built from typed values (verb, validated phase token, validated
  workstream name), never from reader strings; launch through `multiplexer::launch` with
  `ExecArgv::new(["env", "claude", "<command>"])`, the command as ONE argv element, project path as
  cwd; a live session for the project is focused instead of starting a second one.
- 37 D-14..D-16: `A` (gsd-tools `audit-open acknowledge`, bounded child, its popup) stays exactly
  as is **[user]**.
- 38 D-08: a merged same-phase entry (UAT + VERIFICATION) is one row; phase-level actions act on
  all its files. 38 D-11: heuristic labels (`decision?`, `looks resolved`) never move, hide or
  re-rank anything **[user]**.
- 37 D-10: rows refresh from the watcher; nothing is hidden optimistically.
- WR-01: side-effecting letters act only unchorded (`owner_actions::is_plain_action_key`).

### The three outcomes (OWN-29)
- **D-01 [user]:** Outcomes are exactly: **Won't do** (reason required), **Deferred permanently**
  (don't ask again), **Move to phase M** (re-asked in M; the source phase can close without it).
  The roadmap's "defer/move" is split into these two named outcomes; this is a clarification
  inside OWN-29, not a scope change.
- **D-02 [inferred]:** Reason text: required for Won't do (SC1 "with a reason"), optional for
  Deferred permanently and Move. Single line, control characters stripped, capped at 200 chars.
- **D-03 [inferred]:** Targets. A **check** = one item under an expanded group; a **phase** = a
  group row (merged entry included), meaning all its open checks. Outcomes apply to TEST and
  BLOCKED lane rows (things the owner performs or waits on). FIX, DECIDE (context questions, debug
  sessions, handoff, checkpoint), Old debt (archived) and Acknowledged fold rows are refused with
  a status message, like `A` refusals today. Reason: FIX is code work (`/gsd-plan-phase --gaps`),
  DECIDE rows are not checks, and an archived phase is not addressable by `/gsd-verify-work N`.
- **D-04 [inferred]:** Item-level cursor. The cursor stops today are group rows only. When a group
  is expanded, `j/k` also stop on its item lines (in both hosts: inbox and Owner block), so a
  single check can be selected. On an item line `v`, `o`, `y` and `A` keep acting on the parent
  group (unchanged behavior); only the new outcome key distinguishes item vs group. Layout does
  not change (highlight only), so the frozen 36 Owner-block goldens move only where a test
  selects an item.
- **D-05 [inferred]:** Key. One key, `x` ("settle"), in both hosts (free in the inbox; the Owner
  block consumes its own keys before the phase list's `x`). It opens a small chooser popup:
  `w` Won't do, `p` Deferred permanently, `m` Move to phase, `Esc` cancel. Then the reason field
  (and the phase picker for Move), then the confirm popup. Typical flow: `x w <reason> Enter y` or
  `x m <pick> Enter y`. Footer hint and the help overlay rows gain `x`; README "Owner inbox"
  section documents it.
- **D-06 [inferred]:** Move target picker: the project's phases from `ProjectState` (roadmap,
  current milestone and backlog `999.x`) that are not complete and not the source phase, default
  = the next phase after the source. Free entry is allowed when it passes `is_command_token`
  (same predicate as 37 D-11); an unknown phase is accepted with a warning in the confirm popup
  (GSD may create or reject it), never silently.

### Command and instruction (OWN-30)
- **D-07 [user]:** Every outcome launches `/gsd-verify-work <phase token>` (`--ws <name>` for
  workstream groups) with the instruction appended as plain text in the same argv element:
  `claude "/gsd-verify-work 24 --ws foo <instruction>"`. Verify-work already handles "skip" and
  "deferred follow-up" answers; we state intent and let GSD record it. No design around missing
  gsd-core features, no upstream requests.
- **D-08 [inferred]:** Instruction templates (one pure, unit-tested builder; exact wording is a
  human UAT item):
  - Won't do (check): `The user will not perform test K ("<name>") in <file>: <reason>. Record
    this decision so it is no longer open and is not asked again.`
  - Deferred permanently (check): `The user defers test K ("<name>") in <file> permanently[: <reason>].
    Record it as deferred so it is not asked again.`
  - Move (check): `Defer test K ("<name>") in <file> as a follow-up for phase M[: <reason>]. Record
    the deferred follow-up here so phase N can close without it, and add a todo/backlog item for
    phase M so the check comes up again when phase M runs.`
  - Phase-level variants name all open checks of the entry (`tests 1, 3, 4 in 24-UAT.md and item 2
    in 24-VERIFICATION.md`) instead of one.
  Check identity = file basename + test number + name (name quoted, single line, capped ~80 chars,
  control characters stripped). VERIFICATION items without a number use their list position.
- **D-09 [user]:** For Move, the single launched command does both: records the deferral and adds
  the todo/backlog item for phase M. The app does not run `gsd-tools todo` or `/gsd-capture`
  itself. This narrows OWN-31's "`gsd-tools todo …` or `/gsd-capture`": filing still goes through
  gsd-core, by instruction.
- **D-10 [user]:** Confirm popup (reuse `confirm_popup.rs`/`ack_confirm.rs` patterns, default No)
  shows: project, target (check or phase with counts), outcome, the exact argv command line and
  the full instruction, wrapped. Cancel launches nothing. Instruction text renders through the
  existing `shown`/`fit_cells` helpers (`render_escape_guard` coverage, OWN-11).
- **D-11 [inferred]:** Live session present (37 D-12 path): focus it and copy the full command
  line to the clipboard through the existing `y` copy path (native tool or OSC 52); the status
  line says to paste it there. Nothing is injected into the session, no second session is
  started (two GSD sessions in one project could race on STATE.md).
- **D-12 [inferred]:** After launch the row stays until GSD's writes reach the watcher (37 D-10);
  status line: `Launched /gsd-verify-work N in <alias>: <outcome> for <target>`.

### Read-only proof (OWN-31, SC5)
- **D-13 [inferred]:** Automated: (a) a source scan over the new modules and the chooser/confirm
  screens proving no filesystem write API is reachable for project paths (pattern of the OWN-02
  scan); (b) a flow test with a fake launcher that snapshots a fixture project tree (paths +
  hashes) before and after the full `x … y` flow and asserts it is byte-identical, plus argv and
  instruction table tests (each outcome × check/phase × workstream, refusals for FIX/DECIDE/
  archived/acknowledged, bad tokens, bad workstream names, over-long and control-character
  reasons). The "oracle round trip on a throwaway project" from the roadmap's Verification line
  needs a real Claude session (the instruction is interpreted by the model), so it is **human
  UAT**: won't-do, deferred-permanently and move round trips in the real multiplexer on a
  throwaway project, checking that GSD recorded each, that the moved check appears as a todo for
  phase M, and that the instruction wording reads right.

### Claude's Discretion
- Exact popup layout, widths and colors (UI-SPEC territory, reuse 36/37 tokens).
- Module split (e.g. a new `owner_settle.rs` next to `owner_actions.rs`) and how the reason text
  field is implemented (reuse an existing single-line input such as `SlugInputScreen`).
- Whether the chooser and reason field are one popup or two.

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Scope and requirements
- `.planning/ROADMAP.md` §"Phase 39: Check actions through GSD" — goal, SC1-SC5, human UAT
- `.planning/REQUIREMENTS.md` §"Check actions through GSD (OWN-29..31)", Future (OWN-F03, F05,
  F06), Out of Scope ("Any native write into a project")
- `.planning/PROJECT.md` §Constraints "GSD files are read-only"

### Design source
- `.planning/research/OWNER-INBOX-UX.md` §3.3 inbox, §3.4 actions, §4 Layer B (R1 lanes, R10
  commands), §7 decisions D7/D10

### Prior phase decisions
- `.planning/phases/37-inbox-screen-and-actions/37-CONTEXT.md` — D-01..D-16 (inbox screen, run
  command argv, launch/focus path, acknowledge popup)
- `.planning/phases/37-inbox-screen-and-actions/37-UI-SPEC.md` — tokens and popup patterns
- `.planning/phases/37-inbox-screen-and-actions/37-VERIFICATION.md`, `37-SECURITY.md` — threat
  model for argv building and untrusted project strings
- `.planning/phases/38-inbox-digest-and-triage-hygiene/38-CONTEXT.md` — D-08 merged entries,
  D-11 label-only heuristics
- `.planning/phases/38-inbox-digest-and-triage-hygiene/38-VERIFICATION.md`, `38-UAT.md` — open
  human items (80 vs 100 column target) that also bound the new footer hint

### GSD behavior relied on
- `$CLAUDE_CONFIG_DIR/gsd-core/workflows/verify-work.md` — handling of "skip" and "deferred
  follow-up" answers that the instruction maps onto

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/ui/screens/owner_actions.rs`: `run_plan`/`run_group`/`launch_real` (launch or focus),
  `is_plain_action_key`, `copy`/`copy_text` (clipboard path for D-11), `AckTarget`/`ack_args`
  (pattern for a typed target and refusals).
- `src/ui/screens/ack_confirm.rs`: `AckConfirmScreen`, `AckHost` (inbox vs Owner block),
  `ack_body` with `wrap_at_separators` — template for the outcome confirm popup.
- `src/ui/screens/confirm_popup.rs`: default-No confirm.
- `src/ui/screens/driver_launch.rs` `SlugInputScreen`: single-line text input to reuse for the
  reason field.
- `src/state_reader/owner_items/rules.rs` predicates `is_command_token` (~74), `is_workstream_name` (~102).

### Established Patterns
- Inbox key dispatch: `src/ui/screens/inbox.rs` ~1903-1960 (`handle_key`), selection via
  `stop_rows`/`cursor_pos`/`selected`; `InboxRow` ~179, `Stop`/`Fold` ~140-160.
- Owner block key dispatch: `src/ui/screens/detail.rs` `handle_owner_block_key` ~2192-2295;
  cursor via `owner_block::move_cursor`, `skeleton(...).stops`.
- Every screen that renders project strings is registered with `adjudicate_screen!` and covered
  by `render_escape_guard.rs`.

### Integration Points
- New `x` arm in both key handlers; new stop kind for item lines in `inbox.rs` and `owner_block.rs`.
- Help overlay rows in `src/ui/screens/help.rs` (~364-376 Owner block, "Owner Inbox" section).
- README "Owner inbox" section.

</code_context>

<specifics>
## Specific Ideas

- Example instructions from the owner: "the user will not perform test K: <reason>"; "defer test K
  as a follow-up for phase M".
- Move semantics in the owner's words: "moved to another phase (the check comes up again there,
  while the original phase can close)".

</specifics>

<deferred>
## Deferred Ideas

- Outcome actions on FIX rows (accept a gap as won't-fix) and on DECIDE rows (answer a context
  question) — new capabilities, not checks; possible future phase.
- Outcome actions from the `inbox` CLI (non-interactive) — only the TUI is in scope.
- Batch settling of several groups at once.

### Reviewed Todos (not folded)
`todo.match-phase 39` returned 17 keyword matches (score 0.3-0.9: release-map badge, help popup
at 80x24, launcher-marker scan flag, unregister guard, glyph overflow, driverctl hardening,
executor remnants, sandbox tmpfs, capture-pump redaction, lock hard link, run-file hardening,
unsupervised child, ambiguous-wide borders, D-17 recovery block, driverctl LOG redaction,
managed-policy validation, multi-line queue input). None concerns check outcomes; all matched on
generic words (`tui`, `gsd`, `check`, `popup`). Auto-fold (>= 0.4) was overridden by the scope
guardrail **[inferred]**: none folded.

### Musk pass
No cuts: OWN-29..31 and SC1-SC5 are all owned by the user (2026-11-16 decision and this phase's
locked decisions).

</deferred>

---

*Phase: 39-check-actions-through-gsd*
*Context gathered: 2026-11-17*
