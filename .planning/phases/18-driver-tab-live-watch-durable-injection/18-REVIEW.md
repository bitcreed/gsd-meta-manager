---
phase: 18-driver-tab-live-watch-durable-injection
reviewed: 2026-07-30T04:25:17Z
depth: standard
files_reviewed: 33
files_reviewed_list:
  - src/action.rs
  - src/app.rs
  - src/cli.rs
  - src/driver/mod.rs
  - src/driver/reconcile.rs
  - src/driver/run.rs
  - src/driver/spawn.rs
  - src/error.rs
  - src/executor/claude.rs
  - src/executor/stream_json.rs
  - src/journal/inbox.rs
  - src/journal/mod.rs
  - src/journal/redact.rs
  - src/journal/writer.rs
  - src/main.rs
  - src/ui/screens/delete_confirm.rs
  - src/ui/screens/detail.rs
  - src/ui/screens/driver.rs
  - src/ui/screens/driver_confirm.rs
  - src/ui/screens/driver_inject.rs
  - src/ui/screens/driver_start.rs
  - src/ui/screens/help.rs
  - src/ui/screens/mod.rs
  - src/ui/screens/normal.rs
  - tests/driver_inbox.rs
  - tests/driver_kill.rs
  - tests/driver_lock.rs
  - tests/driver_reattach.rs
  - tests/driver_tracer.rs
  - tests/fixtures/fake-claude-paced.sh
  - tests/fixtures/fake-claude-turns.sh
  - tests/journal_run_paths.rs
  - tests/spawn_seam_guard.rs
findings:
  critical: 4
  warning: 12
  info: 0
  total: 16
status: resolved
resolution:
  fixed: 15
  deferred: 1
  fixed_at: 2026-07-29
  gate: "cargo build + cargo test (772 passed, 0 failed) + cargo clippy -- -D warnings clean; cargo clippy --all-targets still exactly 5 pre-existing lints (browser.rs x3, project_creator.rs x1, state_reader/mod.rs x1)"
  deferred_findings:
    - WR-05
---

# Phase 18: Code Review Report

**Reviewed:** 2026-07-30T04:25:17Z
**Depth:** standard
**Files Reviewed:** 33
**Status:** issues_found

## Summary

The review weighted the ten domain invariants the phase brief names, and
**the four carry-ins and the three named safety invariants hold up under
adversarial reading.** Specifically:

* **D-27 / WR-02 (path traversal) is genuinely closed.** `run_paths` is
  fallible, `is_plain_run_id` refuses a normalising id by comparing the
  component back against the original string, and *every* joining call site
  goes through it: `writer::create_run_dir`, `writer::read_active_run`
  (component check ahead of the `is_dir()` check), `reconcile::reconcile_one`,
  `journal::read_run_summary`, `App::schedule_journal_tail`,
  `App::schedule_inbox_append`, `AppContext::schedule_run_list_scan` and
  `driver::drive`'s own CLI-seam refusal. `grep`ing for a bypass
  (`runs_root(..).join(run_id)`) finds none. `tests/journal_run_paths.rs`
  proves it with a full before/after filesystem footprint rather than a
  handful of `exists()` checks.
* **D-30 / WR-16 is done properly**: the `claude_program` / `claude_args`
  fields carry `#[cfg(debug_assertions)]` on the clap variant, on `DriveArgs`,
  and on `main.rs`'s destructuring pattern, and a debug-build override writes
  `Diagnostic { code: "agent_program_overridden" }` *before* the first exec
  record.
* **D-29 / WR-15 is done properly**: `StopDisposition` is a portable value on
  the `Action`, and `observed_runs` / `session_spawned_runs` are mutated only
  on `RunGone`.
* **D-28 / WR-10**: `lock::acquire`, `JournalRun::start`, `dry_run::build_report`,
  the inbox append, the inbox tail, the run-list scan and the journal tail are
  all behind `spawn_blocking`, and both `RunLock` and `JournalRun` are moved
  back *out* of the blocking task rather than dropped inside it.
* **UIFIX-04 clamp ordering is correct in all three files.** `clamp_scroll` and
  `tail_offset` are the single formula, `driver_offset_now` resolves the follow
  bit through it, PageDown adds-then-clamps and PageUp/`k` clamps-then-subtracts
  in `detail.rs`, `driver.rs` and `help.rs`. No parallel
  `total_lines - visible_height` exists anywhere.
* **`type:"result"` is treated as a turn boundary**, not a run terminator, on
  both the executor and driver sides.
* **The Replit rule holds in the render layer.** `terminal_state_cell`,
  `run_state_glyph`, `TerminalState` and the step timeline read only
  `RunVerdict` and the outcome *label*, the label vocabulary is proved equal to
  `driver::run::outcome_label`'s by test, and the injection vocabulary test
  tokenises rather than substring-matches. No glyph is derived from file
  content.
* **Adopted runs are honest**: `ADOPTED_RUN_NOTICE`, `[journal only]`, and no
  spinner, animation or implied imminence anywhere on the surface.

`cargo test` is green (646 lib + integration binaries), `cargo clippy -- -D
warnings` is clean, and `cargo clippy --all-targets` still shows exactly the
five known pre-existing lints (`browser.rs` ×3, `project_creator.rs` ×1,
`state_reader/mod.rs` ×1) — verified through `rtk proxy` so the check is not
vacuous.

**What does not hold up is the live surface.** Four defects are reachable in
ordinary use and each one is the phase's own named "looks done but isn't"
failure landing in a place the tests do not reach:

* **CR-01** — the drain loop closes the agent's stdin at the first turn boundary
  after a message was delivered by the **poll** arm, because that arm's delivery
  count is discarded. Since the poll fires every 750 ms and a turn lasts minutes,
  essentially every injected message is delivered mid-turn — so a run can be
  steered **exactly once**, and every later message is `missed`. The code comment
  three lines above claims the opposite ("This supports N human-steered turns for
  free"), and `tests/driver_inbox.rs` reproduces the causing sequence without ever
  asserting a second injection.
* **CR-02** — the per-alias output ring is never cleared between runs, so a
  finished run's lines — **including its `run ended: succeeded_with_changes`
  terminal record, which `output_body_lines` deliberately renders last** — appear
  under the next run's header in the next run's colour. `DriverOutput::clear()`
  exists and has zero call sites.
* **CR-03** — a message whose `Executor::send` failed is journaled
  `interjected { delivered: false }`, the inbox cursor advances past it, and no
  further record is ever written. The four-state widget renders it `queued`
  ("durably on disk; **nothing has read it**") forever. That is PITFALLS'
  undelivered-injection failure verbatim, in the code that exists to prevent it.
* **CR-04** — the run-list/inbox/journal scan is scheduled from exactly two
  places (tab entry and selection move) and from nowhere else. A run started
  from the Driver tab is invisible, a queued injection never reaches the widget,
  and a run that ends while you watch silently swaps its live output for a stale
  snapshot.

The warnings below include one honest-badge regression (`needs_human`'s
finished-run arm is unreachable in production, `last_outcome` is `None` at the
only call site), one mis-targeted key (`i` injects into the *observed* run, not
the *selected* one), a duplicate-start hazard in the new wizard, and two newly
reachable argv holes created by this phase's free-text command and goal fields.

## Resolution

**Fixed: 15 of 16. Deferred with rationale: 1 (WR-05).** Every fix carries a
regression test that fails before it and passes after; the four BLOCKERs and
WR-03, WR-04, WR-06 and WR-08 were each verified red against the pre-fix code
before the fix was committed.

| Finding | Status | Commit | Note |
|---|---|---|---|
| CR-01 | fixed | `51e198e` | `delivered_since_boundary` carries the poll arm's count to the boundary arm. The existing test was strengthened to assert the SECOND injection, which is what made the defect visible; `fake-claude-paced.sh` now paces two turns so "after a boundary" is an observed fact rather than a 20ms race. |
| CR-02 | fixed | `b20eafe` | `DriverOutput` carries a `run_id` and `retarget`s (clearing) on change; `clear` is private and reachable only through it. `output_for_run` matches the ring on its own run id, not on `observed_runs`. |
| CR-03 | fixed | `e81405d` | A failed `Executor::send` now writes `interjection_missed { reason: MISSED_SEND_FAILED }`, and the render layer treats `interjected { delivered: false }` as positive evidence of a failed write. **Not** fixed by promoting to `delivered`. `InjectionState::Missed` carries a `MissedReason` so the state and its gloss travel together; still four states, one glyph and one label for `missed`. |
| CR-04 | fixed | `853feae` | Rescan scheduled from the existing 20-tick block (gated on the Driver tab being on screen), from `DriverInjectWritten`'s success arm, and from `start_driver_run`'s spawn success (ungated — the confirmation is still on top). The gate is `driver_tab_is_on_top` and deliberately does **not** require a live run. Also resolves WR-09. |
| WR-01 | fixed | `5d3c0d3` | `i` opens only when the selected run is the live one, with a distinct refusal for each of the two conditions. |
| WR-02 | fixed | `82fb612` | `journal::last_ended_outcome` + `reconcile::last_ended_outcomes` feed `AppContext::last_outcomes`, read in the same `spawn_blocking` task as the probe and pruned beside `observed_runs`. `needs_human` takes a `TerminalState` rather than a `RunOutcome`. |
| WR-03 | fixed | `64fedcb` | New `ScreenAction::Replace`; Step B replaces itself with the confirmation. |
| WR-04 | fixed | `e5aee43` | `--goal` gains `allow_hyphen_values`; `--command` gains a must-start-with-`/` refusal at the field. The split is deliberate — `allow_hyphen_values` on `--command` would let it swallow the following flag. |
| **WR-05** | **deferred** | — | See "Deferred: WR-05" below. |
| WR-06 | fixed | `75caaff` | The control replacement widened to `0x7F..=0x9F`, so `U+009B` (CSI), `U+009D` (OSC) and `U+0090` (DCS) cannot survive. |
| WR-07 | fixed | `b20eafe` | Fixed with CR-02 because it is the same function's contract: the live ring is a preference, not a short-circuit, so an empty ring falls through to the journal on disk. |
| WR-08 | fixed | `45187fc` | Deduped on `seq` (monotonic per run), not on the id — one id legitimately carries several records. |
| WR-09 | fixed | `853feae`, `b20eafe` | Both `App` wrappers deleted; `DriverOutput::clear` wired into `retarget` and made private. |
| WR-10 | fixed | `45187fc` | The enum doc now says what is true. |
| WR-11 | fixed | `af192a6` | Parameter dropped; the intent is a doc comment. |
| WR-12 | fixed | `5591abe` | Asymmetry documented at three sites and pinned by a test asserting **both** halves. Not redacted: the inbox copy is the payload the driver writes to the agent's stdin, so redacting it would change the instruction rather than protect it. |

### Deferred: WR-05 (the medium height tier)

**Carried forward, not fixed.** The finding is correct — `render_pipeline_row`
emits a leading blank line before the D-R-P-E-V row, `steps_lines` emits the
`── steps ──` rule before the command row, and at the `8 <= area.height < 14`
tier each section gets one row, so the pane paints an empty line and a bare rule.
The header budget has the same shape.

It is deferred because the fix is a **layout judgement across three height
tiers**, not a local correction:

* Which element gives way at one row is a design decision the UI-SPEC's
  height-tier tables own. The review offers one answer (drop the spacer, budget
  the header as `goal_row_count.min(1) + 1`); choosing between that and raising
  the tier's floor needs the spec's own priority ordering, which this pass does
  not have in hand.
* The named success criterion at risk is the **elapsed-time counter** surviving a
  two-row goal. Verifying that it does — and that the result is legible rather
  than merely present — is a human UAT question at real terminal heights, not
  something a buffer-scraping test settles on its own.
* No render test exercises `render_run_detail`'s height tiers at all, so the fix
  needs new test scaffolding (a scrape at heights 8, 13 and 14) that is worth
  building once, against the spec, rather than twice.

Nothing about it is a correctness or safety risk: no fact is misreported, no
state is overstated, and no evidence is destroyed. The failure mode is a section
that renders empty where it should render one row.

**Carry-forward obligation:** add the height-tier render tests and the
producer-ordering fix as part of the next UI pass on the Driver tab, with the
UI-SPEC height tables in hand and a human check at heights 8, 13 and 14.

## Structural Findings (fallow)

No `<structural_findings>` block was supplied with this review.

## Narrative Findings (AI reviewer)

### Critical

#### CR-01: the drain loop closes stdin after the first mid-turn injection, so a run can be steered exactly once

**File:** `src/driver/run.rs:1240-1281` (the turn-boundary arm), `src/driver/run.rs:1305-1315` (the poll arm)
**Severity:** BLOCKER

**Status:** FIXED in `51e198e`.

**Issue:** Two arms deliver inbox messages, and only one of them is allowed to
keep stdin open.

```rust
// the poll arm — return value DISCARDED
_ = inbox_poll.tick() => {
    if stdin_open {
        deliver_pending_inbox(&executor, &mut handle, …).await;   // usize dropped
    } else { sweep_inbox_as_missed(…).await; }
}

// the turn-boundary arm — return value decides whether stdin survives
if turn_boundary && stdin_open {
    let delivered = deliver_pending_inbox(…).await;
    if delivered == 0 {
        …handle.close_input()…
        stdin_open = false;
    }
}
```

`deliver_pending_inbox` advances `inbox_cursor` past everything it reads, so a
message consumed by the poll arm is **gone from the boundary drain's view**.
The boundary drain then finds nothing, `delivered == 0`, and stdin closes —
even though a human-steered turn is queued in the CLI and about to run.

`INBOX_POLL_INTERVAL` is 750 ms and a GSD command turn is minutes, so the poll
arm wins the race for essentially every message a user types while watching
output. The sequence is:

1. Turn 1 (the command) is running. User injects. Poll delivers it →
   `interjected { delivered: true }`. CLI queues it.
2. Turn 1's `result` arrives. Boundary drain reads an empty inbox →
   `delivered == 0` → `close_input()`, `stdin_open = false`.
3. The CLI runs the injected message as turn 2. At turn 2's boundary
   `stdin_open` is already `false`.
4. Every subsequent message the user types is journaled
   `interjection_missed` and rendered `✗ missed`.

D-11 step 3 and the loop's own comment at `run.rs:1206-1207` state the
opposite: *"If a message was delivered, the agent runs it as a new turn and the
loop repeats from step 2. This supports N human-steered turns for free."* That
sentence is true only for the ≤750 ms window in which a message arrives between
the turn's last poll and its `result`.

`tests/driver_inbox.rs::a_message_appended_before_the_final_drain_is_still_delivered`
executes this exact sequence against `fake-claude-paced.sh` and asserts only
that the *first* message was delivered — so the defect sits directly inside the
test's own fixture without being visible to it. No test injects a second
message after the first has been acknowledged.

**Fix:** Remember that a delivery happened since the last boundary, and let the
poll arm set the same flag the boundary arm reads.

```rust
let mut delivered_since_boundary = 0usize;

// poll arm
_ = inbox_poll.tick() => {
    if stdin_open {
        delivered_since_boundary +=
            deliver_pending_inbox(&executor, &mut handle, …).await;
    } else {
        sweep_inbox_as_missed(…).await;
    }
}

// boundary arm
if turn_boundary && stdin_open {
    let delivered = delivered_since_boundary
        + deliver_pending_inbox(…).await;
    delivered_since_boundary = 0;
    if delivered == 0 {
        // …close_input() as today…
    }
}
```

Add a regression test that injects, waits for `interjection_acted_on`, injects
again, and asserts a **second** `interjected` record and **no**
`interjection_missed`. Without that test the flag can be dropped again by the
next refactor and nothing fails.

---

#### CR-02: the live output ring is never cleared between runs, so a finished run's terminal record renders under the next run

**File:** `src/app.rs:1256`, `src/ui/screens/mod.rs:243-248`, `src/ui/screens/driver.rs:1660-1690`, `src/ui/screens/driver.rs:1701-1718`
**Severity:** BLOCKER

**Status:** FIXED in `b20eafe` (with WR-07).

**Issue:** `ctx.driver_output` is keyed by **alias only** and the buffer carries
no run id:

```rust
let output = self.ctx.driver_output.entry(key.0.clone()).or_default();
```

`DriverOutput` has a `clear()` method (`mod.rs:243-248`) and **zero call sites
in the entire tree** — production or test. Nothing resets the ring when the run
id changes, and `Action::DriverRunsListed` replaces `driver_runs`,
`driver_inbox` and `driver_journal` but not this map.

`output_for_run` hands the whole ring to the renderer whenever the selected run
is the tailed one:

```rust
let tailed = ctx.observed_runs.get(alias)
    .is_some_and(|observed| observed.run_id == run_id);
if tailed { return ctx.driver_output.get(alias); }
```

So after run A finishes and run B starts in the same project, the pane for run
B shows run A's assistant output, run A's diagnostics, and — worst — run A's
terminal records. `output_body_lines` deliberately holds `DriverLineKind::Terminal`
back and appends it **last**, styled with the *currently selected run's*
terminal colour:

```
…run B's live output…
= run ended: succeeded_with_changes        ← run A's record, Magenta (B is live)
= agent finished: exit 0 after 214s        ← run A's record
```

A live run therefore displays a previous run's success as its own visual full
stop. That is precisely the "display disagreeing with the disk in the direction
that flatters the run" the module doc names, and it is the same class of error
the run id inside `DriverRunTally` and `DriverRunJournal` was added to prevent —
the one map that needed it did not get it.

The `dropped()` and `record_truncated()` signals are also carried across runs,
so run B reports run A's dropped-line count as its own.

**Fix:** Give `DriverOutput` a run id and reset on change, exactly as
`update_driver_tally` already does for the tally:

```rust
// src/ui/screens/mod.rs
pub struct DriverOutput {
    run_id: String,
    lines: VecDeque<DriverOutputLine>,
    dropped: u64,
    record_truncated: bool,
}

impl DriverOutput {
    /// Reset when the buffer starts speaking for a different run.
    pub fn retarget(&mut self, run_id: &str) {
        if self.run_id != run_id {
            self.clear();
            self.run_id = run_id.to_string();
        }
    }
    pub fn run_id(&self) -> &str { &self.run_id }
}

// src/app.rs, in the DriverJournalAppended arm, before the first push
let output = self.ctx.driver_output.entry(key.0.clone()).or_default();
output.retarget(&key.1);
```

and have `output_for_run` require `output.run_id() == run_id` rather than
inferring it from `observed_runs`. Add a test that pushes records under two run
ids and asserts the first run's lines are gone.

---

#### CR-03: a message whose stdin write failed sits in `queued` forever — the exact state D-10 exists to abolish

**File:** `src/driver/run.rs:696-740`, `src/ui/screens/driver.rs:1445-1460`
**Severity:** BLOCKER

**Status:** FIXED in `e81405d`.

**Issue:** `deliver_pending_inbox` journals `Interjected { delivered: false }`
when `Executor::send` returns `Err`, and then does nothing further:

```rust
Err(err) => { tracing::warn!(kind = ?err, "…"); false }
…
journal.record(&JournalEvent::Interjected { id, text, delivered })  // delivered == false
```

The message was consumed from the tail, so `inbox_cursor` has already moved
past it. It can never be read again, never retried, and never reaches
`journal_as_missed` — `sweep_inbox_as_missed` only sees messages the cursor has
not passed. There is no third record.

The render side then leaves it in `Queued`:

```rust
"interjected" => {
    let written = record.rest.get("delivered").and_then(as_bool).unwrap_or(false);
    if !written { continue; }          // ← stays Queued forever
    InjectionState::Delivered
}
```

and `Queued`'s glyph and gloss are `○ queued` / *"durably on disk; nothing has
read it yet"* (`driver.rs:1355-1356`, `help.rs:INJECTION_LEGEND`). Both are
false: the driver **did** read it and the write **did** fail. The user is shown
a state that says "waiting to be picked up" about a message that will never be
picked up — PITFALLS' Pitfall 11 in the code written to prevent it, and the
`derive_injection_states` doc actively rationalises it as *"the message stays
`queued`, which is what it still is."*

Reachable whenever the writer task has gone while `stdin_open` is still `true`
— i.e. the agent exited or crashed between the last event and the poll —
returning `SendError::WriterGone` or `NotRunning`.

There is a second path to the same wrong display: `Interjected` is
`is_content()`, so once `MAX_RUN_JOURNAL_BYTES` is reached the delivery record
is suppressed entirely while `interjection_acted_on` (not content) is still
written; a message delivered past the cap and never echoed also shows `queued`
forever.

**Fix:** Make an undeliverable write terminal at the driver, where the fact is
known:

```rust
Err(err) => {
    tracing::warn!(kind = ?err, "could not write an injected message to the agent's stdin");
    if let Err(e) = journal.record(&JournalEvent::InterjectionMissed {
        id: message.id.clone(),
        reason: SEND_FAILED.to_string(),   // a second fixed reason beside MISSED_AFTER_CLOSE
    }) { tracing::warn!(kind = ?e.kind(), "journal write failed"); }
    false
}
```

and, on the render side, stop treating `delivered: false` as an absence of
evidence — an `interjected` record with `delivered: false` is positive evidence
that the write failed, so it must never leave the message in `Queued`. Add the
send-failure reason to the render layer's gloss table and a unit test over
`derive_injection_states` asserting `interjected{delivered:false}` never yields
`Queued`.

---

#### CR-04: the run list, inbox and journal are scanned only on tab entry and selection move, so the Driver tab goes stale while it is open

**File:** `src/ui/screens/detail.rs:639-643`, `src/ui/screens/detail.rs:345-376`, `src/app.rs:923-985`, `src/app.rs:1452-1483`
**Severity:** BLOCKER

**Status:** FIXED in `853feae` (with WR-09).

**Issue:** `AppContext::schedule_run_list_scan` has exactly two call sites —
`switch_to_tab`'s `DetailSubView::Driver` arm and `move_driver_selection`.
Nothing schedules it from `Action::Tick`, from `Action::FileChanged`, from
`Action::DriverInjectWritten`, or from `Action::RunsReconciled`. A watcher event
under `runs/<id>/` classifies as `ChangeKind::DriverJournal` and routes to
`schedule_journal_tail`, which reads `journal.jsonl` and nothing else.

`cache.driver_runs`, `cache.driver_inbox` and `cache.driver_journal` are
therefore frozen at the moment the tab was entered. Three concrete failures,
all in ordinary use:

1. **A run started from the Driver tab is invisible.** `s` → command → goal →
   `y` pops back to `DetailScreen` without re-entering the tab. `driver_runs`
   is still empty, so `render_driver_tab` takes its `runs.is_empty()` early
   return and paints *"No runs yet for "alias". Press [s] to start one."* while
   the run is live and the ring buffer fills invisibly behind it. This is the
   first thing a new user does with TRANS-05.
2. **The `queued` state is never rendered.** `derive_injection_states` iterates
   `cache.driver_inbox`; a message queued through `i` does not appear there
   until a rescan, so `entries` stays empty, `output_body_lines` falls through
   to the raw-record path, and the four-state widget — STEER-02's entire
   deliverable — does not render at all. The user sees the status line
   *"Queued — waiting for the driver to pick it up."* and then nothing until
   the `interjected` record arrives via the journal tail, at which point the
   pane shows a bare `» skip the UI review` with **no state label**.
3. **A run that ends while you watch loses its output.** `reconcile_one` returns
   `None` for `RunVerdict::Ended`, so the alias drops out of `observed_runs`.
   `output_for_run`'s `tailed` flag flips to `false` and the pane switches from
   the live ring to `cache.driver_journal` — the snapshot taken when the tab was
   entered. Every line since then vanishes with no drop notice, at the exact
   moment OBS-05's after-the-fact review is wanted. The run-list glyph
   simultaneously becomes `◇ ?` / *"outcome not recorded"*, because
   `summary.outcome` is the stale `None` from the same snapshot.

**Fix:** Reschedule the scan on the conditions that invalidate it, on the
existing 20-tick block (never a second timer, per the comment at `app.rs:959-965`)
plus the two events that change the answer immediately:

```rust
// Action::Tick, inside the existing 20-tick block
if let Some(alias) = self.ctx.selected_alias() {
    if matches!(self.ctx.detail_sub_view_per_project.get(&alias), Some(DetailSubView::Driver)) {
        if let Some(path) = self.ctx.config.projects.get(&alias).map(|p| p.path.clone()) {
            self.ctx.schedule_run_list_scan(&alias, &path);
        }
    }
}

// Action::DriverInjectWritten, on the `None` (success) arm — the queued state
// is not observable until the inbox is re-read.
// Action::DriverStartRequested's success path in start_driver_run — the new run
// must appear in the list without a tab round-trip.
```

Note the scan is three blocking reads, so gate it on the Driver tab actually
being on top (the same predicate `driver_elapsed_redraw_wanted` already
computes) rather than running it for every project.

---

### Warnings

#### WR-01: `i` injects into the *observed* run, not the *selected* one

**File:** `src/ui/screens/detail.rs:2125-2144`
**Severity:** WARNING

**Status:** FIXED in `5d3c0d3`.

**Issue:** The `i` binding reads the run id out of `ctx.observed_runs`:

```rust
let live_run = ctx.observed_runs.get(&self.alias)
    .filter(|observed| observed.is_live())
    .map(|observed| observed.run_id.clone());
```

but the pane the user is looking at renders `cache.driver_runs[driver_selected_run]`,
and `j`/`k` move that selection freely across the whole run history. A user who
has pressed `j` to look at last week's run and then presses `i` gets an input
aimed at *today's live run* — and because `cache.driver_inbox` is the **selected**
run's inbox, the queued message does not appear anywhere on the surface they are
looking at. The `no_live_run_message` refusal likewise fires on the wrong run:
selecting a finished run while another is live still opens the input.

**Fix:** Refuse unless the selected run is the live one, so the target and the
pane always agree:

```rust
let selected_run_id = ctx.view_cache.get(&self.alias)
    .and_then(|c| c.driver_runs.get(c.driver_selected_run))
    .map(|run| run.run_id.clone());
let live_run = ctx.observed_runs.get(&self.alias)
    .filter(|observed| observed.is_live())
    .filter(|observed| Some(&observed.run_id) == selected_run_id.as_ref())
    .map(|observed| observed.run_id.clone());
```

---

#### WR-02: `needs_human`'s finished-run arm is unreachable in production, so the badge and the `/h` filter never fire for a failed run

**File:** `src/ui/screens/mod.rs:755-778`, `src/ui/screens/mod.rs:828-832`
**Severity:** WARNING

**Status:** FIXED in `82fb612`.

**Issue:** D-14 names four evidence sources for "waiting on a human", one of
which is *"a finished run whose outcome is `PermissionDenied` / `Failed` /
`Stalled` / `TimedOut`"*. The pure function implements it and has unit tests.
The **only production call site** passes `None`:

```rust
pub fn needs_human_for(&self, alias: &str) -> bool {
    self.project_states.get(alias)
        .is_some_and(|state| needs_human(state, self.observed_runs.get(alias), None, false))
}
```

`grep -rn "needs_human(" src/` confirms `Some(&outcome)` appears only inside
`#[cfg(test)]`. So `BADGE_NEEDS_HUMAN`, `attention_rank`'s rank 0 and the `/h`
filter all fire from `state.paused` and `state.external_job_waiting` only — the
two v1.4 signals that existed before this phase. A project whose driver run
failed, was permission-denied, stalled or timed out is not flagged, is not
sorted first, and does not appear under `//h`.

The data is on disk and already reachable: `RunSummary.outcome` comes from
`journal::list_runs`, which the Driver tab already calls. This is the failure
mode 18-CONTEXT names by hand — *"a badge that can never light is worse than no
badge because it teaches the user to ignore it"* — applied to the arm the phase
itself was supposed to wire.

**Fix:** Carry the newest finished run's outcome label per alias (the
reconciliation scan already reads `run.json`; the newest ended run is one more
`read_dir` entry), map it through `TerminalState::from_label`, and pass it:

```rust
pub fn needs_human_for(&self, alias: &str) -> bool {
    let last = self.last_outcomes.get(alias);   // new sibling map, pruned in prune_driver_maps
    self.project_states.get(alias)
        .is_some_and(|state| needs_human(state, self.observed_runs.get(alias), last, false))
}
```

and add a dashboard-level test asserting a project with a `failed` run gets the
badge and survives `//h`. Without that test the arm goes quiet again.

---

#### WR-03: the start wizard does not pop after confirming, so a second Enter starts a duplicate run

**File:** `src/ui/screens/driver_start.rs:218-230`, `src/ui/screens/driver_confirm.rs:210-219`, `src/app.rs:1745-1765`
**Severity:** WARNING

**Status:** FIXED in `64fedcb`.

**Issue:** Step B pushes the confirmation and stays on the stack:

```rust
(StartStep::Goal, KeyCode::Enter) => {
    …
    ScreenAction::Push(Box::new(DriverConfirmScreen::new_start(…)))
}
```

`DriverConfirmScreen`'s `y` arm dispatches the start and returns
`ScreenAction::Pop`, which lands the user back on `DriverStartScreen` at Step B
with an empty input buffer and the dry-run preview closed. The screen looks
exactly as it did before they confirmed. Pressing `Enter` again pushes a second
confirmation for the same command; `y` starts a **second** run.

`admit(live, driver_max_concurrent)` counts live runs across all projects and
does not stop a second run of the same project, and the child's `flock` refusal
is invisible (stdio is `/dev/null`). Meanwhile `start_driver_run` has already
done `observed_runs.insert(alias, …)` with the second driver's pid — so for up
to five seconds `x` would signal the wrong (already-dead) pid while the real
run keeps going.

**Fix:** Pop the wizard when it hands off:

```rust
(StartStep::Goal, KeyCode::Enter) => {
    …
    ScreenAction::Replace(Box::new(DriverConfirmScreen::new_start(…)))
}
```

or, if `ScreenAction` gains no `Replace`, have the confirmation pop twice for a
`Start` that came through the wizard. Add a test that presses `Enter` twice at
Step B and asserts exactly one `DriverStartRequested` on the channel.

---

#### WR-04: the new free-text command and goal reach the child's argv with no hyphen guard

**File:** `src/ui/screens/driver_confirm.rs:303-312`, `src/driver/spawn.rs:57-79`, `src/cli.rs:54-55,79-80`
**Severity:** WARNING

**Status:** FIXED in `e5aee43`.

**Issue:** This phase turned two argv operands into free-text fields. Both are
passed verbatim:

```rust
argv.push(OsString::from("--command"));
argv.push(OsString::from(command));
if let Some(goal) = goal {
    argv.push(OsString::from("--goal"));
    argv.push(OsString::from(goal));
}
```

Neither `#[arg(long)] command: String` nor `#[arg(long)] goal: Option<String>`
carries `allow_hyphen_values`, so any value beginning with `-` makes clap in the
child report *"a value is required for '--command <COMMAND>' but none was
supplied"* and exit non-zero. The child's stdio is `/dev/null`, so nothing is
visible; the TUI has already shown *"Driving {alias} — run {id}"* and inserted an
optimistic `ObservedRun { liveness: Alive }`, which disappears at the next scan
with no error. A goal such as `-- do not touch main` or `-v2 migration notes` is
entirely plausible free text.

Phase 17's WR-13 named this class for the *alias*; this phase made it reachable
for two user-typed fields and did not close it.

**Fix:** Both halves — reject at the seam and terminate option parsing:

```rust
// cli.rs
#[arg(long, allow_hyphen_values = true)]
goal: Option<String>,

// driver_start.rs, Step A commit — a GSD command is a slash command
if !command.starts_with('/') { /* refuse, keep the field open */ }
```

and update `tests/spawn_seam_guard.rs::the_drive_argv_carries_no_development_flag`,
which pins the exact argv.

---

#### WR-05: the medium height tier renders a blank pipeline row and a bare `── steps ──` rule

**File:** `src/ui/screens/driver.rs:1076-1102`, `src/ui/screens/driver.rs:1161-1175`, `src/ui/screens/driver.rs:1211-1234`
**Severity:** WARNING

**Status:** DEFERRED — see "Deferred: WR-05" in the Resolution section above.

**Issue:** For `8 <= area.height < 14` the layout gives the pipeline section and
the steps section one row each:

```rust
Layout::vertical([Constraint::Length(2), Constraint::Length(1),
                  Constraint::Length(1), Constraint::Min(3)])
```

with the comment *"Every section is still present, at one row each. No section
is ever allocated zero rows … because an empty section rule reads as a section
that failed to load."* But both producers emit a leading row that is not the
content:

* `render_pipeline_row` builds `vec![Line::from("")]` and *then* pushes the
  D-R-P-E-V line — so at one row the pane paints **an empty line** and the
  pipeline is invisible.
* `steps_lines` builds `[section_rule("steps", width), <command row>, <honest
  note>]` — so at one row the pane paints **only the rule**, with nothing under
  it. That is verbatim the failure the comment says the tier prevents.

The header budget has the same shape: `Constraint::Length(2)` with
`header.into_iter().take(2)`, where `render_run_header` emits
`goal_lines (1–3) + cmd/elapsed + cost [+ run dir]`. A goal that wraps to two
rows therefore drops the command **and the elapsed-time counter** — a named
success criterion — entirely.

No test exercises `render_run_detail`'s height tiers; the height-related tests
are all against the pure `steps_lines` / `goal_lines` helpers.

**Fix:** Either raise the tier's floor for those sections, or make each producer
put its content first when it has one row:

```rust
fn pipeline_lines(inference: Option<&DiskInference>, rows: u16) -> Vec<Line<'static>> {
    let mut lines = Vec::new();
    if rows >= 2 { lines.push(Line::from("")); }   // the spacer is what gives way
    …
}
```

and budget the header as `goal_row_count.min(1) + 1` at the medium tier so the
command/elapsed row always survives. Add a render test at heights 8, 13 and 14
that scrapes the buffer and asserts the pipeline line and the step row are
present at every tier that claims to show them.

---

#### WR-06: `sanitize_render_line` strips C0 and `ESC` but lets C1 controls through, including `U+009B` (CSI)

**File:** `src/ui/screens/mod.rs:279-320`
**Severity:** WARNING

**Status:** FIXED in `75caaff`.

**Issue:** The gate is documented as *"the single append-time gate"* every
untrusted string from disk passes through, and its stated highest-value rule is
stripping the escape introducer. The filter is:

```rust
if ch == ESC { continue; }
…
} else if (ch as u32) < 0x20 || ch == '\u{7f}' {
    push(&mut out, &mut n, CONTROL_REPLACEMENT)
```

`U+0080`–`U+009F` — the C1 control block — is neither stripped nor replaced.
`U+009B` is the single-character CSI, `U+009D` is OSC, and `U+0090` is DCS.
ratatui writes each grapheme's bytes straight to the terminal, so these reach
the emulator as `0xC2 0x9B` etc. Terminals that honour 8-bit controls decoded
from UTF-8 (xterm without `allowC1Printable`, and several others) will treat the
following bytes as a control sequence — reinstating exactly the repaint-the-screen
/ forge-a-status-line capability the `ESC` rule exists to remove, from agent
prose, a branch name in the dry-run report, or a `Diagnostic.detail`.

**Fix:** Widen the replacement range; the cost is one comparison.

```rust
} else if (ch as u32) < 0x20 || ('\u{7f}'..='\u{9f}').contains(&ch) {
    push(&mut out, &mut n, CONTROL_REPLACEMENT)
```

and extend the existing sanitiser test corpus with `"\u{9b}31m"` and
`"\u{9d}0;title\u{9c}"`, asserting neither survives.

---

#### WR-07: an adopted live run with a quiet journal renders "No journal entries yet." over a journal that is on disk

**File:** `src/ui/screens/driver.rs:1701-1718`, `src/ui/screens/driver.rs:1637-1651`
**Severity:** WARNING

**Status:** FIXED in `b20eafe` (with CR-02 — same function).

**Issue:** `output_for_run` prefers the live ring unconditionally when the
selected run is the observed one:

```rust
if tailed { return ctx.driver_output.get(alias); }
cache.driver_journal…            // never reached for the tailed run
```

and `output_body_lines` then does `output.filter(|output| !output.is_empty())`,
falling into the no-entries branch. After a TUI restart the ring for that alias
is empty until the first watcher event fires, so an adopted run that is alive
but momentarily quiet paints:

```
This run started before the current TUI session. Its live output is gone — showing the journal on disk.
  No journal entries yet.
```

The second line contradicts the first, and `cache.driver_journal` — which the
run-list scan already read whole off disk for exactly this case — is thrown
away.

**Fix:** Fall back rather than short-circuit:

```rust
if tailed {
    if let Some(live) = ctx.driver_output.get(alias).filter(|o| !o.is_empty()) {
        return Some(live);
    }
}
cache.and_then(|cache| cache.driver_journal.as_deref())
    .filter(|journal| journal.run_id == run_id)
    .map(|journal| &journal.output)
```

(Once CR-02's run id lands on `DriverOutput`, key the first branch off that
instead of off `observed_runs`.)

---

#### WR-08: `driver_journal.injections` accumulates duplicates across a scan/tail overlap

**File:** `src/app.rs:1292-1306`
**Severity:** WARNING

**Status:** FIXED in `45187fc`.

**Issue:** The `DriverJournalAppended` handler extends the scanned journal's
injection list:

```rust
journal.injections.extend(records.iter().filter(|r| INJECTION_KINDS.contains(…)).cloned());
```

The scan (`schedule_run_list_scan`) reads the journal **whole from offset zero**
and replaces `driver_journal`, while the tail keeps its own independent
`journal_cursors` offset. When a scan lands after a tail has already consumed
records, the scan's snapshot contains them and the next tail batch re-appends
whatever the scan read past. Nothing dedupes by `seq` or by id, so the vector
grows with duplicates for the life of the selection.

The state derivation is idempotent under duplication (same rank, same
timestamp), so this is a growth and clarity problem rather than a wrong render —
but the field's doc claims it is *"naturally bounded: one record per state
transition of one message a human typed"*, which is no longer true.

**Fix:** Dedupe on insert by the record's `seq`, which the journal already
guarantees is monotonic:

```rust
let known: std::collections::HashSet<u64> =
    journal.injections.iter().map(|r| r.seq).collect();
journal.injections.extend(
    records.iter()
        .filter(|r| INJECTION_KINDS.contains(&r.kind.as_str()) && !known.contains(&r.seq))
        .cloned(),
);
```

---

#### WR-09: three public driver entry points have no callers

**File:** `src/app.rs:828-830`, `src/app.rs:842-845`, `src/ui/screens/mod.rs:243-248`
**Severity:** WARNING

**Status:** FIXED in `853feae` (wrappers) and `b20eafe` (`clear`).

**Issue:** `App::schedule_run_list_scan` and `App::schedule_dry_run_report` are
one-line delegations to the `AppContext` methods, and neither has a caller
anywhere in `src/` or `tests/` — every real call goes to
`ctx.schedule_*` directly (`detail.rs:372`, `detail.rs:641`,
`driver_start.rs:162`). They are `pub`, so no `dead_code` warning fires and the
duplication is invisible to the compiler.

`DriverOutput::clear` is likewise `pub` and uncalled — and its absence is CR-02.

**Fix:** Delete the two `App` wrappers (or make them `#[cfg(test)]` if a test
seam is wanted), and wire `clear` into the retargeting described in CR-02.

---

#### WR-10: `DetailSubView::Driver`'s doc says the tab is unreachable, which it no longer is

**File:** `src/app.rs:30-36`
**Severity:** WARNING

**Status:** FIXED in `45187fc`.

**Issue:**

```rust
/// **Index 10, and it is deliberately not yet reachable.** This plan adds
/// the variant and the two arms `detail.rs` needs to compile; the tab title
/// vector, the `Shift+D` binding, the footer hints and the rendering are
/// plan 18-09's. Until those land, `TAB_TITLES` still has ten entries, so
/// `Right` stops at index 9 and nothing selects this …
Driver,
```

Plan 18-09 landed all five of those. `TAB_COUNT` is 11, `TAB_LABELS_FULL` has
eleven entries, `Shift+D` is bound, the footer tiers exist and both render
dispatches are wired. This is the definitional site for the enum, so a reader
starting there is told the tab does not exist.

**Fix:** Replace the paragraph with what is now true — index 10, reached by
`Left`/`Right` and `Shift+D`, rendered by `super::driver::render_driver_tab`
from both dispatch arms.

---

#### WR-11: `render_run_header` takes a `verdict` it explicitly discards

**File:** `src/ui/screens/driver.rs:762-831`
**Severity:** WARNING

**Status:** FIXED in `af192a6`.

**Issue:** The parameter is threaded through the call chain and then defused:

```rust
// `verdict` is threaded through so a future reader sees that the header has
// the evidence in hand and deliberately renders no status word of its own …
let _ = verdict;
```

A parameter whose only statement is `let _ =` is a parameter that will be
silently mis-wired by the next edit — the compiler cannot tell whether a future
change forgot to use it or deliberately did not. The intent is a comment, not a
signature.

**Fix:** Drop the parameter and keep the sentence as a doc comment on the
function:

```rust
/// **This header renders no status word of its own.** The state word belongs to
/// the run list and the step timeline; saying it three times would invite three
/// renderings of one fact.
fn render_run_header(summary: &RunSummary, cost_usd: Option<f64>, …)
```

---

#### WR-12: the inbox stores the user's message unredacted while the journal redacts the same text

**File:** `src/journal/inbox.rs:178-191`, `src/journal/mod.rs:642-656`
**Severity:** WARNING

**Status:** FIXED in `5591abe`.

**Issue:** `inbox::append` serialises `InboxMessage` and writes it directly; no
`redact::redact` is applied. The same text, when journaled as
`JournalEvent::Interjected`, goes through the redactor like every other journal
write. So a user who pastes a token or a key into the injection input gets it
redacted in `journal.jsonl` and stored verbatim in `inbox.jsonl` beside it — and
the render path reads the **inbox** copy, so the unredacted value is what appears
on screen.

Both files are covered by `RUNS_GITIGNORE_BODY`'s catch-all, so nothing is
committed, and the asymmetry is arguably deliberate (the human's own words come
back verbatim, per OBS-03's register). But the module doc claims *"Nothing here
logs message content"* while the file itself is the one place message content
lands unredacted on the driven project's disk, and the divergence is not
recorded anywhere.

**Fix:** Either redact the inbox copy on write and render the journal's
`Interjected.text` instead, or state the asymmetry explicitly on
[`inbox::append`] and on `InboxMessage::text` — "verbatim and deliberately not
redacted; the journal's copy is the redacted one" — so a later reader does not
assume the redactor covers this path.

---

_Reviewed: 2026-07-30T04:25:17Z_
_Reviewer: Claude (gsd-code-reviewer)_
_Depth: standard_
_Resolved: 2026-07-29 — 15 fixed, 1 deferred (WR-05). Gate green: 772 tests, 0 failed;_
_`cargo clippy -- -D warnings` clean; `--all-targets` still exactly 5 pre-existing lints._
