---
phase: 38-inbox-digest-and-triage-hygiene
plan: 03
subsystem: inbox-cli
tags: [rust, inbox, json, lanes, digest, readme]
requires:
  - phase: 38-01
    provides: triage model, labels, digest, collapsed_block
  - phase: 38-02
    provides: render_project, same-phase-merge, stale-fold, heuristic-labels fixtures
provides:
  - render_lanes (inbox --lane across projects, one line per phase)
  - scripted-checklist fixture (51 steps), inbox-digest and inbox-lane-test goldens
  - additive inbox --json keys (labels, folded_stale, entry_files, summary.next_actions, summary.stale_folded)
  - README and status --skill documentation
affects: []
status: complete
commits: 2
actuals:
  tokens: 120000
  tasks: 2
  commits: 2
key-files:
  created:
    - tests/fixtures/owner/scripted-checklist/
    - tests/fixtures/owner_golden/inbox-digest.txt
    - tests/fixtures/owner_golden/inbox-lane-test.txt
  modified:
    - src/inbox.rs
    - src/inbox_cli.rs
    - tests/owner_digest.rs
    - tests/inbox_cli.rs
    - README.md
    - src/agent_skill.md
requirements-completed: [OWN-24, OWN-25, OWN-26, OWN-27, OWN-28]
---

# Phase 38 Plan 03: Lane view and JSON triage Summary

`inbox --lane L` over several projects now prints one collapsed line per phase (a 51-step UAT is one line
holding `51 checks`), with stale phases folded into a `Stale:` line that counts the selected lanes only.
`inbox --json` gains only added keys: `projects[].groups[].folded_stale`, `entry_files`, `items[].labels`,
`summary.next_actions[]` and `summary.stale_folded`; `build_document` takes `fold_days`.

## Tasks

1. `render_lanes`, Lanes dispatch, `scripted-checklist` fixture, digest and lane goldens, budget and hostile
   tests (commit 33b2422e)
2. JSON additions, schema pins (`INBOX_PATHS` only gains entries), README prose, fence and 15 table rows,
   `status --skill` text, D-14 evidence (commit bebe5277)

## Verification

- Full `cargo test`: all suites green (lib 3628 tests; `owner_digest` 43, `inbox_cli` 36); clippy
  `--all-targets -- -D warnings` and `cargo fmt --check` clean. The oracle conformance suites ran on
  `scripted-checklist` with no SKIP. Legacy goldens `inbox.txt` and `inbox-include-all.txt` are byte-identical.
- One full run showed `driver::lock::tests::probe_at_reports_absent_free_and_held` failing once; it passed on
  every rerun (unrelated timing flake under load, not touched here).

## Golden sizes

`inbox-digest.txt` 24 lines (budget 15 to 25), `inbox-lane-test.txt` 15 lines.

## D-14 evidence (owner's own registry, read-only, numbers only)

- Plain `inbox` prints 25 lines (361 before this phase, per CONTEXT).
- Labels across all items: `decision?` 28, `looks resolved` 1.

## Deviations from Plan

- The three text halves that called `inbox --lane X` with no project now call `inbox --all --lane X`, as the
  plan said (bare `--lane` is the new collapsed lane view). The hostile tree was extracted into a shared
  `hostile_tree()` helper so the new test reuses it.
- A second group (same phase VERIFICATION) was added to the README schema fence so `entry_files` shows both files.

## Inferred decisions

- Groups outside the owner rows (archived, deferred, acknowledged) are listed in the lane view as single
  collapsed entries; the `TODOS` section there is one line per project (no item lines).
- `entry_files` falls back to the group's own basename if a group has no triage entry (cannot occur today).

## Known Stubs

None.

## Self-Check: PASSED
