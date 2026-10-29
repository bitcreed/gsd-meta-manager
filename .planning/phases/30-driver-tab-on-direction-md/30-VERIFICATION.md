---
phase: 30-driver-tab-on-direction-md
verified: 2026-10-29T09:00:00Z
status: passed
score: 6/6 must-haves verified
covered_files:
  - .planning/phases/30-driver-tab-on-direction-md/30-01-PLAN.md
  - .planning/phases/30-driver-tab-on-direction-md/30-01-SUMMARY.md
  - .planning/phases/30-driver-tab-on-direction-md/30-02-PLAN.md
  - .planning/phases/30-driver-tab-on-direction-md/30-02-SUMMARY.md
  - .planning/phases/30-driver-tab-on-direction-md/30-03-PLAN.md
  - .planning/phases/30-driver-tab-on-direction-md/30-03-SUMMARY.md
  - .planning/phases/30-driver-tab-on-direction-md/30-04-PLAN.md
  - .planning/phases/30-driver-tab-on-direction-md/30-04-SUMMARY.md
  - .planning/phases/30-driver-tab-on-direction-md/30-05-PLAN.md
  - .planning/phases/30-driver-tab-on-direction-md/30-05-SUMMARY.md
  - .planning/phases/30-driver-tab-on-direction-md/30-06-PLAN.md
  - .planning/phases/30-driver-tab-on-direction-md/30-06-SUMMARY.md
  - .planning/phases/30-driver-tab-on-direction-md/30-07-PLAN.md
  - .planning/phases/30-driver-tab-on-direction-md/30-07-SUMMARY.md
  - .planning/phases/30-driver-tab-on-direction-md/30-08-PLAN.md
  - .planning/phases/30-driver-tab-on-direction-md/30-08-SUMMARY.md
  - .planning/phases/30-driver-tab-on-direction-md/30-09-PLAN.md
  - .planning/phases/30-driver-tab-on-direction-md/30-09-SUMMARY.md
  - src/app.rs
  - src/driver/direction.rs
  - src/driver/lock.rs
  - src/driver/steer.rs
  - src/driverctl_cli.rs
  - src/journal/reader.rs
  - src/launcher/config_dir.rs
  - src/launcher/stop.rs
  - src/main.rs
  - src/state_reader/driver_frontmatter.rs
  - src/ui/screens/confirm_popup.rs
  - src/ui/screens/detail.rs
  - src/ui/screens/driver_confirm.rs
  - src/ui/screens/driver_launch.rs
  - src/ui/screens/driver_run.rs
  - src/ui/screens/driver_steer.rs
  - tests/driver_tab_launch_e2e.rs
  - tests/driverctl_lock_race.rs
  - tests/driverctl_race.rs
  - tests/editor_command.rs
  - tests/launcher_config_dir.rs
covered_digest: "v4:sha256:03a362b82737de5db2d296e070307eb473f9854413611d4daac617a4ed25346d"
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: passed
  previous_score: 6/6
  gaps_closed: []
  gaps_remaining: []
  regressions: []
gaps: []
deferred: []
advisory:
  - finding: "WR-14 (open): scalar `permissions` settings in src/launcher/config_dir.rs use first-doc-wins while every other scalar uses last-doc-wins."
    category: other
    reason: "Phase 29 launcher config, not on the Driver tab read or steering path. Defeats none of SC1-SC4. Pre-existing inconsistency that 261028-v2z codified with a comment and a test. [inferred — cleanup 2026-10-28]"
    evidence_status: "30-REVIEW.md WR-14 (code-read); no failing test"
  - finding: "WR-15 (open): hook-item validation stops at `type`, so `{type:command}` with a missing or non-string `command` still reaches the managed file."
    category: security
    reason: "Same class as WR-13 one nesting level deeper, in Phase 29 launcher config. The launch e2e and all 28 config_dir tests pass, so SC4 launch is unaffected. Follow-up belongs to the Phase 29 owner. [inferred — cleanup 2026-10-28]"
    evidence_status: "30-REVIEW.md WR-15 (code-read); no failing test"
  - finding: "IN-10..IN-19: wording, diagnostics, duplication, a dead branch in filter_user_hooks (IN-19), silent dropped host policy (IN-18)."
    category: other
    reason: "No must-have depends on the message text or the dead branch. [inferred — cleanup 2026-10-28]"
    evidence_status: "30-REVIEW.md, 30-REVIEW-DISPOSITION.md"
---

# Phase 30: Driver Tab on DIRECTION.md Verification Report (re-verification at HEAD 7cdc35f)

**Phase Goal:** The TUI observes and steers driver runs only through the run's files. Status, caps, heartbeat and crash detection come from the tool-written frontmatter, and steering is an edit to DIRECTION.md.
**Verified:** 2026-10-29
**Status:** passed
**Re-verification:** Yes. The prior report (passed, 6/6, 252666a) went stale because quick task 261028-v2z (601d20f, 1158139, 92dac7a, bfbd231; docs 40870ca) changed `src/launcher/config_dir.rs` and `tests/launcher_config_dir.rs` (WR-13: wrong-shape nested host hooks/permissions now fail closed).

`git diff --name-only 252666a HEAD -- src tests` lists exactly those two files. Every other covered source is byte-identical to the previously verified state, so the other truths take a regression check (existence plus the full named-test set re-run), not a re-derivation. `src/launcher/config_dir.rs` joins the covered set; the fingerprint came from `gsd-tools query verification.fingerprint`, copied verbatim.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | SC1: Driver tab lists run dirs; the selected run shows status, items, caps, audit, heartbeat, `connected[]` from frontmatter, no Claude call | VERIFIED | `grep Command::new` over `driver_run.rs`, `driver_steer.rs`, `driver_launch.rs`, `driver_confirm.rs`, `steer.rs`, `driver_frontmatter.rs` returns nothing at HEAD. 355-test lib filter passes (includes `driver_frontmatter`, `driver_run`). |
| 2 | SC2: `running` + free lock shown crashed; stale heartbeat flagged; dashboard marks driven/parked/crashed | VERIFIED | `derive_status`, `launcher_markers`, `probe_at`, `read_holder_at` tests pass, including `the_header_says_the_probe_failed_for_liveness_unknown_and_never_crashed`. lock.rs is unchanged since the last pass. |
| 3 | SC3: set control, answer/un-hold, open in `$EDITOR`; every edit via the shared flock, frontmatter untouched | VERIFIED | `write_body`, `apply_edit`, `open_run_file`, `a_save_landing`, `exchange_back` tests pass (355 filter). `tests/editor_command.rs` 5 passed; `tests/driverctl_race.rs` 4 passed (1 opt-in ignored); `tests/driverctl_lock_race.rs` 5 passed. |
| 4 | SC4: start a new run, relaunch a crashed one, tail LOG.md live; injection UI is not the steering path | VERIFIED | `tests/driver_tab_launch_e2e.rs` 2 passed; `launcher_new_run`, `launcher_launch`, `read_log_delta` pass. The launch path goes through `config_dir.rs`, the file that moved: `tests/launcher_config_dir.rs` is 28 passed (was 27, +1 for the WR-13 regression test), so the changed code still serves launch. |
| 5 | Status words come from the lock probe, never the frontmatter `status:` word alone | VERIFIED | `derive_status` takes the probe result; tests pass. |
| 6 | D-15/D-22: polite stop is distinguishable from kill; confirmation says "Nothing is killed." | VERIFIED | `launcher_steer_*` and `confirm_popup` tests pass. |

**Score:** 6/6. No behavior-dependent truth is left unexercised: the flock, give-back and probe invariants each have a named passing test.

### Required Artifacts / Key Links

Unchanged files: existence and wiring re-confirmed by the passing 355-test lib filter and the five integration suites, which drive `app.rs` -> `ui/screens/driver_*.rs` -> `driver/{steer,direction,lock}.rs` -> `state_reader/driver_frontmatter.rs` end to end. The one moved file, `src/launcher/config_dir.rs`, is exercised by `tests/launcher_config_dir.rs` (28 passed) and by the launch e2e. No Level 4 gap: run-view data comes from parsed frontmatter and the lock probe, not static values.

### Behavioral Spot-Checks (HEAD, run this pass)

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Steering, popup, launch, new-run, guarded open, write_body, apply_edit, log delta, FIFO, launcher view, derive_status, give-back, markers, frontmatter, run view, probe_at, read_holder_at, exchange_back | `rtk proxy cargo test --no-fail-fast --lib -- launcher_steer confirm_popup launcher_launch launcher_new_run open_run_file write_body apply_edit read_log_delta fifo launcher_view derive_status a_save_landing editor_command launcher_markers driver_frontmatter driver_run probe_at read_holder_at exchange_back` | 355 passed, 0 failed | PASS |
| Launch e2e, editor stubs, config dir, lock race, DIRECTION race | `rtk proxy cargo test --no-fail-fast --test driver_tab_launch_e2e --test editor_command --test launcher_config_dir --test driverctl_lock_race --test driverctl_race` | 2 / 5 / 28 / 5 / 4 passed (1 ignored, opt-in) | PASS |
| Full regression (orchestrator, HEAD) | `rtk proxy cargo test --no-fail-fast` | 55 binaries, 3211 passed, 0 failed (was 3207; +4 from 261028-v2z) | PASS |

### Probe Execution

No probes declared by the phase plans. SKIPPED.

### Requirements Coverage

| Requirement | Source Plans | Status | Evidence |
|-------------|--------------|--------|----------|
| DTAB-01 | 30-01, 02, 03, 07 | SATISFIED | Truth 1 |
| DTAB-02 | 30-01, 02, 03, 07 | SATISFIED | Truths 2, 5 |
| DTAB-03 | 30-04, 06, 07, 08, 09 | SATISFIED | Truths 3, 6 |
| DTAB-04 | 30-02, 04, 05, 07, 09 | SATISFIED | Truth 4 |

Plan frontmatter `requirements:` across the nine plans declares exactly DTAB-01..04 (all four appear). All four are `[x]` in REQUIREMENTS.md (lines 266-279) and "Complete" in the traceability table (405-408). No orphans.

### Anti-Patterns Found

No `TBD`/`FIXME`/`XXX` markers in `src/launcher/config_dir.rs` or `tests/launcher_config_dir.rs` (grep returned nothing). No `Command::new` on the Driver tab read path.

| Finding (30-REVIEW.md at a5ac2aa: 0 Critical) | Severity | Defeats a must-have? |
|---|---|---|
| WR-13 | Warning | Fixed by 261028-v2z; re-review a5ac2aa confirms "VERIFIED FIXED on both paths". |
| WR-14, WR-15 | Warning | No. Phase 29 launcher config, off the Driver tab path. Advisory. |
| IN-18, IN-19 (and IN-10..IN-17) | Info | No. Advisory. |

Security (`30-SECURITY.md`, refresh 3a1d6d9): threats_open 0 (32/32 closed). UI review refresh (691ab6d): 22/24, no UI-facing changes.

### Human Verification

None outstanding. `30-UAT.md` is complete (2/2 passed); the UI review notes are subjective polish that touch no must-have. [inferred — cleanup 2026-10-28]

### Gaps Summary

No gaps. The WR-13 fix touched only Phase 29 launcher config and its test file, did not regress the Driver tab or the launch path, and the goal holds at HEAD. Open items (WR-14, WR-15, IN-*) are advisories outside the phase's success criteria.

---

_Verified: 2026-10-29_
_Verifier: Claude (gsd-verifier)_
