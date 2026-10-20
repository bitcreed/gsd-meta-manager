# Phase 30: Driver Tab on DIRECTION.md - Research

**Researched:** 2026-10-20
**Domain:** Rust TUI (ratatui 0.30 / crossterm 0.29 / tokio): read-only observation of launcher run dirs, locked body-only steering writes, detached launcher lifecycle, watcher routing
**Confidence:** HIGH for the in-repo APIs and seams (every one was opened and read). MEDIUM for the planned decomposition. LOW only where tagged `[ASSUMED]`.
**Mode:** unattended. Inferred decisions are tagged `[inferred — audit]`.

## Summary

Phase 30 is mostly integration work. Phases 27-29 already ship every primitive the tab needs:
- `read_driver_run` / `list_driver_runs` / `DerivedStatus` for status and liveness;
- `driver::direction::write_body` for the locked, body-only write;
- `driverctl::log::parse_log` for LOG events;
- `journal::reader::tail_lines` for the byte-offset tail;
- `driver_templates::{create_run_dir, mint_run_id, lint::lint_direction}` for the new-run flow;
- `launcher::run::launch_detached`, `launcher::stop::stop_run` and `launcher::host::{detect, decide}` for the lifecycle.

New Cargo dependencies: **zero**. The new code is:
- one pure body-transform module (set control / answer / un-hold, plus a body item reader for titles, `hold` and `answer`);
- a selected-run view model with its render;
- a new `ChangeKind` route in the watcher;
- a fleet-wide marker scan that rides the existing 20-tick reconciliation task;
- a set of small input and confirm screens built on the shipped idioms.

The research turned up **six facts that contradict or sharpen the approved artifacts.** The planner must act on each:
1. `write_body` does **not** fail when `DIRECTION.lock` is busy. After its 2 s budget it **proceeds without the lock** and reports `lock_timed_out: true`. The UI-SPEC backstop row expects the opposite ("asserts the error copy and an unchanged body"), so as written it is unimplementable.
2. The launch preflight **refuses** any `control` other than `run`. The UI-SPEC's yellow "will end before its first dispatch" row is therefore a hard block, not a warning.
3. `--max-budget-usd` is passed to **every** `claude` session the launcher starts: each fresh-per-item relaunch and each auth relaunch. "This run may spend up to $X" overstates the guarantee in `fresh-per-item` mode.
4. Today every write under `.planning/meta-manager/drivers/` classifies as `ChangeKind::Planning`, so every frontmatter splice triggers a **full project re-parse**. D-24 is a live performance fix, not a nicety.
5. `launch_detached` re-execs `std::env::current_exe()`. Called in-process from a cargo test, it re-runs the *test harness*, which exits 0, so the call reports a false success. TUI launch tests need a seam.
6. The editor's exit status never reaches the library. `main.rs` only writes it into a status message, so D-17's "editor exited non-zero → nothing written" needs a small binary-side change.

**Primary recommendation:** Build the tab as a new `src/ui/screens/driver_run.rs` sibling, with pure transforms in a new `src/driver/steer.rs`. Wire every disk touch through `spawn_blocking → Action`. Keep the shipped `$VISUAL → $EDITOR → vi` order. Truncate with the shipped display-width helper `detail::fit_cells`, applied after sanitising.

## Settled Open Items (required by the orchestrator)

### Item 1: `$VISUAL` vs `$EDITOR` resolution order — **keep `$VISUAL → $EDITOR → vi`** `[inferred — audit]`

- **The app has exactly one editor handoff.** The decision lives in the binary's main loop:
  `src/main.rs:841-844` `[VERIFIED: src/main.rs:841-844]`, verbatim:
  ```
  // Determine editor: $VISUAL > $EDITOR > vi
  let editor = std::env::var("VISUAL")
      .or_else(|_| std::env::var("EDITOR"))
      .unwrap_or_else(|_| "vi".to_string());
  ```
  - Screens request it with `ScreenAction::SuspendAndEdit(path, line)`, declared at `src/ui/screens/mod.rs:354` `[VERIFIED: src/ui/screens/mod.rs (grep + read of the enum at :324)]`.
  - `App` records the request as `self.pending_editor = Some((path, line));` (`src/app.rs:2613-2615`) `[VERIFIED: src/app.rs:2613-2615]`.
  - The editor's arguments come from `editor_args` (`src/app.rs:528-560`) `[VERIFIED: src/app.rs:528-560]`.
- **Recommendation: follow it.** D-17's parenthetical ("run `$EDITOR` (fallback `$VISUAL`, then `vi`)") is the only place in the project that reverses the order. The UI-SPEC already chose the shipped order. No concrete reason to diverge was found:
  - A second rule would make the Docs, Archive and Queue `e` and the Driver `e` open different editors for the same user.
  - Git uses the same precedence: "The order of preference is $GIT_EDITOR, then core.editor configuration value, then $VISUAL, then $EDITOR" `[VERIFIED: man git-var, local]`.
- **Keep the shipped spawn as-is too.** `Command::new(&editor)` does **no shell word-splitting**, so `EDITOR="code --wait"` fails with "Failed to launch …". This is shipped behaviour for every `e` in the app. Changing it is out of scope, and a split would need either a new dependency (forbidden) or a naive split that breaks paths with spaces. List it in the help/pitfalls instead (Pitfall 7).
- **What does have to change (D-17 needs it):**
  - The exit status is consumed inside `src/main.rs:851-873` and only becomes a status message. The library hook `refresh_after_editor(&path)` (`src/app.rs:565-571`) receives only the path.
  - Change it to something like `app.after_editor(&path, EditorExit::{Success, Failed(String), SpawnError(String)})`, and route the DIRECTION edit-temp case from there.
  - Keep all apply logic in the library (`main.rs` is the binary, unreachable from `cargo test --lib`). Keep it off the async loop (see Pitfall 9).

### Item 2: Truncating long item titles at 60 columns — **display width, `…`, reuse `detail::fit_cells`** `[inferred — audit]`

- **Helper to reuse:** `fit_cells` at `src/ui/screens/detail.rs:10083-10103` `[VERIFIED: src/ui/screens/detail.rs:10080-10103]`. It is `pub(super)`, so it is already reachable from every module under `ui::screens`; `normal.rs:854` calls it as `super::detail::fit_cells(`. Verbatim doc and signature:
  ```
  /// `text` cut to at most `cols` terminal cells, the cut marked with `…`.
  /// Measured with `Span::width` per character, never with `str::len`, so a
  /// wide or multi-byte glyph is never split and never miscounted.
  pub(super) fn fit_cells(text: &str, cols: usize) -> String {
  ```
- **Precedent for this exact case** (a title as the last span of a fixed-prefix row, cut to the remaining room): the Waves row at `src/ui/screens/detail.rs:9619-9627` `[VERIFIED: src/ui/screens/detail.rs:9605-9634]`:
  ```
  let room = cells.saturating_sub(base + word_w + hint_w + tokens_w);
  ...
  if room > 1 {
      let cut = fit_cells(&title, room - 1);
  ```
- **Do not use `driver.rs::ellipsize`.** It sits at `src/ui/screens/driver.rs:793-800` `[VERIFIED: src/ui/screens/driver.rs:792-800]` and counts `char`s (`if text.chars().count() <= width`). A CJK or emoji title would overflow the row by up to its own length, and ratatui would clip off the `…`. `detail.rs::truncate_subject` (`:410-420`) has the same char-count flaw.
- **The rule for the item row** (and every other variable-width field on the tab: run id, reason, `last`, connection fields, LOG lines):
  1. Sanitise first, with `shown_capped` (`driver.rs:654`) or `crate::text::render_for_terminal`. `display_identity` **expands** each invisible-format char into `U+XXXX` (`src/text.rs:305-315`), so cutting before sanitising lets the expansion push the row past its width. The shipped order is `fit_cells(crate::text::render_for_terminal(msg).as_ref(), room)` at `detail.rs:1315`.
  2. Compute `room` from the **rendered widths of the spans already placed** (`Span::width`), not from the constant 27. Item ids are `I-\d+` and unbounded (lint `ITEM_HEADING` `r"^### (I-\d+)[ \t]+(\S.*?)[ \t]*$"`, `src/driver_templates/lint.rs:54-55`) `[VERIFIED: src/driver_templates/lint.rs:54-55]`, so `I-1234` breaks a fixed 5-cell column.
  3. Apply `if room > 1 { fit_cells(&title, room - 1) }` (one cell for the separating space). If `room <= 1`, drop the title span entirely; never draw an orphan `…`.
- This settles the UI-SPEC's single "⚠ unresolved" row. At the 39-cell detail floor the title gets whatever is left, possibly nothing. The full title stays readable in FULL tier and via `[e]`. No wrap key is added, as the UI-SPEC planner assumption says.
- The UI-SPEC says "ellipsised with `…` by `char` count". For ASCII, char count equals display width, so the two agree in the common case. This research overrides only the wide-glyph case, which is the case the UI-SPEC's own "never clipped" backstop checks.

<user_constraints>
## User Constraints (from CONTEXT.md)

### Locked Decisions
### Musk pass (discuss:pre)
- No scope cuts. DTAB-01..04 trace to the 2026-10-14 re-plan (REQUIREMENTS §Driver Tab), and each SC restates
  one of them. The SC4 injection retirement is required for Phase 31 to delete that path. Lean lane not taken:
  the phase spans the reader, the watcher, the TUI screens, steering writes, launcher calls and the dashboard,
  which is well over 3 tasks. `musk milestone-check` reported `run: false`.

### Tab structure and the legacy surface (SC1, SC4)
- **D-01:** The Driver tab becomes **launcher-run first**:
  - The run list is `list_driver_runs` over `.planning/meta-manager/drivers/`, newest first. The selected run's
    detail fills the main pane.
  - The 28-07 launcher section, the `UNSANDBOXED` badge and the stop key move into this view (28-07 "Phase 30
    re-hosts").
  - Phase 17/18 journal runs (`drive`), if any exist, appear only as a collapsed, read-only "legacy runs"
    section below the list. Their code is deleted in Phase 31.

  `[inferred — audit]`
- **D-02:** The Phase 18 injection input (`driver_inject.rs`) and the legacy `drive` start flow
  (`driver_start.rs` / `driver_confirm.rs`) are **no longer reachable from the tab**: their keybindings are
  removed or rebound to the D-16 new-run flow. Their modules and journal kinds stay compiled until Phase 31
  deletes them. The help screen and the footer hints are updated so no key promises injection. This is what
  SC4's "no longer the steering path" means operationally. `[inferred — audit]` — **Reversibility:** reversible
  — the modules remain, so this is only a keymap change until Phase 31.
- **D-03:** The tab keeps the existing experimental gating: whatever gate `driver launch` and the current Driver
  tab use (`GSDMM_EXPERIMENTAL_FEATURES` / `ctx.experimental`). Phase 30 adds no new gate. `[inferred — audit]`

### What the selected-run view shows (DTAB-01, SC1)
- **D-04:** Status is read **only** from `state_reader::driver_frontmatter::read_driver_run` → `DerivedStatus`
  (27 hand-off 4). The raw `status:` line is never shown as the status. Each `DerivedStatus` variant gets its own
  word:
  - `AsWritten(s)` → `s`;
  - `NotLaunched` → "not launched";
  - `Crashed` → **crashed**, error style;
  - `LivenessUnknown` → "liveness unknown", warning style. Never shown as crashed or alive.
- **D-05:** The header carries:
  - run id, derived status and `status_reason`;
  - **live `control` from the DIRECTION body** (`parse_direction`), not the frontmatter echo (27 hand-off 2).
    When they differ, the body value is shown with a "pending" marker until the next regenerate;
  - `sandbox` posture (`UNSANDBOXED` badge when degraded, with `sandbox_missing`);
  - `prompt_version` (a `custom:` value in the warning style);
  - `current_item` / `current_phase` / `current_step`;
  - `last_event`.

  `[inferred — audit]` for the pending marker and the custom-prompt styling.
- **D-06:** Items: one row per item, merging two sources. The item list and its `hold` / `answer` fields come
  from the DIRECTION body. State, dispatches and outcome come from frontmatter `item_states`. The state
  vocabulary is the closed 27 D-19 enum (`pending|running|done|parked|failed`). An item in one source but not the
  other is still listed and marked as such (body-only = "not yet seen by the driver"). `[inferred — audit]`
- **D-07:** Caps: `dispatches used/limit`, `wall_clock_min used/limit` and `gap_rounds used/limit (phase)`, each
  shown as numbers plus a coarse bar. A cap at or over its limit renders in the warning style. Dispatch audit:
  `logged / observed / denied`. **`logged != observed` is flagged** in the error style, because the audit exists
  to catch a driver whose account of its own dispatches diverges from what the hooks saw. `[inferred — audit]`
  for the styling.
- **D-08:** Heartbeat age is `now − heartbeat`, recomputed on the existing UI tick (no file I/O per tick). It is
  **stale** when the age exceeds the frontmatter's `heartbeat_stale_after_s` and the derived status is a live one
  (`running`, or `starting` with the lock held). A stale heartbeat on a live-locked run is flagged as "stale", not
  crashed: the lock is the liveness authority and the heartbeat only a hint (27 D-11 coalescing means the
  frontmatter heartbeat can lag by up to 120 s). The threshold comes from the file, never from a TUI constant.
  `[inferred — audit]`
- **D-09:** `connected[]` is shown as one line per connection. Every frontmatter string, LOG line, run id and
  DIRECTION excerpt is **untrusted**, so all of it renders through the existing `sanitize_render_line` /
  `render_for_terminal` / `shown_capped` path and is covered by the render-escape guard (T-28-19 pattern;
  27-REVIEW: sandbox-originated fields reach LOG.md verbatim). Rationale: 28-SECURITY follow-up — sibling run dirs
  are writable from inside a sandbox, so one run's driver can forge another run's frontmatter. The tab must
  therefore never treat frontmatter content as more than display data, and it **derives liveness from the lock,
  never from frontmatter alone**.
- **D-10:** Layout height tiers are measured from the render, with no header-without-content at any height
  (folded todo, D-29). Below a minimum height the tab shows a one-line summary per run instead of the sectioned
  layout. The UI-SPEC step owns the exact tiers. `[inferred — audit]`

### Crash, stale and dashboard markers (DTAB-02, SC2)
- **D-11:** "Crashed" is exactly `DerivedStatus::Crashed`: `running`, or `starting` with `launches > 0`, while the
  run lock is free or absent (27 D-22). The tab never writes `crashed` to disk; that stays a reading (27 D-07).
  The launcher records `crash-detected` on the next relaunch.
- **D-12:** The dashboard project marker extends OBS-02 with launcher runs. Per project, the runs considered are
  every non-terminal run dir plus the most recent run, and they reduce to one marker with precedence
  **crashed > parked > driven > stale**:
  - **driven:** a live run (lock held);
  - **parked:** run status `parked`, or a live run with any item `parked`;
  - **crashed:** `DerivedStatus::Crashed`;
  - **stale:** a live run with a stale heartbeat.

  `needs_human` (OBS-07, the `h` filter and AttentionFirst sort) becomes true for crashed and parked, **and for
  stale**. `LivenessUnknown` shows the existing unknown/warning marker without counting as needs-human.
  `[inferred — audit]` for the precedence and for counting stale as attention.
- **D-13:** Dashboard reads happen off the render thread, using the existing `spawn_blocking` → `Action` idiom
  and the per-project cache (`ProjectViewCache.launcher_runs`). They are refreshed on watcher events for the
  drivers dir and on the existing slow tick, so a lock released by a SIGKILL (which emits no file event) still
  turns into **crashed** within one tick. `[inferred — audit]`

### Steering writes (DTAB-03, SC3)
- **D-14:** Every tab write to DIRECTION.md goes through `driver::direction::write_body` (27 D-13, hand-off 1),
  which takes `DIRECTION.lock`, never touches the `---` block and refuses an insane body. No other write path
  exists. On non-Linux, `write_body` returns `UnsupportedPlatform`, the steering keys are hidden, and the tab is
  read-only with a one-line reason. — **Reversibility:** costly — the shared-lock rule is what keeps the
  driverctl splice and human edits from losing bytes (27 race test).
- **D-15:** The write operations are pure body transforms, unit-tested against the template body and the spike
  fixtures:
  - **set control:** rewrite the single `- control:` line under `## Direction` (insert one if absent). The value
    is one of `run|pause|stop`. Setting `stop` asks for a confirmation; `pause` and `run` apply immediately.
  - **answer item:** set the item's `- answer:` to a single-line, sanitised value (control characters and
    newlines rejected, length-capped) and set `- hold: false`.
  - **un-hold item:** set `- hold: false`.

  The effect on the driver is the Phase 29 contract (control honoured before the next dispatch; the gate denies
  `LAUNCHER CAP: control=...`). `[inferred — audit]` for the confirm-on-stop rule and the length cap.
- **D-16:** **OQ-R1 is surfaced, not changed.** When the selected parked item's park rule is anything other than
  `needs-human` (`launcher-cap`, `run-budget`, `divergence`, `gap-rounds`, `budget`, `no-progress`), the answer
  action still writes. It also shows that an `answer:` alone will not resume this item and that a budget edit (in
  `$EDITOR`) or a relaunch with higher caps is required (29 D-13). The park rule is read from tool-written data
  (`item_states` outcome, else the item's last `parked --rule` LOG line). `[inferred — audit]`
- **D-17:** **`$EDITOR` edits a body-only copy, never the live file.** The flow:
  1. read the body;
  2. write it to an edit temp in the run dir. The temp must **not** use the splice prefix `.DIRECTION.md.fm.`,
     which `sweep_stale_temps` deletes;
  3. suspend the TUI (existing terminal-handoff pattern) and run `$EDITOR` (fallback `$VISUAL`, then `vi`);
  4. on exit, apply the result with `write_body`, but only if the live body still equals the base the user
     started from. The splice never changes the body, so a change here
     means another human edit or a tab action. If the body changed, nothing is clobbered: the user's text is kept at the temp path and the tab offers
     re-open or discard;
  5. an edited body that fails `body_sane` or the Phase 29 `driver check` lint is applied only after an explicit
     confirm that lists the lint findings.

  Reason: SC3 requires every edit to take the shared flock and never touch the frontmatter, and holding
  `DIRECTION.lock` for a whole editor session would starve the splice (2 s budget, 27 D-20).
  `[inferred — audit]` — **Reversibility:** reversible.

### Lifecycle: new run, relaunch, stop, LOG tail (DTAB-04, SC4)
- **D-18:** **New run** from the tab:
  1. ask for a slug;
  2. mint via the Phase 29 library (`driver_templates::mint_run_id` plus the `driver new` creation path; refuse
     on collision, 29 D-08);
  3. open the new DIRECTION body in the D-17 editor flow;
  4. run the `driver check` lint and show its findings; the launch stays blocked while findings exist;
  5. show a launch confirmation, then launch.

  A user may also leave a minted run unlaunched (it shows as "not launched").
- **D-19:** The **launch confirmation** shows and lets the user edit the `LaunchArgs` that cost money or bound
  the run:
  - `model` (default `opus`), `loop_mode` and `max_budget_usd` (default $10);
  - `cap_dispatches`, `cap_minutes` and `cap_gap_rounds`.

  The defaults are the CLI defaults, never TUI copies. The confirmation states the spend ceiling in plain words.
  Launch calls the launcher library's **detached** path (`launch_detached`, 28 D-04), which re-execs the binary as
  a daemon that holds the run lock and binds the driverctl socket. **The TUI process never binds the run socket
  or embeds `RunStore`.** A run must outlive the TUI, and keeping the bind out of the multithreaded TUI runtime
  makes CR-01 (fixed in 0951aea: socket bound in a private staging dir, no process-wide umask swap) moot for the
  TUI. The CR-01 fix remains defence in depth. `[inferred — audit]` — **Reversibility:** costly — embedding the
  launcher in the TUI would tie run lifetime to the TUI process.
- **D-20:** Degraded hosts: the confirmation runs the 28 D-28 host detection first. If bwrap, systemd-run or the
  user manager is missing, launch is refused with the missing items named. `--unsandboxed` is offered only as a
  **separate, explicit second confirmation** that states the run will be unsandboxed, never as a default or a
  remembered preference (28 D-29: always an explicit per-launch act). `[inferred — audit]`
- **D-21:** **Relaunch** is offered for `Crashed`, and for terminal `stopped` / `parked` / `stalled` runs (the
  route to "a relaunch with higher caps", OQ-R1). It reuses the D-19 confirmation, prefilled from the run's
  recorded caps and model, and launches the same run id (the launcher logs `crash-detected` / `begin_launch`).
  It is refused while the lock is held or liveness is unknown. `[inferred — audit]` for offering relaunch beyond
  crashed.
- **D-22:** **Stop** stays the 28-07 shared `stop_run` path (`LauncherStopRequested`), with its confirmation, and
  is re-hosted in the new view. It is distinct from `control: stop`:
  - `control: stop` is the polite path: the driver ends after recording the in-flight report.
  - Stop kills the scope.

  Both are offered, labelled so the difference is visible. `[inferred — audit]`
- **D-23:** **LOG.md live tail:** a pane in the selected-run view that tails LOG.md. It reads by byte offset
  from the last read, so watcher events append only new bytes, and resets on truncation or replacement. It keeps
  a bounded ring buffer in the `DRIVER_OUTPUT_RING_LINES` style, follow and scroll behave like the existing
  output pane, and every line is sanitised (D-09). It reads LOG.md only, never the stream-json transcript or the
  launch log. `[inferred — audit]` for scope.

### Watcher and performance (OBS-06 carry)
- **D-24:** Events under `.planning/meta-manager/drivers/` are routed to a **driver-run refresh** for that run.
  They never trigger a full project re-parse (OBS-06). The router **ignores** `state.json` and
  `.DIRECTION.md.fm.*` (27 hand-off 3, Pitfall 9) and the D-17 edit temp. DIRECTION.md and LOG.md changes are
  debounced in the existing batch. `[inferred — audit]` for routing through the existing recursive `.planning`
  watch rather than a second watch.
- **D-25:** No file I/O on the render thread (D-28 of Phase 18). Frontmatter reads, lock probes, LOG tail reads
  and body writes all go through `spawn_blocking` → `Action`.

### Verification and spend
- **D-26:** **No paid model spend in Phase 30.** All acceptance is deterministic and CI-safe:
  - pure transform and render tests;
  - `TestBackend` snapshots per height tier;
  - fixture run dirs for every `DerivedStatus`, including a lock held by a test thread versus a free lock;
  - a watcher-routing test;
  - a launch and relaunch test against the Phase 28 stub-`claude` e2e pattern (zero spend).

  A live TUI launch smoke on a real model is **not** required. If the planner wants one, it is opt-in
  (`#[ignore]`), Haiku only, and **needs operator sign-off before any spend**. `[inferred — audit]`
- **D-27:** The editor flow is tested with `$EDITOR` set to a scripted stub that rewrites the temp. The race case
  (live body changed mid-edit) must show no clobber. `[inferred — audit]`
- **D-28:** UI work follows the project's UI-SPEC gate (`workflow.ui_phase: true`): the tab's layout tiers,
  vocabulary and keymap are fixed in a `30-UI-SPEC.md` before planning. The Phase 18 rule on honest vocabulary
  carries over: no spinner, no wording that promises an observation the files cannot make. Example: "control:
  pause (pending)" until the frontmatter echoes it, never "paused". `[inferred — audit]`

### Folded Todos
- **D-29:** "Driver tab renders blank pipeline row at terminal heights 8-13" (score 0.9, ui; Phase 18 WR-05). The
  tab is being re-laid out, so the new height tiers must not render a header or rule without its content at any
  height (D-10). A `TestBackend` sweep over heights 1-40 is the check. The legacy pipeline row only survives
  inside the collapsed legacy section, which obeys the same rule. `[inferred — audit]`

### Claude's Discretion
- Module layout: whether the new view lives in `src/ui/screens/driver.rs` (already ~4.9k lines) or a new
  `driver_run.rs` sibling, and where the pure body transforms live (`driver::direction` versus a new
  `driver::steer`).
- Exact keybindings, subject to D-28's UI-SPEC and no clash with existing global keys.
- How many runs the dashboard scan reads per project (bounded; `list_driver_runs` already takes a `limit`).
- Plan split and waves. The reader/view, the steering transforms, the watcher routing and the lifecycle/launch
  flow are largely independent. The dashboard marker depends on the reader.

### Deferred Ideas (OUT OF SCOPE)
- Deleting the injection, legacy `drive` start and journal-run code: Phase 31.
- Master prompt 1.1 (reconcile-first refusal routing, `$RUN_DIR` substitution): backlog 999.7.
- Masking sibling run dirs from the sandbox (28-SECURITY follow-up): a launcher hardening item, not tab scope.
  `[inferred — audit]`
- A macOS `renamex_np` splice, which would enable steering on macOS: future work (27 deferred).
- Editing caps of a *running* run: caps change only on relaunch (D-21).

#### Reviewed Todos (not folded)
`[inferred — audit]` This departs from the auto rule (fold every todo scoring at least 0.4); each of these sits
outside the tab boundary.
- "The verify-work gate policy must be configurable" (0.9): already folded into the Phase 29 template (29 D-16).
- "Codex runtime: remainder after the MVP slice" (0.7): not the Claude launcher.
- "Dashboard renders quoted VERIFICATION status as Unknown…" (0.6): a dashboard-reader fix (27 D-27), suited to
  a quick task. The D-12 marker does not read VERIFICATION status.
- "Make queue input truly multi-line" (0.3): queue UI, unrelated.
</user_constraints>

<phase_requirements>
## Phase Requirements

| ID | Description | Research Support |
|----|-------------|------------------|
| DTAB-01 | Lists a project's run dirs; for the selected run shows, from frontmatter only: status, per-item state, caps used/limit, dispatch audit, heartbeat age, `connected[]` | `list_driver_runs` / `read_driver_run` / `DerivedStatus` (§Standard Stack). One-read `RunDetail` loader (Pattern 2). Body item reader for titles/hold/answer (Pattern 3). Heartbeat from the `role == "driver"` connection (Pitfall 10). Truncation via `fit_cells` (Item 2) |
| DTAB-02 | Crashed runs and stale heartbeats are flagged; the dashboard marks driven/parked/crashed projects | `DerivedStatus::Crashed` is the only crash source (D-11). Fleet marker scan rides the 20-tick `RunsReconciled` task (Pattern 6). Two `needs_human` predicates (Pitfall 12). Stale threshold read from `heartbeat_stale_after_s` (default 300) |
| DTAB-03 | Set `control`, answer or un-hold a parked item, open DIRECTION.md in `$EDITOR`; every edit takes the shared flock and never touches the frontmatter | `write_body` closure transforms (Pattern 3). Its lock-timeout semantics (Pitfall 1). `<!--` answer injection (Pitfall 4). Editor flow with the base-compare closure (Pattern 4). Editor order settled (Item 1) |
| DTAB-04 | Start a new run, relaunch a crashed run, tail LOG.md live | `create_run_dir` + `lint_direction` + `launch_detached` + `decide(&detect(), …)` (Pattern 5). Launch preflight refuses `control != run` (Pitfall 2). Spend copy (Pitfall 3). `current_exe` test trap (Pitfall 5). `tail_lines` + inode check + `parse_log` for the LOG tail (Pattern 7) |
</phase_requirements>

## Project Constraints (from CLAUDE.md)

- **State reading** must not require running Claude/GSD; read files or cached state. The tab reads only run files and lock probes.
- **Non-intrusive.** Must not interfere with running GSD instances. The lock probe is `LOCK_SH|LOCK_NB` and released immediately (`src/driver/lock.rs:361-382`). The launcher acquires with retry (`acquire_at_with_retry`, `src/launcher/run.rs:585-590`), so a concurrent probe cannot cause a spurious "already launched" `[VERIFIED: both files read]`.
- **Portability.** Works for any GSD user. Linux-only pieces stay behind `#[cfg(target_os = "linux")]` with a typed refusal elsewhere. CI only builds `ubuntu-24.04` (`.github/workflows/release.yml:20,50`), so macOS cfg hygiene is not machine-checked. See Pitfall 13.
- **Stack** (CLAUDE.md table): ratatui 0.30, crossterm 0.29, tokio 1.x, notify 8 + `notify-debouncer-full`, serde/serde_json, anyhow, tracing to a file. **No `std::sync::Mutex` held across `.await`.** No `env_logger`.
- **GSD workflow enforcement:** edits happen through GSD commands, so the phase executes via `/gsd-execute-phase`.
- **Release process** (version bump, `pre-tag-check.sh --container`) is milestone-level and not this phase's concern.
- **Testing rule from project memory:** run `cargo test --no-fail-fast`. The git-version witness failure in `src/envelope/policy.rs` is expected locally. `rtk` filters output, so use `rtk proxy` for any check that greps raw output.

## Architectural Responsibility Map

| Capability | Primary Tier | Secondary Tier | Rationale |
|------------|-------------|----------------|-----------|
| Run listing, frontmatter parse, lock probe | Storage reader (`state_reader::driver_frontmatter`) on `spawn_blocking` | TUI cache (`ProjectViewCache`) | Blocking fs and `flock`. D-25 forbids render-thread I/O |
| Derived status / crash / liveness | Storage reader (`derive_status`) | none | Lock is the liveness authority (D-09, D-11). The TUI never re-derives |
| Heartbeat age, stale flag, cap bars, audit mismatch | TUI view model (pure fns over cached frontmatter + `now`) | none | Recomputed per tick with zero I/O (D-08) |
| Body item reader (title/hold/answer) and steering transforms | Domain library (`driver::steer`, pure) | `driver::direction::write_body` (I/O) | Pure transforms are unit-testable (D-15). `write_body` owns lock + exchange |
| Editor handoff | Binary main loop (`main.rs`) | Library post-editor hook (`App`) | Only the binary owns the terminal. Apply logic must stay in the library |
| New run / lint / launch / stop | Launcher + templates library on `spawn_blocking` | TUI confirm screens | Detached daemon owns the run lock and socket (D-19). The TUI never embeds `RunStore` |
| LOG tail | Journal reader (`tail_lines`) on `spawn_blocking` | TUI ring (`DriverOutput`) | Byte-offset, bounded, append-only |
| Watcher routing | Watcher callback (`classify_change`, pure, no fs) | `App::update` `FileChanged` arm | Debouncer thread has no runtime. Purity is a documented invariant |
| Dashboard marker | Reconcile scan task (20-tick) | `normal.rs` badge + `mod.rs` attention | Fleet-wide, must catch SIGKILL (no file event) within one slow tick |

## Standard Stack

All in-repo. **No new crates.** UI-SPEC: "New Cargo dependencies | **Zero**".

### Core (existing APIs this phase consumes)
| API | Location | Purpose | Notes |
|-----|----------|---------|-------|
| `list_driver_runs(project_root, limit) -> Vec<DriverRunEntry>` | `src/state_reader/driver_frontmatter.rs:826-846` | Run list, newest first | Skips symlinks. An `Err` row stays in the list. Blocking `[VERIFIED: read :796-846]` |
| `read_driver_run(run_dir)` / `derive_status(fm, probe)` | same, `:774-794` / `:751-764` | Frontmatter + `DerivedStatus` | `derive_status` is `pub` and `#[cfg(unix)]`, so a one-read loader can reuse it `[VERIFIED]` |
| `DerivedStatus` variants | same, `:668-684` | Status vocabulary | Verbatim: `AsWritten(RunStatus)`, `NotLaunched`, `Crashed { written: RunStatus }`, `LivenessUnknown { written: RunStatus }` `[VERIFIED: :668-684]` |
| `RunStatus` words | same, `:114-126` | | Verbatim: `"starting"`, `"running"`, `"paused"`, `"parked"`, `"stopped"`, `"done"`, `"crashed"`, `"stalled"`, plus `Unknown(word)`. `is_terminal` = `Stopped \| Done \| Crashed \| Stalled \| Parked` `[VERIFIED: :112-155]` |
| `ItemState` | same, `:186-213` | closed item enum | Verbatim: `"pending"`, `"running"`, `"done"`, `"parked"`, `"failed"` `[VERIFIED]` |
| `HEARTBEAT_STALE_AFTER_S` | same, `:72` | default threshold | `pub const HEARTBEAT_STALE_AFTER_S: u64 = 300;`. The tab reads `fm.heartbeat_stale_after_s`, never this const (D-08) `[VERIFIED]` |
| `DRIVER_RUNS_REL` | same, `:799` | drivers path | `pub const DRIVER_RUNS_REL: &str = ".planning/meta-manager/drivers";` `[VERIFIED]` |
| `driver::direction::{write_body, split_frontmatter, parse_direction, body_sane, blank_comments, section_heading, control_hold}` | `src/driver/direction.rs` | Body read and locked write | See Pattern 3 and Pitfall 1 `[VERIFIED: read whole file :1-673]` |
| `TEMP_PREFIX`, `DIRECTION_LOCK_FILE`, `DIRECTION_LOCK_BUDGET` | `src/driver/direction.rs:72-76` | | `pub const DIRECTION_LOCK_FILE: &str = "DIRECTION.lock";`, `pub const TEMP_PREFIX: &str = ".DIRECTION.md.fm.";`, `pub const DIRECTION_LOCK_BUDGET: Duration = Duration::from_secs(2);` `[VERIFIED]` |
| `driverctl::log::{parse_log, LogEvent, epoch_seconds}` | `src/driverctl/log.rs:28-147` | LOG event parse, park-rule lookup | Not cfg-gated, so cross-platform `[VERIFIED: src/driverctl/mod.rs:114-122]` |
| `journal::reader::tail_lines(path, TailCursor) -> io::Result<TailRead>` | `src/journal/reader.rs:140-219` | Byte-offset line tail | Handles missing file, shrink (`restarted`), torn line, oversize (`MAX_TAIL_BYTES` 4 MiB), non-UTF-8 `[VERIFIED]` |
| `driver_templates::{create_run_dir, mint_run_id, NewRunError, SLUG_RE}` | `src/driver_templates/mod.rs:48-278` | New run | `#[cfg(target_os = "linux")]` (`:50-51`). `SLUG_RE = r"^[A-Za-z0-9][A-Za-z0-9._-]*$"` `[VERIFIED]` |
| `driver_templates::lint::{lint_direction, LintReport, Finding}` | `src/driver_templates/lint.rs:62-111, 200` | `driver check` findings | `Finding: Display` already escapes via `render_for_terminal` `[VERIFIED]` |
| `launcher::run::{launch_detached, LaunchOptions, LoopMode}` | `src/launcher/run.rs:140-182, 1211` | Detached launch | Linux-only module (`src/launcher/mod.rs:81-82`). Blocks up to `DETACH_BUDGET` 10 s (`:117`) `[VERIFIED]` |
| `launcher::stop::stop_run(project, run_id, wait)` | `src/launcher/stop.rs:93` | Kill | Already wired: `App::stop_launcher_run` (`src/app.rs:985-1028`) `[VERIFIED]` |
| `launcher::host::{detect, decide, HostCaps, Posture}` | `src/launcher/host.rs:43-186` | Degraded-host check | `decide` also refuses a missing or too-old `claude` (`MIN_CLAUDE_VERSION = (2, 1, 295)`) `[VERIFIED]` |
| `cli::{LaunchArgs, positive_finite, MAX_BUDGET_USD, MAX_CAP_MINUTES, DEFAULT_CAP_GAP_ROUNDS}` | `src/cli.rs:9-35, 526-574` | Defaults + validators | Defaults verbatim: `default_value = "opus"`, `DriverLoopMode::Loop`, `10.0`, `10`, `60.0`, `DEFAULT_CAP_GAP_ROUNDS` (`= 2`). `positive_finite` is `pub` `[VERIFIED: :1-50, :510-575]` |
| `launcher::push::policy_for` + `registry.preferences.driver_open_pr()` | used at `src/driver_cli.rs:286-313` | Fill `LaunchOptions.open_pr`/`pr_policy` | The daemon recomputes both from `--config` (`driver_cli.rs:310-312` comment) `[VERIFIED]` |
| UI: `sanitize_render_line`, `shown_capped`, `render_for_terminal`, `fit_cells`, `clamp_scroll`, `tail_offset`, `follow_indicator`, `DriverOutput`, `confirm_popup`, `adjudicate_screen!` | `ui/screens/{mod,driver,detail,confirm_popup}.rs` | Rendering primitives | See Item 2 and Pattern 8 |
| `ProjectViewCache` (`#[derive(Default)]`) | `src/ui/screens/mod.rs:1124-1220` | Per-project view state | Adding fields costs nothing. An `AppContext` field costs an edit at every construction site (`mod.rs:1127-1129`) `[VERIFIED]` |
| `LAUNCHER_RUNS_LISTED`, `DRIVER_OUTPUT_RING_LINES`, `DRIVER_OUTPUT_LINE_CELLS` | `src/ui/screens/mod.rs:390-407` | Bounds | `= 20`, `= 2_000`, `= 512` `[VERIFIED]` |

### Alternatives Considered
| Instead of | Could Use | Tradeoff |
|------------|-----------|----------|
| `detail::fit_cells` (display width) | `driver::ellipsize` (char count) | Char count overflows on wide glyphs. Rejected (Item 2) |
| clap-parse `LaunchArgs` for defaults + validation | Hand-copied defaults in the form | D-19 forbids TUI copies. clap is the single source |
| Fleet scan inside the 20-tick reconcile task | A second timer, or the Driver-tab-only scan | A second timer is forbidden by the comment at `src/app.rs:1281-1287`. The tab-only scan never sees other projects |
| `tail_lines` for LOG | A fresh full read each event | Full re-reads grow with the file. `tail_lines` costs only new bytes |
| `tui-textarea` for the answer input | — | UI-SPEC forbids it (`no tui-textarea; no movable cursor`) |

**Installation:** none.

## Package Legitimacy Audit

This phase installs **no external packages** (UI-SPEC: zero new dependencies; confirmed from `Cargo.toml` by reading the dependency lines). There is nothing to audit.

| Package | Registry | Age | Downloads | Source Repo | Verdict | Disposition |
|---------|----------|-----|-----------|-------------|---------|-------------|
| (none) | — | — | — | — | — | — |

**Packages removed due to [SLOP] verdict:** none. **Packages flagged [SUS]:** none.

## Architecture Patterns

### System Architecture Diagram

```
                 key press (detail screen, Driver sub-view)
                               |
        +----------------------+-----------------------------+
        |                      |                             |
   j/k/[/]/PgUp/f/G       c / a / u / e / s / R / x       (render: pure, no I/O)
   (view state only)            |                             ^
        |               ScreenAction::Push(input/confirm)     |
        |                       |                             |
        |               ScreenAction::DispatchAction(Action::Driver*)
        |                       v                             |
        |                 App::update ---- spawn_blocking ----+-----------------+
        |                       |                                               |
        |     +-----------------+------------------+------------------+         |
        |     v                 v                  v                  v         |
        |  steer transform   edit prepare/apply  create_run_dir +   launch_detached / stop_run
        |  + write_body      (temp + base cmp)   lint_direction     (+ detect/decide)
        |  (DIRECTION.lock)        |                                 |
        |     |                    v                                 v
        |     |           pending_editor -> main.rs suspends TUI,   detached daemon holds
        |     |           runs $VISUAL/$EDITOR/vi, then             launcher.lock + socket
        |     |           App::after_editor(path, exit)             (outlives the TUI)
        |     v                    v                                 |
        |  Action::Driver*Done (status copy, refresh request) <------+
        |                       |
        v                       v
   ProjectViewCache  <-- Action::DriverRunRefreshed { detail, log lines, cursor }
        ^                       ^
        |                       | spawn_blocking: read DIRECTION.md once (frontmatter +
        |                       | body + probe launcher.lock) + tail_lines(LOG.md)
        |                       |
   notify debouncer --> classify_change(root, path) [pure]
     (.planning/ recursive)     |-- drivers/<id>/{DIRECTION.md,LOG.md} -> DriverRun{run_id}
                                |-- drivers/<id> (dir create/remove)    -> DriverRun list refresh
                                |-- drivers/<id>/anything else          -> Ignored
                                |-- runs/<id>/...                       -> DriverJournal (legacy)
                                '-- otherwise                           -> Planning (re-parse)

   Action::Tick x20 (~5 s) --> spawn_blocking { reconcile_all; last_ended_outcomes;
                                                per project: list_driver_runs(.., 20) -> marker }
                           --> Action::RunsReconciled{.., launcher_markers}
                           --> dashboard badge (normal.rs) + needs_human/attention (mod.rs)
```

### Recommended Project Structure
```
src/
├── driver/
│   ├── direction.rs        # unchanged except: pub(crate) on control_re()/item_re() (single reader, WR-01)
│   └── steer.rs            # NEW, pure: body item reader + set_control / answer / unhold transforms
├── state_reader/
│   └── driver_frontmatter.rs   # unchanged (consumed)
├── journal/
│   └── mod.rs              # ChangeKind gains DriverRun{run_id} and Ignored; classify_change extended
├── ui/screens/
│   ├── driver_run.rs       # NEW: launcher-run view model + render (header/items/caps/connected/log,
│   │                       #      height tiers, SUMMARY rows, status tokens, footer spans)
│   ├── driver_steer.rs     # NEW: control chooser, answer input, edit-conflict chooser screens
│   ├── driver_launch.rs    # NEW: slug input, lint popup, launch form, unsandboxed confirm
│   ├── driver.rs           # legacy run detail kept for the collapsed legacy section; launcher section removed
│   ├── detail.rs           # keymap: retire i/{/}/X, rebind s, add c/a/u/R/L, e Driver branch, footer
│   ├── normal.rs           # badge ranks for crashed/stale/unknown
│   ├── mod.rs              # needs_human split, marker types, ProjectViewCache fields, constants
│   ├── help.rs             # Driver section + two legends; drop injection legend
│   └── render_escape_guard.rs  # fixtures for every new screen and site
├── app.rs                  # new Action arms, after_editor hook, launch seam, marker scan in the 20-tick task
├── action.rs               # new Action variants (plain data, Clone)
└── main.rs                 # pass the editor exit status to App::after_editor (the only binary change)
```
`[inferred — audit]`: Claude's discretion, chosen because `driver.rs` is 4,875 lines and the legacy view must survive until Phase 31 (D-02).

### Pattern 1: Everything that touches disk is `spawn_blocking` → `Action`
**What:** A key handler or tick only enqueues work. A blocking task does the I/O and sends one plain-data `Action`. `App::update` applies it to the cache.
**Shipped example:** `AppContext::schedule_run_list_scan` (`src/ui/screens/mod.rs:1836-1951`), and `App::stop_launcher_run` (`src/app.rs:985-1028`) for a launcher call. Both refuse visibly when `event_tx` is `None` ("Refuse visibly, never silently (S4)").
**Guard:** `tests/async_blocking_guard.rs` lexically scans `async fn` bodies for `BLOCKING_MARKERS` (`.status()`, `flock(`, `fs::read(`, `fs::write(` …) without a `spawn_blocking` hand-off. Its allowlist covers exactly `("src/main.rs", ".status()")` for the editor. Adding file I/O to `run_tui_loop` would trip it.

### Pattern 2: One read per refresh for the selected run
**What:** `read_driver_run` reads `DIRECTION.md` and discards the body, but the tab also needs the body. Read the text once, then:
```rust
// Source: composed from src/state_reader/driver_frontmatter.rs:751-794 and src/driver/direction.rs:87-116
let text = std::fs::read_to_string(run_dir.join("DIRECTION.md"))?;          // on spawn_blocking
let fm = parse_driver_frontmatter(&text)?;                                     // pub fn, :452
let derived = derive_status(&fm, probe_at(&run_lock_path(run_dir)));           // #[cfg(unix)]
let (_, body) = split_frontmatter(&text);
let info = parse_direction(body);            // live control (D-05), item ids + budgets
let items = steer::read_items(body);         // NEW: titles, hold, answer (Pattern 3)
```
`#[cfg(not(unix))]`: mirror `read_driver_run`'s fallback (`derive(&frontmatter, Seen::Unknown)` is private). On non-unix, call `read_driver_run` for the view and re-read the body separately. Or add a `pub fn derive_status_unknown`. The first option needs no change to the reader.

### Pattern 3: Steering transforms are pure `&str -> Result<String, SteerError>` closures for `write_body`
**What:** `write_body(run_dir, edit)` calls `edit(current_body)`, **possibly more than once** (a human save raced the exchange: `src/driver/direction.rs:416-420`, "it may be called again on a fresh body"). It refuses an insane result with `DirectionError::InsaneNewBody`.
**Offsets:** Use `blank_comments` (same length, same char boundaries, `direction.rs:176-200`) to find matches outside comments, then splice into the original body at the same byte offsets. That is the "one comment reader" rule.
- **set_control(body, word):** find every `- control:` match of `control_re()` (`(?mi)^[ \t]*-[ \t]+control[ \t]*:[ \t]*(\w+)`, `direction.rs:149`) over the blanked body and replace capture 1 with `word`.
  - **Rewrite all matches**, not the first. `parse_direction` lets any `pause`/`stop` win over an earlier `run` (`direction.rs:239-249`), so leaving a second line untouched makes `set run` a silent no-op.
  - If there is no match, insert `- control: {word}\n` directly after the first `## Direction` heading line (`section_heading` returns `Section::Direction`).
  - Post-condition: `parse_direction(&new).control.as_deref() == Some(word)`, else `Err`.
- **Item segment:** from the `### I-NN` heading (`item_re()`, `r"(?m)^### (I-\d+)\b"`, `direction.rs:168`) to the next `### ` or `## ` heading, under `Section::Items` only. This mirrors `parse_direction`'s section rule.
- **unhold(body, id):** rewrite the first uncommented `- hold:` value in the item segment to `false`. Insert `- hold: false` at the segment end if absent. Not found → `Err(NoSuchItem)`.
- **answer(body, id, text):** validate `text` (Pitfall 4), then set the first `- answer:` line to `- answer: {text}` (insert if absent) **and** apply `unhold`.
- **read_items(body):** id, title (rest of the heading line after the id, trimmed), `hold` (`true`/`false`/other), `answer` (empty → `None`), in document order.

`[inferred — audit]`: give `item_re`/`control_re` `pub(crate)` visibility rather than copying the regexes into `steer.rs`. Phase 29 made `parse_direction` and the lint share readers on purpose (WR-01, `lint.rs:8-17`), and a third copy would reopen that drift.

**Example call:**
```rust
// Source: src/driver/direction.rs:421-434 (signature), :514-530 (closure contract)
let mut refused: Option<SteerError> = None;
let report = write_body(&run_dir, |live| match steer::set_control(live, "pause") {
    Ok(new) => new,
    Err(e) => { refused = Some(e); live.to_owned() } // identical body: harmless rewrite
})?;
```
Map the outcomes: `SpliceOutcome::{Ok, SkippedInsaneBody, GaveUp, NoDirection}` (`direction.rs:315-324`) plus `report.lock_timed_out` (Pitfall 1). Each needs its own status copy. The UI-SPEC lists only success and "Write failed".

### Pattern 4: D-17 editor flow (body-only temp, base compare inside the write closure)
1. `e` → `Action::DriverEditPrepare { alias, run_id }` → `spawn_blocking`:
   - read `DIRECTION.md`, `split_frontmatter`, keep `base = body`;
   - create the temp with `OpenOptions::new().write(true).create_new(true).mode(0o600)`. That is `O_EXCL`, and it fails on any existing path, a planted symlink included;
   - send `DriverEditReady { temp, base }`.
2. `App` stores an `EditSession { alias, run_dir, temp, base }` keyed by the temp path and sets `pending_editor = Some((temp, None))`. The main loop runs the editor (Item 1).
3. `main.rs` calls `app.after_editor(&path, exit)`, a **new** signature that carries the exit status. For a known edit temp, `App` dispatches `DriverEditApply` on `spawn_blocking`:
   - non-zero exit → keep the temp, write nothing;
   - read the temp: `symlink_metadata` must say regular file, cap the size, reject non-UTF-8;
   - `edited == base` → remove the temp, report "unchanged";
   - `lint_direction(&edited)`. Lint the **temp text**, so line numbers match what the user saw. Problems → confirm popup;
   - the write itself:
     ```rust
     let mut conflict = false;
     write_body(&run_dir, |live| if live == base { edited.clone() } else { conflict = true; live.to_owned() })
     ```
     - `conflict` → keep the temp and push the chooser;
     - `Err(InsaneNewBody)` → keep the temp, show the insane-body copy;
     - `Ok` → remove the temp.

   Also pre-check `live != base` before calling `write_body`. That avoids a needless exchange in the common conflict case. The in-closure check closes the race.
4. **Temp name:** UI-SPEC chose `.DIRECTION.md.edit` `[inferred — audit]`. It must not start with `TEMP_PREFIX` (`".DIRECTION.md.fm."`), and it doesn't. Optional refinement `[inferred — audit]`: `.DIRECTION.edit.md` keeps a `.md` extension, so editors apply Markdown highlighting. Use one `pub const` either way; the copy strings interpolate it.
5. **Stale temp from a crashed TUI:** step 1's `create_new` fails with `AlreadyExists`. Treat that as the conflict chooser ("a previous edit was not applied": `[r]e-open` / `[d]iscard`), never an overwrite `[inferred — audit]`.
6. **The run dir is read-only inside bwrap** (`--ro-bind` of the run dir, `src/launcher/sandbox.rs:294`) and the two lock files are masked with `/dev/null` (`:295-299`) `[VERIFIED: src/launcher/sandbox.rs:255-314]`. A sandboxed driver therefore cannot tamper with the temp while the human edits. An `--unsandboxed` run has full user permissions anyway.

### Pattern 5: Launch / relaunch through `launch_detached` behind a seam
- **Defaults from clap**, never copied:
  ```rust
  #[derive(clap::Parser)] struct LaunchForm { #[command(flatten)] args: LaunchArgs }
  ```
  - `LaunchForm::try_parse_from(["launch", project, run_id])` yields the defaults.
  - Validate a field by re-parsing with that flag, e.g. `["…", "--max-budget-usd", buf]`, or call `cli::positive_finite(text, what, MAX_*)` directly (it is `pub`, `src/cli.rs:16`).
  - Either way the CLI's own parser is the validator.
- **Relaunch prefill** (D-21): cap limits from `fm.caps.dispatches.limit` (`u32`), `fm.caps.wall_clock_min.limit` (`f64`) and `fm.caps.gap_rounds.limit` (`u32`); model from the `connected[]` entry with `role == "driver"` (`model`).
  - `max_budget_usd` is **not recorded** in the frontmatter, so it always shows the default with "(default)".
  - **Wall clock:** the launcher refuses a relaunch whose `cap_minutes` is already used up: `CapExhausted`, "raise --cap-minutes to continue it" (`src/launcher/run.rs:976-996`) `[VERIFIED]`. When `used >= limit`, prefill `minutes` with a Yellow hint, not the bare old limit.
- **Build `LaunchOptions`** exactly as `src/driver_cli.rs:286-314` does:
  - `alias` and `project` from the registry;
  - `limits: CapLimits { dispatches: Some(..), wall_clock_min: Some(..), gap_rounds: Some(..) }`;
  - `open_pr: ctx.config.preferences.driver_open_pr()`;
  - `pr_policy: policy_for(&alias, project.driver_opt_in.as_ref())`;
  - `prompt_file: None`.

  Then pass `config = Some(&ctx.config_path)` (`AppContext.config_path`, `src/ui/screens/mod.rs:1385`), so the daemon resolves the same registry.
- **Seam:** store `launch_fn: fn(&LaunchOptions, Option<&Path>) -> Result<u32, LauncherError>` (default `launch_detached`) on `App` or `AppContext`. App-level tests swap in a fake. See Pitfall 5 for why.
- **Host check for the form:** `decide(&detect(), false)` on `spawn_blocking` when the form opens. Map each refusal to its own copy:
  - `SandboxUnavailable { missing }` → the D-20 rows and `[U]`;
  - `ClaudeVersion { detail }` → **a refusal the UI-SPEC has no copy for**. Add one: `Launch refused: {detail}`, no `[U]` (both modes refuse it, `host.rs:152-173`).
- **Launch errors worth distinct copy** (`src/launcher/mod.rs:92-125`):
  - `Locked` / `DaemonExited { code: 3, .. }`: "already launched";
  - `DirectionInvalid { problems }`: lint, or control ≠ run (Pitfall 2);
  - `CapExhausted`;
  - `Auth` and `DaemonExited` with `detail` (the `launcher.log` tail): sanitise it, then cap it;
  - `ClaudeVersion`.

### Pattern 6: Dashboard marker = a pure classifier on the existing 20-tick reconcile task
- In the same `spawn_blocking` as `reconcile_all` + `last_ended_outcomes` (`src/app.rs:1292-1309`), call `list_driver_runs(&project.path, LAUNCHER_RUNS_LISTED)` per registered project. Reduce each with a pure `fn launcher_marker(entries, now) -> Option<LauncherMarker>`:
  - per-run first match: crashed / parked / stale / driven / unknown;
  - cross-run precedence: crashed > parked > driven > stale > unknown (UI-SPEC D-12 table).
- Ship the map in `RunsReconciled` (one more field) and store it in `ProjectViewCache` (Default, zero churn).
- A missing drivers dir is an empty list (`list_driver_runs` returns `Vec::new()` on `read_dir` error, `:828-830`), so unregistered or undriven projects cost one failed `read_dir`.
- **Watcher refresh for the marker:** on `ChangeKind::DriverRun` for any project, recompute that project's marker. Reuse the same pure classifier on the per-run refresh result, or schedule a one-project listing.
- **SIGKILL** releases the flock with no file event. The 20-tick rescan (about 5 s, `src/app.rs:1269-1272`) is what turns it into crashed.

### Pattern 7: LOG tail = `tail_lines` + identity check + bounded ring
- Per alias: `LogTail { run_id, cursor: TailCursor, dev_ino: Option<(u64,u64)>, ring: DriverOutput }`.
- On each refresh (`spawn_blocking`):
  1. `metadata(LOG.md)`. If `(dev, ino)` changed, reset `cursor` to 0 and clear the ring with the D-23 notice (replacement).
  2. Loop `tail_lines` until `cursor.offset == len`, bounded by a few iterations (each read is capped at 4 MiB).
  3. `restarted == true` → reset notice (truncation).
- `tail_lines` filters empty lines and leaves a torn last line unconsumed (`src/journal/reader.rs:179-218`).
- **Preamble:** UI-SPEC says to skip through the first `-->` on the first read. A sturdier equivalent `[inferred — audit]`: on a read from offset 0, drop lines until the first event header (`parse_log(line)` yields one). This is identical for the shipped template (`src/driver_templates/assets/LOG.md`), and nothing hostile can stretch it.
- **Park rule (D-16):** keep a `HashMap<item, LogEvent>` of the last `status` event with `state=parked` per item, updated from tail lines as they stream (`parse_log` on each header line). No second full read is needed.
- **Ring:** reuse `DriverOutput` (`src/ui/screens/mod.rs:531-561`). It carries `run_id` (CR-02: a ring must not speak for another run), caps at `DRIVER_OUTPUT_RING_LINES`, and stores `Untrusted` text that cannot be rendered unescaped.

### Pattern 8: Every new screen is adjudicated and probed
- `Screen: RenderAdjudicated` is a sealed supertrait. An unadjudicated `impl Screen` is `error[E0277]` (`src/ui/screens/mod.rs:204-263`). Use `adjudicate_screen!` with `RENDERS_ATTACKER_INFLUENCED_IDENTITY` for screens that echo a run id, item id, lint finding or answer.
- Each new screen also needs a row in `SCREEN_IDENTITY_DISPOSITIONS` and a `fixture_for` entry in `render_escape_guard.rs` (`:368-378` list; `:2838`/`:3056` panic "has no probe fixture"). Otherwise the census test fails.
- New launcher-run render sites inside `DetailScreen` get hostile states in `probe_ctx` (`render_escape_guard.rs:1085`, which already seeds `cache.launcher_runs` at `:1213`).

### Anti-Patterns to Avoid
- **Reading `status:` straight from the file** (27 hand-off 4). Always `DerivedStatus`.
- **Liveness from frontmatter** (`connected[].state == "connected"`, heartbeat freshness). Only the lock probe decides live.
- **Holding `DIRECTION.lock` across the editor session.** It starves the splice (2 s budget) and D-17 forbids it.
- **A second timer** for the marker scan or heartbeat redraw (`src/app.rs:1281-1287` forbids it in as many words).
- **Choosing colour or glyph from LOG text** (UI-SPEC Untrusted Input Boundary).
- **`Command::new` in a new `src/` file**: `tests/spawn_seam_guard.rs` fails any spawn site outside `SPAWN_ALLOWLIST` (`:45`), in-source tests included. The D-27 stub editor belongs in `tests/`, or should be simulated by writing the temp.
- **Binding the run socket or embedding `RunStore` in the TUI** (D-19).

## Don't Hand-Roll

| Problem | Don't Build | Use Instead | Why |
|---------|-------------|-------------|-----|
| Locked body write that never loses a human save | read-modify-write + rename | `driver::direction::write_body` | `renameat2(RENAME_EXCHANGE)` verify-and-give-back loop. A plain rename silently reverts a concurrent editor save (direction.rs module facts 2-3) |
| Comment-aware body parsing | regex over raw text | `blank_comments` + `section_heading` + `parse_direction` | A commented-out item or control line must be invisible exactly as it is to the store and the lint (WR-01) |
| Liveness | pid checks, heartbeat age | `derive_status` + `probe_at` | `LOCK_SH\|LOCK_NB` probe. `Unknown` is never `Free` |
| Byte-offset tail | `seek` + `read_to_string` | `journal::reader::tail_lines` | Torn lines, shrink, oversize and non-UTF-8 are already solved |
| LOG grammar | split on `|` | `driverctl::log::parse_log` / `LogEvent::get` | Single owner of the header format |
| Run id minting / collision | `format!` + `create_dir_all` | `driver_templates::create_run_dir` | Symlink/outside-project refusals, `EEXIST` → `Exists`, cleanup on partial write |
| Lint | ad-hoc checks | `lint_direction` | Same verdict the launch preflight enforces (`run.rs:240-245`) |
| Launch defaults / validators | constants in the TUI | clap `LaunchArgs` / `cli::positive_finite` | D-19: "never TUI copies" |
| Host capability check | `which bwrap` | `launcher::host::{detect, decide}` | Same decision the daemon makes. Includes the user manager + claude version |
| Terminal-safe rendering | `escape_default` | `shown_capped` / `render_for_terminal` / `Untrusted::shown` | Covers the control class + invisible-format class. The probe checks it |
| Width-safe truncation | `chars().take(n)` | `detail::fit_cells` | Display width (Item 2) |
| Scroll clamp / follow | new math | `clamp_scroll`, `tail_offset`, `follow_indicator`, `output_header_line` | UIFIX-04 ordering must not be re-derived (`detail.rs:422-430` doc) |

**Key insight:** every hard problem on this tab (race-free writes, honest liveness, hostile text) already has one owner in the tree, and two of those owners are guarded by tests. A tab-local version of any of them would be a second reader, the exact drift the project spent rounds closing.

## Runtime State Inventory

This phase re-hosts and retires surfaces but renames nothing. For completeness:

| Category | Items Found | Action Required |
|----------|-------------|------------------|
| Stored data | Legacy Phase 17/18 journal runs under `.planning/meta-manager/runs/` stay on disk and are shown read-only (D-01). `inbox.jsonl` files from Phase 18 injection stay; no key can append to them (D-02) | None (Phase 31 deletes the code, not user data) |
| Live service config | None. No external services involved | None |
| OS-registered state | Detached launcher daemons and their systemd user scopes (`systemd-run --user --scope`) outlive the TUI by design (D-19) | None. `x` kill reuses `stop_run`. Nothing to migrate |
| Secrets/env vars | `GSDMM_EXPERIMENTAL_FEATURES` gate unchanged (D-03). `VISUAL`/`EDITOR` read unchanged (Item 1) | None |
| Build artifacts | None | None |

## Common Pitfalls

### Pitfall 1: `write_body` proceeds **without** the lock after 2 s. The UI-SPEC test expects an error
**What goes wrong:** The UI-SPEC row "Steering writes … waiting on the DIRECTION.md lock" says a test "holds the lock past the timeout and asserts the error copy and an unchanged body". The code does the opposite: on `LockError::HeldBy { .. } | LockError::HeldByUnknownRun` it continues with `(None, true)`, writes through the exchange-verify loop, and returns `SpliceReport { lock_timed_out: true, .. }` (`src/driver/direction.rs:544-555`) `[VERIFIED: :532-556]`.
**Why:** 27 D-20. A sandboxed process could once hold the lock. Today the lock files are `/dev/null`-masked inside bwrap (`sandbox.rs:295-299`), so a busy lock means another gsd-mm process: the driverctl splice, another TUI, or an unsandboxed driver.
**How to avoid:** Do not change `write_body` (Phase 27 behaviour, out of scope). The tab:
- treats `lock_timed_out: true` with `SpliceOutcome::Ok` as **written**, with an extra note: "written without DIRECTION.lock (held by another writer); the exchange check kept every save" `[inferred — audit]`;
- corrects the backstop test to assert that, with the lock held by a test thread beyond the budget, the write lands once, the frontmatter block is byte-identical, and `lock_timed_out == true`.

SC3's "every edit through the shared flock" is met in the sense that every edit **goes through the flock-taking primitive**. Record this nuance in VERIFICATION.
**Warning signs:** a test that `sleep`s past 2 s and expects `Err`.

### Pitfall 2: Launching or relaunching a run whose body says `pause`/`stop` is refused, not warned
**What goes wrong:** `preflight_direction` returns `DirectionInvalid` "control is {word}; set - control: run to launch" whenever `control_line` is not `run` (`src/launcher/run.rs:246-253`) `[VERIFIED: :225-256]`. The UI-SPEC treats this as a Yellow advisory row with `y` still live.
**How to avoid:** The launch form must **block `y`** while the live body `control != run`, and offer `[c]` (or an inline "set control: run and launch"). Relaunching a `paused` run (UI-SPEC adds `paused` to `R`) always needs this. Mark the copy `[inferred — audit]`.

### Pitfall 3: The spend line overstates the ceiling
**What goes wrong:** `--max-budget-usd` goes into **each** `claude` session the launcher starts (`claude_argv(&prompt, &ctx.opts.model, ctx.opts.max_budget_usd)`, `src/launcher/run.rs:1341`) `[VERIFIED: :1325-1352]`. The launcher starts a new session in-process on every `fresh-exit` (fresh-per-item) and on auth relaunches (`run.rs:20-30`) `[VERIFIED: :14-47]`. So "This run may spend up to $10.00" is true only for `loop` mode with no auth relaunch.
**How to avoid** (D-28 honest vocabulary), `[inferred — audit]`:
- Copy: "Each driver session may spend up to ${budget} (claude --max-budget-usd).";
- when `loop_mode == fresh-per-item`, add: "fresh-per-item starts one session per item, so the run can spend a multiple of this."

Whether worker subagent spend counts toward the parent session's `--max-budget-usd` is `[ASSUMED]` (yes) and unverified here.

### Pitfall 4: An answer containing `<!--` blanks the rest of DIRECTION.md
**What goes wrong:** `blank_comments` treats an unterminated `<!--` as running to end of text (`direction.rs:176-200`: "An unterminated comment runs to the end"). An answer `- answer: see <!-- old note` therefore hides every later item from `parse_direction`, the lint and the gate.
**How to avoid:** The answer validator rejects:
- control characters, including CR/LF/TAB-free rules per D-15;
- U+2028/U+2029;
- `<!--` and `-->`;
- more than `DIRECTION_ANSWER_MAX_CHARS` (500) chars.

Trim it. Reuse the predicate shape of `driverctl::log::one_line` (`src/driverctl/log.rs:159-172`), but **allow `|`**, which is harmless in the body. Post-write, assert `steer::read_items(new)` still lists every id the old body listed.
**Warning signs:** item count drops after an answer write.

### Pitfall 5: `launch_detached` in a test re-execs the test harness and "succeeds"
**What goes wrong:** `respawn_detached` runs `std::env::current_exe()` (`src/launcher/process.rs:101-111`) `[VERIFIED]`. In `cargo test` that is the test binary. It receives `driver launch …` as test-name filters, runs zero tests and exits 0, and `launch_detached` treats exit 0 as "the whole run fit inside one poll, which is a success" (`run.rs:1288-1290`). Fake tools on `PATH` would also need process-global `set_var`, which is racy in a multi-threaded harness.
**How to avoid:**
- an injectable `launch_fn` seam on the app (Pattern 5) for App-level tests;
- a pure golden test that the TUI's form produces the same `LaunchOptions` and `launch_argv` as `driver launch` would for the same flags (`launch_argv` is pub, `run.rs:1134-1167`);
- for the zero-spend e2e (D-26): a `tests/` integration test using `tests/launcher_support::Fixture` (fake `claude`/`bwrap`/`systemd-run` first on the **child's** `PATH`, `CARGO_BIN_EXE_gsd-meta-manager`) that relaunches a fixture run left `running` with a free lock. That is the crash case D-21 targets. Run it through the real binary's `driver launch`, with the exact argv from the TUI builder.

Record in VERIFICATION that the in-process `launch_detached` call itself is covered by the seam, not by e2e.

### Pitfall 6: Drivers-dir writes currently force a full project re-parse
**What goes wrong:** `classify_change` only recognises `.planning/meta-manager/runs/<id>/…` (`src/journal/mod.rs:1146-1154`) `[VERIFIED: :1088-1157]`. Everything under `drivers/` falls to `Planning` and runs `schedule_reparse` (500 ms dedup, `src/app.rs:1422-1444`). Every event of a live run splices the frontmatter (temp create + exchange + `state.json` rewrite), so this is the OBS-06 regression D-24 targets.
**How to avoid:**
- Add `ChangeKind::DriverRun { run_id }`. Route only `drivers/<id>/DIRECTION.md` and `drivers/<id>/LOG.md` there, plus `drivers/<id>` itself (create/delete → list refresh).
- Add `ChangeKind::Ignored` for every other path under `drivers/`: `state.json`, `.DIRECTION.md.fm.*`, the edit temp, `*.lock`, editor swap files such as `..DIRECTION.edit.md.swp`.
- **Use an allowlist, not a denylist.** The dedup key is `(root, kind)` (`src/watcher.rs:59-77`), so one batch carrying LOG.md then DIRECTION.md for the same run yields **one** action naming only the first path. The handler must refresh frontmatter **and** tail LOG regardless of which path it names.
- `ChangeKind` derives `Hash` and is half the dedup key, so keep `run_id` in it.

### Pitfall 7: GUI editors that return immediately
**What goes wrong:** `code file` without `--wait` returns at once. The apply step sees unchanged text, removes the temp, and the user keeps typing into a deleted file. `EDITOR="code --wait"` cannot fix it, because the app does no word-splitting (Item 1).
**How to avoid:** Keep the UI-SPEC behaviour. Add the limitation to the help text or status copy ("set VISUAL to a terminal editor or a blocking wrapper script") `[inferred — audit]`. Do not change `editor_args` in this phase.

### Pitfall 8: Sanitise before truncating
Covered in Item 2. `display_identity` expands invisible characters into 6+ cell `U+XXXX`, so truncating first produces rows wider than planned. The order is `fit_cells(render_for_terminal(x).as_ref(), room)`.

### Pitfall 9: The editor exit status is lost in the binary
`src/main.rs:851-873` converts the exit status into a status message and then calls `refresh_after_editor(&path)` with no status `[VERIFIED: :833-886]`. D-17 (and the UI-SPEC "editor exited non-zero" row) need the status in the library. Change the call to pass it. Keep the I/O out of the `async fn run_tui_loop` body: the async-blocking guard allowlists only `.status()` there.

### Pitfall 10: The heartbeat source
- The `driver` connection is created with `last_heartbeat: None` (`src/driverctl/store.rs:951-961`), and every hook refreshes it (`store.rs:1118-1121`) `[VERIFIED: :1099-1121]`.
- The `launcher` connection's `last_heartbeat` is its start time and never advances.
- So the D-08 age must use the `role == "driver"` entry. Fall back to the newest only when no driver entry exists, and render the fallback as "(launcher start)" so a frozen launcher stamp is never read as a heartbeat `[inferred — audit]`.
- Frontmatter regeneration on heartbeat-only events is throttled to `HEARTBEAT_REGEN_MIN_S: i64 = 120` (`store.rs:71-72`) `[VERIFIED]`. The displayed age therefore saw-tooths up to 120 s, with the default 300 s stale threshold.
- One long worker tool call (e.g. a 10-minute `cargo test`) can show `stale` on a healthy run. That is exactly why D-08 never calls it crashed.

### Pitfall 11: Parse the heartbeat timestamp tolerantly
Use `driverctl::log::epoch_seconds` (RFC 3339, `None` on error, `src/driverctl/log.rs:55-60`) or `chrono::DateTime::parse_from_rfc3339`. A future stamp renders `0:00` (UI-SPEC). An unparseable one renders `(none recorded)`, never a panic.

### Pitfall 12: Two different "needs human" predicates
The UI-SPEC splits them:
- **badge rank 4** is the legacy predicate alone;
- **filter `h` / AttentionFirst** is legacy **or** marker ∈ {crashed, parked, stale}.

Today one function feeds both (`needs_human_for`, `src/ui/screens/mod.rs:1803-1811`, consumed at `normal.rs:295` for the badge and `mod.rs:2037, 2109` for sort/filter) `[VERIFIED]`. Add a second method and route each consumer explicitly. Gate the marker on `ctx.experimental`, like `row_badge` gates `driven_and_live` (`normal.rs:276-280`) and `attention_rank_for` does (`mod.rs:2031-2037`). Re-run `recompute_filtered_aliases` when markers change.

### Pitfall 13: macOS compile hygiene is unchecked by CI
These are all `#[cfg(target_os = "linux")]`:
- `launcher::{run, stop, host, …}` (`src/launcher/mod.rs:67-84`);
- `driver_templates::{create_run_dir, mint_run_id}` (`mod.rs:50-51`);
- `App::stop_launcher_run` (`app.rs:985`).

`derive_status` is `#[cfg(unix)]`. CI builds only on ubuntu. Every new Action arm that calls these needs a cfg'd body plus a non-Linux refusal (the steering keys are hidden there per D-14). Add a `cargo check --target aarch64-apple-darwin` step to the phase gate if that target is installed. It is **not** installed here: `rustup target list --installed` shows only `x86_64-unknown-linux-gnu`. Otherwise review the cfg pairs by hand.

### Pitfall 14: The legacy WR-05 blank row at heights 8-13
D-29: the collapsed legacy detail must apply the same no-orphan rule. The sweep must select a legacy row at each height, not only launcher rows.

## Code Examples

### Run-list scan on the slow tick (shape to extend)
```rust
// Source: src/app.rs:1292-1309 (verbatim shape; add the launcher scan inside this same closure)
tokio::task::spawn_blocking(move || {
    let runs = crate::driver::reconcile::reconcile_all(&projects);
    let last_outcomes = crate::driver::reconcile::last_ended_outcomes(&projects);
    let _ = tx.send(Action::RunsReconciled { runs, last_outcomes });
});
```

### Watcher classification extension
```rust
// Source: src/journal/mod.rs:1125-1157 (existing); new arm sketched [inferred — audit]
if parts.len() >= 4 && parts[0] == ".planning" && parts[1] == "meta-manager" && parts[2] == "drivers" {
    let run_id = parts[3].to_string();
    return match parts.get(4).copied() {
        None => ChangeKind::DriverRun { run_id },                       // run dir created/removed
        Some("DIRECTION.md") | Some("LOG.md") if parts.len() == 5 => ChangeKind::DriverRun { run_id },
        _ => ChangeKind::Ignored,                                       // state.json, .DIRECTION.md.fm.*, temps, locks, swap files
    };
}
```

### Display-width title cut (Item 2)
```rust
// Source: src/ui/screens/detail.rs:9619-9627 pattern + :10083 fit_cells
let title = crate::text::render_for_terminal(&item.title).to_string(); // sanitise FIRST
let used: usize = spans.iter().map(|s| s.width()).sum();
let room = inner_width.saturating_sub(used);
if room > 1 {
    spans.push(Span::raw(" "));
    spans.push(Span::raw(super::detail::fit_cells(&title, room - 1)));
}
```

### Editor resolution (unchanged; Item 1)
```rust
// Source: src/main.rs:841-844
let editor = std::env::var("VISUAL")
    .or_else(|_| std::env::var("EDITOR"))
    .unwrap_or_else(|_| "vi".to_string());
```

## Test Seams (D-26 / D-27)

`workflow.nyquist_validation` is `false` in `.planning/config.json`, so the formal Validation Architecture section is omitted. These are the seams the D-26/D-27 acceptance needs:
- **Pure:**
  - `steer` transforms against `DIRECTION_TEMPLATE` (`driver_templates::DIRECTION_TEMPLATE`) and its run-id-filled form;
  - `classify_change` path table;
  - marker classifier truth table;
  - heartbeat-age formatter;
  - status-token builder;
  - footer drop order;
  - `fit_cells` room math.
- **Fixture run dirs:**
  - `DIRECTION.md` text built with `DriverFrontmatter::emit()` (`driver_frontmatter.rs:522`) + template body;
  - lock held by `acquire_at` on a test thread vs. free vs. absent;
  - the `tests/driverctl_liveness.rs` shape.
- **TestBackend:** H 1..=40 × W 40/60/80/120 for launcher and legacy selections (D-29).
- **Editor:** call the library apply function directly with a temp the test rewrites, which simulates the editor without a spawn (see the spawn-guard anti-pattern). If a real stub process is wanted, put it in `tests/`. Race case: change the live body between prepare and apply and assert no clobber plus the temp kept.
- **Lock timeout:** Pitfall 1 corrected assertion.
- **Launch:** Pitfall 5.
- **Commands:**
  - quick: `cargo test --lib driver_run steer`;
  - full: `cargo test --no-fail-fast` (git-version witness failure expected locally);
  - clippy: `cargo clippy --all-targets -- -D warnings`.
- No `cargo-nextest` is installed (probe below).

## State of the Art

| Old Approach | Current Approach | When Changed | Impact |
|--------------|------------------|--------------|--------|
| Steering via stream-json injection (`i`, `inbox.jsonl`) | Edit DIRECTION.md body under `DIRECTION.lock` | Phase 27/29 (this phase switches the tab) | `i` shows the retirement copy. Phase 31 deletes the code |
| Liveness from pid/cmdline (`ObservedRun::is_live`) | `launcher.lock` flock probe via `DerivedStatus` | Phase 27 D-22 | Legacy runs keep the old badge source (rank 3/4) |
| 28-07 bottom "Launcher runs" section, `{`/`}`/`X` | Run list is launcher runs. `x` kills the selected run | This phase | Delete `render_launcher_runs`, `launcher_section_height`, `LAUNCHER_LEGACY_MIN_HEIGHT` |
| Sibling run dirs visible inside the sandbox | Drivers dir tmpfs-masked, own run dir ro-bound | quick 261020-f80 (018cc17) | D-09's rationale is now narrower: forgery remains possible from `--unsandboxed` runs and through a run's own driverctl-written fields. The display-data-only rule still holds |

## Assumptions Log

| # | Claim | Section | Risk if Wrong |
|---|-------|---------|---------------|
| A1 | Worker-subagent spend counts toward the parent session's `--max-budget-usd` | Pitfall 3 | Spend copy understates further. The copy would need "excluding workers" |
| A2 | `.DIRECTION.edit.md` (vs the approved `.DIRECTION.md.edit`) gives editors Markdown highlighting | Pattern 4 | Cosmetic only |
| A3 | A stale edit temp from a crashed TUI should open the conflict chooser | Pattern 4 | A worse UX if the operator expects silent discard |
| A4 | Heartbeat fallback when no driver connection exists should be labelled "(launcher start)" | Pitfall 10 | Mislabelled age |
| A5 | A `tests/` e2e through the real binary plus a seam satisfies D-26's "launch and relaunch test" | Pitfall 5 | Verifier may read D-26 as requiring the TUI code path end to end |
| A6 | Re-using `LAUNCHER_RUNS_LISTED` (20) per project for the dashboard scan is cheap enough every ~5 s | Pattern 6 | Many projects × many runs could add measurable I/O. It is bounded and tunable |

## Open Questions

1. **Does SC3's "every edit takes the shared flock" accept `write_body`'s proceed-without-lock fallback?**
   - Known: the fallback is Phase 27 design (27 D-20) and the exchange-verify still prevents loss.
   - Unclear: whether the verifier will read the timeout path as a violation.
   - Recommendation: accept it, surface `lock_timed_out` to the user, and record the reasoning in VERIFICATION. Do not change `write_body` here.
2. **Should `R` on a `paused` run auto-offer "set control: run, then launch"?**
   - Known: the preflight refuses otherwise (Pitfall 2).
   - Recommendation: a single extra key in the launch form (`[c]` jumps to the chooser). No automatic write.
3. **Relaunch prefill for `loop_mode`.**
   - The `driver` connection carries `mode: Some(driver.mode.clone())` (`store.rs:955`), but this research did not confirm that the value is the loop-mode word. Recommendation: map it only if it equals `"loop"`/`"fresh-per-item"`, else use the default with "(default)".

## Environment Availability

| Dependency | Required By | Available | Version | Fallback |
|------------|------------|-----------|---------|----------|
| rustc / cargo | build, tests | ✓ | 1.99.1 / 1.99.1 (floor 1.89, `Cargo.toml:15`) | — |
| bwrap | real sandboxed launch (not needed by tests) | ✓ | bubblewrap 0.11.1 | Tests use fake `bwrap` on `PATH` |
| systemd-run / user manager | real launch | ✓ / `degraded` (counts as usable, `parse_user_manager`) | — | fakes in `tests/launcher_support` |
| vi | editor fallback | ✓ `/usr/bin/vi` | — | — |
| `$EDITOR` / `$VISUAL` | editor | `EDITOR=vim`, `VISUAL` unset | — | `vi` |
| cargo-nextest | faster tests | ✗ | — | `cargo test --no-fail-fast` |
| macOS target (`aarch64-apple-darwin`) | cfg hygiene check | ✗ (only `x86_64-unknown-linux-gnu`) | — | Manual cfg review (Pitfall 13) |
| `claude` | none (no spend, D-26) | not probed | — | fake `claude` in tests |

**Missing dependencies with no fallback:** none.
**Missing with fallback:** nextest (use cargo test), macOS target (manual review).

## Security Domain

`security_enforcement` is absent from `.planning/config.json`, so it is treated as enabled.

### Applicable ASVS Categories
| ASVS Category | Applies | Standard Control |
|---------------|---------|-----------------|
| V2 Authentication | no | Local TUI. Launcher auth is Phase 28's |
| V3 Session Management | no | — |
| V4 Access Control | partial | Experimental gate (`ctx.experimental`, D-03). Steering keys hidden on non-Linux (D-14). Legacy rows read-only |
| V5 Input Validation | **yes** | Answer validator (Pitfall 4). Slug via `SLUG_RE` + `mint_run_id`. Launch numbers via clap/`positive_finite`. Edited body via `body_sane` + `lint_direction`. Run ids from disk via `PLAIN_RUN_ID_RE` (the listing) |
| V6 Cryptography | no | — |
| V12 Files/Resources | **yes** | Edit temp `create_new` + mode 0600. `symlink_metadata` before reading it back, plus a size cap. Run dirs listed without following symlinks (`list_driver_runs`) |
| V14 Config | partial | `--config` path passed to the daemon so it resolves the same registry |

### Known Threat Patterns for this stack
| Pattern | STRIDE | Standard Mitigation |
|---------|--------|---------------------|
| Terminal escape / bidi injection via frontmatter, LOG, titles, lint text (T-28-19) | Tampering / Spoofing | `shown_capped` / `render_for_terminal` / `Untrusted`, plus render-escape-guard fixtures for every new site and screen |
| Forged frontmatter claiming `running` / healthy | Spoofing | Liveness only from `derive_status` lock probe (D-09). Frontmatter is display data only |
| Comment injection through an answer (`<!--`) hides items | Tampering | Reject `<!--`/`-->` in answers. Post-write item-count check (Pitfall 4) |
| Planted symlink at the edit temp path | Tampering | `O_EXCL` create fails on an existing path. Verify a regular file before apply |
| Edit applied over a concurrent human save | Tampering (data loss) | Base compare inside the `write_body` closure (Pattern 4) |
| Unsandboxed launch by accident | Elevation of privilege | Separate second confirm, default No, never remembered (D-20) |
| Unbounded spend | Denial of wallet | Explicit form fields with clap validation. Honest per-session ceiling copy (Pitfall 3) |
| Kill aimed at the wrong run | Tampering | The run id travels from the confirmation that named it (T-28-18, `src/action.rs:376-387`) |

## Sources

### Primary (HIGH: files opened this session)
- `src/main.rs:780-893`. Editor handoff and resolution order.
- `src/app.rs:300-380, 500-580, 900-1028, 1235-1345, 1388-1477, 2590-2632`. Editor hook, rescan gate, tick tasks, FileChanged routing, ScreenAction handling.
- `src/ui/screens/detail.rs:405-430, 770-810, 5380-5570, 9605-9634, 10070-10103`. Truncation helpers, `e` arm, driver keys.
- `src/ui/screens/driver.rs:600-830`. `shown_capped`, `ellipsize`, panes.
- `src/ui/screens/mod.rs:380-410, 700-760, 1100-1220, 1600-2000`. Constants, sanitiser, view cache, `needs_human`, run-list scan.
- `src/ui/screens/normal.rs:85-284`. Badge ranks.
- `src/state_reader/driver_frontmatter.rs:60-846`. Reader, enums, derive, listing.
- `src/driver/direction.rs:1-673`. Split, parse, `write_body`, lock-timeout semantics.
- `src/driver/lock.rs:360-383`. `probe_at`.
- `src/driverctl/{mod.rs, log.rs:1-175, store.rs:60-79, 1095-1135}`. Hand-offs, LOG grammar, heartbeat.
- `src/journal/{mod.rs:1088-1157, reader.rs:99-219}`. ChangeKind, `tail_lines`.
- `src/driver_templates/{mod.rs:25-280, lint.rs:1-215}`, plus the `assets/DIRECTION.md` and `assets/LOG.md` templates.
- `src/launcher/{run.rs:14-53, 95-205, 225-256, 570-599, 745-774, 970-997, 1125-1294, 1325-1352; mod.rs:86-205; host.rs:28-187; sandbox.rs:1-40, 255-315; process.rs:90-114}`.
- `src/cli.rs:1-50, 510-575`. `LaunchArgs` defaults and validators.
- `src/driver_cli.rs:200-400`. How the CLI builds `LaunchOptions`.
- `src/watcher.rs` (whole). Dedup key.
- `src/action.rs:330-395`. Launcher actions.
- `tests/async_blocking_guard.rs` (header + marker lists) and `tests/spawn_seam_guard.rs:40-118`. Guards.
- `tests/launcher_e2e.rs` header and `tests/launcher_support/mod.rs` symbols. Fake-tool e2e pattern.
- `man git-var` (local). Git editor precedence.

### Secondary / Tertiary
- None. No web sources were needed: the phase is fully in-repo.

## Metadata

**Confidence breakdown:**
- Standard stack: HIGH. Every API was read at its definition.
- Architecture: MEDIUM-HIGH. The seams are verified. The module split is discretionary.
- Pitfalls: HIGH for 1, 2, 3, 5, 6, 9, 10, 12 (each traced to code). MEDIUM for 7 and 13.

**Research date:** 2026-10-20
**Valid until:** 2026-11-19 (in-repo APIs, stable unless Phases 27-29 change). Re-check `src/launcher/` after the parallel `auth.rs` edit lands.
