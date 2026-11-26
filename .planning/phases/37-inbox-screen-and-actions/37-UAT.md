---
status: testing
phase: 37-inbox-screen-and-actions
source: [37-VERIFICATION.md, 37-UI-REVIEW.md]
started: 2026-11-15T00:00:00Z
updated: 2026-11-15T00:00:00Z
---

## Current Test

number: 1
name: Ordering and legibility on the real fleet
expected: |
  Open the inbox (`i`) at 120 columns, then at 100, in your real terminal font and theme. Sections run TEST (closes a phase), TEST (phase stays open), DECIDE, FIX. Inside TEST the projects closest to a milestone close come first, then the oldest. DECIDE and FIX list the oldest first. No column is clipped, the phase number is visible on every row, `stale` is readable against your theme, and the `·` and `—` separators leave the checks column aligned. The order feels right for triage.
awaiting: user response

## Tests

### 1. Ordering and legibility on the real fleet
expected: Open the inbox (`i`) at 120 columns, then at 100, in your real terminal font and theme. Sections run TEST (closes a phase), TEST (phase stays open), DECIDE, FIX. Inside TEST the projects closest to a milestone close come first, then the oldest. DECIDE and FIX list the oldest first. No column is clipped, the phase number is visible on every row, `stale` is readable against your theme, and the `·` and `—` separators leave the checks column aligned. The order feels right for triage.
result: [pending]

### 2. `v` into a real multiplexer
expected: From the inbox press `j`/`v` on a TEST row with no live session, under tmux and again under herdr; then repeat with a live session at that project. A new window or pane opens in the project directory and runs `claude /gsd-verify-work N`, and the session behaves normally although `env -u` removed `GSD_WORKSTREAM`, `TMUX_PANE` and the other session variables from its environment. With a live session at exactly that path, that session is focused and the status line reads `Switched to ... · type /gsd-verify-work N`; nothing is typed into it.
result: [pending]

### 3. `A` on a throwaway copy of a project (never a real one)
expected: Press `A` on a UAT row, read the popup, press Esc/n once, then `A` and Yes. The popup is Yellow, defaults to No, and names the project, file, category, milestone, 'file-level', 'lapses when the file changes', 'commits nothing' and the exact argv (wrapped at `/` and spaces, not mid-token). Esc/n starts nothing. Yes writes `audit_acknowledged` into that one file's frontmatter via gsd-core, leaves it uncommitted (`git status` shows the modification), and the row leaves the inbox on the next watcher refresh without a restart. The status line `Acknowledged ... via gsd-core (uncommitted)` stays about 12 s.
result: [pending]

### 4. `y` clipboard on the real desktop and over OSC 52
expected: Press `y` on a group row, then paste. Repeat with no native tool on PATH (or over ssh/tmux with `set -g set-clipboard on`). The pasted text equals what `gsd-meta-manager inbox` prints for that group. The status line says `Copied N lines via {tool}` or `Sent N lines to the terminal clipboard (OSC 52; the terminal must allow it)`. The terminal shows no stray escape output.
result: [pending]

### 5. Held-key and paste safety for `v`
expected: Hold `v` on a row, or paste text containing `v` into the terminal, with the inbox open. Decide whether launching one session per key repeat is acceptable: only the Ctrl/Alt guard (WR-01) was applied; bracketed-paste handling and a per-group cooldown were not, and `v` has no confirm by design (SC1).
result: [pending]

### 6. Decide and Fix visibility on the first screen
expected: With the default lanes the first screen can be all `TEST — closes a phase` rows, so DECIDE and FIX sit below the fold. Decide whether the locked ordering (D-02/D-05) is right, or whether section titles should carry counts / a lane strip or jump key should be added (UI review fix 5).
result: [pending]

### 7. Wide-screen column use at 140 columns
expected: At 140 columns, names are still cut at the 32-cell cap while the reserved phrase column leaves a wide empty gutter between `checks` and `age` (the phrase column is sized over every lane, not the visible one; UI review fix 1, part not done). Decide whether this reads acceptably or should be sized from the active lane.
result: [pending]

### 8. Accept the narrowing-order deviation from the UI-SPEC
expected: The inbox now narrows phrase, then alias, then name, then the phase cell (the UI-SPEC shrank the phase cell before alias and name, which lost the phase number on a real fleet; recorded in 37-UI-REVIEW.md "Fixes applied"). Confirm the long-alias truncation this causes is acceptable.
result: [pending]

### 9. Theme legibility of the cursor, DIM rows and lane state
expected: In your theme the Cyan+BOLD cursor row is clearly distinct; DIM-only BLOCKED and Acknowledged rows stay readable; the active lane filter (shown by REVERSED only) is recognizable after pressing `1`-`4` or `a`.
result: [pending]

### 10. Owner block key discoverability (after Phase 36 UAT)
expected: The detail Owner block now acts on `v o A y` (D-21), but its footer is frozen for the Phase 36 UAT and only the help overlay lists the keys. Decide whether to add `[v]run  [A]ck` to the Owner block footer once Phase 36 UAT lands.
result: [pending]

## Summary

total: 10
passed: 0
issues: 0
pending: 10
skipped: 0
blocked: 0

## Gaps
