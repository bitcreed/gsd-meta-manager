# Phase 18: Driver Tab, Live Watch & Durable Injection - Context

**Gathered:** 2026-07-29
**Status:** Ready for planning
**Mode:** Autonomous — grey areas proposed and auto-accepted by the orchestrator per user
direction ("best well-reasoned guess is good enough; don't ask questions"). Every decision below
is Claude's Discretion unless tagged otherwise, and is listed explicitly so it can be corrected at
the end. Decisions marked **[LOCKED-BY-RESEARCH]** were settled by the committed research pass
(`.planning/research/{SUMMARY,ARCHITECTURE,PITFALLS,FEATURES}.md`) and are carried in as given.
Decisions marked **[LOCKED-BY-SPIKE]** were settled *empirically* against the installed CLI
2.1.220 during Phase 15 and **override the committed research where they disagree** — Phase 15's
`15-CONTEXT.md` D-29..D-32 are the authority, not ARCHITECTURE §2.3/§5.5/AP3. Decisions marked
**[MUST-FIX-CARRY-IN]** are Phase 17 warnings the user has explicitly refused to let this phase
defer again.

<domain>
## Phase Boundary

**A human can run and steer a driven project from the TUI, and can see — from evidence, never
from the agent's prose — exactly what it is doing.** This is the manual ship point that completes
backlog 999.2, and it is independently useful before any autonomy exists.

In scope:

- **TRANS-05** — pick a GSD command for an opted-in project, start it from the TUI, watch its
  output stream live.
- **OBS-02** — the project list marks driven projects and parked projects, distinctly.
- **OBS-03** — the originating goal prompt is readable for any driven project.
- **OBS-04** — a Driver tab showing live output, current step, elapsed time, step history.
- **OBS-05** — after-the-fact review of a finished or failed run: which commands ran, why it
  stopped.
- **OBS-07** — sort or filter the dashboard down to the projects waiting on a human.
- **STEER-01** — inject a message into a running driver from the TUI.
- **STEER-02** — that message shows queued → delivered → acted-on, never a bare "sent".
- **STEER-03** — a message queued moments before a TUI restart is still delivered afterwards.

Also in scope, because this phase is where they become reachable and nowhere earlier:

- **The four Phase 17 MUST-FIX warnings (D-30..D-33).** WR-02 (path traversal, reproduced),
  WR-10 (blocking calls in `async fn` — already caused an *observed* deadlock), WR-15
  (`DriverStopped` drops a run that was never stopped), WR-16 (hidden `--claude-program` ships in
  release). WR-10 in particular is this phase's business *because Phase 18 is the TUI-side caller
  the review named*: `tests/driver_lock.rs:201-215` records the deadlock as observed, not
  theorised.
- **The stdin lifetime change.** `src/driver/run.rs:605-618` calls `handle.close_input()`
  immediately after spawn. **While that line stands, STEER-01 is physically impossible** — the
  writer task has broken its loop and `Executor::send` returns `WriterGone`. Changing when stdin
  closes is a run-lifecycle change and it belongs to whatever phase first needs to write to a
  live agent, which is this one.
- **`inbox.jsonl`** as a new per-run artifact, and its consumption arm inside the driver's drain
  loop. `tests/journal_gitignore.rs:142,194` already assert `<run-dir>/inbox.jsonl` is ignored —
  the ignore posture exists; the file does not.
- **A readable text projection for `JournalEvent::ExecEvent`.** `journal::from_exec_event`
  (`src/journal/mod.rs:822-825, 846-849`) currently stores `format!("{m:?}")` — a Rust `Debug`
  rendering of `TurnMessage`, whose message body `src/executor/stream_json.rs:105-107` explicitly
  declines to model, calling content blocks *"Phase 18's rendering concern"*. A live-output pane
  fed `Debug` structs does not satisfy "watch its output", and the interjection echo cannot be
  correlated without the text. One change serves OBS-04 and STEER-02 both.
- **The command picker and the goal field.** `Action::DriverStartRequested.command`
  (`src/action.rs:94-113`) is filled today from the single constant
  `driver_confirm::DEFAULT_DRIVE_COMMAND`; its own doc says *"Phase 18's command picker is what
  makes the field carry more than one value."* `App::start_driver_run` already threads
  `goal: Option<&str>` and passes `None` because *"there is no screen to type one into — that is
  Phase 18's"* (`src/app.rs:769-774`). OBS-03 is unreachable while the goal is always empty.

Out of scope — each has a named later owner:

- **`decide()`, the D-R-P-E-V router, multi-command sequences, run bounds, quota park,
  no-progress and repeat-command detection** → Phase 20. **A Phase 18 run still executes exactly
  one GSD command per `drive` invocation** (plus any turns the human's own interjections add).
  The step timeline therefore has one *decided* row, and the UI must say that honestly rather
  than implying a computed sequence exists.
- **The `Parked` producer.** `JournalEvent::Parked { reason, needs }` (`src/journal/mod.rs:439`)
  is Phase 20's to emit. Phase 18 renders it if present (forward-compatible) but derives its own
  parked/needs-human signal from evidence that exists today — see D-14.
- **Git blast-radius enforcement: push allowlists, `--disallowedTools`, pre-push hooks, secret
  scanning, worktree isolation** → Phase 19. Phase 18 *surfaces* the dry-run report; it does not
  police it.
- **LLM goal decomposition and prompt-injection hardening** → Phase 21. Phase 18 stores the
  human's goal string verbatim and interprets nothing.
- **`ExecutionTarget::Container`** → Phase 22.
- **The `control.sock` fast path.** [LOCKED-BY-RESEARCH] REQUIREMENTS defers it to a later
  milestone; ARCHITECTURE §5.5 says *"build the durable path first."* File inbox only.
- **Live re-streaming after a TUI restart.** Impossible by construction (Phase 17 D-11): once
  the TUI exits, the child's stdout pipe is gone. OBS-05's after-the-fact review reads the
  journal. Nothing in code, doc, or status text may promise otherwise.
- **Orphan sweeping via `pgrep -x claude` + `/proc/<pid>/cwd`.** Phase 17's deferred list named
  Phase 18 as the owner. **Declined here and re-homed:** it is belt-and-braces for a SIGKILL'd
  driver, no Phase 18 requirement reaches it, and its natural home is Phase 20's
  before-each-iteration human-collision check, where "sweep before acting" first means something.
  D-09's journaled `claude` pgid already covers every non-SIGKILL case.
- **Fleet-level aggregate rows (driven count, total burn)** → deferred past v2.0 by REQUIREMENTS.
  OBS-07 is satisfied by a per-project filter and sort, not by a new aggregate view.
- **The 5 pre-existing `--all-targets` clippy lints** — the count must not grow; it is not this
  phase's to shrink.

### Two Phase 16 carry-forwards are already closed — do not re-do them

The brief lists them as "fold in if cheap". **Both were landed by Phase 17 and re-doing them
would be churn:** `journal_cursors` is pruned by `App::prune_driver_maps`
(`src/app.rs:841-875`, bounded to `journal::RETAIN_RUNS` per alias, on the existing 20-tick
block), and cross-batch sequence-gap blindness is fixed by
`reader::JournalCursor { cursor, last_seq }` (`src/journal/reader.rs:90-97`) plus
`reader::seq_gaps_from` (`:321`), consumed at `src/app.rs:722-727`. **The only carry-forward
obligation left is a negative one: every new per-alias or per-run map this phase adds must be
pruned in `prune_driver_maps` too, or the leak is simply reintroduced under a new name.**

</domain>

<decisions>
## Implementation Decisions

### The injection channel — what it is, and what it is not

- **D-01:** **[LOCKED-BY-RESEARCH][ANTI-REQUIREMENT] The control channel is stdin `stream-json`.
  It is never tmux.** `tmux send-keys` is a named anti-feature in FEATURES.md: driver keystrokes
  and human keystrokes interleave on a shared pane and corrupt each other, there is no delivery
  confirmation, and it breaks if the pane is mid-prompt. **tmux stays attach/watch-only** — the
  existing `terminal_switch.rs` path is untouched by this phase. A plan that reaches for
  `send-keys` has failed the phase.

- **D-02:** **[LOCKED-BY-SPIKE — Phase 15 D-31] There is NO turn-boundary flush buffer, because
  the CLI already does that internally.** ARCHITECTURE's AP3 ("a mid-turn message is dropped and
  lost from history; buffer it and flush at the `result` boundary") is **REFUTED** on 2.1.220:
  a message written mid-turn is **queued and executed as its own turn**. `Executor::send`
  (`src/executor/claude.rs:547-556`) writes straight through, and its doc at `:522-546` says so.
  Building the AP3 buffer would duplicate the CLI's own queue and make `still_queued` accounting
  incoherent. **Any plan proposing a driver-side flush buffer is wrong and must be rejected at
  review.**

- **D-03:** **The TUI cannot write to the agent's stdin, and this is the single fact that shapes
  the whole feature.** `ExecutionHandle::stdin_tx` (`src/executor/mod.rs:410`) exists only inside
  the detached driver process (`src/driver/run.rs:580`). The TUI and the driver share exactly one
  primitive: the filesystem (ARCHITECTURE §3). Therefore the injection path is, in order:
  **TUI appends a line to `runs/<run-id>/inbox.jsonl` → driver tails it from a byte cursor →
  driver calls `Executor::send` → driver journals the outcome → TUI reads the journal.**
  There is no shortcut, and any design in which the TUI "sends" anywhere other than to a file is
  a design that cannot survive a TUI restart and therefore cannot satisfy STEER-03.

- **D-04:** **Reuse `reader::tail_lines` for the inbox.** `src/journal/reader.rs:140` is already a
  byte-offset tail that handles a missing file, a shrunk file, concurrent growth, non-UTF-8
  fragments, and an oversize line without a newline (stepping the cursor over it so later events
  are not invisible forever). The inbox is the same problem. **Do not write a second tailer.**
  The driver holds its own `TailCursor` for the inbox in its run state.

- **D-05:** **`inbox.jsonl` is append-only, one JSON object per line, written with the atomic-append
  discipline the journal writer already uses, and it carries a client-generated id.** Minimum
  shape: `{"id": "<uuid>", "ts": "<rfc3339>", "text": "<verbatim>"}`. The id exists because
  **text alone is not a correlation key** — a user may legitimately send the same sentence twice,
  and STEER-02's three states have to be tracked per message, not per string. The id is generated
  by the TUI at queue time so the queued state is addressable before any other process has seen
  the line.

- **D-06:** **The TUI's inbox append is the durability boundary and must be a real one.** The
  write is `OpenOptions::append` + `write_all` of one `\n`-terminated line + **`sync_data()`**,
  performed on `spawn_blocking`, and the "queued" status is not shown until the write returns
  `Ok`. STEER-03's criterion is *"a message queued moments before the TUI restarts is still
  delivered"* — a buffered write that dies with the process satisfies the UI and fails the
  criterion. The acceptance test kills the TUI-side process between the append and any driver
  read, then asserts delivery.

### The three states, and who is allowed to observe each (STEER-02)

- **D-07:** **[LOCKED-BY-SPIKE — Phase 15 D-31] The three states are `queued`, `delivered`,
  `acted-on`, and each has exactly one authoritative observer.** The distinction is not
  cosmetic; it is what the measured protocol actually supports:

  | State | Means | Evidence | Observed by |
  |---|---|---|---|
  | **queued** | Durably on disk; nothing has read it | the line exists in `inbox.jsonl` and no journal record references its id | the TUI, from its own write |
  | **delivered** | Written to the agent's stdin without error | `Executor::send` returned `Ok`; driver journals it | the **driver** |
  | **acted-on** | The agent **dequeued** it and is running it as its own turn | a `user` envelope with `isReplay: true` whose text matches | the **driver** |

  **`isReplay: true` is emitted at DEQUEUE, not at receipt.** Measured: a message written at
  t=12s was echoed at t=68s — 45 ms after the *previous* turn's `result`. It is a
  "started processing" ack. **It must never be rendered as "received", "read", or "acknowledged".**
  The word on screen for that state is "acted-on" (or equivalent); "delivered" belongs to the
  stdin write and nothing else.

- **D-08:** **The driver, not the TUI, performs the echo correlation — because only the driver has
  parsed envelopes.** The TUI reads `journal.jsonl`, whose `ExecEvent.text` is a projection, and
  reconstructing protocol semantics from a rendered string is exactly the screen-scraping D-01
  forbids in another guise. The driver holds `{id → text}` for every message it has delivered,
  watches its own `ExecutionEvent::Message` stream for a `User` `TurnMessage` with
  `is_replay == true`, matches it, and journals the transition.

- **D-09:** **`JournalEvent::Interjected` is the delivery record and needs a correlation id; the
  acted-on transition needs its own journal record.** The reserved schema is
  `Interjected { text: String, delivered: bool }` (`src/journal/mod.rs:382-387`), doc'd
  *"Schema only in this phase — **Phase 18 emits it**"*, with `"interjected"` already in
  `RESERVED_KINDS` (`:543`). Widen it with `#[serde(default)] id: Option<String>` and add a
  **second variant** for the acted-on edge. Both are schema-safe by construction: `src/journal/`
  has no `deny_unknown_fields` anywhere (guarded mechanically at `src/journal/mod.rs:34-38` and
  `reader.rs:7-13`), `JournalRecord.kind` is a plain `String`, and unknown payload survives in the
  flattened `Map` (`reader.rs:226-236`). Remember to extend `EMITTED_KINDS` (`mod.rs:523-533`) —
  a new emitted kind that is not listed there will trip the phase's own guard test.
  `Interjected::is_content()` is already `true` (`mod.rs:454`), so interjections correctly count
  against `MAX_RUN_JOURNAL_BYTES` while lifecycle events do not; keep that property.

- **D-10:** **There is an honest fourth state and it must be visible: `missed`.** A message
  appended after the driver has closed stdin (D-11) cannot be delivered — ever. The correct
  behaviour is to journal it as undeliverable with the reason, and render it as such. Silently
  leaving it in `queued` forever is the failure PITFALLS Pitfall 11 names (undelivered
  injection), dressed up as a spinner. The driver performs **one final inbox drain immediately
  before closing stdin** so the common race resolves in the user's favour; anything after that
  is `missed`, named, and not retried.

### Run lifecycle: when stdin closes (the change that makes steering possible)

- **D-11:** **Stdin stays open for the life of a steerable run and closes at a turn boundary with
  an empty inbox.** Today `run.rs:605-618` closes it immediately after spawn — one command, one
  message, EOF. That is correct for Phase 17 and fatal for Phase 18. New rule, and it is the
  minimal change that works:

  1. Do **not** close stdin after spawn.
  2. On each `ExecutionEvent::TurnCompleted` (a `result` — **a TURN boundary, not a run
     terminator**, Phase 15 D-29), drain the inbox one final time.
  3. If a message was drained, deliver it — the agent runs it as a new turn and the loop repeats
     from step 2.
  4. If the inbox is empty, `close_input()`. The agent drains any queued turns, finishes, and
     **exits 0**. Closing stdin does not kill the run (D-29).

  This shape supports N human-steered turns for free, terminates naturally, and needs no new
  bound. The existing `idle_cap` (15 min) and `wall_clock_cap` (4 h) in `ExecutionOptions`
  (`src/executor/mod.rs:278-332`) remain the backstop for an agent that goes quiet with stdin
  open — **verify that a run parked at step 4 for the idle cap is reported `Stalled`, not
  `Succeeded`**, because the failure mode of getting this wrong is a run that hangs for 15 minutes
  looking healthy.

- **D-12:** **A run has N turns and one outcome, and the UI must not conflate them.** Phase 15
  D-29 is load-bearing here: a steered run emits multiple `system/init` + `result` pairs in one
  process. `total_cost_usd` is **cumulative**; `num_turns` and `duration_ms` are **per-turn and
  reset**. The Driver tab's cost line reads the cumulative value; any per-turn figure it shows
  must be labelled per-turn. A later `system/init` is informational and must not be rendered as
  "the run restarted" (D-30).

### Status derivation — the Replit rule

- **D-13:** **[LOCKED-BY-RESEARCH][USER-FACING SAFETY] Run status is derived from exit codes, disk
  state, and git. NEVER from the agent's prose summary.** FEATURES.md names this as an
  anti-feature with the incident attached: Replit's agent deleted a production database during an
  explicit freeze, **hid it, fabricated ~4000 fake users and fake test results, and falsely
  claimed rollback was impossible.** Self-report is testimony, not telemetry. Concretely, for this
  phase's rendering:
  - The terminal state comes from `RunRecord.outcome` / `JournalEvent::RunEnded.outcome` /
    `ExecFinished.exit`, all of which Phase 15's four-source derivation already computes from
    `is_error`, `terminal_reason`, `permission_denials[]`, exit code, and a git/disk snapshot.
  - `ResultMessage.result` — the agent's prose — **may be displayed as content in the output
    pane, and may never drive a badge, a colour, a status word, or a sort key.**
  - A plan that computes "did it succeed" by inspecting text has failed this decision. State it
    in the doc comment on the rendering function so the next reader cannot undo it by accident.

- **D-14:** **"Parked / needs a human" is a predicate over evidence that exists today, not a
  fabricated state.** Phase 20 owns the `Parked` *producer*. Phase 18 ships
  `fn needs_human(...) -> bool` fed by: `state.paused` (HANDOFF present — the existing v1.4
  signal, already badged), `state.external_job_waiting`, a finished run whose outcome is
  `PermissionDenied` / `Failed` / `Stalled` / `TimedOut`, and — forward-compatibly — a
  `JournalEvent::Parked` record if one is ever present. That set is honest today and grows by one
  match arm when Phase 20 lands. **Do not invent a parked flag that nothing sets**; a badge that
  can never light is worse than no badge because it teaches the user to ignore it.

### The UI surface

- **D-15:** **`DetailSubView::Driver` is the 11th tab, index 10, and it is reached by `Left`/`Right`
  and by `Shift+D`.** All ten digits are taken (`detail.rs:1001-1011`; `0` is Docs). Uppercase is
  entirely unclaimed in the detail view, `d` lowercase is taken twice already (Defaults toggle
  `:1512`, Queue delete `:1630`), and `KeyCode::Char('D')` arrives without needing the
  `_modifiers` parameter the handler currently ignores. Tab label `"D:Drive"`. **Both render
  dispatch matches must be updated** — `detail.rs:1897-1908` and the duplicate at
  `detail.rs:3199-3210` (used by `EnqueueScreen` to paint the body behind its footer). Also
  `TAB_TITLES` (`:41-52`, an array with a literal length), `tab_index` (`:82-95`),
  `sub_view_from_index` (`:97-112`), `switch_to_tab` (`:215-341`), and `footer_spans`
  (`:3772-3855`). Missing one of those six sites is the classic way this lands half-done.

- **D-16:** **The Driver tab is two panes: a run list and a run detail, on the 40/60 horizontal
  split `render_pipeline_tab` already uses (`detail.rs:2499-2637`).** Left: the runs under
  `meta-manager/runs/`, newest first — `new_run_id`'s format makes lexicographic order
  chronological order (`journal/mod.rs:192`) and that is load-bearing, so sort lexicographically
  and say why. Right, for the selected run, top to bottom: **goal (verbatim, OBS-03)** and command
  and started/elapsed; the **D-R-P-E-V pipeline line**; the step timeline; the live output pane.
  This is what makes OBS-05 fall out of OBS-04 — a finished run is the same view over a journal
  that has stopped growing.

- **D-17:** **[LOCKED-BY-RESEARCH][ANTI-FEATURE] Reuse the existing D-R-P-E-V pipeline widget as
  the step/progress display. A second progress display is a named anti-feature.** The functions
  are already free functions over `&DiskInference` and need no lifting:
  `derive_all_stage_statuses` (`detail.rs:3411`), `build_pipeline_line` (`:3471`),
  `build_stage_detail_lines` (`:3503`), `stage_color` (`:3461`). ARCHITECTURE M5 suggests lifting
  them into `state_reader/` — **decline that for this phase**: the driver process does not render,
  Phase 20's `decide()` is the first genuine second consumer, and moving 130 lines across modules
  now buys nothing and churns the largest file in the repo. The step *timeline* (one row per GSD
  command issued: command, start, duration, terminal state, cost) is the one genuinely new widget,
  it is additive on top, and **in this phase it has exactly one decided row** (D-12 turns are
  shown as turns, not as commands).

- **D-18:** **Live output is a bounded ring buffer — the first in this repo — and it lives on
  `AppContext`, never on `ProjectState`.** [LOCKED-BY-RESEARCH — ARCHITECTURE AP1]
  `ProjectState` derives `PartialEq` and `app.rs` uses that equality to suppress "Updated:
  {alias}" status spam, a deliberate v1.4 feature that live driver state would defeat for an
  entire multi-hour run. `run_states`, `journal_cursors`, `observed_runs` and
  `session_spawned_runs` are the established precedent (`src/ui/screens/mod.rs:138-226`).
  Split: **the buffer** goes in a sibling map on `AppContext` keyed by alias (and **must** be
  pruned in `prune_driver_maps`); **the view state** — scroll offset, the auto-follow bit, the
  selected run index — goes on `ProjectViewCache` (`screens/mod.rs:60-110`, `#[derive(Default)]`,
  so it is additive with no constructor churn). Cap it with a named `pub const` carrying a doc
  that says the number is an untuned starting point, following the `main_loop.rs:41-49` idiom.
  `grep -rn VecDeque src/` returns nothing today; this is greenfield.

- **D-19:** **The output pane auto-follows the tail unless the user has scrolled up.** That
  follow bit does not exist anywhere in the codebase and is the one behaviour a
  `Paragraph::scroll` viewer does not give for free. Model the viewer on the Browse
  `BrowserDepth::View` file viewer (`detail.rs:3054-3090`) — line count → gutter/text split →
  `Cell<ViewportMetrics>` capture → `Paragraph::new(lines).scroll((offset, 0))` — and **preserve
  the clamp ordering invariant** the repo repeats in four comments (`detail.rs:889, 946, 975,
  992`): PageDown adds *then* clamps; PageUp/Up clamps *first* then subtracts. It was a shipped
  bug (UIFIX-04) and the tests at `detail.rs:5046-5204` exist to keep it fixed.

- **D-20:** **The `Action::DriverJournalAppended` handler is where the surface and the redraw flag
  land together.** `src/app.rs:705-739` currently stores the cursor, checks `seq` gaps, and
  **drops `records` on the floor** — deliberately, with the comment at `:697-701` saying *"Phase 18
  is what adds the surface and the flag together."* This phase appends the records into the ring
  buffer and sets `needs_redraw`. It is the seam; it is not a bug to be discovered.

- **D-21:** **Elapsed time redraws on the existing 250 ms tick, and only when it can be seen.**
  `RunRecord.started_at` is RFC3339; elapsed is `now − started_at`. Set `needs_redraw` from the
  `Action::Tick` arm **only** when the top screen is a `DetailScreen` whose active sub-view is
  `Driver` **and** the selected project has a live observed run. An unconditional per-tick redraw
  makes an idle fleet dashboard burn CPU forever, which is the exact opposite of this tool's
  pitch. Do not add a second timer — `app.rs:464-476` forbids it in as many words.

- **D-22:** **Injection input is a pushed screen modelled on `EnqueueScreen`, opened with `i` from
  the Driver tab.** `src/ui/screens/enqueue.rs` (120 lines) is the exact idiom: a `Screen` that
  delegates its body to `DetailScreen::render_main_only` and paints its own one-line input footer
  over it, using the shared `ctx.input_buffer`, with `Enter` = commit + `Pop`, `Esc` = clear +
  `Pop`, `Backspace`/`Char` for editing. `i` is free in the detail view. **No cursor movement, no
  `tui-textarea`** — none of the four existing text inputs has a movable cursor (they fake a caret
  with a trailing `"_"` span), and introducing a fifth, different input model for one field is
  churn. The write itself is dispatched as an `Action` and performed on `spawn_blocking` (D-06);
  the key handler never touches the filesystem.

- **D-23:** **Start flow: a command picker plus an optional goal, replacing the single constant.**
  `driver_confirm::DEFAULT_DRIVE_COMMAND` (`:48`) stays as the default *selection*, not as the
  only value. Seed the picker from `queue_md::suggest_next_commands(state)` — it already exists and
  already backs `EnqueueScreen`'s Tab-completion, so the "what should I run next" logic is not
  re-derived. The goal is a free-text field, **stored verbatim, never paraphrased** (FEATURES.md
  table stakes), and it flows through the `goal: Option<&str>` parameter `App::start_driver_run`
  already accepts (`src/app.rs:903`) into `RunRecord.goal`, which `ObservedRun.goal`
  (`reconcile.rs:81-111`) already surfaces. **OBS-03 needs no new plumbing below the UI — only a
  screen to type into.**

- **D-24:** **OBS-02 badges extend `alias_badge`, and the "at most one badge" rule holds.**
  `src/ui/screens/normal.rs:62-79` returns `Option<(&'static str, Color)>` with a documented
  priority (pause > external-job-waiting > active session) that exists to keep the Alias column
  aligned. Glyphs are fixed `&'static str` and **never derived from file content** — a deliberate
  guard so no HANDOFF body can leak onto a dashboard row; keep it. New order, with the reason:
  **driven-and-live outranks everything** (the user must never be unsure whether something is
  driving their repo — FEATURES.md), then parked/needs-human, then the existing three. Two new
  glyphs, distinct at a glance and distinct in colour from the existing ⏸/⏳/▶.

- **D-25:** **OBS-07 ships as a filter suffix plus one sort toggle — no new screen.**
  `parse_filter` (`src/app.rs:30-56`) already has the `/n`, `/p`, `/s` suffix grammar and
  `FilterColumn`; add a variant and an arm in `recompute_filtered_aliases`
  (`screens/mod.rs:233-272`) for "needs a human" (D-14's predicate). `sorted_aliases`
  (`screens/mod.rs:218-222`) is hardcoded `sort_by_key(to_lowercase)`; add a sort **mode** on
  `AppContext` with alphabetical as the default and "attention first" as the alternative, toggled
  by `s` (free on the dashboard). Keeping the alphabetical default matters: a dashboard whose row
  order changes under the cursor while a run progresses is a usability regression that a demo
  will not catch.

- **D-26:** **Surfacing the dry-run report is TRANS-05-adjacent and cheap, so do it — from the
  Driver tab, not as a new screen.** Phase 17 D-24 explicitly left "the TUI surfacing dry-run
  output" to this phase. `dry_run::build_report` + `render` already produce the exact pinned text
  (`src/driver/dry_run.rs:97,111`); run it on `spawn_blocking` (**this is one of the WR-10 call
  sites**) and show it in the run-detail pane before a start is confirmed. If the planner finds
  this competing for space with the nine requirements, **it is the first thing to cut** — it is
  the only item here with no requirement id.

### The four MUST-FIX carry-ins

- **D-27 (WR-02):** **[MUST-FIX-CARRY-IN] Path traversal — reproduced, not theorised.** `run_paths`
  (`src/journal/mod.rs:168`) does `runs_root(planning).join(run_id)` with no component check.
  Both directions are reachable: **write side**, `--run-id '../../../../escaped'` created
  `run.json` and `journal.jsonl` outside the project in a directory with **no `.gitignore`**
  (exit 0, reproduced in `17-REVIEW.md`); **read side**, `writer::read_active_run` returns
  whatever the `active` file holds, guarded only by an `is_dir()` check that a traversing path
  satisfies — and the `active` file lives inside the driven project, so **the agent controls it**.
  `reconcile_one` then reads `run.json` from anywhere and `App::schedule_journal_tail` tails
  anything.
  **This subsystem runs unattended with git and push rights. It is non-negotiable.**
  Fix, and the shape is chosen deliberately: **make `run_paths` fallible** (`-> Option<RunPaths>`)
  and reject any id that is not exactly one `Component::Normal`, so **the compiler enumerates
  every caller** rather than a human doing it — `JournalRun::start`, `writer::read_active_run`,
  `reconcile::reconcile_one`, `App::schedule_journal_tail`, and whatever this phase adds for
  `inbox.jsonl`. `journal::classify_change` (`mod.rs:263`) already gets this exactly right and is
  the in-repo model. Also refuse a bad `--run-id` at the CLI seam with a typed error, so the
  common case fails loudly instead of quietly yielding `None`.
  **Regression tests (both directions, both required):** a malicious `--run-id` creates nothing
  outside `.planning/meta-manager/runs/` and exits non-zero; an `active` file containing a
  traversing id causes zero reads outside the runs root.

- **D-28 (WR-10):** **[MUST-FIX-CARRY-IN] Blocking calls inside `async fn` — this deadlock has
  already been observed in this repo.** `tests/driver_lock.rs:201-215` records it: a blocking
  `flock` inside an `async fn` defeated `tokio::time::timeout` on a current-thread runtime.
  The comments currently protecting the callers are not a mechanism, and **Phase 18 is the
  TUI-side caller the review predicted** — where the failure mode is a frozen frame, not an error.
  Put every blocking section behind `tokio::task::spawn_blocking`, the boundary the codebase
  already uses for exactly this (`App::schedule_reparse`, `claude.rs::capture_snapshot`):
  `dry_run::build_report` (two sync `git` shell-outs, `driver/dry_run.rs:97-103`),
  `lock::acquire` (`flock` + file writes, `driver/mod.rs:124-156`), `JournalRun::start`
  (prunes directories, writes files, `driver/run.rs:296-316`), **and every new path this phase
  adds** — the TUI's inbox append, the driver's inbox tail, and the run-list directory scan.
  `RunLock` holds a `File` whose descriptor **is** the lock (`driver/lock.rs:112-122`, no `Drop`
  impl) — it must be moved back out of the blocking task and held for the run's duration, never
  dropped inside it.

- **D-29 (WR-15):** **[MUST-FIX-CARRY-IN] `DriverStopped` must not drop a run that was never
  stopped.** `src/app.rs:797-807` unconditionally removes from `observed_runs` and
  `session_spawned_runs`, but `StopOutcome::SignalFailed` means the signal was never delivered and
  `AlreadyGone` means nothing was signalled — in both cases the run may still be live. The
  dashboard then shows no run for up to five seconds, and the `session_spawned_runs` entry is gone
  **permanently**, so a later stop takes the `Adopted` arm for a run this session did spawn. Carry
  the outcome as a value: `StopOutcome` is `#[cfg(unix)]` and `Action` is not, so add a small
  portable enum (or a `stopped: bool` plus the existing text) in `action.rs` and mutate the maps
  only when the run is actually gone. **This one is directly load-bearing for Phase 18: the
  Driver tab is what makes the contradiction visible** — a "no run" pane next to a status line
  saying the signal failed.

- **D-30 (WR-16):** **[MUST-FIX-CARRY-IN] The hidden agent-override flags must not ship in
  release, and a stand-in run must be marked on disk.** `--claude-program` (`src/cli.rs:72-91`)
  lets any caller of the released binary make the "driver" exec an arbitrary program in an
  opted-in project root, while the journal records it as a normal GSD run with nothing
  distinguishing it. `hide = true` removes it from `--help`, not from the parser. Do **both**
  remedies the review offered, because each covers the other's gap:
  (a) gate the fields behind `#[cfg(debug_assertions)]`, and
  (b) when the override is used, journal `Diagnostic { code: "agent_program_overridden", detail }`
  so a stand-in run is never mistakable for a real one on disk.
  **Known consequence, accepted and recorded: `cargo test --release` will no longer build the
  integration tests that pass those flags.** The project gate does not run release tests, the
  nine fixtures under `tests/fixtures/fake-claude*.sh` are debug-only test infrastructure by
  nature, and shipping a remote-code-execution-shaped flag in the released binary to keep a
  release-mode test path is the wrong trade. Guard the cfg mechanically with the grep-guard idiom
  the repo already uses (`tests/spawn_seam_guard.rs`), because a `#[cfg]` that a later refactor
  quietly widens is indistinguishable from never having added it.

### Claude's Discretion

- Plan granularity and wave structure. (Non-binding shape: the four carry-ins are independent of
  each other and of the UI, so they parallelise; the stdin-lifetime change gates injection; the
  Driver tab gates the injection input screen.)
- Module layout for the new UI code. ARCHITECTURE N13 proposes `src/ui/screens/driver.rs`; note
  that every existing sub-tab renders from inside `detail.rs`, which is already 5206 lines. A
  separate module for the Driver tab's render/key logic with `detail.rs` delegating to it is
  likely better, but it is a judgement call — decide and record it.
- Exact `inbox.jsonl` line schema beyond the three fields in D-05.
- The exact journal variant/field shape for the acted-on transition (D-09).
- The ring-buffer cap, the poll interval for the driver's inbox tail (order 500 ms–1 s: the unit
  of work is minutes, per ARCHITECTURE §5.5), and whether the driver polls or uses `notify`.
  **Prefer polling** — the driver has no watcher today and adding `notify` to the detached process
  to save half a second is not a trade this phase needs to make.
- The two new badge glyphs and their colours, within the existing convention (Green active,
  Yellow waiting, Red blocked, DarkGray inert, Cyan selection, Magenta unknown/workstream).
- Whether the cumulative cost line (`JournalEvent::Cost`, already journaled) appears in the run
  header. It is nearly free and FEATURES.md lists it as table stakes; include it if it costs
  nothing.
- Exact test names and file placement, following the `#[cfg(test)] mod tests` convention for
  logic and `tests/*.rs` only where a real process is required.

</decisions>

<code_context>
## Existing Code Insights

Every anchor below was read directly during this context pass against the current tree.

**The two facts that dominate the design**

- **The TUI holds no handle on the run.** `ExecutionHandle::stdin_tx` (`src/executor/mod.rs:410`)
  exists only inside the detached driver (`src/driver/run.rs:580`). The TUI's only channels are
  SIGTERM (`driver::kill::stop_run`) and reading `journal.jsonl`. **There is no TUI→driver message
  path at all today.** The seams are reserved and empty: `JournalEvent::Interjected`
  (`journal/mod.rs:382-387`, *"Phase 18 emits it"*), `RESERVED_KINDS` includes `"interjected"`
  (`:543`), and `tests/journal_gitignore.rs:142,194` already assert `<run-dir>/inbox.jsonl` is
  gitignored. `RunPaths` (`journal/mod.rs:135-146`) has no `inbox` field yet.
- **The driver closes stdin immediately after spawn** — `run.rs:605-618`,
  *"One command means one message, so signal end-of-input immediately."* `WriterCommand::Close`
  breaks the writer loop at `claude.rs:832` and drops the handle. **Injection is impossible until
  that call becomes conditional (D-11).**

**The duplex control channel (Phase 15) — complete, tested, and unused by any UI**
- `Executor` trait `src/executor/mod.rs:71-108`: `start` `:75`, `send` `:88`, `interrupt` `:95`,
  `cancel` `:101`, `is_running` `:104`, `capabilities` `:107`. Doc at `:82-87` states the
  no-flush-buffer rule (D-02).
- `ClaudeExecutor::send` `claude.rs:547-556` = `serde_json::to_string(&message)` → `write_line`.
  Its doc `:522-546` is the Phase-18 specification for STEER-02 and names the measured 55 s
  dequeue delay.
- `ExecutionHandle::write_line` `mod.rs:425` (`SendError::NotRunning` / `WriterGone`),
  `close_input` `:446`, `wait_outcome` `:454` (idempotent/cached).
- `UserMessage::text(..)` `stream_json.rs:249`; wire shape pinned by the test at `:716-723`.
- `interrupt` `claude.rs:576-618` correlates on `request_id` through
  `PendingControl = Arc<Mutex<HashMap<String, oneshot::Sender<ControlResponse>>>>` (`mod.rs:55`),
  bounded by `control_response_cap` (30 s default).
- **`interrupt_stopped_a_turn(&[TurnOutcome]) -> bool` `claude.rs:203`** — *the only honest answer
  to "did my interrupt work?"*, matching `terminal_reason == "aborted_streaming"`. If this phase
  exposes interrupt at all, it must report from this, never from the ack
  (`InterruptAck` doc, `mod.rs:679-694`: `subtype == "success"` means *accepted*, not *acted on*).
- `is_replay` is **modelled but never interpreted**: `stream_json.rs:121-122`
  (`#[serde(default, rename = "isReplay")]`), semantics doc `:113-120` and `claude.rs:533-541`,
  argv flag `--replay-user-messages` at `claude.rs:238`. **No delivery-tracking state machine
  exists. That is this phase's greenfield.**
- `handle_item` `claude.rs:1388-1537`: the `result` arm `:1491-1520` carries
  *"Deliberately no `break` here: `result` closes a TURN, not the run."* A later `system/init` is
  informational `:1473-1489`.
- `TurnMessage` `stream_json.rs:109-129` deliberately does **not** model the message body:
  `:105-107` calls content blocks *"Phase 18's rendering concern"*.

**The journal — the only data the TUI can see**
- `RunPaths { dir, run_json, journal, gitignore, active }` `journal/mod.rs:135-146`;
  `run_paths(planning_dir, run_id)` `:168` ← **WR-02's site**; `runs_root` `:149`.
- `new_run_id(now, session_uuid)` `:192` → `2026-07-28T14-03-11Z-a3f9`; **lexicographic order ==
  chronological order** and D-16 depends on it.
- `JournalEvent` `:306-445` (`#[serde(tag="kind", rename_all="snake_case")]`) — the variants this
  phase renders: `RunStarted{goal,dry_run,target}` `:310`, `ExecStarted{session_id,argv_digest,
  claude_pgid}` `:340`, `ExecEvent{stream,text}` `:364`, `Cost{cumulative_usd}` `:375`,
  `Interjected{text,delivered}` `:382`, `ExecFinished{exit,cost_usd,duration_s}` `:389`,
  `EventsDropped{count}` `:404`, `JournalTruncated{..}` `:414`, `Diagnostic{code,detail}` `:423`,
  `RunEnded{outcome,ended_at}` `:430`, `Parked{reason,needs}` `:439`.
  `is_content()` `:454` is true for `ExecEvent | Interjected` only.
- `EMITTED_KINDS` `:523-533` and `RESERVED_KINDS` `:543` are guarded by tests — a new emitted kind
  must be listed.
- `from_exec_event` `:812-867`: `Message(m)` → `text: format!("{m:?}")` `:822-825`;
  `TurnCompleted` → `format!("{result:?}")` `:846-849`. **This is the live-output content today,
  and it is a Rust `Debug` rendering.**
- `JournalRun::record(&JournalEvent)` `:731` is the append escape hatch this phase uses for
  interjection events. `JournalRun::start` `:608`, `finish` `:661` (writes `RunEnded` **before**
  the record write, then clears `active`, `debug_assert_eq!(record_writes, 2)`).
- `RUNS_GITIGNORE_BODY` `writer.rs:237-244` — `*`, `!*/`, `!.gitignore`, `!*/run.json`. **Any new
  per-run file is ignored by default**, which is why `inbox.jsonl` needs no gitignore change.
- `writer::read_active_run` `:456-472` — WR-02's read-side site; guarded only by `is_dir()`.
- `reader::tail_lines(path, cursor)` `:140` and `TailRead { lines, cursor, restarted,
  skipped_oversize }` `:101-133` — reuse for the inbox (D-04). `JournalCursor { cursor, last_seq }`
  `:90-97`; `seq_gaps_from` `:321`; `read_all(path)` `:351` for a finished run.

**The driver process**
- `driver::drive(args, config)` `mod.rs:171-220` → `dispatch` `:224-230` → `run::execute_run`
  `run.rs:449`. `DriveArgs` `mod.rs:87-116`, `command: String` — **exactly one, not a Vec** `:91-96`.
- `execute_run` order: SIGTERM handler first `:474-481`; `establish_own_group()` `:483`;
  `lock::acquire` `:529`; `JournalRun::start` `:531`; biased select around `executor.start`
  `:566-578`; `set_claude_pgid` `:595`; **`close_input()` `:605-618`**; drain loop `:632-658`
  (`biased`, terminate first, then `handle.events.recv()` → `journal.record_exec`);
  `wait_outcome` `:660`; `journal.finish` `:661`. **The inbox arm goes in that drain loop.**
- `ObservedRun { alias, run_id, pid, pgid, started_at, goal, gsd_command, liveness }`
  `reconcile.rs:81-111`; `verdict()` `:118`, `is_live()` `:132`. **`ObservedRun.live: bool` is
  GONE — use `liveness: Liveness` / `is_live()`.** `reconcile_all` `:307` sorts by alias so the
  handler can equality-guard the redraw.
- `Liveness { Alive, Dead, Unknown }` `liveness.rs:73-83`; `probe` `:167` is the safety-decision
  surface; `LIVENESS_SUPPORTED` `:56`.
- `StopOutcome { ExitedOnTerminate, ExitedAfterKill, AlreadyGone, SignalFailed{detail} }`
  `kill.rs:117-140` — WR-15's payload. `stop_run` `:315`; **never call on the render thread**
  (`:312-314`).
- `spawn::drive_argv(config_path, alias, command, run_id, goal)` `spawn.rs:58` — **`config_path`
  is FIRST** (Phase 17 CR-03) and `--config` precedes the subcommand. `--run-id` is mandatory for
  a real `drive` (`mod.rs:215`, `RunIdRequired`). `admit(live, max)` `:199` is the concurrency gate.
- `dry_run::build_report(project, command)` `dry_run.rs:97`, `render` `:111`, pinned section
  headers `:46,:51,:63`.

**The TUI**
- `AppContext` `src/ui/screens/mod.rs:112-216` — the sibling-map invariant is repeated in four
  doc comments (`:127-135, :146-171, :172-192, :193-208`). `Action` derives `Clone`, so **no file
  handle or `JoinHandle` may enter any of these maps** (D-20 of Phase 16).
- `Screen` trait `:29-38`; `ScreenAction` `:40-51`; screens are `Box<dyn Screen>` on
  `App::screen_stack` (`app.rs:110`), only the top is rendered (`ui/mod.rs:7-12`).
- `ProjectViewCache` `:60-110`, `#[derive(Default)]` — additive.
- `App::schedule_journal_tail` `app.rs:282-367` → `Action::DriverJournalAppended`;
  handler `:705-739` **drops `records`** and deliberately does not set `needs_redraw` (`:697-701`).
- `Action::Tick` 20-tick block `:445-492` — sessions, `reconcile_all`, `prune_driver_maps` `:490`.
  `prune_driver_maps` `:841-875`. **Comment at `:464-476` forbids a second interval.**
- `App::start_driver_run` `:903-980` (`goal` threaded, passed `None`);
  `App::stop_driver_run` `:1005-1048`; `Action::DriverStopped` handler `:797-807` ← **WR-15**.
- `driver_confirm.rs:1-24` is the explicit Phase 17/18 scope fence and **names every Phase 18
  deliverable by hand.** `DEFAULT_DRIVE_COMMAND` `:48`; `DriverAction {Start,Stop,ToggleOptIn}`
  `:51-59`; `do_start_run` `:186-201` (the *affordance*; the real gate is
  `DrivableProject::from_registry`).
- `normal.rs:62-79` `alias_badge` — at most one badge, fixed glyphs, documented priority.
  Dashboard keys `:186-316`; **free on the dashboard: `b e f g h i l m n p s t u v w y z` and all
  uppercase.** Free in the detail view: `b c f h i l m s t u v w y z` and all uppercase but
  `J`/`K`.
- `detail.rs` sites for an 11th tab, all six of them: `TAB_TITLES` `:41-52`, `tab_index` `:82-95`,
  `sub_view_from_index` `:97-112`, `switch_to_tab` `:215-341`, render dispatch `:1897-1908`
  **and its duplicate** `:3199-3210`, `footer_spans` `:3772-3855`.
- `enqueue.rs` (120 lines) is the injection-input model: `render_main_only` behind a one-line
  input footer, `ctx.input_buffer`, Enter/Esc/Backspace/Char, `Tab` cycling
  `queue_md::suggest_next_commands`.
- **`src/ui/project_list.rs` is DEAD CODE** — `src/ui/mod.rs` declares only `roadmap_widget` and
  `screens`. ARCHITECTURE M6 tells you to put the badge there. **Do not.** The live dashboard is
  `normal.rs`.
- **There is no theme module** — colours are `ratatui::style::Color` literals with a consistent
  convention, and three duplicate `status_color` implementations already exist. Do not add a
  fourth; do not start a theme refactor in this phase either.
- **No ring buffer exists**: `grep -rn VecDeque src/` returns zero hits.
- **There are no UI/render integration tests** — all UI tests are `#[cfg(test)] mod tests` inline
  (`detail.rs:4670-5206`, `normal.rs:~730-1102`, `driver_confirm.rs:299-523`) asserting at the
  logic level, with `fn press(screen, ctx, code)` at `detail.rs:5041` as the helper.
  `driver_confirm.rs:310-370` `fn ctx_with_project` is the canonical full-field `AppContext`
  fixture — **adding an `AppContext` field breaks it and `detail.rs:4943` `fn test_ctx`.**

**Golden fixtures that already contain the exact data this phase needs**
- `tests/fixtures/transcripts/05-queued-injection-two-turns.ndjson` and `08-*` — two-turn queued
  injection captures, i.e. the replay-echo timing D-07 depends on.
- `06-*`/`07-*` — interrupt captures. Nine `tests/fixtures/fake-claude*.sh` stand-ins.
- `tests/executor_transport.rs:364` already asserts a mid-turn message is **not** buffered; `:321`
  pins the `send` wire shape; `:433` covers interrupt correlation and `still_queued`.

**Dependencies, toolchain, gates**
- `Cargo.toml`: version 1.6.0, edition 2021, **`rust-version = "1.87"`**. `tokio` with `full`,
  `process-wrap 9.1.0`, `rustix 1.1`, `uuid 1.24`, `tempfile 3`, `chrono 0.4`. Dev-deps:
  `assert_fs` only. **This phase should need no new dependency** — `VecDeque` is std.
- `cargo build && cargo test && cargo clippy -- -D warnings` must pass.
  `cargo clippy --all-targets` has **exactly 5 pre-existing lints** (3× `browser.rs`,
  1× `project_creator.rs`, 1× `state_reader/mod.rs`). **The count must not grow.**
- **Baseline at phase start: 542 tests passing**, measured this pass
  (451 lib + 91 across 14 integration binaries).
- **`rtk` filters cargo output**: raw `warning:` and `test result:` lines are **absent** from
  ordinary `cargo` invocations, so any acceptance criterion that greps for them passes
  **vacuously**. Use `rtk proxy cargo …` wherever a criterion needs raw output. This bit Phase 15
  and Phase 17.

</code_context>

<specifics>
## Specific Ideas

### Success criteria, and how each is reachable without breaking the scope fence

Phase 15 had two criteria that could only be met by out-of-scope work and had to be rewritten
mid-phase. Each of this phase's five was checked against its own fence before this doc was
written:

1. *"A user picks a GSD command for an opted-in project, runs it from the TUI, and watches its
   output, current step, and elapsed time stream live."* Reachable with the fake-`claude` fixtures
   — no subscription needed. **Watch:** "current step" is the D-R-P-E-V pipeline read from disk
   (D-17), not a second progress model; "output" requires D-11's readable text projection or the
   pane renders `Debug` structs and the criterion is met only in the letter.
2. *"The project list marks driven and parked projects distinctly, and can be sorted or filtered
   down to the ones waiting on a human."* Reachable. **Watch:** nothing emits `Parked` in this
   phase — D-14's evidence predicate is what makes the badge honest. A badge wired to a field
   nothing sets satisfies a screenshot and fails a user.
3. *"The originating goal prompt is readable for any driven project, and a finished or failed run
   can be reviewed afterwards — which commands ran and why it stopped."* Reachable: `RunRecord.goal`
   and `ObservedRun.goal` already exist and `run.json` is the one committed artifact; review reads
   the journal with `reader::read_all`. **Watch:** the goal is only non-empty once D-23 gives the
   user a place to type it; and "why it stopped" must come from `RunEnded.outcome`, never from the
   agent's prose (D-13).
4. *"A message typed into a running driver shows queued → delivered → acted-on, never a bare
   'sent'."* Reachable **only because** `--replay-user-messages` is already in the argv baseline
   (`claude.rs:238`) and the echo is already modelled (`stream_json.rs:121`). **Watch:** the
   acted-on state must come from the echo observed by the driver (D-08), and must not be rendered
   as "received" (D-07). **Watch harder:** all three states are unreachable while `close_input()`
   runs at spawn (D-11) — a plan that skips that change cannot satisfy this criterion no matter
   what it renders.
5. *"A message queued moments before the TUI restarts is still delivered to the run afterwards."*
   Reachable and testable without any TUI: append to `inbox.jsonl` from a test, drop the writer,
   assert the driver delivers. **Watch:** the criterion is about **durability**, so the test must
   prove the message was on disk before any reader existed — `sync_data()` plus an ordering
   assertion, not a sleep.

### Things that will look done but are not

Adapted from PITFALLS' own "Looks Done But Isn't" list, filtered to this phase:

- **Injection:** the TUI writes the line, shows "queued", and nothing ever consumes it. The tell is
  that no `interjected` record appears in `journal.jsonl`. `close_input()` at spawn is the cause,
  and the UI looks perfect while it is broken.
- **Delivery confirmation:** derived from "we wrote to the pipe". The ROADMAP names this as a
  phase risk in as many words. The tell is a `delivered` badge that lights instantly — the real
  dequeue ack was measured at **55 seconds**.
- **Three-state UI:** three labels over two real states, with "delivered" and "acted-on" both set
  from the same event. The tell is that no code reads `is_replay`.
- **Live output:** a pane full of `TurnMessage { session_id: Some(..), is_replay: false, .. }`.
- **Step timeline:** a second progress display next to the D-R-P-E-V widget. Named anti-feature.
- **Parked badge:** wired to `JournalEvent::Parked`, which nothing emits until Phase 20. It will
  never light, and no test will catch it because the test will emit the event by hand.
- **Ring buffer:** a `Vec` that grows. The tell is no cap constant and no test that pushes past it.
- **Run status:** taken from `ResultMessage.result`. The tell is any string comparison against
  agent prose. This is the Replit failure mode and it is the one that costs a database.
- **Reattachment:** a Driver tab that promises live output for an adopted run. Physically
  impossible (Phase 17 D-11).
- **WR-02:** a validation helper that exists but is called at three of the five sites. The tell is
  that `run_paths` still returns `RunPaths` infallibly — making it fallible is what conscripts the
  compiler into finding the fifth.
- **WR-10:** `spawn_blocking` wrapped around the calls the review listed, while the phase's own
  new inbox append and directory scan block the render thread.

### Concrete shapes worth fixing early

The inbox line (D-05), and the journal records that answer it (D-09):

```jsonc
// runs/<run-id>/inbox.jsonl   — written by the TUI, tailed by the driver
{"id":"3f2a…","ts":"2026-07-29T21:40:02Z","text":"skip the UI review"}

// runs/<run-id>/journal.jsonl — written by the driver
{"ts":"…","seq":41,"kind":"interjected","id":"3f2a…","text":"skip the UI review","delivered":true}
{"ts":"…","seq":57,"kind":"<acted-on kind>","id":"3f2a…"}   // ← the isReplay echo, ~55s later
```

The TUI's state for a message is then a pure function of two sets: ids present in `inbox.jsonl`,
and ids present in each journal kind. **That is deliberately reconstructible from disk alone**,
which is what makes STEER-03 hold across a restart with no extra state to persist.

The Driver tab, sketched (the UI-SPEC pass owns the real layout):

```
 1:Phases 2:Roadmap … 0:Docs  D:Drive
┌ runs ────────────┬ 2026-07-29T21-40-02Z-3f2a ───────────────────────────┐
│▸2026-07-29…-3f2a │ goal: ship the driver tab                            │
│ 2026-07-29…-91cc │ cmd:  /gsd:execute-phase 18   started 21:40  +04:12   │
│ 2026-07-28…-a3f9 │ [D]---[R]---[P]---[E 2/3]---[V]        $1.83          │
│                  │ ── steps ────────────────────────────────────────────│
│                  │ /gsd:execute-phase 18   21:40  running   turn 3       │
│                  │ ── output ──────────────────────────── [following] ──│
│                  │ assistant: reading src/driver/run.rs…                 │
│                  │ » skip the UI review          acted-on                │
└──────────────────┴──────────────────────────────────────────────────────┘
  [i] inject  [x] stop  [j/k] scroll  [Esc] back
```

</specifics>

<deferred>
## Deferred Ideas

Each was considered during this pass and deliberately pushed out, with its owner named:

- **The `control.sock` fast path** → a later milestone, per REQUIREMENTS' "Future Requirements"
  and ARCHITECTURE §5.5 ("build the durable path first"). The unit of work is minutes; sub-100 ms
  injection buys nothing here.
- **`decide()`, the D-R-P-E-V router, multi-command sequences, run bounds, quota-floor park,
  no-progress and repeat-command detection, the `Parked` producer** → Phase 20.
- **Lifting `derive_all_stage_statuses` out of `detail.rs` into `state_reader/`** (ARCHITECTURE
  M5) → Phase 20, which is its first genuine second consumer. Declined here as churn (D-17).
- **Approve-next / step-through mode and "confirm and steer"** (FEATURES Category C
  differentiators) → Phase 20+. They are cheap *given* injection, which is exactly why injection
  ships first and they do not ship with it.
- **Exposing `interrupt` from the TUI.** Phase 15 built it and `interrupt_stopped_a_turn` is the
  honest reporter, but STEER-01/02/03 are about *messages*, and an interrupt button whose true
  state can only be known one turn later is a second three-state UI. → Phase 20, alongside the
  router that has a reason to abort a turn.
- **Orphan sweeping via `pgrep -x claude` + `/proc/<pid>/cwd`** → re-homed from Phase 18 to
  Phase 20's before-each-iteration collision check (see the Phase Boundary for the reasoning).
- **Fleet aggregate rows, per-project cost history, the decision log view** → past v2.0.
- **A `notify` watcher inside the detached driver** for the inbox → not needed; polling is
  correct at this cadence and the driver has no watcher today.
- **`tui-textarea` or any richer text input** → declined outright. Four text inputs exist, none
  has a movable cursor, and a fifth model for one field is churn (D-22).
- **A theme/style module and collapsing the three duplicate `status_color` implementations** →
  a quick task; unrelated to this phase and it would touch every screen.
- **Deleting the dead `src/ui/project_list.rs`** → a quick task. Worth doing precisely because
  ARCHITECTURE M6 points a future planner straight at it.
- **Phase 17 warnings not assigned to this phase** — WR-03 (teardown grace vs `capture_snapshot`
  budget), WR-04, WR-05, WR-06, WR-07, WR-08, WR-09, WR-11, WR-12, WR-13, WR-14, WR-17. Each keeps
  the owner `17-08-PLAN.md` assigned it. **Exception: WR-11** (`stop_driver_run` returns silently
  with no event channel) sits in the same handler as WR-15 (D-29); if the fix touches those lines
  anyway, adding the missing `error_message` is welcome but is **not** an acceptance criterion.
- **`session_detector.rs`'s `pts/1` vs `pts/11` TTY substring match**, and the `pending_editor`
  blocking shell-out on the render thread → still quick tasks, still not this phase's.
- **Fixing the 5 pre-existing `--all-targets` clippy lints** → unrelated; the count must not grow
  but is not this phase's to shrink.

</deferred>
