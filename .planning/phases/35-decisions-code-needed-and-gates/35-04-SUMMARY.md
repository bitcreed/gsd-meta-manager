---
phase: 35-decisions-code-needed-and-gates
plan: 04
subsystem: owner-inbox
tags: [owner-items, closes-after, milestone-gate, verification, why-human, inbox, status]
requires:
  - phase: 35-01..35-03
    provides: decide/fix signal groups, handoffs, checkpoints, open-signal scan
provides:
  - "owner_items::gate::closes_after (R8 milestone gate) and ClosesAfter / LanePhases"
  - "projects[].milestone.closes_after (inbox), projects[].owner.closes_after (status), plain `closes after:` line"
  - "EntryFields on AuditItem (Layer A extra), OwnerItem.why_human, items[].why_human"
  - "owner-facing VERIFICATION names (test text), Layer A names unchanged"
affects: []
tech-stack:
  added: []
  patterns: ["Layer A extra field never compared with the oracle", "shared closes_after_json for both documents"]
key-files:
  created:
    - src/state_reader/owner_items/gate.rs
    - tests/fixtures/owner/closes-after-milestone/
    - tests/fixtures/owner/verification-human-mapping/
  modified:
    - src/state_reader/owner_items/{mod,rules,verification,uat,deferred}.rs
    - src/inbox.rs
    - src/status_cli.rs
    - src/agent_skill.md
    - README.md
    - tests/{inbox_cli,status_cli,owner_digest,owner_audit_conformance}.rs
    - tests/fixtures/owner_golden/{inbox,inbox-include-all}.txt
key-decisions:
  - "A checkpoint counts 1 in closes_after.decide (as in the digest total), not its item count ([INFERRED], consistency with summary.decide)"
  - "Plain line prints only lanes with work, in the order test, decide, unblock, fix, each with the phases that hold its work; `no owner work (ph ...)` when all four are zero"
  - "Display name order test, truth, name, description for human entries; a VERIFICATION gap skips test ([INFERRED] D-14)"
  - "Plain checklist prints only the clean name; expected and why_human stay in JSON ([INFERRED] D-15)"
requirements-completed: [OWN-08, OWN-10]
duration: one session
completed: 2026-11-14
status: complete
actuals:
  tokens: 40000
  tasks: 3
  commits: 3
plan_head_before: ed1935f38a63a80b69df994412300561603751b0
plan_head_after: c967287893b28f55c198b77fc967c3bdeb54c33a
---

# Phase 35 Plan 04: Milestone gate, VERIFICATION names, phase close-out Summary

Every read project now reports which open current-milestone phases close after which owner work (`closes_after`, in `inbox --json`, `status --json` and a plain `closes after:` line), VERIFICATION `human_verification` entries are named by their test text with `expected` and `why_human` as separate fields, and the new signals are proven escaped and cheap.

## Tasks

| Task | Commit | Result |
|------|--------|--------|
| 1 R8 gate | e56f3bc6 | `gate.rs`, `ClosesAfter`, both JSON documents, plain line, README/skill/tests append-only, fixture `closes-after-milestone`, goldens regenerated (added lines only) |
| 2 VERIFICATION names | f7f7b6cb | `EntryFields`, `AuditItem.fields`, `OwnerItem.why_human`, `display_name`, fixture `verification-human-mapping`, explicit oracle name comparison |
| 3 close-out | c9672878 | hostile test covers debug trigger, HANDOFF, CONTEXT question and mapping `test`/`why_human`; cold 20-project tree carries debug, CONTEXT, HANDOFF |

## Verification (raw)

- `cargo test --locked --no-fail-fast` (whole crate): 4164 passed, 0 failed, 19 ignored.
- `cargo clippy --locked --all-targets -- -D warnings`: clean for host, `x86_64-pc-windows-gnu` and `aarch64-apple-darwin`.
- `owner_audit_conformance` 3 passed (both new fixtures `same`, D-16a name comparison for `verification-human-mapping` equal); `owner_digest` 26; `inbox_cli` 25; `status_cli` 29.
- Golden diff: only added lines (6 `closes after:` lines and one blank line per golden), 0 removed.
- SC4: `git diff -U0 977571d -- tests/inbox_cli.rs tests/status_cli.rs` removes 0 `"projects` / `"summary` key-path strings; SCHEMA_PATHS length 58.
- Container rehearsal `./scripts/pre-tag-check.sh --container`:

```
gate 1  tag / Cargo.toml version   SKIPPED     (no tag argument)
gate 2  MSRV check                 PASS
gate 3  cargo build --release      PASS
gate 4  cargo test                 PASS
gate 5  cargo clippy -D warnings   PASS
gate 6  cargo package --locked     PASS
gate 7  cross-target check+clippy  PASS        (3 targets)
```

## Live read-only checks (2026-11-14, built binary, nothing written)

- sentriq + usbee `inbox --json --lane decide`: sentriq 5 `debug` groups (baseline-purge-reruns-on-reprof, crank-missed-in-standby, faults-disconnected-live-dash, polling-rpm-skips-hung-stop, towing-drive-splits-18min); usbee 1 `handoff` group, `stale: true`, 6 items.
- wordoclock `milestone.closes_after`: `{test:12, decide:0, fix:1, blocked:5, phases_open:["20".."25"], not_built:[]}` (matches RESEARCH).
- wordoclock `status --json` `owner.closes_after`: identical object; `owner.decide` is 2 (two awaiting debug sessions, root signals that never count in the gate).
- mailbot `inbox --json --include archived`: context groups for 03 and 04 only, none for 06-CONTEXT.
- Plain: `wordoclock · v2.2 closes after: test 12 (ph 21 22 24 25) · unblock 5 (ph 20 23) · fix 1 (ph 25)`.

## Deviations from Plan

**1. [Rule 1 - Consistency] A checkpoint counts 1 in `closes_after.decide`.** The plan says Decide groups count items; an unreadable checkpoint lists none and the digest total counts it as 1, so the gate does the same to keep `closes_after.decide <= summary.decide`.

**2. [Rule 3 - Blocking] README milestone rows landed in the status table first.** A first-match replace put the `milestone.closes_after` rows under the `status` milestone table; moved to the inbox table before the commit (the table-coverage test caught it).

No other deviation: plan executed as written.

## Inferred decisions

D-12 workstream scopes are not exposed in `closes_after` (root only); A7 fix = items + `counts.unreadable`; A11 `status owner.closes_after`; D-14 gap names skip `test`; D-15 plain mode prints the clean name only; the plain `closes after:` line follows the §3.2 header shape with `decide` between `test` and `unblock`.

## Known Stubs

None.

## Threat Flags

None beyond the plan's register: T-35-16/17 (hostile test covers every new kind in JSON and plain), T-35-18 (cold 20-project test with the new signals passes), T-35-19 (phase strings through `terminal_capped`, plain line escaped as a whole), T-35-20 (only read-only binary runs against real projects).

## Self-Check: PASSED

gate.rs and both fixture directories exist; commits e56f3bc6, f7f7b6cb, c9672878 are ancestors of HEAD.
