---
phase: 34-owner-items-reader-inbox-cli
plan: 01
subsystem: cli
tags: [owner-inbox, audit-uat, conformance, json-schema, rust]
status: complete

requires:
  - phase: 33
    provides: parse_project_state, phase_num, disk_status, driver_frontmatter capped-read pattern
provides:
  - "owner_items reader (Layer A scan + Layer B R1/R9/R10 digest), process-free"
  - "gsd-meta-manager inbox [--json] [--lane] [--include], schema gsd-meta-manager/inbox v1"
  - "cli_targets: target resolution shared by status and inbox"
  - "oracle conformance harness and the uat-pending fixture"
affects: [34-02, 34-03, 34-04, 34-05, 34-06]

actuals:
  tokens: 37600
  tasks: 2
  commits: 2
plan_head_before: d6f0970831282f2f27b471c81cc897826cb6b993
plan_head_after: 51524409aa02c7eb5a39fc8ec298947e38c5b4a5
commits: 2

tech-stack:
  added: []
  patterns:
    - "Contained capped read: canonicalize dir, parent and file, O_NONBLOCK|O_NOFOLLOW open on the canonical path, regular-file check on the handle, take(cap+1)"
    - "Oracle harness copied from the router harness, multiset comparison, declared divergences asserted"

key-files:
  created:
    - src/state_reader/owner_items/mod.rs
    - src/state_reader/owner_items/scan.rs
    - src/state_reader/owner_items/frontmatter.rs
    - src/state_reader/owner_items/uat.rs
    - src/state_reader/owner_items/rules.rs
    - src/inbox.rs
    - src/inbox_cli.rs
    - src/cli_targets.rs
    - tests/inbox_cli.rs
    - tests/owner_audit_conformance.rs
    - tests/fixtures/owner/uat-pending/
  modified:
    - src/state_reader/mod.rs
    - src/state_reader/state_md.rs
    - src/text.rs
    - src/config.rs
    - src/lib.rs
    - src/cli.rs
    - src/main.rs
    - src/status_cli.rs

key-decisions:
  - "Current-milestone phase dirs are matched against state.phases filtered by state.in_current_milestone, not state.phases alone [inferred: upstream's window is the current milestone's roadmap]"
  - "A Fix group that carries unreadable checks has command null even when it also lists fix items [inferred: schema decision 4]"
  - "Frontmatter body gets a trailing newline before the YAML parse: serde_yml rejects a final bare `key:` otherwise"
  - "status_cli keeps its output byte-identical: Slot::Failed carries data, each command builds its own error entry"

requirements-completed: [OWN-01, OWN-02, OWN-11, OWN-13]

duration: single session
completed: 2026-11-14
---

# Phase 34 Plan 01: Inbox tracer and Layer A oracle conformance Summary

**`gsd-meta-manager inbox --json` reads the open checks of every active-phase UAT natively, prints schema v1 on one line, and the Layer A scan matches gsd-core 1.21.0 `audit-uat` on the committed fixture.**

## Accomplishments

- Task 1 (tracer, commit 6e32d0a): file to stdout for the uat-pending fixture. Layer A (`scan`): current-milestone phase dirs, legacy phase-artifact scoping, contained and capped read (symlink inside followed, escaping one absent, FIFO or directory `NotRegular`, over 1 MiB `TooLarge`, exactly 1 MiB read), byte-0 frontmatter fence, first-form UAT row parser, verbatim `categorize_item`. Layer B: R1 lanes, R9 age, R10 commands built only from validated tokens, D-20 sort, ids with `~N` repeats. `inbox` is dispatched before `init_tracing`, takes no lock and writes nothing.
- Task 2 (commit 5152440): `tests/owner_audit_conformance.rs` compares the sorted `(file, test, result, category)` multiset, parse-gap count and acknowledged count with the real oracle per fixture, follows `@file:` output, unions per-workstream runs, asserts the two declared divergences, fails without an oracle unless `GSD_META_MANAGER_ALLOW_MISSING_ORACLE` is non-blank, and scans `src/` for the oracle verb.

## Verification (raw)

- `cargo test --no-fail-fast`: 63 result lines ok, 3942 tests passed, 0 failed (full suite, includes tests/status_cli.rs unchanged: 23 passed).
- `--test inbox_cli`: 11 passed. `--lib owner_items`: 43 passed. `--test owner_audit_conformance`: 3 passed, "1 compared, 0 declared divergences, over 1 fixtures".
- `cargo clippy --all-targets -- -D warnings`, `--target x86_64-pc-windows-gnu`, `--target aarch64-apple-darwin`: clean.
- Missing oracle (empty `CLAUDE_CONFIG_DIR`, `CODEX_HOME` unset): conformance test fails with the install hint; with `GSD_META_MANAGER_ALLOW_MISSING_ORACLE=1` it passes.
- Mutation check: changing `DeviceNeeded` to `Blocked` in `categorize_item` makes the comparison fail with a diff naming `uat-pending` (reverted).

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] serde_yml rejects a frontmatter body ending in a bare `key:`**
- **Found during:** Task 1 unit tests
- **Issue:** the fence body excludes its final newline and `list: [a]\nnone:` fails to parse.
- **Fix:** append `\n` before parsing (and before the repair retry).
- **Files modified:** src/state_reader/owner_items/frontmatter.rs

**2. [Rule 3 - Blocking] identity-alphabet guard in text.rs**
- **Found during:** full lib run
- **Issue:** `exactly_one_executable_spelling_of_the_identity_alphabet_exists_under_src` flagged two new spellings of `[A-Za-z0-9._-]` (workstream name check, separator list).
- **Fix:** `is_workstream_name` delegates to `text::is_identity_char`; the phase-artifact separators are string literals.

### Plan wording not literally followed

- The Task 2 acceptance line "changing uat-pending row 2 to `issue` in a scratch copy makes the comparison fail" cannot hold as written: the oracle and our port read the same mutated copy and agree. The same intent (the harness detects a parity break) was checked with a code mutation instead (above).
- `owner_items::scan` filters phase directories through `state.in_current_milestone` in addition to `state.phases` (inferred, see key-decisions).

## Known Stubs

None. `closes_phase`, `bookkeeping_only` and `supersedes` carry `false`, `false`, `[]` by plan (34-04 computes them); `summary.decide` is 0 until Phase 35; `acknowledged` is always `false` until 34-06 adds R4. These are documented seams, not stubs that block this plan's goal.

## Notes for later plans

- `age_days` depends on the wall clock; tests must not pin it.
- `uat.rs` is the first form: no fence awareness (34-06), no `## Gaps` parsing or resolved-gap skip (34-02).
- `frontmatter.rs` exposes `extract`, `fm_str`, `source`, `updated`; 34-06 adds the rest.

## Self-Check: PASSED

- Files exist: src/state_reader/owner_items/{mod,scan,frontmatter,uat,rules}.rs, src/inbox.rs, src/inbox_cli.rs, src/cli_targets.rs, tests/inbox_cli.rs, tests/owner_audit_conformance.rs, tests/fixtures/owner/uat-pending/oracle.expect.
- Commits 6e32d0a and 5152440 are ancestors of HEAD.
