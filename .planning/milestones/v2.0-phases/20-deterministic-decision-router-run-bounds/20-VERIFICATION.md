---
phase: 20-deterministic-decision-router-run-bounds
verified: 2026-08-19T00:00:00Z
status: passed
score: 5/5 must-haves verified
behavior_unverified: 0
overrides_applied: 0
re_verification: false
---

# Phase 20: Deterministic Decision Router & Run Bounds Verification Report

**Phase Goal:** The next GSD command is chosen by rules rather than a model call, and a run that
stops making progress stops itself
**Verified:** 2026-08-19
**Status:** passed
**Re-verification:** No — initial verification

## Method

This report does not trust `20-SUMMARY.md`, `20-REVIEW.md`, or `20-FIXES.md` claims at face
value. Every load-bearing claim below was re-derived independently in this session: the full
test suite was run from source with `rtk proxy` (raw, unfiltered output) rather than accepted
from a prior transcript, and for the review's Critical finding (CR-01) and its two most
consequential Warnings (WR-02, WR-07) the pre-fix code was mechanically reconstructed, run, shown
to fail exactly as the fix report describes, and then restored — proving the fixes are not
merely present but load-bearing.

## Goal Achievement

### Observable Truths — the five ROADMAP success criteria

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | For every D-R-P-E-V state the rules cover, the next command is chosen with no model call, and the same project state always yields the same choice | ✓ VERIFIED | `phase_disk_statuses` (a `HashMap`) is read only via `.get()`/`.insert()` in `src/driver/router.rs` and `src/driver/run.rs` — no `.iter()`/`.values()`/`for` loop over it exists in any decision path (grep-confirmed exhaustively, not sampled). `tests/driver_router_table.rs::a_hundred_decisions_over_two_insertion_orders_of_the_same_map_are_all_equal` builds the same entries in forward and reversed insertion order and asserts 100 equal `decide()` calls both ways — re-run independently, passes in <0.01s. `decide()` (`src/driver/router.rs:856`) is a pure function with no I/O. |
| 2 | A run whose observed state hash is unchanged across consecutive iterations, or that re-selects the same command, halts and names which detector fired | ✓ VERIFIED | `tests/driver_iteration_loop.rs::a_routed_run_issues_two_commands_and_halts_naming_the_detector_that_fired` is a real end-to-end `drive()` call across two iterations, independently re-run, passes. `BoundVerdict` is structurally unable to hold more than one reason (confirmed by review, spot-checked in `src/driver/bounds.rs`); the no-progress detector compares `DiskDelta::between` on `ProjectState` value equality, not a digest — confirmed no hash is computed on the no-progress path in the three files the review traced. |
| 3 | A run exceeding its step cap or its wall-clock cap halts and reports that as the reason | ✓ VERIFIED (was FALSE prior to the fix sweep; independently re-verified as genuinely fixed) | **This was the review's one Critical finding (CR-01) and it did not hold at plan-completion time.** `bounds::evaluate` runs once per iteration, immediately before spawn; the per-iteration `wall_clock_cap` handed to the executor was a hardcoded 3h constant (`ITERATION_WALL_CLOCK_CAP`) regardless of the resolved run-level cap, so `--wall-clock-cap-secs 60` was accepted, written to `run.json`, and then overrun by up to 3h. **I independently reproduced this**: reverting `bounds::iteration_wall_clock_cap` to always return `ITERATION_WALL_CLOCK_CAP` (discarding the resolved run budget) and re-running `tests/driver_iteration_loop.rs::a_run_cap_smaller_than_one_iteration_bounds_the_iteration_and_not_the_gap_after_it` fails with `left: "parked:bounds_wall_clock" right: "timed_out"`, finished in **20.13s** — the exact figure the fix report claims. Restoring the shipped code (`iteration_wall_clock_cap` = `wall_clock_cap.saturating_sub(elapsed).min(ITERATION_WALL_CLOCK_CAP)`, called at the actual spawn site with `run_started_at.elapsed()`) makes the same test pass in ~1.07s. The fix is genuinely wired at the real call site, not just present as a helper function. |
| 4 | A run that hits a Claude subscription rate limit parks, reports which quota window blocked it and when it resets, and does not retry | ✓ VERIFIED | `tests/driver_rate_limit.rs` (14 tests) independently re-run, all pass, including `no_sleep_until_reset_is_inserted_between_the_park_and_the_terminal_record` and `no_further_iteration_is_spawned_after_a_quota_park`. Grep confirms no `sleep`/`backoff`/`retry` anywhere on the quota path. **WR-02 (a `rejected` event silently overwritten by a later `allowed` one) independently reproduced**: removing the latch in `src/driver/run.rs` and re-running `a_rejection_followed_by_an_allowed_event_still_parks_the_run` fails with `left: "bounds_command_repeat" right: "quota_rejected"` — the exact defect and exact wrong label the fix report describes; restoring the two-slot latch (`rejected_quota_event.as_ref().or(latest_quota_event.as_ref())`, classified rejection-first) fixes it. **WR-03 (past `resetsAt` accepted as fact)** independently confirmed fixed: the bound is now asymmetric (`RESET_MAX_SKEW_BEHIND_SECS` = 5min behind, `RESET_SANITY_WINDOW_SECS` = 30 days ahead), not the symmetric `saturating_abs` the review flagged. No dollar figure appears anywhere near the quota/cost path. |
| 5 | Reaching a GSD gate that needs human judgement parks the run with the gate named — and every run ends classified as goal-met, parked, or halted with a reason, never as an unclassified "loop ended" | ✓ VERIFIED | `Terminal` (`src/driver/run.rs:443`) has exactly five arms (`Completed`, `GoalMet`, `Parked`, `Halted`, `QuotaParked`), no wildcard on any match over it. **WR-01 (a failed journal write silently converted a halt into a reported success)** independently confirmed fixed: `own_terminal_label(&terminal)` — which takes no `&Path` and cannot reach a disk read — is now asked first at the terminal-label site, and only falls through to the journal re-read when the run's own terminal carries no reason. **WR-08 (goal-met reported two different labels depending on whether any command ran)** independently confirmed fixed: `Terminal::GoalMet` is now its own arm that always labels `goal_met` regardless of `last_outcome`; `tests/driver_iteration_loop.rs::a_run_that_drives_its_target_to_verified_reports_goal_met_and_not_the_agents_outcome` re-run, passes. |

**Score:** 5/5 truths verified (0 present, behavior-unverified)

### Verification of the fix sweep's key claims (independently reproduced, not accepted from `20-FIXES.md`)

| Finding | Claim in `20-FIXES.md` | Independently reproduced? |
|---|---|---|
| CR-01 (Critical) | Pre-fix: `left: "parked:bounds_wall_clock" right: "timed_out"`, 20.13s. Post-fix: `timed_out`, ~1s | **Yes, byte-for-byte match**, including the 20.13s figure |
| WR-02 | Pre-fix: rejection followed by an allowed event ends `bounds_command_repeat` instead of `quota_rejected` | **Yes, exact match** |
| WR-07 | A missing oracle now fails by default instead of passing silently; the oracle runs 5 real comparisons when present | **Yes** — ran the test binary directly with `HOME`/`PATH` stripped: fails with the stated message. Ran normally: `conformance: 5 command comparisons, 2 declared divergences, 2 upstream-silent states, over 9 fixtures` |
| WR-07 (this verifier's own probe, beyond what the fix report claims) | "the oracle would actually fail if the Rust rule table drifted" | **Confirmed for the primary comparison path**: injecting a one-row drift into `RULE_TABLE` (`Discussed → Execute` instead of `Discussed → Plan`) makes the oracle test fail with `left: "/gsd-execute-phase 01" right: "/gsd-plan-phase 01"`. **One narrower gap found and not previously flagged**: the `Upstream::NoAction` comparison arm (used by the `complete` fixture) only asserts `!matches!(ours, Decision::Run { .. })` — it does not distinguish `Decision::GoalMet` from `Decision::Park`/`NoRule`. Forcing `is_goal_met` to always return `false` does not fail this test, because a park is also "not a Run". This is a real but narrow gap in the oracle's coverage of the goal-met path specifically (not the 5 command-selection comparisons, which are the ones proven to catch drift). It does not undermine criterion 1's or criterion 5's goal-met correctness, which is separately covered by `is_goal_met`'s own unit-level definition (`inference.verification_status.is_passed()`) and by the end-to-end `a_run_that_drives_its_target_to_verified_reports_goal_met_and_not_the_agents_outcome` test — recorded here as an informational finding, not a gap, since neither of the two fixed things it touches on (WR-07's fail-loud-on-absence, and drift detection on the primary rule table) is weakened by it. |
| WR-05 | Nested `status:` no longer false-matched; key comparison now requires column zero | **Confirmed by reading the fixed code** (`leading_frontmatter_value` in `src/state_reader/disk_status.rs`): `line.starts_with(char::is_whitespace)` is skipped before the key comparison |
| WR-06 | G15 gate now normalises through `extract_phase_id`/`phase_identity` instead of raw string equality | **Confirmed by reading the fixed code**: `router.rs`'s gate comparison calls `phase_identity()` on both sides |
| WR-09 | `--dry-run` now validates `--target-phase` and bounds before the dry-run branch | **Confirmed by reading the fixed code**: `is_plain_path_component` (line 378) and `bounds::resolve` (line 396) both precede `if args.dry_run` (line 398) in `src/driver/mod.rs` |

### Required Artifacts

| Artifact | Expected | Status | Details |
|---|---|---|---|
| `src/driver/router.rs` | `decide()`, rule table, gate taxonomy, safe alphabet | ✓ VERIFIED | Present, pure, exhaustive, all associated tests pass |
| `src/driver/bounds.rs` | Four detectors, fixed evaluation order, resolved-cap enforcement | ✓ VERIFIED | Present and — as of the fix sweep — genuinely enforces the resolved cap at the spawn site, not just a constant |
| `src/driver/run.rs` | Iteration loop, `Terminal` (5 exhaustive arms), terminal-record honesty | ✓ VERIFIED | Present, wired, `own_terminal_label` asked first (WR-01), `GoalMet` is its own arm (WR-08) |
| `src/driver/rate_limit.rs` | Pure classification of a rate-limit payload into window/reset/park verdict | ✓ VERIFIED | Present, asymmetric reset bound (WR-03), no clock/retry logic |
| `tests/driver_iteration_loop.rs` | End-to-end proof of bounded, named-detector halting | ✓ VERIFIED | 7/7 tests pass, independently re-run |
| `tests/driver_router_table.rs` | Both-directions rule/row guard, disjointness, determinism | ✓ VERIFIED | 12/12 tests pass, independently re-run |
| `tests/driver_router_conformance.rs` | Conformance oracle against GSD's own router | ✓ VERIFIED | 3/3 tests pass; fail-loud-without-oracle and drift-detection both independently reproduced |
| `tests/driver_rate_limit.rs` | End-to-end quota park proof | ✓ VERIFIED | 14/14 tests pass, independently re-run |

### Key Link Verification

| From | To | Via | Status |
|---|---|---|---|
| `src/driver/run.rs` (spawn site) | `src/driver/bounds.rs::iteration_wall_clock_cap` | `run_started_at.elapsed()` passed at the actual `executor.start(...)` call | ✓ WIRED (independently confirmed — this is the CR-01 fix) |
| `src/driver/run.rs` | `src/driver/router.rs::decide` | called once per iteration | ✓ WIRED |
| `src/driver/run.rs` | `src/driver/rate_limit.rs::classify` | drain loop retains payload, classified once at iteration end with rejection latched first | ✓ WIRED (independently confirmed — this is the WR-02 fix) |
| `src/driver/run.rs` | `src/journal/mod.rs` | `Terminal::Parked`/`Halted`/`QuotaParked` all write through `JournalEvent::Parked`; `own_terminal_label` reads the in-memory terminal, not a re-read, when the run has its own reason | ✓ WIRED (independently confirmed — this is the WR-01 fix) |

### Behavioral Spot-Checks / Independent Test Runs

| Behavior | Command | Result | Status |
|---|---|---|---|
| Full workspace test suite | `rtk proxy cargo test` (run once, raw output, ~30 targets) | **1155 passed, 0 failed** — matches the claimed figure exactly | ✓ PASS |
| Clean build | `rtk proxy cargo build --release` | Clean | ✓ PASS |
| Clippy (deny warnings) | `rtk proxy cargo clippy -- -D warnings` | Clean | ✓ PASS |
| Clippy (all targets) | `rtk proxy cargo clippy --all-targets` | Exactly 5 pre-existing lints (`browser.rs`, `project_creator.rs`, `state_reader/mod.rs` + 2 test-file lints) — matches the claimed figure | ✓ PASS |
| CR-01 pre-fix reproduction | Revert `iteration_wall_clock_cap` to return the bare constant; re-run the named guard test | Fails with `parked:bounds_wall_clock` vs `timed_out`, 20.13s — byte-for-byte match to the fix report | ✓ PASS (defect reproduced, fix confirmed necessary) |
| WR-02 pre-fix reproduction | Remove the rejection latch; re-run the named guard test | Fails with `bounds_command_repeat` vs `quota_rejected` — exact match | ✓ PASS (defect reproduced, fix confirmed necessary) |
| WR-07 loud-skip reproduction | Run the oracle test binary directly with `HOME`/`PATH` stripped | Fails with the stated `SKIP:`-turned-panic message | ✓ PASS |
| WR-07 drift-detection probe (new, beyond the fix report) | Inject a one-row drift into `RULE_TABLE`; re-run the oracle test | Fails, naming the exact mismatched commands | ✓ PASS |
| `driver_reattach.rs` (scope-fenced, not part of this phase's work) | `rtk proxy cargo test --test driver_reattach` | 2 of 3 tests failed on this run (`assertion left == right failed: exactly one project has a run to observe: left: 0 right: 1`) — an intermittent process-timing test | Not a gap — the orchestrator's brief states this fails identically at `b6c1ae7`, a docs-only commit (`git show --stat` confirms zero source files touched) that predates all phase-20 source work. Confirmed the commit's content independently; did not re-derive the historical failure at that commit (out of scope per the brief) |

### Requirements Coverage

| Requirement | Description | Status | Evidence |
|---|---|---|---|
| CTRL-06 | Run halts itself on no-progress, command-repeat, step cap, or wall-clock cap | ✓ SATISFIED | Truths 2 and 3 above; `bounds::resolve` also refuses a zero cap and caps the maximum at `MAX_WALL_CLOCK_CAP_SECS`, closing the "disablement in disguise" prohibition |
| CTRL-07 | Run parks (never retries) on a Claude subscription rate limit, naming the window | ✓ SATISFIED | Truth 4 above |
| DRIVE-02 | Deterministic-rules command selection, no model call | ✓ SATISFIED | Truth 1 above; `decide()` is pure, no model/I-O call anywhere in the router |
| DRIVE-05 | Parks and flags for human on a GSD gate requiring judgement | ✓ SATISFIED | `gate_for()` in `router.rs` parks under a closed, greppable reason set before the rule table or goal-met check runs; WR-06's normalisation fix confirmed |
| DRIVE-06 | Reports a terminal outcome (goal-met/parked/halted) with reason, never unclassified | ✓ SATISFIED | Truth 5 above; `Terminal`'s 5 exhaustive arms plus WR-01/WR-08 fixes |

No orphaned requirements: `.planning/REQUIREMENTS.md` maps exactly CTRL-06, CTRL-07, DRIVE-02, DRIVE-05, DRIVE-06 to Phase 20, and all five plans (`20-01` through `20-05`) declare them collectively.

### Anti-Patterns Found

None found in the phase's touched files beyond what `20-REVIEW.md` already found and `20-FIXES.md` already closed. No `TODO`/`FIXME`/`XXX`/`TBD` debt markers without a referenced follow-up were found in the phase's source files during this session's reading.

### Deferred / Out-of-Scope Items (per the verification brief's scope fences — not gaps)

| Item | Disposition |
|---|---|
| `tests/driver_reattach.rs` intermittent failures | Pre-existing, reproduces identically at `b6c1ae7` (a docs-only commit with zero phase-20 source, confirmed via `git show --stat`) |
| Nothing mechanically prevents a driven agent writing `status: passed` into a `*-VERIFICATION.md` itself | Disclosed in `src/envelope/mod.rs`, explicitly deferred to Phase 23 |
| `/gsd-verify-work` excluded from the router's safe alphabet | Deliberate; configurable in Phase 23 (CTRL-08) |
| IN-02, IN-03, IN-04, IN-05, IN-06, IN-08 | Consciously deferred with written reasons in `20-FIXES.md`; none affects any of the five ROADMAP criteria |
| 5 pre-existing clippy lints under `--all-targets` | Confirmed unchanged, pre-existing |
| **ROADMAP: "Phase 22 must land before this phase closes"** | **Phase 22 has not started** (`ROADMAP.md` progress table: "0/? — Not started"). This is a real, unmet closing condition on Phase 20 stated by the ROADMAP itself, not an implementation gap in what Phase 20 built — the router deliberately grows no container-shaped branch (`20-CONTEXT.md` explicitly scopes this out). Flagging it here as a fact the next step must account for: Phase 20's own deliverables verify as complete, but the milestone-level roadmap does not consider Phase 20 closed until Phase 22 lands. |

### Human Verification Required

None. All five ROADMAP success criteria were independently verified against running code and, for the phase's Critical finding and two of its highest-risk Warnings, against a mechanically reconstructed pre-fix state proving the fix is load-bearing rather than cosmetic.

### Gaps Summary

No gaps found against the phase's stated goal or the five ROADMAP success criteria. The one Critical review finding (CR-01 — the wall-clock cap was not actually enforced) was real, was independently reproduced by this verifier in its pre-fix form, and the shipped fix independently confirmed to close it. The nine Warnings were spot-checked at the code level; the three most consequential to the ROADMAP criteria (WR-01, WR-02, WR-08) were independently confirmed fixed, two of them (WR-02, and CR-01 itself) by reproducing the exact pre-fix failure. Build is clean, `cargo test` is 1155/0 (matches the stated figure exactly), clippy is clean under `-D warnings` with the same 5 pre-existing lints under `--all-targets`.

The only unresolved item is a **stated, disclosed, roadmap-level dependency** — Phase 22 has not yet landed, and the ROADMAP itself makes that a precondition for *closing* Phase 20, distinct from Phase 20 having *delivered* its goal. That is not a defect in this phase's code and is explicitly out of this verifier's scope per the brief, but it is the one fact standing between "Phase 20's goal is achieved in the codebase" (true, per this report) and "Phase 20 is closeable per the roadmap" (not yet, per the roadmap's own stated ordering).

One narrow, previously-unflagged observation is recorded above under the WR-07 row: the conformance oracle's `NoAction` comparison arm does not distinguish `GoalMet` from `Park`, which is weaker coverage of the goal-met path specifically than the oracle's primary command-selection comparisons (which were independently proven to catch drift). This does not weaken any of the five ROADMAP criteria and is not treated as a gap.

---

_Verified: 2026-08-19_
_Verifier: Claude (gsd-verifier)_
