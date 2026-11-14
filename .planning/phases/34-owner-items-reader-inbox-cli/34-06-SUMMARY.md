---
phase: 34-owner-items-reader-inbox-cli
plan: 06
subsystem: cli
tags: [owner-inbox, markdown, frontmatter, acknowledgment, hostile-input, rust]
status: complete

requires:
  - phase: 34-01
    provides: owner_items scan, contained capped read, uat parser first form, oracle harness
provides:
  - "owner_items::markdown: tokenize_headings, collect_section, strip_fenced_code, scan_fenced_blocks, match_fence_delimiter, split_table_row, is_delimiter_row"
  - "owner_items::frontmatter: anchor/alias refusal, failsafe_str, fm_list, flatten_object_list_item, is_acknowledged, uat_gap_snapshot"
  - "fence-aware UAT row detection"
  - "R4: acknowledged UAT files (OWN-05)"
  - "hostile-input test set (OWN-11, D-17)"
affects: [34-02, 34-03, 34-04, 34-05]

actuals:
  tokens: 31000
  tasks: 2
  commits: 2
plan_head_before: 30e99584d23dda83d0d4d1c34eb997c0e43bbbc1
plan_head_after: 1f2fddfe204bc0e2239d3a31386bf532f13d0973
commits: 2

key-files:
  created:
    - src/state_reader/owner_items/markdown.rs
    - tests/fixtures/owner/uat-ack-match/
    - tests/fixtures/owner/uat-ack-count-changed/
    - tests/fixtures/owner/uat-ack-status-changed/
    - tests/fixtures/owner/uat-ack-missing-at/
  modified:
    - src/state_reader/owner_items/{frontmatter,uat,scan,rules,mod}.rs
    - src/state_reader/disk_status.rs
    - src/inbox.rs
    - tests/inbox_cli.rs

key-decisions:
  - "markdown.rs carries a module-wide allow(dead_code): plans 34-02 and 34-03 must build on it without editing it, and some helpers have no caller until then"
  - "An acknowledged file still builds its groups (acknowledged = true) but is skipped by every count and by default listing; inbox.rs filters them unless Filters.acknowledged [inferred: plan 34-05 lists them under --include acknowledged]"
  - "Anchor/alias refusal is a conservative token scan (node-start positions, quoted scalars skipped), not a YAML parse; it may over-refuse inside a block scalar, which only empties the frontmatter"
  - "uat.rs rows read result/reason/blocked_by from the fence-stripped block (raw block when a fence is unterminated), expected from the raw block; upstream's clipBlockAtFirstFence and the shortfall tallies stay with plan 34-02"

requirements-completed: [OWN-01, OWN-05, OWN-11]

completed: 2026-11-14
---

# Phase 34 Plan 06: Markdown and frontmatter engines, acknowledgment, hostile inputs

**Row headings inside code fences are ignored as gsd-core ignores them, an acknowledged UAT stays hidden exactly while its `gap_snapshot` still matches, and every hostile file degrades to unreadable or absent without a hang.**

## Accomplishments

- Task 1 (commit 8faf918): `markdown.rs` is a line-indexed port of the sectionizer and table helpers. `frontmatter.rs` gains the anchor/alias refusal, FAILSAFE lists, `is_acknowledged` and `uat_gap_snapshot`. `uat.rs` finds rows with `tokenize_headings` over the blank-indented-fence lines. `scan.rs` sets `AuditFile.acknowledged` before parsing; `rules.rs` implements R4. Four synthetic fixtures (`uat-ack-match`, `-count-changed`, `-status-changed`, `-missing-at`) compare equal with the oracle: acknowledged_files 1, 0, 0, 0 (checked by hand with the oracle first).
- Task 2 (commit 1f2fddf): the hostile set in `scan.rs`: 1 MiB boundary, one byte over, sibling-prefix symlink, escaping symlink to a FIFO, FIFO named `01-UAT.md`, invalid UTF-8, unterminated and empty frontmatter, zero-byte file, alias bomb under 1 s.

## Verification (raw)

- `cargo test --no-fail-fast`: 3974 passed, 0 failed (whole suite).
- `--lib owner_items`: 73 passed. `--test inbox_cli`: 13 passed. `--test owner_audit_conformance`: "5 compared, 0 declared divergences, over 5 fixtures".
- `cargo clippy --all-targets -- -D warnings`, `--target x86_64-pc-windows-gnu`, `--target aarch64-apple-darwin`: clean.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] `parse_project_state` hung on a FIFO named like an artifact**
- **Found during:** Task 2, `a_fifo_uat_is_not_regular_and_returns_promptly` timed out although `owner_items::scan` itself never blocks.
- **Issue:** `disk_status.rs` read UAT, VERIFICATION, PLAN, SUMMARY, `.continue-here.md` and `waves.json` with `std::fs::read_to_string`, which blocks forever on a FIFO. `inbox` and `status` both call it first, so a hostile project could stall them (T-34-22).
- **Fix:** `disk_status::read_text_file` opens with `O_NONBLOCK`, requires a regular file, follows symlinks as before; all six production call sites use it.
- **Files modified:** src/state_reader/disk_status.rs
- **Commit:** 1f2fddf

**2. [Rule 2 - Missing critical] default listing must skip acknowledged groups**
- **Issue:** with R4 building flagged groups, `inbox.rs` would have listed them by default.
- **Fix:** `project_json` lists an acknowledged group only when `Filters.acknowledged` is set (one-line change; plan 34-05 owns the rest of the filter).
- **Files modified:** src/inbox.rs
- **Commit:** 8faf918

### Plan wording not literally followed

- `inbox.rs` and `disk_status.rs` are outside `files_modified`; both were touched only as described above.
- The one-mebibyte stat-then-read growth case is covered by the existing `take(cap + 1)` bound, not by a dedicated racing test (no deterministic way to grow a file between two syscalls).

## Notes for later plans

- Plan 34-03's parse cache must stay after containment, the regular-file check and the size check on the open handle, never before them; `a_fifo_uat_*`, `an_escaping_symlink_*`, the cap tests and `an_alias_bomb_*` must stay green.
- `markdown.rs`/`frontmatter.rs` expose everything plans 34-02 and 34-03 named (`collect_section_stopping_at` is an extra for `stopAtLevel`); neither file needs a change from them.
- `uat.rs` still lacks `## Gaps`, the shortfall tallies, `clipBlockAtFirstFence` and the resolved-gap skip (plan 34-02).

## Known Stubs

None.

## Self-Check: PASSED

- Files exist: markdown.rs, the four fixture directories, this SUMMARY.
- Commits 8faf918 and 1f2fddf are ancestors of HEAD.
