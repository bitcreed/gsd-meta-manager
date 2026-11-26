---
phase: 34-owner-items-reader-inbox-cli
plan: 02
subsystem: cli
tags: [owner-inbox, uat, gaps, conformance, rust]
status: complete

requires:
  - phase: 34-01
    provides: uat.rs first form, oracle harness, uat-pending fixture
  - phase: 34-06
    provides: markdown.rs helpers, fence-aware row detection
provides:
  - "uat.rs parity with parseUatItemsWithStats: parse-gap tallies, clipBlockAtFirstFence, resolved-issue skip"
  - "## Gaps bullets (Origin::Gap) and tables (Origin::GapTable), the input to R3 (OWN-04)"
  - "five oracle-compared fixtures"
affects: [34-03, 34-04, 34-05]

actuals:
  tokens: 21000
  tasks: 2
  commits: 2
plan_head_before: 55b2590722cb953d0a1af548637e95fecb65d914
plan_head_after: f4ac331454b113960fbc121d7556ba5c69ab99e6
commits: 2

key-files:
  created:
    - tests/fixtures/owner/uat-row-shapes/
    - tests/fixtures/owner/uat-parse-gaps/
    - tests/fixtures/owner/uat-testing-quoted/
    - tests/fixtures/owner/uat-gaps-merge/
    - tests/fixtures/owner/uat-gaps-table/
  modified:
    - src/state_reader/owner_items/uat.rs

key-decisions:
  - "Gaps table rows are skipped only on a `resolved` status cell (or, with no status column, any cell reading `resolved`); the plan text also named done/pass, but gsd-core 1.21.0 reads only `resolved` for Gaps tables (done/pass belongs to deferred tables). The oracle decides [inferred]"
  - "read_gaps reads the Gaps section once per file and yields both items and the resolved-test set, so the resolved-issue skip is O(1) per row"
  - "Gap and table test numbers are stored with leading zeros dropped, as parseInt, matching parse_row_text"

requirements-completed: [OWN-01, OWN-04, OWN-11]

completed: 2026-11-14
---

# Phase 34 Plan 02: UAT parser parity (rows and Gaps)

**UAT rows and `## Gaps` entries now read exactly as gsd-core 1.21.0 reads them: a file the parser cannot read is a parse gap, resolved issues vanish, and the 10-issue / 14-gap fixture yields the 24 items R3 needs.**

## Accomplishments

- Task 1 (eff8fc8): parse-gap tallies (fence shortfall, indented rows not owned by a block-scalar header, unterminated fence, rows without a result), `clipBlockAtFirstFence` + `dropTopLevelFencedRegions` for `expected`, and fixtures `uat-row-shapes` (20 rows over every category route, multi-byte heading), `uat-parse-gaps`, `uat-testing-quoted` (D-04).
- Task 2 (code landed with eff8fc8, fixtures in f4ac331): `## Gaps` bullet splitter (hyphen grammar, separator skip, base indent), field lines (`**Key:**` unwrap, first wins, one quote pair), table union with alias headers, `isTestIssueResolvedInGaps`. Fixtures `uat-gaps-merge` (10 issue rows + 14 failed gaps = 24 items, plus one resolved gap) and `uat-gaps-table`.
- 11 new unit tests including the multi-byte row/gap case (OWN-11).

## Verification (raw)

- `--lib owner_items`: 84 passed, 0 failed (`owner_items::uat`: 27 passed).
- `--test owner_audit_conformance`: "10 compared, 0 declared divergences, over 10 fixtures".
- Oracle checked by hand on every new fixture first (24 items for uat-gaps-merge, parse_gap true for uat-parse-gaps, 4 items for uat-gaps-table).
- `cargo clippy --all-targets -- -D warnings`: clean; also `--target x86_64-pc-windows-gnu` and `--target aarch64-apple-darwin`.
- Full `cargo test --no-fail-fast`: 3984 passed, 1 failed on the first run, 3209/1 in lib on the second: `multiplexer::tmux::...focus_without_an_absolute_socket_addresses_no_server_at_all` hit "Text file busy (os error 26)" under parallel load. It passes alone (12 passed) and is unrelated to this plan (pre-existing flake, out of scope).

## Deviations from Plan

- Plan 34-02 expected the Gaps table to skip done/pass cells; upstream does not (see decisions). No fixture row depends on it beyond `resolved`.
- The two tasks share one source file, so the code is in the first commit; the second commit carries the Gaps fixtures only.
- The parse-gap tally is compared by the oracle only as a boolean (`parse_gap`); exact tallies are pinned by unit tests derived from upstream's arithmetic (shortfall = shaped lines minus tokenized rows, when positive).

## Known Stubs

None.

## Self-Check: PASSED

- Fixture directories and uat.rs exist; commits eff8fc8 and f4ac331 are ancestors of HEAD.
