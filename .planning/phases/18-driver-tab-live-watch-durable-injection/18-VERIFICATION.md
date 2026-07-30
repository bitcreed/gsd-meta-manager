---
phase: 18-driver-tab-live-watch-durable-injection
verified: 2026-07-30T05:37:35Z
status: human_needed
score: 4/5 ROADMAP success criteria verified (1 present, behavior-unverified); 10/10 named invariants verified; 9/9 requirements satisfied
behavior_unverified: 1
overrides_applied: 0
re_verification: no — initial verification, run after 18-REVIEW.md's 15-of-16 resolution pass
notes: |
  Verified at HEAD 76720ab (phase base 2e33871). Every cargo/grep check that needed raw
  output was run through `rtk proxy`; the `rtk` wrapper strips `warning:` and
  `test result:` lines from plain `cargo` output, which has already produced vacuous
  passes in Phases 15 and 17.

  A concurrent process committed a different 18-VERIFICATION.md as 76720ab while this
  pass was running. That file is preserved in git (`git show 76720ab:.planning/phases/
  18-driver-tab-live-watch-durable-injection/18-VERIFICATION.md`) and is superseded by
  this one. Four material corrections: (a) it reported `behavior_unverified: 0` and a
  clean 14/14 while simultaneously routing SC1's live-watch to a human — an internally
  inconsistent score; (b) it did not check WR-16/D-30 against an actual release build,
  only against the source-scanning guard test; (c) it asserted no raw `join(run_id)`
  exists without accounting for the two that do (both safe, analysed below); (d) it did
  not check the "no parallel total_lines - visible_height" clause, the dependency fence,
  or the CR-03 residual's bearing on STEER-02. 76720ab also corrected REQUIREMENTS.md's
  stale Pending markers, which this pass independently found and which are now closed.
behavior_unverified_items:
  - truth: "SC1 — a user picks a GSD command, runs it from the TUI, and watches its output, current step, and elapsed time stream live"
    test: "Register and opt in a scratch project. From the Driver tab press `s`, pick a GSD command, confirm, and leave the tab open for the whole run."
    expected: "Output lines appear in the pane as the run progresses (not only on tab re-entry); the D-R-P-E-V pipeline row advances; the elapsed counter increments while the tab is open; when the run ends the pane keeps the run's own output rather than swapping to a stale snapshot."
    why_human: "The chain is watcher -> ChangeKind::DriverJournal -> schedule_journal_tail -> DriverOutput::retarget/push_record -> render. Each link is unit-tested in isolation and the CR-04 rescan is tested, but no test closes the loop through a real spawned driver, a real notify watcher and a rendered terminal. Presence and wiring cannot see whether the frame actually updates."
human_verification:
  - test: "See behavior_unverified_items[0] — the full start -> watch -> stop flow at a real terminal."
    expected: "Output, step and elapsed all update live while the tab is open."
    why_human: "Real-time cross-process behaviour; no test drives a rendered terminal."
  - test: "18-09 D11 — open the Driver tab against a live run at an ordinary terminal size and read the header at a glance."
    expected: "Spacing, colour balance and the header read as a run without study; the two panes are legible side by side."
    why_human: "Visual composition. Declared by 18-09-SUMMARY.md as human_judgment: true with an empty verification list."
  - test: "18-10 D6 — inject a message with `i` during a live run and watch the state cell for the full dequeue gap (~55 s on CLI 2.1.220)."
    expected: "The filling shape progresses queued (circle) -> delivered (half) -> acted-on (filled) and reads correctly across the minute-long gap; no spinner, no animation, nothing implying the echo is imminent."
    why_human: "Perceptual: whether a static four-state glyph reads correctly across a minute of no visible change cannot be scraped from a buffer. Declared by 18-10-SUMMARY.md as human_judgment: true."
  - test: "18-11 D6 — open the help popup at an 80x24 terminal and scroll it."
    expected: "The two legends, the scroll indicator and the injection gloss land as a scannable page rather than a wall of rows."
    why_human: "Legibility judgement at a specific terminal size. Declared by 18-11-SUMMARY.md as human_judgment: true and named as the plan's single verify-work item."
  - test: "WR-05 (recorded deferral) — render the Driver tab's run-detail pane at terminal heights 8, 13 and 14, with a goal long enough to wrap to two rows."
    expected: "The pipeline row and the step-timeline section each render content rather than a blank line and a bare `-- steps --` rule; the elapsed-time counter survives the two-row goal."
    why_human: "18-REVIEW.md deferred this as a layout judgement across three height tiers that the UI-SPEC's own priority ordering owns. No render test exercises render_run_detail's height tiers at all. Not a correctness or safety risk — no fact is misreported."
deferred:
  - truth: "WR-05 — at the 8 <= area.height < 14 tier the pipeline row paints an empty line and the steps section a bare rule"
    addressed_in: "Next UI pass on the Driver tab (no phase number assigned)"
    evidence: "18-REVIEW.md `### Deferred: WR-05` — carry-forward obligation recorded: add height-tier render tests at 8/13/14 plus the producer-ordering fix, with the UI-SPEC height tables in hand."
---

# Phase 18: Driver Tab, Live Watch & Durable Injection — Verification Report

**Phase Goal:** A human can run and steer a GSD project from the TUI and see exactly what it is
doing — the manual ship point, before any autonomy exists
**Verified:** 2026-07-30T05:37:35Z, at HEAD `76720ab` (phase base `2e33871`, 72 commits)
**Status:** human_needed
**Re-verification:** No — initial verification, run after `18-REVIEW.md`'s resolution pass

## Method

`18-REVIEW.md` found 4 BLOCKERs (CR-01..CR-04) and 12 WARNINGs, fixed 15 and deferred WR-05.
Nothing in that document, and nothing in the eleven SUMMARY files, was accepted as evidence. Every
invariant below was read from the current source and, where it names a behavioural property,
confirmed by a test re-run in this session.

**Tooling note, per the phase brief.** The `rtk` wrapper filters `cargo` output — raw `warning:`
and `test result:` lines are absent from plain `cargo`, so a check that greps for them passes
vacuously. This already produced false passes in Phases 15 and 17. **Every check in the Gate and
Behavioural Spot-Check tables below was run through `rtk proxy`**, as was every `grep` and
`git` invocation whose completeness mattered.

## Project Gate (all through `rtk proxy`)

| Check | Command | Result | Status |
|---|---|---|---|
| Build | `rtk proxy cargo build` | clean, exit 0 | ✓ PASS |
| Tests | `rtk proxy cargo test` | **772 passed, 0 failed** across 19 result lines (669 lib + 103 integration). Matches the expected figure exactly; phase-start baseline was 542, so the phase added 230 tests. | ✓ PASS |
| Lint gate | `rtk proxy cargo clippy -- -D warnings` | exit 0, no diagnostics | ✓ PASS |
| All-targets lint delta | `rtk proxy cargo clippy --all-targets` | **exactly 5** warnings: `src/browser.rs:131,132,133` (`bool_assert_comparison`), `src/project_creator.rs:146` (`cmp_owned`), `src/state_reader/mod.rs:258` (`items_after_test_module`). The count did not grow. | ✓ PASS |
| Dependency fence | `rtk proxy git diff 2e33871..HEAD -- Cargo.toml Cargo.lock` | **empty diff** — zero new dependencies, zero version changes | ✓ PASS |
| Debt markers | `TBD\|FIXME\|XXX` and `TODO\|HACK\|PLACEHOLDER` and `unimplemented!\|todo!` across all 33 files the phase touched | **zero matches** | ✓ PASS |

## Goal Achievement — ROADMAP Success Criteria

| # | Truth | Status | Evidence |
|---|---|---|---|
| 1 | A user picks a GSD command for an opted-in project, runs it from the TUI, and watches its output, current step, and elapsed time stream live | ⚠️ PRESENT_BEHAVIOR_UNVERIFIED | Every link exists and is wired: `driver_start.rs` (two-step command+goal picker) → `driver_confirm.rs` → `App::start_driver_run` → `drive_argv` (detached spawn, config carried) → driver writes `journal.jsonl` → notify watcher classifies `ChangeKind::DriverJournal` → `App::schedule_journal_tail` → `DriverOutput::retarget` + `push_record` (`app.rs:1317-1334`) → `output_body_lines`. `elapsed_label` (`driver.rs:538`), `render_pipeline_row` and `steps_lines` supply step + elapsed. The CR-04 rescan (`app.rs:728-742`, three call sites) keeps the tab from freezing. **But the truth asserts a live cross-process state transition that no test closes the loop on** — the links are individually tested, the loop is not. Routed to human verification. |
| 2 | The project list marks driven projects and parked projects distinctly, and can be sorted or filtered down to the ones waiting on a human | ✓ VERIFIED | `alias_badge` (`normal.rs:169`) — 5-rank priority, at most one badge, `BADGE_DRIVEN` `◆` Magenta+BOLD rank 1, `BADGE_NEEDS_HUMAN` `⚑` Red+BOLD rank 2, every glyph a `&'static str` so no file content can reach a row. `needs_human` (`screens/mod.rs:863`) over four evidence sources; `attention_rank`; `SortMode::AttentionFirst` on `s`; `FilterColumn::NeedsHuman` via the `/h` suffix in `parse_filter` (`app.rs:79`). WR-02's fix wired `last_outcomes` (populated in the same `spawn_blocking` task as the probe, `app.rs:1008-1014`, pruned beside `observed_runs`) so the finished-run arm is reachable in production. Tests: `a_bare_needs_human_filter_yields_every_needs_human_project`, `the_needs_human_badge_outranks_an_active_session`, `a_needs_human_filter_matching_nothing_leaves_an_empty_list_and_no_panic`, plus the badge-priority exhaustive bitmask arm. |
| 3 | The originating goal prompt is readable for any driven project, and a finished or failed run can be reviewed afterwards — which commands ran and why it stopped | ✓ VERIFIED | `goal_lines` (`driver.rs:742`) renders `RunSummary.goal` **verbatim**, sanitised, wrapped to ≤3 rows; an absent goal renders pinned copy, never a fabrication. Tests `an_absent_goal_renders_the_pinned_copy_and_nothing_else`, `a_goal_bearing_an_escape_sequence_is_sanitised_before_it_becomes_a_line`, `a_long_goal_wraps_to_at_most_three_rows_then_ellipses`. After-the-fact review: `journal::list_runs` → run list pane, `reader::read_all` → `DriverRunJournal` projection (`screens/mod.rs:1032-1071`), `render_run_header` (command, times, cost, run dir), `steps_lines`, `terminal_state_cell`. |
| 4 | A message typed into a running driver shows queued → delivered → acted-on, never a bare "sent" | ✓ VERIFIED | Four states + a fourth honest terminal state; state machine proven behaviourally against a real fake-claude process, not just rendered. See "Invariant 6" below for the full evidence. The *perceptual* half (does the static filling shape read correctly across a minute?) is 18-10's own declared D6 human item and is listed separately — it is not a state transition, so it does not make the truth behavior-unverified. |
| 5 | A message queued moments before the TUI restarts is still delivered to the run afterwards | ✓ VERIFIED | `inbox::append` = `OpenOptions::append` + `write_all` + **`sync_data()`**, on `spawn_blocking`. `tests/driver_inbox.rs::a_message_written_and_fsynced_before_any_reader_exists_is_delivered_and_journaled` — re-run this session — writes and fsyncs the message, **asserts the journal does not yet exist** (i.e. no reader has ever run), then starts the driver in a separate process and asserts the message reached the agent's stdin in the pinned wire shape, produced exactly one `interjected` record carrying the client-minted id, and that the run still ended naturally with `exit 0` and no `interjection_missed`. That is a stronger proof than a TUI restart: the writing process did not merely die, it never existed. |

**Score:** 4/5 verified, 1 present-but-behavior-unverified.

## The Ten Named Invariants

Each was required to hold by the phase brief. Each was checked against source and, where
behavioural, against a test re-run in this session.

### 1. WR-02 / D-27 — path traversal through a `run_id` — ✓ VERIFIED

`journal::run_paths` (`mod.rs:266`) is fallible and refuses before any `join`. Its predicate
`is_plain_run_id` (`mod.rs:220`) requires a non-empty string that yields exactly one
`Component::Normal` **and** compares the component back against the original string — which is
what refuses `./escape` and `escape/`, both of which `components()` silently normalises to a single
`Normal`. `..` is rejected as a *token*; the filesystem is never consulted, so no symlink the
driven agent plants can decide the answer.

**No bypassing call site exists, on either side.** Exhaustive `rtk proxy grep` for
`join(run_id|&run_id|record.run_id|summary.run_id|rid|id)` across `src/` returns exactly three
hits, and all three are accounted for:

| Site | Why it is safe |
|---|---|
| `journal/mod.rs:271` | Inside `run_paths` itself, after the `is_plain_run_id` guard at 267 |
| `journal/writer.rs:502` (`read_active_run`) | The component check is at **line 491, ahead of the `is_dir()` check at 502**. This ordering is load-bearing: `active` lives inside the driven project, so the agent writes it; checking `is_dir()` first would ask the filesystem about the hostile path before refusing it |
| `journal/writer.rs:672` (`prune_runs`) | The name comes from a `read_dir` entry filtered by `parses_as_run_id` — a directory that already exists inside the runs root, never an external string |

Every *external* entry point routes through `run_paths`: `writer::create_run_dir`,
`reconcile::reconcile_one`, `journal::read_run_summary`, `App::schedule_journal_tail`,
`App::schedule_inbox_append`, `AppContext::schedule_run_list_scan`, `ui::screens::mod.rs:1003`,
`ui::screens::driver.rs:1080`, plus `driver::drive`'s own CLI-seam refusal (`driver/mod.rs:288`).
No infallible variant is kept alongside, so the compiler enumerates callers.

**Regression test:** `tests/journal_run_paths.rs`, 2 tests, re-run individually this session — both
pass in 0.00s.

- Write side: six hostile ids (`../../../../escaped`, `../escaped`, `..`, `nested/escaped`,
  `/tmp/absolute-escaped`, `./escaped`) each driven through the real `drive()` entry point. The
  assertion is a **whole-filesystem-footprint comparison** (`BTreeSet` of every path under the
  sandbox, before vs after) rather than a handful of `exists()` checks — the correct shape, because
  nobody can enumerate in advance where a bad `join` lands. It additionally asserts the typed
  `DriveError::RunIdInvalid` naming the offending id, and that the runs root was never even created.
- Read side: plants a well-formed `run.json` one level above the project, points `active` at
  `../../../../planted`, and **asserts the bait satisfies the old `is_dir()` guard and parses as a
  `RunRecord`** before asserting both `read_active_run` and `reconcile::reconcile_one` refuse it.
  Ends with a positive control on a plain id so a broken reconciliation cannot masquerade as a
  security fix.

The sandbox deliberately roots the project as a *subdirectory* so a traversal lands somewhere the
test can still observe — a fixture rooted at the project would watch the wrong side of the boundary
and pass regardless.

### 2. WR-10 / D-28 — no blocking call inside an `async fn` — ✓ VERIFIED (scope-bounded)

All seven sites D-28 names are behind `tokio::task::spawn_blocking`:

| Site | Location |
|---|---|
| `dry_run::build_report` (two sync `git` shell-outs) | `driver/mod.rs:239` |
| `lock::acquire` (`flock` + file writes) | `driver/run.rs:1079` |
| `JournalRun::start` (prunes directories, writes files) | `driver/run.rs:1096` |
| TUI inbox append | `app.rs:828` |
| Driver inbox tail | `driver/run.rs:637` |
| Journal tail | `app.rs:594` |
| Run-list / inbox / journal scan | `ui/screens/mod.rs:994`, `:1133` |

Both `RunLock` and `JournalRun` are **moved back out** of the blocking task (`let lock = ...await`,
`let journal = ...await`) rather than dropped inside it — `RunLock` holds a `File` whose descriptor
*is* the lock and has no `Drop` impl, so dropping it in the task would silently release it.

`tests/driver_lock.rs` (5 tests, pass) carries the observed reproduction that motivated this: a
blocking `flock` inside an `async fn` defeating `tokio::time::timeout` outright on a current-thread
runtime.

**Bounded observation, not a gap.** Per-record `run.journal.record_exec(&event)` inside the drain
loop (`run.rs:1296`) is a small synchronous append that is outside D-28's named scope
(18-CONTEXT.md:401-414 enumerates the sites). Unlike WR-16, this invariant has **no mechanical
guard test** — it is enforced by review and by doc comments at each site. A future site added
without one would not fail the suite. Worth a `spawn_seam_guard`-style audit in a later pass.

### 3. WR-15 / D-29 — `DriverStopped` must not drop a run nothing stopped — ✓ VERIFIED

`StopDisposition` (`action.rs:25`) is a portable two-variant enum on the `Action` — deliberately
portable because `StopOutcome` is `#[cfg(unix)]` and `Action` is cross-platform.
`From<&StopOutcome>` maps `ExitedOnTerminate | ExitedAfterKill → RunGone` and
**`AlreadyGone | SignalFailed{..} → MayStillBeLive`**. `app.rs:1507` gates both map mutations:

```rust
if disposition == crate::action::StopDisposition::RunGone {
    self.ctx.observed_runs.remove(&alias);
    self.ctx.session_spawned_runs.remove(&run_id);
}
```

The status message is set on **both** arms, so the user is still told what happened. Tests:
`a_run_that_exited_on_terminate_is_dropped_from_both_maps`,
`a_run_that_exited_after_the_uncatchable_signal_is_dropped_from_both_maps`,
`a_stop_that_signalled_nothing_keeps_both_map_entries`,
`a_stop_whose_signal_was_never_delivered_keeps_both_map_entries` — the last two carry the WR-15
reproduction, including the permanent-`session_spawned_runs`-loss consequence (a later stop taking
the `Adopted` reaping arm for a run this session did spawn).

### 4. WR-16 / D-30 — override flags absent from a release build — ✓ VERIFIED AGAINST A REAL RELEASE BUILD

Not inferred from the `#[cfg]` attribute. `rtk proxy cargo build --release`, then:

```
$ ./target/release/gsd-meta-manager drive somealias --command /gsd-progress --claude-program /bin/true
error: unexpected argument '--claude-program' found          # exit 2
$ ./target/release/gsd-meta-manager drive somealias --command /gsd-progress --claude-args foo
error: unexpected argument '--claude-args' found             # exit 2
$ ./target/debug/gsd-meta-manager   drive somealias --command /gsd-progress --claude-program /bin/true
Error: no project is registered under the alias `somealias`  # flag ACCEPTED, parse proceeded
$ grep -c "claude-program" ./target/release/gsd-meta-manager  → 0
$ grep -c "claude-program" ./target/debug/gsd-meta-manager    → 2
```

The literal string is **absent from the release binary's bytes**, present twice in the debug
binary, and the debug build parses the flag through to alias resolution. `drive --help` is
otherwise identical in both builds — all five legitimate options (`--command`, `--run-id`,
`--dry-run`, `--goal`, `--config`) survive, so the gate is surgical rather than a blanket
truncation of the subcommand.

Source: `#[cfg(debug_assertions)]` on the clap variant (`cli.rs:136,149`), on `DriveArgs`
(`driver/mod.rs:125,131`) and on `main.rs`'s destructuring pattern. Mechanism test
`tests/spawn_seam_guard.rs::the_agent_program_override_fields_are_debug_only` walks `src/`, and is
**non-vacuous by construction**: it asserts it found at least two declarations in `src/cli.rs`
(so a rename fails loudly rather than silently auditing nothing), and the sibling
`an_override_field_declared_without_the_debug_gate_is_reported` proves the matcher can fail,
accepts `all(debug_assertions, …)` as a narrowing, and rejects `any(…)` / `not(…)` — the two shapes
that hand the field back to release.

### 5. Injection is stdin `stream-json`, never tmux — ✓ VERIFIED

`--input-format stream-json` in the executor argv (`claude.rs:233`, asserted by
`claude.rs:1728`). `Executor::send` writes a `UserMessage` to the child's stdin. `tmux` appears in
`src/` only in `terminal_switch.rs` (the pre-existing optional human-attach switcher) and a
`session_detector.rs` doc comment about pane-matching. **Nothing in the injection path touches
tmux.** `driver/mod.rs:13` states the anti-requirement explicitly.

### 6. `isReplay: true` is a DEQUEUE ack, and STEER-02 renders it honestly — ✓ VERIFIED

**Four states, one authoritative observer each** (`driver.rs:1366`): `Queued` (in `inbox.jsonl`,
no journal record), `Delivered` (`interjected` with `delivered: true` — exactly "`Executor::send`
returned `Ok`"), `ActedOn` (`interjection_acted_on`, from the agent's own replay echo), and
`Missed(MissedReason)` (undeliverable, terminal, never retried). `rank()` makes a later state win;
`ActedOn` outranks `Missed` because the agent's own dequeue is the stronger evidence.

**The semantics are correct.** `run.rs:181` records the measurement against CLI 2.1.220: a message
written at t=12 s was echoed at t=68 s, 45 ms after the *previous* turn's `result`. The echo means
"the agent has started processing this", and `acted-on` is the only word for it.

**Vocabulary.** Labels are exactly `queued` / `delivered` / `acted-on` / `missed`; glyphs `○ ◐ ● ✗`
carry the meaning so nothing depends on colour. `no_rendered_injection_string_uses_a_word_that_
implies_receipt` (`driver.rs:2894`) checks `["sent","received","read","acknowledged"]` against
every rendered row for all six record shapes, plus every state's label and every `MissedReason`
gloss. It **tokenises rather than substring-matches** (so `already` does not count as `read` — a
substring test would be noisy enough to be weakened, and a weakened test is how the real word gets
back in), holds the forbidden list in the *test* module so the same edit cannot soften both the
copy and the assertion, and ends with a can-it-fail assertion.

**No spinner, no animation, no implied imminence.** Zero spinner/animation/frame-index code
anywhere in `src/ui/` or `src/driver/` — every match for those words is a doc comment forbidding
them. `injection_elapsed` shows time passing without predicting arrival.

**Behavioural proof, not just render tests:** `a_delivered_message_is_journaled_acted_on_when_the_
replay_echo_arrives`, `two_identical_messages_are_acked_in_delivery_order` (exact `String` equality,
front-to-back removal, so the second echo cannot re-ack the first), `a_message_appended_after_
stdin_closed_is_journaled_missed_and_never_retried`, `a_message_whose_stdin_write_failed_is_
journaled_missed_rather_than_left_queued` (CR-03), `a_torn_final_line_is_neither_delivered_nor_lost`.

**Judgment on the CR-03 residual — NOT a blocker for STEER-02.** `JournalEvent::Interjected` is
`is_content()` (`journal/mod.rs:811-816`), so past `MAX_RUN_JOURNAL_BYTES` (64 MB) the delivery
record is suppressed while the two *transitions* keep being written. A message that is delivered
past that cap and never echoed therefore derives `queued`. Three reasons this does not block:

1. **The split is deliberate and is the safer half.** `interjection_acted_on` and
   `interjection_missed` are explicitly kept out of `is_content()` precisely so a delivered message
   cannot strand — so past the cap the *common* case (the echo arrives) still reaches `acted-on`
   correctly. Only delivered-and-never-echoed is affected, a window that also requires the run to
   end inside the ~55 s dequeue gap.
2. **The failure direction is under-claiming, never over-claiming.** STEER-02's stated purpose is
   that a message is not "fire-and-forget"; the anti-requirement it encodes is a bare "sent" that
   promises more than the protocol supports. Rendering `queued` promises *less* than what happened.
   A four-state display that occasionally under-reports is a different, far less dangerous defect
   than one that over-reports.
3. **The trigger requires 64 MB of journal content in a single run**, which is far outside the
   manual ship point this phase delivers.

It is nonetheless a state the UI can report incorrectly, and the gloss ("nothing has read it") is
then a false statement. Recorded here as a carry-forward for the phase that raises or tiers the
content cap, not as a Phase 18 gap.

### 7. `type:"result"` is a TURN boundary, not a run terminator — ✓ VERIFIED

`run.rs:1294`: `let turn_boundary = matches!(event, ExecutionEvent::TurnCompleted(_));`. At each
boundary the loop drains the inbox once more and closes stdin **only if nothing was delivered**;
otherwise the agent runs the message as a new turn and the loop repeats. EOF is "no more input",
not "stop" — the CLI drains its queue and exits 0.

**CR-01 is fixed and the fix is visible.** The poll arm (`run.rs:1377`) now writes
`delivered_since_boundary += deliver_pending_inbox(...)`, and the boundary arm (`run.rs:1303`) adds
that carried count to its own final drain before deciding. Since the poll fires every 750 ms and a
turn lasts minutes, essentially every injected message arrives through the poll arm — discarding
its count meant a run could be steered exactly once.

Regression test `a_message_appended_before_the_final_drain_is_still_delivered`, **re-run
individually this session — PASS in 6.10 s**. It is non-vacuous by construction: it waits for the
first message's `interjection_acted_on` record (which the paced fixture emits as turn *two* starts,
so turn one's boundary is an *observed fact* rather than an elapsed duration), asserts the EOF
marker is still absent at that moment, appends a second message, and then asserts **both** ids
appear in `interjected` in order with no `interjection_missed`. A driver that can be steered once
produces only the first.

### 8. Run status derives from exit codes, disk state and git — never the agent's prose — ✓ VERIFIED

`derive_run_outcome_from_envelopes` (`executor/outcome.rs:173`) is the sole entry point and reads
`permission_denials[]` — a sibling taking the `TurnOutcome` projection is what once let a blocked
run report success, so no second entry point exists. The five-arm precedence in `derive` is:
missing terminal envelope → denials from *any* turn → the last envelope's
`subtype`/`is_error`/`terminal_reason` as string slices with a verbatim fallback → exit status as a
liveness/crash signal (disagreement with a success envelope is *surfaced*, never silently resolved)
→ the disk and git delta (`RunSnapshot`), where a success envelope with no signal becomes the
distinct `SucceededNoChanges` outcome rather than a success. The module states it directly: *"The
envelopes' prose summaries are read by no branch of the derivation."*

On the render side, `TerminalState` (`driver.rs:376`) has 13 variants and both `from_outcome` and
`terminal_state_cell` match **exhaustively with no wildcard**, so a new outcome is a compile error
rather than a silent fallthrough to a word that is wrong. `LivenessUnknown` is kept distinct from
dead (re-flattening the CR-05 tri-state at the render layer would print "your run died" about every
healthy run off Linux), and `Unrecorded` is neither a crash nor a success. Test
`the_render_vocabulary_is_the_one_the_driver_actually_writes` proves
`TerminalState::from_label(outcome_label(o)) == TerminalState::from_outcome(o)` over all nine
`RunOutcome` variants, so the render vocabulary cannot drift from the driver's.

### 9. UIFIX-04 clamp ordering — ✓ VERIFIED

One formula: `clamp_scroll(offset, total, visible) = offset.min(total.saturating_sub(visible))`
(`detail.rs:47`). `tail_offset(vp) = clamp_scroll(u16::MAX, ..)` and
`driver_offset_now(stored, following, vp)` both resolve through it — the latter exists because
while the follow bit is set the stored offset is stale by design, so a key press must start from
the tail. All metrics come from `ViewportMetrics` recorded by the render pass.

| Direction | Order | Site |
|---|---|---|
| PageDown, Driver pane | **add then clamp**, then re-arm follow if it reached the tail | `detail.rs:1240-1245` |
| PageUp, Driver pane | **clamp first (via `driver_offset_now`) then subtract**, and clear follow | `detail.rs:1360-1363` |
| PageDown / PageUp, generic panes | add-then-clamp / clamp-then-subtract | `detail.rs:1251`, `1370-1372` |
| Up / `k`, generic panes | clamp-then-subtract | `detail.rs:1093-1095` |
| PageDown, help popup | add then clamp | `help.rs:291-292` |
| PageUp and `k`/Up, help popup | clamp then subtract | `help.rs:304-305`, `311-312` |

In the Driver sub-view `k`/Up move the *run selection* rather than the output pane (`detail.rs:1086`);
the output pane scrolls with PageUp/PageDown, `f` (follow toggle) and `G` (jump to tail), and every
one of the six `driver_scroll_offset` mutation sites goes through `clamp_scroll` / `tail_offset` /
`driver_offset_now`.

Named tests, all passing: `driver_page_down_from_the_tail_clamps_and_does_not_overshoot`,
`driver_page_up_clamps_first_and_lands_below_max_scroll`, `driver_page_up_from_zero_stays_at_zero`,
`page_down_reaching_the_tail_re_arms_driver_follow`, `an_upward_scroll_clears_the_driver_follow_bit`,
`g_jumps_to_the_driver_tail_and_re_enables_follow`, `f_toggles_the_driver_follow_bit`,
`changing_the_selected_run_resets_the_offset_and_the_follow_bit`, plus the five
`test_clamp_scroll_*` unit tests and three help-popup scroll tests. 50 `detail::tests` pass.

**Correction to `18-REVIEW.md`.** The review states *"No parallel `total_lines - visible_height`
exists anywhere."* That is imprecise: two inline copies exist at `detail.rs:3444` (archive file
view) and `detail.rs:3588` (browser file view). Both were confirmed **pre-existing at `2e33871`**
(then at lines 2930 and 3074) — not introduced by this phase. Both are render-time display clamps
applied to an already-clamped stored offset, not key handlers, and both compute the identical
value, so no divergence is reachable. The invariant as it bears on the Driver pane holds without
qualification.

### 10. Adopted runs promise nothing — ✓ VERIFIED

`ADOPTED_RUN_NOTICE` (`driver.rs:271`): *"This run started before the current TUI session. Its live
output is gone — showing the journal on disk."* Pushed as the pane's **first row**, unconditionally
whenever `adopted`, with `adopted = !ctx.session_spawned_runs.contains(&summary.run_id)`
(`driver.rs:1852`) — not gated on liveness, so an adopted run that is still going carries it too.
`INDICATOR_JOURNAL_ONLY = "[journal only]"` for the ended case; the constant's doc states *"It does
not say 'live'"*.

Test `the_adopted_notice_renders_only_for_a_run_this_session_did_not_spawn` asserts the notice is
row 0, that it contains the word "gone", that a session-spawned run does **not** carry it, and that
the indicator agrees. `an_adopted_live_run_with_an_empty_ring_renders_the_journal_on_disk` (WR-07)
proves the live ring is a preference rather than a short-circuit, so an adopted run falls through
to the journal on disk instead of rendering "No journal entries yet." over a journal that exists.

Help-screen copy (`"Toggle following the live tail"`, `"Inject a message into the live run"`,
`"Stop the live run"`) describes keys that act on a run that *is* live — verified live by the
`/proc` probe — and WR-01's fix means `i` only opens when the selected run is the live one. No copy
promises live *output* for an adopted run.

**Observation (not a gap).** For an adopted run that is still live, `follow_indicator`
(`driver.rs:1307`) returns `[following]` / `[scrolled +N]` rather than `[journal only]`. The doc at
`driver.rs:1297` justifies it — the journal *is* being tailed, so the follow bit is meaningful and
the provenance is carried by the pinned first row instead. The word "following" alone is mildly
ambiguous; worth an eye during the D11 human check, but the unconditional first-row notice makes
the surface honest.

## Requirements Coverage

All 9 requirement IDs the ROADMAP assigns to Phase 18 are declared across the 11 plans. **No
orphaned requirements.**

| Requirement | Declared in | Status | Evidence |
|---|---|---|---|
| TRANS-05 | 18-05, 18-07, 18-08, 18-09, 18-11 | ✓ SATISFIED | Start wizard → confirmation → `start_driver_run` → `drive_argv` detached spawn; live pane |
| OBS-02 | 18-06 | ✓ SATISFIED | `alias_badge` ranks 1 and 2, wired at `normal.rs:725`, at most one badge per row |
| OBS-03 | 18-05, 18-07, 18-09 | ✓ SATISFIED | `goal_lines` verbatim; `goal_or_none` stops an empty buffer arriving as `Some("")` |
| OBS-04 | 18-03, 18-04, 18-05, 18-09, 18-10, 18-11 | ✓ SATISFIED (SC1's live half → human) | Output ring, pipeline row, step timeline, `elapsed_label`, gated tick |
| OBS-05 | 18-03, 18-10 | ✓ SATISFIED | `list_runs`, `read_all`, run header, terminal-state table, adopted notice |
| OBS-07 | 18-04, 18-06, 18-11 | ✓ SATISFIED | `s` sort toggle + indicator, `/h` filter through `FilterColumn::NeedsHuman` |
| STEER-01 | 18-01, 18-05, 18-07 | ✓ SATISFIED | `i` → `DriverInjectScreen` (WR-01: selected run must be the live one) → `DriverInjectRequested` → `schedule_inbox_append` |
| STEER-02 | 18-02, 18-04, 18-10 | ✓ SATISFIED | Four states derived as a pure function of disk; forbidden-vocabulary test |
| STEER-03 | 18-01, 18-02 | ✓ SATISFIED | `sync_data()`-backed append; cross-process delivery test |

**Traceability finding — found open, now closed in flight.** At `cae2f0a` this pass found
`.planning/REQUIREMENTS.md` marking OBS-02, OBS-03, STEER-01 and STEER-03 as `[ ]` / `Pending`
despite all four being delivered and tested — only five of the nine had been flipped during the
phase's plan commits. Commit `76720ab` (landed concurrently with this verification) corrected all
four in both the checklist and the tracking table. Verified closed at HEAD.

## Key Link Verification

| From | To | Via | Status |
|---|---|---|---|
| `App::start_driver_run` | detached driver process | `drive_argv(config_path, …)` + `process_group(0)`; `--config` carried (Phase 17 CR-03) | ✓ WIRED |
| TUI `i` key | `inbox.jsonl` | `DriverInjectScreen` → `DriverInjectRequested` → `schedule_inbox_append` on `spawn_blocking` → `inbox::append` + `sync_data()` | ✓ WIRED |
| `inbox.jsonl` | agent stdin | `read_inbox` (`spawn_blocking`) → `deliver_pending_inbox` → `Executor::send` → `interjected` record | ✓ WIRED |
| Replay echo | `interjection_acted_on` | `observing_replay_echoes` → `echo_rx` arm → `correlate_replay_echo` → `PendingAcks::match_echo` (exact equality, FIFO) | ✓ WIRED |
| `journal.jsonl` growth | Driver pane | notify → `ChangeKind::DriverJournal` → `schedule_journal_tail` → `DriverOutput::retarget` + `push_record` | ✓ WIRED |
| Journal + inbox | four-state widget | `derive_injection_states(inbox, records)` as a pure function of disk; spliced into the pane, superseding raw records | ✓ WIRED |
| `reconcile::last_ended_outcomes` | needs-human badge and `/h` filter | `AppContext::last_outcomes`, read in the same `spawn_blocking` task as the probe, pruned beside `observed_runs` (WR-02 fix) | ✓ WIRED |
| `DriverOutput` / `last_outcomes` | `prune_driver_maps` | `.retain(|alias, _| registered.contains_key(alias))` — asserted by `…_leaked` tests | ✓ WIRED |
| `DriverOutput::clear` | `retarget` only | `clear` is private (`screens/mod.rs:317`), reachable solely through the public `retarget` (CR-02 / WR-09) | ✓ WIRED |

## Behavioural Spot-Checks Re-Run This Session

| Behaviour | Command (all via `rtk proxy`) | Result | Status |
|---|---|---|---|
| Full workspace suite | `cargo test` | 772 passed, 0 failed | ✓ PASS |
| WR-02 traversal, both directions | `cargo test --test journal_run_paths` | 2 passed, 0.00 s | ✓ PASS |
| CR-01 second injection after a boundary | `cargo test --test driver_inbox a_message_appended_before_the_final_drain_is_still_delivered` | 1 passed, 6.10 s | ✓ PASS |
| Driver module unit surface | `cargo test --lib driver_` | 61 passed | ✓ PASS |
| Detail-screen key/scroll surface | `cargo test --lib ui::screens::detail::tests` | 50 passed | ✓ PASS |
| Clamp/follow test existence | `cargo test --lib -- --list \| grep -iE 'clamp\|page_down\|page_up\|follow'` | 22 named tests enumerated across `detail.rs`, `driver.rs`, `help.rs` | ✓ PASS |
| WR-16 release gate | `cargo build --release` + 4 CLI probes + 2 binary string counts | flags rejected in release (rc 2), accepted in debug; string absent from the release binary | ✓ PASS |
| Lint gate + delta | `cargo clippy -- -D warnings`; `cargo clippy --all-targets` | clean; exactly 5 pre-existing | ✓ PASS |
| Dependency fence | `git diff 2e33871..HEAD -- Cargo.toml Cargo.lock` | empty | ✓ PASS |

`tests/driver_reattach.rs::a_fresh_scan_finds_the_orphaned_run_live_with_its_last_journal_step` has
a known flake under full-suite parallel load; it passed in this session's full run.

## Probe Execution

Not applicable — this project has no `scripts/*/tests/probe-*.sh` convention, and no plan or
success criterion in this phase declares a probe. The equivalent gate is the `cargo` toolchain,
executed above.

## Anti-Patterns Found

None. `TBD` / `FIXME` / `XXX` (blocker tier), `TODO` / `HACK` / `PLACEHOLDER` (warning tier), and
`unimplemented!` / `todo!` / "not yet implemented" / "coming soon" all return **zero matches**
across the 33 files this phase touched. WR-05's deferral is recorded in `18-REVIEW.md` with a named
carry-forward obligation — the correct way to record an accepted deviation rather than a silent
one.

## Gaps Summary

**No gaps.** All five ROADMAP success criteria have code-level evidence; four are behaviourally
verified by tests re-run in this session and the fifth (SC1's live-watch) is present, wired, and
routed to a human because no test drives a rendered terminal. All ten named invariants hold,
including the two the phase brief singled out for adversarial treatment: the WR-02 traversal has no
bypassing call site on either side and a whole-filesystem-footprint regression test, and the WR-16
release gate was proven against an actual release binary rather than against its own `#[cfg]`.
All four Phase-17 must-fix carry-ins (D-27/28/29/30) and all four Phase-18 BLOCKERs (CR-01..CR-04)
are fixed in source with named regression tests. Nine of nine requirements are satisfied and the
traceability lag found mid-pass is closed at HEAD.

Three items are carried forward without blocking: WR-05's medium-height tier (a recorded
deferral), the `Interjected`/`is_content()` residual past the 64 MB journal cap (judged not to
block STEER-02, reasoning above), and the absence of a mechanical guard for the D-28 blocking-call
boundary (comment-enforced today, unlike WR-16's `spawn_seam_guard` audit).

Status is `human_needed` rather than `passed` solely because five items require a human at a real
terminal — one behaviour-unverified truth plus the three the phase's own plans declared
`human_judgment: true` and the deferred WR-05 check.

---
_Verified: 2026-07-30T05:37:35Z_
_Verifier: Claude (gsd-verifier)_
