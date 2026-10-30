---
phase: 31-retire-superseded-envelope-goal-layer-code
verified: 2026-10-29T00:00:00Z
status: passed
score: 5/5 must-haves verified
covered_files:
  - .planning/phases/31-retire-superseded-envelope-goal-layer-code/31-01-PLAN.md
  - .planning/phases/31-retire-superseded-envelope-goal-layer-code/31-01-SUMMARY.md
  - .planning/phases/31-retire-superseded-envelope-goal-layer-code/31-02-PLAN.md
  - .planning/phases/31-retire-superseded-envelope-goal-layer-code/31-02-SUMMARY.md
  - .planning/phases/31-retire-superseded-envelope-goal-layer-code/31-03-PLAN.md
  - .planning/phases/31-retire-superseded-envelope-goal-layer-code/31-03-SUMMARY.md
  - .planning/phases/31-retire-superseded-envelope-goal-layer-code/31-04-PLAN.md
  - .planning/phases/31-retire-superseded-envelope-goal-layer-code/31-04-SUMMARY.md
  - .planning/phases/31-retire-superseded-envelope-goal-layer-code/31-05-PLAN.md
  - .planning/phases/31-retire-superseded-envelope-goal-layer-code/31-05-SUMMARY.md
  - .planning/phases/31-retire-superseded-envelope-goal-layer-code/31-06-PLAN.md
  - .planning/phases/31-retire-superseded-envelope-goal-layer-code/31-06-SUMMARY.md
  - .planning/phases/31-retire-superseded-envelope-goal-layer-code/31-07-PLAN.md
  - .planning/phases/31-retire-superseded-envelope-goal-layer-code/31-07-SUMMARY.md
  - .planning/phases/31-retire-superseded-envelope-goal-layer-code/31-08-PLAN.md
  - .planning/phases/31-retire-superseded-envelope-goal-layer-code/31-08-SUMMARY.md
  - .planning/phases/31-retire-superseded-envelope-goal-layer-code/31-09-PLAN.md
  - .planning/phases/31-retire-superseded-envelope-goal-layer-code/31-09-SUMMARY.md
  - Cargo.toml
  - src/config.rs
  - src/driver/kill.rs
  - src/driver/lock.rs
  - src/executor/mod.rs
  - src/launcher/config_dir.rs
  - src/launcher/process.rs
  - src/main_loop.rs
  - src/registry.rs
  - src/session_detector.rs
covered_digest: "v4:sha256:bc94a1c2e800755cba64b6f85b000e229b3afbf781c84dbb894766daabf77586"
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: passed
  previous_score: 5/5
  gaps_closed: []
  gaps_remaining: []
  regressions: []
advisory:
  - finding: "Code re-review (31-REVIEW.md, 0 critical / 3 warning / 3 info) leaves findings deferred; all six are duplicates of Phase 28-30 findings"
    category: other
    reason: "SAFE-04 pump multi-line PEM, pump thread spawn failure, config_dir silent drop of wrong-shape policy values, orphaned RUNTIME_KEY const; none competes with the sandboxed launcher as a safety model"
    evidence_status: "no failing test or reproducible breakage of an SC"
---

# Phase 31: Retire Superseded Envelope & Goal-Layer Code - Verification Report

**Phase Goal:** Code from superseded Phases 19, 21 and 22 is either removed or rewired into the new path, so the codebase carries no unfinished parallel safety model.
**Verified:** 2026-10-29 (HEAD c1933bd)
**Status:** passed
**Re-verification:** Yes. The prior report (HEAD 0844ee8) went stale because covered `src/executor/mod.rs` changed. Between 0844ee8 and HEAD, quick batches 261028-t3c / 261028-v2z and quick 261029-1dp changed lock/state open hardening, `config_dir` merge fail-closed, the SAFE-04 redacting pump in `src/launcher/process.rs`, and the `AgentRuntime` key removal. `git diff --stat 765c9d2..HEAD -- src tests Cargo.toml Cargo.lock scripts` is empty: every commit after the last code change is `.planning/`-only, so the orchestrator's test evidence at 765c9d2 and container evidence at 75e6ffc describe the HEAD source tree.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | SC1: Phase 19 guard + carrier corpora, Phase 21 goal-layer seams and stream-json injection deleted; namespace refusal, secret scan, PR ledger, SAFE-04, `Untrusted`, router kept and rewired | VERIFIED | `src/envelope/` = mod, policy, scan, ledger only; `src/driver/` = direction, kill, lock, liveness, mod, rate_limit, router, steer. No guard/classifier/goal-layer files. `grep -rnE "CodexExecutor\|ClaudeExecutor\|ExecEvent\|DrivableProject\|build_codex_argv\|exec_tx\|process_wrap\|process-wrap"` over src, tests, scripts, Cargo.toml, Cargo.lock: zero hits at HEAD. SAFE-04 is *used* by the new pump (`launcher/process.rs` imports `journal::redact::{redact, redact_value}`), i.e. rewired, not duplicated. |
| 2 | SC2: Phase 22 plans archived as superseded, `bollard` never added, `GSDMM_EXPERIMENTAL_FEATURES` gates only live surfaces | VERIFIED | `22-01..04-PLAN.md` carry `status: superseded` / `superseded_by: phase-28`; `22-SUPERSEDED.md` present. `grep -c bollard Cargo.toml Cargo.lock` = 0 / 0. The flag has one production read (`src/app.rs:812`, parsed in `src/experimental.rs`) stored on `AppContext::experimental`. |
| 3 | SC3: T-19-86/91/111/116 and Phase 21 criterion 4 closed with pointers in `deferred-items.md` | VERIFIED | `.planning/phases/19-*/deferred-items.md:3232` "Phase 31 closure (2026-10-25)". `31-SECURITY.md` `status: verified`, `threats_open: 0`, 34/34 closed, re-audited at 0162e1e after the quick batches. |
| 4 | SC4: `./scripts/pre-tag-check.sh --container` passes on the reduced tree | VERIFIED | Orchestrator run at HEAD 75e6ffc (later commits `.planning/`-only, confirmed by empty src diff above): `--container v1.15.0` exit 0, inner run exit 0, all six gates PASS (tag/version, MSRV, release build, tests, clippy `-D warnings`, `cargo package --locked`), 55 `test result:` lines, 3216 passed / 0 failed. Log: `/tmp/claude-1000/-home-blk-projects-rust-gsd-meta-manager/30911796-0f3c-4659-8e08-ce10f00ad0d8/scratchpad/container-31-r2.log`. Not re-run by me: the source tree is byte-identical to the one the orchestrator ran, so a re-run adds no evidence. |
| 5 | D-04 / 31-07 delivered: Codex executor, in-process Claude transport, TUI exec channel and `process-wrap` removed | VERIFIED | `src/executor/` = `mod.rs` + `stream_json.rs`; `runtime.rs` is gone (261029-1dp deleted the dead `AgentRuntime`, 151 lines). `process-wrap` absent from Cargo.toml/Cargo.lock. |

**Score:** 5/5 truths verified (4 roadmap SCs + D-04).

### Re-verification focus: do the later changes reintroduce a parallel safety model or undo a RETIRE deliverable?

| Change | Check | Verdict |
|--------|-------|---------|
| Lock/state open hardening (`driver/lock.rs`, `driverctl/store.rs`, `direction.rs`) | Adds hardening to the existing single outside lock (`ensure_lock_file` is the only new `pub fn`); no new spawn-allow/deny decision path. | Hardens the kept Phase 28 mechanism; not a second model. |
| `config_dir` merge fail-closed (`launcher/config_dir.rs`) | Launcher-side per-run config policy in the Phase 28 path; no executor or guard code revived. | Part of the new path. |
| SAFE-04 redacting pump (`launcher/process.rs`) | Reuses `journal::redact::{redact, redact_value}` (the kept SAFE-04 table). `pump_redacted` redacts whole lines before they reach the capture file. | Rewires SAFE-04 into the launcher: exactly the RETIRE-01 "kept and rewired" intent. |
| `AgentRuntime` key removal (`config.rs`, `registry.rs`, `executor/mod.rs`) | `grep AgentRuntime src tests`: only comments / a test note remain. Legacy `runtime` and `default_runtime` keys stay in the flattened `extra` maps and round-trip unchanged, selecting nothing. Census assertion re-measured down (5 -> 4 subjects), not relaxed. | Removes a retired-model remnant. |
| CTRL-02/03 supersession (31-CONTEXT.md, REQUIREMENTS.md lines 24, 49-50, 331-332) | `rtk grep -rE "dry_run\|DryRun\|dry-run" src`: no production hit (one test-comment). `is_opted_in` / opt-in records feed only `launcher::push::policy_for` (PR caps/defaults); nothing gates a launch on them. `registry::is_opted_in` docs now say "not a gate". | The retired drive dry-run and opt-in gate stay retired; the doc matches the code. |

No competing safety model is present: the only enforcement surfaces are the Phase 28/28.1 sandbox + outside lock, the kept SAFE-04 redactor, the kept namespace refusal / secret scan / ledger in `src/envelope/`, and DIRECTION.md stop rules.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Full suite (orchestrator, local) | `rtk proxy cargo test --no-fail-fast` at 765c9d2 | 55 suites, 3216 passed, 0 failed | PASS (supplied) |
| Full suite + clippy + package inside the container | `pre-tag-check.sh --container v1.15.0` at 75e6ffc | exit 0, six gates PASS, 3216 passed / 0 failed | PASS (supplied) |
| Source tree unchanged since the tested tree | `git diff --stat 765c9d2..HEAD -- src tests Cargo.toml Cargo.lock scripts` | empty | PASS (run by me) |
| Retired-symbol sweep | `grep -rnE "CodexExecutor\|...\|process-wrap" src tests scripts Cargo.toml Cargo.lock` | no hits | PASS (run by me) |

Passed count rose 3183 -> 3216 since the prior report from the added hardening tests; 0 failures.

### Requirements Coverage

| Requirement | Source Plans | Status | Evidence |
|-------------|--------------|--------|----------|
| RETIRE-01 | 31-01..05, 07, 08, 09 | SATISFIED | SC1 and D-04. `REQUIREMENTS.md:285` `[x]`; traceability line 411 Complete. |
| RETIRE-02 | 31-02, 03, 04, 06 | SATISFIED | SC2. `REQUIREMENTS.md:292` `[x]`; traceability line 412 Complete. |
| RETIRE-03 | 31-05, 06, 07, 09 | SATISFIED | SC3, SC4. `REQUIREMENTS.md:293` `[x]`; traceability line 413 Complete. |

All three IDs are accounted for in REQUIREMENTS.md and map only to Phase 31: no orphans. CTRL-02/CTRL-03 (Phase 17) are recorded as superseded by user decision on 2026-10-29 in REQUIREMENTS.md and 31-CONTEXT.md; they are not Phase 31 IDs and do not block RETIRE-*.

### Code review (31-REVIEW.md at 7999a45; disposition c1933bd)

0 critical / 3 warning / 3 info, all six duplicates of Phase 28-30 findings (SAFE-04 pump multi-line PEM, pump thread-spawn failure, `config_dir` silent drop of wrong-shape policy values, orphaned `RUNTIME_KEY` const, etc.), dispositioned deferred. None is a retired-model remnant or a competing decision path; they are hardening/hygiene in kept code. Recorded as advisory only [inferred - cleanup 2026-10-29]. The older deferred items (test-only `check_prompt_input_drift` / `stream_json` remnants, advisory unregister guard) remain tracked by pending todos and are hygiene, not a second path that can allow or deny a spawn.

### Anti-Patterns Found

None blocking. No debt markers introduced in the touched covered files per the review gate.

### Human Verification Required

None. 31-UI-REVIEW.md (19/24, refreshed 56dcd3a) reports no UI-facing change in this window and is advisory.

### Gaps Summary

No gaps. All four roadmap success criteria and the D-04 deliverable hold at HEAD c1933bd. The later quick work hardened kept Phase 28 launcher/lock code, rewired SAFE-04 into the launcher pump and removed one more retired-model remnant (`AgentRuntime`); nothing reintroduces a parallel safety model or reverses a RETIRE deliverable.

---

_Verified: 2026-10-29_
_Verifier: Claude (gsd-verifier)_
