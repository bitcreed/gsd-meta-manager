# Phase 14: UI Fixes - Context

**Gathered:** 2026-07-28
**Status:** Ready for planning
**Mode:** Autonomous — grey areas proposed and auto-accepted by the orchestrator per user
direction ("best well-reasoned guess is good enough; don't ask questions"). Every decision
below is Claude's Discretion and is listed explicitly so it can be corrected at the end.

<domain>
## Phase Boundary

Four long-standing display defects stop misreporting project state.

In scope: UIFIX-01 (HANDOFF pause badge), UIFIX-02 (DRPEV leading blank),
UIFIX-03 (markdown edit mode activation), UIFIX-04 (PageDown scroll clamp).

Out of scope: anything touching the v2.0 driver subsystem. This phase has zero
dependencies on Phases 15-22 and must not anticipate them.

</domain>

<decisions>
## Implementation Decisions

### UIFIX-01 — verify before implementing

**Decision: treat this as verify-first, not build-first.**

The todo was filed 2026-03-27. Phase 11 "Paused Project Detection" shipped in v1.2
(2026-04-01) and PROJECT.md records `✓ Paused project detection — HANDOFF file detection
with cyan pause badge on dashboard — v1.2` as a *validated* requirement. Corroborating
evidence: the UIFIX-02 todo, filed 2026-04-05, pastes real dashboard output showing
`⏸ cdr-configurator` — a pause badge already rendering.

So the likely truth is that UIFIX-01 is already satisfied and the todo is stale.

Required behavior:
1. Establish empirically whether a non-empty `HANDOFF.md`/`HANDOFF.json` already produces
   a badge on the dashboard row. Read the code and write a test that asserts it.
2. If it already works — do NOT re-implement. Land the regression test, mark UIFIX-01
   satisfied, and record in the phase SUMMARY that the todo was stale.
3. If it only partially works (e.g. `HANDOFF.json` detected but `HANDOFF.md` not, or the
   badge is missing when a session indicator is also present) — fix only the gap.

Note the todo proposes a `||` badge in dim yellow; the shipped implementation uses `⏸` in
cyan. The shipped choice wins — do not change the glyph or color. The todo predates it.

### UIFIX-02 — fix at the source, not the render site

Remove the leading space where the D-R-P-E-V string is *constructed*, not by trimming at
the call site. Success criterion is "no leading blank at any terminal width", so if the
compact and expanded renderings build the string differently, both must be covered. Add a
test asserting the string does not start with whitespace.

### UIFIX-03 — determine whether the feature exists before debugging it

The todo lists three possibilities (unregistered key, broken guard, never implemented).
Resolve which one it is first. Constraint: quick task 260401-t7y already added
`tui-textarea` + `$EDITOR` shell-out for archive/backlog markdown, so the editing
machinery exists — this is very likely a key-routing or screen-guard problem, not a
missing feature. If it turns out the feature genuinely does not exist on this screen,
wire it to the existing `tui-textarea` path rather than inventing a second editor flow.

### UIFIX-04 — clamp using the existing max_scroll calculation

Reuse the `total_lines - visible_height` clamp the rendering code already computes rather
than introducing a parallel calculation that can drift. Clamp on the PageDown handler so
the stored offset is never out of range — do not merely clamp at render time, since the
bug is precisely that the *stored* offset grows unbounded. Apply the same reasoning to any
other unclamped scroll path found in the same handler (e.g. `End`, `Down`), but do not
expand scope beyond the scroll offset.

### Testing

Each fix lands with a regression test. These are display defects that recur easily, and
three of the four have no test coverage today. Tests go in the module they cover,
following the existing `#[cfg(test)] mod tests` convention.

### Claude's Discretion

- Exact test names and placement
- Whether the four fixes land as one plan or several (planner's call; they are independent
  and touch different modules)
- Whether to fix adjacent unclamped scroll paths discovered while fixing UIFIX-04

</decisions>

<code_context>
## Existing Code Insights

- `src/ui/project_list.rs` (318 lines) and `src/ui/screens/` render the dashboard rows
- `src/state_reader/disk_status.rs` (973 lines) derives D-R-P-E-V stage status
- `src/browser.rs` and the Docs tab own markdown viewing; `tui-textarea` is already a
  dependency from quick task 260401-t7y
- PageUp/PageDown on the detail screen came from quick task 260403-p84 — that is the code
  UIFIX-04 regresses against
- Project convention: zero clippy warnings under `cargo clippy -- -D warnings` (lib target).
  Note `--all-targets` currently fails on 5 pre-existing lints in `browser.rs`,
  `project_creator.rs`, and `state_reader/mod.rs` — those are NOT this phase's to fix, and
  the phase must not make them worse.

</code_context>

<specifics>
## Specific Ideas

The four todo files in `.planning/todos/pending/` carry full problem statements and
proposed solutions, including a pasted real-terminal reproduction for UIFIX-02. They are
tagged `resolves_phase: 14` and should be read as the primary spec. Mark them completed
when the phase closes.

</specifics>

<deferred>
## Deferred Ideas

- Fixing the 5 pre-existing `--all-targets` clippy lints — unrelated to these four defects
- Any broader scroll/keybinding refactor across screens

</deferred>
