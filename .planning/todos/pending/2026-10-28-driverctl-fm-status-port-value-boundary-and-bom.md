---
created: 2026-10-28T00:00:00.000Z
title: The driverctl fm_status port still has the IN-06 shapes (no value boundary, line-crossing whitespace, BOM -> None)
area: driver
severity: minor
source: .planning/phases/26.1-driver-validation-spike-003-re-run/26.1-REVIEW.md (IN-06), via quick 261028-n0y
files:
  - src/driverctl/counters.rs:138-157
  - src/driverctl/scan.rs:428-439
  - .planning/spikes/003-divergent-gap-loop-validation/tools/gapcount.py:63
---

## Problem

- The live gap-cap counter's `STATUS_RE` in `src/driverctl/counters.rs` still has the shapes IN-06 removed from
  `gapcount.py`. Rust `\s` crosses a newline, and there is no value boundary, so `status: gaps_found2` counts
  as a gap round and an empty `status:` reads the next line's key.
- Its `fm_status` returns `None` for a UTF-8 BOM-prefixed file, so such a `gaps_found` write is NOT counted by
  the launcher cap. `scan.rs` pins that on purpose (`fm_status_rejects_bom_prefixed_text`, D-27).
- Since quick 261028-n0y the spike definition (`gapcount.py`) and the Rust port disagree on these three shapes.
  The acceptance harnesses use drift.py's status only for logging, never to compute a verdict, so no verdict
  depends on the gap.

## Solution direction

- The `regex` crate has no lookaround. Use `[ \t]*` after the colon, capture `[\w-]+` after the optional quote,
  and accept the capture only when it fully matches `[A-Za-z_]+`. Strip one leading U+FEFF before the `---`
  check.
- Flip `fm_status_rejects_bom_prefixed_text` to the new expectation, and record that as a decision, since it
  reverses a D-27 pin.
- Confirm the 26.1 fixtures under `tests/fixtures/driverctl/spike003/` replay unchanged. Do this before or with
  Phase 27's re-verification.

Deferred `[inferred — cleanup 2026-10-28]`: product cap behaviour belongs to a Phase 27 change, not to a
spike-harness quick task.
