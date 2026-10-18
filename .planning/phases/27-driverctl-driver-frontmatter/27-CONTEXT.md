# Phase 27: driverctl & Driver Frontmatter - Context

**Gathered:** 2026-10-18
**Status:** Ready for planning
**Mode:** `--auto` (operator unavailable). Every decision not taken verbatim from ROADMAP / REQUIREMENTS /
spike evidence is tagged `[inferred — audit]`.

<domain>
## Phase Boundary

Port spike 002's `tools/driverctl.py` to a Rust subcommand, `gsd-meta-manager driverctl serve|log|hook|show`.
It becomes the **single writer** of a driver run's `state.json`, `LOG.md` and the tool-written `---` frontmatter
at the top of `DIRECTION.md`. The phase also adds the typed reader (`DriverFrontmatter` in `state_reader`, with
flock-probe liveness) that Phase 30's Driver tab will consume.

In scope: DCTL-01..04. That covers the socket server and client, the exchange-verify splice, the frontmatter
schema v1 with its reader, tool-observed counters (dispatches from hooks, coalesced gap rounds from VERIFICATION.md
on disk), the 120 s heartbeat coalescing, rc=2 rejection, and the editor-race test port.

Out of scope, owned by later phases:
- the launcher (bwrap, systemd scope, the run flock acquisition, cap gate/trip/kill, exit classification call
  site): Phase 28;
- MASTER-PROMPT and run templates: Phase 29;
- the Driver tab UI and steering edits: Phase 30.
</domain>

<decisions>
## Implementation Decisions

### Protocol & CLI compatibility (DCTL-01, SC1)
- **D-01:** The CLI surface and wire protocol are those of spike 002 `driverctl.py`, kept unchanged:
  - verbs `log <event> …`, `show`, `hook` and `serve`, with the event flags listed in the driverctl.py docstring;
  - one JSON request per connection over a unix socket, the path taken from `DRIVERCTL_SOCK`, created mode 0600;
  - the server unlinks a stale socket at start.

  The spike-002 MASTER-PROMPT must run against the Rust binary with no prompt edits. The acceptance check is a
  scripted replay of a spike-002/003 LOG through the Rust client. — **Reversibility:** costly — the MASTER-PROMPT
  (Phase 29) and the hook settings seeded by the Phase 28 launcher both encode this CLI.
- **D-02:** `driverctl` is a regular clap subcommand of the existing binary, not a second binary (same rule as
  Phase 17 D-01: one build, one version). `[inferred — audit]` Hide it from `--help` (`#[command(hide = true)]`),
  because users never invoke it directly.
- **D-03:** `serve` runs outside the sandbox, on the launcher side. In Phase 27 it is a standalone entry point
  (`serve --run-dir … --project … --sock …`) so it can be tested without the launcher. Phase 28 embeds or spawns it.
  `[inferred — audit]`: Phase 28 is free to call the server as a library (`RunStore`-equivalent) from the launcher
  process, as `launcher_s003.py` did with `dc.RunStore`. Phase 27 therefore exposes the store as a library type
  as well as the subcommand.
- **D-04:** Malformed requests, unknown events, unknown items and enum violations are rejected with **rc=2**, and
  nothing is written for them. The client propagates the server's rc. — from DCTL-04 / SC4.

### Status enum & 26.1 follow-up (DCTL-03, SC3)
- **D-05:** The run status enum is spike 002's `RUN_STATUS_ENUM` plus **`stalled`**:
  `starting | running | paused | parked | stopped | done | crashed | stalled`.
  - `stalled` means the headless driver exited cleanly (rc 0, stream ended in `result success`) while a dispatched
    worker was still unreported and no cap had tripped. It is a terminal status distinct from `crashed`.
  - Source: the 26.1 follow-up (ROADMAP Phase 27 note; spike MANIFEST; `exitclass.py` forward note). The status
    `exitclass.py` wrote was unvalidated in the spike (26.1-RESEARCH A2); in Rust it is a typed enum variant,
    accepted by the server and the reader.

  — **Reversibility:** costly — the frontmatter v1 schema is read by Phase 30 and written into run dirs on disk.
- **D-06:** `[inferred — audit]` Phase 27 ports `exitclass.py::outstanding_dispatch` as a pure function over
  driverctl's own event model, with the same semantics:
  - a dispatch stays open until a later `report`, `recovered` or terminal `status` (done|parked|failed) for its item;
  - a `RUN | dispatch-denied` cancels only the immediately preceding open dispatch, and only when no item-level event
    lies between them (WR-01). RUN-level launcher lines never count as "between".

  Phase 27 also exposes a launcher-only (server-side, not socket-reachable from the sandbox) terminal-status setter
  for `crashed` / `stalled`, which records the reason. The `classify_exit(rc, result_ok, events)` decision itself
  is called by Phase 28's launcher. Rationale: `outstanding_dispatch` reads driverctl-owned events, and one
  definition of "a dispatch with no report" must be shared by the launcher and the Stop guard (26.1 D-03 lesson).
- **D-07:** Reader liveness (DCTL-03):
  - `running`, `starting` or `paused` with the run flock free reads as **crashed**. This is derived at read time and
    never written by the reader.
  - Terminal statuses (`stopped | done | crashed | stalled | parked`) are shown as written; a free lock is expected
    for them.
  - The probe is a non-blocking flock attempt that is released immediately.
  - `[inferred — audit]` The probe targets the lock file that Phase 28's launcher holds. LNCH-02 says that lock
    reuses Phase 17 CTRL-05 (`driver::lock::lock_path`). Phase 27 adds a `probe_held(path) -> bool` beside
    `acquire` in `src/driver/lock.rs` and takes the lock path as a parameter, so Phase 28 can still choose a per-run
    path.
  - The research step must confirm that a momentary probe acquisition cannot make a concurrently starting launcher
    see a false "already running" refusal. If it can, the probe uses `LOCK_SH|LOCK_NB` or a retry.

### Gap-round counting (DCTL-04, SC4; 26.1 lesson)
- **D-08:** Gap rounds come only from `gaps_found` VERIFICATION.md writes on disk, never from the driver's account.
  Phase 27 adopts the **26.1 coalescing rule verbatim** from `tools/gapcount.py`, not spike 002's raw hash-distinct
  count. Phase 26 Run 1 shows why: one verifier `sed -i` re-write 6.5 s later inflated the count to 3. The rule:
  - a digest already seen is a `duplicate` and moves neither the count nor the window anchor;
  - otherwise, a write less than `COALESCE_S = 45` s (strict float seconds, file mtime) after the previous
    gaps_found write of the same phase is `coalesced`, and the anchor slides to it;
  - otherwise it is `new`, and the phase count goes up by 1.

  Status is read by one definition: the leading-frontmatter `status:` key, with the value optionally quoted (WR-03).
  State persists as `caps.gap_rounds {seen, per_phase, last_write, coalesced}` in state.json, so a relaunch keeps
  the window.
  — **Reversibility:** costly — Phase 28's deny-at-L+1 / kill-at-L+2 gate reads `per_phase`, and the replay
  fixtures pin the numbers.
- **D-09:** `[inferred — audit]` Ownership of the counter is shifted from Phase 28 to Phase 27. ROADMAP/MANIFEST
  say "Phase 28 ports this rule", but DCTL-04 makes driverctl the owner of the counters and the frontmatter
  displays `gap_rounds`. Phase 27 therefore owns the counting function and its state, and Phase 28 owns only the
  gate/trip/kill that consume `per_phase`.
  - The selftest fixtures from `gapcount.py` (`selftest`, the Phase 26 Run 1 replay that must show the 6.5 s re-write
    as coalesced, and the `r261a` A9 shape) are ported as Rust unit tests.
  - The *observation* trigger (how a new VERIFICATION.md write reaches the store) is a store method,
    `observe_verification(path, mtime, digest)`. `serve` may drive it with a scan or notify. The launcher monitor
    loop can call it directly.
- **D-10:** Dispatches are counted from hook payloads: a `PreToolUse Agent|Task` payload with no `agent_id` is a
  driver-level dispatch and increments `hook_dispatches` (= `caps.dispatches.used`). A nested payload (with
  `agent_id`) does not count. The dispatch audit `logged == observed` is kept in the frontmatter as
  `dispatch_audit {logged, observed, denied}`. — from DCTL-04 / spike 002.
- **D-11:** Heartbeat-only rewrites of DIRECTION.md (hook `PostToolUse *` / `Stop` events with no state change)
  are coalesced to at most one every **120 s** (`HEARTBEAT_REGEN_MIN_S`). state.json still updates on every
  heartbeat. Any non-heartbeat event regenerates immediately. — from DCTL-04 / spike 002 finding 7.

### Frontmatter write path (DCTL-02, SC2)
- **D-12:** Exchange-verify splice via `rustix::fs::renameat_with(…, RenameFlags::EXCHANGE)`. The steps:
  1. write the new file (new frontmatter plus the current body, byte-for-byte) to a temp file in the same dir;
  2. exchange it with DIRECTION.md;
  3. compare the displaced inode with the one read;
  4. on a mismatch, swap back and retry from a fresh read.

  The human body is preserved byte-for-byte. A body lacking either `## Direction` or `## Items` is never written
  back: the server logs `skipped-insane-body` and leaves the file. Linux-only, behind `#[cfg(target_os = "linux")]`.
  On other platforms the subcommand returns a typed unsupported-platform error, mirroring the `Drive` posture
  (Phase 17 D-05). `[inferred — audit]`: no macOS `renamex_np` path in this phase.
- **D-13:** gsd-mm's own DIRECTION writes take the same flock as driverctl's splice, so they never race. No gsd-mm
  code writes DIRECTION.md today, because the steering UI comes in Phase 30. Phase 27 therefore ships the shared
  primitive:
  - a `direction_lock(run_dir)` and a body-only write helper that never touches the `---` block;
  - Phase 30 must use it.
  - `[inferred — audit]` The lock file is `<run-dir>/DIRECTION.lock`, separate from the run flock, so steering
    edits never contend with liveness.
- **D-14:** Editor-race test (SC2), ported from spike 002 `experiment/race_test.py`:
  - a human writer saves continuously, by rename-save and by in-place save, while the store regenerates in a tight
    loop;
  - a save is LOST when the file still shows an older body 50 ms later;
  - pass criteria: zero lost rename-saves, and an in-place loss rate at or below the spike's measured rate
    (1 / 13466 saves, about 7.4e-5).
  - `[inferred — audit]` The full 15 s run is `#[ignore]` (opt-in, like the conformance oracle test), and a short
    smoke variant runs in the default `cargo test`.

### Schema & reader (DCTL-03, SC3)
- **D-15:** Frontmatter schema v1 is a typed `DriverFrontmatter` (serde, `serde_yml` 0.0.13, already a dependency)
  in `state_reader`. Its fields follow spike 002's generator:
  - `schema`, `run_id`, `status`, `control`, `heartbeat`;
  - `connected[]` and `items{state, outcome, …}`;
  - `caps{dispatches, wall_clock, gap_rounds{used, limit, phase}}`;
  - `dispatch_audit`;
  - `last_event`.

  Parsing goes through the existing `state_md::extract_frontmatter`. Unknown keys are tolerated, so the reader stays
  forward-compatible. Field-level detail is the researcher's job, reading `driverctl.py:270-310` and the spike's
  `templates/`.
- **D-16:** Golden fixtures are kept for every generated state: template, starting, running, stopped, crashed,
  **stalled**, and multi-connection. Each one must round-trip: the Rust generator emits it and the Rust reader
  parses it into `DriverFrontmatter`. The spike-002 Python outputs are cross-checked as additional reader fixtures.

### Run-state lessons from 26.1 (carried as constraints)
- **D-17:** Launch-end classification must never collapse "ended with a worker outstanding" into `crashed`, which
  was the spike 002 `launcher.py:362-363` bug. Phase 27's store must accept both terminal outcomes and record the
  evidence string, for example `status=running and no RUN end; lock was free` for crash-detected. `RUN end`
  logged by the driver, or a tripped cap, takes precedence over classification. — from 26.1 D-03 / exitclass.py.
- **D-18:** Counters stay replay-testable offline. Every counting rule (dispatch audit, gap coalescing, outstanding
  dispatch) is a pure function with fixture tests built from recorded spike LOGs and timestamps. This is the
  26.1 D-02 lesson: prove counting offline before any paid run.

### Musk pass (discuss:pre)
- No scope cuts. DCTL-01..04 all have owners: the spike-002 user requirement for a tool-written frontmatter, and
  the v2.0 re-plan. Lean lane not taken: more than 3 tasks across socket, splice, schema, counters and the race test.

### Claude's Discretion
- Module layout: a new `src/driverctl/` module versus a submodule under `src/driver/`.
- Sync std unix sockets versus tokio for `serve`. The request rate is one per event, so either works.
- Plan split and waves. The schema/reader, counters and splice are largely independent. The socket server and the
  replay test depend on all three.
</decisions>

<canonical_refs>
## Canonical References

**Downstream agents MUST read these before planning or implementing.**

### Requirements & roadmap
- `.planning/ROADMAP.md` §"Phase 27: driverctl & Driver Frontmatter" — goal, SC1-4, and the 26.1 `stalled` note
- `.planning/REQUIREMENTS.md` DCTL-01..04, plus LNCH-01..03 (Phase 28 consumers of the counters and the lock)
- `.planning/ROADMAP.md` §Phase 30 — the reader's consumer, which requires that steering edits use the shared flock

### Spike 002 (the prototype being ported)
- `.planning/spikes/002-launcher-caps-and-tool-frontmatter/tools/driverctl.py` — protocol, enums, RunStore, hook
  handling, frontmatter generator, 120 s coalescing
- `.planning/spikes/002-launcher-caps-and-tool-frontmatter/README.md` §B "Frontmatter: race measurement" and §Results
  (b)/(c) — the splice design, the loss rates and the Rust-compatibility check
- `.planning/spikes/002-launcher-caps-and-tool-frontmatter/experiment/race_test.py` — the race test to port
- `.planning/spikes/002-launcher-caps-and-tool-frontmatter/templates/` — DIRECTION.md / LOG.md templates and fixtures
- `.planning/spikes/002-launcher-caps-and-tool-frontmatter/MASTER-PROMPT.md` — must run unchanged (SC1)

### Spike 003 / Phase 26.1 (lessons and rules to port)
- `.planning/spikes/003-divergent-gap-loop-validation/tools/gapcount.py` — the coalesced gap-round rule
  (`COALESCE_S = 45`), `fm_status`, selftest fixtures
- `.planning/spikes/003-divergent-gap-loop-validation/tools/exitclass.py` — `outstanding_dispatch`,
  `classify_exit`, the `stalled` definition
- `.planning/spikes/003-divergent-gap-loop-validation/tools/launcher_s003.py` — how the launcher drives the
  RunStore (crash-detected evidence, socket dir `$XDG_RUNTIME_DIR/gsdmm-driverctl`)
- `.planning/spikes/MANIFEST.md` — the consolidated requirements and the 26.1 carries
- `.planning/phases/26.1-driver-validation-spike-003-re-run/26.1-CONTEXT.md` — D-01/D-03 (gap dedup, liveness)
- `.planning/phases/26.1-driver-validation-spike-003-re-run/26.1-VERIFICATION.md` — VALID-01 proven, VALID-02 ADJUST

</canonical_refs>

<code_context>
## Existing Code Insights

### Reusable Assets
- `src/driver/lock.rs`: the CTRL-05 `flock` via rustix (`acquire`, `lock_path`, `read_holder`). Add the
  non-blocking `probe_held` here; this file is the repo's only flock idiom.
- `src/state_reader/state_md.rs:325` `extract_frontmatter` and `src/state_reader/disk_status.rs:578`
  `leading_frontmatter_value`: the frontmatter split and scalar read, already shown in spike 002 to parse
  driverctl output.
- `rustix` 1.1 (`fs` feature) is already a dependency. Use `renameat_with` + `RenameFlags::EXCHANGE`, with no new
  crate.
- `serde_yml` 0.0.13, `serde_json`, `sha2` (gap-write digests) and `tempfile` are all present.

### Established Patterns
- Typed refusals instead of silent ones. Unix-only implementation submodules are `#[cfg]`-gated, while the clap
  subcommand stays ungated and returns a typed `UnsupportedPlatform` (src/driver/mod.rs header fact 2).
- One build, one binary: the detached driver is the same binary (`Commands::Drive` in `src/cli.rs`).
- Long doc-comment headers that record the decision behind each non-obvious fact. Follow the style of
  `src/driver/lock.rs` and `src/driver/mod.rs`.
- Run `cargo test --no-fail-fast`, because fail-fast hides the envelope suites (project memory).

### Integration Points
- `src/cli.rs` `Commands`: add `Driverctl { Serve | Log | Hook | Show }`.
- `src/main.rs`: dispatch. `driverctl` runs headless, with no TUI and no terminal ownership.
- `src/state_reader/mod.rs`: export `DriverFrontmatter` and the liveness-derived status for Phase 30.
- Run dirs live under `.planning/meta-manager/drivers/<run-id>/` in the main checkout and are gitignored
  (MANIFEST).

</code_context>

<specifics>
## Specific Ideas

- The frontmatter is a derived view, never LLM-generated. It is regenerated from state.json on every event, and
  only the tool's own `---` block is replaced (spike 002 user requirement).
- The 26.1 evidence numbers are the test oracle:
  - the Phase 26 Run 1 6.5 s `sed -i` re-write must count as one round;
  - three writes 20 s apart are one round;
  - the Run 1b shape (rc 0, `result success`, one open dispatch) must classify as `stalled`.
</specifics>

<deferred>
## Deferred Ideas

- macOS `renamex_np(RENAME_SWAP)` splice: deferred until the sandbox story is settled. Not in this phase.
- The launcher's `classify_exit` call site, cap gate/trip/kill and Stop guard (`stop_guard.py`, at most 3 blocks):
  Phase 28. The GSD stall-watch path (`planner.stall_detection_enabled`): Phase 28/29 carry.
- 26.1 review warnings WR-10 and WR-11 (spike harness pre-registration and budget gate): not relevant to
  driverctl; they stay open in `26.1-REVIEW-DISPOSITION.md`.

### Reviewed Todos (not folded)
`[inferred — audit]` This departs from the auto rule (fold any todo scoring at least 0.4): both todos are outside
the driverctl boundary.
- "Driver tab renders blank pipeline row at terminal heights 8-13" (score 0.9, ui): this is the Phase 30 Driver tab.
- "The verify-work gate policy must be configurable" (score 0.9, driver/router): this is the Phase 20 router or
  Phase 29 prompt policy, not driverctl.
</deferred>

---

*Phase: 27-driverctl-driver-frontmatter*
*Context gathered: 2026-10-18*
