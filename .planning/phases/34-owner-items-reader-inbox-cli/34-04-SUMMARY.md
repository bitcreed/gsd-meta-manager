---
phase: 34-owner-items-reader-inbox-cli
plan: 04
subsystem: cli
tags: [owner-inbox, owner-rules, status-json, router-parity, skill, rust]
status: complete

requires:
  - phase: 34-01
    provides: owner_items digest, R1/R9/R10, inbox, status_cli entries
  - phase: 34-02
    provides: Gap and GapTable items (input to R3)
  - phase: 34-03
    provides: VERIFICATION, deferred and archived AuditFiles
  - phase: 34-06
    provides: R4 acknowledgment, quote-safe frontmatter, non-blocking reads
provides:
  - "UatStatus::Testing, is_upstream_blocking (router G7), is_owner_outstanding; quote-tolerant UAT and VERIFICATION status"
  - "owner rules R2, R3, R5, R6, R7 and the VERIFICATION and deferred lanes in rules.rs"
  - "OwnerDigest::has_open_uat_items"
  - "status --json projects[].owner {test, decide, fix_phases, blocked, closeable_phases}"
  - "skill text that sends agents to inbox --json"
affects: [34-05]

actuals:
  tokens: 60000
  tasks: 3
  commits: 3
plan_head_before: c9ad878e51d0c463d6b6af8e1ca1bf01c60987d6
plan_head_after: 9c64583
commits: 3

key-files:
  modified:
    - src/state_reader/disk_status.rs
    - src/driver/router.rs
    - src/state_reader/owner_items/rules.rs
    - src/status_cli.rs
    - tests/status_cli.rs
    - tests/state_reader_test.rs
    - tests/driver_router_conformance.rs
    - README.md
    - src/agent_skill.md
    - .planning/todos/completed/2026-10-18-dashboard-verification-status-quote-divergence.md

key-decisions:
  - "Deferred work (category deferred and every deferred-items.md entry) is built as groups under Lane::Deferred, not only counted: inbox.rs already hides that lane by default and `--include deferred` needs the groups [inferred]"
  - "A readable VERIFICATION in a status other than human_needed lands wholly in Fix; a human_needed file sends only human_verification items to Test, its frontmatter gap items to Fix [inferred]"
  - "R7 and bookkeeping look only at non-acknowledged current files and groups; archived and acknowledged work never closes or blocks a phase [inferred]"
  - "A bookkeeping-only group is emitted for every non-terminal (outside complete, resolved, passed) current UAT file of a phase with no open items, or for any UAT when the VERIFICATION is human_needed; one group per UAT file [inferred]"
  - "The tripwire fixture uses a passed VERIFICATION plus a testing UAT (state complete, upstream NoAction): the plan's unverified executed state parks the Rust router at a verification gate and makes upstream emit execute-phase, so it cannot isolate the UAT effect"

requirements-completed: [OWN-03, OWN-04, OWN-06, OWN-07, OWN-09, OWN-15]

completed: 2026-11-14
---

# Phase 34 Plan 04: Owner rules, `testing` status, `owner` counts Summary

**`status: testing` counts for the owner and never for the router, rules.rs now folds duplicates, merges gaps, separates archived and deferred work and computes closeable phases, and `status --json` carries additive `owner` counts that the skill points agents to `inbox --json` for.**

## Accomplishments

- Task 1 (3b12d20): `UatStatus::Testing`; the old predicate is renamed `is_upstream_blocking` (router G7, set unchanged) and `is_owner_outstanding` adds `testing`. Both `from_raw` strip one quote pair (D-04); the folded todo moved to `completed/`. Router tripwire fixture `uat_testing_does_not_gate`.
- Task 2 (b7f550a): R2 supersede (exact first-token basename, same scope, workstream and phase, human_needed only), R3 gap merge into `issue` rows, R5 archived counts, R6 deferred lane, R7 closeable phases and bookkeeping-only groups, VERIFICATION lanes, `has_open_uat_items`. `r7_closeable_table` pins the eight combinations; 12 further rules tests.
- Task 3 (9c64583): `owner` object on every readable entry and `owner: null` on errors, owner-facing `uat_outstanding` (owner status, or open items in a non-terminal UAT), six README rows, fence update, skill gains the inbox command, owner bullet, the do-not-run-audit-uat sentence and a jq recipe (106 lines). Tests: SCHEMA_PATHS 50, twelve entry keys, four new status tests.

## Verification (raw)

- `cargo test --no-fail-fast` (whole suite, oracle present): 4036 passed, 0 failed.
- `--lib owner_items::rules`: 23 passed. `--lib disk_status`: 125 passed. `--test status_cli`: 27 passed. `--test driver_router_conformance`: 8 passed.
- `cargo clippy --all-targets -- -D warnings`: clean on the host, `x86_64-pc-windows-gnu` and `aarch64-apple-darwin`.

## Deviations from Plan

### Plan wording not literally followed

- Tripwire fixture state: the plan said "planned and executed" with a testing UAT. That state (no VERIFICATION) makes upstream emit `/gsd-execute-phase` while the Rust router parks, an existing divergence unrelated to UAT. The fixture uses a passed VERIFICATION (state complete, upstream NoAction), where the UAT status is the only variable. Upstream arm was taken from the oracle run.
- The plan's final manual check (`status --json` on the owner's registry) was not run: real projects are read-only and out of scope for this unattended run; the same output is covered by `tests/status_cli.rs`.
- Task 1 and 2 are `tdd="true"`; tests were written with the code rather than as separate RED commits (no RED commits exist).

### Auto-fixed Issues

None beyond the above.

## Known Stubs

None. `owner.decide` is 0 by plan until the decision signals land.

## Notes

- Known flake (not seen this run): `multiplexer::tmux::...focus_without_an_absolute_socket_addresses_no_server_at_all` ("Text file busy").
- Plan 34-05 owns the inbox README section and fixture-level numbers (18 checks, 10 + 14, 200 deferred) over the merged Layer A.

## Self-Check: PASSED

- Files exist: rules.rs, status_cli.rs, completed todo, this SUMMARY.
- Commits 3b12d20, b7f550a, 9c64583 are ancestors of HEAD.
