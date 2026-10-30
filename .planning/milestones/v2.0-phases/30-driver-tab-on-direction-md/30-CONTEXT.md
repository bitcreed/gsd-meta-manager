# Phase 30: Driver Tab on DIRECTION.md - Context

**Gathered:** 2026-10-20
**Status:** Ready for planning
**Mode:** unattended (operator unavailable; equivalent to `--auto`, no auto-advance). Every decision not taken
verbatim from ROADMAP / REQUIREMENTS / the Phase 27-29 hand-offs is tagged `[inferred — audit]`.

<domain>
## Phase Boundary

Rebuild the TUI's Driver tab on the run files that Phases 27-29 produce. The tab observes a run only through
`<project>/.planning/meta-manager/drivers/<run-id>/` (the tool-written DIRECTION.md frontmatter, the
DIRECTION body, LOG.md and the run lock). It steers a run only by editing the DIRECTION body under the shared
`DIRECTION.lock`, and it starts, relaunches and stops runs through the existing launcher library.

In scope: DTAB-01..04.
- Run list per project, and a selected-run view: derived status, per-item state, caps used/limit, dispatch audit,
  heartbeat age, `connected[]`. Everything comes from the frontmatter plus the lock probe, with no Claude call.
- Crashed and stale-heartbeat flags, and dashboard markers for driven, parked and crashed projects, reusing
  OBS-02 and OBS-07.
- Steering: set `control: run|pause|stop`, answer or un-hold a parked item, and edit DIRECTION in `$EDITOR`.
- Lifecycle: start a new run (mint, edit, check, launch), relaunch a crashed run, tail LOG.md live.
- Retire the Phase 18 stream-json injection UI as the steering path.

Out of scope, owned elsewhere:
- Deleting the Phase 17/18/19/21 code (`drive`, inbox injection, envelope goal layer): Phase 31. Phase 30 only
  takes those paths off the tab.
- Launcher, store, prompt or template behaviour changes. The tab consumes Phases 27/28/29 unchanged. The
  master-prompt 1.1 fixes are backlog 999.7.
- Phase 28's pending operator UAT (browser login, macOS run, risk acceptance). Not touched here.
- macOS steering. `write_body` is Linux-only (27 D-12), so the tab is read-only there (D-14).
</domain>

<decisions>
## Implementation Decisions

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

### Claude's Discretion
- Module layout: whether the new view lives in `src/ui/screens/driver.rs` (already ~4.9k lines) or a new
  `driver_run.rs` sibling, and where the pure body transforms live (`driver::direction` versus a new
  `driver::steer`).
- Exact keybindings, subject to D-28's UI-SPEC and no clash with existing global keys.
- How many runs the dashboard scan reads per project (bounded; `list_driver_runs` already takes a `limit`).
- Plan split and waves. The reader/view, the steering transforms, the watcher routing and the lifecycle/launch
  flow are largely independent. The dashboard marker depends on the reader.

### Folded Todos
- **D-29:** "Driver tab renders blank pipeline row at terminal heights 8-13" (score 0.9, ui; Phase 18 WR-05). The
  tab is being re-laid out, so the new height tiers must not render a header or rule without its content at any
  height (D-10). A `TestBackend` sweep over heights 1-40 is the check. The legacy pipeline row only survives
  inside the collapsed legacy section, which obeys the same rule. `[inferred — audit]`
</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements & roadmap
- `.planning/ROADMAP.md` §"Phase 30: Driver Tab on DIRECTION.md" — goal, SC1-4
- `.planning/ROADMAP.md` §"Phase 31" — what Phase 30 hands over (injection path removal)
- `.planning/REQUIREMENTS.md` §"Driver Tab (DTAB): Phase 30" (DTAB-01..04); OBS-02, OBS-06, OBS-07
- `.planning/ROADMAP.md` §"Phase 999.7" — master prompt 1.1 backlog (not in scope; do not change the prompt)

### Phase 27 (reader, lock, splice) — the tab's only data path
- `.planning/phases/27-driverctl-driver-frontmatter/27-CONTEXT.md` — D-13 shared DIRECTION write, D-19
  `item_states`, D-20 lock files, D-22 liveness, D-24 schema v1 keys
- `src/driverctl/mod.rs` §"Phase 30 hand-off" — the four rules (write_body, body `control`, watcher filter,
  derived status)
- `.planning/phases/27-driverctl-driver-frontmatter/27-01-SUMMARY.md` — watcher must ignore `.DIRECTION.md.fm.*`
- `.planning/phases/27-driverctl-driver-frontmatter/27-REVIEW.md` — sandbox-originated LOG fields carry escapes

### Phase 28 (launcher, stop, posture)
- `.planning/phases/28-sandboxed-driver-launcher/28-CONTEXT.md` — D-04 detached launcher, D-09/D-11 stop, D-28/D-29
  degraded mode, D-30 `sandbox` key
- `.planning/phases/28-sandboxed-driver-launcher/28-07-SUMMARY.md` — current Driver-tab launcher section to re-host
- `.planning/phases/28-sandboxed-driver-launcher/28-VERIFICATION.md` — CR-01 note (bind in multithreaded TUI)
- `.planning/phases/28-sandboxed-driver-launcher/28-REVIEW-FIX.md` — CR-01 fix (0951aea)
- `.planning/phases/28-sandboxed-driver-launcher/28-SECURITY.md` — T-28-19 render escapes; follow-up: sibling run
  dirs writable from the sandbox

### Phase 29 (templates, check, control semantics)
- `.planning/phases/29-master-prompt-run-templates/29-CONTEXT.md` — D-08 mint, D-10/D-11 check lint, D-12/D-13
  stop-rule precedence and OQ-R1, D-18 control honoured before next dispatch
- `src/driver_templates/assets/DIRECTION.md` — the body grammar the steering transforms edit (`control`, `hold`,
  `answer`, `budget`)

### Phase 18 (the surface being replaced)
- `.planning/phases/18-driver-tab-live-watch-durable-injection/18-UI-SPEC.md` — height tiers and honest-vocabulary rules the new tab inherits
- `.planning/todos/pending/2026-07-29-driver-tab-layout-at-medium-terminal-heights.md` — folded (D-29)

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `state_reader::driver_frontmatter`: `read_driver_run`, `DerivedStatus`, `list_driver_runs`, `DriverRunEntry`,
  `DriverFrontmatter` (caps, `dispatch_audit`, `connected`, `item_states`, `heartbeat_stale_after_s`, `sandbox`,
  `prompt_version`).
- `driver::direction`: `write_body`, `parse_direction` (live `control`, items), `control_hold`, `body_sane`,
  `blank_comments`, `split_frontmatter`, `TEMP_PREFIX` / `sweep_stale_temps`.
- `driver_templates`: `mint_run_id`, the `driver new` creation path, and `lint` (the `driver check` findings,
  already passed through `render_for_terminal`).
- `launcher`: `launch_detached` (`run.rs`), `stop_run` (`stop.rs`), host detection (`host.rs`); `LaunchArgs`
  defaults in `src/cli.rs:533`.
- UI: `sanitize_render_line`, `shown_capped`, the render-escape guard (`src/ui/screens/render_escape_guard.rs`),
  `DRIVER_OUTPUT_RING_LINES` and the output-pane follow/scroll helpers, the `LauncherStopRequested` /
  `LauncherStopped` actions, the confirm-popup screens, and the `needs_human` / AttentionFirst sort
  (`src/ui/screens/mod.rs:1654`).

### Established Patterns
- `spawn_blocking` → `Action`; no file I/O on the render thread.
- Typed refusals; Linux-only behaviour behind `#[cfg]` with a typed unsupported error.
- Pure transforms plus golden/`TestBackend` tests; live checks opt-in `#[ignore]`.
- Run `cargo test --no-fail-fast`; the local git-version witness failure is expected.

### Integration Points
- `src/ui/screens/driver.rs`: re-layout; retire the injection and legacy-start entry points from the keymap.
- `src/app.rs`: new actions (refresh run, steer write, editor handoff, new/relaunch launch, LOG tail).
- `src/watcher.rs`: route `drivers/` paths to the run refresh, filtering `state.json`, `.DIRECTION.md.fm.*` and the
  edit temp.
- `src/ui/screens/mod.rs`: dashboard marker and `needs_human` extension.
- `src/ui/screens/help.rs`: keymap text.
</code_context>

<specifics>
## Specific Ideas

- The honest-display rule from Phase 18 (D-13 "Replit rule") applies unchanged. Status, liveness and caps come
  from tool-written frontmatter and the lock, never from the driver's prose. The driver's words appear only as
  content in the LOG tail.
- `control: pause` written by the tab is shown as pending until the frontmatter's `control` echoes it. The driver
  honours it before its next dispatch, not instantly.
</specifics>

<deferred>
## Deferred Ideas

- Deleting the injection, legacy `drive` start and journal-run code: Phase 31.
- Master prompt 1.1 (reconcile-first refusal routing, `$RUN_DIR` substitution): backlog 999.7.
- Masking sibling run dirs from the sandbox (28-SECURITY follow-up): a launcher hardening item, not tab scope.
  `[inferred — audit]`
- A macOS `renamex_np` splice, which would enable steering on macOS: future work (27 deferred).
- Editing caps of a *running* run: caps change only on relaunch (D-21).

### Reviewed Todos (not folded)
`[inferred — audit]` This departs from the auto rule (fold every todo scoring at least 0.4); each of these sits
outside the tab boundary.
- "The verify-work gate policy must be configurable" (0.9): already folded into the Phase 29 template (29 D-16).
- "Codex runtime: remainder after the MVP slice" (0.7): not the Claude launcher.
- "Dashboard renders quoted VERIFICATION status as Unknown…" (0.6): a dashboard-reader fix (27 D-27), suited to
  a quick task. The D-12 marker does not read VERIFICATION status.
- "Make queue input truly multi-line" (0.3): queue UI, unrelated.
</deferred>

---

*Phase: 30-driver-tab-on-direction-md*
*Context gathered: 2026-10-20*
