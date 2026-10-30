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
  - id: IN-06
    severity: info
    disposition: open
    title: "`state.json` is read under DIRECTION.md's 1 MiB cap but written with no cap, so a run whose state outgrows it becomes permanently unopenable (new, 2026-10-29)"
  - id: IN-07
    severity: info
    disposition: open
    title: "The legacy-key wording is imprecise in two places: \"since the Phase 28 launcher\", and \"written back unchanged\" for a typed field (new, 2026-10-29)"
open: 13
total: 14
recorded: 2026-10-30T00:39:38.282Z
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
| IN-06 | info | open | - |
| IN-07 | info | open | - |

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

## Re-verification notes (incremental review 6e8e1c0, delta e16a201..e15f3e6)

The 2026-10-29 incremental re-review covers quick batches 261028-t3c / 261028-v2z and quick 261029-1dp (`ce86206`,
`AgentRuntime` removal and doc-comment fixes in `src/ui/screens/{detail,mod}.rs`). None of the files behind WR-01..WR-07
or IN-01..IN-05 changed since e16a201, so those rows are carried unchanged. Delta findings already recorded in the
Phase 29, 30 and 31 reviews are cross-referenced in 24-REVIEW.md and not re-raised here. Two new Info findings take the
next free ids, IN-06 (`state.json` read under a 1 MiB cap but written uncapped, `src/driverctl/store.rs:645,742-763`)
and IN-07 (legacy-key wording in `src/config.rs:63-66` and `docs/CONFIGURATION.md:84`). Neither is in Phase 24's own
surface, neither is critical or security-relevant, and both stay `open`: no gap closure and no `--fix` this round.
[inferred — cleanup 2026-10-28]
