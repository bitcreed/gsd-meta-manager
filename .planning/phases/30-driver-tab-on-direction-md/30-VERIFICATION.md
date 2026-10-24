---
phase: 30-driver-tab-on-direction-md
verified: 2026-10-24T05:00:00Z
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
  - tests/editor_command.rs
  - tests/launcher_config_dir.rs
covered_digest: "v3:sha256:1d0db4153ebf5a0af741cd71319e31ec90858c465d397dea2d8d7873a86da1ab"
behavior_unverified: 0
overrides_applied: 0
re_verification:
  previous_status: human_needed
  previous_score: 6/6
  gaps_closed: []
  gaps_remaining: []
  regressions: []
gaps: []
deferred: []
advisory:
  - finding: "WR-08 (open, pre-existing): DIRECTION.lock is opened by path in lock.rs acquire_at, following symlinks and truncating via set_len; a symlink planted as DIRECTION.lock in a run dir could make a steer truncate and chmod a user-writable file"
    category: security
    reason: "Hardening gap in Phase 17/27 code; needs a hostile writer already in the user's run dir; does not defeat DTAB-03 (the shared flock is still taken on a regular lock file). Recommended follow-up: O_NOFOLLOW|O_NONBLOCK plus a regular-file check in acquire_at."
    evidence_status: "30-REVIEW.md WR-08 (code-read, no failing test)"
  - finding: "WR-09 (open): run_loop's guarded read has no 1 MiB size cap, unlike the other run-dir readers"
    category: other
    reason: "Memory-only concern on a user-owned DIRECTION.md; steering and flock guarantees intact."
    evidence_status: "30-REVIEW.md WR-09"
---

# Phase 30: Driver Tab on DIRECTION.md Verification Report (re-verification, post-staleness)

**Phase Goal:** The TUI observes and steers driver runs only through the run's files. Status, caps, heartbeat and crash detection come from the tool-written frontmatter, and steering is an edit to DIRECTION.md.
**Verified:** 2026-10-24
**Status:** passed
**Re-verification:** Yes. The prior report (human_needed, 6/6) went stale because later quick tasks changed covered files. This pass checks those changes did not regress any must-have, and records that the two former human items are resolved by UAT.

Verified against COMMITTED code: a clean detached worktree at HEAD `fcf5e9e`, not the main checkout (another agent has uncommitted edits to `src/driver/direction.rs` and `tests/driverctl_race.rs`). The fingerprint above was computed in that worktree.

## What changed since the last verification (f2b53ba..HEAD, driver files)

Nine files, +246/-96. Read in full for direction.rs, steer.rs, driver_steer.rs; the rest by stat plus tests.

| Change | Effect on a must-have |
|--------|-----------------------|
| `direction.rs`: `run_loop` reads DIRECTION.md and the displaced inode through `read_run_file` (wraps `open_run_file`); exchange now gated on `linux or macos` (WR-02 fix, quick 261021-101) | Strengthens SC3. Every write is still a `write_body` closure under `DIRECTION.lock`; the frontmatter handling is unchanged. New test `steer_run_refuses_a_fifo_direction_without_blocking` (steer.rs:1858) asserts `Failed` and that the lock is released. |
| `driver_steer.rs` `EditLintConfirmScreen`: skips `fit_list` at a zero row budget (IN-04 fix) | Strengthens SC3. Test `launcher_edit_the_lint_confirmation_confirms_when_its_findings_get_no_rows` pins `y` confirming at 80x7. |
| `driver_launch.rs`: `model_name_ok` admits `[`/`]`, `relaunch_prefill` shares it (WR-07 fix) | Strengthens SC4. This closes the warning the prior report left open. |
| `driver_run.rs`, `app.rs`, `detail.rs`: glyph registry (quick 261021-1id) and the workstream/roadmap quick tasks (05g..05m, wtw) | No SC1/SC2 regression: lib tests pass; UAT test 2 drove the real binary and the layout matched 30-UI-SPEC. |

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | SC1: Driver tab lists run dirs; the selected run shows status, items, caps, audit, heartbeat, `connected[]` from frontmatter, no Claude call | VERIFIED | `grep "Command::new"` over `driver_run.rs`, `driver_steer.rs`, `driver_launch.rs`, `driver_confirm.rs`, `steer.rs`, `driver_frontmatter.rs` returns nothing. UAT test 2 (real release binary, 100x30 tmux, populated fixture) drew header, 5 items, 3 caps bars plus audit, 2 connected rows and the LOG tail. |
| 2 | SC2: `running` + free lock shown crashed; stale heartbeat flagged; dashboard marks driven/parked/crashed | VERIFIED | `derive_status` (`driver_frontmatter.rs:751`) and `launcher_markers_for` (`app.rs:387`) present and exercised; UAT capture shows `x crashed ... reason run lock released without a final status`. Lib tests pass. |
| 3 | SC3: set control, answer/un-hold, open in `$EDITOR`; every edit via the shared flock, frontmatter untouched | VERIFIED | steer.rs doc and code: every DIRECTION.md write is a `write_body` closure (`steer.rs:21`). `editor_command` (`app.rs:839`) with tests `editor_command_*` and `tests/editor_command.rs` (5 passed). FIFO test above. |
| 4 | SC4: start a new run, relaunch a crashed one, tail LOG.md live; injection UI is not the steering path | VERIFIED | `tests/driver_tab_launch_e2e.rs` 2 passed; `launcher_new_run`, `launcher_launch`, `read_log_delta` tests pass. |
| 5 | Status words come from the lock probe, never the frontmatter `status:` word alone | VERIFIED | `derive_status` takes the probe; unchanged by the later commits. |
| 6 | D-15/D-22: polite stop is distinguishable from kill; the confirmation says "Nothing is killed." | VERIFIED | `launcher_steer_the_polite_stop_sentence_survives_every_width` and the `confirm_popup` clip guard are in the passing lib run. |

**Score:** 6/6 truths verified. No behavior-dependent truth lacks a test (cancel/rollback/ordering paths have named tests listed above).

### Behavioral Spot-Checks (committed worktree, targeted, under `nice`)

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Steering, popup, launch, new-run, guarded open, write_body, apply_edit, log delta, FIFO, launcher view | `cargo test --lib -- launcher_steer confirm_popup launcher_launch launcher_new_run open_run_file write_body apply_edit read_log_delta fifo launcher_view` | 182 passed, 0 failed | PASS |
| Broader driver, editor, launcher lib filter | `cargo test --lib -- driver editor launcher` | 1083 passed, 0 failed | PASS |
| New-run/relaunch e2e, editor stubs | `cargo test --test driver_tab_launch_e2e --test editor_command` | 2 passed; 5 passed | PASS |
| Launcher config-dir | `cargo test --test launcher_config_dir` | 11 passed | PASS |

The full suite was not re-run, by instruction. The orchestrator's earlier full run (4068 passed; the one failure is the known machine-local git-version witness) predates the quick tasks; the targeted gate above covers the changed driver surface.

### Requirements Coverage

| Requirement | Source Plans | Status | Evidence |
|-------------|--------------|--------|----------|
| DTAB-01 | 30-01, 02, 03, 07 | SATISFIED | Truth 1 |
| DTAB-02 | 30-01, 02, 03, 07 | SATISFIED | Truth 2 |
| DTAB-03 | 30-04, 06, 07, 08, 09 | SATISFIED | Truths 3 and 6 |
| DTAB-04 | 30-02, 04, 05, 07, 09 | SATISFIED | Truth 4 |

All four IDs appear in plan frontmatter, are `[x]` in REQUIREMENTS.md (lines 233-245) and "Complete" in the traceability table (365-368). No orphans.

### Anti-Patterns Found

No `TBD`/`FIXME`/`XXX` markers in the phase's source files (only `\uXXXX` doc text). No `Command::new` in the Driver tab sources.

Open review findings (`30-REVIEW-DISPOSITION.md`: WR-08, WR-09, IN-06, IN-07 open; IN-05 deferred; WR-02, WR-07, IN-04 and eight others fixed), judged against the must-haves:

| Finding | Severity | Defeats a must-have? |
|---------|----------|----------------------|
| WR-08: `DIRECTION.lock` opened by path, follows symlinks, truncates (`lock.rs` `acquire_at`) | Warning | No. Pre-existing Phase 17/27 code, outside the phase's file set. Needs a hostile writer already in the user's run dir (an `--unsandboxed` run can already act as the user). The shared flock still holds for a regular lock file; frontmatter is untouched. Recorded in `advisory`; recommend fixing in a hardening follow-up. |
| WR-09: no size cap on `run_loop`'s guarded read | Warning | No. Memory only, user-owned file. |
| IN-05 (deferred): body-relative vs file-relative `DIRECTION.md:<line>` across tools | Info | No. |
| IN-06: macOS exchange failure on a volume without `RENAME_SWAP` reports the Linux syscall name | Info | No. |
| IN-07: at a zero row budget both lint popups drop every finding, leaving a colon-ended header | Info | No. The popup stays confirmable. |

### Human Verification

None outstanding. Both former items are resolved by `30-UAT.md` (status complete, 2/2 passed, commit 4af7366): glyphs and labels were operator-signed on the real terminal; FULL-tier density at 100x30 was settled by driving the real release binary against a populated fixture, with the captured screen matching 30-UI-SPEC. No concrete reason to reopen either.

### Gaps Summary

No gaps. The later quick tasks either strengthen Phase 30 (WR-02, WR-07 and IN-04 fixes, with tests) or touch unrelated surfaces (workstream/roadmap views), and none regressed a success criterion or a requirement. The two open warnings (WR-08, WR-09) are hardening of shared, pre-existing code and are carried as advisories, not blockers.

---

_Verified: 2026-10-24_
_Verifier: Claude (gsd-verifier)_
