---
phase: 35-decisions-code-needed-and-gates
plan: 02
subsystem: owner-inbox
tags: [owner-items, context-questions, inbox, audit-open-oracle]
requires:
  - phase: 35-01
    provides: OpenScan/scan_open, ParseCache<K, V>, signal_item, the decide counting rule, owner_open_conformance harness
provides:
  - "owner_items::context (Layer A port of deriveOpenQuestions, sanitizeForDisplay, questions digest, scanContextQuestions targets)"
  - "ContextFile, OpenScan.context_files, ArchivedCounts.decide, summary.archived.decide"
  - "Layer B is_answered and context groups (Decide, or Old debt when archived)"
  - "context half of tests/owner_open_conformance.rs, fixture context-questions"
affects: [35-03, 35-04]
tech-stack:
  added: []
  patterns: ["Layer A port pinned per oracle run (root, then each workstream)", "explicit-marker-only answered filter"]
key-files:
  created:
    - src/state_reader/owner_items/context.rs
    - tests/fixtures/owner/context-questions/
  modified:
    - src/state_reader/owner_items/mod.rs
    - src/state_reader/owner_items/scan.rs
    - src/state_reader/owner_items/rules.rs
    - src/inbox.rs
    - README.md
    - tests/owner_open_conformance.rs
    - tests/owner_digest.rs
    - tests/inbox_cli.rs
key-decisions:
  - "A second static CONTEXT_CACHE (ParseCache<OpenKind, ParsedContext>) instead of widening OPEN_CACHE's value type; evict_project clears all three caches"
  - "In a workstream oracle run only that workstream's own archive is visible, under its bare label; the root run sees every archive with the ws/ prefix. The harness compares exactly that"
  - "Archived question groups name the phase by its directory slug, never by a current roadmap entry that happens to share the number"
  - "An unreadable CONTEXT file is a Decide group with counts.unreadable 1 and no items; the plain row reads `unreadable file <name>`"
requirements-completed: [OWN-10]
duration: one session
completed: 2026-11-14
status: complete
actuals:
  tokens: 14500
  tasks: 2
  commits: 2
plan_head_before: 27ac582690d9ae99a4e9184d63f6738486f41c22
plan_head_after: b7ebacd21bde0ce0476950c3cd29855fd0866f49
---

# Phase 35 Plan 02: CONTEXT open questions as Decide work Summary

Open CONTEXT questions now travel through a process-free, oracle-pinned Layer A port of gsd-core 1.21.0's `deriveOpenQuestions` / `scanContextQuestions` (UTF-16 length-prefixed acknowledgment digest included) and an owner rule that drops only explicitly RESOLVED / ANSWERED / CLOSED questions: current-milestone ones are Decide items, archived ones Old debt counted in the new `summary.archived.decide`.

## Tasks

| Task | Commit | Result |
|------|--------|--------|
| 1 Layer A and oracle parity | 7bdb0ccb | `context.rs` (derive, sanitize, digest, targets), `ContextFile`/`context_files`, shared `archived_targets` in `scan.rs`, per-run context comparison in `owner_open_conformance`, fixture `context-questions` |
| 2 Layer B rules and schema | b7ebacd2 | `is_answered`, context groups in `rules.rs`, `ArchivedCounts.decide`, `summary.archived.decide` (inbox, README fence and table, `inbox_cli` key set and `INBOX_PATHS`), DECIDE rows `<N> open questions in <file>`, `context_questions_fill_decide` |

## Verification (raw)

- `cargo test --no-fail-fast` (whole crate): 4121 passed, 0 failed, 19 ignored.
- `cargo clippy --all-targets -- -D warnings`: clean.
- `owner_open_conformance` 3 passed (3 fixtures, root run plus ws1 run for context-questions, equal to the real oracle); `owner_audit_conformance` 3 passed; `owner_digest` 18, `inbox_cli` 23, `status_cli` 28 passed; lib `owner_items` 180 passed (16 in `context`).
- Oracle on the fixture, root run: 01 x1, 02 x6, 03 x2, 04 x3, archived v0.9 00 x1, acknowledged 1; ws1 run: 01 x1. Digest of `["- Is Y ready?"]` is `f9473be7...4775d`; the emoji digest in the unit test and fixture were computed with node.
- Goldens untouched (no diff under `tests/fixtures/owner_golden/`); the only removed line in `tests/inbox_cli.rs` is the old `summary.archived` key set (no key-path string removed).
- Read-only check on `~/projects/python/mailbot` (`inbox --json --include archived`): no group for 06-CONTEXT, 04-CONTEXT lists 8 (as RESEARCH F3), 03-CONTEXT lists 11 (RESEARCH said 9). The 2 extra are continuation lines of wrapped questions that contain a `?` (`invocation one thread...?`, `reachable as an MCP resource? ...`): upstream's line filter keeps them, so Layer A parity keeps them too; the project's file has also changed since the research. Not fixed (Layer A must equal the oracle).

## Deviations from Plan

**1. [Rule 3 - Blocking] Separate `CONTEXT_CACHE`.** `OPEN_CACHE`'s value type is `debug::ParsedSession`; a second static avoids an enum wrapper in code 35-01 shipped. `OpenKind::Context` is now used (the `allow(dead_code)` is gone).

**2. [Rule 1 - Plan gap] Per-run archive visibility.** The plan said "every archived file" for each run; the oracle shows a workstream run sees only its own archive (bare label) and no root archive. The harness encodes the measured rule.

**3. Shared helper.** `scan::archived_targets` (the archive half of `targets`) and `Target`/`subdirectories` made `pub(super)`; `targets` calls it, UAT enumeration unchanged (`owner_audit_conformance` green).

**4.** `cargo fmt` reflowed `src/state_reader/disk_status.rs` (unrelated); reverted, not committed.

## Inferred decisions

F3 label matcher with the digit requirement, F4 (every active dir is Layer A, only current-milestone ones listed), answered filter on frontmatter strings too, ids by Layer A position, no command on question groups, `archived.decide` in the Old-debt fold, an acknowledged file counts in `acknowledged_files` only when it yields a group, a question emptied by sanitizing stays listed with an empty name (fail-safe, as upstream keeps it in the count). Known limit: a bracketed tag such as `(closed beta)` anywhere in a line reads as an answered marker, as the plan's rule states.

## Known Stubs

None.

## Threat Flags

None beyond the register. T-35-07: `sanitize_for_display` ported and unit-tested per rule (plan 35-04 adds the hostile case); T-35-08: `contained` + NOFOLLOW open, symlink-escape unit test; T-35-09: 1 MiB cap, `CONTEXT_CACHE`; T-35-10: acknowledgment needs the exact digest over the full list. No process API in `src/` (`no_source_file_names_the_open_audit_verb` passes).

## Self-Check: PASSED

`context.rs` and the fixture exist; commits 7bdb0ccb and b7ebacd2 are ancestors of HEAD.
