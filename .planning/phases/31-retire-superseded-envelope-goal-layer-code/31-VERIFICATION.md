---
phase: 31-retire-superseded-envelope-goal-layer-code
verified: 2026-10-28T00:00:00Z
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
  - src/driver/kill.rs
  - src/executor/mod.rs
  - src/main_loop.rs
  - src/session_detector.rs
covered_digest: "v4:sha256:d1da96ac54c3c07501f616a771efd1a26cf23654aa623904525ede743940e7b8"
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: passed
  previous_score: 4/4
  gaps_closed:
    - "D-04 / 31-07 (previously HELD/deferred under a user hold): now delivered after the hold was lifted (1aca88d)"
  gaps_remaining: []
  regressions: []
---

# Phase 31: Retire Superseded Envelope & Goal-Layer Code - Verification Report

**Phase Goal:** Code from superseded Phases 19, 21 and 22 is either removed or rewired into the new path, so the codebase carries no unfinished parallel safety model.
**Verified:** 2026-10-28 (HEAD ccfc6ac)
**Status:** passed
**Re-verification:** Yes. D-04 / 31-07 was held and is now executed (a23d1d4, 2e2e335, b9990c0); it is verified as delivered, not deferred.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | SC1: Phase 19 guard + carrier corpora, Phase 21 goal-layer seams and stream-json injection deleted; namespace refusal, secret scan, PR ledger, SAFE-04, `Untrusted`, router kept and rewired | VERIFIED | `src/envelope/` = mod, policy, scan, ledger; `src/driver/` = direction, kill, lock, liveness, mod, rate_limit, router, steer. No guard/classifier/corpora/goal-layer files remain. `rtk proxy grep` for CodexExecutor, ClaudeExecutor, ExecEvent, DrivableProject, build_codex_argv, exec_tx, process_wrap over src/tests/scripts: zero hits (two `run_states` hits are unrelated help-screen test names). |
| 2 | SC2: Phase 22 plans archived as superseded, `bollard` never added, `GSDMM_EXPERIMENTAL_FEATURES` gates only live surfaces | VERIFIED | 22-0x-PLAN.md frontmatter `status: superseded` / `superseded_by: phase-28`, `22-SUPERSEDED.md` present. `grep -c bollard Cargo.toml Cargo.lock` = 0/0. The flag is read once in `src/experimental.rs` and stored on `AppContext::experimental` (app.rs:815); the legacy start/stop/opt-in surfaces it used to gate were retired in 31-02/03/04. |
| 3 | SC3: T-19-86/91/111/116 and Phase 21 criterion 4 closed with pointers in `deferred-items.md` | VERIFIED | `.planning/phases/19-*/deferred-items.md:3232` "Phase 31 closure" table, one row per open Phase 19 threat id (classes G/N/K). `31-SECURITY.md` threats_open 0 (8c81cd8). |
| 4 | SC4: `./scripts/pre-tag-check.sh --container` passes on the reduced tree | VERIFIED | 31-07-SUMMARY quotes `--container v1.15.0` exit 0, six gates PASS on the final tree (post-D-04). I did not re-run the container (needs a runtime and a cold multi-GB build); the local equivalents were re-run by me (below) and match the claim exactly. |
| 5 | D-04 / 31-07 delivered: Codex executor, in-process Claude transport, TUI exec channel and `process-wrap` removed | VERIFIED | `src/executor/` = mod.rs, runtime.rs, stream_json.rs only (codex, codex_json, claude, gate, outcome deleted). `tests/executor_lifecycle.rs`, `tests/executor_transport.rs`, `tests/fixtures/codex/`, fake-codex/fake-claude fixtures gone. `grep process-wrap` Cargo.toml/Cargo.lock = 0 hits; `uuid` no longer a direct dependency. `AgentRuntime` config parse and `stream_json` (rate-limit fixtures) retained and tested. `src/main_loop.rs` pump reduced to action + redraw arms. `git diff 69d371d HEAD -- src tests scripts Cargo.toml` = 40 files, +212/-11276. |

**Score:** 5/5 truths verified (4 roadmap SCs + D-04).

### Behavioral Spot-Checks (run by verifier)

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Full suite | `rtk proxy cargo test --no-fail-fast` | exit 0; 55 `test result:` lines; passed=3158 failed=0 ignored=17 (ignored unchanged vs the 17 baseline, no new `#[ignore]`) | PASS |
| Lint | `rtk proxy cargo clippy --locked --all-targets -- -D warnings` | exit 0, `--locked` resolves against the regenerated Cargo.lock | PASS |
| No tag / publish | `git tag --points-at HEAD` | empty | PASS |

Cross-target clippy (windows-gnu, aarch64/x86_64 darwin) and the container dry run are taken from 31-07-SUMMARY, not re-run.

### Requirements Coverage

| Requirement | Source Plans | Status | Evidence |
|-------------|--------------|--------|----------|
| RETIRE-01 | 31-01..05, 07, 08, 09 | SATISFIED | SC1 and D-04 above. |
| RETIRE-02 | 31-02, 03, 04, 06 | SATISFIED in code and planning artifacts (SC2). **Bookkeeping stale:** `.planning/REQUIREMENTS.md:290` still shows `[ ]` and traceability line 410 still says `Pending`; both should flip to `[x]` / `Complete`. Not a code gap. | SC2 evidence. |
| RETIRE-03 | 31-05, 06, 07, 09 | SATISFIED | SC3, SC4. |

No orphaned requirement IDs: REQUIREMENTS.md maps only RETIRE-01/02/03 to Phase 31 and all appear in plan frontmatter.

### Code-review findings judged against the goal ("no unfinished parallel safety model")

31-REVIEW.md (post-31-07): 0 critical, 2 warnings, 6 info; disposition file lists them as `open`.
- **WR-01** (`src/driver/kill.rs`): `the_driver_grace_exceeds_the_claude_group_grace_plus_slack` compares against a test-local mirror (`CLAUDE_GROUP_GRACE`) of a deleted constant; the 12 s `DRIVER_TEARDOWN_GRACE` is still consumed by `launcher::run::stop_group`, and the upper-bound test still has a real basis. This is a stale-rationale test and doc comment. It enforces no safety decision and adds no second decision-maker. Cosmetic.
- **WR-02** (`src/session_detector.rs`): comments and two assertion messages still say "this build's own executor emits `--session-id` / `codex exec`". The parser rule itself is sound for externally launched sessions. Documentation accuracy only. Cosmetic.
- Info items (stale comments citing deleted files, a dead intra-doc link, production-dead code with an overstated module doc, retargeted test names, etc.) are hygiene.

None leaves a guard, goal layer, injection path or competing safety model in the tree. They are recommended follow-up cleanup, not gaps. Debt-marker scan (TBD/FIXME/XXX) over `src/driver/kill.rs`, `src/session_detector.rs`, `src/executor/*.rs`: no hits.

### Anti-Patterns Found

None blocking. The prior `#[allow(dead_code)]` at executor/mod.rs:284 and the `envelope_disallowed_tools` plumbing are gone with the executor.

### Human Verification Required

None. 31-UI-REVIEW.md (19/24) is advisory and not a must-have; no item here needs subjective human judgment.

### Gaps Summary

No gaps. The previously held D-04 is now delivered and the final tree re-certifies SC4. Only follow-ups: flip RETIRE-02 to complete in REQUIREMENTS.md, and optionally clean up review WR-01/WR-02 and the info items.

---

_Verified: 2026-10-28_
_Verifier: Claude (gsd-verifier)_
