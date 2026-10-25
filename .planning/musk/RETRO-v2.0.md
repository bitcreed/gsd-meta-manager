# Musk retro: v2.0 Autonomous Orchestration

Run: 2026-10-25 (`/gsd-musk-retro`, unattended). The milestone is still in progress: 28.1 is nearly done, 31 is being planned, 32 is pending. This retro is interim. Re-run it at milestone close.

## 1. Numbers

Calibration (`musk calibrate`, basis `harness`): 29 sessions, 505 subagents, 101.61M tokens (+3118.67M cache read).
Prices: per_task p50 21k (n=74), per_plan_overhead p50 168k (n=74), per_phase p50 5.40M (n=12), per_quick_task p50 410k (n=60).

### Cuts

| class | applied | proposed | reinstated | active |
|---|---|---|---|---|
| scope | 0 | 0 | 0 | 0 |
| impl | 24 | 0 | 2 | 22 |
| ceremony | 0 | 0 | 0 | 0 |

Active cuts by unit: test 7, task 6, abstraction 4, doc 3, flag 2, dep 1, file 1.

**Add-back ratio: 2 / 24 = 8.3%.** That is in the healthy range for impl cuts. No scope cuts were made, though `musk.scope_cuts=defer` is set, because the milestone pass never ran: `MILESTONE-v2.0.md` does not exist. Phases 31 and 32 still have no scope challenge.

### Estimated tokens (basis `harness`, n = 22 active cuts)

| | p25 | p50 | p75 |
|---|---|---|---|
| gross saved | 0 | 506,976 | 1,335,288 |
| pass cost | 29,146 | 29,146 | 29,146 |
| **net** | **-29,146** | **477,830** | **1,306,142** |

Value of the reinstated cuts, which is no longer counted as saved: p50 42k, p75 111k (n=2). Pass cost is measured: 15.0k from fragments plus 14.2k from skills, over 6 discuss and 36 plan injections. p25 is negative because the p25 per-task price is negative (-2k). At the low quartile, the cuts saved about nothing and the pass cost something.

### Unreviewed cuts settled from evidence

All 24 cuts were `unreviewed` (all impl, all `--inferred`). None needs the user.

**Reinstated (2):** the cut thing was built anyway.
- **M-0015** `30-UI-SPEC.md:665` typed-model charset line. It was cut at 05:48Z. 14 minutes later, `524ba05` (quick-261021-101) rewrote it to the bracket rule, citing WR-07.
- **M-0024** `docs/DRIVER-SANDBOX.md` residual-risk lines (WR-02/05/06/08, IN-06). The cut deferred them to `/gsd-code-review 28.1 --fix`, which then wrote them: `f5bab13` (WR-02), `2d5ab57` (WR-08), `e6053d7` (IN-06). Both are the same pattern: a doc line marked "deferred to a later step" that the next step then wrote. A deferral that gets done is not a deletion.

**Confirmed held (22):** M-0001..M-0014, M-0016..M-0023. Nothing rebuilt them:
- `classify_status` is unchanged since `2d33f07`.
- None of these exist in the tree: `is_awaiting_verification`, `infer_long_args`, `GSD_MM_DRIVER_ROOT`, a direct `libc` dep, a launcher cost record, `src/launcher/managed.rs`, a `ManagedSettings` variant or `Resolver` seam, `run-3-answer.sh`, a new fake-claude launcher harness.
- Drive's `run.lock` still uses `lock::acquire`. `acquire_at_with_retry` was adopted only for the direction lock.
- `28.1-SECURITY.md` was produced by `/gsd-secure-phase` (`4205454`), as the cut intended.
- No review, UAT or user message asked for any of them.

## 2. Process review

Cost is `calibration.json` → `by_agent_type`: tokens excluding cache reads, for the whole project history in the transcripts.

| gate / step | cost | caught this milestone |
|---|---|---|
| gsd-code-reviewer + /gsd-code-review + gsd-code-fixer | 3.95M + 1.72M + 1.41M | in every reviewed phase: 15 (4 crit/17 warn), 17 (6/17), 18 (4/12), 20 (1/9), 21 (2/6), 27 (1/9), 28 (0/6), 28.1 (0/7), 29 (0/7), 30 (0/3) |
| gsd-verifier | 2.75M / 30 runs | `gaps_found` in 21 and 28.1 (2 blockers, `ef8cf54`); `human_needed` in 19 and 28.1 |
| gsd-security-auditor + /gsd-secure-phase | 1.57M + 0.65M | 19 blocked with 6 open threats; 28.1 opened T-28.1-02, which was then closed |
| gsd-plan-checker | 2.25M / 26 runs | 19: about 12 plan-check commits with fixes (19-20..19-32). 24-30: no revision commits found |
| gsd-ui-auditor + /gsd-ui-review | 0.48M | 28 scored 18/24 with 5 fixes; fix lists in 28.1 (13) and 30 (3) |
| /gsd-verify-work (UAT) | 0.69M | 19: 3 of 5 passed; 14, 21 and 28.1 still `testing`; 26.1, 28, 29 and 30 all passed |
| gsd-phase-researcher | 4.59M / 12 runs (p50 396k) | no catch artifact. Memory rule: research runs on Opus, and Sonnet research gets redone |
| /gsd-discuss-phase | 0.97M / 8 runs | unattended: most decisions in CONTEXT.md are `[inferred]` (30: 26 of 29 D-items; 29: 23 of 26) |

Other checks:
- **Thin plans:** none. Phases 28.1, 29 and 30 average 2.7–3 tasks per plan. `musk audit` flags no merge candidates.
- **Lean lane:** no phase qualified (`quick_eligible=false` for 26–30 and 28.1).
- **Granularity** is already `coarse`. Per-plan overhead (p50 168k) is about 8 times the per-task cost (p50 21k), so plan count drives cost. Phase 28.1 had 14 plans for 38 tasks.
- **Discuss defaults:** the user is unavailable by design, so discuss answers are inferred rather than defaulted. At about 1% of spend, with its D-items cited downstream, it is not worth cutting.

### Suggested config changes

None. Every gate with a cost caught something this milestone, so none qualifies for removal: plan-check caught issues in 19; verifier, security, code review and UI review all caught issues. Security enforcement is never a candidate.

Non-config follow-ups:
- Run `/gsd-musk-roadmap` over phases 31 and 32 before they are planned. Scope cuts are 0 because the milestone pass never ran.
- Planners: don't log a doc line as a cut when it is only deferred to the next workflow step. Both reinstatements this milestone were that pattern.

Run `gsd-tools musk stats` for current figures.
