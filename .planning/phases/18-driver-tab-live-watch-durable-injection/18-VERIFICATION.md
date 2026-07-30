---
phase: 18-driver-tab-live-watch-durable-injection
verified: 2026-07-29T00:00:00Z
status: human_needed
score: 14/14 must-haves verified (code-level); 0 failed; 2 items routed to human verification
behavior_unverified: 0
overrides_applied: 0
human_verification:
  - test: "Open the Driver tab (`Shift+D`) against a real (or fake-claude fixture) live run at a terminal height between 8 and 13 rows (the 'medium' tier)."
    expected: "The D-R-P-E-V pipeline row and the step-timeline rule render their content, not a blank line / bare `── steps ──` rule with nothing under it; the elapsed-time counter is visible even when the goal wraps to two rows."
    why_human: "WR-05 was explicitly deferred by the code review (18-REVIEW.md) as a layout judgement across three height tiers that needs a human check at real terminal heights 8/13/14, not a buffer-scrape a test can settle alone. No render test exercises this tier today. It does not block any of the 5 roadmap success criteria at ordinary (>=14 row) terminal heights, but a human should confirm the height-tier fix (or accept the deferral) before shipping to users on small panes."
  - test: "Start a real GSD command against an opted-in test project from the Driver tab's start wizard, watch the live output pane stream assistant turns, type a steering message with `i` while the command is mid-turn, and watch it progress queued -> delivered -> acted-on across the ~55s dequeue gap; then stop the run and confirm the after-the-fact review (goal, command, step timeline, terminal state) reads correctly."
    expected: "Output streams live and is legible (not a Debug dump); elapsed time updates while the tab is open; the injected message visibly reaches acted-on with no animation/spinner; the finished run's review pane shows the correct goal, one decided step, and an honest terminal state derived from the outcome, never from agent prose."
    why_human: "This is the full interactive user flow (TRANS-05, OBS-04, OBS-05, STEER-01/02) that the phase's own integration tests exercise against fake-claude fixtures rather than a rendered terminal. The code-level evidence for every state transition and every code-review CRITICAL/WARNING fix is strong (see below), but the visual/real-time 'watches its output stream live' experience itself is the kind of check this framework always routes to a human rather than certifying from grep."
---

# Phase 18: Driver Tab, Live Watch & Durable Injection — Verification Report

**Phase Goal:** A human can run and steer a GSD project from the TUI and see exactly what it is doing — the manual ship point, before any autonomy exists.
**Verified:** 2026-07-29
**Status:** human_needed
**Re-verification:** No — initial verification

## Summary

This phase is unusually thorough: 11 plans, a full adversarial code review (`18-REVIEW.md`) that
found 4 CRITICAL and 12 WARNING defects, and a resolution pass that fixed 15 of 16 (one, WR-05,
explicitly deferred with recorded rationale). I did not trust any of that narrative — every claim
below was checked directly against `src/` and `tests/` at commit `cae2f0a`, and the full test
suite plus both clippy invocations were re-run in this session rather than taken from SUMMARY.md.

**All four CRITICAL findings (CR-01..CR-04) and all four MUST-FIX carry-ins from Phase 17
(D-27/28/29/30) are genuinely fixed in the code, each with a named regression test that was run
and passed in this session.** The one deferred WARNING (WR-05) is a narrow terminal-height cosmetic
issue that does not block any of the five roadmap success criteria at ordinary pane sizes, and is
routed to human verification rather than treated as a gap, per its own recorded rationale.

**Requirements-doc note (not a code gap):** `.planning/REQUIREMENTS.md` still marks OBS-02, OBS-03,
STEER-01 and STEER-03 as `[ ]`/"Pending" — only TRANS-05, OBS-04, OBS-05, OBS-07 and STEER-02 were
flipped to `[x]`/"Complete" during the phase's plan commits. I verified all nine requirement IDs
against the actual code (see Requirements Coverage) and found code-level evidence satisfying all
nine. The unchecked four appear to be a doc-lag from the phase's normal "close-out" traceability
step never having run (compare Phase 17's `docs(17-07): phase gate — traceability, requirement
coverage, clippy delta` commit, which Phase 18 has no equivalent of yet) rather than a functional
gap. This should be corrected as part of phase close-out but is not a reason to fail verification.

## Goal Achievement

### Observable Truths (roadmap Success Criteria)

| # | Truth | Status | Evidence |
|---|-------|--------|----------|
| 1 | A user picks a GSD command, runs it from the TUI, and watches output/step/elapsed time stream live | ✓ VERIFIED (code); interactive flow → human item #2 | `src/ui/screens/driver_start.rs`/`driver_confirm.rs` (command+goal picker), `src/ui/screens/driver.rs` render_run_header/goal_lines/pipeline reuse, `src/app.rs::driver_elapsed_redraw_wanted` gated 250ms tick, `src/journal/mod.rs::exec_message_text`/`turn_result_text` (readable projection, not `Debug`), CR-04 fix (`853feae`) keeps the tab live while open. `tests/driver_inbox.rs` (7/7 pass) exercises the underlying pipeline end-to-end via fake-claude fixtures. |
| 2 | Project list marks driven/parked projects distinctly, sortable/filterable to those needing a human | ✓ VERIFIED | `src/ui/screens/normal.rs::alias_badge`/`row_badge` (5-rank priority, driven-and-live outranks all, wired at `normal.rs:725`), `src/ui/screens/mod.rs::needs_human`/`attention_rank`/`SortMode`/`recompute_filtered_aliases` with `FilterColumn::NeedsHuman` (`/h` suffix) in `src/app.rs::parse_filter`. WR-02(-review-numbering)/`82fb612` wired `last_outcomes` so a finished failed/denied/stalled/timed-out run also lights the badge. |
| 3 | Goal readable for any driven project; finished/failed run reviewable after the fact (commands, why it stopped) | ✓ VERIFIED | `driver.rs::goal_lines` renders `RunSummary.goal` verbatim (`(none given)` DarkGray+DIM when empty, never fabricated); `journal::list_runs`/`read_all` back the after-the-fact review; `TerminalState`/`terminal_state_cell` derive strictly from `RunVerdict`/outcome label, never from `ResultMessage.result` (D-13, see below). |
| 4 | Injected message shows queued → delivered → acted-on, never a bare "sent" | ✓ VERIFIED | `driver.rs::InjectionState` (Queued/Delivered/ActedOn/Missed) with glyph+label+colour per state; `"sent"` is in an explicit `FORBIDDEN` array asserted against in tests (`driver.rs:2896-2960`, `help.rs:524`); CR-01 (`51e198e`) and CR-03 (`e81405d`) fixes make the states honestly reachable more than once and never strand a failed send in `Queued`. |
| 5 | A message queued moments before a TUI restart is still delivered afterwards | ✓ VERIFIED | `src/journal/inbox.rs::append` — `OpenOptions::append` + `write_all` + `sync_data()`, on `spawn_blocking`; `tests/driver_inbox.rs::a_message_written_and_fsynced_before_any_reader_exists_is_delivered_and_journaled` passed in this session, proving durability without any TUI process alive at append time. |

**Score:** 5/5 roadmap success criteria have code-level evidence; criteria 1 and 4's *experiential*
half (actually watching it happen) is additionally routed to a human item (see above) because that
is the category of check this process always defers to a human rather than certifies from grep.

### Requirement-by-requirement (from PLAN frontmatter, cross-checked against REQUIREMENTS.md)

| Requirement | REQUIREMENTS.md state | Code-level status | Evidence |
|---|---|---|---|
| TRANS-05 | Complete | ✓ SATISFIED | Command+goal picker (`driver_start.rs`), spawn (`App::start_driver_run`), live output pane (`driver.rs`) |
| OBS-02 | **Pending (stale)** | ✓ SATISFIED | `alias_badge`/`row_badge` wired into `normal.rs:725`; driven-and-live and needs-human badges both reachable and tested |
| OBS-03 | **Pending (stale)** | ✓ SATISFIED | `goal_lines` in `driver.rs::render_run_header`, fed by `RunRecord.goal` via `App::start_driver_run`'s now-non-`None` `goal` parameter (`goal_or_none`, `app.rs:105,3826-3827`) |
| OBS-04 | Complete | ✓ SATISFIED | Live output pane + D-R-P-E-V reuse + step timeline (`driver.rs`), gated tick redraw |
| OBS-05 | Complete | ✓ SATISFIED | `journal::list_runs`/`read_all`, terminal-state table, adopted-run notice (`ADOPTED_RUN_NOTICE`) |
| OBS-07 | Complete | ✓ SATISFIED | `s` sort toggle (`SortMode::AttentionFirst`), `/h` filter (`FilterColumn::NeedsHuman`) |
| STEER-01 | **Pending (stale)** | ✓ SATISFIED | `i` key on Driver tab → `DriverInjectScreen` (WR-01-fixed to target the *selected* live run, `detail.rs:2135-2174`) → `Action::DriverInjectRequested` → `schedule_inbox_append` |
| STEER-02 | Complete | ✓ SATISFIED | Four-state display, "sent" forbidden and tested, CR-01/CR-03 fixes make the states reliable across multiple injections and send failures |
| STEER-03 | **Pending (stale)** | ✓ SATISFIED | `inbox::append`'s `sync_data()` + `tests/driver_inbox.rs::a_message_written_and_fsynced_before_any_reader_exists_is_delivered_and_journaled` (passed live in this session) |

All nine requirement IDs declared across the 11 plans are accounted for; none are orphaned. The
four marked "Pending" in REQUIREMENTS.md are a documentation-lag finding, not a code gap — see
Summary above.

### Required Artifacts (representative sample, not exhaustive — 11 plans' worth of artifacts were
spot-checked against their PLAN.md `must_haves.artifacts` lists)

| Artifact | Expected | Status | Details |
|---|---|---|---|
| `src/journal/inbox.rs` | append/tail module, fsync durability | ✓ VERIFIED | `OpenOptions::append`+`write_all`+`sync_data()` on `spawn_blocking`; 498 lines, substantive, has its own `#[cfg(test)]` module |
| `src/driver/run.rs` | drain-loop inbox arm, relocated `close_input` | ✓ VERIFIED | `stdin_open` stays `true` after spawn; closes only at `TurnCompleted` boundary when `delivered_since_boundary + deliver_pending_inbox() == 0` (lines 1205-1349) |
| `src/journal/mod.rs` | fallible `run_paths`, widened `Interjected`, readable projection | ✓ VERIFIED | `pub fn run_paths(...) -> Option<RunPaths>` (line 266), `is_plain_run_id` (line 220), `exec_message_text`/`turn_result_text` (readable prose, not `Debug`) |
| `src/ui/screens/driver.rs` | Driver tab render module, 4-state widget, terminal-state table | ✓ VERIFIED | 3421 lines; `InjectionState`, `TerminalState`, `output_for_run` (CR-02/WR-07 fixed), `goal_lines`, D-R-P-E-V reuse |
| `src/ui/screens/detail.rs` | 6 sites for the 11th tab | ✓ VERIFIED | `TAB_COUNT=11`, `tab_index`, `sub_view_from_index`, `switch_to_tab`, both render-dispatch matches (`:2416`, `:3725` incl. `render_main_only`), `footer_spans` |
| `src/ui/screens/driver_inject.rs` | injection input screen | ✓ VERIFIED | 360 lines, modelled on `enqueue.rs`, no movable cursor |
| `src/cli.rs` / `src/driver/mod.rs` | `#[cfg(debug_assertions)]`-gated override flags | ✓ VERIFIED | Both fields gated; `tests/spawn_seam_guard.rs::the_agent_program_override_fields_are_debug_only` passed |
| `tests/driver_inbox.rs` | end-to-end durability + acted-on + missed coverage | ✓ VERIFIED | 7/7 tests pass (ran live in this session) |
| `tests/journal_run_paths.rs` | both WR-02 regression directions | ✓ VERIFIED | 2/2 tests pass (ran live in this session) |
| `tests/driver_lock.rs` | WR-10 non-blocking regression | ✓ VERIFIED | 5/5 tests pass, incl. `acquiring_the_run_lock_does_not_block_the_async_runtime` |
| `src/ui/screens/help.rs` | badge legend (5) + injection legend (4) | ✓ VERIFIED | `BADGE_LEGEND`/`INJECTION_LEGEND` const arrays, both asserted by named tests |

### Key Link Verification

| From | To | Via | Status |
|---|---|---|---|
| TUI inbox append | driver `TailCursor` | `inbox::append` → `sync_data()` → `read_inbox`/`TailCursor` in `run.rs` | ✓ WIRED |
| `deliver_pending_inbox` | `Executor::send` | `executor.send(handle, UserMessage::text(...))` | ✓ WIRED |
| Driver | journal | `journal.record(&JournalEvent::Interjected{...})`, `InterjectionMissed`/acted-on variant | ✓ WIRED |
| `run_paths` fallibility | 5+ callers | `JournalRun::start`, `writer::read_active_run`, `reconcile::reconcile_one`, `App::schedule_journal_tail`, `App::schedule_inbox_append`, `AppContext::schedule_run_list_scan`, CLI seam | ✓ WIRED — grep for a raw `runs_root(..).join(run_id)` bypass finds none |
| `Action::DriverJournalAppended` | `AppContext::driver_output` ring buffer | append + `retarget(run_id)` + `needs_redraw = true` | ✓ WIRED (CR-02 fix) |
| `needs_human` predicate | badge / filter / sort | `alias_badge` rank 2, `FilterColumn::NeedsHuman`, `attention_rank`, single implementation | ✓ WIRED |
| `i` key | `DriverInjectScreen` | selected-run-id must equal live-run-id (WR-01 fix), else a named refusal, never both message+dispatch | ✓ WIRED |
| `DriverOutput` | `prune_driver_maps` | `.retain(|alias,_| registered.contains_key(alias))` | ✓ WIRED |
| `last_outcomes` | `prune_driver_maps` | same retain pattern | ✓ WIRED |

### Behavioral Spot-Checks / Regression Tests Run Live This Session

| Behavior | Command | Result | Status |
|---|---|---|---|
| Full workspace test suite | `rtk proxy cargo test` | 772 passed, 0 failed (sum of all binaries verified) | ✓ PASS |
| `cargo clippy -- -D warnings` | `rtk proxy cargo clippy -- -D warnings` | clean | ✓ PASS |
| `cargo clippy --all-targets` lint count | `rtk proxy cargo clippy --all-targets` | exactly 5 pre-existing lints (unchanged) | ✓ PASS |
| CR-01 regression (second injection after acted-on) | `cargo test --test driver_inbox` | `a_message_appended_before_the_final_drain_is_still_delivered` and 6 siblings, 7/7 pass | ✓ PASS |
| D-27/WR-02 regression (both directions) | `cargo test --test journal_run_paths` | 2/2 pass | ✓ PASS |
| D-28/WR-10 regression (non-blocking lock) | `cargo test --test driver_lock` | 5/5 pass | ✓ PASS |
| STEER-03 durability | `driver_inbox.rs::a_message_written_and_fsynced_before_any_reader_exists_is_delivered_and_journaled` | pass | ✓ PASS |
| `inbox.jsonl` gitignore non-vacuous | `cargo test --test journal_gitignore` | 3/3 pass, inbox file is real and gitignored | ✓ PASS |

### Requirements Coverage

See table above — all 9 requirement IDs SATISFIED at code level; 4 of 9 have stale
"Pending"/`[ ]` markers in REQUIREMENTS.md that should be corrected during phase close-out.

### Anti-Patterns Found

None found that constitute a blocker. Searched for `TBD`/`FIXME`/`XXX`/`TODO`/`HACK`/`placeholder`
across the files this phase touched — the only matches are inside doc comments explaining *why a
past defect looked done but wasn't* (i.e., historical narrative, not live debt markers), and none
reference unresolved work without a named owner. `WR-05`'s deferral is explicitly recorded with a
carry-forward obligation in `18-REVIEW.md`, which is the correct way to record an accepted, tracked
deviation rather than a silent gap.

### Deferred Items

| # | Item | Addressed In | Evidence |
|---|------|-------------|----------|
| 1 | WR-05 — medium terminal-height tier (8-13 rows) renders a blank pipeline row / bare steps rule | Recorded as a carry-forward obligation for "the next UI pass on the Driver tab" (no specific later phase number named) | `18-REVIEW.md` "Deferred: WR-05" section; does not block any of the 5 roadmap success criteria at ordinary (>=14 row) terminal heights |

### Human Verification Required

See frontmatter `human_verification` — two items: (1) the WR-05 medium-height-tier visual check,
explicitly deferred by the review and requiring a human at real terminal heights 8/13/14; (2) the
full interactive "start → watch live → inject → observe queued/delivered/acted-on → stop → review"
flow, which is the category of check (visual appearance, real-time behavior) this process always
routes to a human even when — as here — the underlying state machine has strong, passing,
fixture-driven integration-test coverage for every transition.

### Gaps Summary

No code-level gaps were found. All four CRITICAL code-review findings and all four Phase-17
MUST-FIX carry-ins are genuinely fixed with passing named regression tests, verified by running
them directly in this session rather than trusting SUMMARY.md or 18-REVIEW.md's own narrative. The
one open item (WR-05) is a recorded, narrow, non-blocking deferral. The only paperwork gap is
REQUIREMENTS.md's stale Pending markers for OBS-02, OBS-03, STEER-01 and STEER-03, which contradict
what the code actually does and should be corrected in the phase's close-out commit.

---

_Verified: 2026-07-29_
_Verifier: Claude (gsd-verifier)_
