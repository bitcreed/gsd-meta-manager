---
phase: 26-driver-validation-spike-003
plan: 01
subsystem: spike-tooling
tags: [spike-003, driver-validation, drift-feed, launcher, haiku-smoke]
requires: []
provides:
  - tools/drift.py (host-fed moving-target feed, 9 generations, SUMMARY-commit watcher, selftest)
  - experiment/setup-scratch.sh and experiment/run.sh (modes l7|smoke1|smoke2|run1|run2)
  - MASTER-PROMPT.md v0.3 and generated Run-2 prompt/DIRECTION via tools/make_variant.py
  - results/l7 (allow L, deny L+1, kill L+2 evidence), results/smoke1, results/smoke2
affects: [26-02, 26-03]
tech-stack:
  added: []
  patterns: [python3 stdlib only, bash runner around unmodified spike-002 launcher, asserted-removal variant generation]
key-files:
  created:
    - .planning/spikes/003-divergent-gap-loop-validation/tools/drift.py
    - .planning/spikes/003-divergent-gap-loop-validation/tools/make_variant.py
    - .planning/spikes/003-divergent-gap-loop-validation/experiment/setup-scratch.sh
    - .planning/spikes/003-divergent-gap-loop-validation/experiment/run.sh
    - .planning/spikes/003-divergent-gap-loop-validation/experiment/cap-tests/L7-drift-gap-cap.md
    - .planning/spikes/003-divergent-gap-loop-validation/templates/DIRECTION.run1.md
    - .planning/spikes/003-divergent-gap-loop-validation/templates/DIRECTION.run2.md
    - .planning/spikes/003-divergent-gap-loop-validation/MASTER-PROMPT.md
    - .planning/spikes/003-divergent-gap-loop-validation/experiment/MASTER-PROMPT.run2.md
    - .planning/spikes/003-divergent-gap-loop-validation/experiment/results/l7/
    - .planning/spikes/003-divergent-gap-loop-validation/experiment/results/smoke1/
    - .planning/spikes/003-divergent-gap-loop-validation/experiment/results/smoke2/
  modified: []
key-decisions:
  - "[inferred] Run-2 variant is v0.3 minus asserted spans; comment block of the variant is rewritten so it does not claim the removed rules"
  - "[inferred] OQ-R1 sentence added as its own rule-5 bullet after the needs-human bullet"
requirements-completed: [VALID-01, VALID-02]
duration: ~25 min
completed: 2026-10-17
status: complete
commits: 2
plan_head_before: 3cdf52e460e642d1fca3900712b5586246cc4389
plan_head_after: 3139e8d916f23c415ac938a172cffd366f1b7411
actuals:
  tokens: 29000
  tasks: 2
  commits: 2
---

# Phase 26 Plan 01: Seed and launch kit Summary

A host-fed drifting title feed (9 generations) plus an in-place spike-002 launcher runner, proven end to end on a scripted Haiku driver: the feed advances on a SUMMARY commit made inside the bwrap sandbox, and the gap cap allows at L=2, denies at L+1 and kills at L+2.

## What was built

- **tools/drift.py**: `emit | watch | selftest`. `selftest OK gens=9` proves determinism, prefix property, the trivial slugify passing gen 0 and failing gen 1, and for every k < 9 the reference implementation of classes 1..k passing gens 0..k and failing at least one line of gen k+1; also a full-reference consistency check, no duplicate titles, slug regex, an emit/watch round trip in a temp git repo, refusal without `.planning` and on a symlinked `feed/`, and idempotent restarts (no duplicate snapshots).
- **Final house-rule class list** (apply in this order, then the trivial rule): 1 `+` -> "plus"; 2 `&` -> letter "n", glued; 3 leading article the/a/an dropped; 4 standalone digit 1-9 spelled out (11, 22 stay); 5 abbreviations Saint/Mount/Doctor/Junior -> st/mt/dr/jr (whole word); 6 `#` -> "sharp"; 7 trailing company suffix inc/ltd/llc/co/corp dropped; 8 `/` -> "per"; 9 `.` deleted, glued. Generation 0 is 21 plain titles; each class adds 6 lines. Symbol classes (1, 2, 6, 8, 9) are interleaved with word classes (3, 4, 5, 7).
- **setup-scratch.sh / run.sh**: neutral slugkit scratch project (SC3 is the synced-feed criterion, `feed/` gitignored, seeded at gen 0, no remote), run.sh with scratch guard, token check, env scrub, drift watcher, kill stand-in, auth relaunch, results copy and secret gate. Spike-002 files execute in place (`git diff --exit-code a16e338 -- spike-002` clean; sha256 of launcher.py and driverctl.py recorded in each run-env.txt).
- **MASTER-PROMPT.md v0.3**: v0.2 plus D1 (round semantics), D3 (finding granularity), D1c (divergence precedence), D2 (needs-human on gap-rounds/divergence park) inside rule 5's two bullets, plus the OQ-R1 answer-precedence bullet. Everything from `## Worker brief` on is byte-identical to v0.2.
- **tools/make_variant.py**: asserted removals; `--check` passes; Run-2 DIRECTION differs from Run-1 by exactly two lines.

## Evidence

- **L7**: 1 successful attempt (attempts 0 and 1 aborted before launching the launcher because of a run.sh `set -e` + pipefail bug, fixed; no Claude spend on them). `results/l7/LOG.md` order: `gap-observed count=1`, `count=2`, `count=3`, `dispatch-denied ... phase 1 has failed verification 3 times`, `count=4`, `cap-tripped | cap=gap_rounds | used=4 | limit=2`, `launch-end tripped=gap_rounds`. `evidence=feed-lines before=21 after=27`, drift.log `gen=1 summaries=1`. state.json `hook_dispatches=1`, `denied_dispatches=1`.
- **Guard probe**: `SCRATCH_ROOT=$PWD/guard-probe bash run.sh l7` exited 2; `guard-probe/` never created.
- **Smokes**: smoke1 and smoke2 each have `| RUN | start |` and `dispatch cap reached (0/0)` then `rule=launcher-cap`; smoke2 `standin.log`: `trigger=parked wrote gaps_found via gap-writer.sh`. PROMPT_SHA256 matches the committed prompts.
- **Scratch roots** (outside any git work tree, under `~/.local/state/gsd-meta-manager/spike-003/`): `l7-20261017T021900Z`, `smoke1-20261017T022215Z`, `smoke2-20261017T022242Z`.
- **Spend**: smoke1 $0.0088, smoke2 $0.0083 (result events); L7 killed, no result event, estimated about $0.013 by the cost_split weighting. Total about $0.03.
- Acceptance greps: scratch PROJECT/REQUIREMENTS/ROADMAP/CONTEXT contain 0 matches of drift|generation; results hold no `*.jsonl`/`claude-config`; secret gate clean; `git diff` of src/tests/Cargo files empty.

## Deviations from Plan

### Auto-fixed Issues

**1. [Rule 1 - Bug] run.sh aborted before launching under `set -e` + pipefail**
- **Found during:** Task 1 first run
- **Issue:** `ls|wc` / `find|wc` on a not-yet-existing state dir returned non-zero, killing the script before the launcher ran
- **Fix:** create `STATE_DIR` first and count with `find` on the existing dir
- **Files modified:** experiment/run.sh; **Commit:** b622e94

**2. [Rule 3 - Blocking] spike-002 `__pycache__` appeared untracked**
- **Issue:** `python3 -I` ignores PYTHONDONTWRITEBYTECODE, so `launcher.py init` wrote bytecode into the spike-002 tools dir
- **Fix:** `-B` on the init call in setup-scratch.sh and removal of the generated directory; no spike-002 file changed
- **Commit:** 3139e8d

**3. [Rule 1 - Bug] setup-scratch.sh broke on a relative template path** (it `cd`s into the project). Fixed by resolving the template to an absolute path. **Commit:** 3139e8d

**4. [Rule 2] stand-in poll used `grep -q` in a pipefail pipeline** (SIGPIPE could make a true match read as false); replaced with output to /dev/null. **Commit:** 3139e8d

### [inferred] decisions

- Run-2 prompt's launch-contract comment is replaced (a `v0.3-run2` line) so the variant does not describe the rules it removed; the plan listed only the title change.
- OQ-R1 sentence is a standalone rule-5 bullet placed after the needs-human bullet.
- Stand-in waits for any `| I-01 | status | state=parked` line after the first `dispatch-denied`, or 60 s.
- run.sh exits 1 when the launcher itself returns non-zero (the contract left that case unspecified); `PYTHONDONTWRITEBYTECODE=1` exported.
- Scratch roots under `~/.local/state` as planned; no CLAUDE.md exists on their walk-up (checked).

## Known Stubs

None.

## Threat Flags

None. Threat register T-26-01..07 mitigations implemented (secret gate, scratch guard, env scrub, EXIT trap, drift writes via temp + `os.replace`, spike-002 unchanged).

## Self-Check: PASSED

- drift.py selftest OK, make_variant --check OK, L7-OK and SMOKE-OK checks pass, spike-002 diff clean.
- Commits b622e94 and 3139e8d exist; all files listed under key-files exist.
