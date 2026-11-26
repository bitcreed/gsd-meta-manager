# Phase 34: Owner items reader + inbox CLI - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-11-14
**Phase:** 34-owner-items-reader-inbox-cli
**Areas discussed:** `testing` status and outstanding predicate, Layer A parity and oracle, fixtures,
Layer B details, digest placement and cache, safety, inbox CLI shape and ordering, status/skill

Mode: `--auto` (unattended). Every choice below is the recommended option, decided by the
orchestrator from the report, REQUIREMENTS.md, ROADMAP.md and a code scout.

---

## `testing` status and the outstanding predicate

| Option | Description | Selected |
|--------|-------------|----------|
| Split predicates | Router keeps upstream's blocking set; status/owner use upstream ∪ testing ∪ non-terminal-with-open-items | ✓ |
| Add `testing` to `is_outstanding()` | One predicate; the router then diverges from gsd-core's `BLOCKING_UAT_FM_STATUSES` | |
| Status-only fix | Patch only `status_cli`, leave `from_raw` | |

**Notes:** `router.rs` G7 transcribes upstream and is guarded by the router conformance test.

## Layer A parity and oracle

| Option | Description | Selected |
|--------|-------------|----------|
| New conformance file, same opt-out variable | `tests/owner_audit_conformance.rs`, `GSD_META_MANAGER_ALLOW_MISSING_ORACLE` | ✓ |
| New opt-out variable | Separate env var per oracle | |
| Extend the router conformance file | One file for both oracles | |

**Notes:** Source scan limited to `src/**/*.rs` non-comment lines, since `agent_skill.md` must
mention `audit-uat` in prose.

## Fixtures

| Option | Description | Selected |
|--------|-------------|----------|
| Synthetic, modeled on real cases | Same structure and counts, neutral text | ✓ |
| Copies of real project files | Exact, but leaks private project prose into a crates.io package | |

## Digest placement and cache

| Option | Description | Selected |
|--------|-------------|----------|
| Separate entry point, optional field on `ProjectState` | Driver, `next`, router pay nothing | ✓ |
| Inside `parse_project_state` (report §4 as written) | Every parse scans archives | |

| Option | Description | Selected |
|--------|-------------|----------|
| (path, mtime, len) map in `owner_items`, implicit + watcher eviction | As OWN-12 | ✓ |

## Age (R9)

| Option | Description | Selected |
|--------|-------------|----------|
| `updated:` then mtime; git only if batched | Keeps the < 1 s budget | ✓ |
| `updated:`, git per file, mtime | Report order; one spawn per file | |

## Safety

| Option | Description | Selected |
|--------|-------------|----------|
| Oversize file → parse gap ("unreadable checks") | Never reads as clean | ✓ |
| Oversize file → absent | Silent | |

## Inbox CLI

| Option | Description | Selected |
|--------|-------------|----------|
| Share `status_cli` target code by factoring | One resolver | ✓ |
| Copy the resolver | Two to keep in sync | |

| Option | Description | Selected |
|--------|-------------|----------|
| Phase-35 keys emitted now as zeros; `closes_after` omitted until 35 | Purely additive later | ✓ |
| Omit all Phase-35 keys | More additions later | |

| Option | Description | Selected |
|--------|-------------|----------|
| Comma-separated `--lane` / `--include` | clap `value_delimiter` | ✓ |
| Single-value flags | | |

| Option | Description | Selected |
|--------|-------------|----------|
| TEST order: closes-a-phase, milestone progress, age oldest first | Report §3.3, oldest-first inferred | ✓ |

## Todos cross-reference

| Option | Description | Selected |
|--------|-------------|----------|
| Fold the quoted-status todo only | Same `status:` values the reader parses | ✓ |
| Fold all score ≥ 0.4 matches | Keyword noise (UI, driverctl); scope creep | |

---

## Claude's Discretion

Type layout, module split, helper names, plan count and grouping, cache granularity.

## Deferred Ideas

Router G7 adopting `testing` if upstream does; batched git age; TUI fill of `ProjectState.owner`
(Phase 36).
