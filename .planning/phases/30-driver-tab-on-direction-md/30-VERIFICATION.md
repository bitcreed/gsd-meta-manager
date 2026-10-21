---
phase: 30-driver-tab-on-direction-md
verified: 2026-10-21T04:00:00Z
status: gaps_found
score: 5/6 must-haves verified
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
  - src/driver/steer.rs
  - src/state_reader/driver_frontmatter.rs
  - src/ui/screens/confirm_popup.rs
  - src/ui/screens/driver_run.rs
  - src/ui/screens/driver_steer.rs
covered_digest: "v3:sha256:613aa3d420aced44fc619c12481ac0b7045f50766a9a40811a59d90c36586a2d"
behavior_unverified: 0
overrides_applied: 0
gaps:
  - truth: "D-15/D-22: the polite-stop confirmation tells the user 'Nothing is killed.' (the sentence that separates polite stop from kill)"
    status: failed
    reason: "popup_rect sizes the body with ceil(chars/width) but the Paragraph word-wraps (Wrap{trim:false}), so the body is one row short for realistic run ids. The final sentence is clipped at every width >= 80 while [Yes] stays live. Confirmed by arithmetic (135-char prompt, inner width 70 -> word-wrap gives 3 rows, popup gives 2) and by the UI audit snapshot c4_polite_stop_confirm_80x24."
    artifacts:
      - path: "src/ui/screens/confirm_popup.rs"
        issue: "wrapped_rows (lines 151-161) counts ceil(chars/width) instead of simulating word wrap; popup_rect (139-149) is therefore one row short; render_confirm_popup gives the body inner.height-3 rows (line ~200)"
      - path: "src/ui/screens/driver_steer.rs"
        issue: "PoliteStopScreen::prompt (315-320) / render (363-380) pass the long single-line prompt through that sizing; the existing test (1132-1144) uses a short run id so it passes"
    missing:
      - "Make wrapped_rows count word-wrapped lines (or add a spare row), then add a render test at 80x24 with an 18-char run id like 20261019-0042-abcd that asserts the whole sentence 'Nothing is killed.' is visible"
deferred: []
human_verification:
  - test: "Read DarkGray labels and the glyphs U+2016, U+25C8, U+2691 on the real terminal theme/font"
    expected: "Labels are legible; glyphs render as single cells"
    why_human: "Terminal theme and font are unobservable from a TestBackend"
  - test: "Judge the density of the FULL tier at 100x30"
    expected: "Header, items, caps, connected and log read as a coherent view"
    why_human: "Subjective layout quality"
---

# Phase 30: Driver Tab on DIRECTION.md Verification Report

**Phase Goal:** The TUI observes and steers driver runs only through the run's files. Status, caps, heartbeat and crash detection come from the tool-written frontmatter, and steering is an edit to DIRECTION.md.
**Verified:** 2026-10-21
**Status:** gaps_found
**Re-verification:** No, initial verification

## Goal Achievement

### Observable Truths

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | SC1: Driver tab lists run dirs; selected run shows status, per-item state, caps used/limit, dispatch audit (logged==observed), heartbeat age, `connected[]`, all from frontmatter with no Claude call | VERIFIED | `list_driver_runs` (`driver_frontmatter.rs:875`) is the run list, newest first, via `LAUNCHER_RUNS_LISTED`. `caps_section` (`driver_run.rs:759`) with `audit_row` (647, Red + MISMATCH when logged != observed), `connected_section_fit` (820), `heartbeat_spans` (928), `merged_items` (1226). Rendered by `render_launcher_tab` (called from `detail.rs:6577` and `driver.rs:1949`). `steer.rs`/`driver_run.rs`/`driver_steer.rs` contain no `Command::new` or claude spawn. 308 targeted lib tests pass. |
| 2 | SC2: `running` + free lock is shown crashed; stale heartbeat flagged; dashboard marks driven/parked/crashed (OBS-02/07) | VERIFIED | `derive()`/`derive_status` (`driver_frontmatter.rs:704-761`) turns Free/Absent into `Crashed`, Unknown into `LivenessUnknown`, and nothing is written back. Probe is read-only (`probe_at`). `heartbeat_stale` (`driver_run.rs:427`) uses the file's own `heartbeat_stale_after_s`, only for live runs. `launcher_markers_for` (`app.rs:387`) runs in the 20-tick reconcile (`app.rs:2134`), stores `cache.launcher_marker` (`app.rs:3288`), and `normal.rs:244-284` ranks Crashed/Parked/Driven/Stale/Unknown. Tests `launcher_marker_a_free_lock_running_run_is_crashed`, `a_held_lock_with_an_old_heartbeat_reads_stale_and_a_free_lock_reads_crashed`, `list_driver_runs_derives_the_status_from_the_lock_not_the_written_word` pass. |
| 3 | SC3: set control run/pause/stop, answer or un-hold a parked item, open DIRECTION.md in `$EDITOR`; every edit through the shared flock, frontmatter untouched | VERIFIED | Keys `c`, `a`, `u`, `e`, `o` in `detail.rs:5356-5684`. The only DIRECTION.md writers in the phase are two `write_body` closures (`steer.rs:551`, `787`); `write_body` (`direction.rs:426`) holds `DIRECTION.lock`, runs the exchange-verify loop and keeps the tool block verbatim. Tests `launcher_edit_prepare_then_rewrite_then_apply_applies_and_keeps_the_tool_block`, `launcher_edit_a_save_landing_while_the_write_waits_for_the_lock_is_kept`, and symlink/stale-temp tests pass. |
| 4 | SC4: start a new run, relaunch a crashed one, tail LOG.md live; injection UI no longer the steering path | VERIFIED | `s` (`detail.rs:5490`), `R` (5513), `read_log_delta` byte-offset tail wired through `mod.rs:2160/2214` into `LauncherRunsRefreshed`/`LauncherRunRefreshed` (`app.rs:3195-3270`, stale reads dropped). `i` is retired (`detail.rs:5341`, shows `INJECTION_RETIRED`, opens nothing). `tests/driver_tab_launch_e2e.rs`: 2/2 pass (new-run and crashed-relaunch build the real launcher argv; zero spend). |
| 5 | Dashboard/tab status words come from the lock probe, never the frontmatter `status:` word alone | VERIFIED | Covered under truths 1-2; `status_cell`/`base_status_cell` consume `DerivedStatus` only. |
| 6 | D-15/D-22: polite stop is distinguishable from kill; the confirmation says "Nothing is killed." | FAILED | See gap. `confirm_popup.rs:151-161` undercounts wrapped rows, so the sentence is cut off for a realistic run id. |

**Score:** 5/6 truths verified

### Requirements Coverage

| Requirement | Source Plans | Description | Status | Evidence |
|-------------|--------------|-------------|--------|----------|
| DTAB-01 | 30-01, 30-02, 30-03 | List runs; show status, items, caps, audit, heartbeat, connected from frontmatter | SATISFIED | Truth 1 |
| DTAB-02 | 30-01, 30-02, 30-03 | Crashed and stale flagged; dashboard marks driven/parked/crashed | SATISFIED | Truth 2 |
| DTAB-03 | 30-04 | Set control, answer/un-hold, open in `$EDITOR`; shared flock; frontmatter untouched | SATISFIED (polite-stop confirm copy clipped, see gap) | Truth 3 |
| DTAB-04 | 30-02, 30-04, 30-05 | Start, relaunch, tail LOG.md | SATISFIED | Truth 4 |

All four IDs appear in plan frontmatter and in REQUIREMENTS.md (lines 231-247, traceability 365-368). No orphaned requirements.

### Behavioral Spot-Checks

| Behavior | Command | Result | Status |
|----------|---------|--------|--------|
| Phase 30 lib tests (driver_run, driver_steer, driver_launch, driver_confirm, steer, frontmatter, watcher, direction) | `cargo test --lib -- <filters>` | 308 passed, 0 failed | PASS |
| New-run and relaunch e2e | `cargo test --test driver_tab_launch_e2e` | 2 passed | PASS |
| Full suite (orchestrator) | `cargo test --no-fail-fast` | 4022 passed; 2 failed (git-version witness: environment-only; `launcher_config_dir` ETXTBSY flake, passed 3/3 in isolation) | PASS (accepted) |
| clippy Linux | `cargo clippy --all-targets -D warnings` (orchestrator) | clean | PASS |

### Anti-Patterns Found

No unreferenced TBD/FIXME/XXX debt markers in files changed by the phase (the matches in `detail.rs` are pre-existing test fixture strings). Review findings are listed below; all nine are still `open` in `30-REVIEW-DISPOSITION.md`.

| File | Line | Pattern | Severity | Impact |
|------|------|---------|----------|--------|
| `src/ui/screens/confirm_popup.rs` | 151-161 | Row count ignores word wrap | Blocker (the gap) | Clips "Nothing is killed." |
| `src/ui/screens/driver_launch.rs` | 916-928 (WR-05) | Spend and "keeps running" paragraph is last in the popup body | Warning | Clipped first on short terminals while `y` is live; UI audit D5 shows the same at 80x12 |
| `src/driver/steer.rs` / `src/ui/screens/driver_launch.rs` | WR-01/02/03/04, IN-01..03 | Model validation mismatch, no file-type guard on DIRECTION.md open, no-op rewrites, two line-number bases, unterminated heading, stuck pending slug, two `control` readers | Warning | None blocks a success criterion; frontmatter is still preserved byte for byte |
| `src/main.rs` | 847 (WR-06) | `$EDITOR` with arguments cannot be spawned | Warning | `EDITOR="code -w"` fails with a clean spawn-error status; user body untouched |
| `src/ui/screens/driver_run.rs` | 2472 (UI D2) | "no detail yet" shown as "no control line" in Yellow | Warning | False warning colour while loading |
| `src/ui/screens/driver_confirm.rs` | 445 (UI D6) | Kill confirm paints the dashboard behind it | Warning | Context loss on the destructive action |

Platform note (orchestrator evidence): two test-only helpers (`src/app.rs` `next_matching`, `src/ui/screens/detail.rs` `give_control`) are dead code under `-D warnings` on macOS; CI is Linux-only and the macOS clippy run already fails on pre-phase-30 errors. Not a Phase 30 goal item.

### Human Verification Required

Only subjective or real-terminal items remain (listed in the frontmatter): DarkGray label contrast and the `‖ ◈ ⚑` glyph rendering on the user's terminal, and the density of the FULL tier at 100x30. Everything else was settled from code and tests.

### Gaps Summary

All four ROADMAP success criteria and all four requirement IDs are met in the code, wired end to end, and exercised by passing tests. The one failed truth is the polite-stop confirmation: the sentence that separates it from a kill ("Nothing is killed.") is clipped for a realistic run id at every width of 80 or more, while the Yes button stays live. This is the locked D-22 copy contract on the single prompt where a user chooses between the two. The fix is small (count word-wrapped rows in `wrapped_rows`, or add a spare row, and add a render test with a realistic run id). The UI audit's other warnings and the six open code-review warnings are recorded above and do not block a success criterion, but WR-05 and UI D5 deserve to land in the same gap-closure plan because they also hide decision content while `y` is armed.

---

_Verified: 2026-10-21_
_Verifier: Claude (gsd-verifier)_
