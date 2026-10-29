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
covered_digest: "v4:sha256:24ec560165511e15570713eb64c72078cd10860cf2988258be35f149e0e19be5"
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: passed
  previous_score: 5/5
  gaps_closed: []
  gaps_remaining: []
  regressions: []
advisory:
  - finding: "Code re-review (31-REVIEW.md, 0 critical / 3 warning / 5 info) leaves 7 findings open or deferred; none breaks a must-have"
    category: other
    reason: "Hardening gaps and hygiene in Phase 28/30-era driver code and retired-executor remnants; tracked in 31-REVIEW-DISPOSITION.md and two pending todos"
    evidence_status: "no failing test or reproducible breakage of an SC"
---

# Phase 31: Retire Superseded Envelope & Goal-Layer Code - Verification Report

**Phase Goal:** Code from superseded Phases 19, 21 and 22 is either removed or rewired into the new path, so the codebase carries no unfinished parallel safety model.
**Verified:** 2026-10-28 (HEAD 0844ee8)
**Status:** passed
**Re-verification:** Yes. The prior report went `stale` because quick cleanup 261028-n0u (fixing 8 Phase 31 review findings) touched covered files `Cargo.toml`, `src/driver/kill.rs` and `src/session_detector.rs`. Regenerated at HEAD with a recomputed fingerprint. No truth regressed.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | SC1: Phase 19 guard + carrier corpora, Phase 21 goal-layer seams and stream-json injection deleted; namespace refusal, secret scan, PR ledger, SAFE-04, `Untrusted`, router kept and rewired | VERIFIED | `src/envelope/` = mod, policy, scan, ledger; `src/driver/` = direction, kill, lock, liveness, mod, rate_limit, router, steer. No guard, classifier, corpora or goal-layer files remain. `rtk proxy grep -rnE "CodexExecutor\|ClaudeExecutor\|ExecEvent\|DrivableProject\|build_codex_argv\|exec_tx\|process_wrap\|process-wrap"` over src, tests, scripts, Cargo.toml: zero hits at HEAD. |
| 2 | SC2: Phase 22 plans archived as superseded, `bollard` never added, `GSDMM_EXPERIMENTAL_FEATURES` gates only live surfaces | VERIFIED | `22-01..04-PLAN.md` frontmatter `status: superseded` / `superseded_by: phase-28`; `22-SUPERSEDED.md` present. `grep -c bollard Cargo.toml Cargo.lock` = 0/0. The flag has one production read (`src/app.rs:812`, parse in `src/experimental.rs`), stored on `AppContext::experimental`. |
| 3 | SC3: T-19-86/91/111/116 and Phase 21 criterion 4 closed with pointers in `deferred-items.md` | VERIFIED | `.planning/phases/19-*/deferred-items.md:3232` "Phase 31 closure (2026-10-25)". `31-SECURITY.md`: `status: verified`, `threats_open: 0`, re-audited 2026-10-28 at HEAD (0b08871), SECURED. |
| 4 | SC4: `./scripts/pre-tag-check.sh --container` passes on the reduced tree | VERIFIED | **Re-run by me at HEAD 0844ee8** (see Container Dry Run below): `--container v1.15.0` exit 0, inner run exit 0, six gates PASS, 55 `test result:` lines. |
| 5 | D-04 / 31-07 delivered: Codex executor, in-process Claude transport, TUI exec channel and `process-wrap` removed | VERIFIED | `src/executor/` = mod.rs, runtime.rs, stream_json.rs only. `process-wrap` absent from Cargo.toml/Cargo.lock (grep above). Unchanged from prior verification; the cleanup touched only comments and tests in this area. |

**Score:** 5/5 truths verified (4 roadmap SCs + D-04).

### Container Dry Run (SC4)

Judged whether the prior evidence (31-07-SUMMARY, container exit 0 at the post-D-04 tree) carries over. It does not: `Cargo.toml` changed (`rustix` gained the `system` feature, which gates 2, 3 and 6 build and package), and about 1.8k lines changed across 30 `src`/`tests` files since the prior verified HEAD ccfc6ac. This is the exact failure class the container exists to catch (v1.15.0 publish). Docker 29.1.3 and podman 5.7.0 are both available, so I re-ran it.

`./scripts/pre-tag-check.sh --container v1.15.0` at HEAD 0844ee8: exit 0, inner run exit 0.

```
gate 1  tag / Cargo.toml version   PASS
gate 2  MSRV check                 PASS
gate 3  cargo build --release      PASS
gate 4  cargo test                 PASS
gate 5  cargo clippy -D warnings   PASS
gate 6  cargo package --locked     PASS
```

Log: `/tmp/claude-1000/-home-blk-projects-rust-gsd-meta-manager/30911796-0f3c-4659-8e08-ce10f00ad0d8/scratchpad/container-v31-reverify.log`. Working tree unchanged afterwards (only the two pre-existing `.planning/musk/*` modifications); nothing tagged or pushed. The cache volume exceeded the 20 GB cap post-run and will self-wipe on the next container run; expected behaviour, not a failure.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Full suite (evidence supplied by the orchestrator, not re-run locally) | `rtk proxy cargo test --no-fail-fast` at HEAD 0844ee8 | 55 suites, 3183 passed, 0 failed, 17 ignored (ignored unchanged vs the 17 baseline) | PASS |
| Full suite inside the container (gate 4, run by me) | container dry run | PASS, 55 `test result:` lines | PASS |
| Clippy `-D warnings` inside the container (gate 5, run by me) | container dry run | PASS | PASS |
| No tag points at HEAD | `git tag --points-at HEAD` | empty | PASS |

Passed count rose 3158 -> 3183 since the prior report from the cleanup's added tests; no failures and no new `#[ignore]`.

### Requirements Coverage

| Requirement | Source Plans | Status | Evidence |
|-------------|--------------|--------|----------|
| RETIRE-01 | 31-01..05, 07, 08, 09 | SATISFIED | SC1 and D-04. `.planning/REQUIREMENTS.md:283` `[x]`; traceability line 409 Complete. |
| RETIRE-02 | 31-02, 03, 04, 06 | SATISFIED | SC2. The prior report's bookkeeping gap is closed: `REQUIREMENTS.md:290` is `[x]` and traceability line 410 reads Complete. |
| RETIRE-03 | 31-05, 06, 07, 09 | SATISFIED | SC3, SC4. `REQUIREMENTS.md:291` `[x]`; line 411 Complete. |

All three IDs in plan frontmatter (`requirements:` of 31-01..31-09) are accounted for in REQUIREMENTS.md. REQUIREMENTS.md maps only RETIRE-01/02/03 to Phase 31, so there are no orphaned IDs.

### Code-review findings judged against the goal ("no unfinished parallel safety model")

31-REVIEW.md (re-review at HEAD f3e0464): 0 critical, 3 warnings, 5 info. 31-REVIEW-DISPOSITION.md (0844ee8): 23 rows total, 7 open, 3 deferred, 1 skipped, the rest fixed. The prior round's WR-01/WR-02 (stale grace-mirror test, executor wording in `session_detector.rs`) are now fixed (renumbered WR-101/WR-102; quick 261028-n12 fc9d536, 52b4364). Recorded here as advisories; none breaks a must-have:

- **WR-01** (`src/driver/lock.rs`): `probe_at`/`read_holder_at` do not get the symlink/FIFO hardening that `acquire_at` has. Hardening scope in Phase 28/30 driver code; not a retired-model remnant. Open.
- **WR-02** (`src/driverctl/serve.rs`): `hard_link` socket publish has no fallback on hard-link-less filesystems. Portability of Phase 28/30 code. Open.
- **WR-03** (`src/driver/direction.rs`): first exchange-back in `run_loop` still fails on an absent `DIRECTION.md`. Driver race handling. Open.
- **IN-01..IN-04**: duplicated capped reader, stale "Linux-only" comments, vague "invalid data" message, brittle census counts. Hygiene. Open.
- **IN-05** (deferred, todo `2026-10-28-retire-test-only-executor-remnants.md`): `registry::check_prompt_input_drift`/`OptInDrift` and `head_sha` have no production caller (`rtk proxy grep` shows only `config.rs` tests call them). This is the one item adjacent to the goal wording: it is a retired safeguard left as dead code, not a competing decision-maker. It adds no second path that can allow or deny a spawn, and `DriverOptIn` docs now state nothing on the launcher path compares the digests (IN-07 fixed). Judged hygiene, deferred with a tracked todo [inferred - cleanup 2026-10-28].
- **IN-08/IN-10** (deferred, todo `2026-10-28-unregister-guard-cached-live-check.md`): the unregister guard is advisory and covers the cached scan only. This is a Phase 31 plan 31-08 refinement, not an SC; the guard exists and works for the cached case.

Debt-marker scan (TBD/FIXME/XXX) over `src/driver/kill.rs`, `src/session_detector.rs`, `src/executor/*.rs`, `src/driverctl/store.rs`: no hits.

### Anti-Patterns Found

None blocking.

### Human Verification Required

None. 31-UI-REVIEW.md (19/24, 697b43f) is advisory and not a must-have; nothing here needs subjective human judgment, and every executable check was run or has fresh evidence at HEAD.

### Gaps Summary

No gaps. All four roadmap success criteria and the D-04 deliverable hold at HEAD 0844ee8. The cleanup batch changed `Cargo.toml` (`rustix` `system` feature), `kill.rs` and `session_detector.rs`; I re-ran the container dry run on that tree and it is green, so SC4 is certified fresh rather than carried over. Remaining items are advisory review findings tracked in `31-REVIEW-DISPOSITION.md` and two pending todos.

---

_Verified: 2026-10-28_
_Verifier: Claude (gsd-verifier)_
