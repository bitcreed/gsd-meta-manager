# Quick Task 260923-md1: Roadmap dependency graph view with milestone boundaries - Context

**Gathered:** 2026-09-23
**Status:** Ready for planning
**Mode:** unattended — the human is unavailable. Every decision below marked
`[INFERRED]` was taken by the orchestrator from the brief + codebase and must be
listed in the SUMMARY for later audit.

<domain>
## Task Boundary

The Detail screen's Roadmap tab (`src/ui/roadmap_widget.rs`, called from
`src/ui/screens/detail.rs` ~line 3859) renders phases as a vertical stack of
boxes and ignores dependencies. Render the phases instead as a left-to-right
dependency graph so parallelizable phases are visible, AND show which milestone
each phase belongs to. User's target look (their sketch):

```
8 ─► 9 ─┬─► 10 ─┬─► 12 ─► 13 ─┬─► 14 ─► 15        (M3: live booking)
        └─► 11 ─┘             └─► 16 ─► 17        (M4: support chat)
                   12 ─────────────► 18           (M5: web, kept last as you said)
```

Scope addition from the user (REQUIRED, not optional): milestone boundaries
must be visible in the graph; the sketch was missing them.
</domain>

<decisions>
## Implementation Decisions

### Data source (locked by brief)
- Dependencies come ONLY from `RoadmapPhase::depends_on` (declared `**Depends on**:`).
  Never infer "N depends on N-1" from numbering. No declared deps = root node.
- Phase id matching MUST use `crate::state_reader::phase_num::same_phase`
  (pad-insensitive, `07.1 == 7.1`), never string equality. Decimal ids (21.1) are ordinary nodes.
- Deps naming phases not in the current phase list (archived / unknown) are ignored for
  layout and surfaced in one note line under the graph, e.g. `external deps: 14 ◄ 7`. [INFERRED]

### Layout algorithm — pure module `src/ui/roadmap_graph.rs` [INFERRED details]
- Pure, no ratatui `Buffer` dependency in the core: input = ordered list of
  `(id, deps, milestone)`; output = a `GraphLayout` of rows made of typed segments
  (node label / reference label / edge glyphs / milestone label) that BOTH a
  plain-text renderer (for tests) and the widget (for styling) consume.
- Layering: longest-path, left→right (`layer = 1 + max(layer(dep))`, roots = 0).
- Cycles: detect (Kahn / DFS colouring, no recursion depth risk, must terminate);
  edges that close a cycle are dropped from layout and a note line
  `dependency cycle: A, B, C (edges ignored)` is emitted. Never hang or panic.
- Rows: every non-root node has a "primary parent" = its first-declared dep in
  layer-1 (exists by construction of longest-path). Primary edges form a forest.
  Each node's first primary child (roadmap order) continues on the parent's row
  (keeps chains on one line). Other children branch to a lower row found by
  searching downward from the parent row for the first row that is free for the
  whole branch (junction gap → last chain node → its trailing gap) AND whose
  intermediate rows are free at the junction column (for the `│` pass-through).
  Process branch children right-to-left (deepest layer first) so later short
  branches can tuck into rows under earlier ones. Roots after the first start on
  a fresh row below everything.
- Secondary (non-primary) edges between adjacent layers are drawn as fan-in into
  the target's junction when the junction column is free over the needed rows.
  Any edge that can't be drawn cleanly (spans >1 layer, junction collision,
  ambiguous merged bipartite shape) becomes a **reference row** appended at the
  bottom: `<src> ─────► <tgt>` with both labels at their own layer columns and
  styled as dim references — exactly the "repeat the source label on its own row
  with a long arrow" device from the sketch.
- Gutter width between layer k and k+1: 6 cells (` ─┬─► `) if that gap holds any
  junction, else 4 (` ─► `). Junction glyph at gap offset 2. Junction cell glyph is
  derived from its connections (left/right/up/down) → `┬ └ ┘ ├ ┤ ┴ ┼ │ ─ ┐ ┌`.
- Column width per layer = max label width in that layer; shorter labels on a row
  that continues rightwards are padded with `─`, otherwise with spaces.

### Expected rendering of the user's example topology (test oracle) [INFERRED]
Topology: 8→9→{10,11}→12→13→{14→15, 16→17}, 12→18. With longest-path layering
18 sits in the same layer as 13, so it is drawn as a direct fan-out sibling of 13
(a `│` passes the empty cell on row 2) rather than as a reference row — this is
structurally more faithful than the sketch (18 is parallel-eligible with 13).
Milestones for the test: 8–15 → `M3 live booking`, 16–17 → `M4 support chat`,
18 → `M5 web`; active milestone M3. Expected graph body (before milestone
decorations; executor may adjust whitespace only if the plan-checker agrees and
the SUMMARY records why):

```
8 ─► 9 ─┬─► 10 ─┬─► 12 ─┬─► 13 ─┬─► 14 ─► 15
        └─► 11 ─┘       │       └─► 16 ─► 17
                        └─► 18
```

### Milestone boundaries (REQUIRED) [INFERRED rendering]
- Membership source: parse ROADMAP.md with existing reader code where possible
  (`src/state_reader/roadmap_md.rs` — see `active_milestone`, the `## Milestones`
  list with `Phases A-B` ranges, milestone section headings like
  `### v2.0 Autonomous Orchestration (Phases 14-23)` / `<summary>` blocks /
  `## Milestone N: …` sections). Add a small pure fn (e.g.
  `phase_milestones(content) -> Vec<(phase_id, milestone_label)>` or ranges)
  next to the existing parsers, unit-tested; range membership uses numeric
  phase comparison via `phase_num`, not strings. If no membership can be derived,
  the graph renders without milestone decorations (never fails).
- Rendering, three cues, all driven by one node→milestone map:
  1. **Header band** above the graph: one line listing milestones in roadmap
     order, `Milestones: ◆ M3 live booking  ◇ M4 support chat  ◇ M5 web`, with the
     active milestone highlighted (bold/yellow, `◆`).
  2. **Row-end label** (matches the sketch): each row ends with
     `(<milestones of the row's real nodes in order, joined by " → ">: <name of
     the last one>)`, e.g. `(M3)`-style; row 2 of the example →
     `(M3 → M4: support chat)`, row 1 → `(M3: live booking)`, row 3 → `(M5: web)`.
     The `→` shows exactly where a milestone boundary is crossed inside a row.
  3. **Boundary marker inside a row**: none beyond cue 2 (keeps edges legible).
- Active milestone = STATE.md milestone if available on `ProjectState`, else
  `roadmap_md::active_milestone`.

### Styling (locked by brief)
- Keep per-phase status styling via `PhaseMarker::decide` (Done = DarkGray,
  Current = Yellow+BOLD, Future = default). Current phase node additionally
  highlighted (REVERSED or `▶` prefix). Reference labels dim.
- Under the graph: one detail line for the current phase: `▶ P<id>: <name>`.
- ALL third-party text (phase ids, names, milestone labels) passes through
  `crate::text::render_for_terminal` / the `Untrusted`/`shown()` pattern used in
  `roadmap_widget.rs`, and the census in `crate::ui::tests` /
  `src/ui/screens/render_escape_guard.rs` must be extended to cover the new sink.

### UX
- Graph view is the DEFAULT for the Roadmap tab; a single key toggles back to the
  existing box list (pick an unused key on that tab, `v` suggested — verify no
  conflict). Document it in `src/ui/screens/help.rs` with a minimal, localized
  edit (a parallel task edits help.rs ~line 84 and session code in
  normal.rs/detail.rs — keep diffs small and away from those regions). [INFERRED]
- Wide graphs: horizontal offset computed automatically to keep the current
  phase in view; vertical scroll reuses the existing `scroll_offset`/viewport
  metrics. No new manual horizontal-scroll key unless trivially free. [INFERRED]
- Must not panic at any terminal size (test tiny areas: 1x1, 5x3, 10x3, 20x5).

### Claude's Discretion
- Exact type names, segment representation, cell/char width handling (ids are
  ASCII in practice; measure the escaped form by chars like the existing widget).
- Whether the toggle state lives on the Detail screen struct or per-project.
</decisions>

<specifics>
## Specific Ideas

- Tests required: example topology with milestones → assert exact rendered text
  lines (header band + graph + row-end labels); cycle terminates with note;
  unknown dep ignored with note; decimal id `21.1` and padded `07`/`7` matching;
  no deps at all → each phase a root on its own row; narrow-area no-panic;
  escape-guard coverage for a phase name / milestone name containing ESC / U+202E.
- Commands: `rtk proxy cargo test --no-fail-fast`,
  `rtk proxy cargo clippy --all-targets -- -D warnings`. Known pre-existing local
  failure: git-version constants test in `src/envelope/policy.rs` only.
</specifics>

<canonical_refs>
## Canonical References

- `src/state_reader/roadmap_md.rs` — `depends_on` doc comment (declared, never inferred), `active_milestone`
- `src/state_reader/phase_num.rs` — `same_phase`, `PhaseNum`
- `src/ui/roadmap_widget.rs` — existing box view, escaping pattern, `PhaseMarker`
- `src/ui/screens/render_escape_guard.rs`, `src/ui/mod.rs` tests — escape census
</canonical_refs>
