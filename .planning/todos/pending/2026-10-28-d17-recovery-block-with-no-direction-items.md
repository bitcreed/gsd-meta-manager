---
created: 2026-10-28T00:00:00.000Z
title: D-17 recovery block is unsatisfiable when DIRECTION.md parses no items (28-REVIEW WR-04)
area: driver
severity: minor
source: .planning/phases/28-sandboxed-driver-launcher/28-REVIEW.md (WR-04), deferred by quick 261028-n12 [inferred — cleanup 2026-10-28]
files:
  - src/driverctl/store.rs (blocking_recovery, recovery_rejection, the gate arm)
  - src/driverctl/protocol.rs (known_item)
  - tests/driverctl_reconcile.rs
  - tests/launcher_reconcile.rs
  - src/launcher/assets/MASTER-PROMPT.md
---

## Problem

`blocking_recovery` keeps blocking dispatch while `awaiting_recovery` is non-empty
and `direction.items.is_empty() ||` the awaiting id is still listed. When DIRECTION.md
lists no parsed items, the block holds, but the only way to clear it is a
`recovered` event for a listed item, and `known_item` accepts none. The run is stuck
with a refusal that does not say why or what to repair.

## Solution

Pick between two options:

- (a) The review's fix: drop the `items.is_empty()` clause. Rejected as the default
  because an absent DIRECTION.md reads as empty (NotFound), and a DIRECTION.md whose
  `### I-NN` headers do not parse would then let a crashed run dispatch. That loosens
  a fail-closed gate.
- (b) Keep the fail-closed block, and make both refusals (the gate arm and
  `recovery_rejection`) say "DIRECTION.md lists no items; repair it". Recommended
  [inferred — cleanup 2026-10-28].

Either way, align the `blocking_recovery` doc with the behaviour, and update the
reconcile tests and any MASTER-PROMPT text that quotes the refusal.
