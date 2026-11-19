# Quick Task 261119-f5b: Owner-row action menu and context-aware footer - Context

**Gathered:** 2026-11-19
**Status:** Ready for planning
**Mode:** unattended (owner brief relayed by the coordinator; no interactive questions). Decisions not stated by the owner are marked INFERRED.

<domain>
## Task Boundary

On the project detail Phases tab, with the cursor on an Owner block element, the footer still shows the generic
`[j/k]move [Enter]expand [←/→]folds [Esc]phases [q]uit [?]help`. Owner (verbatim intent): "When on an Owner element,
the key options should reflect the quick actions. Pressing enter on the item that's not the markdown file should open
a context menu with options (actions to pick from — probably the same as the quick key reference in the bottom that's
currently missing, but maybe also an extended list if the quick actions don't list all possibilities). Investigate for
best possible UX, devise 3 solutions and pick the one with the best UX."

Scope: the Owner block (src/ui/screens/detail.rs) AND the global inbox screen (src/ui/screens/inbox.rs), which share
the row model and keys (v run row command via multiplexer / focus live session, o open source read-only, A acknowledge
via gsd-core, x settle via src/ui/screens/settle_popup.rs, y copy checklist, g go to project (inbox only), Enter
expand/collapse a group; quick-261118-u4e made Enter on a check line or `file PATH` line open the read-only viewer at
that spot; quick-261118-v6l added a pending "settling…" badge). Row kinds: TEST/DECIDE/FIX/BLOCKED groups, debug
investigations (`/gsd-debug`), the expanded description line, check lines, file lines, folds (Old debt, Stale,
Acknowledged, Triage, Todos). Not every action applies to every kind — the exact per-kind applicability must be
established from the code, not assumed.

</domain>

<decisions>
## Implementation Decisions

### Solution selection (locked process, open outcome)
- Research must document the current keys per row kind in both screens and write THREE distinct solutions with a short
  UX trade-off each. Starting points (refine or replace):
  - A. Context-aware footer only: footer lists the selected row's applicable actions with the concrete command
    (e.g. `[v]run /gsd-verify-work 16 [x]settle [o]open [y]copy`); Enter unchanged.
  - B. Context-aware footer + Enter opens an action menu (popup anchored near the row, which-key style) listing every
    applicable action with its key letter and a one-line description, primary action (run its command) preselected;
    j/k + Enter or the letter executes; Esc closes. Expand/collapse stays directly reachable (e.g. Space / Right, and as
    a menu entry). Enter on a file/check line still opens the viewer directly.
  - C. Inline action strip under the selected row (chips, ←/→ to pick, Enter to run) — weigh conflicts with ←/→ folds
    and vertical space.
- The plan picks the best UX on the evidence and justifies it. Owner expectation is B, but the decision must be argued.

### Shared implementation, consistent (not identical) screens
- One shared implementation (action model + footer builder + menu widget/state) used by both the Owner block and the
  inbox. No duplicated per-screen action logic.
- Same interaction model in both screens (footer + Enter menu), same key and same label/wording for the same action on
  the same item kind.
- Screen-specific differences are allowed where context justifies them (e.g. `g` go-to-project only in the inbox; extra
  entries or ordering that make sense in one context) but every difference must be an explicit, documented exception in
  code (e.g. a named per-screen extras list with a doc comment), not an accident.
- Required test: for the same item, every action both screens offer uses the same key and the same label; each
  screen-specific difference is listed as an explicit exception and the test asserts the difference set equals that
  documented exception set.

### Footer truthfulness
- Footer always truthful for the selected row: no listed key that does nothing on that row, no applicable key missing.
- Degrade gracefully at 80 and 100 columns by dropping lower-priority hints; `[?]help` is always kept.

### Menu (if chosen)
- Lists only actions that work on that row; unavailable ones either hidden or shown dim WITH the reason (not color-only).
- Executing from the menu goes through the exact same code path as the hotkey (x still shows the settle confirm popup;
  A still asks first).
- Fits 80x24. Mouse click on a menu entry if the screen already supports mouse elsewhere.
- Respect the glyph rules under src/ui/.

### Discoverability and docs
- Expand/collapse stays discoverable: update footers, src/ui/screens/help.rs rows, README.md, docs/GETTING-STARTED.md.
- The help overlay key-highlight feature (quick-261118-tki) requires every key-column token to be matchable — keep its
  tests green when adding/changing help rows.

### Testing
- Test-first where practical; update existing tests whose Enter semantics change.
- Gates: `rtk bypass cargo test --no-fail-fast` and `rtk bypass cargo clippy --all-targets -- -D warnings`, raw counts.

### Claude's Discretion (INFERRED unless the evidence says otherwise)
- Exact key for direct expand/collapse once Enter opens a menu (INFERRED preference: Space and Right/l if they do not
  collide with folds; research must check what ←/→ currently do on group rows vs fold rows).
- Whether unavailable actions are hidden or dimmed-with-reason (INFERRED: dim with reason when the action exists for
  the kind but is blocked right now, e.g. no live session / pending settle; hide when it never applies to the kind).
- Menu placement (INFERRED: anchored below/above the selected row, clamped to the frame).

</decisions>

<specifics>
## Specific Ideas

- Footer example: `[v]run /gsd-verify-work 16 [x]settle [o]open [y]copy`.
- Enter on a check line / `file PATH` line keeps opening the read-only viewer directly (quick-261118-u4e).
- Constraints from CLAUDE.md: work on branch feat/v2.5.0-owner-inbox in /home/blk/projects/rust/gsd-meta-manager-owner-inbox,
  path-scoped conventional commits, never rebase/force/push (public origin), US spelling.

</specifics>

<canonical_refs>
## Canonical References

- src/ui/screens/detail.rs (Owner block), src/ui/screens/inbox.rs, src/ui/screens/settle_popup.rs, src/ui/screens/help.rs
- Glyph rules under src/ui/ (search for the glyph module / tests)
- .planning/quick/*261118-u4e*, *261118-v6l*, *261118-tki* (recent related quick tasks)
- README.md, docs/GETTING-STARTED.md

</canonical_refs>
</content>
</invoke>

<post_research_decisions>
## Decisions on research open questions (orchestrator, unattended — all INFERRED)

- Owner block `y` on the Old debt fold: align with the inbox (copy works in both). The current refusal in the Owner
  block is treated as an unjustified inconsistency and fixed, so it does NOT become a documented exception. INFERRED.
- `x` on a row that is already settling (pending badge from quick-261118-v6l): the hotkey and the menu both refuse with
  one shared message (e.g. "settle already pending"); in the menu the entry is shown dim with that reason (text, not
  color-only). Prevents a double settle. INFERRED.
- The only documented screen-specific exception is `g` (inbox: go to project; Owner block: first row — not an Owner row
  action), unless implementation finds another justified one, which must then be added to the exception list with a doc
  comment.
- Fold header Enter keeps toggling the fold (no menu); check/file lines keep opening the viewer; group/debug rows open
  the menu. Space toggles expand/collapse on any expandable row; Right/`l` expand, Left/`h` collapse on rows where they
  are swallowed today (research verified no conflict with fold navigation). INFERRED from research.
- Menu ignores mouse clicks on entries (neither screen supports row mouse selection today) but must swallow clicks while
  open so the detail screen does not switch tabs underneath it. INFERRED.
- docs/GETTING-STARTED.md has no Owner section: add a short Owner/Inbox actions section. INFERRED.
- `v` in tests: add a test-only recording hook (like `take_settle_launches`) so menu-vs-hotkey equivalence tests never
  launch for real.
</post_research_decisions>
