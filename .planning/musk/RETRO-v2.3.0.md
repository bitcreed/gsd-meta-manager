# Musk retro: v2.3.0 Multiplexer Backends

Run: 2026-11-10 (`/gsd-musk-retro`, unattended, at milestone close). The milestone is Phase 33 (multiplexer backend seam + herdr support, plus its code-review fixes) and quick task 261110-qap (`status --json` + `--skill`). Decisions below were made without the user and are marked [inferred].

## 1. Numbers

Calibration (`musk calibrate`, basis `harness`): 20 sessions, 628 subagents, 117.92M tokens (+3617.90M cache read).
Prices: per_task p50 19k (n=71), per_plan_overhead p50 162k (n=71), per_phase p50 6.84M (n=13), per_quick_task p50 489k (n=59).

### Cuts

This milestone (Phase 33, `musk stats --phase 33`):

| class | applied | proposed | reinstated | active |
|---|---|---|---|---|
| scope | 0 | 0 | 0 | 0 |
| impl | 4 | 0 | 0 | 4 |
| ceremony | 0 | 0 | 0 | 0 |

The milestone pass (`MILESTONE-v2.3.0.md`) reviewed 9 items and cut none: the milestone is one user-requested phase whose seven requirements all trace to the user's ask or to a security mitigation.

Project ledger (all milestones): 47 cuts applied (46 impl, 1 scope), 0 proposed, 4 reinstated.

**Add-back ratio:** 0 / 4 for this milestone (too few cuts to read), **4 / 47 = 8.5%** project-wide. That is in the healthy range.

### Estimated tokens (basis `harness`)

This milestone, n = 4 active cuts:

| | p25 | p50 | p75 |
|---|---|---|---|
| gross saved | 0 | 74,740 | 283,636 |
| pass cost | 20,140 | 20,140 | 20,140 |
| **net** | **-20,140** | **54,600** | **263,496** |

Project-wide, n = 43 active cuts: gross p25 1.75M / p50 7.70M / p75 14.87M; pass cost 430k (measured: 20k fragments + 410k skills, 7 discuss + 43 plan injections); **net p25 1.32M / p50 7.26M / p75 14.44M**. Most of that is one cut: M-0028, the user's 2026-10-28 deferral of Phase 32, which is priced at a whole phase (per_phase p50 6.84M). Without it the impl cuts are worth well under 1M at p50.

### Unreviewed cuts, settled from history

There were 21 unreviewed impl cuts (M-0025..M-0027 from Phase 31, M-0029..M-0042 from v2.1/v2.2 quick tasks, M-0044..M-0047 from Phase 33). No scope cut was unreviewed, so nothing needed the user.

- **Reinstated (rebuilt): M-0040.** quick-261103-tn4 left the `futures` dep and crossterm's `event-stream` feature unused, and deferred their removal to the release dep pass. That pass happened in this release: commit 1f5f404 removed both. [inferred]
- **Held, confirmed (20):** M-0025, M-0026, M-0027, M-0029..M-0039, M-0041, M-0042, M-0044..M-0047. Checked by git log and grep over src/ and tests/ at de74b06:
  - `tests/no_container_crate.rs` is absent, and bollard appears 0 times in Cargo.toml and Cargo.lock.
  - `credential: None` and `CredentialSource` are unchanged. There is no runtime-dir resolver seam.
  - The marker-scan in-flight todo is still pending.
  - The one-off flow has no `--ack-planted`, no PR opening, no retention keys and no compression crate. The only `--ack-planted` is `driver push`'s, from 28.1, which predates the cut.
  - There is no pty editor end-to-end test and no drain on resume. `OneoffRunStarted` still carries only `{alias, result}`.
  - `herdr.rs` documents the no-tty pid tier, has no tab-close or workspace-not-found retry, and the ps ancestry walk is still Linux-only.
  - No review, UAT or user message asked for any of these.

## 2. Process review (steps 4 and 5)

Phase 33 gates and what they caught:

| gate | cost (by_agent_type p50/run) | caught |
|---|---|---|
| research (gsd-phase-researcher) | 413k | yes. Facts E2 and E3 (herdr has no tty; layout.apply is atomic) let three planned tasks be cut safely (M-0044, M-0045, M-0047). |
| plan-check (gsd-plan-checker) | 63k | 0 revisions in Phase 33, and 0 `revise` commits in v2.3.0. Project-wide, only 2 revision commits ever. |
| verifier | 76k | passed, 7/7 requirements and 20/20 truths, with no gaps. |
| code review | 185k (+86k orchestrator) | yes. 3 warnings (WR-01/02 were real herdr focus mis-targets; WR-03 was dead API) and 5 info. All were fixed. |
| UI review (gsd-ui-auditor) | 63k | yes, on a backend phase. UI-01 found a raw backend error leaking into the status line, and it was fixed in 781a36b. |
| security | ~52k orchestrator | threats_open 0. The auditor was correctly not spawned. |

Other checks:
- **Plan size:** 2 plans with 5 tasks, an average of 2.5 per plan. `musk audit --phase 33` flagged no merge candidates.
- **Lean lane:** `quick_eligible=false` (10 files), so the standard lane was correct.
- **Discuss:** this ran `--auto` because the user was unavailable, so every choice was an inferred default. That says nothing about whether the user would accept defaults, so it is not evidence for `skip_discuss`.

### Suggested config changes

None. The one candidate is `workflow.plan_check`: it had 0 revisions this milestone and only 2 revision commits in the project's history, at about 63k per run. But this milestone had a single phase, which is too little evidence to switch it off. Revisit it at the next multi-phase retro:

- `gsd-tools config-set workflow.plan_check false`: only if the next milestone also shows 0 plan revisions across several phases.

Every other gate caught something in this milestone, so none of them is a candidate.
