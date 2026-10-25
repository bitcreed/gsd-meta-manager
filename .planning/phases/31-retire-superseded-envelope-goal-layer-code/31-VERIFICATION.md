---
phase: 31-retire-superseded-envelope-goal-layer-code
verified: 2026-10-25T00:00:00Z
status: gaps_found
score: 3/4 must-haves verified
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
  - scripts/pre-tag-check.sh
  - src/cli.rs
  - src/driver/mod.rs
  - src/envelope/mod.rs
  - src/envelope/policy.rs
  - src/launcher/push.rs
covered_digest: "v3:sha256:044bdd100429312735875dd9ed94e5f8e4a11d344612ae1c28b959accb7c5181"
behavior_unverified: 0
overrides_applied: 0
gaps:
  - truth: "SC4 / RETIRE-03: `./scripts/pre-tag-check.sh --container` passes on the reduced tree"
    status: failed
    reason: "Container dry run `--container v1.15.0` exited 1 (gates 1,2,3,5,6 PASS; gate 4 tests FAIL) on two launcher tests that are environment-dependent in the container image. They fail identically on the pre-phase tree 235a912 and no Phase 31 commit touched them, so Phase 31 did not cause the red gate, but the stated success criterion is not met."
    artifacts:
      - path: "src/launcher/push/planted.rs"
        issue: "test `a_truncation_the_baseline_had_too_is_listed_but_does_not_block_the_push` (line ~1343) depends on read_dir order; fails under overlayfs (introduced by 2d5ab57 fix(28.1) WR-08)"
      - path: "tests/launcher_sandbox.rs"
        issue: "test `no_flag_names_a_credential_directory_and_the_home_block_precedes_the_run_binds` (line ~305) expects /run/user/<uid> to exist; the container has none (--tmpfs /run/user/1000 missing)"
    missing:
      - "Make both tests independent of host directory order and of /run/user/<uid> (a 28.1 follow-up), then re-run `./scripts/pre-tag-check.sh --container v1.15.0` to exit 0"
      - "Alternative: the operator accepts the two container-only failures as a recorded override (risk acceptance is the operator's to sign)"
deferred:
  - truth: "D-04 / Plan 31-07: Codex executor, in-process Claude transport, their fixtures/census rows and the `process-wrap` dependency are removed"
    addressed_in: "Held pending user decision (orchestrator directive); not a later roadmap phase"
    evidence: "31-07-PLAN.md present without a SUMMARY; src/executor/{claude,codex,codex_json,stream_json}.rs and `process-wrap` in Cargo.toml/Cargo.lock remain; `#[allow(dead_code)]` on `with_runtime` at src/executor/mod.rs:284"
---

# Phase 31: Retire Superseded Envelope & Goal-Layer Code - Verification Report

**Phase Goal:** Code from superseded Phases 19, 21 and 22 is either removed or rewired into the new path, so the codebase carries no unfinished parallel safety model.
**Verified:** 2026-10-25
**Status:** gaps_found
**Re-verification:** No - initial verification

## Goal Achievement

### Observable Truths (ROADMAP success criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Phase 19 PreToolUse guard + carrier corpora, Phase 21 goal-layer seams, and stream-json durable-injection are deleted; namespace refusal, secret scan, PR ledger, SAFE-04 redaction, `Untrusted`, router are kept and rewired | VERIFIED | `src/envelope/` is now `mod.rs, policy.rs (18K, namespace + EnvelopePolicy + 3-reason ParkReason only), scan.rs, ledger.rs`; `hooks.rs`, `cred.rs`, `advisory.rs` gone. `src/driver/` has no `goal/escalate/untrusted/run/dry_run/spawn`; `journal/inbox.rs`, `driver_inject.rs`, `driver_start.rs` gone; no `tests/envelope_*` except `envelope_pr_cap.rs`, no driver_goal/injection/escalation/tracer/iteration suites. Zero non-comment hits for `classify_git`, `GitVerdict`, `write_settings_in`, `GUARD_TIMEOUT`, `DriverInject`, `GoalRefused`, `EscalationRefused`, `CONFIG_SECTION_CONSTANTS_DERIVED`. Remaining `PreToolUse` hits are the Phase 28.1 nested-agent gate (`launcher/config_dir.rs:322`, `launcher/gate.rs`, `driverctl_cli.rs`), not the Phase 19 guard. Hidden CLI reduced to `envelope scan` only (`src/cli.rs:131,469`). Rewiring: `launcher/push.rs:247-248` `redact_captured` = `redact_urls` then `journal::redact::redact` (SAFE-04 on persisted child stderr, lines 332, 890); push.rs uses ledger/policy/scan; router, `Untrusted` (`text.rs`), `driver::{lock,kill,rate_limit,steer}` kept. `cargo clippy --all-targets --locked -- -D warnings` clean (re-run by verifier). |
| 2 | Phase 22 plans archived as superseded; `bollard` never added; `GSDMM_EXPERIMENTAL_FEATURES` gates only surfaces that still exist | VERIFIED | `22-01..04-PLAN.md` carry `status: superseded` / `superseded_by: phase-28`; `22-SUPERSEDED.md` present. `grep bollard Cargo.toml Cargo.lock scripts tests .github` = 0 hits. Remaining `ctx.experimental` sites in `src/app.rs` (989 `stop_launcher_run`, 1041 `launch_target`, 1169 `request_new_run`, 1411 `steering_target`, 2620) all gate launcher/steer surfaces; `start_driver_run`/`stop_driver_run` no longer exist. README/CONFIGURATION describe the Driver tab and launcher only. Note: nothing enforces "bollard never added" mechanically (D-14 left a gate optional). |
| 3 | Open threat ids closed with pointers, each named in `deferred-items.md`: T-19-86, T-19-91, T-19-111, T-19-116 and Phase 21 criterion 4 | VERIFIED | `19-*/deferred-items.md` "Phase 31 closure (2026-10-25)" has 27 rows incl. T-19-86/91/111/116 (and 112/121), classes G/N/K, N rows carry "reopens against Phase 28.1 if the D-11 live probe fails". `19-SECURITY.md`: `status: superseded`, `threats_open: 0`, `register_totals_after_phase_31: 139 total / 137 closed / 2 open / 0 >= high`; the 2 open (T-19-72/73, low, surviving code) have a backlog owner todo (honest K-class). `21-*/deferred-items.md` "Phase 31 closure" closes criterion 4 as superseded; CR-01/CR-02 correctly NOT superseded, owner todo `2026-10-24-session-detector-resume-optional-arity.md`. REQUIREMENTS SAFE-02/05/07/08 rows updated. |
| 4 | `./scripts/pre-tag-check.sh --container` passes on the reduced tree | FAILED | Run recorded in 31-05-SUMMARY.md: `--container v1.15.0` exit 1; gates 1,2,3,5,6 PASS (gate 6 `cargo package --locked` OK, 282 files, `driver_router_conformance` 6/6 with oracle), gate 4 FAIL on 2 tests (see gaps). Attribution verified by verifier: `git diff 235a912 HEAD` over `src/launcher/push/planted.rs`, `src/launcher/sandbox.rs`, `tests/launcher_sandbox.rs` is empty (Phase 31 did not touch them); both tests fail identically on `git archive 235a912` per 31-05. Local evidence is green: orchestrator `cargo test --no-fail-fast` log = 57 binaries, 3290 passed, 0 failed, 17 ignored (verifier re-summed the raw log); clippy clean. The criterion's own wording is exit 0, which was not achieved. |

**Score:** 3/4 truths verified (excluding deferred D-04 / 31-07).

### Deferred (held pending user decision)

| Item | Status | Evidence |
|------|--------|----------|
| D-04 / 31-07: delete the Codex executor, in-process Claude transport, their fixtures/census rows and `process-wrap`; re-run container gate on the final tree | HELD by user; not counted against the score | `31-07-PLAN.md` has no SUMMARY. Expected residue present: `src/executor/{claude,codex,codex_json,stream_json,gate,outcome,mod}.rs`, `process-wrap` in Cargo.toml, `envelope_disallowed_tools` plumbing in `executor/{mod,claude,codex}.rs`, narrow `#[allow(dead_code)]` on `DrivableProject::with_runtime` (`executor/mod.rs:284`). This is the only remaining callerless code under `src/executor/`. When 31-07 runs, its container re-run is the natural place to also close the SC4 gap. |

### Required Artifacts / Key Links

| Artifact / link | Status | Details |
|---|---|---|
| `src/envelope/{mod,policy,scan,ledger}.rs` kept; `launcher/push.rs` imports ledger, policy, scan | VERIFIED | `policy.rs` exports only namespace items, `PushVerdict`, 3-variant `ParkReason`, PR-cap constants, `EnvelopePolicy` |
| push step stderr -> `journal::redact` | VERIFIED | `push.rs:247-248,332,890` |
| `driver/mod.rs` surviving set | VERIFIED | direction, steer, router, rate_limit, liveness, kill, lock only |
| `22-SUPERSEDED.md`, Phase 22 plan frontmatter | VERIFIED | present, all four plans |
| `19-*/deferred-items.md`, `19-SECURITY.md`, `21-*/deferred-items.md` closure sections | VERIFIED | see truth 3 |
| `scripts/pre-tag-check.sh` witness/banner retired | VERIFIED | ran all six gates in container; no `CONFIG_SECTION_CONSTANTS` references remain in src |

### Requirements Coverage

| Requirement | Source Plans | Description | Status | Evidence |
|---|---|---|---|---|
| RETIRE-01 | 31-01..05 (07 also declares, deferred) | Superseded paths removed; survivors rewired | SATISFIED (D-04 residue deferred) | Truth 1. Executor remnants are launcher-independent legacy code, not a parallel safety model: no guard, goal layer or injection path remains |
| RETIRE-02 | 31-02, 31-03, 31-04, 31-06 | Phase 22 archived; `bollard` never added | SATISFIED | Truth 2 |
| RETIRE-03 | 31-05, 31-06 (07 also declares) | Threat ids and criterion 4 closed with pointers; container gate passes | PARTIAL - BLOCKED on container clause | Truth 3 satisfied; Truth 4 failed |

All three IDs appear in plan frontmatter and in REQUIREMENTS.md (lines 283-291, traceability rows 409-411, still "Pending": to be flipped by the orchestrator per D-17 once the gate is accepted or green). No orphaned IDs.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|---|---|---|---|---|
| `src/executor/mod.rs` | 284 | `#[allow(dead_code)]` on `with_runtime` | Info | Expected consequence of the held D-04; narrow and commented |
| debt markers (TBD/FIXME/XXX) in files touched | - | none found in the spot checks of kept envelope/driver modules | Info | - |

Advisory artifacts: 31-REVIEW.md (0 critical, 5 warning, dispositioned), 31-SECURITY.md (threats_open 0, T-31-04 medium open), 31-UI-REVIEW.md 16/24. None changes the verdict.

### Human Verification Required

None that gate this verdict. The 28.1 D-11 live probe (28.1-UAT.md test 1) remains pending from Phase 28.1 and is the stated reopening condition for the N-class closures; it is not new to this phase.

### Gaps Summary

One gap: SC4 / RETIRE-03's container clause. The gate is red for two pre-existing, container-environment-dependent Phase 28.1 launcher tests (read_dir order under overlayfs; missing `/run/user/1000`). Phase 31 neither introduced nor touched them, and everything Phase 31 changed (gate 6 packaging, clippy, 3290 local tests, router conformance with oracle) is green. The honest closure is a small 28.1 follow-up making those two tests host-independent, then a repeat container run (which can double as 31-07's final-tree run). If the operator prefers to accept the two failures as pre-existing and environmental, record an `overrides:` entry; that acceptance is theirs to sign.

---

_Verified: 2026-10-25_
_Verifier: Claude (gsd-verifier)_
