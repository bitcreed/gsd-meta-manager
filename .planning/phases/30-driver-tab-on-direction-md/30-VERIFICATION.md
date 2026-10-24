---
phase: 30-driver-tab-on-direction-md
verified: 2026-10-24T20:00:00Z
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
  - tests/driver_lock.rs
  - tests/driver_tab_launch_e2e.rs
  - tests/driverctl_race.rs
  - tests/editor_command.rs
  - tests/launcher_config_dir.rs
covered_digest: "v3:sha256:29df7d459548705d513cc97b0bb8a29e30d2122e475166db057dc8f45750e925"
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
  - finding: "WR-10 (open, new in 717d724): give_back_newest handles an exchange error (including ENOENT when DIRECTION.md is absent between the exchange back and the give-back) by returning Err and leaving the human's newest save at a dot-named temp, which the 60 s stale-temp sweep can later delete. The pre-717d724 plain rename restored it."
    category: other
    reason: "Needs an unlink-then-create save (or a git checkout) landing in a microsecond window between two exchanges, on top of a concurrent tool splice. The flock, the frontmatter-untouched guarantee and the Failed outcome all hold; it narrows no success criterion but can lose one human save. Recommended follow-up: NOREPLACE rename on ENOENT, as 30-REVIEW.md WR-10 sketches. [inferred - audit] judged non-blocking."
    evidence_status: "30-REVIEW.md WR-10 (code-read at direction.rs give_back_newest, confirmed here; no failing test)"
  - finding: "WR-08 (open, pre-existing): DIRECTION.lock is opened by path in lock.rs acquire_at, following symlinks and truncating via set_len"
    category: security
    reason: "Hardening gap in Phase 17/27 code; needs a hostile writer already in the user's run dir; does not defeat DTAB-03. 30-SECURITY.md threats_open 0."
    evidence_status: "30-REVIEW.md WR-08 (code-read, no failing test)"
  - finding: "WR-09 (open): read_run_file in direction.rs has no 1 MiB size cap; widened by 717d724 (displaced and given-back files are read too)"
    category: other
    reason: "Memory-only concern on a user-owned DIRECTION.md; steering and flock guarantees intact."
    evidence_status: "30-REVIEW.md WR-09"
---

# Phase 30: Driver Tab on DIRECTION.md Verification Report (re-verification after quick 261023-v1s)

**Phase Goal:** The TUI observes and steers driver runs only through the run's files. Status, caps, heartbeat and crash detection come from the tool-written frontmatter, and steering is an edit to DIRECTION.md.
**Verified:** 2026-10-24
**Status:** passed
**Re-verification:** Yes. The prior report (passed, 6/6) went stale because quick task 261023-v1s changed `src/driver/direction.rs` (717d724), `src/driverctl_cli.rs` (801c656) and `tests/driver_lock.rs` / `tests/driverctl_race.rs` (d960678, 717d724). This pass checks that those changes regressed no must-have.

Verified against HEAD `4c872fa`. `git status` shows only `.planning/` edits (musk ledger, milestone lock), so the source tree is the committed tree.

## What changed since the last verification (7f1f044..HEAD, covered files)

| Change | Effect on a must-have |
|--------|-----------------------|
| `direction.rs` `give_back_newest`: a newer rename-save is put back by `RENAME_EXCHANGE` and repeated until what comes back is the older file (max 8 rounds), instead of a plain rename (717d724) | Strengthens SC3: closes a lost-update race in the shared write path. Every write is still a `write_body`/`splice` closure inside `locked()` under `DIRECTION.lock`; the frontmatter block handling (`block.unwrap_or_default()` prefix) is unchanged. Regression test `a_save_landing_right_after_the_exchange_back_is_not_reverted` passes. |
| `driverctl_cli.rs`: relay a reply queued before the server closed (801c656) | Outside SC1-SC4 (driverctl client transport). No Driver tab surface touched. |
| `tests/driver_lock.rs`: wait for `exec_started` before killing the lock holder (d960678); `tests/driverctl_race.rs`: one-splice yield handshake | Test-timing only. Both suites pass (8 and 4 passed). |

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | SC1: Driver tab lists run dirs; the selected run shows status, items, caps, audit, heartbeat, `connected[]` from frontmatter, no Claude call | VERIFIED | `grep Command::new` over `driver_run.rs`, `driver_steer.rs`, `driver_launch.rs`, `driver_confirm.rs`, `steer.rs`, `driver_frontmatter.rs` returns nothing. No file in the 7f1f044..HEAD diff is on the read path. UAT 2/2 (real binary, populated fixture) from the prior pass still stands: the diff touches no screen file. |
| 2 | SC2: `running` + free lock shown crashed; stale heartbeat flagged; dashboard marks driven/parked/crashed | VERIFIED | `derive_status` (`driver_frontmatter.rs:751`) and `launcher_markers_for` (`app.rs:387`) present; `launcher_view_*` tests pass in the 189-test lib filter below. |
| 3 | SC3: set control, answer/un-hold, open in `$EDITOR`; every edit via the shared flock, frontmatter untouched | VERIFIED | `locked()` takes `DIRECTION.lock` before `run_loop`; `write_body` re-prefixes the unchanged frontmatter block; `Plan::Keep` skips no-op writes. Tests `launcher_edit_a_save_landing_while_the_write_waits_for_the_lock_is_kept`, `launcher_steer_a_held_lock_past_the_budget_still_writes_once_and_says_so`, `editor_command_*` and `tests/editor_command.rs` (5 passed), `two_writers_under_direction_lock_lose_nothing` and `editor_race_smoke` pass. |
| 4 | SC4: start a new run, relaunch a crashed one, tail LOG.md live; injection UI is not the steering path | VERIFIED | `tests/driver_tab_launch_e2e.rs` 2 passed; `launcher_new_run`, `launcher_launch`, `read_log_delta` tests pass. |
| 5 | Status words come from the lock probe, never the frontmatter `status:` word alone | VERIFIED | `derive_status` takes the probe; file unchanged by the diff. |
| 6 | D-15/D-22: polite stop is distinguishable from kill; confirmation says "Nothing is killed." | VERIFIED | `launcher_steer_the_polite_stop_sentence_survives_every_width` and `confirm_popup` guards in the passing lib filter. |

**Score:** 6/6. The only behavior-dependent code that changed (the give-back ordering invariant) has a named passing test that was red on the base code per the 717d724 message; no truth is left behavior-unverified.

### Behavioral Spot-Checks (HEAD, targeted, under `nice`, raw output via `rtk proxy`)

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Steering, popup, launch, new-run, guarded open, write_body, apply_edit, log delta, FIFO, launcher view, derive_status, editor_command, markers, give-back | `cargo test --lib -- launcher_steer confirm_popup launcher_launch launcher_new_run open_run_file write_body apply_edit read_log_delta fifo launcher_view derive_status a_save_landing editor_command launcher_markers` | 189 passed, 0 failed | PASS |
| Give-back ordering invariant | `cargo test --lib -- a_save_landing_right_after` | 1 passed | PASS |
| New-run/relaunch e2e, editor stubs, config dir | `cargo test --test driver_tab_launch_e2e --test editor_command --test launcher_config_dir` | 2 / 5 / 11 passed | PASS |
| Lock-holder and editor-race integration suites | `cargo test --test driver_lock --test driverctl_race` | 8 passed; 4 passed, 1 ignored (`editor_race_full`, opt-in) | PASS |

Orchestrator regression gate (`cargo test --no-fail-fast`): 4205 passed, 1 failed. The failure is `envelope::policy::tests::the_config_section_constants_record_the_git_version_they_were_derived_against`, the known local-only git-version witness (local git differs from the pinned CI git), unrelated to this phase. `editor_race_smoke` (known flake) passed here.

### Requirements Coverage

| Requirement | Source Plans | Status | Evidence |
|-------------|--------------|--------|----------|
| DTAB-01 | 30-01, 02, 03, 07 | SATISFIED | Truth 1 |
| DTAB-02 | 30-01, 02, 03, 07 | SATISFIED | Truths 2, 5 |
| DTAB-03 | 30-04, 06, 07, 08, 09 | SATISFIED | Truths 3, 6 |
| DTAB-04 | 30-02, 04, 05, 07, 09 | SATISFIED | Truth 4 |

All four IDs are `[x]` in REQUIREMENTS.md (lines 233-245) and "Complete" in the traceability table (365-368); ROADMAP lists the same four. No orphans.

### Anti-Patterns Found

No `TBD`/`FIXME`/`XXX` markers in the changed source or test files. No `Command::new` in the Driver tab sources.

Open review findings against the must-haves (`30-REVIEW.md` re-run, `30-REVIEW-DISPOSITION.md`: WR-08, WR-09, WR-10, IN-06, IN-07, IN-08, IN-09 open; IN-05 deferred):

| Finding | Severity | Defeats a must-have? |
|---------|----------|----------------------|
| WR-10: give-back error leaves the human's newest save at a temp that the stale sweep can delete (`direction.rs` `give_back_newest`) | Warning | No. Needs a delete-then-create save in a microsecond window after a concurrent splice. The shared flock, frontmatter guarantee and error reporting hold; no success criterion promises that this window loses nothing. Carried as advisory. [inferred - audit] |
| WR-08: `DIRECTION.lock` follows symlinks (`lock.rs`) | Warning | No. Pre-existing Phase 17/27 code; `30-SECURITY.md` threats_open 0. |
| WR-09: `read_run_file` has no size cap | Warning | No. Memory only. |
| IN-05/06/07/08/09 | Info | No. Message wording, test-witness wording, and a not-realistic exhaustion path. |

### Human Verification

None outstanding. `30-UAT.md` is complete (2/2 passed). The diff since touches no screen, layout or glyph file, so there is no reason to reopen either item. [inferred - audit]

### Gaps Summary

No gaps. Quick task 261023-v1s hardened the shared DIRECTION.md write path against a lost-update race (with a test that is red on the base code) and fixed two flaky tests. None of the four success criteria or four requirement IDs regressed. WR-10 is the one new defect the change introduced; it is a narrow data-loss edge and is recorded as an advisory with a concrete fix, not a blocker.

---

_Verified: 2026-10-24_
_Verifier: Claude (gsd-verifier)_
