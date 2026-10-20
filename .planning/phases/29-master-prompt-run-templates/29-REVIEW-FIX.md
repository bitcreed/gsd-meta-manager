---
phase: 29-master-prompt-run-templates
fixed_at: 2026-10-20T00:00:00Z
review_path: .planning/phases/29-master-prompt-run-templates/29-REVIEW.md
iteration: 1
findings_in_scope: 12
fixed: 9
skipped: 3
status: partial
---

# Phase 29: Code Review Fix Report

**Fixed at:** 2026-10-20
**Source review:** `.planning/phases/29-master-prompt-run-templates/29-REVIEW.md`
**Iteration:** 1
**Isolation / verification env:** main checkout on `master` (`workflow.use_worktrees` not used: ISOLATION=none, as instructed). All gates below ran in the main checkout, so they are reproducible from this tree.

**Summary:**
- Findings in scope: 12 (7 warnings, 5 info; `fix_scope=all`, info only if trivial)
- Fixed: 9 (WR-01..WR-07, IN-02, IN-04), plus 1 extra operator-requested item (10 commits in all)
- Skipped: 3 (IN-01, IN-03, IN-05), each with a reason below

The frontmatter counts the 12 review findings only (9 fixed / 3 skipped); the extra `claude_argv` doc commit is outside them.

**Verification:** `cargo test --no-fail-fast`: 86 result lines, 3700 passed, 1 failed. The one failure is the expected local-only `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against` (git-version witness, green on the runner). `cargo clippy --all-targets -- -D warnings`: clean. `scripts/prompt-acceptance.sh selftest`: OK. `scripts/prompt-acceptance.sh check h2` still re-derives the committed FAIL byte-for-byte (the committed evidence is untouched).

## Fixed Issues

### WR-01: DIRECTION lint and the store's parser read different plans

**Files modified:** `src/driver/direction.rs`, `src/driver_templates/lint.rs`
**Commit:** 1fb4c24
**Status:** fixed: requires human verification (reader-agreement logic)
**Applied fix:** One shared reader instead of a cross-check. `blank_comments` and a new `section_heading`/`Section` moved into `driver::direction`; `parse_direction` now blanks HTML comments and counts `### I-n` only under `## Items`, exactly as the lint does. The lint also reports a stray `### I-n` outside `## Items`, a `- control:` outside `## Direction`, and a second `- control:` line. Tests: multi-line commented-out item invisible to both readers, stray item under `## Direction` and under a later section flagged and ignored by the store, duplicate and out-of-section control lines flagged.
**Decision [inferred - audit]:** the stray item is made invisible to the store (and flagged by the lint) rather than only flagged, because a misplaced heading that stays dispatchable is the divergence the review describes. `control` is deliberately NOT section-scoped in the store (a pause typed anywhere still holds the run: fail closed); the lint flags it instead.

### WR-02: pause/stop control fails open on a read error and on non-lowercase spellings

**Files modified:** `src/driver/direction.rs`, `src/driverctl/store.rs`, `src/launcher/gate.rs`, `tests/driverctl_store.rs`
**Commit:** e4b8125
**Status:** fixed: requires human verification (control-flow / fail-closed semantics)
**Applied fix:** A single `control_hold` matcher (trim + ASCII case-insensitive `pause`/`stop`) is used by the store's `dispatched` refusal and by `decide_dispatch`. The control regex now tolerates indentation, extra spaces and key case; `parse_direction` lower-cases the word, and a `pause`/`stop` on ANY duplicate control line wins over an earlier `run`. `direction_read()` separates `NotFound` from other read errors: NotFound stays the documented "no direction" case (a bare `driverctl serve`, a run dir not yet filled; allowed); any other error denies the hook dispatch ("DIRECTION.md is unreadable (...): control unknown, dispatch nothing new", recorded in the existing `dispatch-denied` event, no new event vocabulary) and rejects a `dispatched` record. Other log events still record so the driver can end the run. Tests: `Pause`/`PAUSE`/` Stop `/indented/odd-spaced variants for both gate and record, duplicate control lines, unreadable file (a directory in place of DIRECTION.md) for both paths, absent file still allows, plus unit tests in `direction.rs` and `gate.rs`.
**Decisions [inferred - audit]:** NotFound = allow (it is the documented no-direction case); unreadable = deny; no extra `direction-unreadable` log event (the denial reason already lands in `dispatch-denied`, and a new event would widen the pinned LOG grammar). An unrecognised control word (typo) still falls through as before (the pinned test `control_run_or_absent_falls_through...` asserts `Some("other")` allows).

### WR-03: DIRECTION.md content reaches the terminal unescaped through lint findings

**Files modified:** `src/driver_templates/lint.rs`, `src/driver_cli.rs`
**Commit:** 9258e33
**Applied fix:** `Display for Finding` and `Finding::as_warning` (the only two printed forms, used by `driver check` and the launch preflight) run the message through `render_for_terminal`; the raw `message` field stays comparable. `driver new` now prints its `check:`/`launch:` command lines with the target passed through a new `shell_word` (escaped, single-quoted when not a plain word). Tests: a hostile value (ESC, C1 CSI, BEL, RLO) prints no control/bidi character in either form; `shell_word` cases.

### WR-04: `mentions_phase` attributes a decimal phase to its integer parent

**Files modified:** `src/launcher/gate.rs`
**Commit:** 35a7264
**Status:** fixed: requires human verification (matcher logic)
**Applied fix:** The key must now stand alone: the `phase N` forms need a following end/non-alphanumeric/non-dot character (or a dot not followed by a digit); the `N --gaps` form needs a preceding start/non-alphanumeric/non-dot character. The review's suggested pattern was extended with `_` and the leading-side guard (otherwise `26.1 --gaps` still matched key `1`). Tests cover 26 vs 26.1 vs 26.10 vs 260 and 26.1.2 in both directions, the `--gaps` forms, trailing punctuation, and a gate-level test that phase 26 over its cap does not deny a 26.1 dispatch.

### WR-05: DIRECTION template promises an `isolation:` knob the master prompt never reads

**Files modified:** `src/driver_templates/assets/DIRECTION.md`, `src/driver_templates/mod.rs`
**Commit:** 4fd39cb
**Applied fix:** Removed the promise (the `- isolation: worktree` line and its comment) from the template and added a test that every knob the template ships is one the prompt reads.
**Justification (smaller, safer change) [inferred - audit]:** teaching the prompt to read `isolation:` would change `driver-prompt/1.0.0`; that exact version is what the committed live acceptance evidence was measured against, so a bump would invalidate it and need a paid re-run. Removing two template lines changes nothing the prompt or any evidence depends on. Existing DIRECTION files that still carry `isolation:` lint clean (the key is simply ignored, as before). `PA_ISOLATION` in the o1 case of `scripts/prompt-acceptance.sh` is now a no-op (its python branch matches no line); left in place because it changes no behaviour and o1 evidence is already committed.

### WR-06: "reconcile first" refusal enforced only on the `dispatched` log record

**Files modified:** `src/driverctl/store.rs`, `tests/driverctl_store.rs`
**Commit:** 6f45357
**Status:** fixed: requires human verification (gate ordering)
**Applied fix:** The blocking set is factored into `blocking_recovery()` and used by both the `dispatched` record rule and the hook path. After the installed gate allows (so cap and control denials are still named first), the store denies a driver-level Agent/Task dispatch while an item the last launch left running and DIRECTION still lists has no `recovered`: "reconcile first: I-01 was running at launch start; ...". The denial is counted as denied (not as a dispatch) and logged as `dispatch-denied`. Tests: gate refuses, then passes after the `recovered`; a control hold is named before the reconcile rule.
**Decision [inferred - audit]:** enforced in the store's hook path rather than inside the pure `decide_dispatch`, because the pure function has no DIRECTION item list and the store is where both rules already share one filter; it also holds for any installed gate.
**Needs a human (not changed):** hook denials reach the driver as `LAUNCHER CAP: reconcile first: ...`, and master-prompt stop rule 2 says any denial other than `control=` is parked `launcher-cap`. A driver that skips its Startup reconcile and trips this denial would therefore park the item instead of recovering it. The prompt already instructs the reconcile before any dispatch, so this only affects a non-conforming driver, and fixing it means editing the prompt (version bump, evidence invalidation, fixture regeneration). Recommend deciding that together with the next prompt revision.

### WR-07: H2 acceptance case cannot stage its premise

**Files modified:** `scripts/prompt-acceptance.sh`, `tests/prompt_acceptance.rs`
**Commit:** 04a7a40
**Applied fix:** `case_h2` now calls `pa_stage_item_running I-01` after launch 1 ends and before the relaunch: if the store does not already say I-01 is `running`, it logs `status --state running` host-side (free, no model; the `driverctl serve` start/stop of `pa_seed_park` was factored into `pa_serve_up`/`pa_serve_down`, and H3 still passes its rehearsal) and records `premise_i_01_running=native|staged-host-side` plus the pre-staging state in `observed.txt`, so the evidence says which it was. `h2_harness=2` marks evidence from this revision. `pa_evidence_guard` lets an h2 FAIL that lacks that marker (a harness defect, not a product miss) use the guard's existing single rerun; RERUN=1, the consent gate and the ceiling gate are untouched, and a FAIL from the fixed harness is refused again like any logic FAIL. `eval_h2` is unchanged, so `check h2` still re-derives the committed verdict. Tests: scripted rehearsals for the native premise and for a driver that finished I-01 (staged, PASS), and a guard test (old h2 FAIL passes the guard to the consent gate, marked h2 FAIL and h1 FAIL are refused).
**Not done, by instruction:** no paid or live run, no real-model re-run, no committed evidence file touched. **The H2 re-run is pending**: the committed `acceptance/h2/verdict.txt` is still `RESULT FAIL` (H2-5), 29-VERIFICATION.md / 29-UAT.md were not edited, and the live claim "pause mid-dispatch then relaunch reconciles" remains unproven live until the operator consents to the rerun (`RERUN=1 scripts/prompt-acceptance.sh run h2`, which needs the consent file).
**Decisions [inferred - audit]:** staging a `running` state after the driver legitimately finished the item makes H2-5 test the reconcile ordering, not that a live driver was interrupted mid-worker; the observed lines state this explicitly. The guard allowance is keyed on the missing marker so it cannot be used twice.

### IN-02: `pa_selftest` skip logic for a missing spike directory

**Files modified:** `scripts/prompt-acceptance.sh`
**Commit:** 3dfa13e
**Applied fix:** Explicit `if [ -d "$S3" ]; then ... fi` around the three file checks. Selftest passes.

### IN-04: `DEFAULT_GAP_CAP` duplicates the CLI default

**Files modified:** `src/cli.rs`, `src/driver_templates/lint.rs`, `src/driverctl/serve.rs`
**Commit:** 8c57b2c
**Applied fix:** `cli::DEFAULT_CAP_GAP_ROUNDS` is the one constant, used by clap's `default_value_t` and by the lint; the cli test still pins the value 2. `driverctl::serve::DRIVERS_REL` is now an alias of `state_reader::driver_frontmatter::DRIVER_RUNS_REL`.

### Extra (operator request): stale `[inferred - audit]` note on `claude_argv`

**Files modified:** `src/launcher/sandbox.rs`
**Commit:** 2b581fa
**Applied fix:** Replaced the `[inferred — audit]` sentence with a note that the `--` behaviour was verified against the real `claude` 2.1.304 (commit 6ffda4c, Phase 28 `28-REVIEW-FIX.md` WR-03). Doc comment only.

## Skipped Issues

### IN-01: Prompt uses `$RUN_DIR` and `$PROJECT_ROOT` as if they were environment variables

**File:** `src/launcher/assets/MASTER-PROMPT.md:20-24,131`
**Reason:** not trivial. Either fix is a contract change: editing the prompt bumps `driver-prompt/1.0.0` and invalidates the committed live evidence; exporting `RUN_DIR`/`PROJECT_ROOT` in `child_env` changes the driver environment that `launcher_e2e` pins and adds generic-named variables to the sandboxed process. Needs its own decision.
**Original issue:** The launcher substitutes `{RUN_DIR}`/`{PROJECT_ROOT}` textually, but only `DRIVER_LOCK_HELD` is in the driver env, so `cat $RUN_DIR/DIRECTION.md` would expand empty.

### IN-03: Preflight reorders refusals

**File:** `src/launcher/run.rs:229-253,551-553`
**Reason:** not trivial. Probing the lock before linting changes the launch refusal order and exit codes that tests and the detached-launcher path depend on; the lossy-UTF-8 half alone would leave the ordering half undone. Deferred to a change that owns the ordering.
**Original issue:** A paused or invalid DIRECTION now masks "already launched" (exit 1 instead of exit 3), and `read_to_string` refuses a non-UTF-8 file the store reads leniently.

### IN-05: `shell_quote` is lossy for non-UTF-8 paths

**File:** `src/launcher/config_dir.rs:101-104`
**Reason:** not trivial. Quoting from raw bytes changes the shim-writing path from `String` to bytes, and refusing to seed on a non-UTF-8 path adds a new error flow; both need design, and the case is rare.
**Original issue:** `String::from_utf8_lossy` replaces invalid bytes, so the shims could `exec` a different path than the real one.

## Needs a human

- Operator consent for the pending H2 re-run (paid); nothing here spends or edits evidence.
- WR-06 follow-up: the prompt's stop rule 2 would park an item on a `reconcile first` denial; decide with the next prompt revision.
- WR-01/02/04/06 are logic fixes; the commits carry tests, but a read of the fail-closed and matcher semantics is warranted.
- No irreversible or scope-changing fork was hit; every inferred decision is marked `[inferred - audit]` above.

---

_Fixed: 2026-10-20_
_Fixer: Claude (gsd-code-fixer)_
_Iteration: 1_
