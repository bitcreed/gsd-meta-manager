# Phase 38: Inbox digest and triage hygiene - Context

**Gathered:** 2026-11-16
**Status:** Ready for planning

Gathered unattended (`--auto`). Decisions marked **[user]** were made by the owner before this run
and are locked. Everything else follows ROADMAP Phase 38, REQUIREMENTS OWN-24..28,
`.planning/research/OWNER-INBOX-UX.md`, the Phase 34-37 decisions and a read-only look at the live
fleet (`./target/release/gsd-meta-manager inbox`, 2026-11-16: 361 lines of output, 18 projects,
test 165). Decisions marked **[inferred]** were taken by the orchestrator from those sources.

<domain>
## Phase Boundary

Plain `gsd-meta-manager inbox` answers "what should I do next?" in one screen. Today it prints the
full checklist: 361 lines on the owner's fleet. This phase delivers:

- **Digest by default** (OWN-24): totals, per-project lane counts with the `closes after` phases,
  and the top ~5 next actions ranked by payoff.
- **Views** (OWN-25): `inbox <alias>` (full checklist for one project), `--lane` (one lane across
  projects), `--all` (the pre-Phase-38 output, byte for byte).
- **Collapse** (OWN-26): every multi-project view (CLI digest, CLI lane view, TUI inbox `i`) shows
  one entry per phase (count + action), never its checks.
- **Hygiene** (OWN-27): stale fold (~90 days), same-phase duplicate merge (sentriq ph 43), aligned
  `1 check`/`2 checks` column in the TUI.
- **Labels** (OWN-28): `decision?` and `looks resolved`, label only, in the CLI, the TUI and
  `--json`.

Requirements: OWN-24, OWN-25, OWN-26, OWN-27, OWN-28.

Not in this phase: check actions through GSD (won't-do, defer/move: Phase 39, OWN-29..31); context
tags desk/phone/truck **[user: deferred]**; any write to a GSD file **[user]**; moving or hiding a
row because of a heuristic (OWN-F02 stays deferred); changes to the dashboard `You` cell, summary
line or `status --json` `owner` counts.

</domain>

<decisions>
## Implementation Decisions

### Carried forward (not re-decided)
- PROJECT.md "GSD files are read-only" (2026-11-16) and report D7: the app writes nothing into a
  project. Phase 38 is reader and renderer work only.
- Layer A / Layer B split (34-CONTEXT): Layer A must keep matching `audit-uat` (oracle test). Every
  Phase 38 change is Layer B or rendering; Layer A and the oracle fixtures stay untouched.
- D3 strict dedup: a UAT whose `source:` does not name the VERIFICATION does not supersede it.
  The D-08 merge below is display grouping, not supersede: no item is dropped.
- 36 D-17 vocabulary (`ph N`, `ws` prefix, `N checks`, `closes phase`, `stale`, lane tags) and 37
  D-04 (one DECIDE row per debug session, not aggregated).
- 37 D-09: `y` copies exactly what the CLI prints for that group, through `group_checklist`.
- `inbox --json` schema `gsd-meta-manager/inbox` v1, deterministic, `status`-style exit codes;
  additive keys only (SC6).
- `stale` marker at 30 days (`preferences.owner.stale_days`, D9) is unchanged. The 90-day fold is a
  separate threshold (D-09).

### CLI views (OWN-24, OWN-25)
- **D-01 [user]:** Plain `inbox` prints a short digest: per-project counts per lane plus the top
  few items. `inbox <alias>` shows one project; `--lane <test|decide|fix|blocked>` one lane;
  `--all` the full list; `--json` keeps carrying everything.
- **D-02:** View selection, in order **[inferred]**:
  1. `--json`: unchanged document (plus the additive keys of D-12). `--all` and the view choice do
     not change it; `--lane`/`--include` keep filtering as today.
  2. `--all`: the legacy `render_text` exactly as on master today, with `--lane`/`--include`
     honored as today. No merge, no fold, no labels, so `--all` equals the pre-phase output byte
     for byte on every fixture (SC2). The current `tests/fixtures/owner_golden/inbox.txt` and
     `inbox-include-all.txt` become the `--all` goldens unchanged.
  3. Exactly one project argument (alias or path): the full checklist for that project (D-04).
  4. `--lane` without a single project: the lane view (D-05).
  5. Otherwise (no argument, or two or more): the digest (D-03) over the selected projects.
  `--include` given without `--lane`, a single project or `--all` implies `--all`, so existing
  `inbox --include archived` invocations still print the full list **[inferred: back-compat]**.
- **D-03: Digest layout** (target 15-25 lines, asserted by a line-budget test on the fixtures and
  a synthetic 20-project fleet) **[inferred: exact layout]**:
  1. The existing totals line (`Owner inbox: N projects · test T · decide D · fix F phases ·
     blocked B`), numbers unchanged so they keep equaling `summary` and `status --json` `owner`.
  2. Per-project block: one line per project with owner work: alias, milestone version, then the
     `closes after` clauses in `ClauseForm::PhasesCapped(4)` (lane counts with their phases), and,
     for a project whose work is not in `closes_after` (for example root-scoped debug sessions),
     its lane counts alone. At most 10 lines; ordered by closeable phases (most first), then
     alias; the rest summarized as `+N more projects (inbox --all)`.
  3. `NEXT` section: the top 5 next actions (D-07), one line each: alias, `ph N` (with `ws`
     prefix), name capped, `N checks`, `→ closes phase` when it does, labels count (D-11), age,
     command.
  4. The fold lines (Old debt, Stale (D-09), Acknowledged, Deferred, Triage) as today, one line
     each.
  5. A hint line: `inbox <alias> · inbox --lane test|decide|fix|blocked · inbox --all`.
- **D-04: `inbox <alias>`** prints the full checklist for that project with the hygiene applied:
  groups and their items as today, D-08 merged entries, D-11 labels on item lines, and stale groups
  (D-09) listed in a trailing `STALE (> 90 d)` section instead of in their lane. Nothing is hidden
  for a single project **[inferred]**. A 51-step scripted UAT (gnome-victron ph 04 today) lists its
  51 items here; that is the point of the full view.
- **D-05: `inbox --lane L`** (any number of projects) prints the lane's sections exactly as the
  full output orders them, but collapsed: one line per phase entry (D-08 merged), no item lines,
  stale entries folded into one line at the end (D-09). Several `--lane` values print several
  lanes in the usual section order.

### Collapse (OWN-26)
- **D-06 [user: B]:** In multi-project views a phase is one entry (`ph 43 · 3 checks` plus the
  action), never its individual checks. Applies to the digest, the lane view and the TUI inbox
  `i`. The TUI already shows one row per group with items behind `Enter` (37 D-04/D-06); Phase 38
  adds the merge (D-08), the stale fold row (D-09) and labels (D-11) there. The detail Owner block
  is a single-project view and keeps its 36 layout (frozen for the pending 36 UAT).

### Ranking (OWN-24)
- **D-07:** Next actions are chosen from current-milestone, non-acknowledged, non-folded entries in
  Test, Decide and Fix (Blocked is not actionable by the owner, as in 3.1). Sort key: entries that
  close a phase first; then fewest checks (the merged entry's sum) first; then **newer first**
  (older items rank lower, per OWN-24; deliberately the opposite of the TUI's oldest-first triage
  order, which stays as 37 D-05 locked it); then alias, phase key, file for a total order. Bookkeeping-only entries
  ("all checks passed, run /gsd-verify-work N to finalize") count as 0 checks and close a phase,
  so they rank first. One pure, unit-tested function shared by the text digest and `--json`
  (D-12) **[inferred: candidate set, tie order, bookkeeping rank]**. Ages are as R9 computes them.

### Hygiene (OWN-27) — data cleanups (allowed to change what is shown)
- **D-08 [user: D]: Same-phase merge.** Groups with the same project, scope, workstream, phase and
  lane are shown as one entry. Count = the sum of the groups' checks; the entry names its files
  (`2 files`); age = the newest group's age; command and `closes_phase` are per phase and equal.
  Live case: sentriq `v0-15-buyers-mode` ph 43 lists `43-UAT.md` (2 checks, `source:` names only
  SUMMARYs) and `43-VERIFICATION.md` (`human_needed`, 1 check) as two rows today; after the merge
  one entry `ph v0-15-buyers-mode/43 · 3 checks · 2 files`. No item-level dedup: the two
  "real-truck crank" items look alike, but matching them is a heuristic, so both stay (the user
  allows only labels for heuristics) **[inferred]**. Merge happens in Layer B or the view model;
  the JSON `groups` array keeps one group per file (key paths and the oracle unchanged) and gains
  the additive link of D-12. Expanding a merged entry in the TUI lists the items per file, with the
  file name as a sub-heading **[inferred]**. `v` and `y` act on the merged entry (same phase, same
  command; `y` copies all its items). `A` stays file-level: on a merged entry it asks which file
  to acknowledge, or is refused with a message telling the user to expand and pick a file. The
  planner chooses whichever is simpler **[inferred]**.
- **D-09 [user: D]: Stale fold.** Current-milestone entries whose age exceeds the fold threshold
  leave their lane in every multi-project view and are counted in one line `Stale: N checks in P
  phases older than 90 d`, which sits next to the other folds. In the TUI it is a fold row that
  `Enter` expands, like Old debt. Threshold: `preferences.owner.fold_days`, default 90, clamped
  like `stale_days` **[inferred: key name and clamping]**. Entries with no age never fold. Totals,
  `closes_after`, `status --json` `owner` and the dashboard cell keep counting folded entries.
  The work still blocks the milestone; the fold only takes it out of the triage list.
- **D-10 [user: D]: `1 check`/`2 checks` alignment.** The count column renders the number
  right-aligned and the unit left-aligned (`1 check ` / `2 checks`), so the digits line up. Applies
  to the TUI inbox and the CLI collapsed lines. The detail Owner block has the same right-aligned
  `N check(s)` cell, so it also uses the shared helper. The change is alignment only, not layout,
  and its goldens move by at most one cell **[inferred]**. Buffer snapshot asserts the alignment
  (SC4).

### Heuristic labels (OWN-28) — label only
- **D-11 [user: D]:** Labels never move, hide, fold, re-rank or re-count a row. They change no
  lane, no total, no `closes_phase`, no ranking key and no fold.
  - `decision?`: an item's name or `expected` matches, case-insensitively and on word boundaries,
    `owner decision`, `decide`/`decision`, `ratify`/`ratification`, `approve`/`approval`,
    `accept`, `sign-off`/`sign off`, `rule on`, `review`; and `OK` matched case-sensitively as a
    whole word (lowercase "ok" is too common) **[inferred: exact list and forms]**. Not applied to
    Decide-lane items: those are already decisions.
  - `looks resolved`: name, `expected` or `reason` matches `resolved`, `fixed by`, `fixed in`,
    `closed by`, `addressed by`, `addressed in`, `superseded by` **[inferred: exact list]**.
  - One pure matcher module in Layer B with a table-driven test, including false-positive guards
    such as "unresolved", "review" inside another word and "okay".
  - Display: on an item line, a dim suffix (`decision?`, `looks resolved`). On a collapsed entry,
    a dim count (`2 decision?`) only when non-zero. CLI and TUI share the wording.

### JSON (SC6)
- **D-12:** Additive keys only; every key path pinned in `tests/inbox_cli.rs` stays. Names are the
  planner's choice; the data is fixed **[inferred]**:
  - per item: `labels` (array of strings, `[]` when none);
  - per group: `folded_stale` (bool) and a merge link, so a consumer can rebuild the D-08 entry
    (for example `entry_files`: the basenames of every file in the same entry, sorted);
  - `summary`: `next_actions` (the D-07 list: alias, workstream, phase, lane, checks,
    closes_phase, command, age_days) and `stale_folded` counts.
  README `inbox` schema, `status --skill` paragraph (`inbox` now prints a digest; agents use
  `--json` or `--all`) and the help overlay (TUI stale fold row) are updated.

### Tests and UAT
- **D-13:** Fixtures under `tests/fixtures/owner/` with synthetic text:
  - `same-phase-merge` modeled on sentriq ph 43 (UAT whose `source:` names SUMMARYs + a
    `human_needed` VERIFICATION, in a workstream): JSON two groups with the link, one text entry
    with 3 checks;
  - `stale-fold` (an `updated:` more than 90 days back, next to a fresh group);
  - `scripted-checklist` (one UAT with 51 numbered tests): one collapsed line in the digest, lane
    view and TUI, 51 items in `inbox <alias>`;
  - `heuristic-labels` (keyword hits, near misses, a Decide-lane item that is not labeled).
  Goldens: `inbox-digest.txt` (new, line budget asserted), `inbox-lane-test.txt`,
  `inbox-alias.txt`. The existing `inbox.txt` and `inbox-include-all.txt` stay byte-identical as
  `--all` goldens. TUI buffer snapshots: merged entry, stale fold row, aligned count column,
  labels. The oracle conformance test is unchanged and green. `render_escape_guard` covers the new
  strings. Ages in goldens use the existing clock seam (the planner confirms one exists, else
  adds one) **[inferred]**.
- **D-14:** Human UAT (ROADMAP): one readability check. Does `inbox` on the real fleet read as a
  useful one-screen answer? Nothing else goes to a human.

### Musk pass
- Phase pass, no cuts. OWN-24..28 come from the owner's own feedback after the first real use, and
  decisions A, B and D settle them. Every item has an owner. Lean lane not used: the work spans
  three subsystems (CLI renderer, Layer B rules, TUI inbox) **[inferred]**.

### Claude's Discretion
- Exact digest wording, column spacing and the 10-line project cap (within the 15-25 line budget).
- Names of the additive JSON keys (D-12) and of the fold config key (D-09).
- Whether the merge (D-08) lives in Layer B (`owner_items/rules.rs`) or in a shared view model
  used by `inbox.rs` and `ui/screens/inbox.rs`, as long as both views share one implementation.
- `A` behavior on a merged entry (D-08).
- Number and grouping of plans (coarse granularity).

</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Scope and contract
- `.planning/ROADMAP.md` §"Phase 38" — goal, success criteria 1-6, human UAT
- `.planning/REQUIREMENTS.md` — OWN-24..28, OWN-F02 (move/hide stays deferred), OWN-29..31 (Phase
  39, out of scope)
- `.planning/PROJECT.md` — constraint "GSD files are read-only" (2026-11-16)
- `.planning/research/OWNER-INBOX-UX.md` — §3 principle (rows are phases), §3.3 inbox, §3.5 CLI
  and schema, §4 R2/R7/R8/R9 rules, §7 D3/D9
- `README.md` — "Owner inbox" section and the `inbox` schema (to extend additively)

### Prior phases
- `.planning/phases/34-owner-items-reader-inbox-cli/34-CONTEXT.md` — Layer A/B split, oracle, counts
- `.planning/phases/35-decisions-code-needed-and-gates/35-CONTEXT.md` — Decide/Fix signals,
  `closes_after`
- `.planning/phases/36-dashboard-and-detail/36-CONTEXT.md`, `36-UI-SPEC.md`, `36-UAT.md` — Owner
  block layout (frozen, pending UAT), vocabulary, tokens
- `.planning/phases/37-inbox-screen-and-actions/37-CONTEXT.md`, `37-UI-SPEC.md`,
  `37-UI-REVIEW.md` — inbox screen rows, ordering D-05, `y` shared formatter D-09
- `.planning/phases/37-inbox-screen-and-actions/37-UAT.md` — pending owner checks. Item 1 notes
  the checks column alignment, which D-10 settles. Item 6 (DECIDE/FIX below the fold) is still the
  owner's call and is not decided here.

### Code
- `src/cli.rs` ~266-310 — `Commands::Inbox` (`projects`, `--json`, `--lane`, `--include`; add `--all`)
- `src/inbox_cli.rs` ~144 — text dispatch (`render_text`)
- `src/inbox.rs` — `render_text` (becomes the `--all` path unchanged), `group_line`,
  `group_checklist` (shared with TUI `y`), `push_closes_after`, `closes_after_clauses`
  (`ClauseForm::PhasesCapped`), `build_document` (JSON), `plural`
- `src/state_reader/owner_items/rules.rs` — Layer B (merge link, labels, fold flag land here or
  beside it)
- `src/state_reader/owner_items/mod.rs` — `OwnerGroup`, `OwnerItem`, `OwnerDigest`
- `src/config.rs` ~540 — `owner_stale_days` (pattern for the fold threshold)
- `src/ui/screens/inbox.rs` — rows, `CHECKS_W` (~519), count cell (~716), folds (~1283-1345),
  `sort_groups`
- `src/ui/screens/owner_block.rs` ~389 — the Owner block count cell (shared alignment helper)
- `tests/inbox_cli.rs` — JSON key-path pins (must stay unchanged)
- `tests/fixtures/owner/`, `tests/fixtures/owner_golden/` — fixtures and goldens

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `closes_after_clauses(gate, ClauseForm::PhasesCapped(n))` already produces the compact
  per-project lane clauses the digest needs.
- `group_line` / `group_checklist` give the collapsed line and the full checklist; the lane view
  is `group_line` without items.
- `Filters::lists_group` and the `--lane`/`--include` plumbing exist; `--all` only routes to the
  current `render_text`.
- The TUI inbox already renders one row per group with expand on `Enter`, fold rows (Old debt,
  Acknowledged, Todos) and a `sort_groups` function.
- `Preferences::owner_stale_days` is the pattern for a clamped owner threshold.

### Established Patterns
- Render reads caches only; no parse on the render path (OWN-12, `async_blocking_guard`).
- Every project-derived string goes through `terminal_capped` / `render_for_terminal`.
- Non-default states announce themselves, defaults are silent (labels are dim, shown only when
  non-zero).
- One shared formatter between CLI and TUI, so the two cannot disagree (37 D-09).

### Integration Points
- `inbox_cli.rs` view dispatch → new digest / lane / alias renderers.
- `OwnerDigest` → additive merge link, `folded_stale`, item `labels`; JSON `build_document`.
- TUI inbox rows → merged entries, stale fold row, labels, aligned count.

</code_context>

<specifics>
## Specific Ideas

- Live numbers (2026-11-16, read-only run of the worktree build): plain `inbox` prints 361 lines
  for 18 projects; the `closes after` block alone has 10 lines; gnome-victron ph 04 is the
  51-check scripted UAT; sentriq `v0-15-buyers-mode` ph 43 is the duplicate pair (UAT 2 checks +
  VERIFICATION 1 check).
- Example digest line: `  wordoclock  ph 22  iOS Widget Faces  2 checks → closes phase  12 d
  /gsd-verify-work 22`.

</specifics>

<deferred>
## Deferred Ideas

- Context tags (desk / phone / truck) on checks **[user: deferred]**.
- Moving or hiding rows by heuristic (OWN-F02 beyond labels).
- Item-level dedup of near-identical checks across a merged UAT/VERIFICATION pair (needs text
  matching, a heuristic).
- A `NEXT` / digest view inside the TUI (the TUI inbox already is the triage view; not required by
  OWN-26).
- 37 UAT item 6 (section counts or a lane strip so DECIDE/FIX show on the first screen): the owner
  decides in the 37 UAT.
- Check actions through GSD: Phase 39.

### Reviewed Todos (not folded)
`todo.match-phase 38` returned the same keyword-only matches Phases 34-37 reviewed (TUI badge for
release maps, help popup at 80x24, launcher-marker in-flight flag, unregister guard, multi-line
queue input, ...). None touches the inbox digest, the views or the hygiene rules, so none is
folded despite the ≥ 0.4 auto-fold rule; folding them would be scope creep **[inferred]**.

</deferred>

---

*Phase: 38-inbox-digest-and-triage-hygiene*
*Context gathered: 2026-11-16*
