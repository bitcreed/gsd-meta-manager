---
phase: 24
review: 24-REVIEW.md
titles: json
findings:
  - id: WR-01
    severity: warning
    disposition: open
    title: "Colliding `BandKey`s deadlock `j`/`k` navigation and fold two bands at once (still present, probe-confirmed)"
  - id: WR-02
    severity: warning
    disposition: open
    title: "`declared_phase_count` overflows u32 on third-party input (still present, probe-confirmed; narrower than first reported)"
  - id: WR-03
    severity: warning
    disposition: open
    title: "Build-phase range expansion includes the entry's own id, which creates spurious cycles and can invert a real dependency (still present, probe-confirmed)"
  - id: WR-04
    severity: warning
    disposition: open
    title: "A GSD phase entry does not end at a `#### Build phase` heading, so it inherits that heading's dependencies and plan lines (new, probe-confirmed)"
  - id: WR-05
    severity: warning
    disposition: open
    title: "The list pane title advertises a `v` key that is unbound"
  - id: WR-06
    severity: warning
    disposition: deferred
    title: "The Roadmap measures widths in chars, so wide-glyph names push the plans and wave columns out of the pane (still present)"
  - id: WR-07
    severity: warning
    disposition: open
    title: "The Roadmap cursor does not write back to the Phases selection, so the \"shared selection\" (D-B03) only works one way (still present)"
  - id: IN-01
    severity: info
    disposition: open
    title: "PageUp/PageDown steps targets, but the page size counts rows that include connectors (still present)"
  - id: IN-02
    severity: info
    disposition: open
    title: "ARCHITECTURE.md source tree omits `roadmap_graph.rs`"
  - id: IN-03
    severity: info
    disposition: open
    title: "`PROBE_PLANNED_PHASE_ID` sits inside `hostile_project_state`'s doc comment (still present)"
  - id: IN-04
    severity: info
    disposition: open
    title: "The fixture sanitisation guard is narrower than the parser and the README contract"
  - id: IN-05
    severity: info
    disposition: open
    title: "Regexes are compiled on every call beside `OnceLock` statics"
open: 11
total: 12
recorded: 2026-10-29T00:50:49.484Z
---

# Phase 24: Code Review Disposition

| Finding | Severity | Disposition | Source |
|---------|----------|-------------|--------|
| WR-01 | warning | open | - |
| WR-02 | warning | open | - |
| WR-03 | warning | open | - |
| WR-04 | warning | open | - |
| WR-05 | warning | open | - |
| WR-06 | warning | deferred | known debt: D-21-53 (21-30/21-32 SUMMARY) chose chars().count() and declined unicode-width, residual recorded; still present at src/ui/roadmap_view.rs:204-270 [inferred — cleanup 2026-10-28] |
| WR-07 | warning | open | - |
| IN-01 | info | open | - |
| IN-02 | info | open | - |
| IN-03 | info | open | - |
| IN-04 | info | open | - |
| IN-05 | info | open | - |

Dispositions: `open` (recorded, not yet triaged), `fixed`, `skipped`, `deferred`.
Set `deferred` by hand and put the reason in the Source cell; both are preserved. A `|` in the reason is kept as prose and escaped on the next run.
Re-running the gate keeps every row it can. A row the current review no longer reports is kept and its Source cell flagged, so a finding does not leave this record silently. ONE exception: when a finding id is REUSED by a different finding, the earlier decision cannot keep a row — the id is taken — and it is dropped. A RECORDED decision (anything but `open`) is named on the console when that happens; a row still at `open` is replaced silently, because `open` records no decision to lose.

## Re-verification notes (HEAD review e16a201, replacing the 2026-09-24 review 551ecc2)

The 2026-09-24 review predates this ledger and had no fix report, so none of its rows existed here. The HEAD re-review
classifies WR-01, WR-02, WR-03, WR-06, WR-07, IN-01 and IN-03 as still present from it, and WR-04 and WR-05 as new.
It confirms the stale `v` row in `src/ui/screens/help.rs` is gone, and that no live `PhaseList`, `layout_graph`,
`RoadmapGraphWidget` or `roadmap_widget.rs` references remain. The `PhaseList` hits that are left are
`archive::ArchiveDepth::PhaseList`, comments, or test strings. [inferred — cleanup 2026-10-28]

No code was changed in this round, and no `--fix` was run. Every finding except WR-06 stays `open`. WR-06 is
`deferred` because D-21-53 already accepted that residual. WR-01 through WR-04 were confirmed with a scratch probe
crate. WR-02 is reachable only through a third-party ROADMAP, where the range starts at 0. [inferred — cleanup 2026-10-28]
