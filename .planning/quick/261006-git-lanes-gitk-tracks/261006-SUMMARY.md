---
status: complete
quick_id: 261006-git-lanes
---
# Summary

- `src/state_reader/git_ops.rs`: `GitLogEntry.graph` (plain ASCII String, not Untrusted), `assign_lanes`, `--topo-order`, `%H %P` id field prefixed ahead of the unchanged 5-field tail, `--parents` when `planning_only`.
- `src/ui/screens/detail.rs`: lane column drawn before the hash, width charged against the subject budget.

## Inferred decisions (audit)
- I-1: Linear history draws NO graph column (all rows empty) so existing layouts do not shift.
- I-2: ASCII glyphs `* | \ /` only (width-safe, no sanitising needed); lanes capped at 8, `*` kept visible when clipped.
- I-3: `planning_only` uses `--parents` so lanes follow rewritten parents.
- I-4: One text row per commit (no connector rows), unlike `git log --graph`.

## Tests
lib 2010 pass / 2 fail (both pre-existing: envelope git-version witness; README compat note vs GSD_CORE_SYNCED_TREE_VERSION 1.15.0). Integration suites all green.
