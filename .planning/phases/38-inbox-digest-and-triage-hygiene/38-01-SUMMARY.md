---
phase: 38-inbox-digest-and-triage-hygiene
plan: 01
subsystem: inbox-cli
tags: [rust, inbox, triage, digest, labels]
requires:
  - phase: 34-37
    provides: owner digest (Layer A/B), inbox CLI, TUI inbox
provides:
  - triage model (same-phase merge, stale fold, next-action ranking, outside_gate)
  - D-11 label matcher
  - plain `inbox` digest, `inbox --all` legacy checklist, D-02 view dispatch
  - entry_checklist / entry_line / collapsed_block shared formatters
affects: [38-02, 38-03, 38-04]
status: complete
commits: 2
plan_head_before: 7be4a791e0126883e86d8f06d5d9b5b3e13f6d4e
plan_head_after: 19ab3929
actuals:
  tokens: 40000
  tasks: 3
  commits: 2
tech-stack:
  added: []
  patterns: [pure Layer B triage over one digest, library-local cell helpers]
key-files:
  created:
    - src/state_reader/owner_items/triage.rs
    - src/state_reader/owner_items/labels.rs
  modified:
    - src/state_reader/owner_items/gate.rs
    - src/state_reader/owner_items/mod.rs
    - src/config.rs
    - src/cli.rs
    - src/main.rs
    - src/inbox_cli.rs
    - src/inbox.rs
    - tests/inbox_cli.rs
key-decisions:
  - "Plain inbox prints the digest; --all (and --include alone) keeps the old checklist byte for byte"
requirements-completed: [OWN-24, OWN-25, OWN-26, OWN-27, OWN-28]
---

# Phase 38 Plan 01: Digest, triage model and labels Summary

Plain `gsd-meta-manager inbox` now prints a digest of at most 25 lines (totals, per-project `closes after`
lines, up to 5 ranked NEXT entries, fold lines with `Stale:`/`Unreadable:`, hint), built on a pure triage
model; `inbox --all` reproduces the pre-phase checklist (both legacy goldens untouched and matched by a
binary test with ages normalized).

## Tasks

1. Triage model, `fold_days`, `--all`, `view_of`, thin digest (commit 19ab3929 model, b787296c CLI)
2. Full D-03 layout: `gate::lane_items`, `outside_gate`, `stale_fold_text`, project block, 25-line budget
3. `labels.rs` matcher, `label_counts`, `entry_line`, `entry_checklist`, label counts on collapsed lines

Tasks were implemented in one pass and committed as two path-scoped commits (model layer, CLI layer)
rather than three, so the per-task atomicity of the plan is not literal. The CLI commit (b787296c) was
created before the model commit (19ab3929) because the first commit attempt failed on untracked paths, so
b787296c alone does not build; HEAD does.

## Verification

- `cargo test`: all suites green (lib 3613 tests, integration suites incl. `inbox_cli`, `owner_digest`).
- `cargo clippy --all-targets -- -D warnings` clean; `cargo fmt --check` clean.
- Goldens `inbox.txt`, `inbox-include-all.txt` unchanged.
- Worst-case digest test (20 projects, all fold kinds, 2 failed entries): 25 lines.

## Deviations from Plan

None in behavior. `entry_line` and `entry_checklist` carry `#[cfg_attr(not(test), allow(dead_code))]`
until plans 38-02 and 38-04 consume them (needed for `clippy -D warnings`). `outside_gate` and
`LaneCounts` were written together with the Task 1 triage module.

## Inferred decisions

- Project line `elsewhere:` part for owner work the root gate does not count; with no gate clause the lane
  counts follow the alias/version alone (`  alpha fix 1`).
- Adaptive project cap: `min(10, 25 - other lines - 1)`, last line `+N more projects (inbox --all)`.
- `Unreadable: N projects (a, b, c, ...) · inbox --all says why` sits at the end of the fold block.
- NEXT candidates require a command (RESEARCH A1).
- `items` unit in `Stale:` once a folded entry is not a UAT/VERIFICATION one.
- CLI label suffix `  (decision?)` in parentheses; merged entries print `    file NAME` lines.
- Private `cell_width`/`pad_cells` in `src/inbox.rs` instead of importing TUI helpers.

## Known Stubs

None.

## Self-Check: PASSED

triage.rs, labels.rs exist; commits b787296c and 19ab3929 are ancestors of HEAD.
