---
phase: 30-driver-tab-on-direction-md
verified: 2026-10-28T18:00:00Z
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
covered_digest: "v4:sha256:33f6c1bbf5427cd7fc5b48fce3c6bfced903e29448b56f81321b493ece9ca051"
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
  - finding: "WR-13 (open): managed_settings in src/launcher/config_dir.rs:593-616 still copies a non-array nested host hooks/permissions value verbatim, one level below the WR-08 fix."
    category: security
    reason: "The code is Phase 29 launcher config, not a Phase 30 surface; it is not on the Driver tab read or steering path and defeats none of SC1-SC4. Follow-up belongs to the Phase 29 owner. [inferred — cleanup 2026-10-28]"
    evidence_status: "30-REVIEW.md WR-13 (code-read); no failing test"
  - finding: "IN-15..IN-17: lock.rs ensure_lock_file doc says InvalidData for a directory but it is EISDIR; probe_at flattens guard refusals to Unknown(EINVAL); direction.rs read_run_file duplicates read_run_file_capped."
    category: other
    reason: "Wording, diagnostics and duplication. The guards themselves hold (symlink, hard link, FIFO and over-cap tests pass). No must-have depends on the message text. [inferred — cleanup 2026-10-28]"
    evidence_status: "30-REVIEW.md IN-15..IN-17"
---

# Phase 30: Driver Tab on DIRECTION.md Verification Report (re-verification after quick batch 261028-t3c)

**Phase Goal:** The TUI observes and steers driver runs only through the run's files. Status, caps, heartbeat and crash detection come from the tool-written frontmatter, and steering is an edit to DIRECTION.md.
**Verified:** 2026-10-28
**Status:** passed
**Re-verification:** Yes. The prior report (passed, 6/6) went stale because covered sources changed: quick batch 261028-t3c hardened `src/driver/lock.rs` (4977318: WR-11) and `src/driver/direction.rs` (7f4fc49: WR-12), and touched `src/state_reader/driver_frontmatter.rs`, `src/launcher/stop.rs` and `tests/launcher_config_dir.rs`.

`src/launcher/stop.rs` is added to the covered set because 4977318 changed it (the Driver tab's stop path calls the lock probe). The fingerprint came from `gsd-tools query verification.fingerprint`, not by hand.

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | SC1: Driver tab lists run dirs; the selected run shows status, items, caps, audit, heartbeat, `connected[]` from frontmatter, no Claude call | VERIFIED | `grep Command::new` over `driver_run.rs`, `driver_steer.rs`, `driver_launch.rs`, `driver_confirm.rs`, `steer.rs`, `driver_frontmatter.rs` returns nothing at HEAD. Run-view lib tests pass (355-test filter below). |
| 2 | SC2: `running` + free lock shown crashed; stale heartbeat flagged; dashboard marks driven/parked/crashed | VERIFIED | `derive_status` and `launcher_markers_for` tests pass, including `the_header_says_the_probe_failed_for_liveness_unknown_and_never_crashed`. WR-11 made the probe refuse symlinks, hard links and FIFOs as `Unknown` instead of hanging or following; `probe_at_reads_a_symlink_to_a_held_lock_as_unknown_not_held`, `..._hard_linked_lock_file_as_unknown`, `probe_at_reads_a_fifo_as_unknown_without_blocking` (lock.rs:720-760) pass. The "never crashed on probe failure" rule is preserved. |
| 3 | SC3: set control, answer/un-hold, open in `$EDITOR`; every edit via the shared flock, frontmatter untouched | VERIFIED | `locked()` takes `DIRECTION.lock`; `write_body`/`apply_edit` tests pass. WR-12 fix: `exchange_or_restore` (direction.rs:756-769) restores the human's save with `RenameFlags::NOREPLACE` when DIRECTION.md is absent at exchange-back. Tests `a_file_created_before_the_no_replace_restore_is_kept_and_wins`, `exchange_back_enoent_restores_the_human_save_and_retries`, `exchange_back_enoent_keeps_a_file_created_before_the_restore` (direction.rs:1408-1452) pass. `tests/editor_command.rs` 5 passed; `tests/driverctl_race.rs` 4 passed (1 opt-in ignored). |
| 4 | SC4: start a new run, relaunch a crashed one, tail LOG.md live; injection UI is not the steering path | VERIFIED | `tests/driver_tab_launch_e2e.rs` 2 passed; `launcher_new_run`, `launcher_launch`, `read_log_delta` tests pass. Injection retired in Phase 31-02. |
| 5 | Status words come from the lock probe, never the frontmatter `status:` word alone | VERIFIED | `derive_status` takes the probe result; tests pass. |
| 6 | D-15/D-22: polite stop is distinguishable from kill; confirmation says "Nothing is killed." | VERIFIED | `launcher_steer_*` and `confirm_popup` tests pass. |

**Score:** 6/6. No behavior-dependent truth is left unexercised: the flock, give-back and probe invariants each have a named passing test.

### Behavioral Spot-Checks (HEAD, run this pass)

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Steering, popup, launch, new-run, guarded open, write_body, apply_edit, log delta, FIFO, launcher view, derive_status, give-back, markers, frontmatter, run view, probe_at, read_holder_at, exchange_back | `rtk proxy cargo test --no-fail-fast --lib -- launcher_steer confirm_popup launcher_launch launcher_new_run open_run_file write_body apply_edit read_log_delta fifo launcher_view derive_status a_save_landing editor_command launcher_markers driver_frontmatter driver_run probe_at read_holder_at exchange_back` | 355 passed, 0 failed | PASS |
| Launch e2e, editor stubs, config dir, lock race, DIRECTION race | `rtk proxy cargo test --no-fail-fast --test driver_tab_launch_e2e --test editor_command --test launcher_config_dir --test driverctl_lock_race --test driverctl_race` | 2 / 5 / 27 / 5 / 4 passed (1 ignored, opt-in) | PASS |
| Full regression (orchestrator, HEAD) | `rtk proxy cargo test --no-fail-fast` | 55 suites, 3207 passed, 0 failed, 17 ignored | PASS |

### Probe Execution

No probes declared by the phase plans. SKIPPED.

### Requirements Coverage

| Requirement | Source Plans | Status | Evidence |
|-------------|--------------|--------|----------|
| DTAB-01 | 30-01, 02, 03, 07 | SATISFIED | Truth 1 |
| DTAB-02 | 30-01, 02, 03, 07 | SATISFIED | Truths 2, 5 |
| DTAB-03 | 30-04, 06, 07, 08, 09 | SATISFIED | Truths 3, 6 |
| DTAB-04 | 30-02, 04, 05, 07, 09 | SATISFIED | Truth 4 |

Plan frontmatter `requirements:` across the nine plans declares exactly DTAB-01..04. All four are `[x]` in REQUIREMENTS.md (lines 266-279), "Complete" in the traceability table (405-408), and listed in ROADMAP Phase 30. No orphans.

### Anti-Patterns Found

No `TBD`/`FIXME`/`XXX` debt markers in the changed files (the `driver_frontmatter.rs` matches are `\uXXXX` escape documentation). No `Command::new` on the Driver tab read path.

| Finding (30-REVIEW.md at cefb20f: 0 Critical) | Severity | Defeats a must-have? |
|---|---|---|
| WR-11 and WR-12 (previously open) | Warning | Now fixed by 4977318 and 7f4fc49, with pinning tests. |
| WR-13: `config_dir.rs` nested host value copied verbatim | Warning | No. Phase 29 code, off the Driver tab path. Advisory. |
| IN-15..IN-17 | Info | No. Wording, diagnostics, duplication. Advisory. |

Security (`30-SECURITY.md`, commit 8bd0b3c): threats_open 0, verified. UI review refresh (0791cae): 22/24, no UI-facing changes.

### Human Verification

None outstanding. `30-UAT.md` is complete (2/2 passed); the UI review notes are subjective polish that touch no must-have. [inferred — cleanup 2026-10-28]

### Gaps Summary

No gaps. The t3c hardening closed WR-11 and WR-12 without regressing the Driver tab; the remaining open items (WR-13, IN-15..IN-17) are advisories outside the phase's success criteria.

---

_Verified: 2026-10-28_
_Verifier: Claude (gsd-verifier)_
