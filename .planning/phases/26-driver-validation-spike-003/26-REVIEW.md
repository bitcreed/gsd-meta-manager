---
phase: 26-driver-validation-spike-003
reviewed: 2026-10-17T03:16:10Z
depth: standard
files_reviewed: 7
files_reviewed_list:
  - .planning/spikes/003-divergent-gap-loop-validation/experiment/acceptance.py
  - .planning/spikes/003-divergent-gap-loop-validation/experiment/run.sh
  - .planning/spikes/003-divergent-gap-loop-validation/experiment/setup-scratch.sh
  - .planning/spikes/003-divergent-gap-loop-validation/tools/drift.py
  - .planning/spikes/003-divergent-gap-loop-validation/tools/dup_check.py
  - .planning/spikes/003-divergent-gap-loop-validation/tools/make_variant.py
  - .planning/spikes/003-divergent-gap-loop-validation/tools/stage_cost.py
findings:
  critical: 0
  warning: 10
  info: 7
  total: 17
status: issues_found
---

# Phase 26: Code Review Report

**Reviewed:** 2026-10-17T03:16:10Z
**Depth:** standard
**Files Reviewed:** 7
**Status:** issues_found

## Scope note

The evaluation-scope resolver ran in degraded phase-range fallback mode and listed 12 Rust/docs files. They came only from pre-phase quick-task and release commits, so they are intentionally out of scope [inferred]. The review covers only the 7 Python/shell spike tooling files above, which are the phase's source deliverables. `.planning/` result artifacts, MASTER-PROMPT files and templates were not reviewed except where read to confirm a claim (spike 002 `L3-LOG.md`, `driverctl.split_frontmatter`, and the `experiment/results/*` directory layout).

## Summary

This is throwaway measurement and launch tooling for a validation spike. Most of it is carefully built: asserted text removal in `make_variant.py`, UNDETERMINED-instead-of-PASS handling, privacy-preserving transcript reads, and self-tests with fault injection. No security vulnerabilities or data-loss defects in shipped product code were found, so there are no BLOCKERs.

There are several defects that can corrupt or falsely settle the experiment's pre-registered verdicts:
- A results-copy bug on the `RELAUNCH=1` path breaks A8.
- The A9 check mis-fires on a converged run.
- The A8 slug matching can false-PASS on ordinary English words.
- The A11 bypass check can mask a real bypass.
- The feed watcher can silently regress the feed or die without anyone noticing.

## Warnings

### WR-01: `cp -r` nests `verification/` into itself on `RELAUNCH=1`

**File:** `.planning/spikes/003-divergent-gap-loop-validation/experiment/run.sh:176`
**Issue:** `RELAUNCH=1` skips the `rm -rf "$RES"; mkdir -p "$RES"` step (lines 104-105), so `$RES/verification` already exists from the earlier invocation's step (g). `cp -r "$SCRATCH_ROOT/verification" "$RES/verification"` then copies into the existing directory and produces `$RES/verification/verification/*`. Snapshots taken after the relaunch land one level too deep. `acceptance.py` `check_a8` reads `results/verification/<name>` and returns UNDETERMINED (missing input) for them. The resulting run silently loses its A8 evidence. `RELAUNCH` is a documented, designed path.
**Fix:**
```bash
if [ -d "$SCRATCH_ROOT/verification" ]; then
  mkdir -p "$RES/verification"
  cp -a "$SCRATCH_ROOT/verification/." "$RES/verification/" || true
fi
```

### WR-02: Transient git failure regresses the feed to generation 0

**File:** `.planning/spikes/003-divergent-gap-loop-validation/tools/drift.py:220-227, 251-261`
**Issue:** `_git()` returns `""` on any non-zero exit. `count_summaries()` turns that into `0`. `poll()` sees `0 != self.count` and rewrites `feed/` at generation 0, then logs `gen=0 summaries=0`. One failed `ls-tree` makes the feed shrink and un-grow, for example while the executor is mid-commit or `HEAD` is briefly unreadable. A verifier reading the feed in that window sees a non-append-only target. That breaks the spike's central invariant ("never changes an earlier line") and corrupts A10's monotonic-gen premise.
**Fix:** Make `_git` return `None` on failure and have `count_summaries` return `None`. Have `poll()` treat `None` as "no change":
```python
n = count_summaries(self.project)
if n is None or n == self.count:
    self._snapshot_verifications(); return
```
Alternatively, make the generation monotonic: `n = max(n, self.count)`.

### WR-03: A9 FAILs a legitimately converged run

**File:** `.planning/spikes/003-divergent-gap-loop-validation/experiment/acceptance.py:502-511`
**Issue:** A9 asserts `max(gap-observed count) - len(standin) == gsd-verifier runs`. A run that converges (the final verification is `passed`, which is `SEED CONVERGED`) has one more verifier run than `gaps_found` observations. Example: gaps(1), gaps(2), pass gives 2 gaps vs 3 verifier runs. A9 then reports FAIL and `RESULT FAIL` for an outcome the rubric itself recognises as a valid seed class. The selftest only exercises `converged` for the SEED line and never checks A9.
**Fix:** Allow the passing verification: expect `v == seen` or `v == seen + 1` when a `passed` snapshot or an `I-01 status done` exists. Otherwise compare as now.

### WR-04: A8 slug matching false-positives on common English words

**File:** `.planning/spikes/003-divergent-gap-loop-validation/experiment/acceptance.py:452-455`
**Issue:** Expected slugs are matched as bare words anywhere in the `gaps:` block, including prose. `emit(MAX_GEN)` includes slugs such as `end` (from "The End") and `matrix`. A verifier sentence such as "...at the end of the line" in snapshot k adds `("slug","end")` as a "new case". That satisfies the "adds a new case" test and A8 PASSes with no real new finding. This inflates the central pre-registered measurement (non-subset findings).
**Fix:** Restrict slug matching to quoted or backticked tokens, or require the case to co-occur with its title or a `line N` reference. For example, match slugs only inside backticks or quotes: `` r"[`\"']" + re.escape(s) + r"[`\"']" ``.

### WR-05: A11 treats any dispatch right after a deny as denied

**File:** `.planning/spikes/003-divergent-gap-loop-validation/experiment/acceptance.py:383-386`
**Issue:** `prev.event == "dispatch-denied"` is used to skip a `dispatched` line. The `dispatched` line is an ordinary driver log that precedes its own deny (see selftest: dispatched at 740, deny at 741). Real logs also show consecutive denies with no `dispatched` between them (spike 002 `L3-LOG.md` lines 41-42). So the sequence `dispatched(A), denied, dispatched(B), report` skips B because its predecessor is a deny. B is a worker that actually ran after count=3, which is exactly the bypass A11 exists to catch, and it is reported as "denied" or ignored. The look-ahead already handles the correct case, so the look-behind adds only false negatives.
**Fix:** Drop the look-behind and rely on the look-ahead alone. If a deny can precede its `dispatched` line, pair the deny to the nearest unmatched dispatch by order instead of by adjacency.

### WR-06: `SEED VALID` is emitted while A10 is UNDETERMINED or NOT-REACHED

**File:** `.planning/spikes/003-divergent-gap-loop-validation/experiment/acceptance.py:573-575`
**Issue:** `if round1 and a10 != FAIL: return "VALID"`. A missing `drift.log` gives A10 UNDETERMINED. Fewer than two organic `gaps_found` snapshots gives NOT-REACHED. In both cases the seed's defining property (a moving target across rounds) is unproven, yet the seed is classed VALID and the run's verdict is built on it. This contradicts the module's stated rule that an unprovable check must never read as success.
**Fix:** `if round1 and a10 == PASS: return "VALID"`; otherwise return `STALLED` (or a distinct `UNVERIFIED`) for UNDET and NOTREACHED.

### WR-07: Feed watcher has no error handling and nothing checks its liveness

**File:** `.planning/spikes/003-divergent-gap-loop-validation/tools/drift.py:297-308`; `.planning/spikes/003-divergent-gap-loop-validation/experiment/run.sh:143-146`
**Issue:** `cmd_watch` only catches `KeyboardInterrupt`. Any `OSError` in `poll()` (for example a phase directory vanishing between `isdir` and `listdir`, or `shutil.copyfile` racing a rewrite) kills the watcher. `run.sh` backgrounds it with output to `watch.out` and never checks `kill -0 $WATCH_PID`. The launch continues with a frozen feed and an empty `drift.log`, and the run is later mis-scored as A10/A8 NOT-REACHED or as a stalled seed rather than as an infrastructure failure.
**Fix:** Wrap `w.poll()` in `try/except Exception` that logs to the drift log and continues. In `run.sh`, check `kill -0 "$WATCH_PID"` after the launch and record `WATCHER_ALIVE=` in `run-env.txt` (fail the run if it died).

### WR-08: Watcher and stand-in are still running while results are copied

**File:** `.planning/spikes/003-divergent-gap-loop-validation/experiment/run.sh:95-101, 171-185`
**Issue:** `WATCH_PID` and `STANDIN_PID` are killed only in the `EXIT` trap, after step (g) has copied `drift.log`, `verification/` and `standin.log`. After the launcher exits, the stand-in can still be in its up-to-60 s wait loop. It can then run `gap-writer.sh` and append to `standin.log` after the copy. The result is an unlabelled `gaps_found` snapshot in the copied data, or a `standin.log` that does not match the copied `drift.log`. This interacts with A8/A10/A9's stand-in labelling. The watcher can also write a snapshot mid-`cp -r`.
**Fix:** After the launcher loop and the `sleep 2`, explicitly `kill "$STANDIN_PID" "$WATCH_PID"; wait` them before step (g).

### WR-09: Unvalidated label and path feed destructive `rm -rf`, `sed` and results overwrite

**File:** `.planning/spikes/003-divergent-gap-loop-validation/experiment/run.sh:55-57, 105`; `.planning/spikes/003-divergent-gap-loop-validation/experiment/setup-scratch.sh:16, 186`
**Issue:**
- `RESULTS_LABEL` is interpolated unchecked into `rm -rf "$RES"`, `RUN_ID`, the `systemctl ... "gsdmm-$RUN_ID-*.scope"` glob, and `sed "s/{run-id}/$RUN_ID/g"`. A label containing `/`, `..` or `&` deletes the wrong directory or corrupts the DIRECTION.
- A plain re-run of `run.sh run1` (no label, no `RELAUNCH`) silently wipes the committed `results/run1/` evidence. The existing `run1b` label shows this workaround already had to be used.
- `setup-scratch.sh` runs `rm -rf "$P"` guarded only by "absolute and not `/`", so `setup-scratch.sh /home/me` deletes a home directory.

**Fix:**
```bash
case "$LABEL" in ''|.*|*[!A-Za-z0-9._-]*) echo "bad RESULTS_LABEL" >&2; exit 2;; esac
[ -e "$RES" ] && [ "${RELAUNCH:-0}" != 1 ] && { echo "results/$LABEL exists; choose a new RESULTS_LABEL" >&2; exit 2; }
```
In `setup-scratch.sh`, require `basename "$P" = project` (or an existing marker file) before `rm -rf`.

### WR-10: `dup_check.py` ignores workers whose parent metadata is missing

**File:** `.planning/spikes/003-divergent-gap-loop-validation/tools/dup_check.py:156-157, 171`
**Issue:** A node is a top-level worker only if `parentAgentId` is falsy. A node whose `parentAgentId` points at an id with no `meta.json` is counted in `type_total` but is neither a worker nor reachable from one. Duplicate planners or executors under it are never flagged, and `workers=` under-counts. That under-count would pass `len(workers) <= allowed` and let A7 PASS with a false `DUP=0`. `stage_cost.py` handles the same situation correctly (`n["parent"] not in nodes` means top-level), so the two tools disagree on the same data.
**Fix:** Define top-level as `not parent or parent not in the session's node set`, matching `stage_cost.session_agents`/`tops`.

## Info

### IN-01: Three divergent copies of the LOG parser

**File:** `.planning/spikes/003-divergent-gap-loop-validation/experiment/acceptance.py:36,54-82`; `tools/dup_check.py:40,44-62`; `tools/stage_cost.py:42,61-78`
**Issue:** `EV_RE` and `parse_log` are copy-pasted three times. Only `acceptance.py` skips the `<!-- -->` grammar block and keeps detail lines. They will drift (and, for the ` | `-in-value truncation, are already identically lossy).
**Fix:** Put the parser in one module (e.g. `tools/logparse.py`) and import it.

### IN-02: `acceptance.py` module docstring and usage text are stale

**File:** `.planning/spikes/003-divergent-gap-loop-validation/experiment/acceptance.py:1-16`
**Issue:** The docstring documents only `run1`/`selftest` and the `run1` check ids. The CLI also supports `l7`, `run2` and `rubric-check` (and their A5/A11/L7 ids), and the `--dup-report` flag's exit semantics are undocumented.
**Fix:** Update the docstring to match `main()`'s usage string and `checks_for()`.

### IN-03: Temp DIRECTION file leaks if `launcher.py init` fails

**File:** `.planning/spikes/003-divergent-gap-loop-validation/experiment/setup-scratch.sh:185-188`
**Issue:** With `set -e`, a failing `launcher.py init` exits before `rm -f "$DIRECTION"`.
**Fix:** `trap 'rm -f "$DIRECTION"' EXIT` right after `mktemp`.

### IN-04: `stage_cost.py` silently prices unmatched models at $0

**File:** `.planning/spikes/003-divergent-gap-loop-validation/tools/stage_cost.py:262-273, 280`
**Issue:** When a model has no rate (`priced=partial`) its cost is counted as 0. The TOTAL, CHECK and exit code 0 all still read as clean, and the only signal is a word in the header. A `<synthetic>` or unfamiliar model id quietly understates VALID-04 cost.
**Fix:** List the unmatched models explicitly in output and return a non-zero code (or at least a `WARN` line) when `priced=partial`.

### IN-05: Snapshot status regex fails on quoted values

**File:** `.planning/spikes/003-divergent-gap-loop-validation/tools/drift.py:217, 287-289`
**Issue:** `^status:\s*([A-Za-z_]+)` yields `unknown` for `status: "gaps_found"` or `'gaps_found'`. Acceptance filters on `status == "gaps_found"`, so such a snapshot is dropped from A8/A10.
**Fix:** Allow optional quotes: `^status:\s*["']?([A-Za-z_]+)`.

### IN-06: Auth-failure relaunch detection is brittle

**File:** `.planning/spikes/003-divergent-gap-loop-validation/experiment/run.sh:160-162`
**Issue:** `grep -qE 'OAuth token revoked|Failed to authenticate'` scans the whole stream JSONL, including assistant and tool text, so quoted transcript content could trigger a relaunch. `grep -q '"tripped": null'` depends on the launcher's exact JSON spacing.
**Fix:** Parse the launcher result and the stream's `result`/error events with `python3 -I` rather than grepping raw text.

### IN-07: Result copy errors are swallowed

**File:** `.planning/spikes/003-divergent-gap-loop-validation/experiment/run.sh:174-179`
**Issue:** `cp ... 2>/dev/null || true` means a missing `LOG.md`/`state.json` produces `exit 0` ("launcher ran to completion") with an empty results directory. The failure only surfaces later as UNDETERMINED checks.
**Fix:** Record a `COPIED=` / `MISSING=` line in `run-env.txt`, or fail loudly when `LOG.md` is absent.

---

_Reviewed: 2026-10-17T03:16:10Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
