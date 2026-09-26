---
phase: 25-running-agents-live-wave-view
verified: 2026-09-26T05:00:00Z
status: passed
score: 7/7 must-haves verified
covered_files:
  - ".planning/REQUIREMENTS.md"
  - ".planning/phases/25-running-agents-live-wave-view/25-01-PLAN.md"
  - ".planning/phases/25-running-agents-live-wave-view/25-01-SUMMARY.md"
  - ".planning/phases/25-running-agents-live-wave-view/25-02-PLAN.md"
  - ".planning/phases/25-running-agents-live-wave-view/25-02-SUMMARY.md"
  - ".planning/phases/25-running-agents-live-wave-view/25-03-PLAN.md"
  - ".planning/phases/25-running-agents-live-wave-view/25-03-SUMMARY.md"
  - ".planning/phases/25-running-agents-live-wave-view/25-04-PLAN.md"
  - ".planning/phases/25-running-agents-live-wave-view/25-04-SUMMARY.md"
  - ".planning/phases/25-running-agents-live-wave-view/25-05-PLAN.md"
  - ".planning/phases/25-running-agents-live-wave-view/25-05-SUMMARY.md"
  - ".planning/phases/25-running-agents-live-wave-view/25-06-PLAN.md"
  - ".planning/phases/25-running-agents-live-wave-view/25-06-SUMMARY.md"
  - ".planning/phases/25-running-agents-live-wave-view/25-07-PLAN.md"
  - ".planning/phases/25-running-agents-live-wave-view/25-07-SUMMARY.md"
  - ".planning/phases/25-running-agents-live-wave-view/25-CONTEXT.md"
  - ".planning/phases/25-running-agents-live-wave-view/25-REVIEW.md"
  - "README.md"
  - "docs/ARCHITECTURE.md"
  - "src/action.rs"
  - "src/agents/adapters/claude.rs"
  - "src/agents/adapters/mod.rs"
  - "src/agents/fixers.rs"
  - "src/agents/mod.rs"
  - "src/agents/waves.rs"
  - "src/agents/worktrees.rs"
  - "src/app.rs"
  - "src/lib.rs"
  - "src/state_reader/disk_status.rs"
  - "src/state_reader/git_ops.rs"
  - "src/ui/screens/delete_confirm.rs"
  - "src/ui/screens/detail.rs"
  - "src/ui/screens/driver_confirm.rs"
  - "src/ui/screens/help.rs"
  - "src/ui/screens/mod.rs"
  - "src/ui/screens/normal.rs"
  - "src/ui/screens/render_escape_guard.rs"
  - "tests/agents_claude.rs"
  - "tests/agents_fixers.rs"
  - "tests/agents_scan.rs"
  - "tests/agents_waves.rs"
covered_digest: "v1:sha256:c69857b4bd8cbdc30d556bdddb317fb34abba5f90a584d8b358042f2a936d66c"
# NOTE: the installed gsd-core (@opengsd/gsd-core 1.14.0, gsd-tools.cjs at
# /home/blk/projects/node/gsd-core/gsd-core/bin/gsd-tools.cjs) has no
# `verification.fingerprint` verb (`query verification` only exposes
# `status`/`resolve-file`). This digest is a manual substitute: sha256 of the
# sorted `sha256sum` lines for the files above. Recompute with the canonical
# verb once available; do not treat this as the authoritative fingerprint.
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: "6/7 (requirement level; 1 critical correctness defect)"
  gaps_closed:
    - "The dashboard/detail summary shows what is currently running — an aborted or crashed run's leftover worktree must eventually stop reading Finished/Stalled and stop overriding the Status cell (CR-01 / AGENT-05)"
  gaps_remaining: []
  regressions: []
gaps: []
deferred: []
advisory: []
---

# Phase 25: Running Agents & Live Wave View Verification Report

**Phase Goal:** For each registered project, show which GSD agents are currently running in parallel and, for executors, which wave and plans are running / queued / done — read-only from files and git, no Claude invocation, non-intrusive to the running GSD session
**Verified:** 2026-09-26
**Status:** passed
**Re-verification:** Yes — after gap closure (plan 25-07, commits `29cd4f3..06f6045`; code review `5579bbc`)

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Agent worktrees are enumerated with commit/dirty counts, non-intrusively (AGENT-01) | ✓ VERIFIED (regression-checked) | Unchanged since prior pass; `tests/agents_scan.rs` 13/13, including `no_file_under_src_agents_writes_reads_proc_or_spawns` and `scanning_a_live_agent_worktree_never_touches_its_index`. |
| 2 | Plans grouped by `wave:` frontmatter, never plan number; done/running/stalled/queued derived (AGENT-02) | ✓ VERIFIED (regression-checked) | `src/agents/waves.rs::derive`; `tests/agents_waves.rs` 7/7, including the rewritten `a_summary_committed_in_the_worktree_reads_finished_before_the_merge` (now asserts a lone `Finished` row is inactive, and a live sibling row makes it `1 run · 2/3 done`). |
| 3 | Adapter seam degrades gracefully on any failure, never drops a row or fails the scan (AGENT-03) | ✓ VERIFIED (regression-checked) | `tests/agents_scan.rs::a_panicking_adapter_leaves_the_core_rows_intact`, `a_test_only_adapter_claims_a_worktree_outside_the_predicate` pass. |
| 4 | Claude Code adapter attaches type/description/liveness; lists worktree-less subagents (AGENT-04) | ✓ VERIFIED (regression-checked) | `tests/agents_claude.rs` 12/12, including two new orphan-aging tests (`a_released_lock_orphan_days_stale_reads_ended_and_is_not_active`, `a_locked_orphan_days_stale_reads_ended_not_stalled`). |
| 5 | Dashboard Status cell shows a compact summary while agents are active; unchanged otherwise (AGENT-05) | ✓ VERIFIED — gap CR-01 closed | `src/agents/mod.rs:88-137` (`classify_facts`) now returns `Ended` once `age > MAX_AGENT_AGE_SECS`, before the released-lock `Finished` rule; `AgentLiveness::is_running()` (Live\|Idle only) is the single predicate `AgentView::is_active`, `executor_mode`, `N agents` and the fixer gate all use (`src/agents/waves.rs:431-434`). `Finished`/`Stalled` alone never activates the cell. Verified directly by reading `src/agents/mod.rs`, `src/agents/waves.rs`, `src/ui/screens/normal.rs:896-901`, and by running the two byte-identical regression tests myself: `ui::screens::normal::tests::an_aged_out_orphan_leaves_the_status_cell_byte_identical` and `..._a_finished_orphan_within_the_age_bound_leaves_the_status_cell_byte_identical` — both `ok`. `app::tests::an_agents_scan_reaches_the_dashboard_status_cell` also `ok`. |
| 6 | Sessions tab gets an Agents sub-view with summary, per-wave rows, scrollable list, worktree-less group, degraded/empty states (AGENT-06) | ✓ VERIFIED (regression-checked) | `src/ui/screens/detail.rs::render_agents_tab` unchanged by 25-07; `--lib ui::screens::detail` 196/196 pass. |
| 7 | Code-review fix runs show an estimated fixed/total, labelled `~` (AGENT-07) | ✓ VERIFIED (with known estimation-quality warnings, not blocking) | `src/agents/fixers.rs::estimate` now requires at least one *running* (Live/Idle) unattributed fixer before returning `Some` (the running-fixer gate, `fixers.rs:219-224`), closing the "orphan fixer alone switches on the estimate" half of CR-01/WR-01 for this path. `tests/agents_fixers.rs` 7/7, including new `a_finished_fixer_run_yields_no_estimate`. The estimate's phase-vote and finding-count logic downstream of the gate is still untiered — see WR-06 below; this is an accuracy defect in the *estimate*, not a violation of the "labelled estimate while running" truth. |

**Score:** 7/7 truths hold. All regression-checked truths (1–4, 6) are unchanged since the prior pass; truth 5 (AGENT-05) is newly verified as fixed; truth 7 (AGENT-07) still holds with a known, non-blocking accuracy caveat (WR-06, new).

### Required Artifacts

| Artifact | Expected | Status | Details |
|----------|----------|--------|---------|
| `src/agents/mod.rs` | AgentLiveness, classify_facts, MAX_AGENT_AGE_SECS wired into classification | ✓ VERIFIED | `classify_facts` (mod.rs:113-137) now consumes `MAX_AGENT_AGE_SECS` (`age > MAX_AGENT_AGE_SECS` → `Ended`, before the released-lock check); `classify_child` routes through the same function. `AgentLiveness::is_running()` added (mod.rs:77-91). |
| `src/agents/waves.rs` | Tiered active-phase vote; running-only `is_active`/`executor_mode`/`N agents` | ✓ VERIFIED | `derive`'s `vote` closure is tiered: running rows first, `Finished\|Stalled` fallback, then `state.active_phase_number()` (waves.rs:319-336); `is_active` (waves.rs:431-434), `executor_mode` and the `N agents` count all gate on `is_running()`. |
| `src/agents/fixers.rs` | Running-fixer gate before any I/O | ✓ VERIFIED (with WR-06 caveat) | `estimate` returns `None` unless `fixers.iter().any(|r| r.liveness.is_running())` (fixers.rs:219-222), before `own_subjects`/`log_subjects` are read. The phase vote below the gate (fixers.rs:240-258) and the numerator union (fixers.rs:266-286) still treat every `Finished` fixer as equal to a running one — this is WR-06, an accuracy defect downstream of the gate, not a missing gate. |
| `src/ui/screens/normal.rs` | Status-cell wrap using `is_active`/`summary_forms` | ✓ VERIFIED | Unchanged at normal.rs:896-901; correct now because `agent_summary_line` returns `None` for an orphan-only view. Two new byte-identical regression tests added and passing. |
| `tests/agents_claude.rs`, `agents_waves.rs`, `agents_fixers.rs`, `agents_scan.rs` | Orphan-aging and tiered-vote regression coverage | ✓ VERIFIED | I ran all four suites myself (not just re-stating SUMMARY): `agents_claude` 12/12, `agents_waves` 7/7, `agents_fixers` 7/7, `agents_scan` 13/13 — all pass, 0 failed. |

### Key Link Verification

| From | To | Via | Status | Details |
|------|----|-----|--------|---------|
| `src/agents/mod.rs classify_facts` | `MAX_AGENT_AGE_SECS` | direct comparison before the lock-released branch | ✓ WIRED | mod.rs:130-137, confirmed by reading the function body. |
| `src/agents/waves.rs derive`/`is_active`/`summary_forms` | `AgentLiveness::is_running()` | single predicate reused across activation, vote tier 1, executor mode, `N agents` | ✓ WIRED | waves.rs:319-336, 431-475, 485-490. |
| `src/agents/fixers.rs estimate` | `AgentLiveness::is_running()` | gate before `own_subjects`/`log_subjects` | ✓ WIRED (gate only) | fixers.rs:219-224; the gate itself is real and precedes I/O, but the vote/union past the gate does not re-check `is_running()` per-row (WR-06). |
| `normal.rs` status cell | `AgentView::is_active`/`summary_forms` | `agent_summary_line` | ✓ WIRED, regression-tested | normal.rs:896-901; pinned by two new byte-identical tests. |

### Behavioral Spot-Checks / Test Runs (run directly by the verifier, not taken from SUMMARY)

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Phase-25 unit tests | `cargo test --no-fail-fast --lib agents::` | 45 passed, 0 failed | ✓ PASS |
| Phase-25 integration suites | `cargo test --no-fail-fast --test agents_claude --test agents_waves --test agents_fixers --test agents_scan` | 12+7+7+13 = 39 passed, 0 failed | ✓ PASS |
| Byte-identical dashboard regressions | `cargo test --no-fail-fast --lib ui::screens::normal` (filtered to the two new tests) | both `ok`; 55 passed, 0 failed overall | ✓ PASS |
| Tick→scan→Status-cell integration | `cargo test --no-fail-fast --lib app::tests` (filtered to `an_agents_scan_reaches_the_dashboard_status_cell`) | `ok`; 76 passed, 0 failed overall | ✓ PASS |
| Full workspace suite (regression check) | `cargo test --no-fail-fast` (via `rtk proxy`, raw output, once) | 2488 passed, 1 failed (expected: `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`, per project memory), 15 ignored | ✓ PASS (expected failure only) — matches the count the SUMMARY claims, independently reproduced |

### Requirements Coverage

| Requirement | Source Plan | Description | Status | Evidence |
|-------------|-------------|--------------|--------|----------|
| AGENT-01 | 25-01 | Enumerate agent worktrees, non-intrusive git | ✓ SATISFIED | Unchanged; regression-checked |
| AGENT-02 | 25-03, 25-07 | Wave model from frontmatter, done/running/stalled/queued | ✓ SATISFIED | `waves.rs` tiered vote and running-only gates |
| AGENT-03 | 25-01, 25-02 | Adapter seam degrades gracefully | ✓ SATISFIED | Unchanged; regression-checked |
| AGENT-04 | 25-02, 25-07 | Claude Code adapter: metadata + liveness | ✓ SATISFIED | New orphan-aging tests pass |
| AGENT-05 | 25-04, 25-07 | Dashboard summary, unchanged with no active agents | ✓ SATISFIED — CR-01 closed | Age bound + running-only activation; byte-identical regressions pass |
| AGENT-06 | 25-05 | Detail Agents sub-view | ✓ SATISFIED | Unchanged; regression-checked |
| AGENT-07 | 25-06, 25-07 | Fixed/total estimate, labelled as estimate | ✓ SATISFIED (WR-06 accuracy caveat, not blocking) | Running-fixer gate added; the estimate's own phase/count math still needs a tiered vote (follow-up) |

All seven requirement IDs (AGENT-01..07) are declared across the phase's PLAN frontmatter (including 25-07) and are listed in `.planning/REQUIREMENTS.md` under "Agent Observation (AGENT)" with a Traceability row each. `.planning/REQUIREMENTS.md`'s per-requirement checkbox table (lines 108-114) is still unchecked and its traceability table (lines 190-196) still reads "Gaps Found" for all seven IDs — this is stale bookkeeping from before the 25-07 gap closure and this re-verification; it does not reflect code state and should be updated to "Complete" as part of closing out this re-verification, but it is a documentation-sync item, not a code gap.

### Anti-Patterns Found

None. No `TBD`/`FIXME`/`XXX`/`TODO`/`HACK`/`placeholder` markers in any phase-25 file touched by 25-07 (`src/agents/mod.rs`, `src/agents/waves.rs`, `src/agents/fixers.rs`, `src/ui/screens/normal.rs`, `tests/agents_claude.rs`, `tests/agents_waves.rs`, `tests/agents_fixers.rs`) — checked directly with grep, zero matches.

### Code Review Cross-Reference (25-REVIEW.md, re-review after 25-07, commit `5579bbc`)

The re-review (`status: issues_found`, 0 critical / 5 warning / 7 info) confirms CR-01 and WR-01 resolved and reports one new warning (WR-06) plus carried-forward warnings/infos. Independent judgment on each, against the phase's must-haves:

- **CR-01 — Resolved, confirmed independently.** I read `src/agents/mod.rs:113-137`, `src/agents/waves.rs:319-336,431-434`, `src/ui/screens/normal.rs:896-901` myself and ran the cited regression tests; they match the review's evidence exactly. This is the truth-5/AGENT-05 gap from the prior verification, now closed.
- **WR-01 — Resolved in `waves.rs`.** The tiered vote (`derive`'s `vote` closure) is confirmed by reading the code and by `running_agents_outvote_orphans_for_the_active_phase` passing.
- **WR-06 (new) — judged a non-blocking Warning, not a gap.** `fixers::estimate`'s running-fixer *gate* (fixers.rs:219-224) is real and does exactly what AGENT-07 needs — no estimate switches on unless something is running. But past that gate, `is_active_fixer` still keeps `Finished` (fixers.rs:137-148), and the phase vote (fixers.rs:240-258) and the finding-id union (fixers.rs:266-286) treat every `Finished` fixer as equal to the running one(s). So when an aborted fix run's orphaned (but not yet aged-out) fixers outnumber a genuinely running one, the estimate's `fixers` count, `phase`, and `fixed` numerator can all point at the wrong, already-finished run. I confirmed the untiered vote directly in the code (`votes` loop at fixers.rs:243-249 has no `is_running()` filter). This is the same *shape* of defect as CR-01/WR-01, but it does not defeat AGENT-07's stated truth — "the summary shows an estimated fixed/total count ... labelled as an estimate" — because a labelled `~k/N fixed` estimate is still shown while something is running; it can just be attributed to the wrong phase or count the wrong fixers. This is an accuracy/quality defect in the estimate's own math, in the same family as the already-deferred WR-03/WR-04 (wrong REVIEW file, uncapped double-counting) that the prior verification correctly classified as non-blocking warnings under the same "truth only requires a labelled estimate" reasoning. I apply that same standard here rather than a stricter one merely because the new instance resembles CR-01's shape. **Recommendation:** fold WR-06 into the same follow-up as WR-03/WR-04/WR-05 (a small, scoped fixer-estimate accuracy pass), not into this phase's gap-closure loop.
- **WR-02 (hung git call freezes the scan) — carried forward, unchanged, non-blocking.** No must-have promises a scan timeout; this is an availability/robustness concern for follow-up.
- **WR-03, WR-04 (fixer estimate file-selection and double-count) — carried forward, unchanged, non-blocking**, per the same reasoning as the prior verification.
- **WR-05 (long-path prefix fallback can misattribute subagents) — carried forward, unchanged, non-blocking.** An edge case (>200-unit encoded paths colliding on a 200-unit prefix), not covered by any must-have's specific wording.
- **IN-01 through IN-07 — info-level, non-gating.** IN-01 (redraw-storm from orphan rows) is a performance/comment-accuracy note, not a correctness defect in what is shown. IN-04's impact is "increased" per the review but remains explicitly unconfirmed ([inferred], no locking runtime observed that doesn't lock) — left as an open info item, not escalated to human verification, because it is a hypothesis about adapter behavior the review itself flags as unproven, not an observed defect.

### Human Verification Required

None. Every item above — including the new CR-01 closure and the new WR-06 finding — was settled with direct code inspection (file:line citations above), independent test execution (not just re-stating SUMMARY/REVIEW claims), and a full-suite regression run. No visual, real-time, or external-service check was needed to reach a verdict.

### Gaps Summary

No gaps. The prior blocking gap (CR-01: orphaned agent worktrees never aged out of `Finished`/`Stalled`, permanently overriding the dashboard Status cell and defeating AGENT-05's "unchanged otherwise") is closed by plan 25-07: `classify_facts` now returns `Ended` once inactivity exceeds `MAX_AGENT_AGE_SECS` (a day), and `AgentLiveness::is_running()` (Live\|Idle only) is the single predicate that switches the dashboard summary, the active-phase vote's first tier, executor mode, and the fixer estimate's gate. I independently reproduced the fix's regression tests (byte-identical dashboard output for an aged-out orphan and for a within-bound `Finished` orphan) and ran the full phase-25 test surface plus a full workspace regression pass (2488 passed / 1 expected-failure / 15 ignored) myself, rather than trusting the SUMMARY's counts.

One new, non-blocking finding (WR-06) surfaced in the re-review: the fixer estimate's phase attribution and finding-count math, downstream of the new running-fixer gate, still weighs finished orphan fixers equally with running ones. I judged this a quality/accuracy issue for AGENT-07 (consistent with the prior verification's treatment of WR-03/WR-04), not a gap against any stated must-have — recommended for the same follow-up pass as the other open fixer-estimate warnings (WR-03, WR-04, WR-05).

`.planning/REQUIREMENTS.md`'s checkbox and traceability tables for AGENT-01..07 are stale (unchecked / "Gaps Found") relative to this passing re-verification and should be updated as bookkeeping, not as a code gap.

---

_Verified: 2026-09-26_
_Verifier: Claude (gsd-verifier)_
