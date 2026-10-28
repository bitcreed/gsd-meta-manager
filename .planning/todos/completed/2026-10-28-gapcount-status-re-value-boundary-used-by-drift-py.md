---
created: 2026-10-28T00:00:00.000Z
title: gapcount STATUS_RE has no value boundary and crosses lines; drift.py and the acceptance scripts still use it (26.1-REVIEW IN-06, the live drift.py status finding)
area: scripts
severity: minor
source: .planning/phases/26.1-driver-validation-spike-003-re-run/26.1-REVIEW.md
files:
  - .planning/spikes/003-divergent-gap-loop-validation/tools/gapcount.py:63
  - .planning/spikes/003-divergent-gap-loop-validation/tools/drift.py:453
  - scripts/launcher-acceptance.sh:900
  - scripts/prompt-acceptance.sh:1122
completed: 2026-10-28
status: completed
resolved_by: quick 261028-n0y (ee4d054, 61fd1bc), batch 261028-n0u; Rust port residue in todos/pending/2026-10-28-driverctl-fm-status-port-value-boundary-and-bom.md
---

## Problem

Phase 26.1's spike harness is otherwise retired, but `tools/drift.py` is still launched by
`scripts/launcher-acceptance.sh` (:900, :902) and `scripts/prompt-acceptance.sh` (:1122). drift.py reads a
VERIFICATION snapshot's `status:` through `gapcount.fm_status` (WR-03 made that the one quote-tolerant
definition). That regex, `^status:\s*["']?([A-Za-z_]+)` (gapcount.py:63), has two defects (26.1-REVIEW IN-06):

- no value boundary: `status: gaps_found2` reads as `gaps_found`;
- `\s*` crosses a newline: an empty `status:` followed by `phase: x` reads as `phase`;
- a UTF-8 BOM before `---` reads as no frontmatter.

**ID note** [inferred — cleanup 2026-10-28]: the 2026-10-28 triage named this "WR-11 (drift.py accepts a quoted `status:`)". In
26.1-REVIEW.md, WR-11 is the campaign-budget relaunch reservation (spike-only), and "drift.py accepts a quoted
`status:`" is WR-03, already fixed in 364a493. The still-live residue of that finding in a path the scripts
use is IN-06, so IN-06 is what this todo tracks. IN-09 (drift.py verifier-spawn counting trusts the
tool_use block; drift.py:333-336, 402-412) is in the same file and the same `--trigger verifier` path the
scripts use; check it while you are in there.

## Solution

`STATUS_RE = re.compile(r"""^status:[ \t]*["']?([A-Za-z_]+)(?![\w-])""", re.M)`, decode with `utf-8-sig`,
update the pinned `assert STATUS_RE.pattern == ...` selftest line, and add a selftest case per shape. Then run
`tools/drift.py selftest` and both acceptance scripts' selftest modes.

## Resolution

- IN-06 fixed as specified (ee4d054): bounded value, single-line whitespace class, UTF-8 BOM accepted, one selftest case per shape.
- IN-09 fixed for the generation ordering (61fd1bc): gen advances only after the feed write lands, a failed write keeps the spawn queued. The tool_result half is not adopted: a foreground Agent call's tool_result arrives after the verifier has read the feed, and the driverctl gate exempts calls made inside a subagent.
- Closed into `completed/` rather than `done/`, because GSD's todo tooling reads `completed/` and `done/` is empty and untracked [inferred — cleanup 2026-10-28].
