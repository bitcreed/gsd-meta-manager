---
quick_id: 260722-emn
plan: 7
item: F2
type: execute
wave: 3
status: complete
subsystem: state_reader
tags: [smart-entry, gsd-tools, queue, gsd-health, w019]
key-files:
  modified:
    - src/state_reader/queue_md.rs
metrics:
  tasks: 3
  files: 1
  completed: 2026-07-22
---

# Quick Task 260722-emn Plan 7: smart-entry shell-out + queue relocation Summary

`suggest_next_commands` now defers to GSD 1.8.0's `gsd-tools smart-entry` when a
launcher resolves for the project, keeping the keyword heuristic as an instant
fallback; the queue file was relocated to `.planning/meta-manager/QUEUE.md` so
`/gsd-health` W019 no longer flags it.

## What Was Built

### Task 1 — Resolve gsd-tools, run smart-entry, parse its JSON (`7c4d74c`)
- `parse_smart_entry_json(raw)` (pure): deserializes `{ actions: [{command, recommended}] }`
  with `serde_json`, places the `recommended: true` command first and the rest in original
  order, returns `None` for empty/missing `actions` or malformed JSON. Never panics.
- `resolve_gsd_tools(project_root)`: checks the four candidate launchers in GSD's own
  resolution order — `<root>/gsd-core/bin/gsd-tools.cjs`,
  `<root>/.claude/gsd-core/bin/gsd-tools.cjs`, `~/.claude/gsd-core/bin/gsd-tools.cjs`
  (each via `node`), then `gsd-tools` on `PATH` — returning `None` if none resolve.
- `smart_entry_commands(project_root)`: runs `<launcher> smart-entry --json` with
  `current_dir(project_root)` via `std::process::Command`, degrading to `None` on spawn
  failure, non-zero exit, or parse failure.
- Inline tests cover recommended-first ordering, non-recommended order preservation,
  empty-actions → None, and garbage → None.

### Task 2 — Prefer smart-entry in suggest_next_commands, keyword fallback (`22b2969`)
- Extracted the original body into private `keyword_suggestions(state)` (verbatim logic).
- `suggest_next_commands` now: if `project_root` is non-empty, tries `smart_entry_commands`
  and returns a non-empty result; otherwise falls back to `keyword_suggestions`.
- Public signature `suggest_next_commands(&ProjectState) -> Vec<String>` unchanged — no UI
  call sites (detail.rs / enqueue.rs) or mod.rs touched.
- Doc-comment notes on-demand (not per-frame) invocation and instant fallback.
- Added a test asserting an empty `project_root` uses the keyword path; the three existing
  `test_suggest_*` tests remain green.

### Task 3 — Relocate queue to .planning/meta-manager/QUEUE.md, W019 fix (`882f140`)
- Added `queue_paths(planning_dir) -> (canonical, legacy)` returning
  `.planning/meta-manager/QUEUE.md` and legacy `.planning/QUEUE.md`, with a doc-comment
  citing gsd-core `src/verify.cts` (subdirs skipped via `if (!entry.isFile()) continue`)
  and `src/artifacts.cts` (root-file allowlist) for the W019 rationale.
- `load_queue`: reads canonical first, falls back to legacy root — existing installs keep
  working with no user action.
- `save_queue`: writes canonical atomically (tmp+rename, `create_dir_all` for the subdir),
  then removes a lingering legacy root file (one-shot migration on first write). An empty
  save removes both canonical and legacy files.
- Tempdir tests: legacy-only read, canonical-wins-over-legacy, save-writes-new-and-migrates,
  empty-save-removes-both.

## Verification

- `cargo build`: clean.
- `cargo test`: 201 passed (5 suites). queue_md module tests: 20 passed.
- Only `src/state_reader/queue_md.rs` modified; no mod.rs or UI file edited; public
  `suggest_next_commands` signature unchanged.

## Deviations from Plan

None — plan executed exactly as written. Per the executor's explicit constraint, each TDD
task was committed as a single atomic commit (test + implementation together) using the
commit messages specified in the plan's `<commit>` block, rather than separate RED/GREEN
commits.

## Self-Check: PASSED

- `src/state_reader/queue_md.rs`: FOUND (modified)
- Commit `7c4d74c` (Task 1): present
- Commit `22b2969` (Task 2): present
- Commit `882f140` (Task 3): present
