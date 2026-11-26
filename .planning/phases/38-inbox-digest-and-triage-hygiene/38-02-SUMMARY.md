---
phase: 38-inbox-digest-and-triage-hygiene
plan: 02
subsystem: inbox-cli
tags: [rust, inbox, single-project-view, fixtures, golden]
requires:
  - phase: 38-01
    provides: triage model, labels, entry_checklist
provides:
  - render_project (inbox ALIAS single-project view, D-04)
  - fixtures same-phase-merge, stale-fold, heuristic-labels (D-13)
  - golden inbox-alias.txt
affects: [38-03]
status: complete
commits: 2
plan_head_before: 5d54d37de99cf60a98980fde6b061b328f75f719
plan_head_after: 9eceb991d21eb0b64949099350d24a9f36abd63d
actuals:
  tokens: 20000
  tasks: 2
  commits: 2
tech-stack:
  added: []
  patterns: [one renderer over TriageEntry shared with the TUI formatter]
key-files:
  created:
    - tests/fixtures/owner/same-phase-merge/
    - tests/fixtures/owner/stale-fold/
    - tests/fixtures/owner/heuristic-labels/
    - tests/fixtures/owner_golden/inbox-alias.txt
  modified:
    - src/inbox.rs
    - src/inbox_cli.rs
    - tests/inbox_cli.rs
    - tests/owner_digest.rs
key-decisions:
  - "inbox ALIAS prints no Stale: fold line; its STALE (> N d) section lists the folded entries"
requirements-completed: [OWN-25, OWN-27, OWN-28]
---

# Phase 38 Plan 02: Single-project view Summary

`gsd-meta-manager inbox ALIAS` now prints that project's full checklist through `render_project`: totals line,
`closes after` line, lane sections over unfolded entries (a same-phase UAT + VERIFICATION pair is one entry
with `N checks · 2 files` and a `file NAME` line before each part's items), a trailing `STALE (> 90 d)` section,
then the `--include` sections and fold lines. Label suffixes `(decision?)` / `(looks resolved)` appear on item
lines. An unknown name prints the totals line and its error line, exit 0.

## Tasks

1. `render_project`, the `View::Project` arm, fixtures `same-phase-merge` and `stale-fold`, binary tests (commit 056f296b)
2. Fixture `heuristic-labels`, golden `inbox-alias.txt`, lib tests (commit 9eceb991)

## Verification

- `cargo test` all suites green (owner_digest 34, inbox_cli 30, both oracle conformance suites ran on the new
  fixtures, no SKIP); `cargo clippy --all-targets -- -D warnings` and `cargo fmt --check` clean.
- Legacy goldens `inbox.txt` and `inbox-include-all.txt` unchanged; no file under `src/ui/` touched.
- `inbox_writes_nothing_anywhere` also runs `inbox ALIAS`; the hostile tree through `inbox evil` (as written and
  as a fresh copy) carries no ESC/BEL/RLO byte and no line over 1000 chars.

## Deviations from Plan

- [Rule 3] The `entry_checklist` equals `group_checklist` test lives in `src/inbox.rs` unit tests, not
  `tests/owner_digest.rs`: both formatters are `pub(crate)`.
- Dropped the `allow(dead_code)` on `entry_checklist` (now used); `entry_line` keeps it until the TUI (38-04)
  reads it directly.
- Tasks 2's fixture directory was created during Task 1 (same generator script) and committed with Task 2.

## Inferred decisions

- The alias golden is three renders (`merge`, `old-ledger`, `labels`) separated by one blank line; 46 lines.
- Aliases `merge`, `old-ledger` (`old` is taken by a golden fixture) and `labels`.
- The single-project view prints no `Stale:` fold line (the STALE section lists the entries).
- `--lane` narrows the single-project view like `render_text` (`filters.lists_group`).
- Within one project the TEST sort's "phases remaining" key is constant, so `render_project` omits it.

## Known Stubs

None.

## Self-Check: PASSED

Fixtures, golden and SUMMARY exist; commits 056f296b and 9eceb991 are ancestors of HEAD.
