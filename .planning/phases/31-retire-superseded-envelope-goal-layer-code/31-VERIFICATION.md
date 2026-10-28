---
phase: 31-retire-superseded-envelope-goal-layer-code
verified: 2026-10-27T00:00:00Z
status: passed
score: 4/4 must-haves verified
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
  - .planning/phases/31-retire-superseded-envelope-goal-layer-code/31-08-PLAN.md
  - .planning/phases/31-retire-superseded-envelope-goal-layer-code/31-08-SUMMARY.md
  - .planning/phases/31-retire-superseded-envelope-goal-layer-code/31-09-PLAN.md
  - .planning/phases/31-retire-superseded-envelope-goal-layer-code/31-09-SUMMARY.md
  - scripts/pre-tag-check.sh
  - src/cli.rs
  - src/envelope/ledger.rs
  - src/envelope/policy.rs
  - src/launcher/process.rs
  - src/launcher/push.rs
  - src/launcher/push/planted.rs
  - tests/launcher_sandbox.rs
covered_digest: "v4:sha256:d2e4e7778d1db3c9e255cf1bad48478105ab11a27542a9538691bb5e5eace819"
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: gaps_found
  previous_score: 3/4
  gaps_closed:
    - "SC4 / RETIRE-03: `./scripts/pre-tag-check.sh --container` passes on the reduced tree"
  gaps_remaining: []
  regressions: []
deferred:
  - truth: "D-04 / Plan 31-07: Codex executor, in-process Claude transport, their fixtures/census rows and the `process-wrap` dependency are removed"
    addressed_in: "Held by user decision (droppable); not a later roadmap phase and not a ROADMAP success criterion"
    evidence: "31-07-PLAN.md has no SUMMARY; src/executor/{claude,codex,codex_json,stream_json}.rs and `process-wrap` (Cargo.toml:58) remain; `#[allow(dead_code)]` at src/executor/mod.rs:284"
---

# Phase 31: Retire Superseded Envelope & Goal-Layer Code - Verification Report

**Phase Goal:** Code from superseded Phases 19, 21 and 22 is either removed or rewired into the new path, so the codebase carries no unfinished parallel safety model.
**Verified:** 2026-10-27
**Status:** passed
**Re-verification:** Yes - after gap closure (31-08, 31-09). Prior status gaps_found on SC4 only.

## Goal Achievement

### Observable Truths (ROADMAP success criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | Phase 19 guard + carrier corpora, Phase 21 goal-layer seams, stream-json injection deleted; namespace refusal, secret scan, PR ledger, SAFE-04, `Untrusted`, router kept and rewired | VERIFIED (regression check) | `src/envelope/` = `mod, policy, scan, ledger` only; `src/driver/` = direction, kill, lock, liveness, mod, rate_limit, router, steer; `git diff --stat 235a912 HEAD` shows 61 files / -59366 lines. `redact_captured` (push.rs:249-255) = SAFE-04 redaction then a 512-char cap. `cargo clippy --all-targets --locked -- -D warnings` exit 0 (verifier run). No TBD/FIXME/XXX in the touched launcher/envelope/driver files. |
| 2 | Phase 22 plans archived as superseded, `bollard` never added, experimental flag gates only live surfaces | VERIFIED (regression check) | `grep -c bollard Cargo.toml Cargo.lock` = 0 / 0. Unchanged since the prior verification (no code commit touches this). |
| 3 | Open threat ids closed with pointers in `deferred-items.md` (T-19-86/91/111/116, Phase 21 criterion 4) | VERIFIED (regression check) | Only `.planning` docs commits (db8746c, 12fe732, f0eb8b8, 7e414f9) since the prior verification; `31-SECURITY.md` `status: verified`, `threats_open: 0`. |
| 4 | `./scripts/pre-tag-check.sh --container` passes on the reduced tree | VERIFIED | 31-09 ran `--container v1.15.0` cold: exit 0, gates 1-6 PASS, 57 `test result: ok` lines. The two prior blockers are fixed at the source: `list_dir_bounded` (planted.rs:267) gives name-ordered, bounded walks; `tests/launcher_sandbox.rs:299` pins `spec.runtime_dir` to `RUNTIME_DIR` instead of reading host `XDG_RUNTIME_DIR`. No assertion weakened, no new `#[ignore]`. `git diff 69d371d HEAD -- src tests scripts Cargo.toml Cargo.lock` is empty, so the green container run covers the HEAD code. I did not re-run the container (needs a runtime and a cold multi-GB build); the local equivalent was re-run (below). |

**Score:** 4/4 truths verified (excluding held D-04 / 31-07).

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Full suite at HEAD | `rtk proxy cargo test --no-fail-fast > log` | exit 0; 57 `test result:` lines; passed=3294 failed=0 ignored=17 | PASS |
| Lint clean | `rtk proxy cargo clippy --all-targets --locked -- -D warnings` | exit 0 | PASS |
| No tag/publish | `git tag --points-at HEAD` | empty | PASS |

### Deferred (held by user decision)

| Item | Status | Evidence |
|------|--------|----------|
| D-04 / 31-07: delete Codex executor, in-process Claude transport, `process-wrap` | HELD, droppable; not counted against the score | Not a ROADMAP success criterion. CONTEXT D-04 targets the legacy `drive` Codex path, not a safety model. Residue: `src/executor/*`, `process-wrap`, `envelope_disallowed_tools` plumbing in `executor/{mod,claude,codex}.rs` (an unused-by-launcher tool denylist option, not an active guard) and one narrow `#[allow(dead_code)]` (executor/mod.rs:284). No guard, goal layer, injection path or second safety decision-maker remains, so the goal sentence holds. |

### Requirements Coverage

| Requirement | Source Plans | Status | Evidence |
|---|---|---|---|
| RETIRE-01 | 31-01..05, 31-08, 31-09 (07 held) | SATISFIED | Truth 1; WR-01..03 stale-control text and unregister guard fixed by 31-08; WR-04/05 by 31-09 |
| RETIRE-02 | 31-02, 31-03, 31-04, 31-06 | SATISFIED | Truth 2 |
| RETIRE-03 | 31-05, 31-06, 31-09 | SATISFIED | Truths 3 and 4 (container clause now green) |

All three IDs are in plan frontmatter and in REQUIREMENTS.md (lines 283-291; traceability rows 409-411). No orphaned IDs. The checkboxes and "Pending" rows are still unflipped; that is the orchestrator's D-17 step now that the gate is green.

### Anti-Patterns Found

| File | Line | Pattern | Severity | Impact |
|---|---|---|---|---|
| `src/executor/mod.rs` | 284 | `#[allow(dead_code)]` on `with_runtime` | Info | Consequence of held D-04 |
| 31-REVIEW.md | - | 0 critical / 0 warning / 6 info (IN-xx). 31-REVIEW-DISPOSITION.md frontmatter reads `open: 10` because it carries 4 older INs not in the latest review; none block | Info | Cosmetic stale-comment/naming items |

### Human Verification Required

None. The Phase 28.1 D-11 live probe remains the stated reopening condition for N-class threat closures; it is a Phase 28.1 item, not new here.

### Gaps Summary

None. The single prior gap (SC4 container gate) is closed by 31-09 at the source, and no code has changed since the green container run. Residual notes, none gating: (1) 31-07 stays held; (2) the planted-baseline residual accepted in 31-09 (a run launched before the walk-order change may list extra created/removed entries on its first push after upgrade); (3) REQUIREMENTS.md RETIRE-01..03 need flipping to complete.

---

_Verified: 2026-10-27_
_Verifier: Claude (gsd-verifier)_
