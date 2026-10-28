---
phase: 30-driver-tab-on-direction-md
verified: 2026-10-28T12:00:00Z
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
covered_digest: "v4:sha256:38e0946a1688397e72fd196dc44abd0658836ef3c051a6cce149fe3d61f0e29d"
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
  - finding: "WR-11 (open, new): probe_at / read_holder_at in src/driver/lock.rs open launcher.lock with a blocking File::open that follows symlinks; a FIFO planted at launcher.lock parks one worker thread per ~5 s scan."
    category: security
    reason: "Needs a hostile writer already inside the user's own run/launcher directory (same trust boundary as the pre-existing WR-08, whose open-with-no-follow fix landed in 670e099 for the lock-acquire path). It is a denial-of-service on the scan worker, not on status derivation of other runs, and defeats none of SC1-SC4. 30-SECURITY.md stays threats_open 0. Follow-up: O_NONBLOCK|O_NOFOLLOW on the read-only opens. [inferred — cleanup 2026-10-28]"
    evidence_status: "30-REVIEW.md WR-11 (code-read: lock.rs read_holder_at still uses File::open); no failing test"
  - finding: "WR-12 (open, new): if DIRECTION.md is absent at the first exchange-back in run_loop (direction.rs ~901-912), the human's newest save is left at a dot-named temp that the 60 s stale-temp sweep can delete. Residual of WR-10 (44eef32 restored the common path with a no-replace rename)."
    category: other
    reason: "Needs an unlink-then-create save landing in a microsecond window between two exchanges, on top of a concurrent tool splice. The shared flock, the frontmatter-untouched guarantee and the error report all hold; no success criterion promises zero loss in that window. [inferred — cleanup 2026-10-28]"
    evidence_status: "30-REVIEW.md WR-12 (code-read at direction.rs; error path returns Err and never unlinks, confirmed); no failing test"
---

# Phase 30: Driver Tab on DIRECTION.md Verification Report (re-verification after Phases 28.1/31 and quick 261028-n0z/n10/n12)

**Phase Goal:** The TUI observes and steers driver runs only through the run's files. Status, caps, heartbeat and crash detection come from the tool-written frontmatter, and steering is an edit to DIRECTION.md.
**Verified:** 2026-10-28
**Status:** passed
**Re-verification:** Yes. The prior report (passed, 6/6, HEAD 4c872fa) went stale: later commits touched covered files (Phase 31 deletions 77908eb/2a23d9e/9d22292, quick 261028-n0z WR-08/09/10 fixes 670e099/0f85617/44eef32, macOS steering 76a9ecb, quick n12).

The covered set was rebuilt from what exists at HEAD. `tests/driver_lock.rs` was deleted by 2a23d9e (Phase 31-04, the hidden `drive` subcommand with its tests); the lock-holder coverage it gave lives on in `tests/driverctl_lock_race.rs` (5 passed). `src/driver/lock.rs` is added to the set because 670e099 changed it. Fingerprint computed with `gsd_run query verification.fingerprint`, not by hand.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | SC1: Driver tab lists run dirs; the selected run shows status, items, caps, audit, heartbeat, `connected[]` from frontmatter, no Claude call | VERIFIED | `grep Command::new` over `driver_run.rs`, `driver_steer.rs`, `driver_launch.rs`, `driver_confirm.rs`, `steer.rs`, `driver_frontmatter.rs` returns nothing at HEAD. Phase 31 removed only the legacy journal-run section (`driver.rs:12,170`); the Phase 30 run view (`driver_run.rs`) is intact and its lib tests pass. Todo 2026-07-29 TestBackend sweep (heights 8-17 x widths 80/120, populated and empty) holds. |
| 2 | SC2: `running` + free lock shown crashed; stale heartbeat flagged; dashboard marks driven/parked/crashed | VERIFIED | `derive_status` (`driver_frontmatter.rs:795`) and `launcher_markers_for` (`app.rs:153`) present; `RunStatus::Crashed` at `driver_frontmatter.rs:110`; `launcher_view_*`, `derive_status`, `launcher_markers` tests in the 340-test filter pass, including `the_header_says_the_probe_failed_for_liveness_unknown_and_never_crashed`. |
| 3 | SC3: set control, answer/un-hold, open in `$EDITOR`; every edit via the shared flock, frontmatter untouched | VERIFIED | `locked()` (`direction.rs:655`) takes `DIRECTION.lock` and returns `NoDirection` creating nothing when the file is absent; `write_body`/`apply_edit` tests pass. WR-08 (symlink-following lock open) and WR-09 (no size cap) are now fixed (670e099, 0f85617) and WR-10's common path restored by no-replace rename (44eef32); `tests/editor_command.rs` 5 passed, `tests/driverctl_race.rs` 4 passed (1 opt-in ignored). |
| 4 | SC4: start a new run, relaunch a crashed one, tail LOG.md live; injection UI is not the steering path | VERIFIED | `tests/driver_tab_launch_e2e.rs` 2 passed; `launcher_new_run`, `launcher_launch`, `read_log_delta` tests pass. Injection is retired: `driver_run.rs:2640` `INJECTION_RETIRED` message points to `[c]ontrol, [a]nswer, [e]dit`; Phase 31-02 deleted the injection UI. macOS launch/steer/stop enabled by 76a9ecb. |
| 5 | Status words come from the lock probe, never the frontmatter `status:` word alone | VERIFIED | `derive_status` takes the probe result; tests above pass. |
| 6 | D-15/D-22: polite stop is distinguishable from kill; confirmation says "Nothing is killed." | VERIFIED | `launcher_steer_*` and `confirm_popup` tests in the passing filter. |

**Score:** 6/6. No behavior-dependent truth is left unexercised: the cancellation/ordering invariants (flock, give-back) have named passing tests.

### Behavioral Spot-Checks (HEAD, run this pass)

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Steering, popup, launch, new-run, guarded open, write_body, apply_edit, log delta, FIFO, launcher view, derive_status, editor_command, markers, give-back, frontmatter, run view | `rtk proxy cargo test --no-fail-fast --lib -- launcher_steer confirm_popup launcher_launch launcher_new_run open_run_file write_body apply_edit read_log_delta fifo launcher_view derive_status a_save_landing editor_command launcher_markers driver_frontmatter driver_run` | 340 passed, 0 failed | PASS |
| Launch e2e, editor stubs, config dir, lock race, DIRECTION race | `rtk proxy cargo test --no-fail-fast --test driver_tab_launch_e2e --test editor_command --test launcher_config_dir --test driverctl_lock_race --test driverctl_race` | 2 / 5 / 26 / 5 / 4 passed (1 ignored, opt-in) | PASS |
| Full regression (orchestrator, HEAD 7902043) | `rtk proxy cargo test --no-fail-fast` | 3183 passed, 0 failed, 17 ignored, 55 suites, exit 0 (includes `tests/driver_router_conformance.rs`) | PASS |

### Probe Execution

No probes declared by the phase plans. SKIPPED.

### Requirements Coverage

| Requirement | Source Plans | Status | Evidence |
|-------------|--------------|--------|----------|
| DTAB-01 | 30-01, 02, 03, 07 | SATISFIED | Truth 1 |
| DTAB-02 | 30-01, 02, 03, 07 | SATISFIED | Truths 2, 5 |
| DTAB-03 | 30-04, 06, 07, 08, 09 | SATISFIED | Truths 3, 6 |
| DTAB-04 | 30-02, 04, 05, 07, 09 | SATISFIED | Truth 4 |

All four IDs are `[x]` in REQUIREMENTS.md (lines 266-279), "Complete" in the traceability table (405-408), and listed in ROADMAP Phase 30. Plan frontmatter declares exactly these four across the nine plans. No orphans.

### Anti-Patterns Found

No `TBD`/`FIXME`/`XXX` debt markers in phase-owned code. The grep matches in `detail.rs` are `\uXXXX`/`U+XXXX` escape documentation and test fixtures for roadmap "plans TBD" text, not debt. No `Command::new` on the Driver tab read path.

| Finding (30-REVIEW.md re-run, disposition 1082819: 0 Critical, 2 Warning, 5 Info) | Severity | Defeats a must-have? |
|---|---|---|
| WR-11: blocking `File::open` on `launcher.lock` follows symlinks / parks on a FIFO (`lock.rs` `probe_at`, `read_holder_at`) | Warning | No. Hostile writer inside the user's own directory; scan-worker DoS only; SC1-SC4 unaffected. Advisory. |
| WR-12: DIRECTION.md missing at the first exchange-back leaves the human's save at a sweepable temp (`direction.rs` ~901-912) | Warning | No. Microsecond-window double race; flock/frontmatter/error-report guarantees hold. Advisory. |
| IN-05..IN-09 | Info | No. Wording and test-witness items. |

Security (`30-SECURITY.md`, 9140761): threats_open 0, SECURED, 32/32 closed.

### Human Verification

None outstanding. `30-UAT.md` is complete (2/2 passed). `30-UI-REVIEW.md` scores 22/24 with no pillar below 3; its remaining notes (density at 100x30, DarkGray/DIM legibility per theme, glyph rendering per font) are subjective polish that does not affect any must-have, and the layout todo is closed by a TestBackend sweep (2f16c70). The only spec-level finding is that `30-UI-SPEC.md` still describes the deleted legacy section, which is documentation drift, not a code gap. [inferred — cleanup 2026-10-28]

### Gaps Summary

No gaps. Phase 31 deleted the legacy Driver-tab section and the hidden `drive` subcommand without disturbing the Phase 30 DIRECTION.md-driven surface; the cleanup batch closed WR-08 and WR-09 and most of WR-10. Two new narrow review warnings (WR-11, WR-12) are recorded as advisories with concrete follow-ups.

---

_Verified: 2026-10-28_
_Verifier: Claude (gsd-verifier)_
