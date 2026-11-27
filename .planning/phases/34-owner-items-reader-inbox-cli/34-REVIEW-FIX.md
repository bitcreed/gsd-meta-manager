---
phase: 34-owner-items-reader-inbox-cli
fixed_at: 2026-11-27T00:00:00Z
review_path: .planning/phases/34-owner-items-reader-inbox-cli/34-REVIEW.md
iteration: 1
findings_in_scope: 2
fixed: 2
skipped: 0
status: all_fixed
---

# Phase 34: Code Review Fix Report

**Fixed at:** 2026-11-27
**Source review:** .planning/phases/34-owner-items-reader-inbox-cli/34-REVIEW.md
**Iteration:** 1

This report replaces the one from the earlier review round. WR-01 and WR-02 below are the current review's findings.

**Summary:**
- Findings in scope: 2 (Critical and Warning; the two Info findings are out of scope)
- Fixed: 2
- Skipped: 0

Verification ran in the main checkout (`workflow.use_worktrees` was not used; no worktree).

## Fixed Issues

### WR-01: A VERIFICATION with an unterminated or non-mapping frontmatter block still reads as clean

**Files modified:** `src/state_reader/owner_items/frontmatter.rs`, `src/state_reader/owner_items/scan.rs`
**Commit:** 765b137f
**Status:** fixed: requires human verification (logic change in a reader that mirrors upstream)
**Applied fix:** Added `frontmatter::extract_strict`. It returns `Err(Unparseable)` for a block that opens but never closes, and for a closed block whose YAML is a scalar or sequence. Only a text with no fence at all, or an empty or comment-only block (YAML null), stays `Ok(empty)`. `scan::parse_document` uses it for VERIFICATION files, so those files become `Unreadable::Io` Fix entries instead of vanishing from the digest. Regression tests: `a_verification_with_an_unterminated_or_non_mapping_block_is_unreadable_not_clean` (scan.rs, failed before the change) and `strict_extraction_refuses_an_unterminated_or_non_mapping_block` (frontmatter.rs).

**Inferred decisions (audit):**
- The review suggested changing `extract_checked` itself. I added a separate strict function instead, because `extract_checked` is also used by `debug::parse_session`, where an unterminated block already has its own handling (`fence_body` is `None` for an unterminated fence). `extract` (UAT) is unchanged.
- An empty or comment-only closed block stays clean: it holds nothing that could hide a check.
- The review said to declare a divergence in the conformance oracle if upstream differs. The existing `owner_audit_conformance` and `owner_digest` tests still pass, so no divergence was declared. Whether upstream reads an unterminated VERIFICATION as `{}` was not independently checked. That is the reason for the human-verification flag.

### WR-02: The G0 `HANDOFF.json` read is an unbounded, blocking `read_to_string`

**Files modified:** `src/state_reader/mod.rs`
**Commit:** dac7d825
**Applied fix:** The G0 pause check and `detect_handoff`'s HANDOFF.json read both go through `disk_status::read_text_file` (non-blocking open, regular file only, 1 MiB cap). `detect_handoff` was migrated as well because it runs inside `parse_project_state`; fixing only the G0 read would still hang on a FIFO. Regression test: `a_handoff_json_fifo_does_not_block_the_parse_and_records_no_pause` (Linux only; it hung for 5 s before the change).

**Inferred decision (audit):** The `.continue-here.md`, STATE.md, ROADMAP.md and HANDOFF.md reads in `parse_project_state` still use plain `read_to_string`. The review proposed migrating them in a follow-up, so I left them. A FIFO at one of those paths can still park `inbox` and `status`. Migrating them would also add the 1 MiB cap to STATE.md and ROADMAP.md, which is a behaviour change worth deciding on separately.

## Verification

- `cargo clippy --all-targets -- -D warnings`: clean.
- `cargo test --no-fail-fast`: see the orchestrator summary. The only known failure is the unrelated `the_installed_verify_work_still_parses_arguments_this_way` in `tests/owner_settle_oracle.rs`, which was not touched.

---

_Fixed: 2026-11-27_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
