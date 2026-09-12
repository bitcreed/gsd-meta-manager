---
created: 2026-09-12T05:13:07.293Z
title: Show plan token estimate and actual counts
area: ui
severity: major
files:
  - src/state_reader/
---

## Problem

For each plan, show `estimate.tokens` from the plan's frontmatter
(`*-PLAN.md`) and, once executed, show `actuals.tokens` from the
corresponding `*-SUMMARY.md` frontmatter, if available.

There is currently no parser for `*-PLAN.md` or `*-SUMMARY.md` frontmatter in
`src/state_reader/` (only `roadmap_md.rs`, `state_md.rs`, `queue_md.rs`,
`backlog.rs`, `config_json.rs`, and `workstreams.rs` exist) — this is new
capability, not a tweak to existing parsing. Users currently have no way to
see planned vs. actual token cost per plan without opening each
`*-PLAN.md`/`*-SUMMARY.md` file directly, which defeats the point of a
single-pane dashboard across multiple GSD projects.

## Solution

TBD. Likely needs:
- A new state-reader module (or extension of an existing one) that locates
  and parses `*-PLAN.md` frontmatter for `estimate.tokens`, and, when a
  matching `*-SUMMARY.md` exists for that plan, parses its frontmatter for
  `actuals.tokens`.
- Surface both values next to each plan in the phase/plan list UI —
  estimate always if present; actual only once the plan has executed and a
  summary file exists (fall back gracefully when either field is absent,
  since not all GSD versions/plans populate `estimate`/`actuals`).
- Cache parsed values the same way other `.planning/` state is cached
  (per CLAUDE.md: `Arc<RwLock<HashMap<PathBuf, ProjectState>>>`, invalidated
  on `notify` events for that project's directory).
