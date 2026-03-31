# Phase 10: Tech Debt Cleanup - Discussion Log

> **Audit trail only.** Do not use as input to planning, research, or execution agents.
> Decisions are captured in CONTEXT.md — this log preserves the alternatives considered.

**Date:** 2026-03-31
**Phase:** 10-tech-debt-cleanup
**Areas discussed:** Dead code strategy, Test coverage scope, Cleanup thoroughness

---

## Gray Area Selection

| Option | Description | Selected |
|--------|-------------|----------|
| Dead code strategy | 7 #[allow(dead_code)] items: remove unused code vs keep for archive browser? | ✓ |
| Test coverage scope | Tests pass but limited coverage. Add more or just fix broken? | ✓ |
| Cleanup thoroughness | Just dead_code or also clippy, formatting, doc coverage? | ✓ |

**User's choice:** "you decide - thorough cleanup" — delegated all decisions to Claude
**Notes:** User wants comprehensive cleanup, not minimal. All three areas handled via Claude's discretion.

---

## Claude's Discretion

All gray areas delegated to Claude with "thorough cleanup" directive:
- Dead code: remove genuinely unused, wire up structs needed for Phase 12
- Tests: ensure existing pass, no new coverage requirements
- Thoroughness: clippy + fmt + dead_code — full cleanup

## Deferred Ideas

None
